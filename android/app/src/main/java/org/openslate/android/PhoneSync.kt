package org.openslate.android

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.provider.Settings
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import org.json.JSONArray
import org.json.JSONObject
import uniffi.openslate_mobile.SyncAi
import uniffi.openslate_mobile.SyncPairing
import uniffi.openslate_mobile.syncParseQr
import uniffi.openslate_mobile.syncRun
import uniffi.openslate_mobile.syncServiceType
import java.io.File
import java.text.DateFormat
import java.util.Collections
import java.util.Date
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.concurrent.thread

/**
 * La sincronizzazione col computer (il cuore è in Rust, `src/sync`): il
 * telefono si abbina una volta col QR della desktop, poi si sincronizza da
 * solo quando l'app torna in primo piano, poco dopo ogni salvataggio e ogni
 * minuto mentre è aperta. Il segreto sta cifrato con l'Android Keystore.
 */
class PhoneSync(
    private val ctx: Context,
    private val dataDir: File,
    private val libDir: File,
    /** Cosa è cambiato sul telefono ("lib", "notes", "stamps"…): sul thread principale. */
    private val onChanged: (List<String>) -> Unit,
    /** Le impostazioni AI (servizi e chiavi) da mandare al computer. */
    private val aiExport: () -> SyncAi?,
    /** Le impostazioni AI arrivate dal computer (JSON di `ai_sync_json`). */
    private val aiImport: (String) -> Unit,
) {
    private val prefs = ctx.getSharedPreferences("sync", Context.MODE_PRIVATE)
    private val main = Handler(Looper.getMainLooper())
    private val running = AtomicBoolean(false)
    private var resumed = false

    /** Per lo schermo: a chi è collegato, l'ultima volta, l'ultimo errore, se sta lavorando. */
    var deskName by mutableStateOf(prefs.getString("desk_name", null))
        private set
    var lastOk by mutableStateOf(prefs.getLong("last_ok", 0L))
        private set
    var lastError by mutableStateOf<String?>(null)
        private set
    var busy by mutableStateOf(false)
        private set

    val paired get() = deskName != null

    fun lastLabel(): String? = lastOk.takeIf { it > 0 }?.let {
        DateFormat.getDateTimeInstance(DateFormat.SHORT, DateFormat.SHORT).format(Date(it))
    }

    private fun pairing(): SyncPairing? {
        val j = prefs.getString("pairing", null)?.let { runCatching { JSONObject(it) }.getOrNull() } ?: return null
        val secret = prefs.getString("secret", null)?.let(Secrets::decrypt) ?: return null
        val hosts = j.getJSONArray("hosts").let { a -> (0 until a.length()).map { a.getString(it) } }
        return SyncPairing(j.getString("desk_id"), j.getString("desk_name"), hosts, j.getInt("port").toUShort(), j.getString("peer_id"), secret)
    }

    /** Il QR del computer: si abbina e si sincronizza subito. */
    fun pair(qr: String): Result<String> = runCatching {
        val p = syncParseQr(qr)
        val j = JSONObject()
            .put("desk_id", p.deskId).put("desk_name", p.deskName)
            .put("hosts", JSONArray(p.hosts)).put("port", p.port.toInt()).put("peer_id", p.peerId)
        prefs.edit()
            .putString("pairing", j.toString())
            .putString("secret", Secrets.encrypt(p.secret))
            .putString("desk_name", p.deskName)
            .remove("last_host").putLong("last_ok", 0L)
            .apply()
        deskName = p.deskName
        lastOk = 0L
        lastError = null
        syncSoon(0)
        p.deskName
    }

    fun unpair() {
        prefs.edit().clear().apply()
        deskName = null
        lastOk = 0L
        lastError = null
    }

    /** L'app è in primo piano: sincronizza adesso e poi ogni minuto. */
    fun resume() {
        resumed = true
        syncSoon(500)
        main.removeCallbacks(tick)
        main.postDelayed(tick, PERIOD)
    }

    fun pause() {
        resumed = false
        main.removeCallbacks(tick)
    }

    private val tick: Runnable = object : Runnable {
        override fun run() {
            if (!resumed) return
            syncSoon(0)
            main.postDelayed(this, PERIOD)
        }
    }

    private val now = Runnable { runNow() }

    /** Sincronizza fra `delayMs` (le richieste vicine diventano una sola). */
    fun syncSoon(delayMs: Long) {
        if (!paired) return
        main.removeCallbacks(now)
        main.postDelayed(now, delayMs)
    }

    private fun deviceName(): String =
        Settings.Global.getString(ctx.contentResolver, Settings.Global.DEVICE_NAME)?.takeIf { it.isNotBlank() } ?: Build.MODEL

    private fun runNow() {
        val p = pairing() ?: return
        if (!running.compareAndSet(false, true)) return
        busy = true
        val lastHost = prefs.getString("last_host", null)
        val ai = runCatching { aiExport() }.getOrNull()
        thread(name = "sync") {
            val run = { found: List<String> -> runCatching { syncRun(p, found, deviceName(), dataDir.path, libDir.path, ai) } }
            var r = run(listOfNotNull(lastHost))
            // il computer ha cambiato indirizzo: lo si cerca sulla rete (Bonjour)
            if (r.isFailure && r.exceptionOrNull()?.message?.contains("QR") != true) {
                val found = discover(p.deskId)
                if (found.isNotEmpty()) r = run(found)
            }
            main.post {
                running.set(false)
                busy = false
                r.onSuccess { rep ->
                    lastOk = System.currentTimeMillis()
                    lastError = null
                    prefs.edit().putString("last_host", rep.host).putLong("last_ok", lastOk).apply()
                    rep.ai?.let { runCatching { aiImport(it) } }
                    if (rep.changed.isNotEmpty()) onChanged(rep.changed)
                }.onFailure { lastError = it.message }
            }
        }
    }

    /** Cerca il computer sulla rete di casa (al massimo qualche secondo). */
    private fun discover(deskId: String): List<String> {
        val nsd = ctx.getSystemService(Context.NSD_SERVICE) as? NsdManager ?: return emptyList()
        val found = Collections.synchronizedList(mutableListOf<String>())
        val done = CountDownLatch(1)
        val listener = object : NsdManager.DiscoveryListener {
            override fun onServiceFound(info: NsdServiceInfo) {
                if (!info.serviceName.contains(deskId)) return
                @Suppress("DEPRECATION")
                nsd.resolveService(info, object : NsdManager.ResolveListener {
                    override fun onResolveFailed(i: NsdServiceInfo, code: Int) = Unit
                    override fun onServiceResolved(i: NsdServiceInfo) {
                        @Suppress("DEPRECATION")
                        i.host?.hostAddress?.substringBefore('%')?.let { found += it }
                        done.countDown()
                    }
                })
            }
            override fun onStartDiscoveryFailed(type: String, code: Int) = done.countDown()
            override fun onStopDiscoveryFailed(type: String, code: Int) = Unit
            override fun onDiscoveryStarted(type: String) = Unit
            override fun onDiscoveryStopped(type: String) = Unit
            override fun onServiceLost(info: NsdServiceInfo) = Unit
        }
        runCatching { nsd.discoverServices(syncServiceType(), NsdManager.PROTOCOL_DNS_SD, listener) }.onFailure { return emptyList() }
        done.await(5, TimeUnit.SECONDS)
        runCatching { nsd.stopServiceDiscovery(listener) }
        return found.toList()
    }

    companion object {
        private const val PERIOD = 60_000L
    }
}
