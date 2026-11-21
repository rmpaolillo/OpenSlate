//! La pagina del telefono in diretta sulla desktop (lo «specchio»): quello
//! che si scrive sul telefono, col pad o con la penna sullo schermo, compare
//! subito in una finestra della desktop. Si guarda soltanto: la pagina resta
//! del telefono e arriva nell'archivio con la sincronizzazione di sempre.
//!
//! Il telefono (`Tracker`) manda solo quello che è cambiato dall'ultima volta:
//! «dal tratto `from` in poi i tratti sono questi», oppure «il tratto `from`
//! continua con questi punti» (`grow`) mentre lo si scrive. La desktop
//! (`Mirror`) li applica alla sua copia. Ogni messaggio ha un numero (`seq`)
//! dentro una «generazione» (`gen`): se se ne perde uno, la desktop risponde
//! di no e il telefono rimanda la pagina intera con una generazione nuova.
//!
//! Viaggia come il resto della sincronizzazione, cifrato col segreto del
//! telefono: `POST /sync/v1/live` (i tratti, in JSON) e `POST
//! /sync/v1/live/bg` (la pagina del documento sotto i tratti, un'immagine).

use crate::docstore::{ink_to_strokes, InkFmt};
use crate::stroke::{PenEvent, PenPhase, Rgb, Stroke};
use serde::{Deserialize, Serialize};

/// Com'è il foglio del telefono.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Meta {
    /// Titolo della pagina o del documento.
    #[serde(default)]
    pub title: String,
    /// Area del pad (unità del pad).
    pub pad_w: f32,
    pub pad_h: f32,
    /// Quarti di giro, come «Ruota» dell'app.
    #[serde(default)]
    pub quarter: u32,
    /// Il foglio (`Paper::code`): sotto un documento non si disegna.
    #[serde(default)]
    pub paper: String,
    #[serde(default)]
    pub margin: bool,
}

/// Un aggiornamento.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Msg {
    pub gen: u64,
    pub seq: u64,
    /// Primo tratto che cambia.
    #[serde(default)]
    pub from: usize,
    /// Il tratto `from` continua: aveva tanti punti, il primo di `strokes`
    /// sono i punti nuovi; gli altri vengono dopo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grow: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub strokes: Vec<InkFmt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,
    /// Il telefono smette (diretta spenta, app chiusa).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub end: bool,
}

impl Msg {
    /// La pagina intera (si riparte da capo).
    pub fn is_full(&self) -> bool {
        self.from == 0 && self.grow.is_none() && self.meta.is_some()
    }
}

/// Lo sfondo: generazione, riquadro (u, v, larghezza, altezza in 0..1) e
/// immagine (PNG o JPEG; vuota = nessuno sfondo), in un corpo solo.
pub fn bg_encode(gen: u64, rect: [f32; 4], image: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(24 + image.len());
    v.extend_from_slice(&gen.to_le_bytes());
    for f in rect {
        v.extend_from_slice(&f.to_le_bytes());
    }
    v.extend_from_slice(image);
    v
}

pub fn bg_decode(b: &[u8]) -> Option<(u64, [f32; 4], &[u8])> {
    if b.len() < 24 {
        return None;
    }
    let gen = u64::from_le_bytes(b[..8].try_into().ok()?);
    let mut r = [0f32; 4];
    for (i, f) in r.iter_mut().enumerate() {
        *f = f32::from_le_bytes(b[8 + 4 * i..12 + 4 * i].try_into().ok()?);
    }
    Some((gen, r, &b[24..]))
}

// ---------------------------------------------------------------------------
// Desktop: la copia della pagina
// ---------------------------------------------------------------------------

