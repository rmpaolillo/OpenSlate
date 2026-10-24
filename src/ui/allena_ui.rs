//! La finestra «Addestra la tua scrittura» (vedi `crate::campioni`): a
//! turni, le righe da scrivere col pennino; dopo ogni turno i segni
//! ritagliati, da tenere o scartare; alla fine l'anteprima (e, se vuoi, la
//! prova di lettura dell'AI) e «Va bene così».

use super::App;
use crate::campioni::{self, Mode, Round, RoundRead, Row, Store, Style};
use eframe::egui;
use std::collections::HashSet;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

/// Le righe dell'anteprima.
const PREVIEW: [&str; 3] = ["Studiare la funzione $f(x) = \\frac{x^2-1}{x}$", "Il dominio è $x \\neq 0$ e la funzione è dispari", "Quindi $\\lim_{x \\to +\\infty} f(x) = +\\infty$"];
/// La frase che l'AI prova a leggere.
const AI_TEXT: &str = "La funzione è continua e derivabile in tutto il suo dominio";
const AI_PROMPT: &str = "Trascrivi esattamente il testo scritto a mano in questa immagine. Rispondi solo con il testo, senza commenti.";

#[derive(Default)]
enum Phase {
    #[default]
    Closed,
    Start,
    Writing(Round),
    Review(Round, RoundRead, HashSet<(usize, usize)>),
    Preview,
}

enum AiCheck {
    Idle,
    Busy,
    Done { read: String, score: f32 },
    Failed(String),
}

pub(super) struct AllenaUi {
    phase: Phase,
    style: Style,
    store: Store,
    /// Versioni in più chieste con «Un altro turno».
    extra: usize,
    msg: String,
    ai: AiCheck,
    gen: u64,
    tx: Sender<(u64, Result<String, String>)>,
    rx: Receiver<(u64, Result<String, String>)>,
}

impl Default for AllenaUi {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self { phase: Phase::Closed, style: Style::Corsivo, store: Store::default(), extra: 0, msg: String::new(), ai: AiCheck::Idle, gen: 0, tx, rx }
    }
}

/// Quello che si è cliccato nella finestra (si fa dopo averla disegnata).
enum Do {
    Style(Style),
    Begin,
    Clear,
    Read,
    Toggle(usize, usize),
    Keep,
    Redo,
    Preview,
    AskAi,
    Done,
    More,
    Back,
}

/// Disegna dei tratti (y verso il basso) dentro `rect`, centrati e in scala.
fn draw_ink(painter: &egui::Painter, rect: egui::Rect, ink: &[Vec<(f32, f32)>], color: egui::Color32, width: f32) {
    let pts: Vec<(f32, f32)> = ink.iter().flatten().cloned().collect();
    if pts.is_empty() {
        return;
    }
    let (x0, x1, y0, y1) = pts.iter().fold((f32::MAX, f32::MIN, f32::MAX, f32::MIN), |b, p| (b.0.min(p.0), b.1.max(p.0), b.2.min(p.1), b.3.max(p.1)));
    let k = (rect.width() / (x1 - x0).max(1e-3)).min(rect.height() / (y1 - y0).max(1e-3));
    let (ox, oy) = (rect.center().x - (x0 + x1) / 2.0 * k, rect.center().y - (y0 + y1) / 2.0 * k);
    for st in ink {
        let line: Vec<egui::Pos2> = st.iter().map(|p| egui::pos2(ox + p.0 * k, oy + p.1 * k)).collect();
        if line.len() == 1 {
            painter.circle_filled(line[0], width / 2.0, color);
        } else {
            painter.add(egui::Shape::line(line, egui::Stroke::new(width, color)));
        }
    }
}

/// Quanto si somigliano due testi (0..1): distanza di modifica sui
/// caratteri, senza maiuscole e spazi doppi.
fn similarity(a: &str, b: &str) -> f32 {
    let norm = |s: &str| s.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ").chars().collect::<Vec<char>>();
    let (a, b) = (norm(a), norm(b));
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            cur[j + 1] = (prev[j] + usize::from(ca != cb)).min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        prev = cur;
    }
    1.0 - prev[b.len()] as f32 / a.len().max(b.len()) as f32
}

