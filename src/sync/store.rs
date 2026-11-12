//! I file di una parte visti come percorsi logici, uguali sulle due parti:
//!
//! | logico            | desktop                                | telefono            |
//! |-------------------|----------------------------------------|---------------------|
//! | `lib/<id>.json`   | `library/<id>.json`                    | `library/<id>.json` |
//! | `lib/<id>.md`     | `exports/<appunto>.md` (dalla pagina, `exports/.trascrizioni.json`) | `library/<id>.md` |
//! | `notes/<nome>.md` | `exports/<nome>.md`                    | `notes/<nome>.md`   |
//! | `sig/timbri.json` | `signatures/timbri.json`               | `timbri.json`       |
//! | `sig/firma.json`  | `signatures/firma.json`                | `signatures/firma.json` |
//! | `doc/<id>/<file>` | `documents/<id>/<file>`                | `documents/<id>/<file>` |
//!
//! Dei documenti viaggiano `doc.json`, `source.pdf` e il file di partenza;
//! le pagine disegnate e il testo estratto si rifanno da soli.
//!
//! Quello che si sovrascrive o si toglie non sparisce: le pagine vanno nel
//! cestino dell'archivio (`library/cestino`, si ripristinano dall'app), il
//! resto in `sync/cestino/`.

use super::{safe_name, valid_path, write_atomic, Entry, Manifest};
use std::collections::HashMap;
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};

/// Un «file» che non sta sul disco (le impostazioni AI, con le chiavi): chi
/// lo ha lo fa al momento e lo riceve. Non va mai nel cestino, così le
/// chiavi non finiscono in chiaro su disco.
pub trait Virtual: Send {
    /// Il contenuto e quando è cambiato (secondi Unix); None = non c'è.
    fn read(&mut self) -> Option<(Vec<u8>, i64)>;
    fn write(&mut self, data: &[u8]) -> Result<()>;
}

/// Un `Virtual` in memoria: il contenuto lo dà l'app prima di sincronizzare,
/// quello arrivato dall'altra parte lo riprende dopo (`received`).
pub struct MemVirtual {
    pub data: Option<(Vec<u8>, i64)>,
    pub received: std::sync::Arc<std::sync::Mutex<Option<Vec<u8>>>>,
}

impl Virtual for MemVirtual {
    fn read(&mut self) -> Option<(Vec<u8>, i64)> {
        self.data.clone()
    }

    fn write(&mut self, data: &[u8]) -> Result<()> {
        self.data = Some((data.to_vec(), super::now()));
        if let Ok(mut r) = self.received.lock() {
            *r = Some(data.to_vec());
        }
        Ok(())
    }
}

/// Una parte della sincronizzazione.
pub trait Store {
    fn manifest(&mut self) -> Result<Manifest>;
    fn read(&mut self, path: &str) -> Result<Vec<u8>>;
    /// Scrive (la versione di prima va nel cestino).
    fn write(&mut self, path: &str, data: &[u8]) -> Result<()>;
    /// Toglie (nel cestino).
    fn delete(&mut self, path: &str) -> Result<()>;
    /// Dà un nome nuovo, libero, a una pagina (e al suo appunto); ritorna il
    /// percorso nuovo.
    fn rename_free(&mut self, path: &str) -> Result<String>;
}

/// Dove sta l'appunto di una pagina.
enum PageNotes {
    /// Accanto alla pagina: `library/<id>.md` (telefono).
    Beside,
    /// Fra gli appunti, collegato dall'elenco delle trascrizioni (desktop).
    Linked(PathBuf),
}

/// I file sul disco.
pub struct FsStore {
    lib: PathBuf,
    docs: PathBuf,
    notes: PathBuf,
    stamps: PathBuf,
    signature: PathBuf,
    state: PathBuf,
    page_notes: PageNotes,
    /// Impronte già calcolate: file → (mtime, dimensione, impronta); restano
    /// in `sync/impronte.json`, così un PDF grande non si rilegge ogni volta.
    cache: HashMap<PathBuf, (i64, u64, String)>,
    cache_dirty: bool,
    /// Le impostazioni AI (`ai/servizi.json`).
    ai: Option<Box<dyn Virtual>>,
}

