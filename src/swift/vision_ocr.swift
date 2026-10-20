// Riconoscimento della scrittura a mano con Apple Vision (offline, incluso in macOS).
// Uso: vision_ocr <immagine.png> [lingue separate da virgola, es. it-IT,en-US]
// Stampa il testo riconosciuto, una riga per riga di scrittura, dall'alto in basso.

import AppKit
import Foundation
import Vision

func fail(_ msg: String) -> Never {
    FileHandle.standardError.write((msg + "\n").data(using: .utf8)!)
    exit(1)
}

let args = CommandLine.arguments
if args.count < 2 { fail("uso: vision_ocr <immagine.png> [lingue]") }
var langs = ["it-IT", "en-US"]
if args.count >= 3 {
    langs = args[2].split(separator: ",").map { String($0).trimmingCharacters(in: .whitespaces) }
}

guard let image = NSImage(contentsOf: URL(fileURLWithPath: args[1])),
      let cg = image.cgImage(forProposedRect: nil, context: nil, hints: nil)
else { fail("immagine non leggibile: \(args[1])") }

let request = VNRecognizeTextRequest()
request.recognitionLevel = .accurate
request.usesLanguageCorrection = true
if let supported = try? request.supportedRecognitionLanguages() {
    let ok = langs.filter { supported.contains($0) }
    request.recognitionLanguages = ok.isEmpty ? ["en-US"] : ok
} else {
    request.recognitionLanguages = langs
}

do {
    try VNImageRequestHandler(cgImage: cg, options: [:]).perform([request])
} catch {
    fail("Vision: \(error.localizedDescription)")
}

let observations = request.results ?? []

// Raggruppa in righe (Vision ha l'origine in basso a sinistra).
struct Item { let text: String; let box: CGRect }
let items = observations.compactMap { o -> Item? in
    guard let c = o.topCandidates(1).first else { return nil }
    return Item(text: c.string, box: o.boundingBox)
}.sorted { $0.box.midY > $1.box.midY }

var lines: [[Item]] = []
for it in items {
    if var last = lines.last, let ref = last.first,
       abs(ref.box.midY - it.box.midY) < max(ref.box.height, it.box.height) * 0.5 {
        last.append(it)
        lines[lines.count - 1] = last
    } else {
        lines.append([it])
    }
}
for line in lines {
    print(line.sorted { $0.box.minX < $1.box.minX }.map { $0.text }.joined(separator: " "))
}
