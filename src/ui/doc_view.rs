//! Documenti sulla lavagna: import (anche trascinando il file sulla finestra),
//! pagine come sfondo, firma registrata e posizionabile, PDF firmato.

use super::{file_stem, Action, App};
use crate::document::{self, Document};
use crate::export;
use crate::scene::{PageBg, PageGeom, Scene};
use crate::settings::run_hook;
use crate::signature::Signature;
use crate::stroke::Stroke;
use eframe::egui;
use egui_phosphor::regular as ic;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq)]
enum Drag {
    Move,
    Resize,
}

/// Firma in corso di posizionamento.
pub(super) struct Stamp {
    sig: Signature,
    /// u, v, larghezza, altezza sulla pagina (0..1).
    rect: [f32; 4],
    drag: Option<Drag>,
}

pub(super) struct DocUi {
    pub doc: Option<Document>,
    /// Tratti della lavagna mentre è aperto un documento.
    board_stash: Option<Vec<Stroke>>,
    tex: HashMap<String, egui::TextureHandle>,
    saved_rev: u64,
    last_save: Instant,
    pub importing: bool,
    import_tx: std::sync::mpsc::Sender<Result<Document, String>>,
    import_rx: std::sync::mpsc::Receiver<Result<Document, String>>,
    pub stamp: Option<Stamp>,
    /// Registrazione della firma: tratti e sfondo da rimettere dopo.
    pub capture: Option<(Vec<Stroke>, Option<PageBg>)>,
    pub signature: Option<Signature>,
    pub last_export: Option<PathBuf>,
    /// Rettangolo della pagina sullo schermo nell'ultimo fotogramma.
    pub last_page: egui::Rect,
    /// Fusione sfondo + evidenziature preparata in un thread: (chiave, immagine).
    comp_tx: std::sync::mpsc::Sender<(String, egui::ColorImage)>,
    comp_rx: std::sync::mpsc::Receiver<(String, egui::ColorImage)>,
    comp_pending: Option<String>,
    /// Schede dei documenti aperti in questa sessione: (id, titolo).
    pub tabs: Vec<(String, String)>,
    /// Zona della lavagna sullo schermo nell'ultimo fotogramma.
    pub last_view: egui::Rect,
    /// Chi disegna nitide le pagine del documento aperto (WordCraft o PdfCraft).
    renderer: Option<(String, crate::docrender::Renderer)>,
    /// La parte visibile della pagina, disegnata alla risoluzione dello
    /// schermo: chiave, pagina, zona (frazioni della pagina) e immagine.
    detail: Option<(String, usize, [f32; 4], egui::TextureHandle)>,
    /// Zona voluta e da quando (si disegna quando zoom e scorrimento si fermano).
    detail_want: Option<(String, Instant)>,
    detail_pending: bool,
}

/// Immagini delle pagine: lineari e con le mipmap, così rimpicciolite non
/// perdono pezzi delle lettere (senza mipmap il testo fine si sgrana).
fn page_tex_options() -> egui::TextureOptions {
    egui::TextureOptions { mipmap_mode: Some(egui::TextureFilter::Linear), ..egui::TextureOptions::LINEAR }
}

/// Lato massimo (pixel) della parte nitida della pagina.
const DETAIL_MAX: f32 = 4096.0;

impl DocUi {
    pub fn new() -> Self {
        let (import_tx, import_rx) = std::sync::mpsc::channel();
        let (comp_tx, comp_rx) = std::sync::mpsc::channel();
        Self {
            last_view: egui::Rect::NOTHING,
            renderer: None,
            detail: None,
            detail_want: None,
            detail_pending: false,
            doc: None,
            board_stash: None,
            tex: HashMap::new(),
            saved_rev: 0,
            last_save: Instant::now(),
            importing: false,
            import_tx,
            import_rx,
            stamp: None,
            capture: None,
            signature: Signature::load(),
            last_export: None,
            last_page: egui::Rect::NOTHING,
            comp_tx,
            comp_rx,
            comp_pending: None,
            tabs: Vec::new(),
        }
    }
}

pub(super) enum DocAction {
    OpenDialog,
    OpenRecent(String),
    Page(usize),
    SaveSigned,
    Close,
    StartCapture,
    SaveCapture,
    CancelCapture,
    InsertSignature,
    ApplyStamp,
    CancelStamp,
    OpenLastExport,
    /// Scheda «Lavagna»: chiude il documento (la sua scheda resta).
    TabBoard,
    /// ✕ su una scheda: chiude il documento se è quello aperto e toglie la scheda.
    CloseTab(String),
}

