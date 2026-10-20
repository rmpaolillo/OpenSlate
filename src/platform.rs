//! Differenze fra sistemi: aprire file e indirizzi con l'app predefinita,
//! la cartella dei dati dell'app installata, i programmini Swift già
//! compilati dentro l'app per macOS.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Nome dell'app (cartella dei dati, titoli).
pub const APP_NAME: &str = "OpenSlate";

/// Apre un file, una cartella o un indirizzo con l'app predefinita del sistema.
pub fn open(target: impl AsRef<OsStr>) {
    let target = target.as_ref();
    let r = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(target).spawn()
    } else if cfg!(target_os = "windows") {
        // «start» è un comando di cmd; il primo argomento tra virgolette è il titolo
        std::process::Command::new("cmd").args([OsStr::new("/C"), OsStr::new("start"), OsStr::new(""), target]).spawn()
    } else {
        std::process::Command::new("xdg-open").arg(target).spawn()
    };
    if let Err(e) = r {
        tracing::warn!("apertura di {}: {e}", target.to_string_lossy());
    }
}

/// Cartella dei dati quando l'app è installata:
/// macOS `~/Library/Application Support/OpenSlate`, Linux
/// `~/.local/share/openslate` (o `$XDG_DATA_HOME`), Windows `%APPDATA%\OpenSlate`.
pub fn data_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("OPENSLATE_DATA") {
        return Some(PathBuf::from(d));
    }
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if cfg!(target_os = "macos") {
        home.map(|h| h.join("Library/Application Support").join(APP_NAME))
    } else if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join(APP_NAME))
    } else {
        std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).or_else(|| home.map(|h| h.join(".local/share"))).map(|d| d.join("openslate"))
    }
}

/// L'app lavora nella cartella corrente (archivio, documenti, impostazioni…).
/// Si resta dove si è se lì c'è il progetto (`Cargo.toml`, `cargo run`) o
/// un'installazione «portatile» (`settings.conf`); altrimenti — app aperta
/// dal Finder, dal menu Start o dal menu delle applicazioni — si va nella
/// cartella dei dati dell'utente.
pub fn enter_data_dir() {
    let here = std::env::current_dir().unwrap_or_default();
    let portable = here.join("Cargo.toml").exists() || here.join("settings.conf").exists();
    if portable && std::env::var_os("OPENSLATE_DATA").is_none() {
        return;
    }
    let Some(dir) = data_dir() else { return };
    if let Err(e) = std::fs::create_dir_all(&dir).and_then(|_| std::env::set_current_dir(&dir)) {
        eprintln!("{APP_NAME}: cartella dei dati {}: {e}", dir.display());
    }
}

/// Un programmino già compilato dentro l'app (macOS:
/// `OpenSlate.app/Contents/Resources/helpers/<name>`; altrove `helpers/<name>`
/// accanto all'eseguibile). Senza, lo si compila al primo uso con `swiftc`.
pub fn bundled_helper(name: &str) -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    [dir.join("../Resources/helpers").join(name), dir.join("helpers").join(name)].into_iter().find(|p| p.is_file()).map(|p| clean(&p))
}

fn clean(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}
