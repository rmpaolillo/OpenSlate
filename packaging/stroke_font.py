#!/usr/bin/env python3
"""Un carattere a filo singolo da un carattere «a mano» normale.

I caratteri a mano dei siti (Caveat, Kalam…) sono contorni pieni; la penna
dell'app scrive invece linee (tratti veri, che si cancellano e si
esportano come quelli del pennino). Qui ogni lettera si disegna grande, si
riduce alla sua linea di mezzo (scheletro), si ripuliscono i rametti corti e
si salvano le linee nelle unità di Hershey usate da src/handwrite.rs
(maiuscole alte 21, riga di base a y = 9, y verso il basso).

    python3 packaging/stroke_font.py <carattere.ttf|.woff> src/handwrite/<nome>.txt

Serve: fontTools, Pillow, numpy, scikit-image. Il file prodotto va nel
repository (l'app non fa niente di tutto questo: legge solo le linee).
Formato: una riga per carattere, «U+0041<TAB>sinistra<TAB>destra<TAB>tratti»,
tratti separati da «|», punti «x,y» separati da spazi.
"""
import io
import math
import sys

import numpy as np
from fontTools.ttLib import TTFont
from PIL import Image, ImageDraw, ImageFont
from skimage.measure import label
from skimage.morphology import skeletonize

PX = 260  # grandezza a cui si disegna una lettera (pixel)
CAP, BASE = 21.0, 9.0  # unità di Hershey


def load(path):
    f = TTFont(path)
    f.flavor = None
    buf = io.BytesIO()
    f.save(buf)
    buf.seek(0)
    return ImageFont.truetype(buf, PX), f


def raster(font, ch):
    w = int(font.getlength(ch)) + PX
    h = PX * 2
    img = Image.new("L", (w, h), 0)
    d = ImageDraw.Draw(img)
    ox, base = PX // 2, int(PX * 1.25)
    d.text((ox, base), ch, font=font, fill=255, anchor="ls")
    return np.array(img) > 127, ox, base


N8 = [(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)]


def paths(sk):
    """Lo scheletro in linee: da un nodo (capo o incrocio) all'altro, e gli anelli."""
    pts = set(zip(*np.nonzero(sk)))

    def nb(p):
        return [(p[0] + dy, p[1] + dx) for dy, dx in N8 if (p[0] + dy, p[1] + dx) in pts]

    deg = {p: len(nb(p)) for p in pts}
    nodes = {p for p, d in deg.items() if d != 2}
    seen_edges = set()
    out = []
    for n in nodes:
        for m in nb(n):
            if (n, m) in seen_edges:
                continue
            path = [n, m]
            seen_edges.add((n, m))
            seen_edges.add((m, n))
            prev, cur = n, m
            while cur not in nodes:
                nxt = [q for q in nb(cur) if q != prev and (cur, q) not in seen_edges]
                if not nxt:
                    break
                q = nxt[0]
                seen_edges.add((cur, q))
                seen_edges.add((q, cur))
                path.append(q)
                prev, cur = cur, q
            out.append(path)
    # anelli senza nodi (una «o» chiusa)
    used = {p for path in out for p in path}
    rest = pts - used
    while rest:
        start = next(iter(rest))
        path = [start]
        prev, cur = None, start
        while True:
            nxt = [q for q in nb(cur) if q != prev and q in rest and q not in path]
            if not nxt:
                break
            prev, cur = cur, nxt[0]
            path.append(cur)
        if len(path) > 3:
            path.append(start)
            out.append(path)
        rest -= set(path)
    return out, nodes


def length(p):
    return sum(math.dist(a, b) for a, b in zip(p, p[1:]))


def prune(ps, nodes, minlen):
    """Toglie i rametti corti che lo scheletro fa negli angoli e nelle punte."""
    deg = {}
    for p in ps:
        for e in (p[0], p[-1]):
            deg[e] = deg.get(e, 0) + 1
    keep = []
    for p in ps:
        a, b = p[0], p[-1]
        spur = (deg.get(a, 0) == 1) != (deg.get(b, 0) == 1)
        if spur and length(p) < minlen:
            continue
        keep.append(p)
    return keep


