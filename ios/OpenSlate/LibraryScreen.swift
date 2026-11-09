import SwiftUI

struct NoteItem: Identifiable {
    let id: String
    let title: String
    let text: String
}

/// L'archivio: le pagine per giorno, dalla più recente (stesso formato della desktop).
struct LibraryScreen: View {
    @EnvironmentObject var app: AppModel

    var body: some View {
        NavigationStack {
            content
                .navigationTitle(L("Libreria", "Library"))
                .navigationDestination(for: String.self) { id in PageViewer(id: id) }
        }
    }

    @ViewBuilder
    private var content: some View {
        let _ = app.libVersion
        let pages = libraryList(dir: app.libDir)
        if pages.isEmpty {
            Text(L("Ancora nessuna pagina. Salva la lavagna, oppure scarica le pagine scritte sul pad lontano dal telefono.",
                   "No pages yet. Save the board, or download the pages written on the pad away from the phone."))
                .foregroundStyle(.secondary).padding()
        } else {
            let days: [String] = Array(Set(pages.map { $0.day })).sorted(by: >)
            List {
                ForEach(days, id: \.self) { day in
                    Section(dayLabel(day)) {
                        ForEach(pages.filter { $0.day == day }, id: \.id) { p in
                            NavigationLink(value: p.id) { PageRow(page: p) }
                        }
                    }
                }
            }
        }
    }

    private func dayLabel(_ day: String) -> String {
        let f = DateFormatter()
        f.dateFormat = "yyyy-MM-dd"
        guard let d = f.date(from: day) else { return day }
        let out = DateFormatter()
        out.dateStyle = .full
        return out.string(from: d)
    }
}

struct PageRow: View {
    let page: PageSummary

    var body: some View {
        let time = String(page.created.dropFirst(11).prefix(5))
        let source = page.source == "pad" ? L("dal pad", "from the pad") : L("lavagna", "board")
        VStack(alignment: .leading) {
            Text(page.title)
            Text(time + " · " + source).font(.caption).foregroundStyle(.secondary)
        }
    }
}

struct PageViewer: View {
    @EnvironmentObject var app: AppModel
    @Environment(\.dismiss) private var dismiss
    let id: String
    @State private var page: Page?
    @State private var shareURL: URL?
    @State private var renaming = false
    @State private var newTitle = ""
    @State private var deleting = false
    @State private var note: NoteItem?
    @State private var choosingMore = false

    var body: some View {
        VStack(spacing: 0) {
            if let p = page {
                Sheet(strokes: p.strokes, padW: p.width, padH: p.height, quarter: app.quarter, version: 0)
            }
        }
        .navigationTitle(page.map { $0.title.isEmpty ? titleFromDate($0.created) : $0.title } ?? "")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItemGroup(placement: .primaryAction) {
                Button { if let p = page { app.openOnBoard(p) } } label: { Image(systemName: "rectangle.and.pencil.and.ellipsis") }
                Button {
                    guard let p = page else { return }
                    let title = p.title.isEmpty ? titleFromDate(p.created) : p.title
                    if let n = app.noteOf(p.id) { note = NoteItem(id: p.id, title: title, text: n) }
                    else { app.transcribe(p) { text in note = NoteItem(id: p.id, title: title, text: text) } }
                } label: { Image(systemName: "text.viewfinder") }
                Menu {
                    Button { if let p = page { shareURL = try? Export.pdf(p, quarter: app.quarter) } } label: { Label(L("Condividi PDF", "Share as PDF"), systemImage: "doc.richtext") }
                    Button { if let p = page { shareURL = try? Export.png(p, quarter: app.quarter) } } label: { Label(L("Condividi immagine", "Share as image"), systemImage: "photo") }
                    Button { choosingMore = true } label: { Label(L("Altri formati…", "More formats…"), systemImage: "ellipsis.rectangle") }
                    Button { newTitle = page?.title ?? ""; renaming = true } label: { Label(L("Rinomina", "Rename"), systemImage: "pencil") }
                    Button(role: .destructive) { deleting = true } label: { Label(L("Elimina", "Delete"), systemImage: "trash") }
                } label: { Image(systemName: "ellipsis.circle") }
            }
        }
        .onAppear { page = try? libraryLoad(dir: app.libDir, id: id) }
        .sheet(item: $shareURL) { url in ShareSheet(items: [url]) }
        .sheet(item: $note) { n in NoteScreen(item: n) }
        .sheet(isPresented: $choosingMore) {
            MoreFormatsSheet { kind in
                if let p = page { app.exportMore(p, kind: kind, fromBoard: false) { url in shareURL = url } }
            }
        }
        .alert(L("Rinomina", "Rename"), isPresented: $renaming) {
            TextField(L("Titolo", "Title"), text: $newTitle)
            Button("OK") {
                try? libraryRename(dir: app.libDir, id: id, title: newTitle)
                page = try? libraryLoad(dir: app.libDir, id: id)
                app.libVersion += 1
            }
            Button(L("Annulla", "Cancel"), role: .cancel) {}
        }
        .alert(L("Spostare la pagina nel cestino?", "Move the page to the trash?"), isPresented: $deleting) {
            Button(L("Elimina", "Delete"), role: .destructive) {
                try? libraryTrash(dir: app.libDir, id: id)
                app.libVersion += 1
                dismiss()
            }
            Button(L("Annulla", "Cancel"), role: .cancel) {}
        }
    }

    private func titleFromDate(_ c: String) -> String {
        let a = Array(c)
        guard a.count >= 16 else { return c }
        return String(a[8..<10]) + "/" + String(a[5..<7]) + "/" + String(a[0..<4]) + " " + String(a[11..<16])
    }
}

