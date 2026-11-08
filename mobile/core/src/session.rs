//! Sessione con il pad senza I/O: le sequenze di `src/device.rs` (registrazione
//! e avvio del live, come in Tuhi) riscritte come macchina a stati. Kotlin
//! passa i byte ricevuti e scrive quelli restituiti nello `Step`.

use crate::live::LiveDecoder;
use crate::protocol::{self, mode, op, NordicBuffer, ProtoError, Reply};
use crate::stroke::PenPhase;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Attesa normale di una risposta (come `Pad::execute` della desktop).
const REPLY_MS: u32 = 5_000;
/// Attesa della pressione del pulsante in registrazione (come `device::register`).
const BUTTON_MS: u32 = 30_000;

// ---------------------------------------------------------------------------
// Tipi esposti a Kotlin
// ---------------------------------------------------------------------------

/// UUID GATT del pad, come stringhe per `java.util.UUID.fromString`.
#[derive(uniffi::Record)]
pub struct GattUuids {
    pub nordic_service: String,
    pub nordic_write: String,
    pub nordic_notify: String,
    pub live_pen_data: String,
    pub offline_pen_data: String,
    pub sysevent: String,
}

#[uniffi::export]
pub fn gatt_uuids() -> GattUuids {
    GattUuids {
        nordic_service: protocol::NORDIC_UART_SERVICE.to_string(),
        nordic_write: protocol::NORDIC_UART_WRITE.to_string(),
        nordic_notify: protocol::NORDIC_UART_NOTIFY.to_string(),
        live_pen_data: protocol::WACOM_LIVE_PEN_DATA.to_string(),
        offline_pen_data: protocol::WACOM_OFFLINE_PEN_DATA.to_string(),
        sysevent: protocol::SYSEVENT_CHRC.to_string(),
    }
}

/// Una voce dei dati produttore di un annuncio BLE.
#[derive(uniffi::Record)]
pub struct MfrEntry {
    pub company: u16,
    pub len: u32,
}

/// Modalità del pad letta dall'annuncio (come `ble::AdvMode`).
#[derive(uniffi::Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdvMode {
    /// LED blu lampeggiante: pronto per la registrazione.
    Register,
    /// Modalità normale: ci si collega con l'identità registrata.
    Normal,
    /// L'annuncio non porta dati produttore.
    Unknown,
}

#[derive(uniffi::Record, Debug)]
pub struct Advert {
    /// Sembra uno SmartPad Wacom.
    pub candidate: bool,
    pub mode: AdvMode,
}

/// Nomi tipici con cui gli SmartPad Wacom si annunciano (come `ble.rs`).
const NAME_HINTS: &[&str] = &["bamboo", "wacom", "slate", "folio", "spark", "smartpad"];

/// Classifica un annuncio BLE come `Found::is_candidate`/`adv_mode` della
/// desktop. `is_saved`: è il pad usato l'ultima volta.
#[uniffi::export]
pub fn classify_advert(name: String, mfr: Vec<MfrEntry>, services: Vec<String>, is_saved: bool) -> Advert {
    let wacom_services = [
        protocol::NORDIC_UART_SERVICE,
        protocol::WACOM_LIVE_SERVICE,
        protocol::WACOM_OFFLINE_SERVICE,
        protocol::SYSEVENT_SERVICE,
    ];
    // Preferisci la voce con un company ID Wacom, altrimenti la prima.
    let entry = mfr
        .iter()
        .find(|e| protocol::WACOM_COMPANY_IDS.contains(&e.company))
        .or_else(|| mfr.first());
    let n = name.to_lowercase();
    let candidate = is_saved
        || NAME_HINTS.iter().any(|h| n.contains(h))
        || services.iter().any(|s| {
            uuid::Uuid::parse_str(s).map_or(false, |u| wacom_services.contains(&u))
        })
        || entry.map_or(false, |e| protocol::WACOM_COMPANY_IDS.contains(&e.company));
    let mode = match entry.map(|e| e.len as usize) {
        Some(protocol::MFR_DATA_LEN_REGISTER_MODE) => AdvMode::Register,
        Some(_) => AdvMode::Normal,
        None => AdvMode::Unknown,
    };
    Advert { candidate, mode }
}

/// Informazioni lette dal pad (come `device::DeviceInfo`).
#[derive(uniffi::Record, Clone, Debug, Default, PartialEq)]
pub struct PadInfo {
    pub name: String,
    pub firmware: String,
    pub battery_pct: Option<u8>,
    pub charging: bool,
    /// Area attiva in punti del pad (stesse unità delle coordinate live).
    pub width: f32,
    pub height: f32,
}

