//! Testo e formule scritti «in bella», a mano: una riga con testo e
//! matematica in LaTeX (`Risolvi: $x^2 + 2x + 1 = 0$`) diventa tratti di
//! penna, con la matematica impaginata come sul quaderno ordinato: frazioni
//! in colonna, esponenti e pedici piccoli, radici col loro segno, graffe dei
//! sistemi, parentesi alte quanto il contenuto, sommatorie e integrali.
//!
//! Le misure sono nelle unità di Hershey (le maiuscole alte 21), con la riga
//! di base a y = 0 e la y verso il basso; chi disegna le scala.

use crate::handwrite::{char_glyph, Style};

/// Un pezzo impaginato: larghezza, quanto sale sopra e scende sotto la riga
/// di base, e i tratti.
#[derive(Clone, Debug, Default)]
pub struct MBox {
    pub w: f32,
    pub asc: f32,
    pub desc: f32,
    pub strokes: Vec<Vec<(f32, f32)>>,
}

impl MBox {
    fn empty(w: f32) -> MBox {
        MBox { w, asc: 0.0, desc: 0.0, strokes: Vec::new() }
    }

    fn shifted(mut self, dx: f32, dy: f32) -> MBox {
        for s in &mut self.strokes {
            for p in s.iter_mut() {
                p.0 += dx;
                p.1 += dy;
            }
        }
        self
    }

    fn scaled(mut self, k: f32) -> MBox {
        for s in &mut self.strokes {
            for p in s.iter_mut() {
                p.0 *= k;
                p.1 *= k;
            }
        }
        self.w *= k;
        self.asc *= k;
        self.desc *= k;
        self
    }

    /// Mette `other` a `dx`, con la sua riga di base spostata di `dy`.
    fn add(&mut self, other: MBox, dx: f32, dy: f32) {
        self.asc = self.asc.max(other.asc - dy);
        self.desc = self.desc.max(other.desc + dy);
        self.w = self.w.max(dx + other.w);
        self.strokes.extend(other.shifted(dx, dy).strokes);
    }

    fn line(&mut self, a: (f32, f32), b: (f32, f32)) {
        self.strokes.push(vec![a, b]);
    }
}

/// Altezza dell'asse della matematica (il centro di «=» e della linea di
/// frazione), sopra la riga di base.
const AXIS: f32 = 8.0;
/// Le maiuscole (unità di Hershey).
const CAP: f32 = 21.0;

/// Un carattere.
fn glyph_box(style: Style, c: char) -> MBox {
    // con la tua scrittura: il tuo segno, se l'hai addestrato
    if crate::campioni::mode() != crate::campioni::Mode::Chiaro {
        if let Some((adv, st)) = crate::campioni::glyph(c) {
            return from_strokes(adv, st);
        }
    }
    if c == '−' {
        // il meno: largo come il più, all'altezza della sua stanghetta
        let plus = glyph_box(style, '+');
        let y = plus.strokes.iter().filter(|s| s.len() >= 2 && (s[0].1 - s[s.len() - 1].1).abs() < 2.0).map(|s| (s[0].1 + s[s.len() - 1].1) / 2.0).next().unwrap_or(-AXIS);
        let w = plus.w.max(10.0);
        return MBox { w, asc: -y, desc: 0.0, strokes: vec![vec![(w * 0.15, y), (w * 0.85, y)]] };
    }
    match char_glyph(style, c) {
        Some(g) => from_strokes(g.adv, g.strokes),
        None => MBox::empty(8.0),
    }
}

fn from_strokes(w: f32, strokes: Vec<Vec<(f32, f32)>>) -> MBox {
    let (mut asc, mut desc) = (0.0f32, 0.0f32);
    for p in strokes.iter().flatten() {
        asc = asc.max(-p.1);
        desc = desc.max(p.1);
    }
    MBox { w, asc, desc, strokes }
}

/// Testo con «la tua mano»: le parole che il modello ha già scritto, alte
/// come il carattere chiaro (stesso corpo delle minuscole); le altre (con
/// segni che il modello non sa) lettera per lettera.
fn hand_text_box(style: Style, text: &str) -> MBox {
    let words: Vec<&str> = text.split(' ').collect();
    let inks: Vec<Option<crate::mano::WordInk>> = words.iter().map(|w| crate::mano::word(w)).collect();
    let mut xs: Vec<f32> = inks.iter().flatten().map(|w| w.xh).collect();
    xs.sort_by(f32::total_cmp);
    // il corpo delle minuscole del carattere chiaro (la sua x)
    let body = char_glyph(style, 'x').map_or(crate::mano::XH, |g| g.strokes.iter().flatten().map(|p| -p.1).fold(0.0f32, f32::max)).max(5.0);
    let scale = xs.get(xs.len() / 2).map_or(1.0, |m| body / m);
    let space = 0.6 * body;
    let mut b = MBox::default();
    let mut x = 0.0;
    for (k, (w, ink)) in words.iter().zip(inks).enumerate() {
        if k > 0 {
            x += space;
        }
        if w.is_empty() {
            continue;
        }
        let part = match ink {
            Some(ink) => {
                let st: Vec<Vec<(f32, f32)>> = ink.strokes.iter().map(|s| s.iter().map(|p| (p.0 * scale, p.1 * scale)).collect()).collect();
                let wd = st.iter().flatten().map(|p| p.0).fold(0.0f32, f32::max);
                from_strokes(wd, st)
            }
            None => plain_text_box(style, w),
        };
        let wd = part.w;
        b.add(part, x, 0.0);
        x += wd;
    }
    b.w = b.w.max(x);
    b
}

