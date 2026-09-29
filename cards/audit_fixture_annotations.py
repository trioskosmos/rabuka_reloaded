"""Audit test-file card constants against the real card data.

A test constant carries a character name and often a group in its comment:

    const BIBI_2: &str = "PL!-pb2-006-PP"; // 西木野真姫
    const FILLER:  &str = "PL!-sd1-010-SD"; // 高坂穂乃果 (μ's)

If either is wrong the fixture is wrong, and a wrong fixture is SILENT: the card
resolves, it is simply a different card, and the failure looks like a deep engine bug.
That produced three wrong multi-paragraph diagnoses in this expansion — see
docs/PBLL02_ABILITY_REVIEW.md 8.9 and 8.10.

Checks, all exact (no fuzzy matching, so no false alarms):
  1. the card number resolves in cards.json
  2. the character's NAME, taken from the comment's leading CJK run, equals the real name
  3. a group claim: μ's and friends are SERIES in this data; the unit names
     (BiBi / Printemps / AZALEA / 5yncri5e! / …) are checked against `unit`
"""
import io
import json
import re
from pathlib import Path

ROOT = Path(r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded")
TESTS = ROOT / "engine" / "tests"

cards = json.load(io.open(ROOT / "cards" / "cards.json", encoding="utf-8"))

# A card number in a test may use the full-width plus; cards.json may hold either.
ESCAPE_RE = re.compile(r"\\u\{([0-9a-fA-F]{1,6})\}")


def resolve(card_no):
    # A test source may write the full-width plus as a Rust unicode escape,
    # `"PL!S-bp7-003-R\u{ff0b}"`, which Rust does decode in a string literal.
    card_no = ESCAPE_RE.sub(lambda m: chr(int(m.group(1), 16)), card_no)
    if card_no in cards:
        return card_no, None
    alt = card_no.replace("\uff0b", "+")
    if alt in cards:
        return alt, "half-width + not in cards.json (resolved via full-width)"
    return None, "CARD NOT FOUND in cards.json"


# Series-level group names, checked against `series`.
SERIES_GROUPS = {
    "\u03bc's": "\u30e9\u30d6\u30e9\u30a4\u30d6\uff01",
    "lily white": "\u30e9\u30d6\u30e9\u30a4\u30d6\uff01",
    "Printemps": None,  # also a unit; handled below
}

# Unit-level group names, checked against `unit`.
UNIT_GROUPS = {
    "BiBi", "Printemps", "lilywhite", "lily white", "AZALEA", "5yncri5e!",
    "DiverDiva", "KALEIDOSCORE", "SunnyPassion", "CatChu!", "EdelNote", "R3BIRTH",
    "CYaRon!", "CYaRon\uff01", "GuiltyKiss", "QU4RTZ", "A-RISE", "SaintSnow",
    "DOLLCHESTRA", "AiScReam",
}

CJK = r"[\u3040-\u30ff\u4e00-\u9fff]+"

CONST_RE = re.compile(
    r'(?m)^(?P<doc>(?:[ \t]*//[^\n]*\n)*)[ \t]*const\s+(?P<name>[A-Z0-9_]+)\s*:\s*&str\s*=\s*'
    r'"(?P<card>[^"]+)"\s*;[ \t]*(?://\s*(?P<inline>[^\n]*))?$'
)


def comment_of(m):
    if m.group("inline") and m.group("inline").strip():
        return m.group("inline").strip()
    lines = [ln.strip().lstrip("/").strip() for ln in (m.group("doc") or "").splitlines()]
    lines = [ln for ln in lines if ln]
    return lines[-1] if lines else ""


def squeeze(s):
    return re.sub(r"[\s\u3000]+", "", s or "")


problems = []
checked = 0
for path in sorted(TESTS.rglob("*.rs")):
    text = io.open(path, encoding="utf-8", errors="replace").read()
    for m in CONST_RE.finditer(text):
        const_name, card_no = m.group("name"), m.group("card")
        comment = comment_of(m)
        if not comment:
            continue
        key, note = resolve(card_no)
        if key is None:
            problems.append((path.name, const_name, card_no, comment, note))
            continue
        checked += 1
        card = cards[key]
        if note:
            problems.append((path.name, const_name, card_no, comment, note))

        real_name = squeeze(card.get("name") or "")
        # Only the LEADING segment of the comment is a name claim: everything after the
        # first comma / paren / dash is a role or group annotation.
        lead = re.split(r"[,;:()\u3001\uff08\uff09\u2014\u2013\u2192]", comment)[0]
        lead = squeeze(lead)
        GROUP_WORDS = (
            "\u6d45\u79d1\u65e5\u5149", "\u83b1\u30ce\u7a7a", "\u6d5b\u30f6\u5cbd", "\u6d5c\u8ecd",
            "\u83b1\u30ce\u7a7a\u5973\u5b66\u9662", "Hello", "World", "generic", "member",
            "live", "card", "\u30e1\u30f3\u30d0\u30fc", "\u30e9\u30a4\u30d6",
        )
        lead_is_group = any(w in lead for w in GROUP_WORDS)
        # An abbreviation of a longer title is not a mismatch.
        is_prefix = bool(real_name) and (
            real_name.startswith(lead) or lead.startswith(real_name)
        )
        if real_name and lead and not lead_is_group and not is_prefix:
            if re.search(r"[\u4e00-\u9fff]", lead) and len(lead) >= 3:
                problems.append((
                    path.name, const_name, card_no, comment,
                    f"NAME MISMATCH: card is {real_name!r}, comment leads with {lead!r}",
                ))

        real_unit = squeeze(card.get("unit") or "")
        real_series = squeeze(card.get("series") or "")
        cmt = squeeze(comment)
        if "\u03bc's" in cmt and "\u30e9\u30d6\u30e9\u30a4\u30d6\uff01" not in real_series:
            problems.append((
                path.name, const_name, card_no, comment,
                f"GROUP MISMATCH: comment claims \u03bc's, series is {card.get('series')!r}",
            ))
        for g in UNIT_GROUPS:
            if not re.search(re.escape(g), cmt, re.I):
                continue
            # A NEGATED or disclaimed claim is the opposite of a mismatch: these files
            # are full of `NON_BIBI`/`Not lily white` controls, and flagging them makes
            # the audit cry wolf.
            ctx = cmt[max(0, cmt.lower().find(g.lower()) - 24): cmt.lower().find(g.lower()) + len(g) + 6]
            if re.search(r"not\b|non|≠|以外|ではない|,", ctx, re.I):
                break
            if real_unit != g:
                problems.append((
                    path.name, const_name, card_no, comment,
                    f"GROUP MISMATCH: comment claims {g!r}, unit is {card.get('unit')!r}",
                ))
            break

print(f"annotated constants checked: {checked}")
print(f"problems: {len(problems)}")
for p in problems:
    print(f"  {p[0]} ::{p[1]} {p[2]}")
    print(f"      comment: {p[3][:70]}")
    print(f"      -> {p[4]}")
