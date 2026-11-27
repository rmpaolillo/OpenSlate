//! L'export dei fogli sul telefono, senza VectorCraft (vedi `vecsign`): le
//! stesse funzioni di `src/craftexport.rs` della desktop.
//! - SVG: scritto qui, misura A4 reale (lo sfondo come immagine, ogni tratto
//!   un tracciato con le punte tonde, l'evidenziatore in «moltiplica»);
//! - JPEG e WebP: la pagina disegnata da `pagerender` (tiny-skia), scritta da
//!   PhotoCraft con i DPI giusti (stampata è grande quanto il foglio).

use crate::scene::Page;
use anyhow::{anyhow, Result};
use std::fmt::Write as _;

/// Millimetri → punti.
const MM: f32 = 72.0 / 25.4;

/// La pagina in SVG (misura A4 reale).
pub fn svg(page: &Page) -> Result<Vec<u8>> {
    use base64::Engine;
    let (wmm, hmm) = crate::export::page_mm(page);
    let (w, h) = (wmm * MM, hmm * MM);
    let mut s = String::new();
    let _ = writeln!(
        s,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{wmm:.2}mm\" height=\"{hmm:.2}mm\" viewBox=\"0 0 {w:.2} {h:.2}\">\n<rect width=\"{w:.2}\" height=\"{h:.2}\" fill=\"#ffffff\"/>"
    );
    if let Some(bg) = &page.bg {
        let [rx, ry, rw, rh] = bg.rect;
        let _ = writeln!(
            s,
            "<image x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" preserveAspectRatio=\"none\" href=\"data:image/png;base64,{}\"/>",
            rx * w,
            ry * h,
            rw * w,
            rh * h,
            base64::engine::general_purpose::STANDARD.encode(bg.png.as_slice())
        );
    }
    for st in &page.strokes {
        if st.pts.is_empty() {
            continue;
        }
        let [r, g, b] = st.color;
        let color = format!("#{r:02x}{g:02x}{b:02x}");
        let weight = crate::export::stroke_width(st.width, h);
        // l'evidenziatore «moltiplica»: il testo sotto resta leggibile
        let blend = if st.highlight { " style=\"mix-blend-mode:multiply\"" } else { "" };
        let pts: Vec<(f32, f32)> = st.pts.iter().map(|&(u, v)| (u * w, v * h)).collect();
        // tratto naturale: il contorno pieno, con lo spessore della pressione
        if !st.pr.is_empty() {
            let o = crate::inkshape::outline(&pts, &st.pr, weight);
            if o.len() >= 3 {
                let _ = writeln!(s, "<path d=\"{}Z\" fill=\"{color}\"{blend}/>", path_d(&o));
                continue;
            }
        }
        // un punto: un trattino lungo quasi niente, con le punte tonde
        let pts = if pts.len() == 1 { vec![pts[0], (pts[0].0 + 0.01, pts[0].1)] } else { pts };
        let _ = writeln!(
            s,
            "<path d=\"{}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"{weight:.3}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"{blend}/>",
            path_d(&pts)
        );
    }
    s.push_str("</svg>\n");
    Ok(s.into_bytes())
}

fn path_d(pts: &[(f32, f32)]) -> String {
    let mut d = String::new();
    for (i, (x, y)) in pts.iter().enumerate() {
        let _ = write!(d, "{}{x:.2} {y:.2}", if i == 0 { "M" } else { " L" });
    }
    d
}

/// La pagina come immagine nel formato `fmt` (JPEG, WebP…), `long_side`
/// pixel sul lato lungo. JPEG di qualità alta senza sottocampionare i colori
/// (la scrittura resta netta), WebP senza perdite (una pagina scritta pesa poco).
pub fn raster(page: &Page, long_side: u32, fmt: photocraft_codecs::Format) -> Result<Vec<u8>> {
    let (w, h, rgba) = crate::export::page_to_rgba(page, long_side)?;
    let img = photocraft_codecs::Image::from_u8(w as u32, h as u32, photocraft_codecs::ChannelLayout::Rgba, rgba)
        .map_err(|e| anyhow!("PhotoCraft: {e}"))?
        .convert(photocraft_codecs::ChannelLayout::Rgb, photocraft_codecs::SampleType::U8);
    let (wmm, hmm) = crate::export::page_mm(page);
    let dpi = long_side as f32 / (wmm.max(hmm) / 25.4);
    let img = img.with_meta(photocraft_codecs::Metadata { dpi: Some((dpi, dpi)), ..Default::default() });
    let opts = photocraft_codecs::EncodeOptions { jpeg_quality: 90, jpeg_chroma_subsampling: false, webp_lossless: true, ..Default::default() };
    photocraft_codecs::encode(&img, fmt, &opts).map_err(|e| anyhow!("PhotoCraft: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{PageBg, PageStroke};

    fn sample() -> Page {
        let png = {
            let mut pm = tiny_skia::Pixmap::new(210, 297).unwrap();
            pm.fill(tiny_skia::Color::from_rgba8(230, 240, 255, 255));
            pm.encode_png().unwrap()
        };
        Page {
            w: 21000.0,
            h: 29700.0,
            strokes: vec![
                PageStroke { color: [255, 0, 0], width: 2.0, pts: vec![(0.1, 0.1), (0.5, 0.5), (0.9, 0.2)], pr: vec![], highlight: false },
                PageStroke { color: [0, 0, 0], width: 2.0, pts: vec![(0.7, 0.7)], pr: vec![], highlight: false },
                PageStroke { color: [0, 0, 255], width: 3.0, pts: vec![(0.2, 0.8), (0.4, 0.82), (0.6, 0.8)], pr: vec![0.2, 1.0, 0.4], highlight: false },
                PageStroke { color: [255, 230, 0], width: 12.0, pts: vec![(0.1, 0.3), (0.9, 0.3)], pr: vec![], highlight: true },
            ],
            bg: Some(PageBg { png: std::sync::Arc::new(png), rect: [0.0, 0.0, 1.0, 1.0], key: "prova".into() }),
        }
    }

    #[test]
    fn svg_si_riapre_e_si_vede() {
        let svg = svg(&sample()).unwrap();
        let s = std::str::from_utf8(&svg).unwrap();
        assert!(s.contains("width=\"210.00mm\"") && s.contains("height=\"297.00mm\""), "{s}");
        assert!(s.contains("mix-blend-mode:multiply") && s.contains("data:image/png"));
        let t = usvg::Tree::from_data(&svg, &usvg::Options::default()).unwrap();
        let mut pm = resvg::tiny_skia::Pixmap::new(210, 297).unwrap();
        let k = 210.0 / t.size().width();
        resvg::render(&t, resvg::tiny_skia::Transform::from_scale(k, k), &mut pm.as_mut());
        // il tratto rosso (sottile: sfumato) a metà pagina, lo sfondo azzurrino altrove
        let p = pm.pixel(105, 148).unwrap();
        assert!(p.red() as i32 > p.green() as i32 + 50, "{p:?}");
        let bg = pm.pixel(5, 290).unwrap();
        assert_eq!((bg.red(), bg.green(), bg.blue()), (230, 240, 255));
    }

    #[test]
    fn jpeg_e_webp() {
        for fmt in [photocraft_codecs::Format::Jpeg, photocraft_codecs::Format::WebP] {
            let b = raster(&sample(), 800, fmt).unwrap();
            let img = photocraft_codecs::decode(&b).unwrap();
            assert_eq!(img.dimensions(), (566, 800), "{fmt:?}");
        }
    }
}
