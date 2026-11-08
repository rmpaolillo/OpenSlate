import AVFoundation
import CoreGraphics
import SwiftUI
import UIKit

/// «Altri formati» (come la desktop): li scrive il cuore Rust con gli stessi
/// file della desktop; il video MP4 lo codifica AVFoundation.
extension Export {
    static func more(_ page: Page, quarter: Int, kind: ExportKind, note: String?, paper: String) throws -> URL {
        let url = shareDir().appendingPathComponent(fileName(page, exportExt(kind: kind)))
        if kind == .video {
            try Mp4.write(page, quarter: quarter, to: url)
        } else {
            try exportPage(page: page, quarter: UInt32(quarter), kind: kind, note: note, paper: paper, out: url.path)
        }
        return url
    }
}

/// La pagina che si riscrive da sola, in H.264 (stessi tempi di `src/video.rs`).
enum Mp4 {
    struct Failed: LocalizedError {
        let errorDescription: String?
    }

    static func write(_ page: Page, quarter: Int, to url: URL) throws {
        let timing = videoTiming()
        let (drawS, holdS, fps) = (timing[0], timing[1], Int32(timing[2]))
        let (rw, rh) = Ink.rotated(page.width, page.height, quarter)
        let k = 1080 / max(rw, rh)
        let w = (Int(rw * k) / 16) * 16
        let h = (Int(rh * k) / 16) * 16
        try? FileManager.default.removeItem(at: url)
        let writer = try AVAssetWriter(outputURL: url, fileType: .mp4)
        let input = AVAssetWriterInput(mediaType: .video, outputSettings: [
            AVVideoCodecKey: AVVideoCodecType.h264,
            AVVideoWidthKey: w,
            AVVideoHeightKey: h,
        ])
        input.expectsMediaDataInRealTime = false
        let adaptor = AVAssetWriterInputPixelBufferAdaptor(assetWriterInput: input, sourcePixelBufferAttributes: [
            kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA,
            kCVPixelBufferWidthKey as String: w,
            kCVPixelBufferHeightKey as String: h,
        ])
        writer.add(input)
        guard writer.startWriting() else { throw Failed(errorDescription: writer.error?.localizedDescription ?? "video") }
        writer.startSession(atSourceTime: .zero)
        let draw = Int((drawS * Float(fps)).rounded())
        let total = Int(((drawS + holdS) * Float(fps)).rounded())
        var last: CVPixelBuffer?
        for f in 0..<total {
            var buf = last
            if f <= draw || buf == nil {
                let t = min(Float(f) / Float(draw), 1)
                let p = animPageAt(page: page, quarter: UInt32(quarter), t: t)
                guard let pool = adaptor.pixelBufferPool else { throw Failed(errorDescription: writer.error?.localizedDescription ?? "video") }
                var out: CVPixelBuffer?
                CVPixelBufferPoolCreatePixelBuffer(nil, pool, &out)
                guard let pb = out else { throw Failed(errorDescription: "video") }
                CVPixelBufferLockBaseAddress(pb, [])
                if let ctx = CGContext(data: CVPixelBufferGetBaseAddress(pb), width: w, height: h, bitsPerComponent: 8,
                                       bytesPerRow: CVPixelBufferGetBytesPerRow(pb), space: CGColorSpaceCreateDeviceRGB(),
                                       bitmapInfo: CGImageAlphaInfo.premultipliedFirst.rawValue | CGBitmapInfo.byteOrder32Little.rawValue) {
                    ctx.setFillColor(UIColor.white.cgColor)
                    ctx.fill(CGRect(x: 0, y: 0, width: w, height: h))
                    // come UIKit: origine in alto a sinistra
                    ctx.translateBy(x: 0, y: CGFloat(h))
                    ctx.scaleBy(x: 1, y: -1)
                    Ink.draw(ctx, p.strokes, padW: page.width, padH: page.height, quarter: quarter, w: CGFloat(w), h: CGFloat(h))
                }
                CVPixelBufferUnlockBaseAddress(pb, [])
                buf = pb
                last = pb
            }
            while !input.isReadyForMoreMediaData { Thread.sleep(forTimeInterval: 0.005) }
            if !adaptor.append(buf!, withPresentationTime: CMTime(value: CMTimeValue(f), timescale: fps)) {
                throw Failed(errorDescription: writer.error?.localizedDescription ?? "video")
            }
        }
        input.markAsFinished()
        let done = DispatchSemaphore(value: 0)
        writer.finishWriting { done.signal() }
        done.wait()
        if writer.status != .completed { throw Failed(errorDescription: writer.error?.localizedDescription ?? "video") }
    }
}