#[derive(uniffi::Enum, Clone, Debug, PartialEq)]
pub enum SessionEvent {
    /// Il pulsante è stato premuto: il pad conosce questo UUID, da salvare.
    Registered { uuid: Vec<u8> },
    /// Il live è attivo: si può scrivere.
    Live { info: PadInfo },
    /// La sequenza si è fermata.
    Failed { message: String, need_register: bool },
    /// Scaricamento: inizia la pagina `index` (da 1) di `count`.
    SyncProgress { index: u32, count: u32 },
    /// Una pagina scaricata: va salvata, poi si chiama `confirm_saved`.
    Downloaded { raw: Vec<u8>, time_bcd: Vec<u8>, crc_ok: bool },
    /// Scaricamento finito, il pad è di nuovo in live.
    SyncDone { files: u32, deleted: u32 },
}

/// Cosa fare dopo un passo: byte da scrivere sulla caratteristica dei
/// comandi, eventi per l'interfaccia, e quanto attendere la risposta prima di
/// chiamare `on_timeout` (0 = non si attende nulla).
#[derive(uniffi::Record, Debug, Default)]
pub struct Step {
    pub writes: Vec<Vec<u8>>,
    pub events: Vec<SessionEvent>,
    pub wait_ms: u32,
}

#[derive(uniffi::Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Down,
    Move,
    Up,
    Hover,
}

/// Un punto penna in coordinate del pad, pressione 0..=1.
#[derive(uniffi::Record, Clone, Copy, Debug, PartialEq)]
pub struct PenPoint {
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
    pub phase: Phase,
}

// ---------------------------------------------------------------------------
// Macchina a stati
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    DimW,
    DimH,
    Connect,
    SetTime,
    ReadTime,
    Transfer,
    Name,
    Firmware0,
    Firmware1,
    Battery,
    ModeLive,
    // scaricamento (device::sync_offline)
    ModePaper,
    FilesCount,
    GetStrokes,
    Download,
    /// Nessun comando: si attende `c8 ed <crc>` raccogliendo i dati offline.
    EndRead,
    Delete,
    /// Ritorno al live a fine scaricamento.
    ModeLiveBack,
}

#[derive(Clone, Debug)]
struct Cmd {
    kind: Kind,
    op: u8,
    args: Vec<u8>,
    expect: u8,
    /// Se fallisce si ferma tutto (altrimenti si prosegue, come la desktop).
    fatal: bool,
}

impl Cmd {
    fn new(kind: Kind) -> Self {
        let (op, args, expect, fatal): (u8, Vec<u8>, u8, bool) = match kind {
            Kind::DimW => (op::GET_DIMENSION, protocol::ARGS_WIDTH.to_vec(), op::R_DIMENSION, false),
            Kind::DimH => (op::GET_DIMENSION, protocol::ARGS_HEIGHT.to_vec(), op::R_DIMENSION, false),
            Kind::Connect => (op::CONNECT, Vec::new(), op::R_GENERIC, true), // args = uuid
            Kind::SetTime => (op::SET_TIME, protocol::pack_time_now().to_vec(), op::R_GENERIC, false),
            Kind::ReadTime => (op::GET_TIME, Vec::new(), op::R_TIME, false),
            Kind::Transfer => (
                op::SET_FILE_TRANSFER_REPORTING,
                protocol::ARGS_TRANSFER_GATT.to_vec(),
                op::R_GENERIC,
                false,
            ),
            Kind::Name => (op::GET_NAME, Vec::new(), op::R_NAME, false),
            Kind::Firmware0 => (op::GET_FIRMWARE, vec![0], op::R_FIRMWARE, false),
            Kind::Firmware1 => (op::GET_FIRMWARE, vec![1], op::R_FIRMWARE, false),
            Kind::Battery => (op::GET_BATTERY, Vec::new(), op::R_BATTERY, false),
            Kind::ModeLive => (op::SET_MODE, vec![mode::LIVE], op::R_GENERIC, true),
            Kind::ModePaper => (op::SET_MODE, vec![mode::PAPER], op::R_GENERIC, true),
            Kind::FilesCount => (op::FILES_COUNT, Vec::new(), op::R_FILES_COUNT, true),
            Kind::GetStrokes => (op::GET_STROKES, Vec::new(), op::R_STROKES, true),
            Kind::Download => (op::DOWNLOAD_OLDEST, Vec::new(), op::R_DOWNLOAD, true),
            Kind::EndRead => (op::DOWNLOAD_OLDEST, Vec::new(), op::R_DOWNLOAD, true),
            Kind::Delete => (op::DELETE_OLDEST, Vec::new(), op::R_GENERIC, true),
            Kind::ModeLiveBack => (op::SET_MODE, vec![mode::LIVE], op::R_GENERIC, true),
        };
        Cmd { kind, op, args, expect, fatal }
    }
}

