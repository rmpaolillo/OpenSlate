//! Documenti importati (PDF, Word, Markdown…) su cui scrivere e firmare.
//!
//! Ogni documento vive in `documents/<id>/`:
//! - `source.pdf`  il documento convertito in PDF (mai modificato)
//! - `pages/page-NNNN.png`  le pagine disegnate, per lo schermo
//! - `doc.json`  titolo e tratti di ogni pagina (coordinate del pad)
//!
//! «Salva PDF firmato» scrive in `exports/` una copia del PDF con i tratti
//! aggiunti sopra in grafica vettoriale.

use crate::library::{strokes_from_fmt, strokes_to_fmt};
use crate::pdfdoc::{self, OverlayStroke, PdfPage};
use crate::scene::{page_dims, PageBg, PageGeom, View};
use crate::stroke::{Drawing, Stroke};
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const DOC_DIR: &str = "documents";
/// Pixel del lato lungo delle immagini delle pagine.
const PAGE_PX: u32 = 2200;
/// Al massimo tanti caratteri di testo nella chat (≈ 30 mila token).
const TEXT_MAX: usize = 100_000;

/// Testo del documento in `dir` (estratto da `source.pdf` la prima volta e
/// conservato in `text.txt`), con «--- Pagina N ---» fra le pagine. Vuoto
/// per un PDF scansionato (solo immagini). Se è troppo lungo si tagliano le
/// ultime pagine, e lo si dice.
pub fn text_of(dir: &Path) -> Result<String> {
    let cache = dir.join("text.txt");
    let pages: Vec<String> = match std::fs::read_to_string(&cache) {
        Ok(t) => t.split('\u{0C}').map(str::to_string).collect(),
        Err(_) => {
            let pages = crate::docimport::pdf_text(&dir.join("source.pdf"))?;
            let _ = std::fs::write(&cache, pages.join("\u{0C}"));
            pages
        }
    };
    Ok(join_pages(&pages, TEXT_MAX))
}

fn join_pages(pages: &[String], max: usize) -> String {
    if pages.iter().all(|p| p.trim().is_empty()) {
        return String::new();
    }
    let mut out = String::new();
    for (i, p) in pages.iter().enumerate() {
        let part = format!("--- Pagina {} ---\n{}\n\n", i + 1, p.trim());
        if out.len() + part.len() > max && i > 0 {
            out.push_str(&format!("[… testo troncato: mancano le pagine {}–{} su {}]\n", i + 1, pages.len(), pages.len()));
            break;
        }
        out.push_str(&part);
    }
    out
}

pub struct DocPage {
    pub pdf: PdfPage,
    pub png: PathBuf,
    pub strokes: Vec<Stroke>,
}

pub struct Document {
    pub id: String,
    pub title: String,
    pub created: String,
    pub dir: PathBuf,
    pub pages: Vec<DocPage>,
    pub current: usize,
    /// Documento Word: il file originale nella cartella del documento (es.
    /// `source.docx`); «Salva» lo riscrive nello stesso formato, firmato.
    pub original: Option<String>,
    /// L'inchiostro di pagine che il documento non ha più (una modifica le
    /// ha tolte): non si butta, torna se le pagine tornano (Annulla).
    pub extra: Vec<Vec<Stroke>>,
    /// La geometria in cui sono scritti i tratti (vedi `docstore`).
    pub pad: Option<PadGeom>,
}

pub use crate::docstore::DocSummary;
use crate::docstore::{DocFmt, PadGeom};

pub use crate::pagegeom::fit_rect;

