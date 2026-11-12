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
        "path": ai::config_path_full(),
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

/// Stesso server (nome o IP), anche con porta o percorso diversi?
fn same_host(a: &str, b: &str) -> bool {
    let host = |u: &str| {
        let r = u.trim().split("://").nth(1).unwrap_or("").split('/').next().unwrap_or("").to_ascii_lowercase();
        r.rsplit_once(':').map(|(h, _)| h.to_string()).unwrap_or(r)
    };
    host(a) == host(b)
}

/// Stesso indirizzo (senza la barra finale e gli spazi)?
fn same_url(a: &str, b: &str) -> bool {
    a.trim().trim_end_matches('/').eq_ignore_ascii_case(b.trim().trim_end_matches('/'))
}

/// La chiave salvata di un agente va solo al **suo** indirizzo: se nella
/// pagina l'indirizzo è cambiato, la chiave va scritta di nuovo (così una
/// pagina manomessa non può farla arrivare a un server qualsiasi).
fn stored_key_for(old: &AiConfig, name: &str, url: &str) -> Option<String> {
    old.providers.iter().find(|o| o.name == name && same_url(&o.url, url)).map(|o| o.key.clone())
}

/// Chiave per una prova: quella nuova scritta nella pagina, altrimenti quella
/// salvata (anche se in chiaro) se l'indirizzo è lo stesso.
fn provider_for_test(body: &Value) -> Result<Provider> {
    let mut p: Provider = serde_json::from_value(body["provider"].clone()).map_err(|e| anyhow!("dati dell'agente: {e}"))?;
    if p.url.trim().is_empty() {
        bail!("manca l'indirizzo");
    }
    let new_key = body["new_key"].as_str().unwrap_or("").trim();
    let old = AiConfig::load().unwrap_or_default();
    if !new_key.is_empty() {
        p.key = new_key.to_string();
    } else if p.key == HIDDEN {
        p.key = stored_key_for(&old, &p.name, &p.url).ok_or_else(|| anyhow!("l'indirizzo è cambiato: scrivi di nuovo la chiave"))?;
    } else if !p.key.trim().is_empty() {
        // un riferimento (keychain:, env:, dsh:) già usato da un agente salvato
        // vale solo col suo indirizzo
        let users: Vec<&Provider> = old.providers.iter().filter(|o| o.key == p.key).collect();
        if !users.is_empty() && !users.iter().any(|o| same_host(&o.url, &p.url)) {
            bail!("questa chiave è di un altro indirizzo: scrivila di nuovo per questo agente");
        }
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
                // indirizzo cambiato: la chiave salvata non lo segue
                p.key = stored_key_for(&old, &p.name, &p.url).unwrap_or_default();
            } else if !p.key.trim().is_empty() && !p.key.contains("__") {
                // un riferimento (keychain:, env:, dsh:) già usato da un agente salvato
                // non va a un server diverso (un'altra macchina); cambiare la porta sì
                let users: Vec<&Provider> = old.providers.iter().filter(|o| o.key == p.key).collect();
                if !users.is_empty() && !users.iter().any(|o| same_host(&o.url, &p.url)) {
                    p.key = String::new();
                }
            }
        }
        // i server MCP esterni si scrivono solo a mano in ai.toml: la pagina
        // non li vede e non li cambia (un comando da eseguire non arriva da qui)
        cfg.mcp = old.mcp.clone();
        // la voce si sceglie in ai.toml: la pagina la lascia com'è se non la manda
        if v["config"].get("voice").is_none() {
            cfg.voice = old.voice.clone();
        }
        if v["config"].get("speech").is_none() {
            cfg.speech = old.speech.clone();
        }
        if let Some(k) = v["web_new_key"].as_str().map(str::trim).filter(|k| !k.is_empty()) {
            keychain_store("WACOM_AI_WEB_KEY", k)?;
            cfg.web.key = "keychain:WACOM_AI_WEB_KEY".into();
        } else if cfg.web.key == HIDDEN {
            cfg.web.key = if same_url(&cfg.web.url, &old.web.url) { old.web.key.clone() } else { String::new() };
        } else if cfg.web.key == old.web.key && !same_host(&cfg.web.url, &old.web.url) {
            cfg.web.key = String::new();
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
        // chi legge le immagini si prova con un'immagine: è quello che fa la
        // trascrizione (un modello solo testo risponderebbe lo stesso al testo)
        let vision = p.vision;
        let msg = if vision {
            Msg::new("user", "Nell'immagine c'è un quadrato nero. Rispondi solo con: OK").with_image(std::sync::Arc::new(test_png()))
        } else {
            Msg::new("user", "Rispondi solo con: OK")
        };
        let task = if vision { Task::Ocr } else { Task::Chat };
        let r = ai::run(&cfg, task, &[msg], &mut |e| {
            if let ai::Event::Trying { context: c, .. } = e {
                context = c;
            }
            true
        })
        .map_err(|e| if vision { anyhow!("{e:#}\n(prova con un'immagine: se il modello non legge le immagini togli «legge le immagini» o scegli un altro modello)") } else { e })?;
        Ok(json!({
            "ok": true,
            "vision": vision,
            "model": r.model,
            "ms": t.elapsed().as_millis() as u64,
            "text": r.text.chars().take(80).collect::<String>(),
            "context": context,
        }))
    })();
    r.unwrap_or_else(|e| err(e.to_string().replace("nessun agente AI disponibile:\n• ", "")))
}

