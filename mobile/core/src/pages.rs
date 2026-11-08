//! Archivio delle pagine sul telefono, nello STESSO formato della desktop
//! (`src/library.rs`): un file `<id>.json` per pagina, id dalla data
//! ("AAAAMMGG-hhmmss"), cestino in `cestino/`. Così le pagine potranno passare
//! dal telefono al computer e viceversa senza conversioni.

use crate::offline;
use crate::session::{PenPoint, Phase};
use chrono::{Local, NaiveDateTime};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const TIME_FMT: &str = "%Y-%m-%d %H:%M:%S";
const TRASH_DIR: &str = "cestino";
/// Da dove viene la pagina (come la desktop).
pub const SOURCE_PAD: &str = "pad";
pub const SOURCE_BOARD: &str = "lavagna";
/// Inchiostro e spessore predefiniti della desktop (`settings.rs`).
const INK: &str = "#141414";
const PEN_WIDTH: f32 = 2.0;

// ---------------------------------------------------------------------------
// Tipi esposti a Kotlin
// ---------------------------------------------------------------------------

#[derive(uniffi::Record, Clone, Debug, PartialEq)]
pub struct PageStroke {
    /// "#rrggbb".
    pub color: String,
    /// Spessore relativo: pixel su un foglio alto 800 px.
    pub width: f32,
    pub highlight: bool,
    pub points: Vec<PenPoint>,
}

#[derive(uniffi::Record, Clone, Debug, PartialEq)]
pub struct Page {
    /// Vuoto = pagina nuova (l'id lo sceglie `library_save`).
    pub id: String,
    /// Vuoto = si mostra la data.
    pub title: String,
    /// Data e ora locali "AAAA-MM-GG hh:mm:ss".
    pub created: String,
    /// "pad" o "lavagna".
    pub source: String,
    /// Copia grezza del pad, se c'è.
    pub raw: Option<String>,
    /// Area attiva del pad (unità pad).
    pub width: f32,
    pub height: f32,
    pub strokes: Vec<PageStroke>,
}

#[derive(uniffi::Record, Clone, Debug)]
pub struct PageSummary {
    pub id: String,
    /// Titolo scelto o, se manca, la data.
    pub title: String,
    pub created: String,
    /// "AAAA-MM-GG", per raggruppare per giorno.
    pub day: String,
    pub source: String,
    pub strokes: u32,
}

#[derive(uniffi::Error, thiserror::Error, Debug)]
pub enum StoreError {
    #[error("{reason}")]
    Io { reason: String },
}

impl From<std::io::Error> for StoreError {
    fn from(e: std::io::Error) -> Self {
        StoreError::Io { reason: e.to_string() }
    }
}

// ---------------------------------------------------------------------------
// Formato dei file (identico a `library::PageFmt` della desktop)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct StrokeFmt {
    c: String,
    w: f32,
    p: Vec<[f32; 3]>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    h: bool,
}

#[derive(Serialize, Deserialize)]
struct PageFmt {
    version: u32,
    #[serde(default)]
    title: String,
    created: String,
    #[serde(default)]
    source: String,
    #[serde(default)]
    raw: Option<String>,
    width: f32,
    height: f32,
    strokes: Vec<StrokeFmt>,
    /// Grafico dell'AI (desktop): lo si conserva se c'è.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    chart: Option<[f32; 4]>,
}

fn to_fmt(p: &Page, chart: Option<[f32; 4]>) -> PageFmt {
    PageFmt {
        version: 1,
        title: p.title.clone(),
        created: p.created.clone(),
        source: p.source.clone(),
        raw: p.raw.clone(),
        width: p.width,
        height: p.height,
        strokes: p
            .strokes
            .iter()
            .filter(|s| !s.points.is_empty())
            .map(|s| StrokeFmt {
                c: s.color.clone(),
                w: s.width,
                p: s.points.iter().map(|q| [q.x, q.y, q.pressure]).collect(),
                h: s.highlight,
            })
            .collect(),
        chart,
    }
}

fn points_with_phases(pts: impl ExactSizeIterator<Item = (f32, f32, f32)>) -> Vec<PenPoint> {
    let n = pts.len();
    pts.enumerate()
        .map(|(i, (x, y, pressure))| PenPoint {
            x,
            y,
            pressure,
            phase: if i == 0 {
                Phase::Down
            } else if i + 1 == n {
                Phase::Up
            } else {
                Phase::Move
            },
        })
        .collect()
}

