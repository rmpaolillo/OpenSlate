//! Pagine web dentro la finestra principale (WebView figlie): l'appunto
//! (Markdown con le formule), la chat AI nel riquadro a destra e le
//! impostazioni AI. Niente finestre separate.
//!
//! Le pagine arrivano dal server dell'app su 127.0.0.1 (`web::start_local`).
//! Parlano con l'app con `window.ipc.postMessage(JSON)` → [`Embeds::poll`];
//! l'app risponde con [`Embeds::eval`]. Su macOS (e Windows) è una WebView
//! figlia della finestra egui; su Linux una finestrella WebKitGTK senza bordi
//! tenuta sopra il punto giusto della finestra (le figlie X11 non si
//! disegnano con WebKitGTK recente).

use eframe::egui;
use std::sync::mpsc::{channel, Receiver, Sender};

/// Dove sta una pagina.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Slot {
    /// Appunto (al centro, al posto del foglio).
    Note,
    /// Chat AI (riquadro a destra).
    Chat,
    /// Impostazioni AI (al centro).
    Settings,
}

struct View {
    web: wry::WebView,
    #[cfg(target_os = "linux")]
    win: gtk::Window,
    shown: bool,
    rect: egui::Rect,
    #[cfg(target_os = "linux")]
    at: egui::Pos2,
}

pub(super) struct Embeds {
    views: Vec<(Slot, View)>,
    tx: Sender<(Slot, serde_json::Value)>,
    rx: Receiver<(Slot, serde_json::Value)>,
    /// Le WebView non si possono creare qui (es. Linux con Wayland): si usa
    /// la finestra separata di prima.
    pub failed: Option<String>,
    #[cfg(target_os = "linux")]
    gtk: bool,
    /// Angolo dell'area della finestra sullo schermo (punti; serve su Linux).
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    origin: egui::Pos2,
    /// Per svegliare la finestra quando una pagina manda un messaggio (un clic
    /// nella pagina non la ridisegna: il microfono deve partire subito).
    wake: Option<egui::Context>,
}

fn to_wry(r: egui::Rect) -> wry::Rect {
    wry::Rect {
        position: wry::dpi::LogicalPosition::new(r.min.x as f64, r.min.y as f64).into(),
        size: wry::dpi::LogicalSize::new(r.width().max(1.0) as f64, r.height().max(1.0) as f64).into(),
    }
}

fn open_external(url: &str) {
    if !(url.starts_with("http://") || url.starts_with("https://") || url.starts_with("mailto:")) {
        return;
    }
    crate::platform::open(url);
}

