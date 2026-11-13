//! La bella copia e «Disponi sulle righe» sul telefono: lo stesso codice
//! della desktop (`src/inbella.rs`, `src/righe.rs`), con l'AI del telefono
//! (`ai.rs`) per leggere la scrittura.
//!
//! L'app sceglie la zona (col dito) e mostra l'anteprima: la pagina com'è
//! dopo, sotto un velo bianco nella zona cambiata. «Accetta» rifà il lavoro
//! sul foglio com'è in quel momento e lo mette sulla lavagna con una sola
//! voce di annullamento.
//!
//! Sul telefono la bella copia usa sempre il carattere chiaro (la scrittura
//! addestrata e la rete «mano» stanno sul computer). All'AI va solo
//! l'immagine ritagliata della zona, e solo quando si chiede la bella copia.

use crate::ai::{AiListener, AiProvider};
use crate::board::{to_page_stroke, BoardModel};
use crate::pages::PageStroke;
use crate::rigatura::Paper;
use crate::righe::{Arranged, Problem, Sheet};
use crate::scene::PageGeom;
use crate::stroke::Stroke;
use crate::tutor::BoardView;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Una zona della pagina (frazioni della pagina come si vede).
#[derive(uniffi::Record, Clone, Copy, Debug, PartialEq)]
pub struct Zone {
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
}

impl Zone {
    fn rect(self) -> [f32; 4] {
        [self.u0.min(self.u1), self.v0.min(self.v1), self.u0.max(self.u1), self.v0.max(self.v1)]
    }
}

/// Il foglio: la carta (codice delle impostazioni), il margine rosso, una
/// riga ogni quanti quadretti (`None` = quello proposto).
#[derive(uniffi::Record, Clone, Debug)]
pub struct SheetSpec {
    pub paper: String,
    pub margin: bool,
    pub every: Option<u32>,
}

impl SheetSpec {
    fn sheet(&self) -> Sheet {
        Sheet { paper: Paper::from_code(&self.paper), margin: self.margin, every: self.every.map(|e| e.clamp(1, 3)) }
    }
}

/// Perché «Disponi sulle righe» non si è fatto.
#[derive(uniffi::Enum, Clone, Debug, PartialEq, Eq)]
pub enum NeatIssue {
    /// Nella zona non c'è scrittura.
    Nothing,
    /// La carta non ha righe (bianca, pentagramma).
    Unsupported,
    /// Le righe non bastano: ne stanno `fit` su `total`.
    NoRoom { fit: u32, total: u32 },
}

fn issue(p: Problem) -> NeatIssue {
    match p {
        Problem::Nothing => NeatIssue::Nothing,
        Problem::Unsupported => NeatIssue::Unsupported,
        Problem::NoRoom { fit, total } => NeatIssue::NoRoom { fit: fit as u32, total: total as u32 },
    }
}

#[derive(uniffi::Error, thiserror::Error, Debug)]
pub enum NeatError {
    /// «Disponi sulle righe» non si fa (vedi `NeatIssue`).
    #[error("sulle righe no: {issue:?}")]
    Rules { issue: NeatIssue },
    /// La bella copia non ha trovato niente da fare nella zona.
    #[error("niente da mettere in bella")]
    NothingToDo,
    #[error("{reason}")]
    Ai { reason: String },
    #[error("interrotto")]
    Cancelled,
}

/// L'anteprima (o il risultato): tutta la pagina com'è dopo, dove è cambiata
/// e com'è andata.
#[derive(uniffi::Record, Clone, Debug)]
pub struct BoardPreview {
    pub strokes: Vec<PageStroke>,
    pub veil: Zone,
    /// Righe disposte o riscritte in bella, forme rifatte, righe rimpicciolite.
    pub lines: u32,
    pub shapes: u32,
    pub shrunk: u32,
    /// Sui quadretti: una riga ogni quanti quadretti.
    pub every: Option<u32>,
    /// Chi ha letto (bella copia).
    pub by: String,
    /// La bella copia sulle righe non si è potuta fare (ma la bella copia sì).
    pub rules_issue: Option<NeatIssue>,
}

/// La larghezza più stretta e più larga che si può scegliere per la bella copia.
#[uniffi::export]
pub fn fair_width_min() -> f32 {
    crate::inbella::WIDTH.0
}

#[uniffi::export]
pub fn fair_width_max() -> f32 {
    crate::inbella::WIDTH.1
}

/// Dove tenere le impronte dei tratti già in bella (la cartella dell'app).
#[uniffi::export]
pub fn neat_set_dir(dir: String) {
    crate::inbella::set_data_dir(dir);
}

