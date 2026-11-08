//! AI per Android: la catena di provider della desktop (`src/ai.rs`): si
//! prova il primo; se non risponde (spento, chiave mancante, errore, modello
//! senza visione…) si passa al successivo. Tutti parlano il formato OpenAI
//! (`/chat/completions`): LM Studio, Ollama, DeepSeek, Gemini, Claude…
//!
//! Prompt, scelta automatica del modello, filtro del ragionamento e pulizia
//! della trascrizione sono quelli della desktop. Le chiavi le conserva Kotlin
//! (Android Keystore) e arrivano solo per la richiesta: qui non si salvano.

use base64::Engine;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Istruzioni per la trascrizione (uguali a `ai::OCR_PROMPT` della desktop).
pub const OCR_PROMPT: &str = r#"Sei un trascrittore di scrittura a mano. La pagina può contenere qualsiasi cosa: appunti, liste, promemoria, bozze di testi o lettere, verbali di riunione, schemi, tabelle, conti, esercizi o formule. Trascrivi ESATTAMENTE ciò che è scritto nell'immagine, in Markdown.

Regole:
1. Fedeltà assoluta. NON correggere ortografia, grammatica, numeri, calcoli o contenuti sbagliati. NON completare, NON riassumere, NON aggiungere spiegazioni. Se è scritto 2+2=5, trascrivi 2+2=5.
2. Mantieni la lingua in cui è scritta la pagina, l'ordine e gli a capo del foglio.
3. Titoli come titoli Markdown (##) solo se sul foglio si vedono come titoli (sottolineati, più grandi, isolati). Elenchi come elenchi Markdown; caselle da spuntare come "- [ ]" o "- [x]". Tabelle come tabelle Markdown.
4. Solo se ci sono formule o simboli matematici: in linea tra $...$, su una riga propria tra $$...$$, in LaTeX standard (\frac, \sqrt, \sum, \int, \begin{cases}, \begin{pmatrix}…).
5. Parole barrate: ~~...~~ (o \cancel{...} dentro una formula).
6. Parole o simboli illeggibili: [?]. Se sei incerto fra due letture scrivi la più probabile seguita da [?].
7. Disegni, grafici, schemi, piani cartesiani, tabelle disegnate a mano: una riga sola [figura: breve descrizione | riquadro: x1,y1,x2,y2], dove x1,y1 è l'angolo in alto a sinistra e x2,y2 quello in basso a destra della figura, in millesimi della larghezza e dell'altezza dell'immagine (0-1000). Il testo scritto fuori dalla figura si trascrive normalmente.
8. Rispondi SOLO con il Markdown trascritto: niente ``` attorno, niente commenti, niente introduzioni."#;

/// Istruzioni per la chat (uguali a `ai::CHAT_PROMPT` della desktop).
pub const CHAT_PROMPT: &str = r#"Sei un assistente versatile dentro un'app per scrivere a mano su un pad digitale: aiuti con quello che serve all'utente, per esempio appunti, documenti da leggere o firmare, testi da scrivere o rivedere, idee, studio, lavoro, domande di ogni genere. Rispondi nella lingua in cui ti scrive l'utente, in Markdown. Se servono formule scrivile in LaTeX con $...$ in linea e $$...$$ su riga propria (mai \( \) o \[ \]).
Quando ricevi una pagina scritta a mano (immagine e/o trascrizione), la trascrizione è fedele al foglio, errori compresi; se l'immagine e la trascrizione non coincidono, fidati dell'immagine e dillo. Fai quello che ti viene chiesto: correggi, giudichi o completi solo se te lo chiedono. Sii preciso e conciso."#;

/// Sotto questa soglia (miliardi) un modello scelto da solo non si usa.
const MIN_AUTO_SIZE: f32 = 3.0;

#[derive(uniffi::Record, Clone, Debug)]
pub struct AiProvider {
    pub name: String,
    /// Indirizzo base, es. "https://api.deepseek.com/v1".
    pub url: String,
    /// Vuoto = lo si sceglie fra quelli di `/models`.
    pub model: String,
    /// Chiave API (vuota per i server locali).
    pub key: String,
    /// Il modello legge le immagini.
    pub vision: bool,
    pub enabled: bool,
    /// Claude non accetta `temperature` insieme ad altri parametri.
    pub no_temperature: bool,
    pub max_tokens: Option<u32>,
    pub timeout_s: u32,
}

#[derive(uniffi::Record, Clone, Debug)]
pub struct ChatMsg {
    /// "system", "user" o "assistant".
    pub role: String,
    pub text: String,
    /// Immagine (PNG o JPEG): il foglio, una foto…
    pub image: Option<Vec<u8>>,
}

#[derive(uniffi::Enum, Clone, Debug, PartialEq)]
pub enum AiEvent {
    /// Si prova questo provider con questo modello.
    Trying { provider: String, model: String },
    /// Provider saltato o fallito: si passa al prossimo.
    Skipped { provider: String, why: String },
    /// Pezzo di risposta.
    Delta { text: String },
    /// Pezzo del ragionamento del modello (mostrato a parte).
    Thinking { text: String },
    /// Il modello usa uno strumento (es. «Scrivo sul foglio»).
    Tool { label: String },
    /// Strumento finito, con l'esito breve.
    ToolDone { label: String, result: String },
    /// Il tutor ha cambiato la lavagna: rileggerla.
    BoardChanged,
}

/// Il risultato di uno strumento.
#[derive(Default)]
pub struct ToolOut {
    /// Per il modello.
    pub text: String,
    /// Per la chat.
    pub short: String,
    /// Immagini per il modello (se le vede), es. il foglio coi nomi.
    pub images: Vec<Vec<u8>>,
    /// Ha cambiato la lavagna.
    pub changed: bool,
}

/// Strumenti offerti al modello (come `ToolCtx` della desktop).
pub trait ToolRunner: Send + Sync {
    fn defs(&self) -> Vec<Value>;
    /// Da aggiungere alle istruzioni.
    fn note(&self) -> String;
    fn label(&self, name: &str) -> String;
    fn run(&self, name: &str, args: &Value) -> ToolOut;
}

/// Giri di strumenti al massimo (come la desktop).
const MAX_ROUNDS: usize = 8;

#[derive(Default, Clone)]
struct ToolCall {
    id: String,
    name: String,
    args: String,
}

struct RoundOut {
    text: String,
    raw: String,
    reasoning: String,
    calls: Vec<ToolCall>,
}

#[derive(uniffi::Record, Clone, Debug)]
pub struct AiReply {
    pub text: String,
    pub provider: String,
    pub model: String,
}

#[derive(uniffi::Error, thiserror::Error, Debug)]
pub enum AiError {
    #[error("{reason}")]
    Failed { reason: String },
    #[error("interrotto")]
    Cancelled,
}

fn fail(s: impl Into<String>) -> AiError {
    AiError::Failed { reason: s.into() }
}

/// Riceve tentativi e pezzi di risposta; `false` = interrompi.
#[uniffi::export(with_foreign)]
pub trait AiListener: Send + Sync {
    fn on_event(&self, event: AiEvent) -> bool;
}

#[derive(Clone, Copy, PartialEq)]
enum Task {
    Ocr,
    Chat,
}

// ---------------------------------------------------------------------------
// Scelta del modello (come `ai::choose_model_why`)
// ---------------------------------------------------------------------------

pub fn model_size(id: &str) -> Option<f32> {
    id.to_lowercase()
        .split(|c: char| matches!(c, '-' | '_' | ':' | '/' | ' ' | '@'))
        .filter_map(|t| t.strip_suffix('b')?.parse::<f32>().ok())
        .reduce(f32::max)
}

pub fn choose_model_why(ids: &[String], vision: bool) -> Result<String, String> {
    const VISION_HINTS: [&str; 11] = ["vl", "vision", "ocr", "llava", "pixtral", "gemma-3", "gemma3", "minicpm-v", "omni", "kimi-vl", "qwen3.5"];
    let usable: Vec<&String> = ids
        .iter()
        .filter(|id| {
            let l = id.to_lowercase();
            !l.contains("embed") && !l.contains("rerank") && !l.contains("whisper") && !l.contains("tts")
        })
        .collect();
    if usable.is_empty() {
        return Err("nessun modello caricato".into());
    }
    let is_vision = |id: &str| {
        let l = id.to_lowercase();
        VISION_HINTS.iter().any(|h| l.contains(h))
    };
    let cands: Vec<&String> = usable.iter().copied().filter(|id| !vision || is_vision(id)).collect();
    if cands.is_empty() {
        return Err(format!(
            "nessun modello con visione caricato (ci sono: {}). Carica ad es. Qwen3-VL-8B",
            usable.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
        ));
    }
    let size = |id: &str| model_size(id).unwrap_or(7.0);
    let best = cands.iter().copied().max_by(|a, b| size(a).total_cmp(&size(b))).unwrap();
    if size(best) < MIN_AUTO_SIZE {
        return Err(format!("solo modelli troppo piccoli per rispondere bene ({best}): caricane uno più grande o scrivilo nelle impostazioni"));
    }
    Ok(best.to_string())
}

// ---------------------------------------------------------------------------
// Testo: ragionamento e pulizia (come la desktop)
// ---------------------------------------------------------------------------

/// Separa il ragionamento `<think>…</think>` dal testo visibile, in streaming.
#[derive(Default)]
struct ThinkFilter {
    full: String,
    sent: usize,
    think_sent: usize,
}

impl ThinkFilter {
    fn visible(&self) -> &str {
        let t = self.full.trim_start();
        if let Some(rest) = t.strip_prefix("<think>") {
            match rest.find("</think>") {
                Some(i) => rest[i + "</think>".len()..].trim_start(),
                None => "",
            }
        } else if "<think>".starts_with(t) {
            ""
        } else {
            t
        }
    }

    fn thinking(&self) -> &str {
        let t = self.full.trim_start();
        match t.strip_prefix("<think>") {
            Some(rest) => match rest.find("</think>") {
                Some(i) => &rest[..i],
                None => {
                    let keep = (1..="</think>".len()).rev().find(|&k| rest.ends_with(&"</think>"[..k])).unwrap_or(0);
                    &rest[..rest.len() - keep]
                }
            },
            None => "",
        }
    }

    /// Aggiunge un pezzo: nuovo testo visibile e nuovo ragionamento.
    fn push(&mut self, piece: &str) -> (Option<String>, Option<String>) {
        self.full.push_str(piece);
        let vis = self.visible();
        let v = if vis.len() > self.sent && vis.is_char_boundary(self.sent) {
            let d = vis[self.sent..].to_string();
            self.sent = vis.len();
            Some(d)
        } else {
            None
        };
        let th = self.thinking();
        let t = if th.len() > self.think_sent && th.is_char_boundary(self.think_sent) {
            let d = th[self.think_sent..].to_string();
            self.think_sent = th.len();
            Some(d)
        } else {
            None
        };
        (v, t)
    }
}

/// Toglie il ragionamento rimasto in testa.
pub fn clean_reply(s: &str) -> String {
    let s = match s.rfind("</think>") {
        Some(i) => &s[i + "</think>".len()..],
        None => s,
    };
    s.trim().to_string()
}

/// Per la trascrizione: via i ``` attorno, `\( \)` / `\[ \]` → `$` / `$$`.
#[uniffi::export]
pub fn tidy_markdown(s: String) -> String {
    let mut t = clean_reply(&s);
    if t.starts_with("```") {
        if let Some(nl) = t.find('\n') {
            t = t[nl + 1..].to_string();
        }
        if let Some(end) = t.rfind("```") {
            if t[end + 3..].trim().is_empty() {
                t.truncate(end);
            }
        }
    }
    let mut out = String::with_capacity(t.len());
    let mut chars = t.chars().peekable();
    let mut prev_backslash = false;
    while let Some(c) = chars.next() {
        if c == '\\' && !prev_backslash {
            match chars.peek() {
                Some('(') | Some(')') => {
                    chars.next();
                    out.push('$');
                    continue;
                }
                Some('[') | Some(']') => {
                    chars.next();
                    out.push_str("$$");
                    continue;
                }
                _ => {}
            }
            prev_backslash = true;
            out.push(c);
            continue;
        }
        prev_backslash = false;
        out.push(c);
    }
    out.trim().to_string()
}

// ---------------------------------------------------------------------------
// HTTP
// ---------------------------------------------------------------------------

fn base(p: &AiProvider) -> &str {
    p.url.trim().trim_end_matches('/')
}

fn agent(p: &AiProvider) -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(p.timeout_s.max(30) as u64))
        .build()
}

