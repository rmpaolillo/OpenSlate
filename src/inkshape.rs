//! Il tratto naturale: lo spessore segue la pressione della penna e le punte
//! si affusolano, come l'inchiostro vero. Una sola geometria per tutti: la
//! lavagna (desktop e telefono), PNG/PDF/SVG, la copia, i video.
//!
//! Il tratto diventa un poligono chiuso (lato sinistro, punta finale, lato
//! destro all'indietro, punta iniziale) da riempire. Senza dipendenze: lo
//! usano anche le app per telefono (`mobile/core`).

use std::sync::atomic::{AtomicBool, Ordering};

static NATURAL: AtomicBool = AtomicBool::new(true);

/// Tratto naturale acceso (di serie) o spessore uniforme (effetto pennarello).
pub fn natural() -> bool {
    NATURAL.load(Ordering::Relaxed)
}

pub fn set_natural(on: bool) {
    NATURAL.store(on, Ordering::Relaxed);
}

/// La pressione cambia davvero lungo il tratto? I tratti fatti dal programma
/// (AI, forme, firme sintetiche, bella copia) hanno pressione costante: per
/// loro resta lo spessore uniforme di sempre.
pub fn is_varied(pr: &[f32]) -> bool {
    let mut lo = f32::MAX;
    let mut hi = f32::MIN;
    for &p in pr.iter().filter(|p| **p > 0.0) {
        lo = lo.min(p);
        hi = hi.max(p);
    }
    hi - lo > 0.02
}

/// Le pressioni da mettere nella pagina: vuote se l'opzione è spenta o se la
/// pressione è costante (allora si disegna come sempre).
pub fn page_pressure(pr: impl Iterator<Item = f32>, highlight: bool) -> Vec<f32> {
    if highlight || !natural() {
        return Vec::new();
    }
    let v: Vec<f32> = pr.collect();
    if is_varied(&v) { v } else { Vec::new() }
}

