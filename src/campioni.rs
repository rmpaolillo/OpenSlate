//! «La tua scrittura»: la bella copia con i tuoi segni, per chiunque.
//!
//! L'addestramento è a turni: ogni turno è una pagina con alcune righe da
//! scrivere col pennino (lettere, cifre, simboli separati da spazi, e alla
//! fine tre frasi). Si sa cosa si è chiesto e in che ordine, quindi ogni
//! segno si ritaglia e prende la sua etichetta da solo; la scala e la riga
//! di base vengono dalle misure del carattere chiaro (una «x» tua è alta
//! come la sua). Ogni lettera si fa scrivere più volte: le versioni diverse
//! danno varietà (due «a» vicine non sono uguali). Le frasi misurano lo
//! spazio fra le lettere e fra le parole; in corsivo le lettere si legano
//! con un tratto morbido dall'uscita dell'una all'ingresso dell'altra.
//!
//! Niente rete neurale, niente dati di altri: i segni sono tuoi e restano
//! nella cartella `mano/` dei dati (`corsivo.json`, `stampatello.json`).
//! Cifre e simboli valgono per tutti e due gli stili.

use crate::handwrite::{char_glyph, Style as Font};
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// Stili, modi, campioni

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Style {
    Corsivo,
    Stampatello,
}

impl Style {
    pub const ALL: [Style; 2] = [Style::Corsivo, Style::Stampatello];

    pub fn key(self) -> &'static str {
        match self {
            Style::Corsivo => "corsivo",
            Style::Stampatello => "stampatello",
        }
    }

    /// Il nome da mostrare (da tradurre).
    pub fn label(self) -> &'static str {
        match self {
            Style::Corsivo => "In corsivo",
            Style::Stampatello => "In stampatello",
        }
    }

    fn other(self) -> Style {
        match self {
            Style::Corsivo => Style::Stampatello,
            Style::Stampatello => Style::Corsivo,
        }
    }

    fn file(self) -> PathBuf {
        PathBuf::from(crate::mano::DIR).join(format!("{}.json", self.key()))
    }
}

/// Con che scrittura si fa la bella copia.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    /// Il carattere chiaro.
    Chiaro,
    /// I tuoi segni, addestrati.
    Scrittura(Style),
    /// La rete (solo dove c'è `mano/graves.bin`), coi tuoi segni per le
    /// formule.
    Mano,
}

impl Mode {
    pub fn parse(s: &str) -> Mode {
        match s.trim() {
            "corsivo" => Mode::Scrittura(Style::Corsivo),
            "stampatello" => Mode::Scrittura(Style::Stampatello),
            "mano" => Mode::Mano,
            _ => Mode::Chiaro,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Mode::Chiaro => "chiaro",
            Mode::Scrittura(s) => s.key(),
            Mode::Mano => "mano",
        }
    }
}

/// Un tuo segno: avanzamento e tratti nelle unità di Hershey (riga di base
/// a y = 0, y verso il basso, la «x» alta come quella del carattere chiaro).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Sample {
    pub c: char,
    pub adv: f32,
    pub strokes: Vec<Vec<[f32; 2]>>,
    #[serde(default)]
    pub round: u32,
}

/// L'addestramento di uno stile.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Store {
    #[serde(default)]
    pub samples: Vec<Sample>,
    /// Spazio fra le lettere di una parola e fra le parole (unità di Hershey).
    #[serde(default)]
    pub letter_gap: Option<f32>,
    #[serde(default)]
    pub word_gap: Option<f32>,
    /// Le frasi sono state scritte e misurate.
    #[serde(default)]
    pub phrases: bool,
    /// Hai detto «Va bene così».
    #[serde(default)]
    pub done: bool,
    #[serde(default)]
    pub rounds: u32,
    /// Unità della pagina → unità di Hershey (per i turni di sole frasi).
    #[serde(default)]
    pub unit: Option<f32>,
}

impl Store {
    pub fn count(&self, c: char) -> usize {
        self.samples.iter().filter(|s| s.c == c).count()
    }

    pub fn variants(&self, c: char) -> Vec<&Sample> {
        self.samples.iter().filter(|s| s.c == c).collect()
    }

    /// Quanto è fatto (0..1): versioni che ci sono su quelle da avere, e le
    /// frasi.
    pub fn coverage(&self) -> f32 {
        let (mut have, mut need) = (0usize, 0usize);
        for g in &GROUPS {
            for c in g.chars.chars() {
                have += self.count(c).min(g.target);
                need += g.target;
            }
        }
        let ph = PHRASES.len() * 3;
        have += if self.phrases { ph } else { 0 };
        need += ph;
        have as f32 / need as f32
    }
}

/// Un gruppo di segni e quante versioni servono di ognuno.
pub struct Group {
    pub chars: &'static str,
    pub target: usize,
}

pub const GROUPS: [Group; 7] = [
    Group { chars: "abcdefghijklmnopqrstuvwxyz", target: 3 },
    Group { chars: "ABCDEFGHIJKLMNOPQRSTUVWXYZ", target: 3 },
    Group { chars: "0123456789", target: 2 },
    Group { chars: "+−×=≠<>≤≥±()[]{}|/", target: 2 },
    Group { chars: "∫√∞→αβγδεθλμπσφωΔ", target: 2 },
    Group { chars: ".,;:!?'-", target: 2 },
    Group { chars: "àèéìòù", target: 2 },
];
/// I gruppi che valgono per tutti e due gli stili (cifre e simboli).
const SHARED: [usize; 4] = [2, 3, 4, 5];
/// Le frasi dell'ultimo turno (la prima, con le sole lettere della rete, dà
/// anche lo stile alla rete dove c'è).
pub const PHRASES: [&str; 3] = ["Studiare il grafico della seguente funzione", "La derivata prima si annulla in un solo punto", "quindi la funzione ha un minimo assoluto"];
const PER_ROW: usize = 13;
const ROWS_PER_ROUND: usize = 8;
/// Margine a sinistra e a destra di ogni segno (unità di Hershey).
const SIDE: f32 = 1.5;

// ---------------------------------------------------------------------------
// I turni

