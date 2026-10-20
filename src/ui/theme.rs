//! Colori, caratteri e stile dell'interfaccia (chiaro e scuro).
//!
//! Ripreso da WordCraft (`crates/ui-egui/src/theme.rs`,
//! https://github.com/storytold/wordcraft, MIT OR Apache-2.0,
//! © ArtCraft Team e contributori di WordCraft): stessi colori («token»),
//! stesso carattere Inter (SIL OFL 1.1, `assets/fonts/OFL-Inter.txt`).

use eframe::egui;
use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, Visuals};
use std::sync::Arc;

/// Colore dell'app (e la sua versione più scura per il testo sul chiaro).
pub const APP_COLOR: Color32 = Color32::from_rgb(0x3B, 0x5B, 0xDB);
pub const APP_INK: Color32 = Color32::from_rgb(0x2B, 0x47, 0xB5);

/// Tema dell'interfaccia: chiaro, scuro o quello del sistema.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Appearance {
    System,
    Light,
    Dark,
}

impl Appearance {
    pub const ALL: [Appearance; 3] = [Appearance::System, Appearance::Light, Appearance::Dark];

    /// Valore in `settings.conf`.
    pub fn code(self) -> &'static str {
        match self {
            Appearance::System => "sistema",
            Appearance::Light => "chiaro",
            Appearance::Dark => "scuro",
        }
    }

    pub fn from_code(s: &str) -> Appearance {
        Appearance::ALL.into_iter().find(|a| a.code() == s.trim()).unwrap_or(Appearance::System)
    }

    pub fn label(self) -> &'static str {
        match self {
            Appearance::System => "Come il sistema",
            Appearance::Light => "Chiaro",
            Appearance::Dark => "Scuro",
        }
    }

    fn preference(self) -> egui::ThemePreference {
        match self {
            Appearance::System => egui::ThemePreference::System,
            Appearance::Light => egui::ThemePreference::Light,
            Appearance::Dark => egui::ThemePreference::Dark,
        }
    }
}

/// Tutti i colori dell'interfaccia: i widget non scrivono colori a mano.
#[derive(Clone, Copy, Debug)]
pub struct Tokens {
    pub dark: bool,
    pub title_bar: Color32,
    pub ribbon: Color32,
    pub tab_strip: Color32,
    pub canvas: Color32,
    pub panel: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub text_disabled: Color32,
    pub icon: Color32,
    pub hover: Color32,
    pub pressed: Color32,
    pub checked: Color32,
    pub accent: Color32,
    pub accent_text: Color32,
    /// Testo e icone sopra `accent`.
    pub on_accent: Color32,
    pub input: Color32,
    pub group_label: Color32,
    pub ruler: Color32,
    pub ruler_margin: Color32,
    pub ruler_tick: Color32,
    pub status_bar: Color32,
    pub page_shadow: Color32,
    pub menu: Color32,
    pub red: Color32,
    pub green: Color32,
    pub orange: Color32,
}

impl Tokens {
    pub fn light() -> Self {
        Tokens {
            dark: false,
            title_bar: Color32::from_rgb(0xF0, 0xF0, 0xF0),
            ribbon: Color32::from_rgb(0xF9, 0xF9, 0xF9),
            tab_strip: Color32::from_rgb(0xF0, 0xF0, 0xF0),
            canvas: Color32::from_rgb(0xE6, 0xE6, 0xE6),
            panel: Color32::from_rgb(0xFA, 0xFA, 0xFA),
            border: Color32::from_rgb(0xE1, 0xE1, 0xE1),
            border_strong: Color32::from_rgb(0xC8, 0xC8, 0xC8),
            text: Color32::from_rgb(0x24, 0x24, 0x24),
            text_dim: Color32::from_rgb(0x61, 0x61, 0x61),
            text_disabled: Color32::from_rgb(0xA8, 0xA8, 0xA8),
            icon: Color32::from_rgb(0x42, 0x42, 0x42),
            hover: Color32::from_rgb(0xE6, 0xE6, 0xE6),
            pressed: Color32::from_rgb(0xD6, 0xD6, 0xD6),
            checked: Color32::from_rgb(0xDD, 0xE3, 0xF8),
            accent: APP_COLOR,
            accent_text: APP_INK,
            on_accent: Color32::WHITE,
            input: Color32::WHITE,
            group_label: Color32::from_rgb(0x70, 0x70, 0x70),
            ruler: Color32::WHITE,
            ruler_margin: Color32::from_rgb(0xD4, 0xD4, 0xD4),
            ruler_tick: Color32::from_rgb(0x70, 0x70, 0x70),
            status_bar: Color32::from_rgb(0xF0, 0xF0, 0xF0),
            page_shadow: Color32::from_rgba_unmultiplied(0, 0, 0, 0x22),
            menu: Color32::WHITE,
            red: Color32::from_rgb(0xD1, 0x3B, 0x3B),
            green: Color32::from_rgb(0x2E, 0x8B, 0x57),
            orange: Color32::from_rgb(0xE0, 0x7B, 0x1F),
        }
    }

