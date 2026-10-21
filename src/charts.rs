//! Grafici disegnati per la chat AI con Typst e Lilaq (https://lilaq.org, MIT):
//! lo strumento `draw_chart`.
//!
//! Il modello descrive il grafico in JSON (funzioni da tracciare, punti,
//! rette, barre, note); qui si calcolano i valori delle funzioni (con exmex) e
//! si scrive il codice Typst per Lilaq, che lo disegna con i font di LaTeX,
//! gli stessi delle formule. Il risultato è un SVG (per i documenti vettoriali)
//! e un PNG (per Word e le foto), salvati in `charts/` con un id: la chat li
//! mostra e il modello li può mettere nel documento aperto (`docagent`).

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};
use std::fmt::Write as _;
use std::path::PathBuf;

/// Cartella dei grafici (nella cartella dei dati).
pub const DIR: &str = "charts";

/// Punti calcolati per ogni funzione.
const SAMPLES: usize = 600;

/// Definizione dello strumento per il modello (formato OpenAI).
pub fn definition() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "draw_chart",
            "description": "Draw a chart or a math graph (LaTeX look) and show it in the chat. Use it whenever a drawing helps: graphs of functions, points, lines, bar charts, data series. Returns the chart id (e.g. chart-12): pass it to document_insert_image to put the chart in the open document. Do NOT write SVG or ASCII art yourself: use this tool.",
            "parameters": {
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Optional title. Inline math as $...$ (LaTeX)." },
                    "x": { "type": "object", "description": "x axis: {min, max, label}. min/max required for functions.", "properties": {
                        "min": { "type": "number" }, "max": { "type": "number" }, "label": { "type": "string" } } },
                    "y": { "type": "object", "description": "y axis: {min, max, label}; min/max optional (automatic).", "properties": {
                        "min": { "type": "number" }, "max": { "type": "number" }, "label": { "type": "string" } } },
                    "axes": { "type": "string", "enum": ["centered", "box"], "description": "centered = school style, axes crossing at the origin with arrows (default when the ranges contain 0); box = scientific frame." },
                    "grid": { "type": "boolean", "description": "Show the grid (default false)." },
                    "equal_scale": { "type": "boolean", "description": "Same unit length on x and y (default true for centered axes)." },
                    "series": {
                        "type": "array",
                        "description": "What to draw, in order.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "type": { "type": "string", "enum": ["function", "points", "polyline", "vline", "hline", "bar", "parametric"],
                                    "description": "function: y = f(x) from expr; points: marked points [[x,y],…] with optional names; polyline: points joined by segments; vline/hline: vertical line x = value / horizontal line y = value; bar: bars at x with heights y (or categories); parametric: x(t), y(t)." },
                                "expr": { "type": "string", "description": "function: expression in x, e.g. \"1 - x\", \"x^2 - 3*x + 2\", \"sin(x)/x\", \"sqrt(x)\", \"exp(-x^2)\", \"abs(x)\", \"ln(x)\". Use * for products (2*x, not 2x); pi for π." },
                                "x_expr": { "type": "string", "description": "parametric: x(t)" },
                                "y_expr": { "type": "string", "description": "parametric: y(t)" },
                                "t": { "type": "array", "items": { "type": "number" }, "description": "parametric: [t_min, t_max]" },
                                "from": { "type": "number", "description": "function: start of the domain (default x.min)" },
                                "to": { "type": "number", "description": "function: end of the domain (default x.max)" },
                                "points": { "type": "array", "items": { "type": "array", "items": { "type": "number" } }, "description": "points/polyline: [[x, y], …]" },
                                "names": { "type": "array", "items": { "type": "string" }, "description": "points: a name for each point, e.g. [\"P(-1, 2)\"]; $...$ allowed" },
                                "value": { "type": "number", "description": "vline: x; hline: y" },
                                "x": { "type": "array", "items": {}, "description": "bar: positions (numbers) or category names" },
                                "y": { "type": "array", "items": { "type": "number" }, "description": "bar: heights" },
                                "label": { "type": "string", "description": "Legend entry, e.g. \"$y = 1 - x$\"" },
                                "color": { "type": "string", "description": "red, blue, green, orange, purple, black, gray… or #RRGGBB" },
                                "dashed": { "type": "boolean" }
                            },
                            "required": ["type"]
                        }
                    },
                    "notes": { "type": "array", "description": "Text placed on the chart: [{x, y, text}], text may contain $...$.",
                        "items": { "type": "object", "properties": { "x": { "type": "number" }, "y": { "type": "number" }, "text": { "type": "string" } }, "required": ["x", "y", "text"] } },
                    "width_cm": { "type": "number", "description": "Drawing width (default 10)." },
                    "height_cm": { "type": "number", "description": "Drawing height (default 7, or from equal_scale)." }
                },
                "required": ["series"]
            }
        }
    })
}

