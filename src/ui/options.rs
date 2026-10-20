//! Opzioni (pagina File › Opzioni), scorciatoie globali, finestre di aiuto
//! («Scorciatoie da tastiera», «Come si collega lo Slate»).

use crate::i18n;
use super::theme::Appearance;
use super::{doc_view::DocAction, Action, App};
use crate::scene::{Pen, View};
use crate::settings::Settings;
use eframe::egui;
use egui_phosphor::regular as ic;

/// Tasto Comando: «Cmd» su Mac, «Ctrl» altrove (il font non ha ⌘ ⇧ ⌥).
fn cmd(k: &str) -> String {
    format!("{}+{k}", if cfg!(target_os = "macos") { "Cmd" } else { "Ctrl" })
}

/// Finestra delle impostazioni: si modifica una copia, che vale con «Applica».
pub(super) struct Prefs {
    pub draft: Settings,
    pub tab: PrefTab,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PrefTab {
    Aspetto,
    Penna,
    Pad,
    Web,
    Ai,
    Archivio,
    Avanzate,
}

impl PrefTab {
    const ALL: [PrefTab; 7] = [PrefTab::Aspetto, PrefTab::Penna, PrefTab::Pad, PrefTab::Web, PrefTab::Ai, PrefTab::Archivio, PrefTab::Avanzate];
    fn label(self) -> String {
        match self {
            PrefTab::Aspetto => format!("{} Aspetto", ic::PAINT_BRUSH),
            PrefTab::Penna => format!("{} Penna ed evidenziatore", ic::PEN),
            PrefTab::Pad => format!("{} Pad e foglio", ic::DEVICE_TABLET),
            PrefTab::Web => format!("{} Diretta web", ic::BROADCAST),
            PrefTab::Ai => format!("{} Trascrizione e AI", ic::SPARKLE),
            PrefTab::Archivio => format!("{} Archivio ed export", ic::ARCHIVE),
            PrefTab::Avanzate => format!("{} Avanzate", ic::WRENCH),
        }
    }
}

impl App {
    /// Scorciatoie globali dei menu (Cmd+, Cmd+O Cmd+S Cmd+Shift+S Cmd+W Cmd+Q).
    pub(super) fn menu_keys(&self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        ctx.input_mut(|i| {
            let c = egui::Modifiers::COMMAND;
            let cs = egui::Modifiers::COMMAND | egui::Modifiers::SHIFT;
            if i.consume_key(c, egui::Key::Comma) {
                actions.push(Action::OpenPrefs);
            } else if i.consume_key(c, egui::Key::F) || i.consume_key(c, egui::Key::K) {
                actions.push(Action::Palette);
            } else if i.consume_key(c, egui::Key::O) {
                actions.push(Action::Doc(DocAction::OpenDialog));
            } else if i.consume_key(cs, egui::Key::S) {
                actions.push(Action::Doc(DocAction::SaveSigned));
            } else if i.consume_key(c, egui::Key::S) {
                actions.push(if self.docui.doc.is_some() { Action::Doc(DocAction::SaveSigned) } else { Action::SaveBoard });
            } else if i.consume_key(c, egui::Key::W) {
                if self.docui.doc.is_some() {
                    actions.push(Action::Doc(DocAction::Close));
                }
            } else if i.consume_key(c, egui::Key::Q) {
                actions.push(Action::Quit);
            } else if i.consume_key(egui::Modifiers::NONE, egui::Key::F1) {
                actions.push(Action::Backstage(Some(super::backstage::BsPage::Info)));
            }
        });
    }

