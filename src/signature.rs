//! Firma autografa registrata una volta e riutilizzabile su qualsiasi documento.
//!
//! La firma si salva in `signatures/firma.json` già orientata come la si vede
//! (rotazione della vista applicata), con le misure reali in unità del pad:
//! inserita su una pagina ha la sua dimensione naturale, poi si sposta e si
//! ridimensiona col mouse.

use crate::library::{strokes_from_fmt, strokes_to_fmt, StrokeFmt};
use crate::scene::{PageGeom, View};
use crate::stroke::{PenEvent, Stroke};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

pub const SIG_FILE: &str = "signatures/firma.json";

#[derive(Clone, Debug)]
pub struct Signature {
    /// Larghezza e altezza (unità del pad, orientate come la vista).
    pub w: f32,
    pub h: f32,
    /// Punti relativi all'angolo in alto a sinistra della firma.
    pub strokes: Vec<Stroke>,
}

#[derive(Serialize, Deserialize)]
struct SigFmt {
    version: u32,
    w: f32,
    h: f32,
    strokes: Vec<StrokeFmt>,
}

impl Signature {
    /// Dai tratti scritti sul pad (solo inchiostro, niente evidenziatore).
    pub fn from_strokes(strokes: &[Stroke], view: View) -> Option<Signature> {
        let ink: Vec<&Stroke> = strokes.iter().filter(|s| !s.highlight && !s.points.is_empty()).collect();
        if ink.is_empty() {
            return None;
        }
        let pts: Vec<(f32, f32)> = ink
            .iter()
            .flat_map(|s| s.points.iter().map(|p| view.transform(p.x, p.y)))
            .collect();
        let (x0, y0, x1, y1) = crate::stroke::bbox(pts.into_iter());
        let out = ink
            .iter()
            .map(|s| {
                let mut st = (*s).clone();
                st.points = s
                    .points
                    .iter()
                    .map(|p| {
                        let (tx, ty) = view.transform(p.x, p.y);
                        PenEvent { x: tx - x0, y: ty - y0, ..*p }
                    })
                    .collect();
                st
            })
            .collect();
        Some(Signature {
            w: (x1 - x0).max(1.0),
            h: (y1 - y0).max(1.0),
            strokes: out,
        })
    }

    pub fn load() -> Option<Signature> {
        let f: SigFmt = serde_json::from_str(&std::fs::read_to_string(SIG_FILE).ok()?).ok()?;
        Some(Signature { w: f.w, h: f.h, strokes: strokes_from_fmt(f.strokes) })
    }

    pub fn save(&self) -> Result<()> {
        if self.strokes.is_empty() {
            bail!("firma vuota");
        }
        let _ = std::fs::create_dir_all("signatures");
        let f = SigFmt { version: 1, w: self.w, h: self.h, strokes: strokes_to_fmt(&self.strokes) };
        std::fs::write(SIG_FILE, serde_json::to_string(&f)?)?;
        Ok(())
    }

    /// Riquadro (u, v, larghezza, altezza in 0..1) a grandezza naturale,
    /// centrato in (cu, cv).
    pub fn natural_rect(&self, g: &PageGeom, cu: f32, cv: f32) -> [f32; 4] {
        let (w, h) = (self.w / g.w, self.h / g.h);
        [cu - w / 2.0, cv - h / 2.0, w, h]
    }

    /// I tratti della firma posata nel riquadro `r`, in coordinate del pad.
    pub fn place(&self, r: [f32; 4], g: &PageGeom) -> Vec<Stroke> {
        // ingrandimento rispetto alla grandezza naturale (per lo spessore)
        let scale = r[3] / (self.h / g.h);
        self.strokes
            .iter()
            .map(|s| {
                let mut st = s.clone();
                st.width = s.width * scale.clamp(0.3, 4.0);
                st.points = s
                    .points
                    .iter()
                    .map(|p| {
                        let u = r[0] + p.x / self.w * r[2];
                        let v = r[1] + p.y / self.h * r[3];
                        let (x, y) = g.pad(u, v);
                        PenEvent { x, y, ..*p }
                    })
                    .collect();
                st
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stroke::PenPhase;

    fn s(pts: &[(f32, f32)]) -> Stroke {
        let mut st = Stroke::new([20, 40, 160], 2.0);
        st.points = pts.iter().map(|&(x, y)| PenEvent { x, y, pressure: 0.5, phase: PenPhase::Move }).collect();
        st
    }

    #[test]
    fn registra_e_posa_la_firma() {
        let view = View { rot: 1, ..Default::default() };
        let g = PageGeom::new(view, 29700.0, 21000.0);
        let firma = Signature::from_strokes(&[s(&[(10000.0, 5000.0), (11000.0, 9000.0)])], view).unwrap();
        // rot 1: (x, y) → (y, -x): 4000 di larghezza, 1000 di altezza
        assert_eq!((firma.w, firma.h), (4000.0, 1000.0));
        let r = firma.natural_rect(&g, 0.5, 0.5);
        let placed = firma.place(r, &g);
        // a grandezza naturale la firma ha la stessa estensione sul pad
        let p = &placed[0].points;
        assert!(((p[1].x - p[0].x).abs() - 1000.0).abs() < 1.0);
        assert!(((p[1].y - p[0].y).abs() - 4000.0).abs() < 1.0);
        assert!((placed[0].width - 2.0).abs() < 1e-3);
        // al doppio: tratti più spessi
        let big = firma.place([r[0], r[1], r[2] * 2.0, r[3] * 2.0], &g);
        assert!((big[0].width - 4.0).abs() < 1e-3);
    }
}
