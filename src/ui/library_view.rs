//! Scheda «Archivio»: griglia di anteprime raggruppate per giorno, selezione
//! multipla, menu col tasto destro, visualizzatore a pagina intera.

use crate::i18n;
use super::zoom::ZoomView;
use super::{paint_page, zoom_controls, zoom_keys, Action, App, Fmt, Mode};
use crate::library;
use crate::scene::{Page, View};
use eframe::egui;
use egui_phosphor::regular as ic;
use std::collections::{BTreeSet, HashMap};

/// Altezza delle anteprime: limiti del cursore «Anteprime».
pub(super) const THUMB_MIN: f32 = 160.0;
pub(super) const THUMB_MAX: f32 = 600.0;
/// Spazio sotto l'anteprima per titolo e sottotitolo.
const CARD_TEXT_H: f32 = 44.0;
/// Anteprime nuove disegnate per fotogramma (l'interfaccia resta fluida).
const THUMBS_PER_FRAME: usize = 6;

pub(super) struct LibUi {
    /// Altezza delle anteprime in punti (cursore «Anteprime», salvata).
    pub thumb_size: f32,
    pub selected: BTreeSet<String>,
    /// Ultima pagina cliccata (per la selezione con Shift).
    anchor: Option<String>,
    /// Pagina aperta nel visualizzatore.
    pub viewer: Option<String>,
    /// Rinomina in corso: (id, testo).
    rename: Option<(String, String)>,
    /// Il campo della rinomina ha già ricevuto il focus.
    rename_focused: bool,
    search: String,
    /// Anteprime: (vista, pixel di altezza, texture).
    pub thumbs: HashMap<String, (View, u32, egui::TextureHandle)>,
    /// Pagine appena spostate nel cestino (per «Ripristina»).
    pub last_trashed: Vec<String>,
    /// Pagine ancora nel pad (se noto).
    pub pad_files: Option<usize>,
    /// Zoom del visualizzatore (si azzera cambiando pagina).
    pub viewer_zoom: ZoomView,
}

impl LibUi {
    pub fn new(thumb_size: f32) -> Self {
        Self {
            thumb_size: thumb_size.clamp(THUMB_MIN, THUMB_MAX),
            selected: BTreeSet::new(),
            anchor: None,
            viewer: None,
            rename: None,
            rename_focused: false,
            search: String::new(),
            thumbs: HashMap::new(),
            last_trashed: Vec::new(),
            pad_files: None,
            viewer_zoom: ZoomView::default(),
        }
    }

    /// Larghezza e altezza di una scheda (anteprima + testo).
    fn card_size(&self) -> (f32, f32, f32) {
        let h = self.thumb_size;
        let w = (h * 0.76).round();
        (w, h, h + CARD_TEXT_H)
    }
}

pub(super) enum LibAction {
    Select { id: String, cmd: bool, shift: bool },
    SelectAll,
    SelectNone,
    Open(String),
    Close,
    Step(i32),
    ToBoard(String),
    Export(Vec<String>, Fmt),
    Copy(String),
    Ocr(Vec<String>),
    Trash(Vec<String>),
    Restore,
    StartRename(String),
    CommitRename,
    CancelRename,
}

fn lib(a: LibAction) -> Action {
    Action::Lib(a)
}

impl App {
    /// Id delle pagine visibili (filtro di ricerca), nell'ordine dell'archivio.
    fn visible_ids(&self) -> Vec<String> {
        let q = self.lui.search.trim().to_lowercase();
        self.lib
            .entries
            .iter()
            .filter(|e| {
                q.is_empty()
                    || e.meta.display_title().to_lowercase().contains(&q)
                    || e.meta.created.contains(&q)
            })
            .map(|e| e.id.clone())
            .collect()
    }

    fn lib_pages(&self, ids: &[String]) -> Vec<(String, Page)> {
        let view = self.view();
        // In ordine cronologico (dalla più vecchia), come le pagine di un quaderno.
        let mut v: Vec<&library::Entry> = ids.iter().filter_map(|id| self.lib.get(id)).collect();
        v.sort_by(|a, b| a.meta.created.cmp(&b.meta.created));
        v.into_iter()
            .map(|e| {
                let mut p = Page::from_drawing(&e.drawing, view);
                // il grafico dell'AI fa parte della pagina (export, trascrizione)
                p.bg = e.chart.clone();
                (e.meta.display_title(), p)
            })
            .collect()
    }