    /// Pagina File › Opzioni: elenco delle sezioni a sinistra, la sezione a
    /// destra; si modifica una copia, che vale con «Applica» o «OK».
    pub(super) fn prefs_panel(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let Some(p) = &mut self.prefs else { return };
        let t = super::theme::Tokens::get(ui.ctx());
        let (mut apply, mut ok, mut cancel, mut ai) = (false, false, false, false);
        ui.label(egui::RichText::new(i18n::t("Opzioni")).font(super::theme::semibold(24.0)).color(t.text));
        ui.add_space(14.0);
        egui::Panel::bottom("prefs_buttons").frame(egui::Frame::NONE.inner_margin(egui::Margin { left: 0, right: 0, top: 10, bottom: 0 })).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(i18n::t("Salvate in settings.conf")).weak().small());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ok = ui.add(egui::Button::new(egui::RichText::new(i18n::t("   OK   ")).color(t.on_accent)).fill(t.accent)).clicked();
                    apply = ui.button(i18n::t("Applica")).clicked();
                    cancel = ui.button(i18n::tc("cancel", "Annulla")).clicked();
                    if ui.button(i18n::t("Valori di serie")).on_hover_text(i18n::t("Rimette i valori iniziali (non ancora salvati)")).clicked() {
                        p.draft = Settings::default();
                    }
                });
            });
        });
        egui::Panel::left("prefs_tabs").exact_size(230.0).resizable(false).frame(egui::Frame::NONE).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            for tab in PrefTab::ALL {
                let on = p.tab == tab;
                let (r, resp) = ui.allocate_exact_size(egui::vec2(214.0, 32.0), egui::Sense::click());
                if on {
                    ui.painter().rect_filled(r, 4.0, t.checked);
                    ui.painter().rect_filled(egui::Rect::from_min_size(r.min + egui::vec2(0.0, 7.0), egui::vec2(3.0, 18.0)), 2.0, t.accent);
                } else if resp.hovered() {
                    ui.painter().rect_filled(r, 4.0, t.hover);
                }
                ui.painter().text(r.min + egui::vec2(12.0, 16.0), egui::Align2::LEFT_CENTER, i18n::tr(&tab.label()).into_owned(), if on { super::theme::semibold(13.0) } else { super::theme::regular(13.0) }, if on { t.accent_text } else { t.text });
                if resp.clicked() {
                    p.tab = tab;
                }
            }
        });
        egui::CentralPanel::default().frame(egui::Frame::NONE.inner_margin(egui::Margin { left: 20, right: 0, top: 0, bottom: 0 })).show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| prefs_page(ui, p, &mut ai));
        });
        if apply || ok {
            let n = p.draft.clone();
            self.apply_settings(n);
        }
        if ai {
            actions.push(Action::AiSettings);
        }
        if ok || cancel {
            actions.push(Action::Backstage(None));
        }
    }

    /// Le impostazioni scelte valgono subito e si salvano.
    pub(super) fn apply_settings(&mut self, n: Settings) {
        let old_view = self.hub.lock().view;
        self.hub.edit(|s| {
            s.view = View { rot: n.rot, flip_x: n.flip_x, flip_y: n.flip_y };
            s.pen = Pen { color: n.pen_color, width: n.pen_width };
            s.highlighter = Pen { color: n.highlight_color, width: n.highlight_width };
            s.zigzag = n.zigzag;
            s.longpress = n.longpress_toggle;
        });
        if old_view != self.hub.lock().view {
            self.lui.thumbs.clear();
        }
        self.lui.thumb_size = n.thumb_size;
        let port_changed = n.web_port != self.settings.web_port && self.web_url.is_some();
        if n.ui_lang != self.settings.ui_lang {
            crate::i18n::set_lang(crate::i18n::resolve(&n.ui_lang));
            self.lang_changed = true;
        }
        n.save();
        self.settings = n;
        self.status = if port_changed {
            "Impostazioni salvate. La nuova porta della diretta web vale al prossimo avvio dell'app.".into()
        } else {
            "Impostazioni salvate.".into()
        };
    }

    /// Finestre di aiuto.
    pub(super) fn help_window(&mut self, ctx: &egui::Context) {
        let Some(page) = self.help else { return };
        let mut open = true;
        let title = match page {
            HelpPage::Tasti => format!("{} Scorciatoie da tastiera", ic::KEYBOARD),
            HelpPage::Collegamento => format!("{} Come si collega lo Slate", ic::BLUETOOTH),
        };
        egui::Window::new(i18n::tr(&title).into_owned()).open(&mut open).collapsible(false).resizable(false).default_width(460.0).show(ctx, |ui| match page {
            HelpPage::Tasti => {
                egui::Grid::new("keys").striped(true).spacing([24.0, 6.0]).show(ui, |ui| {
                    let rows: &[(&str, String)] = &[
                        ("Annulla / Ripeti", format!("{} / {}", cmd("Z"), cmd("Shift+Z"))),
                        ("Copia il foglio come immagine", cmd("C")),
                        ("Apri documento / Chiudi", format!("{} / {}", cmd("O"), cmd("W"))),
                        ("Salva (archivio o PDF del documento)", cmd("S")),
                        ("Opzioni", cmd(",")),
                        ("Cerca un comando", cmd("F")),
                        ("Zoom + / - / pagina intera", format!("{} / {} / {}", cmd("+"), cmd("-"), cmd("0"))),
                        ("Trackpad: zoom / sposta", "pizzico / due dita".into()),
                        ("Trackpad: gira il foglio", "due dita che ruotano".into()),
                        ("Evidenziatore / Gomma", "H / E".into()),
                        ("Togli le evidenziature", "Shift+H".into()),
                        ("Colori", "1 … 6".into()),
                        ("Spessore", "+ / -".into()),
                        ("Pulisci il foglio", "C".into()),
                        ("Ruota / Specchia", "R / X / Y".into()),
                        ("Schermo intero / esci", "F / Esc".into()),
                        ("Pagine del documento", "PagSu / PagGiù".into()),
                        ("Evidenziatore col mouse: linea dritta", "Shift + trascina".into()),
                        ("Togli un'evidenziatura", "Alt + clic".into()),
                        ("Sposta il foglio (con gomma o evidenziatore)", "tasto centrale".into()),
                    ];
                    for (a, k) in rows {
                        ui.label(i18n::tr(&*a).into_owned());
                        ui.label(egui::RichText::new(i18n::tr(&k).into_owned()).monospace().strong());
                        ui.end_row();
                    }
                });
            }
            HelpPage::Collegamento => {
                ui.label(egui::RichText::new(i18n::t("Prima volta (o dopo aver usato Inkspace)")).strong());
                ui.label(i18n::t("Tieni premuto il pulsante del pad ~6 s, finché il LED lampeggia blu. Poi «Collega» e, quando richiesto, premi una volta il pulsante."));
                ui.add_space(6.0);
                ui.label(egui::RichText::new(i18n::t("Le altre volte (LED verde)")).strong());
                ui.label(i18n::t("Premi brevemente il pulsante del pad, poi «Collega»."));
                ui.add_space(6.0);
                ui.label(egui::RichText::new(i18n::t("La spia in alto a destra")).strong());
                ui.label(i18n::t("Verde: collegato (piena = in diretta). Arancio lampeggiante: mi sto collegando. Rossa: scollegato."));
                ui.add_space(6.0);
                ui.label(egui::RichText::new(i18n::t("Il pad ricorda una sola app alla volta: usare Inkspace richiede di registrare di nuovo questa.")).weak());
            }
        });
        if !open {
            self.help = None;
        }
    }
}

