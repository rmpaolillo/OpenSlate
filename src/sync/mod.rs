//! Sincronizzazione fra la desktop e le app per telefono, sulla rete di casa.
//! Lo stesso codice è nelle app per telefono (`mobile/core`, con `#[path]`).
//!
//! - La desktop fa da centro (`server`): ascolta sulla rete di casa e si fa
//!   trovare col nome Bonjour `_openslate-sync._tcp`.
//! - Il telefono si abbina una volta col QR della desktop (`pair`): dentro c'è
//!   un segreto di 32 byte solo per lui. Da allora è il telefono a
//!   sincronizzare (`client`), da solo, quando può.
//! - Ogni richiesta e ogni risposta sono cifrate e firmate col segreto
//!   (`crypto`, ChaCha20-Poly1305): sulla rete non passa niente in chiaro.
//! - I dati delle due parti sono messi in uno spazio comune di «percorsi
//!   logici» (`store`): `lib/<id>.json` una pagina dell'archivio, `lib/<id>.md`
//!   il suo appunto, `notes/<nome>.md` un appunto, `sig/timbri.json` i timbri,
//!   `doc/<id>/<file>` i file di un documento da firmare (`doc.json`,
//!   `source.pdf`, il file di partenza).
//! - Cosa copiare lo decide `plan`, confrontando le due parti con com'erano
//!   all'ultima sincronizzazione: niente si perde, quello che si toglie o si
//!   sovrascrive va nel cestino.

pub mod client;
pub mod crypto;
pub mod pair;
pub mod plan;
pub mod server;
pub mod store;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Porta della sincronizzazione sulla desktop.
pub const PORT: u16 = 47613;
/// Nome del servizio Bonjour.
pub const SERVICE: &str = "_openslate-sync._tcp.local.";
/// File più grande che si scambia.
pub const MAX_FILE: usize = 200 * 1024 * 1024;

/// Un file nello spazio comune: impronta del contenuto, quando è cambiato
/// (secondi Unix) e quanto è grande.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    #[serde(rename = "h")]
    pub hash: String,
    #[serde(rename = "t")]
    pub mtime: i64,
    #[serde(rename = "s")]
    pub size: u64,
}

/// Tutti i file di una parte, per percorso logico.
pub type Manifest = BTreeMap<String, Entry>;

/// Impronta SHA-256 in esadecimale.
pub fn hash(data: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(data).iter().map(|b| format!("{b:02x}")).collect()
}

/// Un nome di file accettabile: niente cartelle, niente nascosti, niente `..`.
pub fn safe_name(n: &str) -> bool {
    !n.is_empty()
        && n.len() <= 200
        && !n.starts_with('.')
        && !n.contains("..")
        && n.chars().all(|c| c.is_alphanumeric() || " ._-()+,'&".contains(c))
}

/// Un percorso logico che si può scambiare (gli altri si rifiutano).
pub fn valid_path(p: &str) -> bool {
    match p.split_once('/') {
        Some(("lib", n)) => safe_name(n) && (n.ends_with(".json") || n.ends_with(".md")),
        Some(("notes", n)) => safe_name(n) && n.ends_with(".md"),
        Some(("sig", n)) => n == "timbri.json",
        Some(("doc", rest)) => match rest.split_once('/') {
            Some((id, n)) => safe_name(id) && id != "cestino" && safe_name(n) && n != "text.txt",
            None => false,
        },
        _ => false,
    }
}

/// Che cosa è cambiato, per ricaricare solo quello: "lib", "notes", "stamps".
pub fn kind_of(p: &str) -> &'static str {
    match p.split('/').next() {
        Some("lib") if p.ends_with(".md") => "notes",
        Some("lib") => "lib",
        Some("notes") => "notes",
        Some("sig") => "stamps",
        Some("doc") => "docs",
        _ => "",
    }
}

/// Scrittura sicura: file temporaneo nella stessa cartella e poi rinomina.
pub fn write_atomic(p: &Path, data: &[u8]) -> std::io::Result<()> {
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d)?;
    }
    let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let tmp = p.with_file_name(format!(".{name}.sync.tmp"));
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, p)
}

/// Adesso, in secondi Unix.
pub fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// Esadecimale casuale di `n` byte (identificativi).
pub fn random_hex(n: usize) -> String {
    let mut b = vec![0u8; n];
    let _ = getrandom::fill(&mut b);
    b.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percorsi_ammessi() {
        assert!(valid_path("lib/20261112-101500.json"));
        assert!(valid_path("lib/20261112-101500.md"));
        assert!(valid_path("notes/appunti-20261112-101500.md"));
        assert!(valid_path("notes/Riunione con Gino (bozza).md"));
        assert!(valid_path("sig/timbri.json"));
        assert!(valid_path("doc/20261112-090000-Contratto/doc.json"));
        assert!(valid_path("doc/20261112-090000-Contratto/source.pdf"));
        for p in ["lib/../x.json", "lib/.x.json", "lib/a/b.json", "notes/x.txt", "sig/altro.json", "doc/x", "x", "lib/", "doc/cestino/doc.json", "doc/a/pages/p.png", "doc/a/text.txt", "doc/../x/doc.json"] {
            assert!(!valid_path(p), "{p}");
        }
        assert_eq!(kind_of("lib/a.md"), "notes");
        assert_eq!(kind_of("lib/a.json"), "lib");
    }
}
