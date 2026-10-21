//! Tutto in una finestra: l'appunto al centro (come un documento di Word,
//! modificabile come in Typora), la chat AI nel riquadro a destra, le
//! impostazioni AI al centro; la barra «Slate non collegato» con i passi.

use crate::i18n;
use super::chrome::Tab;
use super::embed::{js_str, Slot};
use super::theme::{icon, medium, regular, semibold, Tokens};
use super::widgets::{big, big_ex, column, group, small, small_ex};
use super::{Action, App};
use crate::actor::Link;
use eframe::egui;
use egui::{pos2, vec2, Align2, Sense, Stroke};
use egui_phosphor::regular as ic;

/// Cosa c'è al centro della finestra.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Center {
    Board,
    Note,
    Settings,
}

/// Stato dell'appunto aperto, come lo dice la pagina.
#[derive(Clone, Debug, Default)]
pub(super) struct NoteInfo {
    pub words: usize,
    pub dirty: bool,
    pub source: bool,
    pub zoom: f32,
}

/// Comandi della scheda «Appunto» (eseguiti dalla pagina).
pub(super) fn note_js(cmd: &str) -> String {
    format!("window.wsNote && wsNote.cmd({})", js_str(cmd))
}

impl App {
    /// Indirizzo di una pagina del server interno, con il tema giusto.
    pub(super) fn page_url(&self, page: &str) -> Option<String> {
        let port = self.local_port?;
        let sep = if page.contains('?') { '&' } else { '?' };
        Some(format!("http://127.0.0.1:{port}/{page}{sep}embed=1&theme={}&zoom={:.2}", self.settings.ui_theme, self.settings.chat_zoom))
    }

    fn origin(&self) -> String {
        format!("http://127.0.0.1:{}/", self.local_port.unwrap_or(0))
    }

    /// Apre un appunto al centro (`None`: l'ultimo aperto, con l'elenco).
    pub(super) fn open_note(&mut self, name: Option<String>) {
        if self.local_port.is_none() || self.embeds.failed.is_some() {
            // niente WebView incorporate: la finestra separata di prima
            self.open_appunti_window(name.as_deref());
            return;
        }
        let list = name.is_none();
        if let Some(n) = name {
            if self.embeds.has(Slot::Note) && self.note.as_deref() != Some(n.as_str()) {
                self.embeds.eval(Slot::Note, &format!("window.wsNote && wsNote.open({})", js_str(&n)));
            }
            self.note = Some(n);
            self.note_info = NoteInfo { zoom: 1.0, ..Default::default() };
        }
        if list {
            self.note_nav = true;
            self.embeds.eval(Slot::Note, &note_js("navshow"));
        }
        self.show_note_center();
    }

    /// L'appunto al centro, con la sua scheda.
    pub(super) fn show_note_center(&mut self) {
        self.center = Center::Note;
        self.tab = Tab::Appunto;
        self.show_ribbon = true;
        self.mode = super::Mode::Board;
        self.backstage = None;
        // l'appunto della chat lo aggiorna sync_chat_note (al prossimo fotogramma)
    }

    pub(super) fn close_note(&mut self) {
        self.embeds.eval(Slot::Note, "window.wsNote && wsNote.flush()");
        self.note = None;
        if self.center == Center::Note {
            self.center = Center::Board;
        }
        if self.tab == Tab::Appunto {
            self.tab = Tab::Home;
        }
        // resta quello della pagina sulla lavagna, se ne ha uno
        self.sync_chat_note();
    }

    /// Mostra/nasconde la chat a destra.
    pub(super) fn toggle_chat(&mut self, open: bool) {
        if self.local_port.is_none() || self.embeds.failed.is_some() {
            self.open_page("chat?nuova=1");
            return;
        }
        self.chat_open = open;
        if open {
            self.embeds.focus(Slot::Chat);
        }
    }

