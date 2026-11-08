//! Le impostazioni AI dal computer al telefono con un QR code: la desktop lo
//! mostra (Impostazioni AI → «Collega il telefono»), l'app Android lo
//! inquadra. Lo stesso file è nelle app per telefono (`mobile/core`).
//!
//! Il QR contiene i servizi con le loro chiavi API, compressi: niente passa in
//! rete. Per questo la desktop lo mostra solo a richiesta, per poco, con
//! l'avviso di non fotografarlo né condividerlo; il telefono salva le chiavi
//! cifrate nel suo portachiavi. Gli indirizzi «localhost» (LM Studio, Ollama
//! sul computer) diventano l'indirizzo del computer sulla rete di casa.

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

/// Inizio del testo del QR: riconosce i QR di OpenSlate (e la versione).
pub const PREFIX: &str = "OPENSLATE-AI1:";

/// Un servizio AI (nomi brevi: il QR resta piccolo e facile da inquadrare).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SyncProvider {
    #[serde(rename = "n")]
    pub name: String,
    #[serde(rename = "u")]
    pub url: String,
    #[serde(rename = "m", default, skip_serializing_if = "String::is_empty")]
    pub model: String,
    #[serde(rename = "k", default, skip_serializing_if = "String::is_empty")]
    pub key: String,
    #[serde(rename = "v", default = "yes")]
    pub vision: bool,
    #[serde(rename = "e", default = "yes")]
    pub enabled: bool,
    #[serde(rename = "t", default, skip_serializing_if = "std::ops::Not::not")]
    pub no_temperature: bool,
    #[serde(rename = "x", default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(rename = "s", default, skip_serializing_if = "is_zero")]
    pub timeout_s: u32,
}

fn yes() -> bool {
    true
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

/// Il testo del QR: prefisso + JSON compresso in base64 (URL-safe).
pub fn encode(list: &[SyncProvider]) -> String {
    let json = serde_json::to_vec(list).unwrap_or_default();
    let mut z = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::best());
    let _ = z.write_all(&json);
    let packed = z.finish().unwrap_or_default();
    format!("{PREFIX}{}", base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(packed))
}

/// I servizi dal testo di un QR. Errore se non è un QR di OpenSlate.
pub fn decode(text: &str) -> Result<Vec<SyncProvider>, String> {
    let body = text.trim().strip_prefix(PREFIX).ok_or("non è un QR delle impostazioni AI di OpenSlate")?;
    let packed = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(body).map_err(|e| format!("QR rovinato: {e}"))?;
    let mut json = Vec::new();
    // al massimo 1 MB: un QR non ne contiene di più
    flate2::read::DeflateDecoder::new(&packed[..]).take(1 << 20).read_to_end(&mut json).map_err(|e| format!("QR rovinato: {e}"))?;
    let list: Vec<SyncProvider> = serde_json::from_slice(&json).map_err(|e| format!("QR rovinato: {e}"))?;
    for p in &list {
        if !(p.url.starts_with("http://") || p.url.starts_with("https://")) {
            return Err(format!("indirizzo non valido per «{}»", p.name));
        }
    }
    Ok(list)
}

/// L'indirizzo visto dal telefono: `localhost` (il computer stesso) diventa
/// l'indirizzo del computer sulla rete di casa.
pub fn for_phone(url: &str, lan_ip: Option<std::net::IpAddr>) -> String {
    let Some(ip) = lan_ip else { return url.to_string() };
    let host = match ip {
        std::net::IpAddr::V4(v) => v.to_string(),
        std::net::IpAddr::V6(v) => format!("[{v}]"),
    };
    for local in ["localhost", "127.0.0.1", "[::1]", "0.0.0.0"] {
        for scheme in ["http://", "https://"] {
            if let Some(rest) = url.strip_prefix(&format!("{scheme}{local}")) {
                if rest.is_empty() || rest.starts_with(':') || rest.starts_with('/') {
                    return format!("{scheme}{host}{rest}");
                }
            }
        }
    }
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(name: &str, url: &str, key: &str) -> SyncProvider {
        SyncProvider {
            name: name.into(),
            url: url.into(),
            model: String::new(),
            key: key.into(),
            vision: true,
            enabled: true,
            no_temperature: false,
            max_tokens: None,
            timeout_s: 0,
        }
    }

    #[test]
    fn andata_e_ritorno() {
        let mut claude = p("Anthropic Claude", "https://api.anthropic.com/v1", "sk-ant-prova");
        claude.model = "claude-sonnet-4-6".into();
        claude.no_temperature = true;
        claude.max_tokens = Some(8192);
        let list = vec![claude, p("LM Studio", "http://192.168.1.5:1234/v1", "")];
        let qr = encode(&list);
        assert!(qr.starts_with(PREFIX));
        // piccolo abbastanza per un QR facile da inquadrare
        assert!(qr.len() < 400, "{}", qr.len());
        assert_eq!(decode(&qr).unwrap(), list);
        assert!(decode("https://esempio.it").is_err());
        assert!(decode(&format!("{PREFIX}!!!")).is_err());
    }

    #[test]
    fn localhost_diventa_il_computer() {
        let ip = Some("192.168.30.176".parse().unwrap());
        assert_eq!(for_phone("http://localhost:1234/v1", ip), "http://192.168.30.176:1234/v1");
        assert_eq!(for_phone("http://127.0.0.1:11434/v1", ip), "http://192.168.30.176:11434/v1");
        assert_eq!(for_phone("https://api.deepseek.com/v1", ip), "https://api.deepseek.com/v1");
        assert_eq!(for_phone("http://localhostia.it/v1", ip), "http://localhostia.it/v1");
        assert_eq!(for_phone("http://localhost:1234/v1", None), "http://localhost:1234/v1");
    }
}
