//! Pagina «Impostazioni AI»: lettura e scrittura di `ai.toml` in JSON,
//! prova di un agente, elenco dei modelli, compattazione della chat.
//!
//! Le chiavi API non viaggiano mai verso la pagina: si dice solo se ci sono.
//! Una chiave nuova scritta nella pagina finisce nel Portachiavi di macOS e
//! in `ai.toml` resta solo `keychain:NOME`.

use crate::ai::{self, AiConfig, Msg, Provider, Task};
use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};

/// Segnaposto per una chiave scritta in chiaro in ai.toml (non la si mostra).
const HIDDEN: &str = "__CHIAVE_IN_CHIARO__";

fn has_plain_key(spec: &str) -> bool {
    spec.split('|')
        .map(str::trim)
        .filter(|a| !a.is_empty())
        .any(|a| !(a.starts_with("env:") || a.starts_with("keychain:") || a.starts_with("dsh:")))
}

fn shown_key(spec: &str) -> String {
    if has_plain_key(spec) {
        HIDDEN.into()
    } else {
        spec.to_string()
    }
}

fn err(e: impl std::fmt::Display) -> Value {
    json!({ "ok": false, "error": format!("{e:#}") })
}

/// Configurazione per la pagina (senza i valori delle chiavi).
pub fn to_json() -> Value {
    let cfg = match AiConfig::load() {
        Ok(c) => c,
        Err(e) => return err(e),
    };
    let providers: Vec<Value> = cfg
        .providers
        .iter()
        .map(|p| {
            let mut v = serde_json::to_value(p).unwrap_or_default();
            v["key"] = json!(shown_key(&p.key));
            v["key_ok"] = json!(ai::resolve_key(&p.key).is_some());
            v
        })
        .collect();
    let mut web = serde_json::to_value(&cfg.web).unwrap_or_default();
    web["key"] = json!(shown_key(&cfg.web.key));
    web["key_ok"] = json!(ai::resolve_key(&cfg.web.key).is_some_and(|k| !k.is_empty()));
    json!({
        "ok": true,
        "providers": providers,
        "web": web,
        "chat": cfg.chat,
        "ocr": cfg.ocr,
        "path": ai::config_path(),
        "keychain": cfg!(target_os = "macos"),
        "default_context": ai::DEFAULT_CONTEXT,
    })
}

/// Nome nel Portachiavi per la chiave di un agente.
fn keychain_name(provider: &str) -> String {
    let s: String = provider
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_uppercase() } else { '_' })
        .collect();
    let s = s.split('_').filter(|x| !x.is_empty()).collect::<Vec<_>>().join("_");
    format!("WACOM_AI_{s}_KEY")
}

