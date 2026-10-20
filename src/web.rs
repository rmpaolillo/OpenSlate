//! Diretta sul web: un piccolo server HTTP nella rete di casa.
//!
//! - `/`            pagina lavagna (canvas a tutto schermo, per TV/telefono/PC)
//! - `/events`      flusso Server-Sent Events con i tratti in tempo reale
//! - `/state.json`  stato completo (coordinate 0..1 sulla pagina)
//! - `/page.svg`, `/page.pdf`, `/page.png`  pagina attuale, per integrazioni
//! - `/chat`        chat con gli agenti AI (formule con KaTeX)
//! - `/api/context`, `/context.png`  pagina in discussione
//! - `/api/providers`  stato degli agenti di `ai.toml`
//! - `POST /api/chat`  risposta in streaming (Server-Sent Events)
//! - `/appunti`     appunti .md di exports/ con le formule (anche nella finestra dell'app)
//! - `/api/appunti`, `/api/appunti/<nome>` (GET legge, POST salva), `POST /api/context`
//!
//! Nessuna dipendenza esterna: HTTP/1.1 minimale su tokio.

use crate::export;
use crate::scene::{Hub, WebMsg};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast::error::RecvError;

const INDEX_HTML: &str = include_str!("web/index.html");
const CHAT_HTML: &str = include_str!("web/chat.html");
const APPUNTI_HTML: &str = include_str!("web/appunti.html");
const SETTINGS_HTML: &str = include_str!("web/impostazioni.html");
/// Colori, caratteri e controlli in stile WordCraft, comuni alle pagine.
const WC_CSS: &str = include_str!("web/wc.css");
const I18N_JS: &str = include_str!("web/i18n.js");
/// Richiesta di andare a una pagina, letta dalla finestra dell'app già aperta
/// (così «Chat AI» non apre una finestra nuova ogni volta).
static NAV: std::sync::Mutex<(u64, String)> = std::sync::Mutex::new((0, String::new()));

pub fn navigate(path: &str) {
    let mut n = NAV.lock().unwrap();
    n.0 += 1;
    n.1 = path.to_string();
}

/// Corpo massimo di una richiesta POST (la chat manda anche le immagini incollate).
const MAX_BODY: usize = 40 << 20;

/// Indirizzo del Mac nella rete locale (nessun pacchetto viene inviato).
pub fn lan_ip() -> Option<IpAddr> {
    let s = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("192.168.1.1:80").ok()?;
    Some(s.local_addr().ok()?.ip())
}

/// Avvia il server; ritorna l'URL da aprire sugli altri dispositivi.
pub async fn start(hub: Arc<Hub>, port: u16) -> std::io::Result<String> {
    let listener = TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], port))).await?;
    let port = listener.local_addr()?.port();
    let host = lan_ip().map(|ip| ip.to_string()).unwrap_or_else(|| "localhost".into());
    tracing::info!("Diretta web su http://{host}:{port}/");
    serve(listener, hub);
    Ok(format!("http://{host}:{port}/"))
}

/// Server solo per questo Mac (127.0.0.1, porta libera): le pagine della
/// chat e degli appunti dentro la finestra dell'app. Non si vede dalla rete.
pub async fn start_local(hub: Arc<Hub>) -> std::io::Result<u16> {
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).await?;
    let port = listener.local_addr()?.port();
    serve(listener, hub);
    Ok(port)
}

fn serve(listener: TcpListener, hub: Arc<Hub>) {
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((sock, peer)) => {
                    let hub = hub.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle(sock, hub).await {
                            tracing::debug!("web {peer}: {e}");
                        }
                    });
                }
                Err(e) => {
                    tracing::warn!("web accept: {e}");
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            }
        }
    });
}

