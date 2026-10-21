#!/usr/bin/env python3
"""I caratteri «a mano» degli appunti: OpenSlate Corsivo e OpenSlate Stampatello.

Le lettere sono quelle con cui la chat scrive sulla lavagna (i caratteri di
Hershey a filo singolo, con gli accenti composti da OpenSlate): ogni linea
diventa un tratto di penna con lo spessore dato, e il tratto diventa il
contorno di un normale carattere TrueType. Così gli appunti (pagina, Word, PDF)
hanno la stessa scrittura della lavagna.

1. I tratti escono dall'app stessa (src/handwrite.rs):
     HAND_OUT=<cartella> cargo test -- --ignored glifi_per_i_caratteri
2. Poi:
     python3 packaging/hand_fonts.py <cartella> assets/fonts
   scrive OpenSlateCorsivo-Regular.ttf, -Bold.ttf, OpenSlateStampatello-*.ttf

Serve: pip install fonttools pyclipper.
I caratteri di Hershey: vedi src/handwrite/LICENSE-hershey.txt.
"""

import json
import os
import sys

import pyclipper
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen

UPM = 1000
CAP = 21.0  # altezza delle maiuscole di Hershey (da -12 a 9)
BASE = 9.0  # la riga di base di Hershey
PREC = 8.0  # pyclipper lavora con interi: un po' di decimali

# nome, altezza delle maiuscole (su 1000: nel corsivo le minuscole sono
# piccole, allora tutto un po' più grande) e spessore della penna per peso
FAMILIES = {
    "corsivo": ("OpenSlate Corsivo", 820.0, {"Regular": (400, 72.0), "Bold": (700, 108.0)}),
    "stampatello": ("OpenSlate Stampatello", 700.0, {"Regular": (400, 74.0), "Bold": (700, 122.0)}),
}
K = 700.0 / CAP  # unità del carattere per unità di Hershey (si cambia per famiglia)

NOTICE = (
    "Derived from the Hershey Fonts, originally created by Dr. A. V. Hershey while working at "
    "the U. S. National Bureau of Standards. The format of the Font data was originally "
    "created by James Hurt, Cognition, Inc. Glyph files from "
    "https://github.com/kamalmostafa/hershey-fonts; accents composed and strokes widened by "
    "OpenSlate (https://github.com/rmpaolillo/OpenSlate). Free to use for any purpose, as the "
    "Hershey Fonts licence allows."
)


def outline(strokes, left, width):
    """Le linee del carattere diventano contorni (tratti tondi alle estremità)."""
    pco = pyclipper.PyclipperOffset(miter_limit=2.0, arc_tolerance=0.6 * PREC)
    for st in strokes:
        pts = [(round((x - left) * K * PREC), round((BASE - y) * K * PREC)) for x, y in st]
        # punti uguali di fila: via (pyclipper non li vuole)
        clean = [p for i, p in enumerate(pts) if i == 0 or p != pts[i - 1]]
        if len(clean) == 1:
            clean = clean * 2
        closed = len(clean) > 2 and clean[0] == clean[-1]
        kind = pyclipper.ET_CLOSEDLINE if closed else pyclipper.ET_OPENROUND
        pco.AddPath(clean[:-1] if closed else clean, pyclipper.JT_ROUND, kind)
    if not strokes:
        return []
    polys = pco.Execute(width / 2.0 * PREC)
    # TrueType: il contorno esterno in senso orario (pyclipper lo dà antiorario)
    return [[(x / PREC, y / PREC) for x, y in reversed(p)] for p in polys if len(p) >= 3]


# dove comincia il corpo delle minuscole (y di Hershey)
XTOP = {"OpenSlate Corsivo": 0.0, "OpenSlate Stampatello": -5.0}


def glyph_name(c):
    if c == " ":
        return "space"
    return "uni%04X" % ord(c)


def build(glyphs, family, style, weight, width, out):
    fb = FontBuilder(UPM, isTTF=True)
    order = [".notdef"]
    cmap = {}
    metrics = {}
    shapes = {}
    # .notdef: un rettangolino
    pen = TTGlyphPen(None)
    for x0, y0, x1, y1 in [(50, 0, 450, 700)]:
        pen.moveTo((x0, y0))
        pen.lineTo((x0, y1))
        pen.lineTo((x1, y1))
        pen.lineTo((x1, y0))
        pen.closePath()
        pen.moveTo((x0 + 50, y0 + 50))
        pen.lineTo((x1 - 50, y0 + 50))
        pen.lineTo((x1 - 50, y1 - 50))
        pen.lineTo((x0 + 50, y1 - 50))
        pen.closePath()
    shapes[".notdef"] = pen.glyph()
    metrics[".notdef"] = (500, 50)
    for g in glyphs:
        c = g["c"]
        name = glyph_name(c)
        if name in shapes:
            continue
        adv = round((g["r"] - g["l"]) * K)
        pen = TTGlyphPen(None)
        for poly in outline(g["s"], g["l"], width):
            pen.moveTo((round(poly[0][0]), round(poly[0][1])))
            for x, y in poly[1:]:
                pen.lineTo((round(x), round(y)))
            pen.closePath()
        shapes[name] = pen.glyph()
        order.append(name)
        cmap[ord(c)] = name
        xs = [pt[0] for pt in shapes[name].coordinates] if shapes[name].numberOfContours else [0]
        metrics[name] = (max(adv, 0), min(xs))
    fb.setupGlyphOrder(order)
    fb.setupCharacterMap(cmap)
    fb.setupGlyf(shapes)
    fb.setupHorizontalMetrics(metrics)
    asc, desc = round(28 * K), -round(13 * K)
    fb.setupHorizontalHeader(ascent=asc, descent=desc, lineGap=0)
    full = f"{family} {style}" if style != "Regular" else family
    ps = (family + "-" + style).replace(" ", "")
    fb.setupNameTable(
        {
            "familyName": family,
            "styleName": style,
            "uniqueFontIdentifier": f"OpenSlate: {full}",
            "fullName": full,
            "psName": ps,
            "version": "Version 1.000",
            "copyright": "Hershey Fonts: Dr. A. V. Hershey (U.S. NBS); .jhf format: James Hurt, Cognition, Inc.",
            "description": NOTICE,
            "licenseDescription": NOTICE,
            "vendorURL": "https://github.com/rmpaolillo/OpenSlate",
        }
    )
    bold = style == "Bold"
    fb.setupOS2(
        sTypoAscender=asc,
        sTypoDescender=desc,
        sTypoLineGap=0,
        usWinAscent=asc,
        usWinDescent=-desc,
        sxHeight=round((BASE - XTOP[family]) * K),
        sCapHeight=round(CAP * K),
        usWeightClass=weight,
        fsSelection=0x20 if bold else 0x40,
        achVendID="OSLT",
    )
    fb.font["head"].macStyle = 1 if bold else 0
    fb.setupPost(isFixedPitch=0, underlinePosition=-120, underlineThickness=round(width * 0.8))
    fb.save(out)
    return len(order) - 1


def main():
    global K
    src, dst = sys.argv[1], sys.argv[2]
    os.makedirs(dst, exist_ok=True)
    for key, (family, cap, weights) in FAMILIES.items():
        K = cap / CAP
        glyphs = json.load(open(os.path.join(src, f"glifi-{key}.json"), encoding="utf-8"))
        for style, (weight, width) in weights.items():
            out = os.path.join(dst, f"{family.replace(' ', '')}-{style}.ttf")
            n = build(glyphs, family, style, weight, width, out)
            print(f"{out}: {n} caratteri, {os.path.getsize(out) // 1024} KB")


if __name__ == "__main__":
    main()
