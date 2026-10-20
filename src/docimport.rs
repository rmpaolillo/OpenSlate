//! Import dei documenti e pagine dei PDF, solo con le librerie della suite
//! Craft di ArtCraft (nessun programma esterno):
//!
//! - Word (.docx, .doc, .odt, .rtf), Markdown, .txt, .html: WordCraft li apre
//!   e li impagina in PDF (vedi `wordsign`);
//! - SVG: VectorCraft (`vecsign`); foto (PNG, JPEG…): PhotoCraft (`rasign`);
//! - PDF: usato così com'è.
//!
//! Le pagine dei PDF (anteprime e sfondo della lavagna) e il loro testo (per
//! la chat AI) li ricava il motore di PdfCraft; le immagini si scrivono con
//! PhotoCraft.

use anyhow::{anyhow, bail, Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Estensioni accettate (per il dialogo «Apri»).
pub const EXTENSIONS: &[&str] = &[
    "pdf", "doc", "docx", "odt", "rtf", "txt", "md", "markdown", "html", "htm", "svg", "svgz", "png", "jpg", "jpeg", "webp", "tif", "tiff", "bmp", "gif",
];

/// Un PDF da importare così com'è (gli altri formati li aprono i motori Craft).
pub fn to_pdf(input: &Path, out: &Path) -> Result<()> {
    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    if ext != "pdf" {
        bail!("formato non supportato: .{ext}");
    }
    std::fs::copy(input, out).with_context(|| format!("copia di {}", input.display()))?;
    Ok(())
}

fn renderer(pdf: &Path) -> Result<pdfcraft_render::PageRenderer> {
    let bytes = std::fs::read(pdf).with_context(|| format!("PDF non leggibile: {}", pdf.display()))?;
    let r = pdfcraft_render::PageRenderer::new(Arc::new(bytes), pdfcraft_render::RenderConfig::default());
    if r.page_count() == 0 {
        bail!("PDF non leggibile o protetto da password: toglila (es. stampando in PDF) e riprova");
    }
    Ok(r)
}

/// Testo del PDF, una voce per pagina (vuota per le pagine scansionate), in
/// ordine di lettura: lo estrae PdfCraft.
pub fn pdf_text(pdf: &Path) -> Result<Vec<String>> {
    let mut r = renderer(pdf)?;
    let n = r.page_count();
    let mut pages = Vec::with_capacity(n);
    for page in 0..n {
        let out = r.render(pdfcraft_render::RenderRequest {
            page,
            kind: pdfcraft_render::RequestKind::Text,
            tile: None,
            scale: 1.0,
            tag: 0,
            preview: false,
        });
        pages.push(out.text.map(|t| t.plain_text()).unwrap_or_default().trim().to_string());
    }
    Ok(pages)
}

/// Disegna le pagine del PDF in `dir/page-NNNN.png` (lato lungo `long_px`):
/// le disegna PdfCraft, le scrive PhotoCraft. Ritorna i percorsi in ordine.
pub fn render_pages(pdf: &Path, dir: &Path, long_px: u32) -> Result<Vec<PathBuf>> {
    std::fs::create_dir_all(dir)?;
    let sizes = crate::pdfdoc::pages(pdf)?;
    let mut r = renderer(pdf)?;
    let mut out = Vec::new();
    for (page, info) in sizes.iter().enumerate().take(r.page_count()) {
        let (w, h) = info.display_size();
        let scale = long_px as f32 / w.max(h).max(1.0);
        let p = r.render(pdfcraft_render::RenderRequest {
            page,
            kind: pdfcraft_render::RequestKind::Pixels,
            tile: None,
            scale,
            tag: 0,
            preview: false,
        });
        if let Some(e) = p.error {
            bail!("pagina {}: {e}", page + 1);
        }
        let png = rgba_to_png(p.width, p.height, &p.rgba, 72.0 * scale)?;
        let path = dir.join(format!("page-{:04}.png", page + 1));
        std::fs::write(&path, png)?;
        out.push(path);
    }
    if out.is_empty() {
        return Err(anyhow!("nessuna pagina disegnata"));
    }
    Ok(out)
}

/// PNG da pixel RGBA su fondo opaco (premoltiplicati = normali), con PhotoCraft.
fn rgba_to_png(w: u32, h: u32, rgba: &[u8], dpi: f32) -> Result<Vec<u8>> {
    use photocraft_codecs::{ChannelLayout, EncodeOptions, Format, Image, Metadata, SampleType};
    let img = Image::from_u8(w, h, ChannelLayout::Rgba, rgba[..(w * h * 4) as usize].to_vec())
        .map_err(|e| anyhow!("PhotoCraft: {e}"))?
        .convert(ChannelLayout::Rgb, SampleType::U8)
        .with_meta(Metadata { dpi: Some((dpi, dpi)), ..Default::default() });
    photocraft_codecs::encode(&img, Format::Png, &EncodeOptions::default()).map_err(|e| anyhow!("PhotoCraft: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pagine_e_testo_con_pdfcraft() {
        // un PDF fatto da WordCraft, con un testo noto
        let dir = std::env::temp_dir().join(format!("openslate-docimport-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let md = dir.join("prova.md");
        std::fs::write(&md, "# Titolo di prova\n\nCiao dalla suite Craft.\n").unwrap();
        let pdf = dir.join("prova.pdf");
        crate::wordsign::to_pdf(&md, &pdf).unwrap();
        let text = pdf_text(&pdf).unwrap();
        assert!(text[0].contains("Titolo di prova") && text[0].contains("Ciao dalla suite Craft"), "{text:?}");
        let pngs = render_pages(&pdf, &dir.join("pages"), 400).unwrap();
        let img = tiny_skia::Pixmap::decode_png(&std::fs::read(&pngs[0]).unwrap()).unwrap();
        assert_eq!(img.height().max(img.width()), 400);
        // c'è del nero (il testo) su fondo bianco
        assert!(img.pixels().iter().any(|p| p.red() < 80));
        assert_eq!(img.pixel(2, 2).unwrap().red(), 255);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
