//! Le forme dei disegni rifatte perfette (per «Trasforma in bella», senza
//! AI): segmenti e rette, spezzate e poligoni, frecce, cerchi ed ellissi,
//! archi; le curve libere si lisciano. Quello che non si riconosce con
//! sicurezza resta com'è.
//!
//! I conti sono nelle unità della pagina con x e y nella stessa scala
//! (y verso il basso); `hh` è l'altezza tipica di una lettera.
//!
//! Le linee dritte (segmenti, lati di spezzate e poligoni) condividono i
//! vertici: i capi vicini di tratti diversi si uniscono (il triangolo fatto
//! in tre tratti si chiude), le linee quasi orizzontali o verticali si
//! raddrizzano tenendo uniti i vertici, e un capo che arriva vicino a
//! un'altra linea si appoggia su di lei.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

type P = (f32, f32);

/// Che forma è diventato un tratto (per dirlo, e per i test).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Line,
    Polyline,
    Polygon,
    Arrow,
    Ellipse,
    Arc,
    Curve,
    /// Un trattino di una linea tratteggiata.
    Dash,
}

/// Un tratto rifatto: i punti nuovi e la forma.
pub struct Shaped {
    pub index: usize,
    pub kind: Kind,
    pub points: Vec<P>,
}

/// Una catena di linee dritte: i vertici (indici nella lista comune).
struct Chain {
    index: usize,
    verts: Vec<usize>,
    closed: bool,
}

/// Rifà perfetti i tratti `cand` (indici in `strokes`, punti già nelle
/// unità della pagina). Quello che non è una forma riconosciuta, se è
/// grande, si liscia; se è piccolo resta com'è.
pub fn perfect(strokes: &[Vec<P>], cand: &[usize], hh: f32) -> Vec<Shaped> {
    let hh = hh.max(1e-4);
    let mut out: Vec<Shaped> = Vec::new();
    let mut verts: Vec<P> = Vec::new();
    let mut chains: Vec<Chain> = Vec::new();
    // segmenti corti (forse trattini di una linea tratteggiata)
    let mut dashes: Vec<(usize, P, P)> = Vec::new();
    // tratti aperti e curvi (archi, metà di ellisse, curve)
    let mut opens: Vec<(usize, Vec<P>)> = Vec::new();
    // tratti piccoli (forse punte di freccia disegnate a parte)
    let mut small: Vec<usize> = Vec::new();
    for &i in cand {
        let raw = &strokes[i];
        if raw.len() < 3 || diag(raw) < 0.3 * hh {
            continue;
        }
        // i tratti grandi si guardano senza il tremolio della mano
        let calm;
        let p: &[P] = if diag(raw) >= 2.0 * hh {
            calm = denoise(raw, hh);
            &calm
        } else {
            raw
        };
        if let Some((a, b)) = straight_hh(p, hh) {
            if dist(a, b) < 3.0 * hh {
                dashes.push((i, a, b));
            } else {
                let v0 = push(&mut verts, a);
                let v1 = push(&mut verts, b);
                chains.push(Chain { index: i, verts: vec![v0, v1], closed: false });
            }
            continue;
        }
        if diag(raw) < 0.6 * hh {
            continue;
        }
        if let Some(pts) = arrow(p, hh) {
            out.push(Shaped { index: i, kind: Kind::Arrow, points: pts });
            continue;
        }
        if let Some(pts) = curved_arrow(p, hh, true) {
            out.push(Shaped { index: i, kind: Kind::Arrow, points: pts });
            continue;
        }
        if let Some((corners, closed)) = polyline(p, hh) {
            // una «V» piccola: forse la punta di una freccia (si vede dopo)
            if !closed && corners.len() == 3 && diag(p) <= 1.8 * hh {
                small.push(i);
                continue;
            }
            let vs: Vec<usize> = corners.into_iter().map(|q| push(&mut verts, q)).collect();
            chains.push(Chain { index: i, verts: vs, closed });
            continue;
        }
        if let Some(e) = ellipse_fit(p) {
            out.push(Shaped { index: i, kind: Kind::Ellipse, points: e.full(p[0]) });
            continue;
        }
        if let Some(pts) = curved_arrow(p, hh, false) {
            out.push(Shaped { index: i, kind: Kind::Arrow, points: pts });
            continue;
        }
        if diag(raw) <= 1.8 * hh {
            small.push(i);
            continue;
        }
        opens.push((i, p.to_vec()));
    }
    // le linee tratteggiate; i trattini rimasti soli sono segmenti
    for (i, a, b) in dashed(&dashes, hh, &mut out) {
        if dist(a, b) >= 0.8 * hh {
            let v0 = push(&mut verts, a);
            let v1 = push(&mut verts, b);
            chains.push(Chain { index: i, verts: vec![v0, v1], closed: false });
        }
    }
    // i poligoni quasi regolari diventano regolari (quadrati, rombi, …)
    let mut fixed = vec![false; verts.len()];
    for c in chains.iter().filter(|c| c.closed) {
        if regularize(&mut verts, &c.verts) {
            for &v in &c.verts {
                fixed[v] = true;
            }
        }
    }
    join_and_snap(&mut verts, &chains, &fixed, hh);
    // le ellissi fatte in due metà
    let mut used = vec![false; opens.len()];
    for a in 0..opens.len() {
        for b in a + 1..opens.len() {
            if used[a] || used[b] {
                continue;
            }
            if let Some((pa, pb)) = ellipse_halves(&opens[a].1, &opens[b].1, hh) {
                out.push(Shaped { index: opens[a].0, kind: Kind::Ellipse, points: pa });
                out.push(Shaped { index: opens[b].0, kind: Kind::Ellipse, points: pb });
                used[a] = true;
                used[b] = true;
            }
        }
    }
    for (k, (i, p)) in opens.iter().enumerate() {
        if used[k] {
            continue;
        }
        if let Some(pts) = best_arc(p) {
            out.push(Shaped { index: *i, kind: Kind::Arc, points: pts });
        } else {
            out.push(Shaped { index: *i, kind: Kind::Curve, points: smooth_curve(&strokes[*i], hh) });
        }
    }
    // le punte disegnate a parte: una «V» piccola vicino al capo di una
    // linea, di un arco o di una curva
    let mut ends: Vec<(P, P)> = chains
        .iter()
        .filter(|c| !c.closed)
        .flat_map(|c| {
            let n = c.verts.len();
            [(verts[c.verts[1]], verts[c.verts[0]]), (verts[c.verts[n - 2]], verts[c.verts[n - 1]])]
        })
        .collect();
    for s in out.iter().filter(|s| matches!(s.kind, Kind::Arc | Kind::Curve)) {
        let p = &s.points;
        ends.push((back_along(p, hh, false), p[0]));
        ends.push((back_along(p, hh, true), p[p.len() - 1]));
    }
    for i in small {
        if let Some(pts) = arrow_head(&strokes[i], &ends, hh) {
            out.push(Shaped { index: i, kind: Kind::Arrow, points: pts });
        }
    }
    for c in chains {
        let mut pts: Vec<P> = c.verts.iter().map(|&v| verts[v]).collect();
        if c.closed {
            pts.push(pts[0]);
        }
        let kind = if c.closed {
            Kind::Polygon
        } else if pts.len() == 2 {
            Kind::Line
        } else {
            Kind::Polyline
        };
        out.push(Shaped { index: c.index, kind, points: densify(&pts, 0.25 * hh) });
    }
    out.sort_by_key(|s| s.index);
    out
}

/// Un punto della curva a circa una lettera da un capo (per la direzione
/// dell'ultimo tratto): dall'inizio, o dalla fine con `from_end`.
fn back_along(p: &[P], hh: f32, from_end: bool) -> P {
    let it: Box<dyn Iterator<Item = &P>> = if from_end { Box::new(p.iter().rev()) } else { Box::new(p.iter()) };
    let mut it = it.peekable();
    let start = **it.peek().unwrap();
    let mut last = start;
    for &q in it {
        last = q;
        if dist(q, start) >= hh {
            break;
        }
    }
    last
}

/// Un trattino (forse di una linea tratteggiata): corto e dritto.
pub fn is_dash(p: &[P], hh: f32) -> bool {
    let hh = hh.max(1e-4);
    p.len() >= 2 && straight_hh(p, hh).is_some_and(|(a, b)| dist(a, b) >= 0.3 * hh && dist(a, b) < 3.0 * hh)
}

/// Un tratto grande che è chiaramente una forma (linea, freccia, spezzata,
/// poligono, cerchio, arco, ellisse): un disegno, non scrittura da far leggere.
pub fn is_shape(p: &[P], hh: f32) -> bool {
    let hh = hh.max(1e-4);
    if p.len() < 3 || diag(p) < 2.0 * hh {
        return false;
    }
    let p = &denoise(p, hh)[..];
    straight(p).is_some()
        || arrow(p, hh).is_some()
        || polyline(p, hh).is_some()
        || ellipse_fit(p).is_some()
        || arc(p).is_some()
        || elliptic_arc(p).is_some()
        || curved_arrow(p, hh, false).is_some()
}

fn push(verts: &mut Vec<P>, q: P) -> usize {
    verts.push(q);
    verts.len() - 1
}

fn root(g: &mut [usize], i: usize) -> usize {
    let mut r = i;
    while g[r] != r {
        r = g[r];
    }
    g[i] = r;
    r
}