/// Cosa si trascina da una scheda sulla chat AI.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum ChatDrag {
    Board,
    /// Documento: id e titolo.
    Doc(String, String),
}

fn doc(a: DocAction) -> Action {
    Action::Doc(a)
}

impl App {
    /// Avvia l'import in un thread (la conversione può richiedere qualche secondo).
    pub(super) fn start_import(&mut self, path: PathBuf) {
        if self.docui.importing {
            return;
        }
        self.docui.importing = true;
        self.status = format!(
            "Importo «{}»…",
            path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
        );
        let tx = self.docui.import_tx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(Document::import(&path).map_err(|e| format!("{e:#}")));
        });
    }

    /// Copia i tratti della lavagna nella pagina corrente del documento.
    fn sync_doc_strokes(&mut self) {
        if self.docui.capture.is_some() {
            return;
        }
        let strokes = self.hub.lock().drawing.strokes.clone();
        if let Some(d) = &mut self.docui.doc {
            if let Some(p) = d.pages.get_mut(d.current) {
                p.strokes = strokes;
            }
        }
    }

    fn save_doc(&mut self) {
        self.sync_doc_strokes();
        if let Some(d) = &self.docui.doc {
            if let Err(e) = d.save() {
                self.status = format!("⚠ salvataggio documento: {e}");
            }
        }
        self.docui.saved_rev = self.hub.lock().rev;
        self.docui.last_save = Instant::now();
    }

    pub(super) fn save_doc_on_exit(&mut self) {
        if self.docui.doc.is_some() {
            self.save_doc();
        }
    }

    fn open_doc(&mut self, d: Document) {
        if self.docui.doc.is_some() {
            self.save_doc();
        } else {
            self.docui.board_stash = Some(self.hub.lock().drawing.strokes.clone());
        }
        self.docui.stamp = None;
        if !self.docui.tabs.iter().any(|(id, _)| id == &d.id) {
            self.docui.tabs.push((d.id.clone(), d.title.clone()));
        }
        let n = d.pages.len();
        let title = d.title.clone();
        let cur = d.current;
        self.docui.doc = Some(d);
        self.docui.tex.clear();
        // load_page, non show_page: la lavagna attuale non va copiata nel documento.
        self.load_page(cur);
        self.mode = super::Mode::Board;
        let pagine = if n == 1 { "1 pagina".to_string() } else { format!("{n} pagine") };
        let salva = if self.docui.doc.as_ref().is_some_and(|d| d.original.is_some()) { "Salva Word" } else { "Salva PDF" };
        self.status = format!("«{title}» ({pagine}): scrivi e firma sul pad, compare sul documento. Poi «{salva}» (scheda Documento o Cmd+Shift+S).");
        // Pad non collegato: prova a collegarlo da solo.
        if !self.live_on {
            let _ = self.cmd_tx.send(crate::actor::UiCmd::Connect);
        }
    }

    /// Passa a un'altra pagina conservando i tratti di quella attuale.
    fn show_page(&mut self, i: usize) {
        self.sync_doc_strokes();
        self.load_page(i);
    }

    /// Mostra la pagina `i` del documento (tratti salvati + sfondo), senza
    /// toccare la pagina precedente.
    fn load_page(&mut self, i: usize) {
        let (view, w, h) = {
            let s = self.hub.lock();
            let (w, h) = crate::scene::page_dims(&s.drawing);
            (s.view, w, h)
        };
        let Some(d) = &mut self.docui.doc else { return };
        let i = i.min(d.pages.len().saturating_sub(1));
        d.current = i;
        let strokes = d.pages[i].strokes.clone();
        let bg = d.page_bg(i, view, w, h);
        self.hub.edit(|s| s.set_page(strokes, bg));
        self.board_zoom.fit();
        self.save_doc();
    }

    fn close_doc(&mut self) {
        self.save_doc();
        self.docui.doc = None;
        self.docui.stamp = None;
        self.docui.tex.clear();
        self.docui.renderer = None;
        self.docui.detail = None;
        self.docui.detail_want = None;
        let stash = self.docui.board_stash.take().unwrap_or_default();
        self.hub.edit(|s| s.set_page(stash, None));
        self.status = "Documento chiuso (resta in «Documenti recenti»).".into();
    }

    /// Ogni fotogramma: import finiti, file trascinati, salvataggio automatico,
    /// sfondo allineato a rotazione e misure del pad.
    pub(super) fn doc_tick(&mut self, ctx: &egui::Context) {
        while let Ok(r) = self.docui.import_rx.try_recv() {
            self.docui.importing = false;
            match r {
                Ok(d) => self.open_doc(d),
                Err(e) => self.status = format!("⚠ Import: {e}"),
            }
        }
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| Some(f.path().to_path_buf())).collect());
        if let Some(p) = dropped.into_iter().next() {
            self.start_import(p);
        }
        if self.docui.doc.is_some()
            && self.docui.capture.is_none()
            && self.hub.lock().rev != self.docui.saved_rev
            && self.docui.last_save.elapsed() > Duration::from_secs(4)
        {
            self.save_doc();
        }
        // Rotazione o misure del pad cambiate: ricalcola dove sta il documento.
        if let Some(d) = &self.docui.doc {
            let mut s = self.hub.lock();
            if let Some(bg) = &s.bg {
                let (w, h) = crate::scene::page_dims(&s.drawing);
                let g = PageGeom::new(s.view, w, h);
                if let Some(p) = d.pages.get(d.current) {
                    let (dw, dh) = p.pdf.display_size();
                    let r = document::fit_rect(dw, dh, g.w, g.h);
                    if r.iter().zip(bg.rect.iter()).any(|(a, b)| (a - b).abs() > 1e-4) {
                        if let Some(bg) = &mut s.bg {
                            bg.rect = r;
                        }
                        drop(s);
                        self.hub.edit(|_| {});
                    }
                }
            }
        }
    }

    /// Immagine della pagina del documento per lo schermo. Le evidenziature
    /// finite vengono fuse nell'immagine in "moltiplica" (come in un vero
    /// evidenziatore il testo sotto resta nero); quella in corso si disegna sopra.
    pub(super) fn page_image(&mut self, ctx: &egui::Context) -> Option<super::BgImage> {
        let (bg, hl, keep, dims, view) = {
            let s = self.hub.lock();
            let bg = s.bg.clone()?;
            let keep = s.active_index();
            let hl: Vec<Stroke> = s
                .drawing
                .strokes
                .iter()
                .enumerate()
                .filter(|(i, st)| st.highlight && Some(*i) != keep)
                .map(|(_, st)| st.clone())
                .collect();
            (bg, hl, keep, (s.drawing.width, s.drawing.height), s.view)
        };
        // la parte che si vede, nitida (WordCraft / PdfCraft): con questa le
        // evidenziature si disegnano sopra, trasparenti
        let detail = self.detail_tile(ctx, &bg);
        if hl.is_empty() || detail.is_some() {
            let tex = self.bg_texture(ctx, &bg)?;
            return Some(super::BgImage { tex, rect: bg.rect, baked: false, keep: None, detail });
        }
        // Firma delle evidenziature: se cambiano, si rifà l'immagine.
        let mut h: u64 = 1469598103934665603;
        for st in &hl {
            for v in [st.points.len() as u64, st.points[0].x.to_bits() as u64, st.points.last().unwrap().y.to_bits() as u64] {
                h = (h ^ v).wrapping_mul(1099511628211);
            }
        }
        let r = bg.rect;
        let key = format!("{}-hl{h:x}-{:.4}-{:.4}", bg.key, r[0], r[1]);
        // Immagini pronte dal thread.
        while let Ok((k, img)) = self.docui.comp_rx.try_recv() {
            let tex = ctx.load_texture(format!("doc-{k}"), img, page_tex_options());
            let prefix = format!("{}-hl", bg.key);
            self.docui.tex.retain(|old, _| !old.starts_with(&prefix));
            self.docui.tex.insert(k.clone(), tex);
            if self.docui.comp_pending.as_deref() == Some(k.as_str()) {
                self.docui.comp_pending = None;
            }
        }
        if let Some(t) = self.docui.tex.get(&key) {
            return Some(super::BgImage { tex: t.id(), rect: [0.0, 0.0, 1.0, 1.0], baked: true, keep, detail: None });
        }
        // Non ancora pronta: la prepara in un thread; intanto evidenziature
        // trasparenti disegnate sopra la pagina.
        if self.docui.comp_pending.as_deref() != Some(key.as_str()) {
            self.docui.comp_pending = Some(key.clone());
            let tx = self.docui.comp_tx.clone();
            let ctx2 = ctx.clone();
            let bg2 = bg.clone();
            std::thread::spawn(move || {
                let d = crate::stroke::Drawing { strokes: hl, width: dims.0, height: dims.1 };
                let mut page = crate::scene::Page::from_drawing(&d, view);
                page.bg = Some(bg2);
                if let Ok((w, hgt, px)) = export::page_to_rgba(&page, 2200) {
                    let _ = tx.send((key, egui::ColorImage::from_rgba_premultiplied([w, hgt], &px)));
                    ctx2.request_repaint();
                }
            });
        }
        let tex = self.bg_texture(ctx, &bg)?;
        Some(super::BgImage { tex, rect: bg.rect, baked: false, keep: None, detail: None })
    }

    /// La parte visibile della pagina disegnata alla risoluzione dello
    /// schermo dal motore del documento (WordCraft per i Word, PdfCraft per i
    /// PDF): il testo resta nitido a ogni ingrandimento. Si ridisegna quando
    /// zoom e scorrimento si fermano; intanto resta quella di prima.
    /// Ritorna (immagine, zona della pagina del documento in frazioni).
    fn detail_tile(&mut self, ctx: &egui::Context, bg: &PageBg) -> Option<(egui::TextureId, [f32; 4])> {
        let d = self.docui.doc.as_ref()?;
        let page = d.current;
        if bg.key != format!("{}-{}", d.id, page) {
            return None;
        }
        if self.docui.renderer.as_ref().is_none_or(|(id, _)| *id != d.id) {
            let ctx2 = ctx.clone();
            let r = crate::docrender::Renderer::start(d.dir.clone(), d.original.clone(), move || ctx2.request_repaint());
            self.docui.renderer = Some((d.id.clone(), r));
            self.docui.detail = None;
            self.docui.detail_pending = false;
        }
        let (_, renderer) = self.docui.renderer.as_ref()?;
        while let Some(r) = renderer.try_recv() {
            self.docui.detail_pending = false;
            match r {
                Ok(done) => {
                    let img = egui::ColorImage::from_rgba_premultiplied(done.size, &done.rgba);
                    let tex = ctx.load_texture(format!("doc-detail-{}", done.key), img, egui::TextureOptions::LINEAR);
                    self.docui.detail = Some((done.key, done.page, done.region, tex));
                }
                Err(e) => tracing::warn!("pagina nitida: {e}"),
            }
        }
        let shown = |det: &Option<(String, usize, [f32; 4], egui::TextureHandle)>| det.as_ref().filter(|x| x.1 == page).map(|x| (x.3.id(), x.2));
        // la pagina del documento sullo schermo e la parte che si vede
        let pad = self.docui.last_page;
        let doc_rect = egui::Rect::from_min_size(
            pad.min + egui::vec2(bg.rect[0] * pad.width(), bg.rect[1] * pad.height()),
            egui::vec2(bg.rect[2] * pad.width(), bg.rect[3] * pad.height()),
        );
        let vis = doc_rect.intersect(self.docui.last_view);
        if !vis.is_positive() || !doc_rect.is_positive() {
            return shown(&self.docui.detail);
        }
        let ppp = ctx.pixels_per_point();
        let px = [(vis.width() * ppp).round().clamp(1.0, DETAIL_MAX) as u32, (vis.height() * ppp).round().clamp(1.0, DETAIL_MAX) as u32];
        let region = [
            (vis.min.x - doc_rect.min.x) / doc_rect.width(),
            (vis.min.y - doc_rect.min.y) / doc_rect.height(),
            vis.width() / doc_rect.width(),
            vis.height() / doc_rect.height(),
        ];
        let key = format!("{}-{page}-{:.4}-{:.4}-{:.4}-{:.4}-{}x{}", d.id, region[0], region[1], region[2], region[3], px[0], px[1]);
        if self.docui.detail.as_ref().is_some_and(|x| x.0 == key) {
            return shown(&self.docui.detail);
        }
        let now = Instant::now();
        match &self.docui.detail_want {
            Some((k, since)) if *k == key => {
                if since.elapsed() >= Duration::from_millis(120) && !self.docui.detail_pending {
                    self.docui.detail_pending = true;
                    let page_pt = d.pages.get(page).map(|p| p.pdf.display_size()).unwrap_or((595.0, 842.0));
                    renderer.request(crate::docrender::Request { key, page, region, px, page_pt });
                } else {
                    ctx.request_repaint_after(Duration::from_millis(130));
                }
            }
            _ => {
                self.docui.detail_want = Some((key, now));
                ctx.request_repaint_after(Duration::from_millis(130));
            }
        }
        shown(&self.docui.detail)
    }

    /// Texture dello sfondo (in cache per pagina).
    pub(super) fn bg_texture(&mut self, ctx: &egui::Context, bg: &PageBg) -> Option<egui::TextureId> {
        if let Some(t) = self.docui.tex.get(&bg.key) {
            return Some(t.id());
        }
        let pm = tiny_skia::Pixmap::decode_png(&bg.png).ok()?;
        let img = egui::ColorImage::from_rgba_premultiplied([pm.width() as usize, pm.height() as usize], pm.data());
        let tex = ctx.load_texture(format!("doc-{}", bg.key), img, page_tex_options());
        let id = tex.id();
        self.docui.tex.insert(bg.key.clone(), tex);
        Some(id)
    }

    pub(super) fn apply_doc(&mut self, a: DocAction) {
        match a {
            DocAction::OpenDialog => {
                let picked = rfd::FileDialog::new()
                    .set_title("Apri un documento da firmare")
                    .add_filter("Documenti", crate::docimport::EXTENSIONS)
                    .pick_file();
                if let Some(p) = picked {
                    self.start_import(p);
                }
            }
            DocAction::OpenRecent(id) => match Document::open(&id) {
                Ok(d) => self.open_doc(d),
                Err(e) => self.status = format!("⚠ {e:#}"),
            },
            DocAction::Page(i) => self.show_page(i),
            DocAction::SaveSigned => {
                self.save_doc();
                let Some(d) = &self.docui.doc else { return };
                // stesso formato del documento: Word resta Word, PDF resta PDF
                let dst = export::new_path(&file_stem(&d.title), &d.signed_ext());
                let (view, pad) = {
                    let s = self.hub.lock();
                    (s.view, s.drawing.clone())
                };
                match d.export_signed(view, &pad, &dst) {
                    Ok(()) => {
                        run_hook(&self.settings.hook, &dst.to_string_lossy());
                        self.status = format!("Salvato: {}", dst.display());
                        // Lo apre subito (Anteprima) per vedere il risultato.
                        crate::platform::open(&dst);
                        self.docui.last_export = Some(dst);
                    }
                    Err(e) => self.status = format!("⚠ Salvataggio: {e:#}"),
                }
            }
            DocAction::Close => {
                if let Some(id) = self.docui.doc.as_ref().map(|d| d.id.clone()) {
                    self.docui.tabs.retain(|(t, _)| t != &id);
                }
                self.close_doc();
            }
            DocAction::TabBoard => {
                if self.docui.doc.is_some() {
                    self.close_doc();
                    self.status = "Lavagna. Il documento resta nella sua scheda: un clic per riaprirlo.".into();
                }
            }
            DocAction::CloseTab(id) => {
                if self.docui.doc.as_ref().is_some_and(|d| d.id == id) {
                    self.close_doc();
                }
                self.docui.tabs.retain(|(t, _)| t != &id);
            }
            DocAction::StartCapture => {
                self.docui.stamp = None;
                if self.docui.capture.is_some() {
                    return;
                }
                self.sync_doc_strokes();
                let stash = {
                    let s = self.hub.lock();
                    (s.drawing.strokes.clone(), s.bg.clone())
                };
                self.docui.capture = Some(stash);
                self.hub.edit(|s| {
                    s.set_page(Vec::new(), None);
                    s.tool = crate::scene::Tool::Pen;
                });
                self.board_zoom.fit();
                self.status = "Firma sul pad, come su un foglio. Annulla (Cmd+Z) per riprovare, poi «Salva firma».".into();
            }
            DocAction::SaveCapture | DocAction::CancelCapture => {
                let save = matches!(a, DocAction::SaveCapture);
                let Some((strokes, bg)) = self.docui.capture.take() else { return };
                let (written, view) = {
                    let s = self.hub.lock();
                    (s.drawing.strokes.clone(), s.view)
                };
                self.hub.edit(|s| s.set_page(strokes, bg));
                if !save {
                    self.status = "Registrazione della firma annullata.".into();
                    return;
                }
                match Signature::from_strokes(&written, view) {
                    Some(sig) => match sig.save() {
                        Ok(()) => {
                            self.docui.signature = Some(sig);
                            self.status = "Firma registrata ✓".into();
                            // Con un documento aperto la si posa subito.
                            if self.docui.doc.is_some() {
                                self.apply_doc(DocAction::InsertSignature);
                            }
                        }
                        Err(e) => self.status = format!("⚠ firma: {e}"),
                    },
                    None => self.status = "Nessuna firma scritta: registrazione annullata.".into(),
                }
            }
            DocAction::InsertSignature => {
                let Some(sig) = self.docui.signature.clone() else {
                    self.status = "Prima registra la firma («Registra firma»).".into();
                    return;
                };
                let g = self.hub.lock().geom();
                let mut r = sig.natural_rect(&g, 0.68, 0.84);
                // mai più larga di metà pagina
                if r[2] > 0.5 {
                    let k = 0.5 / r[2];
                    r = sig.natural_rect(&g, 0.68, 0.84);
                    r = [0.68 - r[2] * k / 2.0, 0.84 - r[3] * k / 2.0, r[2] * k, r[3] * k];
                }
                self.docui.stamp = Some(Stamp { sig, rect: r, drag: None });
                self.status = "Trascina la firma dove serve, rotella o angolo per la grandezza, poi Invio.".into();
            }
            DocAction::ApplyStamp => {
                if let Some(st) = self.docui.stamp.take() {
                    let g = self.hub.lock().geom();
                    let strokes = st.sig.place(st.rect, &g);
                    self.hub.edit(|s| s.add_strokes(strokes));
                    self.status = "Firma apposta ✓ (Cmd+Z per toglierla).".into();
                }
            }
            DocAction::CancelStamp => self.docui.stamp = None,
            DocAction::OpenLastExport => {
                if let Some(p) = &self.docui.last_export {
                    crate::platform::open(p);
                }
            }
        }
    }

    /// Schede dei file: «Lavagna» e i documenti aperti, con ✕ per chiuderli.
    /// A destra le pagine del documento aperto.
    pub(super) fn file_tabs(&self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        use super::theme::{icon, medium, semibold, Tokens};
        use egui::{pos2, vec2, Align2, Sense};
        let t = Tokens::get(ui.ctx());
        let active = self.docui.doc.as_ref().map(|d| d.id.clone());
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            // una scheda: Some(true) = clic, Some(false) = chiudi; trascinandola
            // sulla chat AI il foglio o il documento vanno nel contesto della chat
            let tab = |ui: &mut egui::Ui, on: bool, glyph: &str, text: &str, closable: bool, drag: ChatDrag| -> Option<bool> {
                let font = if on { semibold(12.0) } else { medium(12.0) };
                let tw = ui.painter().layout_no_wrap(text.to_string(), font.clone(), t.text).size().x;
                let w = tw + 40.0 + if closable { 20.0 } else { 0.0 };
                let (r, resp) = ui.allocate_exact_size(vec2(w, 27.0), Sense::click_and_drag());
                resp.dnd_set_drag_payload(drag);
                let resp = resp.on_hover_text("Clic per aprire · trascina sulla chat AI per allegarlo");
                let round = egui::CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 };
                if on {
                    ui.painter().rect_filled(r, round, t.input);
                    ui.painter().rect_filled(egui::Rect::from_min_size(r.min + vec2(6.0, 0.0), vec2(r.width() - 12.0, 2.5)), 1.0, t.accent);
                } else if resp.hovered() {
                    ui.painter().rect_filled(r, round, t.hover);
                }
                let col = if on { t.text } else { t.text_dim };
                ui.painter().text(pos2(r.min.x + 16.0, r.center().y), Align2::CENTER_CENTER, glyph, icon(14.0), if on { t.accent_text } else { t.text_dim });
                ui.painter().text(pos2(r.min.x + 28.0, r.center().y), Align2::LEFT_CENTER, text, font, col);
                let mut out = None;
                if closable {
                    let x = egui::Rect::from_center_size(pos2(r.max.x - 14.0, r.center().y), vec2(18.0, 18.0));
                    let xr = ui.interact(x, resp.id.with("close"), Sense::click());
                    if xr.hovered() {
                        ui.painter().rect_filled(x, 4.0, t.pressed);
                    }
                    ui.painter().text(x.center(), Align2::CENTER_CENTER, ic::X, icon(12.0), col);
                    if xr.on_hover_text("Chiudi questo file").clicked() {
                        out = Some(false);
                    }
                }
                if out.is_none() && resp.clicked() {
                    out = Some(true);
                }
                out
            };
            let board = self.center == super::workspace::Center::Board;
            if tab(ui, board && active.is_none(), ic::CHALKBOARD_SIMPLE, "Lavagna", false, ChatDrag::Board) == Some(true) {
                actions.push(Action::ShowBoard);
                actions.push(doc(DocAction::TabBoard));
            }
            for (id, title) in &self.docui.tabs {
                let on = board && active.as_deref() == Some(id.as_str());
                let short: String = if title.chars().count() > 32 { format!("{}…", title.chars().take(31).collect::<String>()) } else { title.clone() };
                match tab(ui, on, ic::FILE_TEXT, &short, true, ChatDrag::Doc(id.clone(), title.clone())) {
                    Some(false) => actions.push(doc(DocAction::CloseTab(id.clone()))),
                    Some(true) if !on => {
                        actions.push(Action::ShowBoard);
                        if active.as_deref() != Some(id.as_str()) {
                            actions.push(doc(DocAction::OpenRecent(id.clone())));
                        }
                    }
                    _ => {}
                }
            }
            self.web_tabs(ui, actions);
            if self.docui.importing {
                ui.spinner();
            } else if super::widgets::icon_button(ui, ic::PLUS, "Apri un documento (Cmd+O)", vec2(24.0, 24.0), true).clicked() {
                actions.push(doc(DocAction::OpenDialog));
            }
            if let Some(d) = self.docui.doc.as_ref().filter(|_| board) {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let n = d.pages.len();
                    if super::widgets::icon_button(ui, ic::CARET_DOWN, "Pagina successiva (PagGiù)", vec2(24.0, 22.0), d.current + 1 < n).clicked() {
                        actions.push(doc(DocAction::Page(d.current + 1)));
                    }
                    ui.label(egui::RichText::new(format!("pagina {} di {n}", d.current + 1)).font(super::theme::regular(11.5)).color(t.text_dim));
                    if super::widgets::icon_button(ui, ic::CARET_UP, "Pagina precedente (PagSu)", vec2(24.0, 22.0), d.current > 0).clicked() {
                        actions.push(doc(DocAction::Page(d.current - 1)));
                    }
                });
            }
        });
    }

    pub(super) fn doc_keys(&self, ctx: &egui::Context, actions: &mut Vec<Action>) -> bool {
        if ctx.egui_wants_keyboard_input() {
            return false;
        }
        ctx.input(|i| {
            if self.docui.stamp.is_some() {
                if i.key_pressed(egui::Key::Enter) {
                    actions.push(doc(DocAction::ApplyStamp));
                    return true;
                }
                if i.key_pressed(egui::Key::Escape) {
                    actions.push(doc(DocAction::CancelStamp));
                    return true;
                }
            }
            if let Some(d) = &self.docui.doc {
                if i.key_pressed(egui::Key::PageDown) && d.current + 1 < d.pages.len() {
                    actions.push(doc(DocAction::Page(d.current + 1)));
                    return true;
                }
                if i.key_pressed(egui::Key::PageUp) && d.current > 0 {
                    actions.push(doc(DocAction::Page(d.current - 1)));
                    return true;
                }
            }
            false
        })
    }

    fn stamp_screen_rect(&self, st: &Stamp) -> egui::Rect {
        let p = self.docui.last_page;
        egui::Rect::from_min_size(
            p.min + egui::vec2(st.rect[0] * p.width(), st.rect[1] * p.height()),
            egui::vec2(st.rect[2] * p.width(), st.rect[3] * p.height()),
        )
    }

    /// Mouse sulla firma da posizionare. Ritorna true se l'input è suo (allora
    /// la pagina non si sposta né si ingrandisce).
    pub(super) fn stamp_input(&mut self, ui: &egui::Ui, resp: &egui::Response) -> bool {
        let page = self.docui.last_page;
        let Some(st) = self.docui.stamp.as_ref() else { return false };
        if !page.is_positive() {
            return false;
        }
        let srect = self.stamp_screen_rect(st);
        let handle = egui::Rect::from_center_size(srect.max, egui::vec2(22.0, 22.0));
        let st = self.docui.stamp.as_mut().unwrap();
        if resp.drag_started() {
            // dove è stato premuto il tasto (non dove si trova ora il puntatore)
            let origin = ui.input(|i| i.pointer.press_origin()).or(resp.interact_pointer_pos());
            st.drag = origin.and_then(|p| {
                if handle.contains(p) {
                    Some(Drag::Resize)
                } else if srect.expand(6.0).contains(p) {
                    Some(Drag::Move)
                } else {
                    None
                }
            });
        }
        if resp.drag_stopped() {
            st.drag = None;
        }
        if let (Some(mode), true) = (st.drag, resp.dragged()) {
            let d = resp.drag_delta();
            match mode {
                Drag::Move => {
                    st.rect[0] += d.x / page.width();
                    st.rect[1] += d.y / page.height();
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                }
                Drag::Resize => {
                    let w_px = st.rect[2] * page.width();
                    let k = ((w_px + d.x) / w_px).max(0.05);
                    if w_px * k > 20.0 {
                        st.rect[2] *= k;
                        st.rect[3] *= k;
                    }
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNwSe);
                }
            }
            return true;
        }
        let Some(hover) = resp.hover_pos() else { return st.drag.is_some() };
        if !(srect.expand(6.0).contains(hover) || handle.contains(hover)) {
            return st.drag.is_some();
        }
        ui.ctx().set_cursor_icon(if handle.contains(hover) {
            egui::CursorIcon::ResizeNwSe
        } else {
            egui::CursorIcon::Grab
        });
        // rotella / pizzico sulla firma: grandezza attorno al centro
        let mut k = 1.0_f32;
        for e in ui.input(|i| i.events.clone()) {
            match e {
                egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Line, delta, .. } => k *= 1.1_f32.powf(delta.y),
                egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta, .. } => k *= (delta.y / 300.0).exp(),
                egui::Event::Zoom(f) => k *= f,
                _ => {}
            }
        }
        if (k - 1.0).abs() > 1e-4 {
            let (cx, cy) = (st.rect[0] + st.rect[2] / 2.0, st.rect[1] + st.rect[3] / 2.0);
            let (w, h) = (st.rect[2] * k, st.rect[3] * k);
            if w * page.width() > 20.0 && w < 1.5 {
                st.rect = [cx - w / 2.0, cy - h / 2.0, w, h];
            }
        }
        true
    }

    /// Disegna sopra la pagina: firma da posizionare, avviso di registrazione.
    pub(super) fn paint_doc_overlay(&self, painter: &egui::Painter, page: egui::Rect, scene: &Scene) {
        if let Some(st) = &self.docui.stamp {
            let r = self.stamp_screen_rect(st);
            let g = scene.geom();
            let scale = st.rect[3] / (st.sig.h / g.h);
            for s in &st.sig.strokes {
                let col = egui::Color32::from_rgb(s.color[0], s.color[1], s.color[2]);
                let w = (s.width * scale * page.height() / 800.0).max(0.8);
                let pts: Vec<egui::Pos2> = s
                    .points
                    .iter()
                    .map(|p| egui::pos2(r.left() + p.x / st.sig.w * r.width(), r.top() + p.y / st.sig.h * r.height()))
                    .collect();
                if pts.len() == 1 {
                    painter.circle_filled(pts[0], w * 0.6, col);
                } else {
                    painter.add(egui::Shape::line(pts, egui::Stroke::new(w, col)));
                }
            }
            let accent = egui::Color32::from_rgb(40, 110, 220);
            let b = r.expand(6.0);
            for (a, c) in [
                (b.left_top(), b.right_top()),
                (b.right_top(), b.right_bottom()),
                (b.right_bottom(), b.left_bottom()),
                (b.left_bottom(), b.left_top()),
            ] {
                painter.add(egui::Shape::dashed_line(&[a, c], egui::Stroke::new(1.5_f32, accent), 6.0, 4.0));
            }
            painter.rect_filled(egui::Rect::from_center_size(r.max, egui::vec2(12.0, 12.0)), 2.0, accent);
        }
        if self.docui.capture.is_some() {
            let text = format!("{} FIRMA SUL PAD", ic::SIGNATURE);
            let galley = painter.layout_no_wrap(text, egui::FontId::proportional(20.0), egui::Color32::WHITE);
            let pos = egui::pos2(page.center().x - galley.size().x / 2.0, page.top() + 16.0);
            let r = egui::Rect::from_min_size(pos, galley.size()).expand2(egui::vec2(12.0, 6.0));
            painter.rect_filled(r, 6.0, egui::Color32::from_rgb(40, 110, 220));
            painter.galley(pos, galley, egui::Color32::WHITE);
        }
    }
}
