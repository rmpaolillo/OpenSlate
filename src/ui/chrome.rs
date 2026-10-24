//! Cornice della finestra in stile Word: barra del titolo (accesso rapido,
//! titolo, ricerca dei comandi, spia dello Slate), schede (File, Home, …),
//! barra multifunzione, schede dei file, righelli e barra di stato con lo zoom.
//!
//! Disposizione, misure e colori ripresi da WordCraft (`chrome.rs`,
//! `ribbon.rs`, `canvas.rs` di https://github.com/storytold/wordcraft,
//! MIT OR Apache-2.0, © ArtCraft Team e contributori di WordCraft).

use crate::i18n;
use super::backstage::BsPage;
use super::doc_view::DocAction;
use super::theme::{icon, medium, regular, semibold, Tokens};
use super::widgets::{icon_button, RIBBON_H};
use super::zoom::{ZoomOp, ZoomView, ZOOM_MAX, ZOOM_MIN};
use super::{library_view, Action, App, Fmt, Mode};
use crate::actor::Link;
use eframe::egui;
use egui::{pos2, vec2, Align2, Color32, Pos2, Rect, Sense, Stroke};
use egui_phosphor::regular as ic;

/// Barra del titolo integrata nella finestra (macOS: accanto ai semafori).
const MAC_TITLE: bool = cfg!(target_os = "macos");

/// Schede della barra multifunzione.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Tab {
    Home,
    Documento,
    Visualizza,
    Ai,
    /// Scheda contestuale della vista Archivio.
    Archivio,
    /// Scheda contestuale dell'appunto aperto.
    Appunto,
}

impl Tab {
    const BOARD: [Tab; 4] = [Tab::Home, Tab::Documento, Tab::Visualizza, Tab::Ai];
    fn label(self) -> &'static str {
        match self {
            Tab::Home => "Home",
            Tab::Documento => "Documento",
            Tab::Visualizza => "Visualizza",
            Tab::Ai => "AI",
            Tab::Archivio => "Archivio",
            Tab::Appunto => "Appunto",
        }
    }
}

/// Ricerca dei comandi (casella in alto o Cmd+F).
pub(super) struct Palette {
    pub query: String,
    pub sel: usize,
    pub fresh: bool,
}

/// Un comando trovabile con la ricerca.
pub(super) struct Cmd {
    pub glyph: &'static str,
    pub label: String,
    pub keys: String,
    pub enabled: bool,
    pub act: Action,
}

fn cmd_key(k: &str) -> String {
    format!("{}+{k}", if cfg!(target_os = "macos") { "Cmd" } else { "Ctrl" })
}

/// Una voce di menu a comparsa (icona, testo, scorciatoia).
/// «Altri formati»: un menu (o un sottomenu) con i formati in più delle
/// pagine (JPEG, WebP, Word, Xournal++, GIF e SVG animati, LaTeX, Typst).
pub(super) fn more_formats_menu(ui: &mut egui::Ui, mut pick: impl FnMut(Fmt)) {
    let label = i18n::tf(format!("{} Altri formati", ic::DOTS_THREE_CIRCLE));
    ui.menu_button(label, |ui| {
        for k in Fmt::MORE {
            if menu_item(ui, true, k.icon(), k.label(), "") {
                pick(k);
            }
        }
    })
    .response
    .on_hover_text(i18n::t("Immagini leggere, Word con l'inchiostro, Xournal++, la scrittura animata, LaTeX e Typst dalla trascrizione"));
}

pub(super) fn menu_item(ui: &mut egui::Ui, enabled: bool, glyph: &str, text: &str, keys: &str) -> bool {
    let b = egui::Button::new(i18n::tf(format!("{glyph}  {}", crate::i18n::tr(text)))).shortcut_text(keys);
    let clicked = ui.add_enabled(enabled, b).clicked();
    if clicked {
        ui.close();
    }
    clicked
}

