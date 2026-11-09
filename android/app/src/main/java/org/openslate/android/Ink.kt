package org.openslate.android

import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Path
import uniffi.openslate_mobile.PageStroke
import uniffi.openslate_mobile.inkOutline

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
                // tratto naturale: il contorno (dal cuore Rust, come la desktop), pieno
                val o = outline(s, base / k)
                if (o.isNotEmpty()) {
                    val path = Path()
                    path.moveTo(mx(o[0], o[1]), my(o[0], o[1]))
                    var j = 2
                    while (j + 1 < o.size) {
                        path.lineTo(mx(o[j], o[j + 1]), my(o[j], o[j + 1]))
                        j += 2
                    }
                    path.close()
                    paint.style = Paint.Style.FILL
                    c.drawPath(path, paint)
                    paint.style = Paint.Style.STROKE
                    continue
                }
                paint.strokeWidth = base
                val path = Path()
                path.moveTo(mx(pts[0].x, pts[0].y), my(pts[0].x, pts[0].y))
                for (i in 1 until pts.size) path.lineTo(mx(pts[i].x, pts[i].y), my(pts[i].x, pts[i].y))
                c.drawPath(path, paint)
            }
        }
    }

    /** Contorni già calcolati: i tratti finiti non cambiano, la lavagna si ridisegna spesso. */
    private data class Key(val n: Int, val x0: Float, val y0: Float, val x1: Float, val y1: Float, val w: Float, val natural: Boolean)
    private val cache = object : LinkedHashMap<Key, FloatArray>(256, 0.75f, true) {
        override fun removeEldestEntry(eldest: MutableMap.MutableEntry<Key, FloatArray>) = size > 4000
    }

    /** Il contorno del tratto naturale in unità del pad (vuoto: si disegna a spessore fisso). */
    private fun outline(s: PageStroke, widthPad: Float): FloatArray {
        if (s.highlight || s.points.size < 2) return FloatArray(0)
        val a = s.points.first()
        val b = s.points.last()
        val key = Key(s.points.size, a.x, a.y, b.x, b.y, widthPad, naturalInk)
        synchronized(cache) { cache[key]?.let { return it } }
        val o = if (naturalInk) inkOutline(s.points, widthPad, false).toFloatArray() else FloatArray(0)
        synchronized(cache) { cache[key] = o }
        return o
    }

    /** Tratto naturale acceso (di serie). Lo imposta MainActivity dalle preferenze. */
    @Volatile var naturalInk = true

    private fun parseColor(hex: String): Int =
        runCatching { android.graphics.Color.parseColor(hex) }.getOrDefault(0xFF141414.toInt())
}