/// Un grafico fatto.
pub struct Chart {
    pub id: String,
    pub svg: PathBuf,
    pub png: PathBuf,
    /// Misure in punti (per metterlo in un documento alla sua grandezza).
    pub width: f32,
    pub height: f32,
}

/// Disegna il grafico descritto da `spec` e lo salva in `charts/`.
pub fn draw(spec: &Value) -> Result<Chart> {
    let src = typst_source(spec)?;
    let doc = crate::typstworld::compile(src.clone()).map_err(|e| {
        tracing::warn!("grafico, sorgente Typst:\n{src}");
        e
    })?;
    let page = &doc.pages()[0];
    let (width, height) = (page.frame.width().to_pt() as f32, page.frame.height().to_pt() as f32);
    std::fs::create_dir_all(DIR)?;
    let id = new_id();
    let svg = PathBuf::from(DIR).join(format!("{id}.svg"));
    let png = PathBuf::from(DIR).join(format!("{id}.png"));
    std::fs::write(&svg, crate::typstworld::svg(&doc))?;
    std::fs::write(&png, crate::typstworld::png(&doc, 300.0)?)?;
    Ok(Chart { id, svg, png, width, height })
}

/// Il grafico `id` già fatto (per inserirlo in un documento).
pub fn find(id: &str) -> Option<Chart> {
    let id = id.trim().trim_end_matches(".svg").trim_end_matches(".png");
    if !id.starts_with("chart-") || !id[6..].chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return None;
    }
    let svg = PathBuf::from(DIR).join(format!("{id}.svg"));
    let png = PathBuf::from(DIR).join(format!("{id}.png"));
    if !svg.exists() || !png.exists() {
        return None;
    }
    // misure dall'SVG di Typst (width="…pt")
    let head = std::fs::read_to_string(&svg).ok()?;
    let num = |a: &str| -> Option<f32> {
        let i = head.find(&format!(" {a}=\""))? + a.len() + 3;
        let rest = &head[i..];
        rest[..rest.find('"')?].trim_end_matches("pt").parse().ok()
    };
    Some(Chart { id: id.to_string(), width: num("width")?, height: num("height")?, svg, png })
}

/// Un'immagine restituita da uno strumento (es. una pagina disegnata da un
/// server MCP) salvata accanto ai grafici; ritorna l'indirizzo per la chat.
pub fn save_shot(png: &[u8]) -> Result<String> {
    std::fs::create_dir_all(DIR)?;
    let id = new_id().replacen("chart-", "shot-", 1);
    let ext = if png.starts_with(b"\x89PNG") { "png" } else if png.starts_with(&[0xFF, 0xD8]) { "jpg" } else { "png" };
    std::fs::write(PathBuf::from(DIR).join(format!("{id}.{ext}")), png)?;
    Ok(format!("api/charts/{id}.{ext}"))
}

/// Il file di `charts/` chiesto dalla chat (`chart-….svg|png`, `shot-….png|jpg`).
pub fn file(name: &str) -> Option<(PathBuf, &'static str)> {
    let (stem, ext) = name.rsplit_once('.')?;
    let ok_stem = (stem.starts_with("chart-") || stem.starts_with("shot-")) && stem.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    let mime = match ext {
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" => "image/jpeg",
        _ => return None,
    };
    let p = PathBuf::from(DIR).join(name);
    (ok_stem && p.is_file()).then_some((p, mime))
}

fn new_id() -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(1);
    format!("chart-{}-{}", chrono::Local::now().format("%Y%m%d%H%M%S"), N.fetch_add(1, Ordering::Relaxed))
}

// ---------------------------------------------------------------------------
// Dalla descrizione al codice Typst/Lilaq
// ---------------------------------------------------------------------------

