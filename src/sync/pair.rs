//! Il QR dell'abbinamento: la desktop lo mostra a richiesta, per poco; il
//! telefono lo inquadra e conserva il segreto nel suo portachiavi. Dentro:
//! chi è la desktop, dove trovarla e il segreto di quel telefono.

use super::crypto::Secret;
use base64::Engine;
use serde::{Deserialize, Serialize};

/// Inizio del testo del QR (e versione).
pub const PREFIX: &str = "OPENSLATE-SYNC1:";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Pairing {
    /// Identificativo della desktop.
    #[serde(rename = "d")]
    pub desk_id: String,
    /// Nome del computer (da mostrare).
    #[serde(rename = "n")]
    pub desk_name: String,
    /// Indirizzi della desktop sulla rete di casa.
    #[serde(rename = "h")]
    pub hosts: Vec<String>,
    #[serde(rename = "p")]
    pub port: u16,
    /// Identificativo del telefono presso la desktop.
    #[serde(rename = "i")]
    pub peer_id: String,
    /// Il segreto, in base64 (URL).
    #[serde(rename = "k")]
    pub secret: String,
}

impl Pairing {
    pub fn key(&self) -> Result<Secret, String> {
        secret_from_b64(&self.secret)
    }
}

pub fn secret_from_b64(s: &str) -> Result<Secret, String> {
    let b = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(s).map_err(|_| "segreto non valido")?;
    b.try_into().map_err(|_| "segreto non valido".to_string())
}

pub fn secret_b64(k: &Secret) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(k)
}

/// Il testo del QR.
pub fn encode(p: &Pairing) -> String {
    let json = serde_json::to_vec(p).unwrap_or_default();
    format!("{PREFIX}{}", base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(json))
}

/// L'abbinamento dal testo di un QR.
pub fn decode(text: &str) -> Result<Pairing, String> {
    let body = text.trim().strip_prefix(PREFIX).ok_or("non è il QR per collegare il telefono al computer")?;
    let json = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(body).map_err(|_| "QR illeggibile")?;
    let p: Pairing = serde_json::from_slice(&json).map_err(|_| "QR illeggibile")?;
    p.key()?;
    if p.hosts.is_empty() || p.peer_id.is_empty() || p.desk_id.is_empty() {
        return Err("QR incompleto".into());
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qr_andata_e_ritorno() {
        let p = Pairing {
            desk_id: "d1".into(),
            desk_name: "Mac di casa".into(),
            hosts: vec!["192.168.1.20".into()],
            port: 47613,
            peer_id: "t1".into(),
            secret: secret_b64(&super::super::crypto::new_secret()),
        };
        let q = encode(&p);
        assert!(q.starts_with(PREFIX));
        assert_eq!(decode(&q).unwrap(), p);
        assert!(decode("OPENSLATE-AI1:xyz").is_err());
        assert!(decode(&format!("{PREFIX}e30")).is_err());
    }
}
