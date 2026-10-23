//! La chat AI sul foglio della lavagna:
//! - `lavagna_leggi`: il foglio a righe e parole, ognuna col suo nome e il suo
//!   riquadro (vedi `boardread`), più l'immagine con i nomi;
//! - `lavagna_annota`: evidenzia, sottolinea, cerchia, frecce, testo a mano…
//!   (vedi `boardmarks`): tratti come quelli del pennino;
//! - `lavagna_modifica`: sposta, ridimensiona, ricolora o cancella quello che
//!   c'è (anche quello che hai scritto tu), ma solo se lo chiedi; per
//!   cancellare buona parte del foglio serve la tua conferma nella chat;
//! - `lavagna_inserisci_grafico`: un grafico fatto da `draw_chart` sul foglio,
//!   anche al posto di un disegno fatto a mano (che si trova da solo: le righe
//!   lunghe, come gli assi, e le scritte lì attorno). Il grafico diventa
//!   l'immagine sotto i tratti, i tratti a mano che sostituisce si tolgono.
//!
//! Ogni modifica si annulla dalla chat («Annulla» rimette il foglio com'era e
//! lascia quello che hai scritto dopo) e con Cmd+Z.

use crate::scene::{Hub, PageBg};
use crate::stroke::{strokes_sig, Stroke};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

static HUB: OnceLock<Arc<Hub>> = OnceLock::new();

/// La lavagna dell'app (all'avvio).
pub fn set_hub(hub: Arc<Hub>) {
    let _ = HUB.set(hub);
}

/// Per le prove: la lavagna di tutte le prove, col foglio vuoto.
#[cfg(test)]
pub fn test_hub() -> Arc<Hub> {
    let hub = HUB.get_or_init(|| Hub::new(crate::scene::Scene::new(&crate::settings::Settings::default()))).clone();
    hub.edit(|s| s.set_page(Vec::new(), None));
    hub
}

/// Una modifica della chat al foglio, per «Annulla».
struct Edit {
    n: u64,
    /// Il foglio prima: tratti e sfondo.
    before: Vec<Stroke>,
    before_bg: Option<PageBg>,
    /// Dopo: quanti tratti c'erano e la loro firma, lo sfondo (la sua chiave).
    kept: usize,
    kept_sig: u64,
    bg_key: Option<String>,
}

static UNDO: Mutex<Vec<Edit>> = Mutex::new(Vec::new());

/// L'ultima lettura del foglio: i nomi (`r2p3`) che il modello ha visto.
static LAST: Mutex<Option<crate::boardread::Reading>> = Mutex::new(None);

/// Le impronte dei tratti messi dall'AI sul foglio: il controllo mentre
/// scrivi non li prende per scrittura nuova, e toglierli o rifarli non
/// chiede la conferma dell'utente. Restano anche dopo un riavvio (file
/// `ai_marks.txt` nella cartella dei dati).
static AI_STROKES: Mutex<Option<Vec<u64>>> = Mutex::new(None);
const AI_FILE: &str = "ai_marks.txt";
const AI_KEEP: usize = 20_000;

/// L'impronta di un tratto, a prova di salvataggio: coordinate arrotondate
/// (l'archivio le salva con tre decimali).
fn ai_sig(s: &Stroke) -> u64 {
    let mut h: u64 = 1469598103934665603;
    let mut mix = |v: u64| h = (h ^ v).wrapping_mul(1099511628211);
    mix(s.points.len() as u64);
    mix(u32::from_le_bytes([s.color[0], s.color[1], s.color[2], s.highlight as u8]) as u64);
    for p in &s.points {
        mix((p.x.round() as i64 as u64) << 32 ^ (p.y.round() as i64 as u64 & 0xffff_ffff));
    }
    h
}

fn ai_list<R>(f: impl FnOnce(&mut Vec<u64>) -> R) -> R {
    let mut g = AI_STROKES.lock().unwrap();
    let v = g.get_or_insert_with(|| {
        if cfg!(test) {
            return Vec::new();
        }
        std::fs::read_to_string(AI_FILE).map(|t| t.lines().filter_map(|l| u64::from_str_radix(l.trim(), 16).ok()).collect()).unwrap_or_default()
    });
    f(v)
}

/// Un tratto messo dall'AI (e non dal pennino)?
pub fn is_ai_stroke(s: &Stroke) -> bool {
    let sig = ai_sig(s);
    ai_list(|v| v.contains(&sig))
}

fn mark_ai(strokes: &[Stroke]) {
    let new: Vec<u64> = strokes.iter().map(ai_sig).collect();
    ai_list(|v| {
        v.extend(&new);
        let n = v.len();
        if n > AI_KEEP {
            v.drain(..n - AI_KEEP);
        }
        if cfg!(test) {
            return;
        }
        let text: String = v.iter().map(|x| format!("{x:x}\n")).collect();
        if let Err(e) = crate::web::write_atomic(std::path::Path::new(AI_FILE), text.as_bytes()) {
            tracing::warn!("{AI_FILE}: {e}");
        }
    });
}

/// Ogni domanda nella chat è un «giro»: una conferma chiesta in un giro vale
/// solo nei giri dopo, cioè dopo la risposta dell'utente.
static TURN: AtomicU64 = AtomicU64::new(0);

/// Una cancellazione grande in attesa della conferma dell'utente.
struct Pending {
    turn: u64,
    sig: u64,
    code: String,
}

static PENDING: Mutex<Option<Pending>> = Mutex::new(None);

/// Comincia un giro nuovo della chat (una domanda dell'utente).
pub fn new_turn() {
    TURN.fetch_add(1, Ordering::SeqCst);
}

/// Il sì dell'utente per una cosa grossa (cancellare quasi tutto il foglio,
/// mettere pagine nel cestino). `sig` dice di che cosa si tratta.
pub enum Gate {
    /// Confermato: si fa.
    Go,
    /// Da chiedere all'utente: il codice per richiamare dopo il suo sì.
    Ask(String),
    /// Il codice è giusto ma è ancora lo stesso giro: l'utente non ha risposto.
    Wait,
}

pub fn gate(sig: u64, given: Option<&str>) -> Gate {
    let turn = TURN.load(Ordering::SeqCst);
    let given = given.map(|c| c.trim().to_uppercase());
    let mut p = PENDING.lock().unwrap();
    match (&given, p.as_ref()) {
        (Some(c), Some(pd)) if *c == pd.code && pd.sig == sig && pd.turn < turn => {
            *p = None;
            Gate::Go
        }
        (Some(c), Some(pd)) if *c == pd.code && pd.turn >= turn => Gate::Wait,
        _ => {
            // la stessa cosa da confermare: lo stesso codice e lo stesso giro
            // (il sì dell'utente nel giro dopo vale ancora)
            let (code, turn) = match p.as_ref() {
                Some(pd) if pd.sig == sig => (pd.code.clone(), pd.turn),
                _ => (confirm_code(sig), turn),
            };
            *p = Some(Pending { turn, sig, code: code.clone() });
            Gate::Ask(code)
        }
    }
}

const MARKS: &str = "evidenzia, sottolinea, barra, cerchia, riquadro, freccia, linea, spunta, croce, testo";

