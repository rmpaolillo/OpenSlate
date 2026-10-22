//! Le pagine delle immagini del README: un esercizio di matematica, appunti di
//! laboratorio, qualche pagina per l'archivio e un modulo Word da firmare,
//! scritti «a mano» con i caratteri di handwrite (fanno da scrittura del pad).
//!
//!   DEMO_OUT=<cartella dati> cargo test -- --ignored pagine_dimostrative
//!
//! Poi l'app con OPENSLATE_DATA=<cartella dati> le trova nell'archivio e fra
//! i documenti. Solo per le prove: non entra nel programma.

use crate::boardread::tests::geom;
use crate::handwrite::{layout, Style};
use crate::library::{Library, Meta};
use crate::scene::PageGeom;
use crate::stroke::{Drawing, PenEvent, PenPhase, Stroke};
use chrono::NaiveDateTime;

const INK: [u8; 3] = [24, 28, 48];
const BLUE: [u8; 3] = [32, 70, 170];

/// Righe di testo a mano: (testo, u, v) in unità della pagina.
fn hand(g: &PageGeom, lines: &[(&str, f32, f32)], style: Style, cap: f32, color: [u8; 3], width: f32) -> Vec<Stroke> {
    let aspect = g.w / g.h;
    let mut out = Vec::new();
    for (i, (t, u, v)) in lines.iter().enumerate() {
        let laid = layout(t, style, cap, aspect, (*u, *v), 0.98 - u, false);
        assert!(laid.missing.is_empty(), "{t}: mancano {:?}", laid.missing);
        for (k, pts) in laid.strokes.into_iter().enumerate() {
            let mut st = Stroke::new(color, width);
            let n = pts.len().max(1) as f32;
            for (j, (u, v)) in pts.into_iter().enumerate() {
                let (x, y) = g.pad(u, v);
                // la pressione cresce e cala lungo il tratto, come col pennino
                let t = j as f32 / n;
                let p = 0.45 + 0.25 * (std::f32::consts::PI * t).sin() + 0.03 * ((i * 7 + k) % 5) as f32;
                st.points.push(PenEvent { x, y, pressure: p, phase: if j == 0 { PenPhase::Down } else { PenPhase::Move } });
            }
            out.push(st);
        }
    }
    out
}

/// Una riga dritta (sottolineature, linee di frazione).
fn rule(g: &PageGeom, a: (f32, f32), b: (f32, f32), color: [u8; 3], width: f32) -> Stroke {
    let mut st = Stroke::new(color, width);
    for k in 0..=12 {
        let t = k as f32 / 12.0;
        let wobble = 0.0015 * (t * 9.0).sin();
        let (x, y) = g.pad(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t + wobble);
        st.points.push(PenEvent { x, y, pressure: 0.55, phase: if k == 0 { PenPhase::Down } else { PenPhase::Move } });
    }
    st
}

fn highlight(g: &PageGeom, a: (f32, f32), b: (f32, f32)) -> Stroke {
    let mut st = rule(g, a, b, [250, 214, 70], 9.0);
    st.highlight = true;
    st
}

fn save(lib: &mut Library, g: &PageGeom, strokes: Vec<Stroke>, title: &str, when: &str) -> String {
    let _ = g;
    let d = Drawing { strokes, width: 29700.0, height: 21000.0 };
    let mut meta = Meta::new(NaiveDateTime::parse_from_str(when, "%Y-%m-%d %H:%M:%S").unwrap(), crate::library::SOURCE_PAD);
    meta.title = title.into();
    lib.add(&d, meta).unwrap()
}

