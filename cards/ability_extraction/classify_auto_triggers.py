"""
Classify all 自動 (auto) abilities by their actual trigger condition from Japanese text.
Then cross-reference with test files for coverage gaps.
"""
import json
import re
import os
from collections import defaultdict

with open(r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\abilities.json", encoding="utf-8") as f:
    data = json.load(f)

auto_abilities = []
for ab in data["unique_abilities"]:
    triggers = ab.get("triggers", "")
    if triggers == "自動":
        auto_abilities.append(ab)

print(f"Total 自動 abilities: {len(auto_abilities)}\n")

# Classify based on the trigger condition in the Japanese text
# 自動 doesn't use 【】 brackets - the condition is directly in the text after the marker
def classify_trigger(full_text, triggerless_text):
    """Classify the auto-ability trigger from the Japanese text."""

    # Remove marker prefixes
    text = triggerless_text

    # Check for use_limit markers that were removed
    # (turn1, turn2 etc are already stripped in triggerless_text)

    # --- Direct trigger pattern matching ---

    # 1. エール系: triggers on yell/cheer
    if "エールしたとき" in text or "エールにより" in text:
        return "on_yell"

    # 2. エリア移動系: triggers on area movement
    if "エリアを移動したとき" in text or "エリアを移動するたび" in text:
        return "on_area_move"

    # 3. 登場かエリア移動: triggers on play OR area move
    if "登場か、エリアを移動" in text or "登場か、エリアを移動したとき" in text:
        return "on_play_or_move"

    # 4. バトンタッチして控え室に置かれたとき: baton touch to discard (must check before general discard)
    if "バトンタッチして控え室に置かれたとき" in text:
        return "on_baton_touch_to_discard"

    # 5. ステージから控え室に置かれたとき: sent from stage to discard
    if "ステージから控え室に置かれたとき" in text:
        return "on_sent_to_discard_from_stage"

    # 5. 自分のステージに...登場したとき: ally member appears on stage
    if re.search(r"自分のステージに.*登場したとき", text):
        if "バトンタッチして登場したとき" in text:
            return "on_baton_touch_appear"
        return "on_ally_appear_on_stage"

    # 6. 自分のステージに...登場するたび: each time ally appears on stage
    if re.search(r"自分のステージに.*登場するたび", text):
        return "on_ally_appear_each_time"

    # 7. ライブ開始時能力が解決したとき: on live_start ability resolved
    if "ライブ開始時" in text and "解決" in text:
        return "on_live_start_ability_resolved"

    # 8. ライブ成功時能力が解決したとき: on live_success ability resolved
    if "ライブ成功時" in text and "解決" in text:
        return "on_live_success_ability_resolved"

    # 9. カードの効果によって...ウェイト状態になったとき: state changed to wait by card effect
    if "ウェイト状態になったとき" in text:
        return "on_state_changed_to_wait"

    # 10. アクティブ状態からウェイト状態になったとき: self changed active->wait
    if "アクティブ状態からウェイト状態になったとき" in text:
        return "on_self_active_to_wait"

    # 11. ライブカード置き場から控え室に置かれたとき: live card sent from live zone to discard
    if "ライブカード置き場から控え室に置かれたとき" in text:
        return "on_live_card_zone_to_discard"

    # 12. 表向きでライブカード置き場に置かれたとき: placed face-up in live card zone
    if "表向きでライブカード置き場に置かれたとき" in text:
        return "on_placed_in_live_card_zone"

    # 13. 控え室から手札に加えられたとき: added from discard to hand
    if "控え室から手札に加えられたとき" in text:
        return "on_discard_to_hand"

    # 14. 手札からカードが...控え室に置かれるたび: cards sent from hand to discard
    if "手札からカード" in text and "控え室に置かれるたび" in text:
        return "on_hand_to_discard_each_time"

    # 15. カードがいずれかの領域から控え室に置かれるたび: any card to discard each time
    if "いずれかの領域から控え室に置かれるたび" in text:
        return "on_any_to_discard_each_time"

    # 16. エネルギー置き場にエネルギーカードが置かれるたび: energy card placed each time
    if "エネルギー置き場にエネルギーカードが置かれるたび" in text:
        return "on_energy_placed_each_time"

    # 17. 自分のカードの効果によって...エリアを移動するか...エネルギーが置かれたとき
    if "エリアを移動するか自分のエネルギー置き場にエネルギーが置かれたとき" in text:
        return "on_move_or_energy_placed"

    # 18. このターン...メンバーが3回登場したとき: temporal count of appearances
    if "このターン" in text and "登場したとき" in text:
        return "on_temporal_appearance_count"

    # 19. エールにより公開された自分のカードの中にライブカードが1枚以上あるとき
    if "エールにより公開された自分のカードの中にライブカードが" in text and "あるとき" in text:
        return "on_yell_revealed_has_live"

    # 20. エールにより自分のカードを1枚以上公開したとき
    if "エールにより自分のカードを1枚以上公開したとき" in text:
        return "on_yell_revealed_cards"

    return "UNCATEGORIZED"


groups = defaultdict(list)
for ab in auto_abilities:
    trigger_type = classify_trigger(ab["full_text"], ab["triggerless_text"])
    groups[trigger_type].append(ab)

# Sort by count descending
sorted_groups = sorted(groups.items(), key=lambda x: len(x[1]), reverse=True)

with open(r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\ability_extraction\auto_trigger_classification.md", "w", encoding="utf-8") as f:
    f.write("# 自動 Ability Trigger Classification\n\n")
    f.write(f"**Total 自動 abilities: {len(auto_abilities)}**\n")
    f.write(f"**Unique trigger types: {len(sorted_groups)}**\n\n")

    # Summary table
    f.write("## Summary Table\n\n")
    f.write("| Trigger Type | Count | Cards |\n")
    f.write("|---|---|---|\n")
    for trigger_type, abilities in sorted_groups:
        card_ids = ", ".join(a["cards"][0].split(" |")[0] for a in abilities)
        f.write(f"| {trigger_type} | {len(abilities)} | {card_ids} |\n")
    f.write("\n---\n\n")

    # Detailed listing
    for trigger_type, abilities in sorted_groups:
        f.write(f"## {trigger_type} ({len(abilities)} abilities)\n\n")
        for i, ab in enumerate(abilities):
            f.write(f"### {i+1}. {ab['cards'][0]} (shared by {ab['card_count']} cards)\n\n")
            f.write(f"- **full_text**: `{ab['full_text']}`\n")
            f.write(f"- **triggerless_text**: `{ab['triggerless_text']}`\n")

            # Check for turn limit
            turn_limit = None
            if "{{turn1.png|ターン1回}}" in ab["full_text"]:
                turn_limit = "1/turn"
            elif "{{turn2.png|ターン2回}}" in ab["full_text"]:
                turn_limit = "2/turn"
            if turn_limit:
                f.write(f"- **use_limit**: {turn_limit}\n")

            # Check for center requirement
            if "{{center.png|センター}}" in ab["full_text"]:
                f.write(f"- **position**: center required\n")

            # Check for parenthetical
            if "対戦相手のカードの効果でも発動する" in ab["triggerless_text"]:
                f.write(f"- **opponent_effect**: also triggers on opponent's card effects\n")

            # Parsed condition type
            effect = ab.get("effect", {})
            if isinstance(effect, dict):
                cond = effect.get("condition", {})
                if isinstance(cond, dict):
                    f.write(f"- **parsed condition type**: `{cond.get('type', 'none')}`\n")
                f.write(f"- **parsed action type**: `{effect.get('action', 'none')}`\n")

            f.write(f"- **cards**:\n")
            for card in ab["cards"]:
                f.write(f"  - `{card}`\n")
            f.write("\n")
        f.write("---\n\n")

print("Wrote auto_trigger_classification.md")

# Print summary to console too
print("\n=== SUMMARY ===\n")
for trigger_type, abilities in sorted_groups:
    card_ids = [a["cards"][0].split(" |")[0] for a in abilities]
    print(f"{trigger_type}: {len(abilities)} abilities")
    for cid in card_ids:
        print(f"  - {cid}")
    print()