/// La pagina del telefono vista dalla desktop.
#[derive(Default)]
pub struct Mirror {
    /// Il telefono che scrive (id dell'abbinamento).
    pub peer: String,
    pub gen: u64,
    seq: u64,
    pub strokes: Vec<Stroke>,
    pub meta: Meta,
    /// L'immagine sotto i tratti e dove sta.
    pub bg: Option<(Vec<u8>, [f32; 4])>,
    /// Cambia a ogni sfondo nuovo (per rifare la texture).
    pub bg_rev: u64,
    /// Cambia a ogni aggiornamento.
    pub rev: u64,
    /// Il telefono ha smesso.
    pub ended: bool,
    /// Ultimo messaggio (secondi Unix).
    pub last: i64,
}

impl Mirror {
    /// Applica un aggiornamento; `false` = non torna (ne manca uno): il
    /// telefono rimanda tutto.
    pub fn apply(&mut self, peer: &str, m: Msg, now: i64) -> bool {
        let follows = peer == self.peer && m.gen == self.gen && m.seq == self.seq + 1;
        if !follows {
            if !m.is_full() {
                return false;
            }
            if peer != self.peer || m.gen != self.gen {
                // un'altra diretta: lo sfondo lo rimanda il telefono
                self.bg = None;
                self.bg_rev += 1;
            }
            self.peer = peer.to_string();
            self.gen = m.gen;
            self.strokes.clear();
        }
        if let Some(off) = m.grow {
            let mut rest = m.strokes.into_iter();
            let (Some(s), Some(tail)) = (self.strokes.get_mut(m.from), rest.next()) else { return false };
            if s.points.len() != off {
                return false;
            }
            if let Some(last) = s.points.last_mut() {
                last.phase = PenPhase::Move;
            }
            s.points.extend(tail.p.iter().map(|p| PenEvent { x: p[0], y: p[1], pressure: p[2], phase: PenPhase::Move }));
            self.strokes.truncate(m.from + 1);
            self.strokes.extend(ink_to_strokes(rest.collect()));
        } else {
            if m.from > self.strokes.len() {
                return false;
            }
            self.strokes.truncate(m.from);
            self.strokes.extend(ink_to_strokes(m.strokes));
        }
        if let Some(meta) = m.meta {
            self.meta = meta;
        }
        self.seq = m.seq;
        self.ended = m.end;
        self.last = now;
        self.rev += 1;
        true
    }

    /// Lo sfondo nuovo; `false` = è di un'altra diretta.
    pub fn set_bg(&mut self, peer: &str, body: &[u8]) -> bool {
        let Some((gen, rect, img)) = bg_decode(body) else { return false };
        if peer != self.peer || gen != self.gen {
            return false;
        }
        self.bg = (!img.is_empty()).then(|| (img.to_vec(), rect));
        self.bg_rev += 1;
        self.rev += 1;
        true
    }
}

// ---------------------------------------------------------------------------
// Telefono: che cosa è cambiato
// ---------------------------------------------------------------------------

/// Un tratto in breve: basta per capire se è ancora lui e se è cresciuto.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Sig {
    len: usize,
    first: (u32, u32),
    color: Rgb,
    width: u32,
    highlight: bool,
    stamp: bool,
}

impl Sig {
    fn of(s: &Stroke) -> Sig {
        let first = s.points.first().map(|p| (p.x.to_bits(), p.y.to_bits())).unwrap_or_default();
        Sig { len: s.points.len(), first, color: s.color, width: s.width.to_bits(), highlight: s.highlight, stamp: s.stamp.is_some() }
    }

    /// Lo stesso tratto (al più con altri punti in fondo).
    fn same(&self, o: &Sig) -> bool {
        Sig { len: 0, ..*self } == Sig { len: 0, ..*o }
    }
}

fn ink(s: &Stroke, from: usize) -> InkFmt {
    InkFmt {
        c: format!("#{:02x}{:02x}{:02x}", s.color[0], s.color[1], s.color[2]),
        w: s.width,
        p: s.points[from.min(s.points.len())..].iter().map(|p| [p.x, p.y, p.pressure]).collect(),
        h: s.highlight,
        t: s.stamp.as_deref().cloned(),
    }
}

/// Quello che la desktop ha già (secondo il telefono).
pub struct Tracker {
    gen: u64,
    seq: u64,
    sent: Vec<Sig>,
    meta: Option<Meta>,
    full: bool,
}