/// Dalla pressione (0..1) al fattore di spessore. Alla pressione tipica
/// della scrittura (~0,75) vale circa 1: il peso medio resta quello di sempre.
fn curve(p: f32) -> f32 {
    0.30 + 0.85 * p.clamp(0.0, 1.0).powf(0.8)
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Il raggio di ogni punto. `width` è lo spessore pieno del tratto nelle
/// stesse unità dei punti.
pub fn radii(pts: &[(f32, f32)], pr: &[f32], width: f32) -> Vec<f32> {
    let half = width * 0.5;
    if !is_varied(pr) || pr.len() != pts.len() {
        return vec![half; pts.len()];
    }
    // pressione lisciata (il sensore trema un po'); il punto di stacco
    // (pressione 0) prende quella di prima
    let mut out = Vec::with_capacity(pts.len());
    let mut ema = pr.iter().copied().find(|p| *p > 0.0).unwrap_or(0.5);
    for &p in pr {
        let p = if p > 0.0 { p } else { ema };
        ema += (p - ema) * 0.35;
        out.push(half * curve(ema));
    }
    // punte affusolate: rampa all'inizio e alla fine, mai più di un terzo
    // del tratto, così punti e tratti corti non spariscono
    let mut acc = vec![0.0f32; pts.len()];
    for i in 1..pts.len() {
        let (ax, ay) = pts[i - 1];
        let (bx, by) = pts[i];
        acc[i] = acc[i - 1] + ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt();
    }
    let total = *acc.last().unwrap_or(&0.0);
    let ramp = (2.5 * width).min(total / 3.0);
    if ramp > 0.0 {
        for (r, &s) in out.iter_mut().zip(&acc) {
            let k = smoothstep(s / ramp).min(smoothstep((total - s) / ramp));
            *r *= 0.3 + 0.7 * k;
        }
    }
    out
}

/// I due lati del tratto e le punte, pronti per chi disegna a triangoli.
#[derive(Debug, Default)]
pub struct Ribbon {
    pub center: Vec<(f32, f32)>,
    pub radius: Vec<f32>,
    pub left: Vec<(f32, f32)>,
    pub right: Vec<(f32, f32)>,
    /// Arco della punta finale, da `left.last()` a `right.last()` (esclusi).
    pub end_cap: Vec<(f32, f32)>,
    /// Arco della punta iniziale, da `right[0]` a `left[0]` (esclusi).
    pub start_cap: Vec<(f32, f32)>,
}

const CAP_STEPS: usize = 8;

fn arc(c: (f32, f32), r: f32, from: f32, sweep: f32, out: &mut Vec<(f32, f32)>) {
    for k in 1..CAP_STEPS {
        let a = from + sweep * k as f32 / CAP_STEPS as f32;
        out.push((c.0 + r * a.cos(), c.1 + r * a.sin()));
    }
}

/// I lati del tratto, dati i punti e il raggio di ciascuno.
pub fn ribbon(pts: &[(f32, f32)], radius: &[f32]) -> Ribbon {
    // via i punti troppo vicini: fanno tacche sui lati
    let mut c: Vec<(f32, f32)> = Vec::with_capacity(pts.len());
    let mut r: Vec<f32> = Vec::with_capacity(pts.len());
    for (i, (&p, &w)) in pts.iter().zip(radius).enumerate() {
        if !(p.0.is_finite() && p.1.is_finite() && w.is_finite()) {
            continue;
        }
        if let Some(&q) = c.last() {
            let d = ((p.0 - q.0).powi(2) + (p.1 - q.1).powi(2)).sqrt();
            let last = i + 1 == pts.len();
            if d < 0.3 * w.max(*r.last().unwrap()) && !(last && d > 0.0) {
                continue;
            }
        }
        c.push(p);
        r.push(w.max(0.0));
    }
    let n = c.len();
    let mut rb = Ribbon::default();
    if n == 0 {
        return rb;
    }
    if n == 1 {
        // un punto: un cerchio (lati vuoti, tutto nella punta)
        let p = c[0];
        rb.left.push((p.0 + r[0], p.1));
        rb.right.push((p.0 + r[0], p.1));
        arc(p, r[0], 0.0, std::f32::consts::TAU, &mut rb.end_cap);
        rb.center = c;
        rb.radius = r;
        return rb;
    }
    let dir = |i: usize| {
        let (a, b) = (c[i.saturating_sub(1)], c[(i + 1).min(n - 1)]);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let l = (dx * dx + dy * dy).sqrt();
        if l > 0.0 { (dx / l, dy / l) } else { (1.0, 0.0) }
    };
    for i in 0..n {
        let (dx, dy) = dir(i);
        let (nx, ny) = (-dy, dx);
        rb.left.push((c[i].0 + nx * r[i], c[i].1 + ny * r[i]));
        rb.right.push((c[i].0 - nx * r[i], c[i].1 - ny * r[i]));
    }
    // la punta finale gira dal lato sinistro al destro passando davanti,
    // quella iniziale dal destro al sinistro passando dietro
    let (dx, dy) = dir(n - 1);
    let a = dy.atan2(dx);
    arc(c[n - 1], r[n - 1], a + std::f32::consts::FRAC_PI_2, -std::f32::consts::PI, &mut rb.end_cap);
    let (dx, dy) = dir(0);
    let a = dy.atan2(dx);
    arc(c[0], r[0], a - std::f32::consts::FRAC_PI_2, -std::f32::consts::PI, &mut rb.start_cap);
    rb.center = c;
    rb.radius = r;
    rb
}

impl Ribbon {
    /// Il poligono chiuso del tratto (da riempire con la regola non-zero).
    pub fn outline(&self) -> Vec<(f32, f32)> {
        if self.center.len() == 1 {
            let mut v = vec![self.left[0]];
            v.extend_from_slice(&self.end_cap);
            return v;
        }
        let mut v = Vec::with_capacity(self.left.len() * 2 + 2 * CAP_STEPS);
        v.extend_from_slice(&self.left);
        v.extend_from_slice(&self.end_cap);
        v.extend(self.right.iter().rev());
        v.extend_from_slice(&self.start_cap);
        v
    }
}

/// Il poligono del tratto naturale: punti, pressioni, spessore pieno.
pub fn outline(pts: &[(f32, f32)], pr: &[f32], width: f32) -> Vec<(f32, f32)> {
    ribbon(pts, &radii(pts, pr, width)).outline()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(n: usize) -> Vec<(f32, f32)> {
        (0..n).map(|i| (i as f32 * 10.0, 0.0)).collect()
    }

    #[test]
    fn pressione_costante_uniforme() {
        let r = radii(&line(30), &[0.5; 30], 8.0);
        assert!(r.iter().all(|&x| (x - 4.0).abs() < 1e-6));
    }

    #[test]
    fn punte_sottili_centro_pieno() {
        let pts = line(60);
        let pr: Vec<f32> = (0..60).map(|i| 0.7 + 0.1 * ((i as f32) * 0.3).sin()).collect();
        let r = radii(&pts, &pr, 8.0);
        assert!(r[0] < r[30] * 0.5, "{} {}", r[0], r[30]);
        assert!(r[59] < r[30] * 0.5);
        // pressione più forte, tratto più largo
        let forte = radii(&pts, &pr.iter().map(|p| p + 0.2).collect::<Vec<_>>(), 8.0);
        assert!(forte[30] > r[30]);
    }

    #[test]
    fn poligono_chiuso_e_finito() {
        let mut pts = line(20);
        pts.insert(5, pts[5]); // un punto doppio
        let pr: Vec<f32> = (0..21).map(|i| 0.2 + i as f32 * 0.03).collect();
        let o = outline(&pts, &pr, 6.0);
        assert!(o.len() > 20);
        assert!(o.iter().all(|p| p.0.is_finite() && p.1.is_finite()));
        // un punto solo: un cerchio
        let o = outline(&[(5.0, 5.0)], &[0.5], 4.0);
        assert_eq!(o.len(), CAP_STEPS);
        assert!(o.iter().all(|p| ((p.0 - 5.0).powi(2) + (p.1 - 5.0).powi(2)).sqrt() - 2.0 < 1e-4));
        assert!(outline(&[], &[], 4.0).is_empty());
    }

    #[test]
    fn spento_niente_pressione() {
        assert!(page_pressure([0.2, 0.9].into_iter(), true).is_empty());
        assert!(page_pressure([0.5, 0.5].into_iter(), false).is_empty());
    }
}
