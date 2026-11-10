//! «Disponi sulle righe»: la scrittura di una zona (o di tutta la pagina),
//! a mano o già in bella, si posa sulle righe del foglio scelto.
//!
//! Ogni riga scritta:
//! - si raddrizza (se sale o scende, la si ruota sulla riga del foglio);
//! - si rimpicciolisce solo se è più alta dello spazio della riga (non si
//!   ingrandisce mai);
//! - comincia dallo stesso bordo a sinistra delle altre (il margine rosso, il
//!   bordo della zona); i trattini di un elenco restano in colonna e il testo
//!   dopo di loro pure;
//! - va sulla riga del foglio dopo quella della riga precedente, senza righe
//!   vuote in mezzo.
//!
//! Le parole di una riga restano alla stessa distanza fra loro; i puntini, gli
//! accenti, le sottolineature e gli evidenziatori seguono la loro riga. I
//! disegni restano dove sono, e le righe non ci finiscono sopra.
//!
//! Sui quadretti (e sui puntini, sulla millimetrata) le righe si fanno ogni N
//! quadretti, N scelto da chi scrive (`suggest_every` propone quello in cui
//! la scrittura ci sta). Sulla carta bianca e sul pentagramma non si fa.
//!
//! Lo stesso codice per la desktop e per le app del telefono.

use crate::inbella::{base_and_xheight, bbox, bullet, drawing_line_by, letters_by, physical, quantile, turns_xy};
use crate::rigatura::{self, Paper, Row, Rows};
use crate::scene::PageGeom;
use crate::stroke::Stroke;

/// Il foglio: la carta, il margine rosso, ogni quanti quadretti una riga
/// (solo per le griglie; `None` = quello proposto).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sheet {
    pub paper: Paper,
    pub margin: bool,
    pub every: Option<u32>,
}

/// La pagina con la scrittura sulle righe.
#[derive(Clone, Debug)]
pub struct Arranged {
    /// Tutti i tratti della pagina, nello stesso ordine: quelli spostati al
    /// loro posto nuovo.
    pub strokes: Vec<Stroke>,
    /// Gli indici dei tratti spostati.
    pub moved: Vec<usize>,
    /// Righe disposte e quante rimpicciolite.
    pub lines: usize,
    pub shrunk: usize,
    /// Una riga ogni quanti quadretti (griglie).
    pub every: Option<u32>,
    /// Dove è cambiato qualcosa (u0, v0, u1, v1, prima e dopo insieme).
    pub area: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    /// Nella zona non c'è scrittura da disporre.
    Nothing,
    /// La carta non ha righe (bianca, pentagramma).
    Unsupported,
    /// Le righe del foglio non bastano: ne stanno `fit` su `total`.
    NoRoom { fit: usize, total: usize },
}

/// Una riga scritta, misurata. Le misure «r» sono dopo averla raddrizzata,
/// rispetto ad `anchor`.
struct Ln {
    strokes: Vec<usize>,
    bullet: Option<usize>,
    anchor: (f32, f32),
    cos: f32,
    sin: f32,
    base: f32,
    xh: f32,
    up: f32,
    down: f32,
    /// Dove comincia il testo (dopo il trattino) e dove finisce la riga.
    tx0: f32,
    bx0: f32,
    x1: f32,
    /// L'altezza tipica di un tratto della riga, e se la riga è fatta di più
    /// righe (un sistema con la graffa, una frazione grande): allora si tiene
    /// intera e prende le righe del foglio che servono.
    gh: f32,
    multi: bool,
    /// La riga di base com'è ora sul foglio, e il riquadro di adesso.
    base_now: f32,
    old: (f32, f32, f32, f32),
}

impl Ln {
    fn rot(&self, p: (f32, f32)) -> (f32, f32) {
        let (dx, dy) = (p.0 - self.anchor.0, p.1 - self.anchor.1);
        (self.cos * dx + self.sin * dy, -self.sin * dx + self.cos * dy)
    }
}

