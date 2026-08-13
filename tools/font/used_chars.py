#!/usr/bin/env python3
"""Single source of truth for the game's used characters (font whitelist).

Both the 3DS (platforms/3ds/scripts/build_font.py) and GBA
(tools/bake_font_tiles.py) font builders get their glyph set from this module,
so the fonts always cover every string the game can display.

Sources scanned:
  - cards/cards.json, cards/abilities.json   (card names, ability text)
  - cards/qa_data.json                       (rules/QA text)
  - engine/src/**/*.rs                        (phase labels, prompts, etc.)
  - platforms/**/src/**/*.rs                  (UI strings on each port)
  - platforms/3ds/romfs/locales/**/*          (en/jp/names/ability locale files)
  - web_ui/decks/*.txt                        (deck names/entries)

ASCII printable (0x20-0x7E) is always included.
"""

import glob
import json
import os


def _walk_str(obj):
    if isinstance(obj, dict):
        for v in obj.values():
            yield from _walk_str(v)
    elif isinstance(obj, list):
        for v in obj:
            yield from _walk_str(v)
    elif isinstance(obj, str):
        yield obj


def compute_used_chars(root: str) -> str:
    chars = set()
    # Always include printable ASCII.
    chars.update(chr(cp) for cp in range(0x20, 0x7F))

    def add_file(path):
        if os.path.isfile(path):
            try:
                chars.update(open(path, encoding="utf-8", errors="ignore").read())
            except Exception:
                pass

    def add_json(path):
        if os.path.isfile(path):
            try:
                data = json.load(open(path, encoding="utf-8"))
            except Exception:
                return
            for s in _walk_str(data):
                chars.update(s)

    # Card + ability data.
    add_json(os.path.join(root, "cards", "cards.json"))
    add_json(os.path.join(root, "cards", "abilities.json"))
    add_json(os.path.join(root, "cards", "qa_data.json"))

    # Rust sources (engine + all platform ports) — holds hardcoded Japanese
    # strings (phase labels like 先攻/後攻, UI prompts) not present in JSON.
    for pattern in [
        "engine/src/**/*.rs",
        "platforms/**/src/**/*.rs",
    ]:
        for p in glob.glob(os.path.join(root, pattern), recursive=True):
            add_file(p)

    # 3DS locale files.
    for p in glob.glob(os.path.join(root, "platforms/3ds/romfs/locales/**/*"), recursive=True):
        add_file(p)

    # Deck files.
    for p in glob.glob(os.path.join(root, "web_ui/decks/*.txt")):
        add_file(p)

    # Drop control chars.
    chars = {c for c in chars if c not in ("\n", "\r", "\t")}
    return "".join(sorted(chars))


def repo_root() -> str:
    return os.path.normpath(os.path.join(os.path.dirname(__file__), "..", ".."))


if __name__ == "__main__":
    text = compute_used_chars(repo_root())
    print(len(text), "chars")
    for ch in "先攻後ライブ勝敗判定ジャンケンエネルギー":
        print(ch, "present:", ch in text)
