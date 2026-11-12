//! Le risposte della desktop alle richieste del telefono, senza la rete
//! (quella sta in `src/phonesync.rs`): si provano anche senza socket.
//!
//! - `GET  /sync/v1/hello`     chi è la desktop (il telefono dice il suo nome)
//! - `GET  /sync/v1/manifest`  i file della desktop
//! - `GET  /sync/v1/f/<p>`     un file
//! - `PUT  /sync/v1/f/<p>`     scrive un file
//! - `DELETE /sync/v1/f/<p>`   toglie un file (nel cestino)
//! - `POST /sync/v1/done`      fine della sincronizzazione
//!
//! Ogni richiesta ha `X-OS-Peer` (il telefono), `X-OS-Time` (secondi Unix) e
//! `X-OS-Mtime`; il corpo è una busta cifrata col segreto del telefono, anche
//! quando è vuoto: così una richiesta senza segreto non arriva a niente.

use super::crypto::{self, Secret};
use super::store::Store;
use std::collections::HashMap;

/// Un telefono abbinato.
pub struct PeerKey {
    pub id: String,
    pub key: Secret,
}

/// Una richiesta già letta dalla rete.
pub struct Request<'a> {
    pub method: &'a str,
    /// Il percorso come arriva (`/sync/v1/f/lib%2Fx.json`).
    pub path: &'a str,
    pub peer: &'a str,
    pub time: i64,
    pub mtime: i64,
    pub body: &'a [u8],
}

pub struct Response {
    pub status: u16,
    pub body: Vec<u8>,
    /// Il telefono che ha chiesto (se riconosciuto).
    pub peer: Option<String>,
    /// Il nome del telefono, da `hello`.
    pub peer_name: Option<String>,
    /// Che cosa è cambiato qui ("lib", "notes", "stamps").
    pub changed: Option<&'static str>,
    /// Fine della sincronizzazione.
    pub done: bool,
}

impl Response {
    fn plain(status: u16, msg: &str) -> Response {
        Response { status, body: msg.as_bytes().to_vec(), peer: None, peer_name: None, changed: None, done: false }
    }
}

/// Ricorda i nonce già visti (una busta ripetuta si rifiuta).
#[derive(Default)]
pub struct Server {
    seen: HashMap<[u8; crypto::NONCE_LEN], i64>,
    pub desk_id: String,
    pub desk_name: String,
}

/// Quanto può essere sbagliato l'orologio del telefono.
const SKEW: i64 = 300;

impl Server {
    pub fn new(desk_id: &str, desk_name: &str) -> Server {
        Server { seen: HashMap::new(), desk_id: desk_id.into(), desk_name: desk_name.into() }
    }

    pub fn handle(&mut self, peers: &[PeerKey], store: &mut dyn Store, req: Request, now: i64) -> Response {
        let Some(peer) = peers.iter().find(|p| p.id == req.peer) else {
            return Response::plain(401, "telefono non collegato");
        };
        if (req.time - now).abs() > SKEW {
            return Response::plain(401, "orologio del telefono sbagliato");
        }
        let aad = crypto::req_aad(req.method, req.path, req.time, req.peer, req.mtime);
        let Ok(plain) = crypto::open(&peer.key, &aad, req.body) else {
            return Response::plain(401, "richiesta non autentica");
        };
        // una busta già vista è una ripetizione
        let nonce = crypto::nonce_of(req.body).unwrap_or_default();
        self.seen.retain(|_, t| now - *t <= 2 * SKEW);
        if self.seen.insert(nonce, now).is_some() {
            return Response::plain(401, "richiesta ripetuta");
        }
        let mut r = self.route(store, &req, &plain);
        r.peer = Some(peer.id.clone());
        r.body = crypto::seal(&peer.key, &crypto::resp_aad(&aad), &r.body);
        r
    }

