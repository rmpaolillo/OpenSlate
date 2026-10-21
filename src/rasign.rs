//! Foto e immagini raster (PNG, JPEG, WebP, TIFF, BMP, GIF) con le librerie di
//! PhotoCraft (https://github.com/storytold/photocraft, MIT OR Apache-2.0,
//! © ArtCraft Team), senza la sua interfaccia, come i Word con WordCraft e gli
//! SVG con VectorCraft:
//! - `photocraft-codecs` legge e scrive i file (profilo colore, DPI, EXIF e
//!   orientamento compresi);
//! - `photocraft-paint` disegna la scrittura del pennino con il suo pennello
//!   (bordi morbidi; l'evidenziatore trasparente non si scurisce dove si ripassa).
//!
//! Per la lavagna l'immagine diventa un PDF di una pagina grande quanto
//! l'immagine (alla sua risoluzione); salvando firmata resta un'immagine dello
//! stesso formato e con gli stessi pixel, la scrittura disegnata sopra.

use crate::wordsign::InkStroke;
use anyhow::{anyhow, bail, Result};
use photocraft_codecs::{ChannelLayout, EncodeOptions, Format, Image, SampleType};
use photocraft_color::{BlendMode, PixelFormat};
use photocraft_geom::Rect;
use photocraft_paint::{BrushSettings, Stroke, StrokePoint};
use photocraft_raster::Surface;
use std::path::Path;

/// Estensioni aperte con PhotoCraft.
pub fn is_raster(path: &Path) -> bool {
    matches!(ext(path).as_str(), "png" | "jpg" | "jpeg" | "webp" | "tif" | "tiff" | "bmp" | "gif")
}

/// Estensione del file firmato: PNG e JPEG restano tali; gli altri formati
/// (GIF a 256 colori, BMP…) diventano PNG, che non perde nulla.
pub fn signed_ext(src_ext: &str) -> &'static str {
    match src_ext {
        "jpg" | "jpeg" => "jpg",
        "webp" => "webp",
        "tif" | "tiff" => "tiff",
        _ => "png",
    }
}

fn ext(path: &Path) -> String {
    path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase()
}

/// Larghezza e altezza in pixel (dritta secondo l'EXIF).
pub fn dimensions(path: &Path) -> Option<(u32, u32)> {
    decode(path).ok().map(|(img, _)| img.dimensions())
}

/// La qualità del JPEG (stimata), per riscriverlo senza perdere troppo.
pub fn jpeg_quality(path: &Path) -> Option<u8> {
    photocraft_codecs::jpeg_quality(&std::fs::read(path).ok()?).map(|q| q.clamp(85, 100))
}

/// L'immagine (dritta secondo l'EXIF).
fn decode(path: &Path) -> Result<(Image, Vec<u8>)> {
    let bytes = std::fs::read(path).map_err(|e| anyhow!("{}: {e}", path.display()))?;
    let img = photocraft_codecs::decode(&bytes).map_err(|e| anyhow!("PhotoCraft non apre l'immagine: {e}"))?;
    Ok((img, bytes))
}

/// Punti tipografici per pixel (72 ÷ DPI; senza DPI 72, come le schermate di
/// solito: 1 px = 1 pt; oltre i 300 DPI si limita, il foglio non diventa minuscolo).
fn pt_per_px(img: &Image) -> f64 {
    let dpi = img.meta.dpi.map(|(x, _)| x as f64).filter(|d| d.is_finite() && *d >= 36.0).unwrap_or(72.0).min(300.0);
    72.0 / dpi
}

/// L'immagine come PDF di una pagina (per le pagine della lavagna).
pub fn to_pdf(input: &Path, out: &Path) -> Result<()> {
    let (img, _) = decode(input)?;
    let (w, h) = img.dimensions();
    // sopra il bianco: la trasparenza di un PNG si vede come carta
    let rgba = img.convert(ChannelLayout::Rgba, SampleType::U8);
    let px = rgba.as_u8().ok_or_else(|| anyhow!("PhotoCraft: pixel non a 8 bit"))?;
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for p in px.chunks_exact(4) {
        let a = p[3] as u32;
        for c in &p[..3] {
            rgb.push(((*c as u32 * a + 255 * (255 - a)) / 255) as u8);
        }
    }
    let k = pt_per_px(&img);
    crate::pdfdoc::image_pdf(out, w, h, &rgb, w as f64 * k, h as f64 * k)?;
    Ok(())
}

