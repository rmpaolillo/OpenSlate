//! Testo «a mano»: lettere fatte di tratti, come quelli del pennino.
//!
//! La chat scrive sulla lavagna con i caratteri di Hershey (a filo singolo:
//! ogni lettera è una o più linee, non un contorno), quindi il testo diventa
//! tratti veri: si salva, si esporta, si cancella e si annulla come quello che
//! scrivi tu. Due stili: stampatello (`futural`) e corsivo (`cursive`).
//! Le lettere accentate si compongono (la lettera più l'accento); un po' di
//! irregolarità (righe, dimensioni) toglie l'aria da stampante.
//!
//! I caratteri di Hershey: vedi `src/handwrite/LICENSE-hershey.txt`.

use std::collections::HashMap;
use std::sync::OnceLock;

const FUTURAL: &str = include_str!("handwrite/futural.jhf");
const CURSIVE: &str = include_str!("handwrite/cursive.jhf");

/// Altezza delle maiuscole nelle unità di Hershey (da -12 a 9).
const CAP: f32 = 21.0;
const CAP_TOP: f32 = -12.0;
/// Da una riga all'altra (unità di Hershey).
const LINE: f32 = 34.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Stampatello,
    Corsivo,
}

impl Style {
    pub fn parse(s: &str) -> Style {
        match s.trim().to_lowercase().as_str() {
            "corsivo" | "cursive" | "script" | "corsiva" => Style::Corsivo,
            _ => Style::Stampatello,
        }
    }
}

#[derive(Clone)]
struct Glyph {
    left: f32,
    right: f32,
    strokes: Vec<Vec<(f32, f32)>>,
}

struct Font {
    glyphs: HashMap<char, Glyph>,
    /// Dove comincia il corpo delle minuscole (y di Hershey, verso il basso).
    x_top: f32,
    /// Le lettere del corsivo pendono: gli accenti vanno un po' a destra.
    slant: f32,
}

fn parse(src: &str) -> HashMap<char, Glyph> {
    // un glifo può andare a capo nel file: le righe che non cominciano con il
    // numero continuano la precedente
    let mut rows: Vec<String> = Vec::new();
    for l in src.lines() {
        let starts = l.len() >= 8 && l[..5].trim().chars().all(|c| c.is_ascii_digit()) && !l[..5].trim().is_empty();
        match rows.last_mut() {
            Some(r) if !starts => r.push_str(l),
            _ => rows.push(l.to_string()),
        }
    }
    let mut out = HashMap::new();
    for (i, r) in rows.iter().enumerate() {
        let b = r.as_bytes();
        if b.len() < 10 {
            continue;
        }
        let Ok(n) = r[5..8].trim().parse::<usize>() else { continue };
        let d = &b[8..];
        let v = |c: u8| c as f32 - b'R' as f32;
        let mut strokes: Vec<Vec<(f32, f32)>> = vec![Vec::new()];
        for k in 1..n {
            let (Some(&x), Some(&y)) = (d.get(2 * k), d.get(2 * k + 1)) else { break };
            if x == b' ' && y == b'R' {
                strokes.push(Vec::new());
            } else {
                strokes.last_mut().unwrap().push((v(x), v(y)));
            }
        }
        strokes.retain(|s| !s.is_empty());
        if let Some(c) = char::from_u32(32 + i as u32) {
            out.insert(c, Glyph { left: v(d[0]), right: v(d[1]), strokes });
        }
    }
    out
}

fn font(style: Style) -> &'static Font {
    static F: OnceLock<Font> = OnceLock::new();
    static C: OnceLock<Font> = OnceLock::new();
    match style {
        Style::Stampatello => F.get_or_init(|| Font { glyphs: parse(FUTURAL), x_top: -5.0, slant: 0.0 }),
        Style::Corsivo => C.get_or_init(|| Font { glyphs: parse(CURSIVE), x_top: 0.0, slant: 2.0 }),
    }
}

#[derive(Clone, Copy)]
enum Accent {
    Grave,
    Acute,
    Circ,
    Uml,
    Tilde,
    Cedilla,
    Ring,
}

