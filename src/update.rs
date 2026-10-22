//! Avviso degli aggiornamenti, come nel launcher di ArtCraft
//! (https://github.com/storytold/craft-launcher, MIT OR Apache-2.0): si
//! chiede a GitHub l'ultima release di OpenSlate e, se è più nuova di questa,
//! l'app lo dice con una barra sotto la barra multifunzione («Scarica»,
//! «Novità», «Salta questa versione»).
//!
//! L'aggiornamento lo fa l'app stessa (`selfupdate`): scarica, controlla e
//! installa la nuova versione al posto di questa, poi si riavvia.
//!
//! L'aggiornamento non tocca i dati: impostazioni (`settings.conf`), agenti AI
//! (`ai.toml`), archivio, documenti ed export stanno nella cartella dei dati
//! dell'utente (macOS: ~/Library/Application Support/OpenSlate), non dentro
//! l'app; le chiavi stanno nel Portachiavi. Sostituire l'app li lascia dove sono.

use anyhow::{anyhow, Result};
use serde_json::Value;
use std::time::Duration;

/// Il repository delle release.
pub const REPO: &str = "rmpaolillo/OpenSlate";
/// Ogni quanto si ricontrolla con l'app aperta.
pub const EVERY: Duration = Duration::from_secs(12 * 3600);

/// Una release più nuova di questa.
#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    /// Versione senza la «v» (0.2.5).
    pub version: String,
    /// Pagina della release su GitHub (novità).
    pub page: String,
    /// Il file da scaricare per questo sistema (o la pagina, se non c'è).
    pub download: String,
    /// Nome del file per questo sistema, se c'è (per l'aggiornamento dall'app).
    pub asset: Option<String>,
    /// Le impronte SHA-256 dei file (`SHA256SUMS.txt`), se la release le ha.
    pub sums: Option<String>,
}

/// La versione di questa app (per provare l'avviso: OPENSLATE_PRETEND_VERSION=0.1.0).
pub fn current() -> &'static str {
    static V: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    V.get_or_init(|| std::env::var("OPENSLATE_PRETEND_VERSION").unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string()))
}

/// L'app arriva da una release (DMG, .deb, zip…), non compilata dai sorgenti:
/// solo allora ha senso confrontarla con le release. Dai sorgenti la versione
/// di Cargo.toml (es. 0.3.0) è sempre «vecchia» rispetto ai tag: ci si aggiorna con
/// `git pull`. Si prova lo stesso con OPENSLATE_UPDATE_TEST=1.
pub fn from_release() -> bool {
    option_env!("OPENSLATE_RELEASE").is_some() || std::env::var_os("OPENSLATE_UPDATE_TEST").is_some()
}

/// `0.2.5`, `v0.2.5` → (0, 2, 5). Le pre-release (`0.3.0-beta`) non contano.
pub fn parse(v: &str) -> Option<(u64, u64, u64)> {
    let v = v.trim().trim_start_matches(['v', 'V']);
    if v.contains('-') {
        return None;
    }
    let v = v.split('+').next()?;
    let mut n = v.split('.').map(|x| x.parse::<u64>());
    let a = n.next()?.ok()?;
    let b = n.next().unwrap_or(Ok(0)).ok()?;
    let c = n.next().unwrap_or(Ok(0)).ok()?;
    Some((a, b, c))
}

/// `latest` è più nuova di `installed`?
pub fn is_newer(latest: &str, installed: &str) -> bool {
    matches!((parse(latest), parse(installed)), (Some(l), Some(i)) if l > i)
}

/// Chiede a GitHub l'ultima release. `Ok(None)`: questa è già l'ultima.
pub fn check() -> Result<Option<Release>> {
    // per le prove: la risposta di GitHub da un file
    if let Some(f) = std::env::var_os("OPENSLATE_UPDATE_JSON") {
        let json: Value = serde_json::from_slice(&std::fs::read(f)?)?;
        return Ok(from_json(&json, current(), std::env::consts::OS, deb_installed()));
    }
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(5))
        .timeout(Duration::from_secs(20))
        .user_agent(&format!("OpenSlate/{}", current()))
        .build();
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let resp = agent.get(&url).set("Accept", "application/vnd.github+json").set("X-GitHub-Api-Version", "2022-11-28").call();
    let json: Value = match resp {
        Ok(r) => r.into_json()?,
        // nessuna release pubblicata
        Err(ureq::Error::Status(404, _)) => return Ok(None),
        Err(ureq::Error::Status(403 | 429, _)) => return Err(anyhow!("GitHub: troppe richieste, riprovo più tardi")),
        Err(ureq::Error::Status(code, _)) => return Err(anyhow!("GitHub: HTTP {code}")),
        Err(e) => return Err(anyhow!("GitHub: {e}")),
    };
    Ok(from_json(&json, current(), std::env::consts::OS, deb_installed()))
}

