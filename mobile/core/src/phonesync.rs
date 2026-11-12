//! La sincronizzazione col computer (vedi `src/sync`, lo stesso codice della
//! desktop): l'app la chiama in un thread suo, quando torna in primo piano,
//! dopo un salvataggio e ogni tanto mentre è aperta.

use crate::pages::StoreError;
use crate::ai::AiProvider;
use crate::aisync::SyncProvider;
use crate::sync::{client, pair, store::{FsStore, MemVirtual}};
use std::path::Path;
use std::sync::Mutex;

fn err(e: impl std::fmt::Display) -> StoreError {
    StoreError::Io { reason: e.to_string() }
}

/// Il computer a cui il telefono è abbinato (dal QR). `secret` va tenuto nel
/// portachiavi del telefono.
#[derive(uniffi::Record, Clone, Debug)]
pub struct SyncPairing {
    pub desk_id: String,
    pub desk_name: String,
    pub hosts: Vec<String>,
    pub port: u16,
    pub peer_id: String,
    pub secret: String,
}

/// Com'è andata una sincronizzazione.
#[derive(uniffi::Record, Clone, Debug)]
pub struct SyncReport {
    /// L'indirizzo del computer che ha risposto (da ricordare).
    pub host: String,
    pub pulled: u32,
    pub pushed: u32,
    pub deleted: u32,
    pub conflicts: u32,
    /// Cosa è cambiato sul telefono: "lib", "notes", "stamps", "docs",
    /// "signature", "ai".
    pub changed: Vec<String>,
    /// Le impostazioni AI arrivate dal computer (`ai_sync_parse`), da salvare
    /// nelle impostazioni dell'app (chiavi nel portachiavi).
    pub ai: Option<String>,
}

/// Le impostazioni AI del telefono per la sincronizzazione: `json` da
/// `ai_sync_json`, `mtime` quando sono cambiate l'ultima volta (secondi Unix).
#[derive(uniffi::Record, Clone, Debug)]
pub struct SyncAi {
    pub json: String,
    pub mtime: i64,
}

/// I servizi AI (con le chiavi) nel formato della sincronizzazione.
#[uniffi::export]
pub fn ai_sync_json(list: Vec<AiProvider>) -> String {
    let v: Vec<SyncProvider> = list
        .into_iter()
        .map(|p| SyncProvider {
            name: p.name,
            url: p.url,
            model: p.model,
            key: p.key,
            vision: p.vision,
            enabled: p.enabled,
            no_temperature: p.no_temperature,
            max_tokens: p.max_tokens,
            timeout_s: p.timeout_s,
        })
        .collect();
    serde_json::to_string(&v).unwrap_or_default()
}

/// I servizi AI arrivati dal computer.
#[uniffi::export]
pub fn ai_sync_parse(json: String) -> Result<Vec<AiProvider>, StoreError> {
    let v: Vec<SyncProvider> = serde_json::from_str(&json).map_err(err)?;
    Ok(v
        .into_iter()
        .map(|p| AiProvider {
            name: p.name,
            url: p.url,
            model: p.model,
            key: p.key,
            vision: p.vision,
            enabled: p.enabled,
            no_temperature: p.no_temperature,
            max_tokens: p.max_tokens,
            timeout_s: if p.timeout_s == 0 { 180 } else { p.timeout_s },
        })
        .collect())
}

impl From<SyncPairing> for pair::Pairing {
    fn from(p: SyncPairing) -> Self {
        pair::Pairing { desk_id: p.desk_id, desk_name: p.desk_name, hosts: p.hosts, port: p.port, peer_id: p.peer_id, secret: p.secret }
    }
}

/// L'abbinamento dal testo del QR mostrato dal computer (Opzioni › Telefono).
#[uniffi::export]
pub fn sync_parse_qr(text: String) -> Result<SyncPairing, StoreError> {
    let p = pair::decode(&text).map_err(err)?;
    Ok(SyncPairing { desk_id: p.desk_id, desk_name: p.desk_name, hosts: p.hosts, port: p.port, peer_id: p.peer_id, secret: p.secret })
}

/// Il servizio Bonjour con cui il computer si annuncia (per NsdManager /
/// NWBrowser): `_openslate-sync._tcp`.
#[uniffi::export]
pub fn sync_service_type() -> String {
    crate::sync::SERVICE.trim_end_matches(".local.").to_string()
}

static RUNNING: Mutex<()> = Mutex::new(());

/// Sincronizza col computer. `found` = indirizzi trovati adesso sulla rete
/// (provati per primi), `device` il nome del telefono, `data_dir` la cartella
/// dei dati dell'app, `library_dir` quella delle pagine. Una alla volta.
#[uniffi::export]
pub fn sync_run(
    pairing: SyncPairing,
    found: Vec<String>,
    device: String,
    data_dir: String,
    library_dir: String,
    ai: Option<SyncAi>,
) -> Result<SyncReport, StoreError> {
    let Ok(_g) = RUNNING.try_lock() else { return Err(err("sincronizzazione già in corso")) };
    let data = Path::new(&data_dir);
    let mut store = FsStore::phone(data, Path::new(&library_dir));
    let received = std::sync::Arc::new(std::sync::Mutex::new(None));
    // senza servizi sul telefono si ricevono quelli del computer
    let data = ai.filter(|a| !a.json.is_empty() && a.json != "[]").map(|a| (a.json.into_bytes(), a.mtime));
    store.set_ai(Box::new(MemVirtual { data, received: received.clone() }));
    let state = store.state_dir().to_path_buf();
    let r = client::run(&pairing.into(), &found, &device, &mut store, &state).map_err(err)?;
    Ok(SyncReport {
        host: r.host,
        pulled: r.pulled,
        pushed: r.pushed,
        deleted: r.deleted,
        conflicts: r.conflicts,
        changed: r.changed.into_iter().map(String::from).collect(),
        ai: received.lock().ok().and_then(|g| g.clone()).map(|b| String::from_utf8_lossy(&b).into_owned()),
    })
}

/// Gli appunti liberi (non legati a una pagina) arrivati dal computer:
/// nomi dei file in `<data_dir>/notes`, dal più recente.
#[uniffi::export]
pub fn notes_list(data_dir: String) -> Vec<String> {
    let dir = Path::new(&data_dir).join("notes");
    let mut v: Vec<(std::time::SystemTime, String)> = std::fs::read_dir(&dir)
        .map(|rd| {
            rd.flatten()
                .filter_map(|e| {
                    let n = e.file_name().to_str()?.to_string();
                    (n.ends_with(".md") && !n.starts_with('.')).then(|| (e.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH), n))
                })
                .collect()
        })
        .unwrap_or_default();
    v.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    v.into_iter().map(|(_, n)| n).collect()
}