pub fn definitions() -> Vec<Value> {
    vec![
        json!({
            "type": "function",
            "function": {
                "name": "lavagna_leggi",
                "description": "Legge il foglio della lavagna (scritto a mano col pennino, accanto alla chat): righe dall'alto e parole da sinistra, \
                    ognuna con un nome (r1, r2…; r2p3 = terza parola della seconda riga) e il riquadro [x1, y1, x2, y2] in 0–1000; \
                    il disegno a mano (d1), gli evidenziatori (e1…), il grafico. Restituisce anche l'immagine del foglio con i nomi scritti \
                    accanto alle righe e un riquadro per parola. Chiamalo prima di annotare il foglio.",
                "parameters": { "type": "object", "properties": {} }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "lavagna_annota",
                "description": format!("Scrive e disegna sul foglio della lavagna, a mano (tratti come quelli del pennino, annullabili dalla chat). \
                    «segni» è la lista dei segni, fatti tutti insieme. Ogni segno: tipo ({MARKS}); dove: «su» un nome di lavagna_leggi \
                    (\"r2p3\", \"r2\", \"r2p1-p4\" da una parola all'altra, [\"r1p2\", \"r3p1\"] più pezzi, \"tutto\") oppure «riquadro» \
                    [x1, y1, x2, y2] o un punto [x, y] in 0–1000. freccia e linea: «da» e «a» (nomi o punti; con solo «a» o «su» la freccia arriva lì \
                    da in alto a destra). testo: «testo», «dove» rispetto a «su» (destra di serie, sotto, sopra, sinistra) oppure «riquadro» \
                    per scriverci dentro (va a capo), «altezza» delle maiuscole in 0–1000 (di serie come la scrittura del foglio), \
                    «stile» stampatello (di serie) o corsivo. Il testo è scritto a mano come sul quaderno: NON LaTeX né Markdown \
                    (niente $, \\, ^, _, **): scrivi x², x₁, 1/2, √x, ⇒, →, ±, ≤, ≥, ≠, ≈, π, Δ, ✓ (il LaTeX viene comunque convertito). \
                    Righe brevi: per più righe un segno per riga, ognuno sotto il precedente; il risultato dice dove è finito ogni testo \
                    («occupa [x1, y1, x2, y2]»): il successivo va più in basso, senza toccare lo scritto. Facoltativi: «colore» (rosso di serie; l'evidenziatore usa il suo), \
                    «spessore», «doppia» per la sottolineatura."),
                "parameters": {
                    "type": "object",
                    "properties": {
                        "segni": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "tipo": { "type": "string", "enum": ["evidenzia", "sottolinea", "barra", "cerchia", "riquadro", "freccia", "linea", "spunta", "croce", "testo"] },
                                    "su": { "description": "Nome di lavagna_leggi (r2p3, r2, r2p1-p4, d1, tutto) o lista di nomi" },
                                    "riquadro": { "type": "array", "items": { "type": "number" }, "description": "[x1, y1, x2, y2] o [x, y] in 0–1000" },
                                    "da": { "description": "freccia/linea: da dove (nome o [x, y])" },
                                    "a": { "description": "freccia/linea: dove arriva (nome o [x, y])" },
                                    "testo": { "type": "string" },
                                    "dove": { "type": "string", "enum": ["destra", "sotto", "sopra", "sinistra"] },
                                    "altezza": { "type": "number", "description": "Altezza delle maiuscole, 0–1000" },
                                    "stile": { "type": "string", "enum": ["stampatello", "corsivo"] },
                                    "colore": { "type": "string", "description": "rosso, blu, verde, giallo, arancione, viola, rosa, nero, grigio o #rrggbb" },
                                    "spessore": { "type": "number" },
                                    "doppia": { "type": "boolean" }
                                },
                                "required": ["tipo"]
                            }
                        }
                    },
                    "required": ["segni"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "lavagna_modifica",
                "description": "Cambia quello che c'è sul foglio della lavagna, anche la scrittura dell'utente: SOLO se l'utente lo chiede. \
                    «azioni» si fanno tutte insieme (un solo Annulla). Ogni azione: azione (sposta, ridimensiona, ricolora, cancella); \
                    cosa: «su» un nome di lavagna_leggi (\"r2p3\", \"r2\", \"r2p1-p4\", \"r2-r4\", \"d1\", \"e1\", \"grafico\", \"tutto\"; \"ai\" = tutti i segni e i testi messi da te) o una lista di nomi, \
                    oppure «riquadro» [x1, y1, x2, y2] in 0–1000 (i tratti dentro); filtri facoltativi «colore» (solo i tratti di quel colore) e \
                    «solo» (evidenziatore o inchiostro). sposta: «dx», «dy» in 0–1000 oppure «a» [x, y] dove va l'angolo in alto a sinistra. \
                    ridimensiona: «fattore» (2 = il doppio), dall'angolo in alto a sinistra. ricolora: «nuovo_colore». \
                    inchiostro: le evidenziature scelte diventano scrittura con la penna (per chi ha scritto con l'evidenziatore senza volerlo). \
                    Per cancellare buona parte di quello che ha scritto l'utente lo strumento chiede la sua conferma e dà un codice: chiedi all'utente e, \
                    se dice sì, richiama con «conferma» e il codice. I segni e i testi che hai messo tu li cancelli o rifai senza conferma. Dopo una modifica i nomi del foglio cambiano: rileggilo.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "azioni": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "azione": { "type": "string", "enum": ["sposta", "ridimensiona", "ricolora", "cancella", "inchiostro"] },
                                    "su": { "description": "Nome di lavagna_leggi o lista di nomi" },
                                    "riquadro": { "type": "array", "items": { "type": "number" }, "description": "[x1, y1, x2, y2] in 0–1000" },
                                    "colore": { "type": "string", "description": "Solo i tratti di questo colore" },
                                    "solo": { "type": "string", "enum": ["evidenziatore", "inchiostro"] },
                                    "dx": { "type": "number" },
                                    "dy": { "type": "number" },
                                    "a": { "type": "array", "items": { "type": "number" }, "description": "[x, y] in 0–1000" },
                                    "fattore": { "type": "number" },
                                    "nuovo_colore": { "type": "string" }
                                },
                                "required": ["azione"]
                            }
                        },
                        "conferma": { "type": "string", "description": "Il codice dato dallo strumento, dopo il sì dell'utente" }
                    },
                    "required": ["azioni"]
                }
            }
        }),
        json!({
        "type": "function",
        "function": {
            "name": "lavagna_inserisci_grafico",
            "description": "Mette sul foglio della lavagna (la pagina scritta a mano accanto alla chat) un grafico fatto con draw_chart. \
                al_posto_del_disegno=true (default) lo mette al posto del disegno/grafico fatto a mano sul foglio, che si trova da solo \
                (gli assi e le scritte attorno) e si toglie; con `riquadro` [x1, y1, x2, y2] (0–1000 sull'immagine della pagina) si sceglie \
                la zona; se sul foglio non c'è un disegno va sotto lo scritto. L'utente lo vede subito e può annullare dalla chat.",
            "parameters": {
                "type": "object",
                "properties": {
                    "grafico": { "type": "string", "description": "L'id del grafico (da draw_chart)" },
                    "al_posto_del_disegno": { "type": "boolean", "description": "Sostituisce il disegno a mano (default true)" },
                    "riquadro": { "type": "array", "items": { "type": "number" }, "description": "Zona [x1, y1, x2, y2], 0–1000 sull'immagine della pagina (facoltativa)" }
                },
                "required": ["grafico"]
            }
        }
    }),
    ]
}

pub fn system_note() -> String {
    "\n\nAccanto alla chat c'è il foglio della lavagna (scritto a mano col pennino) e lo puoi leggere e annotare: \
     lavagna_leggi per vedere righe e parole coi loro nomi, poi lavagna_annota per evidenziare, sottolineare, cerchiare, \
     barrare, mettere frecce, spunte o croci e scrivere a mano (correzioni, risultati, note accanto alle righe). \
     Per un grafico, anche al posto di un disegno fatto a mano: prima draw_chart, poi lavagna_inserisci_grafico. \
     Se l'utente chiede di spostare, ingrandire, ricolorare o cancellare qualcosa sul foglio (anche quello che ha scritto lui, \
     o i segni che hai messo tu): lavagna_modifica. Di tua iniziativa non cancelli e non sposti niente: aggiungi soltanto. \
     Se l'utente ti chiede di correggere o rifare i segni che hai messo tu (testo storto, sovrapposto, scritto male), fallo subito: \
     cancellali con lavagna_modifica («su»: \"ai\", non serve la sua conferma) e riscrivili con lavagna_annota, più corti e uno sotto l'altro. \
     Non dire che non puoi: gli strumenti ci sono. L'utente vede subito il risultato e lo può annullare dalla chat."
        .into()
}

/// Riquadro (frazioni 0..1 della pagina: x0, y0, x1, y1) di un tratto.
fn bbox(pts: &[(f32, f32)]) -> Option<[f32; 4]> {
    let mut b = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
    for &(x, y) in pts {
        b = [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)];
    }
    (b[0] <= b[2]).then_some(b)
}

fn union(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])]
}

