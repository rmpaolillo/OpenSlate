//! Che cosa copiare, confrontando tre cose: i file di qui, quelli dell'altra
//! parte e com'erano all'ultima sincronizzazione (`base`, l'impronta che le
//! due parti avevano uguale). Così si capisce chi ha cambiato o tolto cosa.
//!
//! - cambiato da una parte sola → si copia dall'altra;
//! - tolto da una parte, e l'altra non l'ha cambiato → si toglie anche lì
//!   (nel cestino);
//! - cambiato da tutte e due → vince il più recente; l'altro va nel cestino.
//!   Due pagine nuove con lo stesso nome (fatte nello stesso secondo) si
//!   tengono tutte e due;
//! - mai sincronizzato prima (`base` vuota) → si mettono insieme le due parti,
//!   senza togliere niente.

use super::Manifest;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    /// Prendi il file dall'altra parte.
    Pull(String),
    /// Manda il file all'altra parte.
    Push(String),
    /// Togli il file qui (nel cestino).
    DelLocal(String),
    /// Togli il file dall'altra parte (nel suo cestino).
    DelRemote(String),
    /// Due file diversi con lo stesso nome, nati separati: quello di qui
    /// prende un nome nuovo e si tengono tutti e due.
    KeepBoth(String),
}

/// Le impronte all'ultima sincronizzazione.
pub type Base = BTreeMap<String, String>;

pub fn plan(local: &Manifest, remote: &Manifest, base: &Base) -> Vec<Op> {
    let paths: BTreeSet<&String> = local.keys().chain(remote.keys()).collect();
    let mut ops = Vec::new();
    for p in paths {
        let l = local.get(p);
        let r = remote.get(p);
        let (lh, rh) = (l.map(|e| e.hash.as_str()), r.map(|e| e.hash.as_str()));
        if lh == rh {
            continue;
        }
        let newer_remote = || r.map(|e| e.mtime).unwrap_or(0) > l.map(|e| e.mtime).unwrap_or(0);
        let op = match base.get(p).map(String::as_str) {
            None => match (l, r) {
                (Some(_), None) => Op::Push(p.clone()),
                (None, Some(_)) => Op::Pull(p.clone()),
                // nati separati con lo stesso nome
                _ if p.starts_with("lib/") && p.ends_with(".json") => Op::KeepBoth(p.clone()),
                _ if newer_remote() => Op::Pull(p.clone()),
                _ => Op::Push(p.clone()),
            },
            Some(b) => {
                let (l_same, r_same) = (lh == Some(b), rh == Some(b));
                match (l_same, r_same, l, r) {
                    // cambiato o tolto solo dall'altra parte
                    (true, false, _, Some(_)) => Op::Pull(p.clone()),
                    (true, false, _, None) => Op::DelLocal(p.clone()),
                    // cambiato o tolto solo qui
                    (false, true, Some(_), _) => Op::Push(p.clone()),
                    (false, true, None, _) => Op::DelRemote(p.clone()),
                    // cambiato da tutte e due: chi l'ha ancora vince su chi l'ha tolto
                    (_, _, None, Some(_)) => Op::Pull(p.clone()),
                    (_, _, Some(_), None) => Op::Push(p.clone()),
                    _ if newer_remote() => Op::Pull(p.clone()),
                    _ => Op::Push(p.clone()),
                }
            }
        };
        ops.push(op);
    }
    ops
}

/// La nuova base, dopo la sincronizzazione: i file uguali sulle due parti.
pub fn agreed(local: &Manifest, remote: &Manifest) -> Base {
    local
        .iter()
        .filter(|(p, e)| remote.get(*p).is_some_and(|r| r.hash == e.hash))
        .map(|(p, e)| (p.clone(), e.hash.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::Entry;
    use super::*;

    fn m(items: &[(&str, &str, i64)]) -> Manifest {
        items.iter().map(|(p, h, t)| (p.to_string(), Entry { hash: h.to_string(), mtime: *t, size: 1 })).collect()
    }
    fn b(items: &[(&str, &str)]) -> Base {
        items.iter().map(|(p, h)| (p.to_string(), h.to_string())).collect()
    }

    #[test]
    fn prima_volta_si_unisce_senza_togliere() {
        let l = m(&[("lib/a.json", "1", 10), ("notes/x.md", "x", 10)]);
        let r = m(&[("lib/b.json", "2", 10), ("notes/x.md", "y", 20)]);
        let ops = plan(&l, &r, &Base::new());
        assert_eq!(ops, vec![Op::Push("lib/a.json".into()), Op::Pull("lib/b.json".into()), Op::Pull("notes/x.md".into())]);
    }

    #[test]
    fn pagine_nate_separate_si_tengono_tutte() {
        let l = m(&[("lib/a.json", "1", 10)]);
        let r = m(&[("lib/a.json", "2", 10)]);
        assert_eq!(plan(&l, &r, &Base::new()), vec![Op::KeepBoth("lib/a.json".into())]);
    }

    #[test]
    fn cambi_e_cancellazioni_da_una_parte() {
        let base = b(&[("a", "1"), ("b", "1"), ("c", "1"), ("d", "1"), ("e", "1")]);
        // a cambiato qui, b cambiato là, c tolto qui, d tolto là, e uguale
        let l = m(&[("a", "2", 5), ("b", "1", 1), ("d", "1", 1), ("e", "1", 1)]);
        let r = m(&[("a", "1", 1), ("b", "3", 5), ("c", "1", 1), ("e", "1", 1)]);
        let ops = plan(&l, &r, &base);
        assert_eq!(
            ops,
            vec![Op::Push("a".into()), Op::Pull("b".into()), Op::DelRemote("c".into()), Op::DelLocal("d".into())]
        );
    }

    #[test]
    fn conflitti() {
        let base = b(&[("a", "1"), ("b", "1"), ("c", "1"), ("d", "1")]);
        // a: cambiato da tutte e due, là più recente; b: qui più recente;
        // c: tolto qui ma cambiato là → torna; d: tolti tutti e due → niente
        let l = m(&[("a", "2", 10), ("b", "2", 30)]);
        let r = m(&[("a", "3", 20), ("b", "3", 20), ("c", "4", 20)]);
        assert_eq!(plan(&l, &r, &base), vec![Op::Pull("a".into()), Op::Push("b".into()), Op::Pull("c".into())]);
    }

    #[test]
    fn base_nuova() {
        let l = m(&[("a", "1", 1), ("b", "2", 1)]);
        let r = m(&[("a", "1", 1), ("b", "3", 1), ("c", "1", 1)]);
        assert_eq!(agreed(&l, &r), b(&[("a", "1")]));
    }
}
