"""
Compare raw ability text against parsed JSON to find mechanics
described in text but missing/incorrect in the parsed structure.
"""

import json, re, sys
from collections import defaultdict

with open("cards/abilities.json", "r", encoding="utf-8") as f:
    data = json.load(f)
abilities = data["unique_abilities"]


def json_has(obj, pred):
    """Recursively check if any nested dict/list satisfies pred."""
    if isinstance(obj, dict):
        if pred(obj):
            return True
        for v in obj.values():
            if json_has(v, pred):
                return True
    elif isinstance(obj, list):
        for item in obj:
            if json_has(item, pred):
                return True
    return False


def json_has_action(obj, action_str):
    return json_has(
        obj, lambda d: isinstance(d, dict) and d.get("action") == action_str
    )


def json_has_field(obj, field, value=None):
    def check(d):
        if not isinstance(d, dict):
            return False
        if field not in d:
            return False
        if value is not None:
            return d[field] == value
        return True

    return json_has(obj, check)


# Define text patterns → expected JSON structures
# Each rule: (name, regex, check_fn(parsed_effect) -> bool, description)
RULES = [
    # ─── Number selection ───
    (
        "select_number",
        r"(数を選ん|[数数字]を[選選え]|[選選え]ん[だた]数)",
        lambda e: json_has_action(e, "select_number")
        or json_has_field(e, "action", "select"),
        "Text says 'choose a number' but no select_number action found",
    ),
    # ─── Opponent choice ───
    (
        "opponent_choice",
        r"相手[はが].*[選選え]ぶ",
        lambda e: json_has_field(e, "action_by", "opponent"),
        "Opponent chooses but no action_by: opponent",
    ),
    # ─── Distinct card names ───
    (
        "distinct_name",
        r"(カード名の異なる|カード名が異なる|名前の異なる|名前が異なる|異なるカード名)",
        lambda e: json_has_field(e, "distinct", "card_name"),
        "Distinct card names required but no distinct field",
    ),
    # ─── Reveal until ───
    (
        "reveal_until",
        r"(公開するまで|まで公開|公開し続け|現れるまで)",
        lambda e: json_has_action(e, "reveal_until_live_card")
        or json_has_action(e, "reveal_until_chosen_card"),
        "Reveal until condition but no reveal_until action",
    ),
    # ─── Repeat procedure ───
    (
        "repeat_procedure",
        r"(繰り返す|まで繰り返|もう一度行う|再度)",
        lambda e: json_has_action(e, "repeat_procedure")
        or json_has_field(e, "repeat_limit"),
        "Repeat/loop described but no repeat_procedure or repeat_limit",
    ),
    # ─── Conditional alternative (if X do Y, otherwise do Z) ───
    (
        "conditional_alt",
        r"(なかった場合|なければ|ない場合|なけれ|以外の場合)",
        lambda e: json_has_action(e, "conditional_alternative")
        or json_has_action(e, "conditional_on_result"),
        "Fallback/alternative (if not) but no conditional_alternative",
    ),
    # ─── Placement order ───
    (
        "placement_order",
        r"(好きな順番|任意の順番|好きな順序|任意の順|好きな順)",
        lambda e: json_has_field(e, "placement_order"),
        "Any-order placement but no placement_order field",
    ),
    # ─── Discard to hand count ───
    (
        "discard_until",
        r"(枚になるまで.*捨て|枚になるまで.*トラッシュ|枚になるまで.*墓地|になるまで捨て|になるまでトラッシュ)",
        lambda e: json_has_action(e, "discard_until_count"),
        "Discard until hand size but no discard_until_count",
    ),
    # ─── Pay energy as cost ───
    (
        "pay_energy_cost",
        r"(エネルギーを.*支払|エネルギー.*払う|E.*支払)",
        lambda e: json_has_action(e, "pay_energy"),
        "Pay energy described but no pay_energy action",
    ),
    # ─── All blade timing ───
    (
        "all_blade",
        r"(ALLブレード|全てのブレード|任意の色|任意のブレード|ALL blade)",
        lambda e: json_has_field(e, "all_blade_timing")
        or json_has_action(e, "all_blade_timing"),
        "ALL blade / any-color handling but no all_blade_timing",
    ),
    # ─── Invalidate / suppress ability ───
    (
        "invalidate_ability",
        r"(無効|発動しな|発動を防|無効にす|能力を.*失|無くな)",
        lambda e: json_has_action(e, "invalidate_ability")
        or json_has_action(e, "suppress_ability_trigger"),
        "Ability nullification but no invalidate_ability or suppress_ability_trigger",
    ),
    # ─── Both players / both targets ───
    (
        "both_targets",
        r"(お互い|両プレイヤー|相手と自分|自分と相手|それぞれ)",
        lambda e: json_has_field(e, "target", "both"),
        "Both players affected but no target='both'",
    ),
    # ─── Additional Yell (re-yell / perform_yell) ───
    (
        "additional_yell",
        r"(追加で.*エール|エール.*追加|もう一度.*エール|さらに.*エール|追エール)",
        lambda e: json_has_action(e, "perform_yell") or json_has_action(e, "re_yell"),
        "Additional Yell but no perform_yell or re_yell",
    ),
    # ─── Under member (place or reference) ───
    (
        "under_member",
        r"(の下に置|の下にあ|下から|下に置かれ|下の)",
        lambda e: json_has_field(e, "source", "under_member")
        or json_has_field(e, "destination", "under_member"),
        "Under-member operation but no under_member source/destination",
    ),
    # ─── Energy deck → energy zone ───
    (
        "energy_deck_to_zone",
        r"(エネルギー置き場|エネルギーデッキ|エネルギーを.*アクティブ)",
        lambda e: json_has_field(e, "source", "energy_deck")
        or json_has_field(e, "destination", "energy_zone"),
        "Energy deck/zone operation but no energy_deck/energy_zone field",
    ),
    # ─── Blade type setting ───
    (
        "blade_type",
        r"(ブレード.*として扱|blade.*treat|ブレード.*にする)",
        lambda e: json_has_action(e, "set_blade_type"),
        "Blade type conversion but no set_blade_type",
    ),
    # ─── Set cost ───
    (
        "set_cost",
        r"(コストを.*変更|コスト.*になる|コストを.*する|コスト.*変え)",
        lambda e: json_has_action(e, "set_cost") or json_has_action(e, "modify_cost"),
        "Cost modification but no set_cost or modify_cost",
    ),
    # ─── Draw until count ───
    (
        "draw_until_hand",
        r"(枚になるまで.*引|になるまで.*ドロー|D.*枚になるまで)",
        lambda e: json_has_action(e, "draw_until_count"),
        "Draw until hand size but no draw_until_count",
    ),
    # ─── Heart type setting ───
    (
        "heart_type",
        r"(ハート.*として扱|heart.*treat|ハート.*する|heart.*になる)",
        lambda e: json_has_action(e, "set_heart_type"),
        "Heart type conversion but no set_heart_type",
    ),
    # ─── Card identity setting ───
    (
        "card_identity",
        r"(としても扱|として扱う|同一として扱|として見な)",
        lambda e: json_has_action(e, "set_card_identity"),
        "Card identity/card name treated as but no set_card_identity",
    ),
    # ─── Multiple targets ───
    (
        "multiple_targets",
        r"(枚.*まで|まで.*枚|最大.*枚)",
        lambda e: json_has_field(e, "multiple_targets"),
        "Multiple target count but no multiple_targets field",
    ),
    # ─── Exclude self ───
    (
        "exclude_self",
        r"(自分以外|自身以外|このカード以外|このメンバー以外|自分を除く)",
        lambda e: json_has_field(e, "exclude_self"),
        "Exclude self described but no exclude_self field",
    ),
]