/// La zona del disegno a mano: i tratti lunghi (assi, linee) che si toccano,
/// il gruppo più grande, più le scritte corte attorno (etichette, tacche).
/// Ritorna la zona e gli indici dei tratti che ne fanno parte.
pub fn find_drawing(boxes: &[Option<[f32; 4]>]) -> Option<([f32; 4], Vec<usize>)> {
    const LONG: f32 = 0.12;
    let long: Vec<usize> = (0..boxes.len()).filter(|&i| boxes[i].is_some_and(|b| b[2] - b[0] > LONG || b[3] - b[1] > LONG)).collect();
    if long.is_empty() {
        return None;
    }
    // gruppi di tratti lunghi che si toccano (con un po' di margine)
    let near = |a: [f32; 4], b: [f32; 4]| a[0] - 0.03 <= b[2] && b[0] - 0.03 <= a[2] && a[1] - 0.03 <= b[3] && b[1] - 0.03 <= a[3];
    let mut groups: Vec<([f32; 4], Vec<usize>)> = Vec::new();
    for &i in &long {
        let b = boxes[i].unwrap();
        let hits: Vec<usize> = (0..groups.len()).filter(|&g| near(groups[g].0, b)).collect();
        let mut g = (b, vec![i]);
        for &h in hits.iter().rev() {
            let (gb, gi) = groups.remove(h);
            g.0 = union(g.0, gb);
            g.1.extend(gi);
        }
        groups.push(g);
    }
    // un disegno da sostituire ha gli assi: almeno una riga lunga orizzontale
    // e una verticale (così parole in corsivo o sottolineature non si toccano)
    let axes = |members: &[usize]| {
        let horiz = members.iter().any(|&i| boxes[i].is_some_and(|b| b[2] - b[0] > LONG && (b[3] - b[1]) < (b[2] - b[0]) * 0.25));
        let vert = members.iter().any(|&i| boxes[i].is_some_and(|b| b[3] - b[1] > LONG && (b[2] - b[0]) < (b[3] - b[1]) * 0.25));
        horiz && vert
    };
    let (mut zone, mut members) = groups.into_iter().filter(|g| axes(&g.1)).max_by(|a, b| {
        let area = |r: [f32; 4]| (r[2] - r[0]) * (r[3] - r[1]);
        area(a.0).total_cmp(&area(b.0))
    })?;
    // le scritte corte dentro o appena fuori (etichette degli assi, dei
    // punti): a parole intere (tratti vicini fra loro), e solo le parole che
    // stanno tutte nella zona; una parola che esce («Giusto?» accanto al
    // disegno) resta com'è, tutta
    let pad = 0.06;
    let outer = [zone[0] - pad, zone[1] - pad, zone[2] + pad, zone[3] + pad];
    let short: Vec<usize> =
        (0..boxes.len()).filter(|&i| !members.contains(&i) && boxes[i].is_some_and(|b| b[2] - b[0] <= LONG && b[3] - b[1] <= LONG)).collect();
    let close = |a: [f32; 4], b: [f32; 4]| a[0] - 0.012 <= b[2] && b[0] - 0.012 <= a[2] && a[1] - 0.008 <= b[3] && b[1] - 0.008 <= a[3];
    let mut words: Vec<([f32; 4], Vec<usize>)> = Vec::new();
    for &i in &short {
        let b = boxes[i].unwrap();
        let hits: Vec<usize> = (0..words.len()).filter(|&w| close(words[w].0, b)).collect();
        let mut w = (b, vec![i]);
        for &h in hits.iter().rev() {
            let (wb, wi) = words.remove(h);
            w.0 = union(w.0, wb);
            w.1.extend(wi);
        }
        words.push(w);
    }
    for (wb, wi) in words {
        if wb[0] >= outer[0] && wb[2] <= outer[2] && wb[1] >= outer[1] && wb[3] <= outer[3] {
            members.extend(wi);
            zone = union(zone, wb);
        }
    }
    Some((zone, members))
}

/// Il grafico (proporzioni `aspect` = larghezza/altezza in unità della
/// pagina) dentro la zona, centrato: u, v, larghezza, altezza.
fn fit(zone: [f32; 4], aspect: f32) -> [f32; 4] {
    let (zw, zh) = (zone[2] - zone[0], zone[3] - zone[1]);
    let (w, h) = if zw / zh > aspect { (zh * aspect, zh) } else { (zw, zw / aspect) };
    [zone[0] + (zw - w) / 2.0, zone[1] + (zh - h) / 2.0, w, h]
}

/// Esegue lo strumento della lavagna; None se `name` non è suo.
pub fn execute(name: &str, args: &Value) -> Option<crate::docagent::Outcome> {
    use crate::docagent::Outcome;
    let fail = |e: anyhow::Error| Outcome { text: format!("ERRORE: {e:#}"), short: format!("errore: {e:#}"), ..Default::default() };
    Some(match name {
        "lavagna_inserisci_grafico" => match insert(args) {
            Ok((text, n)) => Outcome { text, short: "grafico sul foglio".into(), edit: Some(n), ..Default::default() },
            Err(e) => fail(e),
        },
        "lavagna_leggi" => match read_board() {
            Ok((text, images)) => Outcome { text, short: "foglio letto".into(), images, ..Default::default() },
            Err(e) => fail(e),
        },
        "lavagna_annota" => match annotate(args) {
            Ok((text, n)) => Outcome { text, short: "segni sul foglio".into(), edit: Some(n), ..Default::default() },
            Err(e) => fail(e),
        },
        "lavagna_modifica" => match modify(args) {
            Ok((text, Some(n))) => Outcome { text, short: "foglio modificato".into(), edit: Some(n), ..Default::default() },
            Ok((text, None)) => Outcome { text, short: "conferma chiesta".into(), ..Default::default() },
            Err(e) => fail(e),
        },
        _ => return None,
    })
}

/// Il foglio letto, in parole e in immagine (coi nomi).
fn read_board() -> Result<(String, Vec<Vec<u8>>)> {
    let hub = HUB.get().context("lavagna non disponibile")?;
    let (reading, page) = {
        let s = hub.lock();
        if s.bg.as_ref().is_some_and(|b| !b.is_chart()) {
            bail!("al centro c'è la pagina di un documento, non la lavagna");
        }
        (crate::boardread::read(&s.drawing.strokes, &s.geom(), s.bg.as_ref()), s.page())
    };
    let text = crate::boardread::describe(&reading);
    let images = if reading.is_empty() { Vec::new() } else { crate::boardread::labelled_png(&page, &reading).ok().into_iter().collect() };
    *LAST.lock().unwrap() = Some(reading);
    Ok((text, images))
}

/// La lettura da usare per i nomi: l'ultima vista dal modello, se il foglio da
/// allora ha solo tratti in più; altrimenti una nuova.
fn reading_for(s: &crate::scene::Scene) -> crate::boardread::Reading {
    let strokes = &s.drawing.strokes;
    if let Some(r) = LAST.lock().unwrap().take() {
        if strokes.len() >= r.n && strokes_sig(&strokes[..r.n]) == r.sig {
            return r;
        }
    }
    crate::boardread::read(strokes, &s.geom(), s.bg.as_ref())
}