/// Una lettera accentata: la lettera di base e l'accento.
fn decompose(c: char) -> Option<(char, Accent)> {
    use Accent::*;
    let (base, a) = match c {
        'à' => ('a', Grave), 'á' => ('a', Acute), 'â' => ('a', Circ), 'ä' => ('a', Uml), 'ã' => ('a', Tilde), 'å' => ('a', Ring),
        'è' => ('e', Grave), 'é' => ('e', Acute), 'ê' => ('e', Circ), 'ë' => ('e', Uml),
        'ì' => ('i', Grave), 'í' => ('i', Acute), 'î' => ('i', Circ), 'ï' => ('i', Uml),
        'ò' => ('o', Grave), 'ó' => ('o', Acute), 'ô' => ('o', Circ), 'ö' => ('o', Uml), 'õ' => ('o', Tilde),
        'ù' => ('u', Grave), 'ú' => ('u', Acute), 'û' => ('u', Circ), 'ü' => ('u', Uml),
        'ñ' => ('n', Tilde), 'ç' => ('c', Cedilla), 'ÿ' => ('y', Uml),
        'À' => ('A', Grave), 'Á' => ('A', Acute), 'Â' => ('A', Circ), 'Ä' => ('A', Uml), 'Ã' => ('A', Tilde), 'Å' => ('A', Ring),
        'È' => ('E', Grave), 'É' => ('E', Acute), 'Ê' => ('E', Circ), 'Ë' => ('E', Uml),
        'Ì' => ('I', Grave), 'Í' => ('I', Acute), 'Î' => ('I', Circ), 'Ï' => ('I', Uml),
        'Ò' => ('O', Grave), 'Ó' => ('O', Acute), 'Ô' => ('O', Circ), 'Ö' => ('O', Uml), 'Õ' => ('O', Tilde),
        'Ù' => ('U', Grave), 'Ú' => ('U', Acute), 'Û' => ('U', Circ), 'Ü' => ('U', Uml),
        'Ñ' => ('N', Tilde), 'Ç' => ('C', Cedilla),
        _ => return None,
    };
    Some((base, a))
}

/// Caratteri che si scrivono come altri (virgolette tipografiche, trattini…).
fn plain(c: char) -> Option<&'static str> {
    Some(match c {
        '’' | '‘' | '′' | 'ʼ' => "'",
        '“' | '”' | '«' | '»' | '„' | '″' => "\"",
        '–' | '—' | '−' | '‐' => "-",
        '…' => "...",
        'ß' => "ss",
        'æ' => "ae",
        'Æ' => "AE",
        'œ' => "oe",
        'Œ' => "OE",
        '\u{a0}' | '\u{2009}' | '\u{202f}' => " ",
        '\t' => "  ",
        _ => return None,
    })
}

fn dot(cx: f32, cy: f32) -> Vec<(f32, f32)> {
    vec![(cx - 0.8, cy), (cx, cy + 0.8), (cx + 0.8, cy), (cx, cy - 0.8), (cx - 0.8, cy)]
}

fn accent(a: Accent, cx: f32, top: f32, upper: bool) -> Vec<Vec<(f32, f32)>> {
    // l'accento sta in una fascia alta 4 unità sopra la lettera
    let (yt, yb) = if upper { (top - 6.0, top - 2.5) } else { (top - 7.0, top - 2.5) };
    match a {
        Accent::Grave => vec![vec![(cx - 2.0, yt), (cx + 1.5, yb)]],
        Accent::Acute => vec![vec![(cx + 2.0, yt), (cx - 1.5, yb)]],
        Accent::Circ => vec![vec![(cx - 3.0, yb), (cx, yt), (cx + 3.0, yb)]],
        Accent::Uml => vec![dot(cx - 2.5, (yt + yb) / 2.0), dot(cx + 2.5, (yt + yb) / 2.0)],
        Accent::Tilde => vec![vec![(cx - 4.0, yb), (cx - 2.5, yt), (cx - 0.5, yt + 0.5), (cx + 1.0, yb - 0.5), (cx + 2.5, yb), (cx + 4.0, yt)]],
        Accent::Ring => {
            let (r, cy) = (1.8, (yt + yb) / 2.0);
            vec![(0..=12).map(|k| (k as f32 / 12.0 * std::f32::consts::TAU).sin_cos()).map(|(s, c)| (cx + r * c, cy + r * s)).collect()]
        }
        Accent::Cedilla => vec![vec![(cx, 9.0), (cx + 1.0, 11.0), (cx + 2.0, 12.0), (cx + 1.0, 14.0), (cx - 1.5, 14.5)]],
    }
}

