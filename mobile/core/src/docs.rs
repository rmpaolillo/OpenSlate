//! Documenti da firmare col pennino, come la desktop (`src/document.rs`): la
//! pagina sta sul foglio del pad adattata e centrata (`pagegeom::fit_rect`),
//! mostrata da un PDF fatto apposta (`source.pdf`); salvando firmato il
//! documento resta nel suo formato, con gli stessi file della desktop:
//! - PDF: l'originale con i tratti sopra in grafica vettoriale (`pdfdoc`);
//! - Word (.docx, .odt, .rtf…): la scrittura come inchiostro di Word (`wordsign`, WordCraft);
//! - foto (PNG, JPEG, WebP…): la scrittura dipinta sopra (`rasign`, PhotoCraft);
//! - SVG: la scrittura come tracciati (`vecsign`, VectorCraft).

use crate::pagegeom::{fit_rect, PageGeom};
use crate::pages::{PageStroke, StoreError};
use crate::pdfdoc::{self, OverlayStroke};
use crate::tutor::BoardView;
use std::path::Path;

fn err(e: impl std::fmt::Display) -> StoreError {
    StoreError::Io { reason: e.to_string() }
}

/// Una pagina del PDF come la si vede (punti, rotazione applicata).
#[derive(uniffi::Record, Clone, Copy, Debug)]
pub struct PdfPageSize {
    pub width: f32,
    pub height: f32,
}

/// Le pagine del PDF.
#[uniffi::export]
pub fn pdf_pages(path: String) -> Result<Vec<PdfPageSize>, StoreError> {
    let pages = pdfdoc::pages(Path::new(&path)).map_err(err)?;
    Ok(pages
        .iter()
        .map(|p| {
            let (width, height) = p.display_size();
            PdfPageSize { width, height }
        })
        .collect())
}

/// Dove sta la pagina del documento sul foglio come lo si vede (u, v,
/// larghezza, altezza in 0..1), come `fit_rect` della desktop.
#[uniffi::export]
pub fn doc_page_rect(page: PdfPageSize, view: BoardView) -> Vec<f32> {
    let g = view.geom();
    fit_rect(page.width, page.height, g.w, g.h).to_vec()
}

/// I tratti di una pagina (unità del pad) → coordinate della pagina del
/// documento (come `Document::overlay` della desktop).
fn overlay(page: PdfPageSize, strokes: &[PageStroke], view: BoardView) -> Vec<OverlayStroke> {
    let g: PageGeom = view.geom();
    let [rx, ry, rw, rh] = fit_rect(page.width, page.height, g.w, g.h);
    strokes
        .iter()
        .filter(|s| !s.points.is_empty())
        .map(|s| OverlayStroke {
            color: crate::board::hex_to_rgb(&s.color).unwrap_or([20, 20, 20]),
            // spessore: frazione dell'altezza della pagina del pad → del documento
            width_frac: s.width / 800.0 / rh,
            pts: s
                .points
                .iter()
                .map(|pt| {
                    let (u, v) = g.unit(pt.x, pt.y);
                    ((u - rx) / rw, (v - ry) / rh)
                })
                .collect(),
            highlight: s.highlight,
            stamp: s.stamp.clone().map(std::sync::Arc::new),
        })
        .collect()
}

/// Scrive in `dst` il PDF `src` con i tratti di ogni pagina sopra
/// (`pages[i]` = i tratti della pagina i, unità del pad).
#[uniffi::export]
pub fn pdf_sign(src: String, dst: String, pages: Vec<Vec<PageStroke>>, view: BoardView) -> Result<(), StoreError> {
    let sizes = pdf_pages(src.clone())?;
    let per_page: Vec<Vec<OverlayStroke>> = sizes
        .iter()
        .enumerate()
        .map(|(i, &p)| pages.get(i).map(|s| overlay(p, s, view)).unwrap_or_default())
        .collect();
    pdfdoc::write_overlay(Path::new(&src), Path::new(&dst), &per_page).map_err(err)
}

/// Che cosa è il documento, dall'estensione.
fn kind(path: &Path) -> &'static str {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    if ext == "pdf" {
        "pdf"
    } else if crate::rasign::is_raster(path) {
        "foto"
    } else if crate::vecsign::is_image(path) {
        "svg"
    } else if crate::wordsign::is_word(path) && !crate::wordsign::is_text(path) {
        "word"
    } else {
        ""
    }
}

