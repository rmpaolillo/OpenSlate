//! Rigatura del foglio della lavagna, come i quaderni italiani: bianco,
//! righe (con i due margini rossi), righe delle elementari, quadretti,
//! carta millimetrata, piano cartesiano, puntinato, isometrica, pentagramma
//! e il foglio per appunti Cornell.
//! Si disegna solo sullo schermo, sotto la scrittura (non va nelle
//! immagini per l'AI né negli export).
//!
//! Le misure delle righe delle elementari sono indicative: ogni marca di
//! quaderni ha le sue (qui: 1ª con tre fasce da 4 mm, 2ª da 3 mm, 3ª con una
//! fascia da 3 mm).

use crate::i18n;
use eframe::egui;
use egui::{pos2, Align2, Color32, FontId, Rect, Stroke};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Paper {
    Bianco,
    /// Righe a 8 mm (medie e superiori).
    Righe,
    /// Righe a 1 cm (più larghe).
    RigheLarghe,
    /// Rigatura di prima elementare: tre fasce per riga.
    Elem1,
    /// Rigatura di seconda: tre fasce più strette.
    Elem2,
    /// Rigatura di terza: una fascia sola per riga.
    Elem3,
    /// Quadretti da 5 mm (medie e superiori).
    Quadretti5,
    /// Quadretti da 5 mm con il margine rosso a sinistra.
    Quadretti5M,
    /// Quadretti da 4 mm.
    Quadretti4,
    /// Quadretti da 1 cm (elementari).
    Quadretti10,
    /// Carta millimetrata: 1 mm, 5 mm e 1 cm.
    Millimetrata,
    /// Quadretti da 5 mm con gli assi x e y numerati.
    Cartesiano,
    /// Un punto ogni 5 mm.
    Puntinato,
    /// Triangoli equilateri da 5 mm (disegno in assonometria).
    Isometrica,
    /// Pentagrammi per la musica.
    Pentagramma,
    /// Appunti Cornell: parole chiave a sinistra, appunti, riassunto in fondo.
    Cornell,
}

impl Paper {
    pub const ALL: [Paper; 16] = [
        Paper::Bianco,
        Paper::Righe,
        Paper::RigheLarghe,
        Paper::Elem1,
        Paper::Elem2,
        Paper::Elem3,
        Paper::Quadretti5,
        Paper::Quadretti5M,
        Paper::Quadretti4,
        Paper::Quadretti10,
        Paper::Millimetrata,
        Paper::Cartesiano,
        Paper::Puntinato,
        Paper::Isometrica,
        Paper::Pentagramma,
        Paper::Cornell,
    ];

    /// I fogli divisi per tipo, per i menu (il bianco sta a parte, in cima).
    pub const GROUPS: [(&'static str, &'static [Paper]); 4] = [
        ("Righe", &[Paper::Righe, Paper::RigheLarghe, Paper::Elem1, Paper::Elem2, Paper::Elem3]),
        ("Quadretti", &[Paper::Quadretti5, Paper::Quadretti5M, Paper::Quadretti4, Paper::Quadretti10]),
        ("Matematica e disegno", &[Paper::Millimetrata, Paper::Cartesiano, Paper::Puntinato, Paper::Isometrica]),
        ("Altro", &[Paper::Pentagramma, Paper::Cornell]),
    ];

