//! Operazioni sul pad, variante Slate: traduzione fedele di `tuhi/wacom.py`
//! (classi WacomRegisterHelper, WacomProtocolBase, WacomProtocolSlate).

use crate::ble::Pad;
use crate::protocol::{self, mode, op, ProtoError, Reply};
use std::time::Duration;

/// Informazioni lette dal pad.
#[derive(Clone, Debug, Default)]
pub struct DeviceInfo {
    pub name: String,
    pub firmware: String,
    pub battery_pct: Option<u8>,
    pub charging: bool,
    /// Area attiva in punti del pad (stesse unità delle coordinate live).
    pub width: f32,
    pub height: f32,
}

/// Un disegno scaricato dalla memoria del pad.
#[derive(Debug, Clone)]
pub struct OfflineFile {
    /// Byte grezzi (formato StrokeFile).
    pub raw: Vec<u8>,
    /// Data del disegno (BCD UTC "aammgghhmmss"), se il pad l'ha data.
    pub time_bcd: Vec<u8>,
    /// Il CRC dei dati corrisponde.
    pub crc_ok: bool,
}

/// Risultato di uno scaricamento dalla memoria del pad.
#[derive(Debug, Default)]
pub struct SyncResult {
    /// Disegni presenti nel pad all'inizio.
    pub files: usize,
    pub downloaded: Vec<OfflineFile>,
    /// Disegni cancellati dal pad dopo il salvataggio.
    pub deleted: usize,
}

// ---------------------------------------------------------------------------
// Identità dell'app (UUID di 6 byte, come Tuhi: uuid4().hex[:12])
// ---------------------------------------------------------------------------

const UUID_FILE: &str = "app_uuid.bin";

/// UUID con cui l'app è registrata sul pad, se c'è.
pub fn load_uuid() -> Option<[u8; 6]> {
    let d = std::fs::read(UUID_FILE).ok()?;
    d.try_into().ok()
}

fn save_uuid(u: &[u8; 6]) {
    let _ = std::fs::write(UUID_FILE, u);
}

fn new_uuid() -> [u8; 6] {
    // xorshift seminato da tempo + pid: basta per un identificativo (non crypto).
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut x = nanos ^ ((std::process::id() as u128) << 64) ^ 0x9e37_79b9_7f4a_7c15;
    let mut out = [0u8; 6];
    for b in out.iter_mut() {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        *b = (x & 0xff) as u8;
    }
    out
}

// ---------------------------------------------------------------------------
// Singoli comandi (tuhi/protocol.py, messaggi variante Slate)
// ---------------------------------------------------------------------------

/// CONNECT con il nostro UUID (Tuhi: check_connection / MsgConnectSlate).
pub async fn check_connection(pad: &mut Pad, uuid: &[u8; 6]) -> Result<(), ProtoError> {
    if pad.authorized {
        return Ok(());
    }
    match pad.execute(op::CONNECT, uuid, op::R_GENERIC).await {
        Ok(_) => {
            pad.authorized = true;
            Ok(())
        }
        Err(ProtoError::Device(0x07)) => Err(ProtoError::Authorization),
        Err(e) => Err(e),
    }
}

pub async fn set_time(pad: &mut Pad) -> Result<(), ProtoError> {
    pad.execute(op::SET_TIME, &protocol::pack_time_now(), op::R_GENERIC)
        .await
        .map(|_| ())
}

pub async fn read_time(pad: &mut Pad) -> Result<Vec<u8>, ProtoError> {
    Ok(pad.execute(op::GET_TIME, &[], op::R_TIME).await?.data)
}

pub async fn select_transfer_gatt(pad: &mut Pad) -> Result<(), ProtoError> {
    pad.execute(
        op::SET_FILE_TRANSFER_REPORTING,
        &protocol::ARGS_TRANSFER_GATT,
        op::R_GENERIC,
    )
    .await
    .map(|_| ())
}

