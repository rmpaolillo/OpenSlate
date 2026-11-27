import Foundation
import PDFKit
import SwiftUI
import UIKit

/**
 La lavagna: il modello (strumenti, gomma, annulla/ripeti) è nel cuore Rust
 (`BoardModel`, la logica di `scene.rs` della desktop); qui la copia da
 disegnare, aggiornata coi cambiamenti, e la pagina da cui viene.
 */
final class Board: ObservableObject {
    let model = BoardModel()
    private(set) var strokes: [PageStroke] = []
    var id = ""
    var title = ""
    var created = ""
    private var savedRev: UInt64 = 0
    /// Cambia a ogni modifica: la lavagna si ridisegna.
    @Published private(set) var version = 0

    init() { savedRev = model.rev() }

    var isEmpty: Bool { model.isEmpty() }
    var dirty: Bool { model.rev() != savedRev && !model.isEmpty() }
    func markSaved() { savedRev = model.rev() }

    /// Dov'è la penna dello Slate (anche in aria, vicina al pad): il pallino sul
    /// foglio. Va dritto al foglio, senza ridisegnare la lavagna (arriva spesso).
    private(set) var tip: PenPoint? { didSet { tipView?.tip = tip } }
    weak var tipView: SheetUIView? { didSet { tipView?.tip = tip } }
    /// Il pad spento o fuori portata non dice che la penna si è allontanata.
    private var tipStale: DispatchWorkItem?

    func slateTip(_ points: [PenPoint]) {
        guard let p = points.last else { return }
        tip = p.phase == .away ? nil : p
        tipStale?.cancel()
        tipStale = nil
        guard tip != nil else { return }
        let w = DispatchWorkItem { [weak self] in self?.tip = nil }
        tipStale = w
        DispatchQueue.main.asyncAfter(deadline: .now() + 3, execute: w)
    }

    func pen(_ points: [PenPoint]) {
        let ops = model.pen(points: points)
        // la penna in aria non cambia il foglio: non lo si ridisegna
        if ops.isEmpty { return }
        for op in ops {
            switch op {
            case .down(let color, let width, let highlight, let point):
                strokes.append(PageStroke(color: color, width: width, highlight: highlight, points: [point], stamp: nil))
            case .move(let point), .up(let point):
                if !strokes.isEmpty { strokes[strokes.count - 1].points.append(point) }
            case .reset:
                reload()
                return
            }
        }
        version += 1
    }

    func reload() {
        strokes = model.strokes()
        version += 1
    }

    func undo() { if model.undo() { reload() } }
    func redo() { if model.redo() { reload() } }
    func clear() { model.clear(); reload() }

    func newPage() {
        model.reset()
        id = ""; title = ""; created = ""
        markSaved()
        reload()
    }

    func load(_ p: Page) {
        model.open(strokes: p.strokes)
        id = p.id; title = p.title; created = p.created
        markSaved()
        reload()
    }

    /// Tratti di una pagina di un documento, senza cronologia.
    func openStrokes(_ s: [PageStroke]) {
        model.open(strokes: s)
        markSaved()
        reload()
    }

    func erase(_ a: CGPoint, _ b: CGPoint, tol: Float, pageH: Float) {
        if model.fingerErase(ax: Float(a.x), ay: Float(a.y), bx: Float(b.x), by: Float(b.y), tol: tol, pageH: pageH) > 0 { reload() }
    }

    func eraseEnd() {
        _ = model.fingerEraseEnd()
        version += 1
    }

    func toPage(_ w: Float, _ h: Float) -> Page {
        Page(id: id, title: title, created: created.isEmpty ? nowCreated() : created, source: "lavagna", raw: nil, width: w, height: h, strokes: model.strokes())
    }
}

/// Un documento da firmare (PDF, Word, foto, SVG), come `DocMode.kt`: nel
/// formato della desktop (`src/docstore.rs`), così si sincronizza col
/// computer. `documents/<id>/` con `doc.json`, `source.pdf` e il file di partenza.
final class OpenDoc: ObservableObject {
    let dir: URL
    let title: String
    let original: URL
    let sizes: [PdfPageSize]
    @Published private(set) var index = 0
    @Published private(set) var image: UIImage?
    private var strokes: [[PageStroke]]
    private var pdfDoc: PDFDocument?

    var id: String { dir.lastPathComponent }
    var pdf: URL { dir.appendingPathComponent("source.pdf") }
    var count: Int { sizes.count }
    var signedExt: String { docSignedExt(original: original.path) }

