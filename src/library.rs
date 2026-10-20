//! Archivio delle pagine sul Mac (come la libreria di Inkspace).
//!
//! Il pad non permette di sfogliare le pagine al suo interno: consegna solo la
//! più vecchia, e per avere la successiva bisogna toglierla. Per questo, come
//! Inkspace, le pagine vengono scaricate qui e gestite sul computer.
//!
//! Ogni pagina è un file `library/<id>.json` leggibile (tratti, colori, data,
//! titolo). Eliminare sposta il file in `library/cestino/` (recuperabile).

use crate::settings::{color_to_hex, hex_to_color};
use crate::stroke::{Drawing, PenEvent, PenPhase, Stroke};
use chrono::{Local, NaiveDateTime};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const LIB_DIR: &str = "library";
const TRASH_DIR: &str = "library/cestino";
const TIME_FMT: &str = "%Y-%m-%d %H:%M:%S";

/// Da dove viene la pagina.
pub const SOURCE_PAD: &str = "pad";
pub const SOURCE_BOARD: &str = "lavagna";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Meta {
    /// Vuoto = si mostra la data.
    #[serde(default)]
    pub title: String,
    /// Data e ora locali "AAAA-MM-GG hh:mm:ss".
    pub created: String,
    #[serde(default)]
    pub source: String,
    /// Copia grezza del pad (nome file in exports/pad-raw), se c'è.
    #[serde(default)]
    pub raw: Option<String>,
}

impl Meta {
    pub fn new(created: NaiveDateTime, source: &str) -> Self {
        Self {
            title: String::new(),
            created: created.format(TIME_FMT).to_string(),
            source: source.into(),
            raw: None,
        }
    }

    pub fn created_dt(&self) -> Option<NaiveDateTime> {
        NaiveDateTime::parse_from_str(&self.created, TIME_FMT).ok()
    }

    /// Titolo da mostrare: quello scelto o la data.
    pub fn display_title(&self) -> String {
        if !self.title.trim().is_empty() {
            return self.title.clone();
        }
        match self.created_dt() {
            Some(t) => t.format("%d/%m/%Y %H:%M").to_string(),
            None => self.created.clone(),
        }
    }
}

/// Un tratto nel formato JSON dei file (archivio, documenti, firma).
#[derive(Clone, Serialize, Deserialize)]
pub struct StrokeFmt {
    /// Colore "#rrggbb".
    c: String,
    /// Spessore relativo.
    w: f32,
    /// Punti [x, y, pressione] in unità del pad.
    p: Vec<[f32; 3]>,
    /// Evidenziatore.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    h: bool,
}

#[derive(Serialize, Deserialize)]
struct PageFmt {
    version: u32,
    #[serde(flatten)]
    meta: Meta,
    width: f32,
    height: f32,
    strokes: Vec<StrokeFmt>,
}

pub struct Entry {
    pub id: String,
    pub meta: Meta,
    pub drawing: Drawing,
}

#[derive(Default)]
pub struct Library {
    /// Dalla più recente alla più vecchia.
    pub entries: Vec<Entry>,
    dir: PathBuf,
}

/// Tratti → formato JSON.
pub fn strokes_to_fmt(strokes: &[Stroke]) -> Vec<StrokeFmt> {
    strokes
        .iter()
        .filter(|s| !s.points.is_empty())
        .map(|s| StrokeFmt {
            c: color_to_hex(s.color),
            w: s.width,
            p: s.points.iter().map(|p| [p.x, p.y, p.pressure]).collect(),
            h: s.highlight,
        })
        .collect()
}

fn to_fmt(d: &Drawing, meta: &Meta) -> PageFmt {
    PageFmt {
        version: 1,
        meta: meta.clone(),
        width: d.width,
        height: d.height,
        strokes: strokes_to_fmt(&d.strokes),
    }
}

/// Formato JSON → tratti.
pub fn strokes_from_fmt(v: Vec<StrokeFmt>) -> Vec<Stroke> {
    v.into_iter()
        .map(|s| {
            let n = s.p.len();
            let mut st = Stroke::new(hex_to_color(&s.c).unwrap_or([20, 20, 20]), s.w);
            st.highlight = s.h;
            st.points = s
                .p
                .iter()
                .enumerate()
                .map(|(i, p)| PenEvent {
                    x: p[0],
                    y: p[1],
                    pressure: p[2],
                    phase: if i == 0 {
                        PenPhase::Down
                    } else if i + 1 == n {
                        PenPhase::Up
                    } else {
                        PenPhase::Move
                    },
                })
                .collect();
            st
        })
        .collect()
}

fn from_fmt(f: PageFmt) -> (Meta, Drawing) {
    let strokes = strokes_from_fmt(f.strokes);
    (
        f.meta,
        Drawing {
            strokes,
            width: f.width,
            height: f.height,
        },
    )
}