fn with_auth(req: ureq::Request, key: &str) -> ureq::Request {
    let key = key.trim();
    if key.is_empty() {
        req
    } else {
        req.set("Authorization", &format!("Bearer {key}"))
    }
}

fn error_text(code: u16, body: &str) -> String {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v["error"]["message"].as_str().or(v["error"].as_str()).or(v["message"].as_str()).map(String::from))
        .map(|m| format!("errore {code}: {m}"))
        .unwrap_or_else(|| format!("errore {code}: {}", body.chars().take(200).collect::<String>()))
}

fn transport(e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(code, r) => error_text(code, &r.into_string().unwrap_or_default()),
        ureq::Error::Transport(t) => match t.kind() {
            ureq::ErrorKind::ConnectionFailed | ureq::ErrorKind::Dns => "non raggiungibile (spento o fuori dalla rete?)".into(),
            _ => t.to_string(),
        },
    }
}

/// I modelli del provider (`/models`).
#[uniffi::export]
pub fn ai_list_models(provider: AiProvider) -> Result<Vec<String>, AiError> {
    let url = format!("{}/models", base(&provider));
    let resp = with_auth(agent(&provider).get(&url), &provider.key).call().map_err(|e| fail(transport(e)))?;
    let v: Value = resp.into_json().map_err(|e| fail(e.to_string()))?;
    Ok(v["data"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| m["id"].as_str().map(|s| s.trim_start_matches("models/").to_string()))
        .collect())
}

