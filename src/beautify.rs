//! La bella scrittura: la tua calligrafia, riordinata mentre scrivi.
//!
//! Non è un altro carattere né un'AI: sono i tuoi tratti, con
//! - i tremolii lisciati (i punti si avvicinano alla media dei vicini, gli
//!   estremi restano dove sono);
//! - le righe dritte: la riga si raddrizza e ogni parola torna sulla riga di
//!   base (gli esponenti e i pedici, più piccoli, seguono la parola accanto
//!   invece di finire sulla riga);
//! - le parole regolari: stessa inclinazione, altezza simile, spazi simili.
//!
//! Le righe e le parole sono quelle che legge `boardread` (i disegni restano
//! fuori). È solo un modo di mostrare: i tratti salvati sono sempre gli
//! originali, l'AI lavora su quelli e i suoi segni (cerchi, sottolineature,
//! correzioni) si spostano insieme alle parole su cui stanno. Spento, torna
//! tutto com'era. Mai sulle pagine dei documenti da firmare.

use crate::scene::{PageGeom, View};
use crate::stroke::{Drawing, Stroke};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

/// 0 = spenta, 1 = leggera, 2 = media, 3 = forte.
static LEVEL: AtomicU8 = AtomicU8::new(0);

pub fn level() -> u8 {
    LEVEL.load(Ordering::Relaxed)
}

pub fn set_level(l: u8) {
    LEVEL.store(l.min(3), Ordering::Relaxed);
}

/// Il nome del livello (per i menu e la barra di stato).
pub fn level_name(l: u8) -> &'static str {
    match l {
        1 => "leggera",
        2 => "media",
        3 => "forte",
        _ => "spenta",
    }
}

struct Params {
    /// Passate di lisciatura.
    smooth: usize,
    /// Quanto si raddrizza la riga (0–1).
    rotate: f32,
    /// Quanto le parole tornano sulla riga di base (0–1).
    shift: f32,
    /// Quanto si uniforma l'altezza delle parole, e di quanto al massimo.
    size: f32,
    size_max: f32,
    /// Quanto si uniforma l'inclinazione.
    slant: f32,
    /// Quanto si uniformano gli spazi fra le parole.
    spacing: f32,
}

fn params(level: u8) -> Option<Params> {
    Some(match level {
        1 => Params { smooth: 2, rotate: 0.6, shift: 0.5, size: 0.0, size_max: 0.0, slant: 0.0, spacing: 0.0 },
        2 => Params { smooth: 3, rotate: 1.0, shift: 0.85, size: 0.5, size_max: 0.12, slant: 0.5, spacing: 0.35 },
        3 => Params { smooth: 5, rotate: 1.0, shift: 1.0, size: 0.85, size_max: 0.2, slant: 0.85, spacing: 0.7 },
        _ => return None,
    })
}

/// Linee, frecce e cerchi fatti a mano diventano perfetti (`perfect_shapes`
/// in settings.conf), anche con la bella scrittura spenta.
static SHAPES: AtomicBool = AtomicBool::new(false);

pub fn shapes() -> bool {
    SHAPES.load(Ordering::Relaxed)
}

pub fn set_shapes(on: bool) {
    SHAPES.store(on, Ordering::Relaxed);
}

/// C'è qualcosa da cambiare in come si vede il foglio?
pub fn active() -> bool {
    mode() > 0
}

/// Livello e forme in un numero (per le chiavi delle pagine già pronte).
pub fn mode() -> u8 {
    level() | if shapes() { 4 } else { 0 }
}

/// Il disegno da mostrare: quello originale se la bella scrittura è spenta.
pub fn nice(d: &Drawing, view: View) -> Drawing {
    let l = level();
    if !active() || d.strokes.is_empty() {
        return d.clone();
    }
    let (pw, ph) = crate::scene::page_dims(d);
    let g = PageGeom::new(view, pw, ph);
    let (strokes, shaped) = if shapes() { perfect(&d.strokes, &g) } else { (d.strokes.clone(), vec![false; d.strokes.len()]) };
    let strokes = if l > 0 { apply_with(&strokes, &g, l, &shaped) } else { strokes };
    Drawing { strokes, width: d.width, height: d.height }
}

/// Solo la lisciatura (per il tratto che si sta scrivendo: il resto si
/// assesta quando si alza la penna).
pub fn smooth_only(s: &Stroke, level: u8) -> Stroke {
    let mut s = s.clone();
    if let Some(p) = params(level) {
        if !s.highlight {
            // senza la pagina: una lettera tipica è alta circa 5 mm (500 unità)
            smooth(&mut s, p.smooth, 25.0);
        }
    }
    s
}

/// I tratti riordinati (stesso numero, stesso ordine: cambiano solo i punti).
pub fn apply(strokes: &[Stroke], g: &PageGeom, level: u8) -> Vec<Stroke> {
    apply_with(strokes, g, level, &vec![false; strokes.len()])
}

