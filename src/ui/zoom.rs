//! Zoom e spostamento della pagina (lavagna e visualizzatore dell'archivio),
//! come nei programmi per le immagini:
//! - rotella del mouse: zoom attorno al puntatore;
//! - trackpad: due dita = sposta, pizzico (o Cmd + due dita) = zoom;
//! - trackpad: due dita che ruotano = gira il foglio di 90° (come in Anteprima);
//! - trascinamento col mouse (tasto sinistro o centrale) = sposta;
//! - doppio clic: 200% nel punto cliccato, oppure torna ad «Adatta»;
//! - Cmd + / Cmd − / Cmd 0 e i pulsanti nella barra.

use eframe::egui;

pub(super) const ZOOM_MIN: f32 = 0.25;
pub(super) const ZOOM_MAX: f32 = 12.0;
/// Rotazione a due dita che gira il foglio (radianti, ~40°).
const ROTATE_TRIGGER: f32 = 0.7;
/// Passo dei pulsanti e dei tasti.
pub(super) const ZOOM_STEP: f32 = 1.25;

#[derive(Clone, Copy, Debug)]
pub(super) struct ZoomView {
    /// 1 = pagina intera nella finestra («Adatta»).
    pub zoom: f32,
    /// Spostamento del centro della pagina rispetto al centro dell'area (punti).
    pub pan: egui::Vec2,
    /// Area di disegno dell'ultimo fotogramma.
    rect: egui::Rect,
    /// Rotazione a due dita in corso: angolo accumulato, ultimo evento, e se
    /// questo gesto ha già girato il foglio (un gesto = un quarto di giro).
    rot_acc: f32,
    rot_last: f64,
    rot_done: bool,
}

impl Default for ZoomView {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: egui::Vec2::ZERO,
            rect: egui::Rect::NOTHING,
            rot_acc: 0.0,
            rot_last: 0.0,
            rot_done: false,
        }
    }
}

/// Comandi di zoom da pulsanti e tastiera.
#[derive(Clone, Copy, Debug)]
pub(super) enum ZoomOp {
    In,
    Out,
    Fit,
    /// Zoom preciso (cursore della barra di stato), attorno al centro.
    To(f32),
}

impl ZoomView {
    pub fn fit(&mut self) {
        self.zoom = 1.0;
        self.pan = egui::Vec2::ZERO;
    }

    pub fn is_fit(&self) -> bool {
        (self.zoom - 1.0).abs() < 0.01 && self.pan.length() < 0.5
    }

    pub fn apply(&mut self, op: ZoomOp) {
        match op {
            ZoomOp::In => self.zoom_at(ZOOM_STEP, None),
            ZoomOp::Out => self.zoom_at(1.0 / ZOOM_STEP, None),
            ZoomOp::Fit => self.fit(),
            ZoomOp::To(z) => {
                let z = z.clamp(ZOOM_MIN, ZOOM_MAX);
                if (z - 1.0).abs() < 0.02 {
                    self.fit();
                } else {
                    self.zoom_at(z / self.zoom, None);
                }
            }
        }
    }

    /// Ingrandisce di `factor` tenendo fermo il punto `anchor` (o il centro).
    pub fn zoom_at(&mut self, factor: f32, anchor: Option<egui::Pos2>) {
        let new = (self.zoom * factor).clamp(ZOOM_MIN, ZOOM_MAX);
        let f = new / self.zoom;
        if (f - 1.0).abs() < 1e-4 {
            return;
        }
        let center = if self.rect.is_positive() { self.rect.center() } else { egui::Pos2::ZERO };
        let p = anchor.unwrap_or(center);
        let c = center + self.pan;
        let c2 = p - (p - c) * f;
        self.pan = c2 - center;
        self.zoom = new;
        // Tornati alla pagina intera: ricentra.
        if (self.zoom - 1.0).abs() < 0.01 {
            self.fit();
        }
    }

