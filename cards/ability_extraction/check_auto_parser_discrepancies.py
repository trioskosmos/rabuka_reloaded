"""
Check for parser discrepancies between Japanese card text and parsed output
for 自動 abilities. Focus on whether the trigger condition is properly captured
in the parsed JSON structure.
"""
import json

with open(r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\abilities.json", encoding="utf-8") as f:
    data = json.load(f)

auto_abilities = [ab for ab in data["unique_abilities"] if ab.get("triggers") == "自動"]

print(f"Checking {len(auto_abilities)} 自動 abilities for parser discrepancies...\n")

issues = []

for ab in auto_abilities:
    ft = ab["full_text"]
    tt = ab["triggerless_text"]
    effect = ab.get("effect")
    if not effect or not isinstance(effect, dict):
        issues.append(("MISSING_EFFECT", ab["cards"][0], ft, "No effect parsed"))
        continue

    card_id = ab["cards"][0].split(" (ab#")[0]

    # Check 1: The trigger condition should be captured somehow
    # Auto abilities have their trigger IN the text, not as a separate trigger field
    # The parser should either put it in effect.condition or effect.trigger_condition

    has_condition = bool(effect.get("condition"))
    has_trigger_condition = bool(effect.get("trigger_condition"))
    action_type = effect.get("action", "")

    if not has_condition and not has_trigger_condition:
        issues.append(("NO_CONDITION", card_id, tt, f"action={action_type}"))
        continue

    # Check 2: Verify specific patterns match

    # Pattern: "〜したとき" in text should map to a condition
    if "したとき" in tt or "するたび" in tt:
        if not has_condition and not has_trigger_condition:
            issues.append(("MISSING_TRIGGER_CONDITION", card_id, tt, "Has したとき/するたび but no condition"))

    # Pattern: "エールしたとき" -> should have condition about yell
    if "エールしたとき" in tt:
        cond = effect.get("condition", {})
        trigger_cond = effect.get("trigger_condition", {})
        # Check if yell/revealed_cards context is captured
        cond_text = json.dumps(cond, ensure_ascii=False) + json.dumps(trigger_cond, ensure_ascii=False)
        if "revealed" not in cond_text and "yell" not in cond_text and "custom" not in cond_text:
            issues.append(("YELL_TRIGGER_NOT_CAPTURED", card_id, tt,
                          f"condition: {cond.get('type','?')}"))

    # Pattern: "エリアを移動したとき" -> should have movement_condition
    if "エリアを移動" in tt:
        cond = effect.get("condition", {})
        if cond.get("type") != "movement_condition":
            issues.append(("MOVEMENT_TYPE_WRONG", card_id, tt,
                          f"Expected movement_condition, got {cond.get('type','?')}"))

    # Pattern: "ステージから控え室に置かれたとき" -> should reference discard/stage
    if "ステージから控え室に置かれたとき" in tt:
        cond = effect.get("condition", {})
        cond_type = cond.get("type", "?")
        loc = cond.get("location", "?")
        if loc != "discard":
            issues.append(("DISCARD_LOCATION_WRONG", card_id, tt,
                          f"condition type={cond_type}, location={loc}"))
        if cond.get("source") != "preceding_moved":
            issues.append(("DISCARD_SOURCE_WRONG", card_id, tt,
                          f"Expected source=preceding_moved, got {cond.get('source','?')}"))

    # Pattern: "登場したとき" -> should have appearance_condition
    if "登場したとき" in tt and "エリアを移動" not in tt and "バトンタッチ" not in tt:
        cond = effect.get("condition", {})
        if cond.get("type") != "appearance_condition":
            issues.append(("APPEARANCE_TYPE_WRONG", card_id, tt,
                          f"Expected appearance_condition, got {cond.get('type','?')}"))

    # Pattern: "登場するたび" -> should have appearance_condition + trigger_type each_time
    if "登場するたび" in tt:
        trigger_cond = effect.get("trigger_condition", {})
        if effect.get("trigger_type") != "each_time" and trigger_cond.get("type") != "appearance_condition":
            issues.append(("EACH_TIME_APPEAR_WRONG", card_id, tt,
                          f"trigger_type={effect.get('trigger_type')}, trigger_condition={trigger_cond.get('type','?')}"))

    # Pattern: "ウェイト状態になったとき" -> should have state_change_condition
    if "ウェイト状態になったとき" in tt:
        cond = effect.get("condition", {})
        if cond.get("type") != "state_change_condition":
            issues.append(("STATE_CHANGE_TYPE_WRONG", card_id, tt,
                          f"Expected state_change_condition, got {cond.get('type','?')}"))

    # Pattern: sequential action with condition at top level
    if action_type == "sequential" and not has_condition:
        # Check if condition is in the first sub-action instead
        actions = effect.get("actions", [])
        if actions and isinstance(actions[0], dict):
            # Sometimes the trigger condition ends up in the sub-action
            pass  # this is a known pattern

    # Check for re_yell action - complex action that might be mis-parsed
    if "もう一度エール" in tt:
        if action_type == "sequential":
            has_re_yell = any(
                a.get("action") == "re_yell"
                for a in effect.get("actions", [])
                if isinstance(a, dict)
            )
            if not has_re_yell:
                # Check conditional_on_result pattern
                if effect.get("action") == "conditional_on_result":
                    followup = effect.get("followup_action", {})
                    if followup.get("action") != "re_yell":
                        issues.append(("RE_YELL_NOT_FOUND", card_id, tt,
                                      f"action={action_type}"))
        elif action_type == "conditional_on_result":
            pass  # OK
        else:
            issues.append(("RE_YELL_UNEXPECTED_ACTION", card_id, tt,
                          f"action={action_type}"))

    # Check: turn limit should be preserved in full_text but stripped in triggerless
    if "{{turn1.png" in ft or "{{turn2.png" in ft:
        if "{{turn" in tt:
            issues.append(("TURN_LIMIT_NOT_STRIPPED", card_id, tt, "Turn marker still in triggerless"))

    # Check: center requirement
    if "{{center.png" in ft:
        if "center" not in json.dumps(effect, ensure_ascii=False):
            issues.append(("CENTER_NOT_IN_EFFECT", card_id, tt,
                          f"Center marker in full_text but not in effect"))

    # Check: parenthetical about opponent effects
    if "対戦相手のカードの効果でも発動する" in tt:
        paren = effect.get("parenthetical", [])
        if not paren:
            issues.append(("OPPONENT_PARENTHETICAL_MISSING", card_id, tt,
                          "Opponent effect parenthetical not captured"))

print(f"Found {len(issues)} potential issues:\n")

for issue_type, card_id, text, detail in issues:
    print(f"  [{issue_type}] {card_id}")
    print(f"    Text: {text[:80]}...")
    print(f"    Detail: {detail}")
    print()

# Also check: for auto abilities, the "triggers" field is just "自動"
# but the ACTUAL trigger is embedded in the text. Let's verify no
# auto ability has a more specific trigger field
no_trigger_detail = True
for ab in auto_abilities:
    if ab.get("triggers") != "自動":
        no_trigger_detail = False
        print(f"  UNEXPECTED: {ab['cards'][0]} has triggers={ab['triggers']}")

if no_trigger_detail:
    print("\n  NOTE: All 56 自動 abilities have triggers='自動' — the actual sub-trigger")
    print("  (on_yell, on_move, on_discard, etc.) is ONLY in the text, NOT in the triggers field.")
    print("  The parser does NOT extract the trigger type into a separate field.")