/// Come `apply`; `shaped` sono le forme già rese perfette (linee, frecce,
/// cerchi): non sono scrittura, seguono le parole che toccano.
fn apply_with(strokes: &[Stroke], g: &PageGeom, level: u8, shaped: &[bool]) -> Vec<Stroke> {
    let Some(p) = params(level) else { return strokes.to_vec() };
    let mut out: Vec<Stroke> = strokes.to_vec();
    let reading = crate::boardread::read(&out, g, None);
    let hh = reading.char_h;
    // il tremolio, non la forma: al più un ventesimo di lettera
    let cap = 0.05 * hh * g.h;
    for s in out.iter_mut().filter(|s| !s.highlight) {
        smooth(s, p.smooth, cap);
    }
    if reading.lines.is_empty() {
        return out;
    }
    let aspect = g.w / g.h;
    // i punti in coordinate «fisiche» della pagina (x e y nella stessa unità)
    let mut ph: Vec<Vec<(f32, f32)>> = out
        .iter()
        .map(|s| {
            s.points
                .iter()
                .map(|q| {
                    let (u, v) = g.unit(q.x, q.y);
                    (u * aspect, v)
                })
                .collect()
        })
        .collect();
    let orig = ph.clone();
    // i disegni restano dove sono; evidenziatori e segni (cerchi, riquadri,
    // frecce, sottolineature: anche quelli dell'AI) seguono le parole che
    // toccano; si riordina la scrittura, di qualunque colore
    let in_drawing: Vec<usize> = reading.drawing.as_ref().map(|d| d.strokes.clone()).unwrap_or_default();
    let mut moved = vec![false; out.len()];
    let mut text = Vec::new();
    let boxes: Vec<Option<[f32; 4]>> = ph.iter().map(|p| pbox(p)).collect();
    let ink: Vec<bool> = out.iter().map(|s| !s.highlight).collect();
    for i in 0..out.len() {
        if out[i].highlight || ph[i].is_empty() {
            continue;
        }
        if in_drawing.contains(&i) {
            moved[i] = true;
        } else if !shaped.get(i).copied().unwrap_or(false) && !is_mark(i, &boxes, &ink, hh) {
            text.push(i);
        }
    }
    // (i tratti di scrittura che non finiscono in nessuna riga, come un «=»
    // isolato, seguono le parole accanto come i segni)
    let mut line_moves: Vec<LineMove> = Vec::new();
    for (members, words) in group_lines(&text, &ph, hh) {
        // dove stavano le parole, per i segni che le seguono
        let before: Vec<((f32, f32), (f32, f32))> = words.iter().map(|w| (word_x(w, &ph), word_center(w, &ph))).collect();
        // 1. la riga dritta: la pendenza delle righe di base delle parole
        let m0: Vec<Option<Metrics>> = words.iter().map(|w| metrics(w, &ph, hh)).collect();
        let big0 = big_words(&m0);
        // le parole vere (non segni, cifre, parentesi): danno la riga di base
        // più sicura e solo loro cambiano inclinazione, altezza e spazi
        let wide: Vec<bool> = words.iter().map(|w| {
            let (l, r) = word_x(w, &ph);
            r - l >= 1.5 * hh
        }).collect();
        let fit = robust_fit(&m0, &big0, &wide, hh);
        let rot = match &fit {
            // quasi dritta: si lascia com'è (niente rumore sulla scrittura già bella)
            Some((a, b, _)) if b.abs() >= 0.012 && big0.iter().filter(|&&k| wide[k]).count() >= 2 => {
                let (a, b) = (*a, *b);
                let theta = -b.atan() * p.rotate;
                let x0 = members.iter().flat_map(|&i| ph[i].iter().map(|q| q.0)).fold(f32::MAX, f32::min);
                Rot { theta, px: x0, py: a + b * x0 }
            }
            _ => Rot { theta: 0.0, px: 0.0, py: 0.0 },
        };
        // le parole «regolari»: quelle vicine alla riga di base. Una parola
        // fatta solo di lettere con la coda o l'asta («g», «l», «gy») ha la
        // base sbagliata: segue le vicine, come gli esponenti
        // già in ordine (dritta, parole sulla riga): non si tocca, si liscia soltanto
        // con meno di due parole vere (numeri, formule, una parola sola) non
        // c'è una riga da raddrizzare: si liscia soltanto
        let few = big0.iter().filter(|&&k| wide[k]).count() < 2;
        let tidy = few
            || rot.theta == 0.0
            && fit.as_ref().is_some_and(|(a, b, reg)| {
                reg.len() >= 2 && reg.iter().all(|&k| m0[k].as_ref().is_some_and(|w| (a + b * w.cx - w.base).abs() < 0.15 * hh))
            });
        let big: Vec<usize> = fit.map(|(_, _, reg)| reg).unwrap_or_default();
        for &i in &members {
            for q in ph[i].iter_mut() {
                *q = rot.apply(*q);
            }
            moved[i] = true;
        }
        if tidy {
            let lb = members.iter().filter_map(|&i| pbox(&orig[i])).reduce(union).unwrap_or([0.0; 4]);
            line_moves.push(LineMove { bbox: lb, rot, avg_dy: 0.0, words: before.iter().map(|&(xs, _)| (xs, (0.0, 0.0))).collect() });
            continue;
        }
        // 2. ogni parola sulla riga di base (la retta che resta dopo averla raddrizzata)
        let m1: Vec<Option<Metrics>> = words.iter().map(|w| metrics(w, &ph, hh)).collect();
        let fit2 = fit_line(&m1, &big, hh);
        let mut dy = vec![0.0f32; words.len()];
        if let Some((a, b)) = fit2 {
            for &k in &big {
                if let Some(m) = &m1[k] {
                    let d = (a + b * m.cx) - m.base;
                    // un niente resta com'è
                    dy[k] = if d.abs() < 0.1 * hh { 0.0 } else { d.clamp(-0.8 * hh, 0.8 * hh) * p.shift };
                }
            }
        }
        // le parole piccole (esponenti, pedici, punteggiatura) seguono la
        // grande più vicina: restano dove sono rispetto a lei
        for k in 0..words.len() {
            if big.contains(&k) {
                continue;
            }
            let cx = word_cx(&words[k], &ph);
            let near = big.iter().copied().min_by(|&a, &b| {
                let da = m1[a].as_ref().map_or(f32::MAX, |m| (m.cx - cx).abs());
                let db = m1[b].as_ref().map_or(f32::MAX, |m| (m.cx - cx).abs());
                da.total_cmp(&db)
            });
            // solo se le sta attaccata (un esponente, una virgola): un numero o
            // un segno lontano resta dov'è
            let (l, r) = word_x(&words[k], &ph);
            dy[k] = near
                .filter(|&n| {
                    let (nl, nr) = word_x(&words[n], &ph);
                    (nl - r).max(l - nr) < 0.8 * hh
                })
                .map_or(0.0, |n| dy[n]);
        }
        // 3. inclinazione e altezza simili nella riga
        let slants: Vec<f32> = big.iter().filter(|&&k| wide[k]).filter_map(|&k| slant(&words[k], &ph, hh)).collect();
        let target_slant = median(&slants);
        let sizes: Vec<f32> = big.iter().filter(|&&k| wide[k]).filter_map(|&k| m1[k].as_ref().map(|m| m.size)).collect();
        let target_size = median(&sizes);
        let mut scale = vec![1.0f32; words.len()];
        for &k in big.iter().filter(|&&k| wide[k]) {
            let Some(m) = &m1[k] else { continue };
            // l'inclinazione: si raddrizza intorno alla riga di base della parola
            if p.slant > 0.0 {
                if let (Some(t), Some(s)) = (target_slant, slant(&words[k], &ph, hh)) {
                    let d = ((t - s).clamp(-0.2, 0.2) * p.slant).tan();
                    for &i in &words[k] {
                        for q in ph[i].iter_mut() {
                            q.0 += (m.base - q.1) * d;
                        }
                    }
                }
            }
            if p.size > 0.0 {
                if let Some(t) = target_size {
                    if m.size > 0.0 {
                        scale[k] = (t / m.size).powf(p.size).clamp(1.0 - p.size_max, 1.0 + p.size_max);
                    }
                }
            }
        }
        // 4. spazi simili fra le parole (i vicinissimi, come un esponente
        // attaccato alla sua lettera, restano attaccati)
        let boxes: Vec<(f32, f32)> = words.iter().map(|w| word_x(w, &ph)).collect();
        let gaps: Vec<f32> = (1..words.len()).map(|k| boxes[k].0 - boxes[k - 1].1).filter(|&gp| gp > 0.0).collect();
        let gm = median(&gaps).unwrap_or(0.0);
        let mut dx = vec![0.0f32; words.len()];
        let mut prev_right: Option<f32> = None;
        for k in 0..words.len() {
            let (l, r) = boxes[k];
            let w = (r - l) * scale[k];
            let new_l = match prev_right {
                None => l,
                Some(pr) => {
                    let gap = l - boxes[k - 1].1;
                    // solo fra parole vere e spazi normali: un esponente, un
                    // segno, uno spazio voluto (una colonna) restano come sono
                    let words_k = big.contains(&k) && big.contains(&(k - 1)) && wide[k] && wide[k - 1];
                    let g2 = if words_k && gap > 0.8 * gm && gap < 2.5 * gm { gap + (gm - gap) * p.spacing } else { gap };
                    pr + g2
                }
            };
            dx[k] = new_l - l;
            prev_right = Some(new_l + w);
        }
        // tutto insieme: scala intorno a inizio e base della parola, poi gli spostamenti
        for k in 0..words.len() {
            let base = m1[k].as_ref().map(|m| m.base).unwrap_or_else(|| word_bottom(&words[k], &ph));
            let left = boxes[k].0;
            for &i in &words[k] {
                for q in ph[i].iter_mut() {
                    let x = left + (q.0 - left) * scale[k];
                    let y = base + (q.1 - base) * scale[k];
                    *q = (x + dx[k], y + dy[k]);
                }
            }
        }
        let avg_dy = if dy.is_empty() { 0.0 } else { dy.iter().sum::<f32>() / dy.len() as f32 };
        // quanto si è mossa ogni parola oltre la rotazione della riga
        let words_moved = words
            .iter()
            .zip(&before)
            .map(|(w, &(xs, c0))| {
                let c1 = word_center(w, &ph);
                let r = rot.apply(c0);
                (xs, (c1.0 - r.0, c1.1 - r.1))
            })
            .collect();
        let lb = members.iter().filter_map(|&i| pbox(&orig[i])).reduce(union).unwrap_or([0.0; 4]);
        line_moves.push(LineMove { bbox: lb, rot, avg_dy, words: words_moved });
    }
    // evidenziatori e segni sopra una riga la seguono: raddrizzati come lei
    // e spostati come le parole che toccano
    for i in 0..out.len() {
        if moved[i] || orig[i].is_empty() {
            continue;
        }
        let pts = &orig[i];
        let cy = pts.iter().map(|q| q.1).sum::<f32>() / pts.len() as f32;
        let (x0, x1) = pts.iter().fold((f32::MAX, f32::MIN), |(l, r), q| (l.min(q.0), r.max(q.0)));
        let line = line_moves
            .iter()
            .filter(|m| cy >= m.bbox[1] - 0.6 * hh && cy <= m.bbox[3] + 0.6 * hh)
            .min_by(|a, b| ((a.bbox[1] + a.bbox[3]) / 2.0 - cy).abs().total_cmp(&((b.bbox[1] + b.bbox[3]) / 2.0 - cy).abs()));
        let Some(m) = line else { continue };
        let touched: Vec<(f32, f32)> = m.words.iter().filter(|((l, r), _)| *r >= x0 && *l <= x1).map(|(_, d)| *d).collect();
        let (tx, ty) = if touched.is_empty() {
            (0.0, m.avg_dy)
        } else {
            let n = touched.len() as f32;
            (touched.iter().map(|d| d.0).sum::<f32>() / n, touched.iter().map(|d| d.1).sum::<f32>() / n)
        };
        for (q, o) in ph[i].iter_mut().zip(pts) {
            let r = m.rot.apply(*o);
            *q = (r.0 + tx, r.1 + ty);
        }
    }
    // di nuovo nelle coordinate del pad
    for (s, pts) in out.iter_mut().zip(ph) {
        for (q, (x, y)) in s.points.iter_mut().zip(pts) {
            if x.is_finite() && y.is_finite() {
                let (px, py) = g.pad(x / aspect, y);
                q.x = px;
                q.y = py;
            }
        }
    }
    out
}

