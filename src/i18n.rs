//! Lingue dell'interfaccia. Il testo nel codice è in italiano e fa da chiave:
//! le tabelle `src/i18n/<lingua>.json` dicono come si scrive nelle altre
//! lingue. Le chiavi con segnaposto (`"Salvato in {}"`, `"{n} pagine"`)
//! traducono anche i testi composti a runtime: il testo già formattato si
//! riconosce dal modello e i pezzi variabili passano nella traduzione.
//!
//! Lingua: quella del sistema (se c'è fra le nostre, altrimenti inglese) o
//! quella scelta in File → Opzioni (`ui_lang` in settings.conf).

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    It,
    En,
    Es,
    Fr,
    De,
    Uk,
    Zh,
    Ja,
    Ko,
}

impl Lang {
    pub const ALL: [Lang; 9] = [Lang::It, Lang::En, Lang::Es, Lang::Fr, Lang::De, Lang::Uk, Lang::Zh, Lang::Ja, Lang::Ko];

    /// Codice ISO (settings.conf, pagine web).
    pub fn code(self) -> &'static str {
        match self {
            Lang::It => "it",
            Lang::En => "en",
            Lang::Es => "es",
            Lang::Fr => "fr",
            Lang::De => "de",
            Lang::Uk => "uk",
            Lang::Zh => "zh",
            Lang::Ja => "ja",
            Lang::Ko => "ko",
        }
    }

    /// Nome nella lingua stessa (menu delle lingue).
    pub fn native(self) -> &'static str {
        match self {
            Lang::It => "Italiano",
            Lang::En => "English",
            Lang::Es => "Español",
            Lang::Fr => "Français",
            Lang::De => "Deutsch",
            Lang::Uk => "Українська",
            Lang::Zh => "中文",
            Lang::Ja => "日本語",
            Lang::Ko => "한국어",
        }
    }

    pub fn from_code(code: &str) -> Option<Lang> {
        let c = code.trim().to_ascii_lowercase();
        let c = c.split(['-', '_', '.']).next().unwrap_or("");
        Lang::ALL.into_iter().find(|l| l.code() == c)
    }

    /// Scrittura che chiede un carattere CJK (cinese, giapponese, coreano).
    pub fn cjk(self) -> bool {
        matches!(self, Lang::Zh | Lang::Ja | Lang::Ko)
    }

    fn table(self) -> Option<&'static str> {
        Some(match self {
            Lang::It => return None,
            Lang::En => include_str!("i18n/en.json"),
            Lang::Es => include_str!("i18n/es.json"),
            Lang::Fr => include_str!("i18n/fr.json"),
            Lang::De => include_str!("i18n/de.json"),
            Lang::Uk => include_str!("i18n/uk.json"),
            Lang::Zh => include_str!("i18n/zh.json"),
            Lang::Ja => include_str!("i18n/ja.json"),
            Lang::Ko => include_str!("i18n/ko.json"),
        })
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn lang() -> Lang {
    Lang::ALL[CURRENT.load(Ordering::Relaxed) as usize % Lang::ALL.len()]
}

pub fn set_lang(l: Lang) {
    let i = Lang::ALL.iter().position(|x| *x == l).unwrap_or(0);
    CURRENT.store(i as u8, Ordering::Relaxed);
    if let Ok(mut c) = CACHE.lock() {
        *c = None;
    }
}

/// La lingua da usare per `ui_lang` di settings.conf: `auto` = quella del
/// sistema se la conosciamo, altrimenti inglese.
pub fn resolve(setting: &str) -> Lang {
    match setting.trim() {
        "" | "auto" => system(),
        s => Lang::from_code(s).unwrap_or_else(system),
    }
}

/// Lingua del sistema (macOS: la prima delle lingue preferite).
pub fn system() -> Lang {
    sys_locale::get_locales().find_map(|l| Lang::from_code(&l)).unwrap_or(Lang::En)
}