/// Nome del pad: può arrivare in più pezzi, l'ultimo termina con 0x0a.
pub async fn get_name(pad: &mut Pad) -> Result<String, ProtoError> {
    let mut bytes = pad.execute(op::GET_NAME, &[], op::R_NAME).await?.data;
    while bytes.last() != Some(&0x0a) {
        let more = pad.wait(op::GET_NAME, op::R_NAME, Duration::from_secs(5)).await?;
        bytes.extend(more.data);
    }
    Ok(String::from_utf8_lossy(&bytes).trim().to_string())
}

/// Firmware: due richieste ([0] e [1]), come in Tuhi.
pub async fn get_firmware(pad: &mut Pad) -> Result<String, ProtoError> {
    let part = |d: &[u8]| d.iter().skip(1).map(|b| format!("{b:x}")).collect::<String>();
    let hi = pad.execute(op::GET_FIRMWARE, &[0], op::R_FIRMWARE).await?;
    let lo = pad.execute(op::GET_FIRMWARE, &[1], op::R_FIRMWARE).await?;
    Ok(format!("{}-{}", part(&hi.data), part(&lo.data)))
}

/// Batteria: (percentuale, in carica).
pub async fn get_battery(pad: &mut Pad) -> Result<(u8, bool), ProtoError> {
    let r = pad.execute(op::GET_BATTERY, &[], op::R_BATTERY).await?;
    let pct = *r
        .data
        .first()
        .ok_or_else(|| ProtoError::UnexpectedData("batteria: risposta vuota".into()))?;
    Ok((pct, r.data.get(1) == Some(&1)))
}

/// Larghezza e altezza dell'area attiva, in punti (Tuhi: update_dimensions).
pub async fn update_dimensions(pad: &mut Pad) -> Result<(f32, f32), ProtoError> {
    fn value(r: Reply, selector: u8) -> Result<f32, ProtoError> {
        if r.data.len() != 6 || r.data[0] != selector || r.data[1] != 0 {
            return Err(ProtoError::UnexpectedData(format!(
                "dimensione: risposta {:02x?}",
                r.data
            )));
        }
        Ok(u32::from_le_bytes([r.data[2], r.data[3], r.data[4], r.data[5]]) as f32)
    }
    let w = value(
        pad.execute(op::GET_DIMENSION, &protocol::ARGS_WIDTH, op::R_DIMENSION)
            .await?,
        0x03,
    )?;
    let h = value(
        pad.execute(op::GET_DIMENSION, &protocol::ARGS_HEIGHT, op::R_DIMENSION)
            .await?,
        0x04,
    )?;
    tracing::info!("Dimensioni pad: {w} x {h} punti");
    Ok((w, h))
}

pub async fn set_mode(pad: &mut Pad, m: u8) -> Result<(), ProtoError> {
    pad.execute(op::SET_MODE, &[m], op::R_GENERIC)
        .await
        .map(|_| ())
}

// ---------------------------------------------------------------------------
// Sequenze (tuhi/wacom.py)
// ---------------------------------------------------------------------------

