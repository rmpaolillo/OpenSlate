//! Documenti Word (.docx, .doc, .odt, .rtf) col motore di WordCraft
//! (https://github.com/storytold/wordcraft, MIT OR Apache-2.0, © ArtCraft Team),
//! senza la sua interfaccia: le pagine per la lavagna (un PDF fatto da WordCraft)
//! e il salvataggio firmato, che resta un .docx con la scrittura del pennino
//! dentro come inchiostro (Word, LibreOffice e WordCraft la mostrano).

use crate::stroke::Rgb;
use anyhow::{anyhow, bail, Result};
use serde_json::json;
use std::path::Path;
use wordcraft_engine::Session;

/// Estensioni aperte con WordCraft: Word, OpenDocument, RTF e anche i testi
/// (Markdown, .txt, .html), che WordCraft impagina come un documento.
pub fn is_word(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    matches!(ext.as_str(), "docx" | "doc" | "docm" | "dotx" | "dotm" | "dot" | "odt" | "rtf") || is_text(path)
}

/// Testi che WordCraft apre ma che non possono tenere l'inchiostro: firmati
/// diventano PDF (le pagine impaginate da WordCraft con i tratti sopra).
pub fn is_text(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    matches!(ext.as_str(), "md" | "markdown" | "txt" | "html" | "htm")
}

/// Apre un documento Word col motore di WordCraft.
/// Un Markdown porta con sé le formule LaTeX e le figure: diventano equazioni
/// e immagini di WordCraft (vedi `mdcraft`).
pub fn open(path: &Path) -> Result<Session> {
    let mut bytes = std::fs::read(path).map_err(|e| anyhow!("{}: {e}", path.display()))?;
    let markdown = crate::mdcraft::is_markdown(path);
    let mut pieces = Vec::new();
    if markdown {
        let base = path.parent().unwrap_or(Path::new("."));
        let (text, p) = crate::mdcraft::split(&String::from_utf8_lossy(&bytes), base);
        bytes = text.into_bytes();
        pieces = p;
    }
    let mut s = Session::new(wordcraft_engine::doc::Document::new());
    s.run(
        "file.open",
        &json!({ "path": path.to_string_lossy(), "data": wordcraft_engine::cmd::insert::base64_encode(&bytes) }),
    )
    .map_err(|e| anyhow!("WordCraft non apre il documento: {e}"))?;
    if markdown {
        crate::mdcraft::style(&mut s);
        crate::mdcraft::place(&mut s, &pieces);
    }
    Ok(s)
}

/// Il documento come PDF, impaginato da WordCraft (per le pagine della lavagna).
pub fn to_pdf(input: &Path, out: &Path) -> Result<()> {
    let mut s = open(input)?;
    s.run("file.exportPdf", &json!({ "path": out.to_string_lossy() })).map_err(|e| anyhow!("WordCraft, PDF: {e}"))?;
    if !out.exists() {
        bail!("WordCraft non ha scritto il PDF");
    }
    Ok(())
}

/// Un tratto da mettere nel documento: pagina, punti in 0..1 sulla pagina
/// (dall'angolo in alto a sinistra), spessore come frazione dell'altezza.
pub struct InkStroke {
    pub page: usize,
    pub pts: Vec<(f32, f32)>,
    pub color: Rgb,
    pub width_frac: f32,
    pub highlight: bool,
}

/// Copia `src` in `dst` (stesso formato Word) con i tratti come inchiostro.
pub fn save_with_ink(src: &Path, dst: &Path, strokes: &[InkStroke]) -> Result<()> {
    let mut s = open(src)?;
    add_ink(&mut s, strokes)?;
    s.run("file.save", &json!({ "path": dst.to_string_lossy() })).map_err(|e| anyhow!("WordCraft, salvataggio: {e}"))?;
    if !dst.exists() {
        bail!("WordCraft non ha scritto {}", dst.display());
    }
    Ok(())
}

