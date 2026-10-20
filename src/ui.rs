//! Interfaccia egui, due schede:
//! - **Lavagna**: foglio A4 a tutto schermo (da mirrorare sulla TV), colori,
//!   annulla/ripeti, gesto zig-zag, export, OCR, diretta web;
//! - **Archivio**: le pagine scaricate dal pad, con anteprime (come Inkspace).
//!
//! Architettura:
//!   UI (thread principale) ──UiCmd──▶ actor Bluetooth (tokio)
//!   UI ◀──UiEvent── actor / OCR / web
//!   decodificatore live ──punti──▶ Hub (lavagna condivisa) ◀── UI, server web

mod backstage;
mod chrome;
mod doc_view;
mod embed;
mod library_view;
mod options;
mod paper;
mod ribbon;
mod theme;
mod widgets;
mod workspace;
mod zoom;

use zoom::{ZoomOp, ZoomView};

use crate::actor::{ble_actor, Link, OcrDone, PadDrawing, UiCmd, UiEvent};
use crate::device::DeviceInfo;
use crate::export;
use crate::library::{self, Library, Meta};
use crate::scene::{Hub, Page, PageGeom, Scene, View};
use crate::settings::{run_hook, Settings};
use crate::stroke::{Drawing, Rgb};
use crate::{ai, ocr, web};
use anyhow::Result;
use eframe::egui;
use egui_phosphor::regular as ic;
use std::sync::Arc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

/// Raggio della gomma usata col mouse (pixel dello schermo).
const ERASER_PX: f32 = 10.0;

/// Tavolozza: (nome, colore). Tasti 1…6.
const PALETTE: [(&str, Rgb); 6] = [
    ("Nero", [20, 20, 20]),
    ("Blu", [25, 90, 200]),
    ("Rosso", [210, 40, 40]),
    ("Verde", [30, 150, 60]),
    ("Arancione", [240, 140, 0]),
    ("Viola", [140, 60, 190]),
];

/// Avvia UI + actor. Ritorna quando la finestra viene chiusa.
pub fn run_app(
    name: Option<String>,
    scan_secs: u64,
    web_now: bool,
    demo: bool,
    open_doc: Option<std::path::PathBuf>,
) -> Result<()> {
    // Tutte le risposte del pad finiscono in captures/gui.log (diagnosi).
    if std::env::var("WACOM_CAPTURE").is_err() {
        let _ = std::fs::create_dir_all("captures");
        std::env::set_var("WACOM_CAPTURE", "captures/gui.log");
    }
    let settings = Settings::load();
    let thumb_size = settings.thumb_size;
    let hub = Hub::new(Scene::new(&settings));

    // Archivio: al primo avvio importa le pagine già scaricate (copie grezze).
    let first_time = !std::path::Path::new(library::LIB_DIR).exists();
    let mut lib = Library::open();
    let mut status = "Pronto. Premi «Collega».".to_string();
    if first_time {
        let dir = std::path::Path::new(export::EXPORT_DIR).join("pad-raw");
        let n = lib.import_raw_dir(&dir, 0.0, 0.0);
        if n > 0 {
            status = format!("Archivio creato: importate {n} pagine già scaricate dal pad.");
        }
    }

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    // server solo per questo Mac: chat, appunti e impostazioni dentro la finestra
    let local_port = match rt.block_on(web::start_local(hub.clone())) {
        Ok(p) => {
            tracing::info!("server interno su http://127.0.0.1:{p}/");
            Some(p)
        }
        Err(e) => {
            tracing::warn!("server interno: {e}");
            None
        }
    };
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel::<UiCmd>();
    let (evt_tx, evt_rx) = tokio::sync::mpsc::unbounded_channel::<UiEvent>();
    rt.spawn(ble_actor(cmd_rx, evt_tx.clone(), hub.clone(), name, scan_secs));
    if demo {
        rt.spawn(demo_strokes(hub.clone()));
    }

    let mut app = App {
        rt: rt.handle().clone(),
        hub,
        settings,
        cmd_tx,
        evt_rx,
        evt_tx,
        info: None,
        status,
        live_on: false,
        fullscreen: false,
        fs_requested: None,
        bar_peek: false,
        bar_height: 90.0,
        web_url: None,
        web_starting: false,
        last_ocr_md: None,
        ocr_busy: false,
        page_pending: None,
        mouse_erase_last: None,
        mouse_hl_on: false,
        tab: chrome::Tab::Home,
        backstage: None,
        palette: None,
        palette_anchor: egui::Rect::NOTHING,
        applied_theme: String::new(),
        link: Link::Off,
        viewer: None,
        prefs: None,
        help: None,
        show_ribbon: true,
        mode: Mode::Board,
        local_port,
        embeds: embed::Embeds::new(),
        center: workspace::Center::Board,
        note: None,
        note_info: workspace::NoteInfo { zoom: 1.0, ..Default::default() },
        note_nav: false,
        note_cmd_pending: None,
        chat_cmd_pending: None,
        chat_open: false,
        chat_title: String::new(),
        chat_rect: None,
        center_rect: None,
        banner_hidden: false,
        chat_drop_hint: false,
        drag_last_pos: None,
        drag_what: None,
        settings_open: false,
        #[cfg(target_os = "macos")]
        mac_menu: None,
        lib,
        lui: library_view::LibUi::new(thumb_size),
        board_zoom: ZoomView::default(),
        docui: doc_view::DocUi::new(),
    };
    if web_now || app.settings.web_auto {
        app.start_web();
    }
    if let Some(p) = open_doc {
        app.start_import(p);
    }

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 880.0])
        .with_min_inner_size([760.0, 520.0])
        .with_title(crate::platform::APP_NAME);
    // icona della finestra (Linux, Windows; su macOS la dà il bundle .app)
    match eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon/openslate-256.png")) {
        Ok(icon) => viewport = viewport.with_icon(std::sync::Arc::new(icon)),
        Err(e) => tracing::warn!("icona della finestra: {e}"),
    }
    // macOS: barra del titolo dell'app accanto ai semafori, come in Word
    if cfg!(target_os = "macos") {
        viewport = viewport.with_fullsize_content_view(true).with_titlebar_shown(false).with_title_shown(false);
    }
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    let res = eframe::run_native(
        "openslate",
        options,
        Box::new(move |cc| {
            // Inter + icone Phosphor; colori e misure in stile Word (WordCraft).
            cc.egui_ctx.set_fonts(theme::fonts());
            // Cmd +/- servono allo zoom della pagina, non dell'interfaccia.
            cc.egui_ctx.options_mut(|o| o.zoom_with_keyboard = false);
            #[allow(unused_mut)]
            let mut app = app;
            // macOS: menu dell'app con «Modifica» (Copia/Incolla nelle pagine incorporate)
            #[cfg(target_os = "macos")]
            {
                app.mac_menu = mac_menu();
            }
            Ok(Box::new(app))
        }),
    )
        .map_err(|e| anyhow::anyhow!("eframe: {e}"));
    rt.shutdown_background();
    res
}

