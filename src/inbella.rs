//! «Trasforma in bella»: un clic, e la scrittura della lavagna diventa la
//! sua bella copia. L'AI legge le righe (formule comprese, in LaTeX); al
//! posto dei tratti scritti col pennino vanno tratti nuovi, in una
//! scrittura a mano chiara (il carattere «naturale») con la matematica
//! impaginata come sul quaderno ordinato (`mathhand`).
//!
//! È inchiostro vero, come quello del pennino: si salva, si esporta, si
//! cancella, e Annulla (Cmd+Z) riporta la scrittura di prima. I disegni, le
//! righe che non si leggono, gli evidenziatori e i segni dell'AI restano
//! come sono; le righe già trasformate non si rileggono.

use crate::boardread::Reading;
use crate::scene::{PageGeom, View};
use crate::stroke::{Drawing, PenEvent, PenPhase, Stroke};
use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::sync::Mutex;

/// Le impronte dei tratti già in bella (file `neat_marks.txt` nella cartella
/// dei dati): una riga fatta solo di questi non si rilegge.
static NEAT: Mutex<Option<Vec<u64>>> = Mutex::new(None);
const NEAT_FILE: &str = "neat_marks.txt";
const NEAT_KEEP: usize = 40_000;

/// L'impronta di un tratto, a prova di salvataggio: coordinate arrotondate
/// (l'archivio le salva con tre decimali).
fn sig(s: &Stroke) -> u64 {
    let mut h: u64 = 0x6a09_e667_f3bc_c908;
    let mut mix = |v: u64| h = (h ^ v).wrapping_mul(0x100_0000_01b3);
    mix(s.points.len() as u64);
    for p in &s.points {
        mix((p.x.round() as i64 as u64) << 32 ^ (p.y.round() as i64 as u64 & 0xffff_ffff));
    }
    h
}

fn neat_list<R>(f: impl FnOnce(&mut Vec<u64>) -> R) -> R {
    let mut g = NEAT.lock().unwrap();
    let v = g.get_or_insert_with(|| {
        if cfg!(test) {
            return Vec::new();
        }
        std::fs::read_to_string(NEAT_FILE).map(|t| t.lines().filter_map(|l| u64::from_str_radix(l.trim(), 16).ok()).collect()).unwrap_or_default()
    });
    f(v)
}

fn is_neat(s: &Stroke) -> bool {
    let k = sig(s);
    neat_list(|v| v.contains(&k))
}

fn mark_neat(strokes: &[Stroke]) {
    let new: Vec<u64> = strokes.iter().map(sig).collect();
    neat_list(|v| {
        v.extend(&new);
        let n = v.len();
        if n > NEAT_KEEP {
            v.drain(..n - NEAT_KEEP);
        }
        if !cfg!(test) {
            let t: String = v.iter().map(|k| format!("{k:x}\n")).collect();
            if let Err(e) = crate::web::write_atomic(std::path::Path::new(NEAT_FILE), t.as_bytes()) {
                tracing::warn!("{NEAT_FILE}: {e}");
            }
        }
    });
}

/// Un tratto da trasformare: scrittura del pennino (non evidenziatore, non
/// un segno dell'AI, non già in bella).
fn writable(s: &Stroke) -> bool {
    !s.highlight && !s.points.is_empty() && !crate::boardagent::is_ai_stroke(s) && !is_neat(s)
}

/// L'impronta di una riga: i suoi tratti, con le coordinate arrotondate.
fn line_key(strokes: &[Stroke], idx: &[usize]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut mix = |v: u64| h = (h ^ v).wrapping_mul(0x100_0000_01b3);
    let mut idx = idx.to_vec();
    idx.sort_unstable();
    for &i in &idx {
        if let Some(s) = strokes.get(i) {
            mix(sig(s));
        }
    }
    h
}

/// Le righe da leggere (nome di `boardread` e impronta): quelle con
/// scrittura del pennino.
pub fn pending(strokes: &[Stroke], g: &PageGeom) -> Vec<(String, u64)> {
    let r = crate::boardread::read(strokes, g, None);
    lines_of(&r, strokes, g)
}

/// I tratti nelle unità della pagina (x come y).
fn physical(strokes: &[Stroke], g: &PageGeom) -> Vec<Vec<(f32, f32)>> {
    let aspect = g.w / g.h;
    strokes
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
        .collect()
}

fn dist2(a: (f32, f32), b: (f32, f32)) -> f32 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

fn bbox(pts: &[(f32, f32)]) -> (f32, f32, f32, f32) {
    pts.iter().fold((f32::MAX, f32::MIN, f32::MAX, f32::MIN), |b, p| (b.0.min(p.0), b.1.max(p.0), b.2.min(p.1), b.3.max(p.1)))
}

