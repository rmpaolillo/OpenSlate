//! Modello dati dei tratti e riconoscimento del gesto "zig-zag" (cancella).

/// Colore RGB.
pub type Rgb = [u8; 3];

/// Fase di un punto penna.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PenPhase {
    /// La penna tocca la superficie (inizio tratto).
    Down,
    /// La penna si muove mentre tocca (prosegue il tratto).
    Move,
    /// La penna si stacca (fine tratto).
    Up,
    /// La penna è vicina ma non tocca (hover): non disegna.
    Hover,
}

/// Un singolo campione penna in coordinate del pad.
#[derive(Clone, Copy, Debug)]
pub struct PenEvent {
    pub x: f32,
    pub y: f32,
    /// Pressione normalizzata 0.0..=1.0.
    pub pressure: f32,
    pub phase: PenPhase,
}

/// Firma di una lista di tratti (tutti i punti): due liste con la stessa
/// firma sono lo stesso disegno.
pub fn strokes_sig(strokes: &[Stroke]) -> u64 {
    let mut h: u64 = 1469598103934665603;
    let mut mix = |v: u64| h = (h ^ v).wrapping_mul(1099511628211);
    mix(strokes.len() as u64);
    for s in strokes {
        mix(s.points.len() as u64 ^ ((s.highlight as u64) << 40));
        mix(u32::from_le_bytes([s.color[0], s.color[1], s.color[2], 0]) as u64 ^ (s.width.to_bits() as u64) << 24);
        for p in &s.points {
            mix(p.x.to_bits() as u64 | (p.y.to_bits() as u64) << 32);
        }
    }
    h
}

/// Un tratto continuo (dalla penna giù alla penna su).
#[derive(Clone, Debug)]
pub struct Stroke {
    pub points: Vec<PenEvent>,
    pub color: Rgb,
    /// Spessore relativo: pixel su un foglio alto 800 px (scala col foglio).
    pub width: f32,
    /// Tratto di evidenziatore: va disegnato sotto l'inchiostro.
    pub highlight: bool,
}

impl Stroke {
    pub fn new(color: Rgb, width: f32) -> Self {
        Self {
            points: Vec::new(),
            color,
            width,
            highlight: false,
        }
    }

    pub fn highlighter(color: Rgb, width: f32) -> Self {
        Self {
            highlight: true,
            ..Self::new(color, width)
        }
    }

    pub fn xy(&self) -> Vec<(f32, f32)> {
        self.points.iter().map(|p| (p.x, p.y)).collect()
    }

    fn bbox(&self) -> (f32, f32, f32, f32) {
        bbox(self.points.iter().map(|p| (p.x, p.y)))
    }
}

/// Un disegno: tratti + dimensioni dell'area attiva del pad (unità pad).
#[derive(Clone, Debug, Default)]
pub struct Drawing {
    pub strokes: Vec<Stroke>,
    /// 0 = sconosciuta.
    pub width: f32,
    pub height: f32,
}

impl Drawing {
    pub fn is_empty(&self) -> bool {
        self.strokes.iter().all(|s| s.points.is_empty())
    }
}

