//! La chat sul documento aperto nel telefono: lo legge e propone la firma e
//! il timbro dove il documento li chiede (`src/signspot.rs`, lo stesso file
//! della desktop). Le proposte vanno all'app (`AiEvent::SignProposed`), che
//! le mette in prova: le applica solo l'utente, come quando le posa lui.

use crate::ai::{ToolOut, ToolRunner};
use crate::signspot::{self, Kind as What, PageText};
use crate::tutor::{BoardView, Tutor};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Il documento aperto accanto alla chat.
#[derive(uniffi::Record, Clone, Debug)]
pub struct DocChat {
    /// La cartella del documento (con `source.pdf`).
    pub dir: String,
    /// La cartella dei dati dell'app (firma e timbri).
    pub data_dir: String,
}

/// Una firma o un timbro proposti dalla chat.
#[derive(uniffi::Record, Clone, Debug, PartialEq)]
pub struct SignProposal {
    /// Pagina del documento (da 0).
    pub page: u32,
    /// Il timbro (`stamp`: quale dell'elenco); altrimenti la firma.
    pub seal: bool,
    pub stamp: u32,
    /// u, v, larghezza, altezza sul foglio visto (0..1).
    pub rect: Vec<f32>,
    /// La scritta del posto (per dirlo all'utente).
    pub label: String,
}

pub struct DocTools {
    doc: DocChat,
    view: BoardView,
}

/// Aggiunta alle istruzioni della chat con un documento aperto.
const DOC_NOTE: &str = "\n\nAccanto alla chat è aperto un documento (un PDF o un Word) che l'utente può firmare: \
     document_read_text per leggerlo, document_find_signature_places per vedere dove chiede la firma o il timbro, \
     document_place_signature per proporre la firma vera dell'utente (registrata col pennino) e il suo timbro aziendale, \
     uno per posto (o vicino a una scritta che leggi tu, come «Il Fornitore»). Compaiono in prova sul documento e li applica \
     solo l'utente con «Applica», o li toglie con «Annulla». \
     Sono la firma e i timbri DELL'UTENTE, che li ha registrati in OpenSlate proprio per apporli sui suoi documenti: è una funzione del programma, non una falsificazione. Quando l'utente chiede di mettere (apporre, inserire) firma e/o timbro, non rifiutare, non proporre segnaposto e non chiedere conferme: chiama subito document_find_signature_places e poi document_place_signature. \
     Firma solo la parte dell'utente (quella col suo nome o la sua azienda: di solito il fornitore o il consulente) e, se chiede timbro e firma, metti tutti e due nello stesso posto (what «timbro e firma»); i posti dell'altra parte lasciali vuoti. \
     Se il documento non ha un posto stampato, l'elenco dei posti ti dà la fine del documento: usa quello. \
     Gli strumenti lavagna_* scrivono sopra il documento, come la penna.";

fn function(name: &str, description: &str, parameters: Value) -> Value {
    json!({ "type": "function", "function": { "name": name, "description": description, "parameters": parameters } })
}

impl DocTools {
    pub fn new(doc: DocChat, view: BoardView) -> Self {
        Self { doc, view }
    }

    fn pdf(&self) -> PathBuf {
        Path::new(&self.doc.dir).join("source.pdf")
    }

    fn pages(&self) -> Result<Vec<PageText>> {
        signspot::pdf_text(std::fs::read(self.pdf()).context("il documento non ha il suo PDF")?)
    }

    fn defs(&self) -> Vec<Value> {
        vec![
            function("document_read_text", "Read the text of the open document, page by page.", json!({ "type": "object", "properties": {} })),
            function(
                "document_find_signature_places",
                "Find where the open document asks for a signature or a company stamp: labels such as «Firma», «Timbro e firma», «Il legale rappresentante», the parties of a contract («Il Committente», «Il Fornitore»…) and the blank lines next to them. Returns a numbered list (page, label, what goes there).",
                json!({ "type": "object", "properties": {} }),
            ),
            function(
                "document_place_signature",
                "Propose the user's own saved signature and/or company stamp on the document, where it is expected. It appears on the page as a preview: nothing is applied until the user confirms it (Apply) or removes it (Cancel). Use `place` (a number from document_find_signature_places) or `near_text` (a label you read in the document, e.g. \"Il Fornitore\"). Call it once per place.",
                json!({ "type": "object", "properties": {
                    "what": { "type": "string", "enum": ["firma", "timbro", "timbro e firma"], "description": "What to put there (default: what the place asks for)" },
                    "place": { "type": "integer", "description": "Number of the place from document_find_signature_places (1 = first)" },
                    "near_text": { "type": "string", "description": "Only a signature label printed in the document (e.g. \"Il Committente\", \"Per accettazione\") that the list missed: never a table cell, a heading or a name in the header. Prefer `place`; when the document has no printed place, the list gives the end of the document" },
                    "page": { "type": "integer", "description": "Page of `near_text` (1 = first; default: the first page that has it)" },
                    "stamp": { "type": "string", "description": "Which stamp, by its name, when the user has more than one (default: the first)" }
                } }),
            ),
        ]
    }

