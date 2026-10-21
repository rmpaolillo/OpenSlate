//! Immagini vettoriali SVG col motore di VectorCraft
//! (https://github.com/storytold/vectorcraft, MIT OR Apache-2.0, © ArtCraft
//! Team), senza la sua interfaccia, come i documenti Word con WordCraft: le
//! pagine per la lavagna (un PDF fatto da VectorCraft, una pagina per tavola)
//! e il salvataggio firmato nello stesso formato. Un SVG resta SVG, con la
//! scrittura del pennino come tracciati vettoriali. (Le foto, PNG e JPEG, le
//! apre PhotoCraft: vedi `rasign`; qui restano per chi chiama `save_with_ink`
//! con un'immagine raster.)

use crate::wordsign::InkStroke;
use anyhow::{anyhow, bail, Result};
use serde_json::json;
use std::path::Path;
use vectorcraft_engine::Session;

/// Estensioni aperte con VectorCraft.
pub fn is_image(path: &Path) -> bool {
    matches!(ext(path).as_str(), "svg" | "svgz")
}

fn ext(path: &Path) -> String {
    path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase()
}

fn run(s: &mut Session, cmd: &str, params: serde_json::Value) -> Result<serde_json::Value> {
    s.execute(cmd, &params).map_err(|e| anyhow!("VectorCraft, {cmd}: {e}"))
}

/// Apre un'immagine col motore di VectorCraft.
pub fn open(path: &Path) -> Result<Session> {
    let mut s = Session::new();
    run(&mut s, "document.open", json!({ "path": path.to_string_lossy() })).map_err(|e| anyhow!("VectorCraft non apre l'immagine: {e}"))?;
    Ok(s)
}

/// L'immagine come PDF (una pagina per tavola), per le pagine della lavagna.
pub fn to_pdf(input: &Path, out: &Path) -> Result<()> {
    let mut s = open(input)?;
    // niente copia del documento di VectorCraft dentro il PDF: serve solo a vedere
    run(&mut s, "document.exportPdf", json!({ "path": out.to_string_lossy(), "preserveEditing": false }))?;
    if !out.exists() {
        bail!("VectorCraft non ha scritto il PDF");
    }
    Ok(())
}

/// L'immagine in PNG (la prima tavola, su bianco), disegnata da VectorCraft:
/// per le figure dentro un documento di WordCraft.
pub fn to_png(input: &Path, ppi: f64) -> Result<Vec<u8>> {
    use base64::Engine;
    let mut s = open(input)?;
    let r = run(&mut s, "document.export", json!({ "format": "png", "artboard": 0, "ppi": ppi, "background": "white" }))?;
    let b = r["dataBase64"].as_str().ok_or_else(|| anyhow!("VectorCraft: nessun PNG"))?;
    Ok(base64::engine::general_purpose::STANDARD.decode(b)?)
}

/// Le tavole (pagine) in punti: x, y, larghezza, altezza.
fn artboards(s: &Session) -> Result<Vec<[f64; 4]>> {
    let d = s.doc().map_err(|e| anyhow!("VectorCraft: {e}"))?;
    Ok(d.doc.artboards.iter().map(|a| [a.rect.x0, a.rect.y0, a.rect.width(), a.rect.height()]).collect())
}

/// Mette i tratti nell'immagine: ogni tratto un tracciato con le punte tonde
/// (l'evidenziatore trasparente, sotto lo stesso colore).
fn add_ink(s: &mut Session, strokes: &[InkStroke]) -> Result<()> {
    let boards = artboards(s)?;
    for st in strokes {
        let Some(&[bx, by, bw, bh]) = boards.get(st.page) else { continue };
        let mut pts: Vec<[f64; 2]> = st
            .pts
            .iter()
            .take(5000)
            .map(|(u, v)| [bx + u.clamp(0.0, 1.0) as f64 * bw, by + v.clamp(0.0, 1.0) as f64 * bh])
            .collect();
        if pts.is_empty() {
            continue;
        }
        // un punto solo (un puntino): un trattino brevissimo
        if pts.len() == 1 {
            pts.push([pts[0][0] + 0.2, pts[0][1]]);
        }
        let color = format!("#{:02x}{:02x}{:02x}", st.color[0], st.color[1], st.color[2]);
        let width = (st.width_frac as f64 * bh).clamp(0.25, 72.0);
        // con niente selezionato questi comandi cambiano il colore e il tratto
        // dei nuovi disegni
        run(s, "paint.setStroke", json!({ "color": color }))?;
        run(s, "stroke.set", json!({ "weight": width, "cap": "round", "join": "round" }))?;
        s.paint.opacity = if st.highlight { 0.4 } else { 1.0 };
        run(s, "path.freehand", json!({ "points": pts, "fidelity": 0.25, "keepSelected": false }))?;
    }
    s.paint.opacity = 1.0;
    Ok(())
}

/// Copia `src` in `dst` (stesso formato) con i tratti.
pub fn save_with_ink(src: &Path, dst: &Path, strokes: &[InkStroke]) -> Result<()> {
    let mut s = open(src)?;
    add_ink(&mut s, strokes)?;
    save_like(&mut s, src, dst)
}

