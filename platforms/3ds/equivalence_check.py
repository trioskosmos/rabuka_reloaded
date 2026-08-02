import re, sys


def read(p):
    with open(p, encoding="utf-8") as f:
        return f.read()


def find_body_end(src, open_idx):
    """Find matching close brace, correctly skipping string/char literals and comments."""
    depth = 0
    i = open_idx
    n = len(src)
    while i < n:
        c = src[i]
        if c == '"':
            i += 1
            while i < n and src[i] != '"':
                if src[i] == "\\":
                    i += 1
                i += 1
            i += 1
            continue
        if c == "'":
            # char literal or lifetime; handle 'x' and '\x'
            i += 1
            if i < n and src[i] == "\\":
                i += 1
            if i < n:
                i += 1  # closing quote
            i += 1
            continue
        if c == "/" and i + 1 < n and src[i + 1] == "/":
            while i < n and src[i] != "\n":
                i += 1
            continue
        if c == "/" and i + 1 < n and src[i + 1] == "*":
            i += 2
            while i + 1 < n and not (src[i] == "*" and src[i + 1] == "/"):
                i += 1
            i += 2
            continue
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return i + 1
        i += 1
    return n


def extract_named(src, kind):
    """Extract all defs of `kind` (fn/struct/enum/const) with bodies."""
    out = {}
    pat = re.compile(
        r"\b(?:pub\s+)?(?:unsafe\s+)?" + kind + r"\s+([A-Za-z_][A-Za-z0-9_]*)"
    )
    for m in pat.finditer(src):
        name = m.group(1)
        if name in ("if", "else", "for", "while", "match", "loop", "let", "return"):
            continue
        if kind == "fn":
            brace = src.find("{", m.end())
        else:
            brace = src.find("{", m.end())
            if brace == -1:
                continue
        if brace == -1:
            # fn without body (declaration) - skip
            continue
        end = find_body_end(src, brace)
        body = src[m.start() : end]
        out.setdefault(name, []).append(body)
    return out


def normalize(s):
    s = re.sub(r"//.*", "", s)
    s = re.sub(r"/\*.*?\*/", "", s, flags=re.S)
    s = re.sub(r"pub\s+", "", s)
    s = re.sub(r"\bstd::ffi::CString\b", "CString", s)
    s = re.sub(r"\brabuka_engine::card::HeartMap\b", "HeartMap", s)
    s = re.sub(
        r"\brabuka_engine::core::game_modifiers::GameModifiers\b", "GameModifiers", s
    )
    s = re.sub(r"\brabuka_engine::card::CardDatabase\b", "CardDatabase", s)
    s = re.sub(r"\brabuka_engine::game_state::GameState\b", "GameState", s)
    s = re.sub(r"\brabuka_engine::card::Card\b", "Card", s)
    s = re.sub(r"\brabuka_engine::card::", "", s)
    s = re.sub(r"\brabuka_engine::", "", s)
    s = re.sub(r"\bcrate::ffi::", "", s)
    s = re.sub(r"\brabuka_3ds::ffi::", "", s)
    s = re.sub(r"\bcrate::util::", "", s)
    s = re.sub(r"\bcrate::ui::text::", "", s)
    s = re.sub(r"\buse super::", "", s)
    s = re.sub(r"\s+", " ", s)
    return s.strip()


orig = read("/tmp/orig_bin.rs")

moved_fns = {
    "lang.rs": ["current_lang", "set_lang", "tl", "tl_fmt"],
    "util.rs": [
        "heart_color_index",
        "format_need_hearts_icons",
        "tl_area",
        "cn_or_empty",
        "ticks_to_ms",
        "looks_like_b64",
        "base64_decode",
    ],
    "net.rs": [
        "mp_can_act",
        "execute_received_action",
        "action_tag_of",
        "route_authoritative_action",
    ],
    "ui/text.rs": [
        "render_text_with_icons",
        "icon_width_for",
        "heart_label_to_icon",
        "build_heart_str",
        "card_stat_line",
        "measure_text_width",
        "compute_card_stats",
        "is_text_only",
        "split_at_px",
        "wrap_ability_text",
        "truncate_aware_segments",
        "segment_text",
        "wrap_text",
    ],
    "ui/hint.rs": ["render_hint_bar"],
    "ui/grid.rs": ["card_grid_input", "render_card_grid", "render_card_detail"],
    "ui/card_atlas.rs": ["build_qr_sorted", "decode_qr_binary"],
}

moved_types = {
    "ui/text.rs": ["CardDisplayStats"],
    "ui/card_atlas.rs": ["CardAtlas"],
    "util.rs": ["YieldReader"],
}

# New types introduced during refactor (not extracted) - verify they're used consistently
new_types = ["TextSeg", "GridAction"]

orig_fns = extract_named(orig, "fn")
orig_structs = extract_named(orig, "struct")

ok = True
for fname, names in moved_fns.items():
    new = read("src/" + fname)
    new_fns = extract_named(new, "fn")
    for n in names:
        o = orig_fns.get(n, [])
        c = new_fns.get(n, [])
        if not o:
            print(f"MISSING-in-orig {fname}::{n}")
            ok = False
            continue
        if not c:
            print(f"MISSING-in-new {fname}::{n}")
            ok = False
            continue
        if len(o) > 1:
            print(f"NOTE: {n} defined {len(o)}x in orig, comparing first")
        if len(c) > 1:
            print(f"NOTE: {n} defined {len(c)}x in new, comparing first")
        match = normalize(o[0]) == normalize(c[0])
        print(("OK  " if match else "DIFF") + f" {fname}::{n}")

for fname, names in moved_types.items():
    new = read("src/" + fname)
    new_structs = extract_named(new, "struct")
    for n in names:
        o = orig_structs.get(n, [])
        c = new_structs.get(n, [])
        if not o or not c:
            print(f"MISSING struct {n} in {'orig' if not o else 'new'}")
            ok = False
            continue
        match = normalize(o[0]) == normalize(c[0])
        print(("OK  " if match else "DIFF") + f" {fname}::struct {n}")

print()
print("ALL MATCH" if ok else "DIFFERENCES FOUND")
