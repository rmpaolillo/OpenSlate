//! Server MCP esterni per la chat AI, come in Claude Desktop o DSH: in
//! `ai.toml` una sezione per server,
//!
//! ```toml
//! [[mcp]]
//! name = "filesystem"
//! command = "npx"
//! args = ["-y", "@modelcontextprotocol/server-filesystem", "/Users/io/Documenti"]
//! env = { TOKEN = "keychain:MIO_TOKEN" }   # valori anche da env:, keychain:, dsh:
//! ```
//!
//! OpenSlate avvia il programma e gli parla in JSON-RPC sullo standard input
//! e output (il trasporto «stdio» di MCP); i suoi strumenti arrivano al
//! modello col nome `<server>__<strumento>`. I processi restano accesi fra una
//! domanda e l'altra e si riavviano se la configurazione cambia o se escono.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct McpServerCfg {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default = "yes")]
    pub enabled: bool,
    /// Secondi massimi per uno strumento.
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

fn yes() -> bool {
    true
}
fn default_timeout() -> u64 {
    120
}

struct Proc {
    cfg: McpServerCfg,
    child: Child,
    stdin: ChildStdin,
    lines: Receiver<String>,
    next_id: u64,
    tools: Vec<Value>,
}

impl Drop for Proc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Un server acceso: configurazione e strumenti (letti senza fermarlo) e il
/// processo, col suo lucchetto: una chiamata lunga a un server non blocca
/// gli altri server né le altre chat.
struct Entry {
    cfg: McpServerCfg,
    tools: Vec<Value>,
    proc: std::sync::Arc<Mutex<Proc>>,
}

/// L'elenco si tiene solo per un attimo (mai durante una chiamata).
static POOL: Mutex<Vec<Entry>> = Mutex::new(Vec::new());

fn pool() -> std::sync::MutexGuard<'static, Vec<Entry>> {
    POOL.lock().unwrap_or_else(|e| e.into_inner())
}

/// Nome sicuro per il modello: lettere, cifre, `_` e `-`.
fn safe(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).collect()
}

fn tool_name(server: &str, tool: &str) -> String {
    let n = format!("{}__{}", safe(server), safe(tool));
    n.chars().take(64).collect()
}

impl Proc {
    fn start(cfg: &McpServerCfg) -> Result<Proc> {
        let mut cmd = Command::new(&cfg.command);
        cmd.args(&cfg.args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
        for (k, v) in &cfg.env {
            let v = crate::ai::resolve_key(v).with_context(|| format!("{}: valore di {k} non trovato", cfg.name))?;
            cmd.env(k, v);
        }
        let mut child = cmd.spawn().with_context(|| format!("server MCP «{}»: non parte «{}»", cfg.name, cfg.command))?;
        let stdin = child.stdin.take().context("stdin")?;
        let stdout = child.stdout.take().context("stdout")?;
        let (tx, lines) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        let mut p = Proc { cfg: cfg.clone(), child, stdin, lines, next_id: 0, tools: Vec::new() };
        p.request(
            "initialize",
            json!({ "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "OpenSlate", "version": crate::update::current() } }),
            Duration::from_secs(30),
        )?;
        p.send(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))?;
        let mut tools = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let params = cursor.as_ref().map(|c| json!({ "cursor": c })).unwrap_or(json!({}));
            let r = p.request("tools/list", params, Duration::from_secs(30))?;
            tools.extend(r["tools"].as_array().cloned().unwrap_or_default());
            match r["nextCursor"].as_str() {
                Some(c) if !c.is_empty() && tools.len() < 500 => cursor = Some(c.to_string()),
                _ => break,
            }
        }
        p.tools = tools;
        Ok(p)
    }

    fn alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    fn send(&mut self, msg: &Value) -> Result<()> {
        writeln!(self.stdin, "{msg}")?;
        self.stdin.flush()?;
        Ok(())
    }

    fn request(&mut self, method: &str, params: Value, timeout: Duration) -> Result<Value> {
        self.next_id += 1;
        let id = self.next_id;
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))?;
        let end = Instant::now() + timeout;
        loop {
            let left = end.saturating_duration_since(Instant::now());
            let line = match self.lines.recv_timeout(left) {
                Ok(l) => l,
                Err(RecvTimeoutError::Timeout) => bail!("server MCP «{}»: nessuna risposta a {method} in {} s", self.cfg.name, timeout.as_secs()),
                Err(RecvTimeoutError::Disconnected) => bail!("server MCP «{}»: il programma è uscito", self.cfg.name),
            };
            let Ok(v) = serde_json::from_str::<Value>(line.trim()) else { continue };
            if v["id"].as_u64() != Some(id) {
                // notifiche o richieste del server: non servono qui
                continue;
            }
            if let Some(e) = v.get("error") {
                bail!("{}", e["message"].as_str().unwrap_or(&e.to_string()));
            }
            return Ok(v["result"].clone());
        }
    }
}

/// Avvia (o riusa) i server accesi in configurazione; ritorna gli errori di
/// quelli che non partono (la chat va avanti senza).
fn ensure(cfgs: &[McpServerCfg]) -> Vec<String> {
    let wanted: Vec<&McpServerCfg> = cfgs.iter().filter(|c| c.enabled && !c.name.trim().is_empty()).collect();
    let missing: Vec<McpServerCfg> = {
        let mut pool = pool();
        // via quelli tolti dalla configurazione o usciti (uno occupato in una
        // chiamata è vivo per forza)
        pool.retain(|e| wanted.iter().any(|c| **c == e.cfg) && e.proc.try_lock().map(|mut p| p.alive()).unwrap_or(true));
        wanted.iter().filter(|c| !pool.iter().any(|e| e.cfg == ***c)).map(|c| (*c).clone()).collect()
    };
    // si avviano fuori dal lucchetto (fino a 30 s l'uno)
    let mut errors = Vec::new();
    for c in missing {
        match Proc::start(&c) {
            Ok(p) => {
                let tools = p.tools.clone();
                let mut pool = pool();
                if !pool.iter().any(|e| e.cfg == c) {
                    pool.push(Entry { cfg: c, tools, proc: std::sync::Arc::new(Mutex::new(p)) });
                }
            }
            Err(e) => errors.push(format!("{e:#}")),
        }
    }
    errors
}

