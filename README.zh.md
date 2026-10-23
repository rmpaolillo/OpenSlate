<p align="center">
  <img src="assets/icon/openslate-256.png" width="128" height="128" alt="OpenSlate 图标">
</p>

<h1 align="center">OpenSlate</h1>

<p align="center">
  一款为 Wacom Bamboo Slate（SmartPad 系列）打造的免费开源配套应用。<br>
  实时画板 · 离线同步 · 用笔签署 Word 和 PDF 文档 · 手写转文字 · 直接在页面上批改练习的 AI 助手
</p>

<p align="center">
  <a href="https://github.com/rmpaolillo/OpenSlate/releases/latest"><img alt="最新版本" src="https://img.shields.io/github/v/release/rmpaolillo/OpenSlate?sort=semver"></a>
  <a href="https://github.com/rmpaolillo/OpenSlate/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/rmpaolillo/OpenSlate/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="许可证：GPL-3.0-or-later" src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue"></a>
</p>

*[English](README.md) · [Italiano](README.it.md) · [Español](README.es.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Українська](README.uk.md) · 中文 · [日本語](README.ja.md) · [한국어](README.ko.md)*

<p align="center">
  <img src="assets/screenshots/chat.png" alt="OpenSlate：AI 助手检查画板上一道手写的数学练习，并用红色墨迹批改" width="900">
</p>

---

## 为什么做这个项目

