//! Timbro aziendale: ragione sociale, indirizzo, P. IVA… con o senza
//! cornice, reso come un timbro vero (grana, inchiostro irregolare,
//! inclinazione, colore e trasparenza).
//!
//! Sul foglio un timbro è un tratto speciale (`Stroke::stamp`): i suoi punti
//! sono i quattro angoli del riquadro (in alto a sinistra, in alto a destra,
//! in basso a destra, in basso a sinistra, come lo si vede), così annulla,
//! gomma e salvataggio funzionano come per l'inchiostro. L'immagine non si
//! salva: si rifà da `StampSpec` (il seme rende la grana sempre uguale) alla
//! risoluzione che serve.

use crate::scene::PageGeom;
use crate::stroke::{PenEvent, PenPhase};
use ab_glyph::{Font, FontRef, GlyphId};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Transform};

static FONT_BOLD: &[u8] = include_bytes!("../assets/fonts/Inter-SemiBold.ttf");
static FONT_TEXT: &[u8] = include_bytes!("../assets/fonts/Inter-Medium.ttf");

/// I dati dell'azienda. I campi vuoti non vanno sul timbro.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(openslate_mobile, derive(uniffi::Record))]
pub struct Company {
    /// Ragione sociale.
    pub name: String,
    pub address: String,
    pub cap: String,
    pub city: String,
    /// Sigla della provincia (GE, MI…).
    pub prov: String,
    /// Partita IVA.
    pub vat: String,
    pub rea: String,
    /// Codice univoco (SDI).
    pub sdi: String,
    /// Posta elettronica certificata.
    pub pec: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(openslate_mobile, derive(uniffi::Enum))]
pub enum Frame {
    /// Senza cornice.
    Plain,
    #[default]
    Rect,
    Rounded,
    Double,
    Oval,
}

impl Frame {
    pub const ALL: [Frame; 5] = [Frame::Plain, Frame::Rect, Frame::Rounded, Frame::Double, Frame::Oval];
}

/// Colori da inchiostro per timbri (non saturi, come quelli veri).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(openslate_mobile, derive(uniffi::Enum))]
pub enum InkColor {
    #[default]
    Blue,
    Grey,
    Black,
    Red,
    Violet,
}

impl InkColor {
    pub const ALL: [InkColor; 5] = [InkColor::Blue, InkColor::Grey, InkColor::Black, InkColor::Red, InkColor::Violet];

    pub fn rgb(self) -> [u8; 3] {
        match self {
            InkColor::Blue => [34, 62, 150],
            InkColor::Grey => [92, 94, 100],
            InkColor::Black => [32, 32, 36],
            InkColor::Red => [178, 38, 44],
            InkColor::Violet => [98, 52, 142],
        }
    }
}

/// Come appare il timbro: le regolazioni che si fanno dopo averlo posato.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(openslate_mobile, derive(uniffi::Record))]
pub struct StampStyle {
    pub frame: Frame,
    pub color: InkColor,
    /// 0.3..1: quanto copre l'inchiostro (il testo sotto si vede).
    pub opacity: f32,
    /// 0..1: inchiostro che manca a puntini e chiazze.
    pub grain: f32,
    /// 0..1: un lato più scarico dell'altro.
    pub uneven: f32,
    /// Da che parte è scarico (gradi, 0 = a destra, 90 = in basso).
    pub uneven_dir: f32,
    /// Inclinazione (gradi, positivi in senso orario), -15..15.
    pub tilt: f32,
    /// Seme della grana: lo stesso seme dà la stessa grana.
    pub seed: u32,
}