/// Una data per esteso nella lingua dell'interfaccia: «18 ottobre 2026»,
/// «18 October 2026», «2026年10月18日»…
pub fn date_long(d: chrono::NaiveDate) -> String {
    use chrono::Datelike;
    let (day, m, y) = (d.day(), d.month0() as usize, d.year());
    const IT: [&str; 12] = ["gennaio", "febbraio", "marzo", "aprile", "maggio", "giugno", "luglio", "agosto", "settembre", "ottobre", "novembre", "dicembre"];
    const EN: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    const ES: [&str; 12] = ["enero", "febrero", "marzo", "abril", "mayo", "junio", "julio", "agosto", "septiembre", "octubre", "noviembre", "diciembre"];
    const FR: [&str; 12] = ["janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre"];
    const DE: [&str; 12] = ["Januar", "Februar", "März", "April", "Mai", "Juni", "Juli", "August", "September", "Oktober", "November", "Dezember"];
    // ucraino: il mese al genitivo («18 жовтня»)
    const UK: [&str; 12] = ["січня", "лютого", "березня", "квітня", "травня", "червня", "липня", "серпня", "вересня", "жовтня", "листопада", "грудня"];
    match lang() {
        Lang::It => format!("{day} {} {y}", IT[m]),
        Lang::En => format!("{day} {} {y}", EN[m]),
        Lang::Es => format!("{day} de {} de {y}", ES[m]),
        Lang::Fr => format!("{day} {} {y}", FR[m]),
        Lang::De => format!("{day}. {} {y}", DE[m]),
        Lang::Uk => format!("{day} {} {y} р.", UK[m]),
        Lang::Zh | Lang::Ja => format!("{y}年{}月{day}日", m + 1),
        Lang::Ko => format!("{y}년 {}월 {day}일", m + 1),
    }
}

// ---------------------------------------------------------------------------
// Tabelle
// ---------------------------------------------------------------------------

struct Table {
    exact: HashMap<String, String>,
    /// Le traduzioni stesse: un testo già tradotto non si ritraduce.
    done: std::collections::HashSet<String>,
    /// Chiavi con segnaposto, dalla più specifica (più testo fisso).
    patterns: Vec<(Pattern, String)>,
}

static TABLES: OnceLock<Vec<Option<Table>>> = OnceLock::new();
static CACHE: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

fn tables() -> &'static [Option<Table>] {
    TABLES.get_or_init(|| {
        let parse = |l: Lang| -> Option<HashMap<String, String>> {
            match serde_json::from_str::<HashMap<String, String>>(l.table()?) {
                Ok(m) => Some(m.into_iter().filter(|(_, v)| !v.is_empty()).collect()),
                Err(e) => {
                    tracing::warn!("traduzioni {}: {e}", l.code());
                    None
                }
            }
        };
        let en = parse(Lang::En).unwrap_or_default();
        Lang::ALL
            .iter()
            .map(|&l| {
                l.table()?;
                // quello che manca in una lingua si mostra in inglese
                let map: HashMap<String, String> = if l == Lang::En {
                    en.clone()
                } else {
                    let mut m = en.clone();
                    m.extend(parse(l).unwrap_or_default());
                    m
                };
                let mut patterns: Vec<(Pattern, String)> = map
                    .iter()
                    .filter(|(_, v)| !v.is_empty())
                    .filter_map(|(k, v)| Pattern::new(k).map(|p| (p, v.clone())))
                    .collect();
                patterns.sort_by_key(|(p, _)| std::cmp::Reverse(p.fixed_len));
                let exact: HashMap<String, String> = map.into_iter().filter(|(_, v)| !v.is_empty()).collect();
                let done = exact.values().map(|v| v.trim().to_string()).collect();
                Some(Table { exact, done, patterns })
            })
            .collect()
    })
}

fn table() -> Option<&'static Table> {
    let i = Lang::ALL.iter().position(|x| *x == lang())?;
    tables().get(i)?.as_ref()
}