#[derive(Default)]
struct Inner {
    nordic: NordicBuffer,
    decoder: LiveDecoder,
    queue: VecDeque<Cmd>,
    /// Comando inviato, in attesa di risposta.
    pending: Option<Cmd>,
    /// Registrazione: in attesa del pulsante con questo UUID.
    button: Option<[u8; 6]>,
    info: PadInfo,
    name_buf: Vec<u8>,
    fw_hi: String,
    live: bool,
    /// Il pad ci ha riconosciuti in questa connessione (CONNECT o registrazione).
    authorized: bool,
    sync: Option<Sync>,
}

/// Stato dello scaricamento in corso.
struct Sync {
    files: u32,
    /// Pagina in corso, da 0.
    index: u32,
    deleted: u32,
    time_bcd: Vec<u8>,
    offline: Vec<u8>,
    last_offline: Instant,
    /// Pagina consegnata a Kotlin: si attende `confirm_saved` (valore: CRC ok).
    awaiting_save: Option<bool>,
}

/// Verifica una risposta come `ble::check_reply` della desktop.
fn check_reply(r: Reply, expect: u8) -> Result<Reply, ProtoError> {
    if r.opcode == op::R_GENERIC {
        match r.data.first() {
            Some(0x00) => Ok(r),
            Some(&code) => Err(ProtoError::Device(code)),
            None => Err(ProtoError::UnexpectedData("risposta b3 vuota".into())),
        }
    } else if r.opcode != expect {
        Err(ProtoError::UnexpectedReply { expected: expect, got: r.opcode, data: r.data })
    } else {
        Ok(r)
    }
}

fn dimension(r: &Reply, selector: u8) -> Option<f32> {
    let d = &r.data;
    (d.len() == 6 && d[0] == selector && d[1] == 0)
        .then(|| u32::from_le_bytes([d[2], d[3], d[4], d[5]]) as f32)
}

fn failed(e: &ProtoError) -> SessionEvent {
    SessionEvent::Failed {
        message: e.to_string(),
        need_register: matches!(e, ProtoError::Authorization),
    }
}

impl Inner {
    fn reset(&mut self) {
        *self = Inner::default();
    }

    /// Invia il prossimo comando della coda, oppure chiude la sequenza.
    fn advance(&mut self, step: &mut Step) {
        self.pending = None;
        if let Some(cmd) = self.queue.pop_front() {
            if cmd.kind != Kind::EndRead {
                step.writes.push(protocol::frame(cmd.op, &cmd.args));
            }
            step.wait_ms = REPLY_MS;
            self.pending = Some(cmd);
        }
    }

    fn fail(&mut self, step: &mut Step, e: &ProtoError) {
        tracing::warn!("sessione: {e}");
        step.events.push(failed(e));
        step.wait_ms = 0;
        self.queue.clear();
        self.pending = None;
        self.button = None;
        self.sync = None;
    }

    /// Fine scaricamento: si torna al live.
    fn finish_sync(&mut self, step: &mut Step) {
        self.queue = VecDeque::from([Cmd::new(Kind::ModeLiveBack)]);
        self.advance(step);
    }

    /// Prossima pagina da scaricare, o la fine.
    fn next_page(&mut self, step: &mut Step) {
        let Some(sy) = self.sync.as_mut() else { return };
        if sy.index >= sy.files {
            return self.finish_sync(step);
        }
        step.events.push(SessionEvent::SyncProgress { index: sy.index + 1, count: sy.files });
        self.queue = VecDeque::from([Cmd::new(Kind::GetStrokes)]);
        self.advance(step);
    }