/// La carta ha righe su cui scrivere (non è bianca né un pentagramma)?
#[uniffi::export]
pub fn paper_has_rows(paper: String) -> bool {
    crate::rigatura::writing_rows(Paper::from_code(&paper), 21.0, 29.7) != crate::rigatura::Rows::Unsupported
}

/// La carta è a quadretti (si sceglie quanti quadretti per riga)?
#[uniffi::export]
pub fn paper_is_grid(paper: String) -> bool {
    matches!(crate::rigatura::writing_rows(Paper::from_code(&paper), 21.0, 29.7), crate::rigatura::Rows::Grid { .. })
}

fn zone_of(zone: Option<Zone>) -> Option<[f32; 4]> {
    zone.map(Zone::rect).filter(|r| *r != [0.0, 0.0, 1.0, 1.0])
}

fn to_zone(r: [f32; 4]) -> Zone {
    Zone { u0: r[0], v0: r[1], u1: r[2], v1: r[3] }
}

/// Sui quadretti: quanti quadretti per riga perché la scrittura della zona ci stia.
#[uniffi::export]
pub fn board_suggest_every(board: Arc<BoardModel>, view: BoardView, zone: Option<Zone>, paper: String) -> Option<u32> {
    let (strokes, _, _) = board.snapshot();
    crate::righe::suggest_every(&strokes, &view.geom(), zone_of(zone), Paper::from_code(&paper))
}

fn arranged_preview(a: Arranged, veil: [f32; 4]) -> BoardPreview {
    BoardPreview {
        strokes: a.strokes.iter().map(to_page_stroke).collect(),
        veil: to_zone(veil),
        lines: a.lines as u32,
        shapes: 0,
        shrunk: a.shrunk as u32,
        every: a.every,
        by: String::new(),
        rules_issue: None,
    }
}

/// «Disponi sulle righe» sulla zona (`None` = tutta la pagina): l'anteprima,
/// o con `commit` direttamente sulla lavagna (una voce di annullamento).
#[uniffi::export]
pub fn board_arrange(board: Arc<BoardModel>, view: BoardView, zone: Option<Zone>, sheet: SheetSpec, commit: bool) -> Result<BoardPreview, NeatError> {
    let (strokes, _, _) = board.snapshot();
    let z = zone_of(zone);
    let a = crate::righe::arrange(&strokes, &view.geom(), z, sheet.sheet()).map_err(|p| NeatError::Rules { issue: issue(p) })?;
    let veil = match z {
        Some(r) => crate::boardread::union(r, a.area),
        None => [0.0, 0.0, 1.0, 1.0],
    };
    if commit {
        board.replace_all(a.strokes.clone());
    }
    Ok(arranged_preview(a, veil))
}

/// Una bella copia sul telefono: si legge la zona (con l'AI), poi si guarda
/// l'anteprima e si accetta.
#[derive(uniffi::Object)]
pub struct FairJob {
    board: Arc<BoardModel>,
    view: BoardView,
    rect: [f32; 4],
    texts: Mutex<HashMap<u64, String>>,
    by: Mutex<String>,
    /// Quanto larghe le lettere (volte il carattere; `None` = come la tua scrittura).
    width: Mutex<Option<f32>>,
}

impl FairJob {
    /// I tratti della zona e la pagina senza di loro.
    fn compute(&self, on_rules: bool, sheet: &SheetSpec) -> Result<(Vec<Stroke>, BoardPreview), NeatError> {
        let (strokes, _, _) = self.board.snapshot();
        let g: PageGeom = self.view.geom();
        let idx = crate::inbella::select(&strokes, &g, self.rect);
        let sub: Vec<Stroke> = idx.iter().map(|&i| strokes[i].clone()).collect();
        let texts = self.texts.lock().unwrap().clone();
        let width = *self.width.lock().unwrap();
        let (new_sub, done) = crate::inbella::apply_region_w(&sub, &texts, &g, self.rect[2], width).ok_or(NeatError::NothingToDo)?;
        let mut page: Vec<Stroke> = strokes.iter().enumerate().filter(|(i, _)| !idx.contains(i)).map(|(_, s)| s.clone()).collect();
        let first = page.len();
        page.extend(new_sub);
        let whole = self.rect == [0.0, 0.0, 1.0, 1.0];
        let zone = (!whole).then_some(self.rect);
        let mut veil = self.rect;
        let (mut shrunk, mut every, mut rules_issue) = (0, None, None);
        if on_rules {
            let movable: Vec<usize> = (first..page.len()).collect();
            match crate::righe::arrange_only(&page, &g, &movable, zone, sheet.sheet()) {
                Ok(a) => {
                    if !whole {
                        veil = crate::boardread::union(self.rect, a.area);
                    }
                    shrunk = a.shrunk as u32;
                    every = a.every;
                    page = a.strokes;
                }
                Err(p) => rules_issue = Some(issue(p)),
            }
        }
        let preview = BoardPreview {
            strokes: page.iter().map(to_page_stroke).collect(),
            veil: to_zone(veil),
            lines: done.lines as u32,
            shapes: done.shapes as u32,
            shrunk,
            every,
            by: self.by.lock().unwrap().clone(),
            rules_issue,
        };
        Ok((page, preview))
    }
}

