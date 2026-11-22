//! Opzioni › Telefono: collegare un telefono col QR (a richiesta, per un
//! minuto) e vedere o scollegare i telefoni abbinati. E la finestra «Dal
//! telefono»: la pagina del telefono in diretta (`sync::live`), da guardare.

use crate::i18n;
use crate::phonesync::PhoneSync;
use eframe::egui;
use egui_phosphor::regular as ic;
use std::time::{Duration, Instant};

/// Quanto resta visibile il QR.
const QR_TIME: Duration = Duration::from_secs(60);

/// Dopo quanto senza notizie la diretta si dice ferma (il telefono dà un
/// segno di vita ogni pochi secondi).
const LIVE_QUIET: i64 = 12;

pub(super) struct PhoneUi {
    pub sync: PhoneSync,
    /// Il QR mostrato: moduli, lato, quando chiuderlo, quanti telefoni c'erano.
    qr: Option<(Vec<bool>, usize, Instant, usize)>,
    msg: String,
    /// La finestra «Dal telefono» è aperta.
    mirror_open: bool,
    /// La diretta (generazione) per cui l'utente l'ha chiusa: non si riapre
    /// da sola finché il telefono non ricomincia.
    mirror_closed: u64,
    /// La pagina del documento sotto i tratti, per `bg_rev`.
    mirror_tex: Option<(u64, egui::TextureHandle)>,
    mirror_zoom: super::zoom::ZoomView,
    /// Il nome del telefono in diretta (id, nome), per non chiederlo al
    /// server a ogni fotogramma (durante un sync lungo aspetterebbe).
    mirror_who: Option<(String, Option<String>)>,
}

impl PhoneUi {
    pub fn new(sync: PhoneSync) -> Self {
        PhoneUi {
            sync,
            qr: None,
            msg: String::new(),
            mirror_open: false,
            mirror_closed: 0,
            mirror_tex: None,
            mirror_zoom: Default::default(),
            mirror_who: None,
        }
    }

    /// Il telefono ha mandato qualcosa: la finestra si apre da sola (se non
    /// la si è chiusa per questa diretta).
    pub fn live_changed(&mut self) {
        let m = self.sync.mirror.lock().unwrap_or_else(|e| e.into_inner());
        if !self.mirror_open && !m.ended && m.gen != self.mirror_closed {
            self.mirror_open = true;
            self.mirror_zoom.fit();
        }
    }

