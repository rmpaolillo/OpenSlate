//! La lavagna: i tratti, gli strumenti e annulla/ripeti. È la parte di
//! modifica di `src/scene.rs` della desktop (stesse regole: gomma, zig-zag,
//! evidenziatore sotto l'inchiostro, una voce di annullamento per passata),
//! senza impostazioni, diretta web e grafici. Gomma e zig-zag usano
//! `src/stroke.rs`, lo stesso file della desktop.
//!
//! Kotlin disegna da una sua copia dei tratti: `pen` restituisce i cambiamenti
//! (come `scene::Change` per i client web) e, dopo un `Reset`, si rilegge
//! tutto con `strokes`.

use crate::pages::PageStroke;
use crate::session::{PenPoint, Phase};
use crate::stroke::{self, PenEvent, PenPhase, Rgb, Stroke};
use std::sync::{Arc, Mutex};

const UNDO_MAX: usize = 300;
/// Il tratto della gomma si vede rosso e sottile (poi sparisce), come sulla desktop.
const ERASER_COLOR: Rgb = [230, 70, 60];
const ERASER_WIDTH: f32 = 1.6;

#[derive(uniffi::Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Pen,
    Highlighter,
}

/// Cosa è cambiato (come `scene::Change`).
#[derive(uniffi::Enum, Clone, Debug, PartialEq)]
pub enum DrawOp {
    /// Nuovo tratto.
    Down { color: String, width: f32, highlight: bool, point: PenPoint },
    /// Punto in più sul tratto in corso.
    Move { point: PenPoint },
    /// Fine del tratto in corso.
    Up { point: PenPoint },
    /// Cambiamento non incrementale: rileggere tutto con `strokes`.
    Reset,
}

enum Undo {
    Add,
    Remove(Vec<(usize, Stroke)>),
    Replace(Vec<Stroke>),
}

enum Redo {
    Add(Stroke),
    Remove(Vec<usize>),
    Replace(Vec<Stroke>),
}

struct Inner {
    strokes: Vec<Stroke>,
    pen: (Rgb, f32),
    highlighter: (Rgb, f32),
    tool: Tool,
    eraser: bool,
    zigzag: bool,
    /// Lato lungo del foglio in unità del pad (per lo zig-zag).
    page_long: f32,
    active: bool,
    undo: Vec<Undo>,
    redo: Vec<Redo>,
    /// Gomma col dito in corso: tratti prima della passata, quanti tolti.
    finger: Option<(Vec<Stroke>, usize)>,
    /// Aumenta a ogni modifica (per sapere se c'è da salvare).
    rev: u64,
}

pub fn hex_to_rgb(s: &str) -> Option<Rgb> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}

