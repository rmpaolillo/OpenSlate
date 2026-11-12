package org.openslate.android

import android.app.Activity
import android.content.Context
import android.os.Bundle
import android.os.CancellationSignal
import android.os.ParcelFileDescriptor
import android.print.PageRange
import android.print.PrintAttributes
import android.print.PrintDocumentAdapter
import android.print.PrintDocumentInfo
import android.print.PrintManager
import java.io.File
import java.io.FileOutputStream

/**
 * Stampa un PDF già pronto (il documento con la scrittura, o la pagina della
 * lavagna) col servizio di stampa di Android: stampanti di rete, copie,
 * pagine, formato e «Salva come PDF».
 */
object Print {
    fun pdf(activity: Activity, file: File, title: String) {
        val pm = activity.getSystemService(Context.PRINT_SERVICE) as PrintManager
        pm.print(title, Adapter(file), PrintAttributes.Builder().build())
    }

    private class Adapter(private val file: File) : PrintDocumentAdapter() {
        override fun onLayout(
            oldAttributes: PrintAttributes?, newAttributes: PrintAttributes, cancel: CancellationSignal?,
            callback: LayoutResultCallback, extras: Bundle?,
        ) {
            if (cancel?.isCanceled == true) {
                callback.onLayoutCancelled()
                return
            }
            // il PDF è già fatto: il servizio di stampa lo adatta al foglio
            val info = PrintDocumentInfo.Builder(file.name).setContentType(PrintDocumentInfo.CONTENT_TYPE_DOCUMENT).build()
            callback.onLayoutFinished(info, oldAttributes != newAttributes)
        }

        override fun onWrite(pages: Array<out PageRange>, dest: ParcelFileDescriptor, cancel: CancellationSignal?, callback: WriteResultCallback) {
            runCatching {
                file.inputStream().use { input -> FileOutputStream(dest.fileDescriptor).use { input.copyTo(it) } }
            }.onSuccess {
                if (cancel?.isCanceled == true) callback.onWriteCancelled() else callback.onWriteFinished(arrayOf(PageRange.ALL_PAGES))
            }.onFailure { callback.onWriteFailed(it.message) }
        }
    }
}