fn pbox(pts: &[(f32, f32)]) -> Option<[f32; 4]> {
    let mut it = pts.iter();
    let f = it.next()?;
    Some(it.fold([f.0, f.1, f.0, f.1], |b, q| [b[0].min(q.0), b[1].min(q.1), b[2].max(q.0), b[3].max(q.1)]))
}

fn union(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])]
}

/// Le righe e le loro parole. Diverso da `boardread` (che serve all'AI e
/// cerca righe orizzontali): qui una riga segue la scrittura da sinistra a
/// destra anche quando sale o scende, perché è proprio quella da
/// raddrizzare. Un tratto si attacca alla riga il cui ultimo pezzo gli sta
/// accanto e alla stessa altezza.
fn group_lines(text: &[usize], ph: &[Vec<(f32, f32)>], hh: f32) -> Vec<(Vec<usize>, Vec<Vec<usize>>)> {
    let boxes: Vec<(usize, [f32; 4])> = text.iter().filter_map(|&i| pbox(&ph[i]).map(|b| (i, b))).collect();
    let small = |b: &[f32; 4]| b[3] - b[1] < 0.4 * hh;
    let mut main: Vec<(usize, [f32; 4])> = boxes.iter().copied().filter(|(_, b)| !small(b)).collect();
    main.sort_by(|a, b| a.1[0].total_cmp(&b.1[0]));
    // ogni riga: i suoi tratti e la «coda» (gli ultimi tre, per seguire la pendenza)
    let mut lines: Vec<(Vec<usize>, Vec<[f32; 4]>)> = Vec::new();
    for &(i, b) in &main {
        let mut best: Option<(usize, f32)> = None;
        for (k, (_, tail)) in lines.iter().enumerate() {
            let t = tail.iter().copied().reduce(union).unwrap();
            let gap = b[0] - t[2];
            if gap > 3.0 * hh || b[2] < t[0] {
                continue;
            }
            let ov = t[3].min(b[3]) - t[1].max(b[1]);
            // metà del più basso, o almeno quasi mezza lettera (una «p» o una
            // «l» toccano la riga solo con la loro parte di mezzo)
            let need = (0.5 * (t[3] - t[1]).min(b[3] - b[1])).min(0.4 * hh);
            // o subito accanto, poco più su o più giù (la riga che sale o scende)
            let close = gap < 1.5 * hh && ((t[1] + t[3]) - (b[1] + b[3])).abs() / 2.0 < 0.7 * hh;
            if ov >= need || close {
                // la più centrata
                let d = ((t[1] + t[3]) - (b[1] + b[3])).abs();
                if best.is_none_or(|(_, bd)| d < bd) {
                    best = Some((k, d));
                }
            }
        }
        match best {
            Some((k, _)) => {
                lines[k].0.push(i);
                lines[k].1.push(b);
                if lines[k].1.len() > 3 {
                    lines[k].1.remove(0);
                }
            }
            None => lines.push((vec![i], vec![b])),
        }
    }
    // le parole: da sinistra, uno spazio più largo di mezza lettera le separa
    let by_idx: std::collections::HashMap<usize, [f32; 4]> = boxes.iter().copied().collect();
    let bx = |i: usize| by_idx[&i];
    let mut out: Vec<(Vec<usize>, Vec<Vec<usize>>)> = lines
        .into_iter()
        .map(|(m, _)| {
            let mut words: Vec<(Vec<usize>, [f32; 4])> = Vec::new();
            for &i in &m {
                let b = bx(i);
                match words.last_mut() {
                    Some((w, wb)) if b[0] - wb[2] < 0.7 * hh => {
                        w.push(i);
                        *wb = union(*wb, b);
                    }
                    _ => words.push((vec![i], b)),
                }
            }
            (m, words.into_iter().map(|(w, _)| w).collect())
        })
        .collect();
    // puntini, accenti, virgole: con la parola più vicina
    let wboxes: Vec<Vec<[f32; 4]>> = out.iter().map(|(_, ws)| ws.iter().map(|w| w.iter().map(|&j| bx(j)).reduce(union).unwrap()).collect()).collect();
    // i pezzi piccoli vicini fanno un segno solo (le due barre di «=», i
    // due punti di «:», «≈»)
    let smalls: Vec<(usize, [f32; 4])> = boxes.iter().copied().filter(|(_, b)| small(b)).collect();
    let mut sp: Vec<usize> = (0..smalls.len()).collect();
    fn rt(p: &mut [usize], mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    for x in 0..smalls.len() {
        for y in x + 1..smalls.len() {
            let (a, b) = (smalls[x].1, smalls[y].1);
            let gx = (b[0] - a[2]).max(a[0] - b[2]);
            let gy = (b[1] - a[3]).max(a[1] - b[3]);
            if gx < 0.15 * hh && gy < 0.35 * hh {
                let (rx, ry) = (rt(&mut sp, x), rt(&mut sp, y));
                sp[rx] = ry;
            }
        }
    }
    let mut groups: std::collections::BTreeMap<usize, (Vec<usize>, [f32; 4])> = std::collections::BTreeMap::new();
    for x in 0..smalls.len() {
        let r = rt(&mut sp, x);
        let e = groups.entry(r).or_insert((Vec::new(), smalls[x].1));
        e.0.push(smalls[x].0);
        e.1 = union(e.1, smalls[x].1);
    }
    let (mut attach, mut alone) = (Vec::new(), Vec::new());
    for (_, (ids, b)) in groups {
        let c = ((b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0);
        // la parola più vicina, in qualunque riga
        let mut best: Option<(usize, usize, f32, f32)> = None;
        for (li, wbs) in wboxes.iter().enumerate() {
            for (wi, &wb) in wbs.iter().enumerate() {
                let dx = (wb[0] - c.0).max(c.0 - wb[2]).max(0.0);
                let dy = (wb[1] - c.1).max(c.1 - wb[3]).max(0.0);
                let d = dx + 2.0 * dy;
                if dy < 0.9 * hh && dx < 2.0 * hh && best.is_none_or(|(_, _, bd, _)| d < bd) {
                    best = Some((li, wi, d, dx));
                }
            }
        }
        match best {
            // sopra o sotto la parola (puntini, accenti) o attaccato a lei: con lei
            Some((li, wi, _, dx)) if dx < 0.3 * hh => attach.extend(ids.iter().map(|&i| (li, wi, i))),
            // un segno da solo («=», «-», «:»): una parola a sé nella stessa riga
            Some((li, _, _, _)) => alone.push((li, ids)),
            // lontano da tutto: segue le parole che tocca, come i segni
            None => {}
        }
    }
    for (li, wi, i) in attach {
        out[li].0.push(i);
        out[li].1[wi].push(i);
    }
    for (li, ids) in alone {
        let x = ids.iter().map(|&i| bx(i)[0]).fold(f32::MAX, f32::min);
        out[li].0.extend(&ids);
        let pos = out[li].1.iter().position(|w| w.iter().map(|&j| bx(j)[0]).fold(f32::MAX, f32::min) > x).unwrap_or(out[li].1.len());
        out[li].1.insert(pos, ids);
    }
    out
}

/// Come si è mossa una riga: il suo riquadro di prima (coordinate fisiche),
/// la rotazione, lo spostamento medio e, per ogni parola, dove stava in
/// orizzontale e di quanto si è spostata oltre la rotazione.
struct LineMove {
    bbox: [f32; 4],
    rot: Rot,
    avg_dy: f32,
    words: Vec<((f32, f32), (f32, f32))>,
}

/// Un segno, non una lettera: molto più alto della scrittura (riquadro,
/// freccia, parentesi graffa), una linea lunga e piatta (sottolineatura,
/// barra) o un giro attorno ad altri tratti (il cerchio attorno a una parola).
fn is_mark(i: usize, boxes: &[Option<[f32; 4]>], ink: &[bool], hh: f32) -> bool {
    let Some(b) = boxes[i] else { return false };
    let (w, h) = (b[2] - b[0], b[3] - b[1]);
    if h > 2.4 * hh || (w > 3.0 * hh && h < 0.35 * hh) {
        return true;
    }
    let m = 0.1 * hh;
    boxes.iter().enumerate().any(|(j, o)| {
        j != i && ink[j] && o.is_some_and(|o| o[3] - o[1] >= 0.4 * hh && o[0] > b[0] + m && o[2] < b[2] - m && o[1] > b[1] + m && o[3] < b[3] - m)
    })
}

// ---------------------------------------------------------------------------
// Forme perfette: linee, frecce, cerchi ed ellissi

/// Riconosce le linee, le frecce e i cerchi fatti a mano e li rifà perfetti
/// (stessi tratti, stesso numero di punti). Solo i segni e i disegni, mai la
/// scrittura: una «o» resta una «o». Il secondo vettore dice quali tratti
/// sono diventati forme.
pub fn perfect(strokes: &[Stroke], g: &PageGeom) -> (Vec<Stroke>, Vec<bool>) {
    let mut out = strokes.to_vec();
    let mut shaped = vec![false; strokes.len()];
    if strokes.is_empty() {
        return (out, shaped);
    }
    let reading = crate::boardread::read(strokes, g, None);
    let hh = reading.char_h;
    let aspect = g.w / g.h;
    let ph: Vec<Vec<(f32, f32)>> = strokes
        .iter()
        .map(|s| {
            s.points
                .iter()
                .map(|q| {
                    let (u, v) = g.unit(q.x, q.y);
                    (u * aspect, v)
                })
                .collect()
        })
        .collect();
    let boxes: Vec<Option<[f32; 4]>> = ph.iter().map(|p| pbox(p)).collect();
    let ink: Vec<bool> = strokes.iter().map(|s| !s.highlight).collect();
    let in_drawing: Vec<usize> = reading.drawing.as_ref().map(|d| d.strokes.clone()).unwrap_or_default();
    let mut new_pts: Vec<Option<Vec<(f32, f32)>>> = vec![None; strokes.len()];
    // le linee perfette (per attaccarci le punte delle frecce): (inizio, fine)
    let mut lines: Vec<((f32, f32), (f32, f32))> = Vec::new();
    for i in 0..strokes.len() {
        if !ink[i] || ph[i].len() < 3 {
            continue;
        }
        let candidate = is_mark(i, &boxes, &ink, hh) || in_drawing.contains(&i) || straight(&ph[i]).is_some_and(|(a, b)| dist(a, b) >= 2.2 * hh)
            || arrow(&ph[i], hh).is_some_and(|a| dist(a[0], a[1]) >= 2.2 * hh);
        if !candidate {
            continue;
        }
        let n = ph[i].len();
        if let Some((a, b)) = straight(&ph[i]) {
            if dist(a, b) >= 1.2 * hh {
                let (a, b) = snap_line(a, b);
                new_pts[i] = Some(resample(&[a, b], n));
                lines.push((a, b));
                continue;
            }
        }
        if let Some(pts) = arrow(&ph[i], hh) {
            new_pts[i] = Some(resample(&pts, n));
            continue;
        }
        if let Some(pts) = ellipse(&ph[i], n) {
            new_pts[i] = Some(pts);
        }
    }
    // le punte disegnate a parte: una «V» piccola vicino alla fine di una linea
    for i in 0..strokes.len() {
        if !ink[i] || new_pts[i].is_some() || ph[i].len() < 3 {
            continue;
        }
        let Some(b) = boxes[i] else { continue };
        if (b[2] - b[0]).max(b[3] - b[1]) > 1.8 * hh {
            continue;
        }
        if let Some(pts) = arrow_head(&ph[i], &lines, hh) {
            new_pts[i] = Some(resample(&pts, ph[i].len()));
        }
    }
    for (i, np) in new_pts.into_iter().enumerate() {
        let Some(np) = np else { continue };
        shaped[i] = true;
        for (q, (x, y)) in out[i].points.iter_mut().zip(np) {
            let (px, py) = g.pad(x / aspect, y);
            q.x = px;
            q.y = py;
        }
    }
    (out, shaped)
}

/// Il tratto è una linea (quasi) dritta? Allora i suoi estremi.
fn straight(p: &[(f32, f32)]) -> Option<((f32, f32), (f32, f32))> {
    let (a, b) = (p[0], *p.last()?);
    let len = dist(a, b);
    if len <= 0.0 {
        return None;
    }
    let path: f32 = p.windows(2).map(|w| dist(w[0], w[1])).sum();
    let dev = p.iter().map(|&q| seg_dist(q, a, b)).fold(0.0f32, f32::max);
    (dev < 0.06 * len && path < 1.15 * len).then_some((a, b))
}

/// Quasi orizzontale o verticale (entro 5°): proprio orizzontale o verticale,
/// attorno al punto di mezzo.
fn snap_line(a: (f32, f32), b: (f32, f32)) -> ((f32, f32), (f32, f32)) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let ang = dy.atan2(dx);
    let len = dist(a, b);
    let m = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
    let step = std::f32::consts::FRAC_PI_2;
    let near = (ang / step).round() * step;
    let ang = if (ang - near).abs() < 5f32.to_radians() { near } else { ang };
    let (s, c) = ang.sin_cos();
    ((m.0 - c * len / 2.0, m.1 - s * len / 2.0), (m.0 + c * len / 2.0, m.1 + s * len / 2.0))
}

/// Una freccia in un tratto solo: un'asta lunga e la punta (uno o due
/// tagli corti) a un capo. La rifà: asta dritta e punta simmetrica.
fn arrow(p: &[(f32, f32)], hh: f32) -> Option<Vec<(f32, f32)>> {
    let total: f32 = p.windows(2).map(|w| dist(w[0], w[1])).sum();
    let simp = rdp(p, (0.04 * total).max(0.12 * hh));
    if simp.len() < 3 || simp.len() > 6 {
        return None;
    }
    let segs: Vec<f32> = simp.windows(2).map(|w| dist(w[0], w[1])).collect();
    let (k, &long) = segs.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1))?;
    let rest: f32 = segs.iter().sum::<f32>() - long;
    if long < 0.6 * (long + rest) || long < 1.5 * hh {
        return None;
    }
    // l'asta va dal capo opposto alla punta
    let (tail, tip, barbs): ((f32, f32), (f32, f32), Vec<(f32, f32)>) = if k == 0 {
        (simp[0], simp[1], simp[2..].to_vec())
    } else if k == segs.len() - 1 {
        (simp[simp.len() - 1], simp[simp.len() - 2], simp[..simp.len() - 2].to_vec())
    } else {
        return None;
    };
    // la punta sta vicino alla fine dell'asta e torna indietro (i punti che
    // ripassano sulla punta non contano)
    let barbs: Vec<(f32, f32)> = barbs.into_iter().filter(|&q| dist(q, tip) > 0.03 * long).collect();
    if barbs.is_empty() {
        return None;
    }
    let dir = ((tip.0 - tail.0) / long, (tip.1 - tail.1) / long);
    let near = barbs.iter().all(|&q| dist(q, tip) < 0.45 * long && ((q.0 - tip.0) * dir.0 + (q.1 - tip.1) * dir.1) < 0.0);
    if !near {
        return None;
    }
    let barb = barbs.iter().map(|&q| dist(q, tip)).fold(0.0f32, f32::max).clamp(0.08 * long, 0.3 * long);
    let (tail, tip) = snap_line(tail, tip);
    let (b1, b2) = head(tail, tip, barb);
    Some(vec![tail, tip, b1, tip, b2])
}

