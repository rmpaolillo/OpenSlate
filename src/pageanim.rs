//! Formati di una pagina che non hanno bisogno della desktop: Xournal++, GIF
//! animata, SVG animato e la «scrittura che si rifà» (anche per il video).
//! Li usa anche l'app per telefoni (`mobile/core`), che include questo file.

use crate::scene::{Page, PageStroke};
use anyhow::{anyhow, bail, Result};

// ---------------------------------------------------------------------------
// Xournal++
// ---------------------------------------------------------------------------

/// Millimetri → punti.
const MM: f32 = 72.0 / 25.4;

fn hex_color(c: [u8; 3], alpha: u8) -> String {
    format!("#{:02x}{:02x}{:02x}{alpha:02x}", c[0], c[1], c[2])
}

/// La pagina come documento di Xournal++ (`.xopp`: XML compresso con gzip).
pub fn xournal(page: &Page, paper: &str) -> Result<Vec<u8>> {
    use std::fmt::Write;
    let (wmm, hmm) = crate::export::page_mm(page);
    let (w, h) = (wmm * MM, hmm * MM);
    let style = if paper.starts_with("righe") {
        "lined"
    } else if paper.starts_with("quadretti") || paper.contains("millimetrata") || paper.contains("carta") {
        "graph"
    } else {
        "plain"
    };
    let mut x = String::new();
    x.push_str("<?xml version=\"1.0\" standalone=\"no\"?>\n<xournal creator=\"OpenSlate\" fileversion=\"4\">\n");
    x.push_str("<title>Xournal++ document - see https://xournalpp.github.io/</title>\n");
    let _ = writeln!(x, "<page width=\"{w:.2}\" height=\"{h:.2}\">");
    let _ = writeln!(x, "<background type=\"solid\" color=\"#ffffffff\" style=\"{style}\"/>");
    x.push_str("<layer>\n");
    if let Some(bg) = &page.bg {
        use base64::Engine;
        let [rx, ry, rw, rh] = bg.rect;
        let _ = writeln!(
            x,
            "<image left=\"{:.2}\" top=\"{:.2}\" right=\"{:.2}\" bottom=\"{:.2}\">{}</image>",
            rx * w,
            ry * h,
            (rx + rw) * w,
            (ry + rh) * h,
            base64::engine::general_purpose::STANDARD.encode(bg.png.as_slice())
        );
    }
    for st in &page.strokes {
        if st.pts.is_empty() {
            continue;
        }
        let width = crate::export::stroke_width(st.width, h);
        let (tool, color) = if st.highlight { ("highlighter", hex_color(st.color, 0x7f)) } else { ("pen", hex_color(st.color, 0xff)) };
        let mut pts: Vec<(f32, f32)> = st.pts.iter().map(|&(u, v)| (u * w, v * h)).collect();
        if pts.len() == 1 {
            pts.push((pts[0].0 + 0.01, pts[0].1));
        }
        let coords: Vec<String> = pts.iter().map(|(a, b)| format!("{a:.2} {b:.2}")).collect();
        let _ = writeln!(x, "<stroke tool=\"{tool}\" color=\"{color}\" width=\"{width:.2}\">{}</stroke>", coords.join(" "));
    }
    x.push_str("</layer>\n</page>\n</xournal>\n");
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    std::io::Write::write_all(&mut gz, x.as_bytes())?;
    Ok(gz.finish()?)
}

// ---------------------------------------------------------------------------
// La scrittura che si rifà: GIF e SVG animato
// ---------------------------------------------------------------------------

