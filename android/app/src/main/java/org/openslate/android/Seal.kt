package org.openslate.android

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.RectF
import android.os.Build
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import uniffi.openslate_mobile.BoardView
import uniffi.openslate_mobile.Company
import uniffi.openslate_mobile.Frame
import uniffi.openslate_mobile.InkColor
import uniffi.openslate_mobile.PageStroke
import uniffi.openslate_mobile.SavedStamp
import uniffi.openslate_mobile.StampSpec
import uniffi.openslate_mobile.stampNew
import uniffi.openslate_mobile.stampPlace
import uniffi.openslate_mobile.stampPng
import uniffi.openslate_mobile.stampSizeCm

/**
 * Il timbro aziendale (come la desktop): i timbri si costruiscono una volta,
 * si posano sul documento, si rendono realistici (colore, grana, inchiostro
 * irregolare, inclinazione) e si confermano. L'immagine la fa il cuore Rust.
 */
object SealArt {
    private data class Key(val spec: StampSpec, val quarter: Int, val step: Int)

    private val cache = object : LinkedHashMap<Key, Bitmap>(16, 0.75f, true) {
        override fun removeEldestEntry(eldest: MutableMap.MutableEntry<Key, Bitmap>) = size > 12
    }

    /** L'immagine del timbro per un riquadro largo `pxW` pixel (a scalini: si rifà di rado). */
    fun bitmap(spec: StampSpec, quarter: Int, pxW: Float): Bitmap? {
        val step = (Math.ceil(pxW.coerceIn(64f, 2400f) / 160.0).toInt()) * 160
        val key = Key(spec, quarter, step)
        synchronized(cache) { cache[key]?.let { return it } }
        val png = stampPng(spec, step.toFloat(), quarter.toUByte())
        val bmp = BitmapFactory.decodeByteArray(png, 0, png.size) ?: return null
        synchronized(cache) { cache[key] = bmp }
        return bmp
    }

    /** Come l'inchiostro vero: «moltiplica» sulla carta (da Android 10). */
    val paint = Paint(Paint.ANTI_ALIAS_FLAG or Paint.FILTER_BITMAP_FLAG).apply {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) blendMode = android.graphics.BlendMode.MULTIPLY
    }

    /** Disegna il timbro nel riquadro (pixel del canvas). */
    fun draw(c: Canvas, spec: StampSpec, quarter: Int, l: Float, t: Float, r: Float, b: Float) {
        val bmp = bitmap(spec, quarter, r - l) ?: return
        c.drawBitmap(bmp, null, RectF(l, t, r, b), paint)
    }

    /** I timbri posati su una pagina (`Ink.draw`): `dstW` x `dstH` è il foglio visto. */
    fun drawPlaced(c: Canvas, s: PageStroke, padW: Float, padH: Float, quarter: Int, dstW: Float, dstH: Float) {
        val spec = s.stamp ?: return
        val p = stampPlace(s.points, BoardView(quarter.toUInt(), padW, padH)) ?: return
        val r = p.rect
        draw(c, spec, p.quarter.toInt(), r[0] * dstW, r[1] * dstH, r[2] * dstW, r[3] * dstH)
    }
}

/** Il timbro che si sta posando: riquadro (u, v, larghezza, altezza) del foglio visto. */
class SealPlace(val spec: StampSpec, val rect: List<Float>, val onRect: (List<Float>) -> Unit)

/** Un timbro in corso di posa. */
data class SealJob(val spec: StampSpec, val saved: Int?, val rect: List<Float>)

fun inkColorOf(c: InkColor) = when (c) {
    InkColor.BLUE -> Color(0xFF223E96)
    InkColor.GREY -> Color(0xFF5C5E64)
    InkColor.BLACK -> Color(0xFF202024)
    InkColor.RED -> Color(0xFFB2262C)
    InkColor.VIOLET -> Color(0xFF62348E)
}

fun inkColorName(c: InkColor) = when (c) {
    InkColor.BLUE -> R.string.stamp_blue
    InkColor.GREY -> R.string.stamp_grey
    InkColor.BLACK -> R.string.stamp_black
    InkColor.RED -> R.string.stamp_red
    InkColor.VIOLET -> R.string.stamp_violet
}

fun frameName(f: Frame) = when (f) {
    Frame.PLAIN -> R.string.stamp_frame_none
    Frame.RECT -> R.string.stamp_frame_rect
    Frame.ROUNDED -> R.string.stamp_frame_rounded
    Frame.DOUBLE -> R.string.stamp_frame_double
    Frame.OVAL -> R.string.stamp_frame_oval
}

/** I pallini dei colori da timbro. */
@Composable
fun InkDots(selected: InkColor, onPick: (InkColor) -> Unit) {
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
        for (c in InkColor.entries) {
            Box(
                Modifier.size(28.dp)
                    .border(if (c == selected) 2.dp else 0.dp, if (c == selected) MaterialTheme.colorScheme.onSurface else Color.Transparent, CircleShape)
                    .padding(4.dp)
                    .background(inkColorOf(c), CircleShape)
                    .clickable { onPick(c) },
            )
        }
    }
}

/** Le cornici, una per chip. */
@Composable
fun FrameChips(selected: Frame, onPick: (Frame) -> Unit) {
    Row(Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
        for (f in Frame.entries) {
            FilterChip(selected = f == selected, onClick = { onPick(f) }, label = { Text(stringResource(frameName(f))) })
        }
    }
}