    /// Valore in `settings.conf`.
    pub fn code(self) -> &'static str {
        match self {
            Paper::Bianco => "bianco",
            Paper::Righe => "righe",
            Paper::RigheLarghe => "righe10",
            Paper::Elem1 => "elem1",
            Paper::Elem2 => "elem2",
            Paper::Elem3 => "elem3",
            Paper::Quadretti5 => "quadretti5",
            Paper::Quadretti5M => "quadretti5m",
            Paper::Quadretti4 => "quadretti4",
            Paper::Quadretti10 => "quadretti10",
            Paper::Millimetrata => "millimetrata",
            Paper::Cartesiano => "cartesiano",
            Paper::Puntinato => "puntinato",
            Paper::Isometrica => "isometrica",
            Paper::Pentagramma => "pentagramma",
            Paper::Cornell => "cornell",
        }
    }

    pub fn from_code(s: &str) -> Paper {
        Paper::ALL.into_iter().find(|p| p.code() == s.trim()).unwrap_or(Paper::Bianco)
    }

    pub fn label(self) -> &'static str {
        match self {
            Paper::Bianco => "Bianco",
            Paper::Righe => "Righe (8 mm)",
            Paper::RigheLarghe => "Righe larghe (1 cm)",
            Paper::Elem1 => "Righe di 1ª elementare",
            Paper::Elem2 => "Righe di 2ª elementare",
            Paper::Elem3 => "Righe di 3ª elementare",
            Paper::Quadretti5 => "Quadretti piccoli (5 mm)",
            Paper::Quadretti5M => "Quadretti 5 mm con margine",
            Paper::Quadretti4 => "Quadretti piccoli (4 mm)",
            Paper::Quadretti10 => "Quadretti grandi (1 cm)",
            Paper::Millimetrata => "Carta millimetrata",
            Paper::Cartesiano => "Piano cartesiano",
            Paper::Puntinato => "Puntinato (5 mm)",
            Paper::Isometrica => "Isometrica (triangoli)",
            Paper::Pentagramma => "Pentagramma",
            Paper::Cornell => "Appunti Cornell",
        }
    }

    /// Foglio a righe (quelli con i margini rossi).
    pub fn ruled(self) -> bool {
        matches!(self, Paper::Righe | Paper::RigheLarghe | Paper::Elem1 | Paper::Elem2 | Paper::Elem3)
    }

    /// Distanza (cm) fra le linee normali più vicine: sotto qualche punto
    /// sullo schermo si sfumano.
    fn pitch(self) -> f32 {
        match self {
            Paper::Righe | Paper::Cornell => 0.8,
            Paper::Elem1 | Paper::Quadretti4 => 0.4,
            Paper::Elem2 | Paper::Elem3 => 0.3,
            Paper::Pentagramma => 0.2,
            Paper::Quadretti5 | Paper::Quadretti5M | Paper::Millimetrata | Paper::Cartesiano | Paper::Puntinato | Paper::Isometrica => 0.5,
            Paper::Bianco | Paper::RigheLarghe | Paper::Quadretti10 => 1.0,
        }
    }

    /// Tinta chiara da quadretti (le righe sono un po' più scure).
    fn grid_like(self) -> bool {
        matches!(
            self,
            Paper::Quadretti5 | Paper::Quadretti5M | Paper::Quadretti4 | Paper::Quadretti10 | Paper::Cartesiano | Paper::Puntinato | Paper::Isometrica
        )
    }
}

/// Margini rossi dei fogli a righe: stessa distanza dal bordo a sinistra e a
/// destra (cm), simmetrici.
const MARGIN_CM: f32 = 2.5;
const RULED_TOP_CM: f32 = 2.0;
const RULED_BOTTOM_CM: f32 = 1.2;

/// Tratto di una linea.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ink {
    /// Sottilissima (i millimetri).
    Fine,
    /// Normale.
    Line,
    /// Più marcata (1 cm della millimetrata, fasce delle elementari, Cornell).
    Strong,
    /// Margine rosso.
    Red,
    /// Assi del piano cartesiano.
    Axis,
}

type Pt = (f32, f32);

/// Un pezzo della rigatura, in cm dall'angolo in alto a sinistra.
#[derive(Clone, Debug, PartialEq)]
enum Mark {
    Line(Ink, Pt, Pt),
    Dot(Pt),
    /// Scritta centrata in quel punto.
    Label(Pt, String),
}

/// `from`, `from+step`, … finché resta prima di `to`.
fn seq(from: f32, to: f32, step: f32) -> Vec<f32> {
    let mut v = Vec::new();
    let mut i = 0;
    loop {
        let x = from + i as f32 * step;
        if x >= to - 1e-3 || v.len() >= 4000 {
            return v;
        }
        v.push(x);
        i += 1;
    }
}

fn hline(m: &mut Vec<Mark>, ink: Ink, y: f32, x0: f32, x1: f32) {
    m.push(Mark::Line(ink, (x0, y), (x1, y)));
}

fn vline(m: &mut Vec<Mark>, ink: Ink, x: f32, y0: f32, y1: f32) {
    m.push(Mark::Line(ink, (x, y0), (x, y1)));
}

fn grid(m: &mut Vec<Mark>, w: f32, h: f32, step: f32, ink: Ink) {
    for y in seq(step, h, step) {
        hline(m, ink, y, 0.0, w);
    }
    for x in seq(step, w, step) {
        vline(m, ink, x, 0.0, h);
    }
}