    private init(dir: URL, data: DocData) {
        self.dir = dir
        title = data.title
        original = URL(fileURLWithPath: data.original)
        sizes = data.sizes
        strokes = data.pages
        index = Int(data.current)
        pdfDoc = PDFDocument(url: dir.appendingPathComponent("source.pdf"))
    }

    func show(_ i: Int, _ board: Board) {
        index = min(max(i, 0), count - 1)
        board.openStrokes(strokes[index])
        image = render(index)
    }

    /// Ricorda i tratti della pagina mostrata (in memoria e in `doc.json`).
    func keep(_ board: Board, view: BoardView) {
        strokes[index] = board.model.strokes()
        try? docSave(dir: dir.path, pages: strokes, current: UInt32(index), view: view)
        board.markSaved()
    }

    func rect(_ view: BoardView) -> [Float] { docPageRect(page: sizes[index], view: view) }

    private func render(_ i: Int) -> UIImage? {
        guard let page = pdfDoc?.page(at: i) else { return nil }
        let w: CGFloat = 1600
        let h = w * CGFloat(sizes[i].height / sizes[i].width)
        return page.thumbnail(of: CGSize(width: w, height: h), for: .cropBox)
    }

    func sign(to dst: URL, view: BoardView) throws {
        try docSign(original: original.path, pdf: pdf.path, dst: dst.path, pages: strokes, view: view)
    }

    /// Il PDF da stampare: le pagine come si vedono, con la scrittura sopra (anche per Word e foto).
    func printPdf(to dst: URL, view: BoardView) throws {
        try pdfSign(src: pdf.path, dst: dst.path, pages: strokes, view: view)
    }

    static var docsRoot: URL {
        FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0].appendingPathComponent("documents", isDirectory: true)
    }

    /// Copia il documento scelto nell'app e prepara le pagine da mostrare.
    static func importFile(_ url: URL, view: BoardView) throws -> OpenDoc {
        let access = url.startAccessingSecurityScopedResource()
        defer { if access { url.stopAccessingSecurityScopedResource() } }
        let ext = url.pathExtension.lowercased()
        guard docExtensions().contains(ext) else { throw NSError(domain: "OpenSlate", code: 1, userInfo: [NSLocalizedDescriptionKey: "." + ext]) }
        let tmp = FileManager.default.temporaryDirectory.appendingPathComponent("import." + ext)
        try? FileManager.default.removeItem(at: tmp)
        try FileManager.default.copyItem(at: url, to: tmp)
        defer { try? FileManager.default.removeItem(at: tmp) }
        try FileManager.default.createDirectory(at: docsRoot, withIntermediateDirectories: true)
        let id = try docImport(root: docsRoot.path, src: tmp.path, name: url.lastPathComponent)
        return try open(docsRoot.appendingPathComponent(id, isDirectory: true), view: view)
    }

    /// Riapre un documento, coi tratti portati nella geometria di `view`.
    static func open(_ dir: URL, view: BoardView) throws -> OpenDoc {
        let d = try docOpen(dir: dir.path, view: view)
        guard !d.sizes.isEmpty else { throw NSError(domain: "OpenSlate", code: 2, userInfo: [NSLocalizedDescriptionKey: "no pages"]) }
        return OpenDoc(dir: dir, data: d)
    }
}

enum Tab: Hashable { case board, library, chat, settings }