fn pick_model(p: &AiProvider, vision: bool) -> Result<String, String> {
    if !p.model.trim().is_empty() {
        return Ok(p.model.trim().to_string());
    }
    let ids = ai_list_models(p.clone()).map_err(|e| e.to_string())?;
    choose_model_why(&ids, vision)
}

fn message_json(m: &ChatMsg, send_image: bool) -> Value {
    match (&m.image, send_image) {
        (Some(img), true) => {
            let b64 = base64::engine::general_purpose::STANDARD.encode(img);
            let mime = if img.starts_with(&[0xFF, 0xD8, 0xFF]) { "image/jpeg" } else { "image/png" };
            let mut parts = Vec::new();
            if !m.text.is_empty() {
                parts.push(json!({ "type": "text", "text": m.text }));
            }
            parts.push(json!({ "type": "image_url", "image_url": { "url": format!("data:{mime};base64,{b64}") } }));
            json!({ "role": m.role, "content": parts })
        }
        _ => json!({ "role": m.role, "content": m.text }),
    }
}

/// Chi risponde, detto al modello (come `ai::identity` della desktop).
fn identity(p: &AiProvider, model: &str) -> String {
    let host = p.url.split("://").nth(1).unwrap_or(&p.url).split('/').next().unwrap_or("");
    format!(
        "Chi sei: in questa chat risponde il modello «{model}» del servizio «{}» ({host}), scelto dall'utente nelle impostazioni dell'app. \
         Se ti chiedono chi sei o che modello sei, rispondi con questo. Non dire di essere un altro modello o di un'altra azienda: \
         se di questo modello non sai altro, dillo invece di inventare.",
        p.name
    )
}

