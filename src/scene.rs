//! Stato condiviso della lavagna: tratti, vista, penna, annulla/ripeti.
//!
//! Lo `Hub` è condiviso da tre parti: il decodificatore live (aggiunge punti),
//! l'interfaccia (disegna e modifica) e il server web (trasmette). Ogni
//! modifica viene annunciata ai client web come messaggio JSON.

use crate::settings::{color_to_hex, Settings};
use crate::stroke::{self, Drawing, PenEvent, PenPhase, Rgb, Stroke};
use std::fmt::Write as _;
use std::sync::{Arc, Mutex, MutexGuard};
use tokio::sync::broadcast;

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
pub struct PageStroke {
    pub color: Rgb,
    pub width: f32,
    pub pts: Vec<(f32, f32)>,
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
                .filter(|s| !s.points.is_empty())
                .map(|s| PageStroke {
                    color: s.color,
                    width: s.width,
                    pts: s.points.iter().map(|p| g.unit(p.x, p.y)).collect(),
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

/// Strumento attivo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Pen,
    Highlighter,
}

/// Tocco lungo: punta ferma almeno così a lungo…
const LONGPRESS: std::time::Duration = std::time::Duration::from_millis(800);
/// …muovendosi al massimo di questa frazione del lato lungo della pagina (~2 mm).
const LONGPRESS_MOVE: f32 = 0.007;

/// Penna per i nuovi tratti.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pen {
    pub color: Rgb,
    pub width: f32,
}

/// Operazioni annullabili.
enum Undo {
    /// È stato aggiunto l'ultimo tratto.
    Add,
    /// Tratti cancellati (indice originale, tratto), in ordine crescente.
    Remove(Vec<(usize, Stroke)>),
    /// Tutti i tratti sostituiti (Pulisci, caricamento pagina): i vecchi.
    Replace(Vec<Stroke>),
}

enum Redo {
    Add(Stroke),
    Remove(Vec<usize>),
    Replace(Vec<Stroke>),
}

const UNDO_MAX: usize = 300;

/// Cosa è cambiato dopo un evento penna (per i client web).
pub enum Change {
    None,
    Down { color: Rgb, width: f32, highlight: bool, u: f32, v: f32 },
    Move { u: f32, v: f32 },
    Up,
    /// Cambiamento non incrementale: i client ricaricano tutto.
    Reset,
}

pub struct Scene {
    pub drawing: Drawing,
    pub view: View,
    pub pen: Pen,
    /// Colore e spessore dell'evidenziatore.
    pub highlighter: Pen,
    pub tool: Tool,
    /// Zig-zag automatico (senza Gomma): spento di serie, perché le firme
    /// sono spesso veri zig-zag.
    pub zigzag: bool,
    /// Gomma: il tratto cancella ciò che attraversa (resta attiva finché la si spegne).
    pub eraser: bool,
    /// Tocco lungo = cambia strumento.
    pub longpress: bool,
    /// Tratto in corso: quando è iniziato, dove, quanto si è mosso.
    down_at: Option<std::time::Instant>,
    down_pos: (f32, f32),
    max_move: f32,
    /// Strumento appena cambiato col tocco lungo (per la barra di stato).
    pub tool_toggled: Option<Tool>,
    /// Pagina di documento sotto i tratti (modalità documento).
    pub bg: Option<PageBg>,
    /// Aumenta a ogni modifica (per il salvataggio automatico).
    pub rev: u64,
    /// Un tratto è in corso (penna appoggiata).
    active: bool,
    undo: Vec<Undo>,
    redo: Vec<Redo>,
    /// Quanti tratti ha cancellato l'ultimo zig-zag (per la barra di stato).
    pub last_erased: Option<usize>,
    /// Gomma col mouse in corso: tratti prima della passata e quanti tolti
    /// (una passata = una sola voce di annullamento).
    mouse_erase: Option<(Vec<Stroke>, usize)>,
    /// Evidenziatore col mouse in corso: indice del tratto e punto di
    /// partenza (u, v sulla pagina visualizzata).
    mouse_hl: Option<(usize, (f32, f32))>,
}

impl Scene {
    pub fn new(s: &Settings) -> Self {
        Self {
            drawing: Drawing::default(),
            view: View {
                rot: s.rot,
                flip_x: s.flip_x,
                flip_y: s.flip_y,
            },
            pen: Pen {
                color: s.pen_color,
                width: s.pen_width,
            },
            highlighter: Pen {
                color: s.highlight_color,
                width: s.highlight_width,
            },
            tool: Tool::Pen,
            zigzag: s.zigzag,
            eraser: false,
            longpress: s.longpress_toggle,
            down_at: None,
            down_pos: (0.0, 0.0),
            max_move: 0.0,
            tool_toggled: None,
            bg: None,
            rev: 0,
            active: false,
            undo: Vec::new(),
            redo: Vec::new(),
            last_erased: None,
            mouse_erase: None,
            mouse_hl: None,
        }
    }

