# OpenSlate per Android (prova)

App Android per il Bamboo Slate: tratti live con zoom (due dita; doppio tocco = pagina intera),
penna, evidenziatore, gomma (con la penna o strofinando col dito), colori e spessori della
desktop, annulla/ripeti, i 16 fogli della desktop (righe, quadretti, millimetrata, Cornell…),
scaricamento delle pagine scritte lontano dal telefono, libreria per giorno (rinomina, cestino,
apri sulla lavagna), condivisione in PDF o PNG, chat AI col foglio allegato e trascrizione della
pagina in nota Markdown, il tutor che legge il foglio e ci scrive (sottolinea, cerchia, ✓/✗,
testo a mano: `lavagna_leggi`/`lavagna_annota` della desktop), documenti da firmare col pennino
(PDF, Word, foto, SVG: la copia firmata resta nel suo formato, come sulla desktop) (stessi prompt e stessa catena di servizi della desktop: Claude, Gemini,
DeepSeek, OpenAI, LM Studio/Ollama sul Mac di casa; chiavi cifrate con l'Android Keystore). Le pagine sono nello stesso formato JSON della
desktop (`library/<id>.json`).

- `../mobile/core/`: il cuore Rust, condiviso con l'app iOS (`../ios`). Include **gli stessi file della desktop** (`../src/protocol.rs`,
  `live.rs`, `offline.rs`, `rigatura.rs`, `stroke.rs`) e aggiunge `board.rs` (la parte di
  modifica di `scene.rs`: strumenti, gomma, annulla/ripeti), `paper.rs`, `session.rs`, le sequenze di `device.rs`
  (registrazione, avvio del live, scaricamento) come macchina a stati senza I/O, e `pages.rs`,
  l'archivio nel formato di `library.rs`, e `ai.rs`, la catena di servizi AI di `src/ai.rs`
  con gli strumenti, e `tutor.rs`, che usa `boardread.rs`, `boardmarks.rs`, `handwrite.rs`,
  `pagegeom.rs` e `pagerender.rs` della desktop, e `docs.rs`, i documenti firmati con
  `pdfdoc.rs`, `wordsign.rs`, `rasign.rs`, `vecsign.rs` della desktop (WordCraft, PhotoCraft,
  VectorCraft; `mdcraft` è sostituito da un modulo vuoto: i Markdown non si aprono).
  `cargo test` qui dentro.
- `app/`: Kotlin + Jetpack Compose. Il Bluetooth lo fa `SlateLink.kt` (`BluetoothGatt`);
  i byte vanno e tornano dal cuore Rust tramite i binding UniFFI.
- `build-rust.sh`: compila il cuore con `cargo ndk` e genera i binding; Gradle lo lancia da sé.

## Compilare

Servono Android Studio (SDK, NDK), Rust con i target Android e cargo-ndk:

    rustup target add aarch64-linux-android x86_64-linux-android
    cargo install cargo-ndk

Poi apri la cartella `android/` in Android Studio e premi Run, oppure:

    ./gradlew assembleDebug        # app/build/outputs/apk/debug/app-debug.apk (anche per l'emulatore)
    ./gradlew assembleRelease      # app/build/outputs/apk/release/app-release.apk (solo telefoni, ~40 MB)

## Provare con il pad

1. Premi brevemente il pulsante dello Slate, poi «Collega».
2. La prima volta: tieni premuto il pulsante ~6 s finché il LED lampeggia blu, «Collega»,
   e quando l'app lo chiede premi una volta il pulsante. L'identità resta salvata nell'app.
3. Scrivi sul pad: i tratti compaiono sul telefono. «Ruota» gira il foglio.

Un pad registrato con la desktop va registrato di nuovo per il telefono (ogni app ha la sua
identità; dopo, la desktop andrà registrata di nuovo a sua volta).
