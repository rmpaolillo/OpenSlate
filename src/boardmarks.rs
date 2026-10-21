//! I segni che la chat fa sul foglio della lavagna: evidenziare,
//! sottolineare, cerchiare, riquadrare, barrare, frecce, linee, spunte, croci
//! e testo a mano. Diventano tratti come quelli del pennino (un po' irregolari,
//! come fatti a mano), quindi si salvano, si esportano e si cancellano come
//! gli altri.
//!
//! Qui solo la geometria: dove vanno e che forma hanno, in unità della pagina
//! (u, v: 0..1 di larghezza e altezza). Si lavora in unità «fisiche»
//! (x × larghezza/altezza, y) perché un cerchio resti tondo.

use crate::boardread::{union, Reading};
use crate::handwrite::{self, Style};
use crate::stroke::Rgb;
use anyhow::{bail, Context, Result};
use serde_json::Value;

/// Un tratto da aggiungere.
#[derive(Clone, Debug)]
pub struct Mark {
    pub pts: Vec<(f32, f32)>,
    pub color: Rgb,
    /// Spessore come quello dei tratti (pixel su un foglio alto 800).
    pub width: f32,
    pub highlight: bool,
}

pub const RED: Rgb = [214, 40, 40];

/// Un colore detto a parole (o #rrggbb).
pub fn parse_color(s: &str, highlight: bool) -> Option<Rgb> {
    let s = s.trim().to_lowercase();
    if let Some(h) = s.strip_prefix('#') {
        let v = u32::from_str_radix(h, 16).ok()?;
        return match h.len() {
            6 => Some([(v >> 16) as u8, (v >> 8) as u8, v as u8]),
            3 => Some([((v >> 8) & 15) as u8 * 17, ((v >> 4) & 15) as u8 * 17, (v & 15) as u8 * 17]),
            _ => None,
        };
    }
    // l'evidenziatore ha i suoi colori, chiari
    let (ink, hl): (Rgb, Rgb) = match s.as_str() {
        "rosso" | "red" | "rossa" => (RED, [255, 140, 140]),
        "blu" | "blue" => ([30, 80, 200], [140, 190, 255]),
        "azzurro" | "celeste" | "light blue" | "cyan" => ([20, 150, 220], [150, 225, 255]),
        "verde" | "green" => ([30, 140, 60], [150, 235, 150]),
        "giallo" | "yellow" | "gialla" => ([220, 170, 0], [255, 235, 90]),
        "arancione" | "arancio" | "orange" => ([235, 120, 20], [255, 195, 120]),
        "viola" | "purple" | "violet" => ([130, 50, 180], [210, 170, 255]),
        "rosa" | "pink" | "fucsia" | "magenta" => ([220, 40, 140], [255, 170, 220]),
        "nero" | "black" | "nera" => ([20, 20, 20], [190, 190, 190]),
        "grigio" | "gray" | "grey" => ([110, 110, 110], [210, 210, 210]),
        "marrone" | "brown" => ([120, 70, 30], [220, 190, 150]),
        _ => return None,
    };
    Some(if highlight { hl } else { ink })
}

/// Piccolo generatore ripetibile, per l'aria fatta a mano.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        ((self.0 >> 11) as f32 / (1u64 << 53) as f32) * 2.0 - 1.0
    }
}

/// Contesto: com'è la pagina e com'è la penna.
pub struct Ctx<'a> {
    pub reading: &'a Reading,
    pub aspect: f32,
    pub pen_width: f32,
    pub highlighter: Rgb,
    /// I testi già messi in questa volta (per non scriverci sopra).
    pub placed: std::cell::RefCell<Vec<[f32; 4]>>,
}