我有一块 Bamboo Slate，每天都在用：写字、做练习、签署文件。Wacom 已经[宣布停止支持](https://support.wacom.com/hc/en-us/articles/36070902398231-Bamboo-Paper-Inkspace-Wacom-Notes-application-End-of-Life-Notification)
Inkspace——这块手写板所依赖的应用，于是这台硬件眼看就要变成一块镇纸。

所以我按自己的需要做了一个自己的应用。OpenSlate 是一个氛围编程（vibe coding）项目：我描述想要的功能，
和一个 AI 编程助手（[Claude](https://claude.ai)）一起把它写出来，在实际使用中不断迭代，而不是事先做一套大而全的设计。
它在我的 macOS 上运行良好。之所以分享出来，是希望对其他 Slate 用户也有用。
难免有些粗糙之处，代码注释也是意大利语（界面本身已翻译成九种语言）。
欢迎提交 issue 和 pull request：请参阅 [CONTRIBUTING.md](CONTRIBUTING.md)。

> OpenSlate 是一个独立项目，与 Wacom 没有任何隶属关系，也未获得 Wacom 的认可或支持。
> 「Wacom」「Bamboo」和「Inkspace」是其各自所有者的商标。

## 一览

- 在纸上书写，在屏幕上看到。Bamboo Slate 通过蓝牙实时传送每一笔；
  离开电脑时写下的页面，之后会下载到档案库中。
- 一位在你的页面上书写的 AI 辅导老师。让它检查一道练习：它会给正确的地方打勾，
  圈出错误，并在旁边用手写体写下改正，然后在聊天中附上公式加以解释。它还能把函数图像画到页面上。
- 用笔签署 Word 和 PDF 文档，并以原格式保存。
- 手写 → 可编辑的笔记，支持表格和公式，可导出为 Markdown、Word 或 PDF，
  以印刷字体或手写字体呈现。
- 导出为你需要的格式：可搜索的 PDF、手写为可编辑墨迹的 Word、轻量的 JPEG/WebP、Xournal++、重现书写过程的 GIF、SVG 动画或 MP4 视频，以及由转写生成的 LaTeX 或 Typst。
- 自由选择 AI：本地模型（LM Studio、Ollama）或云端服务商；API 密钥从不以明文保存。
- 和聊天说话。用麦克风听写：Whisper 在你的电脑上离线转写。
- 免费开源，支持九种语言，适用于 macOS（主要平台）、Linux 和 Windows。

## 功能

- 实时画板。通过蓝牙连接后，你在手写板上写的内容会实时全屏显示在屏幕上。
  提供多种笔、荧光笔（`H`；可在「选项」中开启长按笔切换）、橡皮擦（涂抹或橡皮擦模式）、
  颜色、线宽、撤销/重做、旋转/镜像，以及用触控板手势缩放。
- 纸张样式。可选横线、意大利小学练习本格线、方格（4/5/10 mm）、坐标纸、笛卡尔坐标系、
  点阵、等距网格、五线谱和康奈尔笔记。格线按实际尺寸绘制，且只显示在屏幕上，因此导出的文件保持干净。
- 离线页面。将离开电脑时写下的页面下载到按日期分组的档案库中。
  在那里可以重命名、导出、OCR 识别，或从回收站恢复页面。
- 签署 Word 和 PDF 文档。打开 `.docx`/`.doc`/`.odt`/`.rtf` 或 `.pdf`，用笔在上面书写或签名，
  并以相同格式保存一份副本。在 Word 文件中，手写内容会成为文档内的墨迹，
  因此 Word、Pages 和 LibreOffice 都能显示。在 PDF 中，它会成为叠加在未改动原文件上的矢量笔画。
  页面由 [WordCraft](https://github.com/storytold/wordcraft) 和
  [PdfCraft](https://github.com/storytold/pdfcraft) 引擎渲染，任何缩放比例下都保持清晰。
- 也能在图片上书写。打开 SVG、PNG、JPEG（以及 WebP、TIFF、BMP、GIF），或在剪贴板中有截图或照片时
  点击「粘贴」（`Cmd+V`）。在上面书写后，可将其保存为相同格式和尺寸的图片：SVG 会以矢量路径保留手写内容
  （[VectorCraft](https://github.com/storytold/vectorcraft)），照片则以全分辨率绘入手写内容并保留其色彩配置文件
  （[PhotoCraft](https://github.com/storytold/photocraft)）。
- 像在 Word 中一样复制和粘贴。「主页」选项卡最前面就是「粘贴」和「复制」：「复制」会把页面作为图片放到剪贴板，
  可直接用于 AI 聊天、Word 或电子邮件。
- 手写 → 文字。具备视觉能力的 AI 模型会把页面忠实地（包括错误在内）转写为 Markdown 笔记：
  标题、列表、复选框、表格，如有公式则以 LaTeX 表示。图画、图表和示意图会根据你真实的笔迹
  重新绘制为清晰的 SVG 图形，放在笔记旁边。笔记在应用内以类似 Typora 的可编辑页面打开，
  「导出」可将其保存为 Markdown（图形一并保存）、Word 或 PDF。「字体」可将笔记以印刷字体或手写字体显示，
  手写字体有连笔体和印刷体两种，字母与 AI 聊天在画板上书写的相同；手写字体会写入 Word 文件
  （内嵌字体，在任何电脑上都能显示），也会写入 PDF。
- 万能的 AI 助手。聊天窗口位于页面旁边，能够看到画板、打开的文档或笔记。
  可以用它来总结、誊写整理、提取待办事项、起草文本、检查你写的内容有无错误、回答问题、写入笔记、
  搜索网络以及核验欧盟增值税号。可以把图片粘贴进去（`Cmd+V`、拖放或回形针按钮），
  并用双指捏合或 `Cmd +` / `Cmd −` 放大或缩小文字。如果所选模型无法识别图片（纯文本模型），
  页面会先由「页面识别（OCR）」中选定的智能体读取，再以文字形式交给它。其背后是一条由兼容 OpenAI 的服务商组成的备用链：
  LM Studio、Ollama、DeepSeek、Google Gemini 和 Anthropic Claude。它会显示上下文用量并自动压缩。
  把文档标签页拖到聊天窗口上即可将其附加。
- 在文档中插入图表和进行编辑。向聊天请求一张图表（「画出这两个方程和它们的解」），
  它就会用 [Typst](https://typst.app) 和 [Lilaq](https://lilaq.org) 以 LaTeX 风格绘制：
  函数、点、直线、柱状图、参数曲线。当聊天附加了文档时，它还能读取并编辑该文档，并把图表放进去，
  这一切通过在应用内运行的 Craft 套件 MCP 服务器完成：WordCraft 处理 Word，PdfCraft 处理 PDF，
  VectorCraft 处理 SVG，PhotoCraft 处理照片（裁剪、调整大小、旋转、亮度、色阶、曲线、
  黑白、锐化……图片保持原格式），另有用于 Markdown 的文本工具。每处改动都会立即显示在聊天旁边的文档中，
  并可在聊天中一键撤销；磁盘上的原文件永远不会被改动。在转写笔记中，图表可以替换手绘图形；
  在画板页面上，它会替换手绘的图像（自动识别坐标轴和标签），或放在书写内容下方。
  模型在回复中写出的 SVG 会以图片形式显示。
- 填写 PDF 表单。把 PDF 表单附加到聊天，然后说「用我的信息填写这份表单」：它会用 PdfCraft 读取字段并填写。
  没有字段的纸质或扫描表单会先生成字段（PdfCraft 找出空白处，并按旁边的标签命名），或者由聊天在合适的位置
  写上文字、勾选标记和日期。它只使用你告诉它的信息，缺少的会向你询问。它从不签名：签名由你用笔完成，
  填写的内容也会保留在签名后的副本中。
- 和聊天说话。点击“发送”旁边的麦克风，或在说话时按住右侧的 Alt（Option）键：说的话会进入消息框，
  改好后再发送。Whisper 借助 [whisper.cpp](https://github.com/ggml-org/whisper.cpp)（在 Mac 上使用 GPU）在你的电脑上离线转写，
  你的声音不会离开电脑。模型（547 MB）只在第一次、你同意后下载一次；也可以在 `ai.toml`（`[voice]`）
  中选择更小的模型或在线服务（OpenAI、Groq、Mistral…）。按 Esc 取消。第一次使用时，macOS 会请求麦克风权限。
  无论应用是什么语言，它都按你说的语言转写（可在 `[voice]` 的 `language` 中固定一种语言）。
- 聊天会在画板上书写。让它「检查我的算式并标出错误」「给截止日期划线」或「在第 2 行旁边写上结果」：
  它会逐行、逐词读取画板页面，然后进行高亮、下划线、圈注、删除线、画框、画箭头、打勾和打叉，
  并用手写体书写（印刷体或连笔体，由 [Hershey](https://en.wikipedia.org/wiki/Hershey_fonts) 单线字体的笔画构成）。
  应你的要求，它还能移动、放大、改色或擦除页面上的内容（「把标题放大并改成蓝色」「把最后一行往下移」
  「去掉我的高亮」）；在没有要求时，它只会添加内容。清除页面的大部分内容需要你在聊天中确认。
  它所做的一切都是普通墨迹（和你的墨迹一样保存、导出和擦除），在聊天中点一下就能把它们全部撤回。
- 聊天了解你的档案库。「找到我写十月预算的那一页」「在画板上打开昨天的笔记」
  「把画板保存为『会议』并新建一页」「把本周的页面导出为一个 PDF」：它会搜索标题、日期和转写文字，
  查看页面，在画板上打开页面，保存画板（更新它所来自的那一页），重命名，导出到你常用的导出文件夹（从不覆盖），
  以及把页面移到回收站或从回收站移回。移到回收站总会等待你在聊天中确认。
- 你自己的 MCP 服务器。在 `ai.toml` 中添加任意 MCP 服务器（stdio），方式与 Claude Desktop 或 DSH 相同：
  它的工具即可供聊天使用。
- 在网页上实时查看。通过共享链接，家庭网络中的电视、手机或电脑都能实时观看画板。
  固定的端点为 `/page.pdf`、`/page.svg`、`/page.png` 和 `/state.json`。
  实时画板对家庭网络开放；聊天、笔记和对话则需要 OpenSlate 所显示链接中包含的密钥
  （`?k=…`，之后由浏览器记住）。服务器只响应 IP 地址、`localhost` 和 `.local` 名称，
  并拒绝来自其他网站的写入请求，因此你访问的网页无法操控本应用。
- 什么都不会丢。画板上有尚未存入档案库的内容时退出，应用会询问
  「保存并退出 / 不保存退出 / 不退出」；当应用未经询问就退出（程序坞、关机、更新）时，
  画板会自动存入档案库；即使界面崩溃，画板也会被保存（原因记录在数据文件夹中的 `crash.log`）。
  笔记保存时会进行版本检查，因此 AI 或其他应用所做的修改绝不会被悄悄覆盖。
  如果下载中断，已从手写板上删除的页面会从其原始安全副本中恢复。
- 仅页面。标题栏中「复制/粘贴」旁边的按钮（或 `Cmd+Shift+F`，「视图 → 仅页面」）
  可隐藏菜单栏和功能区；再点一次即可恢复。
- 更多格式。已转写的页面会成为可搜索的 PDF：文字以不可见的方式叠在手写内容上，因此 Spotlight 和任何 PDF 阅读器都能找到，也可以复制。在「更多格式」中：JPEG 和 WebP（轻量，便于发送）、Word（手写内容是可编辑的真正 Word 墨迹，下一页附上转写）、Xournal++（`.xopp`）、逐笔重现书写过程的 GIF 或 SVG 动画或 MP4 视频（视频使用系统自带的编码器：macOS 上为 AVFoundation，Windows 上为 Media Foundation；Linux 上为 ffmpeg，或在你同意后首次使用时下载的 Cisco OpenH264），以及由转写生成的 LaTeX 或 Typst（笔记也可以导出为这两种格式）。AI 聊天也能导出为以上所有格式。
- 保存和导出。可导出为实际 A4 尺寸的矢量 PDF 和 SVG，或导出为 PNG。每次保存或导出都会打开系统的
  「另存为」对话框，因此你总能看到文件存放的位置。对话框从你上次选择的文件夹开始（第一次是 `exports/`）。
  你也可以把页面作为图片复制（`Cmd+C`），粘贴到任何地方。
- 你的语言。整个界面（菜单、功能区、选项、聊天、笔记和 AI 设置）提供英语、意大利语、西班牙语、
  法语、德语、乌克兰语、中文、日语和韩语版本。界面默认跟随系统语言；可在「文件 → 选项 → 外观 → 语言」中更改。
  AI 会用你书写所用的语言回答。

## 截图

截图展示的是英文界面（另外还内置了八种语言）。其中的手写内容、文档和 AI 回复都是为这些截图准备的示例内容；
页面上的标记和图表由应用自身的画板工具绘制。

| | |
|---|---|
| 页面自己重写：导出的 GIF 动画，一笔一笔重现 | 更多格式：JPEG、WebP、带笔迹的 Word、Xournal++、GIF/SVG 动画、MP4 视频、LaTeX、Typst |
| <img src="assets/screenshots/writing.gif" alt="GIF 动画：页面一笔一笔写出来" width="440"> | <img src="assets/screenshots/formats.png" alt="更多格式菜单" width="440"> |
| 页面上的图表：聊天绘制函数图像，并将其放在练习下方 | 用笔签署 Word 或 PDF 文档，并以原格式保存 |
| <img src="assets/screenshots/graph.png" alt="AI 聊天绘制函数图像并将其放在手写页面上" width="440"> | <img src="assets/screenshots/sign.png" alt="一份用笔签署的 Word 文档" width="440"> |
| 手写 → 文字：一页手写内容被转写为可编辑的笔记，公式也包括在内 | 手写体笔记：同一份笔记以连笔手写字体显示 |
| <img src="assets/screenshots/transcribe.png" alt="一页手写内容被转写为带有表格和公式的笔记" width="440"> | <img src="assets/screenshots/notes.png" alt="以 OpenSlate 连笔手写字体显示的笔记" width="440"> |
| 档案库：从手写板下载的页面，按日期分组 | 深色主题和纸张样式（此处为 5 mm 方格） |
| <img src="assets/screenshots/library.png" alt="手写页面档案库" width="440"> | <img src="assets/screenshots/dark.png" alt="深色主题，方格纸，AI 聊天已打开" width="440"> |

## 安装

可直接安装的版本发布在
[Releases](https://github.com/rmpaolillo/OpenSlate/releases) 页面，每个版本的版本号依次递增
（`v0.3.0`、`v0.3.1`……）。

| 平台 | 文件 | 状态 |
|---|---|---|
| macOS 11+，Apple Silicon（M1、M2…） | `OpenSlate-<version>-macos-arm64.dmg` | 主要平台，日常使用 |
| macOS 11+，Intel | `OpenSlate-<version>-macos-intel.dmg` | 可构建运行；测试较少 |
| macOS 11+，两者通用（体积翻倍） | `OpenSlate-<version>-macos-universal.dmg` | 用于从 0.2.x 进行应用内更新 |
| Linux x86_64（Debian/Ubuntu） | `openslate_<version>_amd64.deb` | 可构建运行；测试较少 |
| Linux x86_64（其他发行版） | `openslate-<version>-linux-x86_64.tar.gz` | 可构建运行；测试较少 |
| Windows x86_64 | `openslate-<version>-windows-x86_64.zip` | 实验性 |

### macOS

1. 打开 `.dmg`，将 OpenSlate 拖到「应用程序」中。
2. 使用 Apple Developer ID 构建的版本已签名并经过公证：打开时不会有任何警告。
   没有 Developer ID 的构建（见下文「签名与公证」）采用临时签名，因此第一次打开时 macOS 会提示「无法打开」：
   请打开「系统设置 → 隐私与安全性」并点击「仍要打开」，或运行
   ```bash
   xattr -dr com.apple.quarantine /Applications/OpenSlate.app
   ```
   只有首次安装需要这样做：更新由应用自行下载，可直接打开。
3. 在系统询问时允许蓝牙访问。

无需安装其他任何东西：所有文件都通过
[Craft 套件](https://github.com/storytold)的库打开和保存（见下文「文件与格式」）。

### Linux

```bash
sudo apt install ./openslate_<version>_amd64.deb     # Debian / Ubuntu
# or unpack the tar.gz and run ./openslate
```

运行时需要 BlueZ、GTK 3 和 WebKitGTK 4.1。你的用户必须拥有使用蓝牙的权限。
导出 MP4 视频时，如果已安装 `ffmpeg`（`sudo apt install ffmpeg`）就使用它；否则第一次会先征得你的同意，
再从 Cisco 网站下载 Cisco 免费的 OpenH264 编码器（约 1 MB）。

### Windows（实验性）

解压后运行 `openslate.exe`。需要 Microsoft Edge WebView2 运行时，Windows 10/11 已自带。
在 Windows 的 N 版本上，导出 MP4 视频还需要 Microsoft 的 Media Feature Pack。
并非所有功能都在 Windows 上测试过。非常欢迎提交错误报告。

## 文件与格式

所有文件都通过 ArtCraft 的 Craft 套件中的库来打开、显示和保存，不使用它们的界面，也不依赖外部程序：

| 文件 | 库 |
|---|---|
| Word（.docx、.doc、.odt、.rtf）、Markdown、.txt、.html | [WordCraft](https://github.com/storytold/wordcraft)：打开并排版页面，保存带墨迹的 Word |
| PDF | [PdfCraft](https://github.com/storytold/pdfcraft)：页面、缩略图和文字；其 PDF 库负责保存签名后的副本 |
| SVG | [VectorCraft](https://github.com/storytold/vectorcraft) |
| 照片（PNG、JPEG、WebP、TIFF、BMP、GIF） | [PhotoCraft](https://github.com/storytold/photocraft) |
| 画板和档案库导出（PDF、SVG、PNG、JPEG、WebP） | 由 VectorCraft 绘制和写入；PNG、JPEG 和 WebP 文件由 PhotoCraft 写入 |
| GIF 和 SVG 动画、Xournal++、LaTeX、Typst、可搜索 PDF 的隐藏文字 | 由 OpenSlate 自己写入（`src/formats.rs`、`src/mdconv.rs`） |
| MP4 视频 | 系统自带的编码器：AVFoundation（macOS）、Media Foundation（Windows）；Linux 上为 ffmpeg，或首次使用时下载的 Cisco OpenH264（OpenH264 Video Codec provided by Cisco Systems, Inc.） |

作为文档打开的 Markdown 文件，外观与「笔记」页面一致：正文使用 Source Serif，标题使用 Inter，表格线条清淡。
其中的公式（`$…$`、`$$…$$`、`\(…\)`、`\[…\]`）由 [Typst](https://typst.app) 以 LaTeX 风格绘制
（LaTeX 由 [MiTeX](https://github.com/mitex-rs/mitex) 转换），
旁边的图形（`<img src="…">`、`![…](…)`、由 VectorCraft 绘制的 SVG）则显示为图片。
签名后的 Markdown、.txt 或 .html 文件会保存为 PDF（这些格式无法容纳墨迹）。

## 更新

OpenSlate 会像 Craft 系列应用一样，在你确认后自行更新。当 GitHub 上发布了新版本时，
功能区下方会出现一条蓝色提示栏并提供「更新」按钮；随后应用会下载新版本，用该版本的 `SHA256SUMS.txt`
和开发者签名进行校验，替换正在运行的应用完成安装，并提供「立即重启」（也可以在下次启动时再使用新版本）。
在「选项 → 高级」中可以设置为不经询问直接安装，或完全关闭更新检查；
「文件 → 帮助和信息 → 检查更新」可立即检查更新。

你的设置、AI 智能体、档案库、文档和导出文件都存放在数据文件夹中（macOS：
`~/Library/Application Support/OpenSlate`），位于应用之外，API 密钥则保存在钥匙串中：
更新永远不会动到它们。在 macOS 上，更新会替换 `OpenSlate.app`
（由应用自行下载，因此 Gatekeeper 不会再次询问）；使用 `.deb` 时，会通过 `pkexec` 请求你的密码；
在 Windows 上，更新会在你退出应用时安装。

## 首次连接

- 第一次使用，或用过 Inkspace 之后：按住手写板的按钮约 6 秒，直到 LED 闪烁蓝色。
  然后点击「连接」，并在提示时按一下按钮。
- 之后（LED 为绿色）：短按按钮，然后点击「连接」。

手写板一次只能记住一个应用。如果你重新使用 Inkspace，就需要再次注册 OpenSlate。

## 数据存放位置

安装后，OpenSlate 会将你的档案库、文档、聊天和设置保存在：

- macOS：`~/Library/Application Support/OpenSlate`
- Linux：`~/.local/share/openslate`（或 `$XDG_DATA_HOME/openslate`）
- Windows：`%APPDATA%\OpenSlate`

你可以通过环境变量 `OPENSLATE_DATA` 更改此位置。如果在源码文件夹中用 `cargo run` 启动，
它会改为在该文件夹中工作。在数据文件夹中还能找到 `crash.log`（如果应用曾经崩溃）、`web.key`
（网页实时链接的密钥）和 `updates/`（更新后保留的上一版本应用）。在 Linux 上，如果你允许下载，
`codecs/` 中存放 Cisco 的 OpenH264 编码器。
下载后，`models/whisper/` 中存放聊天语音输入所用的 Whisper 模型。

## AI 服务商与 API 密钥

AI 功能是可选的。服务商可以在「文件 → 选项 → AI」中设置，也可以在 `ai.toml` 文件中设置，
该文件会在首次使用时自动创建并附带注释。任何支持 OpenAI `/chat/completions` API 的服务器都可以使用，
包括本地服务器（LM Studio、Ollama），它们不需要密钥。

页面识别（OCR）有单独的设置（`ai.toml` 中的 `[ocr]`，或「AI 设置 → 页面识别（OCR）」）：
选择用于「转写」以及为纯文本聊天模型读取手写内容的智能体，或留空以使用列表中第一个能识别图片的智能体。
推荐使用 Gemini 这类速度快的模型；`fallback = false` 表示「始终只用这一个」。

谁回答的。聊天中的每条回答都会显示实际写出它的智能体和模型（智能体按顺序尝试，所以不一定是第一个）。聊天还会告诉模型它是谁（服务、地址和模型），因为模型往往不知道自己是谁，可能会自称是别的模型：请以回答上方的标签为准。

语音（`ai.toml` 中的 `[voice]`）：`engine = "whisper"`（默认，在电脑上运行），或支持 `/audio/transcriptions` 的智能体名称（OpenAI、Groq、Mistral…；设为 `enabled = false`，这样聊天不会使用它）；`model`（`large-v3-turbo`、`small`、`base`，或该服务的转写模型）；`language`（留空 = 你说的语言）；`auto_send = true` 会在你停止说话后立即发送消息。

API 密钥从不以明文保存。`ai.toml` 只说明到哪里去找每个密钥：

```toml
key = "env:DEEPSEEK_API_KEY"              # environment variable
key = "keychain:DEEPSEEK_API_KEY"         # macOS Keychain
key = "env:GEMINI_API_KEY | keychain:GEMINI_API_KEY"   # first one found wins
```

```bash
# macOS Keychain (recommended: also works when the app is opened from Finder)
security add-generic-password -a "$USER" -s DEEPSEEK_API_KEY -w 'sk-...'
```

### MCP 服务器

聊天可以使用外部 [MCP](https://modelcontextprotocol.io) 服务器（stdio 传输）提供的工具，
配置方式与 Claude Desktop 相同，写在 `ai.toml` 中：

```toml
[[mcp]]
name = "filesystem"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/Users/me/Documents"]
env = { TOKEN = "keychain:MY_TOKEN" }   # values resolved like API keys
# enabled = false   to turn it off; timeout = 120 (seconds per tool)
```

OpenSlate 会在聊天需要时启动该程序并保持其运行；它的工具以 `name__tool` 的形式提供给模型。
Craft 套件自带的 MCP 服务器（WordCraft、PdfCraft、VectorCraft）无需任何配置：
它们在应用内运行，作用于附加到聊天的文档。

## 从源码构建

你需要 Rust 1.95 或更高版本（`rustup update`）。

```bash
git clone https://github.com/rmpaolillo/OpenSlate.git
cd OpenSlate
cargo run --release                  # GUI
cargo run --release -- gui --demo    # try it without a pad
cargo run --release -- --help        # command-line tools (scan, sync, live, pdf, ai, …)
cargo test
```

Linux 构建依赖（Debian/Ubuntu）：

```bash
sudo apt install pkg-config libgtk-3-dev libwebkit2gtk-4.1-dev libsoup-3.0-dev \
                 libjavascriptcoregtk-4.1-dev libdbus-1-dev libxkbcommon-dev libasound2-dev cmake
```

在 macOS 上需要 Xcode 命令行工具（`xcode-select --install`）和 CMake（`brew install cmake`，用于编译聊天语音输入所需的 whisper.cpp）。用于可选的 Apple Vision OCR 和 MP4 视频的小型 Swift
辅助程序会在首次使用时编译，或已打包在发布版的 `.app` 中。

## 项目结构

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

逐个文件的说明见[意大利语 README](README.it.md#struttura)。

## 发布与版本号

- `ci.yml` 会在每次 push 和 pull request 时在 Linux 和 macOS 上构建并测试。
- `release.yml` 由维护者手动启动（在 `main` 上依次选择 Actions → Release → Run workflow）。
  它会构建 macOS（Apple Silicon、Intel 和通用版）、Linux 和 Windows 版本，并发布一个附带自动生成说明的
  GitHub Release，且只保留最新的两个版本。发布版二进制文件经过 strip，并进行跨 crate 优化
  （`lto = "thin"`、`codegen-units = 1`）；被其他库引入的少用库会针对体积进行优化，每种字体只嵌入一次
  （见 `Cargo.toml` 中的 `[profile.release]` 和 `src/fonts.rs`）。
- 版本号遵循[语义化版本](https://semver.org)：`MAJOR.MINOR` 来自 `Cargo.toml`，
  `PATCH` 根据已有的标签自动计数（`v0.3.0`，然后是 `v0.3.1`……）。
- 要开始一个新的系列（新功能：`0.3.0`；首个稳定版：`1.0.0`），请修改 `Cargo.toml` 中的 `version`；
  下一个版本会从补丁号 `0` 重新开始。
- 发布前会先运行测试，并使用锁定的依赖（`Cargo.lock`）进行构建。每个软件包都包含 `LICENSE`、`NOTICE`、
  `THIRD-PARTY.md` 和 `THIRD-PARTY-LICENSES.txt`（每个第三方许可证的完整文本，由
  `packaging/collect_licenses.py` 生成）。
- 每个版本都附有 `SHA256SUMS.txt`：应用在安装更新前，会用它校验下载的文件。

### 签名与公证（macOS）

要让 macOS 像信任 Craft 系列应用一样不加警告地信任本应用，发布版需要 Apple 的
Developer ID Application 证书（Apple Developer Program，按年付费）。请添加以下仓库机密
（Settings → Secrets and variables → Actions）：

| 机密 | 值 |
|---|---|
| `APPLE_CERTIFICATE` | 导出为 `.p12` 的「Developer ID Application」证书，经 base64 编码（`base64 -i cert.p12 \| pbcopy`） |
| `APPLE_CERTIFICATE_PASSWORD` | 导出 `.p12` 时设置的密码 |
| `APPLE_ID` | 开发者账户的 Apple ID 电子邮件 |
| `APPLE_PASSWORD` | 该 Apple ID 的 App 专用密码（appleid.apple.com → 登录与安全性） |
| `APPLE_TEAM_ID` | 10 个字符的 Team ID（developer.apple.com → Membership） |

配置这些机密后，`release.yml` 会使用强化运行时和时间戳为应用签名，将其提交给 Apple 公证服务，
装订公证票据，并对 DMG 执行同样的操作。如果没有这些机密，应用会采用临时签名，发布版依然可用
（首次打开需通过「仍要打开」）。

## 致谢

没有以下项目及其作者，就不会有 OpenSlate：

- [Tuhi](https://github.com/tuhiproject/tuhi)，由 Tuhi 贡献者开发（GPL-2.0-or-later）。
  SmartPad 系列的蓝牙协议是根据他们的逆向工程成果移植而来的。
- Craft 套件，由 ArtCraft Team 及贡献者开发（MIT OR Apache-2.0），作为库使用，不含其界面：
  - [WordCraft](https://github.com/storytold/wordcraft)：打开、排版并保存带墨迹的 Word 文档（以及 Markdown）；
    类似 Word 的界面改编自它的 egui UI；它的排版 crate 经少量修改后复制在 `vendor/` 中；
  - [PdfCraft](https://github.com/storytold/pdfcraft)：在任何缩放比例下清晰渲染 PDF 页面，
    提取其文字并保存签名后的副本；
  - [VectorCraft](https://github.com/storytold/vectorcraft)：打开并保存带墨迹的 SVG 图片，
    并绘制和写入画板与档案库的导出文件（PDF、SVG、PNG）；
  - [PhotoCraft](https://github.com/storytold/photocraft)：读写照片（PNG、JPEG、WebP、TIFF……），
    保留色彩配置文件和尺寸，并将墨迹绘制到照片上；
  - 它们的 MCP 服务器（`wordcraft-mcp`、`pdfcraft-automation`、`vectorcraft-mcp`、
    `photocraft-automation`），在应用内运行，使 AI 聊天能够读取和编辑旁边的 Word、PDF、SVG 文档或照片；
  - 来自 [craft-libs](https://github.com/storytold/craft-libs) 的打过补丁的库；
  - [craft-launcher](https://github.com/storytold/craft-launcher)：应用内更新和 macOS 签名的设计
    （仅借鉴思路，未复制代码）。
- [Typst](https://github.com/typst/typst)（The Typst Project Developers，Apache-2.0）：绘制 LaTeX 公式和聊天中的图表；
  它的两个 crate 经过一行修改后复制在 `vendor/` 中。[MiTeX](https://github.com/mitex-rs/mitex)（Apache-2.0）
  将 LaTeX 转换为 Typst。
- [Lilaq](https://github.com/lilaq-project/lilaq)（Mc-Zen，MIT）及其使用的 Typst 包——
  [zero](https://github.com/Mc-Zen/zero)、[tiptoe](https://github.com/Mc-Zen/tiptoe)、
  [elembic](https://github.com/PgBiel/elembic)、[suiji](https://github.com/liuguangxi/suiji)：
  AI 聊天所绘制的图表。
- [egui / eframe](https://github.com/emilk/egui)，由 Emil Ernerfeldt 及贡献者开发：GUI 工具包。
- [wry](https://github.com/tauri-apps/wry)、[tao](https://github.com/tauri-apps/tao)、
  [muda](https://github.com/tauri-apps/muda)，由 Tauri 团队开发：内嵌网页视图和菜单。
- [btleplug](https://github.com/deviceplug/btleplug)，由 Nonpolynomial Labs 开发：跨平台低功耗蓝牙。
- [tokio](https://tokio.rs)、[clap](https://github.com/clap-rs/clap)、
  [serde](https://serde.rs)、[tiny-skia](https://github.com/linebender/tiny-skia)、
  [lopdf](https://github.com/J-F-Liu/lopdf)、[hayro](https://github.com/LaurenzV/hayro)、
  [ureq](https://github.com/algesten/ureq)、[arboard](https://github.com/1Password/arboard)、
  [rfd](https://github.com/PolyMeilex/rfd)，以及
  [THIRD-PARTY.md](THIRD-PARTY.md) 中列出的所有其他 crate。
- [Inter](https://github.com/rsms/inter)（Rasmus Andersson）、
  [Source Serif 4](https://github.com/adobe-fonts/source-serif)（Adobe）以及
  [Noto Sans CJK](https://github.com/notofonts/noto-cjk) 的一个小子集（Adobe，用于显示语言名称）：
  字体，SIL OFL 1.1。
- [Phosphor Icons](https://phosphoricons.com)（Helena Zhang、Tobias Fried），通过
  [egui-phosphor](https://github.com/amPerl/egui-phosphor) 使用：图标，MIT。
- [KaTeX](https://katex.org)（Khan Academy）和 [marked](https://marked.js.org)：
  聊天和笔记中的公式与 Markdown，MIT；
  [DOMPurify](https://github.com/cure53/DOMPurify)（Cure53，Apache-2.0 OR MPL-2.0）在显示前清理这些 HTML。
- [Hershey 字体](https://en.wikipedia.org/wiki/Hershey_fonts)（Dr. A. V. Hershey，美国国家标准局；
  `.jhf` 格式由 Cognition Inc. 的 James Hurt 制定；文件来自 Kamal Mostafa 的
  [hershey-fonts](https://github.com/kamalmostafa/hershey-fonts)）：聊天在画板上书写所用的单线字母
  （`src/handwrite/`，自由许可，需注明出处），并制作成手写笔记所用的 OpenSlate Corsivo 和
  OpenSlate Stampatello 字体（`assets/fonts/OpenSlate*.ttf`）。
- [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness)：聊天功能的灵感来源
  （仅借鉴思路，未复制代码）。
- 由 Anthropic 的 [Claude](https://claude.ai) 作为 AI 结对程序员参与编写。

包含许可证和作者信息的完整 crate 列表见 [THIRD-PARTY.md](THIRD-PARTY.md)。
逐文件的署名信息见 [NOTICE](NOTICE)。

## 许可证

Copyright © 2026 Marco Paolillo.

OpenSlate 是自由软件：你可以依据 GNU General Public License 第 3 版或（由你选择）任何更高版本的条款
重新发布和/或修改它。参见 [LICENSE](LICENSE)。之所以采用 GPL，是因为蓝牙协议代码源自 Tuhi。
