package org.openslate.android

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.media.MediaCodec
import android.media.MediaCodecInfo
import android.media.MediaFormat
import android.media.MediaMuxer
import uniffi.openslate_mobile.Page
import uniffi.openslate_mobile.animPageAt
import uniffi.openslate_mobile.videoTiming
import java.io.File

/**
 * Il video MP4 della pagina che si riscrive da sola, come `src/video.rs`
 * della desktop (stessi tempi: la pagina si scrive, poi resta ferma), ma
 * codificato con il codificatore H.264 di Android: niente librerie in più.
 * I fotogrammi sono la pagina a metà scrittura ([animPageAt], dal cuore Rust).
 */
object Mp4 {
    private const val LONG = 1080f
    private const val TIMEOUT_US = 10_000L

    fun write(page: Page, quarter: Int, out: File) {
        val (drawS, holdS, fpsF) = videoTiming()
        val fps = fpsF.toInt()
        val (rw, rh) = Ink.rotated(page.width, page.height, quarter)
        val k = LONG / maxOf(rw, rh)
        // H.264 vuole lati pari (meglio multipli di 16)
        val w = ((rw * k).toInt() / 16) * 16
        val h = ((rh * k).toInt() / 16) * 16
        val format = MediaFormat.createVideoFormat(MediaFormat.MIMETYPE_VIDEO_AVC, w, h).apply {
            setInteger(MediaFormat.KEY_COLOR_FORMAT, MediaCodecInfo.CodecCapabilities.COLOR_FormatYUV420Flexible)
            setInteger(MediaFormat.KEY_BIT_RATE, 4_000_000)
            setInteger(MediaFormat.KEY_FRAME_RATE, fps)
            setInteger(MediaFormat.KEY_I_FRAME_INTERVAL, 1)
        }
        val codec = MediaCodec.createEncoderByType(MediaFormat.MIMETYPE_VIDEO_AVC)
        codec.configure(format, null, null, MediaCodec.CONFIGURE_FLAG_ENCODE)
        codec.start()
        val muxer = MediaMuxer(out.path, MediaMuxer.OutputFormat.MUXER_OUTPUT_MPEG_4)
        var track = -1
        val info = MediaCodec.BufferInfo()
        val bmp = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888)
        val canvas = Canvas(bmp)
        val argb = IntArray(w * h)
        val draw = Math.round(drawS * fps)
        val total = Math.round((drawS + holdS) * fps)

        fun drain(end: Boolean) {
            while (true) {
                val i = codec.dequeueOutputBuffer(info, TIMEOUT_US)
                when {
                    i == MediaCodec.INFO_TRY_AGAIN_LATER -> if (!end) return
                    i == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED -> {
                        track = muxer.addTrack(codec.outputFormat)
                        muxer.start()
                    }
                    i >= 0 -> {
                        val buf = codec.getOutputBuffer(i)!!
                        if (info.flags and MediaCodec.BUFFER_FLAG_CODEC_CONFIG != 0) info.size = 0
                        if (info.size > 0 && track >= 0) muxer.writeSampleData(track, buf, info)
                        codec.releaseOutputBuffer(i, false)
                        if (info.flags and MediaCodec.BUFFER_FLAG_END_OF_STREAM != 0) return
                    }
                }
            }
        }

        try {
            for (f in 0 until total) {
                // la pagina si scrive per `draw` fotogrammi, poi resta ferma
                if (f <= draw) {
                    val t = (f.toFloat() / draw).coerceAtMost(1f)
                    val p = animPageAt(page, quarter.toUInt(), t)
                    canvas.drawColor(Color.WHITE)
                    Ink.draw(canvas, p.strokes, page.width, page.height, quarter, w.toFloat(), h.toFloat())
                    bmp.getPixels(argb, 0, w, 0, 0, w, h)
                }
                var idx: Int
                do {
                    idx = codec.dequeueInputBuffer(TIMEOUT_US)
                    if (idx < 0) drain(false)
                } while (idx < 0)
                val img = codec.getInputImage(idx)!!
                toYuv(argb, w, h, img)
                codec.queueInputBuffer(idx, 0, w * h * 3 / 2, f * 1_000_000L / fps, 0)
                drain(false)
            }
            var idx: Int
            do {
                idx = codec.dequeueInputBuffer(TIMEOUT_US)
                if (idx < 0) drain(false)
            } while (idx < 0)
            codec.queueInputBuffer(idx, 0, 0, total * 1_000_000L / fps, MediaCodec.BUFFER_FLAG_END_OF_STREAM)
            drain(true)
        } finally {
            bmp.recycle()
            runCatching { codec.stop() }
            codec.release()
            runCatching { if (track >= 0) muxer.stop() }
            muxer.release()
        }
    }

    /** ARGB → YUV 4:2:0 nei piani dell'immagine del codificatore (qualsiasi disposizione). */
    private fun toYuv(argb: IntArray, w: Int, h: Int, img: android.media.Image) {
        val (yP, uP, vP) = img.planes
        val y = yP.buffer
        val u = uP.buffer
        val v = vP.buffer
        for (r in 0 until h) {
            val row = r * w
            val yRow = r * yP.rowStride
            for (c in 0 until w) {
                val px = argb[row + c]
                val R = (px shr 16) and 0xff
                val G = (px shr 8) and 0xff
                val B = px and 0xff
                y.put(yRow + c * yP.pixelStride, (((66 * R + 129 * G + 25 * B + 128) shr 8) + 16).toByte())
                if (r % 2 == 0 && c % 2 == 0) {
                    val U = ((-38 * R - 74 * G + 112 * B + 128) shr 8) + 128
                    val V = ((112 * R - 94 * G - 18 * B + 128) shr 8) + 128
                    u.put((r / 2) * uP.rowStride + (c / 2) * uP.pixelStride, U.toByte())
                    v.put((r / 2) * vP.rowStride + (c / 2) * vP.pixelStride, V.toByte())
                }
            }
        }
    }
}
