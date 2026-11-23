//! Firma autografa registrata una volta e riutilizzabile su qualsiasi documento.
//!
//! La firma si salva in `signatures/firma.json` già orientata come la si vede
//! (rotazione della vista applicata), con le misure reali in unità del pad:
//! inserita su una pagina ha la sua dimensione naturale, poi si sposta e si
//! ridimensiona col mouse.

use crate::docstore::{ink_to_strokes as strokes_from_fmt, strokes_to_ink as strokes_to_fmt, InkFmt as StrokeFmt};
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
    /// Intensità dell'inchiostro, 0.3..1 (come per il timbro): più bassa,
    /// tratto più chiaro e un po' più sottile, come una penna che scarica.
    pub intensity: f32,
    /// Il colore scelto (`INKS`); None = quello con cui è stata scritta.
    pub color: Option<crate::stroke::Rgb>,
}

/// Gli inchiostri per la firma: nome (italiano, si traduce dove serve) e colore.
/// Più tonalità per colore, come le penne vere.
pub const INKS: &[(&str, crate::stroke::Rgb)] = &[
    ("Blu notte", [22, 32, 92]),
    ("Blu", [28, 58, 160]),
    ("Blu biro", [38, 84, 205]),
    ("Azzurro", [62, 132, 212]),
    ("Nero", [16, 16, 20]),
    ("Grafite", [52, 54, 60]),
    ("Grigio scuro", [88, 90, 96]),
    ("Grigio", [126, 128, 134]),
    ("Bordeaux", [118, 22, 38]),
    ("Rosso", [192, 32, 42]),
    ("Verde scuro", [24, 92, 52]),
    ("Verde", [40, 138, 72]),
    ("Viola", [98, 52, 150]),
    ("Seppia", [112, 72, 40]),
];

/// Intensità minima della firma.
pub const MIN_INTENSITY: f32 = 0.3;

fn full() -> f32 {
    1.0
}

#[derive(Serialize, Deserialize)]
struct SigFmt {
    version: u32,
    w: f32,
    h: f32,
    strokes: Vec<StrokeFmt>,
    #[serde(default = "full")]
    intensity: f32,
    #[serde(default)]
    color: Option<crate::stroke::Rgb>,
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
            intensity: 1.0,
            color: None,
        })
    }

    pub fn load() -> Option<Signature> {
        Self::load_from(std::path::Path::new(SIG_FILE))
    }

    pub fn save(&self) -> Result<()> {
        self.save_to(std::path::Path::new(SIG_FILE))
    }

    pub fn load_from(file: &std::path::Path) -> Option<Signature> {
        let f: SigFmt = serde_json::from_str(&std::fs::read_to_string(file).ok()?).ok()?;
        Some(Signature { w: f.w, h: f.h, strokes: strokes_from_fmt(f.strokes), intensity: f.intensity.clamp(MIN_INTENSITY, 1.0), color: f.color })
    }

    /// Scrive la firma (passando da un file temporaneo: la sincronizzazione
    /// col telefono non trova mai un file a metà).
    pub fn save_to(&self, file: &std::path::Path) -> Result<()> {
        if self.strokes.is_empty() {
            bail!("firma vuota");
        }
        if let Some(d) = file.parent() {
            std::fs::create_dir_all(d)?;
        }
        let f = SigFmt { version: 1, w: self.w, h: self.h, strokes: strokes_to_fmt(&self.strokes), intensity: self.intensity, color: self.color };
        let tmp = file.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string(&f)?)?;
        std::fs::rename(&tmp, file)?;
        Ok(())
    }

    /// Riquadro (u, v, larghezza, altezza in 0..1) a grandezza naturale,
    /// centrato in (cu, cv).
    pub fn natural_rect(&self, g: &PageGeom, cu: f32, cv: f32) -> [f32; 4] {
        let (w, h) = (self.w / g.w, self.h / g.h);
        [cu - w / 2.0, cv - h / 2.0, w, h]
    }

    /// I tratti col colore scelto e l'intensità applicata: colore verso il bianco della
    /// carta e spessore un po' più sottile. Così vale ovunque (lavagna, PDF,
    /// stampa, telefono) senza bisogno di trasparenza nei tratti.
    pub fn inked(&self) -> Vec<Stroke> {
        let k = 1.0 - self.intensity.clamp(MIN_INTENSITY, 1.0);
        self.strokes
            .iter()
            .map(|s| {
                let mut st = s.clone();
                st.color = self.color.unwrap_or(s.color).map(|c| (c as f32 + (255.0 - c as f32) * k * 0.85).round() as u8);
                st.width = s.width * (1.0 - k * 0.5);
                st
            })
            .collect()
    }

    /// I tratti della firma posata nel riquadro `r`, in coordinate del pad.
    pub fn place(&self, r: [f32; 4], g: &PageGeom) -> Vec<Stroke> {
        // ingrandimento rispetto alla grandezza naturale (per lo spessore)
        let scale = r[3] / (self.h / g.h);
        self.inked()
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

    #[test]
    fn intensita_schiarisce_e_resta_salvata() {
        let mut sig = Signature::from_strokes(&[s(&[(0.0, 0.0), (100.0, 50.0)])], View::default()).unwrap();
        assert_eq!(sig.inked()[0].color, sig.strokes[0].color);
        sig.intensity = 0.5;
        let light = &sig.inked()[0];
        assert!(light.color.iter().zip(sig.strokes[0].color).all(|(a, b)| *a > b));
        assert!(light.width < sig.strokes[0].width);
        let f = std::env::temp_dir().join(format!("openslate-firma-int-{}.json", std::process::id()));
        sig.save_to(&f).unwrap();
        assert_eq!(Signature::load_from(&f).unwrap().intensity, 0.5);
        // il colore scelto vale per tutti i tratti e resta salvato
        sig.intensity = 1.0;
        sig.color = Some(INKS[9].1);
        assert_eq!(sig.inked()[0].color, INKS[9].1);
        sig.save_to(&f).unwrap();
        assert_eq!(Signature::load_from(&f).unwrap().color, Some(INKS[9].1));
        let _ = std::fs::remove_file(&f);
    }
}
