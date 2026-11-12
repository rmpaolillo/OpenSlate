import SwiftUI
import UIKit

/// La firma che si sta posando: riquadro (u, v, larghezza, altezza) e anteprima.
struct SigJob {
    var rect: [Float]
    let art: UIImage
}

/// La firma registrata, come sulla desktop (stesso file, si sincronizza):
/// si registra una volta scrivendo sul foglio, poi si posa sui documenti.
extension AppModel {
    func sigCaptureStart() {
        if let d = doc { keepDoc(d) }
        sealJob = nil
        sigJob = nil
        sigCapture = board.model.strokes()
        board.openStrokes([])
        show(L("Firma sul pad come su un foglio, poi «Salva firma».", "Sign on the pad as on paper, then «Save signature»."))
    }

    func sigCaptureEnd(save: Bool) {
        guard let stash = sigCapture else { return }
        if save {
            do {
                try sigSave(dataDir: dataDir, strokes: board.model.strokes(), view: view)
            } catch {
                if case StoreError.Io(let reason) = error { show(reason) } else { show("\(error)") }
                return
            }
            show(L("Firma salvata ✓", "Signature saved ✓"))
            sync.syncSoon(3)
        }
        sigCapture = nil
        if let d = doc { d.show(d.index, board) } else { board.openStrokes(stash) }
        if save && doc != nil { sigStart() }
    }

    /// Posa la firma registrata: compare a grandezza naturale in basso a destra, poi si sposta.
    func sigStart() {
        guard doc != nil else { return }
        let rect = sigNaturalRect(dataDir: dataDir, view: view, cu: 0.68, cv: 0.84)
        guard rect.count == 4 else { show(L("Nessuna firma: prima registrala.", "No signature yet: record it first.")); return }
        // l'anteprima larga 600 px: il foglio visto, in quei pixel, è alto così
        let seenHOverW = quarter % 2 == 1 ? padSize.width / padSize.height : padSize.height / padSize.width
        let png = sigPng(dataDir: dataDir, pxW: 600, pagePxH: Float(600 / CGFloat(rect[2]) * seenHOverW))
        guard let img = UIImage(data: png) else { return }
        sealJob = nil
        sigJob = SigJob(rect: rect, art: img)
    }

    func sigAccept() {
        guard let j = sigJob else { return }
        if boardAddSignature(board: board.model, view: view, dataDir: dataDir, rect: j.rect) {
            board.reload()
            show(L("Firma posata: Annulla la toglie.", "Signature placed: Undo removes it."))
        }
        sigJob = nil
    }
}

struct SigBar: View {
    @EnvironmentObject var app: AppModel

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(L("Posa la firma", "Place the signature")).font(.headline)
            Text(L("Trascinala dove serve, tira l'angolo per ingrandirla, poi «Applica».", "Drag it where it belongs, pull the corner to resize it, then «Apply»."))
                .font(.footnote).foregroundStyle(.secondary)
            HStack {
                Button(L("Applica", "Apply")) { app.sigAccept() }.buttonStyle(.borderedProminent)
                Button(L("Annulla", "Cancel")) { app.sigJob = nil }
            }
        }
        .padding(12).frame(maxWidth: .infinity, alignment: .leading).background(.bar)
    }
}

struct SigCaptureBar: View {
    @EnvironmentObject var app: AppModel
    @ObservedObject var board: Board

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(L("Registra la firma", "Record the signature")).font(.headline)
            Text(L("Firma sul pad come su un foglio, poi «Salva firma». Va anche sul computer collegato.", "Sign on the pad as on paper, then «Save signature». It also goes to the connected computer."))
                .font(.footnote).foregroundStyle(.secondary)
            HStack {
                Button(L("Salva firma", "Save signature")) { app.sigCaptureEnd(save: true) }.buttonStyle(.borderedProminent).disabled(board.isEmpty)
                Button(L("Pulisci", "Clear")) { board.clear() }
                Button(L("Annulla", "Cancel")) { app.sigCaptureEnd(save: false) }
            }
        }
        .padding(12).frame(maxWidth: .infinity, alignment: .leading).background(.bar)
    }
}