/// Le lettere da trasformare di una riga: tutta la scrittura del pennino
/// (anche le graffe e le parentesi alte, che la bella copia rifà), tranne il
/// giro attorno a una parola e la sottolineatura: una linea lunga e piatta con niente sotto (una linea di
/// frazione ha sotto il denominatore, e va rifatta).
fn letters(line: &[usize], strokes: &[Stroke], ph: &[Vec<(f32, f32)>], hh: f32) -> Vec<usize> {
    let ink: Vec<usize> = line.iter().copied().filter(|&i| strokes.get(i).is_some_and(writable) && !ph[i].is_empty()).collect();
    let boxes: Vec<(f32, f32, f32, f32)> = ink.iter().map(|&i| bbox(&ph[i])).collect();
    (0..ink.len())
        .filter(|&a| {
            let (x0, x1, y0, y1) = boxes[a];
            // un giro attorno ad altri tratti (il cerchio attorno a una parola): resta
            let m = 0.1 * hh;
            let pts = &ph[ink[a]];
            let closed = dist2(pts[0], pts[pts.len() - 1]) < 0.3 * (x1 - x0).max(y1 - y0);
            let around = closed && boxes.iter().enumerate().any(|(b, &(bx0, bx1, by0, by1))| b != a && by1 - by0 >= 0.4 * hh && bx0 > x0 + m && bx1 < x1 - m && by0 > y0 + m && by1 < y1 - m);
            if around {
                return false;
            }
            let flat = x1 - x0 > 3.0 * hh && y1 - y0 < 0.35 * hh;
            if !flat {
                return true;
            }
            let y = (y0 + y1) / 2.0;
            // qualcosa sotto, nella sua larghezza: è una frazione
            boxes.iter().enumerate().any(|(b, &(bx0, bx1, by0, by1))| b != a && bx1 > x0 && bx0 < x1 && (by0 + by1) / 2.0 > y + 0.3 * hh)
        })
        .map(|a| ink[a])
        .collect()
}

/// Una «riga» fatta solo di forme (una freccia, un cerchio) o di trattini
/// (una linea tratteggiata): un disegno, da rifare e non da far leggere.
fn drawing_line(line: &[usize], strokes: &[Stroke], ph: &[Vec<(f32, f32)>], hh: f32) -> bool {
    line.iter().filter(|&&i| writable(&strokes[i])).all(|&i| crate::shapes::is_shape(&ph[i], hh) || crate::shapes::is_dash(&ph[i], hh))
}

fn lines_of(r: &Reading, strokes: &[Stroke], g: &PageGeom) -> Vec<(String, u64)> {
    let ph = physical(strokes, g);
    let hh = r.char_h.clamp(0.012, 0.035);
    r.lines
        .iter()
        .filter(|l| !letters(&l.group.strokes, strokes, &ph, r.char_h).is_empty())
        // una «riga» fatta solo di forme (una freccia, un cerchio) è un disegno:
        // non si fa leggere, si rifà
        .filter(|l| !drawing_line(&l.group.strokes, strokes, &ph, hh))
        .map(|l| (l.group.id.clone(), line_key(strokes, &l.group.strokes)))
        .collect()
}

/// I tratti nel riquadro `rect` (u0, v0, u1, v1 in frazioni della pagina):
/// quelli col centro dentro.
pub fn select(strokes: &[Stroke], g: &PageGeom, rect: [f32; 4]) -> Vec<usize> {
    (0..strokes.len())
        .filter(|&i| {
            let s = &strokes[i];
            if s.points.is_empty() {
                return false;
            }
            let (mut u0, mut u1, mut v0, mut v1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
            for p in &s.points {
                let (u, v) = g.unit(p.x, p.y);
                u0 = u0.min(u);
                u1 = u1.max(u);
                v0 = v0.min(v);
                v1 = v1.max(v);
            }
            let (cu, cv) = ((u0 + u1) / 2.0, (v0 + v1) / 2.0);
            cu >= rect[0] && cu <= rect[2] && cv >= rect[1] && cv <= rect[3]
        })
        .collect()
}

/// Fa leggere all'AI le righe di una zona (bloccante: in un thread).
/// `strokes` sono solo i tratti della zona; all'AI va l'immagine ritagliata
/// sulla zona `crop`, in alta risoluzione. `agent`: chi legge (vuoto = gli
/// agenti di «Lettura dei fogli»). Torna il testo di ogni riga letta, per
/// impronta (vuoto = da lasciare), e il nome dell'agente che ha letto.
pub fn recognize_region(strokes: &[Stroke], width: f32, height: f32, view: View, crop: [f32; 4], agent: &str) -> Result<(HashMap<u64, String>, String)> {
    let d = Drawing { strokes: strokes.to_vec(), width, height };
    let (pw, ph) = crate::scene::page_dims(&d);
    let g = PageGeom::new(view, pw, ph);
    let reading = crate::boardread::read(strokes, &g, None);
    let lines = lines_of(&reading, strokes, &g);
    if lines.is_empty() {
        return Ok((HashMap::new(), String::new()));
    }
    let page = crate::scene::Page::from_drawing(&d, view);
    let png = crate::boardread::labelled_png_crop(&page, &reading, crop)?;
    let ids: Vec<&str> = lines.iter().map(|(id, _)| id.as_str()).collect();
    let prompt = PROMPT.replace("{RIGHE}", &ids.join(", "));
    let reply = crate::ai::read_image_by(std::sync::Arc::new(png), &prompt, agent)?;
    let map = parse_reply(&reply.text)?;
    Ok((lines.iter().filter_map(|(id, k)| map.get(id).map(|v| (*k, v.clone().unwrap_or_default()))).collect(), reply.provider))
}

const PROMPT: &str = "Questa è una pagina (o una sua parte) scritta a mano col pennino. Accanto a ogni riga c'è il suo nome in rosa (r1, r2…) e ogni parola ha un riquadro.\n\
Trascrivi esattamente il contenuto di queste righe: {RIGHE}.\n\
Regole:\n\
- copia fedelmente, errori compresi: non correggere, non completare, non aggiungere niente;\n\
- tutta la matematica in LaTeX fra $…$: \\frac{a}{b} per le frazioni scritte in colonna, ^ e _ per esponenti e pedici, \\sqrt{} per le radici, \\begin{cases}…\\end{cases} per i sistemi, \\cdot, \\le, \\ge, \\neq, \\Rightarrow, le lettere greche (\\alpha…), \\{ \\} per le graffe, \\sum, \\int, \\lim;\n\
- il resto come testo normale, nella lingua in cui è scritto;\n\
- se una riga è un disegno, uno scarabocchio o non si legge, metti null.\n\
Rispondi SOLO con un oggetto JSON, una chiave per riga: {\"r1\": \"…\", \"r2\": null}.";

/// La risposta: un oggetto JSON (anche dentro ```json … ```).
fn parse_reply(text: &str) -> Result<HashMap<String, Option<String>>> {
    let (Some(a), Some(b)) = (text.find('{'), text.rfind('}')) else { bail!("l'AI non ha risposto con le righe (niente JSON)") };
    let v: serde_json::Value = serde_json::from_str(&text[a..=b]).context("risposta dell'AI non leggibile")?;
    let obj = v.as_object().context("risposta dell'AI non leggibile")?;
    Ok(obj
        .iter()
        .map(|(k, v)| (k.trim().to_lowercase(), v.as_str().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())))
        .collect())
}

