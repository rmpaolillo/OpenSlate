//! Decodifica dei disegni salvati nella memoria del pad (formato "StrokeFile").
//! Traduzione di `tuhi/protocol.py` (StrokeFile, StrokeDataType, StrokeDelta…).
//!
//! Il formato è compresso: ogni pacchetto inizia con un byte-maschera; il
//! numero di bit a 1 dice quanti byte seguono. Le coordinate arrivano come
//! valori assoluti o come delta cumulativi (vedi `parse`).

use crate::stroke::{Drawing, PenEvent, PenPhase, Rgb, Stroke};

/// Pressione massima dello Slate.
const PRESSURE_MAX: f32 = 2047.0;

#[derive(Debug, PartialEq)]
enum Kind {
    Unknown,
    FileHeader,
    StrokeHeader,
    StrokeEnd,
    Point,
    Delta,
    Eof,
    LostPoint,
}

fn ones(b: u8) -> usize {
    b.count_ones() as usize
}

fn identify(d: &[u8]) -> Kind {
    let header = d[0];
    let n = ones(header);
    let payload = &d[1..(1 + n).min(d.len())];
    if d.starts_with(&[0x67, 0x82, 0x69, 0x65]) || d.starts_with(&[0x62, 0x38, 0x62, 0x74]) {
        return Kind::FileHeader;
    }
    if d.starts_with(&[0xfc, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]) {
        return Kind::StrokeEnd;
    }
    if payload == [0xff; 8] {
        return Kind::Eof;
    }
    if header & 0x3 == 0 {
        return Kind::Delta;
    }
    if payload.is_empty() {
        return Kind::Unknown;
    }
    if payload[0] == 0xfa || payload.starts_with(&[0xff, 0xee, 0xee]) {
        return Kind::StrokeHeader;
    }
    if payload.starts_with(&[0xff, 0xff]) {
        return Kind::Point;
    }
    if payload.starts_with(&[0xdd, 0xdd]) {
        return Kind::LostPoint;
    }
    Kind::Unknown
}

/// Valore assoluto oppure delta di una coordinata.
#[derive(Default, Clone, Copy)]
struct Coord {
    abs: Option<i32>,
    delta: Option<i32>,
}

/// Decodifica un pacchetto delta: maschera + byte. Ritorna (x, y, p, dimensione).
fn parse_delta(d: &[u8]) -> Option<(Coord, Coord, Coord, usize)> {
    let mask = d[0];
    let mut off = 1;
    let mut take = |m: u8| -> Option<Coord> {
        match m {
            0 => Some(Coord::default()),
            2 => {
                let v = *d.get(off)? as i8 as i32;
                off += 1;
                Some(Coord { abs: None, delta: Some(v) })
            }
            3 => {
                let v = u16::from_le_bytes([*d.get(off)?, *d.get(off + 1)?]) as i32;
                off += 2;
                Some(Coord { abs: Some(v), delta: None })
            }
            _ => None, // maschera 1: non prevista dal formato
        }
    };
    let x = take((mask >> 2) & 3)?;
    let y = take((mask >> 4) & 3)?;
    let p = take((mask >> 6) & 3)?;
    Some((x, y, p, off))
}

/// Tratti come liste di punti (x, y, pressione grezza).
pub type RawStrokes = Vec<Vec<(i32, i32, i32)>>;

