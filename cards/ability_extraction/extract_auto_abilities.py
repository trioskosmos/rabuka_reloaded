"""
Extract all 自動 (auto) abilities from abilities.json and group them by sub-trigger type.
Writes output to auto_abilities_grouped.md
"""
import json
import re
from collections import defaultdict

with open(r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\abilities.json", encoding="utf-8") as f:
    data = json.load(f)

auto_abilities = []
for ab in data["unique_abilities"]:
    triggers = ab.get("triggers", "")
    if triggers == "自動":
        auto_abilities.append(ab)

print(f"Total 自動 abilities: {len(auto_abilities)}")

# The full_text starts with {{jidou.png|自動}}...  and then has a sub-trigger in the text
# We need to figure out the sub-trigger from the text after the 自動 marker
# Common sub-triggers in card text patterns for 自動:
# - 自動【このメンバーがステージに出た時】  -> 登場 / on_play
# - 自動【このメンバーがアタックした時】   -> on_attack
# - 自動【相手がメンバーを登場した時】     -> on_opponent_play
# - 自動【ライブ開始時】                  -> on_live_start
# - 自動【ライブ開始時】 this card is on stage
# - 自動【このカードがアタックした時】     -> on_attack
# - 自動【このメンバーがアタックした時】   -> on_attack
# - 自動【このメンバーがステージに出た時】 -> on_play
# - 自動【相手のターン開始時】            -> on_opponent_turn_start
# - 自動【自分のターン開始時】            -> on_own_turn_start
# - 自動【ラウンド開始時】               -> on_round_start
# - 自動【このメンバーがダメージを受けた時】 -> on_damage
# - 自動【このメンバーがリバースした時】   -> on_reverse
# - 自動【このメンバーが破壊された時】     -> on_destroy
# - 自動【このカードが手札から控え室に置かれた時】 -> on_discard_from_hand
# - 自動【このメンバーが控え室に置かれた時】 -> on_sent_to_discard
# etc.

# Let's extract the bracketed condition from full_text
def extract_auto_trigger(full_text):
    """Extract the sub-trigger condition from 自動 ability text."""
    # Remove the {{jidou.png|自動}} prefix
    text = full_text
    # Match the 【...】 pattern
    m = re.search(r"【(.+?)】", text)
    if m:
        condition = m.group(1).strip()
        # Also extract text after the 】
        after = text[m.end():].strip()
        return condition, after
    return "UNKNOWN (no bracket condition)", full_text

groups = defaultdict(list)
for ab in auto_abilities:
    condition, effect_text = extract_auto_trigger(ab["full_text"])
    groups[condition].append({
        "full_text": ab["full_text"],
        "triggerless_text": ab["triggerless_text"],
        "condition": condition,
        "effect_text": effect_text,
        "card_count": ab["card_count"],
        "cards": ab["cards"],
        "triggers": ab["triggers"],
        "effect": ab.get("effect"),
        "cost": ab.get("cost"),
    })

# Sort groups by count descending
sorted_groups = sorted(groups.items(), key=lambda x: len(x[1]), reverse=True)

with open(r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\ability_extraction\auto_abilities_grouped.md", "w", encoding="utf-8") as f:
    f.write("# 自動 Abilities Grouped by Sub-Trigger Type\n\n")
    f.write(f"**Total 自動 abilities: {len(auto_abilities)}**\n")
    f.write(f"**Unique sub-trigger types: {len(sorted_groups)}**\n\n")

    for condition, abilities in sorted_groups:
        f.write(f"## [{condition}] — {len(abilities)} abilities\n\n")
        for i, ab in enumerate(abilities):
            f.write(f"### {i+1}. {ab['cards'][0]} (shared by {ab['card_count']} cards)\n\n")
            f.write(f"- **full_text**: `{ab['full_text']}`\n")
            f.write(f"- **triggerless_text**: `{ab['triggerless_text']}`\n")
            f.write(f"- **parsed effect**: {json.dumps(ab.get('effect'), ensure_ascii=False, indent=2) if ab.get('effect') else 'null'}\n")
            if ab.get("cost"):
                f.write(f"- **parsed cost**: {json.dumps(ab['cost'], ensure_ascii=False, indent=2)}\n")
            f.write(f"- **cards**:\n")
            for card in ab["cards"]:
                f.write(f"  - `{card}`\n")
            f.write("\n")
        f.write("---\n\n")

print("Wrote auto_abilities_grouped.md")
