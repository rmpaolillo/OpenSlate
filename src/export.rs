//! Una pagina in pixel per l'app (anteprime, «Copia», immagine per l'AI) e i
//! file esportati: quelli li scrivono le librerie della suite Craft
//! (`craftexport`: SVG e PDF di VectorCraft, PNG con PhotoCraft).
//!
//! Lo spessore dei tratti segue la stessa regola dello schermo: `width` pixel
//! su una pagina alta 800 px, in proporzione all'altezza della pagina.

use crate::scene::Page;
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

/// Misura della pagina in millimetri: il lato lungo è 297 mm (A4).
pub(crate) fn page_mm(page: &Page) -> (f32, f32) {
    if page.h >= page.w {
        (297.0 * page.w / page.h, 297.0)
    } else {
        (297.0, 297.0 * page.h / page.w)
    }
}

pub(crate) fn stroke_width(width: f32, page_h: f32) -> f32 {
    (width * page_h / 800.0).max(page_h * 0.0008)
}

// ---------------------------------------------------------------------------
// PNG
// ---------------------------------------------------------------------------

/// Ritaglia la pagina attorno a ciò che è scritto, con un margine (frazione
/// del lato lungo). Lo spessore dei tratti resta proporzionato alla scrittura.
/// Utile per incollare in una chat di AI: la scrittura risulta più grande.
pub fn crop_to_content(page: &Page, margin: f32) -> Page {
    use crate::scene::PageStroke;
    let (mut u0, mut v0, mut u1, mut v1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for st in &page.strokes {
        for &(u, v) in &st.pts {
            u0 = u0.min(u);
            v0 = v0.min(v);
            u1 = u1.max(u);
            v1 = v1.max(v);
        }
    }
    if u0 > u1 {
        return Page { w: page.w, h: page.h, strokes: Vec::new(), bg: page.bg.clone() };
    }
    // margine uguale in orizzontale e verticale, in unità della pagina
    let m = margin * page.w.max(page.h);
    let (mu, mv) = (m / page.w, m / page.h);
    let (u0, v0) = ((u0 - mu).max(0.0), (v0 - mv).max(0.0));
    let (u1, v1) = ((u1 + mu).min(1.0), (v1 + mv).min(1.0));
    let (du, dv) = ((u1 - u0).max(1e-4), (v1 - v0).max(1e-4));
    let (w, h) = (page.w * du, page.h * dv);
    Page {
        w,
        h,
        strokes: page
            .strokes
            .iter()
            .map(|st| PageStroke {
                color: st.color,
                // stesso spessore rispetto alla scrittura
                width: st.width * page.h / h,
                pts: st.pts.iter().map(|&(u, v)| ((u - u0) / du, (v - v0) / dv)).collect(),
                highlight: st.highlight,
            })
            .collect(),
        bg: page.bg.as_ref().map(|b| crate::scene::PageBg {
            rect: [(b.rect[0] - u0) / du, (b.rect[1] - v0) / dv, b.rect[2] / du, b.rect[3] / dv],
            ..b.clone()
        }),
    }
}

/// Rasterizza la pagina in RGBA (larghezza, altezza, pixel).
pub fn page_to_rgba(page: &Page, long_side: u32) -> anyhow::Result<(usize, usize, Vec<u8>)> {
    let pm = render(page, long_side)?;
    // Sfondo bianco opaco: i pixel premoltiplicati coincidono con quelli normali.
    Ok((pm.width() as usize, pm.height() as usize, pm.data().to_vec()))
}

/// Rasterizza la pagina in PNG; `long_side` = pixel del lato lungo.
pub fn page_to_png(page: &Page, long_side: u32) -> anyhow::Result<Vec<u8>> {
    Ok(render(page, long_side)?.encode_png()?)
}

fn render(page: &Page, long_side: u32) -> anyhow::Result<tiny_skia::Pixmap> {
    use tiny_skia::{Color, FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform};
    let (wmm, hmm) = page_mm(page);
    let scale = long_side as f32 / wmm.max(hmm);
    let (w, h) = ((wmm * scale).round().max(1.0), (hmm * scale).round().max(1.0));
    let mut pm = Pixmap::new(w as u32, h as u32).ok_or_else(|| anyhow::anyhow!("pagina vuota"))?;
    pm.fill(Color::WHITE);
    if let Some(bg) = &page.bg {
        let img = Pixmap::decode_png(&bg.png).map_err(|e| anyhow::anyhow!("sfondo: {e}"))?;
        let [rx, ry, rw, rh] = bg.rect;
        let t = Transform::from_row(
            rw * w / img.width() as f32,
            0.0,
            0.0,
            rh * h / img.height() as f32,
            rx * w,
            ry * h,
        );
        let paint = tiny_skia::PixmapPaint { quality: tiny_skia::FilterQuality::Bicubic, ..Default::default() };
        pm.draw_pixmap(0, 0, img.as_ref(), &paint, t, None);
    }
    for st in &page.strokes {
        let mut paint = Paint::default();
        paint.set_color_rgba8(st.color[0], st.color[1], st.color[2], 255);
        paint.anti_alias = true;
        // evidenziatore: "moltiplica", il testo sotto resta leggibile
        if st.highlight {
            paint.blend_mode = tiny_skia::BlendMode::Multiply;
        }
        let sw = stroke_width(st.width, h);
        if st.pts.len() == 1 {
            let (u, v) = st.pts[0];
            if let Some(c) = PathBuilder::from_circle(u * w, v * h, sw * 0.5) {
                pm.fill_path(&c, &paint, FillRule::Winding, Transform::identity(), None);
            }
            continue;
        }
        let mut pb = PathBuilder::new();
        for (j, (u, v)) in st.pts.iter().enumerate() {
            if j == 0 {
                pb.move_to(u * w, v * h);
            } else {
                pb.line_to(u * w, v * h);
            }
        }
        if let Some(path) = pb.finish() {
            let stroke = Stroke {
                width: sw,
                line_cap: LineCap::Round,
                line_join: LineJoin::Round,
                ..Stroke::default()
            };
            pm.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }
    Ok(pm)
}

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