/// Testo fisso: la sua traduzione (o lui stesso, in italiano o se manca).
pub fn t(it: &'static str) -> &'static str {
    match table().and_then(|tb| tb.exact.get(it)) {
        Some(s) => s.as_str(),
        None => it,
    }
}

/// Testo fisso con un contesto, quando la stessa parola italiana ha due
/// traduzioni («Annulla»: annulla l'ultima azione / chiudi senza salvare).
/// In tabella la chiave è `"contesto|testo"`.
pub fn tc(ctx: &str, it: &'static str) -> String {
    match table().and_then(|tb| tb.exact.get(&format!("{ctx}|{it}"))) {
        Some(s) => s.clone(),
        None => t(it).to_string(),
    }
}

/// Qualsiasi testo, anche composto a runtime: traduzione esatta, altrimenti
/// col modello che gli corrisponde, altrimenti com'è.
pub fn tr(s: &str) -> Cow<'_, str> {
    let Some(tb) = table() else { return Cow::Borrowed(s) };
    if s.is_empty() {
        return Cow::Borrowed(s);
    }
    if let Some(v) = tb.exact.get(s) {
        return Cow::Borrowed(v.as_str());
    }
    if tb.done.contains(s.trim()) {
        return Cow::Borrowed(s);
    }
    if let Ok(mut c) = CACHE.lock() {
        let c = c.get_or_insert_with(HashMap::new);
        if let Some(v) = c.get(s) {
            return Cow::Owned(v.clone());
        }
        let out = by_pattern(tb, s, 2).or_else(|| compound(tb, s)).unwrap_or_else(|| {
            missing(s);
            s.to_string()
        });
        if c.len() > 4000 {
            c.clear();
        }
        c.insert(s.to_string(), out.clone());
        return Cow::Owned(out);
    }
    Cow::Borrowed(s)
}

/// Col modello che corrisponde; anche i pezzi variabili si traducono, se sono
/// testi noti («Dal pad: {} nell'archivio» con dentro «2 pagine nuove»).
fn by_pattern(tb: &Table, s: &str, depth: u8) -> Option<String> {
    tb.patterns.iter().find_map(|(p, v)| {
        p.apply_with(s, v, &|c: &str| {
            if c.trim().is_empty() || !c.chars().any(|ch| ch.is_alphabetic()) {
                return None;
            }
            tb.exact.get(c).cloned().or_else(|| if depth > 0 { by_pattern(tb, c, depth - 1) } else { None })
        })
    })
}

/// Messaggi composti che nessuna chiave copre per intero: «⚠ …» e catene di
/// errori «a: b: c» (ogni pezzo si traduce da sé).
fn compound(tb: &Table, s: &str) -> Option<String> {
    let one = |p: &str| -> Option<String> { tb.exact.get(p).cloned().or_else(|| by_pattern(tb, p, 1)) };
    if let Some(rest) = s.strip_prefix("⚠ ") {
        let r = one(rest).or_else(|| compound(tb, rest)).unwrap_or_else(|| rest.to_string());
        return Some(format!("⚠ {r}"));
    }
    if !s.contains(": ") {
        return None;
    }
    let parts: Vec<&str> = s.split(": ").collect();
    let mut changed = false;
    let out: Vec<String> = parts
        .iter()
        .map(|p| match one(p) {
            Some(t) => {
                changed = true;
                t
            }
            None => p.to_string(),
        })
        .collect();
    changed.then(|| out.join(": "))
}

/// Per chi traduce: con `OPENSLATE_I18N_MISSING=<file>` i testi mostrati
/// senza traduzione finiscono in quel file (uno per riga, senza ripetizioni).
fn missing(s: &str) {
    static SEEN: Mutex<Option<std::collections::HashSet<String>>> = Mutex::new(None);
    let Some(path) = std::env::var_os("OPENSLATE_I18N_MISSING") else { return };
    if !s.chars().any(|c| c.is_alphabetic()) {
        return;
    }
    let mut g = SEEN.lock().unwrap();
    if g.get_or_insert_with(Default::default).insert(s.to_string()) {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(f, "{}", s.replace('\n', "\\n"));
        }
    }
}

