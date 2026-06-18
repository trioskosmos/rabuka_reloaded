"""
Cross-reference 自動 trigger types with test coverage - CORRECTED.
We search for distinctive ability-text fragments in test files, not full card IDs.
An ability is "tested" if its trigger mechanism or distinctive effect text appears in a test.
"""
import os
import re
import json
from collections import defaultdict

ENGINE_TESTS_DIR = r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine\tests\test_modules"

with open(r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\abilities.json", encoding="utf-8") as f:
    data = json.load(f)

auto_abilities = [ab for ab in data["unique_abilities"] if ab.get("triggers") == "自動"]

# --- Gather test file contents ---
test_files = [f for f in os.listdir(ENGINE_TESTS_DIR) if f.endswith(".rs")]
file_contents = {}
for tf in test_files:
    with open(os.path.join(ENGINE_TESTS_DIR, tf), encoding="utf-8") as f:
        file_contents[tf] = f.read()


def find_distinctive_fragments(full_text, triggerless_text):
    """
    Extract distinctive fragments from the ability text that would appear in a test
    if the ability were exercised. We want fragments that are unique to this ability
    so a match means this specific ability is being tested.
    """
    text = triggerless_text

    # Strip image markers from the search target - tests typically reference
    # the rendered text or use the card ID / character name
    fragments = set()

    # 1. Distinctive multi-character Japanese phrases from the EFFECT portion
    # (after the trigger clause). These are the parts a test author would
    # quote or comment when testing that ability.

    # Remove common trigger prefixes to isolate the effect body
    body = text
    # Cut at the first とき、 or たび、 to get past the trigger clause
    for sep in ["たび、", "とき、", "場合、"]:
        idx = body.find(sep)
        if idx >= 0:
            body = body[idx + len(sep):]
            break

    # Extract noun phrases that are likely unique: numbers + nouns, named groups, etc.
    # Pattern: sequences of 4+ CJK chars not part of common trigger words
    common_words = {
        "このメンバー", "このカード", "自分の", "相手の", "ステージ", "控え室", "手札",
        "ライブ終了時まで", "ライブカード", "メンバーカード", "エネルギーカード",
        "アクティブ", "ウェイト", "エリアを移動", "ステージから控え室",
        "エールにより公開", "ブレードハート", "このターン",
    }

    # Named groups like 『虹ヶ咲』『Aqours』『蓮ノ空』『EdelNote』『スリーズブーケ』『μ's』『Liella!』
    for m in re.finditer(r"『([^』]+)』", text):
        fragments.add(m.group(1))

    # Specific action phrases (5+ chars)
    for m in re.finditer(r"[一-龥ぁ-んァ-ンー]{4,}", body):
        phrase = m.group(0)
        if phrase not in common_words and len(phrase) >= 5:
            # skip if it's a substring of a common word
            if not any(cw in phrase or phrase in cw for cw in common_words):
                fragments.add(phrase)

    return fragments


def ability_tested_in(ab, file_contents):
    """
    Determine if this ability is exercised in any test file.
    Returns list of (filename, matched_fragment) tuples.
    """
    matches = []
    # Strategy: search for the card IDs (all variants) AND distinctive text
    ft = ab["full_text"]
    tt = ab["triggerless_text"]

    # 1. Check all card IDs for this ability
    for card_ref in ab["cards"]:
        card_id = card_ref.split(" (ab#")[0]
        for tf, content in file_contents.items():
            # Try with and without the ＋ suffix variants
            variants = [card_id, card_id.replace("＋", "+")]
            for v in variants:
                if v in content:
                    matches.append((tf, f"card_id:{v}"))
                    break

    # 2. Check distinctive fragments
    fragments = find_distinctive_fragments(ft, tt)
    for tf, content in file_contents.items():
        for frag in fragments:
            if frag in content:
                matches.append((tf, f"text:{frag}"))

    # Dedupe
    seen = set()
    unique = []
    for tf, why in matches:
        key = (tf, why)
        if key not in seen:
            seen.add(key)
            unique.append((tf, why))
    return unique


# --- Build coverage ---
results = []
for ab in auto_abilities:
    matches = ability_tested_in(ab, file_contents)
    results.append({
        "ability": ab,
        "matches": matches,
        "tested": len(matches) > 0,
    })

tested_count = sum(1 for r in results if r["tested"])
print(f"Tested abilities: {tested_count}/{len(auto_abilities)}")
print(f"Untested abilities: {len(auto_abilities) - tested_count}/{len(auto_abilities)}\n")

# Print per-ability
for r in results:
    ab = r["ability"]
    card_id = ab["cards"][0].split(" (ab#")[0]
    status = "TESTED  " if r["tested"] else "UNTESTED"
    print(f"{status} {card_id} (n={ab['card_count']})")
    if r["tested"]:
        for tf, why in r["matches"][:5]:
            print(f"           -> {tf} ({why})")
    print(f"           text: {ab['triggerless_text'][:70]}...")
    print()
