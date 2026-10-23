//! Gli appunti (il Markdown delle trascrizioni) in altri linguaggi:
//! - LaTeX (`.tex`): un documento che si compila così com'è (pdflatex,
//!   Overleaf), le formule restano quelle scritte;
//! - Typst (`.typ`): le formule LaTeX passano dal pacchetto MiTeX
//!   (`#mi(...)`, `#mitex(...)`), come fa l'app per disegnarle;
//! - testo semplice: per lo strato di testo dei PDF ricercabili.
//!
//! Basta il Markdown che scrive l'AI quando trascrive: titoli, paragrafi,
//! elenchi (anche con le caselle), tabelle, citazioni, codice, formule,
//! figure, grassetto, corsivo, codice in linea e collegamenti.

/// Pacchetto di Typst per le formule LaTeX (lo stesso MiTeX che usa l'app).
pub const MITEX_PKG: &str = "@preview/mitex:0.2.4";

/// Un pezzo di una riga di testo.
#[derive(Debug, Clone, PartialEq)]
enum Inline {
    Text(String),
    Bold(Vec<Inline>),
    Italic(Vec<Inline>),
    Code(String),
    Math(String),
    Link(Vec<Inline>, String),
    Image(String, String),
}

/// Un blocco dell'appunto.
#[derive(Debug, Clone, PartialEq)]
enum Block {
    Heading(usize, String),
    Para(String),
    /// Elenco: numerato?, voci (casella: None = senza, Some(fatto)).
    List(bool, Vec<(Option<bool>, String)>),
    Table(Vec<String>, Vec<Vec<String>>),
    Math(String),
    Code(String),
    Quote(String),
    Rule,
}

fn parse_blocks(md: &str) -> Vec<Block> {
    let lines: Vec<&str> = md.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let mut para: Vec<String> = Vec::new();
    let flush = |para: &mut Vec<String>, out: &mut Vec<Block>| {
        if !para.is_empty() {
            out.push(Block::Para(para.join(" ")));
            para.clear();
        }
    };
    while i < lines.len() {
        let raw = lines[i];
        let t = raw.trim();
        if t.is_empty() {
            flush(&mut para, &mut out);
            i += 1;
            continue;
        }
        // codice
        if t.starts_with("```") || t.starts_with("~~~") {
            flush(&mut para, &mut out);
            let fence = &t[..3];
            let mut code = Vec::new();
            i += 1;
            while i < lines.len() && !lines[i].trim().starts_with(fence) {
                code.push(lines[i]);
                i += 1;
            }
            out.push(Block::Code(code.join("\n")));
            i += 1;
            continue;
        }
        // formula su più righe: $$ … $$ o \[ … \]
        let (open, close) = if t.starts_with("$$") { ("$$", "$$") } else if t.starts_with("\\[") { ("\\[", "\\]") } else { ("", "") };
        if !open.is_empty() {
            flush(&mut para, &mut out);
            let rest = &t[open.len()..];
            if let Some(end) = rest.find(close) {
                out.push(Block::Math(rest[..end].trim().to_string()));
                i += 1;
                continue;
            }
            let mut m = vec![rest.to_string()];
            i += 1;
            while i < lines.len() {
                let l = lines[i];
                if let Some(end) = l.find(close) {
                    m.push(l[..end].to_string());
                    i += 1;
                    break;
                }
                m.push(l.to_string());
                i += 1;
            }
            out.push(Block::Math(m.join("\n").trim().to_string()));
            continue;
        }
        if let Some(h) = heading(t) {
            flush(&mut para, &mut out);
            out.push(h);
            i += 1;
            continue;
        }
        if matches!(t, "---" | "***" | "___") || (t.len() >= 3 && t.chars().all(|c| c == '-')) {
            flush(&mut para, &mut out);
            out.push(Block::Rule);
            i += 1;
            continue;
        }
        // tabella: riga con | e poi la riga dei trattini
        if t.starts_with('|') && i + 1 < lines.len() && is_table_sep(lines[i + 1]) {
            flush(&mut para, &mut out);
            let head = cells(t);
            i += 2;
            let mut rows = Vec::new();
            while i < lines.len() && lines[i].trim().starts_with('|') {
                rows.push(cells(lines[i].trim()));
                i += 1;
            }
            out.push(Block::Table(head, rows));
            continue;
        }
        if let Some((num, _)) = list_item(t) {
            flush(&mut para, &mut out);
            let mut items = Vec::new();
            while i < lines.len() {
                let l = lines[i].trim();
                match list_item(l) {
                    Some((n, (bx, text))) if n == num => items.push((bx, text)),
                    // una riga che continua la voce di prima
                    None if !l.is_empty() && lines[i].starts_with("  ") && !items.is_empty() => {
                        let last: &mut (Option<bool>, String) = items.last_mut().unwrap();
                        last.1.push(' ');
                        last.1.push_str(l);
                    }
                    _ => break,
                }
                i += 1;
            }
            out.push(Block::List(num, items));
            continue;
        }
        if let Some(q) = t.strip_prefix('>') {
            flush(&mut para, &mut out);
            let mut qs = vec![q.trim().to_string()];
            i += 1;
            while i < lines.len() {
                match lines[i].trim().strip_prefix('>') {
                    Some(q) => qs.push(q.trim().to_string()),
                    None => break,
                }
                i += 1;
            }
            out.push(Block::Quote(qs.join(" ")));
            continue;
        }
        para.push(t.to_string());
        i += 1;
    }
    flush(&mut para, &mut out);
    out
}