async fn handle(mut sock: TcpStream, hub: Arc<Hub>) -> std::io::Result<()> {
    // Legge l'intestazione della richiesta (fino a riga vuota, max 16 KB).
    let mut buf = Vec::with_capacity(1024);
    let mut chunk = [0u8; 4096];
    let head_end = loop {
        let n = tokio::time::timeout(Duration::from_secs(10), sock.read(&mut chunk))
            .await
            .map_err(|_| std::io::Error::other("timeout richiesta"))??;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
        if buf.len() > 16384 {
            return respond(&mut sock, "431 Request Header Fields Too Large", "text/plain", b"intestazione troppo lunga").await;
        }
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).into_owned();
    let mut parts = head.lines().next().unwrap_or("").split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("/").split('?').next().unwrap_or("/").to_string();
    let path = path.as_str();

    if method == "POST" {
        let len: usize = head
            .lines()
            .find_map(|l| {
                let (k, v) = l.split_once(':')?;
                k.trim().eq_ignore_ascii_case("content-length").then(|| v.trim().parse().ok())?
            })
            .unwrap_or(0);
        if len > MAX_BODY {
            return respond(&mut sock, "413 Payload Too Large", "text/plain", b"troppo grande").await;
        }
        let mut body = buf[head_end..].to_vec();
        while body.len() < len {
            let n = tokio::time::timeout(Duration::from_secs(20), sock.read(&mut chunk))
                .await
                .map_err(|_| std::io::Error::other("timeout corpo"))??;
            if n == 0 {
                break;
            }
            body.extend_from_slice(&chunk[..n]);
        }
        body.truncate(len);
        let peer = sock.peer_addr().map(|a| a.ip()).ok();
        // Impostazioni e chiavi: solo dal Mac stesso.
        if path.starts_with("/api/config") && !peer.is_some_and(|ip| ip.is_loopback()) {
            return respond(&mut sock, "403 Forbidden", "text/plain", b"impostazioni solo dal Mac").await;
        }
        if !peer.is_some_and(is_local) {
            return respond(&mut sock, "403 Forbidden", "text/plain", b"solo dalla rete locale").await;
        }
        let json_reply = |v: serde_json::Value| v.to_string();
        return match path {
            "/api/config" | "/api/config/test" | "/api/config/models" | "/api/compact" => {
                let path = path.to_string();
                let out = tokio::task::spawn_blocking(move || match path.as_str() {
                    "/api/config" => crate::aiconfig::save_from_json(&body),
                    "/api/config/test" => crate::aiconfig::test_from_json(&body),
                    "/api/config/models" => crate::aiconfig::models_from_json(&body),
                    _ => crate::aiconfig::compact_from_json(&body),
                })
                .await
                .unwrap_or_else(|e| serde_json::json!({ "ok": false, "error": e.to_string() }));
                respond(&mut sock, "200 OK", "application/json", json_reply(out).as_bytes()).await
            }
            "/api/chat" => {
                // Le chiavi API sono del Mac: la chat si usa solo dalla rete di casa.
                let local = sock.peer_addr().map(|a| is_local(a.ip())).unwrap_or(false);
                if !local {
                    return respond(&mut sock, "403 Forbidden", "text/plain", b"solo dalla rete locale").await;
                }
                chat(sock, &body).await
            }
            p if p.starts_with("/api/chats/") => {
                let rest = &p["/api/chats/".len()..];
                let (id, op) = rest.split_once('/').unwrap_or((rest, ""));
                let id = id.to_string();
                let out = match op {
                    "" => crate::chats::save(&id, &body).map(|_| serde_json::json!({ "ok": true })),
                    "delete" => crate::chats::trash(&id).map(|_| serde_json::json!({ "ok": true })),
                    "restore" => crate::chats::restore(&id).map(|_| serde_json::json!({ "ok": true })),
                    "attach" => crate::chats::attach_current(&id).map(|page| serde_json::json!({ "ok": true, "page": page })),
                    _ => Err("operazione sconosciuta".into()),
                };
                let out = out.unwrap_or_else(|e| serde_json::json!({ "ok": false, "error": e }));
                respond(&mut sock, "200 OK", "application/json", out.to_string().as_bytes()).await
            }
            p if p.starts_with("/api/appunti/") || p == "/api/context" => {
                // Scrivere file e cambiare il contesto: solo dalla rete di casa.
                let local = sock.peer_addr().map(|a| is_local(a.ip())).unwrap_or(false);
                if !local {
                    return respond(&mut sock, "403 Forbidden", "text/plain", b"solo dalla rete locale").await;
                }
                if p == "/api/context" {
                    let v: serde_json::Value = serde_json::from_slice(&body).unwrap_or_default();
                    crate::ai::set_context(
                        v["title"].as_str().unwrap_or(""),
                        None,
                        v["markdown"].as_str().unwrap_or(""),
                    );
                    return respond(&mut sock, "200 OK", "application/json", b"{\"ok\":true}").await;
                }
                match appunto_path(&p["/api/appunti/".len()..]) {
                    Some(f) => match std::fs::create_dir_all(export::EXPORT_DIR).and_then(|_| std::fs::write(&f, &body)) {
                        Ok(()) => respond(&mut sock, "200 OK", "application/json", b"{\"ok\":true}").await,
                        Err(e) => respond(&mut sock, "500 Internal Server Error", "text/plain", e.to_string().as_bytes()).await,
                    },
                    None => respond(&mut sock, "400 Bad Request", "text/plain", b"nome non valido").await,
                }
            }
            _ => respond(&mut sock, "404 Not Found", "text/plain", b"non trovato").await,
        };
    }
    if method != "GET" && method != "HEAD" {
        return respond(&mut sock, "405 Method Not Allowed", "text/plain", b"solo GET e POST").await;
    }

    match path {
        "/" | "/index.html" => {
            respond(&mut sock, "200 OK", "text/html; charset=utf-8", INDEX_HTML.as_bytes()).await
        }
        "/events" => events(sock, hub).await,
        "/wc.css" => respond(&mut sock, "200 OK", "text/css; charset=utf-8", WC_CSS.as_bytes()).await,
        // lingua dell'interfaccia per le pagine: tabella + traduttore
        "/i18n.js" => {
            let js = format!(
                "window.I18N_LANG={:?};window.I18N={};\n{I18N_JS}",
                crate::i18n::lang().code(),
                crate::i18n::table_json()
            );
            respond(&mut sock, "200 OK", "text/javascript; charset=utf-8", js.as_bytes()).await
        }
        "/chat" | "/chat.html" => {
            respond(&mut sock, "200 OK", "text/html; charset=utf-8", CHAT_HTML.as_bytes()).await
        }
        "/api/context" => {
            let j = match crate::ai::context() {
                Some(c) => serde_json::json!({
                    "title": c.title, "image": c.png.is_some(), "markdown": c.markdown, "doc": c.doc, "rev": c.rev,
                }),
                None => serde_json::json!({ "title": "", "image": false, "markdown": "", "rev": 0 }),
            };
            respond(&mut sock, "200 OK", "application/json", j.to_string().as_bytes()).await
        }
        "/context.png" => match crate::ai::context().and_then(|c| c.png) {
            Some(png) => respond(&mut sock, "200 OK", "image/png", &png).await,
            None => respond(&mut sock, "404 Not Found", "text/plain", b"nessuna pagina").await,
        },
        p if p.starts_with("/vendor/") => match crate::vendor::get(&p["/vendor/".len()..]) {
            Some((ctype, body)) => respond_cached(&mut sock, ctype, body).await,
            None => respond(&mut sock, "404 Not Found", "text/plain", b"non trovato").await,
        },
        "/appunti" | "/appunti.html" => {
            respond(&mut sock, "200 OK", "text/html; charset=utf-8", APPUNTI_HTML.as_bytes()).await
        }
        "/api/appunti" => {
            let list = tokio::task::spawn_blocking(list_appunti).await.unwrap_or_default();
            respond(&mut sock, "200 OK", "application/json", list.to_string().as_bytes()).await
        }
        p if p.starts_with("/api/appunti-file/") => match appunto_file(&p["/api/appunti-file/".len()..]) {
            Some((f, ctype)) => match std::fs::read(&f) {
                Ok(b) => respond(&mut sock, "200 OK", ctype, &b).await,
                Err(_) => respond(&mut sock, "404 Not Found", "text/plain", b"non trovato").await,
            },
            None => respond(&mut sock, "400 Bad Request", "text/plain", b"nome non valido").await,
        },
        p if p.starts_with("/api/appunti/") => match appunto_path(&p["/api/appunti/".len()..]) {
            Some(f) => match std::fs::read(&f) {
                Ok(b) => respond(&mut sock, "200 OK", "text/markdown; charset=utf-8", &b).await,
                Err(_) => respond(&mut sock, "404 Not Found", "text/plain", b"non trovato").await,
            },
            None => respond(&mut sock, "400 Bad Request", "text/plain", b"nome non valido").await,
        },
        "/api/chats" => {
            let j = tokio::task::spawn_blocking(crate::chats::list).await.unwrap_or_default();
            respond(&mut sock, "200 OK", "application/json", j.to_string().as_bytes()).await
        }
        p if p.starts_with("/api/chats/") => {
            let rest = &p["/api/chats/".len()..];
            match rest.split_once('/') {
                Some((id, "page.png")) if crate::chats::valid_id(id) => match std::fs::read(crate::chats::png_path(id)) {
                    Ok(b) => respond(&mut sock, "200 OK", "image/png", &b).await,
                    Err(_) => respond(&mut sock, "404 Not Found", "text/plain", b"nessuna pagina").await,
                },
                None => match crate::chats::get(rest) {
                    Some(b) => respond(&mut sock, "200 OK", "application/json", &b).await,
                    None => respond(&mut sock, "404 Not Found", "application/json", b"{}").await,
                },
                _ => respond(&mut sock, "404 Not Found", "text/plain", b"non trovato").await,
            }
        }
        "/favicon.ico" => respond(&mut sock, "204 No Content", "image/x-icon", b"").await,
        "/api/nav" => {
            let (rev, path) = NAV.lock().unwrap().clone();
            let j = serde_json::json!({ "rev": rev, "path": path });
            respond(&mut sock, "200 OK", "application/json", j.to_string().as_bytes()).await
        }
        "/impostazioni" | "/impostazioni.html" => {
            respond(&mut sock, "200 OK", "text/html; charset=utf-8", SETTINGS_HTML.as_bytes()).await
        }
        "/api/config" => {
            let j = tokio::task::spawn_blocking(crate::aiconfig::to_json)
                .await
                .unwrap_or_else(|e| serde_json::json!({ "error": e.to_string() }));
            respond(&mut sock, "200 OK", "application/json", j.to_string().as_bytes()).await
        }
        "/api/providers" => {
            let j = match tokio::task::spawn_blocking(crate::ai::AiConfig::load).await {
                Ok(Ok(cfg)) => serde_json::json!({
                    "providers": crate::ai::describe(&cfg)
                        .into_iter()
                        .map(|(n, s)| serde_json::json!({ "name": n, "state": s }))
                        .collect::<Vec<_>>()
                }),
                Ok(Err(e)) => serde_json::json!({ "error": e.to_string() }),
                Err(e) => serde_json::json!({ "error": e.to_string() }),
            };
            respond(&mut sock, "200 OK", "application/json", j.to_string().as_bytes()).await
        }
        "/state.json" => {
            let s = hub.snapshot_json();
            respond(&mut sock, "200 OK", "application/json", s.as_bytes()).await
        }
        "/bg.png" => {
            let bg = hub.lock().bg.clone();
            match bg {
                Some(b) => respond(&mut sock, "200 OK", "image/png", &b.png).await,
                None => respond(&mut sock, "404 Not Found", "text/plain", b"nessun documento").await,
            }
        }
        "/page.svg" => {
            let page = hub.lock().page();
            let s = export::page_to_svg(&page);
            respond(&mut sock, "200 OK", "image/svg+xml", s.as_bytes()).await
        }
        "/page.pdf" if hub.lock().bg.is_some() => {
            // Con un documento aperto il PDF vero si salva dall'app
            // («Salva PDF firmato»): qui si dà l'immagine della pagina.
            sock.write_all(b"HTTP/1.1 303 See Other\r\nLocation: /page.png\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .await
        }
        "/page.pdf" => {
            let page = hub.lock().page();
            let pdf = export::pages_to_pdf(&[&page]);
            respond(&mut sock, "200 OK", "application/pdf", &pdf).await
        }
        "/page.png" => {
            let page = hub.lock().page();
            match tokio::task::spawn_blocking(move || export::page_to_png(&page, 1600)).await {
                Ok(Ok(png)) => respond(&mut sock, "200 OK", "image/png", &png).await,
                _ => respond(&mut sock, "500 Internal Server Error", "text/plain", b"errore").await,
            }
        }
        _ => respond(&mut sock, "404 Not Found", "text/plain", b"non trovato").await,
    }
}

/// File che non cambiano (KaTeX, marked): il browser li tiene in memoria.
async fn respond_cached(sock: &mut TcpStream, ctype: &str, body: &[u8]) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\n\
         Cache-Control: public, max-age=604800\r\nConnection: close\r\n\r\n",
        body.len()
    );
    sock.write_all(head.as_bytes()).await?;
    sock.write_all(body).await?;
    sock.flush().await
}