impl Default for StampStyle {
    fn default() -> Self {
        StampStyle { frame: Frame::Rect, color: InkColor::Blue, opacity: 0.85, grain: 0.35, uneven: 0.3, uneven_dir: 20.0, tilt: -2.0, seed: 1 }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(openslate_mobile, derive(uniffi::Record))]
pub struct StampSpec {
    pub company: Company,
    pub style: StampStyle,
}

/// Un timbro salvato con il suo nome.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(openslate_mobile, derive(uniffi::Record))]
pub struct SavedStamp {
    pub name: String,
    pub spec: StampSpec,
}

impl StampSpec {
    /// Impronta per le cache delle immagini.
    pub fn key(&self) -> u64 {
        let s = serde_json::to_string(self).unwrap_or_default();
        let mut h: u64 = 1469598103934665603;
        for b in s.bytes() {
            h = (h ^ b as u64).wrapping_mul(1099511628211);
        }
        h
    }

    /// Le righe del timbro (testo, grassetto); le vuote si saltano.
    pub fn lines(&self) -> Vec<(String, bool)> {
        let c = &self.company;
        let t = |s: &str| s.trim().to_string();
        let mut out = Vec::new();
        if !t(&c.name).is_empty() {
            out.push((t(&c.name), true));
        }
        if !t(&c.address).is_empty() {
            out.push((t(&c.address), false));
        }
        let mut place = [t(&c.cap), t(&c.city)].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ");
        if !t(&c.prov).is_empty() {
            let p = format!("({})", t(&c.prov).to_uppercase());
            place = if place.is_empty() { p } else { format!("{place} {p}") };
        }
        if !place.is_empty() {
            out.push((place, false));
        }
        let mut fiscal = Vec::new();
        if !t(&c.vat).is_empty() {
            fiscal.push(format!("P.IVA {}", t(&c.vat)));
        }
        if !t(&c.rea).is_empty() {
            fiscal.push(format!("REA {}", t(&c.rea)));
        }
        if !fiscal.is_empty() {
            out.push((fiscal.join(" – "), false));
        }
        if !t(&c.sdi).is_empty() {
            out.push((format!("Codice Univoco: {}", t(&c.sdi).to_uppercase()), false));
        }
        if !t(&c.pec).is_empty() {
            out.push((format!("PEC: {}", t(&c.pec)), false));
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Impaginazione (in cm)

const EM_NAME: f32 = 0.46;
const EM_TEXT: f32 = 0.31;
const LEAD: f32 = 1.22;

struct Fonts {
    bold: FontRef<'static>,
    text: FontRef<'static>,
}

fn fonts() -> Fonts {
    Fonts {
        bold: FontRef::try_from_slice(FONT_BOLD).expect("font Inter SemiBold"),
        text: FontRef::try_from_slice(FONT_TEXT).expect("font Inter Medium"),
    }
}

fn line_width(f: &FontRef, text: &str, em: f32) -> f32 {
    let k = em / f.units_per_em().unwrap_or(1000.0);
    let mut w = 0.0;
    let mut prev: Option<GlyphId> = None;
    for c in text.chars() {
        let id = f.glyph_id(c);
        if let Some(p) = prev {
            w += f.kern_unscaled(p, id) * k;
        }
        w += f.h_advance_unscaled(id) * k;
        prev = Some(id);
    }
    w
}

/// Misure del timbro dritto: (larghezza, altezza, righe con (testo,
/// grassetto, em, x sinistra, linea di base)), tutto in cm.
struct Layout {
    w: f32,
    h: f32,
    lines: Vec<(String, bool, f32, f32, f32)>,
}

fn layout(spec: &StampSpec, f: &Fonts) -> Layout {
    let mut lines = spec.lines();
    if lines.is_empty() {
        lines.push(("Ragione sociale".into(), true));
    }
    let sized: Vec<(String, bool, f32, f32)> = lines
        .into_iter()
        .map(|(t, b)| {
            let em = if b { EM_NAME } else { EM_TEXT };
            let w = line_width(if b { &f.bold } else { &f.text }, &t, em);
            (t, b, em, w)
        })
        .collect();
    let cw = sized.iter().map(|l| l.3).fold(0.0, f32::max);
    // altezza: dalla cima delle maiuscole della prima riga al fondo della
    // base dell'ultima (cifre e maiuscole: niente discendenti di solito)
    const CAP: f32 = 0.73;
    let mut y = 0.0;
    let mut bases = Vec::new();
    for (i, l) in sized.iter().enumerate() {
        y += if i == 0 { l.2 * CAP } else { l.2 * LEAD };
        bases.push(y);
    }
    let ch = y + sized.last().map(|l| l.2 * 0.12).unwrap_or(0.0);
    let (px, py) = match spec.style.frame {
        Frame::Plain => (0.12, 0.12),
        Frame::Rect | Frame::Rounded => (0.34, 0.26),
        Frame::Double => (0.44, 0.36),
        Frame::Oval => (cw * 0.2 + 0.3, ch * 0.32 + 0.22),
    };
    let (w, h) = (cw + 2.0 * px, ch + 2.0 * py);
    let lines = sized
        .into_iter()
        .zip(bases)
        .map(|((t, b, em, lw), base)| (t, b, em, (w - lw) / 2.0, py + base))
        .collect();
    Layout { w, h, lines }
}

/// Grandezza del timbro vero (cm), inclinazione compresa.
pub fn size_cm(spec: &StampSpec) -> (f32, f32) {
    let l = layout(spec, &fonts());
    tilted(l.w, l.h, spec.style.tilt)
}

const EDGE: f32 = 0.06;

fn tilted(w: f32, h: f32, deg: f32) -> (f32, f32) {
    let a = deg.to_radians();
    let (s, c) = (a.sin().abs(), a.cos().abs());
    (w * c + h * s + 2.0 * EDGE, w * s + h * c + 2.0 * EDGE)
}

fn glyphs_path(pb: &mut PathBuilder, f: &FontRef, text: &str, em: f32, x0: f32, base: f32) {
    let k = em / f.units_per_em().unwrap_or(1000.0);
    let mut x = x0;
    let mut prev: Option<GlyphId> = None;
    for c in text.chars() {
        let id = f.glyph_id(c);
        if let Some(p) = prev {
            x += f.kern_unscaled(p, id) * k;
        }
        if let Some(o) = f.outline(id) {
            let m = |p: ab_glyph::Point| (x + p.x * k, base - p.y * k);
            let mut last: Option<(f32, f32)> = None;
            for cv in &o.curves {
                use ab_glyph::OutlineCurve::*;
                let start = match cv {
                    Line(a, _) | Quad(a, _, _) | Cubic(a, _, _, _) => m(*a),
                };
                if last.is_none_or(|l| (l.0 - start.0).abs() > 1e-4 || (l.1 - start.1).abs() > 1e-4) {
                    if last.is_some() {
                        pb.close();
                    }
                    pb.move_to(start.0, start.1);
                }
                let end = match cv {
                    Line(_, b) => {
                        let b = m(*b);
                        pb.line_to(b.0, b.1);
                        b
                    }
                    Quad(_, c1, b) => {
                        let (c1, b) = (m(*c1), m(*b));
                        pb.quad_to(c1.0, c1.1, b.0, b.1);
                        b
                    }
                    Cubic(_, c1, c2, b) => {
                        let (c1, c2, b) = (m(*c1), m(*c2), m(*b));
                        pb.cubic_to(c1.0, c1.1, c2.0, c2.1, b.0, b.1);
                        b
                    }
                };
                last = Some(end);
            }
            if last.is_some() {
                pb.close();
            }
        }
        x += f.h_advance_unscaled(id) * k;
        prev = Some(id);
    }
}

fn rounded_rect(pb: &mut PathBuilder, x0: f32, y0: f32, x1: f32, y1: f32, r: f32) {
    let r = r.min((x1 - x0) / 2.0).min((y1 - y0) / 2.0);
    let k = 0.5523 * r;
    pb.move_to(x0 + r, y0);
    pb.line_to(x1 - r, y0);
    pb.cubic_to(x1 - r + k, y0, x1, y0 + r - k, x1, y0 + r);
    pb.line_to(x1, y1 - r);
    pb.cubic_to(x1, y1 - r + k, x1 - r + k, y1, x1 - r, y1);
    pb.line_to(x0 + r, y1);
    pb.cubic_to(x0 + r - k, y1, x0, y1 - r + k, x0, y1 - r);
    pb.line_to(x0, y0 + r);
    pb.cubic_to(x0, y0 + r - k, x0 + r - k, y0, x0 + r, y0);
    pb.close();
}

/// Un'immagine RGBA a 8 bit, premoltiplicata.
#[derive(Clone)]
pub struct Raster {
    pub w: u32,
    pub h: u32,
    pub px: Vec<u8>,
}

impl Raster {
    /// PNG (non premoltiplicato, come vuole il formato).
    pub fn png(&self) -> Vec<u8> {
        let size = tiny_skia::IntSize::from_wh(self.w.max(1), self.h.max(1)).unwrap();
        Pixmap::from_vec(self.px.clone(), size).and_then(|p| p.encode_png().ok()).unwrap_or_default()
    }

    /// Ruotata di `q` quarti di giro in senso orario.
    pub fn rotated(&self, q: u8) -> Raster {
        let q = q % 4;
        if q == 0 {
            return self.clone();
        }
        let (w, h) = (self.w as usize, self.h as usize);
        let (nw, nh) = if q % 2 == 1 { (h, w) } else { (w, h) };
        let mut px = vec![0u8; nw * nh * 4];
        for y in 0..h {
            for x in 0..w {
                let (nx, ny) = match q {
                    1 => (h - 1 - y, x),
                    2 => (w - 1 - x, h - 1 - y),
                    _ => (y, w - 1 - x),
                };
                let s = (y * w + x) * 4;
                let d = (ny * nw + nx) * 4;
                px[d..d + 4].copy_from_slice(&self.px[s..s + 4]);
            }
        }
        Raster { w: nw as u32, h: nh as u32, px }
    }

    /// I pixel non premoltiplicati: (rgb, alfa) per i PDF.
    pub fn split(&self) -> (Vec<u8>, Vec<u8>) {
        let n = (self.w * self.h) as usize;
        let mut rgb = Vec::with_capacity(n * 3);
        let mut a = Vec::with_capacity(n);
        for p in self.px.chunks_exact(4) {
            let al = p[3];
            a.push(al);
            for c in &p[..3] {
                rgb.push(if al == 0 { 255 } else { ((*c as u32 * 255 + al as u32 / 2) / al as u32).min(255) as u8 });
            }
        }
        (rgb, a)
    }

    /// Quanti pixel hanno inchiostro (alfa > 0).
    pub fn inked(&self) -> usize {
        self.px.chunks_exact(4).filter(|p| p[3] > 8).count()
    }
}

// ---------------------------------------------------------------------------
// Rumore (grana)

fn hash(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8da6b343) ^ (y as u32).wrapping_mul(0xd8163841) ^ seed.wrapping_mul(0xcb1ab31f);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1e995);
    h ^= h >> 15;
    (h & 0xffffff) as f32 / 0xffffff as f32
}

fn vnoise(x: f32, y: f32, seed: u32) -> f32 {
    let (ix, iy) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - ix as f32, y - iy as f32);
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let a = hash(ix, iy, seed);
    let b = hash(ix + 1, iy, seed);
    let c = hash(ix, iy + 1, seed);
    let d = hash(ix + 1, iy + 1, seed);
    let top = a + (b - a) * sx;
    let bot = c + (d - c) * sx;
    top + (bot - top) * sy
}

fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Il timbro come immagine, a `px_per_cm` pixel per centimetro.
pub fn render(spec: &StampSpec, px_per_cm: f32) -> Raster {
    let f = fonts();
    let l = layout(spec, &f);
    let st = &spec.style;
    let (tw, th) = tilted(l.w, l.h, st.tilt);
    let ppc = px_per_cm.clamp(4.0, 400.0);
    let (w, h) = (((tw * ppc).ceil() as u32).max(1), ((th * ppc).ceil() as u32).max(1));
    let Some(mut pm) = Pixmap::new(w, h) else {
        return Raster { w: 1, h: 1, px: vec![0; 4] };
    };
    // cm del timbro dritto → pixel, ruotato attorno al centro
    let tr = Transform::from_translate(-l.w / 2.0, -l.h / 2.0)
        .post_rotate(st.tilt)
        .post_translate(tw / 2.0, th / 2.0)
        .post_scale(ppc, ppc);
    let mut paint = Paint::default();
    paint.set_color_rgba8(0, 0, 0, 255);
    paint.anti_alias = true;
    // testo
    let mut pb = PathBuilder::new();
    for (t, bold, em, x, base) in &l.lines {
        glyphs_path(&mut pb, if *bold { &f.bold } else { &f.text }, t, *em, *x, *base);
    }
    if let Some(p) = pb.finish() {
        pm.fill_path(&p, &paint, FillRule::Winding, tr, None);
    }
    // cornice
    let line = |w: f32| tiny_skia::Stroke { width: w, ..Default::default() };
    let i = 0.05;
    let mut frame = |pb: PathBuilder, width: f32| {
        if let Some(p) = pb.finish() {
            pm.stroke_path(&p, &paint, &line(width), tr, None);
        }
    };
    match st.frame {
        Frame::Plain => {}
        Frame::Rect => {
            let mut pb = PathBuilder::new();
            rounded_rect(&mut pb, i, i, l.w - i, l.h - i, 0.02);
            frame(pb, 0.06);
        }
        Frame::Rounded => {
            let mut pb = PathBuilder::new();
            rounded_rect(&mut pb, i, i, l.w - i, l.h - i, 0.3);
            frame(pb, 0.06);
        }
        Frame::Double => {
            let mut pb = PathBuilder::new();
            rounded_rect(&mut pb, i, i, l.w - i, l.h - i, 0.03);
            frame(pb, 0.075);
            let mut pb = PathBuilder::new();
            let j = i + 0.11;
            rounded_rect(&mut pb, j, j, l.w - j, l.h - j, 0.02);
            frame(pb, 0.03);
        }
        Frame::Oval => {
            let mut pb = PathBuilder::new();
            if let Some(r) = tiny_skia::Rect::from_ltrb(i, i, l.w - i, l.h - i) {
                pb.push_oval(r);
            }
            frame(pb, 0.065);
        }
    }
    // la maschera nitida → inchiostro vero
    let mask: Vec<u8> = pm.pixels().iter().map(|p| p.alpha()).collect();
    let sample = |x: f32, y: f32| -> f32 {
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let at = |xx: i64, yy: i64| -> f32 {
            if xx < 0 || yy < 0 || xx >= w as i64 || yy >= h as i64 {
                0.0
            } else {
                mask[(yy as u32 * w + xx as u32) as usize] as f32 / 255.0
            }
        };
        let (xi, yi) = (x0 as i64, y0 as i64);
        let t = at(xi, yi) + (at(xi + 1, yi) - at(xi, yi)) * fx;
        let b = at(xi, yi + 1) + (at(xi + 1, yi + 1) - at(xi, yi + 1)) * fx;
        t + (b - t) * fy
    };
    let grain = st.grain.clamp(0.0, 1.0);
    let uneven = st.uneven.clamp(0.0, 1.0);
    let opacity = st.opacity.clamp(0.1, 1.0);
    let seed = st.seed;
    let rough = 0.004 + grain * 0.014;
    let (dc, ds) = (st.uneven_dir.to_radians().cos(), st.uneven_dir.to_radians().sin());
    let half = (tw * tw + th * th).sqrt() / 2.0;
    let [r, g, b] = st.color.rgb();
    let mut px = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let (xc, yc) = ((x as f32 + 0.5) / ppc, (y as f32 + 0.5) / ppc);
            // bordo ruvido: si legge la maschera un po' spostati
            let dx = (vnoise(xc / 0.025, yc / 0.025, seed ^ 0x11) - 0.5) * 2.0 * rough;
            let dy = (vnoise(xc / 0.025, yc / 0.025, seed ^ 0x22) - 0.5) * 2.0 * rough;
            let cov = sample(x as f32 + dx * ppc, y as f32 + dy * ppc);
            if cov <= 0.003 {
                continue;
            }
            let fine = vnoise(xc / 0.011, yc / 0.011, seed ^ 0x33);
            let mid = vnoise(xc / 0.05, yc / 0.05, seed ^ 0x44);
            let big = vnoise(xc / 0.3, yc / 0.3, seed ^ 0x55);
            let n = 0.5 * fine + 0.3 * mid + 0.2 * big;
            // 0 = lato carico, 1 = lato scarico
            let side = (((xc - tw / 2.0) * dc + (yc - th / 2.0) * ds) / half * 0.5 + 0.5).clamp(0.0, 1.0);
            let thr = grain * 0.5 + uneven * 0.38 * side * side - 0.06;
            let keep = smooth(thr - 0.05, thr + 0.05, n);
            let density = 1.0 - grain * 0.3 * (1.0 - mid) - uneven * 0.45 * side;
            let a = (cov * keep * density.clamp(0.0, 1.0) * opacity).clamp(0.0, 1.0);
            if a <= 0.0 {
                continue;
            }
            let i = ((y * w + x) * 4) as usize;
            let a8 = (a * 255.0).round();
            px[i] = (r as f32 * a).round() as u8;
            px[i + 1] = (g as f32 * a).round() as u8;
            px[i + 2] = (b as f32 * a).round() as u8;
            px[i + 3] = a8 as u8;
        }
    }
    Raster { w, h, px }
}