    /// La finestra «Dal telefono» (una finestra vera, da spostare anche su
    /// un altro schermo).
    pub fn mirror_window(&mut self, ctx: &egui::Context) {
        if !self.mirror_open {
            return;
        }
        // Si copia la pagina e si lascia subito il lucchetto: il server lo
        // prende tenendo già il suo (`Shared`), quindi qui, col lucchetto
        // della pagina in mano, non si deve chiedere niente al server
        // (`phones()`), altrimenti i due si aspettano a vicenda per sempre.
        let (peer, quiet, bg_rev, new_bg, rect, meta, strokes, gen) = {
            let m = self.sync.mirror.lock().unwrap_or_else(|e| e.into_inner());
            let fresh = self.mirror_tex.as_ref().map(|t| t.0) != Some(m.bg_rev);
            (
                m.peer.clone(),
                m.ended || crate::sync::now() - m.last > LIVE_QUIET,
                m.bg_rev,
                // l'immagine si copia solo quando è nuova
                fresh.then(|| m.bg.as_ref().map(|b| b.0.clone())),
                m.bg.as_ref().map(|b| b.1),
                m.meta.clone(),
                m.strokes.clone(),
                m.gen,
            )
        };
        if self.mirror_who.as_ref().map(|w| &w.0) != Some(&peer) {
            let name = self.sync.phones().into_iter().find(|p| p.id == peer).map(|p| p.name).filter(|n| !n.is_empty());
            self.mirror_who = Some((peer, name));
        }
        let name = self.mirror_who.as_ref().and_then(|w| w.1.clone());
        let who = name.unwrap_or_else(|| i18n::t("Telefono").to_string());
        // la texture dello sfondo, rifatta quando cambia
        if let Some(img) = new_bg {
            self.mirror_tex = img.and_then(|img| {
                let pm = tiny_skia::Pixmap::decode_png(&img).ok()?;
                let ci = egui::ColorImage::from_rgba_premultiplied([pm.width() as usize, pm.height() as usize], pm.data());
                Some((bg_rev, ctx.load_texture("phone-live-bg", ci, egui::TextureOptions::LINEAR)))
            });
        }
        let tex = self.mirror_tex.as_ref().filter(|t| t.0 == bg_rev).map(|t| t.1.id());
        let bg = tex.zip(rect);
        let drawing = crate::stroke::Drawing { strokes, width: meta.pad_w.max(1.0), height: meta.pad_h.max(1.0) };
        let view = crate::scene::View { rot: ((4 - meta.quarter % 4) % 4) as i32, flip_x: false, flip_y: false };
        let paper = (!meta.paper.is_empty()).then(|| (super::paper::Paper::from_code(&meta.paper), meta.margin));
        let title = if meta.title.is_empty() { who.clone() } else { format!("{who} — {}", meta.title) };
        let zoom = &mut self.mirror_zoom;
        let mut close = false;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("openslate-dal-telefono"),
            egui::ViewportBuilder::default().with_title(format!("{} — {title}", i18n::t("Dal telefono"))).with_inner_size([560.0, 760.0]).with_min_inner_size([240.0, 280.0]),
            |ui, _| {
                let t = super::theme::Tokens::get(ui.ctx());
                let full = ui.max_rect();
                let bar = 26.0;
                let (top, body) = (egui::Rect::from_min_max(full.min, egui::pos2(full.max.x, full.min.y + bar)), egui::Rect::from_min_max(egui::pos2(full.min.x, full.min.y + bar), full.max));
                ui.painter().rect_filled(top, 0.0, t.canvas);
                let state = if quiet { i18n::t("non è più in diretta") } else { i18n::t("in diretta") };
                let dot = if quiet { egui::Color32::GRAY } else { egui::Color32::from_rgb(220, 40, 40) };
                ui.painter().circle_filled(top.left_center() + egui::vec2(14.0, 0.0), 5.0, dot);
                ui.painter().text(
                    top.left_center() + egui::vec2(26.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    format!("{title} · {state}"),
                    egui::FontId::proportional(13.0),
                    t.text_dim,
                );
                let resp = ui.interact(body, ui.id().with("phone-live"), egui::Sense::click_and_drag());
                zoom.handle_input(ui, &resp, true);
                if resp.double_clicked() {
                    zoom.fit();
                }
                let bg = bg.map(|(tex, rect)| super::BgImage { tex, paper: false, rect, baked: false, keep: None, detail: None });
                super::paint_page(&ui.painter_at(body), body, &drawing, view, bg, zoom, paper);
                if ui.ctx().input(|i| i.viewport().close_requested()) {
                    close = true;
                }
            },
        );
        if close {
            self.mirror_open = false;
            self.mirror_closed = gen;
        }
        // la scritta «non è più in diretta» arriva anche senza messaggi
        ctx.request_repaint_after(Duration::from_secs(2));
    }

    /// Dopo la sincronizzazione di un telefono (chiude il QR se era il nuovo).
    pub fn synced(&mut self) {
        if self.qr.is_some() {
            self.qr = None;
            self.msg = "Telefono collegato ✓".into();
        }
    }

