"""Generate CJK 16x16 bitmap font for PSP from Windows system fonts.

Extracts all non-ASCII characters from baked card data, renders them
using Windows GDI, and produces a Rust source file with a sorted
(codepoint, [u8; 32]) lookup table for direct bitmap rendering.
"""

import json
import os
import sys
import ctypes
import ctypes.wintypes


BAKED_DIR = os.path.join(os.path.dirname(__file__), "..", "platforms", "psp", "baked")
OUT_PATH = os.path.join(
    os.path.dirname(__file__), "..", "platforms", "psp", "src", "cjk_font.rs"
)

# --- GDI constants ---
GGO_BITMAP = 1
FW_NORMAL = 400
FONT_SIZE = 16
GDI_ERROR = 0xFFFFFFFF


class LOGFONTW(ctypes.Structure):
    _fields_ = [
        ("lfHeight", ctypes.wintypes.LONG),
        ("lfWidth", ctypes.wintypes.LONG),
        ("lfEscapement", ctypes.wintypes.LONG),
        ("lfOrientation", ctypes.wintypes.LONG),
        ("lfWeight", ctypes.wintypes.LONG),
        ("lfItalic", ctypes.c_byte),
        ("lfUnderline", ctypes.c_byte),
        ("lfStrikeOut", ctypes.c_byte),
        ("lfCharSet", ctypes.c_byte),
        ("lfOutPrecision", ctypes.c_byte),
        ("lfClipPrecision", ctypes.c_byte),
        ("lfQuality", ctypes.c_byte),
        ("lfPitchAndFamily", ctypes.c_byte),
        ("lfFaceName", ctypes.c_wchar * 32),
    ]


class POINTFX(ctypes.Structure):
    _fields_ = [
        ("x", ctypes.wintypes.LONG),  # FIXED 16.16
        ("y", ctypes.wintypes.LONG),
    ]


class GLYPHMETRICS(ctypes.Structure):
    _fields_ = [
        ("gmBlackBoxX", ctypes.wintypes.UINT),
        ("gmBlackBoxY", ctypes.wintypes.UINT),
        ("gmptGlyphOrigin", POINTFX),
        ("gmCellIncX", ctypes.c_short),
        ("gmCellIncY", ctypes.c_short),
    ]


class MAT2(ctypes.Structure):
    """MAT2 with FIXED 16.16 fields."""

    _fields_ = [
        ("eM11", ctypes.wintypes.LONG),
        ("eM12", ctypes.wintypes.LONG),
        ("eM21", ctypes.wintypes.LONG),
        ("eM22", ctypes.wintypes.LONG),
    ]


def extract_codepoints():
    """Extract all unique non-ASCII codepoints from card data."""
    all_text = set()
    for fname in os.listdir(BAKED_DIR):
        if fname.endswith(".json") and fname.startswith("deck_"):
            with open(os.path.join(BAKED_DIR, fname), "r", encoding="utf-8") as f:
                cards = json.load(f)
            for card in cards:
                if isinstance(card, dict):
                    for key in ("name", "ability_text", "series", "product"):
                        val = card.get(key, "")
                        if isinstance(val, str):
                            all_text.add(val)
                    faq = card.get("faq", [])
                    if isinstance(faq, list):
                        for item in faq:
                            if isinstance(item, str):
                                all_text.add(item)

    codepoints = set()
    for text in all_text:
        for ch in text:
            cp = ord(ch)
            if cp >= 0x80:
                codepoints.add(cp)

    return sorted(codepoints)


