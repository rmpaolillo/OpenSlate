//! Leggere il foglio della lavagna come lo legge una persona: righe di
//! scrittura dall'alto in basso, parole da sinistra a destra, il disegno a
//! mano, gli evidenziatori, il grafico dell'AI. Ogni pezzo ha un nome (`r2`,
//! `r2p3`, `d1`, `e1`) e un riquadro, così la chat può dire «sottolinea la
//! terza parola della seconda riga» con `"su": "r2p3"`.
//!
//! Le righe e le parole si trovano dalla geometria dei tratti (non serve la
//! trascrizione): i tratti alla stessa altezza fanno una riga, uno spazio più
//! largo di mezza lettera separa due parole. L'immagine con i nomi scritti
//! accanto serve al modello per vedere quale parola è quale.

use crate::scene::{PageBg, PageGeom};
use crate::stroke::{strokes_sig, Rgb, Stroke};

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Line,
    Word,
    Drawing,
    Highlight,
}

/// Un pezzo del foglio.
#[derive(Clone, Debug)]
pub struct Group {
    pub id: String,
    pub kind: Kind,
    /// u0, v0, u1, v1 (0..1 di larghezza e altezza della pagina).
    pub bbox: [f32; 4],
    /// Indici dei tratti.
    pub strokes: Vec<usize>,
    pub color: Rgb,
}

pub struct Line {
    pub group: Group,
    pub words: Vec<Group>,
}

/// Il foglio letto.
pub struct Reading {
    pub lines: Vec<Line>,
    pub drawing: Option<Group>,
    pub highlights: Vec<Group>,
    /// Dove sta il grafico dell'AI (u0, v0, u1, v1).
    pub chart: Option<[f32; 4]>,
    /// Altezza tipica di una lettera (frazione dell'altezza della pagina).
    pub char_h: f32,
    /// Larghezza / altezza della pagina.
    pub aspect: f32,
    /// Quanti tratti c'erano e la loro firma: se il foglio è cambiato solo
    /// perché si è scritto ancora, i nomi restano buoni.
    pub n: usize,
    pub sig: u64,
}

impl Reading {
    /// Il riquadro di un nome (`r2`, `r2p3`, `d1`, `e1`, `grafico`, `tutto`).
    pub fn find(&self, id: &str) -> Option<[f32; 4]> {
        let id = id.trim().to_lowercase();
        if id == "grafico" || id == "chart" {
            return self.chart;
        }
        if id == "tutto" || id == "all" {
            return self.lines.iter().map(|l| l.group.bbox).chain(self.drawing.iter().map(|d| d.bbox)).reduce(union);
        }
        if let Some(d) = self.drawing.as_ref().filter(|d| d.id == id) {
            return Some(d.bbox);
        }
        if let Some(h) = self.highlights.iter().find(|h| h.id == id) {
            return Some(h.bbox);
        }
        for l in &self.lines {
            if l.group.id == id {
                return Some(l.group.bbox);
            }
            if let Some(w) = l.words.iter().find(|w| w.id == id) {
                return Some(w.bbox);
            }
        }
        None
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty() && self.drawing.is_none() && self.highlights.is_empty() && self.chart.is_none()
    }
}

pub fn union(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])]
}

fn bbox(pts: impl Iterator<Item = (f32, f32)>) -> Option<[f32; 4]> {
    let mut b = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
    for (x, y) in pts {
        b = [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)];
    }
    (b[0] <= b[2]).then_some(b)
}

/// Il colore più usato fra i tratti.
fn main_color(strokes: &[Stroke], idx: &[usize]) -> Rgb {
    let mut counts: Vec<(Rgb, usize)> = Vec::new();
    for &i in idx {
        let c = strokes[i].color;
        match counts.iter_mut().find(|(k, _)| *k == c) {
            Some((_, n)) => *n += strokes[i].points.len(),
            None => counts.push((c, strokes[i].points.len())),
        }
    }
    counts.into_iter().max_by_key(|(_, n)| *n).map(|(c, _)| c).unwrap_or([0, 0, 0])
}

