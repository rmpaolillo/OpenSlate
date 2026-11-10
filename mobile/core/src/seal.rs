//! Il timbro aziendale sul telefono (come la desktop, `src/stamp.rs`): i
//! timbri salvati, l'immagine da mostrare, dove posarlo e il tratto-timbro
//! sul foglio (un passo di annulla).

use crate::board::BoardModel;
use crate::pages::StoreError;
use crate::session::PenPoint;
use crate::stamp::{self, SavedStamp, StampSpec};
use crate::stroke::{PenEvent, PenPhase, Stroke};
use crate::tutor::BoardView;
use std::path::Path;
use std::sync::Arc;

const FILE: &str = "timbri.json";

fn err(e: impl std::fmt::Display) -> StoreError {
    StoreError::Io { reason: e.to_string() }
}

/// I timbri salvati nella cartella dei dati dell'app.
#[uniffi::export]
pub fn stamp_list(dir: String) -> Vec<SavedStamp> {
    stamp::load_all(&Path::new(&dir).join(FILE))
}

#[uniffi::export]
pub fn stamp_save(dir: String, list: Vec<SavedStamp>) -> Result<(), StoreError> {
    stamp::save_all(&Path::new(&dir).join(FILE), &list).map_err(err)
}

/// Un timbro nuovo, con le regolazioni di partenza.
#[uniffi::export]
pub fn stamp_new() -> SavedStamp {
    let mut spec = StampSpec::default();
    spec.style.seed = stamp::new_seed(1);
    SavedStamp { name: String::new(), spec }
}

/// Grana nuova (e il lato scarico da un'altra parte).
#[uniffi::export]
pub fn stamp_reseed(spec: StampSpec) -> StampSpec {
    let mut s = spec;
    s.style.seed = stamp::new_seed(s.style.seed);
    s.style.uneven_dir = (s.style.seed % 360) as f32;
    s
}

/// Le righe del timbro (per controllare che ci sia qualcosa).
#[uniffi::export]
pub fn stamp_lines(spec: StampSpec) -> Vec<String> {
    spec.lines().into_iter().map(|l| l.0).collect()
}

/// Grandezza del timbro vero, cm: [larghezza, altezza].
#[uniffi::export]
pub fn stamp_size_cm(spec: StampSpec) -> Vec<f32> {
    let (w, h) = stamp::size_cm(&spec);
    vec![w, h]
}

/// L'immagine PNG (trasparente) larga circa `px_w` pixel, girata di
/// `quarter` quarti di giro orari.
#[uniffi::export]
pub fn stamp_png(spec: StampSpec, px_w: f32, quarter: u8) -> Vec<u8> {
    stamp::render_fit(&spec, quarter, px_w.clamp(32.0, 3000.0)).png()
}

/// Il riquadro a grandezza naturale [u, v, larghezza, altezza] (frazioni del
/// foglio visto), centrato in (cu, cv), mai più largo di metà foglio.
#[uniffi::export]
pub fn stamp_natural_rect(spec: StampSpec, view: BoardView, cu: f32, cv: f32) -> Vec<f32> {
    let g = view.geom();
    let r = stamp::natural_rect(&spec, &g, cu, cv);
    let k = if r[2] > 0.5 { 0.5 / r[2] } else { 1.0 };
    let (w, h) = (r[2] * k, r[3] * k);
    vec![cu - w / 2.0, cv - h / 2.0, w, h]
}

/// Il riquadro dopo una regolazione (cornice, inclinazione…): stesso centro,
/// stessa grandezza rispetto al timbro vero.
#[uniffi::export]
pub fn stamp_refit(before: StampSpec, after: StampSpec, view: BoardView, rect: Vec<f32>) -> Vec<f32> {
    if rect.len() < 4 {
        return rect;
    }
    let g = view.geom();
    let a = stamp::natural_rect(&before, &g, 0.5, 0.5);
    let b = stamp::natural_rect(&after, &g, 0.5, 0.5);
    let k = rect[2] / a[2].max(1e-6);
    let (cx, cy) = (rect[0] + rect[2] / 2.0, rect[1] + rect[3] / 2.0);
    let (w, h) = (b[2] * k, b[3] * k);
    vec![cx - w / 2.0, cy - h / 2.0, w, h]
}