def render_glyph(hdc, font_handle, codepoint, target_w=16, target_h=16):
    """Render a codepoint to a monochrome bitmap, return as 32-byte [u8; 32]."""
    old_font = ctypes.windll.gdi32.SelectObject(hdc, font_handle)
    if not old_font:
        return None

    gm = GLYPHMETRICS()
    mat2 = MAT2()
    mat2.eM11 = 1 << 16  # 1.0 in 16.16 fixed point
    mat2.eM22 = 1 << 16

    buf_size = ctypes.windll.gdi32.GetGlyphOutlineW(
        hdc, codepoint, GGO_BITMAP, ctypes.byref(gm), 0, None, ctypes.byref(mat2)
    )

    bitmap = bytearray(32)

    if buf_size == 0 or buf_size == GDI_ERROR:
        ctypes.windll.gdi32.SelectObject(hdc, old_font)
        return bytes(bitmap)

    buf = (ctypes.c_byte * buf_size)()
    ret = ctypes.windll.gdi32.GetGlyphOutlineW(
        hdc, codepoint, GGO_BITMAP, ctypes.byref(gm), buf_size, buf, ctypes.byref(mat2)
    )

    if ret == GDI_ERROR:
        ctypes.windll.gdi32.SelectObject(hdc, old_font)
        return bytes(bitmap)

    # Parse monochrome bitmap (bottom-up, DWORD-aligned rows)
    stride = ((gm.gmBlackBoxX + 31) // 32) * 4
    glyph_w = min(gm.gmBlackBoxX, target_w)
    glyph_h = min(gm.gmBlackBoxY, target_h)

    for y in range(glyph_h):
        src_y = gm.gmBlackBoxY - 1 - y  # GDI bitmaps are bottom-up
        for x in range(glyph_w):
            gdi_byte = src_y * stride + (x // 8)
            gdi_bit = 7 - (x % 8)
            if gdi_byte < buf_size:
                if (buf[gdi_byte] >> gdi_bit) & 1:
                    dst_byte = (y * target_w + x) // 8
                    dst_bit = 7 - ((y * target_w + x) % 8)
                    if dst_byte < 32:
                        bitmap[dst_byte] |= 1 << dst_bit

    ctypes.windll.gdi32.SelectObject(hdc, old_font)
    return bytes(bitmap)


def create_font(hdc, font_name):
    logfont = LOGFONTW()
    logfont.lfHeight = -FONT_SIZE
    logfont.lfWidth = 0
    logfont.lfWeight = FW_NORMAL
    logfont.lfCharSet = 128  # SHIFTJIS_CHARSET
    logfont.lfQuality = 2  # ANTIALIASED_QUALITY
    logfont.lfFaceName = font_name
    return ctypes.windll.gdi32.CreateFontIndirectW(ctypes.byref(logfont))


def try_fonts(hdc):
    candidates = [
        "MS Gothic",
        "MS Mincho",
        "BIZ UDGothic",
        "BIZ UDMincho",
        "Yu Gothic",
        "Meiryo",
    ]
    for name in candidates:
        hfont = create_font(hdc, name)
        if hfont:
            try:
                old = ctypes.windll.gdi32.SelectObject(hdc, hfont)
                gm = GLYPHMETRICS()
                mat2 = MAT2()
                mat2.eM11 = 1 << 16
                mat2.eM22 = 1 << 16
                ret = ctypes.windll.gdi32.GetGlyphOutlineW(
                    hdc,
                    0x4E00,
                    GGO_BITMAP,
                    ctypes.byref(gm),
                    0,
                    None,
                    ctypes.byref(mat2),
                )
                ctypes.windll.gdi32.SelectObject(hdc, old)
                if ret != GDI_ERROR:
                    return hfont, name
            except:
                pass
            ctypes.windll.gdi32.DeleteObject(hfont)
    return None, None


def generate():
    print("Extracting codepoints from card data...")
    codepoints = extract_codepoints()
    print(f"  Found {len(codepoints)} unique non-ASCII codepoints")

    gdi32 = ctypes.windll.gdi32
    user32 = ctypes.windll.user32

    # Create memory DC for rendering
    hdc_screen = user32.GetDC(None)
    hdc = gdi32.CreateCompatibleDC(hdc_screen)
    hbmp = gdi32.CreateCompatibleBitmap(hdc_screen, 64, 64)
    gdi32.SelectObject(hdc, hbmp)
    user32.ReleaseDC(None, hdc_screen)

    hfont = None
    try:
        hfont, font_name = try_fonts(hdc)
        if not hfont:
            # Try loading from file
            print("ERROR: No CJK font found!")
            sys.exit(1)
        print(f"  Using font: {font_name}")

        entries = []
        missing = []
        for cp in codepoints:
            bitmap = render_glyph(hdc, hfont, cp)
            if bitmap is None:
                missing.append(cp)
            entries.append((cp, bitmap))

        if missing:
            print(
                f"  WARNING: {len(missing)} glyphs not found: {[hex(c) for c in missing]}"
            )

        # Generate Rust source as a sorted static array with binary search
        lines = [
            "// Auto-generated by tools/gen_cjk_font.py",
            f"// Font: {font_name}, Size: {FONT_SIZE}x{FONT_SIZE}",
            f"// Total CJK codepoints: {len(entries)}",
            "",
            "#[rustfmt::skip]",
            "pub(super) static CJK_FONT: &[(u32, [u8; 32])] = &[",
        ]

        for cp, bitmap in entries:
            hex_bytes = ", ".join(f"0x{b:02X}" for b in bitmap)
            lines.append(f"    (0x{cp:04X}, [{hex_bytes}]),")

        lines.append("];")
        lines.append("")
        lines.append(
            "pub(super) fn lookup_cjk(codepoint: u32) -> Option<&'static [u8; 32]> {"
        )
        lines.append("    let slice = CJK_FONT;")
        lines.append("    let mut lo = 0usize;")
        lines.append("    let mut hi = slice.len();")
        lines.append("    while lo < hi {")
        lines.append("        let mid = lo + (hi - lo) / 2;")
        lines.append("        let (key, _) = &slice[mid];")
        lines.append("        if *key == codepoint {")
        lines.append("            return Some(&slice[mid].1);")
        lines.append("        } else if *key < codepoint {")
        lines.append("            lo = mid + 1;")
        lines.append("        } else {")
        lines.append("            hi = mid;")
        lines.append("        }")
        lines.append("    }")
        lines.append("    None")
        lines.append("}")

        with open(OUT_PATH, "w", encoding="utf-8") as f:
            f.write("\n".join(lines))

        print(f"  Wrote {len(entries)} entries to {OUT_PATH}")
        data_bytes = len(entries) * 32  # bitmap data only
        total_bytes = len(entries) * 36 + 200  # (u32 + [u8;32]) + function overhead
        print(f"  Bitmap data: {data_bytes} bytes, total: ~{total_bytes} bytes")

    finally:
        if hfont is not None:
            gdi32.DeleteObject(hfont)
        gdi32.DeleteObject(hbmp)
        gdi32.DeleteDC(hdc)


if __name__ == "__main__":
    generate()