impl FsStore {
    /// La desktop, con i dati nella cartella `root`.
    pub fn desktop(root: &Path) -> FsStore {
        FsStore {
            lib: root.join("library"),
            docs: root.join("documents"),
            notes: root.join("exports"),
            stamps: root.join("signatures/timbri.json"),
            signature: root.join("signatures/firma.json"),
            state: root.join("sync"),
            page_notes: PageNotes::Linked(root.join("exports/.trascrizioni.json")),
            cache: HashMap::new(),
            cache_dirty: false,
            ai: None,
        }
        .with_cache()
    }

    /// Il telefono: `data` la cartella dei dati dell'app, `library` quella
    /// delle pagine (su iPhone sta in Documenti, visibile in File).
    pub fn phone(data: &Path, library: &Path) -> FsStore {
        FsStore {
            lib: library.to_path_buf(),
            docs: data.join("documents"),
            notes: data.join("notes"),
            stamps: data.join("timbri.json"),
            signature: data.join("signatures/firma.json"),
            state: data.join("sync"),
            page_notes: PageNotes::Beside,
            cache: HashMap::new(),
            cache_dirty: false,
            ai: None,
        }
        .with_cache()
    }

    fn with_cache(mut self) -> Self {
        let f = self.state.join("impronte.json");
        let m: HashMap<String, (i64, u64, String)> = std::fs::read_to_string(f).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        self.cache = m.into_iter().map(|(k, v)| (PathBuf::from(k), v)).collect();
        self
    }

