//! Il timbro aziendale sulla desktop: la finestra dove si costruiscono i
//! timbri (dati dell'azienda, cornice, colore), gli strumenti per renderlo
//! realistico dopo averlo posato sul documento, e le immagini dei timbri da
//! disegnare sullo schermo.

use super::doc_view::DocAction;
use super::widgets::*;
use super::{Action, App};
use crate::i18n::{self, tr};
use crate::stamp::{self, Frame, InkColor, SavedStamp, StampSpec};
use eframe::egui;
use egui_phosphor::regular as ic;
use std::cell::RefCell;
use std::collections::HashMap;

/// Dove si salvano i timbri (accanto alla firma).
pub(super) const FILE: &str = "signatures/timbri.json";

/// Una regolazione del timbro posato.
#[derive(Clone, Copy, Debug)]
pub(super) enum SealEdit {
    Color(InkColor),
    Frame(Frame),
    Opacity(f32),
    Grain(f32),
    Uneven(f32),
    Tilt(f32),
    /// Grana nuova (e il lato scarico da un'altra parte).
    Reseed,
}

impl SealEdit {
    pub(super) fn apply(self, spec: &mut StampSpec) {
        let st = &mut spec.style;
        match self {
            SealEdit::Color(c) => st.color = c,
            SealEdit::Frame(f) => st.frame = f,
            SealEdit::Opacity(v) => st.opacity = v.clamp(0.3, 1.0),
            SealEdit::Grain(v) => st.grain = v.clamp(0.0, 1.0),
            SealEdit::Uneven(v) => st.uneven = v.clamp(0.0, 1.0),
            SealEdit::Tilt(v) => st.tilt = v.clamp(-15.0, 15.0),
            SealEdit::Reseed => {
                st.seed = stamp::new_seed(st.seed);
                st.uneven_dir = (st.seed % 360) as f32;
            }
        }
    }
}

pub(super) struct StampUi {
    pub list: Vec<SavedStamp>,
    pub editor: Option<Editor>,
}

pub(super) struct Editor {
    /// Il timbro dell'elenco che si sta modificando (None = nuovo).
    sel: Option<usize>,
    draft: SavedStamp,
    msg: String,
}

impl StampUi {
    pub fn new() -> Self {
        StampUi { list: stamp::load_all(std::path::Path::new(FILE)), editor: None }
    }

    pub fn save(&self) -> Result<(), String> {
        stamp::save_all(std::path::Path::new(FILE), &self.list).map_err(|e| e.to_string())
    }
}

fn new_draft() -> SavedStamp {
    let mut spec = StampSpec::default();
    spec.style.seed = stamp::new_seed(1);
    SavedStamp { name: String::new(), spec }
}

pub(super) fn color_name(c: InkColor) -> &'static str {
    match c {
        InkColor::Blue => "Blu",
        InkColor::Grey => "Grigio",
        InkColor::Black => "Nero",
        InkColor::Red => "Rosso",
        InkColor::Violet => "Viola",
    }
}

pub(super) fn frame_name(f: Frame) -> &'static str {
    match f {
        Frame::Plain => "Senza cornice",
        Frame::Rect => "Rettangolo",
        Frame::Rounded => "Angoli arrotondati",
        Frame::Double => "Doppia linea",
        Frame::Oval => "Ovale",
    }
}

fn rgb32(c: InkColor) -> egui::Color32 {
    let [r, g, b] = c.rgb();
    egui::Color32::from_rgb(r, g, b)
}

// ---------------------------------------------------------------------------
// Immagini dei timbri per lo schermo

thread_local! {
    static TEX: RefCell<HashMap<(u64, u8, u32), egui::TextureHandle>> = RefCell::new(HashMap::new());
}

