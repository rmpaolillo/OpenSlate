//! Dove vanno la firma e il timbro in un documento: le scritte «Firma»,
//! «Timbro e firma», «Il legale rappresentante»… e le righe da riempire
//! («______») trovate nel PDF della pagina, con la loro posizione.
//!
//! Il testo viene da hayro (il lettore PDF della suite Craft): ogni lettera
//! con il suo riquadro, raccolte in righe. Ogni documento ha il suo PDF
//! (`source.pdf`, anche i Word), quindi vale per tutti, sulla desktop e sul
//! telefono (stesso file).
//!
//! Le posizioni sono frazioni della pagina del documento (0..1, dall'alto a
//! sinistra); [`place`] le porta sul foglio della lavagna.

use anyhow::{Context as _, Result};
use hayro_interpret::font::Glyph as HGlyph;
use hayro_interpret::hayro_cmap::BfString;
use hayro_interpret::hayro_syntax::Pdf;
use hayro_interpret::{
    interpret_page, BlendMode, ClipPath, Context, Device, GlyphDrawMode, Image, InterpreterCache, InterpreterSettings, Paint, PathDrawMode,
    SoftMask, TransformExt,
};
use kurbo::{Affine, BezPath, Point, Rect, Shape};

/// Una lettera sulla pagina: il testo e il riquadro (frazioni della pagina).
#[derive(Clone, Debug)]
pub struct Letter {
    pub text: String,
    /// x0, y0, x1, y1.
    pub rect: [f32; 4],
}

/// Una riga di testo.
#[derive(Clone, Debug, Default)]
pub struct Line {
    pub letters: Vec<Letter>,
    /// Il riquadro di tutta la riga.
    pub rect: [f32; 4],
}