    pub fn dark() -> Self {
        Tokens {
            dark: true,
            title_bar: Color32::from_rgb(0x1F, 0x1F, 0x1F),
            ribbon: Color32::from_rgb(0x29, 0x29, 0x29),
            tab_strip: Color32::from_rgb(0x1F, 0x1F, 0x1F),
            canvas: Color32::from_rgb(0x14, 0x14, 0x14),
            panel: Color32::from_rgb(0x26, 0x26, 0x26),
            border: Color32::from_rgb(0x3A, 0x3A, 0x3A),
            border_strong: Color32::from_rgb(0x55, 0x55, 0x55),
            text: Color32::from_rgb(0xEE, 0xEE, 0xEE),
            text_dim: Color32::from_rgb(0xC8, 0xC8, 0xC8),
            text_disabled: Color32::from_rgb(0x6A, 0x6A, 0x6A),
            icon: Color32::from_rgb(0xDD, 0xDD, 0xDD),
            hover: Color32::from_rgb(0x3D, 0x3D, 0x3D),
            pressed: Color32::from_rgb(0x4A, 0x4A, 0x4A),
            checked: Color32::from_rgb(0x2E, 0x3A, 0x66),
            accent: Color32::from_rgb(0x7A, 0x93, 0xF0),
            accent_text: Color32::from_rgb(0x9C, 0xB0, 0xF5),
            on_accent: Color32::from_rgb(0x11, 0x16, 0x2E),
            input: Color32::from_rgb(0x1E, 0x1E, 0x1E),
            group_label: Color32::from_rgb(0xA0, 0xA0, 0xA0),
            ruler: Color32::from_rgb(0x45, 0x45, 0x45),
            ruler_margin: Color32::from_rgb(0x2E, 0x2E, 0x2E),
            ruler_tick: Color32::from_rgb(0xB4, 0xB4, 0xB4),
            status_bar: Color32::from_rgb(0x1F, 0x1F, 0x1F),
            page_shadow: Color32::from_rgba_unmultiplied(0, 0, 0, 0x60),
            menu: Color32::from_rgb(0x2B, 0x2B, 0x2B),
            red: Color32::from_rgb(0xEE, 0x6A, 0x6A),
            green: Color32::from_rgb(0x5B, 0xC0, 0x8A),
            orange: Color32::from_rgb(0xF0, 0xA0, 0x50),
        }
    }

    pub fn get(ctx: &egui::Context) -> Tokens {
        if ctx.global_style().visuals.dark_mode {
            Tokens::dark()
        } else {
            Tokens::light()
        }
    }
}

pub fn regular(size: f32) -> FontId {
    FontId::proportional(size)
}
pub fn medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("medium".into()))
}
pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("semibold".into()))
}
/// Icone (Phosphor) alla grandezza voluta.
pub fn icon(size: f32) -> FontId {
    FontId::proportional(size)
}

/// Nomi dei nostri caratteri.
const F_INTER: &str = "wsr-inter";
const F_MEDIUM: &str = "wsr-inter-medium";
const F_SEMIBOLD: &str = "wsr-inter-semibold";
const F_ICONS: &str = "wsr-phosphor";