    fn on_reply(&mut self, r: Reply, step: &mut Step) {
        if let Some(uuid) = self.button {
            match r.opcode {
                op::R_BUTTON_SLATE | op::R_BUTTON_INTUOS => {
                    self.button = None;
                    self.authorized = true;
                    step.events.push(SessionEvent::Registered { uuid: uuid.to_vec() });
                    // register_device_finish (Slate), poi il live nella stessa sessione.
                    self.queue = [
                        Kind::SetTime,
                        Kind::ReadTime,
                        Kind::Transfer,
                        Kind::Name,
                        Kind::DimW,
                        Kind::DimH,
                        Kind::Firmware0,
                        Kind::Firmware1,
                        Kind::Battery,
                        Kind::ModeLive,
                    ]
                    .into_iter()
                    .map(Cmd::new)
                    .collect();
                    self.advance(step);
                }
                op::R_GENERIC if r.data.first().map_or(false, |c| *c != 0) => {
                    self.fail(step, &ProtoError::Device(r.data[0]));
                }
                other => self.fail(
                    step,
                    &ProtoError::UnexpectedReply { expected: op::R_BUTTON_SLATE, got: other, data: r.data },
                ),
            }
            return;
        }
        let Some(cmd) = self.pending.clone() else {
            tracing::info!("RX non richiesta {:02x} {:02x?}", r.opcode, r.data);
            return;
        };
        match check_reply(r, cmd.expect) {
            Ok(r) => {
                match cmd.kind {
                    Kind::DimW => self.info.width = dimension(&r, 0x03).unwrap_or(self.info.width),
                    Kind::DimH => self.info.height = dimension(&r, 0x04).unwrap_or(self.info.height),
                    Kind::Name => {
                        self.name_buf.extend_from_slice(&r.data);
                        if self.name_buf.last() != Some(&0x0a) {
                            step.wait_ms = REPLY_MS; // il nome continua nel prossimo pezzo
                            return;
                        }
                        self.info.name = String::from_utf8_lossy(&self.name_buf).trim().to_string();
                    }
                    Kind::Firmware0 | Kind::Firmware1 => {
                        let part: String = r.data.iter().skip(1).map(|b| format!("{b:x}")).collect();
                        if cmd.kind == Kind::Firmware0 {
                            self.fw_hi = part;
                        } else {
                            self.info.firmware = format!("{}-{part}", self.fw_hi);
                        }
                    }
                    Kind::Battery => {
                        if let Some(&p) = r.data.first() {
                            self.info.battery_pct = Some(p);
                            self.info.charging = r.data.get(1) == Some(&1);
                        }
                    }
                    Kind::ModeLive => {
                        self.live = true;
                        step.events.push(SessionEvent::Live { info: self.info.clone() });
                    }
                    Kind::Connect => self.authorized = true,
                    Kind::SetTime | Kind::ReadTime | Kind::Transfer | Kind::ModePaper => {}
                    Kind::FilesCount => {
                        let n = match r.data.as_slice() {
                            [lo, hi, ..] => u16::from_le_bytes([*lo, *hi]) as u32,
                            _ => 0,
                        };
                        tracing::info!("Disegni nel pad: {n}");
                        if let Some(sy) = self.sync.as_mut() {
                            sy.files = n;
                        }
                        return self.next_page(step);
                    }
                    Kind::GetStrokes => {
                        if let Some(sy) = self.sync.as_mut() {
                            sy.time_bcd = r.data.get(4..).map(|t| t.to_vec()).unwrap_or_default();
                            sy.offline.clear();
                        }
                        self.queue = VecDeque::from([Cmd::new(Kind::Download)]);
                    }
                    Kind::Download => {
                        if r.data.first() != Some(&0xbe) {
                            let e = ProtoError::UnexpectedData(format!("download: atteso c8 be, ricevuto {:02x?}", r.data));
                            return self.fail(step, &e);
                        }
                        if let Some(sy) = self.sync.as_mut() {
                            sy.last_offline = Instant::now();
                        }
                        self.queue = VecDeque::from([Cmd::new(Kind::EndRead)]);
                    }
                    Kind::EndRead => {
                        if r.data.first() != Some(&0xed) || r.data.len() < 5 {
                            let e = ProtoError::UnexpectedData(format!("fine lettura: atteso c8 ed, ricevuto {:02x?}", r.data));
                            return self.fail(step, &e);
                        }
                        let crc = u32::from_le_bytes([r.data[1], r.data[2], r.data[3], r.data[4]]);
                        let Some(sy) = self.sync.as_mut() else { return };
                        let raw = std::mem::take(&mut sy.offline);
                        let crc_ok = protocol::crc32(&raw) == crc;
                        if !crc_ok {
                            tracing::warn!("CRC dei dati scaricati non corrisponde");
                        }
                        sy.awaiting_save = Some(crc_ok);
                        step.events.push(SessionEvent::Downloaded { raw, time_bcd: sy.time_bcd.clone(), crc_ok });
                        self.pending = None;
                        step.wait_ms = 0;
                        return;
                    }
                    Kind::Delete => {
                        if let Some(sy) = self.sync.as_mut() {
                            sy.deleted += 1;
                            sy.index += 1;
                        }
                        return self.next_page(step);
                    }
                    Kind::ModeLiveBack => {
                        self.live = true;
                        if let Some(sy) = self.sync.take() {
                            step.events.push(SessionEvent::SyncDone { files: sy.files, deleted: sy.deleted });
                        }
                    }
                }
                self.advance(step);
            }
            Err(ProtoError::Device(0x07)) if cmd.kind == Kind::Connect => {
                self.fail(step, &ProtoError::Authorization)
            }
            Err(e) if cmd.fatal => self.fail(step, &e),
            Err(e) => {
                tracing::warn!("{:?}: {e} (proseguo)", cmd.kind);
                self.advance(step);
            }
        }
    }
}