/// Le due alette di una punta in `tip`, per un'asta che viene da `tail`.
fn head(tail: (f32, f32), tip: (f32, f32), barb: f32) -> ((f32, f32), (f32, f32)) {
    let ang = (tail.1 - tip.1).atan2(tail.0 - tip.0);
    let a = 28f32.to_radians();
    let p = |t: f32| (tip.0 + barb * t.cos(), tip.1 + barb * t.sin());
    (p(ang + a), p(ang - a))
}

/// Una punta disegnata a parte: una «V» col vertice vicino al capo di una linea.
fn arrow_head(p: &[(f32, f32)], lines: &[((f32, f32), (f32, f32))], hh: f32) -> Option<Vec<(f32, f32)>> {
    let total: f32 = p.windows(2).map(|w| dist(w[0], w[1])).sum();
    let simp = rdp(p, (0.12 * total).max(0.05 * hh));
    if simp.len() != 3 {
        return None;
    }
    let (b1, apex, b2) = (simp[0], simp[1], simp[2]);
    let (l1, l2) = (dist(b1, apex), dist(b2, apex));
    if l1.min(l2) < 0.4 * l1.max(l2) {
        return None;
    }
    for &(a, b) in lines {
        for (tail, tip) in [(a, b), (b, a)] {
            let len = dist(tail, tip);
            if dist(apex, tip) > 0.6 * hh.max(0.15 * len) {
                continue;
            }
            // le alette tornano verso l'asta
            let dir = ((tip.0 - tail.0) / len, (tip.1 - tail.1) / len);
            let back = |q: (f32, f32)| (q.0 - apex.0) * dir.0 + (q.1 - apex.1) * dir.1 < 0.0;
            if back(b1) && back(b2) {
                let (h1, h2) = head(tail, tip, (l1 + l2) / 2.0);
                return Some(vec![h1, tip, h2]);
            }
        }
    }
    None
}