/// I capi vicini si uniscono; le linee quasi orizzontali o verticali si
/// raddrizzano; gli angoli quasi retti diventano retti; i segmenti quasi
/// paralleli diventano paralleli; un capo vicino a un'altra linea ci si
/// appoggia. I vertici `fixed` (poligoni già regolari) non si toccano.
fn join_and_snap(verts: &mut [P], chains: &[Chain], fixed: &[bool], hh: f32) {
    let tol = 0.45 * hh;
    let nv = verts.len();
    // i capi delle catene aperte
    let ends: Vec<usize> = chains.iter().filter(|c| !c.closed).flat_map(|c| [c.verts[0], *c.verts.last().unwrap()]).collect();
    let mut group: Vec<usize> = (0..nv).collect();
    for (a, &va) in ends.iter().enumerate() {
        for &vb in &ends[a + 1..] {
            if dist(verts[va], verts[vb]) < tol {
                let (ra, rb) = (root(&mut group, va), root(&mut group, vb));
                group[ra] = rb;
            }
        }
    }
    let mut sum: Vec<(f32, f32, f32)> = vec![(0.0, 0.0, 0.0); nv];
    for &v in &ends {
        let r = root(&mut group, v);
        sum[r].0 += verts[v].0;
        sum[r].1 += verts[v].1;
        sum[r].2 += 1.0;
    }
    for &v in &ends {
        let r = root(&mut group, v);
        verts[v] = (sum[r].0 / sum[r].2, sum[r].1 / sum[r].2);
    }
    // i vertici che sono lo stesso punto (uniti), per spostarli insieme
    let members = |group: &mut Vec<usize>, v: usize| -> Vec<usize> {
        let r = root(group, v);
        (0..nv).filter(|&w| w == v || (ends.contains(&w) && root(group, w) == r)).collect()
    };
    let segs: Vec<(usize, usize)> = chains
        .iter()
        .flat_map(|c| {
            let mut s: Vec<(usize, usize)> = c.verts.windows(2).map(|w| (w[0], w[1])).collect();
            if c.closed {
                s.push((*c.verts.last().unwrap(), c.verts[0]));
            }
            s
        })
        .collect();
    let is_fixed = |group: &mut Vec<usize>, v: usize| members(group, v).iter().any(|&w| fixed[w]);
    // orizzontali e verticali
    let snap = 6f32.to_radians();
    for _ in 0..3 {
        for &(a, b) in &segs {
            if is_fixed(&mut group, a) || is_fixed(&mut group, b) {
                continue;
            }
            let (pa, pb) = (verts[a], verts[b]);
            let ang = (pb.1 - pa.1).atan2(pb.0 - pa.0);
            let near = (ang / FRAC_PI_2).round() * FRAC_PI_2;
            if (ang - near).abs() >= snap {
                continue;
            }
            let horizontal = (near / FRAC_PI_2).round() as i32 % 2 == 0;
            let mut m = members(&mut group, a);
            m.extend(members(&mut group, b));
            if horizontal {
                let y = (pa.1 + pb.1) / 2.0;
                for v in m {
                    verts[v].1 = y;
                }
            } else {
                let x = (pa.0 + pb.0) / 2.0;
                for v in m {
                    verts[v].0 = x;
                }
            }
        }
    }
    // gli angoli quasi retti: in ogni punto dove si incontrano due linee
    // (dentro una spezzata, o due tratti uniti)
    let node = |group: &mut Vec<usize>, v: usize| if ends.contains(&v) { root(group, v) } else { v };
    let mut around: std::collections::HashMap<usize, Vec<(usize, usize)>> = std::collections::HashMap::new();
    for &(a, b) in &segs {
        let (na, nb) = (node(&mut group, a), node(&mut group, b));
        around.entry(na).or_default().push((a, b));
        around.entry(nb).or_default().push((b, a));
    }
    let mut corners: Vec<(usize, usize, usize)> = around.values().filter(|v| v.len() == 2).map(|v| (v[0].0, v[0].1, v[1].1)).collect();
    corners.sort_unstable();
    let rect = 7f32.to_radians();
    for (c, a, b) in corners {
        if is_fixed(&mut group, c) {
            continue;
        }
        let (pc, pa, pb) = (verts[c], verts[a], verts[b]);
        let (la, lb) = (dist(pc, pa), dist(pc, pb));
        if la <= 0.0 || lb <= 0.0 {
            continue;
        }
        let ang_a = (pa.1 - pc.1).atan2(pa.0 - pc.0);
        let ang_b = (pb.1 - pc.1).atan2(pb.0 - pc.0);
        let between = wrap(ang_b - ang_a);
        let off = (between.abs() - FRAC_PI_2).abs();
        if off >= rect || off < 1e-4 {
            continue;
        }
        // si gira la linea più corta (che non sia bloccata)
        let (mov, keep_ang, len, sign) = if (lb <= la && !is_fixed(&mut group, b)) || is_fixed(&mut group, a) {
            (b, ang_a, lb, between.signum())
        } else {
            (a, ang_b, la, -between.signum())
        };
        if is_fixed(&mut group, mov) {
            continue;
        }
        let t = keep_ang + sign * FRAC_PI_2;
        let q = (pc.0 + len * t.cos(), pc.1 + len * t.sin());
        for v in members(&mut group, mov) {
            verts[v] = q;
        }
    }
    // i segmenti soli quasi paralleli: proprio paralleli (girati attorno al
    // loro punto di mezzo); un segmento solo si allinea anche a una linea di
    // una figura
    let lone: Vec<(usize, usize)> = chains
        .iter()
        .filter(|c| !c.closed && c.verts.len() == 2)
        .map(|c| (c.verts[0], c.verts[1]))
        .filter(|&(a, b)| members(&mut group, a).len() == 1 && members(&mut group, b).len() == 1)
        .collect();
    let par = 4f32.to_radians();
    let dir = |p: P, q: P| {
        let a = (q.1 - p.1).atan2(q.0 - p.0);
        // senza verso: fra -90° e 90°
        wrap(2.0 * a) / 2.0
    };
    let mut aligned = vec![false; lone.len()];
    for (k, &(a, b)) in lone.iter().enumerate() {
        if aligned[k] {
            continue;
        }
        let da = dir(verts[a], verts[b]);
        let near = (da / FRAC_PI_2).round() * FRAC_PI_2;
        if (da - near).abs() < 1e-4 {
            continue; // già orizzontale o verticale
        }
        let cands: Vec<(f32, Option<usize>)> = segs
            .iter()
            .filter(|&&(c, d)| !((c == a && d == b) || (c == b && d == a)))
            .map(|&(c, d)| (dir(verts[c], verts[d]), lone.iter().position(|&(x, y)| x == c && y == d)))
            .filter(|&(dc, _)| (wrap(2.0 * (dc - da)) / 2.0).abs() < par)
            .collect();
        // prima un altro segmento solo (si incontrano a metà), se no la linea di una figura
        let target = cands
            .iter()
            .find(|c| matches!(c.1, Some(j) if j > k))
            .map(|&(dc, _)| da + (wrap(2.0 * (dc - da)) / 2.0) / 2.0)
            .or_else(|| cands.iter().find(|c| c.1.is_none()).map(|c| c.0));
        if let Some(t) = target {
            let (pa, pb) = (verts[a], verts[b]);
            let m = ((pa.0 + pb.0) / 2.0, (pa.1 + pb.1) / 2.0);
            let half = dist(pa, pb) / 2.0;
            let s = if (pb.0 - pa.0) * t.cos() + (pb.1 - pa.1) * t.sin() >= 0.0 { 1.0 } else { -1.0 };
            verts[a] = (m.0 - s * half * t.cos(), m.1 - s * half * t.sin());
            verts[b] = (m.0 + s * half * t.cos(), m.1 + s * half * t.sin());
            // l'altro segmento solo va allo stesso angolo
            for (j, &(c, d)) in lone.iter().enumerate().skip(k + 1) {
                let dc = dir(verts[c], verts[d]);
                if (wrap(2.0 * (dc - da)) / 2.0).abs() < par {
                    aligned[j] = true;
                    let (pc, pd) = (verts[c], verts[d]);
                    let m = ((pc.0 + pd.0) / 2.0, (pc.1 + pd.1) / 2.0);
                    let half = dist(pc, pd) / 2.0;
                    let s = if (pd.0 - pc.0) * t.cos() + (pd.1 - pc.1) * t.sin() >= 0.0 { 1.0 } else { -1.0 };
                    verts[c] = (m.0 - s * half * t.cos(), m.1 - s * half * t.sin());
                    verts[d] = (m.0 + s * half * t.cos(), m.1 + s * half * t.sin());
                }
            }
        }
    }
    // un capo rimasto solo vicino a un'altra linea: ci si appoggia (una «T»)
    for &v in &ends {
        if members(&mut group, v).len() != 1 {
            continue;
        }
        let q = verts[v];
        let best = segs
            .iter()
            .filter(|&&(a, b)| a != v && b != v)
            .map(|&(a, b)| (seg_dist(q, verts[a], verts[b]), a, b))
            .filter(|&(d, a, b)| d < tol && dist(q, verts[a]) > tol && dist(q, verts[b]) > tol)
            .min_by(|x, y| x.0.total_cmp(&y.0));
        if let Some((_, a, b)) = best {
            verts[v] = project(q, verts[a], verts[b]);
        }
    }
}

/// Come `straight`, ma un tratto corto (un trattino) può tremare un po' di
/// più, in misura della scrittura.
fn straight_hh(p: &[P], hh: f32) -> Option<(P, P)> {
    let (a, b) = (p[0], *p.last()?);
    let len = dist(a, b);
    if len <= 0.0 {
        return None;
    }
    let path: f32 = p.windows(2).map(|w| dist(w[0], w[1])).sum();
    let dev = p.iter().map(|&q| seg_dist(q, a, b)).fold(0.0f32, f32::max);
    (dev < (0.05 * len).max(0.06 * hh) && path < 1.12 * len + 0.15 * hh).then_some((a, b))
}

/// Il tratto è una linea (quasi) dritta? Allora i suoi estremi.
fn straight(p: &[P]) -> Option<(P, P)> {
    let (a, b) = (p[0], *p.last()?);
    let len = dist(a, b);
    if len <= 0.0 {
        return None;
    }
    let path: f32 = p.windows(2).map(|w| dist(w[0], w[1])).sum();
    let dev = p.iter().map(|&q| seg_dist(q, a, b)).fold(0.0f32, f32::max);
    (dev < 0.05 * len && path < 1.12 * len).then_some((a, b))
}

/// Una spezzata di lati dritti (da 2 a 8), aperta o chiusa: i vertici.
/// Ogni lato dev'essere dritto davvero (un cerchio spezzato in pezzi non è
/// un ottagono, una parabola non è una spezzata).
fn polyline(p: &[P], hh: f32) -> Option<(Vec<P>, bool)> {
    let total: f32 = p.windows(2).map(|w| dist(w[0], w[1])).sum();
    let eps = (0.035 * total).max(0.12 * hh);
    let mut idx = rdp_idx(p, 0, p.len() - 1, eps);
    if idx.len() < 3 {
        return None;
    }
    // ogni pezzo fra due vertici è dritto
    for w in idx.windows(2) {
        let (a, b) = (p[w[0]], p[w[1]]);
        let len = dist(a, b);
        if len < 0.6 * hh {
            return None;
        }
        let part = &p[w[0]..=w[1]];
        // vicino agli angoli la mano (e la lisciatura) arrotonda: conta il mezzo del lato
        let dev = part.iter().filter(|&&q| dist(q, a).min(dist(q, b)) > 0.15 * len).map(|&q| seg_dist(q, a, b)).fold(0.0f32, f32::max);
        let path: f32 = part.windows(2).map(|w| dist(w[0], w[1])).sum();
        if dev > (0.06 * len).max(0.08 * hh) || path > 1.12 * len {
            return None;
        }
    }
    let closed = dist(p[idx[0]], p[*idx.last().unwrap()]) < (0.08 * total).max(0.4 * hh);
    let mut v: Vec<P> = idx.iter().map(|&i| p[i]).collect();
    if closed {
        v.pop();
        // l'inizio può cadere a metà di un lato: se lì il tratto va dritto non è un vertice
        if v.len() >= 4 && turn(v[v.len() - 1], v[0], v[1]) < 20f32.to_radians() {
            v.remove(0);
        }
        idx.pop();
    }
    let n_sides = if closed { v.len() } else { v.len() - 1 };
    if !(2..=8).contains(&n_sides) || (closed && v.len() < 3) {
        return None;
    }
    // gli angoli veri
    let sharp = (0..v.len())
        .filter(|&k| closed || (k > 0 && k + 1 < v.len()))
        .all(|k| turn(v[(k + v.len() - 1) % v.len()], v[k], v[(k + 1) % v.len()]) > 25f32.to_radians());
    if !sharp {
        return None;
    }
    Some((refine_corners(p, &v, closed), closed))
}

