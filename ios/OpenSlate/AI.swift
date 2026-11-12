import Foundation
import Security
import SwiftUI
import VisionKit

/// Un servizio AI nelle impostazioni (la chiave sta nel Portachiavi).
struct AiService: Codable, Identifiable, Equatable {
    var id = UUID().uuidString
    var name: String
    var url: String
    var model = ""
    var vision = true
    var enabled = true
    var noTemperature = false
    var maxTokens: UInt32?
    var timeoutS: UInt32 = 180
}

enum AiPresets {
    static let all: [AiService] = [
        AiService(name: "Anthropic Claude", url: "https://api.anthropic.com/v1", model: "claude-sonnet-4-6", noTemperature: true, maxTokens: 8192),
        AiService(name: "Google Gemini", url: "https://generativelanguage.googleapis.com/v1beta/openai", model: "gemini-3.8-flash"),
        AiService(name: "DeepSeek", url: "https://api.deepseek.com/v1", model: "deepseek-flash"),
        AiService(name: "OpenAI", url: "https://api.openai.com/v1"),
        AiService(name: "LM Studio (Mac)", url: "http://IP-DEL-MAC:1234/v1", timeoutS: 300),
        AiService(name: "Ollama (Mac)", url: "http://IP-DEL-MAC:11434/v1", timeoutS: 300),
    ]
}

/// Le chiavi nel Portachiavi di iOS: solo su questo dispositivo, non nei backup.
enum Keychain {
    private static let service = "org.openslate.ai"

    static func set(_ key: String, for id: String) {
        delete(id)
        guard !key.isEmpty else { return }
        let q: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: id,
            kSecValueData as String: Data(key.utf8),
            kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly,
        ]
        SecItemAdd(q as CFDictionary, nil)
    }

    static func get(_ id: String) -> String? {
        let q: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: id,
            kSecReturnData as String: true,
        ]
        var out: AnyObject?
        guard SecItemCopyMatching(q as CFDictionary, &out) == errSecSuccess, let d = out as? Data else { return nil }
        return String(data: d, encoding: .utf8)
    }

    static func delete(_ id: String) {
        let q: [String: Any] = [kSecClass as String: kSecClassGenericPassword, kSecAttrService as String: service, kSecAttrAccount as String: id]
        SecItemDelete(q as CFDictionary)
    }
}

/// L'elenco dei servizi (l'ordine è la catena di riserva, come sulla desktop).
final class AiStore: ObservableObject {
    @Published private(set) var services: [AiService] = []

    init() {
        if let d = UserDefaults.standard.data(forKey: "ai_services"), let l = try? JSONDecoder().decode([AiService].self, from: d) { services = l }
    }

    func save(_ l: [AiService]) {
        for s in services where !l.contains(where: { $0.id == s.id }) { Keychain.delete(s.id) }
        services = l
        UserDefaults.standard.set(try? JSONEncoder().encode(l), forKey: "ai_services")
        touch()
    }

    func hasKey(_ id: String) -> Bool { Keychain.get(id) != nil }
    func setKey(_ key: String, for id: String) {
        Keychain.set(key.trimmingCharacters(in: .whitespacesAndNewlines), for: id)
        touch()
    }

    /// Quando sono cambiate l'ultima volta (secondi Unix; 0 = mai): serve alla sincronizzazione.
    var mtime: Int64 { Int64(UserDefaults.standard.double(forKey: "ai_mtime")) }
    private func touch() { UserDefaults.standard.set(Date().timeIntervalSince1970.rounded(), forKey: "ai_mtime") }

    /// I servizi arrivati dal computer (sincronizzazione): prendono il posto di
    /// quelli di qui, nello stesso ordine; le chiavi nel Portachiavi.
    func replaceFromSync(_ list: [AiProvider]) {
        let out = list.map { p in
            AiService(id: services.first { $0.name == p.name }?.id ?? UUID().uuidString, name: p.name, url: p.url, model: p.model, vision: p.vision,
                      enabled: p.enabled, noTemperature: p.noTemperature, maxTokens: p.maxTokens, timeoutS: p.timeoutS)
        }
        save(out)
        for (p, s) in zip(list, out) { if p.key.isEmpty { Keychain.delete(s.id) } else { setKey(p.key, for: s.id) } }
    }