/// Una connessione al pad (una per `BluetoothGatt`).
#[derive(uniffi::Object, Default)]
pub struct SlateSession {
    inner: Mutex<Inner>,
}

#[uniffi::export]
impl SlateSession {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Avvio del live con l'identità registrata (`device::start_live`):
    /// dimensioni, CONNECT, ora, batteria, SET_MODE LIVE.
    pub fn begin_live(&self, uuid: Vec<u8>) -> Step {
        let mut s = self.inner.lock().unwrap();
        s.reset();
        let mut step = Step::default();
        if uuid.len() != 6 {
            s.fail(&mut step, &ProtoError::Authorization);
            return step;
        }
        let mut connect = Cmd::new(Kind::Connect);
        connect.args = uuid;
        s.queue = VecDeque::from([
            Cmd::new(Kind::DimW),
            Cmd::new(Kind::DimH),
            connect,
            Cmd::new(Kind::SetTime),
            Cmd::new(Kind::Battery),
            Cmd::new(Kind::ModeLive),
        ]);
        s.advance(&mut step);
        step
    }

    /// Registrazione (`device::register`): il pad deve lampeggiare blu e
    /// l'utente deve premere il pulsante dopo questa chiamata.
    pub fn begin_register(&self) -> Step {
        let mut s = self.inner.lock().unwrap();
        s.reset();
        let mut uuid = [0u8; 6];
        if getrandom::fill(&mut uuid).is_err() {
            // Come la desktop: basta un identificativo, non serve crittografia.
            let t = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;
            uuid.copy_from_slice(&t.to_le_bytes()[..6]);
        }
        s.button = Some(uuid);
        Step {
            writes: vec![protocol::frame(op::REGISTER_PRESS_BUTTON, &uuid)],
            events: Vec::new(),
            wait_ms: BUTTON_MS,
        }
    }

    /// Byte arrivati sulla caratteristica Nordic (risposte ai comandi).
    pub fn on_nordic(&self, bytes: Vec<u8>) -> Step {
        let mut s = self.inner.lock().unwrap();
        s.nordic.push(&bytes);
        let mut step = Step::default();
        while let Some(r) = s.nordic.pop() {
            s.on_reply(r, &mut step);
        }
        step
    }

    /// Nessuna risposta entro `wait_ms`: un comando facoltativo si salta,
    /// uno necessario ferma la sequenza.
    pub fn on_timeout(&self) -> Step {
        let mut s = self.inner.lock().unwrap();
        let mut step = Step::default();
        if s.button.is_some() {
            s.fail(&mut step, &ProtoError::MissingReply(op::REGISTER_PRESS_BUTTON));
        } else if let Some(cmd) = s.pending.clone() {
            let receiving = cmd.kind == Kind::EndRead
                && s.sync.as_ref().map_or(false, |sy| sy.last_offline.elapsed() < Duration::from_millis(REPLY_MS as u64));
            if receiving {
                step.wait_ms = REPLY_MS; // i dati stanno ancora arrivando
            } else if cmd.fatal {
                s.fail(&mut step, &ProtoError::MissingReply(cmd.op));
            } else {
                tracing::warn!("{:?}: nessuna risposta (proseguo)", cmd.kind);
                s.advance(&mut step);
            }
        }
        step
    }

