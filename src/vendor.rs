//! KaTeX 0.16.11, marked 12.0.2 (MIT) e DOMPurify 3.4.16 (Apache-2.0 / MPL-2.0) dentro il programma (vedi
//! src/web/vendor/LICENSE-*): le formule si vedono anche senza internet.

/// (percorso dopo /vendor/, tipo, contenuto)
pub static FILES: &[(&str, &str, &[u8])] = &[
    ("katex.min.js", "application/javascript", include_bytes!("web/vendor/katex.min.js")),
    ("katex.min.css", "text/css", include_bytes!("web/vendor/katex.min.css")),
    ("marked.min.js", "application/javascript", include_bytes!("web/vendor/marked.min.js")),
    // DOMPurify 3.4.16 (Apache-2.0 o MPL-2.0, src/web/vendor/LICENSE-dompurify): pulisce l'HTML delle risposte
    ("purify.min.js", "application/javascript", include_bytes!("web/vendor/purify.min.js")),
    // Inter (SIL OFL 1.1, assets/fonts/OFL-Inter.txt): lo stesso carattere dell'interfaccia
    ("inter/Inter-Regular.ttf", "font/ttf", crate::fonts::INTER_REGULAR),
    ("inter/Inter-Medium.ttf", "font/ttf", crate::fonts::INTER_MEDIUM),
    ("inter/Inter-SemiBold.ttf", "font/ttf", crate::fonts::INTER_SEMIBOLD),
    ("hand/OpenSlateCorsivo-Regular.ttf", "font/ttf", include_bytes!("../assets/fonts/OpenSlateCorsivo-Regular.ttf")),
    ("hand/OpenSlateCorsivo-Bold.ttf", "font/ttf", include_bytes!("../assets/fonts/OpenSlateCorsivo-Bold.ttf")),
    ("hand/OpenSlateStampatello-Regular.ttf", "font/ttf", include_bytes!("../assets/fonts/OpenSlateStampatello-Regular.ttf")),
    ("hand/OpenSlateStampatello-Bold.ttf", "font/ttf", include_bytes!("../assets/fonts/OpenSlateStampatello-Bold.ttf")),
    ("fonts/KaTeX_AMS-Regular.woff2", "font/woff2", include_bytes!("web/vendor/fonts/KaTeX_AMS-Regular.woff2")),
    ("fonts/KaTeX_Caligraphic-Bold.woff2", "font/woff2", include_bytes!("web/vendor/fonts/KaTeX_Caligraphic-Bold.woff2")),
    ("fonts/KaTeX_Caligraphic-Regular.woff2", "font/woff2", include_bytes!("web/vendor/fonts/KaTeX_Caligraphic-Regular.woff2")),
    ("fonts/KaTeX_Fraktur-Bold.woff2", "font/woff2", include_bytes!("web/vendor/fonts/KaTeX_Fraktur-Bold.woff2")),
    ("fonts/KaTeX_Fraktur-Regular.woff2", "font/woff2", include_bytes!("web/vendor/fonts/KaTeX_Fraktur-Regular.woff2")),
    ("fonts/KaTeX_Main-Bold.woff2", "font/woff2", include_bytes!("web/vendor/fonts/KaTeX_Main-Bold.woff2")),
    ("fonts/KaTeX_Main-BoldItalic.woff2", "font/woff2", include_bytes!("web/vendor/fonts/KaTeX_Main-BoldItalic.woff2")),
    ("fonts/KaTeX_Main-Italic.woff2", "font/woff2", include_bytes!("web/vendor/fonts/KaTeX_Main-Italic.woff2")),
    ("fonts/KaTeX_Main-Regular.woff2", "font/woff2", include_bytes!("web/vendor/fonts/KaTeX_Main-Regular.woff2")),
    ("fonts/KaTeX_Math-BoldItalic.woff2", "font/woff2", include_bytes!("web/vendor/fonts/KaTeX_Math-BoldItalic.woff2")),
    ("fonts/KaTeX_Math-Italic.woff2", "font/woff2", include_bytes!("web/vendor/fonts/KaTeX_Math-Italic.woff2")),
    ("fonts/KaTeX_SansSerif-Bold.woff2", "font/woff2", include_bytes!("web/vendor/fonts/KaTeX_SansSerif-Bold.woff2")),
    ("fonts/KaTeX_SansSerif-Italic.woff2", "font/woff2", include_bytes!("web/vendor/fonts/KaTeX_SansSerif-Italic.woff2")),
    ("fonts/KaTeX_SansSerif-Regular.woff2", "font/woff2", include_bytes!("web/vendor/fonts/KaTeX_SansSerif-Regular.woff2")),
    ("fonts/KaTeX_Script-Regular.woff2", "font/woff2", include_bytes!("web/vendor/fonts/KaTeX_Script-Regular.woff2")),
    ("fonts/KaTeX_Size1-Regular.woff2", "font/woff2", include_bytes!("web/vendor/fonts/KaTeX_Size1-Regular.woff2")),
    ("fonts/KaTeX_Size2-Regular.woff2", "font/woff2", include_bytes!("web/vendor/fonts/KaTeX_Size2-Regular.woff2")),
    ("fonts/KaTeX_Size3-Regular.woff2", "font/woff2", include_bytes!("web/vendor/fonts/KaTeX_Size3-Regular.woff2")),
    ("fonts/KaTeX_Size4-Regular.woff2", "font/woff2", include_bytes!("web/vendor/fonts/KaTeX_Size4-Regular.woff2")),
    ("fonts/KaTeX_Typewriter-Regular.woff2", "font/woff2", include_bytes!("web/vendor/fonts/KaTeX_Typewriter-Regular.woff2")),
];

pub fn get(path: &str) -> Option<(&'static str, &'static [u8])> {
    // Source Serif 4 (SIL OFL 1.1, assets/fonts/OFL-SourceSerif4.txt): il testo
    // degli appunti, come un documento. È la copia di WordCraft (vedi src/fonts.rs).
    if let Some(ps) = path.strip_prefix("serif/").and_then(|f| f.strip_suffix(".ttf")) {
        return crate::fonts::bundled(ps).map(|b| ("font/ttf", b));
    }
    FILES.iter().find(|(p, _, _)| *p == path).map(|(_, t, b)| (*t, *b))
}
