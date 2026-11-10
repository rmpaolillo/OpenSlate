import SwiftUI

/// La bella copia e «Disponi sulle righe» sul telefono (come la desktop e
/// l'app Android): si sceglie una zona col dito (o tutta la pagina), si guarda
/// l'anteprima, si accetta o si rifiuta. Il lavoro vero lo fa il cuore Rust
/// (`neat.rs`).
enum NeatKind { case fair, rules }

enum NeatPhase { case select, working, preview }

struct ZoneJob {
    var kind: NeatKind
    var phase: NeatPhase = .select
    /// Il riquadro mentre lo si trascina (frazioni della pagina).
    var drag: [Float]?
    /// La zona scelta (nil = tutta la pagina).
    var zone: Zone?
    var fair: FairJob?
    var preview: BoardPreview?
    /// Sui quadretti: una riga ogni quanti quadretti.
    var every: UInt32?
    /// Cosa dire sotto il titolo.
    var note = ""
}

extension Zone {
    var floats: [Float] { [u0, v0, u1, v1] }
}

func neatIssueText(_ i: NeatIssue) -> String {
    switch i {
    case .nothing: return L("Qui non c'è scrittura da mettere sulle righe: trascina un altro riquadro.", "There is no writing to put on the lines here: drag another box.")
    case .unsupported: return L("Scegli prima un foglio a righe o a quadretti (Foglio).", "First choose a lined or squared page (Paper).")
    case .noRoom(let fit, let total):
        return Lf("Non c'è posto per tutte le righe (%d su %d): scegli una zona più grande o meno quadretti per riga.",
                  "Not enough room for all the lines (%d of %d): choose a bigger area or fewer squares per line.", Int(fit), Int(total))
    }
}

/// Il perché di un errore (nil = annullato da te).
func neatErrorText(_ e: Error) -> String? {
    guard let n = e as? NeatError else { return "AI: " + aiMessage(e) }
    switch n {
    case .Rules(let issue): return neatIssueText(issue)
    case .NothingToDo: return L("Qui non c'è niente da mettere in bella: trascina un altro riquadro.", "Nothing to put in fair copy here: drag another box.")
    case .Ai(let reason): return "AI: " + reason
    case .Cancelled: return nil
    }
}

func neatDoneText(_ kind: NeatKind, _ p: BoardPreview) -> String {
    var parts = [kind == .rules
        ? Lf("%d righe sulle righe del foglio.", "%d lines on the page lines.", Int(p.lines))
        : Lf("%d righe in bella, %d forme rifatte.", "%d lines in fair copy, %d shapes redone.", Int(p.lines), Int(p.shapes))]
    if p.shrunk > 0 { parts.append(Lf("%d rimpicciolite perché più alte della riga.", "%d made smaller because taller than the line.", Int(p.shrunk))) }
    if !p.by.isEmpty { parts.append(Lf("Ha letto %@.", "Read by %@.", p.by)) }
    if let i = p.rulesIssue { parts.append(L("Sulle righe no: ", "Not on the lines: ") + neatIssueText(i)) }
    return parts.joined(separator: " ")
}

/// Ascolta l'AI mentre legge: chi prova, chi salta; `false` = annulla.
final class NeatListener: AiListener {
    let alive: () -> Bool
    let onText: (String) -> Void
    init(alive: @escaping () -> Bool, onText: @escaping (String) -> Void) { self.alive = alive; self.onText = onText }
    func onEvent(event: AiEvent) -> Bool {
        switch event {
        case .trying(let p, let m): DispatchQueue.main.async { self.onText(Lf("Chiedo a %@ (%@)…", "Asking %@ (%@)…", p, m)) }
        case .skipped(let p, let w): DispatchQueue.main.async { self.onText(Lf("%@ saltato: %@", "%@ skipped: %@", p, w)) }
        default: break
        }
        return alive()
    }
}

extension AppModel {
    func sheetSpec(_ every: UInt32?) -> SheetSpec { SheetSpec(paper: paperCode, margin: paperMargin, every: every) }

    /// Comincia la bella copia o «Disponi sulle righe»: si sceglie la zona.
    func startNeat(_ kind: NeatKind) {
        if kind == .fair && ai.services.allSatisfy({ !$0.enabled }) {
            show(L("Nessun servizio AI: aggiungine uno in Impostazioni.", "No AI service yet: add one in Settings."))
            return
        }
        if kind == .rules && !paperHasRows(paper: paperCode) {
            show(neatIssueText(.unsupported))
            return
        }
        neatGen += 1
        eraserOn = false
        applyTools()
        zoneJob = ZoneJob(kind: kind, note: L("Trascina col dito un riquadro attorno alla scrittura, o scegli tutta la pagina.",
                                               "Drag a box around the writing with your finger, or choose the whole page."))
    }

    func neatCancel() {
        neatGen += 1
        zoneJob = nil
    }

    /// Il lavoro in un thread; poi l'anteprima (o il perché no).
    private func neatRun(_ job: ZoneJob, _ work: @escaping () throws -> BoardPreview) {
        neatGen += 1
        let gen = neatGen
        var j = job
        j.phase = .working
        j.drag = nil
        zoneJob = j
        DispatchQueue.global(qos: .userInitiated).async {
            let r = Result { try work() }
            DispatchQueue.main.async { [self] in
                guard gen == neatGen, var cur = zoneJob else { return }
                switch r {
                case .success(let p):
                    cur.phase = .preview
                    cur.preview = p
                    cur.every = p.every ?? cur.every
                    cur.note = neatDoneText(cur.kind, p)
                    zoneJob = cur
                case .failure(let e):
                    guard let why = neatErrorText(e) else { zoneJob = nil; return }
                    show(why)
                    cur.phase = .select
                    cur.preview = nil
                    cur.zone = nil
                    cur.fair = nil
                    cur.note = why
                    zoneJob = cur
                }
            }
        }
    }

