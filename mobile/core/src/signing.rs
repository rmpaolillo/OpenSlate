//! La firma registrata, come sulla desktop (`src/signature.rs`, stesso file
//! `signatures/firma.json`): si registra una volta scrivendo sul foglio, poi
//! si posa su qualsiasi documento, a grandezza naturale, e la si sposta o la
//! si ingrandisce col dito come il timbro. Si sincronizza col computer.

use crate::board::{from_page_stroke, BoardModel};
use crate::pages::{PageStroke, StoreError};
use crate::signature::Signature;
use crate::tutor::BoardView;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn file(data_dir: &str) -> PathBuf {
    Path::new(data_dir).join("signatures/firma.json")
}

fn load(data_dir: &str) -> Option<Signature> {
    Signature::load_from(&file(data_dir))
}

/// C'è una firma registrata?
#[uniffi::export]
pub fn sig_exists(data_dir: String) -> bool {
    load(&data_dir).is_some()
}

/// Registra la firma dai tratti scritti sul foglio (l'evidenziatore no).
#[uniffi::export]
pub fn sig_save(data_dir: String, strokes: Vec<PageStroke>, view: BoardView) -> Result<(), StoreError> {
    let st: Vec<_> = strokes.iter().filter(|s| s.stamp.is_none()).map(from_page_stroke).collect();
    let mut sig = Signature::from_strokes(&st, view.view()).ok_or(StoreError::Io { reason: "firma vuota: scrivi la firma sul foglio".into() })?;
    // registrandola di nuovo, colore e intensità restano quelli scelti
    if let Some(old) = load(&data_dir) {
        sig.intensity = old.intensity;
        sig.color = old.color;
    }
    sig.save_to(&file(&data_dir)).map_err(|e| StoreError::Io { reason: e.to_string() })
}

