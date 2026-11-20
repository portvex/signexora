"""Build the Latin and Cyrillic subsets of the Linux looks' fonts.

The Linux looks of SA-RS (Phase 13) use Cantarell (GNOME), Noto Sans (KDE)
and DejaVu Sans (GNOME 2, Motif) when installed, and these bundled subsets
otherwise. All three are open fonts (SIL OFL 1.1; DejaVu: the Bitstream Vera
licence with public-domain changes).

Usage (fontTools 4.x, in a virtual environment):

    python linux_subsets.py <download dir> <output dir>

The download dir holds, from their upstream sources:
  cantarell-Cantarell-Regular.ttf, cantarell-Cantarell-Bold.ttf
      (github.com/google/fonts, ofl/cantarell)
  notosans-var.ttf
      (github.com/google/fonts, ofl/notosans, NotoSans[wdth,wght].ttf)
  dv/dejavu-fonts-ttf-2.37/ttf/DejaVuSans.ttf and DejaVuSans-Bold.ttf
      (github.com/dejavu-fonts/dejavu-fonts, release 2.37)
"""

import sys
from pathlib import Path

from fontTools import subset
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

# Basic Latin, Latin-1, Latin Extended-A and B, general punctuation,
# Cyrillic and Cyrillic Supplement, the currency signs and arrows SA's texts
# never use but fallbacks may meet: enough for the nine languages of the UI
# except Japanese and Korean (their own bundled Noto subsets).
UNICODES = (
    list(range(0x20, 0x7F))
    + list(range(0xA0, 0x250))
    + list(range(0x400, 0x530))
    + list(range(0x2000, 0x2070))
    + list(range(0x20A0, 0x20C0))
    + [0x2116, 0x2122, 0x2212, 0xFFFD]
)


def subset_font(font: TTFont, out: Path) -> None:
    opts = subset.Options()
    opts.layout_features = ["kern", "liga", "calt", "ccmp", "locl", "mark", "mkmk"]
    opts.name_IDs = ["*"]
    opts.name_languages = ["*"]
    opts.notdef_outline = True
    opts.hinting = False
    s = subset.Subsetter(options=opts)
    s.populate(unicodes=UNICODES)
    s.subset(font)
    font.save(out)
    print(f"{out.name}: {out.stat().st_size // 1024} KiB")


def name_instance(font: TTFont, weight: str) -> None:
    """Name a static instance of Noto Sans after its weight.

    The variable font's default names say "Noto Sans Regular" for every
    instance; without this the Bold subset would share the Regular one's
    PostScript name and font loaders would treat the two as one face.
    """
    full = "Noto Sans" if weight == "Regular" else f"Noto Sans {weight}"
    ps = f"NotoSans-{weight}"
    names = font["name"]
    for name_id in (16, 17, 25):
        names.removeNames(nameID=name_id)
    names.setName("Noto Sans", 1, 3, 1, 0x409)
    names.setName(weight, 2, 3, 1, 0x409)
    names.setName(f"{ps};instance", 3, 3, 1, 0x409)
    names.setName(full, 4, 3, 1, 0x409)
    names.setName(ps, 6, 3, 1, 0x409)
    bold = weight == "Bold"
    head = font["head"]
    os2 = font["OS/2"]
    if bold:
        head.macStyle |= 1
        os2.fsSelection = (os2.fsSelection & ~0x40) | 0x20
    else:
        head.macStyle &= ~1
        os2.fsSelection = (os2.fsSelection & ~0x20) | 0x40


def main() -> None:
    src, dst = Path(sys.argv[1]), Path(sys.argv[2])
    dst.mkdir(parents=True, exist_ok=True)
    for weight in ("Regular", "Bold"):
        subset_font(
            TTFont(src / f"cantarell-Cantarell-{weight}.ttf"),
            dst / f"Cantarell-{weight}-subset.ttf",
        )
    for weight, wght in (("Regular", 400), ("Bold", 700)):
        var = TTFont(src / "notosans-var.ttf")
        static = instancer.instantiateVariableFont(
            var, {"wght": wght, "wdth": 100}, updateFontNames=True
        )
        name_instance(static, weight)
        subset_font(static, dst / f"NotoSans-{weight}-subset.ttf")
    dv = src / "dv" / "dejavu-fonts-ttf-2.37" / "ttf"
    for name in ("DejaVuSans", "DejaVuSans-Bold"):
        subset_font(TTFont(dv / f"{name}.ttf"), dst / f"{name}-subset.ttf")


if __name__ == "__main__":
    main()
