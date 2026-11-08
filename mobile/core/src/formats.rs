//! «Altri formati» della desktop (`src/formats.rs`), con gli STESSI file:
//! SVG, JPEG e WebP (`craftexport`: VectorCraft e PhotoCraft), Word con
//! l'inchiostro modificabile (`wordsign`, WordCraft), Xournal++, GIF e SVG
//! animati (`pageanim`), LaTeX e Typst dalla trascrizione (`mdconv`).
//!
//! Il video MP4 lo codifica l'app (MediaCodec su Android, AVFoundation su
//! iOS: niente codificatore in più nell'app); da qui vengono i fotogrammi,
//! cioè la pagina a metà scrittura (`anim_page_at`), come `src/video.rs`.

use crate::pagegeom::{Page as DPage, PageGeom, PageStroke as DStroke};
use crate::pages::{Page, PageStroke, StoreError};
use crate::session::{PenPoint, Phase};
use crate::tutor::BoardView;
use anyhow::{anyhow, Result};
use std::path::Path;

/// Un formato d'uscita del menu «Altri formati».
#[derive(uniffi::Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportKind {
    Svg,
    Jpeg,
    Webp,
    Word,
    Xournal,
    Gif,
    SvgAnim,
    Video,
    Latex,
    Typst,
}

/// Secondi in cui la pagina si scrive, poi ferma (come `src/video.rs`).
pub const VIDEO_DRAW_S: f32 = 6.0;
pub const VIDEO_HOLD_S: f32 = 2.0;
pub const VIDEO_FPS: u32 = 30;

#[uniffi::export]
pub fn export_kinds() -> Vec<ExportKind> {
    use ExportKind::*;
    vec![Svg, Jpeg, Webp, Word, Xournal, Gif, SvgAnim, Video, Latex, Typst]
}

#[uniffi::export]
pub fn export_ext(kind: ExportKind) -> String {
    match kind {
        ExportKind::Svg | ExportKind::SvgAnim => "svg",
        ExportKind::Jpeg => "jpg",
        ExportKind::Webp => "webp",
        ExportKind::Word => "docx",
        ExportKind::Xournal => "xopp",
        ExportKind::Gif => "gif",
        ExportKind::Video => "mp4",
        ExportKind::Latex => "tex",
        ExportKind::Typst => "typ",
    }
    .into()
}

#[uniffi::export]
pub fn export_mime(kind: ExportKind) -> String {
    match kind {
        ExportKind::Svg | ExportKind::SvgAnim => "image/svg+xml",
        ExportKind::Jpeg => "image/jpeg",
        ExportKind::Webp => "image/webp",
        ExportKind::Word => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        ExportKind::Xournal => "application/x-xopp",
        ExportKind::Gif => "image/gif",
        ExportKind::Video => "video/mp4",
        ExportKind::Latex => "application/x-tex",
        ExportKind::Typst => "text/plain",
    }
    .into()
}

/// Esporta la trascrizione, non i tratti (serve la nota).
#[uniffi::export]
pub fn export_needs_note(kind: ExportKind) -> bool {
    matches!(kind, ExportKind::Latex | ExportKind::Typst)
}

/// Secondi di scrittura e di pausa, e fotogrammi al secondo del video.
#[uniffi::export]
pub fn video_timing() -> Vec<f32> {
    vec![VIDEO_DRAW_S, VIDEO_HOLD_S, VIDEO_FPS as f32]
}

fn err(e: impl std::fmt::Display) -> StoreError {
    StoreError::Io { reason: e.to_string() }
}

fn view(page: &Page, quarter: u32) -> PageGeom {
    BoardView { quarter, pad_w: page.width, pad_h: page.height }.geom()
}

/// La pagina del telefono come la pagina della desktop (girata come la si
/// vede, coordinate 0..1, prima gli evidenziatori).
pub(crate) fn to_desktop(page: &Page, quarter: u32) -> DPage {
    let g = view(page, quarter);
    let conv = |s: &PageStroke| DStroke {
        color: crate::board::hex_to_rgb(&s.color).unwrap_or([20, 20, 20]),
        width: s.width,
        pts: s.points.iter().map(|p| g.unit(p.x, p.y)).collect(),
        highlight: s.highlight,
    };
    let strokes = page.strokes.iter().filter(|s| s.highlight).chain(page.strokes.iter().filter(|s| !s.highlight)).filter(|s| !s.points.is_empty()).map(conv).collect();
    DPage { w: g.w, h: g.h, strokes, bg: None }
}