    /// Scarica le pagine dalla memoria del pad (`device::sync_offline`): una
    /// alla volta, dalla più vecchia; ognuna arriva con `Downloaded` e, dopo
    /// `confirm_saved(true)`, viene cancellata dal pad. Alla fine torna il live.
    pub fn begin_sync(&self, uuid: Vec<u8>) -> Step {
        let mut s = self.inner.lock().unwrap();
        let mut step = Step::default();
        if s.pending.is_some() || s.button.is_some() || s.sync.is_some() {
            return step; // c'è già una sequenza in corso
        }
        s.live = false;
        s.sync = Some(Sync {
            files: 0,
            index: 0,
            deleted: 0,
            time_bcd: Vec::new(),
            offline: Vec::new(),
            last_offline: Instant::now(),
            awaiting_save: None,
        });
        let mut q = VecDeque::new();
        if !s.authorized {
            let mut connect = Cmd::new(Kind::Connect);
            connect.args = uuid;
            q.push_back(connect);
        }
        // Prima si esce dal live: in live il pad rifiuta la scelta del canale.
        q.extend([Kind::ModePaper, Kind::SetTime, Kind::DimW, Kind::DimH, Kind::Transfer, Kind::FilesCount].map(Cmd::new));
        s.queue = q;
        s.advance(&mut step);
        step
    }

    /// Dopo `Downloaded`: `saved` = la pagina è al sicuro sul telefono. Solo
    /// allora, e se il CRC tornava, la si cancella dal pad e si passa alla
    /// prossima; altrimenti ci si ferma (come la desktop).
    pub fn confirm_saved(&self, saved: bool) -> Step {
        let mut s = self.inner.lock().unwrap();
        let mut step = Step::default();
        let Some(crc_ok) = s.sync.as_mut().and_then(|sy| sy.awaiting_save.take()) else {
            return step;
        };
        if saved && crc_ok {
            s.queue = VecDeque::from([Cmd::new(Kind::Delete)]);
            s.advance(&mut step);
        } else {
            tracing::warn!("Disegno non verificato o non salvato: NON lo cancello e mi fermo");
            s.finish_sync(&mut step);
        }
        step
    }

    /// Byte arrivati sulla caratteristica OFFLINE (dati delle pagine).
    pub fn on_offline(&self, bytes: Vec<u8>) {
        let mut s = self.inner.lock().unwrap();
        if let Some(sy) = s.sync.as_mut() {
            sy.offline.extend_from_slice(&bytes);
            sy.last_offline = Instant::now();
        }
    }

    pub fn is_syncing(&self) -> bool {
        self.inner.lock().unwrap().sync.is_some()
    }

    /// Pacchetto della caratteristica LIVE → punti penna.
    pub fn on_live(&self, bytes: Vec<u8>) -> Vec<PenPoint> {
        let mut s = self.inner.lock().unwrap();
        s.decoder
            .decode(&bytes)
            .into_iter()
            .map(|e| PenPoint {
                x: e.x,
                y: e.y,
                pressure: e.pressure,
                phase: match e.phase {
                    PenPhase::Down => Phase::Down,
                    PenPhase::Move => Phase::Move,
                    PenPhase::Up => Phase::Up,
                    PenPhase::Hover => Phase::Hover,
                },
            })
            .collect()
    }

    /// Ferma il live (Tuhi usa IDLE): byte da scrivere prima di scollegarsi.
    pub fn stop_frame(&self) -> Vec<u8> {
        self.inner.lock().unwrap().live = false;
        protocol::frame(op::SET_MODE, &[mode::IDLE])
    }