/**
 * La finestra dei timbri: l'elenco, i dati dell'azienda, cornice e colore,
 * l'anteprima. `onSave` restituisce la posizione del timbro salvato.
 */
@Composable
fun StampDialog(
    stamps: List<SavedStamp>,
    start: Int?,
    canPlace: Boolean,
    onSave: (Int?, SavedStamp) -> Int,
    onDelete: (Int) -> Unit,
    onPlace: (Int) -> Unit,
    onClose: () -> Unit,
) {
    var sel by remember { mutableStateOf(start?.takeIf { it in stamps.indices }) }
    var draft by remember { mutableStateOf(sel?.let { stamps[it] } ?: stampNew()) }
    fun company(f: (Company) -> Company) { draft = draft.copy(spec = draft.spec.copy(company = f(draft.spec.company))) }
    Dialog(onDismissRequest = onClose, properties = DialogProperties(usePlatformDefaultWidth = false)) {
        Surface(Modifier.fillMaxWidth(0.95f).heightIn(max = 720.dp), shape = RoundedCornerShape(16.dp)) {
            Column(Modifier.verticalScroll(rememberScrollState()).padding(16.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                Text(stringResource(R.string.stamp_title), style = MaterialTheme.typography.titleLarge)
                // l'elenco
                Row(Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    stamps.forEachIndexed { i, s ->
                        FilterChip(selected = sel == i, onClick = { sel = i; draft = s }, label = { Text(s.name.ifBlank { s.spec.company.name }) })
                    }
                    FilterChip(selected = sel == null, onClick = { sel = null; draft = stampNew() }, label = { Text(stringResource(R.string.stamp_new)) })
                }
                val c = draft.spec.company
                OutlinedTextField(draft.name, { v -> draft = draft.copy(name = v) }, Modifier.fillMaxWidth(), label = { Text(stringResource(R.string.stamp_name)) }, singleLine = true)
                OutlinedTextField(c.name, { v -> company { it.copy(name = v) } }, Modifier.fillMaxWidth(), label = { Text(stringResource(R.string.stamp_company)) }, singleLine = true)
                OutlinedTextField(c.address, { v -> company { it.copy(address = v) } }, Modifier.fillMaxWidth(), label = { Text(stringResource(R.string.stamp_address)) }, singleLine = true)
                Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    OutlinedTextField(c.cap, { v -> company { it.copy(cap = v) } }, Modifier.width(96.dp), label = { Text(stringResource(R.string.stamp_cap)) }, singleLine = true)
                    OutlinedTextField(c.city, { v -> company { it.copy(city = v) } }, Modifier.weight(1f), label = { Text(stringResource(R.string.stamp_city)) }, singleLine = true)
                    OutlinedTextField(c.prov, { v -> company { it.copy(prov = v) } }, Modifier.width(72.dp), label = { Text(stringResource(R.string.stamp_prov)) }, singleLine = true)
                }
                OutlinedTextField(c.vat, { v -> company { it.copy(vat = v) } }, Modifier.fillMaxWidth(), label = { Text(stringResource(R.string.stamp_vat)) }, singleLine = true)
                Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    OutlinedTextField(c.rea, { v -> company { it.copy(rea = v) } }, Modifier.weight(1f), label = { Text(stringResource(R.string.stamp_rea)) }, singleLine = true)
                    OutlinedTextField(c.sdi, { v -> company { it.copy(sdi = v) } }, Modifier.weight(1f), label = { Text(stringResource(R.string.stamp_sdi)) }, singleLine = true)
                }
                Text(stringResource(R.string.stamp_frame), style = MaterialTheme.typography.labelLarge)
                FrameChips(draft.spec.style.frame) { f -> draft = draft.copy(spec = draft.spec.copy(style = draft.spec.style.copy(frame = f))) }
                InkDots(draft.spec.style.color) { col -> draft = draft.copy(spec = draft.spec.copy(style = draft.spec.style.copy(color = col))) }
                // anteprima sul bianco, come sulla carta
                // l'anteprima in un thread, poco dopo l'ultimo tasto: scrivere resta svelto
                val spec = draft.spec
                var preview by remember { mutableStateOf<androidx.compose.ui.graphics.ImageBitmap?>(null) }
                androidx.compose.runtime.LaunchedEffect(spec) {
                    if (preview != null) kotlinx.coroutines.delay(250)
                    kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.Default) {
                        val png = stampPng(spec, 700f, 0u)
                        BitmapFactory.decodeByteArray(png, 0, png.size)?.asImageBitmap()
                    }?.let { preview = it }
                }
                Box(Modifier.fillMaxWidth().height(170.dp).background(Color.White, RoundedCornerShape(8.dp)).padding(8.dp), contentAlignment = Alignment.Center) {
                    preview?.let { Image(it, null) }
                }
                val size = remember(spec) { stampSizeCm(spec) }
                Text(stringResource(R.string.stamp_size, size[0], size[1]), style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant)
                val ok = c.name.isNotBlank()
                Row(Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                    Button(onClick = { sel = onSave(sel, draft) }, enabled = ok) { Text(stringResource(R.string.save)) }
                    OutlinedButton(onClick = { onPlace(onSave(sel, draft)) }, enabled = ok && canPlace) { Text(stringResource(R.string.stamp_place)) }
                    sel?.let { i ->
                        TextButton(onClick = { onDelete(i); sel = null; draft = stampNew() }) { Text(stringResource(R.string.delete)) }
                    }
                    Spacer(Modifier.width(4.dp))
                    TextButton(onClick = onClose) { Text(stringResource(R.string.close)) }
                }
            }
        }
    }
}
