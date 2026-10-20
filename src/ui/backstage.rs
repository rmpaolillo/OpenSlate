//! Pagina «File» a tutta finestra, come in Word: Home, Nuovo, Apri, Salva,
//! Esporta, Condividi, Opzioni, Informazioni, Esci.
//!
//! Disposizione ripresa da WordCraft (`backstage.rs`,
//! https://github.com/storytold/wordcraft, MIT OR Apache-2.0).

use crate::i18n;
use super::doc_view::DocAction;
use super::options::HelpPage;
use super::theme::{icon, medium, regular, semibold, Tokens, APP_COLOR};
use super::{Action, App, Mode};
use eframe::egui;
use egui::{pos2, vec2, Align2, Color32, Rect, Sense, Stroke};
use egui_phosphor::regular as ic;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BsPage {
    Home,
    Nuovo,
    Apri,
    Salva,
    Opzioni,
    Info,
}

const PAGES: [(BsPage, &str, &str); 4] = [
    (BsPage::Home, "Home", ic::HOUSE),
    (BsPage::Nuovo, "Nuovo", ic::FILE_PLUS),
    (BsPage::Apri, "Apri", ic::FOLDER_OPEN),
    (BsPage::Salva, "Salva", ic::FLOPPY_DISK),
];

/// Saluto secondo l'ora.
fn greeting() -> &'static str {
    use chrono::Timelike;
    match chrono::Local::now().hour() {
        5..=12 => "Buongiorno",
        13..=17 => "Buon pomeriggio",
        _ => "Buonasera",
    }
}

fn heading(ui: &mut egui::Ui, s: &str) {
    let t = Tokens::get(ui.ctx());
    ui.label(egui::RichText::new(i18n::tr(&s).into_owned()).font(semibold(24.0)).color(t.text));
    ui.add_space(14.0);
}

fn subheading(ui: &mut egui::Ui, s: &str) {
    let t = Tokens::get(ui.ctx());
    ui.add_space(10.0);
    ui.label(egui::RichText::new(i18n::tr(&s).into_owned()).font(semibold(14.0)).color(t.text));
    ui.add_space(6.0);
}

/// Riquadro cliccabile: icona, titolo e una riga di spiegazione.
fn card(ui: &mut egui::Ui, glyph: &str, title: &str, sub: &str, enabled: bool) -> bool {
    let t = Tokens::get(ui.ctx());
    let w = ui.available_width().min(520.0);
    let (r, resp) = ui.allocate_exact_size(vec2(w, 60.0), if enabled { Sense::click() } else { Sense::hover() });
    let fill = if enabled && resp.hovered() { t.hover } else { t.input };
    ui.painter().rect(r, 6.0, fill, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
    let (ic_c, tx, sx) = if enabled { (t.accent_text, t.text, t.text_dim) } else { (t.text_disabled, t.text_disabled, t.text_disabled) };
    ui.painter().text(pos2(r.min.x + 30.0, r.center().y), Align2::CENTER_CENTER, glyph, icon(24.0), ic_c);
    ui.painter().text(pos2(r.min.x + 58.0, r.min.y + 19.0), Align2::LEFT_CENTER, i18n::tr(&title).into_owned(), semibold(13.5), tx);
    let clip = ui.painter().with_clip_rect(r.shrink(4.0));
    clip.text(pos2(r.min.x + 58.0, r.min.y + 40.0), Align2::LEFT_CENTER, i18n::tr(&sub).into_owned(), regular(11.5), sx);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, title));
    ui.add_space(6.0);
    enabled && resp.clicked()
}

