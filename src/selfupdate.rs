//! OpenSlate si aggiorna da solo, come il launcher di ArtCraft
//! (https://github.com/storytold/craft-launcher, MIT OR Apache-2.0; qui
//! riscritto in piccolo): scarica il file della nuova release, ne controlla
//! l'impronta SHA-256 (`SHA256SUMS.txt` della release) e lo mette al posto di
//! questa copia, nel modo in cui era stata installata. Poi, quando si vuole,
//! un piccolo processo aspetta che l'app si chiuda e la riapre.
//!
//! | Installata come | Cambio |
//! |---|---|
//! | macOS `OpenSlate.app` | la nuova app dal DMG copiata al posto di questa (si può mentre gira) |
//! | Linux, archivio `.tar.gz` | i file nuovi al posto di quelli della cartella (il programma che gira resta valido) |
//! | Linux, pacchetto `.deb` | `pkexec dpkg -i` (il sistema chiede la password) |
//! | Windows, archivio `.zip` | dopo la chiusura un aiutante PowerShell copia i file e riapre l'app |
//!
//! Compilata dai sorgenti (`cargo run`) non si aggiorna da sola: `git pull`.
//!
//! I dati non si toccano: impostazioni, agenti AI, archivio, documenti, chat ed
//! export stanno nella cartella dei dati dell'utente, fuori dall'app; le chiavi
//! nel Portachiavi.

use crate::update::Release;
use anyhow::{anyhow, bail, Context, Result};
use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

/// Come è installata questa copia.
#[derive(Clone, Debug, PartialEq)]
pub enum Install {
    /// macOS: il pacchetto `OpenSlate.app`.
    MacApp(PathBuf),
    /// Linux: il pacchetto .deb (il programma in /usr/bin).
    Deb(PathBuf),
    /// Linux o Windows: la cartella estratta dall'archivio (il programma).
    Folder(PathBuf),
}

/// A che punto è l'aggiornamento (per la barra).
#[derive(Clone, Debug, PartialEq)]
pub enum Phase {
    Downloading { done: u64, total: u64 },
    Verifying,
    Installing,
}

/// Aggiornamento installato: il comando che riapre l'app dopo la chiusura.
#[derive(Clone, Debug, PartialEq)]
pub struct Ready {
    pub version: String,
    relaunch: Vec<OsString>,
    /// Windows: i file si cambiano alla chiusura (l'aiutante aspetta già).
    pub on_exit: bool,
}

/// Come è installata questa copia, se è una che sa aggiornare da sola.
pub fn current() -> Option<Install> {
    if !crate::update::from_release() {
        return None;
    }
    let exe = std::env::current_exe().ok()?;
    let exe = exe.canonicalize().unwrap_or(exe);
    detect(&exe)
}

fn detect(exe: &Path) -> Option<Install> {
    if cfg!(target_os = "macos") {
        let bundle = exe.ancestors().find(|p| p.extension().is_some_and(|e| e == "app"))?;
        return writable(bundle.parent()?).then(|| Install::MacApp(bundle.to_path_buf()));
    }
    if cfg!(target_os = "linux") && exe.starts_with("/usr") {
        return Some(Install::Deb(exe.to_path_buf()));
    }
    let dir = exe.parent()?;
    writable(dir).then(|| Install::Folder(exe.to_path_buf()))
}

/// Si può scrivere nella cartella? (si prova davvero)
fn writable(dir: &Path) -> bool {
    let probe = dir.join(format!(".openslate-scrittura-{}", std::process::id()));
    let ok = std::fs::write(&probe, b"").is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

/// Può aggiornarsi senza chiedere niente all'utente (il .deb vuole la password).
pub fn silent(install: &Install) -> bool {
    !matches!(install, Install::Deb(_))
}

/// Cartella dei file scaricati (nella cartella dei dati).
fn work_dir() -> PathBuf {
    PathBuf::from("updates")
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(60))
        .user_agent(&format!("OpenSlate/{}", crate::update::current()))
        .build()
}

