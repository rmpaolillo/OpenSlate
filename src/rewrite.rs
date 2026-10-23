//! La riscrittura in bella: le righe scritte col pennino, lette dall'AI
//! (formule comprese, in LaTeX), si vedono riscritte con una scrittura a
//! mano chiara (il carattere «naturale») e la matematica impaginata come sul
//! quaderno ordinato (`mathhand`).
//!
//! Come la bella scrittura, cambia solo come la pagina si vede e si esporta:
//! i tratti veri restano (l'AI legge quelli; tenendo premuto O si vedono).
//! Quello che l'AI ha letto si ricorda per riga, con l'impronta dei suoi
//! tratti (file `rewrite_cache.json` nella cartella dei dati): una riga che
//! cambia (ci scrivi ancora) si rilegge.

use crate::boardread::Reading;
use crate::scene::{PageGeom, View};
use crate::stroke::{Drawing, PenEvent, PenPhase, Stroke};
use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

const FILE: &str = "rewrite_cache.json";

/// Riga letta: il testo (vuoto = illeggibile o un disegno: resta com'è).
static CACHE: Mutex<Option<HashMap<u64, String>>> = Mutex::new(None);
static SHOW: AtomicBool = AtomicBool::new(false);

/// Si vede la versione in bella (`rewrite_show`)?
pub fn showing() -> bool {
    SHOW.load(Ordering::Relaxed)
}

pub fn set_showing(on: bool) {
    SHOW.store(on, Ordering::Relaxed);
}

fn with_cache<R>(f: impl FnOnce(&mut HashMap<u64, String>) -> R) -> R {
    let mut g = CACHE.lock().unwrap();
    let m = g.get_or_insert_with(|| {
        if cfg!(test) {
            return HashMap::new();
        }
        std::fs::read_to_string(FILE)
            .ok()
            .and_then(|t| serde_json::from_str::<HashMap<String, String>>(&t).ok())
            .map(|m| m.into_iter().filter_map(|(k, v)| u64::from_str_radix(&k, 16).ok().map(|k| (k, v))).collect())
            .unwrap_or_default()
    });
    f(m)
}

fn save(m: &HashMap<u64, String>) {
    if cfg!(test) {
        return;
    }
    let out: HashMap<String, &String> = m.iter().map(|(k, v)| (format!("{k:x}"), v)).collect();
    if let Ok(t) = serde_json::to_string(&out) {
        if let Err(e) = crate::web::write_atomic(std::path::Path::new(FILE), t.as_bytes()) {
            tracing::warn!("{FILE}: {e}");
        }
    }
}

/// Le righe lette, da ricordare.
pub fn store(lines: &[(u64, String)]) {
    with_cache(|m| {
        for (k, v) in lines {
            m.insert(*k, v.clone());
        }
        // le più vecchie non servono più: si tiene un numero ragionevole
        if m.len() > 20_000 {
            let drop: Vec<u64> = m.keys().take(m.len() - 20_000).copied().collect();
            for k in drop {
                m.remove(&k);
            }
        }
        save(m);
    });
}

fn lookup(k: u64) -> Option<String> {
    with_cache(|m| m.get(&k).cloned())
}

/// L'impronta di una riga: i suoi tratti, con le coordinate arrotondate
/// (l'archivio le salva con tre decimali).
pub fn line_key(strokes: &[Stroke], idx: &[usize]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut mix = |v: u64| h = (h ^ v).wrapping_mul(0x100_0000_01b3);
    let mut idx = idx.to_vec();
    idx.sort_unstable();
    for &i in &idx {
        let Some(s) = strokes.get(i) else { continue };
        mix(s.points.len() as u64);
        for p in &s.points {
            mix((p.x.round() as i64 as u64) << 32 ^ (p.y.round() as i64 as u64 & 0xffff_ffff));
        }
    }
    h
}

/// Le righe da leggere (quelle non ancora lette, o tutte con `all`), coi
/// nomi di `boardread` e le loro impronte.
pub fn pending(strokes: &[Stroke], g: &PageGeom, all: bool) -> (Reading, Vec<(String, u64)>) {
    let r = crate::boardread::read(strokes, g, None);
    let lines = r
        .lines
        .iter()
        .map(|l| (l.group.id.clone(), line_key(strokes, &l.group.strokes)))
        .filter(|(_, k)| all || lookup(*k).is_none())
        .collect();
    (r, lines)
}

