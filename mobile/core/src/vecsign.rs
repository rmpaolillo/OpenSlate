//! Le immagini SVG sul telefono, senza VectorCraft. Il motore intero (effetti,
//! plugin, CAD, EPS e la sua interfaccia) pesava da solo metà dell'app.
//! Stesse funzioni di `src/vecsign.rs` della desktop:
//! - le pagine per la lavagna: l'SVG disegnato da resvg, in un PDF di una pagina;
//! - il salvataggio firmato: un SVG resta SVG, il file com'era più un gruppo
//!   con la scrittura del pennino come tracciati vettoriali (e i timbri come
//!   immagini), come fa VectorCraft sulla desktop.

use crate::wordsign::InkStroke;
use anyhow::{anyhow, bail, Result};
use std::fmt::Write as _;
use std::io::{Read, Write};
use std::path::Path;

/// Estensioni aperte come immagini vettoriali.
pub fn is_image(path: &Path) -> bool {
    matches!(ext(path).as_str(), "svg" | "svgz")
}

fn ext(path: &Path) -> String {
    path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase()
}

/// Il testo dell'SVG (un .svgz si scompatta).
fn read_svg(path: &Path) -> Result<Vec<u8>> {
    let bytes = std::fs::read(path)?;
    if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut out = Vec::new();
        flate2::read::GzDecoder::new(bytes.as_slice()).read_to_end(&mut out)?;
        return Ok(out);
    }
    Ok(bytes)
}

/// I caratteri del telefono, per i testi dentro l'SVG.
fn options() -> usvg::Options<'static> {
    let mut opt = usvg::Options::default();
    let db = opt.fontdb_mut();
    for dir in ["/system/fonts", "/System/Library/Fonts"] {
        if Path::new(dir).is_dir() {
            db.load_fonts_dir(dir);
        }
    }
    opt
}

fn tree(data: &[u8]) -> Result<usvg::Tree> {
    usvg::Tree::from_data(data, &options()).map_err(|e| anyhow!("l'SVG non si apre: {e}"))
}

/// Pixel del lato lungo per le pagine della lavagna.
const LONG_SIDE: f32 = 2400.0;