/// Scarica, controlla e installa `release` al posto di questa copia.
pub fn prepare(install: &Install, release: &Release, phase: &mut dyn FnMut(Phase)) -> Result<Ready> {
    let name = release.asset.clone().ok_or_else(|| anyhow!("la release {} non ha il file per questo sistema", release.version))?;
    let sums_url = release.sums.clone().ok_or_else(|| anyhow!("la release {} non ha le impronte dei file (SHA256SUMS.txt)", release.version))?;
    let want = expected_sha256(&agent().get(&sums_url).call().map_err(|e| anyhow!("impronte: {e}"))?.into_string()?, &name)
        .ok_or_else(|| anyhow!("{name} non è fra le impronte della release"))?;
    std::fs::create_dir_all(work_dir())?;
    let file = work_dir().join(&name);
    download(&release.download, &file, phase)?;
    phase(Phase::Verifying);
    let got = sha256_file(&file)?;
    if got != want {
        let _ = std::fs::remove_file(&file);
        bail!("il file scaricato non corrisponde all'impronta della release: riprova");
    }
    phase(Phase::Installing);
    let res = match install {
        Install::MacApp(bundle) => install_mac(&file, bundle, &release.version),
        Install::Deb(exe) => install_deb(&file, exe),
        Install::Folder(exe) if cfg!(windows) => install_windows(&file, exe),
        Install::Folder(exe) => install_tar(&file, exe),
    };
    let _ = std::fs::remove_file(&file);
    let (relaunch, on_exit) = res?;
    Ok(Ready { version: release.version.clone(), relaunch, on_exit })
}

/// L'impronta di `name` in un `SHA256SUMS.txt` (`<hex>  <nome>`).
pub fn expected_sha256(sums: &str, name: &str) -> Option<String> {
    sums.lines().find_map(|l| {
        let mut it = l.split_whitespace();
        let (h, n) = (it.next()?, it.next()?);
        (n.trim_start_matches('*') == name && h.len() == 64).then(|| h.to_ascii_lowercase())
    })
}

fn sha256_file(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    let mut f = std::fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

fn download(url: &str, to: &Path, phase: &mut dyn FnMut(Phase)) -> Result<()> {
    let resp = agent().get(url).call().map_err(|e| anyhow!("download: {e}"))?;
    let total: u64 = resp.header("Content-Length").and_then(|v| v.parse().ok()).unwrap_or(0);
    let mut r = resp.into_reader();
    let part = to.with_extension("part");
    let mut out = std::fs::File::create(&part)?;
    let mut buf = vec![0u8; 1 << 16];
    let (mut done, mut last) = (0u64, 0u64);
    phase(Phase::Downloading { done, total });
    loop {
        let n = r.read(&mut buf)?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n])?;
        done += n as u64;
        if done - last > 256 * 1024 {
            last = done;
            phase(Phase::Downloading { done, total });
        }
    }
    out.flush()?;
    drop(out);
    if total > 0 && done != total {
        bail!("download interrotto ({done} di {total} byte)");
    }
    std::fs::rename(&part, to)?;
    Ok(())
}

