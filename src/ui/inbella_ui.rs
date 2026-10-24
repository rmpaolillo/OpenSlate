//! Il pulsante «Trasforma in bella»: l'AI legge la pagina in un thread;
//! quando risponde, le righe lette diventano la loro bella copia sulla
//! lavagna (un passo solo da annullare). Vedi `crate::inbella`.

use super::App;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

type Done = anyhow::Result<HashMap<u64, String>>;

pub(super) struct InBellaUi {
    busy: bool,
    /// La risposta arrivata mentre la penna scriveva: si applica appena si alza.
    ready: Option<HashMap<u64, String>>,
    tx: Sender<Done>,
    rx: Receiver<Done>,
}

impl InBellaUi {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self { busy: false, ready: None, tx, rx }
    }
}

impl App {
    /// «Trasforma in bella»: tutta la pagina della lavagna.
    pub(super) fn inbella_now(&mut self) {
        if self.docui.doc.is_some() || self.docui.capture.is_some() {
            self.status = "«Trasforma in bella» è per la lavagna, non per i documenti.".into();
            return;
        }
        if self.inbella.busy {
            self.status = "Sto già trasformando la pagina…".into();
            return;
        }
        let (strokes, w, h, view) = {
            let s = self.hub.lock();
            if s.bg.as_ref().is_some_and(|b| !b.is_chart()) {
                self.status = "«Trasforma in bella» è per la lavagna, non per i documenti.".into();
                return;
            }
            (s.drawing.strokes.clone(), s.drawing.width, s.drawing.height, s.view)
        };
        let d = crate::stroke::Drawing { strokes: strokes.clone(), width: w, height: h };
        let (pw, ph) = crate::scene::page_dims(&d);
        let n = crate::inbella::pending(&strokes, &crate::scene::PageGeom::new(view, pw, ph)).len();
        if n == 0 {
            self.status = "Non c'è niente da trasformare: la pagina è vuota o è già in bella.".into();
            return;
        }
        self.inbella.busy = true;
        self.status = crate::i18n::tf(format!("Leggo {n} righe per trasformarle in bella…"));
        let tx = self.inbella.tx.clone();
        let _ = std::thread::Builder::new().name("in-bella".into()).spawn(move || {
            let _ = tx.send(crate::inbella::recognize(&strokes, w, h, view));
        });
    }

    /// A ogni giro: la risposta dell'AI, che diventa la pagina in bella.
    pub(super) fn inbella_tick(&mut self, ctx: &eframe::egui::Context) {
        if !self.inbella.busy {
            return;
        }
        if self.inbella.ready.is_none() {
            match self.inbella.rx.try_recv() {
                Ok(Ok(texts)) => self.inbella.ready = Some(texts),
                Ok(Err(e)) => {
                    self.inbella.busy = false;
                    tracing::warn!("trasforma in bella: {e:#}");
                    self.status = format!("⚠ {}", crate::i18n::tf(format!("trasforma in bella: {e:#}")));
                    ctx.request_repaint();
                    return;
                }
                Err(_) => {}
            }
        }
        // a penna appoggiata non si tocca la pagina: si aspetta che si alzi
        let pen_down = {
            let s = self.hub.lock();
            s.active_index().is_some() || s.mouse_highlighting()
        };
        if self.inbella.ready.is_none() || pen_down {
            ctx.request_repaint_after(Duration::from_millis(300));
            return;
        }
        let texts = self.inbella.ready.take().unwrap_or_default();
        self.inbella.busy = false;
        let done = self.hub.edit(|s| {
            // non sopra un documento aperto nel frattempo
            if s.bg.as_ref().is_some_and(|b| !b.is_chart()) {
                return 0;
            }
            let (pw, ph) = crate::scene::page_dims(&s.drawing);
            let g = crate::scene::PageGeom::new(s.view, pw, ph);
            let Some((strokes, n)) = crate::inbella::apply(&s.drawing.strokes, &texts, &g) else { return 0 };
            let bg = s.bg.clone();
            s.replace_page(strokes, bg);
            n
        });
        self.status = if done == 0 {
            "Non ho trovato righe da trasformare (disegni o scrittura che non si legge).".into()
        } else {
            crate::i18n::tf(format!("{done} righe trasformate in bella: Annulla riporta la tua scrittura."))
        };
        ctx.request_repaint();
    }
}
