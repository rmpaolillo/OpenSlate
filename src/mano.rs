//! «La tua mano» (sperimentale, solo sul computer dove c'è il modello): la
//! bella copia scritta con la tua grafia invece che col carattere chiaro.
//!
//! - Il testo lo scrive una rete neurale (il modello di sintesi della
//!   scrittura di A. Graves, «Generating Sequences With Recurrent Neural
//!   Networks», 2013, coi pesi del progetto handwriting-synthesis di
//!   S. Vasquez): tre LSTM con una finestra d'attenzione sulle lettere e
//!   una miscela di gaussiane per il prossimo passo della penna. Prima di
//!   scrivere legge una tua riga (la «frase di prova»): da lì prende lo
//!   stile. Scrive una parola alla volta; di ogni parola ne prova alcune e
//!   tiene quella con l'attenzione più regolare.
//! - Cifre, lettere sole e simboli delle formule vengono dalla pagina di
//!   prova: li scrivi una volta, in righe, e si ritagliano uno per uno.
//!
//! Tutto sta nella cartella `mano/` dei dati (mai su GitHub): i pesi
//! (`graves.bin`, che non si distribuiscono: sono addestrati su IAM-OnDB,
//! solo per ricerca), la frase di prova (`prima.txt`), i segni
//! (`glifi.txt`) e le parole già scritte (`parole.txt`). Senza `graves.bin`
//! la funzione non c'è e l'interfaccia non la mostra.

use anyhow::{bail, Context, Result};
use std::cell::Cell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

pub const DIR: &str = "mano";
const WEIGHTS: &str = "graves.bin";
const PRIME: &str = "prima.txt";
const GLYPHS: &str = "glifi.txt";
const CACHE: &str = "parole.txt";

/// L'alfabeto del modello (73 segni: niente Q, X, Z, accenti, + = /).
const ALPHABET: &str = "\0 !\"#'(),-.0123456789:;?ABCDEFGHIJKLMNOPRSTUVWYabcdefghijklmnopqrstuvwxyz";
const NA: usize = 73;
const H: usize = 400;
const K_ATT: usize = 10;
const K_OUT: usize = 20;

/// Il corpo delle minuscole nelle unità di `mathhand` (Hershey).
pub const XH: f32 = 14.0;

fn code(c: char) -> Option<usize> {
    ALPHABET.chars().position(|a| a == c)
}

fn path(name: &str) -> PathBuf {
    PathBuf::from(DIR).join(name)
}

/// C'è il modello? (Allora l'interfaccia mostra «La tua mano».)
pub fn available() -> bool {
    static A: OnceLock<bool> = OnceLock::new();
    *A.get_or_init(|| path(WEIGHTS).is_file())
}

// ---------------------------------------------------------------------------
// Il modello

struct Mat {
    rows: usize,
    cols: usize,
    d: Vec<f32>,
}

struct Graves {
    k1: Mat,
    b1: Vec<f32>,
    k2: Mat,
    b2: Vec<f32>,
    k3: Mat,
    b3: Vec<f32>,
    ka: Mat,
    ba: Vec<f32>,
    kg: Mat,
    bg: Vec<f32>,
}

#[derive(Clone)]
struct St {
    h1: Vec<f32>,
    c1: Vec<f32>,
    h2: Vec<f32>,
    c2: Vec<f32>,
    h3: Vec<f32>,
    c3: Vec<f32>,
    kappa: [f32; K_ATT],
    w: Vec<f32>,
}

impl St {
    fn zero() -> St {
        let z = vec![0.0; H];
        St { h1: z.clone(), c1: z.clone(), h2: z.clone(), c2: z.clone(), h3: z.clone(), c3: z, kappa: [0.0; K_ATT], w: vec![0.0; NA] }
    }
}

/// `out = b + concat(parts) @ k` (k: righe = ingressi).
fn mv(k: &Mat, b: &[f32], parts: &[&[f32]], out: &mut Vec<f32>) {
    out.clear();
    out.extend_from_slice(b);
    let mut r = 0;
    for p in parts {
        for &x in p.iter() {
            if x != 0.0 {
                let row = &k.d[r * k.cols..(r + 1) * k.cols];
                for (o, w) in out.iter_mut().zip(row) {
                    *o += x * w;
                }
            }
            r += 1;
        }
    }
    debug_assert_eq!(r, k.rows);
}

fn sigm(v: f32) -> f32 {
    1.0 / (1.0 + (-v).exp())
}

/// La cella LSTM di TensorFlow (porte i, j, f, o; forget_bias 1).
fn lstm(z: &[f32], h: &mut [f32], c: &mut [f32]) {
    for u in 0..H {
        let (i, j, f, o) = (z[u], z[H + u], z[2 * H + u], z[3 * H + u]);
        c[u] = sigm(f + 1.0) * c[u] + sigm(i) * j.tanh();
        h[u] = sigm(o) * c[u].tanh();
    }
}

fn softplus(a: f32) -> f32 {
    a.max(0.0) + (-a.abs()).exp().ln_1p()
}

impl Graves {
    fn load(bytes: &[u8]) -> Result<Graves> {
        if bytes.len() < 8 || &bytes[..4] != b"GRV1" {
            bail!("graves.bin: formato sconosciuto");
        }
        let u32_at = |o: usize| -> Result<u32> { Ok(u32::from_le_bytes(bytes.get(o..o + 4).context("graves.bin troncato")?.try_into()?)) };
        let n = u32_at(4)? as usize;
        let mut off = 8;
        let mut mats = Vec::new();
        for _ in 0..n {
            let rows = u32_at(off)? as usize;
            let cols = u32_at(off + 4)? as usize;
            off += 8;
            let len = rows * cols * 4;
            let raw = bytes.get(off..off + len).context("graves.bin troncato")?;
            let d: Vec<f32> = raw.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
            off += len;
            mats.push(Mat { rows, cols, d });
        }
        let shapes = [(476, 1600), (1, 1600), (876, 1600), (1, 1600), (876, 1600), (1, 1600), (476, 30), (1, 30), (400, 121), (1, 121)];
        if mats.len() != shapes.len() || mats.iter().zip(shapes).any(|(m, (r, c))| m.rows != r || m.cols != c) {
            bail!("graves.bin: le matrici non sono quelle del modello");
        }
        let mut it = mats.into_iter();
        let mut m = || it.next().unwrap();
        let k1 = m();
        let b1 = m().d;
        let k2 = m();
        let b2 = m().d;
        let k3 = m();
        let b3 = m().d;
        let ka = m();
        let ba = m().d;
        let kg = m();
        let bg = m().d;
        Ok(Graves { k1, b1, k2, b2, k3, b3, ka, ba, kg, bg })
    }