fn heading(t: &str) -> Option<Block> {
    let n = t.chars().take_while(|&c| c == '#').count();
    if (1..=6).contains(&n) && t[n..].starts_with(' ') {
        return Some(Block::Heading(n, t[n..].trim().trim_end_matches('#').trim().to_string()));
    }
    None
}

fn is_table_sep(l: &str) -> bool {
    let t = l.trim();
    t.starts_with('|') && t.contains('-') && t.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
}

fn cells(row: &str) -> Vec<String> {
    let t = row.trim().trim_start_matches('|').trim_end_matches('|');
    // le | dentro una formula non dividono le celle
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut math = false;
    let mut prev = ' ';
    for c in t.chars() {
        if c == '$' && prev != '\\' {
            math = !math;
        }
        if c == '|' && !math && prev != '\\' {
            out.push(cur.trim().to_string());
            cur.clear();
        } else {
            cur.push(c);
        }
        prev = c;
    }
    out.push(cur.trim().to_string());
    out
}

/// Una voce di elenco: (numerato, (casella, testo)).
fn list_item(t: &str) -> Option<(bool, (Option<bool>, String))> {
    let (num, rest) = if let Some(r) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")).or_else(|| t.strip_prefix("+ ")) {
        (false, r)
    } else {
        let d = t.chars().take_while(|c| c.is_ascii_digit()).count();
        if d == 0 || d > 3 {
            return None;
        }
        let r = &t[d..];
        (true, r.strip_prefix(". ").or_else(|| r.strip_prefix(") "))?)
    };
    let rest = rest.trim_start();
    for (mark, done) in [("[ ] ", false), ("[x] ", true), ("[X] ", true)] {
        if let Some(r) = rest.strip_prefix(mark) {
            return Some((num, (Some(done), r.trim().to_string())));
        }
    }
    Some((num, (None, rest.trim().to_string())))
}

