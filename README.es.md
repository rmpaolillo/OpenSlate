<p align="center">
  <img src="assets/icon/openslate-256.png" width="128" height="128" alt="Icono de OpenSlate">
</p>

<h1 align="center">OpenSlate</h1>

<p align="center">
  Una aplicación complementaria libre y de código abierto para el <b>Wacom Bamboo Slate</b> (familia SmartPad).<br>
  Pizarra en directo · sincronización sin conexión · firma de documentos Word y PDF con el bolígrafo · escritura a mano a texto · un asistente de IA que corrige tus ejercicios directamente en la hoja
</p>

<p align="center">
  <a href="https://github.com/rmpaolillo/OpenSlate/releases/latest"><img alt="Última versión" src="https://img.shields.io/github/v/release/rmpaolillo/OpenSlate?sort=semver"></a>
  <a href="https://github.com/rmpaolillo/OpenSlate/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/rmpaolillo/OpenSlate/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="Licencia: GPL-3.0-or-later" src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue"></a>
</p>

*[English](README.md) · [Italiano](README.it.md) · Español · [Français](README.fr.md) · [Deutsch](README.de.md) · [Українська](README.uk.md) · [中文](README.zh.md) · [日本語](README.ja.md) · [한국어](README.ko.md)*

<p align="center">
  <img src="assets/screenshots/chat.png" alt="OpenSlate: el asistente de IA revisa un ejercicio de matemáticas escrito a mano en la pizarra y lo corrige con tinta roja" width="900">
</p>

---

## Por qué existe