    func provider(_ s: AiService, key: String? = nil) -> AiProvider {
        AiProvider(name: s.name, url: s.url, model: s.model, key: key ?? Keychain.get(s.id) ?? "", vision: s.vision, enabled: s.enabled,
                   noTemperature: s.noTemperature, maxTokens: s.maxTokens, timeoutS: s.timeoutS)
    }

    func providers() -> [AiProvider] { services.map { provider($0) } }

    /// I servizi dal QR della desktop: quelli con lo stesso nome si aggiornano.
    func importFromQr(_ list: [AiProvider]) {
        var now = services
        for p in list {
            let i = now.firstIndex { $0.name == p.name }
            let s = AiService(id: i.map { now[$0].id } ?? UUID().uuidString, name: p.name, url: p.url, model: p.model, vision: p.vision,
                              enabled: p.enabled, noTemperature: p.noTemperature, maxTokens: p.maxTokens, timeoutS: p.timeoutS)
            if let i { now[i] = s } else { now.append(s) }
            if !p.key.isEmpty { setKey(p.key, for: s.id) }
        }
        save(now)
    }
}

// MARK: chat

final class UiMsg: ObservableObject, Identifiable {
    let id = UUID()
    let role: String
    let withPage: Bool
    @Published var text: String
    @Published var thinking = ""
    @Published var by = ""
    @Published var error = false
    @Published var steps: [String] = []
    @Published var boardChanged = false
    init(_ role: String, _ text: String, withPage: Bool = false) { self.role = role; self.text = text; self.withPage = withPage }
}

/// La conversazione; le richieste girano fuori dal thread principale.
final class ChatState: ObservableObject {
    @Published var msgs: [UiMsg] = []
    @Published var running = false
    @Published var status = ""
    private var cancel = false

    func stop() { cancel = true }
    func clear() { if !running { msgs.removeAll() } }

    final class Listener: AiListener {
        weak var chat: ChatState?
        let answer: UiMsg
        let onBoard: () -> Void
        init(_ chat: ChatState, _ answer: UiMsg, _ onBoard: @escaping () -> Void) { self.chat = chat; self.answer = answer; self.onBoard = onBoard }
        func onEvent(event: AiEvent) -> Bool {
            DispatchQueue.main.async { [self] in
                switch event {
                case .trying(let p, let m): chat?.status = Lf("Chiedo a %@ (%@)…", "Asking %@ (%@)…", p, m)
                case .skipped(let p, let w): chat?.status = Lf("%@ saltato: %@", "%@ skipped: %@", p, w)
                case .delta(let t): answer.text += t
                case .thinking(let t): answer.thinking += t
                case .tool(let label): chat?.status = label + "…"
                case .toolDone(let label, let result): answer.steps.append(label + ": " + result)
                case .boardChanged:
                    answer.boardChanged = true
                    onBoard()
                }
            }
            return !(chat?.cancel ?? true)
        }
    }