/// Un'immagine piccola per la prova degli agenti con visione: un quadrato nero.
fn test_png() -> Vec<u8> {
    let mut px = tiny_skia::Pixmap::new(96, 64).expect("pixmap");
    px.fill(tiny_skia::Color::WHITE);
    let mut paint = tiny_skia::Paint::default();
    paint.set_color(tiny_skia::Color::BLACK);
    if let Some(r) = tiny_skia::Rect::from_xywh(28.0, 12.0, 40.0, 40.0) {
        px.fill_rect(r, &paint, tiny_skia::Transform::identity(), None);
    }
    px.encode_png().unwrap_or_default()
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

    #[test]
    fn qr_per_il_telefono() {
        let list: Vec<crate::aisync::SyncProvider> = (0..6)
            .map(|i| crate::aisync::SyncProvider {
                name: format!("Servizio {i}"),
                url: format!("https://api.esempio{i}.com/v1"),
                model: "un-modello-abbastanza-lungo-8b".into(),
                key: "sk-0123456789abcdef0123456789abcdef0123456789abcdef".into(),
                vision: true,
                enabled: true,
                no_temperature: false,
                max_tokens: None,
                timeout_s: 180,
            })
            .collect();
        // anche con sei servizi e le chiavi il QR si disegna
        let svg = qr_svg(&crate::aisync::encode(&list)).unwrap();
        assert!(svg.starts_with("<?xml") || svg.contains("<svg"));
    }

    use super::*;

    #[test]
    fn chiavi_nascoste_e_nomi() {
        assert_eq!(shown_key("dsh:A | env:B"), "dsh:A | env:B");
        assert_eq!(shown_key("sk-segreta"), HIDDEN);
        assert_eq!(shown_key("env:A | sk-segreta"), HIDDEN);
        assert_eq!(keychain_name("Mac Studio · MTPLX"), "WACOM_AI_MAC_STUDIO_MTPLX_KEY");
    }
}

/// Il QR code (SVG) di un testo.
fn qr_svg(text: &str) -> Result<String, String> {
    let code = qrcode::QrCode::with_error_correction_level(text.as_bytes(), qrcode::EcLevel::L).map_err(|e| format!("QR: {e}"))?;
    Ok(code.render::<qrcode::render::svg::Color>().min_dimensions(360, 360).quiet_zone(true).build())
}