/// `--demo`: disegna da solo qualche tratto (prova senza pad, anche della
/// diretta web e del gesto zig-zag).
async fn demo_strokes(hub: Arc<Hub>) {
    use crate::stroke::{PenEvent, PenPhase};
    use std::time::Duration;
    let ev = |x: f32, y: f32, phase| PenEvent { x, y, pressure: 0.5, phase };
    let draw = |hub: &Hub, pts: Vec<(f32, f32)>, color: Rgb| {
        hub.lock().pen.color = color;
        for (i, &(x, y)) in pts.iter().enumerate() {
            hub.pen_event(ev(x, y, if i == 0 { PenPhase::Down } else { PenPhase::Move }));
        }
        let (x, y) = *pts.last().unwrap();
        hub.pen_event(ev(x, y, PenPhase::Up));
    };
    // WACOM_DEMO_FIRMA=1 (prova degli editor Word/PDF): dopo qualche secondo
    // una «firma» blu nella parte bassa della pagina, poi basta
    if std::env::var_os("WACOM_DEMO_FIRMA").is_some() {
        tokio::time::sleep(Duration::from_secs(12)).await;
        let g = hub.lock().geom();
        for k in 0..3 {
            let pts = (0..60)
                .map(|i| {
                    let t = i as f32 / 59.0;
                    g.pad(0.45 + k as f32 * 0.12 + t * 0.1, 0.80 + 0.025 * (t * 14.0 + k as f32).sin())
                })
                .collect();
            draw(&hub, pts, [20, 40, 140]);
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        return;
    }
    hub.edit(|s| {
        s.drawing.width = 29_700.0;
        s.drawing.height = 21_000.0;
    });
    let colors = PALETTE.map(|(_, c)| c);
    for (k, c) in colors.iter().enumerate() {
        let y0 = 3000.0 + k as f32 * 2600.0;
        let pts = (0..50).map(|i| (y0 + 900.0 * (i as f32 / 5.0).sin(), 2500.0 + i as f32 * 300.0)).collect();
        draw(&hub, pts, *c);
        tokio::time::sleep(Duration::from_millis(400)).await;
    }
    // evidenziatore sopra l'onda verde e quella arancione
    hub.lock().toggle_tool();
    for x0 in [10_800.0_f32, 13_400.0] {
        let pts = (0..30).map(|i| (x0, 2500.0 + i as f32 * 500.0)).collect();
        draw(&hub, pts, colors[0]);
    }
    hub.lock().toggle_tool();
    tokio::time::sleep(Duration::from_secs(3)).await;
    // zig-zag sopra le prime due onde: devono sparire
    let zz = (0..=8).map(|i| (2000.0 + (i % 2) as f32 * 5000.0, 6000.0 + i as f32 * 400.0)).collect();
    draw(&hub, zz, colors[0]);
    hub.lock().pen.color = colors[0];
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Board,
    Library,
}

struct App {
    rt: tokio::runtime::Handle,
    hub: Arc<Hub>,
    settings: Settings,
    cmd_tx: UnboundedSender<UiCmd>,
    evt_rx: UnboundedReceiver<UiEvent>,
    /// Per i risultati di OCR e server web.
    evt_tx: UnboundedSender<UiEvent>,
    info: Option<DeviceInfo>,
    status: String,
    live_on: bool,
    fullscreen: bool,
    /// Quando abbiamo chiesto il cambio di schermo intero (il sistema
    /// conferma con un po' di ritardo).
    fs_requested: Option<std::time::Instant>,
    /// A schermo intero: barra mostrata perché il mouse è in alto.
    bar_peek: bool,
    bar_height: f32,
    web_url: Option<String>,
    web_starting: bool,
    /// Ultima trascrizione in Markdown (va alla chat insieme al foglio).
    last_ocr_md: Option<String>,
    ocr_busy: bool,
    /// Pagina da aprire appena parte il server web (es. "chat", "appunti?f=…").
    page_pending: Option<String>,
    /// Gomma col mouse: ultima posizione durante la passata.
    mouse_erase_last: Option<egui::Pos2>,
    /// Evidenziatore col mouse in corso.
    mouse_hl_on: bool,
    /// Scheda della barra multifunzione.
    tab: chrome::Tab,
    /// Pagina «File» aperta (a tutta finestra).
    backstage: Option<backstage::BsPage>,
    /// Ricerca dei comandi aperta.
    palette: Option<chrome::Palette>,
    /// Dove sta la casella di ricerca (la finestrella si apre lì sotto).
    palette_anchor: egui::Rect,
    /// Tema messo in egui (si rimette se cambia nelle opzioni).
    applied_theme: String,
    /// Collegamento col pad (spia verde / arancio lampeggiante / rossa).
    link: Link,
    /// Finestra Chat/Appunti già aperta (si riusa invece di aprirne un'altra).
    viewer: Option<std::process::Child>,
    /// Opzioni in modifica (pagina File › Opzioni).
    prefs: Option<options::Prefs>,
    /// Finestra di aiuto aperta.
    help: Option<options::HelpPage>,
    /// Barra multifunzione visibile (doppio clic su una scheda la nasconde).
    show_ribbon: bool,
    mode: Mode,
    /// Porta del server interno (127.0.0.1) per le pagine incorporate.
    local_port: Option<u16>,
    /// Pagine web dentro la finestra (appunto, chat, impostazioni AI).
    embeds: embed::Embeds,
    /// Cosa c'è al centro: foglio, appunto o impostazioni AI.
    center: workspace::Center,
    /// Appunto aperto (nome in exports/).
    note: Option<String>,
    note_info: workspace::NoteInfo,
    /// Elenco degli appunti aperto.
    note_nav: bool,
    /// Comando per l'appunto da eseguire appena la pagina è pronta.
    note_cmd_pending: Option<String>,
    /// Comando per la chat da eseguire appena la pagina è pronta.
    chat_cmd_pending: Option<String>,
    /// Chat AI aperta a destra.
    chat_open: bool,
    chat_title: String,
    chat_rect: Option<egui::Rect>,
    center_rect: Option<(embed::Slot, egui::Rect)>,
    /// Barra «Slate non collegato» chiusa dall'utente.
    banner_hidden: bool,
    /// La chat mostra «lascia qui» (una scheda trascinata ci passa sopra).
    chat_drop_hint: bool,
    /// Ultimo punto del puntatore mentre si trascina una scheda.
    drag_last_pos: Option<egui::Pos2>,
    /// Cosa si sta trascinando (se il rilascio lo vede solo la pagina della chat).
    drag_what: Option<doc_view::ChatDrag>,
    /// Scheda «Impostazioni AI» aperta.
    settings_open: bool,
    #[cfg(target_os = "macos")]
    mac_menu: Option<muda::Menu>,
    lib: Library,
    lui: library_view::LibUi,
    /// Zoom della lavagna.
    board_zoom: ZoomView,
    /// Documenti importati e firma.
    docui: doc_view::DocUi,
}

#[derive(Clone, Copy)]
enum Fmt {
    Svg,
    Pdf,
    Png,
}

/// Azioni raccolte durante il disegno dell'interfaccia, eseguite dopo.
enum Action {
    SetMode(Mode),
    Connect,
    ToggleLive,
    Sync,
    Undo,
    Redo,
    Clear,
    Color(Rgb),
    Width(f32),
    ToggleZigzag,
    /// Gomma accesa/spenta.
    ToggleEraser,
    /// Penna ↔ evidenziatore.
    ToggleTool,
    /// Sceglie lo strumento (spegne la gomma).
    SetTool(crate::scene::Tool),
    /// Cambia scheda della barra.
    Tab(chrome::Tab),
    /// Apre (Some) o chiude (None) la pagina File.
    Backstage(Option<backstage::BsPage>),
    /// Ricerca dei comandi.
    Palette,
    ToggleRulers,
    ToggleSyncDelete,
    Theme(theme::Appearance),
    /// File › Opzioni.
    OpenPrefs,
    Quit,
    /// Scheda trascinata sulla chat AI: il foglio o il documento nel contesto.
    DropOnChat(doc_view::ChatDrag),
    ToggleRibbon,
    ToggleLongpress,
    ShowHelp(options::HelpPage),
    OpenWebBrowser,
    OpenDataFolder,
    OpenReadme,
    /// Toglie tutte le evidenziature della lavagna.
    ClearHighlights,
    /// Toglie un tratto (Alt+clic su un'evidenziatura).
    RemoveStroke(usize),
    Rotate,
    /// Gira di n quarti (gesto a due dita sul trackpad).
    RotateBy(i32),
    FlipX,
    FlipY,
    Fullscreen,
    Export(Fmt),
    Ocr,
    /// Mostra/nasconde la chat AI a destra.
    Chat,
    /// Allega il foglio alla chat e la apre («Discuti il foglio»).
    ChatBoard,
    /// Comando per l'appunto aperto (grassetto, formula, …).
    NoteCmd(String),
    NotePrint,
    NoteTypora,
    OpenNote(Option<String>),
    CloseNote,
    CloseSettings,
    /// Torna al foglio (dalla scheda di un appunto).
    ShowBoard,
    /// «Controlla il foglio»: foglio allegato e richiesta di controllo.
    ChatCheck,
    /// Rigatura del foglio.
    Paper(paper::Paper),
    ToggleMargin,
    /// Apre la finestra degli appunti.
    Appunti,
    /// Apre le impostazioni degli agenti AI.
    AiSettings,
    Web,
    CopyWebUrl,
    OpenExports,
    CopyImage,
    SaveBoard,
    /// Zoom della pagina visibile (lavagna o visualizzatore).
    Zoom(ZoomOp),
    Lib(library_view::LibAction),
    Doc(doc_view::DocAction),
}

/// Disegna una pagina il più grande possibile nel riquadro (stesse
/// proporzioni e posizioni della carta). Ritorna il rettangolo della pagina.
/// Immagine sotto i tratti (pagina di un documento).
pub(super) struct BgImage {
    pub tex: egui::TextureId,
    /// Dove sta sulla pagina (0..1).
    pub rect: [f32; 4],
    /// Le evidenziature sono già fuse nell'immagine (in "moltiplica"), tranne
    /// quella col indice `keep` (in corso): non vanno ridisegnate.
    pub baked: bool,
    pub keep: Option<usize>,
    /// La parte visibile disegnata nitida (immagine, zona della pagina del
    /// documento in frazioni): va sopra l'immagine intera.
    pub detail: Option<(egui::TextureId, [f32; 4])>,
}

fn paint_page(
    painter: &egui::Painter,
    rect: egui::Rect,
    d: &Drawing,
    view: View,
    bg: Option<BgImage>,
    zv: &mut ZoomView,
    paper: Option<(paper::Paper, bool)>,
) -> egui::Rect {
    // tavolo grigio e foglio bianco con l'ombra, come in Word
    let t = theme::Tokens::get(painter.ctx());
    painter.rect_filled(rect, 0.0, t.canvas);
    let (pw, ph) = crate::scene::page_dims(d);
    let g = PageGeom::new(view, pw, ph);
    let margin = (rect.width().min(rect.height()) * 0.025).max(12.0);
    let avail = rect.shrink(margin);
    let scale = (avail.width() / g.w).min(avail.height() / g.h);
    let size = egui::vec2(g.w * scale, g.h * scale);
    let page = zv.place(avail, size);
    for (grow, alpha) in [(6.0, 0.25), (3.0, 0.5), (1.0, 1.0)] {
        painter.rect_filled(page.translate(egui::vec2(0.0, 2.0)).expand(grow), 2.0 + grow, t.page_shadow.gamma_multiply(alpha));
    }
    painter.rect_filled(page, 0.0, egui::Color32::WHITE);
    // righe o quadretti (unità del pad: 1000 = 1 cm), solo senza documento sotto
    if let Some((p, margin)) = paper.filter(|_| bg.is_none()) {
        paper::paint(painter, page, (g.w / 1000.0, g.h / 1000.0), p, margin);
    }

    let map = |x: f32, y: f32| {
        let (u, v) = g.unit(x, y);
        egui::pos2(page.left() + u * page.width(), page.top() + v * page.height())
    };
    let clip = painter.with_clip_rect(page);
    // Pagina del documento (se c'è) sotto tutto.
    let (baked, keep) = bg.as_ref().map(|b| (b.baked, b.keep)).unwrap_or((false, None));
    if let Some(BgImage { tex, rect: [rx, ry, rw, rh], detail, .. }) = bg {
        let r = egui::Rect::from_min_size(
            page.min + egui::vec2(rx * page.width(), ry * page.height()),
            egui::vec2(rw * page.width(), rh * page.height()),
        );
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        clip.image(tex, r, uv, egui::Color32::WHITE);
        if let Some((dt, [dx, dy, dw, dh])) = detail {
            let dr = egui::Rect::from_min_size(r.min + egui::vec2(dx * r.width(), dy * r.height()), egui::vec2(dw * r.width(), dh * r.height()));
            clip.image(dt, dr, uv, egui::Color32::WHITE);
        }
    }
    // Prima gli evidenziatori (sotto), poi l'inchiostro.
    let ordered = d
        .strokes
        .iter()
        .enumerate()
        .filter(|(_, s)| s.highlight)
        .chain(d.strokes.iter().enumerate().filter(|(_, s)| !s.highlight));
    for (i, s) in ordered {
        if s.highlight && baked && keep != Some(i) {
            continue; // già nell'immagine di sfondo
        }
        let col = if s.highlight {
            highlight_screen_color(s.color)
        } else {
            egui::Color32::from_rgb(s.color[0], s.color[1], s.color[2])
        };
        let width = (s.width * page.height() / 800.0).clamp(0.8, 160.0);
        if s.points.len() < 2 {
            if let Some(p) = s.points.first() {
                clip.circle_filled(map(p.x, p.y), width * 0.6, col);
            }
            continue;
        }
        let pts: Vec<egui::Pos2> = s.points.iter().map(|p| map(p.x, p.y)).collect();
        clip.add(egui::Shape::line(pts, egui::Stroke::new(width, col)));
    }
    page
}

/// Colore trasparente per l'evidenziatore sullo schermo: sul bianco dà il
/// colore scelto, sul testo lo lascia leggibile (come un evidenziatore vero).
fn highlight_screen_color(c: Rgb) -> egui::Color32 {
    const A: f32 = 0.55;
    let ch = |v: u8| (255.0 - (255.0 - v as f32) / A).clamp(0.0, 255.0) as u8;
    egui::Color32::from_rgba_unmultiplied(ch(c[0]), ch(c[1]), ch(c[2]), (A * 255.0) as u8)
}

/// Punti di un tratto sullo schermo, dato il rettangolo della pagina.
fn stroke_screen_pts(d: &Drawing, view: View, page: egui::Rect, idx: usize) -> Vec<egui::Pos2> {
    let (pw, ph) = crate::scene::page_dims(d);
    let g = PageGeom::new(view, pw, ph);
    d.strokes[idx]
        .points
        .iter()
        .map(|p| {
            let (u, v) = g.unit(p.x, p.y);
            egui::pos2(page.left() + u * page.width(), page.top() + v * page.height())
        })
        .collect()
}

/// L'evidenziatura sotto il puntatore (la più in alto), se c'è.
fn hit_highlight(d: &Drawing, view: View, page: egui::Rect, pos: egui::Pos2) -> Option<usize> {
    let dist_seg = |p: egui::Pos2, a: egui::Pos2, b: egui::Pos2| {
        let ab = b - a;
        let t = if ab.length_sq() > 0.0 { ((p - a).dot(ab) / ab.length_sq()).clamp(0.0, 1.0) } else { 0.0 };
        (a + ab * t - p).length()
    };
    (0..d.strokes.len()).rev().find(|&i| {
        let s = &d.strokes[i];
        if !s.highlight || s.points.is_empty() {
            return false;
        }
        let tol = s.width * page.height() / 800.0 / 2.0 + 3.0;
        let pts = stroke_screen_pts(d, view, page, i);
        if pts.len() == 1 {
            return (pts[0] - pos).length() <= tol;
        }
        pts.windows(2).any(|w| dist_seg(pos, w[0], w[1]) <= tol)
    })
}

/// Contorno rosso tratteggiato attorno a un tratto (anteprima di cosa si toglie).
fn outline_stroke(painter: &egui::Painter, d: &Drawing, view: View, page: egui::Rect, idx: usize) {
    let pts = stroke_screen_pts(d, view, page, idx);
    let w = d.strokes[idx].width * page.height() / 800.0;
    let col = egui::Color32::from_rgba_unmultiplied(220, 40, 40, 110);
    painter.add(egui::Shape::line(pts, egui::Stroke::new(w + 4.0, col)));
}

/// Cmd + / Cmd = / Cmd − / Cmd 0 (letti dagli eventi).
fn zoom_keys(ctx: &egui::Context) -> Option<ZoomOp> {
    ctx.input_mut(|i| {
        let cmd = egui::Modifiers::COMMAND;
        let shift_cmd = egui::Modifiers::COMMAND | egui::Modifiers::SHIFT;
        if i.consume_key(cmd, egui::Key::Plus)
            || i.consume_key(cmd, egui::Key::Equals)
            || i.consume_key(shift_cmd, egui::Key::Equals)
        {
            Some(ZoomOp::In)
        } else if i.consume_key(cmd, egui::Key::Minus) {
            Some(ZoomOp::Out)
        } else if i.consume_key(cmd, egui::Key::Num0) {
            Some(ZoomOp::Fit)
        } else {
            None
        }
    })
}

/// Pulsanti dello zoom: − 100% + Adatta.
fn zoom_controls(ui: &mut egui::Ui, zv: &ZoomView, actions: &mut Vec<Action>) {
    if ui.button(ic::MAGNIFYING_GLASS_MINUS).on_hover_text("Riduci (Cmd −, rotella)").clicked() {
        actions.push(Action::Zoom(ZoomOp::Out));
    }
    if ui
        .add(egui::Button::new(zv.label()).frame(false))
        .on_hover_text("Clic: pagina intera (Cmd 0)")
        .clicked()
    {
        actions.push(Action::Zoom(ZoomOp::Fit));
    }
    if ui.button(ic::MAGNIFYING_GLASS_PLUS).on_hover_text("Ingrandisci (Cmd +, rotella)").clicked() {
        actions.push(Action::Zoom(ZoomOp::In));
    }
    if ui
        .add_enabled(!zv.is_fit(), egui::Button::new(ic::ARROWS_IN))
        .on_hover_text("Pagina intera (Cmd 0, doppio clic)")
        .clicked()
    {
        actions.push(Action::Zoom(ZoomOp::Fit));
    }
}

/// Nome file sicuro da un titolo.
fn file_stem(title: &str) -> String {
    let s: String = title
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() { "pagina".into() } else { s.chars().take(40).collect() }
}

impl App {
    fn view(&self) -> View {
        self.hub.lock().view
    }

    fn drain_events(&mut self) {
        while let Ok(ev) = self.evt_rx.try_recv() {
            match ev {
                UiEvent::Status(s) => self.status = s,
                UiEvent::Error(e) => self.status = format!("⚠ {e}"),
                UiEvent::Info(i) => {
                    // Teniamo nome e firmware già noti se la nuova lettura non li ha.
                    let mut i = i;
                    if let Some(old) = &self.info {
                        if i.name.is_empty() {
                            i.name = old.name.clone();
                        }
                        if i.firmware.is_empty() {
                            i.firmware = old.firmware.clone();
                        }
                    }
                    self.info = Some(i);
                }
                UiEvent::Live(on) => self.live_on = on,
                UiEvent::Link(l) => {
                    if l == Link::Off && self.link == Link::On {
                        self.status = "Slate scollegato (spento o fuori portata). Premi il pulsante del pad e «Collega».".into();
                    }
                    self.link = l;
                }
                UiEvent::Synced { files, deleted, drawings } => self.on_synced(files, deleted, drawings),
                UiEvent::PadFiles(n) => self.lui.pad_files = Some(n),
                UiEvent::Ocr(r) => {
                    self.ocr_busy = false;
                    match r {
                        Ok(d) => self.on_ocr(d),
                        Err(e) => self.status = format!("⚠ Trascrizione: {e}"),
                    }
                }
                UiEvent::Web(r) => {
                    self.web_starting = false;
                    match r {
                        Ok(url) => {
                            self.status = format!("Diretta web attiva: apri {url} sulla TV o sul telefono");
                            self.web_url = Some(url);
                            if let Some(page) = self.page_pending.take() {
                                self.open_page_now(&page);
                            }
                        }
                        Err(e) => {
                            self.page_pending = None;
                            self.status = format!("⚠ Diretta web: {e}");
                        }
                    }
                }
            }
        }
    }

    /// Pagine scaricate dal pad → archivio.
    fn on_synced(&mut self, files: usize, deleted: usize, drawings: Vec<PadDrawing>) {
        self.lui.pad_files = Some(files.saturating_sub(deleted));
        if files == 0 {
            self.status = "Nessuna pagina nuova nel pad.".into();
            return;
        }
        let mut new_ids = Vec::new();
        let mut dup = 0;
        for d in drawings {
            if self.lib.has_pad_page(d.created, d.drawing.strokes.len()) {
                dup += 1;
                continue;
            }
            let mut meta = Meta::new(d.created, library::SOURCE_PAD);
            meta.raw = d.raw;
            match self.lib.add(&d.drawing, meta) {
                Ok(id) => new_ids.push(id),
                Err(e) => self.status = format!("⚠ archivio: {e}"),
            }
        }
        let left = files.saturating_sub(deleted);
        self.status = format!(
            "Dal pad: {} nell'archivio{}{}",
            if new_ids.len() == 1 { "1 pagina nuova".to_string() } else { format!("{} pagine nuove", new_ids.len()) },
            if dup > 0 { format!(" ({dup} già presenti)") } else { String::new() },
            if left > 0 { format!(" — ne restano {left} sul pad") } else { String::new() },
        );
        self.mode = Mode::Library;
        self.lui.viewer = None;
        self.lui.selected = new_ids.into_iter().collect();
    }

    fn start_web(&mut self) {
        if self.web_url.is_some() || self.web_starting {
            return;
        }
        self.web_starting = true;
        let hub = self.hub.clone();
        let port = self.settings.web_port;
        let tx = self.evt_tx.clone();
        self.rt.spawn(async move {
            let r = web::start(hub, port).await.map_err(|e| e.to_string());
            let _ = tx.send(UiEvent::Web(r));
        });
    }

    fn board_page(&self) -> (String, Page) {
        ("lavagna".into(), self.hub.lock().page())
    }

    /// Riconosce la scrittura di una o più pagine (in sequenza).
    fn ocr_pages(&mut self, pages: Vec<(String, Page)>) {
        if self.ocr_busy {
            return;
        }
        let pages: Vec<(String, Page)> = pages.into_iter().filter(|(_, p)| !p.is_empty()).collect();
        if pages.is_empty() {
            self.status = "Pagina vuota: niente da riconoscere.".into();
            return;
        }
        self.ocr_busy = true;
        self.status = if self.settings.ocr_engine == "vision" {
            "Riconosco la scrittura (Apple Vision)…".into()
        } else {
            "Trascrivo con l'agente AI… può volerci un minuto.".into()
        };
        let cfg = self.settings.clone();
        let tx = self.evt_tx.clone();
        std::thread::spawn(move || {
            let many = pages.len() > 1;
            let title = if many { format!("{} pagine", pages.len()) } else { pages[0].0.clone() };
            let mut out = String::new();
            let mut first_png = None;
            let mut by = String::new();
            let mut markdown = false;
            for (i, (title, page)) in pages.into_iter().enumerate() {
                let r = page_png_for_ai(page).and_then(|png| {
                    let png = Arc::new(png);
                    if first_png.is_none() {
                        first_png = Some(png.clone());
                    }
                    ocr::recognize(&png, &cfg)
                });
                match r {
                    Ok(r) => {
                        markdown = r.markdown;
                        by = r.by;
                        if many {
                            if i > 0 {
                                out.push_str(if markdown { "\n---\n\n" } else { "\n" });
                            }
                            out.push_str(&if markdown { format!("<!-- {title} -->\n") } else { format!("── {title} ──\n") });
                        }
                        out.push_str(r.text.trim());
                        out.push_str("\n");
                    }
                    Err(e) => {
                        let _ = tx.send(UiEvent::Ocr(Err(format!("{e:#}"))));
                        return;
                    }
                }
            }
            let _ = tx.send(UiEvent::Ocr(Ok(OcrDone {
                text: out.trim().to_string(),
                markdown,
                by,
                title,
                png: first_png,
            })));
        });
    }

    /// Trascrizione arrivata: si salva come appunto (.md) e si apre al centro,
    /// come un documento, con le formule impaginate.
    fn on_ocr(&mut self, d: OcrDone) {
        let path = export::new_path("appunti", "md");
        let text = if d.text.is_empty() { "(nessun testo riconosciuto)".to_string() } else { d.text };
        match std::fs::write(&path, format!("{}\n", text.trim_end())) {
            Ok(()) => {
                run_hook(&self.settings.hook, &path.to_string_lossy());
                self.status = format!("Trascritto da {} — appunto {}", d.by, path.display());
            }
            Err(e) => {
                self.status = format!("⚠ salvataggio {}: {e}", path.display());
                return;
            }
        }
        // la chat discute la pagina appena trascritta
        ai::set_context(&d.title, d.png.clone(), &text);
        self.last_ocr_md = Some(text);
        let name = path.file_name().map(|n| n.to_string_lossy().to_string());
        self.open_note(name);
    }

    /// Apre la chat AI (avvia il server web se serve).
    /// La pagina mandata con `ai::set_context` va in una conversazione nuova
    /// (o in quella aperta, se è ancora vuota).
    fn open_chat(&mut self) {
        self.open_page("chat?nuova=1");
    }

    /// Apre gli appunti (.md con le formule), eventualmente su un file.
    fn open_appunti_window(&mut self, file: Option<&str>) {
        let page = match file {
            Some(name) => format!("appunti?f={}", url_escape(name)),
            None => "appunti".to_string(),
        };
        self.open_page(&page);
    }

    /// Apre una pagina del server web in una finestra dell'app.
    fn open_page(&mut self, page: &str) {
        if self.web_url.is_some() {
            self.open_page_now(page);
        } else {
            self.page_pending = Some(page.to_string());
            self.start_web();
            self.status = "Avvio…".into();
        }
    }

    fn open_page_now(&mut self, page: &str) {
        let Some(url) = &self.web_url else { return };
        let port = url.trim_end_matches('/').rsplit(':').next().unwrap_or("8080").to_string();
        let full = format!("http://localhost:{port}/{page}");
        let title = if page.starts_with("chat") {
            "Chat AI"
        } else if page.starts_with("impostazioni") {
            "Impostazioni AI"
        } else {
            "Appunti"
        };
        // finestra già aperta: ci va lei (e torna davanti)
        if let Some(ch) = &mut self.viewer {
            if matches!(ch.try_wait(), Ok(None)) {
                web::navigate(page);
                self.status = format!("{title} aperta.");
                return;
            }
        }
        // altrimenti una finestra nuova dell'app (WebView); se non parte, il browser
        match crate::viewer::spawn(&full, title) {
            Ok(child) => {
                self.viewer = Some(child);
                self.status = format!("{title} aperta.");
            }
            Err(e) => {
                tracing::warn!("finestra {title}: {e}");
                open_path(&full);
                self.status = format!("{title} aperta nel browser: {full}");
            }
        }
    }

    /// Evidenziatore col mouse: tasto sinistro premuto = si evidenzia; con
    /// Shift la linea è dritta (orizzontale o verticale).
    fn mouse_highlighter(&mut self, ui: &egui::Ui, resp: &egui::Response) {
        use crate::stroke::PenPhase;
        let page = self.docui.last_page;
        if page.width() < 1.0 || page.height() < 1.0 {
            return;
        }
        let (down, pos, origin, shift) =
            ui.input(|i| (i.pointer.primary_down(), i.pointer.interact_pos(), i.pointer.press_origin(), i.modifiers.shift));
        let uv = |q: egui::Pos2| ((q.x - page.left()) / page.width(), (q.y - page.top()) / page.height());
        match (down, pos) {
            (true, Some(p)) if self.mouse_hl_on || resp.is_pointer_button_down_on() => {
                if !self.mouse_hl_on {
                    let o = origin.unwrap_or(p);
                    if !page.contains(o) || ui.input(|i| i.modifiers.alt) {
                        return; // fuori dal foglio, o Alt+clic per togliere un'evidenziatura
                    }
                    let (u, v) = uv(o);
                    self.hub.mouse_highlight(PenPhase::Down, u, v, false);
                    self.mouse_hl_on = self.hub.lock().mouse_highlighting();
                }
                if self.mouse_hl_on {
                    let (u, v) = uv(p);
                    self.hub.mouse_highlight(PenPhase::Move, u, v, shift);
                }
            }
            _ => {
                if self.mouse_hl_on {
                    self.mouse_hl_on = false;
                    self.hub.mouse_highlight(PenPhase::Up, 0.0, 0.0, false);
                }
            }
        }
    }

    /// Gomma col mouse: tenendo premuto il tasto sinistro si cancella ciò che
    /// il cerchio tocca (una passata = un solo Cmd+Z).
    fn mouse_eraser(&mut self, ui: &egui::Ui, resp: &egui::Response) {
        let page = self.docui.last_page;
        let (down, pos, origin) = ui.input(|i| (i.pointer.primary_down(), i.pointer.interact_pos(), i.pointer.press_origin()));
        match (down, pos) {
            (true, Some(p)) if self.mouse_erase_last.is_some() || resp.is_pointer_button_down_on() => {
                if page.width() < 1.0 || page.height() < 1.0 {
                    return;
                }
                let uv = |q: egui::Pos2| ((q.x - page.left()) / page.width(), (q.y - page.top()) / page.height());
                // all'inizio si parte dal punto dove si è premuto
                let a = self.mouse_erase_last.or(origin).unwrap_or(p);
                let n = self.hub.lock().mouse_erase_segment(uv(a), uv(p), ERASER_PX / page.height());
                if n > 0 {
                    // ridisegno (anche della diretta web) e salvataggio
                    self.hub.edit(|_| ());
                }
                self.mouse_erase_last = Some(p);
            }
            _ => {
                if self.mouse_erase_last.take().is_some() {
                    self.hub.lock().mouse_erase_end();
                }
            }
        }
    }

    /// Nome della pagina visibile (per trascrizione e chat).
    fn board_title(&self) -> String {
        match &self.docui.doc {
            Some(d) => format!("{} — pagina {}", d.title, d.current + 1),
            None => format!("Lavagna {}", chrono::Local::now().format("%d/%m/%Y %H:%M")),
        }
    }

    /// Pagina attuale (lavagna o documento) come contesto della chat.
    fn set_chat_context_from_board(&mut self) {
        if self.docui.doc.is_some() {
            self.set_chat_context_from_doc();
            return;
        }
        let title = self.board_title();
        let (_, page) = self.board_page();
        // se la trascrizione di questa pagina è aperta, va anche quella
        let md = self.last_ocr_md.clone().unwrap_or_default();
        if page.is_empty() {
            ai::set_context("", None, "");
            return;
        }
        match page_png_for_ai(page) {
            Ok(png) => ai::set_context(&title, Some(Arc::new(png)), &md),
            Err(e) => self.status = format!("⚠ immagine della pagina: {e}"),
        }
    }

    /// Documento aperto come contesto della chat: l'immagine della pagina
    /// aperta (con quello che ci hai scritto sopra) e il testo di tutte le
    /// pagine. Il testo si estrae la prima volta in un thread (può volerci un
    /// po'); la chat se ne accorge da sola e lo allega.
    fn set_chat_context_from_doc(&mut self) {
        let Some(d) = &self.docui.doc else { return };
        let title = format!("{} — pagina {} di {}", d.title, d.current + 1, d.pages.len());
        let dir = d.dir.clone();
        let (_, page) = self.board_page();
        let png = match page_png_for_ai(page) {
            Ok(p) => Some(Arc::new(p)),
            Err(e) => {
                self.status = format!("⚠ immagine della pagina: {e}");
                None
            }
        };
        let done = |text: &str| -> String {
            if text.is_empty() {
                "Documento nella chat: è una scansione senza testo, va l'immagine della pagina aperta".into()
            } else {
                format!("Documento nella chat: testo di tutte le pagine ({} parole) e immagine della pagina aperta", text.split_whitespace().count())
            }
        };
        if dir.join("text.txt").exists() {
            let text = crate::document::text_of(&dir).unwrap_or_default();
            ai::set_context_doc(&title, png, &text);
            self.status = done(&text);
            return;
        }
        ai::set_context_doc(&title, png.clone(), "");
        self.status = "Leggo il testo del documento per la chat…".into();
        let tx = self.evt_tx.clone();
        std::thread::spawn(move || {
            let msg = match crate::document::text_of(&dir) {
                Ok(text) => {
                    // solo se nel frattempo non si è allegato altro
                    let same = ai::context().is_some_and(|c| c.doc && c.title == title);
                    if same && !text.is_empty() {
                        ai::set_context_doc(&title, png, &text);
                    }
                    done(&text)
                }
                Err(e) => format!("⚠ testo del documento: {e} (nella chat va solo l'immagine della pagina)"),
            };
            let _ = tx.send(UiEvent::Status(msg));
        });
    }

    /// Copia una pagina negli appunti come immagine (per una chat AI).
    fn copy_page(&mut self, page: Page) {
        if page.is_empty() {
            self.status = "La pagina è vuota.".into();
            return;
        }
        // Con un documento sotto si copia la pagina intera.
        let page = if self.settings.copy_crop && page.bg.is_none() {
            export::crop_to_content(&page, 0.03)
        } else {
            page
        };
        let res = export::page_to_rgba(&page, self.settings.copy_size).and_then(|(w, h, px)| {
            let mut cb = arboard::Clipboard::new()?;
            cb.set_image(arboard::ImageData { width: w, height: h, bytes: px.into() })?;
            Ok((w, h))
        });
        self.status = match res {
            Ok((w, h)) => format!("Immagine copiata ({w}×{h}): incollala nella chat con Cmd+V"),
            Err(e) => format!("⚠ copia negli appunti: {e}"),
        };
    }

    /// Esporta: PDF = un file con tutte le pagine; PNG/SVG = un file per pagina.
    fn export_pages(&mut self, pages: Vec<(String, Page)>, fmt: Fmt) {
        let pages: Vec<(String, Page)> = pages.into_iter().filter(|(_, p)| !p.is_empty()).collect();
        if pages.is_empty() {
            self.status = "Niente da esportare: pagina vuota.".into();
            return;
        }
        let mut written = Vec::new();
        let mut error = None;
        match fmt {
            Fmt::Pdf => {
                let stem = if pages.len() == 1 {
                    file_stem(&pages[0].0)
                } else {
                    format!("pagine-{}", pages.len())
                };
                let path = export::new_path(&stem, "pdf");
                let refs: Vec<&Page> = pages.iter().map(|(_, p)| p).collect();
                match export::save_pdf(&refs, &path) {
                    Ok(()) => written.push(path),
                    Err(e) => error = Some(e.to_string()),
                }
            }
            Fmt::Png | Fmt::Svg => {
                for (title, page) in &pages {
                    let ext = if matches!(fmt, Fmt::Png) { "png" } else { "svg" };
                    let path = export::new_path(&file_stem(title), ext);
                    let r = if matches!(fmt, Fmt::Png) {
                        export::save_png(page, &path).map_err(|e| e.to_string())
                    } else {
                        export::save_svg(page, &path).map_err(|e| e.to_string())
                    };
                    match r {
                        Ok(()) => written.push(path),
                        Err(e) => error = Some(e),
                    }
                }
            }
        }
        for p in &written {
            run_hook(&self.settings.hook, &p.to_string_lossy());
        }
        self.status = match (error, written.len()) {
            (Some(e), _) => format!("⚠ export: {e}"),
            (None, 1) => format!("Salvato {}", written[0].display()),
            (None, n) => format!("Salvati {n} file in {}/", export::EXPORT_DIR),
        };
    }

    fn apply(&mut self, a: Action, ctx: &egui::Context) {
        match a {
            Action::SetMode(m) => {
                if m == Mode::Library && self.mode != Mode::Library && self.live_on {
                    let _ = self.cmd_tx.send(UiCmd::CountFiles);
                }
                if m == Mode::Library && self.fullscreen {
                    self.fullscreen = false;
                    self.fs_requested = Some(std::time::Instant::now());
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                }
                self.mode = m;
            }
            Action::Connect => {
                let _ = self.cmd_tx.send(UiCmd::Connect);
            }
            Action::ToggleLive => {
                let _ = self.cmd_tx.send(if self.live_on { UiCmd::StopLive } else { UiCmd::StartLive });
            }
            Action::Sync => {
                let _ = self.cmd_tx.send(UiCmd::Sync {
                    delete: self.settings.sync_delete,
                });
            }
            Action::Undo => {
                if !self.hub.edit(|s| s.undo()) {
                    self.status = "Niente da annullare.".into();
                }
            }
            Action::Redo => {
                self.hub.edit(|s| s.redo());
            }
            Action::Clear => self.hub.edit(|s| s.clear()),
            Action::Color(c) => self.hub.lock().pen.color = c,
            Action::Width(d) => {
                let mut s = self.hub.lock();
                s.pen.width = (s.pen.width + d).clamp(0.6, 10.0);
            }
            Action::SetTool(t) => {
                self.hub.edit(|s| {
                    s.tool = t;
                    s.eraser = false;
                });
                self.status = match t {
                    crate::scene::Tool::Highlighter => {
                        "Evidenziatore: col pennino, oppure col mouse (Shift = linea dritta orizzontale o verticale).".into()
                    }
                    crate::scene::Tool::Pen => "Penna attiva.".into(),
                };
            }
            Action::Tab(t) => {
                if t == chrome::Tab::Archivio {
                    self.apply(Action::SetMode(Mode::Library), ctx);
                } else {
                    if self.mode != Mode::Board {
                        self.apply(Action::SetMode(Mode::Board), ctx);
                    }
                    if t == chrome::Tab::Appunto {
                        self.center = workspace::Center::Note;
                    } else if self.center != workspace::Center::Board {
                        self.center = workspace::Center::Board;
                    }
                    self.tab = t;
                    self.show_ribbon = true;
                }
            }
            Action::Backstage(p) => {
                if p != Some(backstage::BsPage::Opzioni) {
                    self.prefs = None;
                }
                if p.is_some() {
                    self.palette = None;
                }
                self.backstage = p;
            }
            Action::Palette => {
                if self.palette.is_none() {
                    self.palette = Some(chrome::Palette { query: String::new(), sel: 0, fresh: true });
                } else {
                    self.palette = None;
                }
            }
            Action::ToggleRulers => {
                self.settings.rulers = !self.settings.rulers;
                self.settings.save();
            }
            Action::ToggleSyncDelete => {
                self.settings.sync_delete = !self.settings.sync_delete;
                self.settings.save();
            }
            Action::Theme(a) => {
                self.settings.ui_theme = a.code().to_string();
                self.settings.save();
            }
            Action::OpenPrefs => {
                self.backstage = Some(backstage::BsPage::Opzioni);
                if self.prefs.is_none() {
                    let mut draft = self.settings.clone();
                    let s = self.hub.lock();
                    draft.rot = s.view.rot;
                    draft.flip_x = s.view.flip_x;
                    draft.flip_y = s.view.flip_y;
                    draft.pen_color = s.pen.color;
                    draft.pen_width = s.pen.width;
                    draft.highlight_color = s.highlighter.color;
                    draft.highlight_width = s.highlighter.width;
                    draft.zigzag = s.zigzag;
                    draft.longpress_toggle = s.longpress;
                    drop(s);
                    self.prefs = Some(options::Prefs { draft, tab: options::PrefTab::Aspetto });
                }
            }
            Action::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            Action::DropOnChat(what) => {
                egui::DragAndDrop::clear_payload(ctx);
                self.drag_what = None;
                // un documento non aperto si apre (la chat vede la pagina aperta e il testo)
                if let doc_view::ChatDrag::Doc(id, _) = &what {
                    if self.docui.doc.as_ref().is_none_or(|d| &d.id != id) {
                        self.apply_doc(doc_view::DocAction::OpenRecent(id.clone()));
                    }
                } else if self.docui.doc.is_some() {
                    self.apply_doc(doc_view::DocAction::TabBoard);
                }
                self.center = workspace::Center::Board;
                self.set_chat_context_from_board();
                if self.chat_open {
                    self.embeds.eval(embed::Slot::Chat, "window.wsChat && wsChat.attachNow()");
                }
                self.toggle_chat(true);
            }
            Action::ToggleRibbon => self.show_ribbon = !self.show_ribbon,
            Action::ToggleLongpress => {
                let on = {
                    let mut s = self.hub.lock();
                    s.longpress = !s.longpress;
                    s.longpress
                };
                self.settings.longpress_toggle = on;
                self.settings.save();
            }
            Action::ShowHelp(p) => self.help = Some(p),
            Action::OpenWebBrowser => {
                if let Some(u) = &self.web_url {
                    open_path(u);
                }
            }
            Action::OpenDataFolder => {
                if let Ok(d) = std::env::current_dir() {
                    open_path(&d.to_string_lossy());
                }
            }
            Action::OpenReadme => {
                if std::path::Path::new("README.md").exists() {
                    open_path("README.md");
                } else {
                    self.status = "README.md non trovato nella cartella dell'app.".into();
                }
            }
            Action::ToggleTool => {
                let mut s = self.hub.lock();
                s.toggle_tool();
                self.status = match s.tool {
                    crate::scene::Tool::Highlighter => "Evidenziatore attivo (H o tocco lungo per tornare alla penna).".into(),
                    crate::scene::Tool::Pen => "Penna attiva.".into(),
                };
            }
            Action::ClearHighlights => {
                let n = self.hub.edit(|s| s.remove_highlights());
                self.status = if n == 0 {
                    "Nessuna evidenziatura da togliere.".into()
                } else {
                    format!("Tolte {n} evidenziature (Cmd+Z per annullare).")
                };
            }
            Action::RemoveStroke(i) => {
                if self.hub.edit(|s| s.remove_stroke(i)) {
                    self.status = "Evidenziatura tolta (Cmd+Z per annullare).".into();
                }
            }
            Action::ToggleEraser => {
                let on = self.hub.edit(|s| {
                    s.eraser = !s.eraser;
                    s.eraser
                });
                self.status = if on {
                    "Gomma attiva: cancella col pennino o col mouse (tasto sinistro); E per tornare a scrivere.".into()
                } else {
                    "Gomma spenta: si scrive.".into()
                };
            }
            Action::ToggleZigzag => {
                let mut s = self.hub.lock();
                s.zigzag = !s.zigzag;
            }
            Action::Rotate => self.apply(Action::RotateBy(1), ctx),
            Action::RotateBy(n) => {
                self.hub.edit(|s| s.view.rot = (s.view.rot + n).rem_euclid(4));
                self.lui.thumbs.clear();
            }
            Action::FlipX => {
                self.hub.edit(|s| s.view.flip_x = !s.view.flip_x);
                self.lui.thumbs.clear();
            }
            Action::FlipY => {
                self.hub.edit(|s| s.view.flip_y = !s.view.flip_y);
                self.lui.thumbs.clear();
            }
            Action::Fullscreen => {
                self.fullscreen = !self.fullscreen;
                self.fs_requested = Some(std::time::Instant::now());
                self.bar_peek = false;
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.fullscreen));
            }
            Action::Export(Fmt::Pdf) if self.docui.doc.is_some() => {
                self.apply_doc(doc_view::DocAction::SaveSigned);
            }
            Action::Export(f) => {
                let p = self.board_page();
                self.export_pages(vec![p], f);
            }
            Action::Ocr => {
                let (_, p) = self.board_page();
                let title = self.board_title();
                self.ocr_pages(vec![(title, p)]);
            }
            Action::Appunti => self.open_note(None),
            Action::AiSettings => {
                if self.local_port.is_some() && self.embeds.failed.is_none() {
                    self.settings_open = true;
                    self.center = workspace::Center::Settings;
                    self.backstage = None;
                } else {
                    self.open_page("impostazioni");
                }
            }
            Action::CloseSettings => {
                self.settings_open = false;
                if self.center == workspace::Center::Settings {
                    self.center = if self.note.is_some() { workspace::Center::Note } else { workspace::Center::Board };
                }
            }
            Action::Chat => self.toggle_chat(!self.chat_open),
            Action::ChatBoard => {
                self.set_chat_context_from_board();
                if self.chat_open {
                    self.embeds.eval(embed::Slot::Chat, "window.wsChat && wsChat.attachNow()");
                }
                self.toggle_chat(true);
            }
            Action::NoteCmd(c) => {
                if c == "nav" {
                    self.note_nav = !self.note_nav;
                }
                if self.center != workspace::Center::Note {
                    if c == "new" {
                        self.show_note_center();
                    } else {
                        self.open_note(None);
                    }
                }
                if self.embeds.has(embed::Slot::Note) {
                    self.embeds.eval(embed::Slot::Note, &workspace::note_js(&c));
                } else {
                    // la pagina non c'è ancora: il comando parte appena caricata
                    self.note_cmd_pending = Some(c);
                }
                self.embeds.focus(embed::Slot::Note);
            }
            Action::NotePrint => self.embeds.print(embed::Slot::Note),
            Action::NoteTypora => {
                if let Some(n) = &self.note {
                    self.embeds.eval(embed::Slot::Note, "window.wsNote && wsNote.flush()");
                    open_with(&std::path::Path::new(export::EXPORT_DIR).join(n), &self.settings.md_app);
                }
            }
            Action::OpenNote(n) => self.open_note(n),
            Action::CloseNote => self.close_note(),
            Action::ChatCheck => {
                self.set_chat_context_from_board();
                if self.embeds.has(embed::Slot::Chat) {
                    self.embeds.eval(embed::Slot::Chat, "window.wsChat && wsChat.check()");
                } else {
                    self.chat_cmd_pending = Some("correggi".into());
                }
                self.toggle_chat(true);
            }
            Action::Paper(p) => {
                self.settings.paper = p.code().into();
                self.settings.save();
                self.status = format!("Foglio: {}.", p.label());
            }
            Action::ToggleMargin => {
                self.settings.paper_margin = !self.settings.paper_margin;
                self.settings.save();
            }
            Action::ShowBoard => {
                self.center = workspace::Center::Board;
                if self.tab == chrome::Tab::Appunto {
                    self.tab = chrome::Tab::Home;
                }
            }
            Action::Web => self.start_web(),
            Action::CopyWebUrl => {
                if let Some(u) = &self.web_url {
                    ctx.copy_text(u.clone());
                    self.status = "Link copiato.".into();
                }
            }
            Action::OpenExports => {
                let _ = std::fs::create_dir_all(export::EXPORT_DIR);
                crate::platform::open(export::EXPORT_DIR);
            }
            Action::CopyImage => {
                let (_, p) = self.board_page();
                self.copy_page(p);
            }
            Action::SaveBoard if self.docui.doc.is_some() => {
                self.status = "Con un documento aperto usa «Salva PDF firmato».".into();
            }
            Action::SaveBoard => {
                let d = self.hub.lock().drawing.clone();
                if d.is_empty() {
                    self.status = "La lavagna è vuota.".into();
                    return;
                }
                let meta = Meta::new(chrono::Local::now().naive_local(), library::SOURCE_BOARD);
                match self.lib.add(&d, meta) {
                    Ok(id) => {
                        self.status = "Lavagna salvata nell'archivio.".into();
                        self.lui.selected = [id].into_iter().collect();
                    }
                    Err(e) => self.status = format!("⚠ archivio: {e}"),
                }
            }
            Action::Zoom(op) if self.mode == Mode::Board && self.center == workspace::Center::Note => {
                let js = match op {
                    ZoomOp::In => workspace::note_js("zoomin"),
                    ZoomOp::Out => workspace::note_js("zoomout"),
                    ZoomOp::Fit => workspace::note_js("zoomfit"),
                    ZoomOp::To(z) => format!("window.wsNote && wsNote.zoom({z})"),
                };
                self.embeds.eval(embed::Slot::Note, &js);
            }
            Action::Zoom(op) => match (self.mode, self.lui.viewer.is_some()) {
                (Mode::Board, _) => self.board_zoom.apply(op),
                (Mode::Library, true) => self.lui.viewer_zoom.apply(op),
                // nella griglia: Cmd +/- cambiano la dimensione delle anteprime
                (Mode::Library, false) => {
                    let t = &mut self.lui.thumb_size;
                    *t = match op {
                        ZoomOp::In => *t * 1.15,
                        ZoomOp::Out => *t / 1.15,
                        ZoomOp::Fit => 330.0,
                        ZoomOp::To(z) => 330.0 * z,
                    }
                    .clamp(library_view::THUMB_MIN, library_view::THUMB_MAX);
                }
            },
            Action::Lib(la) => self.apply_lib(la, ctx),
            Action::Doc(da) => self.apply_doc(da),
        }
    }

    /// Salva le impostazioni se vista/penna/zig-zag sono cambiate.
    fn persist_settings(&mut self) {
        let (view, pen, zz) = {
            let s = self.hub.lock();
            (s.view, s.pen, s.zigzag)
        };
        let mut n = self.settings.clone();
        n.rot = view.rot;
        n.flip_x = view.flip_x;
        n.flip_y = view.flip_y;
        n.pen_color = pen.color;
        n.pen_width = pen.width;
        n.zigzag = zz;
        n.thumb_size = self.lui.thumb_size;
        if n != self.settings {
            n.save();
            self.settings = n;
        }
    }

    fn board_keyboard(&self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        if self.doc_keys(ctx, actions) {
            return;
        }
        // Scorciatoie con Cmd: lette dagli eventi (anche se Cmd è già rilasciato).
        let (redo, undo) = ctx.input_mut(|i| {
            let shift_cmd = egui::Modifiers::COMMAND | egui::Modifiers::SHIFT;
            let redo = i.consume_key(shift_cmd, egui::Key::Z);
            (redo, !redo && i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z))
        });
        if redo || undo {
            actions.push(if redo { Action::Redo } else { Action::Undo });
            return;
        }
        if let Some(op) = zoom_keys(ctx) {
            actions.push(Action::Zoom(op));
            return;
        }
        ctx.input(|i| {
            // Cmd+C: il foglio come immagine negli appunti.
            if i.events.iter().any(|e| matches!(e, egui::Event::Copy)) {
                actions.push(Action::CopyImage);
                return;
            }
            if i.modifiers.command {
                return;
            }
            if i.modifiers.shift && i.key_pressed(egui::Key::H) {
                actions.push(Action::ClearHighlights);
                return;
            }
            let keys = [
                (egui::Key::F, Action::Fullscreen),
                (egui::Key::C, Action::Clear),
                (egui::Key::R, Action::Rotate),
                (egui::Key::X, Action::FlipX),
                (egui::Key::Y, Action::FlipY),
                (egui::Key::E, Action::ToggleEraser),
                (egui::Key::H, Action::ToggleTool),
                (egui::Key::Minus, Action::Width(-0.4)),
                (egui::Key::Plus, Action::Width(0.4)),
                (egui::Key::Equals, Action::Width(0.4)),
            ];
            for (k, a) in keys {
                if i.key_pressed(k) {
                    actions.push(a);
                }
            }
            if i.key_pressed(egui::Key::Escape) && self.fullscreen {
                actions.push(Action::Fullscreen);
            }
            let nums = [egui::Key::Num1, egui::Key::Num2, egui::Key::Num3, egui::Key::Num4, egui::Key::Num5, egui::Key::Num6];
            for (k, (_, c)) in nums.iter().zip(PALETTE) {
                if i.key_pressed(*k) {
                    actions.push(Action::Color(c));
                }
            }
        });
    }

}

