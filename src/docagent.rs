//! La chat AI dentro il documento aperto accanto: lo legge e lo modifica coi
//! server MCP della suite Craft di ArtCraft (MIT OR Apache-2.0), in modalità
//! headless dentro l'app, come farebbero Claude o DSH con quei server:
//!
//! - Word (.docx, .doc, .odt, .rtf): il server MCP di WordCraft;
//! - PDF: quello di PdfCraft;
//! - SVG: quello di VectorCraft;
//! - Markdown, .txt, .html: strumenti sul testo (il Markdown lo impagina
//!   `mdcraft`);
//! - foto: solo l'inserimento di immagini (con VectorCraft).
//!
//! Più uno strumento comune, `document_insert_image`, che mette nel documento
//! un grafico fatto da `draw_chart` (o un'immagine).
//!
//! Ogni modifica si applica subito al file del documento (la copia che
//! OpenSlate tiene nella sua cartella, mai l'originale sul disco), le pagine
//! si ridisegnano e la vista si aggiorna; prima si fa una copia, così la chat
//! mostra «Annulla» e si torna indietro.

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Che cosa sa fare la chat con un documento.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Word,
    Text,
    Pdf,
    Svg,
    Photo,
}

/// Il documento su cui lavora la chat: la sua cartella e il file che si modifica.
#[derive(Clone, Debug)]
pub struct DocRef {
    pub dir: PathBuf,
    pub file: PathBuf,
    pub kind: Kind,
    /// Pagina aperta (da 0), per le immagini nei PDF.
    pub page: usize,
}

impl DocRef {
    /// Il documento nella cartella `dir` (`documents/<id>`).
    pub fn of(dir: &Path, page: usize) -> Option<DocRef> {
        let meta: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("doc.json")).ok()?).ok()?;
        let (file, kind) = match meta["original"].as_str() {
            Some(name) => {
                let p = Path::new(name);
                let kind = if crate::wordsign::is_text(p) {
                    Kind::Text
                } else if crate::wordsign::is_word(p) {
                    Kind::Word
                } else if crate::vecsign::is_image(p) {
                    Kind::Svg
                } else if crate::rasign::is_raster(p) {
                    Kind::Photo
                } else {
                    return None;
                };
                (dir.join(name), kind)
            }
            None => (dir.join("source.pdf"), Kind::Pdf),
        };
        file.exists().then(|| DocRef { dir: dir.to_path_buf(), file, kind, page })
    }

    fn prefix(&self) -> &'static str {
        match self.kind {
            Kind::Word => "word__",
            Kind::Pdf => "pdf__",
            Kind::Svg => "svg__",
            Kind::Photo => "photo__",
            Kind::Text => "",
        }
    }
}

/// Esito di uno strumento.
#[derive(Default)]
pub struct Outcome {
    /// Per il modello.
    pub text: String,
    /// Esito breve per la chat.
    pub short: String,
    /// Immagini restituite (es. una pagina disegnata): la chat le mostra e,
    /// se il modello vede le immagini, le riceve.
    pub images: Vec<Vec<u8>>,
    /// Il documento è cambiato: numero della copia per «Annulla».
    pub edit: Option<u64>,
}

// ---------------------------------------------------------------------------
// Aggiornare la vista
// ---------------------------------------------------------------------------

type Notify = Box<dyn Fn(PathBuf) + Send + Sync>;
static NOTIFY: OnceLock<Notify> = OnceLock::new();

/// L'interfaccia si fa avvisare quando la chat cambia un documento.
pub fn on_change(f: impl Fn(PathBuf) + Send + Sync + 'static) {
    let _ = NOTIFY.set(Box::new(f));
}

/// La chat ha cambiato il foglio della lavagna (avvisa l'interfaccia).
pub fn board_changed() {
    if let Some(n) = NOTIFY.get() {
        n(PathBuf::from("lavagna"));
    }
}

/// Dopo una modifica: pagine ridisegnate, l'app avvisata. Se il numero di
/// pagine è cambiato e sul documento c'era inchiostro, ritorna l'avviso da
/// dare all'utente (le firme stanno sulle pagine per numero).
fn changed(doc: &DocRef) -> Option<String> {
    let id = doc.dir.file_name().and_then(|n| n.to_str()).map(String::from);
    // prima: dalle pagine salvate in doc.json (il file è già cambiato, per un PDF anche source.pdf)
    let before = ink_pages(&doc.dir);
    if let Err(e) = crate::document::rebuild(&doc.dir) {
        tracing::warn!("pagine del documento dopo la modifica: {e:#}");
    }
    // le pagine si ridisegnano qui, non nell'interfaccia
    let after = id.as_deref().and_then(|id| match crate::document::Document::open(id) {
        Ok(d) => Some(d),
        Err(e) => {
            tracing::warn!("pagine del documento dopo la modifica: {e:#}");
            None
        }
    });
    if let Some(n) = NOTIFY.get() {
        n(doc.dir.clone());
    }
    let ((n_before, ink), a) = (before?, after?);
    (ink && n_before != a.pages.len()).then(|| {
        format!(
            "ATTENZIONE: il documento è passato da {} a {} pagine e l'utente ci aveva scritto sopra (firme o note a mano, \
             legate al numero della pagina): dillo all'utente, che controlli dove sono finite; con «Annulla le modifiche» torna tutto com'era.",
            n_before,
            a.pages.len()
        )
    })
}

/// Da doc.json: quante pagine aveva il documento e se c'era inchiostro.
fn ink_pages(dir: &Path) -> Option<(usize, bool)> {
    let v: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("doc.json")).ok()?).ok()?;
    let pages = v["pages"].as_array()?;
    let has = |a: &Value| a.as_array().is_some_and(|ps| ps.iter().any(|p| p.as_array().is_some_and(|s| !s.is_empty())));
    Some((pages.len(), has(&v["pages"]) || has(&v["extra"])))
}

// ---------------------------------------------------------------------------
// Annulla
// ---------------------------------------------------------------------------

fn undo_dir(dir: &Path, n: u64) -> PathBuf {
    dir.join("ai-undo").join(n.to_string())
}

/// Copia del documento prima di una modifica; ritorna il suo numero.
fn snapshot(doc: &DocRef) -> Result<u64> {
    let base = doc.dir.join("ai-undo");
    std::fs::create_dir_all(&base)?;
    let n = std::fs::read_dir(&base)?.filter_map(|e| e.ok()?.file_name().to_str()?.parse::<u64>().ok()).max().unwrap_or(0) + 1;
    let d = undo_dir(&doc.dir, n);
    std::fs::create_dir_all(&d)?;
    // (l'inchiostro, in doc.json, non si tocca: se le pagine cambiano resta
    // da parte e torna quando tornano)
    for f in [doc.file.clone(), doc.dir.join("source.pdf")] {
        if f.exists() {
            std::fs::copy(&f, d.join(f.file_name().context("nome del file")?))?;
        }
    }
    // le figure aggiunte a un Markdown restano: il testo che le cita torna com'era
    Ok(n)
}

/// «Annulla» della chat: il documento torna com'era prima della modifica `n`
/// (e di quelle dopo).
pub fn undo(dir: &Path, n: u64) -> Result<()> {
    let d = undo_dir(dir, n);
    if !d.is_dir() {
        bail!("questa modifica non si può più annullare");
    }
    // il documento com'è adesso resta da parte (ai-undo/annullato-<ora>/)
    let keep = dir.join("ai-undo").join(format!("annullato-{}", chrono::Local::now().format("%Y%m%d-%H%M%S")));
    if std::fs::create_dir_all(&keep).is_ok() {
        for e in std::fs::read_dir(&d)?.flatten() {
            let cur = dir.join(e.file_name());
            if cur.exists() {
                let _ = std::fs::copy(&cur, keep.join(e.file_name()));
            }
        }
    }
    for e in std::fs::read_dir(&d)? {
        let e = e?;
        std::fs::copy(e.path(), dir.join(e.file_name()))?;
    }
    // le copie di questa modifica e delle successive non servono più
    for e in std::fs::read_dir(dir.join("ai-undo"))?.flatten() {
        if e.file_name().to_str().and_then(|s| s.parse::<u64>().ok()).is_some_and(|k| k >= n) {
            let _ = std::fs::remove_dir_all(e.path());
        }
    }
    drop_session(dir);
    let doc = DocRef::of(dir, 0).context("documento non trovato")?;
    let _ = changed(&doc);
    Ok(())
}

// ---------------------------------------------------------------------------
// Server MCP della suite Craft, in memoria
// ---------------------------------------------------------------------------

enum Server {
    Word(wordcraft_mcp::Server),
    Svg(vectorcraft_mcp::Server),
    Pdf(pdfcraft_automation::mcp::McpServer, u64),
    Photo(PhotoMcp),
}

