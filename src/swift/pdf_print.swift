// Stampa un PDF con il dialogo di stampa di macOS (PDFKit): stampante, copie,
// pagine, formato della carta, anteprima e «Salva come PDF».
//
// uso: pdf_print <file.pdf> <titolo del lavoro> [--to <file.pdf>]
// (`--to` scrive dove andrebbe la stampa, senza dialogo: serve alle prove)
// esce con 0 = stampato, 2 = annullato, 1 = errore.

import AppKit
import PDFKit

func fail(_ msg: String) -> Never {
    FileHandle.standardError.write((msg + "\n").data(using: .utf8)!)
    exit(1)
}

let args = CommandLine.arguments
guard args.count >= 3 else { fail("uso: pdf_print <file.pdf> <titolo> [--to <file.pdf>]") }
guard let doc = PDFDocument(url: URL(fileURLWithPath: args[1])) else { fail("PDF illeggibile: \(args[1])") }
let toFile: URL? = args.count >= 5 && args[3] == "--to" ? URL(fileURLWithPath: args[4]) : nil

let app = NSApplication.shared
// niente icona nel Dock: è solo il dialogo dell'app che stampa
app.setActivationPolicy(.accessory)

let info = NSPrintInfo.shared.copy() as! NSPrintInfo
info.isHorizontallyCentered = true
info.isVerticallyCentered = true
if let url = toFile {
    info.jobDisposition = .save
    info.dictionary()[NSPrintInfo.AttributeKey.jobSavingURL] = url
}
// la pagina intera nel foglio, ruotata se è orizzontale
guard let op = doc.printOperation(for: info, scalingMode: .pageScaleToFit, autoRotate: true) else { fail("stampa non disponibile") }
op.jobTitle = args[2]
op.showsPrintPanel = toFile == nil
op.showsProgressPanel = toFile == nil
op.printPanel.options.formUnion([.showsCopies, .showsPageRange, .showsPaperSize, .showsOrientation, .showsScaling, .showsPreview])
if toFile == nil {
    app.activate(ignoringOtherApps: true)
}
exit(op.run() ? 0 : 2)