impl Line {
    pub fn text(&self) -> String {
        let mut s = String::new();
        let mut prev: Option<[f32; 4]> = None;
        for l in &self.letters {
            if let Some(p) = prev {
                let h = (self.rect[3] - self.rect[1]).max(1e-4);
                if l.rect[0] - p[2] > 0.5 * h && !s.ends_with(' ') && l.text != " " {
                    s.push(' ');
                }
            }
            s.push_str(&l.text);
            prev = Some(l.rect);
        }
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn height(&self) -> f32 {
        (self.rect[3] - self.rect[1]).max(1e-4)
    }

    fn is_blank(c: &str) -> bool {
        matches!(c, "_" | "." | "…" | "‗")
    }

    /// Quante lettere della riga sono righe da riempire («___», «....»).
    fn blank_share(&self) -> f32 {
        let ink: Vec<&Letter> = self.letters.iter().filter(|l| !l.text.trim().is_empty()).collect();
        if ink.is_empty() {
            return 0.0;
        }
        ink.iter().filter(|l| Self::is_blank(&l.text)).count() as f32 / ink.len() as f32
    }

    /// Le righe da riempire (almeno 4 segni di fila, senza spazi in mezzo)
    /// dalla lettera `from`: i loro riquadri, da sinistra.
    fn blank_runs(&self, from: usize) -> Vec<[f32; 4]> {
        let h = self.height();
        let mut runs: Vec<(usize, usize)> = Vec::new();
        let mut cur: Option<(usize, usize)> = None;
        for (i, l) in self.letters.iter().enumerate().skip(from) {
            let joins = |b: usize| l.rect[0] - self.letters[b].rect[2] < 0.6 * h;
            cur = match (Self::is_blank(&l.text), cur) {
                (true, Some((a, b))) if joins(b) => Some((a, i)),
                (true, prev) => {
                    runs.extend(prev);
                    Some((i, i))
                }
                (false, prev) => {
                    runs.extend(prev);
                    None
                }
            };
        }
        runs.extend(cur);
        runs.into_iter()
            .filter(|(a, b)| b + 1 - a >= 4)
            .map(|(a, b)| {
                self.letters[a..=b].iter().fold([f32::MAX, f32::MAX, f32::MIN, f32::MIN], |r, l| {
                    [r[0].min(l.rect[0]), r[1].min(l.rect[1]), r[2].max(l.rect[2]), r[3].max(l.rect[3])]
                })
            })
            .collect()
    }

    /// La prima riga da riempire dalla lettera `from`.
    fn blank_run(&self, from: usize) -> Option<[f32; 4]> {
        self.blank_runs(from).into_iter().next()
    }

    /// La riga da riempire più vicina, in orizzontale, alla x `cx`.
    fn blank_near(&self, cx: f32) -> Option<[f32; 4]> {
        let dist = |r: &[f32; 4]| if r[0] <= cx && cx <= r[2] { 0.0 } else { (r[0] - cx).abs().min((r[2] - cx).abs()) };
        self.blank_runs(0).into_iter().min_by(|a, b| dist(a).total_cmp(&dist(b)))
    }
}

/// Le righe di una pagina, dall'alto in basso.
#[derive(Clone, Debug, Default)]
pub struct PageText {
    pub lines: Vec<Line>,
}

/// Raccoglie le lettere disegnate (anche quelle invisibili, come il testo
/// letto sopra una scansione).
struct Collect {
    w: f64,
    h: f64,
    letters: Vec<(String, Rect, Point)>,
}

impl<'a> Device<'a> for Collect {
    fn set_soft_mask(&mut self, _: Option<SoftMask<'a>>) {}
    fn set_blend_mode(&mut self, _: BlendMode) {}
    fn draw_path(&mut self, _: &BezPath, _: Affine, _: &Paint<'a>, _: &PathDrawMode) {}
    fn push_clip_path(&mut self, _: &ClipPath) {}
    fn push_transparency_group(&mut self, _: f32, _: Option<SoftMask<'a>>, _: BlendMode) {}
    fn draw_glyph(&mut self, glyph: &HGlyph<'a>, transform: Affine, glyph_transform: Affine, _: &Paint<'a>, _: &GlyphDrawMode) {
        let text = match glyph.as_unicode() {
            Some(BfString::Char(c)) => c.to_string(),
            Some(BfString::String(s)) => s,
            None => return,
        };
        let t = transform * glyph_transform;
        let origin = t * Point::ORIGIN;
        let rect = match glyph {
            HGlyph::Outline(o) => {
                let path = o.outline();
                if path.elements().is_empty() {
                    Rect::from_points(origin, origin)
                } else {
                    (t * path).bounding_box()
                }
            }
            HGlyph::Type3(_) => Rect::from_points(origin, origin),
        };
        if rect.x1 < 0.0 || rect.y1 < 0.0 || rect.x0 > self.w || rect.y0 > self.h {
            return;
        }
        self.letters.push((text, rect, origin));
    }
    fn draw_image(&mut self, _: Image<'a, '_>, _: Affine) {}
    fn pop_clip_path(&mut self) {}
    fn pop_transparency_group(&mut self) {}
}

/// Il testo con le posizioni di tutte le pagine del PDF.
pub fn pdf_text(bytes: Vec<u8>) -> Result<Vec<PageText>> {
    let pdf = Pdf::new(bytes).ok().context("PDF non leggibile")?;
    let settings = InterpreterSettings::default();
    let cache = InterpreterCache::new();
    let mut out = Vec::new();
    for page in pdf.pages().iter() {
        let (w, h) = page.render_dimensions();
        let (w, h) = (w.max(1.0) as f64, h.max(1.0) as f64);
        let initial = page.initial_transform(true).to_kurbo();
        let mut ctx = Context::new(initial, Rect::new(0.0, 0.0, w, h), &cache, page.xref(), settings.clone());
        let mut dev = Collect { w, h, letters: Vec::new() };
        interpret_page(page, &mut ctx, &mut dev);
        out.push(lines_of(dev.letters, w, h));
    }
    Ok(out)
}

/// Le lettere (in ordine di disegno) raccolte in righe.
fn lines_of(letters: Vec<(String, Rect, Point)>, w: f64, h: f64) -> PageText {
    struct Building {
        line: Line,
        base: f64,
        size: f64,
        last_x: f64,
    }
    let mut lines: Vec<Building> = Vec::new();
    for (text, r, o) in letters {
        let ink = if r.height() > 0.0 { r.height() } else { 0.0 };
        let letter = Letter { text, rect: [(r.x0 / w) as f32, (r.y0 / h) as f32, (r.x1 / w) as f32, (r.y1 / h) as f32] };
        let fits = |b: &Building| {
            let size = b.size.max(ink).max(1.0);
            (o.y - b.base).abs() < 0.5 * size && o.x > b.last_x - 2.0 * size && o.x < b.last_x + 8.0 * size
        };
        // di solito la lettera continua l'ultima riga; a volte una riga già iniziata
        let at = lines.iter().rposition(fits);
        match at {
            Some(i) => {
                let b = &mut lines[i];
                b.size = b.size.max(ink);
                b.last_x = b.last_x.max(r.x1).max(o.x);
                b.line.letters.push(letter);
            }
            None => lines.push(Building { line: Line { letters: vec![letter], rect: [0.0; 4] }, base: o.y, size: ink, last_x: r.x1.max(o.x) }),
        }
    }
    let mut out: Vec<Line> = lines
        .into_iter()
        .map(|mut b| {
            b.line.letters.sort_by(|a, c| a.rect[0].total_cmp(&c.rect[0]));
            let inked: Vec<&Letter> = b.line.letters.iter().filter(|l| l.rect[2] > l.rect[0] || l.rect[3] > l.rect[1]).collect();
            let src = if inked.is_empty() { b.line.letters.iter().collect() } else { inked };
            b.line.rect = src.iter().fold([f32::MAX, f32::MAX, f32::MIN, f32::MIN], |r, l| {
                [r[0].min(l.rect[0]), r[1].min(l.rect[1]), r[2].max(l.rect[2]), r[3].max(l.rect[3])]
            });
            b.line
        })
        .filter(|l| !l.letters.is_empty())
        .collect();
    out.sort_by(|a, b| a.rect[1].total_cmp(&b.rect[1]).then(a.rect[0].total_cmp(&b.rect[0])));
    PageText { lines: out }
}

/// Cosa va in un posto.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Firma,
    Timbro,
    /// «Timbro e firma»: il timbro e la firma sopra.
    Entrambi,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Firma => "firma",
            Kind::Timbro => "timbro",
            Kind::Entrambi => "timbro e firma",
        }
    }

