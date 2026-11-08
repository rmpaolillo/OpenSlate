import CoreBluetooth
import Foundation

/// Cosa mostrare all'utente sul pad.
enum LinkStatus: Equatable {
    case idle, bluetoothOff, searching, notFound, notRegistered
    case connecting(String)
    case pressButton, registeredRetry
    case live(PadInfo)
    case syncing(Int, Int)
    case synced(Int, Int)
    case failed(String)
    case needRegister, disconnected

    var isOn: Bool {
        switch self {
        case .live, .syncing, .synced: return true
        default: return false
        }
    }

    var isBusy: Bool {
        switch self {
        case .searching, .connecting, .pressButton: return true
        default: return false
        }
    }

    var text: String {
        switch self {
        case .idle: return L("Non collegato. Premi brevemente il pulsante del pad, poi Collega.", "Not connected. Briefly press the pad's button, then Connect.")
        case .bluetoothOff: return L("Accendi il Bluetooth, poi Collega.", "Turn Bluetooth on, then Connect.")
        case .searching: return L("Cerco il pad…", "Looking for the pad…")
        case .notFound: return L("Non trovo il pad. Premi brevemente il suo pulsante e subito dopo Collega. Per registrarlo: tieni premuto ~6 s finché il LED lampeggia blu.", "Can't find the pad. Briefly press its button and right after Connect. To register it: hold it ~6 s until the LED blinks blue.")
        case .notRegistered: return L("Questa app non è ancora registrata sul pad. Tieni premuto il pulsante ~6 s finché il LED lampeggia blu, poi Collega.", "This app is not registered on the pad yet. Hold the button ~6 s until the LED blinks blue, then Connect.")
        case .connecting(let n): return Lf("Collegamento a «%@»…", "Connecting to «%@»…", n)
        case .pressButton: return L(">>> Registrazione: PREMI ORA una volta il pulsante del pad <<<", ">>> Registration: press the pad's button once NOW <<<")
        case .registeredRetry: return L("Registrato ✓  Ora premi brevemente il pulsante del pad e poi Collega.", "Registered ✓  Now briefly press the pad's button and then Connect.")
        case .live: return L("Connesso. Live attivo — scrivi sul pad.", "Connected. Live — write on the pad.")
        case .syncing(let i, let n): return n == 0 ? L("Scarico le pagine dal pad…", "Downloading the pages from the pad…") : Lf("Scarico la pagina %d di %d…", "Downloading page %d of %d…", i, n)
        case .synced(let files, let deleted):
            if files == 0 { return L("Nessuna pagina nella memoria del pad. Live attivo.", "No pages in the pad's memory. Live.") }
            if deleted < files { return Lf("Pagine scaricate: %d di %d. Una non si è potuta verificare e resta sul pad.", "Pages downloaded: %d of %d. One could not be verified and stays on the pad.", deleted, files) }
            return Lf("Pagine scaricate: %d (sono nella Libreria). Live attivo.", "Pages downloaded: %d (they are in the Library). Live.", deleted)
        case .failed(let m): return L("Collegamento non riuscito: ", "Connection failed: ") + m
        case .needRegister: return L("Il pad non riconosce questa app. Tieni premuto il pulsante ~6 s finché il LED lampeggia blu, poi Collega.", "The pad doesn't recognise this app. Hold the button ~6 s until the LED blinks blue, then Connect.")
        case .disconnected: return L("Pad scollegato.", "Pad disconnected.")
        }
    }
}

protocol LinkListener: AnyObject {
    func onStatus(_ s: LinkStatus)
    func onPen(_ points: [PenPoint])
    /// Una pagina dal pad: salvarla e dire se è al sicuro (solo allora si toglie dal pad).
    func onPageDownloaded(raw: Data, timeBcd: Data, crcOk: Bool, info: PadInfo) -> Bool
}

/**
 Il Bluetooth col Bamboo Slate (CoreBluetooth), come `SlateLink.kt` dell'app
 Android: ricerca, connessione e GATT. Il protocollo lo fa il cuore Rust
 (`SlateSession`): qui si passano i byte avanti e indietro. Tutto sul thread
 principale.
 */
final class SlateLink: NSObject, CBCentralManagerDelegate, CBPeripheralDelegate {
    weak var listener: LinkListener?
    private var central: CBCentralManager!
    private let uuids = gattUuids()
    private lazy var uWrite = CBUUID(string: uuids.nordicWrite)
    private lazy var uNotify = CBUUID(string: uuids.nordicNotify)
    private lazy var uLive = CBUUID(string: uuids.livePenData)
    private lazy var uOffline = CBUUID(string: uuids.offlinePenData)
    private lazy var uSys = CBUUID(string: uuids.sysevent)

    private var peripheral: CBPeripheral?
    private var session: SlateSession?
    private var writeChar: CBCharacteristic?
    private var writes: [Data] = []
    private var toSubscribe: [CBCharacteristic] = []
    private var pendingServices = 0
    private var registering = false
    private var registeredNow = false
    private var quietClose = false
    private var timerGen = 0
    private var scanning = false
    private var scanStart = Date()
    private var best: (CBPeripheral, AdvMode, Bool)?
    private var wantScan = false
    private(set) var info: PadInfo?

