//! La desktop come centro della sincronizzazione col telefono (`src/sync`):
//! i telefoni abbinati, il server sulla rete di casa e il nome Bonjour con
//! cui il telefono la ritrova quando l'indirizzo cambia.
//!
//! I telefoni stanno in `sync/telefoni.json` (permessi 600: dentro ci sono i
//! segreti). Il server parte solo quando c'è almeno un telefono, così chi non
//! usa il telefono non apre niente sulla rete.

use crate::actor::UiEvent;
use crate::sync::{self, crypto, pair, server, store::FsStore};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::UnboundedSender;

const FILE: &str = "sync/telefoni.json";

/// Un telefono abbinato.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Phone {
    pub id: String,
    #[serde(default)]
    pub name: String,
    /// Il segreto (base64).
    key: String,
    /// Quando si è abbinato (data e ora locali).
    pub paired: String,
    /// Ultima sincronizzazione (data e ora locali), vuota = mai.
    #[serde(default)]
    pub last: String,
}

#[derive(Serialize, Deserialize, Default)]
struct PhonesFile {
    desk_id: String,
    #[serde(default)]
    phones: Vec<Phone>,
}

struct Shared {
    file: PhonesFile,
    server: server::Server,
    store: FsStore,
    /// Cosa è cambiato qui nella sincronizzazione in corso, per telefono.
    changed: std::collections::HashMap<String, BTreeSet<&'static str>>,
}

pub struct PhoneSync {
    shared: Arc<Mutex<Shared>>,
    evt: UnboundedSender<UiEvent>,
    running: bool,
    _mdns: Option<mdns_sd::ServiceDaemon>,
    /// Perché il server non è partito.
    pub error: Option<String>,
}

fn now_label() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn load() -> PhonesFile {
    let mut f: PhonesFile = std::fs::read_to_string(FILE).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    if f.desk_id.is_empty() {
        f.desk_id = sync::random_hex(8);
    }
    f
}