    fn with(self, other: Kind) -> Kind {
        if self == other {
            self
        } else {
            Kind::Entrambi
        }
    }
}

/// Come ci si appoggia al posto.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Sopra una riga da riempire: `anchor` è il centro della riga.
    OnLine,
    /// Sotto la scritta: `anchor` è il centro del suo bordo in basso.
    Below,
}

/// Un posto per la firma o il timbro.
#[derive(Clone, Debug)]
pub struct Spot {
    /// Pagina (da 0).
    pub page: usize,
    pub kind: Kind,
    /// La scritta trovata (per dirlo all'utente).
    pub label: String,
    /// Frazioni della pagina del documento.
    pub anchor: [f32; 2],
    pub mode: Mode,
}

/// Le lettere della riga ridotte a sole lettere minuscole senza accenti,
/// ognuna con la sua posizione nella riga.
fn folded(line: &Line) -> (Vec<char>, Vec<usize>) {
    let mut s = Vec::new();
    let mut at = Vec::new();
    for (i, l) in line.letters.iter().enumerate() {
        for c in l.text.chars() {
            let c = match c.to_lowercase().next().unwrap_or(c) {
                'à' | 'á' | 'â' | 'ä' => 'a',
                'è' | 'é' | 'ê' | 'ë' => 'e',
                'ì' | 'í' | 'î' | 'ï' => 'i',
                'ò' | 'ó' | 'ô' | 'ö' => 'o',
                'ù' | 'ú' | 'û' | 'ü' => 'u',
                c => c,
            };
            if c.is_alphanumeric() {
                s.push(c);
                at.push(i);
            }
        }
    }
    (s, at)
}

fn fold_text(t: &str) -> Vec<char> {
    folded(&Line { letters: vec![Letter { text: t.to_string(), rect: [0.0; 4] }], rect: [0.0; 4] }).0
}

/// Dove compare `word` in `s` (in caratteri).
fn find(s: &[char], word: &[char]) -> Option<usize> {
    if word.is_empty() || word.len() > s.len() {
        return None;
    }
    (0..=s.len() - word.len()).find(|&i| s[i..i + word.len()] == *word)
}

/// Le scritte che chiedono una firma o un timbro (già ridotte come `folded`),
/// le più lunghe prima.
const LABELS: &[(&str, Kind)] = &[
    ("lettoconfermatoesottoscritto", Kind::Firma),
    ("illegalerappresentante", Kind::Firma),
    ("stampandsignature", Kind::Entrambi),
    ("peraccettazione", Kind::Firma),
    ("ildichiarante", Kind::Firma),
    ("ilrichiedente", Kind::Firma),
    ("timbroefirma", Kind::Entrambi),
    ("firmaetimbro", Kind::Entrambi),
    ("timbroefirme", Kind::Entrambi),
    ("signature", Kind::Firma),
    ("signedby", Kind::Firma),
    ("timbro", Kind::Timbro),
    ("stamp", Kind::Timbro),
    ("firma", Kind::Firma),
    ("firme", Kind::Firma),
];

/// Le parti di un contratto: chiedono una firma solo se accanto c'è una riga
/// da riempire (altrimenti sono parole del testo).
const ROLES: &[&str] = &[
    "ilcommittente",
    "ilconsulente",
    "ilcliente",
    "ilfornitore",
    "ilcontraente",
    "ilvenditore",
    "lacquirente",
    "illocatore",
    "ilconduttore",
    "ilcomodante",
    "ilcomodatario",
    "ildatoredilavoro",
    "illavoratore",
    "ildipendente",
    "ilmandante",
    "ilmandatario",
    "ilprestatore",
    "ilgenitore",
    "lostudente",
    "ilcandidato",
    "ildirettore",
    "ilresponsabile",
    "ilpresidente",
    "ilsegretario",
    "iltitolare",
    "ildelegato",
    "lappaltatore",
    "ilsubappaltatore",
    "ilproponente",
    "ilbeneficiario",
    "ildebitore",
    "ilcreditore",
    "ilgarante",
    "client",
    "supplier",
    "customer",
    "contractor",
    "buyer",
    "seller",
];

/// Una scritta più lunga di così è una frase del testo, non un'etichetta.
const MAX_LABEL: usize = 45;

/// Le etichette di una riga (lettere `start..=end`), da sinistra, senza
/// sovrapposizioni: `(start, end, kind, role)`.
fn labels_in(line: &Line) -> Vec<(usize, usize, Kind, bool)> {
    let (s, at) = folded(line);
    let mut taken = vec![false; s.len()];
    let mut out = Vec::new();
    let words = LABELS.iter().map(|(w, k)| (*w, *k, false)).chain(ROLES.iter().map(|w| (*w, Kind::Firma, true)));
    for (w, kind, role) in words {
        let w: Vec<char> = w.chars().collect();
        let mut from = 0;
        while let Some(rel) = find(&s[from..], &w) {
            let pos = from + rel;
            from = pos + 1;
            if taken[pos..pos + w.len()].iter().any(|t| *t) {
                continue;
            }
            // «firmato digitalmente», «firmata per accettazione», «firmatario», «firmare»:
            // parlano della firma, non chiedono di metterla lì
            let after: String = s[pos + w.len()..].iter().take(2).collect();
            if w.iter().collect::<String>() == "firma" && (after.starts_with('t') || after.starts_with("re")) {
                continue;
            }
            taken[pos..pos + w.len()].iter_mut().for_each(|t| *t = true);
            out.push((at[pos], at[pos + w.len() - 1], kind, role));
        }
    }
    out.sort_by_key(|l| l.0);
    out
}