/// Testo semplice (senza formule), su una riga.
pub fn text_box(style: Style, text: &str) -> MBox {
    match crate::campioni::mode() {
        crate::campioni::Mode::Mano => hand_text_box(style, text),
        crate::campioni::Mode::Scrittura(_) => crate::campioni::text_box(text).map_or_else(|| plain_text_box(style, text), |(w, st)| from_strokes(w, st)),
        crate::campioni::Mode::Chiaro => plain_text_box(style, text),
    }
}

fn plain_text_box(style: Style, text: &str) -> MBox {
    let mut b = MBox::default();
    let space = glyph_box(style, ' ').w.max(7.0);
    let mut x = 0.0;
    for c in text.chars() {
        if c == ' ' {
            x += space;
            continue;
        }
        let g = glyph_box(style, c);
        let w = g.w;
        b.add(g, x, 0.0);
        x += w;
    }
    b.w = b.w.max(x);
    b
}

/// Una riga con testo e formule fra `$…$` (o `\(…\)`, `$$…$$`).
pub fn line_box(style: Style, line: &str) -> MBox {
    let t = line.replace("$$", "$").replace("\\(", "$").replace("\\)", "$").replace("\\[", "$").replace("\\]", "$");
    let t = t.replace("**", "").replace('`', "");
    let mut out = MBox::default();
    let mut x = 0.0;
    for (k, part) in t.split('$').enumerate() {
        if part.is_empty() {
            continue;
        }
        let b = if k % 2 == 1 { math_box(style, part) } else { text_box(style, part) };
        let w = b.w;
        out.add(b, x, 0.0);
        x += w;
        // fra testo e formula, uno spazietto se non c'è
        if k % 2 == 1 {
            x += 2.0;
        }
    }
    out.w = out.w.max(x);
    out
}

/// Una formula LaTeX.
pub fn math_box(style: Style, src: &str) -> MBox {
    let toks = tokenize(src);
    let mut p = Parser { t: toks, i: 0, style };
    let items = p.row(&[]);
    hlist(style, items, 1.0)
}

// ---------------------------------------------------------------------------
// LaTeX: pezzi e lettura

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Cmd(String),
    Ch(char),
    Open,
    Close,
    Sup,
    Sub,
    Amp,
    Newline,
}

fn tokenize(s: &str) -> Vec<Tok> {
    let mut out = Vec::new();
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        match c {
            '\\' => {
                if i + 1 < cs.len() && cs[i + 1] == '\\' {
                    out.push(Tok::Newline);
                    i += 2;
                    continue;
                }
                let mut j = i + 1;
                while j < cs.len() && cs[j].is_ascii_alphabetic() {
                    j += 1;
                }
                if j == i + 1 && j < cs.len() {
                    // \{ \} \, \; \! \| …
                    out.push(Tok::Cmd(cs[j].to_string()));
                    i = j + 1;
                } else {
                    out.push(Tok::Cmd(cs[i + 1..j].iter().collect()));
                    i = j;
                }
                continue;
            }
            '{' => out.push(Tok::Open),
            '}' => out.push(Tok::Close),
            '^' => out.push(Tok::Sup),
            '_' => out.push(Tok::Sub),
            '&' => out.push(Tok::Amp),
            c if c.is_whitespace() => {}
            c => out.push(Tok::Ch(c)),
        }
        i += 1;
    }
    out
}

