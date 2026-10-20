//! Pulsanti della barra multifunzione (grandi e piccoli) e gruppi col nome
//! sotto, come in Word.
//!
//! Misure e disegno ripresi da WordCraft (`crates/ui-egui/src/widgets.rs`,
//! https://github.com/storytold/wordcraft, MIT OR Apache-2.0); le icone sono
//! quelle di Phosphor invece di quelle disegnate a mano.

use super::theme::{icon, regular, Tokens};
use eframe::egui;
use egui::{pos2, vec2, Align2, CornerRadius, Rect, Response, Sense, Stroke, Ui};

/// Altezza dei comandi di un gruppo (senza il nome sotto).
pub const CONTENT_H: f32 = 66.0;
/// Altezza della riga col nome del gruppo.
pub const LABEL_H: f32 = 18.0;
/// Pulsanti grandi: centro dell'icona, inizio e passo delle righe del testo.
const BIG_ICON_Y: f32 = 19.0;
const BIG_LABEL_Y: f32 = 37.0;
const LABEL_PITCH: f32 = 13.0;
/// Altezza dell'intera barra (comandi, nomi dei gruppi, margini).
pub const RIBBON_H: f32 = CONTENT_H + LABEL_H + 10.0;

/// Sfondo di un pulsante: premuto, attivo (spuntato) o sotto il mouse.
pub fn bg(ui: &Ui, r: Rect, resp: &Response, checked: bool, t: &Tokens) {
    let fill = if resp.is_pointer_button_down_on() {
        t.pressed
    } else if checked {
        t.checked
    } else if resp.hovered() {
        t.hover
    } else {
        return;
    };
    ui.painter().rect_filled(r, CornerRadius::same(4), fill);
    if checked {
        ui.painter().rect_stroke(r, CornerRadius::same(4), Stroke::new(1.0, t.accent.linear_multiply(0.5)), egui::StrokeKind::Inside);
    }
}

fn text_w(ui: &Ui, s: &str, font: egui::FontId) -> f32 {
    ui.painter().layout_no_wrap(s.to_string(), font, egui::Color32::WHITE).size().x
}

/// Pulsante grande: icona da 30 pt sopra, nome sotto (anche su due righe con
/// `\n`). `menu` aggiunge la freccina (il pulsante apre un menu).
pub fn big_ex(ui: &mut Ui, glyph: &str, label: &str, tip: &str, enabled: bool, checked: bool, menu: bool) -> Response {
    let t = Tokens::get(ui.ctx());
    let lines: Vec<&str> = label.split('\n').collect();
    let w = lines.iter().map(|l| text_w(ui, l, regular(11.5)) + if menu { 12.0 } else { 0.0 }).fold(0.0, f32::max);
    let w = (w + 14.0).max(46.0);
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (r, resp) = ui.allocate_exact_size(vec2(w, CONTENT_H), sense);
    if enabled {
        bg(ui, r, &resp, checked, &t);
    }
    let (ic, tx) = if enabled { (if checked { t.accent_text } else { t.icon }, t.text) } else { (t.text_disabled, t.text_disabled) };
    ui.painter().text(pos2(r.center().x, r.min.y + BIG_ICON_Y), Align2::CENTER_CENTER, glyph, icon(28.0), ic);
    let mut y = r.min.y + BIG_LABEL_Y;
    for (i, l) in lines.iter().enumerate() {
        let last = i + 1 == lines.len();
        let text = if menu && last { format!("{l} {}", egui_phosphor::regular::CARET_DOWN) } else { l.to_string() };
        ui.painter().text(pos2(r.center().x, y), Align2::CENTER_TOP, text, regular(11.5), tx);
        y += LABEL_PITCH;
    }
    let resp = if tip.is_empty() { resp } else { resp.on_hover_text(tip) };
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label.replace('\n', " ")));
    resp
}