/// Un'espressione in x (o t) pronta da calcolare.
fn compile_expr(expr: &str, var: &str) -> Result<impl Fn(f64) -> f64> {
    use exmex::prelude::*;
    // forme comuni scritte dai modelli: **, π, pi, e^, ln/log
    let mut e = expr.trim().replace("**", "^").replace('π', "PI").replace('·', "*").replace('−', "-");
    let words = |s: &str, from: &str, to: &str| -> String {
        // sostituisce `from` come parola intera
        let mut out = String::new();
        let b = s.as_bytes();
        let mut i = 0;
        while i < s.len() {
            if s[i..].starts_with(from)
                && (i == 0 || !(b[i - 1] as char).is_ascii_alphanumeric())
                && !s[i + from.len()..].starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_')
            {
                out.push_str(to);
                i += from.len();
            } else {
                let c = s[i..].chars().next().unwrap_or(' ');
                out.push(c);
                i += c.len_utf8();
            }
        }
        out
    };
    e = words(&e, "pi", "PI");
    e = words(&e, "e", "E");
    e = words(&e, "log", "ln");
    e = words(&e, "sen", "sin");
    e = words(&e, "tg", "tan");
    if var != "x" {
        e = words(&e, var, "x");
    }
    let f = FlatEx::<f64>::parse(&e).map_err(|err| anyhow!("espressione «{expr}»: {err}"))?;
    let vars = f.var_names().to_vec();
    if vars.len() > 1 || vars.first().is_some_and(|v| v != "x") {
        bail!("espressione «{expr}»: usa solo la variabile {var} (trovate: {})", vars.join(", "));
    }
    let constant = vars.is_empty();
    Ok(move |x: f64| if constant { f.eval(&[]).unwrap_or(f64::NAN) } else { f.eval(&[x]).unwrap_or(f64::NAN) })
}

/// Testo con eventuale matematica `$…$` come contenuto Typst.
fn content(text: &str) -> String {
    let mut out = String::from("[");
    for (i, part) in text.split('$').enumerate() {
        if i % 2 == 1 {
            match mitex::convert_math(part, None) {
                Ok(code) => {
                    let _ = write!(out, "#mi({code:?})");
                }
                Err(_) => out.push_str(&escape(part)),
            }
        } else {
            out.push_str(&escape(part));
        }
    }
    out.push(']');
    out
}

fn escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if matches!(c, '\\' | '#' | '[' | ']' | '*' | '_' | '`' | '<' | '>' | '@' | '$' | '~' | '/') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

fn color(name: Option<&str>) -> Option<String> {
    let n = name?.trim().to_ascii_lowercase();
    if n.starts_with('#') && (n.len() == 7 || n.len() == 4) && n[1..].chars().all(|c| c.is_ascii_hexdigit()) {
        return Some(format!("rgb(\"{n}\")"));
    }
    let named = match n.as_str() {
        "red" | "rosso" => "rgb(\"#d62728\")",
        "blue" | "blu" => "rgb(\"#1f77b4\")",
        "green" | "verde" => "rgb(\"#2ca02c\")",
        "orange" | "arancione" => "rgb(\"#ff7f0e\")",
        "purple" | "viola" => "rgb(\"#9467bd\")",
        "black" | "nero" => "black",
        "gray" | "grey" | "grigio" => "gray",
        "brown" | "marrone" => "rgb(\"#8c564b\")",
        "pink" | "rosa" => "rgb(\"#e377c2\")",
        "cyan" | "azzurro" => "rgb(\"#17becf\")",
        "yellow" | "giallo" => "rgb(\"#bcbd22\")",
        _ => return None,
    };
    Some(named.to_string())
}

fn num(v: f64) -> String {
    if v.is_finite() {
        let s = format!("{v:.6}");
        let s = s.trim_end_matches('0').trim_end_matches('.');
        if s == "-0" { "0".into() } else { s.to_string() }
    } else {
        "float.nan".into()
    }
}

fn arr(v: &[f64]) -> String {
    let mut s = String::from("(");
    for x in v {
        s.push_str(&num(*x));
        s.push(',');
    }
    s.push(')');
    s
}

/// Un passo «tondo» (1, 2, 5 × 10ⁿ) vicino a `raw`.
fn nice_step(raw: f64) -> f64 {
    if !(raw > 0.0) {
        return 1.0;
    }
    let p = 10f64.powf(raw.log10().floor());
    let m = raw / p;
    p * if m < 1.5 { 1.0 } else if m < 3.5 { 2.0 } else if m < 7.5 { 5.0 } else { 10.0 }
}

