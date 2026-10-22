//! La chat AI sull'archivio delle pagine: cercare (anche dentro le
//! trascrizioni), guardare una pagina, aprirla sulla lavagna, salvare la
//! lavagna, rinominare, esportare, spostare nel cestino e riprendere.
//!
//! L'archivio vive nell'interfaccia (è lei che lo tiene in memoria e lo
//! mostra): la chat, che lavora in un altro thread, le manda una richiesta e
//! aspetta la risposta (`ask`). Le esportazioni si scrivono qui, fuori
//! dall'interfaccia, nella cartella dove si esporta di solito, senza mai
//! sovrascrivere. Il cestino chiede sempre il sì dell'utente nella chat.

use crate::scene::Page;
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

/// Una pagina dell'archivio, in breve.
#[derive(Clone, Debug)]
pub struct Item {
    pub id: String,
    pub title: String,
    /// "AAAA-MM-GG hh:mm:ss"
    pub created: String,
    pub source: String,
    pub strokes: usize,
    pub chart: bool,
    /// È la pagina aperta sulla lavagna.
    pub on_board: bool,
}

/// Cosa la chat chiede all'interfaccia.
pub enum Req {
    List,
    /// Le pagine (per guardarle o esportarle) e la cartella delle esportazioni.
    Pages(Vec<String>),
    Open(String),
    Save { title: Option<String>, new_page: bool, as_new: bool },
    Rename { id: String, title: String },
    Trash(Vec<String>),
    Restore(Vec<String>),
}

/// Cosa risponde.
pub enum Data {
    Text(String),
    List(Vec<Item>),
    Pages { pages: Vec<(String, String, Page)>, dir: PathBuf },
}

pub type Reply = std::sync::mpsc::Sender<Result<Data, String>>;
type Sink = Box<dyn Fn(Req, Reply) + Send + Sync>;

static SINK: OnceLock<Sink> = OnceLock::new();
static WAKE: OnceLock<eframe::egui::Context> = OnceLock::new();

/// L'interfaccia riceve le richieste (all'avvio).
pub fn set_sink(f: impl Fn(Req, Reply) + Send + Sync + 'static) {
    let _ = SINK.set(Box::new(f));
}

/// Per svegliare l'interfaccia quando arriva una richiesta.
pub fn set_waker(ctx: eframe::egui::Context) {
    let _ = WAKE.set(ctx);
}

/// C'è l'interfaccia che risponde (non in riga di comando).
pub fn available() -> bool {
    SINK.get().is_some()
}

fn ask(req: Req) -> Result<Data> {
    let sink = SINK.get().context("l'archivio c'è solo nell'app")?;
    let (tx, rx) = std::sync::mpsc::channel();
    sink(req, tx);
    if let Some(c) = WAKE.get() {
        c.request_repaint();
    }
    match rx.recv_timeout(Duration::from_secs(20)) {
        Ok(r) => r.map_err(|e| anyhow::anyhow!(e)),
        Err(_) => bail!("l'app non risponde (finestra ridotta a icona o occupata): riprova"),
    }
}

fn list() -> Result<Vec<Item>> {
    match ask(Req::List)? {
        Data::List(v) => Ok(v),
        _ => bail!("risposta inattesa"),
    }
}

fn text(req: Req) -> Result<String> {
    match ask(req)? {
        Data::Text(t) => Ok(t),
        _ => bail!("risposta inattesa"),
    }
}