impl Ctx<'_> {
    pub fn new(reading: &Reading, aspect: f32, pen_width: f32, highlighter: Rgb) -> Ctx<'_> {
        Ctx { reading, aspect, pen_width, highlighter, placed: Default::default() }
    }

    /// Il riquadro tocca lo scritto (le parole, il disegno, il grafico, i
    /// testi già messi)? `except`: il pezzo accanto a cui si scrive.
    fn collides(&self, b: [f32; 4], except: Option<[f32; 4]>) -> bool {
        let m = 0.1 * self.h();
        let hit = |o: [f32; 4]| b[0] < o[2] - m / self.aspect && o[0] + m / self.aspect < b[2] && b[1] < o[3] - m && o[1] + m < b[3];
        let r = self.reading;
        let words = r.lines.iter().flat_map(|l| l.words.iter().map(|w| w.bbox));
        let placed = self.placed.borrow();
        let others = r.drawing.iter().map(|d| d.bbox).chain(r.chart).chain(placed.iter().copied());
        words.chain(others).any(|o| Some(o) != except && hit(o))
    }

    /// Sotto tutto quello che c'è, a sinistra dove comincia lo scritto.
    fn below_all(&self, lay: &dyn Fn((f32, f32), f32, bool, f32) -> handwrite::Laid, cap: f32) -> Result<handwrite::Laid> {
        let h = self.h();
        let all = self.reading.find("tutto").into_iter().chain(self.placed.borrow().iter().copied()).reduce(union);
        let x = all.map_or(0.08, |b| b[0]).clamp(0.03, 0.6);
        let y = all.map_or(0.08, |b| b[3] + 0.8 * h);
        let l = lay((x, y), 0.94 - x, false, cap);
        if l.bbox[3] > 0.99 {
            bail!("sul foglio non c'è più posto per il testo: scrivi più piccolo («altezza»), dentro un «riquadro» libero, o chiedi all'utente una pagina nuova");
        }
        Ok(l)
    }

    fn h(&self) -> f32 {
        self.reading.char_h
    }
    /// Unità della pagina → fisiche.
    fn p(&self, (u, v): (f32, f32)) -> (f32, f32) {
        (u * self.aspect, v)
    }
    fn back(&self, pts: Vec<(f32, f32)>) -> Vec<(f32, f32)> {
        pts.into_iter().map(|(x, y)| ((x / self.aspect).clamp(0.003, 0.997), y.clamp(0.003, 0.997))).collect()
    }
    fn pbox(&self, b: [f32; 4]) -> [f32; 4] {
        [b[0] * self.aspect, b[1], b[2] * self.aspect, b[3]]
    }
}

/// Dove: un nome (`r2p3`), più nomi (uniti), un riquadro [x1, y1, x2, y2]
/// (0–1000) o un punto [x, y] (diventa un riquadro piccolo).
pub fn target(r: &Reading, v: &Value) -> Result<Option<[f32; 4]>> {
    match v {
        Value::Null => Ok(None),
        Value::String(s) => {
            // «r2p1-r2p3» o «r2p1-p3»: da una parola all'altra
            if let Some((a, b)) = s.split_once('-').filter(|(a, b)| !a.is_empty() && !b.is_empty()) {
                let b = if b.starts_with('p') { format!("{}{b}", a.split('p').next().unwrap_or("")) } else { b.to_string() };
                let (ra, rb) = (r.find(a), r.find(&b));
                return match (ra, rb) {
                    (Some(x), Some(y)) => Ok(Some(union(x, y))),
                    _ => bail!("non trovo «{s}» sul foglio: leggi prima il foglio con lavagna_leggi"),
                };
            }
            r.find(s).map(Some).with_context(|| format!("non trovo «{s}» sul foglio: leggi prima il foglio con lavagna_leggi"))
        }
        Value::Array(a) if a.iter().all(|x| x.is_string()) => {
            let mut out: Option<[f32; 4]> = None;
            for x in a {
                let b = target(r, x)?.unwrap();
                out = Some(out.map_or(b, |o| union(o, b)));
            }
            Ok(out)
        }
        Value::Array(a) => {
            let n: Vec<f32> = a.iter().filter_map(|x| x.as_f64().map(|x| x as f32)).collect();
            let s = if n.iter().all(|&v| v <= 1.0) { 1.0 } else { 1000.0 };
            match n.len() {
                4 => {
                    let n: Vec<f32> = n.iter().map(|v| (v / s).clamp(0.0, 1.0)).collect();
                    Ok(Some([n[0].min(n[2]), n[1].min(n[3]), n[0].max(n[2]), n[1].max(n[3])]))
                }
                2 => {
                    let (u, v) = ((n[0] / s).clamp(0.0, 1.0), (n[1] / s).clamp(0.0, 1.0));
                    Ok(Some([u, v, u, v]))
                }
                _ => bail!("un riquadro è [x1, y1, x2, y2] e un punto [x, y], in 0–1000"),
            }
        }
        _ => bail!("posizione non valida: usa un nome del foglio (r2p3) o [x1, y1, x2, y2]"),
    }
}