/// Riquadro (min_x, min_y, max_x, max_y) di una serie di punti.
pub fn bbox(pts: impl Iterator<Item = (f32, f32)>) -> (f32, f32, f32, f32) {
    let mut b = (f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
    for (x, y) in pts {
        b.0 = b.0.min(x);
        b.1 = b.1.min(y);
        b.2 = b.2.max(x);
        b.3 = b.3.max(y);
    }
    b
}

fn dist(a: (f32, f32), b: (f32, f32)) -> f32 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

/// Ricampiona una polilinea a passo costante.
fn resample(pts: &[(f32, f32)], step: f32) -> Vec<(f32, f32)> {
    let mut out = vec![pts[0]];
    let mut prev = pts[0];
    let mut acc = 0.0;
    for &p in &pts[1..] {
        let mut a = prev;
        let mut d = dist(a, p);
        while acc + d >= step && d > 0.0 {
            let t = (step - acc) / d;
            let q = (a.0 + (p.0 - a.0) * t, a.1 + (p.1 - a.1) * t);
            out.push(q);
            a = q;
            d = dist(a, p);
            acc = 0.0;
        }
        acc += d;
        prev = p;
    }
    out
}

/// Riconosce uno zig-zag: almeno 4 inversioni nette di direzione, tracciato
/// lungo rispetto alla sua estensione, dimensione minima `min_size`.
/// Le lettere stampatello (M, W, Z…) hanno al massimo 3 inversioni.
pub fn is_zigzag(pts: &[(f32, f32)], min_size: f32) -> bool {
    if pts.len() < 8 {
        return false;
    }
    let (x0, y0, x1, y1) = bbox(pts.iter().copied());
    let diag = (x1 - x0).hypot(y1 - y0);
    if diag < min_size {
        return false;
    }
    let len: f32 = pts.windows(2).map(|w| dist(w[0], w[1])).sum();
    if len < 1.5 * diag {
        return false;
    }
    let r = resample(pts, diag / 40.0);
    let k = 2;
    if r.len() < 2 * k + 3 {
        return false;
    }
    let mut turns = 0;
    let mut last: Option<usize> = None;
    for i in k..r.len() - k {
        let a = (r[i].0 - r[i - k].0, r[i].1 - r[i - k].1);
        let b = (r[i + k].0 - r[i].0, r[i + k].1 - r[i].1);
        let (na, nb) = (a.0.hypot(a.1), b.0.hypot(b.1));
        if na == 0.0 || nb == 0.0 {
            continue;
        }
        let cos = (a.0 * b.0 + a.1 * b.1) / (na * nb);
        if cos < -0.5 {
            // angolo di svolta > 120°: inversione netta
            if last.map_or(true, |t| i - t > 2 * k) {
                turns += 1;
            }
            last = Some(i);
        }
    }
    turns >= 4
}

fn segments_cross(p1: (f32, f32), p2: (f32, f32), q1: (f32, f32), q2: (f32, f32)) -> bool {
    let cross = |o: (f32, f32), a: (f32, f32), b: (f32, f32)| {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    };
    let d1 = cross(q1, q2, p1);
    let d2 = cross(q1, q2, p2);
    let d3 = cross(p1, p2, q1);
    let d4 = cross(p1, p2, q2);
    (d1 > 0.0) != (d2 > 0.0) && (d3 > 0.0) != (d4 > 0.0)
}

fn dist_point_seg(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (abx, aby) = (b.0 - a.0, b.1 - a.1);
    let len2 = abx * abx + aby * aby;
    let t = if len2 > 0.0 { (((p.0 - a.0) * abx + (p.1 - a.1) * aby) / len2).clamp(0.0, 1.0) } else { 0.0 };
    (a.0 + abx * t - p.0).hypot(a.1 + aby * t - p.1)
}

fn dist_seg_seg(p1: (f32, f32), p2: (f32, f32), q1: (f32, f32), q2: (f32, f32)) -> f32 {
    if segments_cross(p1, p2, q1, q2) {
        return 0.0;
    }
    dist_point_seg(p1, q1, q2)
        .min(dist_point_seg(p2, q1, q2))
        .min(dist_point_seg(q1, p1, p2))
        .min(dist_point_seg(q2, p1, p2))
}

/// Gomma del mouse: il tratto `s` passa a meno di `tol` (più metà del suo
/// spessore, `half_width`) dal segmento a→b? Unità del pad.
pub fn near_segment(s: &Stroke, a: (f32, f32), b: (f32, f32), tol: f32, half_width: f32) -> bool {
    if s.points.is_empty() {
        return false;
    }
    let r = tol + half_width;
    let (sx0, sy0, sx1, sy1) = s.bbox();
    if sx1 < a.0.min(b.0) - r || sx0 > a.0.max(b.0) + r || sy1 < a.1.min(b.1) - r || sy0 > a.1.max(b.1) + r {
        return false;
    }
    let sp = s.xy();
    if sp.len() == 1 {
        return dist_point_seg(sp[0], a, b) <= r;
    }
    sp.windows(2).any(|w| dist_seg_seg(w[0], w[1], a, b) <= r)
}

/// Il tratto `s` va cancellato dal gesto `g`? Sì se lo attraversa o se sta
/// tutto dentro il riquadro del gesto (puntini, accenti).
pub fn erased_by(s: &Stroke, g: &[(f32, f32)]) -> bool {
    if s.points.is_empty() {
        return false;
    }
    let (gx0, gy0, gx1, gy1) = bbox(g.iter().copied());
    let (sx0, sy0, sx1, sy1) = s.bbox();
    if sx1 < gx0 || sx0 > gx1 || sy1 < gy0 || sy0 > gy1 {
        return false;
    }
    if sx0 >= gx0 && sx1 <= gx1 && sy0 >= gy0 && sy1 <= gy1 {
        return true;
    }
    let sp = s.xy();
    for a in sp.windows(2) {
        for b in g.windows(2) {
            if segments_cross(a[0], a[1], b[0], b[1]) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Polilinea che va avanti e indietro in orizzontale (n passate),
    /// con punti intermedi come farebbe la penna.
    fn zig(n: usize, w: f32, h: f32) -> Vec<(f32, f32)> {
        let v: Vec<(f32, f32)> = (0..=n)
            .map(|i| (if i % 2 == 0 { 0.0 } else { w }, i as f32 * h / n as f32))
            .collect();
        let mut out = Vec::new();
        for s in v.windows(2) {
            for t in 0..10 {
                let t = t as f32 / 10.0;
                out.push((s[0].0 + (s[1].0 - s[0].0) * t, s[0].1 + (s[1].1 - s[0].1) * t));
            }
        }
        out.push(*v.last().unwrap());
        out
    }

    #[test]
    fn riconosce_zigzag() {
        assert!(is_zigzag(&zig(6, 3000.0, 800.0), 1000.0));
        assert!(is_zigzag(&zig(5, 3000.0, 400.0), 1000.0));
        // dente di sega che avanza
        let saw: Vec<(f32, f32)> = (0..=60)
            .map(|i| {
                let ph = (i % 12) as f32;
                let y = if ph < 6.0 { ph * 150.0 } else { (12.0 - ph) * 150.0 };
                (i as f32 * 50.0, y)
            })
            .collect();
        assert!(is_zigzag(&saw, 1000.0));
    }

    #[test]
    fn non_scambia_lettere_e_linee() {
        let line: Vec<(f32, f32)> = (0..50).map(|i| (i as f32 * 100.0, 0.0)).collect();
        assert!(!is_zigzag(&line, 1000.0));
        // W / M: 3 inversioni
        assert!(!is_zigzag(&zig(4, 2000.0, 2000.0), 1000.0));
        let c: Vec<(f32, f32)> = (0..80)
            .map(|i| {
                let a = i as f32 / 80.0 * std::f32::consts::TAU;
                (2000.0 * a.cos(), 2000.0 * a.sin())
            })
            .collect();
        assert!(!is_zigzag(&c, 1000.0));
        // zig-zag minuscolo (dentro una lettera)
        assert!(!is_zigzag(&zig(6, 300.0, 100.0), 1000.0));
    }

    #[test]
    fn cancella_tratti_attraversati() {
        let mut s = Stroke::new([0, 0, 0], 2.0);
        for i in 0..10 {
            s.points.push(PenEvent { x: 1500.0, y: i as f32 * 100.0, pressure: 1.0, phase: PenPhase::Move });
        }
        let g = zig(6, 3000.0, 800.0);
        assert!(erased_by(&s, &g));
        let mut far = Stroke::new([0, 0, 0], 2.0);
        far.points.push(PenEvent { x: 9000.0, y: 9000.0, pressure: 1.0, phase: PenPhase::Down });
        assert!(!erased_by(&far, &g));
    }
}