/// Un elemento di una riga di formula.
#[derive(Clone, Debug)]
enum Item {
    /// Un pezzo pronto, e che tipo è (per gli spazi).
    Atom(MBox, Kind),
    /// Esponente e/o pedice attaccati al pezzo prima.
    Scripts(Box<Item>, Option<MBox>, Option<MBox>),
    /// ∑ ∏ con i limiti sopra e sotto.
    BigOp(MBox, Option<MBox>, Option<MBox>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Ord,
    /// + − × ·: spazio medio ai lati.
    Bin,
    /// = < ≤ → ⇒: spazio largo.
    Rel,
    Open,
    Close,
    Punct,
}

struct Parser {
    t: Vec<Tok>,
    i: usize,
    style: Style,
}

const RELS: &str = "=<>≤≥≠≈≡→←⇒⇔∈⊂⊃∝∼";
const BINS: &str = "+−-×·÷±∓∪∩";

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.t.get(self.i)
    }

    fn next(&mut self) -> Option<Tok> {
        let t = self.t.get(self.i).cloned();
        self.i += 1;
        t
    }

    /// Fino alla graffa che chiude, `\\`, `&`, `\right`, `\end`.
    fn row(&mut self, stop: &[&str]) -> Vec<Item> {
        let mut items: Vec<Item> = Vec::new();
        while let Some(t) = self.peek().cloned() {
            match &t {
                Tok::Close | Tok::Amp | Tok::Newline => break,
                Tok::Cmd(c) if c == "right" || c == "end" || stop.contains(&c.as_str()) => break,
                Tok::Sup | Tok::Sub => {
                    self.next();
                    let arg = self.arg();
                    let s = hlist(self.style, arg, 0.7);
                    let base = items.pop().unwrap_or(Item::Atom(MBox::empty(0.0), Kind::Ord));
                    let it = match base {
                        Item::Scripts(b, sup, sub) => {
                            if t == Tok::Sup {
                                Item::Scripts(b, Some(s), sub)
                            } else {
                                Item::Scripts(b, sup, Some(s))
                            }
                        }
                        Item::BigOp(b, sup, sub) => {
                            if t == Tok::Sup {
                                Item::BigOp(b, Some(s), sub)
                            } else {
                                Item::BigOp(b, sup, Some(s))
                            }
                        }
                        other => {
                            if t == Tok::Sup {
                                Item::Scripts(Box::new(other), Some(s), None)
                            } else {
                                Item::Scripts(Box::new(other), None, Some(s))
                            }
                        }
                    };
                    items.push(it);
                }
                _ => {
                    if let Some(it) = self.atom() {
                        items.push(it);
                    }
                }
            }
        }
        items
    }

    /// L'argomento di ^ _ \frac…: un gruppo `{…}` o un pezzo solo.
    fn arg(&mut self) -> Vec<Item> {
        match self.peek() {
            Some(Tok::Open) => {
                self.next();
                let r = self.row(&[]);
                if self.peek() == Some(&Tok::Close) {
                    self.next();
                }
                r
            }
            Some(_) => self.atom().into_iter().collect(),
            None => Vec::new(),
        }
    }

    fn arg_box(&mut self, k: f32) -> MBox {
        let a = self.arg();
        hlist(self.style, a, k)
    }

    /// Il testo di `{…}` così com'è (per \text, \begin{cases}).
    fn raw_group(&mut self) -> String {
        let mut s = String::new();
        if self.peek() != Some(&Tok::Open) {
            if let Some(Tok::Ch(c)) = self.peek().cloned() {
                self.next();
                s.push(c);
            }
            return s;
        }
        self.next();
        let mut depth = 1;
        while let Some(t) = self.next() {
            match t {
                Tok::Open => {
                    depth += 1;
                    s.push('{');
                }
                Tok::Close => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    s.push('}');
                }
                Tok::Ch(c) => s.push(c),
                Tok::Cmd(c) if c.len() == 1 => s.push_str(&c),
                Tok::Cmd(c) => {
                    s.push(' ');
                    s.push_str(&c);
                }
                _ => s.push(' '),
            }
        }
        s
    }

    fn atom(&mut self) -> Option<Item> {
        let st = self.style;
        let t = self.next()?;
        Some(match t {
            Tok::Ch(c) => {
                let c = match c {
                    '-' => '−',
                    '*' => '·',
                    '\'' => '′',
                    c => c,
                };
                if c == '′' {
                    // il primo: un trattino piccolo in alto
                    let mut b = MBox::empty(4.0);
                    b.line((3.0, -22.0), (1.5, -15.0));
                    b.asc = 22.0;
                    return Some(Item::Atom(b, Kind::Ord));
                }
                let kind = if RELS.contains(c) {
                    Kind::Rel
                } else if BINS.contains(c) {
                    Kind::Bin
                } else if "([".contains(c) {
                    Kind::Open
                } else if ")]".contains(c) {
                    Kind::Close
                } else if ",;:".contains(c) {
                    Kind::Punct
                } else {
                    Kind::Ord
                };
                Item::Atom(glyph_box(st, c), kind)
            }
            Tok::Open => {
                let r = self.row(&[]);
                if self.peek() == Some(&Tok::Close) {
                    self.next();
                }
                Item::Atom(hlist(st, r, 1.0), Kind::Ord)
            }
            Tok::Close | Tok::Amp | Tok::Newline => return None,
            Tok::Sup | Tok::Sub => return None,
            Tok::Cmd(c) => self.command(&c)?,
        })
    }

    fn command(&mut self, c: &str) -> Option<Item> {
        let st = self.style;
        let sym = |ch: char, kind: Kind| Some(Item::Atom(glyph_box(st, ch), kind));
        let space = |w: f32| Some(Item::Atom(MBox::empty(w), Kind::Punct));
        match c {
            "frac" | "dfrac" | "tfrac" | "cfrac" => {
                let num = self.arg_box(0.85);
                let den = self.arg_box(0.85);
                Some(Item::Atom(frac(num, den), Kind::Ord))
            }
            "sqrt" => {
                // \sqrt[n]{x}
                let mut index = None;
                if self.peek() == Some(&Tok::Ch('[')) {
                    self.next();
                    let mut items = Vec::new();
                    while let Some(t) = self.peek().cloned() {
                        if t == Tok::Ch(']') {
                            self.next();
                            break;
                        }
                        match self.atom() {
                            Some(it) => items.push(it),
                            None => break,
                        }
                    }
                    index = Some(hlist(st, items, 0.55));
                }
                let body = self.arg_box(1.0);
                Some(Item::Atom(sqrt(body, index), Kind::Ord))
            }
            "left" => {
                let open = self.delim_char();
                let body_items = self.row(&[]);
                let mut close = None;
                if let Some(Tok::Cmd(r)) = self.peek().cloned() {
                    if r == "right" {
                        self.next();
                        close = self.delim_char();
                    }
                }
                let body = hlist(st, body_items, 1.0);
                Some(Item::Atom(delimited(st, open, body, close), Kind::Ord))
            }
            "big" | "Big" | "bigg" | "Bigg" | "bigl" | "bigr" | "Bigl" | "Bigr" => None,
            "begin" => {
                let env = self.raw_group();
                let env = env.trim().to_string();
                if env == "array" {
                    // \begin{array}{cc}: le colonne non contano
                    let _ = self.raw_group();
                }
                let rows = self.table();
                // \end{…}
                if let Some(Tok::Cmd(e)) = self.peek().cloned() {
                    if e == "end" {
                        self.next();
                        let _ = self.raw_group();
                    }
                }
                let b = match env.as_str() {
                    "cases" | "dcases" => cases(st, rows),
                    "pmatrix" => delimited(st, Some('('), matrix(rows, true), Some(')')),
                    "bmatrix" => delimited(st, Some('['), matrix(rows, true), Some(']')),
                    "vmatrix" => delimited(st, Some('|'), matrix(rows, true), Some('|')),
                    "Bmatrix" => delimited(st, Some('{'), matrix(rows, true), Some('}')),
                    "aligned" | "align" | "align*" | "gathered" | "split" | "eqnarray" => matrix(rows, false),
                    _ => matrix(rows, true),
                };
                Some(Item::Atom(b, Kind::Ord))
            }
            "text" | "mathrm" | "textrm" | "mbox" | "operatorname" | "mathit" | "textit" | "mathbf" | "textbf" | "mathsf" => {
                let s = self.raw_group();
                Some(Item::Atom(text_box(st, &s), Kind::Ord))
            }
            "overline" | "bar" => {
                let b = self.arg_box(1.0);
                Some(Item::Atom(over(b, Over::Bar), Kind::Ord))
            }
            "vec" | "overrightarrow" => {
                let b = self.arg_box(1.0);
                Some(Item::Atom(over(b, Over::Arrow), Kind::Ord))
            }
            "hat" | "widehat" => {
                let b = self.arg_box(1.0);
                Some(Item::Atom(over(b, Over::Hat), Kind::Ord))
            }
            "underline" => {
                let b = self.arg_box(1.0);
                Some(Item::Atom(over(b, Over::Under), Kind::Ord))
            }
            "sum" => Some(Item::BigOp(glyph_box(st, 'Σ').scaled(1.25), None, None)),
            "prod" => Some(Item::BigOp(glyph_box(st, 'Π').scaled(1.25), None, None)),
            "lim" => Some(Item::BigOp(text_box(st, "lim"), None, None)),
            "int" | "oint" => Some(Item::Atom(integral(), Kind::Ord)),
            "sin" | "cos" | "tan" | "tg" | "cot" | "sen" | "log" | "ln" | "exp" | "max" | "min" | "arcsin" | "arccos" | "arctan" | "det" | "sinh"
            | "cosh" | "lg" | "mod" | "gcd" | "sup" | "inf" => {
                let mut b = text_box(st, c);
                b.w += 2.0;
                Some(Item::Atom(b, Kind::Ord))
            }
            "cdot" => sym('·', Kind::Bin),
            "times" => sym('×', Kind::Bin),
            "div" => sym('÷', Kind::Bin),
            "pm" => sym('±', Kind::Bin),
            "mp" => sym('∓', Kind::Bin),
            "cup" => sym('∪', Kind::Bin),
            "cap" => sym('∩', Kind::Bin),
            "le" | "leq" | "leqslant" => sym('≤', Kind::Rel),
            "ge" | "geq" | "geqslant" => sym('≥', Kind::Rel),
            "ne" | "neq" => sym('≠', Kind::Rel),
            "approx" | "simeq" | "sim" => sym('≈', Kind::Rel),
            "equiv" => sym('≡', Kind::Rel),
            "to" | "rightarrow" | "longrightarrow" | "mapsto" => sym('→', Kind::Rel),
            "leftarrow" | "gets" => sym('←', Kind::Rel),
            "Rightarrow" | "implies" | "Longrightarrow" => sym('⇒', Kind::Rel),
            "Leftrightarrow" | "iff" | "Longleftrightarrow" => Some(Item::Atom(text_box(st, "<=>"), Kind::Rel)),
            "in" => sym('∈', Kind::Rel),
            "notin" => {
                let mut b = glyph_box(st, '∈');
                let (w, a) = (b.w, b.asc);
                b.line((w * 0.75, -a - 2.0), (w * 0.25, 3.0));
                Some(Item::Atom(b, Kind::Rel))
            }
            "subset" | "subseteq" => sym('⊂', Kind::Rel),
            "supset" | "supseteq" => sym('⊃', Kind::Rel),
            "propto" => sym('∝', Kind::Rel),
            "infty" => sym('∞', Kind::Ord),
            "partial" => sym('∂', Kind::Ord),
            "nabla" => sym('∇', Kind::Ord),
            "exists" => sym('∃', Kind::Ord),
            "forall" => Some(Item::Atom(forall(), Kind::Ord)),
            "angle" => sym('∠', Kind::Ord),
            "perp" => sym('⊥', Kind::Rel),
            "parallel" => sym('∥', Kind::Rel),
            "circ" | "degree" => sym('°', Kind::Ord),
            "checkmark" => sym('✓', Kind::Ord),
            "ldots" | "dots" | "cdots" | "dotsc" => sym('…', Kind::Ord),
            "prime" => Some(Item::Atom(prime(), Kind::Ord)),
            "{" | "lbrace" => Some(Item::Atom(glyph_box(st, '{'), Kind::Open)),
            "}" | "rbrace" => Some(Item::Atom(glyph_box(st, '}'), Kind::Close)),
            "|" | "Vert" => sym('∥', Kind::Ord),
            "vert" | "mid" => sym('|', Kind::Ord),
            "%" => sym('%', Kind::Ord),
            "$" => sym('$', Kind::Ord),
            "," | "thinspace" => space(3.0),
            ";" | ":" | "medspace" | "thickspace" => space(5.0),
            "!" => space(-2.0),
            "quad" => space(21.0),
            "qquad" => space(42.0),
            " " => space(7.0),
            "displaystyle" | "textstyle" | "limits" | "nolimits" | "right" | "end" => None,
            name => {
                if let Some(g) = greek(name) {
                    return sym(g, Kind::Ord);
                }
                // sconosciuto: il nome come testo
                Some(Item::Atom(text_box(st, name), Kind::Ord))
            }
        }
    }

    /// Il delimitatore dopo \left o \right: ( [ { | . (niente)
    fn delim_char(&mut self) -> Option<char> {
        match self.next()? {
            Tok::Ch('.') => None,
            Tok::Ch(c) => Some(c),
            Tok::Cmd(c) => match c.as_str() {
                "{" | "lbrace" => Some('{'),
                "}" | "rbrace" => Some('}'),
                "|" | "Vert" => Some('‖'),
                "vert" => Some('|'),
                "langle" => Some('⟨'),
                "rangle" => Some('⟩'),
                _ => None,
            },
            _ => None,
        }
    }

    /// Le righe e le colonne di un ambiente (cases, matrici, aligned).
    fn table(&mut self) -> Vec<Vec<MBox>> {
        let mut rows: Vec<Vec<MBox>> = Vec::new();
        let mut cur: Vec<MBox> = Vec::new();
        loop {
            let items = self.row(&[]);
            cur.push(hlist(self.style, items, 1.0));
            match self.peek() {
                Some(Tok::Amp) => {
                    self.next();
                }
                Some(Tok::Newline) => {
                    self.next();
                    rows.push(std::mem::take(&mut cur));
                }
                Some(Tok::Close) => {
                    // una graffa che chiude fuori posto: si salta
                    self.next();
                }
                _ => {
                    rows.push(std::mem::take(&mut cur));
                    break;
                }
            }
        }
        rows.retain(|r| r.iter().any(|b| !b.strokes.is_empty()));
        rows
    }
}

