import CoreGraphics
import UIKit

/// Inchiostro e spessore predefiniti (come la desktop).
let inkColor = "#141414"
let penDefaultWidth: Float = 2

/// I colori della desktop (`PALETTE` in ui.rs) e quelli dell'evidenziatore.
let penColors = ["#141414", "#195ac8", "#d22828", "#1e963c", "#f08c00", "#8c3cbe"]
let highlighterColors = ["#ffeb5a", "#a0f08c", "#ffa0d2", "#9bd2ff", "#ffc864"]

func uiColor(_ hex: String, alpha: CGFloat = 1) -> UIColor {
    var s = hex.trimmingCharacters(in: .whitespaces)
    if s.hasPrefix("#") { s.removeFirst() }
    let v = UInt32(s, radix: 16) ?? 0x141414
    return UIColor(red: CGFloat((v >> 16) & 0xff) / 255, green: CGFloat((v >> 8) & 0xff) / 255, blue: CGFloat(v & 0xff) / 255, alpha: alpha)
}

/// Disegna i tratti di una pagina (come `Ink.kt` dell'app Android): coordinate
/// del pad (`padW` x `padH`) ruotate di `quarter` quarti di giro e scalate su `w` x `h`.
enum Ink {
    static func rotated(_ padW: Float, _ padH: Float, _ quarter: Int) -> (Float, Float) {
        quarter % 2 == 1 ? (padH, padW) : (padW, padH)
    }

    static func draw(_ c: CGContext, _ strokes: [PageStroke], padW: Float, padH: Float, quarter: Int, w: CGFloat, h: CGFloat) {
        let (rw, rh) = rotated(padW, padH, quarter)
        let k = min(w / CGFloat(rw), h / CGFloat(rh))
        let unit = CGFloat(rh) * k / 800
        func map(_ p: PenPoint) -> CGPoint {
            let (x, y): (Float, Float)
            switch quarter {
            case 0: (x, y) = (p.x, p.y)
            case 1: (x, y) = (padH - p.y, p.x)
            case 2: (x, y) = (padW - p.x, padH - p.y)
            default: (x, y) = (p.y, padW - p.x)
            }
            return CGPoint(x: CGFloat(x) * k, y: CGFloat(y) * k)
        }
        c.saveGState()
        c.setLineCap(.round)
        c.setLineJoin(.round)
        // l'evidenziatore sta sotto l'inchiostro
        for pass in 0..<2 {
            for s in strokes where s.highlight == (pass == 0) && !s.points.isEmpty {
                let col = uiColor(s.color, alpha: s.highlight ? 0.35 : 1).cgColor
                c.setStrokeColor(col)
                c.setFillColor(col)
                let base = CGFloat(s.width) * unit
                let pts = s.points
                if pts.count == 1 {
                    let p = map(pts[0])
                    c.fillEllipse(in: CGRect(x: p.x - base / 2, y: p.y - base / 2, width: base, height: base))
                    continue
                }
                if s.highlight {
                    c.setLineWidth(base)
                    c.beginPath()
                    c.move(to: map(pts[0]))
                    for p in pts.dropFirst() { c.addLine(to: map(p)) }
                    c.strokePath()
                } else {
                    for i in 1..<pts.count {
                        let a = pts[i - 1], b = pts[i]
                        let pressure = b.phase == .up ? a.pressure : b.pressure
                        c.setLineWidth(base * CGFloat(0.4 + 0.8 * pressure))
                        c.beginPath()
                        c.move(to: map(a))
                        c.addLine(to: map(b))
                        c.strokePath()
                    }
                }
            }
        }
        c.restoreGState()
    }

