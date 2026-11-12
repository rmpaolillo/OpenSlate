//! Stampa: il documento (con firma, timbri e scrittura) o la pagina della
//! lavagna diventano un PDF, che si stampa col dialogo del sistema.
//! - macOS: il dialogo di stampa di macOS (PDFKit, `src/swift/pdf_print.swift`),
//!   con stampante, copie, pagine, anteprima e «Salva come PDF»;
//! - Windows e Linux: il PDF si apre nel lettore di PDF del sistema, da cui si
//!   stampa (Ctrl+P).

use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

/// Com'è andata.
#[derive(Debug, PartialEq, Eq)]
pub enum Printed {
    /// Mandato alla stampante (o salvato dal dialogo).
    Done,
    /// Dialogo chiuso con «Annulla».
    Cancelled,
    /// Aperto nel lettore di PDF del sistema: si stampa da lì.
    Opened,
}

/// Dove scrivere il PDF da stampare: una cartella temporanea, col nome del
/// documento (è il nome che il lettore di PDF mostra).
pub fn temp_pdf(title: &str) -> Result<PathBuf> {
    let dir = std::env::temp_dir().join("openslate-print");
    std::fs::create_dir_all(&dir)?;
    let stem: String = title
        .chars()
        .map(|c| if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control() { '_' } else { c })
        .collect();
    let stem = stem.trim();
    Ok(dir.join(format!("{}.pdf", if stem.is_empty() { "OpenSlate" } else { stem })))
}

/// Stampa il PDF `pdf`; `title` è il nome del lavoro di stampa. Aspetta che
/// il dialogo si chiuda: da chiamare fuori dal thread della finestra.
pub fn print_pdf(pdf: &Path, title: &str) -> Result<Printed> {
    if cfg!(target_os = "macos") {
        let bin = crate::swift::helper("pdf_print", include_str!("swift/pdf_print.swift"))?;
        let out = std::process::Command::new(bin).arg(pdf).arg(title).output()?;
        let _ = std::fs::remove_file(pdf);
        return match out.status.code() {
            Some(0) => Ok(Printed::Done),
            Some(2) => Ok(Printed::Cancelled),
            _ => Err(anyhow!("{}", String::from_utf8_lossy(&out.stderr).trim())),
        };
    }
    // il file resta: il lettore di PDF lo sta usando
    crate::platform::open(pdf);
    Ok(Printed::Opened)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nomi_dei_file_da_stampare() {
        let p = temp_pdf("Contratto 3/2026: «bozza»?").unwrap();
        assert_eq!(p.file_name().unwrap().to_string_lossy(), "Contratto 3_2026_ «bozza»_.pdf");
        assert_eq!(temp_pdf("  ").unwrap().file_name().unwrap().to_string_lossy(), "OpenSlate.pdf");
    }

    /// Il PDF passa dal sistema di stampa di macOS (in un file, senza dialogo).
    #[cfg(target_os = "macos")]
    #[test]
    fn stampa_in_un_file() {
        let dir = std::env::temp_dir().join(format!("openslate-print-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("a.pdf");
        crate::pdfdoc::image_pdf(&src, 2, 2, &[255; 12], 595.0, 842.0).unwrap();
        let Ok(bin) = crate::swift::helper("pdf_print", include_str!("swift/pdf_print.swift")) else {
            eprintln!("swiftc non disponibile: prova saltata");
            return;
        };
        let dst = dir.join("stampato.pdf");
        let st = std::process::Command::new(bin).arg(&src).arg("prova").arg("--to").arg(&dst).status().unwrap();
        assert!(st.success());
        assert_eq!(crate::pdfdoc::pages(&dst).unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