/// Com'è andata: righe riscritte e forme rifatte.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Done {
    pub lines: usize,
    pub shapes: usize,
}

/// La pagina in bella: le righe lette (per impronta) tolte e riscritte al
/// loro posto, e i disegni con le forme rifatte perfette (`shapes`).
/// `None` se non cambia niente. Le righe cambiate nel frattempo (impronta
/// diversa) restano come sono.
pub fn apply(strokes: &[Stroke], texts: &HashMap<u64, String>, g: &PageGeom) -> Option<(Vec<Stroke>, Done)> {
    apply_with(strokes, texts, g, None)
}

/// Come `apply`, per i tratti di una zona: la scrittura nuova non va oltre
/// il bordo destro della zona (`right`, in frazioni della larghezza), così
/// non finisce sopra la colonna accanto.
pub fn apply_region(strokes: &[Stroke], texts: &HashMap<u64, String>, g: &PageGeom, right: f32) -> Option<(Vec<Stroke>, Done)> {
    apply_with(strokes, texts, g, Some(right))
}

fn apply_with(strokes: &[Stroke], texts: &HashMap<u64, String>, g: &PageGeom, right: Option<f32>) -> Option<(Vec<Stroke>, Done)> {
    if strokes.is_empty() {
        return None;
    }
    let reading = crate::boardread::read(strokes, g, None);
    let aspect = g.w / g.h;
    let hh = reading.char_h;
    let ph = physical(strokes, g);
    let mut hidden = vec![false; strokes.len()];
    let mut added: Vec<Stroke> = Vec::new();
    let mut n = 0;
    for line in &reading.lines {
        let k = line_key(strokes, &line.group.strokes);
        let Some(text) = texts.get(&k).filter(|t| !t.trim().is_empty()) else { continue };
        // si tolgono le lettere, non i segni
        let letters = letters(&line.group.strokes, strokes, &ph, hh);
        if letters.is_empty() {
            continue;
        }
        let b = crate::mathhand::line_box(crate::handwrite::Style::Naturale, text);
        if b.strokes.is_empty() {
            continue;
        }
        // dove: a sinistra della riga, sulla sua riga di base, alto come la sua scrittura
        let x0 = letters.iter().flat_map(|&i| ph[i].iter().map(|p| p.0)).fold(f32::MAX, f32::min);
        // la riga di base dalle lettere normali (non dalle graffe alte)
        let body: Vec<usize> = letters.iter().copied().filter(|&i| bbox(&ph[i]).3 - bbox(&ph[i]).2 < 2.4 * hh).collect();
        let body = if body.is_empty() { letters.clone() } else { body };
        let (base, xh) = base_and_xheight(&body, &ph, hh).unwrap_or_else(|| {
            let b = letters.iter().flat_map(|&i| ph[i].iter().map(|p| p.1)).fold(f32::MIN, f32::max);
            (b, 0.6 * hh)
        });
        // unità di Hershey → unità della pagina: il corpo delle minuscole (14) come il tuo
        let mut s = (1.15 * xh / 14.0).clamp(0.7 * hh / 21.0, 1.15 * hh / 21.0);
        let edge = right.map_or(0.985, |r| r.min(0.985)) * aspect;
        let room = (edge - x0).max(0.05);
        if b.w * s > room {
            s = room / b.w;
        }
        // colore e spessore: quelli più usati nella riga
        let mut colors: Vec<([u8; 3], usize)> = Vec::new();
        for &i in &letters {
            match colors.iter_mut().find(|(c, _)| *c == strokes[i].color) {
                Some((_, n)) => *n += strokes[i].points.len(),
                None => colors.push((strokes[i].color, strokes[i].points.len())),
            }
        }
        let color = colors.iter().max_by_key(|(_, n)| *n).map(|(c, _)| *c).unwrap_or([20, 20, 20]);
        let mut widths: Vec<f32> = letters.iter().map(|&i| strokes[i].width).collect();
        widths.sort_by(f32::total_cmp);
        let width = widths[widths.len() / 2];
        for st in b.strokes {
            if st.len() < 2 {
                continue;
            }
            let mut stroke = Stroke::new(color, width);
            let m = st.len();
            for (j, (x, y)) in st.into_iter().enumerate() {
                let (px, py) = g.pad((x0 + x * s) / aspect, base + y * s);
                let phase = if j == 0 { PenPhase::Down } else if j + 1 == m { PenPhase::Up } else { PenPhase::Move };
                stroke.points.push(PenEvent { x: px, y: py, pressure: 0.6, phase });
            }
            added.push(stroke);
        }
        for i in letters {
            hidden[i] = true;
        }
        n += 1;
    }
    // i disegni: i tratti del disegno, quelli delle righe che l'AI ha detto
    // disegni (null), e i tratti grandi rimasti (una sottolineatura, una
    // freccia fra le righe)
    let mut drawing = vec![false; strokes.len()];
    if let Some(d) = &reading.drawing {
        for &i in &d.strokes {
            drawing[i] = true;
        }
    }
    let hh_shapes = hh.clamp(0.012, 0.035);
    for line in &reading.lines {
        let k = line_key(strokes, &line.group.strokes);
        if texts.get(&k).is_some_and(|t| t.trim().is_empty()) || drawing_line(&line.group.strokes, strokes, &ph, hh_shapes) {
            for &i in &line.group.strokes {
                drawing[i] = true;
            }
        }
    }
    // senza scrittura sul foglio l'altezza di una lettera è una stima larga:
    // per le forme serve una misura da quaderno
    let hh = hh.clamp(0.012, 0.035);
    let cand: Vec<usize> = (0..strokes.len())
        .filter(|&i| !hidden[i] && writable(&strokes[i]))
        .filter(|&i| {
            let (x0, x1, y0, y1) = bbox(&ph[i]);
            drawing[i] || (x1 - x0).max(y1 - y0) >= 2.2 * hh
        })
        .collect();
    // fuori dai disegni solo le forme riconosciute: una parola lunga in
    // corsivo non si «liscia»
    let shaped: Vec<crate::shapes::Shaped> = crate::shapes::perfect(&ph, &cand, hh).into_iter().filter(|s| drawing[s.index] || s.kind != crate::shapes::Kind::Curve).collect();
    if added.is_empty() && shaped.is_empty() {
        return None;
    }
    let mut new_shape: Vec<Option<Stroke>> = vec![None; strokes.len()];
    for sh in &shaped {
        let o = &strokes[sh.index];
        let mut st = if o.highlight { Stroke::highlighter(o.color, o.width) } else { Stroke::new(o.color, o.width) };
        let m = sh.points.len();
        let pressure = o.points.get(o.points.len() / 2).map_or(0.6, |q| q.pressure);
        for (j, &(x, y)) in sh.points.iter().enumerate() {
            let (px, py) = g.pad(x / aspect, y);
            let phase = if j == 0 { PenPhase::Down } else if j + 1 == m { PenPhase::Up } else { PenPhase::Move };
            st.points.push(PenEvent { x: px, y: py, pressure, phase });
        }
        new_shape[sh.index] = Some(st);
    }
    let new_shapes: Vec<Stroke> = new_shape.iter().flatten().cloned().collect();
    mark_neat(&added);
    mark_neat(&new_shapes);
    let mut out: Vec<Stroke> = Vec::with_capacity(strokes.len() + added.len());
    for (i, s) in strokes.iter().enumerate() {
        if hidden[i] {
            continue;
        }
        out.push(new_shape[i].take().unwrap_or_else(|| s.clone()));
    }
    out.extend(added);
    Some((out, Done { lines: n, shapes: shaped.len() }))
}

