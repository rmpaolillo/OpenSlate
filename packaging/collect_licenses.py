#!/usr/bin/env python3
"""Testi completi delle licenze di tutto quello che finisce nel programma.

MIT, BSD, Apache, OFL… chiedono che il testo della licenza viaggi con il
programma. Questo script li raccoglie in un solo file di testo, da mettere nei
pacchetti accanto a LICENSE, NOTICE e THIRD-PARTY.md:

- per ogni crate di Cargo.lock (da `cargo metadata`): i file LICENSE*,
  COPYING*, NOTICE*, COPYRIGHT* della sua cartella;
- i file dentro il programma che non sono crate: caratteri, KaTeX, marked,
  DOMPurify, pacchetti Typst (src/typst-pkg), crate copiati in vendor/.

Uso: python3 packaging/collect_licenses.py [file di uscita]
     (di serie build/THIRD-PARTY-LICENSES.txt)
"""

import json
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "build", "THIRD-PARTY-LICENSES.txt")
NAMES = ("license", "licence", "copying", "notice", "copyright", "unlicense")
SEP = "=" * 78


def license_files(folder):
    out = []
    try:
        entries = sorted(os.listdir(folder))
    except OSError:
        return out
    for e in entries:
        low = e.lower()
        p = os.path.join(folder, e)
        if os.path.isfile(p) and low.startswith(NAMES) and not low.endswith((".rs", ".py", ".toml", ".json")):
            out.append(p)
    return out


def read(p):
    with open(p, encoding="utf-8", errors="replace") as f:
        return f.read().strip()


