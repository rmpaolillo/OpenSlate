//! Disegno della rigatura del foglio (la geometria è in `crate::rigatura`).
//! Si disegna solo sullo schermo, sotto la scrittura (non va nelle
//! immagini per l'AI né negli export).

use crate::i18n;
pub(super) use crate::rigatura::Paper;
use crate::rigatura::{layout, Ink, Mark, Pt};
use eframe::egui;
use egui::{pos2, Align2, Color32, FontId, Rect, Stroke};

/// Disegna la rigatura sulla pagina `page` (rettangolo sullo schermo) di
/// `size_cm` centimetri; `margin` aggiunge ai fogli a righe i due margini
/// rossi (i quadretti non li hanno, tranne quelli «con margine»).
pub(super) fn paint(painter: &egui::Painter, page: Rect, size_cm: (f32, f32), paper: Paper, margin: bool) {
    let (w_cm, h_cm) = size_cm;
    if w_cm <= 0.0 || h_cm <= 0.0 {
        return;
    }
    let marks = layout(paper, w_cm, h_cm, margin);
    if marks.is_empty() {
        return;
    }
    let px = page.width() / w_cm; // punti per cm
    let at = |(x, y): Pt| pos2(page.min.x + x * px, page.min.y + y * px);
    // linee troppo fitte per vedersi bene (pagina rimpicciolita): si sfumano
    let fade = |cm: f32| (cm * px / 8.0).clamp(0.0, 1.0);
    let f_line = fade(paper.pitch());
    let f_fine = fade(0.1);
    let rgb = if paper == Paper::Millimetrata {
        (225, 130, 60)
    } else if paper.grid_like() {
        (110, 150, 205)
    } else {
        (80, 125, 200)
    };
    let tint = |a: f32| Color32::from_rgba_unmultiplied(rgb.0, rgb.1, rgb.2, a.clamp(0.0, 255.0) as u8);
    let mm = paper == Paper::Millimetrata;
    let line_alpha = if mm { 75.0 } else if paper.grid_like() { 95.0 } else { 130.0 };
    let strong_alpha = if mm { 140.0 } else { 175.0 };
    let axis = Color32::from_rgba_unmultiplied(60, 70, 95, 210);
    let stroke = |ink: Ink| -> Option<Stroke> {
        match ink {
            Ink::Fine => (f_fine >= 0.35).then(|| Stroke::new(0.6, tint(80.0 * f_fine))),
            Ink::Line => (f_line > 0.05).then(|| Stroke::new(1.0, tint(line_alpha * f_line))),
            Ink::Strong => Some(Stroke::new(1.3, tint(strong_alpha * f_line.max(0.5)))),
            Ink::Red => Some(Stroke::new(1.2, Color32::from_rgba_unmultiplied(215, 70, 70, 150))),
            Ink::Axis => Some(Stroke::new(1.4, axis)),
        }
    };
    let dot_r = (0.04 * px).clamp(0.8, 2.2);
    let label_color = if paper == Paper::Cartesiano { axis } else { tint(200.0) };
    let font = FontId::proportional((0.3 * px).clamp(9.0, 15.0));
    for mk in marks {
        match mk {
            Mark::Line(ink, a, b) => {
                if let Some(st) = stroke(ink) {
                    painter.line_segment([at(a), at(b)], st);
                }
            }
            Mark::Dot(p) => {
                if f_line > 0.05 {
                    painter.circle_filled(at(p), dot_r, tint(170.0 * f_line));
                }
            }
            Mark::Label(p, s) => {
                if px >= 12.0 {
                    painter.text(at(p), Align2::CENTER_CENTER, i18n::tr(&s).into_owned(), font.clone(), label_color);
                }
            }
        }
    }
}
