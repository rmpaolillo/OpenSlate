//! PDF dei documenti importati: misure delle pagine e salvataggio con i
//! tratti (firma, appunti, evidenziature) aggiunti SOPRA l'originale.
//!
//! L'originale non viene ridisegnato: a ogni pagina si aggiunge un flusso di
//! contenuto con i tratti in grafica vettoriale. Testo, immagini e qualità del
//! documento restano identici (il testo resta selezionabile).

use crate::stroke::Rgb;
use anyhow::{anyhow, bail, Context, Result};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};
use std::fmt::Write as _;
use std::path::Path;

/// Una pagina del PDF: riquadro visibile (CropBox, o MediaBox) e rotazione.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PdfPage {
    /// x0, y0, x1, y1 in punti (1/72 di pollice).
    pub bbox: [f32; 4],
    /// 0, 90, 180, 270 (senso orario, come /Rotate).
    pub rotate: i32,
}

impl PdfPage {
    /// Larghezza e altezza della pagina come la si vede (rotazione applicata).
    pub fn display_size(&self) -> (f32, f32) {
        let w = (self.bbox[2] - self.bbox[0]).abs();
        let h = (self.bbox[3] - self.bbox[1]).abs();
        if self.rotate.rem_euclid(180) == 90 {
            (h, w)
        } else {
            (w, h)
        }
    }

    /// Punto della pagina vista (u, v in 0..1, v verso il basso) → coordinate PDF.
    pub fn to_user(&self, u: f32, v: f32) -> (f32, f32) {
        let [x0, y0, x1, y1] = self.bbox;
        let (w, h) = (x1 - x0, y1 - y0);
        match self.rotate.rem_euclid(360) {
            90 => (x0 + v * w, y0 + u * h),
            180 => (x1 - u * w, y0 + v * h),
            270 => (x1 - v * w, y1 - u * h),
            _ => (x0 + u * w, y1 - v * h),
        }
    }
}

/// Un tratto da aggiungere: coordinate 0..1 sulla pagina vista, spessore in
/// frazione dell'altezza della pagina vista.
#[derive(Clone, Debug)]
pub struct OverlayStroke {
    pub color: Rgb,
    pub width_frac: f32,
    pub pts: Vec<(f32, f32)>,
    pub highlight: bool,
}

fn num(doc: &Document, o: &Object) -> Option<f32> {
    match o {
        Object::Reference(id) => doc.get_object(*id).ok().and_then(|o| num(doc, o)),
        o => o.as_float().ok(),
    }
}

fn rect(doc: &Document, o: &Object) -> Option<[f32; 4]> {
    let o = match o {
        Object::Reference(id) => doc.get_object(*id).ok()?,
        o => o,
    };
    let a = o.as_array().ok()?;
    if a.len() != 4 {
        return None;
    }
    let v: Vec<f32> = a.iter().filter_map(|x| num(doc, x)).collect();
    (v.len() == 4).then(|| [v[0].min(v[2]), v[1].min(v[3]), v[0].max(v[2]), v[1].max(v[3])])
}

/// Valore di una chiave della pagina, cercando anche nei genitori (MediaBox,
/// CropBox, Rotate e Resources si ereditano).
fn inherited(doc: &Document, page: ObjectId, key: &[u8]) -> Option<Object> {
    let mut id = page;
    for _ in 0..64 {
        let d = doc.get_dictionary(id).ok()?;
        if let Ok(v) = d.get(key) {
            return Some(v.clone());
        }
        id = d.get(b"Parent").ok()?.as_reference().ok()?;
    }
    None
}

fn page_of(doc: &Document, id: ObjectId) -> PdfPage {
    let media = inherited(doc, id, b"MediaBox")
        .and_then(|o| rect(doc, &o))
        .unwrap_or([0.0, 0.0, 595.276, 841.89]);
    let crop = inherited(doc, id, b"CropBox").and_then(|o| rect(doc, &o));
    // Il CropBox va limitato al MediaBox.
    let bbox = match crop {
        Some(c) => [c[0].max(media[0]), c[1].max(media[1]), c[2].min(media[2]), c[3].min(media[3])],
        None => media,
    };
    let rotate = inherited(doc, id, b"Rotate")
        .and_then(|o| num(doc, &o))
        .map(|r| (r as i32).rem_euclid(360) / 90 * 90)
        .unwrap_or(0);
    PdfPage { bbox, rotate }
}

fn load(path: &Path) -> Result<Document> {
    let doc = Document::load(path).with_context(|| format!("PDF non leggibile: {}", path.display()))?;
    if doc.is_encrypted() {
        bail!("il PDF è protetto da password: toglila (es. stampando in PDF da Anteprima) e riprova");
    }
    Ok(doc)
}

