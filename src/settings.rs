//! Impostazioni persistenti in `settings.conf` (righe `chiave = valore`),
//! modificabili anche a mano.

use crate::stroke::Rgb;

const FILE: &str = "settings.conf";
/// Vecchio formato (solo vista): letto se `settings.conf` non esiste.
const LEGACY_FILE: &str = "view_settings.txt";

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    /// Rotazione in passi di 90° e specchiature.
    pub rot: i32,
    pub flip_x: bool,
    pub flip_y: bool,
    /// Penna per i nuovi tratti.
    pub pen_width: f32,
    pub pen_color: Rgb,
    /// Gesto zig-zag = cancella.
    pub zigzag: bool,
    /// Evidenziatore: colore (chiaro: sta sotto l'inchiostro) e spessore.
    pub highlight_color: Rgb,
    pub highlight_width: f32,
    /// Tocco lungo col pennino = penna ↔ evidenziatore. Spento di serie: chi
    /// scrive guardando la carta, con una pausa a pennino appoggiato,
    /// passerebbe all'evidenziatore senza accorgersene.
    pub longpress_toggle: bool,
    /// Porta del server web per la diretta.
    pub web_port: u16,
    /// Avvia la diretta web all'apertura.
    pub web_auto: bool,
    /// OCR: "ai" (catena di agenti in ai.toml, Markdown + LaTeX) oppure
    /// "vision" (Apple, offline, solo testo).
    pub ocr_engine: String,
    /// Lingue per Apple Vision, separate da virgola.
    pub ocr_langs: String,
    /// App per aprire le trascrizioni .md («Apri in Typora»).
    pub md_app: String,
    /// Scaricando dal pad, cancella i disegni dopo averli salvati.
    pub sync_delete: bool,
    /// «Copia immagine»: ritaglia attorno alla scrittura (altrimenti foglio intero).
    pub copy_crop: bool,
    /// Ultima cartella scelta per salvare ed esportare (vuoto = `exports/`).
    pub save_dir: String,
    /// Lingua dell'interfaccia: `auto` (quella del sistema) o un codice
    /// (it, en, es, fr, de, uk, zh, ja, ko).
    pub ui_lang: String,
    /// «Copia immagine»: pixel del lato lungo.
    pub copy_size: u32,
    /// Archivio: altezza delle anteprime (punti).
    pub thumb_size: f32,
    /// Comando lanciato dopo ogni file prodotto (export, OCR); riceve il
    /// percorso come $1. Vuoto = niente.
    pub hook: String,
    /// Tema dell'interfaccia: sistema, chiaro o scuro.
    pub ui_theme: String,
    /// Righelli in cm attorno al foglio.
    pub rulers: bool,
    /// Chat AI a sinistra o a destra.
    pub chat_side: String,
    /// Larghezza della chat (frazione della finestra).
    pub chat_width: f32,
    /// Grandezza del testo nella chat AI (1 = normale; pizzico, Cmd +/−/0).
    pub chat_zoom: f32,
    /// Vista compatta: senza la barra dei menu e gli strumenti (resta la
    /// barra del titolo col pulsante per farli tornare).
    pub compact_ui: bool,
    /// Scrittura degli appunti: "" (stampata), "corsivo", "stampatello" (a mano).
    pub note_hand: String,
    /// Avvisa quando su GitHub c'è una versione nuova.
    pub update_check: bool,
    /// Versione nuova che l'utente ha scelto di saltare (non se ne avvisa più).
    pub update_skip: String,
    /// Le versioni nuove si scaricano e si installano da sole (si usano al
    /// prossimo avvio, o subito con «Riavvia ora»).
    pub update_auto: bool,
    /// Rigatura del foglio: bianco, righe, righe10, quadretti5, quadretti4, quadretti10.
    pub paper: String,
    /// Margini rossi a sinistra e a destra nei fogli a righe.
    pub paper_margin: bool,
    /// Chi legge la scrittura per la bella copia (un agente con la visione;
    /// vuoto = gli agenti di «Lettura dei fogli»).
    pub fair_agent: String,
    /// Con che scrittura la bella copia: chiaro, corsivo o stampatello (la
    /// tua, addestrata: `crate::campioni`), mano (la rete, solo dove c'è
    /// `mano/graves.bin`).
    pub fair_style: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            rot: 1, // pad nativo orizzontale usato in verticale
            flip_x: false,
            flip_y: false,
            pen_width: 2.0,
            pen_color: [20, 20, 20],
            zigzag: false,
            highlight_color: [255, 235, 90],
            highlight_width: 16.0,
            longpress_toggle: false,
            web_port: 8080,
            web_auto: false,
            ocr_engine: "ai".into(),
            ocr_langs: "it-IT,en-US".into(),
            md_app: "Typora".into(),
            sync_delete: true,
            copy_crop: true,
            save_dir: String::new(),
            ui_lang: "auto".into(),
            copy_size: 1600,
            thumb_size: 330.0,
            hook: String::new(),
            ui_theme: "sistema".into(),
            rulers: true,
            chat_side: "sinistra".into(),
            chat_width: 0.55,
            chat_zoom: 1.0,
            compact_ui: false,
            note_hand: String::new(),
            update_check: true,
            update_skip: String::new(),
            // si chiede prima: la barra azzurra offre «Aggiorna»
            update_auto: false,
            paper: "bianco".into(),
            paper_margin: true,
            fair_agent: String::new(),
            fair_style: "chiaro".into(),
        }
    }
}