pub fn definitions() -> Vec<Value> {
    let f = |name: &str, desc: &str, props: Value, req: &[&str]| {
        json!({ "type": "function", "function": { "name": name, "description": desc,
            "parameters": { "type": "object", "properties": props, "required": req } } })
    };
    vec![
        f(
            "archivio_cerca",
            "Cerca fra le pagine dell'archivio di OpenSlate (pagine scaricate dal pad e salvate dalla lavagna): per titolo, data e \
             testo delle trascrizioni. Senza «testo» dà le più recenti. Ogni pagina ha un id (es. 20261021-093000) da usare con \
             gli altri strumenti archivio_*. «cestino»: true cerca fra le pagine nel cestino.",
            json!({
                "testo": { "type": "string", "description": "Parole da cercare (tutte) nel titolo, nella data o nella trascrizione" },
                "dal": { "type": "string", "description": "Data di inizio, AAAA-MM-GG" },
                "al": { "type": "string", "description": "Data di fine, AAAA-MM-GG" },
                "fonte": { "type": "string", "enum": ["pad", "lavagna"] },
                "quanti": { "type": "integer", "description": "Al massimo (di serie 20)" },
                "cestino": { "type": "boolean" }
            }),
            &[],
        ),
        f(
            "archivio_guarda",
            "Guarda una pagina dell'archivio: l'immagine della pagina e la sua trascrizione, se c'è.",
            json!({ "id": { "type": "string" } }),
            &["id"],
        ),
        f(
            "archivio_apri",
            "Apre una pagina dell'archivio sulla lavagna (al posto di quella che c'è, che se non è nell'archivio ci va prima da sola). \
             Poi la puoi leggere e annotare con gli strumenti lavagna_*.",
            json!({ "id": { "type": "string" } }),
            &["id"],
        ),
        f(
            "archivio_salva",
            "Salva nell'archivio il foglio che c'è sulla lavagna. Se è una pagina aperta dall'archivio la aggiorna (come_nuova: true \
             per farne una pagina nuova). «titolo» per darle un nome; nuova_pagina: true poi pulisce la lavagna per una pagina nuova.",
            json!({
                "titolo": { "type": "string" },
                "nuova_pagina": { "type": "boolean" },
                "come_nuova": { "type": "boolean" }
            }),
            &[],
        ),
        f(
            "archivio_rinomina",
            "Cambia il titolo di una pagina dell'archivio (vuoto = torna la data).",
            json!({ "id": { "type": "string" }, "titolo": { "type": "string" } }),
            &["id", "titolo"],
        ),
        f(
            "archivio_esporta",
            "Esporta pagine dell'archivio nella cartella dove l'utente esporta di solito: pdf (tutte in un file; le pagine trascritte \
             con il testo, così il PDF si cerca), png o svg, jpeg o webp (immagini leggere da mandare), word (la pagina come inchiostro \
             modificabile e la trascrizione), xournal (Xournal++), gif o svg_animato (la scrittura che si rifà tratto dopo tratto), \
             latex o typst (la trascrizione: solo pagine trascritte). Un file per pagina, tranne il pdf. Non sovrascrive niente. Dice dove sono i file.",
            json!({
                "ids": { "type": "array", "items": { "type": "string" } },
                "formato": { "type": "string", "enum": ["pdf", "png", "svg", "jpeg", "webp", "word", "xournal", "gif", "svg_animato", "latex", "typst"] },
                "nome": { "type": "string", "description": "Nome del file (senza estensione), facoltativo" }
            }),
            &["ids", "formato"],
        ),
        f(
            "archivio_cestino",
            "Sposta pagine dell'archivio nel cestino (si possono riprendere con archivio_ripristina). Chiede sempre la conferma \
             dell'utente: la prima volta dà un codice; chiedi all'utente e, se dice sì, richiama con «conferma» e il codice.",
            json!({
                "ids": { "type": "array", "items": { "type": "string" } },
                "conferma": { "type": "string" }
            }),
            &["ids"],
        ),
        f(
            "archivio_ripristina",
            "Riprende dal cestino pagine dell'archivio (gli id da archivio_cerca con cestino: true).",
            json!({ "ids": { "type": "array", "items": { "type": "string" } } }),
            &["ids"],
        ),
    ]
}

pub fn system_note() -> String {
    "\n\nC'è anche l'archivio delle pagine di OpenSlate (quelle scaricate dal pad e salvate dalla lavagna): archivio_cerca per \
     trovarle (anche per quello che c'è scritto, dalle trascrizioni), archivio_guarda per vederne una, archivio_apri per \
     metterla sulla lavagna, archivio_salva, archivio_rinomina, archivio_esporta, archivio_cestino (sempre con il sì \
     dell'utente) e archivio_ripristina. Non dire che non puoi: gli strumenti ci sono."
        .into()
}