/// Il testo delle lettere `a..b` della riga, senza le righe da riempire.
fn text_range(line: &Line, a: usize, b: usize) -> String {
    let part = Line { letters: line.letters[a..b.min(line.letters.len())].to_vec(), rect: line.rect };
    part.text().replace(['_', '…'], "").trim_end_matches(['.', ' ', ':']).trim().to_string()
}

/// Tutti i posti per firma e timbro trovati nel documento.
pub fn find_spots(pages: &[PageText]) -> Vec<Spot> {
    let mut out: Vec<Spot> = Vec::new();
    for (p, page) in pages.iter().enumerate() {
        for (i, line) in page.lines.iter().enumerate() {
            let labels = labels_in(line);
            let long = folded(line).0.len() > MAX_LABEL;
            for (k, &(start, end, kind, role)) in labels.iter().enumerate() {
                let next = labels.get(k + 1).map(|l| l.0).unwrap_or(line.letters.len());
                let blank_after = line.blank_runs(end + 1).into_iter().find(|r| next >= line.letters.len() || r[2] <= line.letters[next].rect[0]);
                if long && blank_after.is_none() {
                    continue;
                }
                let spot = spot_at(page, i, start, end, blank_after);
                if role && spot.mode != Mode::OnLine {
                    continue;
                }
                let label = if labels.len() == 1 { text_range(line, 0, line.letters.len()) } else { text_range(line, start, next) };
                merge(&mut out, Spot { page: p, kind, label, ..spot });
            }
        }
    }
    out
}

/// I posti da proporre: quelli stampati nel documento o, se non ce n'è
/// nessuno (un preventivo, una lettera), la fine del documento: dopo l'ultimo
/// testo, a destra, dove si firma una lettera.
pub fn places(pages: &[PageText]) -> Vec<Spot> {
    let spots = find_spots(pages);
    if !spots.is_empty() {
        return spots;
    }
    end_spot(pages).into_iter().collect()
}

/// Sotto l'ultima riga del testo dell'ultima pagina scritta (non il piè di
/// pagina: le righe nell'ultimo decimo del foglio), a destra.
fn end_spot(pages: &[PageText]) -> Option<Spot> {
    let (p, page) = pages.iter().enumerate().rev().find(|(_, pt)| !pt.lines.is_empty())?;
    let body: Vec<&Line> = page.lines.iter().filter(|l| l.rect[1] < 0.9).collect();
    let body = if body.is_empty() { page.lines.iter().collect() } else { body };
    let last = body.iter().max_by(|a, b| a.rect[3].total_cmp(&b.rect[3]))?;
    Some(Spot {
        page: p,
        kind: Kind::Entrambi,
        label: END_LABEL.into(),
        anchor: [0.72, (last.rect[3] + 2.0 * last.height()).min(0.85)],
        mode: Mode::Below,
    })
}

/// Come si chiama il posto in fondo (per il modello e per l'utente).
const END_LABEL: &str = "fine del documento, a destra (nel documento non c'è un posto di firma stampato)";

/// C'è posto per una firma sotto la riga `i` (riquadro `lr`)? No se altro
/// testo comincia subito sotto, nella stessa colonna (una tabella,
/// un'intestazione): la firma lo coprirebbe.
fn room_below(page: &PageText, i: usize, lr: [f32; 4]) -> bool {
    let cx = (lr[0] + lr[2]) / 2.0;
    let (x0, x1) = (cx - 0.12, cx + 0.12);
    !page.lines.iter().enumerate().any(|(j, l)| j != i && l.rect[2] > x0 && l.rect[0] < x1 && l.rect[1] >= lr[3] - 0.002 && l.rect[1] < lr[3] + 0.05)
}

/// Perché [`find_near`] non ha un posto.
#[derive(Debug, PartialEq, Eq)]
pub enum Near {
    /// La scritta non c'è.
    Missing,
    /// C'è, ma subito sotto c'è altro testo.
    NoRoom,
}

/// Il posto vicino alla scritta `text` (la sceglie l'AI leggendo il documento,
/// per esempio «Il Committente»), sulla pagina `page` o sulla prima che la ha.
pub fn find_near(pages: &[PageText], page: Option<usize>, text: &str, kind: Kind) -> Result<Spot, Near> {
    let want = fold_text(text);
    if want.is_empty() {
        return Err(Near::Missing);
    }
    let mut full = false;
    let order: Vec<usize> = match page {
        Some(p) => std::iter::once(p).chain((0..pages.len()).filter(|&q| q != p)).collect(),
        None => (0..pages.len()).collect(),
    };
    for p in order {
        let Some(pt) = pages.get(p) else { continue };
        for (i, line) in pt.lines.iter().enumerate() {
            let (s, at) = folded(line);
            if let Some(pos) = find(&s, &want) {
                let (start, end) = (at[pos], at[pos + want.len() - 1]);
                let spot = spot_at(pt, i, start, end, line.blank_run(end + 1));
                let lr = [line.letters[start].rect[0], line.rect[1], line.letters[end].rect[2], line.rect[3]];
                if spot.mode == Mode::Below && !room_below(pt, i, lr) {
                    full = true;
                    continue;
                }
                return Ok(Spot { page: p, kind, label: text_range(line, start, end + 1), ..spot });
            }
        }
    }
    Err(if full { Near::NoRoom } else { Near::Missing })
}