impl Embeds {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            views: Vec::new(),
            tx,
            rx,
            failed: None,
            #[cfg(target_os = "linux")]
            gtk: false,
            origin: egui::Pos2::ZERO,
            wake: None,
        }
    }

    pub fn has(&self, slot: Slot) -> bool {
        self.views.iter().any(|(s, _)| *s == slot)
    }

    fn get(&self, slot: Slot) -> Option<&View> {
        self.views.iter().find(|(s, _)| *s == slot).map(|(_, v)| v)
    }

    /// Crea la pagina se non c'è ancora. `origin`: `http://127.0.0.1:PORTA/`.
    pub fn ensure(&mut self, slot: Slot, frame: &eframe::Frame, origin: &str, url: &str, bg: egui::Color32) -> bool {
        if self.has(slot) {
            return true;
        }
        if self.failed.is_some() {
            return false;
        }
        #[cfg(target_os = "linux")]
        {
            if !self.gtk {
                if let Err(e) = gtk::init() {
                    self.failed = Some(format!("GTK: {e}"));
                    return false;
                }
                self.gtk = true;
            }
        }
        let tx = self.tx.clone();
        let wake = self.wake.clone();
        let o1 = origin.to_string();
        let o2 = origin.to_string();
        let builder = wry::WebViewBuilder::new()
            .with_url(url)
            .with_visible(false)
            .with_bounds(to_wry(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(10.0, 10.0))))
            .with_background_color((bg.r(), bg.g(), bg.b(), 255))
            .with_accept_first_mouse(true)
            .with_devtools(cfg!(debug_assertions))
            .with_ipc_handler(move |req| {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(req.body()) {
                    let _ = tx.send((slot, v));
                    if let Some(c) = &wake {
                        c.request_repaint();
                    }
                }
            })
            // le pagine dell'app restano qui; i link esterni nel browser
            .with_navigation_handler(move |u| {
                // niente data: (una pagina data: avrebbe il canale verso l'app)
                if u.starts_with(&o1) || u.starts_with("about:") || u.starts_with("blob:") {
                    true
                } else {
                    open_external(&u);
                    false
                }
            })
            .with_new_window_req_handler(move |u, _| {
                if !u.starts_with(&o2) {
                    open_external(&u);
                }
                wry::NewWindowResponse::Deny
            });
        #[cfg(target_os = "linux")]
        let built = {
            use gtk::prelude::*;
            use wry::WebViewBuilderExtUnix;
            let _ = frame;
            let win = gtk::Window::new(gtk::WindowType::Toplevel);
            win.set_decorated(false);
            win.set_skip_taskbar_hint(true);
            win.set_default_size(10, 10);
            let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
            win.add(&vbox);
            builder.with_visible(true).build_gtk(&vbox).map(|web| View { web, win, shown: false, rect: egui::Rect::NOTHING, at: egui::Pos2::ZERO })
        };
        #[cfg(not(target_os = "linux"))]
        let built = builder.build_as_child(frame).map(|web| View { web, shown: false, rect: egui::Rect::NOTHING });
        match built {
            Ok(v) => {
                self.views.push((slot, v));
                true
            }
            Err(e) => {
                tracing::warn!("WebView incorporata: {e}");
                self.failed = Some(e.to_string());
                false
            }
        }
    }

    /// Il contesto da svegliare quando arriva un messaggio da una pagina.
    pub fn set_wake(&mut self, ctx: &egui::Context) {
        if self.wake.is_none() {
            self.wake = Some(ctx.clone());
        }
    }

    /// Dov'è la finestra sullo schermo (serve su Linux).
    pub fn set_origin(&mut self, origin: Option<egui::Pos2>) {
        if let Some(o) = origin {
            self.origin = o;
        }
    }

    /// Mette la pagina nel rettangolo (punti egui) o la nasconde (`None`).
    #[cfg(target_os = "linux")]
    pub fn place(&mut self, slot: Slot, rect: Option<egui::Rect>) {
        use gtk::prelude::*;
        let origin = self.origin;
        let Some((_, v)) = self.views.iter_mut().find(|(s, _)| *s == slot) else { return };
        match rect {
            Some(r) if r.width() > 4.0 && r.height() > 4.0 => {
                let at = origin + r.min.to_vec2();
                if !v.shown || (v.at - at).length() > 0.5 || (v.rect.size() - r.size()).length() > 0.5 {
                    v.win.move_(at.x.round() as i32, at.y.round() as i32);
                    v.win.resize(r.width().round() as i32, r.height().round() as i32);
                    v.rect = r;
                    v.at = at;
                }
                if !v.shown {
                    v.win.show_all();
                    v.shown = true;
                }
            }
            _ => {
                if v.shown {
                    v.win.hide();
                    v.shown = false;
                }
            }
        }
    }

    /// Mette la pagina nel rettangolo (punti egui) o la nasconde (`None`).
    #[cfg(not(target_os = "linux"))]
    pub fn place(&mut self, slot: Slot, rect: Option<egui::Rect>) {
        let Some((_, v)) = self.views.iter_mut().find(|(s, _)| *s == slot) else { return };
        match rect {
            Some(r) if r.width() > 4.0 && r.height() > 4.0 => {
                if !v.shown || (v.rect.min - r.min).length() > 0.5 || (v.rect.size() - r.size()).length() > 0.5 {
                    let _ = v.web.set_bounds(to_wry(r));
                    v.rect = r;
                }
                if !v.shown {
                    let _ = v.web.set_visible(true);
                    v.shown = true;
                }
            }
            _ => {
                if v.shown {
                    let _ = v.web.set_visible(false);
                    v.shown = false;
                }
            }
        }
    }

    pub fn shown_rect(&self, slot: Slot) -> Option<egui::Rect> {
        self.get(slot).filter(|v| v.shown).map(|v| v.rect)
    }

    pub fn eval(&self, slot: Slot, js: &str) {
        if let Some(v) = self.get(slot) {
            let _ = v.web.evaluate_script(js);
        }
    }

    pub fn load(&self, slot: Slot, url: &str) {
        if let Some(v) = self.get(slot) {
            let _ = v.web.load_url(url);
        }
    }

    pub fn focus(&self, slot: Slot) {
        if let Some(v) = self.get(slot).filter(|v| v.shown) {
            let _ = v.web.focus();
        }
    }

    /// Clic sulla parte egui: la tastiera torna all'app.
    pub fn focus_parent(&self) {
        for (_, v) in &self.views {
            let _ = v.web.focus_parent();
        }
    }

    pub fn print(&self, slot: Slot) {
        if let Some(v) = self.get(slot) {
            let _ = v.web.print();
        }
    }

    /// Messaggi arrivati dalle pagine.
    pub fn poll(&mut self) -> Vec<(Slot, serde_json::Value)> {
        #[cfg(target_os = "linux")]
        if self.gtk {
            while gtk::events_pending() {
                gtk::main_iteration_do(false);
            }
        }
        self.rx.try_iter().collect()
    }
}

/// Stringa JavaScript (con le virgolette) per `eval`.
pub(super) fn js_str(s: &str) -> String {
    serde_json::Value::String(s.to_string()).to_string()
}
