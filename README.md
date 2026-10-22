<p align="center">
  <img src="assets/icon/openslate-256.png" width="128" height="128" alt="OpenSlate icon">
</p>

<h1 align="center">OpenSlate</h1>

<p align="center">
  A free, open-source companion app for the <b>Wacom Bamboo Slate</b> (SmartPad family).<br>
  Live board · offline sync · sign Word and PDF documents with the pen · handwriting to text · AI assistant with charts
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
macOS. It is shared in case it is useful to other Slate owners. Expect rough edges and code
comments in Italian (the interface itself is translated into nine languages).
Issues and pull requests are welcome: see [CONTRIBUTING.md](CONTRIBUTING.md).

> OpenSlate is an independent project. It is **not affiliated with, endorsed by or supported by
> Wacom**. "Wacom", "Bamboo" and "Inkspace" are trademarks of their respective owners.

## Features

- **Live board.** Connect over Bluetooth and see what you write on the pad as you write it,
  full screen. You get pens, a highlighter (`H`; switching with a long press of the pen can be turned on in Options), an eraser (a
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
- **Write on images too.** Open an SVG, PNG, JPEG (also WebP, TIFF, BMP, GIF), or press **Paste**
  (`Cmd+V`) with a screenshot or photo on the clipboard. Write on it and save it back as an image
  of the same format and size: an SVG keeps the handwriting as vector paths
  ([VectorCraft](https://github.com/storytold/vectorcraft)), a photo gets it painted at full
  resolution with its colour profile kept ([PhotoCraft](https://github.com/storytold/photocraft)).
- **Copy and Paste like in Word.** The Home tab starts with **Paste** and **Copy**: Copy puts the
  page on the clipboard as an image, ready for the AI chat, Word or an email.
- **Handwriting → text.** A vision-capable AI model transcribes a page faithfully, mistakes
  included, into a Markdown note: headings, lists, checkboxes, tables, and formulas in LaTeX when
  there are any. Drawings, graphs and diagrams are redrawn from your actual pen strokes as crisp
  SVG figures next to the note. The note opens inside the app as an editable, Typora-like page,
  and **Export** saves it as Markdown (figures alongside), Word or PDF. **Writing** shows the
  notes printed or by hand, in cursive or block letters with the same letters the AI chat writes
  on the board; the handwriting goes into the Word file (embedded, so it shows on any computer)
  and the PDF too.
- **AI assistant, for anything.** The chat sits next to the page and can see the board, the
  open document or the note. Use it to summarise, rewrite neatly, pull out to-dos, draft texts,
  check what you wrote for mistakes, answer questions, write into the note, search the web and
  verify EU VAT numbers. Paste images into it (`Cmd+V`, drag and drop or the paperclip), and make
  its text bigger or smaller with a pinch or `Cmd +` / `Cmd −`. If the model you pick cannot see images (a text-only model), the page is first
  read by the agent chosen under *Page reading (OCR)* and handed over as text. Behind it is a fallback chain of OpenAI-compatible providers:
  LM Studio, Ollama, DeepSeek, Google Gemini and Anthropic Claude. It shows context usage and
  compacts automatically. Drag a document tab onto the chat to attach it.
- **Charts and edits in your documents.** Ask the chat for a graph ("plot the two equations and
  the solution") and it draws it in LaTeX style with [Typst](https://typst.app) and
  [Lilaq](https://lilaq.org): functions, points, lines, bars, parametric curves. With a document
  attached to the chat, it can also read and **edit it**, and put the chart into it, through the
  MCP servers of the Craft suite running inside the app: WordCraft for Word, PdfCraft for PDF,
  VectorCraft for SVG, PhotoCraft for photos (crop, resize, rotate, brightness, levels, curves,
  black and white, sharpen… the image keeps its format) plus text tools for Markdown. Every change shows at once in the document
  next to the chat and can be undone from the chat with one click; the original file on disk is
  never touched. In a transcription note the chart can replace the hand-drawn figure; on the board sheet it replaces the hand-drawn graph (found automatically: axes and labels) or goes under the writing. SVG written by the model in a reply is shown as a picture.
- **The chat writes on the board.** Ask it to "check my sums and mark the mistakes",
  "underline the deadlines" or "write the result next to line 2": it reads the board sheet line
  by line and word by word, then highlights, underlines, circles, strikes through, draws boxes,
  arrows, ticks and crosses, and writes in handwriting (print or cursive, made of strokes with
  the [Hershey](https://en.wikipedia.org/wiki/Hershey_fonts) single-line fonts). On request it
  also moves, enlarges, recolours or erases what is on the sheet ("make the title bigger and
  blue", "move the last line down", "remove my highlights"); on its own it only adds. Clearing
  most of the sheet needs your yes in the chat. Everything it does is ordinary ink (saved,
  exported and erased like yours) and one click in the chat takes it all away again.
- **The chat knows your library.** "Find the page where I wrote about the October budget",
  "open yesterday's notes on the board", "save the board as 'Meeting' and start a new page",
  "export this week's pages as one PDF": it searches titles, dates and the text of the
  transcriptions, looks at a page, opens it on the board, saves the board (updating the page it
  came from), renames, exports to your usual export folder (never overwriting) and moves pages to
  the trash and back. Moving to the trash always waits for your yes in the chat.
- **Your own MCP servers.** Add any MCP server (stdio) in `ai.toml`, as in Claude Desktop or DSH:
  its tools become available to the chat.
- **Live on the web.** A share link lets a TV, phone or PC on your home network watch the board
  in real time. Stable endpoints are `/page.pdf`, `/page.svg`, `/page.png` and `/state.json`.
  The live board is open to the home network; the chat, notes and conversations need the key
  that is part of the link OpenSlate shows (`?k=…`, then remembered by the browser). The server
  only answers to IP addresses, `localhost` and `.local` names, and refuses writes coming from
  other websites, so a page you visit cannot drive the app.
- **Nothing gets lost.** Quitting with something on the board that is not in the library asks
  *Save and quit / Quit without saving / Don't quit*; when the app quits without asking (Dock,
  shutdown, update) the board goes to the library by itself, and if the interface ever crashes
  the board is saved too (the reason goes to `crash.log` in the data folder). Notes are saved
  with a version check, so an edit from the AI or another app is never silently overwritten.
  Pages already removed from the pad are recovered from their raw safety copies if a download
  is interrupted.
- **Page only.** The button next to Copy/Paste in the title bar (or `Cmd+Shift+F`, View → Page
  only) hides the menu bar and the ribbon; click again to bring them back.
- **Save and export.** Export to vector PDF and SVG at true A4 size, or to PNG. Every save or
  export opens the system "Save as" dialog, so you always see where the file goes. It starts in the
  last folder you chose (the first time, in `exports/`). You can also copy the page as an image
  (`Cmd+C`) and paste it anywhere.
- **Your language.** The whole interface (menus, ribbon, options, chat, notes and AI settings)
  is available in English, Italian, Spanish, French, German, Ukrainian, Chinese, Japanese and
  Korean. It follows the system language; change it in File → Options → Appearance → Language.
  The AI answers in the language you write in.

## Screenshots

The screenshots show the Italian interface. The handwriting and documents are sample content.

| | |
|---|---|
| **Sign a Word or PDF document** with the pen and save it in the same format | **Library** of the pages downloaded from the pad, grouped by day |
| <img src="assets/screenshots/sign.png" alt="Signing a Word document" width="440"> | <img src="assets/screenshots/library.png" alt="Library of pages" width="440"> |
| **Handwriting → text**: a page transcribed into an editable note | **Dark theme** and paper styles (here 5 mm squares) |
| <img src="assets/screenshots/transcribe.png" alt="Handwriting transcribed into a note" width="440"> | <img src="assets/screenshots/dark.png" alt="Dark theme with squared paper" width="440"> |

## Install

Ready-to-install builds are published on the
[**Releases**](https://github.com/rmpaolillo/OpenSlate/releases) page, each with an increasing
version number (`v0.2.0`, `v0.2.1`, …).

| Platform | File | Status |
|---|---|---|
| macOS 11+, Apple Silicon (M1, M2…) | `OpenSlate-<version>-macos-arm64.dmg` | main platform, used daily |
| macOS 11+, Intel | `OpenSlate-<version>-macos-intel.dmg` | builds and runs; less tested |
| macOS 11+, both (twice the size) | `OpenSlate-<version>-macos-universal.dmg` | for the in-app update from 0.2.x |
| Linux x86_64 (Debian/Ubuntu) | `openslate_<version>_amd64.deb` | builds and runs; less tested |
| Linux x86_64 (other distros) | `openslate-<version>-linux-x86_64.tar.gz` | builds and runs; less tested |
| Windows x86_64 | `openslate-<version>-windows-x86_64.zip` | **experimental** |

### macOS

1. Open the `.dmg` and drag **OpenSlate** into **Applications**.
2. Releases built with an Apple Developer ID are signed and notarized: they open with no
   warning. A build without it (see "Signing and notarization" below) is signed ad hoc, so the
   first time macOS says it "cannot be opened": open **System Settings → Privacy & Security**
   and click **Open Anyway**, or run
   ```bash
   xattr -dr com.apple.quarantine /Applications/OpenSlate.app
   ```
   Only the first install needs this: updates are downloaded by the app and open directly.
3. Allow **Bluetooth** access when asked.

Nothing else to install: every file is opened and saved with the libraries of the
[Craft suite](https://github.com/storytold) (see "Files and formats" below).

### Linux

```bash
sudo apt install ./openslate_<version>_amd64.deb     # Debian / Ubuntu
# or unpack the tar.gz and run ./openslate
```

At run time it needs BlueZ, GTK 3 and WebKitGTK 4.1. Your user must be allowed to use Bluetooth.

### Windows (experimental)

Unzip and run `openslate.exe`. It needs the Microsoft Edge WebView2 runtime, which ships with
Windows 10/11. Not everything is tested on Windows. Bug reports are very welcome.

## Files and formats

Every file is opened, shown and saved with the libraries of ArtCraft's Craft suite, without
their interfaces and without external programs:

| Files | Library |
|---|---|
| Word (.docx, .doc, .odt, .rtf), Markdown, .txt, .html | [WordCraft](https://github.com/storytold/wordcraft): opens and lays out the pages, saves Word with the ink |
| PDF | [PdfCraft](https://github.com/storytold/pdfcraft): pages, thumbnails and text; its PDF library saves the signed copy |
| SVG | [VectorCraft](https://github.com/storytold/vectorcraft) |
| Photos (PNG, JPEG, WebP, TIFF, BMP, GIF) | [PhotoCraft](https://github.com/storytold/photocraft) |
| Board and library exports (PDF, SVG, PNG) | VectorCraft draws and writes them; PNG files are written by PhotoCraft |

A Markdown file opened as a document looks like the Notes page: Source Serif text, Inter
headings, light tables. Its formulas (`$…$`, `$$…$$`, `\(…\)`, `\[…\]`) are drawn in LaTeX style by
[Typst](https://typst.app) (the LaTeX is converted by [MiTeX](https://github.com/mitex-rs/mitex)),
and the figures next to it (`<img src="…">`, `![…](…)`, SVG drawn by VectorCraft) become pictures.
A signed Markdown, .txt or .html file is saved as PDF (those formats cannot hold the ink).

## Updates

OpenSlate updates itself, as the Craft apps do, after you confirm. When a new release is
published on GitHub a blue bar under the ribbon says so and offers **Update**; then it
downloads it, checks it against the release's `SHA256SUMS.txt` and the developer signature,
installs it in place of the running app and offers **Restart now** (or just use the new version
at the next launch). Options → Advanced can make it install without asking, or turn off the
check itself; File → Help and information → Check for updates checks right away.

Your settings, AI agents, library, documents and exports live in the data folder (macOS:
`~/Library/Application Support/OpenSlate`), outside the app, and the API keys in the
Keychain: an update never touches them. On macOS the update replaces `OpenSlate.app`
(downloaded by the app itself, so Gatekeeper does not ask again); with the `.deb` it asks for
your password through `pkexec`; on Windows it is installed when you quit the app.

## First connection

- **First time**, or after using Inkspace: hold the pad's button for about 6 s until the LED
  blinks blue. Then click **Connect** and press the button once when asked.
- **Afterwards** (LED green): press the button briefly, then click **Connect**.

The pad remembers only one app at a time. If you go back to Inkspace, you will need to register
OpenSlate again.

## Where your data lives

When installed, OpenSlate keeps your library, documents, chats and settings in:

- macOS: `~/Library/Application Support/OpenSlate`
- Linux: `~/.local/share/openslate` (or `$XDG_DATA_HOME/openslate`)
- Windows: `%APPDATA%\OpenSlate`

You can override this with the `OPENSLATE_DATA` environment variable. When started with
`cargo run` from the source folder, it works in that folder instead. There you also find
`crash.log` (if the app ever crashed), `web.key` (the key of the web live link) and `updates/`
(the previous version of the app, kept after an update).

## AI providers and API keys

AI features are optional. Providers are set up in **File → Options → AI** or in the `ai.toml`
file, which is created on first use with comments. Any server that speaks the OpenAI
`/chat/completions` API works, including local ones (LM Studio, Ollama), which need no key.

**Page reading (OCR)** has its own setting (`[ocr]` in `ai.toml`, or *AI settings → Lettura dei
fogli*): pick the agent that reads handwriting for *Trascrivi* and for text-only chat models, or
leave it empty to use the first agent in the list that can see images. A fast one such as Gemini
is recommended; `fallback = false` means "always and only that one".

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

### MCP servers

The chat can use tools of external [MCP](https://modelcontextprotocol.io) servers (stdio
transport), configured in `ai.toml` as in Claude Desktop:

```toml
[[mcp]]
name = "filesystem"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/Users/me/Documents"]
env = { TOKEN = "keychain:MY_TOKEN" }   # values resolved like API keys
# enabled = false   to turn it off; timeout = 120 (seconds per tool)
```

OpenSlate starts the program when the chat needs it and keeps it running; its tools reach the
model as `name__tool`. The Craft suite's own MCP servers (WordCraft, PdfCraft, VectorCraft) need
no configuration: they run inside the app on the document attached to the chat.

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
helper for the optional Apple Vision OCR is compiled on first use, or bundled inside the release `.app`.

## Project layout

```
src/protocol.rs, ble.rs, device.rs, live.rs, offline.rs   Bluetooth protocol (from Tuhi)
src/scene.rs, stroke.rs, library.rs                       board, strokes, page library
src/ui.rs, src/ui/*                                       egui interface (Word-like, from WordCraft)
src/document.rs, wordsign.rs, docrender.rs, pdfdoc.rs     Word/PDF opening, rendering, signing
src/ai.rs, chats.rs, tools.rs, ocr.rs                     AI providers, chat, transcription
src/web.rs, src/web/*                                     web live view, chat and notes pages
src/wordsign.rs, src/vecsign.rs, src/rasign.rs            signing Word (WordCraft), SVG (VectorCraft), photos (PhotoCraft)
src/update.rs, src/selfupdate.rs                          update check (GitHub releases) and in-app install
src/mdcraft.rs, src/mathtex.rs                           Markdown documents: Notes look, formulas drawn by Typst (MiTeX)
src/docxfonts.rs, packaging/hand_fonts.py                 handwriting fonts for the notes (made from src/handwrite.rs), embedded in Word
src/charts.rs, src/typstworld.rs, src/typst-pkg/          chat charts (Typst + Lilaq), Typst packages bundled in the app
src/docagent.rs, src/boardagent.rs, src/mcpclient.rs      chat edits documents (Craft MCP servers), notes and the board sheet; external MCP servers
src/boardread.rs, src/boardmarks.rs, src/handwrite.rs     the chat reads the board (lines, words) and marks it; handwriting from Hershey fonts
src/libagent.rs, src/ui/chat_lib.rs                       the chat on the page library (search, open, save, rename, export, trash)
vendor/                                                   copies of Typst and WordCraft crates with small changes (see vendor/README.md)
src/i18n.rs, src/i18n/*.json, src/web/i18n.js             interface languages (Italian text is the key)
src/platform.rs                                           OS differences (data folder, open files)
packaging/, .github/workflows/                            app bundle, .deb, CI and releases
```

A file-by-file description is in the [Italian README](README.it.md#struttura).

## Releases and versioning

- `ci.yml` builds and tests on Linux and macOS for every push and pull request.
- `release.yml` runs when the maintainer starts it (Actions → Release → Run workflow, on `main`).
  It builds macOS (Apple Silicon, Intel and universal), Linux and Windows and publishes a GitHub Release with
  automatically generated notes. The release binary is stripped and optimised as one program
  (`lto = "fat"`, `codegen-units = 1`); rarely used libraries pulled in by others are optimised
  for size (see `[profile.release]` in `Cargo.toml`).
- Versions follow [Semantic Versioning](https://semver.org): `MAJOR.MINOR` comes from `Cargo.toml`,
  `PATCH` is counted automatically from the existing tags (`v0.2.0`, then `v0.2.1`, …).
- To start a new series (new features: `0.3.0`; first stable: `1.0.0`), change `version` in
  `Cargo.toml`; the next release starts again from patch `0`.
- A release runs the tests first and builds with the locked dependencies (`Cargo.lock`). Each
  package contains `LICENSE`, `NOTICE`, `THIRD-PARTY.md` and `THIRD-PARTY-LICENSES.txt` (the
  full text of every third-party licence, made by `packaging/collect_licenses.py`).
- Every release has a `SHA256SUMS.txt`: the app checks the file it downloads against it before
  installing an update.

### Signing and notarization (macOS)

To make macOS trust the app with no warning, as for the Craft apps, the release needs an Apple
**Developer ID Application** certificate (Apple Developer Program, paid yearly). Add these
repository secrets (Settings → Secrets and variables → Actions):

| Secret | Value |
|---|---|
| `APPLE_CERTIFICATE` | the "Developer ID Application" certificate exported as `.p12`, in base64 (`base64 -i cert.p12 \| pbcopy`) |
| `APPLE_CERTIFICATE_PASSWORD` | the password chosen when exporting the `.p12` |
| `APPLE_ID` | the Apple ID e-mail of the developer account |
| `APPLE_PASSWORD` | an app-specific password for that Apple ID (appleid.apple.com → Sign-In and Security) |
| `APPLE_TEAM_ID` | the 10-character Team ID (developer.apple.com → Membership) |

With them, `release.yml` signs the app with the hardened runtime and a timestamp, sends it to
Apple's notary service, staples the ticket and does the same for the DMG. Without them the app
is signed ad hoc and the release still works (first open via **Open Anyway**).

## Credits

OpenSlate would not exist without these projects and their authors:

- **[Tuhi](https://github.com/tuhiproject/tuhi)** by the Tuhi contributors (GPL-2.0-or-later).
  The Bluetooth protocol of the SmartPad family was translated from their reverse-engineering
  work.
- **The Craft suite** by the ArtCraft Team and contributors (MIT OR Apache-2.0), used as
  libraries without their interfaces:
  - **[WordCraft](https://github.com/storytold/wordcraft)**: opens, lays out and saves Word
    documents (and Markdown) with the ink; the Word-like interface is adapted from its egui UI;
    its layout crate is copied in `vendor/` with a small change;
  - **[PdfCraft](https://github.com/storytold/pdfcraft)**: renders PDF pages crisp at any zoom,
    extracts their text and saves the signed copy;
  - **[VectorCraft](https://github.com/storytold/vectorcraft)**: opens and saves SVG images with
    the ink, and draws and writes the board and library exports (PDF, SVG, PNG);
  - **[PhotoCraft](https://github.com/storytold/photocraft)**: reads and writes photos (PNG, JPEG,
    WebP, TIFF…) keeping colour profile and size, and paints the ink on them;
  - **their MCP servers** (`wordcraft-mcp`, `pdfcraft-automation`, `vectorcraft-mcp`,
    `photocraft-automation`), run inside the app so the AI chat can read and edit the Word, PDF,
    SVG document or photo next to it;
  - patched libraries from [craft-libs](https://github.com/storytold/craft-libs);
  - [craft-launcher](https://github.com/storytold/craft-launcher): the design of the in-app
    updates and of the macOS signing (ideas only, no code copied).
- **[Typst](https://github.com/typst/typst)** (The Typst Project Developers, Apache-2.0): draws the
  LaTeX formulas and the chat's charts; two of its crates are copied in `vendor/` with a one-line
  change. **[MiTeX](https://github.com/mitex-rs/mitex)** (Apache-2.0) turns LaTeX into Typst.
- **[Lilaq](https://github.com/lilaq-project/lilaq)** (Mc-Zen, MIT) and the Typst packages it
  uses — [zero](https://github.com/Mc-Zen/zero), [tiptoe](https://github.com/Mc-Zen/tiptoe),
  [elembic](https://github.com/PgBiel/elembic), [suiji](https://github.com/liuguangxi/suiji):
  the charts drawn by the AI chat.
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
  **[rfd](https://github.com/PolyMeilex/rfd)** and every other crate listed in
  [THIRD-PARTY.md](THIRD-PARTY.md).
- **[Inter](https://github.com/rsms/inter)** (Rasmus Andersson),
  **[Source Serif 4](https://github.com/adobe-fonts/source-serif)** (Adobe) and a small subset
  of **[Noto Sans CJK](https://github.com/notofonts/noto-cjk)** (Adobe, for the language names):
  fonts, SIL OFL 1.1.
- **[Phosphor Icons](https://phosphoricons.com)** (Helena Zhang, Tobias Fried) via
  [egui-phosphor](https://github.com/amPerl/egui-phosphor): icons, MIT.
- **[KaTeX](https://katex.org)** (Khan Academy) and **[marked](https://marked.js.org)**:
  formulas and Markdown in the chat and notes, MIT;
  **[DOMPurify](https://github.com/cure53/DOMPurify)** (Cure53, Apache-2.0 OR MPL-2.0) cleans
  that HTML before it is shown.
- **[Hershey fonts](https://en.wikipedia.org/wiki/Hershey_fonts)** (Dr. A. V. Hershey, U.S.
  National Bureau of Standards; `.jhf` format by James Hurt, Cognition Inc.; files from
  [hershey-fonts](https://github.com/kamalmostafa/hershey-fonts) by Kamal Mostafa): the
  single-line letters the chat writes with on the board (`src/handwrite/`, free licence with
  acknowledgement), and turned into the **OpenSlate Corsivo** and **OpenSlate Stampatello** fonts
  of the handwritten notes (`assets/fonts/OpenSlate*.ttf`).
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