/// Copia `src` in `dst` con i tratti disegnati sopra (stesso formato, stessi pixel).
pub fn save_with_ink(src: &Path, dst: &Path, strokes: &[InkStroke]) -> Result<()> {
    let (img, bytes) = decode(src)?;
    let (w, h) = img.dimensions();
    let rgba = img.convert(ChannelLayout::Rgba, SampleType::U8);
    let px = rgba.as_u8().ok_or_else(|| anyhow!("PhotoCraft: pixel non a 8 bit"))?;
    let area = Rect::from_xywh(0, 0, w, h);
    let mut surface = Surface::from_interleaved(PixelFormat::RGBA8, area, px);
    for st in strokes.iter().filter(|s| s.page == 0 && !s.pts.is_empty()) {
        let mut points: Vec<StrokePoint> =
            st.pts.iter().take(5000).map(|(u, v)| StrokePoint::new(u.clamp(0.0, 1.0) as f64 * w as f64, v.clamp(0.0, 1.0) as f64 * h as f64, 1.0)).collect();
        if points.len() == 1 {
            let mut p = points[0];
            p.x += 0.3;
            points.push(p);
        }
        let [r, g, b] = st.color;
        let brush = BrushSettings {
            size: (st.width_frac * h as f32).clamp(0.6, 400.0),
            hardness: 0.9,
            spacing: 0.05,
            pressure_size: false,
            color: [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0],
            // l'evidenziatore: trasparente e «moltiplica», come un pennarello sulla carta
            opacity: if st.highlight { 0.45 } else { 1.0 },
            mode: if st.highlight { BlendMode::Multiply } else { BlendMode::Normal },
            ..Default::default()
        };
        photocraft_paint::apply_stroke(&mut surface, &Stroke { brush, points }, None, false);
    }
    let painted = surface.to_interleaved(area);
    let out_ext = ext(dst);
    let format = match out_ext.as_str() {
        "jpg" | "jpeg" => Format::Jpeg,
        "webp" => Format::WebP,
        "tif" | "tiff" => Format::Tiff,
        "png" => Format::Png,
        other => bail!("formato non gestito: {other}"),
    };
    let mut out = Image::from_u8(w, h, ChannelLayout::Rgba, painted).map_err(|e| anyhow!("PhotoCraft: {e}"))?;
    // il JPEG non ha la trasparenza
    if format == Format::Jpeg {
        out = out.convert(ChannelLayout::Rgb, SampleType::U8);
    }
    // profilo colore, DPI e metadati dell'originale
    let out = out.with_icc(img.icc.clone()).with_meta(img.meta.clone());
    let opts = EncodeOptions { jpeg_quality: photocraft_codecs::jpeg_quality(&bytes).unwrap_or(92).max(85), ..Default::default() };
    let data = photocraft_codecs::encode(&out, format, &opts).map_err(|e| anyhow!("PhotoCraft, salvataggio: {e}"))?;
    std::fs::write(dst, data)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("openslate-ras-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d.join(name)
    }

    fn ink(highlight: bool) -> Vec<InkStroke> {
        vec![InkStroke { page: 0, pts: vec![(0.2, 0.5), (0.5, 0.55), (0.8, 0.5)], color: [200, 0, 0], width_frac: 0.03, highlight }]
    }

    fn white_png(w: u32, h: u32) -> Vec<u8> {
        let mut px = tiny_skia::Pixmap::new(w, h).unwrap();
        px.fill(tiny_skia::Color::WHITE);
        px.encode_png().unwrap()
    }

    #[test]
    fn png_firmato_stessi_pixel() {
        let src = tmp("foto.png");
        std::fs::write(&src, white_png(300, 150)).unwrap();
        let pdf = tmp("foto.pdf");
        to_pdf(&src, &pdf).unwrap();
        let info = crate::pdfdoc::pages(&pdf).unwrap();
        assert_eq!(info.len(), 1);
        let out = tmp("foto-firmata.png");
        save_with_ink(&src, &out, &ink(false)).unwrap();
        let img = tiny_skia::Pixmap::decode_png(&std::fs::read(&out).unwrap()).unwrap();
        assert_eq!((img.width(), img.height()), (300, 150));
        let red = (70..95).filter_map(|y| img.pixel(150, y)).any(|p| p.red() > 150 && p.green() < 60);
        assert!(red);
        assert_eq!(img.pixel(10, 10).unwrap().green(), 255);
    }

    #[test]
    fn jpeg_firmato_resta_jpeg() {
        let src = tmp("foto.jpg");
        let white = photocraft_codecs::Image::from_u8(200, 100, ChannelLayout::Rgb, vec![255; 200 * 100 * 3]).unwrap();
        std::fs::write(&src, photocraft_codecs::encode(&white, Format::Jpeg, &EncodeOptions::default()).unwrap()).unwrap();
        let out = tmp("foto-firmata.jpg");
        save_with_ink(&src, &out, &ink(true)).unwrap();
        let b = std::fs::read(&out).unwrap();
        assert!(b.starts_with(&[0xFF, 0xD8]));
        let back = photocraft_codecs::decode(&b).unwrap();
        assert_eq!(back.dimensions(), (200, 100));
        assert_eq!(signed_ext("jpeg"), "jpg");
        assert_eq!(signed_ext("gif"), "png");
    }
}