/// I vertici veri: dove si incontrano le rette dei lati (ogni retta passa
/// per il mezzo del suo lato, lontano dagli angoli arrotondati). I capi di
/// una spezzata aperta vanno sulla retta del loro lato.
fn refine_corners(p: &[P], v: &[P], closed: bool) -> Vec<P> {
    let n = v.len();
    let n_sides = if closed { n } else { n - 1 };
    let side = |k: usize| (v[k], v[(k + 1) % n]);
    // la retta di ogni lato: punto medio e direzione (componente principale)
    let mut lines: Vec<Option<(P, P)>> = Vec::with_capacity(n_sides);
    for k in 0..n_sides {
        let (a, b) = side(k);
        let len = dist(a, b);
        let pts: Vec<P> = p
            .iter()
            .copied()
            .filter(|&q| dist(q, a).min(dist(q, b)) > 0.15 * len)
            .filter(|&q| {
                let d = seg_dist(q, a, b);
                (0..n_sides).all(|j| j == k || seg_dist(q, side(j).0, side(j).1) >= d)
            })
            .collect();
        if pts.len() < 3 {
            lines.push(None);
            continue;
        }
        let m = pts.len() as f32;
        let o = pts.iter().fold((0.0, 0.0), |acc, q| (acc.0 + q.0 / m, acc.1 + q.1 / m));
        let (mut sxx, mut syy, mut sxy) = (0.0f32, 0.0f32, 0.0f32);
        for q in &pts {
            sxx += (q.0 - o.0).powi(2);
            syy += (q.1 - o.1).powi(2);
            sxy += (q.0 - o.0) * (q.1 - o.1);
        }
        let ang = 0.5 * (2.0 * sxy).atan2(sxx - syy);
        lines.push(Some((o, (ang.cos(), ang.sin()))));
    }
    let meet = |l1: (P, P), l2: (P, P)| -> Option<P> {
        let ((p1, d1), (p2, d2)) = (l1, l2);
        let den = d1.0 * d2.1 - d1.1 * d2.0;
        if den.abs() < 0.2 {
            return None; // quasi paralleli: l'incrocio è lontano e incerto
        }
        let t = ((p2.0 - p1.0) * d2.1 - (p2.1 - p1.1) * d2.0) / den;
        Some((p1.0 + t * d1.0, p1.1 + t * d1.1))
    };
    let on = |l: (P, P), q: P| {
        let (o, d) = l;
        let t = (q.0 - o.0) * d.0 + (q.1 - o.1) * d.1;
        (o.0 + t * d.0, o.1 + t * d.1)
    };
    let mut out = v.to_vec();
    for k in 0..n {
        let before = if closed { Some((k + n - 1) % n) } else if k > 0 { Some(k - 1) } else { None };
        let after = if closed || k + 1 < n { Some(k % n_sides.max(1)).filter(|&j| j < n_sides) } else { None };
        let new = match (before.and_then(|j| lines[j]), after.and_then(|j| lines[j])) {
            (Some(l1), Some(l2)) => meet(l1, l2),
            (None, Some(l)) | (Some(l), None) => Some(on(l, v[k])),
            _ => None,
        };
        // solo se resta vicino a dov'era (un lato corto non deve spostarlo lontano)
        if let Some(q) = new {
            let near = (0..n_sides).map(|j| dist(side(j).0, side(j).1)).fold(f32::MAX, f32::min);
            if dist(q, v[k]) < 0.25 * near {
                out[k] = q;
            }
        }
    }
    out
}

/// Ramer–Douglas–Peucker sugli indici (da `i` a `j` compresi).
fn rdp_idx(p: &[P], i: usize, j: usize, eps: f32) -> Vec<usize> {
    if j <= i + 1 {
        return vec![i, j];
    }
    let (a, b) = (p[i], p[j]);
    let (k, d) = (i + 1..j).map(|k| (k, seg_dist(p[k], a, b))).fold((i, 0.0f32), |m, x| if x.1 > m.1 { x } else { m });
    if d <= eps {
        return vec![i, j];
    }
    let mut left = rdp_idx(p, i, k, eps);
    left.pop();
    left.extend(rdp_idx(p, k, j, eps));
    left
}

/// Di quanto gira la strada in `b` (0 = dritto).
fn turn(a: P, b: P, c: P) -> f32 {
    let (u, w) = ((b.0 - a.0, b.1 - a.1), (c.0 - b.0, c.1 - b.1));
    let d = (u.0 * w.0 + u.1 * w.1) / ((u.0.hypot(u.1) * w.0.hypot(w.1)).max(1e-9));
    d.clamp(-1.0, 1.0).acos()
}

/// Una freccia in un tratto solo: un'asta lunga e la punta (uno o due
/// tagli corti) a un capo. La rifà: asta dritta e punta simmetrica.
fn arrow(p: &[P], hh: f32) -> Option<Vec<P>> {
    let total: f32 = p.windows(2).map(|w| dist(w[0], w[1])).sum();
    let simp = rdp(p, (0.04 * total).max(0.12 * hh));
    if simp.len() < 3 || simp.len() > 6 {
        return None;
    }
    let segs: Vec<f32> = simp.windows(2).map(|w| dist(w[0], w[1])).collect();
    let (k, &long) = segs.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1))?;
    let rest: f32 = segs.iter().sum::<f32>() - long;
    if long < 0.6 * (long + rest) || long < 1.5 * hh {
        return None;
    }
    let (tail, tip, barbs): (P, P, Vec<P>) = if k == 0 {
        (simp[0], simp[1], simp[2..].to_vec())
    } else if k == segs.len() - 1 {
        (simp[simp.len() - 1], simp[simp.len() - 2], simp[..simp.len() - 2].to_vec())
    } else {
        return None;
    };
    // la punta sta vicino alla fine dell'asta e torna indietro (i punti che
    // ripassano sulla punta non contano)
    let barbs: Vec<P> = barbs.into_iter().filter(|&q| dist(q, tip) > 0.03 * long).collect();
    if barbs.is_empty() {
        return None;
    }
    let dir = ((tip.0 - tail.0) / long, (tip.1 - tail.1) / long);
    let near = barbs.iter().all(|&q| dist(q, tip) < 0.45 * long && ((q.0 - tip.0) * dir.0 + (q.1 - tip.1) * dir.1) < 0.0);
    if !near {
        return None;
    }
    let barb = barbs.iter().map(|&q| dist(q, tip)).fold(0.0f32, f32::max).clamp(0.08 * long, 0.3 * long);
    let (tail, tip) = snap_line(tail, tip);
    let (b1, b2) = head(tail, tip, barb);
    let mut pts = densify(&[tail, tip], 0.25 * hh);
    pts.extend([b1, tip, b2]);
    Some(pts)
}

/// Quasi orizzontale o verticale (entro 6°): proprio orizzontale o
/// verticale, attorno al punto di mezzo.
fn snap_line(a: P, b: P) -> (P, P) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let ang = dy.atan2(dx);
    let len = dist(a, b);
    let m = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
    let near = (ang / FRAC_PI_2).round() * FRAC_PI_2;
    let ang = if (ang - near).abs() < 6f32.to_radians() { near } else { ang };
    let (s, c) = ang.sin_cos();
    ((m.0 - c * len / 2.0, m.1 - s * len / 2.0), (m.0 + c * len / 2.0, m.1 + s * len / 2.0))
}

/// Le due alette di una punta in `tip`, per un'asta che viene da `tail`.
fn head(tail: P, tip: P, barb: f32) -> (P, P) {
    let ang = (tail.1 - tip.1).atan2(tail.0 - tip.0);
    let a = 28f32.to_radians();
    let p = |t: f32| (tip.0 + barb * t.cos(), tip.1 + barb * t.sin());
    (p(ang + a), p(ang - a))
}

/// Una punta disegnata a parte: una «V» col vertice vicino al capo di una
/// linea (`lines`: coda, capo).
fn arrow_head(p: &[P], lines: &[(P, P)], hh: f32) -> Option<Vec<P>> {
    let total: f32 = p.windows(2).map(|w| dist(w[0], w[1])).sum();
    let simp = rdp(p, (0.18 * total).max(0.05 * hh));
    if simp.len() != 3 {
        return None;
    }
    let (b1, apex, b2) = (simp[0], simp[1], simp[2]);
    let (l1, l2) = (dist(b1, apex), dist(b2, apex));
    if l1.min(l2) < 0.4 * l1.max(l2) {
        return None;
    }
    for &(tail, tip) in lines {
        let len = dist(tail, tip);
        if len <= 0.0 || dist(apex, tip) > 0.6 * hh.max(0.15 * len) {
            continue;
        }
        // le alette tornano verso l'asta
        let dir = ((tip.0 - tail.0) / len, (tip.1 - tail.1) / len);
        let back = |q: P| (q.0 - apex.0) * dir.0 + (q.1 - apex.1) * dir.1 < 0.0;
        if back(b1) && back(b2) {
            let (h1, h2) = head(tail, tip, (l1 + l2) / 2.0);
            return Some(vec![h1, tip, h2]);
        }
    }
    None
}

/// Un'ellisse: centro, semiassi, rotazione.
#[derive(Clone, Copy, Debug)]
struct Ell {
    cx: f32,
    cy: f32,
    a: f32,
    b: f32,
    theta: f32,
}

impl Ell {
    fn at(&self, t: f32) -> P {
        let (st, ct) = self.theta.sin_cos();
        let (u, v) = (self.a * t.cos(), self.b * t.sin());
        (self.cx + u * ct - v * st, self.cy + u * st + v * ct)
    }

    /// Il punto nel sistema dell'ellisse (u lungo l'asse a).
    fn local(&self, q: P) -> (f32, f32) {
        let (st, ct) = self.theta.sin_cos();
        let (dx, dy) = (q.0 - self.cx, q.1 - self.cy);
        (dx * ct + dy * st, -dx * st + dy * ct)
    }

    /// L'angolo (parametrico) del punto dell'ellisse più vicino a `q`.
    fn param(&self, q: P) -> f32 {
        let (u, v) = self.local(q);
        (v / self.b).atan2(u / self.a)
    }

    /// Quanto `q` è fuori dall'ellisse (0 = sopra, relativo).
    fn off(&self, q: P) -> f32 {
        let (u, v) = self.local(q);
        (((u / self.a).powi(2) + (v / self.b).powi(2)).sqrt() - 1.0).abs()
    }

    /// Quasi tonda: un cerchio; quasi dritta: senza rotazione.
    fn tidy(mut self) -> Ell {
        if self.b / self.a > 0.85 {
            let r = (self.a + self.b) / 2.0;
            self.a = r;
            self.b = r;
            self.theta = 0.0;
        } else {
            let near = (self.theta / FRAC_PI_2).round() * FRAC_PI_2;
            if (self.theta - near).abs() < 6f32.to_radians() {
                self.theta = near;
            }
        }
        self
    }

    /// I punti da `t0` per un giro (`sweep` con segno).
    fn arc_pts(&self, t0: f32, sweep: f32) -> Vec<P> {
        let n = ((sweep.abs() / TAU) * 72.0).ceil().max(8.0) as usize;
        (0..=n).map(|k| self.at(t0 + sweep * k as f32 / n as f32)).collect()
    }

}