    /// Un passo della penna; ritorna la lettera dove guarda l'attenzione.
    fn step(&self, inp: [f32; 3], st: &mut St, chars: &[usize], z: &mut Vec<f32>) -> usize {
        mv(&self.k1, &self.b1, &[&st.w, &inp, &st.h1], z);
        lstm(z, &mut st.h1, &mut st.c1);
        let mut a = Vec::with_capacity(3 * K_ATT);
        mv(&self.ka, &self.ba, &[&st.w, &inp, &st.h1], &mut a);
        let mut alpha = [0.0f32; K_ATT];
        let mut beta = [0.0f32; K_ATT];
        for i in 0..K_ATT {
            alpha[i] = softplus(a[i]);
            beta[i] = softplus(a[K_ATT + i]).max(0.01);
            st.kappa[i] += softplus(a[2 * K_ATT + i]) / 25.0;
        }
        let mut w = vec![0.0f32; NA];
        let (mut best, mut arg) = (f32::MIN, 0);
        for (u, &ch) in chars.iter().enumerate() {
            let mut phi = 0.0;
            for i in 0..K_ATT {
                let d = st.kappa[i] - u as f32;
                phi += alpha[i] * (-d * d / beta[i]).exp();
            }
            w[ch] += phi;
            if phi > best {
                best = phi;
                arg = u;
            }
        }
        st.w = w;
        mv(&self.k2, &self.b2, &[&inp, &st.h1, &st.w, &st.h2], z);
        lstm(z, &mut st.h2, &mut st.c2);
        mv(&self.k3, &self.b3, &[&inp, &st.h2, &st.w, &st.h3], z);
        lstm(z, &mut st.h3, &mut st.c3);
        arg
    }

    /// Il prossimo passo della penna (dx, dy, penna su), con la «sicurezza»
    /// `bias` (più alta = più regolare).
    fn sample(&self, st: &St, bias: f32, rng: &mut Rng) -> [f32; 3] {
        let mut z = Vec::with_capacity(121);
        mv(&self.kg, &self.bg, &[&st.h3], &mut z);
        let mut pis: Vec<f32> = z[..K_OUT].iter().map(|p| p * (1.0 + bias)).collect();
        let mx = pis.iter().cloned().fold(f32::MIN, f32::max);
        for p in pis.iter_mut() {
            *p = (*p - mx).exp();
        }
        let s: f32 = pis.iter().sum();
        for p in pis.iter_mut() {
            *p /= s;
            if *p < 0.01 {
                *p = 0.0;
            }
        }
        let s: f32 = pis.iter().sum();
        let mut r = rng.uniform() * s;
        let mut k = 0;
        for (i, p) in pis.iter().enumerate() {
            if *p > 0.0 {
                k = i;
                if r < *p {
                    break;
                }
                r -= p;
            }
        }
        let s1 = (z[K_OUT + k] - bias).exp().max(1e-4);
        let s2 = (z[2 * K_OUT + k] - bias).exp().max(1e-4);
        let rho = z[3 * K_OUT + k].tanh().clamp(-0.999_999, 0.999_999);
        let (m1, m2) = (z[4 * K_OUT + k], z[5 * K_OUT + k]);
        let (n1, n2) = (rng.normal(), rng.normal());
        let x = m1 + s1 * n1;
        let y = m2 + s2 * (rho * n1 + (1.0 - rho * rho).sqrt() * n2);
        let mut e = sigm(z[6 * K_OUT]).clamp(1e-8, 1.0 - 1e-8);
        if e < 0.01 {
            e = 0.0;
        }
        let eos = if rng.uniform() < e { 1.0 } else { 0.0 };
        [x, y, eos]
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        let mut r = Rng(seed ^ 0x9e37_79b9_7f4a_7c15);
        r.next();
        r
    }
    fn next(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    fn uniform(&mut self) -> f32 {
        ((self.next() >> 40) as f32 + 0.5) / (1u64 << 24) as f32
    }
    fn normal(&mut self) -> f32 {
        let (a, b) = (self.uniform(), self.uniform());
        (-2.0 * a.ln()).sqrt() * (std::f32::consts::TAU * b).cos()
    }
}

// ---------------------------------------------------------------------------
// I tratti: dai passi della penna ai punti, e il contrario

/// Lisciatura di Savitzky–Golay (finestra 7, grado 3), coi bordi ripetuti.
fn savgol7(v: &[f32]) -> Vec<f32> {
    const C: [f32; 7] = [-2.0, 3.0, 6.0, 7.0, 6.0, 3.0, -2.0];
    let n = v.len() as isize;
    (0..n)
        .map(|i| {
            let mut s = 0.0;
            for (k, c) in C.iter().enumerate() {
                let j = (i + k as isize - 3).clamp(0, n - 1);
                s += c * v[j as usize];
            }
            s / 21.0
        })
        .collect()
}

/// Liscia ogni tratto (punti con la penna su in fondo) di almeno 7 punti.
fn denoise(c: &mut [[f32; 3]]) {
    let mut start = 0;
    for i in 0..c.len() {
        if c[i][2] == 1.0 || i + 1 == c.len() {
            let part = &mut c[start..=i];
            if part.len() >= 7 {
                let xs = savgol7(&part.iter().map(|p| p[0]).collect::<Vec<_>>());
                let ys = savgol7(&part.iter().map(|p| p[1]).collect::<Vec<_>>());
                for (k, p) in part.iter_mut().enumerate() {
                    p[0] = xs[k];
                    p[1] = ys[k];
                }
            }
            start = i + 1;
        }
    }
}

/// Raddrizza la riga (retta dei minimi quadrati) e la porta a y ≈ 0.
fn align(c: &mut [[f32; 3]]) {
    let n = c.len() as f32;
    if n < 2.0 {
        return;
    }
    let (mx, my) = (c.iter().map(|p| p[0]).sum::<f32>() / n, c.iter().map(|p| p[1]).sum::<f32>() / n);
    let sxx: f32 = c.iter().map(|p| (p[0] - mx).powi(2)).sum();
    let sxy: f32 = c.iter().map(|p| (p[0] - mx) * (p[1] - my)).sum();
    let slope = if sxx > 0.0 { sxy / sxx } else { 0.0 };
    let offset = my - slope * mx;
    let th = slope.atan();
    let (cs, sn) = (th.cos(), th.sin());
    for p in c.iter_mut() {
        let (x, y) = (p[0], p[1]);
        p[0] = x * cs + y * sn - offset;
        p[1] = -x * sn + y * cs - offset;
    }
}

fn quantile(v: &mut [f32], q: f32) -> f32 {
    v.sort_by(f32::total_cmp);
    if v.is_empty() {
        return 0.0;
    }
    let pos = q * (v.len() - 1) as f32;
    let (i, f) = (pos.floor() as usize, pos.fract());
    if i + 1 < v.len() {
        v[i] * (1.0 - f) + v[i + 1] * f
    } else {
        v[i]
    }
}

/// La tua riga (tratti con x verso destra e y verso il basso, i punti del
/// pennino) come passi della penna per il modello: un punto ogni k se sono
/// molto più fitti dei suoi, y verso l'alto, raddrizzata, lisciata, passi
/// di lunghezza mediana 1.
pub fn prime_from(strokes: &[Vec<(f32, f32)>]) -> Vec<[f32; 3]> {
    let strokes: Vec<&Vec<(f32, f32)>> = strokes.iter().filter(|s| !s.is_empty()).collect();
    let mut ys: Vec<f32> = strokes.iter().flat_map(|s| s.iter().map(|p| p.1)).collect();
    if ys.len() < 10 {
        return Vec::new();
    }
    let h = quantile(&mut ys.clone(), 0.95) - quantile(&mut ys, 0.05);
    let mut steps: Vec<f32> = strokes.iter().flat_map(|s| s.windows(2).map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1))).collect();
    let med = quantile(&mut steps, 0.5).max(1e-6);
    let k = ((h / med) / 15.0).round().max(1.0) as usize;
    let mut c: Vec<[f32; 3]> = Vec::new();
    for s in strokes {
        let mut q: Vec<(f32, f32)> = if s.len() > 2 * k { s.iter().step_by(k).cloned().collect() } else { s.clone() };
        if q.last() != s.last() {
            q.push(*s.last().unwrap());
        }
        let m = q.len();
        for (j, (x, y)) in q.into_iter().enumerate() {
            c.push([x, -y, if j + 1 == m { 1.0 } else { 0.0 }]);
        }
    }
    align(&mut c);
    denoise(&mut c);
    let mut o = vec![[0.0, 0.0, 1.0]];
    for w in c.windows(2) {
        o.push([w[1][0] - w[0][0], w[1][1] - w[0][1], w[1][2]]);
    }
    o.truncate(1200);
    let mut norms: Vec<f32> = o.iter().map(|p| p[0].hypot(p[1])).collect();
    let med = quantile(&mut norms, 0.5).max(1e-6);
    for p in o.iter_mut() {
        p[0] /= med;
        p[1] /= med;
    }
    o
}