impl Document {
    /// Importa un file: conversione in PDF e disegno delle pagine.
    pub fn import(input: &Path) -> Result<Document> {
        let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("documento").to_string();
        let now = chrono::Local::now();
        let id = crate::docstore::new_id(Path::new(DOC_DIR), &stem, &now);
        let dir = Path::new(DOC_DIR).join(&id);
        std::fs::create_dir_all(&dir)?;
        let result = (|| -> Result<Document> {
            let pdf = dir.join("source.pdf");
            // Word e testi (Markdown, .txt, .html): pagine impaginate da WordCraft
            // (un Word si firma e si salva in Word; un testo firmato diventa PDF)
            let mut original = None;
            if crate::wordsign::is_word(input) {
                let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("docx").to_ascii_lowercase();
                let name = format!("source.{ext}");
                std::fs::copy(input, dir.join(&name))?;
                // un Markdown si porta dietro le sue figure (stessi nomi, accanto)
                if crate::mdcraft::is_markdown(input) {
                    let base = input.parent().unwrap_or(Path::new("."));
                    let md = std::fs::read_to_string(input).unwrap_or_default();
                    for rel in crate::mdcraft::images(&md, base) {
                        let to = dir.join(&rel);
                        if let Some(p) = to.parent() {
                            let _ = std::fs::create_dir_all(p);
                        }
                        let _ = std::fs::copy(base.join(&rel), to);
                    }
                }
                crate::wordsign::to_pdf(&dir.join(&name), &pdf)?;
                original = Some(name);
            }
            // foto (PNG, JPEG…): PhotoCraft; SVG: VectorCraft (si firma e si salva
            // nel formato dell'immagine)
            if crate::rasign::is_raster(input) {
                let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("png").to_ascii_lowercase();
                let name = format!("source.{ext}");
                std::fs::copy(input, dir.join(&name))?;
                crate::rasign::to_pdf(&dir.join(&name), &pdf)?;
                original = Some(name);
            } else if crate::vecsign::is_image(input) {
                let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("png").to_ascii_lowercase();
                let name = format!("source.{ext}");
                std::fs::copy(input, dir.join(&name))?;
                crate::vecsign::to_pdf(&dir.join(&name), &pdf)?;
                original = Some(name);
            }
            if original.is_none() {
                crate::docimport::to_pdf(input, &pdf)?;
            }
            let mut doc = Document {
                id: id.clone(),
                title: stem.clone(),
                created: now.format("%Y-%m-%d %H:%M:%S").to_string(),
                dir: dir.clone(),
                pages: Vec::new(),
                current: 0,
                original,
                extra: Vec::new(),
                pad: None,
            };
            doc.load_pages(None)?;
            doc.save()?;
            Ok(doc)
        })();
        if result.is_err() {
            let _ = std::fs::remove_dir_all(&dir);
        }
        result
    }

    pub fn source_pdf(&self) -> PathBuf {
        self.dir.join("source.pdf")
    }

    /// Testo di tutto il documento per la chat AI (vedi [`text_of`]).
    pub fn text(&self) -> Result<String> {
        text_of(&self.dir)
    }

    /// Legge le misure delle pagine e (se mancano) le disegna.
    fn load_pages(&mut self, strokes: Option<Vec<Vec<Stroke>>>) -> Result<()> {
        let pdf = self.source_pdf();
        let info = pdfdoc::pages(&pdf)?;
        if info.is_empty() {
            bail!("il documento non ha pagine leggibili");
        }
        let pages_dir = self.dir.join("pages");
        let expected = |i: usize| pages_dir.join(format!("page-{:04}.png", i + 1));
        let mut pngs: Vec<PathBuf> = (0..info.len()).map(expected).collect();
        if !pngs.iter().all(|p| p.exists()) {
            let made = crate::docimport::render_pages(&pdf, &pages_dir, PAGE_PX)?;
            // Nomi uniformi page-NNNN.png.
            for (i, p) in made.iter().enumerate() {
                let want = expected(i);
                if *p != want {
                    std::fs::rename(p, &want)?;
                }
            }
            pngs = (0..info.len().min(made.len())).map(expected).collect();
        }
        // l'inchiostro in più (pagine sparite) resta da parte, non si taglia
        let mut strokes = strokes.unwrap_or_default();
        strokes.append(&mut std::mem::take(&mut self.extra));
        let n = pngs.len();
        let mut extra = if strokes.len() > n { strokes.split_off(n) } else { Vec::new() };
        while extra.last().is_some_and(|p| p.is_empty()) {
            extra.pop();
        }
        self.extra = extra;
        strokes.resize_with(n, Vec::new);
        self.pages = info
            .into_iter()
            .zip(pngs)
            .zip(strokes)
            .map(|((pdf, png), strokes)| DocPage { pdf, png, strokes })
            .collect();
        if self.pages.is_empty() {
            bail!("le pagine del documento non si disegnano");
        }
        self.current = self.current.min(self.pages.len().saturating_sub(1));
        Ok(())
    }

    /// Il documento coi tratti come sono salvati.
    pub fn open(id: &str) -> Result<Document> {
        Self::open_impl(id, None)
    }

    /// Il documento coi tratti portati nella geometria `geom` (rotazione e
    /// misure del pad di chi lo apre: un documento firmato sul telefono girato
    /// in un altro modo torna al suo posto).
    pub fn open_for(id: &str, geom: PadGeom) -> Result<Document> {
        Self::open_impl(id, Some(geom))
    }