async fn respond(sock: &mut TcpStream, status: &str, ctype: &str, body: &[u8]) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\n\
         Cache-Control: no-store\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
        body.len()
    );
    sock.write_all(head.as_bytes()).await?;
    sock.write_all(body).await?;
    sock.flush().await
}

/// Decodifica `%20` & co. di un pezzo di URL.
fn percent_decode(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let h = std::str::from_utf8(b.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(h, 16).ok()?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// File di appunti in exports/ dal nome nell'URL: solo `.md` nella cartella,
/// niente percorsi.
fn appunto_path(raw: &str) -> Option<std::path::PathBuf> {
    let name = percent_decode(raw)?;
    let ok = name.ends_with(".md")
        && !name.starts_with('.')
        && name.len() <= 200
        && !name.contains("..")
        && name.chars().all(|c| c.is_alphanumeric() || " ._-()".contains(c));
    ok.then(|| std::path::Path::new(export::EXPORT_DIR).join(name))
}

/// Immagini accanto agli appunti in exports/ (figure ridisegnate): solo
/// `.svg` e `.png` nella cartella, niente percorsi.
fn appunto_file(raw: &str) -> Option<(std::path::PathBuf, &'static str)> {
    let name = percent_decode(raw)?;
    let ctype = if name.ends_with(".svg") {
        "image/svg+xml"
    } else if name.ends_with(".png") {
        "image/png"
    } else {
        return None;
    };
    let ok = !name.starts_with('.')
        && name.len() <= 200
        && !name.contains("..")
        && name.chars().all(|c| c.is_alphanumeric() || " ._-()".contains(c));
    ok.then(|| (std::path::Path::new(export::EXPORT_DIR).join(name), ctype))
}

/// Elenco degli appunti (.md) in exports/, dal più recente.
fn list_appunti() -> serde_json::Value {
    let mut items = Vec::new();
    if let Ok(rd) = std::fs::read_dir(export::EXPORT_DIR) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if !name.ends_with(".md") || appunto_path(&name).is_none() {
                continue;
            }
            let Ok(meta) = e.metadata() else { continue };
            let mtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            // titolo: il primo titolo Markdown, altrimenti la prima riga
            let text = std::fs::read_to_string(e.path()).unwrap_or_default();
            let lines = text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with("<!--"));
            let first = lines.clone().find(|l| l.starts_with('#')).or(lines.clone().next()).unwrap_or("");
            let title: String = first.trim_start_matches('#').trim().chars().take(90).collect();
            items.push(serde_json::json!({ "name": name, "title": title, "mtime": mtime, "size": meta.len() }));
        }
    }
    items.sort_by(|a, b| b["mtime"].as_u64().cmp(&a["mtime"].as_u64()).then(a["name"].as_str().cmp(&b["name"].as_str())));
    serde_json::Value::Array(items)
}