/// Aggiunge al foglio i segni chiesti, tutti insieme (un solo Annulla).
fn annotate(args: &Value) -> Result<(String, u64)> {
    use crate::stroke::{PenEvent, PenPhase};
    let hub = HUB.get().context("lavagna non disponibile")?;
    let list: Vec<Value> = match &args["segni"] {
        Value::Array(a) => a.clone(),
        Value::Object(_) => vec![args["segni"].clone()],
        _ if args.get("tipo").is_some() => vec![args.clone()],
        _ => bail!("manca «segni» (la lista dei segni)"),
    };
    if list.is_empty() {
        bail!("«segni» è vuota");
    }
    if list.len() > 60 {
        bail!("troppi segni insieme (al massimo 60)");
    }
    let mut done: Vec<String> = Vec::new();
    let res = hub.edit(|s| -> Result<Edit> {
        if s.bg.as_ref().is_some_and(|b| !b.is_chart()) {
            bail!("al centro c'è la pagina di un documento, non la lavagna");
        }
        let g = s.geom();
        let reading = reading_for(s);
        let ctx = crate::boardmarks::Ctx::new(&reading, g.w / g.h, s.pen.width, s.highlighter.color);
        let mut add: Vec<Stroke> = Vec::new();
        for (i, m) in list.iter().enumerate() {
            let (marks, what) = crate::boardmarks::build(&ctx, m, i).with_context(|| format!("segno {}", i + 1))?;
            done.push(what);
            for mk in marks {
                let mut st = if mk.highlight { Stroke::highlighter(mk.color, mk.width) } else { Stroke::new(mk.color, mk.width) };
                let last = mk.pts.len().saturating_sub(1);
                for (k, (u, v)) in mk.pts.into_iter().enumerate() {
                    let (x, y) = g.pad(u, v);
                    let phase = if k == 0 { PenPhase::Down } else if k == last { PenPhase::Up } else { PenPhase::Move };
                    st.points.push(PenEvent { x, y, pressure: 0.6, phase });
                }
                add.push(st);
            }
        }
        // la lettura vale ancora: i tratti vecchi non cambiano
        *LAST.lock().unwrap() = Some(reading);
        let before = s.drawing.strokes.clone();
        let before_bg = s.bg.clone();
        mark_ai(&add);
        s.add_strokes(add);
        let now = &s.drawing.strokes;
        Ok(Edit { n: 0, before, before_bg, kept: now.len(), kept_sig: strokes_sig(now), bg_key: s.bg.as_ref().map(|b| b.key.clone()) })
    });
    let edit = res?;
    let n = remember(edit);
    crate::docagent::board_changed();
    Ok((format!("Fatto: sul foglio ci sono {} (l'utente li vede e li può annullare dalla chat).", done.join(", ")), n))
}

/// I tratti scelti da un'azione di `lavagna_modifica`, e se c'è il grafico.
fn select(r: &crate::boardread::Reading, strokes: &[Stroke], g: &crate::scene::PageGeom, a: &Value) -> Result<(Vec<usize>, bool, String)> {
    let mut idx: Vec<usize> = Vec::new();
    let mut chart = false;
    let mut what = Vec::new();
    let names: Vec<String> = match &a["su"] {
        Value::String(x) => vec![x.clone()],
        Value::Array(v) => v.iter().filter_map(|x| x.as_str().map(String::from)).collect(),
        _ => Vec::new(),
    };
    for n in &names {
        let key = n.trim().to_lowercase();
        if matches!(key.as_str(), "grafico" | "chart") {
            chart = r.chart.is_some();
            if !chart {
                bail!("sul foglio non c'è un grafico");
            }
        } else if matches!(key.as_str(), "tutto" | "all") {
            // tutto quello che c'è adesso (anche i segni aggiunti dopo l'ultima lettura)
            idx.extend((0..strokes.len()).filter(|&i| !strokes[i].points.is_empty()));
            chart = r.chart.is_some();
        } else if matches!(key.as_str(), "ai" | "segni_ai" | "miei") {
            // i segni e i testi messi dall'AI
            idx.extend((0..strokes.len()).filter(|&i| !strokes[i].points.is_empty() && is_ai_stroke(&strokes[i])));
            if idx.is_empty() {
                bail!("sul foglio non ci sono segni messi dall'AI");
            }
        } else {
            idx.extend(r.strokes_of(n).with_context(|| format!("non trovo «{n}» sul foglio: rileggilo con lavagna_leggi"))?);
        }
        what.push(n.clone());
    }
    if names.is_empty() {
        let b = crate::boardmarks::target(r, &a["riquadro"])?.context("manca cosa: «su» con un nome del foglio, o «riquadro»")?;
        if b[2] - b[0] < 1e-4 {
            bail!("«riquadro» è [x1, y1, x2, y2]");
        }
        // i tratti che stanno dentro (per più di metà)
        for (i, st) in strokes.iter().enumerate() {
            let pts: Vec<(f32, f32)> = st.points.iter().map(|p| g.unit(p.x, p.y)).collect();
            if pts.is_empty() {
                continue;
            }
            let inside = pts.iter().filter(|p| p.0 >= b[0] && p.0 <= b[2] && p.1 >= b[1] && p.1 <= b[3]).count();
            if inside * 2 > pts.len() {
                idx.push(i);
            }
        }
        if let Some(c) = r.chart {
            // il grafico, se sta tutto nel riquadro
            chart = c[0] >= b[0] && c[2] <= b[2] && c[1] >= b[1] && c[3] <= b[3];
        }
        what.push("il riquadro".to_string());
    }
    if let Some(c) = a["colore"].as_str() {
        let want = crate::boardmarks::parse_color(c, false).map(crate::boardread::color_name).with_context(|| format!("colore «{c}» sconosciuto"))?;
        idx.retain(|&i| crate::boardread::color_name(strokes[i].color) == want || (strokes[i].highlight && crate::boardmarks::parse_color(c, true) == Some(strokes[i].color)));
        chart = false;
        what.push(format!("in {c}"));
    }
    match a["solo"].as_str().map(|x| x.trim().to_lowercase()) {
        Some(x) if x.starts_with("evidenz") || x == "highlighter" => {
            idx.retain(|&i| strokes[i].highlight);
            chart = false;
        }
        Some(x) if x.starts_with("inchiostro") || x == "ink" || x == "penna" => {
            idx.retain(|&i| !strokes[i].highlight);
            chart = false;
        }
        _ => {}
    }
    idx.sort_unstable();
    idx.dedup();
    idx.retain(|&i| i < strokes.len());
    Ok((idx, chart, what.join(", ")))
}

/// Un codice corto per la conferma (lettere che non si confondono).
fn confirm_code(seed: u64) -> String {
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0);
    let mut x = t ^ seed;
    (0..4)
        .map(|_| {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            b"ACDEFHJKMNPRTWXY"[(x >> 60) as usize] as char
        })
        .collect()
}