/// Un servizio come lo vede il telefono: chiave vera, «localhost» → il Mac.
fn for_phone(p: &ai::Provider, ip: Option<std::net::IpAddr>) -> crate::aisync::SyncProvider {
    crate::aisync::SyncProvider {
        name: p.name.clone(),
        url: crate::aisync::for_phone(&p.url, ip),
        model: if p.model.trim().is_empty() { p.vision_model.clone() } else { p.model.clone() },
        key: ai::resolve_key(&p.key).unwrap_or_default(),
        vision: p.vision,
        enabled: p.enabled,
        no_temperature: p.no_temperature,
        max_tokens: p.max_tokens,
        timeout_s: p.timeout_s as u32,
    }
}

fn phone_list(cfg: &AiConfig) -> Vec<crate::aisync::SyncProvider> {
    let ip = crate::web::lan_ip();
    cfg.providers.iter().map(|p| for_phone(p, ip)).collect()
}

/// I servizi arrivati dal telefono (sincronizzazione): l'elenco del telefono
/// prende il posto di quello di qui, ma di ogni servizio cambia solo quello
/// che il telefono ha cambiato (l'indirizzo «localhost», il modello per le
/// immagini, il riferimento alla chiave restano com'erano se uguali). Le
/// chiavi nuove vanno nel Portachiavi.
pub fn import_from_phone(list: Vec<crate::aisync::SyncProvider>) -> Result<()> {
    let mut cfg = AiConfig::load()?;
    merge_from_phone(&mut cfg, list, crate::web::lan_ip(), &|name, key| keychain_store(name, key))?;
    cfg.save()
}

fn merge_from_phone(
    cfg: &mut AiConfig,
    list: Vec<crate::aisync::SyncProvider>,
    ip: Option<std::net::IpAddr>,
    store_key: &dyn Fn(&str, &str) -> Result<()>,
) -> Result<()> {
    let mut out = Vec::new();
    for sp in list {
        let mut p = match cfg.providers.iter().find(|p| p.name == sp.name) {
            Some(p) => p.clone(),
            None => serde_json::from_value(json!({ "name": sp.name, "url": sp.url }))?,
        };
        let now = for_phone(&p, ip);
        if sp.url != now.url {
            p.url = sp.url.clone();
        }
        if sp.model != now.model {
            p.model = sp.model.clone();
        }
        p.vision = sp.vision;
        p.enabled = sp.enabled;
        p.no_temperature = sp.no_temperature;
        p.max_tokens = sp.max_tokens;
        if sp.timeout_s > 0 {
            p.timeout_s = sp.timeout_s as u64;
        }
        if sp.key != now.key && !sp.key.is_empty() {
            let name = keychain_name(&p.name);
            match store_key(&name, &sp.key) {
                Ok(()) => p.key = format!("keychain:{name}"),
                // fuori dal Mac niente Portachiavi: la chiave di qui resta
                Err(e) => tracing::warn!("chiave di «{}» dal telefono: {e}", p.name),
            }
        }
        out.push(p);
    }
    if out.is_empty() {
        bail!("nessun servizio AI dal telefono");
    }
    cfg.providers = out;
    Ok(())
}

/// Le impostazioni AI per la sincronizzazione col telefono (`sync::store::Virtual`):
/// fatte da `ai.toml` (con le chiavi vere, solo in memoria) e rifatte solo
/// quando il file cambia.
#[derive(Default)]
pub struct PhoneAi {
    cache: Option<(i64, Vec<u8>)>,
}

impl crate::sync::store::Virtual for PhoneAi {
    fn read(&mut self) -> Option<(Vec<u8>, i64)> {
        // al primo uso `ai.toml` non c'è ancora: lo crea la lettura
        if !std::path::Path::new(&ai::config_path()).exists() {
            AiConfig::load().ok()?;
        }
        let mtime = std::fs::metadata(ai::config_path())
            .ok()?
            .modified()
            .ok()?
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs() as i64;
        if let Some((t, d)) = &self.cache {
            if *t == mtime {
                return Some((d.clone(), mtime));
            }
        }
        let list = phone_list(&AiConfig::load().ok()?);
        let data = serde_json::to_vec(&list).ok()?;
        self.cache = Some((mtime, data.clone()));
        Some((data, mtime))
    }

