//! La chat AI sul foglio della lavagna: lo strumento `lavagna_inserisci_grafico`
//! mette un grafico fatto da `draw_chart` sul foglio, anche al posto di un
//! disegno fatto a mano (che si trova da solo: le righe lunghe, come gli assi,
//! e le scritte lì attorno). Il grafico diventa l'immagine sotto i tratti
//! (come la pagina di un documento), i tratti a mano che sostituisce si
//! tolgono; «Annulla» nella chat rimette tutto com'era.

use crate::scene::{Hub, PageBg};
use crate::stroke::Stroke;
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex, OnceLock};

static HUB: OnceLock<Arc<Hub>> = OnceLock::new();

/// La lavagna dell'app (all'avvio).
pub fn set_hub(hub: Arc<Hub>) {
    let _ = HUB.set(hub);
}

/// Copie del foglio prima delle modifiche della chat: (numero, tratti, sfondo).
static UNDO: Mutex<Vec<(u64, Vec<Stroke>, Option<PageBg>)>> = Mutex::new(Vec::new());

pub fn definitions() -> Vec<Value> {
    vec![json!({
        "type": "function",
        "function": {
            "name": "lavagna_inserisci_grafico",
            "description": "Mette sul foglio della lavagna (la pagina scritta a mano accanto alla chat) un grafico fatto con draw_chart. \
                al_posto_del_disegno=true (default) lo mette al posto del disegno/grafico fatto a mano sul foglio, che si trova da solo \
                (gli assi e le scritte attorno) e si toglie; con `riquadro` [x1, y1, x2, y2] (0–1000 sull'immagine della pagina) si sceglie \
                la zona; se sul foglio non c'è un disegno va sotto lo scritto. L'utente lo vede subito e può annullare dalla chat.",
            "parameters": {
                "type": "object",
                "properties": {
                    "grafico": { "type": "string", "description": "L'id del grafico (da draw_chart)" },
                    "al_posto_del_disegno": { "type": "boolean", "description": "Sostituisce il disegno a mano (default true)" },
                    "riquadro": { "type": "array", "items": { "type": "number" }, "description": "Zona [x1, y1, x2, y2], 0–1000 sull'immagine della pagina (facoltativa)" }
                },
                "required": ["grafico"]
            }
        }
    })]
}

pub fn system_note() -> String {
    "\n\nAccanto alla chat c'è il foglio della lavagna (scritto a mano col pennino): per metterci un grafico, anche al posto di un \
     disegno fatto a mano, prima draw_chart e poi lavagna_inserisci_grafico con l'id del grafico. Non dire che non puoi: \
     lo strumento c'è. L'utente vede subito il risultato e lo può annullare."
        .into()
}

/// Riquadro (frazioni 0..1 della pagina: x0, y0, x1, y1) di un tratto.
fn bbox(pts: &[(f32, f32)]) -> Option<[f32; 4]> {
    let mut b = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
    for &(x, y) in pts {
        b = [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)];
    }
    (b[0] <= b[2]).then_some(b)
}

fn union(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])]
}