    pub fn is_live(&self) -> bool {
        self.inner.lock().unwrap().live
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reply(opcode: u8, data: &[u8]) -> Vec<u8> {
        let mut v = vec![opcode, data.len() as u8];
        v.extend_from_slice(data);
        v
    }

    #[test]
    fn live_fa_la_sequenza_della_desktop() {
        let s = SlateSession::new();
        let uuid = vec![1, 2, 3, 4, 5, 6];
        let st = s.begin_live(uuid.clone());
        assert_eq!(st.writes, vec![protocol::frame(op::GET_DIMENSION, &protocol::ARGS_WIDTH)]);
        // larghezza 21600, altezza 14800 (risposte spezzate in due notifiche)
        let w = reply(op::R_DIMENSION, &[3, 0, 0x60, 0x54, 0, 0]);
        assert!(s.on_nordic(w[..3].to_vec()).writes.is_empty());
        let st = s.on_nordic(w[3..].to_vec());
        assert_eq!(st.writes, vec![protocol::frame(op::GET_DIMENSION, &protocol::ARGS_HEIGHT)]);
        let st = s.on_nordic(reply(op::R_DIMENSION, &[4, 0, 0xd0, 0x39, 0, 0]));
        assert_eq!(st.writes, vec![protocol::frame(op::CONNECT, &uuid)]);
        let st = s.on_nordic(reply(op::R_GENERIC, &[0]));
        assert_eq!(st.writes[0][0], op::SET_TIME);
        // l'ora può fallire: si prosegue
        let st = s.on_nordic(reply(op::R_GENERIC, &[1]));
        assert_eq!(st.writes, vec![protocol::frame(op::GET_BATTERY, &[])]);
        let st = s.on_nordic(reply(op::R_BATTERY, &[80, 1]));
        assert_eq!(st.writes, vec![protocol::frame(op::SET_MODE, &[mode::LIVE])]);
        let st = s.on_nordic(reply(op::R_GENERIC, &[0]));
        assert_eq!(st.wait_ms, 0);
        let info = PadInfo {
            width: 21600.0,
            height: 14800.0,
            battery_pct: Some(80),
            charging: true,
            ..Default::default()
        };
        assert_eq!(st.events, vec![SessionEvent::Live { info }]);
        assert!(s.is_live());
    }

    #[test]
    fn uuid_sconosciuto_chiede_registrazione() {
        let s = SlateSession::new();
        s.begin_live(vec![9; 6]);
        s.on_nordic(reply(op::R_DIMENSION, &[3, 0, 1, 0, 0, 0]));
        s.on_nordic(reply(op::R_DIMENSION, &[4, 0, 1, 0, 0, 0]));
        let st = s.on_nordic(reply(op::R_GENERIC, &[0x07]));
        assert!(st.writes.is_empty());
        assert!(matches!(st.events[..], [SessionEvent::Failed { need_register: true, .. }]));
    }

    #[test]
    fn registrazione_poi_live() {
        let s = SlateSession::new();
        let st = s.begin_register();
        assert_eq!(st.wait_ms, BUTTON_MS);
        assert_eq!(st.writes[0][..2], [op::REGISTER_PRESS_BUTTON, 6]);
        let uuid = st.writes[0][2..].to_vec();
        let st = s.on_nordic(reply(op::R_BUTTON_SLATE, &[0]));
        assert_eq!(st.events, vec![SessionEvent::Registered { uuid }]);
        assert_eq!(st.writes[0][0], op::SET_TIME);
        // nessuna risposta all'ora: si salta
        let st = s.on_timeout();
        assert_eq!(st.writes[0][0], op::GET_TIME);
        s.on_nordic(reply(op::R_TIME, &[0; 6]));
        let st = s.on_nordic(reply(op::R_GENERIC, &[0])); // transfer
        assert_eq!(st.writes[0][0], op::GET_NAME);
        // nome in due pezzi
        assert!(s.on_nordic(reply(op::R_NAME, b"Bamboo")).writes.is_empty());
        let st = s.on_nordic(reply(op::R_NAME, b" Slate\n"));
        assert_eq!(st.writes[0][0], op::GET_DIMENSION);
        s.on_nordic(reply(op::R_DIMENSION, &[3, 0, 1, 0, 0, 0]));
        s.on_nordic(reply(op::R_DIMENSION, &[4, 0, 2, 0, 0, 0]));
        s.on_nordic(reply(op::R_FIRMWARE, &[0, 0x1a]));
        s.on_nordic(reply(op::R_FIRMWARE, &[1, 0x2b]));
        s.on_nordic(reply(op::R_BATTERY, &[50, 0]));
        let st = s.on_nordic(reply(op::R_GENERIC, &[0]));
        match &st.events[..] {
            [SessionEvent::Live { info }] => {
                assert_eq!(info.name, "Bamboo Slate");
                assert_eq!(info.firmware, "1a-2b");
                assert_eq!((info.width, info.height), (1.0, 2.0));
            }
            e => panic!("eventi {e:?}"),
        }
    }

    #[test]
    fn scarica_due_pagine_e_torna_in_live() {
        let s = SlateSession::new();
        s.begin_live(vec![1; 6]);
        for r in [
            reply(op::R_DIMENSION, &[3, 0, 1, 0, 0, 0]),
            reply(op::R_DIMENSION, &[4, 0, 1, 0, 0, 0]),
            reply(op::R_GENERIC, &[0]),
            reply(op::R_GENERIC, &[0]),
            reply(op::R_BATTERY, &[50, 0]),
            reply(op::R_GENERIC, &[0]),
        ] {
            s.on_nordic(r);
        }
        assert!(s.is_live());
        // già autorizzati: niente CONNECT, si esce dal live
        let st = s.begin_sync(vec![1; 6]);
        assert_eq!(st.writes, vec![protocol::frame(op::SET_MODE, &[mode::PAPER])]);
        assert!(!s.is_live());
        for r in [
            reply(op::R_GENERIC, &[0]),                    // paper
            reply(op::R_GENERIC, &[0]),                    // ora
            reply(op::R_DIMENSION, &[3, 0, 1, 0, 0, 0]),
            reply(op::R_DIMENSION, &[4, 0, 1, 0, 0, 0]),
            reply(op::R_GENERIC, &[0]),                    // canale
        ] {
            s.on_nordic(r);
        }
        let st = s.on_nordic(reply(op::R_FILES_COUNT, &[2, 0]));
        assert_eq!(st.events, vec![SessionEvent::SyncProgress { index: 1, count: 2 }]);
        assert_eq!(st.writes, vec![protocol::frame(op::GET_STROKES, &[])]);
        let page = |s: &SlateSession, data: &[u8]| {
            s.on_nordic(reply(op::R_STROKES, &[9, 0, 0, 0, 0x26, 0x11, 0x08, 0x10, 0x00, 0x00]));
            let st = s.on_nordic(reply(op::R_DOWNLOAD, &[0xbe]));
            assert!(st.writes.is_empty()); // si attendono i dati
            s.on_offline(data[..4].to_vec());
            s.on_offline(data[4..].to_vec());
            let crc = protocol::crc32(data).to_le_bytes();
            s.on_nordic(reply(op::R_DOWNLOAD, &[0xed, crc[0], crc[1], crc[2], crc[3]]))
        };
        let st = page(&s, b"pagina-uno");
        match &st.events[..] {
            [SessionEvent::Downloaded { raw, crc_ok: true, time_bcd }] => {
                assert_eq!(raw, b"pagina-uno");
                assert_eq!(time_bcd[0], 0x26);
            }
            e => panic!("{e:?}"),
        }
        let st = s.confirm_saved(true);
        assert_eq!(st.writes, vec![protocol::frame(op::DELETE_OLDEST, &[])]);
        let st = s.on_nordic(reply(op::R_GENERIC, &[0]));
        assert_eq!(st.events, vec![SessionEvent::SyncProgress { index: 2, count: 2 }]);
        page(&s, b"pagina-due");
        s.confirm_saved(true);
        let st = s.on_nordic(reply(op::R_GENERIC, &[0]));
        assert_eq!(st.writes, vec![protocol::frame(op::SET_MODE, &[mode::LIVE])]);
        let st = s.on_nordic(reply(op::R_GENERIC, &[0]));
        assert_eq!(st.events, vec![SessionEvent::SyncDone { files: 2, deleted: 2 }]);
        assert!(s.is_live());
    }

    #[test]
    fn non_salvata_non_si_cancella() {
        let s = SlateSession::new();
        let st = s.begin_sync(vec![1; 6]);
        assert_eq!(st.writes[0][0], op::CONNECT); // non ancora autorizzati
        for r in [
            reply(op::R_GENERIC, &[0]),
            reply(op::R_GENERIC, &[0]),
            reply(op::R_GENERIC, &[0]),
            reply(op::R_DIMENSION, &[3, 0, 1, 0, 0, 0]),
            reply(op::R_DIMENSION, &[4, 0, 1, 0, 0, 0]),
            reply(op::R_GENERIC, &[0]),
            reply(op::R_FILES_COUNT, &[1, 0]),
            reply(op::R_STROKES, &[0, 0, 0, 0]),
            reply(op::R_DOWNLOAD, &[0xbe]),
        ] {
            s.on_nordic(r);
        }
        s.on_offline(b"x".to_vec());
        // CRC sbagliato
        let st = s.on_nordic(reply(op::R_DOWNLOAD, &[0xed, 0, 0, 0, 0]));
        assert!(matches!(st.events[..], [SessionEvent::Downloaded { crc_ok: false, .. }]));
        let st = s.confirm_saved(true);
        assert_eq!(st.writes, vec![protocol::frame(op::SET_MODE, &[mode::LIVE])]);
    }

    #[test]
    fn annuncio_in_abbinamento() {
        let a = classify_advert("Bamboo Slate".into(), vec![MfrEntry { company: 0x4755, len: 4 }], vec![], false);
        assert!(a.candidate);
        assert_eq!(a.mode, AdvMode::Register);
        let a = classify_advert("Cuffie".into(), vec![MfrEntry { company: 0x004c, len: 9 }], vec![], false);
        assert!(!a.candidate);
    }

    #[test]
    fn punti_live() {
        let s = SlateSession::new();
        let p = s.on_live(vec![0xa1, 0x0c, 0x10, 0x00, 0x20, 0x00, 0x05, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].phase, Phase::Down);
        assert_eq!(p[1].phase, Phase::Up);
    }
}
