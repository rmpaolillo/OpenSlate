package org.openslate.android

import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.journeyapps.barcodescanner.ScanContract
import com.journeyapps.barcodescanner.ScanOptions

/** Impostazioni › Sincronizza col computer: abbinamento col QR e stato. */
@Composable
fun SyncScreen(sync: PhoneSync, onBack: () -> Unit) {
    BackHandler(onBack = onBack)
    val ctx = androidx.compose.ui.platform.LocalContext.current
    var msg by remember { mutableStateOf("") }
    val scan = rememberLauncherForActivityResult(ScanContract()) { r ->
        val text = r.contents ?: return@rememberLauncherForActivityResult
        sync.pair(text).onFailure { msg = it.message.orEmpty() }.onSuccess { msg = "" }
    }
    Column(Modifier.fillMaxSize()) {
        ActionRow {
            TextButton(onClick = onBack) { Text("← " + stringResource(R.string.back)) }
            Text(stringResource(R.string.sync_title), style = MaterialTheme.typography.titleLarge)
        }
        Column(Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            Text(stringResource(R.string.sync_explain), style = MaterialTheme.typography.bodyMedium)
            val desk = sync.deskName
            if (desk == null) {
                Text(stringResource(R.string.sync_where), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                Button(onClick = {
                    scan.launch(ScanOptions().apply {
                        setDesiredBarcodeFormats(ScanOptions.QR_CODE)
                        setPrompt(ctx.getString(R.string.sync_where))
                        setBeepEnabled(false)
                        setOrientationLocked(true)
                    })
                }, Modifier.fillMaxWidth()) { Text(stringResource(R.string.sync_scan)) }
            } else {
                Text(stringResource(R.string.sync_connected, desk), style = MaterialTheme.typography.titleMedium)
                Text(
                    when {
                        sync.busy -> stringResource(R.string.sync_busy)
                        else -> sync.lastLabel()?.let { stringResource(R.string.sync_last, it) } ?: stringResource(R.string.sync_never)
                    },
                    style = MaterialTheme.typography.bodyMedium,
                )
                sync.lastError?.let {
                    Text(stringResource(R.string.sync_error, it), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
                }
                Button(onClick = { sync.syncSoon(0) }, enabled = !sync.busy, modifier = Modifier.fillMaxWidth()) { Text(stringResource(R.string.sync_now)) }
                OutlinedButton(onClick = { sync.unpair() }, Modifier.fillMaxWidth()) { Text(stringResource(R.string.sync_unpair)) }
            }
            if (msg.isNotEmpty()) Text(msg, color = MaterialTheme.colorScheme.error)
        }
    }
}