impl Library {
    /// Apre l'archivio nella cartella `library/`.
    pub fn open() -> Library {
        Self::open_in(Path::new(LIB_DIR))
    }

    pub fn open_in(dir: &Path) -> Library {
        let _ = std::fs::create_dir_all(dir);
        let mut lib = Library {
            entries: Vec::new(),
            dir: dir.to_path_buf(),
        };
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let path = e.path();
                if path.extension().and_then(|x| x.to_str()) != Some("json") {
                    continue;
                }
                let Some(id) = path.file_stem().and_then(|s| s.to_str()).map(String::from) else {
                    continue;
                };
                match std::fs::read_to_string(&path)
                    .map_err(|e| e.to_string())
                    .and_then(|t| serde_json::from_str::<PageFmt>(&t).map_err(|e| e.to_string()))
                {
                    Ok(f) => {
                        let (meta, drawing) = from_fmt(f);
                        lib.entries.push(Entry { id, meta, drawing });
                    }
                    Err(e) => tracing::warn!("archivio: {} illeggibile: {e}", path.display()),
                }
            }
        }
        lib.sort();
        lib
    }

    fn sort(&mut self) {
        self.entries
            .sort_by(|a, b| b.meta.created.cmp(&a.meta.created).then(b.id.cmp(&a.id)));
    }

    fn path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }

    fn trash_dir(&self) -> PathBuf {
        if self.dir == Path::new(LIB_DIR) {
            PathBuf::from(TRASH_DIR)
        } else {
            self.dir.join("cestino")
        }
    }

    pub fn get(&self, id: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.id == id)
    }

    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.entries.iter().position(|e| e.id == id)
    }

    fn write(&self, id: &str, d: &Drawing, meta: &Meta) -> std::io::Result<()> {
        let json = serde_json::to_string(&to_fmt(d, meta)).map_err(std::io::Error::other)?;
        // Scrittura sicura: file temporaneo e poi rinomina.
        let tmp = self.dir.join(format!(".{id}.tmp"));
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, self.path(id))
    }

    /// Aggiunge una pagina; ritorna il suo id.
    pub fn add(&mut self, d: &Drawing, meta: Meta) -> std::io::Result<String> {
        let base = meta
            .created_dt()
            .unwrap_or_else(|| Local::now().naive_local())
            .format("%Y%m%d-%H%M%S")
            .to_string();
        let mut id = base.clone();
        let mut n = 2;
        while self.path(&id).exists() || self.get(&id).is_some() {
            id = format!("{base}-{n}");
            n += 1;
        }
        self.write(&id, d, &meta)?;
        self.entries.push(Entry {
            id: id.clone(),
            meta,
            drawing: d.clone(),
        });
        self.sort();
        Ok(id)
    }

    /// C'è già una pagina del pad con questa data e questo numero di tratti?
    /// (Senza cancellare dal pad si riscarica sempre la stessa pagina.)
    pub fn has_pad_page(&self, created: NaiveDateTime, n_strokes: usize) -> bool {
        let c = created.format(TIME_FMT).to_string();
        self.entries.iter().any(|e| {
            e.meta.source == SOURCE_PAD && e.meta.created == c && e.drawing.strokes.len() == n_strokes
        })
    }

    pub fn rename(&mut self, id: &str, title: &str) -> std::io::Result<()> {
        let Some(i) = self.index_of(id) else {
            return Ok(());
        };
        self.entries[i].meta.title = title.trim().to_string();
        let e = &self.entries[i];
        self.write(id, &e.drawing, &e.meta)
    }

    /// Sposta le pagine nel cestino. Ritorna quante.
    pub fn trash(&mut self, ids: &[String]) -> std::io::Result<usize> {
        let tdir = self.trash_dir();
        std::fs::create_dir_all(&tdir)?;
        let mut n = 0;
        for id in ids {
            let src = self.path(id);
            if src.exists() {
                std::fs::rename(&src, tdir.join(format!("{id}.json")))?;
            }
            if let Some(i) = self.index_of(id) {
                self.entries.remove(i);
                n += 1;
            }
        }
        Ok(n)
    }

    /// Riporta pagine dal cestino.
    pub fn restore(&mut self, ids: &[String]) -> std::io::Result<usize> {
        let tdir = self.trash_dir();
        let mut n = 0;
        for id in ids {
            let src = tdir.join(format!("{id}.json"));
            if !src.exists() {
                continue;
            }
            let dst = self.path(id);
            std::fs::rename(&src, &dst)?;
            if let Ok(f) = serde_json::from_str::<PageFmt>(&std::fs::read_to_string(&dst)?) {
                let (meta, drawing) = from_fmt(f);
                self.entries.push(Entry { id: id.clone(), meta, drawing });
                n += 1;
            }
        }
        self.sort();
        Ok(n)
    }

    /// Importa le copie grezze del pad (exports/pad-raw/*.bin) non ancora in archivio.
    pub fn import_raw_dir(&mut self, dir: &Path, width: f32, height: f32) -> usize {
        let known: std::collections::HashSet<String> =
            self.entries.iter().filter_map(|e| e.meta.raw.clone()).collect();
        let Ok(rd) = std::fs::read_dir(dir) else {
            return 0;
        };
        let mut files: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("bin"))
            .collect();
        files.sort();
        let mut n = 0;
        for p in files {
            let Some(name) = p.file_name().and_then(|s| s.to_str()).map(String::from) else {
                continue;
            };
            if known.contains(&name) {
                continue;
            }
            let Ok(data) = std::fs::read(&p) else { continue };
            let Ok(raw) = crate::offline::parse(&data) else { continue };
            if raw.is_empty() {
                continue;
            }
            let d = crate::offline::to_drawing(&raw, width, height, [20, 20, 20], 2.0);
            // pad-AAAAMMGG-hhmmss[-n].bin → data del disegno
            let created = name
                .strip_prefix("pad-")
                .and_then(|s| s.get(..15))
                .and_then(|s| NaiveDateTime::parse_from_str(s, "%Y%m%d-%H%M%S").ok())
                .or_else(|| {
                    std::fs::metadata(&p)
                        .and_then(|m| m.modified())
                        .ok()
                        .map(|t| chrono::DateTime::<Local>::from(t).naive_local())
                })
                .unwrap_or_else(|| Local::now().naive_local());
            let mut meta = Meta::new(created, SOURCE_PAD);
            meta.raw = Some(name);
            if self.add(&d, meta).is_ok() {
                n += 1;
            }
        }
        n
    }
}

