//! Conversazioni della chat AI salvate sul Mac: `chats/<id>.json` (la
//! conversazione, scritta dalla pagina) e `chats/<id>.png` (la pagina allegata).
//! Le eliminate vanno in `chats/cestino/`.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub const DIR: &str = "chats";

/// Id valido: lettere, cifre e trattini (niente percorsi).
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn json_path(id: &str) -> PathBuf {
    Path::new(DIR).join(format!("{id}.json"))
}

pub fn png_path(id: &str) -> PathBuf {
    Path::new(DIR).join(format!("{id}.png"))
}

/// Rimette a posto una chat eliminata («Annulla»).
pub fn restore(id: &str) -> Result<(), String> {
    if !valid_id(id) {
        return Err("id non valido".into());
    }
    let bin = Path::new(DIR).join("cestino");
    for p in [json_path(id), png_path(id)] {
        let src = bin.join(p.file_name().unwrap());
        if src.exists() && !p.exists() {
            std::fs::rename(&src, &p).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Elenco per la barra laterale (senza i messaggi), dal più recente.
pub fn list() -> Value {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(DIR) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            let Some(id) = name.strip_suffix(".json") else { continue };
            if !valid_id(id) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(e.path()) else { continue };
            let Ok(v) = serde_json::from_str::<Value>(&text) else { continue };
            let msgs = v["messages"].as_array();
            out.push(json!({
                "id": id,
                "title": v["title"].as_str().unwrap_or("Nuova chat"),
                "project": v["project"].as_str().unwrap_or(""),
                "pinned": v["pinned"].as_bool().unwrap_or(false),
                "created": v["created"].as_u64().unwrap_or(0),
                "updated": v["updated"].as_u64().unwrap_or(0),
                "count": msgs.map(|m| m.len()).unwrap_or(0),
                "page": v["page"]["title"].as_str().unwrap_or(""),
            }));
        }
    }
    out.sort_by(|a, b| b["updated"].as_u64().cmp(&a["updated"].as_u64()));
    Value::Array(out)
}

pub fn get(id: &str) -> Option<Vec<u8>> {
    valid_id(id).then(|| std::fs::read(json_path(id)).ok()).flatten()
}

/// Salva la conversazione così come la manda la pagina (dev'essere JSON).
pub fn save(id: &str, body: &[u8]) -> Result<(), String> {
    if !valid_id(id) {
        return Err("id non valido".into());
    }
    let v: Value = serde_json::from_slice(body).map_err(|e| format!("JSON non valido: {e}"))?;
    if !v.is_object() {
        return Err("serve un oggetto JSON".into());
    }
    std::fs::create_dir_all(DIR).map_err(|e| e.to_string())?;
    let tmp = Path::new(DIR).join(format!(".{id}.tmp"));
    std::fs::write(&tmp, body).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, json_path(id)).map_err(|e| e.to_string())
}

/// Nel cestino (con la sua pagina), non cancellata davvero.
pub fn trash(id: &str) -> Result<(), String> {
    if !valid_id(id) {
        return Err("id non valido".into());
    }
    let bin = Path::new(DIR).join("cestino");
    std::fs::create_dir_all(&bin).map_err(|e| e.to_string())?;
    for p in [json_path(id), png_path(id)] {
        if p.exists() {
            let dst = bin.join(p.file_name().unwrap());
            std::fs::rename(&p, dst).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Allega alla conversazione la pagina mandata dall'app («Chat AI»,
/// «Discuti in chat»): l'immagine va in `chats/<id>.png`. Ritorna i dati da
/// mettere nella conversazione: `{title, markdown, image}`.
pub fn attach_current(id: &str) -> Result<Value, String> {
    if !valid_id(id) {
        return Err("id non valido".into());
    }
    let Some(c) = crate::ai::context() else {
        return Ok(json!(null));
    };
    std::fs::create_dir_all(DIR).map_err(|e| e.to_string())?;
    let image = match &c.png {
        Some(png) => {
            std::fs::write(png_path(id), png.as_slice()).map_err(|e| e.to_string())?;
            true
        }
        None => {
            let _ = std::fs::remove_file(png_path(id));
            false
        }
    };
    if !image && c.markdown.trim().is_empty() {
        return Ok(json!(null));
    }
    Ok(json!({ "title": c.title, "markdown": c.markdown, "image": image, "doc": c.doc, "rev": c.rev,
        "dir": c.dir.as_ref().map(|d| d.to_string_lossy().to_string()), "pagina": c.page }))
}

/// La cartella è un documento dell'app (sotto `documents/`)?
fn inside_documents(d: &std::path::Path) -> bool {
    match (std::fs::canonicalize(d), std::fs::canonicalize(crate::document::DOC_DIR)) {
        (Ok(d), Ok(root)) => d.starts_with(&root) && d != root,
        _ => false,
    }
}

/// Copia la pagina allegata (l'immagine) da una conversazione a un'altra
/// («Dirama»).
pub fn copy_page(from: &str, to: &str) -> Result<(), String> {
    if !valid_id(from) || !valid_id(to) {
        return Err("id non valido".into());
    }
    if png_path(from).exists() {
        std::fs::copy(png_path(from), png_path(to)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// La pagina di una conversazione, per mandarla al modello.
pub fn page_context(id: &str) -> Option<crate::ai::ChatContext> {
    let v: Value = serde_json::from_slice(&get(id)?).ok()?;
    let page = &v["page"];
    if !page.is_object() {
        return None;
    }
    let png = if page["image"].as_bool().unwrap_or(false) {
        std::fs::read(png_path(id)).ok().map(std::sync::Arc::new)
    } else {
        None
    };
    Some(crate::ai::ChatContext {
        title: page["title"].as_str().unwrap_or("").to_string(),
        png,
        markdown: page["markdown"].as_str().unwrap_or("").to_string(),
        doc: page["doc"].as_bool().unwrap_or(false),
        rev: 0,
        // il documento della conversazione, se c'è ancora, e solo dentro
        // documents/ (il file della chat non sceglie una cartella qualsiasi)
        dir: page["dir"].as_str().map(std::path::PathBuf::from).filter(|d| d.join("doc.json").exists() && inside_documents(d)),
        page: page["pagina"].as_u64().unwrap_or(0) as usize,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_sicuri() {
        assert!(valid_id("c-20261019-abc123"));
        for bad in ["", "../x", "a/b", "a.b", "con spazio"] {
            assert!(!valid_id(bad), "{bad}");
        }
    }
}
