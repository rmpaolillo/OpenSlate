package org.openslate.android

import android.content.Context
import uniffi.openslate_mobile.BoardPreview
import uniffi.openslate_mobile.FairJob
import uniffi.openslate_mobile.NeatException
import uniffi.openslate_mobile.NeatIssue
import uniffi.openslate_mobile.Zone

/**
 * La bella copia e «Disponi sulle righe» sul telefono (come la desktop): si
 * sceglie una zona col dito (o tutta la pagina), si guarda l'anteprima, si
 * accetta o si rifiuta. Il lavoro vero lo fa il cuore Rust (`neat.rs`).
 */
enum class NeatKind { FAIR, RULES }

enum class NeatPhase { SELECT, WORKING, PREVIEW }

data class ZoneJob(
    val kind: NeatKind,
    val phase: NeatPhase = NeatPhase.SELECT,
    /** Il riquadro mentre lo si trascina (frazioni della pagina). */
    val drag: FloatArray? = null,
    /** La zona scelta (null = tutta la pagina). */
    val zone: Zone? = null,
    val fair: FairJob? = null,
    val preview: BoardPreview? = null,
    /** Sui quadretti: una riga ogni quanti quadretti. */
    val every: UInt? = null,
    /** Cosa dire sotto il titolo (chi legge, com'è andata). */
    val note: String = "",
)

fun Zone.floats() = floatArrayOf(u0, v0, u1, v1)

fun neatIssueText(ctx: Context, issue: NeatIssue): String = when (issue) {
    is NeatIssue.Nothing -> ctx.getString(R.string.neat_nothing)
    is NeatIssue.Unsupported -> ctx.getString(R.string.neat_unsupported)
    is NeatIssue.NoRoom -> ctx.getString(R.string.neat_no_room, issue.fit.toInt(), issue.total.toInt())
}

/** Il perché di un errore, da mostrare (null = annullato da te). */
fun neatErrorText(ctx: Context, e: Throwable): String? = when (e) {
    is NeatException.Rules -> neatIssueText(ctx, e.issue)
    is NeatException.NothingToDo -> ctx.getString(R.string.neat_nothing_fair)
    is NeatException.Ai -> ctx.getString(R.string.ai_failed, e.reason)
    is NeatException.Cancelled -> null
    else -> ctx.getString(R.string.ai_failed, e.message ?: "")
}

/** Com'è andata: righe, forme, righe rimpicciolite, chi ha letto. */
fun neatDoneText(ctx: Context, kind: NeatKind, p: BoardPreview): String {
    val parts = mutableListOf(
        if (kind == NeatKind.RULES) ctx.getString(R.string.neat_done_rules, p.lines.toInt())
        else ctx.getString(R.string.neat_done_fair, p.lines.toInt(), p.shapes.toInt()),
    )
    if (p.shrunk > 0u) parts += ctx.getString(R.string.neat_shrunk, p.shrunk.toInt())
    if (p.by.isNotEmpty()) parts += ctx.getString(R.string.neat_by, p.by)
    p.rulesIssue?.let { parts += ctx.getString(R.string.neat_rules_note, neatIssueText(ctx, it)) }
    return parts.joinToString(" ")
}