fn parse_inline(s: &str) -> Vec<Inline> {
    let c: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut text = String::new();
    let mut i = 0;
    let push_text = |text: &mut String, out: &mut Vec<Inline>| {
        if !text.is_empty() {
            out.push(Inline::Text(std::mem::take(text)));
        }
    };
    let find = |from: usize, pat: &[char]| -> Option<usize> { (from..c.len().saturating_sub(pat.len() - 1)).find(|&k| c[k..k + pat.len()] == *pat) };
    while i < c.len() {
        let ch = c[i];
        // carattere protetto
        if ch == '\\' && i + 1 < c.len() && !matches!(c[i + 1], '(' | '[') && c[i + 1].is_ascii_punctuation() {
            text.push(c[i + 1]);
            i += 2;
            continue;
        }
        // formule: $…$, \(…\)
        if ch == '$' {
            let dbl = c.get(i + 1) == Some(&'$');
            let pat: &[char] = if dbl { &['$', '$'] } else { &['$'] };
            if let Some(end) = find(i + pat.len(), pat) {
                push_text(&mut text, &mut out);
                out.push(Inline::Math(c[i + pat.len()..end].iter().collect::<String>().trim().to_string()));
                i = end + pat.len();
                continue;
            }
        }
        if ch == '\\' && c.get(i + 1) == Some(&'(') {
            if let Some(end) = find(i + 2, &['\\', ')']) {
                push_text(&mut text, &mut out);
                out.push(Inline::Math(c[i + 2..end].iter().collect::<String>().trim().to_string()));
                i = end + 2;
                continue;
            }
        }
        if ch == '`' {
            if let Some(end) = find(i + 1, &['`']) {
                push_text(&mut text, &mut out);
                out.push(Inline::Code(c[i + 1..end].iter().collect()));
                i = end + 1;
                continue;
            }
        }
        if (ch == '*' || ch == '_') && c.get(i + 1) == Some(&ch) {
            if let Some(end) = find(i + 2, &[ch, ch]) {
                push_text(&mut text, &mut out);
                out.push(Inline::Bold(parse_inline(&c[i + 2..end].iter().collect::<String>())));
                i = end + 2;
                continue;
            }
        }
        // corsivo: *x* o _x_ (non dentro una parola, per _)
        let word_before = i > 0 && c[i - 1].is_alphanumeric();
        if (ch == '*' || (ch == '_' && !word_before)) && c.get(i + 1).is_some_and(|n| !n.is_whitespace()) {
            if let Some(end) = find(i + 1, &[ch]) {
                if end > i + 1 {
                    push_text(&mut text, &mut out);
                    out.push(Inline::Italic(parse_inline(&c[i + 1..end].iter().collect::<String>())));
                    i = end + 1;
                    continue;
                }
            }
        }
        // figura ![alt](src) e collegamento [testo](url)
        let img = ch == '!' && c.get(i + 1) == Some(&'[');
        if ch == '[' || img {
            let start = if img { i + 2 } else { i + 1 };
            if let Some(close) = find(start, &[']', '(']) {
                if let Some(end) = find(close + 2, &[')']) {
                    let label: String = c[start..close].iter().collect();
                    let url: String = c[close + 2..end].iter().collect::<String>().trim().split_whitespace().next().unwrap_or("").to_string();
                    push_text(&mut text, &mut out);
                    out.push(if img { Inline::Image(label, url) } else { Inline::Link(parse_inline(&label), url) });
                    i = end + 1;
                    continue;
                }
            }
        }
        // <img src="…">
        if ch == '<' {
            let rest: String = c[i..].iter().collect();
            if rest.to_ascii_lowercase().starts_with("<img") {
                if let Some(end) = rest.find('>') {
                    let tag = &rest[..end];
                    let src = tag.split("src=").nth(1).and_then(|v| {
                        let q = v.chars().next()?;
                        let v = &v[q.len_utf8()..];
                        v.find(q).map(|e| v[..e].to_string())
                    });
                    if let Some(src) = src {
                        push_text(&mut text, &mut out);
                        out.push(Inline::Image(String::new(), src));
                    }
                    i += rest[..=end].chars().count();
                    continue;
                }
            }
        }
        text.push(ch);
        i += 1;
    }
    push_text(&mut text, &mut out);
    out
}

// ---------------------------------------------------------------------------
// Testo semplice
// ---------------------------------------------------------------------------

