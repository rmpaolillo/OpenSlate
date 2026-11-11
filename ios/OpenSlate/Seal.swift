import SwiftUI
import UIKit

/// Il timbro aziendale (come la desktop e l'app Android): i timbri si
/// costruiscono una volta, si posano sul documento, si rendono realistici
/// (colore, grana, inchiostro irregolare, inclinazione) e si confermano.
/// L'immagine la fa il cuore Rust (`src/stamp.rs`).
struct SealJob {
    var spec: StampSpec
    /// Quale dell'elenco (per ricordarne le regolazioni).
    var saved: Int?
    /// u, v, larghezza, altezza del foglio visto.
    var rect: [Float]
}

enum SealArt {
    private static var cache: [String: UIImage] = [:]

    /// L'immagine del timbro per un riquadro largo `pxW` pixel (a scalini: si rifà di rado).
    static func image(_ spec: StampSpec, quarter: Int, pxW: CGFloat) -> UIImage? {
        let step = Int((min(max(pxW, 64), 2400) / 160).rounded(.up)) * 160
        let key = "\(spec.hashValue)-\(quarter)-\(step)"
        if let img = cache[key] { return img }
        let png = stampPng(spec: spec, pxW: Float(step), quarter: UInt8(quarter))
        guard let img = UIImage(data: png) else { return nil }
        if cache.count > 12 { cache.removeAll() }
        cache[key] = img
        return img
    }

    /// Disegna il timbro nel riquadro, in «moltiplica» come l'inchiostro vero.
    static func draw(_ c: CGContext, _ spec: StampSpec, quarter: Int, in r: CGRect) {
        guard r.width > 1, let img = image(spec, quarter: quarter, pxW: r.width * abs(c.ctm.a)) else { return }
        c.saveGState()
        c.setBlendMode(.multiply)
        UIGraphicsPushContext(c)
        img.draw(in: r, blendMode: .multiply, alpha: 1)
        UIGraphicsPopContext()
        c.restoreGState()
    }

    /// Un timbro posato (`Ink.draw`): `w` x `h` è il foglio visto.
    static func drawPlaced(_ c: CGContext, _ s: PageStroke, padW: Float, padH: Float, quarter: Int, w: CGFloat, h: CGFloat) {
        guard let spec = s.stamp,
              let p = stampPlace(points: s.points, view: BoardView(quarter: UInt32(quarter), padW: padW, padH: padH)),
              p.rect.count == 4 else { return }
        let r = p.rect
        let rect = CGRect(x: CGFloat(r[0]) * w, y: CGFloat(r[1]) * h, width: CGFloat(r[2] - r[0]) * w, height: CGFloat(r[3] - r[1]) * h)
        draw(c, spec, quarter: Int(p.quarter), in: rect)
    }
}

func inkColorOf(_ c: InkColor) -> Color {
    switch c {
    case .blue: return Color(red: 0.13, green: 0.24, blue: 0.59)
    case .grey: return Color(red: 0.36, green: 0.37, blue: 0.39)
    case .black: return Color(red: 0.13, green: 0.13, blue: 0.14)
    case .red: return Color(red: 0.70, green: 0.15, blue: 0.17)
    case .violet: return Color(red: 0.38, green: 0.20, blue: 0.56)
    }
}

func inkColorName(_ c: InkColor) -> String {
    switch c {
    case .blue: return L("Blu", "Blue")
    case .grey: return L("Grigio", "Grey")
    case .black: return L("Nero", "Black")
    case .red: return L("Rosso", "Red")
    case .violet: return L("Viola", "Violet")
    }
}

func frameName(_ f: Frame) -> String {
    switch f {
    case .plain: return L("Senza cornice", "No frame")
    case .rect: return L("Rettangolo", "Rectangle")
    case .rounded: return L("Angoli arrotondati", "Rounded corners")
    case .double: return L("Doppia linea", "Double line")
    case .oval: return L("Ovale", "Oval")
    }
}

let allInkColors: [InkColor] = [.blue, .grey, .black, .red, .violet]
let allFrames: [Frame] = [.plain, .rect, .rounded, .double, .oval]

struct InkDots: View {
    let selected: InkColor
    let onPick: (InkColor) -> Void
    var body: some View {
        HStack(spacing: 10) {
            ForEach(allInkColors, id: \.self) { c in
                Circle().fill(inkColorOf(c)).frame(width: 22, height: 22)
                    .overlay(Circle().stroke(Color.primary, lineWidth: c == selected ? 2 : 0).padding(-3))
                    .onTapGesture { onPick(c) }
                    .accessibilityLabel(inkColorName(c))
            }
        }
    }
}