/// Le estensioni che si aprono (per il selettore dei file).
#[uniffi::export]
pub fn doc_extensions() -> Vec<String> {
    ["pdf", "docx", "doc", "odt", "rtf", "png", "jpg", "jpeg", "webp", "tif", "tiff", "bmp", "gif", "svg"].map(String::from).to_vec()
}

/// Prepara `original` per la lavagna: scrive `pdf`, le pagine da mostrare
/// (per un PDF una copia, per gli altri quello che fa la loro libreria,
/// come la desktop). Errore se il formato non si apre.
#[uniffi::export]
pub fn doc_prepare(original: String, pdf: String) -> Result<Vec<PdfPageSize>, StoreError> {
    let (src, out) = (Path::new(&original), Path::new(&pdf));
    match kind(src) {
        "pdf" => std::fs::copy(src, out).map(|_| ()).map_err(err)?,
        "word" => crate::wordsign::to_pdf(src, out).map_err(err)?,
        "foto" => crate::rasign::to_pdf(src, out).map_err(err)?,
        "svg" => crate::vecsign::to_pdf(src, out).map_err(err)?,
        _ => return Err(err("formato non supportato")),
    }
    pdf_pages(pdf)
}

/// Estensione del file firmato (come `Document::signed_ext` della desktop):
/// un Word resta Word (il vecchio .doc diventa .docx), una foto la sua o PNG.
#[uniffi::export]
pub fn doc_signed_ext(original: String) -> String {
    let p = Path::new(&original);
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    match kind(p) {
        "pdf" => "pdf".into(),
        "foto" => crate::rasign::signed_ext(&ext).into(),
        "svg" => ext,
        "word" if matches!(ext.as_str(), "docx" | "docm" | "dotx" | "dotm" | "odt" | "rtf") => ext,
        _ => "docx".into(),
    }
}