/// Inter per il testo, Phosphor per le icone, poi i caratteri di egui
/// (simboli ed emoji).
pub fn fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    merge_ours(&mut fonts);
    // cinese, giapponese, coreano: Inter non ha quei segni, si prende un
    // carattere del sistema (in coda, dopo Inter e le icone)
    let lang = crate::i18n::lang();
    if lang.cjk() {
        match cjk_font(lang) {
            Some((bytes, index)) => {
                let mut data = FontData::from_owned(bytes);
                data.index = index;
                fonts.font_data.insert(F_CJK.into(), Arc::new(data));
                for fam in [FontFamily::Proportional, FontFamily::Name("medium".into()), FontFamily::Name("semibold".into())] {
                    let list = fonts.families.entry(fam).or_default();
                    let at = list.iter().position(|n| n == F_ICONS).map(|i| i + 1).unwrap_or(list.len());
                    list.insert(at, F_CJK.into());
                }
            }
            None => tracing::warn!("nessun carattere {} trovato nel sistema", lang.code()),
        }
    }
    fonts
}

const F_CJK: &str = "cjk";
const F_LANGNAMES: &str = "cjk-langnames";

/// Un carattere del sistema per cinese, giapponese o coreano (byte e indice
/// nella raccolta .ttc).
fn cjk_font(lang: crate::i18n::Lang) -> Option<(Vec<u8>, u32)> {
    use crate::i18n::Lang;
    // Noto Sans CJK (Linux): nella raccolta 0 = giapponese, 1 = coreano, 2 = cinese semplificato
    let noto = match lang {
        Lang::Ja => 0,
        Lang::Ko => 1,
        _ => 2,
    };
    let mut cands: Vec<(&str, u32)> = Vec::new();
    if cfg!(target_os = "macos") {
        match lang {
            Lang::Ja => cands.extend([("/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc", 0), ("/System/Library/Fonts/Hiragino Sans GB.ttc", 0)]),
            Lang::Ko => cands.push(("/System/Library/Fonts/AppleSDGothicNeo.ttc", 0)),
            _ => cands.extend([("/System/Library/Fonts/PingFang.ttc", 0), ("/System/Library/Fonts/Hiragino Sans GB.ttc", 0), ("/System/Library/Fonts/STHeiti Light.ttc", 0)]),
        }
        cands.extend([("/System/Library/Fonts/Supplemental/Arial Unicode.ttf", 0), ("/Library/Fonts/Arial Unicode.ttf", 0)]);
    } else if cfg!(target_os = "windows") {
        match lang {
            Lang::Ja => cands.extend([("C:\\Windows\\Fonts\\YuGothM.ttc", 0), ("C:\\Windows\\Fonts\\meiryo.ttc", 0), ("C:\\Windows\\Fonts\\msgothic.ttc", 0)]),
            Lang::Ko => cands.push(("C:\\Windows\\Fonts\\malgun.ttf", 0)),
            _ => cands.extend([("C:\\Windows\\Fonts\\msyh.ttc", 0), ("C:\\Windows\\Fonts\\simsun.ttc", 0)]),
        }
    } else {
        for dir in ["/usr/share/fonts/opentype/noto", "/usr/share/fonts/noto-cjk", "/usr/share/fonts/google-noto-cjk", "/usr/share/fonts/opentype/noto-cjk"] {
            cands.push((Box::leak(format!("{dir}/NotoSansCJK-Regular.ttc").into_boxed_str()), noto));
        }
        cands.extend([("/usr/share/fonts/truetype/wqy/wqy-microhei.ttc", 0), ("/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf", 0)]);
    }
    cands.into_iter().find_map(|(p, i)| std::fs::read(p).ok().map(|b| (b, i)))
}

/// Aggiunge i nostri caratteri a quelli di egui, in testa alle famiglie: prima
/// Inter (senza i segni dell'area privata dell'Unicode, tolti con pyftsubset),
/// poi le icone Phosphor, poi tutto il resto.
fn merge_ours(fonts: &mut FontDefinitions) {
    let add = |fonts: &mut FontDefinitions, name: &str, bytes: &'static [u8]| {
        fonts.font_data.insert(name.into(), Arc::new(FontData::from_static(bytes)));
    };
    add(fonts, F_INTER, include_bytes!("../../assets/fonts/Inter-Regular.ttf"));
    add(fonts, F_MEDIUM, include_bytes!("../../assets/fonts/Inter-Medium.ttf"));
    add(fonts, F_SEMIBOLD, include_bytes!("../../assets/fonts/Inter-SemiBold.ttf"));
    add(fonts, F_ICONS, egui_phosphor::Variant::Regular.font_bytes());
    // i nomi delle lingue nel menu (中文 日本語 한국어) anche senza i caratteri
    // CJK del sistema: un ritaglio minuscolo di Noto Sans CJK (SIL OFL 1.1)
    add(fonts, F_LANGNAMES, include_bytes!("../../assets/fonts/NotoSansCJK-langnames.otf"));
    let first = |list: &mut Vec<String>, head: &[&str]| {
        list.retain(|n| !head.contains(&n.as_str()));
        for (i, h) in head.iter().enumerate() {
            list.insert(i, h.to_string());
        }
    };
    let base = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    first(fonts.families.entry(FontFamily::Proportional).or_default(), &[F_INTER, F_ICONS]);
    for (fam, head) in [("medium", F_MEDIUM), ("semibold", F_SEMIBOLD)] {
        let list = fonts.families.entry(FontFamily::Name(fam.into())).or_insert_with(|| base.clone());
        first(list, &[head, F_INTER, F_ICONS]);
    }
    for fam in [FontFamily::Proportional, FontFamily::Name("medium".into()), FontFamily::Name("semibold".into())] {
        fonts.families.entry(fam).or_default().push(F_LANGNAMES.into());
    }
}

