import Foundation
import Network
import SwiftUI
import UIKit

/// La sincronizzazione col computer (il cuore è in Rust, `src/sync`): il
/// telefono si abbina una volta col QR della desktop, poi si sincronizza da
/// solo quando l'app torna attiva, poco dopo ogni salvataggio e ogni minuto
/// mentre è aperta. Il segreto sta nel Portachiavi.
final class PhoneSync: ObservableObject {
    @Published private(set) var deskName: String?
    @Published private(set) var lastOk: Date?
    @Published private(set) var lastError: String?
    @Published private(set) var busy = false

    private let dataDir: String
    private let libDir: String
    /// Cosa è cambiato sul telefono ("lib", "notes", "stamps").
    var onChanged: ([String]) -> Void = { _ in }
    private var timer: Timer?
    private var pending: DispatchWorkItem?
    private let defaults = UserDefaults(suiteName: "sync") ?? .standard
    private static let keychainId = "sync-secret"

    var paired: Bool { deskName != nil }

    init(dataDir: String, libDir: String) {
        self.dataDir = dataDir
        self.libDir = libDir
        deskName = defaults.string(forKey: "desk_name")
        let t = defaults.double(forKey: "last_ok")
        lastOk = t > 0 ? Date(timeIntervalSince1970: t) : nil
    }

    var lastLabel: String? {
        lastOk.map { DateFormatter.localizedString(from: $0, dateStyle: .short, timeStyle: .short) }
    }

    private func pairing() -> SyncPairing? {
        guard let d = defaults.data(forKey: "pairing"),
              let j = try? JSONSerialization.jsonObject(with: d) as? [String: Any],
              let secret = Keychain.get(Self.keychainId) else { return nil }
        return SyncPairing(deskId: j["desk_id"] as? String ?? "", deskName: j["desk_name"] as? String ?? "",
                           hosts: j["hosts"] as? [String] ?? [], port: UInt16(j["port"] as? Int ?? 0),
                           peerId: j["peer_id"] as? String ?? "", secret: secret)
    }

    /// Il QR del computer: si abbina e si sincronizza subito.
    func pair(_ qr: String) throws {
        let p = try syncParseQr(text: qr)
        let j: [String: Any] = ["desk_id": p.deskId, "desk_name": p.deskName, "hosts": p.hosts, "port": Int(p.port), "peer_id": p.peerId]
        defaults.set(try JSONSerialization.data(withJSONObject: j), forKey: "pairing")
        defaults.set(p.deskName, forKey: "desk_name")
        defaults.removeObject(forKey: "last_host")
        defaults.set(0.0, forKey: "last_ok")
        Keychain.set(p.secret, for: Self.keychainId)
        deskName = p.deskName
        lastOk = nil
        lastError = nil
        syncSoon(0)
    }

    func unpair() {
        for k in ["pairing", "desk_name", "last_host", "last_ok"] { defaults.removeObject(forKey: k) }
        Keychain.delete(Self.keychainId)
        deskName = nil
        lastOk = nil
        lastError = nil
    }

    /// L'app è attiva: sincronizza adesso e poi ogni minuto.
    func resume() {
        syncSoon(0.5)
        timer?.invalidate()
        timer = Timer.scheduledTimer(withTimeInterval: 60, repeats: true) { [weak self] _ in self?.syncSoon(0) }
    }

    func pause() {
        timer?.invalidate()
        timer = nil
    }

    /// Sincronizza fra `delay` secondi (le richieste vicine diventano una sola).
    func syncSoon(_ delay: Double) {
        guard paired else { return }
        pending?.cancel()
        let w = DispatchWorkItem { [weak self] in self?.runNow() }
        pending = w
        DispatchQueue.main.asyncAfter(deadline: .now() + delay, execute: w)
    }

    private func runNow() {
        guard !busy, let p = pairing() else { return }
        busy = true
        let lastHost = defaults.string(forKey: "last_host")
        let (data, lib, device) = (dataDir, libDir, UIDevice.current.name)
        DispatchQueue.global(qos: .utility).async {
            func run(_ found: [String]) -> Result<SyncReport, Error> {
                Result { try syncRun(pairing: p, found: found, device: device, dataDir: data, libraryDir: lib) }
            }
            var r = run(lastHost.map { [$0] } ?? [])
            // il computer ha cambiato indirizzo: lo si cerca sulla rete (Bonjour)
            if case .failure(let e) = r, !"\(e)".contains("QR") {
                let found = Discovery.find(deskId: p.deskId, timeout: 5)
                if !found.isEmpty { r = run(found) }
            }
            DispatchQueue.main.async {
                self.busy = false
                switch r {
                case .success(let rep):
                    self.lastOk = Date()
                    self.lastError = nil
                    self.defaults.set(rep.host, forKey: "last_host")
                    self.defaults.set(self.lastOk!.timeIntervalSince1970, forKey: "last_ok")
                    if !rep.changed.isEmpty { self.onChanged(rep.changed) }
                case .failure(let e):
                    if case StoreError.Io(let reason) = e { self.lastError = reason } else { self.lastError = "\(e)" }
                }
            }
        }
    }
}

