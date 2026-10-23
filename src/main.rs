//! OpenSlate — app libera per Wacom Bamboo Slate (SmartPad).
//! Rimpiazza le funzioni di Inkspace: live mode, sync offline, export.
//! Il protocollo è una traduzione fedele di Tuhi (GPL).

#![allow(dead_code)]

mod actor;
mod ai;
mod aiconfig;
mod ble;
mod charts;
mod boardagent;
mod boardmarks;
mod beautify;
mod boardread;
mod chats;
mod craftexport;
mod device;
mod docimport;
mod docrender;
mod docagent;
mod docxfonts;
mod document;
mod export;
mod figures;
mod i18n;
mod libagent;
mod library;
mod mcpclient;
mod mathtex;
mod mathhand;
mod rewrite;
mod typstworld;
mod mdcraft;
mod live;
mod handwrite;
mod ocr;
mod offline;
mod pdfdoc;
mod platform;
mod protocol;
mod scene;
mod selfupdate;
mod settings;
mod signature;
mod stroke;
mod swift;
mod tools;
mod ui;
mod update;
mod rasign;
mod vecsign;
#[cfg(test)]
mod demo;
mod fonts;
mod formats;
mod mdconv;
mod video;
mod voice;
mod speech;
mod vendor;
mod viewer;
mod wordsign;
mod web;

use anyhow::{bail, Result};
use ble::{AdvMode, Pad};
use clap::{Parser, Subcommand};
use std::io::Write;

#[derive(Parser)]
#[command(name = "openslate", version, about = "OpenSlate: app libera per Wacom Bamboo Slate")]
struct Cli {
    /// Filtro sul nome del pad (sottostringa). Default: pad salvato o primo Wacom.
    #[arg(long, global = true)]
    name: Option<String>,

    /// Secondi massimi di scansione BLE.
    #[arg(long, global = true, default_value_t = 8)]
    scan_secs: u64,

    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Interfaccia grafica (live a schermo). [default]
    Gui {
        /// Avvia subito la diretta web.
        #[arg(long)]
        web: bool,
        /// Disegna da solo qualche tratto di prova (senza pad).
        #[arg(long)]
        demo: bool,
        /// Apre subito un documento da firmare (PDF, Word, Markdown, SVG, PNG…).
        #[arg(long)]
        apri: Option<std::path::PathBuf>,
    },
    /// Elenca i dispositivi visibili con la modalità del pad (abbinamento/normale).
    Scan,
    /// Connette e stampa nome, firmware, batteria, dimensioni.
    Info,
    /// Registra questa app sul pad (LED blu lampeggiante, poi premi il pulsante).
    Register,
    /// Live da terminale: stampa i punti della penna (--raw per i byte grezzi).
    Live {
        #[arg(long)]
        raw: bool,
    },
    /// Scarica i disegni dalla memoria del pad in exports/ (PDF + copia grezza).
    Sync {
        /// Lascia i disegni sul pad (così se ne scarica solo il più vecchio).
        #[arg(long)]
        keep: bool,
    },
    /// Converte in PDF disegni grezzi già scaricati (exports/pad-raw/*.bin).
    Pdf {
        files: Vec<std::path::PathBuf>,
    },
    /// Importa un documento (PDF, Word, Markdown, SVG, PNG…) per scriverci e firmarlo.
    Importa {
        file: std::path::PathBuf,
    },
    /// Agenti AI di ai.toml: stato (e con --prova una domanda a ciascuno).
    Ai {
        #[arg(long)]
        prova: bool,
    },
    /// Finestra con una pagina del server web (usata dall'app per Appunti e Chat).
    #[command(hide = true)]
    Finestra {
        #[arg(long, default_value = "Appunti")]
        titolo: String,
        url: String,
    },
    /// Trascrive un'immagine (PNG/JPG scritta a mano) in Markdown.
    Trascrivi {
        file: std::path::PathBuf,
    },
}

/// Stato degli agenti AI; con `prova` manda a ciascuno una domanda breve.
fn ai_status(prova: bool) -> Result<()> {
    let cfg = ai::AiConfig::load()?;
    println!("Configurazione: {} (si rilegge a ogni richiesta)\n", ai::config_path());
    for (i, (name, state)) in ai::describe(&cfg).into_iter().enumerate() {
        println!("{}. {name}: {state}", i + 1);
    }
    if !prova {
        println!("\nPer provarli uno per uno: openslate ai --prova");
        return Ok(());
    }
    println!();
    for p in cfg.providers.iter().filter(|p| p.enabled) {
        let one = ai::AiConfig { providers: vec![p.clone()], ..cfg.clone() };
        let msgs = [ai::Msg::new("user", "Rispondi solo con: OK")];
        print!("{}… ", p.name);
        std::io::stdout().flush().ok();
        let t = std::time::Instant::now();
        match ai::complete(&one, ai::Task::Chat, &msgs) {
            Ok(r) => println!("risponde ({}, {:.1} s): {}", r.model, t.elapsed().as_secs_f32(), r.text.lines().next().unwrap_or("")),
            Err(e) => println!("{}", e.to_string().replace("nessun agente AI disponibile:\n• ", "")),
        }
    }
    Ok(())
}

