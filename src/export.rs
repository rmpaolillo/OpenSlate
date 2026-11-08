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
                PageStroke { color: [255, 0, 0], width: 2.0, pts: vec![(0.1, 0.1), (0.5, 0.5)], highlight: false },
                PageStroke { color: [0, 0, 0], width: 2.0, pts: vec![(0.7, 0.7)], highlight: false },
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
            strokes: vec![PageStroke { color: [255, 235, 90], width: 16.0, pts: (0..50).map(|i| (0.1 + i as f32 * 0.01, 0.2)).collect(), highlight: true }],
            bg: Some(PageBg { png: std::sync::Arc::new(png), rect: [0.0, 0.0, 1.0, 1.0], key: "x".into() }),
        };
        let t = std::time::Instant::now();
        let _ = super::page_to_rgba(&page, 2200).unwrap();
        println!("fusione 2200 px: {:?}", t.elapsed());
    }
}