/// Inoltra gli eventi e ricorda se la risposta è iniziata.
struct Watch<'a> {
    inner: &'a dyn AiListener,
    started: AtomicBool,
}

impl Watch<'_> {
    fn send(&self, event: AiEvent) -> Result<(), AiError> {
        if matches!(event, AiEvent::Delta { .. } | AiEvent::ToolDone { .. }) {
            self.started.store(true, Ordering::Relaxed);
        }
        if self.inner.on_event(event) {
            Ok(())
        } else {
            Err(AiError::Cancelled)
        }
    }

    /// Un pezzo di testo del modello, separato dal ragionamento.
    fn piece(&self, filt: &mut ThinkFilter, piece: &str) -> Result<(), AiError> {
        let (v, t) = filt.push(piece);
        if let Some(text) = t {
            self.send(AiEvent::Thinking { text })?;
        }
        if let Some(text) = v {
            self.send(AiEvent::Delta { text })?;
        }
        Ok(())
    }
}

/// Un giro: invia e legge la risposta (testo e strumenti chiesti).
fn round(p: &AiProvider, body: &Value, w: &Watch) -> Result<RoundOut, AiError> {
    let url = format!("{}/chat/completions", base(p));
    let resp = with_auth(agent(p).post(&url), &p.key)
        .set("Accept", "text/event-stream")
        .send_json(body.clone())
        .map_err(|e| fail(transport(e)))?;
    let ctype = resp.content_type().to_string();
    let mut filt = ThinkFilter::default();
    let mut reasoning = String::new();
    let mut calls: Vec<ToolCall> = Vec::new();
    let add_call = |i: usize, c: &Value, calls: &mut Vec<ToolCall>| {
        while calls.len() <= i {
            calls.push(ToolCall::default());
        }
        let t = &mut calls[i];
        if let Some(id) = c["id"].as_str() {
            t.id = id.to_string();
        }
        if let Some(n) = c["function"]["name"].as_str() {
            t.name.push_str(n);
        }
        match &c["function"]["arguments"] {
            Value::String(a) => t.args.push_str(a),
            Value::Null => {}
            other => t.args.push_str(&other.to_string()),
        }
    };
    if ctype.contains("json") {
        // server che ignorano "stream": una risposta sola
        let mut txt = String::new();
        resp.into_reader().read_to_string(&mut txt).map_err(|e| fail(e.to_string()))?;
        let v: Value = serde_json::from_str(&txt).map_err(|e| fail(e.to_string()))?;
        let m = &v["choices"][0]["message"];
        if let Some(r) = m["reasoning_content"].as_str().filter(|r| !r.is_empty()) {
            reasoning.push_str(r);
            w.send(AiEvent::Thinking { text: r.to_string() })?;
        }
        match m["content"].as_str() {
            Some(text) => w.piece(&mut filt, text)?,
            None if m["tool_calls"].is_null() => {
                return Err(fail(format!("risposta inattesa: {}", txt.chars().take(200).collect::<String>())))
            }
            None => {}
        }
        for (i, c) in m["tool_calls"].as_array().into_iter().flatten().enumerate() {
            add_call(i, c, &mut calls);
        }
    } else {
        let started = Instant::now();
        for line in BufReader::new(resp.into_reader()).lines() {
            let line = line.map_err(|e| fail(format!("lettura: {e}")))?;
            let Some(data) = line.strip_prefix("data:") else { continue };
            let data = data.trim();
            if data == "[DONE]" {
                break;
            }
            let Ok(v) = serde_json::from_str::<Value>(data) else { continue };
            if let Some(msg) = v["error"]["message"].as_str() {
                return Err(fail(msg.to_string()));
            }
            let delta = &v["choices"][0]["delta"];
            if let Some(r) = delta["reasoning_content"].as_str().filter(|r| !r.is_empty()) {
                reasoning.push_str(r);
                w.send(AiEvent::Thinking { text: r.to_string() })?;
            }
            if let Some(piece) = delta["content"].as_str() {
                w.piece(&mut filt, piece)?;
            }
            for (j, c) in delta["tool_calls"].as_array().into_iter().flatten().enumerate() {
                let i = c["index"].as_u64().map(|x| x as usize).unwrap_or(j);
                add_call(i, c, &mut calls);
            }
        }
        tracing::info!("{}: risposta in {:.1} s", p.name, started.elapsed().as_secs_f32());
    }
    calls.retain(|c| !c.name.is_empty());
    for (i, c) in calls.iter_mut().enumerate() {
        if c.id.is_empty() {
            c.id = format!("call_{i}");
        }
        if c.args.trim().is_empty() {
            c.args = "{}".into();
        }
    }
    Ok(RoundOut { text: clean_reply(filt.visible()), raw: filt.full.clone(), reasoning, calls })
}