    /// Il cestino della sincronizzazione tiene le cose per 30 giorni.
    fn prune_trash(&self) {
        let limit = std::time::SystemTime::now() - std::time::Duration::from_secs(30 * 24 * 3600);
        let Ok(rd) = std::fs::read_dir(self.state.join("cestino")) else { return };
        for e in rd.flatten() {
            if e.metadata().and_then(|m| m.modified()).is_ok_and(|t| t < limit) {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }

    fn save_cache(&mut self) {
        self.prune_trash();
        if !self.cache_dirty {
            return;
        }
        // solo i file che ci sono ancora
        self.cache.retain(|p, _| p.exists());
        let m: HashMap<String, &(i64, u64, String)> = self.cache.iter().map(|(k, v)| (k.to_string_lossy().into_owned(), v)).collect();
        if write_atomic(&self.state.join("impronte.json"), serde_json::to_string(&m).unwrap_or_default().as_bytes()).is_ok() {
            self.cache_dirty = false;
        }
    }

    /// Chi fa e riceve le impostazioni AI.
    pub fn set_ai(&mut self, ai: Box<dyn Virtual>) {
        self.ai = Some(ai);
    }

    pub fn state_dir(&self) -> &Path {
        &self.state
    }

    /// Collegamenti pagina → appunto della desktop (`"lib:<id>"` → nome).
    fn links(&self) -> serde_json::Map<String, serde_json::Value> {
        match &self.page_notes {
            PageNotes::Linked(f) => std::fs::read_to_string(f).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default(),
            PageNotes::Beside => Default::default(),
        }
    }

    fn save_links(&self, map: serde_json::Map<String, serde_json::Value>) -> Result<()> {
        if let PageNotes::Linked(f) = &self.page_notes {
            write_atomic(f, serde_json::Value::Object(map).to_string().as_bytes())?;
        }
        Ok(())
    }

    /// Il nome dell'appunto collegato alla pagina `id` (desktop).
    fn linked_note(&self, id: &str) -> Option<String> {
        let map = self.links();
        let v = map.get(&format!("lib:{id}"))?;
        let n = v.as_str().or_else(|| v["note"].as_str())?;
        safe_name(n).then(|| n.to_string())
    }

    /// Il file vero di un percorso logico.
    fn physical(&self, p: &str) -> Result<PathBuf> {
        if !valid_path(p) {
            return Err(Error::new(ErrorKind::InvalidInput, format!("percorso non ammesso: {p}")));
        }
        let (area, name) = p.split_once('/').unwrap_or_default();
        Ok(match area {
            "lib" if name.ends_with(".md") => match &self.page_notes {
                PageNotes::Beside => self.lib.join(name),
                PageNotes::Linked(_) => {
                    let id = name.trim_end_matches(".md");
                    self.notes.join(self.linked_note(id).unwrap_or_else(|| format!("{id}.md")))
                }
            },
            "lib" => self.lib.join(name),
            "notes" => self.notes.join(name),
            "doc" => self.docs.join(name),
            "ai" => return Err(Error::new(ErrorKind::InvalidInput, "le impostazioni AI non sono un file")),
            _ if name == "firma.json" => self.signature.clone(),
            _ => self.stamps.clone(),
        })
    }

    fn entry(&mut self, f: &Path) -> Option<Entry> {
        let m = std::fs::metadata(f).ok().filter(|m| m.is_file())?;
        let mtime = m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64;
        let size = m.len();
        if let Some((t, s, h)) = self.cache.get(f) {
            if *t == mtime && *s == size {
                return Some(Entry { hash: h.clone(), mtime, size });
            }
        }
        let hash = super::hash(&std::fs::read(f).ok()?);
        self.cache.insert(f.to_path_buf(), (mtime, size, hash.clone()));
        self.cache_dirty = true;
        Some(Entry { hash, mtime, size })
    }

    /// I file con quell'estensione in una cartella (niente nascosti).
    fn names(dir: &Path, ext: &str) -> Vec<String> {
        let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
        let mut v: Vec<String> = rd
            .flatten()
            .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
            .filter_map(|e| e.file_name().to_str().map(String::from))
            .filter(|n| n.ends_with(ext) && safe_name(n))
            .collect();
        v.sort();
        v
    }

    /// Mette da parte un file prima di sovrascriverlo o toglierlo.
    fn to_trash(&self, p: &str, f: &Path, keep_copy: bool) -> Result<()> {
        if !f.exists() {
            return Ok(());
        }
        // le pagine tolte nel cestino dell'archivio, come le toglie l'app
        let dst = if p.starts_with("lib/") && p.ends_with(".json") && !keep_copy {
            self.lib.join("cestino").join(f.file_name().unwrap_or_default())
        } else {
            let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
            self.state.join("cestino").join(format!("{}-{stamp}", p.replace('/', "-")))
        };
        if let Some(d) = dst.parent() {
            std::fs::create_dir_all(d)?;
        }
        if keep_copy {
            std::fs::copy(f, &dst).map(|_| ())
        } else {
            std::fs::rename(f, &dst)
        }
    }
}

impl Store for FsStore {
    fn manifest(&mut self) -> Result<Manifest> {
        let mut out = Manifest::new();
        let mut linked = std::collections::HashSet::new();
        for n in Self::names(&self.lib, ".json") {
            let f = self.lib.join(&n);
            if let Some(e) = self.entry(&f) {
                out.insert(format!("lib/{n}"), e);
            }
        }
        match &self.page_notes {
            PageNotes::Beside => {
                for n in Self::names(&self.lib, ".md") {
                    let f = self.lib.join(&n);
                    if let Some(e) = self.entry(&f) {
                        out.insert(format!("lib/{n}"), e);
                    }
                }
            }
            PageNotes::Linked(_) => {
                for (k, v) in self.links() {
                    let (Some(id), Some(note)) = (k.strip_prefix("lib:"), v.as_str().or_else(|| v["note"].as_str())) else { continue };
                    let p = format!("lib/{id}.md");
                    if !valid_path(&p) || !safe_name(note) {
                        continue;
                    }
                    let f = self.notes.join(note);
                    if let Some(e) = self.entry(&f) {
                        out.insert(p, e);
                        linked.insert(note.to_string());
                    }
                }
            }
        }
        for n in Self::names(&self.notes, ".md") {
            if linked.contains(&n) {
                continue;
            }
            let f = self.notes.join(&n);
            if let Some(e) = self.entry(&f) {
                out.insert(format!("notes/{n}"), e);
            }
        }
        let st = self.stamps.clone();
        if let Some(e) = self.entry(&st) {
            out.insert("sig/timbri.json".into(), e);
        }
        let sg = self.signature.clone();
        if let Some(e) = self.entry(&sg) {
            out.insert("sig/firma.json".into(), e);
        }
        if let Some((data, mtime)) = self.ai.as_mut().and_then(|a| a.read()) {
            out.insert(format!("ai/{}", super::AI_FILE), Entry { hash: super::hash(&data), mtime, size: data.len() as u64 });
        }
        // i documenti: le cartelle con un doc.json, i file che ci sono dentro
        let mut ids: Vec<String> = std::fs::read_dir(&self.docs)
            .map(|rd| rd.flatten().filter(|e| e.path().join("doc.json").is_file()).filter_map(|e| e.file_name().to_str().map(String::from)).collect())
            .unwrap_or_default();
        ids.sort();
        for id in ids {
            let dir = self.docs.join(&id);
            for n in Self::names(&dir, "") {
                let p = format!("doc/{id}/{n}");
                if !valid_path(&p) {
                    continue;
                }
                if let Some(e) = self.entry(&dir.join(&n)) {
                    out.insert(p, e);
                }
            }
        }
        self.save_cache();
        Ok(out)
    }

    fn read(&mut self, path: &str) -> Result<Vec<u8>> {
        if path.starts_with("ai/") && valid_path(path) {
            return self.ai.as_mut().and_then(|a| a.read()).map(|(d, _)| d).ok_or_else(|| Error::new(ErrorKind::NotFound, "niente impostazioni AI"));
        }
        std::fs::read(self.physical(path)?)
    }

    fn write(&mut self, path: &str, data: &[u8]) -> Result<()> {
        if data.len() > super::MAX_FILE {
            return Err(Error::new(ErrorKind::InvalidData, "file troppo grande"));
        }
        if path.starts_with("ai/") && valid_path(path) {
            return match self.ai.as_mut() {
                Some(a) => a.write(data),
                None => Ok(()),
            };
        }
        let f = self.physical(path)?;
        self.to_trash(path, &f, true)?;
        write_atomic(&f, data)?;
        // pagine nuove: quelle già disegnate e il testo estratto non valgono più
        if path.starts_with("doc/") && path.ends_with("/source.pdf") {
            if let Some(d) = f.parent() {
                let _ = std::fs::remove_dir_all(d.join("pages"));
                let _ = std::fs::remove_file(d.join("text.txt"));
            }
        }
        // un appunto nuovo di una pagina, sulla desktop: lo si collega alla pagina
        if let (PageNotes::Linked(_), Some(id)) = (&self.page_notes, path.strip_prefix("lib/").and_then(|n| n.strip_suffix(".md"))) {
            if self.linked_note(id).is_none() {
                let mut map = self.links();
                let name = f.file_name().unwrap_or_default().to_string_lossy().into_owned();
                map.insert(format!("lib:{id}"), serde_json::Value::String(name));
                self.save_links(map)?;
            }
        }
        Ok(())
    }

    fn delete(&mut self, path: &str) -> Result<()> {
        // le impostazioni AI non si tolgono (al più si svuotano dall'app)
        if path.starts_with("ai/") {
            return Ok(());
        }
        let f = self.physical(path)?;
        // tolto doc.json, il documento intero va nel cestino dei documenti
        if let Some(id) = path.strip_prefix("doc/").and_then(|r| r.strip_suffix("/doc.json")) {
            if self.docs.join(id).exists() {
                crate::docstore::trash(&self.docs, id)?;
            }
            return Ok(());
        }
        self.to_trash(path, &f, false)?;
        if let (PageNotes::Linked(_), Some(id)) = (&self.page_notes, path.strip_prefix("lib/").and_then(|n| n.strip_suffix(".md"))) {
            let mut map = self.links();
            if map.remove(&format!("lib:{id}")).is_some() {
                self.save_links(map)?;
            }
        }
        Ok(())
    }

    fn rename_free(&mut self, path: &str) -> Result<String> {
        let id = path
            .strip_prefix("lib/")
            .and_then(|n| n.strip_suffix(".json"))
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "solo le pagine cambiano nome"))?;
        let taken = |c: &str| {
            self.lib.join(format!("{c}.json")).exists() || self.lib.join("cestino").join(format!("{c}.json")).exists()
        };
        let mut n = 2;
        let new_id = loop {
            let c = format!("{id}-{n}");
            if !taken(&c) {
                break c;
            }
            n += 1;
        };
        std::fs::rename(self.lib.join(format!("{id}.json")), self.lib.join(format!("{new_id}.json")))?;
        match &self.page_notes {
            PageNotes::Beside => {
                let md = self.lib.join(format!("{id}.md"));
                if md.exists() {
                    std::fs::rename(&md, self.lib.join(format!("{new_id}.md")))?;
                }
            }
            PageNotes::Linked(_) => {
                let mut map = self.links();
                if let Some(v) = map.remove(&format!("lib:{id}")) {
                    map.insert(format!("lib:{new_id}"), v);
                    self.save_links(map)?;
                }
            }
        }
        Ok(format!("lib/{new_id}.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("openslate-sync-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn desktop_appunti_delle_pagine_e_liberi() {
        let root = tmp("desk");
        std::fs::create_dir_all(root.join("library")).unwrap();
        std::fs::create_dir_all(root.join("exports")).unwrap();
        std::fs::write(root.join("library/p1.json"), "{}").unwrap();
        std::fs::write(root.join("exports/appunti-1.md"), "# pagina").unwrap();
        std::fs::write(root.join("exports/libero.md"), "# libero").unwrap();
        std::fs::write(root.join("exports/.trascrizioni.json"), r#"{"lib:p1":{"note":"appunti-1.md","sig":"5"},"lavagna":"x.md"}"#).unwrap();
        let mut s = FsStore::desktop(&root);
        let m = s.manifest().unwrap();
        let keys: Vec<&str> = m.keys().map(String::as_str).collect();
        assert_eq!(keys, ["lib/p1.json", "lib/p1.md", "notes/libero.md"]);
        assert_eq!(s.read("lib/p1.md").unwrap(), b"# pagina");
        // un appunto che arriva per una pagina senza appunto: nuovo file, collegato
        s.write("lib/p2.md", b"# due").unwrap();
        assert_eq!(std::fs::read(root.join("exports/p2.md")).unwrap(), b"# due");
        assert!(s.manifest().unwrap().contains_key("lib/p2.md"));
        // sovrascrivere mette da parte la versione di prima
        s.write("lib/p1.md", b"# nuovo").unwrap();
        assert_eq!(std::fs::read(root.join("exports/appunti-1.md")).unwrap(), b"# nuovo");
        assert_eq!(std::fs::read_dir(root.join("sync/cestino")).unwrap().count(), 1);
        // togliere una pagina: nel cestino dell'archivio
        s.delete("lib/p1.json").unwrap();
        assert!(root.join("library/cestino/p1.json").exists());
        assert!(s.read("../x").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn telefono_e_nomi_liberi() {
        let root = tmp("tel");
        let lib = root.join("library");
        std::fs::create_dir_all(&lib).unwrap();
        std::fs::write(lib.join("p1.json"), "{}").unwrap();
        std::fs::write(lib.join("p1.md"), "nota").unwrap();
        std::fs::write(lib.join(".p9.json.tmp"), "x").unwrap();
        let mut s = FsStore::phone(&root, &lib);
        s.write("sig/timbri.json", b"[]").unwrap();
        let m = s.manifest().unwrap();
        let keys: Vec<&str> = m.keys().map(String::as_str).collect();
        assert_eq!(keys, ["lib/p1.json", "lib/p1.md", "sig/timbri.json"]);
        assert_eq!(s.rename_free("lib/p1.json").unwrap(), "lib/p1-2.json");
        assert!(lib.join("p1-2.md").exists() && !lib.join("p1.json").exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
