package org.openslate.android

import android.annotation.SuppressLint
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCallback
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothGattDescriptor
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothProfile
import android.bluetooth.le.ScanCallback
import android.bluetooth.le.ScanResult
import android.bluetooth.le.ScanSettings
import android.content.Context
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.util.Log
import uniffi.openslate_mobile.AdvMode
import uniffi.openslate_mobile.MfrEntry
import uniffi.openslate_mobile.PadInfo
import uniffi.openslate_mobile.PenPoint
import uniffi.openslate_mobile.SessionEvent
import uniffi.openslate_mobile.SlateSession
import uniffi.openslate_mobile.Step
import uniffi.openslate_mobile.classifyAdvert
import uniffi.openslate_mobile.gattUuids
import java.util.ArrayDeque
import java.util.UUID

private const val TAG = "OpenSlate"
private val CCCD: UUID = UUID.fromString("00002902-0000-1000-8000-00805f9b34fb")

/** Cosa mostrare all'utente (i testi sono nelle risorse). */
sealed class LinkStatus {
    object Idle : LinkStatus()
    object Searching : LinkStatus()
    object NotFound : LinkStatus()
    object NotRegistered : LinkStatus()
    data class Connecting(val name: String) : LinkStatus()
    object PressButton : LinkStatus()
    object RegisteredRetry : LinkStatus()
    data class Live(val info: PadInfo) : LinkStatus()
    data class Syncing(val index: Int, val count: Int) : LinkStatus()
    /** Scaricamento finito: `files` nel pad, `deleted` salvate e tolte dal pad. Si è di nuovo in live. */
    data class Synced(val files: Int, val deleted: Int) : LinkStatus()
    data class Failed(val message: String) : LinkStatus()
    object NeedRegister : LinkStatus()
    object Disconnected : LinkStatus()
}

interface LinkListener {
    fun onStatus(status: LinkStatus)
    fun onPen(points: List<PenPoint>)
    /** Una pagina scaricata dal pad: salvarla e dire se è al sicuro (solo allora si toglie dal pad). */
    fun onPageDownloaded(raw: ByteArray, timeBcd: ByteArray, crcOk: Boolean, info: PadInfo): Boolean
}

/**
 * Il Bluetooth con il Bamboo Slate: ricerca, connessione e GATT. Il protocollo
 * lo fa il cuore Rust ([SlateSession]): qui si passano i byte avanti e indietro.
 * Tutto gira sul thread principale (le callback GATT vi vengono rimandate).
 */
@SuppressLint("MissingPermission") // i permessi li chiede MainActivity prima di connect()
class SlateLink(private val context: Context, private val listener: LinkListener) {
    private val main = Handler(Looper.getMainLooper())
    private val prefs = context.getSharedPreferences("slate", Context.MODE_PRIVATE)
    private val bt = context.getSystemService(BluetoothManager::class.java)?.adapter
    private val uuids = gattUuids()
    private val uWrite = UUID.fromString(uuids.nordicWrite)
    private val uNotify = UUID.fromString(uuids.nordicNotify)
    private val uLive = UUID.fromString(uuids.livePenData)
    private val uOffline = UUID.fromString(uuids.offlinePenData)
    private val subscriptions = listOf(uNotify, uLive, uOffline, UUID.fromString(uuids.sysevent))
    /** Ultime informazioni lette dal pad (dimensioni, batteria). */
    var info: PadInfo? = null
        private set

    private var gatt: BluetoothGatt? = null
    private var session: SlateSession? = null
    private var writeChar: BluetoothGattCharacteristic? = null
    private val writes = ArrayDeque<ByteArray>()
    private var writing = false
    private val toSubscribe = ArrayDeque<BluetoothGattCharacteristic>()
    private var registering = false
    /** In questa connessione il pulsante è stato premuto (identità nuova salvata). */
    private var registeredNow = false
    /** Generazione del timer di risposta: un timer vecchio non scatta più. */
    private var timerGen = 0
    private var scanning: ScanCallback? = null
    /** Chiusura voluta da noi: la sua callback non mostra errori. */
    private var quietClose = false