/// Il titolo per LaTeX e Typst: il primo «# » della nota, se no la pagina.
fn note_title(md: &str, page: &Page) -> String {
    md.lines()
        .find_map(|l| l.trim().strip_prefix("# ").map(|t| t.trim().to_string()))
        .unwrap_or_else(|| if page.title.trim().is_empty() { page.created.clone() } else { page.title.clone() })
}

/// La pagina in Word: i tratti come inchiostro di Word e, se c'è, la
/// trascrizione nella pagina dopo (come `formats::word` della desktop).
fn word(page: &DPage, note: Option<&str>, out: &Path) -> Result<()> {
    let src = out.with_extension("md");
    let text = note.filter(|n| !n.trim().is_empty());
    std::fs::write(&src, text.unwrap_or(" \n"))?;
    let res = (|| -> Result<()> {
        let mut s = crate::wordsign::open_with(&src, crate::mdcraft::Hand::Stampato)?;
        if text.is_some() {
            for (cmd, args) in [
                ("caret.docStart", serde_json::json!({})),
                ("text.newParagraph", serde_json::json!({})),
                ("para.pageBreakBefore", serde_json::json!({ "value": true })),
            ] {
                s.run(cmd, &args).map_err(|e| anyhow!("WordCraft, {cmd}: {e}"))?;
            }
        }
        let strokes: Vec<crate::wordsign::InkStroke> = page
            .strokes
            .iter()
            .map(|st| crate::wordsign::InkStroke {
                page: 0,
                pts: st.pts.clone(),
                color: st.color,
                width_frac: crate::export::stroke_width(st.width, page.h) / page.h,
                highlight: st.highlight,
            })
            .collect();
        crate::wordsign::add_ink(&mut s, &strokes)?;
        s.run("file.save", &serde_json::json!({ "path": out.to_string_lossy() })).map_err(|e| anyhow!("WordCraft, Word: {e}"))?;
        if !out.exists() {
            return Err(anyhow!("WordCraft non ha scritto il Word"));
        }
        Ok(())
    })();
    let _ = std::fs::remove_file(&src);
    res
}

/// Scrive la pagina in `out` nel formato `kind`. `note` è la trascrizione
/// (Markdown), se c'è; `paper` la rigatura (per lo sfondo di Xournal++).
#[uniffi::export]
pub fn export_page(page: Page, quarter: u32, kind: ExportKind, note: Option<String>, paper: String, out: String) -> Result<(), StoreError> {
    let out = Path::new(&out);
    let p = to_desktop(&page, quarter);
    if p.strokes.is_empty() && !export_needs_note(kind) {
        return Err(err("pagina vuota"));
    }
    let bytes: Vec<u8> = match kind {
        ExportKind::Svg => crate::craftexport::svg(&p).map_err(err)?,
        ExportKind::Jpeg => crate::craftexport::raster(&p, 2000, photocraft_codecs::Format::Jpeg).map_err(err)?,
        ExportKind::Webp => crate::craftexport::raster(&p, 2000, photocraft_codecs::Format::WebP).map_err(err)?,
        ExportKind::Xournal => crate::pageanim::xournal(&p, &paper).map_err(err)?,
        ExportKind::Gif => crate::pageanim::gif(&p).map_err(err)?,
        ExportKind::SvgAnim => crate::pageanim::svg_anim(&p).into_bytes(),
        ExportKind::Word => return word(&p, note.as_deref(), out).map_err(err),
        ExportKind::Latex | ExportKind::Typst => {
            let md = note.filter(|n| !n.trim().is_empty()).ok_or_else(|| err("la pagina non è trascritta"))?;
            let title = note_title(&md, &page);
            let s = if kind == ExportKind::Latex { crate::mdconv::to_latex(&md, &title) } else { crate::mdconv::to_typst(&md, &title) };
            s.into_bytes()
        }
        ExportKind::Video => return Err(err("il video lo scrive l'app")),
    };
    std::fs::write(out, bytes).map_err(err)
}

/// L'immagine da copiare negli appunti, come «Copia immagine» della desktop:
/// ritagliata attorno alla scrittura (margine 3%), lato lungo 1600 px, PNG.
#[uniffi::export]
pub fn copy_png(page: Page, quarter: u32) -> Result<Vec<u8>, StoreError> {
    let p = crate::pagerender::crop_to_content(&to_desktop(&page, quarter), 0.03);
    if p.strokes.is_empty() {
        return Err(err("pagina vuota"));
    }
    crate::pagerender::page_to_png(&p, 1600).map_err(err)
}

