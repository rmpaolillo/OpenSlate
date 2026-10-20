//! Figure dei fogli trascritti: il modello che legge la pagina segna disegni,
//! grafici e schemi come `[figura: descrizione | riquadro: x1,y1,x2,y2]`
//! (coordinate 0–1000 sull'immagine che ha visto). Qui la figura si
//! «ridisegna» con i tratti veri della penna presi in quel riquadro: un SVG
//! vettoriale accanto all'appunto, che resta nitido a ogni ingrandimento.

use crate::scene::{Page, PageStroke};
use std::path::Path;

/// Un segnaposto `[figura: …]` trovato nel testo.
#[derive(Debug, PartialEq)]
pub struct Placeholder {
    /// Posizione nel testo (byte iniziale e finale, `]` compresa).
    pub start: usize,
    pub end: usize,
    pub desc: String,
    /// Riquadro in frazioni 0..1 dell'immagine: x1, y1, x2, y2.
    pub rect: Option<[f32; 4]>,
}

/// Trova i segnaposto delle figure.
pub fn find(text: &str) -> Vec<Placeholder> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = text[from..].find("[figura:").map(|i| i + from) {
        // la parentesi che chiude (le descrizioni possono avere parentesi tonde)
        let Some(j) = text[i..].find(']').map(|j| i + j) else { break };
        let inner = &text[i + "[figura:".len()..j];
        let (desc, rect) = match inner.rsplit_once('|') {
            Some((d, r)) => (d, parse_rect(r)),
            None => (inner, None),
        };
        out.push(Placeholder { start: i, end: j + 1, desc: desc.trim().to_string(), rect });
        from = j + 1;
    }
    out
}

/// `riquadro: 120,400,610,780` → frazioni 0..1 (accetta anche spazi e 0..1).
fn parse_rect(s: &str) -> Option<[f32; 4]> {
    let s = s.trim();
    let s = s.split_once(':').map(|(_, b)| b).unwrap_or(s);
    let n: Vec<f32> = s
        .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .filter(|t| !t.is_empty())
        .filter_map(|t| t.trim().parse::<f32>().ok())
        .collect();
    if n.len() != 4 {
        return None;
    }
    let scale = if n.iter().all(|&v| v <= 1.0) { 1.0 } else { 1000.0 };
    let [x1, y1, x2, y2] = [n[0] / scale, n[1] / scale, n[2] / scale, n[3] / scale].map(|v| v.clamp(0.0, 1.0));
    let r = [x1.min(x2), y1.min(y2), x1.max(x2), y1.max(y2)];
    (r[2] - r[0] > 0.01 && r[3] - r[1] > 0.01).then_some(r)
}

/// Toglie i riquadri dai segnaposto (per chi riceve solo il testo).
pub fn strip_rects(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for p in find(text) {
        out.push_str(&text[last..p.start]);
        out.push_str(&format!("[figura: {}]", p.desc));
        last = p.end;
    }
    out.push_str(&text[last..]);
    out
}