impl Default for Tracker {
    fn default() -> Self {
        Self::new()
    }
}

impl Tracker {
    pub fn new() -> Tracker {
        Tracker { gen: 0, seq: 0, sent: Vec::new(), meta: None, full: true }
    }

    pub fn gen(&self) -> u64 {
        self.gen
    }

    /// Qualcosa non è arrivato: la prossima volta la pagina intera.
    pub fn reset(&mut self) {
        self.full = true;
    }

    /// Il prossimo aggiornamento, se c'è qualcosa da dire (`beat`: anche
    /// vuoto, per dire che il telefono c'è ancora). Il telefono considera
    /// arrivato quello che manda: se non arriva, chiama `reset`.
    pub fn next(&mut self, strokes: &[Stroke], meta: &Meta, beat: bool) -> Option<Msg> {
        let sigs: Vec<Sig> = strokes.iter().map(Sig::of).collect();
        let mut m = Msg::default();
        if self.full {
            self.full = false;
            self.gen = rand_u64();
            self.seq = 0;
            m.strokes = strokes.iter().filter(|s| !s.points.is_empty()).map(|s| ink(s, 0)).collect();
            m.meta = Some(meta.clone());
            // i tratti vuoti non viaggiano: la copia della desktop non li ha
            self.sent = sigs.into_iter().filter(|s| s.len > 0).collect();
        } else {
            let sigs: Vec<Sig> = sigs.into_iter().filter(|s| s.len > 0).collect();
            let live: Vec<&Stroke> = strokes.iter().filter(|s| !s.points.is_empty()).collect();
            let differs = (0..sigs.len().min(self.sent.len())).find(|&i| sigs[i] != self.sent[i]);
            match differs {
                Some(i) if i + 1 == self.sent.len() && sigs[i].same(&self.sent[i]) && sigs[i].len > self.sent[i].len => {
                    m.from = i;
                    m.grow = Some(self.sent[i].len);
                    m.strokes.push(ink(live[i], self.sent[i].len));
                    m.strokes.extend(live[i + 1..].iter().map(|s| ink(s, 0)));
                }
                Some(i) => {
                    m.from = i;
                    m.strokes = live[i..].iter().map(|s| ink(s, 0)).collect();
                }
                None => {
                    let n = self.sent.len().min(sigs.len());
                    m.from = n;
                    m.strokes = live[n..].iter().map(|s| ink(s, 0)).collect();
                    let changed = sigs.len() != self.sent.len();
                    if !changed && self.meta.as_ref() == Some(meta) && !beat {
                        return None;
                    }
                }
            }
            if self.meta.as_ref() != Some(meta) {
                m.meta = Some(meta.clone());
            }
            self.sent = sigs;
        }
        self.meta = Some(meta.clone());
        self.seq += 1;
        m.gen = self.gen;
        m.seq = self.seq;
        Some(m)
    }

    /// L'ultimo messaggio: il telefono smette.
    pub fn end(&mut self) -> Msg {
        self.seq += 1;
        Msg { gen: self.gen, seq: self.seq, from: self.sent.len(), end: true, ..Default::default() }
    }
}

fn rand_u64() -> u64 {
    let mut b = [0u8; 8];
    let _ = getrandom::fill(&mut b);
    // mai 0: è la generazione di una copia vuota
    u64::from_le_bytes(b).max(1)
}

// ---------------------------------------------------------------------------
// Telefono: il filo che manda
// ---------------------------------------------------------------------------

/// Quello che l'app dice al filo che manda (e il filo all'app).
#[derive(Default)]
pub struct Feed {
    pub meta: Meta,
    /// Lo sfondo (immagine PNG, riquadro); vuoto = nessuno.
    pub bg: Option<(Vec<u8>, [f32; 4])>,
    /// Lo sfondo è cambiato: va rimandato.
    pub bg_dirty: bool,
    /// Basta: il filo saluta e finisce.
    pub stop: bool,
    /// La desktop risponde.
    pub connected: bool,
}

