//! Finestra dell'app che mostra una pagina del nostro server web dentro la
//! WebView di sistema (WKWebView su macOS): così gli appunti in Markdown, con
//! le formule disegnate da KaTeX, si leggono senza Typora né browser.
//!
//! Gira in un processo a parte (`openslate finestra URL`): la finestra
//! principale (egui) ha già il suo ciclo di eventi, e su macOS ce ne può
//! essere uno solo per processo.

use crate::i18n;
use anyhow::Result;

/// Apre la finestra e ritorna quando viene chiusa.
pub fn run(url: &str, title: &str) -> Result<()> {
    use tao::dpi::LogicalSize;
    use tao::event::{Event, WindowEvent};
    use tao::event_loop::{ControlFlow, EventLoopBuilder};
    use tao::window::WindowBuilder;

    // eventi propri: «vai a questa pagina» chiesto dall'app (vedi web::navigate)
    let event_loop = EventLoopBuilder::<String>::with_user_event().build();
    // Su macOS senza il menu «Modifica» Cmd+C / Cmd+V non funzionano.
    #[cfg(target_os = "macos")]
    let _menu = mac_menu();

    let window = WindowBuilder::new()
        .with_title(title)
        .with_inner_size(LogicalSize::new(1180.0, 860.0))
        .build(&event_loop)?;

    // Le pagine del server restano nella finestra; i link esterni (le fonti,
    // i siti citati) si aprono nel browser.
    let origin = origin_of(url);
    let origin2 = origin.clone();
    let origin3 = origin.clone();
    watch_nav(&origin, event_loop.create_proxy());
    let builder = wry::WebViewBuilder::new()
        .with_url(url)
        .with_navigation_handler(move |u| {
            if u.starts_with(&origin) || u.starts_with("about:") || u.starts_with("blob:") {
                true
            } else {
                open_external(&u);
                false
            }
        })
        .with_new_window_req_handler(move |u, _| {
            if !u.starts_with(&origin2) {
                open_external(&u);
            }
            wry::NewWindowResponse::Deny
        });

    #[cfg(not(any(target_os = "linux", target_os = "dragonfly", target_os = "freebsd", target_os = "netbsd", target_os = "openbsd")))]
    let webview = builder.build(&window)?;
    #[cfg(any(target_os = "linux", target_os = "dragonfly", target_os = "freebsd", target_os = "netbsd", target_os = "openbsd"))]
    let _webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window.default_vbox().ok_or_else(|| anyhow::anyhow!("finestra GTK senza contenitore"))?;
        builder.build_gtk(vbox)?
    };
    #[cfg(any(target_os = "linux", target_os = "dragonfly", target_os = "freebsd", target_os = "netbsd", target_os = "openbsd"))]
    let webview = _webview;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,
            Event::UserEvent(path) => {
                let _ = webview.load_url(&format!("{origin3}{}", path.trim_start_matches('/')));
                if path.starts_with("chat") {
                    window.set_title(i18n::t("Chat AI"));
                } else if path.starts_with("impostazioni") {
                    window.set_title(i18n::t("Impostazioni AI"));
                } else if path.starts_with("appunti") {
                    window.set_title(i18n::t("Appunti"));
                }
                window.set_focus();
            }
            _ => {}
        }
    })
}

/// Controlla ogni tanto se l'app chiede di andare a un'altra pagina.
fn watch_nav(origin: &str, proxy: tao::event_loop::EventLoopProxy<String>) {
    let url = format!("{origin}api/nav");
    std::thread::spawn(move || {
        let agent = ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(2)).build();
        let mut seen: Option<u64> = None;
        loop {
            if let Ok(v) = agent.get(&url).call().and_then(|r| Ok(r.into_json::<serde_json::Value>()?)) {
                let rev = v["rev"].as_u64().unwrap_or(0);
                match seen {
                    None => seen = Some(rev),
                    Some(s) if rev > s => {
                        seen = Some(rev);
                        if proxy.send_event(v["path"].as_str().unwrap_or("").to_string()).is_err() {
                            return;
                        }
                    }
                    _ => {}
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(600));
        }
    });
}

/// `http://localhost:8080/appunti?f=x` → `http://localhost:8080/`
fn origin_of(url: &str) -> String {
    let rest = url.split_once("://").map(|(_, r)| r).unwrap_or(url);
    let host = rest.split('/').next().unwrap_or("");
    let scheme = url.split_once("://").map(|(s, _)| s).unwrap_or("http");
    format!("{scheme}://{host}/")
}

fn open_external(url: &str) {
    if !(url.starts_with("http://") || url.starts_with("https://") || url.starts_with("mailto:")) {
        return;
    }
    crate::platform::open(url);
}

#[cfg(target_os = "macos")]
fn mac_menu() -> Option<muda::Menu> {
    use muda::{Menu, PredefinedMenuItem, Submenu};
    let menu = Menu::new();
    let app = Submenu::new(i18n::t("Appunti"), true);
    let edit = Submenu::new(i18n::t("Modifica"), true);
    app.append_items(&[&PredefinedMenuItem::hide(None), &PredefinedMenuItem::separator(), &PredefinedMenuItem::quit(None)])
        .ok()?;
    edit.append_items(&[
        &PredefinedMenuItem::undo(None),
        &PredefinedMenuItem::redo(None),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::cut(None),
        &PredefinedMenuItem::copy(None),
        &PredefinedMenuItem::paste(None),
        &PredefinedMenuItem::select_all(None),
    ])
    .ok()?;
    menu.append_items(&[&app, &edit]).ok()?;
    menu.init_for_nsapp();
    Some(menu)
}

/// Apre la finestra in un processo a parte (questo stesso programma).
pub fn spawn(url: &str, title: &str) -> std::io::Result<std::process::Child> {
    let exe = std::env::current_exe()?;
    std::process::Command::new(exe).args(["finestra", "--titolo", title, url]).spawn()
}

#[cfg(test)]
mod tests {
    #[test]
    fn origine() {
        assert_eq!(super::origin_of("http://localhost:8080/appunti?f=a.md"), "http://localhost:8080/");
        assert_eq!(super::origin_of("https://x.it"), "https://x.it/");
    }
}