/// La zona del disegno a mano: i tratti lunghi (assi, linee) che si toccano,
/// il gruppo più grande, più le scritte corte attorno (etichette, tacche).
/// Ritorna la zona e gli indici dei tratti che ne fanno parte.
pub fn find_drawing(boxes: &[Option<[f32; 4]>]) -> Option<([f32; 4], Vec<usize>)> {
    const LONG: f32 = 0.12;
    let long: Vec<usize> = (0..boxes.len()).filter(|&i| boxes[i].is_some_and(|b| b[2] - b[0] > LONG || b[3] - b[1] > LONG)).collect();
    if long.is_empty() {
        return None;
    }
    // gruppi di tratti lunghi che si toccano (con un po' di margine)
    let near = |a: [f32; 4], b: [f32; 4]| a[0] - 0.03 <= b[2] && b[0] - 0.03 <= a[2] && a[1] - 0.03 <= b[3] && b[1] - 0.03 <= a[3];
    let mut groups: Vec<([f32; 4], Vec<usize>)> = Vec::new();
    for &i in &long {
        let b = boxes[i].unwrap();
        let hits: Vec<usize> = (0..groups.len()).filter(|&g| near(groups[g].0, b)).collect();
        let mut g = (b, vec![i]);
        for &h in hits.iter().rev() {
            let (gb, gi) = groups.remove(h);
            g.0 = union(g.0, gb);
            g.1.extend(gi);
        }
        groups.push(g);
    }
    let (mut zone, mut members) = groups.into_iter().max_by(|a, b| {
        let area = |r: [f32; 4]| (r[2] - r[0]) * (r[3] - r[1]);
        area(a.0).total_cmp(&area(b.0))
    })?;
    // le scritte corte dentro o appena fuori (etichette degli assi, dei
    // punti): a parole intere (tratti vicini fra loro), e solo le parole che
    // stanno tutte nella zona; una parola che esce («Giusto?» accanto al
    // disegno) resta com'è, tutta
    let pad = 0.06;
    let outer = [zone[0] - pad, zone[1] - pad, zone[2] + pad, zone[3] + pad];
    let short: Vec<usize> =
        (0..boxes.len()).filter(|&i| !members.contains(&i) && boxes[i].is_some_and(|b| b[2] - b[0] <= LONG && b[3] - b[1] <= LONG)).collect();
    let close = |a: [f32; 4], b: [f32; 4]| a[0] - 0.012 <= b[2] && b[0] - 0.012 <= a[2] && a[1] - 0.008 <= b[3] && b[1] - 0.008 <= a[3];
    let mut words: Vec<([f32; 4], Vec<usize>)> = Vec::new();
    for &i in &short {
        let b = boxes[i].unwrap();
        let hits: Vec<usize> = (0..words.len()).filter(|&w| close(words[w].0, b)).collect();
        let mut w = (b, vec![i]);
        for &h in hits.iter().rev() {
            let (wb, wi) = words.remove(h);
            w.0 = union(w.0, wb);
            w.1.extend(wi);
        }
        words.push(w);
    }
    for (wb, wi) in words {
        if wb[0] >= outer[0] && wb[2] <= outer[2] && wb[1] >= outer[1] && wb[3] <= outer[3] {
            members.extend(wi);
            zone = union(zone, wb);
        }
    }
    Some((zone, members))
}

/// Il grafico (proporzioni `aspect` = larghezza/altezza in unità della
/// pagina) dentro la zona, centrato: u, v, larghezza, altezza.
fn fit(zone: [f32; 4], aspect: f32) -> [f32; 4] {
    let (zw, zh) = (zone[2] - zone[0], zone[3] - zone[1]);
    let (w, h) = if zw / zh > aspect { (zh * aspect, zh) } else { (zw, zw / aspect) };
    [zone[0] + (zw - w) / 2.0, zone[1] + (zh - h) / 2.0, w, h]
}

/// Esegue lo strumento della lavagna; None se `name` non è suo.
pub fn execute(name: &str, args: &Value) -> Option<crate::docagent::Outcome> {
    if name != "lavagna_inserisci_grafico" {
        return None;
    }
    Some(match insert(args) {
        Ok((text, n)) => crate::docagent::Outcome { text, short: "grafico sul foglio".into(), edit: Some(n), ..Default::default() },
        Err(e) => crate::docagent::Outcome { text: format!("ERRORE: {e:#}"), short: format!("errore: {e:#}"), ..Default::default() },
    })
}