/// Intestazione del gruppo per giorno: "Oggi", "Ieri", "18 ottobre 2026".
pub fn day_label(created: Option<NaiveDateTime>) -> String {
    const MESI: [&str; 12] = [
        "gennaio", "febbraio", "marzo", "aprile", "maggio", "giugno", "luglio", "agosto",
        "settembre", "ottobre", "novembre", "dicembre",
    ];
    use chrono::Datelike;
    let Some(t) = created else {
        return "Senza data".into();
    };
    let today = Local::now().date_naive();
    let d = t.date();
    if d == today {
        "Oggi".into()
    } else if Some(d) == today.pred_opt() {
        "Ieri".into()
    } else {
        format!("{} {} {}", d.day(), MESI[d.month0() as usize], d.year())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drawing() -> Drawing {
        let mut s = Stroke::new([210, 40, 40], 3.0);
        for i in 0..5 {
            s.points.push(PenEvent { x: i as f32 * 100.0, y: 50.0, pressure: 0.5, phase: PenPhase::Move });
        }
        Drawing { strokes: vec![s], width: 29700.0, height: 21000.0 }
    }

    fn tmpdir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("wacom-lib-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn aggiungi_riapri_rinomina_cestino() {
        let dir = tmpdir("base");
        let mut lib = Library::open_in(&dir);
        let t1 = NaiveDateTime::parse_from_str("2026-10-18 10:00:00", TIME_FMT).unwrap();
        let t2 = NaiveDateTime::parse_from_str("2026-10-19 09:00:00", TIME_FMT).unwrap();
        let a = lib.add(&drawing(), Meta::new(t1, SOURCE_PAD)).unwrap();
        let b = lib.add(&drawing(), Meta::new(t2, SOURCE_BOARD)).unwrap();
        let c = lib.add(&drawing(), Meta::new(t2, SOURCE_BOARD)).unwrap();
        assert_ne!(b, c); // stessa data: id distinti
        lib.rename(&a, "Lezione di geometria").unwrap();

        let lib2 = Library::open_in(&dir);
        assert_eq!(lib2.entries.len(), 3);
        assert_eq!(lib2.entries[2].id, a); // la più vecchia in fondo
        assert_eq!(lib2.get(&a).unwrap().meta.display_title(), "Lezione di geometria");
        let d = &lib2.get(&b).unwrap().drawing;
        assert_eq!(d.strokes[0].color, [210, 40, 40]);
        assert_eq!(d.strokes[0].points.len(), 5);
        assert_eq!(d.strokes[0].points[0].phase, PenPhase::Down);

        let mut lib3 = lib2;
        assert_eq!(lib3.trash(&[a.clone(), b.clone()]).unwrap(), 2);
        assert_eq!(Library::open_in(&dir).entries.len(), 1);
        assert_eq!(lib3.restore(&[a.clone()]).unwrap(), 1);
        assert_eq!(Library::open_in(&dir).entries.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn etichette_giorno() {
        let now = Local::now().naive_local();
        assert_eq!(day_label(Some(now)), "Oggi");
        let t = NaiveDateTime::parse_from_str("2025-03-07 10:00:00", TIME_FMT).unwrap();
        assert_eq!(day_label(Some(t)), "7 marzo 2025");
    }
}
