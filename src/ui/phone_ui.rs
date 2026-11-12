//! Opzioni › Telefono: collegare un telefono col QR (a richiesta, per un
//! minuto) e vedere o scollegare i telefoni abbinati.

use crate::i18n;
use crate::phonesync::PhoneSync;
use eframe::egui;
use egui_phosphor::regular as ic;
use std::time::{Duration, Instant};

/// Quanto resta visibile il QR.
const QR_TIME: Duration = Duration::from_secs(60);

pub(super) struct PhoneUi {
    pub sync: PhoneSync,
    /// Il QR mostrato: moduli, lato, quando chiuderlo, quanti telefoni c'erano.
    qr: Option<(Vec<bool>, usize, Instant, usize)>,
    msg: String,
}

impl PhoneUi {
    pub fn new(sync: PhoneSync) -> Self {
        PhoneUi { sync, qr: None, msg: String::new() }
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
        weak(ui, "Le pagine dell'archivio, gli appunti e i timbri restano uguali sul computer e sull'app per telefono (Android, iPhone, iPad), attraverso la rete Wi‑Fi di casa: niente cloud, tutto cifrato.");
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
