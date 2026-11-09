package org.openslate.android

import android.Manifest
import android.content.pm.PackageManager
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material.icons.filled.AddComment
import androidx.compose.material.icons.filled.AttachFile
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Mic
import androidx.compose.material.icons.filled.Stop
import androidx.compose.material.icons.filled.Tune
import androidx.compose.material.icons.outlined.Mic
import androidx.compose.material3.FilledIconButton
import androidx.compose.material3.FilterChip
import androidx.compose.material3.IconButton
import androidx.compose.material3.Surface
import androidx.compose.material3.TextField
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.draw.scale
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.core.content.ContextCompat
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.os.Handler
import android.os.Looper
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Icon
import androidx.compose.material.icons.filled.QrCodeScanner
import androidx.compose.material.icons.filled.Add
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import uniffi.openslate_mobile.AiException
import uniffi.openslate_mobile.AiEvent
import uniffi.openslate_mobile.AiListener
import uniffi.openslate_mobile.ChatMsg
import uniffi.openslate_mobile.BoardModel
import uniffi.openslate_mobile.BoardView
import uniffi.openslate_mobile.aiChatBoard
import uniffi.openslate_mobile.aiListModels
import uniffi.openslate_mobile.aiTranscribe
import kotlin.concurrent.thread

private val main = Handler(Looper.getMainLooper())

/** Un messaggio della chat sullo schermo. */
class UiMsg(val role: String, text: String, val withPage: Boolean = false) {
    var text by mutableStateOf(text)
    var thinking by mutableStateOf("")
    var by by mutableStateOf("")
    var error by mutableStateOf(false)
    /** Passaggi con gli strumenti («Scrivo sul foglio: segni sul foglio»). */
    val steps = mutableStateListOf<String>()
    /** Il tutor ha scritto sulla lavagna. */
    var boardChanged by mutableStateOf(false)
}

/**
 * La conversazione (vive finché l'app è aperta). Le richieste girano in un
 * thread a parte; gli eventi tornano sul thread principale.
 */
class ChatState {
    val msgs = mutableStateListOf<UiMsg>()
    var running by mutableStateOf(false)
        private set
    /** «Provo Gemini…», «DeepSeek saltato: …». */
    var status by mutableStateOf("")
    @Volatile private var cancel = false

    fun stop() {
        cancel = true
    }

    fun clear() {
        if (!running) msgs.clear()
    }

    /**
     * Manda `text`, col foglio (`page`, PNG) se c'è. Il tutor può leggere e
     * annotare la lavagna (`board`, vista come `view`); `onBoard` la ridisegna.
     */
    fun send(ctx: Context, store: AiStore, text: String, page: ByteArray?, board: BoardModel, view: BoardView, onBoard: () -> Unit) {
        if (running || text.isBlank()) return
        val history = msgs.filter { !it.error }.map { ChatMsg(it.role, it.text, null) }
        msgs += UiMsg("user", text.trim(), page != null)
        val answer = UiMsg("assistant", "")
        msgs += answer
        running = true
        cancel = false
        status = ""
        val providers = store.providers()
        val send = history + ChatMsg("user", text.trim(), page)
        thread(name = "ai-chat") {
            val listener = object : AiListener {
                override fun onEvent(event: AiEvent): Boolean {
                    main.post {
                        when (event) {
                            is AiEvent.Trying -> status = ctx.getString(R.string.ai_trying, event.provider, event.model)
                            is AiEvent.Skipped -> status = ctx.getString(R.string.ai_skipped, event.provider, event.why)
                            is AiEvent.Delta -> answer.text += event.text
                            is AiEvent.Thinking -> answer.thinking += event.text
                            is AiEvent.Tool -> status = event.label + "…"
                            is AiEvent.ToolDone -> answer.steps += "${event.label}: ${event.result}"
                            AiEvent.BoardChanged -> {
                                answer.boardChanged = true
                                onBoard()
                            }
                        }
                    }
                    return !cancel
                }
            }
            val result = runCatching { aiChatBoard(providers, send, board, view, listener) }
            main.post {
                result.onSuccess {
                    answer.text = it.text
                    answer.by = "${it.provider} · ${it.model}"
                }.onFailure { e ->
                    if (e is AiException.Cancelled) {
                        if (answer.text.isBlank()) msgs.remove(answer)
                    } else {
                        answer.text = (e as? AiException.Failed)?.reason ?: e.message.orEmpty()
                        answer.error = true
                    }
                }
                status = ""
                running = false
            }
        }
    }
}

