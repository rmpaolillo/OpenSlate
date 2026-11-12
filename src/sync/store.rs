//! I file di una parte visti come percorsi logici, uguali sulle due parti:
//!
//! | logico            | desktop                                | telefono            |
//! |-------------------|----------------------------------------|---------------------|
//! | `lib/<id>.json`   | `library/<id>.json`                    | `library/<id>.json` |
//! | `lib/<id>.md`     | `exports/<appunto>.md` (dalla pagina, `exports/.trascrizioni.json`) | `library/<id>.md` |
//! | `notes/<nome>.md` | `exports/<nome>.md`                    | `notes/<nome>.md`   |
//! | `sig/timbri.json` | `signatures/timbri.json`               | `timbri.json`       |
//!
//! Quello che si sovrascrive o si toglie non sparisce: le pagine vanno nel
//! cestino dell'archivio (`library/cestino`, si ripristinano dall'app), il
//! resto in `sync/cestino/`.

use super::{safe_name, valid_path, write_atomic, Entry, Manifest};
use std::collections::HashMap;
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};

/// Una parte della sincronizzazione.
pub trait Store {
    fn manifest(&mut self) -> Result<Manifest>;
    fn read(&self, path: &str) -> Result<Vec<u8>>;
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
    notes: PathBuf,
    stamps: PathBuf,
    state: PathBuf,
    page_notes: PageNotes,
    /// Impronte già calcolate: file → (mtime, dimensione, impronta).
    cache: HashMap<PathBuf, (i64, u64, String)>,
}

impl FsStore {
    /// La desktop, con i dati nella cartella `root`.
    pub fn desktop(root: &Path) -> FsStore {
        FsStore {
            lib: root.join("library"),
            notes: root.join("exports"),
            stamps: root.join("signatures/timbri.json"),
            state: root.join("sync"),
            page_notes: PageNotes::Linked(root.join("exports/.trascrizioni.json")),
            cache: HashMap::new(),
        }
    }

    /// Il telefono: `data` la cartella dei dati dell'app, `library` quella
    /// delle pagine (su iPhone sta in Documenti, visibile in File).
    pub fn phone(data: &Path, library: &Path) -> FsStore {
        FsStore {
            lib: library.to_path_buf(),
            notes: data.join("notes"),
            stamps: data.join("timbri.json"),
            state: data.join("sync"),
            page_notes: PageNotes::Beside,
            cache: HashMap::new(),
        }
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
        Ok(out)
    }

    fn read(&self, path: &str) -> Result<Vec<u8>> {
        std::fs::read(self.physical(path)?)
    }

    fn write(&mut self, path: &str, data: &[u8]) -> Result<()> {
        if data.len() > super::MAX_FILE {
            return Err(Error::new(ErrorKind::InvalidData, "file troppo grande"));
        }
        let f = self.physical(path)?;
        self.to_trash(path, &f, true)?;
        write_atomic(&f, data)?;
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
        let f = self.physical(path)?;
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