/// Converte un'immagine in PNG (JPG, HEIC… con `sips` di macOS).
fn to_png(file: &std::path::Path) -> Result<Vec<u8>> {
    let out = std::env::temp_dir().join("wacom-trascrivi.png");
    let ok = std::process::Command::new("sips")
        .args(["-s", "format", "png"])
        .arg(file)
        .arg("--out")
        .arg(&out)
        .output()
        .is_ok_and(|o| o.status.success());
    if !ok {
        bail!("{}: serve un PNG (o macOS con sips per convertirlo)", file.display());
    }
    Ok(std::fs::read(&out)?)
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cli = Cli::parse();
    let name = cli.name.clone();
    let scan_secs = cli.scan_secs;

    match cli.cmd.unwrap_or(Cmd::Gui { web: false, demo: false, apri: None }) {
        Cmd::Gui { web, demo, apri } => {
            // il documento da aprire va preso prima di cambiare cartella
            let apri = apri.map(|p| std::fs::canonicalize(&p).unwrap_or(p));
            platform::enter_data_dir();
            ui::run_app(name, scan_secs, web, demo, apri)
        }
        Cmd::Pdf { files } => raw_to_pdf(&files),
        Cmd::Importa { file } => {
            let doc = document::Document::import(&file)?;
            println!("Importato «{}»: {} pagine in {}", doc.title, doc.pages.len(), doc.dir.display());
            for (i, p) in doc.pages.iter().enumerate() {
                let (w, h) = p.pdf.display_size();
                println!("  pagina {}: {:.0} x {:.0} pt, rotazione {}°", i + 1, w, h, p.pdf.rotate);
            }
            Ok(())
        }
        Cmd::Ai { prova } => ai_status(prova),
        Cmd::Finestra { titolo, url } => {
            i18n::set_lang(i18n::resolve(&settings::Settings::load().ui_lang));
            viewer::run(&url, &titolo)
        }
        Cmd::Trascrivi { file } => {
            let bytes = std::fs::read(&file)?;
            let png = if bytes.starts_with(b"\x89PNG") { bytes } else { to_png(&file)? };
            let r = ai::transcribe(std::sync::Arc::new(png))?;
            eprintln!("(trascritto da {} · {})", r.provider, r.model);
            println!("{}", r.text);
            Ok(())
        }
        other => {
            let _ = std::fs::create_dir_all("captures");
            if std::env::var("WACOM_CAPTURE").is_err() {
                std::env::set_var("WACOM_CAPTURE", "captures/cli.log");
            }
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            rt.block_on(run_cli(other, name, scan_secs))
        }
    }
}

