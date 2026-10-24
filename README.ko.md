<p align="center">
  <img src="assets/icon/openslate-256.png" width="128" height="128" alt="OpenSlate 아이콘">
</p>

<h1 align="center">OpenSlate</h1>

<p align="center">
  <b>Wacom Bamboo Slate</b>(SmartPad 제품군)를 위한 무료 오픈 소스 동반 앱입니다.<br>
  실시간 보드 · 오프라인 동기화 · 펜으로 Word 및 PDF 문서 서명 · 손글씨를 텍스트로 변환 · 페이지 위에서 바로 연습 문제를 고쳐 주는 AI 어시스턴트
</p>

<p align="center">
  <a href="https://github.com/rmpaolillo/OpenSlate/releases/latest"><img alt="최신 릴리스" src="https://img.shields.io/github/v/release/rmpaolillo/OpenSlate?sort=semver"></a>
  <a href="https://github.com/rmpaolillo/OpenSlate/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/rmpaolillo/OpenSlate/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="라이선스: GPL-3.0-or-later" src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue"></a>
</p>

*[English](README.md) · [Italiano](README.it.md) · [Español](README.es.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Українська](README.uk.md) · [中文](README.zh.md) · [日本語](README.ja.md) · 한국어*

<p align="center">
  <img src="assets/screenshots/chat.png" alt="OpenSlate: AI 어시스턴트가 보드 위의 손글씨 수학 연습 문제를 확인하고 빨간 잉크로 고쳐 줍니다" width="900">
</p>

---

## 만든 이유