/// Una formula LaTeX come testo da cercare: i simboli comuni diventano i loro
/// caratteri (π, √, ≈…), gli altri comandi e le graffe spariscono.
fn plain_math(m: &str) -> String {
    const SYMS: &[(&str, &str)] = &[
        ("alpha", "α"), ("beta", "β"), ("gamma", "γ"), ("delta", "δ"), ("epsilon", "ε"), ("varepsilon", "ε"), ("theta", "θ"),
        ("lambda", "λ"), ("mu", "μ"), ("pi", "π"), ("rho", "ρ"), ("sigma", "σ"), ("tau", "τ"), ("phi", "φ"), ("varphi", "φ"),
        ("omega", "ω"), ("Delta", "Δ"), ("Gamma", "Γ"), ("Sigma", "Σ"), ("Omega", "Ω"), ("Pi", "Π"), ("sqrt", "√"),
        ("cdot", "·"), ("times", "×"), ("div", "÷"), ("approx", "≈"), ("leq", "≤"), ("le", "≤"), ("geq", "≥"), ("ge", "≥"),
        ("neq", "≠"), ("ne", "≠"), ("infty", "∞"), ("pm", "±"), ("to", "→"), ("rightarrow", "→"), ("Rightarrow", "⇒"),
        ("int", "∫"), ("sum", "∑"), ("partial", "∂"), ("in", "∈"), ("degree", "°"),
    ];
    let c: Vec<char> = m.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < c.len() {
        match c[i] {
            '\\' => {
                let name: String = c[i + 1..].iter().take_while(|ch| ch.is_ascii_alphabetic()).collect();
                if name.is_empty() {
                    // \, \; \  e simili: uno spazio
                    out.push(' ');
                    i += 2;
                    continue;
                }
                if let Some((_, u)) = SYMS.iter().find(|(n, _)| *n == name) {
                    out.push_str(u);
                }
                i += 1 + name.chars().count();
            }
            // \frac{a}{b}: «a b»
            '}' if c.get(i + 1) == Some(&'{') => {
                out.push(' ');
                i += 1;
            }
            '{' | '}' => i += 1,
            ch => {
                out.push(ch);
                i += 1;
            }
        }
    }
    out
}

fn plain_inline(v: &[Inline], out: &mut String) {
    for x in v {
        match x {
            Inline::Math(t) => out.push_str(&plain_math(t)),
            Inline::Text(t) | Inline::Code(t) => out.push_str(t),
            Inline::Bold(i) | Inline::Italic(i) | Inline::Link(i, _) => plain_inline(i, out),
            Inline::Image(..) => {}
        }
    }
}