/// Immagine di una pagina da far leggere a un modello: ritagliata attorno
/// alla scrittura (pagina intera se c'è un documento sotto).
fn page_png_for_ai(page: Page) -> Result<Vec<u8>> {
    let page = if page.bg.is_none() { export::crop_to_content(&page, 0.03) } else { page };
    export::page_to_png(&page, 2000)
}

/// Per un pezzo di URL (nomi di file con spazi, accenti…).
fn url_escape(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// Apre un file o un indirizzo con l'app predefinita.
fn open_path(target: &str) {
    crate::platform::open(target);
}

/// Apre un file con un'app precisa (es. Typora); se manca, con quella predefinita.
fn open_with(path: &std::path::Path, app: &str) {
    let target = path.to_string_lossy().to_string();
    if cfg!(target_os = "macos") && !app.is_empty() {
        let target2 = target.clone();
        let app = app.to_string();
        std::thread::spawn(move || {
            let ok = std::process::Command::new("open")
                .args(["-a", &app, &target2])
                .status()
                .is_ok_and(|s| s.success());
            if !ok {
                open_path(&target2);
            }
        });
    } else {
        open_path(&target);
    }
}

impl App {
    /// Allinea lo stato «schermo intero» a quello vero della finestra: su
    /// macOS si può uscire anche col pulsante verde, con Esc o con un gesto,
    /// e la barra deve ricomparire.
    fn sync_fullscreen(&mut self, ctx: &egui::Context) {
        let actual = ctx.input(|i| i.viewport().fullscreen);
        if let Some(actual) = actual {
            let waiting = self
                .fs_requested
                .is_some_and(|t| t.elapsed() < std::time::Duration::from_millis(1500));
            if actual == self.fullscreen {
                self.fs_requested = None;
            } else if !waiting {
                self.fullscreen = actual;
                self.fs_requested = None;
            }
        }
        if !self.fullscreen {
            self.bar_peek = false;
            return;
        }
        // Mouse in cima: mostra la barra; sceso sotto la barra: la nasconde.
        if let Some(p) = ctx.input(|i| i.pointer.hover_pos()) {
            if p.y < 6.0 {
                self.bar_peek = true;
            } else if p.y > self.bar_height + 30.0 {
                self.bar_peek = false;
            }
        }
    }
}

impl App {
    /// Il foglio della lavagna (con i righelli), gomma ed evidenziatore col mouse.
    fn board_ui(&mut self, root: &mut egui::Ui, ctx: &egui::Context, actions: &mut Vec<Action>) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(root, |ui| {
                let full = ui.available_rect_before_wrap();
                // righelli in cm sopra e a sinistra (non a schermo intero)
                let rulers = self.settings.rulers && !self.fullscreen;
                let rect = if rulers {
                    egui::Rect::from_min_max(full.min + egui::vec2(chrome::RULER, chrome::RULER), full.max)
                } else {
                    full
                };
                let resp = ui.interact(rect, egui::Id::new("board-page"), egui::Sense::click_and_drag());
                // La firma da posizionare ha la precedenza su zoom e spostamento.
                let (erasing, hl_tool) = {
                    let s = self.hub.lock();
                    (s.eraser, s.tool == crate::scene::Tool::Highlighter && !s.eraser)
                };
                if !self.stamp_input(ui, &resp) {
                    // Con la gomma o l'evidenziatore il tasto sinistro
                    // cancella o evidenzia (si sposta col centrale).
                    if erasing {
                        self.mouse_eraser(ui, &resp);
                    } else if self.mouse_erase_last.take().is_some() {
                        self.hub.lock().mouse_erase_end();
                    }
                    if hl_tool {
                        self.mouse_highlighter(ui, &resp);
                    } else if self.mouse_hl_on {
                        self.mouse_hl_on = false;
                        self.hub.mouse_highlight(crate::stroke::PenPhase::Up, 0.0, 0.0, false);
                    }
                    // rotazione a due dita: +1 = orario; nella vista un passo di `rot` è antiorario
                    match self.board_zoom.handle_input(ui, &resp, !erasing && !hl_tool) {
                        1 => actions.push(Action::RotateBy(3)),
                        -1 => actions.push(Action::RotateBy(1)),
                        _ => {}
                    }
                    if hl_tool && resp.hover_pos().is_some_and(|p| self.docui.last_page.contains(p)) {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                    }
                }
                let bgtex = self.page_image(ctx);
                let scene = self.hub.lock();
                let painter = ui.painter_at(rect);
                let paper = Some((paper::Paper::from_code(&self.settings.paper), self.settings.paper_margin));
                let page = paint_page(&painter, rect, &scene.drawing, scene.view, bgtex, &mut self.board_zoom, paper);
                self.docui.last_page = page;
                self.docui.last_view = rect;
                if rulers {
                    let (pw, ph) = crate::scene::page_dims(&scene.drawing);
                    let g = PageGeom::new(scene.view, pw, ph);
                    let h = egui::Rect::from_min_max(egui::pos2(rect.min.x, full.min.y), egui::pos2(full.max.x, rect.min.y));
                    let v = egui::Rect::from_min_max(egui::pos2(full.min.x, rect.min.y), egui::pos2(rect.min.x, full.max.y));
                    let t = theme::Tokens::get(ui.ctx());
                    // unità del pad: 10 µm, cioè 1000 per cm
                    chrome::paint_rulers(&ui.painter_at(full), h, v, page, (g.w / 1000.0, g.h / 1000.0), resp.hover_pos(), &t);
                }
                self.paint_doc_overlay(&painter, page, &scene);
                // Alt + passaggio del mouse: segna l'evidenziatura; Alt+clic la toglie.
                if ui.input(|i| i.modifiers.alt) {
                    if let Some(pos) = resp.hover_pos() {
                        if let Some(idx) = hit_highlight(&scene.drawing, scene.view, page, pos) {
                            outline_stroke(&painter, &scene.drawing, scene.view, page, idx);
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            if resp.clicked() {
                                actions.push(Action::RemoveStroke(idx));
                            }
                        }
                    }
                }
                if scene.eraser {
                    // cerchio della gomma sotto il mouse
                    if let Some(p) = resp.hover_pos().filter(|p| page.contains(*p)) {
                        painter.circle_filled(p, ERASER_PX, egui::Color32::from_rgba_unmultiplied(230, 70, 60, 40));
                        painter.circle_stroke(p, ERASER_PX, egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(200, 50, 40)));
                        ui.ctx().set_cursor_icon(egui::CursorIcon::None);
                    }
                    let pos = egui::pos2(page.left().max(rect.left()) + 10.0, page.top().max(rect.top()) + 10.0);
                    let text = format!("{} GOMMA", ic::ERASER);
                    let galley = painter.layout_no_wrap(text, egui::FontId::proportional(18.0), egui::Color32::WHITE);
                    let r = egui::Rect::from_min_size(pos, galley.size()).expand2(egui::vec2(10.0, 5.0));
                    painter.rect_filled(r, 6.0, egui::Color32::from_rgb(200, 60, 40));
                    painter.galley(pos, galley, egui::Color32::WHITE);
                } else if scene.tool == crate::scene::Tool::Highlighter {
                    // etichetta ben visibile (anche a schermo intero e sulla TV)
                    let pos = egui::pos2(page.left().max(rect.left()) + 10.0, page.top().max(rect.top()) + 10.0);
                    let text = format!("{} EVIDENZIATORE", ic::HIGHLIGHTER);
                    let galley = painter.layout_no_wrap(text, egui::FontId::proportional(18.0), egui::Color32::BLACK);
                    let r = egui::Rect::from_min_size(pos, galley.size()).expand2(egui::vec2(10.0, 5.0));
                    painter.rect_filled(r, 6.0, egui::Color32::from_rgb(255, 225, 60));
                    painter.galley(pos, galley, egui::Color32::BLACK);
                }
            });
    }
}