/// Un cerchio o un'ellisse chiusa, fatta a mano: la stessa figura perfetta
/// (un cerchio se è quasi tondo, dritta se è quasi dritta), con `n` punti
/// dallo stesso punto di partenza e nello stesso verso.
fn ellipse(p: &[(f32, f32)], n: usize) -> Option<Vec<(f32, f32)>> {
    // punti a passo costante (la velocità della mano non deve pesare)
    let q = resample(p, 64);
    let (cx, cy) = q.iter().fold((0.0, 0.0), |a, b| (a.0 + b.0 / 64.0, a.1 + b.1 / 64.0));
    let (mut sxx, mut syy, mut sxy) = (0.0f32, 0.0f32, 0.0f32);
    for &(x, y) in &q {
        sxx += (x - cx) * (x - cx) / 64.0;
        syy += (y - cy) * (y - cy) / 64.0;
        sxy += (x - cx) * (y - cy) / 64.0;
    }
    // assi principali
    let tr = sxx + syy;
    let det = sxx * syy - sxy * sxy;
    let disc = ((tr * tr / 4.0) - det).max(0.0).sqrt();
    let (l1, l2) = (tr / 2.0 + disc, (tr / 2.0 - disc).max(0.0));
    let theta = 0.5 * (2.0 * sxy).atan2(sxx - syy);
    let (a, b) = ((2.0 * l1).sqrt(), (2.0 * l2).sqrt());
    if a <= 0.0 || b < 0.15 * a {
        return None;
    }
    // chiusa: la fine torna vicino all'inizio, e il giro è quasi completo
    if dist(p[0], *p.last()?) > 0.5 * b.max(0.35 * a) {
        return None;
    }
    let (st, ct) = theta.sin_cos();
    let local = |&(x, y): &(f32, f32)| {
        let (dx, dy) = (x - cx, y - cy);
        (dx * ct + dy * st, -dx * st + dy * ct)
    };
    let mut err = 0.0f32;
    let mut worst = 0.0f32;
    let mut sweep = 0.0f32;
    let mut prev: Option<f32> = None;
    for pt in &q {
        let (u, v) = local(pt);
        let r = ((u / a).powi(2) + (v / b).powi(2)).sqrt();
        err += (r - 1.0).abs() / 64.0;
        worst = worst.max((r - 1.0).abs());
        let ang = v.atan2(u);
        if let Some(pa) = prev {
            let mut d = ang - pa;
            while d > std::f32::consts::PI {
                d -= std::f32::consts::TAU;
            }
            while d < -std::f32::consts::PI {
                d += std::f32::consts::TAU;
            }
            sweep += d;
        }
        prev = Some(ang);
    }
    if err > 0.12 || worst > 0.35 || sweep.abs() < 300f32.to_radians() {
        return None;
    }
    // quasi tondo: cerchio; quasi dritta: senza rotazione
    let (a, b, theta) = if b / a > 0.85 {
        let r = (a + b) / 2.0;
        (r, r, 0.0)
    } else {
        let step = std::f32::consts::FRAC_PI_2;
        let near = (theta / step).round() * step;
        (a, b, if (theta - near).abs() < 6f32.to_radians() { near } else { theta })
    };
    let (st, ct) = theta.sin_cos();
    let (u0, v0) = {
        let (dx, dy) = (p[0].0 - cx, p[0].1 - cy);
        (dx * ct + dy * st, -dx * st + dy * ct)
    };
    let start = (v0 / b).atan2(u0 / a);
    let dir = sweep.signum();
    Some(
        (0..n)
            .map(|k| {
                let t = start + dir * std::f32::consts::TAU * k as f32 / (n - 1).max(1) as f32;
                let (u, v) = (a * t.cos(), b * t.sin());
                (cx + u * ct - v * st, cy + u * st + v * ct)
            })
            .collect(),
    )
}

/// `n` punti a distanza costante lungo la spezzata.
fn resample(p: &[(f32, f32)], n: usize) -> Vec<(f32, f32)> {
    if p.len() < 2 || n < 2 {
        return vec![p.first().copied().unwrap_or((0.0, 0.0)); n];
    }
    let cum: Vec<f32> = std::iter::once(0.0)
        .chain(p.windows(2).scan(0.0, |acc, w| {
            *acc += dist(w[0], w[1]);
            Some(*acc)
        }))
        .collect();
    let total = *cum.last().unwrap();
    let mut out = Vec::with_capacity(n);
    let mut j = 0;
    for k in 0..n {
        let t = total * k as f32 / (n - 1) as f32;
        while j + 1 < cum.len() - 1 && cum[j + 1] < t {
            j += 1;
        }
        let seg = (cum[j + 1] - cum[j]).max(1e-9);
        let f = ((t - cum[j]) / seg).clamp(0.0, 1.0);
        out.push((p[j].0 + (p[j + 1].0 - p[j].0) * f, p[j].1 + (p[j + 1].1 - p[j].1) * f));
    }
    out
}

/// Ramer–Douglas–Peucker: gli spigoli della spezzata.
fn rdp(p: &[(f32, f32)], eps: f32) -> Vec<(f32, f32)> {
    if p.len() < 3 {
        return p.to_vec();
    }
    let (a, b) = (p[0], p[p.len() - 1]);
    let (k, d) = p.iter().enumerate().skip(1).take(p.len() - 2).map(|(i, &q)| (i, seg_dist(q, a, b))).fold((0, 0.0f32), |m, x| if x.1 > m.1 { x } else { m });
    if d <= eps {
        return vec![a, b];
    }
    let mut left = rdp(&p[..=k], eps);
    let right = rdp(&p[k..], eps);
    left.pop();
    left.extend(right);
    left
}

/// Distanza di `q` dal segmento a–b.
fn seg_dist(q: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l2 = dx * dx + dy * dy;
    if l2 <= 0.0 {
        return dist(q, a);
    }
    let t = (((q.0 - a.0) * dx + (q.1 - a.1) * dy) / l2).clamp(0.0, 1.0);
    dist(q, (a.0 + t * dx, a.1 + t * dy))
}

#[derive(Clone, Copy)]
struct Rot {
    theta: f32,
    px: f32,
    py: f32,
}

impl Rot {
    fn apply(&self, (x, y): (f32, f32)) -> (f32, f32) {
        if self.theta == 0.0 {
            return (x, y);
        }
        let (s, c) = self.theta.sin_cos();
        let (dx, dy) = (x - self.px, y - self.py);
        (self.px + dx * c - dy * s, self.py + dx * s + dy * c)
    }
}

/// I punti di un tratto si avvicinano alla media dei vicini, di poco
/// (`cap`, unità del pad): si toglie il tremolio della mano, non si
/// arrotondano gli angoli (un «2», un «4», una radice). Gli estremi e gli
/// spigoli restano dove sono.
fn smooth(s: &mut Stroke, passes: usize, cap: f32) {
    let n = s.points.len();
    if n < 3 {
        return;
    }
    let orig: Vec<(f32, f32)> = s.points.iter().map(|p| (p.x, p.y)).collect();
    // gli spigoli: dove il tratto gira di più di 60° (sui vicini a un passo
    // di almeno un ventesimo di lettera, per non scambiare il tremolio per uno spigolo)
    let corner: Vec<bool> = (0..n)
        .map(|i| {
            if i == 0 || i == n - 1 {
                return true;
            }
            let reach = 2.0 * cap;
            let back = (0..i).rev().find(|&j| dist(orig[j], orig[i]) >= reach);
            let fwd = (i + 1..n).find(|&j| dist(orig[j], orig[i]) >= reach);
            match (back, fwd) {
                (Some(a), Some(b)) => {
                    let (ax, ay) = (orig[i].0 - orig[a].0, orig[i].1 - orig[a].1);
                    let (bx, by) = (orig[b].0 - orig[i].0, orig[b].1 - orig[i].1);
                    let c = (ax * bx + ay * by) / ((ax * ax + ay * ay).sqrt() * (bx * bx + by * by).sqrt()).max(1e-6);
                    c < 0.5
                }
                _ => false,
            }
        })
        .collect();
    for _ in 0..passes {
        let prev: Vec<(f32, f32)> = s.points.iter().map(|p| (p.x, p.y)).collect();
        for i in 1..n - 1 {
            if corner[i] {
                continue;
            }
            let (ax, ay) = prev[i - 1];
            let (bx, by) = prev[i + 1];
            let (x, y) = prev[i];
            let nx = x + 0.5 * ((ax + bx) / 2.0 - x);
            let ny = y + 0.5 * ((ay + by) / 2.0 - y);
            // mai più lontano di `cap` dal punto scritto
            let (dx, dy) = (nx - orig[i].0, ny - orig[i].1);
            let d = (dx * dx + dy * dy).sqrt();
            let k = if d > cap { cap / d } else { 1.0 };
            s.points[i].x = orig[i].0 + dx * k;
            s.points[i].y = orig[i].1 + dy * k;
        }
    }
}

