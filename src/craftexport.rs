//! Export dei fogli (lavagna e pagine dell'archivio) con le librerie della
//! suite Craft di ArtCraft: la pagina diventa un documento di VectorCraft
//! (una tavola per pagina, lo sfondo come immagine, ogni tratto un tracciato
//! con le punte tonde, l'evidenziatore in «moltiplica»), e da lì
//! - SVG e PDF li scrive VectorCraft;
//! - PNG: lo disegna il motore di VectorCraft e lo scrive PhotoCraft (con i
//!   DPI giusti, così il foglio stampato è grande quanto un A4).

use crate::scene::Page;
use anyhow::{anyhow, Result};
use serde_json::json;
use vectorcraft_engine::Session;

/// Millimetri → punti.
const MM: f64 = 72.0 / 25.4;

fn run(s: &mut Session, cmd: &str, p: serde_json::Value) -> Result<serde_json::Value> {
    s.execute(cmd, &p).map_err(|e| anyhow!("VectorCraft, {cmd}: {e}"))
}

fn b64(v: &serde_json::Value) -> Result<Vec<u8>> {
    use base64::Engine;
    let s = v["dataBase64"].as_str().ok_or_else(|| anyhow!("VectorCraft: nessun dato"))?;
    Ok(base64::engine::general_purpose::STANDARD.decode(s)?)
}

/// Le pagine come documento di VectorCraft: una tavola per pagina, una
/// accanto all'altra. Ritorna la sessione e le tavole (x, y, l, a in punti).
fn document(pages: &[&Page]) -> Result<(Session, Vec<[f64; 4]>)> {
    let sizes: Vec<(f64, f64)> = pages
        .iter()
        .map(|p| {
            let (w, h) = crate::export::page_mm(p);
            (w as f64 * MM, h as f64 * MM)
        })
        .collect();
    let (w0, h0) = sizes.first().copied().unwrap_or((210.0 * MM, 297.0 * MM));
    let mut s = Session::new();
    s.add_document(vectorcraft_engine::doc::Document::new(w0, h0), None);
    let mut x = 0.0;
    let mut boards = vec![[0.0, 0.0, w0, h0]];
    for (i, &(w, h)) in sizes.iter().enumerate().skip(1) {
        x += sizes[i - 1].0 + 36.0;
        run(&mut s, "artboard.new", json!({ "x": x, "y": 0.0, "width": w, "height": h }))?;
        boards.push([x, 0.0, w, h]);
    }
    // le tavole come le ha messe VectorCraft (coordinate del documento)
    if let Ok(d) = s.doc() {
        let real: Vec<[f64; 4]> = d.doc.artboards.iter().map(|a| [a.rect.x0, a.rect.y0, a.rect.width(), a.rect.height()]).collect();
        if real.len() == boards.len() {
            boards = real;
        }
    }
    for (page, &[bx, by, bw, bh]) in pages.iter().zip(&boards) {
        if let Some(bg) = &page.bg {
            use base64::Engine;
            run(&mut s, "select.none", json!({}))?;
            run(&mut s, "file.place", json!({ "name": "pagina.png", "dataBase64": base64::engine::general_purpose::STANDARD.encode(bg.png.as_slice()) }))?;
            let [rx, ry, rw, rh] = bg.rect.map(|v| v as f64);
            run(&mut s, "object.setBounds", json!({ "x": bx + rx * bw, "y": by + ry * bh, "width": rw * bw, "height": rh * bh, "reference": 0 }))?;
        }
        for st in &page.strokes {
            if st.pts.is_empty() {
                continue;
            }
            run(&mut s, "select.none", json!({}))?;
            let color = format!("#{:02x}{:02x}{:02x}", st.color[0], st.color[1], st.color[2]);
            let weight = crate::export::stroke_width(st.width, bh as f32) as f64;
            run(&mut s, "paint.setFill", json!({ "none": true }))?;
            run(&mut s, "paint.setStroke", json!({ "color": color }))?;
            run(&mut s, "stroke.set", json!({ "weight": weight, "cap": "round", "join": "round" }))?;
            // l'evidenziatore «moltiplica»: il testo sotto resta leggibile
            s.paint.blend = if st.highlight {
                vectorcraft_engine::doc::color::BlendMode::Multiply
            } else {
                vectorcraft_engine::doc::color::BlendMode::Normal
            };
            s.paint.opacity = 1.0;
            let mut anchors: Vec<serde_json::Value> = st.pts.iter().map(|&(u, v)| json!({ "x": bx + u as f64 * bw, "y": by + v as f64 * bh })).collect();
            if anchors.len() == 1 {
                // un punto: un trattino lungo quasi niente, con le punte tonde
                let (u, v) = st.pts[0];
                anchors.push(json!({ "x": bx + u as f64 * bw + 0.01, "y": by + v as f64 * bh }));
            }
            run(&mut s, "path.create", json!({ "anchors": anchors, "closed": false }))?;
        }
    }
    run(&mut s, "select.none", json!({}))?;
    Ok((s, boards))
}

