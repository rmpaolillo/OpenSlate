# Third-party software

OpenSlate stands on the shoulders of a lot of open-source work. This file lists
everything that ends up in (or is needed to build) the program. Thank you to all the
authors and contributors.

OpenSlate itself is licensed under the GNU General Public License v3.0 or later
(see [LICENSE](LICENSE)). The licences of all components below are compatible with it.
Per-file attributions are in [NOTICE](NOTICE). The release packages also contain
`THIRD-PARTY-LICENSES.txt` with the full licence text of every component (made by
`packaging/collect_licenses.py`).

## Code and ideas that shaped the project

| Project | Authors | Licence | How it is used |
|---|---|---|---|
| [Tuhi](https://github.com/tuhiproject/tuhi) | Peter Hutterer, Benjamin Tissoires and the Tuhi contributors | GPL-2.0-or-later | The Bluetooth protocol of the Wacom SmartPad family was translated from Tuhi's Python code. Without Tuhi's reverse engineering this project would not exist. |
| [WordCraft](https://github.com/storytold/wordcraft) | ArtCraft Team and the WordCraft contributors | MIT OR Apache-2.0 | `wordcraft-engine` crate: opening, rendering and saving Word documents with the handwritten ink; its `wordcraft-mcp` MCP server runs in-process (headless) so the AI chat can read and edit the open Word document; its `wordcraft-layout` crate is copied in `vendor/` with a small change (inline pictures follow the character position, as in Word: see `vendor/README.md`). The Word-like look of the interface (colours, measurements, rulers) is adapted from its `ui-egui` crate. |
| [PdfCraft](https://github.com/storytold/pdfcraft) | ArtCraft Team and the PdfCraft contributors | MIT OR Apache-2.0 | `pdfcraft-render` crate: crisp PDF page rendering at any zoom, page images and text extraction; `pdfcraft-automation` (its MCP server, in-process) and `pdfcraft-engine` let the AI chat read and edit the open PDF; its patched `lopdf` writes the signed PDFs and the photo pages; plus its patched `vello_common`. |
| [VectorCraft](https://github.com/storytold/vectorcraft) | ArtCraft Team and the VectorCraft contributors | MIT OR Apache-2.0 | `vectorcraft-engine` and `vectorcraft-render` crates (and `vectorcraft-mcp`, its MCP server run in-process for the AI chat's edits to SVG documents): opening SVG images as pages and saving them with the handwritten ink as vector paths; drawing and writing the board and library exports (PDF, SVG, PNG). |
| [PhotoCraft](https://github.com/storytold/photocraft) | ArtCraft Team and the PhotoCraft contributors | MIT OR Apache-2.0 | `photocraft-codecs` (reading and writing PNG, JPEG, WebP, TIFF… with colour profile and metadata) and `photocraft-paint` (the brush that draws the handwritten ink on photos). |
| [Typst](https://github.com/typst/typst) | The Typst Project Developers | Apache-2.0 | `typst`, `typst-layout`, `typst-render`, `typst-assets` crates: the LaTeX formulas of Markdown files opened as documents are drawn by Typst in LaTeX style. `typst-render` and `typst-svg` are copied in `vendor/` with a one-line change each, to build against the Craft suite's `hayro` (see `vendor/README.md`). |
| [Lilaq](https://github.com/lilaq-project/lilaq) and its dependencies [zero](https://github.com/Mc-Zen/zero), [tiptoe](https://github.com/Mc-Zen/tiptoe), [elembic](https://github.com/PgBiel/elembic), [suiji](https://github.com/liuguangxi/suiji) | Mc-Zen; PgBiel; liuguangxi | MIT (elembic: MIT OR Apache-2.0) | Typst packages bundled unmodified in `src/typst-pkg/`: the charts drawn by the AI chat (`draw_chart`). |
| [MiTeX](https://github.com/mitex-rs/mitex) | Myriad-Dreamin, OrangeX4, mgt and the MiTeX contributors | Apache-2.0 | `mitex` crate: turns LaTeX formulas into Typst code; its Typst definitions (`src/typst-pkg/mitex-specs/`, from the MiTeX Typst package, unmodified) are bundled. |
| [craft-libs](https://github.com/storytold/craft-libs) | ArtCraft Team | MIT OR Apache-2.0 | Patched `krilla` (PDF writing) and `hayro` (PDF reading) used by WordCraft, PdfCraft and VectorCraft. |
| [ArtCraft Launcher](https://github.com/storytold/craft-launcher) | ArtCraft Team | MIT OR Apache-2.0 | Model for the in-app updates: GitHub releases check, download in the background, check against the release's `SHA256SUMS.txt`, install in place of the running app (macOS `.app` from the DMG, `.deb`, archives) and restart; and for the Developer ID signing and notarization in the release workflow. Design adapted, no code copied. |
| [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness) | DeepSeek | — | Inspiration for the chat (ideas only, no code copied). |

## Fonts, icons and web assets

| Asset | Authors | Licence | Where |
|---|---|---|---|
| [Inter](https://github.com/rsms/inter) | Rasmus Andersson and The Inter Project Authors | SIL OFL 1.1 | `assets/fonts/Inter-*.ttf` (subset: Private Use Area removed) |
| [Source Serif 4](https://github.com/adobe-fonts/source-serif) | Frank Grießhammer, Adobe | SIL OFL 1.1 | `assets/fonts/SourceSerif4-*.ttf` |
| [Noto Sans CJK](https://github.com/notofonts/noto-cjk) | Google, Adobe | SIL OFL 1.1 | `assets/fonts/NotoSansCJK-langnames.otf` (subset: only the glyphs of «中文日本語한국어» for the language menu) |
| [New Computer Modern](https://ctan.org/pkg/newcomputermodern) | Antonis Tsolomitis | GUST Font License (NewCM10-Regular: see typst-assets) | embedded by the `typst-assets` crate: the formula fonts (the LaTeX look) |
| [Libertinus Serif](https://github.com/alerque/libertinus), [DejaVu Sans Mono](https://dejavu-fonts.github.io) | The Libertinus Project Authors; DejaVu authors | SIL OFL 1.1; Bitstream Vera licence | embedded by the `typst-assets` crate (Typst's default fonts; licences in its `NOTICE`) |
| [Phosphor Icons](https://phosphoricons.com) | Helena Zhang, Tobias Fried | MIT | via the `egui-phosphor` crate |
| [KaTeX](https://katex.org) | Khan Academy and contributors | MIT | `src/web/vendor/katex*` (formulas) |
| [marked](https://marked.js.org) | Christopher Jeffrey and the marked contributors | MIT | `src/web/vendor/marked.min.js` (Markdown) |
| [DOMPurify](https://github.com/cure53/DOMPurify) 3.4.16 | Cure53 and contributors | Apache-2.0 OR MPL-2.0 | `src/web/vendor/purify.min.js` (cleans the HTML of AI answers and notes before showing them) |
| [Hershey fonts](https://github.com/kamalmostafa/hershey-fonts) (`futural`, `cursive`) | Dr. A. V. Hershey (U.S. NBS); `.jhf` format by James Hurt (Cognition, Inc.); files by Kamal Mostafa | Hershey fonts licence (free use with acknowledgement, `src/handwrite/LICENSE-hershey.txt`) | `src/handwrite/*.jhf` (the chat's handwriting on the board) |

## Platform components used at run time (not bundled)

- **macOS**: CoreBluetooth, WKWebView, Vision (optional OCR, through a small Swift helper in `src/swift/`), Keychain.
- **All systems**: the system's Chinese/Japanese/Korean font (PingFang, Hiragino, Apple SD Gothic Neo, Microsoft YaHei, Yu Gothic, Malgun Gothic, Noto Sans CJK…) when the interface is in one of those languages.
- **Linux**: BlueZ (via D-Bus), GTK 3, WebKitGTK.
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
| [aes](https://github.com/RustCrypto/block-ciphers) | 0.8.4 | MIT OR Apache-2.0 | RustCrypto Developers |
| [aes](https://github.com/RustCrypto/block-ciphers) | 0.9.3 | MIT OR Apache-2.0 | RustCrypto Developers |
| [ahash](https://github.com/tkaitchuck/ahash) | 0.7.8 | MIT OR Apache-2.0 | Tom Kaitchuck |
| [ahash](https://github.com/tkaitchuck/ahash) | 0.8.12 | MIT OR Apache-2.0 | Tom Kaitchuck |
| [aho-corasick](https://github.com/BurntSushi/aho-corasick) | 1.1.5 | Unlicense OR MIT | Andrew Gallant |
| [aligned-vec](https://github.com/sarah-ek/aligned-vec/) | 0.6.4 | MIT | sarah |
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
| [approx](https://github.com/brendanzab/approx) | 0.5.1 | Apache-2.0 | Brendan Zabarauskas |
| [ar_archive_writer](https://github.com/rust-lang/ar_archive_writer) | 0.5.3 | Apache-2.0 WITH LLVM-exception | — |
| [arbitrary](https://github.com/rust-fuzz/arbitrary/) | 1.5.0 | MIT OR Apache-2.0 | The Rust-Fuzz Project Developers, Nick Fitzgerald, Manish Goregaokar, Simonas Kazlauskas, Brian L. Troutwine, Corey Farwell |
| [arboard](https://github.com/1Password/arboard) | 3.7.0 | MIT OR Apache-2.0 | — |
| [arrayref](https://github.com/droundy/arrayref) | 0.3.9 | BSD-2-Clause | David Roundy |
| [arrayvec](https://github.com/bluss/arrayvec) | 0.7.8 | MIT OR Apache-2.0 | bluss |
| [as-raw-xcb-connection](https://github.com/psychon/as-raw-xcb-connection) | 1.0.1 | MIT OR Apache-2.0 | — |
| [ash](https://github.com/ash-rs/ash) | 0.38.0+1.3.281 | MIT OR Apache-2.0 | Maik Klein, Benjamin Saunders, Marijn Suijten |
| [async-broadcast](https://github.com/smol-rs/async-broadcast) | 0.7.2 | MIT OR Apache-2.0 | Stjepan Glavina, Yoshua Wuyts, Zeeshan Ali Khan |
| [async-channel](https://github.com/smol-rs/async-channel) | 2.5.0 | Apache-2.0 OR MIT | Stjepan Glavina |
| [async-executor](https://github.com/smol-rs/async-executor) | 1.14.0 | Apache-2.0 OR MIT | Stjepan Glavina, John Nunley |
| [async-io](https://github.com/smol-rs/async-io) | 2.6.0 | Apache-2.0 OR MIT | Stjepan Glavina |
| [async-lock](https://github.com/smol-rs/async-lock) | 3.4.2 | Apache-2.0 OR MIT | Stjepan Glavina |
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
| [aws-lc-rs](https://github.com/aws/aws-lc-rs) | 1.18.2 | ISC AND (Apache-2.0 OR ISC) | AWS-LibCrypto |
| [aws-lc-sys](https://github.com/aws/aws-lc-rs) | 0.46.0 | ISC AND (Apache-2.0 OR ISC) AND Apache-2.0 AND MIT AND BSD-3-Clause AND (Apache-2.0 OR ISC OR MIT) AND (Apache-2.0 OR ISC OR MIT-0) | AWS-LC |
| [az](https://gitlab.com/tspiteri/az) | 1.3.0 | MIT OR Apache-2.0 | — |
| [base16ct](https://github.com/RustCrypto/formats) | 1.0.0 | Apache-2.0 OR MIT | RustCrypto Developers |
| [base64](https://github.com/marshallpierce/rust-base64) | 0.22.1 | MIT OR Apache-2.0 | Marshall Pierce |
| [base64](https://github.com/marshallpierce/rust-base64) | 0.23.1 | MIT OR Apache-2.0 | Marshall Pierce |
| [base64ct](https://github.com/RustCrypto/formats) | 1.8.3 | Apache-2.0 OR MIT | RustCrypto Developers |
| [beef](https://github.com/maciejhirsz/beef) | 0.5.2 | MIT OR Apache-2.0 | Maciej Hirsz |
| [biblatex](https://github.com/typst/biblatex) | 0.12.0 | MIT OR Apache-2.0 | Martin Haug |
| [bincode](https://github.com/servo/bincode) | 1.3.3 | MIT | Ty Overby, Francesco Mazzoli, David Tolnay, Zoey Riordan |
| [bit-set](https://github.com/contain-rs/bit-set) | 0.8.0 | Apache-2.0 OR MIT | Alexis Beingessner |
| [bit-set](https://github.com/contain-rs/bit-set) | 0.10.0 | Apache-2.0 OR MIT | Alexis Beingessner |
| [bit-vec](https://github.com/contain-rs/bit-vec) | 0.8.0 | Apache-2.0 OR MIT | Alexis Beingessner |
| [bit-vec](https://github.com/contain-rs/bit-vec) | 0.9.1 | Apache-2.0 OR MIT | Alexis Beingessner |
| [bit_field](https://github.com/phil-opp/rust-bit-field) | 0.10.3 | Apache-2.0 OR MIT | Philipp Oppermann |
| [bitflags](https://github.com/bitflags/bitflags) | 1.3.2 | MIT OR Apache-2.0 | The Rust Project Developers |
| [bitflags](https://github.com/bitflags/bitflags) | 2.13.2 | MIT OR Apache-2.0 | The Rust Project Developers |
| [bitvec](https://github.com/bitvecto-rs/bitvec) | 1.1.1 | MIT | — |
| [block](http://github.com/SSheldon/rust-block) | 0.1.6 | MIT | Steven Sheldon |
| [block-buffer](https://github.com/RustCrypto/utils) | 0.10.4 | MIT OR Apache-2.0 | RustCrypto Developers |
| [block-buffer](https://github.com/RustCrypto/utils) | 0.12.1 | MIT OR Apache-2.0 | RustCrypto Developers |
| [block-padding](https://github.com/RustCrypto/utils) | 0.4.2 | MIT OR Apache-2.0 | RustCrypto Developers |
| [block2](https://github.com/madsmtm/objc2) | 0.5.1 | MIT | Steven Sheldon, Mads Marquart |
| [block2](https://github.com/madsmtm/objc2) | 0.6.2 | MIT | Mads Marquart |
| [blocking](https://github.com/smol-rs/blocking) | 1.7.0 | Apache-2.0 OR MIT | — |
| [bluez-async](https://github.com/bluez-rs/bluez-async/) | 0.8.2 | MIT OR Apache-2.0 | Andrew Walbran, David Laban |
| [bluez-generated](https://github.com/bluez-rs/bluez-async/) | 0.4.0 | MIT OR Apache-2.0 | Andrew Walbran, David Laban |
| [boa_ast](https://github.com/boa-dev/boa) | 0.22.0 | Unlicense OR MIT | boa-dev |
| [boa_engine](https://github.com/boa-dev/boa) | 0.22.0 | Unlicense OR MIT | boa-dev |
| [boa_gc](https://github.com/boa-dev/boa) | 0.22.0 | Unlicense OR MIT | boa-dev |
| [boa_interner](https://github.com/boa-dev/boa) | 0.22.0 | Unlicense OR MIT | boa-dev |
| [boa_macros](https://github.com/boa-dev/boa) | 0.22.0 | Unlicense OR MIT | boa-dev |
| [boa_parser](https://github.com/boa-dev/boa) | 0.22.0 | Unlicense OR MIT | boa-dev |
| [boa_string](https://github.com/boa-dev/boa) | 0.22.0 | Unlicense OR MIT | boa-dev |
| [bp256](https://github.com/RustCrypto/elliptic-curves) | 0.14.0 | Apache-2.0 OR MIT | RustCrypto Developers |
| [bp384](https://github.com/RustCrypto/elliptic-curves) | 0.14.0 | Apache-2.0 OR MIT | RustCrypto Developers |
| [brotli](https://github.com/dropbox/rust-brotli) | 8.0.4 | BSD-3-Clause AND MIT | Daniel Reiter Horn, The Brotli Authors |
| [brotli-decompressor](https://github.com/dropbox/rust-brotli-decompressor) | 5.0.3 | BSD-3-Clause OR MIT | Daniel Reiter Horn, The Brotli Authors |
| [btleplug](https://github.com/deviceplug/btleplug) | 0.11.8 | MIT OR Apache-2.0 OR BSD-3-Clause | Nonpolynomial, LLC |
| [bumpalo](https://github.com/fitzgen/bumpalo) | 3.20.3 | MIT OR Apache-2.0 | Nick Fitzgerald |
| [by_address](https://github.com/mbrubeck/by_address) | 1.2.1 | MIT OR Apache-2.0 | Matt Brubeck |
| [bytecheck](https://github.com/djkoloski/bytecheck) | 0.6.12 | MIT | David Koloski |
| [bytecheck_derive](https://github.com/djkoloski/bytecheck) | 0.6.12 | MIT | David Koloski |
| [bytemuck](https://github.com/Lokathor/bytemuck) | 1.25.2 | Zlib OR Apache-2.0 OR MIT | Lokathor |
| [bytemuck_derive](https://github.com/Lokathor/bytemuck) | 1.12.1 | Zlib OR Apache-2.0 OR MIT | Lokathor |
| [byteorder](https://github.com/BurntSushi/byteorder) | 1.5.0 | Unlicense OR MIT | Andrew Gallant |
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
| [cesu8](https://github.com/emk/cesu8-rs) | 1.1.0 | Apache-2.0 OR MIT | Eric Kidd |
| [cfb](https://github.com/mdsteele/rust-cfb) | 0.15.0 | MIT | Matthew D. Steele |
| [cfg-expr](https://github.com/EmbarkStudios/cfg-expr) | 0.15.8 | MIT OR Apache-2.0 | Embark, Jake Shadle |
| [cfg-if](https://github.com/rust-lang/cfg-if) | 1.0.5 | MIT OR Apache-2.0 | Alex Crichton |
| [cfg_aliases](https://github.com/katharostech/cfg_aliases) | 0.2.2 | MIT | Zicklag |
| [cgl](https://github.com/servo/cgl-rs) | 0.3.2 | MIT  OR  Apache-2.0 | The Servo Project Developers |
| [chacha20](https://github.com/RustCrypto/stream-ciphers) | 0.10.2 | MIT OR Apache-2.0 | RustCrypto Developers |
| [chardetng](https://github.com/hsivonen/chardetng) | 1.0.0 | Apache-2.0 OR MIT | Henri Sivonen |
| [chinese-number](https://github.com/magiclen/chinese-number) | 0.7.8 | MIT | Magic Len |
| [chinese-variant](https://github.com/magiclen/chinese-variant) | 1.1.6 | MIT | Magic Len |
| [chrono](https://github.com/chronotope/chrono) | 0.4.45 | MIT OR Apache-2.0 | — |
| [ciborium](https://github.com/enarx/ciborium) | 0.2.2 | Apache-2.0 | Nathaniel McCallum |
| [ciborium-io](https://github.com/enarx/ciborium) | 0.2.2 | Apache-2.0 | Nathaniel McCallum |
| [ciborium-ll](https://github.com/enarx/ciborium) | 0.2.2 | Apache-2.0 | Nathaniel McCallum |
| [cipher](https://github.com/RustCrypto/traits) | 0.4.4 | MIT OR Apache-2.0 | RustCrypto Developers |
| [cipher](https://github.com/RustCrypto/traits) | 0.5.2 | MIT OR Apache-2.0 | RustCrypto Developers |
| [citationberg](https://github.com/typst/citationberg) | 0.7.0 | MIT OR Apache-2.0 | Martin Haug |
| [clap](https://github.com/clap-rs/clap) | 4.6.7 | MIT OR Apache-2.0 | — |
| [clap_builder](https://github.com/clap-rs/clap) | 4.6.7 | MIT OR Apache-2.0 | — |
| [clap_derive](https://github.com/clap-rs/clap) | 4.6.7 | MIT OR Apache-2.0 | — |
| [clap_lex](https://github.com/clap-rs/clap) | 1.1.1 | MIT OR Apache-2.0 | — |
| [clipboard-win](https://github.com/DoumanAsh/clipboard-win) | 5.4.1 | BSL-1.0 | Douman |
| [cmake](https://github.com/rust-lang/cmake-rs) | 0.1.58 | MIT OR Apache-2.0 | Alex Crichton |
| [cmov](https://github.com/RustCrypto/utils) | 0.5.4 | Apache-2.0 OR MIT | RustCrypto Developers |
| [cmpv2](https://github.com/RustCrypto/formats/tree/master/cmpv2) | 0.2.0 | Apache-2.0 OR MIT | RustCrypto Developers |
| [cms](https://github.com/RustCrypto/formats/tree/master/cms) | 0.2.3 | Apache-2.0 OR MIT | RustCrypto Developers |
| [cobs](https://github.com/jamesmunns/cobs.rs) | 0.3.0 | MIT OR Apache-2.0 | Allen Welkie, James Munns |
| [codespan-reporting](https://github.com/brendanzab/codespan) | 0.13.1 | Apache-2.0 | Brendan Zabarauskas |
| [codex](https://github.com/typst/codex) | 0.3.0 | Apache-2.0 | The Typst Project Developers |
| [color](https://github.com/linebender/color) | 0.3.3 | Apache-2.0 OR MIT | — |
| [color_quant](https://github.com/image-rs/color_quant.git) | 1.1.0 | MIT | nwin |
| [colorchoice](https://github.com/rust-cli/anstyle.git) | 1.0.5 | MIT OR Apache-2.0 | — |
| [combine](https://github.com/Marwes/combine) | 4.6.9 | MIT | Markus Westerlind |
| [comemo](https://github.com/typst/comemo) | 0.5.1 | MIT OR Apache-2.0 | Laurenz |
| [comemo-macros](https://github.com/typst/comemo) | 0.5.1 | MIT OR Apache-2.0 | Laurenz |
| [concurrent-queue](https://github.com/smol-rs/concurrent-queue) | 2.5.0 | Apache-2.0 OR MIT | Stjepan Glavina, Taiki Endo, John Nunley |
| [const-oid](https://github.com/RustCrypto/formats/tree/master/const-oid) | 0.9.6 | Apache-2.0 OR MIT | RustCrypto Developers |
| [const-oid](https://github.com/RustCrypto/formats) | 0.10.2 | Apache-2.0 OR MIT | RustCrypto Developers |
| [cookie](https://github.com/SergioBenitez/cookie-rs) | 0.18.2 | MIT OR Apache-2.0 | Sergio Benitez, Alex Crichton |
| [core-foundation](https://github.com/servo/core-foundation-rs) | 0.9.4 | MIT OR Apache-2.0 | The Servo Project Developers |
| [core-foundation](https://github.com/servo/core-foundation-rs) | 0.10.1 | MIT OR Apache-2.0 | The Servo Project Developers |
| [core-foundation-sys](https://github.com/servo/core-foundation-rs) | 0.8.7 | MIT OR Apache-2.0 | The Servo Project Developers |
| [core-graphics](https://github.com/servo/core-foundation-rs) | 0.23.2 | MIT OR Apache-2.0 | The Servo Project Developers |
| [core-graphics](https://github.com/servo/core-foundation-rs) | 0.25.0 | MIT OR Apache-2.0 | The Servo Project Developers |
| [core-graphics-types](https://github.com/servo/core-foundation-rs) | 0.1.3 | MIT OR Apache-2.0 | The Servo Project Developers |
| [core-graphics-types](https://github.com/servo/core-foundation-rs) | 0.2.0 | MIT OR Apache-2.0 | The Servo Project Developers |
| [core_detect](https://github.com/thomcc/core_detect) | 1.0.0 | MIT OR Apache-2.0 | Thom Chiovoloni |
| [core_maths](https://github.com/robertbastian/core_maths) | 0.1.1 | MIT | Robert Bastian <me@robertbastian.dev |
| [countme](https://github.com/matklad/countme) | 3.0.1 | MIT OR Apache-2.0 | Aleksey Kladov |
| [cow-utils](https://github.com/RReverser/cow-utils-rs) | 0.1.3 | MIT | Ingvar Stepanyan |
| [cpubits](https://github.com/RustCrypto/utils) | 0.1.1 | MIT OR Apache-2.0 | RustCrypto Developers |
| [cpufeatures](https://github.com/RustCrypto/utils) | 0.2.17 | MIT OR Apache-2.0 | RustCrypto Developers |
| [cpufeatures](https://github.com/RustCrypto/utils) | 0.3.1 | MIT OR Apache-2.0 | RustCrypto Developers |
| [craft-docpath](https://github.com/storytold/craft-libs) | 0.1.0 | MIT OR Apache-2.0 | — |
| [craft-pdf](https://github.com/storytold/craft-libs) | 0.1.0 | MIT OR Apache-2.0 | — |
| [craft-pdf](https://github.com/storytold/craft-libs) | 0.1.0 | MIT OR Apache-2.0 | — |
| [craft-print](https://github.com/storytold/craft-libs) | 0.1.0 | MIT OR Apache-2.0 | — |
| [crc32fast](https://github.com/srijs/rust-crc32fast) | 1.5.2 | MIT OR Apache-2.0 | Sam Rijs, Alex Crichton |
| [crmf](https://github.com/RustCrypto/formats/tree/master/crmf) | 0.2.0 | Apache-2.0 OR MIT | RustCrypto Developers |
| [crossbeam-channel](https://github.com/crossbeam-rs/crossbeam) | 0.5.17 | MIT OR Apache-2.0 | — |
| [crossbeam-deque](https://github.com/crossbeam-rs/crossbeam) | 0.8.8 | MIT OR Apache-2.0 | — |
| [crossbeam-epoch](https://github.com/crossbeam-rs/crossbeam) | 0.9.21 | MIT OR Apache-2.0 | — |
| [crossbeam-utils](https://github.com/crossbeam-rs/crossbeam) | 0.8.23 | MIT OR Apache-2.0 | — |
| [crunchy](https://github.com/eira-fransham/crunchy) | 0.2.4 | MIT | Eira Fransham |
| [crypto-bigint](https://github.com/RustCrypto/crypto-bigint) | 0.7.5 | Apache-2.0 OR MIT | RustCrypto Developers |
| [crypto-common](https://github.com/RustCrypto/traits) | 0.1.7 | MIT OR Apache-2.0 | RustCrypto Developers |
| [crypto-common](https://github.com/RustCrypto/traits) | 0.2.2 | MIT OR Apache-2.0 | RustCrypto Developers |
| [crypto-primes](https://github.com/entropyxyz/crypto-primes) | 0.7.2 | Apache-2.0 OR MIT | — |
| [cssparser](https://github.com/servo/rust-cssparser) | 0.37.0 | MPL-2.0 | Simon Sapin |
| [cssparser-macros](https://github.com/servo/rust-cssparser) | 0.7.1 | MPL-2.0 | Simon Sapin |
| [csv](https://github.com/BurntSushi/rust-csv) | 1.4.0 | Unlicense OR MIT | Andrew Gallant |
| [csv-core](https://github.com/BurntSushi/rust-csv) | 0.1.13 | Unlicense OR MIT | Andrew Gallant |
| [ctutils](https://github.com/RustCrypto/utils) | 0.4.3 | Apache-2.0 OR MIT | RustCrypto Developers |
| [cursor-icon](https://github.com/rust-windowing/cursor-icon) | 1.2.0 | MIT OR Apache-2.0 OR Zlib | Kirill Chibisov |
| [curve25519-dalek](https://github.com/dalek-cryptography/curve25519-dalek/tree/main/curve25519-dalek) | 5.0.0 | BSD-3-Clause | Isis Lovecruft, Henry de Valence |
| [curve25519-dalek-derive](https://github.com/dalek-cryptography/curve25519-dalek) | 0.1.1 | MIT OR Apache-2.0 | — |
| [dashmap](https://github.com/xacrimon/dashmap) | 5.5.3 | MIT | Acrimon |
| [dashmap](https://github.com/xacrimon/dashmap) | 6.2.1 | MIT | Acrimon |
| [data-url](https://github.com/servo/rust-url) | 0.3.2 | MIT OR Apache-2.0 | Simon Sapin |
| [dbus](https://github.com/diwic/dbus-rs) | 0.9.12 | Apache-2.0 OR MIT | David Henningsson |
| [dbus-tokio](https://github.com/diwic/dbus-rs) | 0.7.6 | Apache-2.0 OR MIT | David Henningsson |
| [der](https://github.com/RustCrypto/formats/tree/master/der) | 0.7.10 | Apache-2.0 OR MIT | RustCrypto Developers |
| [der](https://github.com/RustCrypto/formats) | 0.8.2 | Apache-2.0 OR MIT | RustCrypto Developers |
| [der_derive](https://github.com/RustCrypto/formats/tree/master/der/derive) | 0.7.3 | Apache-2.0 OR MIT | RustCrypto Developers |
| [deranged](https://github.com/jhpratt/deranged) | 0.5.9 | MIT OR Apache-2.0 | Jacob Pratt |
| [derive_arbitrary](https://github.com/rust-fuzz/arbitrary) | 1.5.0 | MIT OR Apache-2.0 | The Rust-Fuzz Project Developers, Nick Fitzgerald, Manish Goregaokar, Andre Bogus, Corey Farwell |
| [derive_more](https://github.com/JelteF/derive_more) | 2.1.1 | MIT | Jelte Fennema |
| [derive_more-impl](https://github.com/JelteF/derive_more) | 2.1.1 | MIT | Jelte Fennema |
| [des](https://github.com/RustCrypto/block-ciphers) | 0.9.0 | MIT OR Apache-2.0 | RustCrypto Developers |
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
| [downcast-rs](https://github.com/marcianx/downcast-rs) | 1.2.1 | MIT OR Apache-2.0 | Ashish Myles, Runji Wang |
| [dpi](https://github.com/rust-windowing/winit) | 0.1.2 | Apache-2.0 AND MIT | — |
| [dtoa](https://github.com/dtolnay/dtoa) | 1.0.11 | MIT OR Apache-2.0 | David Tolnay |
| [dtoa-short](https://github.com/upsuper/dtoa-short) | 0.3.5 | MPL-2.0 | Xidorn Quan |
| [dunce](https://gitlab.com/kornelski/dunce) | 1.0.5 | CC0-1.0 OR MIT-0 OR Apache-2.0 | Kornel |
| [dynify](https://github.com/loichyan/dynify) | 0.1.2 | MIT OR Apache-2.0 | Loi Chyan |
| [dynify-macros](https://github.com/loichyan/dynify) | 0.1.2 | MIT OR Apache-2.0 | Loi Chyan |
| [ecb](https://github.com/RustCrypto/block-modes) | 0.2.1 | MIT OR Apache-2.0 | RustCrypto Developers |
| [ecdsa](https://github.com/RustCrypto/signatures) | 0.17.0 | Apache-2.0 OR MIT | RustCrypto Developers |
| [ecolor](https://github.com/emilk/egui) | 0.36.2 | MIT OR Apache-2.0 | Emil Ernerfeldt, Andreas Reich |
| [ecow](https://github.com/typst/ecow) | 0.2.6 | MIT OR Apache-2.0 | Laurenz |
| [ed25519](https://github.com/RustCrypto/signatures) | 3.0.0 | Apache-2.0 OR MIT | RustCrypto Developers |
| [ed25519-dalek](https://github.com/dalek-cryptography/curve25519-dalek/tree/main/ed25519-dalek) | 3.0.0 | BSD-3-Clause | isis lovecruft, Tony Arcieri, Michael Rosenberg |
| [eframe](https://github.com/emilk/egui/tree/main/crates/eframe) | 0.36.2 | MIT OR Apache-2.0 | Emil Ernerfeldt |
| [egui](https://github.com/emilk/egui) | 0.36.2 | MIT OR Apache-2.0 | Emil Ernerfeldt |
| [egui-phosphor](https://github.com/amPerl/egui-phosphor) | 0.14.0 | MIT OR Apache-2.0 | — |
| [egui-wgpu](https://github.com/emilk/egui/tree/main/crates/egui-wgpu) | 0.36.2 | MIT OR Apache-2.0 | Nils Hasenbanck, embotech, Emil Ernerfeldt |
| [egui-winit](https://github.com/emilk/egui/tree/main/crates/egui-winit) | 0.36.2 | MIT OR Apache-2.0 | Emil Ernerfeldt |
| [egui_glow](https://github.com/emilk/egui/tree/main/crates/egui_glow) | 0.36.2 | MIT OR Apache-2.0 | Emil Ernerfeldt |
| [either](https://github.com/rayon-rs/either) | 1.19.0 | MIT OR Apache-2.0 | — |
| [elliptic-curve](https://github.com/RustCrypto/traits) | 0.14.1 | Apache-2.0 OR MIT | RustCrypto Developers |
| [emath](https://github.com/emilk/egui/tree/main/crates/emath) | 0.36.2 | MIT OR Apache-2.0 | Emil Ernerfeldt |
| [ena](https://github.com/rust-lang/ena) | 0.14.4 | MIT OR Apache-2.0 | Niko Matsakis |
| [encoding_rs](https://github.com/hsivonen/encoding_rs) | 0.8.42 | (Apache-2.0 OR MIT) AND BSD-3-Clause | Henri Sivonen |
| [endi](https://github.com/zeenix/endi) | 1.1.1 | MIT | Zeeshan Ali Khan |
| [enum-ordinalize](https://github.com/magiclen/enum-ordinalize) | 4.4.2 | MIT | — |
| [enum-ordinalize-derive](https://github.com/magiclen/enum-ordinalize) | 4.4.2 | MIT | — |
| [enumflags2](https://github.com/meithecatte/enumflags2) | 0.7.12 | MIT OR Apache-2.0 | maik klein, Maja Kądziołka |
| [enumflags2_derive](https://github.com/meithecatte/enumflags2) | 0.7.12 | MIT OR Apache-2.0 | maik klein, Maja Kądziołka |
| [epaint](https://github.com/emilk/egui/tree/main/crates/epaint) | 0.36.2 | MIT OR Apache-2.0 | Emil Ernerfeldt |
| [epaint_default_fonts](https://github.com/emilk/egui/tree/main/crates/epaint_default_fonts) | 0.36.2 | (MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0 | Emil Ernerfeldt |
| [equator](https://github.com/sarah-ek/equator/) | 0.4.2 | MIT | sarah |
| [equator-macro](https://github.com/sarah-ek/equator/) | 0.4.2 | MIT | sarah |
| [equivalent](https://github.com/indexmap-rs/equivalent) | 1.0.3 | Apache-2.0 OR MIT | — |
| [errno](https://github.com/lambda-fairy/rust-errno) | 0.3.14 | MIT OR Apache-2.0 | Chris Wong, Dan Gohman |
| [error-code](https://github.com/DoumanAsh/error-code) | 3.4.0 | BSL-1.0 | Douman |
| [euclid](https://github.com/servo/euclid) | 0.22.14 | MIT OR Apache-2.0 | The Servo Project Developers |
| [event-listener](https://github.com/smol-rs/event-listener) | 5.4.2 | Apache-2.0 OR MIT | Stjepan Glavina, John Nunley |
| [event-listener-strategy](https://github.com/smol-rs/event-listener-strategy) | 0.5.4 | Apache-2.0 OR MIT | John Nunley |
| [exmex](https://github.com/bertiqwerty/exmex/) | 0.20.5 | MIT OR Apache-2.0 | Behrang Shafei |
| [exr](https://github.com/johannesvollmer/exrs) | 1.74.2 | BSD-3-Clause | johannesvollmer |
| [fancy-regex](https://github.com/fancy-regex/fancy-regex) | 0.16.2 | MIT | Raph Levien, Robin Stocker, Keith Hall |
| [fast-float2](https://github.com/Alexhuszagh/fast-float-rust) | 0.2.4 | MIT OR Apache-2.0 | Ivan Smirnov, Alex Huszagh |
| [fastrand](https://github.com/smol-rs/fastrand) | 2.5.0 | Apache-2.0 OR MIT | Stjepan Glavina |
| [fax](https://github.com/pdf-rs/fax) | 0.2.7 | MIT | Sebastian K |
| [fdeflate](https://github.com/image-rs/fdeflate) | 0.3.7 | MIT OR Apache-2.0 | The image-rs Developers |
| [fearless_simd](https://github.com/linebender/fearless_simd) | 0.4.1 | Apache-2.0 OR MIT | Raph Levien |
| [ff](https://github.com/zkcrypto/ff) | 0.14.0 | MIT OR Apache-2.0 | Sean Bowe, Jack Grigg |
| [fiat-crypto](https://github.com/mit-plv/fiat-crypto) | 0.3.0 | MIT OR Apache-2.0 OR BSD-1-Clause | Fiat Crypto library authors |
| [field-offset](https://github.com/Diggsey/rust-field-offset) | 0.3.6 | MIT OR Apache-2.0 | Diggory Blake |
| [filetime](https://github.com/alexcrichton/filetime) | 0.2.29 | MIT OR Apache-2.0 | Alex Crichton |
| [find-msvc-tools](https://github.com/rust-lang/cc-rs) | 0.1.15 | MIT OR Apache-2.0 | — |
| [fixedbitset](https://github.com/petgraph/fixedbitset) | 0.5.7 | MIT OR Apache-2.0 | bluss |
| [flagset](https://github.com/enarx/flagset) | 0.4.7 | Apache-2.0 | Nathaniel McCallum |
| [flatbuffers](https://github.com/google/flatbuffers) | 24.12.23 | Apache-2.0 | Robert Winslow, FlatBuffers Maintainers |
| [flate2](https://github.com/rust-lang/flate2-rs) | 1.1.10 | MIT OR Apache-2.0 | Alex Crichton, Josh Triplett |
| [float-cmp](https://github.com/mikedilger/float-cmp) | 0.9.0 | MIT | Mike Dilger |
| [fmt-derive](https://github.com/danielschemmel/fmt-derive) | 0.1.2 | MIT OR Apache-2.0 | Daniel Schemmel |
| [fmt-derive-proc](https://github.com/danielschemmel/fmt-derive) | 0.1.2 | MIT OR Apache-2.0 | Daniel Schemmel |
| [fnv](https://github.com/servo/rust-fnv) | 1.0.7 | Apache-2.0  OR  MIT | Alex Crichton |
| [foldhash](https://github.com/orlp/foldhash) | 0.1.5 | Zlib | Orson Peters |
| [foldhash](https://github.com/orlp/foldhash) | 0.2.0 | Zlib | Orson Peters |
| [font-types](https://github.com/googlefonts/fontations) | 0.11.3 | MIT OR Apache-2.0 | — |
| [font-types](https://github.com/googlefonts/fontations) | 0.12.7 | MIT OR Apache-2.0 | — |
| [fontdb](https://github.com/RazrFalcon/fontdb) | 0.23.0 | MIT | Yevhenii Reizner |
| [foreign-types](https://github.com/sfackler/foreign-types) | 0.5.0 | MIT OR Apache-2.0 | Steven Fackler |
| [foreign-types-macros](https://github.com/sfackler/foreign-types) | 0.2.4 | MIT OR Apache-2.0 | Steven Fackler |
| [foreign-types-shared](https://github.com/sfackler/foreign-types) | 0.3.1 | MIT OR Apache-2.0 | Steven Fackler |
| [form_urlencoded](https://github.com/servo/rust-url) | 1.2.2 | MIT OR Apache-2.0 | The rust-url developers |
| [fs_extra](https://github.com/webdesus/fs_extra) | 1.3.0 | MIT | Denis Kurilenko |
| [funty](https://github.com/myrrlyn/funty) | 2.0.0 | MIT | myrrlyn |
| [futures](https://github.com/rust-lang/futures-rs) | 0.3.34 | MIT OR Apache-2.0 | — |
| [futures-channel](https://github.com/rust-lang/futures-rs) | 0.3.34 | MIT OR Apache-2.0 | — |
| [futures-concurrency](https://github.com/yoshuawuyts/futures-concurrency) | 7.7.1 | MIT OR Apache-2.0 | Yoshua Wuyts |
| [futures-core](https://github.com/rust-lang/futures-rs) | 0.3.34 | MIT OR Apache-2.0 | — |
| [futures-executor](https://github.com/rust-lang/futures-rs) | 0.3.34 | MIT OR Apache-2.0 | — |
| [futures-io](https://github.com/rust-lang/futures-rs) | 0.3.34 | MIT OR Apache-2.0 | — |
| [futures-lite](https://github.com/smol-rs/futures-lite) | 2.6.1 | Apache-2.0 OR MIT | Stjepan Glavina, Contributors to futures-rs |
| [futures-macro](https://github.com/rust-lang/futures-rs) | 0.3.34 | MIT OR Apache-2.0 | — |
| [futures-sink](https://github.com/rust-lang/futures-rs) | 0.3.34 | MIT OR Apache-2.0 | — |
| [futures-task](https://github.com/rust-lang/futures-rs) | 0.3.34 | MIT OR Apache-2.0 | — |
| [futures-util](https://github.com/rust-lang/futures-rs) | 0.3.34 | MIT OR Apache-2.0 | — |
| [fxhash](https://github.com/cbreeden/fxhash) | 0.2.1 | Apache-2.0 OR MIT | cbreeden |
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
| [glidesort](https://github.com/orlp/glidesort) | 0.1.2 | MIT OR Apache-2.0 | Orson Peters |
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
| [group](https://github.com/zkcrypto/group) | 0.14.0 | MIT OR Apache-2.0 | Sean Bowe, Jack Grigg |
| [gtk](https://github.com/gtk-rs/gtk3-rs) | 0.18.2 | MIT | The gtk-rs Project Developers |
| [gtk-sys](https://github.com/gtk-rs/gtk3-rs) | 0.18.2 | MIT | The gtk-rs Project Developers |
| [gtk3-macros](https://github.com/gtk-rs/gtk3-rs) | 0.18.2 | MIT | The gtk-rs Project Developers |
| [guillotiere](https://github.com/nical/guillotiere) | 0.7.0 | MIT OR Apache-2.0 | Nicolas Silva |
| [h2](https://github.com/hyperium/h2) | 0.4.20 | MIT | Carl Lerche, Sean McArthur |
| [half](https://github.com/VoidStarKat/half-rs) | 2.7.1 | MIT OR Apache-2.0 | Kathryn Long |
| [harfrust](https://github.com/harfbuzz/harfrust) | 0.12.0 | MIT | — |
| [harfrust](https://github.com/harfbuzz/harfrust) | 0.13.3 | MIT | — |
| [hashbrown](https://github.com/rust-lang/hashbrown) | 0.12.3 | MIT OR Apache-2.0 | Amanieu d'Antras |
| [hashbrown](https://github.com/rust-lang/hashbrown) | 0.14.5 | MIT OR Apache-2.0 | Amanieu d'Antras |
| [hashbrown](https://github.com/rust-lang/hashbrown) | 0.15.5 | MIT OR Apache-2.0 | Amanieu d'Antras |
| [hashbrown](https://github.com/rust-lang/hashbrown) | 0.16.1 | MIT OR Apache-2.0 | Amanieu d'Antras |
| [hashbrown](https://github.com/rust-lang/hashbrown) | 0.17.1 | MIT OR Apache-2.0 | — |
| [hayagriva](https://github.com/typst/hayagriva) | 0.10.1 | MIT OR Apache-2.0 | Martin Haug |
| [hayro](https://github.com/LaurenzV/hayro) | 0.7.1 | Apache-2.0 OR MIT | Laurenz Stampfl |
| [hayro-ccitt](https://github.com/LaurenzV/hayro) | 0.3.0 | Apache-2.0 OR MIT | Laurenz Stampfl |
| [hayro-cmap](https://github.com/LaurenzV/hayro) | 0.1.0 | Apache-2.0 OR MIT | Laurenz Stampfl |
| [hayro-interpret](https://github.com/LaurenzV/hayro) | 0.7.0 | Apache-2.0 OR MIT | Laurenz Stampfl |
| [hayro-jbig2](https://github.com/LaurenzV/hayro) | 0.3.0 | Apache-2.0 OR MIT | Laurenz Stampfl |
| [hayro-jpeg2000](https://github.com/LaurenzV/hayro) | 0.3.5 | Apache-2.0 OR MIT | Laurenz Stampfl |
| [hayro-postscript](https://github.com/LaurenzV/hayro) | 0.1.0 | Apache-2.0 OR MIT | Laurenz Stampfl |
| [hayro-svg](https://github.com/LaurenzV/hayro) | 0.7.0 | Apache-2.0 OR MIT | Laurenz Stampfl |
| [hayro-syntax](https://github.com/LaurenzV/hayro) | 0.7.2 | Apache-2.0 OR MIT | Laurenz Stampfl |
| [heck](https://github.com/withoutboats/heck) | 0.4.1 | MIT OR Apache-2.0 | Without Boats |
| [heck](https://github.com/withoutboats/heck) | 0.5.0 | MIT OR Apache-2.0 | — |
| [hermit-abi](https://github.com/hermit-os/hermit-rs) | 0.5.3 | MIT OR Apache-2.0 | — |
| [hex](https://github.com/KokaKiwi/rust-hex) | 0.4.3 | MIT OR Apache-2.0 | KokaKiwi |
| [hmac](https://github.com/RustCrypto/MACs) | 0.13.0 | MIT OR Apache-2.0 | RustCrypto Developers |
| [home](https://github.com/rust-lang/cargo) | 0.5.12 | MIT OR Apache-2.0 | Brian Anderson |
| [html5ever](https://github.com/servo/html5ever) | 0.39.0 | MIT OR Apache-2.0 | The html5ever Project Developers |
| [http](https://github.com/hyperium/http) | 1.5.0 | MIT OR Apache-2.0 | Alex Crichton, Carl Lerche, Sean McArthur |
| [http-body](https://github.com/hyperium/http-body) | 1.1.0 | MIT | Carl Lerche, Lucio Franco, Sean McArthur |
| [http-body-util](https://github.com/hyperium/http-body) | 0.1.5 | MIT | Carl Lerche, Lucio Franco, Sean McArthur |
| [httparse](https://github.com/seanmonstar/httparse) | 1.10.1 | MIT OR Apache-2.0 | Sean McArthur |
| [hybrid-array](https://github.com/RustCrypto/hybrid-array) | 0.4.15 | MIT OR Apache-2.0 | RustCrypto Developers |
| [hyper](https://github.com/hyperium/hyper) | 1.12.0 | MIT | Sean McArthur |
| [hyper-rustls](https://github.com/rustls/hyper-rustls) | 0.27.10 | Apache-2.0 OR ISC OR MIT | — |
| [hyper-util](https://github.com/hyperium/hyper-util) | 0.1.21 | MIT | Sean McArthur |
| [hypher](https://github.com/typst/hypher) | 0.1.8 | MIT OR Apache-2.0 | Laurenz |
| [iana-time-zone](https://github.com/strawlab/iana-time-zone) | 0.1.65 | MIT OR Apache-2.0 | Andrew Straw, René Kijewski, Ryan Lopopolo |
| [iana-time-zone-haiku](https://github.com/strawlab/iana-time-zone) | 0.1.2 | MIT OR Apache-2.0 | René Kijewski |
| [icu_collator](https://github.com/unicode-org/icu4x) | 2.3.1 | Unicode-3.0 | The ICU4X Project Developers |
| [icu_collator_data](https://github.com/unicode-org/icu4x) | 2.3.0 | Unicode-3.0 | The ICU4X Project Developers |
| [icu_collections](https://github.com/unicode-org/icu4x) | 2.3.0 | Unicode-3.0 | The ICU4X Project Developers |
| [icu_locale](https://github.com/unicode-org/icu4x) | 2.3.1 | Unicode-3.0 | The ICU4X Project Developers |
| [icu_locale_core](https://github.com/unicode-org/icu4x) | 2.3.0 | Unicode-3.0 | The ICU4X Project Developers |
| [icu_locale_data](https://github.com/unicode-org/icu4x) | 2.3.0 | Unicode-3.0 | The ICU4X Project Developers |
| [icu_locale_fallback](https://github.com/unicode-org/icu4x) | 2.3.0 | Unicode-3.0 | The ICU4X Project Developers |
| [icu_locale_fallback_data](https://github.com/unicode-org/icu4x) | 2.3.0 | Unicode-3.0 | The ICU4X Project Developers |
| [icu_normalizer](https://github.com/unicode-org/icu4x) | 2.3.0 | Unicode-3.0 | The ICU4X Project Developers |
| [icu_normalizer_data](https://github.com/unicode-org/icu4x) | 2.3.0 | Unicode-3.0 | The ICU4X Project Developers |
| [icu_properties](https://github.com/unicode-org/icu4x) | 2.3.0 | Unicode-3.0 | The ICU4X Project Developers |
| [icu_properties_data](https://github.com/unicode-org/icu4x) | 2.3.0 | Unicode-3.0 | The ICU4X Project Developers |
| [icu_provider](https://github.com/unicode-org/icu4x) | 2.3.1 | Unicode-3.0 | The ICU4X Project Developers |
| [icu_provider_blob](https://github.com/unicode-org/icu4x) | 2.3.0 | Unicode-3.0 | The ICU4X Project Developers |
| [icu_segmenter](https://github.com/unicode-org/icu4x) | 2.3.0 | Unicode-3.0 | The ICU4X Project Developers |
| [icu_segmenter_data](https://github.com/unicode-org/icu4x) | 2.3.0 | Unicode-3.0 | The ICU4X Project Developers |
| [idna](https://github.com/servo/rust-url/) | 1.1.0 | MIT OR Apache-2.0 | The rust-url developers |
| [idna_adapter](https://github.com/hsivonen/idna_adapter) | 1.2.2 | Apache-2.0 OR MIT | The rust-url developers |
| [image](https://github.com/image-rs/image) | 0.25.10 | MIT OR Apache-2.0 | The image-rs Developers |
| [image-webp](https://github.com/image-rs/image-webp) | 0.2.4 | MIT OR Apache-2.0 | — |
| [imagesize](https://github.com/Roughsketch/imagesize) | 0.14.0 | MIT | Maid Dog |
| [imagesize](https://github.com/Roughsketch/imagesize) | 0.15.0 | MIT | Maid Dog |
| [indexmap](https://github.com/indexmap-rs/indexmap) | 2.14.2 | Apache-2.0 OR MIT | — |
| [inout](https://github.com/RustCrypto/utils) | 0.1.4 | MIT OR Apache-2.0 | RustCrypto Developers |
| [inout](https://github.com/RustCrypto/utils) | 0.2.5 | MIT OR Apache-2.0 | RustCrypto Developers |
| [intrusive-collections](https://github.com/Amanieu/intrusive-rs) | 0.10.3 | MIT OR Apache-2.0 | Amanieu d'Antras |
| [ipnet](https://github.com/krisprice/ipnet) | 2.12.2 | MIT OR Apache-2.0 | Kris Price |
| [is_terminal_polyfill](https://github.com/polyfill-rs/is_terminal_polyfill) | 1.70.2 | MIT OR Apache-2.0 | — |
| [itertools](https://github.com/rust-itertools/itertools) | 0.14.0 | MIT OR Apache-2.0 | bluss |
| [itertools](https://github.com/rust-itertools/itertools) | 0.15.0 | MIT OR Apache-2.0 | bluss |
| [itoa](https://github.com/dtolnay/itoa) | 1.0.18 | MIT OR Apache-2.0 | David Tolnay |
| [javascriptcore-rs](https://github.com/tauri-apps/javascriptcore-rs) | 1.1.2 | MIT | — |
| [javascriptcore-rs-sys](https://github.com/tauri-apps/javascriptcore-rs) | 1.1.1 | MIT | The Gtk-rs Project Developers |
| [jni](https://github.com/jni-rs/jni-rs) | 0.19.0 | MIT OR Apache-2.0 | Josh Chase |
| [jni](https://github.com/jni-rs/jni-rs) | 0.21.1 | MIT OR Apache-2.0 | Josh Chase |
| [jni](https://github.com/jni-rs/jni-rs) | 0.22.4 | MIT OR Apache-2.0 | jni team |
| [jni-macros](https://github.com/jni-rs/jni-rs) | 0.22.4 | MIT OR Apache-2.0 | — |
| [jni-sys](https://github.com/jni-rs/jni-sys) | 0.3.1 | MIT OR Apache-2.0 | Steven Fackler |
| [jni-sys](https://github.com/jni-rs/jni-sys) | 0.4.1 | MIT OR Apache-2.0 | Steven Fackler, Robert Bragg |
| [jni-sys-macros](https://github.com/jni-rs/jni-sys) | 0.4.1 | MIT OR Apache-2.0 | Robert Bragg |
| [jni-utils](https://github.com/deviceplug/jni-utils-rs) | 0.1.1 | BSD-3-Clause | Gedgy Gedgy, Kyle Machulis |
| [jobserver](https://github.com/rust-lang/jobserver-rs) | 0.1.35 | MIT OR Apache-2.0 | Alex Crichton |
| [jpeg-encoder](https://github.com/vstroebel/jpeg-encoder) | 0.6.1 | (MIT OR Apache-2.0) AND IJG | Volker Ströbel |
| [js-sys](https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/js-sys) | 0.3.106 | MIT OR Apache-2.0 | The wasm-bindgen Developers |
| [kamadak-exif](https://github.com/kamadak/exif-rs) | 0.6.1 | BSD-2-Clause | KAMADA Ken'ichi |
| [keccak](https://github.com/RustCrypto/sponges) | 0.2.2 | Apache-2.0 OR MIT | RustCrypto Developers |
| [keyboard-types](https://github.com/pyfisch/keyboard-types) | 0.7.0 | MIT OR Apache-2.0 | Pyfisch |
| [khronos-egl](https://github.com/timothee-haudebourg/khronos-egl) | 6.0.0 | MIT OR Apache-2.0 | Timothée Haudebourg, Sean Kerr |
| [khronos_api](https://github.com/brendanzab/gl-rs/) | 3.1.0 | Apache-2.0 | Brendan Zabarauskas, Corey Richardson, Arseny Kapoulkine, Pierre Krieger |
| [krilla](https://github.com/LaurenzV/krilla) | 0.8.2 | MIT OR Apache-2.0 | Laurenz Stampfl |
| [kurbo](https://github.com/linebender/kurbo) | 0.13.1 | Apache-2.0 OR MIT | — |
| [lazy_static](https://github.com/rust-lang-nursery/lazy-static.rs) | 1.5.1 | MIT OR Apache-2.0 | Marvin Löbel |
| [lebe](https://github.com/johannesvollmer/lebe) | 0.5.3 | BSD-3-Clause | johannesvollmer |
| [libc](https://github.com/rust-lang/libc) | 0.2.191 | MIT OR Apache-2.0 | — |
| [libdbus-sys](https://github.com/diwic/dbus-rs) | 0.2.7 | Apache-2.0 OR MIT | David Henningsson |
| [libloading](https://github.com/nagisa/rust_libloading/) | 0.8.9 | ISC | Simonas Kazlauskas |
| [libm](https://github.com/rust-lang/compiler-builtins) | 0.2.16 | MIT | Alex Crichton, Amanieu d'Antras, Jorge Aparicio, Trevor Gross |
| [libredox](https://gitlab.redox-os.org/redox-os/libredox.git) | 0.1.25 | MIT | 4lDO2 |
| [linebender_resource_handle](https://github.com/linebender/raw_resource_handle) | 0.1.1 | Apache-2.0 OR MIT | — |
| [linesweeper](https://github.com/jneem/linesweeper) | 0.5.0 | MIT OR Apache-2.0 | Joe Neeman |
| [linked-hash-map](https://github.com/contain-rs/linked-hash-map) | 0.5.6 | MIT OR Apache-2.0 | Stepan Koltsov, Andrew Paseltiner |
| [linux-raw-sys](https://github.com/sunfishcode/linux-raw-sys) | 0.4.15 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | Dan Gohman |
| [linux-raw-sys](https://github.com/sunfishcode/linux-raw-sys) | 0.12.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | Dan Gohman |
| [lipsum](https://github.com/mgeisler/lipsum/) | 0.9.1 | MIT | Martin Geisler |
| [litemap](https://github.com/unicode-org/icu4x) | 0.8.3 | Unicode-3.0 | The ICU4X Project Developers |
| [litrs](https://github.com/LukasKalbertodt/litrs) | 1.0.1 | MIT OR Apache-2.0 | Lukas Kalbertodt |
| [lock_api](https://github.com/Amanieu/parking_lot) | 0.4.14 | MIT OR Apache-2.0 | Amanieu d'Antras |
| [log](https://github.com/rust-lang/log) | 0.4.34 | MIT OR Apache-2.0 | The Rust Project Developers |
| [logos](https://github.com/maciejhirsz/logos) | 0.13.0 | MIT OR Apache-2.0 | Maciej Hirsz |
| [logos-codegen](https://github.com/maciejhirsz/logos) | 0.13.0 | MIT OR Apache-2.0 | Maciej Hirsz |
| [logos-derive](https://github.com/maciejhirsz/logos) | 0.13.0 | MIT OR Apache-2.0 | Maciej Hirsz |
| [lopdf](https://github.com/J-F-Liu/lopdf.git) | 0.45.0 | MIT | Junfeng Liu, Emulator |
| [lru-slab](https://github.com/Ralith/lru-slab) | 0.1.3 | MIT OR Apache-2.0 OR Zlib | Benjamin Saunders |
| [malloc_buf](https://github.com/SSheldon/malloc_buf) | 0.0.6 | MIT | Steven Sheldon |
| [markup5ever](https://github.com/servo/html5ever) | 0.39.0 | MIT OR Apache-2.0 | The html5ever Project Developers |
| [matchers](https://github.com/hawkw/matchers) | 0.2.0 | MIT | Eliza Weisman |
| [md-5](https://github.com/RustCrypto/hashes) | 0.10.6 | MIT OR Apache-2.0 | RustCrypto Developers |
| [md-5](https://github.com/RustCrypto/hashes) | 0.11.0 | MIT OR Apache-2.0 | RustCrypto Developers |
| [memchr](https://github.com/BurntSushi/memchr) | 2.8.3 | Unlicense OR MIT | Andrew Gallant, bluss |
| [memmap2](https://github.com/RazrFalcon/memmap2-rs) | 0.9.11 | MIT OR Apache-2.0 | Dan Burkert, Yevhenii Reizner, The Contributors |
| [memoffset](https://github.com/Gilnaa/memoffset) | 0.9.1 | MIT | Gilad Naaman |
| [mime](https://github.com/hyperium/mime) | 0.3.17 | MIT OR Apache-2.0 | Sean McArthur |
| [miniz_oxide](https://github.com/Frommi/miniz_oxide/tree/master/miniz_oxide) | 0.8.9 | MIT OR Zlib OR Apache-2.0 | Frommi, oyvindln, Rich Geldreich richgel99@gmail.com |
| [miniz_oxide](https://github.com/Frommi/miniz_oxide/tree/master/miniz_oxide) | 0.9.1 | MIT OR Zlib OR Apache-2.0 | Frommi, oyvindln, Rich Geldreich richgel99@gmail.com |
| [mio](https://github.com/tokio-rs/mio) | 1.2.4 | MIT | Carl Lerche, Thomas de Zeeuw, Tokio Contributors |
| [mitex](https://github.com/mitex-rs/mitex) | 0.2.4 | Apache-2.0 | Myriad-Dreamin, OrangeX4, mgt |
| [mitex-glob](https://github.com/mitex-rs/mitex) | 0.2.4 | Apache-2.0 | Myriad-Dreamin, OrangeX4, mgt |
| [mitex-lexer](https://github.com/mitex-rs/mitex) | 0.2.4 | Apache-2.0 | Myriad-Dreamin, OrangeX4, mgt |
| [mitex-parser](https://github.com/mitex-rs/mitex) | 0.2.4 | Apache-2.0 | Myriad-Dreamin, OrangeX4, mgt |
| [mitex-spec](https://github.com/mitex-rs/mitex) | 0.2.4 | Apache-2.0 | Myriad-Dreamin, OrangeX4, mgt |
| [mitex-spec-gen](https://github.com/mitex-rs/mitex) | 0.2.4 | Apache-2.0 | Myriad-Dreamin, OrangeX4, mgt |
| [moxcms](https://github.com/awxkee/moxcms.git) | 0.8.1 | BSD-3-Clause OR Apache-2.0 | Radzivon Bartoshyk |
| [muda](https://github.com/tauri-apps/muda) | 0.17.2 | Apache-2.0 OR MIT | — |
| [multiversion_no_op](https://github.com/hsivonen/multiversion_no_op) | 1.0.0 | Apache-2.0 OR MIT | Henri Sivonen |
| [mutate_once](https://github.com/kamadak/mutate_once-rs) | 0.1.2 | BSD-2-Clause | KAMADA Ken'ichi |
| [naga](https://github.com/gfx-rs/wgpu) | 30.0.1 | MIT OR Apache-2.0 | gfx-rs developers |
| [naga-types](https://github.com/gfx-rs/wgpu) | 30.0.1 | MIT OR Apache-2.0 | gfx-rs developers |
| [ndk](https://github.com/rust-mobile/ndk) | 0.9.0 | MIT OR Apache-2.0 | The Rust Mobile contributors |
| [ndk-context](https://github.com/rust-windowing/android-ndk-rs) | 0.1.1 | MIT OR Apache-2.0 | The Rust Windowing contributors |
| [ndk-sys](https://github.com/rust-mobile/ndk) | 0.6.0+11769913 | MIT OR Apache-2.0 | The Rust Windowing contributors |
| [new_debug_unreachable](https://github.com/mbrubeck/rust-debug-unreachable) | 1.0.6 | MIT | Matt Brubeck, Jonathan Reem |
| [nohash-hasher](https://github.com/paritytech/nohash-hasher) | 0.2.0 | Apache-2.0 OR MIT | Parity Technologies |
| [nom](https://github.com/rust-bakery/nom) | 8.0.0 | MIT | contact@geoffroycouprie.com |
| [nu-ansi-term](https://github.com/nushell/nu-ansi-term) | 0.50.3 | MIT | ogham@bsago.me, Ryan Scheel (Havvy), Josh Triplett, The Nushell Project Developers |
| [num-bigint](https://github.com/rust-num/num-bigint) | 0.4.8 | MIT OR Apache-2.0 | The Rust Project Developers |
| [num-bigint](https://github.com/rust-num/num-bigint) | 0.5.1 | MIT OR Apache-2.0 | — |
| [num-complex](https://github.com/rust-num/num-complex) | 0.4.6 | MIT OR Apache-2.0 | The Rust Project Developers |
| [num-conv](https://github.com/jhpratt/num-conv) | 0.2.2 | MIT OR Apache-2.0 | Jacob Pratt |
| [num-integer](https://github.com/rust-num/num-integer) | 0.1.47 | MIT OR Apache-2.0 | The Rust Project Developers |
| [num-traits](https://github.com/rust-num/num-traits) | 0.2.19 | MIT OR Apache-2.0 | The Rust Project Developers |
| [num_cpus](https://github.com/seanmonstar/num_cpus) | 1.17.0 | MIT OR Apache-2.0 | Sean McArthur |
| [num_enum](https://github.com/illicitonion/num_enum) | 0.7.6 | BSD-3-Clause OR MIT OR Apache-2.0 | Daniel Wagner-Hall, Daniel Henry-Mantilla, Vincent Esche |
| [num_enum_derive](https://github.com/illicitonion/num_enum) | 0.7.6 | BSD-3-Clause OR MIT OR Apache-2.0 | Daniel Wagner-Hall, Daniel Henry-Mantilla, Vincent Esche |
| [num_threads](https://github.com/jhpratt/num_threads) | 0.1.8 | MIT OR Apache-2.0 | Jacob Pratt |
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
| [object](https://github.com/gimli-rs/object) | 0.39.1 | Apache-2.0 OR MIT | — |
| [ocrs](https://github.com/robertknight/ocrs) | 0.13.1 | MIT OR Apache-2.0 | Robert Knight |
| [once_cell](https://github.com/matklad/once_cell) | 1.21.4 | MIT OR Apache-2.0 | Aleksey Kladov |
| [once_cell_polyfill](https://github.com/polyfill-rs/once_cell_polyfill) | 1.70.2 | MIT OR Apache-2.0 | — |
| [oneshot](https://github.com/faern/oneshot) | 0.2.1 | MIT OR Apache-2.0 | Linus Färnstrand |
| [openssl-probe](https://github.com/rustls/openssl-probe) | 0.2.1 | MIT OR Apache-2.0 | Alex Crichton |
| [option-ext](https://github.com/soc/option-ext.git) | 0.2.0 | MPL-2.0 | Simon Ochsenreither |
| [orbclient](https://gitlab.redox-os.org/redox-os/orbclient) | 0.3.55 | MIT | Jeremy Soller |
| [ordered-channel](https://gitlab.com/kornelski/ordered-channel) | 1.2.0 | MIT OR Apache-2.0 | — |
| [ordered-float](https://github.com/reem/rust-ordered-float) | 5.5.0 | MIT | Jonathan Reem, Matt Brubeck |
| [ordered-stream](https://github.com/danieldg/ordered-stream) | 0.2.0 | MIT OR Apache-2.0 | Daniel De Graaf, Zeeshan Ali Khan |
| [owned_ttf_parser](https://github.com/alexheretic/owned-ttf-parser) | 0.25.1 | Apache-2.0 | Alex Butler |
| [p256](https://github.com/RustCrypto/elliptic-curves) | 0.14.0 | Apache-2.0 OR MIT | RustCrypto Developers |
| [p384](https://github.com/RustCrypto/elliptic-curves) | 0.14.0 | Apache-2.0 OR MIT | RustCrypto Developers, Frank Denis |
| [p521](https://github.com/RustCrypto/elliptic-curves) | 0.14.0 | Apache-2.0 OR MIT | RustCrypto Developers |
| [palette](https://github.com/Ogeon/palette) | 0.7.7 | MIT OR Apache-2.0 | Erik Hedvall |
| [palette_derive](https://github.com/Ogeon/palette) | 0.7.7 | MIT OR Apache-2.0 | Erik Hedvall |
| [palette_math](https://github.com/Ogeon/palette) | 0.7.7 | MIT OR Apache-2.0 | Erik Hedvall |
| [pango](https://github.com/gtk-rs/gtk-rs-core) | 0.18.3 | MIT | The gtk-rs Project Developers |
| [pango-sys](https://github.com/gtk-rs/gtk-rs-core) | 0.18.0 | MIT | The gtk-rs Project Developers |
| [parking](https://github.com/smol-rs/parking) | 2.2.1 | Apache-2.0 OR MIT | Stjepan Glavina, The Rust Project Developers |
| [parking_lot](https://github.com/Amanieu/parking_lot) | 0.12.5 | MIT OR Apache-2.0 | Amanieu d'Antras |
| [parking_lot_core](https://github.com/Amanieu/parking_lot) | 0.9.12 | MIT OR Apache-2.0 | Amanieu d'Antras |
| [paste](https://github.com/dtolnay/paste) | 1.0.15 | MIT OR Apache-2.0 | David Tolnay |
| [pastey](https://github.com/as1100k/pastey) | 0.2.3 | MIT OR Apache-2.0 | Aditya Kumar, David Tolnay |
| [pbkdf2](https://github.com/RustCrypto/password-hashes) | 0.13.0 | MIT OR Apache-2.0 | RustCrypto Developers |
| [pdf-writer](https://github.com/typst/pdf-writer) | 0.15.0 | MIT OR Apache-2.0 | Laurenz, Martin Haug |
| [pdfcraft-a11y](https://github.com/storytold/pdfcraft) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-annot](https://crates.io/crates/pdfcraft-annot) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-audit](https://github.com/storytold/pdfcraft) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-automation](https://crates.io/crates/pdfcraft-automation) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-compare](https://github.com/storytold/pdfcraft) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-content](https://crates.io/crates/pdfcraft-content) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-cos](https://crates.io/crates/pdfcraft-cos) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-create](https://crates.io/crates/pdfcraft-create) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-crypt](https://github.com/storytold/pdfcraft) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-edit](https://crates.io/crates/pdfcraft-edit) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-engine](https://crates.io/crates/pdfcraft-engine) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-export](https://github.com/storytold/pdfcraft) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-filters](https://github.com/storytold/pdfcraft) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-fonts](https://crates.io/crates/pdfcraft-fonts) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-forms](https://crates.io/crates/pdfcraft-forms) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-geom](https://crates.io/crates/pdfcraft-geom) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-js](https://github.com/storytold/pdfcraft) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-measure](https://crates.io/crates/pdfcraft-measure) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-model](https://crates.io/crates/pdfcraft-model) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-ocr](https://github.com/storytold/pdfcraft) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-optimize](https://github.com/storytold/pdfcraft) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-organize](https://crates.io/crates/pdfcraft-organize) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-platform](https://crates.io/crates/pdfcraft-platform) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-preflight](https://github.com/storytold/pdfcraft) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-print](https://crates.io/crates/pdfcraft-print) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-redact](https://crates.io/crates/pdfcraft-redact) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-render](https://crates.io/crates/pdfcraft-render) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-sign](https://github.com/storytold/pdfcraft) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-xfa](https://github.com/storytold/pdfcraft) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pdfcraft-xfdf](https://crates.io/crates/pdfcraft-xfdf) | 0.9.0 | MIT OR Apache-2.0 | — |
| [pem](https://github.com/jcreekmore/pem-rs.git) | 3.0.6 | MIT | Jonathan Creekmore |
| [pem-rfc7468](https://github.com/RustCrypto/formats/tree/master/pem-rfc7468) | 0.7.0 | Apache-2.0 OR MIT | RustCrypto Developers |
| [pem-rfc7468](https://github.com/RustCrypto/formats) | 1.0.0 | Apache-2.0 OR MIT | RustCrypto Developers |
| [peniko](https://github.com/linebender/peniko) | 0.6.1 | Apache-2.0 OR MIT | — |
| [percent-encoding](https://github.com/servo/rust-url/) | 2.3.2 | MIT OR Apache-2.0 | The rust-url developers |
| [phf](https://github.com/rust-phf/rust-phf) | 0.13.1 | MIT | Steven Fackler |
| [phf](https://github.com/rust-phf/rust-phf) | 0.14.0 | MIT | Steven Fackler |
| [phf_codegen](https://github.com/rust-phf/rust-phf) | 0.13.1 | MIT | Steven Fackler |
| [phf_generator](https://github.com/rust-phf/rust-phf) | 0.13.1 | MIT | Steven Fackler |
| [phf_generator](https://github.com/rust-phf/rust-phf) | 0.14.0 | MIT | Steven Fackler |
| [phf_macros](https://github.com/rust-phf/rust-phf) | 0.13.1 | MIT | Steven Fackler |
| [phf_macros](https://github.com/rust-phf/rust-phf) | 0.14.0 | MIT | Steven Fackler |
| [phf_shared](https://github.com/rust-phf/rust-phf) | 0.13.1 | MIT | Steven Fackler |
| [phf_shared](https://github.com/rust-phf/rust-phf) | 0.14.0 | MIT | Steven Fackler |
| [photocraft-cms](https://crates.io/crates/photocraft-cms) | 0.11.1 | MIT OR Apache-2.0 | — |
| [photocraft-codecs](https://crates.io/crates/photocraft-codecs) | 0.11.1 | MIT OR Apache-2.0 | — |
| [photocraft-color](https://crates.io/crates/photocraft-color) | 0.11.1 | MIT OR Apache-2.0 | — |
| [photocraft-geom](https://crates.io/crates/photocraft-geom) | 0.11.1 | MIT OR Apache-2.0 | — |
| [photocraft-paint](https://crates.io/crates/photocraft-paint) | 0.11.1 | MIT OR Apache-2.0 | — |
| [photocraft-raster](https://crates.io/crates/photocraft-raster) | 0.11.1 | MIT OR Apache-2.0 | — |
| [pic-scale](https://github.com/awxkee/pic-scale) | 0.7.12 | BSD-3-Clause OR Apache-2.0 | Radzivon Bartoshyk |
| [pico-args](https://github.com/RazrFalcon/pico-args) | 0.5.0 | MIT | Yevhenii Reizner |
| [pin-project](https://github.com/taiki-e/pin-project) | 1.1.13 | Apache-2.0 OR MIT | — |
| [pin-project-internal](https://github.com/taiki-e/pin-project) | 1.1.13 | Apache-2.0 OR MIT | — |
| [pin-project-lite](https://github.com/taiki-e/pin-project-lite) | 0.2.17 | Apache-2.0 OR MIT | — |
| [piper](https://github.com/smol-rs/piper) | 0.2.5 | MIT OR Apache-2.0 | Stjepan Glavina, John Nunley |
| [pixglyph](https://github.com/typst/pixglyph) | 0.6.1 | Apache-2.0 | Laurenz |
| [pkcs1](https://github.com/RustCrypto/formats) | 0.8.0-rc.5 | Apache-2.0 OR MIT | RustCrypto Developers |
| [pkcs8](https://github.com/RustCrypto/formats) | 0.11.0 | Apache-2.0 OR MIT | RustCrypto Developers |
| [pkg-config](https://github.com/rust-lang/pkg-config-rs) | 0.3.34 | MIT OR Apache-2.0 | Alex Crichton |
| [plain](https://github.com/randomites/plain) | 0.2.3 | MIT OR Apache-2.0 | jzr |
| [plist](https://github.com/ebarnard/rust-plist/) | 1.10.2 | MIT | Ed Barnard |
| [png](https://github.com/image-rs/image-png) | 0.17.16 | MIT OR Apache-2.0 | The image-rs Developers |
| [png](https://github.com/image-rs/image-png) | 0.18.1 | MIT OR Apache-2.0 | The image-rs Developers |
| [polling](https://github.com/smol-rs/polling) | 3.11.0 | Apache-2.0 OR MIT | Stjepan Glavina, John Nunley |
| [pollster](https://github.com/zesterer/pollster) | 1.0.1 | Apache-2.0 OR MIT | Joshua Barretto |
| [polycool](https://github.com/linebender/kurbo) | 0.4.0 | MIT OR Apache-2.0 | — |
| [portable-atomic](https://github.com/taiki-e/portable-atomic) | 1.15.0 | Apache-2.0 OR MIT | — |
| [portable-atomic-util](https://github.com/taiki-e/portable-atomic-util) | 0.2.8 | Apache-2.0 OR MIT | — |
| [postcard](https://github.com/jamesmunns/postcard) | 1.1.3 | MIT OR Apache-2.0 | James Munns |
| [potential_utf](https://github.com/unicode-org/icu4x) | 0.1.6 | Unicode-3.0 | The ICU4X Project Developers |
| [powerfmt](https://github.com/jhpratt/powerfmt) | 0.2.1 | MIT OR Apache-2.0 | Jacob Pratt |
| [ppv-lite86](https://github.com/cryptocorrosion/cryptocorrosion) | 0.2.21 | MIT OR Apache-2.0 | The CryptoCorrosion Contributors |
| [precomputed-hash](https://github.com/emilio/precomputed-hash) | 0.1.1 | MIT | Emilio Cobos Álvarez |
| [presser](https://github.com/EmbarkStudios/presser) | 0.3.1 | MIT OR Apache-2.0 | Embark, Gray Olson <gray@grayolson.com |
| [primefield](https://github.com/RustCrypto/elliptic-curves) | 0.14.0 | Apache-2.0 OR MIT | RustCrypto Developers |
| [primeorder](https://github.com/RustCrypto/elliptic-curves) | 0.14.0 | Apache-2.0 OR MIT | RustCrypto Developers |
| [proc-macro-crate](https://github.com/bkchr/proc-macro-crate) | 1.3.1 | MIT OR Apache-2.0 | Bastian Köcher |
| [proc-macro-crate](https://github.com/bkchr/proc-macro-crate) | 2.0.2 | MIT OR Apache-2.0 | Bastian Köcher |
| [proc-macro-crate](https://github.com/bkchr/proc-macro-crate) | 3.5.0 | MIT OR Apache-2.0 | Bastian Köcher |
| [proc-macro-error](https://gitlab.com/CreepySkeleton/proc-macro-error) | 1.0.4 | MIT OR Apache-2.0 | CreepySkeleton |
| [proc-macro-error-attr](https://gitlab.com/CreepySkeleton/proc-macro-error) | 1.0.4 | MIT OR Apache-2.0 | CreepySkeleton |
| [proc-macro-hack](https://github.com/dtolnay/proc-macro-hack) | 0.5.20+deprecated | MIT OR Apache-2.0 | David Tolnay |
| [proc-macro2](https://github.com/dtolnay/proc-macro2) | 1.0.107 | MIT OR Apache-2.0 | David Tolnay, Alex Crichton |
| [profiling](https://github.com/aclysma/profiling) | 1.0.18 | MIT OR Apache-2.0 | Philip Degarmo |
| [psm](https://github.com/rust-lang/stacker/) | 0.1.32 | MIT OR Apache-2.0 | Simonas Kazlauskas |
| [ptr_meta](https://github.com/djkoloski/ptr_meta) | 0.1.4 | MIT | David Koloski |
| [ptr_meta_derive](https://github.com/djkoloski/ptr_meta) | 0.1.4 | MIT | David Koloski |
| [pulp](https://github.com/sarah-quinones/pulp/) | 0.22.3 | MIT | sarah quiñones |
| [pulp-wasm-simd-flag](https://github.com/sarah-quinones/pulp/) | 0.1.1 | MIT | sarah quiñones |
| [pxfm](https://github.com/awxkee/pxfm) | 0.1.30 | BSD-3-Clause OR Apache-2.0 | Radzivon Bartoshyk |
| [qoi](https://github.com/aldanor/qoi-rust) | 0.4.1 | MIT OR Apache-2.0 | Ivan Smirnov |
| [quick-error](http://github.com/tailhook/quick-error) | 2.0.1 | MIT OR Apache-2.0 | Paul Colomiets, Colin Kiegel |
| [quick-xml](https://github.com/tafia/quick-xml) | 0.38.4 | MIT | — |
| [quick-xml](https://github.com/tafia/quick-xml) | 0.41.0 | MIT | — |
| [quick-xml](https://github.com/tafia/quick-xml) | 0.42.0 | MIT | — |
| [quinn](https://github.com/quinn-rs/quinn) | 0.11.12 | MIT OR Apache-2.0 | — |
| [quinn-proto](https://github.com/quinn-rs/quinn) | 0.11.19 | MIT OR Apache-2.0 | — |
| [quinn-udp](https://github.com/quinn-rs/quinn) | 0.5.16 | MIT OR Apache-2.0 | — |
| [quote](https://github.com/dtolnay/quote) | 1.0.47 | MIT OR Apache-2.0 | David Tolnay |
| [r-efi](https://github.com/r-efi/r-efi) | 5.3.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | — |
| [r-efi](https://github.com/r-efi/r-efi) | 6.0.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | — |
| [radium](https://github.com/bitvecto-rs/radium) | 0.7.0 | MIT | Nika Layzell, myrrlyn |
| [rand](https://github.com/rust-random/rand) | 0.8.8 | MIT OR Apache-2.0 | The Rand Project Developers, The Rust Project Developers |
| [rand](https://github.com/rust-random/rand) | 0.10.3 | MIT OR Apache-2.0 | The Rand Project Developers, The Rust Project Developers |
| [rand_chacha](https://github.com/rust-random/rand) | 0.3.1 | MIT OR Apache-2.0 | The Rand Project Developers, The Rust Project Developers, The CryptoCorrosion Contributors |
| [rand_core](https://github.com/rust-random/rand) | 0.6.4 | MIT OR Apache-2.0 | The Rand Project Developers, The Rust Project Developers |
| [rand_core](https://github.com/rust-random/rand_core) | 0.10.1 | MIT OR Apache-2.0 | The Rand Project Developers |
| [rand_pcg](https://github.com/rust-random/rngs) | 0.10.2 | MIT OR Apache-2.0 | The Rand Project Developers |
| [range-alloc](https://github.com/gfx-rs/range-alloc) | 0.1.5 | MIT OR Apache-2.0 | the gfx-rs Developers |
| [rangemap](https://github.com/jeffparsons/rangemap) | 1.8.0 | MIT OR Apache-2.0 | Jeff Parsons |
| [raw-cpuid](https://github.com/gz/rust-cpuid) | 11.6.0 | MIT | Gerd Zellweger |
| [raw-window-handle](https://github.com/rust-windowing/raw-window-handle) | 0.6.2 | MIT OR Apache-2.0 OR Zlib | Osspial |
| [raw-window-metal](https://github.com/rust-windowing/raw-window-metal) | 1.1.0 | MIT OR Apache-2.0 | — |
| [rayon](https://github.com/rayon-rs/rayon) | 1.12.0 | MIT OR Apache-2.0 | — |
| [rayon-core](https://github.com/rayon-rs/rayon) | 1.13.0 | MIT OR Apache-2.0 | — |
| [rc2](https://github.com/RustCrypto/block-ciphers) | 0.9.0 | MIT OR Apache-2.0 | RustCrypto Developers |
| [read-fonts](https://github.com/googlefonts/fontations) | 0.39.2 | MIT OR Apache-2.0 | — |
| [read-fonts](https://github.com/googlefonts/fontations) | 0.41.0 | MIT OR Apache-2.0 | — |
| [read-fonts](https://github.com/googlefonts/fontations) | 0.43.3 | MIT OR Apache-2.0 | — |
| [read-fonts](https://github.com/googlefonts/fontations) | 0.44.0 | MIT OR Apache-2.0 | — |
| [reborrow](https://github.com/sarah-ek/reborrow/) | 0.5.5 | MIT | sarah |
| [redox_syscall](https://gitlab.redox-os.org/redox-os/syscall) | 0.4.1 | MIT | Jeremy Soller |
| [redox_syscall](https://gitlab.redox-os.org/redox-os/syscall) | 0.5.18 | MIT | Jeremy Soller |
| [redox_syscall](https://gitlab.redox-os.org/redox-os/kernel) | 0.9.5 | MIT | Jeremy Soller |
| [redox_users](https://gitlab.redox-os.org/redox-os/users) | 0.5.3 | MIT | Jose Narvaez, Wesley Hershberger |
| [regex](https://github.com/rust-lang/regex) | 1.13.1 | MIT OR Apache-2.0 | The Rust Project Developers, Andrew Gallant |
| [regex-automata](https://github.com/rust-lang/regex) | 0.4.18 | MIT OR Apache-2.0 | The Rust Project Developers, Andrew Gallant |
| [regex-syntax](https://github.com/rust-lang/regex) | 0.6.29 | MIT OR Apache-2.0 | The Rust Project Developers |
| [regex-syntax](https://github.com/rust-lang/regex) | 0.8.11 | MIT OR Apache-2.0 | The Rust Project Developers, Andrew Gallant |
| [regress](https://github.com/ridiculousfish/regress) | 0.12.0 | MIT OR Apache-2.0 | ridiculousfish |
| [rend](https://github.com/djkoloski/rend) | 0.4.2 | MIT | David Koloski |
| [renderdoc-sys](https://github.com/ebkalderon/renderdoc-rs) | 1.1.0 | MIT OR Apache-2.0 | Eyal Kalderon |
| [reqwest](https://github.com/seanmonstar/reqwest) | 0.13.5 | MIT OR Apache-2.0 | Sean McArthur |
| [resvg](https://github.com/linebender/resvg) | 0.47.0 | Apache-2.0 OR MIT | — |
| [rfc6979](https://github.com/RustCrypto/signatures) | 0.6.0 | Apache-2.0 OR MIT | RustCrypto Developers |
| [rfd](https://github.com/PolyMeilex/rfd) | 0.14.1 | MIT | Poly |
| [rgb](https://github.com/kornelski/rust-rgb) | 0.8.53 | MIT | Kornel Lesiński, James Forster |
| [ring](https://github.com/briansmith/ring) | 0.17.14 | Apache-2.0 AND ISC | — |
| [ripemd](https://github.com/RustCrypto/hashes) | 0.2.0 | MIT OR Apache-2.0 | RustCrypto Developers |
| [rkyv](https://github.com/rkyv/rkyv) | 0.7.46 | MIT | David Koloski |
| [rkyv_derive](https://github.com/rkyv/rkyv) | 0.7.46 | MIT | David Koloski |
| [roman-numerals-rs](https://github.com/AA-Turner/roman-numerals/) | 3.1.0 | 0BSD OR CC0-1.0 | Adam Turner |
| [rowan](https://github.com/rust-analyzer/rowan) | 0.15.19 | MIT OR Apache-2.0 | Aleksey Kladov |
| [roxmltree](https://github.com/RazrFalcon/roxmltree) | 0.20.0 | MIT OR Apache-2.0 | Yevhenii Reizner |
| [roxmltree](https://github.com/RazrFalcon/roxmltree) | 0.21.1 | MIT OR Apache-2.0 | Yevhenii Reizner |
| [rsa](https://github.com/RustCrypto/RSA) | 0.10.0-rc.18 | MIT OR Apache-2.0 | RustCrypto Developers, dignifiedquire |
| [rten](https://github.com/robertknight/rten) | 0.26.0 | MIT OR Apache-2.0 | Robert Knight |
| [rten-base](https://github.com/robertknight/rten) | 0.26.0 | MIT OR Apache-2.0 | Robert Knight |
| [rten-gemm](https://github.com/robertknight/rten) | 0.26.0 | MIT OR Apache-2.0 | Robert Knight |
| [rten-imageproc](https://github.com/robertknight/rten) | 0.26.0 | MIT OR Apache-2.0 | Robert Knight |
| [rten-model-file](https://github.com/robertknight/rten) | 0.26.0 | MIT OR Apache-2.0 | Robert Knight |
| [rten-parallel](https://github.com/robertknight/rten) | 0.26.0 | MIT OR Apache-2.0 | Robert Knight |
| [rten-shape-inference](https://github.com/robertknight/rten) | 0.26.0 | MIT OR Apache-2.0 | Robert Knight |
| [rten-simd](https://github.com/robertknight/rten) | 0.26.0 | MIT OR Apache-2.0 | Robert Knight |
| [rten-tensor](https://github.com/robertknight/rten) | 0.26.0 | MIT OR Apache-2.0 | Robert Knight |
| [rten-vecmath](https://github.com/robertknight/rten) | 0.26.0 | MIT OR Apache-2.0 | Robert Knight |
| [rust_decimal](https://github.com/paupino/rust-decimal) | 1.43.0 | MIT | Paul Mason |
| [rustc-hash](https://github.com/rust-lang-nursery/rustc-hash) | 1.1.0 | Apache-2.0 OR MIT | The Rust Project Developers |
| [rustc-hash](https://github.com/rust-lang/rustc-hash) | 2.1.3 | Apache-2.0 OR MIT | The Rust Project Developers |
| [rustc_version](https://github.com/djc/rustc-version-rs) | 0.4.1 | MIT OR Apache-2.0 | — |
| [rustix](https://github.com/bytecodealliance/rustix) | 0.38.44 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | Dan Gohman, Jakub Konka |
| [rustix](https://github.com/bytecodealliance/rustix) | 1.1.5 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | Dan Gohman, Jakub Konka |
| [rustls](https://github.com/rustls/rustls) | 0.23.46 | Apache-2.0 OR ISC OR MIT | — |
| [rustls-cng](https://github.com/rustls/rustls-cng) | 0.7.1 | MIT OR Apache-2.0 | Dmitry Pankratov |
| [rustls-native-certs](https://github.com/rustls/rustls-native-certs) | 0.8.5 | Apache-2.0 OR ISC OR MIT | — |
| [rustls-pki-types](https://github.com/rustls/pki-types) | 1.15.1 | MIT OR Apache-2.0 | — |
| [rustls-platform-verifier](https://github.com/rustls/rustls-platform-verifier) | 0.7.1 | MIT OR Apache-2.0 | — |
| [rustls-platform-verifier-android](https://github.com/rustls/rustls-platform-verifier) | 0.2.0 | MIT OR Apache-2.0 | — |
| [rustls-webpki](https://github.com/rustls/webpki) | 0.103.15 | ISC | — |
| [rustversion](https://github.com/dtolnay/rustversion) | 1.0.23 | MIT OR Apache-2.0 | David Tolnay |
| [rustybuzz](https://github.com/harfbuzz/rustybuzz) | 0.20.1 | MIT | Caleb Maclennan, Laurenz Stampfl, Yevhenii Reizner, خالد حسني (Khaled Hosny) |
| [ruzstd](https://github.com/KillingSpark/zstd-rs) | 0.9.1 | MIT | Moritz Borcherding |
| [ryu](https://github.com/dtolnay/ryu) | 1.0.23 | Apache-2.0 OR BSL-1.0 | David Tolnay |
| [ryu-js](https://github.com/boa-dev/ryu-js) | 1.0.3 | Apache-2.0 OR BSL-1.0 | David Tolnay, boa-dev |
| [same-file](https://github.com/BurntSushi/same-file) | 1.0.6 | Unlicense OR MIT | Andrew Gallant |
| [schannel](https://github.com/steffengy/schannel-rs) | 0.1.30 | MIT | Steven Fackler, Steffen Butzer |
| [scoped-tls](https://github.com/alexcrichton/scoped-tls) | 1.0.1 | MIT OR Apache-2.0 | Alex Crichton |
| [scopeguard](https://github.com/bluss/scopeguard) | 1.2.0 | MIT OR Apache-2.0 | bluss |
| [sctk-adwaita](https://github.com/PolyMeilex/sctk-adwaita) | 0.10.1 | MIT | Poly |
| [seahash](https://gitlab.redox-os.org/redox-os/seahash) | 4.1.0 | MIT | ticki, Tom Almeida |
| [sec1](https://github.com/RustCrypto/formats) | 0.8.1 | Apache-2.0 OR MIT | RustCrypto Developers |
| [security-framework](https://github.com/kornelski/rust-security-framework) | 3.7.0 | MIT OR Apache-2.0 | Steven Fackler, Kornel |
| [security-framework-sys](https://github.com/kornelski/rust-security-framework) | 2.17.0 | MIT OR Apache-2.0 | Steven Fackler, Kornel |
| [selectors](https://github.com/servo/stylo) | 0.38.0 | MPL-2.0 | The Servo Project Developers |
| [self_cell](https://github.com/Voultapher/self_cell) | 1.3.0 | Apache-2.0 OR GPL-2.0-only | Lukas Bergdoll |
| [semver](https://github.com/dtolnay/semver) | 1.0.28 | MIT OR Apache-2.0 | David Tolnay |
| [serde](https://github.com/serde-rs/serde) | 1.0.229 | MIT OR Apache-2.0 | Erick Tryzelaar, David Tolnay |
| [serde-xml-rs](https://github.com/RReverser/serde-xml-rs) | 0.8.2 | MIT | Ingvar Stepanyan, William Bartlett |
| [serde_core](https://github.com/serde-rs/serde) | 1.0.229 | MIT OR Apache-2.0 | Erick Tryzelaar, David Tolnay |
| [serde_derive](https://github.com/serde-rs/serde) | 1.0.229 | MIT OR Apache-2.0 | Erick Tryzelaar, David Tolnay |
| [serde_json](https://github.com/serde-rs/json) | 1.0.154 | MIT OR Apache-2.0 | — |
| [serde_json_canonicalizer](https://github.com/evik42/serde-json-canonicalizer) | 0.3.2 | MIT | — |
| [serde_path_to_error](https://github.com/dtolnay/path-to-error) | 0.1.20 | MIT OR Apache-2.0 | David Tolnay |
| [serde_repr](https://github.com/dtolnay/serde-repr) | 0.1.21 | MIT OR Apache-2.0 | David Tolnay |
| [serde_spanned](https://github.com/toml-rs/toml) | 0.6.9 | MIT OR Apache-2.0 | — |
| [serde_urlencoded](https://github.com/nox/serde_urlencoded) | 0.7.1 | MIT OR Apache-2.0 | Anthony Ramine |
| [serde_yaml](https://github.com/dtolnay/serde-yaml) | 0.9.34+deprecated | MIT OR Apache-2.0 | David Tolnay |
| [serdect](https://github.com/RustCrypto/formats) | 0.4.3 | Apache-2.0 OR MIT | RustCrypto Developers |
| [servo_arc](https://github.com/servo/stylo) | 0.4.3 | MIT OR Apache-2.0 | The Servo Project Developers |
| [sha1](https://github.com/RustCrypto/hashes) | 0.10.7 | MIT OR Apache-2.0 | RustCrypto Developers |
| [sha1](https://github.com/RustCrypto/hashes) | 0.11.0 | MIT OR Apache-2.0 | RustCrypto Developers |
| [sha2](https://github.com/RustCrypto/hashes) | 0.10.9 | MIT OR Apache-2.0 | RustCrypto Developers |
| [sha2](https://github.com/RustCrypto/hashes) | 0.11.1 | MIT OR Apache-2.0 | RustCrypto Developers |
| [sha3](https://github.com/RustCrypto/hashes) | 0.12.0 | MIT OR Apache-2.0 | RustCrypto Developers |
| [sharded-slab](https://github.com/hawkw/sharded-slab) | 0.1.7 | MIT | Eliza Weisman |
| [shlex](https://github.com/comex/rust-shlex) | 2.0.1 | MIT OR Apache-2.0 | comex, Fenhl, Adrian Taylor, Alex Touchet, Daniel Parks, Garrett Berg |
| [signal-hook-registry](https://github.com/vorner/signal-hook) | 1.4.8 | MIT OR Apache-2.0 | Michal 'vorner' Vaner, Masaki Hara |
| [signature](https://github.com/RustCrypto/traits/tree/master/signature) | 2.2.0 | Apache-2.0 OR MIT | RustCrypto Developers |
| [signature](https://github.com/RustCrypto/traits) | 3.0.0 | Apache-2.0 OR MIT | RustCrypto Developers |
| [sigstore-bundle](https://github.com/prefix-dev/sigstore-rust) | 0.6.6 | Apache-2.0 | — |
| [sigstore-crypto](https://github.com/prefix-dev/sigstore-rust) | 0.6.6 | Apache-2.0 | — |
| [sigstore-merkle](https://github.com/prefix-dev/sigstore-rust) | 0.6.6 | Apache-2.0 | — |
| [sigstore-rekor](https://github.com/prefix-dev/sigstore-rust) | 0.6.6 | Apache-2.0 | — |
| [sigstore-trust-root](https://github.com/prefix-dev/sigstore-rust) | 0.6.6 | Apache-2.0 | — |
| [sigstore-tsa](https://github.com/prefix-dev/sigstore-rust) | 0.6.6 | Apache-2.0 | — |
| [sigstore-types](https://github.com/prefix-dev/sigstore-rust) | 0.6.6 | Apache-2.0 | — |
| [sigstore-verify](https://github.com/prefix-dev/sigstore-rust) | 0.6.6 | Apache-2.0 | — |
| [simd-adler32](https://github.com/mcountryman/simd-adler32) | 0.3.10 | MIT | Marvin Countryman |
| [simd_cesu8](https://github.com/seancroach/simd_cesu8) | 1.2.0 | Apache-2.0 OR MIT | Sean C. Roach |
| [simdutf8](https://github.com/rusticstuff/simdutf8) | 0.1.5 | MIT OR Apache-2.0 | Hans Kratz |
| [simplecss](https://github.com/linebender/simplecss) | 0.2.2 | Apache-2.0 OR MIT | — |
| [siphasher](https://github.com/jedisct1/rust-siphash) | 1.0.4 | MIT OR Apache-2.0 | Frank Denis |
| [skrifa](https://github.com/googlefonts/fontations) | 0.42.1 | MIT OR Apache-2.0 | — |
| [skrifa](https://github.com/googlefonts/fontations) | 0.44.0 | MIT OR Apache-2.0 | — |
| [skrifa](https://github.com/googlefonts/fontations) | 0.47.0 | MIT OR Apache-2.0 | — |
| [slab](https://github.com/tokio-rs/slab) | 0.4.12 | MIT | Carl Lerche |
| [slotmap](https://github.com/orlp/slotmap) | 1.1.1 | Zlib | Orson Peters |
| [small_btree](https://github.com/boa-dev/boa) | 0.1.0 | Unlicense OR MIT | boa-dev |
| [smallvec](https://github.com/servo/rust-smallvec) | 1.16.3 | MIT OR Apache-2.0 | The Servo Project Developers |
| [smithay-client-toolkit](https://github.com/smithay/client-toolkit) | 0.19.2 | MIT | Elinor Berger, i509VCB, Ashley Wulber |
| [smithay-client-toolkit](https://github.com/smithay/client-toolkit) | 0.20.0 | MIT | Elinor Berger, i509VCB, Ashley Wulber |
| [smithay-clipboard](https://github.com/smithay/smithay-clipboard) | 0.7.3 | MIT | Kirill Chibisov, Elinor Berger |
| [smol_str](https://github.com/rust-analyzer/smol_str) | 0.2.2 | MIT OR Apache-2.0 | Aleksey Kladov |
| [socket2](https://github.com/rust-lang/socket2) | 0.6.5 | MIT OR Apache-2.0 | Alex Crichton, Thomas de Zeeuw |
| [soup3](https://gitlab.gnome.org/World/Rust/soup3-rs) | 0.5.0 | MIT | — |
| [soup3-sys](https://gitlab.gnome.org/World/Rust/soup3-rs) | 0.5.0 | MIT | The Gtk-rs Project Developers |
| [spin](https://github.com/mvdnes/spin-rs.git) | 0.9.9 | MIT | Mathijs van de Nes, John Ericson, Joshua Barretto |
| [spirv](https://github.com/gfx-rs/rspirv) | 0.4.0+sdk-1.4.341.0 | Apache-2.0 | Lei Zhang |
| [spki](https://github.com/RustCrypto/formats/tree/master/spki) | 0.7.3 | Apache-2.0 OR MIT | RustCrypto Developers |
| [spki](https://github.com/RustCrypto/formats) | 0.8.1 | Apache-2.0 OR MIT | RustCrypto Developers |
| [sponge-cursor](https://github.com/RustCrypto/utils) | 0.1.0 | MIT OR Apache-2.0 | RustCrypto Developers |
| [stable_deref_trait](https://github.com/storyyeller/stable_deref_trait) | 1.2.1 | MIT OR Apache-2.0 | Robert Grosse |
| [stacker](https://github.com/rust-lang/stacker) | 0.1.25 | MIT OR Apache-2.0 | Alex Crichton, Simonas Kazlauskas |
| [static_assertions](https://github.com/nvzqz/static-assertions-rs) | 1.1.0 | MIT OR Apache-2.0 | Nikolai Vazquez |
| [strict-num](https://github.com/RazrFalcon/strict-num) | 0.1.1 | MIT | Yevhenii Reizner |
| [string-interner](https://github.com/robbepop/string-interner) | 0.19.0 | MIT OR Apache-2.0 | Robbepop |
| [string_cache](https://github.com/servo/string-cache) | 0.9.0 | MIT OR Apache-2.0 | The Servo Project Developers |
| [string_cache_codegen](https://github.com/servo/string-cache) | 0.6.1 | MIT OR Apache-2.0 | The Servo Project Developers |
| [stringprep](https://github.com/sfackler/rust-stringprep) | 0.1.5 | MIT OR Apache-2.0 | Steven Fackler |
| [strsim](https://github.com/rapidfuzz/strsim-rs) | 0.11.1 | MIT | Danny Guo, maxbachmann |
| [strum](https://github.com/Peternator7/strum) | 0.27.2 | MIT | Peter Glotfelty |
| [strum](https://github.com/Peternator7/strum) | 0.28.0 | MIT | Peter Glotfelty |
| [strum_macros](https://github.com/Peternator7/strum) | 0.27.2 | MIT | Peter Glotfelty |
| [strum_macros](https://github.com/Peternator7/strum) | 0.28.0 | MIT | Peter Glotfelty |
| [subsetter](https://github.com/typst/subsetter) | 0.2.6 | MIT OR Apache-2.0 | Laurenz Mädje, Laurenz Stampfl |
| [subtle](https://github.com/dalek-cryptography/subtle) | 2.6.1 | BSD-3-Clause | Isis Lovecruft, Henry de Valence |
| [svgtypes](https://github.com/linebender/svgtypes) | 0.16.1 | Apache-2.0 OR MIT | — |
| [syn](https://github.com/dtolnay/syn) | 1.0.109 | MIT OR Apache-2.0 | David Tolnay |
| [syn](https://github.com/dtolnay/syn) | 2.0.119 | MIT OR Apache-2.0 | David Tolnay |
| [syn](https://github.com/dtolnay/syn) | 3.0.9 | MIT OR Apache-2.0 | David Tolnay |
| [sync_wrapper](https://github.com/Actyx/sync_wrapper) | 1.0.2 | Apache-2.0 | Actyx AG |
| [synstructure](https://github.com/mystor/synstructure) | 0.14.0 | MIT | Nika Layzell |
| [syntect](https://github.com/trishume/syntect) | 5.3.0 | MIT | Tristan Hume |
| [sys-locale](https://github.com/1Password/sys-locale) | 0.3.2 | MIT OR Apache-2.0 | 1Password |
| [system-configuration](https://github.com/mullvad/system-configuration-rs) | 0.7.0 | MIT OR Apache-2.0 | Mullvad VPN |
| [system-configuration-sys](https://github.com/mullvad/system-configuration-rs) | 0.6.0 | MIT OR Apache-2.0 | Mullvad VPN |
| [system-deps](https://github.com/gdesmott/system-deps) | 6.2.2 | MIT OR Apache-2.0 | Guillaume Desmottes, Josh Triplett |
| [tag_ptr](https://github.com/boa-dev/boa) | 0.1.0 | Unlicense OR MIT | boa-dev |
| [tao](https://github.com/tauri-apps/tao) | 0.37.1 | Apache-2.0 | Tauri Programme within The Commons Conservancy, The winit contributors |
| [tao-macros](https://github.com/tauri-apps/tao) | 0.1.4 | MIT OR Apache-2.0 | Tauri Programme within The Commons Conservancy |
| [tap](https://github.com/myrrlyn/tap) | 1.0.1 | MIT | Elliott Linder, myrrlyn |
| [tar](https://github.com/composefs/tar-rs) | 0.4.46 | MIT OR Apache-2.0 | Alex Crichton |
| [target-lexicon](https://github.com/bytecodealliance/target-lexicon) | 0.12.16 | Apache-2.0 WITH LLVM-exception | Dan Gohman |
| [tempfile](https://github.com/Stebalien/tempfile) | 3.27.0 | MIT OR Apache-2.0 | Steven Allen, The Rust Project Developers, Ashley Mannix, Jason White |
| [tendril](https://github.com/servo/html5ever) | 0.5.1 | MIT OR Apache-2.0 | Keegan McAllister, Simon Sapin, Chris Morgan |
| [termcolor](https://github.com/BurntSushi/termcolor) | 1.4.1 | Unlicense OR MIT | Andrew Gallant |
| [text-size](https://github.com/rust-analyzer/text-size) | 1.1.1 | MIT OR Apache-2.0 | Aleksey Kladov, Christopher Durham (CAD97) |
| [thin-vec](https://github.com/mozilla/thin-vec) | 0.2.21 | MIT OR Apache-2.0 | Aria Beingessner |
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
| [tiny-skia](https://github.com/linebender/tiny-skia) | 0.12.0 | BSD-3-Clause | Yevhenii Reizner |
| [tiny-skia-path](https://github.com/RazrFalcon/tiny-skia/tree/master/path) | 0.11.4 | BSD-3-Clause | Yevhenii Reizner |
| [tiny-skia-path](https://github.com/linebender/tiny-skia/tree/master/path) | 0.12.0 | BSD-3-Clause | Yevhenii Reizner |
| [tinystr](https://github.com/unicode-org/icu4x) | 0.8.4 | Unicode-3.0 | The ICU4X Project Developers |
| [tinyvec](https://github.com/Lokathor/tinyvec) | 1.13.3 | Zlib OR Apache-2.0 OR MIT | Lokathor |
| [tls_codec](https://github.com/RustCrypto/formats) | 0.4.2 | Apache-2.0 OR MIT | RustCrypto Developers |
| [tls_codec_derive](https://github.com/RustCrypto/formats) | 0.4.2 | Apache-2.0 OR MIT | RustCrypto Developers |
| [tokio](https://github.com/tokio-rs/tokio) | 1.53.3 | MIT | Tokio Contributors |
| [tokio-macros](https://github.com/tokio-rs/tokio) | 2.7.2 | MIT | Tokio Contributors |
| [tokio-rustls](https://github.com/rustls/tokio-rustls) | 0.26.6 | MIT OR Apache-2.0 | — |
| [tokio-stream](https://github.com/tokio-rs/tokio) | 0.1.19 | MIT | Tokio Contributors |
| [tokio-util](https://github.com/tokio-rs/tokio) | 0.7.20 | MIT | Tokio Contributors |
| [toml](https://github.com/toml-rs/toml) | 0.8.2 | MIT OR Apache-2.0 | Alex Crichton |
| [toml_datetime](https://github.com/toml-rs/toml) | 0.6.3 | MIT OR Apache-2.0 | Alex Crichton |
| [toml_datetime](https://github.com/toml-rs/toml) | 1.1.2+spec-1.1.0 | MIT OR Apache-2.0 | — |
| [toml_edit](https://github.com/toml-rs/toml) | 0.19.15 | MIT OR Apache-2.0 | Andronik Ordian, Ed Page |
| [toml_edit](https://github.com/toml-rs/toml) | 0.20.2 | MIT OR Apache-2.0 | Andronik Ordian, Ed Page |
| [toml_edit](https://github.com/toml-rs/toml) | 0.25.17+spec-1.1.0 | MIT OR Apache-2.0 | — |
| [toml_parser](https://github.com/toml-rs/toml) | 1.1.5+spec-1.1.0 | MIT OR Apache-2.0 | — |
| [tower](https://github.com/tower-rs/tower) | 0.5.3 | MIT | Tower Maintainers |
| [tower-http](https://github.com/tower-rs/tower-http) | 0.6.11 | MIT | Tower Maintainers |
| [tower-layer](https://github.com/tower-rs/tower) | 0.3.3 | MIT | Tower Maintainers |
| [tower-service](https://github.com/tower-rs/tower) | 0.3.3 | MIT | Tower Maintainers |
| [tracing](https://github.com/tokio-rs/tracing) | 0.1.44 | MIT | Eliza Weisman, Tokio Contributors |
| [tracing-attributes](https://github.com/tokio-rs/tracing) | 0.1.31 | MIT | Tokio Contributors, Eliza Weisman, David Barsky |
| [tracing-core](https://github.com/tokio-rs/tracing) | 0.1.36 | MIT | Tokio Contributors |
| [tracing-log](https://github.com/tokio-rs/tracing) | 0.2.0 | MIT | Tokio Contributors |
| [tracing-subscriber](https://github.com/tokio-rs/tracing) | 0.3.23 | MIT | Eliza Weisman, David Barsky, Tokio Contributors |
| [try-lock](https://github.com/seanmonstar/try-lock) | 0.2.5 | MIT | Sean McArthur |
| [ttf-parser](https://github.com/harfbuzz/ttf-parser) | 0.25.1 | MIT OR Apache-2.0 | Caleb Maclennan, Laurenz Stampfl, Yevhenii Reizner, خالد حسني (Khaled Hosny) |
| [two-face](https://github.com/CosmicHorrorDev/two-face) | 0.4.5 | MIT OR Apache-2.0 | — |
| [twox-hash](https://github.com/shepmaster/twox-hash) | 2.1.5 | MIT | Jake Goulding |
| [type-map](https://github.com/kardeiz/type-map) | 0.5.1 | MIT OR Apache-2.0 | Jacob Brown |
| [typed-arena](https://github.com/SimonSapin/rust-typed-arena) | 2.0.2 | MIT | The typed-arena developers |
| [typeid](https://github.com/dtolnay/typeid) | 1.0.3 | MIT OR Apache-2.0 | David Tolnay |
| [typenum](https://github.com/paholg/typenum) | 1.20.1 | MIT OR Apache-2.0 | — |
| [typst](https://github.com/typst/typst) | 0.15.1 | Apache-2.0 | The Typst Project Developers |
| [typst-assets](https://github.com/typst/typst-assets) | 0.15.1 | Apache-2.0 | The Typst Project Developers |
| [typst-eval](https://github.com/typst/typst) | 0.15.1 | Apache-2.0 | The Typst Project Developers |
| [typst-html](https://github.com/typst/typst) | 0.15.1 | Apache-2.0 | The Typst Project Developers |
| [typst-layout](https://github.com/typst/typst) | 0.15.1 | Apache-2.0 | The Typst Project Developers |
| [typst-library](https://github.com/typst/typst) | 0.15.1 | Apache-2.0 | The Typst Project Developers |
| [typst-macros](https://github.com/typst/typst) | 0.15.1 | Apache-2.0 | The Typst Project Developers |
| [typst-realize](https://github.com/typst/typst) | 0.15.1 | Apache-2.0 | The Typst Project Developers |
| [typst-render](https://github.com/typst/typst) | 0.15.1 | Apache-2.0 | The Typst Project Developers |
| [typst-svg](https://github.com/typst/typst) | 0.15.1 | Apache-2.0 | The Typst Project Developers |
| [typst-syntax](https://github.com/typst/typst) | 0.15.1 | Apache-2.0 | The Typst Project Developers |
| [typst-timing](https://github.com/typst/typst) | 0.15.1 | Apache-2.0 | The Typst Project Developers |
| [typst-utils](https://github.com/typst/typst) | 0.15.1 | Apache-2.0 | The Typst Project Developers |
| [uds_windows](https://github.com/haraldh/rust_uds_windows) | 1.2.1 | MIT | Azure IoT Edge Devs, Harald Hoyer |
| [unic-langid](https://github.com/zbraniecki/unic-locale) | 0.9.6 | MIT OR Apache-2.0 | Zibi Braniecki |
| [unic-langid-impl](https://github.com/zbraniecki/unic-locale) | 0.9.6 | MIT OR Apache-2.0 | Zibi Braniecki |
| [unic-langid-macros](https://github.com/zbraniecki/unic-locale) | 0.9.6 | MIT OR Apache-2.0 | Zibi Braniecki |
| [unic-langid-macros-impl](https://github.com/zbraniecki/unic-locale) | 0.9.6 | MIT OR Apache-2.0 | Zibi Braniecki |
| [unicode-bidi](https://github.com/servo/unicode-bidi) | 0.3.18 | MIT OR Apache-2.0 | The Servo Project Developers |
| [unicode-bidi-mirroring](https://github.com/RazrFalcon/unicode-bidi-mirroring) | 0.4.0 | MIT OR Apache-2.0 | Yevhenii Reizner |
| [unicode-ccc](https://github.com/RazrFalcon/unicode-ccc) | 0.4.0 | MIT OR Apache-2.0 | Yevhenii Reizner |
| [unicode-general-category](https://github.com/yeslogic/unicode-general-category) | 1.1.0 | Apache-2.0 | YesLogic Pty. Ltd. |
| [unicode-ident](https://github.com/dtolnay/unicode-ident) | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 | David Tolnay |
| [unicode-linebreak](https://github.com/axelf4/unicode-linebreak) | 0.1.5 | Apache-2.0 | Axel Forsman |
| [unicode-math-class](https://github.com/typst/unicode-math-class) | 0.1.0 | MIT OR Apache-2.0 | Laurenz |
| [unicode-normalization](https://github.com/unicode-rs/unicode-normalization) | 0.1.26 | (MIT OR Apache-2.0) AND Unicode-3.0 | kwantam, Manish Goregaokar |
| [unicode-properties](https://github.com/unicode-rs/unicode-properties) | 0.1.4 | MIT OR Apache-2.0 | Charles Lew, Manish Goregaokar |
| [unicode-script](https://github.com/unicode-rs/unicode-script) | 0.5.8 | MIT OR Apache-2.0 | Manish Goregaokar |
| [unicode-segmentation](https://github.com/unicode-rs/unicode-segmentation) | 1.13.3 | MIT OR Apache-2.0 | kwantam, Manish Goregaokar |
| [unicode-vo](https://github.com/RazrFalcon/unicode-vo) | 0.1.0 | MIT OR Apache-2.0 | Evgeniy Reizner |
| [unicode-width](https://github.com/unicode-rs/unicode-width) | 0.1.14 | MIT OR Apache-2.0 | kwantam, Manish Goregaokar |
| [unsafe-libyaml](https://github.com/dtolnay/unsafe-libyaml) | 0.2.11 | MIT | David Tolnay |
| [unscanny](https://github.com/typst/unscanny) | 0.1.0 | MIT OR Apache-2.0 | Laurenz |
| [untrusted](https://github.com/briansmith/untrusted) | 0.7.1 | ISC | Brian Smith |
| [untrusted](https://github.com/briansmith/untrusted) | 0.9.0 | ISC | Brian Smith |
| [ureq](https://github.com/algesten/ureq) | 2.12.1 | MIT OR Apache-2.0 | Martin Algesten, Jacob Hoffman-Andrews |
| [url](https://github.com/servo/rust-url) | 2.5.8 | MIT OR Apache-2.0 | The rust-url developers |
| [usvg](https://github.com/linebender/resvg) | 0.47.0 | Apache-2.0 OR MIT | — |
| [usvg](https://github.com/linebender/resvg) | 0.48.1 | Apache-2.0 OR MIT | — |
| [utf16_iter](https://github.com/hsivonen/utf16_iter) | 1.0.5 | Apache-2.0 OR MIT | Henri Sivonen |
| [utf8_iter](https://github.com/hsivonen/utf8_iter) | 1.0.4 | Apache-2.0 OR MIT | Henri Sivonen |
| [utf8parse](https://github.com/alacritty/vte) | 0.2.2 | Apache-2.0 OR MIT | Joe Wilm, Christian Duerr |
| [uuid](https://github.com/uuid-rs/uuid) | 1.28.0 | Apache-2.0 OR MIT | Ashley Mannix, Dylan DPC, Hunar Roop Kahlon |
| [valuable](https://github.com/tokio-rs/valuable) | 0.1.1 | MIT | — |
| [vectorcraft-affinity](https://crates.io/crates/vectorcraft-affinity) | 0.14.0 | MIT OR Apache-2.0 | — |
| [vectorcraft-brush](https://crates.io/crates/vectorcraft-brush) | 0.14.0 | MIT OR Apache-2.0 | — |
| [vectorcraft-cad](https://crates.io/crates/vectorcraft-cad) | 0.14.0 | MIT OR Apache-2.0 | — |
| [vectorcraft-color](https://crates.io/crates/vectorcraft-color) | 0.14.0 | MIT OR Apache-2.0 | — |
| [vectorcraft-doc](https://crates.io/crates/vectorcraft-doc) | 0.14.0 | MIT OR Apache-2.0 | — |
| [vectorcraft-effects](https://crates.io/crates/vectorcraft-effects) | 0.14.0 | MIT OR Apache-2.0 | — |
| [vectorcraft-engine](https://crates.io/crates/vectorcraft-engine) | 0.14.0 | MIT OR Apache-2.0 | — |
| [vectorcraft-eps](https://crates.io/crates/vectorcraft-eps) | 0.14.0 | MIT OR Apache-2.0 | — |
| [vectorcraft-format](https://crates.io/crates/vectorcraft-format) | 0.14.0 | MIT OR Apache-2.0 | — |
| [vectorcraft-geom](https://crates.io/crates/vectorcraft-geom) | 0.14.0 | MIT OR Apache-2.0 | — |
| [vectorcraft-mcp](https://crates.io/crates/vectorcraft-mcp) | 0.14.0 | MIT OR Apache-2.0 | — |
| [vectorcraft-metafile](https://crates.io/crates/vectorcraft-metafile) | 0.14.0 | MIT OR Apache-2.0 | — |
| [vectorcraft-pathops](https://crates.io/crates/vectorcraft-pathops) | 0.14.0 | MIT OR Apache-2.0 | — |
| [vectorcraft-pdf](https://crates.io/crates/vectorcraft-pdf) | 0.14.0 | MIT OR Apache-2.0 | — |
| [vectorcraft-plugins](https://crates.io/crates/vectorcraft-plugins) | 0.14.0 | MIT OR Apache-2.0 | — |
| [vectorcraft-render](https://crates.io/crates/vectorcraft-render) | 0.14.0 | MIT OR Apache-2.0 | — |
| [vectorcraft-scriptcheck](https://crates.io/crates/vectorcraft-scriptcheck) | 0.14.0 | MIT OR Apache-2.0 | — |
| [vectorcraft-svg](https://crates.io/crates/vectorcraft-svg) | 0.14.0 | MIT OR Apache-2.0 | — |
| [vectorcraft-text](https://crates.io/crates/vectorcraft-text) | 0.14.0 | MIT OR Apache-2.0 | — |
| [vectorcraft-three-d](https://crates.io/crates/vectorcraft-three-d) | 0.14.0 | MIT OR Apache-2.0 | — |
| [vectorcraft-tools](https://crates.io/crates/vectorcraft-tools) | 0.14.0 | MIT OR Apache-2.0 | — |
| [vectorcraft-trace](https://crates.io/crates/vectorcraft-trace) | 0.14.0 | MIT OR Apache-2.0 | — |
| [vello_common](https://github.com/linebender/vello) | 0.0.8 | Apache-2.0 OR MIT | — |
| [vello_common](https://github.com/linebender/vello) | 0.0.9 | Apache-2.0 OR MIT | — |
| [vello_common](https://github.com/linebender/vello) | 0.1.0 | Apache-2.0 OR MIT | — |
| [vello_common](https://github.com/linebender/vello) | 0.2.0 | Apache-2.0 OR MIT | — |
| [vello_cpu](https://github.com/linebender/vello) | 0.0.8 | Apache-2.0 OR MIT | — |
| [vello_cpu](https://github.com/linebender/vello) | 0.1.0 | Apache-2.0 OR MIT | — |
| [vello_cpu](https://github.com/linebender/vello) | 0.2.0 | Apache-2.0 OR MIT | — |
| [version-compare](https://gitlab.com/timvisee/version-compare) | 0.2.1 | MIT | Tim Visee |
| [version_check](https://github.com/SergioBenitez/version_check) | 0.9.5 | MIT OR Apache-2.0 | Sergio Benitez |
| [walkdir](https://github.com/BurntSushi/walkdir) | 2.5.0 | Unlicense OR MIT | Andrew Gallant |
| [want](https://github.com/seanmonstar/want) | 0.3.2 | MIT | Sean McArthur |
| [wasi](https://github.com/bytecodealliance/wasi) | 0.11.1+wasi-snapshot-preview1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | The Cranelift Project Developers |
| [wasip2](https://github.com/bytecodealliance/wasi-rs) | 1.0.4+wasi-0.2.12 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | — |
| [wasm-bindgen](https://github.com/wasm-bindgen/wasm-bindgen) | 0.2.129 | MIT OR Apache-2.0 | The wasm-bindgen Developers |
| [wasm-bindgen-futures](https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/futures) | 0.4.79 | MIT OR Apache-2.0 | The wasm-bindgen Developers |
| [wasm-bindgen-macro](https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/macro) | 0.2.129 | MIT OR Apache-2.0 | The wasm-bindgen Developers |
| [wasm-bindgen-macro-support](https://github.com/wasm-bindgen/wasm-bindgen/tree/main/crates/macro-support) | 0.2.129 | MIT OR Apache-2.0 | The wasm-bindgen Developers |
| [wasm-bindgen-shared](https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/shared) | 0.2.129 | MIT OR Apache-2.0 | The wasm-bindgen Developers |
| [wasmi](https://github.com/wasmi-labs/wasmi) | 1.1.0 | MIT OR Apache-2.0 | Robin Freyler |
| [wasmi](https://github.com/wasmi-labs/wasmi) | 2.0.0 | MIT OR Apache-2.0 | Robin Freyler |
| [wasmi_collections](https://github.com/wasmi-labs/wasmi) | 1.1.0 | MIT OR Apache-2.0 | Robin Freyler |
| [wasmi_collections](https://github.com/wasmi-labs/wasmi) | 2.0.0 | MIT OR Apache-2.0 | Robin Freyler |
| [wasmi_core](https://github.com/wasmi-labs/wasmi) | 1.1.0 | MIT OR Apache-2.0 | Robin Freyler |
| [wasmi_core](https://github.com/wasmi-labs/wasmi) | 2.0.0 | MIT OR Apache-2.0 | Robin Freyler |
| [wasmi_ir](https://github.com/wasmi-labs/wasmi) | 1.1.0 | MIT OR Apache-2.0 | Robin Freyler |
| [wasmi_ir](https://github.com/wasmi-labs/wasmi) | 2.0.0 | MIT OR Apache-2.0 | Robin Freyler |
| [wasmparser](https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wasmparser) | 0.228.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | Yury Delendik |
| [wasmparser](https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wasmparser) | 0.239.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | Yury Delendik |
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
| [webpki-root-certs](https://github.com/rustls/webpki-roots) | 1.0.10 | CDLA-Permissive-2.0 | — |
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
| [which](https://github.com/harryfei/which-rs.git) | 5.0.0 | MIT | Harry Fei |
| [winapi](https://github.com/retep998/winapi-rs) | 0.3.9 | MIT OR Apache-2.0 | Peter Atashian |
| [winapi-i686-pc-windows-gnu](https://github.com/retep998/winapi-rs) | 0.4.0 | MIT OR Apache-2.0 | Peter Atashian |
| [winapi-util](https://github.com/BurntSushi/winapi-util) | 0.1.11 | Unlicense OR MIT | Andrew Gallant |
| [winapi-x86_64-pc-windows-gnu](https://github.com/retep998/winapi-rs) | 0.4.0 | MIT OR Apache-2.0 | Peter Atashian |
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
| [windows-registry](https://github.com/microsoft/windows-rs) | 0.6.1 | MIT OR Apache-2.0 | — |
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
| [winprint](https://github.com/ArcticLampyrid/winprint.rs/) | 0.2.1 | BSD-3-Clause | ArcticLampyrid |
| [winsafe](https://github.com/rodrigocfd/winsafe) | 0.0.29 | MIT | Rodrigo Cesar de Freitas Dias |
| [wit-bindgen](https://github.com/bytecodealliance/wit-bindgen) | 0.57.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | Alex Crichton |
| [wnaf](https://github.com/RustCrypto/elliptic-curves) | 0.14.1 | Apache-2.0 OR MIT | RustCrypto Developers |
| [wordcraft-control-key](https://crates.io/crates/wordcraft-control-key) | 0.8.0 | MIT OR Apache-2.0 | — |
| [wordcraft-doc](https://crates.io/crates/wordcraft-doc) | 0.8.0 | MIT OR Apache-2.0 | — |
| [wordcraft-docbin](https://crates.io/crates/wordcraft-docbin) | 0.8.0 | MIT OR Apache-2.0 | — |
| [wordcraft-docx](https://crates.io/crates/wordcraft-docx) | 0.8.0 | MIT OR Apache-2.0 | — |
| [wordcraft-engine](https://crates.io/crates/wordcraft-engine) | 0.8.0 | MIT OR Apache-2.0 | — |
| [wordcraft-fonts](https://crates.io/crates/wordcraft-fonts) | 0.8.0 | MIT OR Apache-2.0 | — |
| [wordcraft-formats](https://crates.io/crates/wordcraft-formats) | 0.8.0 | MIT OR Apache-2.0 | — |
| [wordcraft-geom](https://crates.io/crates/wordcraft-geom) | 0.8.0 | MIT OR Apache-2.0 | — |
| [wordcraft-layout](https://github.com/storytold/wordcraft) | 0.8.0 | MIT OR Apache-2.0 | — |
| [wordcraft-mcp](https://crates.io/crates/wordcraft-mcp) | 0.8.0 | MIT OR Apache-2.0 | — |
| [wordcraft-metafile](https://crates.io/crates/wordcraft-metafile) | 0.8.0 | MIT OR Apache-2.0 | — |
| [wordcraft-pdf](https://crates.io/crates/wordcraft-pdf) | 0.8.0 | MIT OR Apache-2.0 | — |
| [wordcraft-proof](https://crates.io/crates/wordcraft-proof) | 0.8.0 | MIT OR Apache-2.0 | — |
| [wordcraft-render](https://crates.io/crates/wordcraft-render) | 0.8.0 | MIT OR Apache-2.0 | — |
| [write-fonts](https://github.com/googlefonts/fontations) | 0.48.1 | MIT OR Apache-2.0 | — |
| [write16](https://github.com/hsivonen/write16) | 1.0.0 | Apache-2.0 OR MIT | — |
| [writeable](https://github.com/unicode-org/icu4x) | 0.6.4 | Unicode-3.0 | The ICU4X Project Developers |
| [wry](https://github.com/tauri-apps/wry) | 0.57.0 | Apache-2.0 OR MIT | Tauri Programme within The Commons Conservancy |
| [wuff](https://github.com/nicoburns/wuff) | 0.2.9 | MIT | Nico Burns |
| [wyz](https://github.com/myrrlyn/wyz) | 0.5.1 | MIT | myrrlyn |
| [x11](https://github.com/AltF02/x11-rs.git) | 2.21.0 | MIT | daggerbot, Erle Pereira, AltF02 |
| [x11-dl](https://github.com/AltF02/x11-rs.git) | 2.21.0 | MIT | daggerbot, Erle Pereira, AltF02 |
| [x11rb](https://github.com/psychon/x11rb) | 0.13.2 | MIT OR Apache-2.0 | Uli Schlachter, Eduardo Sánchez Muñoz, notgull |
| [x11rb-protocol](https://github.com/psychon/x11rb) | 0.13.2 | MIT OR Apache-2.0 | Uli Schlachter, Eduardo Sánchez Muñoz, notgull |
| [x509-cert](https://github.com/RustCrypto/formats/tree/master/x509-cert) | 0.2.5 | Apache-2.0 OR MIT | RustCrypto Developers |
| [x509-tsp](https://github.com/RustCrypto/formats/tree/master/x509-tsp) | 0.1.0 | Apache-2.0 OR MIT | RustCrypto Developers |
| [xattr](https://github.com/Stebalien/xattr) | 1.6.1 | MIT OR Apache-2.0 | Steven Allen |
| [xcursor](https://github.com/esposm03/xcursor-rs) | 0.3.11 | MIT | Samuele Esposito |
| [xkbcommon-dl](https://github.com/rust-windowing/xkbcommon-dl) | 0.4.2 | MIT | Francesca Frangipane |
| [xkeysym](https://github.com/notgull/xkeysym) | 0.2.1 | MIT OR Apache-2.0 OR Zlib | John Nunley |
| [xml](https://github.com/kornelski/xml-rs) | 1.4.0 | MIT | Vladimir Matveev, Kornel (https://github.com/kornelski) |
| [xml-rs](https://github.com/kornelski/xml-rs) | 0.8.29 | MIT | Vladimir Matveev |
| [xmlwriter](https://github.com/RazrFalcon/xmlwriter) | 0.1.0 | MIT | Evgeniy Reizner |
| [xmp-writer](https://github.com/typst/xmp-writer) | 0.3.3 | MIT OR Apache-2.0 | Martin Haug |
| [yaml-rust](https://github.com/chyh1990/yaml-rust) | 0.4.5 | MIT OR Apache-2.0 | Yuheng Chen |
| [yoke](https://github.com/unicode-org/icu4x) | 0.8.3 | Unicode-3.0 | Manish Goregaokar |
| [yoke-derive](https://github.com/unicode-org/icu4x) | 0.8.4 | Unicode-3.0 | Manish Goregaokar |
| [zbus](https://github.com/z-galaxy/zbus/) | 5.19.0 | MIT | Zeeshan Ali Khan |
| [zbus-lockstep](https://github.com/luukvanderduim/zbus-lockstep) | 0.5.2 | MIT | Luuk van der Duim, Tait Hoyem |
| [zbus-lockstep-macros](https://github.com/luukvanderduim/zbus-lockstep) | 0.5.2 | MIT | Luuk van der Duim |
| [zbus_macros](https://github.com/z-galaxy/zbus/) | 5.19.0 | MIT | Marc-André Lureau, Zeeshan Ali Khan |
| [zbus_names](https://github.com/z-galaxy/zbus/) | 4.3.5 | MIT | Zeeshan Ali Khan |
| [zbus_xml](https://github.com/z-galaxy/zbus/) | 5.2.1 | MIT | Zeeshan Ali Khan |
| [zcheapstr](https://github.com/z-galaxy/zcheapstr/) | 1.1.0 | MIT | Zeeshan Ali Khan |
| [zerocopy](https://github.com/google/zerocopy) | 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT | — |
| [zerocopy-derive](https://github.com/google/zerocopy) | 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT | — |
| [zerofrom](https://github.com/unicode-org/icu4x) | 0.1.8 | Unicode-3.0 | The ICU4X Project Developers |
| [zerofrom-derive](https://github.com/unicode-org/icu4x) | 0.1.8 | Unicode-3.0 | Manish Goregaokar |
| [zeroize](https://github.com/RustCrypto/utils) | 1.9.1 | Apache-2.0 OR MIT | The RustCrypto Project Developers |
| [zeroize_derive](https://github.com/RustCrypto/utils) | 1.5.0 | Apache-2.0 OR MIT | The RustCrypto Project Developers |
| [zerotrie](https://github.com/unicode-org/icu4x) | 0.2.5 | Unicode-3.0 | The ICU4X Project Developers |
| [zerovec](https://github.com/unicode-org/icu4x) | 0.11.8 | Unicode-3.0 | The ICU4X Project Developers |
| [zerovec-derive](https://github.com/unicode-org/icu4x) | 0.11.6 | Unicode-3.0 | Manish Goregaokar |
| [zip](https://github.com/zip-rs/zip2.git) | 3.0.0 | MIT | Mathijs van de Nes, Marli Frost, Ryan Levick, Chris Hennick |
| [zlib-rs](https://github.com/trifectatechfoundation/zlib-rs) | 0.6.8 | Zlib | — |
| [zmij](https://github.com/dtolnay/zmij) | 1.0.23 | MIT | David Tolnay |
| [zopfli](https://github.com/zopfli-rs/zopfli) | 0.8.4 | Apache-2.0 | — |
| [zune-core](https://github.com/etemesi254/zune-image) | 0.5.3 | MIT OR Apache-2.0 OR Zlib | — |
| [zune-inflate](https://github.com/etemesi254/zune-image/tree/main/zune-inflate) | 0.2.54 | MIT OR Apache-2.0 OR Zlib | — |
| [zune-jpeg](https://github.com/etemesi254/zune-image/tree/dev/crates/zune-jpeg) | 0.5.15 | MIT OR Apache-2.0 OR Zlib | caleb |
| [zvariant](https://github.com/z-galaxy/zbus/) | 5.15.1 | MIT | Zeeshan Ali Khan |
| [zvariant_derive](https://github.com/z-galaxy/zbus/) | 5.15.1 | MIT | Zeeshan Ali Khan |
| [zvariant_utils](https://github.com/z-galaxy/zbus/) | 4.2.0 | MIT | Zeeshan Ali Khan, turbocooler |
