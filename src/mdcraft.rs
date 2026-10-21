//! Markdown con le formule e le figure, impaginato da WordCraft con l'aspetto
//! degli Appunti.
//!
//! WordCraft legge il Markdown (titoli, elenchi, tabelle…) ma non le formule
//! LaTeX: `$…$`, `$$…$$`, `\(…\)` e `\[…\]` resterebbero testo, e il Markdown
//! si mangerebbe le barre (`\\` a capo nelle matrici). Allora qui:
//! 1. le formule e le figure (`<img src="…">` delle trascrizioni, `![…](…)`)
//!    si tolgono dal testo e al loro posto resta un segnaposto;
//! 2. WordCraft apre il Markdown e la pagina prende lo stile degli Appunti
//!    (Source Serif, titoli in Inter, tabelle leggere: vedi `style`);
//! 3. ogni segnaposto diventa una formula disegnata da Typst in stile LaTeX
//!    (vedi `mathtex`; se non riesce, un'equazione di WordCraft) o
//!    un'immagine (un SVG disegnato da VectorCraft).
//!
//! Il codice (fra apici inversi e nei blocchi ```) resta com'è.

use serde_json::json;
use std::path::{Path, PathBuf};
use wordcraft_engine::Session;

/// Margini della pagina A4 (punti) come il foglio degli Appunti.
const MARGIN: (f32, f32, f32, f32) = (57.0, 72.0, 66.0, 66.0);

/// Larghezza della colonna di testo (punti), per le figure in percentuale.
const COLUMN_PT: f32 = 595.3 - MARGIN.2 - MARGIN.3;

/// Corpo del testo e delle formule (come negli Appunti: 16 px, KaTeX a 1,21 em).
const TEXT_PT: f32 = 12.0;
const MATH_PT: f32 = 14.5;

#[derive(Debug, Clone, PartialEq)]
pub enum Piece {
    Math { latex: String, display: bool },
    Image { path: PathBuf, alt: String, pct: Option<f32> },
}

pub fn is_markdown(path: &Path) -> bool {
    matches!(path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref(), Some("md" | "markdown"))
}

fn token(k: usize) -> String {
    format!("WCOBJ{k}Z")
}

/// Il testo con i segnaposto e, in ordine, quello che va al loro posto.
pub fn split(md: &str, base: &Path) -> (String, Vec<Piece>) {
    let mut out = String::with_capacity(md.len());
    let mut pieces = Vec::new();
    let mut prose = String::new();
    let mut fence: Option<String> = None;
    for line in md.split_inclusive('\n') {
        let t = line.trim_start();
        let marker = if t.starts_with("```") {
            Some("```")
        } else if t.starts_with("~~~") {
            Some("~~~")
        } else {
            None
        };
        match (&fence, marker) {
            (None, Some(m)) if line.len() - t.len() <= 3 => {
                scan(&prose, base, &mut out, &mut pieces);
                prose.clear();
                fence = Some(m.to_string());
                out.push_str(line);
            }
            (Some(f), Some(m)) if f == m => {
                fence = None;
                out.push_str(line);
            }
            (Some(_), _) => out.push_str(line),
            (None, _) => prose.push_str(line),
        }
    }
    scan(&prose, base, &mut out, &mut pieces);
    (out, pieces)
}

fn find(c: &[char], from: usize, pat: &str) -> Option<usize> {
    let p: Vec<char> = pat.chars().collect();
    (from..c.len().saturating_sub(p.len() - 1)).find(|&i| c[i..i + p.len()] == p[..])
}

fn starts(c: &[char], i: usize, pat: &str) -> bool {
    pat.chars().enumerate().all(|(k, ch)| c.get(i + k) == Some(&ch))
}