impl eframe::App for App {
    fn on_exit(&mut self) {
        self.save_doc_on_exit();
        // la finestra Chat/Appunti vive del server di questa app
        if let Some(mut ch) = self.viewer.take() {
            let _ = ch.kill();
        }
    }

    fn ui(&mut self, root: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx_owned = root.ctx().clone();
        let ctx = &ctx_owned;
        self.drain_events();
        self.doc_tick(ctx);
        if let Some(n) = self.hub.lock().last_erased.take() {
            self.status = match n {
                0 => "Gomma: niente da cancellare lì.".to_string(),
                1 => "Cancellato 1 tratto (Cmd+Z per annullare).".to_string(),
                n => format!("Cancellati {n} tratti (Cmd+Z per annullare)."),
            };
        }
        if let Some(t) = self.hub.lock().tool_toggled.take() {
            self.status = match t {
                crate::scene::Tool::Highlighter => "Tocco lungo: evidenziatore attivo.".into(),
                crate::scene::Tool::Pen => "Tocco lungo: penna attiva.".into(),
            };
        }

        self.sync_fullscreen(ctx);
        if self.applied_theme != self.settings.ui_theme {
            theme::apply(ctx, theme::Appearance::from_code(&self.settings.ui_theme));
            self.applied_theme = self.settings.ui_theme.clone();
            let js = format!("document.documentElement.dataset.theme = {}", embed::js_str(&self.settings.ui_theme));
            for slot in [embed::Slot::Note, embed::Slot::Chat, embed::Slot::Settings] {
                self.embeds.eval(slot, &js);
            }
        }

        let mut actions = Vec::new();
        // messaggi dalle pagine incorporate (appunto, chat)
        for (slot, v) in self.embeds.poll() {
            self.on_page_msg(slot, v, &mut actions);
        }
        // A tutto schermo la cornice sparisce: resta solo il foglio. Torna
        // portando il mouse in cima allo schermo (F o Esc per uscire).
        let chrome = !self.fullscreen || self.bar_peek || self.backstage.is_some();
        let mut bar = 0.0;
        if chrome {
            bar += self.title_bar(root, &mut actions);
        }
        if let Some(page) = self.backstage {
            self.backstage_ui(root, page, &mut actions);
        } else {
            if chrome {
                bar += self.tab_strip(root, &mut actions);
                if self.show_ribbon || self.mode == Mode::Library {
                    bar += self.ribbon_panel(root, &mut actions);
                }
            }
            self.bar_height = bar;
            if !self.fullscreen {
                self.status_bar(root, &mut actions);
            }
            match self.mode {
                Mode::Board => {
                    if chrome {
                        self.connect_banner(root, &mut actions);
                    }
                    // la chat a destra resta aperta anche cambiando scheda
                    self.chat_pane(root, frame);
                    if chrome {
                        self.files_strip(root, &mut actions);
                    }
                    match self.center {
                        workspace::Center::Board => {
                            if self.palette.is_none() {
                                self.board_keyboard(ctx, &mut actions);
                            }
                            self.board_ui(root, ctx, &mut actions);
                        }
                        _ => self.web_center(root, frame),
                    }
                }
                Mode::Library => {
                    self.chat_pane(root, frame);
                    self.library_ui(root, &mut actions)
                }
            }
        }
        self.menu_keys(ctx, &mut actions);
        self.chat_drop(ctx, &mut actions);
        self.help_window(ctx);
        self.palette_ui(ctx, &mut actions);
        self.place_pages(ctx);

        for a in actions {
            self.apply(a, ctx);
        }
        self.persist_settings();

        // Live = ridisegno continuo (in archivio basta ridisegnare ogni tanto).
        if self.mode == Mode::Board {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
    }
}

/// macOS: barra dei menu dell'app. Serve il menu «Modifica» perché Cmd+C,
/// Cmd+V e Cmd+A funzionino nelle pagine incorporate (chat, appunti).
#[cfg(target_os = "macos")]
fn mac_menu() -> Option<muda::Menu> {
    use muda::{Menu, PredefinedMenuItem, Submenu};
    let menu = Menu::new();
    let app = Submenu::new("OpenSlate", true);
    let edit = Submenu::new("Modifica", true);
    app.append_items(&[
        &PredefinedMenuItem::about(Some("Informazioni su OpenSlate"), None),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::hide(None),
        &PredefinedMenuItem::hide_others(None),
        &PredefinedMenuItem::show_all(None),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::quit(None),
    ])
    .ok()?;
    edit.append_items(&[
        &PredefinedMenuItem::cut(None),
        &PredefinedMenuItem::copy(None),
        &PredefinedMenuItem::paste(None),
        &PredefinedMenuItem::select_all(None),
    ])
    .ok()?;
    let window = Submenu::new("Finestra", true);
    window
        .append_items(&[&PredefinedMenuItem::minimize(None), &PredefinedMenuItem::maximize(None), &PredefinedMenuItem::fullscreen(None)])
        .ok()?;
    menu.append_items(&[&app, &edit, &window]).ok()?;
    menu.init_for_nsapp();
    Some(menu)
}