/// Segni che i caratteri di Hershey non hanno, fatti a mano.
fn symbol(c: char) -> Option<Glyph> {
    let g = |left: f32, right: f32, strokes: Vec<Vec<(f32, f32)>>| Some(Glyph { left, right, strokes });
    match c {
        '×' => g(-7.0, 7.0, vec![vec![(-4.0, -5.0), (4.0, 3.0)], vec![(4.0, -5.0), (-4.0, 3.0)]]),
        '÷' => g(-8.0, 8.0, vec![vec![(-5.0, -1.0), (5.0, -1.0)], dot(0.0, -5.5), dot(0.0, 3.5)]),
        '±' => g(-8.0, 8.0, vec![vec![(0.0, -8.0), (0.0, 2.0)], vec![(-5.0, -3.0), (5.0, -3.0)], vec![(-5.0, 6.0), (5.0, 6.0)]]),
        '≠' => g(-8.0, 8.0, vec![vec![(-5.0, -3.5), (5.0, -3.5)], vec![(-5.0, 1.5), (5.0, 1.5)], vec![(3.0, -8.0), (-3.0, 6.0)]]),
        '≈' => g(-8.0, 8.0, vec![
            vec![(-5.0, -3.0), (-3.0, -4.5), (0.0, -3.0), (3.0, -1.5), (5.0, -3.0)],
            vec![(-5.0, 2.0), (-3.0, 0.5), (0.0, 2.0), (3.0, 3.5), (5.0, 2.0)],
        ]),
        '≤' => g(-8.0, 8.0, vec![vec![(5.0, -9.0), (-5.0, -4.0), (5.0, 1.0)], vec![(-5.0, 5.0), (5.0, 5.0)]]),
        '≥' => g(-8.0, 8.0, vec![vec![(-5.0, -9.0), (5.0, -4.0), (-5.0, 1.0)], vec![(-5.0, 5.0), (5.0, 5.0)]]),
        '→' => g(-9.0, 9.0, vec![vec![(-7.0, -1.0), (7.0, -1.0)], vec![(3.0, -4.5), (7.0, -1.0), (3.0, 2.5)]]),
        '←' => g(-9.0, 9.0, vec![vec![(7.0, -1.0), (-7.0, -1.0)], vec![(-3.0, -4.5), (-7.0, -1.0), (-3.0, 2.5)]]),
        '↑' => g(-6.0, 6.0, vec![vec![(0.0, 9.0), (0.0, -11.0)], vec![(-3.5, -7.0), (0.0, -11.0), (3.5, -7.0)]]),
        '↓' => g(-6.0, 6.0, vec![vec![(0.0, -11.0), (0.0, 9.0)], vec![(-3.5, 5.0), (0.0, 9.0), (3.5, 5.0)]]),
        '⇒' => g(-9.0, 9.0, vec![vec![(-7.0, -3.0), (4.0, -3.0)], vec![(-7.0, 1.0), (4.0, 1.0)], vec![(2.0, -6.0), (7.0, -1.0), (2.0, 4.0)]]),
        '°' => g(-4.0, 4.0, vec![(0..=12).map(|k| (k as f32 / 12.0 * std::f32::consts::TAU).sin_cos()).map(|(s, c)| (2.0 * c, -10.0 + 2.0 * s)).collect()]),
        '·' | '•' => g(-3.0, 3.0, vec![dot(0.0, -1.0)]),
        '€' => g(-10.0, 10.0, vec![
            vec![(6.0, -10.0), (4.0, -11.5), (1.0, -12.0), (-2.0, -11.0), (-4.0, -8.0), (-5.0, -4.0), (-5.0, 1.0), (-4.0, 5.0), (-2.0, 8.0), (1.0, 9.0), (4.0, 8.5), (6.0, 7.0)],
            vec![(-8.0, -4.5), (3.0, -4.5)],
            vec![(-8.0, 0.5), (2.0, 0.5)],
        ]),
        '✓' => g(-8.0, 8.0, vec![vec![(-6.0, -1.0), (-2.0, 5.0), (7.0, -10.0)]]),
        '✗' => g(-7.0, 7.0, vec![vec![(-5.0, -8.0), (5.0, 5.0)], vec![(5.0, -8.0), (-5.0, 5.0)]]),
        _ => None,
    }
}

