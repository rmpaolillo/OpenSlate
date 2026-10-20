//! Protocollo BLE del Wacom SmartPad, variante **Slate** (Bamboo Slate/Folio).
//!
//! Traduzione fedele di Tuhi (`tuhi/protocol.py` e `tuhi/wacom.py`, licenza
//! GPL): stessi opcode, stessi argomenti, stesse risposte attese.
//!
//! Regole chiave (da Tuhi):
//! - ogni comando è `[opcode][len][args…]`; un comando senza argomenti si manda
//!   con un singolo byte `0x00` ("Empty messages don't exist");
//! - le risposte arrivano sulla caratteristica Nordic RX come FLUSSO di byte:
//!   vanno accumulate e separate usando il campo lunghezza;
//! - la risposta generica `0xb3` porta un codice: `0x00` = successo, altrimenti
//!   errore del dispositivo; ogni altro comando ha la sua risposta specifica.

use uuid::Uuid;

// ---------------------------------------------------------------------------
// UUID GATT (tuhi/wacom.py)
// ---------------------------------------------------------------------------

pub const NORDIC_UART_SERVICE: Uuid = Uuid::from_u128(0x6e400001_b5a3_f393_e0a9_e50e24dcca9e);
/// Caratteristica su cui scriviamo i comandi (Tuhi: NORDIC_UART_CHRC_TX).
pub const NORDIC_UART_WRITE: Uuid = Uuid::from_u128(0x6e400002_b5a3_f393_e0a9_e50e24dcca9e);
/// Caratteristica da cui arrivano le risposte (Tuhi: NORDIC_UART_CHRC_RX).
pub const NORDIC_UART_NOTIFY: Uuid = Uuid::from_u128(0x6e400003_b5a3_f393_e0a9_e50e24dcca9e);
pub const WACOM_LIVE_SERVICE: Uuid = Uuid::from_u128(0x00001523_1212_efde_1523_785feabcd123);
pub const WACOM_LIVE_PEN_DATA: Uuid = Uuid::from_u128(0x00001524_1212_efde_1523_785feabcd123);
pub const WACOM_OFFLINE_SERVICE: Uuid = Uuid::from_u128(0xffee0001_bbaa_9988_7766_554433221100);
pub const WACOM_OFFLINE_PEN_DATA: Uuid = Uuid::from_u128(0xffee0003_bbaa_9988_7766_554433221100);
pub const SYSEVENT_SERVICE: Uuid = Uuid::from_u128(0x3a340720_c572_11e5_86c5_0002a5d5c51b);
pub const SYSEVENT_CHRC: Uuid = Uuid::from_u128(0x3a340721_c572_11e5_86c5_0002a5d5c51b);

/// Company ID Wacom negli annunci BLE (tuhi/base.py: WACOM_COMPANY_IDS).
pub const WACOM_COMPANY_IDS: [u16; 3] = [0x4755, 0x4157, 0x424d];

/// Lunghezza dei dati produttore nell'annuncio quando il pad è in modalità
/// abbinamento (LED blu lampeggiante). In modalità normale sono 7 byte.
pub const MFR_DATA_LEN_REGISTER_MODE: usize = 4;

// ---------------------------------------------------------------------------
// Opcode (variante Slate) e risposte attese
// ---------------------------------------------------------------------------

pub mod op {
    pub const CONNECT: u8 = 0xe6; // args: uuid (6 byte)     -> b3
    pub const GET_NAME: u8 = 0xbb; // args: [00]             -> bc (nome, termina con 0x0a)
    pub const SET_TIME: u8 = 0xb6; // args: 6 byte BCD       -> b3
    pub const GET_TIME: u8 = 0xb6; // args: [00]             -> bd
    pub const GET_FIRMWARE: u8 = 0xb7; // args: [0] poi [1]  -> b8
    pub const GET_BATTERY: u8 = 0xb9; // args: [00]          -> ba [percento, in carica]
    pub const GET_DIMENSION: u8 = 0xea; // args: [03 00] larghezza / [04 00] altezza -> eb
    pub const SET_MODE: u8 = 0xb1; // args: [modo]           -> b3
    pub const GET_STROKES: u8 = 0xcc; // args: [00]          -> cf
    pub const FILES_COUNT: u8 = 0xc1; // args: [00]          -> c2
    pub const DOWNLOAD_OLDEST: u8 = 0xc3; // args: [00]      -> c8 [be]
    pub const DELETE_OLDEST: u8 = 0xca; // args: [00]        -> b3
    pub const REGISTER_PRESS_BUTTON: u8 = 0xe7; // args: uuid -> nessuna risposta immediata
    pub const SET_FILE_TRANSFER_REPORTING: u8 = 0xec; // args: [06 00 00 00 00 00] -> b3

    // Risposte
    pub const R_GENERIC: u8 = 0xb3;
    pub const R_NAME: u8 = 0xbc;
    pub const R_TIME: u8 = 0xbd;
    pub const R_FIRMWARE: u8 = 0xb8;
    pub const R_BATTERY: u8 = 0xba;
    pub const R_DIMENSION: u8 = 0xeb;
    pub const R_STROKES: u8 = 0xcf;
    pub const R_FILES_COUNT: u8 = 0xc2;
    pub const R_DOWNLOAD: u8 = 0xc8; // [be] = inizio download, [ed crc…] = fine lettura
    pub const R_BUTTON_SLATE: u8 = 0xe4; // pulsante premuto in registrazione (Slate)
    pub const R_BUTTON_INTUOS: u8 = 0x53; // idem (Intuos Pro)
}