    /// Messaggio da una pagina incorporata.
    pub(super) fn on_page_msg(&mut self, slot: Slot, v: serde_json::Value, actions: &mut Vec<Action>) {
        let s = |k: &str| v[k].as_str().unwrap_or("").to_string();
        match v["cmd"].as_str().unwrap_or("") {
            "note_open" if slot == Slot::Note => {
                let n = s("name");
                if !n.is_empty() && self.note.as_deref() != Some(n.as_str()) {
                    self.note = Some(n.clone());
                }
            }
            // l'appunto si è salvato: se si stava uscendo, ora si chiude
            "note_flushed" => self.note_flushed(),
            "note_state" => {
                self.note_info.words = v["words"].as_u64().unwrap_or(0) as usize;
                self.note_info.dirty = v["dirty"].as_bool().unwrap_or(false);
            }
            "note_source" => self.note_info.source = v["on"].as_bool().unwrap_or(false),
            "chat_zoom" => {
                self.settings.chat_zoom = (v["zoom"].as_f64().unwrap_or(1.0) as f32).clamp(0.6, 2.5);
                self.settings.save();
            }
            "note_zoom" => self.note_info.zoom = v["zoom"].as_f64().unwrap_or(1.0) as f32,
            "note_reload" => self.embeds.eval(Slot::Note, "window.wsNote && wsNote.reload()"),
            "open_note" => {
                let n = s("name");
                self.open_note(if n.is_empty() { None } else { Some(n) });
            }
            "status" => self.status = s("text"),
            "chat_title" => self.chat_title = s("title"),
            "attach_board" => {
                self.set_chat_context_from_board();
                self.embeds.eval(Slot::Chat, "window.wsChat && wsChat.attachNow()");
            }
            "ocr" => actions.push(Action::Ocr),
            "close_pane" => self.chat_open = false,
            // il pulsante lasciato sopra la chat l'ha visto la pagina, non la finestra
            "drop_here" => {
                if let Some(w) = self.drag_what.take() {
                    actions.push(Action::DropOnChat(w));
                }
            }
            "open_settings" => actions.push(Action::AiSettings),
            "close_settings" => {
                // impostazioni aperte dentro il riquadro della chat (versione
                // vecchia della pagina): nel riquadro torna la chat
                if slot == Slot::Chat {
                    // l'appunto lo manda sync_chat_note quando la pagina è pronta
                    let note = String::new();
                    self.chat_note_key = None;
                    self.chat_note_sent = None;
                    if let Some(url) = self.page_url(&format!("chat?pane=1&note={}", super::url_escape(&note))) {
                        self.embeds.load(Slot::Chat, &url);
                    }
                }
                actions.push(Action::CloseSettings);
            }
            "config_saved" => self.embeds.eval(Slot::Chat, "window.wsChat && wsChat.reloadConfig()"),
            _ => {}
        }
    }

