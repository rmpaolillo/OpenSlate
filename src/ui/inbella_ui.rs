//! «Bella copia» (pulsante «Trasforma in bella»): scegli una zona, guarda
//! l'anteprima, accetta o rifiuta.
//!
//! 1. Un clic sul pulsante: si trascina col mouse un riquadro attorno a
//!    quello che si vuole in bella (o «Tutta la pagina»).
//! 2. L'AI legge solo quella zona (l'immagine ritagliata, in alta
//!    risoluzione), con un agente solo, scelto nel menu del pulsante; i
//!    disegni della zona diventano perfetti senza AI. La barra di stato dice
//!    chi legge e da quanto.
//! 3. L'anteprima: la zona si vela e sopra si vede la bella copia. Invio la
//!    accetta (un passo solo da annullare), Esc la rifiuta, «Rileggi» la rifà.
//!
//! Ogni riga resta alla sua altezza (le righe di un blocco cominciano dallo
//! stesso bordo, i trattini di un elenco in colonna), e non va oltre il bordo
//! destro della zona (non finisce sulla colonna accanto). Vedi
//! `crate::inbella`. Con «Sulle righe del foglio» la bella copia va poi
//! sulle righe della carta (`crate::righe`).
//!
//! «Disponi sulle righe» usa la stessa zona e la stessa anteprima, senza AI:
//! la scrittura (a mano o in bella) si posa sulle righe del foglio.

use super::App;
use eframe::egui;
use crate::stroke::Stroke;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

/// La risposta dell'AI, col numero della lettura (le risposte di una
/// lettura annullata non contano più).
type Done = (u64, anyhow::Result<(HashMap<u64, String>, String)>);

/// A che punto è la bella copia.
#[derive(Default)]
pub(super) enum Phase {
    #[default]
    Off,
    /// Si sceglie la zona: dove è cominciato il trascinamento e dov'è ora
    /// (frazioni della pagina).
    Select { from: Option<(f32, f32)>, to: (f32, f32) },
    /// L'AI legge la zona.
    Reading { rect: [f32; 4], started: Instant, agent: String },
    /// L'anteprima, da accettare o rifiutare: la pagina com'è dopo, dove è
    /// cambiata (il velo) e quello che ha letto l'AI (per rifarla sul foglio
    /// com'è al momento).
    Preview { rect: [f32; 4], texts: HashMap<u64, String>, page: Vec<Stroke>, veil: [f32; 4], by: String, secs: u64 },
}

/// Che lavoro si fa sulla zona.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Job {
    /// La bella copia (l'AI legge).
    #[default]
    Fair,
    /// La scrittura sulle righe del foglio.
    Righe,
}

/// La pagina dopo il lavoro: tutti i tratti, dove è cambiata, cosa dire.
struct Outcome {
    page: Vec<Stroke>,
    veil: [f32; 4],
    msg: String,
}

pub(super) struct InBellaUi {
    pub phase: Phase,
    pub job: Job,
    /// Sui quadretti: una riga ogni quanti quadretti (`None` = proposto).
    pub every: Option<u32>,
    /// Il numero della lettura in corso.
    gen: u64,
    shown_secs: u64,
    tx: Sender<Done>,
    rx: Receiver<Done>,
}

impl InBellaUi {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self { phase: Phase::Off, job: Job::Fair, every: None, gen: 0, shown_secs: u64::MAX, tx, rx }
    }

    pub fn active(&self) -> bool {
        !matches!(self.phase, Phase::Off)
    }
}

/// Le azioni della bella copia (dal pulsante, dalla barra, dai tasti).
#[derive(Clone, Debug, PartialEq)]
pub enum Fair {
    /// Comincia (si sceglie la zona), o smette se è già cominciata.
    Toggle,
    /// Tutta la pagina come zona.
    All,
    Accept,
    Reject,
    Reread,
    /// Chi legge (vuoto = gli agenti di «Lettura dei fogli»).
    Agent(String),
    /// Con che scrittura (`campioni::Mode::key`).
    Style(String),
    /// Apre l'addestramento della tua scrittura.
    Train,
    /// «Disponi sulle righe»: comincia (si sceglie la zona) o smette.
    Arrange,
    /// La bella copia va subito sulle righe del foglio (sì/no).
    OnRules(bool),
    /// Sui quadretti: una riga ogni N quadretti.
    Every(u32),
}