    val bluetoothOn: Boolean get() = bt?.isEnabled == true
    val busy: Boolean get() = gatt != null || scanning != null

    // ---------------------------------------------------------------- identità

    /** UUID con cui l'app è registrata sul pad (come `app_uuid.bin` della desktop). */
    private var savedUuid: ByteArray?
        get() = prefs.getString("uuid", null)?.let(::unhex)?.takeIf { it.size == 6 }
        set(v) { prefs.edit().putString("uuid", v?.let(::hex)).apply() }

    /** Indirizzo del pad usato l'ultima volta (come `peripheral_id`). */
    private var savedAddress: String?
        get() = prefs.getString("address", null)
        set(v) { prefs.edit().putString("address", v).apply() }

    // ---------------------------------------------------------------- ricerca

    /** Cerca il pad come `Pad::find`: al massimo 10 s, almeno 1,5 s. */
    fun connect() {
        if (busy) return
        val scanner = bt?.bluetoothLeScanner ?: return listener.onStatus(LinkStatus.Failed("Bluetooth"))
        listener.onStatus(LinkStatus.Searching)
        val start = SystemClock.elapsedRealtime()
        var best: Pair<ScanResult, AdvMode>? = null
        var bestHasMfr = false
        val cb = object : ScanCallback() {
            override fun onScanResult(callbackType: Int, result: ScanResult) {
                val rec = result.scanRecord
                val mfr = mutableListOf<MfrEntry>()
                rec?.manufacturerSpecificData?.let { m ->
                    for (i in 0 until m.size()) mfr += MfrEntry(m.keyAt(i).toUShort(), m.valueAt(i).size.toUInt())
                }
                val name = rec?.deviceName ?: result.device.name ?: ""
                val services = rec?.serviceUuids?.map { it.uuid.toString() } ?: emptyList()
                val adv = classifyAdvert(name, mfr, services, result.device.address == savedAddress)
                if (adv.candidate && (best == null || mfr.isNotEmpty())) {
                    best = result to adv.mode
                    bestHasMfr = mfr.isNotEmpty()
                }
                if (bestHasMfr && SystemClock.elapsedRealtime() - start >= 1500) finishScan()
            }

            override fun onScanFailed(errorCode: Int) {
                main.post {
                    scanning = null
                    listener.onStatus(LinkStatus.Failed("scan $errorCode"))
                }
            }

            fun finishScan() {
                main.post {
                    if (scanning !== this) return@post
                    scanner.stopScan(this)
                    scanning = null
                    val found = best
                    if (found == null) listener.onStatus(LinkStatus.NotFound)
                    else open(found.first.device, found.second)
                }
            }
        }
        scanning = cb
        val settings = ScanSettings.Builder().setScanMode(ScanSettings.SCAN_MODE_LOW_LATENCY).build()
        scanner.startScan(null, settings, cb)
        main.postDelayed({ if (scanning === cb) cb.finishScan() }, 10_000)
    }

    // ---------------------------------------------------------------- connessione

    private fun open(device: BluetoothDevice, mode: AdvMode) {
        val uuid = savedUuid
        Log.i(TAG, "Pad ${device.address} modo $mode, identità salvata: ${uuid != null}")
        if (mode == AdvMode.NORMAL && uuid == null) {
            listener.onStatus(LinkStatus.NotRegistered)
            return
        }
        registering = mode == AdvMode.REGISTER || uuid == null
        savedAddress = device.address
        listener.onStatus(LinkStatus.Connecting(device.name ?: device.address))
        gatt = device.connectGatt(context, false, callback, BluetoothDevice.TRANSPORT_LE)
    }

