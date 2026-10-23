//! Il tutor che parla: legge ad alta voce le risposte della chat.
//!
//! Di serie con le voci del sistema (niente da scaricare): `say` su macOS,
//! la sintesi vocale di Windows (SAPI, da PowerShell), `espeak-ng` o
//! `spd-say` su Linux. Oppure, se in `ai.toml` è scelto così (`[speech]`),
//! con un servizio in rete compatibile OpenAI (`/audio/speech`), che manda
//! l'audio da suonare. Una voce alla volta: una nuova ferma quella di prima.

use anyhow::{anyhow, bail, Context, Result};
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

// ---------------------------------------------------------------------------
// Che cosa si legge
// ---------------------------------------------------------------------------

/// Il testo di una risposta, pronto da leggere: senza Markdown, senza codice,
/// formule in forma leggibile, senza emoji.
pub fn speakable(md: &str) -> String {
    let text = crate::mdconv::speech_lines(md)
        .into_iter()
        .map(|l| {
            let l = l.trim().to_string();
            // ogni riga una frase: la voce fa la pausa
            if l.ends_with(['.', '!', '?', ':', ';', '…']) {
                l
            } else {
                format!("{l}.")
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    text.chars().filter(|c| !is_emoji(*c)).collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_emoji(c: char) -> bool {
    matches!(c as u32, 0x1F000..=0x1FAFF | 0x2600..=0x27BF | 0xFE0F | 0x200D | 0x2B50 | 0x2B55)
}

/// La lingua del testo, a occhio (per scegliere la voce): le scritture si
/// riconoscono dai caratteri, le lingue latine dalle parole più comuni.
pub fn detect_lang(text: &str) -> &'static str {
    let (mut cyr, mut han, mut kana, mut hangul, mut latin) = (0, 0, 0, 0, 0);
    for c in text.chars() {
        match c as u32 {
            0x0400..=0x04FF => cyr += 1,
            0x3040..=0x30FF => kana += 1,
            0xAC00..=0xD7AF => hangul += 1,
            0x4E00..=0x9FFF => han += 1,
            _ if c.is_ascii_alphabetic() => latin += 1,
            _ => {}
        }
    }
    if hangul > latin / 4 && hangul > 0 {
        return "ko";
    }
    if kana > 0 && kana + han > latin / 4 {
        return "ja";
    }
    if han > latin / 4 && han > 0 {
        return "zh";
    }
    if cyr > latin / 4 && cyr > 0 {
        return "uk";
    }
    const WORDS: [(&str, &[&str]); 5] = [
        ("it", &["il", "lo", "la", "di", "che", "e", "è", "un", "una", "per", "non", "sono", "del", "della", "con", "come", "anche", "questo", "perché"]),
        ("en", &["the", "and", "is", "of", "to", "a", "in", "that", "it", "you", "for", "this", "are", "with", "not", "be", "your"]),
        ("es", &["el", "la", "de", "que", "y", "en", "los", "las", "es", "un", "una", "por", "con", "para", "no", "se", "del"]),
        ("fr", &["le", "la", "les", "de", "des", "et", "est", "un", "une", "que", "en", "du", "pour", "pas", "vous", "dans", "il"]),
        ("de", &["der", "die", "das", "und", "ist", "nicht", "ein", "eine", "zu", "den", "mit", "von", "du", "ich", "sie", "es", "auf"]),
    ];
    let words: Vec<String> = text.split(|c: char| !c.is_alphabetic()).filter(|w| !w.is_empty()).map(|w| w.to_lowercase()).collect();
    let mut best = ("", 0usize);
    for (lang, list) in WORDS {
        let n = words.iter().filter(|w| list.contains(&w.as_str())).count();
        if n > best.1 {
            best = (lang, n);
        }
    }
    if best.1 == 0 {
        crate::i18n::lang().code()
    } else {
        best.0
    }
}

// ---------------------------------------------------------------------------
// Chi parla
// ---------------------------------------------------------------------------

/// Una voce in corso: un programma del sistema, o l'audio di un servizio.
enum Playing {
    Process(Child),
    Audio(Arc<AtomicBool>),
}

static CURRENT: Mutex<Option<(u64, Playing)>> = Mutex::new(None);
static NEXT: AtomicU64 = AtomicU64::new(1);

/// Ferma la voce, se ce n'è una.
pub fn stop() {
    if let Ok(mut g) = CURRENT.lock() {
        match g.take() {
            Some((_, Playing::Process(mut c))) => {
                let _ = c.kill();
                let _ = c.wait();
            }
            Some((_, Playing::Audio(stop))) => stop.store(true, Ordering::SeqCst),
            None => {}
        }
    }
}

/// Sta parlando adesso?
pub fn speaking() -> bool {
    CURRENT.lock().map(|g| g.is_some()).unwrap_or(false)
}

/// Legge `md` (una risposta in Markdown) e torna quando ha finito o è stata
/// fermata. Da chiamare in un thread suo.
pub fn speak(md: &str) -> Result<()> {
    let text = speakable(md);
    if text.trim().is_empty() {
        return Ok(());
    }
    let cfg = crate::ai::AiConfig::load()?;
    let o = &cfg.speech;
    let lang = if o.language.trim().is_empty() { detect_lang(&text).to_string() } else { o.language.trim().to_lowercase() };
    stop();
    let engine = o.engine.trim();
    if engine.is_empty() || engine.eq_ignore_ascii_case("system") || engine.eq_ignore_ascii_case("sistema") {
        system::speak(&text, &lang, o.voice.trim(), o.rate)
    } else {
        let Some(p) = cfg.providers.iter().find(|p| p.name == engine) else {
            bail!("l'agente «{engine}» scelto per la voce delle risposte in ai.toml non c'è");
        };
        remote(p, o, &text)
    }
}

fn register(p: Playing) -> u64 {
    let id = NEXT.fetch_add(1, Ordering::SeqCst);
    if let Ok(mut g) = CURRENT.lock() {
        *g = Some((id, p));
    }
    id
}

fn unregister(id: u64) {
    if let Ok(mut g) = CURRENT.lock() {
        if g.as_ref().is_some_and(|(i, _)| *i == id) {
            *g = None;
        }
    }
}

/// Aspetta la fine di un programma che parla (o che qualcuno lo fermi).
fn wait_process(id: u64) -> Result<()> {
    loop {
        std::thread::sleep(Duration::from_millis(80));
        let mut g = CURRENT.lock().map_err(|_| anyhow!("voce: errore interno"))?;
        match g.as_mut() {
            Some((i, Playing::Process(c))) if *i == id => match c.try_wait() {
                Ok(Some(st)) => {
                    *g = None;
                    if !st.success() {
                        bail!("la sintesi vocale del sistema si è fermata con un errore ({st})");
                    }
                    return Ok(());
                }
                Ok(None) => {}
                Err(e) => {
                    *g = None;
                    bail!("sintesi vocale: {e}");
                }
            },
            // fermata (o sostituita da un'altra)
            _ => return Ok(()),
        }
    }
}

/// Avvia un programma che legge il testo da stdin.
fn run_stdin(mut cmd: Command, text: &str) -> Result<()> {
    let mut child = cmd.stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).spawn()?;
    if let Some(mut si) = child.stdin.take() {
        si.write_all(text.as_bytes())?;
    }
    let id = register(Playing::Process(child));
    wait_process(id)
}

mod system {
    use super::*;

    /// macOS: `say`, con la voce migliore installata per la lingua.
    #[cfg(target_os = "macos")]
    pub fn speak(text: &str, lang: &str, voice: &str, rate: f32) -> Result<()> {
        let voice = if voice.is_empty() { best_mac_voice(lang) } else { Some(voice.to_string()) };
        let mut cmd = Command::new("/usr/bin/say");
        if let Some(v) = voice {
            cmd.arg("-v").arg(v);
        }
        if (rate - 1.0).abs() > 0.01 {
            cmd.arg("-r").arg(format!("{}", (185.0 * rate.clamp(0.5, 2.0)).round()));
        }
        run_stdin(cmd, text).context("sintesi vocale di macOS (say)")
    }

    /// La voce per `lang` fra quelle di `say -v ?`: prima le «Premium», poi le
    /// «Enhanced», poi le voci classiche, mai quelle buffe.
    #[cfg(target_os = "macos")]
    fn best_mac_voice(lang: &str) -> Option<String> {
        let out = Command::new("/usr/bin/say").args(["-v", "?"]).output().ok()?;
        let list = String::from_utf8_lossy(&out.stdout).to_string();
        super::pick_voice(&list, lang)
    }

    /// Windows: la sintesi vocale del sistema (SAPI), da PowerShell, con una
    /// voce della lingua se c'è.
    #[cfg(windows)]
    pub fn speak(text: &str, lang: &str, voice: &str, rate: f32) -> Result<()> {
        use std::os::windows::process::CommandExt;
        let culture = super::culture(lang);
        let pick = if voice.is_empty() {
            format!("try {{ $s.SelectVoiceByHints('NotSet', 'NotSet', 0, [Globalization.CultureInfo]'{culture}') }} catch {{}}")
        } else {
            format!("try {{ $s.SelectVoice('{}') }} catch {{}}", voice.replace('\'', "''"))
        };
        let r = ((rate.clamp(0.5, 2.0) - 1.0) * 10.0).round() as i32;
        let script = format!(
            "[Console]::InputEncoding = [Text.Encoding]::UTF8; Add-Type -AssemblyName System.Speech; \
             $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; {pick}; $s.Rate = {r}; \
             $s.Speak([Console]::In.ReadToEnd())"
        );
        let mut cmd = Command::new("powershell");
        cmd.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        run_stdin(cmd, text).context("sintesi vocale di Windows")
    }

    /// Linux: `espeak-ng` (o `espeak`), altrimenti `spd-say`.
    #[cfg(all(unix, not(target_os = "macos")))]
    pub fn speak(text: &str, lang: &str, voice: &str, rate: f32) -> Result<()> {
        let voice = if voice.is_empty() { lang.to_string() } else { voice.to_string() };
        let speed = format!("{}", (165.0 * rate.clamp(0.5, 2.0)).round());
        for prog in ["espeak-ng", "espeak"] {
            if which(prog) {
                let mut cmd = Command::new(prog);
                cmd.args(["-v", &voice, "-s", &speed, "--stdin"]);
                return run_stdin(cmd, text).with_context(|| format!("sintesi vocale ({prog})"));
            }
        }
        if which("spd-say") {
            let mut cmd = Command::new("spd-say");
            cmd.args(["-w", "-l", lang, "-r", &format!("{}", ((rate.clamp(0.5, 2.0) - 1.0) * 100.0).round())]).arg(text);
            let child = cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn()?;
            let id = register(Playing::Process(child));
            return wait_process(id);
        }
        bail!("per leggere le risposte su Linux serve espeak-ng (sudo apt install espeak-ng) o speech-dispatcher; oppure un servizio in rete in ai.toml ([speech])")
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    fn which(prog: &str) -> bool {
        std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(prog).is_file()))
    }
}

/// Fra le righe di `say -v ?` («Nome  it_IT  # frase»), la voce migliore
/// per la lingua.
#[cfg_attr(not(any(target_os = "macos", test)), allow(dead_code))]
fn pick_voice(list: &str, lang: &str) -> Option<String> {
    // voci «buffe» di macOS: non per leggere le risposte
    const NOVELTY: [&str; 16] = [
        "Bad News", "Bahh", "Bells", "Boing", "Bubbles", "Cellos", "Good News", "Jester", "Organ", "Superstar", "Trinoids", "Whisper", "Wobble",
        "Zarvox", "Albert", "Fred",
    ];
    let mut best: Option<(i32, String)> = None;
    for line in list.lines() {
        let Some((head, _)) = line.split_once('#') else { continue };
        let head = head.trim_end();
        // il codice della lingua è l'ultima parola prima del «#»
        let Some((name, code)) = head.rsplit_once(char::is_whitespace) else { continue };
        let (name, code) = (name.trim(), code.trim());
        if !code.to_lowercase().starts_with(&format!("{}_", lang.to_lowercase())) && code.to_lowercase() != lang.to_lowercase() {
            continue;
        }
        if NOVELTY.iter().any(|n| name.starts_with(n)) {
            continue;
        }
        let score = if name.contains("Premium") {
            3
        } else if name.contains("Enhanced") || name.contains("Migliorata") {
            2
        } else if name.contains('(') {
            0 // le voci «Eddy (Italiano (Italia))» e simili: simpatiche ma robotiche
        } else {
            1
        };
        if best.as_ref().is_none_or(|(s, _)| score > *s) {
            best = Some((score, name.to_string()));
        }
    }
    best.map(|(_, n)| n)
}

/// La cultura di Windows per la lingua (per scegliere la voce).
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
fn culture(lang: &str) -> &'static str {
    match lang {
        "it" => "it-IT",
        "en" => "en-US",
        "es" => "es-ES",
        "fr" => "fr-FR",
        "de" => "de-DE",
        "uk" => "uk-UA",
        "zh" => "zh-CN",
        "ja" => "ja-JP",
        "ko" => "ko-KR",
        _ => "en-US",
    }
}

