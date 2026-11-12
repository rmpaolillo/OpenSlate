//! I documenti da firmare sul disco, uguali sulla desktop e sul telefono
//! (`mobile/core` include questo file): così si sincronizzano così come sono.
//!
//! Ogni documento sta in `documents/<id>/`:
//! - `doc.json`: titolo, data, i tratti di ogni pagina e la geometria in cui
//!   sono scritti (`pad`);
//! - `source.pdf`: le pagine come si vedono (mai modificato);
//! - `source.<ext>`: il file di partenza, se non è un PDF (Word, foto, SVG…).
//!
//! I tratti sono in unità del pad, visti con una rotazione: la desktop e il
//! telefono possono girare il foglio in modo diverso. Per questo `doc.json`
//! dice in quale geometria sono scritti, e chi li apre li porta nella sua
//! (`convert`): la firma resta nello stesso punto della pagina.

use crate::pagegeom::{fit_rect, PageGeom, View};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const DOC_DIR: &str = "documents";
/// Dove vanno i documenti tolti (dentro `documents/`).
pub const TRASH: &str = "cestino";

/// Un tratto nel formato dei file (lo stesso dell'archivio).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct InkFmt {
    /// Colore "#rrggbb".
    pub c: String,
    /// Spessore relativo.
    pub w: f32,
    /// Punti [x, y, pressione] in unità del pad.
    pub p: Vec<[f32; 3]>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub h: bool,
    /// Timbro (i punti sono i quattro angoli).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub t: Option<crate::stamp::StampSpec>,
}

