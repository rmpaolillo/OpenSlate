package org.openslate.android

import android.graphics.Canvas
import android.graphics.Paint
import uniffi.openslate_mobile.PageStroke
import uniffi.openslate_mobile.Phase

/** Inchiostro e spessore predefiniti (come la desktop). */
const val INK = "#141414"
const val PEN_WIDTH = 2f

/**
 * Disegna i tratti di una pagina su un Canvas Android: lo stesso codice per lo
 * schermo, il PDF e l'immagine. Le coordinate sono del pad (`padW` x `padH`),
 * ruotate di `quarter` quarti di giro e scalate su `dstW` x `dstH`.
 */
object Ink {
    /** Dimensioni del foglio ruotato, in unità del pad. */
    fun rotated(padW: Float, padH: Float, quarter: Int) =
        if (quarter % 2 == 1) padH to padW else padW to padH

    fun draw(c: Canvas, strokes: List<PageStroke>, padW: Float, padH: Float, quarter: Int, dstW: Float, dstH: Float) {
        val (rw, rh) = rotated(padW, padH, quarter)
        val k = minOf(dstW / rw, dstH / rh)
        // Spessore relativo come la desktop: pixel su un foglio alto 800 px.
        val unit = rh * k / 800f
        val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            style = Paint.Style.STROKE
            strokeCap = Paint.Cap.ROUND
            strokeJoin = Paint.Join.ROUND
        }
        fun mx(x: Float, y: Float) = when (quarter) {
            0 -> x
            1 -> padH - y
            2 -> padW - x
            else -> y
        } * k
        fun my(x: Float, y: Float) = when (quarter) {
            0 -> y
            1 -> x
            2 -> padH - y
            else -> padW - x
        } * k
        // L'evidenziatore sta sotto l'inchiostro.
        for (pass in 0..1) {
            for (s in strokes) {
                if (s.highlight != (pass == 0) || s.points.isEmpty()) continue
                paint.color = parseColor(s.color)
                if (s.highlight) paint.alpha = 90
                val base = s.width * unit
                val pts = s.points
                if (pts.size == 1) {
                    paint.strokeWidth = base
                    c.drawPoint(mx(pts[0].x, pts[0].y), my(pts[0].x, pts[0].y), paint)
                    continue
                }
                for (i in 1 until pts.size) {
                    val a = pts[i - 1]
                    val b = pts[i]
                    val pressure = if (b.phase == Phase.UP || s.highlight) a.pressure else b.pressure
                    paint.strokeWidth = if (s.highlight) base else base * (0.4f + 0.8f * pressure)
                    c.drawLine(mx(a.x, a.y), my(a.x, a.y), mx(b.x, b.y), my(b.x, b.y), paint)
                }
            }
        }
    }

    private fun parseColor(hex: String): Int =
        runCatching { android.graphics.Color.parseColor(hex) }.getOrDefault(0xFF141414.toInt())
}