저는 Bamboo Slate를 가지고 있고 매일 사용합니다. 글을 쓰고, 연습 문제를 풀고, 문서에
서명할 때 씁니다. Wacom이 이 패드가 의존하는 앱인 Inkspace의
[지원 종료를 발표](https://support.wacom.com/hc/en-us/articles/36070902398231-Bamboo-Paper-Inkspace-Wacom-Notes-application-End-of-Life-Notification)하면서,
이 하드웨어는 곧 문진 신세가 될 처지였습니다.

그래서 제 필요에 맞춰 직접 앱을 만들었습니다. **OpenSlate는 바이브 코딩 프로젝트입니다**:
원하는 것을 설명하고 AI 코딩 어시스턴트([Claude](https://claude.ai))와 함께 작성했으며,
처음부터 거창한 설계를 따르기보다 실제 사용을 바탕으로 반복해 다듬었습니다. 제 macOS에서는
잘 작동합니다. 다른 Slate 사용자에게도 도움이 될까 해서 공유합니다. 다듬어지지 않은 부분이
있을 수 있고 코드 주석은 이탈리아어로 되어 있습니다(인터페이스 자체는 9개 언어로 번역되어 있어요).
이슈와 풀 리퀘스트는 언제든 환영합니다. [CONTRIBUTING.md](CONTRIBUTING.md)를 참고하세요.

> OpenSlate는 독립 프로젝트입니다. **Wacom과 제휴하거나 Wacom의 보증 또는 지원을 받는 프로젝트가
> 아닙니다**. "Wacom", "Bamboo", "Inkspace"는 각 소유자의 상표입니다.

## 한눈에 보기

- **종이에 쓰고 화면에서 봅니다.** Bamboo Slate는 모든 획을 Bluetooth로 전송하며,
  컴퓨터와 떨어진 곳에서 쓴 페이지는 나중에 보관함으로 다운로드됩니다.
- **페이지에 직접 써 주는 AI 튜터.** 연습 문제를 확인해 달라고 하면 맞은 부분에 체크 표시를 하고,
  틀린 부분에 동그라미를 친 뒤 옆에 손글씨로 정답을 써 주고, 채팅에서 수식과 함께 설명해 줍니다.
  함수의 그래프를 페이지에 그려 줄 수도 있습니다.
- **펜으로 Word 및 PDF 문서에 서명**하고 같은 형식으로 저장합니다.
- **손글씨 → 편집 가능한 노트**: 표와 수식까지 포함해 Markdown, Word 또는 PDF로 내보내며,
  인쇄체나 손글씨 글꼴로 표시할 수 있습니다.
- **필요한 형식으로 내보내기**: 검색 가능한 PDF, 손글씨를 편집 가능한 잉크로 담은 Word, 가벼운 JPEG/WebP, Xournal++, 페이지가 써지는 모습의 움직이는 GIF/SVG나 MP4 동영상, 변환된 텍스트로 만든 LaTeX 또는 Typst.
- **원하는 AI 선택**: 로컬 모델(LM Studio, Ollama) 또는 클라우드 제공업체. API 키는 절대
  평문으로 저장되지 않습니다.
- **채팅에 말하기.** 마이크로 받아쓰기: Whisper가 컴퓨터에서 오프라인으로 받아씁니다.
- **튜터가 소리로 답합니다.** 시스템 음성으로 답변을 읽어 줍니다.
- **무료 오픈 소스**이며 9개 언어를 지원하고, macOS(주 플랫폼), Linux, Windows에서 사용할 수 있습니다.

## 기능

- **실시간 보드.** Bluetooth로 연결하면 패드에 쓰는 내용을 쓰는 즉시 전체 화면으로 볼 수
  있습니다. 펜, 형광펜(`H`, 펜을 길게 눌러 전환하는 기능은 옵션에서 켤 수 있습니다), 지우개(지그재그
  긋기 또는 지우개 모드), 색상, 선 굵기, 실행 취소/다시 실행, 회전/뒤집기, 트랙패드 제스처를
  이용한 확대/축소를 사용할 수 있습니다.
- **용지 스타일.** 줄 노트, 이탈리아 초등학교식 괘선, 모눈(4/5/10 mm), 그래프 용지,
  데카르트 좌표평면, 점선, 등각, 오선지, 코넬 노트 중에서 고를 수 있습니다. 괘선은 실제 크기로
  그려지며 화면에만 표시되므로 내보낸 파일은 깔끔하게 유지됩니다.
- **오프라인 페이지.** 컴퓨터와 떨어진 곳에서 쓴 페이지를 날짜별로 묶인 보관함으로 다운로드합니다.
  보관함에서 페이지 이름을 바꾸거나, 내보내거나, OCR을 실행하거나, 휴지통에서 되살릴 수 있습니다.
- **Word 및 PDF 문서 서명.** `.docx`/`.doc`/`.odt`/`.rtf` 또는 `.pdf` 파일을 열고 펜으로 쓰거나
  서명한 다음 **같은 형식으로** 사본을 저장합니다. Word 파일에서는 손글씨가 문서 안의 잉크가 되므로
  Word, Pages, LibreOffice 모두에서 표시됩니다. PDF에서는 손대지 않은 원본 위에 벡터 획으로
  들어갑니다. 페이지는 [WordCraft](https://github.com/storytold/wordcraft) 및
  [PdfCraft](https://github.com/storytold/pdfcraft) 엔진으로 렌더링되어 어떤 배율에서도
  선명하게 유지됩니다.
- **이미지에도 쓸 수 있습니다.** SVG, PNG, JPEG(WebP, TIFF, BMP, GIF도 가능)를 열거나, 클립보드에
  스크린샷이나 사진이 있을 때 **붙여넣기**(`Cmd+V`)를 누르세요. 그 위에 쓴 다음 같은 형식과 크기의
  이미지로 다시 저장합니다. SVG는 손글씨를 벡터 경로로 유지하고
  ([VectorCraft](https://github.com/storytold/vectorcraft)), 사진은 색상 프로필을 유지한 채
  원본 해상도로 손글씨가 그려집니다([PhotoCraft](https://github.com/storytold/photocraft)).
- **Word처럼 복사하고 붙여넣기.** 홈 탭은 **붙여넣기**와 **복사**로 시작합니다. 복사를 누르면
  페이지가 이미지로 클립보드에 들어가므로 AI 채팅, Word, 이메일에 바로 쓸 수 있습니다.
- **손글씨 → 텍스트.** 이미지를 인식할 수 있는 AI 모델이 페이지를 실수까지 포함해 그대로
  Markdown 노트로 옮겨 적습니다. 제목, 목록, 체크박스, 표, 그리고 수식이 있으면 LaTeX 수식까지
  포함됩니다. 그림, 그래프, 다이어그램은 실제 펜 획을 바탕으로 선명한 SVG 그림으로 다시 그려져
  노트 옆에 놓입니다. 노트는 앱 안에서 Typora처럼 편집 가능한 페이지로 열리며, **내보내기**로
  Markdown(그림은 옆에 함께), Word 또는 PDF로 저장할 수 있습니다. **글씨**는 노트를 인쇄체 또는
  손글씨로 보여 주는데, AI 채팅이 보드에 쓰는 것과 같은 글자로 필기체나 정자체를 고를 수 있습니다.
  손글씨는 Word 파일(어느 컴퓨터에서나 보이도록 포함됨)과 PDF에도 들어갑니다.
- **무엇이든 돕는 AI 어시스턴트.** 채팅은 페이지 옆에 있으며 보드, 열려 있는 문서 또는 노트를
  볼 수 있습니다. 요약, 깔끔하게 다시 쓰기, 할 일 추출, 글 초안 작성, 쓴 내용의 오류 확인,
  질문에 대한 답변, 노트에 쓰기, 웹 검색, EU VAT 번호 확인에 활용하세요. 채팅에 이미지를
  붙여넣을 수 있고(`Cmd+V`, 끌어서 놓기 또는 클립 아이콘), 핀치 동작이나 `Cmd +` / `Cmd −`로
  글자 크기를 키우거나 줄일 수 있습니다. 선택한 모델이 이미지를 볼 수 없는 경우(텍스트 전용 모델),
  먼저 *페이지 읽기 (OCR)* 항목에서 선택한 에이전트가 페이지를 읽어 텍스트로 넘겨줍니다. 그 뒤에는
  OpenAI 호환 제공업체로 이루어진 대체 체인이 있습니다: LM Studio, Ollama, DeepSeek, Google Gemini,
  Anthropic Claude. 컨텍스트 사용량을 표시하고 자동으로 압축합니다. 문서 탭을 채팅으로 끌어다 놓으면
  첨부됩니다.
- **문서 안의 차트와 편집.** 채팅에 그래프를 요청하면("두 방정식과 해를 그려 줘")
  [Typst](https://typst.app)와 [Lilaq](https://lilaq.org)로 LaTeX 스타일의 그래프를 그려 줍니다:
  함수, 점, 직선, 막대, 매개변수 곡선. 채팅에 문서가 첨부되어 있으면 앱 안에서 실행되는 Craft
  제품군의 MCP 서버를 통해 문서를 읽고 **편집**하며 차트를 넣을 수도 있습니다: Word에는 WordCraft,
  PDF에는 PdfCraft, SVG에는 VectorCraft, 사진에는 PhotoCraft(자르기, 크기 조정, 회전, 밝기, 레벨,
  곡선, 흑백, 선명하게… 이미지는 형식을 유지합니다), 그리고 Markdown용 텍스트 도구가 있습니다.
  모든 변경 사항은 채팅 옆의 문서에 바로 표시되며 채팅에서 한 번 클릭으로 되돌릴 수 있고, 디스크의
  원본 파일은 절대 건드리지 않습니다. 텍스트 변환 노트에서는 차트가 손으로 그린 그림을 대체할 수
  있으며, 보드 시트에서는 손으로 그린 그래프(축과 레이블로 자동 탐지)를 대체하거나 글씨 아래에
  들어갑니다. 모델이 답변에 작성한 SVG는 그림으로 표시됩니다.
- **PDF 양식 작성.** PDF 양식을 채팅에 첨부하고 "내 정보로 이 양식을 작성해 줘"라고 하면 PdfCraft로
  필드를 읽고 채웁니다. 필드가 없는 인쇄 또는 스캔 양식은 먼저 필드를 만들고(PdfCraft가 빈칸을 찾아
  옆의 레이블로 이름을 붙입니다), 아니면 채팅이 텍스트, 체크 표시, 날짜를 알맞은 곳에 씁니다. 사용자가
  알려 준 정보만 쓰고 빠진 것은 물어봅니다. 서명은 절대 하지 않습니다. 서명은 사용자가 펜으로 하며,
  작성한 값은 서명된 사본에도 그대로 남습니다.
- **채팅에 말하기.** 보내기 옆의 마이크를 클릭하거나, 말하는 동안 오른쪽 Alt(Option) 키를 누르고
  있으세요: 말한 내용이 메시지 입력란에 들어가고, 고친 다음 보낼 수 있습니다. Whisper가
  [whisper.cpp](https://github.com/ggml-org/whisper.cpp)(Mac에서는 GPU)로 컴퓨터에서 오프라인으로 받아쓰므로 목소리가 컴퓨터
  밖으로 나가지 않습니다. 모델(547MB)은 처음에 동의한 경우에만 한 번 다운로드됩니다. 더 가벼운
  모델이나 온라인 서비스(OpenAI, Groq, Mistral…)는 `ai.toml`(`[voice]`)에서 고를 수 있습니다. Esc를
  누르면 취소됩니다. 처음에는 macOS가 마이크 사용 권한을 요청합니다.
  앱 언어와 상관없이 말한 언어로 받아씁니다(`[voice]`의 `language`로 고정할 수도 있습니다).
- **말하는 튜터.** 각 답변 아래의 **듣기**가 소리 내어 읽어 줍니다(Esc로 멈춤). 질문을 받아쓰기로 했다면 답변도 자동으로 음성으로 돌아옵니다. 인터넷 없이 시스템
  음성을 씁니다: macOS의 `say`(«프리미엄» 또는 «고급» 음성이 가장 자연스럽습니다: 시스템 설정 → 손쉬운 사용 → 읽기 및 말하기), Windows 음성,
  Linux의 `espeak-ng`; 또는 `ai.toml`(`[speech]`)에서 `/audio/speech`를 지원하는 온라인 서비스(OpenAI…)를 쓸 수 있습니다.
  답변의 언어로 읽습니다.
- **깔끔하게 옮겨 쓰기.** 홈 → «깔끔하게 옮겨 쓰기»: AI가 보드에 쓴 내용을 수식까지 읽어서 같은 자리에 또렷한 손글씨로 다시 씁니다.
  수식은 정돈된 공책처럼 조판됩니다(세로 분수, 지수, 근호, 연립방정식의 중괄호, 합, 적분, 극한). 펜과 같은 진짜 잉크이며, 실행 취소하면
  원래 글씨로 돌아갑니다. 그림, 형광펜, AI의 표시, 읽을 수 없는 줄은 그대로 남습니다. 종이를 읽는 에이전트(`[ocr]`)를 씁니다.
- **채팅이 보드에 씁니다.** "덧셈을 확인하고 틀린 곳을 표시해 줘", "마감일에 밑줄을 그어 줘",
  "2번째 줄 옆에 결과를 써 줘"라고 요청하면 보드 시트를 한 줄 한 줄, 한 단어 한 단어 읽은 다음
  강조 표시, 밑줄, 동그라미, 취소선, 상자, 화살표, 체크 표시와 X 표시를 하고, 손글씨로 씁니다
  (정자체 또는 필기체, [Hershey](https://en.wikipedia.org/wiki/Hershey_fonts) 단선 글꼴의
  획으로 이루어짐). 요청하면 시트에 있는 내용을 옮기거나, 키우거나, 색을 바꾸거나, 지우기도 합니다
  ("제목을 더 크게 파란색으로", "마지막 줄을 아래로 옮겨 줘", "내 형광펜 표시를 지워 줘"). 스스로는
  추가만 합니다. 시트의 대부분을 지우려면 채팅에서 여러분의 동의가 필요합니다. 채팅이 하는 모든
  작업은 일반 잉크이며(여러분의 잉크처럼 저장, 내보내기, 지우기가 가능), 채팅에서 한 번 클릭하면
  전부 다시 없앨 수 있습니다.
- **채팅이 보관함을 알고 있습니다.** "10월 예산에 대해 쓴 페이지를 찾아 줘", "어제 노트를 보드에
  열어 줘", "보드를 '회의'로 저장하고 새 페이지를 시작해 줘", "이번 주 페이지를 PDF 하나로 내보내 줘":
  제목, 날짜, 텍스트 변환 내용을 검색하고, 페이지를 살펴보고, 보드에 열고, 보드를 저장하고(원래
  페이지를 업데이트), 이름을 바꾸고, 평소 쓰는 내보내기 폴더로 내보내고(절대 덮어쓰지 않음),
  페이지를 휴지통으로 옮기거나 되돌립니다. 휴지통으로 옮기는 작업은 항상 채팅에서 여러분의 동의를
  기다립니다.
- **나만의 MCP 서버.** Claude Desktop이나 DSH에서처럼 `ai.toml`에 원하는 MCP 서버(stdio)를
  추가하면 그 도구를 채팅에서 사용할 수 있습니다.
- **웹에서 실시간으로.** 공유 링크를 사용하면 홈 네트워크의 TV, 휴대폰 또는 PC에서 보드를
  실시간으로 볼 수 있습니다. 고정 엔드포인트는 `/page.pdf`, `/page.svg`, `/page.png`,
  `/state.json`입니다. 실시간 보드는 홈 네트워크에 열려 있지만, 채팅, 노트, 대화에는 OpenSlate가
  보여 주는 링크에 포함된 키(`?k=…`, 이후 브라우저가 기억함)가 필요합니다. 서버는 IP 주소,
  `localhost`, `.local` 이름에만 응답하고 다른 웹사이트에서 오는 쓰기 요청은 거부하므로, 방문한
  웹 페이지가 앱을 조작할 수 없습니다.
- **아무것도 잃어버리지 않습니다.** 보관함에 없는 내용이 보드에 있는 상태로 종료하면
  *저장하고 종료 / 저장하지 않고 종료 / 종료하지 않음*을 묻습니다. 앱이 묻지 않고 종료될 때(Dock,
  시스템 종료, 업데이트)는 보드가 자동으로 보관함에 들어가며, 인터페이스가 비정상 종료되는 경우에도
  보드가 저장됩니다(원인은 데이터 폴더의 `crash.log`에 기록됩니다). 노트는 버전 확인과 함께
  저장되므로 AI나 다른 앱의 편집 내용이 모르는 사이에 덮어쓰이는 일이 없습니다. 다운로드가 중단되면
  이미 패드에서 삭제된 페이지도 원본 안전 사본에서 복구됩니다.
- **종이만.** 제목 표시줄의 복사/붙여넣기 옆 버튼(또는 `Cmd+Shift+F`, 보기 → 종이만)을 누르면
  메뉴 막대와 리본이 숨겨집니다. 다시 클릭하면 돌아옵니다.
- **다른 형식.** 텍스트로 변환된 페이지는 **검색 가능한 PDF**가 됩니다. 텍스트가 손글씨 위에 보이지 않게 얹혀 있어 Spotlight와 모든 PDF 뷰어에서 찾을 수 있고 복사도 할 수 있습니다. **다른 형식**에는 JPEG와 WebP(가벼움, 보내기용), 손글씨를 편집 가능한 진짜 Word 잉크로 담은 **Word**(다음 페이지에 변환된 텍스트), **Xournal++**(`.xopp`), 페이지를 한 획씩 다시 쓰는 **움직이는 GIF 또는 SVG**나 **MP4 동영상**(동영상은 시스템 인코더 사용: macOS는 AVFoundation, Windows는 Media Foundation, Linux는 ffmpeg 또는 동의하면 처음 사용할 때 Cisco에서 내려받는 OpenH264), 변환된 텍스트로 만든 **LaTeX** 또는 **Typst**(노트도 이 형식으로 내보낼 수 있음)가 있습니다. AI 채팅도 이 모든 형식으로 내보낼 수 있습니다.
- **저장 및 내보내기.** 실제 A4 크기의 벡터 PDF와 SVG, 또는 PNG로 내보냅니다. 저장하거나 내보낼
  때마다 시스템의 "다른 이름으로 저장" 대화상자가 열리므로 파일이 어디에 저장되는지 항상 확인할 수
  있습니다. 대화상자는 마지막으로 선택한 폴더에서 시작합니다(처음에는 `exports/`). 페이지를
  이미지로 복사(`Cmd+C`)해서 어디에나 붙여넣을 수도 있습니다.
- **여러분의 언어.** 인터페이스 전체(메뉴, 리본, 옵션, 채팅, 노트, AI 설정)를 영어, 이탈리아어,
  스페인어, 프랑스어, 독일어, 우크라이나어, 중국어, 일본어, 한국어로 사용할 수 있습니다. 시스템
  언어를 따르며, 파일 → 옵션 → 모양 → 언어에서 바꿀 수 있습니다. AI는 여러분이 쓰는 언어로
  답합니다.

## 스크린샷

스크린샷은 영어 인터페이스를 보여 줍니다(8개 언어가 더 내장되어 있습니다). 손글씨, 문서, AI 답변은
이 이미지를 위해 준비한 예시 콘텐츠이며, 페이지의 표시와 그래프는 앱 자체의 보드 도구로
그렸습니다.

| | |
|---|---|
| **페이지가 스스로 다시 써짐**: 움직이는 GIF로 내보내기, 한 획씩 | **다른 형식**: JPEG, WebP, 잉크가 들어간 Word, Xournal++, 움직이는 GIF/SVG, MP4 동영상, LaTeX, Typst |
| <img src="assets/screenshots/writing.gif" alt="움직이는 GIF: 페이지가 한 획씩 써짐" width="440"> | <img src="assets/screenshots/formats.png" alt="다른 형식 메뉴" width="440"> |
| **페이지 위의 그래프**: 채팅이 함수를 그려 연습 문제 아래에 넣습니다 | 펜으로 **Word 또는 PDF 문서에 서명**하고 같은 형식으로 저장합니다 |
| <img src="assets/screenshots/graph.png" alt="AI 채팅이 함수의 그래프를 그려 손글씨 페이지에 넣습니다" width="440"> | <img src="assets/screenshots/sign.png" alt="펜으로 서명한 Word 문서" width="440"> |
| **손글씨 → 텍스트**: 수식까지 포함해 편집 가능한 노트로 옮겨 적은 페이지 | **손글씨 노트**: 같은 노트를 필기체 손글씨 글꼴로 표시 |
| <img src="assets/screenshots/transcribe.png" alt="표와 수식이 포함된 노트로 옮겨 적은 손글씨 페이지" width="440"> | <img src="assets/screenshots/notes.png" alt="OpenSlate 필기체 손글씨 글꼴로 표시한 노트" width="440"> |
| 패드에서 다운로드한 페이지를 날짜별로 묶은 **보관함** | **다크 테마**와 용지 스타일(여기서는 5 mm 모눈) |
| <img src="assets/screenshots/library.png" alt="손글씨 페이지 보관함" width="440"> | <img src="assets/screenshots/dark.png" alt="모눈 용지와 AI 채팅이 열린 다크 테마" width="440"> |

## 설치

바로 설치할 수 있는 빌드는
[**Releases**](https://github.com/rmpaolillo/OpenSlate/releases) 페이지에 게시되며, 각각 증가하는
버전 번호(`v0.3.0`, `v0.3.1`, …)가 붙습니다.

| 플랫폼 | 파일 | 상태 |
|---|---|---|
| macOS 11+, Apple Silicon (M1, M2…) | `OpenSlate-<version>-macos-arm64.dmg` | 주 플랫폼, 매일 사용 |
| macOS 11+, Intel | `OpenSlate-<version>-macos-intel.dmg` | 빌드 및 실행 가능, 테스트 적음 |
| macOS 11+, 둘 다 (크기 두 배) | `OpenSlate-<version>-macos-universal.dmg` | 0.2.x에서 앱 내 업데이트용 |
| Linux x86_64 (Debian/Ubuntu) | `openslate_<version>_amd64.deb` | 빌드 및 실행 가능, 테스트 적음 |
| Linux x86_64 (기타 배포판) | `openslate-<version>-linux-x86_64.tar.gz` | 빌드 및 실행 가능, 테스트 적음 |
| Windows x86_64 | `openslate-<version>-windows-x86_64.zip` | **실험적** |

### macOS

1. `.dmg`를 열고 **OpenSlate**를 **응용 프로그램**으로 끌어다 놓으세요.
2. Apple Developer ID로 빌드된 릴리스는 서명 및 공증되어 있어 경고 없이 열립니다. Developer ID
   없이 만든 빌드(아래 "서명 및 공증" 참고)는 임시(ad hoc) 서명되어 있으므로, 처음 열 때 macOS가
   "열 수 없습니다"라고 표시합니다. 이때 **시스템 설정 → 개인정보 보호 및 보안**을 열고
   **그래도 열기**를 클릭하거나, 다음을 실행하세요.
   ```bash
   xattr -dr com.apple.quarantine /Applications/OpenSlate.app
   ```
   이 작업은 처음 설치할 때만 필요합니다. 업데이트는 앱이 직접 다운로드하므로 바로 열립니다.
3. 요청이 표시되면 **Bluetooth** 접근을 허용하세요.

그 밖에 설치할 것은 없습니다. 모든 파일은 [Craft 제품군](https://github.com/storytold)의
라이브러리로 열고 저장합니다(아래 "파일과 형식" 참고).

### Linux

```bash
sudo apt install ./openslate_<version>_amd64.deb     # Debian / Ubuntu
# or unpack the tar.gz and run ./openslate
```

실행하려면 BlueZ, GTK 3, WebKitGTK 4.1이 필요합니다. 사용자 계정에 Bluetooth 사용 권한이 있어야
합니다.
MP4 동영상 내보내기에는 설치되어 있으면 `ffmpeg`(`sudo apt install ffmpeg`)를 사용합니다. 없으면 처음에
Cisco 사이트에서 Cisco의 무료 OpenH264 인코더(약 1MB)를 내려받아도 되는지 먼저 묻습니다.
답변을 소리 내어 읽으려면 `espeak-ng`(`sudo apt install espeak-ng`) 또는 speech-dispatcher를 씁니다. macOS와 Windows는 자체 음성을 씁니다.

### Windows (실험적)

압축을 풀고 `openslate.exe`를 실행하세요. Windows 10/11에 기본 포함된 Microsoft Edge WebView2
런타임이 필요합니다. Windows "N" 에디션에서는 MP4 동영상 내보내기에 Microsoft의 Media Feature Pack도
필요합니다. Windows에서는 모든 기능이 테스트되지는 않았습니다. 버그 보고는 언제나
환영합니다.

## 파일과 형식

모든 파일은 ArtCraft의 Craft 제품군 라이브러리로 열고, 표시하고, 저장하며, 해당 앱의 인터페이스나
외부 프로그램은 사용하지 않습니다.

| 파일 | 라이브러리 |
|---|---|
| Word (.docx, .doc, .odt, .rtf), Markdown, .txt, .html | [WordCraft](https://github.com/storytold/wordcraft): 페이지를 열고 레이아웃하며, 잉크와 함께 Word로 저장 |
| PDF | [PdfCraft](https://github.com/storytold/pdfcraft): 페이지, 미리보기 이미지, 텍스트. 서명된 사본은 PdfCraft의 PDF 라이브러리로 저장 |
| SVG | [VectorCraft](https://github.com/storytold/vectorcraft) |
| 사진 (PNG, JPEG, WebP, TIFF, BMP, GIF) | [PhotoCraft](https://github.com/storytold/photocraft) |
| 보드 및 보관함 내보내기 (PDF, SVG, PNG, JPEG, WebP) | VectorCraft가 그리고 기록하며, PNG, JPEG, WebP 파일은 PhotoCraft가 기록 |
| 움직이는 GIF와 SVG, Xournal++, LaTeX, Typst, 검색 가능한 PDF의 숨은 텍스트 | OpenSlate가 직접 기록 (`src/formats.rs`, `src/mdconv.rs`) |
| MP4 동영상 | 시스템 인코더: AVFoundation(macOS), Media Foundation(Windows), Linux는 ffmpeg 또는 처음 사용할 때 내려받는 Cisco OpenH264 (*OpenH264 Video Codec provided by Cisco Systems, Inc.*) |

문서로 연 Markdown 파일은 노트 페이지처럼 보입니다: Source Serif 본문, Inter 제목, 가벼운 표.
수식(`$…$`, `$$…$$`, `\(…\)`, `\[…\]`)은 [Typst](https://typst.app)가 LaTeX 스타일로 그리며
(LaTeX는 [MiTeX](https://github.com/mitex-rs/mitex)로 변환), 옆에 있는 그림(`<img src="…">`,
`![…](…)`, VectorCraft가 그린 SVG)은 이미지로 표시됩니다. 서명한 Markdown, .txt, .html 파일은
PDF로 저장됩니다(이 형식들은 잉크를 담을 수 없기 때문입니다).

## 업데이트

OpenSlate는 Craft 앱들처럼, 확인을 거친 뒤 스스로 업데이트합니다. GitHub에 새 릴리스가 게시되면
리본 아래에 파란색 막대가 나타나 이를 알리고 **업데이트**를 제안합니다. 그러면 업데이트를
다운로드하고, 릴리스의 `SHA256SUMS.txt` 및 개발자 서명과 대조해 확인한 뒤, 실행 중인 앱 자리에
설치하고 **지금 다시 시작**을 제안합니다(또는 다음 실행 때 새 버전을 사용해도 됩니다). 옵션 → 고급에서
묻지 않고 설치하도록 하거나 확인 자체를 끌 수 있으며, 파일 → 도움말 및 정보 → 업데이트 확인을
누르면 즉시 확인합니다.

설정, AI 에이전트, 보관함, 문서, 내보낸 파일은 앱 바깥의 데이터 폴더(macOS:
`~/Library/Application Support/OpenSlate`)에, API 키는 키체인에 있으므로 업데이트가 이를 건드리는
일은 없습니다. macOS에서는 업데이트가 `OpenSlate.app`을 교체하며(앱이 직접 다운로드하므로
Gatekeeper가 다시 묻지 않습니다), `.deb`의 경우 `pkexec`를 통해 암호를 묻고, Windows에서는 앱을
종료할 때 설치됩니다.

## 처음 연결하기

- **처음이거나** Inkspace를 사용한 뒤라면: LED가 파란색으로 깜박일 때까지 패드 버튼을 약 6초간
  누르고 있으세요. 그런 다음 **연결**을 클릭하고, 요청이 표시되면 버튼을 한 번 누르세요.
- **그 이후에는**(LED 녹색): 버튼을 짧게 누른 다음 **연결**을 클릭하세요.

패드는 한 번에 하나의 앱만 기억합니다. Inkspace로 다시 돌아가면 OpenSlate를 다시 등록해야 합니다.

## 데이터 저장 위치

설치된 OpenSlate는 보관함, 문서, 채팅, 설정을 다음 위치에 보관합니다.

- macOS: `~/Library/Application Support/OpenSlate`
- Linux: `~/.local/share/openslate` (또는 `$XDG_DATA_HOME/openslate`)
- Windows: `%APPDATA%\OpenSlate`

`OPENSLATE_DATA` 환경 변수로 이 위치를 바꿀 수 있습니다. 소스 폴더에서 `cargo run`으로 시작하면
대신 그 폴더에서 작업합니다. 그곳에는 `crash.log`(앱이 비정상 종료된 적이 있는 경우),
`web.key`(웹 실시간 링크의 키), `updates/`(업데이트 후 보관되는 이전 버전의 앱)도 있습니다.
Linux에서는 다운로드를 허용한 경우 `codecs/`에 Cisco의 OpenH264 인코더가 있습니다.
`models/whisper/`에는 다운로드한 뒤 채팅 음성 입력용 Whisper 모델이 있습니다.

## AI 제공업체와 API 키

AI 기능은 선택 사항입니다. 제공업체는 **파일 → 옵션 → AI** 또는 처음 사용할 때 주석과 함께
생성되는 `ai.toml` 파일에서 설정합니다. OpenAI `/chat/completions` API를 지원하는 서버라면 무엇이든
작동하며, 키가 필요 없는 로컬 서버(LM Studio, Ollama)도 포함됩니다.

**페이지 읽기 (OCR)** 기능에는 별도의 설정이 있습니다(`ai.toml`의 `[ocr]` 또는 *AI 설정 → 페이지 읽기*).
*텍스트 변환*과 텍스트 전용 채팅 모델을 위해 손글씨를 읽을 에이전트를 고르거나, 비워 두면 목록에서
이미지를 볼 수 있는 첫 번째 에이전트를 사용합니다. Gemini처럼 빠른 에이전트를 권장하며,
`fallback = false`는 "항상 그 에이전트만 사용"을 뜻합니다.

**누가 답했는지.** 채팅의 모든 답변에는 실제로 답을 쓴 에이전트와 모델이 표시됩니다(에이전트는 순서대로 시도되므로 첫 번째가 아닐 수도 있습니다). 또한 채팅은 모델에게 자신이 무엇인지(서비스, 주소, 모델)를 알려 줍니다. 모델은 자신에 대해 잘 모르고 다른 모델이라고 말할 수 있기 때문입니다: 답변 위의 표시를 믿으세요.

**음성 입력**(`ai.toml`의 `[voice]`): `engine = "whisper"`(기본값, 컴퓨터에서) 또는 `/audio/transcriptions`를 지원하는 에이전트 이름(OpenAI, Groq, Mistral…; 채팅이 쓰지 않도록 `enabled = false`로); `model`(`large-v3-turbo`, `small`, `base` 또는 서비스의 받아쓰기 모델); `language`(비우면 말한 언어); `auto_send = true`이면 말을 멈추자마자 메시지를 보냅니다.

**답변 읽어 주기**(`ai.toml`의 `[speech]`): `engine = "system"`(기본값: 시스템 음성) 또는 `/audio/speech`를 지원하는 에이전트 이름(OpenAI…); `model`은 그 에이전트의 모델(예: `gpt-4o-mini-tts`); `voice`(비우면 해당 언어에 가장 좋은 음성, 또는 `"Alice"` 같은 이름); `language`(비우면 답변의 언어); `rate`(0.5~2); `auto = "voice"`는 질문을 받아쓰기로 했을 때 자동으로 읽고, `"always"` 또는 `"never"`(듣기 버튼은 남음).

**API 키는 절대 평문으로 저장되지 않습니다.** `ai.toml`에는 각 키를 *어디에서* 찾을지만 적습니다.

```toml
key = "env:DEEPSEEK_API_KEY"              # environment variable
key = "keychain:DEEPSEEK_API_KEY"         # macOS Keychain
key = "env:GEMINI_API_KEY | keychain:GEMINI_API_KEY"   # first one found wins
```

```bash
# macOS Keychain (recommended: also works when the app is opened from Finder)
security add-generic-password -a "$USER" -s DEEPSEEK_API_KEY -w 'sk-...'
```

### MCP 서버

채팅은 외부 [MCP](https://modelcontextprotocol.io) 서버(stdio 전송)의 도구를 사용할 수 있으며,
Claude Desktop에서처럼 `ai.toml`에서 설정합니다.

```toml
[[mcp]]
name = "filesystem"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/Users/me/Documents"]
env = { TOKEN = "keychain:MY_TOKEN" }   # values resolved like API keys
# enabled = false   to turn it off; timeout = 120 (seconds per tool)
```

OpenSlate는 채팅에 필요할 때 프로그램을 시작하고 계속 실행 상태로 유지합니다. 그 도구들은
`name__tool` 형태로 모델에 전달됩니다. Craft 제품군 자체의 MCP 서버(WordCraft, PdfCraft,
VectorCraft)는 설정이 필요 없습니다. 앱 안에서 채팅에 첨부된 문서를 대상으로 실행됩니다.

## 소스에서 빌드하기

**Rust 1.95 이상**이 필요합니다(`rustup update`).

```bash
git clone https://github.com/rmpaolillo/OpenSlate.git
cd OpenSlate
cargo run --release                  # GUI
cargo run --release -- gui --demo    # try it without a pad
cargo run --release -- --help        # command-line tools (scan, sync, live, pdf, ai, …)
cargo test
```

Linux 빌드 의존성(Debian/Ubuntu):

```bash
sudo apt install pkg-config libgtk-3-dev libwebkit2gtk-4.1-dev libsoup-3.0-dev \
                 libjavascriptcoregtk-4.1-dev libdbus-1-dev libxkbcommon-dev libasound2-dev cmake
```

macOS에서는 Xcode 명령줄 도구(`xcode-select --install`)와 채팅 음성 입력용 whisper.cpp를 빌드하는 CMake(`brew install cmake`)가 필요합니다. 선택 사항인 Apple Vision
OCR과 MP4 동영상용 작은 Swift 도우미는 처음 사용할 때 컴파일되거나, 릴리스 `.app` 안에 포함되어 있습니다.

## 프로젝트 구조

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
src/speech.rs                                             답변 읽어 주기: 시스템 음성(say, SAPI, espeak-ng) 또는 온라인 서비스
src/inbella.rs, src/mathhand.rs                           깔끔하게 옮겨 쓰기: AI가 줄을 읽고, 조판된 수식으로 다시 씀
src/ui/sysdlg.rs                                          system dialogs (rfd; on Linux GTK in the window's thread)
src/fonts.rs                                              the fonts embedded in the app (one copy each)
src/demo.rs                                               sample pages for the screenshots (tests only)
packaging/, .github/workflows/                            app bundle, .deb, CI and releases
```

파일별 설명은 [이탈리아어 README](README.it.md#struttura)에 있습니다.

## 릴리스와 버전 관리

- `ci.yml`은 모든 푸시와 풀 리퀘스트마다 Linux와 macOS에서 빌드하고 테스트합니다.
- `release.yml`은 메인테이너가 시작할 때 실행됩니다(`main`에서 Actions → Release → Run workflow).
  macOS(Apple Silicon, Intel, 유니버설), Linux, Windows용으로 빌드하고 자동 생성된 릴리스 노트와 함께
  GitHub Release를 게시하며, 가장 최근 릴리스 두 개만 유지합니다. 릴리스 바이너리는 심볼이 제거되고
  크레이트 전체에 걸쳐 최적화됩니다(`lto = "thin"`, `codegen-units = 1`). 다른 라이브러리가 끌어오는
  드물게 쓰이는 라이브러리는 크기 기준으로 최적화되고, 각 글꼴은 한 번만 포함됩니다(`Cargo.toml`의
  `[profile.release]`와 `src/fonts.rs` 참고).
- 버전은 [유의적 버전](https://semver.org)을 따릅니다. `MAJOR.MINOR`는 `Cargo.toml`에서 가져오고,
  `PATCH`는 기존 태그(`v0.3.0`, 그다음 `v0.3.1`, …)를 바탕으로 자동으로 매겨집니다.
- 새 시리즈를 시작하려면(새 기능: `0.3.0`, 첫 안정 버전: `1.0.0`) `Cargo.toml`의 `version`을
  바꾸세요. 다음 릴리스는 패치 `0`부터 다시 시작합니다.
- 릴리스는 먼저 테스트를 실행하고 고정된 의존성(`Cargo.lock`)으로 빌드합니다. 각 패키지에는
  `LICENSE`, `NOTICE`, `THIRD-PARTY.md`, `THIRD-PARTY-LICENSES.txt`(모든 서드파티 라이선스의
  전문, `packaging/collect_licenses.py`로 생성)가 들어 있습니다.
- 모든 릴리스에는 `SHA256SUMS.txt`가 있으며, 앱은 업데이트를 설치하기 전에 다운로드한 파일을
  이 파일과 대조해 확인합니다.

### 서명 및 공증 (macOS)

Craft 앱들처럼 macOS가 경고 없이 앱을 신뢰하게 하려면, 릴리스에 Apple
**Developer ID Application** 인증서(Apple Developer Program, 연간 유료)가 필요합니다. 다음
저장소 시크릿을 추가하세요(Settings → Secrets and variables → Actions).

| 시크릿 | 값 |
|---|---|
| `APPLE_CERTIFICATE` | `.p12`로 내보낸 "Developer ID Application" 인증서를 base64로 인코딩한 값 (`base64 -i cert.p12 \| pbcopy`) |
| `APPLE_CERTIFICATE_PASSWORD` | `.p12`를 내보낼 때 정한 암호 |
| `APPLE_ID` | 개발자 계정의 Apple ID 이메일 |
| `APPLE_PASSWORD` | 해당 Apple ID의 앱 암호 (appleid.apple.com → 로그인 및 보안) |
| `APPLE_TEAM_ID` | 10자리 팀 ID (developer.apple.com → 멤버십) |

이 시크릿이 있으면 `release.yml`이 강화된 런타임(hardened runtime)과 타임스탬프로 앱에 서명하고,
Apple 공증 서비스로 보낸 뒤 티켓을 스테이플링하며, DMG에도 같은 작업을 합니다. 시크릿이 없으면
앱은 임시(ad hoc) 서명되며 릴리스는 그래도 작동합니다(처음 열 때 **그래도 열기** 사용).

## 크레딧

OpenSlate는 다음 프로젝트와 그 저작자들이 없었다면 존재하지 않았을 것입니다.

- **[Tuhi](https://github.com/tuhiproject/tuhi)**, Tuhi 기여자들 (GPL-2.0-or-later).
  SmartPad 제품군의 Bluetooth 프로토콜은 이들의 리버스 엔지니어링 작업을 옮겨 온 것입니다.
- **Craft 제품군**, ArtCraft Team 및 기여자들 (MIT OR Apache-2.0). 인터페이스 없이 라이브러리로
  사용합니다.
  - **[WordCraft](https://github.com/storytold/wordcraft)**: Word 문서(및 Markdown)를 잉크와 함께
    열고, 레이아웃하고, 저장합니다. Word 스타일의 인터페이스는 WordCraft의 egui UI를 바탕으로
    만들었으며, 레이아웃 크레이트는 작은 수정과 함께 `vendor/`에 복사되어 있습니다.
  - **[PdfCraft](https://github.com/storytold/pdfcraft)**: PDF 페이지를 어떤 배율에서도 선명하게
    렌더링하고, 텍스트를 추출하며, 서명된 사본을 저장합니다.
  - **[VectorCraft](https://github.com/storytold/vectorcraft)**: SVG 이미지를 잉크와 함께 열고
    저장하며, 보드와 보관함 내보내기(PDF, SVG, PNG)를 그리고 기록합니다.
  - **[PhotoCraft](https://github.com/storytold/photocraft)**: 색상 프로필과 크기를 유지하면서
    사진(PNG, JPEG, WebP, TIFF…)을 읽고 쓰며, 그 위에 잉크를 그립니다.
  - **각 앱의 MCP 서버**(`wordcraft-mcp`, `pdfcraft-automation`, `vectorcraft-mcp`,
    `photocraft-automation`): 앱 안에서 실행되어 AI 채팅이 옆에 있는 Word, PDF, SVG 문서나 사진을
    읽고 편집할 수 있게 합니다.
  - [craft-libs](https://github.com/storytold/craft-libs)의 패치된 라이브러리.
  - [craft-launcher](https://github.com/storytold/craft-launcher): 앱 내 업데이트와 macOS 서명의
    설계(아이디어만 참고, 복사한 코드 없음).
- **[Typst](https://github.com/typst/typst)** (The Typst Project Developers, Apache-2.0): LaTeX
  수식과 채팅의 차트를 그립니다. 크레이트 두 개가 한 줄 수정과 함께 `vendor/`에 복사되어 있습니다.
  **[MiTeX](https://github.com/mitex-rs/mitex)** (Apache-2.0)는 LaTeX를 Typst로 변환합니다.
- **[Lilaq](https://github.com/lilaq-project/lilaq)** (Mc-Zen, MIT)와 Lilaq가 사용하는 Typst 패키지
  — [zero](https://github.com/Mc-Zen/zero), [tiptoe](https://github.com/Mc-Zen/tiptoe),
  [elembic](https://github.com/PgBiel/elembic), [suiji](https://github.com/liuguangxi/suiji):
  AI 채팅이 그리는 차트.
- **[egui / eframe](https://github.com/emilk/egui)**, Emil Ernerfeldt 및 기여자들: GUI 툴킷.
- **[wry](https://github.com/tauri-apps/wry), [tao](https://github.com/tauri-apps/tao),
  [muda](https://github.com/tauri-apps/muda)**, Tauri 팀: 내장 웹 뷰와 메뉴.
- **[btleplug](https://github.com/deviceplug/btleplug)**, Nonpolynomial Labs: 크로스 플랫폼
  Bluetooth LE.
- **[tokio](https://tokio.rs)**, **[clap](https://github.com/clap-rs/clap)**,
  **[serde](https://serde.rs)**, **[tiny-skia](https://github.com/linebender/tiny-skia)**,
  **[lopdf](https://github.com/J-F-Liu/lopdf)**, **[hayro](https://github.com/LaurenzV/hayro)**,
  **[ureq](https://github.com/algesten/ureq)**, **[arboard](https://github.com/1Password/arboard)**,
  **[rfd](https://github.com/PolyMeilex/rfd)** 및 [THIRD-PARTY.md](THIRD-PARTY.md)에 나열된
  다른 모든 크레이트.
- **[Inter](https://github.com/rsms/inter)** (Rasmus Andersson),
  **[Source Serif 4](https://github.com/adobe-fonts/source-serif)** (Adobe) 및
  **[Noto Sans CJK](https://github.com/notofonts/noto-cjk)**의 작은 일부 (Adobe, 언어 이름 표시용):
  글꼴, SIL OFL 1.1.
- **[Phosphor Icons](https://phosphoricons.com)** (Helena Zhang, Tobias Fried),
  [egui-phosphor](https://github.com/amPerl/egui-phosphor)를 통해 사용: 아이콘, MIT.
- **[KaTeX](https://katex.org)** (Khan Academy) 및 **[marked](https://marked.js.org)**:
  채팅과 노트의 수식 및 Markdown, MIT.
  **[DOMPurify](https://github.com/cure53/DOMPurify)** (Cure53, Apache-2.0 OR MPL-2.0)는 그 HTML을
  표시하기 전에 정리합니다.
- **[Hershey 글꼴](https://en.wikipedia.org/wiki/Hershey_fonts)** (Dr. A. V. Hershey, U.S.
  National Bureau of Standards; `.jhf` 형식은 James Hurt, Cognition Inc.; 파일은 Kamal Mostafa의
  [hershey-fonts](https://github.com/kamalmostafa/hershey-fonts)에서 가져옴): 채팅이 보드에 쓸 때
  사용하는 단선 글자(`src/handwrite/`, 출처 표시 조건의 자유 라이선스)이며, 손글씨 노트의
  **OpenSlate Corsivo** 및 **OpenSlate Stampatello** 글꼴(`assets/fonts/OpenSlate*.ttf`)로도
  만들어졌습니다.
- **[DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness)**: 채팅의 영감
  (아이디어만 참고, 복사한 코드 없음).
- Anthropic의 **[Claude](https://claude.ai)**(AI 페어 프로그래머)와 함께 작성했습니다.

라이선스와 저작자가 포함된 전체 크레이트 목록은 [THIRD-PARTY.md](THIRD-PARTY.md)에 있습니다.
파일별 저작자 표시는 [NOTICE](NOTICE)에 있습니다.

## 라이선스

Copyright © 2026 Marco Paolillo.

OpenSlate는 자유 소프트웨어입니다. **GNU General Public License 버전 3 또는 (선택에 따라) 그 이후의
모든 버전**의 조건에 따라 재배포하거나 수정할 수 있습니다. [LICENSE](LICENSE)를 참고하세요.
Bluetooth 프로토콜 코드가 Tuhi에서 파생되었기 때문에 GPL이 필요합니다.