/// L'esercizio di analisi, con un errore alla derivata (per la correzione).
pub fn exercise(g: &PageGeom) -> Vec<Stroke> {
    let cap = 0.019;
    let x0 = 0.10;
    let rows = [
        ("1) Domain: all real x", x0, 0.165),
        ("2) Zeros: x² − 4x + 3 = 0", x0, 0.225),
        ("Δ = 16 − 12 = 4", x0 + 0.06, 0.275),
        ("x = (4 ± 2) / 2  →  x = 3,  x = 1", x0 + 0.06, 0.325),
        ("3) Derivative: f'(x) = 2x − 3", x0, 0.395),
        ("4) f'(x) = 0  →  x = 3/2", x0, 0.455),
        ("5) Minimum: f(3/2) = −3/4", x0, 0.515),
    ];
    let mut s = hand(g, &[("Exercise 4:  f(x) = x² − 4x + 3", x0, 0.085)], Style::Stampatello, 0.021, BLUE, 2.2);
    s.push(rule(g, (x0, 0.118), (0.78, 0.116), BLUE, 2.0));
    s.extend(hand(g, &rows, Style::Stampatello, cap, INK, 2.0));
    s
}

fn lab(g: &PageGeom) -> Vec<Stroke> {
    let x0 = 0.10;
    let mut s = hand(g, &[("Physics lab: the pendulum", x0, 0.085)], Style::Corsivo, 0.026, BLUE, 2.2);
    s.push(rule(g, (x0, 0.125), (0.62, 0.123), BLUE, 2.0));
    s.extend(hand(
        g,
        &[
            ("Goal: measure g with a simple pendulum.", x0, 0.18),
            ("Period: T = 2π √(L / g)", x0, 0.235),
            ("L = 0.50 m   T = 1.42 s", x0 + 0.04, 0.30),
            ("L = 1.00 m   T = 2.01 s", x0 + 0.04, 0.345),
            ("L = 1.50 m   T = 2.46 s", x0 + 0.04, 0.39),
            ("So g = 4π² L / T² ≈ 9.8 m/s²", x0, 0.455),
            ("To do: repeat with 20 swings,", x0, 0.53),
            ("not 10, and check the angle.", x0, 0.575),
        ],
        Style::Corsivo,
        0.019,
        INK,
        2.0,
    ));
    s.push(highlight(g, (x0, 0.467), (0.66, 0.467)));
    s
}

fn shopping(g: &PageGeom) -> Vec<Stroke> {
    let mut s = hand(g, &[("Saturday", 0.12, 0.09)], Style::Corsivo, 0.028, BLUE, 2.2);
    s.extend(hand(
        g,
        &[("• bread, milk, eggs", 0.12, 0.17), ("• tomatoes (2 kg)", 0.12, 0.22), ("• basil", 0.12, 0.27), ("• birthday card for Anna", 0.12, 0.32)],
        Style::Corsivo,
        0.021,
        INK,
        2.0,
    ));
    s.push(highlight(g, (0.13, 0.333), (0.56, 0.333)));
    s
}

fn ideas(g: &PageGeom) -> Vec<Stroke> {
    let mut s = hand(g, &[("App ideas", 0.12, 0.09)], Style::Stampatello, 0.026, [190, 40, 40], 2.2);
    s.extend(hand(
        g,
        &[
            ("- dark theme with squared paper", 0.12, 0.17),
            ("- sign PDFs from the pad", 0.12, 0.22),
            ("- the AI checks my homework", 0.12, 0.27),
            ("- export to Word", 0.12, 0.32),
        ],
        Style::Stampatello,
        0.019,
        INK,
        2.0,
    ));
    s
}

fn triangle(g: &PageGeom) -> Vec<Stroke> {
    let mut s = hand(g, &[("Pythagoras", 0.12, 0.09)], Style::Stampatello, 0.026, BLUE, 2.2);
    let (a, b, c) = ((0.20, 0.42), (0.62, 0.42), (0.20, 0.20));
    for (p, q) in [(a, b), (b, c), (c, a)] {
        s.push(rule(g, p, q, INK, 2.2));
    }
    s.push(rule(g, (0.20, 0.395), (0.235, 0.395), INK, 1.6));
    s.push(rule(g, (0.235, 0.395), (0.235, 0.42), INK, 1.6));
    s.extend(hand(
        g,
        &[("a", 0.15, 0.30), ("b", 0.40, 0.435), ("c", 0.43, 0.28), ("c² = a² + b²", 0.14, 0.52), ("3² + 4² = 9 + 16 = 25 = 5²", 0.14, 0.58)],
        Style::Stampatello,
        0.020,
        INK,
        2.0,
    ));
    s
}

