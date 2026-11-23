//! Decodifica dei pacchetti penna in tempo reale (Tuhi:
//! `WacomProtocolBase._on_pen_data_changed`).
//!
//! Formato (caratteristica LIVE 0x1524):
//!   `a2 len <timestamp>`   la penna entra in prossimità
//!   `a1 len <punti…>`      punti da 6 byte: x u16 LE, y u16 LE, pressione u16 LE
//!                          (pressione 0 = la penna è in aria, vicina al pad)
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
                if !len.is_multiple_of(POINT_SIZE) {
                    tracing::warn!("pacchetto live malformato: {raw:02x?}");
                    return out;
                }
                let end = (2 + len).min(raw.len());
                for chunk in raw[2..end].chunks_exact(POINT_SIZE) {
                    if chunk == [0xff; POINT_SIZE] {
                        self.lift(&mut out); // penna fuori prossimità
                        if let Some((x, y)) = self.last {
                            out.push(PenEvent { x, y, pressure: 0.0, phase: PenPhase::Away });
                        }
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
                        // in aria, vicina al pad: non scrive, ma si vede dov'è
                        (false, false) => out.push(PenEvent {
                            x,
                            y,
                            pressure: 0.0,
                            phase: PenPhase::Hover,
                        }),
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
        assert_eq!(ev.len(), 3);
        assert_eq!(ev[0].phase, PenPhase::Down);
        assert_eq!(ev[1].phase, PenPhase::Up);
        assert_eq!((ev[1].x, ev[1].y), (16.0, 32.0)); // nessun punto a 65535
        // e poi la penna si allontana, sempre dall'ultimo punto vero
        assert_eq!(ev[2].phase, PenPhase::Away);
        assert_eq!((ev[2].x, ev[2].y), (16.0, 32.0));
    }

    #[test]
    fn la_penna_in_aria_si_vede() {
        let mut d = LiveDecoder::new();
        // in aria (pressione 0), poi giù, poi di nuovo in aria
        let ev = d.decode(&[0xa1, 0x12, 0x10, 0x00, 0x20, 0x00, 0x00, 0x00, 0x11, 0x00, 0x21, 0x00, 0xff, 0x03, 0x12, 0x00, 0x22, 0x00, 0x00, 0x00]);
        let phases: Vec<PenPhase> = ev.iter().map(|e| e.phase).collect();
        assert_eq!(phases, [PenPhase::Hover, PenPhase::Down, PenPhase::Up]);
        assert!((ev[1].pressure - 1023.0 / PRESSURE_MAX).abs() < 1e-6);
        let ev = d.decode(&[0xa1, 0x06, 0x13, 0x00, 0x23, 0x00, 0x00, 0x00]);
        assert_eq!(ev[0].phase, PenPhase::Hover);
        assert_eq!((ev[0].x, ev[0].y), (19.0, 35.0));
    }
}