/// Indirizzi della rete di casa (o del Mac stesso).
fn is_local(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => v.is_loopback() || v.is_private() || v.is_link_local(),
        IpAddr::V6(v) => {
            v.is_loopback()
                || (v.segments()[0] & 0xfe00) == 0xfc00
                || (v.segments()[0] & 0xffc0) == 0xfe80
                || v.to_ipv4_mapped().is_some_and(|v4| v4.is_loopback() || v4.is_private())
        }
    }
}

/// `POST /api/chat` con `{"messages":[{"role","content"}], "attach": bool}`:
/// la risposta arriva in streaming come eventi SSE
/// (`try`, `skip`, `delta`, `done`, `error`).
/// I byte di un'immagine incollata: `data:image/…;base64,…` (al massimo 12 MB).
fn data_url_bytes(u: &str) -> Option<std::sync::Arc<Vec<u8>>> {
    use base64::Engine;
    let (head, b64) = u.split_once(',')?;
    if !head.starts_with("data:image/") || !head.ends_with(";base64") || b64.len() > 16 * 1024 * 1024 {
        return None;
    }
    base64::engine::general_purpose::STANDARD.decode(b64.trim()).ok().map(std::sync::Arc::new)
}

async fn chat(mut sock: TcpStream, body: &[u8]) -> std::io::Result<()> {
    #[derive(serde::Deserialize)]
    struct In {
        role: String,
        content: String,
        /// Immagini incollate (data URL `data:image/png;base64,…`).
        #[serde(default)]
        images: Vec<String>,
    }
    #[derive(serde::Deserialize)]
    struct Req {
        messages: Vec<In>,
        #[serde(default, deserialize_with = "flag")]
        attach: bool,
        /// Strumenti (ricerca web, verifica P.IVA).
        #[serde(default = "si", deserialize_with = "flag")]
        web: bool,
        /// Conversazione salvata (per la sua pagina allegata).
        #[serde(default)]
        chat_id: String,
        /// Un agente preciso (nome in ai.toml); vuoto = la catena.
        #[serde(default)]
        provider: String,
        /// Appunto aperto nell'app (nome in exports/): l'AI ci può scrivere.
        #[serde(default)]
        note: String,
    }
    fn si() -> bool {
        true
    }
    /// Sì/no come arriva: true/false, 1/0, "1"/"true" (chat salvate da versioni vecchie).
    fn flag<'de, D: serde::Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
        let v = serde_json::Value::deserialize(d)?;
        Ok(match v {
            serde_json::Value::Bool(b) => b,
            serde_json::Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
            serde_json::Value::String(s) => matches!(s.trim(), "1" | "true" | "si" | "sì"),
            _ => false,
        })
    }
    use serde::Deserialize as _;
    let req: Req = match serde_json::from_slice(body) {
        Ok(r) => r,
        Err(e) => return respond(&mut sock, "400 Bad Request", "text/plain", e.to_string().as_bytes()).await,
    };
    sock.write_all(
        b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-store\r\n\
          Connection: close\r\n\r\n",
    )
    .await?;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<serde_json::Value>();
    let history: Vec<(String, String, Vec<std::sync::Arc<Vec<u8>>>)> =
        req.messages.into_iter().map(|m| (m.role, m.content, m.images.iter().take(8).filter_map(|u| data_url_bytes(u)).collect())).collect();
    let attach = req.attach;
    let use_tools = req.web;
    let chat_id = req.chat_id.clone();
    let only = req.provider.trim().to_string();
    let appunto = Some(req.note.trim()).filter(|n| !n.is_empty()).and_then(appunto_path).filter(|p| p.exists());
    let worker = tokio::task::spawn_blocking(move || {
        use crate::ai::{self, Event};
        let mut cfg = match ai::AiConfig::load() {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(serde_json::json!({ "t": "error", "msg": e.to_string() }));
                return;
            }
        };
        if !only.is_empty() {
            // risponde solo l'agente scelto; gli altri restano per leggere il foglio
            if !cfg.providers.iter().any(|p| p.name == only) {
                let _ = tx.send(serde_json::json!({ "t": "error", "msg": format!("agente «{only}» non trovato in ai.toml") }));
                return;
            }
            cfg.answer_with = Some(only.clone());
        }
        let ctx = match (attach, chat_id.is_empty()) {
            (false, _) => None,
            (true, false) => crate::chats::page_context(&chat_id),
            (true, true) => ai::context(),
        };
        let msgs = ai::chat_messages_with_images(&cfg, &history, ctx.as_ref());
        let res = ai::run_chat(&cfg, ai::Task::Chat, &msgs, use_tools, appunto, &mut |ev| {
            let j = match ev {
                Event::Trying { provider, model, context } => serde_json::json!({ "t": "try", "provider": provider, "model": model, "context": context }),
                Event::Usage { used, max, exact, output } => serde_json::json!({ "t": "usage", "used": used, "max": max, "exact": exact, "output": output }),
                Event::Thinking(d) => serde_json::json!({ "t": "think", "text": d }),
                // i testi dell'app (non quelli del modello) nella lingua dell'interfaccia
                Event::Skipped { provider, why } => serde_json::json!({ "t": "skip", "provider": provider, "why": crate::i18n::tf(why) }),
                Event::Delta(d) => serde_json::json!({ "t": "delta", "text": d }),
                // `key` = il testo italiano, per riconoscere lo strumento nella pagina
                Event::Tool { label } => serde_json::json!({ "t": "tool", "key": label.clone(), "label": crate::i18n::tf(label) }),
                Event::ToolDone { label, result } => serde_json::json!({ "t": "tooldone", "key": label.clone(), "label": crate::i18n::tf(label), "result": crate::i18n::tf(result) }),
            };
            // false = il browser ha chiuso: si interrompe
            tx.send(j).is_ok()
        });
        let _ = tx.send(match res {
            Ok(r) => serde_json::json!({ "t": "done", "provider": r.provider, "model": r.model }),
            Err(e) => serde_json::json!({ "t": "error", "msg": crate::i18n::tf(e.to_string()) }),
        });
    });
    let mut ping = tokio::time::interval(Duration::from_secs(15));
    ping.tick().await;
    loop {
        tokio::select! {
            m = rx.recv() => match m {
                Some(j) => {
                    if let Err(e) = send_event(&mut sock, &j.to_string()).await {
                        drop(rx); // il lavoro in corso se ne accorge e si ferma
                        let _ = worker.await;
                        return Err(e);
                    }
                }
                None => break,
            },
            _ = ping.tick() => {
                sock.write_all(b": ping\n\n").await?;
                sock.flush().await?;
            }
        }
    }
    let _ = worker.await;
    sock.shutdown().await
}

