import AVFoundation
import Speech
import SwiftUI

/// La dettatura della chat (come `Dictation.kt` dell'app Android): il
/// riconoscimento vocale di Apple, sul telefono quando la lingua lo permette
/// (la voce non esce, come Whisper sul computer nella desktop). Il testo
/// compare nella casella mentre parli; tocchi di nuovo il microfono e si ferma.
@MainActor
final class Dictation: ObservableObject {
    @Published private(set) var listening = false
    /// Volume della voce, 0..1, per far «respirare» il microfono.
    @Published private(set) var level: CGFloat = 0
    @Published var error: String?

    private let engine = AVAudioEngine()
    private var request: SFSpeechAudioBufferRecognitionRequest?
    private var task: SFSpeechRecognitionTask?
    private var onText: ((String) -> Void)?

    func toggle(onText: @escaping (String) -> Void) {
        if listening { stop() } else { start(onText: onText) }
    }

    func start(onText: @escaping (String) -> Void) {
        error = nil
        self.onText = onText
        SFSpeechRecognizer.requestAuthorization { status in
            Task { @MainActor in
                guard status == .authorized else {
                    self.error = L("Per dettare, consenti a OpenSlate il riconoscimento vocale (Impostazioni › OpenSlate).",
                                   "To dictate, allow OpenSlate speech recognition (Settings › OpenSlate).")
                    return
                }
                AVAudioSession.sharedInstance().requestRecordPermission { ok in
                    Task { @MainActor in
                        if ok { self.begin() } else {
                            self.error = L("Per dettare, consenti a OpenSlate di usare il microfono.",
                                           "To dictate, allow OpenSlate to use the microphone.")
                        }
                    }
                }
            }
        }
    }

    private func begin() {
        guard let rec = SFSpeechRecognizer(locale: Locale.current) ?? SFSpeechRecognizer(), rec.isAvailable else {
            error = L("La dettatura non è disponibile ora.", "Dictation isn't available right now.")
            return
        }
        do {
            let session = AVAudioSession.sharedInstance()
            try session.setCategory(.record, mode: .measurement, options: .duckOthers)
            try session.setActive(true, options: .notifyOthersOnDeactivation)
            let req = SFSpeechAudioBufferRecognitionRequest()
            req.shouldReportPartialResults = true
            // sul telefono quando si può: la voce non va ai server di Apple
            if rec.supportsOnDeviceRecognition { req.requiresOnDeviceRecognition = true }
            req.addsPunctuation = true
            request = req
            let input = engine.inputNode
            let format = input.outputFormat(forBus: 0)
            input.removeTap(onBus: 0)
            input.installTap(onBus: 0, bufferSize: 1024, format: format) { [weak self] buf, _ in
                req.append(buf)
                let level = Dictation.rms(buf)
                Task { @MainActor in self?.level = level }
            }
            engine.prepare()
            try engine.start()
            listening = true
            task = rec.recognitionTask(with: req) { [weak self] result, err in
                Task { @MainActor in
                    guard let self else { return }
                    if let r = result {
                        self.onText?(r.bestTranscription.formattedString)
                        if r.isFinal { self.stop() }
                    } else if err != nil {
                        self.stop()
                    }
                }
            }
        } catch {
            self.error = L("La dettatura non è andata. Riprova.", "Dictation didn't work. Try again.")
            stop()
        }
    }

    /// Ferma l'ascolto: quello che hai detto finora resta nella casella.
    func stop() {
        if engine.isRunning {
            engine.stop()
            engine.inputNode.removeTap(onBus: 0)
        }
        request?.endAudio()
        task?.finish()
        request = nil
        task = nil
        listening = false
        level = 0
        try? AVAudioSession.sharedInstance().setActive(false, options: .notifyOthersOnDeactivation)
    }

    nonisolated private static func rms(_ buf: AVAudioPCMBuffer) -> CGFloat {
        guard let ch = buf.floatChannelData?[0], buf.frameLength > 0 else { return 0 }
        var sum: Float = 0
        for i in 0..<Int(buf.frameLength) { sum += ch[i] * ch[i] }
        let db = 20 * log10(max(sqrt(sum / Float(buf.frameLength)), 1e-6))
        return CGFloat(min(max((db + 50) / 40, 0), 1))
    }
}
