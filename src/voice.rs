//! La voce nella chat: si parla al microfono e il testo finisce nel riquadro
//! della chat.
//!
//! Il microfono lo registra l'app ([`Recorder`], con cpal: CoreAudio su macOS,
//! WASAPI su Windows, ALSA su Linux), non la pagina della chat. A trascrivere è
//! Whisper sul computer (whisper.cpp, offline; il modello si scarica la prima
//! volta, col permesso dell'utente) oppure, se in `ai.toml` è scelto così, un
//! servizio in rete compatibile OpenAI (`/audio/transcriptions`: OpenAI, Groq,
//! Mistral…). La scelta sta nella sezione `[voice]` di `ai.toml`.

use anyhow::{anyhow, bail, Context, Result};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Whisper vuole l'audio mono a 16 kHz.
pub const RATE: u32 = 16_000;
/// Oltre questa durata la registrazione si ferma da sola.
pub const MAX_SECONDS: u64 = 300;

// ---------------------------------------------------------------------------
// I modelli di Whisper
// ---------------------------------------------------------------------------

/// Un modello di Whisper (formato di whisper.cpp) che si può scaricare.
pub struct Model {
    /// Il nome da scrivere in `[voice] model`.
    pub name: &'static str,
    pub file: &'static str,
    pub mb: u32,
    /// Impronta SHA-1 del file, dall'elenco dei modelli di whisper.cpp.
    sha1: &'static str,
}

/// I modelli compressi (pesano un terzo e sbagliano quasi uguale): il primo è
/// quello di serie, il migliore per l'italiano a questa velocità.
pub const MODELS: [Model; 3] = [
    Model { name: "large-v3-turbo", file: "ggml-large-v3-turbo-q5_0.bin", mb: 547, sha1: "e050f7970618a659205450ad97eb95a18d69c9ee" },
    Model { name: "small", file: "ggml-small-q5_1.bin", mb: 181, sha1: "6fe57ddcfdd1c6b07cdcc73aaf620810ce5fc771" },
    Model { name: "base", file: "ggml-base-q5_1.bin", mb: 57, sha1: "a3733eda680ef76256db5fc5dd9de8629e62c5e7" },
];

const MODELS_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";

/// Il modello chiamato `name` (vuoto o sconosciuto: quello di serie).
pub fn model(name: &str) -> &'static Model {
    let n = name.trim();
    MODELS.iter().find(|m| m.name == n || m.file == n).unwrap_or(&MODELS[0])
}

/// Dove sta il modello: `models/whisper/` nella cartella dei dati.
pub fn model_path(m: &Model) -> PathBuf {
    PathBuf::from("models").join("whisper").join(m.file)
}

// ---------------------------------------------------------------------------
// Chi trascrive
// ---------------------------------------------------------------------------

/// Chi trascrive, secondo `[voice]` di `ai.toml`.
pub enum Engine {
    /// Whisper sul computer.
    Whisper(&'static Model),
    /// Un agente di `ai.toml` con l'API `/audio/transcriptions`, e il modello.
    Remote(crate::ai::Provider, String),
}

pub fn engine(cfg: &crate::ai::AiConfig) -> Result<Engine> {
    let name = cfg.voice.engine.trim();
    if name.is_empty() || name.eq_ignore_ascii_case("whisper") {
        return Ok(Engine::Whisper(model(&cfg.voice.model)));
    }
    let Some(p) = cfg.providers.iter().find(|p| p.name == name) else {
        bail!("l'agente «{name}» scelto per la voce in ai.toml non c'è");
    };
    let m = cfg.voice.model.trim();
    if m.is_empty() || MODELS.iter().any(|x| x.name == m) {
        bail!("in ai.toml, [voice] model: scrivi il modello di trascrizione di «{name}» (per esempio voxtral-mini-latest, whisper-large-v3-turbo o gpt-4o-transcribe)");
    }
    // scelto apposta per la voce: vale anche se è spento per la chat
    Ok(Engine::Remote(p.clone(), m.to_string()))
}

/// Il modello di Whisper da scaricare prima di dettare, se manca.
pub fn model_missing() -> Option<&'static Model> {
    let cfg = crate::ai::AiConfig::load().ok()?;
    match engine(&cfg).ok()? {
        Engine::Whisper(m) if !model_path(m).exists() => Some(m),
        _ => None,
    }
}