/// Il testo accorciato con «…» finché sta in `room` (vuoto se non ci sta
/// nemmeno così). Sempre finito: ogni giro toglie un carattere.
fn fit_text(text: &str, room: f32, width: impl Fn(&str) -> f32) -> String {
    if width(text) <= room {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    for n in (2..chars.len()).rev() {
        let s: String = chars[..n].iter().collect::<String>().trim_end().to_string() + "…";
        if width(&s) <= room {
            return s;
        }
    }
    String::new()
}

fn text_w(ui: &egui::Ui, s: &str, font: egui::FontId) -> f32 {
    ui.painter().layout_no_wrap(i18n::tr(s).into_owned(), font, Color32::WHITE).size().x
}

impl App {
    /// Colore, testo e spiegazione della spia dello Slate.
    fn link_state(&self, time: f64) -> (Color32, String, &'static str) {
        let t = Tokens::light();
        match self.link {
            Link::On => {
                let batt = self.info.as_ref().and_then(|i| i.battery_pct).map(|b| format!(" · {b}%")).unwrap_or_default();
                (
                    Color32::from_rgb(46, 180, 90),
                    format!("Slate{batt}"),
                    if self.live_on { "Slate collegato, in diretta" } else { "Slate collegato (diretta ferma)" },
                )
            }
            Link::Connecting => {
                let a = (0.55 + 0.45 * (time * std::f64::consts::TAU * 1.2).sin()) as f32;
                (t.orange.gamma_multiply(a.clamp(0.15, 1.0)), "Collego…".into(), "Collegamento in corso")
            }
            Link::Off => (Color32::from_rgb(210, 60, 50), "Scollegato".into(), "Slate scollegato: clic per collegarlo, scaricare le pagine e l'aiuto"),
        }
    }

    /// Spia dello Slate (verde / arancio lampeggiante / rossa) con un menu.
    fn led_pill(&self, ui: &mut egui::Ui, actions: &mut Vec<Action>, h: f32) {
        let t = Tokens::get(ui.ctx());
        if self.link == Link::Connecting {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
        }
        let (col, text, tip) = self.link_state(ui.input(|i| i.time));
        let w = text_w(ui, &text, medium(11.5)) + 32.0;
        let (r, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
        let p = ui.painter();
        p.rect(r, h / 2.0, if resp.hovered() { t.hover } else { t.ribbon }, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
        let c = pos2(r.min.x + 13.0, r.center().y);
        p.circle_filled(c, 5.5, col);
        if self.link == Link::On && !self.live_on {
            p.circle_filled(c, 2.4, t.ribbon);
        }
        p.circle_stroke(c, 6.0, Stroke::new(1.0, Color32::from_black_alpha(70)));
        p.text(pos2(r.min.x + 24.0, r.center().y), Align2::LEFT_CENTER, i18n::tr(&&text).into_owned(), medium(11.5), t.text);
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &text));
        // il pad ha un posto solo: questo menu
        let resp = resp.on_hover_text(i18n::tr(&tip).into_owned());
        egui::Popup::menu(&resp).show(|ui| {
            ui.set_min_width(250.0);
            let dim = |ui: &mut egui::Ui, s: String| {
                ui.label(egui::RichText::new(i18n::tr(&s).into_owned()).weak());
            };
            match (&self.info, self.link) {
                (Some(i), Link::On) => {
                    let batt = i.battery_pct.map(|b| format!("{b}%")).unwrap_or_else(|| "?".into());
                    dim(ui, format!("{} {} · batteria {batt}{}", ic::DEVICE_TABLET, if i.name.is_empty() { "Slate" } else { &i.name }, if i.charging { " (in carica)" } else { "" }));
                    if !i.firmware.is_empty() {
                        dim(ui, format!("{} firmware {}", ic::CPU, i.firmware));
                    }
                }
                (_, Link::Connecting) => dim(ui, format!("{} Collegamento in corso…", ic::BLUETOOTH)),
                _ => dim(ui, format!("{} Nessun pad collegato", ic::DEVICE_TABLET)),
            }
            ui.separator();
            let on = self.link == Link::On;
            if menu_item(ui, self.link != Link::Connecting, ic::BLUETOOTH, if on { "Collega di nuovo" } else { "Collega lo Slate" }, "") {
                actions.push(Action::Connect);
            }
            let live = if self.live_on { (ic::STOP, "Ferma la diretta") } else { (ic::PLAY, "Avvia la diretta (quello che scrivi appare subito)") };
            if menu_item(ui, on, live.0, live.1, "") {
                actions.push(Action::ToggleLive);
            }
            if menu_item(ui, on, ic::DOWNLOAD_SIMPLE, "Scarica le pagine dal pad", "") {
                actions.push(Action::Sync);
            }
            ui.separator();
            if menu_item(ui, true, ic::QUESTION, "Come si collega…", "") {
                actions.push(Action::ShowHelp(super::options::HelpPage::Collegamento));
            }
            if menu_item(ui, true, ic::GEAR_SIX, "Opzioni del pad e del pennino…", "") {
                actions.push(Action::OpenPrefs);
            }
        });
    }

    /// Titolo al centro della finestra.
    fn window_title(&self) -> String {
        let what = match (self.mode, self.center, &self.docui.doc) {
            (Mode::Library, _, _) => "Archivio".to_string(),
            (_, super::workspace::Center::Note, _) => self.note.clone().unwrap_or_else(|| "Appunti".into()).trim_end_matches(".md").to_string(),
            (_, super::workspace::Center::Settings, _) => "Impostazioni AI".to_string(),
            (_, _, Some(d)) => d.title.clone(),
            _ => "Lavagna".to_string(),
        };
        format!("{} — OpenSlate", i18n::tr(&what))
    }

    /// Barra del titolo: accesso rapido, titolo, ricerca, spia dello Slate.
    pub(super) fn title_bar(&mut self, root: &mut egui::Ui, actions: &mut Vec<Action>) -> f32 {
        let t = Tokens::get(root.ctx());
        let (h, left) = if MAC_TITLE && !self.fullscreen { (30.0, 84) } else { (38.0, 10) };
        let control_h = if MAC_TITLE { 22.0 } else { 26.0 };
        let (can_undo, can_redo) = {
            let s = self.hub.lock();
            (s.can_undo(), s.can_redo())
        };
        let r = egui::Panel::top("wsr_title_bar")
            .exact_size(h)
            .frame(egui::Frame::NONE.fill(t.title_bar).inner_margin(egui::Margin { left, right: 10, top: 0, bottom: 0 }))
            .show(root, |ui| {
                let full = ui.max_rect();
                // la barra del titolo sposta la finestra (doppio clic = ingrandisci)
                let drag = ui.interact(full, ui.id().with("title_drag"), Sense::click_and_drag());
                if drag.drag_started() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                if drag.double_clicked() {
                    let max = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
                }
                let mut qat_end = full.min.x;
                let mut right_start = full.max.x;
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
                    if !MAC_TITLE {
                        let (r, _) = ui.allocate_exact_size(vec2(24.0, control_h), Sense::hover());
                        ui.painter().text(r.center(), Align2::CENTER_CENTER, ic::PEN_NIB, icon(18.0), t.accent);
                        ui.add_space(6.0);
                    }
                    let save_tip = if self.docui.doc.is_some() { "Salva il documento firmato" } else { "Salva la lavagna nell'archivio" };
                    if icon_button(ui, ic::FLOPPY_DISK, &format!("{} ({})", i18n::t(save_tip), cmd_key("S")), vec2(26.0, control_h), true).clicked() {
                        actions.push(if self.docui.doc.is_some() { Action::Doc(DocAction::SaveSigned) } else { Action::SaveBoard });
                    }
                    if icon_button(ui, ic::ARROW_U_UP_LEFT, &format!("Annulla ({})", cmd_key("Z")), vec2(26.0, control_h), can_undo).clicked() {
                        actions.push(Action::Undo);
                    }
                    if icon_button(ui, ic::ARROW_U_UP_RIGHT, &format!("Ripeti ({})", cmd_key("Shift+Z")), vec2(26.0, control_h), can_redo).clicked() {
                        actions.push(Action::Redo);
                    }
                    // Copia e Incolla valgono in tutte le schede: qui, accanto ad Annulla
                    ui.add_space(4.0);
                    let (sep, _) = ui.allocate_exact_size(vec2(1.0, control_h * 0.55), Sense::hover());
                    ui.painter().rect_filled(sep, 0.0, t.border);
                    ui.add_space(4.0);
                    if icon_button(ui, ic::COPY, &format!("{} ({})", i18n::t("Copia il foglio come immagine"), cmd_key("C")), vec2(26.0, control_h), true).clicked() {
                        actions.push(Action::CopyImage);
                    }
                    if icon_button(ui, ic::CLIPBOARD_TEXT, &format!("{} ({})", i18n::t("Incolla un'immagine (si apre come documento)"), cmd_key("V")), vec2(26.0, control_h), true).clicked() {
                        actions.push(Action::Paste);
                    }
                    // vista compatta: «schermo intero» solo per menu e strumenti
                    ui.add_space(4.0);
                    let (sep, _) = ui.allocate_exact_size(vec2(1.0, control_h * 0.55), Sense::hover());
                    ui.painter().rect_filled(sep, 0.0, t.border);
                    ui.add_space(4.0);
                    let (glyph, tip) = if self.settings.compact_ui {
                        (ic::ARROWS_OUT_SIMPLE, "Mostra la barra dei menu e gli strumenti")
                    } else {
                        (ic::ARROWS_IN_SIMPLE, "Nascondi la barra dei menu e gli strumenti (resta il foglio)")
                    };
                    let r = icon_button(ui, glyph, &format!("{} ({})", i18n::t(tip), cmd_key("Shift+F")), vec2(26.0, control_h), true);
                    if self.settings.compact_ui {
                        ui.painter().rect_stroke(r.rect.shrink(1.0), 4.0, egui::Stroke::new(1.0, t.accent), egui::StrokeKind::Inside);
                    }
                    if r.clicked() {
                        actions.push(Action::ToggleCompact);
                    }
                    qat_end = ui.min_rect().max.x;
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        self.led_pill(ui, actions, control_h);
                        ui.add_space(4.0);
                        // aiuto: scorciatoie, come si collega, guida (File → Informazioni)
                        if icon_button(ui, ic::QUESTION, "Aiuto: scorciatoie, come si collega lo Slate, guida (F1)", vec2(26.0, control_h), true).clicked() {
                            actions.push(Action::Backstage(Some(BsPage::Info)));
                        }
                        ui.add_space(4.0);
                        // la chat AI a destra (come Copilot in Word)
                        // larga quanto la scritta nella lingua scelta
                        let label_w = ui.fonts_mut(|f| f.layout_no_wrap(i18n::t("Chat AI").to_string(), medium(11.5), t.text).size().x);
                        let (r, resp) = ui.allocate_exact_size(vec2((label_w + 37.0).max(control_h + 52.0), control_h), Sense::click());
                        let on = self.chat_open;
                        ui.painter().rect(
                            r,
                            control_h / 2.0,
                            if on { t.checked } else if resp.hovered() { t.hover } else { t.ribbon },
                            Stroke::new(1.0, if on { t.accent.linear_multiply(0.6) } else { t.border }),
                            egui::StrokeKind::Inside,
                        );
                        ui.painter().text(pos2(r.min.x + 14.0, r.center().y), Align2::CENTER_CENTER, ic::SPARKLE, icon(14.0), t.accent_text);
                        ui.painter().text(pos2(r.min.x + 25.0, r.center().y), Align2::LEFT_CENTER, i18n::t("Chat AI"), medium(11.5), if on { t.accent_text } else { t.text });
                        resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, on, "Chat AI"));
                        if resp.on_hover_text(i18n::t("Mostra/nasconde la chat AI a destra")).clicked() {
                            actions.push(Action::Chat);
                        }
                        ui.add_space(8.0);
                        // ricerca dei comandi
                        let sw = (full.width() * 0.22).clamp(150.0, 250.0);
                        let (r, resp) = ui.allocate_exact_size(vec2(sw, control_h), Sense::click());
                        ui.painter().rect(r, 6.0, if resp.hovered() { t.input } else { t.ribbon }, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
                        ui.painter().text(pos2(r.min.x + 15.0, r.center().y), Align2::CENTER_CENTER, ic::MAGNIFYING_GLASS, icon(14.0), t.text_dim);
                        let clip = ui.painter().with_clip_rect(r.shrink(2.0));
                        clip.text(pos2(r.min.x + 28.0, r.center().y), Align2::LEFT_CENTER, i18n::t("Cerca comandi"), regular(11.5), t.text_dim);
                        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Cerca comandi"));
                        if resp.on_hover_text(i18n::tf(format!("Cerca un comando ({})", cmd_key("F")))).clicked() {
                            actions.push(Action::Palette);
                        }
                        self.palette_anchor = r;
                        right_start = ui.min_rect().min.x;
                    });
                });
                // titolo centrato (accorciato se non c'è posto)
                let title = self.window_title();
                let font = semibold(12.5);
                let cx = full.center().x;
                let room = 2.0 * (cx - qat_end - 12.0).min(right_start - 12.0 - cx);
                let shown = fit_text(&title, room, |s| text_w(ui, s, font.clone()));
                if !shown.is_empty() {
                    ui.painter().text(pos2(cx, full.center().y), Align2::CENTER_CENTER, i18n::tr(&shown).into_owned(), font, t.text);
                }
            });
        r.response.rect.height()
    }

    /// Una scheda della striscia: testo, sottolineatura se attiva.
    fn tab_button(ui: &mut egui::Ui, label: &str, active: bool, special: bool) -> egui::Response {
        let t = Tokens::get(ui.ctx());
        let w = text_w(ui, label, medium(12.5)) + 22.0;
        let (r, resp) = ui.allocate_exact_size(vec2(w, 30.0), Sense::click());
        if resp.hovered() && !active {
            ui.painter().rect_filled(r.shrink2(vec2(0.0, 4.0)), 4.0, t.hover);
        }
        let color = if active || special { t.accent_text } else { t.text };
        ui.painter().text(r.center(), Align2::CENTER_CENTER, i18n::tr(&label).into_owned(), if active { semibold(12.5) } else { medium(12.5) }, color);
        if active {
            let u = Rect::from_center_size(pos2(r.center().x, r.max.y - 2.0), vec2(w - 16.0, 3.0));
            ui.painter().rect_filled(u, 2.0, t.accent);
        }
        resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, active, label));
        resp
    }

    /// Striscia delle schede: File (pagina a tutta finestra), le schede della
    /// barra, a destra Lavagna/Archivio e «Condividi».
    pub(super) fn tab_strip(&mut self, root: &mut egui::Ui, actions: &mut Vec<Action>) -> f32 {
        let t = Tokens::get(root.ctx());
        let r = egui::Panel::top("wsr_tabs")
            .exact_size(30.0)
            .frame(egui::Frame::NONE.fill(t.tab_strip).inner_margin(egui::Margin { left: 8, right: 10, top: 0, bottom: 0 }))
            .show(root, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
                    let file = Self::tab_button(ui, "File", false, true).on_hover_text(i18n::t("Nuovo, Apri, Salva, Esporta, Opzioni…"));
                    if file.clicked() {
                        actions.push(Action::Backstage(Some(BsPage::Home)));
                    }
                    let mut tabs = Tab::BOARD.to_vec();
                    if self.mode == Mode::Library {
                        tabs.push(Tab::Archivio);
                    } else if self.note.is_some() || self.center == super::workspace::Center::Note {
                        tabs.push(Tab::Appunto);
                    }
                    for tab in tabs {
                        let active = match self.mode {
                            Mode::Library => tab == Tab::Archivio,
                            Mode::Board => tab == self.tab && self.show_ribbon,
                        };
                        let resp = Self::tab_button(ui, tab.label(), active, matches!(tab, Tab::Archivio | Tab::Appunto));
                        if resp.double_clicked() && tab != Tab::Archivio {
                            actions.push(Action::ToggleRibbon);
                        } else if resp.clicked() {
                            actions.push(Action::Tab(tab));
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        self.share_button(ui, actions);
                        ui.add_space(10.0);
                        self.mode_switch(ui, actions);
                    });
                });
            });
        r.response.rect.height()
    }

    /// Lavagna | Archivio.
    fn mode_switch(&self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let t = Tokens::get(ui.ctx());
        let n = self.lib.entries.len();
        // da destra a sinistra
        let items = [
            (Mode::Library, format!("{} Archivio ({n})", ic::SQUARES_FOUR), "Le pagine salvate e scaricate dal pad"),
            (Mode::Board, format!("{} Lavagna", ic::CHALKBOARD_SIMPLE), "Il foglio in diretta e i documenti"),
        ];
        for (i, (m, label, tip)) in items.into_iter().enumerate() {
            let w = text_w(ui, &label, medium(11.5)) + 18.0;
            let (r, resp) = ui.allocate_exact_size(vec2(w, 22.0), Sense::click());
            let on = self.mode == m;
            let rounding = if i == 0 {
                egui::CornerRadius { nw: 0, sw: 0, ne: 5, se: 5 }
            } else {
                egui::CornerRadius { nw: 5, sw: 5, ne: 0, se: 0 }
            };
            let fill = if on {
                t.checked
            } else if resp.hovered() {
                t.hover
            } else {
                t.ribbon
            };
            ui.painter().rect(r, rounding, fill, Stroke::new(1.0, if on { t.accent.linear_multiply(0.6) } else { t.border_strong }), egui::StrokeKind::Inside);
            ui.painter().text(r.center(), Align2::CENTER_CENTER, i18n::tr(&&label).into_owned(), medium(11.5), if on { t.accent_text } else { t.text });
            if resp.on_hover_text(i18n::tr(&tip).into_owned()).clicked() && !on {
                actions.push(Action::SetMode(m));
            }
        }
    }

    /// Pulsante colorato «Condividi» (diretta web, immagine, PDF).
    fn share_button(&self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let t = Tokens::get(ui.ctx());
        let label = "Condividi";
        let w = text_w(ui, label, medium(12.0)) + 38.0;
        let (r, resp) = ui.allocate_exact_size(vec2(w.max(74.0), 24.0), Sense::click());
        ui.painter().rect_filled(r, 4.0, if resp.hovered() { t.accent_text } else { t.accent });
        ui.painter().text(pos2(r.min.x + 15.0, r.center().y), Align2::CENTER_CENTER, ic::SHARE_NETWORK, icon(14.0), t.on_accent);
        ui.painter().text(pos2(r.min.x + 27.0, r.center().y), Align2::LEFT_CENTER, i18n::tr(&label).into_owned(), medium(12.0), t.on_accent);
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
        let resp = resp.on_hover_text(i18n::t("Diretta web per TV e telefono, copia come immagine, esporta in PDF / PNG / SVG"));
        egui::Popup::menu(&resp).show(|ui| {
            ui.set_min_width(260.0);
            match &self.web_url {
                None => {
                    if menu_item(ui, !self.web_starting, ic::BROADCAST, "Avvia la diretta web", "") {
                        actions.push(Action::Web);
                    }
                }
                Some(u) => {
                    ui.label(egui::RichText::new(i18n::tf(format!("{} {u}", ic::BROADCAST))).weak());
                    if menu_item(ui, true, ic::LINK, "Copia il link della diretta", "") {
                        actions.push(Action::CopyWebUrl);
                    }
                    if menu_item(ui, true, ic::GLOBE, "Apri la diretta nel browser", "") {
                        actions.push(Action::OpenWebBrowser);
                    }
                }
            }
            ui.separator();
            if menu_item(ui, true, ic::COPY, "Copia il foglio come immagine", &cmd_key("C")) {
                actions.push(Action::CopyImage);
            }
            ui.separator();
            if menu_item(ui, true, ic::FILE_PDF, "Esporta in PDF", "") {
                actions.push(Action::Export(Fmt::Pdf));
            }
            if menu_item(ui, true, ic::FILE_PNG, "Esporta in PNG (immagine)", "") {
                actions.push(Action::Export(Fmt::Png));
            }
            if menu_item(ui, true, ic::FILE_SVG, "Esporta in SVG (vettoriale)", "") {
                actions.push(Action::Export(Fmt::Svg));
            }
            more_formats_menu(ui, |k| actions.push(Action::Export(k)));
            if menu_item(ui, true, ic::FOLDER_OPEN, "Apri la cartella dei file salvati", "") {
                actions.push(Action::OpenExports);
            }
        });
    }

    /// Barra multifunzione: i gruppi della scheda scelta.
    pub(super) fn ribbon_panel(&mut self, root: &mut egui::Ui, actions: &mut Vec<Action>) -> f32 {
        let t = Tokens::get(root.ctx());
        let r = egui::Panel::top("wsr_ribbon")
            .exact_size(RIBBON_H)
            .frame(egui::Frame::NONE.fill(t.ribbon).inner_margin(egui::Margin { left: 8, right: 8, top: 4, bottom: 4 }).stroke(Stroke::new(1.0, t.border)))
            .show(root, |ui| {
                egui::ScrollArea::horizontal()
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .show(ui, |ui| {
                        if self.mode == Mode::Library {
                            ui.vertical(|ui| self.library_ribbon(ui, actions));
                            return;
                        }
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
                            match self.tab {
                                Tab::Home => self.ribbon_home(ui, actions),
                                Tab::Documento => self.ribbon_documento(ui, actions),
                                Tab::Visualizza => self.ribbon_visualizza(ui, actions),
                                Tab::Ai => self.ribbon_ai(ui, actions),
                                Tab::Appunto => self.ribbon_appunto(ui, actions),
                                Tab::Archivio => {}
                            }
                        });
                    });
            });
        r.response.rect.height()
    }

    /// Schede dei file aperti (solo se c'è almeno un documento).
    pub(super) fn files_strip(&self, root: &mut egui::Ui, actions: &mut Vec<Action>) {
        if self.docui.tabs.is_empty() && self.docui.doc.is_none() && !self.docui.importing && self.note.is_none() && !self.settings_open {
            return;
        }
        let t = Tokens::get(root.ctx());
        egui::Panel::top("wsr_files")
            .exact_size(30.0)
            .frame(egui::Frame::NONE.fill(t.canvas).inner_margin(egui::Margin { left: 8, right: 10, top: 3, bottom: 0 }))
            .show(root, |ui| self.file_tabs(ui, actions));
    }

    /// Barra di stato: pagina, tratti, strumento, messaggi; a destra lo zoom.
    pub(super) fn status_bar(&mut self, root: &mut egui::Ui, actions: &mut Vec<Action>) {
        let t = Tokens::get(root.ctx());
        egui::Panel::bottom("wsr_status")
            .exact_size(26.0)
            .frame(egui::Frame::NONE.fill(t.status_bar).inner_margin(egui::Margin { left: 12, right: 12, top: 0, bottom: 0 }).stroke(Stroke::new(1.0, t.border)))
            .show(root, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing = vec2(14.0, 0.0);
                    let st = |ui: &mut egui::Ui, s: &str, color: Color32| {
                        ui.label(egui::RichText::new(i18n::tr(&s).into_owned()).font(regular(11.0)).color(color));
                    };
                    match self.mode {
                        Mode::Board if self.center != super::workspace::Center::Board => {
                            if self.center == super::workspace::Center::Note {
                                let i = &self.note_info;
                                st(ui, &format!("{} Appunto", ic::NOTE), t.text_dim);
                                st(ui, &format!("{} parole", i.words), t.text_dim);
                                st(ui, if i.dirty { "Modificato" } else { "Salvato" }, t.text_dim);
                                if i.source {
                                    st(ui, "Sorgente Markdown", t.text_dim);
                                }
                            } else {
                                st(ui, &format!("{} Impostazioni AI", ic::GEAR_SIX), t.text_dim);
                            }
                        }
                        Mode::Board => {
                            let (n, tool) = {
                                let s = self.hub.lock();
                                let tool = if s.eraser {
                                    format!("{} Gomma", ic::ERASER)
                                } else if s.tool == crate::scene::Tool::Highlighter {
                                    format!("{} Evidenziatore", ic::HIGHLIGHTER)
                                } else {
                                    format!("{} Penna {:.1}", ic::PEN, s.pen.width)
                                };
                                (s.drawing.strokes.len(), tool)
                            };
                            match &self.docui.doc {
                                Some(d) => st(ui, &format!("Pagina {} di {}", d.current + 1, d.pages.len()), t.text_dim),
                                None => st(ui, "Lavagna", t.text_dim),
                            }
                            st(ui, &format!("{n} tratti"), t.text_dim);
                            st(ui, &tool, t.text_dim);
                        }
                        Mode::Library => {
                            st(ui, &format!("{} pagine nell'archivio", self.lib.entries.len()), t.text_dim);
                            if !self.lui.selected.is_empty() {
                                st(ui, &format!("{} selezionate", self.lui.selected.len()), t.text_dim);
                            }
                            if !self.lui.last_trashed.is_empty()
                                && ui.small_button(i18n::tf(format!("{} Ripristina", ic::ARROW_COUNTER_CLOCKWISE))).clicked()
                            {
                                actions.push(Action::Lib(library_view::LibAction::Restore));
                            }
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing = vec2(4.0, 0.0);
                        let note_zoom = (self.mode == Mode::Board && self.center == super::workspace::Center::Note).then(|| {
                            let mut z = ZoomView::default();
                            z.zoom = self.note_info.zoom.max(0.3);
                            z
                        });
                        let zv: Option<ZoomView> = match self.mode {
                            Mode::Board if self.center == super::workspace::Center::Settings => None,
                            Mode::Board if note_zoom.is_some() => note_zoom,
                            Mode::Board => Some(self.board_zoom),
                            Mode::Library if self.lui.viewer.is_some() => Some(self.lui.viewer_zoom),
                            Mode::Library => None,
                        };
                        if let Some(zv) = zv {
                            let label = zv.label();
                            let w = text_w(ui, "0000%", regular(11.0));
                            let (r, resp) = ui.allocate_exact_size(vec2(w, 18.0), Sense::click());
                            ui.painter().text(r.right_center(), Align2::RIGHT_CENTER, i18n::tr(&label).into_owned(), regular(11.0), t.text_dim);
                            if resp.on_hover_text(i18n::t("100% = pagina intera. Clic: torna alla pagina intera (Cmd+0)")).clicked() {
                                actions.push(Action::Zoom(ZoomOp::Fit));
                            }
                            if icon_button(ui, ic::PLUS, "Ingrandisci (Cmd +)", vec2(18.0, 18.0), true).clicked() {
                                actions.push(Action::Zoom(ZoomOp::In));
                            }
                            let mut v = zv.zoom;
                            ui.spacing_mut().slider_width = 110.0;
                            if ui.add(egui::Slider::new(&mut v, ZOOM_MIN..=ZOOM_MAX).logarithmic(true).show_value(false)).changed() {
                                actions.push(Action::Zoom(ZoomOp::To(v)));
                            }
                            if icon_button(ui, ic::MINUS, "Riduci (Cmd −)", vec2(18.0, 18.0), true).clicked() {
                                actions.push(Action::Zoom(ZoomOp::Out));
                            }
                            ui.add_space(10.0);
                        }
                        if self.mode == Mode::Board && self.center == super::workspace::Center::Board {
                            if icon_button(ui, ic::CORNERS_OUT, "Schermo intero (F)", vec2(22.0, 20.0), true).clicked() {
                                actions.push(Action::Fullscreen);
                            }
                            let rulers = if self.settings.rulers { "Nascondi i righelli" } else { "Mostra i righelli" };
                            if icon_button(ui, ic::RULER, rulers, vec2(22.0, 20.0), true).clicked() {
                                actions.push(Action::ToggleRulers);
                            }
                            ui.add_space(8.0);
                        }
                        // il messaggio di stato occupa lo spazio che resta
                        let room = ui.available_width();
                        if room > 40.0 && !self.status.is_empty() {
                            let color = if self.status.starts_with('⚠') { t.red } else { t.accent_text };
                            // un errore con i dettagli: clic per rileggerlo per intero
                            let details = self.status.starts_with('⚠') && self.error_box.is_some();
                            let sense = if details { Sense::click() } else { Sense::hover() };
                            let (r, resp) = ui.allocate_exact_size(vec2(room, 18.0), sense);
                            let clip = ui.painter().with_clip_rect(r);
                            // su una riga sola: gli a capo diventano « · »
                            let shown = i18n::tr(&self.status).replace('\n', " · ");
                            let g = clip.layout_no_wrap(shown, regular(11.0), color);
                            let wide = g.size().x > r.width() || self.status.contains('\n');
                            clip.galley(pos2(r.min.x, r.center().y - g.size().y / 2.0), g, color);
                            let resp = if details { resp.on_hover_cursor(egui::CursorIcon::PointingHand) } else { resp };
                            if resp.clicked() {
                                self.error_open = true;
                            }
                            if wide {
                                resp.on_hover_text(i18n::tr(&self.status).into_owned());
                            }
                        }
                    });
                });
            });
    }

    /// Un errore da leggere per intero (per esempio perché la trascrizione non
    /// è riuscita: il motivo di ogni agente), con la strada per sistemarlo.
    pub(super) fn error_window(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        if !self.error_open {
            return;
        }
        let Some((title, body)) = self.error_box.clone() else { return };
        let mut open = true;
        let mut close = false;
        egui::Window::new(format!("⚠ {}", i18n::tr(&title)))
            .id(egui::Id::new("error_box"))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(520.0)
            .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                    let mut text = i18n::tr(&body).into_owned();
                    ui.add(egui::TextEdit::multiline(&mut text).desired_width(f32::INFINITY).font(egui::TextStyle::Monospace).interactive(true));
                });
                ui.add_space(6.0);
                ui.label(egui::RichText::new(i18n::t("Controlla in Impostazioni AI che l'agente risulti «chiave trovata», che «legge le immagini» sia spuntato e premi «Prova»: la prova usa un'immagine, come la trascrizione.")).weak());
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(i18n::t("Impostazioni AI")).clicked() {
                        actions.push(Action::AiSettings);
                        close = true;
                    }
                    if ui.button(i18n::t("Copia")).clicked() {
                        ctx.copy_text(body.clone());
                    }
                    if ui.button(i18n::t("Chiudi")).clicked() {
                        close = true;
                    }
                });
            });
        if !open || close {
            self.error_open = false;
        }
    }

    /// Finestrella della ricerca dei comandi.
    pub(super) fn palette_ui(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        if self.palette.is_none() {
            return;
        }
        let mut cmds = self.commands();
        let Some(p) = &mut self.palette else { return };
        let t = Tokens::get(ctx);
        let anchor = self.palette_anchor;
        let screen = ctx.content_rect();
        let w = 420.0_f32.min(screen.width() - 20.0);
        let pos = if anchor.is_positive() && anchor.max.y < screen.max.y - 100.0 {
            pos2((anchor.min.x).min(screen.max.x - w - 10.0), anchor.max.y + 4.0)
        } else {
            pos2(screen.center().x - w / 2.0, screen.min.y + 60.0)
        };
        let q = p.query.to_lowercase();
        let words: Vec<&str> = q.split_whitespace().collect();
        // si cerca nella lingua dell'interfaccia (e anche in italiano)
        for c in &mut cmds {
            let shown = i18n::tr(&c.label).into_owned();
            if shown != c.label {
                c.label = shown;
            }
        }
        cmds.retain(|c| {
            let l = c.label.to_lowercase();
            words.iter().all(|w| l.contains(w))
        });
        let n = cmds.len();
        p.sel = p.sel.min(n.saturating_sub(1));
        let (mut run, mut close) = (None, false);
        let (down, up, enter, esc) = ctx.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                i.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
                i.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
            )
        });
        if down && n > 0 {
            p.sel = (p.sel + 1) % n;
        }
        if up && n > 0 {
            p.sel = (p.sel + n - 1) % n;
        }
        if enter && n > 0 && cmds[p.sel].enabled {
            run = Some(p.sel);
        }
        if esc {
            close = true;
        }
        let area = egui::Area::new(egui::Id::new("palette")).order(egui::Order::Foreground).fixed_pos(pos).show(ctx, |ui| {
            egui::Frame::popup(ui.style()).inner_margin(8).show(ui, |ui| {
                ui.set_width(w);
                let te = ui.add(
                    egui::TextEdit::singleline(&mut p.query)
                        .hint_text(i18n::tf(format!("{} Scrivi cosa vuoi fare (es. «esporta», «firma», «zoom»)", ic::MAGNIFYING_GLASS)))
                        .desired_width(f32::INFINITY),
                );
                if p.fresh {
                    te.request_focus();
                    p.fresh = false;
                }
                if te.changed() {
                    p.sel = 0;
                }
                ui.add_space(4.0);
                egui::ScrollArea::vertical().max_height(330.0).show(ui, |ui| {
                    if n == 0 {
                        ui.label(egui::RichText::new(i18n::t("Nessun comando trovato.")).weak());
                    }
                    for (i, c) in cmds.iter().enumerate() {
                        let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), if c.enabled { Sense::click() } else { Sense::hover() });
                        let on = i == p.sel;
                        if on || resp.hovered() {
                            ui.painter().rect_filled(r, 4.0, if on { t.checked } else { t.hover });
                        }
                        let col = if c.enabled { t.text } else { t.text_disabled };
                        ui.painter().text(pos2(r.min.x + 14.0, r.center().y), Align2::CENTER_CENTER, i18n::tr(&c.glyph).into_owned(), icon(15.0), if c.enabled { t.icon } else { t.text_disabled });
                        ui.painter().text(pos2(r.min.x + 30.0, r.center().y), Align2::LEFT_CENTER, i18n::tr(&&c.label).into_owned(), regular(12.5), col);
                        if !c.keys.is_empty() {
                            ui.painter().text(pos2(r.max.x - 8.0, r.center().y), Align2::RIGHT_CENTER, i18n::tr(&&c.keys).into_owned(), regular(11.0), t.text_dim);
                        }
                        if on && (down || up) {
                            resp.scroll_to_me(None);
                        }
                        if resp.clicked() {
                            run = Some(i);
                        }
                    }
                });
            });
        });
        // clic fuori: chiude
        let outside = ctx.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().is_some_and(|q| !area.response.rect.contains(q) && !anchor.contains(q)));
        if let Some(i) = run {
            if let Some(c) = cmds.into_iter().nth(i) {
                actions.push(c.act);
            }
            close = true;
        }
        if close || outside {
            self.palette = None;
        }
    }

    /// Tutti i comandi, per la ricerca.
    pub(super) fn commands(&self) -> Vec<Cmd> {
        use crate::scene::Tool;
        let doc = self.docui.doc.is_some();
        let c = |glyph: &'static str, label: &str, keys: &str, act: Action| Cmd { glyph, label: label.into(), keys: keys.into(), enabled: true, act };
        let mut v = vec![
            c(ic::PEN, "Penna", "", Action::SetTool(Tool::Pen)),
            c(ic::HIGHLIGHTER, "Evidenziatore", "H", Action::SetTool(Tool::Highlighter)),
            c(ic::ERASER, "Gomma (accendi/spegni)", "E", Action::ToggleEraser),
            c(ic::ARROW_U_UP_LEFT, "Annulla", &cmd_key("Z"), Action::Undo),
            c(ic::ARROW_U_UP_RIGHT, "Ripeti", &cmd_key("Shift+Z"), Action::Redo),
            c(ic::BROOM, "Pulisci il foglio (nuova pagina)", "C", Action::Clear),
            c(ic::HIGHLIGHTER, "Togli tutte le evidenziature", "Shift+H", Action::ClearHighlights),
            c(ic::PEN, "Evidenziature → scrittura", "", Action::HighlightsToInk),
            c(ic::PLUS, "Tratto più spesso", "+", Action::Width(0.4)),
            c(ic::MINUS, "Tratto più sottile", "-", Action::Width(-0.4)),
            c(ic::FLOPPY_DISK, "Salva la lavagna nell'archivio", &cmd_key("S"), Action::SaveBoard),
            c(ic::COPY, "Copia il foglio come immagine", &cmd_key("C"), Action::CopyImage),
            c(ic::CLIPBOARD_TEXT, "Incolla un'immagine (si apre come documento)", &cmd_key("V"), Action::Paste),
            c(ic::FILE_ARROW_UP, "Apri un documento da firmare (PDF, Word, immagini…)", &cmd_key("O"), Action::Doc(DocAction::OpenDialog)),
            c(ic::SIGNATURE, "Registra la firma", "", Action::Doc(DocAction::StartCapture)),
            c(ic::STAMP, "Inserisci la firma", "", Action::Doc(DocAction::InsertSignature)),
            c(ic::FILE_PDF, "Esporta la pagina in PDF", "", Action::Export(Fmt::Pdf)),
            c(ic::FILE_SVG, "Esporta la pagina in SVG", "", Action::Export(Fmt::Svg)),
            c(ic::FILE_PNG, "Esporta la pagina in PNG", "", Action::Export(Fmt::Png)),
            c(ic::FOLDER_OPEN, "Apri la cartella dei file salvati", "", Action::OpenExports),
            c(ic::MAGNIFYING_GLASS_PLUS, "Ingrandisci", &cmd_key("+"), Action::Zoom(ZoomOp::In)),
            c(ic::MAGNIFYING_GLASS_MINUS, "Riduci", &cmd_key("-"), Action::Zoom(ZoomOp::Out)),
            c(ic::ARROWS_IN, "Pagina intera", &cmd_key("0"), Action::Zoom(ZoomOp::Fit)),
            c(ic::ARROWS_CLOCKWISE, "Ruota il foglio di 90°", "R", Action::Rotate),
            c(ic::ARROWS_LEFT_RIGHT, "Specchia orizzontale", "X", Action::FlipX),
            c(ic::ARROWS_DOWN_UP, "Specchia verticale", "Y", Action::FlipY),
            c(ic::CORNERS_OUT, "Schermo intero", "F", Action::Fullscreen),
            c(ic::ARROWS_IN_SIMPLE, "Nascondi o mostra menu e strumenti", &cmd_key("Shift+F"), Action::ToggleCompact),
            c(ic::RULER, "Righelli (mostra/nascondi)", "", Action::ToggleRulers),
            c(ic::CHALKBOARD_SIMPLE, "Vai alla lavagna", "", Action::SetMode(Mode::Board)),
            c(ic::SQUARES_FOUR, "Vai all'archivio", "", Action::SetMode(Mode::Library)),
            c(ic::TEXT_AA, "Trascrivi la pagina in testo (Markdown)", "", Action::Ocr),
            c(ic::BOOK_OPEN, "Appunti (trascrizioni e chat salvate)", "", Action::Appunti),
            c(ic::CHATS_CIRCLE, "Chat AI (mostra/nascondi)", "", Action::Chat),
            c(ic::CHATS_CIRCLE, if doc { "Discuti il documento nella chat AI" } else { "Discuti il foglio nella chat AI" }, "", Action::ChatBoard),
            c(ic::CHECK_CIRCLE, "Fai controllare il foglio all'AI (errori, punti poco chiari)", "", Action::ChatCheck),
            c(ic::FILE_PLUS, "Nuovo appunto", "", Action::NoteCmd("new".into())),
            c(ic::SPARKLE, "Impostazioni AI (agenti, chiavi, contesto)", "", Action::AiSettings),
            c(ic::BLUETOOTH, "Collega lo Slate", "", Action::Connect),
            c(ic::PLAY, "Diretta dal pad (avvia/ferma)", "", Action::ToggleLive),
            c(ic::DOWNLOAD_SIMPLE, "Scarica le pagine dal pad", "", Action::Sync),
            c(ic::BROADCAST, "Avvia la diretta web (TV, telefono)", "", Action::Web),
            c(ic::GEAR_SIX, "Opzioni", &cmd_key(","), Action::OpenPrefs),
            c(ic::KEYBOARD, "Scorciatoie da tastiera", "", Action::ShowHelp(super::options::HelpPage::Tasti)),
            c(ic::INFO, "Informazioni su OpenSlate", "", Action::Backstage(Some(BsPage::Info))),
            c(ic::ARROW_CIRCLE_UP, "Cerca aggiornamenti", "", Action::CheckUpdates),
            c(ic::SIGN_OUT, "Esci", &cmd_key("Q"), Action::Quit),
        ];
        // gli altri formati della pagina
        for k in Fmt::MORE {
            v.push(Cmd { glyph: k.icon(), label: i18n::tf(format!("Esporta la pagina: {}", i18n::tr(k.label()))), keys: String::new(), enabled: true, act: Action::Export(k) });
        }
        // un comando per ogni foglio («Foglio: righe di 1ª elementare», …)
        for p in super::paper::Paper::ALL {
            let glyph = if p.ruled() || p == super::paper::Paper::Bianco { ic::NOTEBOOK } else { ic::GRID_FOUR };
            v.push(c(glyph, &format!("Foglio: {}", i18n::tr(&p.label())), "", Action::Paper(p)));
        }
        if !doc {
            v.push(c(ic::MAGIC_WAND, "Trasforma la lavagna in bella", "", Action::InBella));
        }
        if doc {
            v.push(c(ic::FILE_PDF, "Salva il documento firmato (nello stesso formato)", &cmd_key("Shift+S"), Action::Doc(DocAction::SaveSigned)));
            v.push(c(ic::X, "Chiudi il documento", &cmd_key("W"), Action::Doc(DocAction::Close)));
        }
        for d in crate::document::list().into_iter().take(8) {
            v.push(Cmd { glyph: ic::FILE_TEXT, label: format!("Apri «{}»", d.title), keys: String::new(), enabled: true, act: Action::Doc(DocAction::OpenRecent(d.id)) });
        }
        if self.ocr_busy {
            for c in v.iter_mut().filter(|c| matches!(c.act, Action::Ocr)) {
                c.enabled = false;
            }
        }
        v
    }
}