/// L'immagine del timbro, girata di `quarter`, per un riquadro largo `px_w`
/// pixel sullo schermo (la risoluzione va a scalini: si rifà di rado).
pub(super) fn texture(ctx: &egui::Context, spec: &StampSpec, quarter: u8, px_w: f32) -> egui::TextureId {
    let step = (px_w.clamp(64.0, 2400.0) / 160.0).ceil() as u32 * 160;
    let key = (spec.key(), quarter, step);
    TEX.with(|t| {
        let mut t = t.borrow_mut();
        if let Some(h) = t.get(&key) {
            return h.id();
        }
        if t.len() > 24 {
            t.clear();
        }
        let r = stamp::render_fit(spec, quarter, step as f32);
        let img = egui::ColorImage::from_rgba_premultiplied([r.w as usize, r.h as usize], &r.px);
        let h = ctx.load_texture(format!("timbro-{:x}-{quarter}-{step}", key.0), img, egui::TextureOptions::LINEAR);
        let id = h.id();
        t.insert(key, h);
        id
    })
}

/// Un timbro posato (tratto con `stamp`) sulla pagina dello schermo.
pub(super) fn paint_placed(painter: &egui::Painter, page: egui::Rect, spec: &StampSpec, corners_uv: [f32; 4], quarter: u8) {
    let [u0, v0, u1, v1] = corners_uv;
    let r = egui::Rect::from_min_max(
        egui::pos2(page.left() + u0 * page.width(), page.top() + v0 * page.height()),
        egui::pos2(page.left() + u1 * page.width(), page.top() + v1 * page.height()),
    );
    if !r.is_positive() || !painter.clip_rect().intersects(r) {
        return;
    }
    let ppp = painter.ctx().pixels_per_point();
    let tex = texture(painter.ctx(), spec, quarter, r.width() * ppp);
    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    painter.image(tex, r, uv, egui::Color32::WHITE);
}

// ---------------------------------------------------------------------------
// La finestra dei timbri