fn greek(name: &str) -> Option<char> {
    Some(match name {
        "alpha" => 'α',
        "beta" => 'β',
        "gamma" => 'γ',
        "delta" => 'δ',
        "epsilon" | "varepsilon" => 'ε',
        "zeta" => 'ζ',
        "eta" => 'η',
        "theta" | "vartheta" => 'θ',
        "iota" => 'ι',
        "kappa" => 'κ',
        "lambda" => 'λ',
        "mu" => 'μ',
        "nu" => 'ν',
        "xi" => 'ξ',
        "pi" => 'π',
        "rho" | "varrho" => 'ρ',
        "sigma" | "varsigma" => 'σ',
        "tau" => 'τ',
        "upsilon" => 'υ',
        "phi" | "varphi" => 'φ',
        "chi" => 'χ',
        "psi" => 'ψ',
        "omega" => 'ω',
        "Gamma" => 'Γ',
        "Delta" => 'Δ',
        "Theta" => 'Θ',
        "Lambda" => 'Λ',
        "Xi" => 'Ξ',
        "Pi" => 'Π',
        "Sigma" => 'Σ',
        "Phi" => 'Φ',
        "Psi" => 'Ψ',
        "Omega" => 'Ω',
        _ => return None,
    })
}

// ---------------------------------------------------------------------------
// Impaginazione