/// Le righe scritte della scelta `sel` (indici in `strokes`), misurate; i
/// disegni non ci sono.
fn measure(strokes: &[Stroke], g: &PageGeom, ph: &[Vec<(f32, f32)>], sel: &[usize]) -> (Vec<Ln>, f32) {
    let sub: Vec<Stroke> = sel.iter().map(|&i| strokes[i].clone()).collect();
    let reading = crate::boardread::read(&sub, g, None);
    let hh = reading.char_h;
    let hs = hh.clamp(0.012, 0.035);
    let mut out: Vec<Ln> = Vec::new();
    // i pezzi staccati di una riga (il denominatore di una frazione, la riga
    // sotto dentro una graffa): stanno con la riga che li contiene
    let mut groups: Vec<Vec<usize>> = reading.lines.iter().map(|l| l.group.strokes.iter().map(|&j| sel[j]).filter(|&i| !ph[i].is_empty()).collect()).collect();
    let gbox = |v: &[usize]| v.iter().map(|&i| bbox(&ph[i])).fold((f32::MAX, f32::MIN, f32::MAX, f32::MIN), |a, b| (a.0.min(b.0), a.1.max(b.1), a.2.min(b.2), a.3.max(b.3)));
    loop {
        let boxes: Vec<(f32, f32, f32, f32)> = groups.iter().map(|g| gbox(g)).collect();
        let mut merge = None;
        'find: for (a, l) in boxes.iter().enumerate() {
            if groups[a].is_empty() {
                continue;
            }
            let (cx, w) = ((l.0 + l.1) / 2.0, l.1 - l.0);
            for (b, m) in boxes.iter().enumerate() {
                if a == b || groups[b].is_empty() || m.1 - m.0 < 2.0 * w {
                    continue;
                }
                let inside = l.2 >= m.2 && l.3 <= m.3 && cx > m.0 && cx < m.1;
                // sporge sotto, ma comincia dentro (il denominatore dell'ultima riga di un sistema)
                let under = w < 4.0 * hh && cx > m.0 && cx < m.1 && l.2 > (m.2 + m.3) / 2.0 && l.2 < m.3;
                if inside || under {
                    merge = Some((a, b));
                    break 'find;
                }
            }
        }
        let Some((a, b)) = merge else { break };
        let moved = std::mem::take(&mut groups[a]);
        groups[b].extend(moved);
    }
    for idx in groups {
        if idx.is_empty() {
            continue;
        }
        // una «riga» fatta solo di forme o di trattini è un disegno
        if drawing_line_by(&idx, ph, hs, |_| true) {
            continue;
        }
        let all = idx.iter().map(|&i| bbox(&ph[i])).fold((f32::MAX, f32::MIN, f32::MAX, f32::MIN), |a, b| (a.0.min(b.0), a.1.max(b.1), a.2.min(b.2), a.3.max(b.3)));
        // uno schizzo: alto, fatto soprattutto di tratti lunghi
        let len = |i: usize| ph[i].windows(2).map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1)).sum::<f32>();
        let total: f32 = idx.iter().map(|&i| len(i)).sum();
        let long: f32 = idx
            .iter()
            .filter(|&&i| {
                let b = bbox(&ph[i]);
                (b.1 - b.0).max(b.3 - b.2) >= 2.5 * hh
            })
            .map(|&i| len(i))
            .sum();
        if all.3 - all.2 > 4.0 * hh && long > 0.5 * total {
            continue;
        }
        let letters = letters_by(&idx, ph, hh, |_| true);
        if letters.is_empty() {
            continue;
        }
        let bullet = bullet(&letters, ph, hh);
        let text: Vec<usize> = letters.iter().copied().filter(|&i| Some(i) != bullet).collect();
        let body: Vec<usize> = text.iter().copied().filter(|&i| bbox(&ph[i]).3 - bbox(&ph[i]).2 < 2.4 * hh).collect();
        let body = if body.is_empty() { text.clone() } else { body };
        // senza una riga di base (una macchia, un simbolo alto) non si sposta
        let Some((base_now, _)) = base_and_xheight(&body, ph, hh) else { continue };
        // quanto pende: la mediana delle pendenze fra i fondi delle lettere
        let mut bottoms: Vec<(f32, f32)> = body.iter().flat_map(|&i| turns_xy(&ph[i], 0.12 * hh).0).collect();
        if bottoms.len() > 200 {
            let step = bottoms.len() as f32 / 200.0;
            bottoms = (0..200).map(|k| bottoms[(k as f32 * step) as usize]).collect();
        }
        let mut slopes = Vec::new();
        for a in 0..bottoms.len() {
            for b in a + 1..bottoms.len() {
                let dx = bottoms[b].0 - bottoms[a].0;
                if dx.abs() > hh {
                    slopes.push((bottoms[b].1 - bottoms[a].1) / dx);
                }
            }
        }
        let mut theta = if slopes.len() >= 3 { quantile(&mut slopes, 0.5).atan() } else { 0.0 };
        theta = theta.clamp(-15f32.to_radians(), 15f32.to_radians());
        if theta.abs() < 0.5f32.to_radians() {
            theta = 0.0;
        }
        let x0 = text.iter().flat_map(|&i| ph[i].iter().map(|p| p.0)).fold(f32::MAX, f32::min);
        let mut ln = Ln {
            strokes: idx.clone(),
            bullet,
            anchor: (x0, base_now),
            cos: theta.cos(),
            sin: theta.sin(),
            base: 0.0,
            xh: 0.0,
            up: 0.0,
            down: 0.0,
            tx0: 0.0,
            bx0: 0.0,
            x1: 0.0,
            gh: 0.0,
            multi: false,
            base_now,
            old: all,
        };
        // le misure sulla riga raddrizzata
        let rph: Vec<Vec<(f32, f32)>> = body.iter().map(|&i| ph[i].iter().map(|&p| ln.rot(p)).collect()).collect();
        let (base, xh) = base_and_xheight(&(0..rph.len()).collect::<Vec<_>>(), &rph, hh).unwrap_or((0.0, 0.6 * hh));
        let mut ys: Vec<f32> = idx.iter().flat_map(|&i| ph[i].iter().map(|&p| ln.rot(p).1)).collect();
        let (top, bottom) = (quantile(&mut ys, 0.01), quantile(&mut ys, 0.99));
        let (tx0, x1, bx0) = {
            let l = &ln;
            let xs_of = |set: &[usize]| set.iter().flat_map(|&i| ph[i].iter().map(|&p| l.rot(p).0)).fold((f32::MAX, f32::MIN), |a, x| (a.0.min(x), a.1.max(x)));
            let tx0 = xs_of(&text).0;
            (tx0, xs_of(&idx).1, bullet.map_or(tx0, |b| xs_of(&[b]).0))
        };
        ln.base = base;
        ln.xh = xh.max(0.2 * hh);
        ln.up = (base - top).max(0.5 * hh);
        ln.down = (bottom - base).max(0.0);
        ln.tx0 = tx0;
        ln.bx0 = bx0;
        ln.x1 = x1;
        let mut hs: Vec<f32> = body.iter().map(|&i| bbox(&ph[i])).map(|b| b.3 - b.2).filter(|&h| h >= 0.4 * hh).collect();
        ln.gh = if hs.is_empty() { ln.up } else { quantile(&mut hs, 0.5) };
        ln.multi = ln.up + ln.down > 3.5 * ln.gh && ln.up + ln.down > 2.0 * hh;
        out.push(ln);
    }
    // in ordine di lettura, dall'alto
    out.sort_by(|a, b| a.base_now.total_cmp(&b.base_now));
    (out, hh)
}