// ---------------------------------------------------------------------------
// Servizio in rete (API compatibile OpenAI: /audio/speech)
// ---------------------------------------------------------------------------

fn remote(p: &crate::ai::Provider, o: &crate::ai::SpeechOpts, text: &str) -> Result<()> {
    let key = crate::ai::resolve_key(&p.key).ok_or_else(|| anyhow!("la chiave di «{}» non si trova ({})", p.name, p.key.trim()))?;
    let model = o.model.trim();
    if model.is_empty() {
        bail!("in ai.toml, [speech] model: scrivi il modello di sintesi vocale di «{}» (per esempio gpt-4o-mini-tts o tts-1)", p.name);
    }
    let voice = if o.voice.trim().is_empty() { "alloy" } else { o.voice.trim() };
    let stop_flag = Arc::new(AtomicBool::new(false));
    let id = register(Playing::Audio(stop_flag.clone()));
    let res = (|| -> Result<()> {
        // i servizi accettano testi di qualche migliaio di caratteri: a pezzi, per frasi
        for part in chunks(text, 3000) {
            if stop_flag.load(Ordering::SeqCst) {
                break;
            }
            let wav = fetch(p, &key, model, voice, o.rate, &part)?;
            if stop_flag.load(Ordering::SeqCst) {
                break;
            }
            play_wav(&wav, &stop_flag)?;
        }
        Ok(())
    })();
    unregister(id);
    res
}