/// Riga breve per la chat.
pub fn label(name: &str) -> Option<&'static str> {
    Some(match name {
        "archivio_cerca" => "Cerco nell'archivio",
        "archivio_guarda" => "Guardo una pagina dell'archivio",
        "archivio_apri" => "Apro la pagina sulla lavagna",
        "archivio_salva" => "Salvo la lavagna nell'archivio",
        "archivio_rinomina" => "Rinomino la pagina",
        "archivio_esporta" => "Esporto le pagine",
        "archivio_cestino" => "Sposto nel cestino",
        "archivio_ripristina" => "Riprendo dal cestino",
        _ => return None,
    })
}

/// Esegue uno strumento dell'archivio; None se `name` non è suo.
pub fn execute(name: &str, args: &Value) -> Option<crate::docagent::Outcome> {
    use crate::docagent::Outcome;
    let r: Result<Outcome> = match name {
        "archivio_cerca" => search(args).map(|t| Outcome { text: t, short: "archivio".into(), ..Default::default() }),
        "archivio_guarda" => look(args),
        "archivio_apri" => id_of(args).and_then(|id| text(Req::Open(id))).map(|t| Outcome { text: t, short: "pagina sulla lavagna".into(), ..Default::default() }),
        "archivio_salva" => text(Req::Save {
            title: args["titolo"].as_str().map(|t| t.trim().to_string()).filter(|t| !t.is_empty()),
            new_page: args["nuova_pagina"].as_bool().unwrap_or(false),
            as_new: args["come_nuova"].as_bool().unwrap_or(false),
        })
        .map(|t| Outcome { text: t, short: "lavagna nell'archivio".into(), ..Default::default() }),
        "archivio_rinomina" => id_of(args)
            .and_then(|id| text(Req::Rename { id, title: args["titolo"].as_str().unwrap_or("").trim().chars().take(120).collect() }))
            .map(|t| Outcome { text: t, short: "pagina rinominata".into(), ..Default::default() }),
        "archivio_esporta" => export(args).map(|t| Outcome { text: t, short: "pagine esportate".into(), ..Default::default() }),
        "archivio_cestino" => trash(args),
        "archivio_ripristina" => ids_of(args)
            .and_then(|ids| text(Req::Restore(ids)))
            .map(|t| Outcome { text: t, short: "pagine riprese".into(), ..Default::default() }),
        _ => return None,
    };
    Some(r.unwrap_or_else(|e| Outcome { text: format!("ERRORE: {e:#}"), short: format!("errore: {e:#}"), ..Default::default() }))
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() < 80 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn id_of(args: &Value) -> Result<String> {
    let id = args["id"].as_str().context("manca «id» (da archivio_cerca)")?.trim().to_string();
    if !valid_id(&id) {
        bail!("id «{id}» non valido: usa quelli di archivio_cerca");
    }
    Ok(id)
}

fn ids_of(args: &Value) -> Result<Vec<String>> {
    let ids: Vec<String> = match &args["ids"] {
        Value::Array(a) => a.iter().filter_map(|v| v.as_str().map(|s| s.trim().to_string())).collect(),
        Value::String(s) => vec![s.trim().to_string()],
        _ => match args["id"].as_str() {
            Some(s) => vec![s.trim().to_string()],
            None => bail!("manca «ids» (da archivio_cerca)"),
        },
    };
    if ids.is_empty() || ids.len() > 200 {
        bail!("da 1 a 200 pagine");
    }
    if let Some(bad) = ids.iter().find(|i| !valid_id(i)) {
        bail!("id «{bad}» non valido: usa quelli di archivio_cerca");
    }
    Ok(ids)
}

/// Trascrizioni collegate alle pagine (`lib:<id>` → nome dell'appunto).
fn transcriptions() -> std::collections::HashMap<String, String> {
    let path = std::path::Path::new(crate::export::EXPORT_DIR).join(".trascrizioni.json");
    let map: Value = std::fs::read_to_string(path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    let mut out = std::collections::HashMap::new();
    if let Some(m) = map.as_object() {
        for (k, v) in m {
            let Some(id) = k.strip_prefix("lib:") else { continue };
            if let Some(n) = v.as_str().or(v["note"].as_str()) {
                out.insert(id.to_string(), n.to_string());
            }
        }
    }
    out
}

fn note_text(name: &str) -> Option<String> {
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return None;
    }
    std::fs::read_to_string(std::path::Path::new(crate::export::EXPORT_DIR).join(name)).ok()
}

/// "AAAA-MM-GG" (o "GG/MM/AAAA") → "AAAA-MM-GG".
fn day(s: &str) -> Option<String> {
    let s = s.trim();
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(d.format("%Y-%m-%d").to_string());
    }
    chrono::NaiveDate::parse_from_str(s, "%d/%m/%Y").ok().map(|d| d.format("%Y-%m-%d").to_string())
}