    /// Manda `text` (col foglio, se c'è): il tutor può leggere e annotare la lavagna.
    func send(_ text: String, page: Data?, store: AiStore, board: BoardModel, view: BoardView, onBoard: @escaping () -> Void) {
        let t = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !running, !t.isEmpty else { return }
        let history = msgs.filter { !$0.error }.map { ChatMsg(role: $0.role, text: $0.text, image: nil) }
        msgs.append(UiMsg("user", t, withPage: page != nil))
        let answer = UiMsg("assistant", "")
        msgs.append(answer)
        running = true
        cancel = false
        status = ""
        let providers = store.providers()
        let all = history + [ChatMsg(role: "user", text: t, image: page)]
        let listener = Listener(self, answer, onBoard)
        DispatchQueue.global(qos: .userInitiated).async {
            let r = Result { try aiChatBoard(providers: providers, msgs: all, board: board, view: view, listener: listener) }
            DispatchQueue.main.async { [self] in
                switch r {
                case .success(let reply):
                    answer.text = reply.text
                    answer.by = reply.provider + " · " + reply.model
                case .failure(let e):
                    if case AiError.Cancelled = e {
                        if answer.text.isEmpty { msgs.removeAll { $0 === answer } }
                    } else {
                        answer.text = aiMessage(e)
                        answer.error = true
                    }
                }
                status = ""
                running = false
            }
        }
    }
}

struct ChatScreen: View {
    @EnvironmentObject var app: AppModel
    @ObservedObject var chat: ChatState
    @State private var input = ""
    @State private var attach = true
    @StateObject private var dictation = Dictation()
    /// quello che c'era prima di dettare: il parlato si aggiunge in fondo
    @State private var base = ""

    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                if app.ai.services.allSatisfy({ !$0.enabled }) {
                    Button(L("Nessun servizio AI: tocca qui per aggiungerne uno (Claude, Gemini… o LM Studio sul Mac).",
                             "No AI service yet: tap here to add one (Claude, Gemini… or LM Studio on your Mac).")) { app.tab = .settings }
                        .padding()
                }
                ScrollViewReader { proxy in
                    ScrollView {
                        LazyVStack(alignment: .leading, spacing: 10) {
                            if chat.msgs.isEmpty {
                                Text(L("Chiedi quello che vuoi. Con «Allega il foglio» l'AI vede anche la lavagna e può scriverci: correggere un esercizio, sottolineare, riassumere…",
                                       "Ask anything. With “Attach the page” the AI also sees the board and can write on it: check an exercise, underline, summarise…"))
                                    .foregroundStyle(.secondary).padding(.vertical)
                            }
                            ForEach(chat.msgs) { m in Bubble(msg: m).id(m.id) }
                        }
                        .padding(.horizontal)
                    }
                    .onChange(of: chat.msgs.last?.text) { _ in if let last = chat.msgs.last { proxy.scrollTo(last.id, anchor: .bottom) } }
                }
                if !chat.status.isEmpty {
                    Text(chat.status).font(.caption).foregroundStyle(.secondary).frame(maxWidth: .infinity, alignment: .leading).padding(.horizontal)
                }
                inputBar
            }
            .navigationTitle(L("Chat", "Chat"))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                Button { chat.clear() } label: { Image(systemName: "square.and.pencil") }
                    .accessibilityLabel(L("Nuova chat", "New chat"))
                    .disabled(chat.running || chat.msgs.isEmpty)
            }
        }
        .onDisappear { dictation.stop() }
    }

    /// La casella del messaggio: si allarga col testo, il foglio da allegare,
    /// il microfono per dettare e il tasto per mandare (o fermare l'AI).
    private var inputBar: some View {
        let canAttach = !app.board.isEmpty
        let empty = input.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
        return VStack(alignment: .leading, spacing: 6) {
            Button { attach.toggle() } label: {
                Label(L("Allega il foglio", "Attach the page"), systemImage: attach && canAttach ? "checkmark" : "paperclip")
                    .font(.subheadline)
                    .padding(.horizontal, 12).padding(.vertical, 6)
                    .background(attach && canAttach ? Color.accentColor.opacity(0.15) : Color.clear, in: Capsule())
                    .overlay(Capsule().stroke(Color.secondary.opacity(0.4)))
            }
            .buttonStyle(.plain)
            .disabled(!canAttach)
            .opacity(canAttach ? 1 : 0.5)
            if let e = dictation.error {
                Text(e).font(.caption).foregroundStyle(.red)
            }
            HStack(alignment: .bottom, spacing: 8) {
                HStack(alignment: .bottom, spacing: 4) {
                    TextField(dictation.listening ? L("Ti ascolto… parla pure", "Listening… speak now") : L("Scrivi un messaggio", "Write a message"),
                              text: $input, axis: .vertical)
                        .lineLimit(1...6)
                        .textInputAutocapitalization(.sentences)
                        .padding(.vertical, 12)
                        .padding(.leading, 16)
                    Button {
                        base = input
                        dictation.toggle { said in
                            input = base.trimmingCharacters(in: .whitespaces).isEmpty ? said : base.trimmingCharacters(in: .whitespaces) + " " + said
                        }
                    } label: {
                        Image(systemName: dictation.listening ? "mic.fill" : "mic")
                            .font(.title3)
                            .foregroundStyle(dictation.listening ? Color.red : Color.secondary)
                            .scaleEffect(1 + 0.25 * dictation.level)
                            .frame(width: 44, height: 44)
                    }
                    .accessibilityLabel(L("Detta", "Dictate"))
                }
                .background(Color(.secondarySystemBackground), in: RoundedRectangle(cornerRadius: 24))
                if chat.running {
                    Button { chat.stop() } label: {
                        Image(systemName: "stop.fill").font(.title3).frame(width: 48, height: 48)
                            .background(Color.accentColor, in: Circle()).foregroundStyle(.white)
                    }
                    .accessibilityLabel(L("Ferma", "Stop"))
                } else {
                    Button {
                        dictation.stop()
                        let png = attach && canAttach ? Export.pngData(app.board.toPage(Float(app.padSize.width), Float(app.padSize.height)), quarter: app.quarter) : nil
                        chat.send(input.trimmingCharacters(in: .whitespacesAndNewlines), page: png, store: app.ai, board: app.board.model, view: app.view) { app.board.reload() }
                        input = ""
                    } label: {
                        Image(systemName: "arrow.up").font(.title3.bold()).frame(width: 48, height: 48)
                            .background(empty ? Color.secondary.opacity(0.3) : Color.accentColor, in: Circle()).foregroundStyle(.white)
                    }
                    .disabled(empty)
                    .accessibilityLabel(L("Invia", "Send"))
                }
            }
        }
        .padding(.horizontal, 12).padding(.vertical, 8)
        .background(.bar)
    }
}