/// Una richiesta, con gli strumenti se ci sono (come `ai::call` della
/// desktop). Restituisce il testo visibile.
fn call(p: &AiProvider, model: &str, task: Task, msgs: &[ChatMsg], send_image: bool, tools: Option<&dyn ToolRunner>, w: &Watch) -> Result<String, AiError> {
    let mut wire: Vec<Value> = msgs.iter().map(|m| message_json(m, send_image)).collect();
    let mut extra = String::new();
    if let Some(t) = tools {
        extra.push_str(&t.note());
    }
    if task == Task::Chat {
        extra.push_str("\n\n");
        extra.push_str(&identity(p, model));
    }
    if !extra.is_empty() {
        match wire.first_mut() {
            Some(first) if first["role"] == "system" => {
                let s = format!("{}{extra}", first["content"].as_str().unwrap_or(""));
                first["content"] = json!(s);
            }
            _ => wire.insert(0, json!({ "role": "system", "content": extra.trim() })),
        }
    }
    let mut tools = tools;
    let mut answer = String::new();
    let mut n = 0;
    while n < MAX_ROUNDS {
        let mut body = json!({ "model": model, "stream": true, "messages": wire });
        if task == Task::Ocr && !p.no_temperature {
            body["temperature"] = json!(0.0);
        }
        if let Some(m) = p.max_tokens {
            body["max_tokens"] = json!(m);
        }
        if let Some(t) = tools {
            body["tools"] = json!(t.defs());
            // all'ultimo giro deve rispondere, non usare ancora gli strumenti
            body["tool_choice"] = json!(if n + 1 < MAX_ROUNDS { "auto" } else { "none" });
        }
        let out = match round(p, &body, w) {
            Ok(o) => o,
            // server che non accetta gli strumenti: si riprova senza
            Err(AiError::Failed { reason }) if tools.is_some() && n == 0 && answer.is_empty() && reason.to_lowercase().contains("tool") => {
                tracing::info!("{}: strumenti non accettati ({reason}), riprovo senza", p.name);
                tools = None;
                continue;
            }
            Err(e) => return Err(e),
        };
        if !out.text.is_empty() {
            if !answer.is_empty() {
                answer.push_str("\n\n");
            }
            answer.push_str(&out.text);
        }
        let last = n + 1 >= MAX_ROUNDS;
        let Some(t) = tools.filter(|_| !out.calls.is_empty() && !last) else {
            return Ok(clean_reply(&answer));
        };
        // il modello ha chiesto degli strumenti: si eseguono e si continua
        let calls: Vec<Value> = out
            .calls
            .iter()
            .map(|c| json!({ "id": c.id, "type": "function", "function": { "name": c.name, "arguments": c.args } }))
            .collect();
        let mut asst = json!({ "role": "assistant", "content": out.raw, "tool_calls": calls });
        if !out.reasoning.is_empty() {
            asst["reasoning_content"] = json!(out.reasoning);
        }
        wire.push(asst);
        let mut pending_images = Vec::new();
        for c in &out.calls {
            let args: Value = serde_json::from_str(&c.args).unwrap_or(json!({}));
            let label = t.label(&c.name);
            w.send(AiEvent::Tool { label: label.clone() })?;
            let res = t.run(&c.name, &args);
            w.send(AiEvent::ToolDone { label, result: res.short.clone() })?;
            if res.changed {
                w.send(AiEvent::BoardChanged)?;
            }
            wire.push(json!({ "role": "tool", "tool_call_id": c.id, "content": res.text }));
            if send_image && !res.images.is_empty() {
                let mut parts = vec![json!({ "type": "text", "text": format!("Immagine restituita da {}:", c.name) })];
                for png in res.images.iter().take(2) {
                    let b = base64::engine::general_purpose::STANDARD.encode(png);
                    parts.push(json!({ "type": "image_url", "image_url": { "url": format!("data:image/png;base64,{b}") } }));
                }
                pending_images.push(json!({ "role": "user", "content": parts }));
            }
        }
        // le immagini vanno dopo tutte le risposte degli strumenti
        wire.append(&mut pending_images);
        n += 1;
    }
    Ok(clean_reply(&answer))
}

