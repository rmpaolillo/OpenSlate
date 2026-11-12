//! La sincronizzazione vista dal telefono: trova la desktop, confronta, copia
//! nei due sensi e ricorda com'è finita (`sync/base.json`).

use super::crypto::{self, Secret};
use super::pair::Pairing;
use super::plan::{self, Base, Op};
use super::server::encode;
use super::store::Store;
use super::Manifest;
use std::collections::BTreeSet;
use std::io::Read;
use std::time::Duration;

/// Com'è andata.
#[derive(Debug, Default, Clone)]
pub struct Report {
    /// L'indirizzo della desktop che ha risposto.
    pub host: String,
    pub pulled: u32,
    pub pushed: u32,
    pub deleted: u32,
    /// File cambiati da tutte e due le parti (una versione è nel cestino).
    pub conflicts: u32,
    /// Che cosa è cambiato qui ("lib", "notes", "stamps"): da ricaricare.
    pub changed: BTreeSet<&'static str>,
}

struct Conn {
    base: String,
    peer: String,
    key: Secret,
    agent: ureq::Agent,
}

impl Conn {
    fn new(host: &str, port: u16, peer: &str, key: Secret, connect: Duration) -> Conn {
        let host = if host.contains(':') && !host.starts_with('[') { format!("[{host}]") } else { host.to_string() };
        Conn {
            base: format!("http://{host}:{port}"),
            peer: peer.into(),
            key,
            agent: ureq::AgentBuilder::new().timeout_connect(connect).timeout_read(Duration::from_secs(120)).timeout_write(Duration::from_secs(120)).build(),
        }
    }

    fn call(&self, method: &str, path: &str, mtime: i64, body: &[u8]) -> Result<Vec<u8>, String> {
        let time = super::now();
        let aad = crypto::req_aad(method, path, time, &self.peer, mtime);
        let env = crypto::seal(&self.key, &aad, body);
        let r = self
            .agent
            .request(method, &format!("{}{path}", self.base))
            .set("X-OS-Peer", &self.peer)
            .set("X-OS-Time", &time.to_string())
            .set("X-OS-Mtime", &mtime.to_string())
            .set("Content-Type", "application/octet-stream")
            .send_bytes(&env);
        let resp = match r {
            Ok(r) => r,
            Err(ureq::Error::Status(401, r)) => {
                return Err(format!("il computer non accetta questo telefono ({}): ricollegalo col QR", r.into_string().unwrap_or_default()))
            }
            Err(ureq::Error::Status(code, r)) => {
                // gli errori arrivano cifrati se la richiesta era buona
                let raw = read_all(r.into_reader())?;
                let msg = crypto::open(&self.key, &crypto::resp_aad(&aad), &raw).unwrap_or(raw);
                return Err(format!("{code}: {}", String::from_utf8_lossy(&msg)));
            }
            Err(e) => return Err(e.to_string()),
        };
        let raw = read_all(resp.into_reader())?;
        crypto::open(&self.key, &crypto::resp_aad(&aad), &raw)
    }

    fn file(&self, p: &str) -> String {
        format!("/sync/v1/f/{}", encode(p))
    }
}

fn read_all(r: impl Read) -> Result<Vec<u8>, String> {
    let mut v = Vec::new();
    r.take(super::MAX_FILE as u64 + 1024).read_to_end(&mut v).map_err(|e| e.to_string())?;
    Ok(v)
}