/// Riga di base e corpo delle minuscole di una riga (y verso il basso): la
/// base è dove la penna tocca il fondo più spesso, il corpo dove tocca la
/// cima; così una «g», una «/», un esponente non spostano la riga.
fn base_and_xheight(idx: &[usize], ph: &[Vec<(f32, f32)>], hh: f32) -> Option<(f32, f32)> {
    let (mut bottoms, mut tops, mut lows, mut highs) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for &i in idx {
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
    }
    let q = |v: &[f32], f: f32| {
        let mut v = v.to_vec();
        (!v.is_empty()).then(|| quantile(&mut v, f))
    };
    let base = q(&bottoms, 0.35).or_else(|| q(&lows, 0.5))?;
    let top = q(&tops, 0.65).or_else(|| q(&highs, 0.5))?;
    Some((base, (base - top).max(0.0)))
}

fn quantile(v: &mut [f32], q: f32) -> f32 {
    v.sort_by(f32::total_cmp);
    v[((v.len() - 1) as f32 * q).round() as usize]
}

/// I fondi e le cime di un tratto: dove la penna, scendendo, torna a salire
/// (e viceversa) di almeno `min`.
pub(crate) fn turns(pts: &[(f32, f32)], min: f32) -> (Vec<f32>, Vec<f32>) {
    let (mut bottoms, mut tops) = (Vec::new(), Vec::new());
    let Some(first) = pts.first() else { return (bottoms, tops) };
    let mut ext = first.1;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn risposta_json() {
        let m = parse_reply("Ecco:\n```json\n{\"r1\": \"Risolvi $x^2=4$\", \"R2\": null, \"r3\": \"  \"}\n```").unwrap();
        assert_eq!(m["r1"].as_deref(), Some("Risolvi $x^2=4$"));
        assert_eq!(m["r2"], None);
        assert_eq!(m["r3"], None);
        assert!(parse_reply("non so").is_err());
    }

    /// Una riga di «lettere» (archetti) sotto un'altra.
    fn page() -> Vec<Stroke> {
        let mut strokes = Vec::new();
        for (row, y) in [(0, 6000.0), (1, 9000.0)] {
            for k in 0..6 {
                let mut s = Stroke::new([30, 60, 160], 2.0);
                for t in 0..20 {
                    let f = t as f32 / 19.0;
                    s.points.push(PenEvent { x: 2000.0 + k as f32 * 700.0 + f * 500.0 + row as f32, y: y - (f * 3.14).sin() * 500.0, pressure: 0.7, phase: PenPhase::Move });
                }
                strokes.push(s);
            }
        }
        strokes
    }

    fn same(a: &Stroke, b: &Stroke) -> bool {
        sig(a) == sig(b)
    }

    #[test]
    fn riga_trasformata() {
        let g = PageGeom::new(View::default(), 21000.0, 29700.0);
        let strokes = page();
        let lines = pending(&strokes, &g);
        assert_eq!(lines.len(), 2);
        // la prima riga letta come una formula, la seconda illeggibile
        let texts: HashMap<u64, String> = [(lines[0].1, "$\\frac{x+1}{2} = 3$".to_string()), (lines[1].1, String::new())].into();
        let (out, n) = apply(&strokes, &texts, &g).unwrap();
        assert_eq!(n.lines, 1);
        // la seconda riga è ancora lì, tale e quale
        for s in &strokes[6..] {
            assert!(out.iter().any(|o| same(o, s)));
        }
        // la prima no: al suo posto la formula, dello stesso colore
        for s in &strokes[..6] {
            assert!(!out.iter().any(|o| same(o, s)));
        }
        let new: Vec<&Stroke> = out.iter().filter(|o| !strokes.iter().any(|s| same(s, o))).collect();
        assert!(new.len() > 5);
        assert!(new.iter().all(|s| s.color == [30, 60, 160]));
        // sulla riga: a sinistra dov'era, e non lontano in altezza
        let xs: Vec<f32> = new.iter().flat_map(|s| s.points.iter().map(|p| p.x)).collect();
        let ys: Vec<f32> = new.iter().flat_map(|s| s.points.iter().map(|p| p.y)).collect();
        let x0 = xs.iter().copied().fold(f32::MAX, f32::min);
        assert!((x0 - 2000.0).abs() < 600.0, "{x0}");
        let (y0, y1) = (ys.iter().copied().fold(f32::MAX, f32::min), ys.iter().copied().fold(f32::MIN, f32::max));
        assert!(y0 > 4000.0 && y1 < 7500.0, "{y0} {y1}");
        // la riga in bella non si rilegge: resta solo la seconda
        let again = pending(&out, &g);
        assert_eq!(again.len(), 1, "{again:?}");
    }

    fn line_stroke(pts: &[(f32, f32)]) -> Stroke {
        let mut s = Stroke::new([20, 20, 20], 2.0);
        for &(x, y) in pts {
            s.points.push(PenEvent { x, y, pressure: 0.7, phase: PenPhase::Move });
        }
        s
    }

    #[test]
    fn graffe_via_sottolineatura_resta() {
        let g = PageGeom::new(View::default(), 21000.0, 29700.0);
        let mut strokes = page()[..6].to_vec();
        // una graffa alta a sinistra (due righe di sistema), una sottolineatura sotto
        let brace: Vec<(f32, f32)> = (0..30).map(|t| {
            let f = t as f32 / 29.0;
            (1500.0 - (f * 6.28).sin().abs() * 200.0, 4800.0 + f * 3200.0)
        }).collect();
        strokes.push(line_stroke(&brace));
        strokes.push(line_stroke(&[(1900.0, 6300.0), (4000.0, 6320.0), (6200.0, 6300.0)]));
        let lines = pending(&strokes, &g);
        assert_eq!(lines.len(), 1, "{lines:?}");
        let texts: HashMap<u64, String> = [(lines[0].1, "$\\begin{cases} x = 1 \\\\ y = 2 \\end{cases}$".to_string())].into();
        let (out, _) = apply(&strokes, &texts, &g).unwrap();
        assert!(!out.iter().any(|o| same(o, &strokes[6])), "la graffa a mano resta");
        // la sottolineatura resta, rifatta dritta
        assert!(
            out.iter().any(|o| o.points.iter().all(|q| (q.y - o.points[0].y).abs() < 1.0) && o.points[0].x < 2100.0 && o.points.last().unwrap().x > 6000.0),
            "la sottolineatura sparisce"
        );
    }

    #[test]
    fn linea_di_frazione_rifatta() {
        let g = PageGeom::new(View::default(), 21000.0, 29700.0);
        let mut strokes = page()[..6].to_vec();
        // la linea di frazione e il denominatore sotto
        strokes.push(line_stroke(&[(1900.0, 6300.0), (4000.0, 6320.0), (6200.0, 6300.0)]));
        let mut den = page()[..3].to_vec();
        for s in den.iter_mut() {
            for p in s.points.iter_mut() {
                p.y += 1100.0;
            }
        }
        strokes.extend(den);
        let lines = pending(&strokes, &g);
        let texts: HashMap<u64, String> = lines.iter().map(|(_, k)| (*k, "$\\frac{abc}{d}$".to_string())).collect();
        let (out, _) = apply(&strokes, &texts, &g).unwrap();
        assert!(!out.iter().any(|o| same(o, &strokes[6])), "la linea di frazione a mano resta");
    }

    /// Un disegno fatto «a mano» prima e dopo, in due PNG:
    /// FORME_PNG=prefisso cargo test -- --ignored disegno_prima_dopo
    #[test]
    #[ignore]
    fn disegno_prima_dopo() {
        let Ok(out) = std::env::var("FORME_PNG") else { return };
        let g = PageGeom::new(View::default(), 21000.0, 29700.0);
        // tremolio della mano e un po' di storto
        let hand = |pts: Vec<(f32, f32)>, seed: f32| -> Stroke {
            let mut s = Stroke::new([20, 20, 30], 2.0);
            let n = pts.len();
            for (k, (x, y)) in pts.into_iter().enumerate() {
                let t = k as f32;
                let (dx, dy) = (35.0 * (t * 0.9 + seed).sin() + 20.0 * (t * 0.31 + seed * 2.0).cos(), 35.0 * (t * 0.7 + seed).cos() + 20.0 * (t * 0.23).sin());
                let phase = if k == 0 { PenPhase::Down } else if k + 1 == n { PenPhase::Up } else { PenPhase::Move };
                s.points.push(PenEvent { x: x + dx, y: y + dy, pressure: 0.7, phase });
            }
            s
        };
        let seg = |a: (f32, f32), b: (f32, f32), n: usize| -> Vec<(f32, f32)> { (0..n).map(|k| { let f = k as f32 / (n - 1) as f32; (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f) }).collect() };
        let mut strokes = Vec::new();
        // assi con le frecce (punte a parte)
        strokes.push(hand(seg((2000.0, 9000.0), (10000.0, 9150.0), 60), 1.0));
        strokes.push(hand(seg((9700.0, 8850.0), (10000.0, 9150.0), 8).into_iter().chain(seg((10000.0, 9150.0), (9680.0, 9420.0), 8)).collect(), 2.0));
        strokes.push(hand(seg((5000.0, 13000.0), (5120.0, 3000.0), 60), 3.0));
        strokes.push(hand(seg((4820.0, 3300.0), (5120.0, 3000.0), 8).into_iter().chain(seg((5120.0, 3000.0), (5400.0, 3320.0), 8)).collect(), 4.0));
        // una parabola
        strokes.push(hand((0..=60).map(|k| { let x = -1.0 + 2.0 * k as f32 / 60.0; (5050.0 + 2800.0 * x, 10500.0 - 5500.0 * (1.0 - x * x)) }).collect(), 5.0));
        // un cerchio, un triangolo in tre tratti, un rettangolo in un tratto, un arco, una freccia
        strokes.push(hand((0..=70).map(|k| { let t = k as f32 / 70.0 * 6.5; (15000.0 + 2200.0 * t.cos(), 6000.0 + 2000.0 * t.sin()) }).collect(), 6.0));
        let (a, b, c) = ((12500.0, 14000.0), (17500.0, 14200.0), (14500.0, 10500.0));
        strokes.push(hand(seg(a, b, 30), 7.0));
        strokes.push(hand(seg((b.0 + 150.0, b.1 - 120.0), c, 30), 8.0));
        strokes.push(hand(seg((c.0 - 100.0, c.1 + 130.0), (a.0 + 160.0, a.1 - 90.0), 30), 9.0));
        strokes.push(hand([seg((2500.0, 16000.0), (8500.0, 16250.0), 25), seg((8500.0, 16250.0), (8600.0, 20000.0), 20), seg((8600.0, 20000.0), (2600.0, 19800.0), 25), seg((2600.0, 19800.0), (2500.0, 16100.0), 20)].concat(), 10.0));
        strokes.push(hand((0..=40).map(|k| { let t = 3.3 + k as f32 / 40.0 * 2.4; (15000.0 + 2500.0 * t.cos(), 19500.0 + 2500.0 * t.sin()) }).collect(), 11.0));
        strokes.push(hand(seg((3000.0, 23000.0), (9000.0, 23300.0), 40).into_iter().chain(seg((9000.0, 23300.0), (8500.0, 22900.0), 6)).chain(seg((8500.0, 22900.0), (9000.0, 23300.0), 3)).chain(seg((9000.0, 23300.0), (8450.0, 23700.0), 6)).collect(), 12.0));
        let before = strokes.clone();
        let (after, done) = apply(&strokes, &HashMap::new(), &g).unwrap();
        eprintln!("{done:?}");
        for (name, st) in [("prima", before), ("dopo", after)] {
            let page = crate::scene::Page::from_drawing(&Drawing { strokes: st, width: 21000.0, height: 29700.0 }, View::default());
            std::fs::write(format!("{out}-{name}.png"), crate::export::page_to_png(&page, 1200).unwrap()).unwrap();
        }
    }

    /// Le figure nuove (archi d'ellisse, angoli retti, parallele, poligoni
    /// regolari, frecce curve e doppie, tratteggi), prima e dopo:
    /// FORME_PNG=prefisso cargo test -- --ignored figure_nuove
    #[test]
    #[ignore]
    fn figure_nuove() {
        let Ok(out) = std::env::var("FORME_PNG") else { return };
        let g = PageGeom::new(View::default(), 21000.0, 29700.0);
        let hand = |pts: Vec<(f32, f32)>, seed: f32| -> Stroke {
            let mut s = Stroke::new([20, 20, 30], 2.0);
            let n = pts.len();
            for (k, (x, y)) in pts.into_iter().enumerate() {
                let t = k as f32;
                let (dx, dy) = (30.0 * (t * 0.9 + seed).sin() + 18.0 * (t * 0.31 + seed * 2.0).cos(), 30.0 * (t * 0.7 + seed).cos() + 18.0 * (t * 0.23).sin());
                let phase = if k == 0 { PenPhase::Down } else if k + 1 == n { PenPhase::Up } else { PenPhase::Move };
                s.points.push(PenEvent { x: x + dx, y: y + dy, pressure: 0.7, phase });
            }
            s
        };
        let seg = |a: (f32, f32), b: (f32, f32), n: usize| -> Vec<(f32, f32)> { (0..n).map(|k| { let f = k as f32 / (n - 1) as f32; (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f) }).collect() };
        let ell = |cx: f32, cy: f32, a: f32, b: f32, t0: f32, t1: f32, n: usize| -> Vec<(f32, f32)> { (0..=n).map(|k| { let t = t0 + (t1 - t0) * k as f32 / n as f32; (cx + a * t.cos(), cy + b * t.sin()) }).collect() };
        let pi = std::f32::consts::PI;
        let mut st = Vec::new();
        // un cilindro: ellisse sopra (due metà), lati, mezza ellisse sotto
        st.push(hand(ell(4000.0, 3000.0, 2000.0, 600.0, 0.05, pi - 0.03, 40), 1.0));
        st.push(hand(ell(4000.0, 3000.0, 2000.0, 600.0, pi + 0.06, 2.0 * pi - 0.04, 40), 2.0));
        st.push(hand(seg((2000.0, 3050.0), (2030.0, 7000.0), 40), 3.0));
        st.push(hand(seg((6000.0, 3000.0), (5980.0, 7000.0), 40), 4.0));
        st.push(hand(ell(4000.0, 7000.0, 2000.0, 600.0, 0.02, pi - 0.02, 40), 5.0));
        // un angolo retto inclinato (87°) e due parallele inclinate (31° e 34°)
        let r = |deg: f32, len: f32, o: (f32, f32)| { let t = deg.to_radians(); (o.0 + len * t.cos(), o.1 + len * t.sin()) };
        let c = (9000.0, 6500.0);
        st.push(hand([seg(r(-30.0, 3500.0, c), c, 30), seg(c, r(-30.0 + 87.0 + 180.0 + 180.0, 2500.0, c), 25)].concat(), 6.0));
        st.push(hand(seg((13000.0, 3000.0), r(31.0, 5000.0, (13000.0, 3000.0)), 40), 7.0));
        st.push(hand(seg((13000.0, 4800.0), r(34.0, 5000.0, (13000.0, 4800.0)), 40), 8.0));
        // un quadrato storto, un rombo, un esagono, un triangolo quasi equilatero
        st.push(hand([seg((2000.0, 10000.0), (5000.0, 10150.0), 20), seg((5000.0, 10150.0), (4900.0, 13050.0), 20), seg((4900.0, 13050.0), (1950.0, 12950.0), 20), seg((1950.0, 12950.0), (2000.0, 10100.0), 20)].concat(), 9.0));
        let (rc, u, w) = ((9000.0, 11500.0), (2200.0, 500.0), (-450.0, 1700.0));
        st.push(hand([seg((rc.0 + u.0, rc.1 + u.1), (rc.0 + w.0, rc.1 + w.1), 20), seg((rc.0 + w.0, rc.1 + w.1), (rc.0 - u.0, rc.1 - u.1), 20), seg((rc.0 - u.0, rc.1 - u.1), (rc.0 - w.0 + 100.0, rc.1 - w.1), 20), seg((rc.0 - w.0 + 100.0, rc.1 - w.1), (rc.0 + u.0, rc.1 + u.1 + 60.0), 20)].concat(), 10.0));
        let hex: Vec<(f32, f32)> = (0..=6).map(|k| { let t = k as f32 * std::f32::consts::TAU / 6.0 + 0.08; let rr = if k % 2 == 0 { 1700.0 } else { 1600.0 }; (15500.0 + rr * t.cos(), 11500.0 + rr * t.sin()) }).collect();
        st.push(hand(hex.windows(2).flat_map(|w| seg(w[0], w[1], 15)).collect(), 11.0));
        st.push(hand([seg((2000.0, 18000.0), (5200.0, 18150.0), 20), seg((5200.0, 18150.0), (3700.0, 15300.0), 20), seg((3700.0, 15300.0), (2000.0, 17900.0), 20)].concat(), 12.0));
        // una freccia curva e una freccia a due punte in un tratto
        let mut cur = ell(9500.0, 17500.0, 2500.0, 2500.0, pi, 1.6 * pi, 40);
        let tip = cur[cur.len() - 1];
        let b1 = (tip.0 + 500.0 * 170f32.to_radians().cos(), tip.1 + 500.0 * 170f32.to_radians().sin());
        let b2 = (tip.0 + 500.0 * 226f32.to_radians().cos(), tip.1 + 500.0 * 226f32.to_radians().sin());
        cur.extend(seg(tip, b1, 5));
        cur.extend(seg(b1, tip, 4));
        cur.extend(seg(tip, b2, 5));
        st.push(hand(cur, 13.0));
        let (a, b) = ((13000.0, 17000.0), (19000.0, 17300.0));
        st.push(hand([seg((a.0 + 500.0, a.1 - 350.0), a, 5), seg(a, (a.0 + 500.0, a.1 + 350.0), 5), seg((a.0 + 500.0, a.1 + 350.0), a, 5), seg(a, b, 40), seg(b, (b.0 - 500.0, b.1 - 350.0), 5), seg((b.0 - 500.0, b.1 - 350.0), b, 5), seg(b, (b.0 - 500.0, b.1 + 350.0), 5)].concat(), 14.0));
        // una linea tratteggiata un po' storta
        for k in 0..7 {
            let x = 2000.0 + k as f32 * 1300.0 + if k == 3 { 150.0 } else { 0.0 };
            let y = 21500.0 + if k % 2 == 0 { 60.0 } else { -50.0 };
            st.push(hand(seg((x, y), (x + 700.0 + 80.0 * (k % 3) as f32, y + 20.0), 8), 15.0 + k as f32));
        }
        let before = st.clone();
        let (after, done) = apply(&st, &HashMap::new(), &g).unwrap();
        eprintln!("{done:?}");
        for (name, st) in [("prima", before), ("dopo", after)] {
            let page = crate::scene::Page::from_drawing(&Drawing { strokes: st, width: 21000.0, height: 29700.0 }, View::default());
            std::fs::write(format!("{out}-{name}.png"), crate::export::page_to_png(&page, 1200).unwrap()).unwrap();
        }
    }

    #[test]
    fn zona_e_bordo_destro() {
        let g = PageGeom::new(View::default(), 21000.0, 29700.0);
        let strokes = page();
        // un riquadro attorno alla prima riga sola
        let lines = pending(&strokes, &g);
        let (u0, v0) = g.unit(1800.0, 5000.0);
        let (u1, v1) = g.unit(6500.0, 6500.0);
        let sel = select(&strokes, &g, [u0.min(u1), v0.min(v1), u0.max(u1), v0.max(v1)]);
        assert_eq!(sel, (0..6).collect::<Vec<_>>());
        // una riga lunga, in una zona stretta: non va oltre il bordo destro
        let sub: Vec<Stroke> = sel.iter().map(|&i| strokes[i].clone()).collect();
        let texts: HashMap<u64, String> = [(lines[0].1, "una riga molto lunga che non ci starebbe mai in una zona così stretta".to_string())].into();
        let right = u0.max(u1);
        let (out, done) = apply_region(&sub, &texts, &g, right).unwrap();
        assert_eq!(done.lines, 1);
        let max_u = out.iter().flat_map(|s| s.points.iter().map(|p| g.unit(p.x, p.y).0)).fold(f32::MIN, f32::max);
        assert!(max_u <= right + 0.01, "{max_u} > {right}");
    }
}
