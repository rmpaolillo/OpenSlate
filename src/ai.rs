//! Agenti AI tramite API in formato OpenAI (`/chat/completions`), con una
//! catena di fallback: si prova il primo provider di `ai.toml`; se non
//! risponde (spento, chiave mancante, errore, modello senza visione…) si passa
//! al successivo.
//!
//! Il formato OpenAI è solo il "dialetto" dell'API: lo parlano LM Studio,
//! MTPLX, Ollama, DeepSeek, Google Gemini e Anthropic Claude.
//!
//! Le chiavi non stanno in chiaro nel file: `key = "env:NOME"` (variabile
//! d'ambiente) oppure `key = "keychain:NOME"` (Portachiavi di macOS), anche
//! in alternativa: `"env:NOME | keychain:NOME"`.

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const CONFIG_FILE: &str = "ai.toml";

/// `ai.toml` nella cartella dell'app (o il file indicato da `WACOM_AI_CONFIG`).
pub fn config_path() -> String {
    std::env::var("WACOM_AI_CONFIG").unwrap_or_else(|_| CONFIG_FILE.to_string())
}

/// File creato al primo avvio (commentato, da modificare a mano).
pub const DEFAULT_TOML: &str = r#"# Agenti AI di OpenSlate (OCR della scrittura e chat).
#
# Tutti i provider parlano l'API in formato OpenAI (/chat/completions).
# Vengono provati IN ORDINE: se il primo non risponde (spento, chiave
# mancante, errore) si passa al successivo. Per cambiare priorità basta
# spostare i blocchi [[provider]]; per escluderne uno: enabled = false.
#
# Campi:
#   name          nome mostrato
#   url           indirizzo base dell'API (quello che finisce con /v1 o simile)
#   model         modello per la chat. Vuoto = scelto da /models
#   vision_model  modello per leggere le immagini (OCR, pagina allegata).
#                 Vuoto = usa "model" (o lo sceglie da /models)
#   vision        false = il provider non legge immagini (riceve solo testo)
#   key           chiave API, MAI in chiaro:
#                   "env:NOME"       variabile d'ambiente
#                   "keychain:NOME"  Portachiavi di macOS, creata con
#                     security add-generic-password -a "$USER" -s NOME -w 'la-chiave'
#                   le due insieme:  "env:NOME | keychain:NOME"
#                   "dsh:NOME"       chiave di DeepSeek Harness (~/.dsh/.credentials.yaml)
#                 Se la chiave non si trova il provider viene saltato.
#   timeout_s     secondi massimi di attesa della risposta
#   temperature   facoltativa (es. 0.2). Se assente: 0 per l'OCR, default del modello in chat
#   no_temperature  true = non mandare mai la temperatura (Claude)
#   max_tokens    facoltativo
#   all_vision    true = ogni modello del server legge le immagini
#   tools         false = non offrire strumenti (ricerca web) a questo provider
#   context       contesto massimo in token (0 = da /models se il modello è
#                 scelto da lì, altrimenti 32768): serve all'indicatore e alla
#                 compattazione della chat

# Le chiavi già date a DeepSeek Harness (~/.dsh/.credentials.yaml) si
# riusano con "dsh:NOME", senza ripeterle.

# 1) LM Studio su questo computer (Developer → Start Server). Per l'OCR serve
#    un modello con visione caricato, es. Qwen3-VL-8B. Su un altro computer
#    della rete: il suo indirizzo al posto di localhost ("Serve on local network").
#    I modelli sotto i 3B (es. llama-3.2-1b) non vengono scelti da soli.
[[provider]]
name = "LM Studio"
url = "http://localhost:1234/v1"
model = ""
vision_model = ""
key = ""
timeout_s = 300

# 2) Ollama su questo computer (ollama serve).
[[provider]]
name = "Ollama"
url = "http://localhost:11434/v1"
model = ""
key = ""
timeout_s = 300

# 3) DeepSeek, API ufficiale (la stessa di DSH). Solo testo: con una pagina
#    allegata riceve la trascrizione, non l'immagine.
[[provider]]
name = "DeepSeek"
url = "https://api.deepseek.com/v1"
model = "deepseek-flash"
vision = false
context = 131072
key = "dsh:DEEPSEEK_API_KEY | env:DEEPSEEK_API_KEY | keychain:DEEPSEEK_API_KEY"
timeout_s = 180

# 4) Google Gemini (endpoint compatibile OpenAI).
[[provider]]
name = "Google Gemini"
url = "https://generativelanguage.googleapis.com/v1beta/openai"
model = "gemini-3.8-flash"
key = "env:GEMINI_API_KEY | keychain:GEMINI_API_KEY"
context = 1048576
timeout_s = 180

# 5) Anthropic Claude (endpoint compatibile OpenAI).
[[provider]]
name = "Anthropic Claude"
url = "https://api.anthropic.com/v1"
model = "claude-sonnet-4-6"
key = "env:ANTHROPIC_API_KEY | keychain:ANTHROPIC_API_KEY"
no_temperature = true
max_tokens = 8192
timeout_s = 180
context = 200000

# Istruzioni proprie (facoltative) al posto di quelle interne: vanno scritte
# IN CIMA al file, prima del primo [[provider]]:
#   ocr_prompt = """..."""
#   chat_prompt = """..."""

# Strumenti della chat (casella «Ricerca web»): l'AI può cercare in rete e
# leggere pagine con l'API DeepSeek (0,75 $ ogni 1000 ricerche; la stessa
# chiave di DSH) e verificare le partite IVA (cifra di controllo + VIES UE,
# gratis). Ai provider che non li reggono: tools = false nel loro blocco.
[chat]
compact_at = 80   # compatta da sola la conversazione all'80% del contesto (0 = mai)
keep_recent = 4   # messaggi più recenti tenuti parola per parola

[web]
enabled = true
key = "dsh:DEEPSEEK_API_KEY | env:DEEPSEEK_API_KEY | keychain:DEEPSEEK_API_KEY"
count = 10        # risultati per ricerca (1-50)
lang = ""         # es. "it" per cercare solo pagine in italiano
"#;

/// Intestazione del file quando lo scrive la pagina «Impostazioni AI».
const SAVED_HEADER: &str = "# Agenti AI di OpenSlate: si modificano comodamente dalla pagina
# «Impostazioni AI» (pulsante nella chat), oppure qui a mano.
# Gli agenti si provano in ordine; il primo che risponde vince.
# key: \"dsh:NOME\" (chiave di DeepSeek Harness), \"keychain:NOME\" (Portachiavi),
#      \"env:NOME\" (variabile d'ambiente), anche in alternativa con |
# context: contesto massimo in token (0 = automatico)
";

/// Istruzioni per la trascrizione: fedele, in Markdown (formule in LaTeX se ci sono).
pub const OCR_PROMPT: &str = r#"Sei un trascrittore di scrittura a mano. La pagina può contenere qualsiasi cosa: appunti, liste, promemoria, bozze di testi o lettere, verbali di riunione, schemi, tabelle, conti, esercizi o formule. Trascrivi ESATTAMENTE ciò che è scritto nell'immagine, in Markdown.

