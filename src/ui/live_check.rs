//! Il controllo mentre scrivi: quando la penna resta ferma per qualche
//! secondo (`live_check_pause`) dopo tratti nuovi, la chat chiede all'AI di
//! guardare solo le righe nuove. Se è tutto giusto l'AI risponde «OK» e la
//! chat lo mostra in una riga piccola; se c'è un errore lo segna sul foglio
//! (cerchio, sottolineatura, correzione a mano) e lo spiega in una riga.
//!
//! Qui si decide quando e su quali righe; il resto lo fa la chat
//! (`wsChat.liveCheck`), che risponde con `{cmd: "live_check", state}`.

use super::embed::Slot;
use super::App;
use crate::stroke::strokes_sig;
use std::time::{Duration, Instant};

pub(super) struct LiveCheck {
    /// I tratti già controllati (i primi `n`, con la loro firma).
    n: usize,
    sig: u64,
    /// L'ultima volta che il foglio è cambiato.
    last_sig: u64,
    changed: Instant,
    /// Una richiesta mandata alla chat, in attesa che la prenda: quanti
    /// tratti comprendeva e quando è partita.
    asked: Option<(usize, Instant)>,
    /// La chat sta controllando.
    running: bool,
    /// Il foglio di prima: se cambia (pagina nuova, Annulla, gomma) si riparte
    /// da quello che c'è, senza ricontrollarlo.
    started: bool,
}

impl LiveCheck {
    pub fn new() -> Self {
        Self { n: 0, sig: 0, last_sig: 0, changed: Instant::now(), asked: None, running: false, started: false }
    }
}

impl App {
    /// Acceso o spento dall'interruttore.
    pub(super) fn set_live_check(&mut self, on: bool) {
        self.settings.live_check = on;
        self.settings.save();
        // si parte da quello che c'è già: si controlla solo il nuovo
        self.live.started = false;
        self.live.asked = None;
        self.live.running = false;
        if on {
            self.toggle_chat(true);
            self.status = crate::i18n::tf(format!(
                "Controllo mentre scrivi acceso: dopo {} s senza scrivere l'AI guarda le righe nuove e segna gli errori sul foglio.",
                fmt_secs(self.settings.live_check_pause)
            ));
        } else {
            self.status = "Controllo mentre scrivi spento.".into();
        }
    }

    /// La risposta della chat.
    pub(super) fn on_live_check(&mut self, state: &str, n: usize) {
        match state {
            "started" => {
                // i tratti mandati sono controllati (anche se l'AI ci mette un po')
                if let Some((asked_n, _)) = self.live.asked.take() {
                    let n = n.min(asked_n);
                    let s = self.hub.lock();
                    let strokes = &s.drawing.strokes;
                    if strokes.len() >= n {
                        self.live.n = n;
                        self.live.sig = strokes_sig(&strokes[..n]);
                    }
                }
                self.live.running = true;
            }
            "done" => self.live.running = false,
            // la chat è occupata (stai chattando): si riprova dopo
            "busy" => {
                if let Some((n, _)) = self.live.asked {
                    self.live.asked = Some((n, Instant::now()));
                }
            }
            _ => {}
        }
    }

    /// A ogni giro: è ora di controllare?
    pub(super) fn live_check_tick(&mut self, ctx: &eframe::egui::Context) {
        if !self.settings.live_check || self.docui.doc.is_some() || self.docui.capture.is_some() {
            return;
        }
        let pause = Duration::from_secs_f32(self.settings.live_check_pause.clamp(1.5, 30.0));
        let (lines, n) = {
            let s = self.hub.lock();
            if s.bg.as_ref().is_some_and(|b| !b.is_chart()) {
                return;
            }
            let strokes = &s.drawing.strokes;
            let all = strokes_sig(strokes);
            if all != self.live.last_sig {
                self.live.last_sig = all;
                self.live.changed = Instant::now();
            }
            // la prima volta (o dopo l'interruttore): quello che c'è è già visto
            let prefix_ok = self.live.started && self.live.n <= strokes.len() && strokes_sig(&strokes[..self.live.n]) == self.live.sig;
            if !prefix_ok {
                self.live.started = true;
                self.live.n = strokes.len();
                self.live.sig = all;
                self.live.asked = None;
                return;
            }
            // i tratti nuovi del pennino (non quelli dell'AI, non gli evidenziatori)
            let fresh: Vec<usize> = (self.live.n..strokes.len())
                .filter(|&i| !strokes[i].highlight && !strokes[i].points.is_empty() && !crate::boardagent::is_ai_stroke(&strokes[i]))
                .collect();
            if fresh.is_empty() {
                if strokes.len() > self.live.n && !self.live.running {
                    self.live.n = strokes.len();
                    self.live.sig = all;
                }
                return;
            }
            if s.active_index().is_some() || self.live.running {
                return;
            }
            let since = self.live.changed.elapsed();
            if since < pause {
                ctx.request_repaint_after(pause - since + Duration::from_millis(30));
                return;
            }
            // già chiesto: si rimanda solo se la chat non l'ha preso (si apre, è occupata)
            if let Some((_, at)) = self.live.asked {
                if at.elapsed() < Duration::from_secs(3) {
                    ctx.request_repaint_after(Duration::from_secs(1));
                    return;
                }
            }
            // le righe dove sono i tratti nuovi, coi nomi di lavagna_leggi
            let reading = crate::boardread::read(strokes, &s.geom(), s.bg.as_ref());
            let lines: Vec<String> = reading
                .lines
                .iter()
                .filter(|l| l.group.strokes.iter().any(|i| fresh.contains(i)))
                .map(|l| l.group.id.clone())
                .collect();
            (lines, strokes.len())
        };
        if lines.is_empty() {
            // solo disegno o segni: niente da controllare
            let s = self.hub.lock();
            self.live.n = n;
            self.live.sig = strokes_sig(&s.drawing.strokes[..n.min(s.drawing.strokes.len())]);
            return;
        }
        if !self.chat_open {
            self.toggle_chat(true);
        }
        self.live.asked = Some((n, Instant::now()));
        if self.embeds.has(Slot::Chat) {
            let arg = serde_json::json!({ "lines": lines, "n": n });
            self.embeds.eval(Slot::Chat, &format!("window.wsChat && wsChat.liveCheck && wsChat.liveCheck({arg})"));
        }
        ctx.request_repaint_after(Duration::from_secs(1));
    }
}

fn fmt_secs(s: f32) -> String {
    if (s - s.round()).abs() < 0.05 {
        format!("{}", s.round() as i32)
    } else {
        format!("{s:.1}")
    }
}
