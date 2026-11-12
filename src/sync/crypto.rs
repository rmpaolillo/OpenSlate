//! Buste cifrate: ogni messaggio è `1 ‖ nonce (12 byte) ‖ cifrato`, con
//! ChaCha20-Poly1305 e il segreto del telefono. I «dati associati» (metodo,
//! percorso, ora, telefono…) sono firmati insieme al contenuto: una busta
//! spostata su un'altra richiesta non si apre.

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};

pub type Secret = [u8; 32];

const VERSION: u8 = 1;
pub const NONCE_LEN: usize = 12;

/// Un segreto nuovo, casuale.
pub fn new_secret() -> Secret {
    let mut k = [0u8; 32];
    getrandom::fill(&mut k).expect("generatore casuale del sistema");
    k
}

/// Chiude `plain` in una busta.
pub fn seal(key: &Secret, aad: &str, plain: &[u8]) -> Vec<u8> {
    let mut nonce = [0u8; NONCE_LEN];
    getrandom::fill(&mut nonce).expect("generatore casuale del sistema");
    let c = ChaCha20Poly1305::new(Key::from_slice(key));
    let ct = c.encrypt(Nonce::from_slice(&nonce), Payload { msg: plain, aad: aad.as_bytes() }).expect("cifratura");
    let mut out = Vec::with_capacity(1 + NONCE_LEN + ct.len());
    out.push(VERSION);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    out
}

/// Apre una busta; errore se è stata toccata, se è di un'altra richiesta o
/// se il segreto è un altro.
pub fn open(key: &Secret, aad: &str, data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < 1 + NONCE_LEN + 16 || data[0] != VERSION {
        return Err("busta non valida".into());
    }
    let c = ChaCha20Poly1305::new(Key::from_slice(key));
    c.decrypt(Nonce::from_slice(&data[1..1 + NONCE_LEN]), Payload { msg: &data[1 + NONCE_LEN..], aad: aad.as_bytes() })
        .map_err(|_| "busta non autentica".to_string())
}

/// Il nonce di una busta (per riconoscere le ripetizioni).
pub fn nonce_of(data: &[u8]) -> Option<[u8; NONCE_LEN]> {
    data.get(1..1 + NONCE_LEN)?.try_into().ok()
}

/// I dati associati di una richiesta.
pub fn req_aad(method: &str, path: &str, time: i64, peer: &str, mtime: i64) -> String {
    format!("OSYNC1\n{method}\n{path}\n{time}\n{peer}\n{mtime}")
}

/// I dati associati della risposta a una richiesta.
pub fn resp_aad(req_aad: &str) -> String {
    format!("R\n{req_aad}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn busta_si_apre_solo_giusta() {
        let k = new_secret();
        let b = seal(&k, "a", b"ciao");
        assert_eq!(open(&k, "a", &b).unwrap(), b"ciao");
        assert!(open(&k, "b", &b).is_err(), "altra richiesta");
        assert!(open(&new_secret(), "a", &b).is_err(), "altro segreto");
        let mut t = b.clone();
        *t.last_mut().unwrap() ^= 1;
        assert!(open(&k, "a", &t).is_err(), "busta toccata");
        assert_ne!(seal(&k, "a", b"ciao"), b, "nonce sempre nuovo");
    }
}