/// Un percorso locale accanto al Markdown (niente indirizzi web, niente «..»).
fn local(base: &Path, src: &str) -> Option<PathBuf> {
    let src = src.trim();
    if src.is_empty() || src.contains("://") || src.starts_with("data:") || src.starts_with('/') || src.contains('\\') {
        return None;
    }
    let rel = Path::new(src);
    if rel.components().any(|c| !matches!(c, std::path::Component::Normal(_))) {
        return None;
    }
    let ext = rel.extension()?.to_str()?.to_ascii_lowercase();
    if !matches!(ext.as_str(), "svg" | "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp") {
        return None;
    }
    let p = base.join(rel);
    p.is_file().then_some(p)
}

/// Esporta un appunto Markdown in `dst`, secondo l'estensione:
/// - `.md`: il testo e, accanto, le figure e i grafici che cita (stessi
///   percorsi relativi, così il file si apre uguale in Typora o altrove);
/// - `.docx` e `.pdf`: impaginati da WordCraft con l'aspetto degli appunti,
///   formule comprese (come si vedono nell'app).
pub fn export_note(src: &Path, dst: &Path) -> anyhow::Result<()> {
    use anyhow::{anyhow, bail, Context};
    let ext = dst.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    match ext.as_str() {
        "md" | "markdown" => {
            let md = std::fs::read_to_string(src).with_context(|| format!("{}", src.display()))?;
            let base = src.parent().unwrap_or(Path::new("."));
            let out = dst.parent().unwrap_or(Path::new("."));
            std::fs::write(dst, &md).with_context(|| format!("{}", dst.display()))?;
            let same = std::fs::canonicalize(base).ok() == std::fs::canonicalize(out).ok();
            if !same {
                for rel in images(&md, base) {
                    let to = out.join(&rel);
                    let from = base.join(&rel);
                    // una figura con lo stesso nome che c'è già là: se è di questo
                    // appunto (il nome comincia come il suo) si aggiorna, se è
                    // un altro file non si sovrascrive (lo si dice)
                    if to.exists() && std::fs::read(&to).ok() != std::fs::read(&from).ok() {
                        let stem = src.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                        let ours = Path::new(&rel).file_name().is_some_and(|n| n.to_string_lossy().starts_with(&stem));
                        if !ours {
                            bail!("nella cartella c'è già «{rel}», diverso da quello dell'appunto: non lo sovrascrivo (scegli un'altra cartella)");
                        }
                    }
                    if let Some(d) = to.parent() {
                        std::fs::create_dir_all(d)?;
                    }
                    std::fs::copy(&from, &to).with_context(|| format!("figura {rel}"))?;
                }
            }
        }
        "docx" => {
            let mut s = crate::wordsign::open(src)?;
            s.run("file.save", &json!({ "path": dst.to_string_lossy() })).map_err(|e| anyhow!("WordCraft, Word: {e}"))?;
            if !dst.exists() {
                bail!("WordCraft non ha scritto {}", dst.display());
            }
        }
        "pdf" => crate::wordsign::to_pdf(src, dst)?,
        _ => bail!("formato non previsto: .{ext}"),
    }
    Ok(())
}

