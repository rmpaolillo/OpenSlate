//! I formati in cui si esportano le pagine (lavagna e archivio).
//!
//! - PDF (VectorCraft), con lo **strato di testo** quando la pagina è stata
//!   trascritta: il testo è invisibile sopra le righe scritte a mano, così il
//!   PDF si cerca (Spotlight, Finder, lettori PDF) e il testo si copia;
//! - PNG e SVG (VectorCraft, PhotoCraft) e, più leggeri da mandare, **JPEG** e
//!   **WebP** (PhotoCraft);
//! - **Word**: la pagina come inchiostro vero di Word (si modifica in Word) e,
//!   se c'è, la trascrizione nella pagina dopo (WordCraft);
//! - **Xournal++** (`.xopp`): i tratti, che lì restano modificabili;
//! - **GIF** e **SVG animato**: la pagina che si riscrive da sola, un tratto
//!   dopo l'altro, nell'ordine in cui è stata scritta;
//! - **LaTeX** e **Typst**: la trascrizione (vedi `mdconv`).

use crate::scene::{Page, PageStroke};
use anyhow::{anyhow, bail, Context, Result};
use std::path::{Path, PathBuf};

/// Un formato d'uscita.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Pdf,
    Png,
    Svg,
    Jpeg,
    Webp,
    Word,
    Xournal,
    Gif,
    SvgAnim,
    Latex,
    Typst,
}

impl Kind {
    /// I formati del menu «Altri formati».
    pub const MORE: [Kind; 8] = [Kind::Jpeg, Kind::Webp, Kind::Word, Kind::Xournal, Kind::Gif, Kind::SvgAnim, Kind::Latex, Kind::Typst];

    pub fn ext(self) -> &'static str {
        match self {
            Kind::Pdf => "pdf",
            Kind::Png => "png",
            Kind::Svg | Kind::SvgAnim => "svg",
            Kind::Jpeg => "jpg",
            Kind::Webp => "webp",
            Kind::Word => "docx",
            Kind::Xournal => "xopp",
            Kind::Gif => "gif",
            Kind::Latex => "tex",
            Kind::Typst => "typ",
        }
    }

    /// Il nome del tipo di file (finestra «Salva con nome»).
    pub fn kind_name(self) -> &'static str {
        match self {
            Kind::Pdf => "PDF",
            Kind::Png => "Immagine PNG",
            Kind::Svg => "Immagine SVG",
            Kind::Jpeg => "Immagine JPEG",
            Kind::Webp => "Immagine WebP",
            Kind::Word => "Word",
            Kind::Xournal => "Xournal++",
            Kind::Gif => "GIF animata",
            Kind::SvgAnim => "SVG animato",
            Kind::Latex => "LaTeX",
            Kind::Typst => "Typst",
        }
    }

    /// La voce del menu.
    pub fn label(self) -> &'static str {
        match self {
            Kind::Pdf => "PDF",
            Kind::Png => "PNG",
            Kind::Svg => "SVG",
            Kind::Jpeg => "JPEG (leggero, da mandare)",
            Kind::Webp => "WebP (leggero, da mandare)",
            Kind::Word => "Word, con l'inchiostro modificabile",
            Kind::Xournal => "Xournal++ (.xopp)",
            Kind::Gif => "GIF animata: la scrittura che si rifà",
            Kind::SvgAnim => "SVG animato: la scrittura che si rifà",
            Kind::Latex => "LaTeX dalla trascrizione (.tex)",
            Kind::Typst => "Typst dalla trascrizione (.typ)",
        }
    }

    pub fn icon(self) -> &'static str {
        use egui_phosphor::regular as ic;
        match self {
            Kind::Pdf => ic::FILE_PDF,
            Kind::Png => ic::FILE_PNG,
            Kind::Svg => ic::FILE_SVG,
            Kind::Jpeg | Kind::Webp => ic::FILE_JPG,
            Kind::Word => ic::MICROSOFT_WORD_LOGO,
            Kind::Xournal => ic::NOTEBOOK,
            Kind::Gif | Kind::SvgAnim => ic::FILM_STRIP,
            Kind::Latex | Kind::Typst => ic::FUNCTION,
        }
    }

    /// Dal nome (strumento della chat): pdf, png, svg, jpeg, webp, word,
    /// xournal, gif, svg_animato, latex, typst (e le estensioni).
    pub fn from_name(s: &str) -> Option<Kind> {
        Some(match s.trim().to_ascii_lowercase().replace(['-', ' '], "_").as_str() {
            "pdf" => Kind::Pdf,
            "png" => Kind::Png,
            "svg" => Kind::Svg,
            "jpg" | "jpeg" => Kind::Jpeg,
            "webp" => Kind::Webp,
            "word" | "docx" => Kind::Word,
            "xournal" | "xournal++" | "xournalpp" | "xopp" => Kind::Xournal,
            "gif" | "gif_animata" => Kind::Gif,
            "svg_animato" | "svg_animata" | "svg_animated" | "animated_svg" => Kind::SvgAnim,
            "latex" | "tex" => Kind::Latex,
            "typst" | "typ" => Kind::Typst,
            _ => return None,
        })
    }

    /// Tutte le pagine in un file solo (il PDF); gli altri: un file per pagina.
    pub fn one_file(self) -> bool {
        self == Kind::Pdf
    }

    /// Esporta la trascrizione, non i tratti.
    pub fn needs_note(self) -> bool {
        matches!(self, Kind::Latex | Kind::Typst)
    }
}

