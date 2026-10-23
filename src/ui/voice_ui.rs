//! Il microfono della chat: la pagina chiede di registrare (`{cmd: "voice"}`),
//! l'app registra, trascrive in un thread e rimanda il testo alla pagina
//! (`wsChat.voice({...})`). La prima volta, se serve Whisper sul computer,
//! chiede il permesso di scaricarne il modello.
//!
//! Qui passa anche la voce che risponde (`{cmd: "speak"}`): il tutor legge
//! una risposta con `crate::speech`, in un thread; una sola voce alla volta.

use super::embed::Slot;
use super::App;
use crate::voice;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

enum Ev {
    Text(anyhow::Result<String>),
    Progress(u64, u64),
    Downloaded(anyhow::Result<()>),
    /// La voce ha finito (o si è fermata): l'id della risposta.
    Spoken(String, anyhow::Result<()>),
}

pub(super) struct VoiceUi {
    rec: Option<voice::Recorder>,
    busy: bool,
    download: Option<Arc<AtomicBool>>,
    tx: Sender<Ev>,
    rx: Receiver<Ev>,
    level_sent: Instant,
    /// La risposta che si sta leggendo.
    speaking: Option<String>,
}

impl VoiceUi {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self { rec: None, busy: false, download: None, tx, rx, level_sent: Instant::now(), speaking: None }
    }

    fn active(&self) -> bool {
        self.rec.is_some() || self.busy || self.download.is_some()
    }
}

impl App {
    fn voice_js(&self, ev: serde_json::Value) {
        self.embeds.eval(Slot::Chat, &format!("window.wsChat && wsChat.voice && wsChat.voice({ev})"));
    }

    fn voice_error(&self, e: &str) {
        tracing::warn!("voce: {e}");
        self.voice_js(serde_json::json!({ "state": "idle", "error": e }));
    }

    /// Un comando dal pulsante del microfono o dal tasto tenuto premuto.
    pub(super) fn on_voice(&mut self, action: &str) {
        match action {
            "start" => self.voice_start(),
            "stop" => self.voice_stop(),
            "download" => self.voice_download(),
            "cancel" => {
                self.voice.rec = None;
                if let Some(c) = self.voice.download.take() {
                    c.store(true, Ordering::SeqCst);
                }
                if !self.voice.busy {
                    self.voice_js(serde_json::json!({ "state": "idle" }));
                }
            }
            _ => {}
        }
    }

    /// Leggere una risposta (`auto`: alla fine di una risposta; legge solo
    /// se `[speech] auto` lo vuole), o fermare la voce (`text` vuoto).
    pub(super) fn on_speak(&mut self, id: &str, text: &str, auto: bool, dictated: bool) {
        if text.trim().is_empty() {
            crate::speech::stop();
            return;
        }
        if auto && !crate::ai::AiConfig::load().map(|c| c.speech.reads_by_itself(dictated)).unwrap_or(false) {
            return;
        }
        if self.voice.rec.is_some() {
            return; // si sta dettando: niente voce sopra il microfono
        }
        crate::speech::stop();
        if let Some(old) = self.voice.speaking.take() {
            self.voice_js(serde_json::json!({ "speak": old, "speaking": false }));
        }
        self.voice.speaking = Some(id.to_string());
        self.voice_js(serde_json::json!({ "speak": id, "speaking": true }));
        let (tx, id, text) = (self.voice.tx.clone(), id.to_string(), text.to_string());
        let _ = std::thread::Builder::new().name("voce-risponde".into()).spawn(move || {
            let r = crate::speech::speak(&text);
            let _ = tx.send(Ev::Spoken(id, r));
        });
    }

    fn voice_start(&mut self) {
        if self.voice.active() {
            return;
        }
        // si detta: la voce che legge tace
        crate::speech::stop();
        if let Some(m) = voice::model_missing() {
            self.voice_offer_download(m);
            return;
        }
        match voice::Recorder::start() {
            Ok(r) => {
                self.voice.rec = Some(r);
                self.voice_js(serde_json::json!({ "state": "rec" }));
            }
            Err(e) => self.voice_error(&format!("{e:#}")),
        }
    }