pub fn rgb_to_hex(c: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

fn to_event(p: &PenPoint) -> PenEvent {
    PenEvent {
        x: p.x,
        y: p.y,
        pressure: p.pressure,
        phase: match p.phase {
            Phase::Down => PenPhase::Down,
            Phase::Move => PenPhase::Move,
            Phase::Up => PenPhase::Up,
            Phase::Hover => PenPhase::Hover,
        },
    }
}

fn to_point(e: &PenEvent) -> PenPoint {
    PenPoint {
        x: e.x,
        y: e.y,
        pressure: e.pressure,
        phase: match e.phase {
            PenPhase::Down => Phase::Down,
            PenPhase::Move => Phase::Move,
            PenPhase::Up => Phase::Up,
            PenPhase::Hover => Phase::Hover,
        },
    }
}

pub fn to_page_stroke(s: &Stroke) -> PageStroke {
    PageStroke {
        color: rgb_to_hex(s.color),
        width: s.width,
        highlight: s.highlight,
        points: s.points.iter().map(to_point).collect(),
    }
}

pub fn from_page_stroke(s: &PageStroke) -> Stroke {
    let color = hex_to_rgb(&s.color).unwrap_or([20, 20, 20]);
    let mut st = if s.highlight { Stroke::highlighter(color, s.width) } else { Stroke::new(color, s.width) };
    st.points = s.points.iter().map(to_event).collect();
    st
}

impl Inner {
    fn push_undo(&mut self, op: Undo) {
        self.undo.push(op);
        if self.undo.len() > UNDO_MAX {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.rev += 1;
    }

    fn start(&mut self, ev: PenEvent, out: &mut Vec<DrawOp>) {
        let hl = self.tool == Tool::Highlighter && !self.eraser;
        let (color, width) = if self.eraser {
            (ERASER_COLOR, ERASER_WIDTH)
        } else if hl {
            self.highlighter
        } else {
            self.pen
        };
        let mut s = if hl { Stroke::highlighter(color, width) } else { Stroke::new(color, width) };
        s.points.push(ev);
        self.strokes.push(s);
        self.active = true;
        self.push_undo(Undo::Add);
        out.push(DrawOp::Down { color: rgb_to_hex(color), width, highlight: hl, point: to_point(&ev) });
    }

    fn event(&mut self, ev: PenEvent, out: &mut Vec<DrawOp>) {
        match ev.phase {
            PenPhase::Down => self.start(ev, out),
            PenPhase::Move => {
                if !self.active {
                    return self.start(ev, out);
                }
                if let Some(s) = self.strokes.last_mut() {
                    s.points.push(ev);
                }
                out.push(DrawOp::Move { point: to_point(&ev) });
            }
            PenPhase::Up => {
                if !self.active {
                    return;
                }
                self.active = false;
                if let Some(s) = self.strokes.last_mut() {
                    s.points.push(ev);
                }
                self.rev += 1;
                if self.eraser {
                    self.erase_with_last_stroke();
                    out.push(DrawOp::Reset);
                } else if self.zigzag && self.try_erase_gesture() {
                    out.push(DrawOp::Reset);
                } else {
                    out.push(DrawOp::Up { point: to_point(&ev) });
                }
            }
            PenPhase::Hover => {}
        }
    }

    /// Togli l'ultimo tratto, che era un gesto e non scrittura.
    fn pop_gesture(&mut self) -> Option<Stroke> {
        let last = self.strokes.pop()?;
        if matches!(self.undo.last(), Some(Undo::Add)) {
            self.undo.pop();
        }
        Some(last)
    }

    /// Gomma: l'ultimo tratto cancella ciò che attraversa e poi sparisce.
    /// Con l'evidenziatore attivo toglie solo le evidenziature.
    fn erase_with_last_stroke(&mut self) -> usize {
        let Some(last) = self.pop_gesture() else { return 0 };
        let g = last.xy();
        let only_hl = self.tool == Tool::Highlighter;
        self.remove_where(|s| (!only_hl || s.highlight) && stroke::erased_by(s, &g))
    }

    /// Zig-zag (se attivo): cancella ciò che copre.
    fn try_erase_gesture(&mut self) -> bool {
        let Some(last) = self.strokes.last() else { return false };
        let g = last.xy();
        if !stroke::is_zigzag(&g, 0.04 * self.page_long) {
            return false;
        }
        self.pop_gesture();
        let only_hl = self.tool == Tool::Highlighter;
        self.remove_where(|s| (!only_hl || s.highlight) && stroke::erased_by(s, &g));
        true
    }

    fn remove_where(&mut self, pred: impl Fn(&Stroke) -> bool) -> usize {
        let mut removed = Vec::new();
        let mut i = 0;
        let mut orig = 0;
        while i < self.strokes.len() {
            if pred(&self.strokes[i]) {
                removed.push((orig, self.strokes.remove(i)));
            } else {
                i += 1;
            }
            orig += 1;
        }
        let n = removed.len();
        if n > 0 {
            self.active = false;
            self.push_undo(Undo::Remove(removed));
        }
        n
    }

    fn replace(&mut self, strokes: Vec<Stroke>) {
        self.active = false;
        let old = std::mem::replace(&mut self.strokes, strokes);
        self.push_undo(Undo::Replace(old));
    }
}

/// Il modello della lavagna (uno per app).
#[derive(uniffi::Object)]
pub struct BoardModel {
    inner: Mutex<Inner>,
}

impl BoardModel {
    /// Per il tutor: i tratti, lo spessore della penna, il colore dell'evidenziatore.
    pub(crate) fn snapshot(&self) -> (Vec<Stroke>, f32, Rgb) {
        let s = self.inner.lock().unwrap();
        (s.strokes.clone(), s.pen.1, s.highlighter.0)
    }

    /// Per il tutor: tratti in più, con una sola voce di annullamento.
    pub(crate) fn add_strokes(&self, add: Vec<Stroke>) {
        if add.is_empty() {
            return;
        }
        let mut s = self.inner.lock().unwrap();
        let mut v = s.strokes.clone();
        v.extend(add);
        s.replace(v);
    }
}

#[uniffi::export]
impl BoardModel {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(Inner {
                strokes: Vec::new(),
                // come le impostazioni predefinite della desktop
                pen: ([20, 20, 20], 2.0),
                highlighter: ([255, 235, 90], 16.0),
                tool: Tool::Pen,
                eraser: false,
                zigzag: false,
                page_long: 21600.0,
                active: false,
                undo: Vec::new(),
                redo: Vec::new(),
                finger: None,
                rev: 0,
            }),
        })
    }

    /// Punti dal pad. Restituisce i cambiamenti da disegnare.
    pub fn pen(&self, points: Vec<PenPoint>) -> Vec<DrawOp> {
        let mut s = self.inner.lock().unwrap();
        let mut out = Vec::new();
        for p in &points {
            s.event(to_event(p), &mut out);
        }
        // dopo un Reset gli altri cambiamenti non servono
        if out.contains(&DrawOp::Reset) {
            return vec![DrawOp::Reset];
        }
        out
    }

    /// Tutti i tratti (dopo un `Reset`, o per salvare).
    pub fn strokes(&self) -> Vec<PageStroke> {
        self.inner.lock().unwrap().strokes.iter().map(to_page_stroke).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.lock().unwrap().strokes.iter().all(|s| s.points.is_empty())
    }

    pub fn rev(&self) -> u64 {
        self.inner.lock().unwrap().rev
    }

    pub fn set_pen(&self, color: String, width: f32) {
        let mut s = self.inner.lock().unwrap();
        s.pen = (hex_to_rgb(&color).unwrap_or(s.pen.0), width.clamp(0.4, 12.0));
    }

    pub fn set_highlighter(&self, color: String, width: f32) {
        let mut s = self.inner.lock().unwrap();
        s.highlighter = (hex_to_rgb(&color).unwrap_or(s.highlighter.0), width.clamp(2.0, 40.0));
    }

    pub fn set_tool(&self, tool: Tool) {
        self.inner.lock().unwrap().tool = tool;
    }

    pub fn set_eraser(&self, on: bool) {
        self.inner.lock().unwrap().eraser = on;
    }

    /// Zig-zag = cancella (spento di serie: le firme sono spesso zig-zag).
    pub fn set_zigzag(&self, on: bool) {
        self.inner.lock().unwrap().zigzag = on;
    }

    /// Dimensioni del foglio in unità del pad.
    pub fn set_page_size(&self, width: f32, height: f32) {
        self.inner.lock().unwrap().page_long = width.max(height).max(1.0);
    }

    pub fn can_undo(&self) -> bool {
        !self.inner.lock().unwrap().undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.inner.lock().unwrap().redo.is_empty()
    }

    pub fn undo(&self) -> bool {
        let mut s = self.inner.lock().unwrap();
        s.active = false;
        let Some(op) = s.undo.pop() else { return false };
        let redo = match op {
            Undo::Add => match s.strokes.pop() {
                Some(st) => Redo::Add(st),
                None => return false,
            },
            Undo::Remove(list) => {
                let idx = list.iter().map(|(i, _)| *i).collect();
                for (i, st) in list {
                    let i = i.min(s.strokes.len());
                    s.strokes.insert(i, st);
                }
                Redo::Remove(idx)
            }
            Undo::Replace(old) => Redo::Replace(std::mem::replace(&mut s.strokes, old)),
        };
        s.redo.push(redo);
        s.rev += 1;
        true
    }

    pub fn redo(&self) -> bool {
        let mut s = self.inner.lock().unwrap();
        s.active = false;
        let Some(op) = s.redo.pop() else { return false };
        let undo = match op {
            Redo::Add(st) => {
                s.strokes.push(st);
                Undo::Add
            }
            Redo::Remove(idx) => {
                let mut removed = Vec::new();
                for &i in idx.iter().rev() {
                    if i < s.strokes.len() {
                        removed.push((i, s.strokes.remove(i)));
                    }
                }
                removed.reverse();
                Undo::Remove(removed)
            }
            Redo::Replace(new) => Undo::Replace(std::mem::replace(&mut s.strokes, new)),
        };
        // non passa da push_undo: non deve svuotare le altre voci di "ripeti"
        s.undo.push(undo);
        s.rev += 1;
        true
    }

    /// Pulisce il foglio (annullabile).
    pub fn clear(&self) {
        let mut s = self.inner.lock().unwrap();
        if !s.strokes.is_empty() {
            s.replace(Vec::new());
        }
    }

    /// Mette sul foglio i tratti di una pagina (annullabile).
    pub fn load(&self, strokes: Vec<PageStroke>) {
        let v = strokes.iter().map(from_page_stroke).filter(|s| !s.points.is_empty()).collect();
        self.inner.lock().unwrap().replace(v);
    }

    /// Apre una pagina come foglio nuovo: i suoi tratti, senza cronologia
    /// (non c'è niente da annullare appena aperta).
    pub fn open(&self, strokes: Vec<PageStroke>) {
        let mut s = self.inner.lock().unwrap();
        s.strokes = strokes.iter().map(from_page_stroke).filter(|st| !st.points.is_empty()).collect();
        s.undo.clear();
        s.redo.clear();
        s.active = false;
        s.finger = None;
        s.rev += 1;
    }

    /// Foglio nuovo: niente tratti e niente cronologia.
    pub fn reset(&self) {
        let mut s = self.inner.lock().unwrap();
        s.strokes.clear();
        s.undo.clear();
        s.redo.clear();
        s.active = false;
        s.finger = None;
        s.rev += 1;
    }

    /// Gomma col dito: toglie i tratti vicini al segmento a→b (unità del
    /// pad). `tol` = raggio della gomma, `page_h` = altezza del foglio come lo
    /// si vede (per lo spessore dei tratti). Restituisce quanti ne ha tolti.
    pub fn finger_erase(&self, ax: f32, ay: f32, bx: f32, by: f32, tol: f32, page_h: f32) -> u32 {
        let mut s = self.inner.lock().unwrap();
        if s.finger.is_none() {
            let snapshot = s.strokes.clone();
            s.finger = Some((snapshot, 0));
        }
        let wscale = page_h / 800.0;
        let only_hl = s.tool == Tool::Highlighter;
        let before = s.strokes.len();
        s.strokes.retain(|st| {
            !((!only_hl || st.highlight) && stroke::near_segment(st, (ax, ay), (bx, by), tol, st.width * wscale / 2.0))
        });
        let n = before - s.strokes.len();
        if let Some(f) = &mut s.finger {
            f.1 += n;
        }
        if n > 0 {
            s.active = false;
            s.rev += 1;
        }
        n as u32
    }

    /// Fine della passata col dito: una sola voce di annullamento.
    pub fn finger_erase_end(&self) -> u32 {
        let mut s = self.inner.lock().unwrap();
        let Some((old, n)) = s.finger.take() else { return 0 };
        if n > 0 {
            s.push_undo(Undo::Replace(old));
        }
        n as u32
    }

    /// Toglie tutte le evidenziature (annullabile).
    pub fn remove_highlights(&self) -> u32 {
        self.inner.lock().unwrap().remove_where(|s| s.highlight) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: f32, y: f32, phase: Phase) -> PenPoint {
        PenPoint { x, y, pressure: if phase == Phase::Up { 0.0 } else { 0.5 }, phase }
    }

    fn line(b: &BoardModel, x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<DrawOp> {
        let n = 10;
        let mut v = vec![pt(x0, y0, Phase::Down)];
        for i in 1..n {
            let t = i as f32 / n as f32;
            v.push(pt(x0 + (x1 - x0) * t, y0 + (y1 - y0) * t, Phase::Move));
        }
        v.push(pt(x1, y1, Phase::Up));
        b.pen(v)
    }

    #[test]
    fn penna_evidenziatore_annulla_ripeti() {
        let b = BoardModel::new();
        b.set_pen("#1965c8".into(), 3.0);
        let ops = line(&b, 0.0, 0.0, 1000.0, 0.0);
        assert!(matches!(&ops[0], DrawOp::Down { color, width, highlight: false, .. } if color == "#1965c8" && *width == 3.0));
        assert!(matches!(ops.last(), Some(DrawOp::Up { .. })));
        b.set_tool(Tool::Highlighter);
        let ops = line(&b, 0.0, 100.0, 1000.0, 100.0);
        assert!(matches!(&ops[0], DrawOp::Down { highlight: true, width, .. } if *width == 16.0));
        assert_eq!(b.strokes().len(), 2);
        assert!(b.undo());
        assert_eq!(b.strokes().len(), 1);
        assert!(b.redo());
        assert_eq!(b.strokes().len(), 2);
        b.clear();
        assert!(b.is_empty());
        assert!(b.undo());
        assert_eq!(b.strokes().len(), 2);
        // una pagina aperta non ha niente da annullare
        let page = b.strokes();
        b.open(page);
        assert!(!b.can_undo() && !b.can_redo());
        assert_eq!(b.strokes().len(), 2);
    }

    #[test]
    fn gomma_della_penna() {
        let b = BoardModel::new();
        line(&b, 0.0, 500.0, 1000.0, 500.0); // orizzontale
        line(&b, 0.0, 2000.0, 1000.0, 2000.0); // lontana
        b.set_eraser(true);
        let ops = line(&b, 500.0, 0.0, 500.0, 1000.0); // la attraversa
        assert_eq!(ops.last(), Some(&DrawOp::Reset));
        assert_eq!(b.strokes().len(), 1); // tolta la prima, sparito il gesto
        assert!(b.undo());
        assert_eq!(b.strokes().len(), 2);
    }

    #[test]
    fn gomma_col_dito_una_voce_di_annullamento() {
        let b = BoardModel::new();
        line(&b, 0.0, 500.0, 1000.0, 500.0);
        line(&b, 0.0, 600.0, 1000.0, 600.0);
        assert_eq!(b.finger_erase(500.0, 400.0, 500.0, 520.0, 20.0, 21600.0), 1);
        assert_eq!(b.finger_erase(500.0, 520.0, 500.0, 700.0, 20.0, 21600.0), 1);
        assert_eq!(b.finger_erase_end(), 2);
        assert!(b.is_empty());
        assert!(b.undo());
        assert_eq!(b.strokes().len(), 2);
    }

    #[test]
    fn colori() {
        assert_eq!(hex_to_rgb("#ff8000"), Some([255, 128, 0]));
        assert_eq!(rgb_to_hex([20, 20, 20]), "#141414");
        assert_eq!(hex_to_rgb("boh"), None);
    }
}
