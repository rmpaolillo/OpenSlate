//! Una pagina in pixel per l'app (anteprime, «Copia», immagine per l'AI) e i
//! file esportati: quelli li scrivono le librerie della suite Craft
//! (`craftexport`: SVG e PDF di VectorCraft, PNG con PhotoCraft).
//!
//! Lo spessore dei tratti segue la stessa regola dello schermo: `width` pixel
//! su una pagina alta 800 px, in proporzione all'altezza della pagina.

use crate::scene::Page;
pub use crate::pagerender::{page_to_png, page_to_rgba};
pub(crate) use crate::pagerender::{page_mm, stroke_width};
use std::path::{Path, PathBuf};

/// Cartella degli export (nella cartella del progetto).
pub const EXPORT_DIR: &str = "exports";

/// Percorso nuovo in `exports/` con data e ora: `exports/<prefisso>-AAAAMMGG-HHMMSS.<ext>`.
pub fn new_path(prefix: &str, ext: &str) -> PathBuf {
    let _ = std::fs::create_dir_all(EXPORT_DIR);
    let ts = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let mut p = PathBuf::from(format!("{EXPORT_DIR}/{prefix}-{ts}.{ext}"));
    let mut n = 2;
    while p.exists() {
        p = PathBuf::from(format!("{EXPORT_DIR}/{prefix}-{ts}-{n}.{ext}"));
        n += 1;
    }
    p
}

// ---------------------------------------------------------------------------
// PNG
// ---------------------------------------------------------------------------

pub use crate::pagerender::crop_to_content;

// ---------------------------------------------------------------------------
// Salvataggio
// ---------------------------------------------------------------------------

// I file si scrivono con le librerie della suite Craft (vedi `craftexport`):
// SVG e PDF da VectorCraft, PNG disegnato da VectorCraft e scritto da PhotoCraft.

pub fn save_svg(page: &Page, path: &Path) -> anyhow::Result<()> {
    std::fs::write(path, crate::craftexport::svg(page)?)?;
    Ok(())
}

pub fn save_pdf(pages: &[&Page], path: &Path) -> anyhow::Result<()> {
    std::fs::write(path, crate::craftexport::pdf(pages)?)?;
    Ok(())
}

/// La pagina con sotto la rigatura del foglio (righe, quadretti, margini…),
/// per stamparla come la si vede: linee sottili coi colori dello schermo (vedi
/// `ui::paper`), sotto la scrittura. Le scritte della rigatura (Cornell,
/// numeri del piano cartesiano) no: solo linee e puntini.
pub fn with_paper(mut page: Page, paper: crate::rigatura::Paper, margin: bool) -> Page {
    use crate::rigatura::{Ink, Mark, Paper};
    use crate::scene::PageStroke;
    let (wcm, hcm) = (page.w / 1000.0, page.h / 1000.0);
    let marks = crate::rigatura::layout(paper, wcm, hcm, margin);
    if marks.is_empty() {
        return page;
    }
    // i colori dello schermo, già fusi sul bianco della carta
    let on_white = |c: [u8; 3], a: f32| -> [u8; 3] { c.map(|v| (255.0 - (255.0 - v as f32) * a).round() as u8) };
    let mm = paper == Paper::Millimetrata;
    let rgb = if mm { [225, 130, 60] } else if paper.grid_like() { [110, 150, 205] } else { [80, 125, 200] };
    let line = if mm { 75.0 } else if paper.grid_like() { 95.0 } else { 130.0 } / 255.0;
    let strong = if mm { 140.0 } else { 175.0 } / 255.0;
    let style = |ink: Ink| -> ([u8; 3], f32) {
        match ink {
            Ink::Fine => (on_white(rgb, 80.0 / 255.0), 0.4),
            Ink::Line => (on_white(rgb, line), 0.6),
            Ink::Strong => (on_white(rgb, strong), 0.9),
            Ink::Red => (on_white([215, 70, 70], 150.0 / 255.0), 0.8),
            Ink::Axis => (on_white([60, 70, 95], 210.0 / 255.0), 1.0),
        }
    };
    let u = |p: (f32, f32)| (p.0 / wcm, p.1 / hcm);
    let mut strokes: Vec<PageStroke> = marks
        .into_iter()
        .filter_map(|m| match m {
            Mark::Line(ink, a, b) => {
                let (color, width) = style(ink);
                Some(PageStroke { color, width, pts: vec![u(a), u(b)], pr: vec![], highlight: false })
            }
            Mark::Dot(p) => Some(PageStroke { color: on_white(rgb, 170.0 / 255.0), width: 1.2, pts: vec![u(p)], pr: vec![], highlight: false }),
            Mark::Label(..) => None,
        })
        .collect();
    strokes.append(&mut page.strokes);
    page.strokes = strokes;
    page
}