// --- righelli (da WordCraft, canvas.rs) ---

/// Spessore dei righelli.
pub(super) const RULER: f32 = 22.0;
/// Tacca di un'unità intera dove non c'è il numero.
const UNIT_TICK: f32 = 7.0;
/// Spazio minimo fra due numeri e fra due tacche (punti dello schermo).
const RULER_LABEL_PX: f32 = 30.0;
const RULER_TICK_PX: f32 = 4.0;
const RULER_SPARSEST: i32 = 1 << 20;
const RULER_MAX_MARKS: usize = 2000;

#[derive(Clone, Copy, Debug, PartialEq)]
enum RulerMark {
    Label(i32),
    Tick(f32),
}

/// Ogni quante unità un numero e ogni quante suddivisioni una tacca, perché
/// non si accavallino quando la pagina è piccola.
fn ruler_density(px_per_unit: f32, div: i32) -> (i32, i32) {
    let px = if px_per_unit.is_finite() { px_per_unit.max(0.0) } else { 0.0 };
    let every = |px: f32, room: f32| {
        let mut n = 1;
        while px * (n as f32) < room && n < RULER_SPARSEST {
            n *= 2;
        }
        n
    };
    (every(px, RULER_LABEL_PX), every(px / div.max(1) as f32, RULER_TICK_PX))
}