    fn voice_stop(&mut self) {
        let Some(rec) = self.voice.rec.take() else { return };
        self.voice.busy = true;
        self.voice_js(serde_json::json!({ "state": "busy" }));
        let tx = self.voice.tx.clone();
        let _ = std::thread::Builder::new().name("voce".into()).spawn(move || {
            let r = rec.finish().and_then(|pcm| voice::transcribe(&pcm));
            let _ = tx.send(Ev::Text(r));
        });
    }

    /// La prima volta: la chat chiede se scaricare il modello di Whisper
    /// (dentro la pagina: una finestra di sistema qui bloccherebbe GTK su Linux).
    fn voice_offer_download(&mut self, m: &'static voice::Model) {
        self.voice_js(serde_json::json!({ "state": "idle", "offer": { "name": m.name, "mb": m.mb } }));
    }

    /// L'utente ha detto sì: si scarica.
    fn voice_download(&mut self) {
        if self.voice.active() {
            return;
        }
        let Some(m) = voice::model_missing() else { return };
        let cancel = Arc::new(AtomicBool::new(false));
        self.voice.download = Some(cancel.clone());
        self.voice_js(serde_json::json!({ "state": "download", "done": 0, "total": m.mb as u64 * 1024 * 1024 }));
        let tx = self.voice.tx.clone();
        let _ = std::thread::Builder::new().name("whisper-scarica".into()).spawn(move || {
            let t2 = tx.clone();
            let r = voice::download(m, &cancel, move |d, t| {
                let _ = t2.send(Ev::Progress(d, t));
            });
            let _ = tx.send(Ev::Downloaded(r));
        });
    }

    /// A ogni giro: i risultati dei thread e il volume del microfono.
    pub(super) fn voice_tick(&mut self, ctx: &eframe::egui::Context) {
        while let Ok(ev) = self.voice.rx.try_recv() {
            match ev {
                Ev::Text(r) => {
                    self.voice.busy = false;
                    match r {
                        Ok(text) => {
                            let send = crate::ai::AiConfig::load().map(|c| c.voice.auto_send).unwrap_or(false);
                            self.voice_js(serde_json::json!({ "state": "idle", "text": text, "send": send }));
                        }
                        Err(e) => self.voice_error(&format!("{e:#}")),
                    }
                }
                Ev::Progress(done, total) => {
                    if self.voice.download.is_some() {
                        self.voice_js(serde_json::json!({ "done": done, "total": total }));
                    }
                }
                Ev::Spoken(id, r) => {
                    if self.voice.speaking.as_deref() == Some(id.as_str()) {
                        self.voice.speaking = None;
                    }
                    match r {
                        Ok(()) => self.voice_js(serde_json::json!({ "speak": id, "speaking": false })),
                        Err(e) => {
                            tracing::warn!("voce che risponde: {e:#}");
                            self.voice_js(serde_json::json!({ "speak": id, "speaking": false, "error": format!("{e:#}") }));
                        }
                    }
                }
                Ev::Downloaded(r) => {
                    self.voice.download = None;
                    match r {
                        Ok(()) => self.voice_js(serde_json::json!({ "state": "idle", "info": "Whisper è pronto: premi il microfono e parla." })),
                        Err(e) if e.to_string().contains("annullato") => self.voice_js(serde_json::json!({ "state": "idle" })),
                        Err(e) => self.voice_error(&format!("{e:#}")),
                    }
                }
            }
        }
        if let Some(rec) = &self.voice.rec {
            if rec.ended() {
                // si è fermata da sola (5 minuti, microfono staccato): si trascrive quello che c'è
                self.voice_stop();
            } else if self.voice.level_sent.elapsed() >= Duration::from_millis(70) {
                self.voice.level_sent = Instant::now();
                self.voice_js(serde_json::json!({ "level": rec.level() }));
            }
        }
        if self.voice.active() {
            ctx.request_repaint_after(Duration::from_millis(60));
        }
    }
}
