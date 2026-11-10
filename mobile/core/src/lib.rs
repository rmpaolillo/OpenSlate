//! OpenSlate per telefoni e tablet (Android e iOS): il "cuore" in Rust.
//!
//! Il Bluetooth lo fa l'app (Kotlin `BluetoothGatt`, Swift CoreBluetooth): questo crate non apre
//! connessioni, riceve i byte delle notifiche e dice quali byte scrivere
//! (stile "sans-IO"). Protocollo, decodifica live e modello dei tratti sono gli
//! STESSI file della desktop (`src/protocol.rs`, `src/live.rs`,
//! `src/offline.rs`, `src/rigatura.rs`, `src/stroke.rs`), inclusi così come sono: una correzione lì vale per
//! entrambe le app.

#![allow(dead_code)]

#[path = "../../../src/aisync.rs"]
pub mod aisync;
#[path = "../../../src/craftexport.rs"]
pub mod craftexport;
#[path = "../../../src/mdconv.rs"]
pub mod mdconv;
#[path = "../../../src/pageanim.rs"]
pub mod pageanim;
#[path = "../../../src/boardmarks.rs"]
pub mod boardmarks;
#[path = "../../../src/boardread.rs"]
pub mod boardread;
#[path = "../../../src/handwrite.rs"]
pub mod handwrite;
#[path = "../../../src/pagegeom.rs"]
pub mod pagegeom;
#[path = "../../../src/inkshape.rs"]
pub mod inkshape;
#[path = "../../../src/rasign.rs"]
pub mod rasign;
#[path = "../../../src/vecsign.rs"]
pub mod vecsign;
#[path = "../../../src/wordsign.rs"]
pub mod wordsign;
#[path = "../../../src/pdfdoc.rs"]
pub mod pdfdoc;
#[path = "../../../src/pagerender.rs"]
pub mod pagerender;
#[path = "../../../src/offline.rs"]
pub mod offline;
#[path = "../../../src/live.rs"]
pub mod live;
#[path = "../../../src/protocol.rs"]
pub mod protocol;
#[path = "../../../src/rigatura.rs"]
pub mod rigatura;
#[path = "../../../src/righe.rs"]
pub mod righe;
#[path = "../../../src/inbella.rs"]
pub mod inbella;
#[path = "../../../src/mathhand.rs"]
pub mod mathhand;
#[path = "../../../src/shapes.rs"]
pub mod shapes;
#[path = "../../../src/stroke.rs"]
pub mod stroke;

/// Come `scene` ed `export` della desktop, per i file inclusi da src/.
pub mod scene {
    pub use crate::pagegeom::*;
}
pub mod export {
    pub use crate::pagerender::*;
}
/// Al posto di `mdcraft` della desktop (Markdown con formule): sul telefono i
/// Markdown non si aprono come documenti, quindi niente da fare.
pub mod mdcraft {
    use std::path::Path;
    use wordcraft_engine::Session;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Hand {
        Stampato,
    }
    pub struct Piece;
    pub fn is_markdown(_: &Path) -> bool {
        false
    }
    pub fn split(md: &str, _: &Path) -> (String, Vec<Piece>) {
        (md.to_string(), Vec::new())
    }
    pub fn style_with(_: &mut Session, _: Hand) {}
    pub fn place(_: &mut Session, _: &[Piece]) {}
}

/// Al posto di `campioni` della desktop: sul telefono la bella copia si fa
/// sempre col carattere chiaro (la scrittura addestrata sta sul computer).
pub mod campioni {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Style {
        Corsivo,
        Stampatello,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Mode {
        Chiaro,
        Scrittura(Style),
        Mano,
    }
    pub fn mode() -> Mode {
        Mode::Chiaro
    }
    pub fn glyph(_: char) -> Option<(f32, Vec<Vec<(f32, f32)>>)> {
        None
    }
    pub fn text_box(_: &str) -> Option<(f32, Vec<Vec<(f32, f32)>>)> {
        None
    }
}
/// Al posto di `mano` della desktop (la rete della scrittura a mano: non sul telefono).
pub mod mano {
    pub const XH: f32 = 14.0;
    pub struct WordInk {
        pub strokes: Vec<Vec<(f32, f32)>>,
        pub xh: f32,
    }
    pub fn word(_: &str) -> Option<WordInk> {
        None
    }
}
/// Al posto di `boardagent` della desktop: sul telefono i segni dell'AI non
/// si distinguono dal resto.
pub mod boardagent {
    pub fn is_ai_stroke(_: &crate::stroke::Stroke) -> bool {
        false
    }
}
pub mod web {
    /// Scrittura sicura: file temporaneo e poi rinomina.
    pub fn write_atomic(p: &std::path::Path, data: &[u8]) -> std::io::Result<()> {
        let tmp = p.with_extension("tmp");
        std::fs::write(&tmp, data)?;
        std::fs::rename(&tmp, p)
    }
}

mod ai;
mod board;
mod docs;
mod formats;
mod ink;
mod neat;
mod pages;
mod paper;
mod session;
mod tutor;

pub use ai::*;
pub use board::*;
pub use formats::*;
pub use ink::*;
pub use neat::*;
pub use docs::*;
pub use pages::*;
pub use paper::*;
pub use session::*;
pub use tutor::*;

uniffi::setup_scaffolding!();