    /// Le pagine da esportare, con la loro trascrizione (se il foglio è
    /// ancora quello trascritto).
    fn lib_items(&self, ids: &[String]) -> Vec<crate::formats::Item> {
        let view = self.view();
        let mut v: Vec<&library::Entry> = ids.iter().filter_map(|id| self.lib.get(id)).collect();
        v.sort_by(|a, b| a.meta.created.cmp(&b.meta.created));
        v.into_iter()
            .map(|e| {
                let mut page = Page::from_drawing(&e.drawing, view);
                page.bg = e.chart.clone();
                let sig = crate::stroke::strokes_sig(&e.drawing.strokes);
                let note = super::ocr_note_for(&format!("lib:{}", e.id), sig)
                    .map(|n| std::path::Path::new(crate::export::EXPORT_DIR).join(n))
                    .filter(|p| p.exists());
                crate::formats::Item { title: e.meta.display_title(), page, note }
            })
            .collect()
    }

    fn selection(&self) -> Vec<String> {
        self.lui.selected.iter().cloned().collect()
    }

    pub(super) fn apply_lib(&mut self, a: LibAction, ctx: &egui::Context) {
        match a {
            LibAction::Select { id, cmd, shift } => {
                if shift {
                    let ids = self.visible_ids();
                    let a = self.lui.anchor.as_ref().and_then(|x| ids.iter().position(|i| i == x));
                    let b = ids.iter().position(|i| *i == id);
                    if let (Some(a), Some(b)) = (a, b) {
                        let (lo, hi) = (a.min(b), a.max(b));
                        if !cmd {
                            self.lui.selected.clear();
                        }
                        self.lui.selected.extend(ids[lo..=hi].iter().cloned());
                        return;
                    }
                }
                if cmd {
                    if !self.lui.selected.remove(&id) {
                        self.lui.selected.insert(id.clone());
                    }
                } else {
                    self.lui.selected = [id.clone()].into_iter().collect();
                }
                self.lui.anchor = Some(id);
            }
            LibAction::SelectAll => self.lui.selected = self.visible_ids().into_iter().collect(),
            LibAction::SelectNone => self.lui.selected.clear(),
            LibAction::Open(id) => {
                self.lui.selected = [id.clone()].into_iter().collect();
                self.lui.viewer = Some(id);
                self.lui.viewer_zoom.fit();
            }
            LibAction::Close => self.lui.viewer = None,
            LibAction::Step(d) => {
                let ids = self.visible_ids();
                if let Some(cur) = &self.lui.viewer {
                    if let Some(i) = ids.iter().position(|x| x == cur) {
                        let j = (i as i32 + d).clamp(0, ids.len() as i32 - 1) as usize;
                        let id = ids[j].clone();
                        self.lui.selected = [id.clone()].into_iter().collect();
                        self.lui.viewer = Some(id);
                        self.lui.viewer_zoom.fit();
                    }
                }
            }
            LibAction::ToBoard(id) => {
                if self.lib.get(&id).is_none() {
                    return;
                }
                // un documento aperto si chiude (salvato): la pagina va sulla
                // lavagna, non sopra il documento
                if self.docui.doc.is_some() {
                    self.apply_doc(super::doc_view::DocAction::TabBoard);
                }
                // quello che c'era sulla lavagna e non era nell'archivio ci va ora
                let mut saved_prev = None;
                if self.board_unsaved() {
                    match self.save_board_to_library() {
                        Ok(prev) => saved_prev = Some(prev),
                        Err(e) => {
                            self.status = e;
                            return;
                        }
                    }
                }
                if let Some(e) = self.lib.get(&id) {
                    let d = e.drawing.clone();
                    let chart = e.chart.clone();
                    let t = e.meta.display_title();
                    self.hub.edit(|s| s.load(d, chart));
                    // è già nell'archivio: uscendo non c'è da chiedere
                    self.mark_board_saved();
                    self.board_from = Some(t.clone());
                    self.board_from_id = Some(id.clone());
                    self.mode = Mode::Board;
                    // la chat vede subito questa pagina (e la sua trascrizione, se c'è)
                    self.set_chat_context_from_board();
                    self.status = if saved_prev.is_some() {
                        format!("«{t}» sulla lavagna; quello che c'era prima è nell'archivio.")
                    } else {
                        format!("«{t}» sulla lavagna: puoi scriverci sopra (Annulla per tornare indietro).")
                    };
                }
            }
            LibAction::Export(ids, fmt) => {
                let items = self.lib_items(&ids);
                self.export_pages(items, fmt);
            }
            LibAction::Copy(id) => {
                if let Some((_, p)) = self.lib_pages(&[id]).pop() {
                    self.copy_page(p);
                }
            }
            LibAction::Ocr(ids) => {
                let pages = self.lib_pages(&ids);
                self.ocr_pages(pages);
            }
            LibAction::Trash(ids) => {
                if ids.is_empty() {
                    return;
                }
                // Nel visualizzatore: dopo l'eliminazione si passa alla pagina accanto.
                let next = self.lui.viewer.as_ref().and_then(|cur| {
                    let vis = self.visible_ids();
                    let i = vis.iter().position(|x| x == cur)?;
                    vis.iter()
                        .skip(i + 1)
                        .chain(vis[..i].iter().rev())
                        .find(|x| !ids.contains(x))
                        .cloned()
                });
                match self.lib.trash(&ids) {
                    Ok(n) => {
                        for id in &ids {
                            self.lui.selected.remove(id);
                            self.lui.thumbs.remove(id);
                        }
                        if self.lui.viewer.is_some() {
                            self.lui.viewer = next.clone();
                            if let Some(n) = next {
                                self.lui.selected = [n].into_iter().collect();
                            }
                        }
                        self.lui.last_trashed = ids;
                        self.status = if n == 1 {
                            "Pagina spostata nel cestino (library/cestino).".to_string()
                        } else {
                            format!("{n} pagine spostate nel cestino (library/cestino).")
                        };
                    }
                    Err(e) => self.status = format!("⚠ cestino: {e}"),
                }
            }
            LibAction::Restore => {
                let ids = std::mem::take(&mut self.lui.last_trashed);
                match self.lib.restore(&ids) {
                    Ok(n) => {
                        self.status = if n == 1 { "Pagina ripristinata.".into() } else { format!("{n} pagine ripristinate.") };
                        self.lui.selected = ids.into_iter().collect();
                    }
                    Err(e) => self.status = format!("⚠ ripristino: {e}"),
                }
            }
            LibAction::StartRename(id) => {
                let t = self.lib.get(&id).map(|e| e.meta.title.clone()).unwrap_or_default();
                self.lui.rename = Some((id, t));
                self.lui.rename_focused = false;
            }
            LibAction::CommitRename => {
                if let Some((id, t)) = self.lui.rename.take() {
                    if let Err(e) = self.lib.rename(&id, &t) {
                        self.status = format!("⚠ rinomina: {e}");
                    }
                }
            }
            LibAction::CancelRename => self.lui.rename = None,
        }
        let _ = ctx;
    }

