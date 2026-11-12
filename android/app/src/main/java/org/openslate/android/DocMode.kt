package org.openslate.android

import android.content.Context
import android.graphics.Bitmap
import android.graphics.Color
import android.graphics.pdf.PdfRenderer
import android.net.Uri
import android.os.ParcelFileDescriptor
import android.provider.OpenableColumns
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import uniffi.openslate_mobile.BoardView
import uniffi.openslate_mobile.Page
import uniffi.openslate_mobile.PageStroke
import uniffi.openslate_mobile.PdfPageSize
import uniffi.openslate_mobile.docPageRect
import uniffi.openslate_mobile.libraryLoad
import uniffi.openslate_mobile.librarySave
import uniffi.openslate_mobile.nowCreated
import uniffi.openslate_mobile.pdfPages
import uniffi.openslate_mobile.docExtensions
import uniffi.openslate_mobile.docPrepare
import uniffi.openslate_mobile.docSign
import uniffi.openslate_mobile.docSignedExt
import uniffi.openslate_mobile.pdfSign
import java.io.File

/**
 * Un documento da firmare col pennino (PDF, Word, foto, SVG), come «Apri
 * documento» della desktop: ogni pagina sta sul foglio del pad (adattata e
 * centrata) e si scrive sopra. Vive in `documents/<id>/`: l'originale (mai
 * modificato), `source.pdf` (le pagine da mostrare) e i tratti di ogni
 * pagina (`pNNNN.json`, formato della libreria). La copia firmata resta nel
 * formato dell'originale.
 */
class OpenDoc private constructor(val dir: File, val title: String, val original: File, val sizes: List<PdfPageSize>) {
    val pdf get() = File(dir, "source.pdf")
    /** Pagina mostrata (da 0). */
    var index by mutableIntStateOf(0)
        private set
    /** L'immagine della pagina mostrata, per lo sfondo del foglio. */
    var image by mutableStateOf<ImageBitmap?>(null)
        private set
    private val strokes = MutableList(sizes.size) { emptyList<PageStroke>() }

    val count get() = sizes.size

    private fun pageId(i: Int) = "p%04d".format(i + 1)

    /** Pagina `i` sullo sfondo e i suoi tratti sulla lavagna. */
    fun show(i: Int, board: Board) {
        index = i.coerceIn(0, count - 1)
        board.openStrokes(strokes[index])
        image = render(index)
    }

    /** Ricorda i tratti della pagina mostrata (in memoria e su disco). */
    fun keep(board: Board) {
        val s = board.model.strokes()
        strokes[index] = s
        runCatching {
            librarySave(dir.path, Page(pageId(index), "", nowCreated(), "documento", null, 0f, 0f, s))
        }
    }

    /** Dove sta la pagina sul foglio (u, v, larghezza, altezza in 0..1). */
    fun rect(view: BoardView): List<Float> = docPageRect(sizes[index], view)

    /** La pagina come immagine, larga 1600 px (nitida anche ingrandita). */
    private fun render(i: Int): ImageBitmap? = runCatching {
        ParcelFileDescriptor.open(pdf, ParcelFileDescriptor.MODE_READ_ONLY).use { fd ->
            PdfRenderer(fd).use { r ->
                r.openPage(i).use { p ->
                    val w = 1600
                    val h = (w * sizes[i].height / sizes[i].width).toInt().coerceAtLeast(1)
                    val bmp = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888)
                    bmp.eraseColor(Color.WHITE)
                    p.render(bmp, null, null, PdfRenderer.Page.RENDER_MODE_FOR_DISPLAY)
                    bmp.asImageBitmap()
                }
            }
        }
    }.getOrNull()

    /** Estensione e tipo della copia firmata (un Word resta Word…). */
    val signedExt: String get() = docSignedExt(original.path)
    val signedMime: String
        get() = android.webkit.MimeTypeMap.getSingleton().getMimeTypeFromExtension(signedExt) ?: "application/octet-stream"

    /** Scrive in `dst` la copia firmata (tutte le pagine). */
    fun sign(dst: File, view: BoardView) = docSign(original.path, pdf.path, dst.path, strokes, view)

    /** Il PDF da stampare: le pagine come si vedono, con la scrittura sopra (anche per Word e foto). */
    fun printPdf(dst: File, view: BoardView) = pdfSign(pdf.path, dst.path, strokes, view)

    val hasInk get() = strokes.any { it.isNotEmpty() }

    companion object {
        /** Tipi di file per il selettore. */
        val MIME = arrayOf(
            "application/pdf",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "application/msword",
            "application/vnd.oasis.opendocument.text",
            "application/rtf", "text/rtf",
            "image/*",
        )

        /**
         * Copia il documento scelto nell'app e prepara le pagine da mostrare
         * (un Word lo impagina WordCraft, una foto PhotoCraft…). Lento per i
         * Word lunghi: da chiamare fuori dal thread principale.
         */
        fun import(ctx: Context, uri: Uri): OpenDoc {
            val name = ctx.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use {
                if (it.moveToFirst()) it.getString(0) else null
            } ?: "documento.pdf"
            val ext = name.substringAfterLast('.', "").lowercase()
            require(ext in docExtensions()) { "formato .$ext non supportato" }
            val dir = File(ctx.filesDir, "documents/" + System.currentTimeMillis())
            dir.mkdirs()
            val original = File(dir, "original.$ext")
            ctx.contentResolver.openInputStream(uri)!!.use { input -> original.outputStream().use { input.copyTo(it) } }
            runCatching { docPrepare(original.path, File(dir, "source.pdf").path) }.onFailure {
                dir.deleteRecursively()
                throw it
            }
            File(dir, "title.txt").writeText(name.substringBeforeLast('.'))
            return open(dir)
        }

        /** Riapre un documento già importato, con i suoi tratti. */
        fun open(dir: File): OpenDoc {
            val title = File(dir, "title.txt").takeIf { it.exists() }?.readText() ?: dir.name
            val sizes = pdfPages(File(dir, "source.pdf").path)
            require(sizes.isNotEmpty()) { "documento senza pagine" }
            // i primi documenti avevano solo source.pdf
            val original = dir.listFiles()?.firstOrNull { it.name.startsWith("original.") } ?: File(dir, "source.pdf")
            val d = OpenDoc(dir, title, original, sizes)
            for (i in sizes.indices) {
                runCatching { libraryLoad(dir.path, d.pageId(i)).strokes }.getOrNull()?.let { d.strokes[i] = it }
            }
            return d
        }
    }
}