fn plain(s: &str) -> String {
    let mut o = String::new();
    plain_inline(&parse_inline(s), &mut o);
    o.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Le righe di testo dell'appunto, senza Markdown (una per titolo, voce,
/// riga di tabella, paragrafo): lo strato di testo dei PDF ricercabili.
pub fn plain_lines(md: &str) -> Vec<String> {
    let mut out = Vec::new();
    for b in parse_blocks(md) {
        match b {
            Block::Heading(_, t) | Block::Para(t) | Block::Quote(t) => out.push(plain(&t)),
            Block::List(_, items) => out.extend(items.iter().map(|(_, t)| plain(t))),
            Block::Table(h, rows) => {
                out.push(h.iter().map(|c| plain(c)).collect::<Vec<_>>().join(" "));
                out.extend(rows.iter().map(|r| r.iter().map(|c| plain(c)).collect::<Vec<_>>().join(" ")));
            }
            Block::Math(m) => out.extend(m.lines().map(|l| plain_math(l).split_whitespace().collect::<Vec<_>>().join(" "))),
            Block::Code(m) => out.extend(m.lines().map(|l| l.trim().to_string())),
            Block::Rule => {}
        }
    }
    out.retain(|l| !l.trim().is_empty());
    out
}

/// Il testo da leggere ad alta voce: come `plain_lines`, ma senza il codice
/// (non si legge a voce) e con le righe di una tabella come frasi.
pub fn speech_lines(md: &str) -> Vec<String> {
    let mut out = Vec::new();
    for b in parse_blocks(md) {
        match b {
            Block::Heading(_, t) | Block::Para(t) | Block::Quote(t) => out.push(plain(&t)),
            Block::List(_, items) => out.extend(items.iter().map(|(_, t)| plain(t))),
            Block::Table(h, rows) => {
                out.push(h.iter().map(|c| plain(c)).collect::<Vec<_>>().join(", "));
                out.extend(rows.iter().map(|r| r.iter().map(|c| plain(c)).collect::<Vec<_>>().join(", ")));
            }
            Block::Math(m) => out.push(m.lines().map(|l| plain_math(l).split_whitespace().collect::<Vec<_>>().join(" ")).collect::<Vec<_>>().join(" ")),
            Block::Code(_) | Block::Rule => {}
        }
    }
    out.retain(|l| !l.trim().is_empty());
    out
}

/// Le figure locali citate (percorsi relativi).
pub fn figures(md: &str) -> Vec<String> {
    fn walk(v: &[Inline], out: &mut Vec<String>) {
        for x in v {
            match x {
                Inline::Image(_, src) if !src.contains("://") && !src.starts_with('/') && !src.contains("..") && !src.is_empty() => out.push(src.clone()),
                Inline::Bold(i) | Inline::Italic(i) | Inline::Link(i, _) => walk(i, out),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    for b in parse_blocks(md) {
        let texts: Vec<String> = match b {
            Block::Heading(_, t) | Block::Para(t) | Block::Quote(t) => vec![t],
            Block::List(_, items) => items.into_iter().map(|(_, t)| t).collect(),
            Block::Table(h, rows) => h.into_iter().chain(rows.into_iter().flatten()).collect(),
            _ => vec![],
        };
        for t in texts {
            walk(&parse_inline(&t), &mut out);
        }
    }
    out.dedup();
    out
}

// ---------------------------------------------------------------------------
// LaTeX
// ---------------------------------------------------------------------------

fn tex_escape(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => o.push_str("\\textbackslash{}"),
            '&' | '%' | '#' | '_' | '{' | '}' | '$' => {
                o.push('\\');
                o.push(c);
            }
            '~' => o.push_str("\\textasciitilde{}"),
            '^' => o.push_str("\\textasciicircum{}"),
            '<' => o.push_str("\\textless{}"),
            '>' => o.push_str("\\textgreater{}"),
            '«' => o.push_str("\\guillemetleft{}"),
            '»' => o.push_str("\\guillemetright{}"),
            _ => o.push(c),
        }
    }
    o
}

fn tex_inline(v: &[Inline], o: &mut String) {
    for x in v {
        match x {
            Inline::Text(t) => o.push_str(&tex_escape(t)),
            Inline::Bold(i) => {
                o.push_str("\\textbf{");
                tex_inline(i, o);
                o.push('}');
            }
            Inline::Italic(i) => {
                o.push_str("\\emph{");
                tex_inline(i, o);
                o.push('}');
            }
            Inline::Code(t) => o.push_str(&format!("\\texttt{{{}}}", tex_escape(t))),
            Inline::Math(m) => o.push_str(&format!("${m}$")),
            Inline::Link(i, url) => {
                o.push_str(&format!("\\href{{{}}}{{", url.replace('%', "\\%").replace('#', "\\#")));
                tex_inline(i, o);
                o.push('}');
            }
            Inline::Image(_, src) => o.push_str(&tex_figure(src)),
        }
    }
}

fn tex_figure(src: &str) -> String {
    let lower = src.to_ascii_lowercase();
    if lower.ends_with(".svg") {
        format!("\\includesvg[width=0.8\\linewidth]{{{}}}", src.trim_end_matches(".svg").trim_end_matches(".SVG"))
    } else {
        format!("\\includegraphics[width=0.8\\linewidth]{{{src}}}")
    }
}

fn tex_line(s: &str) -> String {
    let mut o = String::new();
    tex_inline(&parse_inline(s), &mut o);
    o
}

/// L'appunto come documento LaTeX completo.
pub fn to_latex(md: &str, title: &str) -> String {
    let blocks = parse_blocks(md);
    let svg = figures(md).iter().any(|f| f.to_ascii_lowercase().ends_with(".svg"));
    let mut o = String::new();
    o.push_str("% Esportato da OpenSlate (https://github.com/rmpaolillo/OpenSlate)\n");
    o.push_str("\\documentclass[a4paper,11pt]{article}\n");
    o.push_str("\\usepackage[T1]{fontenc}\n\\usepackage[utf8]{inputenc}\n\\IfFileExists{lmodern.sty}{\\usepackage{lmodern}}{}\n");
    o.push_str("\\usepackage{amsmath,amssymb}\n\\usepackage{graphicx}\n\\usepackage[margin=2.5cm]{geometry}\n");
    o.push_str("\\usepackage{enumitem}\n\\usepackage[hidelinks]{hyperref}\n");
    if svg {
        o.push_str("% le figure SVG: pacchetto svg (vuole Inkscape; su Overleaf c'è)\n\\usepackage{svg}\n");
    }
    o.push_str(&format!("\\hypersetup{{pdftitle={{{}}}}}\n", tex_escape(title)));
    o.push_str("\\setlength{\\parindent}{0pt}\n\\setlength{\\parskip}{0.6em}\n\n\\begin{document}\n\n");
    for b in blocks {
        match b {
            Block::Heading(n, t) => {
                let cmd = match n {
                    1 => "section*",
                    2 => "subsection*",
                    3 => "subsubsection*",
                    _ => "paragraph*",
                };
                o.push_str(&format!("\\{cmd}{{{}}}\n\n", tex_line(&t)));
            }
            Block::Para(t) => o.push_str(&format!("{}\n\n", tex_line(&t))),
            Block::Quote(t) => o.push_str(&format!("\\begin{{quote}}\n{}\n\\end{{quote}}\n\n", tex_line(&t))),
            Block::List(num, items) => {
                let env = if num { "enumerate" } else { "itemize" };
                o.push_str(&format!("\\begin{{{env}}}\n"));
                for (bx, t) in items {
                    let mark = match bx {
                        Some(true) => "[$\\boxtimes$] ",
                        Some(false) => "[$\\square$] ",
                        None => "",
                    };
                    o.push_str(&format!("  \\item {mark}{}\n", tex_line(&t)));
                }
                o.push_str(&format!("\\end{{{env}}}\n\n"));
            }
            Block::Table(h, rows) => {
                let n = h.len().max(rows.iter().map(|r| r.len()).max().unwrap_or(0)).max(1);
                o.push_str(&format!("\\begin{{center}}\n\\begin{{tabular}}{{|{}}}\n\\hline\n", "l|".repeat(n)));
                let row = |r: &[String], bold: bool| {
                    (0..n)
                        .map(|k| {
                            let c = tex_line(r.get(k).map(String::as_str).unwrap_or(""));
                            if bold && !c.is_empty() {
                                format!("\\textbf{{{c}}}")
                            } else {
                                c
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" & ")
                };
                o.push_str(&format!("{} \\\\\n\\hline\n", row(&h, true)));
                for r in &rows {
                    o.push_str(&format!("{} \\\\\n\\hline\n", row(r, false)));
                }
                o.push_str("\\end{tabular}\n\\end{center}\n\n");
            }
            Block::Math(m) => o.push_str(&format!("\\[\n{m}\n\\]\n\n")),
            Block::Code(c) => o.push_str(&format!("\\begin{{verbatim}}\n{c}\n\\end{{verbatim}}\n\n")),
            Block::Rule => o.push_str("\\noindent\\rule{\\linewidth}{0.4pt}\n\n"),
        }
    }
    o.push_str("\\end{document}\n");
    o
}

// ---------------------------------------------------------------------------
// Typst
// ---------------------------------------------------------------------------

fn typ_escape(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    let mut prev = ' ';
    for c in s.chars() {
        match c {
            '\\' | '*' | '_' | '`' | '$' | '#' | '[' | ']' | '<' | '>' | '@' | '~' => {
                o.push('\\');
                o.push(c);
            }
            '/' if prev == '/' => o.push_str("\\/"),
            _ => o.push(c),
        }
        prev = c;
    }
    o
}

/// Il testo di una formula come stringa di Typst (fra virgolette).
fn typ_str(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', " "))
}

fn typ_inline(v: &[Inline], o: &mut String) {
    for x in v {
        match x {
            Inline::Text(t) => o.push_str(&typ_escape(t)),
            Inline::Bold(i) => {
                o.push_str("#strong[");
                typ_inline(i, o);
                o.push(']');
            }
            Inline::Italic(i) => {
                o.push_str("#emph[");
                typ_inline(i, o);
                o.push(']');
            }
            Inline::Code(t) => o.push_str(&format!("#raw({})", typ_str(t))),
            Inline::Math(m) => o.push_str(&format!("#mi({})", typ_str(m))),
            Inline::Link(i, url) => {
                o.push_str(&format!("#link({})[", typ_str(url)));
                typ_inline(i, o);
                o.push(']');
            }
            Inline::Image(_, src) => o.push_str(&format!("#image({}, width: 80%)", typ_str(src))),
        }
    }
}

fn typ_line(s: &str) -> String {
    let mut o = String::new();
    typ_inline(&parse_inline(s), &mut o);
    // all'inizio della riga «-», «+», «=», «/» e i numeri con il punto sono markup
    let after_digits = o.trim_start_matches(|c: char| c.is_ascii_digit());
    let numbered = after_digits.len() < o.len() && (after_digits.starts_with(". ") || after_digits == ".");
    if ["- ", "+ ", "= ", "/ "].iter().any(|m| o.starts_with(m)) || matches!(o.as_str(), "-" | "+" | "=") || numbered {
        o.insert(0, '\\');
    }
    o
}

/// L'appunto come documento Typst completo.
pub fn to_typst(md: &str, title: &str) -> String {
    let mut o = String::new();
    o.push_str("// Esportato da OpenSlate (https://github.com/rmpaolillo/OpenSlate)\n");
    o.push_str(&format!("// Le formule LaTeX passano da MiTeX: `{MITEX_PKG}`.\n"));
    o.push_str(&format!("#import \"{MITEX_PKG}\": mi, mitex\n"));
    o.push_str(&format!("#set document(title: {})\n", typ_str(title)));
    o.push_str("#set page(paper: \"a4\", margin: 2.5cm)\n#set text(size: 11pt)\n#set par(justify: false)\n\n");
    for b in parse_blocks(md) {
        match b {
            Block::Heading(n, t) => o.push_str(&format!("{} {}\n\n", "=".repeat(n.min(6)), typ_line(&t))),
            Block::Para(t) => o.push_str(&format!("{}\n\n", typ_line(&t))),
            Block::Quote(t) => o.push_str(&format!("#quote(block: true)[{}]\n\n", typ_line(&t))),
            Block::List(num, items) => {
                for (bx, t) in items {
                    let mark = match bx {
                        Some(true) => "☒ ",
                        Some(false) => "☐ ",
                        None => "",
                    };
                    let line = typ_line(&t);
                    o.push_str(&format!("{} {mark}{}\n", if num { "+" } else { "-" }, line));
                }
                o.push('\n');
            }
            Block::Table(h, rows) => {
                let n = h.len().max(rows.iter().map(|r| r.len()).max().unwrap_or(0)).max(1);
                o.push_str(&format!("#table(\n  columns: {n},\n  table.header("));
                o.push_str(&(0..n).map(|k| format!("[*{}*]", typ_line(h.get(k).map(String::as_str).unwrap_or("")))).collect::<Vec<_>>().join(", "));
                o.push_str("),\n");
                for r in &rows {
                    o.push_str("  ");
                    o.push_str(&(0..n).map(|k| format!("[{}]", typ_line(r.get(k).map(String::as_str).unwrap_or("")))).collect::<Vec<_>>().join(", "));
                    o.push_str(",\n");
                }
                o.push_str(")\n\n");
            }
            Block::Math(m) => o.push_str(&format!("#mitex({})\n\n", typ_str(&m))),
            Block::Code(c) => o.push_str(&format!("#raw(block: true, {})\n\n", format!("\"{}\"", c.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")))),
            Block::Rule => o.push_str("#line(length: 100%, stroke: 0.4pt)\n\n"),
        }
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    const MD: &str = "# Physics lab: the pendulum\n\n**Goal:** measure $g$ with a *simple* pendulum.\n\n\
                      Period: $T = 2\\pi\\sqrt{L/g}$\n\n| $L$ (m) | $T$ (s) |\n|---|---|\n| 0.50 | 1.42 |\n| 1.00 | 2.01 |\n\n\
                      $$\ng = \\frac{4\\pi^2 L}{T^2}\n$$\n\n- [ ] repeat with 20 swings\n- [x] check the angle\n\n1. one\n2. two & 50% #3\n\n![fig](fig-1.svg)\n";

    #[test]
    fn blocchi() {
        let b = parse_blocks(MD);
        assert!(matches!(&b[0], Block::Heading(1, t) if t == "Physics lab: the pendulum"));
        assert!(b.iter().any(|x| matches!(x, Block::Table(h, r) if h.len() == 2 && r.len() == 2)));
        assert!(b.iter().any(|x| matches!(x, Block::Math(m) if m.contains("\\frac"))));
        assert!(b.iter().any(|x| matches!(x, Block::List(false, i) if i[0].0 == Some(false) && i[1].0 == Some(true))));
        assert!(b.iter().any(|x| matches!(x, Block::List(true, i) if i.len() == 2)));
        assert_eq!(figures(MD), vec!["fig-1.svg".to_string()]);
    }

    #[test]
    fn latex() {
        let t = to_latex(MD, "Physics lab");
        assert!(t.contains("\\section*{Physics lab: the pendulum}"));
        assert!(t.contains("\\textbf{Goal:} measure $g$ with a \\emph{simple} pendulum."));
        assert!(t.contains("$T = 2\\pi\\sqrt{L/g}$"));
        assert!(t.contains("\\begin{tabular}{|l|l|}") && t.contains("0.50 & 1.42 \\\\"));
        assert!(t.contains("\\[\ng = \\frac{4\\pi^2 L}{T^2}\n\\]"));
        assert!(t.contains("two \\& 50\\% \\#3"));
        assert!(t.contains("\\includesvg[width=0.8\\linewidth]{fig-1}") && t.contains("\\usepackage{svg}"));
        assert!(t.trim_end().ends_with("\\end{document}"));
    }

    #[test]
    fn typst() {
        let t = to_typst(MD, "Physics lab");
        assert!(t.contains("#import \"@preview/mitex:0.2.4\": mi, mitex"));
        assert!(t.contains("= Physics lab: the pendulum"));
        assert!(t.contains("#strong[Goal:] measure #mi(\"g\") with a #emph[simple] pendulum."));
        assert!(t.contains("#mi(\"T = 2\\\\pi\\\\sqrt{L/g}\")"));
        assert!(t.contains("#mitex(\"g = \\\\frac{4\\\\pi^2 L}{T^2}\")"));
        assert!(t.contains("columns: 2") && t.contains("[0.50], [1.42]"));
        assert!(t.contains("- ☐ repeat") && t.contains("+ two & 50% \\#3"));
        assert!(t.contains("#image(\"fig-1.svg\", width: 80%)"));
    }

    #[test]
    fn testo_semplice() {
        let l = plain_lines(MD);
        assert_eq!(l[0], "Physics lab: the pendulum");
        assert_eq!(l[1], "Goal: measure g with a simple pendulum.");
        assert!(l.contains(&"L (m) T (s)".to_string()));
        assert!(l.contains(&"repeat with 20 swings".to_string()));
        assert!(l.contains(&"Period: T = 2π√L/g".to_string()), "{l:?}");
        assert!(l.contains(&"g = 4π^2 L T^2".to_string()), "{l:?}");
    }
}