impl App {
    pub(super) fn fair_action(&mut self, a: Fair) {
        match a {
            Fair::Toggle | Fair::Arrange => {
                let job = if a == Fair::Arrange { Job::Righe } else { Job::Fair };
                if self.inbella.active() {
                    let other = self.inbella.job != job;
                    self.fair_stop(if self.inbella.job == Job::Righe { "«Disponi sulle righe» annullato." } else { "Bella copia annullata." });
                    if !other {
                        return;
                    }
                }
                if self.docui.doc.is_some() || self.docui.capture.is_some() || self.hub.lock().bg.as_ref().is_some_and(|b| !b.is_chart()) {
                    self.status = if job == Job::Righe { "«Disponi sulle righe» è per la lavagna, non per i documenti." } else { "«Trasforma in bella» è per la lavagna, non per i documenti." }.into();
                } else if job == Job::Righe && !self.paper_has_rows() {
                    self.status = "Per disporre la scrittura sulle righe scegli prima un foglio a righe o a quadretti (Home › Foglio).".into();
                } else {
                    self.inbella.job = job;
                    self.inbella.every = None;
                    self.inbella.phase = Phase::Select { from: None, to: (0.0, 0.0) };
                    self.status = if job == Job::Righe {
                        "Trascina un riquadro attorno alla scrittura da mettere sulle righe (o «Tutta la pagina»). Esc annulla."
                    } else {
                        "Trascina un riquadro attorno a quello che vuoi in bella (o «Tutta la pagina»). Esc annulla."
                    }
                    .into();
                }
            }
            Fair::OnRules(on) => {
                self.settings.fair_on_rules = on;
                self.settings.save();
                self.fair_refresh();
            }
            Fair::Every(n) => {
                self.inbella.every = Some(n.clamp(1, 3));
                self.fair_refresh();
            }
            Fair::All => {
                if matches!(self.inbella.phase, Phase::Select { .. }) {
                    self.fair_read([0.0, 0.0, 1.0, 1.0]);
                }
            }
            Fair::Accept => self.fair_accept(),
            Fair::Reject => match (&self.inbella.phase, self.inbella.job) {
                (Phase::Off, _) => {}
                (Phase::Preview { .. }, Job::Fair) => self.fair_stop("Bella copia scartata: il foglio è com'era."),
                (Phase::Preview { .. }, Job::Righe) => self.fair_stop("Scrittura lasciata com'era."),
                (_, Job::Fair) => self.fair_stop("Bella copia annullata."),
                (_, Job::Righe) => self.fair_stop("«Disponi sulle righe» annullato."),
            },
            Fair::Reread => {
                if let (Phase::Preview { rect, .. }, Job::Fair) = (&self.inbella.phase, self.inbella.job) {
                    let rect = *rect;
                    self.fair_read(rect);
                }
            }
            Fair::Agent(name) => {
                self.settings.fair_agent = name;
                self.settings.save();
            }
            Fair::Style(k) => {
                self.settings.fair_style = k;
                self.settings.save();
                if self.settings.fair_style == "mano" {
                    crate::mano::preload();
                }
            }
            Fair::Train => self.train_open(),
        }
    }

    fn fair_stop(&mut self, msg: &str) {
        self.inbella.phase = Phase::Off;
        self.inbella.gen += 1;
        self.status = msg.into();
    }