/// La frase dell'AI scritta coi tuoi segni, in un PNG.
fn ai_png(style: Style, store: &Store) -> Option<Vec<u8>> {
    let b = campioni::using_store(Mode::Scrittura(style), Some((style, store.clone())), || crate::mathhand::text_box(crate::handwrite::Style::Naturale, AI_TEXT));
    let pts: Vec<(f32, f32)> = b.strokes.iter().flatten().cloned().collect();
    let (x0, x1, y0, y1) = pts.iter().fold((f32::MAX, f32::MIN, f32::MAX, f32::MIN), |b, p| (b.0.min(p.0), b.1.max(p.0), b.2.min(p.1), b.3.max(p.1)));
    if x0 > x1 {
        return None;
    }
    let s = 4.0;
    let pad = 40.0;
    let mut pm = tiny_skia::Pixmap::new(((x1 - x0) * s + 2.0 * pad) as u32, ((y1 - y0) * s + 2.0 * pad) as u32)?;
    pm.fill(tiny_skia::Color::WHITE);
    let mut paint = tiny_skia::Paint::default();
    paint.set_color_rgba8(20, 20, 30, 255);
    paint.anti_alias = true;
    let stroke = tiny_skia::Stroke { width: 4.0, line_cap: tiny_skia::LineCap::Round, line_join: tiny_skia::LineJoin::Round, ..Default::default() };
    for st in &b.strokes {
        let mut pb = tiny_skia::PathBuilder::new();
        let p = |q: &(f32, f32)| (pad + (q.0 - x0) * s, pad + (q.1 - y0) * s);
        let (a, c) = p(&st[0]);
        pb.move_to(a, c);
        for q in &st[1..] {
            let (a, c) = p(q);
            pb.line_to(a, c);
        }
        if st.len() == 1 {
            pb.line_to(a + 0.5, c);
        }
        if let Some(path) = pb.finish() {
            pm.stroke_path(&path, &paint, &stroke, tiny_skia::Transform::identity(), None);
        }
    }
    pm.encode_png().ok()
}

impl App {
    /// Apre l'addestramento: si continua lo stile già cominciato.
    pub(super) fn train_open(&mut self) {
        let a = &mut self.allena;
        a.style = Style::ALL.into_iter().find(|&s| { let st = campioni::load(s); !st.samples.is_empty() && !st.done }).unwrap_or(match Mode::parse(&self.settings.fair_style) {
            Mode::Scrittura(s) => s,
            _ => Style::Corsivo,
        });
        a.store = campioni::start(a.style);
        a.extra = 0;
        a.msg.clear();
        a.ai = AiCheck::Idle;
        a.phase = Phase::Start;
    }

    /// I tratti della lavagna (penna, non evidenziatore né segni dell'AI),
    /// nelle unità della pagina.
    fn train_ink(&self) -> Vec<Vec<(f32, f32)>> {
        let s = self.hub.lock();
        let (pw, ph) = crate::scene::page_dims(&s.drawing);
        let g = crate::scene::PageGeom::new(s.view, pw, ph);
        let aspect = g.w / g.h;
        s.drawing
            .strokes
            .iter()
            .filter(|st| !st.highlight && !st.points.is_empty() && !crate::boardagent::is_ai_stroke(st))
            .map(|st| {
                st.points
                    .iter()
                    .map(|q| {
                        let (u, v) = g.unit(q.x, q.y);
                        (u * aspect, v)
                    })
                    .collect()
            })
            .collect()
    }

    fn train_clear(&mut self) {
        self.hub.edit(|s| {
            let bg = s.bg.clone();
            s.replace_page(Vec::new(), bg);
        });
        self.status = "Lavagna pulita per il prossimo turno (Annulla la riporta).".into();
    }

    /// Il prossimo turno, o l'anteprima se non manca niente.
    fn train_next(&mut self) {
        let a = &mut self.allena;
        match campioni::next_round(&a.store, a.extra) {
            Some(r) => a.phase = Phase::Writing(r),
            None => a.phase = Phase::Preview,
        }
    }

