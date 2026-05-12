#!/usr/bin/env python3
"""
Test the parser directly on specific ability texts to check for issues.
Focuses on patterns that cause problems or need improvement.
"""
import json, sys, re
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))
from parser import parse_ability, parse_effect, parse_condition, parse_action, _try_implicit_sequential, strip_suffix_period

ABILITIES_FILE = Path(__file__).parent.parent.parent / "abilities.json"
with open(ABILITIES_FILE, encoding="utf-8") as f:
    data = json.load(f)

abilities = data["unique_abilities"]

def show(text, label="Input"):
    print(f">>> {label}: {text[:100]}")
    print(f"    parse_ability: {json.dumps(parse_ability(text), ensure_ascii=False, indent=2)[:500]}")
    print()

# ========== 1. Test それらがすべて pattern ==========
print("=" * 60)
print("1. それらがすべて (All-of-them) pattern")
print("=" * 60)
for a in abilities:
    t = a.get("triggerless_text", "") or a.get("full_text", "")
    if "それらがすべて" in t:
        show(t)
        break

# ========== 2. Test do_nothing causing texts ==========
print("=" * 60)
print("2. Tests that produce do_nothing due to period splitting")
print("=" * 60)
# These are texts where the first part before 。is all condition
test_texts = [
    "このターン、自分のエールにより公開された自分のカードの中にブレードハートを持たないメンバーカードが3枚以上ある場合、ライブ終了時まで、{{icon_all.png|ハート}}を得る。",
    "相手のライブカード置き場にカードが置かれた場合、ライブ終了時まで、{{icon_blade.png|ブレード}}を得る。",
]
for t in test_texts:
    # Check what _try_implicit_sequential does
    result = parse_ability(t)
    eff = result.get("effect", {})
    print(f"  Input: {t[:80]}...")
    print(f"  Action: {eff.get('action')}")
    if eff.get("action") == "sequential":
        acts = eff.get("actions", [])
        for i, a in enumerate(acts):
            print(f"    [{i}] action={a.get('action')} text={a.get('text', '')[:60]}")
    print()

# ========== 3. Test そうした場合 (conditional sequential) ==========
print("=" * 60)
print("3. そうした場合 (conditional sequential) details")
print("=" * 60)
for a in abilities:
    t = a.get("triggerless_text", "") or a.get("full_text", "")
    if "そうした場合" in t:
        eff = a.get("effect", {})
        print(f"  [{a.get('card_count','?')} cards]")
        print(f"    Input: {t[:100]}")
        print(f"    Parsed action: {eff.get('action')}")
        if eff.get("action") == "sequential":
            acts = eff.get("actions", [])
            for i, act in enumerate(acts):
                label = f"action_{i}"
                print(f"    [{i}] action={act.get('action')} text={act.get('text','')[:60]}")
        elif eff.get("action") == "conditional_on_optional":
            opt = eff.get("optional_action", {})
            cond = eff.get("conditional_action", {})
            print(f"    optional action: {opt.get('action')}")
            print(f"    conditional action: {cond.get('action')}")
        elif eff.get("action") == "conditional_on_result":
            pe = eff.get("primary_effect", {})
            fe = eff.get("followup_action", {})
            print(f"    primary_effect: {pe.get('action')}")
            print(f"    followup: {fe.get('action')}")
        print()

# ========== 4. Test これにより pattern ==========
print("=" * 60)
print("4. これにより (as a result) details")
print("=" * 60)
for a in abilities:
    t = a.get("triggerless_text", "") or a.get("full_text", "")
    if "これにより" in t and a.get("card_count", 0) >= 4:
        eff = a.get("effect", {})
        print(f"  [{a.get('card_count','?')} cards]")
        print(f"    Input: {t[:120]}")
        print(f"    action: {eff.get('action')}")
        if eff.get("action") == "conditional_on_result":
            pe = eff.get("primary_effect", {})
            fe = eff.get("followup_action", {})
            rc = eff.get("result_condition")
            print(f"    primary_effect: {pe.get('action')} text={pe.get('text','')[:60]}")
            print(f"    followup: {fe.get('action')} text={fe.get('text','')[:60]}")
            print(f"    result_condition: {rc.get('text','')[:60] if rc else None}")
        print()

# ========== 5. Test 代わりに (conditional_alternative) ==========
print("=" * 60)
print("5. 代わりに (instead) details")
print("=" * 60)
for a in abilities:
    t = a.get("triggerless_text", "") or a.get("full_text", "")
    if "代わりに" in t and a.get("card_count", 0) >= 2:
        eff = a.get("effect", {})
        print(f"  [{a.get('card_count','?')} cards]")
        print(f"    Input: {t[:120]}")
        print(f"    action: {eff.get('action')}")
        if eff.get("action") == "conditional_alternative":
            pe = eff.get("primary_effect", {})
            ae = eff.get("alternative_effect", {})
            print(f"    primary: {pe.get('action')} text={pe.get('text','')[:60]}")
            print(f"    alternative: {ae.get('action')}")
        print()

# ========== 6. Parse cost patterns ==========
print("=" * 60)
print("6. Cost parsing samples")
print("=" * 60)
cost_texts = [
    "手札を1枚控え室に置く：",  # discard from hand
    "{{icon_energy.png|E}}{{icon_energy.png|E}}：",  # energy cost
    "手札の『Aqours』のメンバーカード1枚を控え室に置く：",  # specific discard
    "このメンバーをウェイトにする：",  # wait cost
    "自分のメンバー1人をウェイトにする：",  # wait cost with target
]
for ct in cost_texts:
    print(f"  Cost: {ct}")
    ap = parse_ability(ct + "カードを1枚引く。")
    cost = ap.get("cost", {})
    print(f"    type: {cost.get('type')}")
    print(f"    source: {cost.get('source')} dest: {cost.get('destination')}")
    print()

print("Done.")