    /// Legge la zona `rect`: i tratti col centro dentro.
    fn fair_read(&mut self, rect: [f32; 4]) {
        let (sub, w, h, view, g) = {
            let s = self.hub.lock();
            let (pw, ph) = crate::scene::page_dims(&s.drawing);
            let g = crate::scene::PageGeom::new(s.view, pw, ph);
            let idx = crate::inbella::select(&s.drawing.strokes, &g, rect);
            let sub: Vec<Stroke> = idx.iter().map(|&i| s.drawing.strokes[i].clone()).collect();
            (sub, s.drawing.width, s.drawing.height, s.view, g)
        };
        if sub.is_empty() {
            self.inbella.phase = Phase::Select { from: None, to: (0.0, 0.0) };
            self.status = "Nel riquadro non c'è niente: trascinane un altro (Esc annulla).".into();
            return;
        }
        self.inbella.gen += 1;
        if self.inbella.job == Job::Righe {
            // niente da leggere: subito, sulla pagina com'è
            if self.inbella.every.is_none() {
                let s = self.hub.lock();
                let zone = (rect != [0.0, 0.0, 1.0, 1.0]).then_some(rect);
                self.inbella.every = crate::righe::suggest_every(&s.drawing.strokes, &g, zone, crate::rigatura::Paper::from_code(&self.settings.paper));
            }
            self.fair_preview(rect, HashMap::new(), String::new(), 0);
            return;
        }
        if crate::inbella::pending(&sub, &g).is_empty() {
            // niente da leggere: solo i disegni, subito
            self.fair_preview(rect, HashMap::new(), String::new(), 0);
            return;
        }
        let agent = self.settings.fair_agent.clone();
        let hand = self.settings.fair_style == "mano" && crate::mano::available();
        self.inbella.phase = Phase::Reading { rect, started: Instant::now(), agent: agent.clone() };
        self.inbella.shown_secs = u64::MAX;
        let (tx, gen) = (self.inbella.tx.clone(), self.inbella.gen);
        let _ = std::thread::Builder::new().name("bella-copia".into()).spawn(move || {
            // anche se la lettura si rompe, la risposta arriva (e il pulsante si sblocca)
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let r = crate::inbella::recognize_region(&sub, w, h, view, rect, &agent);
                // con la tua mano: le parole si scrivono qui (un secondo l'una)
                if let (true, Ok((texts, _))) = (hand, &r) {
                    let words: Vec<String> = texts.values().flat_map(|t| crate::mano::words_of(t)).collect();
                    if let Err(e) = crate::mano::warm(&words) {
                        tracing::warn!("la tua mano: {e:#}");
                    }
                }
                r
            }))
            .unwrap_or_else(|_| Err(anyhow::anyhow!("errore interno nella lettura")));
            let _ = tx.send((gen, r));
        });
        self.fair_progress();
    }

    /// Il foglio ha righe su cui scrivere (non è bianco né un pentagramma)?
    fn paper_has_rows(&self) -> bool {
        let p = crate::rigatura::Paper::from_code(&self.settings.paper);
        crate::rigatura::writing_rows(p, 21.0, 29.7) != crate::rigatura::Rows::Unsupported
    }

    fn sheet(&self) -> crate::righe::Sheet {
        crate::righe::Sheet { paper: crate::rigatura::Paper::from_code(&self.settings.paper), margin: self.settings.paper_margin, every: self.inbella.every }
    }

    /// Il lavoro sulla zona, sul foglio com'è adesso: la pagina dopo, dove è
    /// cambiata e cosa dire. `Err` = niente da fare (col perché).
    fn fair_compute(&self, rect: [f32; 4], texts: &HashMap<u64, String>) -> Result<Outcome, String> {
        let s = self.hub.lock();
        let (pw, ph) = crate::scene::page_dims(&s.drawing);
        let g = crate::scene::PageGeom::new(s.view, pw, ph);
        let whole = rect == [0.0, 0.0, 1.0, 1.0];
        let zone = (!whole).then_some(rect);
        if self.inbella.job == Job::Righe {
            return match crate::righe::arrange(&s.drawing.strokes, &g, zone, self.sheet()) {
                Ok(a) => {
                    let veil = if whole { [0.0, 0.0, 1.0, 1.0] } else { crate::boardread::union(rect, a.area) };
                    let msg = righe_status(&a);
                    Ok(Outcome { page: a.strokes, veil, msg })
                }
                Err(e) => Err(righe_problem(e)),
            };
        }
        let idx = crate::inbella::select(&s.drawing.strokes, &g, rect);
        let sub: Vec<Stroke> = idx.iter().map(|&i| s.drawing.strokes[i].clone()).collect();
        let Some((new_sub, done)) = crate::campioni::using(crate::campioni::Mode::parse(&self.settings.fair_style), || crate::inbella::apply_region(&sub, texts, &g, rect[2])) else {
            return Err("Non ho trovato niente da mettere in bella in questa zona (disegni non riconosciuti o scrittura che non si legge). Trascina un altro riquadro, o Esc.".into());
        };
        let mut page: Vec<Stroke> = s.drawing.strokes.iter().enumerate().filter(|(i, _)| !idx.contains(i)).map(|(_, st)| st.clone()).collect();
        let first = page.len();
        page.extend(new_sub);
        let mut msg = inbella_status(done);
        let mut veil = rect;
        if self.settings.fair_on_rules && self.paper_has_rows() {
            let movable: Vec<usize> = (first..page.len()).collect();
            match crate::righe::arrange_only(&page, &g, &movable, zone, self.sheet()) {
                Ok(a) => {
                    veil = if whole { rect } else { crate::boardread::union(rect, a.area) };
                    page = a.strokes;
                }
                Err(e) => msg = format!("{msg} {}", crate::i18n::tf(format!("(Sulle righe no: {})", crate::i18n::tr(&righe_problem(e))))),
            }
        }
        Ok(Outcome { page, veil, msg })
    }

    /// L'anteprima di nuovo (cambiata un'opzione), con quello già letto.
    fn fair_refresh(&mut self) {
        if let Phase::Preview { rect, texts, by, secs, .. } = &self.inbella.phase {
            let (rect, texts, by, secs) = (*rect, texts.clone(), by.clone(), *secs);
            self.fair_preview(rect, texts, by, secs);
        }
    }

    fn fair_preview(&mut self, rect: [f32; 4], texts: HashMap<u64, String>, by: String, secs: u64) {
        match self.fair_compute(rect, &texts) {
            Ok(o) => {
                self.inbella.phase = Phase::Preview { rect, texts, page: o.page, veil: o.veil, by, secs };
                self.status = match self.inbella.job {
                    Job::Fair => "Anteprima della bella copia: Invio accetta, Esc rifiuta, «Rileggi» la rifà.".into(),
                    Job::Righe => crate::i18n::tf(format!("Anteprima: {} Invio accetta, Esc rifiuta.", crate::i18n::tr(&o.msg))),
                };
            }
            Err(why) => {
                self.inbella.phase = Phase::Select { from: None, to: (0.0, 0.0) };
                self.status = why;
            }
        }
    }

    fn fair_accept(&mut self) {
        let Phase::Preview { rect, texts, by, secs, .. } = std::mem::take(&mut self.inbella.phase) else { return };
        let Ok(Outcome { page, msg, .. }) = self.fair_compute(rect, &texts) else {
            self.status = "Il foglio è cambiato e il lavoro non vale più: rifallo.".into();
            return;
        };
        self.hub.edit(|s| {
            let bg = s.bg.clone();
            s.replace_page(page, bg);
        });
        let msg = crate::i18n::tf(msg);
        self.status = if by.is_empty() { msg } else { format!("{msg} {}", crate::i18n::tf(format!("Ha letto {by} in {secs} s."))) };
    }

    /// La barra di stato mentre l'AI legge: chi e da quanti secondi.
    fn fair_progress(&mut self) {
        let Phase::Reading { started, agent, .. } = &self.inbella.phase else { return };
        let secs = started.elapsed().as_secs();
        let hand = crate::mano::progress();
        let key = secs + hand.map_or(0, |(d, _)| (d as u64 + 1) << 32);
        if key == self.inbella.shown_secs {
            return;
        }
        self.inbella.shown_secs = key;
        if let Some((d, n)) = hand {
            self.status = crate::i18n::tf(format!("Bella copia: scrivo con la tua mano… {d}/{n} parole (Esc annulla)"));
            return;
        }
        let who = if agent.is_empty() { crate::i18n::tr("gli agenti di «Lettura dei fogli»").into_owned() } else { agent.clone() };
        self.status = crate::i18n::tf(format!("Bella copia: legge {who}… {secs} s (Esc annulla)"));
    }

    /// A ogni giro: la risposta dell'AI diventa l'anteprima.
    pub(super) fn inbella_tick(&mut self, ctx: &egui::Context) {
        let gen = self.inbella.gen;
        while let Ok((g, r)) = self.inbella.rx.try_recv() {
            if g != gen {
                continue; // una lettura annullata
            }
            let Phase::Reading { rect, started, .. } = &self.inbella.phase else { continue };
            let (rect, secs) = (*rect, started.elapsed().as_secs());
            match r {
                Ok((texts, by)) => self.fair_preview(rect, texts, by, secs),
                Err(e) => {
                    tracing::warn!("bella copia: {e:#}");
                    // i disegni si possono fare lo stesso
                    self.fair_preview(rect, HashMap::new(), String::new(), secs);
                    self.status = format!("⚠ {}", crate::i18n::tf(format!("trasforma in bella: {e:#}")));
                }
            }
            ctx.request_repaint();
        }
        if matches!(self.inbella.phase, Phase::Reading { .. }) {
            self.fair_progress();
            ctx.request_repaint_after(Duration::from_millis(250));
        }
    }

    /// Invio ed Esc durante la bella copia. Ritorna true se il tasto è suo.
    pub(super) fn fair_keys(&self, ctx: &egui::Context, actions: &mut Vec<super::Action>) -> bool {
        if !self.inbella.active() {
            return false;
        }
        ctx.input_mut(|i| {
            if i.consume_key(egui::Modifiers::NONE, egui::Key::Escape) {
                actions.push(super::Action::Fair(Fair::Reject));
                return true;
            }
            if matches!(self.inbella.phase, Phase::Preview { .. }) && i.consume_key(egui::Modifiers::NONE, egui::Key::Enter) {
                actions.push(super::Action::Fair(Fair::Accept));
                return true;
            }
            false
        })
    }

    /// Il mouse sul foglio mentre si sceglie la zona: si trascina un
    /// riquadro. Ritorna true se l'input è suo (allora la pagina non si
    /// sposta né si ingrandisce).
    pub(super) fn fair_input(&mut self, ui: &egui::Ui, resp: &egui::Response) -> bool {
        let page = self.docui.last_page;
        let mut chosen: Option<[f32; 4]> = None;
        let mine = match &mut self.inbella.phase {
            Phase::Select { from, to } => {
                if page.is_positive() {
                    let uv = |q: egui::Pos2| (((q.x - page.left()) / page.width()).clamp(0.0, 1.0), ((q.y - page.top()) / page.height()).clamp(0.0, 1.0));
                    if resp.hover_pos().is_some() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                    }
                    if resp.drag_started() {
                        let o = ui.input(|i| i.pointer.press_origin()).or(resp.interact_pointer_pos());
                        *from = o.map(uv);
                    }
                    if let (Some(_), Some(p)) = (*from, resp.interact_pointer_pos()) {
                        *to = uv(p);
                    }
                    if resp.drag_stopped() {
                        if let Some(a) = from.take() {
                            let b = *to;
                            let r = [a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1)];
                            if r[2] - r[0] > 0.01 && r[3] - r[1] > 0.01 {
                                chosen = Some(r);
                            }
                        }
                    }
                }
                true
            }
            // durante la lettura e l'anteprima il foglio sta fermo
            Phase::Reading { .. } | Phase::Preview { .. } => true,
            Phase::Off => false,
        };
        if let Some(r) = chosen {
            self.fair_read(r);
        }
        mine
    }

    /// Sopra il foglio: il riquadro che si sta trascinando, la zona in
    /// lettura, l'anteprima (la zona velata e sopra la bella copia).
    pub(super) fn fair_paint(&self, painter: &egui::Painter, page: egui::Rect, g: &crate::scene::PageGeom) {
        let paper = crate::rigatura::Paper::from_code(&self.settings.paper);
        let accent = egui::Color32::from_rgb(40, 110, 220);
        let to_screen = |r: [f32; 4]| egui::Rect::from_min_max(egui::pos2(page.left() + r[0] * page.width(), page.top() + r[1] * page.height()), egui::pos2(page.left() + r[2] * page.width(), page.top() + r[3] * page.height()));
        let dashed = |r: egui::Rect| {
            let st = egui::Stroke::new(1.5, accent);
            let pts = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
            for w in pts.windows(2) {
                painter.extend(egui::Shape::dashed_line(&[w[0], w[1]], st, 6.0, 4.0));
            }
        };
        match &self.inbella.phase {
            Phase::Select { from: Some(a), to } => {
                let r = to_screen([a.0.min(to.0), a.1.min(to.1), a.0.max(to.0), a.1.max(to.1)]);
                painter.rect_filled(r, 0.0, egui::Color32::from_rgba_unmultiplied(40, 110, 220, 22));
                dashed(r);
            }
            Phase::Reading { rect, .. } => {
                let r = to_screen(*rect);
                painter.rect_filled(r, 0.0, egui::Color32::from_rgba_unmultiplied(40, 110, 220, 18));
                dashed(r);
            }
            Phase::Preview { page: after, veil, .. } => {
                let r = to_screen(*veil).expand(4.0).intersect(page);
                // la scrittura di prima appena visibile sotto, le righe del
                // foglio, e sopra la pagina com'è dopo
                painter.rect_filled(r, 0.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 225));
                let clip = painter.with_clip_rect(r);
                super::paper::paint(&clip, page, (g.w / 1000.0, g.h / 1000.0), paper, self.settings.paper_margin);
                for s in after {
                    if s.points.len() < 2 {
                        continue;
                    }
                    let pts: Vec<egui::Pos2> = s
                        .points
                        .iter()
                        .map(|p| {
                            let (u, v) = g.unit(p.x, p.y);
                            egui::pos2(page.left() + u * page.width(), page.top() + v * page.height())
                        })
                        .collect();
                    let width = (s.width * page.height() / 800.0).clamp(0.8, 160.0);
                    let col = if s.highlight { egui::Color32::from_rgba_unmultiplied(s.color[0], s.color[1], s.color[2], 120) } else { egui::Color32::from_rgb(s.color[0], s.color[1], s.color[2]) };
                    clip.add(egui::Shape::line(pts, egui::Stroke::new(width, col)));
                }
                dashed(r);
            }
            _ => {}
        }
    }
}