fn save(f: &PhonesFile) -> std::io::Result<()> {
    sync::write_atomic(std::path::Path::new(FILE), serde_json::to_string_pretty(f).unwrap_or_default().as_bytes())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(FILE, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// Il nome del computer, da mostrare sul telefono.
pub fn computer_name() -> String {
    let from_cmd = |c: &str, a: &[&str]| {
        std::process::Command::new(c).args(a).output().ok().filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    let n = if cfg!(target_os = "macos") {
        from_cmd("scutil", &["--get", "ComputerName"])
    } else {
        std::env::var("COMPUTERNAME").ok().or_else(|| std::fs::read_to_string("/etc/hostname").ok().map(|s| s.trim().to_string()))
    };
    n.filter(|s| !s.is_empty()).unwrap_or_else(|| "Computer".into())
}

impl PhoneSync {
    /// Legge i telefoni abbinati e, se ce n'è almeno uno, avvia il server.
    pub fn new(evt: UnboundedSender<UiEvent>) -> PhoneSync {
        let file = load();
        let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let server = server::Server::new(&file.desk_id, &computer_name());
        let any = !file.phones.is_empty();
        let mut store = FsStore::desktop(&root);
        // le impostazioni AI, con le chiavi (solo in memoria e cifrate in rete)
        store.set_ai(Box::new(crate::aiconfig::PhoneAi::default()));
        let mut me = PhoneSync {
            shared: Arc::new(Mutex::new(Shared { file, server, store, changed: Default::default() })),
            evt,
            running: false,
            _mdns: None,
            error: None,
        };
        if any {
            me.ensure_running();
        }
        me
    }

    pub fn phones(&self) -> Vec<Phone> {
        self.shared.lock().map(|s| s.file.phones.clone()).unwrap_or_default()
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Avvia il server e l'annuncio Bonjour (una volta sola).
    pub fn ensure_running(&mut self) {
        if self.running {
            return;
        }
        let listener = match std::net::TcpListener::bind(("0.0.0.0", sync::PORT)) {
            Ok(l) => l,
            Err(e) => {
                self.error = Some(format!("la porta {} è occupata ({e})", sync::PORT));
                return;
            }
        };
        self.running = true;
        self.error = None;
        let shared = self.shared.clone();
        let evt = self.evt.clone();
        std::thread::Builder::new()
            .name("sync-telefono".into())
            .spawn(move || {
                for sock in listener.incoming().flatten() {
                    // solo dalla rete di casa
                    if !sock.peer_addr().is_ok_and(|a| crate::web::is_local(a.ip())) {
                        continue;
                    }
                    let (shared, evt) = (shared.clone(), evt.clone());
                    std::thread::spawn(move || {
                        let _ = server::serve_conn(sock, |req| answer(&shared, &evt, req));
                    });
                }
            })
            .ok();
        self._mdns = advertise(&self.shared.lock().map(|s| s.file.desk_id.clone()).unwrap_or_default());
    }

    /// Un telefono nuovo: il testo del QR da fargli inquadrare.
    pub fn pair(&mut self) -> Result<String, String> {
        self.ensure_running();
        if let Some(e) = &self.error {
            return Err(e.clone());
        }
        let ip = crate::web::lan_ip().filter(|ip| !ip.is_loopback()).ok_or("il computer non è collegato a una rete")?;
        let key = crypto::new_secret();
        let mut s = self.shared.lock().map_err(|_| "errore interno")?;
        let id = sync::random_hex(6);
        s.file.phones.push(Phone { id: id.clone(), name: String::new(), key: pair::secret_b64(&key), paired: now_label(), last: String::new() });
        save(&s.file).map_err(|e| e.to_string())?;
        Ok(pair::encode(&pair::Pairing {
            desk_id: s.file.desk_id.clone(),
            desk_name: s.server.desk_name.clone(),
            hosts: vec![ip.to_string()],
            port: sync::PORT,
            peer_id: id,
            secret: pair::secret_b64(&key),
        }))
    }

    /// Toglie un telefono: il suo segreto non vale più.
    pub fn unpair(&mut self, id: &str) {
        if let Ok(mut s) = self.shared.lock() {
            s.file.phones.retain(|p| p.id != id);
            let _ = save(&s.file);
        }
    }
}

fn answer(shared: &Arc<Mutex<Shared>>, evt: &UnboundedSender<UiEvent>, req: server::Request) -> server::Response {
    let Ok(mut g) = shared.lock() else {
        return server::Response { status: 500, body: Vec::new(), peer: None, peer_name: None, changed: None, done: false };
    };
    let s = &mut *g;
    let peers: Vec<server::PeerKey> = s
        .file
        .phones
        .iter()
        .filter_map(|p| pair::secret_from_b64(&p.key).ok().map(|key| server::PeerKey { id: p.id.clone(), key }))
        .collect();
    let r = s.server.handle(&peers, &mut s.store, req, sync::now());
    let Some(id) = r.peer.clone() else { return r };
    if let Some(name) = r.peer_name.as_ref().filter(|n| !n.trim().is_empty()) {
        if let Some(p) = s.file.phones.iter_mut().find(|p| p.id == id) {
            if p.name != *name {
                p.name = name.clone();
                let _ = save(&s.file);
            }
        }
    }
    if let Some(k) = r.changed.filter(|k| !k.is_empty()) {
        s.changed.entry(id.clone()).or_default().insert(k);
    }
    if r.done {
        let changed: Vec<&'static str> = s.changed.remove(&id).unwrap_or_default().into_iter().collect();
        let mut name = String::new();
        if let Some(p) = s.file.phones.iter_mut().find(|p| p.id == id) {
            p.last = now_label();
            name = p.name.clone();
        }
        let _ = save(&s.file);
        let _ = evt.send(UiEvent::PhoneSynced { name, changed });
    }
    r
}

/// Annuncia la desktop sulla rete di casa (Bonjour), così il telefono la
/// ritrova anche quando cambia indirizzo.
fn advertise(desk_id: &str) -> Option<mdns_sd::ServiceDaemon> {
    let d = mdns_sd::ServiceDaemon::new().map_err(|e| tracing::warn!("Bonjour: {e}")).ok()?;
    let host = format!("openslate-{desk_id}.local.");
    let props = [("id", desk_id)];
    let info = mdns_sd::ServiceInfo::new(sync::SERVICE, &format!("OpenSlate {desk_id}"), &host, "", sync::PORT, &props[..])
        .map_err(|e| tracing::warn!("Bonjour: {e}"))
        .ok()?
        .enable_addr_auto();
    d.register(info).map_err(|e| tracing::warn!("Bonjour: {e}")).ok()?;
    Some(d)
}