/// Flusso SSE: stato completo, poi i tratti man mano.
async fn events(mut sock: TcpStream, hub: Arc<Hub>) -> std::io::Result<()> {
    sock.write_all(
        b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-store\r\n\
          Access-Control-Allow-Origin: *\r\nConnection: keep-alive\r\n\r\nretry: 2000\n\n",
    )
    .await?;
    // Prima ci si iscrive, poi si manda lo stato: così non si perde nulla.
    let mut rx = hub.subscribe();
    send_event(&mut sock, &hub.snapshot_json()).await?;
    let mut ping = tokio::time::interval(Duration::from_secs(15));
    ping.tick().await;
    loop {
        tokio::select! {
            m = rx.recv() => match m {
                Ok(WebMsg::Json(s)) => send_event(&mut sock, &s).await?,
                Ok(WebMsg::Reset) | Err(RecvError::Lagged(_)) => {
                    // Lo stato completo include già tutto ciò che è in coda.
                    while rx.try_recv().is_ok() {}
                    send_event(&mut sock, &hub.snapshot_json()).await?
                }
                Err(RecvError::Closed) => return Ok(()),
            },
            _ = ping.tick() => {
                sock.write_all(b": ping\n\n").await?;
                sock.flush().await?;
            }
        }
    }
}

