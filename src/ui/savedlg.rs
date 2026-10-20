//! Finestre «Salva con nome» del sistema per salvare ed esportare: si vede
//! sempre dove va il file. Si parte dall'ultima cartella scelta (ricordata in
//! `settings.conf`, chiave `save_dir`) oppure, la prima volta, da `exports/`.

use crate::export::EXPORT_DIR;
use crate::settings::Settings;
use std::path::{Path, PathBuf};

/// La cartella di serie: `exports/` nella cartella dei dati dell'app.
pub fn default_dir() -> PathBuf {
    let _ = std::fs::create_dir_all(EXPORT_DIR);
    std::fs::canonicalize(EXPORT_DIR).unwrap_or_else(|_| PathBuf::from(EXPORT_DIR))
}

/// Da dove parte la finestra: l'ultima cartella scelta se c'è ancora,
/// altrimenti quella di serie.
pub fn start_dir(s: &Settings) -> PathBuf {
    let last = PathBuf::from(s.save_dir.trim());
    if !s.save_dir.trim().is_empty() && last.is_dir() {
        last
    } else {
        default_dir()
    }
}

/// «Salva con nome» per un file `<name>.<ext>`. `None` = annullato.
/// Se si toglie l'estensione dal nome la si rimette.
pub fn save_file(s: &mut Settings, title: &str, name: &str, kind: &str, ext: &str) -> Option<PathBuf> {
    let mut p = rfd::FileDialog::new()
        .set_title(title)
        .set_directory(start_dir(s))
        .set_file_name(format!("{name}.{ext}"))
        .add_filter(kind, &[ext])
        .save_file()?;
    if !p.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext)) {
        p = PathBuf::from(format!("{}.{ext}", p.display()));
    }
    if let Some(dir) = p.parent() {
        remember(s, dir);
    }
    Some(p)
}

/// Sceglie una cartella (per più file insieme). `None` = annullato.
pub fn pick_folder(s: &mut Settings, title: &str) -> Option<PathBuf> {
    let dir = rfd::FileDialog::new().set_title(title).set_directory(start_dir(s)).pick_folder()?;
    remember(s, &dir);
    Some(dir)
}

/// Percorso libero in `dir` per `<stem>.<ext>`: se c'è già, `<stem>-2.<ext>`, …
pub fn free_path(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let mut p = dir.join(format!("{stem}.{ext}"));
    let mut n = 2;
    while p.exists() {
        p = dir.join(format!("{stem}-{n}.{ext}"));
        n += 1;
    }
    p
}

/// Ricorda la cartella per la prossima volta.
fn remember(s: &mut Settings, dir: &Path) {
    let d = dir.to_string_lossy().to_string();
    if s.save_dir != d {
        s.save_dir = d;
        s.save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nomi_liberi() {
        let dir = std::env::temp_dir().join(format!("openslate-savedlg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(free_path(&dir, "pagina", "pdf"), dir.join("pagina.pdf"));
        std::fs::write(dir.join("pagina.pdf"), b"x").unwrap();
        assert_eq!(free_path(&dir, "pagina", "pdf"), dir.join("pagina-2.pdf"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn cartella_di_partenza() {
        let mut s = Settings::default();
        s.save_dir = "/questa/cartella/non/esiste".into();
        assert!(start_dir(&s).ends_with(EXPORT_DIR));
        s.save_dir = std::env::temp_dir().to_string_lossy().into();
        assert_eq!(start_dir(&s), std::env::temp_dir());
    }
}