/// L'immagine come PDF di una pagina, per le pagine della lavagna.
pub fn to_pdf(input: &Path, out: &Path) -> Result<()> {
    let t = tree(&read_svg(input)?)?;
    let size = t.size();
    let (sw, sh) = (size.width(), size.height());
    let k = LONG_SIDE / sw.max(sh);
    let (w, h) = ((sw * k).round().max(1.0) as u32, (sh * k).round().max(1.0) as u32);
    let mut pm = resvg::tiny_skia::Pixmap::new(w, h).ok_or_else(|| anyhow!("SVG vuoto"))?;
    // sopra il bianco, come la carta: i pixel sono opachi
    pm.fill(resvg::tiny_skia::Color::WHITE);
    resvg::render(&t, resvg::tiny_skia::Transform::from_scale(w as f32 / sw, h as f32 / sh), &mut pm.as_mut());
    let rgb: Vec<u8> = pm.data().chunks_exact(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
    // le misure dell'SVG sono pixel CSS (96 per pollice): in punti
    crate::pdfdoc::image_pdf(out, w, h, &rgb, sw as f64 * 0.75, sh as f64 * 0.75)
}

/// Da frazioni della pagina (come la mostra la lavagna) alle coordinate
/// dell'SVG: la `viewBox` dentro la finestra, centrata e intera (il modo
/// normale di `preserveAspectRatio`).
struct Frame {
    w: f64,
    h: f64,
    /// coordinate SVG = (pixel - t) / s
    s: f64,
    tx: f64,
    ty: f64,
}

impl Frame {
    fn new(data: &[u8], t: &usvg::Tree) -> Frame {
        let (w, h) = (t.size().width() as f64, t.size().height() as f64);
        let vb = std::str::from_utf8(data)
            .ok()
            .and_then(|s| roxmltree::Document::parse(s).ok())
            .and_then(|d| d.root_element().attribute("viewBox").map(String::from))
            .map(|v| v.split([' ', ',']).filter(|p| !p.is_empty()).filter_map(|p| p.parse::<f64>().ok()).collect::<Vec<_>>())
            .filter(|v| v.len() == 4 && v[2] > 0.0 && v[3] > 0.0);
        match vb {
            Some(v) => {
                let s = (w / v[2]).min(h / v[3]);
                Frame { w, h, s, tx: (w - v[2] * s) / 2.0 - v[0] * s, ty: (h - v[3] * s) / 2.0 - v[1] * s }
            }
            None => Frame { w, h, s: 1.0, tx: 0.0, ty: 0.0 },
        }
    }

    fn at(&self, u: f32, v: f32) -> (f64, f64) {
        ((u.clamp(0.0, 1.0) as f64 * self.w - self.tx) / self.s, (v.clamp(0.0, 1.0) as f64 * self.h - self.ty) / self.s)
    }

    /// Una lunghezza in frazioni dell'altezza della pagina.
    fn len(&self, frac: f32) -> f64 {
        frac as f64 * self.h / self.s
    }
}

/// I tratti come elementi SVG: prima le evidenziature, poi i timbri, poi l'inchiostro.
fn ink_svg(f: &Frame, strokes: &[InkStroke]) -> String {
    use base64::Engine;
    let mut g = String::from("\n<g id=\"openslate-firma\">\n");
    let ordered = strokes
        .iter()
        .filter(|s| s.highlight && s.stamp.is_none())
        .chain(strokes.iter().filter(|s| s.stamp.is_some()))
        .chain(strokes.iter().filter(|s| !s.highlight && s.stamp.is_none()));
    for st in ordered.filter(|s| s.page == 0) {
        if let Some(spec) = &st.stamp {
            let Some((r, quarter)) = crate::stamp::placement_uv(&st.pts) else { continue };
            let ((x0, y0), (x1, y1)) = (f.at(r[0], r[1]), f.at(r[2], r[3]));
            // a 300 DPI sulla pagina (la pagina in punti: pixel CSS × 0,75)
            let px_w = ((r[2] - r[0]) as f64 * f.w * 0.75 / 72.0 * 300.0) as f32;
            let png = crate::stamp::render_fit(spec, quarter, px_w).png();
            let _ = writeln!(
                g,
                "<image x=\"{x0:.2}\" y=\"{y0:.2}\" width=\"{:.2}\" height=\"{:.2}\" preserveAspectRatio=\"none\" style=\"mix-blend-mode:multiply\" href=\"data:image/png;base64,{}\"/>",
                x1 - x0,
                y1 - y0,
                base64::engine::general_purpose::STANDARD.encode(png)
            );
            continue;
        }
        let mut pts: Vec<(f64, f64)> = st.pts.iter().take(5000).map(|&(u, v)| f.at(u, v)).collect();
        let Some(&first) = pts.first() else { continue };
        // un punto solo (un puntino): un trattino brevissimo
        if pts.len() == 1 {
            pts.push((first.0 + 0.2 / f.s, first.1));
        }
        let mut d = String::new();
        for (i, (x, y)) in pts.iter().enumerate() {
            let _ = write!(d, "{}{x:.2} {y:.2}", if i == 0 { "M" } else { " L" });
        }
        let [r, gr, b] = st.color;
        let width = f.len(st.width_frac).max(0.25 / f.s);
        let opacity = if st.highlight { " stroke-opacity=\"0.4\"" } else { "" };
        let _ = writeln!(
            g,
            "<path d=\"{d}\" fill=\"none\" stroke=\"#{r:02x}{gr:02x}{b:02x}\" stroke-width=\"{width:.3}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"{opacity}/>"
        );
    }
    g.push_str("</g>\n");
    g
}

/// Copia `src` in `dst` (stesso formato, SVG o SVGZ) con i tratti.
pub fn save_with_ink(src: &Path, dst: &Path, strokes: &[InkStroke]) -> Result<()> {
    let data = read_svg(src)?;
    let t = tree(&data)?;
    let text = String::from_utf8(data.clone()).map_err(|_| anyhow!("l'SVG non è testo UTF-8"))?;
    // il gruppo va dentro <svg>, prima del suo tag di chiusura (l'ultimo del file)
    let end = text.rfind("</").filter(|&i| text[i..].trim_start_matches("</").trim_start().split('>').next().is_some_and(|n| n.trim().ends_with("svg")));
    let Some(end) = end else { bail!("SVG senza </svg>") };
    let mut out = String::with_capacity(text.len() + 4096);
    out.push_str(&text[..end]);
    out.push_str(&ink_svg(&Frame::new(&data, &t), strokes));
    out.push_str(&text[end..]);
    if ext(dst) == "svgz" {
        let mut gz = flate2::write::GzEncoder::new(std::fs::File::create(dst)?, flate2::Compression::default());
        gz.write_all(out.as_bytes())?;
        gz.finish()?;
    } else {
        std::fs::write(dst, out)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("openslate-mvec-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d.join(name)
    }

    fn ink() -> Vec<InkStroke> {
        vec![InkStroke { page: 0, pts: vec![(0.2, 0.5), (0.5, 0.6), (0.8, 0.5)], color: [200, 0, 0], width_frac: 0.02, highlight: false, stamp: None }]
    }

    const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100" viewBox="0 0 400 200"><rect x="10" y="10" width="50" height="30" fill="#00f"/></svg>"##;

    #[test]
    fn pagina_e_firma() {
        let src = tmp("prova.svg");
        std::fs::write(&src, SVG).unwrap();
        let pdf = tmp("prova.pdf");
        to_pdf(&src, &pdf).unwrap();
        let b = crate::pdfdoc::pages(&pdf).unwrap()[0].bbox;
        // 200 × 100 pixel CSS = 150 × 75 punti
        assert!((b[2] - b[0] - 150.0).abs() < 0.5 && (b[3] - b[1] - 75.0).abs() < 0.5, "{b:?}");
        let out = tmp("prova-firmato.svg");
        save_with_ink(&src, &out, &ink()).unwrap();
        let svg = std::fs::read_to_string(&out).unwrap();
        assert!(svg.starts_with("<svg") && svg.trim_end().ends_with("</svg>") && svg.contains("<rect"), "{svg}");
        // nelle coordinate della viewBox (il doppio della finestra): 20% di 400, metà di 200
        assert!(svg.contains("M80.00 100.00") && svg.contains("#c80000"), "{svg}");
        // si riapre, e la firma si vede: rosso a metà del tratto
        let t = tree(svg.as_bytes()).unwrap();
        let mut pm = resvg::tiny_skia::Pixmap::new(200, 100).unwrap();
        resvg::render(&t, resvg::tiny_skia::Transform::default(), &mut pm.as_mut());
        let p = pm.pixel(100, 60).unwrap();
        assert!(p.red() > 150 && p.green() < 60, "{p:?}");
    }

    #[test]
    fn svgz_resta_svgz() {
        let src = tmp("prova.svgz");
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gz.write_all(SVG.as_bytes()).unwrap();
        std::fs::write(&src, gz.finish().unwrap()).unwrap();
        let out = tmp("prova-firmato.svgz");
        save_with_ink(&src, &out, &ink()).unwrap();
        let back = String::from_utf8(read_svg(&out).unwrap()).unwrap();
        assert!(back.contains("openslate-firma"));
    }

    #[test]
    fn col_timbro() {
        let src = tmp("timbro.svg");
        std::fs::write(&src, SVG).unwrap();
        let spec = crate::stamp::StampSpec::default();
        let st = InkStroke {
            page: 0,
            pts: vec![(0.1, 0.2), (0.6, 0.2), (0.6, 0.8), (0.1, 0.8)],
            color: [0, 0, 0],
            width_frac: 0.0,
            highlight: false,
            stamp: Some(std::sync::Arc::new(spec)),
        };
        let out = tmp("timbro-firmato.svg");
        save_with_ink(&src, &out, &[st]).unwrap();
        let svg = std::fs::read_to_string(&out).unwrap();
        assert!(svg.contains("<image") && svg.contains("data:image/png"), "{}", &svg[..svg.len().min(600)]);
    }
}
