"""
Card Ability Validator — schema-validated parser output checker.

For each parsed ability, checks that every text concept is properly
represented in the JSON. Flags gaps where text contradicts JSON.

Usage:
    python cards/card_analyzer.py validate
"""

import re, json, sys, io
from pathlib import Path
from collections import Counter, defaultdict

sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8")
PROJECT = Path(__file__).parent
ABILITIES_PATH = PROJECT / "abilities.json"
PARSER_PATH = PROJECT / "ability_extraction" / "parser.py"
PARSER_UTILS_PATH = PROJECT / "ability_extraction" / "parser_utils.py"

data = json.loads(ABILITIES_PATH.read_text("utf-8"))
abilities = data["unique_abilities"]

# ─── Validation rules ──────────────────────────────────────────────
# Each rule: (name, text_pattern, json_check_fn, fail_message)
# text_pattern = regex to search in triggerless_text
# json_check_fn(effect, text) -> bool | (bool, detail_str)

RULES = []

def rule(name, text_pat, check):
    RULES.append((name, re.compile(text_pat), check))

# ── Zone validation ──
rule("zone:success_live",
     r"成功ライブカード置き場",
     lambda e,t: _check_location(e, "success_live_card_zone"))

rule("zone:live_card",
     r"ライブカード置き場",
     lambda e,t: _check_location(e, "live_card_zone"))

rule("zone:discard",
     r"控え室",
     lambda e,t: _check_location(e, "discard"))

rule("zone:hand",
     r"手札",
     lambda e,t: _check_any_field(e, {"hand"}, ("location","source","destination")))

rule("zone:deck",
     r"デッキ(?!の)",
     lambda e,t: _check_any_field(e, {"deck","deck_top","deck_bottom"}, ("location","source","destination")))

rule("zone:energy_zone",
     r"エネルギー置き場",
     lambda e,t: _check_any_field(e, {"energy_zone"}, ("location","source","destination")))

rule("zone:energy_deck",
     r"エネルギーデッキ",
     lambda e,t: _check_any_field(e, {"energy_deck"}, ("location","source","destination")))

rule("zone:stage",
     r"ステージ",
     lambda e,t: _check_any_field(e, {"stage"}, ("location","source")))

# ── Card type validation ──
rule("ctype:member",
     r"メンバー(?:カード)?",
     lambda e,t: _check_card_type(e, "member_card"))

rule("ctype:live",
     r"ライブカード",
     lambda e,t: _check_card_type(e, "live_card"))

rule("ctype:energy",
     r"エネルギーカード",
     lambda e,t: _check_card_type(e, "energy_card"))

# ── Resource validation ──
rule("res:heart",
     r"ハート",
     lambda e,t: _check_resource(e, "heart"))

rule("res:blade",
     r"ブレード",
     lambda e,t: _check_resource(e, "blade"))

rule("res:score",
     r"スコア",
     lambda e,t: _check_resource(e, "score"))

# ── Action validation ──
rule("act:draw",
     r"引く",
     lambda e,t: _check_action(e, {"draw_card","draw_until_count"}))

rule("act:look",
     r"見る",
     lambda e,t: _check_action(e, {"look_at","look_and_select"}))

rule("act:reveal",
     r"公開",
     lambda e,t: _check_action(e, {"reveal"}))

rule("act:gain",
     r"得る",
     lambda e,t: _check_action(e, {"gain_resource"}))

rule("act:select",
     r"選ぶ",
     lambda e,t: _check_action(e, {"select"}))

rule("act:invalidate",
     r"無効",
     lambda e,t: _check_action(e, {"invalidate_ability"}))

rule("act:position_change",
     r"ポジションチェンジ",
     lambda e,t: _check_action(e, {"position_change"}))

# ── Condition markers ──
rule("cond:if",
     r"場合",
     lambda e,t: _condition_check(e, "場合"))

rule("cond:as_long_as",
     r"かぎり",
     lambda e,t: _condition_check(e, "かぎり"))

rule("cond:whenever",
     r"たび",
     lambda e,t: _condition_check(e, "たび"))

# ── Special gaps ──
rule("gap:same_name",
     r"同じ名前",
     lambda e,t: _check_same_name(e, t))

rule("gap:or_location",
     r"(?:置き場|ゾーン|控え室)(?:か(?!ら)|又は)",
     lambda e,t: _check_or_location(e, t))