/// Sincronizza con la desktop di `pairing`. `found` sono indirizzi trovati
/// adesso sulla rete (Bonjour), provati prima di quelli del QR; `device` il
/// nome del telefono (lo si vede sulla desktop).
pub fn run(pairing: &Pairing, found: &[String], device: &str, store: &mut dyn Store, state: &std::path::Path) -> Result<Report, String> {
    let key = pairing.key()?;
    let mut hosts: Vec<&String> = found.iter().chain(pairing.hosts.iter()).collect();
    hosts.dedup();
    // il primo indirizzo che risponde, ed è proprio quella desktop
    let mut conn = None;
    let mut last_err = String::from("computer non raggiungibile");
    for h in hosts {
        let c = Conn::new(h, pairing.port, &pairing.peer_id, key, Duration::from_secs(3));
        match c.call("GET", "/sync/v1/hello", 0, device.as_bytes()) {
            Ok(b) => {
                let v: serde_json::Value = serde_json::from_slice(&b).unwrap_or_default();
                if v["id"].as_str() == Some(pairing.desk_id.as_str()) {
                    conn = Some((h.clone(), c));
                    break;
                }
                last_err = "a quell'indirizzo c'è un altro computer".into();
            }
            Err(e) => last_err = e,
        }
    }
    let Some((host, c)) = conn else { return Err(last_err) };
    let mut rep = Report { host, ..Default::default() };

    let base_file = state.join("base.json");
    let base: Base = std::fs::read_to_string(&base_file).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    let manifest = |c: &Conn| -> Result<Manifest, String> {
        serde_json::from_slice(&c.call("GET", "/sync/v1/manifest", 0, &[])?).map_err(|e| e.to_string())
    };
    let remote = manifest(&c)?;
    let local = store.manifest().map_err(|e| e.to_string())?;
    let mtime_of = |m: &Manifest, p: &str| m.get(p).map(|e| e.mtime).unwrap_or(0);

    let mut errors = Vec::new();
    for op in plan::plan(&local, &remote, &base) {
        // cambiato da tutte e due le parti dall'ultima volta
        if let Op::Pull(p) | Op::Push(p) = &op {
            let b = base.get(p);
            if local.contains_key(p) && remote.contains_key(p) && local.get(p).map(|e| &e.hash) != b && remote.get(p).map(|e| &e.hash) != b {
                rep.conflicts += 1;
            }
        }
        let r: Result<(), String> = (|| {
            match &op {
                Op::Pull(p) => {
                    let data = c.call("GET", &c.file(p), 0, &[])?;
                    store.write(p, &data).map_err(|e| e.to_string())?;
                    rep.pulled += 1;
                    rep.changed.insert(super::kind_of(p));
                }
                Op::Push(p) => {
                    let data = store.read(p).map_err(|e| e.to_string())?;
                    c.call("PUT", &c.file(p), mtime_of(&local, p), &data)?;
                    rep.pushed += 1;
                }
                Op::DelLocal(p) => {
                    store.delete(p).map_err(|e| e.to_string())?;
                    rep.deleted += 1;
                    rep.changed.insert(super::kind_of(p));
                }
                Op::DelRemote(p) => {
                    c.call("DELETE", &c.file(p), 0, &[])?;
                    rep.deleted += 1;
                }
                Op::KeepBoth(p) => {
                    let mine = store.rename_free(p).map_err(|e| e.to_string())?;
                    let data = c.call("GET", &c.file(p), 0, &[])?;
                    store.write(p, &data).map_err(|e| e.to_string())?;
                    let data = store.read(&mine).map_err(|e| e.to_string())?;
                    c.call("PUT", &c.file(&mine), mtime_of(&local, p), &data)?;
                    rep.pulled += 1;
                    rep.pushed += 1;
                    rep.conflicts += 1;
                    rep.changed.insert("lib");
                }
            }
            Ok(())
        })();
        if let Err(e) = r {
            errors.push(e);
        }
    }

    // com'è finita: la base sono i file uguali sulle due parti
    let local = store.manifest().map_err(|e| e.to_string())?;
    let remote = manifest(&c)?;
    let _ = std::fs::create_dir_all(state);
    super::write_atomic(&base_file, serde_json::to_string(&plan::agreed(&local, &remote)).unwrap_or_default().as_bytes())
        .map_err(|e| e.to_string())?;
    let _ = c.call("POST", "/sync/v1/done", 0, &[]);
    match errors.first() {
        Some(e) => Err(format!("sincronizzazione a metà ({} errori): {e}", errors.len())),
        None => Ok(rep),
    }
}