/// La lingua di `[voice] language`: vuota = quella dell'interfaccia,
/// «auto» = la riconosce Whisper.
fn language(setting: &str) -> Option<String> {
    match setting.trim() {
        "" => Some(crate::i18n::lang().code().to_string()),
        s if s.eq_ignore_ascii_case("auto") => None,
        s => Some(s.to_ascii_lowercase()),
    }
}

/// Trascrive l'audio registrato (mono, 16 kHz). Errore se non c'è parlato.
pub fn transcribe(pcm: &[f32]) -> Result<String> {
    let cfg = crate::ai::AiConfig::load()?;
    let lang = language(&cfg.voice.language);
    // tutto zero: il sistema non lascia sentire (macOS senza il permesso)
    if pcm.len() > RATE as usize / 2 && pcm.iter().all(|x| *x == 0.0) {
        bail!("il microfono non manda suono: controlla il permesso del microfono nelle impostazioni del sistema (su macOS: Impostazioni di Sistema → Privacy e sicurezza → Microfono)");
    }
    let Some(speech) = trim_silence(pcm) else {
        bail!("non ho sentito niente: parla più vicino al microfono");
    };
    let text = match engine(&cfg)? {
        Engine::Whisper(m) => {
            let path = model_path(m);
            if !path.exists() {
                bail!("manca il modello di Whisper «{}»", m.name);
            }
            local::transcribe(&path, speech, lang.as_deref())?
        }
        Engine::Remote(p, model) => remote(&p, &model, speech, lang.as_deref())?,
    };
    let text = clean(&text);
    if text.is_empty() {
        bail!("non ho sentito niente: parla più vicino al microfono");
    }
    Ok(text)
}

/// Ripulisce il testo trascritto: spazi, e le frasi che Whisper «sente» nel
/// silenzio o nel rumore (le ha imparate dai sottotitoli dei video).
fn clean(text: &str) -> String {
    let t = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let bare = t.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
    const NOISE: [&str; 14] = [
        "sottotitoli creati dalla comunità amara.org",
        "sottotitoli a cura di qtss",
        "sottotitoli e revisione a cura di qtss",
        "grazie per la visione",
        "grazie a tutti",
        "thank you",
        "thanks for watching",
        "thank you for watching",
        "untertitel der amara.org-community",
        "untertitel im auftrag des zdf, 2017",
        "sous-titrage st' 501",
        "ご視聴ありがとうございました",
        "продолжение следует",
        "musica",
    ];
    let only_tags = t.split_whitespace().all(|w| (w.starts_with('[') && w.ends_with(']')) || (w.starts_with('(') && w.ends_with(')')));
    if bare.is_empty() || only_tags || NOISE.contains(&bare.as_str()) {
        return String::new();
    }
    t
}

/// Toglie il silenzio all'inizio e alla fine (con un po' di margine). `None`
/// se non c'è niente che somigli a una voce.
fn trim_silence(pcm: &[f32]) -> Option<&[f32]> {
    const FRAME: usize = (RATE / 50) as usize; // 20 ms
    if pcm.len() < FRAME * 5 {
        return None;
    }
    let rms: Vec<f32> = pcm.chunks(FRAME).map(|c| (c.iter().map(|x| x * x).sum::<f32>() / c.len() as f32).sqrt()).collect();
    let mut sorted = rms.clone();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let noise = sorted[sorted.len() / 5];
    let peak = *sorted.last()?;
    // una voce: ben sopra il rumore di fondo e non un microfono muto
    if peak < 0.004 || peak < noise * 3.0 {
        return None;
    }
    let thr = (noise * 2.5).max(peak * 0.08).max(0.002);
    let first = rms.iter().position(|&r| r > thr)?;
    let last = rms.iter().rposition(|&r| r > thr)?;
    let pad = 15; // 300 ms
    let a = first.saturating_sub(pad) * FRAME;
    let b = ((last + 1 + pad) * FRAME).min(pcm.len());
    // Whisper vuole almeno un secondo: il silenzio intorno resta
    if b - a < RATE as usize {
        let mid = (a + b) / 2;
        let a2 = mid.saturating_sub(RATE as usize / 2);
        return Some(&pcm[a2..(a2 + RATE as usize).min(pcm.len())]);
    }
    Some(&pcm[a..b])
}