/// Fa leggere all'AI le righe indicate (bloccante: in un thread).
pub fn recognize(strokes: &[Stroke], width: f32, height: f32, view: View, lines: &[(String, u64)]) -> Result<Vec<(u64, String)>> {
    if lines.is_empty() {
        return Ok(Vec::new());
    }
    let d = Drawing { strokes: strokes.to_vec(), width, height };
    let (pw, ph) = crate::scene::page_dims(&d);
    let g = PageGeom::new(view, pw, ph);
    let reading = crate::boardread::read(strokes, &g, None);
    let page = crate::scene::Page::from_drawing(&d, view);
    let png = crate::boardread::labelled_png(&page, &reading)?;
    let ids: Vec<&str> = lines.iter().map(|(id, _)| id.as_str()).collect();
    let prompt = PROMPT.replace("{RIGHE}", &ids.join(", "));
    let reply = crate::ai::read_image(std::sync::Arc::new(png), &prompt)?;
    let map = parse_reply(&reply.text)?;
    Ok(lines
        .iter()
        .filter_map(|(id, k)| {
            // una riga che l'AI non nomina: si riproverà
            let v = map.get(id)?;
            Some((*k, v.clone().unwrap_or_default()))
        })
        .collect())
}

const PROMPT: &str = "Questa è una pagina scritta a mano col pennino. Accanto a ogni riga c'è il suo nome in rosa (r1, r2…) e ogni parola ha un riquadro.\n\
Trascrivi esattamente il contenuto di queste righe: {RIGHE}.\n\
Regole:\n\
- copia fedelmente, errori compresi: non correggere, non completare, non aggiungere niente;\n\
- tutta la matematica in LaTeX fra $…$: \\frac{a}{b} per le frazioni scritte in colonna, ^ e _ per esponenti e pedici, \\sqrt{} per le radici, \\begin{cases}…\\end{cases} per i sistemi, \\cdot, \\le, \\ge, \\neq, \\Rightarrow, le lettere greche (\\alpha…), \\{ \\} per le graffe;\n\
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