fn dist(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// Riga di base, altezza e centro di una parola (dai tratti «di corpo»,
/// senza puntini e accenti).
struct Metrics {
    base: f32,
    size: f32,
    cx: f32,
}

fn quantile(v: &mut [f32], q: f32) -> f32 {
    v.sort_by(f32::total_cmp);
    v[((v.len() - 1) as f32 * q).round() as usize]
}

fn metrics(word: &[usize], ph: &[Vec<(f32, f32)>], hh: f32) -> Option<Metrics> {
    // la base è dove la penna tocca il fondo più spesso (i fondi delle
    // lettere), l'altezza dove tocca la cima: così una «g», una «/», un
    // esponente in mezzo alla parola non spostano la riga
    let mut bottoms = Vec::new();
    let mut tops = Vec::new();
    let mut lows = Vec::new();
    let mut highs = Vec::new();
    let (mut x0, mut x1) = (f32::MAX, f32::MIN);
    for &i in word {
        let pts = &ph[i];
        if pts.is_empty() {
            continue;
        }
        let mut ys: Vec<f32> = pts.iter().map(|q| q.1).collect();
        let (lo, hi) = (quantile(&mut ys, 0.0), quantile(&mut ys, 1.0));
        if hi - lo < 0.4 * hh {
            continue;
        }
        lows.push(quantile(&mut ys, 0.9));
        highs.push(quantile(&mut ys, 0.1));
        let (b, t) = turns(pts, 0.12 * hh);
        bottoms.extend(b);
        tops.extend(t);
        for q in pts {
            x0 = x0.min(q.0);
            x1 = x1.max(q.0);
        }
    }
    if lows.is_empty() {
        return None;
    }
    // un po' sotto la metà: le code (g, p, parentesi) contano meno dei
    // fondi sulla riga, le aste (l, h, d) meno delle cime delle minuscole
    let q = |v: &[f32], f: f32| {
        let mut v = v.to_vec();
        (!v.is_empty()).then(|| quantile(&mut v, f))
    };
    let base = q(&bottoms, 0.35).or_else(|| median(&lows))?;
    let top = q(&tops, 0.65).or_else(|| median(&highs))?;
    Some(Metrics { base, size: (base - top).max(0.0), cx: (x0 + x1) / 2.0 })
}

/// I fondi e le cime di un tratto: dove la penna, scendendo, torna a salire
/// (e viceversa) di almeno `min` (y verso il basso).
fn turns(pts: &[(f32, f32)], min: f32) -> (Vec<f32>, Vec<f32>) {
    let (mut bottoms, mut tops) = (Vec::new(), Vec::new());
    if pts.is_empty() {
        return (bottoms, tops);
    }
    // si segue la y: la direzione cambia solo dopo un ritorno di almeno `min`
    let mut ext = pts[0].1;
    let mut going_down: Option<bool> = None;
    for q in &pts[1..] {
        let y = q.1;
        match going_down {
            None => {
                if y - ext >= min {
                    tops.push(ext);
                    going_down = Some(true);
                    ext = y;
                } else if ext - y >= min {
                    bottoms.push(ext);
                    going_down = Some(false);
                    ext = y;
                }
            }
            Some(true) => {
                if y > ext {
                    ext = y;
                } else if ext - y >= min {
                    bottoms.push(ext);
                    going_down = Some(false);
                    ext = y;
                }
            }
            Some(false) => {
                if y < ext {
                    ext = y;
                } else if y - ext >= min {
                    tops.push(ext);
                    going_down = Some(true);
                    ext = y;
                }
            }
        }
    }
    match going_down {
        Some(true) => bottoms.push(ext),
        Some(false) => tops.push(ext),
        None => {}
    }
    (bottoms, tops)
}

/// Le parole «vere» della riga: non molto più piccole delle altre.
fn big_words(m: &[Option<Metrics>]) -> Vec<usize> {
    let sizes: Vec<f32> = m.iter().flatten().map(|m| m.size).collect();
    let Some(ms) = median(&sizes) else { return Vec::new() };
    (0..m.len()).filter(|&k| m[k].as_ref().is_some_and(|w| w.size >= 0.6 * ms)).collect()
}

/// La retta delle righe di base e le parole che ci stanno sopra (scarto
/// entro un terzo di lettera): le altre non spostano la retta.
fn robust_fit(m: &[Option<Metrics>], big: &[usize], wide: &[bool], hh: f32) -> Option<(f32, f32, Vec<usize>)> {
    // la retta la danno le parole lunghe, se ce ne sono almeno due: una «g»,
    // una «/», una «J» da sole hanno la base nella coda
    let long: Vec<usize> = big.iter().copied().filter(|&k| wide[k]).collect();
    let (a, b) = fit_line(m, if long.len() >= 2 { &long } else { big }, hh)?;
    let res = |k: usize, a: f32, b: f32| m[k].as_ref().map_or(f32::MAX, |w| (a + b * w.cx - w.base).abs());
    let reg: Vec<usize> = big.iter().copied().filter(|&k| res(k, a, b) <= 0.35 * hh).collect();
    if reg.len() >= 2 && reg.len() < big.len() {
        let (a2, b2) = fit_line(m, &reg, hh)?;
        let reg2: Vec<usize> = big.iter().copied().filter(|&k| res(k, a2, b2) <= 0.35 * hh).collect();
        return Some((a2, b2, if reg2.is_empty() { reg } else { reg2 }));
    }
    // nessuna (o una sola) parola sulla retta: si tiene la mediana
    if reg.is_empty() {
        let mut v: Vec<(usize, f32)> = big.iter().filter_map(|&k| m[k].as_ref().map(|w| (k, w.base))).collect();
        v.sort_by(|x, y| x.1.total_cmp(&y.1));
        return v.get(v.len() / 2).map(|&(k, base)| (base, 0.0, vec![k]));
    }
    Some((a, b, reg))
}

/// La retta delle righe di base (y = a + b·x), se le parole bastano.
fn fit_line(m: &[Option<Metrics>], big: &[usize], hh: f32) -> Option<(f32, f32)> {
    let pts: Vec<(f32, f32)> = big.iter().filter_map(|&k| m[k].as_ref().map(|w| (w.cx, w.base))).collect();
    if pts.is_empty() {
        return None;
    }
    let n = pts.len() as f32;
    let mx = pts.iter().map(|p| p.0).sum::<f32>() / n;
    let my = pts.iter().map(|p| p.1).sum::<f32>() / n;
    let span = pts.iter().map(|p| p.0).fold(f32::MIN, f32::max) - pts.iter().map(|p| p.0).fold(f32::MAX, f32::min);
    let b = if pts.len() >= 2 && span > 2.0 * hh {
        let sxx: f32 = pts.iter().map(|p| (p.0 - mx).powi(2)).sum();
        let sxy: f32 = pts.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
        if sxx > 0.0 {
            (sxy / sxx).clamp(-0.35, 0.35)
        } else {
            0.0
        }
    } else {
        0.0
    };
    Some((my - b * mx, b))
}

/// L'inclinazione media dei tratti quasi verticali della parola (radianti;
/// positiva = pende a destra).
fn slant(word: &[usize], ph: &[Vec<(f32, f32)>], hh: f32) -> Option<f32> {
    let (mut sum, mut wsum) = (0.0f32, 0.0f32);
    for &i in word {
        for w in ph[i].windows(2) {
            let (dx, dy) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
            let len = (dx * dx + dy * dy).sqrt();
            if dy.abs() > 1.5 * dx.abs() && len > 0.02 * hh {
                // verso l'alto: dy < 0
                let (dx, dy) = if dy > 0.0 { (-dx, -dy) } else { (dx, dy) };
                sum += (dx / -dy).atan() * len;
                wsum += len;
            }
        }
    }
    (wsum > 0.5 * hh).then(|| sum / wsum)
}

fn word_x(word: &[usize], ph: &[Vec<(f32, f32)>]) -> (f32, f32) {
    let xs = word.iter().flat_map(|&i| ph[i].iter().map(|q| q.0));
    let (mut l, mut r) = (f32::MAX, f32::MIN);
    for x in xs {
        l = l.min(x);
        r = r.max(x);
    }
    if l > r {
        (0.0, 0.0)
    } else {
        (l, r)
    }
}

fn word_center(word: &[usize], ph: &[Vec<(f32, f32)>]) -> (f32, f32) {
    let (mut sx, mut sy, mut n) = (0.0f32, 0.0f32, 0usize);
    for q in word.iter().flat_map(|&i| ph[i].iter()) {
        sx += q.0;
        sy += q.1;
        n += 1;
    }
    if n == 0 {
        (0.0, 0.0)
    } else {
        (sx / n as f32, sy / n as f32)
    }
}

fn word_cx(word: &[usize], ph: &[Vec<(f32, f32)>]) -> f32 {
    let (l, r) = word_x(word, ph);
    (l + r) / 2.0
}

fn word_bottom(word: &[usize], ph: &[Vec<(f32, f32)>]) -> f32 {
    word.iter().flat_map(|&i| ph[i].iter().map(|q| q.1)).fold(f32::MIN, f32::max).max(0.0)
}

fn median(v: &[f32]) -> Option<f32> {
    if v.is_empty() {
        return None;
    }
    let mut v = v.to_vec();
    Some(quantile(&mut v, 0.5))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stroke::{PenEvent, PenPhase};

    /// Una «lettera»: un archetto alto `h` con un po' di tremolio, a `x`,
    /// con la base a `y` (coordinate del pad).
    fn letter(x: f32, y: f32, h: f32, seed: u32) -> Stroke {
        let mut s = Stroke::new([0, 0, 0], 2.0);
        for k in 0..30 {
            let t = k as f32 / 29.0;
            let jitter = (((k as u32 * 7919 + seed * 104729) % 97) as f32 / 97.0 - 0.5) * 12.0;
            s.points.push(PenEvent { x: x + t * 300.0 + jitter, y: y - (t * std::f32::consts::PI).sin() * h + jitter, pressure: 0.8, phase: PenPhase::Move });
        }
        s
    }

    /// Tre parole su una riga storta che sale (la base sale di 600 unità per riga).
    fn tilted_line() -> Vec<Stroke> {
        let mut v = Vec::new();
        for w in 0..3 {
            for l in 0..3 {
                let x = 1000.0 + w as f32 * 1500.0 + l as f32 * 330.0;
                let y = 5000.0 - (x - 1000.0) * 0.12;
                v.push(letter(x, y, 500.0, (w * 3 + l) as u32));
            }
        }
        v
    }

    fn geom() -> PageGeom {
        PageGeom::new(View::default(), 21000.0, 29700.0)
    }

    fn base_slope(strokes: &[Stroke], g: &PageGeom) -> f32 {
        // la pendenza delle basi delle parole (y del punto più basso di ogni parola)
        let r = crate::boardread::read(strokes, g, None);
        let l = &r.lines[0];
        let pts: Vec<(f32, f32)> = l.words.iter().map(|w| ((w.bbox[0] + w.bbox[2]) / 2.0, w.bbox[3])).collect();
        (pts.last().unwrap().1 - pts[0].1) / (pts.last().unwrap().0 - pts[0].0)
    }

    #[test]
    fn spenta_non_tocca_niente() {
        let s = tilted_line();
        let g = geom();
        let out = apply(&s, &g, 0);
        assert_eq!(crate::stroke::strokes_sig(&out), crate::stroke::strokes_sig(&s));
        let d = Drawing { strokes: s.clone(), width: 21000.0, height: 29700.0 };
        set_level(0);
        assert_eq!(crate::stroke::strokes_sig(&nice(&d, View::default()).strokes), crate::stroke::strokes_sig(&s));
    }

    #[test]
    fn riga_raddrizzata_e_tratti_lisci() {
        let s = tilted_line();
        let g = geom();
        let before = base_slope(&s, &g);
        assert!(before.abs() > 0.05, "{before}");
        for level in 1..=3u8 {
            let out = apply(&s, &g, level);
            assert_eq!(out.len(), s.len());
            assert!(out.iter().zip(&s).all(|(a, b)| a.points.len() == b.points.len() && a.color == b.color));
            let after = base_slope(&out, &g);
            assert!(after.abs() < before.abs() * if level == 1 { 0.6 } else { 0.25 }, "livello {level}: {before} → {after}");
            // meno tremolio: la lunghezza del tratto si accorcia
            let len = |st: &Stroke| st.points.windows(2).map(|w| ((w[1].x - w[0].x).powi(2) + (w[1].y - w[0].y).powi(2)).sqrt()).sum::<f32>();
            assert!(len(&out[0]) < len(&s[0]), "livello {level}");
            // gli estremi della lisciatura restano (prima delle altre trasformazioni, la forma è la stessa)
            assert!(out.iter().flat_map(|st| st.points.iter()).all(|p| p.x.is_finite() && p.y.is_finite()));
        }
    }

    #[test]
    fn l_esponente_resta_su() {
        // «x²»: la x grande e un 2 piccolo e più in alto, attaccato
        let mut s = Vec::new();
        for w in 0..3 {
            s.push(letter(1000.0 + w as f32 * 1400.0, 5000.0 + w as f32 * 80.0, 500.0, w));
        }
        let exp = letter(1000.0 + 2.0 * 1400.0 + 330.0, 5000.0 + 160.0 - 450.0, 220.0, 9);
        s.push(exp);
        let g = geom();
        let out = apply(&s, &g, 3);
        let top = |st: &Stroke| st.points.iter().map(|p| g.unit(p.x, p.y).1).fold(f32::MAX, f32::min);
        let bottom = |st: &Stroke| st.points.iter().map(|p| g.unit(p.x, p.y).1).fold(f32::MIN, f32::max);
        // l'esponente resta più in alto della base della sua parola
        assert!(bottom(&out[3]) < bottom(&out[2]) - 0.003, "{} {}", bottom(&out[3]), bottom(&out[2]));
        assert!(top(&out[3]) < top(&out[2]) + 0.01);
    }

    #[test]
    fn il_segno_rosso_segue_la_parola() {
        // un cerchio rosso (come quelli dell'AI) attorno alla seconda parola
        let mut s = tilted_line();
        let mut ring = Stroke::new([214, 40, 40], 3.0);
        let (cx, cy) = (1000.0 + 1500.0 + 450.0, 5000.0 - (1500.0 + 450.0) * 0.12 - 250.0);
        for k in 0..=40 {
            let a = k as f32 / 40.0 * std::f32::consts::TAU;
            ring.points.push(PenEvent { x: cx + a.cos() * 700.0, y: cy + a.sin() * 450.0, pressure: 1.0, phase: PenPhase::Move });
        }
        s.push(ring);
        let g = geom();
        let out = apply(&s, &g, 3);
        let center = |pts: &[PenEvent]| {
            let n = pts.len() as f32;
            (pts.iter().map(|p| p.x).sum::<f32>() / n, pts.iter().map(|p| p.y).sum::<f32>() / n)
        };
        let word: Vec<PenEvent> = out[3..6].iter().flat_map(|st| st.points.clone()).collect();
        let word0: Vec<PenEvent> = s[3..6].iter().flat_map(|st| st.points.clone()).collect();
        let (wa, wb) = (center(&word0), center(&word));
        let (ra, rb) = (center(&s[9].points), center(&out[9].points));
        // la parola si è mossa e il cerchio con lei
        assert!((wb.1 - wa.1).abs() > 50.0, "{wa:?} {wb:?}");
        let off0 = (ra.0 - wa.0, ra.1 - wa.1);
        let off1 = (rb.0 - wb.0, rb.1 - wb.1);
        assert!((off0.0 - off1.0).abs() < 120.0 && (off0.1 - off1.1).abs() < 120.0, "{off0:?} {off1:?}");
        assert_eq!(out[9].color, [214, 40, 40]);
    }

    fn stroke_of(pts: &[(f32, f32)]) -> Stroke {
        let mut s = Stroke::new([20, 20, 20], 2.0);
        for &(x, y) in pts {
            s.points.push(PenEvent { x, y, pressure: 0.7, phase: PenPhase::Move });
        }
        s
    }

    #[test]
    fn forme_perfette() {
        let g = geom();
        let wob = |k: usize| ((k * 7919 % 97) as f32 / 97.0 - 0.5) * 120.0;
        // una linea storta e un po' inclinata (2°): diventa orizzontale e dritta
        let line: Vec<(f32, f32)> = (0..60).map(|k| (2000.0 + k as f32 * 150.0, 20000.0 + k as f32 * 150.0 * 0.035 + wob(k))).collect();
        // un cerchio tremolante, un po' più di un giro
        let circle: Vec<(f32, f32)> = (0..80)
            .map(|k| {
                let t = k as f32 / 79.0 * 6.6;
                (10000.0 + 1800.0 * t.cos() + wob(k), 8000.0 + 1800.0 * t.sin() + wob(k + 3))
            })
            .collect();
        // una freccia in un tratto: asta, poi le due alette
        let mut arrow: Vec<(f32, f32)> = (0..40).map(|k| (2000.0 + k as f32 * 150.0, 14000.0 + wob(k) * 0.5)).collect();
        arrow.extend([(7300.0, 13500.0), (7850.0, 14000.0), (7350.0, 14550.0)]);
        // e una parola normale, che non si tocca
        let mut strokes = tilted_line();
        let first_word = strokes.len();
        strokes.push(stroke_of(&line));
        strokes.push(stroke_of(&circle));
        strokes.push(stroke_of(&arrow));
        let (out, shaped) = perfect(&strokes, &g);
        assert_eq!(out.len(), strokes.len());
        // la scrittura resta com'è
        assert!((0..first_word).all(|i| !shaped[i] && out[i].points.iter().zip(&strokes[i].points).all(|(a, b)| a.x == b.x && a.y == b.y)));
        assert!(shaped[first_word] && shaped[first_word + 1] && shaped[first_word + 2], "{shaped:?}");
        // la linea: tutti i punti alla stessa altezza
        let l = &out[first_word].points;
        let (lo, hi) = l.iter().fold((f32::MAX, f32::MIN), |m, p| (m.0.min(p.y), m.1.max(p.y)));
        assert!(hi - lo < 1.0, "{lo} {hi}");
        // il cerchio: tutti alla stessa distanza dal centro
        let c = &out[first_word + 1].points;
        let bx = c.iter().fold([f32::MAX, f32::MAX, f32::MIN, f32::MIN], |b, p| [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)]);
        let (cx, cy) = ((bx[0] + bx[2]) / 2.0, (bx[1] + bx[3]) / 2.0);
        let r: Vec<f32> = c.iter().map(|p| ((p.x - cx).powi(2) + (p.y - cy).powi(2)).sqrt()).collect();
        let (rmin, rmax) = r.iter().fold((f32::MAX, f32::MIN), |m, &x| (m.0.min(x), m.1.max(x)));
        assert!((rmax - rmin) / rmax < 0.02, "{rmin} {rmax}");
        assert_eq!(c.len(), circle.len());
        // la freccia: asta orizzontale e alette uguali
        let a = &out[first_word + 2].points;
        assert_eq!(a.len(), arrow.len());
        assert!((a[0].y - 14000.0).abs() < 200.0);
        let tip = a.iter().max_by(|p, q| p.x.total_cmp(&q.x)).unwrap();
        let ends: Vec<&PenEvent> = a.iter().filter(|p| p.x < tip.x - 100.0 && (p.y - tip.y).abs() > 200.0).collect();
        let up = ends.iter().map(|p| p.y).fold(f32::MAX, f32::min);
        let down = ends.iter().map(|p| p.y).fold(f32::MIN, f32::max);
        assert!(((tip.y - up) - (down - tip.y)).abs() < 40.0, "{} {} {}", up, tip.y, down);
    }

    #[test]
    fn i_disegni_restano() {
        // un grande riquadro (un disegno) e una riga di testo sotto
        let mut s = tilted_line();
        let mut boxs = Stroke::new([0, 0, 0], 2.0);
        for &(x, y) in &[(2000.0, 9000.0), (12000.0, 9000.0), (12000.0, 16000.0), (2000.0, 16000.0), (2000.0, 9000.0)] {
            boxs.points.push(PenEvent { x, y, pressure: 1.0, phase: PenPhase::Move });
        }
        s.push(boxs.clone());
        let out = apply(&s, &geom(), 3);
        let last = out.last().unwrap();
        // gli angoli del disegno non si spostano (al più la lisciatura dei punti intermedi)
        assert_eq!((last.points[0].x, last.points[0].y), (2000.0, 9000.0));
        assert_eq!((last.points[4].x, last.points[4].y), (2000.0, 9000.0));
    }

    /// Prima e dopo, per guardarle: BEAUTIFY_DIR=cartella dell'archivio,
    /// BEAUTIFY_OUT=cartella dei PNG (`cargo test anteprima -- --ignored`).
    #[test]
    #[ignore]
    fn anteprima() {
        let (Ok(dir), Ok(outd)) = (std::env::var("BEAUTIFY_DIR"), std::env::var("BEAUTIFY_OUT")) else { return };
        let lib = crate::library::Library::open_in(std::path::Path::new(&dir));
        let view = View { rot: 1, flip_x: false, flip_y: false };
        for k in 0..lib.entries.len() {
            let e = &lib.entries[k];
            if std::env::var("BEAUTIFY_ID").is_ok_and(|id| id != e.id) {
                continue;
            }
            let t0 = std::time::Instant::now();
            for level in 0..=3u8 {
                if std::env::var("BEAUTIFY_LEVEL").is_ok_and(|l| l != level.to_string()) {
                    continue;
                }
                set_level(level);
                let p = crate::scene::Page::from_drawing_nice(&e.drawing, view);
                std::fs::write(format!("{outd}/{}-{level}.png", e.id), crate::export::page_to_png(&p, 1100).unwrap()).unwrap();
            }
            eprintln!("{}: {:?}", e.id, t0.elapsed());
        }
        set_level(0);
    }

    #[test]
    #[ignore]
    fn forme_trovate() {
        let (Ok(dir), Ok(id)) = (std::env::var("BEAUTIFY_DIR"), std::env::var("BEAUTIFY_ID")) else { return };
        let lib = crate::library::Library::open_in(std::path::Path::new(&dir));
        let e = lib.get(&id).unwrap();
        let view = View { rot: 1, flip_x: false, flip_y: false };
        let (pw, ph0) = crate::scene::page_dims(&e.drawing);
        let g = PageGeom::new(view, pw, ph0);
        let aspect = g.w / g.h;
        let r = crate::boardread::read(&e.drawing.strokes, &g, None);
        let (_, shaped) = perfect(&e.drawing.strokes, &g);
        for (i, s) in e.drawing.strokes.iter().enumerate().rev().take(3) {
            let ph: Vec<(f32, f32)> = s.points.iter().map(|q| { let (u, v) = g.unit(q.x, q.y); (u * aspect, v) }).collect();
            let total: f32 = ph.windows(2).map(|w| dist(w[0], w[1])).sum();
            let simp = rdp(&ph, (0.04 * total).max(0.12 * r.char_h));
            eprintln!("{i}: shaped {} hh {:.4} total {:.4} simp {:?} arrow {:?}", shaped[i], r.char_h, total, simp, arrow(&ph, r.char_h).map(|a| a.len()));
        }
    }

    #[test]
    #[ignore]
    fn righe_trovate() {
        let (Ok(dir), Ok(id)) = (std::env::var("BEAUTIFY_DIR"), std::env::var("BEAUTIFY_ID")) else { return };
        let lib = crate::library::Library::open_in(std::path::Path::new(&dir));
        let e = lib.get(&id).unwrap();
        let view = View { rot: 1, flip_x: false, flip_y: false };
        let (pw, ph0) = crate::scene::page_dims(&e.drawing);
        let g = PageGeom::new(view, pw, ph0);
        let aspect = g.w / g.h;
        let ph: Vec<Vec<(f32, f32)>> = e.drawing.strokes.iter().map(|s| s.points.iter().map(|q| { let (u, v) = g.unit(q.x, q.y); (u * aspect, v) }).collect()).collect();
        let r = crate::boardread::read(&e.drawing.strokes, &g, None);
        let boxes: Vec<Option<[f32; 4]>> = ph.iter().map(|p| pbox(p)).collect();
        let inkf: Vec<bool> = e.drawing.strokes.iter().map(|s| !s.highlight).collect();
        let ink: Vec<usize> = (0..ph.len()).filter(|&i| inkf[i] && !ph[i].is_empty() && !is_mark(i, &boxes, &inkf, r.char_h)).collect();
        eprintln!("hh {}", r.char_h);
        for (m, words) in group_lines(&ink, &ph, r.char_h) {
            let b = m.iter().filter_map(|&i| pbox(&ph[i])).reduce(union).unwrap();
            let ws: Vec<String> = words.iter().map(|w| { let b = w.iter().filter_map(|&i| pbox(&ph[i])).reduce(union).unwrap(); format!("[{:.3} {:.3}-{:.3} n{}]", b[0], b[1], b[3], w.len()) }).collect();
            eprintln!("riga {:.3},{:.3}..{:.3},{:.3}: {}", b[0], b[1], b[2], b[3], ws.join(" "));
        }
    }
}
