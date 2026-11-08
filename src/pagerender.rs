//! Una pagina in pixel (tiny-skia): anteprime, «Copia», l'immagine per l'AI.
//! Lo spessore dei tratti segue la regola dello schermo: `width` pixel su una
//! pagina alta 800 px, in proporzione all'altezza della pagina. La usano la
//! desktop (`export` la riesporta) e le app per telefono (`mobile/core`).

use crate::scene::Page;

/// Misura della pagina in millimetri: il lato lungo è 297 mm (A4).
pub fn page_mm(page: &Page) -> (f32, f32) {
    if page.h >= page.w {
        (297.0 * page.w / page.h, 297.0)
    } else {
        (297.0, 297.0 * page.h / page.w)
    }
}

pub fn stroke_width(width: f32, page_h: f32) -> f32 {
    (width * page_h / 800.0).max(page_h * 0.0008)
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
