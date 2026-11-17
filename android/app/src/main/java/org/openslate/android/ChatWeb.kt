package org.openslate.android

import android.annotation.SuppressLint
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.webkit.JavascriptInterface
import android.webkit.WebResourceRequest
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.Toast
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import org.json.JSONObject

/** La risposta ha Markdown o formule (altrimenti basta il testo semplice). */
fun looksRich(text: String): Boolean =
    Regex("""[$*_`#|\\]|^\s*([-+]|\d+\.)\s|\[[^\]]+]\(""", RegexOption.MULTILINE).containsMatchIn(text)

private fun css(c: Color) = String.format("#%06X", c.toArgb() and 0xFFFFFF)

/**
 * Una risposta della chat col Markdown e le formule, come nella chat della
 * desktop: assets/chat/msg.html (KaTeX, marked, DOMPurify; vedi il task
 * chatWeb). I tocchi li prende Compose, così la chat scorre anche col dito
 * sulla risposta: un tocco su un link lo apre, il tocco lungo copia il testo.
 */
@SuppressLint("SetJavaScriptEnabled")
@Composable
fun RichText(text: String, settled: Boolean, color: Color, link: Color, modifier: Modifier = Modifier) {
    val ctx = LocalContext.current
    val density = LocalDensity.current
    var height by remember { mutableIntStateOf(0) }
    var ready by remember { mutableStateOf(false) }
    val web = remember {
        WebView(ctx).apply {
            setBackgroundColor(android.graphics.Color.TRANSPARENT)
            settings.javaScriptEnabled = true
            settings.allowFileAccess = false
            settings.allowContentAccess = false
            settings.blockNetworkLoads = true
            isVerticalScrollBarEnabled = false
            isHorizontalScrollBarEnabled = false
            overScrollMode = WebView.OVER_SCROLL_NEVER
            addJavascriptInterface(object {
                @JavascriptInterface
                fun height(h: Int) {
                    post { height = h }
                }
            }, "OpenSlate")
            webViewClient = object : WebViewClient() {
                override fun onPageFinished(view: WebView?, url: String?) {
                    ready = true
                }

                override fun shouldOverrideUrlLoading(view: WebView?, request: WebResourceRequest?): Boolean = true
            }
            loadUrl("file:///android_asset/chat/msg.html")
        }
    }
    DisposableEffect(Unit) { onDispose { web.destroy() } }
    LaunchedEffect(ready, text, settled, color, link) {
        if (ready) web.evaluateJavascript("show(${JSONObject.quote(text)}, $settled, {fg: '${css(color)}', link: '${css(link)}'})", null)
    }
    Box(modifier.fillMaxWidth()) {
        if (height == 0) {
            // finché la pagina non è pronta, il testo com'è (niente buco)
            Text(text.ifEmpty { "…" }, color = color)
        }
        AndroidView({ web }, Modifier.fillMaxWidth().height(maxOf(height, 1).dp))
        Box(
            Modifier.matchParentSize().pointerInput(text) {
                detectTapGestures(
                    onTap = { o ->
                        val x = o.x / density.density
                        val y = o.y / density.density
                        web.evaluateJavascript("linkAt($x, $y)") { r ->
                            val url = runCatching { JSONObject("{\"u\":$r}").getString("u") }.getOrDefault("")
                            if (url.startsWith("http://") || url.startsWith("https://") || url.startsWith("mailto:")) {
                                runCatching { ctx.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url))) }
                            }
                        }
                    },
                    onLongPress = { copy(ctx, text) },
                )
            }
        )
    }
}

private fun copy(ctx: Context, text: String) {
    val cm = ctx.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
    cm.setPrimaryClip(ClipData.newPlainText("OpenSlate", text))
    Toast.makeText(ctx, ctx.getString(R.string.reply_copied), Toast.LENGTH_SHORT).show()
}

/**
 * L'anteprima di una nota: tutto il Markdown con le formule, a pagina intera
 * (scorre la pagina stessa). I link si aprono fuori dall'app.
 */
@SuppressLint("SetJavaScriptEnabled")
@Composable
fun MarkdownPage(text: String, color: Color, link: Color, modifier: Modifier = Modifier) {
    val ctx = LocalContext.current
    var ready by remember { mutableStateOf(false) }
    val web = remember {
        WebView(ctx).apply {
            setBackgroundColor(android.graphics.Color.TRANSPARENT)
            settings.javaScriptEnabled = true
            settings.allowFileAccess = false
            settings.allowContentAccess = false
            settings.blockNetworkLoads = true
            webViewClient = object : WebViewClient() {
                override fun onPageFinished(view: WebView?, url: String?) {
                    ready = true
                }

                override fun shouldOverrideUrlLoading(view: WebView?, request: WebResourceRequest?): Boolean {
                    val url = request?.url?.toString().orEmpty()
                    if (url.startsWith("http://") || url.startsWith("https://") || url.startsWith("mailto:")) {
                        runCatching { ctx.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url))) }
                    }
                    return true
                }
            }
            loadUrl("file:///android_asset/chat/msg.html")
        }
    }
    DisposableEffect(Unit) { onDispose { web.destroy() } }
    LaunchedEffect(ready, text, color, link) {
        if (ready) web.evaluateJavascript("show(${JSONObject.quote(text)}, true, {fg: '${css(color)}', link: '${css(link)}', page: true})", null)
    }
    AndroidView({ web }, modifier.fillMaxSize())
}