/// Come `tr`, per una `String` già fatta (es. `format!`).
pub fn tf(s: String) -> String {
    match tr(&s) {
        Cow::Borrowed(_) => s,
        Cow::Owned(o) => o,
    }
}

/// La tabella della lingua attuale in JSON, per le pagine web.
pub fn table_json() -> String {
    match table() {
        Some(tb) => serde_json::to_string(&tb.exact).unwrap_or_else(|_| "{}".into()),
        None => "{}".into(),
    }
}

// ---------------------------------------------------------------------------
// Modelli con segnaposto
// ---------------------------------------------------------------------------

/// Una chiave come `"Salvati {n} file in {}"`: pezzi fissi e segnaposto.
struct Pattern {
    /// Pezzi fissi, uno in più dei segnaposto (anche vuoti).
    fixed: Vec<String>,
    /// Nomi dei segnaposto (`""` per `{}`, `"n"` per `{n}`, `":.0"` per `{:.0}`).
    holes: Vec<String>,
    fixed_len: usize,
}

/// Divide un testo in stile `format!`: pezzi fissi e segnaposto.
fn split_format(s: &str) -> (Vec<String>, Vec<String>) {
    let mut fixed = vec![String::new()];
    let mut holes = Vec::new();
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '{' if it.peek() == Some(&'{') => {
                it.next();
                fixed.last_mut().unwrap().push('{');
            }
            '}' if it.peek() == Some(&'}') => {
                it.next();
                fixed.last_mut().unwrap().push('}');
            }
            '{' => {
                let mut name = String::new();
                for d in it.by_ref() {
                    if d == '}' {
                        break;
                    }
                    name.push(d);
                }
                holes.push(name);
                fixed.push(String::new());
            }
            _ => fixed.last_mut().unwrap().push(c),
        }
    }
    (fixed, holes)
}

impl Pattern {
    fn new(key: &str) -> Option<Pattern> {
        if !key.contains('{') {
            return None;
        }
        let (fixed, holes) = split_format(key);
        if holes.is_empty() {
            return None;
        }
        // abbastanza testo fisso da non confondersi con altro
        let letters = fixed.iter().flat_map(|f| f.chars()).filter(|c| c.is_alphabetic()).count();
        if letters < 3 {
            return None;
        }
        let fixed_len = fixed.iter().map(|f| f.len()).sum();
        Some(Pattern { fixed, holes, fixed_len })
    }

    /// I pezzi variabili di `s`, se `s` è fatto con questo modello.
    fn captures(&self, s: &str) -> Option<Vec<String>> {
        let first = &self.fixed[0];
        let last = self.fixed.last().unwrap();
        if !s.starts_with(first.as_str()) || !s.ends_with(last.as_str()) || s.len() < first.len() + last.len() {
            return None;
        }
        let body = &s[first.len()..s.len() - last.len()];
        let mut caps = Vec::with_capacity(self.holes.len());
        let mut rest = body;
        for mid in &self.fixed[1..self.fixed.len() - 1] {
            let i = if mid.is_empty() { 0 } else { rest.find(mid.as_str())? };
            caps.push(rest[..i].to_string());
            rest = &rest[i + mid.len()..];
        }
        caps.push(rest.to_string());
        Some(caps)
    }

    /// Traduzione di `s` col modello tradotto `to` (stessi segnaposto).
    #[cfg(test)]
    fn apply(&self, s: &str, to: &str) -> Option<String> {
        self.apply_with(s, to, &|_| None)
    }

