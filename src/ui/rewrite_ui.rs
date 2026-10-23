//! La riscrittura in bella sulla lavagna: col pulsante (tutta la pagina) o
//! da sola quando smetti di scrivere (`rewrite_live`). L'AI legge le righe in
//! un thread; quando risponde, la pagina si ridisegna con le righe in bella.

use super::App;
use crate::stroke::strokes_sig;
use std::collections::HashSet;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

/// Quanto aspettare, a penna ferma, prima di leggere le righe nuove.
const PAUSE: Duration = Duration::from_millis(2500);

type Done = anyhow::Result<Vec<(u64, String)>>;

pub(super) struct RewriteUi {
    busy: bool,
    /// Lanciata dal pulsante (allora si dice com'è andata).
    manual: bool,
    tx: Sender<Done>,
    rx: Receiver<Done>,
    last_sig: u64,
    changed: Instant,
    /// Righe già chieste (se l'AI non le nomina, alla pausa non si richiede
    /// subito; il pulsante le richiede).
    asked: HashSet<u64>,
    /// Questa pausa ha già fatto partire la lettura.
    fired: bool,
}

impl RewriteUi {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self { busy: false, manual: false, tx, rx, last_sig: 0, changed: Instant::now(), asked: HashSet::new(), fired: true }
    }
}

impl App {
    /// «Riscrivi in bella»: le righe non ancora lette (`all`: tutte di nuovo).
    pub(super) fn rewrite_now(&mut self, all: bool) {
        if self.docui.doc.is_some() || self.docui.capture.is_some() {
            self.status = "La riscrittura in bella è per la lavagna, non per i documenti.".into();
            return;
        }
        if !self.settings.rewrite_show {
            self.settings.rewrite_show = true;
            self.settings.save();
            crate::rewrite::set_showing(true);
            self.rewrite_refresh();
        }
        if !self.rewrite_start(all, true) && !self.rw.busy {
            self.status = "Tutte le righe sono già in bella (tieni premuto O per vedere la tua scrittura).".into();
        }
    }

    /// Fa partire la lettura; `false` se non c'è niente da leggere.
    fn rewrite_start(&mut self, all: bool, manual: bool) -> bool {
        if self.rw.busy {
            return false;
        }
        let (strokes, w, h, view) = {
            let s = self.hub.lock();
            if s.bg.as_ref().is_some_and(|b| !b.is_chart()) {
                return false;
            }
            (s.drawing.strokes.clone(), s.drawing.width, s.drawing.height, s.view)
        };
        if strokes.is_empty() {
            return false;
        }
        let d = crate::stroke::Drawing { strokes: strokes.clone(), width: w, height: h };
        let (pw, ph) = crate::scene::page_dims(&d);
        let g = crate::scene::PageGeom::new(view, pw, ph);
        let (_, mut lines) = crate::rewrite::pending(&strokes, &g, all);
        if !manual {
            lines.retain(|(_, k)| !self.rw.asked.contains(k));
        }
        if lines.is_empty() {
            return false;
        }
        self.rw.asked.extend(lines.iter().map(|(_, k)| *k));
        self.rw.busy = true;
        self.rw.manual = manual;
        if manual {
            self.status = crate::i18n::tf(format!("Leggo {} righe per riscriverle in bella…", lines.len()));
        }
        let tx = self.rw.tx.clone();
        let _ = std::thread::Builder::new().name("riscrittura".into()).spawn(move || {
            let r = crate::rewrite::recognize(&strokes, w, h, view, &lines);
            let _ = tx.send(r);
        });
        true
    }

    /// La pagina e le anteprime si ridisegnano.
    pub(super) fn rewrite_refresh(&mut self) {
        self.nice.reset();
        self.lui.viewer_nice = None;
        self.lui.thumbs.clear();
    }

    /// A ogni giro: le risposte, e la pausa che fa partire la lettura.
    pub(super) fn rewrite_tick(&mut self, ctx: &eframe::egui::Context) {
        while let Ok(r) = self.rw.rx.try_recv() {
            self.rw.busy = false;
            match r {
                Ok(lines) => {
                    let n = lines.iter().filter(|(_, t)| !t.trim().is_empty()).count();
                    crate::rewrite::store(&lines);
                    self.rewrite_refresh();
                    if self.rw.manual {
                        self.status = if n == 0 {
                            "Non ho trovato righe da riscrivere (disegni o scrittura che non si legge).".into()
                        } else {
                            crate::i18n::tf(format!("{n} righe riscritte in bella: tieni premuto O per vedere la tua scrittura."))
                        };
                    }
                }
                Err(e) => {
                    tracing::warn!("riscrittura: {e:#}");
                    if self.rw.manual {
                        self.status = format!("⚠ {}", crate::i18n::tf(format!("riscrittura in bella: {e:#}")));
                    }
                }
            }
            ctx.request_repaint();
        }
        if self.rw.busy {
            ctx.request_repaint_after(Duration::from_millis(300));
        }
        if !self.settings.rewrite_live || !self.settings.rewrite_show || self.docui.doc.is_some() || self.docui.capture.is_some() {
            return;
        }
        let (sig, pen_down) = {
            let s = self.hub.lock();
            (strokes_sig(&s.drawing.strokes), s.active_index().is_some())
        };
        if sig != self.rw.last_sig {
            // all'avvio (prima firma) non si legge niente: solo dopo aver scritto
            let first = self.rw.last_sig == 0;
            self.rw.last_sig = sig;
            self.rw.changed = Instant::now();
            self.rw.fired = first;
            return;
        }
        if pen_down || self.rw.busy || self.rw.fired {
            return;
        }
        let since = self.rw.changed.elapsed();
        if since < PAUSE {
            ctx.request_repaint_after(PAUSE - since + Duration::from_millis(30));
            return;
        }
        // una volta per pausa: dopo, finché non si scrive ancora, niente
        self.rw.fired = true;
        self.rewrite_start(false, false);
    }
}