#[uniffi::export]
impl FairJob {
    /// La bella copia della zona (`None` = tutta la pagina).
    #[uniffi::constructor]
    pub fn new(board: Arc<BoardModel>, view: BoardView, zone: Option<Zone>) -> Arc<Self> {
        let rect = zone.map_or([0.0, 0.0, 1.0, 1.0], Zone::rect);
        Arc::new(Self { board, view, rect, texts: Mutex::new(HashMap::new()), by: Mutex::new(String::new()), width: Mutex::new(None) })
    }

    /// Quanto larghe le lettere della bella copia, volte il carattere (da
    /// `fair_width_min` a `fair_width_max`; `None` o 0 = come la tua
    /// scrittura). Vale dalla prossima anteprima.
    pub fn set_width(&self, width: Option<f32>) {
        *self.width.lock().unwrap() = width.filter(|w| *w > 0.0 && w.is_finite());
    }

    /// Nella zona c'è scrittura da far leggere all'AI? (Se no, la bella copia
    /// rifà solo i disegni, senza AI.)
    pub fn needs_ai(&self) -> bool {
        let (strokes, _, _) = self.board.snapshot();
        let g = self.view.geom();
        let idx = crate::inbella::select(&strokes, &g, self.rect);
        let sub: Vec<Stroke> = idx.iter().map(|&i| strokes[i].clone()).collect();
        !sub.is_empty() && !crate::inbella::pending(&sub, &g).is_empty()
    }

    /// Fa leggere la zona all'AI (bloccante: in un thread). Torna chi ha letto.
    pub fn read(&self, providers: Vec<AiProvider>, listener: Arc<dyn AiListener>) -> Result<String, NeatError> {
        let (strokes, _, _) = self.board.snapshot();
        let g = self.view.geom();
        let idx = crate::inbella::select(&strokes, &g, self.rect);
        let sub: Vec<Stroke> = idx.iter().map(|&i| strokes[i].clone()).collect();
        let ask = crate::inbella::ask_region(&sub, self.view.pad_w.max(1.0), self.view.pad_h.max(1.0), self.view.view(), self.rect).map_err(|e| NeatError::Ai { reason: format!("{e:#}") })?;
        let Some(ask) = ask else { return Ok(String::new()) };
        let reply = crate::ai::read_image(&providers, &ask.prompt, ask.png.clone(), listener.as_ref()).map_err(|e| match e {
            crate::ai::AiError::Cancelled => NeatError::Cancelled,
            crate::ai::AiError::Failed { reason } => NeatError::Ai { reason },
        })?;
        let texts = crate::inbella::answer(&ask, &reply.text).map_err(|e| NeatError::Ai { reason: format!("{e:#}") })?;
        *self.texts.lock().unwrap() = texts;
        *self.by.lock().unwrap() = reply.provider.clone();
        Ok(reply.provider)
    }

    /// L'anteprima sul foglio com'è adesso.
    pub fn preview(&self, on_rules: bool, sheet: SheetSpec) -> Result<BoardPreview, NeatError> {
        Ok(self.compute(on_rules, &sheet)?.1)
    }