/// Modalità del pad (argomento di SET_MODE).
pub mod mode {
    /// Tempo reale: i tratti escono subito.
    pub const LIVE: u8 = 0x00;
    /// Normale: i tratti si salvano in memoria (per lo scaricamento).
    pub const PAPER: u8 = 0x01;
    /// Live senza invio dati (usato da Tuhi per fermare il live).
    pub const IDLE: u8 = 0x02;
}

pub const ARGS_NONE: [u8; 1] = [0x00];
pub const ARGS_WIDTH: [u8; 2] = [0x03, 0x00];
pub const ARGS_HEIGHT: [u8; 2] = [0x04, 0x00];
pub const ARGS_TRANSFER_GATT: [u8; 6] = [0x06, 0x00, 0x00, 0x00, 0x00, 0x00];

/// Costruisce il frame di un comando. Senza argomenti usa `[0x00]`.
pub fn frame(opcode: u8, args: &[u8]) -> Vec<u8> {
    let args: &[u8] = if args.is_empty() { &ARGS_NONE } else { args };
    let mut v = Vec::with_capacity(2 + args.len());
    v.push(opcode);
    v.push(args.len() as u8);
    v.extend_from_slice(args);
    v
}

/// Una risposta completa del pad.
#[derive(Clone, Debug)]
pub struct Reply {
    pub opcode: u8,
    pub data: Vec<u8>,
}

/// Accumulatore del flusso Nordic (Tuhi: `nordic_answer` + `pop_next_message`).
#[derive(Default)]
pub struct NordicBuffer {
    buf: Vec<u8>,
}

impl NordicBuffer {
    pub fn push(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// Estrae il prossimo messaggio completo, se c'è.
    pub fn pop(&mut self) -> Option<Reply> {
        if self.buf.len() < 2 {
            return None;
        }
        let len = self.buf[1] as usize;
        if self.buf.len() < 2 + len {
            return None; // messaggio non ancora completo
        }
        let opcode = self.buf[0];
        let data = self.buf[2..2 + len].to_vec();
        self.buf.drain(..2 + len);
        Some(Reply { opcode, data })
    }

    pub fn clear(&mut self) {
        self.buf.clear();
    }
}

/// Errori di protocollo (Tuhi: DeviceError, AuthorizationError, …).
#[derive(Debug, thiserror::Error)]
pub enum ProtoError {
    #[error("il pad non riconosce questa app: va registrata di nuovo")]
    Authorization,
    #[error("il pad ha risposto con errore {}", device_error_name(*.0))]
    Device(u8),
    #[error("nessuna risposta dal pad al comando 0x{0:02x}")]
    MissingReply(u8),
    #[error("risposta inattesa 0x{got:02x} {data:02x?} (attesa 0x{expected:02x})")]
    UnexpectedReply { expected: u8, got: u8, data: Vec<u8> },
    #[error("dati non validi: {0}")]
    UnexpectedData(String),
    #[error("connessione Bluetooth: {0}")]
    Link(String),
}

/// Nomi dei codici di errore del dispositivo (Tuhi: DeviceError.ErrorCode).
pub fn device_error_name(code: u8) -> String {
    let name = match code {
        0x01 => "GENERAL_ERROR",
        0x02 => "INVALID_STATE",
        0x03 => "READ_ONLY_PARAM",
        0x04 => "COMMAND_NOT_SUPPORTED",
        0x07 => "AUTHORIZATION_ERROR",
        _ => "sconosciuto",
    };
    format!("0x{code:02x} ({name})")
}

/// Ora attuale per SET_TIME (Tuhi: strftime '%y%m%d%H%M%S' in UTC, BCD).
pub fn pack_time_now() -> [u8; 6] {
    use chrono::{Datelike, Timelike, Utc};
    let now = Utc::now();
    let bcd = |n: u32| (((n / 10) << 4) | (n % 10)) as u8;
    [
        bcd(now.year() as u32 % 100),
        bcd(now.month()),
        bcd(now.day()),
        bcd(now.hour()),
        bcd(now.minute()),
        bcd(now.second()),
    ]
}

/// CRC32 (IEEE) per verificare i dati scaricati (Tuhi: binascii.crc32).
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xffff_ffff;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (!(crc & 1)).wrapping_add(1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_vuoto_usa_00() {
        assert_eq!(frame(0xb9, &[]), vec![0xb9, 0x01, 0x00]);
    }

    #[test]
    fn buffer_separa_messaggi_attaccati_e_spezzati() {
        let mut b = NordicBuffer::default();
        b.push(&[0xb3, 0x01, 0x00, 0xba]);
        let r = b.pop().unwrap();
        assert_eq!((r.opcode, r.data), (0xb3, vec![0x00]));
        assert!(b.pop().is_none()); // 0xba incompleto
        b.push(&[0x02, 0x55, 0x01]);
        let r = b.pop().unwrap();
        assert_eq!((r.opcode, r.data), (0xba, vec![0x55, 0x01]));
    }

    #[test]
    fn crc32_noto() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    }
}