/// L'immagine per un riquadro largo `px_w` pixel (come lo si vede), girata
/// di `quarter` quarti di giro come vuole `placement`.
pub fn render_fit(spec: &StampSpec, quarter: u8, px_w: f32) -> Raster {
    let (w, h) = size_cm(spec);
    let along = if quarter % 2 == 1 { h } else { w };
    render(spec, (px_w / along.max(0.1)).clamp(4.0, 160.0)).rotated(quarter)
}

// ---------------------------------------------------------------------------
// Sul foglio

/// I quattro angoli (in unità del pad) del timbro posato nel riquadro
/// `r` = (u, v, larghezza, altezza) della pagina vista.
pub fn corners(r: [f32; 4], g: &PageGeom) -> Vec<PenEvent> {
    let [u, v, w, h] = r;
    [(u, v), (u + w, v), (u + w, v + h), (u, v + h)]
        .into_iter()
        .map(|(a, b)| {
            let (x, y) = g.pad(a, b);
            PenEvent { x, y, pressure: 1.0, phase: PenPhase::Move }
        })
        .collect()
}

/// Dove sta un timbro sulla pagina vista con `map` (pad → u, v): il
/// riquadro (u0, v0, u1, v1) e di quanti quarti di giro (orari) va girata
/// l'immagine (la vista ruotata dopo averlo posato).
pub fn placement(points: &[PenEvent], map: impl Fn(f32, f32) -> (f32, f32)) -> Option<([f32; 4], u8)> {
    if points.len() < 4 {
        return None;
    }
    let q: Vec<(f32, f32)> = points.iter().take(4).map(|p| map(p.x, p.y)).collect();
    let (u0, v0) = q.iter().fold((f32::MAX, f32::MAX), |a, p| (a.0.min(p.0), a.1.min(p.1)));
    let (u1, v1) = q.iter().fold((f32::MIN, f32::MIN), |a, p| (a.0.max(p.0), a.1.max(p.1)));
    let (dx, dy) = (q[1].0 - q[0].0, q[1].1 - q[0].1);
    let quarter = if dx.abs() >= dy.abs() {
        if dx >= 0.0 { 0 } else { 2 }
    } else if dy >= 0.0 {
        1
    } else {
        3
    };
    Some(([u0, v0, u1, v1], quarter))
}

