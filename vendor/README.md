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

## pdfcraft-render (PdfCraft 3936f93, 0.18.0, MIT OR Apache-2.0)

`crates/render` di https://github.com/storytold/pdfcraft, con le dipendenze del workspace scritte
per esteso e due modifiche in `src/raster.rs`: tolto il caso `InterpreterWarning::ColorFallback` e
il limite `jbig2_work` passato a hayro (impostazione che il hayro di craft-libs non ha).

Motivo: nella stessa app può entrare un solo `hayro-interpret`. Lo vuole quello di craft-libs
VectorCraft (per l'API degli inchiostri), che però non ha questo avviso di PdfCraft. Effetto:
OpenSlate non riceve l'avviso «colore sostituito» su una pagina PDF e non ha il tetto di lavoro per
pagina sulle immagini JBIG2 (aggiunto in PdfCraft 0.18); il disegno è quello di craft-libs. Da togliere quando PdfCraft e craft-libs useranno lo stesso hayro.

## craft-i18n (dati di lingua, 059cbf9, Apache-2.0 / MIT e licenze dei singoli file)

`language-data/` di https://github.com/storytold/craft-i18n, senza modifiche: solo i file che
craft-proof (il controllo ortografico e la sillabazione di WordCraft) include durante la
compilazione. `CRAFT_I18N_DIR` in `.cargo/config.toml` punta qui.

Motivo: fino a WordCraft 0.14 questi dati erano dentro WordCraft; dalla 0.15 craft-proof li prende
da craft-i18n, e senza diventa solo inglese e senza sillabazione (le righe dei documenti Word
andrebbero a capo in un altro punto). Autori e licenze di ogni file: `ATTRIBUTION.md` (Moby e
pattern inglesi di pubblico dominio, SCOWL, pattern TeX MIT o BSD-3-Clause). Quando si aggiorna
WordCraft: ricopiare i file alla revisione di `craft-i18n.rev` del suo repository.

## wordcraft-layout (WordCraft b47079f, 0.15.0, MIT OR Apache-2.0)

`crates/layout` di https://github.com/storytold/wordcraft, con le dipendenze del workspace
scritte per esteso in `Cargo.toml` e due modifiche (`src/para/lines.rs`, `src/display/lines.rs`): un'immagine
nel testo segue la «posizione» del carattere (alzata o abbassata), come in Word. Così le formule
dei Markdown, che sono immagini disegnate da Typst, stanno sulla riga di base del testo anche
quando scendono sotto (f(x), y, le frazioni…). Senza la modifica WordCraft appoggia sempre
l'immagine sulla riga e la formula sale.

Le immagini senza posizione (tutte quelle dei documenti Word normali) restano come prima. La
modifica è adatta a essere proposta a WordCraft.
