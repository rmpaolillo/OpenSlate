//! Archivio delle pagine sul Mac (come la libreria di Inkspace).
//!
//! Il pad non permette di sfogliare le pagine al suo interno: consegna solo la
//! più vecchia, e per avere la successiva bisogna toglierla. Per questo, come
//! Inkspace, le pagine vengono scaricate qui e gestite sul computer.
//!
//! Ogni pagina è un file `library/<id>.json` leggibile (tratti, colori, data,
//! titolo). Eliminare sposta il file in `library/cestino/` (recuperabile).
//! Se sulla lavagna c'era un grafico dell'AI, l'immagine sta accanto in
//! `library/<id>.grafico.png` (e va nel cestino con la pagina).

use crate::scene::PageBg;
use crate::settings::{color_to_hex, hex_to_color};
use crate::stroke::{Drawing, PenEvent, PenPhase, Stroke};
use chrono::{Local, NaiveDateTime};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

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
    /// Il grafico dell'AI sul foglio: dove sta (u, v, larghezza, altezza
    /// sulla pagina, 0..1); l'immagine è `<id>.grafico.png`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    chart: Option<[f32; 4]>,
}

pub struct Entry {
    pub id: String,
    pub meta: Meta,
    pub drawing: Drawing,
    /// Il grafico dell'AI messo sul foglio, se c'era.
    pub chart: Option<PageBg>,
}

/// L'immagine del grafico accanto alla pagina.
fn chart_file(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.grafico.png"))
}

/// Il grafico salvato con la pagina `id` (se c'è e si legge).
fn read_chart(dir: &Path, id: &str, rect: Option<[f32; 4]>) -> Option<PageBg> {
    let rect = rect?;
    match std::fs::read(chart_file(dir, id)) {
        Ok(png) => Some(PageBg { png: Arc::new(png), rect, key: format!("chart:lib-{id}") }),
        Err(e) => {
            tracing::warn!("archivio: grafico di {id} illeggibile: {e}");
            None
        }
    }
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

fn to_fmt(d: &Drawing, meta: &Meta, chart: Option<&PageBg>) -> PageFmt {
    PageFmt {
        version: 1,
        meta: meta.clone(),
        width: d.width,
        height: d.height,
        strokes: strokes_to_fmt(&d.strokes),
        chart: chart.map(|c| c.rect),
    }
}

/// Formato JSON → tratti.
pub fn strokes_from_fmt(v: Vec<StrokeFmt>) -> Vec<Stroke> {
    v.into_iter()
        // un tratto senza punti (file rovinato o scritto a mano) non serve e
        // farebbe inciampare chi disegna
        .filter(|s| !s.p.is_empty())
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

fn from_fmt(f: PageFmt, dir: &Path, id: &str) -> Entry {
    let chart = read_chart(dir, id, f.chart);
    let strokes = strokes_from_fmt(f.strokes);
    Entry {
        id: id.to_string(),
        meta: f.meta,
        drawing: Drawing {
            strokes,
            width: f.width,
            height: f.height,
        },
        chart,
    }
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
                        let e = from_fmt(f, dir, &id);
                        lib.entries.push(e);
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

    fn write(&self, id: &str, d: &Drawing, meta: &Meta, chart: Option<&PageBg>) -> std::io::Result<()> {
        if let Some(c) = chart {
            std::fs::write(chart_file(&self.dir, id), c.png.as_slice())?;
        }
        let json = serde_json::to_string(&to_fmt(d, meta, chart)).map_err(std::io::Error::other)?;
        // Scrittura sicura: file temporaneo e poi rinomina.
        let tmp = self.dir.join(format!(".{id}.tmp"));
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, self.path(id))
    }

    /// Aggiunge una pagina; ritorna il suo id.
    pub fn add(&mut self, d: &Drawing, meta: Meta) -> std::io::Result<String> {
        self.add_with_chart(d, meta, None)
    }

    /// Aggiunge una pagina col grafico dell'AI che c'era sopra.
    pub fn add_with_chart(&mut self, d: &Drawing, meta: Meta, chart: Option<&PageBg>) -> std::io::Result<String> {
        let base = meta
            .created_dt()
            .unwrap_or_else(|| Local::now().naive_local())
            .format("%Y%m%d-%H%M%S")
            .to_string();
        let mut id = base.clone();
        let mut n = 2;
        // anche il cestino: un id già usato là non si riusa (si sovrascriverebbe)
        while self.path(&id).exists() || self.get(&id).is_some() || self.trash_dir().join(format!("{id}.json")).exists() {
            id = format!("{base}-{n}");
            n += 1;
        }
        self.write(&id, d, &meta, chart)?;
        self.entries.push(Entry {
            id: id.clone(),
            meta,
            drawing: d.clone(),
            chart: chart.map(|c| PageBg { key: format!("chart:lib-{id}"), ..c.clone() }),
        });
        self.sort();
        Ok(id)
    }

    /// Riscrive una pagina che c'è già (tratti e grafico; titolo e data
    /// restano): la pagina dell'archivio aperta sulla lavagna, salvata di nuovo.
    pub fn update(&mut self, id: &str, d: &Drawing, chart: Option<&PageBg>) -> std::io::Result<()> {
        let Some(i) = self.index_of(id) else {
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, format!("pagina {id} non trovata")));
        };
        let meta = self.entries[i].meta.clone();
        if chart.is_none() {
            let _ = std::fs::remove_file(chart_file(&self.dir, id));
        }
        self.write(id, d, &meta, chart)?;
        let e = &mut self.entries[i];
        e.drawing = d.clone();
        e.chart = chart.map(|c| PageBg { key: format!("chart:lib-{id}-{}", chrono::Local::now().timestamp_millis()), ..c.clone() });
        Ok(())
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
        // l'immagine del grafico c'è già: si riscrive solo il JSON
        let json = serde_json::to_string(&to_fmt(&e.drawing, &e.meta, e.chart.as_ref())).map_err(std::io::Error::other)?;
        let tmp = self.dir.join(format!(".{id}.tmp"));
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, self.path(id))
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
            let chart = chart_file(&self.dir, id);
            if chart.exists() {
                std::fs::rename(&chart, chart_file(&tdir, id))?;
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
            let chart = chart_file(&tdir, id);
            if chart.exists() {
                std::fs::rename(&chart, chart_file(&self.dir, id))?;
            }
            if let Ok(f) = serde_json::from_str::<PageFmt>(&std::fs::read_to_string(&dst)?) {
                let e = from_fmt(f, &self.dir, id);
                self.entries.push(e);
                n += 1;
            }
        }
        self.sort();
        Ok(n)
    }

    /// Importa le copie grezze del pad (exports/pad-raw/*.bin) non ancora in archivio.
    pub fn import_raw_dir(&mut self, dir: &Path, width: f32, height: f32) -> usize {
        let mut known: std::collections::HashSet<String> =
            self.entries.iter().filter_map(|e| e.meta.raw.clone()).collect();
        // quelle già passate di qui (anche se poi le hai cestinate o erano doppie)
        known.extend(raw_seen(dir));
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
            meta.raw = Some(name.clone());
            if self.add(&d, meta).is_ok() {
                mark_raw_seen(dir, &[name]);
                n += 1;
            }
        }
        n
    }
}