/// Il foglio da mostrare: le righe già lette tolte e riscritte in bella.
/// `orig` sono i tratti veri (per riconoscere le righe), `shown` quelli che
/// si vedono (stesso numero e ordine: bella scrittura, forme perfette).
pub fn replace(orig: &[Stroke], shown: Vec<Stroke>, g: &PageGeom) -> Vec<Stroke> {
    if orig.is_empty() || shown.len() != orig.len() {
        return shown;
    }
    let reading = crate::boardread::read(orig, g, None);
    let aspect = g.w / g.h;
    let hh = reading.char_h;
    let ph: Vec<Vec<(f32, f32)>> = orig
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
    let mut hidden = vec![false; orig.len()];
    let mut added: Vec<Stroke> = Vec::new();
    for line in &reading.lines {
        let k = line_key(orig, &line.group.strokes);
        let Some(text) = lookup(k).filter(|t| !t.trim().is_empty()) else { continue };
        let idx: Vec<usize> = line.group.strokes.iter().copied().filter(|&i| !orig[i].highlight).collect();
        if idx.is_empty() {
            continue;
        }
        // dove: a sinistra della riga, sulla sua riga di base, alto come la sua scrittura
        let x0 = idx.iter().flat_map(|&i| ph[i].iter().map(|p| p.0)).fold(f32::MAX, f32::min);
        let (base, xh) = crate::beautify::base_and_xheight(&idx, &ph, hh).unwrap_or_else(|| {
            let b = idx.iter().flat_map(|&i| ph[i].iter().map(|p| p.1)).fold(f32::MIN, f32::max);
            (b, 0.6 * hh)
        });
        let b = crate::mathhand::line_box(crate::handwrite::Style::Naturale, &text);
        if b.strokes.is_empty() {
            continue;
        }
        // unità di Hershey → unità della pagina: il corpo delle minuscole (14) come il tuo
        let mut s = (1.15 * xh / 14.0).clamp(0.7 * hh / 21.0, 1.15 * hh / 21.0);
        let room = (0.985 * aspect - x0).max(0.05);
        if b.w * s > room {
            s = room / b.w;
        }
        // colore e spessore: quelli più usati nella riga
        let mut colors: Vec<([u8; 3], usize)> = Vec::new();
        for &i in &idx {
            match colors.iter_mut().find(|(c, _)| *c == orig[i].color) {
                Some((_, n)) => *n += orig[i].points.len(),
                None => colors.push((orig[i].color, orig[i].points.len())),
            }
        }
        let color = colors.iter().max_by_key(|(_, n)| *n).map(|(c, _)| *c).unwrap_or([20, 20, 20]);
        let mut widths: Vec<f32> = idx.iter().map(|&i| orig[i].width).collect();
        widths.sort_by(f32::total_cmp);
        let width = widths[widths.len() / 2];
        for st in b.strokes {
            if st.len() < 2 {
                continue;
            }
            let mut stroke = Stroke::new(color, width);
            let n = st.len();
            for (j, (x, y)) in st.into_iter().enumerate() {
                let (px, py) = g.pad((x0 + x * s) / aspect, base + y * s);
                let phase = if j == 0 { PenPhase::Down } else if j + 1 == n { PenPhase::Up } else { PenPhase::Move };
                stroke.points.push(PenEvent { x: px, y: py, pressure: 0.6, phase });
            }
            added.push(stroke);
        }
        // si tolgono le lettere, non i segni (la sottolineatura di un titolo resta)
        for &i in &line.group.strokes {
            if orig[i].highlight {
                continue;
            }
            let (x0, x1, y0, y1) = ph[i].iter().fold((f32::MAX, f32::MIN, f32::MAX, f32::MIN), |b, p| (b.0.min(p.0), b.1.max(p.0), b.2.min(p.1), b.3.max(p.1)));
            let (w, h) = (x1 - x0, y1 - y0);
            let mark = (w > 3.0 * hh && h < 0.35 * hh) || h > 2.4 * hh;
            if !mark {
                hidden[i] = true;
            }
        }
    }
    if added.is_empty() {
        return shown;
    }
    let mut out: Vec<Stroke> = shown.into_iter().zip(hidden).filter(|(_, h)| !h).map(|(s, _)| s).collect();
    out.extend(added);
    out
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

    #[test]
    fn riga_riscritta() {
        // una riga di «lettere» (archetti), poi letta come una formula
        let g = PageGeom::new(View::default(), 21000.0, 29700.0);
        let mut strokes = Vec::new();
        for k in 0..6 {
            let mut s = Stroke::new([30, 60, 160], 2.0);
            for t in 0..20 {
                let f = t as f32 / 19.0;
                s.points.push(PenEvent { x: 2000.0 + k as f32 * 700.0 + f * 500.0, y: 6000.0 - (f * 3.14).sin() * 500.0, pressure: 0.7, phase: PenPhase::Move });
            }
            strokes.push(s);
        }
        let (r, lines) = pending(&strokes, &g, false);
        assert_eq!(r.lines.len(), 1);
        assert_eq!(lines.len(), 1);
        store(&[(lines[0].1, "$\\frac{x+1}{2} = 3$".into())]);
        let out = replace(&strokes, strokes.clone(), &g);
        // i tratti veri spariti, al loro posto la formula, dello stesso colore
        assert!(out.iter().all(|s| s.color == [30, 60, 160]));
        assert!(!out.iter().any(|s| strokes.iter().any(|o| o.points.len() == s.points.len() && o.points[0].x == s.points[0].x)));
        assert!(out.len() > 5);
        // sulla riga: a sinistra dov'era, e non lontano in altezza
        let xs: Vec<f32> = out.iter().flat_map(|s| s.points.iter().map(|p| p.x)).collect();
        let ys: Vec<f32> = out.iter().flat_map(|s| s.points.iter().map(|p| p.y)).collect();
        let x0 = xs.iter().copied().fold(f32::MAX, f32::min);
        assert!((x0 - 2000.0).abs() < 600.0, "{x0}");
        let (y0, y1) = (ys.iter().copied().fold(f32::MAX, f32::min), ys.iter().copied().fold(f32::MIN, f32::max));
        assert!(y0 > 4000.0 && y1 < 7500.0, "{y0} {y1}");
        // la pagina che si vede ha ancora tutte le righe che non sono state lette
        let other = line_key(&strokes, &[0]);
        assert_ne!(other, lines[0].1);
    }
}
