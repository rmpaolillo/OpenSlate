# Librerie modificate

Copie di librerie altrui con piccole modifiche, usate al posto degli originali tramite
`[patch]` in `Cargo.toml`. Quando si aggiorna l'originale: ricopiare la cartella dalla nuova
versione e rifare la modifica (è segnata nel codice con «OpenSlate:»).

## typst-render, typst-svg (Typst 0.15.1, Apache-2.0)

Da crates.io, con una sola modifica in `src/image.rs`: le impostazioni di hayro finiscono con
`..Default::default()`.

Motivo: OpenSlate usa hayro 0.7 nella versione della suite Craft
(https://github.com/storytold/craft-libs, vedi `[patch.crates-io]`), che ha qualche campo in
più; Typst la compila così. OpenSlate usa Typst solo per disegnare le formule dei Markdown
(`src/mathtex.rs`), non le immagini PDF.

## hayro-svg (0.7.0, MIT OR Apache-2.0)

Da crates.io, con una sola modifica in `src/glyph.rs`: la versione di hayro della suite Craft
ha in più il modo `GlyphDrawMode::Clip`, che non disegna niente e qui si tratta come `Invisible`.
Serve solo a far compilare `typst-svg` (vedi sopra): OpenSlate non esporta immagini PDF con Typst.

## pdfcraft-render (PdfCraft 7d78f3e, 0.17.0, MIT OR Apache-2.0)

`crates/render` di https://github.com/storytold/pdfcraft, con le dipendenze del workspace scritte
per esteso e una modifica in `src/raster.rs`: tolto il caso `InterpreterWarning::ColorFallback`.

Motivo: nella stessa app può entrare un solo `hayro-interpret`. Lo vuole quello di craft-libs
VectorCraft (per l'API degli inchiostri), che però non ha questo avviso di PdfCraft. Effetto:
OpenSlate non riceve l'avviso «colore sostituito» su una pagina PDF; il disegno è quello di
craft-libs. Da togliere quando PdfCraft e craft-libs useranno lo stesso hayro.

## wordcraft-layout (WordCraft 5cd53f1, 0.14.0, MIT OR Apache-2.0)

`crates/layout` di https://github.com/storytold/wordcraft, con le dipendenze del workspace
scritte per esteso in `Cargo.toml` e due modifiche (`src/para/lines.rs`, `src/display/lines.rs`): un'immagine
nel testo segue la «posizione» del carattere (alzata o abbassata), come in Word. Così le formule
dei Markdown, che sono immagini disegnate da Typst, stanno sulla riga di base del testo anche
quando scendono sotto (f(x), y, le frazioni…). Senza la modifica WordCraft appoggia sempre
l'immagine sulla riga e la formula sale.

Le immagini senza posizione (tutte quelle dei documenti Word normali) restano come prima. La
modifica è adatta a essere proposta a WordCraft.