/// La catena: il primo provider che risponde (come `ai::run_chat`, senza strumenti).
fn run(providers: &[AiProvider], task: Task, msgs: &[ChatMsg], tools: Option<&dyn ToolRunner>, l: &dyn AiListener) -> Result<AiReply, AiError> {
    let has_image = msgs.iter().any(|m| m.image.is_some());
    let mut fails = Vec::new();
    for p in providers.iter().filter(|p| p.enabled) {
        let mut skip = |why: String| -> Result<(), AiError> {
            fails.push(format!("{}: {why}", p.name));
            if l.on_event(AiEvent::Skipped { provider: p.name.clone(), why }) {
                Ok(())
            } else {
                Err(AiError::Cancelled)
            }
        };
        if task == Task::Ocr && !p.vision {
            skip("non legge immagini".into())?;
            continue;
        }
        // un servizio in rete senza chiave non risponderebbe
        if p.url.trim().starts_with("https://") && p.key.trim().is_empty() {
            skip("manca la chiave".into())?;
            continue;
        }
        let mut send_image = has_image && p.vision;
        let mut picked = pick_model(p, send_image);
        if picked.is_err() && send_image && task == Task::Chat {
            // in chat, senza un modello che vede, si va avanti col solo testo
            if let Ok(m) = pick_model(p, false) {
                send_image = false;
                picked = Ok(m);
            }
        }
        let model = match picked {
            Ok(m) => m,
            Err(e) => {
                skip(e)?;
                continue;
            }
        };
        if !l.on_event(AiEvent::Trying { provider: p.name.clone(), model: model.clone() }) {
            return Err(AiError::Cancelled);
        }
        let w = Watch { inner: l, started: AtomicBool::new(false) };
        match call(p, &model, task, msgs, send_image, tools, &w) {
            Ok(text) if !text.trim().is_empty() => return Ok(AiReply { text, provider: p.name.clone(), model }),
            // ha lavorato con gli strumenti ma non ha scritto niente
            Ok(_) if w.started.load(Ordering::Relaxed) => {
                let text = "Fatto (i passaggi sono qui sopra).".to_string();
                let _ = l.on_event(AiEvent::Delta { text: text.clone() });
                return Ok(AiReply { text, provider: p.name.clone(), model });
            }
            Ok(_) => skip("risposta vuota".into())?,
            Err(AiError::Cancelled) => return Err(AiError::Cancelled),
            // a risposta iniziata non si riparte da un altro provider
            Err(e) if w.started.load(Ordering::Relaxed) => return Err(fail(format!("{} si è interrotto: {e}", p.name))),
            Err(e) => skip(e.to_string())?,
        }
    }
    if fails.is_empty() {
        return Err(fail("nessun servizio AI attivo: aggiungine uno nelle impostazioni AI"));
    }
    Err(fail(format!("nessun servizio AI disponibile:\n• {}", fails.join("\n• "))))
}