/// Pagine di aiuto.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum HelpPage {
    Tasti,
    Collegamento,
}

fn row(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.label(crate::i18n::tr(label));
    add(ui);
    ui.end_row();
}

fn hint(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(crate::i18n::tr(text)).weak().small());
}

/// Contenuto di una pagina delle impostazioni.
fn prefs_page(ui: &mut egui::Ui, p: &mut Prefs, ai: &mut bool) {
    let d = &mut p.draft;
    let grid = |ui: &mut egui::Ui, id: &str, f: &mut dyn FnMut(&mut egui::Ui)| {
        egui::Grid::new(id).num_columns(2).spacing([16.0, 10.0]).show(ui, |ui| f(ui));
    };
    ui.heading(i18n::tr(&p.tab.label()).into_owned());
    ui.add_space(6.0);
    match p.tab {
        PrefTab::Aspetto => grid(ui, "aspetto", &mut |ui| {
            row(ui, "Lingua", |ui| {
                ui.vertical(|ui| {
                    use crate::i18n::{self as l, Lang};
                    let auto = format!("{} ({})", i18n::t("Automatica, come il sistema"), l::system().native());
                    let shown = match Lang::from_code(&d.ui_lang) {
                        Some(x) if d.ui_lang != "auto" => x.native().to_string(),
                        _ => auto.clone(),
                    };
                    egui::ComboBox::from_id_salt("ui_lang").selected_text(shown).width(240.0).height(400.0).show_ui(ui, |ui| {
                        ui.selectable_value(&mut d.ui_lang, "auto".to_string(), auto);
                        for x in Lang::ALL {
                            ui.selectable_value(&mut d.ui_lang, x.code().to_string(), x.native());
                        }
                    });
                    hint(ui, "vale subito per tutta l'app (il menu di macOS al prossimo avvio)");
                });
            });
            row(ui, "Tema dell'interfaccia", |ui| {
                ui.vertical(|ui| {
                    for a in Appearance::ALL {
                        if ui.radio(Appearance::from_code(&d.ui_theme) == a, i18n::tr(&a.label()).into_owned()).clicked() {
                            d.ui_theme = a.code().to_string();
                        }
                    }
                    hint(ui, "il foglio resta bianco anche col tema scuro");
                });
            });
            row(ui, "Righelli", |ui| {
                ui.checkbox(&mut d.rulers, i18n::t("righelli in centimetri attorno al foglio"));
            });
            row(ui, "Foglio della lavagna", |ui| {
                ui.vertical(|ui| {
                    let cur = super::paper::Paper::from_code(&d.paper);
                    let mut pick = cur;
                    egui::ComboBox::from_id_salt("paper")
                        .selected_text(i18n::tr(&cur.label()).into_owned())
                        .width(240.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut pick, super::paper::Paper::Bianco, i18n::tr(&super::paper::Paper::Bianco.label()).into_owned());
                            for (title, group) in super::paper::Paper::GROUPS {
                                ui.separator();
                                ui.weak(title);
                                for &p in group {
                                    ui.selectable_value(&mut pick, p, i18n::tr(&p.label()).into_owned());
                                }
                            }
                        });
                    if pick != cur {
                        d.paper = pick.code().to_string();
                    }
                    ui.checkbox(&mut d.paper_margin, i18n::t("margini rossi a sinistra e a destra (fogli a righe)"));
                    hint(ui, "solo sullo schermo: non va nelle immagini per l'AI né negli export");
                });
            });
        }),
        PrefTab::Penna => grid(ui, "penna", &mut |ui| {
            row(ui, "Colore della penna", |ui| {
                ui.color_edit_button_srgb(&mut d.pen_color);
            });
            row(ui, "Spessore della penna", |ui| {
                ui.add(egui::Slider::new(&mut d.pen_width, 0.4..=12.0).step_by(0.2));
            });
            row(ui, "Colore dell'evidenziatore", |ui| {
                ui.horizontal(|ui| {
                    ui.color_edit_button_srgb(&mut d.highlight_color);
                    hint(ui, "chiaro: sta sotto l'inchiostro");
                });
            });
            row(ui, "Spessore dell'evidenziatore", |ui| {
                ui.add(egui::Slider::new(&mut d.highlight_width, 4.0..=40.0).step_by(1.0));
            });
            row(ui, "Tocco lungo col pennino", |ui| {
                ui.checkbox(&mut d.longpress_toggle, i18n::t("punta ferma ~1 s = penna ↔ evidenziatore"));
            });
            row(ui, "Zig-zag", |ui| {
                ui.vertical(|ui| {
                    ui.checkbox(&mut d.zigzag, i18n::t("uno zig-zag del pennino cancella (anche senza Gomma)"));
                    hint(ui, "sconsigliato con le firme, che spesso sono zig-zag");
                });
            });
        }),
        PrefTab::Pad => grid(ui, "pad", &mut |ui| {
            row(ui, "Rotazione del foglio", |ui| {
                ui.horizontal(|ui| {
                    for (r, t) in [(0, "0°"), (1, "90°"), (2, "180°"), (3, "270°")] {
                        ui.radio_value(&mut d.rot, r, i18n::tr(&t).into_owned());
                    }
                });
            });
            row(ui, "Specchia", |ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut d.flip_x, i18n::t("orizzontale"));
                    ui.checkbox(&mut d.flip_y, i18n::t("verticale"));
                });
            });
            row(ui, "Scaricando dal pad", |ui| {
                ui.vertical(|ui| {
                    ui.checkbox(&mut d.sync_delete, i18n::t("togli le pagine dal pad dopo averle salvate"));
                    hint(ui, "senza, si scarica solo la più vecchia (il pad non ne dà altre)");
                });
            });
        }),
        PrefTab::Web => grid(ui, "web", &mut |ui| {
            row(ui, "Porta", |ui| {
                ui.add(egui::DragValue::new(&mut d.web_port).range(1024..=65535));
            });
            row(ui, "All'avvio", |ui| {
                ui.checkbox(&mut d.web_auto, i18n::t("avvia subito la diretta web"));
            });
            row(ui, "", |ui| hint(ui, "La diretta si apre nel browser della TV, del telefono o di un PC di casa."));
        }),
        PrefTab::Ai => grid(ui, "ai", &mut |ui| {
            row(ui, "Motore della trascrizione", |ui| {
                ui.vertical(|ui| {
                    ui.radio_value(&mut d.ocr_engine, "ai".to_string(), i18n::t("Agenti AI: testo in Markdown, con tabelle e formule (consigliato)"));
                    ui.radio_value(&mut d.ocr_engine, "vision".to_string(), i18n::t("Apple Vision: offline, solo testo"));
                });
            });
            row(ui, "Lingue (Apple Vision)", |ui| {
                ui.text_edit_singleline(&mut d.ocr_langs);
            });
            row(ui, "App per i file .md", |ui| {
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut d.md_app);
                    hint(ui, "per il pulsante «Typora»");
                });
            });
            row(ui, "Riquadro della chat", |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut d.chat_side, "sinistra".to_string(), i18n::t("a sinistra del foglio"));
                        ui.radio_value(&mut d.chat_side, "destra".to_string(), i18n::t("a destra"));
                    });
                    hint(ui, "la larghezza si regola trascinando il bordo della chat e resta");
                });
            });
            row(ui, "Agenti, chiavi, contesto", |ui| {
                if ui.button(i18n::tf(format!("{} Impostazioni AI…", ic::SPARKLE))).clicked() {
                    *ai = true;
                }
            });
        }),
        PrefTab::Archivio => grid(ui, "arch", &mut |ui| {
            row(ui, "Cartella per salvare ed esportare", |ui| {
                ui.vertical(|ui| {
                    let shown = super::savedlg::start_dir(d);
                    ui.label(egui::RichText::new(i18n::tr(&shown.display().to_string()).into_owned()).monospace());
                    ui.horizontal(|ui| {
                        if ui.button(i18n::t("Scegli…")).clicked() {
                            if let Some(dir) = rfd::FileDialog::new()
                                .set_title(i18n::t("Cartella proposta quando si salva o si esporta"))
                                .set_directory(&shown)
                                .pick_folder()
                            {
                                d.save_dir = dir.to_string_lossy().into();
                            }
                        }
                        if !d.save_dir.is_empty() && ui.button(i18n::t("Usa quella di serie")).clicked() {
                            d.save_dir.clear();
                        }
                        if ui.button(i18n::t("Apri")).clicked() {
                            crate::platform::open(&shown);
                        }
                    });
                    hint(ui, "Salva ed Esporta chiedono sempre dove salvare: si parte da qui e si ricorda l'ultima cartella scelta");
                });
            });
            row(ui, "Anteprime dell'archivio", |ui| {
                ui.add(egui::Slider::new(&mut d.thumb_size, 160.0..=640.0).suffix(i18n::t(" pt")));
            });
            row(ui, "«Copia immagine»", |ui| {
                ui.vertical(|ui| {
                    ui.checkbox(&mut d.copy_crop, i18n::t("ritaglia attorno alla scrittura"));
                    ui.add(egui::Slider::new(&mut d.copy_size, 256..=4000).suffix(i18n::t(" px")).text(i18n::t("lato lungo")));
                });
            });
        }),
        PrefTab::Avanzate => grid(ui, "adv", &mut |ui| {
            row(ui, "Comando dopo ogni file", |ui| {
                ui.vertical(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut d.hook).desired_width(320.0).hint_text(i18n::t("es. ~/bin/invia.sh")));
                    hint(ui, "riceve il percorso del file prodotto come $1 (export, trascrizioni)");
                });
            });
            row(ui, "File delle impostazioni", |ui| {
                if ui.button(i18n::t("Apri settings.conf")).clicked() {
                    crate::platform::open("settings.conf");
                }
            });
        }),
    }
}