// ---------------------------------------------------------------------------
// Whisper sul computer
// ---------------------------------------------------------------------------

mod local {
    use super::*;
    use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

    /// Il modello resta in memoria fra un messaggio e l'altro (si carica in un
    /// secondo o due) e si libera dopo qualche minuto senza dettare.
    struct Loaded {
        path: PathBuf,
        ctx: WhisperContext,
        used: Instant,
    }

    static LOADED: Mutex<Option<Loaded>> = Mutex::new(None);
    const IDLE: Duration = Duration::from_secs(300);

    fn reaper() {
        static STARTED: AtomicBool = AtomicBool::new(false);
        if STARTED.swap(true, Ordering::SeqCst) {
            return;
        }
        let _ = std::thread::Builder::new().name("whisper-libera".into()).spawn(|| loop {
            std::thread::sleep(Duration::from_secs(30));
            if let Ok(mut g) = LOADED.lock() {
                if g.as_ref().is_some_and(|l| l.used.elapsed() > IDLE) {
                    *g = None;
                    tracing::info!("voce: modello di Whisper liberato dalla memoria");
                }
            }
        });
    }

    pub fn transcribe(path: &std::path::Path, pcm: &[f32], lang: Option<&str>) -> Result<String> {
        static HOOKS: std::sync::Once = std::sync::Once::new();
        HOOKS.call_once(whisper_rs::install_logging_hooks);
        let mut g = LOADED.lock().map_err(|_| anyhow!("Whisper: errore interno"))?;
        if g.as_ref().is_none_or(|l| l.path != path) {
            *g = None;
            let p = path.to_str().ok_or_else(|| anyhow!("percorso del modello non valido"))?;
            let ctx = WhisperContext::new_with_params(p, WhisperContextParameters::default())
                .map_err(|e| anyhow!("Whisper non carica il modello {}: {e} (cancellalo: si riscarica)", path.display()))?;
            *g = Some(Loaded { path: path.to_path_buf(), ctx, used: Instant::now() });
        }
        let l = g.as_mut().unwrap();
        l.used = Instant::now();
        let mut state = l.ctx.create_state().map_err(|e| anyhow!("Whisper: {e}"))?;
        let mut p = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        p.set_language(Some(lang.unwrap_or("auto")));
        p.set_translate(false);
        p.set_no_context(true);
        p.set_no_timestamps(true);
        p.set_suppress_blank(true);
        p.set_print_special(false);
        p.set_print_progress(false);
        p.set_print_realtime(false);
        p.set_print_timestamps(false);
        let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(1, 8);
        p.set_n_threads(threads as _);
        state.full(p, pcm).map_err(|e| anyhow!("Whisper: {e}"))?;
        let mut out = String::new();
        for i in 0..state.full_n_segments() {
            if let Some(s) = state.get_segment(i) {
                if let Ok(t) = s.to_str_lossy() {
                    out.push_str(&t);
                    out.push(' ');
                }
            }
        }
        drop(g);
        reaper();
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// Servizio in rete (API compatibile OpenAI)
// ---------------------------------------------------------------------------

/// `POST {url}/audio/transcriptions` con il file WAV: OpenAI, Groq, Mistral
/// e i server locali che parlano la stessa lingua.
fn remote(p: &crate::ai::Provider, model: &str, pcm: &[f32], lang: Option<&str>) -> Result<String> {
    let key = crate::ai::resolve_key(&p.key).ok_or_else(|| anyhow!("la chiave di «{}» non si trova ({})", p.name, p.key.trim()))?;
    let boundary = format!("openslate-{:x}", rand_u64());
    let mut fields: Vec<(&str, String)> = vec![("model", model.to_string()), ("response_format", "json".into())];
    if let Some(l) = lang {
        fields.push(("language", l.to_string()));
    }
    let body = multipart(&boundary, &fields, "audio.wav", &wav(pcm));
    let url = format!("{}/audio/transcriptions", p.url.trim().trim_end_matches('/'));
    let mut req = ureq::post(&url)
        .timeout(Duration::from_secs(p.timeout_s.clamp(10, 600)))
        .set("Content-Type", &format!("multipart/form-data; boundary={boundary}"));
    if !key.is_empty() {
        req = req.set("Authorization", &format!("Bearer {key}"));
    }
    let r = match req.send_bytes(&body) {
        Ok(r) => r,
        Err(ureq::Error::Status(code, r)) => {
            let msg = r.into_string().unwrap_or_default();
            let msg: String = msg.chars().take(300).collect();
            bail!("«{}» non trascrive ({code}): {msg}", p.name);
        }
        Err(e) => bail!("«{}» non risponde: {e}", p.name),
    };
    let text = r.into_string().context("risposta della trascrizione")?;
    Ok(match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(v) => v["text"].as_str().unwrap_or_default().to_string(),
        Err(_) => text,
    })
}

fn rand_u64() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
    h.finish()
}