/// I tratti della scelta: tutta la pagina, o nella zona le righe intere (una
/// riga tagliata dal riquadro si prende tutta, se metà dei suoi punti ci
/// stanno) e gli altri tratti col centro dentro.
fn selection(strokes: &[Stroke], g: &PageGeom, zone: Option<[f32; 4]>) -> Vec<usize> {
    let Some(z) = zone else { return (0..strokes.len()).filter(|&i| !strokes[i].points.is_empty()).collect() };
    let reading = crate::boardread::read(strokes, g, None);
    let inside = |i: usize| -> (usize, usize) {
        let pts = &strokes[i].points;
        let n = pts
            .iter()
            .filter(|p| {
                let (u, v) = g.unit(p.x, p.y);
                u >= z[0] && u <= z[2] && v >= z[1] && v <= z[3]
            })
            .count();
        (n, pts.len())
    };
    let mut taken = vec![false; strokes.len()];
    let mut in_line = vec![false; strokes.len()];
    for l in &reading.lines {
        let (n, t) = l.group.strokes.iter().map(|&i| inside(i)).fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1));
        for &i in &l.group.strokes {
            in_line[i] = true;
            if t > 0 && 2 * n >= t {
                taken[i] = true;
            }
        }
    }
    let centred = crate::inbella::select(strokes, g, z);
    for i in centred {
        if !in_line[i] {
            taken[i] = true;
        }
    }
    (0..strokes.len()).filter(|&i| taken[i]).collect()
}

/// Ogni quanti quadretti una riga, perché la scrittura ci stia (1, 2 o 3).
/// `None` se la carta non è a quadretti.
pub fn suggest_every(strokes: &[Stroke], g: &PageGeom, zone: Option<[f32; 4]>, paper: Paper) -> Option<u32> {
    let Rows::Grid { cell } = rigatura::writing_rows(paper, g.w / 1000.0, g.h / 1000.0) else { return None };
    let ph = physical(strokes, g);
    let (lines, _) = measure(strokes, g, &ph, &selection(strokes, g, zone));
    Some(every_for(&lines, cell, 1000.0 / g.h))
}

fn every_for(lines: &[Ln], cell: f32, kcm: f32) -> u32 {
    let mut ups: Vec<f32> = lines.iter().map(|l| l.up / kcm).collect();
    if ups.is_empty() {
        return 2;
    }
    let need = quantile(&mut ups, 0.75);
    (1..=3).find(|&e| 0.88 * e as f32 * cell >= need).unwrap_or(3)
}

/// Dispone sulle righe la scrittura della zona `zone` (u0, v0, u1, v1; `None`
/// = tutta la pagina).
pub fn arrange(strokes: &[Stroke], g: &PageGeom, zone: Option<[f32; 4]>, sheet: Sheet) -> Result<Arranged, Problem> {
    let sel = selection(strokes, g, zone);
    arrange_sel(strokes, g, &sel, zone, sheet)
}

/// Come `arrange`, solo per i tratti `movable` (la bella copia appena fatta).
pub fn arrange_only(strokes: &[Stroke], g: &PageGeom, movable: &[usize], zone: Option<[f32; 4]>, sheet: Sheet) -> Result<Arranged, Problem> {
    arrange_sel(strokes, g, movable, zone, sheet)
}

