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
/// unità della pagina). `big` dice quali sono abbastanza grandi da lisciare
/// se non sono una forma riconosciuta.
pub fn perfect(strokes: &[Vec<P>], cand: &[usize], hh: f32) -> Vec<Shaped> {
    let hh = hh.max(1e-4);
    let mut out: Vec<Shaped> = Vec::new();
    let mut verts: Vec<P> = Vec::new();
    let mut chains: Vec<Chain> = Vec::new();
    let mut rest: Vec<usize> = Vec::new();
    for &i in cand {
        let raw = &strokes[i];
        if raw.len() < 3 || diag(raw) < 0.6 * hh {
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
        if let Some((a, b)) = straight(p) {
            if dist(a, b) >= 0.8 * hh {
                let v0 = push(&mut verts, a);
                let v1 = push(&mut verts, b);
                chains.push(Chain { index: i, verts: vec![v0, v1], closed: false });
                continue;
            }
        }
        if let Some(pts) = arrow(p, hh) {
            out.push(Shaped { index: i, kind: Kind::Arrow, points: pts });
            continue;
        }
        if let Some((corners, closed)) = polyline(p, hh) {
            // una «V» piccola: forse la punta di una freccia (si vede dopo)
            if !closed && corners.len() == 3 && diag(p) <= 1.8 * hh {
                rest.push(i);
                continue;
            }
            let vs: Vec<usize> = corners.into_iter().map(|q| push(&mut verts, q)).collect();
            chains.push(Chain { index: i, verts: vs, closed });
            continue;
        }
        if let Some(pts) = ellipse(p) {
            out.push(Shaped { index: i, kind: Kind::Ellipse, points: pts });
            continue;
        }
        if let Some(pts) = arc(p) {
            out.push(Shaped { index: i, kind: Kind::Arc, points: pts });
            continue;
        }
        rest.push(i);
    }
    join_and_snap(&mut verts, &chains, hh);
    // le punte disegnate a parte: una «V» piccola vicino al capo di una linea
    let segs: Vec<(P, P)> = chains
        .iter()
        .filter(|c| !c.closed)
        .flat_map(|c| {
            let n = c.verts.len();
            [(verts[c.verts[1]], verts[c.verts[0]]), (verts[c.verts[n - 2]], verts[c.verts[n - 1]])]
        })
        .collect();
    for i in rest {
        let p = &strokes[i];
        if diag(p) <= 1.8 * hh {
            if let Some(pts) = arrow_head(p, &segs, hh) {
                out.push(Shaped { index: i, kind: Kind::Arrow, points: pts });
                continue;
            }
        }
        if diag(p) >= 2.0 * hh {
            out.push(Shaped { index: i, kind: Kind::Curve, points: smooth_curve(p, hh) });
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

/// Un tratto grande che è chiaramente una forma (linea, freccia, spezzata,
/// poligono, cerchio, arco): un disegno, non scrittura da far leggere.
pub fn is_shape(p: &[P], hh: f32) -> bool {
    let hh = hh.max(1e-4);
    if p.len() < 3 || diag(p) < 2.0 * hh {
        return false;
    }
    let p = &denoise(p, hh)[..];
    straight(p).is_some() || arrow(p, hh).is_some() || polyline(p, hh).is_some() || ellipse(p).is_some() || arc(p).is_some()
}

fn push(verts: &mut Vec<P>, q: P) -> usize {
    verts.push(q);
    verts.len() - 1
}

/// I capi vicini si uniscono, le linee quasi orizzontali o verticali si
/// raddrizzano (i vertici restano condivisi), un capo vicino a un'altra
/// linea ci si appoggia.
fn join_and_snap(verts: &mut [P], chains: &[Chain], hh: f32) {
    let tol = 0.45 * hh;
    // i capi delle catene aperte
    let ends: Vec<usize> = chains.iter().filter(|c| !c.closed).flat_map(|c| [c.verts[0], *c.verts.last().unwrap()]).collect();
    // gruppi di capi vicini (di catene diverse o della stessa, se si chiude)
    let mut group: Vec<usize> = (0..verts.len()).collect();
    fn root(g: &mut Vec<usize>, i: usize) -> usize {
        let mut r = i;
        while g[r] != r {
            r = g[r];
        }
        g[i] = r;
        r
    }
    for (a, &va) in ends.iter().enumerate() {
        for &vb in &ends[a + 1..] {
            if dist(verts[va], verts[vb]) < tol {
                let (ra, rb) = (root(&mut group, va), root(&mut group, vb));
                group[ra] = rb;
            }
        }
    }
    let mut sum: Vec<(f32, f32, f32)> = vec![(0.0, 0.0, 0.0); verts.len()];
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
    // le linee: coppie di vertici; quelli uniti sono «uguali»
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
    let snap = 6f32.to_radians();
    for _ in 0..3 {
        for &(a, b) in &segs {
            let (pa, pb) = (verts[a], verts[b]);
            let ang = (pb.1 - pa.1).atan2(pb.0 - pa.0);
            let near = (ang / FRAC_PI_2).round() * FRAC_PI_2;
            if (ang - near).abs() >= snap {
                continue;
            }
            let horizontal = (near / FRAC_PI_2).round() as i32 % 2 == 0;
            let (ra, rb) = (root(&mut group, a), root(&mut group, b));
            let members: Vec<usize> = (0..verts.len()).filter(|&v| v == a || v == b || (ends.contains(&v) && (root(&mut group, v) == ra || root(&mut group, v) == rb))).collect();
            if horizontal {
                let y = (pa.1 + pb.1) / 2.0;
                for v in members {
                    verts[v].1 = y;
                }
            } else {
                let x = (pa.0 + pb.0) / 2.0;
                for v in members {
                    verts[v].0 = x;
                }
            }
        }
    }
    // un capo rimasto solo vicino a un'altra linea: ci si appoggia (una «T»)
    for &v in &ends {
        let r = root(&mut group, v);
        let alone = ends.iter().filter(|&&w| root(&mut group, w) == r).count() == 1;
        if !alone {
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
        let dev = part.iter().map(|&q| seg_dist(q, a, b)).fold(0.0f32, f32::max);
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
    sharp.then_some((v, closed))
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

/// Un cerchio o un'ellisse chiusa: la stessa figura perfetta (un cerchio se
/// è quasi tonda, dritta se è quasi dritta), dallo stesso punto di partenza
/// e nello stesso verso.
fn ellipse(p: &[P]) -> Option<Vec<P>> {
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
    let (st, ct) = theta.sin_cos();
    let local = |&(x, y): &P| {
        let (dx, dy) = (x - cx, y - cy);
        (dx * ct + dy * st, -dx * st + dy * ct)
    };
    let (mut err, mut worst, mut sweep) = (0.0f32, 0.0f32, 0.0f32);
    let mut prev: Option<f32> = None;
    for pt in &q {
        let (u, v) = local(pt);
        let r = ((u / a).powi(2) + (v / b).powi(2)).sqrt();
        err += (r - 1.0).abs() / 64.0;
        worst = worst.max((r - 1.0).abs());
        let ang = v.atan2(u);
        if let Some(pa) = prev {
            sweep += wrap(ang - pa);
        }
        prev = Some(ang);
    }
    if err > 0.12 || worst > 0.35 || sweep.abs() < 300f32.to_radians() {
        return None;
    }
    let (a, b, theta) = if b / a > 0.85 {
        let r = (a + b) / 2.0;
        (r, r, 0.0)
    } else {
        let near = (theta / FRAC_PI_2).round() * FRAC_PI_2;
        (a, b, if (theta - near).abs() < 6f32.to_radians() { near } else { theta })
    };
    let (st, ct) = theta.sin_cos();
    let (u0, v0) = {
        let (dx, dy) = (p[0].0 - cx, p[0].1 - cy);
        (dx * ct + dy * st, -dx * st + dy * ct)
    };
    let start = (v0 / b).atan2(u0 / a);
    let dir = sweep.signum();
    let n = 72;
    Some(
        (0..=n)
            .map(|k| {
                let t = start + dir * TAU * k as f32 / n as f32;
                let (u, v) = (a * t.cos(), b * t.sin());
                (cx + u * ct - v * st, cy + u * st + v * ct)
            })
            .collect(),
    )
}

/// Un arco di cerchio aperto (da 30° a 320°): lo stesso arco perfetto.
fn arc(p: &[P]) -> Option<Vec<P>> {
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
    Some(
        (0..=n)
            .map(|k| {
                let t = start + sweep * k as f32 / n as f32;
                (cx + r * t.cos(), cy + r * t.sin())
            })
            .collect(),
    )
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
/// coi vicini (su poco più di mezza lettera); i capi restano dove sono.
fn denoise(p: &[P], hh: f32) -> Vec<P> {
    let total: f32 = p.windows(2).map(|w| dist(w[0], w[1])).sum();
    let n = ((total / (0.1 * hh)).ceil() as usize).clamp(8, 600);
    let mut q = resample(p, n);
    let k = 3usize;
    for _ in 0..2 {
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
}