    fn open_impl(id: &str, geom: Option<PadGeom>) -> Result<Document> {
        let dir = Path::new(DOC_DIR).join(id);
        let mut f: DocFmt = serde_json::from_str(
            &std::fs::read_to_string(dir.join("doc.json")).with_context(|| format!("documento {id} non trovato"))?,
        )?;
        if let (Some(from), Some(to)) = (f.pad, geom) {
            let sizes: Vec<(f32, f32)> = pdfdoc::pages(&dir.join("source.pdf"))?.iter().map(|p| p.display_size()).collect();
            crate::docstore::convert(&mut f.pages, &sizes, from, to);
            crate::docstore::convert(&mut f.extra, &sizes, from, to);
        }
        let mut doc = Document {
            pad: geom.or(f.pad),
            id: id.to_string(),
            title: f.title,
            created: f.created,
            dir,
            pages: Vec::new(),
            current: f.current,
            original: f.original,
            extra: f.extra.into_iter().map(strokes_from_fmt).collect(),
        };
        doc.load_pages(Some(f.pages.into_iter().map(strokes_from_fmt).collect()))?;
        Ok(doc)
    }

    pub fn save(&self) -> Result<()> {
        let f = DocFmt {
            version: 1,
            title: self.title.clone(),
            created: self.created.clone(),
            current: self.current,
            original: self.original.clone(),
            pages: self.pages.iter().map(|p| strokes_to_fmt(&p.strokes)).collect(),
            extra: self.extra.iter().map(|p| strokes_to_fmt(p)).collect(),
            pad: self.pad,
        };
        let tmp = self.dir.join(".doc.json.tmp");
        std::fs::write(&tmp, serde_json::to_string(&f)?)?;
        std::fs::rename(&tmp, self.dir.join("doc.json"))?;
        Ok(())
    }

    /// Sfondo della pagina `i` per una pagina del pad di misure (pad_w, pad_h)
    /// vista con `view`.
    pub fn page_bg(&self, i: usize, view: View, pad_w: f32, pad_h: f32) -> Option<PageBg> {
        let p = self.pages.get(i)?;
        let png = std::fs::read(&p.png).ok()?;
        let g = PageGeom::new(view, pad_w, pad_h);
        let (dw, dh) = p.pdf.display_size();
        Some(PageBg {
            png: Arc::new(png),
            rect: fit_rect(dw, dh, g.w, g.h),
            key: format!("{}-{}", self.id, i),
        })
    }

    /// Estensione del file firmato: quella del Word o dell'immagine di
    /// partenza (il vecchio .doc diventa .docx), altrimenti pdf.
    pub fn signed_ext(&self) -> String {
        match self.original.as_deref().and_then(|n| Path::new(n).extension()).and_then(|e| e.to_str()) {
            Some(e @ ("docx" | "docm" | "dotx" | "dotm" | "odt" | "rtf" | "svg" | "svgz")) => e.to_string(),
            Some(e @ ("png" | "jpg" | "jpeg" | "webp" | "tif" | "tiff" | "bmp" | "gif")) => crate::rasign::signed_ext(e).to_string(),
            // un testo (Markdown, .txt, .html) non tiene l'inchiostro: diventa PDF
            Some("md" | "markdown" | "txt" | "html" | "htm") => "pdf".into(),
            Some(_) => "docx".into(),
            None => "pdf".into(),
        }
    }

    /// Salva il documento con la scrittura: un Word resta Word (inchiostro nel
    /// documento), un PDF resta PDF (tratti vettoriali sopra l'originale).
    pub fn export_signed(&self, view: View, pad: &Drawing, dst: &Path) -> Result<()> {
        let per_page = self.overlay(view, pad);
        // un testo aperto con WordCraft: le sue pagine (il PDF di WordCraft) con i tratti sopra
        if let Some(orig) = self.original.as_ref().filter(|o| !crate::wordsign::is_text(Path::new(o))) {
            let ink: Vec<crate::wordsign::InkStroke> = per_page
                .into_iter()
                .enumerate()
                .flat_map(|(page, strokes)| {
                    strokes.into_iter().map(move |s| crate::wordsign::InkStroke {
                        page,
                        pts: s.pts,
                        color: s.color,
                        width_frac: s.width_frac,
                        highlight: s.highlight,
                        stamp: s.stamp,
                    })
                })
                .collect();
            let src = self.dir.join(orig);
            if crate::rasign::is_raster(&src) {
                return crate::rasign::save_with_ink(&src, dst, &ink);
            }
            if crate::vecsign::is_image(&src) {
                return crate::vecsign::save_with_ink(&src, dst, &ink);
            }
            return crate::wordsign::save_with_ink(&src, dst, &ink);
        }
        pdfdoc::write_overlay(&self.source_pdf(), dst, &per_page)
    }

