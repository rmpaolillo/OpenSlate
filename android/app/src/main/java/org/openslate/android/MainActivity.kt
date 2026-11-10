package org.openslate.android

import android.Manifest
import android.os.Build
import android.os.Bundle
import android.util.Log
import android.view.WindowManager
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.AlertDialog
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Chat
import androidx.compose.material.icons.automirrored.filled.Redo
import androidx.compose.material.icons.automirrored.filled.TextSnippet
import androidx.compose.material.icons.automirrored.filled.Undo
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Slider
import androidx.compose.material3.Switch
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.verticalScroll
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.material3.Checkbox
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.RadioButton
import androidx.compose.material3.VerticalDivider
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.draw.clip
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import uniffi.openslate_mobile.ExportKind
import uniffi.openslate_mobile.setNaturalInk
import uniffi.openslate_mobile.exportMime
import uniffi.openslate_mobile.exportNeedsNote
import uniffi.openslate_mobile.PadInfo
import uniffi.openslate_mobile.Page
import uniffi.openslate_mobile.PageStroke
import uniffi.openslate_mobile.PenPoint
import uniffi.openslate_mobile.BoardModel
import uniffi.openslate_mobile.AiEvent
import uniffi.openslate_mobile.AiListener
import uniffi.openslate_mobile.BoardPreview
import uniffi.openslate_mobile.FairJob
import uniffi.openslate_mobile.SheetSpec
import uniffi.openslate_mobile.Zone
import uniffi.openslate_mobile.boardArrange
import uniffi.openslate_mobile.boardSuggestEvery
import uniffi.openslate_mobile.neatSetDir
import uniffi.openslate_mobile.paperHasRows
import uniffi.openslate_mobile.paperIsGrid
import androidx.compose.material3.CircularProgressIndicator
import uniffi.openslate_mobile.BoardView
import uniffi.openslate_mobile.DrawOp
import uniffi.openslate_mobile.Tool
import uniffi.openslate_mobile.libraryLoad
import uniffi.openslate_mobile.librarySave
import uniffi.openslate_mobile.nowCreated
import uniffi.openslate_mobile.padTimeLabel
import uniffi.openslate_mobile.pageFromPad
import java.io.File
import java.io.FileOutputStream

/**
 * La lavagna. Il modello (strumenti, gomma, annulla/ripeti) è nel cuore Rust
 * ([BoardModel], la logica di `scene.rs` della desktop); qui c'è la copia da
 * disegnare, aggiornata coi cambiamenti, e la pagina da cui viene.
 */
class Board {
    val model = BoardModel()
    val strokes = ArrayList<PageStroke>()
    /** Pagina dell'archivio aperta sulla lavagna ("" = nuova). */
    var id = ""
    var title = ""
    var created = ""
    private var savedRev = model.rev()
    /** Cambia a ogni modifica: la lavagna si ridisegna. */
    var version by mutableIntStateOf(0)
        private set

    val isEmpty get() = model.isEmpty()
    /** Ci sono cambiamenti non salvati. */
    val dirty get() = model.rev() != savedRev && !model.isEmpty()

    fun markSaved() {
        savedRev = model.rev()
    }

    fun pen(points: List<PenPoint>) {
        for (op in model.pen(points)) {
            when (op) {
                is DrawOp.Down -> strokes += PageStroke(op.color, op.width, op.highlight, arrayListOf(op.point))
                is DrawOp.Move -> (strokes.lastOrNull()?.points as? ArrayList<PenPoint>)?.add(op.point)
                is DrawOp.Up -> (strokes.lastOrNull()?.points as? ArrayList<PenPoint>)?.add(op.point)
                DrawOp.Reset -> {
                    reload()
                    return
                }
            }
        }
        version++
    }

    /** Rilegge tutti i tratti dal modello. */
    fun reload() {
        strokes.clear()
        model.strokes().forEach { strokes += it.copy(points = ArrayList(it.points)) }
        version++
    }

    fun undo() { if (model.undo()) reload() }
    fun redo() { if (model.redo()) reload() }
    fun clear() { model.clear(); reload() }

    /** Foglio nuovo: niente tratti, niente cronologia, nessuna pagina. */
    fun newPage() {
        model.reset()
        id = ""
        title = ""
        created = ""
        markSaved()
        reload()
    }

    /** Tratti di un'altra pagina (di un documento), senza cronologia. */
    fun openStrokes(strokes: List<PageStroke>) {
        model.open(strokes)
        markSaved()
        reload()
    }

    fun load(p: Page) {
        model.open(p.strokes)
        id = p.id
        title = p.title
        created = p.created
        markSaved()
        reload()
    }

    val eraser = object : FingerEraser {
        override fun erase(ax: Float, ay: Float, bx: Float, by: Float, tol: Float, pageH: Float) {
            if (model.fingerErase(ax, ay, bx, by, tol, pageH) > 0u) reload()
        }

        override fun end() {
            model.fingerEraseEnd()
            version++
        }
    }

    fun toPage(width: Float, height: Float, source: String = "lavagna") = Page(
        id = id,
        title = title,
        created = created.ifEmpty { nowCreated() },
        source = source,
        raw = null,
        width = width,
        height = height,
        strokes = model.strokes(),
    )
}

/** I colori della desktop (`PALETTE` in ui.rs) e quelli dell'evidenziatore. */
val PEN_COLORS = listOf("#141414", "#195ac8", "#d22828", "#1e963c", "#f08c00", "#8c3cbe")
val HL_COLORS = listOf("#ffeb5a", "#a0f08c", "#ffa0d2", "#9bd2ff", "#ffc864")

private sealed class Screen {
    object Board : Screen()
    object Library : Screen()
    data class Viewer(val page: Page) : Screen()
    object Chat : Screen()
    object Settings : Screen()
    data class AiSettings(val back: Screen) : Screen()
    /** La nota trascritta della pagina `id`; `back` = dove si torna. */
    data class Note(val id: String, val title: String, val text: String, val back: Screen) : Screen()
}

class MainActivity : ComponentActivity(), LinkListener {
    private lateinit var link: SlateLink
    private val board = Board()
    private var status by mutableStateOf<LinkStatus>(LinkStatus.Idle)
    private var battery by mutableStateOf<Int?>(null)
    /** Area attiva del pad in punti (dal pad quando parte il live). */
    private var padSize by mutableStateOf(Size(21600f, 14800f))
    private var screen by mutableStateOf<Screen>(Screen.Board)
    /** Cambia quando l'archivio cambia: la libreria si rilegge. */
    private var libVersion by mutableIntStateOf(0)
    /** Rotazione del foglio in quarti di giro (ricordata). */
    private var quarter by mutableIntStateOf(0)
    /** Bluetooth spento all'ultimo «Collega». */
    private var btOff by mutableStateOf(false)