/// Ogni quanto si guarda se la pagina è cambiata.
const TICK: std::time::Duration = std::time::Duration::from_millis(40);
/// Ogni quanto un segno di vita anche senza novità.
const BEAT: std::time::Duration = std::time::Duration::from_secs(4);

/// Manda la pagina del telefono alla desktop finché `feed.stop`. `next`
/// chiede al `Tracker` l'aggiornamento sulla pagina di adesso (l'app ci mette
/// i suoi tratti). Se la desktop non risponde si riprova ogni pochi secondi.
pub fn send_loop(
    pairing: &super::pair::Pairing,
    found: &[String],
    device: &str,
    feed: &std::sync::Mutex<Feed>,
    next: &mut dyn FnMut(&mut Tracker, &Meta, bool) -> Option<Msg>,
) {
    use std::time::{Duration, Instant};
    let lock = || feed.lock().unwrap_or_else(|e| e.into_inner());
    let mut tracker = Tracker::new();
    let mut conn: Option<super::client::Conn> = None;
    let mut retry = Instant::now();
    let mut last = Instant::now();
    while !lock().stop {
        if conn.is_none() {
            if Instant::now() < retry {
                std::thread::sleep(TICK);
                continue;
            }
            match super::client::connect(pairing, found, device, Duration::from_secs(2), Duration::from_secs(15)) {
                Ok((_, c)) => {
                    conn = Some(c);
                    tracker.reset();
                    let mut f = lock();
                    f.connected = true;
                    f.bg_dirty = true;
                }
                Err(_) => {
                    lock().connected = false;
                    retry = Instant::now() + Duration::from_secs(5);
                    continue;
                }
            }
        }
        let Some(c) = conn.as_ref() else { continue };
        let meta = lock().meta.clone();
        let mut res: Result<(), String> = Ok(());
        if let Some(m) = next(&mut tracker, &meta, last.elapsed() > BEAT) {
            let full = m.is_full();
            res = c.call("POST", "/sync/v1/live", 0, &serde_json::to_vec(&m).unwrap_or_default()).map(|_| ());
            if res.is_ok() {
                last = Instant::now();
                if full {
                    // una diretta nuova per la desktop: lo sfondo di nuovo
                    lock().bg_dirty = true;
                }
            }
        }
        if res.is_ok() {
            let bg = {
                let mut f = lock();
                std::mem::take(&mut f.bg_dirty).then(|| f.bg.clone())
            };
            if let Some(bg) = bg {
                let (img, rect) = bg.unwrap_or_default();
                res = c.call("POST", "/sync/v1/live/bg", 0, &bg_encode(tracker.gen(), rect, &img)).map(|_| ());
                if res.is_err() {
                    lock().bg_dirty = true;
                }
            }
        }
        if let Err(e) = res {
            tracker.reset();
            // 409: la desktop c'è ma vuole tutto; altrimenti la si ricerca
            if !e.starts_with("409") {
                conn = None;
                lock().connected = false;
                retry = Instant::now() + Duration::from_secs(2);
            }
        }
        std::thread::sleep(TICK);
    }
    if let Some(c) = conn {
        let _ = c.call("POST", "/sync/v1/live", 0, &serde_json::to_vec(&tracker.end()).unwrap_or_default());
    }
    lock().connected = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: f32, phase: PenPhase) -> PenEvent {
        PenEvent { x, y: x * 2.0, pressure: 0.5, phase }
    }

    fn stroke(xs: &[f32], color: Rgb) -> Stroke {
        let mut s = Stroke::new(color, 2.0);
        s.points = xs.iter().map(|&x| pt(x, PenPhase::Move)).collect();
        s
    }

    fn meta() -> Meta {
        Meta { title: "Pagina".into(), pad_w: 21600.0, pad_h: 14800.0, quarter: 1, paper: "righe".into(), margin: true }
    }

    fn xs(v: &[Stroke]) -> Vec<Vec<f32>> {
        v.iter().map(|s| s.points.iter().map(|p| p.x).collect()).collect()
    }

    /// Ogni messaggio passa per JSON, come in rete.
    fn send(t: &mut Tracker, m: &mut Mirror, page: &[Stroke], beat: bool) -> Option<bool> {
        let msg = t.next(page, &meta(), beat)?;
        let msg: Msg = serde_json::from_str(&serde_json::to_string(&msg).unwrap()).unwrap();
        Some(m.apply("tel", msg, 0))
    }

    #[test]
    fn la_copia_segue_la_pagina() {
        let (mut t, mut m) = (Tracker::new(), Mirror::default());
        let mut page = vec![stroke(&[1.0, 2.0], [0, 0, 0])];
        assert_eq!(send(&mut t, &mut m, &page, false), Some(true));
        assert_eq!(m.meta, meta());
        // niente di nuovo: niente da dire (se non il battito)
        assert_eq!(send(&mut t, &mut m, &page, false), None);
        assert_eq!(send(&mut t, &mut m, &page, true), Some(true));
        // un tratto nuovo che cresce mentre lo si scrive
        page.push(stroke(&[5.0], [200, 0, 0]));
        assert_eq!(send(&mut t, &mut m, &page, false), Some(true));
        page[1].points.push(pt(6.0, PenPhase::Move));
        page[1].points.push(pt(7.0, PenPhase::Up));
        let msg = t.next(&page, &meta(), false).unwrap();
        assert_eq!((msg.from, msg.grow, msg.strokes[0].p.len()), (1, Some(1), 2), "solo i punti nuovi");
        assert!(m.apply("tel", msg, 0));
        assert_eq!(xs(&m.strokes), xs(&page));
        // annulla: l'ultimo tratto sparisce
        page.pop();
        assert_eq!(send(&mut t, &mut m, &page, false), Some(true));
        assert_eq!(xs(&m.strokes), xs(&page));
        // la gomma toglie un tratto in mezzo, poi se ne scrive un altro
        page = vec![stroke(&[1.0, 2.0], [0, 0, 0]), stroke(&[3.0], [0, 0, 0]), stroke(&[4.0], [0, 0, 0])];
        send(&mut t, &mut m, &page, false);
        page.remove(1);
        page.push(stroke(&[9.0, 9.5], [0, 0, 200]));
        assert_eq!(send(&mut t, &mut m, &page, false), Some(true));
        assert_eq!(xs(&m.strokes), xs(&page));
        assert_eq!(m.strokes[2].color, [0, 0, 200]);
        // pagina nuova (vuota)
        assert_eq!(send(&mut t, &mut m, &[], false), Some(true));
        assert!(m.strokes.is_empty());
    }

    #[test]
    fn un_messaggio_perso_fa_rimandare_tutto() {
        let (mut t, mut m) = (Tracker::new(), Mirror::default());
        let mut page = vec![stroke(&[1.0], [0, 0, 0])];
        send(&mut t, &mut m, &page, false);
        page.push(stroke(&[2.0], [0, 0, 0]));
        t.next(&page, &meta(), false); // perso
        page.push(stroke(&[3.0], [0, 0, 0]));
        assert_eq!(send(&mut t, &mut m, &page, false), Some(false), "ne manca uno");
        t.reset();
        assert_eq!(send(&mut t, &mut m, &page, false), Some(true));
        assert_eq!(xs(&m.strokes), xs(&page));
    }

    #[test]
    fn la_desktop_riavviata_chiede_tutto() {
        let (mut t, mut m) = (Tracker::new(), Mirror::default());
        let mut page = vec![stroke(&[1.0], [0, 0, 0])];
        send(&mut t, &mut m, &page, false);
        let mut fresh = Mirror::default();
        page.push(stroke(&[2.0], [0, 0, 0]));
        assert_eq!(send(&mut t, &mut fresh, &page, false), Some(false));
        t.reset();
        assert_eq!(send(&mut t, &mut fresh, &page, false), Some(true));
        assert_eq!(xs(&fresh.strokes), xs(&page));
    }

    #[test]
    fn lo_sfondo_va_con_la_sua_diretta() {
        let (mut t, mut m) = (Tracker::new(), Mirror::default());
        send(&mut t, &mut m, &[], false);
        assert!(m.set_bg("tel", &bg_encode(t.gen(), [0.0, 0.1, 1.0, 0.8], b"png")));
        assert_eq!(m.bg.as_ref().map(|b| (b.0.as_slice(), b.1)), Some((&b"png"[..], [0.0, 0.1, 1.0, 0.8])));
        assert!(!m.set_bg("tel", &bg_encode(t.gen() ^ 1, [0.0; 4], b"x")), "di un'altra diretta");
        assert!(m.set_bg("tel", &bg_encode(t.gen(), [0.0; 4], b"")));
        assert!(m.bg.is_none());
        // fine della diretta
        let end = t.end();
        assert!(m.apply("tel", end, 0) && m.ended);
    }

    #[test]
    fn diretta_vera_su_127_0_0_1() {
        use super::super::pair::{secret_b64, Pairing};
        use super::super::server::{serve_conn, PeerKey, Server};
        use super::super::store::FsStore;
        use std::sync::{Arc, Mutex};
        let root = std::env::temp_dir().join(format!("openslate-live-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&root);
        let key = super::super::crypto::new_secret();
        let mirror = Arc::new(Mutex::new(Mirror::default()));
        let mut srv = Server::new("D", "Mac");
        let mm = mirror.clone();
        srv.live = Some(Box::new(move |peer, what, body| {
            let mut m = mm.lock().unwrap();
            let ok = if what == "live" { m.apply(peer, serde_json::from_slice(body).unwrap(), 0) } else { m.set_bg(peer, body) };
            if ok { 200 } else { 409 }
        }));
        let state = Arc::new(Mutex::new((srv, FsStore::desktop(&root))));
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for s in l.incoming().flatten() {
                let st = state.clone();
                std::thread::spawn(move || {
                    let _ = serve_conn(s, |req| {
                        let mut g = st.lock().unwrap();
                        let (srv, store) = &mut *g;
                        srv.handle(&[PeerKey { id: "T".into(), key }], store, req, super::super::now())
                    });
                });
            }
        });
        let p = Pairing { desk_id: "D".into(), desk_name: "Mac".into(), hosts: vec!["127.0.0.1".into()], port, peer_id: "T".into(), secret: secret_b64(&key) };
        let feed = Arc::new(Mutex::new(Feed { meta: meta(), bg: Some((b"png".to_vec(), [0.0, 0.0, 1.0, 1.0])), ..Default::default() }));
        let page = Arc::new(Mutex::new(vec![stroke(&[1.0, 2.0], [0, 0, 0])]));
        let (f2, p2) = (feed.clone(), page.clone());
        let h = std::thread::spawn(move || {
            send_loop(&p, &[], "Pixel", &f2, &mut |t, m, beat| t.next(&p2.lock().unwrap(), m, beat));
        });
        let wait = |cond: &dyn Fn(&Mirror) -> bool| {
            for _ in 0..200 {
                if cond(&mirror.lock().unwrap()) {
                    return true;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            false
        };
        assert!(wait(&|m| m.strokes.len() == 1 && m.bg.is_some()), "pagina e sfondo arrivati");
        page.lock().unwrap().push(stroke(&[3.0, 4.0, 5.0], [255, 0, 0]));
        assert!(wait(&|m| m.strokes.len() == 2 && m.strokes[1].points.len() == 3));
        // la desktop dimentica tutto (riavviata): il telefono rimanda la pagina
        *mirror.lock().unwrap() = Mirror::default();
        page.lock().unwrap().push(stroke(&[6.0], [0, 0, 0]));
        assert!(wait(&|m| m.strokes.len() == 3 && m.bg.is_some()), "di nuovo tutto, sfondo compreso");
        feed.lock().unwrap().stop = true;
        h.join().unwrap();
        assert!(mirror.lock().unwrap().ended, "il telefono ha salutato");
    }
}