/// Dove disegnare un timbro posato.
#[derive(uniffi::Record, Clone, Debug, PartialEq)]
pub struct StampPlace {
    /// u0, v0, u1, v1 (frazioni del foglio visto).
    pub rect: Vec<f32>,
    /// Quarti di giro orari dell'immagine.
    pub quarter: u8,
}

#[uniffi::export]
pub fn stamp_place(points: Vec<PenPoint>, view: BoardView) -> Option<StampPlace> {
    let g = view.geom();
    let ev: Vec<PenEvent> = points.iter().map(|p| PenEvent { x: p.x, y: p.y, pressure: 1.0, phase: PenPhase::Move }).collect();
    let (r, quarter) = stamp::placement(&ev, |x, y| g.unit(x, y))?;
    Some(StampPlace { rect: r.to_vec(), quarter })
}

/// Posa il timbro nel riquadro [u, v, larghezza, altezza]: un passo di annulla.
#[uniffi::export]
pub fn board_add_stamp(board: Arc<BoardModel>, view: BoardView, spec: StampSpec, rect: Vec<f32>) {
    if rect.len() < 4 {
        return;
    }
    let g = view.geom();
    let corners = stamp::corners([rect[0], rect[1], rect[2], rect[3]], &g);
    board.add_strokes(vec![Stroke::stamp(spec, corners)]);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> StampSpec {
        let mut s = stamp_new().spec;
        s.style.seed = 7;
        s.company.name = "CORDUA S.R.L.".into();
        s.company.city = "Genova".into();
        s
    }

    #[test]
    fn posa_annulla_e_salva() {
        let b = BoardModel::new();
        let view = BoardView { quarter: 1, pad_w: 21600.0, pad_h: 14800.0 };
        let r = stamp_natural_rect(spec(), view, 0.5, 0.7);
        board_add_stamp(b.clone(), view, spec(), r.clone());
        let st = b.strokes();
        assert_eq!(st.len(), 1);
        assert!(st[0].stamp.is_some());
        let p = stamp_place(st[0].points.clone(), view).unwrap();
        assert_eq!(p.quarter, 0);
        assert!((p.rect[0] - r[0]).abs() < 1e-3 && (p.rect[3] - (r[1] + r[3])).abs() < 1e-3);
        assert!(b.undo());
        assert!(b.strokes().is_empty());
        assert!(b.redo());
        // nella pagina salvata il timbro resta
        let dir = std::env::temp_dir().join(format!("osl-seal-{}", std::process::id()));
        let d = dir.to_string_lossy().to_string();
        let page = crate::pages::Page {
            id: String::new(),
            title: String::new(),
            created: "2026-11-10 10:00:00".into(),
            source: "documento".into(),
            raw: None,
            width: 21600.0,
            height: 14800.0,
            strokes: b.strokes(),
        };
        let id = crate::pages::library_save(d.clone(), page).unwrap();
        let back = crate::pages::library_load(d.clone(), id).unwrap();
        assert_eq!(back.strokes[0].stamp, Some(spec()));
        // i timbri salvati
        stamp_save(d.clone(), vec![SavedStamp { name: "Cordua".into(), spec: spec() }]).unwrap();
        assert_eq!(stamp_list(d.clone())[0].name, "Cordua");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn regolazione_tiene_centro_e_grandezza() {
        let view = BoardView { quarter: 0, pad_w: 21600.0, pad_h: 14800.0 };
        let a = spec();
        let mut b = a.clone();
        b.style.tilt = 12.0;
        let r = stamp_natural_rect(a.clone(), view, 0.4, 0.6);
        let r2 = stamp_refit(a, b, view, r.clone());
        assert!((r2[0] + r2[2] / 2.0 - 0.4).abs() < 1e-4);
        assert!(r2[2] > r[2]);
        assert!(!stamp_png(spec(), 300.0, 1).is_empty());
    }
}