/// Il segno descritto da `m` (un elemento di `segni`).
pub fn build(c: &Ctx, m: &Value, n: usize) -> Result<(Vec<Mark>, String)> {
    let kind = m["tipo"].as_str().or(m["type"].as_str()).context("manca «tipo»")?.trim().to_lowercase();
    let at = target(c.reading, if m["su"].is_null() { &m["riquadro"] } else { &m["su"] })?;
    let hl = matches!(kind.as_str(), "evidenzia" | "evidenziatore" | "highlight");
    let color = match m["colore"].as_str().or(m["color"].as_str()) {
        Some(s) => parse_color(s, hl).with_context(|| format!("colore «{s}» sconosciuto (rosso, blu, verde, giallo, arancione, viola, rosa, nero, grigio, #rrggbb)"))?,
        None if hl => c.highlighter,
        None => RED,
    };
    let width = m["spessore"].as_f64().map(|w| (w as f32).clamp(0.5, 12.0)).unwrap_or(c.pen_width.max(1.5));
    let mut rng = Rng::new(0x5EED ^ (n as u64).wrapping_mul(0x9E37_79B9) ^ (m.to_string().len() as u64) << 20);
    let h = c.h();
    let ink = |pts: Vec<(f32, f32)>| Mark { pts: c.back(pts), color, width, highlight: false };
    let need = |what: &str| -> Result<[f32; 4]> { at.with_context(|| format!("«{what}»: manca dove («su» con un nome del foglio, o «riquadro»)")) };
    let marks: Vec<Mark> = match kind.as_str() {
        "evidenzia" | "evidenziatore" | "highlight" => {
            let b = c.pbox(need(&kind)?);
            // a fasce alte quanto una riga di scrittura
            let bands = (((b[3] - b[1]) / (h * 1.6)).round() as usize).clamp(1, 40);
            let bh = (b[3] - b[1]) / bands as f32 + 0.25 * h;
            (0..bands)
                .map(|k| {
                    let y = b[1] + (k as f32 + 0.5) * (b[3] - b[1]) / bands as f32;
                    // le estremità tonde sporgono di mezza fascia
                    let (x0, x1) = (b[0] - 0.1 * h + bh * 0.3, b[2] + 0.1 * h - bh * 0.3);
                    let pts = if x1 > x0 { vec![(x0, y + rng.next() * 0.05 * h), (x1, y + rng.next() * 0.05 * h)] } else { vec![((b[0] + b[2]) / 2.0, y)] };
                    Mark { pts: c.back(pts), color, width: bh * 800.0, highlight: true }
                })
                .collect()
        }
        "sottolinea" | "sottolineatura" | "underline" => {
            let b = c.pbox(need(&kind)?);
            let mut out = Vec::new();
            let lines = if m["doppia"].as_bool().unwrap_or(false) { 2 } else { 1 };
            for k in 0..lines {
                let y = b[3] + (0.18 + 0.22 * k as f32) * h;
                out.push(ink(wobbly((b[0] - 0.1 * h, y + rng.next() * 0.05 * h), (b[2] + 0.15 * h, y + rng.next() * 0.08 * h), 0.04 * h, &mut rng)));
            }
            out
        }
        "barra" | "cancella" | "strike" | "barrato" => {
            let b = c.pbox(need(&kind)?);
            let y = (b[1] + b[3]) / 2.0;
            vec![ink(wobbly((b[0] - 0.1 * h, y + 0.08 * h), (b[2] + 0.1 * h, y - 0.08 * h), 0.03 * h, &mut rng))]
        }
        "cerchia" | "cerchio" | "circle" => {
            let b = c.pbox(need(&kind)?);
            let (cx, cy) = ((b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0);
            let (rx, ry) = ((b[2] - b[0]) / 2.0 * 1.12 + 0.35 * h, (b[3] - b[1]) / 2.0 * 1.2 + 0.35 * h);
            let a0 = std::f32::consts::PI * (1.1 + rng.next() * 0.05);
            let turn = std::f32::consts::TAU * 1.08;
            let steps = 56;
            let wob = rng.next() * 0.04;
            vec![ink(
                (0..=steps)
                    .map(|k| {
                        let t = k as f32 / steps as f32;
                        let a = a0 + turn * t;
                        // la fine del giro un po' più larga: non si chiude preciso
                        let r = 1.0 + wob * (3.0 * a).sin() + 0.05 * t;
                        (cx + rx * r * a.cos(), cy + ry * r * a.sin())
                    })
                    .collect(),
            )]
        }
        "riquadro" | "rettangolo" | "box" | "incornicia" => {
            let b = c.pbox(need(&kind)?);
            let p = 0.3 * h;
            let (x0, y0, x1, y1) = (b[0] - p, b[1] - p, b[2] + p, b[3] + p);
            let j = |rng: &mut Rng| rng.next() * 0.06 * h;
            vec![ink(vec![
                (x0 + j(&mut rng), y0 + j(&mut rng)),
                (x1 + j(&mut rng), y0 + j(&mut rng)),
                (x1 + j(&mut rng), y1 + j(&mut rng)),
                (x0 + j(&mut rng), y1 + j(&mut rng)),
                (x0 + j(&mut rng), y0 - 0.05 * h),
                (x0 + 0.25 * h, y0 - 0.02 * h),
            ])]
        }
        "freccia" | "arrow" | "linea" | "line" => {
            let arrow = matches!(kind.as_str(), "freccia" | "arrow");
            let to = target(c.reading, &m["a"])?.or(at).with_context(|| format!("«{kind}»: manca dove arriva («a» o «su»)"))?;
            let from = target(c.reading, &m["da"])?;
            let tb = c.pbox(to);
            let (start, end) = match from {
                Some(f) => {
                    let fb = c.pbox(f);
                    let (fc, tc) = (center(fb), center(tb));
                    (exit(fb, tc, 0.25 * h), exit(tb, fc, 0.25 * h))
                }
                None => {
                    // da in alto a destra verso il pezzo
                    let e = (tb[2] + 0.25 * h, (tb[1] + tb[3]) / 2.0 - 0.1 * h);
                    ((e.0 + 2.6 * h, e.1 - 1.6 * h), e)
                }
            };
            let mut out = vec![ink(if arrow { curve(start, end, 0.08, &mut rng) } else { wobbly(start, end, 0.03 * h, &mut rng) })];
            if arrow {
                let len = ((end.0 - start.0).hypot(end.1 - start.1) * 0.3).min(1.1 * h).max(0.4 * h);
                let ang = (end.1 - start.1).atan2(end.0 - start.0);
                let wing = |s: f32| (end.0 - len * (ang + s).cos(), end.1 - len * (ang + s).sin());
                out.push(ink(vec![wing(0.48), end, wing(-0.48)]));
            }
            out
        }
        "spunta" | "check" | "visto" | "croce" | "cross" | "x" => {
            let b = c.pbox(need(&kind)?);
            let s = 1.0 * h;
            let (x, y) = if b[2] - b[0] < 1e-4 { (b[0], b[1]) } else { (b[2] + 0.5 * h, (b[1] + b[3]) / 2.0 - 0.5 * s) };
            if matches!(kind.as_str(), "spunta" | "check" | "visto") {
                vec![ink(vec![(x, y + 0.55 * s), (x + 0.35 * s, y + s), (x + 0.6 * s, y + 0.45 * s), (x + 1.0 * s, y - 0.2 * s)])]
            } else {
                vec![ink(vec![(x, y), (x + 0.8 * s, y + 0.9 * s)]), ink(vec![(x + 0.8 * s, y - 0.05 * s), (x + 0.05 * s, y + 0.95 * s)])]
            }
        }
        "testo" | "text" | "scrivi" => {
            let text = m["testo"].as_str().or(m["text"].as_str()).filter(|t| !t.trim().is_empty()).context("«testo»: manca il testo")?;
            let style = Style::parse(m["stile"].as_str().unwrap_or("stampatello"));
            // di serie alto come la scrittura del foglio
            let cap = m["altezza"].as_f64().map(|v| v as f32 / if v > 1.0 { 1000.0 } else { 1.0 }).unwrap_or(h * 0.9).clamp(0.01, 0.12);
            let dove = m["dove"].as_str().unwrap_or("destra").trim().to_lowercase();
            let given = !m["riquadro"].is_null() && m["su"].is_null();
            let lay = |origin: (f32, f32), max_w: f32, right: bool, cap: f32| handwrite::layout(text, style, cap, c.aspect, origin, max_w.max(0.08), right);
            let mut note = String::new();
            let laid = match at {
                // dentro un riquadro: va a capo, e rimpicciolisce finché ci sta
                Some(b) if given && b[2] - b[0] > 1e-4 => {
                    let mut l = lay((b[0], b[1]), b[2] - b[0], false, cap);
                    for k in [0.85, 0.7, 0.55] {
                        if l.bbox[3] <= b[3] + 0.2 * h {
                            break;
                        }
                        l = lay((b[0], b[1]), b[2] - b[0], false, cap * k);
                    }
                    if c.collides(l.bbox, None) {
                        note.push_str(" (copre in parte lo scritto)");
                    }
                    l
                }
                // da un punto
                Some(b) if b[2] - b[0] < 1e-4 => lay((b[0], b[1]), 0.97 - b[0], false, cap),
                // accanto a un pezzo del foglio: dove c'è posto
                Some(b) => {
                    let mut order = vec![dove.as_str()];
                    for d in ["destra", "sotto", "sopra"] {
                        if !order.contains(&d) {
                            order.push(d);
                        }
                    }
                    let mut found = None;
                    'try_: for d in order {
                        for k in [1.0, 0.8, 0.65] {
                            let cap = cap * k;
                            let gap = 0.6 * h / c.aspect;
                            let l = match d {
                                "sotto" | "below" => lay((b[0], b[3] + 0.35 * h), 0.97 - b[0], false, cap),
                                "sopra" | "above" => {
                                    let l = lay((b[0], 0.0), 0.97 - b[0], false, cap);
                                    let th = l.bbox[3] - l.bbox[1];
                                    lay((b[0], b[1] - 0.3 * h - th), 0.97 - b[0], false, cap)
                                }
                                "sinistra" | "left" => lay((b[0] - gap, (b[1] + b[3]) / 2.0 - cap / 2.0), b[0] - gap - 0.02, true, cap),
                                _ => lay((b[2] + gap, (b[1] + b[3]) / 2.0 - cap / 2.0), 0.97 - b[2] - gap, false, cap),
                            };
                            let inside = l.bbox[0] >= 0.0 && l.bbox[1] >= 0.0 && l.bbox[2] <= 1.0 && l.bbox[3] <= 0.99;
                            if inside && !c.collides(l.bbox, Some(b)) {
                                if d != dove {
                                    note.push_str(&format!(" (scritto {d}: {dove} non c'era posto)"));
                                }
                                found = Some(l);
                                break 'try_;
                            }
                        }
                    }
                    match found {
                        Some(l) => l,
                        None => {
                            note.push_str(" (accanto non c'era posto: scritto sotto tutto)");
                            c.below_all(&lay, cap)?
                        }
                    }
                }
                None => c.below_all(&lay, cap)?,
            };
            if laid.strokes.is_empty() {
                bail!("non so scrivere questi caratteri a mano: {}", laid.missing.iter().collect::<String>());
            }
            if laid.bbox[3] > 1.0 {
                bail!("il testo esce dal foglio in basso: scegli un altro posto, scrivi più piccolo («altezza») o usa una pagina nuova");
            }
            c.placed.borrow_mut().push(laid.bbox);
            let mut out: Vec<Mark> = laid
                .strokes
                .into_iter()
                .map(|pts| Mark { pts: pts.into_iter().map(|(u, v)| (u.clamp(0.003, 0.997), v.clamp(0.003, 0.997))).collect(), color, width, highlight: false })
                .collect();
            out.retain(|m| !m.pts.is_empty());
            if !laid.missing.is_empty() {
                note.push_str(&format!(" (saltati: {})", laid.missing.iter().collect::<String>()));
            }
            return Ok((out, format!("testo «{}»{note}", short(text))));
        }
        other => bail!("tipo «{other}» sconosciuto: evidenzia, sottolinea, barra, cerchia, riquadro, freccia, linea, spunta, croce, testo"),
    };
    Ok((marks, kind))
}