fn arrange_sel(strokes: &[Stroke], g: &PageGeom, sel: &[usize], zone: Option<[f32; 4]>, sheet: Sheet) -> Result<Arranged, Problem> {
    let (wcm, hcm) = (g.w / 1000.0, g.h / 1000.0);
    let rows_kind = rigatura::writing_rows(sheet.paper, wcm, hcm);
    if rows_kind == Rows::Unsupported {
        return Err(Problem::Unsupported);
    }
    let aspect = g.w / g.h;
    // centimetri → unità della pagina (l'altezza della pagina è 1)
    let kcm = 1000.0 / g.h;
    let ph = physical(strokes, g);
    let (mut lines, hh) = measure(strokes, g, &ph, sel);
    if lines.is_empty() {
        return Err(Problem::Nothing);
    }
    // gli evidenziatori seguono la riga su cui stanno
    for &i in sel.iter().filter(|&&i| strokes[i].highlight && !ph[i].is_empty()) {
        let b = bbox(&ph[i]);
        let c = ((b.0 + b.1) / 2.0, (b.2 + b.3) / 2.0);
        let m = 0.3 * hh;
        if let Some(l) = lines.iter_mut().find(|l| c.0 > l.old.0 - m && c.0 < l.old.1 + m && c.1 > l.old.2 - m && c.1 < l.old.3 + m) {
            l.strokes.push(i);
        }
    }
    let (rows, every, cell) = match rows_kind {
        Rows::Fixed(r) => (r, None, None),
        Rows::Grid { cell } => {
            let e = sheet.every.unwrap_or_else(|| every_for(&lines, cell, kcm)).clamp(1, 6);
            (rigatura::grid_rows(cell, e, hcm), Some(e), Some(cell))
        }
        Rows::Unsupported => unreachable!(),
    };
    if rows.is_empty() {
        return Err(Problem::NoRoom { fit: 0, total: lines.len() });
    }
    let rows: Vec<Row> = rows.into_iter().map(|r| Row { base: r.base * kcm, up: r.up * kcm, down: r.down * kcm, xh: r.xh.map(|x| x * kcm) }).collect();
    let pitch = if rows.len() > 1 { rows[1].base - rows[0].base } else { rows[0].up };
    // il bordo a sinistra e a destra
    let snap = |x: f32| match cell {
        Some(c) => ((x / kcm / c) - 1e-3).ceil() * c * kcm,
        None => x,
    };
    let red = rigatura::left_edge(sheet.paper, wcm, sheet.margin).map(|x| x * kcm);
    let left = match zone {
        Some(z) => {
            let (zl, zr) = (z[0] * aspect, z[2] * aspect);
            let l = zl + 0.1 * kcm;
            snap(red.filter(|&r| r > zl && r < zr).map_or(l, |r| r.max(l)))
        }
        None => {
            let now = lines.iter().map(|l| l.old.0).fold(f32::MAX, f32::min);
            snap(red.unwrap_or(now).max(kcm))
        }
    };
    // a destra fino al margine (o al bordo del foglio), anche oltre il riquadro:
    // quello che c'è accanto è un ostacolo e la riga non ci finisce sopra
    let page_right = rigatura::right_edge(sheet.paper, wcm, sheet.margin).unwrap_or(wcm - 0.5) * kcm;
    let right = match zone {
        Some(z) => (z[2] * aspect).max(page_right),
        None => page_right,
    };
    // il testo dopo i trattini: tutti nella stessa colonna
    let gap = {
        let mut v: Vec<f32> = lines.iter().filter(|l| l.bullet.is_some()).map(|l| l.tx0 - l.bx0).collect();
        (!v.is_empty()).then(|| quantile(&mut v, 0.5))
    };
    // ostacoli: quello che resta fermo (non gli evidenziatori)
    let moving: std::collections::HashSet<usize> = lines.iter().flat_map(|l| l.strokes.iter().copied()).collect();
    let obstacles: Vec<(usize, (f32, f32, f32, f32))> =
        (0..strokes.len()).filter(|i| !moving.contains(i) && !strokes[*i].highlight && !ph[*i].is_empty()).map(|i| (i, bbox(&ph[i]))).collect();
    let hits = |b: (f32, f32, f32, f32)| {
        obstacles.iter().any(|&(i, o)| {
            if o.1 < b.0 || o.0 > b.1 || o.3 < b.2 || o.2 > b.3 {
                return false;
            }
            // un tratto lungo (una freccia, un asse) blocca solo dove passa davvero
            if o.3 - o.2 < 4.0 * pitch {
                return true;
            }
            ph[i].iter().any(|p| p.0 >= b.0 && p.0 <= b.1 && p.1 >= b.2 && p.1 <= b.3)
        })
    };
    // la scala di ogni riga (solo più piccola)
    let scale = |l: &Ln, row: &Row| -> f32 {
        let mut s = (row.up / l.up).min(1.0);
        if l.down > 0.05 * row.up {
            s = s.min(1.3 * row.down / l.down);
        }
        if let Some(xh) = row.xh {
            s = s.min(1.15 * xh / l.xh);
        }
        s
    };
    let first_top = zone.map_or(0.0, |z| z[1]);
    let start_row = {
        let b = lines[0].base_now;
        (0..rows.len()).filter(|&j| rows[j].base - rows[j].up >= first_top - 0.3 * pitch).min_by(|&a, &b2| (rows[a].base - b).abs().total_cmp(&(rows[b2].base - b).abs())).unwrap_or(0)
    };
    // (riga del foglio dove va la base, scala, x del trattino, x del testo)
    let mut best_fit = 0;
    for f in [1.0f32, 0.9, 0.8, 0.7] {
        let mut placed: Vec<(f32, f32, f32, f32)> = Vec::new();
        let mut j = start_row;
        // dove finisce quello già messo (un sistema non ci va sopra)
        let mut prev_bottom = f32::MIN;
        for l in &lines {
            let lead = match (l.bullet, gap) {
                (Some(_), Some(g)) => g,
                _ => 0.0,
            };
            let w = lead + (l.x1 - l.tx0);
            let mut ok = None;
            let mut jj = j;
            while jj < rows.len() {
                let row = rows[jj];
                // la scala: una riga normale sta nello spazio della riga; un
                // sistema tiene le sue lettere grandi come una riga normale
                let mut s = if l.multi { (row.up / l.gh).min(1.0) } else { scale(l, &row) } * f;
                // non oltre il bordo destro
                if left + w * s > right && w > 0.0 {
                    s = s.min((right - left) / w);
                }
                let (bx, tx) = (left, left + lead * s);
                // la riga di base: sulla riga del foglio; un sistema comincia
                // in cima alla riga
                let base = if l.multi { row.base - row.up + l.up * s } else { row.base };
                let b = if l.multi {
                    (bx - 0.1 * hh, tx + (l.x1 - l.tx0) * s + 0.1 * hh, base - l.up * s, base + l.down * s)
                } else {
                    (bx - 0.1 * hh, tx + (l.x1 - l.tx0) * s + 0.1 * hh, base - 0.85 * l.up * s, base + 0.9 * l.down * s)
                };
                if base + l.down * s <= 1.0 && !hits(b) && (!l.multi || b.2 >= prev_bottom) {
                    // la prossima riga libera: la prima sotto quello che occupa
                    let bottom = base + if l.multi { l.down * s } else { 0.0 };
                    let next = if l.multi { (jj + 1..rows.len()).find(|&r| rows[r].base - 0.6 * rows[r].up >= bottom).unwrap_or(rows.len()) } else { jj + 1 };
                    ok = Some(((base, s, bx, tx), next));
                    break;
                }
                jj += 1;
            }
            match ok {
                Some((p, next)) => {
                    prev_bottom = p.0 + l.down * p.1;
                    placed.push(p);
                    j = next;
                }
                None => break,
            }
        }
        best_fit = best_fit.max(placed.len());
        if placed.len() == lines.len() {
            return Ok(apply(strokes, g, &ph, &lines, &placed, every, aspect));
        }
    }
    Err(Problem::NoRoom { fit: best_fit, total: lines.len() })
}