/// Una riga di formula: i pezzi uno dopo l'altro, con gli spazi giusti
/// (`k`: rimpicciolita, per esponenti e frazioni).
fn hlist(style: Style, items: Vec<Item>, k: f32) -> MBox {
    let _ = style;
    let mut out = MBox::default();
    let mut x = 0.0f32;
    let mut prev: Option<Kind> = None;
    let n = items.len();
    for (idx, it) in items.into_iter().enumerate() {
        let kind = match &it {
            Item::Atom(_, k) => *k,
            Item::Scripts(b, _, _) => match **b {
                Item::Atom(_, k) => k,
                _ => Kind::Ord,
            },
            Item::BigOp(..) => Kind::Ord,
        };
        // + e − all'inizio (segno) non hanno spazio
        let gap = match kind {
            Kind::Rel => 6.0,
            Kind::Bin if prev.is_some() && prev != Some(Kind::Open) && prev != Some(Kind::Rel) && prev != Some(Kind::Bin) => 4.0,
            _ => 0.0,
        };
        let after = match kind {
            Kind::Rel => 6.0,
            Kind::Bin if gap > 0.0 => 4.0,
            Kind::Punct => 3.0,
            _ => 0.0,
        };
        x += gap;
        let b = layout_item(it);
        let w = b.w;
        out.add(b, x, 0.0);
        x += w + if idx + 1 < n { after } else { 0.0 };
        prev = Some(kind);
    }
    out.w = out.w.max(x);
    if k != 1.0 {
        out = out.scaled(k);
    }
    out
}