pub fn color_to_hex(c: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

pub fn hex_to_color(s: &str) -> Option<Rgb> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}

impl Settings {
    pub fn load() -> Self {
        let mut s = Settings::default();
        if let Ok(text) = std::fs::read_to_string(FILE) {
            for line in text.lines() {
                let line = line.trim();
                if line.starts_with('#') {
                    continue;
                }
                if let Some((k, v)) = line.split_once('=') {
                    s.set(k.trim(), v.trim());
                }
            }
        } else if let Ok(text) = std::fs::read_to_string(LEGACY_FILE) {
            let p: Vec<&str> = text.split_whitespace().collect();
            if p.len() == 4 {
                s.rot = p[0].parse().unwrap_or(s.rot);
                s.flip_x = p[1] == "1";
                s.flip_y = p[2] == "1";
                s.pen_width = p[3].parse().unwrap_or(s.pen_width);
            }
        }
        s
    }

    fn set(&mut self, k: &str, v: &str) {
        let b = |v: &str| matches!(v, "1" | "true" | "si" | "sì" | "yes");
        match k {
            "rot" => self.rot = v.parse().unwrap_or(self.rot),
            "flip_x" => self.flip_x = b(v),
            "flip_y" => self.flip_y = b(v),
            "pen_width" => self.pen_width = v.parse().unwrap_or(self.pen_width),
            "pen_color" => self.pen_color = hex_to_color(v).unwrap_or(self.pen_color),
            // Vecchia chiave: ignorata (lo zig-zag automatico ora è spento di serie).
            "zigzag" => {}
            "zigzag_auto" => self.zigzag = b(v),
            "highlight_color" => self.highlight_color = hex_to_color(v).unwrap_or(self.highlight_color),
            "highlight_width" => self.highlight_width = v.parse().unwrap_or(self.highlight_width),
            // la chiave vecchia («longpress_toggle», acceso di serie) non vale più:
            // il tocco lungo si accende di nuovo solo a mano
            "longpress_tool" => self.longpress_toggle = b(v),
            "web_port" => self.web_port = v.parse().unwrap_or(self.web_port),
            "web_auto" => self.web_auto = b(v),
            // Vecchie chiavi dell'OCR: ora i modelli stanno in ai.toml.
            "ocr_backend" | "ocr_url" | "ocr_model" | "ocr_key" => {}
            "ocr_engine" => self.ocr_engine = v.into(),
            "ocr_langs" => self.ocr_langs = v.into(),
            "md_app" => self.md_app = v.into(),
            "sync_delete" => self.sync_delete = b(v),
            "copy_crop" => self.copy_crop = b(v),
            "save_dir" => self.save_dir = v.trim().into(),
            "ui_lang" => self.ui_lang = v.trim().into(),
            "thumb_size" => self.thumb_size = v.parse().unwrap_or(self.thumb_size),
            "copy_size" => self.copy_size = v.parse().unwrap_or(self.copy_size).clamp(256, 8000),
            "hook" => self.hook = v.into(),
            "ui_theme" => self.ui_theme = v.into(),
            "rulers" => self.rulers = b(v),
            "chat_side" => self.chat_side = if v.trim() == "destra" { "destra".into() } else { "sinistra".into() },
            "paper" => self.paper = v.into(),
            "paper_margin" => self.paper_margin = b(v),
            "fair_agent" => self.fair_agent = v.trim().to_string(),
            "fair_style" => self.fair_style = crate::campioni::Mode::parse(v).key().into(),
            // com'era prima: «la tua mano» accesa o spenta
            "fair_hand" if b(v) => self.fair_style = "mano".into(),
            "compact_ui" => self.compact_ui = b(v),
            "note_hand" => self.note_hand = crate::mdcraft::Hand::parse(v).key().into(),
            "chat_zoom" => self.chat_zoom = v.parse::<f32>().unwrap_or(self.chat_zoom).clamp(0.6, 2.5),
            "update_check" => self.update_check = b(v),
            "update_skip" => self.update_skip = v.trim().into(),
            // la vecchia chiave (installazione automatica di serie) non vale più:
            // l'aggiornamento si conferma col pulsante, a meno di sceglierlo di nuovo
            "update_auto" => {}
            "update_install_auto" => self.update_auto = b(v),
            "chat_width" => self.chat_width = v.parse::<f32>().unwrap_or(self.chat_width).clamp(0.2, 0.85),
            _ => tracing::warn!("settings.conf: chiave sconosciuta «{k}»"),
        }
    }

