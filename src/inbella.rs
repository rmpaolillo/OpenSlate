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

fn bbox(pts: &[(f32, f32)]) -> (f32, f32, f32, f32) {
    pts.iter().fold((f32::MAX, f32::MIN, f32::MAX, f32::MIN), |b, p| (b.0.min(p.0), b.1.max(p.0), b.2.min(p.1), b.3.max(p.1)))
}

/// Le lettere da trasformare di una riga: tutta la scrittura del pennino
/// (anche le graffe e le parentesi alte, che la bella copia rifà), tranne la
/// sottolineatura: una linea lunga e piatta con niente sotto (una linea di
/// frazione ha sotto il denominatore, e va rifatta).
fn letters(line: &[usize], strokes: &[Stroke], ph: &[Vec<(f32, f32)>], hh: f32) -> Vec<usize> {
    let ink: Vec<usize> = line.iter().copied().filter(|&i| strokes.get(i).is_some_and(writable) && !ph[i].is_empty()).collect();
    let boxes: Vec<(f32, f32, f32, f32)> = ink.iter().map(|&i| bbox(&ph[i])).collect();
    (0..ink.len())
        .filter(|&a| {
            let (x0, x1, y0, y1) = boxes[a];
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

fn lines_of(r: &Reading, strokes: &[Stroke], g: &PageGeom) -> Vec<(String, u64)> {
    let ph = physical(strokes, g);
    r.lines
        .iter()
        .filter(|l| !letters(&l.group.strokes, strokes, &ph, r.char_h).is_empty())
        .map(|l| (l.group.id.clone(), line_key(strokes, &l.group.strokes)))
        .collect()
}

/// Fa leggere all'AI le righe della pagina (bloccante: in un thread).
/// Torna il testo di ogni riga letta, per impronta (vuoto = da lasciare).
pub fn recognize(strokes: &[Stroke], width: f32, height: f32, view: View) -> Result<HashMap<u64, String>> {
    let d = Drawing { strokes: strokes.to_vec(), width, height };
    let (pw, ph) = crate::scene::page_dims(&d);
    let g = PageGeom::new(view, pw, ph);
    let reading = crate::boardread::read(strokes, &g, None);
    let lines = lines_of(&reading, strokes, &g);
    if lines.is_empty() {
        return Ok(HashMap::new());
    }
    let page = crate::scene::Page::from_drawing(&d, view);
    let png = crate::boardread::labelled_png(&page, &reading)?;
    let ids: Vec<&str> = lines.iter().map(|(id, _)| id.as_str()).collect();
    let prompt = PROMPT.replace("{RIGHE}", &ids.join(", "));
    let reply = crate::ai::read_image(std::sync::Arc::new(png), &prompt)?;
    let map = parse_reply(&reply.text)?;
    Ok(lines.iter().filter_map(|(id, k)| map.get(id).map(|v| (*k, v.clone().unwrap_or_default()))).collect())
}

const PROMPT: &str = "Questa è una pagina scritta a mano col pennino. Accanto a ogni riga c'è il suo nome in rosa (r1, r2…) e ogni parola ha un riquadro.\n\
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

/// La pagina in bella: le righe lette (per impronta) tolte e riscritte al
/// loro posto. `None` se non cambia niente; altrimenti i tratti nuovi e
/// quante righe sono state trasformate. Le righe cambiate nel frattempo
/// (impronta diversa) restano come sono.
pub fn apply(strokes: &[Stroke], texts: &HashMap<u64, String>, g: &PageGeom) -> Option<(Vec<Stroke>, usize)> {
    if strokes.is_empty() || texts.is_empty() {
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
        let room = (0.985 * aspect - x0).max(0.05);
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
    if added.is_empty() {
        return None;
    }
    mark_neat(&added);
    let mut out: Vec<Stroke> = strokes.iter().zip(hidden).filter(|(_, h)| !h).map(|(s, _)| s.clone()).collect();
    out.extend(added);
    Some((out, n))
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
fn turns(pts: &[(f32, f32)], min: f32) -> (Vec<f32>, Vec<f32>) {
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
        assert_eq!(n, 1);
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
        assert!(out.iter().any(|o| same(o, &strokes[7])), "la sottolineatura sparisce");
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
}
