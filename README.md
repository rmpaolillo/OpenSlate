<p align="center">
  <img src="assets/icon/openslate-256.png" width="128" height="128" alt="OpenSlate icon">
</p>

<h1 align="center">OpenSlate</h1>

<p align="center">
  A free, open-source companion app for the <b>Wacom Bamboo Slate</b> (SmartPad family).<br>
  Live board · offline sync · sign Word and PDF documents with the pen · handwriting to text · AI assistant
</p>

<p align="center">
  <a href="https://github.com/rmpaolillo/OpenSlate/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/rmpaolillo/OpenSlate?sort=semver"></a>
  <a href="https://github.com/rmpaolillo/OpenSlate/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/rmpaolillo/OpenSlate/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="License: GPL-3.0-or-later" src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue"></a>
</p>

*English · [Italiano](README.it.md)*

<p align="center">
  <img src="assets/screenshots/chat.png" alt="OpenSlate: a handwritten page on the board, with the AI assistant summarising it" width="900">
</p>

---

## Why this exists

I own a Bamboo Slate and use it every day: to write, to work through exercises, and to sign
documents. Wacom has [announced the end of life](https://support.wacom.com/hc/en-us/articles/36070902398231-Bamboo-Paper-Inkspace-Wacom-Notes-application-End-of-Life-Notification)
of Inkspace, the app the pad depends on, so the hardware was about to become a paperweight.

So I built my own app, for my own needs. **OpenSlate is a vibe-coded project**: I described
what I wanted and wrote it together with an AI coding assistant ([Claude](https://claude.ai)),
iterating on real use rather than following a big design up front. It works well for me on
macOS. It is shared in case it is useful to other Slate owners. Expect rough edges, an
interface that is still mostly **in Italian**, and code comments in Italian too.
Issues and pull requests are welcome: see [CONTRIBUTING.md](CONTRIBUTING.md).

> OpenSlate is an independent project. It is **not affiliated with, endorsed by or supported by
> Wacom**. "Wacom", "Bamboo" and "Inkspace" are trademarks of their respective owners.

## Features

- **Live board.** Connect over Bluetooth and see what you write on the pad as you write it,
  full screen. You get pens, a highlighter (long-press the pen to switch), an eraser (a
  scribble or eraser mode), colours, line widths, undo/redo, rotate/mirror and zoom with
  trackpad gestures.
- **Paper styles.** Choose from lined, Italian primary-school rulings, squared (4/5/10 mm),
  graph paper, Cartesian plane, dotted, isometric, music staff and Cornell notes. The rulings
  are drawn at true size and appear on screen only, so exports stay clean.
- **Offline pages.** Download the pages you wrote away from the computer into a library
  grouped by day. From there you can rename, export, OCR or restore pages from the trash.
- **Sign Word and PDF documents.** Open a `.docx`/`.doc`/`.odt`/`.rtf` or a `.pdf`, write or
  sign on it with the pen, and save a copy **in the same format**. In a Word file the
  handwriting becomes ink inside the document, so Word, Pages and LibreOffice all show it. In a
  PDF it becomes vector strokes over the untouched original. Pages stay crisp at any zoom,
  rendered by the [WordCraft](https://github.com/storytold/wordcraft) and
  [PdfCraft](https://github.com/storytold/pdfcraft) engines.
- **Handwriting → text.** A vision-capable AI model transcribes a page faithfully, mistakes
  included, into a Markdown note: headings, lists, checkboxes, tables, and formulas in LaTeX when
  there are any. The note opens inside the app as an editable, Typora-like page.
- **AI assistant, for anything.** The chat sits next to the page and can see the board, the
  open document or the note. Use it to summarise, rewrite neatly, pull out to-dos, draft texts,
  check what you wrote for mistakes, answer questions, write into the note, search the web and
  verify EU VAT numbers. Behind it is a fallback chain of OpenAI-compatible providers:
  LM Studio, Ollama, DeepSeek, Google Gemini and Anthropic Claude. It shows context usage and
  compacts automatically. Drag a document tab onto the chat to attach it.
- **Live on the web.** A share link lets a TV, phone or PC on your home network watch the board
  in real time. Stable endpoints are `/page.pdf`, `/page.svg`, `/page.png` and `/state.json`.
- **Export.** Export to vector PDF and SVG at true A4 size, or to PNG. You can also copy the page
  as an image (`Cmd+C`) and paste it anywhere.

## Screenshots

The interface is in Italian (for now). The handwriting and documents below are sample content.

| | |
|---|---|
| **Sign a Word or PDF document** with the pen and save it in the same format | **Library** of the pages downloaded from the pad, grouped by day |
| <img src="assets/screenshots/sign.png" alt="Signing a Word document" width="440"> | <img src="assets/screenshots/library.png" alt="Library of pages" width="440"> |
| **Handwriting → text**: a page transcribed into an editable note | **Dark theme** and paper styles (here 5 mm squares) |
| <img src="assets/screenshots/transcribe.png" alt="Handwriting transcribed into a note" width="440"> | <img src="assets/screenshots/dark.png" alt="Dark theme with squared paper" width="440"> |

## Install

Ready-to-install builds are published on the
[**Releases**](https://github.com/rmpaolillo/OpenSlate/releases) page. Every push to `main`
produces a new release with an increasing version number (`v0.1.N`).

| Platform | File | Status |
|---|---|---|
| macOS 11+ (Apple Silicon and Intel) | `OpenSlate-<version>-macos-universal.dmg` | main platform, used daily |
| Linux x86_64 (Debian/Ubuntu) | `openslate_<version>_amd64.deb` | builds and runs; less tested |
| Linux x86_64 (other distros) | `openslate-<version>-linux-x86_64.tar.gz` | builds and runs; less tested |
| Windows x86_64 | `openslate-<version>-windows-x86_64.zip` | **experimental** |

### macOS

1. Open the `.dmg` and drag **OpenSlate** into **Applications**.
2. The app is not notarized by Apple (that needs a paid developer account), so the first time
   macOS will say it "cannot be opened". Either right-click the app, choose **Open** and confirm,
   or run:
   ```bash
   xattr -dr com.apple.quarantine /Applications/OpenSlate.app
   ```
3. Allow **Bluetooth** access when asked.

Word documents open best with [LibreOffice](https://www.libreoffice.org) installed
(`brew install --cask libreoffice`). Without it, OpenSlate falls back to macOS `textutil`.

### Linux

```bash
sudo apt install ./openslate_<version>_amd64.deb     # Debian / Ubuntu
# or unpack the tar.gz and run ./openslate
```

At run time it needs BlueZ, GTK 3 and WebKitGTK 4.1. LibreOffice and Poppler (`pdftoppm`) are
optional, for importing documents. Your user must be allowed to use Bluetooth.

### Windows (experimental)

Unzip and run `openslate.exe`. It needs the Microsoft Edge WebView2 runtime, which ships with
Windows 10/11. Not everything is tested on Windows. Bug reports are very welcome.

## First connection

- **First time**, or after using Inkspace: hold the pad's button for about 6 s until the LED
  blinks blue. Then click **Collega** (Connect) and press the button once when asked.
- **Afterwards** (LED green): press the button briefly, then click **Collega**.

The pad remembers only one app at a time. If you go back to Inkspace, you will need to register
OpenSlate again.

## Where your data lives

When installed, OpenSlate keeps your library, documents, chats and settings in:

- macOS: `~/Library/Application Support/OpenSlate`
- Linux: `~/.local/share/openslate` (or `$XDG_DATA_HOME/openslate`)
- Windows: `%APPDATA%\OpenSlate`

You can override this with the `OPENSLATE_DATA` environment variable. When started with
`cargo run` from the source folder, it works in that folder instead.

## AI providers and API keys

AI features are optional. Providers are set up in **File → Options → AI** or in the `ai.toml`
file, which is created on first use with comments. Any server that speaks the OpenAI
`/chat/completions` API works, including local ones (LM Studio, Ollama), which need no key.

**API keys are never stored in plain text.** `ai.toml` only says *where* to find each key:

```toml
key = "env:DEEPSEEK_API_KEY"              # environment variable
key = "keychain:DEEPSEEK_API_KEY"         # macOS Keychain
key = "env:GEMINI_API_KEY | keychain:GEMINI_API_KEY"   # first one found wins
```

```bash
# macOS Keychain (recommended: also works when the app is opened from Finder)
security add-generic-password -a "$USER" -s DEEPSEEK_API_KEY -w 'sk-...'
```

## Build from source

You need **Rust 1.95 or newer** (`rustup update`).

```bash
git clone https://github.com/rmpaolillo/OpenSlate.git
cd OpenSlate
cargo run --release                  # GUI
cargo run --release -- gui --demo    # try it without a pad
cargo run --release -- --help        # command-line tools (scan, sync, live, pdf, ai, …)
cargo test
```

Linux build dependencies (Debian/Ubuntu):

```bash
sudo apt install pkg-config libgtk-3-dev libwebkit2gtk-4.1-dev libsoup-3.0-dev \
                 libjavascriptcoregtk-4.1-dev libdbus-1-dev libxkbcommon-dev
```

On macOS you need the Xcode command-line tools (`xcode-select --install`). The small Swift
helpers (PDFKit, Vision OCR) are compiled on first use, or bundled inside the release `.app`.

## Project layout

```
src/protocol.rs, ble.rs, device.rs, live.rs, offline.rs   Bluetooth protocol (from Tuhi)
src/scene.rs, stroke.rs, library.rs                       board, strokes, page library
src/ui.rs, src/ui/*                                       egui interface (Word-like, from WordCraft)
src/document.rs, wordsign.rs, docrender.rs, pdfdoc.rs     Word/PDF opening, rendering, signing
src/ai.rs, chats.rs, tools.rs, ocr.rs                     AI providers, chat, transcription
src/web.rs, src/web/*                                     web live view, chat and notes pages
src/platform.rs                                           OS differences (data folder, open files)
packaging/, .github/workflows/                            app bundle, .deb, CI and releases
```

A file-by-file description is in the [Italian README](README.it.md#struttura).

## Releases and versioning

- `ci.yml` builds and tests on Linux and macOS for every push and pull request.
- `release.yml` runs on every push to `main`. It sets the version to `0.1.<run number>`, builds
  macOS (universal), Linux and Windows, and publishes a GitHub Release tagged `v0.1.<run number>`
  with automatically generated notes.
- To start a new minor or major series, change the `0.1` prefix in `.github/workflows/release.yml`.

## Credits

OpenSlate would not exist without these projects and their authors:

- **[Tuhi](https://github.com/tuhiproject/tuhi)** by the Tuhi contributors (GPL-2.0-or-later).
  The Bluetooth protocol of the SmartPad family was translated from their reverse-engineering
  work.
- **[WordCraft](https://github.com/storytold/wordcraft)** and
  **[PdfCraft](https://github.com/storytold/pdfcraft)** by the ArtCraft Team and contributors
  (MIT OR Apache-2.0):
  - their engines open, render and save Word and PDF documents;
  - the Word-like interface is adapted from WordCraft's egui UI;
  - patched libraries come from [craft-libs](https://github.com/storytold/craft-libs).
- **[egui / eframe](https://github.com/emilk/egui)** by Emil Ernerfeldt and contributors: the
  GUI toolkit.
- **[wry](https://github.com/tauri-apps/wry), [tao](https://github.com/tauri-apps/tao),
  [muda](https://github.com/tauri-apps/muda)** by the Tauri team: embedded web views and menus.
- **[btleplug](https://github.com/deviceplug/btleplug)** by Nonpolynomial Labs: cross-platform
  Bluetooth LE.
- **[tokio](https://tokio.rs)**, **[clap](https://github.com/clap-rs/clap)**,
  **[serde](https://serde.rs)**, **[tiny-skia](https://github.com/linebender/tiny-skia)**,
  **[lopdf](https://github.com/J-F-Liu/lopdf)**, **[hayro](https://github.com/LaurenzV/hayro)**,
  **[ureq](https://github.com/algesten/ureq)**, **[arboard](https://github.com/1Password/arboard)**,
  **[rfd](https://github.com/PolyMeilex/rfd)**,
  **[pulldown-cmark](https://github.com/pulldown-cmark/pulldown-cmark)** and every other crate
  listed in [THIRD-PARTY.md](THIRD-PARTY.md).
- **[Inter](https://github.com/rsms/inter)** (Rasmus Andersson) and
  **[Source Serif 4](https://github.com/adobe-fonts/source-serif)** (Adobe): fonts, SIL OFL 1.1.
- **[Phosphor Icons](https://phosphoricons.com)** (Helena Zhang, Tobias Fried) via
  [egui-phosphor](https://github.com/amPerl/egui-phosphor): icons, MIT.
- **[KaTeX](https://katex.org)** (Khan Academy) and **[marked](https://marked.js.org)**:
  formulas and Markdown in the chat and notes, MIT.
- **[DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness)**: inspiration for the
  chat (ideas only, no code copied).
- Written with **[Claude](https://claude.ai)** by Anthropic as an AI pair programmer.

A complete list of crates with licences and authors is in [THIRD-PARTY.md](THIRD-PARTY.md).
Per-file attributions are in [NOTICE](NOTICE).

## License

Copyright © 2026 Marco Paolillo.

OpenSlate is free software: you can redistribute it and/or modify it under the terms of the
**GNU General Public License, version 3 or (at your option) any later version**. See
[LICENSE](LICENSE). The GPL is required because the Bluetooth protocol code is derived from Tuhi.