def join(ps, max_turn=50):
    """Unisce le linee che si continuano dritte a un incrocio (meno pezzi, più tratti)."""
    def direction(p, at_end):
        q = p[-1] if at_end else p[0]
        r = p[-min(6, len(p))] if at_end else p[min(5, len(p) - 1)]
        return math.atan2(q[0] - r[0], q[1] - r[1])

    changed = True
    while changed:
        changed = False
        for i in range(len(ps)):
            for j in range(len(ps)):
                if i == j or ps[i][0] == ps[i][-1] or ps[j][0] == ps[j][-1]:
                    continue
                a, b = ps[i], ps[j]
                for ra in (False, True):
                    for rb in (False, True):
                        aa = a[::-1] if ra else a
                        bb = b[::-1] if rb else b
                        if aa[-1] != bb[0]:
                            continue
                        d1 = direction(aa, True)
                        d2 = math.atan2(bb[min(5, len(bb) - 1)][0] - bb[0][0], bb[min(5, len(bb) - 1)][1] - bb[0][1])
                        turn = abs((d2 - d1 + math.pi) % (2 * math.pi) - math.pi)
                        if math.degrees(turn) < max_turn:
                            ps[i] = aa + bb[1:]
                            del ps[j]
                            changed = True
                            break
                    if changed:
                        break
                if changed:
                    break
            if changed:
                break
    return ps


def smooth(p, k=3):
    if len(p) < 5:
        return p
    closed = p[0] == p[-1]
    out = []
    n = len(p)
    for i in range(n):
        if not closed and (i == 0 or i == n - 1):
            out.append(p[i])
            continue
        idx = [(i + d) % (n - 1) if closed else min(max(i + d, 0), n - 1) for d in range(-k, k + 1)]
        out.append((sum(p[j][0] for j in idx) / len(idx), sum(p[j][1] for j in idx) / len(idx)))
    if closed:
        out[-1] = out[0]
    return out


def rdp(p, eps):
    if len(p) < 3:
        return p
    a, b = p[0], p[-1]

    def dist(q):
        dx, dy = b[1] - a[1], b[0] - a[0]
        l2 = dx * dx + dy * dy
        if l2 == 0:
            return math.dist(q, a)
        t = max(0, min(1, ((q[1] - a[1]) * dx + (q[0] - a[0]) * dy) / l2))
        return math.dist(q, (a[0] + t * dy, a[1] + t * dx))

    k, d = max(((i, dist(q)) for i, q in enumerate(p[1:-1], 1)), key=lambda x: x[1])
    if d <= eps:
        return [a, b]
    return rdp(p[: k + 1], eps)[:-1] + rdp(p[k:], eps)


def main(src, dst):
    font, tt = load(src)
    # altezza delle maiuscole (dalla «H»)
    img, ox, base = raster(font, "H")
    ys = np.nonzero(img)[0]
    cap_px = base - ys.min()
    k = CAP / cap_px
    lines = []
    for code in range(33, 127):
        ch = chr(code)
        if tt.getBestCmap().get(code) is None:
            continue
        img, ox, base = raster(font, ch)
        if not img.any():
            continue
        # pezzo per pezzo: i puntini (della i, dei due punti) sono pezzi a sé
        lab = label(img, connectivity=2)
        ps = []
        dots = []
        for n in range(1, lab.max() + 1):
            comp = lab == n
            yy, xx = np.nonzero(comp)
            if max(np.ptp(yy), np.ptp(xx)) < 0.16 * cap_px:
                dots.append((yy.mean(), xx.mean(), max(np.ptp(yy), np.ptp(xx))))
                continue
            cps, nodes = paths(skeletonize(comp))
            cps = prune(cps, nodes, 0.12 * cap_px)
            ps.extend(join(cps))
        strokes = []
        for cy, cx, size in dots:
            # un puntino: un tratto cortissimo, come la penna che tocca il foglio
            r = max(size * 0.25, 0.02 * cap_px)
            strokes.append(f"{(cx - r - ox) * k:.2f},{BASE + (cy + r * 0.3 - base) * k:.2f} {(cx + r - ox) * k:.2f},{BASE + (cy - r * 0.3 - base) * k:.2f}")
        for p in ps:
            if length(p) < 0.04 * cap_px and len(ps) > 1:
                continue
            p = smooth([(float(y), float(x)) for y, x in p])
            p = rdp(p, 0.6)
            strokes.append(" ".join(f"{(x - ox) * k:.2f},{BASE + (y - base) * k:.2f}" for y, x in p))
        # da sinistra, dall'alto: l'ordine in cui si scrive di solito
        adv = font.getlength(ch) * k
        lines.append(f"U+{code:04X}\t0\t{adv:.2f}\t" + "|".join(strokes))
    # lo spazio
    lines.insert(0, f"U+0020\t0\t{font.getlength(' ') * k:.2f}\t")
    with open(dst, "w") as f:
        f.write("# Carattere a filo singolo ricavato da " + src.split("/")[-1] + " con packaging/stroke_font.py (SIL OFL 1.1)\n")
        f.write("\n".join(lines) + "\n")
    print(dst, len(lines), "caratteri")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
