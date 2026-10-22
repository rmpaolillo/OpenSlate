//! I caratteri dentro il programma, una copia sola ciascuno: un
//! `include_bytes!` scritto in due posti mette nel programma due copie dello
//! stesso file.
//!
//! - Inter (SIL OFL 1.1, assets/fonts/OFL-Inter.txt): il nostro ritaglio senza
//!   l'area privata dell'Unicode (lì ci sono le icone Phosphor), per
//!   l'interfaccia e per le pagine web dell'app.
//! - Source Serif 4 (SIL OFL 1.1, assets/fonts/OFL-SourceSerif4.txt): è già
//!   dentro WordCraft (`wordcraft_fonts::bundled`), gli stessi file: si usa
//!   quella copia.

use std::collections::HashMap;
use std::sync::OnceLock;

pub static INTER_REGULAR: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
pub static INTER_MEDIUM: &[u8] = include_bytes!("../assets/fonts/Inter-Medium.ttf");
pub static INTER_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Inter-SemiBold.ttf");

/// Un carattere di WordCraft dal suo nome PostScript (es. `SourceSerif4-Bold`).
pub fn bundled(ps: &str) -> Option<&'static [u8]> {
    static BY_NAME: OnceLock<HashMap<String, &'static [u8]>> = OnceLock::new();
    BY_NAME.get_or_init(|| wordcraft_fonts::bundled().iter().filter_map(|b| ps_name(b).map(|n| (n, *b))).collect()).get(ps).copied()
}

/// Il nome PostScript (name ID 6) dalla tabella `name`.
fn ps_name(font: &[u8]) -> Option<String> {
    let u16_at = |o: usize| font.get(o..o + 2).map(|b| u16::from_be_bytes([b[0], b[1]]) as usize);
    let u32_at = |o: usize| font.get(o..o + 4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize);
    let tables = u16_at(4)?;
    let name = (0..tables).map(|i| 12 + 16 * i).find(|&r| font.get(r..r + 4) == Some(&b"name"[..]))?;
    let base = u32_at(name + 8)?;
    let (count, strings) = (u16_at(base + 2)?, base + u16_at(base + 4)?);
    for i in 0..count {
        let r = base + 6 + 12 * i;
        let (platform, id, len, off) = (u16_at(r)?, u16_at(r + 6)?, u16_at(r + 8)?, u16_at(r + 10)?);
        if id != 6 {
            continue;
        }
        let raw = font.get(strings + off..strings + off + len)?;
        return Some(if platform == 1 {
            raw.iter().map(|&c| c as char).collect()
        } else {
            char::decode_utf16(raw.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]]))).filter_map(Result::ok).collect()
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caratteri_di_wordcraft() {
        for ps in ["SourceSerif4-Regular", "SourceSerif4-It", "SourceSerif4-Bold"] {
            let b = bundled(ps).unwrap_or_else(|| panic!("WordCraft non ha più {ps}"));
            assert_eq!(ps_name(b).as_deref(), Some(ps));
        }
        assert_eq!(ps_name(INTER_REGULAR).as_deref(), Some("Inter-Regular"));
    }
}