/// Tacche e numeri di un righello lungo `width` unità (cm), `scale` punti per
/// unità, `div` suddivisioni per unità.
fn ruler_marks(width: f32, div: i32, scale: f32) -> Vec<(f32, RulerMark)> {
    let div = div.max(2);
    let mut marks = Vec::new();
    if !(width.is_finite() && scale.is_finite() && scale > 0.0) {
        return marks;
    }
    let (label_every, tick_every) = ruler_density(scale, div);
    let per_label = div.saturating_mul(label_every);
    let mut k = tick_every;
    for _ in 0..RULER_MAX_MARKS {
        let d = k as f32 / div as f32;
        if d > width + 1e-3 {
            break;
        }
        let mark = if k % per_label == 0 && d < width - 0.3 {
            RulerMark::Label(k / div)
        } else if k % div == 0 {
            RulerMark::Tick(UNIT_TICK)
        } else if k % (div / 2) == 0 {
            RulerMark::Tick(5.0)
        } else {
            RulerMark::Tick(1.5)
        };
        marks.push((d, mark));
        k = k.saturating_add(tick_every);
    }
    marks
}

fn ruler_mark(p: &egui::Painter, at: Pos2, mark: RulerMark, horizontal: bool, t: &Tokens) {
    let len = match mark {
        RulerMark::Label(n) => {
            p.text(at, Align2::CENTER_CENTER, i18n::tr(&n.to_string()).into_owned(), regular(9.5), t.ruler_tick);
            return;
        }
        RulerMark::Tick(len) => len,
    };
    let (a, b) = if horizontal {
        (pos2(at.x, at.y - len / 2.0), pos2(at.x, at.y + len / 2.0))
    } else {
        (pos2(at.x - len / 2.0, at.y), pos2(at.x + len / 2.0, at.y))
    };
    p.line_segment([a, b], Stroke::new(1.0, t.ruler_tick));
}