/// Elenco delle copie grezze del pad già passate nell'archivio
/// (`exports/pad-raw/.importati`): una pagina cestinata non torna.
const RAW_SEEN: &str = ".importati";

pub fn raw_seen(dir: &Path) -> std::collections::HashSet<String> {
    std::fs::read_to_string(dir.join(RAW_SEEN)).unwrap_or_default().lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect()
}

/// C'è già l'elenco? (Prima di questa versione non c'era.)
pub fn raw_seen_exists(dir: &Path) -> bool {
    dir.join(RAW_SEEN).exists()
}

/// Segna le copie grezze come passate nell'archivio.
pub fn mark_raw_seen(dir: &Path, names: &[String]) {
    use std::io::Write;
    if names.is_empty() && raw_seen_exists(dir) {
        return;
    }
    let _ = std::fs::create_dir_all(dir);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join(RAW_SEEN)) {
        for n in names {
            let _ = writeln!(f, "{n}");
        }
    }
}

/// Tutte le copie grezze nella cartella (per partire con l'elenco pieno).
pub fn raw_files(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .filter_map(|e| e.file_name().to_str().map(String::from))
                .filter(|n| n.ends_with(".bin"))
                .collect()
        })
        .unwrap_or_default()
}

/// Intestazione del gruppo per giorno: "Oggi", "Ieri", "18 ottobre 2026".
pub fn day_label(created: Option<NaiveDateTime>) -> String {
    use crate::i18n::t;
    let Some(t0) = created else {
        return t("Senza data").into();
    };
    let today = Local::now().date_naive();
    let d = t0.date();
    if d == today {
        t("Oggi").into()
    } else if Some(d) == today.pred_opt() {
        t("Ieri").into()
    } else {
        crate::i18n::date_long(d)
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

    /// Il grafico dell'AI sul foglio va nell'archivio con la pagina, e nel
    /// cestino e ritorno con lei.
    #[test]
    fn pagina_col_grafico() {
        let dir = tmpdir("grafico");
        let mut lib = Library::open_in(&dir);
        let t = NaiveDateTime::parse_from_str("2026-10-21 12:00:00", TIME_FMT).unwrap();
        let bg = PageBg { png: Arc::new(b"\x89PNG finto".to_vec()), rect: [0.2, 0.3, 0.5, 0.4], key: "chart:x".into() };
        let id = lib.add_with_chart(&drawing(), Meta::new(t, SOURCE_BOARD), Some(&bg)).unwrap();
        let senza = lib.add(&drawing(), Meta::new(t, SOURCE_BOARD)).unwrap();
        lib.rename(&id, "Sistema").unwrap();
        let lib2 = Library::open_in(&dir);
        let c = lib2.get(&id).unwrap().chart.as_ref().expect("il grafico torna");
        assert_eq!(c.rect, [0.2, 0.3, 0.5, 0.4]);
        assert_eq!(c.png.as_slice(), b"\x89PNG finto");
        assert!(c.is_chart());
        assert!(lib2.get(&senza).unwrap().chart.is_none());
        let mut lib3 = lib2;
        lib3.trash(std::slice::from_ref(&id)).unwrap();
        assert!(!chart_file(&dir, &id).exists());
        lib3.restore(std::slice::from_ref(&id)).unwrap();
        assert!(Library::open_in(&dir).get(&id).unwrap().chart.is_some());
        // riscritta senza grafico e con un tratto in più: titolo e data restano
        let mut d = drawing();
        d.strokes.push(d.strokes[0].clone());
        lib3.update(&id, &d, None).unwrap();
        let lib4 = Library::open_in(&dir);
        let e = lib4.get(&id).unwrap();
        assert_eq!((e.drawing.strokes.len(), e.meta.display_title().as_str(), e.chart.is_none()), (2, "Sistema", true));
        assert!(!chart_file(&dir, &id).exists());
        assert_eq!(lib4.entries.len(), 2);
        assert!(lib3.update("nessuna", &d, None).is_err());
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