/// Misure di tutte le pagine.
/// Un PDF di una pagina con solo un'immagine RGB (`w`×`h` pixel), grande
/// `pw`×`ph` punti: le foto aperte come documento.
pub fn image_pdf(out: &Path, w: u32, h: u32, rgb: &[u8], pw: f64, ph: f64) -> Result<()> {
    use lopdf::dictionary;
    if rgb.len() < (w * h * 3) as usize {
        bail!("immagine incompleta");
    }
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let mut img = Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Image", "Width" => w as i64, "Height" => h as i64,
            "ColorSpace" => "DeviceRGB", "BitsPerComponent" => 8,
        },
        rgb[..(w * h * 3) as usize].to_vec(),
    );
    let _ = img.compress();
    let img_id = doc.add_object(img);
    let content = format!("q {pw:.3} 0 0 {ph:.3} 0 0 cm /Im0 Do Q");
    let content_id = doc.add_object(Stream::new(Dictionary::new(), content.into_bytes()));
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages_id,
        "MediaBox" => vec![0.into(), 0.into(), Object::Real(pw as f32), Object::Real(ph as f32)],
        "Contents" => content_id,
        "Resources" => dictionary! { "XObject" => dictionary! { "Im0" => img_id } },
    });
    doc.objects.insert(pages_id, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page_id.into()], "Count" => 1 }));
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog);
    doc.save(out).with_context(|| format!("scrittura di {}", out.display()))?;
    Ok(())
}

pub fn pages(path: &Path) -> Result<Vec<PdfPage>> {
    let doc = load(path)?;
    Ok(doc.get_pages().values().map(|&id| page_of(&doc, id)).collect())
}

/// Misure delle pagine di un PDF in memoria.
pub fn pages_from_bytes(bytes: &[u8]) -> Result<Vec<PdfPage>> {
    let doc = Document::load_mem(bytes).context("PDF non leggibile")?;
    Ok(doc.get_pages().values().map(|&id| page_of(&doc, id)).collect())
}

/// Operatori PDF che disegnano i tratti di una pagina.
fn overlay_content(page: &PdfPage, strokes: &[OverlayStroke]) -> String {
    let (_, dh) = page.display_size();
    let mut c = String::from("Q\nq\n1 J 1 j\n");
    let draw = |c: &mut String, s: &OverlayStroke| {
        let [r, g, b] = s.color;
        let _ = writeln!(
            c,
            "{:.4} {:.4} {:.4} RG {:.4} {:.4} {:.4} rg {:.3} w",
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            (s.width_frac * dh).max(0.2)
        );
        for (i, &(u, v)) in s.pts.iter().enumerate() {
            let (x, y) = page.to_user(u, v);
            let _ = writeln!(c, "{x:.2} {y:.2} {}", if i == 0 { "m" } else { "l" });
        }
        if s.pts.len() == 1 {
            let (x, y) = page.to_user(s.pts[0].0, s.pts[0].1);
            let _ = writeln!(c, "{x:.2} {y:.2} l");
        }
        c.push_str("S\n");
    };
    // Evidenziature in modalità "moltiplica": il testo sotto resta nero.
    if strokes.iter().any(|s| s.highlight) {
        c.push_str("q /WacomHL gs\n");
        for s in strokes.iter().filter(|s| s.highlight && !s.pts.is_empty()) {
            draw(&mut c, s);
        }
        c.push_str("Q\n");
    }
    for s in strokes.iter().filter(|s| !s.highlight && !s.pts.is_empty()) {
        draw(&mut c, s);
    }
    c.push_str("Q\n");
    c
}