/// Una pagina da esportare: titolo, pagina, l'appunto con la sua trascrizione.
pub struct Item {
    pub title: String,
    pub page: Page,
    pub note: Option<PathBuf>,
}

/// Le scelte che valgono per tutti i file.
#[derive(Clone)]
pub struct Opts {
    /// La scrittura degli appunti (Word con la trascrizione).
    pub hand: crate::mdcraft::Hand,
    /// La rigatura del foglio (`settings.paper`), per lo sfondo di Xournal++.
    pub paper: String,
}

impl Default for Opts {
    fn default() -> Self {
        Opts { hand: crate::mdcraft::Hand::Stampato, paper: "bianco".into() }
    }
}

/// Scrive una pagina in `path` nel formato `kind` (non il PDF di più pagine:
/// quello è `write_pdf`). Ritorna i file scritti (le figure accanto, per
/// LaTeX e Typst).
pub fn write_one(kind: Kind, item: &Item, path: &Path, opts: &Opts) -> Result<Vec<PathBuf>> {
    let page = &item.page;
    match kind {
        Kind::Pdf => write_pdf(std::slice::from_ref(item), path)?,
        Kind::Png => crate::export::save_png(page, path)?,
        Kind::Svg => crate::export::save_svg(page, path)?,
        Kind::Jpeg => std::fs::write(path, crate::craftexport::raster(page, 2000, photocraft_codecs::Format::Jpeg)?)?,
        Kind::Webp => std::fs::write(path, crate::craftexport::raster(page, 2000, photocraft_codecs::Format::WebP)?)?,
        Kind::Word => word(item, path, opts)?,
        Kind::Xournal => std::fs::write(path, xournal(page, &opts.paper)?)?,
        Kind::Gif => std::fs::write(path, gif(page)?)?,
        Kind::SvgAnim => std::fs::write(path, svg_anim(page))?,
        Kind::Latex | Kind::Typst => {
            let Some(note) = item.note.as_deref().filter(|n| n.exists()) else {
                bail!("«{}» non è trascritta: prima AI → Trascrivi, poi esporta in {}", item.title, kind.kind_name());
            };
            return crate::mdcraft::export_note_written(note, path, opts.hand);
        }
    }
    Ok(vec![path.to_path_buf()])
}

// ---------------------------------------------------------------------------
// PDF ricercabile
// ---------------------------------------------------------------------------

/// Le pagine in un PDF; quelle trascritte con il testo invisibile sopra.
pub fn write_pdf(items: &[Item], path: &Path) -> Result<()> {
    let refs: Vec<&Page> = items.iter().map(|i| &i.page).collect();
    let mut pdf = crate::craftexport::pdf(&refs)?;
    let layers: Vec<Vec<TextLine>> = items
        .iter()
        .map(|i| {
            i.note
                .as_deref()
                .and_then(|n| std::fs::read_to_string(n).ok())
                .map(|md| text_lines(&i.page, &crate::mdconv::plain_lines(&md)))
                .unwrap_or_default()
        })
        .collect();
    if layers.iter().any(|l| !l.is_empty()) {
        pdf = add_text_layer(&pdf, &layers)?;
    }
    std::fs::write(path, pdf).with_context(|| path.display().to_string())?;
    Ok(())
}

/// Una riga di testo invisibile: il riquadro (u0, v0, u1, v1 sulla pagina) e il testo.
#[derive(Debug, Clone, PartialEq)]
pub struct TextLine {
    pub rect: [f32; 4],
    pub text: String,
}

