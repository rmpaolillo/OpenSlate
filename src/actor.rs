//! "Actor" Bluetooth: possiede il collegamento col pad e processa i comandi
//! dell'interfaccia. I punti live vanno direttamente nello `Hub` condiviso.

use crate::ble::{AdvMode, LiveStream, Pad};
use crate::device::{self, DeviceInfo, OfflineFile};
use crate::export::EXPORT_DIR;
use crate::live::LiveDecoder;
use crate::offline;
use crate::protocol::{mode, ProtoError};
use crate::scene::Hub;
use crate::stroke::Drawing;
use anyhow::Result;
use btleplug::platform::Adapter;
use std::sync::Arc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

/// Comandi dalla UI verso l'actor.
pub enum UiCmd {
    Connect,
    StartLive,
    StopLive,
    /// Scarica i disegni dalla memoria del pad.
    Sync { delete: bool },
    /// Conta le pagine ancora nel pad (se connesso).
    CountFiles,
}

/// Un disegno scaricato dal pad, già decodificato.
pub struct PadDrawing {
    /// Data del disegno (ora locale).
    pub created: chrono::NaiveDateTime,
    /// Nome della copia grezza in exports/pad-raw.
    pub raw: Option<String>,
    pub drawing: Drawing,
}

/// Scrittura riconosciuta (una o più pagine).
pub struct OcrDone {
    pub text: String,
    /// Markdown + LaTeX (agenti AI) oppure testo semplice (Apple Vision).
    pub markdown: bool,
    /// Chi ha trascritto.
    pub by: String,
    pub title: String,
    /// Immagine della (prima) pagina, per la chat.
    pub png: Option<Arc<Vec<u8>>>,
}

/// Collegamento col pad.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Link {
    Off,
    Connecting,
    On,
}

/// Cresce a ogni nuova connessione: la fine di un collegamento vecchio non
/// deve spegnere la spia di quello nuovo.
static LINK_GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Eventi verso la UI.
pub enum UiEvent {
    Status(String),
    Error(String),
    Info(DeviceInfo),
    Live(bool),
    Synced {
        files: usize,
        deleted: usize,
        drawings: Vec<PadDrawing>,
    },
    /// Pagine ancora nel pad.
    PadFiles(usize),
    Ocr(Result<OcrDone, String>),
    /// Stato del collegamento col pad (spia nella barra).
    Link(Link),
    Web(Result<String, String>),
    /// Risposta di GitHub sugli aggiornamenti (`manual`: chiesto dall'utente).
    Update { result: Result<Option<crate::update::Release>, String>, manual: bool },
    /// Aggiornamento dall'app: a che punto è, poi com'è finito.
    UpdatePhase(crate::selfupdate::Phase),
    UpdateDone(Result<crate::selfupdate::Ready, String>),
    /// La chat AI ha modificato il documento in questa cartella.
    DocChanged(std::path::PathBuf),
}