    /// La rigatura (geometria da `src/rigatura.rs`, colori di `src/ui/paper.rs`).
    static func drawPaper(_ c: CGContext, _ marks: [PaperMark], kind: PaperKind, left: CGFloat, top: CGFloat, pxPerCm: CGFloat) {
        guard !marks.isEmpty else { return }
        let fade = { (cm: Float) -> CGFloat in min(max(CGFloat(cm) * pxPerCm / 8, 0), 1) }
        let fLine = fade(kind.pitch), fFine = fade(0.1)
        let mm = kind.code == "millimetrata"
        let rgb: (CGFloat, CGFloat, CGFloat) = mm ? (225, 130, 60) : kind.gridLike ? (110, 150, 205) : (80, 125, 200)
        func tint(_ a: CGFloat) -> CGColor { UIColor(red: rgb.0 / 255, green: rgb.1 / 255, blue: rgb.2 / 255, alpha: min(max(a, 0), 255) / 255).cgColor }
        let lineAlpha: CGFloat = mm ? 75 : kind.gridLike ? 95 : 130
        let strongAlpha: CGFloat = mm ? 140 : 175
        let axis = UIColor(red: 60 / 255, green: 70 / 255, blue: 95 / 255, alpha: 210 / 255).cgColor
        func style(_ ink: PaperInk) -> (CGFloat, CGColor)? {
            switch ink {
            case .fine: return fFine >= 0.35 ? (0.6, tint(80 * fFine)) : nil
            case .line: return fLine > 0.05 ? (1.0, tint(lineAlpha * fLine)) : nil
            case .strong: return (1.3, tint(strongAlpha * max(fLine, 0.5)))
            case .red: return (1.2, UIColor(red: 215 / 255, green: 70 / 255, blue: 70 / 255, alpha: 150 / 255).cgColor)
            case .axis: return (1.4, axis)
            }
        }
        let dotR = min(max(0.04 * pxPerCm, 0.8), 2.2)
        let font = UIFont.systemFont(ofSize: min(max(0.3 * pxPerCm, 9), 15))
        let labelColor = kind.code == "cartesiano" ? UIColor(cgColor: axis) : UIColor(cgColor: tint(200))
        c.saveGState()
        for m in marks {
            switch m {
            case .line(let ink, let x0, let y0, let x1, let y1):
                guard let (w, col) = style(ink) else { continue }
                c.setLineWidth(w)
                c.setStrokeColor(col)
                c.beginPath()
                c.move(to: CGPoint(x: left + CGFloat(x0) * pxPerCm, y: top + CGFloat(y0) * pxPerCm))
                c.addLine(to: CGPoint(x: left + CGFloat(x1) * pxPerCm, y: top + CGFloat(y1) * pxPerCm))
                c.strokePath()
            case .dot(let x, let y):
                guard fLine > 0.05 else { continue }
                c.setFillColor(tint(170 * fLine))
                c.fillEllipse(in: CGRect(x: left + CGFloat(x) * pxPerCm - dotR, y: top + CGFloat(y) * pxPerCm - dotR, width: 2 * dotR, height: 2 * dotR))
            case .label(let x, let y, let text):
                guard pxPerCm >= 12 else { continue }
                let s = NSAttributedString(string: paperLabel(text), attributes: [.font: font, .foregroundColor: labelColor])
                let size = s.size()
                UIGraphicsPushContext(c)
                s.draw(at: CGPoint(x: left + CGFloat(x) * pxPerCm - size.width / 2, y: top + CGFloat(y) * pxPerCm - size.height / 2))
                UIGraphicsPopContext()
            }
        }
        c.restoreGState()
    }
}

/// Esporta una pagina in PDF vettoriale o PNG (come `Export.kt`).
enum Export {
    static func size(_ page: Page, quarter: Int, long: CGFloat) -> CGSize {
        let (rw, rh) = Ink.rotated(page.width, page.height, quarter)
        let k = long / CGFloat(max(rw, rh))
        return CGSize(width: CGFloat(rw) * k, height: CGFloat(rh) * k)
    }

    private static func fileName(_ page: Page, _ ext: String) -> String {
        let bad = CharacterSet(charactersIn: "\\/:*?\"<>|\n\r\t")
        let t = page.title.trimmingCharacters(in: .whitespaces).components(separatedBy: bad).joined(separator: "_")
        return (t.isEmpty ? "OpenSlate-" + (page.id.isEmpty ? "pagina" : page.id) : String(t.prefix(80))) + "." + ext
    }

    static func shareDir() -> URL {
        let d = FileManager.default.temporaryDirectory.appendingPathComponent("share", isDirectory: true)
        try? FileManager.default.removeItem(at: d)
        try? FileManager.default.createDirectory(at: d, withIntermediateDirectories: true)
        return d
    }

    static func pdf(_ page: Page, quarter: Int) throws -> URL {
        let s = size(page, quarter: quarter, long: 842)
        let url = shareDir().appendingPathComponent(fileName(page, "pdf"))
        let r = UIGraphicsPDFRenderer(bounds: CGRect(origin: .zero, size: s))
        try r.writePDF(to: url) { ctx in
            ctx.beginPage()
            Ink.draw(ctx.cgContext, page.strokes, padW: page.width, padH: page.height, quarter: quarter, w: s.width, h: s.height)
        }
        return url
    }

    static func pngData(_ page: Page, quarter: Int, long: CGFloat = 1600) -> Data {
        let s = size(page, quarter: quarter, long: long)
        let format = UIGraphicsImageRendererFormat()
        format.scale = 1
        let r = UIGraphicsImageRenderer(size: s, format: format)
        return r.pngData { ctx in
            UIColor.white.setFill()
            ctx.fill(CGRect(origin: .zero, size: s))
            Ink.draw(ctx.cgContext, page.strokes, padW: page.width, padH: page.height, quarter: quarter, w: s.width, h: s.height)
        }
    }

    static func png(_ page: Page, quarter: Int) throws -> URL {
        let url = shareDir().appendingPathComponent(fileName(page, "png"))
        try pngData(page, quarter: quarter, long: 2400).write(to: url)
        return url
    }
}