async fn run_cli(cmd: Cmd, name: Option<String>, scan_secs: u64) -> Result<()> {
    // Il Manager va tenuto vivo per tutta la sessione (vedi nota in ble.rs).
    let (_manager, adapter) = Pad::manager_adapter().await?;

    if let Cmd::Scan = cmd {
        println!("Scansione per {scan_secs} s…");
        let found = Pad::scan_all(&adapter, scan_secs).await?;
        let mut log = std::fs::File::create("captures/scan.log")?;
        for f in &found {
            let line = format!("{}  [modalità: {:?}]", f.describe(), f.adv_mode());
            println!("{line}");
            writeln!(log, "{line}")?;
        }
        println!("{} dispositivi (salvato in captures/scan.log)", found.len());
        return Ok(());
    }

    let Some(found) = Pad::find(&adapter, name.as_deref(), scan_secs).await? else {
        bail!(
            "Pad non trovato. Premi brevemente il pulsante del pad e riprova; \
             per registrarlo tienilo premuto ~6 s (LED blu lampeggiante)."
        );
    };
    let adv = found.adv_mode();
    println!("Trovato: {}  [modalità: {adv:?}]", found.describe());

    match cmd {
        Cmd::Register => {
            if adv != AdvMode::Register {
                println!(
                    "Attenzione: il pad non sembra in abbinamento ({adv:?}). \
                     Tieni premuto ~6 s finché il LED lampeggia blu. Provo comunque."
                );
            }
            let (mut pad, _live) = Pad::connect(found).await?;
            println!(">>> PREMI ORA una volta il pulsante del pad (30 s) <<<");
            let res = device::register(&mut pad).await;
            pad.disconnect().await;
            let (_, info) = res?;
            println!("Registrato ✓  {info:?}");
        }
        Cmd::Info => {
            let uuid = need_uuid(adv)?;
            let (mut pad, _live) = Pad::connect(found).await?;
            let res = async {
                let (w, h) = device::update_dimensions(&mut pad).await?;
                device::check_connection(&mut pad, &uuid).await?;
                let name = device::get_name(&mut pad).await?;
                let fw = device::get_firmware(&mut pad).await?;
                let (pct, chg) = device::get_battery(&mut pad).await?;
                Ok::<_, protocol::ProtoError>((name, fw, pct, chg, w, h))
            }
            .await;
            pad.disconnect().await;
            let (name, fw, pct, chg, w, h) = res?;
            println!("Nome:       {name}");
            println!("Firmware:   {fw}");
            println!("Batteria:   {pct}%{}", if chg { " (in carica)" } else { "" });
            println!("Dimensioni: {w} x {h} punti");
        }
        Cmd::Live { raw } => {
            let uuid = need_uuid(adv)?;
            let (mut pad, mut live_rx) = Pad::connect(found).await?;
            let info = match device::start_live(&mut pad, &uuid).await {
                Ok(i) => i,
                Err(e) => {
                    pad.disconnect().await;
                    return Err(e.into());
                }
            };
            println!(
                "Live attivo ({} x {}). Scrivi sul pad, Ctrl+C per uscire.",
                info.width, info.height
            );
            let mut dec = live::LiveDecoder::new();
            loop {
                tokio::select! {
                    pkt = live_rx.recv() => {
                        let Some(pkt) = pkt else { break };
                        if raw {
                            println!("{pkt:02x?}");
                        } else {
                            for ev in dec.decode(&pkt) {
                                println!("{:?} x={:.0} y={:.0} p={:.2}",
                                    ev.phase, ev.x, ev.y, ev.pressure);
                            }
                        }
                    }
                    _ = tokio::signal::ctrl_c() => break,
                }
            }
            let _ = device::stop_live(&mut pad).await;
            pad.disconnect().await;
        }
        Cmd::Sync { keep } => {
            let uuid = need_uuid(adv)?;
            let (mut pad, _live) = Pad::connect(found).await?;
            // la stessa copia di sicurezza dell'app (exports/pad-raw/): le
            // pagine finiscono nell'archivio al prossimo avvio
            let res = device::sync_offline(&mut pad, &uuid, !keep, |f| actor::save_raw(f).is_some())
            .await;
            pad.disconnect().await;
            let res = res?;
            println!(
                "Disegni nel pad: {}, scaricati: {}, cancellati: {}",
                res.files,
                res.downloaded.len(),
                res.deleted
            );
            for f in &res.downloaded {
                match offline::parse(&f.raw) {
                    Ok(raw) => {
                        let d = offline::to_drawing(&raw, 0.0, 0.0, [20, 20, 20], 2.0);
                        let page = scene::Page::from_drawing(&d, view_from_settings());
                        let path = export::new_path("pad", "pdf");
                        export::save_pdf(&[&page], &path)?;
                        println!("{} ({} tratti){}", path.display(), raw.len(),
                            if f.crc_ok { "" } else { "  [CRC non valido]" });
                    }
                    Err(e) => println!("disegno non decodificato: {e}"),
                }
            }
        }
        Cmd::Gui { .. } | Cmd::Scan | Cmd::Pdf { .. } | Cmd::Importa { .. } | Cmd::Ai { .. } | Cmd::Trascrivi { .. } | Cmd::Finestra { .. } => unreachable!(),
    }
    Ok(())
}

/// Vista (rotazione) salvata nelle impostazioni dell'interfaccia.
fn view_from_settings() -> scene::View {
    let s = settings::Settings::load();
    scene::View { rot: s.rot, flip_x: s.flip_x, flip_y: s.flip_y }
}

/// `pdf`: converte file grezzi del pad in un unico PDF.
fn raw_to_pdf(files: &[std::path::PathBuf]) -> Result<()> {
    if files.is_empty() {
        bail!("indica uno o più file .bin (es. exports/pad-raw/*.bin)");
    }
    let view = view_from_settings();
    let mut pages = Vec::new();
    for f in files {
        let data = std::fs::read(f)?;
        match offline::parse(&data) {
            Ok(raw) => {
                println!("{}: {} tratti", f.display(), raw.len());
                let d = offline::to_drawing(&raw, 0.0, 0.0, [20, 20, 20], 2.0);
                pages.push(scene::Page::from_drawing(&d, view));
            }
            Err(e) => println!("{}: {e}", f.display()),
        }
    }
    if pages.is_empty() {
        bail!("nessun disegno decodificato");
    }
    let refs: Vec<&scene::Page> = pages.iter().collect();
    let path = export::new_path("pad", "pdf");
    export::save_pdf(&refs, &path)?;
    println!("Salvato {}", path.display());
    Ok(())
}

/// Identità registrata, necessaria per CONNECT.
fn need_uuid(adv: AdvMode) -> Result<[u8; 6]> {
    if adv == AdvMode::Register {
        bail!(
            "Il pad è in abbinamento (LED blu): usa `register`, oppure premi una volta \
             il pulsante per tornare a verde."
        );
    }
    device::load_uuid().ok_or_else(|| {
        anyhow::anyhow!("App non registrata: tieni premuto ~6 s (LED blu) e lancia `register`.")
    })
}