#[cfg(test)]
mod tests {
    use super::super::pair::{secret_b64, Pairing};
    use super::super::server::{serve_conn, PeerKey, Server};
    use super::super::store::{FsStore, MemVirtual};
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("openslate-sync-rt-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// Una desktop finta su 127.0.0.1, con i dati in `root`.
    fn desk(root: &Path, key: Secret, ai: MemVirtual) -> u16 {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let mut store = FsStore::desktop(root);
        store.set_ai(Box::new(ai));
        let state = Arc::new(Mutex::new((Server::new("D", "Mac"), store)));
        std::thread::spawn(move || {
            for s in l.incoming().flatten() {
                let st = state.clone();
                std::thread::spawn(move || {
                    let _ = serve_conn(s, |req| {
                        let mut g = st.lock().unwrap();
                        let (srv, store) = &mut *g;
                        srv.handle(&[PeerKey { id: "T".into(), key }], store, req, super::super::now())
                    });
                });
            }
        });
        port
    }

    fn files(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(dir).map(|r| r.flatten().filter(|e| e.path().is_file()).map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
        v.sort();
        v
    }

    #[test]
    fn giro_completo_telefono_desktop() {
        let (d, t) = (tmp("desk"), tmp("tel"));
        std::fs::create_dir_all(d.join("library")).unwrap();
        std::fs::create_dir_all(d.join("exports")).unwrap();
        std::fs::create_dir_all(d.join("signatures")).unwrap();
        std::fs::write(d.join("library/d1.json"), "desk").unwrap();
        std::fs::write(d.join("exports/appunti-1.md"), "nota desk").unwrap();
        std::fs::write(d.join("signatures/timbri.json"), "[1]").unwrap();
        let tl = t.join("library");
        std::fs::create_dir_all(&tl).unwrap();
        std::fs::write(tl.join("t1.json"), "tel").unwrap();
        std::fs::write(tl.join("t1.md"), "nota tel").unwrap();
        // un documento sulla desktop, con le pagine già disegnate (non viaggiano)
        let dd = d.join("documents/20261112-090000-Contratto");
        std::fs::create_dir_all(dd.join("pages")).unwrap();
        std::fs::write(dd.join("doc.json"), "{}").unwrap();
        std::fs::write(dd.join("source.pdf"), "%PDF").unwrap();
        std::fs::write(dd.join("text.txt"), "testo").unwrap();
        std::fs::write(dd.join("pages/page-0001.png"), "png").unwrap();

        let key = crypto::new_secret();
        // le impostazioni AI: sulla desktop più recenti, arrivano sul telefono
        let desk_ai = Arc::new(Mutex::new(None));
        let port = desk(&d, key, MemVirtual { data: Some((b"[desk]".to_vec(), 2_000_000_000)), received: desk_ai.clone() });
        let p = Pairing { desk_id: "D".into(), desk_name: "Mac".into(), hosts: vec!["10.255.255.1".into()], port, peer_id: "T".into(), secret: secret_b64(&key) };
        let mut store = FsStore::phone(&t, &tl);
        let phone_ai = Arc::new(Mutex::new(None));
        store.set_ai(Box::new(MemVirtual { data: Some((b"[tel]".to_vec(), 1_000)), received: phone_ai.clone() }));
        let state = t.join("sync");
        // il QR ha un indirizzo che non risponde: si usa quello trovato sulla rete
        let r = run(&p, &["127.0.0.1".into()], "Pixel", &mut store, &state).unwrap();
        assert_eq!((r.pulled, r.pushed), (6, 2), "{r:?}");
        assert_eq!(phone_ai.lock().unwrap().as_deref(), Some(&b"[desk]"[..]));
        assert!(desk_ai.lock().unwrap().is_none() && r.changed.contains("ai"));
        assert!(!t.join("sync/cestino").exists() || std::fs::read_dir(t.join("sync/cestino")).unwrap().all(|e| !e.unwrap().file_name().to_string_lossy().starts_with("ai")));
        let td = t.join("documents/20261112-090000-Contratto");
        assert_eq!(files(&td), ["doc.json", "source.pdf"]);
        assert!(r.changed.contains("docs"));
        assert_eq!(files(&tl), ["d1.json", "t1.json", "t1.md"]);
        assert_eq!(std::fs::read(t.join("notes/appunti-1.md")).unwrap(), b"nota desk");
        assert_eq!(std::fs::read(t.join("timbri.json")).unwrap(), b"[1]");
        assert_eq!(std::fs::read(d.join("library/t1.json")).unwrap(), b"tel");
        assert_eq!(std::fs::read(d.join("exports/t1.md")).unwrap(), b"nota tel");

        // tolta una pagina sul telefono, cambiati i timbri sulla desktop
        std::fs::remove_file(tl.join("d1.json")).unwrap();
        std::thread::sleep(Duration::from_millis(1100));
        std::fs::write(d.join("signatures/timbri.json"), "[2]").unwrap();
        let r = run(&p, &["127.0.0.1".into()], "Pixel", &mut store, &state).unwrap();
        assert_eq!((r.pulled, r.deleted), (1, 1), "{r:?}");
        assert!(d.join("library/cestino/d1.json").exists() && !d.join("library/d1.json").exists());
        assert_eq!(std::fs::read(t.join("timbri.json")).unwrap(), b"[2]");
        assert!(r.changed.contains("stamps"));

        // niente da fare
        let r = run(&p, &["127.0.0.1".into()], "Pixel", &mut store, &state).unwrap();
        assert_eq!((r.pulled, r.pushed, r.deleted), (0, 0, 0));

        // il documento firmato sul telefono, poi tolto: sulla desktop arriva la
        // firma, poi il documento intero va nel cestino dei documenti
        std::thread::sleep(Duration::from_millis(1100));
        std::fs::write(td.join("doc.json"), "{\"firmato\":1}").unwrap();
        run(&p, &["127.0.0.1".into()], "Pixel", &mut store, &state).unwrap();
        assert_eq!(std::fs::read(dd.join("doc.json")).unwrap(), b"{\"firmato\":1}");
        crate::docstore::trash(&t.join("documents"), "20261112-090000-Contratto").unwrap();
        run(&p, &["127.0.0.1".into()], "Pixel", &mut store, &state).unwrap();
        assert!(!dd.exists() && d.join("documents/cestino/20261112-090000-Contratto/source.pdf").exists());

        // un altro segreto non entra
        let bad = Pairing { secret: secret_b64(&crypto::new_secret()), ..p.clone() };
        assert!(run(&bad, &["127.0.0.1".into()], "Pixel", &mut store, &state).is_err());
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::remove_dir_all(&t);
    }
}
