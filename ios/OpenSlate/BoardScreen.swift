import SwiftUI
import UIKit
import UniformTypeIdentifiers

/// Il foglio: rigatura, pagina del documento, tratti; zoom con due dita,
/// doppio tocco = pagina intera; con la gomma un dito strofina e cancella.
final class SheetUIView: UIView {
    var strokes: [PageStroke] = [] { didSet { setNeedsDisplay() } }
    var padW: Float = 21600, padH: Float = 14800
    var quarter = 1 { didSet { if oldValue != quarter { marksKey = "" }; setNeedsDisplay() } }
    var paper: (String, Bool)? { didSet { marksKey = ""; setNeedsDisplay() } }
    var background: (UIImage, [Float])? { didSet { setNeedsDisplay() } }
    var erasing = false
    var onErase: ((CGPoint, CGPoint, Float, Float) -> Void)?
    var onEraseEnd: (() -> Void)?
    /// Un dito sceglie una zona (bella copia, «Disponi sulle righe»).
    var selecting = false
    var onZoneDrag: (([Float]?) -> Void)?
    var onZone: (([Float]) -> Void)?
    /// Il riquadro scelto o trascinato; l'anteprima: la zona velata e i tratti di dopo.
    var zoneRect: [Float]? { didSet { setNeedsDisplay() } }
    var veil: [Float]? { didSet { setNeedsDisplay() } }
    var preview: [PageStroke]? { didSet { setNeedsDisplay() } }
    private var zoneStart: CGPoint?
    /// Il timbro da posare (u, v, larghezza, altezza): un dito lo sposta, l'angolo lo ingrandisce.
    var seal: (StampSpec, [Float])? { didSet { setNeedsDisplay() } }
    var onSealRect: (([Float]) -> Void)?
    /// 0 = niente, 1 = sposta, 2 = grandezza.
    private var sealMode = 0

    private var zoom: CGFloat = 1
    private var offset = CGPoint.zero
    private var marks: [PaperMark] = []
    private var marksKey = ""
    private var lastPan = CGPoint.zero
    private var erased = false
    private let margin: CGFloat = 12