/// Un'ellisse chiusa (o un cerchio) fatta a mano: i suoi parametri, con il
/// verso in cui è stata disegnata.
fn ellipse_fit(p: &[P]) -> Option<EllDir> {
    let q = resample(p, 64);
    let (cx, cy) = q.iter().fold((0.0, 0.0), |a, b| (a.0 + b.0 / 64.0, a.1 + b.1 / 64.0));
    let (mut sxx, mut syy, mut sxy) = (0.0f32, 0.0f32, 0.0f32);
    for &(x, y) in &q {
        sxx += (x - cx) * (x - cx) / 64.0;
        syy += (y - cy) * (y - cy) / 64.0;
        sxy += (x - cx) * (y - cy) / 64.0;
    }
    let tr = sxx + syy;
    let det = sxx * syy - sxy * sxy;
    let disc = ((tr * tr / 4.0) - det).max(0.0).sqrt();
    let (l1, l2) = (tr / 2.0 + disc, (tr / 2.0 - disc).max(0.0));
    let theta = 0.5 * (2.0 * sxy).atan2(sxx - syy);
    let (a, b) = ((2.0 * l1).sqrt(), (2.0 * l2).sqrt());
    if a <= 0.0 || b < 0.15 * a {
        return None;
    }
    // chiusa: la fine torna vicino all'inizio, e il giro è quasi completo
    if dist(p[0], *p.last()?) > 0.5 * b.max(0.35 * a) {
        return None;
    }
    let e = Ell { cx, cy, a, b, theta };
    let (mut err, mut worst, mut sweep) = (0.0f32, 0.0f32, 0.0f32);
    let mut prev: Option<f32> = None;
    for &pt in &q {
        let r = e.off(pt);
        err += r / 64.0;
        worst = worst.max(r);
        let (u, v) = e.local(pt);
        let ang = v.atan2(u);
        if let Some(pa) = prev {
            sweep += wrap(ang - pa);
        }
        prev = Some(ang);
    }
    if err > 0.12 || worst > 0.35 || sweep.abs() < 300f32.to_radians() {
        return None;
    }
    Some(EllDir { e: e.tidy(), dir: sweep.signum() })
}

/// Un'ellisse con il verso del giro.
#[derive(Clone, Copy, Debug)]
struct EllDir {
    e: Ell,
    dir: f32,
}

impl EllDir {
    /// Il giro intero dal punto più vicino a `start`, nel verso del disegno.
    fn full(&self, start: P) -> Vec<P> {
        self.e.arc_pts(self.e.param(start), self.dir * TAU)
    }
}

impl std::ops::Deref for EllDir {
    type Target = Ell;
    fn deref(&self) -> &Ell {
        &self.e
    }
}

/// Due tratti aperti che insieme fanno un'ellisse (le due metà): i capi si
/// toccano e insieme fanno il giro. Torna i punti nuovi di tutti e due, che
/// si toccano esattamente.
fn ellipse_halves(a: &[P], b: &[P], hh: f32) -> Option<(Vec<P>, Vec<P>)> {
    let size = diag(&[a, b].concat());
    let tol = (0.6 * hh).max(0.12 * size);
    let (a0, a1) = (a[0], a[a.len() - 1]);
    // b nello stesso verso del giro: comincia dove finisce a
    let b_fwd = dist(a1, b[0]) < tol && dist(b[b.len() - 1], a0) < tol;
    let b_rev = dist(a1, b[b.len() - 1]) < tol && dist(b[0], a0) < tol;
    if !b_fwd && !b_rev {
        return None;
    }
    let bb: Vec<P> = if b_fwd { b.to_vec() } else { b.iter().rev().copied().collect() };
    // ognuna è un pezzo vero (non un trattino che chiude un cerchio quasi intero)
    let (la, lb) = (path_len(a), path_len(&bb));
    if la.min(lb) < 0.2 * (la + lb) {
        return None;
    }
    let mut loop_pts = a.to_vec();
    loop_pts.extend(&bb);
    let ed = ellipse_fit(&loop_pts)?;
    let e = ed.e;
    let mid = |p: P, q: P| ((p.0 + q.0) / 2.0, (p.1 + q.1) / 2.0);
    let j1 = e.param(mid(a1, bb[0]));
    let j0 = e.param(mid(bb[bb.len() - 1], a0));
    let d = ed.dir;
    let span = |from: f32, to: f32| ((to - from) * d).rem_euclid(TAU) * d;
    let pa = e.arc_pts(j0, span(j0, j1));
    let mut pb = e.arc_pts(j1, span(j1, j0));
    if b_rev {
        pb.reverse();
    }
    Some((pa, pb))
}

fn path_len(p: &[P]) -> f32 {
    p.windows(2).map(|w| dist(w[0], w[1])).sum()
}

/// Un arco d'ellisse aperto (come la mezza ellisse di un cilindro): lo
/// stesso arco perfetto. L'ellisse si trova coi minimi quadrati diretti
/// (Fitzgibbon, nella forma stabile di Halíř e Flusser).
fn elliptic_arc(p: &[P]) -> Option<Vec<P>> {
    elliptic_arc_fit(p).map(|(pts, _)| pts)
}

/// Come `elliptic_arc`, con l'errore (relativo).
fn elliptic_arc_fit(p: &[P]) -> Option<(Vec<P>, f32)> {
    let q = resample(p, 64);
    let e = fit_conic_ellipse(&q)?;
    if e.b < 0.12 * e.a || e.a > 1.5 * diag(p) {
        return None;
    }
    let mut worst = 0.0f32;
    let mut rms = 0.0f32;
    for &pt in &q {
        let r = e.off(pt);
        worst = worst.max(r);
        rms += r * r / q.len() as f32;
    }
    if rms.sqrt() > 0.035 || worst > 0.1 {
        return None;
    }
    // il giro: sempre nello stesso verso, da 100° a 340°
    let mut sweep = 0.0f32;
    let mut prev = e.param(q[0]);
    for &pt in &q[1..] {
        let t = e.param(pt);
        let d = wrap(t - prev);
        if sweep != 0.0 && d.signum() != sweep.signum() && d.abs() > 0.15 {
            return None;
        }
        sweep += d;
        prev = t;
    }
    if sweep.abs() < 100f32.to_radians() || sweep.abs() > 340f32.to_radians() {
        return None;
    }
    let e = e.tidy();
    Some((e.arc_pts(e.param(q[0]), sweep), rms.sqrt()))
}

/// L'ellisse dei minimi quadrati diretti per i punti (None se la conica
/// migliore non è un'ellisse).
fn fit_conic_ellipse(q: &[P]) -> Option<Ell> {
    // centrati e in scala, per i conti
    let n = q.len() as f64;
    let (mx, my) = q.iter().fold((0.0f64, 0.0f64), |a, b| (a.0 + b.0 as f64 / n, a.1 + b.1 as f64 / n));
    let s = q.iter().map(|b| ((b.0 as f64 - mx).powi(2) + (b.1 as f64 - my).powi(2)).sqrt()).sum::<f64>() / n;
    if s <= 0.0 {
        return None;
    }
    let pts: Vec<(f64, f64)> = q.iter().map(|b| ((b.0 as f64 - mx) / s, (b.1 as f64 - my) / s)).collect();
    let mut s1 = [[0.0f64; 3]; 3];
    let mut s2 = [[0.0f64; 3]; 3];
    let mut s3 = [[0.0f64; 3]; 3];
    for &(x, y) in &pts {
        let d1 = [x * x, x * y, y * y];
        let d2 = [x, y, 1.0];
        for i in 0..3 {
            for j in 0..3 {
                s1[i][j] += d1[i] * d1[j];
                s2[i][j] += d1[i] * d2[j];
                s3[i][j] += d2[i] * d2[j];
            }
        }
    }
    let s3i = inv3(&s3)?;
    // T = -S3⁻¹ S2ᵀ
    let mut t = [[0.0f64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            t[i][j] = -(0..3).map(|k| s3i[i][k] * s2[j][k]).sum::<f64>();
        }
    }
    // M = S1 + S2 T, poi C1⁻¹ M
    let mut m = [[0.0f64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            m[i][j] = s1[i][j] + (0..3).map(|k| s2[i][k] * t[k][j]).sum::<f64>();
        }
    }
    let m = [[m[2][0] / 2.0, m[2][1] / 2.0, m[2][2] / 2.0], [-m[1][0], -m[1][1], -m[1][2]], [m[0][0] / 2.0, m[0][1] / 2.0, m[0][2] / 2.0]];
    let mut best: Option<[f64; 3]> = None;
    for lam in eig3(&m) {
        let Some(v) = null3(&m, lam) else { continue };
        if 4.0 * v[0] * v[2] - v[1] * v[1] > 0.0 {
            best = Some(v);
            break;
        }
    }
    let a1 = best?;
    let a2: Vec<f64> = (0..3).map(|i| (0..3).map(|k| t[i][k] * a1[k]).sum::<f64>()).collect();
    let (a, b, c, d, f, g) = (a1[0], a1[1] / 2.0, a1[2], a2[0] / 2.0, a2[1] / 2.0, a2[2]);
    // centro, semiassi e rotazione della conica a x² + 2b xy + c y² + 2d x + 2f y + g = 0
    let den = b * b - a * c;
    if den >= 0.0 {
        return None;
    }
    let x0 = (c * d - b * f) / den;
    let y0 = (a * f - b * d) / den;
    let num = 2.0 * (a * f * f + c * d * d + g * b * b - 2.0 * b * d * f - a * c * g);
    let root = ((a - c).powi(2) + 4.0 * b * b).sqrt();
    let l1 = num / (den * (root - (a + c)));
    let l2 = num / (den * (-root - (a + c)));
    if l1 <= 0.0 || l2 <= 0.0 {
        return None;
    }
    let (r1, r2) = (l1.sqrt(), l2.sqrt());
    let mut theta = if b.abs() < 1e-12 { if a < c { 0.0 } else { std::f64::consts::FRAC_PI_2 } } else { 0.5 * (2.0 * b).atan2(a - c) };
    let (mut ra, mut rb) = (r1, r2);
    if ra < rb {
        std::mem::swap(&mut ra, &mut rb);
        theta += std::f64::consts::FRAC_PI_2;
    }
    let e = Ell { cx: (x0 * s + mx) as f32, cy: (y0 * s + my) as f32, a: (ra * s) as f32, b: (rb * s) as f32, theta: theta as f32 };
    // la rotazione giusta: si prova anche quella di 90° e si tiene la migliore
    let e2 = Ell { theta: e.theta + FRAC_PI_2, ..e };
    let err = |e: &Ell| q.iter().map(|&p| e.off(p)).sum::<f32>();
    Some(if err(&e2) < err(&e) { e2 } else { e })
}

fn inv3(m: &[[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0]) + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    if det.abs() < 1e-14 {
        return None;
    }
    let c = |i: usize, j: usize| {
        let (r0, r1) = ((i + 1) % 3, (i + 2) % 3);
        let (c0, c1) = ((j + 1) % 3, (j + 2) % 3);
        m[r0][c0] * m[r1][c1] - m[r0][c1] * m[r1][c0]
    };
    let mut out = [[0.0f64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            out[j][i] = c(i, j) / det;
        }
    }
    Some(out)
}