/// Scrive in `dst` il documento firmato, nel suo formato: `pages[i]` sono i
/// tratti della pagina i (unità del pad), `pdf` le pagine mostrate.
#[uniffi::export]
pub fn doc_sign(original: String, pdf: String, dst: String, pages: Vec<Vec<PageStroke>>, view: BoardView) -> Result<(), StoreError> {
    let src = Path::new(&original);
    if kind(src) == "pdf" {
        return pdf_sign(original, dst, pages, view);
    }
    let sizes = pdf_pages(pdf)?;
    let ink: Vec<crate::wordsign::InkStroke> = sizes
        .iter()
        .enumerate()
        .flat_map(|(page, &p)| {
            pages.get(page).map(|s| overlay(p, s, view)).unwrap_or_default().into_iter().map(move |s| crate::wordsign::InkStroke {
                page,
                pts: s.pts,
                color: s.color,
                width_frac: s.width_frac,
                highlight: s.highlight,
                stamp: s.stamp,
            })
        })
        .collect();
    let dst = Path::new(&dst);
    match kind(src) {
        "word" => crate::wordsign::save_with_ink(src, dst, &ink),
        "foto" => crate::rasign::save_with_ink(src, dst, &ink),
        "svg" => crate::vecsign::save_with_ink(src, dst, &ink),
        _ => return Err(err("formato non supportato")),
    }
    .map_err(err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{PenPoint, Phase};

    #[test]
    fn firma_un_pdf() {
        let dir = std::env::temp_dir().join(format!("openslate-docs-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // un PDF di una pagina A4 vuota, fatto con lo stesso pdfdoc
        let src = dir.join("a.pdf");
        pdfdoc::image_pdf(&src, 2, 2, &[255; 12], 595.0, 842.0).unwrap();
        let sizes = pdf_pages(src.to_string_lossy().into()).unwrap();
        assert_eq!(sizes.len(), 1);
        assert!((sizes[0].width - 595.0).abs() < 1.0);
        let view = BoardView { quarter: 1, pad_w: 21600.0, pad_h: 14800.0 };
        let r = doc_page_rect(sizes[0], view);
        assert!(r[2] <= 1.0 && r[3] <= 1.0 && (r[2] - 1.0).abs() < 1e-3 || (r[3] - 1.0).abs() < 1e-3);
        let firma = PageStroke {
            color: "#1428a0".into(),
            width: 2.0,
            highlight: false,
            points: vec![
                PenPoint { x: 5000.0, y: 5000.0, pressure: 0.5, phase: Phase::Down },
                PenPoint { x: 6000.0, y: 5500.0, pressure: 0.5, phase: Phase::Up },
            ],
            stamp: None,
        };
        let dst = dir.join("firmato.pdf");
        pdf_sign(src.to_string_lossy().into(), dst.to_string_lossy().into(), vec![vec![firma]], view).unwrap();
        let out = std::fs::read(&dst).unwrap();
        assert!(out.starts_with(b"%PDF") && out.len() > std::fs::read(&src).unwrap().len());
        assert_eq!(pdf_pages(dst.to_string_lossy().into()).unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    fn firma() -> PageStroke {
        PageStroke {
            color: "#1428a0".into(),
            width: 2.0,
            highlight: false,
            points: (0..20)
                .map(|i| PenPoint {
                    x: 6000.0 + i as f32 * 200.0,
                    y: 7000.0 + (i % 2) as f32 * 300.0,
                    pressure: 0.5,
                    phase: if i == 0 { Phase::Down } else if i == 19 { Phase::Up } else { Phase::Move },
                })
                .collect(),
            stamp: None,
        }
    }

    #[test]
    fn firma_una_foto_e_resta_foto() {
        let dir = std::env::temp_dir().join(format!("openslate-foto-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // una foto bianca 400 x 300 fatta con PhotoCraft
        use photocraft_codecs::{ChannelLayout, EncodeOptions, Format, Image};
        let img = Image::from_u8(400, 300, ChannelLayout::Rgb, vec![255; 400 * 300 * 3]).unwrap();
        let src = dir.join("foto.png");
        std::fs::write(&src, photocraft_codecs::encode(&img, Format::Png, &EncodeOptions::default()).unwrap()).unwrap();
        let s = |p: &std::path::Path| p.to_string_lossy().into_owned();
        let pdf = dir.join("source.pdf");
        let pages = doc_prepare(s(&src), s(&pdf)).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(doc_signed_ext(s(&src)), "png");
        let dst = dir.join("firmata.png");
        let view = BoardView { quarter: 0, pad_w: 21600.0, pad_h: 14800.0 };
        doc_sign(s(&src), s(&pdf), s(&dst), vec![vec![firma()]], view).unwrap();
        let out = photocraft_codecs::decode(&std::fs::read(&dst).unwrap()).unwrap();
        assert_eq!((out.width(), out.height()), (400, 300));
        // ci sono pixel scuri: la firma è dipinta
        let px = out.convert(ChannelLayout::Rgb, photocraft_codecs::SampleType::U8);
        assert!(px.as_u8().unwrap().chunks(3).any(|c| c[2] as i32 > c[0] as i32 + 60), "firma blu non trovata");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn firma_un_word_e_resta_word() {
        let dir = std::env::temp_dir().join(format!("openslate-word-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let s = |p: &std::path::Path| p.to_string_lossy().into_owned();
        // un .docx fatto da WordCraft, con una riga «Firma:»
        let src = dir.join("contratto.docx");
        let mut w = wordcraft_engine::Session::new(wordcraft_engine::doc::Document::new());
        w.run("text.insert", &serde_json::json!({ "text": "Firma: ____________" })).unwrap();
        w.run("file.save", &serde_json::json!({ "path": s(&src) })).unwrap();
        let pdf = dir.join("source.pdf");
        assert!(!doc_prepare(s(&src), s(&pdf)).unwrap().is_empty());
        assert_eq!(doc_signed_ext(s(&src)), "docx");
        let dst = dir.join("firmato.docx");
        let view = BoardView { quarter: 1, pad_w: 21600.0, pad_h: 14800.0 };
        doc_sign(s(&src), s(&pdf), s(&dst), vec![vec![firma()]], view).unwrap();
        // l'inchiostro è dentro il documento Word
        let bytes = std::fs::read(&dst).unwrap();
        assert!(bytes.starts_with(b"PK"));
        let zip = String::from_utf8_lossy(&bytes);
        assert!(zip.contains("word/document.xml"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