/// Riquadro «pagina» per Nuovo (anteprima del foglio sopra il nome).
fn tile(ui: &mut egui::Ui, glyph: &str, label: &str, blank: bool) -> bool {
    let t = Tokens::get(ui.ctx());
    let (r, resp) = ui.allocate_exact_size(vec2(132.0, 176.0), Sense::click());
    let page = Rect::from_min_size(r.min + vec2(6.0, 6.0), vec2(120.0, 140.0));
    if resp.hovered() {
        ui.painter().rect_filled(r, 6.0, t.hover);
    }
    ui.painter().rect_filled(page.translate(vec2(0.0, 2.0)), 3.0, t.page_shadow);
    ui.painter().rect(page, 3.0, if blank { Color32::WHITE } else { t.input }, Stroke::new(1.0, if resp.hovered() { t.accent } else { t.border_strong }), egui::StrokeKind::Inside);
    ui.painter().text(page.center(), Align2::CENTER_CENTER, glyph, icon(34.0), if blank { Color32::from_gray(190) } else { t.accent_text });
    ui.painter().text(pos2(r.center().x, page.max.y + 15.0), Align2::CENTER_CENTER, i18n::tr(&label).into_owned(), medium(12.0), t.text);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    resp.clicked()
}

impl App {
    /// La pagina File. Ritorna dopo averla disegnata al posto della lavagna.
    pub(super) fn backstage_ui(&mut self, root: &mut egui::Ui, page: BsPage, actions: &mut Vec<Action>) {
        let t = Tokens::get(root.ctx());
        let doc_open = self.docui.doc.is_some();
        egui::Panel::left("wsr_backstage_nav")
            .exact_size(200.0)
            .resizable(false)
            .frame(egui::Frame::NONE.fill(APP_COLOR).inner_margin(egui::Margin { left: 0, right: 0, top: 8, bottom: 12 }))
            .show(root, |ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                let (r, back) = ui.allocate_exact_size(vec2(200.0, 40.0), Sense::click());
                if back.hovered() {
                    ui.painter().rect_filled(r, 0.0, Color32::from_white_alpha(26));
                }
                ui.painter().text(pos2(r.min.x + 26.0, r.center().y), Align2::CENTER_CENTER, ic::ARROW_CIRCLE_LEFT, icon(22.0), Color32::WHITE);
                if back.on_hover_text(i18n::t("Torna al foglio (Esc)")).clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    actions.push(Action::Backstage(None));
                }
                ui.add_space(6.0);
                let nav = |ui: &mut egui::Ui, active: bool, glyph: &str, label: &str, h: f32, font: egui::FontId| -> bool {
                    let (r, resp) = ui.allocate_exact_size(vec2(200.0, h), Sense::click());
                    if active {
                        ui.painter().rect_filled(r, 0.0, Color32::from_white_alpha(46));
                    } else if resp.hovered() {
                        ui.painter().rect_filled(r, 0.0, Color32::from_white_alpha(26));
                    }
                    if !glyph.is_empty() {
                        ui.painter().text(pos2(r.min.x + 31.0, r.center().y), Align2::CENTER_CENTER, glyph, icon(18.0), Color32::WHITE);
                    }
                    ui.painter().text(pos2(r.min.x + 52.0, r.center().y), Align2::LEFT_CENTER, i18n::tr(&label).into_owned(), font, Color32::WHITE);
                    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
                    resp.clicked()
                };
                for (p, label, glyph) in PAGES {
                    let active = p == page;
                    if nav(ui, active, glyph, label, 38.0, if active { semibold(14.0) } else { medium(14.0) }) {
                        actions.push(Action::Backstage(Some(p)));
                    }
                }
                if doc_open && nav(ui, false, ic::X_SQUARE, "Chiudi", 38.0, medium(14.0)) {
                    actions.push(Action::Doc(DocAction::Close));
                    actions.push(Action::Backstage(None));
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    if nav(ui, false, ic::SIGN_OUT, "Esci", 34.0, regular(13.0)) {
                        actions.push(Action::Quit);
                    }
                    if nav(ui, page == BsPage::Info, ic::QUESTION, "Aiuto e info", 34.0, regular(13.0)) {
                        actions.push(Action::Backstage(Some(BsPage::Info)));
                    }
                    if nav(ui, page == BsPage::Opzioni, ic::GEAR_SIX, "Opzioni", 34.0, regular(13.0)) {
                        actions.push(Action::OpenPrefs);
                    }
                });
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(t.panel).inner_margin(egui::Margin { left: 40, right: 40, top: 28, bottom: 20 }))
            .show(root, |ui| {
                if page == BsPage::Opzioni {
                    self.prefs_panel(ui, actions);
                    return;
                }
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match page {
                    BsPage::Home => self.bs_home(ui, actions),
                    BsPage::Nuovo => self.bs_new(ui, actions),
                    BsPage::Apri => self.bs_open(ui, actions),
                    BsPage::Salva => self.bs_save(ui, actions),
                    BsPage::Info => self.bs_info(ui, actions),
                    BsPage::Opzioni => {}
                });
            });
    }

    fn new_tiles(&self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 14.0;
            if tile(ui, ic::PEN_NIB, "Lavagna vuota", true) {
                actions.push(Action::Doc(DocAction::TabBoard));
                actions.push(Action::SetMode(Mode::Board));
                actions.push(Action::Clear);
                actions.push(Action::Backstage(None));
            }
            if tile(ui, ic::FILE_ARROW_UP, "Documento da firmare", false) {
                actions.push(Action::Doc(DocAction::OpenDialog));
                actions.push(Action::Backstage(None));
            }
            if tile(ui, ic::SIGNATURE, "Registra la firma", false) {
                actions.push(Action::SetMode(Mode::Board));
                actions.push(Action::Doc(DocAction::StartCapture));
                actions.push(Action::Backstage(None));
            }
            if tile(ui, ic::SQUARES_FOUR, "Archivio", false) {
                actions.push(Action::SetMode(Mode::Library));
                actions.push(Action::Backstage(None));
            }
        });
    }

    fn recent_list(&self, ui: &mut egui::Ui, actions: &mut Vec<Action>, max: usize) {
        let t = Tokens::get(ui.ctx());
        let recent = crate::document::list();
        if recent.is_empty() {
            ui.label(egui::RichText::new(i18n::t("Nessun documento aperto finora. Apri un PDF o un file Word da firmare: lo ritrovi qui.")).color(t.text_dim));
            return;
        }
        let w = ui.available_width().min(720.0);
        for d in recent.iter().take(max) {
            let (r, resp) = ui.allocate_exact_size(vec2(w, 38.0), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(r, 4.0, t.hover);
            }
            let open = self.docui.doc.as_ref().is_some_and(|x| x.id == d.id);
            ui.painter().text(pos2(r.min.x + 18.0, r.center().y), Align2::CENTER_CENTER, ic::FILE_TEXT, icon(18.0), t.accent_text);
            ui.painter().with_clip_rect(r.shrink(2.0)).text(
                pos2(r.min.x + 40.0, r.center().y),
                Align2::LEFT_CENTER,
                i18n::tr(&if open { format!("{}  (aperto)", d.title) } else { d.title.clone() }).into_owned(),
                medium(13.0),
                t.text,
            );
            ui.painter().text(pos2(r.max.x - 10.0, r.center().y), Align2::RIGHT_CENTER, i18n::tr(&d.created.get(..16).unwrap_or(&d.created).replace('T', " ")).into_owned(), regular(11.5), t.text_dim);
            if resp.on_hover_text(i18n::t("Apri sulla lavagna")).clicked() {
                actions.push(Action::SetMode(Mode::Board));
                actions.push(Action::Doc(DocAction::OpenRecent(d.id.clone())));
                actions.push(Action::Backstage(None));
            }
        }
    }

    fn bs_home(&self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        heading(ui, greeting());
        subheading(ui, "Nuovo");
        self.new_tiles(ui, actions);
        subheading(ui, "Documenti recenti");
        self.recent_list(ui, actions, 8);
    }

    fn bs_new(&self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        heading(ui, "Nuovo");
        self.new_tiles(ui, actions);
        ui.add_space(8.0);
        let t = Tokens::get(ui.ctx());
        ui.label(egui::RichText::new(i18n::t("«Lavagna vuota» pulisce il foglio (Cmd+Z per tornare indietro). Salva prima nell'archivio se vuoi conservarlo.")).color(t.text_dim));
    }

    fn bs_open(&self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        heading(ui, "Apri");
        if card(ui, ic::FOLDER_OPEN, "Sfoglia…", "PDF, Word (.doc, .docx), .odt, .rtf, .md, .txt, .html — anche trascinando il file sulla finestra", !self.docui.importing) {
            actions.push(Action::Doc(DocAction::OpenDialog));
            actions.push(Action::Backstage(None));
        }
        if card(ui, ic::SQUARES_FOUR, "Archivio delle pagine", &format!("{} pagine scritte sul pad o salvate dalla lavagna", self.lib.entries.len()), true) {
            actions.push(Action::SetMode(Mode::Library));
            actions.push(Action::Backstage(None));
        }
        subheading(ui, "Documenti recenti");
        self.recent_list(ui, actions, 50);
    }

    fn bs_save(&self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        heading(ui, "Salva");
        let doc = self.docui.doc.as_ref();
        if card(
            ui,
            ic::FILE_PDF,
            "Salva il documento firmato",
            &match doc {
                Some(d) => format!("«{}» con quello che hai scritto, nello stesso formato ({}), aperto subito", d.title, d.signed_ext()),
                None => "Apri prima un documento da firmare".into(),
            },
            doc.is_some(),
        ) {
            actions.push(Action::Doc(DocAction::SaveSigned));
            actions.push(Action::Backstage(None));
        }
        if card(ui, ic::FLOPPY_DISK, "Salva la lavagna nell'archivio", "Il foglio diventa una pagina dell'archivio (Cmd+S)", doc.is_none()) {
            actions.push(Action::SaveBoard);
            actions.push(Action::Backstage(None));
        }
        if card(ui, ic::FILE_MAGNIFYING_GLASS, "Apri l'ultimo PDF salvato", "In Anteprima", self.docui.last_export.is_some()) {
            actions.push(Action::Doc(DocAction::OpenLastExport));
        }
    }

    fn bs_info(&self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let t = Tokens::get(ui.ctx());
        heading(ui, "Aiuto e informazioni");
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(64.0, 64.0), Sense::hover());
            ui.painter().rect_filled(r, 14.0, APP_COLOR);
            ui.painter().text(r.center(), Align2::CENTER_CENTER, ic::PEN_NIB, icon(36.0), Color32::WHITE);
            ui.add_space(10.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(i18n::t("OpenSlate")).font(semibold(20.0)).color(t.text));
                ui.label(egui::RichText::new(i18n::tf(format!("OpenSlate {} — licenza {}", env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_LICENSE")))).color(t.text_dim));
                ui.label(egui::RichText::new(i18n::tr(&env!("CARGO_PKG_AUTHORS")).into_owned()).color(t.text_dim));
            });
        });
        ui.add_space(12.0);
        ui.label(i18n::t("App libera per Wacom Bamboo Slate: lavagna in diretta, documenti da firmare, archivio, trascrizione della scrittura a mano e chat AI."));
        subheading(ui, "Aiuto");
        if card(ui, ic::KEYBOARD, "Scorciatoie da tastiera", "Tutti i tasti dell'app", true) {
            actions.push(Action::ShowHelp(HelpPage::Tasti));
        }
        if card(ui, ic::BLUETOOTH, "Come si collega lo Slate", "Prima volta, LED, Inkspace", true) {
            actions.push(Action::ShowHelp(HelpPage::Collegamento));
        }
        if card(ui, ic::BOOK_OPEN_TEXT, "Guida completa", "README.md", true) {
            actions.push(Action::OpenReadme);
        }
        if card(ui, ic::FOLDER_OPEN, "Cartella dei dati", "Archivio, documenti, export, impostazioni", true) {
            actions.push(Action::OpenDataFolder);
        }
        subheading(ui, "Crediti");
        for line in [
            "Protocollo Bluetooth del pad tradotto da Tuhi (GPL).",
            "Interfaccia in stile Word ripresa da WordCraft di ArtCraft (MIT / Apache-2.0).",
            "Carattere Inter di Rasmus Andersson (SIL Open Font License 1.1). Icone Phosphor (MIT).",
            "Formule: KaTeX (MIT). Markdown: marked (MIT). Interfaccia: egui (MIT / Apache-2.0).",
        ] {
            ui.label(egui::RichText::new(i18n::tr(&line).into_owned()).color(t.text_dim));
        }
    }
}