/// Le pagine nel cestino (si leggono dai file: non sono in memoria).
fn trash_items() -> Vec<Item> {
    let dir = std::path::Path::new(crate::library::LIB_DIR).join("cestino");
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) != Some("json") {
            continue;
        }
        let Some(id) = p.file_stem().and_then(|s| s.to_str()).map(String::from) else { continue };
        let Ok(v) = std::fs::read_to_string(&p).map(|t| serde_json::from_str::<Value>(&t).unwrap_or_default()) else { continue };
        out.push(Item {
            id,
            title: v["title"].as_str().unwrap_or("").to_string(),
            created: v["created"].as_str().unwrap_or("").to_string(),
            source: v["source"].as_str().unwrap_or("").to_string(),
            strokes: v["strokes"].as_array().map_or(0, |a| a.len()),
            chart: !v["chart"].is_null(),
            on_board: false,
        });
    }
    out.sort_by(|a, b| b.created.cmp(&a.created));
    out
}

fn shown_title(it: &Item) -> String {
    if !it.title.trim().is_empty() {
        return it.title.clone();
    }
    chrono::NaiveDateTime::parse_from_str(&it.created, "%Y-%m-%d %H:%M:%S").map(|t| t.format("%d/%m/%Y %H:%M").to_string()).unwrap_or(it.created.clone())
}

fn search(args: &Value) -> Result<String> {
    let in_trash = args["cestino"].as_bool().unwrap_or(false);
    let items = if in_trash { trash_items() } else { list()? };
    let total = items.len();
    let q: Vec<String> = args["testo"].as_str().unwrap_or("").to_lowercase().split_whitespace().map(String::from).collect();
    let from = args["dal"].as_str().and_then(day);
    let to = args["al"].as_str().and_then(day);
    let source = args["fonte"].as_str().map(|s| if s.starts_with("lav") || s == "board" { crate::library::SOURCE_BOARD } else { crate::library::SOURCE_PAD });
    let max = args["quanti"].as_u64().unwrap_or(20).clamp(1, 100) as usize;
    let notes = transcriptions();
    let mut out = Vec::new();
    let mut found = 0;
    for it in &items {
        let date = it.created.get(..10).unwrap_or("");
        if from.as_deref().is_some_and(|f| date < f) || to.as_deref().is_some_and(|t| date > t) || source.is_some_and(|s| it.source != s) {
            continue;
        }
        let note = notes.get(&it.id);
        let mut snippet = None;
        if !q.is_empty() {
            let title = shown_title(it).to_lowercase();
            let body = note.and_then(|n| note_text(n)).unwrap_or_default();
            let low = body.to_lowercase();
            let hay = format!("{title} {} {date} {low}", it.created);
            if !q.iter().all(|w| hay.contains(w.as_str())) {
                continue;
            }
            // un pezzo della trascrizione attorno alla prima parola trovata
            if let Some(pos) = low.find(q[0].as_str()) {
                let start = low[..pos].char_indices().rev().nth(60).map_or(0, |(i, _)| i);
                let piece: String = low[start..].chars().take(160).collect();
                snippet = Some(piece.split_whitespace().collect::<Vec<_>>().join(" "));
            }
        }
        found += 1;
        if out.len() < max {
            let mut line = format!("{} — «{}» — {} — {} — {} tratti", it.id, shown_title(it), it.created, it.source, it.strokes);
            if it.chart {
                line.push_str(" — col grafico dell'AI");
            }
            if it.on_board {
                line.push_str(" — APERTA SULLA LAVAGNA");
            }
            if let Some(n) = note {
                line.push_str(&format!(" — trascrizione: {n}"));
            }
            if let Some(s) = snippet {
                line.push_str(&format!("\n    «…{s}…»"));
            }
            out.push(line);
        }
    }
    let place = if in_trash { "nel cestino" } else { "nell'archivio" };
    if out.is_empty() {
        return Ok(format!("Nessuna pagina trovata {place} ({} in tutto).", pages(total)));
    }
    let more = if found > out.len() { format!(" (ne mostro {}: restringi la ricerca o aumenta «quanti»)", out.len()) } else { String::new() };
    let what = if found == 1 { "1 pagina trovata".to_string() } else { format!("{found} pagine trovate") };
    Ok(format!("{what} {place} su {total}{more}, dalla più recente:\n{}", out.join("\n")))
}

