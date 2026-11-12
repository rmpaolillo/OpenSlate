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
        "word" => "docx".into(),
        // un testo arrivato dal computer (Markdown, .txt…) diventa PDF, come là
        _ => "pdf".into(),
    }
}

/// Scrive in `dst` il documento firmato, nel suo formato: `pages[i]` sono i
/// tratti della pagina i (unità del pad), `pdf` le pagine mostrate.
#[uniffi::export]
pub fn doc_sign(original: String, pdf: String, dst: String, pages: Vec<Vec<PageStroke>>, view: BoardView) -> Result<(), StoreError> {
    let src = Path::new(&original);
    // un PDF, o un testo arrivato dal computer (Markdown…): si firma il PDF delle pagine
    if kind(src) == "pdf" || kind(src).is_empty() {
        return pdf_sign(if kind(src) == "pdf" { original } else { pdf }, dst, pages, view);
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

// ---------------------------------------------------------------------------
// I documenti nel formato della desktop (`src/docstore.rs`): `doc.json`,
// `source.pdf` e il file di partenza, così si sincronizzano col computer.
// ---------------------------------------------------------------------------

use crate::docstore::{self, DocFmt, InkFmt, PadGeom};

/// Riga dell'elenco dei documenti.
#[derive(uniffi::Record, Clone, Debug)]
pub struct DocEntry {
    pub id: String,
    pub title: String,
    /// "AAAA-MM-GG hh:mm:ss".
    pub created: String,
}

/// Un documento aperto: pagine e tratti già nella geometria di chi guarda.
#[derive(uniffi::Record, Clone, Debug)]
pub struct DocData {
    pub title: String,
    /// Il file da firmare (il file di partenza, o `source.pdf`).
    pub original: String,
    /// Le pagine da mostrare.
    pub pdf: String,
    pub sizes: Vec<PdfPageSize>,
    pub pages: Vec<Vec<PageStroke>>,
    pub current: u32,
}

fn geom(view: BoardView) -> PadGeom {
    PadGeom::new(view.view(), view.pad_w.max(1.0), view.pad_h.max(1.0))
}

fn to_ink(s: &PageStroke) -> InkFmt {
    InkFmt { c: s.color.clone(), w: s.width, p: s.points.iter().map(|p| [p.x, p.y, p.pressure]).collect(), h: s.highlight, t: s.stamp.clone() }
}

fn from_ink(s: InkFmt) -> PageStroke {
    PageStroke {
        color: s.c,
        width: s.w,
        highlight: s.h,
        points: crate::pages::points_with_phases(s.p.iter().map(|q| (q[0], q[1], q[2]))),
        stamp: s.t,
    }
}

/// I documenti in `root` (la cartella `documents`), dal più recente.
#[uniffi::export]
pub fn doc_list(root: String) -> Vec<DocEntry> {
    docstore::list(Path::new(&root)).into_iter().map(|d| DocEntry { id: d.id, title: d.title, created: d.created }).collect()
}

/// Apre il documento nella cartella `dir`, coi tratti portati nella
/// geometria di `view` (anche se è stato firmato sul computer girato in un
/// altro modo).
#[uniffi::export]
pub fn doc_open(dir: String, view: BoardView) -> Result<DocData, StoreError> {
    let d = Path::new(&dir);
    let mut f = docstore::read(d).map_err(err)?;
    let pdf = d.join("source.pdf");
    let sizes = pdf_pages(pdf.to_string_lossy().into_owned())?;
    if let Some(from) = f.pad {
        let s: Vec<(f32, f32)> = sizes.iter().map(|p| (p.width, p.height)).collect();
        docstore::convert(&mut f.pages, &s, from, geom(view));
    }
    let mut pages: Vec<Vec<PageStroke>> = f.pages.into_iter().map(|p| p.into_iter().filter(|s| !s.p.is_empty()).map(from_ink).collect()).collect();
    pages.resize_with(sizes.len(), Vec::new);
    pages.truncate(sizes.len());
    let original = f.original.as_ref().map(|o| d.join(o)).filter(|p| p.exists()).unwrap_or_else(|| pdf.clone());
    Ok(DocData {
        title: f.title,
        original: original.to_string_lossy().into_owned(),
        pdf: pdf.to_string_lossy().into_owned(),
        current: (f.current as u32).min(sizes.len().saturating_sub(1) as u32),
        sizes,
        pages,
    })
}

/// Salva i tratti di tutte le pagine (geometria di `view`) e la pagina vista.
#[uniffi::export]
pub fn doc_save(dir: String, pages: Vec<Vec<PageStroke>>, current: u32, view: BoardView) -> Result<(), StoreError> {
    let d = Path::new(&dir);
    let mut f = docstore::read(d).map_err(err)?;
    f.pages = pages.iter().map(|p| p.iter().filter(|s| !s.points.is_empty()).map(to_ink).collect()).collect();
    f.current = current as usize;
    // l'inchiostro in più (pagine sparite) era nella geometria di prima
    if let Some(from) = f.pad {
        let sizes: Vec<(f32, f32)> = pdf_pages(d.join("source.pdf").to_string_lossy().into_owned())?.iter().map(|p| (p.width, p.height)).collect();
        docstore::convert(&mut f.extra, &sizes, from, geom(view));
    }
    f.pad = Some(geom(view));
    docstore::write(d, &f).map_err(err)
}

/// Importa `src` (copia temporanea del file scelto, chiamato `name`) in
/// `root`: prepara le pagine e ritorna l'id del documento.
#[uniffi::export]
pub fn doc_import(root: String, src: String, name: String) -> Result<String, StoreError> {
    let root = Path::new(&root);
    let named = Path::new(&name);
    let ext = named.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let stem = named.file_stem().and_then(|s| s.to_str()).unwrap_or("documento").to_string();
    let k = kind(&Path::new("x").with_extension(&ext));
    if k.is_empty() {
        return Err(err(format!("formato .{ext} non supportato")));
    }
    let now = chrono::Local::now();
    let id = docstore::new_id(root, &stem, &now);
    let dir = root.join(&id);
    std::fs::create_dir_all(&dir)?;
    let r = (|| -> Result<(), StoreError> {
        let pdf = dir.join("source.pdf");
        let original = if k == "pdf" {
            std::fs::copy(&src, &pdf)?;
            None
        } else {
            let o = format!("source.{ext}");
            std::fs::copy(&src, dir.join(&o))?;
            doc_prepare(dir.join(&o).to_string_lossy().into_owned(), pdf.to_string_lossy().into_owned())?;
            Some(o)
        };
        let n = pdf_pages(pdf.to_string_lossy().into_owned())?.len();
        if n == 0 {
            return Err(err("il documento non ha pagine leggibili"));
        }
        let f = DocFmt {
            version: 1,
            title: stem.clone(),
            created: now.format("%Y-%m-%d %H:%M:%S").to_string(),
            current: 0,
            original,
            pages: vec![Vec::new(); n],
            extra: Vec::new(),
            pad: None,
        };
        docstore::write(&dir, &f).map_err(err)
    })();
    if let Err(e) = r {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(e);
    }
    Ok(id)
}

/// Sposta il documento nel cestino dei documenti.
#[uniffi::export]
pub fn doc_trash(root: String, id: String) -> Result<(), StoreError> {
    docstore::trash(Path::new(&root), &id).map(|_| ()).map_err(err)
}

/// Porta i documenti delle versioni di prima dell'app (`original.<ext>`,
/// `title.txt`, una pagina `pNNNN.json` per pagina) al formato della desktop.
/// Ritorna quanti ne ha convertiti.
#[uniffi::export]
pub fn doc_migrate(root: String, view: BoardView) -> u32 {
    let Ok(rd) = std::fs::read_dir(&root) else { return 0 };
    let mut n = 0;
    for e in rd.flatten() {
        let d = e.path();
        if !d.is_dir() || d.join("doc.json").exists() || !d.join("source.pdf").exists() || e.file_name() == docstore::TRASH {
            continue;
        }
        let Ok(sizes) = pdf_pages(d.join("source.pdf").to_string_lossy().into_owned()) else { continue };
        let title = std::fs::read_to_string(d.join("title.txt")).map(|t| t.trim().to_string()).unwrap_or_default();
        let original = std::fs::read_dir(&d).ok().and_then(|rd| {
            rd.flatten().map(|x| x.file_name().to_string_lossy().into_owned()).find(|n| n.starts_with("original."))
        });
        let original = match original {
            Some(o) if o.ends_with(".pdf") => {
                let _ = std::fs::remove_file(d.join(&o));
                None
            }
            Some(o) => {
                let s = o.replacen("original.", "source.", 1);
                if std::fs::rename(d.join(&o), d.join(&s)).is_err() {
                    continue;
                }
                Some(s)
            }
            None => None,
        };
        let dir = d.to_string_lossy().into_owned();
        let pages: Vec<Vec<InkFmt>> = (0..sizes.len())
            .map(|i| {
                crate::pages::library_load(dir.clone(), format!("p{:04}", i + 1)).map(|p| p.strokes.iter().map(to_ink).collect()).unwrap_or_default()
            })
            .collect();
        // la data: dal nome della cartella (millisecondi) se si può
        let created = e
            .file_name()
            .to_str()
            .and_then(|s| s.parse::<i64>().ok())
            .and_then(chrono::DateTime::from_timestamp_millis)
            .map(|t| t.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
        let f = DocFmt {
            version: 1,
            title: if title.is_empty() { "Documento".into() } else { title },
            created,
            current: 0,
            original,
            pages,
            extra: Vec::new(),
            pad: Some(geom(view)),
        };
        if docstore::write(&d, &f).is_ok() {
            let _ = std::fs::remove_file(d.join("title.txt"));
            for i in 0..sizes.len() {
                let _ = std::fs::remove_file(d.join(format!("p{:04}.json", i + 1)));
            }
            n += 1;
        }
    }
    n
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

    #[test]
    fn documenti_come_la_desktop() {
        let root = std::env::temp_dir().join(format!("openslate-docs-fmt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let src = root.join("in.pdf");
        pdfdoc::image_pdf(&src, 2, 2, &[255; 12], 595.0, 842.0).unwrap();
        let r = root.join("documents").to_string_lossy().into_owned();
        let id = doc_import(r.clone(), src.to_string_lossy().into(), "Contratto.pdf".into()).unwrap();
        let dir = format!("{r}/{id}");
        assert_eq!(doc_list(r.clone())[0].title, "Contratto");
        // firmato col telefono in verticale, riaperto in orizzontale: stessa posizione sulla pagina
        let vert = BoardView { quarter: 1, pad_w: 21600.0, pad_h: 14800.0 };
        let oriz = BoardView { quarter: 0, pad_w: 21600.0, pad_h: 14800.0 };
        let firma = PageStroke {
            color: "#141414".into(),
            width: 2.0,
            highlight: false,
            points: vec![
                PenPoint { x: 9000.0, y: 6000.0, pressure: 0.5, phase: Phase::Down },
                PenPoint { x: 9500.0, y: 6500.0, pressure: 0.5, phase: Phase::Up },
            ],
            stamp: None,
        };
        doc_save(dir.clone(), vec![vec![firma.clone()]], 0, vert).unwrap();
        let on_page = |p: &PenPoint, v: BoardView| {
            let g = v.geom();
            let [rx, ry, rw, rh] = fit_rect(595.0, 842.0, g.w, g.h);
            let (u, w) = g.unit(p.x, p.y);
            (((u - rx) / rw * 1000.0).round(), ((w - ry) / rh * 1000.0).round())
        };
        let d = doc_open(dir.clone(), oriz).unwrap();
        assert_eq!(on_page(&d.pages[0][0].points[0], oriz), on_page(&firma.points[0], vert));
        assert_eq!(d.original, d.pdf);
        assert_eq!(doc_open(dir.clone(), vert).unwrap().pages[0][0].points, firma.points);
        doc_trash(r.clone(), id).unwrap();
        assert!(doc_list(r.clone()).is_empty());

        // un documento delle versioni di prima
        let old = root.join("documents/1731400000000");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::copy(&src, old.join("source.pdf")).unwrap();
        std::fs::copy(&src, old.join("original.pdf")).unwrap();
        std::fs::write(old.join("title.txt"), "Vecchio").unwrap();
        crate::pages::library_save(old.to_string_lossy().into(), crate::pages::Page {
            id: "p0001".into(), title: String::new(), created: "2026-11-01 10:00:00".into(), source: "documento".into(), raw: None, width: 0.0, height: 0.0, strokes: vec![firma.clone()],
        }).unwrap();
        assert_eq!(doc_migrate(r.clone(), vert), 1);
        assert_eq!(doc_list(r.clone())[0].title, "Vecchio");
        assert!(!old.join("title.txt").exists() && !old.join("original.pdf").exists() && !old.join("p0001.json").exists());
        assert_eq!(doc_open(old.to_string_lossy().into(), vert).unwrap().pages[0][0].points, firma.points);
        assert_eq!(doc_migrate(r, vert), 0);
        let _ = std::fs::remove_dir_all(&root);
    }

}
