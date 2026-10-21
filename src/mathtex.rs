//! Formule LaTeX disegnate in stile LaTeX, per i Markdown aperti come documento.
//!
//! Il LaTeX diventa codice Typst con MiTeX (https://github.com/mitex-rs/mitex,
//! Apache-2.0) e Typst (https://typst.app, Apache-2.0) lo impagina coi suoi
//! font New Computer Modern, gli stessi disegni di LaTeX e di KaTeX: le
//! formule escono come negli Appunti. Le definizioni che MiTeX si aspetta
//! (`src/typst-pkg/mitex-specs/`, dal suo pacchetto Typst) sono dentro l'app: niente rete,
//! niente programmi esterni.
//!
//! Il risultato è un PNG ad alta risoluzione con le misure in punti e la
//! profondità sotto la linea di base (per allineare le formule nel testo).

use anyhow::{anyhow, bail, Result};
use std::collections::HashMap;
use std::sync::Mutex;
use typst::layout::{Abs, FrameItem, FrameKind, Point};

/// Risoluzione delle formule (pixel per pollice).
const PPI: f32 = 600.0;

/// Una formula disegnata.
#[derive(Clone)]
pub struct Formula {
    pub png: Vec<u8>,
    /// Larghezza e altezza in punti.
    pub width: f32,
    pub height: f32,
    /// Quanto scende sotto la linea di base del testo (punti).
    pub depth: f32,
}

/// La formula `latex` (senza `$`), in blocco (`display`) o nel testo, con il
/// corpo del carattere di `size` punti. Le formule già disegnate si ricordano.
pub fn render(latex: &str, display: bool, size: f32) -> Result<Formula> {
    static CACHE: Mutex<Option<HashMap<(String, bool, u32), Formula>>> = Mutex::new(None);
    let key = (latex.to_string(), display, (size * 100.0) as u32);
    if let Some(f) = CACHE.lock().unwrap().get_or_insert_with(HashMap::new).get(&key) {
        return Ok(f.clone());
    }
    let f = render_uncached(latex, display, size)?;
    let mut c = CACHE.lock().unwrap();
    let c = c.get_or_insert_with(HashMap::new);
    if c.len() > 2000 {
        c.clear();
    }
    c.insert(key, f.clone());
    Ok(f)
}

fn render_uncached(latex: &str, display: bool, size: f32) -> Result<Formula> {
    let code = mitex::convert_math(latex, None).map_err(|e| anyhow!("MiTeX: {e}"))?;
    // il codice convertito va valutato con le definizioni di MiTeX; la formula
    // in un riquadro (`box`) tiene la sua linea di base
    let src = format!(
        "#import \"/mitex/mod.typ\": mitex-scope\n\
         #set page(width: auto, height: auto, margin: (x: 0.6pt, y: 2pt), fill: none)\n\
         #set text(size: {size}pt, top-edge: \"bounds\", bottom-edge: \"bounds\")\n\
         #set math.cases(gap: 0.45em)\n\
         #show math.mat: it => math.display(it)\n\
         #let f = math.equation(block: {display}, eval(\"$\" + {code:?} + \"$\", scope: mitex-scope))\n\
         #box(f)\n"
    );
    let doc = crate::typstworld::compile(src)?;
    let page = &doc.pages()[0];
    let frame = &page.frame;
    let (w, h) = (frame.width().to_pt() as f32, frame.height().to_pt() as f32);
    if w <= 0.0 || h <= 0.0 {
        bail!("Typst: formula vuota");
    }
    // la linea di base: quella del riquadro della formula
    let base = baseline(frame, Point::zero()).map(|b| b.to_pt() as f32).unwrap_or(h);
    let png = crate::typstworld::png(&doc, PPI)?;
    Ok(Formula { png, width: w, height: h, depth: (h - base).max(0.0) })
}

/// La linea di base del primo riquadro rigido (la formula) dentro la pagina.
fn baseline(frame: &typst::layout::Frame, at: Point) -> Option<Abs> {
    for (pos, item) in frame.items() {
        if let FrameItem::Group(g) = item {
            let p = at + *pos;
            if g.frame.kind() == FrameKind::Hard && g.frame.has_baseline() {
                return Some(p.y + g.frame.baseline());
            }
            if let Some(b) = baseline(&g.frame, p) {
                return Some(b);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formule_comuni() {
        let all = [
            (r"\int_a^b f(x)\,dx", true),
            ("x^2", false),
            (r"\begin{cases} x-y=1\\ x+y=0 \end{cases}", true),
            (r"\frac{a}{b}+\sqrt{2}+\sqrt[3]{x}", false),
            (r"\mathbb{R}\to\mathrm{d}x \text{ per ogni } \operatorname{sen} x", false),
            (r"\begin{pmatrix}1&2\\3&4\end{pmatrix}", true),
            (r"\begin{aligned}a&=b\\c&=d\end{aligned}", true),
            (r"\left(\frac{1}{2}\right)^n \sum_{i=1}^n i \lim_{x\to 0}", true),
            (r"\overline{AB}\hat{x}\vec{v}\cdot\le\ne\infty\alpha\Delta", false),
            (r"\binom{n}{k} \dfrac{1}{2} \boxed{y}", true),
        ];
        for (l, d) in all {
            let f = render(l, d, 14.5).unwrap_or_else(|e| panic!("{l}: {e:#}"));
            assert!(f.png.starts_with(b"\x89PNG") && f.width > 3.0 && f.height > 3.0, "{l}");
        }
        // x² nel testo: quasi niente sotto la linea di base; f(x) scende un po'
        let x2 = render("x^2", false, 14.5).unwrap();
        let fx = render("f(x)", false, 14.5).unwrap();
        assert!(x2.depth < 3.0 && fx.depth > x2.depth, "{} {}", x2.depth, fx.depth);
    }
}