    pub fn geom(&self) -> PageGeom {
        let (w, h) = page_dims(&self.drawing);
        PageGeom::new(self.view, w, h)
    }

    pub fn page(&self) -> Page {
        let mut p = Page::from_drawing(&self.drawing, self.view);
        p.bg = self.bg.clone();
        p
    }

    /// Mostra un'altra pagina (di documento): tratti e sfondo. Annulla/ripeti
    /// ripartono da zero.
    pub fn set_page(&mut self, strokes: Vec<Stroke>, bg: Option<PageBg>) {
        self.drawing.strokes = strokes;
        self.bg = bg;
        self.active = false;
        self.undo.clear();
        self.redo.clear();
        self.rev += 1;
    }

    /// Aggiunge più tratti in un colpo solo (es. la firma). Annullabile.
    pub fn add_strokes(&mut self, strokes: Vec<Stroke>) {
        if strokes.is_empty() {
            return;
        }
        self.active = false;
        let old = self.drawing.strokes.clone();
        self.drawing.strokes.extend(strokes);
        self.push_undo(Undo::Replace(old));
    }

    /// Indice del tratto in corso (penna appoggiata), se c'è.
    pub fn active_index(&self) -> Option<usize> {
        (self.active && !self.drawing.strokes.is_empty()).then(|| self.drawing.strokes.len() - 1)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    fn push_undo(&mut self, op: Undo) {
        self.undo.push(op);
        if self.undo.len() > UNDO_MAX {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// Aggiunge un evento penna.
    pub fn pen_event(&mut self, ev: PenEvent) -> Change {
        self.pen_event_at(ev, std::time::Instant::now())
    }

    pub fn toggle_tool(&mut self) {
        self.tool = match self.tool {
            Tool::Pen => Tool::Highlighter,
            Tool::Highlighter => Tool::Pen,
        };
    }

    /// Come `pen_event`, con l'istante dell'evento (per i test).
    pub fn pen_event_at(&mut self, ev: PenEvent, now: std::time::Instant) -> Change {
        let g = self.geom();
        match ev.phase {
            PenPhase::Down => self.start_stroke(ev, &g, now),
            PenPhase::Move => {
                if !self.active {
                    return self.start_stroke(ev, &g, now);
                }
                let d = (ev.x - self.down_pos.0).hypot(ev.y - self.down_pos.1);
                self.max_move = self.max_move.max(d);
                if let Some(s) = self.drawing.strokes.last_mut() {
                    s.points.push(ev);
                }
                let (u, v) = g.unit(ev.x, ev.y);
                Change::Move { u, v }
            }
            PenPhase::Up => {
                if !self.active {
                    return Change::None;
                }
                self.active = false;
                if let Some(s) = self.drawing.strokes.last_mut() {
                    s.points.push(ev);
                }
                let d = (ev.x - self.down_pos.0).hypot(ev.y - self.down_pos.1);
                self.max_move = self.max_move.max(d);
                if self.longpress && self.is_long_press(now) {
                    // Il tocco lungo non è un tratto: via il puntino, cambia strumento.
                    self.drawing.strokes.pop();
                    if matches!(self.undo.last(), Some(Undo::Add)) {
                        self.undo.pop();
                    }
                    self.toggle_tool();
                    self.tool_toggled = Some(self.tool);
                    return Change::Reset;
                }
                if self.eraser {
                    self.erase_with_last_stroke();
                    return Change::Reset;
                }
                if self.zigzag && self.try_erase_gesture() {
                    Change::Reset
                } else {
                    Change::Up
                }
            }
            PenPhase::Hover => Change::None,
        }
    }

    fn is_long_press(&self, now: std::time::Instant) -> bool {
        let (w, h) = page_dims(&self.drawing);
        self.down_at.is_some_and(|t| now.duration_since(t) >= LONGPRESS)
            && self.max_move <= LONGPRESS_MOVE * w.max(h)
    }

    fn start_stroke(&mut self, ev: PenEvent, g: &PageGeom, now: std::time::Instant) -> Change {
        let hl = self.tool == Tool::Highlighter && !self.eraser;
        // Con la gomma il tratto si vede rosso e sottile (poi sparisce).
        let pen = if self.eraser {
            Pen { color: [230, 70, 60], width: 1.6 }
        } else if hl {
            self.highlighter
        } else {
            self.pen
        };
        let mut s = if hl {
            Stroke::highlighter(pen.color, pen.width)
        } else {
            Stroke::new(pen.color, pen.width)
        };
        s.points.push(ev);
        self.drawing.strokes.push(s);
        self.active = true;
        self.down_at = Some(now);
        self.down_pos = (ev.x, ev.y);
        self.max_move = 0.0;
        self.push_undo(Undo::Add);
        let (u, v) = g.unit(ev.x, ev.y);
        Change::Down {
            color: pen.color,
            width: pen.width,
            highlight: hl,
            u,
            v,
        }
    }

    /// Gomma: l'ultimo tratto (qualsiasi forma) cancella ciò che attraversa e
    /// poi sparisce. Con l'evidenziatore attivo toglie solo le evidenziature.
    fn erase_with_last_stroke(&mut self) {
        let Some(last) = self.drawing.strokes.pop() else { return };
        if matches!(self.undo.last(), Some(Undo::Add)) {
            self.undo.pop();
        }
        let g = last.xy();
        let only_hl = self.tool == Tool::Highlighter;
        let n = self.remove_where(|s| (!only_hl || s.highlight) && stroke::erased_by(s, &g));
        self.last_erased = Some(n);
    }

    /// Se l'ultimo tratto è uno zig-zag, lo toglie e cancella ciò che copre.
    fn try_erase_gesture(&mut self) -> bool {
        let Some(last) = self.drawing.strokes.last() else {
            return false;
        };
        let g = last.xy();
        let (w, h) = page_dims(&self.drawing);
        if !stroke::is_zigzag(&g, 0.04 * w.max(h)) {
            return false;
        }
        // Il gesto non è un tratto: via lui e la sua voce di annullamento.
        self.drawing.strokes.pop();
        if matches!(self.undo.last(), Some(Undo::Add)) {
            self.undo.pop();
        }
        // Con l'evidenziatore attivo lo zig-zag toglie solo le evidenziature
        // (l'inchiostro resta); con la penna cancella tutto ciò che attraversa.
        let only_hl = self.tool == Tool::Highlighter;
        let n = self.remove_where(|s| (!only_hl || s.highlight) && stroke::erased_by(s, &g));
        self.last_erased = Some(n);
        tracing::info!("zig-zag: cancellati {n} tratti");
        true
    }

    /// Toglie (annullabile) i tratti che soddisfano `pred`. Ritorna quanti.
    fn remove_where(&mut self, pred: impl Fn(&Stroke) -> bool) -> usize {
        let mut removed = Vec::new();
        let mut i = 0;
        let mut orig = 0;
        while i < self.drawing.strokes.len() {
            if pred(&self.drawing.strokes[i]) {
                removed.push((orig, self.drawing.strokes.remove(i)));
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

    /// Evidenziatore col mouse (punti in u, v 0..1 sulla pagina visualizzata).
    /// Con `straight` (Shift) il tratto è una linea dritta, orizzontale o
    /// verticale secondo la direzione prevalente, dal punto di partenza.
    pub fn mouse_highlight(&mut self, phase: PenPhase, u: f32, v: f32, straight: bool) -> Change {
        let g = self.geom();
        let (u, v) = (u.clamp(0.0, 1.0), v.clamp(0.0, 1.0));
        let ev = |u: f32, v: f32, phase| {
            let (x, y) = g.pad(u, v);
            PenEvent { x, y, pressure: 0.5, phase }
        };
        match phase {
            PenPhase::Down => {
                if self.active || self.mouse_hl.is_some() {
                    return Change::None; // il pennino sta scrivendo
                }
                let pen = self.highlighter;
                let mut s = Stroke::highlighter(pen.color, pen.width);
                s.points.push(ev(u, v, PenPhase::Down));
                self.drawing.strokes.push(s);
                self.mouse_hl = Some((self.drawing.strokes.len() - 1, (u, v)));
                self.push_undo(Undo::Add);
                Change::Down { color: pen.color, width: pen.width, highlight: true, u, v }
            }
            PenPhase::Move => {
                let Some((i, (su, sv))) = self.mouse_hl else { return Change::None };
                let Some(s) = self.drawing.strokes.get_mut(i) else { return Change::None };
                if straight {
                    // direzione prevalente misurata sulla pagina vera (non 0..1)
                    let (u, v) = if (u - su).abs() * g.w >= (v - sv).abs() * g.h { (u, sv) } else { (su, v) };
                    let first = s.points[0];
                    s.points = vec![first, ev(u, v, PenPhase::Move)];
                    Change::Reset
                } else {
                    s.points.push(ev(u, v, PenPhase::Move));
                    Change::Move { u, v }
                }
            }
            PenPhase::Up => {
                if self.mouse_hl.take().is_none() {
                    return Change::None;
                }
                Change::Up
            }
            PenPhase::Hover => Change::None,
        }
    }

    pub fn mouse_highlighting(&self) -> bool {
        self.mouse_hl.is_some()
    }

    /// Gomma col mouse: inizia una passata.
    pub fn mouse_erase_begin(&mut self) {
        self.mouse_erase = Some((self.drawing.strokes.clone(), 0));
    }

    /// Gomma col mouse: toglie i tratti vicini al segmento a→b (punti in
    /// (u, v) 0..1 sulla pagina visualizzata). `radius` è in frazione
    /// dell'altezza della pagina. Con l'evidenziatore attivo toglie solo le
    /// evidenziature. Ritorna quanti ne ha tolti.
    pub fn mouse_erase_segment(&mut self, a: (f32, f32), b: (f32, f32), radius: f32) -> usize {
        if self.mouse_erase.is_none() {
            self.mouse_erase_begin();
        }
        let g = self.geom();
        let (pa, pb) = (g.pad(a.0, a.1), g.pad(b.0, b.1));
        let tol = radius * g.h;
        // spessore a schermo = width * altezza / 800 (vedi paint_page)
        let wscale = g.h / 800.0;
        let only_hl = self.tool == Tool::Highlighter;
        let before = self.drawing.strokes.len();
        self.drawing
            .strokes
            .retain(|s| !((!only_hl || s.highlight) && stroke::near_segment(s, pa, pb, tol, s.width * wscale / 2.0)));
        let n = before - self.drawing.strokes.len();
        if let Some(m) = &mut self.mouse_erase {
            m.1 += n;
        }
        n
    }

    /// Gomma col mouse: fine passata (registra l'annullamento). Ritorna
    /// quanti tratti sono stati tolti in tutto.
    pub fn mouse_erase_end(&mut self) -> usize {
        let Some((old, n)) = self.mouse_erase.take() else { return 0 };
        if n > 0 {
            self.active = false;
            self.push_undo(Undo::Replace(old));
        }
        self.last_erased = Some(n);
        n
    }

    /// Toglie tutte le evidenziature (l'inchiostro resta). Annullabile.
    pub fn remove_highlights(&mut self) -> usize {
        self.remove_where(|s| s.highlight)
    }

    /// Toglie un singolo tratto (per indice). Annullabile.
    pub fn remove_stroke(&mut self, index: usize) -> bool {
        if index >= self.drawing.strokes.len() {
            return false;
        }
        let s = self.drawing.strokes.remove(index);
        self.active = false;
        self.push_undo(Undo::Remove(vec![(index, s)]));
        true
    }

    pub fn clear(&mut self) {
        if self.drawing.strokes.is_empty() {
            return;
        }
        self.active = false;
        let old = std::mem::take(&mut self.drawing.strokes);
        self.push_undo(Undo::Replace(old));
    }

    /// Sostituisce i tratti (es. pagina scaricata dal pad), annullabile.
    pub fn load(&mut self, d: Drawing) {
        self.active = false;
        if d.width > 0.0 && d.height > 0.0 {
            self.drawing.width = d.width;
            self.drawing.height = d.height;
        }
        let old = std::mem::replace(&mut self.drawing.strokes, d.strokes);
        self.push_undo(Undo::Replace(old));
    }

    pub fn undo(&mut self) -> bool {
        self.active = false;
        let Some(op) = self.undo.pop() else {
            return false;
        };
        let strokes = &mut self.drawing.strokes;
        let redo = match op {
            Undo::Add => match strokes.pop() {
                Some(s) => Redo::Add(s),
                None => return false,
            },
            Undo::Remove(list) => {
                let idx = list.iter().map(|(i, _)| *i).collect();
                for (i, s) in list {
                    let i = i.min(strokes.len());
                    strokes.insert(i, s);
                }
                Redo::Remove(idx)
            }
            Undo::Replace(old) => Redo::Replace(std::mem::replace(strokes, old)),
        };
        self.redo.push(redo);
        true
    }

    pub fn redo(&mut self) -> bool {
        self.active = false;
        let Some(op) = self.redo.pop() else {
            return false;
        };
        let strokes = &mut self.drawing.strokes;
        let undo = match op {
            Redo::Add(s) => {
                strokes.push(s);
                Undo::Add
            }
            Redo::Remove(idx) => {
                let mut removed = Vec::new();
                for &i in idx.iter().rev() {
                    if i < strokes.len() {
                        removed.push((i, strokes.remove(i)));
                    }
                }
                removed.reverse();
                Undo::Remove(removed)
            }
            Redo::Replace(new) => Undo::Replace(std::mem::replace(strokes, new)),
        };
        // Non passa da push_undo: non deve svuotare le altre voci di "ripeti".
        self.undo.push(undo);
        true
    }

    /// Stato completo in JSON per i client web.
    pub fn snapshot_json(&self) -> String {
        let page = self.page();
        let mut s = String::with_capacity(64 + page.strokes.len() * 256);
        let _ = write!(s, "{{\"t\":\"snap\",\"aw\":{:.1},\"ah\":{:.1},\"o\":{},", page.w, page.h, self.active);
        if let Some(bg) = &self.bg {
            let r = bg.rect;
            let _ = write!(
                s,
                "\"bg\":{{\"k\":\"{}\",\"r\":[{:.5},{:.5},{:.5},{:.5}]}},",
                bg.key.replace(['"', '\\'], ""),
                r[0], r[1], r[2], r[3]
            );
        }
        s.push_str("\"s\":[");
        for (i, st) in page.strokes.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            let _ = write!(
                s,
                "{{\"c\":\"{}\",\"w\":{:.2},\"h\":{},\"p\":[",
                color_to_hex(st.color),
                st.width,
                st.highlight as u8
            );
            for (j, (u, v)) in st.pts.iter().enumerate() {
                if j > 0 {
                    s.push(',');
                }
                let _ = write!(s, "[{u:.4},{v:.4}]");
            }
            s.push_str("]}");
        }
        s.push_str("]}");
        s
    }
}

/// Messaggio per i client web.
#[derive(Clone, Debug)]
pub enum WebMsg {
    Json(Arc<str>),
    /// Inviare di nuovo lo stato completo.
    Reset,
}

pub struct Hub {
    scene: Mutex<Scene>,
    tx: broadcast::Sender<WebMsg>,
}

impl Hub {
    pub fn new(scene: Scene) -> Arc<Hub> {
        let (tx, _) = broadcast::channel(4096);
        Arc::new(Hub {
            scene: Mutex::new(scene),
            tx,
        })
    }

    pub fn lock(&self) -> MutexGuard<'_, Scene> {
        self.scene.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<WebMsg> {
        self.tx.subscribe()
    }

    pub fn pen_event(&self, ev: PenEvent) {
        let change = {
            let mut sc = self.lock();
            let ch = sc.pen_event(ev);
            if matches!(ch, Change::Up | Change::Reset) {
                sc.rev += 1;
            }
            ch
        };
        self.broadcast(change);
    }

    /// Evidenziatore col mouse (vedi `Scene::mouse_highlight`), anche in diretta web.
    pub fn mouse_highlight(&self, phase: PenPhase, u: f32, v: f32, straight: bool) {
        let change = {
            let mut sc = self.lock();
            let ch = sc.mouse_highlight(phase, u, v, straight);
            if matches!(ch, Change::Up | Change::Reset) {
                sc.rev += 1;
            }
            ch
        };
        self.broadcast(change);
    }

    fn broadcast(&self, change: Change) {
        let msg = match change {
            Change::None => return,
            Change::Down { color, width, highlight, u, v } => WebMsg::Json(
                format!(
                    "{{\"t\":\"down\",\"c\":\"{}\",\"w\":{width:.2},\"h\":{},\"u\":{u:.4},\"v\":{v:.4}}}",
                    color_to_hex(color),
                    highlight as u8
                )
                .into(),
            ),
            Change::Move { u, v } => {
                WebMsg::Json(format!("{{\"t\":\"move\",\"u\":{u:.4},\"v\":{v:.4}}}").into())
            }
            Change::Up => WebMsg::Json("{\"t\":\"up\"}".into()),
            Change::Reset => WebMsg::Reset,
        };
        let _ = self.tx.send(msg);
    }

    /// Modifica la scena e avvisa i client web di ricaricare.
    pub fn edit<R>(&self, f: impl FnOnce(&mut Scene) -> R) -> R {
        let r = {
            let mut sc = self.lock();
            let r = f(&mut sc);
            sc.rev += 1;
            r
        };
        let _ = self.tx.send(WebMsg::Reset);
        r
    }

    pub fn snapshot_json(&self) -> String {
        self.lock().snapshot_json()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(x: f32, y: f32, phase: PenPhase) -> PenEvent {
        PenEvent { x, y, pressure: 0.5, phase }
    }

    fn line(sc: &mut Scene, y: f32) {
        sc.pen_event(ev(1000.0, y, PenPhase::Down));
        sc.pen_event(ev(5000.0, y, PenPhase::Move));
        sc.pen_event(ev(5000.0, y, PenPhase::Up));
    }

    #[test]
    fn annulla_e_ripeti() {
        let mut sc = Scene::new(&Settings::default());
        line(&mut sc, 1000.0);
        line(&mut sc, 2000.0);
        assert_eq!(sc.drawing.strokes.len(), 2);
        sc.clear();
        assert!(sc.drawing.strokes.is_empty());
        assert!(sc.undo());
        assert_eq!(sc.drawing.strokes.len(), 2);
        assert!(sc.undo());
        assert_eq!(sc.drawing.strokes.len(), 1);
        assert!(sc.redo());
        assert!(sc.redo());
        assert!(sc.drawing.strokes.is_empty());
    }

    #[test]
    fn zigzag_cancella_e_si_annulla() {
        let mut sc = Scene::new(&Settings::default());
        sc.zigzag = true; // automatico (spento di serie)
        line(&mut sc, 1000.0); // verrà attraversata
        line(&mut sc, 15000.0); // lontana
        // zig-zag verticale sopra la prima linea
        for i in 0..=8 {
            let x = 2000.0 + i as f32 * 300.0;
            let y = if i % 2 == 0 { 0.0 } else { 2000.0 };
            let ph = if i == 0 { PenPhase::Down } else { PenPhase::Move };
            sc.pen_event(ev(x, y, ph));
        }
        let ch = sc.pen_event(ev(4400.0, 0.0, PenPhase::Up));
        assert!(matches!(ch, Change::Reset));
        assert_eq!(sc.drawing.strokes.len(), 1);
        assert_eq!(sc.last_erased, Some(1));
        assert!(sc.undo());
        assert_eq!(sc.drawing.strokes.len(), 2);
        // la linea è tornata al suo posto (prima)
        assert_eq!(sc.drawing.strokes[0].points[0].y, 1000.0);
    }

    #[test]
    fn firma_a_zigzag_non_si_cancella_e_la_gomma_si() {
        let mut sc = Scene::new(&Settings::default());
        line(&mut sc, 1000.0);
        let zz = |sc: &mut Scene| {
            for i in 0..=8 {
                let x = 2000.0 + i as f32 * 300.0;
                let y = if i % 2 == 0 { 0.0 } else { 2000.0 };
                sc.pen_event(ev(x, y, if i == 0 { PenPhase::Down } else { PenPhase::Move }));
            }
            sc.pen_event(ev(4400.0, 0.0, PenPhase::Up))
        };
        // senza gomma: lo zig-zag è una firma, resta e non cancella nulla
        assert!(matches!(zz(&mut sc), Change::Up));
        assert_eq!(sc.drawing.strokes.len(), 2);
        // con la gomma: anche una semplice passata cancella ciò che attraversa
        sc.eraser = true;
        sc.pen_event(ev(1500.0, 0.0, PenPhase::Down));
        sc.pen_event(ev(1500.0, 3000.0, PenPhase::Move));
        let ch = sc.pen_event(ev(1500.0, 3000.0, PenPhase::Up));
        assert!(matches!(ch, Change::Reset));
        assert_eq!(sc.drawing.strokes.len(), 1); // via la linea, resta la firma
        assert_eq!(sc.last_erased, Some(1));
        assert!(sc.undo());
        assert_eq!(sc.drawing.strokes.len(), 2);
    }

    #[test]
    fn tocco_lungo_cambia_strumento() {
        use std::time::{Duration, Instant};
        let mut sc = Scene::new(&Settings::default());
        let t0 = Instant::now();
        // tocco lungo: fermo per 1 s
        sc.pen_event_at(ev(5000.0, 5000.0, PenPhase::Down), t0);
        sc.pen_event_at(ev(5010.0, 5005.0, PenPhase::Move), t0 + Duration::from_millis(500));
        let ch = sc.pen_event_at(ev(5010.0, 5005.0, PenPhase::Up), t0 + Duration::from_millis(1000));
        assert!(matches!(ch, Change::Reset));
        assert_eq!(sc.tool, Tool::Highlighter);
        assert!(sc.drawing.strokes.is_empty());
        assert!(!sc.can_undo());
        // ora si evidenzia
        line(&mut sc, 3000.0);
        assert!(sc.drawing.strokes[0].highlight);
        assert_eq!(sc.drawing.strokes[0].color, Settings::default().highlight_color);
        // un tratto lento ma lungo NON cambia strumento
        sc.pen_event_at(ev(1000.0, 9000.0, PenPhase::Down), t0);
        sc.pen_event_at(ev(6000.0, 9000.0, PenPhase::Up), t0 + Duration::from_secs(2));
        assert_eq!(sc.tool, Tool::Highlighter);
        // nelle pagine gli evidenziatori vengono prima dell'inchiostro
        sc.tool = Tool::Pen;
        line(&mut sc, 12000.0);
        sc.drawing.strokes.rotate_left(1); // inchiostro prima nella lista
        let page = sc.page();
        assert!(page.strokes[0].highlight && !page.strokes.last().unwrap().highlight);
    }

    #[test]
    fn togliere_evidenziature() {
        let mut sc = Scene::new(&Settings::default());
        sc.zigzag = true;
        line(&mut sc, 1000.0); // inchiostro
        sc.tool = Tool::Highlighter;
        line(&mut sc, 1000.0); // evidenziatura sopra
        line(&mut sc, 5000.0);
        // zig-zag con l'evidenziatore attivo: toglie solo l'evidenziatura sotto
        for i in 0..=8 {
            let x = 2000.0 + i as f32 * 300.0;
            let y = if i % 2 == 0 { 0.0 } else { 2000.0 };
            sc.pen_event(ev(x, y, if i == 0 { PenPhase::Down } else { PenPhase::Move }));
        }
        sc.pen_event(ev(4400.0, 0.0, PenPhase::Up));
        assert_eq!(sc.drawing.strokes.len(), 2);
        assert!(!sc.drawing.strokes[0].highlight); // l'inchiostro è rimasto
        // tutte le evidenziature
        assert_eq!(sc.remove_highlights(), 1);
        assert!(sc.drawing.strokes.iter().all(|s| !s.highlight));
        assert!(sc.undo());
        assert_eq!(sc.drawing.strokes.len(), 2);
        // un tratto solo
        assert!(sc.remove_stroke(1));
        assert_eq!(sc.drawing.strokes.len(), 1);
        assert!(sc.undo());
        assert!(sc.drawing.strokes[1].highlight);
    }

    #[test]
    fn inverso_della_vista() {
        for rot in 0..4 {
            for (fx, fy) in [(false, false), (true, false), (false, true), (true, true)] {
                let g = PageGeom::new(View { rot, flip_x: fx, flip_y: fy }, 29700.0, 21000.0);
                let (u, v) = g.unit(1234.0, 5678.0);
                let (x, y) = g.pad(u, v);
                assert!((x - 1234.0).abs() < 0.5 && (y - 5678.0).abs() < 0.5, "rot {rot} {fx} {fy}");
            }
        }
    }

    #[test]
    fn evidenziatore_col_mouse() {
        let mut sc = Scene::new(&Settings::default());
        sc.drawing.width = 21000.0;
        sc.drawing.height = 29700.0;
        sc.view = View::default();
        // a mano libera: tutti i punti
        sc.mouse_highlight(PenPhase::Down, 0.1, 0.1, false);
        sc.mouse_highlight(PenPhase::Move, 0.2, 0.12, false);
        sc.mouse_highlight(PenPhase::Move, 0.3, 0.11, false);
        assert!(matches!(sc.mouse_highlight(PenPhase::Up, 0.3, 0.11, false), Change::Up));
        assert_eq!(sc.drawing.strokes[0].points.len(), 3);
        assert!(sc.drawing.strokes[0].highlight);
        // con Shift: linea dritta orizzontale (2 punti, stessa v)
        sc.mouse_highlight(PenPhase::Down, 0.1, 0.5, false);
        sc.mouse_highlight(PenPhase::Move, 0.3, 0.52, true);
        sc.mouse_highlight(PenPhase::Move, 0.6, 0.53, true);
        sc.mouse_highlight(PenPhase::Up, 0.6, 0.53, true);
        let g = sc.geom();
        let s = &sc.drawing.strokes[1];
        assert_eq!(s.points.len(), 2);
        let (_, v0) = g.unit(s.points[0].x, s.points[0].y);
        let (u1, v1) = g.unit(s.points[1].x, s.points[1].y);
        assert!((v0 - v1).abs() < 1e-4 && (u1 - 0.6).abs() < 1e-3);
        // verticale se il movimento è soprattutto in giù
        sc.mouse_highlight(PenPhase::Down, 0.5, 0.2, false);
        sc.mouse_highlight(PenPhase::Move, 0.52, 0.6, true);
        sc.mouse_highlight(PenPhase::Up, 0.52, 0.6, true);
        let s = &sc.drawing.strokes[2];
        let (ua, _) = g.unit(s.points[0].x, s.points[0].y);
        let (ub, _) = g.unit(s.points[1].x, s.points[1].y);
        assert!((ua - ub).abs() < 1e-4);
        // si annulla come un tratto qualsiasi
        assert!(sc.undo());
        assert_eq!(sc.drawing.strokes.len(), 2);
    }

    #[test]
    fn gomma_col_mouse() {
        let mut sc = Scene::new(&Settings::default());
        sc.drawing.width = 21000.0;
        sc.drawing.height = 29700.0;
        sc.view = View::default();
        let t0 = std::time::Instant::now();
        let line = |sc: &mut Scene, y: f32| {
            sc.pen_event_at(ev(1000.0, y, PenPhase::Down), t0);
            sc.pen_event_at(ev(15000.0, y, PenPhase::Move), t0);
            sc.pen_event_at(ev(15000.0, y, PenPhase::Up), t0);
        };
        line(&mut sc, 5000.0);
        line(&mut sc, 20000.0);
        let g = sc.geom();
        let (u, v) = g.unit(8000.0, 5000.0);
        let (u2, v2) = g.unit(8000.0, 20000.0);
        // passata che tocca solo la prima linea, poi un clic sulla seconda
        assert_eq!(sc.mouse_erase_segment((u, v - 0.05), (u, v + 0.05), 0.005), 1);
        assert_eq!(sc.mouse_erase_segment((u2, v2 - 0.02), (u2, v2 - 0.02), 0.005), 0);
        assert_eq!(sc.mouse_erase_segment((u2, v2), (u2, v2), 0.005), 1);
        assert_eq!(sc.mouse_erase_end(), 2);
        assert!(sc.drawing.strokes.is_empty());
        // un solo annulla rimette tutto
        assert!(sc.undo());
        assert_eq!(sc.drawing.strokes.len(), 2);
    }

    #[test]
    fn geometria_ruotata() {
        let g = PageGeom::new(View { rot: 1, ..Default::default() }, 300.0, 200.0);
        assert_eq!((g.w, g.h), (200.0, 300.0));
        let (u, v) = g.unit(0.0, 0.0);
        assert!((0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v));
    }
}