    pub(super) fn train_window(&mut self, ctx: &egui::Context) {
        if matches!(self.allena.phase, Phase::Closed) {
            return;
        }
        while let Ok((g, r)) = self.allena.rx.try_recv() {
            if g == self.allena.gen {
                self.allena.ai = match r {
                    Ok(read) => {
                        let score = similarity(&read, AI_TEXT);
                        AiCheck::Done { read, score }
                    }
                    Err(e) => AiCheck::Failed(e),
                };
            }
        }
        if matches!(self.allena.ai, AiCheck::Busy) {
            ctx.request_repaint_after(std::time::Duration::from_millis(300));
        }
        let mut open = true;
        let mut todo: Vec<Do> = Vec::new();
        let board_empty = self.hub.lock().drawing.strokes.is_empty();
        let a = &self.allena;
        let tr = |s: &str| crate::i18n::tr(s).into_owned();
        egui::Window::new(crate::i18n::t("Addestra la tua scrittura"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(440.0)
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-16.0, 170.0))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().max_height((ctx.content_rect().height() - 40.0).max(300.0)).show(ui, |ui| {
                    ui.set_width(430.0);
                    match &a.phase {
                        Phase::Closed => {}
                        Phase::Start => {
                            ui.label(tr("Insegna a OpenSlate la tua scrittura: a turni ti chiede di scrivere col pennino lettere, cifre e simboli, poi tre frasi. Con i tuoi segni la bella copia sembra scritta da te. Ci vuole circa un quarto d'ora, e puoi smettere e riprendere quando vuoi."));
                            ui.add_space(8.0);
                            ui.label(tr("Che stile vuoi addestrare?"));
                            for st in Style::ALL {
                                let pct = (campioni::load(st).coverage() * 100.0).round() as u32;
                                let label = format!("{} — {}", tr(st.label()), crate::i18n::tf(format!("{pct}% fatto")));
                                if ui.radio(a.style == st, label).clicked() {
                                    todo.push(Do::Style(st));
                                }
                            }
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                let label = if a.store.rounds == 0 { tr("Comincia") } else { tr("Continua") };
                                if ui.button(label).clicked() {
                                    todo.push(Do::Begin);
                                }
                                if a.store.rounds > 0 && ui.button(tr("Anteprima")).clicked() {
                                    todo.push(Do::Preview);
                                }
                            });
                        }
                        Phase::Writing(r) => {
                            ui.heading(crate::i18n::tf(format!("Turno {}", r.number)));
                            ui.label(tr("Su una pagina vuota scrivi col pennino queste righe, una sotto l'altra e ben staccate; i segni di una riga con un po' di spazio fra l'uno e l'altro. Poi «Fatto»."));
                            ui.add_space(6.0);
                            egui::Frame::group(ui.style()).show(ui, |ui| {
                                ui.set_width(410.0);
                                for row in &r.rows {
                                    ui.label(egui::RichText::new(row.text()).size(18.0));
                                }
                            });
                            ui.add_space(6.0);
                            ui.add(egui::ProgressBar::new(a.store.coverage()).show_percentage().desired_width(410.0));
                            if !board_empty && a.msg.is_empty() {
                                ui.weak(tr("Quando hai scritto tutte le righe, premi «Fatto». Se sulla lavagna c'è altro, puliscila prima."));
                            }
                            ui.horizontal(|ui| {
                                if ui.button(tr("Fatto: leggi la pagina")).clicked() {
                                    todo.push(Do::Read);
                                }
                                if !board_empty && ui.button(tr("Pulisci la lavagna")).clicked() {
                                    todo.push(Do::Clear);
                                }
                                if a.store.rounds > 0 && ui.button(tr("Anteprima")).clicked() {
                                    todo.push(Do::Preview);
                                }
                            });
                        }
                        Phase::Review(_, read, dropped) => {
                            ui.label(tr("Ecco i segni che ho ritagliato. Un clic su un segno lo scarta (o lo riprende); quelli col bordo arancione sembrano diversi dalle altre versioni."));
                            ui.add_space(6.0);
                            for (r, row) in read.rows.iter().enumerate() {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(egui::RichText::new(format!("{}", r + 1)).weak());
                                    if let Some(e) = &row.error {
                                        ui.colored_label(egui::Color32::from_rgb(200, 60, 50), crate::i18n::tf(e.clone()));
                                        return;
                                    }
                                    if let Row::Phrase(p) = &row.row {
                                        ui.label(format!("✓ {p}"));
                                        return;
                                    }
                                    for (k, s) in row.samples.iter().enumerate() {
                                        let (rect, resp) = ui.allocate_exact_size(egui::vec2(38.0, 50.0), egui::Sense::click());
                                        let p = ui.painter();
                                        let off = dropped.contains(&(r, k));
                                        let odd = row.odd.get(k).copied().unwrap_or(false);
                                        p.rect_filled(rect.shrink(1.0), 4.0, egui::Color32::WHITE);
                                        let border = if off {
                                            egui::Stroke::new(1.0, egui::Color32::from_gray(180))
                                        } else if odd {
                                            egui::Stroke::new(2.0, egui::Color32::from_rgb(230, 140, 20))
                                        } else {
                                            egui::Stroke::new(1.0, egui::Color32::from_gray(200))
                                        };
                                        p.rect_stroke(rect.shrink(1.0), 4.0, border, egui::StrokeKind::Inside);
                                        let ink: Vec<Vec<(f32, f32)>> = s.strokes.iter().map(|st| st.iter().map(|q| (q[0], q[1])).collect()).collect();
                                        let col = if off { egui::Color32::from_gray(200) } else { egui::Color32::from_rgb(20, 30, 60) };
                                        draw_ink(p, egui::Rect::from_min_size(rect.min + egui::vec2(5.0, 5.0), egui::vec2(28.0, 28.0)), &ink, col, 1.6);
                                        p.text(egui::pos2(rect.center().x, rect.bottom() - 9.0), egui::Align2::CENTER_CENTER, s.c, egui::FontId::proportional(11.0), egui::Color32::from_gray(110));
                                        if off {
                                            let q = rect.shrink(9.0);
                                            let red = egui::Stroke::new(2.0, egui::Color32::from_rgb(200, 60, 50));
                                            p.line_segment([q.left_top(), q.right_bottom()], red);
                                            p.line_segment([q.right_top(), q.left_bottom()], red);
                                        }
                                        let resp = if odd { resp.on_hover_text(tr("Diverso dalle altre versioni: un clic lo scarta")) } else { resp };
                                        if resp.clicked() {
                                            todo.push(Do::Toggle(r, k));
                                        }
                                    }
                                });
                            }
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                if ui.button(tr("Tieni e vai avanti")).clicked() {
                                    todo.push(Do::Keep);
                                }
                                if ui.button(tr("Rifai il turno")).clicked() {
                                    todo.push(Do::Redo);
                                }
                            });
                        }
                        Phase::Preview => {
                            ui.label(tr("Così scrive la bella copia con i tuoi segni:"));
                            let boxes: Vec<crate::mathhand::MBox> = campioni::using_store(Mode::Scrittura(a.style), Some((a.style, a.store.clone())), || {
                                PREVIEW.iter().map(|t| crate::mathhand::line_box(crate::handwrite::Style::Naturale, t)).collect()
                            });
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(420.0, 56.0 * boxes.len() as f32 + 12.0), egui::Sense::hover());
                            let p = ui.painter();
                            p.rect_filled(rect, 6.0, egui::Color32::WHITE);
                            let wmax = boxes.iter().map(|b| b.w).fold(1.0f32, f32::max);
                            let k = (400.0 / wmax).min(1.7);
                            for (i, b) in boxes.iter().enumerate() {
                                let base = egui::pos2(rect.left() + 10.0, rect.top() + 40.0 + 56.0 * i as f32);
                                for st in &b.strokes {
                                    let line: Vec<egui::Pos2> = st.iter().map(|q| egui::pos2(base.x + q.0 * k, base.y + q.1 * k)).collect();
                                    if line.len() > 1 {
                                        p.add(egui::Shape::line(line, egui::Stroke::new(1.6, egui::Color32::from_rgb(20, 30, 60))));
                                    }
                                }
                            }
                            ui.add_space(6.0);
                            ui.add(egui::ProgressBar::new(a.store.coverage()).show_percentage().desired_width(410.0));
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                let busy = matches!(a.ai, AiCheck::Busy);
                                if ui.add_enabled(!busy, egui::Button::new(tr("Fai leggere all'AI"))).on_hover_text(tr("L'AI prova a leggere una frase scritta coi tuoi segni: se la legge giusta, la scrittura è leggibile")).clicked() {
                                    todo.push(Do::AskAi);
                                }
                                match &a.ai {
                                    AiCheck::Idle => {}
                                    AiCheck::Busy => {
                                        ui.spinner();
                                    }
                                    AiCheck::Done { score, .. } => {
                                        let pct = (score * 100.0).round() as u32;
                                        let col = if *score >= 0.9 { egui::Color32::from_rgb(40, 150, 70) } else { egui::Color32::from_rgb(220, 130, 20) };
                                        ui.colored_label(col, crate::i18n::tf(format!("{pct}% giusto")));
                                    }
                                    AiCheck::Failed(e) => {
                                        ui.colored_label(egui::Color32::from_rgb(200, 60, 50), crate::i18n::tf(e.clone()));
                                    }
                                }
                            });
                            if let AiCheck::Done { read, .. } = &a.ai {
                                ui.weak(crate::i18n::tf(format!("Scritto: «{AI_TEXT}» — letto: «{read}»")));
                            }
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                if ui.button(tr("Va bene così")).clicked() {
                                    todo.push(Do::Done);
                                }
                                if ui.button(tr("Un altro turno")).on_hover_text(tr("Altre versioni delle lettere, per più varietà")).clicked() {
                                    todo.push(Do::More);
                                }
                                if ui.button(tr("Indietro")).clicked() {
                                    todo.push(Do::Back);
                                }
                            });
                        }
                    }
                    if !a.msg.is_empty() {
                        ui.add_space(6.0);
                        ui.colored_label(egui::Color32::from_rgb(200, 90, 40), &a.msg);
                    }
                });
            });
        if !open {
            self.allena.phase = Phase::Closed;
            return;
        }
        for d in todo {
            self.train_do(d);
        }
    }

    fn train_do(&mut self, d: Do) {
        match d {
            Do::Style(st) => {
                self.allena.style = st;
                self.allena.store = campioni::start(st);
                self.allena.extra = 0;
            }
            Do::Begin => {
                self.allena.msg.clear();
                self.train_next();
            }
            Do::Clear => self.train_clear(),
            Do::Read => {
                let Phase::Writing(round) = &self.allena.phase else { return };
                let round = round.clone();
                let ink = self.train_ink();
                if ink.is_empty() {
                    self.allena.msg = crate::i18n::tr("La pagina è vuota: scrivi le righe col pennino, poi «Fatto».").into_owned();
                    return;
                }
                let read = campioni::read_round(&round, &ink, &self.allena.store);
                if let Some(e) = &read.error {
                    self.allena.msg = crate::i18n::tf(e.clone());
                    return;
                }
                self.allena.msg.clear();
                self.allena.phase = Phase::Review(round, read, HashSet::new());
            }
            Do::Toggle(r, k) => {
                if let Phase::Review(_, _, dropped) = &mut self.allena.phase {
                    if !dropped.remove(&(r, k)) {
                        dropped.insert((r, k));
                    }
                }
            }
            Do::Keep => {
                let Phase::Review(_, read, dropped) = std::mem::take(&mut self.allena.phase) else { return };
                let a = &mut self.allena;
                campioni::commit(&mut a.store, &read, &dropped);
                if let Err(e) = campioni::save(a.style, &a.store) {
                    a.msg = format!("⚠ {e:#}");
                }
                // dove c'è la rete, la prima frase le dà lo stile
                if let Some(pr) = &read.prime {
                    if crate::mano::available() {
                        if let Err(e) = crate::mano::save_prime(pr, campioni::PHRASES[0]) {
                            tracing::warn!("la tua mano: {e:#}");
                        }
                    }
                }
                let bad = read.rows.iter().filter(|r| r.error.is_some()).count();
                a.msg = if bad > 0 { crate::i18n::tf(format!("{bad} righe da rifare: le trovi nel prossimo turno.")) } else { String::new() };
                self.train_clear();
                self.train_next();
            }
            Do::Redo => {
                let Phase::Review(round, ..) = std::mem::take(&mut self.allena.phase) else { return };
                self.train_clear();
                self.allena.phase = Phase::Writing(round);
            }
            Do::Preview => {
                self.allena.msg.clear();
                self.allena.phase = Phase::Preview;
            }
            Do::AskAi => {
                let a = &mut self.allena;
                let Some(png) = ai_png(a.style, &a.store) else { return };
                a.gen += 1;
                a.ai = AiCheck::Busy;
                let (tx, gen, agent) = (a.tx.clone(), a.gen, self.settings.fair_agent.clone());
                let _ = std::thread::Builder::new().name("prova-lettura".into()).spawn(move || {
                    let r = crate::ai::read_image_by(Arc::new(png), AI_PROMPT, &agent).map(|r| r.text.trim().trim_matches('"').to_string()).map_err(|e| format!("{e:#}"));
                    let _ = tx.send((gen, r));
                });
            }
            Do::Done => {
                let a = &mut self.allena;
                a.store.done = true;
                if let Err(e) = campioni::save(a.style, &a.store) {
                    a.msg = format!("⚠ {e:#}");
                    return;
                }
                self.settings.fair_style = a.style.key().into();
                self.settings.save();
                a.phase = Phase::Closed;
                self.status = "La tua scrittura è pronta: la bella copia la usa (si cambia dal menu «Scrittura»).".into();
            }
            Do::More => {
                self.allena.extra += 1;
                match campioni::next_round(&self.allena.store, self.allena.extra) {
                    Some(r) => self.allena.phase = Phase::Writing(r),
                    None => self.allena.msg = crate::i18n::tr("Non manca niente.").into_owned(),
                }
            }
            Do::Back => self.allena.phase = Phase::Start,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn somiglianza() {
        assert!((similarity("Ciao  mondo", "ciao mondo") - 1.0).abs() < 1e-6);
        assert!(similarity("la funzione", "la funzone") > 0.85);
        assert!(similarity("abc", "xyz") < 0.1);
    }
}
