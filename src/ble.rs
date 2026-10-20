//! Livello BLE: ricerca del pad (con la sua modalità letta dall'annuncio),
//! connessione e scambio comando/risposta come in Tuhi.
//!
//! Usa btleplug (CoreBluetooth su macOS). Dopo la connessione un task "reader"
//! smista ogni notifica: risposte Nordic, dati LIVE, dati OFFLINE.

use crate::protocol::{self, NordicBuffer, ProtoError, Reply};
use anyhow::{anyhow, Context, Result};
use btleplug::api::{Central, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::StreamExt;
use std::io::Write;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use uuid::Uuid;

/// Nomi tipici con cui gli SmartPad Wacom si annunciano.
const NAME_HINTS: &[&str] = &["bamboo", "wacom", "slate", "folio", "spark", "smartpad"];

fn is_wacom_service(u: &Uuid) -> bool {
    *u == protocol::NORDIC_UART_SERVICE
        || *u == protocol::WACOM_LIVE_SERVICE
        || *u == protocol::WACOM_OFFLINE_SERVICE
        || *u == protocol::SYSEVENT_SERVICE
}

/// Identificativo BLE del pad scelto (file `peripheral_id` nella cartella di
/// lavoro): così riagganciamo sempre lo stesso pad, anche se rinominato.
fn load_saved_id() -> Option<String> {
    std::fs::read_to_string("peripheral_id")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn save_id(id: &str) {
    let _ = std::fs::write("peripheral_id", id);
}

/// Modalità del pad, letta dai dati produttore del suo annuncio BLE
/// (tuhi/base.py: 4 byte = abbinamento, altrimenti modalità normale).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdvMode {
    /// LED blu lampeggiante: pronto per la registrazione.
    Register,
    /// Modalità normale: ci si collega con l'identità registrata.
    Normal,
    /// L'annuncio non porta dati produttore.
    Unknown,
}

/// Un dispositivo visto durante la scansione.
pub struct Found {
    pub peripheral: Peripheral,
    pub name: String,
    pub id: String,
    pub company: Option<u16>,
    pub mfr: Option<Vec<u8>>,
    pub services: Vec<Uuid>,
}

impl Found {
    pub fn adv_mode(&self) -> AdvMode {
        match self.mfr.as_ref().map(|m| m.len()) {
            Some(protocol::MFR_DATA_LEN_REGISTER_MODE) => AdvMode::Register,
            Some(_) => AdvMode::Normal,
            None => AdvMode::Unknown,
        }
    }

    fn is_candidate(&self, saved: Option<&str>, name_filter: Option<&str>) -> bool {
        let n = self.name.to_lowercase();
        if let Some(f) = name_filter {
            return n.contains(&f.to_lowercase());
        }
        saved == Some(self.id.as_str())
            || NAME_HINTS.iter().any(|h| n.contains(h))
            || self.services.iter().any(is_wacom_service)
            || self
                .company
                .map_or(false, |c| protocol::WACOM_COMPANY_IDS.contains(&c))
    }

    pub fn describe(&self) -> String {
        format!(
            "name='{}' id={} company={} mfr={} modo={:?}",
            self.name,
            self.id,
            self.company
                .map(|c| format!("0x{c:04x}"))
                .unwrap_or_else(|| "-".into()),
            self.mfr
                .as_ref()
                .map(|m| format!("{m:02x?}"))
                .unwrap_or_else(|| "-".into()),
            self.adv_mode()
        )
    }
}

async fn describe_peripheral(p: Peripheral) -> Option<Found> {
    let props = p.properties().await.ok()??;
    // Preferisci la voce con un company ID Wacom, altrimenti la prima.
    let entry = props
        .manufacturer_data
        .iter()
        .find(|(c, _)| protocol::WACOM_COMPANY_IDS.contains(c))
        .or_else(|| props.manufacturer_data.iter().next());
    let (company, mfr) = match entry {
        Some((c, d)) => (Some(*c), Some(d.clone())),
        None => (None, None),
    };
    Some(Found {
        id: p.id().to_string(),
        name: props.local_name.unwrap_or_default(),
        company,
        mfr,
        services: props.services,
        peripheral: p,
    })
}

/// Handle verso un pad connesso.
pub struct Pad {
    peripheral: Peripheral,
    write_char: Characteristic,
    replies_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    offline_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    offline_open: bool,
    nordic: NordicBuffer,
    offline_buf: Vec<u8>,
    last_offline: Instant,
    capture: Option<std::fs::File>,
    pub name: String,
    /// In questa connessione il pad ci ha già riconosciuti (registrazione o
    /// CONNECT riuscito): un secondo CONNECT risponderebbe INVALID_STATE.
    pub authorized: bool,
}

/// Canale dei dati penna LIVE grezzi (consumato da un task dedicato).
pub type LiveStream = mpsc::UnboundedReceiver<Vec<u8>>;

impl Pad {
    /// Crea il Manager Bluetooth e il primo adapter, INSIEME.
    /// IMPORTANTE (macOS): il `Manager` va tenuto vivo per tutta la sessione,
    /// altrimenti CoreBluetooth chiude tutto ("Event receiver died").
    pub async fn manager_adapter() -> Result<(Manager, Adapter)> {
        let manager = Manager::new().await?;
        let adapters = manager.adapters().await?;
        let adapter = adapters
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("Nessun adapter Bluetooth trovato"))?;
        Ok((manager, adapter))
    }

    /// Diagnostica: tutti i dispositivi visti in `secs` secondi.
    pub async fn scan_all(adapter: &Adapter, secs: u64) -> Result<Vec<Found>> {
        adapter.start_scan(ScanFilter::default()).await?;
        tokio::time::sleep(Duration::from_secs(secs)).await;
        let _ = adapter.stop_scan().await;
        let mut out = Vec::new();
        for p in adapter.peripherals().await? {
            if let Some(f) = describe_peripheral(p).await {
                out.push(f);
            }
        }
        Ok(out)
    }

    /// Cerca il pad per al massimo `secs` secondi; si ferma appena lo vede con
    /// i dati produttore (cioè con la modalità nota).
    pub async fn find(
        adapter: &Adapter,
        name_filter: Option<&str>,
        secs: u64,
    ) -> Result<Option<Found>> {
        let saved = load_saved_id();
        adapter
            .start_scan(ScanFilter::default())
            .await
            .context("start_scan fallito")?;
        let start = Instant::now();
        let deadline = start + Duration::from_secs(secs.max(2));
        // Scansioniamo almeno 1,5 s: così l'annuncio corrente aggiorna i dati
        // eventualmente rimasti in cache da una ricerca precedente.
        let min_scan = Duration::from_millis(1500);
        let mut best: Option<Found> = None;
        while Instant::now() < deadline {
            for p in adapter.peripherals().await? {
                if let Some(f) = describe_peripheral(p).await {
                    if f.is_candidate(saved.as_deref(), name_filter)
                        && (best.is_none() || f.mfr.is_some())
                    {
                        best = Some(f);
                    }
                }
            }
            let ready = best.as_ref().map_or(false, |f| f.mfr.is_some());
            if ready && start.elapsed() >= min_scan {
                break;
            }
            tokio::time::sleep(Duration::from_millis(400)).await;
        }
        let _ = adapter.stop_scan().await;
        if let Some(f) = &best {
            tracing::info!("Pad trovato: {}", f.describe());
        }
        Ok(best)
    }

    /// Si connette al pad trovato e prepara i canali.
    pub async fn connect(found: Found) -> Result<(Pad, LiveStream)> {
        let peripheral = found.peripheral;
        save_id(&found.id);
        tracing::info!("Connessione a «{}» (id {})…", found.name, found.id);
        peripheral.connect().await.context("connect fallito")?;
        peripheral
            .discover_services()
            .await
            .context("discover_services fallito")?;

        let find = |uuid: Uuid| -> Option<Characteristic> {
            peripheral.characteristics().into_iter().find(|c| c.uuid == uuid)
        };
        let write_char = find(protocol::NORDIC_UART_WRITE)
            .ok_or_else(|| anyhow!("Caratteristica comandi (Nordic UART) assente"))?;
        let notify_char = find(protocol::NORDIC_UART_NOTIFY)
            .ok_or_else(|| anyhow!("Caratteristica risposte (Nordic UART) assente"))?;

        // Le stesse caratteristiche che usa Tuhi.
        peripheral.subscribe(&notify_char).await?;
        for (uuid, label) in [
            (protocol::WACOM_LIVE_PEN_DATA, "LIVE"),
            (protocol::WACOM_OFFLINE_PEN_DATA, "OFFLINE"),
            (protocol::SYSEVENT_CHRC, "SYSEVENT"),
        ] {
            match find(uuid) {
                Some(c) => match peripheral.subscribe(&c).await {
                    Ok(()) => tracing::info!("Sottoscritta {label}"),
                    Err(e) => tracing::warn!("Subscribe {label} fallita: {e}"),
                },
                None => tracing::warn!("Caratteristica {label} assente"),
            }
        }

        let (replies_tx, replies_rx) = mpsc::unbounded_channel();
        let (live_tx, live_rx) = mpsc::unbounded_channel();
        let (offline_tx, offline_rx) = mpsc::unbounded_channel();

        // Registro diagnostico opzionale (WACOM_CAPTURE=<file>).
        let cap_path = std::env::var("WACOM_CAPTURE").ok();
        let open_cap = |p: &str| {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(p)
                .ok()
        };
        let mut reader_cap = cap_path.as_deref().and_then(open_cap);
        let capture = cap_path.as_deref().and_then(open_cap);
        if let Some(f) = reader_cap.as_mut() {
            let _ = writeln!(f, "# connessione {} {}", found.name, found.id);
        }

        let mut stream = peripheral.notifications().await?;
        tokio::spawn(async move {
            while let Some(n) = stream.next().await {
                if let Some(f) = reader_cap.as_mut() {
                    let hex: String = n.value.iter().map(|b| format!("{b:02x}")).collect();
                    let _ = writeln!(f, "RX {} {hex}", n.uuid);
                }
                let _ = if n.uuid == protocol::NORDIC_UART_NOTIFY {
                    replies_tx.send(n.value)
                } else if n.uuid == protocol::WACOM_LIVE_PEN_DATA {
                    live_tx.send(n.value)
                } else if n.uuid == protocol::WACOM_OFFLINE_PEN_DATA {
                    offline_tx.send(n.value)
                } else {
                    Ok(()) // SYSEVENT: solo registrato
                };
            }
            tracing::info!("Pad disconnesso (fine notifiche)");
        });

        let pad = Pad {
            peripheral,
            write_char,
            replies_rx,
            offline_rx,
            offline_open: true,
            nordic: NordicBuffer::default(),
            offline_buf: Vec::new(),
            last_offline: Instant::now(),
            capture,
            name: found.name,
            authorized: false,
        };
        Ok((pad, live_rx))
    }

    /// Invia un comando senza attendere risposta.
    pub async fn send(&mut self, opcode: u8, args: &[u8]) -> Result<(), ProtoError> {
        let f = protocol::frame(opcode, args);
        tracing::info!("TX {f:02x?}");
        if let Some(c) = self.capture.as_mut() {
            let hex: String = f.iter().map(|b| format!("{b:02x}")).collect();
            let _ = writeln!(c, "TX {hex}");
        }
        self.peripheral
            .write(&self.write_char, &f, WriteType::WithoutResponse)
            .await
            .map_err(|e| ProtoError::Link(e.to_string()))
    }

    /// Prossimo messaggio completo dal flusso Nordic. Intanto raccoglie i dati
    /// offline che arrivano sull'altra caratteristica.
    pub async fn recv(&mut self, for_op: u8, timeout: Duration) -> Result<Reply, ProtoError> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            if let Some(r) = self.nordic.pop() {
                tracing::info!("RX {:02x} {:02x?}", r.opcode, r.data);
                return Ok(r);
            }
            tokio::select! {
                m = self.replies_rx.recv() => match m {
                    Some(bytes) => self.nordic.push(&bytes),
                    None => return Err(ProtoError::Link("il pad si è disconnesso".into())),
                },
                m = self.offline_rx.recv(), if self.offline_open => match m {
                    Some(bytes) => {
                        self.offline_buf.extend_from_slice(&bytes);
                        self.last_offline = Instant::now();
                    }
                    None => self.offline_open = false,
                },
                _ = tokio::time::sleep_until(deadline) => {
                    return Err(ProtoError::MissingReply(for_op));
                }
            }
        }
    }

    /// Esegue un comando e verifica la risposta (Tuhi: `Msg.execute`).
    pub async fn execute(
        &mut self,
        opcode: u8,
        args: &[u8],
        expect: u8,
    ) -> Result<Reply, ProtoError> {
        self.flush_stale();
        self.send(opcode, args).await?;
        self.wait(opcode, expect, Duration::from_secs(5)).await
    }

    /// Attende (senza inviare nulla) una risposta e la verifica.
    pub async fn wait(
        &mut self,
        for_op: u8,
        expect: u8,
        timeout: Duration,
    ) -> Result<Reply, ProtoError> {
        let r = self.recv(for_op, timeout).await?;
        check_reply(r, expect)
    }

    /// Scarta messaggi arrivati senza essere richiesti (es. `cb`, o risposte in
    /// ritardo a un comando precedente): altrimenti verrebbero scambiati per la
    /// risposta al comando successivo.
    fn flush_stale(&mut self) {
        while let Ok(bytes) = self.replies_rx.try_recv() {
            self.nordic.push(&bytes);
        }
        while let Some(r) = self.nordic.pop() {
            tracing::info!("RX ignorata (non richiesta) {:02x} {:02x?}", r.opcode, r.data);
        }
    }

    /// Dati offline raccolti finora (e svuota il buffer).
    pub fn take_offline(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.offline_buf)
    }

    /// Istante dell'ultimo blocco di dati offline ricevuto.
    pub fn last_offline(&self) -> Instant {
        self.last_offline
    }

    pub async fn disconnect(&self) {
        let _ = self.peripheral.disconnect().await;
    }
}

/// Verifica una risposta come Tuhi: `b3` porta un codice (0x00 = ok), ogni
/// altro opcode deve essere quello atteso.
pub fn check_reply(r: Reply, expect: u8) -> Result<Reply, ProtoError> {
    if r.opcode == protocol::op::R_GENERIC {
        match r.data.first() {
            Some(0x00) => Ok(r),
            Some(&code) => Err(ProtoError::Device(code)),
            None => Err(ProtoError::UnexpectedData("risposta b3 vuota".into())),
        }
    } else if r.opcode != expect {
        Err(ProtoError::UnexpectedReply {
            expected: expect,
            got: r.opcode,
            data: r.data,
        })
    } else {
        Ok(r)
    }
}
