//! Scrittura a mano → testo.
//!
//! Due motori, scelti in `settings.conf` (`ocr_engine`):
//! - `ai` (predefinito): un modello con visione della catena di `ai.toml`
//!   (un server locale, poi DeepSeek, Gemini, Claude…). Trascrive in
//!   Markdown con formule LaTeX, fedele al foglio, errori compresi.
//! - `vision`: Apple Vision, offline, già incluso in macOS (solo testo, niente
//!   formule). Un piccolo programma Swift viene compilato al primo uso.

use crate::settings::Settings;
use anyhow::{anyhow, bail, Context, Result};
use std::process::Command;
use std::sync::Arc;

const VISION_SRC: &str = include_str!("swift/vision_ocr.swift");

/// Risultato del riconoscimento.
#[derive(Clone, Debug)]
pub struct Recognized {
    pub text: String,
    /// Markdown + LaTeX (motore `ai`) oppure testo semplice (`vision`).
    pub markdown: bool,
    /// Chi ha risposto, es. «LM Studio · qwen3-vl-8b».
    pub by: String,
}

/// Riconosce la scrittura di un'immagine PNG.
pub fn recognize(png: &[u8], cfg: &Settings) -> Result<Recognized> {
    match cfg.ocr_engine.trim() {
        "ai" | "" => {
            let r = crate::ai::transcribe(Arc::new(png.to_vec()))?;
            Ok(Recognized { text: r.text, markdown: true, by: format!("{} · {}", r.provider, r.model) })
        }
        "vision" => Ok(Recognized {
            text: vision(png, &cfg.ocr_langs)?,
            markdown: false,
            by: "Apple Vision".into(),
        }),
        other => bail!("ocr_engine sconosciuto: «{other}» (usa ai oppure vision)"),
    }
}

// ---------------------------------------------------------------------------
// Apple Vision
// ---------------------------------------------------------------------------

fn vision(png: &[u8], langs: &str) -> Result<String> {
    if !cfg!(target_os = "macos") {
        bail!("Apple Vision c'è solo su macOS: usa ocr_engine = ai");
    }
    let bin = crate::swift::helper("vision_ocr", VISION_SRC).map_err(|e| {
        anyhow!("{e}\nIn alternativa usa ocr_engine = ai in settings.conf.")
    })?;
    let img = std::env::temp_dir().join(format!("openslate-ocr-{}.png", std::process::id()));
    std::fs::write(&img, png)?;
    let out = Command::new(&bin)
        .arg(&img)
        .arg(langs)
        .output()
        .context("avvio dell'helper OCR")?;
    let _ = std::fs::remove_file(&img);
    if !out.status.success() {
        bail!("OCR: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motore_sconosciuto() {
        let cfg = Settings { ocr_engine: "boh".into(), ..Settings::default() };
        assert!(recognize(b"x", &cfg).unwrap_err().to_string().contains("ocr_engine"));
    }
}