fn short(t: &str) -> String {
    let t = t.replace('\n', " ");
    if t.chars().count() > 30 {
        format!("{}…", t.chars().take(30).collect::<String>())
    } else {
        t
    }
}

fn center(b: [f32; 4]) -> (f32, f32) {
    ((b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0)
}

/// Il punto dove la retta dal centro di `b` verso `to` esce dal riquadro
/// (allargato di `pad`).
fn exit(b: [f32; 4], to: (f32, f32), pad: f32) -> (f32, f32) {
    let c = center(b);
    let (dx, dy) = (to.0 - c.0, to.1 - c.1);
    let (hw, hh) = ((b[2] - b[0]) / 2.0 + pad, (b[3] - b[1]) / 2.0 + pad);
    if dx.abs() < 1e-6 && dy.abs() < 1e-6 {
        return c;
    }
    let t = (hw / dx.abs().max(1e-6)).min(hh / dy.abs().max(1e-6)).min(1.0);
    (c.0 + dx * t, c.1 + dy * t)
}

/// Una linea quasi dritta, come tirata a mano.
fn wobbly(a: (f32, f32), b: (f32, f32), amp: f32, rng: &mut Rng) -> Vec<(f32, f32)> {
    let n = 6;
    let (nx, ny) = {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let l = dx.hypot(dy).max(1e-6);
        (-dy / l, dx / l)
    };
    let bow = rng.next() * amp;
    (0..=n)
        .map(|k| {
            let t = k as f32 / n as f32;
            let off = bow * (std::f32::consts::PI * t).sin() + if k == 0 || k == n { 0.0 } else { rng.next() * amp * 0.3 };
            (a.0 + (b.0 - a.0) * t + nx * off, a.1 + (b.1 - a.1) * t + ny * off)
        })
        .collect()
}

/// Una curva leggera da `a` a `b` (le frecce).
fn curve(a: (f32, f32), b: (f32, f32), bend: f32, rng: &mut Rng) -> Vec<(f32, f32)> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l = dx.hypot(dy);
    let side = if rng.next() >= 0.0 { 1.0 } else { -1.0 };
    let ctrl = ((a.0 + b.0) / 2.0 - dy / l.max(1e-6) * l * bend * side, (a.1 + b.1) / 2.0 + dx / l.max(1e-6) * l * bend * side);
    (0..=16)
        .map(|k| {
            let t = k as f32 / 16.0;
            let u = 1.0 - t;
            (u * u * a.0 + 2.0 * u * t * ctrl.0 + t * t * b.0, u * u * a.1 + 2.0 * u * t * ctrl.1 + t * t * b.1)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boardread::{read, tests::*};
    use serde_json::json;

    #[test]
    fn segni_sul_foglio() {
        let g = geom();
        let (mut strokes, end) = word(&g, 0.10, 0.10, 4);
        strokes.extend(word(&g, end + 0.03, 0.10, 3).0);
        let r = read(&strokes, &g, None);
        let c = Ctx::new(&r, g.w / g.h, 2.0, [255, 235, 90]);
        let wb = r.find("r1p2").unwrap();
        // sottolinea: sotto la parola, lunga quanto lei
        let (m, _) = build(&c, &json!({ "tipo": "sottolinea", "su": "r1p2" }), 0).unwrap();
        assert_eq!(m.len(), 1);
        let ys: Vec<f32> = m[0].pts.iter().map(|p| p.1).collect();
        assert!(ys.iter().all(|&y| y > wb[3] && y < wb[3] + 0.02), "{ys:?} {wb:?}");
        assert!(m[0].pts[0].0 < wb[0] + 0.005 && m[0].pts.last().unwrap().0 > wb[2] - 0.005);
        assert_eq!(m[0].color, RED);
        // evidenzia: dell'evidenziatore, sulla parola
        let (m, _) = build(&c, &json!({ "tipo": "evidenzia", "su": "r1p1" }), 1).unwrap();
        assert!(m[0].highlight && m[0].color == [255, 235, 90] && m[0].width > 10.0);
        // cerchia: tutto attorno
        let (m, _) = build(&c, &json!({ "tipo": "cerchia", "su": ["r1p1", "r1p2"], "colore": "blu" }), 2).unwrap();
        let b = m[0].pts.iter().fold([1f32, 1.0, 0.0, 0.0], |b, p| [b[0].min(p.0), b[1].min(p.1), b[2].max(p.0), b[3].max(p.1)]);
        let all = r.find("r1").unwrap();
        assert!(b[0] < all[0] && b[2] > all[2] && b[1] < all[1] && b[3] > all[3], "{b:?} {all:?}");
        assert_eq!(m[0].color, [30, 80, 200]);
        // freccia fra due parole: due tratti (asta e punta)
        let (m, _) = build(&c, &json!({ "tipo": "freccia", "da": "r1p1", "a": "r1p2" }), 3).unwrap();
        assert_eq!(m.len(), 2);
        // testo a destra della riga, alla sua altezza
        let (m, what) = build(&c, &json!({ "tipo": "testo", "su": "r1", "testo": "Bene!", "colore": "verde" }), 4).unwrap();
        assert!(what.contains("Bene!"));
        let tb = m.iter().flat_map(|m| m.pts.iter()).fold([1f32, 1.0, 0.0, 0.0], |b, p| [b[0].min(p.0), b[1].min(p.1), b[2].max(p.0), b[3].max(p.1)]);
        assert!(tb[0] > all[2] && tb[1] < all[3] && tb[3] > all[1], "{tb:?} {all:?}");
        // riquadri e punti in 0–1000
        assert_eq!(target(&r, &json!([100, 200, 300, 400])).unwrap(), Some([0.1, 0.2, 0.3, 0.4]));
        assert!(target(&r, &json!("r9")).is_err());
        assert!(build(&c, &json!({ "tipo": "boh", "su": "r1" }), 5).is_err());
        assert!(build(&c, &json!({ "tipo": "sottolinea" }), 6).is_err());
        assert_eq!(parse_color("#ff8000", false), Some([255, 128, 0]));
        // da una parola all'altra
        let span = target(&r, &json!("r1p1-p2")).unwrap().unwrap();
        assert!(span[0] <= r.find("r1p1").unwrap()[0] && span[2] >= wb[2]);
    }
}