/// Il server MCP di PhotoCraft gira con tokio (rmcp): un runtime suo e due
/// tubi in memoria, una riga JSON per messaggio, come sullo stdio.
struct PhotoMcp {
    rt: tokio::runtime::Runtime,
    w: tokio::io::DuplexStream,
    r: tokio::io::Lines<tokio::io::BufReader<tokio::io::DuplexStream>>,
}

impl PhotoMcp {
    fn start(root: &Path) -> Result<PhotoMcp> {
        use tokio::io::AsyncBufReadExt;
        let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(1).thread_name("photocraft-mcp").enable_all().build()?;
        // legge e scrive solo nella cartella del documento
        let ws = photocraft_automation::AuthorizedWorkspace::new(Some(root), Some(root)).map_err(|e| anyhow!("PhotoCraft: {e}"))?;
        let server = photocraft_automation::PhotocraftMcp::headless_with_workspace(ws);
        let (w, server_in) = tokio::io::duplex(1 << 22);
        let (server_out, r) = tokio::io::duplex(1 << 24);
        rt.spawn(async move {
            if let Err(e) = server.serve_io(server_in, server_out).await {
                tracing::warn!("PhotoCraft MCP: {e}");
            }
        });
        Ok(PhotoMcp { rt, w, r: tokio::io::BufReader::new(r).lines() })
    }

    /// Manda un messaggio; per una richiesta aspetta la sua risposta (le
    /// notifiche del server in mezzo si saltano).
    fn handle(&mut self, msg: Value) -> Option<Value> {
        use tokio::io::AsyncWriteExt;
        let PhotoMcp { rt, w, r } = self;
        let id = msg.get("id").cloned();
        rt.block_on(async {
            w.write_all(format!("{msg}\n").as_bytes()).await.ok()?;
            w.flush().await.ok()?;
            let id = id?;
            loop {
                let line = tokio::time::timeout(std::time::Duration::from_secs(170), r.next_line()).await.ok()?.ok()??;
                let v: Value = serde_json::from_str(&line).ok()?;
                if v.get("id") == Some(&id) {
                    return Some(v);
                }
            }
        })
    }
}

struct Session {
    dir: PathBuf,
    server: Server,
    /// Strumenti offerti (già filtrati), nel formato MCP.
    tools: Vec<Value>,
    /// Data di modifica del file quando lo si è aperto o salvato.
    mtime: Option<std::time::SystemTime>,
    next_id: u64,
}

/// I server MCP non si spostano fra thread: vivono tutti in un thread loro,
/// che esegue i lavori che gli si mandano.
type Job = Box<dyn FnOnce(&mut Option<Session>) + Send>;

fn session_thread() -> &'static Mutex<std::sync::mpsc::Sender<Job>> {
    static TX: OnceLock<Mutex<std::sync::mpsc::Sender<Job>>> = OnceLock::new();
    TX.get_or_init(|| Mutex::new(new_session_thread()))
}

/// Il thread dei server MCP. Un lavoro che va in crash azzera la sessione
/// (si riapre al prossimo), il thread resta.
fn new_session_thread() -> std::sync::mpsc::Sender<Job> {
    let (tx, rx) = std::sync::mpsc::channel::<Job>();
    let _ = std::thread::Builder::new().name("documento-mcp".into()).spawn(move || {
        let mut session: Option<Session> = None;
        while let Ok(job) = rx.recv() {
            if crate::platform::guarded(|| job(&mut session)).is_err() {
                session = None;
            }
        }
    });
    tx
}

/// Tempo massimo di un lavoro sul documento (un file che fa impazzire
/// l'impaginazione non deve bloccare la chat per sempre).
const JOB_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(180);

/// Esegue `f` nel thread dei server MCP e ne aspetta il risultato.
fn on_session_thread<T: Send + 'static>(f: impl FnOnce(&mut Option<Session>) -> T + Send + 'static) -> Result<T> {
    let (tx, rx) = std::sync::mpsc::channel();
    let job: Job = Box::new(move |s| {
        let _ = tx.send(f(s));
    });
    session_thread().lock().unwrap_or_else(|e| e.into_inner()).send(job).map_err(|_| anyhow!("server MCP fermi"))?;
    match rx.recv_timeout(JOB_TIMEOUT) {
        Ok(v) => Ok(v),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            // (il lavoro continua nel suo thread: non se ne apre un altro sullo
            // stesso file, che scriverebbero in due)
            bail!("il documento non risponde da {} s: riprova fra poco", JOB_TIMEOUT.as_secs())
        }
        Err(_) => bail!("server MCP: lavoro interrotto (errore interno, vedi crash.log)"),
    }
}

fn mtime(p: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(p).ok()?.modified().ok()
}

fn drop_session(dir: &Path) {
    let dir = dir.to_path_buf();
    let _ = on_session_thread(move |g| {
        if g.as_ref().is_some_and(|s| s.dir == dir) {
            *g = None;
        }
    });
}

/// Strumenti di ogni server che la chat usa (gli altri riguardano finestre,
/// file fuori dal documento o lavori di massa).
fn allowed(kind: Kind, name: &str) -> bool {
    match kind {
        Kind::Word => matches!(name, "list_commands" | "execute" | "batch" | "type_text" | "select_text" | "inspect_document" | "render_page"),
        Kind::Svg => matches!(
            name,
            "list_commands" | "command_list" | "command_batch" | "render_preview" | "draw_path" | "draw_shape" | "set_paint" | "add_text" | "transform"
                | "create_graph" | "apply_effect" | "pathfinder" | "text_wrap" | "undo" | "redo"
        ),
        Kind::Pdf => matches!(
            name,
            "doc_info" | "doc_inspect" | "page_render" | "text_extract" | "text_find" | "text_lines" | "text_paragraphs" | "text_word_count" | "text_edit"
                | "content_list" | "content_update" | "content_delete" | "object_list" | "object_move" | "comment_list" | "comment_edit"
                | "comment_delete" | "comment_reply" | "link_list" | "link_add" | "page_insert_blank" | "page_delete" | "page_rotate"
                | "page_move" | "page_duplicate" | "page_number" | "doc_watermark" | "doc_background" | "edit_undo" | "edit_redo"
                | "command_list" | "command_run" | "command_batch"
                // i moduli: leggere e compilare i campi, crearli nei moduli stampati, scrivere dove non ce ne sono
                | "form_fields" | "form_fill" | "form_reset" | "form_detect_fields" | "fill_sign_add"
        ),
        // aprire, salvare ed esportare li fa l'app; niente finestre
        Kind::Photo => matches!(name, "doc_inspect" | "doc_render_preview" | "render_preview" | "command_list" | "command_run" | "command_batch"),
        Kind::Text => false,
    }
}

impl Session {
    fn rpc(&mut self, method: &str, params: Value) -> Result<Value> {
        self.next_id += 1;
        let msg = json!({ "jsonrpc": "2.0", "id": self.next_id, "method": method, "params": params });
        let reply = match &mut self.server {
            Server::Word(s) => s.handle(msg),
            Server::Svg(s) => s.handle(msg),
            Server::Pdf(s, _) => s.handle(&msg),
            Server::Photo(s) => s.handle(msg),
        };
        let reply = reply.ok_or_else(|| anyhow!("{method}: nessuna risposta"))?;
        if let Some(e) = reply.get("error") {
            bail!("{}", e["message"].as_str().unwrap_or(&e.to_string()));
        }
        Ok(reply["result"].clone())
    }

    fn notify(&mut self, method: &str) {
        let msg = json!({ "jsonrpc": "2.0", "method": method });
        match &mut self.server {
            Server::Word(s) => {
                s.handle(msg);
            }
            Server::Svg(s) => {
                s.handle(msg);
            }
            Server::Pdf(s, _) => {
                s.handle(&msg);
            }
            Server::Photo(s) => {
                s.handle(msg);
            }
        }
    }

