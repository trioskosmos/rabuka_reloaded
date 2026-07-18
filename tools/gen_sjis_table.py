"""Generate UNICODE_TO_SJIS Rust static array from JIS0208.TXT"""

import sys
import os


def main():
    # Find JIS0208.TXT
    home = os.path.expanduser("~")
    possible_paths = [
        os.path.join(
            home, r".local\share\opencode\tool-output\tool_f7340cceb0017azNtCAzWBdJJ6"
        ),
        os.path.join(os.path.dirname(__file__), "JIS0208.TXT"),
    ]
    txt_path = None
    for p in possible_paths:
        if os.path.exists(p):
            txt_path = p
            break
    if not txt_path:
        print("ERROR: JIS0208.TXT not found", file=sys.stderr)
        sys.exit(1)

    entries = []
    with open(txt_path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            parts = line.split("\t")
            if len(parts) >= 3:
                sjis_str = parts[0]  # e.g. 0x8140
                unicode_str = parts[2]  # e.g. 0x3000
                try:
                    sjis = int(sjis_str, 16)
                    unicode_val = int(unicode_str, 16)
                    entries.append((unicode_val, sjis))
                except ValueError:
                    continue

    # Sort by unicode code point for binary search
    entries.sort(key=lambda x: x[0])

    # Generate Rust code
    lines = []
    lines.append("// Auto-generated from JIS0208.TXT")
    lines.append(f"// Total entries: {len(entries)}")
    lines.append("#[rustfmt::skip]")
    lines.append("static UNICODE_TO_SJIS: [(u16, u16); {}] = [".format(len(entries)))

    for i, (uc, sjis) in enumerate(entries):
        comma = "," if i < len(entries) - 1 else " "
        lines.append(f"    (0x{uc:04X}, 0x{sjis:04X}),")

    lines.append("];")
    lines.append("")
    lines.append("fn unicode_to_sjis(c: char) -> Option<u16> {")
    lines.append("    let code = c as u16;")
    lines.append("    // Binary search on sorted UNICODE_TO_SJIS")
    lines.append("    let slice: &[(u16, u16)] = &UNICODE_TO_SJIS;")
    lines.append("    let mut lo = 0usize;")
    lines.append("    let mut hi = slice.len();")
    lines.append("    while lo < hi {")
    lines.append("        let mid = lo + (hi - lo) / 2;")
    lines.append("        let (key, _) = slice[mid];")
    lines.append("        if key == code {")
    lines.append("            return Some(slice[mid].1);")
    lines.append("        } else if key < code {")
    lines.append("            lo = mid + 1;")
    lines.append("        } else {")
    lines.append("            hi = mid;")
    lines.append("        }")
    lines.append("    }")
    lines.append("    None")
    lines.append("}")

    # Write output
    out_path = os.path.join(
        os.path.dirname(__file__), "..", "engine_psp", "src", "sjis_table.rs"
    )
    with open(out_path, "w", encoding="utf-8") as f:
        f.write("\n".join(lines))
    print(f"Wrote {len(entries)} entries to {os.path.abspath(out_path)}")


if __name__ == "__main__":
    main()