fn from_fmt(id: &str, f: PageFmt) -> Page {
    Page {
        id: id.to_string(),
        title: f.title,
        created: f.created,
        source: f.source,
        raw: f.raw,
        width: f.width,
        height: f.height,
        strokes: f
            .strokes
            .into_iter()
            .filter(|s| !s.p.is_empty())
            .map(|s| PageStroke {
                color: s.c,
                width: s.w,
                highlight: s.h,
                points: points_with_phases(s.p.iter().map(|q| (q[0], q[1], q[2]))),
            })
            .collect(),
    }
}

fn path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.json"))
}

fn read(dir: &Path, id: &str) -> Result<PageFmt, StoreError> {
    let text = std::fs::read_to_string(path(dir, id))?;
    serde_json::from_str(&text).map_err(|e| StoreError::Io { reason: e.to_string() })
}

/// Un id è un nome di file: niente percorsi.
fn check_id(id: &str) -> Result<(), StoreError> {
    if id.is_empty() || id.contains(['/', '\\']) || id.starts_with('.') {
        return Err(StoreError::Io { reason: format!("id non valido: {id}") });
    }
    Ok(())
}

/// Scrive passando da un file temporaneo: una pagina non resta mai a metà.
fn write(dir: &Path, id: &str, f: &PageFmt) -> Result<(), StoreError> {
    let json = serde_json::to_string(f).map_err(|e| StoreError::Io { reason: e.to_string() })?;
    let tmp = dir.join(format!(".{id}.json.tmp"));
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, path(dir, id))?;
    Ok(())
}

fn display_title(title: &str, created: &str) -> String {
    if !title.trim().is_empty() {
        return title.to_string();
    }
    match NaiveDateTime::parse_from_str(created, TIME_FMT) {
        Ok(t) => t.format("%d/%m/%Y %H:%M").to_string(),
        Err(_) => created.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Funzioni esposte
// ---------------------------------------------------------------------------

/// Data e ora locali adesso, nel formato delle pagine.
#[uniffi::export]
pub fn now_created() -> String {
    Local::now().naive_local().format(TIME_FMT).to_string()
}

/// Le pagine dell'archivio, dalla più recente.
#[uniffi::export]
pub fn library_list(dir: String) -> Vec<PageSummary> {
    let dir = Path::new(&dir);
    let _ = std::fs::create_dir_all(dir);
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) != Some("json") {
            continue;
        }
        let Some(id) = p.file_stem().and_then(|s| s.to_str()) else { continue };
        if id.starts_with('.') {
            continue;
        }
        match read(dir, id) {
            Ok(f) => out.push(PageSummary {
                id: id.to_string(),
                title: display_title(&f.title, &f.created),
                day: f.created.get(..10).unwrap_or_default().to_string(),
                created: f.created,
                source: f.source,
                strokes: f.strokes.len() as u32,
            }),
            Err(e) => tracing::warn!("archivio: {id} illeggibile: {e}"),
        }
    }
    out.sort_by(|a, b| b.created.cmp(&a.created).then(b.id.cmp(&a.id)));
    out
}

#[uniffi::export]
pub fn library_load(dir: String, id: String) -> Result<Page, StoreError> {
    check_id(&id)?;
    Ok(from_fmt(&id, read(Path::new(&dir), &id)?))
}

/// Salva una pagina. Id vuoto = pagina nuova (id dalla data, mai uno già
/// usato, nemmeno nel cestino); altrimenti la riscrive. Restituisce l'id.
#[uniffi::export]
pub fn library_save(dir: String, page: Page) -> Result<String, StoreError> {
    let dir = Path::new(&dir);
    std::fs::create_dir_all(dir)?;
    if !page.id.is_empty() {
        check_id(&page.id)?;
        // il grafico dell'AI di una pagina venuta dalla desktop resta
        let chart = read(dir, &page.id).ok().and_then(|f| f.chart);
        write(dir, &page.id, &to_fmt(&page, chart))?;
        return Ok(page.id);
    }
    let base = NaiveDateTime::parse_from_str(&page.created, TIME_FMT)
        .unwrap_or_else(|_| Local::now().naive_local())
        .format("%Y%m%d-%H%M%S")
        .to_string();
    let trash = dir.join(TRASH_DIR);
    let mut id = base.clone();
    let mut n = 2;
    while path(dir, &id).exists() || path(&trash, &id).exists() {
        id = format!("{base}-{n}");
        n += 1;
    }
    write(dir, &id, &to_fmt(&page, None))?;
    Ok(id)
}

#[uniffi::export]
pub fn library_rename(dir: String, id: String, title: String) -> Result<(), StoreError> {
    check_id(&id)?;
    let dir = Path::new(&dir);
    let mut f = read(dir, &id)?;
    f.title = title.trim().to_string();
    write(dir, &id, &f)
}