    fn lib_keyboard(&self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        if ctx.egui_wants_keyboard_input() || self.lui.rename.is_some() {
            return;
        }
        if let Some(op) = zoom_keys(ctx) {
            actions.push(Action::Zoom(op));
            return;
        }
        let select_all = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::A));
        ctx.input(|i| {
            let sel = self.selection();
            if let Some(cur) = &self.lui.viewer {
                if i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::ArrowDown) {
                    actions.push(lib(LibAction::Step(1)));
                }
                if i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::ArrowUp) {
                    actions.push(lib(LibAction::Step(-1)));
                }
                if i.key_pressed(egui::Key::Escape) {
                    actions.push(lib(LibAction::Close));
                }
                if i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace) {
                    actions.push(lib(LibAction::Trash(vec![cur.clone()])));
                }
                if i.events.iter().any(|e| matches!(e, egui::Event::Copy)) {
                    actions.push(lib(LibAction::Copy(cur.clone())));
                }
                return;
            }
            if select_all {
                actions.push(lib(LibAction::SelectAll));
            }
            if i.key_pressed(egui::Key::Escape) {
                actions.push(lib(LibAction::SelectNone));
            }
            if (i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)) && !sel.is_empty() {
                actions.push(lib(LibAction::Trash(sel.clone())));
            }
            if (i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space)) && sel.len() == 1 {
                actions.push(lib(LibAction::Open(sel[0].clone())));
            }
            let copy_evt = i.events.iter().any(|e| matches!(e, egui::Event::Copy));
            if copy_evt && sel.len() == 1 {
                actions.push(lib(LibAction::Copy(sel[0].clone())));
            }
        });
    }

    /// Prepara le anteprime mancanti (poche per fotogramma).
    fn make_thumbs(&mut self, ctx: &egui::Context, ids: &[String]) {
        let view = self.view();
        // pixel necessari (schermi Retina: ppp = 2)
        let need = (self.lui.thumb_size * ctx.pixels_per_point()).round() as u32;
        let mut made = 0;
        for id in ids {
            if made >= THUMBS_PER_FRAME {
                ctx.request_repaint();
                break;
            }
            // Si rifà solo se cambia la vista o la dimensione cambia molto.
            if matches!(self.lui.thumbs.get(id), Some((v, px, _))
                if *v == view && *px as f32 >= need as f32 * 0.8 && *px as f32 <= need as f32 * 1.6)
            {
                continue;
            }
            let Some(e) = self.lib.get(id) else { continue };
            let mut page = Page::from_drawing(&e.drawing, view);
            page.bg = e.chart.clone();
            // tratti un po' più marcati: in piccolo restano leggibili
            for s in &mut page.strokes {
                s.width *= 1.6;
            }
            // un'anteprima che non si fa resta bianca (non si riprova a ogni fotogramma)
            let img = match crate::export::page_to_rgba(&page, need) {
                Ok((w, h, px)) => egui::ColorImage::from_rgba_unmultiplied([w, h], &px),
                Err(e) => {
                    tracing::warn!("anteprima di {id}: {e}");
                    egui::ColorImage::new([2, 3], vec![egui::Color32::WHITE; 6])
                }
            };
            let tex = ctx.load_texture(format!("thumb-{id}"), img, egui::TextureOptions::LINEAR);
            self.lui.thumbs.insert(id.clone(), (view, need, tex));
            made += 1;
        }
    }

    pub(super) fn library_ui(&mut self, root: &mut egui::Ui, actions: &mut Vec<Action>) {
        let ctx_owned = root.ctx().clone();
        let ctx = &ctx_owned;
        self.lib_keyboard(ctx, actions);
        let viewer = self.lui.viewer.clone().filter(|id| self.lib.get(id).is_some());
        if viewer.is_none() {
            self.lui.viewer = None;
        }

        egui::CentralPanel::default().show(root, |ui| match &viewer {
            Some(id) => {
                let rect = ui.available_rect_before_wrap();
                let resp = ui.interact(rect, egui::Id::new("viewer-page"), egui::Sense::click_and_drag());
                match self.lui.viewer_zoom.handle_input(ui, &resp, true) {
                    1 => actions.push(Action::RotateBy(3)),
                    -1 => actions.push(Action::RotateBy(1)),
                    _ => {}
                }
                let view = self.view();
                let chart = self.lib.get(id).and_then(|e| e.chart.clone());
                // il grafico dell'AI salvato con la pagina, sotto i tratti
                let bg = chart.and_then(|c| {
                    let tex = self.bg_texture(ctx, &c)?;
                    Some(super::BgImage { tex, paper: true, rect: c.rect, baked: false, keep: None, detail: None })
                });
                if let Some(e) = self.lib.get(id) {
                    paint_page(&ui.painter_at(rect), rect, &e.drawing, view, bg, &mut self.lui.viewer_zoom, None);
                }
            }
            None => self.grid(ui, ctx, actions),
        });

        self.rename_window(ctx, actions);
    }

    /// Scheda «Archivio» della barra: comandi della griglia o del visualizzatore.
    pub(super) fn library_ribbon(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let viewer = self.lui.viewer.clone().filter(|id| self.lib.get(id).is_some());
        ui.horizontal(|ui| {
            match self.lui.pad_files {
                Some(0) => ui.label(egui::RichText::new(i18n::tf(format!("{} Pad: nessuna pagina da scaricare", ic::DEVICE_TABLET))).weak()),
                Some(n) => ui.label(egui::RichText::new(i18n::tf(format!("{} Sul pad: {n} pagine da scaricare", ic::DEVICE_TABLET))).strong()),
                None => ui.label(i18n::t("")),
            };
            if ui.button(i18n::tf(format!("{} Scarica dal pad", ic::DOWNLOAD_SIMPLE))).on_hover_text(i18n::t("Scarica nell'archivio le pagine scritte senza Mac")).clicked() {
                actions.push(Action::Sync);
            }
        });
        match &viewer {
            Some(id) => self.viewer_toolbar(ui, id, actions),
            None => self.grid_toolbar(ui, actions),
        }
    }

    fn grid_toolbar(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let sel = self.selection();
        let ocr_busy = self.ocr_busy;
        let LibUi { search, thumb_size, .. } = &mut self.lui;
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::TextEdit::singleline(search)
                    .hint_text(i18n::tf(format!("{} Cerca per titolo o data", ic::MAGNIFYING_GLASS)))
                    .desired_width(180.0),
            );
            if ui.button(ic::CHECK_SQUARE).on_hover_text(i18n::t("Seleziona tutto (Cmd+A)")).clicked() {
                actions.push(lib(LibAction::SelectAll));
            }
            ui.label(ic::IMAGES).on_hover_text(i18n::t("Dimensione delle anteprime"));
            ui.add(egui::Slider::new(thumb_size, THUMB_MIN..=THUMB_MAX).show_value(false))
                .on_hover_text(i18n::t("Dimensione delle anteprime"));
            ui.separator();
            if sel.is_empty() {
                ui.label(i18n::t("Clic = seleziona • doppio clic = apri • tasto destro = altre azioni"));
                return;
            }
            let label = if sel.len() == 1 { "1 selezionata".to_string() } else { format!("{} selezionate", sel.len()) };
            ui.label(egui::RichText::new(i18n::tr(&label).into_owned()).strong());
            if sel.len() == 1 {
                if ui.button(i18n::tf(format!("{} Apri", ic::CORNERS_OUT))).clicked() {
                    actions.push(lib(LibAction::Open(sel[0].clone())));
                }
                if ui.button(i18n::tf(format!("{} Sulla lavagna", ic::CHALKBOARD_SIMPLE))).clicked() {
                    actions.push(lib(LibAction::ToBoard(sel[0].clone())));
                }
                if ui.button(i18n::tf(format!("{} Rinomina", ic::PENCIL_SIMPLE))).clicked() {
                    actions.push(lib(LibAction::StartRename(sel[0].clone())));
                }
                if ui.button(i18n::tf(format!("{} Copia immagine", ic::COPY))).on_hover_text(i18n::t("Per incollarla in una chat AI")).clicked() {
                    actions.push(lib(LibAction::Copy(sel[0].clone())));
                }
            }
            ui.label(i18n::tf(format!("{} Esporta:", ic::EXPORT)));
            if ui.button(i18n::tf(format!("{} PDF", ic::FILE_PDF))).on_hover_text(i18n::t("Un unico PDF con tutte le pagine selezionate")).clicked() {
                actions.push(lib(LibAction::Export(sel.clone(), Fmt::Pdf)));
            }
            if ui.button(i18n::tf(format!("{} PNG", ic::FILE_PNG))).clicked() {
                actions.push(lib(LibAction::Export(sel.clone(), Fmt::Png)));
            }
            if ui.button(i18n::tf(format!("{} SVG", ic::FILE_SVG))).clicked() {
                actions.push(lib(LibAction::Export(sel.clone(), Fmt::Svg)));
            }
            super::chrome::more_formats_menu(ui, |k| actions.push(lib(LibAction::Export(sel.clone(), k))));
            if ui.add_enabled(!ocr_busy, egui::Button::new(i18n::tf(format!("{} Trascrivi", ic::MATH_OPERATIONS)))).clicked() {
                actions.push(lib(LibAction::Ocr(sel.clone())));
            }
            if ui.button(i18n::tf(format!("{} Elimina", ic::TRASH))).on_hover_text(i18n::t("Sposta nel cestino (recuperabile)")).clicked() {
                actions.push(lib(LibAction::Trash(sel.clone())));
            }
            if ui.button(ic::SELECTION_SLASH).on_hover_text(i18n::t("Deseleziona (Esc)")).clicked() {
                actions.push(lib(LibAction::SelectNone));
            }
        });
    }

    fn viewer_toolbar(&self, ui: &mut egui::Ui, id: &str, actions: &mut Vec<Action>) {
        let Some(e) = self.lib.get(id) else { return };
        let ids = self.visible_ids();
        let pos = ids.iter().position(|x| x == id).unwrap_or(0);
        ui.horizontal_wrapped(|ui| {
            if ui.button(i18n::tf(format!("{} Archivio", ic::ARROW_LEFT))).on_hover_text(i18n::t("Esc")).clicked() {
                actions.push(lib(LibAction::Close));
            }
            ui.separator();
            if ui.add_enabled(pos > 0, egui::Button::new(i18n::tf(format!("{} Precedente", ic::CARET_LEFT)))).clicked() {
                actions.push(lib(LibAction::Step(-1)));
            }
            ui.label(i18n::tf(format!("Pagina {} di {}", pos + 1, ids.len())));
            if ui.add_enabled(pos + 1 < ids.len(), egui::Button::new(i18n::tf(format!("Successiva {}", ic::CARET_RIGHT)))).clicked() {
                actions.push(lib(LibAction::Step(1)));
            }
            ui.separator();
            ui.label(egui::RichText::new(i18n::tr(&e.meta.display_title()).into_owned()).strong());
            if !e.meta.title.is_empty() {
                ui.label(egui::RichText::new(i18n::tr(&&e.meta.created).into_owned()).weak());
            }
            ui.label(egui::RichText::new(i18n::tf(format!("{} tratti", e.drawing.strokes.len()))).weak());
            ui.separator();
            zoom_controls(ui, &self.lui.viewer_zoom, actions);
        });
        ui.horizontal_wrapped(|ui| {
            let one = vec![id.to_string()];
            if ui.button(i18n::tf(format!("{} Rinomina", ic::PENCIL_SIMPLE))).clicked() {
                actions.push(lib(LibAction::StartRename(id.to_string())));
            }
            if ui.button(i18n::tf(format!("{} Sulla lavagna", ic::CHALKBOARD_SIMPLE))).on_hover_text(i18n::t("Per mostrarla in diretta o scriverci sopra")).clicked() {
                actions.push(lib(LibAction::ToBoard(id.to_string())));
            }
            if ui.button(i18n::tf(format!("{} Copia immagine", ic::COPY))).on_hover_text(i18n::t("Cmd+C, poi Cmd+V nella chat AI")).clicked() {
                actions.push(lib(LibAction::Copy(id.to_string())));
            }
            ui.separator();
            ui.label(i18n::tf(format!("{} Esporta:", ic::EXPORT)));
            if ui.button(i18n::tf(format!("{} PDF", ic::FILE_PDF))).clicked() {
                actions.push(lib(LibAction::Export(one.clone(), Fmt::Pdf)));
            }
            if ui.button(i18n::tf(format!("{} PNG", ic::FILE_PNG))).clicked() {
                actions.push(lib(LibAction::Export(one.clone(), Fmt::Png)));
            }
            if ui.button(i18n::tf(format!("{} SVG", ic::FILE_SVG))).clicked() {
                actions.push(lib(LibAction::Export(one.clone(), Fmt::Svg)));
            }
            super::chrome::more_formats_menu(ui, |k| actions.push(lib(LibAction::Export(one.clone(), k))));
            if ui.add_enabled(!self.ocr_busy, egui::Button::new(i18n::tf(format!("{} Trascrivi", ic::MATH_OPERATIONS)))).clicked() {
                actions.push(lib(LibAction::Ocr(one.clone())));
            }
            ui.separator();
            if ui.button(i18n::tf(format!("{} Elimina", ic::TRASH))).on_hover_text(i18n::t("Sposta nel cestino (Canc)")).clicked() {
                actions.push(lib(LibAction::Trash(one)));
            }
        });
    }

    fn grid(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, actions: &mut Vec<Action>) {
        let ids = self.visible_ids();
        if self.lib.entries.is_empty() {
            ui.add_space(40.0);
            ui.vertical_centered(|ui| {
                ui.heading(i18n::t("L'archivio è vuoto"));
                ui.label(i18n::t("Scrivi sul pad senza Mac, poi «Collega» e «Scarica dal pad»."));
                ui.label(i18n::t("Oppure, dalla Lavagna, «Salva in archivio»."));
            });
            return;
        }
        if ids.is_empty() {
            ui.label(i18n::t("Nessuna pagina corrisponde alla ricerca."));
            return;
        }
        self.make_thumbs(ctx, &ids);

        // Gruppi per giorno, nell'ordine dell'archivio (dal più recente).
        let mut groups: Vec<(String, Vec<String>)> = Vec::new();
        for id in &ids {
            let label = library::day_label(self.lib.get(id).and_then(|e| e.meta.created_dt()));
            match groups.last_mut() {
                Some((l, v)) if *l == label => v.push(id.clone()),
                _ => groups.push((label, vec![id.clone()])),
            }
        }

        // pizzico sul trackpad: anteprime più grandi o più piccole
        let over = ui.input(|i| i.pointer.hover_pos()).is_some_and(|p| ui.max_rect().contains(p));
        if over {
            let pinch = ui.input(|i| i.events.iter().map(|e| if let egui::Event::Zoom(f) = e { *f } else { 1.0 }).product::<f32>());
            if (pinch - 1.0).abs() > 1e-4 {
                self.lui.thumb_size = (self.lui.thumb_size * pinch).clamp(THUMB_MIN, THUMB_MAX);
            }
        }
        let mods = ui.input(|i| i.modifiers);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (label, gids) in &groups {
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new(i18n::tf(format!("{label}  ({})", gids.len()))).heading());
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(16.0, 16.0);
                        for id in gids {
                            self.card(ui, id, mods, actions);
                        }
                    });
                    ui.add_space(10.0);
                }
            });
    }

    fn card(&self, ui: &mut egui::Ui, id: &str, mods: egui::Modifiers, actions: &mut Vec<Action>) {
        let Some(e) = self.lib.get(id) else { return };
        let (card_w, thumb_h, card_h) = self.lui.card_size();
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(card_w, card_h), egui::Sense::click());
        let selected = self.lui.selected.contains(id);
        if ui.is_rect_visible(rect) {
            let p = ui.painter();
            let thumb = egui::Rect::from_min_size(rect.min, egui::vec2(card_w, thumb_h));
            if selected {
                p.rect_filled(rect.expand(5.0), 6.0, ui.visuals().selection.bg_fill.gamma_multiply(0.35));
            } else if resp.hovered() {
                p.rect_filled(rect.expand(5.0), 6.0, ui.visuals().widgets.hovered.bg_fill.gamma_multiply(0.5));
            }
            // foglio con ombra, anteprima centrata con le sue proporzioni
            let sheet = match self.lui.thumbs.get(id) {
                Some((_, _, tex)) => {
                    let [tw, th] = tex.size();
                    let s = (card_w / tw as f32).min(thumb_h / th as f32);
                    let size = egui::vec2(tw as f32 * s, th as f32 * s);
                    egui::Rect::from_center_size(thumb.center(), size)
                }
                None => thumb.shrink2(egui::vec2(card_w * 0.04, 0.0)),
            };
            p.rect_filled(sheet.translate(egui::vec2(2.0, 2.0)), 2.0, egui::Color32::from_black_alpha(60));
            p.rect_filled(sheet, 2.0, egui::Color32::WHITE);
            if let Some((_, _, tex)) = self.lui.thumbs.get(id) {
                let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                p.image(tex.id(), sheet, uv, egui::Color32::WHITE);
            }
            let border = if selected {
                egui::Stroke::new(3.0_f32, ui.visuals().selection.stroke.color)
            } else {
                egui::Stroke::new(1.0_f32, egui::Color32::from_gray(150))
            };
            p.rect_stroke(sheet, 2.0, border, egui::StrokeKind::Inside);

            // titolo (una riga, con …) e ora
            let text_col = ui.visuals().text_color();
            let mut job = egui::text::LayoutJob::single_section(
                e.meta.display_title(),
                egui::TextFormat::simple(egui::FontId::proportional(13.5), text_col),
            );
            job.wrap = egui::text::TextWrapping {
                max_width: card_w,
                max_rows: 1,
                break_anywhere: true,
                overflow_character: Some('…'),
            };
            let galley = ui.fonts_mut(|f| f.layout_job(job));
            p.galley(egui::pos2(rect.left(), thumb.bottom() + 6.0), galley, text_col);
            let origin = if e.meta.source == library::SOURCE_BOARD { i18n::t("dalla lavagna") } else { i18n::t("dal pad") };
            let sub = match (e.meta.title.is_empty(), e.meta.created_dt()) {
                (false, Some(t)) => format!("{}  •  {origin}", t.format("%d/%m/%Y %H:%M")),
                _ => origin.to_string(),
            };
            // anche la data resta nella sua scheda (anteprime piccole: con …)
            let weak = ui.visuals().weak_text_color();
            let mut job = egui::text::LayoutJob::single_section(i18n::tr(&sub).into_owned(), egui::TextFormat::simple(egui::FontId::proportional(11.5), weak));
            job.wrap = egui::text::TextWrapping { max_width: card_w, max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
            let galley = ui.fonts_mut(|f| f.layout_job(job));
            p.galley(egui::pos2(rect.left(), thumb.bottom() + 24.0), galley, weak);
        }

        if resp.double_clicked() {
            actions.push(lib(LibAction::Open(id.to_string())));
        } else if resp.clicked() {
            actions.push(lib(LibAction::Select {
                id: id.to_string(),
                cmd: mods.command,
                shift: mods.shift,
            }));
        }
        let sel = self.selection();
        resp.context_menu(|ui| {
            // Il menu agisce sulla selezione se la pagina ne fa parte, altrimenti sulla pagina.
            let targets = if selected && sel.len() > 1 { sel.clone() } else { vec![id.to_string()] };
            let many = targets.len() > 1;
            if !many {
                if ui.button(i18n::tf(format!("{} Apri", ic::CORNERS_OUT))).clicked() {
                    actions.push(lib(LibAction::Open(id.to_string())));
                    ui.close();
                }
                if ui.button(i18n::tf(format!("{} Sulla lavagna", ic::CHALKBOARD_SIMPLE))).clicked() {
                    actions.push(lib(LibAction::ToBoard(id.to_string())));
                    ui.close();
                }
                if ui.button(i18n::tf(format!("{} Rinomina…", ic::PENCIL_SIMPLE))).clicked() {
                    actions.push(lib(LibAction::StartRename(id.to_string())));
                    ui.close();
                }
                if ui.button(i18n::tf(format!("{} Copia immagine", ic::COPY))).clicked() {
                    actions.push(lib(LibAction::Copy(id.to_string())));
                    ui.close();
                }
                ui.separator();
            } else {
                ui.label(i18n::tf(format!("{} pagine selezionate", targets.len())));
                ui.separator();
            }
            if ui.button(i18n::tf(format!("{} {}", ic::FILE_PDF, if many { "Esporta in un PDF" } else { "Esporta PDF" }))).clicked() {
                actions.push(lib(LibAction::Export(targets.clone(), Fmt::Pdf)));
                ui.close();
            }
            if ui.button(i18n::tf(format!("{} Esporta PNG", ic::FILE_PNG))).clicked() {
                actions.push(lib(LibAction::Export(targets.clone(), Fmt::Png)));
                ui.close();
            }
            if ui.button(i18n::tf(format!("{} Esporta SVG", ic::FILE_SVG))).clicked() {
                actions.push(lib(LibAction::Export(targets.clone(), Fmt::Svg)));
                ui.close();
            }
            super::chrome::more_formats_menu(ui, |k| actions.push(lib(LibAction::Export(targets.clone(), k))));
            if ui.button(i18n::tf(format!("{} Trascrivi", ic::MATH_OPERATIONS))).clicked() {
                actions.push(lib(LibAction::Ocr(targets.clone())));
                ui.close();
            }
            ui.separator();
            if ui.button(i18n::tf(format!("{} Elimina", ic::TRASH))).clicked() {
                actions.push(lib(LibAction::Trash(targets)));
                ui.close();
            }
        });
    }

    fn rename_window(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        let Some((_, text)) = &mut self.lui.rename else { return };
        let focused = &mut self.lui.rename_focused;
        let mut commit = false;
        let mut cancel = false;
        egui::Window::new(i18n::t("Rinomina pagina"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                let r = ui.add(egui::TextEdit::singleline(text).hint_text(i18n::t("Titolo (vuoto = data)")).desired_width(280.0));
                if !*focused {
                    r.request_focus();
                    *focused = true;
                }
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    commit = true;
                }
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    cancel = true;
                }
                ui.horizontal(|ui| {
                    if ui.button(i18n::tf(format!("{} OK", ic::CHECK))).clicked() {
                        commit = true;
                    }
                    if ui.button(i18n::tf(format!("{} Annulla", ic::X))).clicked() {
                        cancel = true;
                    }
                });
            });
        if commit {
            actions.push(lib(LibAction::CommitRename));
        } else if cancel {
            actions.push(lib(LibAction::CancelRename));
        }
    }
}