# Find ALL actions in parsed JSON for reference
def find_actions(obj, depth=0):
    actions = set()
    if isinstance(obj, dict):
        if "action" in obj:
            actions.add(obj["action"])
        for v in obj.values():
            if isinstance(v, (dict, list)):
                actions.update(find_actions(v, depth + 1))
    elif isinstance(obj, list):
        for item in obj:
            actions.update(find_actions(item, depth + 1))
    return actions


# Analyze
results = []  # (ability_idx, pattern_name, cards, trigger, text_snippet, actual_actions)
all_text_checks = defaultdict(list)
seen_by_rule = defaultdict(set)

for idx, entry in enumerate(abilities):
    text = entry.get("triggerless_text", "") or entry.get("full_text", "")
    effect = entry.get("effect")
    if not effect or not isinstance(effect, dict):
        continue
    if not text:
        continue

    actual_actions = find_actions(effect)
    cards = entry.get("cards", [])

    for rule_name, pattern, check_fn, desc in RULES:
        m = re.search(pattern, text)
        if m:
            matched = m.group(0)
            # Check if JSON has the expected structure
            if not check_fn(effect):
                text_snippet = text[max(0, m.start() - 20) : m.end() + 30]
                results.append(
                    (
                        idx,
                        rule_name,
                        cards,
                        entry.get("triggers", ""),
                        text_snippet,
                        actual_actions,
                        desc,
                    )
                )

