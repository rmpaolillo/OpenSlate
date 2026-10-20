//! Piccoli programmi Swift per usare i servizi di macOS (Vision, PDFKit).
//! Vengono compilati al primo uso con `swiftc` (Xcode Command Line Tools) e
//! ricompilati solo se il sorgente cambia.

use anyhow::{anyhow, bail, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

const HELPERS_DIR: &str = "target/swift-helpers";

/// Ritorna l'eseguibile dell'helper `name`, compilandolo se serve.
pub fn helper(name: &str, source: &str) -> Result<PathBuf> {
    if !cfg!(target_os = "macos") {
        bail!("questa funzione usa i servizi di macOS");
    }
    // l'app per macOS li porta già compilati (non serve swiftc)
    if let Some(bin) = crate::platform::bundled_helper(name) {
        return Ok(bin);
    }
    let dir = Path::new(HELPERS_DIR).join(name);
    std::fs::create_dir_all(&dir)?;
    // Un solo file chiamato main.swift: swiftc accetta il codice "di primo livello".
    let src = dir.join("main.swift");
    let bin = dir.join(name);
    if bin.exists() && std::fs::read_to_string(&src).ok().as_deref() == Some(source) {
        return Ok(bin);
    }
    std::fs::write(&src, source)?;
    tracing::info!("Compilo l'helper {name} (solo la prima volta)…");
    let out = Command::new("swiftc")
        .arg("-O")
        .arg("-o")
        .arg(&bin)
        .arg(&src)
        .output()
        .map_err(|e| {
            anyhow!("swiftc non trovato ({e}). Installa gli strumenti di Apple con «xcode-select --install».")
        })?;
    if !out.status.success() {
        let _ = std::fs::remove_file(&bin);
        bail!("compilazione di {name} fallita:\n{}", String::from_utf8_lossy(&out.stderr));
    }
    Ok(bin)
}