/// Esponenti e pedici: la cifra piccola, in alto o in basso.
fn script(c: char) -> Option<(char, bool)> {
    const SUP: &str = "⁰¹²³⁴⁵⁶⁷⁸⁹⁺⁻⁼⁽⁾ⁿⁱ";
    const SUB: &str = "₀₁₂₃₄₅₆₇₈₉₊₋₌₍₎";
    const BASE: &str = "0123456789+-=()ni";
    if let Some(i) = SUP.chars().position(|x| x == c) {
        return BASE.chars().nth(i).map(|b| (b, true));
    }
    SUB.chars().position(|x| x == c).and_then(|i| BASE.chars().nth(i)).map(|b| (b, false))
}

/// Il glifo di un carattere (con l'accento, se c'è).
fn glyph(f: &Font, c: char) -> Option<Glyph> {
    if let Some(g) = f.glyphs.get(&c).filter(|_| c.is_ascii()) {
        return Some(g.clone());
    }
    if let Some(g) = symbol(c) {
        return Some(g);
    }
    if let Some((base, up)) = script(c) {
        let g = f.glyphs.get(&base)?;
        let k = 0.6;
        let y = |y: f32| if up { CAP_TOP - 1.0 + (y - CAP_TOP) * k } else { 9.0 + 3.0 - (9.0 - y) * k };
        return Some(Glyph {
            left: g.left * k,
            right: g.right * k + 1.0,
            strokes: g.strokes.iter().map(|s| s.iter().map(|&(x, yy)| (x * k, y(yy))).collect()).collect(),
        });
    }
    let (base, a) = decompose(c)?;
    let mut g = f.glyphs.get(&base)?.clone();
    let upper = base.is_uppercase();
    let top = if upper { CAP_TOP } else { f.x_top };
    if base == 'i' {
        // la i accentata perde il puntino
        g.strokes.retain(|s| s.iter().any(|p| p.1 > top + 0.5));
    }
    let xs = g.strokes.iter().flatten().map(|p| p.0);
    let (x0, x1) = xs.fold((f32::MAX, f32::MIN), |(a, b), x| (a.min(x), b.max(x)));
    let cx = if x0 <= x1 { (x0 + x1) / 2.0 } else { 0.0 } + if upper { 0.0 } else { f.slant };
    g.strokes.extend(accent(a, cx, top, upper));
    Some(g)
}

/// Testo disposto sulla pagina.
pub struct Laid {
    /// I tratti, in unità della pagina (u, v: 0..1 di larghezza e altezza).
    pub strokes: Vec<Vec<(f32, f32)>>,
    /// Il riquadro occupato (u0, v0, u1, v1).
    pub bbox: [f32; 4],
    /// Caratteri che non so scrivere (saltati).
    pub missing: Vec<char>,
}

/// Generatore pseudo-casuale piccolo e ripetibile (lo stesso testo viene
/// sempre uguale).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        ((self.0 >> 11) as f32 / (1u64 << 53) as f32) * 2.0 - 1.0
    }
}

/// Larghezza di una parola in unità di Hershey.
fn word_width(f: &Font, w: &str) -> f32 {
    w.chars()
        .map(|c| match plain(c) {
            Some(p) => p.chars().filter_map(|c| glyph(f, c)).map(|g| g.right - g.left).sum(),
            None => glyph(f, c).map(|g| g.right - g.left).unwrap_or(0.0),
        })
        .sum()
}