/// Le fasce delle elementari: in ogni riga le linee agli scostamenti
/// `offs` (cm), poi si va avanti di `pitch`.
fn bands(m: &mut Vec<Mark>, w: f32, h: f32, offs: &[(f32, Ink)], pitch: f32) {
    let last = offs.last().map(|o| o.0).unwrap_or(0.0);
    let mut y = RULED_TOP_CM;
    while y + last <= h - RULED_BOTTOM_CM + 1e-3 && m.len() < 4000 {
        for &(o, ink) in offs {
            hline(m, ink, y + o, 0.0, w);
        }
        y += pitch;
    }
}

fn margins(m: &mut Vec<Mark>, w: f32, h: f32) {
    if w > 2.0 * MARGIN_CM + 2.0 {
        vline(m, Ink::Red, MARGIN_CM, 0.0, h);
        vline(m, Ink::Red, w - MARGIN_CM, 0.0, h);
    }
}

/// Tutta la rigatura di un foglio di `w` × `h` cm.
fn layout(paper: Paper, w: f32, h: f32, margin: bool) -> Vec<Mark> {
    let mut m = Vec::new();
    if w <= 0.0 || h <= 0.0 {
        return m;
    }
    match paper {
        Paper::Bianco => {}
        Paper::Righe | Paper::RigheLarghe => {
            let s = if paper == Paper::Righe { 0.8 } else { 1.0 };
            for y in seq(RULED_TOP_CM, h - RULED_BOTTOM_CM + 1e-3, s) {
                hline(&mut m, Ink::Line, y, 0.0, w);
            }
        }
        // tre fasce: quella in mezzo (per le lettere piccole) fra due linee marcate
        Paper::Elem1 => bands(&mut m, w, h, &[(0.0, Ink::Line), (0.4, Ink::Strong), (0.8, Ink::Strong), (1.2, Ink::Line)], 1.6),
        Paper::Elem2 => bands(&mut m, w, h, &[(0.0, Ink::Line), (0.3, Ink::Strong), (0.6, Ink::Strong), (0.9, Ink::Line)], 1.2),
        Paper::Elem3 => bands(&mut m, w, h, &[(0.0, Ink::Strong), (0.3, Ink::Strong)], 1.0),
        Paper::Quadretti5 => grid(&mut m, w, h, 0.5, Ink::Line),
        Paper::Quadretti4 => grid(&mut m, w, h, 0.4, Ink::Line),
        Paper::Quadretti10 => grid(&mut m, w, h, 1.0, Ink::Line),
        Paper::Quadretti5M => {
            grid(&mut m, w, h, 0.5, Ink::Line);
            if w > MARGIN_CM + 2.0 {
                vline(&mut m, Ink::Red, MARGIN_CM, 0.0, h);
            }
        }
        Paper::Millimetrata => {
            // contando i millimetri a interi: niente errori di arrotondamento
            let ink = |i: i32| if i % 10 == 0 { Ink::Strong } else if i % 5 == 0 { Ink::Line } else { Ink::Fine };
            for i in 1..=((h - 1e-3) / 0.1).floor() as i32 {
                hline(&mut m, ink(i), i as f32 * 0.1, 0.0, w);
            }
            for i in 1..=((w - 1e-3) / 0.1).floor() as i32 {
                vline(&mut m, ink(i), i as f32 * 0.1, 0.0, h);
            }
        }
        Paper::Cartesiano => {
            grid(&mut m, w, h, 0.5, Ink::Line);
            if w < 4.0 || h < 4.0 {
                return m;
            }
            // origine su un centimetro intero vicino al centro
            let (ox, oy) = ((w / 2.0).round(), (h / 2.0).round());
            let (x0, x1, y0, y1) = (1.0, w - 1.0, 1.0, h - 1.0);
            hline(&mut m, Ink::Axis, oy, x0, x1);
            vline(&mut m, Ink::Axis, ox, y0, y1);
            // frecce
            m.push(Mark::Line(Ink::Axis, (x1, oy), (x1 - 0.3, oy - 0.15)));
            m.push(Mark::Line(Ink::Axis, (x1, oy), (x1 - 0.3, oy + 0.15)));
            m.push(Mark::Line(Ink::Axis, (ox, y0), (ox - 0.15, y0 + 0.3)));
            m.push(Mark::Line(Ink::Axis, (ox, y0), (ox + 0.15, y0 + 0.3)));
            m.push(Mark::Label((x1 - 0.1, oy + 0.45), "x".into()));
            m.push(Mark::Label((ox + 0.45, y0 + 0.1), "y".into()));
            m.push(Mark::Label((ox - 0.3, oy + 0.35), "O".into()));
            // tacche e numeri ogni centimetro
            let kx = (w / 2.0).ceil() as i32 + 1;
            for k in -kx..=kx {
                let x = ox + k as f32;
                if k == 0 || x < x0 + 0.5 || x > x1 - 0.5 {
                    continue;
                }
                vline(&mut m, Ink::Axis, x, oy - 0.12, oy + 0.12);
                m.push(Mark::Label((x, oy + 0.4), k.to_string()));
            }
            let ky = (h / 2.0).ceil() as i32 + 1;
            for k in -ky..=ky {
                let y = oy - k as f32;
                if k == 0 || y < y0 + 0.5 || y > y1 - 0.5 {
                    continue;
                }
                hline(&mut m, Ink::Axis, y, ox - 0.12, ox + 0.12);
                m.push(Mark::Label((ox - 0.45, y), k.to_string()));
            }
        }
        Paper::Puntinato => {
            for y in seq(0.5, h, 0.5) {
                for x in seq(0.5, w, 0.5) {
                    m.push(Mark::Dot((x, y)));
                }
            }
        }
        Paper::Isometrica => {
            // reticolo di triangoli equilateri di lato s: orizzontali ogni
            // s·√3/2 e due famiglie a ±60°, tagliate ai bordi del foglio
            let s = 0.5;
            let r3 = 3f32.sqrt();
            for y in seq(s * r3 / 2.0, h, s * r3 / 2.0) {
                hline(&mut m, Ink::Line, y, 0.0, w);
            }
            let clip = |ya: f32, yb: f32| (ya.max(0.0), yb.min(h));
            let n0 = (-(h / r3) / s).floor() as i32;
            let n1 = ((w + h / r3) / s).ceil() as i32;
            for n in n0..=n1 {
                let c = n as f32 * s;
                // x = c + y/√3
                let (ya, yb) = clip(-c * r3, (w - c) * r3);
                if yb - ya > 1e-3 {
                    m.push(Mark::Line(Ink::Line, (c + ya / r3, ya), (c + yb / r3, yb)));
                }
                // x = c − y/√3
                let (ya, yb) = clip((c - w) * r3, c * r3);
                if yb - ya > 1e-3 {
                    m.push(Mark::Line(Ink::Line, (c - ya / r3, ya), (c - yb / r3, yb)));
                }
            }
        }
        Paper::Pentagramma => {
            // cinque linee a 2 mm, un pentagramma ogni 2 cm, 1,5 cm dai lati
            let (x0, x1) = if w > 5.0 { (1.5, w - 1.5) } else { (0.0, w) };
            let mut y = RULED_TOP_CM;
            while y + 0.8 <= h - RULED_BOTTOM_CM + 1e-3 && m.len() < 4000 {
                for i in 0..5 {
                    hline(&mut m, Ink::Strong, y + i as f32 * 0.2, x0, x1);
                }
                vline(&mut m, Ink::Strong, x0, y, y + 0.8);
                vline(&mut m, Ink::Strong, x1, y, y + 0.8);
                y += 2.0;
            }
            // le stanghette ai lati non sono pentagrammi: le togliamo dal conto
            // nei test guardando solo le orizzontali
        }
        Paper::Cornell => {
            let top = RULED_TOP_CM;
            let sum = (h - 5.0).max(top + 2.0);
            let cue = if w > 14.0 { 6.0 } else { w * 0.3 };
            hline(&mut m, Ink::Strong, top, 0.0, w);
            for y in seq(top + 0.8, sum - 0.2, 0.8) {
                hline(&mut m, Ink::Line, y, 0.0, w);
            }
            vline(&mut m, Ink::Strong, cue, top, sum);
            hline(&mut m, Ink::Strong, sum, 0.0, w);
            for y in seq(sum + 0.8, h - RULED_BOTTOM_CM + 1e-3, 0.8) {
                hline(&mut m, Ink::Line, y, 0.0, w);
            }
            m.push(Mark::Label((cue / 2.0, top + 0.4), "Parole chiave".into()));
            m.push(Mark::Label(((cue + w) / 2.0, top + 0.4), "Appunti".into()));
            m.push(Mark::Label((w / 2.0, sum + 0.4), "Riassunto".into()));
        }
    }
    if margin && paper.ruled() {
        margins(&mut m, w, h);
    }
    m
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(m: &[Mark], want: impl Fn(Ink) -> bool) -> Vec<(Pt, Pt)> {
        m.iter()
            .filter_map(|k| match k {
                Mark::Line(i, a, b) if want(*i) => Some((*a, *b)),
                _ => None,
            })
            .collect()
    }

    fn horiz(m: &[Mark], ink: Ink) -> usize {
        lines(m, |i| i == ink).iter().filter(|(a, b)| (a.1 - b.1).abs() < 1e-4).count()
    }

    fn labels(m: &[Mark]) -> Vec<String> {
        m.iter()
            .filter_map(|k| match k {
                Mark::Label(_, s) => Some(s.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn tutte_le_rigature() {
        let (w, h) = (21.0, 29.7); // A4 verticale
        for p in Paper::ALL {
            assert_eq!(Paper::from_code(p.code()), p);
            if p != Paper::Bianco {
                assert!(Paper::GROUPS.iter().any(|(_, g)| g.contains(&p)), "{p:?} manca nei gruppi");
            }
        }
        assert_eq!(Paper::from_code("boh"), Paper::Bianco);
        assert!(layout(Paper::Bianco, w, h, true).is_empty());

        // quadretti da 5 mm: 59 righe e 41 colonne, nessun margine rosso
        let q = layout(Paper::Quadretti5, w, h, true);
        assert_eq!(lines(&q, |i| i == Ink::Line).len(), 59 + 41);
        assert!(lines(&q, |i| i == Ink::Red).is_empty());
        let q = layout(Paper::Quadretti10, w, h, true);
        assert_eq!(lines(&q, |_| true).len(), 29 + 20);
        assert_eq!(lines(&layout(Paper::Quadretti5M, w, h, false), |i| i == Ink::Red).len(), 1);

        // righe da 8 mm dai 2 cm in giù e i due margini simmetrici
        let r = layout(Paper::Righe, w, h, true);
        let ys: Vec<f32> = lines(&r, |i| i == Ink::Line).iter().map(|(a, _)| a.1).collect();
        assert!((ys[0] - 2.0).abs() < 1e-4 && (ys[1] - 2.8).abs() < 1e-4);
        assert!(*ys.last().unwrap() <= h - 1.2 + 1e-3);
        let red: Vec<f32> = lines(&r, |i| i == Ink::Red).iter().map(|(a, _)| a.0).collect();
        assert_eq!(red, vec![2.5, w - 2.5]);
        assert!(lines(&layout(Paper::Righe, w, h, false), |i| i == Ink::Red).is_empty());

        // elementari: righe intere (4 linee in 1ª e 2ª, 2 in 3ª)
        for (p, k) in [(Paper::Elem1, 4), (Paper::Elem2, 4), (Paper::Elem3, 2)] {
            let n = lines(&layout(p, w, h, false), |_| true).len();
            assert!(n > 0 && n % k == 0, "{p:?}: {n}");
        }

        // millimetrata: linee del centimetro come i quadretti da 1 cm
        let mm = layout(Paper::Millimetrata, w, h, false);
        assert_eq!(lines(&mm, |i| i == Ink::Strong).len(), 29 + 20);
        assert_eq!(lines(&mm, |_| true).len(), 296 + 209);

        let c = labels(&layout(Paper::Cartesiano, w, h, false));
        assert!(c.contains(&"x".to_string()) && c.contains(&"y".to_string()) && c.contains(&"-5".to_string()));

        let dots = layout(Paper::Puntinato, w, h, false).iter().filter(|k| matches!(k, Mark::Dot(_))).count();
        assert_eq!(dots, 59 * 41);

        let iso = layout(Paper::Isometrica, w, h, false);
        assert!(iso.len() > 100);
        for (a, b) in lines(&iso, |_| true) {
            for p in [a, b] {
                assert!(p.0 >= -1e-3 && p.0 <= w + 1e-3 && p.1 >= -1e-3 && p.1 <= h + 1e-3, "{p:?}");
            }
        }

        let n = horiz(&layout(Paper::Pentagramma, w, h, false), Ink::Strong);
        assert!(n > 0 && n % 5 == 0);

        assert!(labels(&layout(Paper::Cornell, w, h, false)).contains(&"Riassunto".to_string()));
    }
}