/// Sposta, ridimensiona, ricolora, cancella. Ritorna il testo e il numero per
/// «Annulla» (None: serve prima la conferma dell'utente).
fn modify(args: &Value) -> Result<(String, Option<u64>)> {
    let hub = HUB.get().context("lavagna non disponibile")?;
    let list: Vec<Value> = match &args["azioni"] {
        Value::Array(a) => a.clone(),
        Value::Object(_) => vec![args["azioni"].clone()],
        _ if args.get("azione").is_some() => vec![args.clone()],
        _ => bail!("manca «azioni» (la lista delle azioni)"),
    };
    if list.is_empty() || list.len() > 40 {
        bail!("da 1 a 40 azioni");
    }
    let code_given = args["conferma"].as_str().map(String::from);
    let res = hub.edit(|s| -> Result<std::result::Result<(Edit, String), String>> {
        if s.bg.as_ref().is_some_and(|b| !b.is_chart()) {
            bail!("al centro c'è la pagina di un documento, non la lavagna");
        }
        let g = s.geom();
        let reading = reading_for(s);
        let mut strokes = s.drawing.strokes.clone();
        let mut bg = s.bg.clone();
        let mut deleted: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
        let mut report: Vec<String> = Vec::new();
        let mut moved: Vec<(Vec<usize>, [f32; 4])> = Vec::new();
        let unit_box = |sts: &[Stroke], idx: &[usize]| -> Option<[f32; 4]> {
            let mut b = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
            for &i in idx {
                for p in &sts[i].points {
                    let (u, v) = g.unit(p.x, p.y);
                    b = [b[0].min(u), b[1].min(v), b[2].max(u), b[3].max(v)];
                }
            }
            (b[0] <= b[2]).then_some(b)
        };
        let num = |v: &Value| v.as_f64().map(|x| x as f32 / 1000.0);
        for (k, a) in list.iter().enumerate() {
            let act = a["azione"].as_str().or(a["action"].as_str()).with_context(|| format!("azione {}: manca «azione»", k + 1))?.trim().to_lowercase();
            let (mut idx, chart, what) = select(&reading, &strokes, &g, a).with_context(|| format!("azione {}", k + 1))?;
            idx.retain(|i| !deleted.contains(i));
            if idx.is_empty() && !chart {
                bail!("azione {} ({what}): non c'è nessun tratto lì", k + 1);
            }
            let chart_box = bg.as_ref().filter(|_| chart).map(|b| [b.rect[0], b.rect[1], b.rect[0] + b.rect[2], b.rect[1] + b.rect[3]]);
            let sel_box = match (unit_box(&strokes, &idx), chart_box) {
                (Some(a), Some(b)) => Some(crate::boardread::union(a, b)),
                (a, b) => a.or(b),
            };
            match act.as_str() {
                "sposta" | "move" => {
                    let Some(b) = sel_box else { bail!("azione {}: niente da spostare", k + 1) };
                    let (mut du, mut dv) = match a["a"].as_array() {
                        Some(p) if p.len() == 2 => (num(&p[0]).unwrap_or(b[0]) - b[0], num(&p[1]).unwrap_or(b[1]) - b[1]),
                        _ => (num(&a["dx"]).unwrap_or(0.0), num(&a["dy"]).unwrap_or(0.0)),
                    };
                    if du == 0.0 && dv == 0.0 {
                        bail!("azione {} (sposta): servono «dx»/«dy» o «a»", k + 1);
                    }
                    // resta tutto sul foglio
                    du = du.clamp(0.003 - b[0], (0.997 - b[2]).max(0.003 - b[0]));
                    dv = dv.clamp(0.003 - b[1], (0.997 - b[3]).max(0.003 - b[1]));
                    for &i in &idx {
                        for p in &mut strokes[i].points {
                            let (u, v) = g.unit(p.x, p.y);
                            (p.x, p.y) = g.pad(u + du, v + dv);
                        }
                    }
                    if chart {
                        if let Some(c) = bg.as_mut() {
                            c.rect[0] = (c.rect[0] + du).clamp(0.0, (1.0 - c.rect[2]).max(0.0));
                            c.rect[1] = (c.rect[1] + dv).clamp(0.0, (1.0 - c.rect[3]).max(0.0));
                        }
                    }
                    moved.push((idx.clone(), [b[0] + du, b[1] + dv, b[2] + du, b[3] + dv]));
                    report.push(format!("spostato {what}"));
                }
                "ridimensiona" | "scala" | "ingrandisci" | "rimpicciolisci" | "resize" | "scale" => {
                    let f = a["fattore"].as_f64().or(a["factor"].as_f64()).map(|f| f as f32).with_context(|| format!("azione {} (ridimensiona): manca «fattore»", k + 1))?;
                    if !(0.1..=8.0).contains(&f) {
                        bail!("azione {}: «fattore» fra 0.1 e 8", k + 1);
                    }
                    let Some(b) = sel_box else { bail!("azione {}: niente da ridimensionare", k + 1) };
                    // non oltre il bordo del foglio
                    let fit = ((0.997 - b[0]) / (b[2] - b[0]).max(1e-4)).min((0.997 - b[1]) / (b[3] - b[1]).max(1e-4));
                    let f = f.min(fit).max(0.1);
                    for &i in &idx {
                        for p in &mut strokes[i].points {
                            let (u, v) = g.unit(p.x, p.y);
                            (p.x, p.y) = g.pad(b[0] + (u - b[0]) * f, b[1] + (v - b[1]) * f);
                        }
                        let w = strokes[i].width;
                        strokes[i].width = if strokes[i].highlight { w * f } else { (w * f.sqrt()).clamp(0.5, 12.0) };
                    }
                    if chart {
                        if let Some(c) = bg.as_mut() {
                            c.rect[0] = b[0] + (c.rect[0] - b[0]) * f;
                            c.rect[1] = b[1] + (c.rect[1] - b[1]) * f;
                            c.rect[2] *= f;
                            c.rect[3] *= f;
                        }
                    }
                    moved.push((idx.clone(), [b[0], b[1], b[0] + (b[2] - b[0]) * f, b[1] + (b[3] - b[1]) * f]));
                    report.push(format!("ridimensionato {what} (×{f:.2})"));
                }
                "ricolora" | "colora" | "recolor" | "color" => {
                    let c = a["nuovo_colore"].as_str().or(a["new_color"].as_str()).with_context(|| format!("azione {} (ricolora): manca «nuovo_colore»", k + 1))?;
                    for &i in &idx {
                        strokes[i].color = crate::boardmarks::parse_color(c, strokes[i].highlight).with_context(|| format!("colore «{c}» sconosciuto"))?;
                    }
                    report.push(format!("ricolorato {what} in {c}"));
                }
                "inchiostro" | "scrittura" | "ink" => {
                    let mut n = 0;
                    for &i in &idx {
                        if strokes[i].highlight {
                            strokes[i].highlight = false;
                            strokes[i].color = s.pen.color;
                            strokes[i].width = s.pen.width;
                            n += 1;
                        }
                    }
                    report.push(format!("{n} evidenziature di {what} diventate scrittura"));
                }
                "cancella" | "togli" | "elimina" | "delete" | "erase" => {
                    deleted.extend(idx.iter().copied());
                    if chart {
                        bg = None;
                    }
                    report.push(format!("cancellato {what}"));
                }
                other => bail!("azione «{other}» sconosciuta: sposta, ridimensiona, ricolora, cancella, inchiostro"),
            }
        }
        // cancellare buona parte del foglio: prima il sì dell'utente (in un
        // giro dopo: la sua risposta)
        // (i segni messi dall'AI stessa non contano: rifarli o toglierli è
        // correggere il proprio lavoro, non cancellare quello dell'utente)
        let n_ink = s.drawing.strokes.iter().filter(|x| !x.points.is_empty() && !is_ai_stroke(x)).count();
        let user_deleted = deleted.iter().filter(|&&i| s.drawing.strokes.get(i).is_some_and(|x| !is_ai_stroke(x))).count();
        // la conferma vale per quei tratti: se intanto si scrive altrove resta buona
        let sig = {
            let del: Vec<Stroke> = deleted.iter().filter_map(|&i| s.drawing.strokes.get(i).cloned()).collect();
            strokes_sig(&del)
        };
        if user_deleted >= 20 && user_deleted * 3 >= n_ink {
            match gate(sig, code_given.as_deref()) {
                Gate::Go => {}
                Gate::Wait => {
                    *LAST.lock().unwrap() = Some(reading);
                    return Ok(Err("NON FATTO: la conferma la dà l'utente. Chiedigli se va bene e aspetta la sua risposta prima di richiamare.".into()));
                }
                Gate::Ask(code) => {
                    *LAST.lock().unwrap() = Some(reading);
                    return Ok(Err(format!(
                        "NON FATTO: cancellerebbe {} tratti su {} scritti dall'utente ({}). Chiedi all'utente se va bene (potrà comunque annullare); se risponde di sì, \
                         richiama lavagna_modifica con le stesse azioni e \"conferma\": \"{code}\".",
                        user_deleted,
                        n_ink,
                        report.join("; ")
                    )));
                }
            }
        }
        // dopo uno spostamento: tocca altro scritto?
        let m = 0.1 * reading.char_h;
        let mut warn = String::new();
        'w: for (idx, mb) in &moved {
            for w in reading.lines.iter().flat_map(|l| l.words.iter()) {
                if w.strokes.iter().any(|i| idx.contains(i)) {
                    continue;
                }
                let o = w.bbox;
                if mb[0] < o[2] - m && o[0] + m < mb[2] && mb[1] < o[3] - m && o[1] + m < mb[3] {
                    warn = format!(" Attenzione: ora tocca altro scritto (vicino a {}).", w.id);
                    break 'w;
                }
            }
        }
        let kept: Vec<Stroke> = strokes.into_iter().enumerate().filter(|(i, _)| !deleted.contains(i)).map(|(_, st)| st).collect();
        let before = s.drawing.strokes.clone();
        let before_bg = s.bg.clone();
        s.replace_page(kept, bg);
        let now = &s.drawing.strokes;
        let edit = Edit { n: 0, before, before_bg, kept: now.len(), kept_sig: strokes_sig(now), bg_key: s.bg.as_ref().map(|b| b.key.clone()) };
        Ok(Ok((edit, format!("{}.{warn}", report.join("; ")))))
    })?;
    match res {
        Err(text) => Ok((text, None)),
        Ok((edit, done)) => {
            *LAST.lock().unwrap() = None;
            let n = remember(edit);
            crate::docagent::board_changed();
            Ok((
                format!(
                    "Fatto: {done} L'utente lo vede e lo può annullare dalla chat. I nomi del foglio sono cambiati: rileggilo con lavagna_leggi \
                     prima di altri segni o modifiche."
                ),
                Some(n),
            ))
        }
    }
}