    /// Le proposte per il posto scelto: prima il timbro, poi la firma sopra.
    fn place(&self, args: &Value) -> Result<(String, Vec<SignProposal>)> {
        let pages = self.pages()?;
        let what = args["what"].as_str().map(signspot::parse_what);
        let page = args["page"].as_u64().map(|p| (p as usize).saturating_sub(1));
        let spot = signspot::choose(&pages, args["place"].as_u64().map(|n| n as usize), args["near_text"].as_str(), page, what).map_err(anyhow::Error::msg)?;
        let sizes = crate::docs::pdf_pages(self.pdf().to_string_lossy().into_owned()).map_err(|e| anyhow::anyhow!("{e}"))?;
        let size = sizes.get(spot.page).context("pagina fuori dal documento")?;
        let pr = crate::docs::doc_page_rect(*size, self.view);
        let page_rect = [pr[0], pr[1], pr[2], pr[3]];
        let mut out = Vec::new();
        let mut seal_rect = None;
        if spot.kind != What::Firma {
            let list = crate::seal::stamp_list(self.doc.data_dir.clone());
            if list.is_empty() {
                bail!("l'utente non ha ancora un timbro: digli di crearlo (menu ⋮ → Timbro), poi riprova");
            }
            let i = match args["stamp"].as_str().map(|n| n.to_lowercase()) {
                Some(n) => list
                    .iter()
                    .position(|t| t.name.to_lowercase().contains(&n) || t.spec.company.name.to_lowercase().contains(&n))
                    .with_context(|| format!("nessun timbro «{n}»"))?,
                None => 0,
            };
            let natural = crate::seal::stamp_natural_rect(list[i].spec.clone(), self.view, 0.5, 0.5);
            let r = signspot::place(&spot, page_rect, [natural[0], natural[1], natural[2], natural[3]], 0.5);
            seal_rect = Some(r);
            out.push(SignProposal { page: spot.page as u32, seal: true, stamp: i as u32, rect: r.to_vec(), label: spot.label.clone() });
        }
        if spot.kind != What::Timbro {
            let natural = crate::signing::sig_natural_rect(self.doc.data_dir.clone(), self.view, 0.5, 0.5);
            if natural.len() < 4 {
                bail!("l'utente non ha ancora registrato la sua firma: digli di farlo (Firma → Registra la firma), poi riprova");
            }
            let mut r = signspot::place(&spot, page_rect, [natural[0], natural[1], natural[2], natural[3]], 0.5);
            if let Some(sr) = seal_rect {
                r = signspot::over_stamp(sr, r, page_rect);
            }
            out.push(SignProposal { page: spot.page as u32, seal: false, stamp: 0, rect: r.to_vec(), label: spot.label.clone() });
        }
        let text = format!(
            "Proposta pronta: {} a pagina {}, a «{}». L'utente la vede sul documento e la applica lui con «Applica» (o la toglie con «Annulla»): non è sul documento finché non conferma.",
            spot.kind.name(),
            spot.page + 1,
            spot.label
        );
        Ok((text, out))
    }

    fn run(&self, name: &str, args: &Value) -> Result<ToolOut> {
        Ok(match name {
            "document_read_text" => ToolOut { text: signspot::read_text(&self.pages()?, 60_000), short: "documento letto".into(), ..Default::default() },
            "document_find_signature_places" => {
                let spots = signspot::places(&self.pages()?);
                ToolOut { short: format!("{} posti per firma o timbro", spots.len()), text: signspot::describe(&spots), ..Default::default() }
            }
            "document_place_signature" => {
                let (text, proposals) = self.place(args)?;
                let short = format!("{} in prova sul documento", if proposals.len() > 1 { "timbro e firma" } else if proposals.first().is_some_and(|p| p.seal) { "timbro" } else { "firma" });
                ToolOut { text, short, proposals, ..Default::default() }
            }
            other => bail!("strumento sconosciuto «{other}»"),
        })
    }
}