/// Un inchiostro per la firma (`signature::INKS`).
#[derive(uniffi::Record, Clone)]
pub struct SigInk {
    /// Nome in italiano (per l'accessibilità).
    pub name: String,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// Gli inchiostri per la firma: più tonalità di blu, nero, grigio, rosso…
#[uniffi::export]
pub fn sig_inks() -> Vec<SigInk> {
    crate::signature::INKS.iter().map(|(n, c)| SigInk { name: n.to_string(), r: c[0], g: c[1], b: c[2] }).collect()
}

/// Colore e intensità della firma, come per il timbro.
#[derive(uniffi::Record, Clone, PartialEq, Debug)]
pub struct SigLook {
    /// Quale di `sig_inks`; -1 = il colore con cui è stata scritta.
    pub ink: i32,
    /// 0.3..1.
    pub intensity: f32,
}

#[uniffi::export]
pub fn sig_look(data_dir: String) -> SigLook {
    let Some(sig) = load(&data_dir) else { return SigLook { ink: -1, intensity: 1.0 } };
    let ink = sig.color.and_then(|c| crate::signature::INKS.iter().position(|(_, k)| *k == c)).map(|i| i as i32).unwrap_or(-1);
    SigLook { ink, intensity: sig.intensity }
}

/// Cambia colore e intensità della firma registrata (restano per la prossima volta).
#[uniffi::export]
pub fn sig_set_look(data_dir: String, look: SigLook) -> Result<(), StoreError> {
    let mut sig = load(&data_dir).ok_or(StoreError::Io { reason: "nessuna firma registrata".into() })?;
    sig.color = usize::try_from(look.ink).ok().and_then(|i| crate::signature::INKS.get(i)).map(|(_, c)| *c);
    sig.intensity = look.intensity.clamp(crate::signature::MIN_INTENSITY, 1.0);
    sig.save_to(&file(&data_dir)).map_err(|e| StoreError::Io { reason: e.to_string() })
}

/// Il riquadro [u, v, larghezza, altezza] a grandezza naturale, centrato in
/// (cu, cv), mai più largo di metà foglio. Vuoto se non c'è la firma.
#[uniffi::export]
pub fn sig_natural_rect(data_dir: String, view: BoardView, cu: f32, cv: f32) -> Vec<f32> {
    let Some(sig) = load(&data_dir) else { return Vec::new() };
    let r = sig.natural_rect(&view.geom(), cu, cv);
    let k = if r[2] > 0.5 { 0.5 / r[2] } else { 1.0 };
    let (w, h) = (r[2] * k, r[3] * k);
    vec![cu - w / 2.0, cv - h / 2.0, w, h]
}

/// L'anteprima della firma (PNG trasparente largo circa `px_w`), per vederla
/// mentre la si posa; `page_px_h` è l'altezza del foglio in quei pixel (lo
/// spessore dei tratti è in pixel di un foglio alto 800).
#[uniffi::export]
pub fn sig_png(data_dir: String, px_w: f32, page_px_h: f32) -> Vec<u8> {
    use tiny_skia::{Color, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform};
    let Some(sig) = load(&data_dir) else { return Vec::new() };
    let k = px_w.clamp(16.0, 3000.0) / sig.w;
    let (w, h) = ((sig.w * k).ceil() as u32 + 8, (sig.h * k).ceil() as u32 + 8);
    let Some(mut px) = Pixmap::new(w.max(1), h.max(1)) else { return Vec::new() };
    // lo spessore come sul foglio: pixel di un foglio alto 800, in proporzione
    let line_k = page_px_h.max(1.0) / 800.0;
    for s in &sig.inked() {
        let mut pb = PathBuilder::new();
        for (i, p) in s.points.iter().enumerate() {
            let (x, y) = (p.x * k + 4.0, p.y * k + 4.0);
            if i == 0 {
                pb.move_to(x, y);
            } else {
                pb.line_to(x, y);
            }
        }
        if s.points.len() == 1 {
            let p = &s.points[0];
            pb.line_to(p.x * k + 4.5, p.y * k + 4.0);
        }
        let Some(path) = pb.finish() else { continue };
        let mut paint = Paint::default();
        paint.set_color(Color::from_rgba8(s.color[0], s.color[1], s.color[2], 255));
        paint.anti_alias = true;
        let stroke = Stroke { width: (s.width * line_k).max(1.0), line_cap: LineCap::Round, line_join: LineJoin::Round, ..Default::default() };
        px.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
    px.encode_png().unwrap_or_default()
}

/// Posa la firma nel riquadro [u, v, larghezza, altezza]: un passo di annulla.
#[uniffi::export]
pub fn board_add_signature(board: Arc<BoardModel>, view: BoardView, data_dir: String, rect: Vec<f32>) -> bool {
    let (Some(sig), [u, v, w, h, ..]) = (load(&data_dir), rect.as_slice()) else { return false };
    board.add_strokes(sig.place([*u, *v, *w, *h], &view.geom()));
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{PenPoint, Phase};

    #[test]
    fn registra_anteprima_e_posa() {
        let dir = std::env::temp_dir().join(format!("openslate-firma-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let d = dir.to_string_lossy().into_owned();
        let view = BoardView { quarter: 1, pad_w: 21600.0, pad_h: 14800.0 };
        assert!(!sig_exists(d.clone()) && sig_natural_rect(d.clone(), view, 0.5, 0.5).is_empty());
        assert!(sig_save(d.clone(), vec![], view).is_err());
        let firma = PageStroke {
            color: "#1428a0".into(),
            width: 2.0,
            highlight: false,
            points: vec![
                PenPoint { x: 5000.0, y: 5000.0, pressure: 0.5, phase: Phase::Down },
                PenPoint { x: 7000.0, y: 5600.0, pressure: 0.5, phase: Phase::Up },
            ],
            stamp: None,
        };
        sig_save(d.clone(), vec![firma], view).unwrap();
        assert!(sig_exists(d.clone()));
        let r = sig_natural_rect(d.clone(), view, 0.6, 0.8);
        assert_eq!(r.len(), 4);
        assert!(sig_png(d.clone(), 300.0, 1000.0).starts_with(b"\x89PNG"));
        let board = BoardModel::new();
        assert!(board_add_signature(board.clone(), view, d.clone(), r));
        assert_eq!(board.strokes().len(), 1);
        // colore e intensità: si salvano, si vedono nei tratti posati e restano registrandola di nuovo
        assert_eq!(sig_look(d.clone()), SigLook { ink: -1, intensity: 1.0 });
        let look = SigLook { ink: 9, intensity: 0.6 };
        sig_set_look(d.clone(), look.clone()).unwrap();
        assert_eq!(sig_look(d.clone()), look);
        assert!(board_add_signature(board.clone(), view, d.clone(), sig_natural_rect(d.clone(), view, 0.5, 0.5)));
        let red = crate::signature::INKS[9].1;
        assert_ne!(board.strokes()[1].color, format!("#{:02x}{:02x}{:02x}", red[0], red[1], red[2]), "più chiaro con intensità 0.6");
        sig_save(d.clone(), board.strokes()[..1].to_vec(), view).unwrap();
        assert_eq!(sig_look(d.clone()), look);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