/// La figura: i tratti il cui centro cade nel riquadro (allargato un poco),
/// ritagliati attorno a sé stessi. `None` se lì non c'è inchiostro.
pub fn crop(page: &Page, rect: [f32; 4]) -> Option<Page> {
    let pad = 0.03;
    let [x1, y1, x2, y2] = [rect[0] - pad, rect[1] - pad, rect[2] + pad, rect[3] + pad];
    let picked: Vec<&PageStroke> = page
        .strokes
        .iter()
        .filter(|st| {
            let (mut u0, mut v0, mut u1, mut v1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
            for &(u, v) in &st.pts {
                u0 = u0.min(u);
                v0 = v0.min(v);
                u1 = u1.max(u);
                v1 = v1.max(v);
            }
            let (cu, cv) = ((u0 + u1) / 2.0, (v0 + v1) / 2.0);
            !st.pts.is_empty() && cu >= x1 && cu <= x2 && cv >= y1 && cv <= y2
        })
        .collect();
    if picked.is_empty() {
        return None;
    }
    let sub = Page { w: page.w, h: page.h, strokes: picked.into_iter().cloned().collect(), bg: None };
    Some(crate::export::crop_to_content(&sub, 0.015))
}

/// Sostituisce i segnaposto con le figure ridisegnate: un SVG per figura in
/// `dir` (`<stem>-fig<N>.svg`) e nel testo l'immagine con la didascalia.
/// I segnaposto senza riquadro o senza inchiostro restano come testo.
pub fn extract(text: &str, page: &Page, dir: &Path, stem: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    let mut n = 0;
    for p in find(text) {
        out.push_str(&text[last..p.start]);
        last = p.end;
        let fig = p.rect.and_then(|r| crop(page, r));
        let Some(fig) = fig else {
            out.push_str(&format!("[figura: {}]", p.desc));
            continue;
        };
        n += 1;
        let name = format!("{stem}-fig{n}.svg");
        if let Err(e) = std::fs::create_dir_all(dir).and_then(|_| crate::export::save_svg(&fig, &dir.join(&name))) {
            tracing::warn!("figura {name}: {e}");
            out.push_str(&format!("[figura: {}]", p.desc));
            continue;
        }
        // larga quanto sul foglio, rispetto alla pagina vista dal modello
        let pct = ((fig.w / page.w) * 100.0).clamp(25.0, 100.0).round();
        let alt = p.desc.replace('"', "'");
        out.push_str(&format!("<img src=\"{name}\" alt=\"{alt}\" width=\"{pct}%\">\n\n*Figura: {}*", p.desc));
    }
    out.push_str(&text[last..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page_with(strokes: Vec<Vec<(f32, f32)>>) -> Page {
        Page {
            w: 21_000.0,
            h: 29_700.0,
            strokes: strokes.into_iter().map(|pts| PageStroke { color: [0, 0, 0], width: 2.0, pts, highlight: false }).collect(),
            bg: None,
        }
    }

    #[test]
    fn segnaposto() {
        let t = "Testo\n[figura: piano cartesiano (P) | riquadro: 100,400,600,800]\nGiusto?\n[figura: freccia]";
        let f = find(t);
        assert_eq!(f.len(), 2);
        assert_eq!(f[0].desc, "piano cartesiano (P)");
        assert_eq!(f[0].rect, Some([0.1, 0.4, 0.6, 0.8]));
        assert_eq!(f[1].rect, None);
        assert_eq!(strip_rects(t), "Testo\n[figura: piano cartesiano (P)]\nGiusto?\n[figura: freccia]");
        assert_eq!(parse_rect("riquadro: 0.1 0.2 0.5 0.6"), Some([0.1, 0.2, 0.5, 0.6]));
        assert_eq!(parse_rect("riquadro: 1,2"), None);
    }

    #[test]
    fn figura_ridisegnata() {
        // testo in alto, un asse in basso
        let page = page_with(vec![vec![(0.1, 0.1), (0.5, 0.1)], vec![(0.2, 0.6), (0.8, 0.6)], vec![(0.5, 0.45), (0.5, 0.75)]]);
        let fig = crop(&page, [0.15, 0.4, 0.85, 0.8]).unwrap();
        assert_eq!(fig.strokes.len(), 2);
        assert!(crop(&page, [0.9, 0.9, 0.95, 0.95]).is_none());
        let dir = std::env::temp_dir().join(format!("openslate-fig-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let out = extract("Sopra\n[figura: assi | riquadro: 150,400,850,800]\nSotto", &page, &dir, "appunti-x");
        assert!(out.contains("<img src=\"appunti-x-fig1.svg\"") && out.contains("*Figura: assi*") && out.ends_with("Sotto"), "{out}");
        assert!(std::fs::read_to_string(dir.join("appunti-x-fig1.svg")).unwrap().contains("<svg"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
