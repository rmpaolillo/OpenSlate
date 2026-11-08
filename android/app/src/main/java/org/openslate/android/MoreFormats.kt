package org.openslate.android

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.layout.Column
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Animation
import androidx.compose.material.icons.filled.Description
import androidx.compose.material.icons.filled.Functions
import androidx.compose.material.icons.filled.Gif
import androidx.compose.material.icons.filled.Image
import androidx.compose.material.icons.filled.MenuBook
import androidx.compose.material.icons.filled.Movie
import androidx.compose.material.icons.filled.SaveAlt
import androidx.compose.material.icons.filled.Share
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import uniffi.openslate_mobile.ExportKind
import uniffi.openslate_mobile.exportKinds

/** La voce del menu e l'icona di un formato (come il menu della desktop). */
private fun label(k: ExportKind): Int = when (k) {
    ExportKind.SVG -> R.string.fmt_svg
    ExportKind.JPEG -> R.string.fmt_jpeg
    ExportKind.WEBP -> R.string.fmt_webp
    ExportKind.WORD -> R.string.fmt_word
    ExportKind.XOURNAL -> R.string.fmt_xournal
    ExportKind.GIF -> R.string.fmt_gif
    ExportKind.SVG_ANIM -> R.string.fmt_svg_anim
    ExportKind.VIDEO -> R.string.fmt_video
    ExportKind.LATEX -> R.string.fmt_latex
    ExportKind.TYPST -> R.string.fmt_typst
}

private fun icon(k: ExportKind): ImageVector = when (k) {
    ExportKind.SVG, ExportKind.JPEG, ExportKind.WEBP -> Icons.Filled.Image
    ExportKind.WORD -> Icons.Filled.Description
    ExportKind.XOURNAL -> Icons.Filled.MenuBook
    ExportKind.GIF -> Icons.Filled.Gif
    ExportKind.SVG_ANIM -> Icons.Filled.Animation
    ExportKind.VIDEO -> Icons.Filled.Movie
    ExportKind.LATEX, ExportKind.TYPST -> Icons.Filled.Functions
}

/**
 * «Altri formati»: per ognuno, condividi (email, WhatsApp, Drive…) o salva
 * in una cartella a scelta.
 */
@Composable
fun MoreFormatsDialog(onDismiss: () -> Unit, onPick: (ExportKind, Boolean) -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.more_formats)) },
        text = {
            Column(Modifier.verticalScroll(rememberScrollState())) {
                for (k in exportKinds()) {
                    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                        Icon(icon(k), null)
                        Spacer(Modifier.width(12.dp))
                        Text(stringResource(label(k)), Modifier.weight(1f).padding(vertical = 4.dp), style = MaterialTheme.typography.bodyMedium)
                        IconButton(onClick = { onPick(k, false) }) { Icon(Icons.Filled.Share, stringResource(R.string.share)) }
                        IconButton(onClick = { onPick(k, true) }) { Icon(Icons.Filled.SaveAlt, stringResource(R.string.save_to)) }
                    }
                }
            }
        },
        confirmButton = { TextButton(onClick = onDismiss) { Text(stringResource(R.string.close)) } },
    )
}