/** Trascrive una pagina (PNG) in un thread a parte. */
fun transcribe(ctx: Context, store: AiStore, png: ByteArray, onStatus: (String) -> Unit, done: (Result<String>) -> Unit) {
    val providers = store.providers()
    thread(name = "ai-ocr") {
        val listener = object : AiListener {
            override fun onEvent(event: AiEvent): Boolean {
                when (event) {
                    is AiEvent.Trying -> main.post { onStatus(ctx.getString(R.string.ai_trying, event.provider, event.model)) }
                    is AiEvent.Skipped -> main.post { onStatus(ctx.getString(R.string.ai_skipped, event.provider, event.why)) }
                    else -> {}
                }
                return true
            }
        }
        val r = runCatching { aiTranscribe(providers, png, listener).text }
            .recoverCatching { e -> throw Exception((e as? AiException.Failed)?.reason ?: e.message) }
        main.post { done(r) }
    }
}

@Composable
fun ChatScreen(
    chat: ChatState,
    store: AiStore,
    page: () -> ByteArray?,
    hasPage: Boolean,
    send: (String, ByteArray?) -> Unit,
    onBack: () -> Unit,
    onSettings: () -> Unit,
) {
    BackHandler(onBack = onBack)
    val ctx = LocalContext.current
    var input by remember { mutableStateOf("") }
    var attach by remember { mutableStateOf(hasPage) }
    val list = rememberLazyListState()
    val noServices = remember { store.services().none { it.enabled } }
    LaunchedEffect(chat.msgs.size, chat.msgs.lastOrNull()?.text?.length) {
        if (chat.msgs.isNotEmpty()) list.animateScrollToItem(chat.msgs.size - 1)
    }
    Column(Modifier.fillMaxSize().imePadding()) {
        ActionRow {
            TextButton(onClick = onBack) { Text("← " + stringResource(R.string.back)) }
            Text(stringResource(R.string.chat), style = MaterialTheme.typography.titleLarge)
            Spacer(Modifier.weight(1f))
            IconButton(onClick = { chat.clear() }, enabled = !chat.running && chat.msgs.isNotEmpty()) {
                Icon(Icons.Filled.AddComment, stringResource(R.string.new_chat))
            }
            IconButton(onClick = onSettings) { Icon(Icons.Filled.Tune, stringResource(R.string.ai_settings)) }
        }
        if (noServices) {
            Text(
                stringResource(R.string.ai_none),
                Modifier.padding(16.dp).clickable(onClick = onSettings),
                color = MaterialTheme.colorScheme.primary,
            )
        }
        LazyColumn(Modifier.weight(1f).fillMaxWidth().padding(horizontal = 12.dp), state = list) {
            if (chat.msgs.isEmpty() && !noServices) {
                item { Text(stringResource(R.string.chat_hint), Modifier.padding(vertical = 16.dp), color = MaterialTheme.colorScheme.onSurfaceVariant) }
            }
            itemsIndexed(chat.msgs) { _, m -> Bubble(m, onBack) }
        }
        if (chat.status.isNotEmpty()) {
            Text(chat.status, Modifier.padding(horizontal = 12.dp), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        ChatInput(
            input = input,
            onInput = { input = it },
            attach = attach && hasPage,
            onAttach = { attach = it },
            hasPage = hasPage,
            running = chat.running,
            onSend = {
                send(input.trim(), if (attach && hasPage) page() else null)
                input = ""
            },
            onStop = { chat.stop() },
        )
    }
}

/**
 * La casella del messaggio: si allarga col testo, il foglio da allegare, il
 * microfono per dettare e il tasto per mandare (o fermare l'AI).
 */
@Composable
private fun ChatInput(
    input: String,
    onInput: (String) -> Unit,
    attach: Boolean,
    onAttach: (Boolean) -> Unit,
    hasPage: Boolean,
    running: Boolean,
    onSend: () -> Unit,
    onStop: () -> Unit,
) {
    val ctx = LocalContext.current
    // quello che c'era prima di dettare: il parlato si aggiunge in fondo
    var base by remember { mutableStateOf("") }
    val current by rememberUpdatedState(input)
    val setInput by rememberUpdatedState(onInput)
    val dictation = remember {
        Dictation(ctx) { text, _ ->
            setInput(if (base.isBlank()) text.replaceFirstChar { it.uppercase() } else base.trimEnd() + " " + text)
        }
    }
    DisposableEffect(Unit) { onDispose { dictation.release() } }
    val startDictation = {
        base = current
        dictation.start()
    }
    val askMic = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { ok ->
        if (ok) startDictation() else dictation.error = ctx.getString(R.string.dictation_no_permission)
    }
    val inApp = remember { dictation.available() }
    val systemDictation = rememberLauncherForActivityResult(ActivityResultContracts.StartActivityForResult()) { r ->
        dictation.result(r.data)?.let { t -> setInput(if (current.isBlank()) t.replaceFirstChar { it.uppercase() } else current.trimEnd() + " " + t) }
    }
    Surface(tonalElevation = 2.dp, shadowElevation = 4.dp) {
        Column(Modifier.fillMaxWidth().padding(start = 12.dp, end = 12.dp, top = 6.dp, bottom = 8.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                FilterChip(
                    selected = attach,
                    onClick = { onAttach(!attach) },
                    enabled = hasPage,
                    label = { Text(stringResource(R.string.attach_page)) },
                    leadingIcon = { Icon(if (attach) Icons.Filled.Check else Icons.Filled.AttachFile, null, Modifier.size(18.dp)) },
                )
            }
            dictation.error?.let {
                Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
            }
            Row(verticalAlignment = Alignment.Bottom, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                TextField(
                    value = input,
                    onValueChange = onInput,
                    modifier = Modifier.weight(1f).heightIn(min = 52.dp),
                    placeholder = { Text(stringResource(if (dictation.listening) R.string.dictation_listening else R.string.chat_placeholder)) },
                    minLines = 1,
                    maxLines = 6,
                    shape = RoundedCornerShape(26.dp),
                    keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Sentences),
                    colors = TextFieldDefaults.colors(
                        focusedIndicatorColor = Color.Transparent,
                        unfocusedIndicatorColor = Color.Transparent,
                        disabledIndicatorColor = Color.Transparent,
                    ),
                    trailingIcon = {
                            IconButton(onClick = {
                                if (!inApp) {
                                    try {
                                        systemDictation.launch(dictation.systemIntent())
                                    } catch (_: android.content.ActivityNotFoundException) {
                                        dictation.error = ctx.getString(R.string.dictation_missing)
                                    }
                                } else if (dictation.listening) dictation.stop()
                                else if (ContextCompat.checkSelfPermission(ctx, Manifest.permission.RECORD_AUDIO) == PackageManager.PERMISSION_GRANTED) startDictation()
                                else askMic.launch(Manifest.permission.RECORD_AUDIO)
                            }) {
                                Icon(
                                    if (dictation.listening) Icons.Filled.Mic else Icons.Outlined.Mic,
                                    stringResource(R.string.dictate),
                                    Modifier.scale(1f + 0.25f * dictation.level),
                                    tint = if (dictation.listening) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                    },
                )
                if (running) {
                    FilledIconButton(onClick = onStop, Modifier.size(52.dp)) {
                        Icon(Icons.Filled.Stop, stringResource(R.string.stop))
                    }
                } else {
                    FilledIconButton(
                        onClick = { if (dictation.listening) dictation.stop(); onSend() },
                        modifier = Modifier.size(52.dp),
                        enabled = input.isNotBlank(),
                    ) {
                        Icon(Icons.AutoMirrored.Filled.Send, stringResource(R.string.send))
                    }
                }
            }
        }
    }
}

@Composable
private fun Bubble(m: UiMsg, showBoard: () -> Unit) {
    val mine = m.role == "user"
    var showThinking by remember { mutableStateOf(false) }
    Column(Modifier.fillMaxWidth().padding(vertical = 6.dp), horizontalAlignment = if (mine) Alignment.End else Alignment.Start) {
        Column(
            Modifier
                .background(
                    when {
                        m.error -> MaterialTheme.colorScheme.errorContainer
                        mine -> MaterialTheme.colorScheme.primaryContainer
                        else -> MaterialTheme.colorScheme.surfaceVariant
                    },
                    RoundedCornerShape(14.dp),
                )
                .padding(12.dp)
        ) {
            if (m.thinking.isNotEmpty()) {
                Text(
                    (if (showThinking) "▾ " else "▸ ") + stringResource(R.string.thinking),
                    Modifier.clickable { showThinking = !showThinking },
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                if (showThinking) Text(m.thinking, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            for (step in m.steps) {
                Text("✓ $step", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            SelectionContainer { Text(m.text.ifEmpty { "…" }) }
            if (m.boardChanged) {
                TextButton(onClick = showBoard) { Text(stringResource(R.string.see_board)) }
            }
            if (m.withPage) Text("📄 " + stringResource(R.string.page_attached), style = MaterialTheme.typography.labelSmall)
        }
        if (m.by.isNotEmpty()) Text(m.by, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

/** La nota trascritta di una pagina: si corregge, si salva, si condivide. */
@Composable
fun NoteScreen(title: String, initial: String, onSave: (String) -> Unit, onBack: () -> Unit) {
    BackHandler(onBack = onBack)
    val ctx = LocalContext.current
    var text by remember(initial) { mutableStateOf(initial) }
    Column(Modifier.fillMaxSize().imePadding()) {
        ActionRow {
            TextButton(onClick = onBack) { Text("← " + stringResource(R.string.back)) }
            Button(onClick = { onSave(text) }) { Text(stringResource(R.string.save)) }
            OutlinedButton(onClick = {
                val cm = ctx.getSystemService(ClipboardManager::class.java)
                cm.setPrimaryClip(ClipData.newPlainText(title, text))
            }) { Text(stringResource(R.string.copy)) }
            OutlinedButton(onClick = {
                val send = Intent(Intent.ACTION_SEND).apply {
                    type = "text/markdown"
                    putExtra(Intent.EXTRA_SUBJECT, title)
                    putExtra(Intent.EXTRA_TEXT, text)
                }
                ctx.startActivity(Intent.createChooser(send, title))
            }) { Text(stringResource(R.string.share)) }
        }
        Text(title, Modifier.padding(horizontal = 12.dp), style = MaterialTheme.typography.titleMedium)
        OutlinedTextField(
            text, { text = it },
            Modifier.fillMaxSize().padding(12.dp),
            textStyle = MaterialTheme.typography.bodyMedium.copy(fontFamily = FontFamily.Monospace),
        )
    }
}

/** Impostazioni AI: i servizi in ordine (il primo che risponde vince). */
@Composable
fun AiSettingsScreen(store: AiStore, onBack: () -> Unit) {
    BackHandler(onBack = onBack)
    val ctx = LocalContext.current
    var list by remember { mutableStateOf(store.services()) }
    var editing by remember { mutableStateOf<AiService?>(null) }
    var adding by remember { mutableStateOf(false) }
    var testResult by remember { mutableStateOf("") }
    fun update(l: List<AiService>) {
        list = l
        store.save(l)
    }
    // il QR della desktop: si legge, si chiede conferma, si salva
    var scanned by remember { mutableStateOf<List<uniffi.openslate_mobile.AiProvider>?>(null) }
    val scan = androidx.activity.compose.rememberLauncherForActivityResult(com.journeyapps.barcodescanner.ScanContract()) { r ->
        val text = r.contents ?: return@rememberLauncherForActivityResult
        runCatching { uniffi.openslate_mobile.aiImportQr(text) }
            .onSuccess { scanned = it }
            .onFailure { e -> testResult = (e as? AiException.Failed)?.reason ?: e.message.orEmpty() }
    }
    Column(Modifier.fillMaxSize()) {
        ActionRow {
            TextButton(onClick = onBack) { Text("← " + stringResource(R.string.back)) }
            Text(stringResource(R.string.ai_settings), style = MaterialTheme.typography.titleLarge)
        }
        // uno sotto l'altro, a tutta larghezza: affiancati il secondo non si legge
        Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedButton(onClick = {
                scan.launch(com.journeyapps.barcodescanner.ScanOptions().apply {
                    setDesiredBarcodeFormats(com.journeyapps.barcodescanner.ScanOptions.QR_CODE)
                    setPrompt(ctx.getString(R.string.scan_prompt))
                    setBeepEnabled(false)
                    setOrientationLocked(true)
                })
            }, Modifier.fillMaxWidth()) {
                Icon(androidx.compose.material.icons.Icons.Filled.QrCodeScanner, null)
                Spacer(Modifier.width(8.dp))
                Text(stringResource(R.string.import_qr))
            }
            Button(onClick = { adding = true }, Modifier.fillMaxWidth()) {
                Icon(androidx.compose.material.icons.Icons.Filled.Add, null)
                Spacer(Modifier.width(8.dp))
                Text(stringResource(R.string.add_service))
            }
        }
        Text(stringResource(R.string.ai_settings_hint), Modifier.padding(horizontal = 16.dp), style = MaterialTheme.typography.bodySmall)
        if (testResult.isNotEmpty()) Text(testResult, Modifier.padding(16.dp), color = MaterialTheme.colorScheme.primary)
        LazyColumn(Modifier.fillMaxSize()) {
            itemsIndexed(list, key = { _, s -> s.id }) { i, s ->
                Row(Modifier.fillMaxWidth().clickable { editing = s }.padding(horizontal = 16.dp, vertical = 10.dp), verticalAlignment = Alignment.CenterVertically) {
                    Column(Modifier.weight(1f)) {
                        Text("${i + 1}. ${s.name}", style = MaterialTheme.typography.bodyLarge)
                        val key = if (s.url.startsWith("https://")) {
                            if (store.hasKey(s.id)) stringResource(R.string.key_set) else stringResource(R.string.key_missing)
                        } else ""
                        Text(
                            listOf(s.model.ifEmpty { stringResource(R.string.model_auto) }, key).filter { it.isNotEmpty() }.joinToString(" · "),
                            style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    TextButton(onClick = { if (i > 0) update(list.toMutableList().apply { add(i - 1, removeAt(i)) }) }, enabled = i > 0) { Text("↑") }
                    Switch(checked = s.enabled, onCheckedChange = { on -> update(list.map { if (it.id == s.id) it.copy(enabled = on) else it }) })
                }
                HorizontalDivider()
            }
        }
    }
    scanned?.let { got ->
        AlertDialog(
            onDismissRequest = { scanned = null },
            title = { Text(stringResource(R.string.import_qr)) },
            text = {
                Column {
                    Text(stringResource(R.string.import_confirm))
                    for (p in got) {
                        val key = if (p.key.isNotBlank()) " · " + stringResource(R.string.key_set) else ""
                        Text("• ${p.name}$key", Modifier.padding(top = 4.dp), style = MaterialTheme.typography.bodyMedium)
                    }
                }
            },
            confirmButton = {
                TextButton(onClick = {
                    val n = store.import(got)
                    list = store.services()
                    scanned = null
                    testResult = ctx.getString(R.string.import_done, n)
                }) { Text(stringResource(R.string.import_ok)) }
            },
            dismissButton = { TextButton(onClick = { scanned = null }) { Text(stringResource(R.string.cancel)) } },
        )
    }
    if (adding) {
        AlertDialog(
            onDismissRequest = { adding = false },
            title = { Text(stringResource(R.string.add_service)) },
            text = {
                Column(Modifier.verticalScroll(rememberScrollState())) {
                    for (p in AiPresets.all) {
                        Text(p.name, Modifier.fillMaxWidth().clickable {
                            adding = false
                            editing = p.copy(id = java.util.UUID.randomUUID().toString())
                        }.padding(vertical = 12.dp))
                    }
                }
            },
            confirmButton = { TextButton(onClick = { adding = false }) { Text(stringResource(R.string.cancel)) } },
        )
    }
    editing?.let { s ->
        var name by remember(s.id) { mutableStateOf(s.name) }
        var url by remember(s.id) { mutableStateOf(s.url) }
        var model by remember(s.id) { mutableStateOf(s.model) }
        var key by remember(s.id) { mutableStateOf("") }
        var vision by remember(s.id) { mutableStateOf(s.vision) }
        val isNew = list.none { it.id == s.id }
        fun current() = s.copy(name = name.trim(), url = url.trim(), model = model.trim(), vision = vision)
        AlertDialog(
            onDismissRequest = { editing = null },
            title = { Text(s.name) },
            text = {
                Column(Modifier.verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedTextField(name, { name = it }, label = { Text(stringResource(R.string.name)) }, singleLine = true)
                    OutlinedTextField(url, { url = it }, label = { Text(stringResource(R.string.url)) }, singleLine = true,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri))
                    OutlinedTextField(model, { model = it }, label = { Text(stringResource(R.string.model_hint)) }, singleLine = true)
                    OutlinedTextField(
                        key, { key = it },
                        label = { Text(stringResource(if (store.hasKey(s.id)) R.string.key_replace else R.string.key_hint)) },
                        singleLine = true,
                        visualTransformation = PasswordVisualTransformation(),
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
                    )
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Checkbox(checked = vision, onCheckedChange = { vision = it })
                        Text(stringResource(R.string.reads_images))
                    }
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        OutlinedButton(onClick = {
                            val p = current()
                            if (key.isNotBlank()) store.setKey(p.id, key)
                            testResult = ctx.getString(R.string.testing, p.name)
                            val prov = uniffi.openslate_mobile.AiProvider(
                                p.name, p.url, p.model,
                                if (key.isNotBlank()) key else store.providers().firstOrNull { it.name == s.name }?.key ?: "",
                                p.vision, true, p.noTemperature, p.maxTokens?.toUInt(), p.timeoutS.toUInt(),
                            )
                            thread {
                                val r = runCatching { aiListModels(prov) }
                                main.post {
                                    testResult = r.fold(
                                        { ctx.getString(R.string.test_ok, p.name, it.size) },
                                        { e -> ctx.getString(R.string.test_fail, p.name, (e as? AiException.Failed)?.reason ?: e.message.orEmpty()) },
                                    )
                                }
                            }
                        }) { Text(stringResource(R.string.test)) }
                        if (!isNew) OutlinedButton(onClick = {
                            update(list.filter { it.id != s.id })
                            editing = null
                        }) { Text(stringResource(R.string.delete)) }
                    }
                }
            },
            confirmButton = {
                TextButton(onClick = {
                    val p = current()
                    if (key.isNotBlank()) store.setKey(p.id, key)
                    update(if (isNew) list + p else list.map { if (it.id == p.id) p else it })
                    editing = null
                }, enabled = name.isNotBlank() && (url.startsWith("http://") || url.startsWith("https://"))) { Text(stringResource(R.string.ok)) }
            },
            dismissButton = { TextButton(onClick = { editing = null }) { Text(stringResource(R.string.cancel)) } },
        )
    }
}