/// Mette l'SVG `svg` (un grafico della chat) nell'immagine `file`, alla
/// larghezza e altezza date (punti), con l'angolo in alto a sinistra in `at`
/// (punti dalla tavola) o al centro; riscrive il file nel suo formato.
pub fn place_svg(file: &Path, svg: &Path, at: Option<[f32; 2]>, w: f32, h: f32) -> Result<()> {
    use base64::Engine;
    let mut s = open(file)?;
    let [bx, by, bw, bh] = *artboards(&s)?.first().ok_or_else(|| anyhow!("VectorCraft: nessuna tavola"))?;
    let (w, h) = if (w as f64) > bw * 0.95 { (bw * 0.95, h as f64 * bw * 0.95 / w as f64) } else { (w as f64, h as f64) };
    let [x, y] = at.map(|[x, y]| [x as f64, y as f64]).unwrap_or([(bw - w) / 2.0, (bh - h) / 2.0]);
    run(&mut s, "select.none", json!({}))?;
    let data = base64::engine::general_purpose::STANDARD.encode(std::fs::read(svg)?);
    run(&mut s, "file.place", json!({ "name": "grafico.svg", "dataBase64": data }))?;
    run(&mut s, "object.setBounds", json!({ "x": bx + x, "y": by + y, "width": w, "height": h, "reference": 0 }))?;
    run(&mut s, "select.none", json!({}))?;
    let tmp = file.with_extension(format!("nuovo.{}", ext(file)));
    save_like(&mut s, file, &tmp)?;
    std::fs::rename(&tmp, file)?;
    Ok(())
}

/// Scrive la sessione in `dst` nel formato di `src` (un raster alla sua risoluzione).
fn save_like(s: &mut Session, src: &Path, dst: &Path) -> Result<()> {
    let s = s;
    match ext(src).as_str() {
        "svg" | "svgz" => {
            let format = if ext(dst) == "svgz" { "svgz" } else { "svg" };
            run(s, "document.save", json!({ "path": dst.to_string_lossy(), "format": format, "acknowledgeLoss": true }))?;
        }
        e @ ("png" | "jpg" | "jpeg") => {
            // stessa risoluzione dell'originale: i pixel restano quelli
            let bytes = std::fs::read(src)?;
            let ppi = vectorcraft_engine::cmd::fileio::ppi::resolution(&bytes).map(|(x, _)| x).filter(|p| *p > 0.0).unwrap_or(72.0);
            let format = if e == "png" { "png" } else { "jpg" };
            let mut p = json!({ "path": dst.to_string_lossy(), "format": format, "artboard": 0, "ppi": ppi });
            if format == "jpg" {
                p["background"] = json!("white");
            }
            run(s, "document.export", p)?;
        }
        other => bail!("formato non gestito: {other}"),
    }
    if !dst.exists() {
        bail!("VectorCraft non ha scritto {}", dst.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("openslate-vec-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d.join(name)
    }

    fn ink() -> Vec<InkStroke> {
        vec![InkStroke { page: 0, pts: vec![(0.2, 0.5), (0.5, 0.6), (0.8, 0.5)], color: [200, 0, 0], width_frac: 0.02, highlight: false }]
    }

    #[test]
    fn svg_firmato_resta_svg() {
        let src = tmp("prova.svg");
        std::fs::write(&src, r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100" viewBox="0 0 200 100"><rect x="10" y="10" width="50" height="30" fill="#00f"/></svg>"##).unwrap();
        let pdf = tmp("prova.pdf");
        to_pdf(&src, &pdf).unwrap();
        assert!(std::fs::read(&pdf).unwrap().starts_with(b"%PDF"));
        let out = tmp("prova-firmato.svg");
        save_with_ink(&src, &out, &ink()).unwrap();
        let svg = std::fs::read_to_string(&out).unwrap();
        assert!(svg.contains("<svg") && svg.contains("<path") && svg.to_lowercase().contains("c80000"), "{svg}");
    }

    #[test]
    fn png_firmato_resta_png_con_gli_stessi_pixel() {
        let src = tmp("prova.png");
        let mut px = tiny_skia::Pixmap::new(300, 150).unwrap();
        px.fill(tiny_skia::Color::WHITE);
        std::fs::write(&src, px.encode_png().unwrap()).unwrap();
        let pdf = tmp("prova-png.pdf");
        to_pdf(&src, &pdf).unwrap();
        let out = tmp("prova-firmato.png");
        save_with_ink(&src, &out, &ink()).unwrap();
        let img = tiny_skia::Pixmap::decode_png(&std::fs::read(&out).unwrap()).unwrap();
        assert_eq!((img.width(), img.height()), (300, 150));
        // a metà del tratto (x 50%, y 50–65%) c'è il rosso pieno, e il resto è bianco
        let red = (75..98).filter_map(|y| img.pixel(150, y)).any(|p| p.red() > 150 && p.green() < 60);
        assert!(red, "{:?}", (75..98).map(|y| img.pixel(150, y).unwrap().green()).collect::<Vec<_>>());
        assert_eq!(img.pixel(20, 20).unwrap().green(), 255);
    }
}