/// La pagina in SVG (misura A4 reale), scritto da VectorCraft.
pub fn svg(page: &Page) -> Result<Vec<u8>> {
    let (mut s, _) = document(&[page])?;
    b64(&run(&mut s, "document.export", json!({ "format": "svg", "artboard": 0, "background": "white" }))?)
}

/// Le pagine in un PDF vettoriale (una pagina per foglio), scritto da VectorCraft.
pub fn pdf(pages: &[&Page]) -> Result<Vec<u8>> {
    let (mut s, _) = document(pages)?;
    b64(&run(&mut s, "document.exportPdf", json!({ "preserveEditing": false }))?)
}

/// La pagina in PNG, `long_side` pixel sul lato lungo: la disegna VectorCraft,
/// la scrive PhotoCraft (con i DPI, così stampata è grande quanto il foglio).
pub fn png(page: &Page, long_side: u32) -> Result<Vec<u8>> {
    raster(page, long_side, photocraft_codecs::Format::Png)
}

/// La pagina come immagine nel formato `fmt` (PNG, JPEG, WebP…), `long_side`
/// pixel sul lato lungo: la disegna VectorCraft, la scrive PhotoCraft. JPEG di
/// qualità alta senza sottocampionare i colori (la scrittura resta netta),
/// WebP senza perdite (una pagina scritta pesa poco).
pub fn raster(page: &Page, long_side: u32, fmt: photocraft_codecs::Format) -> Result<Vec<u8>> {
    let (s, boards) = document(&[page])?;
    let d = s.doc().map_err(|e| anyhow!("VectorCraft: {e}"))?;
    let rect = d.doc.artboards.first().map(|a| a.rect).ok_or_else(|| anyhow!("VectorCraft: nessuna tavola"))?;
    let [_, _, bw, bh] = boards[0];
    let scale = long_side as f64 / bw.max(bh);
    let r = vectorcraft_render::Renderer::new().render_region(&d.doc, rect, scale, true);
    let img = photocraft_codecs::Image::from_u8(r.width, r.height, photocraft_codecs::ChannelLayout::Rgba, r.to_straight())
        .map_err(|e| anyhow!("PhotoCraft: {e}"))?
        .convert(photocraft_codecs::ChannelLayout::Rgb, photocraft_codecs::SampleType::U8);
    let dpi = (72.0 * scale) as f32;
    let img = img.with_meta(photocraft_codecs::Metadata { dpi: Some((dpi, dpi)), ..Default::default() });
    let opts = photocraft_codecs::EncodeOptions { jpeg_quality: 90, jpeg_chroma_subsampling: false, webp_lossless: true, ..Default::default() };
    photocraft_codecs::encode(&img, fmt, &opts).map_err(|e| anyhow!("PhotoCraft: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{PageBg, PageStroke};

    fn sample(bg: bool) -> Page {
        let png = {
            let mut pm = tiny_skia::Pixmap::new(210, 297).unwrap();
            pm.fill(tiny_skia::Color::from_rgba8(230, 240, 255, 255));
            pm.encode_png().unwrap()
        };
        Page {
            w: 21000.0,
            h: 29700.0,
            strokes: vec![
                PageStroke { color: [255, 0, 0], width: 2.0, pts: vec![(0.1, 0.1), (0.5, 0.5), (0.9, 0.2)], highlight: false },
                PageStroke { color: [0, 0, 0], width: 2.0, pts: vec![(0.7, 0.7)], highlight: false },
                PageStroke { color: [255, 235, 90], width: 16.0, pts: vec![(0.1, 0.8), (0.9, 0.8)], highlight: true },
            ],
            bg: bg.then(|| PageBg { png: std::sync::Arc::new(png), rect: [0.1, 0.1, 0.8, 0.8], key: "x".into() }),
        }
    }

    #[test]
    fn svg_pdf_png_dalla_suite() {
        let p = sample(true);
        let svg = String::from_utf8(svg(&p).unwrap()).unwrap();
        assert!(svg.contains("<svg") && svg.to_lowercase().contains("ff0000"), "{}", &svg[..svg.len().min(800)]);
        let pdf = pdf(&[&p, &sample(false)]).unwrap();
        assert!(pdf.starts_with(b"%PDF"));
        let info = crate::pdfdoc::pages_from_bytes(&pdf).unwrap();
        assert_eq!(info.len(), 2);
        let (w, h) = info[0].display_size();
        assert!((w - 595.3).abs() < 1.5 && (h - 841.9).abs() < 1.5, "{w}×{h}");
        let png = png(&p, 600).unwrap();
        let img = tiny_skia::Pixmap::decode_png(&png).unwrap();
        assert_eq!(img.height(), 600);
        // lo sfondo dentro il suo riquadro, il bianco fuori
        if let Ok(d) = std::env::var("CRAFT_OUT") {
            std::fs::write(format!("{d}/c.png"), &png).unwrap();
            std::fs::write(format!("{d}/c.svg"), svg.as_bytes()).unwrap();
            std::fs::write(format!("{d}/c.pdf"), &pdf).unwrap();
        }
        let inside = img.pixel(img.width() / 4, 360).unwrap();
        let outside = img.pixel(5, 5).unwrap();
        assert!(inside.blue() == 255 && inside.red() < 240, "{inside:?}");
        assert_eq!((outside.red(), outside.green(), outside.blue()), (255, 255, 255));
    }
}

#[cfg(test)]
mod bench {
    /// cargo test --release -- --ignored tempo_craft --nocapture
    #[test]
    #[ignore]
    fn tempo_craft() {
        use crate::scene::{Page, PageStroke};
        let page = Page {
            w: 21000.0,
            h: 29700.0,
            strokes: (0..300)
                .map(|k| PageStroke { color: [0, 0, 0], width: 2.0, pts: (0..120).map(|i| (0.05 + i as f32 * 0.007, 0.03 + k as f32 * 0.003)).collect(), highlight: false })
                .collect(),
            bg: None,
        };
        let t = std::time::Instant::now();
        let s = super::svg(&page).unwrap();
        println!("svg {:?} ({} KB)", t.elapsed(), s.len() / 1024);
        let t = std::time::Instant::now();
        let p = super::pdf(&[&page]).unwrap();
        println!("pdf {:?} ({} KB)", t.elapsed(), p.len() / 1024);
        let t = std::time::Instant::now();
        let p = super::png(&page, 1600).unwrap();
        println!("png {:?} ({} KB)", t.elapsed(), p.len() / 1024);
        let t = std::time::Instant::now();
        let _ = crate::export::page_to_png(&page, 1600).unwrap();
        println!("png tiny-skia {:?}", t.elapsed());
    }
}