/// Gli autovalori reali di una matrice 3×3 (dal polinomio caratteristico).
fn eig3(m: &[[f64; 3]; 3]) -> Vec<f64> {
    let tr = m[0][0] + m[1][1] + m[2][2];
    let minors = m[0][0] * m[1][1] - m[0][1] * m[1][0] + m[0][0] * m[2][2] - m[0][2] * m[2][0] + m[1][1] * m[2][2] - m[1][2] * m[2][1];
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0]) + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    // λ³ - tr λ² + minors λ - det = 0
    let (a, b, c) = (-tr, minors, -det);
    let p = b - a * a / 3.0;
    let qq = 2.0 * a * a * a / 27.0 - a * b / 3.0 + c;
    let disc = qq * qq / 4.0 + p * p * p / 27.0;
    let shift = -a / 3.0;
    if disc > 1e-18 {
        let sd = disc.sqrt();
        vec![(-qq / 2.0 + sd).cbrt() + (-qq / 2.0 - sd).cbrt() + shift]
    } else if p.abs() < 1e-18 {
        vec![shift]
    } else {
        let r = (-p / 3.0).sqrt();
        let phi = ((-qq / 2.0) / (r * r * r)).clamp(-1.0, 1.0).acos();
        (0..3).map(|k| 2.0 * r * ((phi + 2.0 * std::f64::consts::PI * k as f64) / 3.0).cos() + shift).collect()
    }
}