    // Strumenti (ricordati fra un avvio e l'altro, come le impostazioni della desktop).
    private var tool by mutableStateOf(Tool.PEN)
    private var eraserOn by mutableStateOf(false)
    private var penColor by mutableStateOf("#141414")
    private var penWidth by mutableFloatStateOf(2f)
    private var hlColor by mutableStateOf("#ffeb5a")
    private var hlWidth by mutableFloatStateOf(16f)
    private var paperCode by mutableStateOf("bianco")
    private var paperMargin by mutableStateOf(true)
    private var zigzag by mutableStateOf(false)
    private var naturalInk by mutableStateOf(true)
    /** La bella copia va subito sulle righe del foglio. */
    private var fairOnRules by mutableStateOf(false)
    private var choosingPaper by mutableStateOf(false)
    /** Bella copia o «Disponi sulle righe» in corso (null = no). */
    private var zoneJob by mutableStateOf<ZoneJob?>(null)
    /** Il numero del lavoro in corso: le risposte di uno annullato non contano. */
    private var neatGen = 0

    /** Dimensioni del foglio (area attiva del pad), ricordate per i fogli nuovi. */
    private fun setPadSize(w: Float, h: Float) {
        padSize = Size(w, h)
        board.model.setPageSize(w, h)
        prefs.edit().putFloat("pad_w", w).putFloat("pad_h", h).apply()
    }

    private fun loadTools() {
        val w = prefs.getFloat("pad_w", 0f)
        val h = prefs.getFloat("pad_h", 0f)
        if (w > 0f && h > 0f) setPadSize(w, h)
        tool = if (prefs.getString("tool", "pen") == "highlighter") Tool.HIGHLIGHTER else Tool.PEN
        penColor = prefs.getString("pen_color", penColor)!!
        penWidth = prefs.getFloat("pen_width", penWidth)
        hlColor = prefs.getString("hl_color", hlColor)!!
        hlWidth = prefs.getFloat("hl_width", hlWidth)
        paperCode = prefs.getString("paper", paperCode)!!
        paperMargin = prefs.getBoolean("paper_margin", paperMargin)
        zigzag = prefs.getBoolean("zigzag", zigzag)
        naturalInk = prefs.getBoolean("natural_ink", naturalInk)
        fairOnRules = prefs.getBoolean("fair_on_rules", fairOnRules)
        applyTools()
    }

    /** Passa gli strumenti al modello e li ricorda. */
    private fun applyTools() {
        val m = board.model
        m.setTool(tool)
        m.setEraser(eraserOn)
        m.setPen(penColor, penWidth)
        m.setHighlighter(hlColor, hlWidth)
        m.setZigzag(zigzag)
        setNaturalInk(naturalInk)
        Ink.naturalInk = naturalInk
        prefs.edit()
            .putString("tool", if (tool == Tool.HIGHLIGHTER) "highlighter" else "pen")
            .putString("pen_color", penColor).putFloat("pen_width", penWidth)
            .putString("hl_color", hlColor).putFloat("hl_width", hlWidth)
            .putString("paper", paperCode).putBoolean("paper_margin", paperMargin)
            .putBoolean("zigzag", zigzag)
            .putBoolean("natural_ink", naturalInk)
            .putBoolean("fair_on_rules", fairOnRules)
            .apply()
    }

    private val prefs by lazy { getSharedPreferences("view", MODE_PRIVATE) }
    private val aiStore by lazy { AiStore(this) }
    private val chat = ChatState()
    /** Il PDF aperto per firmarlo (null = lavagna normale). */
    private var doc by mutableStateOf<OpenDoc?>(null)