/// Dove va il timbro «data e luogo» per il posto `near` (dove vanno firma e
/// timbro): sulla scritta «Luogo e data» (o «Data», «Data e luogo»…) più
/// vicina della stessa pagina; se non c'è, alla stessa altezza dall'altra
/// parte del foglio, come «Barletta, 28/11/2026 ……… Firma».
pub fn date_spot(pages: &[PageText], near: &Spot) -> Spot {
    const PAIRS: &[&str] = &["luogoedata", "dataeluogo", "luogodata", "dataluogo", "localitaedata", "luogoedatadifirma"];
    let mut best: Option<(f32, Spot)> = None;
    if let Some(pt) = pages.get(near.page) {
        for (i, line) in pt.lines.iter().enumerate() {
            let (s, at) = folded(line);
            if s.is_empty() || s.len() > MAX_LABEL {
                continue;
            }
            let hit = PAIRS
                .iter()
                .find_map(|w| {
                    let w: Vec<char> = w.chars().collect();
                    find(&s, &w).map(|p| (p, w.len()))
                })
                // «Data» da sola: tutta la riga (non «Data di nascita»)
                .or_else(|| (s.iter().collect::<String>() == "data").then_some((0, 4)));
            let Some((p, n)) = hit else { continue };
            let (start, end) = (at[p], at[p + n - 1]);
            let spot = spot_at(pt, i, start, end, line.blank_run(end + 1));
            let d = (spot.anchor[0] - near.anchor[0]).hypot(spot.anchor[1] - near.anchor[1]);
            if d < 0.45 && best.as_ref().is_none_or(|b| d < b.0) {
                best = Some((d, Spot { page: near.page, kind: Kind::Timbro, label: text_range(line, start, end + 1), ..spot }));
            }
        }
    }
    best.map(|b| b.1).unwrap_or_else(|| Spot {
        page: near.page,
        kind: Kind::Timbro,
        label: "luogo e data".into(),
        anchor: [if near.anchor[0] >= 0.5 { 0.27 } else { 0.73 }, near.anchor[1]],
        mode: near.mode,
    })
}

/// Il posto per la scritta alle lettere `start..=end` della riga `i`.
fn spot_at(page: &PageText, i: usize, start: usize, end: usize, blank_after: Option<[f32; 4]>) -> Spot {
    let line = &page.lines[i];
    let base = Spot { page: 0, kind: Kind::Firma, label: String::new(), anchor: [0.0; 2], mode: Mode::Below };
    // «Firma ______» sulla stessa riga
    if let Some(r) = blank_after {
        return Spot { anchor: [(r[0] + r[2]) / 2.0, r[3]], mode: Mode::OnLine, ..base };
    }
    let lr = [line.letters[start].rect[0], line.rect[1], line.letters[end].rect[2], line.rect[3]];
    let (cx, lh) = ((lr[0] + lr[2]) / 2.0, line.height());
    // una riga da riempire appena sopra o poco sotto la scritta, nella stessa colonna
    let near = page
        .lines
        .iter()
        .enumerate()
        .filter(|(j, l)| *j != i && l.blank_share() > 0.6)
        .filter_map(|(_, l)| {
            let r = l.blank_near(cx)?;
            let wide = (r[2] - r[0]).max(lr[2] - lr[0]);
            let dx = if r[0] <= cx && cx <= r[2] { 0.0 } else { (r[0] - cx).abs().min((r[2] - cx).abs()) };
            let below = r[1] >= lr[3];
            let d = if below { r[1] - lr[3] } else { lr[1] - r[3] };
            (dx < wide && d >= -0.25 * lh && d < if below { 10.0 } else { 2.5 } * lh).then_some((d, r))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0));
    if let Some((_, r)) = near {
        return Spot { anchor: [(r[0] + r[2]) / 2.0, r[3]], mode: Mode::OnLine, ..base };
    }
    Spot { anchor: [cx, lr[3]], mode: Mode::Below, ..base }
}

/// Due scritte per lo stesso posto («Timbro» e «Firma» una sotto l'altra):
/// un posto solo, con tutte e due.
fn merge(out: &mut Vec<Spot>, s: Spot) {
    if let Some(o) = out.iter_mut().find(|o| o.page == s.page && (o.anchor[0] - s.anchor[0]).abs() < 0.08 && (o.anchor[1] - s.anchor[1]).abs() < 0.05) {
        o.kind = o.kind.with(s.kind);
        if !o.label.contains(&s.label) {
            o.label = format!("{} / {}", o.label, s.label);
        }
        return;
    }
    out.push(s);
}