pub fn save_png(page: &Page, path: &Path) -> anyhow::Result<()> {
    std::fs::write(path, crate::craftexport::png(page, 2480)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::PageStroke;

    fn sample() -> Page {
        Page {
            bg: None,
            w: 21000.0,
            h: 29700.0,
            strokes: vec![
                PageStroke { color: [255, 0, 0], width: 2.0, pts: vec![(0.1, 0.1), (0.5, 0.5)], pr: vec![], highlight: false },
                PageStroke { color: [0, 0, 0], width: 2.0, pts: vec![(0.7, 0.7)], pr: vec![], highlight: false },
            ],
        }
    }

    #[test]
    fn png() {
        let p = sample();
        let png = page_to_png(&p, 400).unwrap();
        assert_eq!(&png[1..4], b"PNG");
        let (w, h, px) = page_to_rgba(&p, 400).unwrap();
        assert_eq!(px.len(), w * h * 4);
    }

    #[test]
    fn stampa_con_le_righe() {
        let r = with_paper(sample(), crate::rigatura::Paper::Quadretti5, false);
        // sotto la scrittura, le linee dei quadretti (ogni 5 mm su 21 × 29,7 cm)
        assert!(r.strokes.len() > 90, "{}", r.strokes.len());
        assert_eq!(&r.strokes[r.strokes.len() - 2..].iter().map(|s| s.color).collect::<Vec<_>>(), &[[255, 0, 0], [0, 0, 0]]);
        assert!(r.strokes.iter().all(|s| s.pts.iter().all(|&(x, y)| (-0.01..=1.01).contains(&x) && (-0.01..=1.01).contains(&y))));
        assert_eq!(with_paper(sample(), crate::rigatura::Paper::Bianco, true).strokes.len(), 2);
        assert!(crate::craftexport::pdf(&[&r]).unwrap().starts_with(b"%PDF"));
    }

    #[test]
    fn ritaglio_attorno_alla_scrittura() {
        let p = sample(); // tratti tra 0.1 e 0.7
        let c = crop_to_content(&p, 0.0);
        assert!((c.w - 21000.0 * 0.6).abs() < 1.0 && (c.h - 29700.0 * 0.6).abs() < 1.0);
        assert_eq!(c.strokes[0].pts[0], (0.0, 0.0));
        assert!((c.strokes[0].width - 2.0 / 0.6).abs() < 1e-3);
        assert!(crop_to_content(&Page { w: 1.0, h: 1.0, ..Default::default() }, 0.05).is_empty());
    }
}

#[cfg(test)]
mod bench {
    /// Tempo della fusione sfondo + evidenziature (cargo test --release -- --ignored tempo_fusione --nocapture)
    #[test]
    #[ignore]
    fn tempo_fusione() {
        use crate::scene::{Page, PageBg, PageStroke};
        let png = {
            let mut pm = tiny_skia::Pixmap::new(1555, 2200).unwrap();
            pm.fill(tiny_skia::Color::WHITE);
            pm.encode_png().unwrap()
        };
        let page = Page {
            w: 21000.0,
            h: 29700.0,
            strokes: vec![PageStroke { color: [255, 235, 90], width: 16.0, pts: (0..50).map(|i| (0.1 + i as f32 * 0.01, 0.2)).collect(), pr: vec![], highlight: true }],
            bg: Some(PageBg { png: std::sync::Arc::new(png), rect: [0.0, 0.0, 1.0, 1.0], key: "x".into() }),
        };
        let t = std::time::Instant::now();
        let _ = super::page_to_rgba(&page, 2200).unwrap();
        println!("fusione 2200 px: {:?}", t.elapsed());
    }
}
