#!/usr/bin/env python3
"""
Deep-dive into specific parser issues with full trace output.
"""
import json, sys, re, traceback
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))
from parser import (
    parse_effect, _try_period_conditional, _try_implicit_sequential,
    _try_kore_niyori_result, _try_conditional_sequential,
)

ABILITIES_FILE = Path(__file__).parent.parent.parent / "abilities.json"
with open(ABILITIES_FILE, encoding="utf-8") as f:
    data = json.load(f)

abilities = data["unique_abilities"]

# ===================== ISSUE 1: do_nothing artifacts =====================
print("=" * 70)
print("ISSUE 1: do_nothing artifacts - root cause analysis")
print("=" * 70)

# Collect texts that produce do_nothing
do_nothing_texts = []
for a in abilities:
    t = a.get("triggerless_text", "") or a.get("full_text", "")
    e = a.get("effect", {})
    if not isinstance(e, dict):
        continue

    def find_do_nothing(d, path=""):
        if not isinstance(d, dict):
            return []
        issues = []
        if d.get("action") == "do_nothing":
            issues.append((path, d.get("text", "")))
        for sub_key in ("actions", "options", "primary_effect", "alternative_effect"):
            sub = d.get(sub_key)
            if isinstance(sub, list):
                for i, item in enumerate(sub):
                    issues.extend(find_do_nothing(item, f"{path}.{sub_key}[{i}]"))
            elif isinstance(sub, dict):
                issues.extend(find_do_nothing(sub, f"{path}.{sub_key}"))
        return issues

    issues = find_do_nothing(e)
    for path, txt in issues:
        do_nothing_texts.append((t, path, txt))

print(f"Total do_nothing occurrences: {len(do_nothing_texts)}")
print()

# Show each one
for full_text, path, dn_text in do_nothing_texts:
    print(f"--- [{path}] ---")
    print(f"FULL:    {full_text[:120]}")
    print(f"DO_NOTHING TEXT: {repr(dn_text)}")
    # What is the text right before the period/where the split happens?
    # Show a more relevant portion
    if dn_text:
        idx = full_text.find(dn_text)
        if idx >= 0:
            start = max(0, idx - 30)
            end = min(len(full_text), idx + 60)
            print(f"CONTEXT: ...{full_text[start:end]}...")
    print()

# ===================== ISSUE 2: そうした場合 =====================
print("=" * 70)
print("ISSUE 2: so shita baai - conditional semantics lost")
print("=" * 70)

for a in abilities:
    t = a.get("triggerless_text", "") or a.get("full_text", "")
    if "そうした場合" not in t:
        continue
    e = a.get("effect", {})
    act = e.get("action")
    
    # Test: what does _try_conditional_sequential produce?
    result = _try_conditional_sequential(t)
    
    print(f"  Text: {t[:100]}")
    print(f"  Current: action={act}")
    
    # Check if the period-split is eating the そうした場合
    if "。" in t:
        period_pos = t.find("。")
        sou_pos = t.find("そうした場合")
        print(f"  period at {period_pos}, そうした場合 at {sou_pos}")
        if sou_pos > period_pos:
            print(f"  >>> Period BEFORE so shita baai - sequential split will eat it!")
            before_period = t[:period_pos]
            after_period = t[period_pos+1:]
            print(f"  BEFORE 。: {before_period[:80]}")
            print(f"  AFTER  。: {after_period[:80]}")
        else:
            # そうした場合 is in the first sentence — _try_conditional_sequential should catch it
            if result:
                print(f"  _try_conditional_sequential result: {result.get('action')} - would work but handler priority?")
    print()

# ===================== ISSUE 3: _try_kore_niyori_result do_nothing =====================
print("=" * 70)
print("ISSUE 3: kore_niyori_result do_nothing primary")
print("=" * 70)

for a in abilities:
    t = a.get("triggerless_text", "") or a.get("full_text", "")
    e = a.get("effect", {})
    if e.get("action") != "conditional_on_result":
        continue
    pe = e.get("primary_effect", {})
    if pe.get("action") == "do_nothing":
        print(f"  Text: {t[:120]}")
        print(f"  primary_effect text: {repr(pe.get('text', ''))}")
        print(f"  followup_action: {e.get('followup_action', {}).get('action')}")
        print(f"  result_condition: {e.get('result_condition', {})}")
        print()

# ===================== ISSUE 4: parse_cost debug =====================
print("=" * 70)
print("ISSUE 4: parse_cost debug")
print("=" * 70)

# Check parser.py around line 2196
with open(Path(__file__).parent.parent / "parser.py", encoding="utf-8") as f:
    lines = f.readlines()
for i, line in enumerate(lines, 1):
    if "DEBUG: parse_cost" in line:
        print(f"  Line {i}: {line.rstrip()}")
        # Show surrounding context
        for j in range(max(0, i-3), min(len(lines), i+3)):
            print(f"    {j+1}: {lines[j].rstrip()}")

print()
print("All issues examined.")