fn insert(args: &Value) -> Result<(String, u64)> {
    let hub = HUB.get().context("lavagna non disponibile")?;
    let id = args["grafico"].as_str().or(args["image"].as_str()).context("manca «grafico» (l'id di draw_chart)")?;
    let chart = crate::charts::find(id).with_context(|| format!("non trovo il grafico «{id}»: fallo prima con draw_chart"))?;
    // il bianco del grafico diventa trasparente: sotto si vedono righe e quadretti
    let png = Arc::new(white_to_alpha(&std::fs::read(&chart.png)?)?);
    let replace = args["al_posto_del_disegno"].as_bool().unwrap_or(true);
    let given: Option<[f32; 4]> = args["riquadro"].as_array().and_then(|a| {
        let n: Vec<f32> = a.iter().filter_map(|v| v.as_f64().map(|x| x as f32)).collect();
        (n.len() == 4).then(|| {
            let s = if n.iter().all(|&v| v <= 1.0) { 1.0 } else { 1000.0 };
            [n[0] / s, n[1] / s, n[2] / s, n[3] / s].map(|v| v.clamp(0.0, 1.0))
        })
    });
    let s = hub.lock();
    if s.bg.as_ref().is_some_and(|b| !b.is_chart()) {
        bail!("sul foglio c'è la pagina di un documento: usa document_insert_image");
    }
    let g = s.geom();
    let boxes: Vec<Option<[f32; 4]>> =
        s.drawing.strokes.iter().map(|st| bbox(&st.points.iter().map(|p| g.unit(p.x, p.y)).collect::<Vec<_>>())).collect();
    let aspect = (chart.width / chart.height) * g.h / g.w;
    let (zone, remove): ([f32; 4], Vec<usize>) = match (given, replace) {
        (Some(r), true) => {
            let r = [r[0].min(r[2]), r[1].min(r[3]), r[0].max(r[2]), r[1].max(r[3])];
            let inside: Vec<usize> = boxes
                .iter()
                .enumerate()
                .filter_map(|(i, b)| {
                    let b = (*b)?;
                    let (cx, cy) = ((b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0);
                    (cx >= r[0] && cx <= r[2] && cy >= r[1] && cy <= r[3]).then_some(i)
                })
                .collect();
            (r, inside)
        }
        (Some(r), false) => ([r[0].min(r[2]), r[1].min(r[3]), r[0].max(r[2]), r[1].max(r[3])], Vec::new()),
        (None, true) if find_drawing(&boxes).is_some() => {
            let (z, m) = find_drawing(&boxes).unwrap();
            // un po' più grande del disegno a mano: il grafico ha numeri e legenda
            let pad = 0.03;
            ([(z[0] - pad).max(0.02), (z[1] - pad).max(0.02), (z[2] + pad).min(0.98), (z[3] + pad).min(0.98)], m)
        }
        _ => {
            // sotto lo scritto, largo metà foglio
            let bottom = boxes.iter().flatten().map(|b| b[3]).fold(0.05f32, f32::max);
            let h = (0.5 / aspect).min(0.95 - bottom - 0.02).max(0.15);
            ([0.25, (bottom + 0.02).min(0.8), 0.75, (bottom + 0.02 + h).min(0.98)], Vec::new())
        }
    };
    if zone[2] - zone[0] < 0.05 || zone[3] - zone[1] < 0.05 {
        bail!("zona troppo piccola per il grafico");
    }
    let rect = fit(zone, aspect);
    // copia per «Annulla»
    let n = {
        let mut u = UNDO.lock().unwrap();
        let n = u.last().map(|x| x.0 + 1).unwrap_or(1);
        u.push((n, s.drawing.strokes.clone(), s.bg.clone()));
        n
    };
    let kept: Vec<Stroke> = s.drawing.strokes.iter().enumerate().filter(|(i, _)| !remove.contains(i)).map(|(_, st)| st.clone()).collect();
    // un grafico già messo prima resta: i due si fondono in un'immagine sola
    let bg = match s.bg.clone() {
        Some(old) => merge(&old, &png, rect, g.w / g.h)?,
        None => PageBg { png, rect, key: format!("chart:{}", chart.id) },
    };
    drop(s);
    hub.edit(|s| s.set_page(kept, Some(bg)));
    crate::docagent::board_changed();
    Ok((
        format!(
            "Fatto: il grafico {} è sul foglio della lavagna{}.",
            chart.id,
            if remove.is_empty() { String::new() } else { format!(", al posto del disegno a mano ({} tratti tolti)", remove.len()) }
        ),
        n,
    ))
}

/// Il bianco diventa trasparente («colore → alfa»): sul foglio bianco
/// l'immagine resta identica, sulla carta a righe o a quadretti le righe si
/// vedono sotto, come se il grafico fosse stampato sul foglio.
pub fn white_to_alpha(png: &[u8]) -> Result<Vec<u8>> {
    let mut img = tiny_skia::Pixmap::decode_png(png).map_err(|e| anyhow::anyhow!("PNG: {e}"))?;
    for px in img.pixels_mut() {
        let c = px.demultiply();
        let a0 = c.alpha() as f32 / 255.0;
        let [r, g, b] = [c.red(), c.green(), c.blue()].map(|v| v as f32 / 255.0);
        // quanto il colore si allontana dal bianco
        let a = (1.0 - r).max(1.0 - g).max(1.0 - b);
        let (nr, ng, nb, na) = if a <= 1e-4 {
            (0.0, 0.0, 0.0, 0.0)
        } else {
            let f = |v: f32| (1.0 - (1.0 - v) / a).clamp(0.0, 1.0);
            (f(r), f(g), f(b), a * a0)
        };
        let q = |v: f32| (v * 255.0).round() as u8;
        *px = tiny_skia::ColorU8::from_rgba(q(nr), q(ng), q(nb), q(na)).premultiply();
    }
    img.encode_png().map_err(|e| anyhow::anyhow!("PNG: {e}"))
}

/// Due immagini sul foglio in una sola, grande quanto la pagina.
fn merge(old: &PageBg, new_png: &[u8], rect: [f32; 4], aspect: f32) -> Result<PageBg> {
    use tiny_skia::{Pixmap, PixmapPaint, Transform};
    let w = 2400u32;
    let h = ((w as f32) / aspect).round().max(1.0) as u32;
    let mut canvas = Pixmap::new(w, h).context("immagine vuota")?;
    for (png, r) in [(old.png.as_slice(), old.rect), (new_png, rect)] {
        let img = Pixmap::decode_png(png).map_err(|e| anyhow::anyhow!("PNG: {e}"))?;
        let t = Transform::from_row(
            r[2] * w as f32 / img.width() as f32,
            0.0,
            0.0,
            r[3] * h as f32 / img.height() as f32,
            r[0] * w as f32,
            r[1] * h as f32,
        );
        canvas.draw_pixmap(0, 0, img.as_ref(), &PixmapPaint { quality: tiny_skia::FilterQuality::Bicubic, ..Default::default() }, t, None);
    }
    let png = canvas.encode_png().map_err(|e| anyhow::anyhow!("PNG: {e}"))?;
    Ok(PageBg { png: Arc::new(png), rect: [0.0, 0.0, 1.0, 1.0], key: format!("chart:{}", chrono::Local::now().timestamp_millis()) })
}

/// «Annulla» della chat sul foglio: com'era prima della modifica `n`.
pub fn undo(n: u64) -> Result<()> {
    let hub = HUB.get().context("lavagna non disponibile")?;
    let mut u = UNDO.lock().unwrap();
    let i = u.iter().position(|x| x.0 == n).context("questa modifica non si può più annullare")?;
    let (_, strokes, bg) = u[i].clone();
    u.truncate(i);
    drop(u);
    hub.edit(|s| s.set_page(strokes, bg));
    crate::docagent::board_changed();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trova_il_disegno_a_mano() {
        // testo in alto (tratti corti), assi lunghi al centro, etichette vicine, testo in basso
        let b = |x0: f32, y0: f32, x1: f32, y1: f32| Some([x0, y0, x1, y1]);
        let boxes = vec![
            b(0.10, 0.10, 0.14, 0.13), // «R» del testo
            b(0.15, 0.10, 0.19, 0.13),
            b(0.30, 0.40, 0.70, 0.43), // asse x
            b(0.50, 0.25, 0.52, 0.60), // asse y
            b(0.34, 0.28, 0.40, 0.31), // «P(-1,2)»
            b(0.42, 0.45, 0.45, 0.47), // «-1»
            b(0.10, 0.80, 0.16, 0.84), // «Giusto?»
            // «Giu» fuori e «sto?» appena sotto l'asse: la parola resta intera
            b(0.20, 0.63, 0.235, 0.66),
            b(0.24, 0.63, 0.27, 0.66),
            b(0.275, 0.63, 0.31, 0.66),
        ];
        let (zone, m) = find_drawing(&boxes).unwrap();
        let mut m = m;
        m.sort();
        assert_eq!(m, vec![2, 3, 4, 5]);
        assert!(zone[0] <= 0.30 && zone[2] >= 0.70 && zone[1] <= 0.25 && zone[3] >= 0.60);
        assert!(find_drawing(&[b(0.1, 0.1, 0.12, 0.12)]).is_none());
        // bianco → trasparente, il nero resta nero e pieno
        let mut pm = tiny_skia::Pixmap::new(2, 1).unwrap();
        pm.pixels_mut()[0] = tiny_skia::ColorU8::from_rgba(255, 255, 255, 255).premultiply();
        pm.pixels_mut()[1] = tiny_skia::ColorU8::from_rgba(0, 0, 0, 255).premultiply();
        let out = tiny_skia::Pixmap::decode_png(&white_to_alpha(&pm.encode_png().unwrap()).unwrap()).unwrap();
        assert_eq!(out.pixels()[0].alpha(), 0);
        assert_eq!(out.pixels()[1].demultiply(), tiny_skia::ColorU8::from_rgba(0, 0, 0, 255));
        let r = fit([0.2, 0.2, 0.8, 0.5], 2.0);
        assert!((r[2] - 0.6).abs() < 1e-6 && (r[3] - 0.3).abs() < 1e-6);
    }

    /// Il grafico al posto degli assi disegnati a mano, poi Annulla. Cambia
    /// la cartella corrente: cargo test -- --ignored --test-threads=1 grafico_sulla_lavagna
    #[test]
    #[ignore]
    fn grafico_sulla_lavagna() {
        use crate::scene::Scene;
        use crate::stroke::{PenEvent, PenPhase};
        let work = std::env::temp_dir().join(format!("openslate-board-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&work);
        std::fs::create_dir_all(&work).unwrap();
        std::env::set_current_dir(&work).unwrap();
        let hub = crate::scene::Hub::new(Scene::new(&crate::settings::Settings::default()));
        set_hub(hub.clone());
        let g = hub.lock().geom();
        let line = |a: (f32, f32), b: (f32, f32)| {
            let mut st = Stroke::new([0, 0, 0], 2.0);
            for k in 0..=10 {
                let t = k as f32 / 10.0;
                let (x, y) = g.pad(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
                st.points.push(PenEvent { x, y, pressure: 0.5, phase: PenPhase::Move });
            }
            st
        };
        let strokes = vec![
            line((0.10, 0.10), (0.14, 0.12)),  // testo
            line((0.30, 0.45), (0.72, 0.43)),  // asse x
            line((0.52, 0.28), (0.53, 0.65)),  // asse y
            line((0.36, 0.33), (0.41, 0.34)),  // «P(-1,2)»
            line((0.10, 0.85), (0.16, 0.86)),  // «Giusto?»
        ];
        hub.edit(|s| s.set_page(strokes.clone(), None));
        let spec = json!({ "x": { "min": -4, "max": 4 }, "series": [{ "type": "function", "expr": "1 - x" }, { "type": "points", "points": [[-1, 2]], "names": ["$P(-1, 2)$"] }] });
        let id = crate::charts::draw(&spec).unwrap().id;
        let o = execute("lavagna_inserisci_grafico", &json!({ "grafico": id })).unwrap();
        assert!(o.text.starts_with("Fatto") && o.text.contains("3 tratti tolti"), "{}", o.text);
        {
            let s = hub.lock();
            assert_eq!(s.drawing.strokes.len(), 2);
            let bg = s.bg.as_ref().unwrap();
            assert!(bg.rect[0] < 0.35 && bg.rect[1] < 0.33 && bg.rect[0] + bg.rect[2] > 0.6, "{:?}", bg.rect);
        }
        // un secondo grafico sotto lo scritto: le immagini si fondono
        let o2 = execute("lavagna_inserisci_grafico", &json!({ "grafico": id, "al_posto_del_disegno": false })).unwrap();
        assert!(o2.text.starts_with("Fatto"), "{}", o2.text);
        assert_eq!(hub.lock().bg.as_ref().unwrap().rect, [0.0, 0.0, 1.0, 1.0]);
        undo(o.edit.unwrap()).unwrap();
        let s = hub.lock();
        assert_eq!(s.drawing.strokes.len(), 5);
        assert!(s.bg.is_none());
    }
}