/// Lo stato dell'app (come `MainActivity` dell'app Android).
final class AppModel: ObservableObject, LinkListener {
    let board = Board()
    let link = SlateLink()
    let ai = AiStore()
    let chat = ChatState()
    @Published var status: LinkStatus = .idle
    @Published var battery: Int?
    @Published var padSize = CGSize(width: 21600, height: 14800)
    @Published var tab: Tab = .board
    @Published var docsVersion = 0
    @Published var libVersion = 0 {
        didSet { if !syncReloading { sync.syncSoon(3) } }
    }
    @Published var doc: OpenDoc?
    /// Timbri aziendali salvati e quello che si sta posando.
    @Published var stamps: [SavedStamp] = [] {
        didSet { if !syncReloading { sync.syncSoon(3) } }
    }
    /// La sincronizzazione col computer.
    let sync: PhoneSync
    /// Si sta ricaricando quello che è arrivato dal computer (non si risincronizza).
    private var syncReloading = false
    /// La cartella dei dati dell'app (timbri, appunti, sincronizzazione).
    var dataDir: String { FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0].path }
    @Published var sealJob: SealJob?
    /// La firma che si sta posando, e la firma che si sta registrando (i tratti messi da parte).
    @Published var sigJob: SigJob?
    @Published var sigCapture: [PageStroke]?
    /// Firme e timbri proposti dalla chat, dopo quello in prova (li applica l'utente).
    var proposed: [SignProposal] = []
    /// A tutto schermo, come sulla desktop: resta solo il foglio (per scrivere e firmare sullo schermo).
    @Published var compact = false
    @Published var busy: String?
    @Published var toast: String?

    // strumenti, ricordati (come le impostazioni della desktop)
    @AppStorage("quarter") var quarter = 1
    @AppStorage("tool_hl") var highlighterOn = false
    @Published var eraserOn = false
    @AppStorage("pen_color") var penColor = "#141414"
    @AppStorage("pen_width") var penWidth = 2.0
    @AppStorage("hl_color") var hlColor = "#ffeb5a"
    @AppStorage("hl_width") var hlWidth = 16.0
    @AppStorage("paper") var paperCode = "bianco"
    @AppStorage("paper_margin") var paperMargin = true
    /// Stampando la lavagna, anche la rigatura del foglio.
    @AppStorage("print_paper") var printPaper = false
    @AppStorage("zigzag") var zigzag = false
    @AppStorage("natural_ink") var naturalInk = true
    /// L'Apple Pencil sullo schermo scrive sul foglio come lo Slate.
    @AppStorage("screen_pen") var screenPen = false
    /// …e anche un dito o una penna passiva (l'iPhone la vede come un dito).
    @AppStorage("touch_pen") var touchPen = false
    /// Lo schermo gira col telefono (spento: sempre in verticale).
    @AppStorage("auto_rotate") var autoRotate = true { didSet { OrientationDelegate.apply() } }
    /// La bella copia va subito sulle righe del foglio.
    @AppStorage("fair_on_rules") var fairOnRules = false
    /// Bella copia: quanto larghe le lettere (volte il carattere; 0 = come la tua scrittura).
    @AppStorage("fair_width") var fairWidth = 0.0
    /// Bella copia: il testo dal margine sinistro (non da dove hai cominciato).
    @AppStorage("fair_left") var fairLeft = false
    /// Bella copia: il carattere a mano (inclinata, naturale, rotonda, ariosa, corsiva).
    @AppStorage("fair_font") var fairFont = "inclinata"
    /// Bella copia o «Disponi sulle righe» in corso (nil = no).
    @Published var zoneJob: ZoneJob?
    /// Il numero del lavoro in corso: le risposte di uno annullato non contano.
    var neatGen = 0

    let libDir: String
    private let rawDir: URL

    init() {
        let support = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        let docsDir = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        // la libreria in Documenti: si vede anche nell'app File
        libDir = docsDir.appendingPathComponent("library", isDirectory: true).path
        rawDir = support.appendingPathComponent("pad-raw", isDirectory: true)
        sync = PhoneSync(dataDir: support.path, libDir: libDir)
        try? FileManager.default.createDirectory(atPath: libDir, withIntermediateDirectories: true)
        try? FileManager.default.createDirectory(at: support, withIntermediateDirectories: true)
        // le impronte dei tratti già in bella
        neatSetDir(dir: support.path)
        stamps = stampList(dir: support.path)
        sync.onChanged = { [weak self] kinds in self?.syncChanged(kinds) }
        sync.aiExport = { [weak self] in
            guard let ai = self?.ai else { return nil }
            return SyncAi(json: aiSyncJson(list: ai.providers()), mtime: ai.mtime)
        }
        sync.aiImport = { [weak self] json in
            if let l = try? aiSyncParse(json: json) { self?.ai.replaceFromSync(l) }
        }
        link.listener = self
        let w = UserDefaults.standard.double(forKey: "pad_w"), h = UserDefaults.standard.double(forKey: "pad_h")
        if w > 0 && h > 0 { setPadSize(Float(w), Float(h)) }
        applyTools()
        // i documenti delle versioni di prima passano al formato della desktop
        _ = docMigrate(root: OpenDoc.docsRoot.path, view: view)
        if let path = UserDefaults.standard.string(forKey: "doc_dir") {
            if let d = try? OpenDoc.open(URL(fileURLWithPath: path), view: view) {
                doc = d
                d.show(d.index, board)
            } else {
                UserDefaults.standard.removeObject(forKey: "doc_dir")
            }
        }
    }

    var view: BoardView { BoardView(quarter: UInt32(quarter), padW: Float(padSize.width), padH: Float(padSize.height)) }

    // --- la lavagna in diretta sul computer (`PhoneSync.live`)

    /// Lo sfondo già dato alla diretta: l'immagine e il riquadro.
    private var liveBgImage: UIImage?
    private var liveBgRect: [Float] = []

    func startLive() {
        sync.onLiveTick = { [weak self] fresh in self?.liveUpdate(fresh: fresh) }
        sync.startLive(board.model)
    }

    /// Com'è il foglio e, su un documento, la sua pagina sotto i tratti.
    func liveUpdate(fresh: Bool) {
        guard let live = sync.live else { return }
        let d = doc
        live.setMeta(title: d?.title ?? board.title, padW: Float(padSize.width), padH: Float(padSize.height), quarter: UInt32(quarter),
                     paper: d == nil ? paperCode : "", margin: paperMargin)
        let img = d?.image
        let rect = (d != nil && img != nil) ? d!.rect(view) : []
        if !fresh && img === liveBgImage && rect == liveBgRect { return }
        liveBgImage = img
        liveBgRect = rect
        guard let img else {
            live.setBackground(png: Data(), rect: [])
            return
        }
        DispatchQueue.global(qos: .utility).async {
            // basta una pagina leggera: sul computer è solo da guardare
            let k = min(1, 1600 / max(img.size.width, img.size.height))
            let size = CGSize(width: img.size.width * k, height: img.size.height * k)
            let fmt = UIGraphicsImageRendererFormat()
            fmt.scale = 1
            let small = UIGraphicsImageRenderer(size: size, format: fmt).image { _ in img.draw(in: CGRect(origin: .zero, size: size)) }
            live.setBackground(png: small.pngData() ?? Data(), rect: rect)
        }
    }

    func setPadSize(_ w: Float, _ h: Float) {
        padSize = CGSize(width: CGFloat(w), height: CGFloat(h))
        board.model.setPageSize(width: w, height: h)
        UserDefaults.standard.set(Double(w), forKey: "pad_w")
        UserDefaults.standard.set(Double(h), forKey: "pad_h")
    }

    func applyTools() {
        let m = board.model
        m.setTool(tool: highlighterOn ? .highlighter : .pen)
        m.setEraser(on: eraserOn)
        m.setPen(color: penColor, width: Float(penWidth))
        m.setHighlighter(color: hlColor, width: Float(hlWidth))
        m.setZigzag(on: zigzag)
        setNaturalInk(on: naturalInk)
        Ink.natural = naturalInk
        objectWillChange.send()
    }

    func show(_ msg: String) {
        toast = msg
        DispatchQueue.main.asyncAfter(deadline: .now() + 3) { [weak self] in if self?.toast == msg { self?.toast = nil } }
    }

    // MARK: archivio

    @discardableResult
    func saveBoard() -> Bool {
        do {
            board.id = try librarySave(dir: libDir, page: board.toPage(Float(padSize.width), Float(padSize.height)))
            if board.created.isEmpty { board.created = (try? libraryLoad(dir: libDir, id: board.id).created) ?? "" }
            board.markSaved()
            libVersion += 1
            return true
        } catch {
            show(L("Salvataggio non riuscito: ", "Could not save: ") + "\(error)")
            return false
        }
    }

    func saveBoardIfNeeded() {
        if sigCapture != nil { return }
        if let d = doc { keepDoc(d) } else if board.dirty { saveBoard() }
    }

    func openOnBoard(_ p: Page) {
        saveBoardIfNeeded()
        if doc != nil { closeDoc() }
        board.load(p)
        if p.width > 0 && p.height > 0 { setPadSize(p.width, p.height) }
        tab = .board
    }

    // MARK: documenti

    func openDocument(_ url: URL) {
        saveBoardIfNeeded()
        busy = L("Apro il documento…", "Opening the document…")
        // un Word si impagina, una foto grande si converte: non sul thread principale
        DispatchQueue.global(qos: .userInitiated).async {
            let v = self.view
            let r = Result { try OpenDoc.importFile(url, view: v) }
            DispatchQueue.main.async { [self] in
                busy = nil
                switch r {
                case .success(let d):
                    if doc != nil { closeDoc() }
                    doc = d
                    UserDefaults.standard.set(d.dir.path, forKey: "doc_dir")
                    d.show(0, board)
                case .failure(let e):
                    show(L("Impossibile aprire il documento: ", "Could not open the document: ") + e.localizedDescription)
                }
            }
        }
    }

    /// Salva i tratti del documento aperto e lo fa arrivare al computer.
    func keepDoc(_ d: OpenDoc) {
        // mentre si registra la firma, sul foglio c'è la firma, non la pagina
        if sigCapture != nil { return }
        let changed = board.dirty
        d.keep(board, view: view)
        if changed { sync.syncSoon(3) }
    }

    /// Apre un documento già importato (dall'elenco o arrivato dal computer).
    func openDocDir(_ dir: URL) {
        saveBoardIfNeeded()
        do {
            let d = try OpenDoc.open(dir, view: view)
            if doc != nil { closeDoc() }
            doc = d
            UserDefaults.standard.set(d.dir.path, forKey: "doc_dir")
            d.show(d.index, board)
            tab = .board
        } catch {
            show(L("Impossibile aprire il documento: ", "Could not open the document: ") + error.localizedDescription)
        }
    }

    /// Sposta il documento nel cestino (sparisce anche dal computer).
    func trashDoc(_ id: String) {
        if doc?.id == id { closeDoc() }
        do { try docTrash(root: OpenDoc.docsRoot.path, id: id) } catch { show("\(error)") }
        docsVersion += 1
        sync.syncSoon(3)
    }

    func closeDoc() {
        doc?.keep(board, view: view)
        doc = nil
        proposed.removeAll()
        UserDefaults.standard.removeObject(forKey: "doc_dir")
        board.newPage()
    }

    func goPage(_ delta: Int) {
        guard let d = doc else { return }
        keepDoc(d)
        d.show(d.index + delta, board)
    }

    /// Arrivato qualcosa dal computer: si ricarica quello che è cambiato.
    func syncChanged(_ kinds: [String]) {
        syncReloading = true
        if kinds.contains("lib") || kinds.contains("notes") { libVersion += 1 }
        if kinds.contains("stamps") { stamps = stampList(dir: dataDir) }
        syncReloading = false
        if kinds.contains("docs") {
            docsVersion += 1
            if let d = doc {
                if !FileManager.default.fileExists(atPath: d.dir.appendingPathComponent("doc.json").path) {
                    // tolto sul computer: si chiude
                    doc = nil
                    UserDefaults.standard.removeObject(forKey: "doc_dir")
                    board.newPage()
                    show(L("Il documento è stato spostato nel cestino sul computer.", "The document was moved to the trash on the computer."))
                } else if !board.dirty, let nd = try? OpenDoc.open(d.dir, view: view) {
                    // cambiato sul computer, e qui niente da salvare: si riapre
                    doc = nd
                    nd.show(d.index, board)
                }
            }
        }
    }

    /// La copia firmata in un file temporaneo (per salvarla o condividerla).
    func signedFile() -> URL? {
        guard let d = doc else { return nil }
        keepDoc(d)
        let f = Export.shareDir().appendingPathComponent(d.title + " " + L("(firmato)", "(signed)") + "." + d.signedExt)
        do {
            try d.sign(to: f, view: view)
            return f
        } catch {
            show(L("Salvataggio non riuscito: ", "Could not save: ") + "\(error)")
            return nil
        }
    }

    /// Stampa il documento con la scrittura, o la pagina della lavagna, col
    /// dialogo di stampa di iOS (AirPrint, copie, pagine, «Salva in File»).
    func printNow(withPaper: Bool? = nil) {
        if let w = withPaper { printPaper = w }
        let url: URL
        let title: String
        do {
            if let d = doc {
                keepDoc(d)
                url = Export.shareDir().appendingPathComponent(d.title + ".pdf")
                try d.printPdf(to: url, view: view)
                title = d.title
            } else {
                let page = board.toPage(Float(padSize.width), Float(padSize.height))
                url = try Export.pdf(page, quarter: quarter, paper: printPaper ? (paperCode, paperMargin) : nil)
                title = page.title.isEmpty ? "OpenSlate" : page.title
            }
        } catch {
            show(L("Stampa non riuscita: ", "Could not print: ") + "\(error)")
            return
        }
        let info = UIPrintInfo(dictionary: nil)
        info.outputType = .general
        info.jobName = title
        let pc = UIPrintInteractionController.shared
        pc.printInfo = info
        pc.printingItem = url
        pc.present(animated: true)
    }

    // MARK: pad

    func onStatus(_ s: LinkStatus) {
        status = s
        if case .live(let info) = s {
            battery = info.batteryPct.map { Int($0) }
            if info.width > 0 && info.height > 0 { setPadSize(info.width, info.height) }
        }
        switch s {
        case .idle, .disconnected, .failed: battery = nil
        default: break
        }
        // mentre il pad è collegato lo schermo resta acceso
        UIApplication.shared.isIdleTimerDisabled = s.isOn
    }

    func onPen(_ points: [PenPoint]) {
        board.slateTip(points)
        board.pen(points)
    }

    /// Prima la copia grezza (la copia di sicurezza, come la desktop), poi la
    /// pagina nell'archivio. Al sicuro (e quindi da togliere dal pad) solo se
    /// la copia grezza è scritta.
    func onPageDownloaded(raw: Data, timeBcd: Data, crcOk: Bool, info: PadInfo) -> Bool {
        try? FileManager.default.createDirectory(at: rawDir, withIntermediateDirectories: true)
        let label = padTimeLabel(timeBcd: timeBcd)
        var f = rawDir.appendingPathComponent(label + ".bin")
        var n = 2
        while FileManager.default.fileExists(atPath: f.path) {
            f = rawDir.appendingPathComponent("\(label)-\(n).bin")
            n += 1
        }
        do { try raw.write(to: f, options: .atomic) } catch { return false }
        let w = info.width > 0 ? info.width : Float(padSize.width)
        let h = info.height > 0 ? info.height : Float(padSize.height)
        if let p = pageFromPad(raw: raw, timeBcd: timeBcd, width: w, height: h, rawName: f.lastPathComponent) {
            _ = try? librarySave(dir: libDir, page: p)
        }
        libVersion += 1
        return true
    }

    // MARK: AI

    func transcribe(_ page: Page, done: @escaping (String) -> Void) {
        if ai.services.allSatisfy({ !$0.enabled }) {
            show(L("Nessun servizio AI: aggiungine uno in Impostazioni.", "No AI service yet: add one in Settings."))
            return
        }
        // come la desktop: la scrittura ritagliata e grande, non tutto il foglio
        let png: Data
        do { png = try aiPagePng(page: page, quarter: UInt32(quarter)) } catch { show("\(error)"); return }
        busy = L("Trascrivo la pagina…", "Transcribing the page…")
        let providers = ai.providers()
        DispatchQueue.global(qos: .userInitiated).async {
            let r = Result { try aiTranscribe(providers: providers, png: png, listener: StatusListener { [weak self] t in self?.busy = L("Trascrivo la pagina…", "Transcribing the page…") + "\n" + t }) }
            DispatchQueue.main.async { [self] in
                busy = nil
                switch r {
                case .success(let reply):
                    // ha letto un servizio di ripiego: di solito legge peggio, va detto
                    if let why = reply.fallback {
                        show(L("Ha trascritto \(reply.provider) perché prima non ha risposto \(why). Il ripiego può leggere peggio formule e scrittura: se serve, ripeti «Trascrivi».",
                               "Transcribed by \(reply.provider) because \(why) did not answer first. The fallback may read formulas and handwriting less well: if needed, transcribe again."))
                    }
                    try? reply.text.write(toFile: libDir + "/\(page.id).md", atomically: true, encoding: .utf8)
                    done(reply.text)
                case .failure(let e):
                    show("AI: " + aiMessage(e))
                }
            }
        }
    }

    func noteOf(_ id: String) -> String? { try? String(contentsOfFile: libDir + "/\(id).md", encoding: .utf8) }
}

/// Il messaggio di un errore del cuore Rust.
func aiMessage(_ e: Error) -> String {
    if let a = e as? AiError {
        switch a {
        case .Failed(let reason): return reason
        case .Cancelled: return L("interrotto", "stopped")
        }
    }
    if let s = e as? StoreError, case .Io(let reason) = s { return reason }
    return e.localizedDescription
}

/// Ascolta solo i tentativi (per la trascrizione).
final class StatusListener: AiListener {
    let onText: (String) -> Void
    init(_ f: @escaping (String) -> Void) { onText = f }
    func onEvent(event: AiEvent) -> Bool {
        switch event {
        case .trying(let p, let m): DispatchQueue.main.async { self.onText(Lf("Chiedo a %@ (%@)…", "Asking %@ (%@)…", p, m)) }
        case .skipped(let p, let w): DispatchQueue.main.async { self.onText(Lf("%@ saltato: %@", "%@ skipped: %@", p, w)) }
        default: break
        }
        return true
    }
}