fn layout_item(it: Item) -> MBox {
    match it {
        Item::Atom(b, _) => b,
        Item::Scripts(base, sup, sub) => {
            let base = layout_item(*base);
            let mut out = MBox::default();
            let bw = base.w;
            let basc = base.asc;
            let bdesc = base.desc;
            out.add(base, 0.0, 0.0);
            let mut w = bw;
            if let Some(s) = sup {
                // l'esponente: la sua base a metà dell'altezza del pezzo (almeno)
                let up = (basc - 0.45 * CAP).max(0.42 * CAP) + s.desc * 0.5;
                let sw = s.w;
                out.add(s, bw + 1.0, -up);
                w = w.max(bw + 1.0 + sw);
            }
            if let Some(s) = sub {
                let down = (bdesc + 0.1 * CAP).max(0.22 * CAP);
                let sw = s.w;
                out.add(s, bw + 1.0, down);
                w = w.max(bw + 1.0 + sw);
            }
            out.w = w + 1.0;
            out
        }
        Item::BigOp(op, sup, sub) => {
            // limiti sopra e sotto, centrati
            let mut out = MBox::default();
            let w = [Some(op.w), sup.as_ref().map(|s| s.w), sub.as_ref().map(|s| s.w)].into_iter().flatten().fold(0.0, f32::max);
            let (oasc, odesc) = (op.asc, op.desc);
            let ow = op.w;
            // il simbolo centrato sull'asse
            let dy = -AXIS - (odesc - oasc) / 2.0;
            out.add(op, (w - ow) / 2.0, dy);
            if let Some(s) = sup {
                let sw = s.w;
                let sd = s.desc;
                out.add(s, (w - sw) / 2.0, dy - oasc - 3.0 - sd);
            }
            if let Some(s) = sub {
                let sw = s.w;
                let sa = s.asc;
                out.add(s, (w - sw) / 2.0, dy + odesc + 3.0 + sa);
            }
            out.w = w + 3.0;
            out
        }
    }
}

/// Frazione: numeratore sopra, linea all'altezza dell'asse, denominatore sotto.
fn frac(num: MBox, den: MBox) -> MBox {
    let w = num.w.max(den.w) + 4.0;
    let mut out = MBox::default();
    let gap = 3.5;
    let (nw, nd) = (num.w, num.desc);
    let (dw, da) = (den.w, den.asc);
    out.add(num, (w - nw) / 2.0, -AXIS - gap - nd);
    out.add(den, (w - dw) / 2.0, -AXIS + gap + da);
    out.line((0.5, -AXIS), (w - 0.5, -AXIS));
    out.asc = out.asc.max(AXIS);
    out.w = w + 1.0;
    out
}

/// Radice: il segno alto quanto il contenuto, con la linea sopra.
fn sqrt(body: MBox, index: Option<MBox>) -> MBox {
    let mut out = MBox::default();
    let top = -(body.asc.max(0.6 * CAP) + 3.0);
    let bot = body.desc.max(0.0) + 1.0;
    let mid = (top + bot) / 2.0 + 2.0;
    let x0 = index.as_ref().map(|i| (i.w - 3.0).max(0.0)).unwrap_or(0.0);
    let bw = body.w;
    out.strokes.push(vec![(x0, mid), (x0 + 2.5, mid - 1.5), (x0 + 6.0, bot), (x0 + 10.0, top), (x0 + 13.0 + bw, top)]);
    out.add(body, x0 + 12.0, 0.0);
    if let Some(i) = index {
        let id = i.desc;
        out.add(i, 0.0, mid - 2.0 - id);
    }
    out.asc = out.asc.max(-top);
    out.desc = out.desc.max(bot);
    out.w = x0 + 14.0 + bw;
    out
}

/// Parentesi alte quanto il contenuto.
fn delimited(style: Style, open: Option<char>, body: MBox, close: Option<char>) -> MBox {
    let mut out = MBox::default();
    let h_top = body.asc.max(0.7 * CAP) + 2.0;
    let h_bot = body.desc.max(0.15 * CAP) + 2.0;
    let mut x = 0.0;
    if let Some(c) = open {
        let d = tall_delim(style, c, h_top, h_bot);
        let w = d.w;
        out.add(d, 0.0, 0.0);
        x += w + 1.0;
    }
    let bw = body.w;
    out.add(body, x, 0.0);
    x += bw + 1.0;
    if let Some(c) = close {
        let d = tall_delim(style, c, h_top, h_bot);
        let w = d.w;
        out.add(d, x, 0.0);
        x += w;
    }
    out.w = x + 1.0;
    out
}

/// Un delimitatore dal glifo, allungato da `-top` a `bot`.
fn tall_delim(style: Style, c: char, top: f32, bot: f32) -> MBox {
    let h = top + bot;
    if c == '{' || c == '}' {
        return brace(c == '{', top, bot);
    }
    let c = match c {
        '‖' => '∥',
        '⟨' => '<',
        '⟩' => '>',
        c => c,
    };
    let g = glyph_box(style, c);
    let gh = (g.asc + g.desc).max(1.0);
    let ky = h / gh;
    // più alta, appena più larga
    let kx = ky.sqrt().clamp(1.0, 1.6);
    let mut out = MBox { w: g.w * kx, asc: top, desc: bot, strokes: Vec::new() };
    for s in g.strokes {
        out.strokes.push(s.into_iter().map(|(x, y)| (x * kx, -top + (y + g.asc) * ky)).collect());
    }
    out
}