/// Decodifica un file. Errore se l'intestazione non è riconosciuta.
pub fn parse(data: &[u8]) -> Result<RawStrokes, String> {
    let mut off = if data.starts_with(&[0x62, 0x38, 0x62, 0x74]) {
        4 // Slate / Spark
    } else if data.starts_with(&[0x67, 0x82, 0x69, 0x65]) {
        16 // Intuos Pro
    } else {
        return Err(format!(
            "formato sconosciuto: {:02x?}",
            &data[..data.len().min(4)]
        ));
    };

    let mut strokes: RawStrokes = Vec::new();
    let mut points: Vec<(i32, i32, i32)> = Vec::new();
    let mut last = (0i32, 0i32, 0i32); // ultimo punto assoluto calcolato
    let mut last_delta = (0i32, 0i32, 0i32); // delta cumulativo

    while off < data.len() {
        let d = &data[off..];
        let kind = identify(d);
        let simple = 1 + ones(d[0]);
        let size = match kind {
            Kind::FileHeader => {
                tracing::warn!("intestazione inattesa al byte {off}");
                break;
            }
            Kind::StrokeEnd => {
                if !points.is_empty() {
                    strokes.push(std::mem::take(&mut points));
                }
                simple
            }
            Kind::Eof => {
                if !points.is_empty() {
                    strokes.push(std::mem::take(&mut points));
                }
                break;
            }
            Kind::StrokeHeader => {
                last_delta = (0, 0, 0);
                if !points.is_empty() {
                    strokes.push(std::mem::take(&mut points));
                }
                simple
            }
            Kind::LostPoint | Kind::Unknown => simple,
            Kind::Point | Kind::Delta => {
                // Un POINT è un delta con due byte ff ff dopo la maschera.
                let parsed = if kind == Kind::Point {
                    let mut buf = Vec::with_capacity(d.len().min(16));
                    buf.push(d[0] & !0x3);
                    buf.extend_from_slice(&d[3.min(d.len())..d.len().min(16)]);
                    parse_delta(&buf).map(|(x, y, p, n)| (x, y, p, n + 2))
                } else {
                    parse_delta(d)
                };
                let Some((cx, cy, cp, n)) = parsed else {
                    tracing::warn!("pacchetto non valido al byte {off}: {:02x?}", &d[..d.len().min(9)]);
                    break;
                };
                // Compressione del pad: delta cumulativi per asse, azzerati
                // quando arriva un valore assoluto (vedi commento in Tuhi).
                let (mut x, mut y, mut p) = last;
                let (mut dx, mut dy, mut dp) = last_delta;
                if let Some(v) = cx.delta {
                    dx += v;
                } else if let Some(v) = cx.abs {
                    x = v;
                    dx = 0;
                }
                if let Some(v) = cy.delta {
                    dy += v;
                } else if let Some(v) = cy.abs {
                    y = v;
                    dy = 0;
                }
                if let Some(v) = cp.delta {
                    dp += v;
                } else if let Some(v) = cp.abs {
                    p = v;
                    dp = 0;
                }
                last_delta = (dx, dy, dp);
                last = (x + dx, y + dy, p + dp);
                points.push(last);
                n
            }
        };
        off += size.max(1);
    }
    if !points.is_empty() {
        strokes.push(points);
    }
    Ok(strokes)
}

/// Costruisce un `Drawing` dai tratti decodificati.
pub fn to_drawing(raw: &RawStrokes, width: f32, height: f32, color: Rgb, pen_width: f32) -> Drawing {
    let strokes = raw
        .iter()
        .map(|pts| {
            let mut s = Stroke::new(color, pen_width);
            let n = pts.len();
            s.points = pts
                .iter()
                .enumerate()
                .map(|(i, &(x, y, p))| PenEvent {
                    x: x as f32,
                    y: y as f32,
                    pressure: (p as f32 / PRESSURE_MAX).clamp(0.0, 1.0),
                    phase: if i == 0 {
                        PenPhase::Down
                    } else if i + 1 == n {
                        PenPhase::Up
                    } else {
                        PenPhase::Move
                    },
                })
                .collect();
            s
        })
        .collect();
    Drawing {
        strokes,
        width,
        height,
    }
}

/// Data della risposta GET_STROKES (BCD UTC "aammgghhmmss") in ora locale.
pub fn bcd_time_local(bcd: &[u8]) -> Option<chrono::NaiveDateTime> {
    use chrono::{Local, NaiveDateTime, TimeZone, Utc};
    if bcd.len() < 6 {
        return None;
    }
    let s: String = bcd[..6].iter().map(|b| format!("{b:02x}")).collect();
    let t = NaiveDateTime::parse_from_str(&s, "%y%m%d%H%M%S").ok()?;
    Some(Utc.from_utc_datetime(&t).with_timezone(&Local).naive_local())
}

/// Come `bcd_time_local`, formattata per un nome file.
pub fn bcd_time_label(bcd: &[u8]) -> Option<String> {
    bcd_time_local(bcd).map(|t| t.format("%Y%m%d-%H%M%S").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodifica_file_sintetico() {
        let mut f = vec![0x62, 0x38, 0x62, 0x74];
        // intestazione tratto Slate: maschera 0xff (8 byte) ff ee ee ...
        f.extend_from_slice(&[0xff, 0xff, 0xee, 0xee, 0x00, 0x00, 0x10, 0x00, 0x00]);
        // POINT assoluto: maschera 0xff, ff ff, x=1000, y=2000, p=500
        f.extend_from_slice(&[0xff, 0xff, 0xff, 0xe8, 0x03, 0xd0, 0x07, 0xf4, 0x01]);
        // DELTA: dx=+5, dy=-3, p assente -> maschera 0b0010_1000 = 0x28
        f.extend_from_slice(&[0x28, 0x05, 0xfd]);
        // DELTA: dx=+1 -> cumulativo 6
        f.extend_from_slice(&[0x08, 0x01]);
        // fine tratto
        f.extend_from_slice(&[0xfc, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
        // EOF
        f.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
        let s = parse(&f).unwrap();
        assert_eq!(s.len(), 1);
        assert_eq!(s[0], vec![(1000, 2000, 500), (1005, 1997, 500), (1011, 1994, 500)]);
    }

    #[test]
    fn data_bcd() {
        assert!(bcd_time_label(&[0x26, 0x10, 0x18, 0x21, 0x30, 0x05]).is_some());
        assert!(bcd_time_label(&[0x26, 0x99]).is_none());
    }
}