/// Il tratto `s` fino alla frazione `t` della sua lunghezza (`a` = larghezza
/// su altezza della pagina, per misurare le lunghezze giuste).
fn partial(s: &PageStroke, t: f32, a: f32) -> PageStroke {
    let len = stroke_len(s, a);
    let want = len * t.clamp(0.0, 1.0);
    let mut pts = vec![s.pts[0]];
    let mut acc = 0.0;
    for w in s.pts.windows(2) {
        let d = (((w[1].0 - w[0].0) * a).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt();
        if acc + d >= want {
            let f = if d > 0.0 { (want - acc) / d } else { 0.0 };
            pts.push((w[0].0 + (w[1].0 - w[0].0) * f, w[0].1 + (w[1].1 - w[0].1) * f));
            break;
        }
        acc += d;
        pts.push(w[1]);
    }
    PageStroke { pts, ..s.clone() }
}

fn stroke_len(s: &PageStroke, a: f32) -> f32 {
    s.pts.windows(2).map(|w| (((w[1].0 - w[0].0) * a).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt()).sum::<f32>()
}

/// I tratti nell'ordine in cui si rifanno: prima l'inchiostro (nell'ordine
/// scritto), poi gli evidenziatori; e quando comincia e finisce ciascuno, in
/// frazioni del disegno intero (i salti fra un tratto e l'altro contano un po').
pub fn timeline(page: &Page) -> Vec<(&PageStroke, f32, f32)> {
    let a = page.w / page.h;
    let order: Vec<&PageStroke> = page.strokes.iter().filter(|s| !s.highlight).chain(page.strokes.iter().filter(|s| s.highlight)).filter(|s| !s.pts.is_empty()).collect();
    const GAP: f32 = 0.004;
    let lens: Vec<f32> = order.iter().map(|s| stroke_len(s, a).max(0.002)).collect();
    let total: f32 = lens.iter().sum::<f32>() + GAP * order.len() as f32;
    let mut t = 0.0;
    order
        .into_iter()
        .zip(lens)
        .map(|(s, l)| {
            let start = t / total;
            t += l;
            let end = t / total;
            t += GAP;
            (s, start, end)
        })
        .collect()
}

/// La pagina al momento `t` (0..1) del disegno.
pub fn page_at(page: &Page, tl: &[(&PageStroke, f32, f32)], t: f32) -> Page {
    let a = page.w / page.h;
    let strokes = tl
        .iter()
        .filter(|(_, s, _)| *s <= t)
        .map(|(st, s, e)| if t >= *e { (*st).clone() } else { partial(st, (t - s) / (e - s).max(1e-6), a) })
        .collect();
    Page { w: page.w, h: page.h, strokes, bg: page.bg.clone() }
}

/// GIF animata: la pagina che si scrive in ~5 s, poi resta ferma 3 s e
/// ricomincia. Ogni fotogramma porta solo la parte che cambia.
pub fn gif(page: &Page) -> Result<Vec<u8>> {
    const LONG: u32 = 900;
    const FRAMES: usize = 80;
    const DELAY: u16 = 6; // centesimi di secondo
    let tl = timeline(page);
    if tl.is_empty() {
        bail!("pagina vuota");
    }
    let (w, h, last) = crate::export::page_to_rgba(page, LONG)?;
    // tavolozza unica, presa dalla pagina finita
    let nq = color_quant::NeuQuant::new(10, 256, &last);
    let palette: Vec<u8> = nq.color_map_rgb();
    let mut cache: std::collections::HashMap<[u8; 3], u8> = std::collections::HashMap::new();
    let mut index = |px: &[u8]| -> u8 {
        let k = [px[0], px[1], px[2]];
        *cache.entry(k).or_insert_with(|| nq.index_of(&[px[0], px[1], px[2], 255]) as u8)
    };
    let mut out = Vec::new();
    {
        let mut enc = gif::Encoder::new(&mut out, w as u16, h as u16, &palette).map_err(|e| anyhow!("GIF: {e}"))?;
        enc.set_repeat(gif::Repeat::Infinite).map_err(|e| anyhow!("GIF: {e}"))?;
        let mut prev: Option<Vec<u8>> = None;
        for k in 0..=FRAMES {
            let t = k as f32 / FRAMES as f32;
            let rgba = if k == FRAMES { last.clone() } else { crate::export::page_to_rgba(&page_at(page, &tl, t), LONG)?.2 };
            // la zona cambiata rispetto al fotogramma prima
            let (x0, y0, x1, y1) = match &prev {
                None => (0, 0, w, h),
                Some(p) => {
                    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
                    for y in 0..h {
                        let row = y * w * 4;
                        for x in 0..w {
                            let i = row + x * 4;
                            if p[i..i + 3] != rgba[i..i + 3] {
                                x0 = x0.min(x);
                                y0 = y0.min(y);
                                x1 = x1.max(x + 1);
                                y1 = y1.max(y + 1);
                            }
                        }
                    }
                    if x1 <= x0 {
                        (0, 0, 1, 1)
                    } else {
                        (x0, y0, x1, y1)
                    }
                }
            };
            let (fw, fh) = (x1 - x0, y1 - y0);
            let mut buf = Vec::with_capacity(fw * fh);
            for y in y0..y1 {
                for x in x0..x1 {
                    let i = (y * w + x) * 4;
                    buf.push(index(&rgba[i..i + 4]));
                }
            }
            let frame = gif::Frame {
                left: x0 as u16,
                top: y0 as u16,
                width: fw as u16,
                height: fh as u16,
                buffer: std::borrow::Cow::Owned(buf),
                delay: if k == FRAMES { 300 } else { DELAY },
                dispose: gif::DisposalMethod::Keep,
                ..Default::default()
            };
            enc.write_frame(&frame).map_err(|e| anyhow!("GIF: {e}"))?;
            prev = Some(rgba);
        }
    }
    Ok(out)
}

/// SVG animato: ogni tratto si disegna da solo (CSS), poi la pagina resta
/// ferma e ricomincia. Si apre in qualsiasi browser.
pub fn svg_anim(page: &Page) -> String {
    use std::fmt::Write;
    const DRAW: f32 = 6.0; // secondi per scrivere la pagina
    const HOLD: f32 = 3.0;
    let cycle = DRAW + HOLD;
    let (wmm, hmm) = crate::export::page_mm(page);
    let tl = timeline(page);
    let mut s = String::new();
    let _ = writeln!(
        s,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"{wmm:.1}mm\" height=\"{hmm:.1}mm\" viewBox=\"0 0 {wmm:.3} {hmm:.3}\">"
    );
    s.push_str("<!-- OpenSlate: la pagina che si riscrive da sola (https://github.com/rmpaolillo/OpenSlate) -->\n<style>\n");
    s.push_str(".s{fill:none;stroke-linecap:round;stroke-linejoin:round;stroke-dasharray:1 2;stroke-dashoffset:1.01}\n.h{mix-blend-mode:multiply}\n");
    for (i, (_, a, b)) in tl.iter().enumerate() {
        let (p0, p1) = (a * DRAW / cycle * 100.0, (b * DRAW / cycle * 100.0).max(a * DRAW / cycle * 100.0 + 0.01));
        let _ = writeln!(
            s,
            "@keyframes k{i}{{0%,{p0:.3}%{{stroke-dashoffset:1.01}}{p1:.3}%,100%{{stroke-dashoffset:0}}}}\n#t{i}{{animation:k{i} {cycle}s linear infinite}}"
        );
    }
    s.push_str("</style>\n");
    let _ = writeln!(s, "<rect width=\"{wmm:.3}\" height=\"{hmm:.3}\" fill=\"#ffffff\"/>");
    if let Some(bg) = &page.bg {
        use base64::Engine;
        let [rx, ry, rw, rh] = bg.rect;
        let _ = writeln!(
            s,
            "<image x=\"{:.3}\" y=\"{:.3}\" width=\"{:.3}\" height=\"{:.3}\" preserveAspectRatio=\"none\" xlink:href=\"data:image/png;base64,{}\"/>",
            rx * wmm,
            ry * hmm,
            rw * wmm,
            rh * hmm,
            base64::engine::general_purpose::STANDARD.encode(bg.png.as_slice())
        );
    }
    for (i, (st, _, _)) in tl.iter().enumerate() {
        let mut d = String::new();
        for (k, &(u, v)) in st.pts.iter().enumerate() {
            let _ = write!(d, "{}{:.2} {:.2}", if k == 0 { "M" } else { " L" }, u * wmm, v * hmm);
        }
        if st.pts.len() == 1 {
            let _ = write!(d, " l0.01 0");
        }
        let width = crate::export::stroke_width(st.width, hmm);
        let c = st.color;
        let _ = writeln!(
            s,
            "<path id=\"t{i}\" class=\"s{}\" pathLength=\"1\" stroke=\"#{:02x}{:02x}{:02x}\" stroke-width=\"{width:.3}\" d=\"{d}\"/>",
            if st.highlight { " h" } else { "" },
            c[0],
            c[1],
            c[2]
        );
    }
    s.push_str("</svg>\n");
    s
}