extension ExportKind {
    var label: String {
        switch self {
        case .svg: return L("SVG (immagine vettoriale)", "SVG (vector image)")
        case .jpeg: return L("JPEG (leggero, da mandare)", "JPEG (light, for sending)")
        case .webp: return L("WebP (leggero, da mandare)", "WebP (light, for sending)")
        case .word: return L("Word, con l'inchiostro modificabile", "Word, with editable ink")
        case .xournal: return "Xournal++ (.xopp)"
        case .gif: return L("GIF animata: la scrittura che si rifà", "Animated GIF: the writing redraws itself")
        case .svgAnim: return L("SVG animato: la scrittura che si rifà", "Animated SVG: the writing redraws itself")
        case .video: return L("Video MP4: la scrittura che si rifà", "MP4 video: the writing redraws itself")
        case .latex: return L("LaTeX dalla trascrizione (.tex)", "LaTeX from the transcription (.tex)")
        case .typst: return L("Typst dalla trascrizione (.typ)", "Typst from the transcription (.typ)")
        }
    }

    var icon: String {
        switch self {
        case .svg, .jpeg, .webp: return "photo"
        case .word: return "doc.text"
        case .xournal: return "book.closed"
        case .gif, .svgAnim: return "film.stack"
        case .video: return "video"
        case .latex, .typst: return "function"
        }
    }
}

/// «Altri formati»: si sceglie, poi il Condividi di iOS (Mail, Messaggi,
/// «Salva in File» per una cartella a scelta…).
struct MoreFormatsSheet: View {
    @Environment(\.dismiss) private var dismiss
    let onPick: (ExportKind) -> Void

    var body: some View {
        NavigationStack {
            List(exportKinds(), id: \.self) { k in
                Button { dismiss(); onPick(k) } label: { Label(k.label, systemImage: k.icon) }
            }
            .navigationTitle(L("Altri formati", "More formats"))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar { ToolbarItem(placement: .cancellationAction) { Button(L("Chiudi", "Close")) { dismiss() } } }
        }
        .presentationDetents([.medium, .large])
    }
}

extension AppModel {
    /// Esporta in uno degli «Altri formati». LaTeX e Typst vengono dalla
    /// trascrizione: se manca, prima si trascrive la pagina.
    func exportMore(_ start: Page, kind: ExportKind, fromBoard: Bool, done: @escaping (URL) -> Void) {
        var page = start
        if fromBoard && (exportNeedsNote(kind: kind) || kind == .word) && (board.id.isEmpty || board.dirty) {
            // la nota sta accanto alla pagina: prima la si salva
            if saveBoard() { page = board.toPage(Float(padSize.width), Float(padSize.height)) }
        }
        let note = page.id.isEmpty ? nil : noteOf(page.id)
        if exportNeedsNote(kind: kind) && note == nil {
            guard !page.id.isEmpty else { return }
            let p = page
            transcribe(p) { [weak self] text in self?.runExport(p, kind: kind, note: text, done: done) }
            return
        }
        runExport(page, kind: kind, note: note, done: done)
    }

    private func runExport(_ page: Page, kind: ExportKind, note: String?, done: @escaping (URL) -> Void) {
        busy = L("Esportazione…", "Exporting…")
        let (q, paper) = (quarter, paperCode)
        DispatchQueue.global(qos: .userInitiated).async {
            let r = Result { try Export.more(page, quarter: q, kind: kind, note: note, paper: paper) }
            DispatchQueue.main.async { [self] in
                busy = nil
                switch r {
                case .success(let url): done(url)
                case .failure(let e): show(L("Esportazione non riuscita: ", "Export failed: ") + aiMessage(e))
                }
            }
        }
    }
}