/// Mette in egui i due temi (chiaro e scuro) e sceglie quale usare.
pub fn apply(ctx: &egui::Context, appearance: Appearance) {
    ctx.set_visuals_of(egui::Theme::Light, visuals(&Tokens::light()));
    ctx.set_visuals_of(egui::Theme::Dark, visuals(&Tokens::dark()));
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = egui::vec2(6.0, 4.0);
        s.spacing.button_padding = egui::vec2(8.0, 3.0);
        s.spacing.interact_size = egui::vec2(24.0, 22.0);
        s.spacing.menu_margin = egui::Margin::same(6);
        s.text_styles.insert(egui::TextStyle::Body, regular(12.5));
        s.text_styles.insert(egui::TextStyle::Button, regular(12.5));
        s.text_styles.insert(egui::TextStyle::Small, regular(10.5));
        s.text_styles.insert(egui::TextStyle::Heading, semibold(17.0));
        s.text_styles.insert(egui::TextStyle::Monospace, FontId::monospace(12.0));
        s.interaction.tooltip_delay = 0.45;
    });
    ctx.options_mut(|o| {
        o.fallback_theme = egui::Theme::Light;
        o.theme_preference = appearance.preference();
    });
}

fn visuals(t: &Tokens) -> Visuals {
    let mut v = if t.dark { Visuals::dark() } else { Visuals::light() };
    v.panel_fill = t.ribbon;
    v.window_fill = t.menu;
    v.extreme_bg_color = t.input;
    v.faint_bg_color = t.panel;
    v.window_stroke = Stroke::new(1.0, t.border_strong);
    v.window_corner_radius = CornerRadius::same(8);
    v.menu_corner_radius = CornerRadius::same(6);
    v.selection.bg_fill = if t.dark { Color32::from_rgba_unmultiplied(0x7A, 0x93, 0xF0, 0x55) } else { Color32::from_rgba_unmultiplied(0x3B, 0x5B, 0xDB, 0x48) };
    v.selection.stroke = Stroke::new(1.0, t.accent);
    v.hyperlink_color = t.accent_text;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, t.border);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, t.text);
    v.widgets.inactive.bg_fill = t.input;
    v.widgets.inactive.weak_bg_fill = t.input;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, t.border_strong);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, t.text);
    v.widgets.inactive.corner_radius = CornerRadius::same(4);
    v.widgets.hovered.bg_fill = t.hover;
    v.widgets.hovered.weak_bg_fill = t.hover;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, t.border_strong);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, t.text);
    v.widgets.hovered.corner_radius = CornerRadius::same(4);
    v.widgets.active.bg_fill = t.pressed;
    v.widgets.active.weak_bg_fill = t.pressed;
    v.widgets.active.fg_stroke = Stroke::new(1.0, t.text);
    v.widgets.active.corner_radius = CornerRadius::same(4);
    v.widgets.open.bg_fill = t.pressed;
    v.widgets.open.weak_bg_fill = t.pressed;
    v.popup_shadow = egui::Shadow { offset: [0, 4], blur: 16, spread: 0, color: Color32::from_black_alpha(if t.dark { 120 } else { 40 }) };
    v.window_shadow = egui::Shadow { offset: [0, 8], blur: 28, spread: 0, color: Color32::from_black_alpha(if t.dark { 140 } else { 50 }) };
    v
}
