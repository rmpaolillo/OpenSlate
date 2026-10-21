//! Pagine nitide dei documenti, disegnate dal motore del programma «Craft»
//! che corrisponde al documento: WordCraft per i Word, PdfCraft per i PDF
//! (https://github.com/storytold, MIT OR Apache-2.0, © ArtCraft Team), senza
//! le loro interfacce. Si disegna solo la parte che si vede, alla risoluzione
//! dello schermo: il testo resta nitido a ogni ingrandimento, come in un
//! programma di videoscrittura, e non sembra un'immagine.
//!
//! Il lavoro gira in un thread per documento; le richieste arrivate mentre si
//! disegna si saltano tutte tranne l'ultima (zoom e scorrimento veloci).

use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

/// Una parte di pagina da disegnare.
#[derive(Clone, Debug)]
pub struct Request {
    pub key: String,
    pub page: usize,
    /// Parte della pagina (frazioni 0..1 della pagina vista: x, y, larghezza, altezza).
    pub region: [f32; 4],
    /// Pixel dell'immagine (larghezza, altezza).
    pub px: [u32; 2],
    /// Misure della pagina vista, in punti (serve ai PDF).
    pub page_pt: (f32, f32),
}

/// Una parte di pagina disegnata (RGBA premoltiplicato).
pub struct Done {
    pub key: String,
    pub page: usize,
    pub region: [f32; 4],
    pub size: [usize; 2],
    pub rgba: Vec<u8>,
}

enum Engine {
    Word(Box<wordcraft_engine::Session>),
    Pdf(Box<pdfcraft_render::PageRenderer>),
}

pub struct Renderer {
    tx: Sender<Request>,
    rx: Receiver<Result<Done, String>>,
}

impl Renderer {
    /// Il disegnatore del documento nella cartella `dir` (`source.pdf`, e il
    /// Word originale se c'è); `wake` sveglia l'interfaccia a lavoro fatto.
    pub fn start(dir: PathBuf, original: Option<String>, wake: impl Fn() + Send + 'static) -> Self {
        let (tx, req_rx) = channel::<Request>();
        let (done_tx, rx) = channel();
        std::thread::spawn(move || {
            let mut engine: Option<Engine> = None;
            while let Ok(mut req) = req_rx.recv() {
                // solo l'ultima richiesta conta
                while let Ok(newer) = req_rx.try_recv() {
                    req = newer;
                }
                if engine.is_none() {
                    engine = Some(build(&dir, original.as_deref()));
                }
                let r = match engine.as_mut() {
                    Some(e) => render(e, &req).map_err(|e| format!("{e:#}")),
                    None => Err("nessun motore".into()),
                };
                if done_tx.send(r).is_err() {
                    return; // documento chiuso
                }
                wake();
            }
        });
        Self { tx, rx }
    }

    pub fn request(&self, req: Request) {
        let _ = self.tx.send(req);
    }

    pub fn try_recv(&self) -> Option<Result<Done, String>> {
        self.rx.try_recv().ok()
    }
}

/// WordCraft per un Word aperto bene, altrimenti PdfCraft sul PDF delle pagine.
fn build(dir: &std::path::Path, original: Option<&str>) -> Engine {
    // solo Word e testi: un SVG o una foto li ha già impaginati nel PDF il loro
    // motore (VectorCraft, PhotoCraft); WordCraft leggerebbe l'SVG come testo
    if let Some(name) = original.filter(|n| crate::wordsign::is_word(std::path::Path::new(n))) {
        match crate::wordsign::open(&dir.join(name)) {
            Ok(mut s) => {
                // niente sottolineature rosse del controllo ortografico: si firma, non si scrive
                s.view.proofing = false;
                return Engine::Word(Box::new(s));
            }
            Err(e) => tracing::warn!("pagine nitide del Word: {e:#}; uso il PDF"),
        }
    }
    let bytes = std::fs::read(dir.join("source.pdf")).unwrap_or_default();
    Engine::Pdf(Box::new(pdfcraft_render::PageRenderer::new(Arc::new(bytes), pdfcraft_render::RenderConfig::default())))
}

fn render(engine: &mut Engine, req: &Request) -> Result<Done> {
    let [rx, ry, rw, rh] = req.region;
    let (size, rgba) = match engine {
        Engine::Word(s) => {
            let layout = s.layout();
            let page = layout.pages.get(req.page).ok_or_else(|| anyhow!("pagina {} assente", req.page + 1))?;
            let (x, y, w, h) = (rx * page.w, ry * page.h, rw * page.w, rh * page.h);
            let scale = req.px[0] as f32 / w.max(1e-3);
            // come sullo schermo del Mac (testo appena più pieno), come in WordCraft
            let opts = wordcraft_engine::render::RenderOptions { text_darkening: cfg!(target_os = "macos"), ..Default::default() };
            let r = wordcraft_engine::render::render_area(&s.doc, page, x, y, w, h, scale, &opts);
            ([r.width as usize, r.height as usize], r.pixels)
        }
        Engine::Pdf(p) => {
            let (pw, _) = req.page_pt;
            let scale = req.px[0] as f32 / (rw * pw).max(1e-3);
            let (pw_px, ph_px) = (pw * scale, req.page_pt.1 * scale);
            let tile = pdfcraft_render::Tile {
                x: (rx * pw_px).round() as u32,
                y: (ry * ph_px).round() as u32,
                w: req.px[0].max(1),
                h: req.px[1].max(1),
            };
            let r = p.render(pdfcraft_render::RenderRequest {
                page: req.page,
                kind: pdfcraft_render::RequestKind::Pixels,
                tile: Some(tile),
                scale,
                tag: 0,
                preview: false,
            });
            if let Some(e) = r.error {
                return Err(anyhow!("PdfCraft: {e}"));
            }
            ([r.width as usize, r.height as usize], r.rgba.to_vec())
        }
    };
    if size[0] * size[1] * 4 != rgba.len() || size[0] == 0 {
        return Err(anyhow!("immagine della pagina di misure sbagliate"));
    }
    Ok(Done { key: req.key.clone(), page: req.page, region: req.region, size, rgba })
}