pub fn big(ui: &mut Ui, glyph: &str, label: &str, tip: &str) -> Response {
    big_ex(ui, glyph, label, tip, true, false, false)
}

/// Pulsante piccolo: icona da 16 pt e nome a destra, alto 22 pt (tre per
/// colonna in un gruppo).
pub fn small_ex(ui: &mut Ui, glyph: &str, label: &str, tip: &str, enabled: bool, checked: bool) -> Response {
    let t = Tokens::get(ui.ctx());
    let tw = if label.is_empty() { 0.0 } else { text_w(ui, label, regular(11.5)) + 6.0 };
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (r, resp) = ui.allocate_exact_size(vec2(24.0 + tw, 22.0), sense);
    if enabled {
        bg(ui, r, &resp, checked, &t);
    }
    let (ic, tx) = if enabled { (if checked { t.accent_text } else { t.icon }, t.text) } else { (t.text_disabled, t.text_disabled) };
    ui.painter().text(pos2(r.min.x + 12.0, r.center().y), Align2::CENTER_CENTER, glyph, icon(16.0), ic);
    if !label.is_empty() {
        ui.painter().text(pos2(r.min.x + 24.0, r.center().y), Align2::LEFT_CENTER, label, regular(11.5), tx);
    }
    let resp = if tip.is_empty() { resp } else { resp.on_hover_text(tip) };
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, if label.is_empty() { tip } else { label }));
    resp
}

pub fn small(ui: &mut Ui, glyph: &str, label: &str, tip: &str) -> Response {
    small_ex(ui, glyph, label, tip, true, false)
}

/// Colonna di pulsanti piccoli (fino a tre, uno sotto l'altro).
pub fn column(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    ui.allocate_ui_with_layout(vec2(0.0, CONTENT_H), egui::Layout::top_down(egui::Align::Min), |ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        ui.add_space(1.0);
        add(ui);
    });
}

/// Gruppo della barra: i comandi, il nome centrato sotto e una linea a destra.
pub fn group(ui: &mut Ui, title: &str, add: impl FnOnce(&mut Ui)) {
    let t = Tokens::get(ui.ctx());
    let start = ui.cursor().min;
    let label_w = text_w(ui, title, regular(11.0));
    let inner = ui.vertical(|ui| {
        ui.set_min_width(label_w + 10.0);
        ui.allocate_ui_with_layout(vec2(ui.available_width(), CONTENT_H), egui::Layout::left_to_right(egui::Align::Min), |ui| {
            ui.spacing_mut().item_spacing = vec2(1.0, 1.0);
            add(ui);
        });
    });
    let r = inner.response.rect;
    let title_r = Rect::from_min_max(pos2(start.x, start.y + CONTENT_H), pos2(r.max.x, start.y + CONTENT_H + LABEL_H));
    ui.painter().text(title_r.center(), Align2::CENTER_CENTER, title, regular(11.0), t.group_label);
    ui.add_space(5.0);
    let x = ui.cursor().min.x;
    ui.painter().line_segment([pos2(x, start.y + 4.0), pos2(x, start.y + CONTENT_H + LABEL_H - 4.0)], Stroke::new(1.0, t.border));
    ui.add_space(6.0);
}

/// Pulsantino solo icona (barra del titolo, barra di stato).
pub fn icon_button(ui: &mut Ui, glyph: &str, tip: &str, size: egui::Vec2, enabled: bool) -> Response {
    let t = Tokens::get(ui.ctx());
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (r, resp) = ui.allocate_exact_size(size, sense);
    if enabled && (resp.hovered() || resp.is_pointer_button_down_on()) {
        ui.painter().rect_filled(r, 4.0, if resp.is_pointer_button_down_on() { t.pressed } else { t.hover });
    }
    let c = if enabled { t.icon } else { t.text_disabled };
    ui.painter().text(r.center(), Align2::CENTER_CENTER, glyph, icon((size.y * 0.66).min(17.0)), c);
    resp.on_hover_text(tip)
}
