//! Import di documenti: tutto diventa PDF, poi le pagine diventano immagini
//! da mostrare sulla lavagna.
//!
//! - PDF: usato così com'è.
//! - Word (.doc, .docx), .odt, .rtf, .txt, .html: LibreOffice, se c'è;
//!   altrimenti (macOS) `textutil` → HTML → Google Chrome stampa in PDF.
//! - Markdown (.md): convertito in HTML qui, poi Chrome (o LibreOffice).
//!
//! Le pagine si disegnano con PDFKit (macOS) oppure con `pdftoppm` (Poppler).

use anyhow::{anyhow, bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const PDF_RENDER_SRC: &str = include_str!("swift/pdf_render.swift");
const PDF_TEXT_SRC: &str = include_str!("swift/pdf_text.swift");

/// Estensioni accettate (per il dialogo «Apri»).
pub const EXTENSIONS: &[&str] = &["pdf", "doc", "docx", "odt", "rtf", "txt", "md", "markdown", "html", "htm"];

/// Esegue un comando con un tempo massimo; errore con lo stderr se fallisce.
fn run(cmd: &mut Command, timeout: Duration, what: &str) -> Result<()> {
    let mut child = cmd
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("{what}: non avviato"))?;
    let start = Instant::now();
    loop {
        if let Some(st) = child.try_wait()? {
            if st.success() {
                return Ok(());
            }
            let mut err = String::new();
            if let Some(mut e) = child.stderr.take() {
                use std::io::Read;
                let _ = e.read_to_string(&mut err);
            }
            bail!("{what} non riuscito: {}", err.trim());
        }
        if start.elapsed() > timeout {
            let _ = child.kill();
            bail!("{what}: troppo tempo, interrotto");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn first_existing(candidates: &[&str]) -> Option<PathBuf> {
    candidates.iter().map(PathBuf::from).find(|p| p.exists())
}

fn in_path(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|d| d.join(name))
            .find(|p| p.is_file())
    })
}

fn soffice() -> Option<PathBuf> {
    in_path("soffice")
        .or_else(|| in_path("libreoffice"))
        .or_else(|| first_existing(&["/Applications/LibreOffice.app/Contents/MacOS/soffice"]))
}

fn chrome() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("WACOM_CHROME") {
        return Some(PathBuf::from(p));
    }
    first_existing(&[
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
    ])
    .or_else(|| in_path("google-chrome"))
    .or_else(|| in_path("chromium"))
}

fn libreoffice_to_pdf(soffice: &Path, input: &Path, out: &Path) -> Result<()> {
    let dir = out.parent().unwrap_or(Path::new("."));
    // Profilo separato: funziona anche se LibreOffice è già aperto.
    let profile = std::env::temp_dir().join("openslate-lo-profile");
    run(
        Command::new(soffice)
            .arg(format!("-env:UserInstallation=file://{}", profile.display()))
            .args(["--headless", "--convert-to", "pdf", "--outdir"])
            .arg(dir)
            .arg(input),
        Duration::from_secs(180),
        "LibreOffice",
    )?;
    let produced = dir.join(format!(
        "{}.pdf",
        input.file_stem().and_then(|s| s.to_str()).unwrap_or("documento")
    ));
    if produced != out {
        std::fs::rename(&produced, out).with_context(|| format!("PDF di LibreOffice non trovato: {}", produced.display()))?;
    }
    Ok(())
}

fn chrome_html_to_pdf(chrome: &Path, html: &Path, out: &Path) -> Result<()> {
    let html = html.canonicalize()?;
    let profile = std::env::temp_dir().join("openslate-chrome-profile");
    let mut cmd = Command::new(chrome);
    // Su Linux Chrome può rifiutarsi di partire (es. come root) senza questa opzione.
    if cfg!(target_os = "linux") {
        cmd.arg("--no-sandbox");
    }
    run(
        cmd
            .args(["--headless=new", "--disable-gpu", "--no-pdf-header-footer", "--print-to-pdf-no-header", "--no-first-run"])
            .arg(format!("--user-data-dir={}", profile.display()))
            .arg(format!("--print-to-pdf={}", out.display()))
            .arg(format!("file://{}", html.display())),
        Duration::from_secs(120),
        "Chrome (stampa in PDF)",
    )?;
    if !out.exists() {
        bail!("Chrome non ha prodotto il PDF");
    }
    Ok(())
}

