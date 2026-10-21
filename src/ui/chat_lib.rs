//! Le richieste della chat AI sull'archivio (vedi `libagent`): arrivano dal
//! thread della chat, l'interfaccia le fa (è lei che ha l'archivio) e
//! risponde. Aprire una pagina e salvare la lavagna fanno quello che fanno i
//! pulsanti: niente si perde (quello che c'era sulla lavagna va nell'archivio).

use super::library_view::LibAction;
use super::App;
use crate::libagent::{Data, Item, Req};
use crate::scene::Page;
use eframe::egui;

impl App {
    pub(super) fn chat_lib_requests(&mut self, ctx: &egui::Context) {
        for (req, reply) in std::mem::take(&mut self.chat_lib) {
            let r = self.chat_lib_req(req, ctx);
            let _ = reply.send(r);
        }
    }

    fn chat_lib_req(&mut self, req: Req, ctx: &egui::Context) -> Result<Data, String> {
        match req {
            Req::List => Ok(Data::List(
                self.lib
                    .entries
                    .iter()
                    .map(|e| Item {
                        id: e.id.clone(),
                        title: e.meta.title.clone(),
                        created: e.meta.created.clone(),
                        source: e.meta.source.clone(),
                        strokes: e.drawing.strokes.len(),
                        chart: e.chart.is_some(),
                        on_board: self.board_from_id.as_deref() == Some(e.id.as_str()),
                    })
                    .collect(),
            )),
            Req::Pages(ids) => {
                let view = self.view();
                let pages = ids
                    .iter()
                    .filter_map(|id| self.lib.get(id))
                    .map(|e| {
                        let mut p = Page::from_drawing(&e.drawing, view);
                        p.bg = e.chart.clone();
                        (e.meta.display_title(), e.id.clone(), p)
                    })
                    .collect();
                Ok(Data::Pages { pages, dir: super::savedlg::start_dir(&self.settings) })
            }
            Req::Open(id) => {
                let Some(t) = self.lib.get(&id).map(|e| e.meta.display_title()) else {
                    return Err(format!("non trovo la pagina «{id}» nell'archivio"));
                };
                self.apply_lib(LibAction::ToBoard(id.clone()), ctx);
                if self.board_from_id.as_deref() != Some(id.as_str()) {
                    return Err(self.status.clone());
                }
                let before = if self.status.contains("prima è nell'archivio") { " (quello che c'era prima è andato nell'archivio)" } else { "" };
                Ok(Data::Text(format!("Fatto: «{t}» è sulla lavagna{before}. Ora la puoi leggere e annotare con gli strumenti lavagna_*.")))
            }
            Req::Save { title, new_page, as_new } => self.chat_save_board(title, new_page, as_new).map(Data::Text),
            Req::Rename { id, title } => {
                if self.lib.get(&id).is_none() {
                    return Err(format!("non trovo la pagina «{id}» nell'archivio"));
                }
                self.lib.rename(&id, &title).map_err(|e| format!("archivio: {e}"))?;
                let shown = self.lib.get(&id).map(|e| e.meta.display_title()).unwrap_or_default();
                if self.board_from_id.as_deref() == Some(id.as_str()) {
                    self.board_from = Some(shown.clone());
                }
                Ok(Data::Text(format!("Fatto: la pagina {id} ora si chiama «{shown}».")))
            }
            Req::Trash(ids) => {
                let before = self.lib.entries.len();
                let on_board = self.board_from_id.as_ref().is_some_and(|b| ids.contains(b));
                self.apply_lib(LibAction::Trash(ids.clone()), ctx);
                let n = before - self.lib.entries.len();
                if n == 0 {
                    return Err(self.status.clone());
                }
                if on_board {
                    // la lavagna ha ancora la pagina: non è più nell'archivio
                    self.board_from_id = None;
                    self.quit.saved_sig = 0;
                }
                Ok(Data::Text(format!("Fatto: {n} nel cestino (si riprendono con archivio_ripristina).", n = if n == 1 { "1 pagina".to_string() } else { format!("{n} pagine") })))
            }
            Req::Restore(ids) => {
                let n = self.lib.restore(&ids).map_err(|e| format!("cestino: {e}"))?;
                if n == 0 {
                    return Err("nessuna di queste pagine è nel cestino".into());
                }
                Ok(Data::Text(format!("Fatto: {} dal cestino.", if n == 1 { "ripresa 1 pagina".to_string() } else { format!("riprese {n} pagine") })))
            }
        }
    }

    /// «Salva la lavagna»: la pagina aperta dall'archivio si aggiorna, se no
    /// ne nasce una nuova; poi, se chiesto, la lavagna si pulisce.
    fn chat_save_board(&mut self, title: Option<String>, new_page: bool, as_new: bool) -> Result<String, String> {
        if new_page && self.docui.doc.is_some() {
            return Err("al centro c'è un documento aperto: per una pagina nuova torna prima alla lavagna".into());
        }
        let (d, chart) = self.board_drawing();
        if d.is_empty() && chart.is_none() {
            return Err("La lavagna è vuota.".into());
        }
        let existing = self.board_from_id.clone().filter(|id| self.lib.get(id).is_some() && !as_new);
        let (id, how) = match existing {
            Some(id) => {
                if self.board_unsaved() {
                    self.lib.update(&id, &d, chart.as_ref()).map_err(|e| format!("archivio: {e}"))?;
                    self.lui.thumbs.remove(&id);
                    (id, "aggiornata")
                } else {
                    (id, "era già salvata così")
                }
            }
            None => (self.save_board_to_library()?, "salvata come pagina nuova"),
        };
        self.mark_board_saved();
        if let Some(t) = &title {
            self.lib.rename(&id, t).map_err(|e| format!("archivio: {e}"))?;
        }
        let shown = self.lib.get(&id).map(|e| e.meta.display_title()).unwrap_or_default();
        self.board_from_id = Some(id.clone());
        self.board_from = Some(shown.clone());
        let mut text = format!("Fatto: la lavagna è nell'archivio, pagina {id} «{shown}» ({how}).");
        if new_page {
            self.hub.edit(|s| s.clear());
            self.board_from_id = None;
            self.board_from = None;
            self.mark_board_saved();
            self.set_chat_context_from_board();
            text.push_str(" La lavagna è pulita per una pagina nuova (Cmd+Z la riporta).");
        }
        self.status = "Lavagna salvata nell'archivio dalla chat AI.".into();
        Ok(text)
    }
}
