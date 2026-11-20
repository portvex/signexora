#!/usr/bin/env python3
"""Build the SA-RS user-interface font from W95FA.

W95FA (Alina Sava, FontsArena, SIL Open Font License 1.1) redraws the Windows 95
system font as outlines on a pixel grid of 80 font units. Its glyphs start
0.625 px from the origin and their advances are not whole pixels, so at the
grid size (12.5 px) the letters drift between pixels and render blurred.

This script makes a derivative font, still under the OFL:
  - every outline point is snapped to the 80-unit grid with the left bearing
    removed, and every advance is rounded to whole pixels (one pixel of space
    after each letter, as in a bitmap font);
  - line metrics are set to 11 px ascent and 2 px descent (13 px, as MS Sans
    Serif 8 pt at 96 dpi);
  - a Bold face is derived by drawing each glyph twice, one pixel apart, which
    is how the Windows 95 bold system font looks.

Usage: python3 scripts/fonts/w95fa_grid.py W95FA.otf OUT_DIR
Needs fontTools (pip install fonttools).
"""

import sys
from pathlib import Path

from fontTools.pens.recordingPen import RecordingPen
from fontTools.pens.t2CharStringPen import T2CharStringPen
from fontTools.ttLib import TTFont

GRID = 80  # font units per pixel
LSB = 50  # W95FA's left bearing
FAMILY = "W95FA Grid"


def snap(v: float) -> int:
    return round(v / GRID) * GRID


def snap_x(x: float) -> int:
    return round((x - LSB) / GRID) * GRID


def advance(width: int, has_outline: bool) -> int:
    if not has_outline:
        return max(GRID, snap(width))
    return max(GRID, snap(width - 2 * LSB) + GRID)


def contours(rec: RecordingPen, dx: int):
    out = []
    for op, pts in rec.value:
        out.append((op, tuple((snap_x(x) + dx, snap(y)) for x, y in pts)))
    return out


def rebuild(font: TTFont, bold: bool) -> None:
    gs = font.getGlyphSet()
    cff = font["CFF "].cff
    top = cff.topDictIndex[0]
    charstrings = top.CharStrings
    hmtx = font["hmtx"].metrics
    for name in font.getGlyphOrder():
        rec = RecordingPen()
        gs[name].draw(rec)
        has_outline = any(op != "closePath" for op, _ in rec.value)
        width = advance(hmtx[name][0], has_outline)
        ops = contours(rec, 0)
        if bold and has_outline:
            ops += contours(rec, GRID)
            width += GRID
        pen = T2CharStringPen(width, gs)
        for op, pts in ops:
            getattr(pen, op)(*pts)
        cs = pen.getCharString(private=top.Private, globalSubrs=cff.GlobalSubrs)
        charstrings[name] = cs
        hmtx[name] = (width, 0)
    top.Private.nominalWidthX = 0
    top.Private.defaultWidthX = 0


def metrics(font: TTFont) -> None:
    asc, desc = 11 * GRID, -2 * GRID
    hhea = font["hhea"]
    hhea.ascent, hhea.descent, hhea.lineGap = asc, desc, 0
    os2 = font["OS/2"]
    os2.sTypoAscender, os2.sTypoDescender, os2.sTypoLineGap = asc, desc, 0
    os2.usWinAscent, os2.usWinDescent = asc, -desc
    os2.fsSelection |= 1 << 7  # USE_TYPO_METRICS


def rename(font: TTFont, bold: bool) -> None:
    style = "Bold" if bold else "Regular"
    ps = f"W95FAGrid-{style}"
    names = font["name"]
    for rec in list(names.names):
        if rec.nameID in (1, 4, 6, 16, 17, 2, 3):
            names.removeNames(nameID=rec.nameID)
    for nid, text in (
        (1, FAMILY),
        (2, style),
        (3, f"1.002;{ps}"),
        (4, f"{FAMILY} {style}" if bold else FAMILY),
        (6, ps),
    ):
        names.setName(text, nid, 3, 1, 0x409)
        names.setName(text, nid, 1, 0, 0)
    desc = (
        "Derived from W95FA by Alina Sava (FontsArena), SIL Open Font License 1.1: "
        "snapped to its pixel grid for SA-RS"
        + ("; bold drawn one pixel wider." if bold else ".")
    )
    names.setName(desc, 10, 3, 1, 0x409)
    font["CFF "].cff.fontNames = [ps]
    top = font["CFF "].cff.topDictIndex[0]
    top.FullName = f"{FAMILY} {style}"
    top.FamilyName = FAMILY
    top.Weight = style
    head = font["head"]
    os2 = font["OS/2"]
    if bold:
        head.macStyle |= 1
        os2.usWeightClass = 700
        os2.fsSelection = (os2.fsSelection & ~0b1000000) | 0b100000
    else:
        os2.usWeightClass = 400


def main() -> None:
    src, out_dir = Path(sys.argv[1]), Path(sys.argv[2])
    for bold in (False, True):
        font = TTFont(src)
        rebuild(font, bold)
        metrics(font)
        rename(font, bold)
        name = "W95FAGrid-Bold.otf" if bold else "W95FAGrid-Regular.otf"
        font.save(out_dir / name)
        print("wrote", out_dir / name)


if __name__ == "__main__":
    main()
