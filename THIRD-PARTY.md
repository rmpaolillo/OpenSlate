# Third-party software

OpenSlate stands on the shoulders of a lot of open-source work. This file lists
everything that ends up in (or is needed to build) the program. Thank you to all the
authors and contributors.

OpenSlate itself is licensed under the GNU General Public License v3.0 or later
(see [LICENSE](LICENSE)). The licences of all components below are compatible with it.
Per-file attributions are in [NOTICE](NOTICE).

## Code and ideas that shaped the project

| Project | Authors | Licence | How it is used |
|---|---|---|---|
| [Tuhi](https://github.com/tuhiproject/tuhi) | Peter Hutterer, Benjamin Tissoires and the Tuhi contributors | GPL-2.0-or-later | The Bluetooth protocol of the Wacom SmartPad family was translated from Tuhi's Python code. Without Tuhi's reverse engineering this project would not exist. |
| [WordCraft](https://github.com/storytold/wordcraft) | ArtCraft Team and the WordCraft contributors | MIT OR Apache-2.0 | `wordcraft-engine` crate: opening, rendering and saving Word documents with the handwritten ink. The Word-like look of the interface (colours, measurements, rulers) is adapted from its `ui-egui` crate. |
| [PdfCraft](https://github.com/storytold/pdfcraft) | ArtCraft Team and the PdfCraft contributors | MIT OR Apache-2.0 | `pdfcraft-render` crate: crisp PDF page rendering at any zoom, plus the patched `hayro`, `lopdf` and `vello_common` crates. |
| [craft-libs](https://github.com/storytold/craft-libs) | ArtCraft Team | MIT OR Apache-2.0 | Patched `krilla` (PDF writing) used by WordCraft. |
| [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness) | DeepSeek | — | Inspiration for the chat (ideas only, no code copied). |

## Fonts, icons and web assets

| Asset | Authors | Licence | Where |
|---|---|---|---|
| [Inter](https://github.com/rsms/inter) | Rasmus Andersson and The Inter Project Authors | SIL OFL 1.1 | `assets/fonts/Inter-*.ttf` (subset: Private Use Area removed) |
| [Source Serif 4](https://github.com/adobe-fonts/source-serif) | Frank Grießhammer, Adobe | SIL OFL 1.1 | `assets/fonts/SourceSerif4-*.ttf` |
| [Phosphor Icons](https://phosphoricons.com) | Helena Zhang, Tobias Fried | MIT | via the `egui-phosphor` crate |
| [KaTeX](https://katex.org) | Khan Academy and contributors | MIT | `src/web/vendor/katex*` (formulas) |
| [marked](https://marked.js.org) | Christopher Jeffrey and the marked contributors | MIT | `src/web/vendor/marked.min.js` (Markdown) |

## Platform components used at run time (not bundled)

- **macOS**: CoreBluetooth, WKWebView, PDFKit and Vision (through small Swift helpers in `src/swift/`), Keychain.
- **Linux**: BlueZ (via D-Bus), GTK 3, WebKitGTK; optionally LibreOffice and Poppler (`pdftoppm`).
- **Windows**: WinRT Bluetooth, WebView2.

## Rust crates

Generated from `cargo metadata` (all targets). Licence and author data come from each
crate's own manifest; see the crate's repository for the full licence text.

| Crate | Version | Licence | Authors |
|---|---|---|---|
| [ab_glyph](https://github.com/alexheretic/ab-glyph) | 0.2.32 | Apache-2.0 | Alex Butler |
| [ab_glyph_rasterizer](https://github.com/alexheretic/ab-glyph) | 0.1.10 | Apache-2.0 | Alex Butler |
| [accesskit](https://github.com/AccessKit/accesskit) | 0.24.1 | MIT OR Apache-2.0 | The AccessKit contributors |
| [accesskit_atspi_common](https://github.com/AccessKit/accesskit) | 0.18.1 | MIT OR Apache-2.0 | The AccessKit contributors |
| [accesskit_consumer](https://github.com/AccessKit/accesskit) | 0.35.0 | MIT OR Apache-2.0 | The AccessKit contributors |
| [accesskit_consumer](https://github.com/AccessKit/accesskit) | 0.36.0 | MIT OR Apache-2.0 | The AccessKit contributors |
| [accesskit_consumer](https://github.com/AccessKit/accesskit) | 0.38.0 | MIT OR Apache-2.0 | The AccessKit contributors |
| [accesskit_macos](https://github.com/AccessKit/accesskit) | 0.26.3 | MIT OR Apache-2.0 | The AccessKit contributors |
| [accesskit_unix](https://github.com/AccessKit/accesskit) | 0.21.1 | MIT OR Apache-2.0 | The AccessKit contributors |
| [accesskit_windows](https://github.com/AccessKit/accesskit) | 0.32.1 | MIT OR Apache-2.0 | The AccessKit contributors |
| [accesskit_winit](https://github.com/AccessKit/accesskit) | 0.32.2 | Apache-2.0 | The AccessKit contributors |
| [adler2](https://github.com/oyvindln/adler2) | 2.0.1 | 0BSD OR MIT OR Apache-2.0 | Jonas Schievink, oyvindln |
| [aes](https://github.com/RustCrypto/block-ciphers) | 0.9.3 | MIT OR Apache-2.0 | RustCrypto Developers |
| [ahash](https://github.com/tkaitchuck/ahash) | 0.8.12 | MIT OR Apache-2.0 | Tom Kaitchuck |
| [aho-corasick](https://github.com/BurntSushi/aho-corasick) | 1.1.5 | Unlicense OR MIT | Andrew Gallant |
| [alloc-no-stdlib](https://github.com/dropbox/rust-alloc-no-stdlib) | 2.0.4 | BSD-3-Clause | Daniel Reiter Horn |
| [alloc-stdlib](https://github.com/dropbox/rust-alloc-no-stdlib) | 0.2.4 | BSD-3-Clause | Daniel Reiter Horn |
| [allocator-api2](https://github.com/zakarumych/allocator-api2) | 0.2.21 | MIT OR Apache-2.0 | Zakarum |
| [android-activity](https://github.com/rust-mobile/android-activity) | 0.6.1 | MIT OR Apache-2.0 | — |
| [android-properties](https://github.com/miklelappo/android-properties) | 0.2.2 | MIT | Mikhail Lappo |
| [android_system_properties](https://github.com/nical/android_system_properties) | 0.1.6 | MIT OR Apache-2.0 | Nicolas Silva |
| [anstream](https://github.com/rust-cli/anstyle.git) | 1.0.0 | MIT OR Apache-2.0 | — |
| [anstyle](https://github.com/rust-cli/anstyle.git) | 1.0.14 | MIT OR Apache-2.0 | — |
| [anstyle-parse](https://github.com/rust-cli/anstyle.git) | 1.0.0 | MIT OR Apache-2.0 | — |
| [anstyle-query](https://github.com/rust-cli/anstyle.git) | 1.1.5 | MIT OR Apache-2.0 | — |
| [anstyle-wincon](https://github.com/rust-cli/anstyle.git) | 3.0.11 | MIT OR Apache-2.0 | — |
| [anyhow](https://github.com/dtolnay/anyhow) | 1.0.104 | MIT OR Apache-2.0 | David Tolnay |
| [arbitrary](https://github.com/rust-fuzz/arbitrary/) | 1.5.0 | MIT OR Apache-2.0 | The Rust-Fuzz Project Developers, Nick Fitzgerald, Manish Goregaokar, Simonas Kazlauskas, Brian L. Troutwine, Corey F… |
| [arboard](https://github.com/1Password/arboard) | 3.7.0 | MIT OR Apache-2.0 | — |
| [arrayref](https://github.com/droundy/arrayref) | 0.3.9 | BSD-2-Clause | David Roundy |
| [arrayvec](https://github.com/bluss/arrayvec) | 0.7.8 | MIT OR Apache-2.0 | bluss |
| [as-raw-xcb-connection](https://github.com/psychon/as-raw-xcb-connection) | 1.0.1 | MIT OR Apache-2.0 | — |
| [ash](https://github.com/ash-rs/ash) | 0.38.0+1.3.281 | MIT OR Apache-2.0 | Maik Klein, Benjamin Saunders, Marijn Suijten |
| [ashpd](https://github.com/bilelmoussaoui/ashpd) | 0.8.2 | MIT | Bilal Elmoussaoui |
| [async-broadcast](https://github.com/smol-rs/async-broadcast) | 0.7.2 | MIT OR Apache-2.0 | Stjepan Glavina, Yoshua Wuyts, Zeeshan Ali Khan |
| [async-channel](https://github.com/smol-rs/async-channel) | 2.5.0 | Apache-2.0 OR MIT | Stjepan Glavina |
| [async-executor](https://github.com/smol-rs/async-executor) | 1.14.0 | Apache-2.0 OR MIT | Stjepan Glavina, John Nunley |
| [async-fs](https://github.com/smol-rs/async-fs) | 2.2.0 | Apache-2.0 OR MIT | Stjepan Glavina |
| [async-io](https://github.com/smol-rs/async-io) | 2.6.0 | Apache-2.0 OR MIT | Stjepan Glavina |
| [async-lock](https://github.com/smol-rs/async-lock) | 3.4.2 | Apache-2.0 OR MIT | Stjepan Glavina |
| [async-net](https://github.com/smol-rs/async-net) | 2.0.0 | Apache-2.0 OR MIT | Stjepan Glavina |
| [async-process](https://github.com/smol-rs/async-process) | 2.5.0 | Apache-2.0 OR MIT | Stjepan Glavina |
| [async-recursion](https://github.com/dcchut/async-recursion) | 1.2.0 | MIT OR Apache-2.0 | Robert Usher |
| [async-signal](https://github.com/smol-rs/async-signal) | 0.2.14 | Apache-2.0 OR MIT | John Nunley |
| [async-task](https://github.com/smol-rs/async-task) | 4.7.1 | Apache-2.0 OR MIT | Stjepan Glavina |
| [async-trait](https://github.com/dtolnay/async-trait) | 0.1.92 | MIT OR Apache-2.0 | David Tolnay |
| [atk](https://github.com/gtk-rs/gtk3-rs) | 0.18.2 | MIT | The gtk-rs Project Developers |
| [atk-sys](https://github.com/gtk-rs/gtk3-rs) | 0.18.2 | MIT | The gtk-rs Project Developers |
| [atomic-waker](https://github.com/smol-rs/atomic-waker) | 1.1.2 | Apache-2.0 OR MIT | Stjepan Glavina, Contributors to futures-rs |
| [atspi](https://github.com/odilia-app/atspi) | 0.29.0 | Apache-2.0 OR MIT | Alberto Tirla, DataTriny, Luuk van der Duim, Michael Connor Buchan, Tait Hoyem |
| [atspi-common](https://github.com/odilia-app/atspi) | 0.13.0 | Apache-2.0 OR MIT | — |
| [atspi-proxies](https://github.com/odilia-app/atspi) | 0.13.0 | Apache-2.0 OR MIT | — |
| [autocfg](https://github.com/cuviper/autocfg) | 1.5.1 | Apache-2.0 OR MIT | Josh Stone |
| [base64](https://github.com/marshallpierce/rust-base64) | 0.22.1 | MIT OR Apache-2.0 | Marshall Pierce |
| [bit-set](https://github.com/contain-rs/bit-set) | 0.10.0 | Apache-2.0 OR MIT | Alexis Beingessner |
| [bit-set](https://github.com/contain-rs/bit-set) | 0.8.0 | Apache-2.0 OR MIT | Alexis Beingessner |
| [bit-vec](https://github.com/contain-rs/bit-vec) | 0.8.0 | Apache-2.0 OR MIT | Alexis Beingessner |
| [bit-vec](https://github.com/contain-rs/bit-vec) | 0.9.1 | Apache-2.0 OR MIT | Alexis Beingessner |
| [bitflags](https://github.com/bitflags/bitflags) | 1.3.2 | MIT/Apache-2.0 | The Rust Project Developers |
| [bitflags](https://github.com/bitflags/bitflags) | 2.13.2 | MIT OR Apache-2.0 | The Rust Project Developers |
| [block](http://github.com/SSheldon/rust-block) | 0.1.6 | MIT | Steven Sheldon |
| [block-buffer](https://github.com/RustCrypto/utils) | 0.10.4 | MIT OR Apache-2.0 | RustCrypto Developers |
| [block-buffer](https://github.com/RustCrypto/utils) | 0.12.1 | MIT OR Apache-2.0 | RustCrypto Developers |
| [block-padding](https://github.com/RustCrypto/utils) | 0.4.2 | MIT OR Apache-2.0 | RustCrypto Developers |
| [block2](https://github.com/madsmtm/objc2) | 0.5.1 | MIT | Steven Sheldon, Mads Marquart |
| [block2](https://github.com/madsmtm/objc2) | 0.6.2 | MIT | Mads Marquart |
| [blocking](https://github.com/smol-rs/blocking) | 1.7.0 | Apache-2.0 OR MIT | — |
| [bluez-async](https://github.com/bluez-rs/bluez-async/) | 0.8.2 | MIT OR Apache-2.0 | Andrew Walbran, David Laban |
| [bluez-generated](https://github.com/bluez-rs/bluez-async/) | 0.4.0 | MIT OR Apache-2.0 | Andrew Walbran, David Laban |
| [brotli](https://github.com/dropbox/rust-brotli) | 8.0.4 | BSD-3-Clause AND MIT | Daniel Reiter Horn, The Brotli Authors |
| [brotli-decompressor](https://github.com/dropbox/rust-brotli-decompressor) | 5.0.3 | BSD-3-Clause/MIT | Daniel Reiter Horn, The Brotli Authors |
| [btleplug](https://github.com/deviceplug/btleplug) | 0.11.8 | MIT/Apache-2.0/BSD-3-Clause | Nonpolynomial, LLC |
| [bumpalo](https://github.com/fitzgen/bumpalo) | 3.20.3 | MIT OR Apache-2.0 | Nick Fitzgerald |
| [bytemuck](https://github.com/Lokathor/bytemuck) | 1.25.2 | Zlib OR Apache-2.0 OR MIT | Lokathor |
| [bytemuck_derive](https://github.com/Lokathor/bytemuck) | 1.12.1 | Zlib OR Apache-2.0 OR MIT | Lokathor |
| [byteorder-lite](https://github.com/image-rs/byteorder-lite) | 0.1.0 | Unlicense OR MIT | — |
| [bytes](https://github.com/tokio-rs/bytes) | 1.12.1 | MIT | Carl Lerche, Sean McArthur |
| [cairo-rs](https://github.com/gtk-rs/gtk-rs-core) | 0.18.5 | MIT | The gtk-rs Project Developers |
| [cairo-sys-rs](https://github.com/gtk-rs/gtk-rs-core) | 0.18.2 | MIT | The gtk-rs Project Developers |
| [calloop](https://github.com/Smithay/calloop) | 0.13.0 | MIT | Elinor Berger |
| [calloop](https://github.com/Smithay/calloop) | 0.14.5 | MIT | Elinor Berger |
| [calloop-wayland-source](https://github.com/smithay/calloop-wayland-source) | 0.3.0 | MIT | Kirill Chibisov |
| [calloop-wayland-source](https://github.com/smithay/calloop-wayland-source) | 0.4.1 | MIT | Kirill Chibisov |
| [cbc](https://github.com/RustCrypto/block-modes) | 0.2.1 | MIT OR Apache-2.0 | RustCrypto Developers |
| [cc](https://github.com/rust-lang/cc-rs) | 1.9.0 | MIT OR Apache-2.0 | — |
| [cesu8](https://github.com/emk/cesu8-rs) | 1.1.0 | Apache-2.0/MIT | Eric Kidd |
| [cfb](https://github.com/mdsteele/rust-cfb) | 0.15.0 | MIT | Matthew D. Steele |
| [cfg-expr](https://github.com/EmbarkStudios/cfg-expr) | 0.15.8 | MIT OR Apache-2.0 | Embark, Jake Shadle |
| [cfg-if](https://github.com/rust-lang/cfg-if) | 1.0.5 | MIT OR Apache-2.0 | Alex Crichton |
| [cfg_aliases](https://github.com/katharostech/cfg_aliases) | 0.2.2 | MIT | Zicklag |
| [cgl](https://github.com/servo/cgl-rs) | 0.3.2 | MIT / Apache-2.0 | The Servo Project Developers |
| [chacha20](https://github.com/RustCrypto/stream-ciphers) | 0.10.2 | MIT OR Apache-2.0 | RustCrypto Developers |
| [chardetng](https://github.com/hsivonen/chardetng) | 1.0.0 | Apache-2.0 OR MIT | Henri Sivonen |
| [chrono](https://github.com/chronotope/chrono) | 0.4.45 | MIT OR Apache-2.0 | — |
| [cipher](https://github.com/RustCrypto/traits) | 0.5.2 | MIT OR Apache-2.0 | RustCrypto Developers |
| [clap](https://github.com/clap-rs/clap) | 4.6.7 | MIT OR Apache-2.0 | — |
| [clap_builder](https://github.com/clap-rs/clap) | 4.6.7 | MIT OR Apache-2.0 | — |
| [clap_derive](https://github.com/clap-rs/clap) | 4.6.7 | MIT OR Apache-2.0 | — |
| [clap_lex](https://github.com/clap-rs/clap) | 1.1.1 | MIT OR Apache-2.0 | — |
| [clipboard-win](https://github.com/DoumanAsh/clipboard-win) | 5.4.1 | BSL-1.0 | Douman |
| [cmov](https://github.com/RustCrypto/utils) | 0.5.4 | Apache-2.0 OR MIT | RustCrypto Developers |
| [codespan-reporting](https://github.com/brendanzab/codespan) | 0.13.1 | Apache-2.0 | Brendan Zabarauskas |
| [color](https://github.com/linebender/color) | 0.3.3 | Apache-2.0 OR MIT | — |
| [color_quant](https://github.com/image-rs/color_quant.git) | 1.1.0 | MIT | nwin |
| [colorchoice](https://github.com/rust-cli/anstyle.git) | 1.0.5 | MIT OR Apache-2.0 | — |
| [combine](https://github.com/Marwes/combine) | 4.6.9 | MIT | Markus Westerlind |
| [concurrent-queue](https://github.com/smol-rs/concurrent-queue) | 2.5.0 | Apache-2.0 OR MIT | Stjepan Glavina, Taiki Endo, John Nunley |
| [const-oid](https://github.com/RustCrypto/formats) | 0.10.2 | Apache-2.0 OR MIT | RustCrypto Developers |
| [cookie](https://github.com/SergioBenitez/cookie-rs) | 0.18.2 | MIT OR Apache-2.0 | Sergio Benitez, Alex Crichton |
| [core-foundation](https://github.com/servo/core-foundation-rs) | 0.10.1 | MIT OR Apache-2.0 | The Servo Project Developers |
| [core-foundation](https://github.com/servo/core-foundation-rs) | 0.9.4 | MIT OR Apache-2.0 | The Servo Project Developers |
| [core-foundation-sys](https://github.com/servo/core-foundation-rs) | 0.8.7 | MIT OR Apache-2.0 | The Servo Project Developers |
| [core-graphics](https://github.com/servo/core-foundation-rs) | 0.23.2 | MIT OR Apache-2.0 | The Servo Project Developers |
| [core-graphics](https://github.com/servo/core-foundation-rs) | 0.25.0 | MIT OR Apache-2.0 | The Servo Project Developers |
| [core-graphics-types](https://github.com/servo/core-foundation-rs) | 0.1.3 | MIT OR Apache-2.0 | The Servo Project Developers |
| [core-graphics-types](https://github.com/servo/core-foundation-rs) | 0.2.0 | MIT OR Apache-2.0 | The Servo Project Developers |
| [core_detect](https://github.com/thomcc/core_detect) | 1.0.0 | MIT/Apache-2.0 | Thom Chiovoloni |
| [core_maths](https://github.com/robertbastian/core_maths) | 0.1.1 | MIT | Robert Bastian |
| [cpubits](https://github.com/RustCrypto/utils) | 0.1.1 | MIT OR Apache-2.0 | RustCrypto Developers |
| [cpufeatures](https://github.com/RustCrypto/utils) | 0.2.17 | MIT OR Apache-2.0 | RustCrypto Developers |
| [cpufeatures](https://github.com/RustCrypto/utils) | 0.3.1 | MIT OR Apache-2.0 | RustCrypto Developers |
| [craft-pdf](https://github.com/storytold/craft-libs) (git) | 0.1.0 | MIT OR Apache-2.0 | — |
| [crc32fast](https://github.com/srijs/rust-crc32fast) | 1.5.2 | MIT OR Apache-2.0 | Sam Rijs, Alex Crichton |
| [crossbeam-channel](https://github.com/crossbeam-rs/crossbeam) | 0.5.17 | MIT OR Apache-2.0 | — |
| [crossbeam-deque](https://github.com/crossbeam-rs/crossbeam) | 0.8.8 | MIT OR Apache-2.0 | — |
| [crossbeam-epoch](https://github.com/crossbeam-rs/crossbeam) | 0.9.21 | MIT OR Apache-2.0 | — |
| [crossbeam-utils](https://github.com/crossbeam-rs/crossbeam) | 0.8.23 | MIT OR Apache-2.0 | — |
| [crunchy](https://github.com/eira-fransham/crunchy) | 0.2.4 | MIT | Eira Fransham |
| [crypto-common](https://github.com/RustCrypto/traits) | 0.1.7 | MIT OR Apache-2.0 | RustCrypto Developers |
| [crypto-common](https://github.com/RustCrypto/traits) | 0.2.2 | MIT OR Apache-2.0 | RustCrypto Developers |
| [cssparser](https://github.com/servo/rust-cssparser) | 0.37.0 | MPL-2.0 | Simon Sapin |
| [cssparser-macros](https://github.com/servo/rust-cssparser) | 0.7.1 | MPL-2.0 | Simon Sapin |
| [ctutils](https://github.com/RustCrypto/utils) | 0.4.3 | Apache-2.0 OR MIT | RustCrypto Developers |
| [cursor-icon](https://github.com/rust-windowing/cursor-icon) | 1.2.0 | MIT OR Apache-2.0 OR Zlib | Kirill Chibisov |
| [dashmap](https://github.com/xacrimon/dashmap) | 5.5.3 | MIT | Acrimon |
| [dashmap](https://github.com/xacrimon/dashmap) | 6.2.1 | MIT | Acrimon |
| [dbus](https://github.com/diwic/dbus-rs) | 0.9.12 | Apache-2.0/MIT | David Henningsson |
| [dbus-tokio](https://github.com/diwic/dbus-rs) | 0.7.6 | Apache-2.0/MIT | David Henningsson |
| [deranged](https://github.com/jhpratt/deranged) | 0.5.9 | MIT OR Apache-2.0 | Jacob Pratt |
| [derive_arbitrary](https://github.com/rust-fuzz/arbitrary) | 1.5.0 | MIT OR Apache-2.0 | The Rust-Fuzz Project Developers, Nick Fitzgerald, Manish Goregaokar, Andre Bogus, Corey Farwell |
| [derive_more](https://github.com/JelteF/derive_more) | 2.1.1 | MIT | Jelte Fennema |
| [derive_more-impl](https://github.com/JelteF/derive_more) | 2.1.1 | MIT | Jelte Fennema |
| [digest](https://github.com/RustCrypto/traits) | 0.10.7 | MIT OR Apache-2.0 | RustCrypto Developers |
| [digest](https://github.com/RustCrypto/traits) | 0.11.3 | MIT OR Apache-2.0 | RustCrypto Developers |
| [dirs](https://codeberg.org/dirs/dirs-rs) | 7.0.0 | MIT OR Apache-2.0 | Simon Ochsenreither |
| [dirs-sys](https://github.com/dirs-dev/dirs-sys-rs) | 0.5.0 | MIT OR Apache-2.0 | Simon Ochsenreither |
| [dispatch](http://github.com/SSheldon/rust-dispatch) | 0.2.0 | MIT | Steven Sheldon |
| [dispatch2](https://github.com/madsmtm/objc2) | 0.3.1 | Zlib OR Apache-2.0 OR MIT | Mads Marquart, Mary |
| [displaydoc](https://github.com/yaahc/displaydoc) | 0.2.7 | MIT OR Apache-2.0 | Jane Lusby |
| [dlib](https://github.com/elinorbgr/dlib) | 0.5.3 | MIT | Elinor Berger |
| [dlopen2](https://github.com/OpenByteDev/dlopen2) | 0.8.2 | MIT | Szymon Wieloch, Ahmed Masud, OpenByte |
| [dlopen2_derive](https://github.com/OpenByteDev/dlopen2) | 0.4.3 | MIT | Szymon Wieloch, OpenByte |
| [document-features](https://github.com/slint-ui/document-features) | 0.2.12 | MIT OR Apache-2.0 | Slint Developers |
| [dom_query](https://github.com/niklak/dom_query) | 0.28.0 | MIT | niklak, importcjj |
| [downcast-rs](https://github.com/marcianx/downcast-rs) | 1.2.1 | MIT/Apache-2.0 | Ashish Myles, Runji Wang |
| [dpi](https://github.com/rust-windowing/winit) | 0.1.2 | Apache-2.0 AND MIT | — |
| [dtoa](https://github.com/dtolnay/dtoa) | 1.0.11 | MIT OR Apache-2.0 | David Tolnay |
| [dtoa-short](https://github.com/upsuper/dtoa-short) | 0.3.5 | MPL-2.0 | Xidorn Quan |
| [dunce](https://gitlab.com/kornelski/dunce) | 1.0.5 | CC0-1.0 OR MIT-0 OR Apache-2.0 | Kornel |
| [ecb](https://github.com/RustCrypto/block-modes) | 0.2.1 | MIT OR Apache-2.0 | RustCrypto Developers |
| [ecolor](https://github.com/emilk/egui) | 0.36.2 | MIT OR Apache-2.0 | Emil Ernerfeldt, Andreas Reich |
| [eframe](https://github.com/emilk/egui/tree/main/crates/eframe) | 0.36.2 | MIT OR Apache-2.0 | Emil Ernerfeldt |
| [egui](https://github.com/emilk/egui) | 0.36.2 | MIT OR Apache-2.0 | Emil Ernerfeldt |
| [egui-phosphor](https://github.com/amPerl/egui-phosphor) | 0.14.0 | MIT OR Apache-2.0 | — |
| [egui-wgpu](https://github.com/emilk/egui/tree/main/crates/egui-wgpu) | 0.36.2 | MIT OR Apache-2.0 | Nils Hasenbanck, embotech, Emil Ernerfeldt |
| [egui-winit](https://github.com/emilk/egui/tree/main/crates/egui-winit) | 0.36.2 | MIT OR Apache-2.0 | Emil Ernerfeldt |
| [egui_glow](https://github.com/emilk/egui/tree/main/crates/egui_glow) | 0.36.2 | MIT OR Apache-2.0 | Emil Ernerfeldt |
| [either](https://github.com/rayon-rs/either) | 1.19.0 | MIT OR Apache-2.0 | — |
| [emath](https://github.com/emilk/egui/tree/main/crates/emath) | 0.36.2 | MIT OR Apache-2.0 | Emil Ernerfeldt |
| [encoding_rs](https://github.com/hsivonen/encoding_rs) | 0.8.42 | (Apache-2.0 OR MIT) AND BSD-3-Clause | Henri Sivonen |
| [endi](https://github.com/zeenix/endi) | 1.1.1 | MIT | Zeeshan Ali Khan |
| [enumflags2](https://github.com/meithecatte/enumflags2) | 0.7.12 | MIT OR Apache-2.0 | maik klein, Maja Kądziołka |
| [enumflags2_derive](https://github.com/meithecatte/enumflags2) | 0.7.12 | MIT OR Apache-2.0 | maik klein, Maja Kądziołka |
| [epaint](https://github.com/emilk/egui/tree/main/crates/epaint) | 0.36.2 | MIT OR Apache-2.0 | Emil Ernerfeldt |
| [epaint_default_fonts](https://github.com/emilk/egui/tree/main/crates/epaint_default_fonts) | 0.36.2 | (MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0 | Emil Ernerfeldt |
| [equivalent](https://github.com/indexmap-rs/equivalent) | 1.0.3 | Apache-2.0 OR MIT | — |
| [errno](https://github.com/lambda-fairy/rust-errno) | 0.3.14 | MIT OR Apache-2.0 | Chris Wong, Dan Gohman |
| [error-code](https://github.com/DoumanAsh/error-code) | 3.4.0 | BSL-1.0 | Douman |
| [euclid](https://github.com/servo/euclid) | 0.22.14 | MIT OR Apache-2.0 | The Servo Project Developers |
| [event-listener](https://github.com/smol-rs/event-listener) | 5.4.2 | Apache-2.0 OR MIT | Stjepan Glavina, John Nunley |
| [event-listener-strategy](https://github.com/smol-rs/event-listener-strategy) | 0.5.4 | Apache-2.0 OR MIT | John Nunley |
| [fastrand](https://github.com/smol-rs/fastrand) | 2.5.0 | Apache-2.0 OR MIT | Stjepan Glavina |
| [fax](https://github.com/pdf-rs/fax) | 0.2.7 | MIT | Sebastian K |
| [fdeflate](https://github.com/image-rs/fdeflate) | 0.3.7 | MIT OR Apache-2.0 | The image-rs Developers |
| [fearless_simd](https://github.com/linebender/fearless_simd) | 0.4.1 | Apache-2.0 OR MIT | Raph Levien |
| [field-offset](https://github.com/Diggsey/rust-field-offset) | 0.3.6 | MIT OR Apache-2.0 | Diggory Blake |
| [find-msvc-tools](https://github.com/rust-lang/cc-rs) | 0.1.15 | MIT OR Apache-2.0 | — |
| [flate2](https://github.com/rust-lang/flate2-rs) | 1.1.10 | MIT OR Apache-2.0 | Alex Crichton, Josh Triplett |
| [float-cmp](https://github.com/mikedilger/float-cmp) | 0.9.0 | MIT | Mike Dilger |
| [fnv](https://github.com/servo/rust-fnv) | 1.0.7 | Apache-2.0 / MIT | Alex Crichton |
| [foldhash](https://github.com/orlp/foldhash) | 0.2.0 | Zlib | Orson Peters |
| [font-types](https://github.com/googlefonts/fontations) | 0.11.3 | MIT OR Apache-2.0 | — |
| [font-types](https://github.com/googlefonts/fontations) | 0.12.7 | MIT OR Apache-2.0 | — |
| [foreign-types](https://github.com/sfackler/foreign-types) | 0.5.0 | MIT/Apache-2.0 | Steven Fackler |
| [foreign-types-macros](https://github.com/sfackler/foreign-types) | 0.2.4 | MIT/Apache-2.0 | Steven Fackler |
| [foreign-types-shared](https://github.com/sfackler/foreign-types) | 0.3.1 | MIT/Apache-2.0 | Steven Fackler |
| [form_urlencoded](https://github.com/servo/rust-url) | 1.2.2 | MIT OR Apache-2.0 | The rust-url developers |
| [futures](https://github.com/rust-lang/futures-rs) | 0.3.34 | MIT OR Apache-2.0 | — |
| [futures-channel](https://github.com/rust-lang/futures-rs) | 0.3.34 | MIT OR Apache-2.0 | — |
| [futures-core](https://github.com/rust-lang/futures-rs) | 0.3.34 | MIT OR Apache-2.0 | — |
| [futures-executor](https://github.com/rust-lang/futures-rs) | 0.3.34 | MIT OR Apache-2.0 | — |
| [futures-io](https://github.com/rust-lang/futures-rs) | 0.3.34 | MIT OR Apache-2.0 | — |
| [futures-lite](https://github.com/smol-rs/futures-lite) | 2.6.1 | Apache-2.0 OR MIT | Stjepan Glavina, Contributors to futures-rs |
| [futures-macro](https://github.com/rust-lang/futures-rs) | 0.3.34 | MIT OR Apache-2.0 | — |
| [futures-sink](https://github.com/rust-lang/futures-rs) | 0.3.34 | MIT OR Apache-2.0 | — |
| [futures-task](https://github.com/rust-lang/futures-rs) | 0.3.34 | MIT OR Apache-2.0 | — |
| [futures-util](https://github.com/rust-lang/futures-rs) | 0.3.34 | MIT OR Apache-2.0 | — |
| [gdk](https://github.com/gtk-rs/gtk3-rs) | 0.18.2 | MIT | The gtk-rs Project Developers |
| [gdk-pixbuf](https://github.com/gtk-rs/gtk-rs-core) | 0.18.5 | MIT | The gtk-rs Project Developers |
| [gdk-pixbuf-sys](https://github.com/gtk-rs/gtk-rs-core) | 0.18.0 | MIT | The gtk-rs Project Developers |
| [gdk-sys](https://github.com/gtk-rs/gtk3-rs) | 0.18.2 | MIT | The gtk-rs Project Developers |
| [gdkwayland-sys](https://github.com/gtk-rs/gtk3-rs) | 0.18.2 | MIT | The gtk-rs Project Developers |
| [gdkx11](https://github.com/gtk-rs/gtk3-rs) | 0.18.2 | MIT | The gtk-rs Project Developers |
| [gdkx11-sys](https://github.com/gtk-rs/gtk3-rs) | 0.18.2 | MIT | The gtk-rs Project Developers |
| [generic-array](https://github.com/fizyk20/generic-array.git) | 0.14.7 | MIT | Bartłomiej Kamiński, Aaron Trent |
| [gethostname](https://codeberg.org/swsnr/gethostname.rs.git) | 1.1.0 | Apache-2.0 | Sebastian Wiesner |
| [getrandom](https://github.com/rust-random/getrandom) | 0.2.17 | MIT OR Apache-2.0 | The Rand Project Developers |
| [getrandom](https://github.com/rust-random/getrandom) | 0.3.4 | MIT OR Apache-2.0 | The Rand Project Developers |
| [getrandom](https://github.com/rust-random/getrandom) | 0.4.3 | MIT OR Apache-2.0 | The Rand Project Developers |
| [gif](https://github.com/image-rs/image-gif) | 0.14.2 | MIT OR Apache-2.0 | The image-rs Developers |
| [gio](https://github.com/gtk-rs/gtk-rs-core) | 0.18.4 | MIT | The gtk-rs Project Developers |
| [gio-sys](https://github.com/gtk-rs/gtk-rs-core) | 0.18.1 | MIT | The gtk-rs Project Developers |
| [gl_generator](https://github.com/brendanzab/gl-rs/) | 0.14.0 | Apache-2.0 | Brendan Zabarauskas, Corey Richardson, Arseny Kapoulkine |
| [glib](https://github.com/gtk-rs/gtk-rs-core) | 0.18.5 | MIT | The gtk-rs Project Developers |
| [glib-macros](https://github.com/gtk-rs/gtk-rs-core) | 0.18.5 | MIT | The gtk-rs Project Developers |
| [glib-sys](https://github.com/gtk-rs/gtk-rs-core) | 0.18.1 | MIT | The gtk-rs Project Developers |
| [glifo](https://github.com/linebender/vello) | 0.1.1 | Apache-2.0 OR MIT | — |
| [glifo](https://github.com/linebender/vello) | 0.2.0 | Apache-2.0 OR MIT | — |
| [glifo](https://github.com/linebender/vello) | 0.3.0 | Apache-2.0 OR MIT | — |
| [glow](https://github.com/grovesNL/glow) | 0.17.0 | MIT OR Apache-2.0 OR Zlib | Joshua Groves, Dzmitry Malyshau |
| [glutin](https://github.com/rust-windowing/glutin) | 0.32.3 | Apache-2.0 | Kirill Chibisov |
| [glutin-winit](https://github.com/rust-windowing/glutin) | 0.5.0 | MIT | Kirill Chibisov |
| [glutin_egl_sys](https://github.com/rust-windowing/glutin) | 0.7.1 | Apache-2.0 | Kirill Chibisov |
| [glutin_glx_sys](https://github.com/rust-windowing/glutin) | 0.6.1 | Apache-2.0 | Kirill Chibisov |
| [glutin_wgl_sys](https://github.com/rust-windowing/glutin) | 0.6.1 | Apache-2.0 | Kirill Chibisov |
| [gobject-sys](https://github.com/gtk-rs/gtk-rs-core) | 0.18.0 | MIT | The gtk-rs Project Developers |
| [gpu-allocator](https://github.com/Traverse-Research/gpu-allocator) | 0.28.0 | MIT OR Apache-2.0 | Traverse Research |
| [gtk](https://github.com/gtk-rs/gtk3-rs) | 0.18.2 | MIT | The gtk-rs Project Developers |
| [gtk-sys](https://github.com/gtk-rs/gtk3-rs) | 0.18.2 | MIT | The gtk-rs Project Developers |
| [gtk3-macros](https://github.com/gtk-rs/gtk3-rs) | 0.18.2 | MIT | The gtk-rs Project Developers |
| [guillotiere](https://github.com/nical/guillotiere) | 0.7.0 | MIT/Apache-2.0 | Nicolas Silva |
| [half](https://github.com/VoidStarKat/half-rs) | 2.7.1 | MIT OR Apache-2.0 | Kathryn Long |
| [harfrust](https://github.com/harfbuzz/harfrust) | 0.12.0 | MIT | — |
| [harfrust](https://github.com/harfbuzz/harfrust) | 0.13.3 | MIT | — |
| [hashbrown](https://github.com/rust-lang/hashbrown) | 0.14.5 | MIT OR Apache-2.0 | Amanieu d'Antras |
| [hashbrown](https://github.com/rust-lang/hashbrown) | 0.16.1 | MIT OR Apache-2.0 | Amanieu d'Antras |
| [hashbrown](https://github.com/rust-lang/hashbrown) | 0.17.1 | MIT OR Apache-2.0 | — |
| [hayro](https://github.com/LaurenzV/hayro) (git) | 0.7.1 | Apache-2.0 OR MIT | Laurenz Stampfl |
| [hayro-ccitt](https://github.com/LaurenzV/hayro) | 0.3.0 | Apache-2.0 OR MIT | Laurenz Stampfl |
| [hayro-cmap](https://github.com/LaurenzV/hayro) | 0.1.0 | Apache-2.0 OR MIT | Laurenz Stampfl |
| [hayro-interpret](https://github.com/LaurenzV/hayro) (git) | 0.7.0 | Apache-2.0 OR MIT | Laurenz Stampfl |
| [hayro-jbig2](https://github.com/LaurenzV/hayro) (git) | 0.3.0 | Apache-2.0 OR MIT | Laurenz Stampfl |
| [hayro-jpeg2000](https://github.com/LaurenzV/hayro) | 0.3.5 | Apache-2.0 OR MIT | Laurenz Stampfl |
| [hayro-postscript](https://github.com/LaurenzV/hayro) | 0.1.0 | Apache-2.0 OR MIT | Laurenz Stampfl |
| [hayro-syntax](https://github.com/LaurenzV/hayro) (git) | 0.7.2 | Apache-2.0 OR MIT | Laurenz Stampfl |
| [heck](https://github.com/withoutboats/heck) | 0.4.1 | MIT OR Apache-2.0 | Without Boats |
| [heck](https://github.com/withoutboats/heck) | 0.5.0 | MIT OR Apache-2.0 | — |
| [hermit-abi](https://github.com/hermit-os/hermit-rs) | 0.5.3 | MIT OR Apache-2.0 | — |
| [hex](https://github.com/KokaKiwi/rust-hex) | 0.4.3 | MIT OR Apache-2.0 | KokaKiwi |
| [hmac](https://github.com/RustCrypto/MACs) | 0.13.0 | MIT OR Apache-2.0 | RustCrypto Developers |
| [html5ever](https://github.com/servo/html5ever) | 0.39.0 | MIT OR Apache-2.0 | The html5ever Project Developers |
| [http](https://github.com/hyperium/http) | 1.5.0 | MIT OR Apache-2.0 | Alex Crichton, Carl Lerche, Sean McArthur |
| [hybrid-array](https://github.com/RustCrypto/hybrid-array) | 0.4.15 | MIT OR Apache-2.0 | RustCrypto Developers |
| [iana-time-zone](https://github.com/strawlab/iana-time-zone) | 0.1.65 | MIT OR Apache-2.0 | Andrew Straw, René Kijewski, Ryan Lopopolo |
| [iana-time-zone-haiku](https://github.com/strawlab/iana-time-zone) | 0.1.2 | MIT OR Apache-2.0 | René Kijewski |
| [icu_collections](https://github.com/unicode-org/icu4x) | 2.3.0 | Unicode-3.0 | The ICU4X Project Developers |
| [icu_locale_core](https://github.com/unicode-org/icu4x) | 2.3.0 | Unicode-3.0 | The ICU4X Project Developers |
| [icu_normalizer](https://github.com/unicode-org/icu4x) | 2.3.0 | Unicode-3.0 | The ICU4X Project Developers |
| [icu_normalizer_data](https://github.com/unicode-org/icu4x) | 2.3.0 | Unicode-3.0 | The ICU4X Project Developers |
| [icu_properties](https://github.com/unicode-org/icu4x) | 2.3.0 | Unicode-3.0 | The ICU4X Project Developers |
| [icu_properties_data](https://github.com/unicode-org/icu4x) | 2.3.0 | Unicode-3.0 | The ICU4X Project Developers |
| [icu_provider](https://github.com/unicode-org/icu4x) | 2.3.1 | Unicode-3.0 | The ICU4X Project Developers |
| [idna](https://github.com/servo/rust-url/) | 1.1.0 | MIT OR Apache-2.0 | The rust-url developers |
| [idna_adapter](https://github.com/hsivonen/idna_adapter) | 1.2.2 | Apache-2.0 OR MIT | The rust-url developers |
| [image](https://github.com/image-rs/image) | 0.25.10 | MIT OR Apache-2.0 | The image-rs Developers |
| [image-webp](https://github.com/image-rs/image-webp) | 0.2.4 | MIT OR Apache-2.0 | — |
| [imagesize](https://github.com/Roughsketch/imagesize) | 0.14.0 | MIT | Maid Dog |
| [indexmap](https://github.com/indexmap-rs/indexmap) | 2.14.2 | Apache-2.0 OR MIT | — |
| [inout](https://github.com/RustCrypto/utils) | 0.2.5 | MIT OR Apache-2.0 | RustCrypto Developers |
| [is_terminal_polyfill](https://github.com/polyfill-rs/is_terminal_polyfill) | 1.70.2 | MIT OR Apache-2.0 | — |
| [itertools](https://github.com/rust-itertools/itertools) | 0.14.0 | MIT OR Apache-2.0 | bluss |
| [itertools](https://github.com/rust-itertools/itertools) | 0.15.0 | MIT OR Apache-2.0 | bluss |
| [itoa](https://github.com/dtolnay/itoa) | 1.0.18 | MIT OR Apache-2.0 | David Tolnay |
| [javascriptcore-rs](https://github.com/tauri-apps/javascriptcore-rs) | 1.1.2 | MIT | — |
| [javascriptcore-rs-sys](https://github.com/tauri-apps/javascriptcore-rs) | 1.1.1 | MIT | The Gtk-rs Project Developers |
| [jni](https://github.com/jni-rs/jni-rs) | 0.19.0 | MIT/Apache-2.0 | Josh Chase |
| [jni](https://github.com/jni-rs/jni-rs) | 0.21.1 | MIT/Apache-2.0 | Josh Chase |
| [jni](https://github.com/jni-rs/jni-rs) | 0.22.4 | MIT OR Apache-2.0 | jni team |
| [jni-macros](https://github.com/jni-rs/jni-rs) | 0.22.4 | MIT OR Apache-2.0 | — |
| [jni-sys](https://github.com/jni-rs/jni-sys) | 0.3.1 | MIT OR Apache-2.0 | Steven Fackler |
| [jni-sys](https://github.com/jni-rs/jni-sys) | 0.4.1 | MIT OR Apache-2.0 | Steven Fackler, Robert Bragg |
| [jni-sys-macros](https://github.com/jni-rs/jni-sys) | 0.4.1 | MIT OR Apache-2.0 | Robert Bragg |
| [jni-utils](https://github.com/deviceplug/jni-utils-rs) | 0.1.1 | BSD-3-Clause | Gedgy Gedgy, Kyle Machulis |
| [jobserver](https://github.com/rust-lang/jobserver-rs) | 0.1.35 | MIT OR Apache-2.0 | Alex Crichton |
| [js-sys](https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/js-sys) | 0.3.106 | MIT OR Apache-2.0 | The wasm-bindgen Developers |
| [keyboard-types](https://github.com/pyfisch/keyboard-types) | 0.7.0 | MIT OR Apache-2.0 | Pyfisch |
| [khronos-egl](https://github.com/timothee-haudebourg/khronos-egl) | 6.0.0 | MIT/Apache-2.0 | Timothée Haudebourg, Sean Kerr |
| [khronos_api](https://github.com/brendanzab/gl-rs/) | 3.1.0 | Apache-2.0 | Brendan Zabarauskas, Corey Richardson, Arseny Kapoulkine, Pierre Krieger |
| [krilla](https://github.com/LaurenzV/krilla) (git) | 0.8.2 | MIT OR Apache-2.0 | Laurenz Stampfl |
| [kurbo](https://github.com/linebender/kurbo) | 0.13.1 | Apache-2.0 OR MIT | — |
| [lazy_static](https://github.com/rust-lang-nursery/lazy-static.rs) | 1.5.1 | MIT OR Apache-2.0 | Marvin Löbel |
| [libc](https://github.com/rust-lang/libc) | 0.2.191 | MIT OR Apache-2.0 | — |
| [libdbus-sys](https://github.com/diwic/dbus-rs) | 0.2.7 | Apache-2.0/MIT | David Henningsson |
| [libloading](https://github.com/nagisa/rust_libloading/) | 0.8.9 | ISC | Simonas Kazlauskas |
| [libm](https://github.com/rust-lang/compiler-builtins) | 0.2.16 | MIT | Alex Crichton, Amanieu d'Antras, Jorge Aparicio, Trevor Gross |
| [libredox](https://gitlab.redox-os.org/redox-os/libredox.git) | 0.1.25 | MIT | 4lDO2 |
| [linebender_resource_handle](https://github.com/linebender/raw_resource_handle) | 0.1.1 | Apache-2.0 OR MIT | — |
| [linux-raw-sys](https://github.com/sunfishcode/linux-raw-sys) | 0.12.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | Dan Gohman |
| [linux-raw-sys](https://github.com/sunfishcode/linux-raw-sys) | 0.4.15 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | Dan Gohman |
| [litemap](https://github.com/unicode-org/icu4x) | 0.8.3 | Unicode-3.0 | The ICU4X Project Developers |
| [litrs](https://github.com/LukasKalbertodt/litrs) | 1.0.1 | MIT OR Apache-2.0 | Lukas Kalbertodt |
| [lock_api](https://github.com/Amanieu/parking_lot) | 0.4.14 | MIT OR Apache-2.0 | Amanieu d'Antras |
| [log](https://github.com/rust-lang/log) | 0.4.34 | MIT OR Apache-2.0 | The Rust Project Developers |
| [lopdf](https://github.com/J-F-Liu/lopdf.git) | 0.34.0 | MIT | Junfeng Liu, Emulator |
| [lopdf](https://github.com/J-F-Liu/lopdf.git) (git) | 0.45.0 | MIT | Junfeng Liu, Emulator |
| [malloc_buf](https://github.com/SSheldon/malloc_buf) | 0.0.6 | MIT | Steven Sheldon |
| [markup5ever](https://github.com/servo/html5ever) | 0.39.0 | MIT OR Apache-2.0 | The html5ever Project Developers |
| [matchers](https://github.com/hawkw/matchers) | 0.2.0 | MIT | Eliza Weisman |
| [md-5](https://github.com/RustCrypto/hashes) | 0.10.6 | MIT OR Apache-2.0 | RustCrypto Developers |
| [md-5](https://github.com/RustCrypto/hashes) | 0.11.0 | MIT OR Apache-2.0 | RustCrypto Developers |
| [memchr](https://github.com/BurntSushi/memchr) | 2.8.3 | Unlicense OR MIT | Andrew Gallant, bluss |
| [memmap2](https://github.com/RazrFalcon/memmap2-rs) | 0.9.11 | MIT OR Apache-2.0 | Dan Burkert, Yevhenii Reizner, The Contributors |
| [memoffset](https://github.com/Gilnaa/memoffset) | 0.9.1 | MIT | Gilad Naaman |
| [minimal-lexical](https://github.com/Alexhuszagh/minimal-lexical) | 0.2.1 | MIT/Apache-2.0 | Alex Huszagh |
| [miniz_oxide](https://github.com/Frommi/miniz_oxide/tree/master/miniz_oxide) | 0.8.9 | MIT OR Zlib OR Apache-2.0 | Frommi, oyvindln, Rich Geldreich richgel99@gmail.com |
| [miniz_oxide](https://github.com/Frommi/miniz_oxide/tree/master/miniz_oxide) | 0.9.1 | MIT OR Zlib OR Apache-2.0 | Frommi, oyvindln, Rich Geldreich richgel99@gmail.com |
| [mio](https://github.com/tokio-rs/mio) | 1.2.4 | MIT | Carl Lerche, Thomas de Zeeuw, Tokio Contributors |
| [moxcms](https://github.com/awxkee/moxcms.git) | 0.8.1 | BSD-3-Clause OR Apache-2.0 | Radzivon Bartoshyk |
| [muda](https://github.com/tauri-apps/muda) | 0.17.2 | Apache-2.0 OR MIT | — |
| [multiversion_no_op](https://github.com/hsivonen/multiversion_no_op) | 1.0.0 | Apache-2.0 OR MIT | Henri Sivonen |
| [naga](https://github.com/gfx-rs/wgpu) | 30.0.1 | MIT OR Apache-2.0 | gfx-rs developers |
| [naga-types](https://github.com/gfx-rs/wgpu) | 30.0.1 | MIT OR Apache-2.0 | gfx-rs developers |
| [ndk](https://github.com/rust-mobile/ndk) | 0.9.0 | MIT OR Apache-2.0 | The Rust Mobile contributors |
| [ndk-context](https://github.com/rust-windowing/android-ndk-rs) | 0.1.1 | MIT OR Apache-2.0 | The Rust Windowing contributors |
| [ndk-sys](https://github.com/rust-mobile/ndk) | 0.6.0+11769913 | MIT OR Apache-2.0 | The Rust Windowing contributors |
| [new_debug_unreachable](https://github.com/mbrubeck/rust-debug-unreachable) | 1.0.6 | MIT | Matt Brubeck, Jonathan Reem |
| [nix](https://github.com/nix-rust/nix) | 0.29.0 | MIT | The nix-rust Project Developers |
| [nohash-hasher](https://github.com/paritytech/nohash-hasher) | 0.2.0 | Apache-2.0 OR MIT | Parity Technologies |
| [nom](https://github.com/Geal/nom) | 7.1.3 | MIT | contact@geoffroycouprie.com |
| [nom](https://github.com/rust-bakery/nom) | 8.0.0 | MIT | contact@geoffroycouprie.com |
| [nu-ansi-term](https://github.com/nushell/nu-ansi-term) | 0.50.3 | MIT | ogham@bsago.me, Ryan Scheel (Havvy), Josh Triplett, The Nushell Project Developers |
| [num-conv](https://github.com/jhpratt/num-conv) | 0.2.2 | MIT OR Apache-2.0 | Jacob Pratt |
| [num-traits](https://github.com/rust-num/num-traits) | 0.2.19 | MIT OR Apache-2.0 | The Rust Project Developers |
| [num_enum](https://github.com/illicitonion/num_enum) | 0.7.6 | BSD-3-Clause OR MIT OR Apache-2.0 | Daniel Wagner-Hall, Daniel Henry-Mantilla, Vincent Esche |
| [num_enum_derive](https://github.com/illicitonion/num_enum) | 0.7.6 | BSD-3-Clause OR MIT OR Apache-2.0 | Daniel Wagner-Hall, Daniel Henry-Mantilla, Vincent Esche |
| [objc](http://github.com/SSheldon/rust-objc) | 0.2.7 | MIT | Steven Sheldon |
| [objc-foundation](http://github.com/SSheldon/rust-objc-foundation) | 0.1.1 | MIT | Steven Sheldon |
| [objc-sys](https://github.com/madsmtm/objc2) | 0.3.5 | MIT | Mads Marquart |
| [objc2](https://github.com/madsmtm/objc2) | 0.5.3 | MIT | Steven Sheldon, Mads Marquart |
| [objc2](https://github.com/madsmtm/objc2) | 0.6.5 | MIT | Mads Marquart |
| [objc2-app-kit](https://github.com/madsmtm/objc2) | 0.2.2 | MIT | — |
| [objc2-app-kit](https://github.com/madsmtm/objc2) | 0.3.2 | Zlib OR Apache-2.0 OR MIT | — |
| [objc2-cloud-kit](https://github.com/madsmtm/objc2) | 0.2.2 | MIT | — |
| [objc2-cloud-kit](https://github.com/madsmtm/objc2) | 0.3.2 | Zlib OR Apache-2.0 OR MIT | — |
| [objc2-contacts](https://github.com/madsmtm/objc2) | 0.2.2 | MIT | — |
| [objc2-core-bluetooth](https://github.com/madsmtm/objc2) | 0.2.2 | MIT | — |
| [objc2-core-data](https://github.com/madsmtm/objc2) | 0.2.2 | MIT | — |
| [objc2-core-data](https://github.com/madsmtm/objc2) | 0.3.2 | Zlib OR Apache-2.0 OR MIT | — |
| [objc2-core-foundation](https://github.com/madsmtm/objc2) | 0.3.2 | Zlib OR Apache-2.0 OR MIT | — |
| [objc2-core-graphics](https://github.com/madsmtm/objc2) | 0.3.2 | Zlib OR Apache-2.0 OR MIT | — |
| [objc2-core-image](https://github.com/madsmtm/objc2) | 0.2.2 | MIT | — |
| [objc2-core-image](https://github.com/madsmtm/objc2) | 0.3.2 | Zlib OR Apache-2.0 OR MIT | — |
| [objc2-core-location](https://github.com/madsmtm/objc2) | 0.2.2 | MIT | — |
| [objc2-core-location](https://github.com/madsmtm/objc2) | 0.3.2 | Zlib OR Apache-2.0 OR MIT | — |
| [objc2-core-text](https://github.com/madsmtm/objc2) | 0.3.2 | Zlib OR Apache-2.0 OR MIT | — |
| [objc2-encode](https://github.com/madsmtm/objc2) | 4.1.0 | MIT | Mads Marquart |
| [objc2-exception-helper](https://github.com/madsmtm/objc2) | 0.1.1 | Zlib OR Apache-2.0 OR MIT | Mads Marquart |
| [objc2-foundation](https://github.com/madsmtm/objc2) | 0.2.2 | MIT | — |
| [objc2-foundation](https://github.com/madsmtm/objc2) | 0.3.2 | MIT | — |
| [objc2-io-surface](https://github.com/madsmtm/objc2) | 0.3.2 | Zlib OR Apache-2.0 OR MIT | — |
| [objc2-link-presentation](https://github.com/madsmtm/objc2) | 0.2.2 | MIT | — |
| [objc2-metal](https://github.com/madsmtm/objc2) | 0.2.2 | MIT | — |
| [objc2-metal](https://github.com/madsmtm/objc2) | 0.3.2 | Zlib OR Apache-2.0 OR MIT | — |
| [objc2-quartz-core](https://github.com/madsmtm/objc2) | 0.2.2 | MIT | — |
| [objc2-quartz-core](https://github.com/madsmtm/objc2) | 0.3.2 | Zlib OR Apache-2.0 OR MIT | — |
| [objc2-symbols](https://github.com/madsmtm/objc2) | 0.2.2 | MIT | — |
| [objc2-ui-kit](https://github.com/madsmtm/objc2) | 0.2.2 | MIT | — |
| [objc2-ui-kit](https://github.com/madsmtm/objc2) | 0.3.2 | Zlib OR Apache-2.0 OR MIT | — |
| [objc2-uniform-type-identifiers](https://github.com/madsmtm/objc2) | 0.2.2 | MIT | — |
| [objc2-user-notifications](https://github.com/madsmtm/objc2) | 0.2.2 | MIT | — |
| [objc2-user-notifications](https://github.com/madsmtm/objc2) | 0.3.2 | Zlib OR Apache-2.0 OR MIT | — |
| [objc2-web-kit](https://github.com/madsmtm/objc2) | 0.3.2 | Zlib OR Apache-2.0 OR MIT | — |
| [objc_id](http://github.com/SSheldon/rust-objc-id) | 0.1.1 | MIT | Steven Sheldon |
| [once_cell](https://github.com/matklad/once_cell) | 1.21.4 | MIT OR Apache-2.0 | Aleksey Kladov |
| [once_cell_polyfill](https://github.com/polyfill-rs/once_cell_polyfill) | 1.70.2 | MIT OR Apache-2.0 | — |
| [option-ext](https://github.com/soc/option-ext.git) | 0.2.0 | MPL-2.0 | Simon Ochsenreither |
| [orbclient](https://gitlab.redox-os.org/redox-os/orbclient) | 0.3.55 | MIT | Jeremy Soller |
| [ordered-channel](https://gitlab.com/kornelski/ordered-channel) | 1.2.0 | MIT OR Apache-2.0 | — |
| [ordered-float](https://github.com/reem/rust-ordered-float) | 5.5.0 | MIT | Jonathan Reem, Matt Brubeck |
| [ordered-stream](https://github.com/danieldg/ordered-stream) | 0.2.0 | MIT OR Apache-2.0 | Daniel De Graaf, Zeeshan Ali Khan |
| [owned_ttf_parser](https://github.com/alexheretic/owned-ttf-parser) | 0.25.1 | Apache-2.0 | Alex Butler |
| [pango](https://github.com/gtk-rs/gtk-rs-core) | 0.18.3 | MIT | The gtk-rs Project Developers |
| [pango-sys](https://github.com/gtk-rs/gtk-rs-core) | 0.18.0 | MIT | The gtk-rs Project Developers |
| [parking](https://github.com/smol-rs/parking) | 2.2.1 | Apache-2.0 OR MIT | Stjepan Glavina, The Rust Project Developers |
| [parking_lot](https://github.com/Amanieu/parking_lot) | 0.12.5 | MIT OR Apache-2.0 | Amanieu d'Antras |
| [parking_lot_core](https://github.com/Amanieu/parking_lot) | 0.9.12 | MIT OR Apache-2.0 | Amanieu d'Antras |
| [pdf-writer](https://github.com/typst/pdf-writer) | 0.15.0 | MIT OR Apache-2.0 | Laurenz, Martin Haug |
| pdfcraft-cos (git) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-crypt](https://github.com/storytold/pdfcraft) (git) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-filters](https://github.com/storytold/pdfcraft) (git) | 0.9.0 | MIT OR Apache-2.0 | — |
| pdfcraft-fonts (git) | 0.9.0 | MIT OR Apache-2.0 | — |
| pdfcraft-render (git) | 0.9.0 | MIT OR Apache-2.0 | — |
| [peniko](https://github.com/linebender/peniko) | 0.6.1 | Apache-2.0 OR MIT | — |
| [percent-encoding](https://github.com/servo/rust-url/) | 2.3.2 | MIT OR Apache-2.0 | The rust-url developers |
| [phf](https://github.com/rust-phf/rust-phf) | 0.13.1 | MIT | Steven Fackler |
| [phf_codegen](https://github.com/rust-phf/rust-phf) | 0.13.1 | MIT | Steven Fackler |
| [phf_generator](https://github.com/rust-phf/rust-phf) | 0.13.1 | MIT | Steven Fackler |
| [phf_macros](https://github.com/rust-phf/rust-phf) | 0.13.1 | MIT | Steven Fackler |
| [phf_shared](https://github.com/rust-phf/rust-phf) | 0.13.1 | MIT | Steven Fackler |
| [pic-scale](https://github.com/awxkee/pic-scale) | 0.7.12 | BSD-3-Clause OR Apache-2.0 | Radzivon Bartoshyk |
| [pin-project](https://github.com/taiki-e/pin-project) | 1.1.13 | Apache-2.0 OR MIT | — |
| [pin-project-internal](https://github.com/taiki-e/pin-project) | 1.1.13 | Apache-2.0 OR MIT | — |
| [pin-project-lite](https://github.com/taiki-e/pin-project-lite) | 0.2.17 | Apache-2.0 OR MIT | — |
| [piper](https://github.com/smol-rs/piper) | 0.2.5 | MIT OR Apache-2.0 | Stjepan Glavina, John Nunley |
| [pkg-config](https://github.com/rust-lang/pkg-config-rs) | 0.3.34 | MIT OR Apache-2.0 | Alex Crichton |
| [plain](https://github.com/randomites/plain) | 0.2.3 | MIT/Apache-2.0 | jzr |
| [png](https://github.com/image-rs/image-png) | 0.17.16 | MIT OR Apache-2.0 | The image-rs Developers |
| [png](https://github.com/image-rs/image-png) | 0.18.1 | MIT OR Apache-2.0 | The image-rs Developers |
| [polling](https://github.com/smol-rs/polling) | 3.11.0 | Apache-2.0 OR MIT | Stjepan Glavina, John Nunley |
| [pollster](https://github.com/zesterer/pollster) | 0.3.0 | Apache-2.0/MIT | Joshua Barretto |
| [pollster](https://github.com/zesterer/pollster) | 1.0.1 | Apache-2.0/MIT | Joshua Barretto |
| [polycool](https://github.com/linebender/kurbo) | 0.4.0 | MIT OR Apache-2.0 | — |
| [portable-atomic](https://github.com/taiki-e/portable-atomic) | 1.15.0 | Apache-2.0 OR MIT | — |
| [portable-atomic-util](https://github.com/taiki-e/portable-atomic-util) | 0.2.8 | Apache-2.0 OR MIT | — |
| [potential_utf](https://github.com/unicode-org/icu4x) | 0.1.6 | Unicode-3.0 | The ICU4X Project Developers |
| [powerfmt](https://github.com/jhpratt/powerfmt) | 0.2.1 | MIT OR Apache-2.0 | Jacob Pratt |
| [ppv-lite86](https://github.com/cryptocorrosion/cryptocorrosion) | 0.2.21 | MIT OR Apache-2.0 | The CryptoCorrosion Contributors |
| [precomputed-hash](https://github.com/emilio/precomputed-hash) | 0.1.1 | MIT | Emilio Cobos Álvarez |
| [presser](https://github.com/EmbarkStudios/presser) | 0.3.1 | MIT OR Apache-2.0 | Embark, Gray Olson |
| [proc-macro-crate](https://github.com/bkchr/proc-macro-crate) | 1.3.1 | MIT OR Apache-2.0 | Bastian Köcher |
| [proc-macro-crate](https://github.com/bkchr/proc-macro-crate) | 2.0.2 | MIT OR Apache-2.0 | Bastian Köcher |
| [proc-macro-crate](https://github.com/bkchr/proc-macro-crate) | 3.5.0 | MIT OR Apache-2.0 | Bastian Köcher |
| [proc-macro-error](https://gitlab.com/CreepySkeleton/proc-macro-error) | 1.0.4 | MIT OR Apache-2.0 | CreepySkeleton |
| [proc-macro-error-attr](https://gitlab.com/CreepySkeleton/proc-macro-error) | 1.0.4 | MIT OR Apache-2.0 | CreepySkeleton |
| [proc-macro2](https://github.com/dtolnay/proc-macro2) | 1.0.107 | MIT OR Apache-2.0 | David Tolnay, Alex Crichton |
| [profiling](https://github.com/aclysma/profiling) | 1.0.18 | MIT OR Apache-2.0 | Philip Degarmo |
| [pulldown-cmark](https://github.com/raphlinus/pulldown-cmark) | 0.12.2 | MIT | Raph Levien, Marcus Klaas de Vries |
| [pulldown-cmark-escape](https://github.com/raphlinus/pulldown-cmark) | 0.11.0 | MIT | Raph Levien, Marcus Klaas de Vries |
| [pxfm](https://github.com/awxkee/pxfm) | 0.1.30 | BSD-3-Clause OR Apache-2.0 | Radzivon Bartoshyk |
| [quick-error](http://github.com/tailhook/quick-error) | 2.0.1 | MIT/Apache-2.0 | Paul Colomiets, Colin Kiegel |
| [quick-xml](https://github.com/tafia/quick-xml) | 0.41.0 | MIT | — |
| [quote](https://github.com/dtolnay/quote) | 1.0.47 | MIT OR Apache-2.0 | David Tolnay |
| [r-efi](https://github.com/r-efi/r-efi) | 5.3.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | — |
| [r-efi](https://github.com/r-efi/r-efi) | 6.0.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | — |
| [rand](https://github.com/rust-random/rand) | 0.10.3 | MIT OR Apache-2.0 | The Rand Project Developers, The Rust Project Developers |
| [rand](https://github.com/rust-random/rand) | 0.8.8 | MIT OR Apache-2.0 | The Rand Project Developers, The Rust Project Developers |
| [rand_chacha](https://github.com/rust-random/rand) | 0.3.1 | MIT OR Apache-2.0 | The Rand Project Developers, The Rust Project Developers, The CryptoCorrosion Contributors |
| [rand_core](https://github.com/rust-random/rand_core) | 0.10.1 | MIT OR Apache-2.0 | The Rand Project Developers |
| [rand_core](https://github.com/rust-random/rand) | 0.6.4 | MIT OR Apache-2.0 | The Rand Project Developers, The Rust Project Developers |
| [range-alloc](https://github.com/gfx-rs/range-alloc) | 0.1.5 | MIT OR Apache-2.0 | the gfx-rs Developers |
| [rangemap](https://github.com/jeffparsons/rangemap) | 1.8.0 | MIT/Apache-2.0 | Jeff Parsons |
| [raw-window-handle](https://github.com/rust-windowing/raw-window-handle) | 0.6.2 | MIT OR Apache-2.0 OR Zlib | Osspial |
| [raw-window-metal](https://github.com/rust-windowing/raw-window-metal) | 1.1.0 | MIT OR Apache-2.0 | — |
| [rayon](https://github.com/rayon-rs/rayon) | 1.12.0 | MIT OR Apache-2.0 | — |
| [rayon-core](https://github.com/rayon-rs/rayon) | 1.13.0 | MIT OR Apache-2.0 | — |
| [read-fonts](https://github.com/googlefonts/fontations) | 0.39.2 | MIT OR Apache-2.0 | — |
| [read-fonts](https://github.com/googlefonts/fontations) | 0.41.0 | MIT OR Apache-2.0 | — |
| [read-fonts](https://github.com/googlefonts/fontations) | 0.43.3 | MIT OR Apache-2.0 | — |
| [read-fonts](https://github.com/googlefonts/fontations) | 0.44.0 | MIT OR Apache-2.0 | — |
| [redox_syscall](https://gitlab.redox-os.org/redox-os/syscall) | 0.4.1 | MIT | Jeremy Soller |
| [redox_syscall](https://gitlab.redox-os.org/redox-os/syscall) | 0.5.18 | MIT | Jeremy Soller |
| [redox_syscall](https://gitlab.redox-os.org/redox-os/kernel) | 0.9.5 | MIT | Jeremy Soller |
| [redox_users](https://gitlab.redox-os.org/redox-os/users) | 0.5.3 | MIT | Jose Narvaez, Wesley Hershberger |
| [regex](https://github.com/rust-lang/regex) | 1.13.1 | MIT OR Apache-2.0 | The Rust Project Developers, Andrew Gallant |
| [regex-automata](https://github.com/rust-lang/regex) | 0.4.18 | MIT OR Apache-2.0 | The Rust Project Developers, Andrew Gallant |
| [regex-syntax](https://github.com/rust-lang/regex) | 0.8.11 | MIT OR Apache-2.0 | The Rust Project Developers, Andrew Gallant |
| [renderdoc-sys](https://github.com/ebkalderon/renderdoc-rs) | 1.1.0 | MIT OR Apache-2.0 | Eyal Kalderon |
| [rfd](https://github.com/PolyMeilex/rfd) | 0.14.1 | MIT | Poly |
| [ring](https://github.com/briansmith/ring) | 0.17.14 | Apache-2.0 AND ISC | — |
| [rustc-hash](https://github.com/rust-lang-nursery/rustc-hash) | 1.1.0 | Apache-2.0/MIT | The Rust Project Developers |
| [rustc-hash](https://github.com/rust-lang/rustc-hash) | 2.1.3 | Apache-2.0 OR MIT | The Rust Project Developers |
| [rustc_version](https://github.com/djc/rustc-version-rs) | 0.4.1 | MIT OR Apache-2.0 | — |
| [rustix](https://github.com/bytecodealliance/rustix) | 0.38.44 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | Dan Gohman, Jakub Konka |
| [rustix](https://github.com/bytecodealliance/rustix) | 1.1.5 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | Dan Gohman, Jakub Konka |
| [rustls](https://github.com/rustls/rustls) | 0.23.46 | Apache-2.0 OR ISC OR MIT | — |
| [rustls-pki-types](https://github.com/rustls/pki-types) | 1.15.1 | MIT OR Apache-2.0 | — |
| [rustls-webpki](https://github.com/rustls/webpki) | 0.103.15 | ISC | — |
| [rustversion](https://github.com/dtolnay/rustversion) | 1.0.23 | MIT OR Apache-2.0 | David Tolnay |
| [rustybuzz](https://github.com/harfbuzz/rustybuzz) | 0.20.1 | MIT | Caleb Maclennan, Laurenz Stampfl, Yevhenii Reizner, خالد حسني (Khaled Hosny) |
| [ryu](https://github.com/dtolnay/ryu) | 1.0.23 | Apache-2.0 OR BSL-1.0 | David Tolnay |
| [same-file](https://github.com/BurntSushi/same-file) | 1.0.6 | Unlicense/MIT | Andrew Gallant |
| [scoped-tls](https://github.com/alexcrichton/scoped-tls) | 1.0.1 | MIT/Apache-2.0 | Alex Crichton |
| [scopeguard](https://github.com/bluss/scopeguard) | 1.2.0 | MIT OR Apache-2.0 | bluss |
| [sctk-adwaita](https://github.com/PolyMeilex/sctk-adwaita) | 0.10.1 | MIT | Poly |
| [selectors](https://github.com/servo/stylo) | 0.38.0 | MPL-2.0 | The Servo Project Developers |
| [self_cell](https://github.com/Voultapher/self_cell) | 1.3.0 | Apache-2.0 OR GPL-2.0-only | Lukas Bergdoll |
| [semver](https://github.com/dtolnay/semver) | 1.0.28 | MIT OR Apache-2.0 | David Tolnay |
| [serde](https://github.com/serde-rs/serde) | 1.0.229 | MIT OR Apache-2.0 | Erick Tryzelaar, David Tolnay |
| [serde-xml-rs](https://github.com/RReverser/serde-xml-rs) | 0.8.2 | MIT | Ingvar Stepanyan, William Bartlett |
| [serde_core](https://github.com/serde-rs/serde) | 1.0.229 | MIT OR Apache-2.0 | Erick Tryzelaar, David Tolnay |
| [serde_derive](https://github.com/serde-rs/serde) | 1.0.229 | MIT OR Apache-2.0 | Erick Tryzelaar, David Tolnay |
| [serde_json](https://github.com/serde-rs/json) | 1.0.154 | MIT OR Apache-2.0 | — |
| [serde_repr](https://github.com/dtolnay/serde-repr) | 0.1.21 | MIT OR Apache-2.0 | David Tolnay |
| [serde_spanned](https://github.com/toml-rs/toml) | 0.6.9 | MIT OR Apache-2.0 | — |
| [servo_arc](https://github.com/servo/stylo) | 0.4.3 | MIT OR Apache-2.0 | The Servo Project Developers |
| [sha1](https://github.com/RustCrypto/hashes) | 0.10.7 | MIT OR Apache-2.0 | RustCrypto Developers |
| [sha1](https://github.com/RustCrypto/hashes) | 0.11.0 | MIT OR Apache-2.0 | RustCrypto Developers |
| [sha2](https://github.com/RustCrypto/hashes) | 0.10.9 | MIT OR Apache-2.0 | RustCrypto Developers |
| [sha2](https://github.com/RustCrypto/hashes) | 0.11.1 | MIT OR Apache-2.0 | RustCrypto Developers |
| [sharded-slab](https://github.com/hawkw/sharded-slab) | 0.1.7 | MIT | Eliza Weisman |
| [shlex](https://github.com/comex/rust-shlex) | 2.0.1 | MIT OR Apache-2.0 | comex, Fenhl, Adrian Taylor, Alex Touchet, Daniel Parks, Garrett Berg |
| [signal-hook-registry](https://github.com/vorner/signal-hook) | 1.4.8 | MIT OR Apache-2.0 | Michal 'vorner' Vaner, Masaki Hara |
| [simd-adler32](https://github.com/mcountryman/simd-adler32) | 0.3.10 | MIT | Marvin Countryman |
| [simd_cesu8](https://github.com/seancroach/simd_cesu8) | 1.2.0 | Apache-2.0 OR MIT | Sean C. Roach |
| [simdutf8](https://github.com/rusticstuff/simdutf8) | 0.1.5 | MIT OR Apache-2.0 | Hans Kratz |
| [siphasher](https://github.com/jedisct1/rust-siphash) | 1.0.4 | MIT OR Apache-2.0 | Frank Denis |
| [skrifa](https://github.com/googlefonts/fontations) | 0.42.1 | MIT OR Apache-2.0 | — |
| [skrifa](https://github.com/googlefonts/fontations) | 0.44.0 | MIT OR Apache-2.0 | — |
| [skrifa](https://github.com/googlefonts/fontations) | 0.47.0 | MIT OR Apache-2.0 | — |
| [slab](https://github.com/tokio-rs/slab) | 0.4.12 | MIT | Carl Lerche |
| [slotmap](https://github.com/orlp/slotmap) | 1.1.1 | Zlib | Orson Peters |
| [smallvec](https://github.com/servo/rust-smallvec) | 1.16.3 | MIT OR Apache-2.0 | The Servo Project Developers |
| [smithay-client-toolkit](https://github.com/smithay/client-toolkit) | 0.19.2 | MIT | Elinor Berger, i509VCB, Ashley Wulber |
| [smithay-client-toolkit](https://github.com/smithay/client-toolkit) | 0.20.0 | MIT | Elinor Berger, i509VCB, Ashley Wulber |
| [smithay-clipboard](https://github.com/smithay/smithay-clipboard) | 0.7.3 | MIT | Kirill Chibisov, Elinor Berger |
| [smol_str](https://github.com/rust-analyzer/smol_str) | 0.2.2 | MIT OR Apache-2.0 | Aleksey Kladov |
| [socket2](https://github.com/rust-lang/socket2) | 0.6.5 | MIT OR Apache-2.0 | Alex Crichton, Thomas de Zeeuw |
| [soup3](https://gitlab.gnome.org/World/Rust/soup3-rs) | 0.5.0 | MIT | — |
| [soup3-sys](https://gitlab.gnome.org/World/Rust/soup3-rs) | 0.5.0 | MIT | The Gtk-rs Project Developers |
| [spirv](https://github.com/gfx-rs/rspirv) | 0.4.0+sdk-1.4.341.0 | Apache-2.0 | Lei Zhang |
| [stable_deref_trait](https://github.com/storyyeller/stable_deref_trait) | 1.2.1 | MIT OR Apache-2.0 | Robert Grosse |
| [static_assertions](https://github.com/nvzqz/static-assertions-rs) | 1.1.0 | MIT OR Apache-2.0 | Nikolai Vazquez |
| [strict-num](https://github.com/RazrFalcon/strict-num) | 0.1.1 | MIT | Yevhenii Reizner |
| [string_cache](https://github.com/servo/string-cache) | 0.9.0 | MIT OR Apache-2.0 | The Servo Project Developers |
| [string_cache_codegen](https://github.com/servo/string-cache) | 0.6.1 | MIT OR Apache-2.0 | The Servo Project Developers |
| [stringprep](https://github.com/sfackler/rust-stringprep) | 0.1.5 | MIT/Apache-2.0 | Steven Fackler |
| [strsim](https://github.com/rapidfuzz/strsim-rs) | 0.11.1 | MIT | Danny Guo, maxbachmann |
| [subsetter](https://github.com/typst/subsetter) | 0.2.6 | MIT OR Apache-2.0 | Laurenz Mädje, Laurenz Stampfl |
| [subtle](https://github.com/dalek-cryptography/subtle) | 2.6.1 | BSD-3-Clause | Isis Lovecruft, Henry de Valence |
| [syn](https://github.com/dtolnay/syn) | 1.0.109 | MIT OR Apache-2.0 | David Tolnay |
| [syn](https://github.com/dtolnay/syn) | 2.0.119 | MIT OR Apache-2.0 | David Tolnay |
| [syn](https://github.com/dtolnay/syn) | 3.0.9 | MIT OR Apache-2.0 | David Tolnay |
| [synstructure](https://github.com/mystor/synstructure) | 0.14.0 | MIT | Nika Layzell |
| [system-deps](https://github.com/gdesmott/system-deps) | 6.2.2 | MIT OR Apache-2.0 | Guillaume Desmottes, Josh Triplett |
| [tao](https://github.com/tauri-apps/tao) | 0.37.1 | Apache-2.0 | Tauri Programme within The Commons Conservancy, The winit contributors |
| [tao-macros](https://github.com/tauri-apps/tao) | 0.1.4 | MIT OR Apache-2.0 | Tauri Programme within The Commons Conservancy |
| [target-lexicon](https://github.com/bytecodealliance/target-lexicon) | 0.12.16 | Apache-2.0 WITH LLVM-exception | Dan Gohman |
| [tempfile](https://github.com/Stebalien/tempfile) | 3.27.0 | MIT OR Apache-2.0 | Steven Allen, The Rust Project Developers, Ashley Mannix, Jason White |
| [tendril](https://github.com/servo/html5ever) | 0.5.1 | MIT OR Apache-2.0 | Keegan McAllister, Simon Sapin, Chris Morgan |
| [termcolor](https://github.com/BurntSushi/termcolor) | 1.4.1 | Unlicense OR MIT | Andrew Gallant |
| [thiserror](https://github.com/dtolnay/thiserror) | 1.0.69 | MIT OR Apache-2.0 | David Tolnay |
| [thiserror](https://github.com/dtolnay/thiserror) | 2.0.21 | MIT OR Apache-2.0 | David Tolnay |
| [thiserror-impl](https://github.com/dtolnay/thiserror) | 1.0.69 | MIT OR Apache-2.0 | David Tolnay |
| [thiserror-impl](https://github.com/dtolnay/thiserror) | 2.0.21 | MIT OR Apache-2.0 | David Tolnay |
| [thread_local](https://github.com/Amanieu/thread_local-rs) | 1.1.10 | MIT OR Apache-2.0 | Amanieu d'Antras |
| [tiff](https://github.com/image-rs/image-tiff) | 0.11.3 | MIT | The image-rs Developers |
| [time](https://github.com/time-rs/time) | 0.3.55 | MIT OR Apache-2.0 | Jacob Pratt, Time contributors |
| [time-core](https://github.com/time-rs/time) | 0.1.9 | MIT OR Apache-2.0 | Jacob Pratt, Time contributors |
| [time-macros](https://github.com/time-rs/time) | 0.2.32 | MIT OR Apache-2.0 | Jacob Pratt, Time contributors |
| [tiny-skia](https://github.com/RazrFalcon/tiny-skia) | 0.11.4 | BSD-3-Clause | Yevhenii Reizner |
| [tiny-skia-path](https://github.com/RazrFalcon/tiny-skia/tree/master/path) | 0.11.4 | BSD-3-Clause | Yevhenii Reizner |
| [tiny-skia-path](https://github.com/linebender/tiny-skia/tree/master/path) | 0.12.0 | BSD-3-Clause | Yevhenii Reizner |
| [tinystr](https://github.com/unicode-org/icu4x) | 0.8.4 | Unicode-3.0 | The ICU4X Project Developers |
| [tinyvec](https://github.com/Lokathor/tinyvec) | 1.13.3 | Zlib OR Apache-2.0 OR MIT | Lokathor |
| [tokio](https://github.com/tokio-rs/tokio) | 1.53.3 | MIT | Tokio Contributors |
| [tokio-macros](https://github.com/tokio-rs/tokio) | 2.7.2 | MIT | Tokio Contributors |
| [tokio-stream](https://github.com/tokio-rs/tokio) | 0.1.19 | MIT | Tokio Contributors |
| [tokio-util](https://github.com/tokio-rs/tokio) | 0.7.20 | MIT | Tokio Contributors |
| [toml](https://github.com/toml-rs/toml) | 0.8.2 | MIT OR Apache-2.0 | Alex Crichton |
| [toml_datetime](https://github.com/toml-rs/toml) | 0.6.3 | MIT OR Apache-2.0 | Alex Crichton |
| [toml_datetime](https://github.com/toml-rs/toml) | 1.1.2+spec-1.1.0 | MIT OR Apache-2.0 | — |
| [toml_edit](https://github.com/toml-rs/toml) | 0.19.15 | MIT OR Apache-2.0 | Andronik Ordian, Ed Page |
| [toml_edit](https://github.com/toml-rs/toml) | 0.20.2 | MIT OR Apache-2.0 | Andronik Ordian, Ed Page |
| [toml_edit](https://github.com/toml-rs/toml) | 0.25.17+spec-1.1.0 | MIT OR Apache-2.0 | — |
| [toml_parser](https://github.com/toml-rs/toml) | 1.1.5+spec-1.1.0 | MIT OR Apache-2.0 | — |
| [tracing](https://github.com/tokio-rs/tracing) | 0.1.44 | MIT | Eliza Weisman, Tokio Contributors |
| [tracing-attributes](https://github.com/tokio-rs/tracing) | 0.1.31 | MIT | Tokio Contributors, Eliza Weisman, David Barsky |
| [tracing-core](https://github.com/tokio-rs/tracing) | 0.1.36 | MIT | Tokio Contributors |
| [tracing-log](https://github.com/tokio-rs/tracing) | 0.2.0 | MIT | Tokio Contributors |
| [tracing-subscriber](https://github.com/tokio-rs/tracing) | 0.3.23 | MIT | Eliza Weisman, David Barsky, Tokio Contributors |
| [ttf-parser](https://github.com/harfbuzz/ttf-parser) | 0.25.1 | MIT OR Apache-2.0 | Caleb Maclennan, Laurenz Stampfl, Yevhenii Reizner, خالد حسني (Khaled Hosny) |
| [type-map](https://github.com/kardeiz/type-map) | 0.5.1 | MIT/Apache-2.0 | Jacob Brown |
| [typenum](https://github.com/paholg/typenum) | 1.20.1 | MIT OR Apache-2.0 | — |
| [uds_windows](https://github.com/haraldh/rust_uds_windows) | 1.2.1 | MIT | Azure IoT Edge Devs, Harald Hoyer |
| [unicase](https://github.com/seanmonstar/unicase) | 2.10.0 | MIT OR Apache-2.0 | Sean McArthur |
| [unicode-bidi](https://github.com/servo/unicode-bidi) | 0.3.18 | MIT OR Apache-2.0 | The Servo Project Developers |
| [unicode-bidi-mirroring](https://github.com/RazrFalcon/unicode-bidi-mirroring) | 0.4.0 | MIT/Apache-2.0 | Yevhenii Reizner |
| [unicode-ccc](https://github.com/RazrFalcon/unicode-ccc) | 0.4.0 | MIT/Apache-2.0 | Yevhenii Reizner |
| [unicode-general-category](https://github.com/yeslogic/unicode-general-category) | 1.1.0 | Apache-2.0 | YesLogic Pty. Ltd. |
| [unicode-ident](https://github.com/dtolnay/unicode-ident) | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 | David Tolnay |
| [unicode-linebreak](https://github.com/axelf4/unicode-linebreak) | 0.1.5 | Apache-2.0 | Axel Forsman |
| [unicode-normalization](https://github.com/unicode-rs/unicode-normalization) | 0.1.26 | (MIT OR Apache-2.0) AND Unicode-3.0 | kwantam, Manish Goregaokar |
| [unicode-properties](https://github.com/unicode-rs/unicode-properties) | 0.1.4 | MIT/Apache-2.0 | Charles Lew, Manish Goregaokar |
| [unicode-script](https://github.com/unicode-rs/unicode-script) | 0.5.8 | MIT OR Apache-2.0 | Manish Goregaokar |
| [unicode-segmentation](https://github.com/unicode-rs/unicode-segmentation) | 1.13.3 | MIT OR Apache-2.0 | kwantam, Manish Goregaokar |
| [unicode-width](https://github.com/unicode-rs/unicode-width) | 0.1.14 | MIT OR Apache-2.0 | kwantam, Manish Goregaokar |
| [untrusted](https://github.com/briansmith/untrusted) | 0.9.0 | ISC | Brian Smith |
| [ureq](https://github.com/algesten/ureq) | 2.12.1 | MIT OR Apache-2.0 | Martin Algesten, Jacob Hoffman-Andrews |
| [url](https://github.com/servo/rust-url) | 2.5.8 | MIT OR Apache-2.0 | The rust-url developers |
| [urlencoding](https://github.com/kornelski/rust_urlencoding) | 2.1.3 | MIT | Kornel, Bertram Truong |
| [utf8_iter](https://github.com/hsivonen/utf8_iter) | 1.0.4 | Apache-2.0 OR MIT | Henri Sivonen |
| [utf8parse](https://github.com/alacritty/vte) | 0.2.2 | Apache-2.0 OR MIT | Joe Wilm, Christian Duerr |
| [uuid](https://github.com/uuid-rs/uuid) | 1.28.0 | Apache-2.0 OR MIT | Ashley Mannix&lt;ashleymannix@live.com.au&gt;, Dylan DPC&lt;dylan.dpc@gmail.com&gt;, Hunar Roop Kahlon&lt;hunar.roop@gmail.com&gt; |
| [valuable](https://github.com/tokio-rs/valuable) | 0.1.1 | MIT | — |
| [vello_common](https://github.com/linebender/vello) (git) | 0.0.8 | Apache-2.0 OR MIT | — |
| [vello_common](https://github.com/linebender/vello) | 0.0.9 | Apache-2.0 OR MIT | — |
| [vello_common](https://github.com/linebender/vello) | 0.1.0 | Apache-2.0 OR MIT | — |
| [vello_common](https://github.com/linebender/vello) | 0.2.0 | Apache-2.0 OR MIT | — |
| [vello_cpu](https://github.com/linebender/vello) | 0.0.8 | Apache-2.0 OR MIT | — |
| [vello_cpu](https://github.com/linebender/vello) | 0.1.0 | Apache-2.0 OR MIT | — |
| [vello_cpu](https://github.com/linebender/vello) | 0.2.0 | Apache-2.0 OR MIT | — |
| [version-compare](https://gitlab.com/timvisee/version-compare) | 0.2.1 | MIT | Tim Visee |
| [version_check](https://github.com/SergioBenitez/version_check) | 0.9.5 | MIT/Apache-2.0 | Sergio Benitez |
| [walkdir](https://github.com/BurntSushi/walkdir) | 2.5.0 | Unlicense/MIT | Andrew Gallant |
| [wasi](https://github.com/bytecodealliance/wasi) | 0.11.1+wasi-snapshot-preview1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | The Cranelift Project Developers |
| [wasip2](https://github.com/bytecodealliance/wasi-rs) | 1.0.4+wasi-0.2.12 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | — |
| [wasm-bindgen](https://github.com/wasm-bindgen/wasm-bindgen) | 0.2.129 | MIT OR Apache-2.0 | The wasm-bindgen Developers |
| [wasm-bindgen-futures](https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/futures) | 0.4.79 | MIT OR Apache-2.0 | The wasm-bindgen Developers |
| [wasm-bindgen-macro](https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/macro) | 0.2.129 | MIT OR Apache-2.0 | The wasm-bindgen Developers |
| [wasm-bindgen-macro-support](https://github.com/wasm-bindgen/wasm-bindgen/tree/main/crates/macro-support) | 0.2.129 | MIT OR Apache-2.0 | The wasm-bindgen Developers |
| [wasm-bindgen-shared](https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/shared) | 0.2.129 | MIT OR Apache-2.0 | The wasm-bindgen Developers |
| [wayland-backend](https://github.com/smithay/wayland-rs) | 0.3.17 | MIT | Elinor Berger |
| [wayland-client](https://github.com/smithay/wayland-rs) | 0.31.15 | MIT | Elinor Berger |
| [wayland-csd-frame](https://github.com/rust-windowing/wayland-csd-frame) | 0.3.0 | MIT | Kirill Chibisov |
| [wayland-cursor](https://github.com/smithay/wayland-rs) | 0.31.14 | MIT | Elinor Berger |
| [wayland-protocols](https://github.com/smithay/wayland-rs) | 0.32.13 | MIT | Elinor Berger |
| [wayland-protocols-experimental](https://github.com/smithay/wayland-rs) | 20250721.0.1 | MIT | Elinor Berger |
| [wayland-protocols-misc](https://github.com/smithay/wayland-rs) | 0.3.12 | MIT | Elinor Berger |
| [wayland-protocols-plasma](https://github.com/smithay/wayland-rs) | 0.3.12 | MIT | Elinor Berger |
| [wayland-protocols-wlr](https://github.com/smithay/wayland-rs) | 0.3.12 | MIT | Elinor Berger |
| [wayland-scanner](https://github.com/smithay/wayland-rs) | 0.31.11 | MIT | Elinor Berger |
| [wayland-sys](https://github.com/smithay/wayland-rs) | 0.31.11 | MIT | Elinor Berger |
| [web-sys](https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/web-sys) | 0.3.106 | MIT OR Apache-2.0 | The wasm-bindgen Developers |
| [web-time](https://github.com/daxpedda/web-time) | 1.1.0 | MIT OR Apache-2.0 | — |
| [web_atoms](https://github.com/servo/html5ever) | 0.2.6 | MIT OR Apache-2.0 | The html5ever Project Developers |
| [webbrowser](https://github.com/amodm/webbrowser-rs) | 1.2.4 | MIT OR Apache-2.0 | Amod Malviya @amodm |
| [webkit2gtk](https://github.com/tauri-apps/webkit2gtk-rs) | 2.0.2 | MIT | — |
| [webkit2gtk-sys](https://github.com/tauri-apps/webkit2gtk-rs) | 2.0.2 | MIT | — |
| [webpki-roots](https://github.com/rustls/webpki-roots) | 0.26.11 | CDLA-Permissive-2.0 | — |
| [webpki-roots](https://github.com/rustls/webpki-roots) | 1.0.9 | CDLA-Permissive-2.0 | — |
| [webview2-com](https://github.com/wravery/webview2-rs) | 0.39.1 | MIT | — |
| [webview2-com-macros](https://github.com/wravery/webview2-rs) | 0.8.1 | MIT | — |
| [webview2-com-sys](https://github.com/wravery/webview2-rs) | 0.39.1 | MIT | — |
| [weezl](https://github.com/image-rs/weezl) | 0.1.12 | MIT OR Apache-2.0 | The image-rs Developers |
| [weezl](https://github.com/image-rs/weezl) | 0.2.1 | MIT OR Apache-2.0 | The image-rs Developers |
| [wgpu](https://github.com/gfx-rs/wgpu) | 30.0.1 | MIT OR Apache-2.0 | gfx-rs developers |
| [wgpu-core](https://github.com/gfx-rs/wgpu) | 30.0.1 | MIT OR Apache-2.0 | gfx-rs developers |
| [wgpu-core-deps-apple](https://github.com/gfx-rs/wgpu) | 30.0.1 | MIT OR Apache-2.0 | gfx-rs developers |
| [wgpu-core-deps-emscripten](https://github.com/gfx-rs/wgpu) | 30.0.1 | MIT OR Apache-2.0 | gfx-rs developers |
| [wgpu-core-deps-wasm](https://github.com/gfx-rs/wgpu) | 30.0.1 | MIT OR Apache-2.0 | gfx-rs developers |
| [wgpu-core-deps-windows-linux-android](https://github.com/gfx-rs/wgpu) | 30.0.1 | MIT OR Apache-2.0 | gfx-rs developers |
| [wgpu-hal](https://github.com/gfx-rs/wgpu) | 30.0.1 | MIT OR Apache-2.0 | gfx-rs developers |
| [wgpu-naga-bridge](https://github.com/gfx-rs/wgpu) | 30.0.1 | MIT OR Apache-2.0 | gfx-rs developers |
| [wgpu-types](https://github.com/gfx-rs/wgpu) | 30.0.1 | MIT OR Apache-2.0 | gfx-rs developers |
| [winapi](https://github.com/retep998/winapi-rs) | 0.3.9 | MIT/Apache-2.0 | Peter Atashian |
| [winapi-i686-pc-windows-gnu](https://github.com/retep998/winapi-rs) | 0.4.0 | MIT/Apache-2.0 | Peter Atashian |
| [winapi-util](https://github.com/BurntSushi/winapi-util) | 0.1.11 | Unlicense OR MIT | Andrew Gallant |
| [winapi-x86_64-pc-windows-gnu](https://github.com/retep998/winapi-rs) | 0.4.0 | MIT/Apache-2.0 | Peter Atashian |
| [windows](https://github.com/microsoft/windows-rs) | 0.61.3 | MIT OR Apache-2.0 | Microsoft |
| [windows](https://github.com/microsoft/windows-rs) | 0.62.2 | MIT OR Apache-2.0 | — |
| [windows-collections](https://github.com/microsoft/windows-rs) | 0.2.0 | MIT OR Apache-2.0 | — |
| [windows-collections](https://github.com/microsoft/windows-rs) | 0.3.2 | MIT OR Apache-2.0 | — |
| [windows-core](https://github.com/microsoft/windows-rs) | 0.61.2 | MIT OR Apache-2.0 | Microsoft |
| [windows-core](https://github.com/microsoft/windows-rs) | 0.62.2 | MIT OR Apache-2.0 | — |
| [windows-future](https://github.com/microsoft/windows-rs) | 0.2.1 | MIT OR Apache-2.0 | — |
| [windows-future](https://github.com/microsoft/windows-rs) | 0.3.2 | MIT OR Apache-2.0 | — |
| [windows-implement](https://github.com/microsoft/windows-rs) | 0.60.2 | MIT OR Apache-2.0 | — |
| [windows-interface](https://github.com/microsoft/windows-rs) | 0.59.3 | MIT OR Apache-2.0 | — |
| [windows-link](https://github.com/microsoft/windows-rs) | 0.1.3 | MIT OR Apache-2.0 | Microsoft |
| [windows-link](https://github.com/microsoft/windows-rs) | 0.2.1 | MIT OR Apache-2.0 | — |
| [windows-numerics](https://github.com/microsoft/windows-rs) | 0.2.0 | MIT OR Apache-2.0 | — |
| [windows-numerics](https://github.com/microsoft/windows-rs) | 0.3.1 | MIT OR Apache-2.0 | — |
| [windows-result](https://github.com/microsoft/windows-rs) | 0.3.4 | MIT OR Apache-2.0 | Microsoft |
| [windows-result](https://github.com/microsoft/windows-rs) | 0.4.1 | MIT OR Apache-2.0 | — |
| [windows-strings](https://github.com/microsoft/windows-rs) | 0.4.2 | MIT OR Apache-2.0 | Microsoft |
| [windows-strings](https://github.com/microsoft/windows-rs) | 0.5.1 | MIT OR Apache-2.0 | — |
| [windows-sys](https://github.com/microsoft/windows-rs) | 0.45.0 | MIT OR Apache-2.0 | Microsoft |
| [windows-sys](https://github.com/microsoft/windows-rs) | 0.48.0 | MIT OR Apache-2.0 | Microsoft |
| [windows-sys](https://github.com/microsoft/windows-rs) | 0.52.0 | MIT OR Apache-2.0 | Microsoft |
| [windows-sys](https://github.com/microsoft/windows-rs) | 0.59.0 | MIT OR Apache-2.0 | Microsoft |
| [windows-sys](https://github.com/microsoft/windows-rs) | 0.60.2 | MIT OR Apache-2.0 | Microsoft |
| [windows-sys](https://github.com/microsoft/windows-rs) | 0.61.2 | MIT OR Apache-2.0 | — |
| [windows-targets](https://github.com/microsoft/windows-rs) | 0.42.2 | MIT OR Apache-2.0 | Microsoft |
| [windows-targets](https://github.com/microsoft/windows-rs) | 0.48.5 | MIT OR Apache-2.0 | Microsoft |
| [windows-targets](https://github.com/microsoft/windows-rs) | 0.52.6 | MIT OR Apache-2.0 | Microsoft |
| [windows-targets](https://github.com/microsoft/windows-rs) | 0.53.5 | MIT OR Apache-2.0 | — |
| [windows-threading](https://github.com/microsoft/windows-rs) | 0.1.0 | MIT OR Apache-2.0 | Microsoft |
| [windows-threading](https://github.com/microsoft/windows-rs) | 0.2.1 | MIT OR Apache-2.0 | — |
| [windows-version](https://github.com/microsoft/windows-rs) | 0.1.7 | MIT OR Apache-2.0 | — |
| [windows_aarch64_gnullvm](https://github.com/microsoft/windows-rs) | 0.42.2 | MIT OR Apache-2.0 | Microsoft |
| [windows_aarch64_gnullvm](https://github.com/microsoft/windows-rs) | 0.48.5 | MIT OR Apache-2.0 | Microsoft |
| [windows_aarch64_gnullvm](https://github.com/microsoft/windows-rs) | 0.52.6 | MIT OR Apache-2.0 | Microsoft |
| [windows_aarch64_gnullvm](https://github.com/microsoft/windows-rs) | 0.53.1 | MIT OR Apache-2.0 | — |
| [windows_aarch64_msvc](https://github.com/microsoft/windows-rs) | 0.42.2 | MIT OR Apache-2.0 | Microsoft |
| [windows_aarch64_msvc](https://github.com/microsoft/windows-rs) | 0.48.5 | MIT OR Apache-2.0 | Microsoft |
| [windows_aarch64_msvc](https://github.com/microsoft/windows-rs) | 0.52.6 | MIT OR Apache-2.0 | Microsoft |
| [windows_aarch64_msvc](https://github.com/microsoft/windows-rs) | 0.53.1 | MIT OR Apache-2.0 | — |
| [windows_i686_gnu](https://github.com/microsoft/windows-rs) | 0.42.2 | MIT OR Apache-2.0 | Microsoft |
| [windows_i686_gnu](https://github.com/microsoft/windows-rs) | 0.48.5 | MIT OR Apache-2.0 | Microsoft |
| [windows_i686_gnu](https://github.com/microsoft/windows-rs) | 0.52.6 | MIT OR Apache-2.0 | Microsoft |
| [windows_i686_gnu](https://github.com/microsoft/windows-rs) | 0.53.1 | MIT OR Apache-2.0 | — |
| [windows_i686_gnullvm](https://github.com/microsoft/windows-rs) | 0.52.6 | MIT OR Apache-2.0 | Microsoft |
| [windows_i686_gnullvm](https://github.com/microsoft/windows-rs) | 0.53.1 | MIT OR Apache-2.0 | — |
| [windows_i686_msvc](https://github.com/microsoft/windows-rs) | 0.42.2 | MIT OR Apache-2.0 | Microsoft |
| [windows_i686_msvc](https://github.com/microsoft/windows-rs) | 0.48.5 | MIT OR Apache-2.0 | Microsoft |
| [windows_i686_msvc](https://github.com/microsoft/windows-rs) | 0.52.6 | MIT OR Apache-2.0 | Microsoft |
| [windows_i686_msvc](https://github.com/microsoft/windows-rs) | 0.53.1 | MIT OR Apache-2.0 | — |
| [windows_x86_64_gnu](https://github.com/microsoft/windows-rs) | 0.42.2 | MIT OR Apache-2.0 | Microsoft |
| [windows_x86_64_gnu](https://github.com/microsoft/windows-rs) | 0.48.5 | MIT OR Apache-2.0 | Microsoft |
| [windows_x86_64_gnu](https://github.com/microsoft/windows-rs) | 0.52.6 | MIT OR Apache-2.0 | Microsoft |
| [windows_x86_64_gnu](https://github.com/microsoft/windows-rs) | 0.53.1 | MIT OR Apache-2.0 | — |
| [windows_x86_64_gnullvm](https://github.com/microsoft/windows-rs) | 0.42.2 | MIT OR Apache-2.0 | Microsoft |
| [windows_x86_64_gnullvm](https://github.com/microsoft/windows-rs) | 0.48.5 | MIT OR Apache-2.0 | Microsoft |
| [windows_x86_64_gnullvm](https://github.com/microsoft/windows-rs) | 0.52.6 | MIT OR Apache-2.0 | Microsoft |
| [windows_x86_64_gnullvm](https://github.com/microsoft/windows-rs) | 0.53.1 | MIT OR Apache-2.0 | — |
| [windows_x86_64_msvc](https://github.com/microsoft/windows-rs) | 0.42.2 | MIT OR Apache-2.0 | Microsoft |
| [windows_x86_64_msvc](https://github.com/microsoft/windows-rs) | 0.48.5 | MIT OR Apache-2.0 | Microsoft |
| [windows_x86_64_msvc](https://github.com/microsoft/windows-rs) | 0.52.6 | MIT OR Apache-2.0 | Microsoft |
| [windows_x86_64_msvc](https://github.com/microsoft/windows-rs) | 0.53.1 | MIT OR Apache-2.0 | — |
| [winit](https://github.com/rust-windowing/winit) | 0.30.13 | Apache-2.0 | The winit contributors, Pierre Krieger |
| [winnow](https://github.com/winnow-rs/winnow) | 0.5.40 | MIT | — |
| [winnow](https://github.com/winnow-rs/winnow) | 1.0.4 | MIT | — |
| [wit-bindgen](https://github.com/bytecodealliance/wit-bindgen) | 0.57.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | Alex Crichton |
| wordcraft-doc (git) | 0.8.0 | MIT OR Apache-2.0 | — |
| wordcraft-docbin (git) | 0.8.0 | MIT OR Apache-2.0 | — |
| wordcraft-docx (git) | 0.8.0 | MIT OR Apache-2.0 | — |
| wordcraft-engine (git) | 0.8.0 | MIT OR Apache-2.0 | — |
| wordcraft-fonts (git) | 0.8.0 | MIT OR Apache-2.0 | — |
| wordcraft-formats (git) | 0.8.0 | MIT OR Apache-2.0 | — |
| wordcraft-geom (git) | 0.8.0 | MIT OR Apache-2.0 | — |
| wordcraft-layout (git) | 0.8.0 | MIT OR Apache-2.0 | — |
| wordcraft-metafile (git) | 0.8.0 | MIT OR Apache-2.0 | — |
| wordcraft-pdf (git) | 0.8.0 | MIT OR Apache-2.0 | — |
| wordcraft-proof (git) | 0.8.0 | MIT OR Apache-2.0 | — |
| wordcraft-render (git) | 0.8.0 | MIT OR Apache-2.0 | — |
| [write-fonts](https://github.com/googlefonts/fontations) | 0.48.1 | MIT OR Apache-2.0 | — |
| [writeable](https://github.com/unicode-org/icu4x) | 0.6.4 | Unicode-3.0 | The ICU4X Project Developers |
| [wry](https://github.com/tauri-apps/wry) | 0.57.0 | Apache-2.0 OR MIT | Tauri Programme within The Commons Conservancy |
| [wuff](https://github.com/nicoburns/wuff) | 0.2.9 | MIT | Nico Burns |
| [x11](https://github.com/AltF02/x11-rs.git) | 2.21.0 | MIT | daggerbot, Erle Pereira, AltF02 |
| [x11-dl](https://github.com/AltF02/x11-rs.git) | 2.21.0 | MIT | daggerbot, Erle Pereira, AltF02 |
| [x11rb](https://github.com/psychon/x11rb) | 0.13.2 | MIT OR Apache-2.0 | Uli Schlachter, Eduardo Sánchez Muñoz, notgull |
| [x11rb-protocol](https://github.com/psychon/x11rb) | 0.13.2 | MIT OR Apache-2.0 | Uli Schlachter, Eduardo Sánchez Muñoz, notgull |
| [xcursor](https://github.com/esposm03/xcursor-rs) | 0.3.11 | MIT | Samuele Esposito |
| [xdg-home](https://github.com/zeenix/xdg-home) | 1.3.0 | MIT | Zeeshan Ali Khan |
| [xkbcommon-dl](https://github.com/rust-windowing/xkbcommon-dl) | 0.4.2 | MIT | Francesca Frangipane |
| [xkeysym](https://github.com/notgull/xkeysym) | 0.2.1 | MIT OR Apache-2.0 OR Zlib | John Nunley |
| [xml](https://github.com/kornelski/xml-rs) | 1.4.0 | MIT | Vladimir Matveev, Kornel (https://github.com/kornelski) |
| [xml-rs](https://github.com/kornelski/xml-rs) | 0.8.29 | MIT | Vladimir Matveev |
| [xmp-writer](https://github.com/typst/xmp-writer) | 0.3.3 | MIT OR Apache-2.0 | Martin Haug |
| [yoke](https://github.com/unicode-org/icu4x) | 0.8.3 | Unicode-3.0 | Manish Goregaokar |
| [yoke-derive](https://github.com/unicode-org/icu4x) | 0.8.4 | Unicode-3.0 | Manish Goregaokar |
| [zbus](https://github.com/dbus2/zbus/) | 4.4.0 | MIT | Zeeshan Ali Khan |
| [zbus](https://github.com/z-galaxy/zbus/) | 5.19.0 | MIT | Zeeshan Ali Khan |
| [zbus-lockstep](https://github.com/luukvanderduim/zbus-lockstep) | 0.5.2 | MIT | Luuk van der Duim, Tait Hoyem&lt;tait@tait.tech&gt; |
| [zbus-lockstep-macros](https://github.com/luukvanderduim/zbus-lockstep) | 0.5.2 | MIT | Luuk van der Duim |
| [zbus_macros](https://github.com/dbus2/zbus/) | 4.4.0 | MIT | Marc-André Lureau, Zeeshan Ali Khan |
| [zbus_macros](https://github.com/z-galaxy/zbus/) | 5.19.0 | MIT | Marc-André Lureau, Zeeshan Ali Khan |
| [zbus_names](https://github.com/dbus2/zbus/) | 3.0.0 | MIT | Zeeshan Ali Khan |
| [zbus_names](https://github.com/z-galaxy/zbus/) | 4.3.5 | MIT | Zeeshan Ali Khan |
| [zbus_xml](https://github.com/z-galaxy/zbus/) | 5.2.1 | MIT | Zeeshan Ali Khan |
| [zcheapstr](https://github.com/z-galaxy/zcheapstr/) | 1.1.0 | MIT | Zeeshan Ali Khan |
| [zerocopy](https://github.com/google/zerocopy) | 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT | — |
| [zerocopy-derive](https://github.com/google/zerocopy) | 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT | — |
| [zerofrom](https://github.com/unicode-org/icu4x) | 0.1.8 | Unicode-3.0 | The ICU4X Project Developers |
| [zerofrom-derive](https://github.com/unicode-org/icu4x) | 0.1.8 | Unicode-3.0 | Manish Goregaokar |
| [zeroize](https://github.com/RustCrypto/utils) | 1.9.1 | Apache-2.0 OR MIT | The RustCrypto Project Developers |
| [zerotrie](https://github.com/unicode-org/icu4x) | 0.2.5 | Unicode-3.0 | The ICU4X Project Developers |
| [zerovec](https://github.com/unicode-org/icu4x) | 0.11.8 | Unicode-3.0 | The ICU4X Project Developers |
| [zerovec-derive](https://github.com/unicode-org/icu4x) | 0.11.6 | Unicode-3.0 | Manish Goregaokar |
| [zip](https://github.com/zip-rs/zip2.git) | 3.0.0 | MIT | Mathijs van de Nes, Marli Frost, Ryan Levick, Chris Hennick |
| [zlib-rs](https://github.com/trifectatechfoundation/zlib-rs) | 0.6.8 | Zlib | — |
| [zmij](https://github.com/dtolnay/zmij) | 1.0.23 | MIT | David Tolnay |
| [zopfli](https://github.com/zopfli-rs/zopfli) | 0.8.4 | Apache-2.0 | — |
| [zune-core](https://github.com/etemesi254/zune-image) | 0.5.3 | MIT OR Apache-2.0 OR Zlib | — |
| [zune-jpeg](https://github.com/etemesi254/zune-image/tree/dev/crates/zune-jpeg) | 0.5.15 | MIT OR Apache-2.0 OR Zlib | caleb |
| [zvariant](https://github.com/dbus2/zbus/) | 4.2.0 | MIT | Zeeshan Ali Khan |
| [zvariant](https://github.com/z-galaxy/zbus/) | 5.15.1 | MIT | Zeeshan Ali Khan |
| [zvariant_derive](https://github.com/dbus2/zbus/) | 4.2.0 | MIT | Zeeshan Ali Khan |
| [zvariant_derive](https://github.com/z-galaxy/zbus/) | 5.15.1 | MIT | Zeeshan Ali Khan |
| [zvariant_utils](https://github.com/dbus2/zbus/) | 2.1.0 | MIT | Zeeshan Ali Khan, turbocooler |
| [zvariant_utils](https://github.com/z-galaxy/zbus/) | 4.2.0 | MIT | Zeeshan Ali Khan, turbocooler |
