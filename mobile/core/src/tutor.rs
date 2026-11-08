//! Il tutor sulla lavagna del telefono: gli strumenti `lavagna_leggi` e
//! `lavagna_annota` della desktop (`src/boardagent.rs`), con la lettura del
//! foglio (`src/boardread.rs`), i segni e la scrittura a mano
//! (`src/boardmarks.rs`, `src/handwrite.rs`): gli stessi file. Tutto quello
//! che aggiunge è una sola voce di annullamento sulla lavagna (↶).

use crate::ai::{ToolOut, ToolRunner};
use crate::board::BoardModel;
use crate::boardread::{self, Reading};
use crate::scene::{Page, PageGeom, View};
use crate::stroke::{strokes_sig, Drawing, PenEvent, PenPhase, Stroke};
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::sync::{Arc, Mutex};

/// Come si vede il foglio sul telefono.
#[derive(uniffi::Record, Clone, Copy, Debug)]
pub struct BoardView {
    /// Quarti di giro (come «Ruota» dell'app).
    pub quarter: u32,
    /// Area attiva del pad (unità pad).
    pub pad_w: f32,
    pub pad_h: f32,
}

impl BoardView {
    /// La rotazione della desktop che dà lo stesso foglio dell'app: l'app gira
    /// in senso orario, `View` in senso antiorario.
    pub(crate) fn view(&self) -> View {
        View { rot: ((4 - self.quarter % 4) % 4) as i32, flip_x: false, flip_y: false }
    }

    pub(crate) fn geom(&self) -> PageGeom {
        PageGeom::new(self.view(), self.pad_w.max(1.0), self.pad_h.max(1.0))
    }
}

/// Aggiunta alle istruzioni della chat (come `boardagent::system_note`, senza
/// grafici e modifiche, che sul telefono non ci sono ancora).
pub const BOARD_NOTE: &str = "\n\nAccanto alla chat c'è il foglio della lavagna (scritto a mano col pennino) e lo puoi leggere e annotare: \
     lavagna_leggi per vedere righe e parole coi loro nomi, poi lavagna_annota per evidenziare, sottolineare, cerchiare, \
     barrare, mettere frecce, spunte o croci e scrivere a mano (correzioni, risultati, note accanto alle righe). \
     Di tua iniziativa non cancelli e non sposti niente: aggiungi soltanto. \
     Non dire che non puoi: gli strumenti ci sono. L'utente vede subito il risultato e lo può annullare.";

pub struct Tutor {
    pub board: Arc<BoardModel>,
    pub view: BoardView,
    /// L'ultima lettura: i nomi (`r2p3`) che il modello ha visto.
    last: Mutex<Option<Reading>>,
}

impl Tutor {
    pub fn new(board: Arc<BoardModel>, view: BoardView) -> Self {
        Self { board, view, last: Mutex::new(None) }
    }

    fn drawing(&self, strokes: Vec<Stroke>) -> Drawing {
        Drawing { strokes, width: self.view.pad_w, height: self.view.pad_h }
    }

    /// Il foglio letto, in parole e in immagine (coi nomi).
    fn read(&self) -> Result<(String, Vec<Vec<u8>>)> {
        let strokes = self.board.snapshot().0;
        let g = self.view.geom();
        let reading = boardread::read(&strokes, &g, None);
        let text = boardread::describe(&reading);
        let page = Page::from_drawing(&self.drawing(strokes), self.view.view());
        let images = if reading.is_empty() { Vec::new() } else { boardread::labelled_png(&page, &reading).ok().into_iter().collect() };
        *self.last.lock().unwrap() = Some(reading);
        Ok((text, images))
    }

    /// L'ultima lettura vista dal modello, se da allora ci sono solo tratti in più.
    fn reading_for(&self, strokes: &[Stroke], g: &PageGeom) -> Reading {
        if let Some(r) = self.last.lock().unwrap().take() {
            if strokes.len() >= r.n && strokes_sig(&strokes[..r.n]) == r.sig {
                return r;
            }
        }
        boardread::read(strokes, g, None)
    }

    /// I segni chiesti, tutti insieme (come `boardagent::annotate`).
    fn annotate(&self, args: &Value) -> Result<String> {
        let list: Vec<Value> = match &args["segni"] {
            Value::Array(a) => a.clone(),
            Value::Object(_) => vec![args["segni"].clone()],
            _ if args.get("tipo").is_some() => vec![args.clone()],
            _ => bail!("manca «segni» (la lista dei segni)"),
        };
        if list.is_empty() {
            bail!("«segni» è vuota");
        }
        if list.len() > 60 {
            bail!("troppi segni insieme (al massimo 60)");
        }
        let (strokes, pen_width, highlighter) = self.board.snapshot();
        let g = self.view.geom();
        let reading = self.reading_for(&strokes, &g);
        let ctx = crate::boardmarks::Ctx::new(&reading, g.w / g.h, pen_width, highlighter);
        let mut add: Vec<Stroke> = Vec::new();
        let mut done = Vec::new();
        for (i, m) in list.iter().enumerate() {
            let (marks, what) = crate::boardmarks::build(&ctx, m, i).with_context(|| format!("segno {}", i + 1))?;
            done.push(what);
            for mk in marks {
                let mut st = if mk.highlight { Stroke::highlighter(mk.color, mk.width) } else { Stroke::new(mk.color, mk.width) };
                let last = mk.pts.len().saturating_sub(1);
                for (k, (u, v)) in mk.pts.into_iter().enumerate() {
                    let (x, y) = g.pad(u, v);
                    let phase = if k == 0 { PenPhase::Down } else if k == last { PenPhase::Up } else { PenPhase::Move };
                    st.points.push(PenEvent { x, y, pressure: 0.6, phase });
                }
                add.push(st);
            }
        }
        // la lettura vale ancora: i tratti vecchi non cambiano
        *self.last.lock().unwrap() = Some(reading);
        self.board.add_strokes(add);
        Ok(format!("Fatto: sul foglio ci sono {} (l'utente li vede e li può annullare).", done.join(", ")))
    }
}

impl ToolRunner for Tutor {
    fn defs(&self) -> Vec<Value> {
        crate::boardmarks::tool_definitions()
    }

    fn note(&self) -> String {
        BOARD_NOTE.into()
    }

    fn label(&self, name: &str) -> String {
        match name {
            "lavagna_leggi" => "Leggo il foglio della lavagna".into(),
            "lavagna_annota" => "Scrivo sul foglio".into(),
            other => other.into(),
        }
    }

    fn run(&self, name: &str, args: &Value) -> ToolOut {
        let fail = |e: anyhow::Error| ToolOut { text: format!("ERRORE: {e:#}"), short: format!("errore: {e:#}"), ..Default::default() };
        match name {
            "lavagna_leggi" => match self.read() {
                Ok((text, images)) => ToolOut { text, short: "foglio letto".into(), images, changed: false },
                Err(e) => fail(e),
            },
            "lavagna_annota" => match self.annotate(args) {
                Ok(text) => ToolOut { text, short: "segni sul foglio".into(), images: Vec::new(), changed: true },
                Err(e) => fail(e),
            },
            other => ToolOut { text: format!("ERRORE: strumento sconosciuto «{other}»"), short: "sconosciuto".into(), ..Default::default() },
        }
    }
}