// ---------------------------------------------------------------------------
// La mano: il modello che ha già letto la tua riga

const ACCENTS: [(char, char, bool); 8] =
    [('à', 'a', true), ('è', 'e', true), ('é', 'e', false), ('ì', 'i', true), ('ò', 'o', true), ('ù', 'u', true), ('È', 'E', true), ('À', 'A', true)];
/// Lettere che il modello non conosce: una vicina.
const SUBST: [(char, char); 3] = [('Q', 'O'), ('X', 'x'), ('Z', 'z')];
/// Lettere senza aste né gambe (per trovare la riga di base).
const XLOW: &str = "acemnorsuvwxz";

fn base_char(c: char) -> Option<char> {
    let c = ACCENTS.iter().find(|a| a.0 == c).map_or(c, |a| a.1);
    let c = SUBST.iter().find(|a| a.0 == c).map_or(c, |a| a.1);
    (c != '\0' && code(c).is_some()).then_some(c)
}

/// Una parola che il modello sa scrivere (lettere sue, accenti italiani).
pub fn writable_word(w: &str) -> bool {
    !w.is_empty() && w.chars().count() <= 24 && w.chars().all(|c| base_char(c).is_some())
}

/// La «sicurezza» del modello (più alta = più regolare, meno varia) e
/// quante prove per parola.
const BIAS: f32 = 2.0;
const TRIES: u64 = 6;

/// Una parola scritta: tratti con la riga di base a y = 0 e x da 0 (y verso
/// il basso, unità del modello) e il corpo delle minuscole.
#[derive(Clone, Debug)]
pub struct WordInk {
    pub strokes: Vec<Vec<(f32, f32)>>,
    pub xh: f32,
}

pub struct Hand {
    g: Graves,
    /// La frase di prova (e lo spazio dopo), già letta.
    prime_chars: Vec<usize>,
    st: St,
    primed: bool,
    /// Il corpo delle minuscole di questa mano (unità del modello): uguale
    /// per tutte le parole, misurato una volta su una parola di sole lettere
    /// basse (0 finché non si è misurato).
    xh: f32,
}

impl Hand {
    fn new(g: Graves, prime: &[[f32; 3]], prime_text: &str) -> Hand {
        let primed = !prime.is_empty() && !prime_text.is_empty();
        let mut prime_chars = Vec::new();
        let mut st = St::zero();
        if primed {
            prime_chars = format!("{prime_text} ").chars().map(|c| code(c).unwrap_or(0)).collect();
            let mut chars = prime_chars.clone();
            chars.push(0);
            let mut z = Vec::with_capacity(1600);
            for &x in prime {
                g.step(x, &mut st, &chars, &mut z);
            }
        }
        let mut h = Hand { g, prime_chars, st, primed, xh: 0.0 };
        h.xh = h.body();
        h
    }

    /// Il corpo delle minuscole: l'altezza di «ammaccano» (solo lettere
    /// basse, niente puntini).
    fn body(&self) -> f32 {
        let Some(w) = self.word_with("ammaccano", 7, BIAS, TRIES) else { return 6.0 };
        let mut ys: Vec<f32> = w.strokes.iter().flatten().map(|p| p.1).collect();
        let b = quantile(&mut ys.clone(), 0.95) - quantile(&mut ys, 0.05);
        if b > 0.5 { b } else { 6.0 }
    }

    fn one(&self, base: &[usize], bias: f32, seed: u64) -> (Vec<[f32; 3]>, Vec<i32>, bool) {
        let mut rng = Rng::new(seed);
        let mut chars = self.prime_chars.clone();
        chars.extend_from_slice(base);
        chars.push(0);
        let n = chars.len();
        let np = self.prime_chars.len() as i32;
        let mut st = self.st.clone();
        let mut z = Vec::with_capacity(1600);
        let mut inp = if self.primed { self.g.sample(&st, bias, &mut rng) } else { [0.0, 0.0, 1.0] };
        let (mut out, mut pos) = (Vec::new(), Vec::new());
        let mut on_last = 0;
        for _ in 0..45 * base.len() + 40 {
            out.push(inp);
            let idx = self.g.step(inp, &mut st, &chars, &mut z);
            pos.push(idx as i32 - np);
            let nxt = self.g.sample(&st, bias, &mut rng);
            if idx + 2 >= n {
                on_last += 1;
            }
            // sull'ultima lettera si resta un po' (la seconda asta della x,
            // il taglio della t) anche se la penna si alza prima
            if idx + 1 >= n && nxt[2] == 1.0 && on_last >= 14 {
                return (out, pos, true);
            }
            inp = nxt;
        }
        (out, pos, false)
    }

    /// Più basso è meglio: l'attenzione che avanza regolare, lettera per
    /// lettera, e una lunghezza da parola normale.
    /// I passi di ogni lettera si confrontano con la sua larghezza tipica
    /// (una m ne chiede il doppio di una i): una lettera saltata pesa.
    fn score(word: &[char], out: &[[f32; 3]], pos: &[i32], finished: bool) -> f32 {
        if !finished {
            return 1e9;
        }
        let len = word.len();
        let counts: Vec<f32> = (0..len as i32).map(|i| pos.iter().filter(|&&p| p == i).count() as f32).collect();
        if counts.iter().cloned().fold(f32::MAX, f32::min) < 2.0 {
            return 1e8;
        }
        let weight = |c: char| match c {
            'm' | 'w' | 'M' | 'W' => 1.7,
            'i' | 'l' | 'j' | '\'' | '.' | ',' | ':' | ';' | '!' => 0.5,
            't' | 'f' | 'r' | 'I' => 0.75,
            c if c.is_uppercase() => 1.3,
            _ => 1.0,
        };
        let norm: Vec<f32> = counts.iter().zip(word).map(|(n, &c)| n / weight(c)).collect();
        let per = out.len() as f32 / len.max(1) as f32;
        let mean = norm.iter().sum::<f32>() / norm.len() as f32;
        let sd = (norm.iter().map(|c| (c - mean).powi(2)).sum::<f32>() / norm.len() as f32).sqrt();
        let thin = norm.iter().filter(|&&c| c < 0.35 * mean).count() as f32;
        let back = pos.windows(2).filter(|w| w[1] < w[0]).count() as f32;
        sd / mean.max(1e-6) + (per / 24.0).ln().abs() + 0.2 * back + 0.5 * thin
    }