fn fetch(p: &crate::ai::Provider, key: &str, model: &str, voice: &str, rate: f32, text: &str) -> Result<Vec<u8>> {
    let url = format!("{}/audio/speech", p.url.trim().trim_end_matches('/'));
    let mut body = serde_json::json!({ "model": model, "input": text, "voice": voice, "response_format": "wav" });
    if (rate - 1.0).abs() > 0.01 {
        body["speed"] = serde_json::json!(rate.clamp(0.5, 2.0));
    }
    let mut req = ureq::post(&url).timeout(Duration::from_secs(p.timeout_s.clamp(10, 600))).set("Content-Type", "application/json");
    if !key.is_empty() {
        req = req.set("Authorization", &format!("Bearer {key}"));
    }
    let r = match req.send_string(&body.to_string()) {
        Ok(r) => r,
        Err(ureq::Error::Status(code, r)) => {
            let msg: String = r.into_string().unwrap_or_default().chars().take(300).collect();
            bail!("«{}» non legge il testo ({code}): {msg}", p.name);
        }
        Err(e) => bail!("«{}» non risponde: {e}", p.name),
    };
    let mut out = Vec::new();
    std::io::Read::read_to_end(&mut r.into_reader().take(200 << 20), &mut out)?;
    Ok(out)
}