/// Salva una chiave nel Portachiavi di macOS (sostituisce quella che c'è).
fn keychain_store(service: &str, value: &str) -> Result<()> {
    if !cfg!(target_os = "macos") {
        bail!("salvare le chiavi nel Portachiavi si può solo su macOS: usa una variabile d'ambiente (env:NOME)");
    }
    let user = std::env::var("USER").unwrap_or_else(|_| "wacom-slate".into());
    let out = std::process::Command::new("security")
        .args(["add-generic-password", "-U", "-a", &user, "-s", service, "-w", value])
        .output()?;
    if !out.status.success() {
        bail!("Portachiavi: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(())
}

/// Chiave per una prova: quella nuova scritta nella pagina, altrimenti quella
/// salvata (anche se in chiaro).
fn provider_for_test(body: &Value) -> Result<Provider> {
    let mut p: Provider = serde_json::from_value(body["provider"].clone()).map_err(|e| anyhow!("dati dell'agente: {e}"))?;
    let new_key = body["new_key"].as_str().unwrap_or("").trim();
    if !new_key.is_empty() {
        p.key = new_key.to_string();
    } else if p.key == HIDDEN {
        let old = AiConfig::load()?;
        p.key = old.providers.iter().find(|o| o.name == p.name).map(|o| o.key.clone()).unwrap_or_default();
    }
    if p.url.trim().is_empty() {
        bail!("manca l'indirizzo");
    }
    Ok(p)
}

/// `POST /api/config`: `{config, new_keys: {"<indice>": "sk-…"}, web_new_key}`.
pub fn save_from_json(body: &[u8]) -> Value {
    let r = (|| -> Result<()> {
        let v: Value = serde_json::from_slice(body)?;
        let old = AiConfig::load().unwrap_or_default();
        let mut c = v["config"].clone();
        // in ai.toml i blocchi si chiamano [[provider]]
        if let Some(list) = c.as_object_mut().and_then(|o| o.remove("providers")) {
            c["provider"] = list;
        }
        let mut cfg: AiConfig = serde_json::from_value(c).map_err(|e| anyhow!("dati non validi: {e}"))?;
        // le istruzioni non si modificano dalla pagina: restano quelle di prima
        cfg.ocr_prompt = old.ocr_prompt.clone();
        cfg.chat_prompt = old.chat_prompt.clone();
        for (i, p) in cfg.providers.iter_mut().enumerate() {
            p.name = p.name.trim().to_string();
            p.url = p.url.trim().trim_end_matches('/').to_string();
            if let Some(k) = v["new_keys"][i.to_string()].as_str().map(str::trim).filter(|k| !k.is_empty()) {
                let name = keychain_name(&p.name);
                keychain_store(&name, k)?;
                p.key = format!("keychain:{name}");
            } else if p.key == HIDDEN {
                p.key = old.providers.iter().find(|o| o.name == p.name).map(|o| o.key.clone()).unwrap_or_default();
            }
        }
        if let Some(k) = v["web_new_key"].as_str().map(str::trim).filter(|k| !k.is_empty()) {
            keychain_store("WACOM_AI_WEB_KEY", k)?;
            cfg.web.key = "keychain:WACOM_AI_WEB_KEY".into();
        } else if cfg.web.key == HIDDEN {
            cfg.web.key = old.web.key.clone();
        }
        cfg.chat.compact_at = cfg.chat.compact_at.min(95);
        cfg.save()
    })();
    match r {
        Ok(()) => json!({ "ok": true }),
        Err(e) => err(e),
    }
}

/// `POST /api/config/test`: una domanda breve a un agente (anche non salvato).
pub fn test_from_json(body: &[u8]) -> Value {
    let r = (|| -> Result<Value> {
        let v: Value = serde_json::from_slice(body)?;
        let mut p = provider_for_test(&v)?;
        p.enabled = true;
        p.timeout_s = p.timeout_s.min(120);
        let cfg = AiConfig { providers: vec![p.clone()], ..Default::default() };
        let t = std::time::Instant::now();
        let mut context = 0;
        let r = ai::run(&cfg, Task::Chat, &[Msg::new("user", "Rispondi solo con: OK")], &mut |e| {
            if let ai::Event::Trying { context: c, .. } = e {
                context = c;
            }
            true
        })?;
        Ok(json!({
            "ok": true,
            "model": r.model,
            "ms": t.elapsed().as_millis() as u64,
            "text": r.text.chars().take(80).collect::<String>(),
            "context": context,
        }))
    })();
    r.unwrap_or_else(|e| err(e.to_string().replace("nessun agente AI disponibile:\n• ", "")))
}

/// `POST /api/config/models`: i modelli che il server dichiara.
pub fn models_from_json(body: &[u8]) -> Value {
    let r = (|| -> Result<Value> {
        let v: Value = serde_json::from_slice(body)?;
        let p = provider_for_test(&v)?;
        let key = ai::resolve_key(&p.key).ok_or_else(|| anyhow!("chiave non trovata ({})", p.key))?;
        let models = ai::list_models(&p, &key)?;
        Ok(json!({
            "ok": true,
            "models": models.into_iter().map(|(id, c)| json!({ "id": id, "context": c })).collect::<Vec<_>>(),
        }))
    })();
    r.unwrap_or_else(err)
}

/// `POST /api/compact`: `{messages: [{role, content}]}` → riassunto.
pub fn compact_from_json(body: &[u8]) -> Value {
    let r = (|| -> Result<Value> {
        let v: Value = serde_json::from_slice(body)?;
        let history: Vec<(String, String)> = v["messages"]
            .as_array()
            .ok_or_else(|| anyhow!("manca messages"))?
            .iter()
            .map(|m| (m["role"].as_str().unwrap_or("user").to_string(), m["content"].as_str().unwrap_or("").to_string()))
            .collect();
        if history.is_empty() {
            bail!("niente da compattare");
        }
        let cfg = AiConfig::load()?;
        let r = ai::compact(&cfg, &history)?;
        Ok(json!({ "ok": true, "summary": r.text, "provider": r.provider, "model": r.model }))
    })();
    r.unwrap_or_else(err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chiavi_nascoste_e_nomi() {
        assert_eq!(shown_key("dsh:A | env:B"), "dsh:A | env:B");
        assert_eq!(shown_key("sk-segreta"), HIDDEN);
        assert_eq!(shown_key("env:A | sk-segreta"), HIDDEN);
        assert_eq!(keychain_name("Mac Studio · MTPLX"), "WACOM_AI_MAC_STUDIO_MTPLX_KEY");
    }
}
