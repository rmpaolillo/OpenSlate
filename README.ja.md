<p align="center">
  <img src="assets/icon/openslate-256.png" width="128" height="128" alt="OpenSlate のアイコン">
</p>

<h1 align="center">OpenSlate</h1>

<p align="center">
  <b>Wacom Bamboo Slate</b>（SmartPad ファミリー）のための、無料でオープンソースのコンパニオンアプリです。<br>
  ライブボード · オフライン同期 · ペンで Word と PDF の文書に署名 · 手書きをテキストに · ページ上で直接演習を添削する AI アシスタント
</p>

<p align="center">
  <a href="https://github.com/rmpaolillo/OpenSlate/releases/latest"><img alt="最新リリース" src="https://img.shields.io/github/v/release/rmpaolillo/OpenSlate?sort=semver"></a>
  <a href="https://github.com/rmpaolillo/OpenSlate/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/rmpaolillo/OpenSlate/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="ライセンス: GPL-3.0-or-later" src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue"></a>
</p>

*[English](README.md) · [Italiano](README.it.md) · [Español](README.es.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Українська](README.uk.md) · [中文](README.zh.md) · 日本語 · [한국어](README.ko.md)*

<p align="center">
  <img src="assets/screenshots/chat.png" alt="OpenSlate: AI アシスタントがボード上の手書きの数学の演習を確認し、赤インクで添削している様子" width="900">
</p>

---

## このプロジェクトが生まれた理由

