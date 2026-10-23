//! Contenuto delle schede della barra multifunzione (Home, Documento,
//! Visualizza, AI), con i pulsanti grandi e piccoli in gruppi come in Word.
//! Ogni comando ha un posto solo: il pad sta nella spia in alto a destra,
//! condivisione ed export nel pulsante «Condividi», l'aiuto in «?» / File.

use crate::i18n;
use super::chrome::menu_item;
use super::doc_view::DocAction;
use super::theme::{medium, regular, Tokens};
use super::widgets::{big, big_ex, column, group, small, small_ex};
use super::zoom::ZoomOp;
use super::{Action, App, PALETTE};
use crate::scene::Tool;
use eframe::egui;
use egui::{pos2, vec2, Align2, Sense, Stroke};
use egui_phosphor::regular as ic;

fn check_icon(on: bool) -> &'static str {
    if on {
        ic::CHECK_SQUARE
    } else {
        ic::SQUARE
    }
}

fn radio_icon(on: bool) -> &'static str {
    if on {
        ic::RADIO_BUTTON
    } else {
        ic::CIRCLE
    }
}

impl App {
    pub(super) fn ribbon_home(&self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let (pen, eraser, tool) = {
            let s = self.hub.lock();
            (s.pen, s.eraser, s.tool)
        };
        group(ui, "Strumenti", |ui| {
            if big_ex(ui, ic::PEN, "Penna", "Penna: si scrive col pennino, a mano libera", true, tool == Tool::Pen && !eraser, false).clicked() {
                actions.push(Action::SetTool(Tool::Pen));
            }
            if big_ex(
                ui,
                ic::HIGHLIGHTER,
                "Evidenziatore",
                "Evidenziatore (H; col pennino tocco lungo, se acceso nelle Opzioni).\nAnche col mouse: tasto sinistro; con Shift linea dritta orizzontale o verticale.\nCol mouse il foglio si sposta col tasto centrale.",
                true,
                tool == Tool::Highlighter && !eraser,
                false,
            )
            .clicked()
            {
                actions.push(Action::SetTool(Tool::Highlighter));
            }
            if big_ex(ui, ic::ERASER, "Gomma", "Gomma (E): col pennino o col mouse cancella ciò che tocca. Resta attiva finché non la spegni.", true, eraser, false).clicked() {
                actions.push(Action::ToggleEraser);
            }
            let r = big_ex(ui, ic::BROOM, "Pulisci", "Pulisci il foglio o solo le evidenziature (Cmd+Z per tornare indietro)", true, false, true);
            egui::Popup::menu(&r).show(|ui| {
                if menu_item(ui, true, ic::BROOM, "Tutto il foglio", "C") {
                    actions.push(Action::Clear);
                }
                if menu_item(ui, true, ic::HIGHLIGHTER, "Solo le evidenziature", "Shift+H") {
                    actions.push(Action::ClearHighlights);
                }
                if menu_item(ui, true, ic::PEN, "Evidenziature → scrittura", "") {
                    actions.push(Action::HighlightsToInk);
                }
                ui.label(egui::RichText::new(i18n::t("Una sola evidenziatura: Alt+clic sopra")).weak().small());
            });
        });
        group(ui, "Colore", |ui| {
            let t = Tokens::get(ui.ctx());
            egui::Grid::new("palette").spacing(vec2(2.0, 2.0)).show(ui, |ui| {
                for (n, (name, c)) in PALETTE.iter().enumerate() {
                    let (rect, resp) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::click());
                    let col = egui::Color32::from_rgb(c[0], c[1], c[2]);
                    let p = ui.painter();
                    if pen.color == *c {
                        p.rect_filled(rect, 4.0, t.checked);
                        p.rect_stroke(rect, 4.0, Stroke::new(1.0, t.accent.linear_multiply(0.5)), egui::StrokeKind::Inside);
                    } else if resp.hovered() {
                        p.rect_filled(rect, 4.0, t.hover);
                    }
                    p.circle_filled(rect.center(), 9.0, col);
                    p.circle_stroke(rect.center(), 9.0, Stroke::new(1.0, egui::Color32::from_gray(140)));
                    if resp.on_hover_text(format!("{} ({})", i18n::tr(name), n + 1)).clicked() {
                        actions.push(Action::Color(*c));
                    }
                    if n % 3 == 2 {
                        ui.end_row();
                    }
                }
            });
        });
        group(ui, "Spessore", |ui| {
            column(ui, |ui| {
                if small(ui, ic::PLUS, "Più spesso", "Tratto più spesso (+)").clicked() {
                    actions.push(Action::Width(0.4));
                }
                if small(ui, ic::MINUS, "Più sottile", "Tratto più sottile (-)").clicked() {
                    actions.push(Action::Width(-0.4));
                }
                // anteprima del tratto
                let t = Tokens::get(ui.ctx());
                let (r, _) = ui.allocate_exact_size(vec2(84.0, 22.0), Sense::hover());
                let col = egui::Color32::from_rgb(pen.color[0], pen.color[1], pen.color[2]);
                ui.painter().line_segment([pos2(r.min.x + 6.0, r.center().y), pos2(r.min.x + 40.0, r.center().y)], Stroke::new(pen.width.clamp(0.6, 9.0), col));
                ui.painter().text(pos2(r.min.x + 46.0, r.center().y), Align2::LEFT_CENTER, i18n::tf(format!("{:.1}", pen.width)), regular(11.5), t.text_dim);
            });
        });
        group(ui, "Foglio", |ui| {
            let doc = self.docui.doc.is_some();
            let cur = super::paper::Paper::from_code(&self.settings.paper);
            let r = big_ex(
                ui,
                ic::NOTEBOOK,
                "Righe e\nquadretti",
                "Righe, elementari, quadretti, millimetrata, piano cartesiano, puntinato, isometrica, pentagramma, Cornell",
                !doc,
                cur != super::paper::Paper::Bianco,
                true,
            );
            egui::Popup::menu(&r).show(|ui| {
                ui.set_min_width(250.0);
                let radio = |p| if p == cur { ic::RADIO_BUTTON } else { ic::CIRCLE };
                let b = super::paper::Paper::Bianco;
                if menu_item(ui, true, radio(b), b.label(), "") {
                    actions.push(Action::Paper(b));
                }
                for (title, group) in super::paper::Paper::GROUPS {
                    ui.separator();
                    ui.add_space(2.0);
                    ui.label(egui::RichText::new(i18n::tr(&title).into_owned()).font(medium(11.0)).color(Tokens::get(ui.ctx()).text_dim));
                    for &p in group {
                        if menu_item(ui, true, radio(p), p.label(), "") {
                            actions.push(Action::Paper(p));
                        }
                    }
                }
                ui.separator();
                if menu_item(ui, cur.ruled(), check_icon(self.settings.paper_margin), "Margini rossi (fogli a righe)", "") {
                    actions.push(Action::ToggleMargin);
                }
            });
            let lv = self.settings.beautify;
            let r = big_ex(
                ui,
                ic::MAGIC_WAND,
                "Bella\nscrittura",
                "Righe dritte, tratti lisci, parole regolari: solo come si vede ed esporta, i tratti veri restano (mai su documenti e firme)",
                !doc,
                lv > 0,
                true,
            );
            egui::Popup::menu(&r).show(|ui| {
                ui.set_min_width(250.0);
                let levels = [
                    (0u8, "Spenta", "La scrittura com'è"),
                    (1, "Leggera", "Tratti lisci e righe quasi dritte"),
                    (2, "Media", "Righe dritte, parole sulla riga, altezze simili"),
                    (3, "Forte", "Come la media, più decisa (anche spazi e inclinazione)"),
                ];
                for (l, label, tip) in levels {
                    let icon = if l == lv { ic::RADIO_BUTTON } else { ic::CIRCLE };
                    if menu_item(ui, true, icon, label, "") {
                        actions.push(Action::Beautify(l));
                    }
                    let t = Tokens::get(ui.ctx());
                    ui.horizontal(|ui| {
                        ui.add_space(26.0);
                        ui.label(egui::RichText::new(i18n::tr(tip).into_owned()).font(regular(11.0)).color(t.text_dim));
                    });
                }
                ui.separator();
                let shapes = self.settings.perfect_shapes;
                if menu_item(ui, true, check_icon(shapes), "Linee, frecce e cerchi perfetti", "") {
                    actions.push(Action::PerfectShapes(!shapes));
                }
                ui.separator();
                ui.add_space(2.0);
                ui.label(egui::RichText::new(i18n::tr("Come scrive l'AI sul foglio").into_owned()).font(medium(11.0)).color(Tokens::get(ui.ctx()).text_dim));
                let hand = self.settings.ai_hand;
                for (m, label) in [(0u8, "Stampatello pulito"), (1, "A mano (con le irregolarità della mano)"), (2, "Scrittura naturale")] {
                    let icon = if m == hand { ic::RADIO_BUTTON } else { ic::CIRCLE };
                    if menu_item(ui, true, icon, label, "") {
                        actions.push(Action::AiHand(m));
                    }
                }
            });
            let rw_on = self.settings.rewrite_show;
            let r = big_ex(
                ui,
                ic::SCRIBBLE_LOOP,
                "Riscrivi\nin bella",
                "L'AI legge quello che hai scritto, formule comprese, e lo mostra in una scrittura chiara con la matematica impaginata (frazioni, esponenti, sistemi). La tua scrittura resta: tieni premuto O per vederla",
                !doc,
                rw_on && self.settings.rewrite_live,
                true,
            );
            egui::Popup::menu(&r).show(|ui| {
                ui.set_min_width(270.0);
                if menu_item(ui, true, ic::SCRIBBLE_LOOP, "Riscrivi la pagina adesso", "") {
                    actions.push(Action::Rewrite(false));
                }
                if menu_item(ui, true, ic::ARROWS_CLOCKWISE, "Rileggi tutte le righe", "") {
                    actions.push(Action::Rewrite(true));
                }
                ui.separator();
                let live = self.settings.rewrite_live;
                if menu_item(ui, true, check_icon(live), "Anche mentre scrivo (alla pausa)", "") {
                    actions.push(Action::RewriteLive(!live));
                }
                if menu_item(ui, true, check_icon(rw_on), "Mostra le righe in bella", "") {
                    actions.push(Action::RewriteShow(!rw_on));
                }
                let t = Tokens::get(ui.ctx());
                ui.horizontal(|ui| {
                    ui.add_space(26.0);
                    ui.label(egui::RichText::new(i18n::tr("Tieni premuto O per vedere la tua scrittura").into_owned()).font(regular(11.0)).color(t.text_dim));
                });
            });
            if big_ex(
                ui,
                ic::FLOPPY_DISK,
                "Salva",
                if doc { "Con un documento aperto: «Salva PDF» (scheda Documento)" } else { "Conserva la lavagna come pagina dell'archivio (Cmd+S)" },
                !doc,
                false,
                false,
            )
            .clicked()
            {
                actions.push(Action::SaveBoard);
            }
        });
    }

    pub(super) fn ribbon_documento(&self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let doc = |a: DocAction| Action::Doc(a);
        if self.docui.capture.is_some() {
            group(ui, "Registrazione della firma", |ui| {
                if big(ui, ic::CHECK_CIRCLE, "Salva\nfirma", "Conserva la firma scritta sul pad").clicked() {
                    actions.push(doc(DocAction::SaveCapture));
                }
                if big(ui, ic::X_CIRCLE, &crate::i18n::tc("cancel", "Annulla"), "Torna al foglio senza registrare").clicked() {
                    actions.push(doc(DocAction::CancelCapture));
                }
            });
            return;
        }
        if self.docui.stamp.is_some() {
            group(ui, "Posiziona la firma", |ui| {
                if big(ui, ic::CHECK_CIRCLE, "Applica\nfirma", "Invio").clicked() {
                    actions.push(doc(DocAction::ApplyStamp));
                }
                if big(ui, ic::X_CIRCLE, &crate::i18n::tc("cancel", "Annulla"), "Esc").clicked() {
                    actions.push(doc(DocAction::CancelStamp));
                }
            });
            return;
        }
        let open = self.docui.doc.is_some();
        group(ui, "Documento", |ui| {
            if big_ex(
                ui,
                ic::FILE_ARROW_UP,
                if self.docui.importing { "Apro…" } else { "Apri" },
                "PDF, Word (.doc, .docx), .odt, .rtf, .md, .txt, .html, SVG, PNG, JPEG\nAnche trascinando il file sulla finestra (Cmd+O)",
                !self.docui.importing,
                false,
                false,
            )
            .clicked()
            {
                actions.push(doc(DocAction::OpenDialog));
            }
            let recent = crate::document::list();
            let r = big_ex(ui, ic::CLOCK_COUNTER_CLOCKWISE, "Recenti", "Documenti aperti di recente", !recent.is_empty(), false, true);
            egui::Popup::menu(&r).show(|ui| {
                ui.set_min_width(280.0);
                for d in recent.iter().take(20) {
                    if menu_item(ui, true, ic::FILE_TEXT, &d.title, d.created.get(..10).unwrap_or(&d.created)) {
                        actions.push(doc(DocAction::OpenRecent(d.id.clone())));
                    }
                }
            });
            if big_ex(ui, ic::X_SQUARE, "Chiudi", "Chiudi il documento: resta tra i recenti (Cmd+W)", open, false, false).clicked() {
                actions.push(doc(DocAction::Close));
            }
        });
        if let Some(d) = &self.docui.doc {
            let (cur, n) = (d.current, d.pages.len());
            group(ui, "Pagine", |ui| {
                column(ui, |ui| {
                    if small_ex(ui, ic::CARET_UP, "Precedente", "PagSu", cur > 0, false).clicked() {
                        actions.push(doc(DocAction::Page(cur - 1)));
                    }
                    if small_ex(ui, ic::CARET_DOWN, "Successiva", "PagGiù", cur + 1 < n, false).clicked() {
                        actions.push(doc(DocAction::Page(cur + 1)));
                    }
                    let t = Tokens::get(ui.ctx());
                    let (r, _) = ui.allocate_exact_size(vec2(96.0, 22.0), Sense::hover());
                    ui.painter().text(pos2(r.min.x + 6.0, r.center().y), Align2::LEFT_CENTER, i18n::tf(format!("Pagina {} di {n}", cur + 1)), regular(11.5), t.text_dim);
                });
            });
        }
        group(ui, "Firma", |ui| {
            if big(ui, ic::SIGNATURE, "Registra\nfirma", "Firma sul pad: la si conserva per i documenti").clicked() {
                actions.push(doc(DocAction::StartCapture));
            }
            let has = self.docui.signature.is_some();
            if big_ex(ui, ic::STAMP, "Inserisci\nfirma", if has { "Posa la firma registrata sulla pagina" } else { "Prima registra la firma" }, has, false, false).clicked() {
                actions.push(doc(DocAction::InsertSignature));
            }
        });
        group(ui, "Salva", |ui| {
            let word = self.docui.doc.as_ref().is_some_and(|d| d.original.is_some());
            let (glyph, label, tip) = if word {
                (ic::FILE_DOC, "Salva\nWord", "Il documento Word con quello che hai scritto: scegli dove salvarlo (Cmd+Shift+S). L'originale resta com'era")
            } else {
                (ic::FILE_PDF, "Salva\nPDF", "Il PDF con quello che hai scritto: scegli dove salvarlo (Cmd+Shift+S). L'originale resta com'era")
            };
            if big_ex(ui, glyph, label, tip, open, false, false).clicked() {
                actions.push(doc(DocAction::SaveSigned));
            }
            if big_ex(ui, ic::FILE_MAGNIFYING_GLASS, "Ultimo\nPDF", "Apri l'ultimo PDF salvato", self.docui.last_export.is_some(), false, false).clicked() {
                actions.push(doc(DocAction::OpenLastExport));
            }
        });
    }

    pub(super) fn ribbon_visualizza(&self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        group(ui, "Mostra", |ui| {
            column(ui, |ui| {
                if small(ui, check_icon(self.settings.rulers), "Righelli", "Righelli in centimetri attorno al foglio").clicked() {
                    actions.push(Action::ToggleRulers);
                }
                if small(ui, check_icon(self.show_ribbon), "Barra multifunzione", "Doppio clic su una scheda per nasconderla").clicked() {
                    actions.push(Action::ToggleRibbon);
                }
                if small(ui, check_icon(self.settings.compact_ui), "Solo il foglio", "Via la barra dei menu e gli strumenti; tornano col pulsante in alto o Cmd+Shift+F").clicked() {
                    actions.push(Action::ToggleCompact);
                }
            });
        });
        group(ui, "Zoom", |ui| {
            if big_ex(ui, ic::ARROWS_IN, "Pagina\nintera", "Cmd+0, doppio clic sul foglio", true, self.board_zoom.is_fit(), false).clicked() {
                actions.push(Action::Zoom(ZoomOp::Fit));
            }
            column(ui, |ui| {
                if small(ui, ic::MAGNIFYING_GLASS_PLUS, "Ingrandisci", "Cmd +, rotella").clicked() {
                    actions.push(Action::Zoom(ZoomOp::In));
                }
                if small(ui, ic::MAGNIFYING_GLASS_MINUS, "Riduci", "Cmd −, rotella").clicked() {
                    actions.push(Action::Zoom(ZoomOp::Out));
                }
                if small(ui, ic::MAGNIFYING_GLASS, "200%", "Il doppio della pagina intera").clicked() {
                    actions.push(Action::Zoom(ZoomOp::To(2.0)));
                }
            });
        });
        group(ui, "Orientamento", |ui| {
            if big(ui, ic::ARROWS_CLOCKWISE, "Ruota\ndi 90°", "Come tieni il pad (R)").clicked() {
                actions.push(Action::Rotate);
            }
            let r = big_ex(ui, ic::ARROWS_LEFT_RIGHT, "Specchia", "Specchia il foglio (X / Y)", true, false, true);
            egui::Popup::menu(&r).show(|ui| {
                if menu_item(ui, true, ic::ARROWS_LEFT_RIGHT, "Specchia orizzontale", "X") {
                    actions.push(Action::FlipX);
                }
                if menu_item(ui, true, ic::ARROWS_DOWN_UP, "Specchia verticale", "Y") {
                    actions.push(Action::FlipY);
                }
            });
        });
        group(ui, "Finestra", |ui| {
            if big(ui, ic::CORNERS_OUT, "Schermo\nintero", "F, Esc per uscire. La barra torna portando il mouse in cima.").clicked() {
                actions.push(Action::Fullscreen);
            }
        });
    }

    pub(super) fn ribbon_ai(&self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        group(ui, "Scrittura", |ui| {
            if big_ex(
                ui,
                if self.ocr_busy { ic::HOURGLASS } else { ic::TEXT_AA },
                if self.ocr_busy { "Trascrivo…" } else { "Trascrivi" },
                "Scrittura a mano → testo in Markdown, fedele al foglio. Legge un agente AI che vede le immagini: quale, in Impostazioni AI → «Lettura dei fogli (OCR)» (offline: Apple Vision, in File → Opzioni)",
                !self.ocr_busy,
                false,
                false,
            )
            .clicked()
            {
                actions.push(Action::Ocr);
            }
            if big(ui, ic::BOOK_OPEN, "Appunti", "Trascrizioni e chat salvate (.md), impaginate; da lì anche «Nuovo»").clicked() {
                actions.push(Action::Appunti);
            }
        });
        group(ui, "Agente", |ui| {
            if big(ui, ic::CHECK_CIRCLE, "Controlla\nil foglio", "L'AI rilegge quello che hai scritto e segnala errori (ortografia, dati, calcoli, ragionamento) e punti poco chiari, con la correzione").clicked() {
                actions.push(Action::ChatCheck);
            }
            let on = self.settings.live_check;
            let r = big_ex(
                ui,
                ic::SEAL_CHECK,
                "Controllo\nmentre scrivi",
                "Quando smetti di scrivere per qualche secondo l'AI guarda le righe nuove: se trova un errore lo segna sul foglio e lo spiega in una riga, altrimenti tace",
                self.docui.doc.is_none(),
                on,
                true,
            );
            egui::Popup::menu(&r).show(|ui| {
                ui.set_min_width(240.0);
                if menu_item(ui, true, check_icon(on), "Controllo acceso", "") {
                    actions.push(Action::LiveCheck(!on));
                }
                ui.separator();
                let cur = self.settings.live_check_pause;
                for (secs, label) in [(3.0f32, "Dopo 3 secondi di pausa"), (4.0, "Dopo 4 secondi di pausa"), (6.0, "Dopo 6 secondi di pausa"), (10.0, "Dopo 10 secondi di pausa")] {
                    let icon = if (cur - secs).abs() < 0.05 { ic::RADIO_BUTTON } else { ic::CIRCLE };
                    if menu_item(ui, true, icon, label, "") {
                        actions.push(Action::LiveCheckPause(secs));
                    }
                }
            });
            let doc_open = self.docui.doc.is_some();
            if big(
                ui,
                ic::CHAT_TEXT,
                if doc_open { "Discuti il\ndocumento" } else { "Discuti\nil foglio" },
                if doc_open { "Allega alla chat il documento: il testo di tutte le pagine e l'immagine della pagina aperta" } else { "Allega il foglio alla chat" },
            )
            .clicked()
            {
                actions.push(Action::ChatBoard);
            }
        });
        group(ui, "Impostazioni", |ui| {
            if big(ui, ic::SPARKLE, "Impostazioni\nAI", "Agenti, indirizzi, chiavi, contesto massimo, compattazione").clicked() {
                actions.push(Action::AiSettings);
            }
        });
    }
}
