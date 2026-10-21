//! Uscire senza perdere niente.
//!
//! La lavagna vive solo in memoria (in diretta il pad non conserva quello che
//! scrivi): se c'è scritto qualcosa che non è nell'archivio, alla chiusura si
//! chiede «Salva ed esci / Esci senza salvare / Non uscire», come in Word.
//! Se l'app si chiude senza poter chiedere (Esci dal Dock, spegnimento del
//! Mac, riavvio per l'aggiornamento) la lavagna si salva da sola
//! nell'archivio. L'appunto aperto si salva prima di chiudere.

use super::theme::Tokens;
use super::App;
use crate::i18n;
use crate::library::{self, Meta};
use crate::scene::PageBg;
use crate::stroke::{Drawing, Stroke};
use eframe::egui;
use std::time::{Duration, Instant};

/// Stato della chiusura.
#[derive(Default)]
pub(super) struct Quit {
    /// Firma della lavagna com'era all'ultimo salvataggio (o caricamento
    /// dall'archivio): se è uguale non c'è niente da salvare.
    pub saved_sig: u64,
    /// La finestra «Salvare la lavagna?» è aperta.
    pub asking: bool,
    /// L'utente ha deciso: si chiude davvero.
    pub go: bool,
    /// «Esci senza salvare»: all'uscita non si salva da soli.
    pub discard: bool,
    /// Chiusura rimandata finché l'appunto aperto non dice di essersi
    /// salvato (al massimo fino a quest'ora).
    pub close_at: Option<Instant>,
    /// L'appunto aperto si è salvato: si può chiudere.
    pub flushed: bool,
}

/// Firma dei tratti (numero, punti, colori) e del grafico dell'AI: cambia
/// se cambia il foglio.
pub(super) fn sig(strokes: &[Stroke], chart: Option<&PageBg>) -> u64 {
    let mut h: u64 = 1469598103934665603;
    let mut mix = |v: u64| h = (h ^ v).wrapping_mul(1099511628211);
    if let Some(c) = chart {
        mix(c.png.len() as u64);
        for v in c.rect {
            mix(v.to_bits() as u64);
        }
    }
    for s in strokes.iter().filter(|s| !s.points.is_empty()) {
        mix(s.points.len() as u64);
        mix(u32::from_le_bytes([s.color[0], s.color[1], s.color[2], s.highlight as u8]) as u64);
        for p in [s.points.first(), s.points.get(s.points.len() / 2), s.points.last()].into_iter().flatten() {
            mix(p.x.to_bits() as u64);
            mix(p.y.to_bits() as u64);
        }
    }
    h
}

impl App {
    /// I tratti della lavagna (anche se in questo momento al centro c'è un
    /// documento: allora la lavagna è da parte).
    /// I tratti della lavagna e il grafico dell'AI che c'è sopra.
    pub(super) fn board_drawing(&self) -> (Drawing, Option<PageBg>) {
        let s = self.hub.lock();
        // mentre si registra una firma la lavagna è da parte
        if self.docui.doc.is_none() {
            if let Some((strokes, bg)) = &self.docui.capture {
                return (Drawing { strokes: strokes.clone(), width: s.drawing.width, height: s.drawing.height }, bg.clone().filter(|b| b.is_chart()));
            }
        }
        match (&self.docui.doc, self.docui.board_stash_ref()) {
            (Some(_), Some((stash, chart))) => {
                (Drawing { strokes: stash.clone(), width: s.drawing.width, height: s.drawing.height }, chart.clone())
            }
            (Some(_), None) => (Drawing { strokes: Vec::new(), width: 0.0, height: 0.0 }, None),
            (None, _) => (s.drawing.clone(), s.bg.clone().filter(|b| b.is_chart())),
        }
    }

    /// C'è qualcosa sulla lavagna che non è nell'archivio?
    pub(super) fn board_unsaved(&self) -> bool {
        let (d, chart) = self.board_drawing();
        (!d.is_empty() || chart.is_some()) && sig(&d.strokes, chart.as_ref()) != self.quit.saved_sig
    }

    /// La lavagna così com'è adesso conta come salvata.
    pub(super) fn mark_board_saved(&mut self) {
        let (d, chart) = self.board_drawing();
        self.quit.saved_sig = sig(&d.strokes, chart.as_ref());
    }

    /// Mette la lavagna nell'archivio. Ritorna l'id della pagina.
    pub(super) fn save_board_to_library(&mut self) -> Result<String, String> {
        let (d, chart) = self.board_drawing();
        if d.is_empty() && chart.is_none() {
            return Err("La lavagna è vuota.".into());
        }
        let meta = Meta::new(chrono::Local::now().naive_local(), library::SOURCE_BOARD);
        let id = self.lib.add_with_chart(&d, meta, chart.as_ref()).map_err(|e| format!("⚠ archivio: {e}"))?;
        self.quit.saved_sig = sig(&d.strokes, chart.as_ref());
        Ok(id)
    }