/// Le righe scritte a mano della pagina (riquadri in unità della pagina),
/// dall'alto: i tratti che si sovrappongono in verticale stanno sulla stessa
/// riga. Si saltano evidenziatori e disegni alti.
pub fn handwriting_rows(page: &Page) -> Vec<[f32; 4]> {
    let mut boxes: Vec<[f32; 4]> = page
        .strokes
        .iter()
        .filter(|s| !s.highlight && !s.pts.is_empty())
        .map(|s| s.pts.iter().fold([f32::MAX, f32::MAX, f32::MIN, f32::MIN], |b, &(u, v)| [b[0].min(u), b[1].min(v), b[2].max(u), b[3].max(v)]))
        .filter(|b| b[3] - b[1] < 0.12)
        .collect();
    boxes.sort_by(|a, b| (a[1] + a[3]).total_cmp(&(b[1] + b[3])));
    let mut rows: Vec<[f32; 4]> = Vec::new();
    for b in boxes {
        let h = (b[3] - b[1]).max(0.004);
        let hit = rows.iter_mut().find(|r| {
            let over = r[3].min(b[3]) - r[1].max(b[1]);
            let rh = (r[3] - r[1]).max(0.004);
            // segni piccoli (punti, trattini) vanno con la riga che li contiene
            over > 0.4 * h.min(rh) || (h < 0.35 * rh && (b[1] + b[3]) / 2.0 > r[1] && (b[1] + b[3]) / 2.0 < r[3])
        });
        match hit {
            Some(r) => *r = [r[0].min(b[0]), r[1].min(b[1]), r[2].max(b[2]), r[3].max(b[3])],
            None => rows.push(b),
        }
    }
    rows.sort_by(|a, b| a[1].total_cmp(&b[1]));
    rows
}

/// Dove va ogni riga della trascrizione: sulla riga scritta corrispondente se
/// sono tante quante, altrimenti distribuite sullo spazio scritto.
pub fn text_lines(page: &Page, lines: &[String]) -> Vec<TextLine> {
    let lines: Vec<&String> = lines.iter().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() {
        return Vec::new();
    }
    let rows = handwriting_rows(page);
    if rows.len() == lines.len() {
        return rows.iter().zip(&lines).map(|(r, t)| TextLine { rect: *r, text: t.to_string() }).collect();
    }
    let band = rows.iter().copied().reduce(|a, b| [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])]).unwrap_or([0.1, 0.08, 0.9, 0.9]);
    let n = lines.len() as f32;
    let slot = (band[3] - band[1]).max(0.02) / n;
    let typical = rows.iter().map(|r| r[3] - r[1]).fold(0.0f32, f32::max).clamp(0.012, 0.04);
    let h = slot.min(typical);
    lines
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let top = band[1] + i as f32 * slot;
            TextLine { rect: [band[0], top, band[2].max(band[0] + 0.05), top + h], text: t.to_string() }
        })
        .collect()
}

/// Il carattere vuoto (due glifi senza segni) dello strato di testo.
const GLYPHLESS: &[u8] = include_bytes!("../assets/fonts/GlyphLess.ttf");

