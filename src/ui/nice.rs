//! La bella scrittura sulla lavagna, mentre si scrive.
//!
//! Riordinare tutta la pagina a ogni punto sarebbe inutile (e la riga
//! ballerebbe sotto la penna): si tiene la pagina già riordinata e il tratto
//! nuovo si mostra solo lisciato; quando la penna è alzata da un attimo, la
//! pagina si riordina di nuovo e la parola nuova «si assesta».
//!
//! I tratti della lavagna restano quelli veri: questo è solo come si vedono.

use crate::beautify;
use crate::scene::View;
use crate::stroke::{strokes_sig, Drawing};
use std::borrow::Cow;
use std::time::{Duration, Instant};

/// Quanto aspettare, a penna alzata, prima di riordinare.
const SETTLE: Duration = Duration::from_millis(600);

#[derive(Clone, Copy, PartialEq)]
struct Key {
    level: u8,
    view: View,
    w: u32,
    h: u32,
}

pub(super) struct NiceBoard {
    /// La pagina riordinata: per quali impostazioni, per quanti tratti e con
    /// che firma (dei tratti originali da cui viene).
    key: Option<Key>,
    n: usize,
    sig: u64,
    drawing: Drawing,
    /// L'ultima volta che il foglio è cambiato (firma di tutto il foglio).
    last_sig: u64,
    changed: Instant,
}

impl NiceBoard {
    pub fn new() -> Self {
        Self { key: None, n: 0, sig: 0, drawing: Drawing::default(), last_sig: 0, changed: Instant::now() }
    }

    /// Dimentica la pagina riordinata (cambio di livello).
    pub fn reset(&mut self) {
        self.key = None;
    }

    /// Il foglio da disegnare. `pen_down`: si sta scrivendo. Chiede un
    /// ridisegno quando la pagina deve ancora assestarsi.
    pub fn get<'a>(&'a mut self, d: &'a Drawing, view: View, pen_down: bool, ctx: &eframe::egui::Context) -> Cow<'a, Drawing> {
        let level = beautify::level();
        if !beautify::active() || d.strokes.is_empty() {
            return Cow::Borrowed(d);
        }
        let key = Key { level: beautify::mode(), view, w: d.width.to_bits(), h: d.height.to_bits() };
        let all = strokes_sig(&d.strokes);
        if all != self.last_sig {
            self.last_sig = all;
            self.changed = Instant::now();
        }
        let same = self.key == Some(key);
        if same && self.n == d.strokes.len() && self.sig == all {
            return Cow::Borrowed(&self.drawing);
        }
        // la pagina riordinata vale ancora per i tratti di prima? (si è solo
        // aggiunto qualcosa: niente gomma, annulla, pagina nuova)
        let prefix_ok = same && self.n <= d.strokes.len() && strokes_sig(&d.strokes[..self.n]) == self.sig;
        let settled = !pen_down && self.changed.elapsed() >= SETTLE;
        if !prefix_ok || settled {
            self.drawing = beautify::nice(d, view);
            self.key = Some(key);
            self.n = d.strokes.len();
            self.sig = all;
            return Cow::Borrowed(&self.drawing);
        }
        // si sta scrivendo: il già riordinato, più i tratti nuovi lisciati
        if !pen_down {
            ctx.request_repaint_after(SETTLE.saturating_sub(self.changed.elapsed()) + Duration::from_millis(20));
        }
        let mut strokes = self.drawing.strokes.clone();
        strokes.extend(d.strokes[self.n..].iter().map(|s| beautify::smooth_only(s, level)));
        Cow::Owned(Drawing { strokes, width: d.width, height: d.height })
    }
}