    /// Una parola: se ne scrivono alcune (in parallelo) e si tiene la migliore.
    pub fn word(&self, word: &str, seed: u64) -> Option<WordInk> {
        self.word_with(word, seed, BIAS, TRIES)
    }

    fn word_with(&self, word: &str, seed: u64, bias: f32, tries: u64) -> Option<WordInk> {
        let base: Vec<char> = word.chars().map(|c| base_char(c).unwrap_or('o')).collect();
        let codes: Vec<usize> = base.iter().map(|&c| code(c).unwrap_or(0)).collect();
        let cands: Vec<(f32, Vec<[f32; 3]>, Vec<i32>)> = std::thread::scope(|sc| {
            let hs: Vec<_> = (0..tries)
                .map(|k| {
                    let (codes, base) = (&codes, &base);
                    sc.spawn(move || {
                        let (o, p, f) = self.one(codes, bias, seed.wrapping_mul(1000).wrapping_add(k));
                        (Self::score(base, &o, &p, f), o, p)
                    })
                })
                .collect();
            hs.into_iter().filter_map(|h| h.join().ok()).collect()
        });
        let (score, out, pos) = cands.into_iter().min_by(|a, b| a.0.total_cmp(&b.0))?;
        if score >= 1e8 {
            return None;
        }
        // dai passi ai punti
        let mut c: Vec<[f32; 3]> = Vec::with_capacity(out.len());
        let (mut x, mut y) = (0.0, 0.0);
        for o in &out {
            x += o[0];
            y += o[1];
            c.push([x, y, o[2]]);
        }
        denoise(&mut c);
        let mut strokes: Vec<Vec<(f32, f32)>> = Vec::new();
        let mut owners: Vec<Vec<i32>> = Vec::new();
        let (mut cur, mut curp) = (Vec::new(), Vec::new());
        for (p, &q) in c.iter().zip(&pos) {
            cur.push((p[0], -p[1]));
            curp.push(q);
            if p[2] == 1.0 {
                if cur.len() > 1 {
                    strokes.push(std::mem::take(&mut cur));
                    owners.push(std::mem::take(&mut curp));
                }
                cur.clear();
                curp.clear();
            }
        }
        if cur.len() > 1 {
            strokes.push(cur);
            owners.push(curp);
        }
        if strokes.is_empty() {
            return None;
        }
        // via i puntini sparsi (non quelli delle i e delle j)
        let all_y: Vec<f32> = strokes.iter().flatten().map(|p| p.1).collect();
        let hgt_all = all_y.iter().cloned().fold(f32::MIN, f32::max) - all_y.iter().cloned().fold(f32::MAX, f32::min) + 1e-6;
        let mut keep_s = Vec::new();
        let mut keep_o = Vec::new();
        for (s, ow) in strokes.into_iter().zip(owners) {
            let (x0, x1, y0, y1) = s.iter().fold((f32::MAX, f32::MIN, f32::MAX, f32::MIN), |b, p| (b.0.min(p.0), b.1.max(p.0), b.2.min(p.1), b.3.max(p.1)));
            let mut o: Vec<f32> = ow.iter().map(|&q| q as f32).collect();
            let mid = quantile(&mut o, 0.5).round().clamp(0.0, (base.len() - 1) as f32) as usize;
            if (x1 - x0).max(y1 - y0) < 0.08 * hgt_all && !matches!(base[mid], 'i' | 'j') {
                continue;
            }
            keep_s.push(s);
            keep_o.push(ow);
        }
        let (strokes, owners) = (keep_s, keep_o);
        // la riga di base: dove la penna gira in basso più spesso (come per la
        // tua scrittura sulla lavagna); il corpo è quello della mano, o una
        // stima dalle lettere senza aste né gambe
        let mut low: Vec<f32> = Vec::new();
        for (s, ow) in strokes.iter().zip(&owners) {
            for (p, &q) in s.iter().zip(ow) {
                if q >= 0 && (q as usize) < base.len() && XLOW.contains(base[q as usize]) {
                    low.push(p.1);
                }
            }
        }
        let mut ys: Vec<f32> = strokes.iter().flatten().map(|p| p.1).collect();
        let rough = if low.len() >= 4 {
            quantile(&mut low.clone(), 0.85) - quantile(&mut low, 0.1)
        } else {
            0.5 * (quantile(&mut ys.clone(), 1.0) - quantile(&mut ys, 0.0))
        }
        .max(1e-3);
        let xh = if self.xh > 0.0 { self.xh } else { rough };
        let mut bottoms = Vec::new();
        for s in &strokes {
            bottoms.extend(crate::inbella::turns(s, 0.25 * xh).0);
        }
        let baseline = if bottoms.len() >= 2 { quantile(&mut bottoms, 0.35) } else { quantile(&mut ys, 0.85) };
        // via i puntini rimasti oltre la fine della parola
        let ext = |s: &Vec<(f32, f32)>| {
            let (x0, x1, y0, y1) = s.iter().fold((f32::MAX, f32::MIN, f32::MAX, f32::MIN), |b, p| (b.0.min(p.0), b.1.max(p.0), b.2.min(p.1), b.3.max(p.1)));
            (x0, x1, (x1 - x0).max(y1 - y0))
        };
        let right = strokes.iter().map(ext).filter(|e| e.2 >= 0.25 * xh).map(|e| e.1).fold(f32::MIN, f32::max);
        let keep: Vec<bool> = strokes.iter().map(|s| {
            let e = ext(s);
            e.2 >= 0.25 * xh || e.0 <= right + 0.6 * xh
        }).collect();
        let mut k = keep.iter();
        let (mut strokes, owners): (Vec<_>, Vec<_>) = strokes.into_iter().zip(owners).filter(|_| *k.next().unwrap()).unzip();
        // gli accenti, sopra la lettera giusta (si sa dove, dall'attenzione)
        for (i, ch) in word.chars().enumerate() {
            let Some(&(_, _, grave)) = ACCENTS.iter().find(|a| a.0 == ch) else { continue };
            let pts: Vec<(f32, f32)> = strokes.iter().zip(&owners).flat_map(|(s, ow)| s.iter().zip(ow).filter(|(_, &q)| q == i as i32).map(|(p, _)| *p)).collect();
            if pts.is_empty() {
                continue;
            }
            let (x0, x1, top, bot) = pts.iter().fold((f32::MAX, f32::MIN, f32::MAX, f32::MIN), |b, p| (b.0.min(p.0), b.1.max(p.0), b.2.min(p.1), b.3.max(p.1)));
            let (cx, h) = ((x0 + x1) / 2.0, (bot - top).max(0.5 * xh));
            let g = 0.35 * h;
            strokes.push(if grave {
                vec![(cx - 0.25 * h, top - g - 0.55 * h), (cx + 0.1 * h, top - g)]
            } else {
                vec![(cx - 0.1 * h, top - g), (cx + 0.25 * h, top - g - 0.55 * h)]
            });
        }
        let x0 = strokes.iter().flatten().map(|p| p.0).fold(f32::MAX, f32::min);
        for s in strokes.iter_mut() {
            for p in s.iter_mut() {
                p.0 -= x0;
                p.1 -= baseline;
            }
        }
        Some(WordInk { strokes, xh: xh.max(1e-3) })
    }
}

// ---------------------------------------------------------------------------
// I dati nella cartella `mano/`

