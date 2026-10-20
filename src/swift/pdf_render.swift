// Disegna le pagine di un PDF in immagini PNG con PDFKit (incluso in macOS).
// Uso: pdf_render <file.pdf> <cartella> <pixel del lato lungo>
// Scrive <cartella>/page-0001.png, page-0002.png, … (rotazione e CropBox applicati).

import AppKit
import Foundation
import PDFKit

func fail(_ msg: String) -> Never {
    FileHandle.standardError.write((msg + "\n").data(using: .utf8)!)
    exit(1)
}

let args = CommandLine.arguments
if args.count < 4 { fail("uso: pdf_render <file.pdf> <cartella> <pixel>") }
guard let doc = PDFDocument(url: URL(fileURLWithPath: args[1])) else {
    fail("PDF non leggibile: \(args[1])")
}
if doc.isLocked { fail("il PDF è protetto da password") }
let longPx = CGFloat(Double(args[3]) ?? 2000)

for i in 0..<doc.pageCount {
    guard let page = doc.page(at: i) else { continue }
    let box = page.bounds(for: .cropBox)
    var w = box.width
    var h = box.height
    if page.rotation % 180 != 0 { swap(&w, &h) }
    let s = longPx / max(w, h)
    let pw = max(1, Int((w * s).rounded()))
    let ph = max(1, Int((h * s).rounded()))
    guard let ctx = CGContext(
        data: nil, width: pw, height: ph, bitsPerComponent: 8, bytesPerRow: 0,
        space: CGColorSpaceCreateDeviceRGB(),
        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
    ) else { fail("memoria insufficiente per la pagina \(i + 1)") }
    ctx.setFillColor(CGColor(red: 1, green: 1, blue: 1, alpha: 1))
    ctx.fill(CGRect(x: 0, y: 0, width: pw, height: ph))
    ctx.scaleBy(x: s, y: s)
    page.transform(ctx, for: .cropBox)
    page.draw(with: .cropBox, to: ctx)
    guard let img = ctx.makeImage() else { fail("pagina \(i + 1) non disegnata") }
    let rep = NSBitmapImageRep(cgImage: img)
    guard let png = rep.representation(using: .png, properties: [:]) else {
        fail("PNG non creato per la pagina \(i + 1)")
    }
    let out = URL(fileURLWithPath: args[2]).appendingPathComponent(String(format: "page-%04d.png", i + 1))
    do { try png.write(to: out) } catch { fail("scrittura \(out.path): \(error.localizedDescription)") }
}
