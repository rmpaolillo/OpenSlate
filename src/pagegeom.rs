//! Geometria della pagina della lavagna: misure A4, rotazione della vista,
//! passaggio da coordinate del pad a coordinate 0..1 della pagina, e la
//! pagina pronta per disegnarla. Senza dipendenze dall'interfaccia: la usano
//! la desktop (`scene` la riesporta) e le app per telefono (`mobile/core`).

use crate::stroke::{Drawing, Rgb};
use std::sync::Arc;

/// Misure A4 in unità del pad (10 µm), pad in orientamento nativo
/// orizzontale. Usate se il pad non comunica le sue.
pub const A4_LONG: f32 = 29_700.0;
pub const A4_SHORT: f32 = 21_000.0;

/// Rotazione (passi di 90°) e specchiature.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct View {
    pub rot: i32,
    pub flip_x: bool,
    pub flip_y: bool,
}

impl View {
    pub fn transform(&self, x: f32, y: f32) -> (f32, f32) {
        let (mut x, mut y) = match self.rot.rem_euclid(4) {
            1 => (y, -x),
            2 => (-x, -y),
            3 => (-y, x),
            _ => (x, y),
        };
        if self.flip_x {
            x = -x;
        }
        if self.flip_y {
            y = -y;
        }
        (x, y)
    }
}

/// Geometria della pagina dopo rotazione: per passare da coordinate del pad a
/// coordinate normalizzate 0..1 sulla pagina visualizzata.
#[derive(Clone, Copy, Debug)]
pub struct PageGeom {
    view: View,
    min_x: f32,
    min_y: f32,
    /// Larghezza e altezza della pagina come appare (unità pad).
    pub w: f32,
    pub h: f32,
}

impl PageGeom {
    pub fn new(view: View, pad_w: f32, pad_h: f32) -> Self {
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for (cx, cy) in [(0.0, 0.0), (pad_w, 0.0), (0.0, pad_h), (pad_w, pad_h)] {
            let (tx, ty) = view.transform(cx, cy);
            x0 = x0.min(tx);
            y0 = y0.min(ty);
            x1 = x1.max(tx);
            y1 = y1.max(ty);
        }
        Self {
            view,
            min_x: x0,
            min_y: y0,
            w: (x1 - x0).max(1.0),
            h: (y1 - y0).max(1.0),
        }
    }

    /// Punto del pad → (u, v) in 0..1 sulla pagina visualizzata.
    pub fn unit(&self, x: f32, y: f32) -> (f32, f32) {
        let (tx, ty) = self.view.transform(x, y);
        ((tx - self.min_x) / self.w, (ty - self.min_y) / self.h)
    }

    /// Inverso di `unit`: (u, v) sulla pagina visualizzata → punto del pad.
    pub fn pad(&self, u: f32, v: f32) -> (f32, f32) {
        let (mut tx, mut ty) = (u * self.w + self.min_x, v * self.h + self.min_y);
        if self.view.flip_x {
            tx = -tx;
        }
        if self.view.flip_y {
            ty = -ty;
        }
        match self.view.rot.rem_euclid(4) {
            1 => (-ty, tx),
            2 => (-tx, -ty),
            3 => (ty, -tx),
            _ => (tx, ty),
        }
    }
}

/// Un tratto già proiettato sulla pagina (coordinate 0..1).
#[derive(Clone)]
pub struct PageStroke {
    pub color: Rgb,
    pub width: f32,
    pub pts: Vec<(f32, f32)>,
    /// Pressione di ogni punto, per il tratto naturale (`inkshape`). Vuota:
    /// spessore uniforme.
    pub pr: Vec<f32>,
    /// Evidenziatore (le pagine li mettono prima dell'inchiostro).
    pub highlight: bool,
}

/// Sfondo di una pagina (pagina di un documento importato).
#[derive(Clone, Debug)]
pub struct PageBg {
    /// Immagine PNG della pagina del documento.
    pub png: Arc<Vec<u8>>,
    /// Dove sta sulla pagina: u, v, larghezza, altezza (0..1).
    pub rect: [f32; 4],
    /// Identifica l'immagine (documento + pagina), per le cache.
    pub key: String,
}

impl PageBg {
    /// Un grafico messo dall'AI sulla lavagna (non la pagina di un
    /// documento): sta col foglio, si salva nell'archivio con lui.
    pub fn is_chart(&self) -> bool {
        self.key.starts_with("chart:")
    }
}

/// Una pagina pronta per export/web/OCR.
#[derive(Default)]
pub struct Page {
    /// Proporzioni della pagina visualizzata (unità pad).
    pub w: f32,
    pub h: f32,
    pub strokes: Vec<PageStroke>,
    /// Documento sotto i tratti, se c'è.
    pub bg: Option<PageBg>,
}

impl Page {
    pub fn from_drawing(d: &Drawing, view: View) -> Page {
        let (pw, ph) = page_dims(d);
        let g = PageGeom::new(view, pw, ph);
        Page {
            w: g.w,
            h: g.h,
            // Prima gli evidenziatori, poi l'inchiostro: così chi disegna la
            // pagina in ordine (SVG, PDF, PNG, web) li mette sotto.
            strokes: d
                .strokes
                .iter()
                .filter(|s| s.highlight)
                .chain(d.strokes.iter().filter(|s| !s.highlight))
                // i timbri (documenti) non sono tratti da disegnare
                .filter(|s| !s.points.is_empty() && s.is_ink())
                .map(|s| PageStroke {
                    color: s.color,
                    width: s.width,
                    pts: s.points.iter().map(|p| g.unit(p.x, p.y)).collect(),
                    pr: crate::inkshape::page_pressure(s.points.iter().map(|p| p.pressure), s.highlight),
                    highlight: s.highlight,
                })
                .collect(),
            bg: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.strokes.is_empty()
    }
}

/// Dimensioni del pad: quelle lette dal dispositivo o A4.
pub fn page_dims(d: &Drawing) -> (f32, f32) {
    if d.width > 0.0 && d.height > 0.0 {
        (d.width, d.height)
    } else {
        (A4_LONG, A4_SHORT)
    }
}

/// Dove sta l'immagine della pagina dentro la pagina del pad (0..1): la pagina
/// del documento viene adattata mantenendo le proporzioni, centrata.
pub fn fit_rect(doc_w: f32, doc_h: f32, pad_w: f32, pad_h: f32) -> [f32; 4] {
    let s = (pad_w / doc_w).min(pad_h / doc_h);
    let (w, h) = (doc_w * s / pad_w, doc_h * s / pad_h);
    [(1.0 - w) / 2.0, (1.0 - h) / 2.0, w, h]
}
