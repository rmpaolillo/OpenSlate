package org.openslate.android

import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.Color
import android.graphics.pdf.PdfDocument
import androidx.core.content.FileProvider
import uniffi.openslate_mobile.Page
import java.io.File

/**
 * Esporta una pagina in PDF vettoriale o PNG e la passa al "Condividi" di
 * Android (email, WhatsApp, Drive, stampa…). I file stanno nella cache
 * dell'app, in `share/`, e vengono sostituiti a ogni esportazione.
 */
object Export {
    /** Lato lungo del PDF: A4 (842 punti). */
    private const val PDF_LONG = 842f
    private const val PNG_LONG = 2400f

    private fun size(page: Page, quarter: Int, long: Float): Pair<Float, Float> {
        val (rw, rh) = Ink.rotated(page.width, page.height, quarter)
        val k = long / maxOf(rw, rh)
        return rw * k to rh * k
    }

    private fun shareDir(ctx: Context) = File(ctx.cacheDir, "share").apply {
        deleteRecursively()
        mkdirs()
    }

    /** Nome del file: il titolo, ripulito, o "OpenSlate-<id>". */
    private fun fileName(page: Page, ext: String): String {
        val t = page.title.trim().replace(Regex("""[\\/:*?"<>|\n\r\t]"""), "_").take(80)
        return (t.ifEmpty { "OpenSlate-" + page.id.ifEmpty { "pagina" } }) + ".$ext"
    }

    fun pdf(ctx: Context, page: Page, quarter: Int): File {
        val (w, h) = size(page, quarter, PDF_LONG)
        val doc = PdfDocument()
        val p = doc.startPage(PdfDocument.PageInfo.Builder(w.toInt(), h.toInt(), 1).create())
        Ink.draw(p.canvas, page.strokes, page.width, page.height, quarter, w, h)
        doc.finishPage(p)
        val f = File(shareDir(ctx), fileName(page, "pdf"))
        f.outputStream().use { doc.writeTo(it) }
        doc.close()
        return f
    }

    fun png(ctx: Context, page: Page, quarter: Int): File {
        val (w, h) = size(page, quarter, PNG_LONG)
        val bmp = Bitmap.createBitmap(w.toInt(), h.toInt(), Bitmap.Config.ARGB_8888)
        val c = android.graphics.Canvas(bmp)
        c.drawColor(Color.WHITE)
        Ink.draw(c, page.strokes, page.width, page.height, quarter, w, h)
        val f = File(shareDir(ctx), fileName(page, "png"))
        f.outputStream().use { bmp.compress(Bitmap.CompressFormat.PNG, 100, it) }
        bmp.recycle()
        return f
    }

    /** La pagina come PNG per l'AI (lato lungo 1600, sfondo bianco, ruotata come la si vede). */
    fun pngBytes(page: Page, quarter: Int, long: Float = 1600f): ByteArray {
        val (w, h) = size(page, quarter, long)
        val bmp = Bitmap.createBitmap(w.toInt(), h.toInt(), Bitmap.Config.ARGB_8888)
        val c = android.graphics.Canvas(bmp)
        c.drawColor(Color.WHITE)
        Ink.draw(c, page.strokes, page.width, page.height, quarter, w, h)
        val out = java.io.ByteArrayOutputStream()
        bmp.compress(Bitmap.CompressFormat.PNG, 100, out)
        bmp.recycle()
        return out.toByteArray()
    }

    fun share(ctx: Context, file: File, mime: String) {
        val uri = FileProvider.getUriForFile(ctx, ctx.packageName + ".files", file)
        val send = Intent(Intent.ACTION_SEND).apply {
            type = mime
            putExtra(Intent.EXTRA_STREAM, uri)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        }
        ctx.startActivity(Intent.createChooser(send, file.name))
    }
}