Regole:
1. Fedeltà assoluta. NON correggere ortografia, grammatica, numeri, calcoli o contenuti sbagliati. NON completare, NON riassumere, NON aggiungere spiegazioni. Se è scritto 2+2=5, trascrivi 2+2=5.
2. Mantieni la lingua in cui è scritta la pagina, l'ordine e gli a capo del foglio.
3. Titoli come titoli Markdown (##) solo se sul foglio si vedono come titoli (sottolineati, più grandi, isolati). Elenchi come elenchi Markdown; caselle da spuntare come "- [ ]" o "- [x]". Tabelle come tabelle Markdown.
4. Solo se ci sono formule o simboli matematici: in linea tra $...$, su una riga propria tra $$...$$, in LaTeX standard (\frac, \sqrt, \sum, \int, \begin{cases}, \begin{pmatrix}…).
5. Parole barrate: ~~...~~ (o \cancel{...} dentro una formula).
6. Parole o simboli illeggibili: [?]. Se sei incerto fra due letture scrivi la più probabile seguita da [?].
7. Disegni, frecce fra blocchi, schemi o grafici: [figura: breve descrizione].
8. Rispondi SOLO con il Markdown trascritto: niente ``` attorno, niente commenti, niente introduzioni."#;

/// Istruzioni per la chat.
pub const CHAT_PROMPT: &str = r#"Sei un assistente versatile dentro un'app per scrivere a mano su un pad digitale: aiuti con quello che serve all'utente, per esempio appunti, documenti da leggere o firmare, testi da scrivere o rivedere, idee, studio, lavoro, domande di ogni genere. Rispondi nella lingua in cui ti scrive l'utente, in Markdown. Se servono formule scrivile in LaTeX con $...$ in linea e $$...$$ su riga propria (mai \( \) o \[ \]).
Quando ricevi una pagina scritta a mano (immagine e/o trascrizione), la trascrizione è fedele al foglio, errori compresi; se l'immagine e la trascrizione non coincidono, fidati dell'immagine e dillo. Fai quello che ti viene chiesto: correggi, giudichi o completi solo se te lo chiedono. Sii preciso e conciso."#;

// ---------------------------------------------------------------------------
// Configurazione
// ---------------------------------------------------------------------------

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Provider {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub vision_model: String,
    #[serde(default = "yes")]
    pub vision: bool,
    #[serde(default)]
    pub key: String,
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default = "default_timeout")]
    pub timeout_s: u64,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub no_temperature: bool,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    /// Tutti i modelli di questo server leggono le immagini (es. MTPLX con
    /// Qwen3.8): la scelta automatica non guarda il nome.
    #[serde(default)]
    pub all_vision: bool,
    /// false = a questo provider non si offrono strumenti (ricerca web…).
    #[serde(default = "yes")]
    pub tools: bool,
    /// Contesto massimo del modello in token (0 = lo si chiede a /models,
    /// altrimenti 32768).
    #[serde(default)]
    pub context: u32,
}

impl Provider {
    /// Server che mandano il conteggio dei token anche in streaming se
    /// glielo si chiede (`stream_options`); agli altri non si chiede, per
    /// non rischiare un rifiuto: i token si stimano.
    fn stream_usage(&self) -> bool {
        ["api.deepseek.com", "googleapis.com", "api.openai.com", "openrouter.ai"].iter().any(|h| self.url.contains(h))
    }

    pub fn new(name: &str, url: &str) -> Self {
        Self {
            name: name.into(),
            url: url.into(),
            model: String::new(),
            vision_model: String::new(),
            vision: true,
            key: String::new(),
            enabled: true,
            timeout_s: default_timeout(),
            temperature: None,
            no_temperature: false,
            max_tokens: None,
            all_vision: false,
            tools: true,
            context: 0,
        }
    }
}

/// Opzioni della chat.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ChatOpts {
    /// Compattazione automatica quando la conversazione arriva a questa
    /// percentuale del contesto (0 = mai).
    #[serde(default = "default_compact_at")]
    pub compact_at: u8,
    /// Messaggi più recenti tenuti parola per parola quando si compatta.
    #[serde(default = "default_keep_recent")]
    pub keep_recent: usize,
}

fn default_compact_at() -> u8 {
    80
}
fn default_keep_recent() -> usize {
    4
}

impl Default for ChatOpts {
    fn default() -> Self {
        Self { compact_at: default_compact_at(), keep_recent: default_keep_recent() }
    }
}

fn yes() -> bool {
    true
}
fn default_timeout() -> u64 {
    180
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct AiConfig {
    #[serde(default, rename = "provider")]
    pub providers: Vec<Provider>,
    #[serde(default)]
    pub ocr_prompt: Option<String>,
    #[serde(default)]
    pub chat_prompt: Option<String>,
    /// Strumenti della chat (ricerca web, verifica P.IVA).
    #[serde(default)]
    pub web: crate::tools::WebTools,
    #[serde(default)]
    pub chat: ChatOpts,
}

impl AiConfig {
    /// Legge `ai.toml` (lo crea al primo uso). Riletto a ogni richiesta: le
    /// modifiche valgono subito, senza riavviare.
    pub fn load() -> Result<Self> {
        let path = config_path();
        if !std::path::Path::new(&path).exists() {
            std::fs::write(&path, DEFAULT_TOML).with_context(|| format!("creazione di {path}"))?;
        }
        let text = std::fs::read_to_string(&path).with_context(|| format!("lettura di {path}"))?;
        Self::parse(&text)
    }

    /// Riscrive `ai.toml` (dalla pagina «Impostazioni AI»), tenendo una
    /// copia della versione precedente in `ai.toml.bak`.
    pub fn save(&self) -> Result<()> {
        if self.providers.is_empty() {
            bail!("serve almeno un agente");
        }
        for p in &self.providers {
            if p.name.trim().is_empty() || p.url.trim().is_empty() {
                bail!("ogni agente deve avere nome e indirizzo");
            }
            if !(p.url.starts_with("http://") || p.url.starts_with("https://")) {
                bail!("«{}»: l'indirizzo deve iniziare con http:// o https://", p.name);
            }
        }
        let body = toml::to_string_pretty(self).map_err(|e| anyhow!("{e}"))?;
        let text = format!("{SAVED_HEADER}\n{body}");
        Self::parse(&text)?; // deve rileggersi uguale
        let path = config_path();
        if std::path::Path::new(&path).exists() {
            let _ = std::fs::copy(&path, format!("{path}.bak"));
        }
        std::fs::write(&path, text).with_context(|| format!("scrittura di {path}"))?;
        Ok(())
    }

    pub fn parse(text: &str) -> Result<Self> {
        let cfg: AiConfig = toml::from_str(text).map_err(|e| anyhow!("{CONFIG_FILE}: {e}"))?;
        if cfg.providers.is_empty() {
            bail!("{CONFIG_FILE}: nessun [[provider]]");
        }
        Ok(cfg)
    }

    pub fn ocr_prompt(&self) -> &str {
        self.ocr_prompt.as_deref().filter(|s| !s.trim().is_empty()).unwrap_or(OCR_PROMPT)
    }

    pub fn chat_prompt(&self) -> &str {
        self.chat_prompt.as_deref().filter(|s| !s.trim().is_empty()).unwrap_or(CHAT_PROMPT)
    }
}

/// Risolve la chiave: `None` = richiesta ma non trovata (provider saltato),
/// `Some("")` = nessuna chiave necessaria.
pub fn resolve_key(spec: &str) -> Option<String> {
    let spec = spec.trim();
    if spec.is_empty() {
        return Some(String::new());
    }
    for alt in spec.split('|').map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(name) = alt.strip_prefix("env:") {
            if let Ok(v) = std::env::var(name.trim()) {
                if !v.trim().is_empty() {
                    return Some(v.trim().to_string());
                }
            }
        } else if let Some(name) = alt.strip_prefix("keychain:") {
            if let Some(v) = keychain(name.trim()) {
                return Some(v);
            }
        } else if let Some(name) = alt.strip_prefix("dsh:") {
            if let Some(v) = dsh_key(name.trim()) {
                return Some(v);
            }
        } else {
            // chiave scritta in chiaro (sconsigliato, ma accettato)
            return Some(alt.to_string());
        }
    }
    None
}

/// Chiave salvata da DeepSeek Harness (`~/.dsh/.credentials.yaml`, sezione
/// `refs`): così le chiavi già date a DSH non vanno ripetute.
fn dsh_key(name: &str) -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let path = std::env::var("WACOM_DSH_CREDENTIALS").unwrap_or_else(|_| format!("{home}/.dsh/.credentials.yaml"));
    dsh_key_from(&std::fs::read_to_string(path).ok()?, name)
}

/// Legge `refs: { NOME: valore }` da un YAML semplice (senza librerie).
fn dsh_key_from(text: &str, name: &str) -> Option<String> {
    let mut in_refs = false;
    for line in text.lines() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let indented = line.starts_with(' ') || line.starts_with('\t');
        if !indented {
            in_refs = line.trim_end() == "refs:";
            continue;
        }
        if !in_refs {
            continue;
        }
        let Some((k, v)) = line.trim().split_once(':') else { continue };
        if k.trim().trim_matches(|c| c == '"' || c == '\'') == name {
            let v = v.trim().trim_matches(|c| c == '"' || c == '\'').trim();
            return (!v.is_empty()).then(|| v.to_string());
        }
    }
    None
}

fn keychain(service: &str) -> Option<String> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    let out = std::process::Command::new("security")
        .args(["find-generic-password", "-s", service, "-w"])
        .output()
        .ok()?;
    let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && !v.is_empty()).then_some(v)
}

// ---------------------------------------------------------------------------
// Messaggi
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Msg {
    pub role: String,
    pub text: String,
    /// Immagine PNG allegata (solo ai provider con visione).
    pub image: Option<Arc<Vec<u8>>>,
}

impl Msg {
    pub fn new(role: &str, text: impl Into<String>) -> Self {
        Self { role: role.into(), text: text.into(), image: None }
    }
    pub fn with_image(mut self, png: Arc<Vec<u8>>) -> Self {
        self.image = Some(png);
        self
    }
}

/// Cosa sta succedendo (per mostrarlo in chat).
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Si prova questo provider con questo modello.
    Trying { provider: String, model: String, context: u32 },
    /// Provider saltato o fallito: si passa al prossimo.
    Skipped { provider: String, why: String },
    /// Pezzo di risposta (streaming).
    Delta(String),
    /// Pezzo del ragionamento del modello (`reasoning_content` o `<think>`),
    /// mostrato a parte e richiudibile.
    Thinking(String),
    /// Il modello usa uno strumento (es. «Cerco in rete: …»).
    Tool { label: String },
    /// Strumento finito, con l'esito breve.
    ToolDone { label: String, result: String },
    /// Token usati dalla conversazione (esatti se il server li dice,
    /// altrimenti stimati), token della risposta e contesto massimo del modello.
    Usage { used: u32, max: u32, exact: bool, output: u32 },
}

#[derive(Clone, Debug)]
pub struct Reply {
    pub text: String,
    pub provider: String,
    pub model: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Task {
    /// Trascrizione: serve la visione, temperatura 0.
    Ocr,
    /// Chat: le immagini vanno solo a chi le legge.
    Chat,
}

/// Risposta completa (senza streaming verso l'esterno).
pub fn complete(cfg: &AiConfig, task: Task, msgs: &[Msg]) -> Result<Reply> {
    run(cfg, task, msgs, &mut |_| true)
}

/// Prova i provider in ordine. `on_event` riceve tentativi e pezzi di
/// risposta; se ritorna `false` la richiesta si interrompe.
pub fn run(cfg: &AiConfig, task: Task, msgs: &[Msg], on_event: &mut dyn FnMut(Event) -> bool) -> Result<Reply> {
    run_with(cfg, task, msgs, false, on_event)
}

/// Come `run`; con `use_tools` la chat può cercare in rete e verificare
/// partite IVA (sezione `[web]` di ai.toml).
pub fn run_with(
    cfg: &AiConfig,
    task: Task,
    msgs: &[Msg],
    use_tools: bool,
    on_event: &mut dyn FnMut(Event) -> bool,
) -> Result<Reply> {
    run_chat(cfg, task, msgs, use_tools, None, on_event)
}

/// Come `run_with`, e con un appunto aperto il modello lo può leggere e
/// scriverci (strumenti `appunto_leggi` / `appunto_scrivi`).
pub fn run_chat(
    cfg: &AiConfig,
    task: Task,
    msgs: &[Msg],
    use_tools: bool,
    appunto: Option<std::path::PathBuf>,
    on_event: &mut dyn FnMut(Event) -> bool,
) -> Result<Reply> {
    let use_web = use_tools && cfg.web.enabled;
    let tool_ctx = (task == Task::Chat && (use_web || appunto.is_some())).then(|| ToolCtx::with(&cfg.web, use_web, appunto));
    let has_image = msgs.iter().any(|m| m.image.is_some());
    let mut fails: Vec<String> = Vec::new();
    for p in cfg.providers.iter().filter(|p| p.enabled) {
        let mut skip = |why: String, on_event: &mut dyn FnMut(Event) -> bool| {
            fails.push(format!("{}: {why}", p.name));
            on_event(Event::Skipped { provider: p.name.clone(), why })
        };
        if task == Task::Ocr && !p.vision {
            if !skip("non legge immagini".into(), on_event) {
                bail!("interrotto");
            }
            continue;
        }
        let Some(key) = resolve_key(&p.key) else {
            if !skip(format!("chiave non trovata ({})", p.key.trim()), on_event) {
                bail!("interrotto");
            }
            continue;
        };
        let mut send_image = has_image && p.vision;
        let mut picked = pick_model(p, &key, send_image);
        if picked.is_err() && send_image && task == Task::Chat {
            // in chat senza modello visivo basta la trascrizione
            if let Ok(m) = pick_model(p, &key, false) {
                send_image = false;
                picked = Ok(m);
            }
        }
        let model = match picked {
            Ok(m) => m,
            Err(e) => {
                if !skip(short_err(&e), on_event) {
                    bail!("interrotto");
                }
                continue;
            }
        };
        let context = context_for(p, &key, &model);
        if !on_event(Event::Trying { provider: p.name.clone(), model: model.clone(), context }) {
            bail!("interrotto");
        }
        let mut started = false;
        let tools = tool_ctx.as_ref().filter(|_| p.tools);
        let res = call(p, &key, &model, task, msgs, send_image, tools, &mut |e| {
            if matches!(e, Event::Delta(_)) {
                started = true;
            }
            on_event(e)
        });
        match res {
            Ok(text) if !text.trim().is_empty() => {
                return Ok(Reply { text, provider: p.name.clone(), model });
            }
            Ok(_) => {
                if !skip("risposta vuota".into(), on_event) {
                    bail!("interrotto");
                }
            }
            // A risposta già iniziata non si può ripartire da un altro provider.
            Err(e) if started => bail!("{} si è interrotto: {}", p.name, short_err(&e)),
            Err(e) => {
                if e.to_string() == "interrotto" || !skip(short_err(&e), on_event) {
                    bail!("interrotto");
                }
            }
        }
    }
    if fails.is_empty() {
        bail!("nessun provider attivo in {CONFIG_FILE}");
    }
    bail!("nessun agente AI disponibile:\n• {}", fails.join("\n• "))
}

fn short_err(e: &anyhow::Error) -> String {
    let s = format!("{e:#}");
    let s = s.trim();
    if s.chars().count() > 300 {
        format!("{}…", s.chars().take(300).collect::<String>())
    } else {
        s.to_string()
    }
}

fn agent(p: &Provider) -> ureq::Agent {
    ureq::AgentBuilder::new()
        // un server spento deve far passare subito al prossimo
        .timeout_connect(Duration::from_secs(4))
        .timeout_read(Duration::from_secs(p.timeout_s.max(5)))
        .build()
}

fn base(p: &Provider) -> &str {
    p.url.trim().trim_end_matches('/')
}

fn with_auth(req: ureq::Request, key: &str) -> ureq::Request {
    if key.is_empty() {
        req
    } else {
        req.set("Authorization", &format!("Bearer {key}"))
    }
}

/// Modello da usare: quello configurato, altrimenti scelto da `/models`
/// (per le immagini preferisce i modelli con visione).
pub(crate) fn pick_model(p: &Provider, key: &str, vision: bool) -> Result<String> {
    let configured = if vision && !p.vision_model.trim().is_empty() { &p.vision_model } else { &p.model };
    if !configured.trim().is_empty() {
        return Ok(configured.trim().to_string());
    }
    let models = list_models(p, key)?;
    {
        let mut c = CONTEXTS.lock().unwrap();
        for (id, ctx) in &models {
            if let Some(n) = ctx {
                c.retain(|(u, m, _)| !(u == &p.url && m == id));
                c.push((p.url.clone(), id.clone(), *n));
            }
        }
    }
    let ids: Vec<String> = models.into_iter().map(|(id, _)| id).collect();
    choose_model_why(&ids, vision && !p.all_vision).map_err(|e| anyhow!("{e}"))
}

/// Contesto se non si sa altro.
pub const DEFAULT_CONTEXT: u32 = 32768;

static CONTEXTS: Mutex<Vec<(String, String, u32)>> = Mutex::new(Vec::new());

fn context_cached(url: &str, model: &str) -> Option<u32> {
    CONTEXTS.lock().unwrap().iter().find(|(u, m, _)| u == url && m == model).map(|x| x.2)
}

/// Contesto massimo del modello: quello scritto in ai.toml, altrimenti quello
/// dichiarato da `/models` quando il modello è scelto da lì (LM Studio, MTPLX,
/// vLLM, OpenRouter…), altrimenti 32768.
fn context_for(p: &Provider, key: &str, model: &str) -> u32 {
    if p.context > 0 {
        CONTEXTS.lock().unwrap().retain(|(u, m, _)| !(u == &p.url && m == model));
        CONTEXTS.lock().unwrap().push((p.url.clone(), model.to_string(), p.context));
        return p.context;
    }
    if let Some(c) = context_cached(&p.url, model) {
        return c;
    }
    // (con il modello scelto da /models il contesto è già nella cache;
    // per un modello scritto a mano si usa context di ai.toml)
    let _ = key;
    DEFAULT_CONTEXT
}

/// Contesto dichiarato in una voce di `/models`, se c'è.
pub fn context_of_model_entry(m: &serde_json::Value) -> Option<u32> {
    for k in ["context_length", "context_window", "max_context_length", "loaded_context_length", "max_model_len", "max_input_tokens", "inputTokenLimit"] {
        if let Some(n) = m[k].as_u64().filter(|&n| n > 0) {
            return Some(n.min(u32::MAX as u64) as u32);
        }
    }
    m["top_provider"]["context_length"].as_u64().map(|n| n as u32)
}

/// Modelli di un server, con il contesto se lo dichiara.
pub fn list_models(p: &Provider, key: &str) -> Result<Vec<(String, Option<u32>)>> {
    let url = format!("{}/models", base(p));
    let r = http(p, key, &url, None).map_err(|e| anyhow!("non raggiungibile ({e})"))?;
    if r.status >= 400 {
        bail!("{url}: HTTP {}", r.status);
    }
    let v: serde_json::Value = serde_json::from_reader(r.body).map_err(|e| anyhow!("{url}: risposta non valida ({e})"))?;
    Ok(v["data"]
        .as_array()
        .or(v["models"].as_array())
        .map(|a| {
            a.iter()
                .filter_map(|m| Some((m["id"].as_str().or(m["name"].as_str())?.to_string(), context_of_model_entry(m))))
                .collect()
        })
        .unwrap_or_default())
}

/// Stima grossolana dei token di un testo (italiano e LaTeX: ~3,2 caratteri
/// per token).
pub fn estimate_text(s: &str) -> u32 {
    (s.chars().count() as f32 / 3.2).ceil() as u32
}

/// Token stimati di un'immagine allegata.
pub const IMAGE_TOKENS: u32 = 1200;

fn estimate_wire(wire: &[serde_json::Value]) -> u32 {
    wire.iter()
        .map(|m| match &m["content"] {
            serde_json::Value::String(t) => estimate_text(t) + 4,
            serde_json::Value::Array(parts) => parts
                .iter()
                .map(|p| if p["type"] == "image_url" { IMAGE_TOKENS } else { estimate_text(p["text"].as_str().unwrap_or("")) })
                .sum::<u32>(),
            _ => estimate_text(&m.to_string()),
        })
        .sum()
}

/// Istruzioni per riassumere la conversazione (compattazione).
const COMPACT_PROMPT: &str = "Riassumi la conversazione che segue in modo che possa continuare senza perdere nulla di importante. \
Scrivi nella lingua della conversazione, in Markdown (formule in LaTeX con $...$ e $$...$$, se ce ne sono). Conserva: l'argomento \
e i materiali di cui si parla (pagine, documenti, appunti: il contenuto essenziale), i risultati raggiunti, le decisioni prese, \
gli errori trovati e le correzioni, le fonti citate (con link), le domande ancora aperte. Togli saluti, ripetizioni e passaggi superati. Non aggiungere nulla che non sia nella conversazione. \
Inizia direttamente con il riassunto.";

/// Compatta una conversazione: ritorna il riassunto e chi l'ha scritto.
pub fn compact(cfg: &AiConfig, history: &[(String, String)]) -> Result<Reply> {
    let mut transcript = String::new();
    for (role, text) in history {
        let who = match role.as_str() {
            "assistant" => "AI",
            "summary" => "Riassunto precedente",
            _ => "Utente",
        };
        transcript.push_str(&format!("### {who}\n{}\n\n", text.trim()));
    }
    let msgs = vec![Msg::new("system", COMPACT_PROMPT), Msg::new("user", transcript)];
    let mut r = run(cfg, Task::Chat, &msgs, &mut |_| true)?;
    r.text = clean_reply(&r.text);
    Ok(r)
}

/// Sceglie fra gli id di `/models`.
pub fn choose_model(ids: &[String], vision: bool) -> Option<String> {
    choose_model_why(ids, vision).ok()
}

/// Dimensione dichiarata nel nome, in miliardi di parametri
/// («qwen3-vl-8b-instruct» → 8, «qwen3-30b-a3b» → 30). Sconosciuta = None.
pub fn model_size(id: &str) -> Option<f32> {
    id.to_lowercase()
        .split(|c: char| matches!(c, '-' | '_' | ':' | '/' | ' ' | '@'))
        .filter_map(|t| t.strip_suffix('b')?.parse::<f32>().ok())
        .reduce(f32::max)
}

/// Sotto questa soglia (miliardi) un modello scelto da solo non si usa:
/// risponde male e legge male la scrittura a mano.
const MIN_AUTO_SIZE: f32 = 3.0;

/// Come `choose_model`, ma spiega perché non ne ha trovato uno.
/// Fra i candidati preferisce il più grande (dimensione letta dal nome).
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
    // per le immagini solo i modelli con visione; in chat va bene qualunque
    let cands: Vec<&String> = usable.iter().copied().filter(|id| !vision || is_vision(id)).collect();
    if cands.is_empty() {
        return Err(format!(
            "nessun modello con visione caricato (ci sono: {}). Carica ad es. Qwen3-VL-8B",
            usable.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
        ));
    }
    let size = |id: &str| model_size(id).unwrap_or(7.0);
    let best = cands
        .iter()
        .copied()
        .max_by(|a, b| size(a).total_cmp(&size(b)))
        .unwrap();
    if size(best) < MIN_AUTO_SIZE {
        return Err(format!(
            "solo modelli troppo piccoli per rispondere bene ({best}): caricane uno più grande o scrivilo in «model» di ai.toml"
        ));
    }
    Ok(best.to_string())
}

fn transport_msg(e: &ureq::Error) -> String {
    match e {
        ureq::Error::Transport(t) => {
            let s = t.to_string();
            if s.contains("Connection refused") {
                "connessione rifiutata: server spento?".into()
            } else if s.contains("os error 65") || s.contains("No route to host") || s.contains("os error 113") {
                // macOS dà questo errore anche quando manca il permesso «Rete locale»
                "computer non raggiungibile (No route to host): spento o in stop, indirizzo IP cambiato, \
                 oppure macOS non dà il permesso «Rete locale» all'app da cui parte OpenSlate \
                 (Impostazioni di Sistema → Privacy e sicurezza → Rete locale)"
                    .into()
            } else if s.to_lowercase().contains("timed out") {
                "nessuna risposta (tempo scaduto)".into()
            } else {
                s
            }
        }
        other => other.to_string(),
    }
}

fn message_json(m: &Msg, send_image: bool) -> serde_json::Value {
    use base64::Engine;
    match (&m.image, send_image) {
        (Some(png), true) => {
            let b64 = base64::engine::general_purpose::STANDARD.encode(png.as_slice());
            let mut parts = Vec::new();
            if !m.text.is_empty() {
                parts.push(serde_json::json!({ "type": "text", "text": m.text }));
            }
            parts.push(serde_json::json!({
                "type": "image_url",
                "image_url": { "url": format!("data:image/png;base64,{b64}") }
            }));
            serde_json::json!({ "role": m.role, "content": parts })
        }
        _ => serde_json::json!({ "role": m.role, "content": m.text }),
    }
}

/// Strumenti offerti al modello in questa richiesta.
pub struct ToolCtx {
    pub web: crate::tools::WebTools,
    /// Chiave per ricerca e lettura pagine (None = solo verifica P.IVA).
    pub key: Option<String>,
    /// Appunto aperto nell'app: il modello lo può leggere e scriverci.
    pub appunto: Option<std::path::PathBuf>,
    defs: Vec<serde_json::Value>,
    note: String,
}

impl ToolCtx {
    pub fn new(web: &crate::tools::WebTools) -> Self {
        Self::with(web, true, None)
    }

    /// Strumenti per la chat: quelli del web (se `use_web`) e quelli
    /// dell'appunto aperto (se c'è).
    pub fn with(web: &crate::tools::WebTools, use_web: bool, appunto: Option<std::path::PathBuf>) -> Self {
        let key = if use_web { resolve_key(&web.key).filter(|k| !k.is_empty()) } else { None };
        let mut defs = if use_web { crate::tools::definitions(key.is_some()) } else { Vec::new() };
        let mut note = if use_web { crate::tools::system_note(key.is_some()) } else { String::new() };
        if let Some(p) = &appunto {
            defs.extend(crate::tools::note_definitions());
            note.push_str(&crate::tools::note_system_note(p));
        }
        Self { web: web.clone(), defs, note, key, appunto }
    }
}

/// Giri massimi domanda → strumenti → risposta.
const MAX_ROUNDS: usize = 6;

/// Una richiesta a un provider (in streaming), con gli strumenti se offerti:
/// il modello può chiedere ricerche, le si esegue e si continua finché
/// risponde. Ritorna il testo pulito.
#[allow(clippy::too_many_arguments)]
fn call(
    p: &Provider,
    key: &str,
    model: &str,
    task: Task,
    msgs: &[Msg],
    send_image: bool,
    tools: Option<&ToolCtx>,
    on_event: &mut dyn FnMut(Event) -> bool,
) -> Result<String> {
    let context = context_cached(&p.url, model).unwrap_or(DEFAULT_CONTEXT);
    let url = format!("{}/chat/completions", base(p));
    let mut wire: Vec<serde_json::Value> = msgs.iter().map(|m| message_json(m, send_image)).collect();
    if let Some(t) = tools {
        match wire.first_mut() {
            Some(first) if first["role"] == "system" => {
                let s = format!("{}{}", first["content"].as_str().unwrap_or(""), t.note);
                first["content"] = serde_json::json!(s);
            }
            _ => wire.insert(0, serde_json::json!({ "role": "system", "content": t.note.trim() })),
        }
    }
    let mut tools = tools;
    let mut answer = String::new();
    let mut round = 0;
    while round < MAX_ROUNDS {
        let mut body = serde_json::json!({ "model": model, "stream": true, "messages": wire });
        if p.stream_usage() {
            body["stream_options"] = serde_json::json!({ "include_usage": true });
        }
        let temp = p.temperature.or(if task == Task::Ocr { Some(0.0) } else { None });
        if let (Some(t), false) = (temp, p.no_temperature) {
            body["temperature"] = serde_json::json!(t);
        }
        if let Some(n) = p.max_tokens {
            body["max_tokens"] = serde_json::json!(n);
        }
        if let Some(t) = tools {
            body["tools"] = serde_json::json!(t.defs);
            // all'ultimo giro deve rispondere, non cercare ancora
            body["tool_choice"] = serde_json::json!(if round + 1 < MAX_ROUNDS { "auto" } else { "none" });
        }
        let sep = !answer.is_empty();
        let mut first = true;
        let out = stream_round(p, key, &url, &body, &mut |think, d| {
            if think {
                return on_event(Event::Thinking(d));
            }
            let d = if first && sep { format!("\n\n{d}") } else { d };
            first = false;
            on_event(Event::Delta(d))
        });
        let out = match out {
            Ok(o) => o,
            // server che non accetta gli strumenti: si riprova senza
            Err(e) if tools.is_some() && round == 0 && answer.is_empty() && e.to_string().to_lowercase().contains("tool") => {
                tracing::info!("{}: strumenti non accettati ({e}), riprovo senza", p.name);
                tools = None;
                continue;
            }
            Err(e) => return Err(e),
        };
        if !out.text.is_empty() {
            if sep {
                answer.push_str("\n\n");
            }
            answer.push_str(&out.text);
        }
        let Some(t) = tools.filter(|_| !out.calls.is_empty()) else {
            // token della conversazione: quelli del server se li dice
            let (used, exact) = match out.usage {
                Some(u) => (u, true),
                None => (estimate_wire(&wire) + estimate_text(&out.raw), false),
            };
            let output = out.output.unwrap_or_else(|| estimate_text(&answer) + estimate_text(&out.reasoning));
            on_event(Event::Usage { used, max: context, exact, output });
            return Ok(clean_reply(&answer));
        };
        // Il modello ha chiesto degli strumenti: si eseguono e si continua.
        let calls: Vec<serde_json::Value> = out
            .calls
            .iter()
            .map(|c| serde_json::json!({ "id": c.id, "type": "function", "function": { "name": c.name, "arguments": c.args } }))
            .collect();
        let mut asst = serde_json::json!({ "role": "assistant", "content": out.raw, "tool_calls": calls });
        if !out.reasoning.is_empty() {
            asst["reasoning_content"] = serde_json::json!(out.reasoning);
        }
        wire.push(asst);
        for c in &out.calls {
            let args: serde_json::Value = serde_json::from_str(&c.args).unwrap_or(serde_json::json!({}));
            let label = crate::tools::label(&c.name, &args);
            if !on_event(Event::Tool { label: label.clone() }) {
                bail!("interrotto");
            }
            let (result, short) = match t.appunto.as_deref().and_then(|p| crate::tools::note_execute(p, &c.name, &args)) {
                Some(r) => r,
                None => crate::tools::execute(&t.web, t.key.as_deref(), &c.name, &args),
            };
            if !on_event(Event::ToolDone { label, result: short }) {
                bail!("interrotto");
            }
            wire.push(serde_json::json!({ "role": "tool", "tool_call_id": c.id, "content": result }));
        }
        round += 1;
    }
    Ok(clean_reply(&answer))
}

struct ToolCall {
    id: String,
    name: String,
    args: String,
}

struct RoundOut {
    /// Token totali (domanda + risposta) se il server li dice.
    usage: Option<u32>,
    /// Token della risposta (con il ragionamento) se il server li dice.
    output: Option<u32>,
    /// Testo visibile (senza ragionamento).
    text: String,
    /// Testo così come arrivato (per rimandarlo al modello).
    raw: String,
    reasoning: String,
    calls: Vec<ToolCall>,
}

/// Un giro: invia la richiesta e legge la risposta (testo e strumenti chiesti).
fn stream_round(
    p: &Provider,
    key: &str,
    url: &str,
    body: &serde_json::Value,
    on_delta: &mut dyn FnMut(bool, String) -> bool,
) -> Result<RoundOut> {
    let mut resp = http(p, key, url, Some(body))?;
    if resp.status >= 400 {
        let code = resp.status;
        let mut txt = String::new();
        let _ = std::io::Read::read_to_string(&mut resp.body, &mut txt);
        let msg = serde_json::from_str::<serde_json::Value>(&txt)
            .ok()
            .and_then(|v| {
                v["error"]["message"]
                    .as_str()
                    .or(v[0]["error"]["message"].as_str())
                    .or(v["message"].as_str())
                    .map(String::from)
            })
            .unwrap_or(txt);
        bail!("HTTP {code}: {}", msg.trim());
    }

    let mut filt = ThinkFilter::default();
    let mut reasoning = String::new();
    let mut calls: Vec<ToolCall> = Vec::new();
    let mut usage: Option<u32> = None;
    let mut output: Option<u32> = None;
    let read_output = |v: &serde_json::Value| v["usage"]["completion_tokens"].as_u64().map(|x| x as u32).filter(|&x| x > 0);
    let read_usage = |v: &serde_json::Value| -> Option<u32> {
        let u = &v["usage"];
        u["total_tokens"]
            .as_u64()
            .or_else(|| Some(u["prompt_tokens"].as_u64()? + u["completion_tokens"].as_u64().unwrap_or(0)))
            .map(|x| x as u32)
            .filter(|&x| x > 0)
    };
    let add_call = |i: usize, c: &serde_json::Value, calls: &mut Vec<ToolCall>| {
        while calls.len() <= i {
            calls.push(ToolCall { id: String::new(), name: String::new(), args: String::new() });
        }
        let t = &mut calls[i];
        if let Some(id) = c["id"].as_str() {
            t.id = id.to_string();
        }
        if let Some(n) = c["function"]["name"].as_str() {
            t.name.push_str(n);
        }
        match &c["function"]["arguments"] {
            serde_json::Value::String(a) => t.args.push_str(a),
            serde_json::Value::Null => {}
            other => t.args.push_str(&other.to_string()),
        }
    };
    // Alcuni server ignorano "stream": risposta JSON unica.
    if resp.ctype.contains("json") {
        let v: serde_json::Value = serde_json::from_reader(resp.body)?;
        usage = read_usage(&v);
        output = read_output(&v);
        let m = &v["choices"][0]["message"];
        if let Some(r) = m["reasoning_content"].as_str().filter(|r| !r.is_empty()) {
            if !on_delta(true, r.to_string()) {
                bail!("interrotto");
            }
        }
        if let Some(text) = m["content"].as_str() {
            let (d, t) = filt.push2(text);
            if let Some(t) = t {
                if !on_delta(true, t) {
                    bail!("interrotto");
                }
            }
            if let Some(d) = d {
                if !on_delta(false, d) {
                    bail!("interrotto");
                }
            }
        } else if m["tool_calls"].is_null() {
            bail!("risposta inattesa: {v}");
        }
        if let Some(r) = m["reasoning_content"].as_str() {
            reasoning.push_str(r);
        }
        for (i, c) in m["tool_calls"].as_array().into_iter().flatten().enumerate() {
            add_call(i, c, &mut calls);
        }
    } else {
        for line in resp.body.lines() {
            let line = line.map_err(|e| anyhow!("lettura: {e}"))?;
            let Some(data) = line.strip_prefix("data:") else { continue };
            let data = data.trim();
            if data == "[DONE]" {
                break;
            }
            let Ok(v) = serde_json::from_str::<serde_json::Value>(data) else { continue };
            if let Some(msg) = v["error"]["message"].as_str() {
                bail!("{msg}");
            }
            if let Some(u) = read_usage(&v) {
                usage = Some(u);
            }
            if let Some(o) = read_output(&v) {
                output = Some(o);
            }
            let delta = &v["choices"][0]["delta"];
            if let Some(r) = delta["reasoning_content"].as_str().filter(|r| !r.is_empty()) {
                reasoning.push_str(r);
                if !on_delta(true, r.to_string()) {
                    bail!("interrotto");
                }
            }
            if let Some(piece) = delta["content"].as_str() {
                let (d, t) = filt.push2(piece);
                if let Some(t) = t {
                    if !on_delta(true, t) {
                        bail!("interrotto");
                    }
                }
                if let Some(d) = d {
                    if !on_delta(false, d) {
                        bail!("interrotto");
                    }
                }
            }
            for (j, c) in delta["tool_calls"].as_array().into_iter().flatten().enumerate() {
                let i = c["index"].as_u64().map(|x| x as usize).unwrap_or(j);
                add_call(i, c, &mut calls);
            }
        }
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
    Ok(RoundOut { usage, output, text: clean_reply(filt.visible()), raw: filt.full.clone(), reasoning, calls })
}

// ---------------------------------------------------------------------------
// HTTP: ureq, oppure curl di sistema
// ---------------------------------------------------------------------------

/// Risposta HTTP con il corpo da leggere (anche a righe, per lo streaming).
struct HttpResp {
    status: u16,
    ctype: String,
    body: Box<dyn BufRead + Send>,
    /// Con curl: il processo e il file temporaneo, chiusi a fine lettura.
    _curl: Option<CurlGuard>,
}

/// GET (senza corpo) o POST JSON verso un provider.
///
/// Su macOS un programma compilato in proprio (come questo, lanciato da
/// cargo) senza il permesso «Rete locale» riceve «No route to host» verso
/// gli altri computer di casa, mentre `/usr/bin/curl` di Apple passa. In quel
/// caso si ripete la richiesta con curl: il server locale risponde lo stesso.
fn http(p: &Provider, key: &str, url: &str, body: Option<&serde_json::Value>) -> Result<HttpResp> {
    let req = match body {
        Some(_) => with_auth(agent(p).post(url), key).set("Accept", "text/event-stream"),
        None => with_auth(agent(p).get(url), key),
    };
    let res = match body {
        Some(b) => req.send_json(b.clone()),
        None => req.call(),
    };
    let to_resp = |r: ureq::Response| HttpResp {
        status: r.status(),
        ctype: r.content_type().to_string(),
        body: Box::new(BufReader::new(r.into_reader())),
        _curl: None,
    };
    match res {
        Ok(r) => Ok(to_resp(r)),
        Err(ureq::Error::Status(_, r)) => Ok(to_resp(r)),
        Err(e) if no_route(&e) && curl_available() => {
            // avvisa una volta sola, poi in silenzio
            static ONCE: std::sync::Once = std::sync::Once::new();
            ONCE.call_once(|| {
                tracing::info!("macOS blocca la rete locale a questo programma (permesso «Rete locale»): uso curl")
            });
            http_curl(url, key, body, p.timeout_s)
                .map_err(|ce| anyhow!("{} — anche con curl: {ce}", transport_msg(&e)))
        }
        Err(e) => bail!("{}", transport_msg(&e)),
    }
}

fn no_route(e: &ureq::Error) -> bool {
    let s = e.to_string();
    s.contains("os error 65") || s.contains("No route to host") || s.contains("os error 113")
}

fn curl_available() -> bool {
    std::path::Path::new(CURL).exists()
}

const CURL: &str = "/usr/bin/curl";

struct CurlGuard {
    child: std::process::Child,
    body_file: Option<std::path::PathBuf>,
}

impl Drop for CurlGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(f) = &self.body_file {
            let _ = std::fs::remove_file(f);
        }
    }
}

/// Stringa per il file di configurazione di curl (tra virgolette).
fn curl_quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// La stessa richiesta fatta da curl. La chiave passa nella configurazione
/// letta da stdin (non compare fra i processi), il corpo da un file temporaneo.
fn http_curl(url: &str, key: &str, body: Option<&serde_json::Value>, timeout_s: u64) -> Result<HttpResp> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    let mut cfg = format!("url = {}\n", curl_quote(url));
    if !key.is_empty() {
        cfg.push_str(&format!("header = {}\n", curl_quote(&format!("Authorization: Bearer {key}"))));
    }
    let mut body_file = None;
    if let Some(b) = body {
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let f = std::env::temp_dir().join(format!("openslate-ai-{}-{n}.json", std::process::id()));
        {
            let mut o = std::fs::OpenOptions::new();
            o.write(true).create(true).truncate(true);
            #[cfg(unix)]
            std::os::unix::fs::OpenOptionsExt::mode(&mut o, 0o600);
            o.open(&f)?.write_all(b.to_string().as_bytes())?;
        }
        cfg.push_str("request = \"POST\"\n");
        cfg.push_str("header = \"Content-Type: application/json\"\n");
        cfg.push_str("header = \"Accept: text/event-stream\"\n");
        cfg.push_str(&format!("data-binary = {}\n", curl_quote(&format!("@{}", f.display()))));
        body_file = Some(f);
    }
    let mut child = Command::new(CURL)
        .args(["-sS", "-i", "-N", "--connect-timeout", "4", "--speed-limit", "1", "--speed-time"])
        .arg(timeout_s.max(5).to_string())
        .args(["-K", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("avvio di curl")?;
    child.stdin.take().unwrap().write_all(cfg.as_bytes())?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take();
    let mut guard = CurlGuard { child, body_file };
    let mut reader = BufReader::new(stdout);

    // Intestazioni (-i): si saltano le risposte intermedie «100 Continue».
    let (mut status, mut ctype) = (0u16, String::new());
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            let _ = guard.child.wait();
            let mut err = String::new();
            if let Some(mut e) = stderr {
                let _ = std::io::Read::read_to_string(&mut e, &mut err);
            }
            let err = err.trim().trim_start_matches("curl: ").to_string();
            bail!("{}", if err.is_empty() { "nessuna risposta".to_string() } else { err });
        }
        let l = line.trim_end();
        if l.starts_with("HTTP/") {
            status = l.split_whitespace().nth(1).and_then(|c| c.parse().ok()).unwrap_or(0);
        } else if let Some((k, v)) = l.split_once(':') {
            if k.eq_ignore_ascii_case("content-type") {
                ctype = v.trim().to_lowercase();
            }
        } else if l.is_empty() && status >= 200 {
            break;
        }
    }
    Ok(HttpResp { status, ctype, body: Box::new(reader), _curl: Some(guard) })
}

/// Nasconde il ragionamento `<think>…</think>` mentre arriva in streaming
/// (e lo passa a parte).
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
            "" // potrebbe essere l'inizio di <think>
        } else {
            t
        }
    }

    /// Il ragionamento dentro `<think>` arrivato finora.
    fn thinking(&self) -> &str {
        let t = self.full.trim_start();
        match t.strip_prefix("<think>") {
            Some(rest) => match rest.find("</think>") {
                Some(i) => &rest[..i],
                // la fine potrebbe essere a metà di «</think>»
                None => {
                    let keep = (1..="</think>".len()).rev().find(|&k| rest.ends_with(&"</think>"[..k])).unwrap_or(0);
                    &rest[..rest.len() - keep]
                }
            },
            None => "",
        }
    }

    /// Aggiunge un pezzo; ritorna il nuovo testo visibile e il nuovo
    /// ragionamento (se ci sono).
    fn push2(&mut self, piece: &str) -> (Option<String>, Option<String>) {
        let vis = self.push(piece);
        let th = self.thinking();
        let t = if th.len() > self.think_sent && th.is_char_boundary(self.think_sent) {
            let d = th[self.think_sent..].to_string();
            self.think_sent = th.len();
            Some(d)
        } else {
            None
        };
        (vis, t)
    }

    /// Aggiunge un pezzo; ritorna il nuovo testo visibile (se c'è).
    fn push(&mut self, piece: &str) -> Option<String> {
        self.full.push_str(piece);
        let vis = self.visible();
        if vis.len() > self.sent {
            let d = vis[self.sent..].to_string();
            self.sent = vis.len();
            Some(d)
        } else {
            None
        }
    }
}

/// Pulisce la risposta finale: ragionamento, ``` attorno.
pub fn clean_reply(s: &str) -> String {
    let s = match s.rfind("</think>") {
        Some(i) => &s[i + "</think>".len()..],
        None => s,
    };
    s.trim().to_string()
}

/// Per la trascrizione: toglie i ``` attorno e porta `\( \)` / `\[ \]` a
/// `$` / `$$` (Typora li mostra di serie).
pub fn tidy_markdown(s: &str) -> String {
    let mut t = clean_reply(s);
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
// Contesto della chat: la pagina che si sta discutendo
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct ChatContext {
    pub title: String,
    pub png: Option<Arc<Vec<u8>>>,
    /// Trascrizione (Markdown + LaTeX), se fatta; per un documento, il suo
    /// testo (tutte le pagine).
    pub markdown: String,
    /// È un documento aperto (PDF, Word…): `markdown` è il testo estratto.
    pub doc: bool,
    /// Cresce a ogni cambio (la pagina web se ne accorge).
    pub rev: u64,
}

static CONTEXT: Mutex<Option<ChatContext>> = Mutex::new(None);

/// Imposta la pagina da discutere in chat.
pub fn set_context(title: &str, png: Option<Arc<Vec<u8>>>, markdown: &str) {
    let mut g = CONTEXT.lock().unwrap();
    let rev = g.as_ref().map(|c| c.rev + 1).unwrap_or(1);
    *g = Some(ChatContext { title: title.into(), png, markdown: markdown.into(), doc: false, rev });
}

/// Imposta un documento da discutere: l'immagine della pagina aperta e il
/// testo di tutto il documento.
pub fn set_context_doc(title: &str, png: Option<Arc<Vec<u8>>>, text: &str) {
    let mut g = CONTEXT.lock().unwrap();
    let rev = g.as_ref().map(|c| c.rev + 1).unwrap_or(1);
    *g = Some(ChatContext { title: title.into(), png, markdown: text.into(), doc: true, rev });
}

pub fn context() -> Option<ChatContext> {
    CONTEXT.lock().unwrap().clone()
}

/// Messaggi per il modello: istruzioni, pagina (se allegata), conversazione.
pub fn chat_messages(cfg: &AiConfig, history: &[(String, String)], ctx: Option<&ChatContext>) -> Vec<Msg> {
    let mut msgs = vec![Msg::new("system", cfg.chat_prompt())];
    let mut first_user = true;
    for (role, text) in history {
        // il riassunto di una compattazione arriva come messaggio dell'utente
        let summary;
        let text = if role == "summary" {
            summary = format!("[Riassunto della conversazione precedente, compattata per spazio]\n\n{text}");
            &summary
        } else {
            text
        };
        let role = if role == "assistant" { "assistant" } else { "user" };
        if role == "user" && first_user {
            first_user = false;
            if let Some(c) = ctx.filter(|c| c.png.is_some() || !c.markdown.trim().is_empty()) {
                let mut t = if c.doc {
                    format!("Documento allegato: «{}». L'immagine è la pagina aperta (con quello che ho scritto sopra a mano).\n", c.title)
                } else {
                    format!("Pagina allegata: «{}».\n", c.title)
                };
                if !c.markdown.trim().is_empty() {
                    t.push_str(if c.doc {
                        "Testo del documento, estratto dal PDF (le pagine sono separate da «--- Pagina N ---»):\n\n"
                    } else {
                        "Trascrizione fedele (Markdown):\n\n"
                    });
                    t.push_str(c.markdown.trim());
                    t.push_str("\n\n");
                }
                t.push_str("---\n\n");
                t.push_str(text);
                let mut m = Msg::new("user", t);
                if let Some(png) = &c.png {
                    m = m.with_image(png.clone());
                }
                msgs.push(m);
                continue;
            }
        }
        msgs.push(Msg::new(role, text.clone()));
    }
    // due messaggi di fila dello stesso ruolo (es. riassunto + domanda) si
    // uniscono: alcuni server non li accettano separati
    let mut merged: Vec<Msg> = Vec::with_capacity(msgs.len());
    for m in msgs {
        match merged.last_mut() {
            Some(last) if last.role == m.role && m.role != "system" => {
                last.text = format!("{}\n\n{}", last.text, m.text);
                if last.image.is_none() {
                    last.image = m.image;
                }
            }
            _ => merged.push(m),
        }
    }
    merged
}

/// Trascrive una pagina (PNG) in Markdown.
pub fn transcribe(png: Arc<Vec<u8>>) -> Result<Reply> {
    let cfg = AiConfig::load()?;
    let msgs = vec![
        Msg::new("system", cfg.ocr_prompt()),
        Msg::new("user", "Trascrivi questa pagina.").with_image(png),
    ];
    let mut r = complete(&cfg, Task::Ocr, &msgs)?;
    r.text = tidy_markdown(&r.text);
    Ok(r)
}

/// Stato dei provider (per `OpenSlate ai` e la pagina della chat).
pub fn describe(cfg: &AiConfig) -> Vec<(String, String)> {
    cfg.providers
        .iter()
        .map(|p| {
            let state = if !p.enabled {
                "disattivato".to_string()
            } else if resolve_key(&p.key).is_none() {
                format!("chiave non trovata ({})", p.key.trim())
            } else {
                let m = if p.model.trim().is_empty() { "automatico" } else { p.model.trim() };
                let v = if !p.vision {
                    "senza visione".to_string()
                } else if p.vision_model.trim().is_empty() {
                    "visione: stesso modello".to_string()
                } else {
                    format!("visione: {}", p.vision_model.trim())
                };
                format!("pronto — modello {m}, {v}")
            };
            (p.name.clone(), state)
        })
        .collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// Finto server OpenAI: risponde a ogni richiesta con `respond(req)`.
    /// Ritorna (porta, richieste ricevute).
    pub(crate) fn fake_server(respond: fn(&str) -> String) -> (u16, Arc<Mutex<Vec<String>>>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let log = Arc::new(Mutex::new(Vec::new()));
        let log2 = log.clone();
        std::thread::spawn(move || {
            for s in l.incoming() {
                let Ok(mut s) = s else { continue };
                let mut req = Vec::new();
                let mut buf = [0u8; 65536];
                loop {
                    let n = match s.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => n,
                    };
                    req.extend_from_slice(&buf[..n]);
                    let t = String::from_utf8_lossy(&req);
                    if let Some(i) = t.find("\r\n\r\n") {
                        let len: usize = t
                            .lines()
                            .find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse().unwrap()))
                            .unwrap_or(0);
                        if req.len() >= i + 4 + len {
                            break;
                        }
                    }
                }
                let req = String::from_utf8_lossy(&req).into_owned();
                let out = respond(&req);
                log2.lock().unwrap().push(req);
                let _ = s.write_all(out.as_bytes());
            }
        });
        (port, log)
    }

    pub(crate) fn sse_reply(pieces: &[&str]) -> String {
        let mut body = String::new();
        for p in pieces {
            let j = serde_json::json!({ "choices": [{ "delta": { "content": p } }] });
            body.push_str(&format!("data: {j}\n\n"));
        }
        body.push_str("data: [DONE]\n\n");
        format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
    }

    fn provider(name: &str, url: String) -> Provider {
        let mut p = Provider::new(name, &url);
        p.timeout_s = 10;
        p
    }

    #[test]
    fn config_di_serie_valida() {
        let c = AiConfig::parse(DEFAULT_TOML).unwrap();
        let names: Vec<_> = c.providers.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["LM Studio", "Ollama", "DeepSeek", "Google Gemini", "Anthropic Claude"]);
        assert!(!c.providers[0].all_vision && !c.providers[2].vision);
        assert!(c.providers[4].no_temperature);
        assert_eq!(c.ocr_prompt(), OCR_PROMPT);
        assert!(c.web.enabled && c.web.count == 10 && c.web.key.starts_with("dsh:"));
    }

    #[test]
    fn salvataggio_rileggibile() {
        let mut c = AiConfig::parse(DEFAULT_TOML).unwrap();
        c.providers[0].context = 65536;
        c.chat.compact_at = 70;
        let text = format!("{SAVED_HEADER}\n{}", toml::to_string_pretty(&c).unwrap());
        let back = AiConfig::parse(&text).unwrap();
        assert_eq!(back.providers.len(), 5);
        assert_eq!(back.providers[0].context, 65536);
        assert_eq!(back.providers[2].model, "deepseek-flash");
        assert_eq!(back.chat.compact_at, 70);
        assert!(back.web.key.starts_with("dsh:"));
    }

    /// Token esatti quando il server li manda (ultimo pezzo con "usage").
    #[test]
    fn token_dal_server() {
        let (port, _) = fake_server(|_| {
            let body = "data: {\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\n\ndata: {\"choices\":[],\"usage\":{\"prompt_tokens\":1200,\"completion_tokens\":34,\"total_tokens\":1234}}\n\ndata: [DONE]\n\n";
            format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n{body}", body.len())
        });
        let mut p = provider("Remoto", format!("http://127.0.0.1:{port}/v1"));
        p.model = "m".into();
        p.context = 100_000;
        let cfg = AiConfig { providers: vec![p], ..Default::default() };
        let mut usage = None;
        run(&cfg, Task::Chat, &[Msg::new("user", "ciao")], &mut |e| {
            if let Event::Usage { used, max, exact, .. } = e {
                usage = Some((used, max, exact));
            }
            true
        })
        .unwrap();
        assert_eq!(usage, Some((1234, 100_000, true)));
    }

    #[test]
    fn chiavi() {
        std::env::set_var("WS_TEST_KEY", "sk-123");
        assert_eq!(resolve_key(""), Some(String::new()));
        assert_eq!(resolve_key("env:WS_NON_ESISTE | env:WS_TEST_KEY").as_deref(), Some("sk-123"));
        assert_eq!(resolve_key("env:WS_NON_ESISTE"), None);
        let yaml = "version: 1\nrefs:\n  DEEPSEEK_API_KEY: sk-abc\n  \"MTPLX_API_KEY\": 'mt-1'\nrecords:\n  x:\n    secret: no\n";
        assert_eq!(dsh_key_from(yaml, "DEEPSEEK_API_KEY").as_deref(), Some("sk-abc"));
        assert_eq!(dsh_key_from(yaml, "MTPLX_API_KEY").as_deref(), Some("mt-1"));
        assert_eq!(dsh_key_from(yaml, "secret"), None);
    }

    #[test]
    fn scelta_modello() {
        let ids: Vec<String> = ["text-embedding-nomic", "qwen3-27b", "qwen3-vl-8b-instruct"].iter().map(|s| s.to_string()).collect();
        assert_eq!(choose_model(&ids, true).as_deref(), Some("qwen3-vl-8b-instruct"));
        assert_eq!(choose_model(&ids, false).as_deref(), Some("qwen3-27b"));
        assert_eq!(choose_model(&ids[..2], true), None);
        // un server locale con solo un modello da 1B e uno di embedding
        let mac: Vec<String> = ["llama-3.2-1b-instruct", "text-embedding-nomic-embed-text-v1.5"].iter().map(|s| s.to_string()).collect();
        assert!(choose_model_why(&mac, false).unwrap_err().contains("troppo piccoli"));
        assert!(choose_model_why(&mac, true).unwrap_err().contains("Qwen3-VL-8B"));
        // in chat va bene anche un modello con visione, se è il più grande
        let ids2: Vec<String> = ["llama-3.2-1b-instruct", "qwen3-vl-8b"].iter().map(|s| s.to_string()).collect();
        assert_eq!(choose_model(&ids2, false).as_deref(), Some("qwen3-vl-8b"));
        assert_eq!(model_size("qwen3-30b-a3b-instruct"), Some(30.0));
        assert_eq!(model_size("llama-3.2-1b-instruct"), Some(1.0));
        assert_eq!(model_size("gpt-oss"), None);
    }

    #[test]
    fn filtro_ragionamento_in_streaming() {
        let mut f = ThinkFilter::default();
        let mut out = String::new();
        for p in ["<thi", "nk>uhm, vediamo", "</think>\n\nCia", "o $x^2$"] {
            if let Some(d) = f.push(p) {
                out.push_str(&d);
            }
        }
        assert_eq!(out, "Ciao $x^2$");
        let mut f = ThinkFilter::default();
        assert_eq!(f.push("<"), None);
        assert_eq!(f.push("b>"), Some("<b>".into()));
        // il ragionamento arriva a parte, senza pezzi di «</think>»
        let mut f = ThinkFilter::default();
        let (mut vis, mut th) = (String::new(), String::new());
        for p in ["<think>pen", "so a x</thi", "nk>Ri", "sposta"] {
            let (v, t) = f.push2(p);
            vis.push_str(&v.unwrap_or_default());
            th.push_str(&t.unwrap_or_default());
        }
        assert_eq!(th, "penso a x");
        assert_eq!(vis, "Risposta");
    }

    #[test]
    fn markdown_ordinato() {
        assert_eq!(tidy_markdown("```markdown\n$x$ e \\(y\\)\n\\[z\\]\n```"), "$x$ e $y$\n$$z$$");
        // \\ (a capo LaTeX) resta com'è
        assert_eq!(tidy_markdown("$$\\begin{pmatrix}1\\\\2\\end{pmatrix}$$"), "$$\\begin{pmatrix}1\\\\2\\end{pmatrix}$$");
    }

    /// Primo provider spento, secondo senza chiave, terzo risponde in streaming.
    #[test]
    fn fallback_e_streaming() {
        let (port, log) = fake_server(|req| {
            if req.starts_with("GET /v1/models") {
                let b = r#"{"data":[{"id":"nomic-embed"},{"id":"qwen3-vl-8b"}]}"#;
                format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{b}", b.len())
            } else {
                sse_reply(&["<think>.</think>", "## Esercizio 1\n", "$$\\int_0^1 x\\,dx = 1$$"])
            }
        });
        // porta chiusa: un listener appena liberato
        let dead = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let mut nokey = provider("Senza chiave", format!("http://127.0.0.1:{port}/v1"));
        nokey.key = "env:WS_CHIAVE_ASSENTE".into();
        let cfg = AiConfig {
            providers: vec![
                provider("Spento", format!("http://127.0.0.1:{dead}/v1")),
                nokey,
                provider("Locale", format!("http://127.0.0.1:{port}/v1/")),
            ],
            ..Default::default()
        };
        let msgs = vec![Msg::new("user", "trascrivi").with_image(Arc::new(b"\x89PNG".to_vec()))];
        let mut events = Vec::new();
        let r = run(&cfg, Task::Ocr, &msgs, &mut |e| {
            events.push(e);
            true
        })
        .unwrap();
        assert_eq!(r.provider, "Locale");
        assert_eq!(r.model, "qwen3-vl-8b");
        assert_eq!(r.text, "## Esercizio 1\n$$\\int_0^1 x\\,dx = 1$$");
        let skipped: Vec<_> = events.iter().filter_map(|e| match e { Event::Skipped { provider, .. } => Some(provider.as_str()), _ => None }).collect();
        assert_eq!(skipped, ["Spento", "Senza chiave"]);
        let deltas: String = events.iter().filter_map(|e| match e { Event::Delta(d) => Some(d.as_str()), _ => None }).collect();
        assert_eq!(deltas, r.text);
        let reqs = log.lock().unwrap();
        let post = reqs.iter().find(|r| r.starts_with("POST /v1/chat/completions")).unwrap();
        assert!(post.contains("\"stream\":true") && post.contains("data:image/png;base64,iVBORw"));
        assert!(post.contains("\"temperature\":0"));
    }

    /// Errore HTTP (es. modello senza visione) → si passa al prossimo;
    /// tutti falliti → elenco dei motivi.
    #[test]
    fn errori_spiegati() {
        let (port, _) = fake_server(|_| {
            let b = r#"{"error":{"message":"model does not support images"}}"#;
            format!("HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{b}", b.len())
        });
        let mut p = provider("Remoto", format!("http://127.0.0.1:{port}"));
        p.model = "m".into();
        let cfg = AiConfig { providers: vec![p], ..Default::default() };
        let e = complete(&cfg, Task::Chat, &[Msg::new("user", "ciao")]).unwrap_err().to_string();
        assert!(e.contains("Remoto: HTTP 400: model does not support images"), "{e}");
    }

    /// Provider senza visione: in chat riceve solo il testo.
    #[test]
    fn immagine_tolta_a_chi_non_vede() {
        let (port, log) = fake_server(|_| sse_reply(&["ok"]));
        let mut p = provider("Testo", format!("http://127.0.0.1:{port}"));
        p.model = "deepseek-v4-flash".into();
        p.vision = false;
        p.no_temperature = true;
        let cfg = AiConfig { providers: vec![p], ..Default::default() };
        let msgs = vec![Msg::new("user", "guarda").with_image(Arc::new(vec![1, 2, 3]))];
        assert_eq!(complete(&cfg, Task::Chat, &msgs).unwrap().text, "ok");
        let req = log.lock().unwrap()[0].clone();
        assert!(!req.contains("image_url") && req.contains("\"content\":\"guarda\""));
        assert!(!req.contains("temperature"));
        // per l'OCR invece viene saltato
        assert!(complete(&cfg, Task::Ocr, &msgs).is_err());
    }

    /// Il trasporto di riserva con curl: modelli (GET) e risposta in streaming (POST).
    #[test]
    fn trasporto_curl() {
        if !curl_available() {
            return;
        }
        let (port, log) = fake_server(|req| {
            if req.starts_with("GET") {
                let b = r#"{"data":[{"id":"mtplx-qwen38-27b"}]}"#;
                format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{b}", b.len())
            } else {
                sse_reply(&["ciao ", "$x$"])
            }
        });
        let url = format!("http://127.0.0.1:{port}/v1");
        let r = http_curl(&format!("{url}/models"), "k\"1", None, 10).unwrap();
        assert_eq!(r.status, 200);
        let v: serde_json::Value = serde_json::from_reader(r.body).unwrap();
        assert_eq!(v["data"][0]["id"], "mtplx-qwen38-27b");
        let body = serde_json::json!({ "model": "m", "stream": true, "messages": [] });
        let r = http_curl(&format!("{url}/chat/completions"), "", Some(&body), 10).unwrap();
        assert!(r.ctype.contains("event-stream"));
        let text: String = r.body.lines().map_while(Result::ok).filter(|l| l.starts_with("data: {")).count().to_string();
        assert_eq!(text, "2");
        let reqs = log.lock().unwrap();
        assert!(reqs[0].contains("Bearer k\"1"));
        assert!(reqs[1].starts_with("POST /v1/chat/completions") && reqs[1].contains("\"stream\":true"));
        // server spento: errore leggibile
        let dead = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        assert!(http_curl(&format!("http://127.0.0.1:{dead}/v1/models"), "", None, 5).is_err());
    }

    /// Il modello chiede uno strumento (in streaming, argomenti a pezzi),
    /// lo si esegue e la risposta finale arriva al secondo giro.
    #[test]
    fn strumenti_in_chat() {
        let (port, log) = fake_server(|req| {
            if req.contains("\"role\":\"tool\"") {
                sse_reply(&["La partita IVA ", "non è valida."])
            } else {
                let c1 = serde_json::json!({"choices":[{"delta":{"tool_calls":[{"index":0,"id":"c1","type":"function","function":{"name":"verifica_partita_iva","arguments":"{\"partita_"}}]}}]});
                let c2 = serde_json::json!({"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"iva\":\"18690721009\"}"}}]}}]});
                let body = format!("data: {c1}\n\ndata: {c2}\n\ndata: [DONE]\n\n");
                format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
            }
        });
        let mut p = provider("Locale", format!("http://127.0.0.1:{port}/v1"));
        p.model = "m".into();
        let cfg = AiConfig {
            providers: vec![p],
            web: crate::tools::WebTools { key: "env:WS_NESSUNA_CHIAVE".into(), ..Default::default() },
            ..Default::default()
        };
        let mut events = Vec::new();
        let r = run_with(&cfg, Task::Chat, &[Msg::new("system", "S"), Msg::new("user", "è valida?")], true, &mut |e| {
            events.push(e);
            true
        })
        .unwrap();
        assert_eq!(r.text, "La partita IVA non è valida.");
        assert!(events.iter().any(|e| matches!(e, Event::ToolDone { result, .. } if result == "numero non valido")));
        let reqs = log.lock().unwrap();
        // primo giro: solo lo strumento P.IVA (senza chiave niente ricerca web)
        assert!(reqs[0].contains("verifica_partita_iva") && !reqs[0].contains("web_search"));
        assert!(reqs[0].contains("Oggi è il"));
        assert!(reqs[1].contains("\"tool_call_id\":\"c1\"") && reqs[1].contains("SBAGLIATA"));
        // senza strumenti la richiesta non li offre
        drop(reqs);
        let _ = run(&cfg, Task::Chat, &[Msg::new("user", "ciao")], &mut |_| true);
        assert!(!log.lock().unwrap().last().unwrap().contains("\"tools\""));
    }

    #[test]
    fn contesto_nel_primo_messaggio() {
        let cfg = AiConfig::default();
        let ctx = ChatContext { title: "Es. 3".into(), png: Some(Arc::new(vec![9])), markdown: "$x=2$".into(), doc: false, rev: 1 };
        let h = vec![("user".to_string(), "è giusto?".to_string()), ("assistant".into(), "no".into()), ("user".into(), "perché?".into())];
        let m = chat_messages(&cfg, &h, Some(&ctx));
        assert_eq!(m.len(), 4);
        assert_eq!(m[0].role, "system");
        // riassunto + domanda: un solo messaggio dell'utente, con la pagina
        let h2 = vec![("summary".to_string(), "Si parlava di $x$".to_string()), ("user".into(), "e poi?".into())];
        let m2 = chat_messages(&cfg, &h2, Some(&ctx));
        assert_eq!(m2.len(), 2);
        assert!(m2[1].text.contains("Riassunto della conversazione") && m2[1].text.ends_with("e poi?") && m2[1].image.is_some());
        assert!(m[1].text.contains("$x=2$") && m[1].text.ends_with("è giusto?") && m[1].image.is_some());
        assert!(m[3].image.is_none());
    }
}