/// I tratti di un file → tratti da disegnare.
pub fn ink_to_strokes(v: Vec<InkFmt>) -> Vec<crate::stroke::Stroke> {
    use crate::stroke::{PenEvent, PenPhase, Stroke};
    v.into_iter()
        // un tratto senza punti (file rovinato o scritto a mano) non serve
        .filter(|s| !s.p.is_empty())
        .map(|s| {
            let n = s.p.len();
            let mut st = Stroke::new(hex_rgb(&s.c).unwrap_or([20, 20, 20]), s.w);
            st.highlight = s.h;
            st.stamp = s.t.map(std::sync::Arc::new);
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

/// Tratti → formato dei file.
pub fn strokes_to_ink(strokes: &[crate::stroke::Stroke]) -> Vec<InkFmt> {
    strokes
        .iter()
        .filter(|s| !s.points.is_empty())
        .map(|s| InkFmt {
            c: format!("#{:02x}{:02x}{:02x}", s.color[0], s.color[1], s.color[2]),
            w: s.width,
            p: s.points.iter().map(|p| [p.x, p.y, p.pressure]).collect(),
            h: s.highlight,
            t: s.stamp.as_deref().cloned(),
        })
        .collect()
}

fn hex_rgb(s: &str) -> Option<[u8; 3]> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}

/// La geometria in cui sono scritti i tratti: misure del pad e vista.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct PadGeom {
    pub w: f32,
    pub h: f32,
    #[serde(default)]
    pub rot: i32,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub fx: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub fy: bool,
}

impl PadGeom {
    pub fn new(view: View, w: f32, h: f32) -> PadGeom {
        PadGeom { w, h, rot: view.rot.rem_euclid(4), fx: view.flip_x, fy: view.flip_y }
    }

    pub fn view(&self) -> View {
        View { rot: self.rot, flip_x: self.fx, flip_y: self.fy }
    }

    fn page(&self) -> PageGeom {
        PageGeom::new(self.view(), self.w.max(1.0), self.h.max(1.0))
    }

    fn same(&self, o: &PadGeom) -> bool {
        self.rot.rem_euclid(4) == o.rot.rem_euclid(4) && self.fx == o.fx && self.fy == o.fy && (self.w - o.w).abs() < 0.5 && (self.h - o.h).abs() < 0.5
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DocFmt {
    pub version: u32,
    pub title: String,
    pub created: String,
    #[serde(default)]
    pub current: usize,
    /// Il file di partenza accanto a `source.pdf` (None = era un PDF).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original: Option<String>,
    pub pages: Vec<Vec<InkFmt>>,
    /// Inchiostro delle pagine che non ci sono più.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra: Vec<Vec<InkFmt>>,
    /// La geometria dei tratti (None = documenti di prima: quella di chi apre).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pad: Option<PadGeom>,
}

pub fn read(dir: &Path) -> std::io::Result<DocFmt> {
    serde_json::from_str(&std::fs::read_to_string(dir.join("doc.json"))?).map_err(std::io::Error::other)
}

/// Scrive `doc.json` passando da un file temporaneo.
pub fn write(dir: &Path, f: &DocFmt) -> std::io::Result<()> {
    let tmp = dir.join(".doc.json.tmp");
    std::fs::write(&tmp, serde_json::to_string(f).map_err(std::io::Error::other)?)?;
    std::fs::rename(&tmp, dir.join("doc.json"))
}

/// Un pezzo di nome sicuro per la cartella.
pub fn safe_id(stem: &str) -> String {
    let s: String = stem
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(40)
        .collect();
    if s.is_empty() { "documento".into() } else { s }
}

/// Una cartella nuova e libera per il documento `stem`: `<data-ora>-<nome>`.
pub fn new_id(root: &Path, stem: &str, now: &chrono::DateTime<chrono::Local>) -> String {
    let base = format!("{}-{}", now.format("%Y%m%d-%H%M%S"), safe_id(stem));
    let mut id = base.clone();
    let mut n = 2;
    while root.join(&id).exists() || root.join(TRASH).join(&id).exists() {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

/// Riga dell'elenco dei documenti.
#[derive(Clone, Debug)]
pub struct DocSummary {
    pub id: String,
    pub title: String,
    pub created: String,
}

/// I documenti in `root`, dal più recente.
pub fn list(root: &Path) -> Vec<DocSummary> {
    let mut v: Vec<DocSummary> = std::fs::read_dir(root)
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.file_name() != TRASH)
                .filter_map(|e| {
                    let id = e.file_name().to_str()?.to_string();
                    let f = read(&e.path()).ok()?;
                    Some(DocSummary { id, title: f.title, created: f.created })
                })
                .collect()
        })
        .unwrap_or_default();
    v.sort_by(|a, b| b.created.cmp(&a.created).then(b.id.cmp(&a.id)));
    v
}

/// Sposta il documento nel cestino (`documents/cestino/<id>`).
pub fn trash(root: &Path, id: &str) -> std::io::Result<PathBuf> {
    if id.is_empty() || id == TRASH || id.contains(['/', '\\']) || id.starts_with('.') {
        return Err(std::io::Error::other("documento non valido"));
    }
    let t = root.join(TRASH);
    std::fs::create_dir_all(&t)?;
    let mut dst = t.join(id);
    let mut n = 2;
    while dst.exists() {
        dst = t.join(format!("{id}-{n}"));
        n += 1;
    }
    std::fs::rename(root.join(id), &dst)?;
    Ok(dst)
}

/// Porta i tratti dalla geometria `from` a `to`, pagina per pagina: ogni
/// punto resta nello stesso posto della pagina del documento (`sizes` le
/// misure delle pagine, in punti), e lo spessore segue.
pub fn convert(pages: &mut [Vec<InkFmt>], sizes: &[(f32, f32)], from: PadGeom, to: PadGeom) {
    if from.same(&to) {
        return;
    }
    let (gf, gt) = (from.page(), to.page());
    for (i, page) in pages.iter_mut().enumerate() {
        let (dw, dh) = sizes.get(i).or(sizes.last()).copied().unwrap_or((595.0, 842.0));
        let [fx, fy, fw, fh] = fit_rect(dw, dh, gf.w, gf.h);
        let [tx, ty, tw, th] = fit_rect(dw, dh, gt.w, gt.h);
        for s in page.iter_mut() {
            for p in s.p.iter_mut() {
                let (u, v) = gf.unit(p[0], p[1]);
                let (du, dv) = ((u - fx) / fw, (v - fy) / fh);
                let (x, y) = gt.pad(tx + du * tw, ty + dv * th);
                p[0] = x;
                p[1] = y;
            }
            // lo spessore va in proporzione all'altezza della pagina del documento
            // sul foglio (come lo legge chi salva la copia firmata)
            s.w *= th / fh.max(1e-6);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ink(pts: &[(f32, f32)]) -> InkFmt {
        InkFmt { c: "#000000".into(), w: 2.0, p: pts.iter().map(|&(x, y)| [x, y, 0.5]).collect(), h: false, t: None }
    }

    #[test]
    fn la_firma_resta_nello_stesso_punto_della_pagina() {
        let a4 = [(595.0, 842.0)];
        let desk = PadGeom::new(View::default(), 21600.0, 14800.0);
        let phone = PadGeom::new(View { rot: 3, flip_x: false, flip_y: false }, 21600.0, 14800.0);
        let mut pages = vec![vec![ink(&[(10800.0, 7400.0), (12000.0, 8000.0)])]];
        let orig = pages.clone();
        // dove sta sulla pagina del documento, prima
        let where_on_page = |pages: &[Vec<InkFmt>], g: PadGeom| {
            let pg = g.page();
            let [rx, ry, rw, rh] = fit_rect(595.0, 842.0, pg.w, pg.h);
            pages[0][0].p.iter().map(|p| {
                let (u, v) = pg.unit(p[0], p[1]);
                (((u - rx) / rw * 1000.0).round(), ((v - ry) / rh * 1000.0).round())
            }).collect::<Vec<_>>()
        };
        let before = where_on_page(&pages, desk);
        convert(&mut pages, &a4, desk, phone);
        assert_ne!(pages, orig, "le coordinate del pad cambiano");
        assert_eq!(where_on_page(&pages, phone), before, "ma il punto sulla pagina no");
        // e lo spessore, in proporzione all'altezza della pagina vista, nemmeno
        let h_page = |g: PadGeom| fit_rect(595.0, 842.0, g.page().w, g.page().h)[3];
        let frac = |w: f32, g: PadGeom| w / 800.0 / h_page(g);
        assert!((frac(pages[0][0].w, phone) - frac(2.0, desk)).abs() < 1e-6);
        // andata e ritorno
        convert(&mut pages, &a4, phone, desk);
        for (a, b) in pages[0][0].p.iter().zip(&orig[0][0].p) {
            assert!((a[0] - b[0]).abs() < 0.5 && (a[1] - b[1]).abs() < 0.5);
        }
        assert!((pages[0][0].w - 2.0).abs() < 1e-4);
    }

    #[test]
    fn elenco_nomi_e_cestino() {
        let root = std::env::temp_dir().join(format!("openslate-docstore-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let now = chrono::Local::now();
        let id = new_id(&root, "Contratto 2026/3", &now);
        assert!(id.ends_with("-Contratto-2026-3"));
        let dir = root.join(&id);
        std::fs::create_dir_all(&dir).unwrap();
        let f = DocFmt { version: 1, title: "Contratto".into(), created: "2026-11-12 09:00:00".into(), current: 0, original: None, pages: vec![vec![]], extra: vec![], pad: None };
        write(&dir, &f).unwrap();
        assert_eq!(list(&root).len(), 1);
        assert_ne!(new_id(&root, "Contratto 2026/3", &now), id);
        trash(&root, &id).unwrap();
        assert!(list(&root).is_empty());
        assert!(root.join(TRASH).join(&id).join("doc.json").exists());
        assert!(trash(&root, "../x").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }
}
