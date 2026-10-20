//! Documenti Word (.docx, .doc, .odt, .rtf) col motore di WordCraft
//! (https://github.com/storytold/wordcraft, MIT OR Apache-2.0, © ArtCraft Team),
//! senza la sua interfaccia: le pagine per la lavagna (un PDF fatto da WordCraft)
//! e il salvataggio firmato, che resta un .docx con la scrittura del pennino
//! dentro come inchiostro (Word, LibreOffice e WordCraft la mostrano).

use crate::stroke::Rgb;
use anyhow::{anyhow, bail, Result};
use serde_json::json;
use std::path::Path;
use wordcraft_engine::Session;

/// Estensioni aperte con WordCraft.
pub fn is_word(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    matches!(ext.as_str(), "docx" | "doc" | "docm" | "dotx" | "dotm" | "dot" | "odt" | "rtf")
}

/// Apre un documento Word col motore di WordCraft.
pub fn open(path: &Path) -> Result<Session> {
    let bytes = std::fs::read(path).map_err(|e| anyhow!("{}: {e}", path.display()))?;
    let mut s = Session::new(wordcraft_engine::doc::Document::new());
    s.run(
        "file.open",
        &json!({ "path": path.to_string_lossy(), "data": wordcraft_engine::cmd::insert::base64_encode(&bytes) }),
    )
    .map_err(|e| anyhow!("WordCraft non apre il documento: {e}"))?;
    Ok(s)
}

/// Il documento come PDF, impaginato da WordCraft (per le pagine della lavagna).
pub fn to_pdf(input: &Path, out: &Path) -> Result<()> {
    let mut s = open(input)?;
    s.run("file.exportPdf", &json!({ "path": out.to_string_lossy() })).map_err(|e| anyhow!("WordCraft, PDF: {e}"))?;
    if !out.exists() {
        bail!("WordCraft non ha scritto il PDF");
    }
    Ok(())
}

/// Un tratto da mettere nel documento: pagina, punti in 0..1 sulla pagina
/// (dall'angolo in alto a sinistra), spessore come frazione dell'altezza.
pub struct InkStroke {
    pub page: usize,
    pub pts: Vec<(f32, f32)>,
    pub color: Rgb,
    pub width_frac: f32,
    pub highlight: bool,
}

/// Copia `src` in `dst` (stesso formato Word) con i tratti come inchiostro.
pub fn save_with_ink(src: &Path, dst: &Path, strokes: &[InkStroke]) -> Result<()> {
    let mut s = open(src)?;
    let layout = s.layout();
    for st in strokes {
        let Some(page) = layout.pages.get(st.page) else { continue };
        let points: Vec<[f32; 2]> = st.pts.iter().take(5000).map(|(u, v)| [u.clamp(0.0, 1.0) * page.w, v.clamp(0.0, 1.0) * page.h]).collect();
        if points.is_empty() {
            continue;
        }
        let color = format!("{:02X}{:02X}{:02X}", st.color[0], st.color[1], st.color[2]);
        let width = (st.width_frac * page.h).clamp(0.25, 72.0);
        s.run(
            "draw.stroke",
            &json!({ "page": st.page, "points": points, "tool": if st.highlight { "highlighter" } else { "pen" }, "color": color, "width": width }),
        )
        .map_err(|e| anyhow!("inchiostro sulla pagina {}: {e}", st.page + 1))?;
    }
    s.run("file.save", &json!({ "path": dst.to_string_lossy() })).map_err(|e| anyhow!("WordCraft, salvataggio: {e}"))?;
    if !dst.exists() {
        bail!("WordCraft non ha scritto {}", dst.display());
    }
    Ok(())
}
