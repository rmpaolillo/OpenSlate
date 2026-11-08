//! La rigatura del foglio: la geometria è `src/rigatura.rs` della desktop
//! (stesso file); qui si passa a Kotlin, che la disegna sotto i tratti.

use crate::rigatura::{layout, Ink, Mark, Paper};

#[derive(uniffi::Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaperInk {
    Fine,
    Line,
    Strong,
    Red,
    Axis,
}

/// Un pezzo della rigatura, in cm dall'angolo in alto a sinistra del foglio
/// come lo si vede (già ruotato).
#[derive(uniffi::Enum, Clone, Debug, PartialEq)]
pub enum PaperMark {
    Line { ink: PaperInk, x0: f32, y0: f32, x1: f32, y1: f32 },
    Dot { x: f32, y: f32 },
    /// Scritta centrata nel punto: è una chiave italiana della desktop
    /// ("Parole chiave", "x", "3"…), da tradurre se serve.
    Label { x: f32, y: f32, text: String },
}

/// Un tipo di foglio, per il menu.
#[derive(uniffi::Record, Clone, Debug)]
pub struct PaperKind {
    /// Codice di `settings.conf` della desktop ("quadretti5"…).
    pub code: String,
    /// Gruppo del menu (chiave della desktop: "Righe", "Quadretti"…; vuoto = bianco).
    pub group: String,
    /// Distanza fra le linee più vicine (cm): sotto pochi pixel si sfumano.
    pub pitch: f32,
    /// Tinta dei quadretti (più chiara delle righe).
    pub grid_like: bool,
    /// Ha i margini rossi (fogli a righe).
    pub ruled: bool,
}

#[uniffi::export]
pub fn paper_kinds() -> Vec<PaperKind> {
    Paper::ALL
        .into_iter()
        .map(|p| PaperKind {
            code: p.code().into(),
            group: Paper::GROUPS
                .iter()
                .find(|(_, g)| g.contains(&p))
                .map(|(t, _)| t.to_string())
                .unwrap_or_default(),
            pitch: p.pitch(),
            grid_like: p.grid_like(),
            ruled: p.ruled(),
        })
        .collect()
}

/// La rigatura di un foglio di `w_cm` × `h_cm`; `margin` = margini rossi sui fogli a righe.
#[uniffi::export]
pub fn paper_marks(code: String, w_cm: f32, h_cm: f32, margin: bool) -> Vec<PaperMark> {
    let ink = |i: Ink| match i {
        Ink::Fine => PaperInk::Fine,
        Ink::Line => PaperInk::Line,
        Ink::Strong => PaperInk::Strong,
        Ink::Red => PaperInk::Red,
        Ink::Axis => PaperInk::Axis,
    };
    layout(Paper::from_code(&code), w_cm, h_cm, margin)
        .into_iter()
        .map(|m| match m {
            Mark::Line(i, a, b) => PaperMark::Line { ink: ink(i), x0: a.0, y0: a.1, x1: b.0, y1: b.1 },
            Mark::Dot(p) => PaperMark::Dot { x: p.0, y: p.1 },
            Mark::Label(p, text) => PaperMark::Label { x: p.0, y: p.1, text },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fogli_della_desktop() {
        let k = paper_kinds();
        assert_eq!(k.len(), 16);
        assert_eq!(k[0].code, "bianco");
        assert!(paper_marks("bianco".into(), 21.0, 29.7, true).is_empty());
        let q = paper_marks("quadretti5".into(), 21.0, 29.7, true);
        assert_eq!(q.len(), 59 + 41);
    }
}