/// Aggiunge a ogni pagina del PDF le sue righe di testo invisibili (modo 3
/// di riempimento): un carattere Type0 con codici = Unicode (UTF-16) e una
/// ToUnicode identica, come fanno i programmi di OCR. I glifi sono vuoti.
pub fn add_text_layer(pdf: &[u8], per_page: &[Vec<TextLine>]) -> Result<Vec<u8>> {
    use lopdf::{dictionary, Dictionary, Document, Object, Stream};
    let mut doc = Document::load_mem(pdf).context("PDF non leggibile")?;
    let page_ids: Vec<lopdf::ObjectId> = doc.get_pages().values().copied().collect();

    // il carattere, una volta per tutte le pagine
    let mut ff = Stream::new(dictionary! { "Length1" => GLYPHLESS.len() as i64 }, GLYPHLESS.to_vec());
    let _ = ff.compress();
    let ff_id = doc.add_object(ff);
    let gid1: Vec<u8> = (0..65536u32).flat_map(|_| [0u8, 1u8]).collect();
    let mut cid2gid = Stream::new(Dictionary::new(), gid1);
    let _ = cid2gid.compress();
    let cid2gid_id = doc.add_object(cid2gid);
    let fd_id = doc.add_object(dictionary! {
        "Type" => "FontDescriptor", "FontName" => "GlyphLessFont", "Flags" => 5,
        "FontBBox" => vec![0.into(), 0.into(), 500.into(), 1000.into()],
        "ItalicAngle" => 0, "Ascent" => 1000, "Descent" => 0, "CapHeight" => 1000, "StemV" => 80,
        "FontFile2" => ff_id,
    });
    let cid_id = doc.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "CIDFontType2", "BaseFont" => "GlyphLessFont",
        "CIDSystemInfo" => dictionary! { "Registry" => Object::string_literal("Adobe"), "Ordering" => Object::string_literal("Identity"), "Supplement" => 0 },
        "FontDescriptor" => fd_id, "DW" => 500, "CIDToGIDMap" => cid2gid_id,
    });
    let mut cmap = String::from(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
         /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
         /CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n",
    );
    let hi: Vec<u32> = (0..256u32).filter(|h| !(0xD8..=0xDF).contains(h)).collect();
    for chunk in hi.chunks(100) {
        cmap.push_str(&format!("{} beginbfrange\n", chunk.len()));
        for h in chunk {
            cmap.push_str(&format!("<{h:02X}00> <{h:02X}FF> <{h:02X}00>\n"));
        }
        cmap.push_str("endbfrange\n");
    }
    cmap.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");
    let mut tu = Stream::new(Dictionary::new(), cmap.into_bytes());
    let _ = tu.compress();
    let tu_id = doc.add_object(tu);
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type0", "BaseFont" => "GlyphLessFont", "Encoding" => "Identity-H",
        "DescendantFonts" => vec![cid_id.into()], "ToUnicode" => tu_id,
    });

    for (i, lines) in per_page.iter().enumerate() {
        let Some(&pid) = page_ids.get(i) else { break };
        if lines.is_empty() {
            continue;
        }
        let page = crate::pdfdoc::page_of(&doc, pid);
        let (pw, ph) = page.display_size();
        let mut c = String::from("q\nBT\n3 Tr\n");
        for l in lines {
            let units: Vec<u16> = l.text.chars().filter(|&ch| (ch as u32) < 0x10000 && !ch.is_control()).map(|ch| ch as u16).collect();
            if units.is_empty() {
                continue;
            }
            let [u0, v0, u1, v1] = l.rect;
            let fs = ((v1 - v0) * ph * 0.85).clamp(4.0, 40.0);
            let width = ((u1 - u0) * pw).max(fs);
            let natural = units.len() as f32 * 0.5 * fs;
            let tz = (100.0 * width / natural).clamp(1.0, 1000.0);
            // la linea di base poco sopra il fondo della riga scritta
            let (x, y) = page.to_user(u0, v1 - (v1 - v0) * 0.15);
            let hex: String = units.iter().map(|u| format!("{u:04X}")).collect();
            c.push_str(&format!("/FGL {fs:.2} Tf\n{tz:.2} Tz\n1 0 0 1 {x:.2} {y:.2} Tm\n<{hex}> Tj\n"));
        }
        c.push_str("ET\nQ\n");
        let tx_id = doc.add_object(Stream::new(Dictionary::new(), c.into_bytes()));
        // risorse (anche ereditate) + il carattere
        let mut res = match crate::pdfdoc::inherited(&doc, pid, b"Resources") {
            Some(Object::Reference(id)) => doc.get_dictionary(id).cloned().unwrap_or_default(),
            Some(Object::Dictionary(d)) => d,
            _ => Dictionary::new(),
        };
        let mut fonts = match res.get(b"Font") {
            Ok(Object::Reference(id)) => doc.get_dictionary(*id).cloned().unwrap_or_default(),
            Ok(Object::Dictionary(d)) => d.clone(),
            _ => Dictionary::new(),
        };
        fonts.set("FGL", Object::Reference(font_id));
        res.set("Font", Object::Dictionary(fonts));
        let old = doc.get_dictionary(pid)?.get(b"Contents").ok().cloned();
        let mut arr = Vec::new();
        match old {
            Some(Object::Array(a)) => arr.extend(a),
            Some(Object::Reference(id)) => match doc.get_object(id) {
                Ok(Object::Array(a)) => arr.extend(a.clone()),
                _ => arr.push(Object::Reference(id)),
            },
            _ => {}
        }
        arr.push(Object::Reference(tx_id));
        let d = doc.get_dictionary_mut(pid)?;
        d.set("Resources", Object::Dictionary(res));
        d.set("Contents", Object::Array(arr));
    }
    let mut out = Vec::new();
    doc.save_to(&mut out).map_err(|e| anyhow!("PDF: {e}"))?;
    Ok(out)
}

// ---------------------------------------------------------------------------
// Word
// ---------------------------------------------------------------------------