    /// A ogni fotogramma: la richiesta di chiudere la finestra (pulsante
    /// rosso, Cmd+Q, File › Esci) passa da qui.
    pub(super) fn guard_close(&mut self, ctx: &egui::Context) {
        if let Some(t) = self.quit.close_at {
            if self.quit.flushed || Instant::now() >= t {
                // salvato (o la pagina non risponde: si chiude lo stesso)
                self.quit.close_at = None;
                self.quit.flushed = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                ctx.request_repaint_after(Duration::from_millis(50));
            }
        }
        if !ctx.input(|i| i.viewport().close_requested()) {
            return;
        }
        // si sta già aspettando l'appunto
        if self.quit.close_at.is_some() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            return;
        }
        if !self.quit.go && self.board_unsaved() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.quit.asking = true;
            // la finestra potrebbe essere nascosta dietro le altre
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            return;
        }
        self.quit.go = true;
        if self.note.is_some() && !self.quit.flushed {
            // l'appunto aperto si salva, poi si chiude (su ogni strada: anche
            // dopo «Salva ed esci» o «Esci senza salvare»)
            self.embeds.eval(super::embed::Slot::Note, "window.wsNote && wsNote.flush()");
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.quit.close_at = Some(Instant::now() + Duration::from_millis(1500));
        }
    }

    /// La pagina dell'appunto ha finito di salvare.
    pub(super) fn note_flushed(&mut self) {
        if self.quit.close_at.is_some() {
            self.quit.flushed = true;
        }
    }

    /// «Salvare la lavagna prima di uscire?»
    pub(super) fn quit_window(&mut self, ctx: &egui::Context) {
        if !self.quit.asking {
            return;
        }
        let t = Tokens::get(ctx);
        let mut choice: Option<u8> = None;
        egui::Modal::new(egui::Id::new("quit_ask")).show(ctx, |ui| {
            ui.set_width(420.0);
            ui.add_space(4.0);
            ui.label(egui::RichText::new(i18n::t("Salvare la lavagna prima di uscire?")).size(17.0).strong());
            ui.add_space(6.0);
            ui.label(i18n::t(
                "Quello che hai scritto sulla lavagna non è nell'archivio, e in diretta il pad non lo conserva: se esci senza salvarlo lo perdi.",
            ));
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                let save = egui::Button::new(egui::RichText::new(i18n::t("Salva ed esci")).color(t.on_accent)).fill(t.accent);
                if ui.add(save).clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    choice = Some(1);
                }
                if ui.button(i18n::t("Esci senza salvare")).clicked() {
                    choice = Some(2);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(i18n::t("Non uscire")).clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        choice = Some(0);
                    }
                });
            });
        });
        match choice {
            Some(1) => match self.save_board_to_library() {
                Ok(_) => self.quit_now(ctx, false),
                Err(e) => {
                    self.status = e;
                    self.quit.asking = false;
                }
            },
            Some(2) => self.quit_now(ctx, true),
            Some(_) => self.quit.asking = false,
            None => {}
        }
    }

    fn quit_now(&mut self, ctx: &egui::Context, discard: bool) {
        self.quit.asking = false;
        self.quit.go = true;
        self.quit.discard = discard;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    /// All'uscita senza aver potuto chiedere: la lavagna va nell'archivio da sola.
    pub(super) fn save_board_on_exit(&mut self) {
        if !self.quit.discard && self.board_unsaved() {
            match self.save_board_to_library() {
                Ok(id) => tracing::info!("lavagna salvata all'uscita: {id}"),
                Err(e) => tracing::warn!("lavagna all'uscita: {e}"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stroke::{PenEvent, PenPhase};

    #[test]
    fn firma_della_lavagna() {
        let tratto = |x: f32| {
            let mut s = Stroke::new([0, 0, 0], 2.0);
            for k in 0..5 {
                s.points.push(PenEvent { x: x + k as f32, y: 10.0, pressure: 0.5, phase: PenPhase::Move });
            }
            s
        };
        let a = vec![tratto(0.0)];
        let b = vec![tratto(0.0), tratto(50.0)];
        assert_eq!(sig(&a, None), sig(&a.clone(), None));
        assert_ne!(sig(&a, None), sig(&b, None), "un tratto in più = da salvare");
        assert_ne!(sig(&a, None), sig(&[tratto(1.0)], None), "un tratto diverso = da salvare");
        let g = PageBg { png: std::sync::Arc::new(vec![1, 2, 3]), rect: [0.1, 0.1, 0.5, 0.5], key: "chart:1".into() };
        assert_ne!(sig(&a, None), sig(&a, Some(&g)), "un grafico in più = da salvare");
        // i tratti vuoti non contano
        let mut c = a.clone();
        c.push(Stroke::new([0, 0, 0], 2.0));
        assert_eq!(sig(&a, None), sig(&c, None));
    }
}
