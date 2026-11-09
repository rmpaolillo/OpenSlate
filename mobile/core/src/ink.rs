//! Il tratto naturale per le app (`src/inkshape.rs` della desktop): lo
//! spessore segue la pressione e le punte si affusolano. Le app chiedono il
//! contorno di ogni tratto e lo riempiono, così schermo, PDF e immagini sono
//! uguali alla desktop.

use crate::session::PenPoint;

/// Accende o spegne il tratto naturale (di serie acceso). Spento: spessore
/// sempre uguale, come un pennarello.
#[uniffi::export]
pub fn set_natural_ink(on: bool) {
    crate::inkshape::set_natural(on);
}

#[uniffi::export]
pub fn natural_ink() -> bool {
    crate::inkshape::natural()
}

/// Il contorno del tratto naturale in unità del pad: coppie x, y da unire e
/// riempire. `width` è lo spessore pieno in unità del pad. Vuoto se il tratto
/// va disegnato come sempre (opzione spenta, evidenziatore, pressione costante).
#[uniffi::export]
pub fn ink_outline(points: Vec<PenPoint>, width: f32, highlight: bool) -> Vec<f32> {
    let pr = crate::inkshape::page_pressure(points.iter().map(|p| p.pressure), highlight);
    if pr.is_empty() {
        return Vec::new();
    }
    let pts: Vec<(f32, f32)> = points.iter().map(|p| (p.x, p.y)).collect();
    crate::inkshape::outline(&pts, &pr, width).into_iter().flat_map(|(x, y)| [x, y]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Phase;

    #[test]
    fn contorno_solo_con_pressione_vera() {
        let pt = |i: usize, p: f32| PenPoint { x: i as f32 * 50.0, y: 100.0, pressure: p, phase: Phase::Move };
        let costante: Vec<PenPoint> = (0..20).map(|i| pt(i, 0.5)).collect();
        assert!(ink_outline(costante, 30.0, false).is_empty());
        let vera: Vec<PenPoint> = (0..20).map(|i| pt(i, 0.3 + i as f32 * 0.03)).collect();
        let o = ink_outline(vera.clone(), 30.0, false);
        assert!(o.len() > 40 && o.len() % 2 == 0);
        assert!(ink_outline(vera, 30.0, true).is_empty());
    }
}