/// Tiene la modifica per «Annulla»; ritorna il suo numero.
fn remember(edit: Edit) -> u64 {
    let mut u = UNDO.lock().unwrap();
    let n = u.last().map(|x| x.n + 1).unwrap_or(1);
    u.push(Edit { n, ..edit });
    // non serve tenerne tante
    if u.len() > 20 {
        u.remove(0);
    }
    n
}

fn insert(args: &Value) -> Result<(String, u64)> {
    let hub = HUB.get().context("lavagna non disponibile")?;
    let id = args["grafico"].as_str().or(args["image"].as_str()).context("manca «grafico» (l'id di draw_chart)")?;
    let chart = crate::charts::find(id).with_context(|| format!("non trovo il grafico «{id}»: fallo prima con draw_chart"))?;
    // il bianco del grafico diventa trasparente: sotto si vedono righe e quadretti
    let png = Arc::new(white_to_alpha(&std::fs::read(&chart.png)?)?);
    let replace = args["al_posto_del_disegno"].as_bool().unwrap_or(true);
    let given: Option<[f32; 4]> = args["riquadro"].as_array().and_then(|a| {
        let n: Vec<f32> = a.iter().filter_map(|v| v.as_f64().map(|x| x as f32)).collect();
        (n.len() == 4).then(|| {
            let s = if n.iter().all(|&v| v <= 1.0) { 1.0 } else { 1000.0 };
            [n[0] / s, n[1] / s, n[2] / s, n[3] / s].map(|v| v.clamp(0.0, 1.0))
        })
    });
    let s = hub.lock();
    if s.bg.as_ref().is_some_and(|b| !b.is_chart()) {
        bail!("sul foglio c'è la pagina di un documento: usa document_insert_image");
    }
    let g = s.geom();
    let boxes: Vec<Option<[f32; 4]>> =
        s.drawing.strokes.iter().map(|st| bbox(&st.points.iter().map(|p| g.unit(p.x, p.y)).collect::<Vec<_>>())).collect();
    let aspect = (chart.width / chart.height) * g.h / g.w;
    let (zone, remove): ([f32; 4], Vec<usize>) = match (given, replace) {
        (Some(r), true) => {
            let r = [r[0].min(r[2]), r[1].min(r[3]), r[0].max(r[2]), r[1].max(r[3])];
            let inside: Vec<usize> = boxes
                .iter()
                .enumerate()
                .filter_map(|(i, b)| {
                    let b = (*b)?;
                    let (cx, cy) = ((b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0);
                    (cx >= r[0] && cx <= r[2] && cy >= r[1] && cy <= r[3]).then_some(i)
                })
                .collect();
            (r, inside)
        }
        (Some(r), false) => ([r[0].min(r[2]), r[1].min(r[3]), r[0].max(r[2]), r[1].max(r[3])], Vec::new()),
        (None, true) if find_drawing(&boxes).is_some() => {
            let (z, m) = find_drawing(&boxes).unwrap();
            // un po' più grande del disegno a mano: il grafico ha numeri e legenda
            let pad = 0.03;
            ([(z[0] - pad).max(0.02), (z[1] - pad).max(0.02), (z[2] + pad).min(0.98), (z[3] + pad).min(0.98)], m)
        }
        _ => {
            // sotto lo scritto, largo metà foglio
            let bottom = boxes.iter().flatten().map(|b| b[3]).fold(0.05f32, f32::max);
            let h = (0.5 / aspect).min(0.95 - bottom - 0.02).max(0.15);
            ([0.25, (bottom + 0.02).min(0.8), 0.75, (bottom + 0.02 + h).min(0.98)], Vec::new())
        }
    };
    if zone[2] - zone[0] < 0.05 || zone[3] - zone[1] < 0.05 {
        bail!("zona troppo piccola per il grafico");
    }
    let rect = fit(zone, aspect);
    // il lavoro pesante (fondere le immagini) senza tenere ferma la lavagna
    let before = s.drawing.strokes.clone();
    let before_bg = s.bg.clone();
    let ratio = g.w / g.h;
    drop(s);
    let before_sig = strokes_sig(&before);
    let kept: Vec<Stroke> = before.iter().enumerate().filter(|(i, _)| !remove.contains(i)).map(|(_, st)| st.clone()).collect();
    // un grafico già messo prima resta: i due si fondono in un'immagine sola
    let bg = match before_bg.clone() {
        Some(old) => merge(&old, &png, rect, ratio)?,
        None => PageBg { png, rect, key: format!("chart:{}", chart.id) },
    };
    let edit = Edit { n: 0, before, before_bg, kept: kept.len(), kept_sig: strokes_sig(&kept), bg_key: Some(bg.key.clone()) };
    let applied = hub.edit(|s| {
        // nel frattempo il foglio non è cambiato (un tratto, un'altra pagina)?
        if strokes_sig(&s.drawing.strokes) != before_sig || s.bg.as_ref().map(|b| &b.key) != edit.before_bg.as_ref().map(|b| &b.key) {
            return false;
        }
        s.replace_page(kept, Some(bg));
        true
    });
    if !applied {
        bail!("il foglio è cambiato mentre preparavo il grafico: riprova");
    }
    let n = remember(edit);
    *LAST.lock().unwrap() = None;
    crate::docagent::board_changed();
    Ok((
        format!(
            "Fatto: il grafico {} è sul foglio della lavagna{}.",
            chart.id,
            if remove.is_empty() { String::new() } else { format!(", al posto del disegno a mano ({} tratti tolti)", remove.len()) }
        ),
        n,
    ))
}

/// Il bianco diventa trasparente («colore → alfa»): sul foglio bianco
/// l'immagine resta identica, sulla carta a righe o a quadretti le righe si
/// vedono sotto, come se il grafico fosse stampato sul foglio.
pub fn white_to_alpha(png: &[u8]) -> Result<Vec<u8>> {
    let mut img = tiny_skia::Pixmap::decode_png(png).map_err(|e| anyhow::anyhow!("PNG: {e}"))?;
    for px in img.pixels_mut() {
        let c = px.demultiply();
        let a0 = c.alpha() as f32 / 255.0;
        let [r, g, b] = [c.red(), c.green(), c.blue()].map(|v| v as f32 / 255.0);
        // quanto il colore si allontana dal bianco
        let a = (1.0 - r).max(1.0 - g).max(1.0 - b);
        let (nr, ng, nb, na) = if a <= 1e-4 {
            (0.0, 0.0, 0.0, 0.0)
        } else {
            let f = |v: f32| (1.0 - (1.0 - v) / a).clamp(0.0, 1.0);
            (f(r), f(g), f(b), a * a0)
        };
        let q = |v: f32| (v * 255.0).round() as u8;
        *px = tiny_skia::ColorU8::from_rgba(q(nr), q(ng), q(nb), q(na)).premultiply();
    }
    img.encode_png().map_err(|e| anyhow::anyhow!("PNG: {e}"))
}

/// Due immagini sul foglio in una sola, grande quanto la pagina.
fn merge(old: &PageBg, new_png: &[u8], rect: [f32; 4], aspect: f32) -> Result<PageBg> {
    use tiny_skia::{Pixmap, PixmapPaint, Transform};
    let w = 2400u32;
    let h = ((w as f32) / aspect).round().max(1.0) as u32;
    let mut canvas = Pixmap::new(w, h).context("immagine vuota")?;
    for (png, r) in [(old.png.as_slice(), old.rect), (new_png, rect)] {
        let img = Pixmap::decode_png(png).map_err(|e| anyhow::anyhow!("PNG: {e}"))?;
        let t = Transform::from_row(
            r[2] * w as f32 / img.width() as f32,
            0.0,
            0.0,
            r[3] * h as f32 / img.height() as f32,
            r[0] * w as f32,
            r[1] * h as f32,
        );
        canvas.draw_pixmap(0, 0, img.as_ref(), &PixmapPaint { quality: tiny_skia::FilterQuality::Bicubic, ..Default::default() }, t, None);
    }
    let png = canvas.encode_png().map_err(|e| anyhow::anyhow!("PNG: {e}"))?;
    Ok(PageBg { png: Arc::new(png), rect: [0.0, 0.0, 1.0, 1.0], key: format!("chart:{}", chrono::Local::now().timestamp_millis()) })
}

/// «Annulla» della chat sul foglio: com'era prima della modifica `n`.
/// Toglie i segni o il grafico e rimette i tratti che il grafico aveva
/// sostituito; quello che hai scritto dopo resta. Non tocca un documento aperto, né un foglio cambiato
/// in altro modo (cancellature, un'altra pagina): lì serve Cmd+Z.
pub fn undo(n: u64) -> Result<()> {
    let hub = HUB.get().context("lavagna non disponibile")?;
    let mut u = UNDO.lock().unwrap();
    let i = u.iter().position(|x| x.n == n).context("questa modifica non si può più annullare")?;
    if i + 1 != u.len() {
        bail!("prima annulla la modifica al foglio fatta dopo questa");
    }
    let e = &u[i];
    let res = hub.edit(|s| -> Result<()> {
        match &s.bg {
            Some(b) if !b.is_chart() => bail!("al centro c'è un documento: torna alla lavagna e riprova"),
            b if b.as_ref().map(|b| &b.key) == e.bg_key.as_ref() => {}
            _ => bail!("il foglio non è più quello di questa modifica (pagina cambiata o pulita)"),
        }
        let now = &s.drawing.strokes;
        if now.len() < e.kept || strokes_sig(&now[..e.kept]) != e.kept_sig {
            bail!("dopo questa modifica il foglio è cambiato (cancellature): usa Cmd+Z per tornare indietro");
        }
        // i tratti di prima, più quelli scritti dopo il grafico
        let mut strokes = e.before.clone();
        strokes.extend(now[e.kept..].iter().cloned());
        s.replace_page(strokes, e.before_bg.clone());
        Ok(())
    });
    res?;
    u.truncate(i);
    drop(u);
    *LAST.lock().unwrap() = None;
    crate::docagent::board_changed();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trova_il_disegno_a_mano() {
        // testo in alto (tratti corti), assi lunghi al centro, etichette vicine, testo in basso
        let b = |x0: f32, y0: f32, x1: f32, y1: f32| Some([x0, y0, x1, y1]);
        let boxes = vec![
            b(0.10, 0.10, 0.14, 0.13), // «R» del testo
            b(0.15, 0.10, 0.19, 0.13),
            b(0.30, 0.40, 0.70, 0.43), // asse x
            b(0.50, 0.25, 0.52, 0.60), // asse y
            b(0.34, 0.28, 0.40, 0.31), // «P(-1,2)»
            b(0.42, 0.45, 0.45, 0.47), // «-1»
            b(0.10, 0.80, 0.16, 0.84), // «Giusto?»
            // «Giu» fuori e «sto?» appena sotto l'asse: la parola resta intera
            b(0.20, 0.63, 0.235, 0.66),
            b(0.24, 0.63, 0.27, 0.66),
            b(0.275, 0.63, 0.31, 0.66),
        ];
        let (zone, m) = find_drawing(&boxes).unwrap();
        let mut m = m;
        m.sort();
        assert_eq!(m, vec![2, 3, 4, 5]);
        assert!(zone[0] <= 0.30 && zone[2] >= 0.70 && zone[1] <= 0.25 && zone[3] >= 0.60);
        assert!(find_drawing(&[b(0.1, 0.1, 0.12, 0.12)]).is_none());
        // solo righe di testo in corsivo (lunghe ma basse) e una sottolineatura: niente da sostituire
        assert!(find_drawing(&[b(0.10, 0.20, 0.80, 0.24), b(0.10, 0.30, 0.75, 0.34), b(0.10, 0.36, 0.60, 0.365)]).is_none());
        // bianco → trasparente, il nero resta nero e pieno
        let mut pm = tiny_skia::Pixmap::new(2, 1).unwrap();
        pm.pixels_mut()[0] = tiny_skia::ColorU8::from_rgba(255, 255, 255, 255).premultiply();
        pm.pixels_mut()[1] = tiny_skia::ColorU8::from_rgba(0, 0, 0, 255).premultiply();
        let out = tiny_skia::Pixmap::decode_png(&white_to_alpha(&pm.encode_png().unwrap()).unwrap()).unwrap();
        assert_eq!(out.pixels()[0].alpha(), 0);
        assert_eq!(out.pixels()[1].demultiply(), tiny_skia::ColorU8::from_rgba(0, 0, 0, 255));
        let r = fit([0.2, 0.2, 0.8, 0.5], 2.0);
        assert!((r[2] - 0.6).abs() < 1e-6 && (r[3] - 0.3).abs() < 1e-6);
    }

    /// Leggere e annotare un foglio scritto «a mano» (col corsivo di
    /// handwrite, che fa da scrittura dell'utente), poi Annulla. Scrive le
    /// immagini in $BOARD_OUT se c'è: cargo test -- --ignored annota_la_lavagna
    #[test]
    #[ignore]
    fn annota_la_lavagna() {
        use crate::stroke::{PenEvent, PenPhase};
        let hub = test_hub();
        let g = hub.lock().geom();
        let aspect = g.w / g.h;
        let mut strokes = Vec::new();
        let righe = ["Esercizio 3: 12 + 7 = 19", "Il perimetro del quadrato è 4 × l", "Domani: ripassare le frazioni"];
        for (i, t) in righe.iter().enumerate() {
            let laid = crate::handwrite::layout(t, crate::handwrite::Style::Corsivo, 0.028, aspect, (0.08, 0.10 + i as f32 * 0.07), 0.88, false);
            for pts in laid.strokes {
                let mut st = Stroke::new([20, 20, 30], 2.0);
                for (k, (u, v)) in pts.into_iter().enumerate() {
                    let (x, y) = g.pad(u, v);
                    st.points.push(PenEvent { x, y, pressure: 0.5, phase: if k == 0 { PenPhase::Down } else { PenPhase::Move } });
                }
                strokes.push(st);
            }
        }
        hub.edit(|s| s.set_page(strokes.clone(), None));
        let r = execute("lavagna_leggi", &json!({})).unwrap();
        println!("{}", r.text);
        assert_eq!(r.images.len(), 1);
        let out = std::env::var("BOARD_OUT").ok().map(std::path::PathBuf::from);
        if let Some(o) = &out {
            std::fs::create_dir_all(o).unwrap();
            std::fs::write(o.join("letto.png"), &r.images[0]).unwrap();
        }
        assert!(r.text.contains("r3") && !r.text.contains("r4 ["), "{}", r.text);
        let a = execute(
            "lavagna_annota",
            &json!({ "segni": [
                { "tipo": "cerchia", "su": "r1p6" },
                { "tipo": "spunta", "su": "r1", "colore": "verde" },
                { "tipo": "sottolinea", "su": "r2p2-p3", "colore": "blu" },
                { "tipo": "evidenzia", "su": "r3p1" },
                { "tipo": "testo", "su": "r2", "dove": "sotto", "testo": "Giusto: P = 4 × l. Così l'area è l²?", "colore": "rosso" },
                { "tipo": "freccia", "da": [700, 520], "a": "r3p4", "colore": "viola" },
                { "tipo": "testo", "riquadro": [600, 480, 950, 600], "testo": "anche i decimali!", "stile": "corsivo", "colore": "viola" },
                { "tipo": "barra", "su": "r3p3" }
            ] }),
        )
        .unwrap();
        println!("{}", a.text);
        assert!(a.text.starts_with("Fatto"), "{}", a.text);
        let n = a.edit.unwrap();
        let after = hub.lock().drawing.strokes.len();
        assert!(after > strokes.len());
        if let Some(o) = &out {
            let page = hub.lock().page();
            std::fs::write(o.join("annotato.png"), crate::export::page_to_png(&page, 1600).unwrap()).unwrap();
        }
        // un nome sbagliato: niente cambia, l'errore lo dice
        let bad = execute("lavagna_annota", &json!({ "segni": [{ "tipo": "cerchia", "su": "r9p1" }] })).unwrap();
        assert!(bad.text.starts_with("ERRORE") && bad.text.contains("r9p1"), "{}", bad.text);
        assert_eq!(hub.lock().drawing.strokes.len(), after);
        // l'utente scrive ancora, poi Annulla: via i segni, resta il tratto nuovo
        hub.edit(|s| s.add_strokes(vec![crate::boardread::tests::line(&g, (0.8, 0.9), (0.9, 0.92))]));
        undo(n).unwrap();
        assert_eq!(hub.lock().drawing.strokes.len(), strokes.len() + 1);
        assert!(undo(n).is_err());
    }

    /// Spostare, ridimensionare, ricolorare e cancellare; la conferma per
    /// cancellare quasi tutto. cargo test -- --ignored modifica_la_lavagna
    #[test]
    #[ignore]
    fn modifica_la_lavagna() {
        use crate::stroke::{PenEvent, PenPhase};
        let hub = test_hub();
        let g = hub.lock().geom();
        let aspect = g.w / g.h;
        let mut strokes = Vec::new();
        for (i, t) in ["Lista della spesa", "pane e latte", "mele rosse"].iter().enumerate() {
            let laid = crate::handwrite::layout(t, crate::handwrite::Style::Corsivo, 0.03, aspect, (0.1, 0.1 + i as f32 * 0.08), 0.85, false);
            for pts in laid.strokes {
                let mut st = Stroke::new([20, 20, 30], 2.0);
                for (k, (u, v)) in pts.into_iter().enumerate() {
                    let (x, y) = g.pad(u, v);
                    st.points.push(PenEvent { x, y, pressure: 0.5, phase: if k == 0 { PenPhase::Down } else { PenPhase::Move } });
                }
                strokes.push(st);
            }
        }
        hub.edit(|s| s.set_page(strokes.clone(), None));
        let r = execute("lavagna_leggi", &json!({})).unwrap();
        assert!(r.text.contains("r3 ["), "{}", r.text);
        let box_of = |id: &str| {
            let s = hub.lock();
            crate::boardread::read(&s.drawing.strokes, &s.geom(), None).find(id).unwrap()
        };
        let r3 = box_of("r3");
        let o = execute(
            "lavagna_modifica",
            &json!({ "azioni": [
                { "azione": "ricolora", "su": "r1", "nuovo_colore": "blu" },
                { "azione": "ridimensiona", "su": "r1", "fattore": 1.4 },
                { "azione": "sposta", "su": "r3", "dy": 300 },
                { "azione": "cancella", "su": "r2p2" }
            ] }),
        )
        .unwrap();
        assert!(o.text.starts_with("Fatto"), "{}", o.text);
        let n1 = o.edit.unwrap();
        {
            let s = hub.lock();
            let blue = s.drawing.strokes.iter().filter(|st| st.color == [30, 80, 200]).count();
            assert!(blue > 0);
            assert!(s.drawing.strokes.len() < strokes.len(), "una parola in meno");
        }
        if let Ok(o) = std::env::var("BOARD_OUT") {
            std::fs::create_dir_all(&o).unwrap();
            std::fs::write(std::path::Path::new(&o).join("modificato.png"), crate::export::page_to_png(&hub.lock().page(), 1400).unwrap()).unwrap();
        }
        // «mele rosse» è scesa di 0.3: la riga più in basso
        let bottom = box_of("tutto")[3];
        assert!((bottom - r3[3] - 0.3).abs() < 0.01, "{bottom} {r3:?}");
        // cancellare tutto: prima la conferma, e non nello stesso giro
        execute("lavagna_leggi", &json!({})).unwrap();
        let ask = execute("lavagna_modifica", &json!({ "azioni": [{ "azione": "cancella", "su": "tutto" }] })).unwrap();
        assert!(ask.text.starts_with("NON FATTO") && ask.edit.is_none(), "{}", ask.text);
        let code: String = ask.text.split("\"conferma\": \"").nth(1).unwrap().chars().take(4).collect();
        let again = execute("lavagna_modifica", &json!({ "azioni": [{ "azione": "cancella", "su": "tutto" }], "conferma": code })).unwrap();
        assert!(again.text.contains("aspetta la sua risposta"), "{}", again.text);
        new_turn();
        let done = execute("lavagna_modifica", &json!({ "azioni": [{ "azione": "cancella", "su": "tutto" }], "conferma": code })).unwrap();
        assert!(done.text.starts_with("Fatto"), "{}", done.text);
        assert!(hub.lock().drawing.strokes.is_empty());
        // Annulla dalla chat, uno alla volta
        assert!(undo(n1).is_err());
        undo(done.edit.unwrap()).unwrap();
        undo(n1).unwrap();
        assert_eq!(crate::stroke::strokes_sig(&hub.lock().drawing.strokes), crate::stroke::strokes_sig(&strokes));
        // un nome che non c'è
        let bad = execute("lavagna_modifica", &json!({ "azioni": [{ "azione": "cancella", "su": "r7" }] })).unwrap();
        assert!(bad.text.starts_with("ERRORE"), "{}", bad.text);
    }

    /// Il grafico al posto degli assi disegnati a mano, poi Annulla. Cambia
    /// la cartella corrente: cargo test -- --ignored --test-threads=1 grafico_sulla_lavagna
    #[test]
    #[ignore]
    fn grafico_sulla_lavagna() {
        use crate::stroke::{PenEvent, PenPhase};
        let work = std::env::temp_dir().join(format!("openslate-board-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&work);
        std::fs::create_dir_all(&work).unwrap();
        std::env::set_current_dir(&work).unwrap();
        let hub = test_hub();
        let g = hub.lock().geom();
        let line = |a: (f32, f32), b: (f32, f32)| {
            let mut st = Stroke::new([0, 0, 0], 2.0);
            for k in 0..=10 {
                let t = k as f32 / 10.0;
                let (x, y) = g.pad(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
                st.points.push(PenEvent { x, y, pressure: 0.5, phase: PenPhase::Move });
            }
            st
        };
        let strokes = vec![
            line((0.10, 0.10), (0.14, 0.12)),  // testo
            line((0.30, 0.45), (0.72, 0.43)),  // asse x
            line((0.52, 0.28), (0.53, 0.65)),  // asse y
            line((0.36, 0.33), (0.41, 0.34)),  // «P(-1,2)»
            line((0.10, 0.85), (0.16, 0.86)),  // «Giusto?»
        ];
        hub.edit(|s| s.set_page(strokes.clone(), None));
        let spec = json!({ "x": { "min": -4, "max": 4 }, "series": [{ "type": "function", "expr": "1 - x" }, { "type": "points", "points": [[-1, 2]], "names": ["$P(-1, 2)$"] }] });
        let id = crate::charts::draw(&spec).unwrap().id;
        let o = execute("lavagna_inserisci_grafico", &json!({ "grafico": id })).unwrap();
        assert!(o.text.starts_with("Fatto") && o.text.contains("3 tratti tolti"), "{}", o.text);
        {
            let s = hub.lock();
            assert_eq!(s.drawing.strokes.len(), 2);
            let bg = s.bg.as_ref().unwrap();
            assert!(bg.rect[0] < 0.35 && bg.rect[1] < 0.33 && bg.rect[0] + bg.rect[2] > 0.6, "{:?}", bg.rect);
        }
        // un secondo grafico sotto lo scritto: le immagini si fondono
        let o2 = execute("lavagna_inserisci_grafico", &json!({ "grafico": id, "al_posto_del_disegno": false })).unwrap();
        assert!(o2.text.starts_with("Fatto"), "{}", o2.text);
        assert_eq!(hub.lock().bg.as_ref().unwrap().rect, [0.0, 0.0, 1.0, 1.0]);
        // prima il più recente
        assert!(undo(o.edit.unwrap()).is_err());
        undo(o2.edit.unwrap()).unwrap();
        // l'utente scrive ancora: Annulla toglie il grafico ma non il tratto nuovo
        hub.edit(|s| s.add_strokes(vec![line((0.8, 0.9), (0.9, 0.92))]));
        undo(o.edit.unwrap()).unwrap();
        {
            let s = hub.lock();
            assert_eq!(s.drawing.strokes.len(), 6);
            assert!(s.bg.is_none());
        }
        // e Cmd+Z rimette il grafico (l'annullamento della chat è annullabile)
        assert!(hub.edit(|s| s.undo()));
        assert!(hub.lock().bg.is_some());
    }
}