/// Come `placement`, con i punti già in frazioni della pagina vista.
pub fn placement_uv(pts: &[(f32, f32)]) -> Option<([f32; 4], u8)> {
    let ev: Vec<PenEvent> = pts.iter().map(|&(x, y)| PenEvent { x, y, pressure: 1.0, phase: PenPhase::Move }).collect();
    placement(&ev, |x, y| (x, y))
}

/// Il riquadro a grandezza naturale (u, v, larghezza, altezza) centrato in (cu, cv).
pub fn natural_rect(spec: &StampSpec, g: &PageGeom, cu: f32, cv: f32) -> [f32; 4] {
    let (w, h) = size_cm(spec);
    let (w, h) = (w * 1000.0 / g.w, h * 1000.0 / g.h);
    [cu - w / 2.0, cv - h / 2.0, w, h]
}

// ---------------------------------------------------------------------------
// Timbri salvati

pub fn load_all(path: &Path) -> Vec<SavedStamp> {
    std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

pub fn save_all(path: &Path, list: &[SavedStamp]) -> anyhow::Result<()> {
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(list)?)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Un seme nuovo per la grana.
pub fn new_seed(old: u32) -> u32 {
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(7);
    let s = (old ^ t).wrapping_mul(2654435761).wrapping_add(0x9e3779b9);
    if s == old { s.wrapping_add(1) } else { s }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::scene::View;

    pub(crate) fn cordua() -> StampSpec {
        StampSpec {
            company: Company {
                name: "CORDUA S.R.L.".into(),
                address: "Piazza della Vittoria, 15/D".into(),
                cap: "16121".into(),
                city: "Genova".into(),
                prov: "ge".into(),
                vat: "02597400999".into(),
                rea: "".into(),
                sdi: "usal8pv".into(),
                pec: "".into(),
            },
            style: StampStyle::default(),
        }
    }

    #[test]
    fn timbri_salvati_prima_della_pec() {
        let c: Company = serde_json::from_str(r#"{"name":"CORDUA S.R.L.","sdi":"USAL8PV"}"#).unwrap();
        assert_eq!(c.pec, "");
        assert_eq!(c.sdi, "USAL8PV");
    }

    #[test]
    fn righe_del_timbro() {
        let s = cordua();
        let l: Vec<String> = s.lines().into_iter().map(|l| l.0).collect();
        assert_eq!(
            l,
            ["CORDUA S.R.L.", "Piazza della Vittoria, 15/D", "16121 Genova (GE)", "P.IVA 02597400999", "Codice Univoco: USAL8PV"]
        );
        let mut s2 = s.clone();
        s2.company.address.clear();
        s2.company.rea = "GE-123456".into();
        s2.company.sdi = "  ".into();
        s2.company.pec = " cordua@pec.it ".into();
        let l: Vec<String> = s2.lines().into_iter().map(|l| l.0).collect();
        assert_eq!(l, ["CORDUA S.R.L.", "16121 Genova (GE)", "P.IVA 02597400999 – REA GE-123456", "PEC: cordua@pec.it"]);
    }

    #[test]
    fn grandezza_naturale_e_inclinazione() {
        let mut s = cordua();
        s.style.tilt = 0.0;
        let (w, h) = size_cm(&s);
        assert!((4.0..9.0).contains(&w), "larghezza {w}");
        assert!((1.5..4.0).contains(&h), "altezza {h}");
        s.style.tilt = 10.0;
        let (w2, h2) = size_cm(&s);
        assert!(w2 > w && h2 > h);
    }

    #[test]
    fn stessa_grana_con_lo_stesso_seme() {
        let s = cordua();
        let a = render(&s, 40.0);
        let b = render(&s, 40.0);
        assert_eq!(a.px, b.px);
        let mut s2 = s.clone();
        s2.style.seed = 99;
        assert_ne!(render(&s2, 40.0).px, a.px);
    }

    #[test]
    fn piu_grana_meno_inchiostro() {
        let mut s = cordua();
        s.style.grain = 0.0;
        s.style.uneven = 0.0;
        let pieno = render(&s, 60.0).inked();
        s.style.grain = 1.0;
        let sgranato = render(&s, 60.0).inked();
        assert!(pieno > 1000);
        assert!((sgranato as f32) < pieno as f32 * 0.8, "{sgranato} vs {pieno}");
    }

    #[test]
    fn colore_e_trasparenza() {
        let mut s = cordua();
        s.style.grain = 0.0;
        s.style.uneven = 0.0;
        s.style.opacity = 0.5;
        s.style.color = InkColor::Red;
        let r = render(&s, 50.0);
        let max_a = r.px.chunks_exact(4).map(|p| p[3]).max().unwrap();
        assert!((120..=130).contains(&max_a), "alfa {max_a}");
        let (rgb, _) = r.split();
        let i = r.px.chunks_exact(4).position(|p| p[3] == max_a).unwrap();
        assert!(rgb[i * 3] > 150 && rgb[i * 3 + 2] < 80);
    }

    #[test]
    fn angoli_e_posizione() {
        let view = View { rot: 1, ..Default::default() };
        let g = PageGeom::new(view, 29700.0, 21000.0);
        let r = [0.2, 0.3, 0.25, 0.1];
        let c = corners(r, &g);
        let (b, q) = placement(&c, |x, y| g.unit(x, y)).unwrap();
        assert_eq!(q, 0);
        for (a, e) in b.iter().zip([0.2, 0.3, 0.45, 0.4]) {
            assert!((a - e).abs() < 1e-4);
        }
        // la stessa pagina vista ruotata di un quarto: l'immagine gira con lei
        let g2 = PageGeom::new(View { rot: 2, ..Default::default() }, 29700.0, 21000.0);
        let (_, q2) = placement(&c, |x, y| g2.unit(x, y)).unwrap();
        assert_ne!(q2, 0);
    }

    #[test]
    fn ruota_di_un_quarto() {
        let r = Raster { w: 2, h: 1, px: vec![1, 1, 1, 255, 2, 2, 2, 255] };
        let q = r.rotated(1);
        assert_eq!((q.w, q.h), (1, 2));
        assert_eq!(q.px[0], 1);
        assert_eq!(q.px[4], 2);
        assert_eq!(r.rotated(4).px, r.px);
    }

    #[test]
    fn timbri_salvati() {
        let dir = std::env::temp_dir().join(format!("osl-timbri-{}", std::process::id()));
        let p = dir.join("timbri.json");
        let list = vec![SavedStamp { name: "Cordua".into(), spec: cordua() }];
        save_all(&p, &list).unwrap();
        assert_eq!(load_all(&p), list);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    #[ignore]
    fn timbro_png() {
        // STAMP_PNG=/tmp/osl/timbro cargo test --bin openslate timbro_png -- --ignored
        let out = std::env::var("STAMP_PNG").unwrap_or("/tmp/osl/timbro".into());
        let _ = std::fs::create_dir_all(std::path::Path::new(&out).parent().unwrap());
        for (i, f) in Frame::ALL.iter().enumerate() {
            let mut s = cordua();
            s.style.frame = *f;
            s.style.color = InkColor::ALL[i];
            s.style.grain = i as f32 * 0.22;
            s.style.uneven = 0.5;
            s.style.tilt = -3.0 + i as f32 * 2.0;
            std::fs::write(format!("{out}-{i}.png"), render(&s, 118.0).png()).unwrap();
        }
    }
}