struct FramePicker: View {
    let selected: Frame
    let onPick: (Frame) -> Void
    var body: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 6) {
                ForEach(allFrames, id: \.self) { f in
                    Button(frameName(f)) { onPick(f) }
                        .buttonStyle(.bordered)
                        .tint(f == selected ? .accentColor : .secondary)
                        .controlSize(.small)
                }
            }
        }
    }
}

extension AppModel {
    var stampDir: String {
        FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0].path
    }

    /// Salva il timbro (nuovo o al posto `sel`); restituisce dove sta nell'elenco.
    func saveStamp(_ sel: Int?, _ s: SavedStamp) -> Int {
        var named = s
        if named.name.trimmingCharacters(in: .whitespaces).isEmpty { named.name = s.spec.company.name.trimmingCharacters(in: .whitespaces) }
        var list = stamps
        let i: Int
        if let sel, list.indices.contains(sel) { list[sel] = named; i = sel } else { list.append(named); i = list.count - 1 }
        stamps = list
        do { try stampSave(dir: stampDir, list: list) } catch { show("\(error)") }
        return i
    }

    func deleteStamp(_ i: Int) {
        guard stamps.indices.contains(i) else { return }
        stamps.remove(at: i)
        try? stampSave(dir: stampDir, list: stamps)
    }

    /// Posa il timbro n. `i`: compare a grandezza naturale, poi si regola.
    func startSeal(_ i: Int) {
        guard doc != nil, stamps.indices.contains(i) else { return }
        let spec = stamps[i].spec
        eraserOn = false
        applyTools()
        sealJob = SealJob(spec: spec, saved: i, rect: stampNaturalRect(spec: spec, view: view, cu: 0.3, cv: 0.84))
    }

    func sealEdit(_ spec: StampSpec) {
        guard let j = sealJob else { return }
        sealJob = SealJob(spec: spec, saved: j.saved, rect: stampRefit(before: j.spec, after: spec, view: view, rect: j.rect))
    }

    /// Conferma: il timbro sul foglio (un passo di annulla); le regolazioni restano per la prossima volta.
    func sealAccept() {
        guard let j = sealJob else { return }
        boardAddStamp(board: board.model, view: view, spec: j.spec, rect: j.rect)
        board.reload()
        if let i = j.saved, stamps.indices.contains(i) {
            stamps[i].spec.style = j.spec.style
            try? stampSave(dir: stampDir, list: stamps)
        }
        sealJob = nil
        show(L("Timbro apposto: Annulla lo toglie.", "Stamp applied: Undo removes it."))
    }
}