    /// La bella copia sulla lavagna (una voce di annullamento).
    pub fn commit(&self, on_rules: bool, sheet: SheetSpec) -> Result<BoardPreview, NeatError> {
        let (page, preview) = self.compute(on_rules, &sheet)?;
        self.board.replace_all(page);
        Ok(preview)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{Phase, PenPoint};

    fn view() -> BoardView {
        BoardView { quarter: 0, pad_w: 21000.0, pad_h: 29700.0 }
    }

    /// Una riga di archetti storta, con la base a `y`.
    fn write(board: &BoardModel, x: f32, y: f32, rise: f32) {
        for k in 0..5 {
            let pts: Vec<PenPoint> = (0..20)
                .map(|t| {
                    let f = t as f32 / 19.0;
                    let px = x + k as f32 * 600.0 + f * 450.0;
                    let phase = if t == 0 { Phase::Down } else if t == 19 { Phase::Up } else { Phase::Move };
                    PenPoint { x: px, y: y - (px - x) * rise / 1000.0 - (f * 3.14).sin() * 450.0, pressure: 0.7, phase }
                })
                .collect();
            board.pen(pts);
        }
    }

    fn sheet() -> SheetSpec {
        SheetSpec { paper: "righe".into(), margin: true, every: None }
    }

    #[test]
    fn sulle_righe_e_annulla() {
        let board = BoardModel::new();
        write(&board, 5000.0, 5300.0, 50.0);
        write(&board, 6000.0, 9100.0, 0.0);
        let before = board.strokes();
        // l'anteprima non tocca la lavagna
        let p = board_arrange(board.clone(), view(), None, sheet(), false).unwrap();
        assert_eq!(p.lines, 2);
        assert_eq!(board.strokes().len(), before.len());
        assert_eq!(board.strokes()[0].points[0].x, before[0].points[0].x);
        // accettata: tutte e due dal margine (2,7 cm), su righe vicine
        let p = board_arrange(board.clone(), view(), None, sheet(), true).unwrap();
        let after = board.strokes();
        let left = |r: std::ops::Range<usize>| after[r].iter().flat_map(|s| s.points.iter().map(|q| q.x)).fold(f32::MAX, f32::min);
        assert!((left(0..5) - 2700.0).abs() < 50.0 && (left(5..10) - 2700.0).abs() < 50.0);
        assert_eq!(p.strokes.len(), after.len());
        // un annulla solo riporta com'era
        assert!(board.undo());
        assert_eq!(board.strokes()[0].points[0].x, before[0].points[0].x);
        // carta bianca: no
        let e = board_arrange(board.clone(), view(), None, SheetSpec { paper: "bianco".into(), ..sheet() }, false).unwrap_err();
        assert!(matches!(e, NeatError::Rules { issue: NeatIssue::Unsupported }));
        assert!(paper_has_rows("quadretti5".into()) && paper_is_grid("quadretti5".into()) && !paper_has_rows("pentagramma".into()));
        assert_eq!(board_suggest_every(board.clone(), view(), None, "quadretti5".into()), Some(2));
    }

    struct Quiet;
    impl AiListener for Quiet {
        fn on_event(&self, _: crate::ai::AiEvent) -> bool {
            true
        }
    }

    /// Un server finto (OpenAI-compatibile) che risponde sempre `reply`.
    fn server(reply: &str) -> String {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/v1", l.local_addr().unwrap());
        let reply = reply.to_string();
        std::thread::spawn(move || {
            use std::io::{Read, Write};
            for s in l.incoming().flatten().take(8) {
                let mut s = s;
                let mut buf = vec![0u8; 1 << 22];
                let mut got = Vec::new();
                loop {
                    let n = s.read(&mut buf).unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    got.extend_from_slice(&buf[..n]);
                    let t = String::from_utf8_lossy(&got);
                    if let Some(h) = t.find("\r\n\r\n") {
                        let len = t[..h].lines().find_map(|l| l.to_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap_or(0))).unwrap_or(0);
                        if got.len() >= h + 4 + len {
                            break;
                        }
                    }
                }
                let t = String::from_utf8_lossy(&got);
                let body = if t.starts_with("GET") {
                    serde_json::json!({ "data": [{ "id": "vista" }] }).to_string()
                } else {
                    serde_json::json!({ "choices": [{ "message": { "role": "assistant", "content": reply } }] }).to_string()
                };
                let _ = write!(s, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
            }
        });
        url
    }

    #[test]
    fn bella_copia_sul_telefono() {
        let dir = std::env::temp_dir().join(format!("osl-neat-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        neat_set_dir(dir.to_string_lossy().into_owned());
        let board = BoardModel::new();
        write(&board, 5000.0, 6000.0, 0.0);
        let n = board.strokes().len();
        let url = server("{\"r1\": \"ciao\"}");
        let prov = AiProvider { name: "Finto".into(), url, model: "vista".into(), key: String::new(), vision: true, enabled: true, no_temperature: false, max_tokens: None, timeout_s: 20 };
        let job = FairJob::new(board.clone(), view(), None);
        assert!(job.needs_ai());
        assert_eq!(job.read(vec![prov], Arc::new(Quiet)).unwrap(), "Finto");
        let p = job.preview(false, sheet()).unwrap();
        assert_eq!(p.lines, 1);
        assert_eq!(board.strokes().len(), n, "l'anteprima non tocca la lavagna");
        // sulle righe: la bella copia comincia dal margine
        let p = job.commit(true, sheet()).unwrap();
        assert!(p.rules_issue.is_none(), "{:?}", p.rules_issue);
        let after = board.strokes();
        let left = after.iter().flat_map(|s| s.points.iter().map(|q| q.x)).fold(f32::MAX, f32::min);
        assert!((left - 2700.0).abs() < 60.0, "{left}");
        assert!(board.undo());
        assert_eq!(board.strokes().len(), n);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