/// La release dal JSON di GitHub, se è più nuova di `installed`.
pub fn from_json(j: &Value, installed: &str, os: &str, deb: bool) -> Option<Release> {
    if j["draft"].as_bool() == Some(true) || j["prerelease"].as_bool() == Some(true) {
        return None;
    }
    let tag = j["tag_name"].as_str()?;
    if !is_newer(tag, installed) {
        return None;
    }
    let page = j["html_url"].as_str().map(String::from).unwrap_or_else(|| format!("https://github.com/{REPO}/releases/tag/{tag}"));
    let assets: Vec<(&str, &str)> = j["assets"]
        .as_array()
        .map(|a| a.iter().filter_map(|x| Some((x["name"].as_str()?, x["browser_download_url"].as_str()?))).collect())
        .unwrap_or_default();
    let want = |name: &str| -> bool {
        match os {
            "macos" => name.ends_with(".dmg"),
            "linux" if deb => name.ends_with("_amd64.deb"),
            "linux" => name.ends_with("-linux-x86_64.tar.gz"),
            "windows" => name.ends_with(".zip") && name.contains("windows"),
            _ => false,
        }
    };
    // Mac: prima il file per questo processore (più piccolo), poi l'universale
    let mac_arch = if cfg!(target_arch = "aarch64") { "-macos-arm64.dmg" } else { "-macos-intel.dmg" };
    let found = if os == "macos" {
        assets
            .iter()
            .find(|(n, _)| n.ends_with(mac_arch))
            .or_else(|| assets.iter().find(|(n, _)| n.ends_with("-macos-universal.dmg")))
            .or_else(|| assets.iter().find(|(n, _)| want(n)))
    } else {
        assets.iter().find(|(n, _)| want(n))
    };
    let download = found.map(|(_, u)| u.to_string()).unwrap_or_else(|| page.clone());
    let asset = found.map(|(n, _)| n.to_string());
    let sums = assets.iter().find(|(n, _)| *n == "SHA256SUMS.txt").map(|(_, u)| u.to_string());
    Some(Release { version: tag.trim_start_matches(['v', 'V']).to_string(), page, download, asset, sums })
}

/// Linux: installata col pacchetto .deb (allora si propone il .deb).
pub fn deb_installed() -> bool {
    cfg!(target_os = "linux") && std::env::current_exe().is_ok_and(|p| p.starts_with("/usr"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn versioni() {
        assert_eq!(parse("v0.2.5"), Some((0, 2, 5)));
        assert_eq!(parse("0.3"), Some((0, 3, 0)));
        assert_eq!(parse("0.3.0-beta.1"), None);
        assert!(is_newer("v0.2.10", "0.2.9"));
        assert!(!is_newer("v0.2.3", "0.2.3"));
        assert!(!is_newer("v0.2.2", "0.2.3"));
        assert!(is_newer("v1.0.0", "0.9.9"));
    }

    #[test]
    fn release_di_github() {
        let j = json!({
            "tag_name": "v0.2.7", "html_url": "https://github.com/rmpaolillo/OpenSlate/releases/tag/v0.2.7",
            "draft": false, "prerelease": false,
            "assets": [
                {"name": "OpenSlate-0.2.7-macos-universal.dmg", "browser_download_url": "https://x/OpenSlate-0.2.7-macos-universal.dmg"},
                {"name": "openslate_0.2.7_amd64.deb", "browser_download_url": "https://x/openslate_0.2.7_amd64.deb"},
                {"name": "openslate-0.2.7-linux-x86_64.tar.gz", "browser_download_url": "https://x/openslate-0.2.7-linux-x86_64.tar.gz"},
                {"name": "openslate-0.2.7-windows-x86_64.zip", "browser_download_url": "https://x/openslate-0.2.7-windows-x86_64.zip"}
            ]
        });
        let r = from_json(&j, "0.2.5", "macos", false).unwrap();
        assert_eq!(r.version, "0.2.7");
        assert!(r.download.ends_with(".dmg"));
        assert!(from_json(&j, "0.2.5", "linux", true).unwrap().download.ends_with(".deb"));
        assert!(from_json(&j, "0.2.5", "linux", false).unwrap().download.ends_with(".tar.gz"));
        assert!(from_json(&j, "0.2.5", "windows", false).unwrap().download.ends_with(".zip"));
        // già aggiornata, o pre-release
        assert!(from_json(&j, "0.2.7", "macos", false).is_none());
        let mut beta = j.clone();
        beta["prerelease"] = json!(true);
        assert!(from_json(&beta, "0.2.5", "macos", false).is_none());
        // senza il file per il sistema: la pagina della release
        assert_eq!(from_json(&j, "0.2.5", "freebsd", false).unwrap().download, r.page);
        // Mac: il file per questo processore, anche se l'universale viene prima
        let mut three = j.clone();
        three["assets"].as_array_mut().unwrap().extend([
            json!({"name": "OpenSlate-0.2.7-macos-arm64.dmg", "browser_download_url": "https://x/OpenSlate-0.2.7-macos-arm64.dmg"}),
            json!({"name": "OpenSlate-0.2.7-macos-intel.dmg", "browser_download_url": "https://x/OpenSlate-0.2.7-macos-intel.dmg"}),
        ]);
        let mine = if cfg!(target_arch = "aarch64") { "macos-arm64.dmg" } else { "macos-intel.dmg" };
        let m = from_json(&three, "0.2.5", "macos", false).unwrap();
        assert!(m.asset.as_deref().is_some_and(|a| a.ends_with(mine)), "{:?}", m.asset);
        assert!(m.download.ends_with(mine));
    }
}