    /**
     * Ferma il live (SET_MODE IDLE, come Tuhi) e chiude il collegamento.
     * `report = false` quando il motivo è già stato mostrato.
     */
    fun disconnect(report: Boolean = true) {
        scanning?.let { bt?.bluetoothLeScanner?.stopScan(it) }
        scanning = null
        if (report) listener.onStatus(LinkStatus.Idle)
        val g = gatt ?: return
        quietClose = true
        val s = session
        if (s != null && s.isLive()) {
            enqueue(listOf(s.stopFrame()))
            main.postDelayed({ g.disconnect() }, 300)
        } else {
            g.disconnect()
        }
        // Se il pad è già spento la callback può non arrivare mai.
        main.postDelayed({ if (gatt === g) closed() }, 3_000)
    }

    private fun closed() {
        quietClose = false
        gatt?.close()
        gatt = null
        session?.destroy()
        session = null
        writeChar = null
        writes.clear()
        writing = false
        toSubscribe.clear()
        timerGen++
    }

    private val callback = object : BluetoothGattCallback() {
        override fun onConnectionStateChange(g: BluetoothGatt, status: Int, newState: Int) {
            main.post {
                if (g !== gatt) return@post
                when (newState) {
                    BluetoothProfile.STATE_CONNECTED -> g.requestMtu(247)
                    BluetoothProfile.STATE_DISCONNECTED -> {
                        val wasLive = session?.isLive() == true
                        val quiet = quietClose
                        closed()
                        if (!quiet) {
                            listener.onStatus(if (wasLive) LinkStatus.Disconnected else LinkStatus.Failed("GATT $status"))
                        }
                    }
                }
            }
        }

        override fun onMtuChanged(g: BluetoothGatt, mtu: Int, status: Int) {
            main.post { if (g === gatt) g.discoverServices() }
        }

        override fun onServicesDiscovered(g: BluetoothGatt, status: Int) {
            main.post {
                if (g !== gatt) return@post
                val chars = g.services.flatMap { it.characteristics }
                writeChar = chars.firstOrNull { it.uuid == uWrite }
                if (writeChar == null || chars.none { it.uuid == uNotify }) {
                    listener.onStatus(LinkStatus.Failed("Nordic UART"))
                    disconnect(report = false)
                    return@post
                }
                toSubscribe.clear()
                subscriptions.forEach { u -> chars.firstOrNull { it.uuid == u }?.let(toSubscribe::add) }
                subscribeNext()
            }
        }

        override fun onDescriptorWrite(g: BluetoothGatt, d: BluetoothGattDescriptor, status: Int) {
            main.post { if (g === gatt) subscribeNext() }
        }

        override fun onCharacteristicWrite(g: BluetoothGatt, c: BluetoothGattCharacteristic, status: Int) {
            main.post {
                if (g !== gatt) return@post
                writing = false
                writeNext()
            }
        }

        override fun onCharacteristicChanged(g: BluetoothGatt, c: BluetoothGattCharacteristic, value: ByteArray) {
            val copy = value.copyOf()
            main.post { if (g === gatt) received(c.uuid, copy) }
        }

        @Deprecated("Android 12 e precedenti")
        override fun onCharacteristicChanged(g: BluetoothGatt, c: BluetoothGattCharacteristic) {
            @Suppress("DEPRECATION") val copy = c.value?.copyOf() ?: return
            main.post { if (g === gatt) received(c.uuid, copy) }
        }
    }

    /** Le notifiche su Android si accendono una alla volta (CCCD). */
    private fun subscribeNext() {
        val g = gatt ?: return
        val c = toSubscribe.pollFirst() ?: return startSession()
        g.setCharacteristicNotification(c, true)
        val d = c.getDescriptor(CCCD) ?: return subscribeNext()
        val on = BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE
        if (Build.VERSION.SDK_INT >= 33) {
            g.writeDescriptor(d, on)
        } else {
            @Suppress("DEPRECATION")
            d.value = on
            @Suppress("DEPRECATION")
            g.writeDescriptor(d)
        }
    }

    private fun startSession() {
        val s = SlateSession()
        session = s
        registeredNow = false
        val uuid = savedUuid
        if (registering || uuid == null) {
            listener.onStatus(LinkStatus.PressButton)
            apply(s.beginRegister())
        } else {
            apply(s.beginLive(uuid))
        }
    }

