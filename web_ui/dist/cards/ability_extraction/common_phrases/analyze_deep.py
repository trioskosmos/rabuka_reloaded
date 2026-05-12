#!/usr/bin/env python3
"""
Deeper analysis of parser issues and ability patterns.
"""
import json, re, sys
from collections import Counter, defaultdict
from pathlib import Path

ABILITIES_FILE = Path(__file__).parent.parent.parent / "abilities.json"
with open(ABILITIES_FILE, encoding="utf-8") as f:
    data = json.load(f)

abilities = data["unique_abilities"]

def safe_eff(a):
    e = a.get("effect")
    return e if isinstance(e, dict) else {}

# ========== 1. Abilities that become "do_nothing" in sequential ==========
print("=== 1. Abilities with 'do_nothing' in effect tree ===")
for a in abilities:
    e = safe_eff(a)
    t = a.get("triggerless_text", "") or a.get("full_text", "")
    def find_do_nothing(d, path=""):
        if not isinstance(d, dict):
            return
        if d.get("action") == "do_nothing":
            print(f"  [{a.get('card_count','?')} cards] path={path} text={t[:120]}")
        for sub_key in ("actions", "options", "primary_effect", "alternative_effect"):
            sub = d.get(sub_key)
            if isinstance(sub, list):
                for i, item in enumerate(sub):
                    find_do_nothing(item, f"{path}.{sub_key}[{i}]")
            elif isinstance(sub, dict):
                find_do_nothing(sub, f"{path}.{sub_key}")
    find_do_nothing(e, "")

# ========== 2. Abilities with 代わりに (conditional_alternative) ==========
print()
print("=== 2. Conditional Alternative (代わりに) details ===")
for a in abilities:
    t = a.get("triggerless_text", "") or a.get("full_text", "")
    if "代わりに" in t:
        e = safe_eff(a)
        print(f"  [{a.get('card_count','?')} cards] action={e.get('action')} {t[:150]}")

# ========== 3. Abilities with それらがすべて ==========
print()
print("=== 3. 'それらがすべて' parser output ===")
for a in abilities:
    t = a.get("triggerless_text", "") or a.get("full_text", "")
    if "それらがすべて" in t:
        e = safe_eff(a)
        print(f"  [{a.get('card_count','?')} cards]")
        print(f"    text: {t[:120]}")
        print(f"    parsed: {json.dumps(e, ensure_ascii=False)[:300]}")
        print()

# ========== 4. Abilities with これにより ==========
print()
print("=== 4. 'これにより' parser output (sample) ===")
for a in abilities:
    t = a.get("triggerless_text", "") or a.get("full_text", "")
    if "これにより" in t and a.get("card_count", 0) >= 3:
        e = safe_eff(a)
        print(f"  [{a.get('card_count','?')} cards]")
        print(f"    text: {t[:120]}")
        print(f"    parsed action: {e.get('action')}")
        cond = e.get("condition", {})
        if cond:
            print(f"    condition type: {cond.get('type')}")
        print()

# ========== 5. Check results of そうした場合 parsing ==========
print()
print("=== 5. 'そうした場合' output action type ===")
sc = Counter()
for a in abilities:
    t = a.get("triggerless_text", "") or a.get("full_text", "")
    if "そうした場合" in t:
        e = safe_eff(a)
        sc[e.get("action")] += 1
for act, cnt in sc.most_common():
    print(f"  {cnt:3d} | {act}")

# ========== 6. Source/Destination coverage ==========
print()
print("=== 6. Source/Destination coverage in move_cards ===")
src_missing = 0
dst_missing = 0
both_ok = 0
for a in abilities:
    e = safe_eff(a)
    def check_move(d, path=""):
        global src_missing, dst_missing, both_ok
        if not isinstance(d, dict):
            return
        if d.get("action") == "move_cards":
            has_src = d.get("source") is not None
            has_dst = d.get("destination") is not None
            if has_src and has_dst:
                both_ok += 1
            else:
                if not has_src:
                    src_missing += 1
                if not has_dst:
                    dst_missing += 1
        for sub_key in ("actions", "options", "primary_effect", "alternative_effect", "select_action", "look_action", "opponent_action", "followup_action", "optional_action", "conditional_action"):
            sub = d.get(sub_key)
            if isinstance(sub, list):
                for item in sub:
                    check_move(item, f"{path}.{sub_key}[]")
            elif isinstance(sub, dict):
                check_move(sub, f"{path}.{sub_key}")
    check_move(e, "")
print(f"  Both source+destination: {both_ok}")
print(f"  Source missing: {src_missing}")
print(f"  Destination missing: {dst_missing}")

# ========== 7. Sequential(do_nothing, X) investigation ==========
print()
print("=== 7. Sequential with do_nothing investigation ===")
# These happen when a sentence boundary (。) splits text into parts
# and the first part parses as do_nothing
for a in abilities:
    t = a.get("triggerless_text", "") or a.get("full_text", "")
    e = safe_eff(a)
    if e.get("action") == "sequential":
        acts = e.get("actions", [])
        if acts and acts[0].get("action") == "do_nothing":
            print(f"  [{a.get('card_count','?')} cards] text={t[:120]}")

# ========== 8. Phrases that appear but aren't in common_phrases list ==========
print()
print("=== 8. Existing but unlisted phrases of interest ===")
# Check for "その場合" (て form of そうした場合)
check_phrases = [
    "その場合",
    "ある場合",
    "ない場合",
    "いた場合",
    "した場合",
    "含む",
    "すべて",
    "全て",
    "全員",
    "全体",
    "いずれか",
    "どちらか",
    "あるいは",
    "もしくは",
    "または",
    "のうち",
    "のいずれか",
    "かどうか",
    "によって",
    "により",
    "による",
    "に関して",
    "について",
    "に対して",
    "に関する",
    "における",
    "とした",
    "として",
    "を除く",
    "以外",
    "上",
    "下",
    "左",
    "右",
    "中",
    "前",
    "後",
    "隣",
    "端",
    "際",
]
for phrase in check_phrases:
    count = sum(1 for a in abilities for t in [(a.get("triggerless_text", "") or a.get("full_text", ""))] if phrase in t)
    if count:
        print(f"  {count:4d} | {phrase}")

print()
print("=== Done ===")
