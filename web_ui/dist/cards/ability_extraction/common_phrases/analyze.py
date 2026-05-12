#!/usr/bin/env python3
"""
Analyze common phrases in ability texts from abilities.json
and identify patterns/issues in parser.py.
"""

import json
import re
from collections import Counter, defaultdict
from pathlib import Path

ABILITIES_FILE = Path(__file__).parent.parent.parent / "abilities.json"

with open(ABILITIES_FILE, encoding="utf-8") as f:
    data = json.load(f)

unique_abilities = data["unique_abilities"]
total = len(unique_abilities)
print(f"Total unique abilities: {total}")
print()

# Collect all triggerless_text
texts = [a.get("triggerless_text", "") or a.get("full_text", "") for a in unique_abilities]
texts = [t for t in texts if t]

# ============ 1. Find common phrase patterns ============
# Common Japanese phrases used in abilities
common_phrases = [
    "それらがすべて",
    "それらのカード",
    "これにより",
    "そうした場合",
    "そうしなかった場合",
    "その中から",
    "その後、",
    "場合、",
    "とき、",
    "なら、",
    "かつ",
    "か、",
    "または",
    "かつ",
    "代わりに",
    "さらに",
    "につき",
    "たび",
    "かぎり",
    "いずれかの場合",
    "それぞれ",
    "ずつ",
    "すべての",
    "全ての",
    "好きな順番で",
    "好きな枚数",
    "任意の枚数",
    "合計",
    "ちょうど",
    "同じ",
    "異なる",
    "以上",
    "以下",
    "未満",
    "超",
    "のみ",
    "もよい",
    "てもよい",
    "支払わないかぎり",
    "相手は",
    "自分か相手",
    "自分と相手",
    "回答が",
    "バトンタッチ",
    "ポジションチェンジ",
    "フォーメーションチェンジ",
    "エマパンチ",
    "何もしない",
    "無効に",
    "アクティブにする",
    "ウェイトにする",
    "シャッフル",
    "入れ替える",
    "ライブ中",
    "このターン",
    "このライブ",
    "ライブ終了時まで",
    "ターン終了時まで",
    "エール",
    "公開する",
    "公開された",
    "引く",
    "引き",
    "見る",
    "見て",
    "選ぶ",
    "選ん",
    "選び",
    "加える",
    "加え",
    "置く",
    "置いて",
    "置き",
    "登場",
    "移動",
    "を得る",
    "を失う",
    "同じことを行う",
    "能力を発動",
    "起動できる",
    "発動する",
    "発動させる",
]

print("=== Common Phrase Frequency ===")
phrase_counts = {}
for phrase in common_phrases:
    count = sum(1 for t in texts if phrase in t)
    if count > 0:
        phrase_counts[phrase] = count

for phrase, count in sorted(phrase_counts.items(), key=lambda x: -x[1]):
    print(f"  {count:5d} | {phrase}")

print()

# ============ 2. Find abilities that have specific "troublesome" patterns ============
print("=== Abilities with 'それらがすべて' pattern ===")
for a in unique_abilities:
    t = a.get("triggerless_text", "") or a.get("full_text", "")
    if "それらがすべて" in t:
        print(f"  [{a.get('card_count', '?')} cards] {t[:120]}")

print()
print("=== Abilities with 'それらのカード' pattern ===")
for a in unique_abilities:
    t = a.get("triggerless_text", "") or a.get("full_text", "")
    if "それらのカード" in t:
        print(f"  [{a.get('card_count', '?')} cards] {t[:120]}")

print()
print("=== Abilities with 'そうした場合' pattern ===")
for a in unique_abilities:
    t = a.get("triggerless_text", "") or a.get("full_text", "")
    if "そうした場合" in t:
        e = a.get("effect", {})
        act = e.get("action", "?")
        print(f"  [{act}] [{a.get('card_count', '?')} cards] {t[:120]}")

print()
print("=== Abilities with '代わりに' pattern ===")
for a in unique_abilities:
    t = a.get("triggerless_text", "") or a.get("full_text", "")
    if "代わりに" in t:
        e = a.get("effect", {})
        act = e.get("action", "?")
        print(f"  [{act}] [{a.get('card_count', '?')} cards] {t[:120]}")

print()
print("=== Abilities with 'エマパンチ' pattern ===")
for a in unique_abilities:
    t = a.get("triggerless_text", "") or a.get("full_text", "")
    if "エマパンチ" in t:
        e = a.get("effect", {})
        act = e.get("action", "?")
        print(f"  [{act}] [{a.get('card_count', '?')} cards] {t[:120]}")

print()
print("=== Abilities with '無効に' pattern ===")
for a in unique_abilities:
    t = a.get("triggerless_text", "") or a.get("full_text", "")
    if "無効に" in t:
        e = a.get("effect", {})
        act = e.get("action", "?")
        print(f"  [{act}] [{a.get('card_count', '?')} cards] {t[:120]}")

# ============ 3. Find effect types distribution ============
print()
print("=== Effect Type Distribution ===")
effect_types = Counter()
for a in unique_abilities:
    e = a.get("effect", {})
    if not e:
        effect_types["(no effect)"] += 1
        continue
    act = e.get("action", "?") if isinstance(e, dict) else "?"
    # Check for nested sequential
    if act == "sequential":
        sub_acts = [s.get("action", "?") for s in e.get("actions", []) if isinstance(s, dict)]
        act = f"sequential({','.join(sub_acts[:3])})"
    effect_types[act] += 1

for act, count in effect_types.most_common(30):
    print(f"  {count:5d} | {act}")

# ============ 4. Check for parsing issues ============
print()
print("=== Potential Parsing Issues ===")

# Check for custom action types
def safe_eff(a):
    e = a.get("effect")
    return e if isinstance(e, dict) else {}

custom_count = sum(1 for a in unique_abilities if safe_eff(a).get("action") == "custom")
print(f"  Custom (unparsed) actions: {custom_count} / {total}")

# Check for is_null
null_count = sum(1 for a in unique_abilities if a.get("is_null"))
print(f"  Null abilities: {null_count} / {total}")

# Abilities with empty effect but not null
empty_effect = sum(1 for a in unique_abilities if not a.get("is_null") and not a.get("effect"))
print(f"  Non-null with no effect: {empty_effect} / {total}")

# Check for effect with no action
no_action = sum(1 for a in unique_abilities if isinstance(a.get("effect"), dict) and not a["effect"].get("action") and not a["effect"].get("actions"))
print(f"  Effect with no action/actions: {no_action} / {total}")

# Custom action details
print()
print("=== Custom (unparsed) actions ===")
for a in unique_abilities:
    e = safe_eff(a)
    if e.get("action") == "custom":
        t = a.get("triggerless_text", "") or a.get("full_text", "")
        print(f"  [{a.get('card_count', '?')} cards] {t[:130]}")

# ============ 5. Find longest/shortest abilities ============
print()
print("=== Unique Card Count Distribution ===")
cc_buckets = Counter()
for a in unique_abilities:
    cc = a.get("card_count", 0)
    if cc == 0: b = "0"
    elif cc == 1: b = "1"
    elif cc <= 3: b = "2-3"
    elif cc <= 5: b = "4-5"
    elif cc <= 10: b = "6-10"
    elif cc <= 20: b = "11-20"
    else: b = "21+"
    cc_buckets[b] += 1
for b in ["0", "1", "2-3", "4-5", "6-10", "11-20", "21+"]:
    print(f"  {b:>5} cards: {cc_buckets[b]}")

print()
print("=== Done ===")