/// Un vettore del nucleo di (M − λI).
fn null3(m: &[[f64; 3]; 3], lam: f64) -> Option<[f64; 3]> {
    let r: Vec<[f64; 3]> = (0..3).map(|i| [m[i][0] - if i == 0 { lam } else { 0.0 }, m[i][1] - if i == 1 { lam } else { 0.0 }, m[i][2] - if i == 2 { lam } else { 0.0 }]).collect();
    let cross = |u: &[f64; 3], v: &[f64; 3]| [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
    let cands = [cross(&r[0], &r[1]), cross(&r[0], &r[2]), cross(&r[1], &r[2])];
    let best = cands.into_iter().max_by(|x, y| (x.iter().map(|v| v * v).sum::<f64>()).total_cmp(&y.iter().map(|v| v * v).sum::<f64>()))?;
    let n = best.iter().map(|v| v * v).sum::<f64>().sqrt();
    (n > 1e-12).then(|| [best[0] / n, best[1] / n, best[2] / n])
}

/// Una freccia con l'asta curva, o con due punte: si tolgono le punte, si
/// rifà l'asta (dritta, arco di cerchio o d'ellisse, curva lisciata) e si
/// rimettono le punte, simmetriche, nella direzione dell'asta.
/// Con `both` solo le frecce con due punte (si guarda prima delle spezzate).
fn curved_arrow(p: &[P], hh: f32, both: bool) -> Option<Vec<P>> {
    let total = path_len(p);
    let idx = rdp_idx(p, 0, p.len() - 1, (0.025 * total).max(0.1 * hh));
    let end = head_at_end(p, &idx, hh);
    let rev: Vec<P> = p.iter().rev().copied().collect();
    let ridx: Vec<usize> = idx.iter().rev().map(|&i| p.len() - 1 - i).collect();
    let start = head_at_end(&rev, &ridx, hh);
    if (end.is_none() && start.is_none()) || (both && (end.is_none() || start.is_none())) {
        return None;
    }
    let s = start.map_or(0, |(t, _)| p.len() - 1 - t);
    let e = end.map_or(p.len() - 1, |(t, _)| t);
    if e <= s + 2 {
        return None;
    }
    let body = &p[s..=e];
    let blen = path_len(body);
    if blen < 2.0 * hh {
        return None;
    }
    // le punte non più lunghe di un terzo dell'asta
    for (_, barb) in start.iter().chain(end.iter()) {
        if *barb > 0.35 * blen {
            return None;
        }
    }
    let shaft: Vec<P> = if let Some((a, b)) = straight(body) {
        let (a, b) = snap_line(a, b);
        densify(&[a, b], 0.25 * hh)
    } else if let Some(pts) = best_arc(body) {
        pts
    } else {
        smooth_curve(body, hh)
    };
    let mut pts = Vec::new();
    if let Some((_, barb)) = start {
        let tip = shaft[0];
        let (b1, b2) = head(back_along(&shaft, barb.max(hh) * 0.8, false), tip, barb);
        pts.extend([b1, tip, b2]);
    }
    pts.extend(&shaft);
    if let Some((_, barb)) = end {
        let tip = shaft[shaft.len() - 1];
        let (b1, b2) = head(back_along(&shaft, barb.max(hh) * 0.8, true), tip, barb);
        pts.extend([b1, tip, b2]);
    }
    Some(pts)
}

/// La punta alla fine di un tratto: un vertice (fra gli ultimi della
/// spezzata semplificata) dopo il quale il tratto torna indietro di poco,
/// una o due alette. Torna l'indice della punta e la lunghezza delle alette.
fn head_at_end(p: &[P], idx: &[usize], hh: f32) -> Option<(usize, f32)> {
    let m = idx.len();
    if m < 3 {
        return None;
    }
    // si prova dal vertice più indietro (una «V» intera: punta, aletta,
    // punta, aletta) al più vicino alla fine
    for back in (2..=4.min(m - 1)).rev() {
        let t = idx[m - back];
        let tip = p[t];
        let tail = back_along(&p[..=t], hh, true);
        let dir = (tip.0 - tail.0, tip.1 - tail.1);
        let dl = dir.0.hypot(dir.1);
        if dl <= 0.0 {
            continue;
        }
        let after = &p[t + 1..];
        let far: Vec<P> = after.iter().copied().filter(|&q| dist(q, tip) > 0.3 * hh).collect();
        if far.is_empty() {
            continue;
        }
        let barb = far.iter().map(|&q| dist(q, tip)).fold(0.0f32, f32::max);
        if barb > 2.5 * hh || path_len(after) > 3.0 * barb + 0.5 * hh {
            continue;
        }
        // tutte le alette tornano indietro rispetto all'asta (almeno 100°)
        let back_ok = far.iter().all(|&q| {
            let w = (q.0 - tip.0, q.1 - tip.1);
            (w.0 * dir.0 + w.1 * dir.1) / (w.0.hypot(w.1) * dl) < -0.17
        });
        if back_ok {
            return Some((t, barb));
        }
    }
    None
}

/// Le linee tratteggiate: tre o più trattini sulla stessa retta, a passo
/// regolare. Diventano trattini uguali, allineati, equidistanti. Torna i
/// trattini rimasti soli.
fn dashed(dashes: &[(usize, P, P)], hh: f32, out: &mut Vec<Shaped>) -> Vec<(usize, P, P)> {
    let n = dashes.len();
    let mut used = vec![false; n];
    let dir = |a: P, b: P| wrap(2.0 * (b.1 - a.1).atan2(b.0 - a.0)) / 2.0;
    for s in 0..n {
        if used[s] {
            continue;
        }
        let mut group = vec![s];
        loop {
            // la retta del gruppo: direzione media e un punto
            let ang = dir(dashes[group[0]].1, dashes[group[0]].2);
            let (c, sn) = (ang.cos(), ang.sin());
            let pts: Vec<P> = group.iter().flat_map(|&g| [dashes[g].1, dashes[g].2]).collect();
            let o = pts.iter().fold((0.0, 0.0), |a, b| (a.0 + b.0 / pts.len() as f32, a.1 + b.1 / pts.len() as f32));
            let along = |q: P| (q.0 - o.0) * c + (q.1 - o.1) * sn;
            let across = |q: P| (-(q.0 - o.0) * sn + (q.1 - o.1) * c).abs();
            let (lo, hi) = pts.iter().fold((f32::MAX, f32::MIN), |b, &q| (b.0.min(along(q)), b.1.max(along(q))));
            let maxlen = group.iter().map(|&g| dist(dashes[g].1, dashes[g].2)).fold(0.0f32, f32::max);
            // il più vicino fra quelli sulla stessa retta, oltre un capo del gruppo
            let next = (0..n)
                .filter(|&k| !used[k] && !group.contains(&k))
                .filter_map(|k| {
                    let (_, a, b) = dashes[k];
                    let d = (wrap(2.0 * (dir(a, b) - ang)) / 2.0).abs();
                    let m = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
                    let (ka, kb) = (along(a).min(along(b)), along(a).max(along(b)));
                    let gap = if kb < lo { lo - kb } else if ka > hi { ka - hi } else { 0.0 };
                    (d < 8f32.to_radians() && across(m) < 0.45 * hh && gap > 0.0 && gap < 3.0 * maxlen.max(0.5 * hh)).then_some((gap, k))
                })
                .min_by(|x, y| x.0.total_cmp(&y.0))
                .map(|(_, k)| k);
            match next {
                Some(k) => group.push(k),
                None => break,
            }
        }
        if group.len() < 3 {
            continue;
        }
        for &g in &group {
            used[g] = true;
        }
        // la retta: la direzione di tutti i capi (componente principale)
        let pts: Vec<P> = group.iter().flat_map(|&g| [dashes[g].1, dashes[g].2]).collect();
        let k = pts.len() as f32;
        let o = pts.iter().fold((0.0, 0.0), |a, b| (a.0 + b.0 / k, a.1 + b.1 / k));
        let (mut sxx, mut syy, mut sxy) = (0.0f32, 0.0f32, 0.0f32);
        for &(x, y) in &pts {
            sxx += (x - o.0).powi(2);
            syy += (y - o.1).powi(2);
            sxy += (x - o.0) * (y - o.1);
        }
        let mut ang = 0.5 * (2.0 * sxy).atan2(sxx - syy);
        let near = (ang / FRAC_PI_2).round() * FRAC_PI_2;
        if (ang - near).abs() < 6f32.to_radians() {
            ang = near;
        }
        let (c, sn) = (ang.cos(), ang.sin());
        let along = |q: P| (q.0 - o.0) * c + (q.1 - o.1) * sn;
        let mut spans: Vec<(f32, f32, usize)> = group.iter().map(|&g| {
            let (a, b) = (along(dashes[g].1), along(dashes[g].2));
            (a.min(b), a.max(b), g)
        }).collect();
        spans.sort_by(|x, y| x.0.total_cmp(&y.0));
        let mut lens: Vec<f32> = spans.iter().map(|s| s.1 - s.0).collect();
        lens.sort_by(f32::total_cmp);
        let len = lens[lens.len() / 2];
        let (first, last) = (spans[0].0, spans[spans.len() - 1].1);
        let step = (last - first - len) / (spans.len() - 1) as f32;
        for (j, &(_, _, g)) in spans.iter().enumerate() {
            let s0 = first + j as f32 * step;
            let a = (o.0 + s0 * c, o.1 + s0 * sn);
            let b = (o.0 + (s0 + len) * c, o.1 + (s0 + len) * sn);
            // nel verso in cui era stato tracciato
            let (_, da, db) = dashes[g];
            let (a, b) = if along(da) <= along(db) { (a, b) } else { (b, a) };
            out.push(Shaped { index: dashes[g].0, kind: Kind::Dash, points: densify(&[a, b], 0.25 * hh) });
        }
    }
    (0..n).filter(|&k| !used[k]).map(|k| dashes[k]).collect()
}

/// Un poligono chiuso quasi regolare diventa regolare: rettangolo (e
/// quadrato), rombo, parallelogramma, trapezio (i lati quasi paralleli),
/// triangolo equilatero o isoscele, poligono regolare. Torna se l'ha toccato.
fn regularize(verts: &mut [P], vs: &[usize]) -> bool {
    let n = vs.len();
    let v: Vec<P> = vs.iter().map(|&i| verts[i]).collect();
    let side = |k: usize| dist(v[k], v[(k + 1) % n]);
    let sides: Vec<f32> = (0..n).map(side).collect();
    let mean = sides.iter().sum::<f32>() / n as f32;
    let even = |tol: f32| sides.iter().all(|&s| (s - mean).abs() < tol * mean);
    // angolo interno al vertice k
    let inner = |k: usize| PI - turn(v[(k + n - 1) % n], v[k], v[(k + 1) % n]);
    let c = v.iter().fold((0.0, 0.0), |a, b| (a.0 + b.0 / n as f32, a.1 + b.1 / n as f32));
    let snap_ang = |a: f32| {
        let near = (a / FRAC_PI_2).round() * FRAC_PI_2;
        if (a - near).abs() < 6f32.to_radians() { near } else { a }
    };
    let dir_of = |k: usize| (v[(k + 1) % n].1 - v[k].1).atan2(v[(k + 1) % n].0 - v[k].0);
    let set = |verts: &mut [P], new: &[P]| {
        for (k, &i) in vs.iter().enumerate() {
            verts[i] = new[k];
        }
    };
    let close = |a: f32, b: f32, tol: f32| (wrap(2.0 * (a - b)) / 2.0).abs() < tol;
    match n {
        4 => {
            let right = (0..4).all(|k| (inner(k) - FRAC_PI_2).abs() < 10f32.to_radians());
            if right {
                // rettangolo: orientamento medio dei lati (modulo 90°)
                let mut acc = (0.0f32, 0.0f32);
                for k in 0..4 {
                    let a = 4.0 * dir_of(k);
                    acc.0 += a.cos();
                    acc.1 += a.sin();
                }
                let ang = snap_ang(acc.1.atan2(acc.0) / 4.0);
                let (ca, sa) = (ang.cos(), ang.sin());
                // le misure lungo i due assi
                let along = |q: P| ((q.0 - c.0) * ca + (q.1 - c.1) * sa, -(q.0 - c.0) * sa + (q.1 - c.1) * ca);
                let loc: Vec<(f32, f32)> = v.iter().map(|&q| along(q)).collect();
                let mut w = loc.iter().map(|q| q.0.abs()).sum::<f32>() / 4.0;
                let mut h = loc.iter().map(|q| q.1.abs()).sum::<f32>() / 4.0;
                if (w - h).abs() < 0.1 * w.max(h) {
                    let m = (w + h) / 2.0;
                    w = m;
                    h = m;
                }
                let new: Vec<P> = loc
                    .iter()
                    .map(|q| {
                        let (u, t) = (w * q.0.signum(), h * q.1.signum());
                        (c.0 + u * ca - t * sa, c.1 + u * sa + t * ca)
                    })
                    .collect();
                set(verts, &new);
                return true;
            }
            let par02 = close(dir_of(0), dir_of(2), 6f32.to_radians());
            let par13 = close(dir_of(1), dir_of(3), 6f32.to_radians());
            if par02 && par13 {
                // parallelogramma (e rombo se i lati sono quasi uguali): le
                // diagonali si tagliano a metà, nel rombo ad angolo retto
                let mut u = ((v[0].0 - v[2].0) / 2.0, (v[0].1 - v[2].1) / 2.0);
                let mut w = ((v[1].0 - v[3].0) / 2.0, (v[1].1 - v[3].1) / 2.0);
                if even(0.1) {
                    let (lu, lw) = (u.0.hypot(u.1), w.0.hypot(w.1));
                    let a = snap_ang(u.1.atan2(u.0));
                    let s = if (w.0 * (-a.sin()) + w.1 * a.cos()) >= 0.0 { 1.0 } else { -1.0 };
                    u = (lu * a.cos(), lu * a.sin());
                    w = (-s * lw * a.sin(), s * lw * a.cos());
                }
                let new = [(c.0 + u.0, c.1 + u.1), (c.0 + w.0, c.1 + w.1), (c.0 - u.0, c.1 - u.1), (c.0 - w.0, c.1 - w.1)];
                set(verts, &new);
                return true;
            }
            // trapezio: la coppia di lati quasi paralleli diventa parallela
            // (si gira il lato più corto attorno al suo punto di mezzo)
            for (k, j) in [(0usize, 2usize), (1, 3)] {
                if close(dir_of(k), dir_of(j), 6f32.to_radians()) {
                    let (keep, mov) = if sides[k] >= sides[j] { (k, j) } else { (j, k) };
                    let a = dir_of(keep) + PI; // i lati opposti vanno in versi opposti
                    let (p0, p1) = (v[mov], v[(mov + 1) % 4]);
                    let m = ((p0.0 + p1.0) / 2.0, (p0.1 + p1.1) / 2.0);
                    let half = sides[mov] / 2.0;
                    let mut new = v.clone();
                    new[mov] = (m.0 - half * a.cos(), m.1 - half * a.sin());
                    new[(mov + 1) % 4] = (m.0 + half * a.cos(), m.1 + half * a.sin());
                    set(verts, &new);
                    return true;
                }
            }
            false
        }
        3 => {
            if even(0.08) {
                // equilatero: stesso centro, un lato com'era orientato (dritto se quasi)
                let k = (0..3).min_by(|&x, &y| (wrap(2.0 * dir_of(x)) / 2.0).abs().total_cmp(&(wrap(2.0 * dir_of(y)) / 2.0).abs())).unwrap();
                let r = mean / 3f32.sqrt();
                let a0 = snap_ang(dir_of(k));
                // il vertice opposto al lato k, dalla parte dov'era
                let apex = v[(k + 2) % 3];
                let normal = (-(a0.sin()), a0.cos());
                let sgn = if (apex.0 - c.0) * normal.0 + (apex.1 - c.1) * normal.1 >= 0.0 { 1.0 } else { -1.0 };
                let up = (normal.0 * sgn, normal.1 * sgn);
                let along = (a0.cos(), a0.sin());
                let mut new = v.clone();
                new[(k + 2) % 3] = (c.0 + r * up.0, c.1 + r * up.1);
                let base = (c.0 - r / 2.0 * up.0, c.1 - r / 2.0 * up.1);
                new[k] = (base.0 - mean / 2.0 * along.0, base.1 - mean / 2.0 * along.1);
                new[(k + 1) % 3] = (base.0 + mean / 2.0 * along.0, base.1 + mean / 2.0 * along.1);
                set(verts, &new);
                return true;
            }
            // isoscele: due lati quasi uguali; la base si mette simmetrica sotto il vertice
            for k in 0..3 {
                let (s1, s2) = (sides[(k + 2) % 3], sides[k]); // i due lati che toccano il vertice k
                if (s1 - s2).abs() < 0.06 * s1.max(s2) {
                    let apex = v[k];
                    let (b0, b1) = (v[(k + 1) % 3], v[(k + 2) % 3]);
                    let mid = ((b0.0 + b1.0) / 2.0, (b0.1 + b1.1) / 2.0);
                    let mut axis = (mid.0 - apex.0, mid.1 - apex.1);
                    let hgt = axis.0.hypot(axis.1);
                    if hgt <= 0.0 {
                        return false;
                    }
                    let a = snap_ang(axis.1.atan2(axis.0));
                    axis = (a.cos(), a.sin());
                    let half = dist(b0, b1) / 2.0;
                    let perp = (-axis.1, axis.0);
                    let s = if (b0.0 - mid.0) * perp.0 + (b0.1 - mid.1) * perp.1 >= 0.0 { 1.0 } else { -1.0 };
                    let m2 = (apex.0 + hgt * axis.0, apex.1 + hgt * axis.1);
                    let mut new = v.clone();
                    new[(k + 1) % 3] = (m2.0 + s * half * perp.0, m2.1 + s * half * perp.1);
                    new[(k + 2) % 3] = (m2.0 - s * half * perp.0, m2.1 - s * half * perp.1);
                    set(verts, &new);
                    return true;
                }
            }
            false
        }
        5..=8 => {
            let want = PI * (n as f32 - 2.0) / n as f32;
            if !even(0.15) || !(0..n).all(|k| (inner(k) - want).abs() < 15f32.to_radians()) {
                return false;
            }
            let r = v.iter().map(|&q| dist(q, c)).sum::<f32>() / n as f32;
            // il verso del giro com'era, e un lato dritto se quasi
            let area: f32 = (0..n).map(|k| v[k].0 * v[(k + 1) % n].1 - v[(k + 1) % n].0 * v[k].1).sum();
            let dir = area.signum();
            let mut a0 = (v[0].1 - c.1).atan2(v[0].0 - c.0);
            let step = dir * TAU / n as f32;
            // il primo lato: se è quasi orizzontale o verticale lo diventa
            let side_ang = a0 + step / 2.0 + dir * FRAC_PI_2;
            let snapped = snap_ang(wrap(side_ang));
            a0 += wrap(snapped - wrap(side_ang));
            let new: Vec<P> = (0..n).map(|k| {
                let t = a0 + step * k as f32;
                (c.0 + r * t.cos(), c.1 + r * t.sin())
            }).collect();
            set(verts, &new);
            true
        }
        _ => false,
    }
}

/// Un arco di cerchio aperto (da 30° a 320°): lo stesso arco perfetto.
fn arc(p: &[P]) -> Option<Vec<P>> {
    arc_fit(p).map(|(pts, _)| pts)
}

/// Come `arc`, con l'errore (relativo al raggio).
fn arc_fit(p: &[P]) -> Option<(Vec<P>, f32)> {
    let q = resample(p, 48);
    let (cx, cy, r) = fit_circle(&q)?;
    if !(r.is_finite()) || r <= 0.0 {
        return None;
    }
    let mut worst = 0.0f32;
    let mut rms = 0.0f32;
    for &(x, y) in &q {
        let d = ((x - cx).hypot(y - cy) - r).abs();
        worst = worst.max(d);
        rms += d * d / q.len() as f32;
    }
    let chord = dist(q[0], q[q.len() - 1]);
    // un cerchio enorme è una linea quasi dritta, già vista sopra; un arco
    // vero si stacca bene dalla corda
    if rms.sqrt() > 0.035 * r || worst > 0.1 * r || r > 6.0 * chord.max(diag(p)) {
        return None;
    }
    let mut sweep = 0.0f32;
    let mut prev = (q[0].1 - cy).atan2(q[0].0 - cx);
    let start = prev;
    for &(x, y) in &q[1..] {
        let a = (y - cy).atan2(x - cx);
        let d = wrap(a - prev);
        // l'arco va sempre nello stesso verso
        if sweep != 0.0 && d.signum() != sweep.signum() && d.abs() > 0.15 {
            return None;
        }
        sweep += d;
        prev = a;
    }
    if sweep.abs() < 30f32.to_radians() || sweep.abs() > 320f32.to_radians() {
        return None;
    }
    let n = ((sweep.abs() / TAU) * 72.0).ceil().max(8.0) as usize;
    let pts = (0..=n)
        .map(|k| {
            let t = start + sweep * k as f32 / n as f32;
            (cx + r * t.cos(), cy + r * t.sin())
        })
        .collect();
    Some((pts, rms.sqrt() / r))
}

/// Il miglior arco per il tratto: di cerchio, o d'ellisse se ci sta
/// chiaramente meglio (la mezza ellisse di un cilindro).
fn best_arc(p: &[P]) -> Option<Vec<P>> {
    match (arc_fit(p), elliptic_arc_fit(p)) {
        (Some((c, ec)), Some((e, ee))) => Some(if ee < 0.9 * ec { e } else { c }),
        (Some((c, _)), None) => Some(c),
        (None, Some((e, _))) => Some(e),
        (None, None) => None,
    }
}

/// Il cerchio che passa meglio per i punti (minimi quadrati, Kåsa).
fn fit_circle(q: &[P]) -> Option<(f32, f32, f32)> {
    let n = q.len() as f64;
    let (mx, my) = q.iter().fold((0.0f64, 0.0f64), |a, b| (a.0 + b.0 as f64 / n, a.1 + b.1 as f64 / n));
    let (mut suu, mut svv, mut suv, mut suuu, mut svvv, mut suvv, mut svuu) = (0.0f64, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for &(x, y) in q {
        let (u, v) = (x as f64 - mx, y as f64 - my);
        suu += u * u;
        svv += v * v;
        suv += u * v;
        suuu += u * u * u;
        svvv += v * v * v;
        suvv += u * v * v;
        svuu += v * u * u;
    }
    let det = suu * svv - suv * suv;
    if det.abs() < 1e-18 {
        return None;
    }
    let (b1, b2) = ((suuu + suvv) / 2.0, (svvv + svuu) / 2.0);
    let uc = (b1 * svv - b2 * suv) / det;
    let vc = (suu * b2 - suv * b1) / det;
    let r = (uc * uc + vc * vc + (suu + svv) / n).sqrt();
    Some(((uc + mx) as f32, (vc + my) as f32, r as f32))
}

/// Il tratto senza il tremolio della mano: punti a passo costante, mediati
/// coi vicini (su poco più di mezza lettera). I capi e gli spigoli veri
/// (dove il tratto gira molto in poco spazio) restano dove sono, e la media
/// non li scavalca: gli angoli di un poligono non si arrotondano.
fn denoise(p: &[P], hh: f32) -> Vec<P> {
    let total: f32 = p.windows(2).map(|w| dist(w[0], w[1])).sum();
    let n = ((total / (0.1 * hh)).ceil() as usize).clamp(8, 600);
    let mut q = resample(p, n);
    // gli spigoli: dove gira più di 45° su mezza lettera, il punto che gira di più
    let m = 5usize;
    let bend: Vec<f32> = (0..n).map(|i| if i < m || i + m >= n { 0.0 } else { turn(q[i - m], q[i], q[i + m]) }).collect();
    let mut corner = vec![false; n];
    corner[0] = true;
    corner[n - 1] = true;
    let lim = 45f32.to_radians();
    let mut i = 0;
    while i < n {
        if bend[i] > lim {
            let start = i;
            while i < n && bend[i] > lim {
                i += 1;
            }
            let best = (start..i).max_by(|&x, &y| bend[x].total_cmp(&bend[y])).unwrap();
            corner[best] = true;
        } else {
            i += 1;
        }
    }
    let k = 3usize;
    for _ in 0..2 {
        let prev = q.clone();
        for i in 1..n - 1 {
            if corner[i] {
                continue;
            }
            let mut lo = i.saturating_sub(k);
            let mut hi = (i + k).min(n - 1);
            // la media resta dal suo lato dello spigolo
            if let Some(c) = (lo..i).rev().find(|&j| corner[j]) {
                lo = c;
            }
            if let Some(c) = (i + 1..=hi).find(|&j| corner[j]) {
                hi = c;
            }
            let cnt = (hi - lo + 1) as f32;
            let (sx, sy) = prev[lo..=hi].iter().fold((0.0, 0.0), |a, b| (a.0 + b.0, a.1 + b.1));
            q[i] = (sx / cnt, sy / cnt);
        }
    }
    q
}

/// Una curva libera, lisciata: niente tremolio, stessi capi, stessa strada.
fn smooth_curve(p: &[P], hh: f32) -> Vec<P> {
    let total: f32 = p.windows(2).map(|w| dist(w[0], w[1])).sum();
    let n = ((total / (0.12 * hh)).ceil() as usize).clamp(8, 400);
    let mut q = resample(p, n);
    let k = 3usize;
    for _ in 0..5 {
        let prev = q.clone();
        for i in 1..q.len() - 1 {
            let (lo, hi) = (i.saturating_sub(k), (i + k).min(q.len() - 1));
            let m = (hi - lo + 1) as f32;
            let (sx, sy) = prev[lo..=hi].iter().fold((0.0, 0.0), |a, b| (a.0 + b.0, a.1 + b.1));
            q[i] = (sx / m, sy / m);
        }
    }
    q
}

/// La spezzata con punti intermedi (passo `step`): un tratto di penna.
fn densify(p: &[P], step: f32) -> Vec<P> {
    let mut out = vec![p[0]];
    for w in p.windows(2) {
        let n = (dist(w[0], w[1]) / step.max(1e-6)).ceil().max(1.0) as usize;
        for k in 1..=n {
            let f = k as f32 / n as f32;
            out.push((w[0].0 + (w[1].0 - w[0].0) * f, w[0].1 + (w[1].1 - w[0].1) * f));
        }
    }
    out
}

/// `n` punti a distanza costante lungo la spezzata.
fn resample(p: &[P], n: usize) -> Vec<P> {
    if p.len() < 2 || n < 2 {
        return vec![p.first().copied().unwrap_or((0.0, 0.0)); n.max(1)];
    }
    let cum: Vec<f32> = std::iter::once(0.0)
        .chain(p.windows(2).scan(0.0, |acc, w| {
            *acc += dist(w[0], w[1]);
            Some(*acc)
        }))
        .collect();
    let total = *cum.last().unwrap();
    let mut out = Vec::with_capacity(n);
    let mut j = 0;
    for k in 0..n {
        let t = total * k as f32 / (n - 1) as f32;
        while j + 1 < cum.len() - 1 && cum[j + 1] < t {
            j += 1;
        }
        let seg = (cum[j + 1] - cum[j]).max(1e-9);
        let f = ((t - cum[j]) / seg).clamp(0.0, 1.0);
        out.push((p[j].0 + (p[j + 1].0 - p[j].0) * f, p[j].1 + (p[j + 1].1 - p[j].1) * f));
    }
    out
}

/// Ramer–Douglas–Peucker: gli spigoli della spezzata.
fn rdp(p: &[P], eps: f32) -> Vec<P> {
    if p.len() < 3 {
        return p.to_vec();
    }
    let (a, b) = (p[0], p[p.len() - 1]);
    let (k, d) = p.iter().enumerate().skip(1).take(p.len() - 2).map(|(i, &q)| (i, line_or_point_dist(q, a, b))).fold((0, 0.0f32), |m, x| if x.1 > m.1 { x } else { m });
    if d <= eps {
        return vec![a, b];
    }
    let mut left = rdp(&p[..=k], eps);
    let right = rdp(&p[k..], eps);
    left.pop();
    left.extend(right);
    left
}

/// Come `seg_dist`, ma per un tratto chiuso (capi uguali) la distanza dal capo.
fn line_or_point_dist(q: P, a: P, b: P) -> f32 {
    seg_dist(q, a, b)
}

fn project(q: P, a: P, b: P) -> P {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l2 = dx * dx + dy * dy;
    if l2 <= 0.0 {
        return a;
    }
    let t = (((q.0 - a.0) * dx + (q.1 - a.1) * dy) / l2).clamp(0.0, 1.0);
    (a.0 + t * dx, a.1 + t * dy)
}

fn seg_dist(q: P, a: P, b: P) -> f32 {
    dist(q, project(q, a, b))
}

fn dist(a: P, b: P) -> f32 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

fn diag(p: &[P]) -> f32 {
    let (x0, x1, y0, y1) = p.iter().fold((f32::MAX, f32::MIN, f32::MAX, f32::MIN), |b, q| (b.0.min(q.0), b.1.max(q.0), b.2.min(q.1), b.3.max(q.1)));
    (x1 - x0).hypot(y1 - y0)
}

fn wrap(mut d: f32) -> f32 {
    while d > PI {
        d -= TAU;
    }
    while d < -PI {
        d += TAU;
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Il tremolio della mano: un po' di rumore, sempre uguale.
    fn shaky(p: Vec<P>, amp: f32) -> Vec<P> {
        p.into_iter().enumerate().map(|(i, (x, y))| (x + amp * ((i as f32 * 1.7).sin()), y + amp * ((i as f32 * 2.3).cos()))).collect()
    }

    fn line(a: P, b: P, n: usize) -> Vec<P> {
        (0..n).map(|k| {
            let f = k as f32 / (n - 1) as f32;
            (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f)
        }).collect()
    }

    const H: f32 = 0.02;

    fn kind_of(strokes: &[Vec<P>]) -> Vec<Kind> {
        let cand: Vec<usize> = (0..strokes.len()).collect();
        perfect(strokes, &cand, H).iter().map(|s| s.kind).collect()
    }

    #[test]
    fn linea_storta_diventa_orizzontale() {
        let s = vec![shaky(line((0.1, 0.300), (0.5, 0.312), 40), 0.001)];
        let out = perfect(&s, &[0], H);
        assert_eq!(out[0].kind, Kind::Line);
        let ys: Vec<f32> = out[0].points.iter().map(|p| p.1).collect();
        assert!(ys.iter().all(|&y| (y - ys[0]).abs() < 1e-5), "{ys:?}");
    }

    #[test]
    fn cerchio_e_arco() {
        let circle: Vec<P> = (0..=60).map(|k| {
            let t = k as f32 / 60.0 * TAU * 1.02;
            (0.5 + 0.1 * t.cos() * 1.05, 0.5 + 0.1 * t.sin())
        }).collect();
        let arc_pts: Vec<P> = (0..=40).map(|k| {
            let t = PI + k as f32 / 40.0 * PI * 0.8;
            (0.3 + 0.08 * t.cos(), 0.8 + 0.08 * t.sin())
        }).collect();
        assert_eq!(kind_of(&[shaky(circle, 0.002), shaky(arc_pts, 0.0008)]), vec![Kind::Ellipse, Kind::Arc]);
    }

    #[test]
    fn triangolo_in_tre_tratti_si_chiude() {
        let (a, b, c) = ((0.2, 0.6), (0.6, 0.605), (0.4, 0.3));
        let s = vec![line(a, b, 20), line((b.0 + 0.004, b.1 - 0.003), c, 20), line((c.0 - 0.003, c.1 + 0.002), (a.0 + 0.005, a.1 - 0.004), 20)];
        let out = perfect(&s, &[0, 1, 2], H);
        assert!(out.iter().all(|s| s.kind == Kind::Line));
        let ends: Vec<P> = out.iter().flat_map(|s| [s.points[0], *s.points.last().unwrap()]).collect();
        // ogni capo coincide con un altro
        for (i, e) in ends.iter().enumerate() {
            assert!(ends.iter().enumerate().any(|(j, f)| j != i && dist(*e, *f) < 1e-5), "{ends:?}");
        }
        // la base è orizzontale
        assert!((out[0].points[0].1 - out[0].points.last().unwrap().1).abs() < 1e-5);
    }

    #[test]
    fn rettangolo_e_freccia() {
        let rect = [line((0.1, 0.1), (0.4, 0.105), 15), line((0.4, 0.105), (0.402, 0.3), 15), line((0.402, 0.3), (0.1, 0.298), 15), line((0.1, 0.298), (0.1, 0.11), 15)].concat();
        let shaft = line((0.1, 0.6), (0.5, 0.61), 30);
        let barb = line((0.5, 0.61), (0.46, 0.585), 6);
        let barb2 = line((0.46, 0.585), (0.5, 0.61), 3);
        let barb3 = line((0.5, 0.61), (0.465, 0.64), 6);
        let arrow_pts = [shaft, barb, barb2, barb3].concat();
        let out = perfect(&[rect, arrow_pts], &[0, 1], H);
        assert_eq!(out[0].kind, Kind::Polygon);
        assert_eq!(out[1].kind, Kind::Arrow);
        // il rettangolo ha i lati dritti, orizzontali e verticali
        let r = &out[0].points;
        let (x0, x1, y0, y1) = r.iter().fold((f32::MAX, f32::MIN, f32::MAX, f32::MIN), |b, q| (b.0.min(q.0), b.1.max(q.0), b.2.min(q.1), b.3.max(q.1)));
        assert!(r.iter().all(|q| (q.0 - x0).abs() < 1e-5 || (q.0 - x1).abs() < 1e-5 || (q.1 - y0).abs() < 1e-5 || (q.1 - y1).abs() < 1e-5), "{r:?}");
    }

    #[test]
    fn curva_libera_lisciata_e_scarabocchio_piccolo_no() {
        let parab: Vec<P> = (0..=80).map(|k| {
            let x = -1.0 + 2.0 * k as f32 / 80.0;
            (0.5 + 0.2 * x, 0.6 - 0.2 * (x * x - 0.3 * x * x * x))
        }).collect();
        let tiny = shaky(line((0.1, 0.1), (0.105, 0.108), 10), 0.002);
        let out = perfect(&[shaky(parab, 0.002), tiny], &[0, 1], H);
        assert_eq!(out.len(), 1);
        assert!(matches!(out[0].kind, Kind::Curve | Kind::Arc));
    }

    fn ell_pts(cx: f32, cy: f32, a: f32, b: f32, t0: f32, t1: f32, n: usize) -> Vec<P> {
        (0..=n).map(|k| {
            let t = t0 + (t1 - t0) * k as f32 / n as f32;
            (cx + a * t.cos(), cy + b * t.sin())
        }).collect()
    }

    fn angle_at(c: P, a: P, b: P) -> f32 {
        let (u, w) = ((a.0 - c.0, a.1 - c.1), (b.0 - c.0, b.1 - c.1));
        ((u.0 * w.0 + u.1 * w.1) / (u.0.hypot(u.1) * w.0.hypot(w.1))).clamp(-1.0, 1.0).acos().to_degrees()
    }

    /// I vertici di una spezzata rifatta (i punti dove gira).
    fn corners(p: &[P]) -> Vec<P> {
        let mut v = vec![p[0]];
        for w in p.windows(3) {
            if turn(w[0], w[1], w[2]) > 0.2 {
                v.push(w[1]);
            }
        }
        v.push(p[p.len() - 1]);
        v
    }

    #[test]
    fn mezza_ellisse() {
        let half = shaky(ell_pts(0.5, 0.5, 0.15, 0.05, 0.0, PI, 50), 0.0006);
        let out = perfect(&[half], &[0], H);
        assert_eq!(out[0].kind, Kind::Arc);
        let e = Ell { cx: 0.5, cy: 0.5, a: 0.15, b: 0.05, theta: 0.0 };
        assert!(out[0].points.len() > 25 && out[0].points.iter().all(|&q| e.off(q) < 0.05), "{:?}", out[0].points);
    }

    #[test]
    fn ellisse_in_due_meta() {
        let top = ell_pts(0.5, 0.5, 0.14, 0.07, 0.03, PI - 0.02, 40);
        let bottom = ell_pts(0.5, 0.5, 0.14, 0.07, PI + 0.04, TAU - 0.03, 40);
        let out = perfect(&[shaky(top, 0.0008), shaky(bottom, 0.0008)], &[0, 1], H);
        assert_eq!(out.iter().map(|s| s.kind).collect::<Vec<_>>(), vec![Kind::Ellipse, Kind::Ellipse]);
        let (a, b) = (&out[0].points, &out[1].points);
        assert!(dist(a[a.len() - 1], b[0]) < 1e-4 && dist(b[b.len() - 1], a[0]) < 1e-4, "{:?} {:?}", (a[0], a[a.len() - 1]), (b[0], b[b.len() - 1]));
    }

    #[test]
    fn angolo_retto_storto_e_parallele() {
        // una «L» inclinata di 30°, con l'angolo di 86°
        let r = |deg: f32, len: f32, o: P| { let t = deg.to_radians(); (o.0 + len * t.cos(), o.1 + len * t.sin()) };
        let c = (0.3, 0.3);
        let el = [line(r(30.0, 0.2, c), c, 20), line(c, r(30.0 + 86.0, 0.12, c), 15)].concat();
        // due segmenti soli quasi paralleli (25° e 28°)
        let s1 = line((0.1, 0.7), r(25.0, 0.3, (0.1, 0.7)), 20);
        let s2 = line((0.1, 0.8), r(28.0, 0.3, (0.1, 0.8)), 20);
        let out = perfect(&[el, s1, s2], &[0, 1, 2], H);
        let v = corners(&out[0].points);
        assert_eq!(v.len(), 3, "{v:?}");
        assert!((angle_at(v[1], v[0], v[2]) - 90.0).abs() < 0.2, "{}", angle_at(v[1], v[0], v[2]));
        let d = |p: &[P]| (p[p.len() - 1].1 - p[0].1).atan2(p[p.len() - 1].0 - p[0].0).to_degrees();
        assert!((d(&out[1].points) - d(&out[2].points)).abs() < 0.05, "{} {}", d(&out[1].points), d(&out[2].points));
    }

    #[test]
    fn quadrato_rombo_esagono_triangolo() {
        let sq = [line((0.1, 0.1), (0.3, 0.102), 15), line((0.3, 0.102), (0.305, 0.31), 15), line((0.305, 0.31), (0.1, 0.305), 15), line((0.1, 0.305), (0.1, 0.11), 15)].concat();
        // rombo inclinato: lati quasi uguali, diagonali non perpendicolari di poco
        let (c, u, w) = ((0.6, 0.25), (0.12, 0.03), (-0.025, 0.09));
        let rh = [line((c.0 + u.0, c.1 + u.1), (c.0 + w.0, c.1 + w.1), 15), line((c.0 + w.0, c.1 + w.1), (c.0 - u.0, c.1 - u.1), 15), line((c.0 - u.0, c.1 - u.1), (c.0 - w.0 + 0.004, c.1 - w.1), 15), line((c.0 - w.0 + 0.004, c.1 - w.1), (c.0 + u.0, c.1 + u.1 + 0.002), 15)].concat();
        let hex: Vec<P> = (0..=6).map(|k| { let t = k as f32 * TAU / 6.0 + 0.1; let r = if k % 2 == 0 { 0.1 } else { 0.093 }; (0.3 + r * t.cos(), 0.7 + r * t.sin()) }).collect();
        let hexs: Vec<P> = hex.windows(2).flat_map(|w| line(w[0], w[1], 12)).collect();
        let tri = [line((0.6, 0.8), (0.8, 0.805), 15), line((0.8, 0.805), (0.705, 0.63), 15), line((0.705, 0.63), (0.6, 0.79), 15)].concat();
        let out = perfect(&[sq, rh, hexs, tri], &[0, 1, 2, 3], H);
        assert!(out.iter().all(|s| s.kind == Kind::Polygon), "{:?}", out.iter().map(|s| s.kind).collect::<Vec<_>>());
        let sides = |p: &[P]| { let v = corners(p); v.windows(2).map(|w| dist(w[0], w[1])).collect::<Vec<f32>>() };
        for (k, sh) in out.iter().enumerate() {
            let s = sides(&sh.points);
            let m = s.iter().sum::<f32>() / s.len() as f32;
            assert!(s.iter().all(|&x| (x - m).abs() < 0.002), "figura {k}: {s:?}");
        }
        // il quadrato è dritto, il rombo ha le diagonali perpendicolari
        let v = corners(&out[0].points);
        assert!((v[0].1 - v[1].1).abs() < 1e-5 || (v[0].0 - v[1].0).abs() < 1e-5);
        let r = corners(&out[1].points);
        let (d1, d2) = ((r[2].0 - r[0].0, r[2].1 - r[0].1), (r[3].0 - r[1].0, r[3].1 - r[1].1));
        assert!((d1.0 * d2.0 + d1.1 * d2.1).abs() < 1e-5, "{r:?}");
    }

    #[test]
    fn freccia_curva_e_doppia() {
        let mut curved = ell_pts(0.4, 0.5, 0.15, 0.15, PI, 1.6 * PI, 40);
        let tip = curved[curved.len() - 1];
        // la punta: un'aletta che torna indietro e una dall'altra parte
        // l'asta arriva da sinistra in basso (verso 18°): alette a 170° e 226°
        let b1 = (tip.0 + 0.03 * 170f32.to_radians().cos(), tip.1 + 0.03 * 170f32.to_radians().sin());
        let b2 = (tip.0 + 0.03 * 226f32.to_radians().cos(), tip.1 + 0.03 * 226f32.to_radians().sin());
        curved.extend(line(tip, b1, 5));
        curved.extend(line(b1, tip, 4));
        curved.extend(line(tip, b2, 5));
        let a = (0.1, 0.85);
        let b = (0.6, 0.86);
        let double = [line((a.0 + 0.03, a.1 - 0.02), a, 5), line(a, (a.0 + 0.03, a.1 + 0.02), 5), line((a.0 + 0.03, a.1 + 0.02), a, 5), line(a, b, 40), line(b, (b.0 - 0.03, b.1 - 0.02), 5), line((b.0 - 0.03, b.1 - 0.02), b, 5), line(b, (b.0 - 0.03, b.1 + 0.02), 5)].concat();
        let out = perfect(&[curved, double], &[0, 1], H);
        assert_eq!(out.iter().map(|s| s.kind).collect::<Vec<_>>(), vec![Kind::Arrow, Kind::Arrow]);
        // l'asta curva sta sul cerchio
        let on: usize = out[0].points.iter().filter(|q| (dist(**q, (0.4, 0.5)) - 0.15).abs() < 0.003).count();
        assert!(on > 20, "{on}");
    }

    #[test]
    fn tratteggio_allineato() {
        let ds: Vec<Vec<P>> = (0..5).map(|k| {
            let x = 0.1 + k as f32 * 0.06 + if k == 2 { 0.008 } else { 0.0 };
            let y = 0.5 + if k % 2 == 0 { 0.003 } else { -0.002 };
            line((x, y), (x + 0.035 + 0.004 * (k % 2) as f32, y + 0.001), 6)
        }).collect();
        let out = perfect(&ds, &[0, 1, 2, 3, 4], H);
        assert!(out.iter().all(|s| s.kind == Kind::Dash), "{:?}", out.iter().map(|s| s.kind).collect::<Vec<_>>());
        let y0 = out[0].points[0].1;
        assert!(out.iter().all(|s| s.points.iter().all(|q| (q.1 - y0).abs() < 1e-5)));
        let starts: Vec<f32> = out.iter().map(|s| s.points[0].0.min(s.points[s.points.len() - 1].0)).collect();
        let gaps: Vec<f32> = starts.windows(2).map(|w| w[1] - w[0]).collect();
        assert!(gaps.iter().all(|g| (g - gaps[0]).abs() < 1e-4), "{gaps:?}");
    }
}