/// Una graffa disegnata, alta da `-top` a `bot` (`left`: quella che apre).
fn brace(left: bool, top: f32, bot: f32) -> MBox {
    let h = top + bot;
    let w = (h * 0.16).clamp(5.0, 9.0);
    let (y0, y1) = (-top, bot);
    let mid = (y0 + y1) / 2.0;
    let r = (h * 0.12).min(6.0);
    // curve di Bézier: dall'alto a destra, giù al centro, la punta a sinistra, e di nuovo
    let cubic = |p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), p3: (f32, f32)| -> Vec<(f32, f32)> {
        (0..=8)
            .map(|k| {
                let t = k as f32 / 8.0;
                let u = 1.0 - t;
                (
                    u * u * u * p0.0 + 3.0 * u * u * t * p1.0 + 3.0 * u * t * t * p2.0 + t * t * t * p3.0,
                    u * u * u * p0.1 + 3.0 * u * u * t * p1.1 + 3.0 * u * t * t * p2.1 + t * t * t * p3.1,
                )
            })
            .collect()
    };
    let xm = w * 0.5;
    let mut pts = Vec::new();
    pts.extend(cubic((w, y0), (xm, y0), (xm, y0 + r * 0.3), (xm, y0 + r)));
    pts.extend(cubic((xm, y0 + r), (xm, mid - r * 0.6), (xm, mid - r * 0.6), (0.0, mid)).into_iter().skip(1));
    pts.extend(cubic((0.0, mid), (xm, mid + r * 0.6), (xm, mid + r * 0.6), (xm, y1 - r)).into_iter().skip(1));
    pts.extend(cubic((xm, y1 - r), (xm, y1 - r * 0.3), (xm, y1), (w, y1)).into_iter().skip(1));
    let pts: Vec<(f32, f32)> = if left { pts } else { pts.into_iter().map(|(x, y)| (w - x, y)).collect() };
    MBox { w: w + 2.0, asc: top, desc: bot, strokes: vec![pts] }
}

/// Un sistema: la graffa a sinistra e le righe una sotto l'altra, centrate sull'asse.
fn cases(style: Style, rows: Vec<Vec<MBox>>) -> MBox {
    let body = matrix(rows, false);
    let top = body.asc;
    let bot = body.desc;
    let mut out = MBox::default();
    let b = brace(true, top + 2.0, bot + 2.0);
    let bw = b.w;
    out.add(b, 0.0, 0.0);
    let w = body.w;
    out.add(body, bw + 3.0, 0.0);
    out.w = bw + 3.0 + w;
    let _ = style;
    out
}

/// Righe e colonne (allineate a sinistra se `center` è falso), centrate
/// sull'asse della formula.
fn matrix(rows: Vec<Vec<MBox>>, center: bool) -> MBox {
    if rows.is_empty() {
        return MBox::default();
    }
    let ncol = rows.iter().map(|r| r.len()).max().unwrap_or(1);
    let colw: Vec<f32> = (0..ncol).map(|c| rows.iter().filter_map(|r| r.get(c)).map(|b| b.w).fold(0.0, f32::max)).collect();
    let gap = if center { 10.0 } else { 3.0 };
    let row_h: Vec<(f32, f32)> = rows.iter().map(|r| (r.iter().map(|b| b.asc).fold(0.6 * CAP, f32::max), r.iter().map(|b| b.desc).fold(0.2 * CAP, f32::max))).collect();
    let total: f32 = row_h.iter().map(|(a, d)| a + d).sum::<f32>() + 5.0 * (rows.len() as f32 - 1.0);
    let mut out = MBox::default();
    // la prima riga parte da qui (y della sua cima), così il tutto sta centrato sull'asse
    let mut y = -AXIS - total / 2.0;
    for (r, row) in rows.into_iter().enumerate() {
        let (a, d) = row_h[r];
        let base = y + a;
        let mut x = 0.0;
        for (c, b) in row.into_iter().enumerate() {
            let bw = b.w;
            let dx = if center { (colw[c] - bw) / 2.0 } else { 0.0 };
            out.add(b, x + dx, base);
            x += colw[c] + gap;
        }
        y = base + d + 5.0;
    }
    out.w = colw.iter().sum::<f32>() + gap * (ncol as f32 - 1.0);
    out
}

enum Over {
    Bar,
    Arrow,
    Hat,
    Under,
}

/// Linea, freccia o cappello sopra (o linea sotto) un pezzo.
fn over(body: MBox, kind: Over) -> MBox {
    let mut out = MBox::default();
    let (w, asc, desc) = (body.w, body.asc.max(0.6 * CAP), body.desc);
    out.add(body, 0.0, 0.0);
    let y = -asc - 3.0;
    match kind {
        Over::Bar => out.line((0.5, y), (w - 0.5, y)),
        Over::Arrow => {
            out.line((0.5, y), (w, y));
            out.strokes.push(vec![(w - 3.0, y - 2.0), (w, y), (w - 3.0, y + 2.0)]);
        }
        Over::Hat => out.strokes.push(vec![(w / 2.0 - 4.0, y + 1.5), (w / 2.0, y - 2.0), (w / 2.0 + 4.0, y + 1.5)]),
        Over::Under => {
            let y = desc + 3.0;
            out.line((0.5, y), (w - 0.5, y));
            out.desc = y;
        }
    }
    out.asc = out.asc.max(-y + 2.0);
    out.w = w;
    out
}

/// ∫ alto, disegnato.
fn integral() -> MBox {
    let pts: Vec<(f32, f32)> = (0..=30)
        .map(|k| {
            let t = k as f32 / 30.0;
            let y = -28.0 + t * 36.0;
            let x = 6.0 - 3.0 * (t - 0.5) * 2.0 + if t < 0.1 { (0.1 - t) * 40.0 } else if t > 0.9 { -(t - 0.9) * 40.0 } else { 0.0 };
            (x, y)
        })
        .collect();
    MBox { w: 12.0, asc: 28.0, desc: 8.0, strokes: vec![pts] }
}