/// Le figure locali citate dal Markdown (percorsi relativi), da copiare
/// accanto a lui quando lo si importa.
pub fn images(md: &str, base: &Path) -> Vec<String> {
    let (_, pieces) = split(md, base);
    pieces
        .into_iter()
        .filter_map(|p| match p {
            Piece::Image { path, .. } => path.strip_prefix(base).ok().map(|r| r.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect()
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let mut from = 0;
    while let Some(i) = lower[from..].find(name) {
        let at = from + i;
        let before_ok = at == 0 || lower.as_bytes()[at - 1].is_ascii_whitespace();
        let rest = tag[at + name.len()..].trim_start();
        if before_ok {
            if let Some(rest) = rest.strip_prefix('=') {
                let rest = rest.trim_start();
                let q = rest.chars().next()?;
                if q == '"' || q == '\'' {
                    let end = rest[1..].find(q)?;
                    return Some(rest[1..1 + end].to_string());
                }
                return Some(rest.split(|c: char| c.is_whitespace() || c == '>' || c == '/').next()?.to_string());
            }
        }
        from = at + name.len();
    }
    None
}

fn scan(text: &str, base: &Path, out: &mut String, pieces: &mut Vec<Piece>) {
    let c: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut push = |out: &mut String, p: Piece, own_para: bool| {
        let t = token(pieces.len());
        pieces.push(p);
        if own_para {
            out.push_str("\n\n");
            out.push_str(&t);
            out.push_str("\n\n");
        } else {
            out.push_str(&t);
        }
    };
    let collect = |a: usize, b: usize| c[a..b].iter().collect::<String>();
    while i < c.len() {
        let ch = c[i];
        // codice in linea: com'è
        if ch == '`' {
            let n = (i..c.len()).take_while(|&k| c[k] == '`').count();
            let run: String = "`".repeat(n);
            let mut j = i + n;
            let mut end = None;
            while let Some(k) = find(&c, j, &run) {
                let m = (k..c.len()).take_while(|&q| c[q] == '`').count();
                if m == n {
                    end = Some(k + n);
                    break;
                }
                j = k + m;
            }
            let stop = end.unwrap_or(i + n);
            out.push_str(&collect(i, stop));
            i = stop;
            continue;
        }
        if ch == '\\' {
            // \[ … \] e \( … \)
            if let Some((close, display)) = match c.get(i + 1) {
                Some('[') => Some(("\\]", true)),
                Some('(') => Some(("\\)", false)),
                _ => None,
            } {
                if let Some(k) = find(&c, i + 2, close) {
                    let latex = collect(i + 2, k).trim().to_string();
                    if !latex.is_empty() {
                        push(out, Piece::Math { latex, display }, display);
                        i = k + 2;
                        continue;
                    }
                }
            }
            // un carattere protetto (\$, \\…) resta com'è
            out.push(ch);
            if let Some(&n) = c.get(i + 1) {
                out.push(n);
            }
            i += 2;
            continue;
        }
        if ch == '$' {
            if c.get(i + 1) == Some(&'$') {
                if let Some(k) = find(&c, i + 2, "$$") {
                    let latex = collect(i + 2, k).trim().to_string();
                    if !latex.is_empty() {
                        push(out, Piece::Math { latex, display: true }, true);
                        i = k + 2;
                        continue;
                    }
                }
                out.push_str("$$");
                i += 2;
                continue;
            }
            // $…$: niente spazio subito dentro, niente cifra subito dopo (i prezzi: «5$ e 6$»)
            let mut k = i + 1;
            let mut end = None;
            while k < c.len() {
                match c[k] {
                    '\\' => k += 2,
                    '\n' if c.get(k + 1) == Some(&'\n') => break,
                    '$' => {
                        end = Some(k);
                        break;
                    }
                    _ => k += 1,
                }
            }
            if let Some(k) = end {
                let inner = collect(i + 1, k);
                let ok = !inner.is_empty()
                    && !inner.starts_with(char::is_whitespace)
                    && !inner.ends_with(char::is_whitespace)
                    && !c.get(k + 1).is_some_and(|n| n.is_ascii_digit());
                if ok {
                    push(out, Piece::Math { latex: inner, display: false }, false);
                    i = k + 1;
                    continue;
                }
            }
            out.push('$');
            i += 1;
            continue;
        }
        // <img src="figura.svg" alt="…" width="60%"> delle trascrizioni
        if ch == '<' && c.get(i + 1..i + 5).is_some_and(|w| w.iter().collect::<String>().eq_ignore_ascii_case("img ")) {
            if let Some(k) = find(&c, i, ">") {
                let tag = collect(i, k + 1);
                if let Some(path) = attr(&tag, "src").and_then(|s| local(base, &s)) {
                    let alt = attr(&tag, "alt").unwrap_or_default();
                    let pct = attr(&tag, "width").and_then(|w| w.trim().strip_suffix('%').and_then(|n| n.trim().parse::<f32>().ok()));
                    push(out, Piece::Image { path, alt, pct }, false);
                    i = k + 1;
                    continue;
                }
            }
        }
        // ![alt](figura.svg)
        if ch == '!' && c.get(i + 1) == Some(&'[') {
            if let Some(k) = find(&c, i + 2, "](") {
                if let Some(e) = find(&c, k + 2, ")") {
                    let alt = collect(i + 2, k);
                    let src = collect(k + 2, e);
                    let src = src.split_whitespace().next().unwrap_or("").trim_matches(|c| c == '<' || c == '>').to_string();
                    if !alt.contains('\n') && !src.contains('\n') {
                        if let Some(path) = local(base, &src) {
                            push(out, Piece::Image { path, alt, pct: None }, false);
                            i = e + 1;
                            continue;
                        }
                    }
                }
            }
        }
        if starts(&c, i, "WCOBJ") {
            // un testo che somiglia a un segnaposto: si spezza, così non si confonde
            out.push_str("WCOBJ\u{200B}");
            i += 5;
            continue;
        }
        out.push(ch);
        i += 1;
    }
}

/// Mette al posto dei segnaposto le formule e le figure. Quello che non
/// riesce resta com'era scritto (il LaTeX o il testo alternativo).
pub fn place(s: &mut Session, pieces: &[Piece]) {
    for (k, p) in pieces.iter().enumerate() {
        if s.run("select.text", &json!({ "text": token(k) })).is_err() {
            continue;
        }
        let done = match p {
            Piece::Math { latex, display } => math(s, latex, *display),
            Piece::Image { path, alt, pct } => match picture(path) {
                Ok(png) => {
                    let mut v = json!({ "data": wordcraft_engine::cmd::insert::base64_encode(&png), "alt": alt });
                    if let Some(p) = pct {
                        v["width"] = json!(COLUMN_PT * p.clamp(5.0, 100.0) / 100.0);
                    }
                    s.run("insert.picture", &v).is_ok()
                }
                Err(e) => {
                    tracing::warn!("figura {}: {e:#}", path.display());
                    false
                }
            },
        };
        if !done {
            let fallback = match p {
                Piece::Math { latex, display: true } => format!("$${latex}$$"),
                Piece::Math { latex, .. } => format!("${latex}$"),
                Piece::Image { alt, .. } => format!("[{alt}]"),
            };
            let _ = s.run("text.insert", &json!({ "text": fallback }));
        }
    }
    let _ = s.run("select.collapse", &json!({}));
}

/// Una formula al posto del segnaposto selezionato: disegnata da Typst (come
/// negli Appunti); nel testo scende sotto la riga quanto serve perché la sua
/// linea di base sia quella del testo. Se Typst non ce la fa, un'equazione
/// di WordCraft.
fn math(s: &mut Session, latex: &str, display: bool) -> bool {
    match crate::mathtex::render(latex, display, MATH_PT) {
        Ok(f) => {
            let v = json!({ "data": wordcraft_engine::cmd::insert::base64_encode(&f.png), "width": f.width, "alt": latex });
            if s.run("insert.picture", &v).is_err() {
                return false;
            }
            if !display && f.depth > 0.05 {
                // la formula appena messa (il carattere prima del cursore) scende sotto la riga
                if let Ok(end) = serde_json::to_value(&s.sel.focus) {
                    let mut start = end.clone();
                    if let Some(off) = end["off"].as_u64().filter(|o| *o > 0) {
                        start["off"] = json!(off - "\u{FFFC}".len() as u64);
                        if s.run("select.range", &json!({ "anchor": start, "focus": end })).is_ok() {
                            let _ = s.run("format.position", &json!({ "points": -f.depth }));
                            let _ = s.run("select.range", &json!({ "anchor": end, "focus": end }));
                        }
                    }
                }
            }
            if display {
                // come le formule su riga propria degli Appunti: centrate, con aria attorno
                let _ = s.run("para.alignCenter", &json!({}));
                let _ = s.run("para.spacing", &json!({ "before": 14, "after": 22 }));
            }
            true
        }
        Err(e) => {
            tracing::warn!("formula {latex:?}: {e:#}");
            s.run("insert.equation", &json!({ "latex": latex, "display": display, "edit": false })).is_ok()
        }
    }
}

/// I caratteri degli Appunti (dentro l'app) nel motore di WordCraft.
fn fonts() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let db = wordcraft_fonts::FontDb::global();
        for data in [
            &include_bytes!("../assets/fonts/SourceSerif4-Regular.ttf")[..],
            include_bytes!("../assets/fonts/SourceSerif4-It.ttf"),
            include_bytes!("../assets/fonts/SourceSerif4-Bold.ttf"),
            include_bytes!("../assets/fonts/Inter-Regular.ttf"),
            include_bytes!("../assets/fonts/Inter-SemiBold.ttf"),
        ] {
            db.add_font(data.to_vec());
        }
    });
}

