// Lingue delle pagine (chat, appunti, impostazioni AI, diretta web).
// Il server mette prima di questo file `window.I18N_LANG` e `window.I18N`
// (la tabella italiano → lingua scelta, la stessa dell'app). Il testo nelle
// pagine è in italiano e fa da chiave: qui si traducono i testi e gli
// attributi (title, placeholder) della pagina, anche quelli aggiunti dopo,
// e `T("…")` serve ai testi che non finiscono nella pagina (prompt, valori).
// Le chiavi con segnaposto ("La pagina «{}» è allegata") traducono anche i
// testi composti. Non si toccano i contenuti: messaggi, appunti, codice.
(function () {
  const L = window.I18N || {};
  const lang = window.I18N_LANG || "it";
  document.documentElement.lang = lang;

  // --- modelli con segnaposto, come in src/i18n.rs ---
  function split(s) {
    const fixed = [""], holes = [];
    for (let i = 0; i < s.length; i++) {
      const c = s[i];
      if (c === "{" && s[i + 1] === "{") { fixed[fixed.length - 1] += "{"; i++; }
      else if (c === "}" && s[i + 1] === "}") { fixed[fixed.length - 1] += "}"; i++; }
      else if (c === "{") {
        const j = s.indexOf("}", i);
        if (j < 0) { fixed[fixed.length - 1] += s.slice(i); break; }
        holes.push(s.slice(i + 1, j)); fixed.push(""); i = j;
      } else fixed[fixed.length - 1] += c;
    }
    return { fixed, holes };
  }
  function compile(key) {
    if (!key.includes("{")) return null;
    const p = split(key);
    if (!p.holes.length) return null;
    const letters = p.fixed.join("").replace(/[^\p{L}]/gu, "").length;
    if (letters < 3) return null;
    p.len = p.fixed.join("").length;
    return p;
  }
  function captures(p, s) {
    const first = p.fixed[0], last = p.fixed[p.fixed.length - 1];
    if (!s.startsWith(first) || !s.endsWith(last) || s.length < first.length + last.length) return null;
    let rest = s.slice(first.length, s.length - last.length);
    const caps = [];
    for (const mid of p.fixed.slice(1, -1)) {
      const i = mid ? rest.indexOf(mid) : 0;
      if (i < 0) return null;
      caps.push(rest.slice(0, i)); rest = rest.slice(i + mid.length);
    }
    caps.push(rest);
    return caps;
  }
  function apply(p, s, to, depth) {
    let caps = captures(p, s);
    if (!caps) return null;
    // anche i pezzi variabili, se sono testi noti
    caps = caps.map((c) => (/\p{L}/u.test(c) && c.trim() ? (L[c] || (depth > 0 ? byPattern(c, depth - 1) : null) || c) : c));
    const t = split(to), used = caps.map(() => false);
    let out = "";
    t.fixed.forEach((piece, k) => {
      out += piece;
      if (k < t.holes.length) {
        const name = t.holes[k];
        let j = name ? p.holes.findIndex((h, i) => h === name && !used[i]) : -1;
        if (j < 0 && name) j = p.holes.indexOf(name);
        if (j < 0) j = used.findIndex((u) => !u);
        if (j >= 0) { used[j] = true; out += caps[j]; }
      }
    });
    return out;
  }
  const pats = [];
  for (const k in L) {
    if (!L[k]) continue;
    const p = compile(k);
    if (p) pats.push([p, L[k]]);
  }
  pats.sort((a, b) => b[0].len - a[0].len);
  function byPattern(s, depth) { for (const [p, v] of pats) { const r = apply(p, s, v, depth); if (r !== null) return r; } return null; }
  const one = (s) => L[s] || byPattern(s, 2);
  function compound(s) {
    if (s.startsWith("⚠ ")) return "⚠ " + (one(s.slice(2)) ?? compound(s.slice(2)) ?? s.slice(2));
    if (!s.includes(": ")) return null;
    let changed = false;
    const out = s.split(": ").map((x) => { const t = one(x); if (t !== null) { changed = true; return t; } return x; });
    return changed ? out.join(": ") : null;
  }
  const cache = new Map();
  // Qualsiasi testo: tradotto se la tabella lo conosce, altrimenti com'è.
  function tr(s) {
    if (lang === "it" || !s) return s;
    if (L[s]) return L[s];
    if (cache.has(s)) return cache.get(s);
    const r = one(s) ?? compound(s) ?? s;
    if (cache.size > 3000) cache.clear();
    cache.set(s, r);
    return r;
  }
  // T("testo con {} e {}", a, b): il modello tradotto con i valori al posto dei {}.
  function T(s, ...args) {
    let t = lang === "it" ? s : (L[s] || s);
    if (args.length) { let i = 0; t = t.replace(/\{[^}]*\}/g, () => String(args[i++] ?? "")); }
    return t;
  }
  window.tr = tr;
  window.T = T;
  window.I18N_DATE = { it: "it-IT", en: "en-GB", es: "es-ES", fr: "fr-FR", de: "de-DE", uk: "uk-UA", zh: "zh-CN", ja: "ja-JP", ko: "ko-KR" }[lang] || "it-IT";
  if (lang === "it") return;

  // --- la pagina: testi e attributi, anche quelli che arrivano dopo ---
  // contenuti (messaggi della chat, testo degli appunti): non si traducono
  const SKIP = "script,style,textarea,code,pre,.katex,[data-noi18n],.msg .body,#blocks";
  const ATTRS = ["title", "placeholder", "aria-label"];
  const skipped = (el) => !!(el && el.closest && el.closest(SKIP));
  // gli attributi (placeholder, title) si traducono anche su textarea e input
  const CONTENT = "[data-noi18n],.msg .body,#blocks";
  const skippedAttr = (el) => !!(el && el.closest && el.closest(CONTENT));
  function doText(n) {
    const v = n.nodeValue, t = v.trim();
    if (!t || !/\p{L}/u.test(t)) return;
    // il testo scritto su più righe nell'HTML: spazi e a capo come uno spazio
    const x = tr(t.replace(/\s+/g, " "));
    if (x !== t.replace(/\s+/g, " ")) n.nodeValue = v.replace(t, x);
  }
  function doEl(el) {
    for (const a of ATTRS) {
      const v = el.getAttribute(a);
      if (v) { const x = tr(v); if (x !== v) el.setAttribute(a, x); }
    }
  }
  function walk(root) {
    if (root.nodeType === 3) { if (!skipped(root.parentElement)) doText(root); return; }
    if (root.nodeType !== 1) return;
    if (skipped(root)) { if (!skippedAttr(root)) { doEl(root); root.querySelectorAll && root.querySelectorAll("[title],[placeholder],[aria-label]").forEach((e) => { if (!skippedAttr(e)) doEl(e); }); } return; }
    doEl(root);
    const w = document.createTreeWalker(root, NodeFilter.SHOW_ELEMENT | NodeFilter.SHOW_TEXT, {
      acceptNode: (n) => {
        if (n.nodeType === 1 && n.matches(SKIP)) { if (!skippedAttr(n)) doEl(n); return NodeFilter.FILTER_REJECT; }
        return NodeFilter.FILTER_ACCEPT;
      },
    });
    let n;
    while ((n = w.nextNode())) { if (n.nodeType === 3) doText(n); else doEl(n); }
  }
  function start() {
    document.title = tr(document.title);
    walk(document.body);
    new MutationObserver((ms) => {
      for (const m of ms) {
        if (m.type === "characterData") { if (!skipped(m.target.parentElement)) doText(m.target); }
        else if (m.type === "attributes") { if (!skippedAttr(m.target)) doEl(m.target); }
        else m.addedNodes.forEach(walk);
      }
    }).observe(document.body, { subtree: true, childList: true, characterData: true, attributes: true, attributeFilter: ATTRS });
  }
  if (document.body) start(); else document.addEventListener("DOMContentLoaded", start);
})();