    private let defaults = UserDefaults.standard

    override init() {
        super.init()
        central = CBCentralManager(delegate: self, queue: .main)
    }

    var bluetoothOn: Bool { central.state == .poweredOn }

    /// UUID con cui l'app è registrata sul pad.
    private var savedUuid: Data? {
        get { defaults.data(forKey: "pad_uuid").flatMap { $0.count == 6 ? $0 : nil } }
        set { defaults.set(newValue, forKey: "pad_uuid") }
    }

    private var savedId: String? {
        get { defaults.string(forKey: "pad_id") }
        set { defaults.set(newValue, forKey: "pad_id") }
    }

    var live: Bool { session.map { $0.isLive() && !$0.isSyncing() } ?? false }

    // MARK: ricerca

    func connect() {
        guard peripheral == nil, !scanning else { return }
        guard central.state == .poweredOn else {
            // il primo uso chiede il permesso: la ricerca parte quando il Bluetooth è pronto
            wantScan = true
            if central.state == .poweredOff { listener?.onStatus(.bluetoothOff) }
            return
        }
        startScan()
    }

    private func startScan() {
        listener?.onStatus(.searching)
        scanning = true
        scanStart = Date()
        best = nil
        central.scanForPeripherals(withServices: nil, options: [CBCentralManagerScanOptionAllowDuplicatesKey: true])
        DispatchQueue.main.asyncAfter(deadline: .now() + 10) { [weak self] in
            guard let self, self.scanning else { return }
            self.finishScan()
        }
    }

    private func finishScan() {
        central.stopScan()
        scanning = false
        if let (p, mode, _) = best { open(p, mode) } else { listener?.onStatus(.notFound) }
    }

    func centralManagerDidUpdateState(_ c: CBCentralManager) {
        if c.state == .poweredOn, wantScan {
            wantScan = false
            startScan()
        } else if c.state == .poweredOff {
            listener?.onStatus(.bluetoothOff)
        }
    }

    func centralManager(_ c: CBCentralManager, didDiscover p: CBPeripheral, advertisementData ad: [String: Any], rssi: NSNumber) {
        guard scanning else { return }
        var mfr: [MfrEntry] = []
        // su iOS i dati produttore cominciano col company ID (2 byte, little endian)
        if let d = ad[CBAdvertisementDataManufacturerDataKey] as? Data, d.count >= 2 {
            let company = UInt16(d[d.startIndex]) | UInt16(d[d.startIndex + 1]) << 8
            mfr.append(MfrEntry(company: company, len: UInt32(d.count - 2)))
        }
        let name = (ad[CBAdvertisementDataLocalNameKey] as? String) ?? p.name ?? ""
        let services = ((ad[CBAdvertisementDataServiceUUIDsKey] as? [CBUUID]) ?? []).map { $0.uuidString }
        let adv = classifyAdvert(name: name, mfr: mfr, services: services, isSaved: p.identifier.uuidString == savedId)
        if adv.candidate && (best == nil || !mfr.isEmpty) {
            best = (p, adv.mode, !mfr.isEmpty)
        }
        if best?.2 == true && Date().timeIntervalSince(scanStart) >= 1.5 { finishScan() }
    }

    // MARK: connessione

    private func open(_ p: CBPeripheral, _ mode: AdvMode) {
        let uuid = savedUuid
        if mode == .normal && uuid == nil {
            listener?.onStatus(.notRegistered)
            return
        }
        registering = mode == .register || uuid == nil
        savedId = p.identifier.uuidString
        peripheral = p
        p.delegate = self
        listener?.onStatus(.connecting(p.name ?? L("pad", "pad")))
        central.connect(p)
    }