fn f64_of(v: &Value) -> Option<f64> {
    v.as_f64().or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
}

/// Pezzi continui di una curva campionata (si spezza dove la funzione non
/// esiste o salta, es. 1/x vicino a 0).
fn segments(xs: &[f64], ys: &[f64], span: f64) -> Vec<(Vec<f64>, Vec<f64>)> {
    let mut out: Vec<(Vec<f64>, Vec<f64>)> = Vec::new();
    let mut cur: (Vec<f64>, Vec<f64>) = (Vec::new(), Vec::new());
    for (&x, &y) in xs.iter().zip(ys) {
        let jump = cur.1.last().is_some_and(|&py: &f64| (y - py).abs() > span * 2.0);
        if !y.is_finite() || jump {
            if cur.0.len() > 1 {
                out.push(std::mem::take(&mut cur));
            } else {
                cur = (Vec::new(), Vec::new());
            }
            if !y.is_finite() {
                continue;
            }
        }
        cur.0.push(x);
        cur.1.push(y);
    }
    if cur.0.len() > 1 {
        out.push(cur);
    }
    out
}

pub fn typst_source(spec: &Value) -> Result<String> {
    let series = spec["series"].as_array().filter(|s| !s.is_empty()).context("serve almeno una serie in «series»")?;
    let (mut xmin, mut xmax) = (f64_of(&spec["x"]["min"]), f64_of(&spec["x"]["max"]));
    // intervallo x dai dati, se non dato
    if xmin.is_none() || xmax.is_none() {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for s in series {
            for p in s["points"].as_array().into_iter().flatten() {
                if let Some(x) = p.get(0).and_then(f64_of) {
                    lo = lo.min(x);
                    hi = hi.max(x);
                }
            }
            if let Some(v) = f64_of(&s["value"]).filter(|_| s["type"] == "vline") {
                lo = lo.min(v);
                hi = hi.max(v);
            }
        }
        if lo.is_finite() && hi.is_finite() {
            let pad = ((hi - lo) * 0.15).max(1.0);
            xmin = xmin.or(Some(lo - pad));
            xmax = xmax.or(Some(hi + pad));
        }
    }
    let has_bars = series.iter().any(|s| s["type"] == "bar");
    let (xmin, xmax) = match (xmin, xmax) {
        (Some(a), Some(b)) if b > a => (a, b),
        _ if has_bars => (f64::NAN, f64::NAN),
        _ => (-5.0, 5.0),
    };
    let span_x = xmax - xmin;
    let mut body = String::new();
    let mut ylo = f64::INFINITY;
    let mut yhi = f64::NEG_INFINITY;
    let mut fun_ys: Vec<f64> = Vec::new();
    for (k, s) in series.iter().enumerate() {
        let kind = s["type"].as_str().unwrap_or("function");
        let label = s["label"].as_str().filter(|l| !l.trim().is_empty()).map(content);
        let col = color(s["color"].as_str());
        let mut style = String::new();
        if let Some(c) = &col {
            let _ = write!(style, ", color: {c}");
        }
        let dash = if s["dashed"].as_bool() == Some(true) { "dashed" } else { "solid" };
        let lab = |first: bool| match (&label, first) {
            (Some(l), true) => format!(", label: {l}"),
            _ => String::new(),
        };
        match kind {
            "function" | "parametric" => {
                let (xs, ys): (Vec<f64>, Vec<f64>) = if kind == "function" {
                    let expr = s["expr"].as_str().context("una funzione vuole «expr»")?;
                    let f = compile_expr(expr, "x")?;
                    let a = f64_of(&s["from"]).unwrap_or(xmin).max(if xmin.is_finite() { xmin } else { f64::MIN });
                    let b = f64_of(&s["to"]).unwrap_or(xmax).min(if xmax.is_finite() { xmax } else { f64::MAX });
                    if !(b > a) {
                        bail!("intervallo vuoto per «{expr}»");
                    }
                    (0..=SAMPLES).map(|i| a + (b - a) * i as f64 / SAMPLES as f64).map(|x| (x, f(x))).unzip()
                } else {
                    let fx = compile_expr(s["x_expr"].as_str().context("una curva parametrica vuole «x_expr»")?, "t")?;
                    let fy = compile_expr(s["y_expr"].as_str().context("una curva parametrica vuole «y_expr»")?, "t")?;
                    let t = s["t"].as_array();
                    let t0 = t.and_then(|t| t.first()).and_then(f64_of).unwrap_or(0.0);
                    let t1 = t.and_then(|t| t.get(1)).and_then(f64_of).unwrap_or(std::f64::consts::TAU);
                    (0..=SAMPLES).map(|i| t0 + (t1 - t0) * i as f64 / SAMPLES as f64).map(|t| (fx(t), fy(t))).unzip()
                };
                fun_ys.extend(ys.iter().copied().filter(|y| y.is_finite()));
                let span = {
                    let mut v: Vec<f64> = ys.iter().copied().filter(|y| y.is_finite()).collect();
                    v.sort_by(|a, b| a.total_cmp(b));
                    if v.len() > 10 { (v[v.len() * 95 / 100] - v[v.len() * 5 / 100]).abs().max(1e-9) } else { f64::INFINITY }
                };
                for (i, (sx, sy)) in segments(&xs, &ys, if kind == "function" { span } else { f64::INFINITY }).into_iter().enumerate() {
                    let _ = writeln!(body, "  lq.plot({}, {}, mark: none, stroke: (dash: \"{dash}\", thickness: 1.1pt){style}{}),", arr(&sx), arr(&sy), lab(i == 0));
                }
            }
            "points" | "polyline" => {
                let pts: Vec<(f64, f64)> = s["points"]
                    .as_array()
                    .context("«points» vuole [[x, y], …]")?
                    .iter()
                    .filter_map(|p| Some((f64_of(p.get(0)?)?, f64_of(p.get(1)?)?)))
                    .collect();
                if pts.is_empty() {
                    bail!("nessun punto valido in «points»");
                }
                for &(_, y) in &pts {
                    ylo = ylo.min(y);
                    yhi = yhi.max(y);
                }
                let (xs, ys): (Vec<f64>, Vec<f64>) = pts.iter().copied().unzip();
                if kind == "points" {
                    let _ = writeln!(body, "  lq.plot({}, {}, stroke: none, mark: \"o\", mark-size: 5pt{style}{}),", arr(&xs), arr(&ys), lab(true));
                    for (i, name) in s["names"].as_array().into_iter().flatten().enumerate() {
                        if let (Some(n), Some(&(x, y))) = (name.as_str(), pts.get(i)) {
                            let _ = writeln!(body, "  lq.place({}, {}, align: left + bottom, pad(left: 4pt, bottom: 3pt, box(fill: white.transparentize(15%), inset: (x: 1.5pt, y: 1pt), {}))),", num(x), num(y), content(n));
                        }
                    }
                } else {
                    let _ = writeln!(body, "  lq.plot({}, {}, mark: none, stroke: (dash: \"{dash}\", thickness: 1.1pt){style}{}),", arr(&xs), arr(&ys), lab(true));
                }
            }
            "vline" | "hline" => {
                let v = f64_of(&s["value"]).context("una retta verticale/orizzontale vuole «value»")?;
                if kind == "hline" {
                    ylo = ylo.min(v);
                    yhi = yhi.max(v);
                }
                let f = if kind == "vline" { "vlines" } else { "hlines" };
                let stroke = format!("stroke: (dash: \"{dash}\", thickness: 1.1pt{})", col.as_ref().map(|c| format!(", paint: {c}")).unwrap_or_default());
                let _ = writeln!(body, "  lq.{f}({}, {stroke}{}),", num(v), lab(true));
            }
            "bar" => {
                let ys: Vec<f64> = s["y"].as_array().context("le barre vogliono «y»")?.iter().filter_map(f64_of).collect();
                let xs_raw = s["x"].as_array().cloned().unwrap_or_default();
                let numeric: Option<Vec<f64>> = xs_raw.iter().map(f64_of).collect();
                for &y in &ys {
                    ylo = ylo.min(y.min(0.0));
                    yhi = yhi.max(y.max(0.0));
                }
                match numeric.filter(|x| x.len() == ys.len()) {
                    Some(xs) => {
                        let _ = writeln!(body, "  lq.bar({}, {}{style}{}),", arr(&xs), arr(&ys), lab(true));
                    }
                    None => {
                        // categorie: posizioni 0, 1, 2… con i nomi sotto
                        let xs: Vec<f64> = (0..ys.len()).map(|i| i as f64).collect();
                        let _ = writeln!(body, "  lq.bar({}, {}{style}{}),", arr(&xs), arr(&ys), lab(true));
                        let names: Vec<String> = xs_raw.iter().map(|v| content(v.as_str().unwrap_or(&v.to_string()))).collect();
                        let ticks: Vec<String> = names.iter().enumerate().map(|(i, n)| format!("({i}, {n})")).collect();
                        body.insert_str(0, &format!("  xaxis: (ticks: ({},), subticks: none),\n", ticks.join(", ")));
                    }
                }
            }
            other => bail!("tipo di serie sconosciuto: «{other}» (serie {})", k + 1),
        }
    }
    for n in spec["notes"].as_array().into_iter().flatten() {
        if let (Some(x), Some(y), Some(t)) = (f64_of(&n["x"]), f64_of(&n["y"]), n["text"].as_str()) {
            let _ = writeln!(body, "  lq.place({}, {}, align: left + bottom, pad(left: 3pt, bottom: 2pt, box(fill: white.transparentize(15%), inset: (x: 1.5pt, y: 1pt), {}))),", num(x), num(y), content(t));
        }
    }
    // intervallo y: quello dato, o dai dati (le funzioni senza gli estremi che scappano)
    if !fun_ys.is_empty() {
        fun_ys.sort_by(|a, b| a.total_cmp(b));
        let n = fun_ys.len();
        ylo = ylo.min(fun_ys[n * 2 / 100]);
        yhi = yhi.max(fun_ys[(n * 98 / 100).min(n - 1)]);
    }
    let (ymin, ymax) = match (f64_of(&spec["y"]["min"]), f64_of(&spec["y"]["max"])) {
        (Some(a), Some(b)) if b > a => (a, b),
        (a, b) => {
            let (lo, hi) = if ylo.is_finite() && yhi.is_finite() { (ylo, yhi) } else { (-5.0, 5.0) };
            let pad = ((hi - lo) * 0.12).max(0.5);
            (a.unwrap_or(lo - pad), b.unwrap_or(hi + pad))
        }
    };
    let contains0 = xmin.is_finite() && xmin < 0.0 && xmax > 0.0 && ymin < 0.0 && ymax > 0.0;
    let centered = match spec["axes"].as_str() {
        Some("centered") => xmin.is_finite(),
        Some("box") => false,
        _ => contains0 && !has_bars,
    };
    let equal = spec["equal_scale"].as_bool().unwrap_or(centered) && xmin.is_finite();
    let w_cm = f64_of(&spec["width_cm"]).unwrap_or(10.0).clamp(3.0, 18.0);
    let h_cm = match f64_of(&spec["height_cm"]) {
        Some(h) => h.clamp(2.0, 24.0),
        None if equal => (w_cm * (ymax - ymin) / span_x).clamp(3.0, 16.0),
        None => w_cm * 0.68,
    };

    let mut src = String::new();
    src.push_str("#import \"@preview/lilaq:0.6.0\" as lq\n#import \"@preview/tiptoe:0.4.0\"\n#import \"/mitex/mod.typ\": mitex-scope\n");
    src.push_str("#let mi(code) = eval(\"$\" + code + \"$\", scope: mitex-scope)\n");
    src.push_str("#set page(width: auto, height: auto, margin: 8pt, fill: white)\n#set text(size: 10pt, font: \"New Computer Modern\")\n");
    let _ = write!(src, "#lq.diagram(\n  width: {}cm, height: {}cm,\n", num(w_cm), num(h_cm));
    if let Some(t) = spec["title"].as_str().filter(|t| !t.trim().is_empty()) {
        let _ = writeln!(src, "  title: {},", content(t));
    }
    if xmin.is_finite() {
        let _ = writeln!(src, "  xlim: ({}, {}),", num(xmin), num(xmax));
    }
    let _ = writeln!(src, "  ylim: ({}, {}),", num(ymin), num(ymax));
    let xl = spec["x"]["label"].as_str().map(content);
    let yl = spec["y"]["label"].as_str().map(content);
    if centered {
        // assi che si incrociano nell'origine, con le frecce (come a scuola),
        // stesso passo delle tacche sui due assi e i nomi degli assi in punta
        let step = nice_step((xmax - xmin).max(ymax - ymin) / 9.0);
        let filt = "filter: (value, distance) => value != 0";
        let _ = writeln!(
            src,
            "  xaxis: (position: 0, tip: tiptoe.stealth, tick-distance: {s}, subticks: none, {filt}),\n  yaxis: (position: 0, tip: tiptoe.stealth, tick-distance: {s}, subticks: none, {filt}),",
            s = num(step)
        );
        if let Some(l) = &xl {
            let _ = writeln!(body, "  lq.place({}, 0, align: right + bottom, pad(bottom: 4pt, {l})),", num(xmax));
        }
        if let Some(l) = &yl {
            let _ = writeln!(body, "  lq.place(0, {}, align: left + top, pad(left: 6pt, {l})),", num(ymax));
        }
    } else {
        if let Some(l) = &xl {
            let _ = writeln!(src, "  xlabel: {l},");
        }
        if let Some(l) = &yl {
            let _ = writeln!(src, "  ylabel: {l},");
        }
    }
    if spec["grid"].as_bool() != Some(true) {
        src.push_str("  grid: none,\n");
    }
    if !series.iter().any(|s| s["label"].as_str().is_some_and(|l| !l.trim().is_empty())) {
        src.push_str("  legend: none,\n");
    }
    // l'asse x delle categorie (se c'è) va prima delle serie
    if let Some(i) = body.find("  xaxis: (ticks:") {
        let end = body[i..].find('\n').map(|e| i + e + 1).unwrap_or(body.len());
        let line = body[i..end].to_string();
        body.replace_range(i..end, "");
        src.push_str(&line);
    }
    src.push_str(&body);
    src.push_str(")\n");
    Ok(src)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sistema_con_punto() {
        let spec = json!({
            "title": "Sistema $x + y = 1$, $x = -1$",
            "x": { "min": -4, "max": 4, "label": "$x$" },
            "y": { "min": -3, "max": 4, "label": "$y$" },
            "series": [
                { "type": "function", "expr": "1 - x", "label": "$x + y = 1$", "color": "blue" },
                { "type": "vline", "value": -1, "label": "$x = -1$", "color": "green", "dashed": true },
                { "type": "points", "points": [[-1, 2]], "names": ["$P(-1, 2)$"], "color": "red" }
            ]
        });
        let dir = std::env::temp_dir().join(format!("openslate-charts-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = typst_source(&spec).unwrap();
        let doc = crate::typstworld::compile(src.clone()).unwrap_or_else(|e| panic!("{e:#}\n{src}"));
        let png = crate::typstworld::png(&doc, 150.0).unwrap();
        assert!(png.starts_with(b"\x89PNG"));
        let svg = crate::typstworld::svg(&doc);
        assert!(svg.starts_with("<svg"));
        if let Ok(d) = std::env::var("CHART_OUT") {
            std::fs::write(format!("{d}/sistema.png"), &png).unwrap();
        }
    }

    #[test]
    fn funzioni_e_barre() {
        for spec in [
            json!({ "x": { "min": -6.3, "max": 6.3 }, "series": [{ "type": "function", "expr": "sin(x)/x" }, { "type": "function", "expr": "1/x", "dashed": true }] }),
            json!({ "x": { "min": 0, "max": 10 }, "axes": "box", "grid": true, "series": [{ "type": "function", "expr": "exp(-x/3)*cos(2*x)", "label": "smorzata" }] }),
            json!({ "series": [{ "type": "bar", "x": ["gen", "feb", "mar"], "y": [3, 5, 2] }], "title": "Vendite" }),
            json!({ "series": [{ "type": "parametric", "x_expr": "cos(t)", "y_expr": "sin(t)", "t": [0, 6.2832] }], "x": { "min": -1.5, "max": 1.5 } }),
        ] {
            let src = typst_source(&spec).unwrap();
            crate::typstworld::compile(src.clone()).unwrap_or_else(|e| panic!("{e:#}\n{src}"));
        }
        assert!(compile_expr("2x", "x").is_err() || compile_expr("2*x", "x").is_ok());
        assert!((compile_expr("x^2 + pi", "x").unwrap()(1.0) - (1.0 + std::f64::consts::PI)).abs() < 1e-9);
    }
}