# Remove dupes across same card sets
seen_cardsets = set()
unique_results = []
for r in sorted(results, key=lambda x: x[1]):
    cardset = tuple(sorted(r[2])) if r[2] else r[0]
    key = (r[1], cardset)
    if key not in seen_cardsets:
        seen_cardsets.add(key)
        unique_results.append(r)

# Group by rule
by_rule = defaultdict(list)
for r in unique_results:
    by_rule[r[1]].append(r)

# Summary
print("=" * 80)
print("MISSING MECHANICS ANALYSIS")
print("=" * 80)
print(f"\nTotal abilities checked: {len(abilities)}")
print(f"Total mismatches found: {len(unique_results)}")
print(f"Unique rule types triggered: {len(by_rule)}")
print()

# Print by rule, most impactful first (prioritize rare action types like select_number)
priority_order = [
    "select_number",  # 1 ability, unique mechanic
    "distinct_name",  # Core mechanic, easy to miss
    "opponent_choice",  # Core mechanic, recent bug
    "reveal_until",  # Rare action type
    "repeat_procedure",  # Rare action type
    "discard_until",  # Rare action type
    "draw_until_hand",  # Rare action type
    "additional_yell",  # Rare action type
    "under_member",  # Niche mechanic
    "placement_order",  # Qualitative detail
    "multiple_targets",  # Field presence
    "energy_deck_to_zone",  # Niche mechanic
    "conditional_alt",  # Structural
    "both_targets",  # Target correctness
    "blade_type",  # Format correctness
    "set_cost",  # Could be alternative parsing
    "heart_type",  # Could be alternative parsing
    "card_identity",  # Could be alternative parsing
    "invalidate_ability",  # Rare action type
    "exclude_self",  # Field presence
    "pay_energy_cost",  # Cost structure
    "all_blade",  # Format correctness
]

SEP = "-" * 60

printed_rules = set()
for rule_name in priority_order:
    if rule_name not in by_rule:
        continue
    printed_rules.add(rule_name)
    entries = by_rule[rule_name]
    desc = entries[0][6]
    print(f"\n{SEP}")
    print(f">> {rule_name}  ({len(entries)} abilities)")
    print(f"   {desc}")
    print(SEP)
    for r in entries[:10]:
        _, _, cards, trigger, snippet, actions, _ = r
        card_str = cards[0] if cards else "(no card)"
        action_str = ", ".join(sorted(actions)) if actions else "(none)"
        print(f"\n  CARD: {card_str}")
        print(f"  TRIGGER: {trigger}")
        print(f"  TEXT: ...{snippet}...")
        print(f"  ACTIONS: {action_str}")

# Any rules not in priority order
for rule_name in sorted(by_rule.keys()):
    if rule_name in printed_rules:
        continue
    entries = by_rule[rule_name]
    desc = entries[0][6]
    print(f"\n{SEP}")
    print(f">> {rule_name}  ({len(entries)} abilities)")
    print(f"   {desc}")
    print(SEP)
    for r in entries[:5]:
        _, _, cards, trigger, snippet, actions, _ = r
        card_str = cards[0] if cards else "(no card)"
        action_str = ", ".join(sorted(actions)) if actions else "(none)"
        print(f"\n  CARD: {card_str}")
        print(f"  TEXT: ...{snippet}...")
        print(f"  ACTIONS: {action_str}")

print(f"\n\n{'=' * 80}")
print("END OF REPORT")
print(f"{'=' * 80}")