/// I servizi AI dal QR code della desktop (Impostazioni AI → «Collega il
/// telefono»), con le loro chiavi: vedi `src/aisync.rs`.
#[uniffi::export]
pub fn ai_import_qr(text: String) -> Result<Vec<AiProvider>, AiError> {
    let list = crate::aisync::decode(&text).map_err(fail)?;
    Ok(list
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

/// Chat. `msgs` senza il messaggio di sistema: lo aggiunge questa funzione.
#[uniffi::export]
pub fn ai_chat(providers: Vec<AiProvider>, msgs: Vec<ChatMsg>, listener: Arc<dyn AiListener>) -> Result<AiReply, AiError> {
    let mut all = vec![ChatMsg { role: "system".into(), text: CHAT_PROMPT.into(), image: None }];
    all.extend(msgs);
    run(&providers, Task::Chat, &all, None, listener.as_ref())
}

/// Chat col tutor sulla lavagna: legge il foglio e ci scrive (con
/// `lavagna_leggi` e `lavagna_annota`, come la desktop).
#[uniffi::export]
pub fn ai_chat_board(
    providers: Vec<AiProvider>,
    msgs: Vec<ChatMsg>,
    board: Arc<crate::board::BoardModel>,
    view: crate::tutor::BoardView,
    listener: Arc<dyn AiListener>,
) -> Result<AiReply, AiError> {
    let tutor = crate::tutor::Tutor::new(board, view);
    let mut all = vec![ChatMsg { role: "system".into(), text: CHAT_PROMPT.into(), image: None }];
    all.extend(msgs);
    run(&providers, Task::Chat, &all, Some(&tutor), listener.as_ref())
}

/// Trascrizione fedele di una pagina (PNG) in Markdown.
#[uniffi::export]
pub fn ai_transcribe(providers: Vec<AiProvider>, png: Vec<u8>, listener: Arc<dyn AiListener>) -> Result<AiReply, AiError> {
    let msgs = vec![
        ChatMsg { role: "system".into(), text: OCR_PROMPT.into(), image: None },
        ChatMsg { role: "user".into(), text: "Trascrivi questa pagina.".into(), image: Some(png) },
    ];
    let mut r = run(&providers, Task::Ocr, &msgs, None, listener.as_ref())?;
    r.text = tidy_markdown(r.text);
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::TcpListener;
    use std::sync::Mutex;

    struct Log(Mutex<Vec<AiEvent>>);
    impl AiListener for Log {
        fn on_event(&self, e: AiEvent) -> bool {
            self.0.lock().unwrap().push(e);
            true
        }
    }

    fn server(reply: &'static str) -> (String, Arc<Mutex<Vec<String>>>) {
        server_seq(vec![reply])
    }

    /// Server finto: risponde alle richieste con `replies` in ordine (l'ultima
    /// si ripete) e ricorda le richieste.
    fn server_seq(replies: Vec<&'static str>) -> (String, Arc<Mutex<Vec<String>>>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/v1", l.local_addr().unwrap());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let s2 = seen.clone();
        std::thread::spawn(move || {
            for mut c in l.incoming().flatten() {
                let mut buf = vec![0u8; 1 << 16];
                let mut got = Vec::new();
                loop {
                    let n = std::io::Read::read(&mut c, &mut buf).unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    got.extend_from_slice(&buf[..n]);
                    let t = String::from_utf8_lossy(&got).to_string();
                    if let Some(h) = t.find("\r\n\r\n") {
                        let len = t[..h]
                            .lines()
                            .find_map(|l| l.to_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap_or(0)))
                            .unwrap_or(0);
                        if got.len() >= h + 4 + len {
                            break;
                        }
                    }
                }
                let k = {
                    let mut seen = s2.lock().unwrap();
                    seen.push(String::from_utf8_lossy(&got).to_string());
                    seen.len() - 1
                };
                let reply = replies[k.min(replies.len() - 1)];
                let _ = write!(c, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n{reply}", reply.len());
            }
        });
        (url, seen)
    }

    fn prov(name: &str, url: &str) -> AiProvider {
        AiProvider {
            name: name.into(),
            url: url.into(),
            model: "prova-8b".into(),
            key: String::new(),
            vision: true,
            enabled: true,
            no_temperature: false,
            max_tokens: None,
            timeout_s: 30,
        }
    }

    #[test]
    fn streaming_con_ragionamento_e_fallback() {
        let (url, seen) = server(
            "data: {\"choices\":[{\"delta\":{\"content\":\"<think>penso\"}}]}\n\n\
             data: {\"choices\":[{\"delta\":{\"content\":\"</think>Ciao \"}}]}\n\n\
             data: {\"choices\":[{\"delta\":{\"content\":\"Marco\"}}]}\n\n\
             data: [DONE]\n\n",
        );
        let dead = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let log = Arc::new(Log(Mutex::new(Vec::new())));
        let r = ai_chat(
            vec![prov("Cloud", "https://esempio.invalid/v1"), prov("Spento", &format!("http://127.0.0.1:{dead}/v1")), prov("Locale", &url)],
            vec![ChatMsg { role: "user".into(), text: "ciao".into(), image: Some(vec![0x89, b'P', b'N', b'G']) }],
            log.clone(),
        )
        .unwrap();
        assert_eq!(r.text, "Ciao Marco");
        assert_eq!(r.provider, "Locale");
        let ev = log.0.lock().unwrap().clone();
        assert!(matches!(&ev[0], AiEvent::Skipped { why, .. } if why == "manca la chiave"), "{ev:?}");
        assert!(matches!(&ev[2], AiEvent::Skipped { provider, why } if provider == "Spento" && why.contains("non raggiungibile")), "{ev:?}");
        assert!(ev.contains(&AiEvent::Thinking { text: "penso".into() }), "{ev:?}");
        let req = seen.lock().unwrap()[0].clone();
        assert!(req.contains("Sei un assistente versatile") && req.contains("«prova-8b» del servizio «Locale»"));
        assert!(req.contains("data:image/png;base64,"));
    }

    #[test]
    fn trascrizione_pulita() {
        let (url, seen) = server("data: {\"choices\":[{\"delta\":{\"content\":\"```markdown\\n## Titolo\\n\\\\(x^2\\\\)\\n```\"}}]}\n\ndata: [DONE]\n\n");
        let log = Arc::new(Log(Mutex::new(Vec::new())));
        let mut testo = prov("Testo", &url);
        testo.vision = false;
        let r = ai_transcribe(vec![testo, prov("Occhi", &url)], vec![1, 2, 3], log).unwrap();
        assert_eq!(r.provider, "Occhi");
        assert_eq!(r.text, "## Titolo\n$x^2$");
        let req = seen.lock().unwrap()[0].clone();
        assert!(req.contains("Sei un trascrittore") && req.contains("\"temperature\":0.0"));
    }

    #[test]
    fn il_tutor_legge_e_scrive_sul_foglio() {
        use crate::board::BoardModel;
        use crate::session::{PenPoint, Phase};
        let (url, seen) = server_seq(vec![
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"a\",\"function\":{\"name\":\"lavagna_leggi\",\"arguments\":\"{}\"}}]}}]}\n\ndata: [DONE]\n\n",
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"b\",\"function\":{\"name\":\"lavagna_annota\",\"arguments\":\"{\\\"segni\\\":[{\\\"tipo\\\":\\\"sottolinea\\\",\\\"su\\\":\\\"r1\\\"},{\\\"tipo\\\":\\\"testo\\\",\\\"su\\\":\\\"r1\\\",\\\"testo\\\":\\\"giusto\\\"}]}\"}}]}}]}\n\ndata: [DONE]\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"Ho sottolineato la riga.\"}}]}\n\ndata: [DONE]\n\n",
        ]);
        // una «parola» scritta a mano sul foglio: un tratto a zig-zag stretto
        let board = BoardModel::new();
        let mut pts = Vec::new();
        for i in 0..40 {
            let x = 3000.0 + i as f32 * 60.0;
            let y = 3000.0 + if i % 2 == 0 { 0.0 } else { 500.0 };
            let phase = if i == 0 { Phase::Down } else if i == 39 { Phase::Up } else { Phase::Move };
            pts.push(PenPoint { x, y, pressure: 0.5, phase });
        }
        board.pen(pts);
        let before = board.strokes().len();
        let log = Arc::new(Log(Mutex::new(Vec::new())));
        let view = crate::tutor::BoardView { quarter: 0, pad_w: 21600.0, pad_h: 14800.0 };
        let r = ai_chat_board(vec![prov("Locale", &url)], vec![ChatMsg { role: "user".into(), text: "correggi".into(), image: None }], board.clone(), view, log.clone()).unwrap();
        assert_eq!(r.text, "Ho sottolineato la riga.");
        assert!(board.strokes().len() > before + 1, "nessun segno aggiunto");
        let ev = log.0.lock().unwrap().clone();
        assert!(ev.contains(&AiEvent::BoardChanged), "{ev:?}");
        assert!(ev.iter().any(|e| matches!(e, AiEvent::ToolDone { result, .. } if result == "segni sul foglio")), "{ev:?}");
        // le richieste dicono al modello degli strumenti e gli rimandano i risultati
        let reqs = seen.lock().unwrap().clone();
        assert!(reqs[0].contains("\"name\":\"lavagna_annota\"") && reqs[0].contains("Accanto alla chat c'è il foglio"));
        assert!(reqs[1].contains("\"tool_call_id\":\"a\"") && reqs[1].contains("r1"));
        // un solo ↶ toglie tutto quello che ha scritto il tutor
        assert!(board.undo());
        assert_eq!(board.strokes().len(), before);
    }

    #[test]
    fn modello_scelto_da_solo() {
        let ids: Vec<String> = ["text-embed", "qwen3-vl-8b", "llama-3.2-1b", "qwen3-32b"].iter().map(|s| s.to_string()).collect();
        assert_eq!(choose_model_why(&ids, false).unwrap(), "qwen3-32b");
        assert_eq!(choose_model_why(&ids, true).unwrap(), "qwen3-vl-8b");
    }
}
