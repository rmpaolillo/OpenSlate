//! Typst dentro l'app (https://typst.app, Apache-2.0): formule (`mathtex`) e
//! grafici (`charts`).
//!
//! Un «mondo» Typst tutto in memoria: i font di Typst (New Computer Modern,
//! lo stile LaTeX), le definizioni di MiTeX e i pacchetti Typst inclusi
//! nell'app (`src/typst-pkg/`: Lilaq per i grafici e le sue dipendenze), così
//! si compila senza rete e senza file esterni. Un `#import "@preview/…"` di un
//! pacchetto incluso funziona; gli altri no.

use anyhow::{anyhow, bail, Result};
use std::sync::OnceLock;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};

#[path = "typstfiles.rs"]
mod files;

struct Shared {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
}

fn shared() -> &'static Shared {
    static S: OnceLock<Shared> = OnceLock::new();
    S.get_or_init(|| {
        let fonts: Vec<Font> = typst_assets::fonts().flat_map(|d| Font::iter(Bytes::new(d))).collect();
        Shared { library: LazyHash::new(Library::default()), book: LazyHash::new(FontBook::from_fonts(&fonts)), fonts }
    })
}

/// Il file incluso che corrisponde a `id`: i pacchetti `@preview/nome:versione`
/// e, nel progetto, `/mitex/…` (le definizioni di MiTeX).
fn embedded(id: FileId) -> Option<&'static [u8]> {
    let rp = id.get();
    let path = rp.vpath().get_without_slash();
    let key = match rp.root() {
        VirtualRoot::Package(spec) if spec.namespace == "preview" => format!("{}/{}/{}", spec.name, spec.version, path),
        VirtualRoot::Project => format!("mitex-specs/{}", path.strip_prefix("mitex/")?),
        _ => return None,
    };
    files::FILES.iter().find(|(k, _)| *k == key).map(|(_, b)| *b)
}

struct MemWorld {
    main: Source,
}

impl World for MemWorld {
    fn library(&self) -> &LazyHash<Library> {
        &shared().library
    }
    fn book(&self) -> &LazyHash<FontBook> {
        &shared().book
    }
    fn main(&self) -> FileId {
        self.main.id()
    }
    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main.id() {
            return Ok(self.main.clone());
        }
        let bytes = embedded(id).ok_or(FileError::AccessDenied)?;
        let text = std::str::from_utf8(bytes).map_err(|_| FileError::InvalidUtf8)?;
        Ok(Source::new(id, text.to_string()))
    }
    fn file(&self, id: FileId) -> FileResult<Bytes> {
        embedded(id).map(Bytes::new).ok_or(FileError::AccessDenied)
    }
    fn font(&self, index: usize) -> Option<Font> {
        shared().fonts.get(index).cloned()
    }
    fn today(&self, _: Option<typst::foundations::Duration>) -> Option<Datetime> {
        None
    }
}

/// Compila il sorgente Typst `src` (una pagina): ritorna il documento.
pub fn compile(src: String) -> Result<typst_layout::PagedDocument> {
    let main = RootedPath::new(VirtualRoot::Project, VirtualPath::new("/main.typ").map_err(|e| anyhow!("{e:?}"))?).intern();
    let world = MemWorld { main: Source::new(main, src) };
    let doc = typst::compile::<typst_layout::PagedDocument>(&world)
        .output
        .map_err(|errs| anyhow!("Typst: {}", errs.iter().map(|e| e.message.to_string()).collect::<Vec<_>>().join("; ")))?;
    if doc.pages().is_empty() {
        bail!("Typst: nessuna pagina");
    }
    Ok(doc)
}

/// La prima pagina in PNG a `ppi` pixel per pollice (fondo trasparente se la
/// pagina non ne ha uno).
pub fn png(doc: &typst_layout::PagedDocument, ppi: f32) -> Result<Vec<u8>> {
    let page = &doc.pages()[0];
    let pixmap = typst_render::render(page, &typst_render::RenderOptions { pixel_per_pt: typst::utils::Scalar::new(ppi as f64 / 72.0), ..Default::default() });
    pixmap.encode_png().map_err(|e| anyhow!("PNG: {e}"))
}

/// La prima pagina in SVG.
pub fn svg(doc: &typst_layout::PagedDocument) -> String {
    typst_svg::svg(&doc.pages()[0], &Default::default())
}