    /// Una scheda (lavagna o documento) trascinata verso la chat AI: accanto
    /// al puntatore cosa si porta, sopra la chat la chat dice «lascia qui» e,
    /// lasciandola, il foglio o il documento vanno nel contesto della chat.
    pub(super) fn chat_drop(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        use super::doc_view::ChatDrag;
        let payload = egui::DragAndDrop::payload::<ChatDrag>(ctx);
        // sopra la chat il puntatore può «sparire» per la finestra (la pagina
        // della chat sta sopra): vale l'ultimo punto visto, vicino al bordo
        let seen = ctx.pointer_latest_pos();
        if payload.is_none() {
            // trascinamento finito (comunque sia finito): via il «lascia qui»
            if self.drag_last_pos.take().is_some() {
                self.chat_drop_hint = false;
                self.embeds.eval(Slot::Chat, "window.wsChat && wsChat.dropHint && wsChat.dropHint(false)");
            }
        } else if seen.is_some() {
            self.drag_last_pos = seen;
        }
        let pos = seen.or(self.drag_last_pos);
        // trascinando con la chat chiusa: si apre, così c'è dove lasciarlo
        if payload.is_some() && !self.chat_open && self.local_port.is_some() && self.embeds.failed.is_none() {
            self.chat_open = true;
        }
        let over = payload.is_some() && pos.is_some_and(|p| self.chat_rect.is_some_and(|r| r.expand(if seen.is_none() { 24.0 } else { 0.0 }).contains(p)));
        if over != self.chat_drop_hint {
            self.chat_drop_hint = over;
            self.embeds.eval(Slot::Chat, &format!("window.wsChat && wsChat.dropHint && wsChat.dropHint({over})"));
        }
        let Some(what) = payload else {
            self.drag_what = None;
            return;
        };
        self.drag_what = Some((*what).clone());
        if let Some(p) = pos.filter(|_| !over) {
            let t = Tokens::get(ctx);
            let (glyph, name) = match &*what {
                ChatDrag::Board => (ic::CHALKBOARD_SIMPLE, "Lavagna".to_string()),
                ChatDrag::Doc(_, title) => (ic::FILE_TEXT, title.clone()),
            };
            let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("wsr_chat_drag")));
            let text = format!("{glyph}  {name}  →  chat AI");
            let galley = painter.layout_no_wrap(i18n::tf(text), medium(12.0), t.text);
            let r = egui::Rect::from_min_size(p + vec2(14.0, 12.0), galley.size() + vec2(18.0, 10.0));
            painter.rect(r, 6.0, t.menu, Stroke::new(1.0, t.border_strong), egui::StrokeKind::Inside);
            painter.galley(r.min + vec2(9.0, 5.0), galley, t.text);
        }
        if ctx.input(|i| i.pointer.any_released()) {
            if over {
                if let Some(w) = egui::DragAndDrop::take_payload::<ChatDrag>(ctx) {
                    actions.push(Action::DropOnChat((*w).clone()));
                }
            }
            if self.chat_drop_hint {
                self.chat_drop_hint = false;
                self.embeds.eval(Slot::Chat, "window.wsChat && wsChat.dropHint && wsChat.dropHint(false)");
            }
        }
    }

    /// Chat a destra (prima del centro: i pannelli laterali vanno messi prima).
    pub(super) fn chat_pane(&mut self, root: &mut egui::Ui, frame: &eframe::Frame) {
        let want = self.chat_open && !self.fullscreen && self.local_port.is_some() && self.embeds.failed.is_none();
        if !want {
            self.chat_rect = None;
            return;
        }
        let t = Tokens::get(root.ctx());
        if !self.embeds.has(Slot::Chat) {
            let note = String::new();
            self.chat_note_key = None;
            self.chat_note_sent = None;
            let cmd = self.chat_cmd_pending.take().map(|c| format!("&cmd={c}")).unwrap_or_default();
            if let Some(url) = self.page_url(&format!("chat?pane=1&note={}{cmd}", super::url_escape(&note))) {
                let origin = self.origin();
                self.embeds.ensure(Slot::Chat, frame, &origin, &url, t.panel);
            }
        }
        // a sinistra (di serie) o a destra; larga più del foglio, si allarga col bordo
        let left = self.settings.chat_side != "destra";
        let total = root.available_rect_before_wrap().width();
        let max = (total - 280.0).max(340.0);
        let want_w = (self.settings.chat_width * total).clamp(320.0, max);
        let panel = if left { egui::Panel::left("ai_pane_sx") } else { egui::Panel::right("ai_pane_dx") };
        let r = panel
            .resizable(true)
            .default_size(want_w)
            .size_range(320.0..=max)
            .frame(egui::Frame::NONE.fill(t.panel).stroke(Stroke::new(1.0, t.border)))
            .show(root, |ui| {
                let r = ui.max_rect();
                ui.allocate_rect(r, Sense::hover());
                if !self.embeds.has(Slot::Chat) {
                    ui.painter().text(r.center(), Align2::CENTER_CENTER, i18n::t("Apro la chat…"), regular(12.0), t.text_dim);
                }
                r
            });
        // un filo libero dal lato del bordo che allarga il riquadro
        self.chat_rect = Some(if left { r.inner.with_max_x(r.inner.max.x - 4.0) } else { r.inner.with_min_x(r.inner.min.x + 4.0) });
        // la larghezza scelta trascinando il bordo resta per la prossima volta
        let w = r.response.rect.width();
        let dragging = root.ctx().input(|i| i.pointer.any_down());
        if !dragging && total > 400.0 && (w / total - self.settings.chat_width).abs() > 0.01 {
            self.settings.chat_width = (w / total).clamp(0.2, 0.85);
            self.settings.save();
        }
    }

    /// Centro: appunto o impostazioni AI (pagine incorporate).
    pub(super) fn web_center(&mut self, root: &mut egui::Ui, frame: &eframe::Frame) {
        let t = Tokens::get(root.ctx());
        let slot = if self.center == Center::Settings { Slot::Settings } else { Slot::Note };
        if !self.embeds.has(slot) {
            let page = match slot {
                Slot::Settings => "impostazioni".to_string(),
                _ => {
                    let f = self.note.clone().unwrap_or_default();
                    let cmd = self.note_cmd_pending.take().map(|c| format!("&cmd={c}")).unwrap_or_default();
                    let nav = self.note_nav || (f.is_empty() && cmd.is_empty());
                    format!("appunti?f={}{}{cmd}", super::url_escape(&f), if nav { "&nav=1" } else { "" })
                }
            };
            if let Some(url) = self.page_url(&page) {
                let origin = self.origin();
                self.embeds.ensure(slot, frame, &origin, &url, t.canvas);
            }
        }
        let r = egui::CentralPanel::default().frame(egui::Frame::NONE.fill(t.canvas)).show(root, |ui| {
            let r = ui.max_rect();
            ui.allocate_rect(r, Sense::hover());
            if !self.embeds.has(slot) {
                let msg = self.embeds.failed.clone().map(|e| format!("Pagina non disponibile: {e}")).unwrap_or_else(|| "Apro…".into());
                ui.painter().text(r.center(), Align2::CENTER_CENTER, i18n::tr(&msg).into_owned(), regular(12.5), t.text_dim);
            }
            r
        });
        self.center_rect = Some((slot, r.inner));
    }

    /// Mette le pagine al loro posto; le nasconde sotto menu, finestre e
    /// pagina File (una WebView sta sempre sopra a quello che disegna egui).
    pub(super) fn place_pages(&mut self, ctx: &egui::Context) {
        let covers: Vec<egui::Rect> = ctx.memory(|m| {
            m.areas()
                .visible_layer_ids()
                .into_iter()
                .filter(|l| matches!(l.order, egui::Order::Foreground | egui::Order::Middle))
                .filter_map(|l| m.area_rect(l.id))
                .collect()
        });
        let free = |r: Option<egui::Rect>| r.filter(|r| !covers.iter().any(|c| c.intersects(*r)));
        let back = self.backstage.is_some();
        self.embeds.set_origin(ctx.input(|i| i.viewport().inner_rect.map(|r| r.min)));
        let center = self.center_rect.take();
        let mut note = None;
        let mut settings = None;
        if !back {
            match center {
                Some((Slot::Note, r)) => note = free(Some(r)),
                Some((Slot::Settings, r)) => settings = free(Some(r)),
                _ => {}
            }
        }
        let chat = if back { None } else { free(self.chat_rect) };
        self.embeds.place(Slot::Note, note);
        self.embeds.place(Slot::Settings, settings);
        self.embeds.place(Slot::Chat, chat);
        // clic sulla parte egui: la tastiera torna all'app
        let pressed = ctx.input(|i| i.pointer.any_pressed().then(|| i.pointer.interact_pos()).flatten());
        if let Some(p) = pressed {
            let inside = [note, settings, chat].iter().flatten().any(|r| r.contains(p));
            if !inside {
                self.embeds.focus_parent();
            }
        }
    }

    /// Chiede a GitHub, in un thread, se c'è una versione nuova.
    pub(super) fn check_updates(&mut self, manual: bool) {
        self.update_next = None;
        if manual {
            self.status = i18n::t("Cerco aggiornamenti…").into();
        }
        let tx = self.evt_tx.clone();
        std::thread::spawn(move || {
            let result = crate::update::check().map_err(|e| format!("{e:#}"));
            let _ = tx.send(crate::actor::UiEvent::Update { result, manual });
        });
    }

    /// Scarica e installa la nuova versione (in un thread), come il launcher di ArtCraft.
    pub(super) fn install_update(&mut self) {
        let Some(r) = self.update.clone() else { return };
        let Some(install) = crate::selfupdate::current() else {
            crate::platform::open(&r.download);
            return;
        };
        if self.update_phase.is_some() {
            return;
        }
        self.update_error = None;
        self.update_phase = Some(crate::selfupdate::Phase::Downloading { done: 0, total: 0 });
        let tx = self.evt_tx.clone();
        std::thread::spawn(move || {
            let res = crate::selfupdate::prepare(&install, &r, &mut |p| {
                let _ = tx.send(crate::actor::UiEvent::UpdatePhase(p));
            });
            let _ = tx.send(crate::actor::UiEvent::UpdateDone(res.map_err(|e| format!("{e:#}"))));
        });
    }

    /// Barra degli aggiornamenti (come nel launcher di ArtCraft): «È disponibile…»
    /// con «Aggiorna», poi l'avanzamento, poi «Riavvia ora».
    pub(super) fn update_banner(&mut self, root: &mut egui::Ui, actions: &mut Vec<Action>) {
        let Some(r) = self.update.clone() else { return };
        if self.update_hidden {
            return;
        }
        let t = Tokens::get(root.ctx());
        let (fill, edge) = if t.dark {
            (egui::Color32::from_rgb(0x1d, 0x2a, 0x42), egui::Color32::from_rgb(0x2f, 0x4a, 0x7a))
        } else {
            (egui::Color32::from_rgb(0xE9, 0xF0, 0xFD), egui::Color32::from_rgb(0xBF, 0xD3, 0xF7))
        };
        let can_self = crate::selfupdate::current().is_some();
        let phase = self.update_phase.clone();
        let ready = self.update_ready.clone();
        let error = self.update_error.clone();
        egui::Panel::top("update_banner")
            .exact_size(30.0)
            .frame(egui::Frame::NONE.fill(fill).stroke(Stroke::new(1.0, edge)).inner_margin(egui::Margin { left: 14, right: 10, top: 2, bottom: 2 }))
            .show(root, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    let glyph = if ready.is_some() { ic::CHECK_CIRCLE } else if error.is_some() { ic::WARNING } else { ic::ARROW_CIRCLE_UP };
                    ui.label(egui::RichText::new(glyph).font(icon(15.0)).color(if error.is_some() { t.red } else { t.accent_text }));
                    let keep = i18n::t("Le impostazioni, gli agenti AI, l'archivio e i documenti restano: stanno nella cartella dei dati, fuori dall'app, e le chiavi nel Portachiavi.");
                    let text = if let Some(rd) = &ready {
                        if rd.on_exit {
                            i18n::tf(format!("OpenSlate {} si installa quando chiudi l'app.", rd.version))
                        } else {
                            i18n::tf(format!("OpenSlate {} è installato: si apre col prossimo avvio.", rd.version))
                        }
                    } else if let Some(p) = &phase {
                        use crate::selfupdate::Phase;
                        match p {
                            Phase::Downloading { done, total } if *total > 0 => {
                                i18n::tf(format!("Aggiorno OpenSlate a {}: scarico {:.0} di {:.0} MB…", r.version, *done as f64 / 1e6, *total as f64 / 1e6))
                            }
                            Phase::Downloading { .. } => i18n::tf(format!("Aggiorno OpenSlate a {}: scarico…", r.version)),
                            Phase::Verifying => i18n::tf(format!("Aggiorno OpenSlate a {}: controllo il file…", r.version)),
                            Phase::Installing => i18n::tf(format!("Aggiorno OpenSlate a {}: installo…", r.version)),
                        }
                    } else if let Some(e) = &error {
                        i18n::tf(format!("Aggiornamento a OpenSlate {} non riuscito: {}", r.version, e.lines().next().unwrap_or("")))
                    } else {
                        i18n::tf(format!("È disponibile OpenSlate {} (questa è la {}).", r.version, crate::update::current()))
                    };
                    let lbl = ui.add(egui::Label::new(egui::RichText::new(text).font(medium(11.5)).color(t.text)).truncate());
                    match &error {
                        Some(e) => lbl.on_hover_text(i18n::tr(e).into_owned()),
                        None => lbl.on_hover_text(keep),
                    };
                    if let Some(crate::selfupdate::Phase::Downloading { done, total }) = &phase {
                        if *total > 0 {
                            ui.add(egui::ProgressBar::new(*done as f32 / *total as f32).desired_width(120.0).desired_height(8.0));
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let primary = |ui: &mut egui::Ui, label: String| ui.add(egui::Button::new(egui::RichText::new(label).color(egui::Color32::WHITE)).fill(t.accent).small());
                        if phase.is_some() {
                            return; // mentre scarica e installa: niente pulsanti
                        }
                        if super::widgets::icon_button(ui, ic::X, "Più tardi (torna al prossimo avvio)", vec2(20.0, 20.0), true).clicked() {
                            self.update_hidden = true;
                        }
                        if let Some(rd) = &ready {
                            if !rd.on_exit
                                && primary(ui, i18n::tf(format!("{}  Riavvia ora", ic::ARROW_CLOCKWISE)))
                                    .on_hover_text(i18n::t("Chiude OpenSlate e lo riapre aggiornato (quello che hai scritto è già salvato)."))
                                    .clicked()
                            {
                                actions.push(Action::RestartForUpdate);
                            }
                            return;
                        }
                        if error.is_some() && ui.small_button(i18n::tf(format!("{}  Scarica", ic::DOWNLOAD_SIMPLE))).clicked() {
                            crate::platform::open(&r.download);
                        }
                        if error.is_none() && ui.small_button(i18n::t("Salta questa versione")).clicked() {
                            actions.push(Action::SkipUpdate);
                        }
                        if ui.small_button(i18n::t("Novità")).clicked() {
                            crate::platform::open(&r.page);
                        }
                        if can_self {
                            let label = if error.is_some() { format!("{}  {}", ic::ARROW_CLOCKWISE, i18n::t("Riprova")) } else { format!("{}  {}", ic::ARROW_CIRCLE_UP, i18n::t("Aggiorna")) };
                            if primary(ui, label)
                                .on_hover_text(i18n::t("Scarica la nuova versione, controlla il file e la installa al posto di questa. Impostazioni e archivio restano."))
                                .clicked()
                            {
                                actions.push(Action::InstallUpdate);
                            }
                        } else {
                            let dl = primary(ui, i18n::tf(format!("{}  Scarica", ic::DOWNLOAD_SIMPLE)));
                            if dl.on_hover_text(i18n::t("Scarica la nuova versione dal sito di GitHub: si installa come la prima volta, sostituendo l'app. Impostazioni e archivio restano.")).clicked() {
                                crate::platform::open(&r.download);
                            }
                        }
                    });
                });
            });
    }

    /// Barra sotto la barra multifunzione quando lo Slate non è collegato:
    /// cosa fare, passo per passo.
    pub(super) fn connect_banner(&mut self, root: &mut egui::Ui, actions: &mut Vec<Action>) {
        if self.link == Link::On {
            self.banner_hidden = false;
            return;
        }
        if self.banner_hidden || self.fullscreen || self.center != Center::Board {
            return;
        }
        let t = Tokens::get(root.ctx());
        let connecting = self.link == Link::Connecting;
        // pad già registrato: basta una riga sottile
        let known = std::path::Path::new("peripheral_id").exists();
        if known && !connecting {
            let fill = if t.dark { egui::Color32::from_rgb(0x3a, 0x33, 0x1c) } else { egui::Color32::from_rgb(0xFF, 0xF4, 0xCE) };
            egui::Panel::top("connect_banner_thin")
                .exact_size(28.0)
                .frame(egui::Frame::NONE.fill(fill).inner_margin(egui::Margin { left: 14, right: 10, top: 2, bottom: 2 }))
                .show(root, |ui| {
                    ui.horizontal_centered(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        ui.label(egui::RichText::new(ic::BLUETOOTH).font(icon(14.0)).color(t.accent_text));
                        ui.label(egui::RichText::new(i18n::t("Slate non collegato: premi il pulsante rotondo del pad, poi «Collega».")).font(regular(11.5)).color(t.text));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if super::widgets::icon_button(ui, ic::X, "Nascondi (torna se lo Slate si scollega)", vec2(20.0, 20.0), true).clicked() {
                                self.banner_hidden = true;
                            }
                            if ui.small_button(i18n::tf(format!("{}  Collega", ic::BLUETOOTH))).clicked() {
                                actions.push(Action::Connect);
                            }
                        });
                    });
                });
            return;
        }
        let fill = if t.dark { egui::Color32::from_rgb(0x3a, 0x33, 0x1c) } else { egui::Color32::from_rgb(0xFF, 0xF4, 0xCE) };
        let edge = if t.dark { egui::Color32::from_rgb(0x6b, 0x5a, 0x1e) } else { egui::Color32::from_rgb(0xF0, 0xD8, 0x8A) };
        egui::Panel::top("connect_banner")
            .exact_size(if connecting { 34.0 } else { 56.0 })
            .frame(egui::Frame::NONE.fill(fill).stroke(Stroke::new(1.0, edge)).inner_margin(egui::Margin { left: 14, right: 10, top: 4, bottom: 4 }))
            .show(root, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    let (r, _) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::hover());
                    let glyph = if connecting { ic::BLUETOOTH } else { ic::INFO };
                    ui.painter().text(r.center(), Align2::CENTER_CENTER, glyph, icon(18.0), if connecting { t.orange } else { t.accent_text });
                    // il testo va a capo nello spazio lasciato dai pulsanti
                    let text_w = (ui.available_width() - if connecting { 40.0 } else { 260.0 }).max(120.0);
                    ui.allocate_ui_with_layout(vec2(text_w, ui.available_height()), egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.set_width(text_w);
                        ui.spacing_mut().item_spacing.y = 1.0;
                        if connecting {
                            ui.add(egui::Label::new(egui::RichText::new(i18n::t("Collegamento allo Slate in corso…  Se il LED del pad è spento, premi una volta il pulsante rotondo.")).font(medium(12.0)).color(t.text)).wrap());
                        } else {
                            ui.label(egui::RichText::new(i18n::t("Lo Slate non è collegato.")).font(semibold(12.5)).color(t.text));
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(
                                        i18n::t("1) Premi una volta il pulsante rotondo del pad (il LED si accende).  2) Clic su «Collega».  \
                                         Prima volta, o dopo Inkspace: tieni premuto il pulsante ~6 s finché il LED lampeggia blu, poi «Collega» e premi il pulsante quando richiesto."),
                                    )
                                    .font(regular(11.5))
                                    .color(t.text_dim),
                                )
                                .wrap(),
                            );
                        }
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if super::widgets::icon_button(ui, ic::X, "Nascondi (torna se lo Slate si scollega)", vec2(22.0, 22.0), true).clicked() {
                            self.banner_hidden = true;
                        }
                        if !connecting {
                            if ui.button(i18n::t("Come si collega")).clicked() {
                                actions.push(Action::ShowHelp(super::options::HelpPage::Collegamento));
                            }
                            let b = egui::Button::new(egui::RichText::new(i18n::tf(format!("{}  Collega", ic::BLUETOOTH))).color(t.on_accent)).fill(t.accent);
                            if ui.add(b).clicked() {
                                actions.push(Action::Connect);
                            }
                        }
                    });
                });
            });
    }

    /// Scheda contestuale «Appunto» (con un appunto al centro).
    pub(super) fn ribbon_appunto(&self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let n = |c: &str| Action::NoteCmd(c.to_string());
        group(ui, "Appunti", |ui| {
            if big(ui, ic::FILE_PLUS, "Nuovo", "Un appunto vuoto").clicked() {
                actions.push(n("new"));
            }
            if big_ex(ui, ic::LIST_BULLETS, "Elenco", "Tutti gli appunti (trascrizioni e chat salvate)", true, self.note_nav, false).clicked() {
                actions.push(n("nav"));
            }
            let r = big_ex(ui, ic::EXPORT, "Esporta", "Salva l'appunto dove vuoi: Markdown (con le figure), Word o PDF", self.note.is_some(), false, true);
            egui::Popup::menu(&r).show(|ui| {
                if super::chrome::menu_item(ui, true, ic::MARKDOWN_LOGO, "Markdown (.md)", "") {
                    actions.push(Action::NoteExport("md"));
                }
                if super::chrome::menu_item(ui, true, ic::MICROSOFT_WORD_LOGO, "Word (.docx)", "") {
                    actions.push(Action::NoteExport("docx"));
                }
                if super::chrome::menu_item(ui, true, ic::FILE_PDF, "PDF", "") {
                    actions.push(Action::NoteExport("pdf"));
                }
            });
            column(ui, |ui| {
                if small(ui, ic::COPY, "Copia Markdown", "Per un'altra chat AI o per Typora").clicked() {
                    actions.push(n("copy"));
                }
                if small(ui, ic::PRINTER, "Stampa / PDF", "Stampa, oppure «Salva come PDF» dal dialogo di stampa").clicked() {
                    actions.push(Action::NotePrint);
                }
                if small_ex(ui, ic::ARROW_SQUARE_OUT, "Apri in Typora", "", self.note.is_some(), false).clicked() {
                    actions.push(Action::NoteTypora);
                }
            });
        });
        group(ui, "Paragrafo", |ui| {
            column(ui, |ui| {
                if small(ui, ic::TEXT_H_ONE, "Titolo 1", "").clicked() {
                    actions.push(n("h1"));
                }
                if small(ui, ic::TEXT_H_TWO, "Titolo 2", "").clicked() {
                    actions.push(n("h2"));
                }
                if small(ui, ic::TEXT_T, "Testo normale", "").clicked() {
                    actions.push(n("p"));
                }
            });
            column(ui, |ui| {
                if small(ui, ic::LIST_BULLETS, "Elenco puntato", "").clicked() {
                    actions.push(n("ul"));
                }
                if small(ui, ic::LIST_NUMBERS, "Elenco numerato", "").clicked() {
                    actions.push(n("ol"));
                }
                if small(ui, ic::QUOTES, "Citazione", "").clicked() {
                    actions.push(n("quote"));
                }
            });
        });
        group(ui, "Carattere", |ui| {
            column(ui, |ui| {
                if small(ui, ic::TEXT_B, "Grassetto", "Cmd+B").clicked() {
                    actions.push(n("bold"));
                }
                if small(ui, ic::TEXT_ITALIC, "Corsivo", "Cmd+I").clicked() {
                    actions.push(n("italic"));
                }
                if small(ui, ic::MINUS, "Linea", "Separatore orizzontale").clicked() {
                    actions.push(n("hr"));
                }
            });
        });
        group(ui, "Inserisci", |ui| {
            if big(ui, ic::TABLE, "Tabella", "Tabella Markdown").clicked() {
                actions.push(n("table"));
            }
            column(ui, |ui| {
                if small(ui, ic::FUNCTION, "Formula", "Formula su una riga sua ($$ … $$)").clicked() {
                    actions.push(n("mathblock"));
                }
                if small(ui, ic::MATH_OPERATIONS, "Formula nel testo", "Cmd+M").clicked() {
                    actions.push(n("math"));
                }
            });
        });
        group(ui, "Vista", |ui| {
            if big_ex(ui, ic::CODE, "Sorgente", "Tutto il Markdown (Cmd+/)", true, self.note_info.source, false).clicked() {
                actions.push(n("source"));
            }
        });
    }

    /// Scheda dell'appunto o delle impostazioni nella striscia dei file.
    pub(super) fn web_tabs(&self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let t = Tokens::get(ui.ctx());
        let mut tab = |ui: &mut egui::Ui, on: bool, glyph: &str, text: &str, close: Action, open: Action| {
            let text = i18n::tr(text);
            let text: &str = &text;
            let font = if on { semibold(12.0) } else { medium(12.0) };
            let tw = ui.painter().layout_no_wrap(text.to_string(), font.clone(), t.text).size().x;
            let (r, resp) = ui.allocate_exact_size(vec2(tw + 60.0, 27.0), Sense::click());
            let round = egui::CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 };
            if on {
                ui.painter().rect_filled(r, round, t.input);
                ui.painter().rect_filled(egui::Rect::from_min_size(r.min + vec2(6.0, 0.0), vec2(r.width() - 12.0, 2.5)), 1.0, t.accent);
            } else if resp.hovered() {
                ui.painter().rect_filled(r, round, t.hover);
            }
            ui.painter().text(pos2(r.min.x + 16.0, r.center().y), Align2::CENTER_CENTER, glyph, icon(14.0), if on { t.accent_text } else { t.text_dim });
            ui.painter().text(pos2(r.min.x + 28.0, r.center().y), Align2::LEFT_CENTER, i18n::tr(&text).into_owned(), font, if on { t.text } else { t.text_dim });
            let x = egui::Rect::from_center_size(pos2(r.max.x - 14.0, r.center().y), vec2(18.0, 18.0));
            let xr = ui.interact(x, resp.id.with("close"), Sense::click());
            if xr.hovered() {
                ui.painter().rect_filled(x, 4.0, t.pressed);
            }
            ui.painter().text(x.center(), Align2::CENTER_CENTER, ic::X, icon(12.0), t.text_dim);
            if xr.on_hover_text(i18n::t("Chiudi")).clicked() {
                actions.push(close);
            } else if resp.clicked() && !on {
                actions.push(open);
            }
        };
        if self.note.is_none() && self.center == Center::Note {
            tab(ui, true, ic::NOTE, "Appunti", Action::CloseNote, Action::OpenNote(None));
        }
        if let Some(n) = &self.note {
            let title = n.trim_end_matches(".md");
            let short: String = if title.chars().count() > 34 { format!("{}…", title.chars().take(33).collect::<String>()) } else { title.to_string() };
            tab(ui, self.center == Center::Note, ic::NOTE, &short, Action::CloseNote, Action::OpenNote(Some(n.clone())));
        }
        if self.settings_open {
            tab(ui, self.center == Center::Settings, ic::GEAR_SIX, "Impostazioni AI", Action::CloseSettings, Action::AiSettings);
        }
    }
}