/// La pagina in Word: i tratti come inchiostro di Word (si spostano, si
/// ricolorano, si cancellano in Word) e, se la pagina è trascritta, la
/// trascrizione nella pagina dopo.
fn word(item: &Item, path: &Path, opts: &Opts) -> Result<()> {
    let tmp;
    let note = item.note.as_deref().filter(|n| n.exists());
    let src = match note {
        Some(n) => n.to_path_buf(),
        None => {
            tmp = tempfile::Builder::new().prefix("openslate-").suffix(".md").tempfile()?;
            std::fs::write(tmp.path(), " \n")?;
            tmp.path().to_path_buf()
        }
    };
    let mut s = crate::wordsign::open_with(&src, opts.hand)?;
    if note.is_some() {
        // la pagina scritta a mano prima, la trascrizione dalla pagina dopo: un
        // paragrafo vuoto in cima (lì si aggancia l'inchiostro) e il primo della
        // trascrizione con «interruzione di pagina prima» (resta un titolo)
        for (cmd, args) in [("caret.docStart", serde_json::json!({})), ("text.newParagraph", serde_json::json!({})), ("para.pageBreakBefore", serde_json::json!({ "value": true }))] {
            s.run(cmd, &args).map_err(|e| anyhow!("WordCraft, {cmd}: {e}"))?;
        }
    }
    let strokes: Vec<crate::wordsign::InkStroke> = item
        .page
        .strokes
        .iter()
        .filter(|st| !st.pts.is_empty())
        .map(|st| crate::wordsign::InkStroke {
            page: 0,
            pts: st.pts.clone(),
            color: st.color,
            width_frac: crate::export::stroke_width(st.width, item.page.h) / item.page.h,
            highlight: st.highlight,
        })
        .collect();
    crate::wordsign::add_ink(&mut s, &strokes)?;
    s.run("file.save", &serde_json::json!({ "path": path.to_string_lossy() })).map_err(|e| anyhow!("WordCraft, Word: {e}"))?;
    if !path.exists() {
        bail!("WordCraft non ha scritto {}", path.display());
    }
    if note.is_some() {
        if let Some((name, regular, bold)) = opts.hand.files() {
            crate::docxfonts::embed(path, name, regular, bold).context("caratteri a mano nel Word")?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Xournal++
// ---------------------------------------------------------------------------

/// Millimetri → punti.
const MM: f32 = 72.0 / 25.4;

fn hex_color(c: [u8; 3], alpha: u8) -> String {
    format!("#{:02x}{:02x}{:02x}{alpha:02x}", c[0], c[1], c[2])
}

/// La pagina come documento di Xournal++ (`.xopp`: XML compresso con gzip).
pub fn xournal(page: &Page, paper: &str) -> Result<Vec<u8>> {
    use std::fmt::Write;
    let (wmm, hmm) = crate::export::page_mm(page);
    let (w, h) = (wmm * MM, hmm * MM);
    let style = if paper.starts_with("righe") {
        "lined"
    } else if paper.starts_with("quadretti") || paper.contains("millimetrata") || paper.contains("carta") {
        "graph"
    } else {
        "plain"
    };
    let mut x = String::new();
    x.push_str("<?xml version=\"1.0\" standalone=\"no\"?>\n<xournal creator=\"OpenSlate\" fileversion=\"4\">\n");
    x.push_str("<title>Xournal++ document - see https://xournalpp.github.io/</title>\n");
    let _ = writeln!(x, "<page width=\"{w:.2}\" height=\"{h:.2}\">");
    let _ = writeln!(x, "<background type=\"solid\" color=\"#ffffffff\" style=\"{style}\"/>");
    x.push_str("<layer>\n");
    if let Some(bg) = &page.bg {
        use base64::Engine;
        let [rx, ry, rw, rh] = bg.rect;
        let _ = writeln!(
            x,
            "<image left=\"{:.2}\" top=\"{:.2}\" right=\"{:.2}\" bottom=\"{:.2}\">{}</image>",
            rx * w,
            ry * h,
            (rx + rw) * w,
            (ry + rh) * h,
            base64::engine::general_purpose::STANDARD.encode(bg.png.as_slice())
        );
    }
    for st in &page.strokes {
        if st.pts.is_empty() {
            continue;
        }
        let width = crate::export::stroke_width(st.width, h);
        let (tool, color) = if st.highlight { ("highlighter", hex_color(st.color, 0x7f)) } else { ("pen", hex_color(st.color, 0xff)) };
        let mut pts: Vec<(f32, f32)> = st.pts.iter().map(|&(u, v)| (u * w, v * h)).collect();
        if pts.len() == 1 {
            pts.push((pts[0].0 + 0.01, pts[0].1));
        }
        let coords: Vec<String> = pts.iter().map(|(a, b)| format!("{a:.2} {b:.2}")).collect();
        let _ = writeln!(x, "<stroke tool=\"{tool}\" color=\"{color}\" width=\"{width:.2}\">{}</stroke>", coords.join(" "));
    }
    x.push_str("</layer>\n</page>\n</xournal>\n");
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    std::io::Write::write_all(&mut gz, x.as_bytes())?;
    Ok(gz.finish()?)
}

// ---------------------------------------------------------------------------
// La scrittura che si rifà: GIF e SVG animato
// ---------------------------------------------------------------------------

/// Il tratto `s` fino alla frazione `t` della sua lunghezza (`a` = larghezza
/// su altezza della pagina, per misurare le lunghezze giuste).
fn partial(s: &PageStroke, t: f32, a: f32) -> PageStroke {
    let len = stroke_len(s, a);
    let want = len * t.clamp(0.0, 1.0);
    let mut pts = vec![s.pts[0]];
    let mut acc = 0.0;
    for w in s.pts.windows(2) {
        let d = (((w[1].0 - w[0].0) * a).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt();
        if acc + d >= want {
            let f = if d > 0.0 { (want - acc) / d } else { 0.0 };
            pts.push((w[0].0 + (w[1].0 - w[0].0) * f, w[0].1 + (w[1].1 - w[0].1) * f));
            break;
        }
        acc += d;
        pts.push(w[1]);
    }
    PageStroke { pts, ..s.clone() }
}

fn stroke_len(s: &PageStroke, a: f32) -> f32 {
    s.pts.windows(2).map(|w| (((w[1].0 - w[0].0) * a).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt()).sum::<f32>()
}

/// I tratti nell'ordine in cui si rifanno: prima l'inchiostro (nell'ordine
/// scritto), poi gli evidenziatori; e quando comincia e finisce ciascuno, in
/// frazioni del disegno intero (i salti fra un tratto e l'altro contano un po').
fn timeline(page: &Page) -> Vec<(&PageStroke, f32, f32)> {
    let a = page.w / page.h;
    let order: Vec<&PageStroke> = page.strokes.iter().filter(|s| !s.highlight).chain(page.strokes.iter().filter(|s| s.highlight)).filter(|s| !s.pts.is_empty()).collect();
    const GAP: f32 = 0.004;
    let lens: Vec<f32> = order.iter().map(|s| stroke_len(s, a).max(0.002)).collect();
    let total: f32 = lens.iter().sum::<f32>() + GAP * order.len() as f32;
    let mut t = 0.0;
    order
        .into_iter()
        .zip(lens)
        .map(|(s, l)| {
            let start = t / total;
            t += l;
            let end = t / total;
            t += GAP;
            (s, start, end)
        })
        .collect()
}

/// La pagina al momento `t` (0..1) del disegno.
fn page_at(page: &Page, tl: &[(&PageStroke, f32, f32)], t: f32) -> Page {
    let a = page.w / page.h;
    let strokes = tl
        .iter()
        .filter(|(_, s, _)| *s <= t)
        .map(|(st, s, e)| if t >= *e { (*st).clone() } else { partial(st, (t - s) / (e - s).max(1e-6), a) })
        .collect();
    Page { w: page.w, h: page.h, strokes, bg: page.bg.clone() }
}

/// GIF animata: la pagina che si scrive in ~5 s, poi resta ferma 3 s e
/// ricomincia. Ogni fotogramma porta solo la parte che cambia.
pub fn gif(page: &Page) -> Result<Vec<u8>> {
    const LONG: u32 = 900;
    const FRAMES: usize = 80;
    const DELAY: u16 = 6; // centesimi di secondo
    let tl = timeline(page);
    if tl.is_empty() {
        bail!("pagina vuota");
    }
    let (w, h, last) = crate::export::page_to_rgba(page, LONG)?;
    // tavolozza unica, presa dalla pagina finita
    let nq = color_quant::NeuQuant::new(10, 256, &last);
    let palette: Vec<u8> = nq.color_map_rgb();
    let mut cache: std::collections::HashMap<[u8; 3], u8> = std::collections::HashMap::new();
    let mut index = |px: &[u8]| -> u8 {
        let k = [px[0], px[1], px[2]];
        *cache.entry(k).or_insert_with(|| nq.index_of(&[px[0], px[1], px[2], 255]) as u8)
    };
    let mut out = Vec::new();
    {
        let mut enc = gif::Encoder::new(&mut out, w as u16, h as u16, &palette).map_err(|e| anyhow!("GIF: {e}"))?;
        enc.set_repeat(gif::Repeat::Infinite).map_err(|e| anyhow!("GIF: {e}"))?;
        let mut prev: Option<Vec<u8>> = None;
        for k in 0..=FRAMES {
            let t = k as f32 / FRAMES as f32;
            let rgba = if k == FRAMES { last.clone() } else { crate::export::page_to_rgba(&page_at(page, &tl, t), LONG)?.2 };
            // la zona cambiata rispetto al fotogramma prima
            let (x0, y0, x1, y1) = match &prev {
                None => (0, 0, w, h),
                Some(p) => {
                    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
                    for y in 0..h {
                        let row = y * w * 4;
                        for x in 0..w {
                            let i = row + x * 4;
                            if p[i..i + 3] != rgba[i..i + 3] {
                                x0 = x0.min(x);
                                y0 = y0.min(y);
                                x1 = x1.max(x + 1);
                                y1 = y1.max(y + 1);
                            }
                        }
                    }
                    if x1 <= x0 {
                        (0, 0, 1, 1)
                    } else {
                        (x0, y0, x1, y1)
                    }
                }
            };
            let (fw, fh) = (x1 - x0, y1 - y0);
            let mut buf = Vec::with_capacity(fw * fh);
            for y in y0..y1 {
                for x in x0..x1 {
                    let i = (y * w + x) * 4;
                    buf.push(index(&rgba[i..i + 4]));
                }
            }
            let frame = gif::Frame {
                left: x0 as u16,
                top: y0 as u16,
                width: fw as u16,
                height: fh as u16,
                buffer: std::borrow::Cow::Owned(buf),
                delay: if k == FRAMES { 300 } else { DELAY },
                dispose: gif::DisposalMethod::Keep,
                ..Default::default()
            };
            enc.write_frame(&frame).map_err(|e| anyhow!("GIF: {e}"))?;
            prev = Some(rgba);
        }
    }
    Ok(out)
}

/// SVG animato: ogni tratto si disegna da solo (CSS), poi la pagina resta
/// ferma e ricomincia. Si apre in qualsiasi browser.
pub fn svg_anim(page: &Page) -> String {
    use std::fmt::Write;
    const DRAW: f32 = 6.0; // secondi per scrivere la pagina
    const HOLD: f32 = 3.0;
    let cycle = DRAW + HOLD;
    let (wmm, hmm) = crate::export::page_mm(page);
    let tl = timeline(page);
    let mut s = String::new();
    let _ = writeln!(
        s,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"{wmm:.1}mm\" height=\"{hmm:.1}mm\" viewBox=\"0 0 {wmm:.3} {hmm:.3}\">"
    );
    s.push_str("<!-- OpenSlate: la pagina che si riscrive da sola (https://github.com/rmpaolillo/OpenSlate) -->\n<style>\n");
    s.push_str(".s{fill:none;stroke-linecap:round;stroke-linejoin:round;stroke-dasharray:1 2;stroke-dashoffset:1.01}\n.h{mix-blend-mode:multiply}\n");
    for (i, (_, a, b)) in tl.iter().enumerate() {
        let (p0, p1) = (a * DRAW / cycle * 100.0, (b * DRAW / cycle * 100.0).max(a * DRAW / cycle * 100.0 + 0.01));
        let _ = writeln!(
            s,
            "@keyframes k{i}{{0%,{p0:.3}%{{stroke-dashoffset:1.01}}{p1:.3}%,100%{{stroke-dashoffset:0}}}}\n#t{i}{{animation:k{i} {cycle}s linear infinite}}"
        );
    }
    s.push_str("</style>\n");
    let _ = writeln!(s, "<rect width=\"{wmm:.3}\" height=\"{hmm:.3}\" fill=\"#ffffff\"/>");
    if let Some(bg) = &page.bg {
        use base64::Engine;
        let [rx, ry, rw, rh] = bg.rect;
        let _ = writeln!(
            s,
            "<image x=\"{:.3}\" y=\"{:.3}\" width=\"{:.3}\" height=\"{:.3}\" preserveAspectRatio=\"none\" xlink:href=\"data:image/png;base64,{}\"/>",
            rx * wmm,
            ry * hmm,
            rw * wmm,
            rh * hmm,
            base64::engine::general_purpose::STANDARD.encode(bg.png.as_slice())
        );
    }
    for (i, (st, _, _)) in tl.iter().enumerate() {
        let mut d = String::new();
        for (k, &(u, v)) in st.pts.iter().enumerate() {
            let _ = write!(d, "{}{:.2} {:.2}", if k == 0 { "M" } else { " L" }, u * wmm, v * hmm);
        }
        if st.pts.len() == 1 {
            let _ = write!(d, " l0.01 0");
        }
        let width = crate::export::stroke_width(st.width, hmm);
        let c = st.color;
        let _ = writeln!(
            s,
            "<path id=\"t{i}\" class=\"s{}\" pathLength=\"1\" stroke=\"#{:02x}{:02x}{:02x}\" stroke-width=\"{width:.3}\" d=\"{d}\"/>",
            if st.highlight { " h" } else { "" },
            c[0],
            c[1],
            c[2]
        );
    }
    s.push_str("</svg>\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Page {
        let line = |v: f32, n: usize| -> Vec<PageStroke> {
            (0..n)
                .map(|k| {
                    let u = 0.1 + k as f32 * 0.05;
                    PageStroke { color: [20, 20, 40], width: 2.0, pts: vec![(u, v), (u + 0.02, v + 0.02), (u + 0.04, v)], highlight: false }
                })
                .collect()
        };
        let mut strokes = line(0.10, 6);
        strokes.extend(line(0.20, 4));
        strokes.push(PageStroke { color: [250, 210, 60], width: 9.0, pts: vec![(0.1, 0.21), (0.3, 0.21)], highlight: true });
        Page { w: 21000.0, h: 29700.0, strokes, bg: None }
    }

    #[test]
    fn righe_della_pagina() {
        let r = handwriting_rows(&sample());
        assert_eq!(r.len(), 2, "{r:?}");
        assert!(r[0][1] < r[1][1]);
        let t = text_lines(&sample(), &["prima riga".into(), "seconda".into()]);
        assert_eq!(t[1].rect, r[1]);
        // più righe di testo che righe scritte: distribuite
        let t = text_lines(&sample(), &["a".into(), "b".into(), "c".into()]);
        assert_eq!(t.len(), 3);
        assert!(t[0].rect[1] < t[2].rect[1]);
    }

    #[test]
    fn pdf_che_si_cerca() {
        let dir = tempfile::tempdir().unwrap();
        let note = dir.path().join("n.md");
        std::fs::write(&note, "# Misure del pendolo\n\nSo $g = 9.8$ m/s² — città\n").unwrap();
        let out = dir.path().join("p.pdf");
        write_pdf(&[Item { title: "p".into(), page: sample(), note: Some(note) }], &out).unwrap();
        let text = crate::docimport::pdf_text(&out).unwrap().join("\n");
        assert!(text.contains("Misure del pendolo"), "{text}");
        assert!(text.contains("città"), "{text}");
    }

    #[test]
    fn xournal_e_animazioni() {
        let p = sample();
        let x = xournal(&p, "righe").unwrap();
        let mut s = String::new();
        std::io::Read::read_to_string(&mut flate2::read::GzDecoder::new(&x[..]), &mut s).unwrap();
        assert!(s.contains("<xournal") && s.contains("style=\"lined\"") && s.matches("<stroke").count() == 11);
        assert!(s.contains("tool=\"highlighter\" color=\"#fad23c7f\""));

        let svg = svg_anim(&p);
        assert_eq!(svg.matches("<path").count(), 11);
        assert!(svg.contains("@keyframes k10") && svg.contains("infinite"));

        let g = gif(&p).unwrap();
        let mut d = gif::DecodeOptions::new();
        d.set_color_output(gif::ColorOutput::Indexed);
        let mut r = d.read_info(&g[..]).unwrap();
        let mut n = 0;
        while r.read_next_frame().unwrap().is_some() {
            n += 1;
        }
        assert_eq!(n, 81);
    }

    #[test]
    fn immagini_leggere() {
        let p = sample();
        let j = crate::craftexport::raster(&p, 600, photocraft_codecs::Format::Jpeg).unwrap();
        assert_eq!(&j[..2], &[0xFF, 0xD8]);
        let w = crate::craftexport::raster(&p, 600, photocraft_codecs::Format::WebP).unwrap();
        assert_eq!(&w[..4], b"RIFF");
        assert_eq!(&w[8..12], b"WEBP");
    }

    #[test]
    fn word_con_inchiostro_e_trascrizione() {
        let dir = tempfile::tempdir().unwrap();
        let note = dir.path().join("n.md");
        std::fs::write(&note, "# Titolo\n\nLa trascrizione.\n").unwrap();
        let out = dir.path().join("p.docx");
        let item = Item { title: "p".into(), page: sample(), note: Some(note) };
        write_one(Kind::Word, &item, &out, &Opts::default()).unwrap();
        let mut z = zip::ZipArchive::new(std::fs::File::open(&out).unwrap()).unwrap();
        let mut xml = String::new();
        std::io::Read::read_to_string(&mut z.by_name("word/document.xml").unwrap(), &mut xml).unwrap();
        assert!(xml.contains("La trascrizione."));
        assert!(xml.contains("pageBreakBefore"), "manca il salto pagina");
        let ink = (0..z.len()).filter(|&i| z.by_index(i).unwrap().name().contains("ink")).count();
        let drawings = xml.matches("<w:drawing").count() + xml.matches("contentPart").count();
        assert!(ink > 0 || drawings >= 11, "manca l'inchiostro: {ink} parti, {drawings} disegni");
        // senza trascrizione: solo la pagina a mano
        let out2 = dir.path().join("q.docx");
        write_one(Kind::Word, &Item { title: "q".into(), page: sample(), note: None }, &out2, &Opts::default()).unwrap();
        assert!(out2.exists());
    }

    #[test]
    fn latex_e_typst_vogliono_la_trascrizione() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("p.tex");
        let e = write_one(Kind::Latex, &Item { title: "p".into(), page: sample(), note: None }, &out, &Opts::default()).unwrap_err();
        assert!(e.to_string().contains("Trascrivi"));
        let note = dir.path().join("n.md");
        std::fs::write(&note, "# T\n\n$x^2$\n").unwrap();
        let w = write_one(Kind::Typst, &Item { title: "p".into(), page: sample(), note: Some(note) }, &dir.path().join("p.typ"), &Opts::default()).unwrap();
        assert!(std::fs::read_to_string(&w[0]).unwrap().contains("#mi(\"x^2\")"));
        assert_eq!(Kind::from_name("svg animato"), Some(Kind::SvgAnim));
        assert_eq!(Kind::from_name("DOCX"), Some(Kind::Word));
    }
}