    fn route(&mut self, store: &mut dyn Store, req: &Request, plain: &[u8]) -> Response {
        let ok = |body: Vec<u8>| Response { status: 200, body, peer: None, peer_name: None, changed: None, done: false };
        let err = |status: u16, e: String| Response { status, body: e.into_bytes(), peer: None, peer_name: None, changed: None, done: false };
        let Some(rest) = req.path.strip_prefix("/sync/v1/") else { return err(404, "?".into()) };
        match (req.method, rest) {
            ("GET", "hello") => {
                let mut r = ok(serde_json::json!({ "id": self.desk_id, "name": self.desk_name }).to_string().into_bytes());
                r.peer_name = Some(String::from_utf8_lossy(plain).chars().take(80).collect());
                r
            }
            ("GET", "manifest") => match store.manifest() {
                Ok(m) => ok(serde_json::to_vec(&m).unwrap_or_default()),
                Err(e) => err(500, e.to_string()),
            },
            ("POST", "done") => {
                let mut r = ok(Vec::new());
                r.done = true;
                r
            }
            (m, f) if f.starts_with("f/") => {
                let p = decode(&f[2..]);
                if !super::valid_path(&p) {
                    return err(400, "percorso non ammesso".into());
                }
                let res = match m {
                    "GET" => store.read(&p).map(Some),
                    "PUT" => store.write(&p, plain).map(|_| None),
                    "DELETE" => store.delete(&p).map(|_| None),
                    _ => return err(405, "?".into()),
                };
                match res {
                    Ok(Some(data)) => ok(data),
                    Ok(None) => {
                        let mut r = ok(Vec::new());
                        r.changed = Some(super::kind_of(&p));
                        r
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => err(404, "non c'è".into()),
                    Err(e) => err(500, e.to_string()),
                }
            }
            _ => err(404, "?".into()),
        }
    }
}

/// Legge una richiesta HTTP da `sock`, la passa a `answer` e scrive la
/// risposta. Una richiesta per collegamento (`Connection: close`).
pub fn serve_conn(mut sock: std::net::TcpStream, answer: impl FnOnce(Request) -> Response) -> std::io::Result<()> {
    use std::io::{Read, Write};
    let t = Some(std::time::Duration::from_secs(60));
    sock.set_read_timeout(t)?;
    sock.set_write_timeout(t)?;
    let mut buf = Vec::with_capacity(4096);
    let mut chunk = [0u8; 8192];
    let head_end = loop {
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i;
        }
        if buf.len() > 16 * 1024 {
            return Err(std::io::Error::other("intestazione troppo lunga"));
        }
        let n = sock.read(&mut chunk)?;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&chunk[..n]);
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).into_owned();
    let mut lines = head.split("\r\n");
    let mut first = lines.next().unwrap_or("").split(' ');
    let (method, path) = (first.next().unwrap_or("").to_string(), first.next().unwrap_or("").to_string());
    let mut h: HashMap<String, String> = HashMap::new();
    for l in lines {
        if let Some((k, v)) = l.split_once(':') {
            h.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }
    let len: usize = h.get("content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
    if len > super::MAX_FILE + 1024 {
        return reply(&mut sock, &Response::plain(413, "troppo grande"));
    }
    let mut body = buf[head_end + 4..].to_vec();
    while body.len() < len {
        let n = sock.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
    }
    body.truncate(len);
    let num = |k: &str| h.get(k).and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
    let peer = h.get("x-os-peer").cloned().unwrap_or_default();
    let r = answer(Request { method: &method, path: &path, peer: &peer, time: num("x-os-time"), mtime: num("x-os-mtime"), body: &body });
    reply(&mut sock, &r)?;
    sock.flush()
}

fn reply(sock: &mut std::net::TcpStream, r: &Response) -> std::io::Result<()> {
    use std::io::Write;
    let reason = match r.status {
        200 => "OK",
        401 => "Unauthorized",
        404 => "Not Found",
        413 => "Payload Too Large",
        _ => "Error",
    };
    let head = format!(
        "HTTP/1.1 {} {reason}\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        r.status,
        r.body.len()
    );
    sock.write_all(head.as_bytes())?;
    sock.write_all(&r.body)
}

/// Un percorso logico in un URL (`%XX` per tutto tranne lettere, cifre e `-._`).
pub fn encode(p: &str) -> String {
    p.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

pub fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() && b[i + 1].is_ascii_hexdigit() && b[i + 2].is_ascii_hexdigit() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percorsi_negli_url() {
        for p in ["lib/a.json", "notes/Riunione (bozza) è.md", "sig/timbri.json"] {
            assert_eq!(decode(&encode(p)), p);
        }
        assert_eq!(decode("a%2"), "a%2");
    }
}