Tengo un Bamboo Slate y lo uso todos los días: para escribir, para resolver ejercicios y para firmar
documentos. Wacom ha [anunciado el fin de vida](https://support.wacom.com/hc/en-us/articles/36070902398231-Bamboo-Paper-Inkspace-Wacom-Notes-application-End-of-Life-Notification)
de Inkspace, la aplicación de la que depende el pad, así que el hardware estaba a punto de convertirse en un pisapapeles.

Así que hice mi propia aplicación, para mis propias necesidades. **OpenSlate es un proyecto *vibe-coded***: describí
lo que quería y lo escribí junto con un asistente de programación con IA ([Claude](https://claude.ai)),
iterando sobre el uso real en lugar de seguir un gran diseño previo. A mí me funciona bien en
macOS. Lo comparto por si resulta útil a otros propietarios de un Slate. Cuenta con algunas asperezas y con comentarios
del código en italiano (la interfaz en sí está traducida a nueve idiomas).
Las incidencias y las pull requests son bienvenidas: consulta [CONTRIBUTING.md](CONTRIBUTING.md).

> OpenSlate es un proyecto independiente. **No está afiliado, respaldado ni soportado por
> Wacom**. «Wacom», «Bamboo» e «Inkspace» son marcas comerciales de sus respectivos propietarios.

## De un vistazo

- **Escribe en papel y míralo en la pantalla.** El Bamboo Slate transmite cada trazo por Bluetooth;
  las páginas escritas lejos del ordenador se descargan más tarde en un archivo.
- **Un tutor de IA que escribe en tu hoja.** Pídele que revise un ejercicio: marca con un tic lo que está bien,
  rodea el error con un círculo y escribe la corrección al lado, a mano, y luego te lo explica en
  el chat, con fórmulas. También puede dibujar la gráfica de una función en la hoja.
- **Firma documentos Word y PDF con el bolígrafo**, guardados en el mismo formato.
- **Escritura a mano → notas editables**, con tablas y fórmulas, exportadas a Markdown, Word o PDF,
  en letra de imprenta o en una fuente manuscrita.
- **La IA que tú elijas**: modelos locales (LM Studio, Ollama) o proveedores en la nube; las claves de API nunca se guardan
  en texto plano.
- **Libre y de código abierto**, en nueve idiomas, para macOS (plataforma principal), Linux y Windows.

## Funciones

- **Pizarra en directo.** Conéctate por Bluetooth y ve lo que escribes en el pad mientras lo escribes,
  a pantalla completa. Tienes bolígrafos, un rotulador fluorescente (`H`; el cambio con una pulsación larga del bolígrafo se puede activar en Opciones), un borrador (modo
  garabato o modo borrador), colores, grosores de trazo, deshacer/rehacer, rotar/reflejar y zoom con
  gestos del trackpad.
- **Tipos de papel.** Elige entre rayado, pautas de la escuela primaria italiana, cuadriculado (4/5/10 mm),
  papel milimetrado, plano cartesiano, punteado, isométrico, pentagrama y notas Cornell. Las pautas
  se dibujan a tamaño real y solo aparecen en pantalla, así que las exportaciones quedan limpias.
- **Páginas sin conexión.** Descarga las páginas que escribiste lejos del ordenador en un archivo
  agrupado por días. Desde ahí puedes renombrar, exportar, aplicar OCR o recuperar páginas de la papelera.
- **Firma documentos Word y PDF.** Abre un `.docx`/`.doc`/`.odt`/`.rtf` o un `.pdf`, escribe o
  firma en él con el bolígrafo y guarda una copia **en el mismo formato**. En un archivo Word la
  escritura a mano se convierte en tinta dentro del documento, de modo que Word, Pages y LibreOffice la muestran. En un
  PDF se convierte en trazos vectoriales sobre el original intacto. Las páginas se ven nítidas con cualquier zoom,
  renderizadas por los motores [WordCraft](https://github.com/storytold/wordcraft) y
  [PdfCraft](https://github.com/storytold/pdfcraft).
- **Escribe también sobre imágenes.** Abre un SVG, PNG o JPEG (también WebP, TIFF, BMP, GIF), o pulsa **Pegar**
  (`Cmd+V`) con una captura de pantalla o una foto en el portapapeles. Escribe encima y vuelve a guardarla como imagen
  del mismo formato y tamaño: un SVG conserva la escritura como trazados vectoriales
  ([VectorCraft](https://github.com/storytold/vectorcraft)); en una foto se pinta a resolución
  completa conservando su perfil de color ([PhotoCraft](https://github.com/storytold/photocraft)).
- **Copiar y Pegar como en Word.** La pestaña Inicio empieza con **Pegar** y **Copiar**: Copiar pone la
  página en el portapapeles como imagen, lista para el chat de IA, Word o un correo electrónico.
- **Escritura a mano → texto.** Un modelo de IA con visión transcribe una página fielmente, errores
  incluidos, a una nota en Markdown: títulos, listas, casillas de verificación, tablas y fórmulas en LaTeX cuando
  las hay. Los dibujos, gráficas y diagramas se redibujan a partir de tus trazos reales como figuras
  SVG nítidas junto a la nota. La nota se abre dentro de la aplicación como una página editable al estilo de Typora,
  y **Exportar** la guarda como Markdown (con las figuras al lado), Word o PDF. **Escritura** muestra las
  notas en letra de imprenta o a mano, en cursiva o en mayúsculas de molde con las mismas letras que el chat de IA escribe
  en la pizarra; la escritura a mano va al archivo Word (incrustada, para que se vea en cualquier ordenador)
  y también al PDF.
- **Asistente de IA, para todo.** El chat está junto a la hoja y puede ver la pizarra, el
  documento abierto o la nota. Úsalo para resumir, pasar a limpio, extraer tareas pendientes, redactar textos,
  buscar errores en lo que has escrito, responder preguntas, escribir en la nota, buscar en la web y
  verificar números de IVA de la UE. Pega imágenes en él (`Cmd+V`, arrastrar y soltar o el clip), y haz
  su texto más grande o más pequeño con un gesto de pellizco o con `Cmd +` / `Cmd −`. Si el modelo que eliges no puede ver imágenes (un modelo solo de texto), la página la
  lee primero el agente elegido en *Lectura de páginas (OCR)* y se le entrega como texto. Detrás hay una cadena de respaldo de proveedores compatibles con OpenAI:
  LM Studio, Ollama, DeepSeek, Google Gemini y Anthropic Claude. Muestra el uso del contexto y lo
  compacta automáticamente. Arrastra la pestaña de un documento al chat para adjuntarlo.
- **Gráficas y cambios en tus documentos.** Pídele al chat una gráfica («dibuja las dos ecuaciones y
  la solución») y la dibuja en estilo LaTeX con [Typst](https://typst.app) y
  [Lilaq](https://lilaq.org): funciones, puntos, rectas, barras, curvas paramétricas. Con un documento
  adjunto al chat, también puede leerlo y **editarlo**, e insertar en él la gráfica, a través de los
  servidores MCP de la suite Craft que se ejecutan dentro de la aplicación: WordCraft para Word, PdfCraft para PDF,
  VectorCraft para SVG, PhotoCraft para fotos (recortar, redimensionar, rotar, brillo, niveles, curvas,
  blanco y negro, enfocar… la imagen conserva su formato), además de herramientas de texto para Markdown. Cada cambio aparece al instante en el documento
  junto al chat y se puede deshacer desde el chat con un clic; el archivo original en el disco
  nunca se toca. En una nota de transcripción la gráfica puede sustituir a la figura dibujada a mano; en la hoja de la pizarra sustituye a la gráfica dibujada a mano (que se localiza automáticamente: ejes y etiquetas) o se coloca debajo de lo escrito. El SVG que el modelo escribe en una respuesta se muestra como imagen.
- **El chat escribe en la pizarra.** Pídele «revisa mis sumas y marca los errores»,
  «subraya los plazos» o «escribe el resultado junto a la línea 2»: lee la hoja de la pizarra línea
  a línea y palabra a palabra, y después resalta, subraya, rodea con un círculo, tacha, dibuja recuadros,
  flechas, tics y cruces, y escribe a mano (en imprenta o cursiva, hecha de trazos con
  las fuentes de un solo trazo [Hershey](https://en.wikipedia.org/wiki/Hershey_fonts)). Si se lo pides,
  también mueve, amplía, cambia de color o borra lo que hay en la hoja («haz el título más grande y
  azul», «baja la última línea», «quita mis resaltados»); por su cuenta solo añade. Borrar
  la mayor parte de la hoja requiere tu sí en el chat. Todo lo que hace es tinta normal (se guarda,
  se exporta y se borra como la tuya) y un clic en el chat lo elimina todo de nuevo.
- **El chat conoce tu archivo.** «Busca la página en la que escribí sobre el presupuesto de octubre»,
  «abre en la pizarra las notas de ayer», «guarda la pizarra como "Reunión" y empieza una página nueva»,
  «exporta las páginas de esta semana como un único PDF»: busca en los títulos, las fechas y el texto de las
  transcripciones, mira una página, la abre en la pizarra, guarda la pizarra (actualizando la página de la que
  procede), renombra, exporta a tu carpeta de exportación habitual (sin sobrescribir nunca) y mueve páginas a
  la papelera y de vuelta. Mover a la papelera siempre espera tu sí en el chat.
- **Tus propios servidores MCP.** Añade cualquier servidor MCP (stdio) en `ai.toml`, como en Claude Desktop o DSH:
  sus herramientas quedan disponibles para el chat.
- **En directo en la web.** Un enlace para compartir permite que un televisor, un teléfono o un PC de tu red doméstica vean la pizarra
  en tiempo real. Los endpoints estables son `/page.pdf`, `/page.svg`, `/page.png` y `/state.json`.
  La pizarra en directo está abierta a la red doméstica; el chat, las notas y las conversaciones necesitan la clave
  que forma parte del enlace que muestra OpenSlate (`?k=…`, que luego recuerda el navegador). El servidor
  solo responde a direcciones IP, `localhost` y nombres `.local`, y rechaza las escrituras que llegan desde
  otros sitios web, así que una página que visites no puede controlar la aplicación.
- **No se pierde nada.** Si sales con algo en la pizarra que no está en el archivo, te pregunta
  *Guardar y salir / Salir sin guardar / No salir*; cuando la aplicación se cierra sin preguntar (Dock,
  apagado, actualización) la pizarra va al archivo por sí sola, y si la interfaz llega a bloquearse
  la pizarra también se guarda (el motivo va a `crash.log` en la carpeta de datos). Las notas se guardan
  con una comprobación de versión, de modo que una modificación de la IA o de otra aplicación nunca se sobrescribe sin avisar.
  Las páginas ya eliminadas del pad se recuperan de sus copias de seguridad en bruto si una descarga
  se interrumpe.
- **Solo la hoja.** El botón junto a Copiar/Pegar en la barra de título (o `Cmd+Shift+F`, Ver → Solo
  la hoja) oculta la barra de menús y la cinta; vuelve a hacer clic para que reaparezcan.
- **Más formatos.** Una página transcrita se convierte en un **PDF con búsqueda**: el texto queda invisible sobre la escritura a mano, así Spotlight y cualquier lector de PDF lo encuentran y puedes copiarlo. En **Más formatos**: JPEG y WebP (ligeros, para enviar), **Word** con la escritura como tinta real de Word que puedes editar (y la transcripción en la página siguiente), **Xournal++** (`.xopp`), un **GIF o SVG animado** que vuelve a escribir la página trazo a trazo, y **LaTeX** o **Typst** a partir de la transcripción (las notas también se exportan a ellos). El chat de IA puede exportar a todos ellos.
- **Guardar y exportar.** Exporta a PDF y SVG vectoriales a tamaño A4 real, o a PNG. Cada vez que guardas o
  exportas se abre el cuadro de diálogo «Guardar como» del sistema, así que siempre ves adónde va el archivo. Empieza en la
  última carpeta que elegiste (la primera vez, en `exports/`). También puedes copiar la página como imagen
  (`Cmd+C`) y pegarla en cualquier sitio.
- **Tu idioma.** Toda la interfaz (menús, cinta, opciones, chat, notas y ajustes de IA)
  está disponible en inglés, italiano, español, francés, alemán, ucraniano, chino, japonés y
  coreano. Sigue el idioma del sistema; cámbialo en Archivo → Opciones → Apariencia → Idioma.
  La IA responde en el idioma en el que escribes.

## Capturas de pantalla

Las capturas muestran la interfaz en inglés (hay ocho idiomas más integrados). La escritura a mano,
los documentos y las respuestas de la IA son contenido de ejemplo preparado para estas imágenes; las marcas en la
hoja y la gráfica las dibujaron las propias herramientas de pizarra de la aplicación.

| | |
|---|---|
| **Una gráfica en la hoja**: el chat dibuja la función y la coloca debajo del ejercicio | **Firma un documento Word o PDF** con el bolígrafo y guárdalo en el mismo formato |
| <img src="assets/screenshots/graph.png" alt="El chat de IA dibuja la gráfica de la función y la coloca en la hoja escrita a mano" width="440"> | <img src="assets/screenshots/sign.png" alt="Un documento Word firmado con el bolígrafo" width="440"> |
| **Escritura a mano → texto**: una página transcrita a una nota editable, fórmulas incluidas | **Notas a mano**: la misma nota en la fuente manuscrita cursiva |
| <img src="assets/screenshots/transcribe.png" alt="Una página escrita a mano transcrita a una nota con una tabla y fórmulas" width="440"> | <img src="assets/screenshots/notes.png" alt="La nota mostrada en la fuente manuscrita cursiva de OpenSlate" width="440"> |
| **Archivo** de las páginas descargadas del pad, agrupadas por días | **Tema oscuro** y tipos de papel (aquí cuadrícula de 5 mm) |
| <img src="assets/screenshots/library.png" alt="Archivo de páginas escritas a mano" width="440"> | <img src="assets/screenshots/dark.png" alt="Tema oscuro con papel cuadriculado y el chat de IA abierto" width="440"> |

## Instalación

Las versiones listas para instalar se publican en la página
[**Releases**](https://github.com/rmpaolillo/OpenSlate/releases), cada una con un número de versión
creciente (`v0.3.0`, `v0.3.1`, …).

| Plataforma | Archivo | Estado |
|---|---|---|
| macOS 11+, Apple Silicon (M1, M2…) | `OpenSlate-<version>-macos-arm64.dmg` | plataforma principal, de uso diario |
| macOS 11+, Intel | `OpenSlate-<version>-macos-intel.dmg` | se compila y funciona; menos probado |
| macOS 11+, ambas (el doble de tamaño) | `OpenSlate-<version>-macos-universal.dmg` | para la actualización desde la aplicación a partir de 0.2.x |
| Linux x86_64 (Debian/Ubuntu) | `openslate_<version>_amd64.deb` | se compila y funciona; menos probado |
| Linux x86_64 (otras distribuciones) | `openslate-<version>-linux-x86_64.tar.gz` | se compila y funciona; menos probado |
| Windows x86_64 | `openslate-<version>-windows-x86_64.zip` | **experimental** |

### macOS

1. Abre el `.dmg` y arrastra **OpenSlate** a **Aplicaciones**.
2. Las versiones compiladas con un Apple Developer ID están firmadas y notarizadas: se abren sin
   ninguna advertencia. Una compilación sin él (consulta «Firma y notarización» más abajo) está firmada ad hoc, así que la
   primera vez macOS dice que «no se puede abrir»: abre **Ajustes del Sistema → Privacidad y seguridad**
   y haz clic en **Abrir igualmente**, o ejecuta
   ```bash
   xattr -dr com.apple.quarantine /Applications/OpenSlate.app
   ```
   Solo hace falta en la primera instalación: las actualizaciones las descarga la aplicación y se abren directamente.
3. Permite el acceso a **Bluetooth** cuando se te pida.

No hay que instalar nada más: cada archivo se abre y se guarda con las bibliotecas de la
[suite Craft](https://github.com/storytold) (consulta «Archivos y formatos» más abajo).

### Linux

```bash
sudo apt install ./openslate_<version>_amd64.deb     # Debian / Ubuntu
# or unpack the tar.gz and run ./openslate
```

En tiempo de ejecución necesita BlueZ, GTK 3 y WebKitGTK 4.1. Tu usuario debe tener permiso para usar Bluetooth.

### Windows (experimental)

Descomprime y ejecuta `openslate.exe`. Necesita el runtime Microsoft Edge WebView2, que viene incluido con
Windows 10/11. No todo está probado en Windows. Los informes de errores son muy bienvenidos.

## Archivos y formatos

Cada archivo se abre, se muestra y se guarda con las bibliotecas de la suite Craft de ArtCraft, sin
sus interfaces y sin programas externos:

| Archivos | Biblioteca |
|---|---|
| Word (.docx, .doc, .odt, .rtf), Markdown, .txt, .html | [WordCraft](https://github.com/storytold/wordcraft): abre y compone las páginas, guarda Word con la tinta |
| PDF | [PdfCraft](https://github.com/storytold/pdfcraft): páginas, miniaturas y texto; su biblioteca PDF guarda la copia firmada |
| SVG | [VectorCraft](https://github.com/storytold/vectorcraft) |
| Fotos (PNG, JPEG, WebP, TIFF, BMP, GIF) | [PhotoCraft](https://github.com/storytold/photocraft) |
| Exportaciones de la pizarra y del archivo (PDF, SVG, PNG, JPEG, WebP) | VectorCraft las dibuja y las escribe; los archivos PNG, JPEG y WebP los escribe PhotoCraft |
| GIF y SVG animados, Xournal++, LaTeX, Typst, el texto oculto de los PDF con búsqueda | los escribe el propio OpenSlate (`src/formats.rs`, `src/mdconv.rs`) |

Un archivo Markdown abierto como documento tiene el mismo aspecto que la página de Notas: texto en Source Serif, títulos en
Inter, tablas ligeras. Sus fórmulas (`$…$`, `$$…$$`, `\(…\)`, `\[…\]`) las dibuja en estilo LaTeX
[Typst](https://typst.app) (el LaTeX lo convierte [MiTeX](https://github.com/mitex-rs/mitex)),
y las figuras que lo acompañan (`<img src="…">`, `![…](…)`, SVG dibujado por VectorCraft) se convierten en imágenes.
Un archivo Markdown, .txt o .html firmado se guarda como PDF (esos formatos no pueden contener la tinta).

## Actualizaciones

OpenSlate se actualiza solo, como las aplicaciones Craft, después de que lo confirmes. Cuando se publica una nueva versión
en GitHub, una barra azul bajo la cinta lo indica y ofrece **Actualizar**; después la
descarga, la comprueba con el `SHA256SUMS.txt` de la versión y con la firma del desarrollador,
la instala en lugar de la aplicación en ejecución y ofrece **Reiniciar ahora** (o simplemente usa la nueva versión
en el siguiente inicio). Opciones → Avanzado permite que se instale sin preguntar, o desactivar la
comprobación en sí; Archivo → Ayuda e información → Buscar actualizaciones comprueba en ese momento.

Tus ajustes, agentes de IA, archivo, documentos y exportaciones se encuentran en la carpeta de datos (macOS:
`~/Library/Application Support/OpenSlate`), fuera de la aplicación, y las claves de API en el
Llavero: una actualización nunca los toca. En macOS la actualización sustituye `OpenSlate.app`
(descargada por la propia aplicación, así que Gatekeeper no vuelve a preguntar); con el `.deb` pide
tu contraseña mediante `pkexec`; en Windows se instala al salir de la aplicación.

## Primera conexión

- **La primera vez**, o después de usar Inkspace: mantén pulsado el botón del pad unos 6 s hasta que el LED
  parpadee en azul. Luego haz clic en **Conectar** y pulsa el botón una vez cuando se te pida.
- **Las veces siguientes** (LED verde): pulsa el botón brevemente y luego haz clic en **Conectar**.

El pad solo recuerda una aplicación a la vez. Si vuelves a Inkspace, tendrás que registrar
OpenSlate de nuevo.

## Dónde se guardan tus datos

Una vez instalado, OpenSlate guarda tu archivo, documentos, chats y ajustes en:

- macOS: `~/Library/Application Support/OpenSlate`
- Linux: `~/.local/share/openslate` (o `$XDG_DATA_HOME/openslate`)
- Windows: `%APPDATA%\OpenSlate`

Puedes cambiarlo con la variable de entorno `OPENSLATE_DATA`. Cuando se inicia con
`cargo run` desde la carpeta del código fuente, trabaja en esa carpeta. Allí también encontrarás
`crash.log` (si la aplicación llegó a bloquearse), `web.key` (la clave del enlace web en directo) y `updates/`
(la versión anterior de la aplicación, conservada tras una actualización).

## Proveedores de IA y claves de API

Las funciones de IA son opcionales. Los proveedores se configuran en **Archivo → Opciones → IA** o en el archivo `ai.toml`,
que se crea con comentarios la primera vez que se usa. Funciona cualquier servidor que hable la API de OpenAI
`/chat/completions`, incluidos los locales (LM Studio, Ollama), que no necesitan clave.

**Lectura de páginas (OCR)** tiene su propio ajuste (`[ocr]` en `ai.toml`, o *Ajustes de IA → Lectura de
páginas*): elige el agente que lee la escritura a mano para *Transcribir* y para los modelos de chat solo de texto, o
déjalo vacío para usar el primer agente de la lista que pueda ver imágenes. Se recomienda uno rápido como Gemini;
`fallback = false` significa «siempre y solo ese».

**Las claves de API nunca se guardan en texto plano.** `ai.toml` solo indica *dónde* encontrar cada clave:

```toml
key = "env:DEEPSEEK_API_KEY"              # environment variable
key = "keychain:DEEPSEEK_API_KEY"         # macOS Keychain
key = "env:GEMINI_API_KEY | keychain:GEMINI_API_KEY"   # first one found wins
```

```bash
# macOS Keychain (recommended: also works when the app is opened from Finder)
security add-generic-password -a "$USER" -s DEEPSEEK_API_KEY -w 'sk-...'
```

### Servidores MCP

El chat puede usar herramientas de servidores [MCP](https://modelcontextprotocol.io) externos (transporte
stdio), configurados en `ai.toml` como en Claude Desktop:

```toml
[[mcp]]
name = "filesystem"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/Users/me/Documents"]
env = { TOKEN = "keychain:MY_TOKEN" }   # values resolved like API keys
# enabled = false   to turn it off; timeout = 120 (seconds per tool)
```

OpenSlate inicia el programa cuando el chat lo necesita y lo mantiene en ejecución; sus herramientas llegan al
modelo como `name__tool`. Los servidores MCP propios de la suite Craft (WordCraft, PdfCraft, VectorCraft) no necesitan
configuración: se ejecutan dentro de la aplicación sobre el documento adjunto al chat.

## Compilar desde el código fuente

Necesitas **Rust 1.95 o posterior** (`rustup update`).

```bash
git clone https://github.com/rmpaolillo/OpenSlate.git
cd OpenSlate
cargo run --release                  # GUI
cargo run --release -- gui --demo    # try it without a pad
cargo run --release -- --help        # command-line tools (scan, sync, live, pdf, ai, …)
cargo test
```

Dependencias de compilación en Linux (Debian/Ubuntu):

```bash
sudo apt install pkg-config libgtk-3-dev libwebkit2gtk-4.1-dev libsoup-3.0-dev \
                 libjavascriptcoregtk-4.1-dev libdbus-1-dev libxkbcommon-dev
```

En macOS necesitas las herramientas de línea de comandos de Xcode (`xcode-select --install`). El pequeño asistente
en Swift para el OCR opcional de Apple Vision se compila la primera vez que se usa, o viene incluido dentro del `.app` de la versión publicada.

## Estructura del proyecto

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
src/fonts.rs                                              the fonts embedded in the app (one copy each)
src/demo.rs                                               sample pages for the screenshots (tests only)
packaging/, .github/workflows/                            app bundle, .deb, CI and releases
```

Hay una descripción archivo por archivo en el [README en italiano](README.it.md#struttura).

## Versiones y numeración

- `ci.yml` compila y ejecuta las pruebas en Linux y macOS en cada push y pull request.
- `release.yml` se ejecuta cuando el mantenedor lo inicia (Actions → Release → Run workflow, en `main`).
  Compila para macOS (Apple Silicon, Intel y universal), Linux y Windows, y publica una GitHub Release con
  notas generadas automáticamente, y conserva solo las dos versiones más recientes. El binario de la versión se
  despoja de símbolos y se optimiza entre crates (`lto = "thin"`, `codegen-units = 1`); las bibliotecas
  poco usadas que arrastran otras se optimizan para tamaño, y cada fuente se incrusta una sola vez (consulta
  `[profile.release]` en `Cargo.toml` y `src/fonts.rs`).
- Las versiones siguen [Semantic Versioning](https://semver.org): `MAJOR.MINOR` procede de `Cargo.toml`,
  `PATCH` se cuenta automáticamente a partir de las etiquetas existentes (`v0.3.0`, luego `v0.3.1`, …).
- Para empezar una nueva serie (nuevas funciones: `0.3.0`; primera estable: `1.0.0`), cambia `version` en
  `Cargo.toml`; la siguiente versión vuelve a empezar desde el parche `0`.
- Una versión ejecuta primero las pruebas y compila con las dependencias bloqueadas (`Cargo.lock`). Cada
  paquete contiene `LICENSE`, `NOTICE`, `THIRD-PARTY.md` y `THIRD-PARTY-LICENSES.txt` (el
  texto completo de cada licencia de terceros, generado por `packaging/collect_licenses.py`).
- Cada versión tiene un `SHA256SUMS.txt`: la aplicación comprueba con él el archivo que descarga antes de
  instalar una actualización.

### Firma y notarización (macOS)

Para que macOS confíe en la aplicación sin advertencias, como con las aplicaciones Craft, la versión necesita un certificado
**Developer ID Application** de Apple (Apple Developer Program, de pago anual). Añade estos
secretos al repositorio (Settings → Secrets and variables → Actions):

| Secreto | Valor |
|---|---|
| `APPLE_CERTIFICATE` | el certificado «Developer ID Application» exportado como `.p12`, en base64 (`base64 -i cert.p12 \| pbcopy`) |
| `APPLE_CERTIFICATE_PASSWORD` | la contraseña elegida al exportar el `.p12` |
| `APPLE_ID` | el correo electrónico del Apple ID de la cuenta de desarrollador |
| `APPLE_PASSWORD` | una contraseña específica de app para ese Apple ID (appleid.apple.com → Inicio de sesión y seguridad) |
| `APPLE_TEAM_ID` | el Team ID de 10 caracteres (developer.apple.com → Membership) |

Con ellos, `release.yml` firma la aplicación con el hardened runtime y una marca de tiempo, la envía al
servicio de notarización de Apple, grapa el ticket y hace lo mismo con el DMG. Sin ellos, la aplicación
se firma ad hoc y la versión sigue funcionando (la primera apertura mediante **Abrir igualmente**).

## Créditos

OpenSlate no existiría sin estos proyectos y sus autores:

- **[Tuhi](https://github.com/tuhiproject/tuhi)**, de los colaboradores de Tuhi (GPL-2.0-or-later).
  El protocolo Bluetooth de la familia SmartPad se tradujo a partir de su trabajo de ingeniería
  inversa.
- **La suite Craft**, del ArtCraft Team y colaboradores (MIT OR Apache-2.0), usada como
  bibliotecas sin sus interfaces:
  - **[WordCraft](https://github.com/storytold/wordcraft)**: abre, compone y guarda documentos
    Word (y Markdown) con la tinta; la interfaz al estilo de Word está adaptada de su UI en egui;
    su crate de maquetación está copiado en `vendor/` con un pequeño cambio;
  - **[PdfCraft](https://github.com/storytold/pdfcraft)**: renderiza las páginas PDF nítidas con cualquier zoom,
    extrae su texto y guarda la copia firmada;
  - **[VectorCraft](https://github.com/storytold/vectorcraft)**: abre y guarda imágenes SVG con
    la tinta, y dibuja y escribe las exportaciones de la pizarra y del archivo (PDF, SVG, PNG);
  - **[PhotoCraft](https://github.com/storytold/photocraft)**: lee y escribe fotos (PNG, JPEG,
    WebP, TIFF…) conservando el perfil de color y el tamaño, y pinta la tinta sobre ellas;
  - **sus servidores MCP** (`wordcraft-mcp`, `pdfcraft-automation`, `vectorcraft-mcp`,
    `photocraft-automation`), que se ejecutan dentro de la aplicación para que el chat de IA pueda leer y editar el documento Word, PDF,
    SVG o la foto que tiene al lado;
  - bibliotecas parcheadas de [craft-libs](https://github.com/storytold/craft-libs);
  - [craft-launcher](https://github.com/storytold/craft-launcher): el diseño de las actualizaciones
    desde la aplicación y de la firma en macOS (solo ideas, sin copiar código).
- **[Typst](https://github.com/typst/typst)** (The Typst Project Developers, Apache-2.0): dibuja las
  fórmulas LaTeX y las gráficas del chat; dos de sus crates están copiados en `vendor/` con un cambio
  de una línea. **[MiTeX](https://github.com/mitex-rs/mitex)** (Apache-2.0) convierte LaTeX en Typst.
- **[Lilaq](https://github.com/lilaq-project/lilaq)** (Mc-Zen, MIT) y los paquetes de Typst que
  usa — [zero](https://github.com/Mc-Zen/zero), [tiptoe](https://github.com/Mc-Zen/tiptoe),
  [elembic](https://github.com/PgBiel/elembic), [suiji](https://github.com/liuguangxi/suiji):
  las gráficas que dibuja el chat de IA.
- **[egui / eframe](https://github.com/emilk/egui)**, de Emil Ernerfeldt y colaboradores: el
  toolkit de interfaz gráfica.
- **[wry](https://github.com/tauri-apps/wry), [tao](https://github.com/tauri-apps/tao),
  [muda](https://github.com/tauri-apps/muda)**, del equipo de Tauri: vistas web integradas y menús.
- **[btleplug](https://github.com/deviceplug/btleplug)**, de Nonpolynomial Labs: Bluetooth LE
  multiplataforma.
- **[tokio](https://tokio.rs)**, **[clap](https://github.com/clap-rs/clap)**,
  **[serde](https://serde.rs)**, **[tiny-skia](https://github.com/linebender/tiny-skia)**,
  **[lopdf](https://github.com/J-F-Liu/lopdf)**, **[hayro](https://github.com/LaurenzV/hayro)**,
  **[ureq](https://github.com/algesten/ureq)**, **[arboard](https://github.com/1Password/arboard)**,
  **[rfd](https://github.com/PolyMeilex/rfd)** y todos los demás crates enumerados en
  [THIRD-PARTY.md](THIRD-PARTY.md).
- **[Inter](https://github.com/rsms/inter)** (Rasmus Andersson),
  **[Source Serif 4](https://github.com/adobe-fonts/source-serif)** (Adobe) y un pequeño subconjunto
  de **[Noto Sans CJK](https://github.com/notofonts/noto-cjk)** (Adobe, para los nombres de los idiomas):
  fuentes, SIL OFL 1.1.
- **[Phosphor Icons](https://phosphoricons.com)** (Helena Zhang, Tobias Fried) mediante
  [egui-phosphor](https://github.com/amPerl/egui-phosphor): iconos, MIT.
- **[KaTeX](https://katex.org)** (Khan Academy) y **[marked](https://marked.js.org)**:
  fórmulas y Markdown en el chat y en las notas, MIT;
  **[DOMPurify](https://github.com/cure53/DOMPurify)** (Cure53, Apache-2.0 OR MPL-2.0) limpia
  ese HTML antes de mostrarlo.
- **[Fuentes Hershey](https://en.wikipedia.org/wiki/Hershey_fonts)** (Dr. A. V. Hershey, U.S.
  National Bureau of Standards; formato `.jhf` de James Hurt, Cognition Inc.; archivos de
  [hershey-fonts](https://github.com/kamalmostafa/hershey-fonts), de Kamal Mostafa): las
  letras de un solo trazo con las que el chat escribe en la pizarra (`src/handwrite/`, licencia libre con
  reconocimiento), convertidas además en las fuentes **OpenSlate Corsivo** y **OpenSlate Stampatello**
  de las notas manuscritas (`assets/fonts/OpenSlate*.ttf`).
- **[DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness)**: inspiración para el
  chat (solo ideas, sin copiar código).
- Escrito con **[Claude](https://claude.ai)**, de Anthropic, como programador en pareja de IA.

Hay una lista completa de crates con licencias y autores en [THIRD-PARTY.md](THIRD-PARTY.md).
Las atribuciones archivo por archivo están en [NOTICE](NOTICE).

## Licencia

Copyright © 2026 Marco Paolillo.

OpenSlate es software libre: puedes redistribuirlo y/o modificarlo según los términos de la
**GNU General Public License, versión 3 o (a tu elección) cualquier versión posterior**. Consulta
[LICENSE](LICENSE). La GPL es obligatoria porque el código del protocolo Bluetooth deriva de Tuhi.