/// La pagina al momento `t` (0..1) della scrittura, per i fotogrammi del
/// video: gli stessi tempi della GIF e del video della desktop. I tratti
/// tornano in unità del pad (la si disegna come una pagina qualsiasi).
#[uniffi::export]
pub fn anim_page_at(page: Page, quarter: u32, t: f32) -> Page {
    let g = view(&page, quarter);
    let d = to_desktop(&page, quarter);
    let tl = crate::pageanim::timeline(&d);
    let at = crate::pageanim::page_at(&d, &tl, t);
    let strokes = at
        .strokes
        .iter()
        .map(|s| {
            let n = s.pts.len();
            PageStroke {
                color: crate::board::rgb_to_hex(s.color),
                width: s.width,
                highlight: s.highlight,
                points: s
                    .pts
                    .iter()
                    .enumerate()
                    .map(|(i, &(u, v))| {
                        let (x, y) = g.pad(u, v);
                        PenPoint { x, y, pressure: 1.0, phase: if i == 0 { Phase::Down } else if i + 1 == n { Phase::Up } else { Phase::Move } }
                    })
                    .collect(),
            }
        })
        .collect();
    Page { strokes, ..page }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page() -> Page {
        let line = |y: f32, c: &str, hl: bool| PageStroke {
            color: c.into(),
            width: if hl { 9.0 } else { 2.0 },
            highlight: hl,
            points: (0..20).map(|i| PenPoint { x: 2000.0 + i as f32 * 400.0, y, pressure: 1.0, phase: Phase::Move }).collect(),
        };
        Page {
            id: "20260101-120000".into(),
            title: "Prova".into(),
            created: "2026-01-01 12:00:00".into(),
            source: "lavagna".into(),
            raw: None,
            width: 21600.0,
            height: 14800.0,
            strokes: vec![line(3000.0, "#141414", false), line(5000.0, "#d02020", false), line(5000.0, "#ffe060", true)],
        }
    }

    fn tmp(name: &str) -> String {
        let d = std::env::temp_dir().join(format!("osl-formats-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d.join(name).to_string_lossy().into_owned()
    }

    #[test]
    fn tutti_i_formati() {
        let note = Some("# Titolo\n\nUna riga con $x^2$.\n".to_string());
        for k in export_kinds() {
            if k == ExportKind::Video {
                continue;
            }
            let out = tmp(&format!("p.{}", export_ext(k)));
            export_page(page(), 1, k, note.clone(), "righe".into(), out.clone()).unwrap_or_else(|e| panic!("{k:?}: {e}"));
            let b = std::fs::read(&out).unwrap();
            assert!(b.len() > 20, "{k:?}");
            match k {
                ExportKind::Jpeg => assert_eq!(&b[..2], &[0xff, 0xd8]),
                ExportKind::Webp => assert_eq!(&b[8..12], b"WEBP"),
                ExportKind::Gif => assert_eq!(&b[..3], b"GIF"),
                ExportKind::Word | ExportKind::Xournal => assert!(b[0] == b'P' || b[0] == 0x1f, "{k:?}"),
                ExportKind::Latex => assert!(String::from_utf8_lossy(&b).contains("\\begin{document}")),
                ExportKind::Typst => assert!(String::from_utf8_lossy(&b).contains("Titolo")),
                _ => assert!(String::from_utf8_lossy(&b).contains("<svg")),
            }
        }
        // senza trascrizione: Word solo inchiostro, LaTeX no
        export_page(page(), 0, ExportKind::Word, None, "bianco".into(), tmp("solo.docx")).unwrap();
        assert!(export_page(page(), 0, ExportKind::Latex, None, "bianco".into(), tmp("x.tex")).is_err());
    }

    #[test]
    fn copia_ritagliata() {
        let png = copy_png(page(), 1).unwrap();
        let img = tiny_skia::Pixmap::decode_png(&png).unwrap();
        // la scrittura occupa una striscia: l'immagine non è più un foglio intero
        assert_eq!(img.width().max(img.height()), 1600);
        let (w, h) = (img.width() as f32, img.height() as f32);
        assert!(w / h < 0.6 || h / w < 0.6, "{w}×{h}");
    }

    #[test]
    fn fotogrammi_del_video() {
        let p = page();
        assert!(anim_page_at(p.clone(), 1, 0.0).strokes.len() <= 1);
        let mid: usize = anim_page_at(p.clone(), 1, 0.5).strokes.iter().map(|s| s.points.len()).sum();
        let end = anim_page_at(p.clone(), 1, 1.0);
        let all: usize = end.strokes.iter().map(|s| s.points.len()).sum();
        assert!(mid < all && end.strokes.len() == 3);
        // torna in unità del pad, nello stesso posto
        let a = &p.strokes[0].points[5];
        let b = end.strokes.iter().find(|s| s.color == "#141414").unwrap();
        assert!((b.points[5].x - a.x).abs() < 1.0 && (b.points[5].y - a.y).abs() < 1.0, "{:?} {:?}", b.points[5], a);
    }
}