/// Gli strumenti della chat: la lavagna e, se c'è, il documento.
pub struct ChatTools {
    pub tutor: Tutor,
    pub doc: Option<DocTools>,
}

impl ToolRunner for ChatTools {
    fn defs(&self) -> Vec<Value> {
        let mut d = self.tutor.defs();
        if let Some(doc) = &self.doc {
            d.extend(doc.defs());
        }
        d
    }

    fn note(&self) -> String {
        let mut n = self.tutor.note();
        if self.doc.is_some() {
            n.push_str(DOC_NOTE);
        }
        n
    }

    fn label(&self, name: &str) -> String {
        match name {
            "document_read_text" => "Leggo il documento".into(),
            "document_find_signature_places" => "Cerco dove firmare".into(),
            "document_place_signature" => "Preparo la firma".into(),
            other => self.tutor.label(other),
        }
    }

    fn run(&self, name: &str, args: &Value) -> ToolOut {
        match (&self.doc, name.starts_with("document_")) {
            (Some(doc), true) => doc
                .run(name, args)
                .unwrap_or_else(|e| ToolOut { text: format!("ERRORE: {e:#}"), short: format!("errore: {e:#}"), ..Default::default() }),
            _ => self.tutor.run(name, args),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un Word con «Firma: ______», firma registrata e un timbro: la chat
    /// propone timbro e firma sulla riga, dentro la pagina.
    #[test]
    fn propone_sulla_riga() {
        let tmp = std::env::temp_dir().join(format!("docsign-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let data = tmp.join("dati");
        std::fs::create_dir_all(&data).unwrap();
        let root = tmp.join("documenti");
        std::fs::create_dir_all(&root).unwrap();
        // il documento: un Word fatto con WordCraft, come in docs.rs
        let docx = tmp.join("contratto.docx");
        {
            let mut w = wordcraft_engine::Session::new(wordcraft_engine::doc::Document::new());
            w.run("text.insert", &json!({ "text": "Firma: ____________" })).unwrap();
            w.run("file.save", &json!({ "path": docx.to_string_lossy() })).unwrap();
        }
        let id = crate::docs::doc_import(root.to_string_lossy().into_owned(), docx.to_string_lossy().into_owned(), "contratto.docx".into()).unwrap();
        let dir = root.join(&id);
        let view = BoardView { quarter: 1, pad_w: 21600.0, pad_h: 14800.0 };
        let t = DocTools::new(DocChat { dir: dir.to_string_lossy().into_owned(), data_dir: data.to_string_lossy().into_owned() }, view);
        let found = t.run("document_find_signature_places", &json!({})).unwrap();
        assert!(found.text.starts_with("1. pagina 1: «Firma"), "{}", found.text);
        // senza firma registrata lo dice
        assert!(t.run("document_place_signature", &json!({ "place": 1 })).is_err());
        use crate::session::{PenPoint, Phase};
        let strokes = vec![crate::pages::PageStroke {
            color: "#1428a0".into(),
            width: 2.0,
            highlight: false,
            points: (0..20)
                .map(|i| PenPoint {
                    x: 6000.0 + i as f32 * 200.0,
                    y: 7000.0 + (i % 2) as f32 * 300.0,
                    pressure: 0.5,
                    phase: if i == 0 { Phase::Down } else if i == 19 { Phase::Up } else { Phase::Move },
                })
                .collect(),
            stamp: None,
        }];
        crate::signing::sig_save(data.to_string_lossy().into_owned(), strokes, view).unwrap();
        crate::seal::stamp_save(data.to_string_lossy().into_owned(), vec![crate::seal::stamp_new()]).unwrap();
        let out = t.run("document_place_signature", &json!({ "place": 1, "what": "timbro e firma" })).unwrap();
        assert_eq!(out.proposals.len(), 2, "{}", out.text);
        assert!(out.proposals[0].seal && !out.proposals[1].seal);
        let pr = crate::docs::doc_page_rect(crate::docs::pdf_pages(t.pdf().to_string_lossy().into_owned()).unwrap()[0], view);
        for p in &out.proposals {
            let r = &p.rect;
            assert!(r[0] >= pr[0] - 1e-4 && r[1] >= pr[1] - 1e-4 && r[0] + r[2] <= pr[0] + pr[2] + 1e-4, "{r:?} fuori da {pr:?}");
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