/// La pagina come quella degli Appunti: A4, testo in Source Serif, titoli in
/// Inter quasi neri, paragrafi ariosi, tabelle strette con i bordi grigi e
/// l'intestazione in grassetto.
pub fn style(s: &mut Session) {
    fonts();
    let _ = s.run("layout.size", &json!({ "name": "A4" }));
    let (t, b, l, r) = MARGIN;
    let _ = s.run("layout.margins", &json!({ "top": t, "bottom": b, "left": l, "right": r }));
    if let Ok(mut v) = serde_json::to_value(&s.doc.styles) {
        v["defaultChr"]["font"] = json!("Source Serif 4");
        v["defaultChr"]["size"] = json!(TEXT_PT);
        v["defaultPara"]["lineSpacing"] = json!({ "rule": "multiple", "value": 1.3 });
        v["defaultPara"]["spaceAfter"] = json!(10.0);
        let ink = json!({ "Rgb": [26, 26, 26] });
        if let Some(list) = v["styles"].as_array_mut() {
            for st in list {
                let id = st["id"].as_str().unwrap_or("").to_string();
                let heading = |size: f32, before: f32| json!({ "font": "Inter", "bold": true, "color": ink, "size": size, "italic": false, "_b": before });
                let h = match id.as_str() {
                    "Title" | "Heading1" => Some(heading(19.5, 14.0)),
                    "Heading2" => Some(heading(15.75, 12.0)),
                    "Heading3" => Some(heading(12.75, 10.0)),
                    "Heading4" | "Heading5" | "Heading6" => Some(heading(TEXT_PT, 8.0)),
                    _ => None,
                };
                if let Some(mut h) = h {
                    let before = h["_b"].take();
                    h.as_object_mut().unwrap().remove("_b");
                    for (k, val) in h.as_object().unwrap() {
                        st["chr"][k] = val.clone();
                    }
                    st["chr"].as_object_mut().map(|c| c.remove("kern"));
                    st["para"]["spaceBefore"] = before;
                    st["para"]["spaceAfter"] = json!(5.0);
                    st["para"]["lineSpacing"] = json!({ "rule": "multiple", "value": 1.1 });
                }
                if id == "Code" {
                    // i blocchi di codice degli Appunti: fondo grigio chiaro, corpo minore
                    st["chr"]["size"] = json!(10.0);
                    st["para"]["shading"] = json!([245, 245, 245]);
                    st["para"]["spaceAfter"] = json!(0.0);
                    st["para"]["spaceBefore"] = json!(0.0);
                    st["para"]["lineSpacing"] = json!({ "rule": "multiple", "value": 1.15 });
                    st["para"]["indentLeft"] = json!(4.0);
                }
                if id == "Quote" {
                    // come il riquadro delle citazioni degli Appunti: a sinistra, grigio
                    st["chr"]["italic"] = json!(false);
                    st["chr"]["color"] = json!({ "Rgb": [85, 85, 85] });
                    st["para"]["align"] = json!("left");
                    st["para"]["indentLeft"] = json!(12.0);
                    st["para"]["borders"] = json!({ "left": { "style": "single", "width": 2.25, "color": [208, 208, 208], "space": 8.0 } });
                }
            }
        }
        match serde_json::from_value(v) {
            Ok(styles) => s.doc.styles = styles,
            Err(e) => tracing::warn!("stili degli appunti: {e}"),
        }
    }
    // il codice nel testo (`…`) su un fondo grigio, come negli Appunti
    if let Ok(mut body) = serde_json::to_value(&s.doc.body) {
        fn shade(v: &mut serde_json::Value) {
            match v {
                serde_json::Value::Array(a) => a.iter_mut().for_each(shade),
                serde_json::Value::Object(o) => {
                    if let Some(serde_json::Value::Array(runs)) = o.get_mut("runs") {
                        for r in runs {
                            if r["props"]["font"].as_str().is_some_and(|f| f.eq_ignore_ascii_case("Courier New")) {
                                r["props"]["shading"] = json!([242, 242, 242]);
                                r["props"]["size"] = json!(10.5);
                            }
                        }
                    }
                    o.values_mut().for_each(shade);
                }
                _ => {}
            }
        }
        shade(&mut body);
        match serde_json::from_value(body) {
            Ok(b) => s.doc.body = b,
            Err(e) => tracing::warn!("codice nel testo: {e}"),
        }
    }
    // tabelle: una per una, dal primo paragrafo della prima cella
    let tables: Vec<usize> = serde_json::to_value(&s.doc.body)
        .ok()
        .and_then(|b| b.as_array().map(|a| a.iter().enumerate().filter(|(_, x)| x["kind"] == "table").map(|(i, _)| i).collect()))
        .unwrap_or_default();
    for i in tables {
        let pos = json!({ "story": "body", "path": [i, 0, 0, 0], "off": 0 });
        if s.run("select.range", &json!({ "anchor": pos, "focus": pos })).is_err() {
            continue;
        }
        let _ = s.run("table.selectTable", &json!({}));
        let _ = s.run("para.spacing", &json!({ "before": 0, "after": 0 }));
        let _ = s.run("para.lineSpacing", &json!({ "value": 1.15 }));
        let _ = s.run("table.borders", &json!({ "kind": "all", "width": 0.75, "color": "CCCCCC" }));
        let _ = s.run("table.cellMargins", &json!({ "top": 3, "bottom": 3, "left": 6, "right": 6 }));
        let _ = s.run("select.range", &json!({ "anchor": pos, "focus": pos }));
        let _ = s.run("table.selectRow", &json!({}));
        let _ = s.run("format.bold", &json!({ "value": true }));
        let _ = s.run("select.range", &json!({ "anchor": pos, "focus": pos }));
        let _ = s.run("table.autofit", &json!({ "mode": "contents" }));
    }
    let _ = s.run("select.collapse", &json!({}));
}