pub async fn ble_actor(
    mut cmd_rx: UnboundedReceiver<UiCmd>,
    evt_tx: UnboundedSender<UiEvent>,
    hub: Arc<Hub>,
    name: Option<String>,
    scan_secs: u64,
) {
    let mut pad: Option<Pad> = None;

    // Manager tenuto vivo per tutta la sessione (vedi nota in ble.rs).
    let (_manager, adapter) = match Pad::manager_adapter().await {
        Ok(x) => x,
        Err(e) => {
            err(&evt_tx, format!("Bluetooth non disponibile: {e}"));
            return;
        }
    };

    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            UiCmd::Connect => {
                LINK_GEN.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let _ = evt_tx.send(UiEvent::Link(Link::Connecting));
                if let Some(p) = pad.take() {
                    p.disconnect().await;
                }
                let _ = evt_tx.send(UiEvent::Live(false));
                match connect_flow(&adapter, scan_secs, name.as_deref(), &evt_tx, &hub).await {
                    Ok(p) => {
                        let _ = evt_tx.send(UiEvent::Link(if p.is_some() { Link::On } else { Link::Off }));
                        pad = p;
                    }
                    Err(e) => {
                        let _ = evt_tx.send(UiEvent::Link(Link::Off));
                        err(&evt_tx, e.to_string());
                    }
                }
            }
            UiCmd::StartLive => match pad.as_mut() {
                Some(p) => match device::set_mode(p, mode::LIVE).await {
                    Ok(()) => {
                        let _ = evt_tx.send(UiEvent::Live(true));
                        status(&evt_tx, "Live attivo — scrivi sul pad.");
                    }
                    Err(e) => err(&evt_tx, explain(&e)),
                },
                None => err(&evt_tx, "Non connesso: premi «Collega».".into()),
            },
            UiCmd::StopLive => {
                if let Some(p) = pad.as_mut() {
                    match device::stop_live(p).await {
                        Ok(()) => {
                            let _ = evt_tx.send(UiEvent::Live(false));
                            status(&evt_tx, "Live fermato.");
                        }
                        Err(e) => err(&evt_tx, explain(&e)),
                    }
                }
            }
            UiCmd::Sync { delete } => match (pad.as_mut(), device::load_uuid()) {
                (Some(p), Some(uuid)) => {
                    status(&evt_tx, "Scarico i disegni dal pad…");
                    let mut raw_names: Vec<Option<String>> = Vec::new();
                    let res = device::sync_offline(p, &uuid, delete, |f| {
                        let r = save_raw(f);
                        let ok = r.is_some();
                        raw_names.push(r);
                        ok
                    })
                    .await;
                    // Il pad ora è in modalità "carta": rimettiamo il live.
                    let live = device::set_mode(p, mode::LIVE).await.is_ok();
                    let _ = evt_tx.send(UiEvent::Live(live));
                    match res {
                        Ok(res) => {
                            let (w, h) = {
                                let s = hub.lock();
                                (s.drawing.width, s.drawing.height)
                            };
                            let pen_w = hub.lock().pen.width;
                            let drawings = res
                                .downloaded
                                .iter()
                                .enumerate()
                                .filter_map(|(i, f)| match offline::parse(&f.raw) {
                                    Ok(raw) => Some(PadDrawing {
                                        created: offline::bcd_time_local(&f.time_bcd)
                                            .unwrap_or_else(|| chrono::Local::now().naive_local()),
                                        raw: raw_names.get(i).cloned().flatten(),
                                        drawing: offline::to_drawing(&raw, w, h, [20, 20, 20], pen_w),
                                    }),
                                    Err(e) => {
                                        tracing::warn!("disegno {i} non decodificato: {e}");
                                        None
                                    }
                                })
                                .collect();
                            let _ = evt_tx.send(UiEvent::Synced {
                                files: res.files,
                                deleted: res.deleted,
                                drawings,
                            });
                        }
                        Err(e) => err(&evt_tx, explain(&e)),
                    }
                }
                _ => err(&evt_tx, "Prima connetti il pad.".into()),
            },
            UiCmd::CountFiles => {
                if let Some(p) = pad.as_mut() {
                    match device::count_files(p).await {
                        Ok(n) => {
                            let _ = evt_tx.send(UiEvent::PadFiles(n));
                        }
                        Err(e) => tracing::warn!("conteggio pagine sul pad: {e}"),
                    }
                }
            }
        }
    }
}

/// Copia di sicurezza dei byte grezzi, PRIMA di qualunque cancellazione.
/// Ritorna il nome del file salvato.
fn save_raw(f: &OfflineFile) -> Option<String> {
    let dir = format!("{EXPORT_DIR}/pad-raw");
    if std::fs::create_dir_all(&dir).is_err() {
        return None;
    }
    let label = offline::bcd_time_label(&f.time_bcd)
        .unwrap_or_else(|| chrono::Local::now().format("%Y%m%d-%H%M%S%3f").to_string());
    let mut path = format!("{dir}/pad-{label}.bin");
    let mut n = 2;
    while std::path::Path::new(&path).exists() {
        path = format!("{dir}/pad-{label}-{n}.bin");
        n += 1;
    }
    let ok = std::fs::write(&path, &f.raw).is_ok()
        && std::fs::metadata(&path).map(|m| m.len() as usize == f.raw.len()).unwrap_or(false);
    if !ok {
        return None;
    }
    tracing::info!("Salvato {path} ({} byte)", f.raw.len());
    std::path::Path::new(&path)
        .file_name()
        .and_then(|n| n.to_str())
        .map(String::from)
}

fn status(tx: &UnboundedSender<UiEvent>, s: &str) {
    let _ = tx.send(UiEvent::Status(s.to_string()));
}

fn err(tx: &UnboundedSender<UiEvent>, s: String) {
    let _ = tx.send(UiEvent::Error(s));
}

/// Traduce un errore di protocollo in istruzioni pratiche.
fn explain(e: &ProtoError) -> String {
    match e {
        ProtoError::Authorization => "Il pad non riconosce questa app (forse è stato \
            registrato con Inkspace). Tieni premuto il pulsante ~6 s finché il LED \
            lampeggia blu, poi premi «Collega» per registrarlo di nuovo."
            .into(),
        ProtoError::Device(0x02) => "Il pad è in uno stato non valido (INVALID_STATE). \
            Con il LED blu, premi una volta il pulsante per riportarlo a verde, poi \
            premi «Collega»."
            .into(),
        other => other.to_string(),
    }
}