struct Bubble: View {
    @EnvironmentObject var app: AppModel
    @ObservedObject var msg: UiMsg
    @State private var showThinking = false

    var body: some View {
        let mine = msg.role == "user"
        VStack(alignment: mine ? .trailing : .leading, spacing: 2) {
            VStack(alignment: .leading, spacing: 4) {
                if !msg.thinking.isEmpty {
                    Button((showThinking ? "▾ " : "▸ ") + L("Ragionamento", "Reasoning")) { showThinking.toggle() }.font(.caption)
                    if showThinking { Text(msg.thinking).font(.caption).foregroundStyle(.secondary) }
                }
                ForEach(msg.steps, id: \.self) { s in Text("✓ " + s).font(.caption).foregroundStyle(.secondary) }
                Text(msg.text.isEmpty ? "…" : msg.text).textSelection(.enabled)
                if msg.withPage { Text("📄 " + L("foglio allegato", "page attached")).font(.caption2) }
                if msg.boardChanged { Button(L("Vedi il foglio", "See the page")) { app.tab = .board }.font(.callout) }
            }
            .padding(10)
            .background(msg.error ? Color.red.opacity(0.15) : mine ? Color.accentColor.opacity(0.15) : Color.secondary.opacity(0.12))
            .clipShape(RoundedRectangle(cornerRadius: 14))
            if !msg.by.isEmpty { Text(msg.by).font(.caption2).foregroundStyle(.secondary) }
        }
        .frame(maxWidth: .infinity, alignment: mine ? .trailing : .leading)
    }
}

// MARK: impostazioni AI