/// La nota trascritta: si corregge, si salva accanto alla pagina, si condivide.
struct NoteScreen: View {
    @EnvironmentObject var app: AppModel
    @Environment(\.dismiss) private var dismiss
    let item: NoteItem
    @State private var text = ""

    var body: some View {
        NavigationStack {
            TextEditor(text: $text).font(.system(.body, design: .monospaced)).padding(8)
                .navigationTitle(item.title)
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) { Button(L("Chiudi", "Close")) { dismiss() } }
                    ToolbarItemGroup(placement: .primaryAction) {
                        ShareLink(item: text) { Image(systemName: "square.and.arrow.up") }
                        Button(L("Salva", "Save")) {
                            try? text.write(toFile: app.libDir + "/\(item.id).md", atomically: true, encoding: .utf8)
                            app.show(L("Nota salvata.", "Note saved."))
                            dismiss()
                        }
                    }
                }
        }
        .onAppear { text = item.text }
    }
}

/// Impostazioni, per argomento: AI, foglio, scrittura.
struct SettingsScreen: View {
    @EnvironmentObject var app: AppModel
    @State private var choosingPaper = false

    var body: some View {
        NavigationStack {
            List {
                Section(L("Intelligenza artificiale", "Artificial intelligence")) {
                    NavigationLink {
                        AiSettingsScreen(store: app.ai)
                    } label: {
                        let on = app.ai.services.filter { $0.enabled }.map { $0.name }
                        LabeledContent(L("Impostazioni AI", "AI settings"), value: on.isEmpty ? L("Nessun servizio", "No service yet") : on.joined(separator: ", "))
                    }
                }
                Section(L("Foglio", "Sheet")) {
                    Button { choosingPaper = true } label: { LabeledContent(L("Foglio", "Paper"), value: paperName(app.paperCode)) }
                    Button { app.quarter = (app.quarter + 1) % 4; app.applyTools() } label: { LabeledContent(L("Ruota", "Rotate"), value: "\(app.quarter * 90)°") }
                }
                Section {
                    Toggle(L("Tratto naturale", "Natural stroke"), isOn: Binding(get: { app.naturalInk }, set: { app.naturalInk = $0; app.applyTools() }))
                } header: { Text(L("Scrittura", "Writing")) } footer: {
                    Text(L("Lo spessore segue la pressione della penna e le punte si affusolano, come l'inchiostro vero. Spento: spessore sempre uguale, come un pennarello.",
                           "The thickness follows the pen pressure and the ends taper, like real ink. Off: always the same thickness, like a felt-tip marker."))
                }
                Section {
                    Toggle(L("Lo zig-zag cancella", "Zig-zag erases"), isOn: Binding(get: { app.zigzag }, set: { app.zigzag = $0; app.applyTools() }))
                } footer: {
                    Text(L("Uno zig-zag sopra lo scritto lo cancella (spento di serie: le firme sono spesso zig-zag).",
                           "A zig-zag over the writing erases it (off by default: signatures are often zig-zags)."))
                }
            }
            .navigationTitle(L("Impostazioni", "Settings"))
            .sheet(isPresented: $choosingPaper) { PaperPicker() }
        }
    }
}