    pub fn label(&self) -> String {
        format!("{:.0}%", self.zoom * 100.0)
    }

    /// Rettangolo della pagina, dato quello «adattato» (zoom 1, centrato).
    /// Limita lo spostamento perché un pezzo di pagina resti sempre visibile.
    pub fn place(&mut self, area: egui::Rect, fit_size: egui::Vec2) -> egui::Rect {
        self.rect = area;
        let size = fit_size * self.zoom;
        let max = (size + area.size()) * 0.5 - egui::vec2(60.0, 60.0);
        self.pan.x = self.pan.x.clamp(-max.x.max(0.0), max.x.max(0.0));
        self.pan.y = self.pan.y.clamp(-max.y.max(0.0), max.y.max(0.0));
        egui::Rect::from_center_size(area.center() + self.pan, size)
    }

    /// Mouse, rotella e trackpad sopra l'area della pagina.
    /// `primary`: il tasto sinistro serve a spostare e al doppio clic (falso
    /// quando è impegnato da altro, es. la gomma: si sposta col tasto centrale).
    /// Ritorna i quarti di giro chiesti ruotando due dita sul trackpad
    /// (+1 = senso orario, −1 = antiorario, 0 = niente).
    pub fn handle_input(&mut self, ui: &egui::Ui, resp: &egui::Response, primary: bool) -> i32 {
        let mut turn = 0;
        let now = ui.input(|i| i.time);
        // gesto finito (niente rotazione da un po'): si ricomincia
        if now - self.rot_last > 0.5 {
            self.rot_acc = 0.0;
            self.rot_done = false;
        }
        if (primary && resp.dragged_by(egui::PointerButton::Primary)) || resp.dragged_by(egui::PointerButton::Middle) {
            self.pan += resp.drag_delta();
        }
        if primary && resp.double_clicked() {
            if (self.zoom - 1.0).abs() < 0.01 {
                self.zoom_at(2.0, resp.interact_pointer_pos());
            } else {
                self.fit();
            }
        }
        if resp.hovered() || resp.dragged() {
            let hover = resp.hover_pos().or(resp.interact_pointer_pos());
            let events = ui.input(|i| i.events.clone());
            for e in events {
                match e {
                    egui::Event::MouseWheel { unit, delta, modifiers, .. } => match unit {
                        // rotella del mouse: zoom (Shift = sposta di lato)
                        egui::MouseWheelUnit::Line => {
                            if modifiers.shift {
                                self.pan.x += (delta.x + delta.y) * 40.0;
                            } else if delta.y != 0.0 {
                                self.zoom_at(1.2_f32.powf(delta.y), hover);
                            } else {
                                self.pan.x += delta.x * 40.0;
                            }
                        }
                        // trackpad: sposta; con Cmd/Ctrl fa zoom
                        egui::MouseWheelUnit::Point => {
                            if modifiers.command || modifiers.ctrl {
                                self.zoom_at((delta.y / 200.0).exp(), hover);
                            } else {
                                self.pan += delta;
                            }
                        }
                        egui::MouseWheelUnit::Page => {}
                    },
                    // pizzico sul trackpad
                    egui::Event::Zoom(f) => self.zoom_at(f, hover),
                    // due dita che ruotano (angolo positivo = senso orario)
                    egui::Event::Rotate(a) => {
                        self.rot_last = now;
                        if !self.rot_done {
                            self.rot_acc += a;
                            if self.rot_acc.abs() >= ROTATE_TRIGGER {
                                turn = if self.rot_acc > 0.0 { 1 } else { -1 };
                                self.rot_done = true;
                            }
                        }
                    }
                    _ => {}
                }
            }
            if !primary {
                // il puntatore lo disegna chi usa il tasto sinistro
            } else if resp.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            } else if self.zoom > 1.0 {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            }
        }
        turn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zv() -> ZoomView {
        let mut z = ZoomView::default();
        z.place(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0)), egui::vec2(400.0, 560.0));
        z
    }

    #[test]
    fn zoom_attorno_al_puntatore() {
        let mut z = zv();
        let p = egui::pos2(600.0, 150.0);
        // punto della pagina sotto il puntatore, in coordinate relative al centro pagina
        let before = (p - (z.rect.center() + z.pan)) / z.zoom;
        z.zoom_at(2.0, Some(p));
        let after = (p - (z.rect.center() + z.pan)) / z.zoom;
        assert!((before - after).length() < 1e-3);
        assert!((z.zoom - 2.0).abs() < 1e-6);
    }

    /// Fa girare `handle_input` in un egui senza finestra con questi eventi
    /// (il puntatore è già sopra il foglio).
    fn gesti(z: &mut ZoomView, frames: Vec<Vec<egui::Event>>) -> i32 {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
        let mut turns = 0;
        let mut all = vec![vec![egui::Event::PointerMoved(egui::pos2(400.0, 300.0))]];
        all.extend(frames);
        for (n, events) in all.into_iter().enumerate() {
            let raw = egui::RawInput { screen_rect: Some(screen), events, time: Some(n as f64 * 0.016), ..Default::default() };
            let mut out = ctx.run_ui(raw, |ui| {
                let rect = ui.max_rect();
                let resp = ui.interact(rect, egui::Id::new("foglio"), egui::Sense::click_and_drag());
                z.place(rect, egui::vec2(400.0, 560.0));
                turns += z.handle_input(ui, &resp, true);
            });
            out.textures_delta.clear();
        }
        turns
    }

    #[test]
    fn gesti_del_trackpad() {
        // pizzico: ingrandisce
        let mut z = ZoomView::default();
        gesti(&mut z, vec![vec![egui::Event::Zoom(1.5)], vec![egui::Event::Zoom(1.2)]]);
        assert!((z.zoom - 1.8).abs() < 1e-3, "{}", z.zoom);
        // due dita: sposta
        let pan = z.pan;
        let wheel = |d: egui::Vec2, m: egui::Modifiers| egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta: d, phase: egui::TouchPhase::Move, modifiers: m };
        gesti(&mut z, vec![vec![wheel(egui::vec2(0.0, -30.0), egui::Modifiers::NONE)]]);
        assert!((z.pan.y - (pan.y - 30.0)).abs() < 1e-3);
        // Cmd + due dita: zoom
        let before = z.zoom;
        gesti(&mut z, vec![vec![wheel(egui::vec2(0.0, 100.0), egui::Modifiers::COMMAND)]]);
        assert!(z.zoom > before);
        // rotazione: un solo quarto di giro per gesto, nel verso giusto
        let mut z = ZoomView::default();
        let rot = |a: f32| vec![egui::Event::Rotate(a)];
        assert_eq!(gesti(&mut z, vec![rot(0.3), rot(0.3), rot(0.3), rot(0.5), rot(0.5)]), 1);
        let mut z = ZoomView::default();
        assert_eq!(gesti(&mut z, vec![rot(-0.4), rot(-0.4)]), -1);
        // una rotazione piccola (mentre si pizzica) non gira il foglio
        let mut z = ZoomView::default();
        assert_eq!(gesti(&mut z, vec![rot(0.2), rot(0.2)]), 0);
    }

    #[test]
    fn limiti_e_adatta() {
        let mut z = zv();
        for _ in 0..50 {
            z.apply(ZoomOp::In);
        }
        assert_eq!(z.zoom, ZOOM_MAX);
        for _ in 0..80 {
            z.apply(ZoomOp::Out);
        }
        assert_eq!(z.zoom, ZOOM_MIN);
        z.apply(ZoomOp::Fit);
        assert!(z.is_fit());
        // tornando a 100% con i tasti la pagina si ricentra
        z.pan = egui::vec2(30.0, 0.0);
        z.zoom = 1.0 / ZOOM_STEP;
        z.apply(ZoomOp::In);
        assert!(z.is_fit());
    }
}