/// Decodifica i pacchetti live e li mette nella lavagna condivisa. Quando il
/// flusso finisce (pad spento, fuori portata) la spia diventa rossa.
fn spawn_live_forwarder(mut live_rx: LiveStream, hub: Arc<Hub>, evt_tx: UnboundedSender<UiEvent>) {
    let gen = LINK_GEN.load(std::sync::atomic::Ordering::SeqCst);
    tokio::spawn(async move {
        let mut dec = LiveDecoder::new();
        while let Some(raw) = live_rx.recv().await {
            for ev in dec.decode(&raw) {
                hub.pen_event(ev);
            }
        }
        if LINK_GEN.load(std::sync::atomic::Ordering::SeqCst) == gen {
            let _ = evt_tx.send(UiEvent::Link(Link::Off));
            let _ = evt_tx.send(UiEvent::Live(false));
        }
    });
}

fn send_info(evt_tx: &UnboundedSender<UiEvent>, hub: &Hub, info: DeviceInfo) {
    if info.width > 0.0 && info.height > 0.0 {
        hub.edit(|s| {
            s.drawing.width = info.width;
            s.drawing.height = info.height;
        });
    }
    let _ = evt_tx.send(UiEvent::Info(info));
}

/// Connessione completa, decisa dalla modalità del pad, come in Tuhi:
/// - abbinamento (LED blu lampeggiante): registrazione, poi live;
/// - modalità normale: CONNECT con l'identità registrata, poi live.
///
/// `Ok(None)`: nessuna connessione utile, le istruzioni sono già nella barra.
async fn connect_flow(
    adapter: &Adapter,
    scan_secs: u64,
    name: Option<&str>,
    evt_tx: &UnboundedSender<UiEvent>,
    hub: &Arc<Hub>,
) -> Result<Option<Pad>> {
    status(evt_tx, "Cerco il pad…");
    let Some(found) = Pad::find(adapter, name, scan_secs).await? else {
        status(
            evt_tx,
            "Non trovo il pad. Premi brevemente il suo pulsante e subito dopo «Collega». \
             Per registrarlo: tieni premuto ~6 s finché il LED lampeggia blu.",
        );
        return Ok(None);
    };

    let adv = found.adv_mode();
    let uuid = device::load_uuid();
    tracing::info!("Modalità pad: {adv:?}, identità salvata: {}", uuid.is_some());
    if adv == AdvMode::Normal && uuid.is_none() {
        status(
            evt_tx,
            "Questa app non è ancora registrata sul pad. Tieni premuto il pulsante \
             ~6 s finché il LED lampeggia blu, poi premi «Collega».",
        );
        return Ok(None);
    }
    let must_register = adv == AdvMode::Register || uuid.is_none();

    let (mut pad, live_rx) = Pad::connect(found).await?;
    spawn_live_forwarder(live_rx, hub.clone(), evt_tx.clone());

    if must_register {
        status(evt_tx, ">>> Registrazione: PREMI ORA una volta il pulsante del pad <<<");
        let info = match device::register(&mut pad).await {
            Ok((_, info)) => info,
            Err(e) => {
                pad.disconnect().await;
                anyhow::bail!("Registrazione non riuscita: {}", explain(&e));
            }
        };
        send_info(evt_tx, hub, info);
        // Nella stessa sessione il live parte con il solo SET_MODE (verificato
        // sul tuo Slate); se non va, basta una nuova connessione normale.
        match device::set_mode(&mut pad, mode::LIVE).await {
            Ok(()) => {
                let _ = evt_tx.send(UiEvent::Live(true));
                status(evt_tx, "Registrato ✓  Live attivo — scrivi sul pad.");
                Ok(Some(pad))
            }
            Err(e) => {
                tracing::warn!("live subito dopo la registrazione: {e}");
                pad.disconnect().await;
                status(
                    evt_tx,
                    "Registrato ✓  Ora premi brevemente il pulsante del pad e poi «Collega».",
                );
                Ok(None)
            }
        }
    } else {
        let uuid = uuid.expect("identità presente");
        match device::start_live(&mut pad, &uuid).await {
            Ok(info) => {
                send_info(evt_tx, hub, info);
                let _ = evt_tx.send(UiEvent::Live(true));
                status(evt_tx, "Connesso. Live attivo — scrivi sul pad.");
                Ok(Some(pad))
            }
            Err(e) => {
                pad.disconnect().await;
                anyhow::bail!("{}", explain(&e));
            }
        }
    }
}