const HTML_STYLE: &str = "<style>
@page { size: A4; margin: 20mm; }
body { font-family: -apple-system, 'Helvetica Neue', Helvetica, Arial, sans-serif; font-size: 12pt; line-height: 1.45; color: #111; }
h1, h2, h3 { line-height: 1.2; }
table { border-collapse: collapse; } td, th { border: 1px solid #999; padding: 4px 8px; }
pre, code { font-family: Menlo, monospace; font-size: 10pt; } pre { background: #f4f4f4; padding: 8px; white-space: pre-wrap; }
img { max-width: 100%; } blockquote { border-left: 3px solid #ccc; margin-left: 0; padding-left: 12px; color: #444; }
</style>";

fn markdown_to_html(md: &str, title: &str) -> String {
    use pulldown_cmark::{html, Options, Parser};
    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let mut body = String::new();
    html::push_html(&mut body, Parser::new_ext(md, opts));
    format!(
        "<!doctype html><html lang=\"it\"><head><meta charset=\"utf-8\"><title>{}</title>{HTML_STYLE}</head><body>{body}</body></html>",
        title.replace('<', "&lt;")
    )
}

/// Converte qualunque documento supportato in `out` (PDF). `work` è una
/// cartella per i file intermedi.
pub fn to_pdf(input: &Path, out: &Path, work: &Path) -> Result<()> {
    std::fs::create_dir_all(work)?;
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("documento").to_string();
    match ext.as_str() {
        "pdf" => {
            std::fs::copy(input, out)?;
            Ok(())
        }
        "md" | "markdown" => {
            let md = std::fs::read_to_string(input)?;
            let html = work.join(format!("{stem}.html"));
            std::fs::write(&html, markdown_to_html(&md, &stem))?;
            if let Some(c) = chrome() {
                chrome_html_to_pdf(&c, &html, out)
            } else if let Some(s) = soffice() {
                libreoffice_to_pdf(&s, &html, out)
            } else {
                bail!("per i file Markdown serve Google Chrome oppure LibreOffice (brew install --cask libreoffice)")
            }
        }
        "html" | "htm" => {
            if let Some(c) = chrome() {
                chrome_html_to_pdf(&c, input, out)
            } else if let Some(s) = soffice() {
                libreoffice_to_pdf(&s, input, out)
            } else {
                bail!("per i file HTML serve Google Chrome oppure LibreOffice")
            }
        }
        "doc" | "docx" | "odt" | "rtf" | "txt" => {
            if let Some(s) = soffice() {
                return libreoffice_to_pdf(&s, input, out);
            }
            // macOS senza LibreOffice: textutil → HTML → Chrome.
            if let (Some(tu), Some(c)) = (in_path("textutil"), chrome()) {
                let html = work.join(format!("{stem}.html"));
                run(
                    Command::new(tu).args(["-convert", "html", "-output"]).arg(&html).arg(input),
                    Duration::from_secs(60),
                    "textutil",
                )?;
                return chrome_html_to_pdf(&c, &html, out);
            }
            bail!(
                "per aprire i file Word serve LibreOffice (brew install --cask libreoffice) \
                 oppure Google Chrome. In alternativa salvalo in PDF da Word e importa il PDF."
            )
        }
        other => bail!("formato non supportato: .{other}"),
    }
}

/// Testo del PDF, una voce per pagina (vuota per le pagine scansionate).
/// macOS: PDFKit; altrove `pdftotext` (poppler) o, in mancanza, lopdf.
pub fn pdf_text(pdf: &Path, work: &Path) -> Result<Vec<String>> {
    std::fs::create_dir_all(work)?;
    let out = work.join("text-tmp.txt");
    let _ = std::fs::remove_file(&out);
    let mut errors = Vec::new();
    if cfg!(target_os = "macos") {
        match crate::swift::helper("pdf_text", PDF_TEXT_SRC)
            .and_then(|bin| run(Command::new(bin).arg(pdf).arg(&out), Duration::from_secs(120), "lettura del testo"))
        {
            Ok(()) => {}
            Err(e) => errors.push(e.to_string()),
        }
    }
    if !out.exists() {
        if let Some(p) = in_path("pdftotext") {
            match run(Command::new(p).args(["-enc", "UTF-8"]).arg(pdf).arg(&out), Duration::from_secs(120), "pdftotext") {
                Ok(()) => {}
                Err(e) => errors.push(e.to_string()),
            }
        }
    }
    if let Ok(text) = std::fs::read_to_string(&out) {
        let _ = std::fs::remove_file(&out);
        let mut pages: Vec<String> = text.split('\u{0C}').map(|p| p.trim().to_string()).collect();
        while pages.len() > 1 && pages.last().is_some_and(|p| p.is_empty()) {
            pages.pop();
        }
        return Ok(pages);
    }
    // ultima possibilità: lopdf (testo semplice, non sempre perfetto)
    let doc = lopdf::Document::load(pdf).with_context(|| format!("PDF non leggibile ({})", errors.join("; ")))?;
    let n = doc.get_pages().len() as u32;
    Ok((1..=n).map(|i| doc.extract_text(&[i]).unwrap_or_default().trim().to_string()).collect())
}

/// Disegna le pagine del PDF in `dir/page-NNNN.png` (lato lungo `long_px`).
/// Ritorna i percorsi in ordine.
pub fn render_pages(pdf: &Path, dir: &Path, long_px: u32) -> Result<Vec<PathBuf>> {
    std::fs::create_dir_all(dir)?;
    for e in std::fs::read_dir(dir)?.flatten() {
        if e.path().extension().and_then(|x| x.to_str()) == Some("png") {
            let _ = std::fs::remove_file(e.path());
        }
    }
    let mut done = false;
    let mut errors = Vec::new();
    if cfg!(target_os = "macos") {
        match crate::swift::helper("pdf_render", PDF_RENDER_SRC).and_then(|bin| {
            run(
                Command::new(bin).arg(pdf).arg(dir).arg(long_px.to_string()),
                Duration::from_secs(300),
                "disegno delle pagine",
            )
        }) {
            Ok(()) => done = true,
            Err(e) => errors.push(e.to_string()),
        }
    }
    if !done {
        if let Some(p) = in_path("pdftoppm") {
            run(
                Command::new(p)
                    .args(["-png", "-cropbox", "-scale-to"])
                    .arg(long_px.to_string())
                    .arg(pdf)
                    .arg(dir.join("page")),
                Duration::from_secs(300),
                "pdftoppm",
            )?;
            done = true;
        } else {
            errors.push("pdftoppm non trovato".into());
        }
    }
    if !done {
        bail!("non riesco a disegnare le pagine: {}", errors.join("; "));
    }
    let mut pngs: Vec<PathBuf> = std::fs::read_dir(dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("png"))
        .collect();
    // pdftoppm numera page-1, page-01… : ordina per numero.
    let n = |p: &PathBuf| -> u32 {
        p.file_stem()
            .and_then(|s| s.to_str())
            .and_then(|s| s.rsplit('-').next())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    };
    pngs.sort_by_key(n);
    if pngs.is_empty() {
        return Err(anyhow!("nessuna pagina disegnata"));
    }
    Ok(pngs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_in_html() {
        let h = markdown_to_html("# Titolo\n\n| a | b |\n|---|---|\n| 1 | 2 |\n", "x");
        assert!(h.contains("<h1>Titolo</h1>") && h.contains("<table>") && h.contains("size: A4"));
    }
}