    /// Il PDF da stampare: le pagine come si vedono (anche di un Word o di una
    /// foto) con firma, timbri e scrittura sopra.
    pub fn export_print(&self, view: View, pad: &Drawing, dst: &Path) -> Result<()> {
        pdfdoc::write_overlay(&self.source_pdf(), dst, &self.overlay(view, pad))
    }

    /// I tratti di ogni pagina in coordinate della pagina (0..1 dall'alto a sinistra).
    fn overlay(&self, view: View, pad: &Drawing) -> Vec<Vec<OverlayStroke>> {
        let (pw, ph) = page_dims(pad);
        let g = PageGeom::new(view, pw, ph);
        self
            .pages
            .iter()
            .map(|p| {
                let (dw, dh) = p.pdf.display_size();
                let [rx, ry, rw, rh] = fit_rect(dw, dh, g.w, g.h);
                p.strokes
                    .iter()
                    .filter(|s| !s.points.is_empty())
                    .map(|s| OverlayStroke {
                        color: s.color,
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
                        stamp: s.stamp.clone(),
                    })
                    .collect()
            })
            .collect()
    }
}

/// Documenti salvati, dal più recente.
/// Dopo una modifica al file del documento (dalla chat AI): rifà il PDF delle
/// pagine e dimentica pagine disegnate e testo, che si rifanno alla riapertura.
pub fn rebuild(dir: &Path) -> Result<()> {
    let f: DocFmt = serde_json::from_str(&std::fs::read_to_string(dir.join("doc.json"))?)?;
    let pdf = dir.join("source.pdf");
    if let Some(name) = &f.original {
        let src = dir.join(name);
        let tmp = dir.join("source.nuovo.pdf");
        if crate::wordsign::is_word(&src) {
            crate::wordsign::to_pdf(&src, &tmp)?;
        } else if crate::rasign::is_raster(&src) {
            crate::rasign::to_pdf(&src, &tmp)?;
        } else if crate::vecsign::is_image(&src) {
            crate::vecsign::to_pdf(&src, &tmp)?;
        }
        if tmp.exists() {
            std::fs::rename(&tmp, &pdf)?;
        }
    }
    let _ = std::fs::remove_dir_all(dir.join("pages"));
    let _ = std::fs::remove_file(dir.join("text.txt"));
    Ok(())
}

pub fn list() -> Vec<DocSummary> {
    crate::docstore::list(Path::new(DOC_DIR))
}

/// Sposta il documento nel cestino dei documenti (`documents/cestino`).
pub fn trash(id: &str) -> Result<()> {
    crate::docstore::trash(Path::new(DOC_DIR), id)?;
    Ok(())
}

/// La geometria dei tratti per chi guarda con `view` un pad come `pad`.
pub fn geom_for(view: View, pad: &Drawing) -> PadGeom {
    let (w, h) = page_dims(pad);
    PadGeom::new(view, w, h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn testo_del_documento() {
        let p = vec!["Titolo".to_string(), "".into(), "Fine".into()];
        let t = join_pages(&p, 10_000);
        assert!(t.starts_with("--- Pagina 1 ---\nTitolo") && t.contains("--- Pagina 3 ---\nFine"));
        assert!(join_pages(&["".into(), " ".into()], 100).is_empty());
        let lunghe: Vec<String> = (0..10).map(|_| "x".repeat(50)).collect();
        let t = join_pages(&lunghe, 200);
        assert!(t.contains("troncato") && t.contains("su 10"));
    }

    #[test]
    fn adattamento() {
        // A4 verticale dentro una pagina del pad un po' più larga
        let r = fit_rect(595.0, 842.0, 21600.0, 29700.0);
        assert!((r[3] - 1.0).abs() < 1e-4 && r[2] < 1.0 && (r[0] - (1.0 - r[2]) / 2.0).abs() < 1e-6);
        // pagina orizzontale: bande sopra e sotto
        let r = fit_rect(842.0, 595.0, 21600.0, 29700.0);
        assert!((r[2] - 1.0).abs() < 1e-4 && r[3] < 1.0);
    }
}

