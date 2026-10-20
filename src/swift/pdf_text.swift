// Testo di un PDF con PDFKit (incluso in macOS), pagina per pagina.
// Uso: pdf_text <file.pdf> <file.txt>
// Scrive il testo in <file.txt> (UTF-8); le pagine sono separate dal
// carattere «form feed» (\u{0C}), come pdftotext.

import Foundation
import PDFKit

func fail(_ msg: String) -> Never {
    FileHandle.standardError.write((msg + "\n").data(using: .utf8)!)
    exit(1)
}

let args = CommandLine.arguments
if args.count < 3 { fail("uso: pdf_text <file.pdf> <file.txt>") }
guard let doc = PDFDocument(url: URL(fileURLWithPath: args[1])) else {
    fail("PDF non leggibile: \(args[1])")
}
if doc.isLocked { fail("il PDF è protetto da password") }
var out = ""
for i in 0..<doc.pageCount {
    if i > 0 { out += "\u{0C}" }
    out += doc.page(at: i)?.string ?? ""
}
do { try out.write(toFile: args[2], atomically: true, encoding: .utf8) } catch { fail("scrittura \(args[2]): \(error.localizedDescription)") }