/// La figura per WordCraft: un SVG lo disegna VectorCraft, una foto va com'è.
fn picture(path: &Path) -> anyhow::Result<Vec<u8>> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    if ext == "svg" {
        crate::vecsign::to_png(path, 192.0)
    } else {
        Ok(std::fs::read(path)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// «Esporta» dell'appunto: Markdown con la figura accanto, Word e PDF.
    #[test]
    fn esporta_appunto() {
        let t = std::env::temp_dir().join(format!("openslate-esporta-{}", std::process::id()));
        let (src, out) = (t.join("exports"), t.join("fuori"));
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(src.join("a-fig1.svg"), r#"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20"><rect width="40" height="20" fill="red"/></svg>"#).unwrap();
        let note = src.join("a.md");
        std::fs::write(&note, "# Sistema\n\n$$\\begin{cases} x-1=0 \\\\ y+x=1 \\end{cases}$$\n\nLa soluzione è $x=1$.\n\n<img src=\"a-fig1.svg\" width=\"50%\">\n").unwrap();
        export_note(&note, &out.join("a.md")).unwrap();
        assert!(out.join("a-fig1.svg").is_file(), "la figura va accanto al .md");
        export_note(&note, &out.join("a.docx")).unwrap();
        let docx = std::fs::read(out.join("a.docx")).unwrap();
        assert!(docx.starts_with(b"PK") && docx.len() > 2000);
        export_note(&note, &out.join("a.pdf")).unwrap();
        assert!(std::fs::read(out.join("a.pdf")).unwrap().starts_with(b"%PDF"));
        assert!(export_note(&note, &out.join("a.xyz")).is_err());
        let _ = std::fs::remove_dir_all(&t);
    }

    fn math(p: &Piece) -> (&str, bool) {
        match p {
            Piece::Math { latex, display } => (latex.as_str(), *display),
            _ => panic!("{p:?}"),
        }
    }

    #[test]
    fn formule_tolte_dal_testo() {
        let md = "Risolvi:\n\n$$\n\\begin{cases}\nx+y=1\\\\\nx=-1\n\\end{cases}\n$$\n\n[figura: il punto $P(-1,2)$, costa 5$ e 6$]\n\n`$codice$` e \\$ e \\(a^2\\)\n\n```\n$non$ toccare\n```\n";
        let (text, pieces) = split(md, Path::new("."));
        assert_eq!(pieces.len(), 3, "{pieces:?}");
        assert_eq!(math(&pieces[0]), ("\\begin{cases}\nx+y=1\\\\\nx=-1\n\\end{cases}", true));
        assert_eq!(math(&pieces[1]), ("P(-1,2)", false));
        assert_eq!(math(&pieces[2]), ("a^2", false));
        assert!(text.contains("\n\nWCOBJ0Z\n\n") && text.contains("punto WCOBJ1Z, costa 5$ e 6$"), "{text}");
        assert!(text.contains("`$codice$` e \\$ e WCOBJ2Z") && text.contains("$non$ toccare"), "{text}");
    }

    #[test]
    fn equazioni_e_figure_in_wordcraft() {
        let dir = std::env::temp_dir().join(format!("openslate-md-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("fig.svg"),
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100" viewBox="0 0 200 100"><rect x="10" y="10" width="80" height="50" fill="#00f"/></svg>"##,
        )
        .unwrap();
        let md = dir.join("prova.md");
        std::fs::write(
            &md,
            "# Titolo\n\nSistema:\n\n$$\n\\begin{cases}\nx+y=1\\\\\nx=-1\n\\end{cases}\n$$\n\nIl punto $P(-1,2)$.\n\n<img src=\"fig.svg\" alt=\"figura\" width=\"50%\">\n",
        )
        .unwrap();
        assert_eq!(images(&std::fs::read_to_string(&md).unwrap(), &dir), vec!["fig.svg".to_string()]);
        let s = crate::wordsign::open(&md).unwrap();
        let json = serde_json::to_string(&s.doc).unwrap();
        assert!(!json.contains("WCOBJ"), "segnaposto rimasti");
        // le formule sono immagini disegnate da Typst (3) più la figura
        assert!(json.matches("image/png").count() + json.matches("\"png\"").count() >= 1 || json.contains("media"), "immagini mancanti");
        // e il PDF si fa
        let pdf = dir.join("prova.pdf");
        crate::wordsign::to_pdf(&md, &pdf).unwrap();
        let text = crate::docimport::pdf_text(&pdf).unwrap().join("\n");
        assert!(text.contains("Titolo") && !text.contains("WCOBJ") && !text.contains("begin{cases}"), "{text}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

#[cfg(test)]
mod prova_visiva {
    /// MD_IN=<file.md> MD_OUT=<cartella> cargo test -- --ignored pagina_md
    #[test]
    #[ignore]
    fn pagina_md() {
        let input = std::path::PathBuf::from(std::env::var("MD_IN").unwrap());
        let out = std::path::PathBuf::from(std::env::var("MD_OUT").unwrap());
        let pdf = out.join("md.pdf");
        crate::wordsign::to_pdf(&input, &pdf).unwrap();
        crate::docimport::render_pages(&pdf, &out, std::env::var("MD_PX").ok().and_then(|v| v.parse().ok()).unwrap_or(1100)).unwrap();
    }
}

