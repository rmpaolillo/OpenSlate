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

    func pen(_ points: [PenPoint]) {
        for op in model.pen(points: points) {
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

/// Un documento da firmare (PDF, Word, foto, SVG), come `DocMode.kt`.
final class OpenDoc: ObservableObject {
    let dir: URL
    let title: String
    let original: URL
    let sizes: [PdfPageSize]
    @Published private(set) var index = 0
    @Published private(set) var image: UIImage?
    private var strokes: [[PageStroke]]
    private var pdfDoc: PDFDocument?

    var pdf: URL { dir.appendingPathComponent("source.pdf") }
    var count: Int { sizes.count }
    var signedExt: String { docSignedExt(original: original.path) }

    private init(dir: URL, title: String, original: URL, sizes: [PdfPageSize]) {
        self.dir = dir; self.title = title; self.original = original; self.sizes = sizes
        strokes = Array(repeating: [], count: sizes.count)
        pdfDoc = PDFDocument(url: dir.appendingPathComponent("source.pdf"))
    }

    private func pageId(_ i: Int) -> String { String(format: "p%04d", i + 1) }

    func show(_ i: Int, _ board: Board) {
        index = min(max(i, 0), count - 1)
        board.openStrokes(strokes[index])
        image = render(index)
    }

    func keep(_ board: Board) {
        let s = board.model.strokes()
        strokes[index] = s
        _ = try? librarySave(dir: dir.path, page: Page(id: pageId(index), title: "", created: nowCreated(), source: "documento", raw: nil, width: 0, height: 0, strokes: s))
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
    static func importFile(_ url: URL) throws -> OpenDoc {
        let access = url.startAccessingSecurityScopedResource()
        defer { if access { url.stopAccessingSecurityScopedResource() } }
        let ext = url.pathExtension.lowercased()
        guard docExtensions().contains(ext) else { throw NSError(domain: "OpenSlate", code: 1, userInfo: [NSLocalizedDescriptionKey: "." + ext]) }
        let dir = docsRoot.appendingPathComponent(String(Int(Date().timeIntervalSince1970 * 1000)), isDirectory: true)
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let original = dir.appendingPathComponent("original." + ext)
        try FileManager.default.copyItem(at: url, to: original)
        do {
            _ = try docPrepare(original: original.path, pdf: dir.appendingPathComponent("source.pdf").path)
        } catch {
            try? FileManager.default.removeItem(at: dir)
            throw error
        }
        try url.deletingPathExtension().lastPathComponent.write(to: dir.appendingPathComponent("title.txt"), atomically: true, encoding: .utf8)
        return try open(dir)
    }

    static func open(_ dir: URL) throws -> OpenDoc {
        let title = (try? String(contentsOf: dir.appendingPathComponent("title.txt"), encoding: .utf8)) ?? dir.lastPathComponent
        let sizes = try pdfPages(path: dir.appendingPathComponent("source.pdf").path)
        guard !sizes.isEmpty else { throw NSError(domain: "OpenSlate", code: 2, userInfo: [NSLocalizedDescriptionKey: "no pages"]) }
        let files = (try? FileManager.default.contentsOfDirectory(atPath: dir.path)) ?? []
        let original = files.first { $0.hasPrefix("original.") }.map { dir.appendingPathComponent($0) } ?? dir.appendingPathComponent("source.pdf")
        let d = OpenDoc(dir: dir, title: title, original: original, sizes: sizes)
        for i in sizes.indices {
            if let p = try? libraryLoad(dir: dir.path, id: d.pageId(i)) { d.strokes[i] = p.strokes }
        }
        return d
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
    @Published var libVersion = 0
    @Published var doc: OpenDoc?
    /// Timbri aziendali salvati e quello che si sta posando.
    @Published var stamps: [SavedStamp] = []
    @Published var sealJob: SealJob?
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
    @AppStorage("zigzag") var zigzag = false
    @AppStorage("natural_ink") var naturalInk = true
    /// La bella copia va subito sulle righe del foglio.
    @AppStorage("fair_on_rules") var fairOnRules = false
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
        try? FileManager.default.createDirectory(atPath: libDir, withIntermediateDirectories: true)
        try? FileManager.default.createDirectory(at: support, withIntermediateDirectories: true)
        // le impronte dei tratti già in bella
        neatSetDir(dir: support.path)
        stamps = stampList(dir: support.path)
        link.listener = self
        let w = UserDefaults.standard.double(forKey: "pad_w"), h = UserDefaults.standard.double(forKey: "pad_h")
        if w > 0 && h > 0 { setPadSize(Float(w), Float(h)) }
        applyTools()
        if let path = UserDefaults.standard.string(forKey: "doc_dir"), let d = try? OpenDoc.open(URL(fileURLWithPath: path)) {
            doc = d
            d.show(0, board)
        }
    }

    var view: BoardView { BoardView(quarter: UInt32(quarter), padW: Float(padSize.width), padH: Float(padSize.height)) }

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
        if let d = doc { d.keep(board) } else if board.dirty { saveBoard() }
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
            let r = Result { try OpenDoc.importFile(url) }
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

    func closeDoc() {
        doc?.keep(board)
        doc = nil
        UserDefaults.standard.removeObject(forKey: "doc_dir")
        board.newPage()
    }

    func goPage(_ delta: Int) {
        guard let d = doc else { return }
        d.keep(board)
        d.show(d.index + delta, board)
    }

    /// La copia firmata in un file temporaneo (per salvarla o condividerla).
    func signedFile() -> URL? {
        guard let d = doc else { return nil }
        d.keep(board)
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
    func printNow() {
        let url: URL
        let title: String
        do {
            if let d = doc {
                d.keep(board)
                url = Export.shareDir().appendingPathComponent(d.title + ".pdf")
                try d.printPdf(to: url, view: view)
                title = d.title
            } else {
                let page = board.toPage(Float(padSize.width), Float(padSize.height))
                url = try Export.pdf(page, quarter: quarter)
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

    func onPen(_ points: [PenPoint]) { board.pen(points) }

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
        busy = L("Trascrivo la pagina…", "Transcribing the page…")
        let png = Export.pngData(page, quarter: quarter)
        let providers = ai.providers()
        DispatchQueue.global(qos: .userInitiated).async {
            let r = Result { try aiTranscribe(providers: providers, png: png, listener: StatusListener { [weak self] t in self?.busy = L("Trascrivo la pagina…", "Transcribing the page…") + "\n" + t }) }
            DispatchQueue.main.async { [self] in
                busy = nil
                switch r {
                case .success(let reply):
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