fn run(cmd: &mut Command, what: &str) -> Result<String> {
    let out = cmd.output().with_context(|| format!("{what}: non avviato"))?;
    if !out.status.success() {
        bail!("{what}: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// macOS: apre il DMG, copia la nuova app accanto a questa, controlla che sia
/// la versione attesa e la scambia con questa (l'app che gira non se ne accorge).
fn install_mac(dmg: &Path, bundle: &Path, version: &str) -> Result<(Vec<OsString>, bool)> {
    let parent = bundle.parent().ok_or_else(|| anyhow!("l'app non è in una cartella"))?;
    let mount = std::env::temp_dir().join(format!("openslate-update-{}", std::process::id()));
    std::fs::create_dir_all(&mount)?;
    let attach = run(
        Command::new("/usr/bin/hdiutil").args(["attach", "-nobrowse", "-readonly", "-noautoopen", "-noverify", "-mountpoint"]).arg(&mount).arg(dmg),
        "apertura del DMG",
    );
    if let Err(e) = attach {
        let _ = std::fs::remove_dir(&mount);
        return Err(e);
    }
    let result = (|| -> Result<()> {
        let src = mount.join("OpenSlate.app");
        if !src.is_dir() {
            bail!("nel DMG non c'è OpenSlate.app");
        }
        let staging = parent.join(format!(".openslate-update-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&staging);
        std::fs::create_dir_all(&staging)?;
        let staged = staging.join("OpenSlate.app");
        // ditto tiene firma e collegamenti; senza gli attributi estesi del volume
        run(Command::new("/usr/bin/ditto").args(["--norsrc", "--noextattr", "--noacl"]).arg(&src).arg(&staged), "copia della nuova app")?;
        let plist = staged.join("Contents/Info.plist");
        let v = run(Command::new("/usr/libexec/PlistBuddy").args(["-c", "Print :CFBundleShortVersionString"]).arg(&plist), "versione della nuova app")?;
        if v.trim() != version {
            let _ = std::fs::remove_dir_all(&staging);
            bail!("la nuova app dice di essere la {}, non la {version}", v.trim());
        }
        if run(Command::new("/usr/bin/codesign").args(["--verify", "--deep"]).arg(&staged), "firma della nuova app").is_err() {
            let _ = std::fs::remove_dir_all(&staging);
            bail!("la firma della nuova app non è valida");
        }
        // lo scambio: la vecchia da parte, la nuova al suo posto
        let old = staging.join("OpenSlate-vecchia.app");
        std::fs::rename(bundle, &old).context("spostamento della versione attuale")?;
        if let Err(e) = std::fs::rename(&staged, bundle) {
            let _ = std::fs::rename(&old, bundle);
            bail!("sostituzione dell'app: {e}");
        }
        let _ = std::fs::remove_dir_all(&staging);
        Ok(())
    })();
    let _ = Command::new("/usr/bin/hdiutil").args(["detach", "-quiet"]).arg(&mount).output();
    let _ = std::fs::remove_dir(&mount);
    result?;
    Ok((vec!["/usr/bin/open".into(), bundle.as_os_str().to_owned()], false))
}

/// Linux .deb: il gestore dei pacchetti, con la password chiesta dal sistema.
fn install_deb(deb: &Path, exe: &Path) -> Result<(Vec<OsString>, bool)> {
    let deb = deb.canonicalize()?;
    run(Command::new("pkexec").arg("dpkg").arg("-i").arg(&deb), "installazione del pacchetto")
        .map_err(|e| anyhow!("{e}\n(serve pkexec per chiedere la password; altrimenti: sudo apt install {})", deb.display()))?;
    Ok((vec![exe.as_os_str().to_owned()], false))
}

/// Linux .tar.gz: i file nuovi al posto di quelli della cartella.
fn install_tar(tgz: &Path, exe: &Path) -> Result<(Vec<OsString>, bool)> {
    let dir = exe.parent().ok_or_else(|| anyhow!("il programma non è in una cartella"))?;
    let tmp = work_dir().join(format!("estratto-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp)?;
    run(Command::new("tar").arg("-xzf").arg(tgz).arg("-C").arg(&tmp), "apertura dell'archivio")?;
    // l'archivio ha una cartella sola: openslate-<versione>-linux-x86_64/
    let top = std::fs::read_dir(&tmp)?.flatten().map(|e| e.path()).find(|p| p.is_dir()).ok_or_else(|| anyhow!("archivio vuoto"))?;
    if !top.join("openslate").is_file() {
        bail!("nell'archivio non c'è il programma");
    }
    for e in std::fs::read_dir(&top)?.flatten() {
        let src = e.path();
        let dst = dir.join(e.file_name());
        // copiato accanto e rinominato: il programma che gira resta valido
        let staged = dir.join(format!(".{}.nuovo", e.file_name().to_string_lossy()));
        std::fs::copy(&src, &staged)?;
        std::fs::rename(&staged, &dst)?;
    }
    let _ = std::fs::remove_dir_all(&tmp);
    Ok((vec![exe.as_os_str().to_owned()], false))
}

/// Windows: il programma che gira non si può sostituire; un aiutante
/// PowerShell aspetta la chiusura, copia i file e riapre l'app.
fn install_windows(zip: &Path, exe: &Path) -> Result<(Vec<OsString>, bool)> {
    let dir = exe.parent().ok_or_else(|| anyhow!("il programma non è in una cartella"))?;
    // il file scaricato resta finché l'aiutante non l'ha usato
    let keep = work_dir().join("aggiornamento.zip");
    std::fs::copy(zip, &keep)?;
    let keep = keep.canonicalize()?;
    let tmp = std::env::temp_dir().join("openslate-update");
    let q = |p: &Path| format!("'{}'", p.display().to_string().replace('\'', "''"));
    let script = format!(
        "$p = {pid}; while (Get-Process -Id $p -ErrorAction SilentlyContinue) {{ Start-Sleep -Milliseconds 200 }}; \
         $t = {tmp}; Remove-Item -LiteralPath $t -Recurse -Force -ErrorAction SilentlyContinue; \
         Expand-Archive -LiteralPath {zip} -DestinationPath $t -Force; \
         $top = Get-ChildItem -LiteralPath $t | Select-Object -First 1; \
         Copy-Item -Path (Join-Path $top.FullName '*') -Destination {dir} -Recurse -Force; \
         Remove-Item -LiteralPath $t -Recurse -Force -ErrorAction SilentlyContinue; Remove-Item -LiteralPath {zip} -ErrorAction SilentlyContinue; \
         Start-Process -FilePath {exe}",
        pid = std::process::id(),
        tmp = q(&tmp),
        zip = q(&keep),
        dir = q(dir),
        exe = q(exe),
    );
    Command::new("powershell").args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script]).spawn().context("aiutante dell'aggiornamento")?;
    Ok((Vec::new(), true))
}

/// Riapre l'app appena questa si chiude (un `sh` che aspetta e poi la avvia).
/// Dopo, l'app deve chiudersi.
pub fn relaunch_after_exit(ready: &Ready) -> Result<()> {
    if ready.on_exit || ready.relaunch.is_empty() {
        return Ok(());
    }
    if cfg!(unix) {
        Command::new("/bin/sh")
            .arg("-c")
            .arg(r#"pid=$1; shift; i=0; while kill -0 "$pid" 2>/dev/null && [ $i -lt 600 ]; do sleep 0.1; i=$((i+1)); done; exec "$@""#)
            .arg("sh")
            .arg(std::process::id().to_string())
            .args(&ready.relaunch)
            .env_remove("OPENSLATE_PRETEND_VERSION")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .context("riavvio")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impronte() {
        let s = "3a1b…\nabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd  OpenSlate-0.2.4-macos-universal.dmg\n\
                 0000000000000000000000000000000000000000000000000000000000000000 *openslate_0.2.4_amd64.deb\n";
        assert_eq!(expected_sha256(s, "OpenSlate-0.2.4-macos-universal.dmg").unwrap().len(), 64);
        assert_eq!(expected_sha256(s, "openslate_0.2.4_amd64.deb").unwrap(), "0".repeat(64));
        assert!(expected_sha256(s, "altro").is_none());
    }

    #[test]
    fn archivio_linux_al_posto_dei_file() {
        if !cfg!(target_os = "linux") {
            return;
        }
        let base = std::env::temp_dir().join(format!("openslate-selfupdate-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let inst = base.join("installata");
        std::fs::create_dir_all(&inst).unwrap();
        std::fs::write(inst.join("openslate"), b"vecchio").unwrap();
        std::fs::write(inst.join("README.md"), b"vecchio").unwrap();
        // l'archivio della release: una cartella con il programma
        let src = base.join("src/openslate-9.9.9-linux-x86_64");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("openslate"), b"nuovo").unwrap();
        std::fs::write(src.join("README.md"), b"nuovo").unwrap();
        let tgz = base.join("rel.tar.gz");
        assert!(Command::new("tar").arg("-czf").arg(&tgz).arg("-C").arg(base.join("src")).arg("openslate-9.9.9-linux-x86_64").status().unwrap().success());
        let cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(&base).unwrap();
        let r = install_tar(&tgz, &inst.join("openslate"));
        std::env::set_current_dir(cwd).unwrap();
        let (relaunch, on_exit) = r.unwrap();
        assert!(!on_exit && relaunch[0] == inst.join("openslate").as_os_str());
        assert_eq!(std::fs::read(inst.join("openslate")).unwrap(), b"nuovo");
        assert_eq!(std::fs::read(inst.join("README.md")).unwrap(), b"nuovo");
        std::fs::remove_dir_all(&base).unwrap();
    }
}