/// «firma», «timbro», «timbro e firma» (come li scrive il modello).
pub fn parse_what(w: &str) -> Kind {
    let w = w.to_lowercase();
    match (w.contains("timbro") || w.contains("stamp"), w.contains("firma") || w.contains("sign")) {
        (true, true) => Kind::Entrambi,
        (true, false) => Kind::Timbro,
        _ => Kind::Firma,
    }
}

/// Il posto scelto dalla chat: il numero `place` dell'elenco di [`find_spots`]
/// (da 1) o la scritta `near_text` (sulla pagina `page`, da 0); `what`
/// cambia cosa ci va. L'errore è già in parole per il modello.
pub fn choose(pages: &[PageText], place: Option<usize>, near_text: Option<&str>, page: Option<usize>, what: Option<Kind>) -> Result<Spot, String> {
    let mut spot = if let Some(n) = place {
        let spots = places(pages);
        spots.get(n.wrapping_sub(1)).cloned().ok_or_else(|| format!("non c'è il posto {n}: i posti sono {} (document_find_signature_places)", spots.len()))?
    } else if let Some(t) = near_text {
        find_near(pages, page, t, what.unwrap_or(Kind::Firma)).map_err(|e| match e {
            Near::Missing => format!("«{t}» non c'è nel testo del documento: copia la scritta com'è stampata, o chiedi all'utente dove"),
            Near::NoRoom => format!(
                "sotto «{t}» c'è altro testo (una tabella, un'intestazione): la firma lo coprirebbe. Non è un posto per firmare. Usa i posti di document_find_signature_places (se il documento non ne ha, c'è la fine del documento), o chiedi all'utente dove"
            ),
        })?
    } else {
        return Err("indica `place` (da document_find_signature_places) o `near_text`".into());
    };
    if let Some(w) = what {
        spot.kind = w;
    }
    Ok(spot)
}

/// Che cosa ha chiesto la chat: il posto, se ci vanno firma e/o timbro
/// (`sign`) e se anche (o solo) «data e luogo», e dove.
#[derive(Clone, Debug)]
pub struct Request {
    pub spot: Spot,
    pub sign: bool,
    pub date: Option<Spot>,
}

/// Come [`choose`], più «data e luogo»: `what` «data e luogo» li chiede da
/// soli, `date_place` insieme a firma e timbro. L'errore è in parole per il modello.
pub fn request(pages: &[PageText], place: Option<usize>, near_text: Option<&str>, page: Option<usize>, what: Option<&str>, date_place: bool) -> Result<Request, String> {
    let only_date = what.is_some_and(|w| {
        let w = w.to_lowercase();
        (w.contains("data") || w.contains("luogo") || w.contains("date")) && !["firma", "timbro", "sign", "stamp"].iter().any(|x| w.contains(x))
    });
    let spot = choose(pages, place, near_text, page, if only_date { None } else { what.map(parse_what) })?;
    let date = if only_date && near_text.is_some() {
        // la scritta scelta è già quella della data («Luogo e data»)
        Some(Spot { kind: Kind::Timbro, ..spot.clone() })
    } else if only_date || date_place {
        Some(date_spot(pages, &spot))
    } else {
        None
    };
    Ok(Request { spot, sign: !only_date, date })
}

/// Il testo delle pagine, per il modello che non vede il documento.
pub fn read_text(pages: &[PageText], max_chars: usize) -> String {
    let mut s = String::new();
    for (p, page) in pages.iter().enumerate() {
        s.push_str(&format!("--- Pagina {} ---\n", p + 1));
        for l in &page.lines {
            s.push_str(&l.text());
            s.push('\n');
        }
        if s.len() > max_chars {
            let mut cut = max_chars;
            while !s.is_char_boundary(cut) {
                cut -= 1;
            }
            s.truncate(cut);
            s.push_str("\n[…]");
            break;
        }
    }
    s
}

/// I posti in parole, per il modello e per l'utente.
pub fn describe(spots: &[Spot]) -> String {
    if spots.is_empty() {
        return "Nel documento non c'è testo da leggere (forse è una scansione): chiedi all'utente dove firmare.".into();
    }
    let mut s = String::new();
    for (i, sp) in spots.iter().enumerate() {
        let how = match sp.mode {
            Mode::OnLine => "sopra la riga da riempire",
            Mode::Below => "sotto la scritta",
        };
        if sp.label == END_LABEL {
            s.push_str(&format!("{}. pagina {}: {} → {} (sotto l'ultimo testo)\n", i + 1, sp.page + 1, sp.label, sp.kind.name()));
            continue;
        }
        s.push_str(&format!("{}. pagina {}: «{}» → {} ({how})\n", i + 1, sp.page + 1, sp.label, sp.kind.name()));
    }
    s
}

/// Dove va sul foglio della lavagna (u, v, larghezza, altezza in 0..1) una
/// firma o un timbro grande `natural` (il suo rettangolo naturale: conta solo
/// la misura) per il posto `spot`, con la pagina del documento in
/// `page_rect` sul foglio. Mai più largo di `max_w`.
pub fn place(spot: &Spot, page_rect: [f32; 4], natural: [f32; 4], max_w: f32) -> [f32; 4] {
    let [rx, ry, rw, rh] = page_rect;
    let (mut w, mut h) = (natural[2], natural[3]);
    if w > max_w && w > 0.0 {
        let k = max_w / w;
        w *= k;
        h *= k;
    }
    let (ax, ay) = (rx + spot.anchor[0] * rw, ry + spot.anchor[1] * rh);
    let (u, v) = match spot.mode {
        // la firma si appoggia alla riga e scende appena sotto
        Mode::OnLine => (ax - w / 2.0, ay - h * 0.8),
        Mode::Below => (ax - w / 2.0, ay + 0.004),
    };
    inside([u, v, w, h], page_rect)
}