    /// Ferma il live e chiude il collegamento; `report = false` se il motivo è già stato mostrato.
    func disconnect(report: Bool = true) {
        if scanning {
            central.stopScan()
            scanning = false
        }
        wantScan = false
        if report { listener?.onStatus(.idle) }
        guard let p = peripheral else { return }
        quietClose = true
        if let s = session, s.isLive() {
            enqueue([s.stopFrame()])
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) { [weak self] in self?.central.cancelPeripheralConnection(p) }
        } else {
            central.cancelPeripheralConnection(p)
        }
        // se il pad è già spento la risposta può non arrivare mai
        DispatchQueue.main.asyncAfter(deadline: .now() + 3) { [weak self] in
            if self?.peripheral === p { self?.closed() }
        }
    }

    private func closed() {
        quietClose = false
        peripheral = nil
        session = nil
        writeChar = nil
        writes.removeAll()
        toSubscribe.removeAll()
        timerGen += 1
    }

    func centralManager(_ c: CBCentralManager, didConnect p: CBPeripheral) {
        guard p === peripheral else { return }
        p.discoverServices(nil)
    }

    func centralManager(_ c: CBCentralManager, didFailToConnect p: CBPeripheral, error: Error?) {
        guard p === peripheral else { return }
        closed()
        listener?.onStatus(.failed(error?.localizedDescription ?? "GATT"))
    }

    func centralManager(_ c: CBCentralManager, didDisconnectPeripheral p: CBPeripheral, error: Error?) {
        guard p === peripheral else { return }
        let wasLive = session?.isLive() == true
        let quiet = quietClose
        closed()
        if !quiet { listener?.onStatus(wasLive ? .disconnected : .failed(error?.localizedDescription ?? "GATT")) }
    }

    func peripheral(_ p: CBPeripheral, didDiscoverServices error: Error?) {
        let services = p.services ?? []
        pendingServices = services.count
        for s in services { p.discoverCharacteristics(nil, for: s) }
    }

    func peripheral(_ p: CBPeripheral, didDiscoverCharacteristicsFor service: CBService, error: Error?) {
        pendingServices -= 1
        guard pendingServices == 0 else { return }
        let chars = (p.services ?? []).flatMap { $0.characteristics ?? [] }
        writeChar = chars.first { $0.uuid == uWrite }
        guard writeChar != nil, chars.contains(where: { $0.uuid == uNotify }) else {
            listener?.onStatus(.failed("Nordic UART"))
            disconnect(report: false)
            return
        }
        toSubscribe = [uNotify, uLive, uOffline, uSys].compactMap { u in chars.first { $0.uuid == u } }
        subscribeNext()
    }

    /// Le notifiche si accendono una alla volta.
    private func subscribeNext() {
        guard let p = peripheral else { return }
        if toSubscribe.isEmpty { return startSession() }
        p.setNotifyValue(true, for: toSubscribe.removeFirst())
    }

    func peripheral(_ p: CBPeripheral, didUpdateNotificationStateFor c: CBCharacteristic, error: Error?) {
        subscribeNext()
    }

    private func startSession() {
        let s = SlateSession()
        session = s
        registeredNow = false
        if registering || savedUuid == nil {
            listener?.onStatus(.pressButton)
            apply(s.beginRegister())
        } else {
            apply(s.beginLive(uuid: savedUuid!))
        }
    }

    /// Scarica le pagine dalla memoria del pad (poi torna il live).
    func sync() {
        guard let s = session, let uuid = savedUuid, s.isLive() else { return }
        listener?.onStatus(.syncing(0, 0))
        apply(s.beginSync(uuid: uuid))
    }

    // MARK: dati

    func peripheral(_ p: CBPeripheral, didUpdateValueFor c: CBCharacteristic, error: Error?) {
        guard let s = session, let v = c.value else { return }
        switch c.uuid {
        case uNotify: apply(s.onNordic(bytes: v))
        case uLive:
            let pts = s.onLive(bytes: v)
            if !pts.isEmpty { listener?.onPen(pts) }
        case uOffline: s.onOffline(bytes: v)
        default: break
        }
    }

    private func apply(_ step: Step) {
        enqueue(step.writes)
        for e in step.events {
            switch e {
            case .registered(let uuid):
                savedUuid = uuid
                registeredNow = true
            case .live(let i):
                info = i
                listener?.onStatus(.live(i))
            case .syncProgress(let i, let n):
                listener?.onStatus(.syncing(Int(i), Int(n)))
            case .downloaded(let raw, let bcd, let crc):
                let pad = info ?? PadInfo(name: "", firmware: "", batteryPct: nil, charging: false, width: 0, height: 0)
                let saved = listener?.onPageDownloaded(raw: raw, timeBcd: bcd, crcOk: crc, info: pad) ?? false
                if let s = session { apply(s.confirmSaved(saved: saved)) }
            case .syncDone(let files, let deleted):
                listener?.onStatus(.synced(Int(files), Int(deleted)))
            case .failed(let message, let needRegister):
                listener?.onStatus(needRegister ? .needRegister : (registeredNow ? .registeredRetry : .failed(message)))
                disconnect(report: false)
            }
        }
        if step.events.contains(where: {
            if case .live = $0 { return true }
            if case .failed = $0 { return true }
            if case .syncDone = $0 { return true }
            return false
        }) { timerGen += 1 }
        if step.waitMs > 0 {
            timerGen += 1
            let gen = timerGen
            DispatchQueue.main.asyncAfter(deadline: .now() + .milliseconds(Int(step.waitMs))) { [weak self] in
                guard let self, gen == self.timerGen, let s = self.session else { return }
                self.apply(s.onTimeout())
            }
        }
    }

    private func enqueue(_ frames: [Data]) {
        writes.append(contentsOf: frames)
        writeNext()
    }

    /// Scritture senza risposta, quando il pad è pronto a riceverle.
    private func writeNext() {
        guard let p = peripheral, let c = writeChar else { return }
        while !writes.isEmpty && p.canSendWriteWithoutResponse {
            p.writeValue(writes.removeFirst(), for: c, type: .withoutResponse)
        }
    }

    func peripheralIsReady(toSendWriteWithoutResponse p: CBPeripheral) {
        writeNext()
    }
}
