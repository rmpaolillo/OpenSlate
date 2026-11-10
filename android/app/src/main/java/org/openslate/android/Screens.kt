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

/**
 * Scelta di una zona col dito (bella copia, «Disponi sulle righe»): il
 * riquadro mentre lo si trascina (u0, v0, u1, v1 in frazioni della pagina
 * vista; null = niente) e quello scelto quando si alza il dito.
 */
class ZoneSelect(val onDrag: (FloatArray?) -> Unit, val onZone: (FloatArray) -> Unit)

/**
 * Sopra il foglio: il riquadro che si sta trascinando, oppure l'anteprima (la
 * zona velata, le righe del foglio e sopra i tratti di come sarà la pagina).
 */
class SheetOverlay(val rect: FloatArray?, val veil: FloatArray? = null, val preview: List<PageStroke>? = null)

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
    /** Un dito sceglie una zona (invece di spostare il foglio). */
    zone: ZoneSelect? = null,
    overlay: SheetOverlay? = null,
    /** Un timbro da posare: un dito lo sposta, l'angolo lo ingrandisce. */
    seal: SealPlace? = null,
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
    val currentZone by rememberUpdatedState(zone)
    val currentSeal by rememberUpdatedState(seal)
    /** Punto dello schermo → frazioni della pagina vista (0..1). */
    fun toUnit(p: Offset): Offset {
        val (k, ox, oy) = fit(canvasSize)
        val u = ((p.x - offset.x) / zoom - ox) / (rw * k)
        val v = ((p.y - offset.y) / zoom - oy) / (rh * k)
        return Offset(u.coerceIn(0f, 1f), v.coerceIn(0f, 1f))
    }
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
                    val zoneStart = currentZone?.let { toUnit(first.position) }
                    var zoneRect: FloatArray? = null
                    // il timbro: preso nel riquadro (sposta) o sull'angolo (grandezza)
                    val sealMode = currentSeal?.let { sp ->
                        val (k, ox, oy) = fit(canvasSize)
                        val (pw, ph) = rw * k to rh * k
                        val r = sp.rect
                        val l = offset.x + (ox + r[0] * pw) * zoom
                        val t = offset.y + (oy + r[1] * ph) * zoom
                        val rr = offset.x + (ox + (r[0] + r[2]) * pw) * zoom
                        val b = offset.y + (oy + (r[1] + r[3]) * ph) * zoom
                        val p = first.position
                        val grip = 28f * density
                        when {
                            kotlin.math.abs(p.x - rr) < grip && kotlin.math.abs(p.y - b) < grip -> 2
                            p.x in l - 8f..rr + 8f && p.y in t - 8f..b + 8f -> 1
                            else -> 0
                        }
                    } ?: 0
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
                            val zs = currentZone
                            val sp = currentSeal
                            if (sp != null && sealMode != 0) {
                                val (k, _, _) = fit(canvasSize)
                                val du = (p.x - last.x) / (rw * k * zoom)
                                val dv = (p.y - last.y) / (rh * k * zoom)
                                val r = sp.rect
                                if (sealMode == 1) {
                                    sp.onRect(listOf(r[0] + du, r[1] + dv, r[2], r[3]))
                                } else {
                                    val f = ((r[2] + du) / r[2]).coerceIn(0.5f, 2f)
                                    if (r[2] * f > 0.03f && r[2] * f < 1.2f) sp.onRect(listOf(r[0], r[1], r[2] * f, r[3] * f))
                                }
                                down[0].consume()
                            } else if (zs != null && zoneStart != null) {
                                val b = toUnit(p)
                                zoneRect = floatArrayOf(min(zoneStart.x, b.x), min(zoneStart.y, b.y), maxOf(zoneStart.x, b.x), maxOf(zoneStart.y, b.y))
                                zs.onDrag(zoneRect)
                                down[0].consume()
                            } else if (er != null) {
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
                    currentZone?.let { zs ->
                        val r = zoneRect
                        if (r != null && r[2] - r[0] > 0.01f && r[3] - r[1] > 0.01f) zs.onZone(r) else zs.onDrag(null)
                    }
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
        // il timbro da posare, con il bordo tratteggiato e l'angolo da tirare
        seal?.let { sp ->
            val r = sp.rect
            val l = offset.x + (ox + r[0] * pw) * zoom
            val t = offset.y + (oy + r[1] * ph) * zoom
            val rr = offset.x + (ox + (r[0] + r[2]) * pw) * zoom
            val b = offset.y + (oy + (r[1] + r[3]) * ph) * zoom
            drawIntoCanvas { SealArt.draw(it.nativeCanvas, sp.spec, 0, l, t, rr, b) }
            val accent = Color(0xFF2868DC)
            val g = 6f * density
            drawRect(
                accent, Offset(l - g, t - g), Size(rr - l + 2 * g, b - t + 2 * g),
                style = androidx.compose.ui.graphics.drawscope.Stroke(
                    width = 1.5f * density,
                    pathEffect = androidx.compose.ui.graphics.PathEffect.dashPathEffect(floatArrayOf(6f * density, 4f * density)),
                ),
            )
            drawRect(accent, Offset(rr - 7f * density, b - 7f * density), Size(14f * density, 14f * density))
        }
        // la zona scelta, o l'anteprima sotto un velo
        overlay?.let { o ->
            val accent = Color(0xFF2868DC)
            fun screen(r: FloatArray) = androidx.compose.ui.geometry.Rect(
                offset.x + (ox + r[0] * pw) * zoom, offset.y + (oy + r[1] * ph) * zoom,
                offset.x + (ox + r[2] * pw) * zoom, offset.y + (oy + r[3] * ph) * zoom,
            )
            val dash = androidx.compose.ui.graphics.drawscope.Stroke(
                width = 1.5f * density,
                pathEffect = androidx.compose.ui.graphics.PathEffect.dashPathEffect(floatArrayOf(6f * density, 4f * density)),
            )
            o.veil?.let { v ->
                val r = screen(v)
                drawRect(Color.White.copy(alpha = 0.88f), r.topLeft, r.size)
                drawIntoCanvas {
                    val c = it.nativeCanvas
                    c.save()
                    c.clipRect(r.left, r.top, r.right, r.bottom)
                    if (marks.isNotEmpty()) {
                        PaperDraw.draw(c, marks, PaperDraw.kind(paper!!.code), offset.x + ox * zoom, offset.y + oy * zoom, pw * zoom / (rw / 1000f), density) { t -> t }
                    }
                    c.translate(offset.x + ox * zoom, offset.y + oy * zoom)
                    c.scale(zoom, zoom)
                    o.preview?.let { Ink.draw(c, it, padW, padH, quarter, pw, ph) }
                    c.restore()
                }
                drawRect(accent, r.topLeft, r.size, style = dash)
            }
            o.rect?.let { z ->
                val r = screen(z)
                drawRect(accent.copy(alpha = 0.09f), r.topLeft, r.size)
                drawRect(accent, r.topLeft, r.size, style = dash)
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
    onMore: (Page) -> Unit,
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
                    DropdownMenuItem(text = { Text(stringResource(R.string.more_formats)) }, onClick = { menu = false; onMore(page) })
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