/// Il rettangolo spostato quanto basta per stare dentro la pagina.
fn inside(r: [f32; 4], page_rect: [f32; 4]) -> [f32; 4] {
    let [rx, ry, rw, rh] = page_rect;
    let [u, v, w, h] = r;
    [u.clamp(rx, (rx + rw - w).max(rx)), v.clamp(ry, (ry + rh - h).max(ry)), w, h]
}

/// La firma sopra il timbro, come si fa a mano: un po' spostata a destra,
/// sempre dentro la pagina.
pub fn over_stamp(stamp: [f32; 4], sig: [f32; 4], page_rect: [f32; 4]) -> [f32; 4] {
    let (cx, cy) = (stamp[0] + stamp[2] * 0.6, stamp[1] + stamp[3] * 0.5);
    inside([cx - sig[2] / 2.0, cy - sig[3] / 2.0, sig[2], sig[3]], page_rect)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un PDF di prova con le righe date (Helvetica 11, y in punti dall'alto).
    fn pdf(lines: &[(f32, f32, &str)]) -> Vec<u8> {
        use lopdf::content::{Content, Operation};
        use lopdf::{dictionary, Document, Object, Stream};
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let font_id = doc.add_object(dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding" });
        let resources_id = doc.add_object(dictionary! { "Font" => dictionary! { "F1" => font_id } });
        let mut ops = Vec::new();
        for (x, y, t) in lines {
            ops.push(Operation::new("BT", vec![]));
            ops.push(Operation::new("Tf", vec!["F1".into(), 11.into()]));
            ops.push(Operation::new("Td", vec![(*x).into(), (842.0 - *y).into()]));
            ops.push(Operation::new("Tj", vec![Object::string_literal(*t)]));
            ops.push(Operation::new("ET", vec![]));
        }
        let content = Content { operations: ops };
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
        let page_id = doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages_id, "Contents" => content_id });
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page_id.into()], "Count" => 1, "Resources" => resources_id, "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()] }),
        );
        let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        doc.trailer.set("Root", catalog_id);
        let mut out = Vec::new();
        doc.save_to(&mut out).unwrap();
        out
    }

    #[test]
    fn legge_le_righe() {
        let pages = pdf_text(pdf(&[(72.0, 100.0, "Preventivo n. 32"), (72.0, 130.0, "Totale 9.430,00")])).unwrap();
        assert_eq!(pages.len(), 1);
        let texts: Vec<String> = pages[0].lines.iter().map(|l| l.text()).collect();
        assert_eq!(texts, vec!["Preventivo n. 32", "Totale 9.430,00"]);
        let r = pages[0].lines[0].rect;
        assert!((r[0] - 72.0 / 595.0).abs() < 0.01 && r[1] > 0.1 && r[3] < 100.0 / 842.0 + 0.01, "{r:?}");
    }

    #[test]
    fn firma_sulla_riga() {
        let pages = pdf_text(pdf(&[(72.0, 100.0, "Il presente contratto si intende accettato in ogni sua parte dalle parti."), (72.0, 700.0, "Luogo e data ____________   Firma ________________")])).unwrap();
        let spots = find_spots(&pages);
        assert_eq!(spots.len(), 1, "{spots:?}");
        let s = &spots[0];
        assert_eq!((s.kind, s.mode), (Kind::Firma, Mode::OnLine));
        // il centro della riga dopo «Firma», non quella della data
        assert!(s.anchor[0] > 0.45 && (s.anchor[1] - 700.0 / 842.0).abs() < 0.02, "{s:?}");
    }

    #[test]
    fn timbro_e_firma_sotto() {
        let pages = pdf_text(pdf(&[(380.0, 650.0, "Timbro e firma")])).unwrap();
        let spots = find_spots(&pages);
        assert_eq!(spots.len(), 1);
        assert_eq!((spots[0].kind, spots[0].mode), (Kind::Entrambi, Mode::Below));
        let r = place(&spots[0], [0.1, 0.0, 0.8, 1.0], [0.0, 0.0, 0.2, 0.1], 0.5);
        // sotto la scritta, centrata su di essa
        assert!(r[1] > 650.0 / 842.0 && (r[0] + r[2] / 2.0 - (0.1 + spots[0].anchor[0] * 0.8)).abs() < 1e-4, "{r:?}");
    }

    #[test]
    fn riga_sopra_la_scritta() {
        let pages = pdf_text(pdf(&[(350.0, 700.0, "______________________"), (380.0, 714.0, "Il legale rappresentante")])).unwrap();
        let spots = find_spots(&pages);
        assert_eq!(spots.len(), 1, "{spots:?}");
        assert_eq!(spots[0].mode, Mode::OnLine);
        assert!(spots[0].anchor[1] < 714.0 / 842.0, "{:?}", spots[0]);
    }

    #[test]
    fn frasi_e_firmato_no() {
        let pages = pdf_text(pdf(&[
            (72.0, 100.0, "La presente offerta, una volta firmata per accettazione dal cliente, vale come contratto."),
            (72.0, 130.0, "Documento firmato digitalmente"),
        ]))
        .unwrap();
        assert!(find_spots(&pages).is_empty(), "{:?}", find_spots(&pages));
    }

    #[test]
    fn due_parti_con_le_righe_sotto() {
        let pages = pdf_text(pdf(&[
            (72.0, 100.0, "Il Committente si impegna a pagare il Consulente entro trenta giorni."),
            (110.0, 640.0, "IL COMMITTENTE                 IL CONSULENTE"),
            (72.0, 700.0, "_________________________      __________________________"),
        ]))
        .unwrap();
        let spots = find_spots(&pages);
        assert_eq!(spots.len(), 2, "{spots:?}");
        assert!(spots.iter().all(|s| s.mode == Mode::OnLine && (s.anchor[1] - 700.0 / 842.0).abs() < 0.02), "{spots:?}");
        assert!(spots[0].anchor[0] < 0.35 && spots[1].anchor[0] > 0.45, "{spots:?}");
        assert_eq!((spots[0].label.as_str(), spots[1].label.as_str()), ("IL COMMITTENTE", "IL CONSULENTE"));
    }

    /// Prova su un PDF vero: `SIGNSPOT_PDF=file.pdf cargo test signspot -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn su_un_file() {
        let path = std::env::var("SIGNSPOT_PDF").expect("SIGNSPOT_PDF");
        let pages = pdf_text(std::fs::read(path).unwrap()).unwrap();
        for (p, page) in pages.iter().enumerate() {
            println!("--- pagina {}: {} righe", p + 1, page.lines.len());
        }
        print!("{}", describe(&places(&pages)));
        if std::env::var("SIGNSPOT_TEXT").is_ok() {
            print!("{}", read_text(&pages, 20000));
        }
    }

    /// Un preventivo senza posto di firma (come quello vero): non sotto una
    /// scritta della tabella, ma in fondo, a destra.
    #[test]
    fn preventivo_senza_posto() {
        let pages = pdf_text(pdf(&[
            (72.0, 100.0, "Preventivo n.32 del 08/09/2026"),
            (72.0, 140.0, "Committente      Tito Tiani & Partners"),
            (72.0, 160.0, "Fornitore        Paolillo Ruggiero Marco"),
            (72.0, 180.0, "Oggetto          Piattaforma AI"),
            (72.0, 400.0, "Condizioni: preventivo valido 60 giorni."),
            (72.0, 800.0, "Paolillo Ruggiero Marco - Via III Novembre 15, Barletta"),
        ]))
        .unwrap();
        let spots = places(&pages);
        assert_eq!(spots.len(), 1, "{spots:?}");
        let s = &spots[0];
        assert!(s.anchor[0] > 0.6 && s.anchor[1] > 400.0 / 842.0 && s.anchor[1] < 0.6 && s.kind == Kind::Entrambi, "{s:?}");
        assert!(describe(&spots).contains("fine del documento"));
        // «Fornitore» ha la riga «Oggetto» subito sotto: non è un posto
        assert_eq!(find_near(&pages, None, "Fornitore", Kind::Firma).unwrap_err(), Near::NoRoom);
        assert!(choose(&pages, None, Some("Fornitore"), None, None).unwrap_err().contains("altro testo"));
        assert!(choose(&pages, Some(1), None, None, None).is_ok());
    }

    #[test]
    fn data_e_luogo_sulla_sua_riga_o_accanto() {
        let pages = pdf_text(pdf(&[
            (72.0, 600.0, "Luogo e data ____________________"),
            (350.0, 600.0, "Firma ____________________"),
        ]))
        .unwrap();
        let sig = find_spots(&pages).into_iter().find(|s| s.label.contains("Firma")).unwrap();
        let d = date_spot(&pages, &sig);
        assert!(d.label.to_lowercase().contains("luogo e data") && d.anchor[0] < 0.45 && d.mode == Mode::OnLine, "{d:?}");
        // un preventivo senza «Luogo e data»: dall'altra parte, alla stessa altezza
        let pages = pdf_text(pdf(&[(72.0, 100.0, "Preventivo n.32"), (72.0, 400.0, "Condizioni: 60 giorni.")])).unwrap();
        let end = places(&pages).remove(0);
        let d = date_spot(&pages, &end);
        assert!(d.anchor[0] < 0.5 && (d.anchor[1] - end.anchor[1]).abs() < 1e-6, "{d:?} {end:?}");
    }

    #[test]
    fn vicino_a_un_testo_scelto() {
        let pages = pdf_text(pdf(&[(72.0, 600.0, "Il Committente"), (350.0, 600.0, "Il Fornitore")])).unwrap();
        let s = find_near(&pages, None, "il fornitore", Kind::Firma).unwrap();
        assert!(s.anchor[0] > 0.6 && s.mode == Mode::Below, "{s:?}");
        assert_eq!(find_near(&pages, None, "notaio", Kind::Firma).unwrap_err(), Near::Missing);
    }
}