impl App {
    pub(super) fn stamp_window(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        if self.stampui.editor.is_none() {
            return;
        }
        let can_place = self.docui.doc.is_some();
        let mut open = true;
        let mut close = false;
        let StampUi { list, editor } = &mut self.stampui;
        let ed = editor.as_mut().unwrap();
        let mut save = false;
        let mut delete = false;
        let mut place = false;
        let mut pick: Option<Option<usize>> = None;
        egui::Window::new(i18n::t("Timbri aziendali"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(680.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.horizontal_top(|ui| {
                    // l'elenco
                    ui.vertical(|ui| {
                        ui.set_width(170.0);
                        ui.label(egui::RichText::new(tr("I tuoi timbri")).strong());
                        egui::ScrollArea::vertical().max_height(330.0).show(ui, |ui| {
                            for (i, s) in list.iter().enumerate() {
                                let name = if s.name.trim().is_empty() { s.spec.company.name.clone() } else { s.name.clone() };
                                if ui.selectable_label(ed.sel == Some(i), name).clicked() {
                                    pick = Some(Some(i));
                                }
                            }
                            if list.is_empty() {
                                ui.label(egui::RichText::new(tr("Nessun timbro salvato.")).weak());
                            }
                        });
                        ui.add_space(6.0);
                        if ui.button(format!("{} {}", ic::PLUS, tr("Nuovo timbro"))).clicked() {
                            pick = Some(None);
                        }
                    });
                    ui.separator();
                    // il timbro scelto
                    ui.vertical(|ui| {
                        ui.set_width(470.0);
                        let d = &mut ed.draft;
                        let c = &mut d.spec.company;
                        egui::Grid::new("timbro_campi").num_columns(2).spacing([8.0, 5.0]).show(ui, |ui| {
                            let row = |ui: &mut egui::Ui, label: &str, v: &mut String, hint: &str, w: f32| {
                                ui.label(tr(label));
                                ui.add(egui::TextEdit::singleline(v).hint_text(tr(hint)).desired_width(w));
                                ui.end_row();
                            };
                            row(ui, "Nome del timbro", &mut d.name, "per esempio: Cordua", 300.0);
                            row(ui, "Ragione sociale", &mut c.name, "CORDUA S.R.L.", 300.0);
                            row(ui, "Indirizzo", &mut c.address, "Piazza della Vittoria, 15/D", 300.0);
                            ui.label(tr("CAP, città, provincia"));
                            ui.horizontal(|ui| {
                                ui.add(egui::TextEdit::singleline(&mut c.cap).hint_text("16121").desired_width(60.0));
                                ui.add(egui::TextEdit::singleline(&mut c.city).hint_text("Genova").desired_width(170.0));
                                ui.add(egui::TextEdit::singleline(&mut c.prov).hint_text("GE").desired_width(40.0));
                            });
                            ui.end_row();
                            row(ui, "P. IVA", &mut c.vat, "02597400999", 160.0);
                            row(ui, "REA (se c'è)", &mut c.rea, "GE-123456", 160.0);
                            row(ui, "Codice univoco (se c'è)", &mut c.sdi, "USAL8PV", 160.0);
                        });
                        ui.add_space(6.0);
                        let st = &mut d.spec.style;
                        ui.horizontal(|ui| {
                            ui.label(tr("Cornice"));
                            egui::ComboBox::from_id_salt("timbro_cornice").selected_text(tr(frame_name(st.frame))).show_ui(ui, |ui| {
                                for f in Frame::ALL {
                                    ui.selectable_value(&mut st.frame, f, tr(frame_name(f)));
                                }
                            });
                            ui.add_space(10.0);
                            ui.label(tr("Colore"));
                            for col in InkColor::ALL {
                                let (r, resp) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::click());
                                ui.painter().circle_filled(r.center(), 8.0, rgb32(col));
                                if st.color == col {
                                    ui.painter().circle_stroke(r.center(), 10.0, egui::Stroke::new(2.0, ui.visuals().strong_text_color()));
                                }
                                if resp.on_hover_text(tr(color_name(col))).clicked() {
                                    st.color = col;
                                }
                            }
                        });
                        ui.add_space(8.0);
                        // anteprima, sul bianco come sulla carta
                        let (bw, bh) = (470.0, 190.0);
                        let (r, _) = ui.allocate_exact_size(egui::vec2(bw, bh), egui::Sense::hover());
                        ui.painter().rect_filled(r, 4.0, egui::Color32::WHITE);
                        let (sw, sh) = stamp::size_cm(&d.spec);
                        let k = ((bw - 20.0) / sw).min((bh - 20.0) / sh);
                        let sr = egui::Rect::from_center_size(r.center(), egui::vec2(sw * k, sh * k));
                        let tex = texture(ui.ctx(), &d.spec, 0, sr.width() * ui.ctx().pixels_per_point());
                        ui.painter().image(tex, sr, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                        ui.label(
                            egui::RichText::new(i18n::tf(format!(
                                "Grandezza vera: {:.1} × {:.1} cm. Grana, intensità e inclinazione si regolano dopo averlo posato sul documento.",
                                sw, sh
                            )))
                            .weak()
                            .small(),
                        );
                        if !ed.msg.is_empty() {
                            ui.label(egui::RichText::new(tr(&ed.msg)).color(ui.visuals().warn_fg_color));
                        }
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            let ok = !ed.draft.spec.company.name.trim().is_empty();
                            if ui.add_enabled(ok, egui::Button::new(format!("{} {}", ic::FLOPPY_DISK, tr("Salva")))).clicked() {
                                save = true;
                            }
                            if ui
                                .add_enabled(ok && can_place, egui::Button::new(format!("{} {}", ic::STAMP, tr("Apponi sul documento"))))
                                .on_disabled_hover_text(tr("Apri prima un documento"))
                                .clicked()
                            {
                                place = true;
                            }
                            if ed.sel.is_some() && ui.button(format!("{} {}", ic::TRASH, tr("Elimina"))).clicked() {
                                delete = true;
                            }
                            if ui.button(format!("{} {}", ic::X, tr("Chiudi"))).clicked() {
                                close = true;
                            }
                        });
                    });
                });
            });
        if let Some(p) = pick {
            ed.sel = p;
            ed.draft = p.and_then(|i| list.get(i).cloned()).unwrap_or_else(new_draft);
            ed.msg.clear();
        }
        if save || place {
            let mut d = ed.draft.clone();
            if d.name.trim().is_empty() {
                d.name = d.spec.company.name.trim().to_string();
            }
            match ed.sel {
                Some(i) if i < list.len() => list[i] = d.clone(),
                _ => {
                    list.push(d.clone());
                    ed.sel = Some(list.len() - 1);
                }
            }
            ed.draft = d;
            match stamp::save_all(std::path::Path::new(FILE), list) {
                Ok(()) => ed.msg.clear(),
                Err(e) => ed.msg = format!("⚠ {e}"),
            }
            if place {
                if let Some(i) = ed.sel {
                    actions.push(Action::Doc(DocAction::PlaceSeal(i)));
                }
                close = true;
            }
        }
        if delete {
            if let Some(i) = ed.sel.filter(|&i| i < list.len()) {
                list.remove(i);
                let _ = stamp::save_all(std::path::Path::new(FILE), list);
                ed.sel = None;
                ed.draft = new_draft();
            }
        }
        if close || !open {
            self.stampui.editor = None;
        }
    }

    /// Apre la finestra dei timbri (sul timbro `sel`, o su uno nuovo).
    pub(super) fn open_stamp_window(&mut self, sel: Option<usize>) {
        let draft = sel.and_then(|i| self.stampui.list.get(i).cloned()).unwrap_or_else(new_draft);
        self.stampui.editor = Some(Editor { sel: sel.filter(|&i| i < self.stampui.list.len()), draft, msg: String::new() });
    }

    /// I comandi della barra mentre si posa un timbro: colore, cornice, grana…
    pub(super) fn ribbon_seal(&self, ui: &mut egui::Ui, spec: &StampSpec, actions: &mut Vec<Action>) {
        let edit = |e: SealEdit| Action::Doc(DocAction::Seal(e));
        let st = &spec.style;
        group(ui, "Colore", |ui| {
            column(ui, |ui| {
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    for col in InkColor::ALL {
                        let (r, resp) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::click());
                        ui.painter().circle_filled(r.center(), 8.0, rgb32(col));
                        if st.color == col {
                            ui.painter().circle_stroke(r.center(), 10.0, egui::Stroke::new(2.0, ui.visuals().strong_text_color()));
                        }
                        if resp.on_hover_text(tr(color_name(col))).clicked() {
                            actions.push(edit(SealEdit::Color(col)));
                        }
                    }
                });
                let r = small(ui, ic::FRAME_CORNERS, frame_name(st.frame), "La cornice del timbro");
                egui::Popup::menu(&r).show(|ui| {
                    for f in Frame::ALL {
                        if ui.selectable_label(st.frame == f, tr(frame_name(f))).clicked() {
                            actions.push(edit(SealEdit::Frame(f)));
                        }
                    }
                });
                if small(ui, ic::SHUFFLE, "Nuova grana", "Un'altra impronta: grana e lato scarico cambiano").clicked() {
                    actions.push(edit(SealEdit::Reseed));
                }
            });
        });
        let slider = |ui: &mut egui::Ui, label: &str, tip: &str, v: f32, range: std::ops::RangeInclusive<f32>, suffix: &str, mk: fn(f32) -> SealEdit, actions: &mut Vec<Action>| {
            ui.horizontal(|ui| {
                ui.add_sized(egui::vec2(118.0, 20.0), egui::Label::new(egui::RichText::new(tr(label)).size(11.5))).on_hover_text(tr(tip));
                let mut x = v;
                let r = ui.add(egui::Slider::new(&mut x, range).suffix(suffix).fixed_decimals(0).show_value(true));
                if r.changed() {
                    actions.push(edit(mk(x)));
                }
            });
        };
        group(ui, "Realismo", |ui| {
            column(ui, |ui| {
                ui.spacing_mut().slider_width = 110.0;
                slider(ui, "Intensità", "Quanto copre l'inchiostro: più bassa, più si vede il testo sotto", st.opacity * 100.0, 30.0..=100.0, " %", |x| SealEdit::Opacity(x / 100.0), actions);
                slider(ui, "Grana", "L'inchiostro che manca a puntini e chiazze, come un timbro vero", st.grain * 100.0, 0.0..=100.0, " %", |x| SealEdit::Grain(x / 100.0), actions);
                slider(ui, "Inchiostro irregolare", "Un lato più scarico dell'altro, come un timbro premuto storto", st.uneven * 100.0, 0.0..=100.0, " %", |x| SealEdit::Uneven(x / 100.0), actions);
            });
            column(ui, |ui| {
                ui.spacing_mut().slider_width = 110.0;
                slider(ui, "Inclinazione", "Ruota il timbro di qualche grado, come messo a mano", st.tilt, -15.0..=15.0, "°", SealEdit::Tilt, actions);
            });
        });
    }
}