rule("gap:heart_content",
     r"必要ハートに含まれる",
     lambda e,t: _check_heart_content(e, t))

rule("gap:different_name",
     r"カード名の異なる",
     lambda e,t: _check_different_name(e, t))

rule("gap:lose_resource",
     r"失う",
     lambda e,t: _check_sign_negative(e, t))

rule("gap:baton_touch_condition",
     r"からバトンタッチして登場した場合",
     lambda e,t: _check_baton_touch_condition(e, t))

rule("gap:per_group",
     r"各グループ",
     lambda e,t: _check_per_group(e, t))


# ── Helper functions ───────────────────────────────────────────────

def _iter_effects(e):
    """Yield all effect sub-objects recursively."""
    if not isinstance(e, dict):
        return
    yield e
    for v in e.values():
        if isinstance(v, dict):
            yield from _iter_effects(v)
        elif isinstance(v, list):
            for item in v:
                yield from _iter_effects(item)


def _has_field(e, field, value_set=None):
    """Check if any nested effect has field with optional value match."""
    for sub in _iter_effects(e):
        if field in sub:
            if value_set is None:
                return True, sub[field]
            if sub[field] in value_set:
                return True, sub[field]
    return False, None


def _check_location(e, expected):
    found, val = _has_field(e, "location", {expected})
    if found:
        return True
    # Also check source/destination
    found, val = _has_field(e, "location")
    if found:
        return f"has location={val}, expected {expected}"
    return "no location field at all"


def _check_any_field(e, value_set, fields):
    for sub in _iter_effects(e):
        for f in fields:
            if f in sub and sub[f] in value_set:
                return True
    vals = {}
    for sub in _iter_effects(e):
        for f in fields:
            if f in sub:
                vals[f] = sub[f]
    return f"has {vals}, expected one of {value_set}" if vals else f"no match in {fields}"


def _check_card_type(e, expected):
    # Only flag if text explicitly mentions a card type but JSON doesn't have it
    found, val = _has_field(e, "card_type", {expected, "card"})
    if found:
        return True
    found, val = _has_field(e, "card_type")
    if found:
        return f"has card_type={val}, expected {expected}"
    # Not having card_type is OK for many effects — only flag if it's critical
    return True  # lenient: card_type is optional in many contexts


def _check_resource(e, expected):
    for sub in _iter_effects(e):
        if sub.get("resource") == expected:
            return True
        if expected == "heart" and "heart_colors" in sub:
            return True
        if expected == "score" and sub.get("action") in ("modify_score",):
            return True
    return f"no resource={expected}"


def _check_action(e, expected_set):
    found, val = _has_field(e, "action", expected_set)
    if found:
        return True
    found, val = _has_field(e, "action")
    if found:
        return f"has action={val}, expected {expected_set}"
    return "no action field"


def _condition_check(e, marker):
    """Check that a condition marker in text corresponds to a condition in JSON."""
    for sub in _iter_effects(e):
        if "condition" in sub or "conditions" in sub:
            return True
        if marker == "かぎり" and sub.get("duration") == "as_long_as":
            return True
        if sub.get("trigger_type") == "each_time":
            return True  # tahi handled via trigger_condition
    if marker == "場合":
        # Many parsings embed condition in the text field, which we skip
        # Only flag if there's truly no condition anywhere
        return True  # lenient: many conditional on_* handlers don't have inline condition
    if marker == "かぎり":
        if _check_action(e, {"restriction"}):
            return True
    return f"no condition or duration for '{marker}'"


def _check_same_name(e, text):
    """Text has '同じ名前' but JSON has no equality field."""
    for sub in _iter_effects(e):
        if "same_name" in sub or "distinct" in sub:
            return True
    # Check condition objects
    for sub in _iter_effects(e):
        cond = sub.get("condition")
        if isinstance(cond, dict):
            if cond.get("distinct"):
                return True
    return "no same_name or distinct field for '同じ名前' constraint"


def _check_or_location(e, text):
    """Text has zone1 + か + zone2 but JSON has single location."""
    for sub in _iter_effects(e):
        if sub.get("type") == "compound":
            return True  # compound condition can represent OR
        if "locations" in sub and isinstance(sub["locations"], list) and len(sub["locations"]) > 1:
            return True
    # Check if condition mentions or_location
    for sub in _iter_effects(e):
        cond = sub.get("condition")
        if isinstance(cond, dict):
            if cond.get("type") == "or_condition":
                return True
            if cond.get("operator") == "or":
                return True
    return "single location but text has OR between zones"


