// OpenSlate: il video della pagina che si riscrive, in MP4 (H.264), con il
// codificatore di macOS (AVFoundation).
//
// uso: video_mp4 <larghezza> <altezza> <fotogrammi al secondo> <file.mp4>
// I fotogrammi arrivano su stdin uno dopo l'altro, BGRA, larghezza×altezza×4
// byte ciascuno, righe dall'alto. Alla fine scrive «ok N» (N = fotogrammi).

import AVFoundation
import CoreMedia
import CoreVideo
import Foundation

struct Failure: Error, CustomStringConvertible {
    let description: String
}

func fail(_ msg: String) -> Never {
    FileHandle.standardError.write((msg + "\n").data(using: .utf8)!)
    exit(1)
}

let args = CommandLine.arguments
guard args.count == 5, let width = Int(args[1]), let height = Int(args[2]), let fps = Int32(args[3]),
      width > 0, height > 0, fps > 0 else {
    fail("uso: video_mp4 <larghezza> <altezza> <fps> <file.mp4>")
}
let url = URL(fileURLWithPath: args[4])
try? FileManager.default.removeItem(at: url)

/// Legge esattamente `count` byte da stdin (meno solo alla fine).
func readExactly(_ count: Int) -> Data {
    var data = Data()
    data.reserveCapacity(count)
    while data.count < count {
        let chunk = FileHandle.standardInput.readData(ofLength: count - data.count)
        if chunk.isEmpty { break }
        data.append(chunk)
    }
    return data
}

do {
    let writer = try AVAssetWriter(outputURL: url, fileType: .mp4)
    let compression: [String: Any] = [
        // una pagina scritta: pochi colori, molto bianco; abbastanza per i tratti sottili
        AVVideoAverageBitRateKey: max(1_000_000, width * height * 3),
        AVVideoProfileLevelKey: AVVideoProfileLevelH264HighAutoLevel,
        AVVideoMaxKeyFrameIntervalKey: Int(fps) * 2,
    ]
    let settings: [String: Any] = [
        AVVideoCodecKey: AVVideoCodecType.h264,
        AVVideoWidthKey: width,
        AVVideoHeightKey: height,
        AVVideoCompressionPropertiesKey: compression,
    ]
    let input = AVAssetWriterInput(mediaType: .video, outputSettings: settings)
    input.expectsMediaDataInRealTime = false
    let attributes: [String: Any] = [
        kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA,
        kCVPixelBufferWidthKey as String: width,
        kCVPixelBufferHeightKey as String: height,
    ]
    let adaptor = AVAssetWriterInputPixelBufferAdaptor(assetWriterInput: input, sourcePixelBufferAttributes: attributes)
    guard writer.canAdd(input) else { throw Failure(description: "AVAssetWriter non accetta il video") }
    writer.add(input)
    guard writer.startWriting() else {
        throw writer.error ?? Failure(description: "AVAssetWriter non parte")
    }
    writer.startSession(atSourceTime: .zero)

    let rowBytes = width * 4
    let frameBytes = rowBytes * height
    var frame: Int64 = 0
    while true {
        let data = readExactly(frameBytes)
        if data.count < frameBytes { break }
        while !input.isReadyForMoreMediaData {
            Thread.sleep(forTimeInterval: 0.002)
        }
        guard let pool = adaptor.pixelBufferPool else {
            throw writer.error ?? Failure(description: "nessun pixel buffer")
        }
        var made: CVPixelBuffer?
        CVPixelBufferPoolCreatePixelBuffer(nil, pool, &made)
        guard let buffer = made else { throw Failure(description: "pixel buffer non creato") }
        CVPixelBufferLockBaseAddress(buffer, [])
        if let base = CVPixelBufferGetBaseAddress(buffer) {
            let stride = CVPixelBufferGetBytesPerRow(buffer)
            data.withUnsafeBytes { (src: UnsafeRawBufferPointer) in
                guard let from = src.baseAddress else { return }
                for y in 0..<height {
                    memcpy(base + y * stride, from + y * rowBytes, rowBytes)
                }
            }
        }
        CVPixelBufferUnlockBaseAddress(buffer, [])
        if !adaptor.append(buffer, withPresentationTime: CMTime(value: frame, timescale: fps)) {
            throw writer.error ?? Failure(description: "fotogramma \(frame) non scritto")
        }
        frame += 1
    }
    input.markAsFinished()
    let done = DispatchSemaphore(value: 0)
    writer.finishWriting { done.signal() }
    done.wait()
    if writer.status != .completed {
        throw writer.error ?? Failure(description: "video non completato")
    }
    print("ok \(frame)")
} catch {
    fail("errore: \(error.localizedDescription)")
}