/// Righello orizzontale (`h`) e verticale (`v`) attorno alla pagina, in cm.
/// `hover`: posizione del puntatore, segnata sui righelli.
pub(super) fn paint_rulers(painter: &egui::Painter, h: Rect, v: Rect, page: Rect, size_cm: (f32, f32), hover: Option<Pos2>, t: &Tokens) {
    let corner = Rect::from_min_max(pos2(v.min.x, h.min.y), pos2(v.max.x, h.max.y));
    for r in [corner, h, v] {
        painter.rect_filled(r, 0.0, t.canvas);
    }
    let (w_cm, h_cm) = size_cm;
    if w_cm <= 0.0 || h_cm <= 0.0 {
        return;
    }
    // orizzontale
    let hp = painter.with_clip_rect(h);
    let bar = Rect::from_min_max(pos2(page.min.x, h.min.y + 4.0), pos2(page.max.x, h.max.y - 3.0));
    hp.rect_filled(bar, 0.0, t.ruler);
    hp.rect_stroke(bar, 0.0, Stroke::new(1.0, t.ruler_margin), egui::StrokeKind::Inside);
    let sx = page.width() / w_cm;
    for (d, mark) in ruler_marks(w_cm, 4, sx) {
        ruler_mark(&hp, pos2(page.min.x + d * sx, bar.center().y), mark, true, t);
    }
    // verticale
    let vp = painter.with_clip_rect(v);
    let bar_v = Rect::from_min_max(pos2(v.min.x + 4.0, page.min.y), pos2(v.max.x - 3.0, page.max.y));
    vp.rect_filled(bar_v, 0.0, t.ruler);
    vp.rect_stroke(bar_v, 0.0, Stroke::new(1.0, t.ruler_margin), egui::StrokeKind::Inside);
    let sy = page.height() / h_cm;
    for (d, mark) in ruler_marks(h_cm, 4, sy) {
        ruler_mark(&vp, pos2(bar_v.center().x, page.min.y + d * sy), mark, false, t);
    }
    // dove sta il puntatore
    if let Some(p) = hover.filter(|p| page.contains(*p)) {
        hp.line_segment([pos2(p.x, bar.min.y), pos2(p.x, bar.max.y)], Stroke::new(1.0, t.accent));
        vp.line_segment([pos2(bar_v.min.x, p.y), pos2(bar_v.max.x, p.y)], Stroke::new(1.0, t.accent));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titolo_accorciato_senza_bloccarsi() {
        let w = |s: &str| s.chars().count() as f32 * 7.0;
        assert_eq!(fit_text("Lavagna — OpenSlate", 1000.0, w), "Lavagna — OpenSlate");
        assert_eq!(fit_text("Lavagna — OpenSlate", 70.0, w), "Lavagna —…");
        assert_eq!(fit_text("Lavagna — OpenSlate", 56.0, w), "Lavagna…");
        assert_eq!(fit_text("Lavagna", 5.0, w), "");
        assert_eq!(fit_text("Lavagna", -40.0, w), "");
    }

    #[test]
    fn righello_in_cm() {
        // A4 orizzontale a 20 punti per cm: numeri 1..29, tacche ogni 1/4 cm
        let m = ruler_marks(29.7, 4, 20.0);
        let labels: Vec<i32> = m.iter().filter_map(|(_, k)| if let RulerMark::Label(n) = k { Some(*n) } else { None }).collect();
        assert_eq!(labels.first(), Some(&2));
        assert!(labels.windows(2).all(|w| w[1] - w[0] == 2), "{labels:?}");
        // più grande: un numero per ogni cm
        let m = ruler_marks(21.0, 4, 40.0);
        let labels: Vec<i32> = m.iter().filter_map(|(_, k)| if let RulerMark::Label(n) = k { Some(*n) } else { None }).collect();
        assert_eq!(labels, (1..=20).collect::<Vec<_>>());
        // pagina minuscola: poche tacche, mai troppe
        assert!(ruler_marks(29.7, 4, 0.5).len() < 40);
        assert!(ruler_marks(1e9, 4, 1.0).len() <= RULER_MAX_MARKS);
    }
}