    pub fn save(&self) {
        let b = |v: bool| if v { 1 } else { 0 };
        let text = format!(
            "# Impostazioni di OpenSlate (modificabili a mano, app chiusa)\n\
             rot = {}\nflip_x = {}\nflip_y = {}\n\
             pen_width = {}\npen_color = {}\n\
             # zig-zag automatico = cancella (anche senza Gomma); attenzione alle firme\nzigzag_auto = {}\n\
             # evidenziatore; tocco lungo (punta ferma 1,5 s) = penna/evidenziatore, spento di serie\n\
             highlight_color = {}\nhighlight_width = {}\nlongpress_tool = {}\n\
             # diretta web\nweb_port = {}\nweb_auto = {}\n\
             # OCR: ai (agenti di ai.toml: Markdown + LaTeX) oppure vision (Apple, offline, solo testo)\n\
             ocr_engine = {}\nocr_langs = {}\n\
             # app per le trascrizioni .md\nmd_app = {}\n\
             # scaricando dal pad: cancella i disegni dopo averli salvati\nsync_delete = {}\n\
             # «Copia immagine»: 1 = ritaglia attorno alla scrittura; lato lungo in pixel\ncopy_crop = {}\ncopy_size = {}\n\
             # archivio: altezza delle anteprime\nthumb_size = {}\n\
             # comando lanciato dopo ogni file prodotto (riceve il percorso come $1)\nhook = {}\n\
             # aspetto: tema (sistema, chiaro, scuro) e righelli\nui_theme = {}\nrulers = {}\n\
             # chat AI: lato (sinistra, destra) e larghezza (frazione della finestra)\nchat_side = {}\nchat_width = {:.2}\nchat_zoom = {:.2}\n\
             # vista compatta: niente barra dei menu e strumenti\ncompact_ui = {}\n\
             # scrittura degli appunti: vuoto (stampata), corsivo, stampatello (a mano)\nnote_hand = {}\n\
             # rigatura del foglio: bianco, righe, righe10, quadretti5, quadretti4, quadretti10; margini rossi (solo righe)\npaper = {}\npaper_margin = {}\n\
             # bella copia: chi legge la scrittura (vuoto = gli agenti di «Lettura dei fogli»)\nfair_agent = {}\n\
             # bella copia: con che scrittura (chiaro, corsivo, stampatello: la tua, addestrata; mano: la rete, se c'è mano/graves.bin)\nfair_style = {}\n\
             # cartella proposta quando si salva o si esporta (l'ultima scelta; vuoto = exports/)\nsave_dir = {}\n\
             # lingua dell'interfaccia: auto (quella del sistema), it, en, es, fr, de, uk, zh, ja, ko\nui_lang = {}\n\
             # aggiornamenti: avvisa quando c'è una versione nuova su GitHub; versione saltata; installa da solo senza chiedere\nupdate_check = {}\nupdate_skip = {}\nupdate_install_auto = {}\n",
            self.rot,
            b(self.flip_x),
            b(self.flip_y),
            self.pen_width,
            color_to_hex(self.pen_color),
            b(self.zigzag),
            color_to_hex(self.highlight_color),
            self.highlight_width,
            b(self.longpress_toggle),
            self.web_port,
            b(self.web_auto),
            self.ocr_engine,
            self.ocr_langs,
            self.md_app,
            b(self.sync_delete),
            b(self.copy_crop),
            self.copy_size,
            self.thumb_size,
            self.hook,
            self.ui_theme,
            b(self.rulers),
            self.chat_side,
            self.chat_width,
            self.chat_zoom,
            b(self.compact_ui),
            self.note_hand,
            self.paper,
            b(self.paper_margin),
            self.fair_agent,
            self.fair_style,
            self.save_dir,
            self.ui_lang,
            b(self.update_check),
            self.update_skip,
            b(self.update_auto),
        );
        // un file scritto a metà (crash) farebbe ripartire coi valori di serie
        if let Err(e) = crate::web::write_atomic(std::path::Path::new(FILE), text.as_bytes()) {
            tracing::warn!("salvataggio {FILE}: {e}");
        }
    }
}

/// Lancia il comando `hook` (se impostato) sul file appena prodotto.
pub fn run_hook(hook: &str, path: &str) {
    if hook.trim().is_empty() {
        return;
    }
    let hook = hook.to_string();
    let path = path.to_string();
    std::thread::spawn(move || {
        match std::process::Command::new("sh")
            .arg("-c")
            .arg(&hook)
            .arg("hook")
            .arg(&path)
            .status()
        {
            Ok(s) if !s.success() => tracing::warn!("hook terminato con {s}"),
            Err(e) => tracing::warn!("hook non avviato: {e}"),
            _ => {}
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colori_hex() {
        assert_eq!(hex_to_color("#1a2b3c"), Some([0x1a, 0x2b, 0x3c]));
        assert_eq!(color_to_hex([0x1a, 0x2b, 0x3c]), "#1a2b3c");
        assert_eq!(hex_to_color("zz"), None);
    }
}
