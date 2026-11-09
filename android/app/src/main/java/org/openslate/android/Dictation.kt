package org.openslate.android

import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.Bundle
import android.speech.RecognitionListener
import android.speech.RecognizerIntent
import android.speech.SpeechRecognizer
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import java.util.Locale

/**
 * La dettatura della chat: il riconoscimento vocale del sistema, sul telefono
 * quando c'è (come Whisper sul computer nella desktop: la voce non esce).
 * Mentre parli il testo compare nella casella; tocchi di nuovo il microfono
 * (o stai zitto) e si ferma.
 */
class Dictation(private val ctx: Context, private val onText: (partial: String, final: Boolean) -> Unit) {
    var listening by mutableStateOf(false)
        private set
    /** Volume della voce, 0..1, per far «respirare» il microfono. */
    var level by mutableFloatStateOf(0f)
        private set
    var error by mutableStateOf<String?>(null)

    private var rec: SpeechRecognizer? = null

    fun available(): Boolean = SpeechRecognizer.isRecognitionAvailable(ctx) || onDevice()

    /** Senza un servizio da usare dentro l'app: la finestra di dettatura del sistema. */
    fun systemIntent(): Intent = Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH).apply {
        putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM)
        putExtra(RecognizerIntent.EXTRA_LANGUAGE, Locale.getDefault().toLanguageTag())
        putExtra(RecognizerIntent.EXTRA_PREFER_OFFLINE, true)
    }

    fun result(data: Intent?): String? =
        data?.getStringArrayListExtra(RecognizerIntent.EXTRA_RESULTS)?.firstOrNull()?.takeIf { it.isNotBlank() }

    private fun onDevice(): Boolean =
        Build.VERSION.SDK_INT >= 31 && SpeechRecognizer.isOnDeviceRecognitionAvailable(ctx)

    fun start() = start(onDevice())

    private fun start(local: Boolean) {
        if (listening && rec != null) return
        error = null
        val r = if (local) SpeechRecognizer.createOnDeviceSpeechRecognizer(ctx) else SpeechRecognizer.createSpeechRecognizer(ctx)
        rec = r
        r.setRecognitionListener(object : RecognitionListener {
            override fun onReadyForSpeech(params: Bundle?) { listening = true }
            override fun onBeginningOfSpeech() {}
            override fun onRmsChanged(rmsdB: Float) { level = ((rmsdB + 2f) / 12f).coerceIn(0f, 1f) }
            override fun onBufferReceived(buffer: ByteArray?) {}
            override fun onEndOfSpeech() {}
            override fun onError(code: Int) {
                // sul telefono manca la lingua (pacchetto non scaricato): si riprova col
                // riconoscimento normale del sistema, che comunque preferisce il telefono
                if (local && code in LANGUAGE_ERRORS) {
                    finish()
                    start(false)
                    return
                }
                // niente parlato: si chiude senza far rumore
                if (code != SpeechRecognizer.ERROR_NO_MATCH && code != SpeechRecognizer.ERROR_SPEECH_TIMEOUT && code != SpeechRecognizer.ERROR_CLIENT) {
                    error = ctx.getString(
                        if (code == SpeechRecognizer.ERROR_INSUFFICIENT_PERMISSIONS) R.string.dictation_no_permission
                        else if (code == SpeechRecognizer.ERROR_NETWORK || code == SpeechRecognizer.ERROR_NETWORK_TIMEOUT) R.string.dictation_network
                        else R.string.dictation_failed
                    )
                }
                finish()
            }
            override fun onResults(results: Bundle?) {
                first(results)?.let { onText(it, true) }
                finish()
            }
            override fun onPartialResults(partial: Bundle?) { first(partial)?.let { onText(it, false) } }
            override fun onEvent(eventType: Int, params: Bundle?) {}
        })
        val intent = Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH).apply {
            putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM)
            putExtra(RecognizerIntent.EXTRA_LANGUAGE, Locale.getDefault().toLanguageTag())
            putExtra(RecognizerIntent.EXTRA_PARTIAL_RESULTS, true)
            putExtra(RecognizerIntent.EXTRA_PREFER_OFFLINE, true)
            // una frase lunga con le pause per pensare non si interrompe subito
            putExtra(RecognizerIntent.EXTRA_SPEECH_INPUT_COMPLETE_SILENCE_LENGTH_MILLIS, 2500L)
            putExtra(RecognizerIntent.EXTRA_SPEECH_INPUT_POSSIBLY_COMPLETE_SILENCE_LENGTH_MILLIS, 2500L)
        }
        listening = true
        r.startListening(intent)
    }

    /** Ferma l'ascolto: quello che hai detto finora resta nella casella. */
    fun stop() {
        rec?.stopListening()
    }

    private fun finish() {
        listening = false
        level = 0f
        rec?.destroy()
        rec = null
    }

    fun release() = finish()

    private companion object {
        /** ERROR_SERVER_DISCONNECTED, ERROR_LANGUAGE_NOT_SUPPORTED, ERROR_LANGUAGE_UNAVAILABLE. */
        val LANGUAGE_ERRORS = setOf(11, 12, 13)
    }

    private fun first(b: Bundle?): String? =
        b?.getStringArrayList(SpeechRecognizer.RESULTS_RECOGNITION)?.firstOrNull()?.takeIf { it.isNotBlank() }
}
