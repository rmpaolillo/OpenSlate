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
            Kind::Text | Kind::Photo => "",
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

fn changed(doc: &DocRef) {
    if let Err(e) = crate::document::rebuild(&doc.dir) {
        tracing::warn!("pagine del documento dopo la modifica: {e:#}");
    }
    // le pagine si ridisegnano qui, non nell'interfaccia
    if let Some(id) = doc.dir.file_name().and_then(|n| n.to_str()) {
        if let Err(e) = crate::document::Document::open(id) {
            tracing::warn!("pagine del documento dopo la modifica: {e:#}");
        }
    }
    if let Some(n) = NOTIFY.get() {
        n(doc.dir.clone());
    }
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
    changed(&doc);
    Ok(())
}

// ---------------------------------------------------------------------------
// Server MCP della suite Craft, in memoria
// ---------------------------------------------------------------------------

enum Server {
    Word(wordcraft_mcp::Server),
    Svg(vectorcraft_mcp::Server),
    Pdf(pdfcraft_automation::mcp::McpServer, u64),
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
    TX.get_or_init(|| {
        let (tx, rx) = std::sync::mpsc::channel::<Job>();
        std::thread::Builder::new()
            .name("documento-mcp".into())
            .spawn(move || {
                let mut session: Option<Session> = None;
                while let Ok(job) = rx.recv() {
                    job(&mut session);
                }
            })
            .expect("thread dei server MCP");
        Mutex::new(tx)
    })
}

/// Esegue `f` nel thread dei server MCP e ne aspetta il risultato.
fn on_session_thread<T: Send + 'static>(f: impl FnOnce(&mut Option<Session>) -> T + Send + 'static) -> Result<T> {
    let (tx, rx) = std::sync::mpsc::channel();
    let job: Job = Box::new(move |s| {
        let _ = tx.send(f(s));
    });
    session_thread().lock().unwrap().send(job).map_err(|_| anyhow!("server MCP fermi"))?;
    rx.recv().map_err(|_| anyhow!("server MCP: lavoro interrotto"))
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
        Kind::Word => matches!(name, "list_commands" | "execute" | "batch" | "type_text" | "select_text" | "inspect_document" | "render_page" | "undo" | "redo"),
        Kind::Svg => matches!(
            name,
            "list_commands" | "command_list" | "command_batch" | "render_preview" | "draw_path" | "draw_shape" | "set_paint" | "add_text" | "transform"
                | "create_graph" | "line" | "polygon" | "star" | "apply_effect" | "pathfinder" | "text_wrap" | "undo" | "redo"
        ),
        Kind::Pdf => matches!(
            name,
            "doc_info" | "doc_inspect" | "page_render" | "text_extract" | "text_find" | "text_lines" | "text_paragraphs" | "text_word_count" | "text_edit"
                | "content_list" | "content_update" | "content_delete" | "object_list" | "object_move" | "comment_list" | "comment_edit"
                | "comment_delete" | "comment_reply" | "link_list" | "link_add" | "page_insert_blank" | "page_delete" | "page_rotate"
                | "page_move" | "page_duplicate" | "page_number" | "doc_watermark" | "doc_background" | "edit_undo" | "edit_redo"
                | "command_list" | "command_run" | "command_batch"
        ),
        Kind::Text | Kind::Photo => false,
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
                let roots = vectorcraft_engine::file_access::AutomationRoots::new(Some(&root), Some(&root)).map_err(|e| anyhow!(e))?;
                let mut h = vectorcraft_mcp::Headless::new().with_automation_roots(roots);
                h.open(&json!({ "path": file.to_string_lossy() })).map_err(|e| anyhow!("VectorCraft: {e}"))?;
                Server::Svg(vectorcraft_mcp::Server::new(Box::new(h)))
            }
            Kind::Pdf => {
                let a = pdfcraft_automation::Automation::new().with_root(&root)?;
                Server::Pdf(pdfcraft_automation::mcp::McpServer::new(a), 0)
            }
            Kind::Text | Kind::Photo => bail!("nessun server MCP per questo documento"),
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
        Kind::Word | Kind::Pdf | Kind::Svg => match with_session(doc, |s, _| Ok(s.tools.clone())) {
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
                    }
                    defs.push(function(&name, &desc, params));
                }
            }
            Err(e) => tracing::warn!("strumenti del documento: {e:#}"),
        },
        Kind::Photo => {}
    }
    defs
}

/// Istruzioni per il modello.
pub fn system_note(doc: &DocRef, title: &str) -> String {
    let (what, how) = match doc.kind {
        Kind::Word => ("un documento Word", "gli strumenti word__* (il server MCP di WordCraft: word__inspect_document per vedere com'è fatto, word__execute / word__batch per i comandi di WordCraft, word__list_commands per cercarli, word__render_page per guardare una pagina)"),
        Kind::Pdf => ("un PDF", "gli strumenti pdf__* (il server MCP di PdfCraft: pdf__doc_inspect, pdf__text_find, pdf__text_edit, pdf__page_render per guardare una pagina…)"),
        Kind::Svg => ("un disegno SVG", "gli strumenti svg__* (il server MCP di VectorCraft: svg__render_preview per guardarlo, svg__draw_shape, svg__add_text, svg__command_batch…)"),
        Kind::Text => ("un testo (Markdown, .txt o .html)", "text_read, text_replace e text_insert (nel Markdown le formule in LaTeX con $…$ e $$…$$)"),
        Kind::Photo => ("una foto", "document_insert_image"),
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
    changed(doc);
    Ok(n)
}

fn mcp_call(doc: &DocRef, tool: &str, args: &Value) -> Result<Outcome> {
    if !allowed(doc.kind, tool) {
        bail!("strumento non disponibile: {tool}");
    }
    let before = std::fs::read(&doc.file)?;
    let n = snapshot(doc)?;
    let (tool_s, args_s) = (tool.to_string(), args.clone());
    let res = with_session(doc, move |s, doc| {
        let tool = tool_s.as_str();
        let mut args = args_s;
        if let (Server::Pdf(_, id), Some(o)) = (&s.server, args.as_object_mut()) {
            o.entry("doc").or_insert(json!(*id));
        }
        let ro = s.tools.iter().find(|t| t["name"] == tool).and_then(|t| t["annotations"]["readOnlyHint"].as_bool()).unwrap_or(false);
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
    let text = result_text(&r);
    let images = result_images(&r);
    let error = r["isError"].as_bool() == Some(true);
    let after = std::fs::read(&doc.file)?;
    let edit = if after != before {
        changed(doc);
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
    changed(doc);
    Ok(Outcome {
        text: format!("Fatto: il grafico {} è nel documento ({:.1} × {:.1} cm).", chart.id, width / CM, height / CM),
        short: "grafico inserito nel documento".into(),
        edit: Some(n),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