    /** Si scrive in live (non durante uno scaricamento). */
    val live: Boolean get() = session?.let { it.isLive() && !it.isSyncing() } == true

    /** Scarica le pagine dalla memoria del pad (poi torna il live). */
    fun sync() {
        val s = session ?: return
        val uuid = savedUuid ?: return
        if (!s.isLive()) return
        listener.onStatus(LinkStatus.Syncing(0, 0))
        apply(s.beginSync(uuid))
    }

    // ---------------------------------------------------------------- dati

    private fun received(uuid: UUID, bytes: ByteArray) {
        val s = session ?: return
        when (uuid) {
            uNotify -> apply(s.onNordic(bytes))
            uLive -> s.onLive(bytes).takeIf { it.isNotEmpty() }?.let(listener::onPen)
            uOffline -> s.onOffline(bytes)
            // SYSEVENT: non serve.
        }
    }

    /** Esegue un passo della sessione: scritture, eventi, timer di risposta. */
    private fun apply(step: Step) {
        enqueue(step.writes)
        for (e in step.events) {
            when (e) {
                is SessionEvent.Registered -> {
                    savedUuid = e.uuid
                    registeredNow = true
                }
                is SessionEvent.Live -> {
                    info = e.info
                    listener.onStatus(LinkStatus.Live(e.info))
                }
                is SessionEvent.SyncProgress -> listener.onStatus(LinkStatus.Syncing(e.index.toInt(), e.count.toInt()))
                is SessionEvent.Downloaded -> {
                    val pad = info ?: PadInfo("", "", null, false, 0f, 0f)
                    val saved = runCatching { listener.onPageDownloaded(e.raw, e.timeBcd, e.crcOk, pad) }
                        .onFailure { Log.e(TAG, "salvataggio pagina", it) }
                        .getOrDefault(false)
                    session?.let { apply(it.confirmSaved(saved)) }
                }
                is SessionEvent.SyncDone -> listener.onStatus(LinkStatus.Synced(e.files.toInt(), e.deleted.toInt()))
                is SessionEvent.Failed -> {
                    listener.onStatus(
                        when {
                            e.needRegister -> LinkStatus.NeedRegister
                            registeredNow -> LinkStatus.RegisteredRetry
                            else -> LinkStatus.Failed(e.message)
                        }
                    )
                    disconnect(report = false)
                }
            }
        }
        if (step.events.any { it is SessionEvent.Live || it is SessionEvent.Failed || it is SessionEvent.SyncDone }) timerGen++
        if (step.waitMs > 0u) {
            val gen = ++timerGen
            main.postDelayed({
                if (gen == timerGen) session?.let { apply(it.onTimeout()) }
            }, step.waitMs.toLong())
        }
    }

    private fun enqueue(frames: List<ByteArray>) {
        writes.addAll(frames)
        writeNext()
    }

    /** Una scrittura GATT alla volta (Android scarta quelle sovrapposte). */
    private fun writeNext() {
        if (writing) return
        val g = gatt ?: return
        val c = writeChar ?: return
        val f = writes.peekFirst() ?: return
        Log.i(TAG, "TX ${hex(f)}")
        val type = BluetoothGattCharacteristic.WRITE_TYPE_NO_RESPONSE
        val ok = if (Build.VERSION.SDK_INT >= 33) {
            g.writeCharacteristic(c, f, type) == BluetoothGatt.GATT_SUCCESS
        } else {
            @Suppress("DEPRECATION")
            c.writeType = type
            @Suppress("DEPRECATION")
            c.value = f
            @Suppress("DEPRECATION")
            g.writeCharacteristic(c)
        }
        if (ok) {
            writes.pollFirst()
            writing = true
        } else {
            main.postDelayed(::writeNext, 50) // GATT occupato: si riprova
        }
    }
}

private fun hex(b: ByteArray) = b.joinToString("") { "%02x".format(it) }
private fun unhex(s: String) = runCatching { s.chunked(2).map { it.toInt(16).toByte() }.toByteArray() }.getOrNull()
