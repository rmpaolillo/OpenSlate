#!/bin/sh
# Compila il cuore Rust (mobile/core, condiviso con iOS) per Android e genera i binding Kotlin.
# Lo chiama Gradle prima di ogni build; si può anche lanciare a mano.
set -eu
cd "$(dirname "$0")/../mobile/core"
export PATH="$HOME/.cargo/bin:$PATH"
SDK="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
if [ -z "${ANDROID_NDK_HOME:-}" ]; then
    ANDROID_NDK_HOME="$(ls -d "$SDK"/ndk/* | sort -V | tail -1)"
    export ANDROID_NDK_HOME
fi
OUT=../../android/app/build/generated/rust
cargo ndk -t arm64-v8a -t x86_64 --platform 26 -o "$OUT/jniLibs" build --release
# I binding si leggono dalla libreria per il computer (quella Android è senza simboli).
cargo build --lib
case "$(uname)" in
    Darwin) HOST_LIB=target/debug/libopenslate_mobile.dylib ;;
    *) HOST_LIB=target/debug/libopenslate_mobile.so ;;
esac
cargo run --features cli --bin uniffi-bindgen -- generate \
    --library "$HOST_LIB" --language kotlin --out-dir "$OUT/kotlin" --no-format