fn multipart(boundary: &str, fields: &[(&str, String)], file_name: &str, file: &[u8]) -> Vec<u8> {
    let mut b = Vec::with_capacity(file.len() + 1024);
    for (k, v) in fields {
        b.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{k}\"\r\n\r\n{v}\r\n").as_bytes());
    }
    b.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{file_name}\"\r\nContent-Type: audio/wav\r\n\r\n").as_bytes());
    b.extend_from_slice(file);
    b.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    b
}

/// WAV a 16 bit, mono, 16 kHz.
fn wav(pcm: &[f32]) -> Vec<u8> {
    let data = (pcm.len() * 2) as u32;
    let mut b = Vec::with_capacity(44 + data as usize);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes()); // PCM
    b.extend_from_slice(&1u16.to_le_bytes()); // mono
    b.extend_from_slice(&RATE.to_le_bytes());
    b.extend_from_slice(&(RATE * 2).to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data.to_le_bytes());
    for &x in pcm {
        b.extend_from_slice(&((x.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    b
}

// ---------------------------------------------------------------------------
// Scaricare il modello
// ---------------------------------------------------------------------------

/// Scarica il modello da Hugging Face (il deposito di whisper.cpp) e lo tiene
/// solo se l'impronta è quella giusta. `progress(scaricati, totale)`;
/// `cancel` lo ferma.
pub fn download(m: &Model, cancel: &AtomicBool, progress: impl FnMut(u64, u64)) -> Result<()> {
    download_to(&format!("{MODELS_URL}/{}", m.file), m, &model_path(m), cancel, progress)
}

fn download_to(url: &str, m: &Model, path: &std::path::Path, cancel: &AtomicBool, mut progress: impl FnMut(u64, u64)) -> Result<()> {
    use sha1::Digest;
    use std::io::{Read, Write};
    let agent = ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(30)).timeout_read(Duration::from_secs(90)).build();
    let r = agent.get(url).call().map_err(|e| anyhow!("modello di Whisper non scaricato: {e}"))?;
    let expected = m.mb as u64 * 1024 * 1024;
    let total = r.header("Content-Length").and_then(|v| v.parse::<u64>().ok()).unwrap_or(expected);
    if total > expected * 2 {
        bail!("modello di Whisper: il file è più grande del previsto ({} MB)", total >> 20);
    }
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    let part = path.with_extension("bin.part");
    let res = (|| -> Result<()> {
        let mut f = std::fs::File::create(&part).with_context(|| format!("scrittura di {}", part.display()))?;
        let mut hash = sha1::Sha1::new();
        let mut reader = r.into_reader().take(expected * 2);
        let mut buf = vec![0u8; 1 << 16];
        let (mut done, mut shown) = (0u64, 0u64);
        loop {
            if cancel.load(Ordering::Relaxed) {
                bail!("scaricamento annullato");
            }
            let n = reader.read(&mut buf).context("modello di Whisper: scaricamento interrotto")?;
            if n == 0 {
                break;
            }
            hash.update(&buf[..n]);
            f.write_all(&buf[..n])?;
            done += n as u64;
            if done - shown >= 1 << 20 {
                shown = done;
                progress(done, total.max(done));
            }
        }
        f.flush()?;
        let got: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
        if got != m.sha1 {
            bail!("modello di Whisper scaricato ma diverso da quello atteso (impronta {got}): non lo uso");
        }
        Ok(())
    })();
    match res {
        Ok(()) => {
            std::fs::rename(&part, path)?;
            Ok(())
        }
        Err(e) => {
            let _ = std::fs::remove_file(&part);
            Err(e)
        }
    }
}

// ---------------------------------------------------------------------------
// Il microfono
// ---------------------------------------------------------------------------

/// Una registrazione dal microfono predefinito, in un thread suo.
pub struct Recorder {
    stop: Arc<AtomicBool>,
    level: Arc<AtomicU32>,
    thread: Option<std::thread::JoinHandle<Result<Vec<f32>>>>,
    pub started: Instant,
}

impl Recorder {
    /// Apre il microfono predefinito e comincia a registrare.
    pub fn start() -> Result<Recorder> {
        let stop = Arc::new(AtomicBool::new(false));
        let level = Arc::new(AtomicU32::new(0));
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<()>>();
        let (s2, l2) = (stop.clone(), level.clone());
        let thread = std::thread::Builder::new().name("microfono".into()).spawn(move || record(s2, l2, ready_tx))?;
        match ready_rx.recv_timeout(Duration::from_secs(10)) {
            Ok(Ok(())) => Ok(Recorder { stop, level, thread: Some(thread), started: Instant::now() }),
            Ok(Err(e)) => {
                let _ = thread.join();
                Err(e)
            }
            Err(_) => {
                stop.store(true, Ordering::SeqCst);
                bail!("il microfono non risponde")
            }
        }
    }

    /// Volume adesso, 0–1 (per l'indicatore).
    pub fn level(&self) -> f32 {
        f32::from_bits(self.level.load(Ordering::Relaxed))
    }

    /// Si è fermata da sola (troppo lunga, microfono staccato).
    pub fn ended(&self) -> bool {
        self.thread.as_ref().is_none_or(|t| t.is_finished())
    }

    /// Ferma la registrazione e dà l'audio: mono, 16 kHz.
    pub fn finish(mut self) -> Result<Vec<f32>> {
        self.stop.store(true, Ordering::SeqCst);
        self.thread.take().ok_or_else(|| anyhow!("registrazione già chiusa"))?.join().map_err(|_| anyhow!("microfono: errore interno"))?
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

fn record(stop: Arc<AtomicBool>, level: Arc<AtomicU32>, ready: std::sync::mpsc::Sender<Result<()>>) -> Result<Vec<f32>> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    let opened = (|| -> Result<(cpal::Stream, Arc<Mutex<Vec<f32>>>, u32)> {
        let host = cpal::default_host();
        let dev = host.default_input_device().ok_or_else(|| anyhow!("nessun microfono trovato"))?;
        let cfg = dev.default_input_config().map_err(|e| anyhow!("microfono: {e}"))?;
        let rate = cfg.sample_rate();
        let channels = cfg.channels().max(1) as usize;
        let format = cfg.sample_format();
        let buf: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::with_capacity(rate as usize * 30)));
        let err = |e: cpal::Error| tracing::warn!("microfono: {e}");
        macro_rules! stream {
            ($t:ty) => {{
                let (b, l) = (buf.clone(), level.clone());
                dev.build_input_stream::<$t, _, _>(
                    cfg.config(),
                    move |data: &[$t], _| {
                        let mut sum = 0.0f32;
                        let mut n = 0usize;
                        if let Ok(mut out) = b.lock() {
                            for frame in data.chunks(channels) {
                                let x = frame.iter().map(|s| <f32 as cpal::FromSample<$t>>::from_sample_(*s)).sum::<f32>() / frame.len() as f32;
                                out.push(x);
                                sum += x * x;
                                n += 1;
                            }
                        }
                        if n > 0 {
                            l.store((sum / n as f32).sqrt().min(1.0).to_bits(), Ordering::Relaxed);
                        }
                    },
                    err,
                    None,
                )
            }};
        }
        let stream = match format {
            cpal::SampleFormat::F32 => stream!(f32),
            cpal::SampleFormat::F64 => stream!(f64),
            cpal::SampleFormat::I16 => stream!(i16),
            cpal::SampleFormat::I32 => stream!(i32),
            cpal::SampleFormat::U16 => stream!(u16),
            cpal::SampleFormat::I8 => stream!(i8),
            cpal::SampleFormat::U8 => stream!(u8),
            other => bail!("microfono: formato audio {other:?} non gestito"),
        }
        .map_err(|e| anyhow!("microfono: {e}"))?;
        stream.play().map_err(|e| anyhow!("microfono: {e}"))?;
        Ok((stream, buf, rate))
    })();
    let (stream, buf, rate) = match opened {
        Ok(v) => {
            let _ = ready.send(Ok(()));
            v
        }
        Err(e) => {
            let msg = format!("{e:#}");
            let _ = ready.send(Err(e));
            bail!("{msg}");
        }
    };
    let t0 = Instant::now();
    while !stop.load(Ordering::SeqCst) && t0.elapsed() < Duration::from_secs(MAX_SECONDS) {
        std::thread::sleep(Duration::from_millis(20));
    }
    drop(stream);
    let raw = std::mem::take(&mut *buf.lock().map_err(|_| anyhow!("microfono: errore interno"))?);
    Ok(resample(&raw, rate, RATE))
}

/// Cambia la frequenza: verso il basso media i campioni che diventano uno
/// (un passa-basso semplice, basta per la voce), verso l'alto interpola.
fn resample(x: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || x.is_empty() || from == 0 {
        return x.to_vec();
    }
    let ratio = from as f64 / to as f64;
    let n = (x.len() as f64 / ratio).floor() as usize;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let c = i as f64 * ratio;
        if ratio > 1.0 {
            let a = (c - ratio / 2.0).max(0.0) as usize;
            let b = ((c + ratio / 2.0).ceil() as usize).clamp(a + 1, x.len());
            out.push(x[a..b].iter().sum::<f32>() / (b - a) as f32);
        } else {
            let a = c.floor() as usize;
            let f = (c - a as f64) as f32;
            let y0 = x[a.min(x.len() - 1)];
            let y1 = x[(a + 1).min(x.len() - 1)];
            out.push(y0 + (y1 - y0) * f);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(secs: f32, amp: f32) -> Vec<f32> {
        (0..(secs * RATE as f32) as usize).map(|i| amp * (i as f32 * 2.0 * std::f32::consts::PI * 220.0 / RATE as f32).sin()).collect()
    }

    #[test]
    fn ricampionamento() {
        // 48 kHz → 16 kHz: un terzo dei campioni, il tono resta
        let x: Vec<f32> = (0..48_000).map(|i| (i as f32 * 2.0 * std::f32::consts::PI * 220.0 / 48_000.0).sin()).collect();
        let y = resample(&x, 48_000, RATE);
        assert_eq!(y.len(), 16_000);
        let rms = (y.iter().map(|v| v * v).sum::<f32>() / y.len() as f32).sqrt();
        assert!((rms - 0.707).abs() < 0.02, "{rms}");
        // 8 kHz → 16 kHz: il doppio
        assert_eq!(resample(&[0.0, 1.0, 0.0, -1.0], 8_000, RATE).len(), 8);
        assert_eq!(resample(&[0.5; 10], RATE, RATE), vec![0.5; 10]);
    }

    #[test]
    fn silenzio_tolto() {
        // silenzio, voce, silenzio: resta la voce con un po' di margine
        let mut pcm = vec![0.0005f32; RATE as usize * 2];
        pcm.extend(tone(1.5, 0.3));
        pcm.extend(vec![0.0005; RATE as usize * 2]);
        let s = trim_silence(&pcm).unwrap();
        let secs = s.len() as f32 / RATE as f32;
        assert!((1.5..2.3).contains(&secs), "{secs}");
        // solo rumore di fondo, o niente: nessuna voce
        assert!(trim_silence(&vec![0.0005; RATE as usize * 3]).is_none());
        assert!(trim_silence(&[]).is_none());
        // una parola breve: almeno un secondo per Whisper
        let mut short = vec![0.0; RATE as usize];
        short.extend(tone(0.3, 0.3));
        short.extend(vec![0.0; RATE as usize]);
        assert_eq!(trim_silence(&short).unwrap().len(), RATE as usize);
    }

    #[test]
    fn testo_ripulito() {
        assert_eq!(clean("  Ciao,   come   stai? "), "Ciao, come stai?");
        assert_eq!(clean(" Sottotitoli creati dalla comunità Amara.org "), "");
        assert_eq!(clean("Grazie per la visione!"), "");
        assert_eq!(clean("[Musica]"), "");
        assert_eq!(clean("(applausi) [MUSIC]"), "");
        assert_eq!(clean("Grazie per la visione del documento"), "Grazie per la visione del documento");
    }

    #[test]
    fn wav_e_multipart() {
        let w = wav(&[0.0, 1.0, -1.0]);
        assert_eq!(&w[..4], b"RIFF");
        assert_eq!(w.len(), 44 + 6);
        assert_eq!(u32::from_le_bytes(w[24..28].try_into().unwrap()), RATE);
        assert_eq!(i16::from_le_bytes([w[46], w[47]]), 32767);
        let m = multipart("BB", &[("model", "voxtral-mini-latest".into())], "audio.wav", b"DATA");
        let s = String::from_utf8_lossy(&m);
        assert!(s.starts_with("--BB\r\nContent-Disposition: form-data; name=\"model\"\r\n\r\nvoxtral-mini-latest\r\n"));
        assert!(s.contains("name=\"file\"; filename=\"audio.wav\"\r\nContent-Type: audio/wav\r\n\r\nDATA\r\n--BB--\r\n"));
    }

    /// Il servizio in rete: un server finto che risponde come OpenAI/Mistral.
    #[test]
    fn trascrizione_in_rete() {
        use std::io::{Read, Write};
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let srv = std::thread::spawn(move || {
            let (mut c, _) = l.accept().unwrap();
            c.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut req = Vec::new();
            let mut buf = [0u8; 65536];
            // intestazioni + corpo (Content-Length)
            loop {
                let n = c.read(&mut buf).unwrap();
                req.extend_from_slice(&buf[..n]);
                let txt = String::from_utf8_lossy(&req).to_string();
                if let Some(h) = txt.find("\r\n\r\n") {
                    let len: usize = txt.lines().find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length: ").map(|v| v.trim().parse().unwrap())).unwrap_or(0);
                    if req.len() >= h + 4 + len {
                        break;
                    }
                }
                if n == 0 {
                    break;
                }
            }
            let body = r#"{"text":" Mi chiamo Michele e abito a Barletta. "}"#;
            write!(c, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", body.len()).unwrap();
            String::from_utf8_lossy(&req).to_string()
        });
        let p: crate::ai::Provider = toml::from_str(&format!("name = \"Mistral\"\nurl = \"http://127.0.0.1:{port}/v1/\"\nkey = \"\"")).unwrap();
        let text = remote(&p, "voxtral-mini-latest", &tone(1.0, 0.3), Some("it")).unwrap();
        assert_eq!(clean(&text), "Mi chiamo Michele e abito a Barletta.");
        let req = srv.join().unwrap();
        assert!(req.starts_with("POST /v1/audio/transcriptions HTTP/1.1"), "{}", &req[..80]);
        assert!(req.contains("multipart/form-data; boundary=openslate-"));
        assert!(req.contains("name=\"model\"\r\n\r\nvoxtral-mini-latest") && req.contains("name=\"language\"\r\n\r\nit"));
        assert!(req.contains("filename=\"audio.wav\"") && req.contains("RIFF"));
        assert!(!req.to_ascii_lowercase().contains("authorization"), "senza chiave niente Authorization");
    }

    #[test]
    fn motore_dalla_configurazione() {
        let cfg: crate::ai::AiConfig = toml::from_str("[[provider]]\nname = \"Groq\"\nurl = \"https://api.groq.com/openai/v1\"\nenabled = false\n").unwrap();
        // di serie: Whisper large-v3-turbo
        assert!(matches!(engine(&cfg).unwrap(), Engine::Whisper(m) if m.name == "large-v3-turbo"));
        let mut c = cfg.clone();
        c.voice.engine = "Groq".into();
        // un agente senza il suo modello di trascrizione: lo dice
        assert!(engine(&c).is_err());
        c.voice.model = "whisper-large-v3-turbo".into();
        assert!(matches!(engine(&c).unwrap(), Engine::Remote(p, m) if p.name == "Groq" && m == "whisper-large-v3-turbo"));
        c.voice.engine = "Nessuno".into();
        assert!(engine(&c).is_err());
    }

    /// Un server che manda `body` a ogni richiesta (una alla volta).
    fn serve(body: Vec<u8>, times: usize) -> u16 {
        use std::io::{Read, Write};
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for _ in 0..times {
                let (mut c, _) = l.accept().unwrap();
                let mut buf = [0u8; 4096];
                let _ = c.read(&mut buf);
                let _ = write!(c, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", body.len());
                let _ = c.write_all(&body);
            }
        });
        port
    }

    /// Scaricare: si tiene solo il file con l'impronta giusta, mai uno a metà.
    #[test]
    fn scaricamento_del_modello() {
        use sha1::Digest;
        let dir = std::env::temp_dir().join(format!("openslate-voce-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let at = |m: &Model| dir.join(model_path(m));
        let body: Vec<u8> = (0..3_000_000u32).map(|i| (i % 251) as u8).collect();
        let sha: String = sha1::Sha1::digest(&body).iter().map(|b| format!("{b:02x}")).collect();
        let good = Model { name: "prova", file: "ggml-prova.bin", mb: 3, sha1: Box::leak(sha.into_boxed_str()) };
        let port = serve(body.clone(), 2);
        let url = format!("http://127.0.0.1:{port}/ggml-prova.bin");
        let mut seen = 0;
        download_to(&url, &good, &at(&good), &AtomicBool::new(false), |d, t| {
            assert!(d <= t);
            seen += 1;
        })
        .unwrap();
        assert_eq!(std::fs::read(at(&good)).unwrap(), body);
        assert!(seen >= 2);
        // impronta sbagliata: niente file, né intero né a metà
        let bad = Model { name: "prova2", file: "ggml-prova2.bin", mb: 3, sha1: "0000000000000000000000000000000000000000" };
        let e = download_to(&url, &bad, &at(&bad), &AtomicBool::new(false), |_, _| {}).unwrap_err();
        assert!(e.to_string().contains("diverso"), "{e}");
        assert!(!at(&bad).exists() && !at(&bad).with_extension("bin.part").exists());
        // annullato
        let port = serve(body, 1);
        let e = download_to(&format!("http://127.0.0.1:{port}/x"), &good, &dir.join("annullato.bin"), &AtomicBool::new(true), |_, _| {}).unwrap_err();
        assert!(e.to_string().contains("annullato"), "{e}");
        // un modello rotto: Whisper lo rifiuta con un messaggio, l'app non cade
        let broken = dir.join("rotto.bin");
        std::fs::write(&broken, b"non sono un modello").unwrap();
        let e = local::transcribe(&broken, &tone(1.0, 0.3), Some("it")).unwrap_err();
        assert!(e.to_string().contains("non carica il modello"), "{e}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn modelli_e_lingua() {
        assert_eq!(model("").name, "large-v3-turbo");
        assert_eq!(model("small").file, "ggml-small-q5_1.bin");
        assert_eq!(model("ggml-base-q5_1.bin").name, "base");
        assert_eq!(model("boh").name, "large-v3-turbo");
        assert!(model_path(model("base")).ends_with("models/whisper/ggml-base-q5_1.bin"));
        assert_eq!(language("auto"), None);
        assert_eq!(language("EN").as_deref(), Some("en"));
        assert!(language("").is_some());
    }
}