fn apply(strokes: &[Stroke], g: &PageGeom, ph: &[Vec<(f32, f32)>], lines: &[Ln], placed: &[(f32, f32, f32, f32)], every: Option<u32>, aspect: f32) -> Arranged {
    let mut out = strokes.to_vec();
    let mut moved = Vec::new();
    let mut shrunk = 0;
    let mut area = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
    let mut grow = |x: f32, y: f32| {
        area = [area[0].min(x / aspect), area[1].min(y), area[2].max(x / aspect), area[3].max(y)];
    };
    for (l, &(base, s, bx, tx)) in lines.iter().zip(placed) {
        if s < 0.99 {
            shrunk += 1;
        }
        grow(l.old.0, l.old.2);
        grow(l.old.1, l.old.3);
        for &i in &l.strokes {
            let (from, to) = if Some(i) == l.bullet { (l.bx0, bx) } else { (l.tx0, tx) };
            let st = &mut out[i];
            for (p, &q) in st.points.iter_mut().zip(&ph[i]) {
                let r = l.rot(q);
                let (x, y) = (to + s * (r.0 - from), base + s * (r.1 - l.base));
                grow(x, y);
                let (px, py) = g.pad(x / aspect, y);
                p.x = px;
                p.y = py;
            }
            moved.push(i);
        }
    }
    moved.sort_unstable();
    Arranged { strokes: out, moved, lines: lines.len(), shrunk, every, area: [area[0].max(0.0), area[1].max(0.0), area[2].min(1.0), area[3].min(1.0)] }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::View;
    use crate::stroke::{PenEvent, PenPhase};

    fn geom() -> PageGeom {
        PageGeom::new(View::default(), 21000.0, 29700.0)
    }

    /// Una riga di «lettere» (archetti alti `h`) che parte da `x` sulla base
    /// `y`, e sale di `rise` ogni 1000 unità.
    fn word_line(x: f32, y: f32, n: usize, h: f32, rise: f32) -> Vec<Stroke> {
        (0..n)
            .map(|k| {
                let mut s = Stroke::new([20, 20, 20], 2.0);
                for t in 0..20 {
                    let f = t as f32 / 19.0;
                    let px = x + k as f32 * 1.3 * h + f * h;
                    s.points.push(PenEvent { x: px, y: y - (px - x) * rise / 1000.0 - (f * 3.14).sin() * h, pressure: 0.7, phase: PenPhase::Move });
                }
                s
            })
            .collect()
    }

    fn sheet(paper: Paper) -> Sheet {
        Sheet { paper, margin: true, every: None }
    }

    /// Le basi (fondo degli archetti) delle righe nuove, in cm dall'alto.
    fn bases(out: &[Stroke], idx: &[std::ops::Range<usize>]) -> Vec<f32> {
        idx.iter().map(|r| out[r.clone()].iter().flat_map(|s| s.points.iter().map(|p| p.y)).fold(f32::MIN, f32::max) / 1000.0).collect()
    }

    fn lefts(out: &[Stroke], idx: &[std::ops::Range<usize>]) -> Vec<f32> {
        idx.iter().map(|r| out[r.clone()].iter().flat_map(|s| s.points.iter().map(|p| p.x)).fold(f32::MAX, f32::min) / 1000.0).collect()
    }

    #[test]
    fn righe_dritte_in_fila_e_a_sinistra() {
        let g = geom();
        // tre righe: una storta, una spostata a destra, una dopo un vuoto grande
        let mut strokes = word_line(4000.0, 5300.0, 6, 450.0, 60.0);
        strokes.extend(word_line(5200.0, 6400.0, 5, 450.0, 0.0));
        strokes.extend(word_line(3600.0, 11500.0, 4, 450.0, 0.0));
        let idx = [0..6, 6..11, 11..15];
        let a = arrange(&strokes, &g, None, sheet(Paper::Righe)).unwrap();
        assert_eq!(a.lines, 3);
        assert_eq!(a.shrunk, 0);
        let b = bases(&a.strokes, &idx);
        // sulle righe da 8 mm (2,0 + k·0,8), una dopo l'altra
        for y in &b {
            let k = (y - 2.0) / 0.8;
            assert!((k - k.round()).abs() < 0.06, "{b:?}");
        }
        assert!((b[1] - b[0] - 0.8).abs() < 0.05 && (b[2] - b[1] - 0.8).abs() < 0.05, "{b:?}");
        // tutte dal margine rosso
        let l = lefts(&a.strokes, &idx);
        assert!(l.iter().all(|x| (x - 2.7).abs() < 0.05), "{l:?}");
        // la riga storta ora è dritta: i fondi degli archetti alla stessa altezza
        let bottoms: Vec<f32> = a.strokes[0..6].iter().map(|s| s.points.iter().map(|p| p.y).fold(f32::MIN, f32::max)).collect();
        let spread = bottoms.iter().fold(f32::MIN, |m, &y| m.max(y)) - bottoms.iter().fold(f32::MAX, |m, &y| m.min(y));
        assert!(spread < 25.0, "{bottoms:?}");
        // rifarlo non cambia niente
        let again = arrange(&a.strokes, &g, None, sheet(Paper::Righe)).unwrap();
        for (p, q) in a.strokes.iter().zip(&again.strokes) {
            for (u, v) in p.points.iter().zip(&q.points) {
                assert!((u.x - v.x).abs() < 20.0 && (u.y - v.y).abs() < 20.0);
            }
        }
    }

    #[test]
    fn grande_si_rimpicciolisce_piccola_no() {
        let g = geom();
        // 1,2 cm su righe da 8 mm: troppo alta; 3 mm: resta com'è
        let mut strokes = word_line(4000.0, 6000.0, 4, 1200.0, 0.0);
        strokes.extend(word_line(4000.0, 8000.0, 4, 300.0, 0.0));
        let a = arrange(&strokes, &g, None, sheet(Paper::Righe)).unwrap();
        assert_eq!(a.shrunk, 1);
        let h = |r: std::ops::Range<usize>| {
            let ys: Vec<f32> = a.strokes[r].iter().flat_map(|s| s.points.iter().map(|p| p.y)).collect();
            ys.iter().fold(f32::MIN, |m, &y| m.max(y)) - ys.iter().fold(f32::MAX, |m, &y| m.min(y))
        };
        assert!(h(0..4) < 0.8 * 1000.0, "{}", h(0..4));
        assert!((h(4..8) - 300.0).abs() < 15.0, "{}", h(4..8));
    }

    #[test]
    fn quadretti_ogni_n() {
        let g = geom();
        let small = word_line(4000.0, 6000.0, 5, 700.0, 0.0);
        assert_eq!(suggest_every(&small, &g, None, Paper::Quadretti5), Some(2));
        let big = word_line(4000.0, 6000.0, 5, 1200.0, 0.0);
        assert_eq!(suggest_every(&big, &g, None, Paper::Quadretti5), Some(3));
        assert_eq!(suggest_every(&big, &g, None, Paper::Righe), None);
        // ogni 2 quadretti: le basi sui centimetri interi; il bordo su una linea
        let mut strokes = small.clone();
        strokes.extend(word_line(4300.0, 8000.0, 5, 700.0, 0.0));
        let a = arrange(&strokes, &g, None, sheet(Paper::Quadretti5)).unwrap();
        assert_eq!(a.every, Some(2));
        let b = bases(&a.strokes, &[0..5, 5..10]);
        assert!(b.iter().all(|y| (y - y.round()).abs() < 0.06) && (b[1] - b[0] - 1.0).abs() < 0.06, "{b:?}");
        let l = lefts(&a.strokes, &[0..5, 5..10]);
        assert!(l.iter().all(|x| ((x / 0.5) - (x / 0.5).round()).abs() < 0.1), "{l:?}");
        // ogni quadretto: la scrittura si stringe in mezzo centimetro
        let a = arrange(&strokes, &g, None, Sheet { every: Some(1), ..sheet(Paper::Quadretti5) }).unwrap();
        assert_eq!(a.shrunk, 2);
    }

    #[test]
    fn i_disegni_restano() {
        let g = geom();
        let mut strokes = word_line(4000.0, 6000.0, 5, 450.0, 0.0);
        // un rettangolo grande e una diagonale lunga, sotto la scrittura
        let rect = [(3000.0, 9000.0), (15000.0, 9000.0), (15000.0, 14000.0), (3000.0, 14000.0), (3000.0, 9000.0)];
        let mut r = Stroke::new([20, 20, 20], 2.0);
        for w in rect.windows(2) {
            for t in 0..20 {
                let f = t as f32 / 20.0;
                r.points.push(PenEvent { x: w[0].0 + (w[1].0 - w[0].0) * f, y: w[0].1 + (w[1].1 - w[0].1) * f, pressure: 0.7, phase: PenPhase::Move });
            }
        }
        strokes.push(r.clone());
        strokes.extend(word_line(4000.0, 7800.0, 5, 450.0, 0.0));
        let a = arrange(&strokes, &g, None, sheet(Paper::Righe)).unwrap();
        assert_eq!(a.lines, 2);
        assert!(!a.moved.contains(&5));
        assert!(a.strokes[5].points.iter().zip(&r.points).all(|(p, q)| p.x == q.x && p.y == q.y));
    }

    #[test]
    fn l_accento_resta_sulla_lettera() {
        let g = geom();
        let mut strokes = word_line(4000.0, 6100.0, 5, 450.0, 0.0);
        let mut dot = Stroke::new([20, 20, 20], 2.0);
        dot.points.push(PenEvent { x: 4200.0, y: 5450.0, pressure: 0.7, phase: PenPhase::Move });
        dot.points.push(PenEvent { x: 4230.0, y: 5420.0, pressure: 0.7, phase: PenPhase::Move });
        strokes.push(dot);
        let a = arrange(&strokes, &g, None, sheet(Paper::Righe)).unwrap();
        let (l0, d) = (&a.strokes[0].points[0], &a.strokes[5].points[0]);
        assert!(((d.x - l0.x) - 200.0).abs() < 5.0 && ((d.y - l0.y) + 650.0).abs() < 5.0, "{:?} {:?}", l0, d);
    }

    #[test]
    #[cfg(not(openslate_mobile))]
    fn elenco_coi_trattini() {
        let g = geom();
        let strokes = crate::inbella::tests::list_page(&[1500.0, 1250.0, 1650.0, 1400.0], &[2300.0, 2600.0, 2200.0, 2500.0], 6000.0, 1400.0);
        let a = arrange(&strokes, &g, None, sheet(Paper::Righe)).unwrap();
        assert_eq!(a.lines, 4);
        // i trattini (il primo tratto di ogni riga) in colonna, e il testo pure
        let dash: Vec<f32> = (0..4).map(|r| a.strokes[r * 5].points[0].x).collect();
        let text: Vec<f32> = (0..4).map(|r| a.strokes[r * 5 + 1].points[0].x).collect();
        assert!(dash.iter().all(|x| (x - dash[0]).abs() < 5.0), "{dash:?}");
        assert!(text.iter().all(|x| (x - text[0]).abs() < 5.0), "{text:?}");
        assert!(text[0] > dash[0] + 400.0);
    }

    #[test]
    fn la_zona_prende_righe_intere() {
        let g = geom();
        let mut strokes = word_line(3000.0, 6000.0, 8, 450.0, 0.0);
        strokes.extend(word_line(3300.0, 7000.0, 8, 450.0, 0.0));
        // un riquadro che taglia via i primi archetti di tutte e due le righe
        let z = [4200.0 / 21000.0, 5000.0 / 29700.0, 0.9, 7300.0 / 29700.0];
        let a = arrange(&strokes, &g, Some(z), sheet(Paper::Righe)).unwrap();
        assert_eq!(a.lines, 2);
        assert_eq!(a.moved.len(), 16, "le righe intere, non i pezzi");
    }

    #[test]
    fn senza_posto_e_senza_righe() {
        let g = geom();
        let mut strokes = Vec::new();
        for k in 0..40 {
            strokes.extend(word_line(3000.0, 2000.0 + k as f32 * 650.0, 3, 450.0, 0.0));
        }
        assert!(matches!(arrange(&strokes, &g, Some([0.0, 0.5, 1.0, 1.0]), sheet(Paper::Righe)), Err(Problem::NoRoom { .. })));
        assert_eq!(arrange(&strokes, &g, None, sheet(Paper::Bianco)).unwrap_err(), Problem::Unsupported);
        assert_eq!(arrange(&strokes, &g, None, sheet(Paper::Pentagramma)).unwrap_err(), Problem::Unsupported);
        assert_eq!(arrange(&[], &g, None, sheet(Paper::Righe)).unwrap_err(), Problem::Nothing);
    }

    /// A occhio, su una pagina vera dell'archivio:
    /// `RIGHE_PAGE=<id> RIGHE_PAPER=righe RIGHE_PNG=/tmp/righe cargo test righe_pagina_vera -- --ignored`
    /// (`RIGHE_ZONE=u0,v0,u1,v1`, `RIGHE_EVERY=2`); scrive `-prima.png` e `-dopo.png`.
    #[test]
    #[ignore]
    #[cfg(not(openslate_mobile))]
    fn righe_pagina_vera() {
        let (Ok(id), Ok(out)) = (std::env::var("RIGHE_PAGE"), std::env::var("RIGHE_PNG")) else { return };
        let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap()).join("Library/Application Support/OpenSlate/library");
        let lib = crate::library::Library::open_in(&dir);
        let d = lib.get(&id).expect("pagina").drawing.clone();
        let paper = Paper::from_code(&std::env::var("RIGHE_PAPER").unwrap_or("righe".into()));
        let every = std::env::var("RIGHE_EVERY").ok().and_then(|e| e.parse().ok());
        let zone = std::env::var("RIGHE_ZONE").ok().map(|z| {
            let v: Vec<f32> = z.split(',').map(|x| x.parse().unwrap()).collect();
            [v[0], v[1], v[2], v[3]]
        });
        let (pw, ph) = crate::scene::page_dims(&d);
        let view = View { rot: std::env::var("RIGHE_ROT").ok().and_then(|r| r.parse().ok()).unwrap_or(0), ..View::default() };
        let g = PageGeom::new(view, pw, ph);
        let a = match arrange(&d.strokes, &g, zone, Sheet { paper, margin: true, every }) {
            Ok(a) => a,
            Err(e) => panic!("{e:?}"),
        };
        eprintln!("{} righe, {} rimpicciolite, ogni {:?}", a.lines, a.shrunk, a.every);
        // la rigatura come tratti grigi chiari (la PNG della pagina non la disegna)
        let paper_strokes: Vec<Stroke> = rigatura::layout(paper, g.w / 1000.0, g.h / 1000.0, true)
            .into_iter()
            .filter_map(|m| match m {
                rigatura::Mark::Line(ink, a, b) => {
                    let mut s = Stroke::new(if ink == rigatura::Ink::Red { [240, 150, 150] } else { [170, 200, 235] }, 0.6);
                    for (x, y) in [a, b] {
                        let (px, py) = g.pad(x * 1000.0 / g.w, y * 1000.0 / g.h);
                        s.points.push(PenEvent { x: px, y: py, pressure: 0.5, phase: PenPhase::Move });
                    }
                    Some(s)
                }
                _ => None,
            })
            .collect();
        for (name, strokes) in [("prima", &d.strokes), ("dopo", &a.strokes)] {
            let mut all = paper_strokes.clone();
            all.extend(strokes.iter().cloned());
            let page = crate::scene::Page::from_drawing(&crate::stroke::Drawing { strokes: all, width: d.width, height: d.height }, view);
            std::fs::write(format!("{out}-{name}.png"), crate::export::page_to_png(&page, 1600).unwrap()).unwrap();
        }
    }

    #[test]
    fn pagina_girata() {
        let g = PageGeom::new(View { rot: 1, ..View::default() }, 21000.0, 29700.0);
        // sul foglio girato la scrittura corre lungo y: si dispone lo stesso
        let strokes: Vec<Stroke> = word_line(4000.0, 6000.0, 5, 450.0, 0.0)
            .into_iter()
            .map(|mut s| {
                for p in &mut s.points {
                    let (u, v) = (p.x / 29700.0, p.y / 21000.0);
                    let (x, y) = g.pad(u, v);
                    p.x = x;
                    p.y = y;
                }
                s
            })
            .collect();
        let a = arrange(&strokes, &g, None, sheet(Paper::Righe)).unwrap();
        assert_eq!(a.lines, 1);
    }
}
