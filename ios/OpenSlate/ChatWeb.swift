import SwiftUI
import WebKit

/// La risposta ha Markdown o formule (altrimenti basta il testo semplice).
func looksRich(_ text: String) -> Bool {
    text.range(of: #"[$*_`#|\\]|(?m)^\s*([-+]|\d+\.)\s|\[[^\]]+\]\("#, options: .regularExpression) != nil
}

/// Una risposta della chat col Markdown e le formule, come nella chat della
/// desktop: msg.html (da mobile/web) con KaTeX, marked e DOMPurify (vendor/,
/// da src/web/vendor). L'altezza la dice la pagina; i link si aprono in Safari.
struct RichText: View {
    let text: String
    let settled: Bool
    @State private var height: CGFloat = 0

    var body: some View {
        ZStack(alignment: .topLeading) {
            if height == 0 {
                // finché la pagina non è pronta, il testo com'è (niente buco)
                Text(text.isEmpty ? "…" : text)
            }
            MsgWeb(text: text, settled: settled, page: false, height: $height)
                .frame(height: max(height, 1))
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .contextMenu {
            Button { UIPasteboard.general.string = text } label: { Label(L("Copia", "Copy"), systemImage: "doc.on.doc") }
        }
    }
}

/// L'anteprima di una nota: tutto il Markdown con le formule, a pagina
/// intera (scorre la pagina stessa).
struct MarkdownPage: View {
    let text: String
    @State private var height: CGFloat = 0

    var body: some View {
        MsgWeb(text: text, settled: true, page: true, height: $height)
    }
}

private struct MsgWeb: UIViewRepresentable {
    let text: String
    let settled: Bool
    /// Pagina intera: scorre da sé, con un po' di margine.
    let page: Bool
    @Binding var height: CGFloat
    @Environment(\.colorScheme) private var scheme

    func makeCoordinator() -> Coordinator { Coordinator(height: $height) }

    func makeUIView(context: Context) -> WKWebView {
        let cfg = WKWebViewConfiguration()
        cfg.userContentController.add(context.coordinator, name: "height")
        let web = WKWebView(frame: .zero, configuration: cfg)
        web.isOpaque = false
        web.backgroundColor = .clear
        web.scrollView.backgroundColor = .clear
        web.scrollView.isScrollEnabled = page
        web.scrollView.bounces = page
        web.navigationDelegate = context.coordinator
        if let url = Bundle.main.url(forResource: "msg", withExtension: "html") {
            web.loadFileURL(url, allowingReadAccessTo: url.deletingLastPathComponent())
        }
        return web
    }

    func updateUIView(_ web: WKWebView, context: Context) {
        let c = context.coordinator
        let fg = scheme == .dark ? "#f2f2f7" : "#1c1b1f"
        c.pending = (text, settled, fg, page)
        c.flush(web)
    }

    static func dismantleUIView(_ web: WKWebView, coordinator: Coordinator) {
        web.configuration.userContentController.removeScriptMessageHandler(forName: "height")
    }

    final class Coordinator: NSObject, WKScriptMessageHandler, WKNavigationDelegate {
        var height: Binding<CGFloat>
        var ready = false
        var pending: (String, Bool, String, Bool)?
        private var shown: (String, Bool, String, Bool)?

        init(height: Binding<CGFloat>) { self.height = height }

        func flush(_ web: WKWebView) {
            guard ready, let p = pending else { return }
            if let s = shown, s.0 == p.0, s.1 == p.1, s.2 == p.2 { return }
            shown = p
            guard let data = try? JSONSerialization.data(withJSONObject: [p.0], options: [.fragmentsAllowed]),
                  let arr = String(data: data, encoding: .utf8) else { return }
            web.evaluateJavaScript("show(\(arr)[0], \(p.1), {fg: '\(p.2)', link: '#007aff', page: \(p.3)})")
        }

        func userContentController(_ uc: WKUserContentController, didReceive message: WKScriptMessage) {
            if let h = message.body as? NSNumber {
                let v = CGFloat(truncating: h)
                DispatchQueue.main.async { if self.height.wrappedValue != v { self.height.wrappedValue = v } }
            }
        }

        func webView(_ web: WKWebView, didFinish navigation: WKNavigation!) {
            ready = true
            flush(web)
        }

        func webView(_ web: WKWebView, decidePolicyFor action: WKNavigationAction, decisionHandler: @escaping (WKNavigationActionPolicy) -> Void) {
            if action.navigationType == .linkActivated, let url = action.request.url {
                if ["http", "https", "mailto"].contains(url.scheme ?? "") { UIApplication.shared.open(url) }
                decisionHandler(.cancel)
                return
            }
            decisionHandler(action.request.url?.isFileURL == true ? .allow : .cancel)
        }
    }
}