/// Mette i tratti nel documento come inchiostro di Word.
///
/// Ogni tratto è un disegno agganciato a un paragrafo: si sposta con lui.
/// WordCraft lo aggancia al paragrafo sotto il punto dove il tratto comincia,
/// e così una firma finiva divisa fra più paragrafi, spesso vuoti (l'a capo in
/// fondo al documento). Word e LibreOffice impaginano un po' diversamente:
/// quando quei paragrafi cambiavano pagina, metà firma andava alla pagina
/// dopo. Qui i tratti vicini (una firma, una sigla) restano insieme,
/// agganciati all'ultimo paragrafo con del testo che comincia sopra di loro
/// (per esempio la riga dell'IBAN o «Firma:»): la firma segue il testo che
/// firma. Gli evidenziatori restano sul testo che segnano.
fn add_ink(s: &mut Session, strokes: &[InkStroke]) -> Result<()> {
    let layout = s.layout();
    // i tratti in punti della pagina
    let pts: Vec<Option<Vec<[f32; 2]>>> = strokes
        .iter()
        .map(|st| {
            let page = layout.pages.get(st.page)?;
            let p: Vec<[f32; 2]> = st.pts.iter().take(5000).map(|(u, v)| [u.clamp(0.0, 1.0) * page.w, v.clamp(0.0, 1.0) * page.h]).collect();
            (!p.is_empty()).then_some(p)
        })
        .collect();
    // firme: gruppi di tratti a penna vicini sulla stessa pagina → un paragrafo per gruppo
    let boxes: Vec<Option<[f32; 4]>> = pts.iter().map(|p| p.as_deref().map(bounds)).collect();
    let pens: Vec<usize> = (0..strokes.len()).filter(|&i| boxes[i].is_some() && !strokes[i].highlight).collect();
    let groups = clusters(&pens, |i| (strokes[i].page, boxes[i].unwrap_or_default()));
    let mut target: Vec<Option<doc::Path>> = vec![None; strokes.len()];
    for g in &groups {
        let page = strokes[g[0]].page;
        let top = g.iter().filter_map(|&i| boxes[i]).map(|b| b[1]).fold(f32::MAX, f32::min);
        let left = g.iter().filter_map(|&i| boxes[i]).map(|b| b[0]).fold(f32::MAX, f32::min);
        let anchor = text_paragraph_above(s, &layout, page, left, top);
        for &i in g {
            target[i] = anchor.clone();
        }
    }
    for (i, st) in strokes.iter().enumerate() {
        let Some(points) = &pts[i] else { continue };
        let color = format!("{:02X}{:02X}{:02X}", st.color[0], st.color[1], st.color[2]);
        let page_h = layout.pages.get(st.page).map(|p| p.h).unwrap_or(842.0);
        let width = (st.width_frac * page_h).clamp(0.25, 72.0);
        let r = s
            .run(
                "draw.stroke",
                &json!({ "page": st.page, "points": points, "tool": if st.highlight { "highlighter" } else { "pen" }, "color": color, "width": width }),
            )
            .map_err(|e| anyhow!("inchiostro sulla pagina {}: {e}", st.page + 1))?;
        if let Some(to) = &target[i] {
            let at: doc::Pos = serde_json::from_value(r["pos"].clone()).map_err(|e| anyhow!("inchiostro: posizione {e}"))?;
            let rect: Vec<f32> = r["rect"].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).map(|v| v as f32).collect()).unwrap_or_default();
            if &at.path != to && rect.len() == 4 {
                move_ink(s, &at, to, st.page, (rect[0], rect[1]))?;
            }
        }
    }
    Ok(())
}

use wordcraft_engine::doc;

/// Riquadro dei punti: x0, y0, x1, y1.
fn bounds(p: &[[f32; 2]]) -> [f32; 4] {
    p.iter().fold([f32::MAX, f32::MAX, f32::MIN, f32::MIN], |b, [x, y]| [b[0].min(*x), b[1].min(*y), b[2].max(*x), b[3].max(*y)])
}

