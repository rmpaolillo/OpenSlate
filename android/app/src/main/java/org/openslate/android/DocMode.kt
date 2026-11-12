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
import uniffi.openslate_mobile.PageStroke
import uniffi.openslate_mobile.PdfPageSize
import uniffi.openslate_mobile.docPageRect
import uniffi.openslate_mobile.docExtensions
import uniffi.openslate_mobile.docImport
import uniffi.openslate_mobile.docOpen
import uniffi.openslate_mobile.docSave
import uniffi.openslate_mobile.docSign
import uniffi.openslate_mobile.docSignedExt
import uniffi.openslate_mobile.pdfSign
import java.io.File

/**
 * Un documento da firmare col pennino (PDF, Word, foto, SVG), come «Apri
 * documento» della desktop e nello stesso formato (`src/docstore.rs`), così
 * si sincronizza col computer: vive in `documents/<id>/` con `doc.json` (i
 * tratti di ogni pagina), `source.pdf` (le pagine da mostrare) e il file di
 * partenza (`source.<ext>`, mai modificato). La copia firmata resta nel
 * formato dell'originale.
 */
class OpenDoc private constructor(
    val dir: File,
    val title: String,
    val original: File,
    val sizes: List<PdfPageSize>,
    pages: List<List<PageStroke>>,
    start: Int,
) {
    val pdf get() = File(dir, "source.pdf")
    val id: String get() = dir.name
    /** Pagina mostrata (da 0). */
    var index by mutableIntStateOf(start.coerceIn(0, (sizes.size - 1).coerceAtLeast(0)))
        private set
    /** L'immagine della pagina mostrata, per lo sfondo del foglio. */
    var image by mutableStateOf<ImageBitmap?>(null)
        private set
    private val strokes = pages.toMutableList()

    val count get() = sizes.size

    /** Pagina `i` sullo sfondo e i suoi tratti sulla lavagna. */
    fun show(i: Int, board: Board) {
        index = i.coerceIn(0, count - 1)
        board.openStrokes(strokes[index])
        image = render(index)
    }

    /** Ricorda i tratti della pagina mostrata (in memoria e in `doc.json`). */
    fun keep(board: Board, view: BoardView) {
        strokes[index] = board.model.strokes()
        runCatching { docSave(dir.path, strokes, index.toUInt(), view) }
        board.markSaved()
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

        /** La cartella dei documenti. */
        fun root(ctx: Context) = File(ctx.filesDir, "documents")

        /**
         * Copia il documento scelto nell'app e prepara le pagine da mostrare
         * (un Word lo impagina WordCraft, una foto PhotoCraft…). Lento per i
         * Word lunghi: da chiamare fuori dal thread principale.
         */
        fun import(ctx: Context, uri: Uri, view: BoardView): OpenDoc {
            val name = ctx.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use {
                if (it.moveToFirst()) it.getString(0) else null
            } ?: "documento.pdf"
            val ext = name.substringAfterLast('.', "").lowercase()
            require(ext in docExtensions()) { "formato .$ext non supportato" }
            val tmp = File(ctx.cacheDir, "import.$ext")
            ctx.contentResolver.openInputStream(uri)!!.use { input -> tmp.outputStream().use { input.copyTo(it) } }
            try {
                val id = docImport(root(ctx).path, tmp.path, name)
                return open(File(root(ctx), id), view)
            } finally {
                tmp.delete()
            }
        }

        /** Riapre un documento, coi tratti portati nella geometria di `view`. */
        fun open(dir: File, view: BoardView): OpenDoc {
            val d = docOpen(dir.path, view)
            require(d.sizes.isNotEmpty()) { "documento senza pagine" }
            return OpenDoc(dir, d.title, File(d.original), d.sizes, d.pages, d.current.toInt())
        }
    }
}