/// Registrazione. Il pad deve essere in modalità abbinamento (LED blu
/// lampeggiante). Per lo Slate Tuhi NON manda il CONNECT: va dritto a
/// REGISTER_PRESS_BUTTON e poi attende la pressione del pulsante.
/// Il chiamante deve dire all'utente di premere il pulsante PRIMA di chiamarla.
pub async fn register(pad: &mut Pad) -> Result<([u8; 6], DeviceInfo), ProtoError> {
    let uuid = new_uuid();
    tracing::info!("Registrazione con UUID {:02x?}", uuid);
    pad.send(op::REGISTER_PRESS_BUTTON, &uuid).await?;

    // REGISTER_WAIT_FOR_BUTTON: e4 (Slate) o 53 (Intuos Pro).
    let r = pad.recv(op::REGISTER_PRESS_BUTTON, Duration::from_secs(30)).await?;
    match r.opcode {
        op::R_BUTTON_SLATE | op::R_BUTTON_INTUOS => {}
        op::R_GENERIC if r.data.first().map_or(false, |c| *c != 0) => {
            return Err(ProtoError::Device(r.data[0]));
        }
        other => {
            return Err(ProtoError::UnexpectedReply {
                expected: op::R_BUTTON_SLATE,
                got: other,
                data: r.data,
            });
        }
    }
    // Da qui il pad conosce il nostro UUID: lo salviamo subito.
    save_uuid(&uuid);
    pad.authorized = true;
    tracing::info!("Pulsante premuto: registrazione accettata");

    // register_device_finish (Slate): ora, verifica ora, canale dati, nome,
    // dimensioni, firmware, batteria.
    let mut info = DeviceInfo::default();
    if let Err(e) = set_time(pad).await {
        tracing::warn!("set_time: {e}");
    }
    let _ = read_time(pad).await;
    if let Err(e) = select_transfer_gatt(pad).await {
        tracing::warn!("select_transfer_gatt: {e}");
    }
    info.name = get_name(pad).await.unwrap_or_default();
    if let Ok((w, h)) = update_dimensions(pad).await {
        info.width = w;
        info.height = h;
    }
    info.firmware = get_firmware(pad).await.unwrap_or_default();
    if let Ok((p, c)) = get_battery(pad).await {
        info.battery_pct = Some(p);
        info.charging = c;
    }
    Ok((uuid, info))
}

/// Avvio del live (WacomProtocolSlate.live_mode): dimensioni, CONNECT, poi
/// SET_MODE LIVE. Ora e batteria vengono lette dopo il CONNECT come in
/// retrieve_data (servono solo per l'interfaccia).
pub async fn start_live(pad: &mut Pad, uuid: &[u8; 6]) -> Result<DeviceInfo, ProtoError> {
    let mut info = DeviceInfo::default();
    match update_dimensions(pad).await {
        Ok((w, h)) => {
            info.width = w;
            info.height = h;
        }
        Err(e) => tracing::warn!("dimensioni: {e}"),
    }
    check_connection(pad, uuid).await?;
    let _ = set_time(pad).await;
    if let Ok((p, c)) = get_battery(pad).await {
        info.battery_pct = Some(p);
        info.charging = c;
    }
    set_mode(pad, mode::LIVE).await?;
    Ok(info)
}

/// Stop del live (Tuhi usa IDLE, non PAPER).
pub async fn stop_live(pad: &mut Pad) -> Result<(), ProtoError> {
    set_mode(pad, mode::IDLE).await
}