    /// La zona scelta (nil = tutta la pagina).
    func neatZone(_ r: [Float]?) {
        guard var job = zoneJob else { return }
        let zone = r.map { Zone(u0: $0[0], v0: $0[1], u1: $0[2], v1: $0[3]) }
        let model = board.model, v = view
        job.zone = zone
        switch job.kind {
        case .rules:
            let every = job.every ?? (paperIsGrid(paper: paperCode) ? boardSuggestEvery(board: model, view: v, zone: zone, paper: paperCode) : nil)
            job.every = every
            job.note = L("Dispongo…", "Arranging…")
            let spec = sheetSpec(every)
            neatRun(job) { try boardArrange(board: model, view: v, zone: zone, sheet: spec, commit: false) }
        case .fair:
            let fj = FairJob(board: model, view: v, zone: zone)
            job.fair = fj
            job.note = L("Leggo la scrittura…", "Reading the writing…")
            let providers = ai.providers()
            let onRules = fairOnRules && paperHasRows(paper: paperCode)
            let spec = sheetSpec(job.every)
            let gen = neatGen + 1
            let listener = NeatListener(alive: { [weak self] in self?.neatGen == gen }) { [weak self] t in
                guard let self, self.neatGen == gen else { return }
                self.zoneJob?.note = t
            }
            neatRun(job) {
                if fj.needsAi() { _ = try fj.read(providers: providers, listener: listener) }
                return try fj.preview(onRules: onRules, sheet: spec)
            }
        }
    }

    /// L'anteprima di nuovo, cambiata un'opzione.
    func neatRefresh(_ job: ZoneJob) {
        let model = board.model, v = view, spec = sheetSpec(job.every)
        switch job.kind {
        case .rules: neatRun(job) { try boardArrange(board: model, view: v, zone: job.zone, sheet: spec, commit: false) }
        case .fair:
            guard let fj = job.fair else { return }
            let onRules = fairOnRules && paperHasRows(paper: paperCode)
            neatRun(job) { try fj.preview(onRules: onRules, sheet: spec) }
        }
    }

    /// Accetta: il lavoro rifatto sul foglio com'è adesso, una voce di annullamento.
    func neatAccept() {
        guard let job = zoneJob else { return }
        let r = Result { () throws -> BoardPreview in
            switch job.kind {
            case .rules: return try boardArrange(board: board.model, view: view, zone: job.zone, sheet: sheetSpec(job.every), commit: true)
            case .fair: return try job.fair!.commit(onRules: fairOnRules && paperHasRows(paper: paperCode), sheet: sheetSpec(job.every))
            }
        }
        neatCancel()
        board.reload()
        switch r {
        case .success: show(L("Fatto: Annulla riporta la scrittura di prima.", "Done: Undo brings back the previous writing."))
        case .failure(let e): if let why = neatErrorText(e) { show(why) }
        }
    }
}

/// La barra durante la bella copia o «Disponi sulle righe», al posto degli strumenti.
struct NeatBar: View {
    @EnvironmentObject var app: AppModel

    var body: some View {
        if let job = app.zoneJob {
            let grid = paperIsGrid(paper: app.paperCode) && (job.kind == .rules || app.fairOnRules)
            VStack(alignment: .leading, spacing: 4) {
                HStack(spacing: 8) {
                    if job.phase == .working { ProgressView().controlSize(.small) }
                    Text(job.kind == .fair ? L("Bella copia", "Fair copy") : L("Sulle righe", "On the lines")).font(.subheadline.bold())
                }
                Text(job.note).font(.caption).foregroundStyle(.secondary).lineLimit(3)
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 8) {
                        switch job.phase {
                        case .select:
                            Button { app.neatZone(nil) } label: { Label(L("Tutta la pagina", "Whole page"), systemImage: "selection.pin.in.out") }
                                .buttonStyle(.bordered)
                        case .preview:
                            Button(L("Accetta", "Accept")) { app.neatAccept() }.buttonStyle(.borderedProminent)
                            if job.kind == .fair {
                                Button(L("Rileggi", "Read again")) { app.neatZone(job.zone?.floats) }.buttonStyle(.bordered)
                            }
                        case .working: EmptyView()
                        }
                        if job.kind == .fair && paperHasRows(paper: app.paperCode) && job.phase != .working {
                            Toggle(isOn: Binding(get: { app.fairOnRules }, set: { on in
                                app.fairOnRules = on
                                if job.phase == .preview { app.neatRefresh(job) }
                            })) { Label(L("Sulle righe", "On the lines"), systemImage: "text.alignleft") }
                                .toggleStyle(.button)
                        }
                        if grid && job.phase != .working {
                            Text(L("Quadretti per riga", "Squares per line")).font(.caption)
                            ForEach([UInt32(1), 2, 3], id: \.self) { n in
                                Button("\(n)") {
                                    var j = job
                                    j.every = n
                                    app.zoneJob = j
                                    if j.phase == .preview { app.neatRefresh(j) }
                                }
                                .buttonStyle(.bordered)
                                .tint(job.every == n ? .accentColor : .secondary)
                            }
                        }
                        Button(job.phase == .preview ? L("Rifiuta", "Reject") : L("Annulla", "Cancel"), role: .cancel) { app.neatCancel() }
                    }
                }
            }
            .padding(.horizontal, 12).padding(.vertical, 8)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(.bar)
        }
    }
}