def main():
    meta = json.loads(
        subprocess.run(
            ["cargo", "metadata", "--format-version", "1", "--locked"],
            # in UTF-8 anche su Windows (lì il testo di serie sarebbe cp1252)
            cwd=ROOT, check=True, capture_output=True, text=True, encoding="utf-8", errors="replace",
        ).stdout
    )
    parts = []
    missing = []
    seen = set()
    texts = {}  # testo normalizzato -> (numero, testo): ogni testo uguale una volta sola

    def ref(path):
        t = read(path)
        k = " ".join(t.split())
        if k not in texts:
            texts[k] = (len(texts) + 1, t)
        n = texts[k][0]
        cr = [l.strip() for l in t.splitlines() if "copyright" in l.lower() and any(c.isdigit() for c in l)][:6]
        line = f"{os.path.basename(path)}: full text [T{n}] at the end"
        return line + ("".join("\n  " + c for c in cr))

    spdx_seen = set()
    pkgs = sorted(meta["packages"], key=lambda p: (p["name"], p["version"]))
    for p in pkgs:
        if p["name"] == "openslate":
            continue
        key = (p["name"], p["version"])
        if key in seen:
            continue
        seen.add(key)
        folder = os.path.dirname(p["manifest_path"])
        files = license_files(folder)
        if p.get("license_file"):
            lf = os.path.join(folder, p["license_file"])
            if os.path.isfile(lf) and lf not in files:
                files.insert(0, lf)
        head = f"{p['name']} {p['version']} — {p.get('license') or 'see below'}"
        if p.get("authors"):
            head += "\n" + ", ".join(p["authors"])
        if p.get("repository"):
            head += "\n" + p["repository"]
        if files:
            body = "\n".join(ref(f) for f in files)
        else:
            missing.append(p["name"])
            lic = p.get("license") or "unknown"
            who = ", ".join(p.get("authors") or []) or "the authors"
            body = f"Copyright (c) {who}. Licence: {lic}; the standard texts are at the end (S: {lic})."
            for part in lic.replace("(", " ").replace(")", " ").replace("/", " ").split():
                if part not in ("OR", "AND", "WITH"):
                    spdx_seen.add(part)
        parts.append(f"{SEP}\n{head}\n{SEP}\n{body}\n")

    # file dentro il programma che non vengono da un crate
    extra = [
        ("Inter (assets/fonts)", "assets/fonts/OFL-Inter.txt"),
        ("Source Serif 4 (assets/fonts)", "assets/fonts/OFL-SourceSerif4.txt"),
        ("Noto Sans CJK subset (assets/fonts)", "assets/fonts/OFL-NotoSansCJK.txt"),
        ("KaTeX (src/web/vendor)", "src/web/vendor/LICENSE-katex"),
        ("marked (src/web/vendor)", "src/web/vendor/LICENSE-marked.md"),
        ("DOMPurify (src/web/vendor)", "src/web/vendor/LICENSE-dompurify"),
        ("Hershey fonts (src/handwrite)", "src/handwrite/LICENSE-hershey.txt"),
        ("OpenSlate Corsivo / Stampatello (assets/fonts)", "assets/fonts/LICENSE-OpenSlateHand.txt"),
    ]
    pkg_root = os.path.join(ROOT, "src", "typst-pkg")
    for dirpath, _dirs, files in os.walk(pkg_root):
        for f in sorted(files):
            if f.lower().startswith(NAMES):
                rel = os.path.relpath(os.path.join(dirpath, f), ROOT)
                extra.append((f"Typst package {os.path.relpath(dirpath, pkg_root)}", rel))
    for v in sorted(os.listdir(os.path.join(ROOT, "vendor"))):
        for f in license_files(os.path.join(ROOT, "vendor", v)):
            extra.append((f"vendor/{v}", os.path.relpath(f, ROOT)))
    for title, rel in extra:
        p = os.path.join(ROOT, rel)
        if os.path.isfile(p):
            parts.append(f"{SEP}\n{title}\n{SEP}\n{ref(p)}\n")
        else:
            missing.append(rel)

    # testi standard (per i crate senza file): da un crate che li ha
    standard = []
    want = {
        "MIT": ("license-mit", "license"), "Apache-2.0": ("license-apache", "license-apache-2.0"),
        "BSD-3-Clause": ("license-bsd", "license"), "BSD-2-Clause": ("license-bsd", "license"),
        "ISC": ("license-isc", "license"), "Zlib": ("license-zlib", "license"), "MPL-2.0": ("license-mpl", "license"),
        "Unicode-3.0": ("license-unicode", "license"), "BSL-1.0": ("license-boost", "license"),
        "CC0-1.0": ("license-cc0", "license"), "0BSD": ("license-0bsd", "license"), "Unlicense": ("unlicense", "license"),
    }
    for sp in sorted(spdx_seen):
        found = None
        for p in pkgs:
            lic = p.get("license") or ""
            if lic.strip() != sp:
                continue
            for f in license_files(os.path.dirname(p["manifest_path"])):
                found = f
                break
            if found:
                break
        if not found:
            names = want.get(sp, ())
            for p in pkgs:
                if sp in (p.get("license") or ""):
                    for f in license_files(os.path.dirname(p["manifest_path"])):
                        if os.path.basename(f).lower().split(".")[0] in names[:1]:
                            found = f
                            break
                if found:
                    break
        if found:
            standard.append(f"{SEP}\nStandard text: {sp}\n{SEP}\n{ref(found)}\n")
        else:
            standard.append(f"{SEP}\nStandard text: {sp}\n{SEP}\nhttps://spdx.org/licenses/{sp}.html\n")
    parts.extend(standard)
    appendix = [f"{SEP}\n[T{n}]\n{SEP}\n{t}\n" for n, t in sorted(texts.values())]

    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8") as f:
        f.write(
            "OpenSlate — third-party licences\n\n"
            "OpenSlate is GPL-3.0-or-later (LICENSE). It includes the components below; each\n"
            "keeps its own licence, reproduced here in full as the licences require.\n"
            "A summary table is in THIRD-PARTY.md; attributions are in NOTICE.\n\n"
        )
        f.write("\n".join(parts))
        f.write("\n\n" + SEP + "\nFULL LICENCE TEXTS\n" + SEP + "\n\n")
        f.write("\n".join(appendix))
    print(f"{OUT}: {len(seen)} crates, {len(extra)} other components; without a licence file: {len(missing)}")
    if missing:
        print("  " + ", ".join(missing[:40]) + (" …" if len(missing) > 40 else ""))


if __name__ == "__main__":
    main()
