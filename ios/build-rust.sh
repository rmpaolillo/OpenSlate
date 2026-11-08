#!/bin/sh
# Compila il cuore Rust (mobile/core, condiviso con Android) per iPhone e per il
# simulatore, genera i binding Swift e li impacchetta in Generated/ per Xcode
# (OpenSlateCore.xcframework + openslate_mobile.swift). Lo lancia Xcode prima
# di compilare l'app; si può anche lanciare a mano.
set -eu
# Dentro Xcode le variabili del progetto (SDK, architetture, flag) confondono
# cargo e i compilatori C delle dipendenze: si riparte in un ambiente pulito.
if [ -n "${XCODE_VERSION_ACTUAL:-}" ] && [ -z "${OPENSLATE_CLEAN_ENV:-}" ]; then
    exec env -i HOME="$HOME" USER="${USER:-}" PATH="/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin" \
        DEVELOPER_DIR="${DEVELOPER_DIR:-/Applications/Xcode.app/Contents/Developer}" OPENSLATE_CLEAN_ENV=1 \
        OPENSLATE_PRODUCTS="${BUILT_PRODUCTS_DIR:-}" OPENSLATE_PLATFORM="${PLATFORM_NAME:-}" \
        /bin/sh "$0" "$@"
fi
cd "$(dirname "$0")/../mobile/core"
export PATH="$HOME/.cargo/bin:$PATH"
if [ -z "${DEVELOPER_DIR:-}" ] && [ -d /Applications/Xcode.app/Contents/Developer ]; then
    export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
fi
export IPHONEOS_DEPLOYMENT_TARGET=16.0
OUT=../../ios/Generated
cargo build --release --lib --target aarch64-apple-ios
cargo build --release --lib --target aarch64-apple-ios-sim
# i binding si leggono dalla libreria per il Mac (quelle per iOS sono senza simboli)
cargo build --lib
cargo run --features cli --bin uniffi-bindgen -- generate \
    --library target/debug/libopenslate_mobile.dylib --language swift --out-dir "$OUT/swift" --no-format
mkdir -p "$OUT/headers"
cp "$OUT/swift/openslate_mobileFFI.h" "$OUT/headers/"
cp "$OUT/swift/openslate_mobileFFI.modulemap" "$OUT/headers/module.modulemap"
# si prepara a parte e si mette al posto del vecchio con due rinomine: Xcode
# lo legge mentre questo script gira, non deve mai trovarlo a metà
rm -rf "$OUT/.new.xcframework" "$OUT/.old.xcframework"
xcodebuild -create-xcframework \
    -library target/aarch64-apple-ios/release/libopenslate_mobile.a -headers "$OUT/headers" \
    -library target/aarch64-apple-ios-sim/release/libopenslate_mobile.a -headers "$OUT/headers" \
    -output "$OUT/.new.xcframework"
[ -d "$OUT/OpenSlateCore.xcframework" ] && mv "$OUT/OpenSlateCore.xcframework" "$OUT/.old.xcframework"
mv "$OUT/.new.xcframework" "$OUT/OpenSlateCore.xcframework"
rm -rf "$OUT/.old.xcframework"
# Xcode copia libreria e intestazioni dell'XCFramework nella cartella della
# build PRIMA di questo script (anche se l'app dipende da lui): dopo una
# modifica al cuore le copie sarebbero vecchie, quindi le si aggiorna qui.
if [ -n "${OPENSLATE_PRODUCTS:-}" ]; then
    case "${OPENSLATE_PLATFORM:-}" in
        iphonesimulator) LIB=target/aarch64-apple-ios-sim/release/libopenslate_mobile.a ;;
        *) LIB=target/aarch64-apple-ios/release/libopenslate_mobile.a ;;
    esac
    mkdir -p "$OPENSLATE_PRODUCTS/include"
    cp "$LIB" "$OPENSLATE_PRODUCTS/"
    cp "$OUT/headers/openslate_mobileFFI.h" "$OUT/headers/module.modulemap" "$OPENSLATE_PRODUCTS/include/"
fi