fn plan(g: &PageGeom) -> Vec<Stroke> {
    let mut s = hand(g, &[("This week", 0.12, 0.09)], Style::Corsivo, 0.028, [40, 130, 70], 2.2);
    s.extend(hand(
        g,
        &[
            ("Mon: maths test (limits)", 0.12, 0.17),
            ("Tue: football 18:00", 0.12, 0.22),
            ("Wed: lab report due", 0.12, 0.27),
            ("Thu: dentist 9:15", 0.12, 0.32),
            ("Fri: pizza with Luca", 0.12, 0.37),
        ],
        Style::Corsivo,
        0.020,
        INK,
        2.0,
    ));
    s.push(highlight(g, (0.13, 0.183), (0.60, 0.183)));
    s
}

fn meeting(g: &PageGeom) -> Vec<Stroke> {
    let mut s = hand(g, &[("Meeting 14:30", 0.12, 0.09)], Style::Corsivo, 0.027, INK, 2.2);
    s.extend(hand(
        g,
        &[("Budget: 12k, confirm with Lisa", 0.12, 0.17), ("Launch: end of March", 0.12, 0.22), ("Next: Friday 10:00, room B", 0.12, 0.27)],
        Style::Corsivo,
        0.020,
        INK,
        2.0,
    ));
    s.push(rule(g, (0.12, 0.30), (0.52, 0.298), [190, 40, 40], 2.0));
    s
}

/// La firma (in corsivo) e la data sulle righe del modulo: (u, v) dentro la
/// pagina del documento, 0..1.
fn signature(g: &PageGeom, rect: [f32; 4], sign_at: (f32, f32), date_at: (f32, f32)) -> Vec<Stroke> {
    let to_pad = |(u, v): (f32, f32)| (rect[0] + u * rect[2], rect[1] + v * rect[3]);
    let mut s = hand(g, &[("Marco Rossi", to_pad(sign_at).0, to_pad(sign_at).1)], Style::Corsivo, 0.024 * rect[3], BLUE, 2.4);
    s.extend(hand(g, &[("22/10/2026", to_pad(date_at).0, to_pad(date_at).1)], Style::Stampatello, 0.014 * rect[3], BLUE, 2.0));
    s
}

#[test]
#[ignore]
fn pagine_dimostrative() {
    let Some(out) = std::env::var_os("DEMO_OUT").map(std::path::PathBuf::from) else {
        eprintln!("DEMO_OUT non c'è: niente da fare");
        return;
    };
    std::fs::create_dir_all(&out).unwrap();
    let g = geom();
    let mut lib = Library::open_in(&out.join(crate::library::LIB_DIR));
    let pages = [
        (meeting(&g), "Meeting", "2026-10-20 14:32:00"),
        (ideas(&g), "App ideas", "2026-10-20 21:05:00"),
        (plan(&g), "This week", "2026-10-21 08:02:00"),
        (triangle(&g), "Pythagoras", "2026-10-21 10:15:00"),
        (shopping(&g), "Shopping list", "2026-10-21 18:12:00"),
        (lab(&g), "Physics lab", "2026-10-22 11:40:00"),
        (exercise(&g), "Exercise 4", "2026-10-22 16:20:00"),
    ];
    for (s, t, w) in pages {
        let id = save(&mut lib, &g, s, t, w);
        println!("pagina {id}: {t}");
    }
    // il modulo da firmare: importato come fa l'app, poi la firma in fondo
    if let Some(docx) = std::env::var_os("DEMO_DOCX") {
        let here = std::env::current_dir().unwrap();
        std::env::set_current_dir(&out).unwrap();
        let mut doc = crate::document::Document::import(std::path::Path::new(&docx)).unwrap();
        let (w, h) = doc.pages[0].pdf.display_size();
        let rect = crate::document::fit_rect(w, h, g.w, g.h);
        doc.pages[0].strokes = signature(&g, rect, (0.50, 0.508), (0.135, 0.522));
        doc.save().unwrap();
        println!("documento {}", doc.dir.display());
        std::env::set_current_dir(here).unwrap();
    }
}
