import SwiftUI
import UIKit

/// La firma che si sta posando: riquadro (u, v, larghezza, altezza) e anteprima.
struct SigJob {
    var rect: [Float]
    var art: UIImage
    /// Colore e intensità (restano per la prossima volta).
    var look: SigLook
}

/// La firma registrata, come sulla desktop (stesso file, si sincronizza):
/// si registra una volta scrivendo sul foglio, poi si posa sui documenti o sulla lavagna.
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
        // appena registrata la si posa, sul documento o sulla lavagna
        if save { sigStart() }
    }

    /// Posa la firma registrata: compare a grandezza naturale in basso a destra, poi si sposta.
    func sigStart() {
        let rect = sigNaturalRect(dataDir: dataDir, view: view, cu: 0.68, cv: 0.84)
        guard rect.count == 4 else { show(L("Nessuna firma: prima registrala.", "No signature yet: record it first.")); return }
        guard let img = sigArt(rect) else { return }
        sealJob = nil
        sigJob = SigJob(rect: rect, art: img, look: sigLook(dataDir: dataDir))
    }

    /// L'anteprima larga 600 px, col colore e l'intensità scelti.
    func sigArt(_ rect: [Float]) -> UIImage? {
        // il foglio visto, in quei pixel, è alto così
        let seenHOverW = quarter % 2 == 1 ? padSize.width / padSize.height : padSize.height / padSize.width
        return UIImage(data: sigPng(dataDir: dataDir, pxW: 600, pagePxH: Float(600 / CGFloat(rect[2]) * seenHOverW)))
    }

    /// Cambia colore o intensità della firma: restano per la prossima volta (e vanno al computer).
    func sigSetLookNow(_ l: SigLook) {
        guard var j = sigJob else { return }
        try? sigSetLook(dataDir: dataDir, look: l)
        j.look = l
        if let img = sigArt(j.rect) { j.art = img }
        sigJob = j
        sync.syncSoon(3)
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
    /// Colore e intensità: di solito chiusi, così il foglio resta grande.
    @State private var look = false
    private let inks = sigInks()

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(L("Posa la firma", "Place the signature")).font(.headline)
            Text(L("Tocca o trascina col dito dove la vuoi, tira l'angolo per ingrandirla, poi «Applica».", "Tap or drag where you want it, pull the corner to resize it, then «Apply»."))
                .font(.footnote).foregroundStyle(.secondary)
            if look, let l = app.sigJob?.look {
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 6) {
                        Button(L("Come l'hai scritta", "As written")) { var n = l; n.ink = -1; app.sigSetLookNow(n) }
                            .buttonStyle(.bordered).tint(l.ink < 0 ? .accentColor : .secondary).font(.caption)
                        ForEach(Array(inks.enumerated()), id: \.offset) { i, k in
                            Circle()
                                .fill(Color(red: Double(k.r) / 255, green: Double(k.g) / 255, blue: Double(k.b) / 255))
                                .frame(width: 22, height: 22)
                                .padding(3)
                                .overlay(Circle().stroke(Int32(i) == l.ink ? Color.primary : .clear, lineWidth: 2))
                                .accessibilityLabel(k.name)
                                .onTapGesture { var n = l; n.ink = Int32(i); app.sigSetLookNow(n) }
                        }
                    }
                }
                HStack {
                    Text(L("Intensità", "Intensity")).font(.caption).frame(width: 90, alignment: .leading)
                    Slider(value: Binding(get: { l.intensity }, set: { v in var n = l; n.intensity = v; app.sigSetLookNow(n) }), in: 0.3...1)
                    Text("\(Int((l.intensity * 100).rounded()))%").font(.caption).frame(width: 40)
                }
            }
            HStack {
                Button(L("Applica", "Apply")) { app.sigAccept() }.buttonStyle(.borderedProminent)
                Button { look.toggle() } label: {
                    Label(L("Aspetto", "Look"), systemImage: look ? "chevron.down" : "slider.horizontal.3")
                }.buttonStyle(.bordered)
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
