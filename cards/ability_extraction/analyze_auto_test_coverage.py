"""
Cross-reference 自動 trigger types with test coverage in Rust test files.
Also checks for parser discrepancies by comparing Japanese text patterns with parsed output types.
"""
import os
import re
import json
from collections import defaultdict

ENGINE_TESTS_DIR = r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine\tests\test_modules"

# --- Step 1: Gather all auto abilities with their card IDs ---
with open(r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\abilities.json", encoding="utf-8") as f:
    data = json.load(f)

auto_abilities = []
auto_card_ids = set()  # card ID without ability suffix
for ab in data["unique_abilities"]:
    if ab.get("triggers") == "自動":
        auto_abilities.append(ab)
        for card_ref in ab["cards"]:
            card_id = card_ref.split(" (ab#")[0]  # e.g. "PL!S-bp2-007-R＋"
            auto_card_ids.add(card_id)

print(f"Auto ability card IDs (unique): {len(auto_card_ids)}")

# --- Step 2: Scan Rust test files for references to auto ability card IDs ---
test_dir = ENGINE_TESTS_DIR
test_files = [f for f in os.listdir(test_dir) if f.endswith(".rs")]

# Map card_id -> list of test files that mention it
card_test_coverage = defaultdict(list)

for tf in test_files:
    filepath = os.path.join(test_dir, tf)
    try:
        with open(filepath, encoding="utf-8") as f:
            content = f.read()
    except:
        continue

    for card_id in auto_card_ids:
        # Match card ID with potential rarity suffix
        # Card IDs can be like "PL!S-bp2-007-R+" - we want partial match
        # Remove special chars that might differ
        base_id = card_id.replace("＋", "+")
        # Check for the base pattern
        if base_id in content:
            card_test_coverage[card_id].append(tf)

# --- Step 3: Also scan for trigger-related patterns in test files ---
# Check for specific 自動 trigger patterns in test names / comments
trigger_pattern_coverage = defaultdict(list)

trigger_patterns = {
    "on_yell": ["yell", "エール", "cheer", "auto_yell"],
    "on_area_move": ["area_move", "エリア移動", "エリアを移動"],
    "on_sent_to_discard_from_stage": ["sent_to_discard", "stage_to_discard", "控え室に置かれた", "stage_from_discard"],
    "on_ally_appear_on_stage": ["ally_appear", "登場したとき", "member_appear"],
    "on_state_changed_to_wait": ["state_change", "ウェイト状態", "active_to_wait"],
    "on_ally_appear_each_time": ["each_time", "登場するたび"],
    "on_live_start_ability_resolved": ["live_start", "ライブ開始時", "live_start_resolve"],
    "on_live_success_ability_resolved": ["live_success", "ライブ成功時", "live_success_resolve"],
    "on_baton_touch_to_discard": ["baton_touch", "バトンタッチ"],
    "on_baton_touch_appear": ["baton_touch", "バトンタッチ"],
    "on_move_or_energy_placed": ["move_or_energy"],
    "on_any_to_discard_each_time": ["discard_each_time"],
    "on_live_card_zone_to_discard": ["live_card_zone"],
    "on_hand_to_discard_each_time": ["hand_to_discard"],
    "on_discard_to_hand": ["discard_to_hand"],
    "on_placed_in_live_card_zone": ["placed_live_card_zone"],
    "on_energy_placed_each_time": ["energy_placed"],
    "on_temporal_appearance_count": ["temporal_appearance"],
}

for tf in test_files:
    filepath = os.path.join(test_dir, tf)
    try:
        with open(filepath, encoding="utf-8") as f:
            content = f.read()
    except:
        continue
    content_lower = content.lower()

    for trigger_type, patterns in trigger_patterns.items():
        for pattern in patterns:
            if pattern.lower() in content_lower:
                trigger_pattern_coverage[trigger_type].append((tf, pattern))
                break  # one match per file per trigger is enough

# --- Step 4: Also check specific test files that look auto-related ---
auto_related_test_files = []
for tf in test_files:
    filepath = os.path.join(test_dir, tf)
    try:
        with open(filepath, encoding="utf-8") as f:
            content = f.read()
    except:
        continue
    if "自動" in content or "auto" in content.lower() or "jidou" in content.lower():
        auto_related_test_files.append(tf)

print(f"\nTest files mentioning 自動/auto/jidou: {len(auto_related_test_files)}")
for tf in sorted(auto_related_test_files):
    print(f"  {tf}")

# --- Step 5: Check for specific auto card tests ---
print(f"\n=== Card-level test coverage for 自動 abilities ===")
covered = 0
uncovered = 0
uncovered_cards = []
for card_id in sorted(auto_card_ids):
    tests = card_test_coverage.get(card_id, [])
    if tests:
        covered += 1
        print(f"  COVERED: {card_id}")
        for t in tests:
            print(f"    -> {t}")
    else:
        uncovered += 1
        uncovered_cards.append(card_id)
        print(f"  UNCOVERED: {card_id}")

print(f"\nCovered: {covered}/{len(auto_card_ids)}")
print(f"Uncovered: {uncovered}/{len(auto_card_ids)}")

# --- Step 6: Write comprehensive report ---
with open(r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\ability_extraction\auto_test_coverage_report.md", "w", encoding="utf-8") as f:
    f.write("# 自動 Ability Test Coverage Report\n\n")

    f.write(f"**Total 自動 unique abilities**: {len(auto_abilities)}\n")
    f.write(f"**Unique card IDs**: {len(auto_card_ids)}\n")
    f.write(f"**Card IDs with tests**: {covered}\n")
    f.write(f"**Card IDs WITHOUT tests**: {uncovered}\n\n")

    # Trigger type summary
    f.write("## Trigger Type Summary\n\n")

    # Reclassify for the report
    def classify_trigger(triggerless_text):
        text = triggerless_text
        if "エールしたとき" in text or "エールにより" in text:
            return "on_yell"
        if "エリアを移動したとき" in text or "エリアを移動するたび" in text:
            return "on_area_move"
        if "登場か、エリアを移動" in text:
            return "on_play_or_move"
        if "バトンタッチして控え室に置かれたとき" in text:
            return "on_baton_touch_to_discard"
        if "ステージから控え室に置かれたとき" in text:
            return "on_sent_to_discard_from_stage"
        if re.search(r"自分のステージに.*登場したとき", text):
            if "バトンタッチして登場したとき" in text:
                return "on_baton_touch_appear"
            return "on_ally_appear_on_stage"
        if re.search(r"自分のステージに.*登場するたび", text):
            return "on_ally_appear_each_time"
        if "ライブ開始時" in text and "解決" in text:
            return "on_live_start_ability_resolved"
        if "ライブ成功時" in text and "解決" in text:
            return "on_live_success_ability_resolved"
        if "ウェイト状態になったとき" in text:
            return "on_state_changed_to_wait"
        if "アクティブ状態からウェイト状態になったとき" in text:
            return "on_self_active_to_wait"
        if "ライブカード置き場から控え室に置かれたとき" in text:
            return "on_live_card_zone_to_discard"
        if "表向きでライブカード置き場に置かれたとき" in text:
            return "on_placed_in_live_card_zone"
        if "控え室から手札に加えられたとき" in text:
            return "on_discard_to_hand"
        if "手札からカード" in text and "控え室に置かれるたび" in text:
            return "on_hand_to_discard_each_time"
        if "いずれかの領域から控え室に置かれるたび" in text:
            return "on_any_to_discard_each_time"
        if "エネルギー置き場にエネルギーカードが置かれるたび" in text:
            return "on_energy_placed_each_time"
        if "エリアを移動するか自分のエネルギー置き場にエネルギーが置かれたとき" in text:
            return "on_move_or_energy_placed"
        if "このターン" in text and "登場したとき" in text:
            return "on_temporal_appearance_count"
        if "エールにより公開された自分のカードの中にライブカードが" in text:
            return "on_yell_revealed_has_live"
        if "エールにより自分のカードを1枚以上公開したとき" in text:
            return "on_yell_revealed_cards"
        return "UNCATEGORIZED"

    trigger_groups = defaultdict(lambda: {"total": 0, "tested": 0, "untested_cards": [], "tested_cards": []})
    for ab in auto_abilities:
        tt = classify_trigger(ab["triggerless_text"])
        for card_ref in ab["cards"]:
            card_id = card_ref.split(" (ab#")[0]
            trigger_groups[tt]["total"] += 1
            if card_id in card_test_coverage:
                trigger_groups[tt]["tested"] += 1
                trigger_groups[tt]["tested_cards"].append(card_id)
            else:
                trigger_groups[tt]["untested_cards"].append(card_id)

    f.write("| Trigger Type | Total | Tested | Untested | Coverage %|\n")
    f.write("|---|---|---|---|---|\n")
    for tt in sorted(trigger_groups.keys(), key=lambda x: trigger_groups[x]["total"], reverse=True):
        g = trigger_groups[tt]
        pct = (g["tested"] / g["total"] * 100) if g["total"] > 0 else 0
        f.write(f"| {tt} | {g['total']} | {g['tested']} | {g['total'] - g['tested']} | {pct:.0f}% |\n")

    f.write("\n---\n\n")

    # Detailed: trigger types with NO tests
    f.write("## Trigger Types with NO Test Coverage\n\n")
    no_tests = False
    for tt in sorted(trigger_groups.keys()):
        g = trigger_groups[tt]
        if g["tested"] == 0:
            no_tests = True
            f.write(f"### {tt} — 0/{g['total']} tested\n\n")
            for card_id in g["untested_cards"]:
                f.write(f"- `{card_id}`\n")
            f.write("\n")
    if not no_tests:
        f.write("*(All trigger types have at least some test coverage)*\n\n")

    # Detailed: partially tested trigger types
    f.write("## Trigger Types with Partial Test Coverage\n\n")
    for tt in sorted(trigger_groups.keys(), key=lambda x: trigger_groups[x]["total"], reverse=True):
        g = trigger_groups[tt]
        if 0 < g["tested"] < g["total"]:
            f.write(f"### {tt} — {g['tested']}/{g['total']} tested\n\n")
            f.write(f"**Tested:**\n")
            for card_id in g["tested_cards"]:
                tests = card_test_coverage.get(card_id, [])
                f.write(f"- `{card_id}` in {', '.join(tests)}\n")
            f.write(f"\n**Untested:**\n")
            for card_id in g["untested_cards"]:
                f.write(f"- `{card_id}`\n")
            f.write("\n")

    # Detailed: fully tested trigger types
    f.write("## Fully Tested Trigger Types\n\n")
    fully_tested = False
    for tt in sorted(trigger_groups.keys()):
        g = trigger_groups[tt]
        if g["tested"] == g["total"] and g["total"] > 0:
            fully_tested = True
            f.write(f"### {tt} — {g['tested']}/{g['total']} tested ✓\n\n")
            for card_id in g["tested_cards"]:
                tests = card_test_coverage.get(card_id, [])
                f.write(f"- `{card_id}` in {', '.join(tests)}\n")
            f.write("\n")
    if not fully_tested:
        f.write("*(No trigger type has full coverage)*\n\n")

    # Trigger pattern matches in test files
    f.write("\n---\n\n")
    f.write("## Trigger Pattern Matches in Test Files\n\n")
    f.write("(Test files that contain keywords related to each trigger type)\n\n")
    for tt in sorted(trigger_pattern_coverage.keys()):
        files = trigger_pattern_coverage[tt]
        f.write(f"### {tt}\n\n")
        if files:
            for (fname, pattern) in files:
                f.write(f"- `{fname}` (matched: `{pattern}`)\n")
        else:
            f.write("- *(no matches)*\n")
        f.write("\n")

print("\nWrote auto_test_coverage_report.md")