fn pages(n: usize) -> String {
    if n == 1 { "1 pagina".into() } else { format!("{n} pagine") }
}

fn look(args: &Value) -> Result<crate::docagent::Outcome> {
    let id = id_of(args)?;
    let Data::Pages { pages, .. } = ask(Req::Pages(vec![id.clone()]))? else { bail!("risposta inattesa") };
    let (title, _, page) = pages.into_iter().next().with_context(|| format!("non trovo la pagina «{id}»"))?;
    let png = crate::export::page_to_png(&page, 1400)?;
    let mut text = format!("Pagina {id} «{title}»: l'immagine è allegata.");
    match transcriptions().get(&id).and_then(|n| note_text(n).map(|t| (n.clone(), t))) {
        Some((n, t)) => {
            let t: String = t.chars().take(6000).collect();
            text.push_str(&format!("\nTrascrizione ({n}):\n{t}"));
        }
        None => text.push_str(" Non ha una trascrizione."),
    }
    Ok(crate::docagent::Outcome { text, short: format!("pagina «{title}»"), images: vec![png], ..Default::default() })
}

fn file_stem(t: &str) -> String {
    let s: String = t.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' }).collect();
    let s = s.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");
    if s.is_empty() { "pagina".into() } else { s.chars().take(60).collect() }
}