    override init(frame: CGRect) {
        super.init(frame: frame)
        backgroundColor = UIColor(white: 0.9, alpha: 1)
        contentMode = .redraw
        isMultipleTouchEnabled = true
        let pinch = UIPinchGestureRecognizer(target: self, action: #selector(onPinch(_:)))
        let pan = UIPanGestureRecognizer(target: self, action: #selector(onPan(_:)))
        pan.maximumNumberOfTouches = 2
        let dbl = UITapGestureRecognizer(target: self, action: #selector(onDouble))
        dbl.numberOfTapsRequired = 2
        [pinch, pan, dbl].forEach(addGestureRecognizer)
    }

    required init?(coder: NSCoder) { fatalError() }

    /// Pagina intera: scala pad→punti e angolo in alto a sinistra.
    private func fit() -> (CGFloat, CGFloat, CGFloat, CGFloat, CGFloat) {
        let (rw, rh) = Ink.rotated(padW, padH, quarter)
        let k = min((bounds.width - 2 * margin) / CGFloat(rw), (bounds.height - 2 * margin) / CGFloat(rh))
        let pw = CGFloat(rw) * k, ph = CGFloat(rh) * k
        return (k, (bounds.width - pw) / 2, (bounds.height - ph) / 2, pw, ph)
    }

    /// Punto dello schermo → coordinate del pad.
    private func toPad(_ p: CGPoint) -> CGPoint {
        let (k, ox, oy, _, _) = fit()
        let X = Float(((p.x - offset.x) / zoom - ox) / k), Y = Float(((p.y - offset.y) / zoom - oy) / k)
        switch quarter {
        case 0: return CGPoint(x: CGFloat(X), y: CGFloat(Y))
        case 1: return CGPoint(x: CGFloat(Y), y: CGFloat(padH - X))
        case 2: return CGPoint(x: CGFloat(padW - X), y: CGFloat(padH - Y))
        default: return CGPoint(x: CGFloat(padW - Y), y: CGFloat(X))
        }
    }

    /// Punto dello schermo → frazioni della pagina vista (0..1).
    private func toUnit(_ p: CGPoint) -> CGPoint {
        let (_, ox, oy, pw, ph) = fit()
        let u = ((p.x - offset.x) / zoom - ox) / pw, v = ((p.y - offset.y) / zoom - oy) / ph
        return CGPoint(x: min(max(u, 0), 1), y: min(max(v, 0), 1))
    }

    @objc private func onDouble() {
        zoom = 1
        offset = .zero
        setNeedsDisplay()
    }

    @objc private func onPinch(_ g: UIPinchGestureRecognizer) {
        let c = g.location(in: self)
        let z = min(max(zoom * g.scale, 1), 8)
        offset = CGPoint(x: c.x - (c.x - offset.x) * (z / zoom), y: c.y - (c.y - offset.y) * (z / zoom))
        zoom = z
        if zoom == 1 { offset = .zero }
        g.scale = 1
        setNeedsDisplay()
    }

    @objc private func onPan(_ g: UIPanGestureRecognizer) {
        let p = g.location(in: self)
        if g.state == .began { lastPan = p; erased = false }
        if let (_, r) = seal, r.count == 4 {
            let (_, ox, oy, pw, ph) = fit()
            if g.state == .began {
                let l = offset.x + (ox + CGFloat(r[0]) * pw) * zoom, t = offset.y + (oy + CGFloat(r[1]) * ph) * zoom
                let rr = offset.x + (ox + CGFloat(r[0] + r[2]) * pw) * zoom, b = offset.y + (oy + CGFloat(r[1] + r[3]) * ph) * zoom
                if abs(p.x - rr) < 28 && abs(p.y - b) < 28 { sealMode = 2 }
                else if p.x >= l - 8 && p.x <= rr + 8 && p.y >= t - 8 && p.y <= b + 8 { sealMode = 1 }
                else { sealMode = 0 }
            }
            if sealMode != 0 && g.numberOfTouches <= 1 {
                let du = Float((p.x - lastPan.x) / (pw * zoom)), dv = Float((p.y - lastPan.y) / (ph * zoom))
                if sealMode == 1 {
                    onSealRect?([r[0] + du, r[1] + dv, r[2], r[3]])
                } else {
                    let f = min(max((r[2] + du) / r[2], 0.5), 2)
                    if r[2] * f > 0.03 && r[2] * f < 1.2 { onSealRect?([r[0], r[1], r[2] * f, r[3] * f]) }
                }
                lastPan = p
                if g.state == .ended || g.state == .cancelled { sealMode = 0 }
                return
            }
        }
        if selecting {
            if g.state == .began && g.numberOfTouches == 1 { zoneStart = toUnit(p) }
            if let a = zoneStart {
                let b = toUnit(p)
                let r = [Float(min(a.x, b.x)), Float(min(a.y, b.y)), Float(max(a.x, b.x)), Float(max(a.y, b.y))]
                if g.state == .ended || g.state == .cancelled {
                    zoneStart = nil
                    if r[2] - r[0] > 0.01 && r[3] - r[1] > 0.01 { onZone?(r) } else { onZoneDrag?(nil) }
                } else {
                    onZoneDrag?(r)
                }
            }
            lastPan = p
            return
        }
        if erasing && g.numberOfTouches == 1 {
            let (k, _, _, _, _) = fit()
            let (_, rh) = Ink.rotated(padW, padH, quarter)
            onErase?(toPad(lastPan), toPad(p), Float(14 / (k * zoom)), rh)
            erased = true
        } else if zoom > 1 {
            offset.x += p.x - lastPan.x
            offset.y += p.y - lastPan.y
            setNeedsDisplay()
        }
        lastPan = p
        if (g.state == .ended || g.state == .cancelled) && erased { onEraseEnd?() }
    }

    override func draw(_ rect: CGRect) {
        guard let c = UIGraphicsGetCurrentContext() else { return }
        let (_, ox, oy, pw, ph) = fit()
        let (rw, rh) = Ink.rotated(padW, padH, quarter)
        c.saveGState()
        c.translateBy(x: offset.x, y: offset.y)
        c.scaleBy(x: zoom, y: zoom)
        c.setFillColor(UIColor.white.cgColor)
        c.fill(CGRect(x: ox, y: oy, width: pw, height: ph))
        c.restoreGState()
        // rigatura a spessore fisso, sotto i tratti
        if let (code, margin) = paper, code != "bianco" {
            let key = "\(code)-\(margin)-\(rw)-\(rh)"
            if key != marksKey { marks = paperMarks(code: code, wCm: rw / 1000, hCm: rh / 1000, margin: margin); marksKey = key }
            if let kind = paperKinds().first(where: { $0.code == code }) {
                c.saveGState()
                c.clip(to: CGRect(x: offset.x + ox * zoom, y: offset.y + oy * zoom, width: pw * zoom, height: ph * zoom))
                Ink.drawPaper(c, marks, kind: kind, left: offset.x + ox * zoom, top: offset.y + oy * zoom, pxPerCm: pw * zoom / CGFloat(rw / 1000))
                c.restoreGState()
            }
        }
        c.saveGState()
        c.translateBy(x: offset.x + ox * zoom, y: offset.y + oy * zoom)
        c.scaleBy(x: zoom, y: zoom)
        if let (img, r) = background, r.count == 4 {
            img.draw(in: CGRect(x: CGFloat(r[0]) * pw, y: CGFloat(r[1]) * ph, width: CGFloat(r[2]) * pw, height: CGFloat(r[3]) * ph))
        }
        Ink.draw(c, strokes, padW: padW, padH: padH, quarter: quarter, w: pw, h: ph)
        c.restoreGState()
        // la zona scelta, o l'anteprima sotto un velo
        let accent = UIColor(red: 0.16, green: 0.41, blue: 0.86, alpha: 1)
        func screen(_ r: [Float]) -> CGRect {
            CGRect(x: offset.x + (ox + CGFloat(r[0]) * pw) * zoom, y: offset.y + (oy + CGFloat(r[1]) * ph) * zoom,
                   width: CGFloat(r[2] - r[0]) * pw * zoom, height: CGFloat(r[3] - r[1]) * ph * zoom)
        }
        func dashed(_ r: CGRect) {
            c.saveGState()
            c.setStrokeColor(accent.cgColor)
            c.setLineWidth(1.5)
            c.setLineDash(phase: 0, lengths: [6, 4])
            c.stroke(r)
            c.restoreGState()
        }
        if let v = veil, v.count == 4 {
            let r = screen(v)
            c.saveGState()
            c.clip(to: r)
            c.setFillColor(UIColor.white.withAlphaComponent(0.88).cgColor)
            c.fill(r)
            if let (code, _) = paper, code != "bianco", let kind = paperKinds().first(where: { $0.code == code }) {
                Ink.drawPaper(c, marks, kind: kind, left: offset.x + ox * zoom, top: offset.y + oy * zoom, pxPerCm: pw * zoom / CGFloat(rw / 1000))
            }
            c.translateBy(x: offset.x + ox * zoom, y: offset.y + oy * zoom)
            c.scaleBy(x: zoom, y: zoom)
            if let p = preview { Ink.draw(c, p, padW: padW, padH: padH, quarter: quarter, w: pw, h: ph) }
            c.restoreGState()
            dashed(r)
        }
        if let z = zoneRect, z.count == 4 {
            let r = screen(z)
            c.setFillColor(accent.withAlphaComponent(0.09).cgColor)
            c.fill(r)
            dashed(r)
        }
        // il timbro da posare, con il bordo tratteggiato e l'angolo da tirare
        if let (spec, s) = seal, s.count == 4 {
            let r = screen([s[0], s[1], s[0] + s[2], s[1] + s[3]])
            SealArt.draw(c, spec, quarter: 0, in: r)
            dashed(r.insetBy(dx: -6, dy: -6))
            c.setFillColor(accent.cgColor)
            c.fill(CGRect(x: r.maxX - 7, y: r.maxY - 7, width: 14, height: 14))
        }
    }
}

struct Sheet: UIViewRepresentable {
    let strokes: [PageStroke]
    let padW: Float, padH: Float
    let quarter: Int
    let version: Int
    var paper: (String, Bool)?
    var background: (UIImage, [Float])?
    var eraser: Board?
    /// Scelta della zona col dito: mentre si trascina, quando si alza il dito.
    var onZoneDrag: (([Float]?) -> Void)?
    var onZone: (([Float]) -> Void)?
    var zoneRect: [Float]?
    var veil: [Float]?
    var preview: [PageStroke]?
    var seal: (StampSpec, [Float])?
    var onSealRect: (([Float]) -> Void)?

    func makeUIView(context: Context) -> SheetUIView { SheetUIView() }

    func updateUIView(_ v: SheetUIView, context: Context) {
        v.padW = padW
        v.padH = padH
        v.quarter = quarter
        if v.paper?.0 != paper?.0 || v.paper?.1 != paper?.1 { v.paper = paper }
        v.background = background
        v.erasing = eraser != nil
        v.onErase = eraser.map { b in { a, c, tol, h in b.erase(a, c, tol: tol, pageH: h) } }
        v.onEraseEnd = eraser.map { b in { b.eraseEnd() } }
        v.selecting = onZone != nil
        v.onZoneDrag = onZoneDrag
        v.onZone = onZone
        v.zoneRect = zoneRect
        v.veil = veil
        v.preview = preview
        v.onSealRect = onSealRect
        if v.seal?.0 != seal?.0 || v.seal?.1 != seal?.1 { v.seal = seal }
        v.strokes = strokes
    }
}

struct BoardScreen: View {
    @EnvironmentObject var app: AppModel
    @ObservedObject var board: Board
    @State private var choosingPaper = false
    @State private var importing = false
    @State private var shareURL: URL?
    @State private var choosingMore = false
    @State private var note: NoteItem?

    var body: some View {
        let d = app.doc
        VStack(spacing: 0) {
            topBar
            if let d { DocBar(doc: d) }
            if !(app.status.isOn) && app.status != .idle {
                Text(app.status.text).font(.caption).foregroundStyle(.secondary).frame(maxWidth: .infinity, alignment: .leading).padding(.horizontal, 12).padding(.vertical, 4)
            }
            Sheet(strokes: board.strokes, padW: Float(app.padSize.width), padH: Float(app.padSize.height), quarter: app.quarter, version: board.version,
                  paper: d == nil ? (app.paperCode, app.paperMargin) : nil,
                  background: d?.image.map { ($0, d!.rect(app.view)) },
                  eraser: app.eraserOn ? board : nil,
                  onZoneDrag: app.zoneJob?.phase == .select ? { r in app.zoneJob?.drag = r } : nil,
                  onZone: app.zoneJob?.phase == .select ? { r in app.neatZone(r) } : nil,
                  zoneRect: app.zoneJob.flatMap { $0.phase == .preview ? nil : ($0.drag ?? $0.zone?.floats) },
                  veil: app.zoneJob.flatMap { $0.phase == .preview ? $0.preview?.veil.floats : nil },
                  preview: app.zoneJob?.preview?.strokes,
                  seal: app.sealJob.map { ($0.spec, $0.rect) },
                  onSealRect: { r in app.sealJob?.rect = r })
            if app.sealJob != nil { SealBar() } else if app.zoneJob != nil { NeatBar() } else { ToolBar(board: board, choosingPaper: $choosingPaper) }
        }
        .sheet(isPresented: $choosingPaper) { PaperPicker() }
        .fileImporter(isPresented: $importing, allowedContentTypes: docTypes) { r in if case .success(let url) = r { app.openDocument(url) } }
        .sheet(item: $shareURL) { url in ShareSheet(items: [url]) }
        .sheet(isPresented: $choosingMore) {
            MoreFormatsSheet { kind in
                let page = board.toPage(Float(app.padSize.width), Float(app.padSize.height))
                app.exportMore(page, kind: kind, fromBoard: true) { url in shareURL = url }
            }
        }
        .sheet(item: $note) { n in NoteScreen(item: n) }
    }

    /// La lavagna com'è adesso negli appunti, come immagine.
    private func copyBoard() {
        Export.copy(board.toPage(Float(app.padSize.width), Float(app.padSize.height)), quarter: app.quarter)
        app.show(L("Lavagna copiata negli appunti.", "Board copied to the clipboard."))
    }

    private var docTypes: [UTType] {
        docExtensions().compactMap { UTType(filenameExtension: $0) }
    }

    private var topBar: some View {
        HStack(spacing: 6) {
            Menu {
                Text(app.status.text)
                if app.status.isOn || app.status.isBusy {
                    Button { app.link.disconnect() } label: { Label(L("Scollega", "Disconnect"), systemImage: "personalhotspot.slash") }
                } else {
                    Button { app.link.connect() } label: { Label(L("Collega", "Connect"), systemImage: "dot.radiowaves.left.and.right") }
                }
                Button { app.link.sync() } label: { Label(L("Scarica pagine", "Download pages"), systemImage: "square.and.arrow.down") }
                    .disabled(!app.link.live)
            } label: {
                Label(app.battery.map { "\($0)%" } ?? L("Pad", "Pad"), systemImage: app.status.isOn ? "wave.3.right.circle.fill" : app.status.isBusy ? "wave.3.right.circle" : "circle.dotted")
                    .padding(.horizontal, 10).padding(.vertical, 6)
                    .background(Capsule().stroke(Color.secondary.opacity(0.5)))
            }
            Text(app.doc?.title ?? (board.title.isEmpty ? L("Lavagna", "Board") : board.title))
                .font(.headline).lineLimit(1).frame(maxWidth: .infinity, alignment: .leading)
            Button { board.undo() } label: { Image(systemName: "arrow.uturn.backward") }.disabled(!board.model.canUndo())
            Button { board.redo() } label: { Image(systemName: "arrow.uturn.forward") }.disabled(!board.model.canRedo())
            Button { copyBoard() } label: { Image(systemName: "doc.on.doc") }.disabled(board.isEmpty || app.doc != nil)
            Menu {
                Section(L("Pagina", "Page")) {
                    Button { app.saveBoardIfNeeded(); board.newPage() } label: { Label(L("Nuova pagina", "New page"), systemImage: "doc.badge.plus") }.disabled(app.doc != nil)
                    Button { if app.saveBoard() { app.show(L("Salvata nella libreria.", "Saved in the library.")) } } label: { Label(L("Salva", "Save"), systemImage: "square.and.arrow.down.on.square") }.disabled(board.isEmpty || app.doc != nil)
                    Button(role: .destructive) { board.clear() } label: { Label(L("Pulisci il foglio", "Clear the page"), systemImage: "trash") }.disabled(board.isEmpty)
                    Button {
                        if board.id.isEmpty || board.dirty { app.saveBoard() }
                        let page = board.toPage(Float(app.padSize.width), Float(app.padSize.height))
                        app.transcribe(page) { text in note = NoteItem(id: page.id, title: page.title.isEmpty ? String(page.created.prefix(16)) : page.title, text: text) }
                    } label: { Label(L("Trascrivi in nota", "Transcribe to a note"), systemImage: "text.viewfinder") }.disabled(board.isEmpty || app.doc != nil)
                }
                Section(L("Scrittura", "Writing")) {
                    Button { app.startNeat(.fair) } label: { Label(L("Trasforma in bella…", "Fair copy…"), systemImage: "wand.and.stars") }
                        .disabled(board.isEmpty || app.doc != nil)
                    Button { app.startNeat(.rules) } label: { Label(L("Disponi sulle righe", "Arrange on lines"), systemImage: "text.alignleft") }
                        .disabled(board.isEmpty || app.doc != nil || !paperHasRows(paper: app.paperCode))
                }
                Section(L("Condividi", "Share")) {
                    Button { app.printNow() } label: { Label(L("Stampa…", "Print…"), systemImage: "printer") }.disabled(board.isEmpty && app.doc == nil)
                    Button { shareURL = try? Export.pdf(board.toPage(Float(app.padSize.width), Float(app.padSize.height)), quarter: app.quarter) } label: { Label(L("Condividi PDF", "Share as PDF"), systemImage: "doc.richtext") }.disabled(board.isEmpty)
                    Button { shareURL = try? Export.png(board.toPage(Float(app.padSize.width), Float(app.padSize.height)), quarter: app.quarter) } label: { Label(L("Condividi immagine", "Share as image"), systemImage: "photo") }.disabled(board.isEmpty)
                    Button { copyBoard() } label: { Label(L("Copia negli appunti", "Copy to clipboard"), systemImage: "doc.on.doc") }.disabled(board.isEmpty || app.doc != nil)
                    Button { choosingMore = true } label: { Label(L("Altri formati…", "More formats…"), systemImage: "ellipsis.rectangle") }.disabled(board.isEmpty || app.doc != nil)
                }
                Section(L("Documento", "Document")) {
                    Button { importing = true } label: { Label(L("Apri un documento da firmare…", "Open a document to sign…"), systemImage: "signature") }
                }
            } label: { Image(systemName: "ellipsis.circle").font(.title3) }
        }
        .padding(.horizontal, 10).padding(.vertical, 6)
    }
}

struct DocBar: View {
    @EnvironmentObject var app: AppModel
    @ObservedObject var doc: OpenDoc
    @State private var shareURL: URL?
    @State private var exporting = false
    @State private var editing: EditStamps?

    struct EditStamps: Identifiable { let id = UUID(); let sel: Int? }

    var body: some View {
        HStack {
            Image(systemName: "doc.text")
            Button { app.goPage(-1) } label: { Image(systemName: "chevron.left") }.disabled(doc.index == 0)
            Text("\(doc.index + 1) / \(doc.count)").font(.callout.monospacedDigit())
            Button { app.goPage(1) } label: { Image(systemName: "chevron.right") }.disabled(doc.index + 1 >= doc.count)
            Spacer()
            Menu {
                ForEach(Array(app.stamps.enumerated()), id: \.offset) { i, s in
                    Button { app.startSeal(i) } label: { Label(s.name.isEmpty ? s.spec.company.name : s.name, systemImage: "seal") }
                }
                Divider()
                Button { editing = EditStamps(sel: app.stamps.isEmpty ? nil : 0) } label: {
                    Label(app.stamps.isEmpty ? L("Crea un timbro…", "Create a stamp…") : L("Crea o modifica i timbri…", "Create or edit stamps…"), systemImage: "pencil")
                }
            } label: { Label(L("Timbro", "Stamp"), systemImage: "seal") }
            .disabled(app.sealJob != nil)
            Button { app.printNow() } label: { Label(L("Stampa", "Print"), systemImage: "printer") }
            Button { shareURL = app.signedFile() } label: { Label(L("Condividi firmato", "Share signed"), systemImage: "square.and.arrow.up") }
            Button { app.closeDoc() } label: { Image(systemName: "xmark") }
        }
        .padding(.horizontal, 12).padding(.vertical, 6)
        .background(Color.accentColor.opacity(0.12))
        .sheet(item: $shareURL) { url in ShareSheet(items: [url]) }
        .sheet(item: $editing) { e in StampEditor(sel: e.sel).environmentObject(app) }
    }
}

struct ToolBar: View {
    @EnvironmentObject var app: AppModel
    @ObservedObject var board: Board
    @Binding var choosingPaper: Bool

    var body: some View {
        let hl = app.highlighterOn
        HStack(spacing: 10) {
            Picker("", selection: Binding(
                get: { app.eraserOn ? 2 : (hl ? 1 : 0) },
                set: { v in
                    app.eraserOn = v == 2
                    if v < 2 { app.highlighterOn = v == 1 }
                    app.applyTools()
                })) {
                Image(systemName: "pencil.tip").tag(0)
                Image(systemName: "highlighter").tag(1)
                Image(systemName: "eraser").tag(2)
            }
            .pickerStyle(.segmented)
            Menu {
                ForEach(hl ? highlighterColors : penColors, id: \.self) { c in
                    Button { if hl { app.hlColor = c } else { app.penColor = c }; app.eraserOn = false; app.applyTools() } label: {
                        Label(c, systemImage: "circle.fill").tint(Color(uiColor(c)))
                    }
                }
            } label: {
                Circle().fill(Color(uiColor(hl ? app.hlColor : app.penColor))).frame(width: 26, height: 26).overlay(Circle().stroke(Color.gray))
            }
            Menu {
                let range: [Double] = hl ? [8, 12, 16, 24, 32] : [0.8, 1.2, 2, 3.2, 5]
                ForEach(range, id: \.self) { w in
                    Button(String(format: "%.1f", w)) { if hl { app.hlWidth = w } else { app.penWidth = w }; app.applyTools() }
                }
            } label: { Image(systemName: "lineweight").font(.title3) }
            Button { choosingPaper = true } label: { Image(systemName: "squareshape.split.3x3").font(.title3) }
        }
        .padding(.horizontal, 12).padding(.vertical, 8)
        .background(.bar)
    }
}

struct PaperPicker: View {
    @EnvironmentObject var app: AppModel
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            List {
                let kinds = paperKinds()
                ForEach(["", "Righe", "Quadretti", "Matematica e disegno", "Altro"], id: \.self) { g in
                    Section(g.isEmpty ? "" : paperGroupName(g)) {
                        ForEach(kinds.filter { $0.group == g }, id: \.code) { k in
                            Button { app.paperCode = k.code; app.applyTools(); dismiss() } label: {
                                HStack { Text(paperName(k.code)).foregroundStyle(.primary); Spacer(); if app.paperCode == k.code { Image(systemName: "checkmark") } }
                            }
                        }
                    }
                }
                Toggle(L("Margini rossi sui fogli a righe", "Red margins on lined paper"), isOn: Binding(get: { app.paperMargin }, set: { app.paperMargin = $0; app.applyTools() }))
            }
            .navigationTitle(L("Foglio", "Paper"))
            .toolbar { Button("OK") { dismiss() } }
        }
    }
}

extension URL: @retroactive Identifiable { public var id: String { absoluteString } }

struct ShareSheet: UIViewControllerRepresentable {
    let items: [Any]
    func makeUIViewController(context: Context) -> UIActivityViewController { UIActivityViewController(activityItems: items, applicationActivities: nil) }
    func updateUIViewController(_ vc: UIActivityViewController, context: Context) {}
}