    pub fn page(&mut self, ui: &mut egui::Ui) {
        let t = super::theme::Tokens::get(ui.ctx());
        let weak = |ui: &mut egui::Ui, s: &str| {
            ui.label(egui::RichText::new(i18n::tr(s)).weak());
        };
        weak(ui, "I documenti da firmare, la firma, i timbri, le pagine dell'archivio, gli appunti e le impostazioni AI (chiavi comprese) restano uguali sul computer e sull'app per telefono (Android, iPhone, iPad), attraverso la rete Wi‑Fi di casa: niente cloud, tutto cifrato.");
        weak(ui, "Il telefono si sincronizza da solo quando è sulla stessa rete e OpenSlate è aperto qui. Quello che si toglie o si sovrascrive va nel cestino, non si perde.");
        ui.add_space(10.0);

        if let Some((mods, n, until, _)) = &self.qr {
            let left = until.saturating_duration_since(Instant::now());
            if left.is_zero() {
                self.qr = None;
            } else {
                let (mods, n) = (mods.clone(), *n);
                ui.label(egui::RichText::new(i18n::t("Nell'app sul telefono: Impostazioni › Sincronizza col computer › Inquadra il QR.")).strong());
                let side = 280.0;
                let (r, _) = ui.allocate_exact_size(egui::vec2(side + 32.0, side + 32.0), egui::Sense::hover());
                ui.painter().rect_filled(r, 6.0, egui::Color32::WHITE);
                let cell = side / n as f32;
                let origin = r.min + egui::vec2(16.0, 16.0);
                for y in 0..n {
                    for x in 0..n {
                        if mods[y * n + x] {
                            let p = origin + egui::vec2(x as f32 * cell, y as f32 * cell);
                            ui.painter().rect_filled(egui::Rect::from_min_size(p, egui::vec2(cell + 0.5, cell + 0.5)), 0.0, egui::Color32::BLACK);
                        }
                    }
                }
                ui.label(egui::RichText::new(i18n::t("Non fotografarlo e non condividerlo: chi lo inquadra può leggere e cambiare i tuoi dati.")).color(t.orange));
                ui.label(egui::RichText::new(i18n::tf(format!("Si chiude da solo fra {} secondi.", left.as_secs() + 1))).weak().small());
                if ui.button(i18n::t("Chiudi il QR")).clicked() {
                    self.qr = None;
                }
                ui.ctx().request_repaint_after(Duration::from_millis(500));
            }
        }
        if self.qr.is_none() && ui.button(format!("{}  {}", ic::QR_CODE, i18n::t("Collega un telefono…"))).clicked() {
            match self.sync.pair() {
                Ok(text) => match qrcode::QrCode::with_error_correction_level(text.as_bytes(), qrcode::EcLevel::M) {
                    Ok(code) => {
                        let n = code.width();
                        let mods = code.to_colors().into_iter().map(|c| c == qrcode::Color::Dark).collect();
                        self.qr = Some((mods, n, Instant::now() + QR_TIME, self.sync.phones().len()));
                        self.msg.clear();
                    }
                    Err(e) => self.msg = format!("⚠ QR: {e}"),
                },
                Err(e) => self.msg = format!("⚠ {e}"),
            }
        }
        if let Some(e) = &self.sync.error {
            ui.label(egui::RichText::new(format!("⚠ {}", i18n::tf(e.clone()))).color(t.orange));
        }
        if !self.msg.is_empty() {
            ui.label(i18n::tr(&self.msg));
        }

        ui.add_space(14.0);
        ui.label(egui::RichText::new(i18n::t("Telefoni collegati")).strong());
        weak(ui, "Nell'app per telefono si può accendere «Mostra in diretta sul computer»: quello che scrivi sul telefono compare qui in una finestra, mentre lo scrivi.");
        let has_live = self.sync.mirror.lock().map(|m| m.gen != 0).unwrap_or(false);
        if has_live && !self.mirror_open && ui.button(format!("{}  {}", ic::DEVICE_MOBILE, i18n::t("Mostra la pagina del telefono"))).clicked() {
            self.mirror_open = true;
        }
        let phones = self.sync.phones();
        if phones.is_empty() {
            weak(ui, "Nessuno.");
        }
        let mut drop = None;
        egui::Grid::new("phones").num_columns(3).spacing([18.0, 8.0]).show(ui, |ui| {
            for p in &phones {
                let name = if p.name.is_empty() { i18n::t("Telefono (in attesa)").to_string() } else { p.name.clone() };
                ui.label(format!("{}  {name}", ic::DEVICE_MOBILE));
                let last = if p.last.is_empty() { i18n::t("mai sincronizzato").to_string() } else { i18n::tf(format!("ultima sincronizzazione: {}", p.last)) };
                ui.label(egui::RichText::new(last).weak());
                if ui.button(i18n::t("Scollega")).on_hover_text(i18n::t("Il telefono non potrà più sincronizzarsi (per ricollegarlo serve un QR nuovo)")).clicked() {
                    drop = Some(p.id.clone());
                }
                ui.end_row();
            }
        });
        if let Some(id) = drop {
            self.sync.unpair(&id);
        }
    }
}