static HAND: Mutex<Option<Arc<Hand>>> = Mutex::new(None);
static WORDS: Mutex<Option<HashMap<String, Option<WordInk>>>> = Mutex::new(None);
static GLYPHS_MEM: Mutex<Option<HashMap<char, Glyph>>> = Mutex::new(None);
/// Parole scritte e da scrivere (per la barra di stato).
static DONE: AtomicUsize = AtomicUsize::new(0);
static TOTAL: AtomicUsize = AtomicUsize::new(0);

/// A che punto è: (parole scritte, parole da scrivere), se sta scrivendo.
pub fn progress() -> Option<(usize, usize)> {
    let t = TOTAL.load(Ordering::Relaxed);
    (t > 0).then(|| (DONE.load(Ordering::Relaxed), t))
}

fn read_prime() -> (Vec<[f32; 3]>, String) {
    let Ok(t) = std::fs::read_to_string(path(PRIME)) else { return (Vec::new(), String::new()) };
    let mut lines = t.lines();
    let text = lines.next().unwrap_or("").trim().to_string();
    let pts = lines
        .filter_map(|l| {
            let v: Vec<f32> = l.split_whitespace().filter_map(|x| x.parse().ok()).collect();
            (v.len() == 3).then(|| [v[0], v[1], v[2]])
        })
        .collect();
    (pts, text)
}

/// Un'impronta della frase di prova: se cambia, le parole già scritte non
/// valgono più.
fn prime_sig() -> String {
    let t = std::fs::read(path(PRIME)).unwrap_or_default();
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in t {
        h = (h ^ b as u64).wrapping_mul(0x100_0000_01b3);
    }
    format!("{h:016x}")
}

/// La mano (si carica la prima volta: qualche secondo, da fare fuori dal
/// disegno della finestra).
fn hand() -> Result<Arc<Hand>> {
    let mut g = HAND.lock().unwrap();
    if let Some(h) = g.as_ref() {
        return Ok(h.clone());
    }
    let bytes = std::fs::read(path(WEIGHTS)).context("mano/graves.bin")?;
    let model = Graves::load(&bytes)?;
    let (prime, text) = read_prime();
    let h = Arc::new(Hand::new(model, &prime, &text));
    *g = Some(h.clone());
    Ok(h)
}

/// Carica la mano in sottofondo (così la prima bella copia non aspetta).
pub fn preload() {
    if available() {
        let _ = std::thread::Builder::new().name("la-tua-mano".into()).spawn(|| {
            if let Err(e) = hand() {
                tracing::warn!("la tua mano: {e:#}");
            }
        });
    }
}

fn words_map<R>(f: impl FnOnce(&mut HashMap<String, Option<WordInk>>) -> R) -> R {
    let mut g = WORDS.lock().unwrap();
    let m = g.get_or_insert_with(load_cache);
    f(m)
}

fn load_cache() -> HashMap<String, Option<WordInk>> {
    let mut m = HashMap::new();
    let Ok(t) = std::fs::read_to_string(path(CACHE)) else { return m };
    let mut lines = t.lines();
    if lines.next().map(|l| l.trim().to_string()) != Some(format!("# prima {} v3", prime_sig())) {
        return m;
    }
    for l in lines {
        let mut f = l.split('\t');
        let (Some(w), Some(xh), Some(st)) = (f.next(), f.next(), f.next()) else { continue };
        let Ok(xh) = xh.parse::<f32>() else { continue };
        let strokes: Vec<Vec<(f32, f32)>> = st
            .split('|')
            .map(|s| {
                s.split(' ')
                    .filter_map(|p| {
                        let (a, b) = p.split_once(',')?;
                        Some((a.parse().ok()?, b.parse().ok()?))
                    })
                    .collect::<Vec<_>>()
            })
            .filter(|s: &Vec<(f32, f32)>| s.len() > 1)
            .collect();
        if !strokes.is_empty() {
            m.insert(w.to_string(), Some(WordInk { strokes, xh }));
        }
    }
    m
}

fn save_cache() {
    let mut out = format!("# prima {} v3\n", prime_sig());
    words_map(|m| {
        let mut keys: Vec<&String> = m.keys().collect();
        keys.sort();
        for k in keys {
            if let Some(Some(w)) = m.get(k) {
                let st: Vec<String> = w.strokes.iter().map(|s| s.iter().map(|p| format!("{:.2},{:.2}", p.0, p.1)).collect::<Vec<_>>().join(" ")).collect();
                out.push_str(&format!("{k}\t{:.3}\t{}\n", w.xh, st.join("|")));
            }
        }
    });
    let _ = std::fs::create_dir_all(DIR);
    if let Err(e) = crate::web::write_atomic(&path(CACHE), out.as_bytes()) {
        tracing::warn!("mano/{CACHE}: {e}");
    }
}

/// Il seme di una parola: la stessa parola si scrive sempre uguale.
fn seed_of(w: &str) -> u64 {
    let mut h: u64 = 0x6a09_e667_f3bc_c908;
    for b in w.bytes() {
        h = (h ^ b as u64).wrapping_mul(0x100_0000_01b3);
    }
    h >> 8
}