    /// Apre il documento nel server giusto e ne prende gli strumenti.
    fn open(doc: &DocRef) -> Result<Session> {
        let root = doc.dir.canonicalize()?;
        let file = doc.file.canonicalize()?;
        let server = match doc.kind {
            Kind::Word => {
                let roots = wordcraft_engine::file_access::AutomationRoots::new(Some(&root), Some(&root)).map_err(|e| anyhow!(e))?;
                let mut h = wordcraft_mcp::Headless::default().with_automation_roots(roots);
                h.session = crate::wordsign::open(&file)?;
                Server::Word(wordcraft_mcp::Server::new(Box::new(h)))
            }
            Kind::Svg => {
                let roots = vectorcraft_engine::file_access::AutomationRoots::new(&[&root], &[&root]).map_err(|e| anyhow!(e))?;
                let mut h = vectorcraft_mcp::Headless::new().with_automation_roots(roots);
                h.open(&json!({ "path": file.to_string_lossy() })).map_err(|e| anyhow!("VectorCraft: {e}"))?;
                Server::Svg(vectorcraft_mcp::Server::new(Box::new(h)))
            }
            Kind::Pdf => {
                let a = pdfcraft_automation::Automation::new().with_root(&root)?;
                Server::Pdf(pdfcraft_automation::mcp::McpServer::new(a), 0)
            }
            Kind::Photo => Server::Photo(PhotoMcp::start(&root)?),
            Kind::Text => bail!("nessun server MCP per questo documento"),
        };
        let mut s = Session { dir: doc.dir.clone(), server, tools: Vec::new(), mtime: mtime(&doc.file), next_id: 0 };
        s.rpc("initialize", json!({ "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "OpenSlate", "version": crate::update::current() } }))?;
        s.notify("notifications/initialized");
        if doc.kind == Kind::Pdf {
            let r = s.rpc("tools/call", json!({ "name": "doc_open", "arguments": { "path": file.to_string_lossy() } }))?;
            let text = result_text(&r);
            let id = serde_json::from_str::<Value>(&text).ok().and_then(|v| v["doc"].as_u64().or(v["id"].as_u64())).unwrap_or(1);
            if let Server::Pdf(_, d) = &mut s.server {
                *d = id;
            }
        }
        if doc.kind == Kind::Photo {
            let r = s.rpc("tools/call", json!({ "name": "doc_open", "arguments": { "path": file.to_string_lossy() } }))?;
            if r["isError"].as_bool() == Some(true) {
                bail!("PhotoCraft non apre la foto: {}", result_text(&r));
            }
        }
        let list = s.rpc("tools/list", json!({}))?;
        s.tools = list["tools"].as_array().cloned().unwrap_or_default().into_iter().filter(|t| t["name"].as_str().is_some_and(|n| allowed(doc.kind, n))).collect();
        Ok(s)
    }

    /// Scrive il documento sul suo file.
    fn save(&mut self, doc: &DocRef) -> Result<()> {
        let path = doc.file.canonicalize()?.to_string_lossy().to_string();
        match &mut self.server {
            Server::Word(s) => {
                s.backend().call("file.save", json!({ "path": path })).map_err(|e| anyhow!("WordCraft: {e}"))?;
            }
            Server::Svg(s) => {
                let format = if path.to_ascii_lowercase().ends_with(".svgz") { "svgz" } else { "svg" };
                s.backend().call("app.save", json!({ "path": path, "format": format, "acknowledgeLoss": true })).map_err(|e| anyhow!("VectorCraft: {e}"))?;
            }
            Server::Pdf(_, id) => {
                let id = *id;
                self.rpc("tools/call", json!({ "name": "doc_save", "arguments": { "doc": id, "path": path } }))?;
            }
            Server::Photo(_) => {
                // nello stesso formato, il JPEG con la sua qualità; prima in un
                // file accanto, poi al posto dell'originale (mai mezzo scritto)
                let ext = doc.file.extension().and_then(|e| e.to_str()).unwrap_or("png").to_ascii_lowercase();
                let name = format!("salvataggio-ai.{ext}");
                let tmp = doc.dir.join(&name);
                // (la cartella del documento è la radice del server: il nome basta)
                let mut args = json!({ "path": name });
                if matches!(ext.as_str(), "jpg" | "jpeg") {
                    args["quality"] = json!(crate::rasign::jpeg_quality(&doc.file).unwrap_or(92));
                }
                let r = self.rpc("tools/call", json!({ "name": "doc_save", "arguments": args }))?;
                if r["isError"].as_bool() == Some(true) {
                    let _ = std::fs::remove_file(&tmp);
                    bail!("PhotoCraft non salva la foto: {}", result_text(&r));
                }
                std::fs::rename(&tmp, &doc.file)?;
            }
        }
        self.mtime = mtime(&doc.file);
        Ok(())
    }
}

/// Il testo di un risultato MCP (`content` di tipo testo).
fn result_text(r: &Value) -> String {
    r["content"].as_array().into_iter().flatten().filter_map(|c| c["text"].as_str()).collect::<Vec<_>>().join("\n")
}

/// Le immagini di un risultato MCP.
fn result_images(r: &Value) -> Vec<Vec<u8>> {
    use base64::Engine;
    r["content"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|c| c["type"] == "image")
        .filter_map(|c| base64::engine::general_purpose::STANDARD.decode(c["data"].as_str()?).ok())
        .collect()
}

/// Con la sessione del documento (aperta ora o riusata se il file non è
/// cambiato da fuori).
fn with_session<T: Send + 'static>(doc: &DocRef, f: impl FnOnce(&mut Session, &DocRef) -> Result<T> + Send + 'static) -> Result<T> {
    let doc = doc.clone();
    on_session_thread(move |g| {
        let reuse = g.as_ref().is_some_and(|s| s.dir == doc.dir && s.mtime == mtime(&doc.file));
        if !reuse {
            *g = Some(Session::open(&doc)?);
        }
        f(g.as_mut().unwrap(), &doc)
    })?
}

// ---------------------------------------------------------------------------
// Strumenti per il modello
// ---------------------------------------------------------------------------

fn function(name: &str, description: &str, parameters: Value) -> Value {
    json!({ "type": "function", "function": { "name": name, "description": description, "parameters": parameters } })
}

/// Gli strumenti per il documento `doc` (formato OpenAI).
pub fn definitions(doc: &DocRef) -> Vec<Value> {
    let mut defs = vec![function(
        "document_insert_image",
        "Insert a chart made with draw_chart (by its id) into the document open next to the chat. Word/Markdown: as a centred picture in its own paragraph after the paragraph containing `after_text` (or at the end). PDF: on page `page` (default: the page the user is looking at), centred or with its top-left corner at `at` [x, y] (points from the top-left of the page). SVG/photo: centred, or at `at`. The change shows at once and the user can undo it.",
        json!({ "type": "object", "properties": {
            "image": { "type": "string", "description": "Chart id from draw_chart, e.g. chart-20261021-1" },
            "after_text": { "type": "string", "description": "Word/Markdown: put it after the paragraph containing this text" },
            "page": { "type": "integer", "description": "PDF: page number (1 = first)" },
            "at": { "type": "array", "items": { "type": "number" }, "description": "PDF/SVG/photo: [x, y] of the top-left corner, in points" },
            "width_cm": { "type": "number", "description": "Width in cm (default: the chart's own size, at most the text width)" }
        }, "required": ["image"] }),
    )];
    match doc.kind {
        Kind::Text => {
            defs.push(function("text_read", "Read the whole text of the open document (Markdown, plain text or HTML), with line numbers.", json!({ "type": "object", "properties": {} })));
            defs.push(function(
                "text_replace",
                "Replace an exact piece of the document's text (it must appear once) with new text. Markdown: formulas in LaTeX ($...$, $$...$$).",
                json!({ "type": "object", "properties": { "old": { "type": "string" }, "new": { "type": "string" } }, "required": ["old", "new"] }),
            ));
            defs.push(function(
                "text_insert",
                "Insert text (Markdown, with LaTeX formulas) as new paragraphs after the paragraph containing `after_text`, or at the end.",
                json!({ "type": "object", "properties": { "text": { "type": "string" }, "after_text": { "type": "string" } }, "required": ["text"] }),
            ));
        }
        Kind::Word | Kind::Pdf | Kind::Svg | Kind::Photo => match with_session(doc, |s, _| Ok(s.tools.clone())) {
            Ok(tools) => {
                for t in tools {
                    let name = format!("{}{}", doc.prefix(), t["name"].as_str().unwrap_or(""));
                    let mut desc = t["description"].as_str().unwrap_or("").to_string();
                    let mut params = t["inputSchema"].clone();
                    if doc.kind == Kind::Pdf {
                        // il documento è uno solo: `doc` lo mette OpenSlate
                        if let Some(req) = params["required"].as_array_mut() {
                            req.retain(|r| r != "doc");
                        }
                        desc.push_str(" (`doc` can be omitted: it is the open document.)");
                        if t["name"] == "fill_sign_add" {
                            without_signature(&mut params);
                            desc.push_str(" In OpenSlate the user signs with the pen: signature and initials are not available here, and neither are image files (`path`).");
                        }
                    }
                    defs.push(function(&name, &desc, params));
                }
            }
            Err(e) => tracing::warn!("strumenti del documento: {e:#}"),
        },
    }
    defs
}

/// Istruzioni per il modello.
pub fn system_note(doc: &DocRef, title: &str) -> String {
    let (what, how) = match doc.kind {
        Kind::Word => ("un documento Word", "gli strumenti word__* (il server MCP di WordCraft: word__inspect_document per vedere com'è fatto, word__execute / word__batch per i comandi di WordCraft, word__list_commands per cercarli, word__render_page per guardare una pagina)"),
        Kind::Pdf => ("un PDF", "gli strumenti pdf__* (il server MCP di PdfCraft: pdf__doc_inspect, pdf__text_find, pdf__text_edit, pdf__page_render per guardare una pagina…). \
            Per compilare un modulo: pdf__form_fields per vedere i campi, pdf__form_fill per riempirli tutti in una volta, pdf__form_reset per svuotarli. \
            Se il modulo non ha campi (stampato o scansionato): pdf__form_detect_fields li trova e li crea, oppure pdf__fill_sign_add scrive testo, crocette e la data \
            dove servono (guarda prima la pagina con pdf__page_render; la data nel formato dell'utente con `format`, per esempio dd/mm/yyyy in italiano, e `language`). \
            Usa solo i dati che l'utente ti ha dato in questa conversazione o che sono nei documenti: quelli che mancano chiedili, non inventarli. \
            La firma non la metti tu: alla fine di' all'utente dove firmare con la penna"),
        Kind::Svg => ("un disegno SVG", "gli strumenti svg__* (il server MCP di VectorCraft: svg__render_preview per guardarlo, svg__draw_shape, svg__add_text, svg__command_batch…)"),
        Kind::Text => ("un testo (Markdown, .txt o .html)", "text_read, text_replace e text_insert (nel Markdown le formule in LaTeX con $…$ e $$…$$)"),
        Kind::Photo => ("una foto", "gli strumenti photo__* (il server MCP di PhotoCraft: photo__doc_inspect per vedere com'è fatta, photo__command_list con «filter» per cercare i comandi (ritaglia, ridimensiona, ruota, luminosità, contrasto, livelli, curve, saturazione, bianco e nero, nitidezza, sfocatura, testo…), photo__command_run e photo__command_batch per usarli, photo__doc_render_preview per guardare il risultato). Ritagliare, ruotare o ridimensionare cambia le misure della foto: se l'utente ci ha scritto o firmato sopra, avvisalo"),
    };
    format!(
        "\n\nAccanto alla chat è aperto «{title}», {what}: lo puoi leggere e modificare con {how}. \
         Per metterci un grafico: prima draw_chart, poi document_insert_image con l'id del grafico. \
         Le modifiche si vedono subito nel documento e l'utente le può annullare: falle solo quando l'utente lo chiede, \
         e alla fine di' in breve che cosa hai cambiato."
    )
}

/// Esegue lo strumento `name` sul documento; None se non è suo.
pub fn execute(doc: &DocRef, name: &str, args: &Value) -> Option<Outcome> {
    let mine = name == "document_insert_image"
        || (doc.kind == Kind::Text && matches!(name, "text_read" | "text_replace" | "text_insert"))
        || (!doc.prefix().is_empty() && name.starts_with(doc.prefix()));
    if !mine {
        return None;
    }
    Some(run(doc, name, args).unwrap_or_else(|e| Outcome { text: format!("ERRORE: {e:#}"), short: format!("errore: {e:#}"), ..Default::default() }))
}

fn run(doc: &DocRef, name: &str, args: &Value) -> Result<Outcome> {
    match name {
        "document_insert_image" => insert_image(doc, args),
        "text_read" => {
            let t = std::fs::read_to_string(&doc.file)?;
            let numbered: String = t.lines().enumerate().map(|(i, l)| format!("{:>4}  {l}\n", i + 1)).collect();
            Ok(Outcome { short: format!("{} righe", t.lines().count()), text: numbered, ..Default::default() })
        }
        "text_replace" | "text_insert" => {
            let t = std::fs::read_to_string(&doc.file)?;
            let new = if name == "text_replace" {
                let old = args["old"].as_str().context("manca «old»")?;
                let n = t.matches(old).count();
                if n != 1 {
                    bail!("il testo da sostituire compare {n} volte: dev'essere una sola (allunga «old»)");
                }
                t.replacen(old, args["new"].as_str().unwrap_or(""), 1)
            } else {
                insert_paragraph(&t, args["after_text"].as_str(), args["text"].as_str().context("manca «text»")?)?
            };
            let n = edit_file(doc, new.as_bytes())?;
            Ok(Outcome { text: "Fatto: il documento è aggiornato.".into(), short: "documento modificato".into(), edit: Some(n), ..Default::default() })
        }
        _ => mcp_call(doc, &name[doc.prefix().len()..], args),
    }
}

/// Testo con un nuovo paragrafo dopo quello che contiene `after` (o in fondo).
fn insert_paragraph(text: &str, after: Option<&str>, add: &str) -> Result<String> {
    let add = add.trim_matches('\n');
    let Some(after) = after.filter(|a| !a.trim().is_empty()) else {
        return Ok(format!("{}\n\n{add}\n", text.trim_end()));
    };
    let at = text.find(after).with_context(|| format!("non trovo «{after}» nel documento"))?;
    // fine del paragrafo: la prima riga vuota dopo il punto trovato
    let end = text[at..].find("\n\n").map(|i| at + i).unwrap_or(text.len());
    Ok(format!("{}\n\n{add}\n\n{}", text[..end].trim_end(), text[end..].trim_start()))
}

/// Scrive il nuovo contenuto del file (copia per «Annulla» prima) e aggiorna.
fn edit_file(doc: &DocRef, bytes: &[u8]) -> Result<u64> {
    let n = snapshot(doc)?;
    std::fs::write(&doc.file, bytes)?;
    drop_session(&doc.dir);
    if let Some(w) = changed(doc) {
        tracing::warn!("{w}");
    }
    Ok(n)
}

/// Un comando delle app Craft che esce dal documento: apre, salva o esporta file,
/// stampa, tocca le preferenze dell'app o ne cambia i preset, i plugin, gli
/// script e le macro, la stampa unione, il sito web.
/// Le app aggiungono comandi a ogni versione: si guarda ogni parola del nome
/// (anche dentro il camelCase: `layer.quickExportAsPng`, `tools.recordMacro`),
/// non un elenco di comandi. (Copia e incolla restano: dentro OpenSlate gli
/// appunti delle app Craft sono loro, non quelli del sistema.)
fn command_outside_document(id: &str) -> bool {
    const PREFIXES: [&str; 4] = ["file.", "app.", "help.", "document.open"];
    const WORDS: [&str; 23] = [
        "export", "import", "print", "plugin", "plugins", "script", "scripts", "macro", "macros", "preferences", "settings", "recent",
        "clipboard", "template", "templates", "mailings", "install", "website", "save", "browser", "url", "email", "share",
    ];
    // un preset si può usare, non creare, cambiare o cancellare (sono dell'app)
    const PRESET_CHANGES: [&str; 10] = ["new", "delete", "rename", "edit", "move", "reset", "update", "migrate", "manager", "define"];
    // una selezione salvata resta nel documento
    const INSIDE: [&str; 2] = ["select.save", "select.saveselection"];
    let lower = id.trim().to_ascii_lowercase();
    if INSIDE.contains(&lower.as_str()) {
        return false;
    }
    if PREFIXES.iter().any(|p| lower.starts_with(p)) {
        return true;
    }
    let mut words: Vec<String> = Vec::new();
    let mut cur = String::new();
    for ch in id.trim().chars() {
        if (!ch.is_ascii_alphanumeric() || (ch.is_ascii_uppercase() && !cur.is_empty())) && !cur.is_empty() {
            words.push(std::mem::take(&mut cur));
        }
        if ch.is_ascii_alphanumeric() {
            cur.push(ch.to_ascii_lowercase());
        }
    }
    if !cur.is_empty() {
        words.push(cur);
    }
    let has = |list: &[&str]| words.iter().any(|w| list.contains(&w.as_str()));
    has(&WORDS) || (has(&["preset", "presets"]) && has(&PRESET_CHANGES))
}

/// I comandi «generici» (execute, batch, command_run…) non possono aprire,
/// salvare o esportare file né uscire dal documento: aprire, salvare ed
/// esportare lo fa l'app (il file giusto, nel posto giusto, con la copia per
/// «Annulla»); il resto non serve alla chat.
fn forbidden_command(args: &Value) -> Option<String> {
    fn walk(v: &Value, out: &mut Option<String>) {
        match v {
            Value::Object(o) => {
                for (k, x) in o {
                    if matches!(k.as_str(), "command" | "cmd" | "id") {
                        if let Some(c) = x.as_str().filter(|c| command_outside_document(c)) {
                            *out = Some(c.trim().to_string());
                        }
                    }
                    walk(x, out);
                }
            }
            Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let mut out = None;
    walk(args, &mut out);
    out
}

/// Fill & Sign di PdfCraft sa anche firmare (una firma «scritta» in corsivo o
/// un'immagine): dalla chat no, la firma la mette l'utente con la penna.
fn signature_refused(args: &Value) -> Option<&'static str> {
    if matches!(args["type"].as_str(), Some("signature" | "initials")) {
        return Some("la firma e le iniziali le mette l'utente con la penna sullo Slate (o col mouse), non la chat: compila il resto e lascia a lui la firma");
    }
    if args.get("path").is_some() || args.get("scan").is_some() {
        return Some("dalla chat Fill & Sign non usa file di immagini: solo testo, crocette, segni e data");
    }
    None
}

/// Toglie dallo schema di fill_sign_add la firma, le iniziali e le immagini.
fn without_signature(params: &mut Value) {
    if let Some(e) = params["properties"]["type"]["enum"].as_array_mut() {
        e.retain(|v| !matches!(v.as_str(), Some("signature" | "initials")));
    }
    if let Some(p) = params["properties"].as_object_mut() {
        for k in ["path", "scan", "sensitivity"] {
            p.remove(k);
        }
    }
}

fn mcp_call(doc: &DocRef, tool: &str, args: &Value) -> Result<Outcome> {
    if !allowed(doc.kind, tool) {
        bail!("strumento non disponibile: {tool}");
    }
    if doc.kind == Kind::Pdf && tool == "fill_sign_add" {
        if let Some(why) = signature_refused(args) {
            bail!("{why}");
        }
    }
    if let Some(c) = forbidden_command(args) {
        bail!("il comando «{c}» non si usa dalla chat: la chat lavora solo dentro il documento (aprire, salvare ed esportare li fa l'app da sola; niente stampa, plugin, script, macro, preferenze o preset dell'app)");
    }
    let before = std::fs::read(&doc.file)?;
    let size_before = if doc.kind == Kind::Photo { crate::rasign::dimensions(&doc.file) } else { None };
    let n = snapshot(doc)?;
    let mut args = args.clone();
    if doc.kind == Kind::Photo {
        // i comandi lunghi (filtri) finiscono prima di salvare: niente lavori in sottofondo
        if tool == "command_run" {
            args["wait"] = json!(true);
        }
        if let Some(steps) = args.get_mut("steps").and_then(|v| v.as_array_mut()) {
            for st in steps.iter_mut().filter(|s| s.is_object()) {
                st["wait"] = json!(true);
            }
        }
    }
    let (tool_s, args_s) = (tool.to_string(), args);
    let res = with_session(doc, move |s, doc| {
        let tool = tool_s.as_str();
        let mut args = args_s;
        if let (Server::Pdf(_, id), Some(o)) = (&s.server, args.as_object_mut()) {
            o.entry("doc").or_insert(json!(*id));
        }
        let ro = match &s.server {
            // PhotoCraft: cambiano la foto solo i comandi
            Server::Photo(_) => !matches!(tool, "command_run" | "command_batch"),
            _ => s.tools.iter().find(|t| t["name"] == tool).and_then(|t| t["annotations"]["readOnlyHint"].as_bool()).unwrap_or(false),
        };
        let r = s.rpc("tools/call", json!({ "name": tool, "arguments": args }))?;
        if !ro && r["isError"].as_bool() != Some(true) {
            s.save(doc)?;
        }
        Ok(r)
    });
    let r = match res {
        Ok(r) => r,
        Err(e) => {
            let _ = std::fs::remove_dir_all(undo_dir(&doc.dir, n));
            drop_session(&doc.dir);
            return Err(e);
        }
    };
    let mut text = result_text(&r);
    let images = result_images(&r);
    let error = r["isError"].as_bool() == Some(true);
    let after = std::fs::read(&doc.file)?;
    let edit = if after != before {
        if let Some(w) = changed(doc) {
            text = format!("{text}\n\n{w}");
        }
        // la foto ha cambiato misure e c'era inchiostro sopra
        if let (Some(a), Some(b)) = (size_before, crate::rasign::dimensions(&doc.file)) {
            if a != b && ink_pages(&doc.dir).is_some_and(|(_, ink)| ink) {
                text = format!(
                    "{text}\n\nATTENZIONE: la foto è passata da {}×{} a {}×{} pixel e l'utente ci aveva scritto o firmato sopra: \
                     dillo all'utente, che controlli dove è finito l'inchiostro; con «Annulla le modifiche» torna tutto com'era.",
                    a.0, a.1, b.0, b.1
                );
            }
        }
        Some(n)
    } else {
        let _ = std::fs::remove_dir_all(undo_dir(&doc.dir, n));
        None
    };
    let short = if error {
        format!("errore: {}", text.lines().next().unwrap_or(""))
    } else if edit.is_some() {
        "documento modificato".into()
    } else if !images.is_empty() {
        "immagine della pagina".into()
    } else {
        "fatto".into()
    };
    let mut text = if text.len() > 30_000 { format!("{}…", &text[..text.floor_char_boundary(30_000)]) } else { text };
    if !images.is_empty() {
        text.push_str("\n(L'immagine è mostrata all'utente nella chat.)");
    }
    Ok(Outcome { text: if error { format!("ERRORE: {text}") } else { text }, short, images, edit })
}

// ---------------------------------------------------------------------------
// Appunti (le trascrizioni in exports/): grafico e «Annulla»
// ---------------------------------------------------------------------------

fn note_undo_dir(note: &Path) -> PathBuf {
    let name = note.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    note.parent().unwrap_or(Path::new(".")).join(".ai-undo").join(name)
}

/// Copia dell'appunto prima che la chat lo modifichi; ritorna il suo numero.
pub fn note_snapshot(note: &Path) -> Result<u64> {
    let d = note_undo_dir(note);
    std::fs::create_dir_all(&d)?;
    let n = std::fs::read_dir(&d)?
        .filter_map(|e| e.ok()?.path().file_stem()?.to_str()?.parse::<u64>().ok())
        .max()
        .unwrap_or(0)
        + 1;
    std::fs::copy(note, d.join(format!("{n}.md")))?;
    Ok(n)
}

/// Toglie una copia non servita (la modifica non c'è stata).
pub fn note_forget(note: &Path, n: u64) {
    let _ = std::fs::remove_file(note_undo_dir(note).join(format!("{n}.md")));
}

/// «Annulla» della chat su un appunto: torna com'era prima della modifica `n`.
pub fn note_undo(note: &Path, n: u64) -> Result<()> {
    let d = note_undo_dir(note);
    let f = d.join(format!("{n}.md"));
    if !f.is_file() {
        bail!("questa modifica non si può più annullare");
    }
    // quello che c'è adesso (anche ciò che hai scritto dopo la modifica
    // dell'AI) resta da parte: «annullato-<ora>.md» accanto alle copie
    let _ = std::fs::copy(note, d.join(format!("annullato-{}.md", chrono::Local::now().format("%Y%m%d-%H%M%S"))));
    std::fs::copy(&f, note)?;
    for e in std::fs::read_dir(&d)?.flatten() {
        if e.path().file_stem().and_then(|s| s.to_str()).and_then(|s| s.parse::<u64>().ok()).is_some_and(|k| k >= n) {
            let _ = std::fs::remove_file(e.path());
        }
    }
    Ok(())
}

/// Il grafico `grafico` nell'appunto: al posto di un testo, dopo un paragrafo o in fondo.
pub fn note_insert_chart(note: &Path, args: &Value) -> Result<(String, String)> {
    let id = args["grafico"].as_str().or(args["image"].as_str()).context("manca «grafico» (l'id di draw_chart)")?;
    let chart = crate::charts::find(id).with_context(|| format!("non trovo il grafico «{id}»: fallo prima con draw_chart"))?;
    let stem = note.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let name = format!("{stem}-{}.svg", chart.id);
    std::fs::copy(&chart.svg, note.parent().unwrap_or(Path::new(".")).join(&name))?;
    let pct = args["larghezza_percento"].as_f64().unwrap_or(60.0).clamp(10.0, 100.0).round();
    let tag = format!("<img src=\"{name}\" alt=\"grafico\" width=\"{pct}%\">");
    let text = std::fs::read_to_string(note).unwrap_or_default();
    let new = match args["al_posto_di"].as_str().filter(|t| !t.trim().is_empty()) {
        Some(old) => match text.matches(old).count() {
            1 => text.replacen(old, &tag, 1),
            0 => bail!("il testo da sostituire non c'è nell'appunto: rileggilo con appunto_leggi"),
            n => bail!("il testo da sostituire compare {n} volte: allunga «al_posto_di»"),
        },
        None => insert_paragraph(&text, args["dopo"].as_str(), &tag)?,
    };
    std::fs::write(note, new)?;
    Ok((format!("Fatto: il grafico {} è nell'appunto (file {name}).", chart.id), "grafico nell'appunto".into()))
}

// ---------------------------------------------------------------------------
// Un grafico nel documento
// ---------------------------------------------------------------------------

const CM: f32 = 72.0 / 2.54;

fn insert_image(doc: &DocRef, args: &Value) -> Result<Outcome> {
    let id = args["image"].as_str().context("manca «image» (l'id del grafico)")?;
    let chart = crate::charts::find(id).with_context(|| format!("non trovo il grafico «{id}»: fallo prima con draw_chart"))?;
    let width = args["width_cm"].as_f64().map(|w| w as f32 * CM).unwrap_or(chart.width).clamp(2.0 * CM, 17.0 * CM);
    let height = chart.height * width / chart.width;
    let at = args["at"].as_array().and_then(|a| Some([a.first()?.as_f64()? as f32, a.get(1)?.as_f64()? as f32]));
    let n = snapshot(doc)?;
    let res = (|| -> Result<()> {
        match doc.kind {
            Kind::Word => {
                use wordcraft_engine::cmd::insert::base64_encode;
                let mut s = crate::wordsign::open(&doc.file)?;
                let run = |s: &mut wordcraft_engine::Session, c: &str, p: Value| s.run(c, &p).map_err(|e| anyhow!("WordCraft, {c}: {e}"));
                match args["after_text"].as_str().filter(|t| !t.trim().is_empty()) {
                    Some(t) => {
                        run(&mut s, "select.text", json!({ "text": t })).with_context(|| format!("non trovo «{t}» nel documento"))?;
                        run(&mut s, "select.collapse", json!({}))?;
                        // in fondo al paragrafo: inizio del successivo e un passo indietro
                        if run(&mut s, "caret.paraDown", json!({})).is_ok() {
                            let _ = run(&mut s, "caret.left", json!({}));
                        }
                    }
                    None => {
                        run(&mut s, "caret.docEnd", json!({}))?;
                    }
                }
                run(&mut s, "text.newParagraph", json!({}))?;
                let png = std::fs::read(&chart.png)?;
                run(&mut s, "insert.picture", json!({ "data": base64_encode(&png), "width": width, "alt": chart.id }))?;
                let _ = run(&mut s, "para.alignCenter", json!({}));
                run(&mut s, "file.save", json!({ "path": doc.file.canonicalize()?.to_string_lossy() }))?;
            }
            Kind::Text => {
                // il Markdown cita il grafico, che va accanto (lo impagina mdcraft)
                let name = format!("{}.svg", chart.id);
                std::fs::copy(&chart.svg, doc.dir.join(&name))?;
                let pct = (width / (595.3 - 132.0) * 100.0).clamp(10.0, 100.0).round();
                let line = format!("<img src=\"{name}\" alt=\"{}\" width=\"{pct}%\">", chart.id);
                let t = std::fs::read_to_string(&doc.file)?;
                std::fs::write(&doc.file, insert_paragraph(&t, args["after_text"].as_str(), &line)?)?;
            }
            Kind::Pdf => {
                let bytes = std::sync::Arc::new(std::fs::read(&doc.file)?);
                let mut s = pdfcraft_engine::Session::new();
                let id = s.open("source.pdf", None, bytes, None).map_err(|e| anyhow!("PdfCraft: {e:?}"))?;
                let pages = crate::pdfdoc::pages(&doc.file)?;
                let page = args["page"].as_u64().map(|p| (p as usize).saturating_sub(1)).unwrap_or(doc.page).min(pages.len().saturating_sub(1));
                let (pw, ph) = pages.get(page).map(|p| p.display_size()).unwrap_or((595.0, 842.0));
                let (w, h) = (width.min(pw - 36.0), height * width.min(pw - 36.0) / width);
                let [x, y] = at.unwrap_or([(pw - w) / 2.0, (ph - h) / 2.0]);
                let rect = [x as f64, y as f64, (x + w) as f64, (y + h) as f64];
                let png = std::sync::Arc::new(std::fs::read(&chart.png)?);
                s.apply(id, pdfcraft_engine::Edit::AddImage { page, rect: Some(rect), name: format!("{}.png", chart.id), bytes: png })
                    .map_err(|e| anyhow!("PdfCraft: {e:?}"))?;
                let out = s.save_bytes(id).map_err(|e| anyhow!("PdfCraft: {e:?}"))?;
                std::fs::write(&doc.file, out.as_slice())?;
            }
            Kind::Svg | Kind::Photo => {
                crate::vecsign::place_svg(&doc.file, &chart.svg, at, width, height)?;
            }
        }
        Ok(())
    })();
    if let Err(e) = res {
        // il documento torna com'era
        let _ = undo(&doc.dir, n);
        return Err(e);
    }
    drop_session(&doc.dir);
    let warn = changed(doc).map(|w| format!("\n\n{w}")).unwrap_or_default();
    Ok(Outcome {
        text: format!("Fatto: il grafico {} è nel documento ({:.1} × {:.1} cm).{warn}", chart.id, width / CM, height / CM),
        short: "grafico inserito nel documento".into(),
        edit: Some(n),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comandi_di_file_vietati() {
        assert!(forbidden_command(&json!({ "command": "file.save", "params": { "path": "/tmp/x.docx" } })).is_some());
        assert!(forbidden_command(&json!({ "commands": [{ "command": "format.bold" }, { "command": "File.Open" }] })).is_some());
        assert!(forbidden_command(&json!({ "command": "insert.text", "params": { "text": "salva il file.docx" } })).is_none());
        // dentro un lotto annidato (VectorCraft: action.batch)
        assert!(forbidden_command(&json!({ "steps": [{ "id": "action.batch", "params": { "steps": [{ "id": "plugin.run" }] } }] })).is_some());
    }

    /// I comandi che escono dal documento, presi dai cataloghi di WordCraft 0.10,
    /// VectorCraft 0.15, PdfCraft 0.12 e PhotoCraft 0.13, e quelli che servono alla chat.
    #[test]
    fn comandi_fuori_dal_documento() {
        for c in [
            "file.save", "File.Open", "app.quit", "document.open", "document.save", "document.exportPdf", "help.website", "help.check_updates",
            "export.docx", "export.pptx", "print.dialog", "form.export_data", "comment.import", "plugin.install", "plugin.run", "script.check",
            "tools.macros", "tools.recordMacro", "tools.attachTemplate", "mailings.start", "edit.preferences.general", "edit.colorSettings",
            "layer.quickExportAsPng", "layer.exportAs", "layer.smartObjects.saveContents", "pdf.preset.save", "brush.presets.importAbr",
            "edit.defineBrushPreset", "edit.presets.presetManager", "gradient.presets.new", "document.recent.add", "type.saveDefaultTypeStyles",
            "clipboard.exportPng", "webExport.settings", "library.export", "css.export", "swatches.export", "print.presets.save",
        ] {
            assert!(command_outside_document(c), "{c}");
        }
        for c in [
            "text.insert", "text.newParagraph", "format.bold", "para.alignCenter", "table.borders", "insert.equation", "insert.picture",
            "references.updateToc", "edit.findReplace", "edit.undo", "edit.copy", "edit.paste", "edit.pasteFormat", "list.multilevelPreset",
            "select.none", "select.save", "select.saveSelection", "path.create", "object.setBounds", "effect.lightingPreset",
            "gradient.presets.apply", "style.presets.apply", "image.imageSize", "image.adjustments.brightnessContrast", "text.copyright",
            "page.rotate", "edit.watermark.update", "layer.layerStyle.pasteLayerStyle",
        ] {
            assert!(!command_outside_document(c), "{c}");
        }
    }

    #[test]
    fn paragrafo_dopo_un_testo() {
        let t = "# Titolo\n\nPrimo paragrafo\nsu due righe.\n\nSecondo.\n";
        let out = insert_paragraph(t, Some("Primo"), "NUOVO").unwrap();
        assert_eq!(out, "# Titolo\n\nPrimo paragrafo\nsu due righe.\n\nNUOVO\n\nSecondo.\n");
        assert!(insert_paragraph(t, None, "FINE").unwrap().ends_with("Secondo.\n\nFINE\n"));
        assert!(insert_paragraph(t, Some("assente"), "x").is_err());
    }
}

#[cfg(test)]
mod e2e {
    use super::*;

    /// La foto col server MCP di PhotoCraft: comandi, ridimensiona, schiarisci,
    /// salva nello stesso formato, Annulla. Cambia la cartella corrente:
    /// cargo test -- --ignored --test-threads=1 foto_con_photocraft --nocapture
    #[test]
    #[ignore]
    fn foto_con_photocraft() {
        use photocraft_codecs::{ChannelLayout, EncodeOptions, Format, Image};
        let work = std::env::temp_dir().join(format!("openslate-photo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&work);
        std::fs::create_dir_all(&work).unwrap();
        std::env::set_current_dir(&work).unwrap();
        // una foto grigia 400×300 con un quadrato scuro
        let (w, h) = (400u32, 300u32);
        let mut px = vec![128u8; (w * h * 3) as usize];
        for y in 100..200 {
            for x in 150..250 {
                let i = ((y * w + x) * 3) as usize;
                px[i..i + 3].copy_from_slice(&[30, 30, 30]);
            }
        }
        let img = Image::from_u8(w, h, ChannelLayout::Rgb, px).unwrap();
        let src = work.join("foto.jpg");
        std::fs::write(&src, photocraft_codecs::encode(&img, Format::Jpeg, &EncodeOptions::default()).unwrap()).unwrap();
        let doc = {
            let d = crate::document::Document::import(&src).unwrap();
            DocRef::of(&d.dir, 0).unwrap()
        };
        assert_eq!(doc.kind, Kind::Photo);
        let names: Vec<String> = definitions(&doc).iter().filter_map(|d| d["function"]["name"].as_str().map(String::from)).collect();
        println!("strumenti: {names:?}");
        assert!(names.iter().any(|n| n == "photo__command_run") && names.iter().any(|n| n == "photo__doc_render_preview"));
        assert!(!names.iter().any(|n| n == "photo__doc_save" || n == "photo__doc_open"), "{names:?}");
        let mut found = Vec::new();
        for f in ["image.", "adjust", "crop", "rotate", "text"] {
            let list = execute(&doc, "photo__command_list", &json!({ "filter": f })).unwrap();
            let v: Value = serde_json::from_str(&list.text).unwrap_or_else(|_| panic!("{}", &list.text[..list.text.len().min(300)]));
            for c in v.as_array().unwrap() {
                found.push(format!("{}  {}  {}", c["id"].as_str().unwrap_or(""), c["label"].as_str().unwrap_or(""), c["params"].as_str().unwrap_or("")));
            }
        }
        if let Ok(o) = std::env::var("PHOTO_IDS") {
            std::fs::write(o, found.join("\n")).unwrap();
        }
        assert!(found.len() > 10);
        // più chiara e grande la metà, in un colpo
        let o = execute(
            &doc,
            "photo__command_batch",
            &json!({ "steps": [
                { "id": "image.adjustments.brightnessContrast", "params": { "brightness": 60 } },
                { "id": "image.imageSize", "params": { "width": 200 } }
            ] }),
        )
        .unwrap();
        assert!(!o.text.starts_with("ERRORE"), "{}", o.text);
        let n = o.edit.expect("la foto è cambiata");
        assert_eq!(crate::rasign::dimensions(&doc.file), Some((200, 150)));
        let back = photocraft_codecs::decode(&std::fs::read(&doc.file).unwrap()).unwrap();
        assert!(std::fs::read(&doc.file).unwrap().starts_with(&[0xFF, 0xD8]), "resta un JPEG");
        assert!(!doc.dir.join("salvataggio-ai.jpg").exists());
        let corner = back.to_rgba8()[0];
        assert!(corner > 150, "più chiara: {corner}");
        // guardare non cambia niente
        let look = execute(&doc, "photo__doc_render_preview", &json!({ "max_side": 256 })).unwrap();
        assert_eq!(look.images.len(), 1);
        assert!(look.edit.is_none());
        // un comando di file non passa
        let bad = execute(&doc, "photo__command_run", &json!({ "id": "file.saveAs", "params": { "path": "/tmp/x.png" } })).unwrap();
        assert!(bad.text.starts_with("ERRORE"), "{}", bad.text);
        // Annulla: com'era
        undo(&doc.dir, n).unwrap();
        assert_eq!(crate::rasign::dimensions(&doc.file), Some((400, 300)));
    }

    fn chart() -> String {
        let spec = json!({
            "x": { "min": -4, "max": 4, "label": "$x$" }, "y": { "min": -3, "max": 4, "label": "$y$" },
            "series": [
                { "type": "function", "expr": "1 - x", "label": "$x + y = 1$", "color": "blue" },
                { "type": "vline", "value": -1, "label": "$x = -1$", "color": "green", "dashed": true },
                { "type": "points", "points": [[-1, 2]], "names": ["$P(-1, 2)$"], "color": "red" }
            ]
        });
        crate::charts::draw(&spec).unwrap().id
    }

    /// Un modulo stampato (senza campi): la chat trova i campi, li compila,
    /// scrive data e crocetta con Fill & Sign; la firma no.
    /// cargo test -- --ignored --test-threads=1 modulo_pdf (immagine in $E2E_OUT)
    #[test]
    #[ignore]
    fn modulo_pdf() {
        let work = std::env::temp_dir().join(format!("openslate-modulo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&work);
        std::fs::create_dir_all(&work).unwrap();
        std::env::set_current_dir(&work).unwrap();
        let md = work.join("modulo.md");
        std::fs::write(&md, "# Modulo di iscrizione\n\nNome: ______________________________\n\nCognome: ______________________________\n\nCittà: ______________________________\n\nData: ______________\n").unwrap();
        let pdf = work.join("modulo.pdf");
        crate::wordsign::to_pdf(&md, &pdf).unwrap();
        let p = import(&pdf);
        assert_eq!(p.kind, Kind::Pdf);

        // gli strumenti ci sono, Fill & Sign senza firma
        let defs = definitions(&p);
        let names: Vec<&str> = defs.iter().filter_map(|d| d["function"]["name"].as_str()).collect();
        for n in ["pdf__form_fields", "pdf__form_fill", "pdf__form_reset", "pdf__form_detect_fields", "pdf__fill_sign_add"] {
            assert!(names.contains(&n), "{n}: {names:?}");
        }
        let fs = defs.iter().find(|d| d["function"]["name"] == "pdf__fill_sign_add").unwrap();
        let types = fs["function"]["parameters"]["properties"]["type"]["enum"].to_string();
        assert!(!types.contains("signature") && !types.contains("initials") && types.contains("date"), "{types}");
        assert!(fs["function"]["parameters"]["properties"].get("path").is_none());
        assert!(system_note(&p, "modulo").contains("pdf__form_fill"));

        // i campi del modulo stampato
        let f = execute(&p, "pdf__form_fields", &json!({})).unwrap();
        assert!(!f.text.starts_with("ERRORE") && f.edit.is_none(), "{}", f.text);
        let d = execute(&p, "pdf__form_detect_fields", &json!({})).unwrap();
        assert!(!d.text.starts_with("ERRORE") && d.edit.is_some(), "{}", d.text);
        let f = execute(&p, "pdf__form_fields", &json!({})).unwrap();
        let v: Value = serde_json::from_str(&f.text).unwrap_or(json!({}));
        let fields: Vec<Value> = v.as_array().cloned().or_else(|| v["fields"].as_array().cloned()).unwrap_or_default();
        let field_names: Vec<String> = fields.iter().filter_map(|x| x["name"].as_str().map(String::from)).collect();
        eprintln!("campi trovati: {field_names:?}");
        assert!(field_names.len() >= 3, "{}", f.text);
        // i campi prendono il nome dall'etichetta stampata accanto
        let pick = |w: &str| field_names.iter().find(|n| n.trim().eq_ignore_ascii_case(w)).cloned().unwrap_or_else(|| panic!("{w}: {field_names:?}"));
        let o = execute(&p, "pdf__form_fill", &json!({ "values": { pick("Nome"): "Michele", pick("Cognome"): "Paolillo", pick("Città"): "Barletta" } })).unwrap();
        assert!(!o.text.starts_with("ERRORE") && o.edit.is_some(), "{}", o.text);
        let f = execute(&p, "pdf__form_fields", &json!({})).unwrap();
        assert!(f.text.contains("Michele") && f.text.contains("Paolillo") && f.text.contains("Barletta"), "{}", f.text);

        // Fill & Sign: testo, data e crocetta sì
        let o = execute(&p, "pdf__fill_sign_add", &json!({ "page": 1, "type": "date", "at": [400, 90], "format": "dd/mm/yyyy" })).unwrap();
        assert!(!o.text.starts_with("ERRORE") && o.edit.is_some(), "{}", o.text);
        let o = execute(&p, "pdf__fill_sign_add", &json!({ "page": 1, "type": "check", "at": [60, 90] })).unwrap();
        assert!(!o.text.starts_with("ERRORE"), "{}", o.text);
        // la firma no, nemmeno passando dalle immagini
        let before = std::fs::read(&p.file).unwrap();
        for a in [
            json!({ "page": 1, "type": "signature", "text": "Michele Paolillo", "at": [300, 600] }),
            json!({ "page": 1, "type": "initials", "text": "MP", "at": [300, 600] }),
            json!({ "page": 1, "type": "text", "path": "firma.png", "at": [300, 600] }),
        ] {
            let o = execute(&p, "pdf__fill_sign_add", &a).unwrap();
            assert!(o.text.starts_with("ERRORE") && o.edit.is_none(), "{}", o.text);
        }
        assert_eq!(before, std::fs::read(&p.file).unwrap());

        // poi l'utente firma con la penna: i valori, la data e la crocetta restano
        drop_session(&p.dir);
        let firmato = work.join("modulo-firmato.pdf");
        let firma = crate::pdfdoc::OverlayStroke { color: [20, 40, 160], width_frac: 0.003, pts: vec![(0.55, 0.5), (0.65, 0.45), (0.75, 0.52)], highlight: false };
        crate::pdfdoc::write_overlay(&p.file, &firmato, &[vec![firma]]).unwrap();
        let s = import(&firmato);
        let f = execute(&s, "pdf__form_fields", &json!({})).unwrap();
        assert!(f.text.contains("Michele") && f.text.contains("Barletta"), "{}", f.text);
        let c = execute(&s, "pdf__comment_list", &json!({})).unwrap();
        assert!(!c.text.starts_with("ERRORE") && c.text.contains("2026"), "{}", c.text);

        if let Ok(o) = std::env::var("E2E_OUT") {
            let pngs = crate::docimport::render_pages(&firmato, &Path::new(&o).join("modulo"), 1200).unwrap();
            std::fs::copy(&pngs[0], format!("{o}/modulo.png")).unwrap();
        }
    }

    fn import(src: &Path) -> DocRef {
        let d = crate::document::Document::import(src).unwrap();
        DocRef::of(&d.dir, 0).unwrap()
    }

    /// cargo test -- --ignored --test-threads=1 grafico_nei_documenti --nocapture
    /// (lavora in una cartella temporanea: cambia la cartella corrente)
    #[test]
    #[ignore]
    fn grafico_nei_documenti() {
        let work = std::env::temp_dir().join(format!("openslate-docagent-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&work);
        std::fs::create_dir_all(&work).unwrap();
        std::env::set_current_dir(&work).unwrap();
        let out = std::env::var("E2E_OUT").ok();

        // --- Word, fatto con WordCraft
        let docx = work.join("sistema.docx");
        let mut s = wordcraft_engine::Session::new(wordcraft_engine::doc::Document::new());
        for (c, p) in [
            ("text.insert", json!({ "text": "Risolvi questo sistema di equazioni lineari." })),
            ("text.newParagraph", json!({})),
            ("text.insert", json!({ "text": "Giusto?" })),
            ("file.save", json!({ "path": docx.to_string_lossy() })),
        ] {
            s.run(c, &p).unwrap();
        }
        let word = import(&docx);
        assert_eq!(word.kind, Kind::Word);
        let defs = definitions(&word);
        let names: Vec<&str> = defs.iter().filter_map(|d| d["function"]["name"].as_str()).collect();
        assert!(names.contains(&"word__inspect_document") && names.contains(&"word__execute") && names.contains(&"document_insert_image"), "{names:?}");
        let id = chart();
        let o = execute(&word, "document_insert_image", &json!({ "image": id, "after_text": "Risolvi", "width_cm": 9 })).unwrap();
        assert!(o.edit.is_some(), "{}", o.text);
        // il Word ha un'immagine e il testo è rimasto
        let ins = execute(&word, "word__inspect_document", &json!({})).unwrap();
        assert!(ins.text.contains("Giusto"), "{}", ins.text);
        let saved = std::fs::read(&word.file).unwrap();
        let z = zip_names(&saved);
        assert!(z.iter().any(|n| n.starts_with("word/media/")), "{z:?}");
        // un comando di WordCraft via MCP: testo in fondo
        let o2 = execute(&word, "word__batch", &json!({ "commands": [ { "command": "caret.docEnd" }, { "command": "text.newParagraph" }, { "command": "text.insert", "params": { "text": "Soluzione: P(-1, 2)." } } ] })).unwrap();
        assert!(o2.edit.is_some(), "{}", o2.text);
        let pdf_text = crate::docimport::pdf_text(&word.dir.join("source.pdf")).unwrap().join("\n");
        assert!(pdf_text.contains("Soluzione"), "{pdf_text}");
        if let Some(o) = &out {
            let pngs = crate::docimport::render_pages(&word.dir.join("source.pdf"), Path::new(o), 1400).unwrap();
            std::fs::copy(&pngs[0], format!("{o}/word.png")).unwrap();
        }
        // una pagina disegnata dal server MCP torna come immagine
        let r = execute(&word, "word__render_page", &json!({ "page": 1 })).unwrap();
        assert!(!r.images.is_empty(), "{}", r.text);
        // Annulla: indietro alla prima modifica
        undo(&word.dir, o.edit.unwrap()).unwrap();
        let z = zip_names(&std::fs::read(&word.file).unwrap());
        assert!(!z.iter().any(|n| n.starts_with("word/media/")), "{z:?}");

        // --- PDF (lo fa WordCraft dal Markdown)
        let md = work.join("nota.md");
        std::fs::write(&md, "# Esercizio\n\nRisolvi il sistema.\n\nGiusto?\n").unwrap();
        let pdf = work.join("nota.pdf");
        crate::wordsign::to_pdf(&md, &pdf).unwrap();
        let p = import(&pdf);
        assert_eq!(p.kind, Kind::Pdf);
        let names: Vec<String> = definitions(&p).iter().filter_map(|d| d["function"]["name"].as_str().map(String::from)).collect();
        assert!(names.iter().any(|n| n == "pdf__text_find"), "{names:?}");
        let f = execute(&p, "pdf__text_find", &json!({ "query": "Giusto" })).unwrap();
        assert!(!f.text.starts_with("ERRORE"), "{}", f.text);
        let before = std::fs::read(&p.file).unwrap().len();
        let o = execute(&p, "document_insert_image", &json!({ "image": id, "page": 1, "at": [100, 300], "width_cm": 8 })).unwrap();
        assert!(o.edit.is_some() && std::fs::read(&p.file).unwrap().len() > before, "{}", o.text);
        if let Some(o) = &out {
            let pngs = crate::docimport::render_pages(&p.file, &Path::new(o).join("pdf"), 1400).unwrap();
            std::fs::copy(&pngs[0], format!("{o}/pdf.png")).unwrap();
        }

        // --- SVG
        let svg = work.join("disegno.svg");
        std::fs::write(&svg, r##"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="300" viewBox="0 0 400 300"><rect x="10" y="10" width="80" height="50" fill="#fc0"/></svg>"##).unwrap();
        let v = import(&svg);
        assert_eq!(v.kind, Kind::Svg);
        let o = execute(&v, "document_insert_image", &json!({ "image": id, "width_cm": 6 })).unwrap();
        assert!(o.edit.is_some(), "{}", o.text);
        let t = std::fs::read_to_string(&v.file).unwrap();
        assert!(t.matches("<path").count() > 5, "{}", &t[..t.len().min(500)]);
        let names: Vec<String> = definitions(&v).iter().filter_map(|d| d["function"]["name"].as_str().map(String::from)).collect();
        assert!(names.iter().any(|n| n == "svg__render_preview"), "{names:?}");

        // --- Markdown: il grafico accanto, citato nel testo
        let m = import(&md);
        assert_eq!(m.kind, Kind::Text);
        let o = execute(&m, "document_insert_image", &json!({ "image": id, "after_text": "Risolvi" })).unwrap();
        assert!(o.edit.is_some(), "{}", o.text);
        let t = std::fs::read_to_string(&m.file).unwrap();
        assert!(t.contains(&format!("<img src=\"{id}.svg\"")) && m.dir.join(format!("{id}.svg")).exists(), "{t}");
        let o = execute(&m, "text_replace", &json!({ "old": "Giusto?", "new": "La soluzione è $P(-1, 2)$." })).unwrap();
        assert!(o.edit.is_some(), "{}", o.text);
        if let Some(o) = &out {
            let pngs = crate::docimport::render_pages(&m.dir.join("source.pdf"), &Path::new(o).join("md"), 1400).unwrap();
            std::fs::copy(&pngs[0], format!("{o}/md.png")).unwrap();
        }

        // --- Appunto di una trascrizione: il grafico al posto della figura a mano, poi Annulla
        std::fs::create_dir_all("exports").unwrap();
        let note = work.join("exports/appunti-x.md");
        let fig = "<img src=\"appunti-x-p1-fig1.svg\" alt=\"assi\" width=\"50%\">";
        let before = format!("Risolvi il sistema.\n\n{fig}\n\n*Figura: piano cartesiano*\n\nGiusto?\n");
        std::fs::write(&note, &before).unwrap();
        let n = note_snapshot(&note).unwrap();
        let (text, _) = note_insert_chart(&note, &json!({ "grafico": id, "al_posto_di": fig })).unwrap();
        assert!(text.starts_with("Fatto"), "{text}");
        let after = std::fs::read_to_string(&note).unwrap();
        assert!(!after.contains("fig1.svg") && after.contains(&format!("appunti-x-{id}.svg")) && after.contains("*Figura: piano cartesiano*"), "{after}");
        assert!(work.join(format!("exports/appunti-x-{id}.svg")).exists());
        assert!(note_insert_chart(&note, &json!({ "grafico": id, "al_posto_di": "assente" })).is_err());
        note_undo(&note, n).unwrap();
        assert_eq!(std::fs::read_to_string(&note).unwrap(), before);
    }

    fn zip_names(b: &[u8]) -> Vec<String> {
        // nomi dei file nel .docx (directory centrale dello zip)
        let mut out = Vec::new();
        let mut i = 0;
        while i + 46 < b.len() {
            if &b[i..i + 4] == b"PK\x01\x02" {
                let n = u16::from_le_bytes([b[i + 28], b[i + 29]]) as usize;
                let (m, k) = (u16::from_le_bytes([b[i + 30], b[i + 31]]) as usize, u16::from_le_bytes([b[i + 32], b[i + 33]]) as usize);
                out.push(String::from_utf8_lossy(&b[i + 46..(i + 46 + n).min(b.len())]).to_string());
                i += 46 + n + m + k;
            } else {
                i += 1;
            }
        }
        out
    }
}