/// Dispone `text` con le maiuscole alte `cap` (frazione dell'altezza della
/// pagina), a partire da `origin` (u, v: in alto a sinistra); `aspect` =
/// larghezza / altezza della pagina; va a capo per stare in `max_w` (frazione
/// della larghezza). `align` 0 = a sinistra, 1 = a destra (da `origin`).
pub fn layout(text: &str, style: Style, cap: f32, aspect: f32, origin: (f32, f32), max_w: f32, right_align: bool) -> Laid {
    let f = font(style);
    let s = cap / CAP; // unità della pagina (in altezza) per unità di Hershey
    let kx = s / aspect; // la stessa misura in larghezza
    let space = word_width(f, " ").max(8.0);
    let max_units = (max_w / kx).max(40.0);
    // righe: a capo dove lo dice il testo, poi per la larghezza
    let mut lines: Vec<String> = Vec::new();
    for para in text.split('\n') {
        let mut cur = String::new();
        let mut cur_w = 0.0;
        for w in para.split(' ').filter(|w| !w.is_empty()) {
            let ww = word_width(f, w);
            if !cur.is_empty() && cur_w + space + ww > max_units {
                lines.push(std::mem::take(&mut cur));
                cur_w = 0.0;
            }
            if !cur.is_empty() {
                cur.push(' ');
                cur_w += space;
            }
            cur.push_str(w);
            cur_w += ww;
        }
        lines.push(cur);
    }
    let seed = text.bytes().fold(0x9E37_79B9_7F4A_7C15u64, |h, b| (h ^ b as u64).wrapping_mul(0x100_0000_01B3)) | 1;
    let mut rng = Rng(seed);
    let mut out: Vec<Vec<(f32, f32)>> = Vec::new();
    let mut missing = Vec::new();
    let mut bb = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
    for (li, line) in lines.iter().enumerate() {
        let lw = word_width(f, line);
        let mut x = 0.0f32;
        // la riga sale o scende appena, come a mano
        let tilt = rng.next() * 0.012;
        let line_dy = li as f32 * LINE + rng.next() * 0.6;
        let start = if right_align { -lw } else { 0.0 };
        let chars: Vec<char> = line.chars().flat_map(|c| plain(c).map(|p| p.chars().collect::<Vec<_>>()).unwrap_or_else(|| vec![c])).collect();
        for c in chars {
            let Some(g) = glyph(f, c) else {
                if !c.is_whitespace() && !missing.contains(&c) {
                    missing.push(c);
                }
                continue;
            };
            let gs = 1.0 + rng.next() * 0.035;
            let gdy = rng.next() * 0.45;
            for st in &g.strokes {
                let pts: Vec<(f32, f32)> = st
                    .iter()
                    .map(|&(gx, gy)| {
                        let hx = start + x + (gx - g.left) * gs;
                        let hy = line_dy + gdy + (gy - CAP_TOP) * gs + hx * tilt;
                        (origin.0 + hx * kx, origin.1 + hy * s)
                    })
                    .collect();
                for &(u, v) in &pts {
                    bb = [bb[0].min(u), bb[1].min(v), bb[2].max(u), bb[3].max(v)];
                }
                out.push(pts);
            }
            x += (g.right - g.left) * gs;
        }
    }
    if bb[0] > bb[2] {
        bb = [origin.0, origin.1, origin.0, origin.1];
    }
    Laid { strokes: out, bbox: bb, missing }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrive_a_mano() {
        for style in [Style::Stampatello, Style::Corsivo] {
            let f = font(style);
            assert_eq!(f.glyphs.len(), 96, "tutti i caratteri ASCII");
            let l = layout("Perché è così: 12 × 3 = 36 € e x² + H₂O", style, 0.04, 1.4, (0.1, 0.1), 2.0, false);
            assert!(l.missing.is_empty(), "{:?}", l.missing);
            assert!(!l.strokes.is_empty());
            // le maiuscole alte quanto chiesto (con l'irregolarità)
            let h = l.bbox[3] - l.bbox[1];
            assert!(h > 0.035 && h < 0.08, "{h}");
            assert!((l.bbox[0] - 0.1).abs() < 0.02 && (l.bbox[1] - 0.1).abs() < 0.02, "{:?}", l.bbox);
            // va a capo per stare nella larghezza
            let lunga = layout("una frase lunga che non sta su una riga sola del foglio", style, 0.04, 1.4, (0.1, 0.1), 0.3, false);
            assert!(lunga.bbox[2] <= 0.1 + 0.3 + 0.02, "{:?}", lunga.bbox);
            assert!(lunga.bbox[3] - lunga.bbox[1] > 0.1);
            // sempre uguale
            let again = layout("Perché è così: 12 × 3 = 36 € e x² + H₂O", style, 0.04, 1.4, (0.1, 0.1), 2.0, false);
            assert_eq!(l.strokes, again.strokes);
        }
        // la i accentata perde il puntino: un tratto in meno della «i» più l'accento
        let f = font(Style::Stampatello);
        assert_eq!(glyph(f, 'ì').unwrap().strokes.len(), glyph(f, 'i').unwrap().strokes.len());
        let l = layout("中", Style::Stampatello, 0.04, 1.4, (0.1, 0.1), 0.8, false);
        assert_eq!(l.missing, vec!['中']);
        // allineato a destra: finisce a `origin`
        let r = layout("ok", Style::Stampatello, 0.04, 1.4, (0.5, 0.1), 0.8, true);
        assert!(r.bbox[2] <= 0.5 + 0.005 && r.bbox[0] < 0.5, "{:?}", r.bbox);
    }
}
