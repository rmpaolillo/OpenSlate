import SwiftUI

/// I documenti da firmare (anche quelli arrivati dal computer), dal più recente.
struct DocsSheet: View {
    @EnvironmentObject var app: AppModel
    @Environment(\.dismiss) private var dismiss
    @State private var trashing: DocEntry?

    var body: some View {
        NavigationStack {
            let _ = app.docsVersion
            let docs = docList(root: OpenDoc.docsRoot.path)
            Group {
                if docs.isEmpty {
                    Text(L("Ancora nessun documento. Aprine uno da firmare dal menu: compare anche sul computer collegato.",
                           "No documents yet. Open one to sign from the menu: it also appears on the connected computer."))
                        .foregroundStyle(.secondary).padding()
                } else {
                    List(docs, id: \.id) { d in
                        Button {
                            app.openDocDir(OpenDoc.docsRoot.appendingPathComponent(d.id, isDirectory: true))
                            dismiss()
                        } label: {
                            VStack(alignment: .leading) {
                                Text(d.title).foregroundStyle(.primary)
                                Text(String(d.created.prefix(16)) + (app.doc?.id == d.id ? L(" · aperto ora", " · open now") : ""))
                                    .font(.caption).foregroundStyle(.secondary)
                            }
                        }
                        .swipeActions {
                            Button(role: .destructive) { trashing = d } label: { Label(L("Cestino", "Trash"), systemImage: "trash") }
                        }
                    }
                }
            }
            .navigationTitle(L("I miei documenti", "My documents"))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar { ToolbarItem(placement: .cancellationAction) { Button(L("Chiudi", "Close")) { dismiss() } } }
            .alert(L("Spostare nel cestino?", "Move to trash?"), isPresented: Binding(get: { trashing != nil }, set: { if !$0 { trashing = nil } })) {
                Button(L("Sposta nel cestino", "Move to trash"), role: .destructive) {
                    if let d = trashing { app.trashDoc(d.id) }
                    trashing = nil
                }
                Button(L("Annulla", "Cancel"), role: .cancel) { trashing = nil }
            } message: {
                Text((trashing?.title ?? "") + L(" sparisce anche dal computer collegato.", " also disappears from the connected computer."))
            }
        }
    }
}