/// Scrive (una volta) le parole che servono: da chiamare fuori dal disegno
/// della finestra (ci vuole circa un secondo a parola).
pub fn warm(words: &[String]) -> Result<()> {
    let mut need: Vec<String> = Vec::new();
    words_map(|m| {
        for w in words {
            if writable_word(w) && !m.contains_key(w) && !need.contains(w) {
                need.push(w.clone());
            }
        }
    });
    if need.is_empty() {
        return Ok(());
    }
    DONE.store(0, Ordering::Relaxed);
    TOTAL.store(need.len(), Ordering::Relaxed);
    let r = (|| -> Result<()> {
        let h = hand()?;
        for w in &need {
            let ink = h.word(w, seed_of(w));
            words_map(|m| m.insert(w.clone(), ink));
            DONE.fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    })();
    TOTAL.store(0, Ordering::Relaxed);
    save_cache();
    r
}

/// Le parole del testo di una bella copia (fuori dalle formule).
pub fn words_of(text: &str) -> Vec<String> {
    let t = text.replace("$$", "$").replace("\\(", "$").replace("\\)", "$").replace("\\[", "$").replace("\\]", "$");
    t.split('$').step_by(2).flat_map(|p| p.split_whitespace().map(str::to_string).collect::<Vec<_>>()).collect()
}

/// Una parola già scritta (None se non c'è: allora si compone coi segni).
pub fn word(w: &str) -> Option<WordInk> {
    words_map(|m| m.get(w).cloned().flatten())
}

// ---------------------------------------------------------------------------
// I segni della pagina di prova

/// Un segno: avanzamento e tratti (unità di Hershey, riga di base a 0).
#[derive(Clone, Debug)]
pub struct Glyph {
    pub adv: f32,
    pub strokes: Vec<Vec<(f32, f32)>>,
}

fn load_glyphs() -> HashMap<char, Glyph> {
    let mut m = HashMap::new();
    let Ok(t) = std::fs::read_to_string(path(GLYPHS)) else { return m };
    for l in t.lines() {
        let mut f = l.split('\t');
        let (Some(c), Some(adv), Some(st)) = (f.next(), f.next(), f.next()) else { continue };
        let (Some(c), Ok(adv)) = (c.chars().next(), adv.parse::<f32>()) else { continue };
        let strokes: Vec<Vec<(f32, f32)>> = st
            .split('|')
            .map(|s| {
                s.split(' ')
                    .filter_map(|p| {
                        let (a, b) = p.split_once(',')?;
                        Some((a.parse().ok()?, b.parse().ok()?))
                    })
                    .collect::<Vec<_>>()
            })
            .filter(|s: &Vec<(f32, f32)>| !s.is_empty())
            .collect();
        m.insert(c, Glyph { adv, strokes });
    }
    m
}

/// Il tuo segno per `c`, se l'hai scritto nella pagina di prova.
pub fn glyph(c: char) -> Option<Glyph> {
    let mut g = GLYPHS_MEM.lock().unwrap();
    g.get_or_insert_with(load_glyphs).get(&c).cloned()
}

/// Quanti segni ci sono (per la finestra della pagina di prova).
pub fn glyph_count() -> usize {
    let mut g = GLYPHS_MEM.lock().unwrap();
    g.get_or_insert_with(load_glyphs).len()
}

/// La frase di prova salvata (vuota se non c'è).
pub fn prime_text() -> String {
    read_prime().1
}

// ---------------------------------------------------------------------------
// Bella copia con la tua mano: il segnale per `mathhand`

thread_local! {
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
}

/// Esegue `f` con «la tua mano» accesa (se `on` e se c'è il modello).
pub fn using<R>(on: bool, f: impl FnOnce() -> R) -> R {
    let on = on && (available() || cfg!(test));
    let prev = ACTIVE.with(|a| a.replace(on));
    let r = f();
    ACTIVE.with(|a| a.set(prev));
    r
}

pub fn active() -> bool {
    ACTIVE.with(|a| a.get())
}

// ---------------------------------------------------------------------------
// La pagina di prova

/// La frase di prova (solo lettere del modello).
pub const SENTENCE: &str = "Studiare il grafico della seguente funzione";

/// Le righe della pagina di prova: la frase, poi i segni separati da spazi.
pub const ROWS: [&str; 9] = [
    SENTENCE,
    "0 1 2 3 4 5 6 7 8 9",
    "a b c d e f g h i j k l m",
    "n o p q r s t u v w x y z",
    "A B C D E F G H I J K L M",
    "N O P Q R S T U V W X Y Z",
    "+ − × = ≠ < > ≤ ≥ ± /",
    "( ) [ ] { } | ∫ √ ∞ →",
    "α β γ δ ε θ λ μ π σ φ ω Δ",
];

pub struct Calib {
    pub prime: Vec<[f32; 3]>,
    pub glyphs: Vec<(char, Glyph)>,
}

fn bb(pts: impl Iterator<Item = (f32, f32)>) -> (f32, f32, f32, f32) {
    pts.fold((f32::MAX, f32::MIN, f32::MAX, f32::MIN), |b, p| (b.0.min(p.0), b.1.max(p.0), b.2.min(p.1), b.3.max(p.1)))
}

/// Divide i tratti in righe (dall'alto): un tratto va nella riga se il suo
/// centro è dentro la riga o se la tocca per metà della sua altezza; le
/// «righe» minuscole (il puntino di una i sopra la riga) vanno con la più
/// vicina.
fn rows_of(strokes: &[Vec<(f32, f32)>]) -> Vec<Vec<usize>> {
    let boxes: Vec<(f32, f32, f32, f32)> = strokes.iter().map(|s| bb(s.iter().cloned())).collect();
    let mut order: Vec<usize> = (0..strokes.len()).filter(|&i| !strokes[i].is_empty()).collect();
    order.sort_by(|&a, &b| ((boxes[a].2 + boxes[a].3) / 2.0).total_cmp(&((boxes[b].2 + boxes[b].3) / 2.0)));
    let mut rows: Vec<(f32, f32, Vec<usize>)> = Vec::new();
    for i in order {
        let (_, _, y0, y1) = boxes[i];
        let cy = (y0 + y1) / 2.0;
        let h = (y1 - y0).max(1e-6);
        if let Some(r) = rows.last_mut() {
            let rh = r.1 - r.0;
            let overlap = (y1.min(r.1) - y0.max(r.0)).max(0.0);
            if cy <= r.1 + 0.1 * rh || overlap >= 0.5 * h {
                r.0 = r.0.min(y0);
                r.1 = r.1.max(y1);
                r.2.push(i);
                continue;
            }
        }
        rows.push((y0, y1, vec![i]));
    }
    // le righe piccole con la vicina
    loop {
        let mut hs: Vec<f32> = rows.iter().map(|r| r.1 - r.0).collect();
        let med = quantile(&mut hs, 0.5);
        let Some(k) = rows.iter().position(|r| r.1 - r.0 < 0.35 * med) else { break };
        if rows.len() < 2 {
            break;
        }
        let r = rows.remove(k);
        let cy = (r.0 + r.1) / 2.0;
        let j = (0..rows.len()).min_by(|&a, &b| {
            let d = |x: &(f32, f32, Vec<usize>)| if cy < x.0 { x.0 - cy } else if cy > x.1 { cy - x.1 } else { 0.0 };
            d(&rows[a]).total_cmp(&d(&rows[b]))
        });
        let j = j.unwrap();
        rows[j].0 = rows[j].0.min(r.0);
        rows[j].1 = rows[j].1.max(r.1);
        rows[j].2.extend(r.2);
    }
    rows.into_iter()
        .map(|mut r| {
            r.2.sort_unstable();
            r.2
        })
        .collect()
}

/// Divide una riga in segni (da sinistra): tratti che si sovrappongono in
/// orizzontale stanno nello stesso segno; se ne vengono troppi si uniscono
/// quelli più vicini.
fn glyphs_of(strokes: &[Vec<(f32, f32)>], row: &[usize], want: usize) -> Vec<Vec<usize>> {
    let mut idx = row.to_vec();
    let boxes: HashMap<usize, (f32, f32, f32, f32)> = idx.iter().map(|&i| (i, bb(strokes[i].iter().cloned()))).collect();
    idx.sort_by(|a, b| boxes[a].0.total_cmp(&boxes[b].0));
    let mut out: Vec<(f32, f32, Vec<usize>)> = Vec::new();
    for i in idx {
        let (x0, x1, _, _) = boxes[&i];
        if let Some(g) = out.last_mut() {
            if x0 < g.1 {
                g.1 = g.1.max(x1);
                g.2.push(i);
                continue;
            }
        }
        out.push((x0, x1, vec![i]));
    }
    while out.len() > want && out.len() > 1 {
        let k = (0..out.len() - 1).min_by(|&a, &b| (out[a + 1].0 - out[a].1).total_cmp(&(out[b + 1].0 - out[b].1))).unwrap();
        let g = out.remove(k + 1);
        out[k].1 = out[k].1.max(g.1);
        out[k].2.extend(g.2);
    }
    out.into_iter()
        .map(|mut g| {
            g.2.sort_unstable();
            g.2
        })
        .collect()
}

/// La pagina di prova scritta (tratti nell'ordine in cui li hai scritti, x
/// verso destra e y verso il basso) → la tua frase e i tuoi segni. L'errore
/// è un messaggio da mostrare (con i numeri dentro, da tradurre con `tf`).
pub fn calibrate(strokes: &[Vec<(f32, f32)>]) -> std::result::Result<Calib, String> {
    let rows = rows_of(strokes);
    if rows.len() != ROWS.len() {
        return Err(format!("Ho trovato {} righe scritte, ne servono {}: una riga per ogni riga dell'esempio, ben staccate.", rows.len(), ROWS.len()));
    }
    let prime_strokes: Vec<Vec<(f32, f32)>> = rows[0].iter().map(|&i| strokes[i].clone()).collect();
    let prime = prime_from(&prime_strokes);
    if prime.len() < 60 {
        return Err("La frase della prima riga è troppo corta.".into());
    }
    // i segni di ogni riga, con la scala e la riga di base dai segni chiari
    // (le stesse misure del carattere chiaro)
    let mut per_row: Vec<Vec<(char, Vec<usize>)>> = Vec::new();
    for (r, spec) in ROWS.iter().enumerate().skip(1) {
        let chars: Vec<char> = spec.split(' ').filter_map(|s| s.chars().next()).collect();
        let gs = glyphs_of(strokes, &rows[r], chars.len());
        if gs.len() != chars.len() {
            return Err(format!("Riga {}: ho trovato {} segni, ne servono {} (lascia un po' di spazio fra un segno e l'altro).", r + 1, gs.len(), chars.len()));
        }
        per_row.push(chars.into_iter().zip(gs).collect());
    }
    let reference = |c: char| -> Option<(f32, f32)> {
        let g = crate::handwrite::char_glyph(crate::handwrite::Style::Naturale, c)?;
        let (_, _, t, b) = bb(g.strokes.iter().flatten().cloned());
        (b - t > 3.0).then_some((t, b))
    };
    // ref = a·y + b, ai minimi quadrati, riga per riga
    let mut fits: Vec<(f32, Vec<(f32, f32)>)> = Vec::new();
    for row in &per_row {
        let mut pairs: Vec<(f32, f32)> = Vec::new();
        for (c, g) in row {
            let Some((rt, rb)) = reference(*c) else { continue };
            let (_, _, t, b) = bb(g.iter().flat_map(|&i| strokes[i].iter().cloned()));
            pairs.push((t, rt));
            pairs.push((b, rb));
        }
        let n = pairs.len() as f32;
        let (mx, my) = (pairs.iter().map(|p| p.0).sum::<f32>() / n, pairs.iter().map(|p| p.1).sum::<f32>() / n);
        let sxx: f32 = pairs.iter().map(|p| (p.0 - mx).powi(2)).sum();
        let sxy: f32 = pairs.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
        fits.push(((sxx > 0.0).then(|| sxy / sxx).unwrap_or(0.0), pairs));
    }
    let mut a_all: Vec<f32> = fits.iter().map(|f| f.0).filter(|a| *a > 0.0).collect();
    if a_all.is_empty() {
        return Err("Non riesco a misurare i segni della pagina di prova.".into());
    }
    let a = quantile(&mut a_all, 0.5);
    const SIDE: f32 = 1.5;
    let mut glyphs = Vec::new();
    for (row, (_, pairs)) in per_row.iter().zip(&fits) {
        let b = if pairs.is_empty() { 0.0 } else { pairs.iter().map(|p| p.1 - a * p.0).sum::<f32>() / pairs.len() as f32 };
        for (c, g) in row {
            let (x0, x1, _, _) = bb(g.iter().flat_map(|&i| strokes[i].iter().cloned()));
            let st: Vec<Vec<(f32, f32)>> = g.iter().map(|&i| strokes[i].iter().map(|p| (a * (p.0 - x0) + SIDE, a * p.1 + b)).collect()).collect();
            glyphs.push((*c, Glyph { adv: a * (x1 - x0) + 2.0 * SIDE, strokes: st }));
        }
    }
    Ok(Calib { prime, glyphs })
}

/// Salva la pagina di prova in `mano/` e ricomincia con la mano nuova.
pub fn save_calib(c: &Calib) -> Result<()> {
    std::fs::create_dir_all(DIR)?;
    let mut p = format!("{SENTENCE}\n");
    for x in &c.prime {
        p.push_str(&format!("{:.5} {:.5} {}\n", x[0], x[1], x[2] as i32));
    }
    crate::web::write_atomic(&path(PRIME), p.as_bytes())?;
    let mut g = String::new();
    for (ch, gl) in &c.glyphs {
        let st: Vec<String> = gl.strokes.iter().map(|s| s.iter().map(|p| format!("{:.2},{:.2}", p.0, p.1)).collect::<Vec<_>>().join(" ")).collect();
        g.push_str(&format!("{ch}\t{:.2}\t{}\n", gl.adv, st.join("|")));
    }
    crate::web::write_atomic(&path(GLYPHS), g.as_bytes())?;
    // via la mano vecchia, i segni e le parole scritte con lei
    *HAND.lock().unwrap() = None;
    *GLYPHS_MEM.lock().unwrap() = None;
    *WORDS.lock().unwrap() = Some(HashMap::new());
    save_cache();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alfabeto() {
        assert_eq!(ALPHABET.chars().count(), NA);
        assert!(writable_word("funzione,"));
        assert!(writable_word("perché"));
        assert!(!writable_word("x+1"));
        assert_eq!(words_of("Risolvi: $x^2=1$ con $x>0$ ok"), vec!["Risolvi:", "con", "ok"]);
    }

    #[test]
    fn lisciatura_e_raddrizzata() {
        // una retta storta torna orizzontale (come `align` di numpy)
        let mut c: Vec<[f32; 3]> = (0..20).map(|i| [i as f32, 0.5 * i as f32 + 3.0, 0.0]).collect();
        align(&mut c);
        assert!(c.iter().all(|p| (p[1] - c[0][1]).abs() < 1e-3), "{c:?}");
        let v: Vec<f32> = (0..10).map(|i| (i * i) as f32).collect();
        let s = savgol7(&v);
        // un polinomio di grado 2 resta com'è, lontano dai bordi
        for i in 3..7 {
            assert!((s[i] - v[i]).abs() < 1e-3);
        }
    }

    /// Una pagina di prova finta: ogni segno è un quadratino (due tratti per
    /// «=»), le righe ben staccate.
    #[test]
    fn pagina_di_prova() {
        let mut strokes: Vec<Vec<(f32, f32)>> = Vec::new();
        for (r, spec) in ROWS.iter().enumerate() {
            let y = 0.1 + r as f32 * 0.1;
            if r == 0 {
                // la frase: un'onda lunga
                for k in 0..8 {
                    strokes.push((0..40).map(|i| (0.05 + k as f32 * 0.1 + i as f32 * 0.002, y - 0.02 * ((i as f32) * 0.5).sin().abs())).collect());
                }
                continue;
            }
            for (k, c) in spec.split(' ').enumerate() {
                let x = 0.05 + k as f32 * 0.06;
                let h = if c.chars().next().unwrap().is_uppercase() { 0.03 } else { 0.02 };
                strokes.push(vec![(x, y), (x + 0.02, y), (x + 0.02, y - h), (x, y - h), (x, y)]);
                if c == "=" || c == "i" {
                    strokes.push(vec![(x + 0.005, y - h - 0.006), (x + 0.012, y - h - 0.006)]);
                }
            }
        }
        let c = calibrate(&strokes).unwrap();
        assert!(c.prime.len() > 60);
        let n: usize = ROWS.iter().skip(1).map(|r| r.split(' ').count()).sum();
        assert_eq!(c.glyphs.len(), n);
        let x = &c.glyphs.iter().find(|g| g.0 == 'x').unwrap().1;
        let (_, _, t, b) = bb(x.strokes.iter().flatten().cloned());
        // riga di base vicina a 0, corpo come quello del carattere chiaro
        assert!(b.abs() < 4.0 && t < -5.0, "{t} {b}");
        // righe che mancano: messaggio
        assert!(calibrate(&strokes[..strokes.len() - 13]).is_err());
    }

    /// Un passo del modello contro numpy (`GRAVES_DIR` con graves.bin e
    /// ref.txt).
    #[test]
    #[ignore]
    fn passo_come_numpy() {
        let dir = std::env::var("GRAVES_DIR").unwrap();
        let g = Graves::load(&std::fs::read(format!("{dir}/graves.bin")).unwrap()).unwrap();
        let r: Vec<f32> = std::fs::read_to_string(format!("{dir}/ref.txt")).unwrap().split_whitespace().map(|x| x.parse().unwrap()).collect();
        let chars: Vec<usize> = "ab".chars().map(|c| code(c).unwrap()).chain([0]).collect();
        let mut st = St::zero();
        let mut z = Vec::new();
        let mut arg = 0;
        for inp in [[0.0, 0.0, 1.0], [0.5, -0.3, 0.0], [1.0, 0.2, 0.0]] {
            arg = g.step(inp, &mut st, &chars, &mut z);
        }
        let mut out = Vec::new();
        mv(&g.kg, &g.bg, &[&st.h3], &mut out);
        for i in 0..8 {
            assert!((st.h3[i] - r[i]).abs() < 1e-4, "h3 {i}: {} {}", st.h3[i], r[i]);
            assert!((out[i] - r[27 + i]).abs() < 1e-3, "z {i}");
        }
        for i in 0..K_ATT {
            assert!((st.kappa[i] - r[8 + i]).abs() < 1e-4, "kappa {i}");
        }
        assert_eq!(arg as f32, r[26]);
    }

    /// Righe di bella copia (testo e formule) col carattere chiaro e con la
    /// tua mano, in un PNG (`MANO_PNG`; serve `mano/graves.bin`).
    #[test]
    #[ignore]
    fn bella_copia_png() {
        let lines = [
            "Studiare la funzione $f(x) = \\frac{x^2-1}{x}$",
            "Il dominio è $x \\neq 0$ e la funzione è dispari",
            "$\\lim_{x \\to +\\infty} f(x) = +\\infty$ quindi non ci sono asintoti orizzontali",
        ];
        // la cartella dei dati con mano/ dentro
        if let Ok(d) = std::env::var("MANO_CWD") {
            std::env::set_current_dir(d).unwrap();
        }
        let words: Vec<String> = lines.iter().flat_map(|l| words_of(l)).collect();
        let t0 = std::time::Instant::now();
        warm(&words).unwrap();
        eprintln!("{} parole: {:?}", words.len(), t0.elapsed());
        let style = crate::handwrite::Style::Naturale;
        let mut boxes = Vec::new();
        for l in lines {
            boxes.push(crate::mathhand::line_box(style, l));
            boxes.push(using(true, || crate::mathhand::line_box(style, l)));
        }
        let s = 2.2;
        let mut pm = tiny_skia::Pixmap::new(1600, 110 * boxes.len() as u32 + 40).unwrap();
        pm.fill(tiny_skia::Color::WHITE);
        let mut paint = tiny_skia::Paint::default();
        paint.set_color_rgba8(20, 30, 60, 255);
        paint.anti_alias = true;
        let stroke = tiny_skia::Stroke { width: 2.2, line_cap: tiny_skia::LineCap::Round, line_join: tiny_skia::LineJoin::Round, ..Default::default() };
        for (r, b) in boxes.iter().enumerate() {
            let base = 80.0 + r as f32 * 110.0;
            for st in &b.strokes {
                let mut pb = tiny_skia::PathBuilder::new();
                pb.move_to(30.0 + st[0].0 * s, base + st[0].1 * s);
                for p in &st[1..] {
                    pb.line_to(30.0 + p.0 * s, base + p.1 * s);
                }
                if st.len() == 1 {
                    pb.line_to(30.5 + st[0].0 * s, base + st[0].1 * s);
                }
                if let Some(p) = pb.finish() {
                    pm.stroke_path(&p, &paint, &stroke, tiny_skia::Transform::identity(), None);
                }
            }
        }
        pm.save_png(std::env::var("MANO_PNG").unwrap()).unwrap();
    }

    /// Parole scritte con la tua mano, in un PNG (`GRAVES_DIR` con graves.bin
    /// e prima.txt; `MANO_PNG` il file).
    #[test]
    #[ignore]
    fn parole_png() {
        let dir = std::env::var("GRAVES_DIR").unwrap();
        let g = Graves::load(&std::fs::read(format!("{dir}/graves.bin")).unwrap()).unwrap();
        let t = std::fs::read_to_string(format!("{dir}/prima.txt")).unwrap();
        let mut lines = t.lines();
        let text = lines.next().unwrap().to_string();
        let prime: Vec<[f32; 3]> = lines
            .map(|l| {
                let v: Vec<f32> = l.split_whitespace().map(|x| x.parse().unwrap()).collect();
                [v[0], v[1], v[2]]
            })
            .collect();
        let t0 = std::time::Instant::now();
        let h = Hand::new(g, &prime, &text);
        eprintln!("prima: {:?}", t0.elapsed());
        let biases: Vec<f32> = std::env::var("MANO_BIASES").unwrap_or("1.0".into()).split(',').map(|x| x.parse().unwrap()).collect();
        let base_rows = ["Il dominio della funzione", "La derivata seconda è positiva"];
        let rows: Vec<(&str, f32)> = biases.iter().flat_map(|&b| base_rows.iter().map(move |r| (*r, b))).collect();
        let mut pm = tiny_skia::Pixmap::new(1400, 120 * rows.len() as u32 + 40).unwrap();
        pm.fill(tiny_skia::Color::WHITE);
        let mut paint = tiny_skia::Paint::default();
        paint.set_color_rgba8(20, 30, 60, 255);
        paint.anti_alias = true;
        let stroke = tiny_skia::Stroke { width: 2.5, line_cap: tiny_skia::LineCap::Round, line_join: tiny_skia::LineJoin::Round, ..Default::default() };
        for (r, (line, bias)) in rows.iter().enumerate() {
            let mut x = 30.0;
            let t1 = std::time::Instant::now();
            let words: Vec<WordInk> = line.split(' ').filter_map(|w| h.word_with(w, seed_of(w), *bias, TRIES)).collect();
            eprintln!("{line} ({bias}): {:?}", t1.elapsed());
            let mut xs: Vec<f32> = words.iter().map(|w| w.xh).collect();
            let s = 28.0 / quantile(&mut xs, 0.5);
            for w in &words {
                let base = 90.0 + r as f32 * 120.0;
                let wx = w.strokes.iter().flatten().map(|p| p.0).fold(0.0f32, f32::max);
                for st in &w.strokes {
                    let mut pb = tiny_skia::PathBuilder::new();
                    pb.move_to(x + st[0].0 * s, base + st[0].1 * s);
                    for p in &st[1..] {
                        pb.line_to(x + p.0 * s, base + p.1 * s);
                    }
                    if let Some(p) = pb.finish() {
                        pm.stroke_path(&p, &paint, &stroke, tiny_skia::Transform::identity(), None);
                    }
                }
                x += wx * s + 1.0 * 28.0;
            }
        }
        pm.save_png(std::env::var("MANO_PNG").unwrap()).unwrap();
    }
}
