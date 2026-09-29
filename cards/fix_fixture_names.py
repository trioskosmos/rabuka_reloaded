"""Fix test-file constants whose comment names the WRONG character.

Only rewrites a comment whose leading segment is purely a character name (CJK plus
joining marks). A prose lead like `自分のエネルギーデッキから` or `控え室に置かれたとき` is a
description, not a name claim, and is left alone.

Driven by audit_fixture_annotations.py, which found 22 name mismatches; the genuinely
wrong ones are fixed here and the rest are reported for a human.
"""
import io
import json
import re
from pathlib import Path

ROOT = Path(r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded")
TESTS = ROOT / "engine" / "tests"
cards = json.load(io.open(ROOT / "cards" / "cards.json", encoding="utf-8"))

ESCAPE_RE = re.compile(r"\\u\{([0-9a-fA-F]{1,6})\}")
LEAD_SPLIT = re.compile(r"[,;:()\u3001\uff08\uff09\u2014\u2013\u2192]")
# A pure character name: CJK, hiragana, katakana, the ＆/& joiner, '?' and '※'.
PURE_NAME = re.compile(r"^[\u3040-\u30ff\u4e00-\u9fff\uff06&\u3001\u2026\?\*\s\u3000]+$")

CONST_RE = re.compile(
    r'(?P<doc>(?:[ \t]*//[^\n]*\n)*)(?P<indent>[ \t]*)const\s+(?P<name>[A-Z0-9_]+)\s*:\s*&str\s*=\s*'
    r'"(?P<card>[^"]+)"\s*;[ \t]*(?://\s*(?P<comment>[^\n]*))?$'
)

GROUP_WORDS = (
    "\u6d45\u79d1\u65e5\u5149", "\u83b1\u30cc\u7a7a", "\u6d5b\u30f6\u5cbd", "\u6d5c\u8ecd",
    "\u30e1\u30f3\u30d0\u30fc", "\u30e9\u30a4\u30d6", "\u83b1\u30cc\u7a7a\u5973\u5b66\u9662",
)


def resolve(card_no):
    card_no = ESCAPE_RE.sub(lambda m: chr(int(m.group(1), 16)), card_no)
    if card_no in cards:
        return card_no
    alt = card_no.replace("\uff0b", "+")
    return alt if alt in cards else None


def squeeze(s):
    return re.sub(r"[\s\u3000]+", "", s or "")


fixed, skipped = [], []
for path in sorted(TESTS.rglob("*.rs")):
    text = io.open(path, encoding="utf-8", errors="replace").read()
    out_lines = []
    changed = False
    for line in text.splitlines(keepends=True):
        m = CONST_RE.match(line)
        if not m or not (m.group("comment") or "").strip():
            out_lines.append(line)
            continue
        key = resolve(m.group("card"))
        comment = m.group("comment")
        lead = LEAD_SPLIT.split(comment)[0]
        sl = squeeze(lead)
        if not key or not sl or not PURE_NAME.match(sl) or len(sl) < 2:
            out_lines.append(line)
            continue
        if any(w in sl for w in GROUP_WORDS):
            out_lines.append(line)
            continue
        real = squeeze(cards[key].get("name") or "")
        if not real or not re.search(r"[\u3040-\u30ff\u4e00-\u9fff]", real):
            out_lines.append(line)
            continue
        if real == sl or real.startswith(sl) or sl.startswith(real):
            out_lines.append(line)
            continue
        # Rewrite ONLY the leading name, keeping the rest of the comment.
        new_lead = real
        rest = comment[len(lead):]
        out_lines.append(line.replace(comment, new_lead + rest, 1))
        changed = True
        fixed.append((path.name, m.group("name"), m.group("card"), sl, real))
    if changed:
        path.write_text("".join(out_lines), encoding="utf-8")

print(f"rewritten: {len(fixed)}")
for f in fixed:
    print(f"  {f[0]} ::{f[1]} {f[2]}: {f[3]!r} -> {f[4]!r}")
