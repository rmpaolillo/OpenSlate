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
#[path = "../../../src/boardmarks.rs"]
pub mod boardmarks;
#[path = "../../../src/boardread.rs"]
pub mod boardread;
#[path = "../../../src/handwrite.rs"]
pub mod handwrite;
#[path = "../../../src/pagegeom.rs"]
pub mod pagegeom;
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

mod ai;
mod board;
mod docs;
mod pages;
mod paper;
mod session;
mod tutor;

pub use ai::*;
pub use board::*;
pub use docs::*;
pub use pages::*;
pub use paper::*;
pub use session::*;
pub use tutor::*;

uniffi::setup_scaffolding!();
