//! Le finestre del sistema (Salva con nome, Apri, scegli una cartella, sì/no).
//!
//! Su macOS e Windows le fa rfd. Su Linux no: rfd apre GTK in un thread suo,
//! ma la chat e gli appunti (WebKitGTK) usano GTK nel thread della finestra, e
//! GTK vuole un thread solo; con la chat già aperta la finestra di rfd non
//! arriva mai e l'app resta bloccata. Su Linux quindi le finestre si fanno con
//! GTK, nello stesso thread della finestra.

use crate::i18n;
use std::path::{Path, PathBuf};

/// «Salva con nome»: `name` è il nome proposto (con l'estensione), `kind` e
/// `ext` il filtro dei file. `None` = annullato.
pub fn save_file(title: &str, dir: &Path, name: &str, kind: &str, ext: &str) -> Option<PathBuf> {
    imp::save_file(title, dir, name, kind, ext)
}

/// Sceglie una cartella. `None` = annullato.
pub fn pick_folder(title: &str, dir: &Path) -> Option<PathBuf> {
    imp::pick_folder(title, dir)
}

/// Apre un file fra quelli con le estensioni `exts`. `None` = annullato.
pub fn pick_file(title: &str, kind: &str, exts: &[&str]) -> Option<PathBuf> {
    imp::pick_file(title, kind, exts)
}

/// Una domanda sì/no. `true` = sì.
pub fn confirm(title: &str, text: &str) -> bool {
    imp::confirm(title, text)
}

#[cfg(not(target_os = "linux"))]
mod imp {
    use super::*;

    pub fn save_file(title: &str, dir: &Path, name: &str, kind: &str, ext: &str) -> Option<PathBuf> {
        rfd::FileDialog::new().set_title(title).set_directory(dir).set_file_name(name).add_filter(kind, &[ext]).save_file()
    }

    pub fn pick_folder(title: &str, dir: &Path) -> Option<PathBuf> {
        rfd::FileDialog::new().set_title(title).set_directory(dir).pick_folder()
    }

    pub fn pick_file(title: &str, kind: &str, exts: &[&str]) -> Option<PathBuf> {
        rfd::FileDialog::new().set_title(title).add_filter(kind, exts).pick_file()
    }

    pub fn confirm(title: &str, text: &str) -> bool {
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Info)
            .set_title(title)
            .set_description(text)
            .set_buttons(rfd::MessageButtons::YesNo)
            .show()
            == rfd::MessageDialogResult::Yes
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::*;
    use gtk::prelude::*;

    /// GTK nel thread della finestra (se la chat l'ha già acceso, non fa niente).
    fn ready() -> bool {
        if gtk::is_initialized_main_thread() {
            return true;
        }
        match gtk::init() {
            Ok(()) => true,
            Err(e) => {
                tracing::warn!("finestre del sistema: GTK non parte: {e}");
                false
            }
        }
    }

    /// Fa sparire la finestra chiusa (la finestra dell'app non fa girare GTK
    /// finché non torna al suo ciclo).
    fn flush() {
        while gtk::events_pending() {
            gtk::main_iteration_do(false);
        }
    }

    fn chooser(title: &str, action: gtk::FileChooserAction, ok: &str) -> gtk::FileChooserDialog {
        let d = gtk::FileChooserDialog::with_buttons::<gtk::Window>(
            Some(title),
            None,
            action,
            &[(&i18n::tc("cancel", "Annulla"), gtk::ResponseType::Cancel), (ok, gtk::ResponseType::Accept)],
        );
        d.set_default_response(gtk::ResponseType::Accept);
        d.set_keep_above(true);
        d.set_position(gtk::WindowPosition::Center);
        d
    }

    fn filter(kind: &str, exts: &[&str]) -> gtk::FileFilter {
        let f = gtk::FileFilter::new();
        f.set_name(Some(kind));
        for e in exts {
            f.add_pattern(&format!("*.{}", e.to_lowercase()));
            f.add_pattern(&format!("*.{}", e.to_uppercase()));
        }
        f
    }

    fn run(d: gtk::FileChooserDialog) -> Option<PathBuf> {
        let r = d.run();
        let p = d.filename();
        // SAFETY: la finestra è nostra e non si usa più dopo
        unsafe { d.destroy() };
        flush();
        if r == gtk::ResponseType::Accept {
            p
        } else {
            None
        }
    }

    pub fn save_file(title: &str, dir: &Path, name: &str, kind: &str, ext: &str) -> Option<PathBuf> {
        if !ready() {
            return None;
        }
        let d = chooser(title, gtk::FileChooserAction::Save, i18n::t("Salva"));
        d.set_do_overwrite_confirmation(true);
        d.set_current_folder(dir);
        d.set_current_name(name);
        d.add_filter(filter(kind, &[ext]));
        run(d)
    }

    pub fn pick_folder(title: &str, dir: &Path) -> Option<PathBuf> {
        if !ready() {
            return None;
        }
        let d = chooser(title, gtk::FileChooserAction::SelectFolder, i18n::t("Apri"));
        d.set_current_folder(dir);
        run(d)
    }

    pub fn pick_file(title: &str, kind: &str, exts: &[&str]) -> Option<PathBuf> {
        if !ready() {
            return None;
        }
        let d = chooser(title, gtk::FileChooserAction::Open, i18n::t("Apri"));
        d.add_filter(filter(kind, exts));
        run(d)
    }

    pub fn confirm(title: &str, text: &str) -> bool {
        if !ready() {
            return false;
        }
        let d = gtk::MessageDialog::new::<gtk::Window>(None, gtk::DialogFlags::MODAL, gtk::MessageType::Info, gtk::ButtonsType::YesNo, text);
        d.set_title(title);
        d.set_keep_above(true);
        d.set_position(gtk::WindowPosition::Center);
        let r = d.run();
        // SAFETY: la finestra è nostra e non si usa più dopo
        unsafe { d.destroy() };
        flush();
        r == gtk::ResponseType::Yes
    }
}