fn inbella_status(d: crate::inbella::Done) -> String {
    let (l, f) = (d.lines, d.shapes);
    if l > 0 && f > 0 {
        crate::i18n::tf(format!("{l} righe e {f} forme in bella: Annulla riporta la tua scrittura."))
    } else if l > 0 {
        crate::i18n::tf(format!("{l} righe trasformate in bella: Annulla riporta la tua scrittura."))
    } else {
        crate::i18n::tf(format!("{f} forme rifatte perfette: Annulla riporta i tuoi tratti."))
    }
}

/// Com'è andata «Disponi sulle righe».
fn righe_status(a: &crate::righe::Arranged) -> String {
    let n = a.lines;
    let mut m = crate::i18n::tf(format!("{n} righe sulle righe del foglio."));
    if let Some(e) = a.every {
        m = format!("{m} {}", crate::i18n::tf(format!("Una riga ogni {e} quadretti.")));
    }
    if a.shrunk > 0 {
        let k = a.shrunk;
        m = format!("{m} {}", crate::i18n::tf(format!("{k} rimpicciolite perché più alte della riga.")));
    }
    m
}

fn righe_problem(p: crate::righe::Problem) -> String {
    use crate::righe::Problem;
    match p {
        Problem::Nothing => "Nella zona non c'è scrittura da mettere sulle righe: trascina un altro riquadro, o Esc.".into(),
        Problem::Unsupported => "Per disporre la scrittura sulle righe scegli prima un foglio a righe o a quadretti (Home › Foglio).".into(),
        Problem::NoRoom { fit, total } => crate::i18n::tf(format!("Non c'è posto per tutte le righe ({fit} su {total}): scegli una zona più grande o meno quadretti per riga.")),
    }
}