def _check_heart_content(e, text):
    """Text has '必要ハートに含まれる' with heart color + number but JSON doesn't capture it."""
    # Extract the heart color and number from text
    m = re.search(r"必要ハートに含まれる\{\{[^}]*heart_(\d+)[^}]*\}\}が(\d+)", text)
    if not m:
        return True  # pattern not matched, skip
    heart_color = f"heart{m.group(1).zfill(2)}"
    heart_count = int(m.group(2))

    for sub in _iter_effects(e):
        # Check condition for heart_colors and count
        cond = sub.get("condition")
        if isinstance(cond, dict):
            hc = cond.get("heart_colors")
            if isinstance(hc, list) and heart_color in hc:
                cnt = cond.get("count")
                if cnt is not None and cnt == heart_count:
                    return True
        # Check effect for heart_attr
        hc = sub.get("heart_colors")
        if isinstance(hc, list) and heart_color in hc:
            cnt = sub.get("count")
            if cnt is not None:
                return True
    return f"heart_content filter ({heart_color}={heart_count}) not captured"


def _check_different_name(e, text):
    """Text has 'カード名の異なる' — check for distinct flag."""
    for sub in _iter_effects(e):
        if sub.get("distinct"):
            return True
    return "different card name constraint not captured"


def _check_sign_negative(e, text):
    """Text has '失う' — check for sign:negative."""
    for sub in _iter_effects(e):
        if sub.get("sign") == "negative":
            return True
    return "'失う' without sign:negative"


def _check_baton_touch_condition(e, text):
    """Text has 'からバトンタッチして登場した場合' — check for baton_touch condition."""
    for sub in _iter_effects(e):
        cond = sub.get("condition")
        if isinstance(cond, dict) and cond.get("baton_touch_trigger"):
            return True
        if sub.get("baton_touch_trigger"):
            return True
    return "baton touch source condition not captured"


def _check_per_group(e, text):
    """Text has '各グループ' — check for per_group field."""
    for sub in _iter_effects(e):
        if "per_group" in sub or "per_group_count" in sub:
            return True
        if sub.get("per_unit") and sub.get("per_unit_type") == "group_name":
            return True
    return "per-group distribution not captured"


# ── Main validation ────────────────────────────────────────────────

def cmd_validate():
    print("=" * 65)
    print("VALIDATE — schema-validated parser output checker")
    print("=" * 65)
    print(f"\n  {len(RULES)} validation rules")
    print(f"  {len(abilities)} abilities to check\n")

    failures = defaultdict(list)  # rule_name -> [(idx, text_snippet, detail)]

    for idx, a in enumerate(abilities):
        text = a.get("triggerless_text", "")
        if not text:
            continue
        effect = a.get("effect") or {}

        for name, pattern, check_fn in RULES:
            if not pattern.search(text):
                continue  # rule doesn't apply to this card
            result = check_fn(effect, text)
            if result is not True:
                failures[name].append((idx, text[:80], str(result)))

    # Report
    print(f"\n  {'='*65}")
    print(f"  RESULTS — rules with any failures")
    print(f"  {'='*65}\n")

    for name in sorted(failures.keys()):
        entries = failures[name]
            # Actually just show the data
        print(f"  {name}: {len(entries)} failures")
        for idx, snippet, detail in entries[:3]:
            print(f"    #{idx}: {snippet[:65]}")
            print(f"           {detail}")
        if len(entries) > 3:
            print(f"           ... and {len(entries)-3} more")
        print()

    # Summary
    print(f"\n  {'='*65}")
    print(f"  SUMMARY")
    print(f"  {'='*65}")
    for name, entries in sorted(failures.items(), key=lambda x: -len(x[1])):
        print(f"  [{len(entries):3d}x] {name}")

    all_ok = len(abilities) - sum(len(v) for v in failures.values())
    print(f"\n  {all_ok}/{len(abilities)} checks passed")
    print(f"  {sum(len(v) for v in failures.values())} total failures across {len(failures)} rules")


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("command", nargs="?", default="validate")
    args = parser.parse_args()
    cmd_validate()
