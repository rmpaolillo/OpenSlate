package org.openslate.android

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import org.json.JSONArray
import org.json.JSONObject
import uniffi.openslate_mobile.AiProvider
import java.security.KeyStore
import java.util.UUID
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/** Un servizio AI nelle impostazioni (la chiave sta a parte, cifrata). */
data class AiService(
    val id: String = UUID.randomUUID().toString(),
    val name: String,
    val url: String,
    val model: String = "",
    val vision: Boolean = true,
    val enabled: Boolean = true,
    val noTemperature: Boolean = false,
    val maxTokens: Int? = null,
    val timeoutS: Int = 180,
)

/** Servizi pronti, come nell'`ai.toml` della desktop. */
object AiPresets {
    val all = listOf(
        AiService(name = "Anthropic Claude", url = "https://api.anthropic.com/v1", model = "claude-sonnet-4-6", noTemperature = true, maxTokens = 8192),
        AiService(name = "Google Gemini", url = "https://generativelanguage.googleapis.com/v1beta/openai", model = "gemini-3.8-flash"),
        AiService(name = "DeepSeek", url = "https://api.deepseek.com/v1", model = "deepseek-flash"),
        AiService(name = "OpenAI", url = "https://api.openai.com/v1", model = ""),
        // sul Mac di casa: l'indirizzo del Mac al posto di IP-DEL-MAC
        AiService(name = "LM Studio (Mac)", url = "http://IP-DEL-MAC:1234/v1", timeoutS = 300),
        AiService(name = "Ollama (Mac)", url = "http://IP-DEL-MAC:11434/v1", timeoutS = 300),
    )
}

/**
 * Le impostazioni AI sul telefono: l'elenco dei servizi (l'ordine è la
 * catena di riserva, come sulla desktop) e le chiavi, cifrate con una chiave
 * dell'Android Keystore che non esce mai dal telefono. Nessun backup: le
 * chiavi non finiscono nel cloud.
 */
class AiStore(context: Context) {
    private val prefs = context.getSharedPreferences("ai", Context.MODE_PRIVATE)

    fun services(): List<AiService> {
        val arr = runCatching { JSONArray(prefs.getString("services", "[]")) }.getOrDefault(JSONArray())
        return (0 until arr.length()).map { i ->
            val o = arr.getJSONObject(i)
            AiService(
                id = o.getString("id"),
                name = o.getString("name"),
                url = o.getString("url"),
                model = o.optString("model"),
                vision = o.optBoolean("vision", true),
                enabled = o.optBoolean("enabled", true),
                noTemperature = o.optBoolean("noTemperature", false),
                maxTokens = if (o.has("maxTokens")) o.getInt("maxTokens") else null,
                timeoutS = o.optInt("timeoutS", 180),
            )
        }
    }

    fun save(list: List<AiService>) {
        val arr = JSONArray()
        for (s in list) {
            arr.put(JSONObject().apply {
                put("id", s.id); put("name", s.name); put("url", s.url); put("model", s.model)
                put("vision", s.vision); put("enabled", s.enabled); put("noTemperature", s.noTemperature)
                s.maxTokens?.let { put("maxTokens", it) }
                put("timeoutS", s.timeoutS)
            })
        }
        prefs.edit().putString("services", arr.toString()).putLong("mtime", System.currentTimeMillis() / 1000).apply()
        // chiavi di servizi tolti: via
        val ids = list.map { it.id }.toSet()
        val e = prefs.edit()
        prefs.all.keys.filter { it.startsWith("key_") && it.removePrefix("key_") !in ids }.forEach { e.remove(it) }
        e.apply()
    }

    /**
     * I servizi presi dal QR della desktop: quelli con lo stesso nome si
     * aggiornano (chiave compresa), gli altri si aggiungono in fondo.
     */
    fun import(list: List<AiProvider>): Int {
        val now = services().toMutableList()
        for (p in list) {
            val i = now.indexOfFirst { it.name == p.name }
            val s = AiService(
                id = if (i >= 0) now[i].id else UUID.randomUUID().toString(),
                name = p.name,
                url = p.url,
                model = p.model,
                vision = p.vision,
                enabled = p.enabled,
                noTemperature = p.noTemperature,
                maxTokens = p.maxTokens?.toInt(),
                timeoutS = p.timeoutS.toInt(),
            )
            if (i >= 0) now[i] = s else now += s
            if (p.key.isNotBlank()) setKey(s.id, p.key)
        }
        save(now)
        return list.size
    }

    fun hasKey(id: String) = prefs.contains("key_$id")

    fun setKey(id: String, key: String) {
        val e = prefs.edit().putLong("mtime", System.currentTimeMillis() / 1000)
        if (key.isBlank()) e.remove("key_$id").apply()
        else e.putString("key_$id", Secrets.encrypt(key.trim())).apply()
    }

    /** Quando sono cambiate l'ultima volta (secondi Unix; 0 = mai). */
    val mtime: Long get() = prefs.getLong("mtime", 0L)

    /**
     * I servizi arrivati dal computer (sincronizzazione): prendono il posto di
     * quelli di qui, nello stesso ordine; le chiavi nel Keystore.
     */
    fun replaceFromSync(list: List<AiProvider>) {
        val now = services()
        val out = list.map { p ->
            AiService(
                id = now.firstOrNull { it.name == p.name }?.id ?: UUID.randomUUID().toString(),
                name = p.name,
                url = p.url,
                model = p.model,
                vision = p.vision,
                enabled = p.enabled,
                noTemperature = p.noTemperature,
                maxTokens = p.maxTokens?.toInt(),
                timeoutS = p.timeoutS.toInt(),
            )
        }
        save(out)
        list.zip(out).forEach { (p, s) -> setKey(s.id, p.key) }
    }

    private fun key(id: String): String = prefs.getString("key_$id", null)?.let(Secrets::decrypt) ?: ""

    /** La catena per il cuore Rust, con le chiavi in chiaro solo per la richiesta. */
    fun providers(): List<AiProvider> = services().map {
        AiProvider(
            name = it.name,
            url = it.url,
            model = it.model,
            key = key(it.id),
            vision = it.vision,
            enabled = it.enabled,
            noTemperature = it.noTemperature,
            maxTokens = it.maxTokens?.toUInt(),
            timeoutS = it.timeoutS.toUInt(),
        )
    }

    fun provider(s: AiService) = providers().first { it.name == s.name && it.url == s.url }
}

/** Cifratura AES-GCM con una chiave dell'Android Keystore. */
internal object Secrets {
    private const val ALIAS = "openslate-ai-keys"

    private fun secretKey(): SecretKey {
        val ks = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        (ks.getKey(ALIAS, null) as? SecretKey)?.let { return it }
        val g = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore")
        g.init(
            KeyGenParameterSpec.Builder(ALIAS, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .build()
        )
        return g.generateKey()
    }

    fun encrypt(plain: String): String {
        val c = Cipher.getInstance("AES/GCM/NoPadding")
        c.init(Cipher.ENCRYPT_MODE, secretKey())
        val b = { a: ByteArray -> Base64.encodeToString(a, Base64.NO_WRAP) }
        return b(c.iv) + ":" + b(c.doFinal(plain.toByteArray()))
    }

    fun decrypt(s: String): String? = runCatching {
        val (iv, ct) = s.split(":").map { Base64.decode(it, Base64.NO_WRAP) }
        val c = Cipher.getInstance("AES/GCM/NoPadding")
        c.init(Cipher.DECRYPT_MODE, secretKey(), GCMParameterSpec(128, iv))
        String(c.doFinal(ct))
    }.getOrNull()
}