/// Il nome di un colore, per il modello.
pub fn color_name(c: Rgb) -> &'static str {
    let [r, g, b] = c.map(|v| v as f32);
    let (mx, mn) = (r.max(g).max(b), r.min(g).min(b));
    if mx < 70.0 {
        return "nero";
    }
    if mx - mn < 30.0 {
        return if mx > 200.0 { "bianco" } else { "grigio" };
    }
    let h = {
        let d = mx - mn;
        let h = if mx == r { ((g - b) / d).rem_euclid(6.0) } else if mx == g { (b - r) / d + 2.0 } else { (r - g) / d + 4.0 };
        h * 60.0
    };
    match h {
        h if !(15.0..345.0).contains(&h) => if mn > 150.0 { "rosa" } else { "rosso" },
        h if h < 40.0 => if mx < 160.0 { "marrone" } else { "arancione" },
        h if h < 70.0 => "giallo",
        h if h < 165.0 => "verde",
        h if h < 200.0 => "azzurro",
        h if h < 255.0 => "blu",
        h if h < 290.0 => "viola",
        _ => "rosa",
    }
}

/// Legge il foglio.
pub fn read(strokes: &[Stroke], g: &PageGeom, bg: Option<&PageBg>) -> Reading {
    let aspect = g.w / g.h;
    let ub: Vec<Option<[f32; 4]>> = strokes.iter().map(|st| bbox(st.points.iter().map(|p| g.unit(p.x, p.y)))).collect();
    let n = strokes.len();
    let ink: Vec<usize> = (0..n).filter(|&i| ub[i].is_some() && !strokes[i].highlight).collect();
    // il disegno a mano (assi e scritte attorno)
    let ink_boxes: Vec<Option<[f32; 4]>> = (0..n).map(|i| if ink.contains(&i) { ub[i] } else { None }).collect();
    let drawing = crate::boardagent::find_drawing(&ink_boxes).map(|(b, m)| Group {
        id: "d1".into(),
        kind: Kind::Drawing,
        bbox: b,
        color: main_color(strokes, &m),
        strokes: m,
    });
    let in_drawing = |i: usize| drawing.as_ref().is_some_and(|d| d.strokes.contains(&i));
    let text: Vec<usize> = ink.iter().copied().filter(|&i| !in_drawing(i)).collect();
    // altezza tipica di una lettera: fra le più alte dei tratti (senza
    // puntini); nel corsivo un tratto è una parola intera, nello stampatello
    // un pezzo di lettera
    let mut hs: Vec<f32> = text.iter().map(|&i| ub[i].unwrap()).map(|b| b[3] - b[1]).filter(|&h| h > 0.004).collect();
    hs.sort_by(f32::total_cmp);
    let char_h = hs.get(hs.len() * 3 / 4).copied().unwrap_or(0.03).clamp(0.008, 0.08);
    let hh = char_h;
    let phys = |b: [f32; 4]| [b[0] * aspect, b[1], b[2] * aspect, b[3]];
    // puntini, accenti, trattini, sbarrette della t: si aggiungono dopo
    let small = |b: [f32; 4]| b[3] - b[1] < 0.4 * hh;
    let main: Vec<usize> = text.iter().copied().filter(|&i| !small(ub[i].unwrap())).collect();
    // righe: tratti vicini a fianco e alla stessa altezza (si sovrappongono
    // in verticale per almeno metà del più basso) stanno insieme…
    let mut parent: Vec<usize> = (0..main.len()).collect();
    fn root(p: &mut [usize], mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    let pb: Vec<[f32; 4]> = main.iter().map(|&i| phys(ub[i].unwrap())).collect();
    let same_row = |a: [f32; 4], b: [f32; 4], k: f32| {
        let ov = a[3].min(b[3]) - a[1].max(b[1]);
        ov >= k * (a[3] - a[1]).min(b[3] - b[1])
    };
    for a in 0..main.len() {
        for b in a + 1..main.len() {
            let gap = (pb[b][0] - pb[a][2]).max(pb[a][0] - pb[b][2]).max(0.0);
            if gap < 2.5 * hh && same_row(pb[a], pb[b], 0.5) {
                let (ra, rb) = (root(&mut parent, a), root(&mut parent, b));
                parent[ra] = rb;
            }
        }
    }
    let mut comps: Vec<(Vec<usize>, [f32; 4])> = Vec::new();
    let mut slot: Vec<Option<usize>> = vec![None; main.len()];
    for a in 0..main.len() {
        let r = root(&mut parent, a);
        match slot[r] {
            Some(k) => {
                comps[k].0.push(main[a]);
                comps[k].1 = union(comps[k].1, pb[a]);
            }
            None => {
                slot[r] = Some(comps.len());
                comps.push((vec![main[a]], pb[a]));
            }
        }
    }
    // …e i pezzi lontani alla stessa altezza fanno la stessa riga
    comps.sort_by(|a, b| ((a.1[1] + a.1[3]) / 2.0).total_cmp(&((b.1[1] + b.1[3]) / 2.0)));
    struct L {
        members: Vec<usize>,
    }
    let mut lines: Vec<(L, [f32; 4])> = Vec::new();
    for (m, b) in comps {
        match lines.iter_mut().find(|(_, lb)| same_row(*lb, b, 0.6)) {
            Some((l, lb)) => {
                l.members.extend(m);
                *lb = union(*lb, b);
            }
            None => lines.push((L { members: m }, b)),
        }
    }
    let tiny = small;
    let mut lines: Vec<L> = lines.into_iter().map(|(l, _)| l).collect();
    // puntini, accenti, virgole: alla riga più vicina
    for &i in text.iter().filter(|&&i| tiny(ub[i].unwrap())) {
        let b = phys(ub[i].unwrap());
        let c = (b[1] + b[3]) / 2.0;
        let mut best: Option<(usize, f32)> = None;
        for (k, l) in lines.iter().enumerate() {
            let lb = l.members.iter().map(|&j| phys(ub[j].unwrap())).reduce(union).unwrap();
            let dy = (lb[1] - 0.6 * hh - c).max(c - lb[3] - 0.3 * hh).max(0.0);
            let dx = (lb[0] - b[2]).max(b[0] - lb[2]).max(0.0);
            if dy < 0.8 * hh && dx < 1.5 * hh {
                let d = dy + dx;
                if best.is_none_or(|(_, bd)| d < bd) {
                    best = Some((k, d));
                }
            }
        }
        match best {
            Some((k, _)) => lines[k].members.push(i),
            None => lines.push(L { members: vec![i] }),
        }
    }
    let mut lines: Vec<(Vec<usize>, [f32; 4])> = lines
        .into_iter()
        .map(|l| {
            let b = l.members.iter().map(|&j| ub[j].unwrap()).reduce(union).unwrap();
            (l.members, b)
        })
        .collect();
    lines.sort_by(|a, b| ((a.1[1] + a.1[3]) / 2.0).total_cmp(&((b.1[1] + b.1[3]) / 2.0)).then(a.1[0].total_cmp(&b.1[0])));
    // parole: da sinistra, uno spazio più largo di mezza lettera le separa
    let mut out = Vec::new();
    for (li, (mut members, lb)) in lines.into_iter().enumerate() {
        members.sort_by(|&a, &b| ub[a].unwrap()[0].total_cmp(&ub[b].unwrap()[0]));
        let mut words: Vec<(Vec<usize>, [f32; 4])> = Vec::new();
        for &i in &members {
            let b = ub[i].unwrap();
            match words.last_mut() {
                Some((m, wb)) if (b[0] - wb[2]) * aspect < 0.45 * hh => {
                    m.push(i);
                    *wb = union(*wb, b);
                }
                _ => words.push((vec![i], b)),
            }
        }
        let rid = format!("r{}", li + 1);
        let words = words
            .into_iter()
            .enumerate()
            .map(|(wi, (m, b))| Group { id: format!("{rid}p{}", wi + 1), kind: Kind::Word, bbox: b, color: main_color(strokes, &m), strokes: m })
            .collect();
        out.push(Line { group: Group { id: rid, kind: Kind::Line, bbox: lb, color: main_color(strokes, &members), strokes: members }, words });
    }
    let highlights = (0..n)
        .filter(|&i| ub[i].is_some() && strokes[i].highlight)
        .enumerate()
        .map(|(k, i)| Group { id: format!("e{}", k + 1), kind: Kind::Highlight, bbox: ub[i].unwrap(), strokes: vec![i], color: strokes[i].color })
        .collect();
    let chart = bg.filter(|b| b.is_chart()).map(|b| [b.rect[0], b.rect[1], b.rect[0] + b.rect[2], b.rect[1] + b.rect[3]]);
    Reading { lines: out, drawing, highlights, chart, char_h, aspect, n, sig: strokes_sig(strokes) }
}

fn k(b: [f32; 4]) -> String {
    let q = |v: f32| (v * 1000.0).round().clamp(0.0, 1000.0) as i32;
    format!("[{}, {}, {}, {}]", q(b[0]), q(b[1]), q(b[2]), q(b[3]))
}

/// Il foglio in parole, per il modello.
pub fn describe(r: &Reading) -> String {
    if r.is_empty() {
        return "Il foglio della lavagna è vuoto.".into();
    }
    let mut s = format!(
        "Foglio della lavagna. Coordinate 0–1000 sull'immagine della pagina (x da sinistra, y dall'alto; la pagina è larga {:.2} volte la sua altezza). \
         Altezza tipica della scrittura: {:.0}.\n",
        r.aspect,
        r.char_h * 1000.0
    );
    if !r.lines.is_empty() {
        s.push_str("Righe scritte a mano, dall'alto (con le parole da sinistra):\n");
        for l in &r.lines {
            s.push_str(&format!("{} {} {}:", l.group.id, k(l.group.bbox), color_name(l.group.color)));
            for (i, w) in l.words.iter().enumerate() {
                s.push_str(&format!("{} {} {}", if i == 0 { "" } else { "," }, w.id, k(w.bbox)));
                if w.color != l.group.color {
                    s.push_str(&format!(" {}", color_name(w.color)));
                }
            }
            s.push('\n');
        }
    }
    if let Some(d) = &r.drawing {
        s.push_str(&format!("{} disegno a mano (assi, figure, con le scritte attorno) {}\n", d.id, k(d.bbox)));
    }
    for h in &r.highlights {
        s.push_str(&format!("{} evidenziatore {} {}\n", h.id, color_name(h.color), k(h.bbox)));
    }
    if let Some(c) = r.chart {
        s.push_str(&format!("grafico dell'AI {}\n", k(c)));
    }
    s.push_str(
        "Le righe e le parole vengono dalla posizione dei tratti, non dalla lettura: nell'immagine allegata ogni riga ha il suo nome \
         a sinistra e ogni parola un riquadro; guarda lì quale parola è quale (e per leggere il testo). Un nome «r2p3» vale finché \
         il foglio non cambia: dopo una modifica tua o dell'utente rileggi il foglio.",
    );
    s
}

/// L'immagine del foglio con le righe nominate e le parole nei riquadri.
pub fn labelled_png(page: &crate::scene::Page, r: &Reading) -> anyhow::Result<Vec<u8>> {
    use tiny_skia::{Paint, PathBuilder, Pixmap, Stroke as Sk, Transform};
    let png = crate::export::page_to_png(page, 1400)?;
    let mut pm = Pixmap::decode_png(&png).map_err(|e| anyhow::anyhow!("PNG: {e}"))?;
    let (w, h) = (pm.width() as f32, pm.height() as f32);
    let mut boxp = Paint::default();
    boxp.set_color_rgba8(40, 120, 230, 150);
    boxp.anti_alias = true;
    let mut labp = Paint::default();
    labp.set_color_rgba8(200, 30, 140, 255);
    labp.anti_alias = true;
    let thin = Sk { width: 1.4, ..Sk::default() };
    let lab = Sk { width: 2.0, line_cap: tiny_skia::LineCap::Round, line_join: tiny_skia::LineJoin::Round, ..Sk::default() };
    let rect = |pm: &mut Pixmap, b: [f32; 4], p: &Paint| {
        if let Some(rr) = tiny_skia::Rect::from_ltrb(b[0] * w - 2.0, b[1] * h - 2.0, b[2] * w + 2.0, b[3] * h + 2.0) {
            let path = PathBuilder::from_rect(rr);
            pm.stroke_path(&path, p, &thin, Transform::identity(), None);
        }
    };
    let label = |pm: &mut Pixmap, text: &str, right_of: f32, v: f32| {
        // il nome a sinistra della riga (o dentro il margine, se non c'è posto)
        let cap = (16.0 / h).max(0.012);
        let laid = crate::handwrite::layout(text, crate::handwrite::Style::Stampatello, cap, w / h, (right_of, v - cap / 2.0), 1.0, true);
        for st in &laid.strokes {
            let mut pb = PathBuilder::new();
            for (j, &(u, vv)) in st.iter().enumerate() {
                if j == 0 {
                    pb.move_to(u * w, vv * h);
                } else {
                    pb.line_to(u * w, vv * h);
                }
            }
            if let Some(path) = pb.finish() {
                pm.stroke_path(&path, &labp, &lab, Transform::identity(), None);
            }
        }
    };
    for l in &r.lines {
        for wd in &l.words {
            rect(&mut pm, wd.bbox, &boxp);
        }
        let b = l.group.bbox;
        let x = if b[0] > 0.06 { b[0] - 0.008 } else { b[0] + 0.05 };
        let y = if b[0] > 0.06 { (b[1] + b[3]) / 2.0 } else { b[1] - 0.012 };
        label(&mut pm, &l.group.id, x, y);
    }
    if let Some(d) = &r.drawing {
        rect(&mut pm, d.bbox, &labp);
        label(&mut pm, &d.id, d.bbox[0] - 0.008, d.bbox[1] + 0.01);
    }
    for e in &r.highlights {
        label(&mut pm, &e.id, e.bbox[0] - 0.008, (e.bbox[1] + e.bbox[3]) / 2.0);
    }
    pm.encode_png().map_err(|e| anyhow::anyhow!("PNG: {e}"))
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::scene::Scene;
    use crate::stroke::{PenEvent, PenPhase};

    /// Un tratto da (u, v) a (u, v) sulla pagina, con `n` punti.
    pub fn line(g: &PageGeom, a: (f32, f32), b: (f32, f32)) -> Stroke {
        let mut st = Stroke::new([0, 0, 0], 2.0);
        for k in 0..=8 {
            let t = k as f32 / 8.0;
            let (x, y) = g.pad(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
            st.points.push(PenEvent { x, y, pressure: 0.5, phase: if k == 0 { PenPhase::Down } else { PenPhase::Move } });
        }
        st
    }

    /// Una «lettera» alta `h` a (u, v) (in alto a sinistra), larga `w`.
    pub fn letter(g: &PageGeom, u: f32, v: f32, w: f32, h: f32) -> Stroke {
        line(g, (u, v + h), (u + w, v))
    }

    /// Una parola di `n` lettere: ritorna i tratti e dove finisce.
    pub fn word(g: &PageGeom, u: f32, v: f32, n: usize) -> (Vec<Stroke>, f32) {
        let (lw, gap, h) = (0.012, 0.004, 0.03);
        let s = (0..n).map(|k| letter(g, u + k as f32 * (lw + gap), v, lw, h)).collect();
        (s, u + n as f32 * (lw + gap))
    }

    pub fn geom() -> PageGeom {
        Scene::new(&crate::settings::Settings::default()).geom()
    }

    #[test]
    fn righe_e_parole() {
        let g = geom();
        let mut strokes = Vec::new();
        // riga 1: tre parole; riga 2 (un po' storta): due parole, con un puntino
        let (w, end) = word(&g, 0.10, 0.10, 4);
        strokes.extend(w);
        let (w, end) = word(&g, end + 0.03, 0.10, 3);
        strokes.extend(w);
        let (w, _) = word(&g, end + 0.03, 0.101, 5);
        strokes.extend(w);
        let (w, end2) = word(&g, 0.10, 0.20, 6);
        strokes.extend(w);
        let (w, _) = word(&g, end2 + 0.035, 0.208, 2);
        strokes.extend(w);
        strokes.push(line(&g, (0.105, 0.195), (0.106, 0.196))); // il puntino della «i»
        let r = read(&strokes, &g, None);
        assert_eq!(r.lines.len(), 2, "{}", describe(&r));
        assert_eq!(r.lines[0].words.len(), 3, "{}", describe(&r));
        assert_eq!(r.lines[1].words.len(), 2, "{}", describe(&r));
        assert!(r.lines[1].group.strokes.contains(&(strokes.len() - 1)), "il puntino sta con la riga sotto");
        let b = r.find("r1p2").unwrap();
        assert!(b[0] > 0.15 && b[2] < 0.25 && (b[1] - 0.10).abs() < 0.005, "{b:?}");
        assert!(r.find("r3").is_none());
        let t = describe(&r);
        assert!(t.contains("r1 [") && t.contains("r2p2") && t.contains("nero"), "{t}");
        // un foglio vuoto
        assert!(describe(&read(&[], &g, None)).contains("vuoto"));
        assert_eq!(color_name([20, 60, 200]), "blu");
        assert_eq!(color_name([220, 30, 30]), "rosso");
        assert_eq!(color_name([255, 230, 80]), "giallo");
    }
}