/// Scaricamento dei disegni (WacomProtocolSlate.retrieve_data +
/// read_offline_data).
///
/// Per ogni disegno, dal più vecchio: scarica, chiama `save` (che deve
/// salvarlo su disco e restituire `true` se ci è riuscito) e, solo se
/// `delete` è attivo, il CRC torna e il salvataggio è riuscito, lo cancella
/// dal pad. Senza cancellare, il pad dà sempre il più vecchio: se ne scarica
/// uno solo (come Tuhi con `peek_at_drawing`).
pub async fn sync_offline(
    pad: &mut Pad,
    uuid: &[u8; 6],
    delete: bool,
    mut save: impl FnMut(&OfflineFile) -> bool,
) -> Result<SyncResult, ProtoError> {
    check_connection(pad, uuid).await?;
    // Prima si esce dal live (in live il pad rifiuta la scelta del canale
    // dati con INVALID_STATE), poi si sceglie il canale per i file.
    set_mode(pad, mode::PAPER).await?;
    if let Err(e) = set_time(pad).await {
        tracing::warn!("set_time: {e}");
    }
    let _ = update_dimensions(pad).await;
    if let Err(e) = select_transfer_gatt(pad).await {
        tracing::warn!("select_transfer_gatt: {e} (proseguo)");
    }

    let r = pad.execute(op::FILES_COUNT, &[], op::R_FILES_COUNT).await?;
    let count = match r.data.as_slice() {
        [lo, hi, ..] => u16::from_le_bytes([*lo, *hi]) as usize,
        _ => 0,
    };
    tracing::info!("Disegni nel pad: {count}");
    let mut res = SyncResult {
        files: count,
        ..Default::default()
    };

    for i in 0..count {
        let s = pad.execute(op::GET_STROKES, &[], op::R_STROKES).await?;
        let time_bcd = s.data.get(4..).map(|t| t.to_vec()).unwrap_or_default();
        if s.data.len() >= 4 {
            let nbytes = u32::from_le_bytes([s.data[0], s.data[1], s.data[2], s.data[3]]);
            tracing::info!("Disegno {}/{count}: {nbytes} byte, data {time_bcd:02x?}", i + 1);
        }

        let _ = pad.take_offline();
        let d = pad.execute(op::DOWNLOAD_OLDEST, &[], op::R_DOWNLOAD).await?;
        if d.data.first() != Some(&0xbe) {
            return Err(ProtoError::UnexpectedData(format!(
                "download: atteso c8 be, ricevuto {:02x?}",
                d.data
            )));
        }
        let (raw, crc_ok) = wait_for_end_read(pad).await?;
        let file = OfflineFile { raw, time_bcd, crc_ok };
        let saved = save(&file);
        res.downloaded.push(file);

        if !delete {
            if count > 1 {
                tracing::info!("Senza cancellare si può scaricare solo il disegno più vecchio");
            }
            break;
        }
        if !crc_ok || !saved {
            tracing::warn!("Disegno non verificato o non salvato: NON lo cancello e mi fermo");
            break;
        }
        pad.execute(op::DELETE_OLDEST, &[], op::R_GENERIC).await?;
        res.deleted += 1;
    }
    Ok(res)
}

/// Quante pagine sono ancora nella memoria del pad. Il conteggio richiede la
/// modalità "carta"; alla fine il pad torna in live.
pub async fn count_files(pad: &mut Pad) -> Result<usize, ProtoError> {
    set_mode(pad, mode::PAPER).await?;
    let r = pad.execute(op::FILES_COUNT, &[], op::R_FILES_COUNT).await;
    let back = set_mode(pad, mode::LIVE).await;
    let r = r?;
    back?;
    Ok(match r.data.as_slice() {
        [lo, hi, ..] => u16::from_le_bytes([*lo, *hi]) as usize,
        _ => 0,
    })
}

/// Attende la fine del download (`c8 ed <crc>`) raccogliendo i dati offline
/// (Tuhi: wait_for_end_read, variante Slate).
async fn wait_for_end_read(pad: &mut Pad) -> Result<(Vec<u8>, bool), ProtoError> {
    loop {
        match pad
            .wait(op::DOWNLOAD_OLDEST, op::R_DOWNLOAD, Duration::from_secs(5))
            .await
        {
            Ok(r) => {
                if r.data.first() != Some(&0xed) || r.data.len() < 5 {
                    return Err(ProtoError::UnexpectedData(format!(
                        "fine lettura: atteso c8 ed, ricevuto {:02x?}",
                        r.data
                    )));
                }
                let crc = u32::from_le_bytes([r.data[1], r.data[2], r.data[3], r.data[4]]);
                let data = pad.take_offline();
                let ok = protocol::crc32(&data) == crc;
                if !ok {
                    tracing::warn!("CRC dei dati scaricati non corrisponde");
                }
                return Ok((data, ok));
            }
            // Se i dati stanno ancora arrivando, continuiamo ad aspettare.
            Err(ProtoError::MissingReply(_))
                if pad.last_offline().elapsed() < Duration::from_secs(5) =>
            {
                continue
            }
            Err(e) => return Err(e),
        }
    }
}
