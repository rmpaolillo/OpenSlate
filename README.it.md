# OpenSlate

*[English](README.md) · Italiano · [Español](README.es.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Українська](README.uk.md) · [中文](README.zh.md) · [日本語](README.ja.md) · [한국어](README.ko.md)*

App **libera, in Rust e modificabile** per il **Wacom Bamboo Slate** (famiglia SmartPad).
Non è un prodotto Wacom e non è collegata a Wacom.

![OpenSlate: l'assistente AI controlla un esercizio di matematica scritto a mano sulla lavagna e lo corregge in rosso](assets/screenshots/chat.png)

## In breve

- **Scrivi sulla carta, lo vedi sullo schermo.** Lo Slate manda ogni tratto in Bluetooth; le
  pagine scritte lontano dal computer si scaricano dopo nell'archivio.
- **Un'AI che corregge sul foglio.** Le chiedi di controllare un esercizio: spunta quello che è
  giusto, cerchia l'errore e scrive accanto la correzione a mano, poi la spiega nella chat, con le
  formule. Sa anche disegnare sul foglio il grafico di una funzione.
- **Firmi documenti Word e PDF con la penna**, salvati nello stesso formato.
- **Dalla scrittura a mano agli appunti**, con tabelle e formule, esportati in Markdown, Word o
  PDF, in stampato o con un carattere a mano.
- **Esporti in quello che ti serve**: PDF ricercabile, Word con la scrittura come inchiostro
  modificabile, JPEG/WebP leggeri, Xournal++, GIF, SVG o video MP4 della pagina che si scrive, LaTeX
  o Typst dalla trascrizione.
- **Parli alla chat.** Detti col microfono: Whisper trascrive sul tuo computer, senza rete.
- **Il tutor ti risponde a voce** con le voci del sistema.
- **L'AI che vuoi tu**: modelli locali (LM Studio, Ollama) o in cloud; le chiavi API mai in chiaro.
- **Libera e open source**, in nove lingue, per macOS (piattaforma principale), Linux e Windows.

Le immagini mostrano l'interfaccia in inglese. La scrittura, i documenti e le risposte dell'AI
sono esempi preparati per queste immagini; i segni sul foglio e il grafico li hanno fatti gli
strumenti della lavagna dell'app.

| | |
|---|---|
| **La pagina che si riscrive**: l'export in GIF animata, tratto dopo tratto | **Altri formati**: JPEG, WebP, Word con l'inchiostro, Xournal++, GIF/SVG animati, video MP4, LaTeX, Typst |
| ![La pagina che si riscrive](assets/screenshots/writing.gif) | ![Altri formati](assets/screenshots/formats.png) |
| **Un grafico sul foglio**: la chat disegna la funzione e la mette sotto l'esercizio | **Firma di un documento Word o PDF** con la penna, salvato nello stesso formato |
| ![Grafico sul foglio](assets/screenshots/graph.png) | ![Firma](assets/screenshots/sign.png) |
| **Scrittura → testo**: la pagina diventa un appunto modificabile, formule comprese | **Appunti a mano**: lo stesso appunto con il carattere corsivo |
| ![Trascrizione](assets/screenshots/transcribe.png) | ![Appunti in corsivo](assets/screenshots/notes.png) |
| **Archivio** delle pagine scaricate dal pad, per giorno | **Tema scuro** e rigature (qui quadretti da 5 mm) |
| ![Archivio](assets/screenshots/library.png) | ![Tema scuro](assets/screenshots/dark.png) |

Il protocollo Bluetooth è una traduzione fedele di [Tuhi](https://github.com/tuhiproject/tuhi) (GPL).

## Cosa fa

L'interfaccia è **in stile Word**, ripresa da [WordCraft](https://github.com/storytold/wordcraft)
(stessi colori, carattere Inter, misure della barra multifunzione, righelli, barra di stato):

Ogni comando ha **un posto solo** (più la ricerca `Cmd+K`, il tasto destro e le scorciatoie):

- **Barra del titolo**: a sinistra accesso rapido (Salva, Annulla, Ripeti), al centro il nome del
  foglio o del documento, a destra **«Cerca comandi»** (`Cmd+F` / `Cmd+K`: scrivi «esporta»,
  «firma», «zoom»… e `Invio`), **«Chat AI»** (apre e chiude il riquadro della chat), **«?»**
  (aiuto, anche `F1`) e la **spia dello Slate**, che è la casa del pad: **verde** collegato (piena
  = in diretta), **arancio lampeggiante** mentre si collega, **rossa** scollegato. Un clic apre il
  menu: Collega, diretta, scarica le pagine, batteria e firmware, «Come si collega», opzioni del
  pennino. Su macOS la barra sta accanto ai semafori della finestra.
- **Quattro schede**: **File** apre la pagina a tutta finestra come in Word (Home con i documenti
  recenti, Nuovo, Apri, Salva, **Aiuto e info**, **Opzioni** `Cmd+,`; `Esc` per tornare), poi
  **Home** (penna, evidenziatore, gomma, pulisci, colori, spessore, righe e quadretti, salva),
  **Documento** (apri, recenti, chiudi, pagine, **registra e inserisci la firma**, salva PDF),
  **Visualizza** (righelli, zoom, ruota e specchia, schermo intero), **AI** (trascrivi, appunti,
  controlla il foglio, discuti il foglio o il documento, impostazioni AI). Doppio clic su una
  scheda nasconde la barra (e la rimostra). Con un appunto aperto compare la scheda «Appunto», nella
  vista Archivio la scheda «Archivio». A destra: **Lavagna | Archivio** e il pulsante blu
  **Condividi** (diretta web, copia come immagine `Cmd+C`, esporta in PDF / PNG / SVG, cartella
  degli export).
- **Tema chiaro/scuro**, lato della chat, rigatura di serie e opzioni del pennino stanno in
  **File → Opzioni**.
- **Il foglio** sta su un tavolo grigio con l'ombra, con i **righelli in centimetri** (si
  nascondono da Visualizza o dal pulsante righello in basso); sopra il foglio, una scheda per ogni
  file aperto con la ✕ per chiuderlo.
- **Barra di stato**: pagina, numero di tratti, strumento, messaggi; a destra righelli, schermo
  intero e lo **zoom** (−, cursore, +, percentuale: clic = pagina intera).
- **Slate non collegato**: sotto la barra compare una striscia gialla con i passi (premi il pulsante
  del pad, poi «Collega»; la prima volta tieni premuto ~6 s finché il LED lampeggia blu) e il
  pulsante **Collega**. Sparisce appena lo Slate è collegato.
- **Tutto in una finestra**: appunti, chat AI e impostazioni AI non aprono altre finestre.
- **Opzioni** (File › Opzioni): aspetto (tema, righelli), penna ed evidenziatore, pad e foglio,
  diretta web, trascrizione e AI, archivio ed export, avanzate; «Applica» o «OK».
- **Lingua dell'interfaccia**: italiano, inglese, spagnolo, francese, tedesco, ucraino, cinese,
  giapponese e coreano. Di serie segue la lingua del sistema; si cambia in File › Opzioni › Aspetto ›
  Lingua e vale subito (il menu di macOS al prossimo avvio). L'AI risponde nella lingua in cui scrivi.

| Funzione | Come |
|---|---|
| Lavagna live a tutto schermo | «Collega», poi scrivi sul pad. `F` = schermo intero (la barra sparisce), `Esc` per uscire |
| **Firmare Word e PDF** | Apri un **.docx / .doc / .odt / .rtf** o un **.pdf** (Documento → Apri, il «+» delle schede, trascinando il file o con `gui --apri file`): le pagine compaiono sulla lavagna, con la solita barra. Scrivi e firmi col pennino; **«Salva»** (scheda Documento, o Cmd+Shift+S) chiede dove salvare (ultima cartella scelta, la prima volta `exports/`) e scrive una copia **nello stesso formato**: un Word resta Word (la scrittura è inchiostro dentro il documento: la vedono Word, Pages, LibreOffice), un PDF resta PDF (tratti vettoriali sopra l'originale, testo selezionabile). L'originale non viene toccato. Le pagine si vedono nitide a ogni ingrandimento, come in un programma di videoscrittura: la parte visibile la disegnano, alla risoluzione dello schermo, il motore di [WordCraft](https://github.com/storytold/wordcraft) per i Word e quello di [PdfCraft](https://github.com/storytold/pdfcraft) per i PDF, senza le loro interfacce |
| **Immagini** | Apri un **SVG, PNG, JPEG** (anche WebP, TIFF, BMP, GIF) come un documento, oppure **Incolla** (`Cmd+V`, Home → Copia e incolla) una schermata o una foto dagli appunti: ci scrivi e la salvi firmata **nello stesso formato e con gli stessi pixel**. Un SVG tiene la scrittura come tracciati vettoriali (motore di [VectorCraft](https://github.com/storytold/vectorcraft)); su una foto la scrittura la dipinge il pennello di [PhotoCraft](https://github.com/storytold/photocraft), con profilo colore, DPI e metadati dell'originale. Solo le loro librerie, senza le interfacce |
| Documenti da firmare (altri formati) | .md, .txt, .html: le pagine diventano lo sfondo della lavagna: ci scrivi e firmi col pennino, anche in diretta web. Tutto si salva da solo in `documents/` e si riapre da «Recenti». **«Salva PDF»** scrive in `exports/` il documento con tratti e firma sopra in grafica vettoriale |
| Zoom e gesti del trackpad (lavagna e archivio) | **Trackpad del Mac**: pizzico = zoom attorno alle dita, due dita = sposta il foglio, Cmd + due dita su/giù = zoom, **due dita che ruotano = gira il foglio di 90°** (come in Anteprima). Mouse: rotella = zoom attorno al puntatore, Shift + rotella = di lato; trascina = sposta (col tasto centrale se è attiva la gomma o l'evidenziatore); doppio clic = 200% / pagina intera. Tastiera: `Cmd +`, `Cmd −`, `Cmd 0`. In basso a destra il cursore dello zoom. Nella griglia dell'archivio il pizzico (o `Cmd +/−`) cambia la grandezza delle anteprime |
| Evidenziatore giallo | Pulsante «Evidenziatore» o tasto `H`, oppure **dal pennino: tocco lungo** (da accendere in Opzioni: spento di serie, perché chi scrive guardando la carta non si accorgerebbe del cambio; punta ferma 1,5 s, poi alzala). **Anche col mouse**: tasto sinistro premuto sul foglio; **con Shift la linea è dritta**, orizzontale o verticale (col pennino resta a mano libera); col mouse il foglio si sposta col tasto centrale. È trasparente come un evidenziatore vero: il testo dei documenti sotto resta nero e leggibile (schermo, diretta web, PDF, PNG). Un'etichetta gialla sul foglio dice quando è attivo. Per togliere: Annulla, pulsante accanto a «Evidenziatore» (tutte, `Shift+H`; oppure «Evidenziature → scrittura» se hai scritto con l'evidenziatore senza volerlo), `Alt`+clic su una, oppure «Gomma» con l'evidenziatore attivo (toglie solo il giallo, l'inchiostro resta) |
| Righe e quadretti | Scheda Home → «Righe e quadretti»: foglio bianco; righe (8 mm o 1 cm) e righe di 1ª, 2ª e 3ª elementare con i due margini rossi a sinistra e a destra (si possono togliere); quadretti da 5 mm (anche con margine a sinistra), 4 mm o 1 cm; carta millimetrata; piano cartesiano con gli assi numerati; puntinato; isometrica (triangoli); pentagramma; appunti Cornell (parole chiave, appunti, riassunto). Le righe delle elementari hanno misure indicative (ogni marca ha le sue). Le misure sono vere (centimetri del pad). Solo sullo schermo: le immagini per l'AI e gli export restano puliti |
| Colori | 6 colori nella barra, oppure tasti `1`…`6` |
| Spessore | `–` / `+` nella barra, oppure tasti `-` / `+` |
| Gomma | Pulsante «Gomma» o tasto `E`: il pennino cancella ciò che attraversa (uno zig-zag o una passata). **Anche col mouse**: tieni premuto il tasto sinistro e passa sul foglio, cancella ciò che tocca il cerchio rosso (una passata = un solo `Cmd+Z`; per spostare il foglio, tasto centrale o trackpad). Con l'evidenziatore attivo toglie solo il giallo. Resta attiva finché la spegni; etichetta rossa GOMMA sul foglio. Lo zig-zag automatico (senza Gomma) è spento di serie perché le firme sono spesso zig-zag: si riaccende con `zigzag_auto = 1` |
| Annulla / Ripeti | `Cmd+Z` / `Shift+Cmd+Z` (anche dopo «Pulisci» o la gomma) |
| Foglio in una chat AI | **Copia** (Home → Copia e incolla) oppure `Cmd+C`, poi `Cmd+V` nella chat. Ritaglia attorno alla scrittura (`copy_crop = 0` per il foglio intero) |
| Immagini nella chat AI | Incollale nel riquadro di scrittura (`Cmd+V`), trascinale lì o scegline con la graffetta (fino a 8 per messaggio): le vede l'agente che legge le immagini; a uno solo testo le legge prima un agente con visione. Il testo della chat si ingrandisce col pizzico sul trackpad o con `Cmd +` / `Cmd −` / `Cmd 0`, e resta |
| Aggiornamenti | Come le app Craft, OpenSlate **si aggiorna da solo, dopo che lo confermi**: quando su GitHub esce una versione nuova una barra azzurra sotto la barra multifunzione lo dice e offre **Aggiorna**; allora la scarica (sul Mac la versione per il suo processore, Apple Silicon o Intel: metà del peso di quella universale), la controlla con le impronte della release (`SHA256SUMS.txt`) e la firma dello sviluppatore, la installa al posto di quella in uso e offre **Riavvia ora** (oppure la nuova versione parte al prossimo avvio). Opzioni → Avanzate la fa installare senza chiedere, o spegne il controllo stesso; File → Aiuto e informazioni → Cerca aggiornamenti controlla subito. Impostazioni, agenti AI, archivio, documenti ed export stanno nella cartella dei dati (macOS: `~/Library/Application Support/OpenSlate`), fuori dall'app, e le chiavi nel Portachiavi: l'aggiornamento non li tocca. Su macOS l'app scaricata da OpenSlate stesso non chiede di nuovo il permesso a Gatekeeper; col `.deb` chiede la password (`pkexec`); su Windows si installa quando chiudi l'app |
| Salvare ed esportare | PDF e SVG vettoriali in misura A4 reale, PNG. Ogni volta si apre la finestra «Salva con nome» del sistema, così si vede dove va il file: parte dall'ultima cartella scelta (la prima volta da `exports/`); più immagini insieme = si sceglie la cartella. La cartella proposta si cambia anche in File → Opzioni → Archivio ed export. Una pagina trascritta diventa un **PDF ricercabile**: il testo sta, invisibile, sopra la scrittura (lo trovano Spotlight e i lettori PDF, e si copia). **Altri formati**: JPEG e WebP (leggeri, da mandare), **Word** con la scrittura come inchiostro vero di Word, modificabile (e la trascrizione nella pagina dopo), **Xournal++** (`.xopp`), **GIF o SVG animato** o **video MP4** che riscrive la pagina tratto per tratto (il video con il codificatore del sistema: AVFoundation su macOS, Media Foundation su Windows; su Linux ffmpeg, oppure OpenH264 di Cisco scaricato al primo uso se lo vuoi), **LaTeX** o **Typst** dalla trascrizione (anche dagli appunti: Esporta). Anche la chat AI esporta in tutti questi formati |
| Archivio (come la libreria di Inkspace) | Scheda «Archivio»: anteprime delle pagine raggruppate per giorno. Clic = seleziona (Cmd/Shift per più pagine), doppio clic = apri a tutta finestra (frecce per sfogliare), tasto destro = menu. Rinomina, porta sulla lavagna, copia immagine, esporta (più pagine = un unico PDF), testo (OCR), elimina (va in `library/cestino`, con «Ripristina») |
| Uscire senza perdere niente | Se sulla lavagna c'è qualcosa che non è nell'archivio, chiudendo (pulsante rosso, Cmd+Q, File → Esci) OpenSlate chiede **Salva ed esci / Esci senza salvare / Non uscire**. Se si chiude senza poter chiedere (Esci dal Dock, spegnimento, riavvio per l'aggiornamento) la lavagna va nell'archivio da sola. Se l'interfaccia va in crash la lavagna si salva lo stesso (il motivo in `crash.log` nella cartella dei dati). Gli appunti si salvano con un controllo di versione: una modifica dell'AI o di un'altra app non viene mai sovrascritta senza chiedere. Le pagine già tolte dal pad si recuperano dalle copie grezze se lo scaricamento si interrompe. Un grafico che l'AI ha messo sul foglio si salva con la pagina (`library/<id>.grafico.png`) e torna quando la rimetti sulla lavagna; si vede in anteprima, negli export e nella trascrizione. Documenti e appunti si salvano da soli |
| Pagine fatte senza Mac | «Scarica dal pad»: le pagine finiscono nell'archivio, con una copia grezza in `exports/pad-raw/`. Solo dopo il salvataggio le toglie dal pad. L'archivio mostra anche quante pagine restano sul pad |
| Scrittura → testo | «Trascrivi» (scheda AI o Appunto): un agente AI con visione legge il foglio e lo riscrive in Markdown (titoli, elenchi, caselle, tabelle; le formule in LaTeX `$…$`, `$$…$$` se ci sono), **fedele al foglio, errori compresi**. **Disegni, grafici e schemi vengono ridisegnati** con i tratti veri della penna (un SVG nitido accanto all'appunto, con la didascalia) (illeggibile = `[?]`). L'appunto (`exports/appunti-*.md`) si apre **dentro l'app, come un documento di Word**: foglio bianco, formule impaginate. Anche dall'archivio, su più pagine |
| Appunti (come Typora, dentro l'app) | Una scheda accanto a «Lavagna». **Clic su un paragrafo o su una formula = il suo Markdown**, con l'anteprima della formula sotto; fuori = di nuovo impaginato (`Esc`). Invio su una riga vuota = paragrafo nuovo, frecce su/giù tra i paragrafi, `Cmd+B` / `Cmd+I` / `Cmd+M` (formula), `Cmd+/` tutto il Markdown, pizzico o `Cmd +/−` = zoom. Si salva da solo. Scheda **Appunto**: titoli, elenchi, grassetto, formule (in linea, su riga sua, sistema), tabella, sorgente, elenco degli appunti, **Esporta** (Markdown con le figure accanto, Word, PDF, LaTeX, Typst), **Scrittura** (stampata, oppure a mano in corsivo o stampatello con le stesse lettere della lavagna: anche nel Word, coi caratteri dentro il file, e nel PDF), copia Markdown, stampa / PDF, apri in Typora. Se l'AI (o un altro programma) cambia il file, la pagina si aggiorna e i paragrafi nuovi lampeggiano |
| Chat AI (come DSH, aspetto WordCraft) | **Dentro la finestra, a sinistra e più larga del foglio** (pulsante «Chat AI» in alto); a sinistra o a destra si sceglie in File → Opzioni → Trascrizione e AI, la larghezza si regola trascinando il bordo e resta. Un assistente per qualsiasi cosa: riassumere, riscrivere in bella copia, ricavare le cose da fare, scrivere testi, rispondere a domande. **«Controlla il foglio»** (scheda AI, o `/controlla`): l'AI rilegge quello che hai scritto e segnala errori (ortografia, dati, calcoli, ragionamento) e punti poco chiari, con la correzione. Conversazioni salvate in `chats/` con progetti, ricerca, fissate, rinomina. **Eliminare**: cestino accanto alla chat (al passaggio del mouse), tasto destro → Elimina, `/elimina`, oppure il cestino in alto nell'elenco per selezionarne più insieme («Tutte», «Vuote»); vanno in `chats/cestino` e per qualche secondo c'è «Annulla». **Comandi `/`**: `/nuova`, `/elimina`, `/compatta`, `/modello`, `/web`, `/pagina` (allega il foglio), `/trascrivi`, `/appunto`, `/esporta`, `/dirama`, `/impostazioni`. **Documenti nella chat**: con un documento aperto (PDF, Word…), scheda AI → «Discuti il documento» (o `/pagina` nella chat): va il testo di tutte le pagine (estratto da PdfCraft; fino a ~100 mila caratteri) più l'immagine della pagina aperta con quello che ci hai scritto; i PDF scansionati mandano solo l'immagine. Se l'agente scelto non legge le immagini (un modello solo testo), il foglio lo legge prima l'agente scelto in «Lettura dei fogli (OCR)» (riga «Leggo il foglio con … · letto in N s» nella chat) e gli arriva come testo; la lettura si ricorda, la stessa pagina non si rilegge. Il **ragionamento** del modello in una riga che si apre con un clic anche mentre il modello sta ancora pensando (lo si legge in diretta), gli **strumenti** in righe compatte (ricerca, pagine, partita IVA, appunto), per ogni risposta token, velocità, attesa e durata. Rigenera, Modifica, **Dirama** (continua in una chat nuova), **Nell'appunto**. Messaggi scritti mentre risponde = **in coda**. Scelta dell'agente e cerchietto del contesto nel riquadro di scrittura (clic: dettaglio e «Compatta adesso») |
| L'AI scrive nell'appunto | Con un appunto aperto la chat lo vede (chip 📝 nel riquadro di scrittura, clic per escluderlo): «riordina l'appunto», «aggiungi un riassunto in fondo», «trasforma la lista in una tabella» → l'agente lo legge e lo cambia (aggiunge in fondo, sostituisce un pezzo o lo riscrive), e lo vedi subito sulla pagina. Con «Ricerca web» cerca in rete e legge pagine (API DeepSeek) e verifica partite IVA (VIES), citando le fonti |
| Contesto della chat | Sotto la chat una barra colorata mostra quanto contesto è usato (istruzioni, pagina allegata, riassunti, conversazione, messaggio che stai scrivendo; passa il mouse per il dettaglio) e il numero «≈ 12K / 128K token · 9%» (esatto quando il server lo dice). La linea rossa è la soglia di compattazione |
| Compattazione | Prima che il contesto si riempia (di serie all'80%) la parte vecchia della conversazione diventa un riassunto (argomento, risultati, decisioni, domande aperte) e gli ultimi messaggi restano interi; «Compatta» lo fa a mano. Il riassunto resta leggibile nella chat |
| Impostazioni AI | Pulsante con l'ingranaggio (o «Impostazioni AI» nella chat): agenti in ordine con le frecce, indirizzo API in formato OpenAI, modello («Leggi modelli» li elenca con il loro contesto), **contesto massimo**, chiave (da DSH, Portachiavi, variabile d'ambiente, o incollata: va nel Portachiavi), «Prova». Anche la soglia di compattazione e la ricerca web. Si salva in `ai.toml` (copia della versione precedente in `ai.toml.bak`) e vale subito |
| Agenti AI con riserva | In `ai.toml`: prima i modelli locali (LM Studio, Ollama), poi DeepSeek, Google Gemini, Anthropic Claude. Se uno non risponde si passa al successivo; la chat dice chi ha risposto e chi è stato saltato. Vedi sotto |
| Grafici e modifiche nei documenti | Chiedi alla chat un grafico («disegna le due rette e la soluzione»): lo disegna in stile LaTeX con [Typst](https://typst.app) e [Lilaq](https://lilaq.org) (funzioni, punti, rette, barre, curve parametriche) e lo mostra nella risposta. Con un documento allegato alla chat lo può anche leggere e **modificare**, e metterci il grafico, coi server MCP della suite Craft che girano dentro l'app: WordCraft per i Word, PdfCraft per i PDF, VectorCraft per gli SVG, PhotoCraft per le foto (ritaglia, ridimensiona, ruota, luminosità, livelli, curve, bianco e nero, nitidezza…; la foto resta nel suo formato), e strumenti sul testo per i Markdown. Ogni modifica si vede subito nel documento accanto e si annulla dalla chat con un clic; il file originale sul disco non si tocca. Sugli appunti delle trascrizioni il grafico va al posto della figura disegnata a mano; sul foglio della lavagna (una pagina dello Slate) va al posto del disegno a mano, che OpenSlate trova da solo (assi ed etichette), o sotto lo scritto. Un SVG scritto dal modello nella risposta si vede disegnato |
| Compilare i moduli PDF | Con un modulo PDF allegato alla chat, chiedile «compila questo modulo con i miei dati»: legge i campi e li compila con PdfCraft. Un modulo stampato o scansionato senza campi li riceve prima (PdfCraft trova gli spazi da riempire e li chiama come le etichette accanto), oppure la chat scrive testo, crocette e data dove servono. Usa solo quello che le dici e chiede quello che manca. Non firma mai: la firma la metti tu con la penna, e i valori compilati restano nella copia firmata |
| Parlare alla chat | Clic sul microfono accanto a Invia, oppure tieni premuto il tasto Alt (Opzione) di destra mentre parli: le parole finiscono nel riquadro del messaggio, da correggere e mandare. Le trascrive Whisper sul tuo computer con [whisper.cpp](https://github.com/ggml-org/whisper.cpp) (sui Mac sulla scheda grafica), senza rete: la voce non esce dal computer. Il modello (547 MB) si scarica una volta sola, la prima, se sei d'accordo; in `ai.toml` (`[voice]`) si sceglie un modello più leggero o un servizio in rete (OpenAI, Groq, Mistral…), e `auto_send = true` manda il messaggio appena finisci di parlare. Scrive nella lingua che parli, qualunque sia la lingua dell'app (`language` in `[voice]` ne fissa una). Esc annulla. La prima volta macOS chiede il permesso per il microfono |
| Il tutor che parla | Sotto ogni risposta **Ascolta** la legge ad alta voce (Esc la ferma); se la domanda l'hai dettata, la risposta torna a voce da sola. Usa le voci del sistema, senza rete: `say` su macOS (le voci «Premium» o «Migliorata» sono le più naturali: Impostazioni di Sistema → Accessibilità → Contenuto letto), le voci di Windows, `espeak-ng` su Linux; oppure un servizio in rete che parla `/audio/speech` (OpenAI…), in `ai.toml` (`[speech]`). Legge nella lingua della risposta |
| Trasforma in bella | Home → «Trasforma in bella»: l'AI legge quello che hai scritto sulla lavagna, formule comprese, e al suo posto lo riscrive con una scrittura a mano chiara e la matematica impaginata come sul quaderno ordinato (frazioni in colonna, esponenti, radici, graffe dei sistemi, sommatorie, integrali, limiti). È inchiostro vero, come quello della penna: Annulla riporta la tua scrittura. Disegni, evidenziatori, segni dell'AI e righe che non si leggono restano come sono. Usa gli agenti che leggono i fogli (`[ocr]`). Nei disegni, senza AI, rette, frecce (anche con la punta fatta a parte), cerchi ed ellissi, archi e poligoni diventano perfetti: le linee quasi orizzontali o verticali si raddrizzano, i capi vicini si uniscono, le curve libere si lisciano; quello che non si riconosce resta com'è |
| La chat scrive sulla lavagna | Chiedile «controlla i conti e segna gli errori», «sottolinea le scadenze», «scrivi il risultato accanto alla seconda riga»: legge il foglio riga per riga e parola per parola, poi evidenzia, sottolinea, cerchia, barra, fa riquadri, frecce, spunte e croci, e scrive a mano (stampatello o corsivo, coi caratteri di [Hershey](https://it.wikipedia.org/wiki/Hershey_fonts) a filo singolo: tratti veri). Se glielo chiedi sposta, ingrandisce, ricolora o cancella quello che c'è («il titolo più grande e blu», «sposta l'ultima riga più giù», «togli i miei evidenziatori»); da sola aggiunge soltanto, e per cancellare buona parte del foglio aspetta il tuo sì nella chat. Quello che fa è inchiostro come il tuo (si salva, si esporta, si cancella) e si toglie tutto con un clic nella chat |
| La chat conosce l'archivio | «Trova la pagina dove ho scritto del bilancio di ottobre», «apri sulla lavagna gli appunti di ieri», «salva la lavagna come Riunione e dammi una pagina nuova», «esporta in un PDF le pagine di questa settimana»: cerca per titolo, data e testo delle trascrizioni, guarda una pagina, la apre sulla lavagna, salva la lavagna (aggiornando la pagina da cui viene), rinomina, esporta nella cartella dove esporti di solito (senza mai sovrascrivere), sposta nel cestino e riprende. Per il cestino aspetta sempre il tuo sì nella chat |
| Server MCP | Altri server MCP (stdio) in `ai.toml`, come in Claude Desktop o DSH: `[[mcp]]` con `name`, `command`, `args`, `env` (valori come le chiavi: `env:`, `keychain:`, `dsh:`), `enabled`, `timeout`. I loro strumenti arrivano alla chat come `nome__strumento` |
| Diretta web | «Diretta web»: link da aprire nel browser della TV, del telefono o di un PC della rete di casa. Si aggiorna in tempo reale. La lavagna in diretta la vede la rete di casa; chat, appunti e conversazioni chiedono la chiave che sta nel link mostrato da OpenSlate (`?k=…`, poi il browser la ricorda). Il server risponde solo a indirizzi IP, `localhost` e nomi `.local` e rifiuta le scritture che arrivano da altri siti: una pagina web aperta nel browser non può comandare l'app |
| Solo il foglio | Il pulsante accanto a Copia/Incolla nella barra del titolo (o `Cmd+Shift+F`, Vista → Solo il foglio) toglie la barra dei menu e gli strumenti; un altro clic li rimette |
| Integrazione | Pagina attuale in `http://<mac>:8080/page.pdf`, `/page.svg`, `/page.png`, `/state.json`; chat su `/chat`. Il comando `hook` parte dopo ogni file prodotto |

## Avvio

**Versioni pronte** (macOS, Linux, Windows sperimentale): pagina
[Releases](https://github.com/rmpaolillo/OpenSlate/releases) — istruzioni nel [README inglese](README.md#install).

Su macOS le release fatte con un **Developer ID** di Apple sono firmate e notarizzate e si aprono
senza avvisi. Servono cinque secret nel repository (Settings → Secrets and variables → Actions):
`APPLE_CERTIFICATE` (il certificato «Developer ID Application» esportato in `.p12`, in base64),
`APPLE_CERTIFICATE_PASSWORD`, `APPLE_ID`, `APPLE_PASSWORD` (password specifica per app) e
`APPLE_TEAM_ID`; dettagli nel [README inglese](README.md#signing-and-notarization-macos). Senza,
l'app è firmata «ad hoc» e la prima volta va aperta da Impostazioni di sistema → Privacy e
sicurezza → **Apri comunque**; gli aggiornamenti successivi, scaricati dall'app, no.

Per compilare serve **Rust 1.95 o più recente** (i motori di WordCraft e PdfCraft lo chiedono): `rustup update`.
Serve anche **CMake**, che compila whisper.cpp per la voce nella chat: `brew install cmake` su macOS,
`sudo apt install cmake` su Linux (su Windows c'è già con Visual Studio).
La prima compilazione è più lunga del solito; dalla seconda torna veloce.

Il video MP4 usa il codificatore del sistema: AVFoundation su macOS, Media Foundation su Windows
(nelle edizioni «N» serve il Media Feature Pack di Microsoft). Su Linux usa `ffmpeg` se c'è
(`sudo apt install ffmpeg`); altrimenti la prima volta chiede prima di scaricare dal sito di Cisco il
loro codificatore gratuito OpenH264 (circa 1 MB).
Per leggere le risposte a voce su Linux usa `espeak-ng` (`sudo apt install espeak-ng`) o speech-dispatcher; macOS e Windows usano le loro voci.

```bash
cargo run --release                 # interfaccia grafica
cargo run --release -- gui --web    # interfaccia + diretta web subito attiva
cargo run --release -- gui --demo   # prova senza pad (disegna da solo, zig-zag compreso)
```

### Connessione

- **Prima volta**, o dopo aver usato Inkspace: tieni premuto il pulsante del pad ~6 s, finché
  il LED lampeggia blu. Poi «Collega» e, quando richiesto, premi una volta il pulsante.
- **Le altre volte** (LED verde): premi brevemente il pulsante del pad, poi «Collega».

Il pad ricorda una sola app alla volta: usare Inkspace richiede di registrare di nuovo questa.

### Perché l'archivio è sul Mac

Il pad non lascia sfogliare le pagine al suo interno: consegna solo la più vecchia, e per avere
la successiva bisogna toglierla. Per questo, come Inkspace, le pagine si scaricano e si gestiscono
sul computer. Ogni pagina è un file `library/<data>.json` leggibile (tratti, colori, titolo).

## Agenti AI (`ai.toml`)

Si crea da solo al primo uso, con i commenti. Si rilegge a ogni richiesta: le modifiche valgono
subito. Tutti i provider usano l'API **in formato OpenAI** (`/chat/completions`), che parlano
anche LM Studio, MTPLX, Ollama, DeepSeek, Gemini e Claude. Vengono provati **in ordine**:

1. **LM Studio** — `http://localhost:1234/v1`, sul tuo computer o su un altro della rete di
   casa (cambia l'indirizzo). Con `model = ""` sceglie da solo il modello più grande fra quelli
   caricati (per le immagini uno con «vl», «vision», «ocr» nel nome); quelli sotto i 3B non li usa.
2. **Ollama** — `http://localhost:11434/v1`, come sopra.
3. **DeepSeek** — API ufficiale, `deepseek-flash`, che legge anche le immagini ([guida](https://api-docs.deepseek.com/guides/vision/)).
4. **Google Gemini** — `gemini-3.8-flash`.
5. **Anthropic Claude** — `claude-sonnet-4-6` (si può mettere `claude-opus-5-5`).

**Lettura dei fogli (OCR)**: sezione `[ocr]` di `ai.toml`, o «Impostazioni AI → Lettura dei fogli». Sceglie chi legge la scrittura a mano in «Trascrivi» e in chat per gli agenti che non vedono le immagini (modelli solo testo): `agent = ""` è il primo della lista che vede le immagini, altrimenti il nome di un agente (meglio uno veloce, es. Google Gemini); `fallback = false` usa sempre e solo quello. La lettura aspetta al massimo 90 s per agente.

**Chi ha risposto.** Ogni risposta della chat mostra l'agente e il modello che l'hanno scritta davvero (gli agenti si provano in ordine, quindi può non essere il primo). La chat dice anche al modello chi è, servizio, indirizzo e modello, perché spesso i modelli non lo sanno e possono dire di essere un altro: vale l'etichetta sopra la risposta.

**Voce** (sezione `[voice]`): `engine = "whisper"` (di serie, sul computer) oppure il nome di un agente che parla `/audio/transcriptions` (OpenAI, Groq, Mistral…; con `enabled = false`, così la chat non lo usa); `model` (`large-v3-turbo`, `small`, `base`, o il modello di trascrizione del servizio); `language` (vuota = la lingua che parli); `auto_send = true` manda il messaggio appena finisci di parlare.

**Voce delle risposte** (sezione `[speech]`): `engine = "system"` (di serie: le voci del sistema) oppure il nome di un agente che parla `/audio/speech` (OpenAI…); `model` per quell'agente (es. `gpt-4o-mini-tts`); `voice` (vuota = la migliore per la lingua, oppure un nome come `"Alice"`); `language` (vuota = quella della risposta); `rate` (da 0.5 a 2); `auto = "voice"` legge da sola se hai dettato la domanda, `"always"` o `"never"` (resta il pulsante Ascolta).

Tutto si cambia dalla pagina «Impostazioni AI»; a mano: l'ordine è quello dei blocchi `[[provider]]`,
`enabled = false` ne esclude uno, `context` è il contesto massimo in token.

**Chiavi API: mai in chiaro.** Nel file c'è solo *dove* trovarle. Quelle già date a
DeepSeek Harness si riusano con `dsh:NOME` (lette da `~/.dsh/.credentials.yaml`). Per le altre:

```bash
# Portachiavi di macOS (consigliato: vale anche aprendo l'app dal Finder)
security add-generic-password -a "$USER" -s DEEPSEEK_API_KEY  -w 'sk-...'
security add-generic-password -a "$USER" -s GEMINI_API_KEY    -w '...'
security add-generic-password -a "$USER" -s ANTHROPIC_API_KEY -w 'sk-ant-...'
# oppure variabili d'ambiente (es. in ~/.zshrc)
export DEEPSEEK_API_KEY=sk-...
```

Un provider senza chiave viene saltato. Per controllare:

```bash
cargo run --release -- ai            # stato di ogni agente
cargo run --release -- ai --prova    # una domanda breve a ciascuno, con i tempi
cargo run --release -- trascrivi foto-appunti.jpg   # prova della trascrizione da riga di comando
```

La chat usa le chiavi del Mac: risponde solo a chi è nella rete di casa. Chat, appunti e
impostazioni AI stanno dentro la finestra dell'app (WebView di sistema, servite su 127.0.0.1:
non si vedono dalla rete); le formule le disegna KaTeX, incluso nel programma. La ricerca web costa 0,75 $ ogni 1000 ricerche sul conto DeepSeek (sezione `[web]`
di `ai.toml`).

## Impostazioni (`settings.conf`)

Si crea da solo. Si modifica a mano ad app chiusa.

```ini
rot = 1                  # rotazione del foglio (0-3), anche col pulsante Ruota
pen_color = #141414
pen_width = 2
zigzag_auto = 0          # zig-zag che cancella anche senza «Gomma» (sconsigliato con le firme)
highlight_color = #ffeb5a   # evidenziatore (chiaro: sta sotto l'inchiostro)
highlight_width = 16
longpress_tool = 0       # tocco lungo col pennino (1,5 s) = penna/evidenziatore; spento di serie
web_port = 8080
web_auto = 0             # 1 = diretta web attiva all'avvio
ocr_engine = ai          # ai (agenti di ai.toml: testo in Markdown) | vision (Apple, offline, solo testo)
ocr_langs = it-IT,en-US  # lingue per Apple Vision
md_app = Typora          # app per «Apri in Typora»
sync_delete = 1          # 0 = lascia i disegni sul pad (ne scarica solo il più vecchio)
copy_crop = 1            # Copia immagine: 1 = ritaglia attorno alla scrittura
copy_size = 1600         # Copia immagine: pixel del lato lungo
thumb_size = 330         # Archivio: altezza anteprime (anche col cursore nella barra)
hook =                   # es: ~/bin/send-to-crm.sh   (riceve il file come $1)
ui_theme = sistema       # sistema | chiaro | scuro
ui_lang = auto           # auto (lingua del sistema) | it | en | es | fr | de | uk | zh | ja | ko
chat_zoom = 1.00         # grandezza del testo della chat AI (pizzico, Cmd +/−/0)
update_check = 1         # avvisa quando su GitHub c'è una versione nuova
update_skip =            # versione saltata («Salta questa versione»)
update_auto = 1          # installa da solo gli aggiornamenti (si usano dal prossimo avvio)
rulers = 1               # righelli in cm attorno al foglio
chat_side = sinistra     # chat AI: sinistra | destra
chat_width = 0.55        # larghezza della chat (frazione della finestra)
paper = bianco           # bianco | righe | righe10 | elem1 | elem2 | elem3 | quadretti5 | quadretti5m | quadretti4
                         # quadretti10 | millimetrata | cartesiano | puntinato | isometrica | pentagramma | cornell
paper_margin = 1         # margini rossi a sinistra e a destra (solo fogli a righe)
save_dir =               # cartella proposta da «Salva con nome» (l'ultima scelta; vuoto = exports/)
```

Le vecchie chiavi `ocr_backend`, `ocr_url`, `ocr_model`, `ocr_key` non servono più (i modelli
stanno in `ai.toml`) e vengono ignorate.

L'OCR con Apple Vision e il video MP4 compilano un piccolo programma Swift al primo uso (nelle
release è già dentro l'app). Servono gli strumenti di Apple (`xcode-select --install`).

## Riga di comando

```bash
cargo run --release -- scan            # dispositivi visibili e modalità del pad
cargo run --release -- info            # nome, firmware, batteria, dimensioni
cargo run --release -- register        # registrazione (LED blu)
cargo run --release -- live [--raw]    # punti della penna nel terminale
cargo run --release -- sync [--keep]   # scarica i disegni in exports/
cargo run --release -- pdf exports/pad-raw/*.bin   # riconverte copie grezze in PDF
cargo run --release -- ai [--prova]    # agenti AI configurati
cargo run --release -- trascrivi foto.png   # immagine → testo in Markdown
```

Non serve installare altro: ogni file si apre, si vede e si salva con le librerie della suite
Craft di ArtCraft, senza le loro interfacce e senza programmi esterni:

| File | Libreria |
|---|---|
| Word (.docx, .doc, .odt, .rtf), Markdown, .txt, .html | [WordCraft](https://github.com/storytold/wordcraft): apre e impagina; salva il Word con l'inchiostro |
| PDF | [PdfCraft](https://github.com/storytold/pdfcraft): pagine, anteprime e testo; la sua libreria PDF salva la copia firmata |
| SVG | [VectorCraft](https://github.com/storytold/vectorcraft) |
| Foto (PNG, JPEG, WebP, TIFF, BMP, GIF) | [PhotoCraft](https://github.com/storytold/photocraft) |
| Export di lavagna e archivio (PDF, SVG, PNG, JPEG, WebP) | li disegna e scrive VectorCraft; PNG, JPEG e WebP li scrive PhotoCraft |
| GIF e SVG animati, Xournal++, LaTeX, Typst, il testo nascosto dei PDF ricercabili | li scrive OpenSlate stesso (`src/formats.rs`, `src/mdconv.rs`) |
| Video MP4 | il codificatore del sistema: AVFoundation (macOS), Media Foundation (Windows); su Linux ffmpeg, oppure OpenH264 di Cisco scaricato al primo uso (*OpenH264 Video Codec provided by Cisco Systems, Inc.*) |

Un Markdown aperto come documento ha l'aspetto degli Appunti: testo in Source Serif, titoli in Inter, tabelle leggere. Le formule (`$…$`, `$$…$$`, `\(…\)`, `\[…\]`) le disegna [Typst](https://typst.app) in stile LaTeX (il LaTeX lo converte [MiTeX](https://github.com/mitex-rs/mitex)) e le figure accanto (`<img src="…">`, `![…](…)`, gli SVG disegnati da VectorCraft) diventano immagini.
Un Markdown, .txt o .html firmato si salva in PDF (quei formati non possono tenere l'inchiostro).

Diagnosi: tutti i comandi scambiati col pad finiscono in `captures/gui.log` (o `cli.log`).

## Dove stanno i dati

Avviata con `cargo run` dalla cartella del progetto, l'app lavora lì. Installata, usa la cartella
dei dati dell'utente: macOS `~/Library/Application Support/OpenSlate`, Linux
`~/.local/share/openslate`, Windows `%APPDATA%\OpenSlate` (si cambia con `OPENSLATE_DATA`).
Lì ci sono anche `crash.log` (se l'app è andata in crash), `web.key` (la chiave del link della
diretta web), `updates/` (la versione precedente dell'app, tenuta dopo un aggiornamento) e, su
Linux, `codecs/` (il codificatore OpenH264 di Cisco, se ne hai permesso lo scaricamento). In
`models/whisper/` c'è il modello di Whisper per la voce nella chat, una volta scaricato.

## Struttura

```
src/protocol.rs   opcode e formato dei messaggi (Tuhi)
src/ble.rs        Bluetooth (btleplug): ricerca, connessione, invio/ricezione
src/device.rs     sequenze: registrazione, live, scaricamento
src/live.rs       decodifica dei punti in tempo reale
src/offline.rs    decodifica dei disegni salvati nel pad
src/stroke.rs     tratti e riconoscimento dello zig-zag
src/scene.rs      lavagna condivisa: vista, penna, annulla/ripeti
src/actor.rs      collegamento col pad per l'interfaccia
src/library.rs    archivio delle pagine (library/*.json)
src/ui.rs         interfaccia (egui): lavagna, azioni
src/ui/theme.rs   colori chiaro/scuro, carattere Inter (da WordCraft)
src/ui/widgets.rs pulsanti grandi/piccoli e gruppi della barra (da WordCraft)
src/ui/chrome.rs  barra del titolo, schede, barra di stato, righelli, «Cerca comandi»
src/ui/ribbon.rs  contenuto delle schede (Home, Documento, Visualizza, AI)
src/ui/backstage.rs pagina File (Nuovo, Apri, Salva, Esporta, Condividi, Informazioni)
src/ui/options.rs File › Opzioni, scorciatoie, finestre di aiuto
src/ui/library_view.rs  interfaccia: archivio con anteprime
src/ui/quit.rs    uscita: «Salvare la lavagna?», salvataggio automatico all'uscita, attesa dell'appunto
src/ui/savedlg.rs finestre «Salva con nome» (ultima cartella scelta)
src/ui/zoom.rs    zoom e gesti del trackpad
src/export.rs     PDF / SVG / PNG
src/craftexport.rs export con VectorCraft (SVG, PDF) e PhotoCraft (PNG, JPEG, WebP)
src/formats.rs    i formati d'uscita delle pagine: PDF ricercabile (testo invisibile), Word con l'inchiostro, Xournal++, GIF e SVG animati
src/mdconv.rs     gli appunti in LaTeX e Typst (e in testo semplice per i PDF ricercabili)
src/video.rs      il video MP4 della pagina che si riscrive: AVFoundation (macOS, con src/swift/video_mp4.swift), Media Foundation (Windows), ffmpeg o OpenH264 di Cisco (Linux)
src/voice.rs      la voce nella chat: il microfono (cpal) e chi trascrive, Whisper sul computer (whisper.cpp) o un servizio in rete (src/ui/voice_ui.rs: il pulsante e il tasto)
src/speech.rs     le risposte lette ad alta voce: voci del sistema (say, SAPI, espeak-ng) o un servizio in rete
src/inbella.rs    «Trasforma in bella»: l'AI legge le righe e le riscrive al loro posto, con la matematica impaginata da src/mathhand.rs
src/shapes.rs     i disegni in bella: rette, frecce, cerchi, archi e poligoni rifatti perfetti
src/ui/sysdlg.rs  le finestre del sistema (rfd; su Linux GTK nel thread della finestra, come la chat)
src/figures.rs    figure della trascrizione ridisegnate dai tratti veri della penna
src/update.rs     controllo degli aggiornamenti (ultima release su GitHub)
src/selfupdate.rs aggiornamento dentro l'app: scarica, controlla le impronte, installa, riavvia
src/mdcraft.rs    Markdown per WordCraft con l'aspetto degli Appunti; figure → immagini; scrittura a mano
src/docxfonts.rs  i caratteri a mano dentro il Word esportato (packaging/hand_fonts.py li fa da src/handwrite.rs)
src/mathtex.rs    formule LaTeX disegnate da Typst (via MiTeX)
src/typstworld.rs Typst dentro l'app: font e pacchetti inclusi (src/typst-pkg/: Lilaq e dipendenze, definizioni di MiTeX)
src/charts.rs     strumento draw_chart della chat: grafici con Typst + Lilaq (SVG e PNG in charts/)
src/docagent.rs   la chat nel documento aperto: server MCP di WordCraft, PdfCraft, VectorCraft in memoria; Annulla
src/boardagent.rs la chat sul foglio della lavagna: leggere, annotare, grafico al posto del disegno a mano, Annulla
src/boardread.rs  il foglio letto a righe e parole (coi nomi r2p3) e l'immagine coi nomi per il modello
src/boardmarks.rs i segni della chat: evidenzia, sottolinea, cerchia, barra, riquadri, frecce, spunte, croci, testo
                  (spostare, ridimensionare, ricolorare e cancellare sono in boardagent.rs, con la conferma)
src/handwrite.rs  testo a mano coi caratteri di Hershey a filo singolo (src/handwrite/*.jhf)
src/libagent.rs   la chat sull'archivio: cerca (anche nelle trascrizioni), guarda, apri, salva, rinomina, esporta, cestino
src/ui/chat_lib.rs le richieste della chat all'archivio, fatte dall'interfaccia
src/mcpclient.rs  server MCP esterni da ai.toml ([[mcp]], trasporto stdio)
vendor/           copie di librerie di Typst e WordCraft con piccole modifiche (vendor/README.md)
src/rasign.rs     foto (PNG, JPEG…): pagine e firma con le librerie di PhotoCraft
src/vecsign.rs    SVG: pagine e firma col motore di VectorCraft
src/i18n.rs       lingue: il testo italiano è la chiave, tabelle in src/i18n/<codice>.json
                  (l'inglese fa da riserva), src/web/i18n.js le applica alle pagine web
src/web.rs        diretta web (HTTP + Server-Sent Events), pagine in src/web/ (index.html, chat.html, appunti.html, impostazioni.html);
                  controlli di sicurezza (nome del server, origine, chiave della diretta, CSP), appunti con versione
src/ocr.rs        trascrizione (agenti AI oppure Apple Vision)
src/ai.rs         agenti AI: ai.toml, catena di riserva, streaming, strumenti, contesto della chat
src/chats.rs      conversazioni della chat (chats/*.json e pagina allegata)
src/aiconfig.rs   pagina «Impostazioni AI» (ai.toml in JSON, prova, modelli, compattazione)
src/tools.rs      strumenti della chat: ricerca web e pagine (DeepSeek), verifica partita IVA (VIES)
src/viewer.rs     finestra separata con una pagina web (riserva se le pagine incorporate non partono)
src/ui/embed.rs   pagine web dentro la finestra (WebView): appunto, chat, impostazioni AI
src/ui/workspace.rs appunto al centro, chat a lato, barra «Slate non collegato», scheda Appunto
src/ui/paper.rs   rigature del foglio (righe, elementari, quadretti, millimetrata, cartesiano, …)
src/vendor.rs     KaTeX, marked e DOMPurify inclusi (src/web/vendor)
src/swift.rs      helper Swift per macOS (src/swift/vision_ocr.swift: OCR con Apple Vision)
src/docimport.rs  documenti → PDF → immagini delle pagine
src/wordsign.rs   documenti Word: pagine e salvataggio firmato col motore di WordCraft
src/docrender.rs  pagine nitide a ogni zoom (WordCraft per i Word, PdfCraft per i PDF)
src/document.rs   documenti aperti: pagine, tratti, PDF firmato
src/pdfdoc.rs     PDF: misure delle pagine e tratti sovrapposti (lopdf)
src/signature.rs  firma registrata
src/ui/doc_view.rs  interfaccia: documenti e firma
src/settings.rs   settings.conf e hook
src/platform.rs   differenze fra sistemi: cartella dei dati, apertura dei file, paracadute dei lavori in sottofondo
src/fonts.rs      i caratteri dentro il programma, una copia sola ciascuno (Source Serif 4 è quello di WordCraft)
src/demo.rs       le pagine di esempio delle immagini del README (solo nelle prove)
packaging/collect_licenses.py  testi completi delle licenze per i pacchetti (THIRD-PARTY-LICENSES.txt)
```

Test: `cargo test`.

## Crediti

- Protocollo Bluetooth: traduzione di [Tuhi](https://github.com/tuhiproject/tuhi) (GPL).
- Interfaccia in stile Word: ripresa da [WordCraft](https://github.com/storytold/wordcraft)
  (© ArtCraft Team e contributori, MIT OR Apache-2.0); vedi `NOTICE`.
- La suite Craft (© ArtCraft Team e contributori, MIT OR Apache-2.0), usata come librerie a una
  versione fissa (vedi `Cargo.toml`), senza le loro interfacce:
  - [WordCraft](https://github.com/storytold/wordcraft): apre, impagina e salva i Word (e i Markdown) con l'inchiostro;
    la sua impaginazione è copiata in `vendor/` con una piccola modifica;
  - [PdfCraft](https://github.com/storytold/pdfcraft): pagine PDF nitide a ogni zoom, testo, copia firmata;
  - [VectorCraft](https://github.com/storytold/vectorcraft): SVG con l'inchiostro ed export della lavagna e dell'archivio (PDF, SVG, PNG);
  - [PhotoCraft](https://github.com/storytold/photocraft): foto (PNG, JPEG, WebP, TIFF…) con profilo colore e misure, e l'inchiostro dipinto sopra;
  - i loro **server MCP** (`wordcraft-mcp`, `pdfcraft-automation`, `vectorcraft-mcp`, `photocraft-automation`), dentro l'app: la chat AI legge e modifica il documento Word, PDF, SVG o la foto accanto;
  - librerie corrette di [craft-libs](https://github.com/storytold/craft-libs);
  - [craft-launcher](https://github.com/storytold/craft-launcher): l'idea degli aggiornamenti dentro l'app e della firma per macOS (solo le idee, nessun codice copiato).
- [Typst](https://github.com/typst/typst) (Apache-2.0): formule LaTeX e grafici della chat; due suoi crate sono copiati in `vendor/` con una riga cambiata.
  [MiTeX](https://github.com/mitex-rs/mitex) (Apache-2.0) converte il LaTeX in Typst.
- [Lilaq](https://github.com/lilaq-project/lilaq) (Mc-Zen, MIT) e i pacchetti Typst che usa ([zero](https://github.com/Mc-Zen/zero),
  [tiptoe](https://github.com/Mc-Zen/tiptoe), [elembic](https://github.com/PgBiel/elembic), [suiji](https://github.com/liuguangxi/suiji)): i grafici della chat AI.
- Carattere [Inter](https://github.com/rsms/inter) (SIL OFL 1.1, `assets/fonts/OFL-Inter.txt`),
  senza i segni dell'area privata Unicode, dove stanno le icone. Icone [Phosphor](https://phosphoricons.com) (MIT).
- Carattere [Source Serif 4](https://github.com/adobe-fonts/source-serif) per il testo degli appunti
  (SIL OFL 1.1, `assets/fonts/OFL-SourceSerif4.txt`).
- Chat ispirata a [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness) (solo le idee: nessun codice copiato).
- Formule [KaTeX](https://katex.org) (MIT), Markdown [marked](https://marked.js.org) (MIT),
  HTML ripulito da [DOMPurify](https://github.com/cure53/DOMPurify) (Apache-2.0 OR MPL-2.0).
- [Caratteri di Hershey](https://it.wikipedia.org/wiki/Hershey_fonts) (Dr. A. V. Hershey, U.S. National Bureau of Standards; formato `.jhf` di James Hurt, Cognition Inc.; file da [hershey-fonts](https://github.com/kamalmostafa/hershey-fonts) di Kamal Mostafa): le lettere a filo singolo con cui la chat scrive sulla lavagna (`src/handwrite/`, licenza libera con citazione), e i caratteri **OpenSlate Corsivo** e **OpenSlate Stampatello** degli appunti a mano (`assets/fonts/OpenSlate*.ttf`).
- Sottoinsieme di [Noto Sans CJK](https://github.com/notofonts/noto-cjk) (SIL OFL 1.1) per i nomi delle lingue nel menu.