#[derive(Clone, Debug, PartialEq)]
pub enum Row {
    Signs(Vec<char>),
    Phrase(String),
}

impl Row {
    /// Come si mostra da copiare.
    pub fn text(&self) -> String {
        match self {
            Row::Signs(cs) => cs.iter().map(|c| c.to_string()).collect::<Vec<_>>().join("  "),
            Row::Phrase(p) => p.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Round {
    pub number: u32,
    pub rows: Vec<Row>,
}

/// Divide `cs` in righe quasi uguali di al più `PER_ROW` segni.
fn chunks(cs: &[char]) -> Vec<Vec<char>> {
    if cs.is_empty() {
        return Vec::new();
    }
    let n = cs.len().div_ceil(PER_ROW);
    let size = cs.len().div_ceil(n);
    cs.chunks(size).map(|c| c.to_vec()).collect()
}

/// Il prossimo turno: quello che manca (ogni segno tante volte quante ne
/// servono, `extra` versioni in più per le lettere), poi le frasi. None se
/// non manca niente.
pub fn next_round(store: &Store, extra: usize) -> Option<Round> {
    let mut rows = Vec::new();
    for (gi, g) in GROUPS.iter().enumerate() {
        let want = g.target + if gi < 2 { extra } else { 0 };
        let cs: Vec<char> = g.chars.chars().collect();
        let passes = cs.iter().map(|&c| want.saturating_sub(store.count(c))).max().unwrap_or(0);
        for p in 0..passes {
            let lacking: Vec<char> = cs.iter().copied().filter(|&c| store.count(c) + p < want).collect();
            rows.extend(chunks(&lacking).into_iter().map(Row::Signs));
        }
    }
    if !store.phrases {
        rows.extend(PHRASES.iter().map(|p| Row::Phrase(p.to_string())));
    }
    if rows.is_empty() {
        return None;
    }
    rows.truncate(ROWS_PER_ROUND);
    Some(Round { number: store.rounds + 1, rows })
}

// ---------------------------------------------------------------------------
// La lettura di un turno

/// Una riga letta: i segni (con le etichette) e quali sembrano diversi
/// dalle altre versioni; o perché non va.
#[derive(Clone, Debug)]
pub struct RowRead {
    pub row: Row,
    pub samples: Vec<Sample>,
    pub odd: Vec<bool>,
    /// Il problema (in italiano, coi numeri dentro: da tradurre con `tf`).
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct RoundRead {
    pub rows: Vec<RowRead>,
    pub error: Option<String>,
    pub letter_gap: Option<f32>,
    pub word_gap: Option<f32>,
    pub unit: Option<f32>,
    /// La prima frase (tratti del pennino), per lo stile della rete.
    pub prime: Option<Vec<Vec<(f32, f32)>>>,
}

fn bb(pts: impl Iterator<Item = (f32, f32)>) -> (f32, f32, f32, f32) {
    pts.fold((f32::MAX, f32::MIN, f32::MAX, f32::MIN), |b, p| (b.0.min(p.0), b.1.max(p.0), b.2.min(p.1), b.3.max(p.1)))
}

fn median(v: &mut [f32]) -> Option<f32> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(f32::total_cmp);
    let n = v.len();
    Some(if n % 2 == 1 { v[n / 2] } else { (v[n / 2 - 1] + v[n / 2]) / 2.0 })
}

/// Divide i tratti in righe (dall'alto): un tratto va nella riga se il suo
/// centro è dentro la riga o se la tocca per metà della sua altezza; le
/// «righe» minuscole (il puntino di una i sopra la riga) vanno con la più
/// vicina; se le righe sono più di `want`, si uniscono quelle più vicine
/// (gli accenti sopra una riga di lettere).
pub(crate) fn rows_of(strokes: &[Vec<(f32, f32)>], want: usize) -> Vec<Vec<usize>> {
    let boxes: Vec<(f32, f32, f32, f32)> = strokes.iter().map(|s| bb(s.iter().cloned())).collect();
    let live: Vec<usize> = (0..strokes.len()).filter(|&i| !strokes[i].is_empty()).collect();
    // prima i tratti alti (le aste, i corpi delle lettere): i tagli, i
    // puntini e gli accenti si aggiungono dopo alla riga più vicina
    let mut hs: Vec<f32> = live.iter().map(|&i| boxes[i].3 - boxes[i].2).collect();
    hs.sort_by(f32::total_cmp);
    let tall = hs.get(hs.len() * 3 / 4).copied().unwrap_or(0.0) * 0.4;
    let (mut order, small): (Vec<usize>, Vec<usize>) = live.iter().partition(|&&i| boxes[i].3 - boxes[i].2 >= tall);
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
    for i in small {
        let cy = (boxes[i].2 + boxes[i].3) / 2.0;
        let d = |x: &(f32, f32, Vec<usize>)| if cy < x.0 { x.0 - cy } else if cy > x.1 { cy - x.1 } else { 0.0 };
        match (0..rows.len()).min_by(|&a, &b| d(&rows[a]).total_cmp(&d(&rows[b]))) {
            Some(j) => rows[j].2.push(i),
            None => rows.push((boxes[i].2, boxes[i].3, vec![i])),
        }
    }
    while rows.len() >= 2 {
        let mut hs: Vec<f32> = rows.iter().map(|r| r.1 - r.0).collect();
        let med = median(&mut hs).unwrap_or(0.0);
        let Some(k) = rows.iter().position(|r| r.1 - r.0 < 0.35 * med) else { break };
        let r = rows.remove(k);
        let cy = (r.0 + r.1) / 2.0;
        let d = |x: &(f32, f32, Vec<usize>)| if cy < x.0 { x.0 - cy } else if cy > x.1 { cy - x.1 } else { 0.0 };
        let j = (0..rows.len()).min_by(|&a, &b| d(&rows[a]).total_cmp(&d(&rows[b]))).unwrap();
        rows[j].0 = rows[j].0.min(r.0);
        rows[j].1 = rows[j].1.max(r.1);
        rows[j].2.extend(r.2);
    }
    while rows.len() > want && rows.len() > 1 {
        let gap = |k: usize| (rows[k + 1].0 - rows[k].1).max(0.0) / (rows[k].1 - rows[k].0).min(rows[k + 1].1 - rows[k + 1].0).max(1e-6);
        let k = (0..rows.len() - 1).min_by(|&a, &b| gap(a).total_cmp(&gap(b))).unwrap();
        let r = rows.remove(k + 1);
        rows[k].1 = rows[k].1.max(r.1);
        rows[k].2.extend(r.2);
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
pub(crate) fn glyphs_of(strokes: &[Vec<(f32, f32)>], row: &[usize], want: usize) -> Vec<Vec<usize>> {
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

/// Dove sta in alto e in basso il carattere chiaro (unità di Hershey).
fn reference(c: char) -> Option<(f32, f32)> {
    let g = char_glyph(Font::Naturale, c)?;
    let (_, _, t, b) = bb(g.strokes.iter().flatten().cloned());
    (b - t > 3.0).then_some((t, b))
}

fn ink_size(s: &Sample) -> (f32, f32) {
    let (x0, x1, y0, y1) = bb(s.strokes.iter().flatten().map(|p| (p[0], p[1])));
    ((x1 - x0).max(0.0), (y1 - y0).max(0.0))
}

/// I punti di un segno lungo i tratti, nel quadrato unitario (per
/// confrontare le forme).
fn shape_points(s: &Sample) -> Vec<(f32, f32)> {
    let (x0, x1, y0, y1) = bb(s.strokes.iter().flatten().map(|p| (p[0], p[1])));
    let k = (x1 - x0).max(y1 - y0).max(1e-3);
    let mut out = Vec::new();
    for st in &s.strokes {
        for w in st.windows(2) {
            let n = (((w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1])) / (0.04 * k)).ceil().max(1.0) as usize;
            for j in 0..n {
                let t = j as f32 / n as f32;
                out.push((((w[0][0] + (w[1][0] - w[0][0]) * t) - x0) / k, ((w[0][1] + (w[1][1] - w[0][1]) * t) - y0) / k));
            }
        }
        if let Some(p) = st.last() {
            out.push(((p[0] - x0) / k, (p[1] - y0) / k));
        }
    }
    out
}

/// Distanza fra due forme (media delle distanze dal punto più vicino, nei
/// due versi): non dipende dall'ordine dei tratti.
fn shape_dist(a: &[(f32, f32)], b: &[(f32, f32)]) -> f32 {
    if a.is_empty() || b.is_empty() {
        return 1.0;
    }
    let near = |p: &(f32, f32), q: &[(f32, f32)]| q.iter().map(|r| (p.0 - r.0).hypot(p.1 - r.1)).fold(f32::MAX, f32::min);
    let ab = a.iter().map(|p| near(p, b)).sum::<f32>() / a.len() as f32;
    let ba = b.iter().map(|p| near(p, a)).sum::<f32>() / b.len() as f32;
    (ab + ba) / 2.0
}

/// Un segno molto diverso dalle altre versioni (grandezza o forma).
fn odd(s: &Sample, others: &[&Sample]) -> bool {
    if others.len() < 2 {
        return false;
    }
    let (w, h) = ink_size(s);
    let mut ws: Vec<f32> = others.iter().map(|o| ink_size(o).0).collect();
    let mut hs: Vec<f32> = others.iter().map(|o| ink_size(o).1).collect();
    let (mw, mh) = (median(&mut ws).unwrap_or(w), median(&mut hs).unwrap_or(h));
    if (h - mh).abs() > 0.45 * mh.max(4.0) || (w - mw).abs() > 0.6 * mw.max(4.0) {
        return true;
    }
    let me = shape_points(s);
    let best = others.iter().map(|o| shape_dist(&me, &shape_points(o))).fold(f32::MAX, f32::min);
    best > 0.15
}

/// La pagina di un turno (tratti nell'ordine in cui li hai scritti, x verso
/// destra e y verso il basso, unità della pagina) → i segni, riga per riga.
pub fn read_round(round: &Round, strokes: &[Vec<(f32, f32)>], store: &Store) -> RoundRead {
    let rows = rows_of(strokes, round.rows.len());
    if rows.len() != round.rows.len() {
        return RoundRead {
            error: Some(format!("Ho trovato {} righe scritte, ne servono {}: una riga per ogni riga del turno, ben staccate.", rows.len(), round.rows.len())),
            ..Default::default()
        };
    }
    // i segni di ogni riga (indici dei tratti) e le parole delle frasi
    let mut parts: Vec<Result<Vec<Vec<usize>>, String>> = Vec::new();
    for (r, spec) in round.rows.iter().enumerate() {
        parts.push(match spec {
            Row::Signs(cs) => {
                let gs = glyphs_of(strokes, &rows[r], cs.len());
                if gs.len() == cs.len() {
                    Ok(gs)
                } else {
                    Err(format!("Riga {}: ho trovato {} segni, ne servono {} (lascia un po' di spazio fra un segno e l'altro).", r + 1, gs.len(), cs.len()))
                }
            }
            Row::Phrase(p) => {
                let n = p.split_whitespace().count();
                let ws = glyphs_of(strokes, &rows[r], n);
                if ws.len() == n {
                    Ok(ws)
                } else {
                    Err(format!("Riga {}: non riesco a separare le parole (lascia più spazio fra una parola e l'altra).", r + 1))
                }
            }
        });
    }
    // la scala: ref = a·y + b ai minimi quadrati, riga per riga, coi segni
    // che il carattere chiaro ha (la a è la mediana delle righe)
    let ink = |g: &[usize]| bb(g.iter().flat_map(|&i| strokes[i].iter().cloned()));
    let mut fits: Vec<Vec<(f32, f32)>> = vec![Vec::new(); round.rows.len()];
    let mut slopes: Vec<f32> = Vec::new();
    for (r, spec) in round.rows.iter().enumerate() {
        let (Row::Signs(cs), Ok(gs)) = (spec, &parts[r]) else { continue };
        for (c, g) in cs.iter().zip(gs) {
            let Some((rt, rb)) = reference(*c) else { continue };
            let (_, _, t, b) = ink(g);
            fits[r].push((t, rt));
            fits[r].push((b, rb));
        }
        let pairs = &fits[r];
        if pairs.len() >= 4 {
            let n = pairs.len() as f32;
            let (mx, my) = (pairs.iter().map(|p| p.0).sum::<f32>() / n, pairs.iter().map(|p| p.1).sum::<f32>() / n);
            let sxx: f32 = pairs.iter().map(|p| (p.0 - mx).powi(2)).sum();
            let sxy: f32 = pairs.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
            if sxx > 0.0 && sxy > 0.0 {
                slopes.push(sxy / sxx);
            }
        }
    }
    let a = median(&mut slopes).or(store.unit);
    let mut out = RoundRead { unit: median(&mut slopes), ..Default::default() };
    let mut fresh: Vec<Sample> = Vec::new();
    for (r, spec) in round.rows.iter().enumerate() {
        let mut rr = RowRead { row: spec.clone(), samples: Vec::new(), odd: Vec::new(), error: parts[r].as_ref().err().cloned() };
        if let (Row::Signs(cs), Ok(gs)) = (spec, &parts[r]) {
            match a {
                None => rr.error = Some("Non riesco a misurare i segni di questo turno.".into()),
                Some(a) => {
                    let b = if fits[r].is_empty() {
                        // niente segni di riferimento: il fondo dei segni sulla riga di base
                        let mut bots: Vec<f32> = gs.iter().map(|g| ink(g).3).collect();
                        -a * median(&mut bots).unwrap_or(0.0)
                    } else {
                        fits[r].iter().map(|p| p.1 - a * p.0).sum::<f32>() / fits[r].len() as f32
                    };
                    for (c, g) in cs.iter().zip(gs) {
                        let (x0, x1, _, _) = ink(g);
                        let st: Vec<Vec<[f32; 2]>> = g.iter().map(|&i| strokes[i].iter().map(|p| [a * (p.0 - x0) + SIDE, a * p.1 + b]).collect()).collect();
                        let s = Sample { c: *c, adv: a * (x1 - x0) + 2.0 * SIDE, strokes: st, round: round.number };
                        let others: Vec<&Sample> = store.samples.iter().chain(fresh.iter()).filter(|o| o.c == *c).collect();
                        rr.odd.push(odd(&s, &others));
                        fresh.push(s.clone());
                        rr.samples.push(s);
                    }
                }
            }
        }
        out.rows.push(rr);
    }
    // le frasi: spazio fra le lettere (larghezza vera della parola meno
    // quella dei segni) e fra le parole
    let ink_w = |c: char| -> f32 {
        let mut ws: Vec<f32> = store.samples.iter().chain(fresh.iter()).filter(|s| s.c == c).map(|s| ink_size(s).0).collect();
        median(&mut ws).unwrap_or_else(|| char_glyph(Font::Naturale, c).map_or(8.0, |g| bb(g.strokes.iter().flatten().cloned()).1 - bb(g.strokes.iter().flatten().cloned()).0))
    };
    let (mut lgaps, mut wgaps) = (Vec::new(), Vec::new());
    for (r, spec) in round.rows.iter().enumerate() {
        let (Row::Phrase(p), Ok(ws)) = (spec, &parts[r]) else { continue };
        if p == PHRASES[0] {
            out.prime = Some(rows[r].iter().map(|&i| strokes[i].clone()).collect());
        }
        let Some(a) = a else { continue };
        let boxes: Vec<(f32, f32, f32, f32)> = ws.iter().map(|g| ink(g)).collect();
        for (w, bx) in p.split_whitespace().zip(&boxes) {
            let n = w.chars().count();
            if n >= 2 {
                let synth: f32 = w.chars().map(ink_w).sum();
                lgaps.push((a * (bx.1 - bx.0) - synth) / (n - 1) as f32);
            }
        }
        for k in 1..boxes.len() {
            wgaps.push(a * (boxes[k].0 - boxes[k - 1].1));
        }
    }
    out.letter_gap = median(&mut lgaps).map(|g| g.clamp(-4.0, 10.0));
    out.word_gap = median(&mut wgaps).map(|g| g.clamp(5.0, 30.0));
    out
}

/// Aggiunge al tuo addestramento quello che hai tenuto di un turno
/// (`drop`: i segni scartati, come (riga, segno)).
pub fn commit(store: &mut Store, read: &RoundRead, drop: &HashSet<(usize, usize)>) {
    for (r, row) in read.rows.iter().enumerate() {
        if row.error.is_some() {
            continue;
        }
        for (k, s) in row.samples.iter().enumerate() {
            if !drop.contains(&(r, k)) {
                store.samples.push(s.clone());
            }
        }
    }
    if read.letter_gap.is_some() {
        store.letter_gap = read.letter_gap;
        store.word_gap = read.word_gap.or(store.word_gap);
        store.phrases = true;
    }
    if read.unit.is_some() {
        store.unit = read.unit;
    }
    store.rounds += 1;
}

// ---------------------------------------------------------------------------
// Salvataggio

static STORES: Mutex<Option<HashMap<Style, Store>>> = Mutex::new(None);

fn stores<R>(f: impl FnOnce(&mut HashMap<Style, Store>) -> R) -> R {
    let mut g = STORES.lock().unwrap();
    let m = g.get_or_insert_with(|| {
        let mut m = HashMap::new();
        if !cfg!(test) {
            for st in Style::ALL {
                if let Some(s) = std::fs::read_to_string(st.file()).ok().and_then(|t| serde_json::from_str::<Store>(&t).ok()) {
                    m.insert(st, s);
                }
            }
        }
        m
    });
    f(m)
}

/// L'addestramento di uno stile (vuoto se non c'è).
pub fn load(style: Style) -> Store {
    stores(|m| m.get(&style).cloned().unwrap_or_default())
}

/// Per cominciare (o continuare) uno stile: se è nuovo, cifre e simboli
/// vengono dall'altro stile, se ci sono.
pub fn start(style: Style) -> Store {
    let mut s = load(style);
    if s.samples.is_empty() {
        let other = load(style.other());
        let shared: String = SHARED.iter().map(|&g| GROUPS[g].chars).collect();
        s.samples = other.samples.into_iter().filter(|x| shared.contains(x.c)).collect();
        s.unit = s.unit.or(other.unit);
    }
    s
}

pub fn save(style: Style, store: &Store) -> anyhow::Result<()> {
    stores(|m| m.insert(style, store.clone()));
    if cfg!(test) {
        return Ok(());
    }
    std::fs::create_dir_all(crate::mano::DIR)?;
    crate::web::write_atomic(&style.file(), serde_json::to_string(store)?.as_bytes())?;
    Ok(())
}

/// Gli stili addestrati (hai detto «Va bene così»).
pub fn trained() -> Vec<Style> {
    Style::ALL.into_iter().filter(|&s| load(s).done).collect()
}

// ---------------------------------------------------------------------------
// La scrittura: il segnale per `mathhand` e le parole composte

thread_local! {
    static MODE: Cell<Mode> = const { Cell::new(Mode::Chiaro) };
    static CUR: RefCell<Option<(Style, Store)>> = const { RefCell::new(None) };
    static TURN: RefCell<HashMap<char, usize>> = RefCell::new(HashMap::new());
}

/// Esegue `f` (che impagina con `mathhand`) con la scrittura `mode`.
pub fn using<R>(mode: Mode, f: impl FnOnce() -> R) -> R {
    let store = match mode {
        Mode::Chiaro => None,
        Mode::Scrittura(st) => Some((st, load(st))),
        Mode::Mano => Style::ALL.into_iter().map(|st| (st, load(st))).find(|(_, s)| !s.samples.is_empty()),
    };
    using_store(mode, store, f)
}

/// Come `using`, con un addestramento dato (l'anteprima, le prove).
pub fn using_store<R>(mode: Mode, store: Option<(Style, Store)>, f: impl FnOnce() -> R) -> R {
    let mode = match mode {
        Mode::Mano if !(crate::mano::available() || cfg!(test)) => Mode::Chiaro,
        m => m,
    };
    let prev_mode = MODE.with(|m| m.replace(mode));
    let prev_store = CUR.with(|c| c.replace(store));
    let prev_turn = TURN.with(|t| std::mem::take(&mut *t.borrow_mut()));
    let r = f();
    MODE.with(|m| m.set(prev_mode));
    CUR.with(|c| *c.borrow_mut() = prev_store);
    TURN.with(|t| *t.borrow_mut() = prev_turn);
    r
}

pub fn mode() -> Mode {
    MODE.with(|m| m.get())
}

type Ink = Vec<Vec<(f32, f32)>>;

fn to_ink(s: &Sample) -> Ink {
    s.strokes.iter().map(|st| st.iter().map(|p| (p[0], p[1])).collect()).collect()
}

const ACCENTS: [(char, char, bool); 6] = [('à', 'a', true), ('è', 'e', true), ('é', 'e', false), ('ì', 'i', true), ('ò', 'o', true), ('ù', 'u', true)];

/// Un segno per `c`: la versione `k` delle tue (in giro), o la lettera con
/// l'accento aggiunto, o il carattere chiaro. Il booleano: è un segno tuo.
fn pick(store: &Store, c: char, k: usize) -> Option<(f32, Ink, bool)> {
    let v = store.variants(c);
    if !v.is_empty() {
        let s = v[k % v.len()];
        return Some((s.adv, to_ink(s), true));
    }
    if let Some(&(_, base, grave)) = ACCENTS.iter().find(|a| a.0 == c) {
        let (adv, mut ink, mine) = pick(store, base, k)?;
        let (x0, x1, top, _) = bb(ink.iter().flatten().cloned());
        let (cx, h) = ((x0 + x1) / 2.0, 6.0);
        let y = top.min(-12.0) - 2.0;
        ink.push(if grave { vec![(cx - 0.4 * h, y - h), (cx + 0.2 * h, y)] } else { vec![(cx - 0.2 * h, y), (cx + 0.4 * h, y - h)] });
        return Some((adv, ink, mine));
    }
    char_glyph(Font::Naturale, c).map(|g| (g.adv, g.strokes, false))
}

/// Il tuo segno per `c` nelle formule (le versioni a giro), se c'è.
pub fn glyph(c: char) -> Option<(f32, Ink)> {
    CUR.with(|cur| {
        let cur = cur.borrow();
        let (_, store) = cur.as_ref()?;
        if store.count(c) == 0 && !ACCENTS.iter().any(|a| a.0 == c && store.count(a.1) > 0) {
            return None;
        }
        let k = TURN.with(|t| {
            let mut t = t.borrow_mut();
            let e = t.entry(c).or_insert(0);
            *e += 1;
            *e - 1
        });
        pick(store, c, k).map(|(adv, ink, _)| (adv, ink))
    })
}

fn hash(s: &str, k: u64) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ k.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    for b in s.bytes() {
        h = (h ^ b as u64).wrapping_mul(0x100_0000_01b3);
    }
    h ^ (h >> 29)
}

/// Un numero fra -1 e 1 (sempre lo stesso per la stessa parola e posizione).
fn wobble(word: &str, i: usize, k: u64) -> f32 {
    (hash(word, (i as u64) << 8 | k) % 2001) as f32 / 1000.0 - 1.0
}

/// I tratti lunghi (non puntini né accenti) di un segno.
fn long_strokes(ink: &Ink) -> Vec<usize> {
    (0..ink.len())
        .filter(|&i| {
            let (x0, x1, y0, y1) = bb(ink[i].iter().cloned());
            ink[i].len() >= 2 && (x1 - x0).max(y1 - y0) >= 3.0
        })
        .collect()
}

/// Il tratto morbido da `a` (uscita, che va verso `ta`) a `b` (ingresso,
/// che va verso `tb`).
fn bridge(a: (f32, f32), ta: (f32, f32), b: (f32, f32), tb: (f32, f32)) -> Vec<(f32, f32)> {
    let d = (b.0 - a.0).hypot(b.1 - a.1);
    let norm = |v: (f32, f32)| {
        let l = v.0.hypot(v.1).max(1e-6);
        (v.0 / l, v.1 / l)
    };
    let (ta, tb) = (norm(ta), norm(tb));
    let p1 = (a.0 + ta.0 * d * 0.4, a.1 + ta.1 * d * 0.4);
    let p2 = (b.0 - tb.0 * d * 0.4, b.1 - tb.1 * d * 0.4);
    (1..7)
        .map(|j| {
            let t = j as f32 / 7.0;
            let u = 1.0 - t;
            (
                u * u * u * a.0 + 3.0 * u * u * t * p1.0 + 3.0 * u * t * t * p2.0 + t * t * t * b.0,
                u * u * u * a.1 + 3.0 * u * u * t * p1.1 + 3.0 * u * t * t * p2.1 + t * t * t * b.1,
            )
        })
        .collect()
}

/// Una parola coi tuoi segni: x da 0, riga di base a 0. Ritorna i tratti e
/// la larghezza.
fn compose_word(store: &Store, style: Style, word: &str) -> (Ink, f32) {
    let gap = store.letter_gap.unwrap_or(if style == Style::Corsivo { 1.0 } else { 3.0 });
    let mut out: Ink = Vec::new();
    let mut cursor = 0.0f32;
    // il tratto (in `out`) che finisce con l'uscita della lettera prima, se
    // si può legare
    let mut exit: Option<usize> = None;
    let mut seen: HashMap<char, usize> = HashMap::new();
    for (i, c) in word.chars().enumerate() {
        let n = seen.entry(c).or_insert(0);
        let k = (hash(word, i as u64) as usize).wrapping_add(*n);
        *n += 1;
        let Some((_, ink, mine)) = pick(store, c, k) else {
            cursor += 8.0;
            exit = None;
            continue;
        };
        let (x0, x1, _, _) = bb(ink.iter().flatten().cloned());
        if x0 > x1 {
            cursor += 8.0;
            exit = None;
            continue;
        }
        let s = 1.0 + 0.03 * wobble(word, i, 1);
        let dy = 0.4 * wobble(word, i, 2);
        let ink: Ink = ink.iter().map(|st| st.iter().map(|p| (cursor + (p.0 - x0) * s, p.1 * s + dy)).collect()).collect();
        let joinable = style == Style::Corsivo && mine && c.is_alphabetic() && c.is_lowercase();
        let long = long_strokes(&ink);
        let mut map: Vec<Option<usize>> = vec![None; ink.len()];
        if let (true, Some(pi)) = (joinable, exit) {
            if let Some(&e) = long.iter().min_by(|&&a, &&b| ink[a][0].0.total_cmp(&ink[b][0].0)) {
                let a = *out[pi].last().unwrap();
                let b = ink[e][0];
                // fino a una lettera e mezza: le lettere tonde (a, d, o) cominciano
                // in alto a destra del giro
                if (-6.0..=22.0).contains(&(b.0 - a.0)) {
                    let pa = out[pi][out[pi].len().saturating_sub(2)];
                    let ta = (a.0 - pa.0, a.1 - pa.1);
                    let tb = (ink[e][1].0 - b.0, ink[e][1].1 - b.1);
                    let br = bridge(a, ta, b, tb);
                    out[pi].extend(br);
                    out[pi].extend(ink[e].iter().cloned());
                    map[e] = Some(pi);
                }
            }
        }
        for (j, st) in ink.iter().enumerate() {
            if map[j].is_none() {
                out.push(st.clone());
                map[j] = Some(out.len() - 1);
            }
        }
        exit = if joinable { long.iter().max_by(|&&a, &&b| ink[a].last().unwrap().0.total_cmp(&ink[b].last().unwrap().0)).and_then(|&x| map[x]) } else { None };
        // l'uscita deve essere la fine del tratto in `out`
        if let Some(pi) = exit {
            let lx = long.iter().map(|&x| ink[x].last().unwrap().0).fold(f32::MIN, f32::max);
            if (out[pi].last().unwrap().0 - lx).abs() > 1e-3 {
                exit = None;
            }
        }
        cursor += (x1 - x0) * s + gap;
    }
    let w = (cursor - gap).max(0.0);
    (out, w)
}

/// Testo (senza formule) coi tuoi segni: larghezza e tratti, o None se non
/// si scrive con la tua scrittura.
pub fn text_box(text: &str) -> Option<(f32, Ink)> {
    CUR.with(|cur| {
        let cur = cur.borrow();
        let (style, store) = cur.as_ref()?;
        let space = store.word_gap.unwrap_or(10.0);
        let mut out: Ink = Vec::new();
        let mut x = 0.0;
        for (k, w) in text.split(' ').enumerate() {
            if k > 0 {
                x += space;
            }
            if w.is_empty() {
                continue;
            }
            let (ink, wd) = compose_word(store, *style, w);
            out.extend(ink.into_iter().map(|st| st.into_iter().map(|p| (p.0 + x, p.1)).collect()));
            x += wd;
        }
        Some((x, out))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un segno finto: un quadratino alto `h`, largo `w`, con la base a 0.
    fn square(c: char, w: f32, h: f32) -> Sample {
        Sample { c, adv: w + 3.0, strokes: vec![vec![[1.5, 0.0], [1.5 + w, 0.0], [1.5 + w, -h], [1.5, -h], [1.5, 0.0]]], round: 1 }
    }

    #[test]
    fn primo_turno_e_poi() {
        let s = Store::default();
        let r = next_round(&s, 0).unwrap();
        assert_eq!(r.number, 1);
        assert_eq!(r.rows.len(), ROWS_PER_ROUND);
        // le minuscole tre volte, in due righe da 13
        assert_eq!(r.rows[0], Row::Signs("abcdefghijklm".chars().collect()));
        assert_eq!(r.rows[5], Row::Signs("nopqrstuvwxyz".chars().collect()));
        // tutto fatto: niente turni, salvo le versioni in più
        let mut full = Store { phrases: true, ..Default::default() };
        for g in &GROUPS {
            for c in g.chars.chars() {
                for _ in 0..g.target {
                    full.samples.push(square(c, 5.0, 10.0));
                }
            }
        }
        assert!((full.coverage() - 1.0).abs() < 1e-6);
        assert!(next_round(&full, 0).is_none());
        let more = next_round(&full, 1).unwrap();
        assert!(more.rows.iter().all(|r| matches!(r, Row::Signs(cs) if cs.iter().all(|c| c.is_alphabetic()))));
        // chi ha solo i simboli: le frasi alla fine
        let mut almost = full.clone();
        almost.phrases = false;
        assert!(matches!(next_round(&almost, 0).unwrap().rows[0], Row::Phrase(_)));
        assert_eq!(Mode::parse(Mode::Scrittura(Style::Corsivo).key()), Mode::Scrittura(Style::Corsivo));
        assert_eq!(Mode::parse("qualcosa"), Mode::Chiaro);
    }

    /// Una pagina finta per un turno: quadratini per i segni (le maiuscole
    /// più alte), onde per le parole delle frasi.
    fn page(round: &Round) -> Vec<Vec<(f32, f32)>> {
        let mut strokes = Vec::new();
        for (r, row) in round.rows.iter().enumerate() {
            let y = 0.1 + r as f32 * 0.1;
            match row {
                Row::Signs(cs) => {
                    for (k, c) in cs.iter().enumerate() {
                        let x = 0.05 + k as f32 * 0.06;
                        let h = if c.is_uppercase() || c.is_ascii_digit() { 0.03 } else { 0.02 };
                        strokes.push(vec![(x, y), (x + 0.02, y), (x + 0.02, y - h), (x, y - h), (x, y)]);
                        if *c == 'i' {
                            strokes.push(vec![(x + 0.005, y - h - 0.006), (x + 0.012, y - h - 0.006)]);
                        }
                    }
                }
                Row::Phrase(p) => {
                    let mut x = 0.05;
                    for w in p.split_whitespace() {
                        let len = 0.012 * w.chars().count() as f32;
                        strokes.push((0..30).map(|i| (x + len * i as f32 / 29.0, y - 0.02 * ((i as f32) * 0.6).sin().abs())).collect());
                        x += len + 0.02;
                    }
                }
            }
        }
        strokes
    }

    #[test]
    fn turni_letti_e_tenuti() {
        let mut store = Store::default();
        let r1 = next_round(&store, 0).unwrap();
        let read = read_round(&r1, &page(&r1), &store);
        assert!(read.error.is_none());
        assert!(read.rows.iter().all(|r| r.error.is_none()), "{:?}", read.rows.iter().map(|r| &r.error).collect::<Vec<_>>());
        let x = read.rows[0].samples.iter().find(|s| s.c == 'm').unwrap();
        let (_, _, t, b) = bb(x.strokes.iter().flatten().map(|p| (p[0], p[1])));
        // riga di base vicina a 0, corpo come quello del carattere chiaro
        assert!(b.abs() < 4.0 && t < -5.0, "{t} {b}");
        // uno scartato
        let drop: HashSet<(usize, usize)> = [(0, 0)].into_iter().collect();
        commit(&mut store, &read, &drop);
        assert_eq!(store.count('a'), 2);
        assert_eq!(store.count('b'), 3);
        assert_eq!(store.rounds, 1);
        // righe che mancano: messaggio, niente segni
        let short = page(&r1);
        let bad = read_round(&r1, &short[..short.len() - 13], &store);
        assert!(bad.error.is_some());
        // fino alla fine, frasi comprese
        let mut n = 0;
        while let Some(r) = next_round(&store, 0) {
            let read = read_round(&r, &page(&r), &store);
            assert!(read.error.is_none(), "{:?}", read.error);
            commit(&mut store, &read, &HashSet::new());
            n += 1;
            assert!(n < 12);
        }
        assert!(store.phrases && store.letter_gap.is_some() && store.word_gap.is_some());
        assert!((store.coverage() - 1.0).abs() < 1e-6);
        // si salva e si rilegge uguale
        let t = serde_json::to_string(&store).unwrap();
        assert_eq!(serde_json::from_str::<Store>(&t).unwrap(), store);
    }

    #[test]
    fn segno_diverso_segnalato() {
        let a = square('a', 6.0, 8.0);
        let b = square('a', 6.2, 8.1);
        let c = square('a', 5.9, 7.9);
        assert!(!odd(&c, &[&a, &b]));
        assert!(odd(&square('a', 6.0, 20.0), &[&a, &b]));
        // stessa grandezza, forma diversa: una diagonale
        let x = Sample { c: 'a', adv: 9.0, strokes: vec![vec![[1.5, 0.0], [7.5, -8.0]]], round: 1 };
        assert!(odd(&x, &[&a, &b]));
    }

    /// In corsivo le lettere tue si legano: meno tratti che lettere.
    #[test]
    fn corsivo_legato() {
        let mut st = Store { letter_gap: Some(1.0), word_gap: Some(10.0), ..Default::default() };
        // una «n» corsiva: un tratto solo che entra in basso a sinistra ed esce in basso a destra
        for c in "lnu".chars() {
            st.samples.push(Sample { c, adv: 12.0, strokes: vec![vec![[1.5, 0.0], [4.0, -9.0], [7.0, -9.0], [9.0, 0.0], [10.5, -1.0]]], round: 1 });
        }
        let (w, ink) = using_store(Mode::Scrittura(Style::Corsivo), Some((Style::Corsivo, st.clone())), || text_box("nun lu")).unwrap();
        assert_eq!(ink.len(), 2, "una parola, un tratto");
        assert!(w > 30.0);
        // in stampatello no
        let (_, ink) = using_store(Mode::Scrittura(Style::Stampatello), Some((Style::Stampatello, st.clone())), || text_box("nun lu")).unwrap();
        assert_eq!(ink.len(), 5);
        // l'accento su una lettera tua; nelle formule le versioni a giro
        st.samples.push(Sample { c: 'e', adv: 9.0, strokes: vec![vec![[1.5, -4.0], [7.0, -4.0], [4.0, -8.0], [1.5, 0.0], [7.0, 0.0]]], round: 1 });
        st.samples.push(Sample { c: 'e', adv: 9.5, strokes: vec![vec![[1.5, -4.0], [7.5, -4.0], [4.0, -8.5], [1.5, 0.0], [7.5, 0.0]]], round: 1 });
        using_store(Mode::Scrittura(Style::Corsivo), Some((Style::Corsivo, st)), || {
            let (_, g) = glyph('è').unwrap();
            assert_eq!(g.len(), 2);
            let a = glyph('e').unwrap().0;
            let b = glyph('e').unwrap().0;
            assert_ne!(a, b);
            assert!(glyph('Q').is_none());
        });
    }

    /// Un addestramento intero «scritto» coi caratteri di Hershey (corsivo
    /// o stampatello) un po' storti, letto turno per turno, e l'anteprima in
    /// un PNG (`CAMPIONI_PNG`).
    #[test]
    #[ignore]
    fn addestramento_png() {
        let mut rows_png: Vec<Ink> = Vec::new();
        for style in Style::ALL {
            let font = if style == Style::Corsivo { Font::Corsivo } else { Font::Stampatello };
            let mut seed = 1u64;
            let mut rnd = move || {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                ((seed >> 33) as f32 / (1u64 << 31) as f32) * 2.0 - 1.0
            };
            let mut store = Store::default();
            let mut n = 0;
            while let Some(round) = next_round(&store, 0) {
                let k = 0.03 / 21.0;
                let mut strokes: Ink = Vec::new();
                for (r, row) in round.rows.iter().enumerate() {
                    let y = 0.08 + r as f32 * 0.11;
                    let mut x = 0.04;
                    let mut put = |c: char, x: &mut f32, rnd: &mut dyn FnMut() -> f32| {
                        let c = if c == '−' { '-' } else { c };
                        let Some(g) = char_glyph(font, c).or_else(|| char_glyph(Font::Naturale, c)) else { panic!("{c}") };
                        let sc = 1.0 + 0.06 * rnd();
                        let sl = 0.15 + 0.05 * rnd();
                        for st in &g.strokes {
                            strokes.push(st.iter().map(|p| (*x + (p.0 - sl * p.1) * k * sc, y + p.1 * k * sc)).collect());
                        }
                        *x += g.adv * k * sc;
                    };
                    match row {
                        Row::Signs(cs) => {
                            for &c in cs {
                                put(c, &mut x, &mut rnd);
                                x += 0.025;
                            }
                        }
                        Row::Phrase(p) => {
                            for w in p.split_whitespace() {
                                for c in w.chars() {
                                    put(c, &mut x, &mut rnd);
                                }
                                x += 0.03;
                            }
                        }
                    }
                }
                // la pagina del primo turno, per provare la finestra (`CAMPIONI_PAGE`)
                if let (Ok(f), 1, Style::Corsivo) = (std::env::var("CAMPIONI_PAGE"), round.number, style) {
                    let aspect = 21000.0 / 29700.0;
                    let st: Vec<serde_json::Value> = strokes
                        .iter()
                        .map(|s| serde_json::json!({"c": "#181c30", "w": 2.2, "p": s.iter().map(|p| vec![29700.0 * (1.0 - p.1), 21000.0 * p.0 / aspect, 0.6]).collect::<Vec<_>>()}))
                        .collect();
                    let page = serde_json::json!({"version": 1, "title": "Turno 1", "created": "2026-10-24 10:00:00", "source": "pad", "raw": null, "width": 29700.0, "height": 21000.0, "strokes": st});
                    std::fs::write(f, page.to_string()).unwrap();
                }
                let read = read_round(&round, &strokes, &store);
                if read.error.is_some() {
                    for r in &round.rows {
                        eprintln!("chiesto: {}", r.text());
                    }
                    for r in rows_of(&strokes, usize::MAX) {
                        let (_, _, y0, y1) = bb(r.iter().flat_map(|&i| strokes[i].iter().cloned()));
                        eprintln!("riga {y0:.3}..{y1:.3}: {} tratti", r.len());
                    }
                }
                assert!(read.error.is_none(), "{style:?} turno {}: {:?}", round.number, read.error);
                let rws = rows_of(&strokes, round.rows.len());
                for (ri, r) in read.rows.iter().enumerate() {
                    if let Some(e) = &r.error {
                        eprintln!("{style:?} turno {}: {e}", round.number);
                        for &i in &rws[ri] {
                            let b = bb(strokes[i].iter().cloned());
                            eprintln!("   x {:.3}..{:.3} y {:.3}..{:.3} n{}", b.0, b.1, b.2, b.3, strokes[i].len());
                        }
                    }
                }
                commit(&mut store, &read, &HashSet::new());
                n += 1;
                assert!(n < 15, "troppi turni");
            }
            eprintln!("{style:?}: {n} turni, spazi {:?} {:?}", store.letter_gap, store.word_gap);
            if let Ok(d) = std::env::var("CAMPIONI_OUT") {
                std::fs::write(format!("{d}/{}.json", style.key()), serde_json::to_string(&Store { done: true, ..store.clone() }).unwrap()).unwrap();
            }
            for t in ["Studiare la funzione $f(x) = \\frac{x^2-1}{x}$", "Il dominio è $x \\neq 0$ e la funzione è dispari"] {
                rows_png.push(using_store(Mode::Scrittura(style), Some((style, store.clone())), || crate::mathhand::line_box(Font::Naturale, t)).strokes);
            }
        }
        let s = 2.2;
        let mut pm = tiny_skia::Pixmap::new(1500, 110 * rows_png.len() as u32 + 40).unwrap();
        pm.fill(tiny_skia::Color::WHITE);
        let mut paint = tiny_skia::Paint::default();
        paint.set_color_rgba8(20, 30, 60, 255);
        paint.anti_alias = true;
        let stroke = tiny_skia::Stroke { width: 2.2, line_cap: tiny_skia::LineCap::Round, line_join: tiny_skia::LineJoin::Round, ..Default::default() };
        for (r, strokes) in rows_png.iter().enumerate() {
            let base = 80.0 + r as f32 * 110.0;
            for st in strokes {
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
        pm.save_png(std::env::var("CAMPIONI_PNG").unwrap()).unwrap();
    }
}
