<p align="center">
  <img src="assets/icon/openslate-256.png" width="128" height="128" alt="Іконка OpenSlate">
</p>

<h1 align="center">OpenSlate</h1>

<p align="center">
  Безкоштовний застосунок-компаньйон із відкритим кодом для <b>Wacom Bamboo Slate</b> (сімейство SmartPad).<br>
  Дошка наживо · офлайн-синхронізація · підпис документів Word і PDF пером · рукопис у текст · ШІ-асистент, який виправляє ваші вправи просто на сторінці
</p>

<p align="center">
  <a href="https://github.com/rmpaolillo/OpenSlate/releases/latest"><img alt="Останній випуск" src="https://img.shields.io/github/v/release/rmpaolillo/OpenSlate?sort=semver"></a>
  <a href="https://github.com/rmpaolillo/OpenSlate/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/rmpaolillo/OpenSlate/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="Ліцензія: GPL-3.0-or-later" src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue"></a>
</p>

*[English](README.md) · [Italiano](README.it.md) · [Español](README.es.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · Українська · [中文](README.zh.md) · [日本語](README.ja.md) · [한국어](README.ko.md)*

<p align="center">
  <img src="assets/screenshots/chat.png" alt="OpenSlate: ШІ-асистент перевіряє рукописну вправу з математики на дошці й виправляє її червоним чорнилом" width="900">
</p>

---

## Навіщо це все

У мене є Bamboo Slate, і я користуюся ним щодня: щоб писати, розв'язувати вправи й підписувати
документи. Wacom [оголосила про припинення підтримки](https://support.wacom.com/hc/en-us/articles/36070902398231-Bamboo-Paper-Inkspace-Wacom-Notes-application-End-of-Life-Notification)
Inkspace — застосунку, від якого залежить планшет, тож пристрій ось-ось мав перетворитися на прес-пап'є.

Тому я написав власний застосунок для власних потреб. **OpenSlate — це vibe-coded проєкт**: я описував,
чого хочу, і писав його разом з ШІ-асистентом для програмування ([Claude](https://claude.ai)),
вдосконалюючи його в реальному використанні, а не за великим заздалегідь продуманим планом. У мене на
macOS він працює добре. Я ділюся ним на випадок, якщо він стане в пригоді іншим власникам Slate. Будьте готові до
шорсткостей і коментарів у коді італійською (сам інтерфейс перекладено дев'ятьма мовами).
Issues і pull requests вітаються: див. [CONTRIBUTING.md](CONTRIBUTING.md).

> OpenSlate — незалежний проєкт. Він **не пов'язаний із Wacom, не схвалений і не підтримується
> Wacom**. «Wacom», «Bamboo» та «Inkspace» є торговельними марками відповідних власників.

## Коротко

- **Пишіть на папері — бачте на екрані.** Bamboo Slate передає кожен штрих через Bluetooth;
  сторінки, написані далеко від комп'ютера, згодом завантажуються в архів.
- **ШІ-репетитор, який пише на вашій сторінці.** Попросіть його перевірити вправу: він позначає галочкою правильне,
  обводить помилку й поруч від руки пише виправлення, а потім пояснює його в
  чаті, з формулами. Він також може намалювати на сторінці графік функції.
- **Підписуйте документи Word і PDF пером** і зберігайте їх у тому самому форматі.
- **Рукопис → редаговані нотатки** з таблицями й формулами, експорт у Markdown, Word або PDF,
  друкованим текстом або рукописним шрифтом.
- **Експорт у те, що потрібно**: PDF із пошуком, Word із письмом як чорнилом, яке можна редагувати, легкі JPEG/WebP, Xournal++, анімований GIF, SVG або відео MP4 сторінки, що пишеться, а також LaTeX чи Typst із розпізнаного тексту.
- **ШІ на ваш вибір**: локальні моделі (LM Studio, Ollama) або хмарні провайдери; API-ключі ніколи не зберігаються
  відкритим текстом.
- **Говоріть із чатом.** Диктуйте в мікрофон: Whisper розпізнає мову на вашому комп'ютері, без мережі.
- **Безкоштовний і з відкритим кодом**, дев'ятьма мовами, для macOS (основна платформа), Linux і Windows.

## Можливості

- **Дошка наживо.** Під'єднайтеся через Bluetooth і бачте те, що пишете на планшеті, просто під час письма,
  на весь екран. До ваших послуг пера, маркер (`H`; перемикання довгим натисканням пера можна ввімкнути в «Параметрах»), гумка (режим
  штрихування або стирання), кольори, товщина ліній, скасування/повтор, поворот/віддзеркалення та масштабування
  жестами трекпада.
- **Види паперу.** На вибір: у лінійку, італійські шкільні розлінування для початкових класів, у клітинку (4/5/10 мм),
  міліметрівка, декартова площина, у крапку, ізометрична сітка, нотний стан і нотатки Корнелла. Розлінування
  малюються в натуральну величину й видимі лише на екрані, тож експортовані файли залишаються чистими.
- **Офлайн-сторінки.** Завантажуйте сторінки, написані далеко від комп'ютера, в архів,
  згрупований за днями. Звідти їх можна перейменовувати, експортувати, розпізнавати (OCR) або відновлювати з кошика.
- **Підпис документів Word і PDF.** Відкрийте `.docx`/`.doc`/`.odt`/`.rtf` або `.pdf`, пишіть чи
  підписуйте пером і збережіть копію **в тому самому форматі**. У файлі Word
  рукопис стає чорнилом усередині документа, тож його показують і Word, і Pages, і LibreOffice. У
  PDF він стає векторними штрихами поверх незмінного оригіналу. Сторінки залишаються чіткими за будь-якого масштабу:
  їх рендерять рушії [WordCraft](https://github.com/storytold/wordcraft) і
  [PdfCraft](https://github.com/storytold/pdfcraft).
- **Пишіть і на зображеннях.** Відкрийте SVG, PNG, JPEG (а також WebP, TIFF, BMP, GIF) або натисніть **Вставити**
  (`Cmd+V`), коли в буфері обміну є знімок екрана чи фото. Пишіть на ньому й збережіть назад як зображення
  того самого формату й розміру: SVG зберігає рукопис як векторні контури
  ([VectorCraft](https://github.com/storytold/vectorcraft)), а на фото його наносять у повній
  роздільності зі збереженням колірного профілю ([PhotoCraft](https://github.com/storytold/photocraft)).
- **Копіювання й вставлення, як у Word.** Вкладка «Головна» починається з кнопок **Вставити** і **Копіювати**: «Копіювати» кладе
  сторінку в буфер обміну як зображення, готове для ШІ-чату, Word чи електронного листа.
- **Рукопис → текст.** ШІ-модель із підтримкою зору дослівно транскрибує сторінку, разом із
  помилками, у нотатку Markdown: заголовки, списки, прапорці, таблиці та формули в LaTeX, якщо
  вони є. Малюнки, графіки й діаграми перемальовуються з ваших справжніх штрихів пера як чіткі
  SVG-ілюстрації поруч із нотаткою. Нотатка відкривається в застосунку як редагована сторінка в стилі Typora,
  а **Експорт** зберігає її як Markdown (з ілюстраціями поруч), Word або PDF. **Письмо** показує
  нотатки друкованими або написаними від руки, прописними чи друкованими літерами — тими самими, якими ШІ-чат пише
  на дошці; рукопис потрапляє у файл Word (вбудований, тож його видно на будь-якому комп'ютері)
  і в PDF теж.
- **ШІ-асистент для будь-чого.** Чат розташований поруч зі сторінкою і бачить дошку,
  відкритий документ або нотатку. Використовуйте його, щоб підсумувати, охайно переписати, виокремити завдання, скласти чернетки текстів,
  перевірити написане на помилки, відповісти на запитання, написати в нотатку, пошукати в інтернеті та
  перевірити номери ПДВ у ЄС. Вставляйте в нього зображення (`Cmd+V`, перетягуванням або через скріпку) і робіть
  його текст більшим чи меншим щипком або `Cmd +` / `Cmd −`. Якщо вибрана модель не бачить зображень (суто текстова модель), сторінку спершу
  читає агент, вибраний у *Читання сторінок (OCR)*, і передає її як текст. За ним стоїть резервний ланцюжок OpenAI-сумісних провайдерів:
  LM Studio, Ollama, DeepSeek, Google Gemini та Anthropic Claude. Він показує використання контексту й
  автоматично його стискає. Перетягніть вкладку документа на чат, щоб прикріпити його.
- **Графіки та правки у ваших документах.** Попросіть чат побудувати графік («побудуй графіки двох рівнянь і
  розв'язок»), і він намалює його в стилі LaTeX за допомогою [Typst](https://typst.app) і
  [Lilaq](https://lilaq.org): функції, точки, лінії, стовпчики, параметричні криві. Якщо до чату
  прикріплено документ, він також може його прочитати й **відредагувати** та вставити в нього графік через
  MCP-сервери пакета Craft, що працюють усередині застосунку: WordCraft для Word, PdfCraft для PDF,
  VectorCraft для SVG, PhotoCraft для фото (кадрування, зміна розміру, поворот, яскравість, рівні, криві,
  чорно-біле, різкість… зображення зберігає свій формат) плюс текстові інструменти для Markdown. Кожна зміна одразу видна в документі
  поруч із чатом, і її можна скасувати з чату одним клацанням; оригінальний файл на диску
  ніколи не змінюється. У нотатці-транскрипції графік може замінити намальовану від руки ілюстрацію; на аркуші дошки він замінює намальований від руки графік (знаходить його автоматично: осі й підписи) або розміщується під написаним. SVG, який модель пише у відповіді, показується як зображення.
- **Заповнення PDF-форм.** Прикріпіть до чату PDF-форму й попросіть «заповни цю форму моїми
  даними»: він прочитає поля й заповнить їх через PdfCraft. Друкована чи сканована форма без полів
  спершу їх отримує (PdfCraft знаходить порожні місця й називає їх за підписами поруч), або чат
  пише текст, позначки й дату там, де треба. Він використовує лише те, що ви йому сказали, і питає
  про те, чого бракує. Він ніколи не підписує: підпис ставите ви пером, а заповнені значення
  залишаються в підписаній копії.
- **Говоріть із чатом.** Клацніть мікрофон поруч із «Надіслати» або тримайте праву клавішу Alt
  (Option), поки говорите: слова з'являються в полі повідомлення, готові до виправлення й надсилання.
  Whisper розпізнає їх на вашому комп'ютері за допомогою [whisper.cpp](https://github.com/ggml-org/whisper.cpp) (на Mac — на
  графічному процесорі), без мережі: ваш голос не залишає комп'ютер. Модель (547 МБ) завантажується
  один раз, першого разу, якщо ви погодитеся; легшу модель чи онлайн-сервіс (OpenAI, Groq, Mistral…)
  можна вибрати в `ai.toml` (`[voice]`). Esc скасовує. Першого разу macOS просить дозвіл на мікрофон.
- **Чат пише на дошці.** Попросіть його «перевір мої обчислення й познач помилки»,
  «підкресли дедлайни» або «напиши результат поруч із рядком 2»: він читає аркуш дошки рядок
  за рядком і слово за словом, а потім виділяє, підкреслює, обводить, закреслює, малює рамки,
  стрілки, галочки й хрестики та пише від руки (друкованими або прописними літерами, складеними зі штрихів
  однолінійних шрифтів [Hershey](https://en.wikipedia.org/wiki/Hershey_fonts)). На прохання він
  також переміщує, збільшує, перефарбовує або стирає те, що є на аркуші («зроби заголовок більшим і
  синім», «пересунь останній рядок нижче», «прибери мої виділення»); сам по собі він лише додає. Щоб очистити
  більшу частину аркуша, потрібна ваша згода в чаті. Усе, що він робить, — звичайне чорнило (зберігається,
  експортується й стирається так само, як ваше), і одне клацання в чаті знову все це прибирає.
- **Чат знає ваш архів.** «Знайди сторінку, де я писав про жовтневий бюджет»,
  «відкрий учорашні нотатки на дошці», «збережи дошку як „Нарада“ і почни нову сторінку»,
  «експортуй сторінки цього тижня одним PDF»: він шукає за назвами, датами й текстом
  транскрипцій, переглядає сторінку, відкриває її на дошці, зберігає дошку (оновлюючи сторінку, з якої вона
  походить), перейменовує, експортує у вашу звичну папку експорту (ніколи не перезаписуючи) і переміщує сторінки в
  кошик і назад. Переміщення в кошик завжди чекає на вашу згоду в чаті.
- **Ваші власні MCP-сервери.** Додайте будь-який MCP-сервер (stdio) в `ai.toml`, як у Claude Desktop або DSH:
  його інструменти стануть доступні чату.
- **Наживо в мережі.** Посилання для спільного доступу дає змогу телевізору, телефону чи ПК у вашій домашній мережі стежити за дошкою
  в реальному часі. Сталі адреси: `/page.pdf`, `/page.svg`, `/page.png` і `/state.json`.
  Дошка наживо відкрита для домашньої мережі; для чату, нотаток і розмов потрібен ключ,
  який є частиною посилання, що показує OpenSlate (`?k=…`, далі браузер його запам'ятовує). Сервер
  відповідає лише на IP-адреси, `localhost` та імена `.local` і відхиляє записи, що надходять з
  інших вебсайтів, тож сторінка, яку ви відвідуєте, не може керувати застосунком.
- **Нічого не губиться.** Якщо ви виходите, коли на дошці є щось, чого немає в архіві, застосунок питає
  *Зберегти й вийти / Вийти без збереження / Не виходити*; коли застосунок закривається без запитання (Dock,
  вимкнення, оновлення), дошка сама потрапляє в архів, а якщо інтерфейс колись аварійно завершиться,
  дошку теж буде збережено (причина записується в `crash.log` у папці даних). Нотатки зберігаються
  з перевіркою версії, тож правку від ШІ чи іншого застосунку ніколи не буде мовчки перезаписано.
  Сторінки, уже видалені з планшета, відновлюються з їхніх необроблених резервних копій, якщо завантаження
  перервалося.
- **Лише аркуш.** Кнопка поруч із «Копіювати»/«Вставити» в рядку заголовка (або `Cmd+Shift+F`, «Перегляд» → «Лише
  аркуш») ховає рядок меню та стрічку; клацніть ще раз, щоб повернути їх.
- **Інші формати.** Розпізнана сторінка стає **PDF із пошуком**: текст лежить невидимо поверх рукописного, тож Spotlight і будь-яка програма для PDF знаходять його, а ви можете його скопіювати. В «Інші формати»: JPEG і WebP (легкі, для надсилання), **Word** із письмом як справжнім чорнилом Word, яке можна редагувати (і розпізнаним текстом на наступній сторінці), **Xournal++** (`.xopp`), **анімований GIF або SVG** чи **відео MP4**, що переписує сторінку штрих за штрихом (відео використовує системний кодувальник: AVFoundation на macOS, Media Foundation у Windows; у Linux ffmpeg або OpenH264 від Cisco, завантажений під час першого використання, якщо ви погодитеся), а також **LaTeX** або **Typst** із розпізнаного тексту (нотатки теж експортуються в них). Чат ШІ може експортувати в усі ці формати.
- **Збереження й експорт.** Експорт у векторні PDF і SVG у справжньому розмірі A4 або в PNG. Кожне збереження чи
  експорт відкриває системне вікно «Зберегти як», тож ви завжди бачите, куди потрапить файл. Воно відкривається в
  останній вибраній вами папці (уперше — в `exports/`). Також можна скопіювати сторінку як зображення
  (`Cmd+C`) і вставити її будь-куди.
- **Ваша мова.** Увесь інтерфейс (меню, стрічка, параметри, чат, нотатки й налаштування ШІ)
  доступний англійською, італійською, іспанською, французькою, німецькою, українською, китайською, японською та
  корейською. Він відповідає мові системи; змінити її можна у «Файл» → «Параметри» → «Вигляд» → «Мова».
  ШІ відповідає мовою, якою ви пишете.

## Знімки екрана

На знімках показано англійський інтерфейс (вбудовано ще вісім мов). Рукопис,
документи й відповіді ШІ — це зразки, підготовлені для цих ілюстрацій; позначки на
сторінці та графік намальовано власними інструментами дошки застосунку.

| | |
|---|---|
| **Сторінка пишеться сама**: експорт в анімований GIF, штрих за штрихом | **Інші формати**: JPEG, WebP, Word із чорнилом, Xournal++, анімовані GIF/SVG, відео MP4, LaTeX, Typst |
| <img src="assets/screenshots/writing.gif" alt="Анімований GIF: сторінка пишеться штрих за штрихом" width="440"> | <img src="assets/screenshots/formats.png" alt="Меню «Інші формати»" width="440"> |
| **Графік на сторінці**: чат малює функцію й розміщує її під вправою | **Підпишіть документ Word або PDF** пером і збережіть його в тому самому форматі |
| <img src="assets/screenshots/graph.png" alt="ШІ-чат малює графік функції й розміщує його на рукописній сторінці" width="440"> | <img src="assets/screenshots/sign.png" alt="Документ Word, підписаний пером" width="440"> |
| **Рукопис → текст**: сторінка, транскрибована в редаговану нотатку, разом із формулами | **Нотатки від руки**: та сама нотатка рукописним прописним шрифтом |
| <img src="assets/screenshots/transcribe.png" alt="Рукописна сторінка, транскрибована в нотатку з таблицею та формулами" width="440"> | <img src="assets/screenshots/notes.png" alt="Нотатка, показана рукописним прописним шрифтом OpenSlate" width="440"> |
| **Архів** сторінок, завантажених із планшета, згрупованих за днями | **Темна тема** й види паперу (тут клітинка 5 мм) |
| <img src="assets/screenshots/library.png" alt="Архів рукописних сторінок" width="440"> | <img src="assets/screenshots/dark.png" alt="Темна тема з папером у клітинку та відкритим ШІ-чатом" width="440"> |

## Встановлення

Готові до встановлення збірки публікуються на сторінці
[**Releases**](https://github.com/rmpaolillo/OpenSlate/releases), кожна зі зростаючим
номером версії (`v0.3.0`, `v0.3.1`, …).

| Платформа | Файл | Стан |
|---|---|---|
| macOS 11+, Apple Silicon (M1, M2…) | `OpenSlate-<version>-macos-arm64.dmg` | основна платформа, щоденне використання |
| macOS 11+, Intel | `OpenSlate-<version>-macos-intel.dmg` | збирається й працює; менше протестовано |
| macOS 11+, обидві (удвічі більший розмір) | `OpenSlate-<version>-macos-universal.dmg` | для оновлення із застосунку з версії 0.2.x |
| Linux x86_64 (Debian/Ubuntu) | `openslate_<version>_amd64.deb` | збирається й працює; менше протестовано |
| Linux x86_64 (інші дистрибутиви) | `openslate-<version>-linux-x86_64.tar.gz` | збирається й працює; менше протестовано |
| Windows x86_64 | `openslate-<version>-windows-x86_64.zip` | **експериментальна** |

### macOS

1. Відкрийте `.dmg` і перетягніть **OpenSlate** у **Програми**.
2. Випуски, зібрані з Apple Developer ID, підписані й нотаризовані: вони відкриваються без
   попереджень. Збірка без нього (див. «Підписування й нотаризація» нижче) підписана ad hoc, тож
   першого разу macOS повідомляє, що її «неможливо відкрити»: відкрийте **Системні параметри → Приватність і безпека**
   і клацніть **Усе одно відкрити** або виконайте
   ```bash
   xattr -dr com.apple.quarantine /Applications/OpenSlate.app
   ```
   Це потрібно лише під час першого встановлення: оновлення завантажує сам застосунок, і вони відкриваються одразу.
3. Дозвольте доступ до **Bluetooth**, коли з'явиться запит.

Більше нічого встановлювати не треба: кожен файл відкривається й зберігається бібліотеками
[пакета Craft](https://github.com/storytold) (див. «Файли й формати» нижче).

### Linux

```bash
sudo apt install ./openslate_<version>_amd64.deb     # Debian / Ubuntu
# or unpack the tar.gz and run ./openslate
```

Для роботи потрібні BlueZ, GTK 3 і WebKitGTK 4.1. Вашому користувачеві має бути дозволено використовувати Bluetooth.
Для експорту відео MP4 використовується `ffmpeg`, якщо він встановлений (`sudo apt install ffmpeg`); інакше
першого разу застосунок запитає, перш ніж завантажити з сайту Cisco безкоштовний кодувальник OpenH264 (близько 1 МБ).

### Windows (експериментально)

Розпакуйте архів і запустіть `openslate.exe`. Потрібне середовище виконання Microsoft Edge WebView2, яке постачається з
Windows 10/11. У виданнях Windows «N» для відео MP4 потрібен ще Media Feature Pack від Microsoft.
На Windows протестовано не все. Повідомлення про помилки дуже вітаються.

## Файли й формати

Кожен файл відкривається, показується й зберігається бібліотеками пакета Craft від ArtCraft, без
їхніх інтерфейсів і без зовнішніх програм:

| Файли | Бібліотека |
|---|---|
| Word (.docx, .doc, .odt, .rtf), Markdown, .txt, .html | [WordCraft](https://github.com/storytold/wordcraft): відкриває й верстає сторінки, зберігає Word із чорнилом |
| PDF | [PdfCraft](https://github.com/storytold/pdfcraft): сторінки, мініатюри й текст; його PDF-бібліотека зберігає підписану копію |
| SVG | [VectorCraft](https://github.com/storytold/vectorcraft) |
| Фото (PNG, JPEG, WebP, TIFF, BMP, GIF) | [PhotoCraft](https://github.com/storytold/photocraft) |
| Експорт дошки й архіву (PDF, SVG, PNG, JPEG, WebP) | VectorCraft малює й записує їх; файли PNG, JPEG і WebP записує PhotoCraft |
| Анімовані GIF і SVG, Xournal++, LaTeX, Typst, прихований текст PDF із пошуком | записує сам OpenSlate (`src/formats.rs`, `src/mdconv.rs`) |
| Відео MP4 | системний кодувальник: AVFoundation (macOS), Media Foundation (Windows); у Linux ffmpeg або OpenH264 від Cisco, завантажений під час першого використання (*OpenH264 Video Codec provided by Cisco Systems, Inc.*) |

Файл Markdown, відкритий як документ, має такий самий вигляд, як сторінка «Нотатки»: текст Source Serif, заголовки
Inter, легкі таблиці. Його формули (`$…$`, `$$…$$`, `\(…\)`, `\[…\]`) малюються в стилі LaTeX за допомогою
[Typst](https://typst.app) (LaTeX перетворює [MiTeX](https://github.com/mitex-rs/mitex)),
а ілюстрації поруч (`<img src="…">`, `![…](…)`, SVG, намальовані VectorCraft) стають зображеннями.
Підписаний файл Markdown, .txt або .html зберігається як PDF (ці формати не можуть містити чорнило).

## Оновлення

OpenSlate оновлюється сам, як і застосунки Craft, після вашого підтвердження. Коли на GitHub
публікується новий випуск, синя смуга під стрічкою повідомляє про це й пропонує **Оновити**; далі застосунок
завантажує його, перевіряє за `SHA256SUMS.txt` випуску та підписом розробника,
встановлює замість запущеного застосунку й пропонує **Перезапустити** (або просто користуйтеся новою версією
під час наступного запуску). У «Параметри» → «Додатково» можна налаштувати встановлення без запитання або вимкнути
саму перевірку; «Файл» → «Довідка та інформація» → «Перевірити оновлення» перевіряє одразу.

Ваші налаштування, ШІ-агенти, архів, документи й експортовані файли зберігаються в папці даних (macOS:
`~/Library/Application Support/OpenSlate`), поза застосунком, а API-ключі — у
Зв'язці ключів: оновлення їх ніколи не зачіпає. На macOS оновлення замінює `OpenSlate.app`
(його завантажує сам застосунок, тож Gatekeeper не питає знову); з `.deb` воно запитує
ваш пароль через `pkexec`; на Windows воно встановлюється, коли ви виходите із застосунку.

## Перше під'єднання

- **Уперше** або після використання Inkspace: утримуйте кнопку планшета приблизно 6 с, доки світлодіод
  не заблимає синім. Потім клацніть **Під'єднати** й натисніть кнопку один раз, коли з'явиться запит.
- **Надалі** (світлодіод зелений): коротко натисніть кнопку, потім клацніть **Під'єднати**.

Планшет пам'ятає лише один застосунок водночас. Якщо ви повернетеся до Inkspace, доведеться знову зареєструвати
OpenSlate.

## Де зберігаються ваші дані

Після встановлення OpenSlate зберігає ваш архів, документи, чати й налаштування в:

- macOS: `~/Library/Application Support/OpenSlate`
- Linux: `~/.local/share/openslate` (або `$XDG_DATA_HOME/openslate`)
- Windows: `%APPDATA%\OpenSlate`

Це можна змінити змінною середовища `OPENSLATE_DATA`. Якщо запустити його через
`cargo run` з папки з вихідним кодом, він натомість працює в цій папці. Там же є
`crash.log` (якщо застосунок колись аварійно завершувався), `web.key` (ключ вебпосилання наживо) і `updates/`
(попередня версія застосунку, збережена після оновлення). На Linux у `codecs/` лежить кодувальник
OpenH264 від Cisco, якщо ви дозволили завантаження.
У `models/whisper/` — модель Whisper для голосу в чаті, коли її завантажено.

## ШІ-провайдери та API-ключі

Функції ШІ необов'язкові. Провайдери налаштовуються в **Файл → Параметри → ШІ** або у файлі `ai.toml`,
який створюється з коментарями під час першого використання. Працює будь-який сервер, що підтримує API OpenAI
`/chat/completions`, зокрема локальні (LM Studio, Ollama), яким ключ не потрібен.

**Читання сторінок (OCR)** має окреме налаштування (`[ocr]` в `ai.toml` або *Налаштування ШІ → Читання
сторінок (OCR)*): виберіть агента, який читає рукопис для *Розпізнати* і для суто текстових моделей чату, або
залиште поле порожнім, щоб використовувати першого агента в списку, який бачить зображення. Рекомендовано швидкий, як-от Gemini;
`fallback = false` означає «завжди й лише цей».

**API-ключі ніколи не зберігаються відкритим текстом.** `ai.toml` лише вказує, *де* знайти кожен ключ:

```toml
key = "env:DEEPSEEK_API_KEY"              # environment variable
key = "keychain:DEEPSEEK_API_KEY"         # macOS Keychain
key = "env:GEMINI_API_KEY | keychain:GEMINI_API_KEY"   # first one found wins
```

```bash
# macOS Keychain (recommended: also works when the app is opened from Finder)
security add-generic-password -a "$USER" -s DEEPSEEK_API_KEY -w 'sk-...'
```

### MCP-сервери

Чат може використовувати інструменти зовнішніх серверів [MCP](https://modelcontextprotocol.io) (транспорт
stdio), налаштованих в `ai.toml`, як у Claude Desktop:

```toml
[[mcp]]
name = "filesystem"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/Users/me/Documents"]
env = { TOKEN = "keychain:MY_TOKEN" }   # values resolved like API keys
# enabled = false   to turn it off; timeout = 120 (seconds per tool)
```

OpenSlate запускає програму, коли вона потрібна чату, і тримає її запущеною; її інструменти доходять до
моделі як `name__tool`. Власні MCP-сервери пакета Craft (WordCraft, PdfCraft, VectorCraft) не потребують
налаштування: вони працюють усередині застосунку з документом, прикріпленим до чату.

## Збирання з вихідного коду

Потрібен **Rust 1.95 або новіший** (`rustup update`).

```bash
git clone https://github.com/rmpaolillo/OpenSlate.git
cd OpenSlate
cargo run --release                  # GUI
cargo run --release -- gui --demo    # try it without a pad
cargo run --release -- --help        # command-line tools (scan, sync, live, pdf, ai, …)
cargo test
```

Залежності для збирання на Linux (Debian/Ubuntu):

```bash
sudo apt install pkg-config libgtk-3-dev libwebkit2gtk-4.1-dev libsoup-3.0-dev \
                 libjavascriptcoregtk-4.1-dev libdbus-1-dev libxkbcommon-dev libasound2-dev
```

На macOS потрібні інструменти командного рядка Xcode (`xcode-select --install`). Невеликі допоміжні модулі на Swift
(необов'язковий OCR через Apple Vision і відео MP4) компілюються під час першого використання або входять до складу `.app` випуску.

## Структура проєкту

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

Опис кожного файлу є в [італійському README](README.it.md#struttura).

## Випуски та версіонування

- `ci.yml` збирає й тестує на Linux і macOS під час кожного push і pull request.
- `release.yml` запускається, коли його запускає мейнтейнер (Actions → Release → Run workflow, на `main`).
  Він збирає macOS (Apple Silicon, Intel та універсальну), Linux і Windows, публікує GitHub Release з
  автоматично згенерованими нотатками й залишає лише два найновіші випуски. Бінарний файл випуску
  очищено від налагоджувальних символів і оптимізовано між crates (`lto = "thin"`, `codegen-units = 1`); рідко використовувані
  бібліотеки, які підтягують інші, оптимізовано за розміром, а кожен шрифт вбудовано один раз (див.
  `[profile.release]` у `Cargo.toml` і `src/fonts.rs`).
- Версії відповідають [Semantic Versioning](https://semver.org): `MAJOR.MINOR` береться з `Cargo.toml`,
  `PATCH` обчислюється автоматично з наявних тегів (`v0.3.0`, потім `v0.3.1`, …).
- Щоб почати нову серію (нові функції: `0.3.0`; перша стабільна: `1.0.0`), змініть `version` у
  `Cargo.toml`; наступний випуск знову почнеться з патча `0`.
- Випуск спершу запускає тести й збирається із зафіксованими залежностями (`Cargo.lock`). Кожен
  пакет містить `LICENSE`, `NOTICE`, `THIRD-PARTY.md` і `THIRD-PARTY-LICENSES.txt` (повний
  текст кожної сторонньої ліцензії, створений `packaging/collect_licenses.py`).
- Кожен випуск має `SHA256SUMS.txt`: застосунок перевіряє за ним завантажений файл перед
  встановленням оновлення.

### Підписування й нотаризація (macOS)

Щоб macOS довіряла застосунку без попереджень, як і застосункам Craft, випуску потрібен сертифікат Apple
**Developer ID Application** (Apple Developer Program, щорічна оплата). Додайте такі
секрети репозиторію (Settings → Secrets and variables → Actions):

| Секрет | Значення |
|---|---|
| `APPLE_CERTIFICATE` | сертифікат «Developer ID Application», експортований як `.p12`, у base64 (`base64 -i cert.p12 \| pbcopy`) |
| `APPLE_CERTIFICATE_PASSWORD` | пароль, вибраний під час експорту `.p12` |
| `APPLE_ID` | e-mail Apple ID облікового запису розробника |
| `APPLE_PASSWORD` | пароль для застосунку для цього Apple ID (appleid.apple.com → Sign-In and Security) |
| `APPLE_TEAM_ID` | 10-символьний Team ID (developer.apple.com → Membership) |

З ними `release.yml` підписує застосунок із hardened runtime і позначкою часу, надсилає його до
служби нотаризації Apple, прикріплює квиток (staple) і робить те саме для DMG. Без них застосунок
підписується ad hoc, і випуск усе одно працює (перше відкриття через **Усе одно відкрити**).

## Подяки

OpenSlate не існував би без цих проєктів і їхніх авторів:

- **[Tuhi](https://github.com/tuhiproject/tuhi)** від учасників проєкту Tuhi (GPL-2.0-or-later).
  Bluetooth-протокол сімейства SmartPad перекладено з результатів їхнього зворотного
  інжинірингу.
- **Пакет Craft** від ArtCraft Team і учасників (MIT OR Apache-2.0), використаний як
  бібліотеки без їхніх інтерфейсів:
  - **[WordCraft](https://github.com/storytold/wordcraft)**: відкриває, верстає й зберігає документи Word
    (і Markdown) із чорнилом; інтерфейс у стилі Word адаптовано з його egui UI;
    його crate для верстки скопійовано у `vendor/` з невеликою зміною;
  - **[PdfCraft](https://github.com/storytold/pdfcraft)**: рендерить сторінки PDF чіткими за будь-якого масштабу,
    витягує їхній текст і зберігає підписану копію;
  - **[VectorCraft](https://github.com/storytold/vectorcraft)**: відкриває й зберігає SVG-зображення з
    чорнилом, а також малює й записує експорт дошки та архіву (PDF, SVG, PNG);
  - **[PhotoCraft](https://github.com/storytold/photocraft)**: читає й записує фото (PNG, JPEG,
    WebP, TIFF…), зберігаючи колірний профіль і розмір, і наносить на них чорнило;
  - **їхні MCP-сервери** (`wordcraft-mcp`, `pdfcraft-automation`, `vectorcraft-mcp`,
    `photocraft-automation`), що працюють усередині застосунку, щоб ШІ-чат міг читати й редагувати документ Word, PDF,
    SVG або фото поруч із ним;
  - пропатчені бібліотеки з [craft-libs](https://github.com/storytold/craft-libs);
  - [craft-launcher](https://github.com/storytold/craft-launcher): підхід до оновлень із
    застосунку та до підписування на macOS (лише ідеї, код не скопійовано).
- **[Typst](https://github.com/typst/typst)** (The Typst Project Developers, Apache-2.0): малює
  формули LaTeX і графіки чату; два його crates скопійовано у `vendor/` зі зміною в один
  рядок. **[MiTeX](https://github.com/mitex-rs/mitex)** (Apache-2.0) перетворює LaTeX на Typst.
- **[Lilaq](https://github.com/lilaq-project/lilaq)** (Mc-Zen, MIT) і пакети Typst, які він
  використовує, — [zero](https://github.com/Mc-Zen/zero), [tiptoe](https://github.com/Mc-Zen/tiptoe),
  [elembic](https://github.com/PgBiel/elembic), [suiji](https://github.com/liuguangxi/suiji):
  графіки, які малює ШІ-чат.
- **[egui / eframe](https://github.com/emilk/egui)** від Emil Ernerfeldt і учасників:
  інструментарій GUI.
- **[wry](https://github.com/tauri-apps/wry), [tao](https://github.com/tauri-apps/tao),
  [muda](https://github.com/tauri-apps/muda)** від команди Tauri: вбудовані веб-подання та меню.
- **[btleplug](https://github.com/deviceplug/btleplug)** від Nonpolynomial Labs: кросплатформний
  Bluetooth LE.
- **[tokio](https://tokio.rs)**, **[clap](https://github.com/clap-rs/clap)**,
  **[serde](https://serde.rs)**, **[tiny-skia](https://github.com/linebender/tiny-skia)**,
  **[lopdf](https://github.com/J-F-Liu/lopdf)**, **[hayro](https://github.com/LaurenzV/hayro)**,
  **[ureq](https://github.com/algesten/ureq)**, **[arboard](https://github.com/1Password/arboard)**,
  **[rfd](https://github.com/PolyMeilex/rfd)** і всі інші crates, перелічені в
  [THIRD-PARTY.md](THIRD-PARTY.md).
- **[Inter](https://github.com/rsms/inter)** (Rasmus Andersson),
  **[Source Serif 4](https://github.com/adobe-fonts/source-serif)** (Adobe) і невелика підмножина
  **[Noto Sans CJK](https://github.com/notofonts/noto-cjk)** (Adobe, для назв мов):
  шрифти, SIL OFL 1.1.
- **[Phosphor Icons](https://phosphoricons.com)** (Helena Zhang, Tobias Fried) через
  [egui-phosphor](https://github.com/amPerl/egui-phosphor): іконки, MIT.
- **[KaTeX](https://katex.org)** (Khan Academy) і **[marked](https://marked.js.org)**:
  формули й Markdown у чаті та нотатках, MIT;
  **[DOMPurify](https://github.com/cure53/DOMPurify)** (Cure53, Apache-2.0 OR MPL-2.0) очищує
  цей HTML перед показом.
- **[Шрифти Hershey](https://en.wikipedia.org/wiki/Hershey_fonts)** (д-р A. V. Hershey, U.S.
  National Bureau of Standards; формат `.jhf` від James Hurt, Cognition Inc.; файли з
  [hershey-fonts](https://github.com/kamalmostafa/hershey-fonts) від Kamal Mostafa):
  однолінійні літери, якими чат пише на дошці (`src/handwrite/`, вільна ліцензія із
  зазначенням авторства), перетворені також на шрифти **OpenSlate Corsivo** і **OpenSlate Stampatello**
  рукописних нотаток (`assets/fonts/OpenSlate*.ttf`).
- **[DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness)**: натхнення для
  чату (лише ідеї, код не скопійовано).
- Написано за допомогою **[Claude](https://claude.ai)** від Anthropic як ШІ-напарника з програмування.

Повний перелік crates з ліцензіями та авторами — у [THIRD-PARTY.md](THIRD-PARTY.md).
Атрибуції для окремих файлів — у [NOTICE](NOTICE).

## Ліцензія

Copyright © 2026 Marco Paolillo.

OpenSlate — вільне програмне забезпечення: ви можете поширювати та/або змінювати його на умовах
**GNU General Public License версії 3 або (на ваш вибір) будь-якої пізнішої версії**. Див.
[LICENSE](LICENSE). GPL обов'язкова, бо код Bluetooth-протоколу походить від Tuhi.
