"""
Tightened coverage analysis. Real test signal = actual card IDs referenced
in tests (via game.id() or string literals), plus distinctive effect phrases.
"""
import os
import re
import json
from collections import defaultdict

ENGINE_TESTS_DIR = r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine\tests\test_modules"

with open(r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\abilities.json", encoding="utf-8") as f:
    data = json.load(f)

auto_abilities = [ab for ab in data["unique_abilities"] if ab.get("triggers") == "自動"]

test_files = [f for f in os.listdir(ENGINE_TESTS_DIR) if f.endswith(".rs")]
file_contents = {}
for tf in test_files:
    with open(os.path.join(ENGINE_TESTS_DIR, tf), encoding="utf-8") as f:
        file_contents[tf] = f.read()

# --- Build the set of all card IDs that exist in the DB ---
# so we can detect when ANY card sharing this ability is referenced
all_card_ids = set()
for ab in data["unique_abilities"]:
    for card_ref in ab["cards"]:
        cid = card_ref.split(" (ab#")[0]
        all_card_ids.add(cid)
        if cid.endswith("＋"):
            all_card_ids.add(cid.replace("＋", "+"))

# Generic fragments that are too common to be meaningful signal
GENERIC = {
    "そうした場合", "このメンバー", "このカード", "自分の", "相手の", "ステージ",
    "控え室", "手札", "ライブ終了時まで", "ライブカード", "メンバーカード",
    "エネルギーカード", "アクティブ", "ウェイト", "エリアを移動",
    "ステージから控え室", "エールにより公開", "ブレードハート", "このターン",
    "エネルギーを", "カードを1枚引", "枚以上ある場合", "以下のメンバー",
    "つ以下のメンバー", "を持たない場合", "これにより", "そのメンバーが",
    "そのメンバーは", "この能力では", "カードを2枚引き",
}
# Group names: match counts ONLY if combined with a specific effect, not alone
GROUP_NAMES = {"虹ヶ咲", "蓮ノ空", "Aqours", "μ's", "EdelNote", "Liella!", "スリーズブーケ"}


def get_distinctive_phrases(text):
    """Phrases long & specific enough to be real signal."""
    phrases = set()
    for m in re.finditer(r"[一-龥ぁ-んァ-ンーa-zA-Z0-9！！?]{6,}", text):
        p = m.group(0)
        if p in GENERIC:
            continue
        if any(g in p for g in GROUP_NAMES):
            continue  # we'll handle group names separately
        # skip if it's a substring of a generic word
        if any(p in g for g in GENERIC):
            continue
        phrases.add(p)
    return phrases


def ability_matches(ab):
    """Return (tested: bool, evidence: list). Use strict signals."""
    evidence = []
    tt = ab["triggerless_text"]

    # Signal 1: any card ID for this ability appears as a literal in a test
    for card_ref in ab["cards"]:
        cid = card_ref.split(" (ab#")[0]
        for tf, content in file_contents.items():
            variants = [cid, cid.replace("＋", "+")]
            for v in variants:
                if v in content:
                    evidence.append((tf, f"card_id:{v}"))
                    break

    # Signal 2: distinctive effect phrases (6+ chars, non-generic)
    # Require the FULL phrase to appear, and it must be genuinely distinctive
    # (not appearing in many other abilities)
    phrases = get_distinctive_phrases(tt)
    # Filter phrases that appear in many abilities (low specificity)
    for p in list(phrases):
        occurrences = sum(1 for ab2 in auto_abilities if p in ab2["triggerless_text"])
        if occurrences > 2:  # appears in too many auto abilities
            phrases.discard(p)

    for tf, content in file_contents.items():
        for p in phrases:
            if p in content:
                evidence.append((tf, f"phrase:{p}"))

    # Signal 3: group name + this ability's signature effect together
    for tf, content in file_contents.items():
        for g in GROUP_NAMES:
            if g in tt and g in content:
                # require also a specific nearby phrase from THIS ability
                # check the effect body after trigger
                body = tt
                for sep in ["たび、", "とき、", "場合、"]:
                    i = body.find(sep)
                    if i >= 0:
                        body = body[i + len(sep):]
                        break
                # find a specific 4+ char run in the body
                for m in re.finditer(r"[一-龥ぁ-んァ-ンー]{4,}", body):
                    bp = m.group(0)
                    if bp not in GENERIC and g in content and bp in content:
                        evidence.append((tf, f"group+phrase:{g}+{bp}"))
                        break

    # dedupe
    seen = set()
    out = []
    for tf, why in evidence:
        if (tf, why) not in seen:
            seen.add((tf, why))
            out.append((tf, why))
    return out


tested = 0
untested_list = []
for ab in auto_abilities:
    ev = ability_matches(ab)
    cid = ab["cards"][0].split(" (ab#")[0]
    if ev:
        tested += 1
        print(f"TESTED   {cid} (n={ab['card_count']})")
        for tf, why in ev[:3]:
            print(f"            {tf} ({why})")
    else:
        untested_list.append(ab)
        print(f"UNTESTED {cid} (n={ab['card_count']})")
        print(f"            {ab['triggerless_text'][:75]}")
    print()

print(f"\n=== Tested: {tested}/{len(auto_abilities)} ===")
print(f"=== Untested: {len(untested_list)}/{len(auto_abilities)} ===")
