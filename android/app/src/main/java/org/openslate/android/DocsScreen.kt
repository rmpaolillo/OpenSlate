package org.openslate.android

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Description
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import uniffi.openslate_mobile.DocEntry
import uniffi.openslate_mobile.docList
import java.io.File

/** I documenti da firmare (anche quelli arrivati dal computer), dal più recente. */
@Composable
fun DocsScreen(root: File, version: Int, openId: String?, onBack: () -> Unit, onOpen: (File) -> Unit, onTrash: (String) -> Unit) {
    BackHandler(onBack = onBack)
    val docs = remember(version) { docList(root.path) }
    var trashing by remember { mutableStateOf<DocEntry?>(null) }
    Column(Modifier.fillMaxSize()) {
        ActionRow {
            TextButton(onClick = onBack) { Text("← " + stringResource(R.string.back)) }
            Text(stringResource(R.string.my_docs), style = MaterialTheme.typography.titleLarge)
        }
        if (docs.isEmpty()) {
            Text(stringResource(R.string.docs_empty), Modifier.padding(24.dp))
            return@Column
        }
        LazyColumn(Modifier.fillMaxSize()) {
            items(docs, key = { it.id }) { d ->
                Row(
                    Modifier.fillMaxWidth().clickable { onOpen(File(root, d.id)) }.padding(start = 16.dp, top = 6.dp, bottom = 6.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Icon(Icons.Filled.Description, null)
                    Column(Modifier.weight(1f).padding(start = 16.dp)) {
                        Text(d.title, style = MaterialTheme.typography.bodyLarge, maxLines = 1, overflow = TextOverflow.Ellipsis)
                        val when_ = d.created.take(16)
                        Text(
                            if (d.id == openId) stringResource(R.string.doc_open_now, when_) else when_,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    IconButton(onClick = { trashing = d }) { Icon(Icons.Filled.Delete, stringResource(R.string.doc_trash)) }
                }
                HorizontalDivider()
            }
        }
    }
    trashing?.let { d ->
        AlertDialog(
            onDismissRequest = { trashing = null },
            title = { Text(stringResource(R.string.doc_trash)) },
            text = { Text(stringResource(R.string.doc_trash_q, d.title)) },
            confirmButton = { TextButton(onClick = { trashing = null; onTrash(d.id) }) { Text(stringResource(R.string.doc_trash)) } },
            dismissButton = { TextButton(onClick = { trashing = null }) { Text(stringResource(R.string.cancel)) } },
        )
    }
}