/// La finestra dei timbri: l'elenco, i dati dell'azienda, cornice e colore, l'anteprima.
struct StampEditor: View {
    @EnvironmentObject var app: AppModel
    @Environment(\.dismiss) private var dismiss
    @State var sel: Int?
    @State private var draft: SavedStamp = stampNew()
    @State private var loaded = false
    @State private var preview: UIImage?

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    ScrollView(.horizontal, showsIndicators: false) {
                        HStack {
                            ForEach(Array(app.stamps.enumerated()), id: \.offset) { i, s in
                                Button(s.name.isEmpty ? s.spec.company.name : s.name) { sel = i; draft = s }
                                    .buttonStyle(.bordered).tint(sel == i ? .accentColor : .secondary)
                            }
                            Button(L("Nuovo timbro", "New stamp")) { sel = nil; draft = stampNew() }
                                .buttonStyle(.bordered).tint(sel == nil ? .accentColor : .secondary)
                        }
                    }
                }
                Section(L("Dati dell'azienda", "Company details")) {
                    TextField(L("Nome del timbro", "Stamp name"), text: $draft.name)
                    TextField(L("Ragione sociale", "Company name"), text: $draft.spec.company.name)
                    TextField(L("Indirizzo", "Address"), text: $draft.spec.company.address)
                    HStack {
                        TextField(L("CAP", "Postcode"), text: $draft.spec.company.cap).frame(maxWidth: 80)
                        TextField(L("Città", "City"), text: $draft.spec.company.city)
                        TextField(L("Prov.", "Prov."), text: $draft.spec.company.prov).frame(maxWidth: 50)
                    }
                    TextField(L("P. IVA", "VAT number"), text: $draft.spec.company.vat)
                    TextField(L("REA (se c'è)", "REA (if any)"), text: $draft.spec.company.rea)
                    TextField(L("Codice univoco (se c'è)", "Recipient code (if any)"), text: $draft.spec.company.sdi)
                    TextField(L("PEC (se c'è)", "Certified email (PEC, if any)"), text: $draft.spec.company.pec)
                        .keyboardType(.emailAddress).textInputAutocapitalization(.never).autocorrectionDisabled()
                }
                Section(L("Aspetto", "Look")) {
                    FramePicker(selected: draft.spec.style.frame) { draft.spec.style.frame = $0 }
                    InkDots(selected: draft.spec.style.color) { draft.spec.style.color = $0 }
                    if let img = preview {
                        Image(uiImage: img).resizable().scaledToFit().frame(maxHeight: 160).frame(maxWidth: .infinity)
                            .padding(8).background(Color.white).clipShape(RoundedRectangle(cornerRadius: 8))
                    }
                    let size = stampSizeCm(spec: draft.spec)
                    Text(Lf("Grandezza vera: %.1f × %.1f cm. Grana, intensità e inclinazione si regolano dopo averlo posato sul documento.",
                            "Real size: %.1f × %.1f cm. Grain, intensity and tilt are adjusted after placing it on the document.", Double(size[0]), Double(size[1])))
                        .font(.caption).foregroundStyle(.secondary)
                }
                Section {
                    let ok = !draft.spec.company.name.trimmingCharacters(in: .whitespaces).isEmpty
                    Button(L("Salva", "Save")) { sel = app.saveStamp(sel, draft) }.disabled(!ok)
                    Button(L("Apponi sul documento", "Place on the document")) {
                        let i = app.saveStamp(sel, draft)
                        dismiss()
                        app.startSeal(i)
                    }.disabled(!ok || app.doc == nil)
                    if let i = sel {
                        Button(L("Elimina", "Delete"), role: .destructive) { app.deleteStamp(i); sel = nil; draft = stampNew() }
                    }
                }
            }
            .navigationTitle(L("Timbri aziendali", "Company stamps"))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar { ToolbarItem(placement: .cancellationAction) { Button(L("Chiudi", "Close")) { dismiss() } } }
            // l'anteprima in un thread, poco dopo l'ultimo tasto: scrivere resta svelto
            .task(id: draft.spec) {
                if preview != nil { try? await Task.sleep(nanoseconds: 250_000_000) }
                if Task.isCancelled { return }
                let spec = draft.spec
                let png = await Task.detached { stampPng(spec: spec, pxW: 700, quarter: 0) }.value
                if !Task.isCancelled, let img = UIImage(data: png) { preview = img }
            }
            .onAppear {
                guard !loaded else { return }
                loaded = true
                if let i = sel, app.stamps.indices.contains(i) { draft = app.stamps[i] } else { sel = nil }
            }
        }
    }
}

/// La barra mentre si posa un timbro, al posto degli strumenti.
struct SealBar: View {
    @EnvironmentObject var app: AppModel

    var body: some View {
        if let j = app.sealJob {
            let st = j.spec.style
            VStack(alignment: .leading, spacing: 4) {
                Text(L("Timbro aziendale", "Company stamp")).font(.subheadline.bold())
                Text(L("Trascina il timbro col dito, tira l'angolo per la grandezza, rendilo realistico, poi Applica.",
                       "Drag the stamp with a finger, pull the corner to resize it, make it look real, then Apply."))
                    .font(.caption).foregroundStyle(.secondary)
                InkDots(selected: st.color) { c in var s = j.spec; s.style.color = c; app.sealEdit(s) }
                FramePicker(selected: st.frame) { f in var s = j.spec; s.style.frame = f; app.sealEdit(s) }
                slider(L("Intensità", "Intensity"), st.opacity, 0.3...1) { v in var s = j.spec; s.style.opacity = v; app.sealEdit(s) }
                slider(L("Grana", "Grain"), st.grain, 0...1) { v in var s = j.spec; s.style.grain = v; app.sealEdit(s) }
                slider(L("Inchiostro irregolare", "Uneven ink"), st.uneven, 0...1) { v in var s = j.spec; s.style.uneven = v; app.sealEdit(s) }
                slider(L("Inclinazione", "Tilt"), st.tilt, -15...15) { v in var s = j.spec; s.style.tilt = v; app.sealEdit(s) }
                HStack {
                    Button(L("Applica", "Apply")) { app.sealAccept() }.buttonStyle(.borderedProminent)
                    Button(L("Nuova grana", "New grain")) { app.sealEdit(stampReseed(spec: j.spec)) }.buttonStyle(.bordered)
                    Spacer()
                    Button(L("Annulla", "Cancel"), role: .cancel) { app.sealJob = nil }
                }
            }
            .padding(.horizontal, 12).padding(.vertical, 8)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(.bar)
        }
    }

    private func slider(_ label: String, _ v: Float, _ range: ClosedRange<Float>, _ set: @escaping (Float) -> Void) -> some View {
        HStack {
            Text(label).font(.caption).frame(width: 130, alignment: .leading)
            Slider(value: Binding(get: { v }, set: set), in: range)
        }
    }
}