/// Copia `src` in `dst` aggiungendo i tratti; `per_page[i]` sono quelli della
/// pagina i (le pagine senza tratti restano intatte).
pub fn write_overlay(src: &Path, dst: &Path, per_page: &[Vec<OverlayStroke>]) -> Result<()> {
    let mut doc = load(src)?;
    let page_ids: Vec<ObjectId> = doc.get_pages().values().copied().collect();

    // Stato grafico per le evidenziature (condiviso da tutte le pagine).
    let mut gs = Dictionary::new();
    gs.set("Type", Object::Name(b"ExtGState".to_vec()));
    gs.set("BM", Object::Name(b"Multiply".to_vec()));
    let gs_id = doc.add_object(Object::Dictionary(gs));
    // "q" all'inizio: isola lo stato grafico dell'originale dal nostro.
    let q_id = doc.add_object(Object::Stream(Stream::new(Dictionary::new(), b"q\n".to_vec())));

    for (i, strokes) in per_page.iter().enumerate() {
        let Some(&pid) = page_ids.get(i) else { break };
        if strokes.iter().all(|s| s.pts.is_empty()) {
            continue;
        }
        let page = page_of(&doc, pid);
        let content = overlay_content(&page, strokes);
        let ov_id = doc.add_object(Object::Stream(Stream::new(Dictionary::new(), content.into_bytes())));

        // Risorse (anche ereditate) + il nostro stato grafico.
        let mut res = match inherited(&doc, pid, b"Resources") {
            Some(Object::Reference(id)) => doc.get_dictionary(id).cloned().unwrap_or_default(),
            Some(Object::Dictionary(d)) => d,
            _ => Dictionary::new(),
        };
        let mut ext = match res.get(b"ExtGState") {
            Ok(Object::Reference(id)) => doc.get_dictionary(*id).cloned().unwrap_or_default(),
            Ok(Object::Dictionary(d)) => d.clone(),
            _ => Dictionary::new(),
        };
        ext.set("WacomHL", Object::Reference(gs_id));
        res.set("ExtGState", Object::Dictionary(ext));

        // Contenuti: [q, originale…, Q + tratti]
        let old = doc.get_dictionary(pid)?.get(b"Contents").ok().cloned();
        let mut arr = vec![Object::Reference(q_id)];
        match old {
            Some(Object::Array(a)) => arr.extend(a),
            Some(Object::Reference(id)) => match doc.get_object(id) {
                Ok(Object::Array(a)) => arr.extend(a.clone()),
                _ => arr.push(Object::Reference(id)),
            },
            _ => {}
        }
        arr.push(Object::Reference(ov_id));

        let d = doc.get_dictionary_mut(pid)?;
        d.set("Resources", Object::Dictionary(res));
        d.set("Contents", Object::Array(arr));
    }
    doc.save(dst)
        .map_err(|e| anyhow!("salvataggio di {}: {e}", dst.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotazioni() {
        let p = PdfPage { bbox: [0.0, 0.0, 600.0, 800.0], rotate: 0 };
        assert_eq!(p.display_size(), (600.0, 800.0));
        assert_eq!(p.to_user(0.0, 0.0), (0.0, 800.0)); // in alto a sinistra
        let r = PdfPage { rotate: 90, ..p };
        assert_eq!(r.display_size(), (800.0, 600.0));
        // pagina ruotata di 90°: in alto a sinistra della vista = (x0, y0)
        assert_eq!(r.to_user(0.0, 0.0), (0.0, 0.0));
        assert_eq!(r.to_user(1.0, 0.0), (0.0, 800.0));
        let r = PdfPage { rotate: 270, ..p };
        assert_eq!(r.to_user(0.0, 0.0), (600.0, 800.0));
        let r = PdfPage { rotate: 180, ..p };
        assert_eq!(r.to_user(0.0, 0.0), (600.0, 0.0));
    }

    #[test]
    fn sovrapposizione_su_pdf_vero() {
        // PDF di due pagine fatto dal nostro esportatore, poi "firmato".
        let page = crate::scene::Page {
            w: 210.0,
            h: 297.0,
            strokes: vec![crate::scene::PageStroke {
                color: [0, 0, 0],
                width: 2.0,
                pts: vec![(0.1, 0.1), (0.9, 0.1)],
                highlight: false,
            }],
            ..Default::default()
        };
        let dir = std::env::temp_dir().join(format!("wacom-pdf-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("a.pdf");
        std::fs::write(&src, crate::craftexport::pdf(&[&page, &page]).unwrap()).unwrap();
        let info = pages(&src).unwrap();
        assert_eq!(info.len(), 2);
        let firma = OverlayStroke {
            color: [20, 40, 160],
            width_frac: 0.003,
            pts: vec![(0.6, 0.9), (0.7, 0.85), (0.8, 0.92)],
            highlight: false,
        };
        let hl = OverlayStroke { color: [255, 235, 90], width_frac: 0.02, pts: vec![(0.1, 0.1), (0.9, 0.1)], highlight: true };
        let dst = dir.join("b.pdf");
        write_overlay(&src, &dst, &[vec![], vec![firma, hl]]).unwrap();
        let out = Document::load(&dst).unwrap();
        assert_eq!(out.get_pages().len(), 2);
        let p2 = *out.get_pages().get(&2).unwrap();
        let contents = out.get_dictionary(p2).unwrap().get(b"Contents").unwrap().as_array().unwrap().len();
        assert!(contents >= 3);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