    /** Lavoro AI in corso (trascrizione): il testo da mostrare. */
    private var busy by mutableStateOf<String?>(null)
    val libDir: String by lazy { File(filesDir, "library").path }
    /** La pagina per cui si sceglie uno degli «Altri formati» (null = finestra chiusa). */
    private var moreFor by mutableStateOf<Page?>(null)
    private var moreFromBoard = false
    /** Il file esportato che l'utente sta salvando in una cartella. */
    private var pendingSave: File? = null
    private val saveExport = registerForActivityResult(ActivityResultContracts.CreateDocument("*/*")) { uri ->
        val f = pendingSave
        pendingSave = null
        if (uri != null && f != null) {
            runCatching { contentResolver.openOutputStream(uri)!!.use { out -> f.inputStream().use { it.copyTo(out) } } }
                .onSuccess { toast(getString(R.string.export_saved)) }
                .onFailure { toast(getString(R.string.save_failed, it.message ?: "")) }
        }
    }
    private val rawDir by lazy { File(filesDir, "pad-raw") }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        link = SlateLink(this, this)
        neatSetDir(filesDir.path)
        quarter = prefs.getInt("quarter", 1) // lo Slate si usa in verticale
        loadTools()
        prefs.getString("doc_dir", null)?.let { path ->
            runCatching { OpenDoc.open(File(path)) }.onSuccess { d -> doc = d; d.show(0, board) }
        }
        setContent {
            MaterialTheme {
                Surface(Modifier.fillMaxSize()) {
                  Column(Modifier.fillMaxSize().safeDrawingPadding()) {
                    Box(Modifier.weight(1f).fillMaxWidth()) {
                        when (val s = screen) {
                            Screen.Settings -> SettingsScreen()
                            Screen.Board -> BoardScreen()
                            Screen.Library -> LibraryScreen(
                                dir = libDir,
                                version = libVersion,
                                onBack = { screen = Screen.Board },
                                onOpen = { id -> openViewer(id) },
                            )
                            Screen.Chat -> ChatScreen(
                                chat = chat,
                                store = aiStore,
                                page = { Export.pngBytes(board.toPage(padSize.width, padSize.height), quarter) },
                                hasPage = !board.isEmpty,
                                send = { text, png ->
                                    chat.send(
                                        this@MainActivity, aiStore, text, png, board.model,
                                        BoardView(quarter.toUInt(), padSize.width, padSize.height),
                                    ) { board.reload() }
                                },
                                onBack = { screen = Screen.Board },
                                onSettings = { screen = Screen.AiSettings(Screen.Chat) },
                            )
                            is Screen.AiSettings -> AiSettingsScreen(aiStore, onBack = { screen = s.back })
                            is Screen.Note -> NoteScreen(s.title, s.text, onSave = { text ->
                                runCatching { File(libDir, "${s.id}.md").writeText(text) }
                                    .onSuccess { toast(getString(R.string.note_saved)); screen = s.copy(text = text) }
                                    .onFailure { toast(getString(R.string.save_failed, it.message ?: "")) }
                            }, onBack = { screen = s.back })
                            is Screen.Viewer -> ViewerScreen(
                                page = s.page,
                                quarter = quarter,
                                dir = libDir,
                                onBack = { screen = Screen.Library },
                                onChanged = { p ->
                                    libVersion++
                                    screen = if (p == null) Screen.Library else Screen.Viewer(p)
                                },
                                onOpenOnBoard = { p ->
                                    saveBoardIfNeeded()
                                    if (doc != null) closeDoc()
                                    board.load(p)
                                    if (p.width > 0f && p.height > 0f) setPadSize(p.width, p.height)
                                    screen = Screen.Board
                                },
                            )
                        }
                        moreFor?.let { p -> MoreFormatsDialog(onDismiss = { moreFor = null }) { kind, save -> moreFor = null; exportMore(p, kind, save) } }
                        busy?.let { BusyOverlay(it) }
                    }
                    // le quattro parti dell'app, sempre a portata (non nelle schermate di dettaglio)
                    val top = screen
                    if (top == Screen.Board || top == Screen.Library || top == Screen.Chat || top == Screen.Settings) {
                        NavigationBar {
                            val go = { to: Screen ->
                                if (to != Screen.Board) saveBoardIfNeeded()
                                screen = to
                            }
                            NavigationBarItem(top == Screen.Board, { go(Screen.Board) }, { Icon(Icons.Filled.Gesture, null) },
                                label = { Text(stringResource(R.string.board)) })
                            NavigationBarItem(top == Screen.Library, { go(Screen.Library) }, { Icon(Icons.Filled.PhotoLibrary, null) },
                                label = { Text(stringResource(R.string.library)) })
                            NavigationBarItem(top == Screen.Chat, { go(Screen.Chat) }, { Icon(Icons.AutoMirrored.Filled.Chat, null) },
                                label = { Text(stringResource(R.string.chat)) })
                            NavigationBarItem(top == Screen.Settings, { go(Screen.Settings) }, { Icon(Icons.Filled.Settings, null) },
                                label = { Text(stringResource(R.string.settings)) })
                        }
                    }
                  }
                }
            }
        }
    }

    @Composable
    private fun BusyOverlay(text: String) {
        Box(
            Modifier.fillMaxSize().background(androidx.compose.ui.graphics.Color(0x88000000)).clickable(enabled = true) {},
            contentAlignment = Alignment.Center,
        ) {
            Surface(shape = MaterialTheme.shapes.large, tonalElevation = 6.dp) {
                Column(Modifier.padding(24.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                    androidx.compose.material3.CircularProgressIndicator()
                    Text(text, Modifier.padding(top = 16.dp))
                }
            }
        }
    }

    /**
     * Trascrive la pagina (come «Scrittura → testo» della desktop) e apre la
     * nota, salvata accanto alla pagina come `<id>.md`.
     */
    private fun transcribePage(page: Page, back: Screen) {
        val title = page.title.ifBlank { page.created.take(16) }
        transcribeText(page, back) { text -> screen = Screen.Note(page.id, title, text, back) }
    }

    /** Trascrive la pagina e salva la nota accanto (`<id>.md`); poi `done`. */
    private fun transcribeText(page: Page, back: Screen, done: (String) -> Unit) {
        if (aiStore.services().none { it.enabled }) {
            toast(getString(R.string.ai_none))
            screen = Screen.AiSettings(back)
            return
        }
        busy = getString(R.string.transcribing)
        val png = Export.pngBytes(page, quarter)
        transcribe(this, aiStore, png, onStatus = { busy = getString(R.string.transcribing) + "\n" + it }) { r ->
            busy = null
            r.onSuccess { text ->
                runCatching { File(libDir, "${page.id}.md").writeText(text) }
                done(text)
            }.onFailure { toast(getString(R.string.ai_failed, it.message ?: "")) }
        }
    }

    /** Apre la scelta degli «Altri formati» per la lavagna o una pagina dell'archivio. */
    fun chooseMore(page: Page, fromBoard: Boolean) {
        moreFromBoard = fromBoard
        moreFor = page
    }

    /**
     * Esporta in uno degli «Altri formati» e condivide (o salva in una
     * cartella). LaTeX e Typst vengono dalla trascrizione: se manca, prima si
     * trascrive la pagina (come «Trascrivi» della desktop).
     */
    private fun exportMore(start: Page, kind: ExportKind, save: Boolean) {
        var page = start
        if (moreFromBoard && (exportNeedsNote(kind) || kind == ExportKind.WORD) && (board.id.isEmpty() || board.dirty)) {
            // la nota sta accanto alla pagina: prima la si salva
            if (saveBoard()) page = board.toPage(padSize.width, padSize.height)
        }
        val note = page.id.takeIf { it.isNotEmpty() }?.let { noteOf(it) }
        if (exportNeedsNote(kind) && note == null) {
            if (page.id.isEmpty()) { toast(getString(R.string.export_failed, getString(R.string.empty_page))); return }
            val p = page
            transcribeText(p, screen) { text -> runExport(p, kind, text, save) }
            return
        }
        runExport(page, kind, note, save)
    }

    private fun runExport(page: Page, kind: ExportKind, note: String?, save: Boolean) {
        busy = getString(R.string.exporting)
        kotlin.concurrent.thread(name = "export") {
            val r = runCatching { Export.more(this, page, quarter, kind, note, paperCode) }
            runOnUiThread {
                busy = null
                r.onSuccess { f ->
                    if (save) { pendingSave = f; saveExport.launch(f.name) }
                    else Export.share(this, f, exportMime(kind))
                }.onFailure {
                    Log.e("OpenSlate", "esportazione", it)
                    toast(getString(R.string.export_failed, it.message ?: ""))
                }
            }
        }
    }

    /** La nota già fatta di una pagina, se c'è. */
    fun noteOf(id: String): String? = File(libDir, "$id.md").takeIf { it.exists() }?.readText()

    /** Come la desktop: nulla si perde. Uscendo dall'app la lavagna va nell'archivio. */
    override fun onStop() {
        saveBoardIfNeeded()
        super.onStop()
    }

    override fun onDestroy() {
        if (isFinishing) link.disconnect(report = false)
        super.onDestroy()
    }

    // ---------------------------------------------------------------- archivio

    private fun saveBoard(): Boolean = runCatching {
        board.id = librarySave(libDir, board.toPage(padSize.width, padSize.height))
        if (board.created.isEmpty()) board.created = libraryLoad(libDir, board.id).created
        board.markSaved()
        libVersion++
    }.onFailure {
        Log.e("OpenSlate", "salvataggio lavagna", it)
        toast(getString(R.string.save_failed, it.message ?: ""))
    }.isSuccess

    private fun saveBoardIfNeeded() {
        val d = doc
        if (d != null) d.keep(board) else if (board.dirty) saveBoard()
    }

    private fun view() = BoardView(quarter.toUInt(), padSize.width, padSize.height)

    private fun openPdf(uri: android.net.Uri) {
        saveBoardIfNeeded()
        busy = getString(R.string.doc_opening)
        // un Word si impagina, una foto grande si converte: non sul thread principale
        kotlin.concurrent.thread(name = "doc-open") {
            val r = runCatching { OpenDoc.import(this, uri) }
            runOnUiThread {
                busy = null
                r.onSuccess { d ->
                    if (doc != null) closeDoc()
                    doc = d
                    prefs.edit().putString("doc_dir", d.dir.path).apply()
                    d.show(0, board)
                }.onFailure { toast(getString(R.string.doc_failed, it.message ?: "")) }
            }
        }
    }

    private fun closeDoc() {
        doc?.keep(board)
        doc = null
        prefs.edit().remove("doc_dir").apply()
        board.newPage()
    }

    private fun goPage(delta: Int) {
        val d = doc ?: return
        d.keep(board)
        d.show(d.index + delta, board)
    }

    /** La copia firmata in un file temporaneo (per salvarla o condividerla). */
    private fun signedFile(): File? {
        val d = doc ?: return null
        d.keep(board)
        val dir = File(cacheDir, "share").apply { deleteRecursively(); mkdirs() }
        val f = File(dir, d.title + " " + getString(R.string.signed_suffix) + "." + d.signedExt)
        return runCatching { d.sign(f, view()); f }
            .onFailure { toast(getString(R.string.save_failed, it.message ?: "")) }
            .getOrNull()
    }

    private fun openViewer(id: String) {
        runCatching { libraryLoad(libDir, id) }
            .onSuccess { screen = Screen.Viewer(it) }
            .onFailure { toast(it.message ?: "") }
    }

    private fun toast(msg: String) = Toast.makeText(this, msg, Toast.LENGTH_SHORT).show()

    // ---------------------------------------------------------------- pad

    override fun onStatus(status: LinkStatus) {
        this.status = status
        if (status is LinkStatus.Live) {
            battery = status.info.batteryPct?.toInt()
            if (status.info.width > 0f && status.info.height > 0f) setPadSize(status.info.width, status.info.height)
        }
        if (status is LinkStatus.Idle || status is LinkStatus.Disconnected || status is LinkStatus.Failed) battery = null
        // Mentre il pad è collegato lo schermo resta acceso.
        val on = status is LinkStatus.Live || status is LinkStatus.Syncing || status is LinkStatus.Synced
        if (on) window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        else window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
    }

    override fun onPen(points: List<PenPoint>) = board.pen(points)

    /**
     * Una pagina scaricata: prima la copia grezza (come la desktop, è la copia
     * di sicurezza), poi la pagina nell'archivio. È "al sicuro", e quindi si
     * può togliere dal pad, solo se la copia grezza è scritta su disco.
     */
    override fun onPageDownloaded(raw: ByteArray, timeBcd: ByteArray, crcOk: Boolean, info: PadInfo): Boolean {
        rawDir.mkdirs()
        val label = padTimeLabel(timeBcd)
        var f = File(rawDir, "$label.bin")
        var n = 2
        while (f.exists()) f = File(rawDir, "$label-${n++}.bin")
        FileOutputStream(f).use {
            it.write(raw)
            it.fd.sync()
        }
        val w = if (info.width > 0f) info.width else padSize.width
        val h = if (info.height > 0f) info.height else padSize.height
        runCatching { pageFromPad(raw, timeBcd, w, h, f.name)?.let { librarySave(libDir, it) } }
            .onFailure { Log.e("OpenSlate", "pagina del pad non salvata nell'archivio", it) }
        libVersion++
        return true
    }

    private fun startConnect() {
        btOff = !link.bluetoothOn
        if (btOff) status = LinkStatus.Idle else link.connect()
    }

    // ---------------------------------------------------------------- lavagna

    /** Stato del pad in breve, per il pulsante «Pad». */
    private val padOn get() = status.let { it is LinkStatus.Live || it is LinkStatus.Syncing || it is LinkStatus.Synced }
    private val padBusy get() = status.let { it is LinkStatus.Searching || it is LinkStatus.Connecting || it is LinkStatus.PressButton }

    @Composable
    private fun BoardScreen() {
        val perms = if (Build.VERSION.SDK_INT >= 31) {
            arrayOf(Manifest.permission.BLUETOOTH_SCAN, Manifest.permission.BLUETOOTH_CONNECT)
        } else {
            arrayOf(Manifest.permission.ACCESS_FINE_LOCATION)
        }
        var denied by rememberSaveable { mutableStateOf(false) }
        val ask = rememberLauncherForActivityResult(ActivityResultContracts.RequestMultiplePermissions()) { res ->
            denied = !res.values.all { it }
            if (!denied) startConnect()
        }
        val s = status
        val canSync = s is LinkStatus.Live || s is LinkStatus.Synced
        var padMenu by remember { mutableStateOf(false) }
        var pageMenu by remember { mutableStateOf(false) }
        val pickDoc = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri -> uri?.let(::openPdf) }
        val saveSigned = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("*/*")) { uri ->
            if (uri != null) signedFile()?.let { f ->
                runCatching { contentResolver.openOutputStream(uri)!!.use { out -> f.inputStream().use { it.copyTo(out) } } }
                    .onSuccess { toast(getString(R.string.signed_saved)) }
                    .onFailure { toast(getString(R.string.save_failed, it.message ?: "")) }
            }
        }
        val d = doc
        board.version // i pulsanti seguono la lavagna
        val empty = board.isEmpty
        val statusLine = statusText(if (denied) null else s)

        Column(Modifier.fillMaxSize()) {
            // --- barra in alto: pad | titolo | annulla/ripeti | menu della pagina
            Row(Modifier.fillMaxWidth().padding(horizontal = 4.dp, vertical = 2.dp), verticalAlignment = Alignment.CenterVertically) {
                Box {
                    AssistChip(
                        onClick = { padMenu = true },
                        label = { Text(battery?.let { "$it%" } ?: stringResource(R.string.pad)) },
                        leadingIcon = {
                            Icon(
                                when {
                                    padOn -> Icons.Filled.BluetoothConnected
                                    padBusy -> Icons.Filled.BluetoothSearching
                                    else -> Icons.Filled.Bluetooth
                                },
                                null,
                                tint = if (padOn) MaterialTheme.colorScheme.primary else LocalContentColor.current,
                            )
                        },
                    )
                    DropdownMenu(expanded = padMenu, onDismissRequest = { padMenu = false }) {
                        Text(statusLine, Modifier.padding(horizontal = 16.dp, vertical = 8.dp).widthIn(max = 280.dp), style = MaterialTheme.typography.bodySmall)
                        HorizontalDivider()
                        if (padOn || padBusy) {
                            DropdownMenuItem(
                                text = { Text(stringResource(R.string.disconnect)) },
                                leadingIcon = { Icon(Icons.Filled.BluetoothDisabled, null) },
                                onClick = { padMenu = false; link.disconnect() },
                            )
                        } else {
                            DropdownMenuItem(
                                text = { Text(stringResource(R.string.connect)) },
                                leadingIcon = { Icon(Icons.Filled.Bluetooth, null) },
                                onClick = { padMenu = false; ask.launch(perms) },
                            )
                        }
                        DropdownMenuItem(
                            text = { Text(stringResource(R.string.download)) },
                            leadingIcon = { Icon(Icons.Filled.Download, null) },
                            enabled = canSync,
                            onClick = { padMenu = false; link.sync() },
                        )
                    }
                }
                Text(
                    d?.title ?: board.title.ifBlank { stringResource(R.string.board) },
                    Modifier.weight(1f).padding(horizontal = 8.dp),
                    style = MaterialTheme.typography.titleMedium,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                IconButton(onClick = { board.undo() }, enabled = board.model.canUndo()) {
                    Icon(Icons.AutoMirrored.Filled.Undo, stringResource(R.string.undo))
                }
                IconButton(onClick = { board.redo() }, enabled = board.model.canRedo()) {
                    Icon(Icons.AutoMirrored.Filled.Redo, stringResource(R.string.redo))
                }
                IconButton(onClick = { copyPage(board.toPage(padSize.width, padSize.height)) }, enabled = !empty && d == null) {
                    Icon(Icons.Filled.ContentCopy, stringResource(R.string.copy_board))
                }
                Box {
                    IconButton(onClick = { pageMenu = true }) { Icon(Icons.Filled.MoreVert, stringResource(R.string.more)) }
                    DropdownMenu(expanded = pageMenu, onDismissRequest = { pageMenu = false }) {
                        MenuHeader(stringResource(R.string.grp_page))
                        DropdownMenuItem(text = { Text(stringResource(R.string.new_page)) }, leadingIcon = { Icon(Icons.Filled.NoteAdd, null) },
                            enabled = d == null, onClick = { pageMenu = false; saveBoardIfNeeded(); board.newPage() })
                        DropdownMenuItem(text = { Text(stringResource(R.string.save)) }, leadingIcon = { Icon(Icons.Filled.Save, null) },
                            enabled = !empty && d == null, onClick = { pageMenu = false; if (saveBoard()) toast(getString(R.string.saved)) })
                        DropdownMenuItem(text = { Text(stringResource(R.string.clear_page)) }, leadingIcon = { Icon(Icons.Filled.DeleteSweep, null) },
                            enabled = !empty, onClick = { pageMenu = false; board.clear() })
                        DropdownMenuItem(text = { Text(stringResource(R.string.transcribe)) }, leadingIcon = { Icon(Icons.AutoMirrored.Filled.TextSnippet, null) },
                            enabled = !empty && d == null, onClick = {
                                pageMenu = false
                                // la nota sta accanto alla pagina: prima la si salva
                                if (board.id.isEmpty() || board.dirty) saveBoard()
                                if (board.id.isNotEmpty()) transcribePage(board.toPage(padSize.width, padSize.height), Screen.Board)
                            })
                        HorizontalDivider()
                        MenuHeader(stringResource(R.string.grp_writing))
                        DropdownMenuItem(text = { Text(stringResource(R.string.neat_fair)) }, leadingIcon = { Icon(Icons.Filled.AutoFixHigh, null) },
                            enabled = !empty && d == null, onClick = { pageMenu = false; startNeat(NeatKind.FAIR) })
                        DropdownMenuItem(text = { Text(stringResource(R.string.neat_rules)) }, leadingIcon = { Icon(Icons.Filled.FormatAlignLeft, null) },
                            enabled = !empty && d == null && paperHasRows(paperCode), onClick = { pageMenu = false; startNeat(NeatKind.RULES) })
                        HorizontalDivider()
                        MenuHeader(stringResource(R.string.grp_share))
                        DropdownMenuItem(text = { Text(stringResource(R.string.share_pdf)) }, leadingIcon = { Icon(Icons.Filled.PictureAsPdf, null) },
                            enabled = !empty, onClick = { pageMenu = false; sharePage(board.toPage(padSize.width, padSize.height), pdf = true) })
                        DropdownMenuItem(text = { Text(stringResource(R.string.share_image)) }, leadingIcon = { Icon(Icons.Filled.Image, null) },
                            enabled = !empty, onClick = { pageMenu = false; sharePage(board.toPage(padSize.width, padSize.height), pdf = false) })
                        DropdownMenuItem(text = { Text(stringResource(R.string.copy_board)) }, leadingIcon = { Icon(Icons.Filled.ContentCopy, null) },
                            enabled = !empty && d == null, onClick = { pageMenu = false; copyPage(board.toPage(padSize.width, padSize.height)) })
                        DropdownMenuItem(text = { Text(stringResource(R.string.more_formats)) }, leadingIcon = { Icon(Icons.Filled.MoreHoriz, null) },
                            enabled = !empty && d == null, onClick = { pageMenu = false; chooseMore(board.toPage(padSize.width, padSize.height), fromBoard = true) })
                        HorizontalDivider()
                        MenuHeader(stringResource(R.string.grp_document))
                        DropdownMenuItem(text = { Text(stringResource(R.string.open_pdf)) }, leadingIcon = { Icon(Icons.Filled.FileOpen, null) },
                            onClick = { pageMenu = false; pickDoc.launch(OpenDoc.MIME) })
                    }
                }
            }
            // --- documento aperto: pagine, salva, condividi, chiudi
            if (d != null) {
                Surface(color = MaterialTheme.colorScheme.secondaryContainer) {
                    Row(Modifier.fillMaxWidth().padding(horizontal = 4.dp), verticalAlignment = Alignment.CenterVertically) {
                        Icon(Icons.Filled.Description, null, Modifier.padding(start = 8.dp))
                        IconButton(onClick = { goPage(-1) }, enabled = d.index > 0) { Icon(Icons.Filled.ChevronLeft, stringResource(R.string.prev_page)) }
                        Text("${d.index + 1} / ${d.count}", style = MaterialTheme.typography.labelLarge)
                        IconButton(onClick = { goPage(1) }, enabled = d.index + 1 < d.count) { Icon(Icons.Filled.ChevronRight, stringResource(R.string.next_page)) }
                        Spacer(Modifier.weight(1f))
                        IconButton(onClick = { saveSigned.launch(d.title + " " + getString(R.string.signed_suffix) + "." + d.signedExt) }) {
                            Icon(Icons.Filled.Save, stringResource(R.string.save_signed))
                        }
                        IconButton(onClick = { signedFile()?.let { Export.share(this@MainActivity, it, d.signedMime) } }) {
                            Icon(Icons.Filled.Share, stringResource(R.string.share_signed))
                        }
                        IconButton(onClick = { closeDoc() }) { Icon(Icons.Filled.Close, stringResource(R.string.close)) }
                    }
                }
            }
            // --- stato del pad, solo quando c'è qualcosa da dire
            if (!(s is LinkStatus.Live) && !(s is LinkStatus.Idle && !btOff && !denied)) {
                Text(statusLine, Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 4.dp),
                    style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            Box(Modifier.weight(1f).fillMaxWidth()) {
                Sheet(
                    board.strokes, padSize.width, padSize.height, quarter, board.version,
                    // sul documento niente rigatura: c'è la sua pagina
                    paper = if (d == null) PaperSpec(paperCode, paperMargin) else null,
                    eraser = if (eraserOn) board.eraser else null,
                    background = d?.image?.let { it to d.rect(view()) },
                    zone = zoneJob?.takeIf { it.phase == NeatPhase.SELECT }?.let {
                        ZoneSelect(onDrag = { r -> zoneJob = zoneJob?.copy(drag = r) }, onZone = { r -> neatZone(r) })
                    },
                    overlay = zoneJob?.let { j ->
                        val p = j.preview
                        if (j.phase == NeatPhase.PREVIEW && p != null) SheetOverlay(null, p.veil.floats(), p.strokes)
                        else SheetOverlay(j.drag ?: j.zone?.floats())
                    },
                )
            }
            if (zoneJob != null) NeatBar() else ToolBar()
        }
        if (zoneJob != null) BackHandler { neatCancel() }
        if (choosingPaper) PaperDialog()
    }

    // ------------------------------------------- bella copia, sulle righe

    private fun sheetSpec(every: UInt?) = SheetSpec(paperCode, paperMargin, every)

    /** Comincia la bella copia o «Disponi sulle righe»: si sceglie la zona. */
    private fun startNeat(kind: NeatKind) {
        if (kind == NeatKind.FAIR && aiStore.services().none { it.enabled }) {
            toast(getString(R.string.ai_none))
            screen = Screen.AiSettings(Screen.Board)
            return
        }
        if (kind == NeatKind.RULES && !paperHasRows(paperCode)) {
            toast(getString(R.string.neat_unsupported))
            return
        }
        neatGen++
        eraserOn = false
        applyTools()
        zoneJob = ZoneJob(kind, note = getString(R.string.neat_hint))
    }

    private fun neatCancel() {
        neatGen++
        zoneJob = null
    }

    /** Il lavoro su un pezzo di pagina, in un thread; poi l'anteprima (o il perché no). */
    private fun neatRun(job: ZoneJob, work: () -> BoardPreview) {
        val gen = ++neatGen
        zoneJob = job.copy(phase = NeatPhase.WORKING, drag = null)
        kotlin.concurrent.thread(name = "neat") {
            val r = runCatching(work)
            runOnUiThread {
                if (gen != neatGen) return@runOnUiThread
                val cur = zoneJob ?: return@runOnUiThread
                r.onSuccess { p ->
                    zoneJob = cur.copy(phase = NeatPhase.PREVIEW, preview = p, every = p.every ?: cur.every, note = neatDoneText(this, cur.kind, p))
                }.onFailure { e ->
                    val why = neatErrorText(this, e)
                    if (why == null) { zoneJob = null; return@onFailure }
                    toast(why)
                    // si sceglie un'altra zona
                    zoneJob = cur.copy(phase = NeatPhase.SELECT, preview = null, zone = null, fair = null, note = why)
                }
            }
        }
    }

    /** La zona scelta (null = tutta la pagina): si legge o si dispone. */
    private fun neatZone(r: FloatArray?) {
        val job = zoneJob ?: return
        val zone = r?.let { Zone(it[0], it[1], it[2], it[3]) }
        val model = board.model
        val v = view()
        when (job.kind) {
            NeatKind.RULES -> {
                val every = job.every ?: if (paperIsGrid(paperCode)) boardSuggestEvery(model, v, zone, paperCode) else null
                val spec = sheetSpec(every)
                neatRun(job.copy(zone = zone, every = every, note = getString(R.string.neat_working))) { boardArrange(model, v, zone, spec, false) }
            }
            NeatKind.FAIR -> {
                val fj = FairJob(model, v, zone)
                val providers = aiStore.providers()
                val onRules = fairOnRules && paperHasRows(paperCode)
                val spec = sheetSpec(job.every)
                val gen = neatGen + 1
                val listener = object : AiListener {
                    override fun onEvent(event: AiEvent): Boolean {
                        val msg = when (event) {
                            is AiEvent.Trying -> getString(R.string.ai_trying, event.provider, event.model)
                            is AiEvent.Skipped -> getString(R.string.ai_skipped, event.provider, event.why)
                            else -> null
                        }
                        if (msg != null) runOnUiThread { if (gen == neatGen) zoneJob = zoneJob?.copy(note = msg) }
                        return gen == neatGen
                    }
                }
                neatRun(job.copy(zone = zone, fair = fj, note = getString(R.string.neat_reading))) {
                    if (fj.needsAi()) fj.read(providers, listener)
                    fj.preview(onRules, spec)
                }
            }
        }
    }

    /** L'anteprima di nuovo, cambiata un'opzione (quadretti per riga, sulle righe). */
    private fun neatRefresh(job: ZoneJob) {
        val model = board.model
        val v = view()
        val spec = sheetSpec(job.every)
        when (job.kind) {
            NeatKind.RULES -> neatRun(job) { boardArrange(model, v, job.zone, spec, false) }
            NeatKind.FAIR -> {
                val fj = job.fair ?: return
                val onRules = fairOnRules && paperHasRows(paperCode)
                neatRun(job) { fj.preview(onRules, spec) }
            }
        }
    }

    /** Accetta: il lavoro rifatto sul foglio com'è adesso, con una voce di annullamento. */
    private fun neatAccept() {
        val job = zoneJob ?: return
        val r = runCatching {
            when (job.kind) {
                NeatKind.RULES -> boardArrange(board.model, view(), job.zone, sheetSpec(job.every), true)
                NeatKind.FAIR -> job.fair!!.commit(fairOnRules && paperHasRows(paperCode), sheetSpec(job.every))
            }
        }
        neatGen++
        zoneJob = null
        board.reload()
        r.onSuccess { toast(getString(R.string.neat_applied)) }
            .onFailure { e -> neatErrorText(this, e)?.let { toast(it) } }
    }

    /** La barra durante la bella copia o «Disponi sulle righe», al posto degli strumenti. */
    @Composable
    private fun NeatBar() {
        val job = zoneJob ?: return
        val grid = paperIsGrid(paperCode) && (job.kind == NeatKind.RULES || fairOnRules)
        Surface(tonalElevation = 2.dp) {
            Column(Modifier.fillMaxWidth().padding(top = 6.dp)) {
                Row(Modifier.padding(horizontal = 12.dp), verticalAlignment = Alignment.CenterVertically) {
                    if (job.phase == NeatPhase.WORKING) {
                        CircularProgressIndicator(Modifier.size(16.dp), strokeWidth = 2.dp)
                        Spacer(Modifier.width(8.dp))
                    }
                    Text(
                        stringResource(if (job.kind == NeatKind.FAIR) R.string.neat_title_fair else R.string.neat_title_rules),
                        style = MaterialTheme.typography.titleSmall,
                    )
                }
                Text(job.note, Modifier.padding(horizontal = 12.dp, vertical = 2.dp), style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 3)
                ActionRow {
                    when (job.phase) {
                        NeatPhase.SELECT -> AssistChip(onClick = { neatZone(null) }, label = { Text(stringResource(R.string.neat_whole_page)) },
                            leadingIcon = { Icon(Icons.Filled.SelectAll, null) })
                        NeatPhase.PREVIEW -> {
                            Button(onClick = { neatAccept() }) { Text(stringResource(R.string.neat_accept)) }
                            if (job.kind == NeatKind.FAIR) {
                                OutlinedButton(onClick = { zoneJob = job.copy(fair = null); neatZone(job.zone?.floats()) }) { Text(stringResource(R.string.neat_reread)) }
                            }
                        }
                        NeatPhase.WORKING -> {}
                    }
                    if (job.kind == NeatKind.FAIR && paperHasRows(paperCode) && job.phase != NeatPhase.WORKING) {
                        FilterChip(selected = fairOnRules, onClick = {
                            fairOnRules = !fairOnRules
                            applyTools()
                            if (job.phase == NeatPhase.PREVIEW) neatRefresh(job)
                        }, label = { Text(stringResource(R.string.neat_on_rules)) }, leadingIcon = { Icon(Icons.Filled.FormatAlignLeft, null) })
                    }
                    if (grid && job.phase != NeatPhase.WORKING) {
                        Text(stringResource(R.string.neat_squares), style = MaterialTheme.typography.labelMedium)
                        for (n in 1u..3u) {
                            FilterChip(selected = job.every == n, onClick = {
                                val j = job.copy(every = n)
                                zoneJob = j
                                if (j.phase == NeatPhase.PREVIEW) neatRefresh(j)
                            }, label = { Text(n.toString()) })
                        }
                    }
                    TextButton(onClick = { neatCancel() }) {
                        Text(stringResource(if (job.phase == NeatPhase.PREVIEW) R.string.neat_reject else R.string.cancel))
                    }
                }
            }
        }
    }

    @Composable
    private fun MenuHeader(text: String) {
        Text(text, Modifier.padding(horizontal = 16.dp, vertical = 6.dp), style = MaterialTheme.typography.labelMedium,
            color = MaterialTheme.colorScheme.primary)
    }

    /** Strumenti, raggruppati: lo strumento, il colore, lo spessore, il foglio. */
    @Composable
    private fun ToolBar() {
        val hl = tool == Tool.HIGHLIGHTER
        var colors by remember { mutableStateOf(false) }
        var widths by remember { mutableStateOf(false) }
        Surface(tonalElevation = 2.dp) {
            Row(Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 6.dp), verticalAlignment = Alignment.CenterVertically) {
                SingleChoiceSegmentedButtonRow(Modifier.weight(1f)) {
                    val items = listOf(
                        Triple(stringResource(R.string.pen), Icons.Filled.Draw, tool == Tool.PEN && !eraserOn),
                        Triple(stringResource(R.string.highlighter), Icons.Filled.Highlight, hl && !eraserOn),
                        Triple(stringResource(R.string.eraser), Icons.Filled.CleaningServices, eraserOn),
                    )
                    items.forEachIndexed { i, (label, icon, sel) ->
                        SegmentedButton(
                            selected = sel,
                            onClick = {
                                when (i) {
                                    0 -> { tool = Tool.PEN; eraserOn = false }
                                    1 -> { tool = Tool.HIGHLIGHTER; eraserOn = false }
                                    else -> eraserOn = !eraserOn
                                }
                                applyTools()
                            },
                            shape = SegmentedButtonDefaults.itemShape(i, items.size),
                            icon = {},
                        ) {
                            // l'icona sempre, la scritta solo sullo strumento scelto (lo spazio è poco)
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                Icon(icon, if (sel) null else label, Modifier.size(18.dp))
                                if (sel) Text(" $label", maxLines = 1, overflow = TextOverflow.Ellipsis, style = MaterialTheme.typography.labelMedium)
                            }
                        }
                    }
                }
                // colore
                Box {
                    val current = if (hl) hlColor else penColor
                    IconButton(onClick = { colors = true }) {
                        Box(Modifier.size(26.dp).clip(CircleShape).background(androidx.compose.ui.graphics.Color(android.graphics.Color.parseColor(current)))
                            .border(1.dp, androidx.compose.ui.graphics.Color.Gray, CircleShape))
                    }
                    DropdownMenu(expanded = colors, onDismissRequest = { colors = false }) {
                        Row(Modifier.padding(8.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            for (c in if (hl) HL_COLORS else PEN_COLORS) {
                                val sel = current.equals(c, ignoreCase = true)
                                Box(
                                    Modifier.size(36.dp).clip(CircleShape)
                                        .background(androidx.compose.ui.graphics.Color(android.graphics.Color.parseColor(c)))
                                        .border(if (sel) 3.dp else 1.dp, if (sel) MaterialTheme.colorScheme.primary else androidx.compose.ui.graphics.Color.Gray, CircleShape)
                                        .clickable {
                                            if (hl) hlColor = c else penColor = c
                                            eraserOn = false
                                            applyTools()
                                            colors = false
                                        },
                                )
                            }
                        }
                    }
                }
                // spessore
                Box {
                    IconButton(onClick = { widths = true }) { Icon(Icons.Filled.LineWeight, stringResource(R.string.width)) }
                    DropdownMenu(expanded = widths, onDismissRequest = { widths = false }) {
                        val w = if (hl) hlWidth else penWidth
                        val range = if (hl) 4f..40f else 0.4f..12f
                        Column(Modifier.padding(horizontal = 16.dp, vertical = 8.dp).width(240.dp)) {
                            Text(stringResource(R.string.width) + ": " + "%.1f".format(w), style = MaterialTheme.typography.labelLarge)
                            Slider(value = w, onValueChange = {
                                if (hl) hlWidth = it else penWidth = it
                                applyTools()
                            }, valueRange = range)
                        }
                    }
                }
                // foglio (rigatura)
                IconButton(onClick = { choosingPaper = true }) { Icon(Icons.Filled.GridOn, stringResource(R.string.paper)) }
            }
        }
    }

    /** Impostazioni, per argomento: AI, foglio, scrittura. */
    @Composable
    private fun SettingsScreen() {
        BackHandler { screen = Screen.Board }
        val scroll = androidx.compose.foundation.rememberScrollState()
        Column(Modifier.fillMaxSize().verticalScroll(scroll)) {
            Text(stringResource(R.string.settings), Modifier.padding(16.dp), style = MaterialTheme.typography.headlineSmall)
            SettingsHeader(stringResource(R.string.grp_ai))
            SettingsRow(Icons.Filled.SmartToy, stringResource(R.string.ai_settings), aiSummary()) { screen = Screen.AiSettings(Screen.Settings) }
            SettingsHeader(stringResource(R.string.grp_sheet))
            SettingsRow(Icons.Filled.GridOn, stringResource(R.string.paper).trimEnd('…'), paperName(paperCode)) { choosingPaper = true }
            SettingsRow(Icons.Filled.Rotate90DegreesCw, stringResource(R.string.rotate), "${quarter * 90}°") {
                quarter = (quarter + 1) % 4
                prefs.edit().putInt("quarter", quarter).apply()
            }
            SettingsHeader(stringResource(R.string.grp_writing))
            Row(Modifier.fillMaxWidth().clickable { naturalInk = !naturalInk; applyTools() }.padding(horizontal = 16.dp, vertical = 12.dp),
                verticalAlignment = Alignment.CenterVertically) {
                Icon(Icons.Filled.Draw, null)
                Column(Modifier.weight(1f).padding(start = 16.dp)) {
                    Text(stringResource(R.string.natural_ink))
                    Text(stringResource(R.string.natural_ink_hint), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                Switch(checked = naturalInk, onCheckedChange = { naturalInk = it; applyTools() })
            }
            Row(Modifier.fillMaxWidth().clickable { fairOnRules = !fairOnRules; applyTools() }.padding(horizontal = 16.dp, vertical = 12.dp),
                verticalAlignment = Alignment.CenterVertically) {
                Icon(Icons.Filled.FormatAlignLeft, null)
                Column(Modifier.weight(1f).padding(start = 16.dp)) {
                    Text(stringResource(R.string.fair_on_rules))
                    Text(stringResource(R.string.fair_on_rules_hint), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                Switch(checked = fairOnRules, onCheckedChange = { fairOnRules = it; applyTools() })
            }
            Row(Modifier.fillMaxWidth().clickable { zigzag = !zigzag; applyTools() }.padding(horizontal = 16.dp, vertical = 12.dp),
                verticalAlignment = Alignment.CenterVertically) {
                Icon(Icons.Filled.Gesture, null)
                Column(Modifier.weight(1f).padding(start = 16.dp)) {
                    Text(stringResource(R.string.zigzag))
                    Text(stringResource(R.string.zigzag_hint), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                Switch(checked = zigzag, onCheckedChange = { zigzag = it; applyTools() })
            }
        }
        if (choosingPaper) PaperDialog()
    }

    @Composable
    private fun aiSummary(): String {
        val on = aiStore.services().filter { it.enabled }
        return if (on.isEmpty()) stringResource(R.string.ai_none_short) else on.joinToString(", ") { it.name }
    }

    @Composable
    private fun SettingsHeader(text: String) {
        Text(text, Modifier.padding(start = 16.dp, top = 16.dp, bottom = 4.dp), style = MaterialTheme.typography.labelLarge,
            color = MaterialTheme.colorScheme.primary)
    }

    @Composable
    private fun SettingsRow(icon: androidx.compose.ui.graphics.vector.ImageVector, title: String, value: String, onClick: () -> Unit) {
        Row(Modifier.fillMaxWidth().clickable(onClick = onClick).padding(horizontal = 16.dp, vertical = 12.dp), verticalAlignment = Alignment.CenterVertically) {
            Icon(icon, null)
            Column(Modifier.weight(1f).padding(start = 16.dp)) {
                Text(title)
                Text(value, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 1,
                    overflow = TextOverflow.Ellipsis)
            }
            Icon(Icons.Filled.ChevronRight, null)
        }
    }

    /** Scelta del foglio, divisa per tipo come nel menu della desktop. */
    @Composable
    private fun PaperDialog() {
        val groups = listOf(
            "" to R.string.paper_bianco,
            "Righe" to R.string.pg_lined,
            "Quadretti" to R.string.pg_squared,
            "Matematica e disegno" to R.string.pg_maths,
            "Altro" to R.string.pg_other,
        )
        AlertDialog(
            onDismissRequest = { choosingPaper = false },
            title = { Text(stringResource(R.string.paper).trimEnd('…')) },
            text = {
                LazyColumn {
                    for ((g, title) in groups) {
                        val list = PaperDraw.kinds.filter { it.group == g }
                        if (g.isNotEmpty()) item { Text(stringResource(title), style = MaterialTheme.typography.labelLarge, modifier = Modifier.padding(top = 10.dp)) }
                        items(list.size) { i ->
                            val k = list[i]
                            Row(
                                Modifier.fillMaxWidth().clickable { paperCode = k.code; applyTools(); choosingPaper = false },
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                RadioButton(selected = paperCode == k.code, onClick = null)
                                Text(paperName(k.code), Modifier.padding(start = 8.dp, top = 8.dp, bottom = 8.dp))
                            }
                        }
                    }
                    item {
                        Row(
                            Modifier.fillMaxWidth().padding(top = 12.dp).clickable { paperMargin = !paperMargin; applyTools() },
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Checkbox(checked = paperMargin, onCheckedChange = null)
                            Text(stringResource(R.string.paper_margin), Modifier.padding(start = 8.dp))
                        }
                    }
                }
            },
            confirmButton = { TextButton(onClick = { choosingPaper = false }) { Text(stringResource(R.string.ok)) } },
        )
    }

    @Composable
    private fun paperName(code: String): String {
        val id = resources.getIdentifier("paper_$code", "string", packageName)
        return if (id != 0) stringResource(id) else code
    }

    /** La lavagna com'è adesso negli appunti, come immagine. */
    fun copyPage(page: Page) {
        runCatching { Export.copy(this, page, quarter) }
            .onSuccess {
                // da Android 13 il sistema mostra già la sua conferma
                if (android.os.Build.VERSION.SDK_INT < 33) toast(getString(R.string.copied))
            }
            .onFailure {
                Log.e("OpenSlate", "copia", it)
                toast(it.message ?: "")
            }
    }

    fun sharePage(page: Page, pdf: Boolean) {
        runCatching {
            if (pdf) Export.share(this, Export.pdf(this, page, quarter), "application/pdf")
            else Export.share(this, Export.png(this, page, quarter), "image/png")
        }.onFailure {
            Log.e("OpenSlate", "esportazione", it)
            toast(it.message ?: "")
        }
    }

    @Composable
    private fun statusText(s: LinkStatus?): String = when {
        s == null -> stringResource(R.string.st_permission)
        btOff && s is LinkStatus.Idle -> stringResource(R.string.st_bt_off)
        else -> when (s) {
            LinkStatus.Idle -> stringResource(R.string.st_idle)
            LinkStatus.Searching -> stringResource(R.string.st_searching)
            LinkStatus.NotFound -> stringResource(R.string.st_not_found)
            LinkStatus.NotRegistered -> stringResource(R.string.st_not_registered)
            is LinkStatus.Connecting -> stringResource(R.string.st_connecting, s.name)
            LinkStatus.PressButton -> stringResource(R.string.st_press_button)
            LinkStatus.RegisteredRetry -> stringResource(R.string.st_registered_retry)
            is LinkStatus.Live -> stringResource(R.string.st_live)
            is LinkStatus.Syncing ->
                if (s.count == 0) stringResource(R.string.st_sync_start)
                else stringResource(R.string.st_syncing, s.index, s.count)
            is LinkStatus.Synced -> when {
                s.files == 0 -> stringResource(R.string.st_sync_none)
                s.deleted < s.files -> stringResource(R.string.st_sync_stopped, s.deleted, s.files)
                else -> stringResource(R.string.st_synced, s.deleted)
            }
            is LinkStatus.Failed -> stringResource(R.string.st_failed, s.message)
            LinkStatus.NeedRegister -> stringResource(R.string.st_need_register)
            LinkStatus.Disconnected -> stringResource(R.string.st_disconnected)
        }
    }

    @Composable
    private fun ViewerScreen(
        page: Page,
        quarter: Int,
        dir: String,
        onBack: () -> Unit,
        onChanged: (Page?) -> Unit,
        onOpenOnBoard: (Page) -> Unit,
    ) {
        BackHandler(onBack = onBack)
        PageViewer(page, quarter, dir, onBack, onChanged, onOpenOnBoard, ::sharePage, onMore = { chooseMore(it, fromBoard = false) }, onNote = {
            val title = page.title.ifBlank { page.created.take(16) }
            val note = noteOf(page.id)
            if (note != null) screen = Screen.Note(page.id, title, note, Screen.Viewer(page))
            else transcribePage(page, Screen.Viewer(page))
        })
    }
}