use std::io::Read as _;

/// Il testo in pezzi di al più `max` caratteri, tagliati fra una frase e l'altra.
fn chunks(text: &str, max: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for sentence in text.split_inclusive(['.', '!', '?', '…']) {
        if cur.chars().count() + sentence.chars().count() > max && !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
        if sentence.chars().count() > max {
            // una frase lunghissima: a pezzi per parole
            for w in sentence.split_inclusive(' ') {
                if cur.chars().count() + w.chars().count() > max && !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
                cur.push_str(w);
            }
        } else {
            cur.push_str(sentence);
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur);
    }
    out.into_iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

/// Un WAV (PCM a 16 o 32 bit, o float): campioni mono e frequenza.
fn decode_wav(b: &[u8]) -> Result<(Vec<f32>, u32)> {
    if b.len() < 12 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        bail!("il servizio non ha mandato un file WAV");
    }
    let (mut fmt, mut channels, mut rate, mut bits) = (1u16, 1u16, 0u32, 16u16);
    let mut i = 12;
    while i + 8 <= b.len() {
        let id = &b[i..i + 4];
        let len = u32::from_le_bytes(b[i + 4..i + 8].try_into().unwrap()) as usize;
        let start = i + 8;
        if id == b"fmt " && start + 16 <= b.len() {
            fmt = u16::from_le_bytes([b[start], b[start + 1]]);
            channels = u16::from_le_bytes([b[start + 2], b[start + 3]]).max(1);
            rate = u32::from_le_bytes(b[start + 4..start + 8].try_into().unwrap());
            bits = u16::from_le_bytes([b[start + 14], b[start + 15]]);
        } else if id == b"data" {
            // in streaming la lunghezza può mancare (0 o 0xFFFFFFFF): fino in fondo
            let end = if len == 0 || len == u32::MAX as usize || start + len > b.len() { b.len() } else { start + len };
            let data = &b[start..end];
            let ch = channels as usize;
            let samples: Vec<f32> = match (fmt, bits) {
                (1, 16) | (0xFFFE, 16) => data.chunks_exact(2).map(|s| i16::from_le_bytes([s[0], s[1]]) as f32 / 32768.0).collect(),
                (1, 32) | (0xFFFE, 32) => data.chunks_exact(4).map(|s| i32::from_le_bytes(s.try_into().unwrap()) as f32 / 2147483648.0).collect(),
                (3, 32) => data.chunks_exact(4).map(|s| f32::from_le_bytes(s.try_into().unwrap())).collect(),
                _ => bail!("formato WAV non gestito ({fmt}, {bits} bit)"),
            };
            let mono: Vec<f32> = samples.chunks(ch).map(|f| f.iter().sum::<f32>() / f.len() as f32).collect();
            if rate == 0 {
                bail!("WAV senza frequenza");
            }
            return Ok((mono, rate));
        }
        i = start + len + (len & 1);
    }
    bail!("WAV senza dati")
}