struct AiSettingsScreen: View {
    @ObservedObject var store: AiStore
    @State private var editing: AiService?
    @State private var adding = false
    @State private var scanning = false
    @State private var scanned: [AiProvider]?
    @State private var result = ""

    var body: some View {
        List {
            Section {
                Text(L("Si usa il primo servizio che risponde; se non va si prova il successivo. Le chiavi restano su questo dispositivo, nel Portachiavi.",
                       "The first service that answers is used; if it fails the next one is tried. Keys stay on this device, in the Keychain."))
                    .font(.footnote).foregroundStyle(.secondary)
                Button { scanning = true } label: { Label(L("Importa dal computer (QR)", "Import from the computer (QR)"), systemImage: "qrcode.viewfinder") }
                Button { adding = true } label: { Label(L("Aggiungi un servizio", "Add a service"), systemImage: "plus") }
                if !result.isEmpty { Text(result).font(.footnote).foregroundStyle(.tint) }
            }
            Section(L("Servizi", "Services")) {
                ForEach(store.services) { s in
                    Button { editing = s } label: {
                        HStack {
                            VStack(alignment: .leading) {
                                Text(s.name).foregroundStyle(.primary)
                                let key = s.url.hasPrefix("https://") ? (store.hasKey(s.id) ? L(" · chiave salvata", " · key saved") : L(" · manca la chiave", " · key missing")) : ""
                                Text((s.model.isEmpty ? L("modello automatico", "automatic model") : s.model) + key).font(.caption).foregroundStyle(.secondary)
                            }
                            Spacer()
                            Image(systemName: s.enabled ? "checkmark.circle.fill" : "circle").foregroundStyle(s.enabled ? Color.accentColor : .secondary)
                        }
                    }
                }
                .onMove { from, to in var l = store.services; l.move(fromOffsets: from, toOffset: to); store.save(l) }
                .onDelete { idx in var l = store.services; l.remove(atOffsets: idx); store.save(l) }
            }
        }
        .navigationTitle(L("Impostazioni AI", "AI settings"))
        .toolbar { EditButton() }
        .sheet(item: $editing) { s in ServiceEditor(store: store, service: s, isNew: !store.services.contains { $0.id == s.id }) }
        .confirmationDialog(L("Aggiungi un servizio", "Add a service"), isPresented: $adding) {
            ForEach(AiPresets.all, id: \.name) { p in Button(p.name) { var n = p; n.id = UUID().uuidString; editing = n } }
        }
        .sheet(isPresented: $scanning) {
            QrScanner { text in
                scanning = false
                do { scanned = try aiImportQr(text: text) } catch { result = aiMessage(error) }
            }
        }
        .alert(L("Importa dal computer", "Import from the computer"), isPresented: Binding(get: { scanned != nil }, set: { if !$0 { scanned = nil } })) {
            Button(L("Importa", "Import")) {
                if let l = scanned { store.importFromQr(l); result = Lf("Importati: %d servizi.", "Imported: %d services.", l.count) }
                scanned = nil
            }
            Button(L("Annulla", "Cancel"), role: .cancel) { scanned = nil }
        } message: {
            Text(L("Questi servizi (con le loro chiavi) verranno salvati su questo dispositivo:\n", "These services (with their keys) will be saved on this device:\n")
                 + (scanned ?? []).map { "• " + $0.name }.joined(separator: "\n"))
        }
    }
}