私は Bamboo Slate を持っていて、毎日使っています。書くため、演習を解くため、そして文書に署名するためです。
Wacom は、このパッドが依存しているアプリ Inkspace の[サポート終了を発表しました](https://support.wacom.com/hc/en-us/articles/36070902398231-Bamboo-Paper-Inkspace-Wacom-Notes-application-End-of-Life-Notification)。
このままではハードウェアがただの文鎮になってしまうところでした。

そこで、自分のニーズのために自分でアプリを作りました。**OpenSlate はバイブコーディングで作られたプロジェクトです**。
欲しいものを説明し、AI コーディングアシスタント（[Claude](https://claude.ai)）と一緒に書きました。
最初に大きな設計を立てるのではなく、実際に使いながら改良を重ねてきました。私の macOS 環境ではよく動いています。
ほかの Slate ユーザーの役に立つかもしれないと思い、公開しています。粗削りな部分があることや、コードのコメントがイタリア語であることはご了承ください（インターフェース自体は 9 か国語に翻訳されています）。
Issue やプルリクエストを歓迎します。[CONTRIBUTING.md](CONTRIBUTING.md) をご覧ください。

> OpenSlate は独立したプロジェクトです。**Wacom とは提携しておらず、Wacom による承認やサポートも受けていません**。
> 「Wacom」「Bamboo」「Inkspace」はそれぞれの所有者の商標です。

## ひと目でわかる OpenSlate

- **紙に書いて、画面で見る。** Bamboo Slate はすべてのストロークを Bluetooth で送信します。
  コンピューターから離れて書いたページは、あとでアーカイブにダウンロードされます。
- **ページに書き込む AI チューター。** 演習の確認を頼むと、正しい部分にチェックを付け、
  間違いを丸で囲み、その横に手書きで訂正を書き込み、そのあとチャットで数式を交えて説明します。
  関数のグラフをページに描くこともできます。
- **ペンで Word と PDF の文書に署名**し、同じ形式で保存できます。
- **手書き → 編集可能なノート**。表や数式にも対応し、Markdown、Word、PDF にエクスポートできます。
  活字でも手書きフォントでも出力できます。
- **必要な形式で書き出し**：検索できる PDF、手書きを編集できるインクにした Word、軽い JPEG/WebP、Xournal++、ページが書かれていく様子のアニメーション GIF / SVG や MP4 動画、文字起こしから作る LaTeX または Typst。
- **AI は自由に選べます**: ローカルモデル（LM Studio、Ollama）またはクラウドプロバイダー。API キーが平文で保存されることはありません。
- **チャットに話しかける。** マイクで音声入力：Whisper がコンピュータ上でオフラインで文字起こしします。
- **無料でオープンソース**、9 か国語対応、macOS（メインプラットフォーム）、Linux、Windows で動作します。

## 機能

- **ライブボード。** Bluetooth で接続すると、パッドに書いた内容が書いたそばから全画面で表示されます。
  ペン、蛍光ペン（`H`。ペンの長押しで切り替える機能はオプションで有効にできます）、消しゴム（なぞり消しまたは消しゴムモード）、
  色、線の太さ、元に戻す/やり直し、回転/反転、トラックパッドのジェスチャーによるズームが使えます。
- **用紙のスタイル。** 罫線、イタリアの小学校用罫線、方眼（4/5/10 mm）、方眼紙、座標平面、ドット、
  等角（アイソメトリック）、五線譜、コーネル式ノートから選べます。罫線は実寸で描かれ、
  画面上にのみ表示されるため、エクスポートした結果はきれいなままです。
- **オフラインページ。** コンピューターから離れて書いたページを、日付ごとにまとめられたアーカイブにダウンロードできます。
  そこから名前の変更、エクスポート、OCR、ゴミ箱からの復元ができます。
- **Word と PDF の文書に署名。** `.docx`/`.doc`/`.odt`/`.rtf` または `.pdf` を開き、ペンで書き込んだり
  署名したりして、**同じ形式で**コピーを保存できます。Word ファイルでは、手書きは文書内のインクになるため、
  Word、Pages、LibreOffice のいずれでも表示されます。PDF では、手を加えていない元の文書の上にベクターストロークとして重ねられます。
  ページは [WordCraft](https://github.com/storytold/wordcraft) と
  [PdfCraft](https://github.com/storytold/pdfcraft) のエンジンで描画され、どのズーム倍率でも鮮明です。
- **画像にも書き込めます。** SVG、PNG、JPEG（WebP、TIFF、BMP、GIF にも対応）を開くか、スクリーンショットや写真を
  クリップボードにコピーした状態で **貼り付け**（`Cmd+V`）を押します。書き込んだあと、同じ形式・同じサイズの画像として保存できます。
  SVG では手書きがベクターパスとして保持され（[VectorCraft](https://github.com/storytold/vectorcraft)）、
  写真ではカラープロファイルを保ったままフル解像度で描き込まれます（[PhotoCraft](https://github.com/storytold/photocraft)）。
- **Word と同じようにコピーと貼り付け。** 「ホーム」タブの先頭には **貼り付け** と **コピー** があります。
  コピーはページを画像としてクリップボードに置くので、AI チャット、Word、メールにすぐ使えます。
- **手書き → テキスト。** 画像を読める AI モデルが、ページを間違いも含めて忠実に Markdown のノートへ文字起こしします。
  見出し、リスト、チェックボックス、表、そして数式があれば LaTeX で書き起こします。図、グラフ、ダイアグラムは、
  実際のペンストロークから鮮明な SVG 図として描き直され、ノートの横に置かれます。ノートはアプリ内で
  Typora のような編集可能なページとして開き、**エクスポート** で Markdown（図も一緒に）、Word、PDF として保存できます。
  **書体** では、ノートを活字または手書きで表示できます。手書きは筆記体かブロック体で、AI チャットがボードに書くのと
  同じ文字を使います。手書きは Word ファイル（埋め込まれるので、どのコンピューターでも表示されます）と PDF にも反映されます。
- **何にでも使える AI アシスタント。** チャットはページの横にあり、ボード、開いている文書、ノートを見ることができます。
  要約、きれいな書き直し、ToDo の抽出、文章の下書き、書いた内容の誤りチェック、質問への回答、ノートへの書き込み、
  Web 検索、EU の VAT 番号の確認に使えます。画像をチャットに貼り付けたり（`Cmd+V`、ドラッグ＆ドロップ、またはクリップのアイコン）、
  ピンチ操作や `Cmd +` / `Cmd −` で文字を大きくしたり小さくしたりできます。選んだモデルが画像を読めない場合（テキスト専用モデル）は、
  まず「ページの読み取り (OCR)」で選んだエージェントがページを読み取り、テキストとして渡します。背後には OpenAI 互換プロバイダーの
  フォールバックチェーンがあります: LM Studio、Ollama、DeepSeek、Google Gemini、Anthropic Claude。コンテキストの使用量を表示し、
  自動的に圧縮します。文書のタブをチャットにドラッグすると添付できます。
- **文書にグラフを入れ、編集する。** チャットにグラフを頼むと（「2 つの方程式と解をプロットして」）、
  [Typst](https://typst.app) と [Lilaq](https://lilaq.org) を使って LaTeX 風に描きます。関数、点、直線、棒グラフ、媒介変数曲線に対応しています。
  チャットに文書を添付すると、アプリ内で動作する Craft スイートの MCP サーバーを通じて、その文書を読んで **編集し**、
  グラフを挿入することもできます。Word には WordCraft、PDF には PdfCraft、SVG には VectorCraft、写真には PhotoCraft
  （切り抜き、サイズ変更、回転、明るさ、レベル補正、トーンカーブ、白黒、シャープ… 画像の形式はそのまま保たれます）、
  さらに Markdown 用のテキストツールがあります。すべての変更はチャットの横の文書にすぐ反映され、チャットからワンクリックで
  元に戻せます。ディスク上の元のファイルには一切触れません。文字起こしのノートでは、グラフで手描きの図を置き換えられます。
  ボードの用紙では、手描きのグラフ（軸とラベルから自動的に見つけます）を置き換えるか、書き込みの下に配置します。
  モデルが返信内に書いた SVG は画像として表示されます。
- **PDF フォームの記入。** PDF フォームをチャットに添付して「このフォームに私の情報を記入して」と頼むと、
  PdfCraft でフィールドを読み取って記入します。フィールドのない印刷・スキャンされたフォームには、まず
  フィールドを作成します（PdfCraft が空欄を見つけ、ラベルから名前を付けます）。または、チャットが
  テキスト、チェックマーク、日付を適切な場所に書き込みます。使うのはあなたが伝えた情報だけで、足りない
  ものは質問します。署名は決してしません。署名はあなたがペンで行い、記入した値は署名済みのコピーにも残ります。
- **チャットに話しかける。** 送信ボタンの横のマイクをクリックするか、話している間は右側の Alt（Option）
  キーを押し続けます：言葉がメッセージ欄に入り、直してから送信できます。Whisper が
  [whisper.cpp](https://github.com/ggml-org/whisper.cpp)（Mac では GPU）を使ってコンピュータ上でオフラインで文字起こしするので、
  あなたの声がコンピュータの外に出ることはありません。モデル（547 MB）は初回に、同意した場合だけ
  一度ダウンロードされます。`ai.toml`（`[voice]`）で、より軽いモデルやオンラインサービス（OpenAI、
  Groq、Mistral…）も選べます。Esc でキャンセル。初回は macOS がマイクの使用許可を求めます。
- **チャットがボードに書き込む。** 「計算をチェックして間違いに印を付けて」「締め切りに下線を引いて」
  「2 行目の横に結果を書いて」と頼むと、ボードの用紙を 1 行ずつ、1 単語ずつ読み取り、ハイライト、下線、丸囲み、取り消し線、
  枠、矢印、チェックマーク、バツ印を付け、手書きで書き込みます（活字体または筆記体。
  [Hershey](https://en.wikipedia.org/wiki/Hershey_fonts) の一筆書きフォントによるストロークで書かれます）。頼めば、用紙上のものを
  移動、拡大、色の変更、消去することもできます（「タイトルを大きく青くして」「最後の行を下に移動して」「私のハイライトを消して」）。
  自分の判断では追加しかしません。用紙の大部分を消すには、チャットであなたの同意が必要です。チャットが行ったことはすべて
  通常のインク（あなたのインクと同じように保存、エクスポート、消去できます）で、チャットでワンクリックすればすべて元に戻せます。
- **チャットはあなたのアーカイブを知っています。** 「10 月の予算について書いたページを探して」「昨日のノートをボードで開いて」
  「ボードを『会議』として保存して新しいページを始めて」「今週のページを 1 つの PDF にエクスポートして」と頼むと、
  タイトル、日付、文字起こしのテキストを検索し、ページを確認し、ボードで開き、ボードを保存し（元のページを更新します）、
  名前を変更し、いつものエクスポート先フォルダーにエクスポートし（上書きはしません）、ページをゴミ箱に移したり戻したりします。
  ゴミ箱への移動は、必ずチャットであなたの同意を待ちます。
- **独自の MCP サーバー。** Claude Desktop や DSH と同じように、`ai.toml` に任意の MCP サーバー（stdio）を追加できます。
  そのツールがチャットで使えるようになります。
- **Web でライブ表示。** 共有リンクを使えば、ホームネットワーク上のテレビ、スマートフォン、PC からボードをリアルタイムで見られます。
  固定のエンドポイントは `/page.pdf`、`/page.svg`、`/page.png`、`/state.json` です。
  ライブボードはホームネットワークに公開されます。チャット、ノート、会話には、OpenSlate が表示するリンクに含まれるキー
  （`?k=…`。その後はブラウザーが記憶します）が必要です。サーバーは IP アドレス、`localhost`、`.local` の名前にのみ応答し、
  ほかの Web サイトからの書き込みを拒否するので、あなたが訪れたページがアプリを操作することはできません。
- **何も失われません。** アーカイブにないものがボードにある状態で終了しようとすると、
  「保存して終了 / 保存せずに終了 / 終了しない」と尋ねます。アプリが確認なしに終了する場合（Dock、シャットダウン、アップデート）は、
  ボードは自動的にアーカイブに入ります。万一インターフェースがクラッシュした場合もボードは保存されます（原因はデータフォルダーの
  `crash.log` に記録されます）。ノートはバージョンチェック付きで保存されるため、AI やほかのアプリによる編集が知らないうちに
  上書きされることはありません。ダウンロードが中断された場合でも、すでにパッドから削除されたページは生データの安全コピーから復元されます。
- **用紙のみ。** タイトルバーのコピー/貼り付けの横にあるボタン（または `Cmd+Shift+F`、表示 → 用紙のみ）で、
  メニューバーとリボンを隠せます。もう一度クリックすると元に戻ります。
- **その他の形式。** 文字起こし済みのページは**検索できる PDF** になります。テキストが手書きの上に見えない形で重ねられるので、Spotlight や PDF リーダーで見つけられ、コピーもできます。「その他の形式」には、JPEG と WebP（軽量・送信向け）、手書きを編集できる本物の Word インクにした **Word**（次のページに文字起こし）、**Xournal++**（`.xopp`）、ページを一画ずつ書き直す**アニメーション GIF / SVG** や **MP4 動画**（動画はシステムのエンコーダーを使用：macOS は AVFoundation、Windows は Media Foundation、Linux は ffmpeg か、同意すれば初回に Cisco からダウンロードする OpenH264）、文字起こしから作る **LaTeX** または **Typst**（ノートもこれらに書き出せます）があります。AI チャットからもすべての形式に書き出せます。
- **保存とエクスポート。** 実寸の A4 サイズでベクター PDF と SVG に、または PNG にエクスポートできます。保存やエクスポートのたびに
  システムの「名前を付けて保存」ダイアログが開くので、ファイルの保存先を常に確認できます。ダイアログは最後に選んだフォルダーで開きます
  （初回は `exports/`）。ページを画像としてコピー（`Cmd+C`）して、どこにでも貼り付けることもできます。
- **あなたの言語で。** インターフェース全体（メニュー、リボン、オプション、チャット、ノート、AI 設定）が、英語、イタリア語、
  スペイン語、フランス語、ドイツ語、ウクライナ語、中国語、日本語、韓国語で利用できます。システムの言語に従います。
  変更は ファイル → オプション → 外観 → 言語 で行えます。AI はあなたが書いた言語で答えます。

## スクリーンショット

スクリーンショットは英語のインターフェースです（ほかに 8 つの言語が組み込まれています）。手書き、文書、AI の返信は
これらの画像のために用意したサンプルです。ページ上の印とグラフは、アプリ自身のボードツールで描かれたものです。

| | |
|---|---|
| **ページがひとりでに書き直される**：アニメーション GIF の書き出し、一画ずつ | **その他の形式**：JPEG、WebP、インク付き Word、Xournal++、アニメーション GIF / SVG、MP4 動画、LaTeX、Typst |
| <img src="assets/screenshots/writing.gif" alt="アニメーション GIF：ページが一画ずつ書かれていく" width="440"> | <img src="assets/screenshots/formats.png" alt="「その他の形式」メニュー" width="440"> |
| **ページ上のグラフ**: チャットが関数を描き、演習の下に配置します | ペンで **Word や PDF の文書に署名**し、同じ形式で保存します |
| <img src="assets/screenshots/graph.png" alt="AI チャットが関数のグラフを描き、手書きのページに配置している様子" width="440"> | <img src="assets/screenshots/sign.png" alt="ペンで署名した Word 文書" width="440"> |
| **手書き → テキスト**: 数式も含めて、編集可能なノートに文字起こししたページ | **手書きのノート**: 同じノートを筆記体の手書きフォントで表示 |
| <img src="assets/screenshots/transcribe.png" alt="表と数式を含むノートに文字起こしした手書きのページ" width="440"> | <img src="assets/screenshots/notes.png" alt="OpenSlate の筆記体手書きフォントで表示したノート" width="440"> |
| パッドからダウンロードしたページの **アーカイブ**（日付ごとにまとめて表示） | **ダークテーマ** と用紙のスタイル（ここでは 5 mm 方眼） |
| <img src="assets/screenshots/library.png" alt="手書きページのアーカイブ" width="440"> | <img src="assets/screenshots/dark.png" alt="方眼用紙と AI チャットを開いたダークテーマ" width="440"> |

## インストール

すぐにインストールできるビルドは
[**Releases**](https://github.com/rmpaolillo/OpenSlate/releases) ページで公開されており、それぞれに増加していく
バージョン番号（`v0.3.0`、`v0.3.1`、…）が付いています。

| プラットフォーム | ファイル | 状態 |
|---|---|---|
| macOS 11 以降、Apple Silicon（M1、M2…） | `OpenSlate-<version>-macos-arm64.dmg` | メインプラットフォーム、日常的に使用 |
| macOS 11 以降、Intel | `OpenSlate-<version>-macos-intel.dmg` | ビルド・動作可、テストは少なめ |
| macOS 11 以降、両方（サイズは 2 倍） | `OpenSlate-<version>-macos-universal.dmg` | 0.2.x からのアプリ内アップデート用 |
| Linux x86_64（Debian/Ubuntu） | `openslate_<version>_amd64.deb` | ビルド・動作可、テストは少なめ |
| Linux x86_64（その他のディストリビューション） | `openslate-<version>-linux-x86_64.tar.gz` | ビルド・動作可、テストは少なめ |
| Windows x86_64 | `openslate-<version>-windows-x86_64.zip` | **試験的** |

### macOS

1. `.dmg` を開き、**OpenSlate** を **アプリケーション** にドラッグします。
2. Apple Developer ID でビルドされたリリースは署名と公証が済んでいるため、警告なしで開きます。
   それがないビルド（下記「署名と公証」を参照）はアドホック署名なので、初回は macOS に「開けません」と表示されます。
   **システム設定 → プライバシーとセキュリティ** を開いて **このまま開く** をクリックするか、次を実行してください。
   ```bash
   xattr -dr com.apple.quarantine /Applications/OpenSlate.app
   ```
   これが必要なのは最初のインストール時だけです。アップデートはアプリがダウンロードするので、そのまま開きます。
3. 求められたら **Bluetooth** へのアクセスを許可します。

ほかにインストールするものはありません。すべてのファイルは
[Craft スイート](https://github.com/storytold)のライブラリで開いて保存します（下記「ファイルと形式」を参照）。

### Linux

```bash
sudo apt install ./openslate_<version>_amd64.deb     # Debian / Ubuntu
# or unpack the tar.gz and run ./openslate
```

実行時には BlueZ、GTK 3、WebKitGTK 4.1 が必要です。ユーザーが Bluetooth を使用できる権限を持っている必要があります。
MP4 動画の書き出しには、インストールされていれば `ffmpeg`（`sudo apt install ffmpeg`）を使います。ない場合は初回に、
Cisco の無料エンコーダー OpenH264（約 1 MB）を Cisco のサイトからダウンロードしてよいか確認します。

### Windows（試験的）

展開して `openslate.exe` を実行します。Microsoft Edge WebView2 ランタイムが必要ですが、これは
Windows 10/11 に標準で含まれています。Windows の「N」エディションでは、MP4 動画の書き出しに Microsoft の
Media Feature Pack も必要です。Windows ではすべてがテストされているわけではありません。バグ報告を大いに歓迎します。

## ファイルと形式

すべてのファイルは ArtCraft の Craft スイートのライブラリで開き、表示し、保存します。各アプリのインターフェースも
外部プログラムも使いません。

| ファイル | ライブラリ |
|---|---|
| Word（.docx、.doc、.odt、.rtf）、Markdown、.txt、.html | [WordCraft](https://github.com/storytold/wordcraft): ページを開いてレイアウトし、Word をインク付きで保存 |
| PDF | [PdfCraft](https://github.com/storytold/pdfcraft): ページ、サムネイル、テキスト。その PDF ライブラリが署名済みのコピーを保存 |
| SVG | [VectorCraft](https://github.com/storytold/vectorcraft) |
| 写真（PNG、JPEG、WebP、TIFF、BMP、GIF） | [PhotoCraft](https://github.com/storytold/photocraft) |
| ボードとアーカイブのエクスポート（PDF、SVG、PNG、JPEG、WebP） | VectorCraft が描画して書き出し、PNG・JPEG・WebP ファイルは PhotoCraft が書き出し |
| アニメーション GIF / SVG、Xournal++、LaTeX、Typst、検索できる PDF の隠しテキスト | OpenSlate 自身が書き出し（`src/formats.rs`、`src/mdconv.rs`） |
| MP4 動画 | システムのエンコーダー：AVFoundation（macOS）、Media Foundation（Windows）、Linux は ffmpeg か初回にダウンロードする Cisco の OpenH264（*OpenH264 Video Codec provided by Cisco Systems, Inc.*） |

文書として開いた Markdown ファイルは、ノートのページと同じ見た目になります: 本文は Source Serif、見出しは Inter、
表は軽やかなデザインです。数式（`$…$`、`$$…$$`、`\(…\)`、`\[…\]`）は [Typst](https://typst.app) によって LaTeX 風に描かれ
（LaTeX は [MiTeX](https://github.com/mitex-rs/mitex) で変換されます）、横に置かれた図（`<img src="…">`、`![…](…)`、
VectorCraft で描かれた SVG）は画像になります。
署名した Markdown、.txt、.html ファイルは PDF として保存されます（これらの形式はインクを保持できないためです）。

## アップデート

OpenSlate は、Craft アプリと同様に、あなたが確認したあとで自動的にアップデートします。GitHub で新しいリリースが
公開されると、リボンの下に青いバーが表示されて **更新** を提案します。続いてアップデートをダウンロードし、リリースの
`SHA256SUMS.txt` と開発者の署名で検証してから、実行中のアプリと置き換える形でインストールし、**今すぐ再起動** を提案します
（または、次回の起動時から新しいバージョンを使うこともできます）。オプション → 詳細 で、確認なしでインストールするように設定したり、
確認そのものをオフにしたりできます。ファイル → ヘルプと情報 → アップデートを確認 で、すぐに確認できます。

設定、AI エージェント、アーカイブ、文書、エクスポートはアプリの外にあるデータフォルダー（macOS:
`~/Library/Application Support/OpenSlate`）に、API キーはキーチェーンに保存されているため、アップデートで変更されることはありません。
macOS ではアップデートが `OpenSlate.app` を置き換えます（アプリ自身がダウンロードするので、Gatekeeper が再度確認することはありません）。
`.deb` の場合は `pkexec` を通じてパスワードを求めます。Windows ではアプリを終了したときにインストールされます。

## 初めての接続

- **初めて接続するとき**、または Inkspace を使ったあと: パッドのボタンを約 6 秒、LED が青く点滅するまで押し続けます。
  次に **接続** をクリックし、求められたらボタンを 1 回押します。
- **2 回目以降**（LED が緑）: ボタンを短く押してから **接続** をクリックします。

パッドが記憶できるアプリは一度に 1 つだけです。Inkspace に戻った場合は、OpenSlate をもう一度登録する必要があります。

## データの保存場所

インストールした場合、OpenSlate はアーカイブ、文書、チャット、設定を次の場所に保存します。

- macOS: `~/Library/Application Support/OpenSlate`
- Linux: `~/.local/share/openslate`（または `$XDG_DATA_HOME/openslate`）
- Windows: `%APPDATA%\OpenSlate`

環境変数 `OPENSLATE_DATA` でこれを変更できます。ソースフォルダーから `cargo run` で起動した場合は、
代わりにそのフォルダーで動作します。そこには `crash.log`（アプリがクラッシュしたことがある場合）、`web.key`（Web ライブリンクのキー）、
`updates/`（アップデート後に保持される、以前のバージョンのアプリ）もあります。Linux では、ダウンロードを
許可した場合、`codecs/` に Cisco の OpenH264 エンコーダーが置かれます。
`models/whisper/` には、ダウンロード後、チャットの音声入力用の Whisper モデルが置かれます。

## AI プロバイダーと API キー

AI 機能はオプションです。プロバイダーは **ファイル → オプション → AI** または `ai.toml` ファイルで設定します。
このファイルは初回使用時にコメント付きで作成されます。OpenAI の `/chat/completions` API に対応したサーバーであれば、
ローカルのもの（LM Studio、Ollama）も含めて使えます。ローカルのものにはキーは不要です。

**ページの読み取り (OCR)** には専用の設定があります（`ai.toml` の `[ocr]`、または「AI 設定 → ページの読み取り (OCR)」）。
「文字起こし」とテキスト専用のチャットモデルのために手書きを読み取るエージェントを選ぶか、空欄のままにして、
リストの中で画像を読める最初のエージェントを使います。Gemini のような高速なものをおすすめします。
`fallback = false` は「常にそのエージェントだけを使う」という意味です。

**API キーが平文で保存されることはありません。** `ai.toml` には各キーを *どこで* 見つけるかだけが書かれています。

```toml
key = "env:DEEPSEEK_API_KEY"              # environment variable
key = "keychain:DEEPSEEK_API_KEY"         # macOS Keychain
key = "env:GEMINI_API_KEY | keychain:GEMINI_API_KEY"   # first one found wins
```

```bash
# macOS Keychain (recommended: also works when the app is opened from Finder)
security add-generic-password -a "$USER" -s DEEPSEEK_API_KEY -w 'sk-...'
```

### MCP サーバー

チャットは外部の [MCP](https://modelcontextprotocol.io) サーバー（stdio トランスポート）のツールを使えます。
Claude Desktop と同じように `ai.toml` で設定します。

```toml
[[mcp]]
name = "filesystem"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/Users/me/Documents"]
env = { TOKEN = "keychain:MY_TOKEN" }   # values resolved like API keys
# enabled = false   to turn it off; timeout = 120 (seconds per tool)
```

OpenSlate はチャットが必要とするときにプログラムを起動し、そのまま実行し続けます。そのツールは `name__tool` という名前でモデルに渡されます。
Craft スイート自身の MCP サーバー（WordCraft、PdfCraft、VectorCraft）は設定不要です。アプリ内で、チャットに添付された文書に対して動作します。

## ソースからのビルド

**Rust 1.95 以降** が必要です（`rustup update`）。

```bash
git clone https://github.com/rmpaolillo/OpenSlate.git
cd OpenSlate
cargo run --release                  # GUI
cargo run --release -- gui --demo    # try it without a pad
cargo run --release -- --help        # command-line tools (scan, sync, live, pdf, ai, …)
cargo test
```

Linux のビルド依存関係（Debian/Ubuntu）:

```bash
sudo apt install pkg-config libgtk-3-dev libwebkit2gtk-4.1-dev libsoup-3.0-dev \
                 libjavascriptcoregtk-4.1-dev libdbus-1-dev libxkbcommon-dev libasound2-dev cmake
```

macOS では Xcode コマンドラインツール（`xcode-select --install`）と、チャットの音声入力用に whisper.cpp をビルドする CMake（`brew install cmake`）が必要です。オプションの Apple Vision OCR と
MP4 動画用の小さな Swift ヘルパーは、初回使用時にコンパイルされるか、リリース版の `.app` に同梱されています。

## プロジェクト構成

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
src/formats.rs, src/mdconv.rs                             export formats: searchable PDF, Word ink, Xournal++, animations, LaTeX and Typst
src/video.rs, src/swift/video_mp4.swift                  MP4 video: AVFoundation (macOS), Media Foundation (Windows), ffmpeg or Cisco's OpenH264 (Linux)
src/voice.rs, src/ui/voice_ui.rs                         voice in the AI chat: microphone (cpal), Whisper on the computer (whisper.cpp) or an online service
src/ui/sysdlg.rs                                          system dialogs (rfd; on Linux GTK in the window's thread)
src/fonts.rs                                              the fonts embedded in the app (one copy each)
src/demo.rs                                               sample pages for the screenshots (tests only)
packaging/, .github/workflows/                            app bundle, .deb, CI and releases
```

ファイルごとの説明は[イタリア語の README](README.it.md#struttura) にあります。

## リリースとバージョン管理

- `ci.yml` は、プッシュとプルリクエストのたびに Linux と macOS でビルドとテストを行います。
- `release.yml` はメンテナーが起動したときに実行されます（`main` 上で Actions → Release → Run workflow）。
  macOS（Apple Silicon、Intel、ユニバーサル）、Linux、Windows 向けにビルドし、自動生成されたリリースノート付きで
  GitHub Release を公開し、最新の 2 つのリリースだけを残します。リリース用のバイナリはストリップされ、
  クレートをまたいで最適化されます（`lto = "thin"`、`codegen-units = 1`）。ほかのライブラリから取り込まれる、あまり使われない
  ライブラリはサイズ優先で最適化され、各フォントは 1 回だけ埋め込まれます（`Cargo.toml` の `[profile.release]` と
  `src/fonts.rs` を参照）。
- バージョンは [Semantic Versioning](https://semver.org) に従います。`MAJOR.MINOR` は `Cargo.toml` から取得し、
  `PATCH` は既存のタグ（`v0.3.0`、次に `v0.3.1`、…）から自動的にカウントされます。
- 新しいシリーズ（新機能: `0.3.0`、最初の安定版: `1.0.0`）を始めるには、`Cargo.toml` の `version` を変更します。
  次のリリースはパッチ `0` から再開します。
- リリースではまずテストを実行し、ロックされた依存関係（`Cargo.lock`）でビルドします。各パッケージには
  `LICENSE`、`NOTICE`、`THIRD-PARTY.md`、`THIRD-PARTY-LICENSES.txt`（すべてのサードパーティライセンスの全文。
  `packaging/collect_licenses.py` で作成）が含まれます。
- すべてのリリースには `SHA256SUMS.txt` があります。アプリはアップデートをインストールする前に、ダウンロードしたファイルをこれで検証します。

### 署名と公証（macOS）

Craft アプリと同様に macOS が警告なしでアプリを信頼するようにするには、リリースに Apple の
**Developer ID Application** 証明書（Apple Developer Program、年間有料）が必要です。次のリポジトリシークレットを追加してください
（Settings → Secrets and variables → Actions）。

| シークレット | 値 |
|---|---|
| `APPLE_CERTIFICATE` | `.p12` としてエクスポートした「Developer ID Application」証明書を base64 にしたもの（`base64 -i cert.p12 \| pbcopy`） |
| `APPLE_CERTIFICATE_PASSWORD` | `.p12` のエクスポート時に設定したパスワード |
| `APPLE_ID` | デベロッパーアカウントの Apple ID のメールアドレス |
| `APPLE_PASSWORD` | その Apple ID 用の App 用パスワード（appleid.apple.com → サインインとセキュリティ） |
| `APPLE_TEAM_ID` | 10 文字のチーム ID（developer.apple.com → メンバーシップ） |

これらがあれば、`release.yml` は Hardened Runtime とタイムスタンプ付きでアプリに署名し、Apple の公証サービスに送信し、
チケットをステープルして、DMG にも同じ処理を行います。これらがない場合、アプリはアドホック署名となりますが、
リリースはそのまま機能します（初回は **このまま開く** で開きます）。

## クレジット

OpenSlate は、以下のプロジェクトとその作者なしには存在しませんでした。

- Tuhi コントリビューターによる **[Tuhi](https://github.com/tuhiproject/tuhi)**（GPL-2.0-or-later）。
  SmartPad ファミリーの Bluetooth プロトコルは、彼らのリバースエンジニアリングの成果から移植しました。
- ArtCraft Team とコントリビューターによる **Craft スイート**（MIT OR Apache-2.0）。インターフェースを除いた
  ライブラリとして使用しています。
  - **[WordCraft](https://github.com/storytold/wordcraft)**: Word 文書（および Markdown）をインク付きで開き、レイアウトし、保存します。
    Word 風のインターフェースはその egui UI を元にしています。レイアウト用のクレートは小さな変更を加えて `vendor/` にコピーしています。
  - **[PdfCraft](https://github.com/storytold/pdfcraft)**: PDF のページをどのズーム倍率でも鮮明に描画し、
    テキストを抽出し、署名済みのコピーを保存します。
  - **[VectorCraft](https://github.com/storytold/vectorcraft)**: SVG 画像をインク付きで開いて保存し、
    ボードとアーカイブのエクスポート（PDF、SVG、PNG）を描画して書き出します。
  - **[PhotoCraft](https://github.com/storytold/photocraft)**: カラープロファイルとサイズを保ったまま写真（PNG、JPEG、
    WebP、TIFF…）を読み書きし、その上にインクを描き込みます。
  - **それらの MCP サーバー**（`wordcraft-mcp`、`pdfcraft-automation`、`vectorcraft-mcp`、
    `photocraft-automation`）。アプリ内で動作し、AI チャットが横にある Word、PDF、SVG の文書や写真を読んで編集できるようにします。
  - [craft-libs](https://github.com/storytold/craft-libs) のパッチ適用済みライブラリ。
  - [craft-launcher](https://github.com/storytold/craft-launcher): アプリ内アップデートと macOS の署名の設計
    （アイデアのみで、コードはコピーしていません）。
- **[Typst](https://github.com/typst/typst)**（The Typst Project Developers、Apache-2.0）: LaTeX の数式とチャットの
  グラフを描画します。そのクレートのうち 2 つは、1 行の変更を加えて `vendor/` にコピーしています。
  **[MiTeX](https://github.com/mitex-rs/mitex)**（Apache-2.0）は LaTeX を Typst に変換します。
- **[Lilaq](https://github.com/lilaq-project/lilaq)**（Mc-Zen、MIT）と、それが使用する Typst パッケージ
  — [zero](https://github.com/Mc-Zen/zero)、[tiptoe](https://github.com/Mc-Zen/tiptoe)、
  [elembic](https://github.com/PgBiel/elembic)、[suiji](https://github.com/liuguangxi/suiji):
  AI チャットが描くグラフ。
- Emil Ernerfeldt とコントリビューターによる **[egui / eframe](https://github.com/emilk/egui)**:
  GUI ツールキット。
- Tauri チームによる **[wry](https://github.com/tauri-apps/wry)、[tao](https://github.com/tauri-apps/tao)、
  [muda](https://github.com/tauri-apps/muda)**: 埋め込み Web ビューとメニュー。
- Nonpolynomial Labs による **[btleplug](https://github.com/deviceplug/btleplug)**: クロスプラットフォームの
  Bluetooth LE。
- **[tokio](https://tokio.rs)**、**[clap](https://github.com/clap-rs/clap)**、
  **[serde](https://serde.rs)**、**[tiny-skia](https://github.com/linebender/tiny-skia)**、
  **[lopdf](https://github.com/J-F-Liu/lopdf)**、**[hayro](https://github.com/LaurenzV/hayro)**、
  **[ureq](https://github.com/algesten/ureq)**、**[arboard](https://github.com/1Password/arboard)**、
  **[rfd](https://github.com/PolyMeilex/rfd)**、および
  [THIRD-PARTY.md](THIRD-PARTY.md) に記載されたその他すべてのクレート。
- **[Inter](https://github.com/rsms/inter)**（Rasmus Andersson）、
  **[Source Serif 4](https://github.com/adobe-fonts/source-serif)**（Adobe）、および言語名表示用の
  **[Noto Sans CJK](https://github.com/notofonts/noto-cjk)**（Adobe）の小さなサブセット:
  フォント、SIL OFL 1.1。
- **[Phosphor Icons](https://phosphoricons.com)**（Helena Zhang、Tobias Fried）、
  [egui-phosphor](https://github.com/amPerl/egui-phosphor) 経由: アイコン、MIT。
- **[KaTeX](https://katex.org)**（Khan Academy）と **[marked](https://marked.js.org)**:
  チャットとノートの数式と Markdown、MIT。
  **[DOMPurify](https://github.com/cure53/DOMPurify)**（Cure53、Apache-2.0 OR MPL-2.0）は、
  その HTML を表示する前にクリーンアップします。
- **[Hershey フォント](https://en.wikipedia.org/wiki/Hershey_fonts)**（Dr. A. V. Hershey、U.S.
  National Bureau of Standards。`.jhf` 形式は James Hurt、Cognition Inc.。ファイルは Kamal Mostafa による
  [hershey-fonts](https://github.com/kamalmostafa/hershey-fonts) から）: チャットがボードに書くときに使う
  一筆書きの文字（`src/handwrite/`、謝辞の記載を条件とする自由なライセンス）。また、手書きノートの
  **OpenSlate Corsivo** フォントと **OpenSlate Stampatello** フォントにも変換されています（`assets/fonts/OpenSlate*.ttf`）。
- **[DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness)**: チャットのインスピレーション
  （アイデアのみで、コードはコピーしていません）。
- Anthropic の **[Claude](https://claude.ai)** を AI ペアプログラマーとして使って書かれました。

ライセンスと作者を含むクレートの完全なリストは [THIRD-PARTY.md](THIRD-PARTY.md) にあります。
ファイルごとの帰属表示は [NOTICE](NOTICE) にあります。

## ライセンス

Copyright © 2026 Marco Paolillo.

OpenSlate はフリーソフトウェアです。**GNU General Public License, version 3 or (at your option) any later version**
の条件の下で、再配布および改変することができます。[LICENSE](LICENSE) をご覧ください。
Bluetooth プロトコルのコードが Tuhi から派生しているため、GPL が必要となります。