    /// Come `apply`; `inner` traduce i pezzi variabili quando li conosce.
    fn apply_with(&self, s: &str, to: &str, inner: &dyn Fn(&str) -> Option<String>) -> Option<String> {
        let caps: Vec<String> = self.captures(s)?.into_iter().map(|c| inner(&c).unwrap_or(c)).collect();
        let (tfixed, tholes) = split_format(to);
        let mut used = vec![false; caps.len()];
        let mut out = String::with_capacity(s.len() + 16);
        for (k, piece) in tfixed.iter().enumerate() {
            out.push_str(piece);
            if let Some(name) = tholes.get(k) {
                // stesso nome (anche ripetuto) o, per i `{}`, nello stesso ordine
                let j = (0..caps.len())
                    .find(|&j| !used[j] && self.holes[j] == *name && !name.is_empty())
                    .or_else(|| (0..caps.len()).find(|&j| self.holes[j] == *name && !name.is_empty()))
                    .or_else(|| (0..caps.len()).find(|&j| !used[j]))?;
                used[j] = true;
                out.push_str(&caps[j]);
            }
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modelli() {
        let p = Pattern::new("Salvati {n} file in {}").unwrap();
        assert_eq!(p.apply("Salvati 3 file in /tmp/x", "Saved {n} files to {}").unwrap(), "Saved 3 files to /tmp/x");
        let p = Pattern::new("«{t}» sulla lavagna: puoi scriverci sopra").unwrap();
        assert_eq!(p.apply("«Note» sulla lavagna: puoi scriverci sopra", "“{t}” on the board").unwrap(), "“Note” on the board");
        assert!(p.apply("altro testo", "x").is_none());
        // ordine diverso nella traduzione: si va per nome
        let p = Pattern::new("{a} di {b} pagine").unwrap();
        assert_eq!(p.apply("2 di 5 pagine", "{b}ページ中{a}").unwrap(), "5ページ中2");
        // parentesi raddoppiate
        let (f, h) = split_format("a {{b}} {c}");
        assert_eq!((f, h), (vec!["a {b} ".to_string(), String::new()], vec!["c".to_string()]));
        assert!(Pattern::new("{} {}").is_none());
    }

    #[test]
    fn tabelle_valide() {
        // ogni tabella si legge e i segnaposto coincidono con quelli della chiave
        for l in Lang::ALL {
            let Some(raw) = l.table() else { continue };
            let map: HashMap<String, String> = serde_json::from_str(raw).unwrap_or_else(|e| panic!("{}: {e}", l.code()));
            for (k, v) in &map {
                if v.is_empty() {
                    continue;
                }
                let (_, mut hk) = split_format(k);
                let (_, mut hv) = split_format(v);
                hk.sort();
                hv.sort();
                assert_eq!(hk, hv, "{}: segnaposto diversi in «{k}» → «{v}»", l.code());
            }
        }
    }

    #[test]
    fn stesse_chiavi() {
        // ogni lingua traduce tutti i testi dell'inglese (e niente di più)
        let keys = |l: Lang| -> std::collections::BTreeSet<String> {
            let map: HashMap<String, String> = serde_json::from_str(l.table().unwrap()).unwrap();
            map.into_keys().collect()
        };
        let en = keys(Lang::En);
        for l in Lang::ALL.iter().copied().filter(|l| l.table().is_some() && *l != Lang::En) {
            let k = keys(l);
            let manca: Vec<_> = en.difference(&k).take(5).collect();
            let in_piu: Vec<_> = k.difference(&en).take(5).collect();
            assert!(manca.is_empty() && in_piu.is_empty(), "{}: mancano {manca:?}, in più {in_piu:?}", l.code());
        }
    }

    #[test]
    fn codici() {
        assert_eq!(Lang::from_code("en-US"), Some(Lang::En));
        assert_eq!(Lang::from_code("zh_Hans_CN"), Some(Lang::Zh));
        assert_eq!(Lang::from_code("uk-UA"), Some(Lang::Uk));
        assert_eq!(Lang::from_code("pt-BR"), None);
        assert_eq!(resolve("ja"), Lang::Ja);
    }
}