/// Cerca il computer sulla rete di casa col suo nome Bonjour e ne ricava
/// l'indirizzo (aspettando al massimo `timeout` secondi).
enum Discovery {
    static func find(deskId: String, timeout: Double) -> [String] {
        let q = DispatchQueue(label: "sync-bonjour")
        let done = DispatchSemaphore(value: 0)
        var out: [String] = []
        let browser = NWBrowser(for: .bonjour(type: syncServiceType(), domain: "local."), using: .tcp)
        var conn: NWConnection?
        browser.browseResultsChangedHandler = { results, _ in
            guard conn == nil, let r = results.first(where: {
                if case .service(let name, _, _, _) = $0.endpoint { return name.contains(deskId) }
                return false
            }) else { return }
            // collegandosi al servizio si scopre l'indirizzo vero
            let c = NWConnection(to: r.endpoint, using: .tcp)
            conn = c
            c.stateUpdateHandler = { st in
                if case .ready = st, case .hostPort(let host, _) = c.currentPath?.remoteEndpoint {
                    var s = "\(host)"
                    if let i = s.firstIndex(of: "%") { s = String(s[..<i]) }
                    out.append(s)
                    done.signal()
                } else if case .failed = st {
                    done.signal()
                }
            }
            c.start(queue: q)
        }
        browser.start(queue: q)
        _ = done.wait(timeout: .now() + timeout)
        conn?.cancel()
        browser.cancel()
        return q.sync { out }
    }
}

/// Impostazioni › Sincronizza col computer.
struct SyncScreen: View {
    @EnvironmentObject var app: AppModel
    @ObservedObject var sync: PhoneSync
    @State private var scanning = false
    @State private var msg = ""

    var body: some View {
        List {
            Section {
                Text(L("I documenti da firmare, le pagine dell'archivio, gli appunti e i timbri restano uguali sul telefono e sul computer, attraverso la rete Wi‑Fi di casa: niente cloud, tutto cifrato. Il telefono si sincronizza da solo quando è sulla stessa rete e OpenSlate è aperto sul computer. Quello che si toglie o si sovrascrive va nel cestino, non si perde.",
                       "Documents to sign, library pages, notes and stamps stay the same on the phone and on the computer, over your home Wi‑Fi: no cloud, everything encrypted. The phone syncs by itself when it is on the same network and OpenSlate is open on the computer. Anything removed or overwritten goes to the trash, nothing is lost."))
                    .font(.footnote).foregroundStyle(.secondary)
            }
            if let desk = sync.deskName {
                Section {
                    LabeledContent(L("Collegato a", "Connected to"), value: desk)
                    Text(sync.busy ? L("Sincronizzo…", "Syncing…")
                         : sync.lastLabel.map { L("Ultima sincronizzazione: ", "Last sync: ") + $0 } ?? L("Non ancora sincronizzato", "Not synced yet"))
                    if let e = sync.lastError {
                        Text(L("Ultimo tentativo: ", "Last attempt: ") + e).font(.footnote).foregroundStyle(.red)
                    }
                    Button { sync.syncSoon(0) } label: { Label(L("Sincronizza adesso", "Sync now"), systemImage: "arrow.triangle.2.circlepath") }
                        .disabled(sync.busy)
                    Button(role: .destructive) { sync.unpair() } label: { Label(L("Scollega", "Disconnect"), systemImage: "xmark.circle") }
                }
            } else {
                Section {
                    Button { scanning = true } label: { Label(L("Inquadra il QR del computer", "Scan the computer's QR code"), systemImage: "qrcode.viewfinder") }
                } footer: {
                    Text(L("Sul computer: OpenSlate › Opzioni › Telefono › Collega un telefono…", "On the computer: OpenSlate › Options › Phone › Connect a phone…"))
                }
            }
            if !msg.isEmpty { Text(msg).foregroundStyle(.red) }
        }
        .navigationTitle(L("Sincronizza col computer", "Sync with computer"))
        .sheet(isPresented: $scanning) {
            QrScanner(hint: L("Sul computer: OpenSlate › Opzioni › Telefono › Collega un telefono…, poi inquadra il QR.",
                              "On the computer: OpenSlate › Options › Phone › Connect a phone…, then frame the QR code."),
                      placeholder: "OPENSLATE-SYNC1:…",
                      title: L("Collega al computer", "Connect to the computer")) { text in
                scanning = false
                do { try sync.pair(text); msg = "" } catch {
                    if case StoreError.Io(let reason) = error { msg = reason } else { msg = "\(error)" }
                }
            }
        }
    }
}
