//! Su macOS collega la libreria di runtime di clang (`libclang_rt.osx.a`).
//!
//! whisper.cpp (Metal) usa `@available`, che chiama `__isPlatformVersionAtLeast`
//! quando l'app punta a un macOS più vecchio dell'SDK (le release: macOS 11).
//! Il collegamento finale di Rust non include quella libreria: senza, il link
//! fallisce con «Undefined symbols: ___isPlatformVersionAtLeast».

use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    let Ok(out) = Command::new("xcrun").args(["clang", "--print-runtime-dir"]).output() else {
        println!("cargo:warning=xcrun non trovato: libclang_rt.osx.a non collegata");
        return;
    };
    let lib = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim()).join("libclang_rt.osx.a");
    if lib.is_file() {
        println!("cargo:rustc-link-arg={}", lib.display());
    } else {
        println!("cargo:warning=libclang_rt.osx.a non trovata in {}", lib.display());
    }
}
