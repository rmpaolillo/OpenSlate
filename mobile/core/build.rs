// I file di src/ inclusi qui (boardread, boardmarks) escludono i loro test
// della desktop con questo cfg.
fn main() {
    println!("cargo::rustc-check-cfg=cfg(openslate_mobile)");
    println!("cargo:rustc-cfg=openslate_mobile");
}
