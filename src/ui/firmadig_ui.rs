//! La finestra «Firma digitale»: firma un PDF con la firma remota di Aruba
//! (vedi `crate::firmadigitale`). Prima si mettono firma grafica e timbro e si
//! salva il PDF; poi la firma digitale, per ultima: dopo, il PDF non si tocca.

use super::{Action, App};
use crate::firmadigitale::{self, Account};
use crate::i18n;
use eframe::egui;
use egui_phosphor::regular as ic;
use std::path::PathBuf;

/// La finestra aperta.
#[derive(Default)]
pub(super) struct FirmaForm {
    pdf: Option<PathBuf>,
    account: String,
    password: String,
    otp: String,
    busy: bool,
    error: Option<String>,
    /// Il cursore va nel primo campo vuoto, una volta sola.
    focused: bool,
}

/// Cosa fa la finestra.
pub(super) enum FirmaAction {
    Open,
    PickPdf,
    Sign,
    Close,
}

impl App {
    pub(super) fn apply_firma(&mut self, a: FirmaAction) {
        match a {
            FirmaAction::Open => {
                // l'ultimo PDF salvato, se c'è
                let pdf = self.docui.last_export.clone().filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf")) && p.exists());
                self.docui.firma = Some(FirmaForm { pdf, account: self.settings.aruba_account.clone(), ..Default::default() });
            }
            FirmaAction::PickPdf => {
                if let Some(p) = super::sysdlg::pick_file(i18n::t("Il PDF da firmare digitalmente"), "PDF", &["pdf"]) {
                    if let Some(f) = &mut self.docui.firma {
                        f.pdf = Some(p);
                        f.error = None;
                    }
                }
            }
            FirmaAction::Close => {
                if self.docui.firma.as_ref().is_some_and(|f| !f.busy) {
                    self.docui.firma = None;
                }
            }
            FirmaAction::Sign => {
                let Some(f) = &mut self.docui.firma else { return };
                let Some(src) = f.pdf.clone() else { return };
                let acc = Account::parse(&f.account);
                self.settings.aruba_account = acc.label();
                self.settings.save();
                f.account = acc.label();
                f.busy = true;
                f.error = None;
                let (password, otp) = (f.password.clone(), std::mem::take(&mut f.otp));
                self.status = "Firma digitale: chiedo ad Aruba…".into();
                let tx = self.evt_tx.clone();
                std::thread::spawn(move || {
                    let r = (|| -> anyhow::Result<PathBuf> {
                        let pdf = std::fs::read(&src)?;
                        let t: std::sync::Arc<dyn firmadigitale::Transport> = std::sync::Arc::new(firmadigitale::Https);
                        let certs = firmadigitale::certificates(&*t, &acc, &password)?;
                        let out = firmadigitale::sign_pdf(t, &pdf, &acc, &password, &otp, &certs[0])?;
                        let dst = firmadigitale::signed_path(&src);
                        crate::web::write_atomic(&dst, &out)?;
                        Ok(dst)
                    })();
                    let _ = tx.send(crate::actor::UiEvent::DigitalSigned(r.map_err(|e| format!("{e:#}"))));
                });
            }
        }
    }

    /// Com'è andata la firma digitale.
    pub(super) fn on_digital_signed(&mut self, r: Result<PathBuf, String>) {
        match r {
            Ok(p) => {
                self.docui.firma = None;
                crate::settings::run_hook(&self.settings.hook, &p.to_string_lossy());
                crate::platform::open(&p);
                self.status = format!("Firmato digitalmente: {}", p.display());
                self.docui.last_export = Some(p);
            }
            Err(e) => {
                self.status = format!("⚠ {e}");
                if let Some(f) = &mut self.docui.firma {
                    f.busy = false;
                    f.error = Some(e);
                }
            }
        }
    }

    pub(super) fn firma_window(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        let Some(f) = &mut self.docui.firma else { return };
        let mut act = None;
        let mut open = true;
        egui::Window::new(i18n::t("Firma digitale (Aruba)"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(420.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label(i18n::t("Firma PAdES con la tua firma remota Aruba. Ad Aruba va solo l'impronta del documento, non il file. Fallo per ultimo: dopo la firma digitale il PDF non va più modificato."));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let name = f.pdf.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned());
                    ui.label(format!("{} {}", ic::FILE_PDF, name.as_deref().unwrap_or(i18n::t("nessun PDF scelto"))));
                    if ui.add_enabled(!f.busy, egui::Button::new(i18n::t("Scegli…"))).clicked() {
                        act = Some(FirmaAction::PickPdf);
                    }
                });
                ui.add_space(4.0);
                egui::Grid::new("firma_grid").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                    ui.label(i18n::t("Utente"));
                    let a = ui.add_enabled(!f.busy, egui::TextEdit::singleline(&mut f.account).hint_text("utente@firma").desired_width(240.0));
                    ui.end_row();
                    ui.label(i18n::t("Password"));
                    let p = ui.add_enabled(!f.busy, egui::TextEdit::singleline(&mut f.password).password(true).desired_width(240.0));
                    ui.end_row();
                    ui.label(i18n::t("Codice OTP"));
                    let o = ui.add_enabled(!f.busy, egui::TextEdit::singleline(&mut f.otp).hint_text(i18n::t("dall'app Aruba OTP")).desired_width(120.0));
                    ui.end_row();
                    if !f.focused {
                        f.focused = true;
                        let first = if f.account.trim().is_empty() { a } else if f.password.is_empty() { p } else { o };
                        first.request_focus();
                    }
                });
                ui.small(i18n::t("Utente come nel SelfCare di Aruba: utente@dominio (se l'hai comprata su pec.it il dominio è «firma»). La password e l'OTP non vengono salvati."));
                if let Some(e) = &f.error {
                    ui.add_space(4.0);
                    ui.colored_label(egui::Color32::from_rgb(200, 40, 40), e);
                }
                ui.add_space(6.0);
                let ready = !f.busy && f.pdf.is_some() && !f.account.trim().is_empty() && !f.password.is_empty() && f.otp.trim().len() >= 6;
                ui.horizontal(|ui| {
                    let label = if f.busy { i18n::tf(format!("{} Firmo…", ic::HOURGLASS)) } else { i18n::tf(format!("{} Firma", ic::SEAL_CHECK)) };
                    if ui.add_enabled(ready, egui::Button::new(label)).clicked() || (ready && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                        act = Some(FirmaAction::Sign);
                    }
                    if ui.add_enabled(!f.busy, egui::Button::new(i18n::tf(format!("{} Annulla", ic::X)))).clicked() {
                        act = Some(FirmaAction::Close);
                    }
                });
            });
        if !open {
            act = Some(FirmaAction::Close);
        }
        if let Some(a) = act {
            actions.push(Action::Firma(a));
        }
    }
}