async fn send_event(sock: &mut TcpStream, json: &str) -> std::io::Result<()> {
    let mut msg = Vec::with_capacity(json.len() + 8);
    msg.extend_from_slice(b"data: ");
    msg.extend_from_slice(json.as_bytes());
    msg.extend_from_slice(b"\n\n");
    sock.write_all(&msg).await?;
    sock.flush().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Scene;
    use crate::settings::Settings;
    use crate::stroke::{PenEvent, PenPhase};

    fn ev(x: f32, y: f32, phase: PenPhase) -> PenEvent {
        PenEvent { x, y, pressure: 0.5, phase }
    }

    async fn get(port: u16, path: &str) -> String {
        let mut s = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        s.write_all(format!("GET {path} HTTP/1.1\r\nHost: x\r\n\r\n").as_bytes()).await.unwrap();
        let mut out = Vec::new();
        s.read_to_end(&mut out).await.unwrap();
        String::from_utf8_lossy(&out).into_owned()
    }

    #[tokio::test]
    async fn pagina_stato_ed_eventi() {
        let hub = Hub::new(Scene::new(&Settings::default()));
        let url = start(hub.clone(), 0).await.unwrap();
        let port: u16 = url.rsplit(':').next().unwrap().trim_end_matches('/').parse().unwrap();

        let index = get(port, "/").await;
        assert!(index.starts_with("HTTP/1.1 200") && index.contains("EventSource"));

        // client SSE
        let mut sse = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        sse.write_all(b"GET /events HTTP/1.1\r\n\r\n").await.unwrap();
        let mut buf = vec![0u8; 4096];
        let mut got = String::new();
        while !got.contains("\"t\":\"snap\"") {
            let n = sse.read(&mut buf).await.unwrap();
            got.push_str(&String::from_utf8_lossy(&buf[..n]));
        }
        hub.pen_event(ev(1000.0, 1000.0, PenPhase::Down));
        hub.pen_event(ev(2000.0, 1500.0, PenPhase::Move));
        hub.pen_event(ev(2000.0, 1500.0, PenPhase::Up));
        while !got.contains("\"t\":\"up\"") {
            let n = sse.read(&mut buf).await.unwrap();
            got.push_str(&String::from_utf8_lossy(&buf[..n]));
        }
        assert!(got.contains("\"t\":\"down\"") && got.contains("\"t\":\"move\""));

        let state = get(port, "/state.json").await;
        assert!(state.contains("\"s\":[{\"c\":\"#141414\""));
        let pdf = get(port, "/page.pdf").await;
        assert!(pdf.contains("application/pdf") && pdf.contains("%PDF-1.4"));
    }

    async fn post(port: u16, path: &str, body: &str) -> String {
        let mut s = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        s.write_all(
            format!("POST {path} HTTP/1.1\r\nHost: x\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", body.len())
                .as_bytes(),
        )
        .await
        .unwrap();
        let mut out = Vec::new();
        s.read_to_end(&mut out).await.unwrap();
        String::from_utf8_lossy(&out).into_owned()
    }

    /// Scrive un ai.toml di prova che punta a un finto provider.
    fn fake_ai_config(port: u16) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("ws-ai-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ai.toml");
        std::fs::write(
            &path,
            format!(
                "[[provider]]\nname = \"Spento\"\nurl = \"http://127.0.0.1:9/v1\"\nmodel = \"x\"\n\n\
                 [[provider]]\nname = \"Finto\"\nurl = \"http://127.0.0.1:{port}/v1\"\nmodel = \"finto-vl\"\n"
            ),
        )
        .unwrap();
        std::env::set_var("WACOM_AI_CONFIG", &path);
        path
    }

    #[test]
    fn nomi_degli_appunti() {
        assert!(appunto_path("appunti-20261019-115120.md").is_some());
        assert_eq!(appunto_path("chat%20Es.%203.md").unwrap(), std::path::Path::new("exports/chat Es. 3.md"));
        for bad in ["../x.md", "a/b.md", "%2e%2e%2fsettings.md", "x.txt", ".nascosto.md", "a%2Fb.md"] {
            assert!(appunto_path(bad).is_none(), "{bad}");
        }
    }

    #[tokio::test]
    async fn chat_in_streaming() {
        let (aport, log) = crate::ai::tests::fake_server(|_| {
            crate::ai::tests::sse_reply(&["Al passaggio 2 ", "manca il segno: $-\\frac{1}{x^2}$."])
        });
        fake_ai_config(aport);
        crate::ai::set_context("Es. 1", Some(Arc::new(b"\x89PNG".to_vec())), "$f(x)=1/x$");
        let hub = Hub::new(Scene::new(&Settings::default()));
        let url = start(hub, 0).await.unwrap();
        let port: u16 = url.rsplit(':').next().unwrap().trim_end_matches('/').parse().unwrap();

        assert!(get(port, "/chat").await.contains("vendor/katex.min.js"));
        assert!(get(port, "/vendor/katex.min.js").await.contains("Cache-Control: public"));
        assert!(get(port, "/vendor/fonts/KaTeX_Main-Regular.woff2").await.contains("font/woff2"));
        assert!(get(port, "/appunti").await.contains("api/appunti"));
        let c = get(port, "/api/context").await;
        assert!(c.contains("\"title\":\"Es. 1\"") && c.contains("\"image\":true"));
        assert!(get(port, "/context.png").await.contains("image/png"));
        assert!(get(port, "/api/providers").await.contains("Finto"));

        let out = post(port, "/api/chat", r#"{"messages":[{"role":"user","content":"è giusto?"}],"attach":true}"#).await;
        assert!(out.contains("text/event-stream"), "{out}");
        let evs: Vec<serde_json::Value> = out
            .lines()
            .filter_map(|l| l.strip_prefix("data: "))
            .map(|d| serde_json::from_str(d).unwrap())
            .collect();
        let kinds: Vec<&str> = evs.iter().map(|e| e["t"].as_str().unwrap()).collect();
        assert_eq!(kinds, ["try", "skip", "try", "delta", "delta", "usage", "done"], "{out}");
        let usage = evs.iter().find(|e| e["t"] == "usage").unwrap();
        assert!(usage["used"].as_u64().unwrap() > 10 && usage["max"] == 32768 && usage["exact"] == false);
        assert_eq!(evs[1]["provider"], "Spento");
        assert_eq!(evs[2]["model"], "finto-vl");
        let text: String = evs.iter().filter_map(|e| e["text"].as_str()).collect();
        assert_eq!(text, "Al passaggio 2 manca il segno: $-\\frac{1}{x^2}$.");
        let req = log.lock().unwrap().last().unwrap().clone();
        assert!(req.contains("data:image/png;base64,iVBORw") && req.contains("$f(x)=1/x$") && req.contains("assistente versatile"));
        // richiesta malformata
        assert!(post(port, "/api/chat", "boh").await.starts_with("HTTP/1.1 400"));
    }

    /// Chat dimostrativa con un finto agente (per provarla nel browser):
    /// `cargo test serve_chat_demo -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn serve_chat_demo() {
        let (aport, _) = crate::ai::tests::fake_server(|_| {
            std::thread::sleep(Duration::from_millis(300));
            crate::ai::tests::sse_reply(&[
                "Il passaggio 2 contiene un errore di segno.\n\n",
                "La derivata corretta è\n\n$$f'(x) = -\\frac{1}{x^2}, \\quad x \\neq 0$$\n\n",
                "mentre sul foglio hai scritto $f'(x) = \\frac{1}{x^2}$. ",
                "Per la matrice:\n\n$$A = \\begin{pmatrix} 1 & 2 \\\\ 3 & 4 \\end{pmatrix}, \\quad \\det A = -2$$",
            ])
        });
        fake_ai_config(aport);
        let hub = Hub::new(Scene::new(&Settings::default()));
        let page = hub.lock().page();
        let png = export::page_to_png(&page, 600).unwrap();
        crate::ai::set_context(
            "Esercizio 3 — derivate",
            Some(Arc::new(png)),
            "## Esercizio 3\n$f(x) = \\frac{1}{x}$\n\n$$f'(x) = \\frac{1}{x^2}$$",
        );
        let url = start(hub, 18081).await.unwrap();
        println!("chat su {url}chat");
        tokio::time::sleep(Duration::from_secs(120)).await;
    }

    /// Server dimostrativo per provare la pagina nel browser:
    /// `cargo test serve_demo -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn serve_demo() {
        let hub = Hub::new(Scene::new(&Settings::default()));
        hub.edit(|s| {
            s.drawing.width = 29700.0;
            s.drawing.height = 21000.0;
        });
        let url = start(hub.clone(), 18080).await.unwrap();
        println!("demo su {url}");
        // Una scritta pre-esistente e poi un tratto "dal vivo" a colori.
        let draw = |pts: &[(f32, f32)], color| {
            hub.lock().pen.color = color;
            for (i, &(x, y)) in pts.iter().enumerate() {
                let ph = if i == 0 { PenPhase::Down } else { PenPhase::Move };
                hub.pen_event(ev(x, y, ph));
            }
            let (x, y) = *pts.last().unwrap();
            hub.pen_event(ev(x, y, PenPhase::Up));
        };
        let wave: Vec<(f32, f32)> = (0..60).map(|i| (3000.0 + i as f32 * 300.0, 4000.0 + 1500.0 * (i as f32 / 6.0).sin())).collect();
        draw(&wave, [20, 20, 20]);
        tokio::time::sleep(Duration::from_secs(3)).await;
        let circle: Vec<(f32, f32)> = (0..=80)
            .map(|i| {
                let a = i as f32 / 80.0 * std::f32::consts::TAU;
                (15000.0 + 4000.0 * a.cos(), 14000.0 + 4000.0 * a.sin())
            })
            .collect();
        // tratto lento, punto per punto
        hub.lock().pen.color = [210, 40, 40];
        for (i, &(x, y)) in circle.iter().enumerate() {
            hub.pen_event(ev(x, y, if i == 0 { PenPhase::Down } else { PenPhase::Move }));
            tokio::time::sleep(Duration::from_millis(40)).await;
        }
        hub.pen_event(ev(15000.0 + 4000.0, 14000.0, PenPhase::Up));
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}
