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

## pdfcraft-render (PdfCraft 37fde37, 0.21.0, MIT OR Apache-2.0)

`crates/render` di https://github.com/storytold/pdfcraft, con le dipendenze del workspace scritte
per esteso e qualche modifica dove PdfCraft usa cose che il hayro di craft-libs non ha (segnate
con «OpenSlate:»):
- `src/raster.rs`: tolti i casi `InterpreterWarning::ColorFallback`, `MalformedIndexedImage` e
  `Type3WidthsMissing`, il limite `jbig2_work` e `view_auto_state` (gli stati automatici `/View`
  dei livelli) passati a hayro;
- `src/inspect/core.rs`: niente `rebuilt_unlinked_update` (l'avviso sui file con un aggiornamento
  senza `/Prev`);
- `src/text/device.rs`: niente `vertical_glyph_width`, si usa la larghezza normale del glifo.

Motivo: nella stessa app può entrare un solo `hayro-interpret`: quello di craft-libs (1b34eba, lo
stesso di PhotoCraft e WordCraft; VectorCraft lo vuole per l'API degli inchiostri), mentre PdfCraft
ne ha uno suo più avanti. Effetto: OpenSlate non riceve gli avvisi «colore sostituito», «immagine indicizzata
malformata», «larghezze Type 3 mancanti» e «aggiornamento senza /Prev» su un PDF, non ha il tetto di lavoro per pagina
sulle immagini JBIG2, non applica ai livelli gli stati automatici di visualizzazione (`/AS` con
`/Event /View`) e il testo estratto dai caratteri verticali (giapponese, cinese) può avere
riquadri meno precisi; il disegno è quello di craft-libs. Da togliere quando PdfCraft e
craft-libs useranno lo stesso hayro.

## craft-i18n (dati di lingua, 1e51542, Apache-2.0 / MIT e licenze dei singoli file)

`language-data/` di https://github.com/storytold/craft-i18n, senza modifiche: solo i file che
craft-proof (il controllo ortografico e la sillabazione di WordCraft) include durante la
compilazione. `CRAFT_I18N_DIR` in `.cargo/config.toml` punta qui.

Motivo: fino a WordCraft 0.14 questi dati erano dentro WordCraft; dalla 0.15 craft-proof li prende
da craft-i18n, e senza diventa solo inglese e senza sillabazione (le righe dei documenti Word
andrebbero a capo in un altro punto). Autori e licenze di ogni file: `ATTRIBUTION.md` (Moby e
pattern inglesi di pubblico dominio, SCOWL, pattern TeX MIT o BSD-3-Clause). Quando si aggiorna
WordCraft: ricopiare i file alla revisione di `craft-i18n.rev` del suo repository.