/// Raggruppa gli elementi vicini (riquadri a meno di `GAP` punti, sulla stessa
/// pagina): i tratti di una firma o di una sigla.
fn clusters(items: &[usize], at: impl Fn(usize) -> (usize, [f32; 4])) -> Vec<Vec<usize>> {
    const GAP: f32 = 28.0;
    let mut parent: Vec<usize> = (0..items.len()).collect();
    fn root(p: &mut [usize], mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    for a in 0..items.len() {
        for b in a + 1..items.len() {
            let ((pa, ra), (pb, rb)) = (at(items[a]), at(items[b]));
            let near = pa == pb && ra[0] - GAP <= rb[2] && rb[0] - GAP <= ra[2] && ra[1] - GAP <= rb[3] && rb[1] - GAP <= ra[3];
            if near {
                let (x, y) = (root(&mut parent, a), root(&mut parent, b));
                parent[x] = y;
            }
        }
    }
    let mut out: Vec<Vec<usize>> = Vec::new();
    let mut idx: std::collections::HashMap<usize, usize> = Default::default();
    for i in 0..items.len() {
        let r = root(&mut parent, i);
        let k = *idx.entry(r).or_insert_with(|| {
            out.push(Vec::new());
            out.len() - 1
        });
        out[k].push(items[i]);
    }
    out
}

/// L'ultimo paragrafo del testo (non in tabella, con qualcosa scritto) che
/// comincia su `page` sopra `y`; se sopra non ce n'è, il primo della pagina.
fn text_paragraph_above(s: &Session, layout: &wordcraft_engine::layout::DocLayout, page: usize, _x: f32, y: f32) -> Option<doc::Path> {
    use wordcraft_engine::layout::Placed;
    let pg = layout.pages.get(page)?;
    let mut starts: Vec<(doc::Path, f32)> = pg
        .items
        .iter()
        .filter_map(|it| match it {
            Placed::Lines { story: doc::StoryRef::Body, path, l0: 0, y, .. } if path.depth() == 0 => Some((path.clone(), *y)),
            _ => None,
        })
        .filter(|(path, _)| {
            let pos = doc::Pos { story: doc::StoryRef::Body, path: path.clone(), off: 0 };
            s.doc.para_at(&pos).is_some_and(|p| p.plain_text().chars().any(|c| c != doc::para::OBJ && !c.is_whitespace()))
        })
        .collect();
    starts.sort_by(|a, b| a.1.total_cmp(&b.1));
    starts.iter().rev().find(|(_, top)| *top <= y + 0.5).or(starts.first()).map(|(p, _)| p.clone())
}

/// Sposta il disegno in `at` all'inizio del paragrafo `to`, lasciandolo dov'è
/// sulla pagina (angolo in alto a sinistra in `xy`, punti della pagina).
fn move_ink(s: &mut Session, at: &doc::Pos, to: &doc::Path, page: usize, (x, y): (f32, f32)) -> Result<()> {
    use doc::para::{Anchor, InlineObject, OBJ};
    let len = OBJ.len_utf8();
    let from = s.doc.para_mut(at.story, &at.path).map_err(|e| anyhow!("inchiostro: {e}"))?;
    let obj = from.object_at(at.off).cloned().ok_or_else(|| anyhow!("inchiostro: il tratto non c'è"))?;
    from.delete(at.off, at.off + len).map_err(|e| anyhow!("inchiostro: {e}"))?;
    from.touch();
    let dest = s.doc.para_mut(doc::StoryRef::Body, to).map_err(|e| anyhow!("inchiostro: {e}"))?;
    dest.insert_object(0, obj, &doc::CharProps::default()).map_err(|e| anyhow!("inchiostro: {e}"))?;
    dest.touch();
    let new = doc::Pos { story: doc::StoryRef::Body, path: to.clone(), off: 0 };
    s.touch();
    let origin = s.layout().object(&new, page).map(|h| h.origin).ok_or_else(|| anyhow!("inchiostro: il tratto spostato non è sulla pagina"))?;
    let para = s.doc.para_mut(doc::StoryRef::Body, to).map_err(|e| anyhow!("inchiostro: {e}"))?;
    if let Some(InlineObject::Shape { float, .. }) = para.object_at_mut(0) {
        float.h_rel = Anchor::Column;
        float.v_rel = Anchor::Paragraph;
        (float.h_align, float.v_align) = (None, None);
        float.x = x - origin.x;
        float.y = y - origin.y;
    }
    para.touch();
    s.touch();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Una firma sotto l'ultima riga, sopra gli a capo vuoti in fondo: tutti i
    /// tratti vanno sulla riga con il testo e restano dove sono stati scritti.
    #[test]
    fn firma_agganciata_al_testo() {
        let mut s = Session::new(wordcraft_engine::doc::Document::new());
        s.run("document.setText", &json!({"text": "Contratto.\nIBAN: IT00X\n\n\n\n\n"})).unwrap();
        let layout = s.layout();
        let (pw, ph) = (layout.pages[0].w, layout.pages[0].h);
        // tre tratti a 60–110 pt sotto l'inizio della pagina di testo + uno lontano
        let stroke = |x0: f32, y0: f32, x1: f32, y1: f32| InkStroke {
            page: 0,
            pts: vec![(x0 / pw, y0 / ph), (x1 / pw, y1 / ph)],
            color: [20, 20, 20],
            width_frac: 1.0 / ph,
            highlight: false,
        };
        let mut st = vec![stroke(150.0, 150.0, 200.0, 170.0), stroke(210.0, 140.0, 260.0, 175.0), stroke(270.0, 160.0, 320.0, 150.0)];
        st.push(stroke(400.0, 600.0, 450.0, 620.0));
        add_ink(&mut s, &st).unwrap();
        // dove stanno i disegni
        let mut found = Vec::new();
        let lay = s.layout();
        for (i, b) in s.doc.body.iter().enumerate() {
            if let Some(p) = b.as_para() {
                for off in p.object_offsets() {
                    let hit = lay.object(&doc::Pos { story: doc::StoryRef::Body, path: doc::Path::top(i), off }, 0).unwrap();
                    found.push((i, hit.rect.x, hit.rect.y));
                }
            }
        }
        assert_eq!(found.len(), 4, "{found:?}");
        let iban = s.doc.body.iter().position(|b| b.as_para().is_some_and(|p| p.plain_text().contains("IBAN"))).unwrap();
        // la firma (i primi tre) sul paragrafo dell'IBAN, quello lontano pure (ultimo testo sopra)
        assert!(found.iter().all(|f| f.0 == iban), "{found:?}");
        // e sulla pagina restano dove sono stati scritti (riquadro = punti − metà spessore)
        let mut ys: Vec<f32> = found.iter().map(|f| f.2).collect();
        ys.sort_by(f32::total_cmp);
        for (got, want) in ys.iter().zip([139.5, 149.5, 149.5, 599.5]) {
            assert!((got - want).abs() < 0.6, "{ys:?}");
        }
    }
}
