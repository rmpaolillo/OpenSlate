# OpenSlate per iPhone e iPad (prima versione)

App SwiftUI con lo stesso cuore Rust dell'app Android (`../mobile/core`, che usa gli stessi
file della desktop): lavagna live col Bamboo Slate (CoreBluetooth), pagine scaricate dal pad,
libreria nel formato della desktop, strumenti, fogli, chat AI col tutor che scrive sul foglio,
trascrizione in nota, documenti da firmare (PDF, Word, foto, SVG) e impostazioni AI importate
dalla desktop col QR (Impostazioni AI → «Collega il telefono»). Le chiavi API stanno nel
Portachiavi di iOS, solo su questo dispositivo.

## Compilare

Servono Xcode, Rust con i target iOS e XcodeGen:

    rustup target add aarch64-apple-ios aarch64-apple-ios-sim
    brew install xcodegen
    cd ios
    ./build-rust.sh          # il cuore Rust → Generated/ (XCFramework + binding Swift)
    xcodegen generate        # → OpenSlate.xcodeproj
    open OpenSlate.xcodeproj

Xcode rilancia `build-rust.sh` da sé prima di ogni compilazione.

## Sul tuo iPhone

1. In Xcode → Settings → Components scarica la piattaforma iOS della tua versione di Xcode.
2. Xcode → Settings → Accounts: aggiungi il tuo Apple ID.
3. Nel progetto, target OpenSlate → Signing & Capabilities: scegli la tua squadra (Team).
   Con un Apple ID gratuito l'app resta installata 7 giorni; con l'Apple Developer Program
   (99 €/anno) si può distribuire con TestFlight.
4. Collega l'iPhone, sceglilo come destinazione e premi Run.

Il Bluetooth non c'è nel simulatore: il pad si prova solo su un iPhone o un iPad veri.