struct ServiceEditor: View {
    @ObservedObject var store: AiStore
    @State var service: AiService
    let isNew: Bool
    @State private var key = ""
    @State private var test = ""
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            Form {
                TextField(L("Nome", "Name"), text: $service.name)
                TextField(L("Indirizzo (…/v1)", "Address (…/v1)"), text: $service.url).keyboardType(.URL).textInputAutocapitalization(.never).autocorrectionDisabled()
                TextField(L("Modello (vuoto = automatico)", "Model (empty = automatic)"), text: $service.model).textInputAutocapitalization(.never).autocorrectionDisabled()
                SecureField(store.hasKey(service.id) ? L("Nuova chiave (vuota = tieni quella di adesso)", "New key (empty = keep the current one)") : L("Chiave API", "API key"), text: $key)
                Toggle(L("Legge le immagini", "Reads images"), isOn: $service.vision)
                Toggle(L("Acceso", "Enabled"), isOn: $service.enabled)
                Section {
                    Button(L("Prova", "Test")) {
                        test = Lf("Provo %@…", "Testing %@…", service.name)
                        let p = store.provider(service, key: key.isEmpty ? nil : key)
                        DispatchQueue.global().async {
                            let r = Result { try aiListModels(provider: p) }
                            DispatchQueue.main.async {
                                switch r {
                                case .success(let m): test = Lf("%@ risponde: %d modelli.", "%@ answers: %d models.", service.name, m.count)
                                case .failure(let e): test = service.name + ": " + aiMessage(e)
                                }
                            }
                        }
                    }
                    if !test.isEmpty { Text(test).font(.footnote) }
                }
            }
            .navigationTitle(service.name)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) { Button(L("Annulla", "Cancel")) { dismiss() } }
                ToolbarItem(placement: .confirmationAction) {
                    Button("OK") {
                        if !key.isEmpty { store.setKey(key, for: service.id) }
                        var l = store.services
                        if let i = l.firstIndex(where: { $0.id == service.id }) { l[i] = service } else { l.append(service) }
                        store.save(l)
                        dismiss()
                    }.disabled(service.name.isEmpty || !(service.url.hasPrefix("http://") || service.url.hasPrefix("https://")))
                }
            }
        }
    }
}

/// Lo scanner del QR (VisionKit). Sul simulatore non c'è la fotocamera: si incolla il testo.
struct QrScanner: View {
    var hint = L("Sul computer: OpenSlate → Impostazioni AI → Collega il telefono. Inquadra il QR.",
                 "On the computer: OpenSlate → AI settings → Connect the phone. Frame the QR code.")
    var placeholder = "OPENSLATE-AI1:…"
    var title = L("Importa dal computer", "Import from the computer")
    let found: (String) -> Void
    @State private var pasted = ""

    var body: some View {
        NavigationStack {
            VStack {
                if DataScannerViewController.isSupported && DataScannerViewController.isAvailable {
                    ScannerView(found: found)
                    Text(hint).font(.footnote).padding()
                } else {
                    Form {
                        Text(L("La fotocamera non è disponibile: incolla qui il testo del QR.", "The camera is not available: paste the QR text here."))
                        TextField(placeholder, text: $pasted, axis: .vertical)
                        Button(L("Importa", "Import")) { found(pasted) }.disabled(pasted.isEmpty)
                    }
                }
            }
            .navigationTitle(title)
            .navigationBarTitleDisplayMode(.inline)
        }
    }
}

struct ScannerView: UIViewControllerRepresentable {
    let found: (String) -> Void

    func makeUIViewController(context: Context) -> DataScannerViewController {
        let vc = DataScannerViewController(recognizedDataTypes: [.barcode(symbologies: [.qr])], isHighlightingEnabled: true)
        vc.delegate = context.coordinator
        try? vc.startScanning()
        return vc
    }

    func updateUIViewController(_ vc: DataScannerViewController, context: Context) {}

    func makeCoordinator() -> Coordinator { Coordinator(found) }

    final class Coordinator: NSObject, DataScannerViewControllerDelegate {
        let found: (String) -> Void
        var done = false
        init(_ f: @escaping (String) -> Void) { found = f }
        func dataScanner(_ s: DataScannerViewController, didAdd items: [RecognizedItem], allItems: [RecognizedItem]) {
            for case .barcode(let b) in items {
                guard !done, let text = b.payloadStringValue, text.hasPrefix("OPENSLATE-AI1:") else { continue }
                done = true
                s.stopScanning()
                found(text)
            }
        }
    }
}