/// Sposta la pagina nel cestino (recuperabile, come la desktop).
#[uniffi::export]
pub fn library_trash(dir: String, id: String) -> Result<(), StoreError> {
    check_id(&id)?;
    let dir = Path::new(&dir);
    let trash = dir.join(TRASH_DIR);
    std::fs::create_dir_all(&trash)?;
    std::fs::rename(path(dir, &id), path(&trash, &id))?;
    Ok(())
}

/// Una pagina scaricata dal pad (byte grezzi "StrokeFile" e data BCD della
/// risposta GET_STROKES), decodificata con `src/offline.rs` della desktop.
/// `None` se il formato non si riconosce.
#[uniffi::export]
pub fn page_from_pad(raw: Vec<u8>, time_bcd: Vec<u8>, width: f32, height: f32, raw_name: Option<String>) -> Option<Page> {
    let strokes = match offline::parse(&raw) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("pagina del pad non decodificata: {e}");
            return None;
        }
    };
    let created = offline::bcd_time_local(&time_bcd).unwrap_or_else(|| Local::now().naive_local());
    Some(Page {
        id: String::new(),
        title: String::new(),
        created: created.format(TIME_FMT).to_string(),
        source: SOURCE_PAD.into(),
        raw: raw_name,
        width,
        height,
        strokes: strokes
            .iter()
            .map(|pts| PageStroke {
                color: INK.into(),
                width: PEN_WIDTH,
                highlight: false,
                points: points_with_phases(pts.iter().map(|&(x, y, p)| {
                    (x as f32, y as f32, (p as f32 / crate::live::PRESSURE_MAX).clamp(0.0, 1.0))
                })),
            })
            .collect(),
    })
}

/// Etichetta della data BCD per il nome della copia grezza ("AAAAMMGG-hhmmss").
#[uniffi::export]
pub fn pad_time_label(time_bcd: Vec<u8>) -> String {
    offline::bcd_time_label(&time_bcd).unwrap_or_else(|| Local::now().format("%Y%m%d-%H%M%S").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> String {
        let d = std::env::temp_dir().join(format!("openslate-pages-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d.to_string_lossy().into_owned()
    }

    fn page() -> Page {
        Page {
            id: String::new(),
            title: String::new(),
            created: "2026-11-08 10:30:00".into(),
            source: SOURCE_BOARD.into(),
            raw: None,
            width: 21600.0,
            height: 14800.0,
            strokes: vec![PageStroke {
                color: INK.into(),
                width: 2.0,
                highlight: false,
                points: points_with_phases([(1.0, 2.0, 0.5), (3.0, 4.0, 0.6)].into_iter()),
            }],
        }
    }

    #[test]
    fn salva_elenca_rinomina_cestina() {
        let dir = tmp("salva");
        let a = library_save(dir.clone(), page()).unwrap();
        let b = library_save(dir.clone(), page()).unwrap();
        assert_eq!((a.as_str(), b.as_str()), ("20261108-103000", "20261108-103000-2"));
        let l = library_list(dir.clone());
        assert_eq!(l.len(), 2);
        assert_eq!(l[0].title, "08/11/2026 10:30");
        assert_eq!(l[0].day, "2026-11-08");
        library_rename(dir.clone(), a.clone(), " Riunione ".into()).unwrap();
        let p = library_load(dir.clone(), a.clone()).unwrap();
        assert_eq!(p.title, "Riunione");
        assert_eq!(p.strokes, page().strokes);
        library_trash(dir.clone(), a.clone()).unwrap();
        assert_eq!(library_list(dir.clone()).len(), 1);
        // l'id del cestino non si riusa
        assert_eq!(library_save(dir.clone(), page()).unwrap(), "20261108-103000-3");
        assert!(library_load(dir.clone(), "../x".into()).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn legge_il_formato_della_desktop() {
        let dir = tmp("desktop");
        std::fs::create_dir_all(&dir).unwrap();
        let json = r##"{"version":1,"title":"","created":"2026-11-01 09:00:00","source":"pad","raw":null,"width":100.0,"height":50.0,"strokes":[{"c":"#ff0000","w":3.0,"p":[[1,2,0.5],[2,3,0.5],[3,4,0.0]],"h":true}],"chart":[0.1,0.1,0.5,0.5]}"##;
        std::fs::write(Path::new(&dir).join("20261101-090000.json"), json).unwrap();
        let mut p = library_load(dir.clone(), "20261101-090000".into()).unwrap();
        assert!(p.strokes[0].highlight);
        assert_eq!(p.strokes[0].points[2].phase, Phase::Up);
        // riscrivendola il grafico resta
        p.title = "x".into();
        library_save(dir.clone(), p).unwrap();
        let t = std::fs::read_to_string(Path::new(&dir).join("20261101-090000.json")).unwrap();
        assert!(t.contains("\"chart\":[0.1,0.1,0.5,0.5]"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
