package org.openslate.android

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.calculateCentroid
import androidx.compose.foundation.gestures.calculatePan
import androidx.compose.foundation.gestures.calculateZoom
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.graphics.drawscope.translate
import androidx.compose.ui.graphics.drawscope.drawIntoCanvas
import androidx.compose.ui.graphics.nativeCanvas
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import uniffi.openslate_mobile.Page
import uniffi.openslate_mobile.PageStroke
import uniffi.openslate_mobile.libraryList
import uniffi.openslate_mobile.libraryLoad
import uniffi.openslate_mobile.libraryRename
import uniffi.openslate_mobile.libraryTrash
import uniffi.openslate_mobile.paperMarks
import java.time.LocalDate
import java.time.format.DateTimeFormatter
import java.time.format.FormatStyle
import kotlin.math.min

/** Una fila di pulsanti che scorre di lato se non ci sta. */
@Composable
fun ActionRow(content: @Composable RowScope.() -> Unit) {
    Row(
        Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(horizontal = 12.dp, vertical = 8.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically,
        content = content,
    )
}

/** Rigatura da mostrare sotto i tratti. */
data class PaperSpec(val code: String, val margin: Boolean)

/** Gomma col dito: segmento a→b in unità del pad, raggio `tol`, altezza del foglio visto `pageH`. */
interface FingerEraser {
    fun erase(ax: Float, ay: Float, bx: Float, by: Float, tol: Float, pageH: Float)
    fun end()
}

/**
 * Il foglio. Due dita: ingrandisci e sposta; doppio tocco: pagina intera. Con
 * la gomma (`eraser` non nullo) un dito cancella ciò che strofina; altrimenti
 * un dito sposta il foglio ingrandito. `version` cambia quando cambiano i tratti.
 */
@Composable
fun Sheet(
    strokes: List<PageStroke>,
    padW: Float,
    padH: Float,
    quarter: Int,
    version: Int,
    paper: PaperSpec? = null,
    eraser: FingerEraser? = null,
    /** La pagina di un documento sotto i tratti, e dove sta (u, v, larghezza, altezza in 0..1). */
    background: Pair<androidx.compose.ui.graphics.ImageBitmap, List<Float>>? = null,
) {
    var zoom by remember { mutableFloatStateOf(1f) }
    var offset by remember { mutableStateOf(Offset.Zero) }
    var canvasSize by remember { mutableStateOf(Size.Zero) }
    val density = LocalDensity.current.density
    val margin = 12f * density
    val (rw, rh) = Ink.rotated(padW, padH, quarter)
    /** Pagina intera (zoom 1): scala pad→px e angolo in alto a sinistra. */
    fun fit(size: Size): Triple<Float, Float, Float> {
        val k = min((size.width - 2 * margin) / rw, (size.height - 2 * margin) / rh)
        return Triple(k, (size.width - rw * k) / 2, (size.height - rh * k) / 2)
    }
    /** Punto dello schermo → coordinate del pad. */
    fun toPad(p: Offset): Offset {
        val (k, ox, oy) = fit(canvasSize)
        val X = ((p.x - offset.x) / zoom - ox) / k
        val Y = ((p.y - offset.y) / zoom - oy) / k
        return when (quarter) {
            0 -> Offset(X, Y)
            1 -> Offset(Y, padH - X)
            2 -> Offset(padW - X, padH - Y)
            else -> Offset(padW - Y, X)
        }
    }
    val marks = remember(paper, rw, rh) {
        paper?.takeIf { it.code != "bianco" }?.let { paperMarks(it.code, rw / 1000f, rh / 1000f, it.margin) } ?: emptyList()
    }
    val labelKeywords = stringResource(R.string.lbl_keywords)
    val labelNotes = stringResource(R.string.lbl_notes)
    val labelSummary = stringResource(R.string.lbl_summary)
    val currentEraser by rememberUpdatedState(eraser)
    Canvas(
        Modifier
            .fillMaxSize()
            .background(Color(0xFFE6E6E6))
            .clipToBounds()
            // la misura si prende qui: scriverla mentre si disegna ridisegnerebbe all'infinito
            .onSizeChanged { canvasSize = Size(it.width.toFloat(), it.height.toFloat()) }
            .pointerInput(Unit) {
                detectTapGestures(onDoubleTap = {
                    zoom = 1f
                    offset = Offset.Zero
                })
            }
            .pointerInput(quarter, padW, padH) {
                awaitEachGesture {
                    val first = awaitFirstDown(requireUnconsumed = false)
                    var last = first.position
                    var fingers = 1
                    var erased = false
                    while (true) {
                        val ev = awaitPointerEvent()
                        val down = ev.changes.filter { it.pressed }
                        if (down.isEmpty()) break
                        if (down.size != fingers) {
                            // cambia il numero di dita: si riparte da qui, senza salti
                            fingers = down.size
                            last = down[0].position
                            continue
                        }
                        if (down.size >= 2) {
                            val z = (zoom * ev.calculateZoom()).coerceIn(1f, 8f)
                            val c = ev.calculateCentroid()
                            offset = c - (c - offset) * (z / zoom) + ev.calculatePan()
                            zoom = z
                            if (zoom == 1f) offset = Offset.Zero
                            ev.changes.forEach { it.consume() }
                        } else {
                            val p = down[0].position
                            val er = currentEraser
                            if (er != null) {
                                val a = toPad(last)
                                val b = toPad(p)
                                val (k, _, _) = fit(canvasSize)
                                er.erase(a.x, a.y, b.x, b.y, 14f * density / (k * zoom), rh)
                                erased = true
                                down[0].consume()
                            } else if (zoom > 1f) {
                                offset += p - last
                                down[0].consume()
                            }
                            last = p
                        }
                    }
                    if (erased) currentEraser?.end()
                }
            },
    ) {
        version // si ridisegna a ogni cambiamento
        val (k, ox, oy) = fit(size)
        val pw = rw * k
        val ph = rh * k
        // il foglio bianco
        translate(offset.x, offset.y) {
            scale(zoom, Offset.Zero) {
                drawRect(Color.White, Offset(ox, oy), Size(pw, ph))
            }
        }
        // la rigatura a spessore fisso, sopra il bianco e sotto i tratti
        if (marks.isNotEmpty()) {
            val kind = PaperDraw.kind(paper!!.code)
            drawIntoCanvas {
                val c = it.nativeCanvas
                c.save()
                c.clipRect(offset.x + ox * zoom, offset.y + oy * zoom, offset.x + (ox + pw) * zoom, offset.y + (oy + ph) * zoom)
                PaperDraw.draw(c, marks, kind, offset.x + ox * zoom, offset.y + oy * zoom, pw * zoom / (rw / 1000f), density) { t ->
                    when (t) {
                        "Parole chiave" -> labelKeywords
                        "Appunti" -> labelNotes
                        "Riassunto" -> labelSummary
                        else -> t
                    }
                }
                c.restore()
            }
        }
        translate(offset.x, offset.y) {
            scale(zoom, Offset.Zero) {
                translate(ox, oy) {
                    background?.let { (img, r) ->
                        drawImage(
                            img,
                            dstOffset = androidx.compose.ui.unit.IntOffset((r[0] * pw).toInt(), (r[1] * ph).toInt()),
                            dstSize = androidx.compose.ui.unit.IntSize((r[2] * pw).toInt(), (r[3] * ph).toInt()),
                        )
                    }
                    drawIntoCanvas { Ink.draw(it.nativeCanvas, strokes, padW, padH, quarter, pw, ph) }
                }
            }
        }
    }
}

/** L'archivio: le pagine raggruppate per giorno, dalla più recente. */
@Composable
fun LibraryScreen(dir: String, version: Int, onBack: () -> Unit, onOpen: (String) -> Unit) {
    BackHandler(onBack = onBack)
    val pages = remember(version) { libraryList(dir) }
    Column(Modifier.fillMaxSize()) {
        ActionRow {
            TextButton(onClick = onBack) { Text("← " + stringResource(R.string.back)) }
            Text(stringResource(R.string.library), style = MaterialTheme.typography.titleLarge)
        }
        if (pages.isEmpty()) {
            Text(stringResource(R.string.library_empty), Modifier.padding(24.dp))
            return@Column
        }
        val days = pages.groupBy { it.day }
        LazyColumn(Modifier.fillMaxSize()) {
            days.forEach { (day, list) ->
                item(key = "day-$day") {
                    Text(
                        dayLabel(day),
                        Modifier.fillMaxWidth().background(MaterialTheme.colorScheme.surfaceVariant)
                            .padding(horizontal = 16.dp, vertical = 6.dp),
                        style = MaterialTheme.typography.labelLarge,
                    )
                }
                items(list, key = { it.id }) { p ->
                    Column(Modifier.fillMaxWidth().clickable { onOpen(p.id) }.padding(horizontal = 16.dp, vertical = 12.dp)) {
                        Text(p.title, style = MaterialTheme.typography.bodyLarge, maxLines = 1, overflow = TextOverflow.Ellipsis)
                        val src = stringResource(if (p.source == "pad") R.string.src_pad else R.string.src_board)
                        Text(
                            "${p.created.drop(11).take(5)} · $src",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    HorizontalDivider()
                }
            }
        }
    }
}

private fun dayLabel(day: String): String = runCatching {
    LocalDate.parse(day).format(DateTimeFormatter.ofLocalizedDate(FormatStyle.FULL))
}.getOrDefault(day)

/** Una pagina dell'archivio: condividi, apri sulla lavagna, rinomina, elimina. */
@Composable
fun PageViewer(
    page: Page,
    quarter: Int,
    dir: String,
    onBack: () -> Unit,
    onChanged: (Page?) -> Unit,
    onOpenOnBoard: (Page) -> Unit,
    share: (Page, Boolean) -> Unit,
    onNote: () -> Unit,
) {
    var renaming by remember { mutableStateOf(false) }
    var deleting by remember { mutableStateOf(false) }
    val shown = page.title.ifBlank { titleFromDate(page.created) }
    Column(Modifier.fillMaxSize()) {
        var menu by remember { mutableStateOf(false) }
        Row(verticalAlignment = Alignment.CenterVertically) {
            Box(Modifier.weight(1f)) {
                ActionRow {
                    TextButton(onClick = onBack) { Text("← " + stringResource(R.string.back)) }
                    OutlinedButton(onClick = { onOpenOnBoard(page) }) { Text(stringResource(R.string.open_on_board)) }
                    OutlinedButton(onClick = onNote) { Text(stringResource(R.string.note)) }
                }
            }
            Box {
                TextButton(onClick = { menu = true }) { Text("⋮", style = MaterialTheme.typography.titleLarge) }
                DropdownMenu(expanded = menu, onDismissRequest = { menu = false }) {
                    DropdownMenuItem(text = { Text(stringResource(R.string.share_pdf)) }, onClick = { menu = false; share(page, true) })
                    DropdownMenuItem(text = { Text(stringResource(R.string.share_image)) }, onClick = { menu = false; share(page, false) })
                    DropdownMenuItem(text = { Text(stringResource(R.string.rename)) }, onClick = { menu = false; renaming = true })
                    DropdownMenuItem(text = { Text(stringResource(R.string.delete)) }, onClick = { menu = false; deleting = true })
                }
            }
        }
        Text(shown, Modifier.padding(horizontal = 12.dp), style = MaterialTheme.typography.titleMedium)
        Box(Modifier.weight(1f).fillMaxWidth()) {
            Sheet(page.strokes, page.width, page.height, quarter, 0)
        }
    }
    if (renaming) {
        var text by remember { mutableStateOf(page.title) }
        AlertDialog(
            onDismissRequest = { renaming = false },
            title = { Text(stringResource(R.string.rename)) },
            text = {
                OutlinedTextField(text, { text = it }, singleLine = true, label = { Text(stringResource(R.string.title_hint)) })
            },
            confirmButton = {
                TextButton(onClick = {
                    renaming = false
                    runCatching { libraryRename(dir, page.id, text) }
                    onChanged(runCatching { libraryLoad(dir, page.id) }.getOrDefault(page))
                }) { Text(stringResource(R.string.ok)) }
            },
            dismissButton = { TextButton(onClick = { renaming = false }) { Text(stringResource(R.string.cancel)) } },
        )
    }
    if (deleting) {
        AlertDialog(
            onDismissRequest = { deleting = false },
            text = { Text(stringResource(R.string.delete_confirm, shown)) },
            confirmButton = {
                TextButton(onClick = {
                    deleting = false
                    runCatching { libraryTrash(dir, page.id) }
                    onChanged(null)
                }) { Text(stringResource(R.string.delete)) }
            },
            dismissButton = { TextButton(onClick = { deleting = false }) { Text(stringResource(R.string.cancel)) } },
        )
    }
}

/** "AAAA-MM-GG hh:mm:ss" → "GG/MM/AAAA hh:mm" (come la desktop). */
private fun titleFromDate(created: String): String =
    if (created.length >= 16) "${created.substring(8, 10)}/${created.substring(5, 7)}/${created.take(4)} ${created.substring(11, 16)}"
    else created
