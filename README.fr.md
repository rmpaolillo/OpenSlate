<p align="center">
  <img src="assets/icon/openslate-256.png" width="128" height="128" alt="Icône d'OpenSlate">
</p>

<h1 align="center">OpenSlate</h1>

<p align="center">
  Une application compagnon libre et open source pour le <b>Wacom Bamboo Slate</b> (famille SmartPad).<br>
  Tableau en direct · synchronisation hors ligne · signature de documents Word et PDF au stylet · écriture manuscrite en texte · un assistant IA qui corrige vos exercices directement sur la page
</p>

<p align="center">
  <a href="https://github.com/rmpaolillo/OpenSlate/releases/latest"><img alt="Dernière version" src="https://img.shields.io/github/v/release/rmpaolillo/OpenSlate?sort=semver"></a>
  <a href="https://github.com/rmpaolillo/OpenSlate/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/rmpaolillo/OpenSlate/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="Licence : GPL-3.0-or-later" src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue"></a>
</p>

*[English](README.md) · [Italiano](README.it.md) · [Español](README.es.md) · Français · [Deutsch](README.de.md) · [Українська](README.uk.md) · [中文](README.zh.md) · [日本語](README.ja.md) · [한국어](README.ko.md)*

<p align="center">
  <img src="assets/screenshots/chat.png" alt="OpenSlate : l'assistant IA vérifie un exercice de maths manuscrit sur le tableau et le corrige à l'encre rouge" width="900">
</p>

---

## Pourquoi ce projet

