package org.openslate.android

import android.graphics.Canvas
import android.graphics.Paint
import uniffi.openslate_mobile.PaperInk
import uniffi.openslate_mobile.PaperKind
import uniffi.openslate_mobile.PaperMark
import uniffi.openslate_mobile.paperKinds

/**
 * Disegna la rigatura del foglio (geometria da `src/rigatura.rs`, la stessa
 * della desktop) con i colori e le sfumature di `src/ui/paper.rs`. Solo sullo
 * schermo: non va nei PDF e nelle immagini, come sulla desktop.
 */
object PaperDraw {
    val kinds: List<PaperKind> by lazy { paperKinds() }

    fun kind(code: String) = kinds.firstOrNull { it.code == code } ?: kinds.first()

    /**
     * `pxPerCm`: pixel dello schermo per centimetro del foglio; `density`:
     * pixel per dp (gli spessori della desktop sono in punti ≈ dp).
     * `label` traduce le scritte (chiavi italiane della desktop).
     */
    fun draw(
        c: Canvas,
        marks: List<PaperMark>,
        kind: PaperKind,
        left: Float,
        top: Float,
        pxPerCm: Float,
        density: Float,
        label: (String) -> String,
    ) {
        if (marks.isEmpty()) return
        val pxDp = pxPerCm / density // punti per cm, come sulla desktop
        val fade = { cm: Float -> (cm * pxDp / 8f).coerceIn(0f, 1f) }
        val fLine = fade(kind.pitch)
        val fFine = fade(0.1f)
        val mm = kind.code == "millimetrata"
        val rgb = when {
            mm -> Triple(225, 130, 60)
            kind.gridLike -> Triple(110, 150, 205)
            else -> Triple(80, 125, 200)
        }
        fun tint(a: Float) = android.graphics.Color.argb(a.coerceIn(0f, 255f).toInt(), rgb.first, rgb.second, rgb.third)
        val lineAlpha = if (mm) 75f else if (kind.gridLike) 95f else 130f
        val strongAlpha = if (mm) 140f else 175f
        val axis = android.graphics.Color.argb(210, 60, 70, 95)
        val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.STROKE }
        /** Spessore (punti) e colore di una linea, o null se non si vede. */
        fun style(ink: PaperInk): Pair<Float, Int>? = when (ink) {
            PaperInk.FINE -> if (fFine >= 0.35f) 0.6f to tint(80f * fFine) else null
            PaperInk.LINE -> if (fLine > 0.05f) 1.0f to tint(lineAlpha * fLine) else null
            PaperInk.STRONG -> 1.3f to tint(strongAlpha * maxOf(fLine, 0.5f))
            PaperInk.RED -> 1.2f to android.graphics.Color.argb(150, 215, 70, 70)
            PaperInk.AXIS -> 1.4f to axis
        }
        val dotR = (0.04f * pxDp).coerceIn(0.8f, 2.2f) * density
        val text = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            textAlign = Paint.Align.CENTER
            textSize = (0.3f * pxDp).coerceIn(9f, 15f) * density
            color = if (kind.code == "cartesiano") axis else tint(200f)
        }
        val fill = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL; color = tint(170f * fLine) }
        for (m in marks) {
            when (m) {
                is PaperMark.Line -> style(m.ink)?.let { (w, col) ->
                    paint.strokeWidth = w * density
                    paint.color = col
                    c.drawLine(left + m.x0 * pxPerCm, top + m.y0 * pxPerCm, left + m.x1 * pxPerCm, top + m.y1 * pxPerCm, paint)
                }
                is PaperMark.Dot -> if (fLine > 0.05f) c.drawCircle(left + m.x * pxPerCm, top + m.y * pxPerCm, dotR, fill)
                is PaperMark.Label -> if (pxDp >= 12f) {
                    val y = top + m.y * pxPerCm - (text.ascent() + text.descent()) / 2
                    c.drawText(label(m.text), left + m.x * pxPerCm, y, text)
                }
            }
        }
    }
}