/// Gli strumenti dei server esterni (formato OpenAI) e una riga di istruzioni.
pub fn definitions(cfgs: &[McpServerCfg]) -> (Vec<Value>, String) {
    if cfgs.iter().all(|c| !c.enabled) {
        return (Vec::new(), String::new());
    }
    for e in ensure(cfgs) {
        tracing::warn!("{e}");
    }
    let pool = pool();
    let mut defs = Vec::new();
    let mut names = Vec::new();
    for p in pool.iter() {
        names.push(p.cfg.name.clone());
        for t in &p.tools {
            let Some(n) = t["name"].as_str() else { continue };
            let mut params = t["inputSchema"].clone();
            if !params.is_object() {
                params = json!({ "type": "object", "properties": {} });
            }
            defs.push(json!({ "type": "function", "function": {
                "name": tool_name(&p.cfg.name, n),
                "description": t["description"].as_str().unwrap_or(""),
                "parameters": params,
            }}));
        }
    }
    let note = if names.is_empty() {
        String::new()
    } else {
        format!("\n- server MCP esterni ({}): i loro strumenti hanno il nome del server davanti (server__strumento).", names.join(", "))
    };
    (defs, note)
}

/// Esegue lo strumento `name` se è di un server esterno: (testo per il
/// modello, esito breve, immagini).
pub fn execute(name: &str, args: &Value) -> Option<(String, String, Vec<Vec<u8>>)> {
    let (proc, tool, timeout) = {
        let pool = pool();
        let e = pool.iter().find(|e| name.starts_with(&format!("{}__", safe(&e.cfg.name))))?;
        let tool = e.tools.iter().filter_map(|t| t["name"].as_str()).find(|t| tool_name(&e.cfg.name, t) == name)?.to_string();
        (e.proc.clone(), tool, Duration::from_secs(e.cfg.timeout.clamp(5, 3600)))
    };
    let mut p = proc.lock().unwrap_or_else(|e| e.into_inner());
    let out = match p.request("tools/call", json!({ "name": tool, "arguments": args }), timeout) {
        Ok(r) => {
            use base64::Engine;
            let text: String = r["content"].as_array().into_iter().flatten().filter_map(|c| c["text"].as_str()).collect::<Vec<_>>().join("\n");
            let images: Vec<Vec<u8>> = r["content"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|c| c["type"] == "image")
                .filter_map(|c| base64::engine::general_purpose::STANDARD.decode(c["data"].as_str()?).ok())
                .collect();
            let text = if text.is_empty() && !r["structuredContent"].is_null() { r["structuredContent"].to_string() } else { text };
            if r["isError"].as_bool() == Some(true) {
                (format!("ERRORE: {text}"), format!("errore: {}", text.lines().next().unwrap_or("")), images)
            } else {
                let short = format!("{} caratteri", text.chars().count());
                (text, short, images)
            }
        }
        Err(e) => (format!("ERRORE: {e:#}"), format!("errore: {e:#}"), Vec::new()),
    };
    Some(out)
}

/// Strumento esterno? (per l'etichetta nella chat)
pub fn server_of(name: &str) -> Option<String> {
    pool().iter().find(|e| name.starts_with(&format!("{}__", safe(&e.cfg.name)))).map(|e| e.cfg.name.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un server MCP finto in Python: un solo strumento che fa l'eco.
    #[test]
    fn server_esterno_stdio() {
        if Command::new("python3").arg("--version").output().is_err() {
            return;
        }
        let script = r#"
import sys, json
for line in sys.stdin:
    m = json.loads(line)
    if "id" not in m: continue
    if m["method"] == "initialize": r = {"protocolVersion": "2025-06-18", "capabilities": {"tools": {}}, "serverInfo": {"name": "eco"}}
    elif m["method"] == "tools/list": r = {"tools": [{"name": "eco", "description": "ripete", "inputSchema": {"type": "object", "properties": {"t": {"type": "string"}}}}]}
    elif m["method"] == "tools/call": r = {"content": [{"type": "text", "text": "eco: " + m["params"]["arguments"]["t"]}]}
    else: r = {}
    print(json.dumps({"jsonrpc": "2.0", "id": m["id"], "result": r}), flush=True)
"#;
        let cfg = McpServerCfg { name: "prova eco".into(), command: "python3".into(), args: vec!["-c".into(), script.into()], env: Default::default(), enabled: true, timeout: 20 };
        let (defs, note) = definitions(std::slice::from_ref(&cfg));
        assert_eq!(defs[0]["function"]["name"], "prova_eco__eco");
        assert!(note.contains("prova eco"));
        let (text, _, _) = execute("prova_eco__eco", &json!({ "t": "ciao" })).unwrap();
        assert_eq!(text, "eco: ciao");
        assert!(execute("altro__x", &json!({})).is_none());
        // spento in configurazione: il processo si ferma
        let off = McpServerCfg { enabled: false, ..cfg };
        let _ = ensure(std::slice::from_ref(&off));
        assert!(server_of("prova_eco__eco").is_none());
    }
}