    fn write(&mut self, data: &[u8]) -> std::io::Result<()> {
        let list: Vec<crate::aisync::SyncProvider> = serde_json::from_slice(data).map_err(std::io::Error::other)?;
        import_from_phone(list).map_err(|e| std::io::Error::other(format!("{e:#}")))?;
        self.cache = None;
        Ok(())
    }
}

/// Il QR code con i servizi AI per l'app sul telefono (`POST
/// /api/config/phone`, solo dal Mac): le chiavi sono quelle vere (dal
/// Portachiavi, dalle variabili d'ambiente…), «localhost» diventa l'indirizzo
/// del Mac sulla rete di casa. Vedi `aisync`.
pub fn phone_qr() -> Value {
    let cfg = match AiConfig::load() {
        Ok(c) => c,
        Err(e) => return json!({ "ok": false, "error": e.to_string() }),
    };
    let list = phone_list(&cfg);
    if list.is_empty() {
        return json!({ "ok": false, "error": "nessun agente da passare al telefono" });
    }
    let svg = match qr_svg(&crate::aisync::encode(&list)) {
        Ok(svg) => svg,
        Err(e) => return json!({ "ok": false, "error": e }),
    };
    let without_key: Vec<&str> =
        list.iter().filter(|p| p.key.is_empty() && p.url.starts_with("https://")).map(|p| p.name.as_str()).collect();
    json!({ "ok": true, "svg": svg, "names": list.iter().map(|p| &p.name).collect::<Vec<_>>(), "without_key": without_key })
}

#[cfg(test)]
mod sync_tests {
    use super::*;

    #[test]
    fn servizi_dal_telefono() {
        let mut cfg = AiConfig::parse(r#"
[[provider]]
name = "LM Studio"
url = "http://localhost:1234/v1"
vision_model = "qwen-vl"
key = "env:OPENSLATE_TEST_NON_ESISTE"

[[provider]]
name = "Vecchio"
url = "https://vecchio.example/v1"
"#).unwrap();
        let ip: Option<std::net::IpAddr> = Some("192.168.1.20".parse().unwrap());
        let mut list: Vec<crate::aisync::SyncProvider> = cfg.providers.iter().map(|p| for_phone(p, ip)).collect();
        // sul telefono: tolto «Vecchio», spento LM Studio, aggiunto un servizio con la chiave
        list.pop();
        list[0].enabled = false;
        list.push(crate::aisync::SyncProvider {
            name: "Nuovo".into(), url: "https://nuovo.example/v1".into(), model: "m".into(), key: "sk-segreta".into(),
            vision: true, enabled: true, no_temperature: false, max_tokens: None, timeout_s: 60,
        });
        let saved = std::cell::RefCell::new(Vec::new());
        merge_from_phone(&mut cfg, list, ip, &|n, k| { saved.borrow_mut().push((n.to_string(), k.to_string())); Ok(()) }).unwrap();
        let names: Vec<&str> = cfg.providers.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["LM Studio", "Nuovo"]);
        let lm = &cfg.providers[0];
        // l'indirizzo del Mac torna «localhost», il modello per le immagini resta, la chiave pure
        assert_eq!((lm.url.as_str(), lm.model.as_str(), lm.key.as_str(), lm.enabled), ("http://localhost:1234/v1", "", "env:OPENSLATE_TEST_NON_ESISTE", false));
        assert_eq!(cfg.providers[1].key, "keychain:WACOM_AI_NUOVO_KEY");
        assert_eq!(*saved.borrow(), [("WACOM_AI_NUOVO_KEY".to_string(), "sk-segreta".to_string())]);
    }
}