J'ai un Bamboo Slate et je m'en sers tous les jours : pour écrire, pour faire des exercices et pour signer
des documents. Wacom a [annoncé la fin de vie](https://support.wacom.com/hc/en-us/articles/36070902398231-Bamboo-Paper-Inkspace-Wacom-Notes-application-End-of-Life-Notification)
d'Inkspace, l'application dont dépend le bloc : le matériel allait donc devenir un presse-papiers.

J'ai donc écrit ma propre application, pour mes propres besoins. **OpenSlate est un projet « vibe-codé »** : j'ai décrit
ce que je voulais et je l'ai écrit avec un assistant de programmation IA ([Claude](https://claude.ai)),
en avançant au fil de l'usage réel plutôt qu'en suivant une grande conception établie à l'avance. Il me rend de bons services sur
macOS. Je le partage au cas où il serait utile à d'autres possesseurs de Slate. Attendez-vous à quelques aspérités et à des commentaires
de code en italien (l'interface, elle, est traduite en neuf langues).
Les tickets et les pull requests sont les bienvenus : voir [CONTRIBUTING.md](CONTRIBUTING.md).

> OpenSlate est un projet indépendant. Il n'est **ni affilié à Wacom, ni approuvé ni soutenu par
> Wacom**. « Wacom », « Bamboo » et « Inkspace » sont des marques de leurs propriétaires respectifs.

## En bref

- **Écrivez sur le papier, voyez-le à l'écran.** Le Bamboo Slate transmet chaque trait en Bluetooth ;
  les pages écrites loin de l'ordinateur sont téléchargées plus tard dans les archives.
- **Un tuteur IA qui écrit sur votre page.** Demandez-lui de vérifier un exercice : il coche ce qui est juste,
  entoure l'erreur et écrit la correction à côté, à la main, puis l'explique dans
  le chat, avec les formules. Il peut aussi tracer le graphe d'une fonction sur la page.
- **Signez des documents Word et PDF au stylet**, enregistrés dans le même format.
- **Écriture manuscrite → notes modifiables**, avec tableaux et formules, exportées en Markdown, Word ou PDF,
  imprimées ou dans une police manuscrite.
- **L'IA de votre choix** : modèles locaux (LM Studio, Ollama) ou fournisseurs cloud ; les clés d'API ne sont jamais stockées
  en clair.
- **Libre et open source**, en neuf langues, pour macOS (plateforme principale), Linux et Windows.

## Fonctionnalités

- **Tableau en direct.** Connectez-vous en Bluetooth et voyez ce que vous écrivez sur le bloc au moment où vous l'écrivez,
  en plein écran. Vous disposez de stylos, d'un surligneur (`H` ; le basculement par un appui long du stylet peut être activé dans les Options), d'une gomme (mode
  gribouillis ou mode gomme), de couleurs, d'épaisseurs de trait, d'annuler/rétablir, de rotation/miroir et du zoom avec les
  gestes du trackpad.
- **Styles de papier.** Choisissez entre lignes, réglures des écoles primaires italiennes, petits carreaux (4/5/10 mm),
  papier millimétré, repère cartésien, pointillés, isométrique, portée musicale et prise de notes Cornell. Les réglures
  sont dessinées en taille réelle et n'apparaissent qu'à l'écran : les exports restent propres.
- **Pages hors ligne.** Téléchargez les pages écrites loin de l'ordinateur dans des archives
  classées par jour. De là, vous pouvez renommer, exporter, passer à l'OCR ou récupérer des pages de la corbeille.
- **Signez des documents Word et PDF.** Ouvrez un `.docx`/`.doc`/`.odt`/`.rtf` ou un `.pdf`, écrivez ou
  signez dessus au stylet et enregistrez-en une copie **dans le même format**. Dans un fichier Word,
  l'écriture devient de l'encre à l'intérieur du document : Word, Pages et LibreOffice l'affichent tous. Dans un
  PDF, elle devient des traits vectoriels posés sur l'original intact. Les pages restent nettes à tous les niveaux de zoom,
  rendues par les moteurs [WordCraft](https://github.com/storytold/wordcraft) et
  [PdfCraft](https://github.com/storytold/pdfcraft).
- **Écrivez aussi sur des images.** Ouvrez un SVG, PNG, JPEG (ou encore WebP, TIFF, BMP, GIF), ou appuyez sur **Coller**
  (`Cmd+V`) avec une capture d'écran ou une photo dans le presse-papiers. Écrivez dessus et réenregistrez-la comme image
  de même format et de même taille : un SVG conserve l'écriture sous forme de tracés vectoriels
  ([VectorCraft](https://github.com/storytold/vectorcraft)), une photo la reçoit peinte en pleine
  résolution, avec son profil colorimétrique conservé ([PhotoCraft](https://github.com/storytold/photocraft)).
- **Copier et Coller comme dans Word.** L'onglet Accueil commence par **Coller** et **Copier** : Copier place la
  page dans le presse-papiers sous forme d'image, prête pour le chat IA, Word ou un e-mail.
- **Écriture manuscrite → texte.** Un modèle d'IA doté de vision transcrit fidèlement une page, fautes
  comprises, en une note Markdown : titres, listes, cases à cocher, tableaux, et formules en LaTeX s'il
  y en a. Les dessins, graphiques et schémas sont redessinés à partir de vos vrais traits de stylo sous forme de figures
  SVG nettes à côté de la note. La note s'ouvre dans l'application comme une page modifiable façon Typora,
  et **Exporter** l'enregistre en Markdown (figures à côté), Word ou PDF. **Écriture** affiche les
  notes imprimées ou à la main, en cursive ou en script, avec les mêmes lettres que le chat IA écrit
  sur le tableau ; l'écriture manuscrite passe dans le fichier Word (intégrée, elle s'affiche donc sur n'importe quel ordinateur)
  ainsi que dans le PDF.
- **Un assistant IA, pour tout.** Le chat se trouve à côté de la page et peut voir le tableau, le
  document ouvert ou la note. Servez-vous-en pour résumer, remettre au propre, extraire les tâches à faire, rédiger des textes,
  chercher les erreurs dans ce que vous avez écrit, répondre à des questions, écrire dans la note, faire des recherches sur le web et
  vérifier des numéros de TVA intracommunautaire. Collez-y des images (`Cmd+V`, glisser-déposer ou le trombone), et agrandissez
  ou réduisez son texte en pinçant ou avec `Cmd +` / `Cmd −`. Si le modèle choisi ne voit pas les images (modèle texte seul), la page est d'abord
  lue par l'agent choisi sous *Lecture des pages (OCR)* et transmise sous forme de texte. Derrière, une chaîne de repli de fournisseurs compatibles OpenAI :
  LM Studio, Ollama, DeepSeek, Google Gemini et Anthropic Claude. Il affiche l'utilisation du contexte et le
  compacte automatiquement. Faites glisser l'onglet d'un document sur le chat pour le joindre.
- **Graphiques et modifications dans vos documents.** Demandez un graphique au chat (« tracez les deux équations et
  la solution ») et il le dessine dans le style LaTeX avec [Typst](https://typst.app) et
  [Lilaq](https://lilaq.org) : fonctions, points, droites, barres, courbes paramétriques. Avec un document
  joint au chat, il peut aussi le lire et **le modifier**, et y insérer le graphique, grâce aux
  serveurs MCP de la suite Craft qui tournent dans l'application : WordCraft pour Word, PdfCraft pour PDF,
  VectorCraft pour SVG, PhotoCraft pour les photos (recadrer, redimensionner, faire pivoter, luminosité, niveaux, courbes,
  noir et blanc, netteté… l'image garde son format), plus des outils texte pour Markdown. Chaque modification apparaît aussitôt dans le document
  à côté du chat et peut être annulée depuis le chat d'un seul clic ; le fichier original sur le disque n'est
  jamais modifié. Dans une note de transcription, le graphique peut remplacer la figure dessinée à la main ; sur la page du tableau, il remplace le graphique dessiné à la main (trouvé automatiquement : axes et légendes) ou se place sous l'écriture. Le SVG écrit par le modèle dans une réponse s'affiche comme une image.
- **Le chat écrit sur le tableau.** Demandez-lui « vérifiez mes additions et marquez les erreurs »,
  « soulignez les échéances » ou « écrivez le résultat à côté de la ligne 2 » : il lit la page du tableau ligne
  par ligne et mot par mot, puis surligne, souligne, entoure, barre, dessine des cadres,
  des flèches, des coches et des croix, et écrit à la main (en script ou en cursive, avec des traits tirés
  des polices à trait unique [Hershey](https://en.wikipedia.org/wiki/Hershey_fonts)). Sur demande, il
  déplace, agrandit, recolore ou efface aussi ce qui se trouve sur la page (« mettez le titre plus grand et en
  bleu », « descendez la dernière ligne », « enlevez mes surlignages ») ; de lui-même, il ne fait qu'ajouter. Effacer
  la plus grande partie de la page nécessite votre accord dans le chat. Tout ce qu'il fait est de l'encre ordinaire (enregistrée,
  exportée et effacée comme la vôtre) et un clic dans le chat retire le tout.
- **Le chat connaît vos archives.** « Retrouvez la page où j'ai écrit sur le budget d'octobre »,
  « ouvrez les notes d'hier sur le tableau », « enregistrez le tableau sous “Réunion” et commencez une nouvelle page »,
  « exportez les pages de cette semaine en un seul PDF » : il cherche dans les titres, les dates et le texte des
  transcriptions, regarde une page, l'ouvre sur le tableau, enregistre le tableau (en mettant à jour la page dont il
  provient), renomme, exporte dans votre dossier d'export habituel (sans jamais écraser) et met des pages à
  la corbeille ou les en sort. La mise à la corbeille attend toujours votre accord dans le chat.
- **Vos propres serveurs MCP.** Ajoutez n'importe quel serveur MCP (stdio) dans `ai.toml`, comme dans Claude Desktop ou DSH :
  ses outils deviennent disponibles pour le chat.
- **En direct sur le web.** Un lien de partage permet à une TV, un téléphone ou un PC de votre réseau domestique de suivre le tableau
  en temps réel. Les points d'accès stables sont `/page.pdf`, `/page.svg`, `/page.png` et `/state.json`.
  Le tableau en direct est ouvert au réseau domestique ; le chat, les notes et les conversations nécessitent la clé
  qui fait partie du lien affiché par OpenSlate (`?k=…`, ensuite mémorisée par le navigateur). Le serveur
  ne répond qu'aux adresses IP, à `localhost` et aux noms en `.local`, et refuse les écritures provenant
  d'autres sites web : une page que vous visitez ne peut donc pas piloter l'application.
- **Rien ne se perd.** Quitter alors que le tableau contient quelque chose qui n'est pas dans les archives vous propose
  *Enregistrer et quitter / Quitter sans enregistrer / Ne pas quitter* ; quand l'application se ferme sans demander (Dock,
  extinction, mise à jour), le tableau part de lui-même dans les archives, et si jamais l'interface plante,
  le tableau est enregistré aussi (la cause est consignée dans `crash.log` du dossier de données). Les notes sont enregistrées
  avec un contrôle de version : une modification faite par l'IA ou une autre application n'est jamais écrasée en silence.
  Les pages déjà supprimées du bloc sont récupérées à partir de leurs copies de sécurité brutes si un téléchargement
  est interrompu.
- **Page seule.** Le bouton à côté de Copier/Coller dans la barre de titre (ou `Cmd+Shift+F`, Affichage → Page
  seule) masque la barre des menus et le ruban ; cliquez à nouveau pour les faire revenir.
- **Autres formats.** Une page transcrite devient un **PDF consultable** : le texte est posé, invisible, sur l'écriture manuscrite, si bien que Spotlight et n'importe quel lecteur PDF le trouvent et que vous pouvez le copier. Dans **Autres formats** : JPEG et WebP (légers, à envoyer), **Word** avec l'écriture comme véritable encre Word modifiable (et la transcription sur la page suivante), **Xournal++** (`.xopp`), un **GIF ou SVG animé** qui réécrit la page trait par trait, et **LaTeX** ou **Typst** à partir de la transcription (les notes s'y exportent aussi). Le chat IA peut exporter dans tous ces formats.
- **Enregistrer et exporter.** Exportez en PDF et SVG vectoriels au format A4 réel, ou en PNG. Chaque enregistrement ou
  export ouvre la boîte de dialogue système « Enregistrer sous » : vous voyez toujours où va le fichier. Elle s'ouvre dans le
  dernier dossier choisi (la première fois, dans `exports/`). Vous pouvez aussi copier la page comme image
  (`Cmd+C`) et la coller n'importe où.
- **Votre langue.** Toute l'interface (menus, ruban, options, chat, notes et réglages IA)
  est disponible en anglais, italien, espagnol, français, allemand, ukrainien, chinois, japonais et
  coréen. Elle suit la langue du système ; changez-la dans Fichier → Options → Apparence → Langue.
  L'IA répond dans la langue dans laquelle vous écrivez.

## Captures d'écran

Les captures montrent l'interface en anglais (huit autres langues sont intégrées). L'écriture manuscrite,
les documents et les réponses de l'IA sont des contenus d'exemple préparés pour ces images ; les annotations sur la
page et le graphique ont été dessinés par les outils de tableau de l'application elle-même.

| | |
|---|---|
| **Un graphique sur la page** : le chat trace la fonction et la place sous l'exercice | **Signez un document Word ou PDF** au stylet et enregistrez-le dans le même format |
| <img src="assets/screenshots/graph.png" alt="Le chat IA trace le graphe de la fonction et le place sur la page manuscrite" width="440"> | <img src="assets/screenshots/sign.png" alt="Un document Word signé au stylet" width="440"> |
| **Écriture manuscrite → texte** : une page transcrite en note modifiable, formules comprises | **Notes à la main** : la même note dans la police manuscrite cursive |
| <img src="assets/screenshots/transcribe.png" alt="Une page manuscrite transcrite en note avec un tableau et des formules" width="440"> | <img src="assets/screenshots/notes.png" alt="La note affichée dans la police manuscrite cursive d'OpenSlate" width="440"> |
| **Archives** des pages téléchargées depuis le bloc, classées par jour | **Thème sombre** et styles de papier (ici des carreaux de 5 mm) |
| <img src="assets/screenshots/library.png" alt="Archives de pages manuscrites" width="440"> | <img src="assets/screenshots/dark.png" alt="Thème sombre avec papier à carreaux et le chat IA ouvert" width="440"> |

## Installation

Des versions prêtes à installer sont publiées sur la page
[**Releases**](https://github.com/rmpaolillo/OpenSlate/releases), chacune avec un numéro de version
croissant (`v0.3.0`, `v0.3.1`, …).

| Plateforme | Fichier | État |
|---|---|---|
| macOS 11+, Apple Silicon (M1, M2…) | `OpenSlate-<version>-macos-arm64.dmg` | plateforme principale, utilisée au quotidien |
| macOS 11+, Intel | `OpenSlate-<version>-macos-intel.dmg` | se compile et fonctionne ; moins testée |
| macOS 11+, les deux (taille double) | `OpenSlate-<version>-macos-universal.dmg` | pour la mise à jour intégrée depuis la 0.2.x |
| Linux x86_64 (Debian/Ubuntu) | `openslate_<version>_amd64.deb` | se compile et fonctionne ; moins testée |
| Linux x86_64 (autres distributions) | `openslate-<version>-linux-x86_64.tar.gz` | se compile et fonctionne ; moins testée |
| Windows x86_64 | `openslate-<version>-windows-x86_64.zip` | **expérimentale** |

### macOS

1. Ouvrez le `.dmg` et faites glisser **OpenSlate** dans **Applications**.
2. Les versions compilées avec un Developer ID Apple sont signées et notarisées : elles s'ouvrent sans
   avertissement. Une version qui n'en a pas (voir « Signature et notarisation » plus bas) est signée ad hoc : la
   première fois, macOS indique qu'elle « ne peut pas être ouverte ». Ouvrez **Réglages Système → Confidentialité et sécurité**
   et cliquez sur **Ouvrir quand même**, ou exécutez
   ```bash
   xattr -dr com.apple.quarantine /Applications/OpenSlate.app
   ```
   Seule la première installation l'exige : les mises à jour sont téléchargées par l'application et s'ouvrent directement.
3. Autorisez l'accès au **Bluetooth** lorsque c'est demandé.

Rien d'autre à installer : chaque fichier est ouvert et enregistré avec les bibliothèques de la
[suite Craft](https://github.com/storytold) (voir « Fichiers et formats » plus bas).

### Linux

```bash
sudo apt install ./openslate_<version>_amd64.deb     # Debian / Ubuntu
# or unpack the tar.gz and run ./openslate
```

À l'exécution, il faut BlueZ, GTK 3 et WebKitGTK 4.1. Votre utilisateur doit être autorisé à utiliser le Bluetooth.

### Windows (expérimental)

Décompressez l'archive et lancez `openslate.exe`. Il faut le runtime Microsoft Edge WebView2, fourni avec
Windows 10/11. Tout n'est pas testé sous Windows. Les rapports de bugs sont les très bienvenus.

## Fichiers et formats

Chaque fichier est ouvert, affiché et enregistré avec les bibliothèques de la suite Craft d'ArtCraft, sans
leurs interfaces et sans programmes externes :

| Fichiers | Bibliothèque |
|---|---|
| Word (.docx, .doc, .odt, .rtf), Markdown, .txt, .html | [WordCraft](https://github.com/storytold/wordcraft) : ouvre et met en page, enregistre le Word avec l'encre |
| PDF | [PdfCraft](https://github.com/storytold/pdfcraft) : pages, miniatures et texte ; sa bibliothèque PDF enregistre la copie signée |
| SVG | [VectorCraft](https://github.com/storytold/vectorcraft) |
| Photos (PNG, JPEG, WebP, TIFF, BMP, GIF) | [PhotoCraft](https://github.com/storytold/photocraft) |
| Exports du tableau et des archives (PDF, SVG, PNG, JPEG, WebP) | VectorCraft les dessine et les écrit ; les fichiers PNG, JPEG et WebP sont écrits par PhotoCraft |
| GIF et SVG animés, Xournal++, LaTeX, Typst, le texte caché des PDF consultables | écrits par OpenSlate lui-même (`src/formats.rs`, `src/mdconv.rs`) |

Un fichier Markdown ouvert comme document ressemble à la page Notes : texte en Source Serif, titres en Inter,
tableaux légers. Ses formules (`$…$`, `$$…$$`, `\(…\)`, `\[…\]`) sont dessinées dans le style LaTeX par
[Typst](https://typst.app) (le LaTeX est converti par [MiTeX](https://github.com/mitex-rs/mitex)),
et les figures qui l'accompagnent (`<img src="…">`, `![…](…)`, SVG dessiné par VectorCraft) deviennent des images.
Un fichier Markdown, .txt ou .html signé est enregistré en PDF (ces formats ne peuvent pas contenir l'encre).

## Mises à jour

OpenSlate se met à jour tout seul, comme les applications Craft, après votre confirmation. Quand une nouvelle version est
publiée sur GitHub, une barre bleue sous le ruban le signale et propose **Mettre à jour** ; l'application la
télécharge alors, la vérifie avec le `SHA256SUMS.txt` de la version et la signature du développeur,
l'installe à la place de l'application en cours et propose **Redémarrer** (ou utilisez simplement la nouvelle version
au prochain lancement). Options → Avancé permet de l'installer sans demander, ou de désactiver
la vérification elle-même ; Fichier → Aide et informations → Rechercher des mises à jour vérifie immédiatement.

Vos réglages, agents IA, archives, documents et exports se trouvent dans le dossier de données (macOS :
`~/Library/Application Support/OpenSlate`), en dehors de l'application, et les clés d'API dans le
Trousseau : une mise à jour n'y touche jamais. Sur macOS, la mise à jour remplace `OpenSlate.app`
(téléchargée par l'application elle-même, Gatekeeper ne redemande donc rien) ; avec le `.deb`, elle demande
votre mot de passe via `pkexec` ; sous Windows, elle est installée quand vous quittez l'application.

## Première connexion

- **La première fois**, ou après avoir utilisé Inkspace : maintenez le bouton du bloc enfoncé environ 6 s jusqu'à ce que la LED
  clignote en bleu. Cliquez ensuite sur **Connecter** et appuyez une fois sur le bouton lorsque c'est demandé.
- **Ensuite** (LED verte) : appuyez brièvement sur le bouton, puis cliquez sur **Connecter**.

Le bloc ne mémorise qu'une seule application à la fois. Si vous revenez à Inkspace, vous devrez enregistrer
OpenSlate à nouveau.

## Où se trouvent vos données

Une fois installé, OpenSlate conserve vos archives, documents, chats et réglages dans :

- macOS : `~/Library/Application Support/OpenSlate`
- Linux : `~/.local/share/openslate` (ou `$XDG_DATA_HOME/openslate`)
- Windows : `%APPDATA%\OpenSlate`

Vous pouvez changer cet emplacement avec la variable d'environnement `OPENSLATE_DATA`. Lancé avec
`cargo run` depuis le dossier des sources, il travaille plutôt dans ce dossier. Vous y trouverez aussi
`crash.log` (si l'application a déjà planté), `web.key` (la clé du lien web en direct) et `updates/`
(la version précédente de l'application, conservée après une mise à jour).

## Fournisseurs d'IA et clés d'API

Les fonctions d'IA sont facultatives. Les fournisseurs se configurent dans **Fichier → Options → IA** ou dans le fichier
`ai.toml`, créé avec des commentaires lors de la première utilisation. Tout serveur qui parle l'API OpenAI
`/chat/completions` fonctionne, y compris les serveurs locaux (LM Studio, Ollama), qui n'ont pas besoin de clé.

**La lecture des pages (OCR)** a son propre réglage (`[ocr]` dans `ai.toml`, ou *Réglages IA → Lecture des
pages (OCR)*) : choisissez l'agent qui lit l'écriture manuscrite pour *Transcrire* et pour les modèles de chat texte seul, ou
laissez vide pour utiliser le premier agent de la liste capable de voir les images. Un agent rapide comme Gemini
est recommandé ; `fallback = false` signifie « toujours et uniquement celui-là ».

**Les clés d'API ne sont jamais stockées en clair.** `ai.toml` indique seulement *où* trouver chaque clé :

```toml
key = "env:DEEPSEEK_API_KEY"              # environment variable
key = "keychain:DEEPSEEK_API_KEY"         # macOS Keychain
key = "env:GEMINI_API_KEY | keychain:GEMINI_API_KEY"   # first one found wins
```

```bash
# macOS Keychain (recommended: also works when the app is opened from Finder)
security add-generic-password -a "$USER" -s DEEPSEEK_API_KEY -w 'sk-...'
```

### Serveurs MCP

Le chat peut utiliser les outils de serveurs [MCP](https://modelcontextprotocol.io) externes (transport
stdio), configurés dans `ai.toml` comme dans Claude Desktop :

```toml
[[mcp]]
name = "filesystem"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/Users/me/Documents"]
env = { TOKEN = "keychain:MY_TOKEN" }   # values resolved like API keys
# enabled = false   to turn it off; timeout = 120 (seconds per tool)
```

OpenSlate lance le programme quand le chat en a besoin et le laisse tourner ; ses outils parviennent au
modèle sous la forme `name__tool`. Les serveurs MCP propres à la suite Craft (WordCraft, PdfCraft, VectorCraft) ne nécessitent
aucune configuration : ils tournent dans l'application, sur le document joint au chat.

## Compiler depuis les sources

Il vous faut **Rust 1.95 ou plus récent** (`rustup update`).

```bash
git clone https://github.com/rmpaolillo/OpenSlate.git
cd OpenSlate
cargo run --release                  # GUI
cargo run --release -- gui --demo    # try it without a pad
cargo run --release -- --help        # command-line tools (scan, sync, live, pdf, ai, …)
cargo test
```

Dépendances de compilation sous Linux (Debian/Ubuntu) :

```bash
sudo apt install pkg-config libgtk-3-dev libwebkit2gtk-4.1-dev libsoup-3.0-dev \
                 libjavascriptcoregtk-4.1-dev libdbus-1-dev libxkbcommon-dev
```

Sur macOS, il vous faut les outils en ligne de commande de Xcode (`xcode-select --install`). Le petit utilitaire
Swift pour l'OCR facultatif Apple Vision est compilé à la première utilisation, ou intégré dans le `.app` de la version publiée.

## Structure du projet

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

Une description fichier par fichier se trouve dans le [README italien](README.it.md#struttura).

## Versions et numérotation

- `ci.yml` compile et teste sous Linux et macOS à chaque push et pull request.
- `release.yml` s'exécute quand le mainteneur le lance (Actions → Release → Run workflow, sur `main`).
  Il compile pour macOS (Apple Silicon, Intel et universel), Linux et Windows, publie une GitHub Release avec
  des notes générées automatiquement et ne conserve que les deux versions les plus récentes. Le binaire de la version est
  dépouillé de ses symboles et optimisé à travers les crates (`lto = "thin"`, `codegen-units = 1`) ; les bibliothèques
  rarement utilisées, tirées par d'autres, sont optimisées pour la taille, et chaque police n'est intégrée qu'une fois (voir
  `[profile.release]` dans `Cargo.toml` et `src/fonts.rs`).
- Les versions suivent le [Semantic Versioning](https://semver.org) : `MAJOR.MINOR` vient de `Cargo.toml`,
  `PATCH` est compté automatiquement à partir des tags existants (`v0.3.0`, puis `v0.3.1`, …).
- Pour démarrer une nouvelle série (nouvelles fonctionnalités : `0.3.0` ; première version stable : `1.0.0`), changez `version` dans
  `Cargo.toml` ; la version suivante repart du patch `0`.
- Une version exécute d'abord les tests et se compile avec les dépendances verrouillées (`Cargo.lock`). Chaque
  paquet contient `LICENSE`, `NOTICE`, `THIRD-PARTY.md` et `THIRD-PARTY-LICENSES.txt` (le
  texte intégral de chaque licence tierce, produit par `packaging/collect_licenses.py`).
- Chaque version comporte un `SHA256SUMS.txt` : l'application vérifie le fichier qu'elle télécharge avec celui-ci avant
  d'installer une mise à jour.

### Signature et notarisation (macOS)

Pour que macOS fasse confiance à l'application sans avertissement, comme pour les applications Craft, la version a besoin d'un certificat Apple
**Developer ID Application** (Apple Developer Program, payant chaque année). Ajoutez ces
secrets au dépôt (Settings → Secrets and variables → Actions) :

| Secret | Valeur |
|---|---|
| `APPLE_CERTIFICATE` | le certificat « Developer ID Application » exporté en `.p12`, en base64 (`base64 -i cert.p12 \| pbcopy`) |
| `APPLE_CERTIFICATE_PASSWORD` | le mot de passe choisi lors de l'export du `.p12` |
| `APPLE_ID` | l'adresse e-mail de l'identifiant Apple du compte développeur |
| `APPLE_PASSWORD` | un mot de passe pour app propre à cet identifiant Apple (appleid.apple.com → Connexion et sécurité) |
| `APPLE_TEAM_ID` | le Team ID à 10 caractères (developer.apple.com → Adhésion) |

Avec eux, `release.yml` signe l'application avec le runtime renforcé et un horodatage, l'envoie au
service de notarisation d'Apple, agrafe le ticket et fait de même pour le DMG. Sans eux, l'application
est signée ad hoc et la version fonctionne quand même (première ouverture via **Ouvrir quand même**).

## Remerciements

OpenSlate n'existerait pas sans ces projets et leurs auteurs :

- **[Tuhi](https://github.com/tuhiproject/tuhi)** par les contributeurs de Tuhi (GPL-2.0-or-later).
  Le protocole Bluetooth de la famille SmartPad a été traduit à partir de leur travail de rétro-ingénierie.
- **La suite Craft** par l'ArtCraft Team et ses contributeurs (MIT OR Apache-2.0), utilisée comme
  bibliothèques sans leurs interfaces :
  - **[WordCraft](https://github.com/storytold/wordcraft)** : ouvre, met en page et enregistre les documents
    Word (et Markdown) avec l'encre ; l'interface façon Word est adaptée de son interface egui ;
    sa crate de mise en page est copiée dans `vendor/` avec une petite modification ;
  - **[PdfCraft](https://github.com/storytold/pdfcraft)** : affiche les pages PDF nettes à tous les niveaux de zoom,
    en extrait le texte et enregistre la copie signée ;
  - **[VectorCraft](https://github.com/storytold/vectorcraft)** : ouvre et enregistre les images SVG avec
    l'encre, et dessine et écrit les exports du tableau et des archives (PDF, SVG, PNG) ;
  - **[PhotoCraft](https://github.com/storytold/photocraft)** : lit et écrit les photos (PNG, JPEG,
    WebP, TIFF…) en conservant profil colorimétrique et taille, et y peint l'encre ;
  - **leurs serveurs MCP** (`wordcraft-mcp`, `pdfcraft-automation`, `vectorcraft-mcp`,
    `photocraft-automation`), exécutés dans l'application pour que le chat IA puisse lire et modifier le document Word, PDF,
    SVG ou la photo d'à côté ;
  - des bibliothèques corrigées issues de [craft-libs](https://github.com/storytold/craft-libs) ;
  - [craft-launcher](https://github.com/storytold/craft-launcher) : la conception des mises à jour intégrées
    et de la signature macOS (idées seulement, aucun code copié).
- **[Typst](https://github.com/typst/typst)** (The Typst Project Developers, Apache-2.0) : dessine les
  formules LaTeX et les graphiques du chat ; deux de ses crates sont copiées dans `vendor/` avec une modification
  d'une ligne. **[MiTeX](https://github.com/mitex-rs/mitex)** (Apache-2.0) convertit le LaTeX en Typst.
- **[Lilaq](https://github.com/lilaq-project/lilaq)** (Mc-Zen, MIT) et les paquets Typst qu'il
  utilise — [zero](https://github.com/Mc-Zen/zero), [tiptoe](https://github.com/Mc-Zen/tiptoe),
  [elembic](https://github.com/PgBiel/elembic), [suiji](https://github.com/liuguangxi/suiji) :
  les graphiques dessinés par le chat IA.
- **[egui / eframe](https://github.com/emilk/egui)** par Emil Ernerfeldt et ses contributeurs : la
  boîte à outils graphique.
- **[wry](https://github.com/tauri-apps/wry), [tao](https://github.com/tauri-apps/tao),
  [muda](https://github.com/tauri-apps/muda)** par l'équipe Tauri : vues web intégrées et menus.
- **[btleplug](https://github.com/deviceplug/btleplug)** par Nonpolynomial Labs : Bluetooth LE
  multiplateforme.
- **[tokio](https://tokio.rs)**, **[clap](https://github.com/clap-rs/clap)**,
  **[serde](https://serde.rs)**, **[tiny-skia](https://github.com/linebender/tiny-skia)**,
  **[lopdf](https://github.com/J-F-Liu/lopdf)**, **[hayro](https://github.com/LaurenzV/hayro)**,
  **[ureq](https://github.com/algesten/ureq)**, **[arboard](https://github.com/1Password/arboard)**,
  **[rfd](https://github.com/PolyMeilex/rfd)** et toutes les autres crates listées dans
  [THIRD-PARTY.md](THIRD-PARTY.md).
- **[Inter](https://github.com/rsms/inter)** (Rasmus Andersson),
  **[Source Serif 4](https://github.com/adobe-fonts/source-serif)** (Adobe) et un petit sous-ensemble
  de **[Noto Sans CJK](https://github.com/notofonts/noto-cjk)** (Adobe, pour les noms des langues) :
  polices, SIL OFL 1.1.
- **[Phosphor Icons](https://phosphoricons.com)** (Helena Zhang, Tobias Fried) via
  [egui-phosphor](https://github.com/amPerl/egui-phosphor) : icônes, MIT.
- **[KaTeX](https://katex.org)** (Khan Academy) et **[marked](https://marked.js.org)** :
  formules et Markdown dans le chat et les notes, MIT ;
  **[DOMPurify](https://github.com/cure53/DOMPurify)** (Cure53, Apache-2.0 OR MPL-2.0) nettoie
  ce HTML avant son affichage.
- **[Polices Hershey](https://en.wikipedia.org/wiki/Hershey_fonts)** (Dr A. V. Hershey, U.S.
  National Bureau of Standards ; format `.jhf` par James Hurt, Cognition Inc. ; fichiers issus de
  [hershey-fonts](https://github.com/kamalmostafa/hershey-fonts) par Kamal Mostafa) : les
  lettres à trait unique avec lesquelles le chat écrit sur le tableau (`src/handwrite/`, licence libre avec
  mention obligatoire), transformées aussi en polices **OpenSlate Corsivo** et **OpenSlate Stampatello**
  pour les notes manuscrites (`assets/fonts/OpenSlate*.ttf`).
- **[DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness)** : source d'inspiration pour le
  chat (idées seulement, aucun code copié).
- Écrit avec **[Claude](https://claude.ai)** d'Anthropic comme binôme de programmation IA.

La liste complète des crates avec leurs licences et leurs auteurs se trouve dans [THIRD-PARTY.md](THIRD-PARTY.md).
Les attributions fichier par fichier se trouvent dans [NOTICE](NOTICE).

## Licence

Copyright © 2026 Marco Paolillo.

OpenSlate est un logiciel libre : vous pouvez le redistribuer et/ou le modifier selon les termes de la
**GNU General Public License, version 3 ou (à votre choix) toute version ultérieure**. Voir
[LICENSE](LICENSE). La GPL est requise parce que le code du protocole Bluetooth est dérivé de Tuhi.
