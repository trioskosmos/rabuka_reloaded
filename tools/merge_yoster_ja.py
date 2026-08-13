import sys
from fontTools.ttLib import TTFont

YOSTER = r"C:\Users\trios\AppData\Local\Temp\opencode\agb_examples\examples\the-dungeon-puzzlers-lament\fnt\yoster.ttf"
PIXEL = r"C:\Users\trios\AppData\Local\Temp\opencode\pm\PixelMplus-20130602\PixelMplus12-Regular.ttf"
OUT = r"C:\Users\trios\AppData\Local\Temp\opencode\yoster_ja.ttf"


def is_cjk(cp):
    return (
        (0x1100 <= cp <= 0x11FF)  # Hangul jamo
        or (0x2E80 <= cp <= 0x2EFF)  # CJK radicals
        or (0x3000 <= cp <= 0x30FF)  # CJK punct + kana
        or (0x3100 <= cp <= 0x31FF)  # bopomofo
        or (0x3200 <= cp <= 0x32FF)  # CJK compat
        or (0x3300 <= cp <= 0x4DBF)  # CJK ext A
        or (0x4E00 <= cp <= 0x9FFF)  # CJK unified
        or (0xAC00 <= cp <= 0xD7AF)  # Hangul syllables
        or (0xF900 <= cp <= 0xFAFF)  # CJK compat ideographs
        or (0xFE30 <= cp <= 0xFE4F)  # CJK compat forms
        or (0xFF00 <= cp <= 0xFFEF)  # fullwidth forms
    )


y = TTFont(YOSTER)
p = TTFont(PIXEL)

y_cmap = y.getBestCmap()
p_cmap = p.getBestCmap()

y_glyf = y["glyf"]
p_glyf = p["glyf"]
y_hmtx = y["hmtx"]
p_hmtx = p["hmtx"]

# codepoints to add: CJK chars present in PixelMplus but not already in yoster
to_add = []
for cp, gname in p_cmap.items():
    if cp in y_cmap:
        continue
    if is_cjk(cp):
        to_add.append((cp, gname))
print("adding", len(to_add), "CJK codepoints")

added = set()
order = list(y.getGlyphOrder())


def copy_glyph(gname):
    if gname in added or gname in y_glyf.glyphs:
        return
    glyph = p_glyf[gname]
    if hasattr(glyph, "components") and glyph.components:
        for comp in glyph.components:
            copy_glyph(comp.glyphName)
    added.add(gname)
    order.append(gname)
    adv, lsb = p_hmtx.metrics.get(gname, p_hmtx.metrics.get(".notdef", (0, 0)))
    y_hmtx.metrics[gname] = (adv, lsb)


for cp, gname in to_add:
    copy_glyph(gname)

# apply glyphs to yoster glyf
for gname in added:
    y_glyf.glyphs[gname] = p_glyf[gname]

y.setGlyphOrder(order)
y["maxp"].numGlyphs = len(order)

# add codepoints to every cmap subtable
for table in y["cmap"].tables:
    if table.platformID == 3 and table.platEncID in (1, 10):
        for cp, gname in to_add:
            table.cmap[cp] = gname
    elif table.platformID == 0:
        for cp, gname in to_add:
            table.cmap[cp] = gname

y.save(OUT)
print("saved", OUT, "with", len(order), "glyphs")
