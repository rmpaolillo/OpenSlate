//! Decodifica dei pacchetti penna in tempo reale (Tuhi:
//! `WacomProtocolBase._on_pen_data_changed`).
//!
//! Formato (caratteristica LIVE 0x1524):
//!   `a2 len <timestamp>`   la penna entra in prossimità
//!   `a1 len <punti…>`      punti da 6 byte: x u16 LE, y u16 LE, pressione u16 LE
//!                          `ff ff ff ff ff ff` = la penna esce dalla prossimità
//!   `10 …`                 pressione/pulsanti (ignorato)
//! Le coordinate sono in punti del pad; la pressione arriva a 2047 (Slate).

use crate::stroke::{PenEvent, PenPhase};

pub const PKT_PROXIMITY: u8 = 0xa2;
pub const PKT_COORD: u8 = 0xa1;
pub const POINT_SIZE: usize = 6;
/// Pressione massima dello Slate (tuhi/wacom.py: pressure = 2047).
pub const PRESSURE_MAX: f32 = 2047.0;

/// Decodificatore con stato: emette Down/Move/Up in base alla pressione.
#[derive(Default)]
pub struct LiveDecoder {
    was_down: bool,
    last: Option<(f32, f32)>,
}

impl LiveDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Chiude il tratto in corso (penna sollevata o uscita di prossimità).
    fn lift(&mut self, out: &mut Vec<PenEvent>) {
        if self.was_down {
            if let Some((x, y)) = self.last {
                out.push(PenEvent {
                    x,
                    y,
                    pressure: 0.0,
                    phase: PenPhase::Up,
                });
            }
        }
        self.was_down = false;
    }

    /// Decodifica un pacchetto grezzo in zero o più eventi penna.
    pub fn decode(&mut self, raw: &[u8]) -> Vec<PenEvent> {
        let mut out = Vec::new();
        if raw.len() < 2 {
            return out;
        }
        match raw[0] {
            PKT_PROXIMITY => self.lift(&mut out),
            PKT_COORD => {
                let len = raw[1] as usize;
                if len % POINT_SIZE != 0 {
                    tracing::warn!("pacchetto live malformato: {raw:02x?}");
                    return out;
                }
                let end = (2 + len).min(raw.len());
                for chunk in raw[2..end].chunks_exact(POINT_SIZE) {
                    if chunk == [0xff; POINT_SIZE] {
                        self.lift(&mut out); // penna fuori prossimità
                        continue;
                    }
                    let x = u16::from_le_bytes([chunk[0], chunk[1]]) as f32;
                    let y = u16::from_le_bytes([chunk[2], chunk[3]]) as f32;
                    let p = u16::from_le_bytes([chunk[4], chunk[5]]) as f32;
                    let is_down = p > 0.0;
                    let pressure = (p / PRESSURE_MAX).clamp(0.0, 1.0);
                    match (self.was_down, is_down) {
                        (false, true) => out.push(PenEvent {
                            x,
                            y,
                            pressure,
                            phase: PenPhase::Down,
                        }),
                        (true, true) => out.push(PenEvent {
                            x,
                            y,
                            pressure,
                            phase: PenPhase::Move,
                        }),
                        (true, false) => out.push(PenEvent {
                            x,
                            y,
                            pressure: 0.0,
                            phase: PenPhase::Up,
                        }),
                        (false, false) => {} // in aria: niente
                    }
                    self.was_down = is_down;
                    self.last = Some((x, y));
                }
            }
            _ => {}
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ff_chiude_il_tratto_senza_punti_fantasma() {
        let mut d = LiveDecoder::new();
        let ev = d.decode(&[0xa1, 0x0c, 0x10, 0x00, 0x20, 0x00, 0x05, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
        assert_eq!(ev.len(), 2);
        assert_eq!(ev[0].phase, PenPhase::Down);
        assert_eq!(ev[1].phase, PenPhase::Up);
        assert_eq!((ev[1].x, ev[1].y), (16.0, 32.0)); // nessun punto a 65535
    }
}