/// Suona un WAV sull'uscita audio predefinita, finché finisce o `stop`.
fn play_wav(wav: &[u8], stop: &AtomicBool) -> Result<()> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    let (mono, rate) = decode_wav(wav)?;
    let host = cpal::default_host();
    let dev = host.default_output_device().ok_or_else(|| anyhow!("nessuna uscita audio trovata"))?;
    let cfg = dev.default_output_config().map_err(|e| anyhow!("uscita audio: {e}"))?;
    if cfg.sample_format() != cpal::SampleFormat::F32 {
        bail!("uscita audio: formato {:?} non gestito", cfg.sample_format());
    }
    let out_rate = cfg.sample_rate();
    let channels = cfg.channels().max(1) as usize;
    let samples = Arc::new(crate::voice::resample(&mono, rate, out_rate));
    let pos = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (s2, p2) = (samples.clone(), pos.clone());
    let stream = dev
        .build_output_stream::<f32, _, _>(
            cfg.config(),
            move |out: &mut [f32], _| {
                let mut p = p2.load(Ordering::Relaxed);
                for frame in out.chunks_mut(channels) {
                    let v = s2.get(p).copied().unwrap_or(0.0);
                    frame.iter_mut().for_each(|x| *x = v);
                    p += 1;
                }
                p2.store(p, Ordering::Relaxed);
            },
            |e| tracing::warn!("uscita audio: {e}"),
            None,
        )
        .map_err(|e| anyhow!("uscita audio: {e}"))?;
    stream.play().map_err(|e| anyhow!("uscita audio: {e}"))?;
    while !stop.load(Ordering::SeqCst) && pos.load(Ordering::Relaxed) < samples.len() {
        std::thread::sleep(Duration::from_millis(30));
    }
    // un attimo perché l'ultimo pezzo esca dalle casse
    if !stop.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(150));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn testo_da_leggere() {
        let md = "## Esercizio\n\nIl risultato è **giusto** 👍: $x^2 = 4$ quindi $x = 2$\n\n```rust\nfn main() {}\n```\n\n- primo punto\n- secondo punto!\n\n| a | b |\n|---|---|\n| 1 | 2 |";
        let t = speakable(md);
        assert!(t.starts_with("Esercizio. Il risultato è giusto"), "{t}");
        assert!(!t.contains("fn main") && !t.contains("👍") && !t.contains("**") && !t.contains('$'), "{t}");
        assert!(t.contains("primo punto. secondo punto!") && t.contains("1, 2."), "{t}");
    }

    #[test]
    fn lingua_a_occhio() {
        assert_eq!(detect_lang("Il risultato è giusto, ma la seconda riga non torna perché manca un segno."), "it");
        assert_eq!(detect_lang("The result is right, but the second line is not because a sign is missing."), "en");
        assert_eq!(detect_lang("El resultado es correcto, pero la segunda línea no porque falta un signo."), "es");
        assert_eq!(detect_lang("Le résultat est juste, mais la deuxième ligne ne l'est pas."), "fr");
        assert_eq!(detect_lang("Das Ergebnis ist richtig, aber die zweite Zeile ist es nicht."), "de");
        assert_eq!(detect_lang("Результат правильний, але другий рядок ні."), "uk");
        assert_eq!(detect_lang("结果是对的，但是第二行不对。"), "zh");
        assert_eq!(detect_lang("結果は正しいですが、二行目は違います。"), "ja");
        assert_eq!(detect_lang("결과는 맞지만 두 번째 줄은 틀렸습니다."), "ko");
    }

    #[test]
    fn voce_del_mac() {
        let list = "Albert              en_US    # Hello! My name is Albert.\n\
                    Alice               it_IT    # Ciao! Mi chiamo Alice.\n\
                    Eddy (Italiano (Italia)) it_IT    # Ciao! Mi chiamo Eddy.\n\
                    Federica (Premium)  it_IT    # Ciao! Mi chiamo Federica.\n\
                    Luca (Enhanced)     it_IT    # Ciao! Mi chiamo Luca.\n\
                    Samantha            en_US    # Hello! My name is Samantha.\n";
        assert_eq!(pick_voice(list, "it").as_deref(), Some("Federica (Premium)"));
        assert_eq!(pick_voice(list, "en").as_deref(), Some("Samantha"));
        assert_eq!(pick_voice(list, "fr"), None);
        assert_eq!(culture("it"), "it-IT");
    }

    #[test]
    fn pezzi_e_wav() {
        let text = "Prima frase. Seconda frase! Terza frase?";
        assert_eq!(chunks(text, 20), vec!["Prima frase.", "Seconda frase!", "Terza frase?"]);
        assert_eq!(chunks(text, 1000), vec![text.to_string()]);
        let long = "parola ".repeat(100);
        assert!(chunks(&long, 50).iter().all(|c| c.chars().count() <= 50));
        // un WAV stereo a 16 bit di 24 kHz, con la lunghezza dei dati mancante (streaming)
        let mut w = Vec::new();
        w.extend_from_slice(b"RIFF\xff\xff\xff\xffWAVEfmt ");
        w.extend_from_slice(&16u32.to_le_bytes());
        w.extend_from_slice(&1u16.to_le_bytes());
        w.extend_from_slice(&2u16.to_le_bytes());
        w.extend_from_slice(&24_000u32.to_le_bytes());
        w.extend_from_slice(&96_000u32.to_le_bytes());
        w.extend_from_slice(&4u16.to_le_bytes());
        w.extend_from_slice(&16u16.to_le_bytes());
        w.extend_from_slice(b"data\xff\xff\xff\xff");
        for v in [16384i16, -16384, 0, 0] {
            w.extend_from_slice(&v.to_le_bytes());
        }
        let (mono, rate) = decode_wav(&w).unwrap();
        assert_eq!(rate, 24_000);
        assert_eq!(mono, vec![0.0, 0.0]);
        assert!(decode_wav(b"non sono un wav").is_err());
    }

    /// Il servizio in rete: la richiesta giusta a /audio/speech.
    #[test]
    fn richiesta_al_servizio() {
        use std::io::Read;
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let srv = std::thread::spawn(move || {
            let (mut c, _) = l.accept().unwrap();
            c.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut buf = vec![0u8; 65536];
            let mut req = Vec::new();
            loop {
                let n = c.read(&mut buf).unwrap();
                req.extend_from_slice(&buf[..n]);
                let t = String::from_utf8_lossy(&req).to_string();
                if t.contains("\r\n\r\n") && t.trim_end().ends_with('}') || n == 0 {
                    break;
                }
            }
            let body = b"RIFF\x24\x00\x00\x00WAVE";
            let _ = write!(c, "HTTP/1.1 200 OK\r\nContent-Type: audio/wav\r\nContent-Length: {}\r\n\r\n", body.len());
            let _ = c.write_all(body);
            String::from_utf8_lossy(&req).to_string()
        });
        let p: crate::ai::Provider = toml::from_str(&format!("name = \"OpenAI\"\nurl = \"http://127.0.0.1:{port}/v1\"\nkey = \"\"")).unwrap();
        let wav = fetch(&p, "", "gpt-4o-mini-tts", "alloy", 1.0, "Ciao Marco.").unwrap();
        assert!(wav.starts_with(b"RIFF"));
        let req = srv.join().unwrap();
        assert!(req.starts_with("POST /v1/audio/speech HTTP/1.1"), "{}", &req[..60]);
        assert!(req.contains("\"model\":\"gpt-4o-mini-tts\"") && req.contains("\"voice\":\"alloy\"") && req.contains("\"input\":\"Ciao Marco.\"") && req.contains("\"response_format\":\"wav\""));
    }
}