fn export(args: &Value) -> Result<String> {
    use crate::formats::{Item, Kind, Opts};
    let ids = ids_of(args)?;
    let name_fmt = args["formato"].as_str().unwrap_or("pdf").trim().to_lowercase();
    let Some(fmt) = Kind::from_name(&name_fmt) else {
        bail!("formato «{name_fmt}»: pdf, png, svg, jpeg, webp, word, xournal, gif, svg_animato, latex o typst");
    };
    let Data::Pages { pages, dir } = ask(Req::Pages(ids.clone()))? else { bail!("risposta inattesa") };
    if pages.is_empty() {
        bail!("nessuna di queste pagine è nell'archivio");
    }
    let missing: Vec<&String> = ids.iter().filter(|i| !pages.iter().any(|p| &p.1 == *i)).collect();
    let notes = transcriptions();
    let mut items: Vec<Item> = pages
        .into_iter()
        .filter(|p| !p.2.is_empty())
        .map(|(title, id, page)| {
            let note = notes.get(&id).filter(|n| note_text(n).is_some()).map(|n| std::path::Path::new(crate::export::EXPORT_DIR).join(n));
            Item { title, page, note }
        })
        .collect();
    if items.is_empty() {
        bail!("le pagine sono vuote: niente da esportare");
    }
    let mut untranscribed = Vec::new();
    if fmt.needs_note() {
        untranscribed = items.iter().filter(|i| i.note.is_none()).map(|i| i.title.clone()).collect();
        items.retain(|i| i.note.is_some());
        if items.is_empty() {
            bail!("nessuna di queste pagine è trascritta: LaTeX e Typst esportano la trascrizione (prima «Trascrivi»)");
        }
    }
    std::fs::create_dir_all(&dir)?;
    let name = args["nome"].as_str().map(file_stem);
    let mut written: Vec<PathBuf> = Vec::new();
    if fmt.one_file() {
        let stem = name.unwrap_or_else(|| if items.len() == 1 { file_stem(&items[0].title) } else { format!("pagine-{}", items.len()) });
        let path = free_path(&dir, &stem, fmt.ext());
        crate::formats::write_pdf(&items, &path)?;
        written.push(path);
    } else {
        let opts = Opts::default();
        for (k, item) in items.iter().enumerate() {
            let stem = match &name {
                Some(n) if items.len() > 1 => format!("{n}-{}", k + 1),
                Some(n) => n.clone(),
                None => file_stem(&item.title),
            };
            let path = free_path(&dir, &stem, fmt.ext());
            written.extend(crate::formats::write_one(fmt, item, &path, &opts)?);
        }
    }
    let list: Vec<String> = written.iter().map(|p| p.display().to_string()).collect();
    let mut t = format!("Fatto: {} file in {}:\n{}", written.len(), dir.display(), list.join("\n"));
    if !missing.is_empty() {
        t.push_str(&format!("\nNon trovate: {}", missing.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")));
    }
    if !untranscribed.is_empty() {
        t.push_str(&format!("\nNon trascritte (saltate): {}", untranscribed.join(", ")));
    }
    Ok(t)
}

/// Percorso libero in `dir` per `<stem>.<ext>`: se c'è già, `<stem>-2.<ext>`, …
fn free_path(dir: &std::path::Path, stem: &str, ext: &str) -> PathBuf {
    let mut p = dir.join(format!("{stem}.{ext}"));
    let mut n = 2;
    while p.exists() {
        p = dir.join(format!("{stem}-{n}.{ext}"));
        n += 1;
    }
    p
}

fn trash(args: &Value) -> Result<crate::docagent::Outcome> {
    use crate::boardagent::Gate;
    let mut ids = ids_of(args)?;
    ids.sort();
    ids.dedup();
    let items = list()?;
    let missing: Vec<&String> = ids.iter().filter(|i| !items.iter().any(|it| &it.id == *i)).collect();
    if !missing.is_empty() {
        bail!("non trovo nell'archivio: {}", missing.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "));
    }
    let sig = ids.iter().fold(0x51u64, |h, id| id.bytes().fold(h, |h, b| (h ^ b as u64).wrapping_mul(0x100_0000_01B3)) ^ 0xFF);
    let names: Vec<String> = ids.iter().filter_map(|i| items.iter().find(|it| &it.id == i)).map(|it| format!("«{}»", shown_title(it))).collect();
    let short_names = if names.len() > 6 { format!("{} e altre {}", names[..6].join(", "), names.len() - 6) } else { names.join(", ") };
    match crate::boardagent::gate(sig, args["conferma"].as_str()) {
        Gate::Ask(code) => Ok(crate::docagent::Outcome {
            text: format!(
                "NON FATTO: sposterebbe nel cestino {} ({short_names}). Chiedi all'utente se va bene; se risponde di sì, \
                 richiama archivio_cestino con gli stessi ids e \"conferma\": \"{code}\".",
                pages(ids.len())
            ),
            short: "conferma chiesta".into(),
            ..Default::default()
        }),
        Gate::Wait => Ok(crate::docagent::Outcome {
            text: "NON FATTO: la conferma la dà l'utente. Chiedigli se va bene e aspetta la sua risposta prima di richiamare.".into(),
            short: "conferma chiesta".into(),
            ..Default::default()
        }),
        Gate::Go => {
            let t = text(Req::Trash(ids))?;
            Ok(crate::docagent::Outcome { text: t, short: "pagine nel cestino".into(), ..Default::default() })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Gli strumenti dell'archivio con un'«interfaccia» finta che tiene un
    /// archivio vero in una cartella di prova. Cambia la cartella corrente:
    /// cargo test -- --ignored --test-threads=1 archivio_dalla_chat
    #[test]
    #[ignore]
    fn archivio_dalla_chat() {
        use crate::library::{Library, Meta, SOURCE_BOARD, SOURCE_PAD};
        use std::sync::{Arc, Mutex};
        let work = std::env::temp_dir().join(format!("openslate-libagent-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&work);
        std::fs::create_dir_all(work.join("exports")).unwrap();
        std::env::set_current_dir(&work).unwrap();
        let g = crate::boardread::tests::geom();
        let d = crate::stroke::Drawing { strokes: crate::boardread::tests::word(&g, 0.1, 0.1, 5).0, width: 0.0, height: 0.0 };
        let lib = Arc::new(Mutex::new(Library::open()));
        let t = |s: &str| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").unwrap();
        let a = lib.lock().unwrap().add(&d, Meta::new(t("2026-10-18 10:00:00"), SOURCE_PAD)).unwrap();
        let b = lib.lock().unwrap().add(&d, Meta::new(t("2026-10-20 09:30:00"), SOURCE_BOARD)).unwrap();
        lib.lock().unwrap().rename(&b, "Riunione Tiani").unwrap();
        // la pagina «a» ha una trascrizione che parla di bilancio
        std::fs::write("exports/spesa.md", "Lista: pane, latte. Controllare il bilancio di ottobre.").unwrap();
        std::fs::write("exports/.trascrizioni.json", json!({ format!("lib:{a}"): { "note": "spesa.md", "sig": "1" } }).to_string()).unwrap();
        let out = work.join("esportati");
        let l2 = lib.clone();
        let out2 = out.clone();
        set_sink(move |req, reply| {
            let out = &out2;
            let mut lib = l2.lock().unwrap();
            let r = match req {
                Req::List => Ok(Data::List(
                    lib.entries
                        .iter()
                        .map(|e| Item { id: e.id.clone(), title: e.meta.title.clone(), created: e.meta.created.clone(), source: e.meta.source.clone(), strokes: e.drawing.strokes.len(), chart: false, on_board: false })
                        .collect(),
                )),
                Req::Pages(ids) => Ok(Data::Pages {
                    pages: ids
                        .iter()
                        .filter_map(|id| lib.get(id))
                        .map(|e| (e.meta.display_title(), e.id.clone(), crate::scene::Page::from_drawing(&e.drawing, Default::default())))
                        .collect(),
                    dir: out.clone(),
                }),
                Req::Rename { id, title } => lib.rename(&id, &title).map(|_| Data::Text("Fatto".into())).map_err(|e| e.to_string()),
                Req::Trash(ids) => lib.trash(&ids).map(|n| Data::Text(format!("Fatto: {n}"))).map_err(|e| e.to_string()),
                Req::Restore(ids) => lib.restore(&ids).map(|n| Data::Text(format!("Fatto: {n}"))).map_err(|e| e.to_string()),
                _ => Err("non in questa prova".into()),
            };
            let _ = reply.send(r);
        });
        let run = |name: &str, args: Value| execute(name, &args).unwrap();
        // tutte, dalla più recente
        let all = run("archivio_cerca", json!({}));
        assert!(all.text.starts_with("2 pagine trovate") && all.text.find(&b).unwrap() < all.text.find(&a).unwrap(), "{}", all.text);
        // per titolo, per trascrizione, per data
        assert!(run("archivio_cerca", json!({ "testo": "tiani" })).text.contains(&b));
        let bil = run("archivio_cerca", json!({ "testo": "bilancio ottobre" })).text;
        assert!(bil.contains(&a) && !bil.contains(&b) && bil.contains("spesa.md") && bil.contains("bilancio"), "{bil}");
        assert!(run("archivio_cerca", json!({ "dal": "2026-10-19" })).text.starts_with("1 pagina trovata"));
        assert!(run("archivio_cerca", json!({ "testo": "nessunacosa" })).text.starts_with("Nessuna"));
        // guardare: immagine e trascrizione
        let look = run("archivio_guarda", json!({ "id": a }));
        assert_eq!(look.images.len(), 1);
        assert!(look.text.contains("Controllare il bilancio"), "{}", look.text);
        // rinominare ed esportare (senza sovrascrivere)
        assert!(run("archivio_rinomina", json!({ "id": a, "titolo": "Spesa" })).text.starts_with("Fatto"));
        assert_eq!(lib.lock().unwrap().get(&a).unwrap().meta.title, "Spesa");
        let e1 = run("archivio_esporta", json!({ "ids": [a, b], "formato": "pdf" }));
        assert!(e1.text.starts_with("Fatto: 1 file") && out.join("pagine-2.pdf").exists(), "{}", e1.text);
        run("archivio_esporta", json!({ "ids": [a, b], "formato": "pdf" }));
        assert!(out.join("pagine-2-2.pdf").exists());
        let e2 = run("archivio_esporta", json!({ "ids": [a], "formato": "png", "nome": "spesa" }));
        assert!(out.join("spesa.png").exists(), "{}", e2.text);
        // cestino: la conferma dell'utente, in un giro dopo
        let ask = run("archivio_cestino", json!({ "ids": [b] }));
        assert!(ask.text.starts_with("NON FATTO") && ask.text.contains("Riunione Tiani"), "{}", ask.text);
        let code: String = ask.text.split("\"conferma\": \"").nth(1).unwrap().chars().take(4).collect();
        assert!(run("archivio_cestino", json!({ "ids": [b], "conferma": code })).text.contains("aspetta"));
        crate::boardagent::new_turn();
        // il codice vale solo per quelle pagine
        assert!(run("archivio_cestino", json!({ "ids": [a], "conferma": code })).text.starts_with("NON FATTO"));
        let ask = run("archivio_cestino", json!({ "ids": [b] }));
        let code: String = ask.text.split("\"conferma\": \"").nth(1).unwrap().chars().take(4).collect();
        crate::boardagent::new_turn();
        let done = run("archivio_cestino", json!({ "ids": [b], "conferma": code }));
        assert!(done.text.starts_with("Fatto"), "{}", done.text);
        assert_eq!(lib.lock().unwrap().entries.len(), 1);
        let tr = run("archivio_cerca", json!({ "cestino": true }));
        assert!(tr.text.contains(&b) && tr.text.contains("Riunione Tiani"), "{}", tr.text);
        assert!(run("archivio_ripristina", json!({ "ids": [b] })).text.starts_with("Fatto"));
        assert_eq!(lib.lock().unwrap().entries.len(), 2);
        // id strani
        assert!(run("archivio_guarda", json!({ "id": "../../etc" })).text.starts_with("ERRORE"));
        assert!(run("archivio_cestino", json!({ "ids": ["20990101-000000"] })).text.starts_with("ERRORE"));
    }

    #[test]
    fn nomi_e_date() {
        assert_eq!(day("2026-10-21").as_deref(), Some("2026-10-21"));
        assert_eq!(day("21/10/2026").as_deref(), Some("2026-10-21"));
        assert!(day("ieri").is_none());
        assert_eq!(file_stem("Riunione: Tiani / bilancio"), "Riunione-Tiani-bilancio");
        assert_eq!(file_stem("???"), "pagina");
        assert!(valid_id("20261021-093000-2") && !valid_id("../x") && !valid_id(""));
        assert!(ids_of(&json!({ "ids": ["a/b"] })).is_err());
        assert_eq!(ids_of(&json!({ "id": "20261021-093000" })).unwrap(), vec!["20261021-093000"]);
        let it = Item { id: "x".into(), title: String::new(), created: "2026-10-21 09:30:00".into(), source: "pad".into(), strokes: 3, chart: false, on_board: false };
        assert_eq!(shown_title(&it), "21/10/2026 09:30");
    }
}