fn forall() -> MBox {
    MBox { w: 14.0, asc: CAP, desc: 0.0, strokes: vec![vec![(1.0, -CAP), (7.0, 0.0), (13.0, -CAP)], vec![(3.5, -9.0), (10.5, -9.0)]] }
}

fn prime() -> MBox {
    MBox { w: 4.0, asc: 22.0, desc: 0.0, strokes: vec![vec![(3.0, -22.0), (1.5, -15.0)]] }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bx(s: &str) -> MBox {
        line_box(Style::Naturale, s)
    }

    #[test]
    fn frazione_in_colonna() {
        let f = bx("$\\frac{a+b}{2}$");
        let a = bx("$a+b$");
        // più alta di una riga sola, sopra e sotto la riga di base
        assert!(f.asc > a.asc + 5.0 && f.desc > 5.0, "{:?} {:?}", (f.asc, f.desc), (a.asc, a.desc));
        // e più stretta della scritta in fila
        assert!(f.w < bx("$(a+b)/2$").w);
    }

    #[test]
    fn esponente_in_alto_e_piccolo() {
        let x = bx("$x$");
        let x2 = bx("$x^2$");
        assert!(x2.asc > x.asc + 3.0);
        assert!(x2.w < x.w + 16.0, "{} {}", x.w, x2.w);
    }

    #[test]
    fn sistema_con_la_graffa() {
        let s = bx("$\\begin{cases} x + y = 3 \\\\ x - y = 1 \\end{cases}$");
        let one = bx("$x + y = 3$");
        assert!(s.asc + s.desc > 1.8 * (one.asc + one.desc), "{:?}", (s.asc, s.desc));
        assert!(s.w > one.w);
    }

    #[test]
    fn con_la_tua_mano_senza_dati() {
        // senza parole scritte né segni: lettera per lettera, come prima
        let t = "Risolvi: $x^2+2x+1=0$";
        let a = bx(t);
        let b = crate::campioni::using_store(crate::campioni::Mode::Mano, None, || bx(t));
        assert!(b.w > 20.0 && !b.strokes.is_empty());
        assert_eq!(a.strokes.len(), b.strokes.len());
    }

    #[test]
    fn x_alta_come_il_corpo() {
        // il corpo delle minuscole del carattere chiaro è quello di `mano::XH`
        let g = glyph_box(Style::Naturale, 'x');
        assert!((g.asc - crate::mano::XH).abs() < 3.0, "{}", g.asc);
    }

    #[test]
    fn tutto_scritto() {
        for t in [
            "Risolvi: $x^2+2x+1=0$",
            "$\\sqrt{x^2+y^2} \\le \\frac{1}{2}$",
            "$\\sum_{i=1}^{n} i = \\frac{n(n+1)}{2}$",
            "$\\int_0^1 x\\,dx = \\frac{1}{2}$",
            "$\\left( \\frac{a}{b} \\right)^2 \\cdot \\alpha + \\beta \\neq \\pi$",
            "$\\vec{v} = \\overline{AB}$, $f'(x) = \\lim_{h \\to 0} \\frac{f(x+h)-f(x)}{h}$",
            "$\\begin{pmatrix} 1 & 2 \\\\ 3 & 4 \\end{pmatrix}$ e $\\{1, 2, 3\\}$",
            "$\\frac{1}{x$ (non chiusa)",
        ] {
            let b = bx(t);
            assert!(b.w > 20.0 && !b.strokes.is_empty(), "{t}");
            assert!(b.strokes.iter().flatten().all(|p| p.0.is_finite() && p.1.is_finite()), "{t}");
        }
    }

    /// Le formule a confronto: MATH_PNG=file.png cargo test -- --ignored formule_a_mano
    #[test]
    #[ignore]
    fn formule_a_mano() {
        let Ok(out) = std::env::var("MATH_PNG") else { return };
        let lines = [
            "Risolvi: $x^2+2x+1=0 \\Rightarrow (x+1)^2=0 \\Rightarrow x=-1$",
            "$\\frac{a+b}{2} \\ge \\sqrt{ab}$ e $\\sqrt[3]{x+1}$",
            "$\\begin{cases} 2x + y = 5 \\\\ x - y = 1 \\end{cases}$  sistema",
            "$\\sum_{i=1}^{n} i = \\frac{n(n+1)}{2}$, $\\int_0^1 x^2\\,dx = \\frac{1}{3}$",
            "$\\left( \\frac{a}{b} \\right)^2 + \\alpha\\beta \\neq \\pi$, $\\{x \\in R : x > 0\\}$",
            "$f'(x) = \\lim_{h \\to 0} \\frac{f(x+h)-f(x)}{h}$",
        ];
        let aspect = 1.4;
        let cap = 0.035;
        let mut strokes = Vec::new();
        let mut v = 0.08;
        for l in lines {
            let b = bx(l);
            let s = cap / CAP;
            v += b.asc * s + 0.01;
            for st in b.strokes {
                strokes.push(crate::scene::PageStroke {
                    color: [30, 60, 160],
                    width: 2.0,
                    pts: st.into_iter().map(|(x, y)| (0.04 + x * s / aspect, v + y * s)).collect(),
                    highlight: false,
                });
            }
            v += b.desc * s + 0.035;
        }
        let page = crate::scene::Page { w: aspect, h: 1.0, strokes, bg: None };
        std::fs::write(out, crate::export::page_to_png(&page, 1500).unwrap()).unwrap();
    }
}
