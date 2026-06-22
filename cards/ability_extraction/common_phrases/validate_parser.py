"""
Semantic parser validation: checks that parsed JSON correctly represents ability text.
Walks BOTH cost and effect trees recursively.

Severity:
  ERROR   — Definitely wrong, will cause incorrect game behavior
  WARNING — Likely wrong or incomplete, may cause edge-case issues
  INFO    — Inconsistency worth reviewing

Usage:
  python common_phrases/validate_parser.py
"""

import json, re, sys
from collections import defaultdict
from datetime import datetime
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))
import parser

ABILITIES_FILE = Path(__file__).parent.parent.parent / "abilities.json"


def walk_tree(d, callback, path=""):
    """Walk entire JSON tree including all sub-keys."""
    if not isinstance(d, dict):
        return
    try:
        callback(d, path)
    except Exception:
        pass
    for k, v in d.items():
        if k == "text":
            continue
        if isinstance(v, dict):
            walk_tree(v, callback, f"{path}.{k}")
        elif isinstance(v, list):
            for i, item in enumerate(v):
                walk_tree(item, callback, f"{path}.{k}[{i}]")


def find_in_tree(root, key):
    """Find all values for a key anywhere in the tree. Returns set of str or None."""
    results = set()

    def cb(d, _):
        if key in d:
            v = d[key]
            if isinstance(v, list):
                for x in v:
                    results.add(str(x))
            elif isinstance(v, bool):
                results.add(str(v))
            elif v is not None:
                results.add(str(v))

    walk_tree(root, cb)
    return results if results else None


def has_any_action(root, *actions):
    """Check if any node has one of the given action types (or type field for costs)."""
    found = set()

    def cb(d, _):
        a = d.get("action") or d.get("type")
        if a in actions:
            found.add(a)

    walk_tree(root, cb)
    return found


def ability_context(a):
    """Return (triggerless_text, combined_tree) for an ability entry."""
    t = a.get("triggerless_text", "") or a.get("full_text", "")
    eff = a.get("effect")
    cost = a.get("cost")
    # Build a combined tree with both cost and effect
    combined = {}
    if isinstance(cost, dict):
        combined["cost"] = cost
    if isinstance(eff, dict):
        combined["effect"] = eff
    return t, combined


def validate(abilities):
    results = []

    def fail(sev, rule, idx, text, exp, act):
        results.append((sev, rule, idx, text[:100], exp, str(act)[:100]))

    for idx, a in enumerate(abilities):
        t, combined = ability_context(a)
        if not t or not combined:
            continue

        skip_effect_only = False  # for checks that should only apply to effect

        # ==================== SOURCE / DESTINATION CHECKS ====================

        # SD1: hand → discard pattern
        if "手札を" in t and "控え室に置く" in t:
            sources = find_in_tree(combined, "source") or set()
            dests = find_in_tree(combined, "destination") or set()
            if "hand" not in sources or "discard" not in dests:
                fail(
                    "WARNING",
                    "hand_to_discard_not_found",
                    idx,
                    t,
                    "source=hand AND dest=discard somewhere in tree",
                    f"sources={sources} dests={dests}",
                )

        # SD2: deck_top source
        if (
            "デッキの上から" in t
            or "デッキの一番上からカードを" in t
            or "デッキの一番上のカードを" in t
        ):
            sources = find_in_tree(combined, "source") or set()
            if "deck_top" not in sources:
                fail(
                    "WARNING",
                    "deck_top_source_missing",
                    idx,
                    t,
                    "source=deck_top in tree",
                    f"sources={sources}",
                )

        # SD3: discard source
        if ("控え室から" in t or "控え室にある" in t) and "手札に加える" in t:
            sources = find_in_tree(combined, "source") or set()
            if "discard" not in sources:
                fail(
                    "WARNING",
                    "discard_source_missing",
                    idx,
                    t,
                    "source=discard expected for 控え室から手札に加える",
                    f"sources={sources}",
                )

        # SD4: hand destination
        if "手札に加える" in t or "手札に置く" in t:
            dests = find_in_tree(combined, "destination") or set()
            if "hand" not in dests:
                fail(
                    "WARNING",
                    "hand_dest_missing",
                    idx,
                    t,
                    "destination=hand expected for 手札に加える/置く",
                    f"dests={dests}",
                )

        # SD5: discard destination
        if "控え室に置く" in t and "手札を" not in t:
            dests = find_in_tree(combined, "destination") or set()
            # Only flag if text clearly implies movement TO discard
            if "discard" not in dests and ("コスト" not in t or "を" not in t):
                fail(
                    "INFO",
                    "discard_dest_missing",
                    idx,
                    t,
                    "destination=discard might be expected",
                    f"dests={dests}",
                )

        # SD6: stage destination
        if "ステージに置く" in t or "登場させる" in t:
            dests = find_in_tree(combined, "destination") or set()
            if "stage" not in dests:
                fail(
                    "WARNING",
                    "stage_dest_missing",
                    idx,
                    t,
                    "destination=stage expected for ステージに置く/登場させる",
                    f"dests={dests}",
                )

        # ==================== TARGET CHECKS ====================

        # T1: opponent target
        if "相手の" in t and "自分の" not in t and "自分と相手" not in t:
            targets = find_in_tree(combined, "target") or set()
            if "opponent" not in targets:
                fail(
                    "WARNING",
                    "target_opponent_missing",
                    idx,
                    t,
                    "target=opponent in tree",
                    f"targets={targets}",
                )

        # T2: both target
        if "自分と相手" in t:
            targets = find_in_tree(combined, "target") or set()
            if "both" not in targets:
                fail(
                    "WARNING",
                    "target_both_missing",
                    idx,
                    t,
                    "target=both in tree",
                    f"targets={targets}",
                )

        # ==================== FLAG CHECKS ====================

        # F1: optional
        if "もよい" in t or "てもよい" in t:
            optionals = find_in_tree(combined, "optional") or set()
            if "True" not in optionals:
                fail(
                    "WARNING",
                    "optional_flag_missing",
                    idx,
                    t,
                    "optional=True somewhere in tree",
                    f"optional={optionals}",
                )

        # F2: all
        if "すべての" in t or "全ての" in t or "全部の" in t or "カードをすべて" in t:
            alls = find_in_tree(combined, "all") or set()
            if "True" not in alls:
                fail(
                    "WARNING",
                    "all_flag_missing",
                    idx,
                    t,
                    "all=True in tree",
                    f"all={alls}",
                )

        # F3: shuffle
        if "シャッフルする" in t or "シャッフルして" in t:
            shuffles = find_in_tree(combined, "shuffle") or set()
            if "True" not in shuffles:
                fail(
                    "WARNING",
                    "shuffle_flag_missing",
                    idx,
                    t,
                    "shuffle=True in tree",
                    f"shuffle={shuffles}",
                )

        # F4: exclude_self
        if "このメンバー以外" in t or re.search(r"ほかの.*メンバー", t):
            excludes = find_in_tree(combined, "exclude_self") or set()
            if "True" not in excludes:
                fail(
                    "WARNING",
                    "exclude_self_missing",
                    idx,
                    t,
                    "exclude_self=True in tree",
                    f"exclude_self={excludes}",
                )

        # F5: any_number
        if "好きな枚数" in t or "任意の枚数" in t:
            any_nums = find_in_tree(combined, "any_number") or set()
            if "True" not in any_nums:
                fail(
                    "WARNING",
                    "any_number_missing",
                    idx,
                    t,
                    "any_number=True in tree",
                    f"any_number={any_nums}",
                )

        # F6: multiple_targets
        if "それぞれ" in t or "ずつ" in t:
            mults = find_in_tree(combined, "multiple_targets") or set()
            if "True" not in mults:
                fail(
                    "WARNING",
                    "multiple_targets_missing",
                    idx,
                    t,
                    "multiple_targets=True in tree",
                    f"multiple_targets={mults}",
                )

        # F7: max
        if "人まで" in t or "枚まで" in t:
            maxes = find_in_tree(combined, "max") or set()
            if "True" not in maxes:
                fail(
                    "WARNING",
                    "max_flag_missing",
                    idx,
                    t,
                    "max=True in tree",
                    f"max={maxes}",
                )

        # F8: original_value
        if "元々持つ" in t or "元々の" in t:
            origs = find_in_tree(combined, "original_value") or set()
            if "True" not in origs:
                fail(
                    "WARNING",
                    "original_value_missing",
                    idx,
                    t,
                    "original_value=True in tree",
                    f"original_value={origs}",
                )

        # F9: placement_order
        if "好きな順番で" in t:
            orders = find_in_tree(combined, "placement_order") or set()
            if "any_order" not in orders:
                fail(
                    "WARNING",
                    "placement_order_missing",
                    idx,
                    t,
                    "placement_order=any_order in tree",
                    f"placement_order={orders}",
                )

        # ==================== FIELD COMPLETENESS ====================

        # C1: move_cards with missing source or destination
        def check_move(d, _):
            if d.get("action") == "move_cards":
                if not d.get("source") or not d.get("destination"):
                    return True
            return False

        incomplete_moves = []

        def cb(d, _):
            if d.get("action") == "move_cards":
                if not d.get("source") or not d.get("destination"):
                    incomplete_moves.append(d)

        walk_tree(combined, cb)
        for d in incomplete_moves:
            fail(
                "ERROR",
                "move_cards_incomplete",
                idx,
                t,
                f"move_cards must have source + destination",
                f"source={d.get('source')} dest={d.get('destination')} text={d.get('text', '')[:40]}",
            )

        # C2: gain_resource with missing resource or count
        incomplete_gains = []

        def cb(d, _):
            if d.get("action") == "gain_resource":
                if not d.get("resource"):
                    incomplete_gains.append(d)
                elif not d.get("count") and not d.get("dynamic_count"):
                    # all=true or dynamic_count can substitute for explicit count
                    if (
                        not d.get("all")
                        and not d.get("per_unit")
                        and not d.get("any_number")
                    ):
                        incomplete_gains.append(d)

        walk_tree(combined, cb)
        for d in incomplete_gains:
            fail(
                "ERROR",
                "gain_resource_incomplete",
                idx,
                t,
                "gain_resource must have resource + count",
                f"resource={d.get('resource')} count={d.get('count')} dyn={d.get('dynamic_count')} text={d.get('text', '')[:40]}",
            )

        # C3: change_state with missing state_change
        def cb(d, _):
            if d.get("action") == "change_state" and not d.get("state_change"):
                fail(
                    "ERROR",
                    "change_state_no_state",
                    idx,
                    t,
                    "change_state must have state_change field",
                    f"text={d.get('text', '')[:40]}",
                )

        walk_tree(combined, cb)

        # C4: modify_score with missing operation or value
        def cb(d, _):
            if d.get("action") == "modify_score":
                if not d.get("operation") or not d.get("value"):
                    fail(
                        "ERROR",
                        "modify_score_incomplete",
                        idx,
                        t,
                        "modify_score must have operation + value",
                        f"op={d.get('operation')} val={d.get('value')} text={d.get('text', '')[:40]}",
                    )

        walk_tree(combined, cb)

        # C5: gain_ability with missing ability_gain
        def cb(d, _):
            if d.get("action") == "gain_ability" and not d.get("ability_gain"):
                fail(
                    "ERROR",
                    "gain_ability_no_text",
                    idx,
                    t,
                    "gain_ability must have ability_gain field",
                    f"text={d.get('text', '')[:40]}",
                )

        walk_tree(combined, cb)

        # C6: draw_card with missing count
        def cb(d, _):
            if (
                d.get("action") == "draw_card"
                and not d.get("count")
                and not d.get("dynamic_count")
            ):
                fail(
                    "ERROR",
                    "draw_card_no_count",
                    idx,
                    t,
                    "draw_card must have count or dynamic_count",
                    f"text={d.get('text', '')[:40]}",
                )

        walk_tree(combined, cb)

        # C7: choice with missing or empty options
        def cb(d, _):
            if d.get("action") == "choice":
                opts = d.get("options", [])
                if len(opts) < 2:
                    fail(
                        "WARNING",
                        "choice_too_few_options",
                        idx,
                        t,
                        "choice should have >= 2 options",
                        f"options={len(opts)} text={d.get('text', '')[:40]}",
                    )

        walk_tree(combined, cb)

        # C8: {{icon_score.png|スコア}}を持つ requires card_property: has_score_icon
        if "{{icon_score.png|スコア}}を持つ" in t:
            props = find_in_tree(combined, "card_property") or set()
            if "has_score_icon" not in props:
                fail(
                    "ERROR",
                    "score_icon_missing_card_property",
                    idx,
                    t,
                    "card_property=has_score_icon expected when text contains {{icon_score.png|スコア}}を持つ",
                    f"card_property values found: {props}",
                )

        # ==================== STRUCTURE CHECKS ====================

        # S1: 代わりに → conditional_alternative
        if "代わりに" in t and "以下から1つを選ぶ" not in t:
            acts = has_any_action(combined, "conditional_alternative")
            if "conditional_alternative" not in acts:
                fail(
                    "INFO",
                    "conditional_alternative_not_parsed",
                    idx,
                    t,
                    "conditional_alternative expected for 代わりに",
                    f"actions={acts}",
                )

        # S2: そうした場合 → conditional sequential
        if "そうした場合" in t:
            found = False

            def cb(d, _):
                nonlocal found
                if d.get("action") == "sequential" and d.get("conditional"):
                    found = True

            walk_tree(combined, cb)
            if not found:
                fail(
                    "INFO",
                    "conditional_sequential_not_parsed",
                    idx,
                    t,
                    "sequential with conditional=True expected",
                    f"top={list(has_any_action(combined, 'sequential'))}",
                )

        # S3: そうしなかった場合 → conditional_on_optional
        if "そうしなかった場合" in t:
            acts = has_any_action(combined, "conditional_on_optional")
            if "conditional_on_optional" not in acts:
                fail(
                    "INFO",
                    "conditional_on_optional_not_parsed",
                    idx,
                    t,
                    "conditional_on_optional expected",
                    f"actions={acts}",
                )

        # S4: たび + たび、/たびに → each_time
        if ("たび、" in t or "たびに" in t) and "たびに" in t:
            triggers = find_in_tree(combined, "trigger_type") or set()
            if "each_time" not in triggers:
                fail(
                    "INFO",
                    "each_time_not_parsed",
                    idx,
                    t,
                    "trigger_type=each_time expected",
                    f"trigger_type={triggers}",
                )

        # S5: かぎり + かぎり、 → as_long_as
        if "かぎり、" in t:
            durs = find_in_tree(combined, "duration") or set()
            if "as_long_as" not in durs:
                fail(
                    "INFO",
                    "duration_as_long_as_not_parsed",
                    idx,
                    t,
                    "duration=as_long_as expected",
                    f"duration={durs}",
                )

        # S6: につき → per_unit (excluding cost modification patterns)
        if "につき" in t and "グループ名につき" not in t:
            if not re.search(r"コスト[はが].*につき.*減る", t):
                per_units = find_in_tree(combined, "per_unit") or set()
                if "True" not in per_units:
                    fail(
                        "INFO",
                        "per_unit_not_parsed",
                        idx,
                        t,
                        "per_unit=True expected",
                        f"per_unit={per_units}",
                    )

        # S7: これにより + 場合 → conditional_on_result or condition
        if "これにより" in t and "場合" in t:
            acts = has_any_action(combined, "conditional_on_result")
            if "conditional_on_result" not in acts:
                conds = find_in_tree(combined, "condition") or set()
                has_kore = any("これにより" in str(c) for c in conds)
                if not has_kore:
                    fail(
                        "INFO",
                        "kore_niyori_not_parsed",
                        idx,
                        t,
                        "conditional_on_result or condition with これにより expected",
                        f"actions={has_any_action(combined, 'conditional_on_result', 'conditional_alternative')}",
                    )

        # S8: 以下から1つを選ぶ → choice
        if "以下から1つを選ぶ" in t:
            acts = has_any_action(combined, "choice")
            if "choice" not in acts:
                fail(
                    "WARNING",
                    "choice_not_parsed",
                    idx,
                    t,
                    "action=choice expected",
                    f"actions={acts}",
                )

        # S9: 無効に → invalidate_ability
        if "無効に" in t:
            acts = has_any_action(combined, "invalidate_ability")
            if "invalidate_ability" not in acts:
                fail(
                    "INFO",
                    "invalidate_not_parsed",
                    idx,
                    t,
                    "invalidate_ability expected",
                    f"actions={acts}",
                )

        # S10: シャッフル standalone (not combined with move_cards)
        if "シャッフルする" in t:
            shuffle_without_move = []

            def cb(d, _):
                if d.get("action") == "shuffle":
                    shuffle_without_move.append(d)
                elif d.get("action") == "move_cards" and d.get("shuffle"):
                    pass  # This is fine - shuffle merged into move

            walk_tree(combined, cb)
            if shuffle_without_move:
                fail(
                    "INFO",
                    "shuffle_without_move",
                    idx,
                    t,
                    "shuffle should be merged into move_cards",
                    f"standalone shuffle exists",
                )

        # S11: pay_energy check in costs
        if "{{icon_energy.png|E}}" in t and ("支払う" in t or "支払って" in t):
            acts = has_any_action(combined, "pay_energy")
            if "pay_energy" not in acts:
                fail(
                    "INFO",
                    "pay_energy_not_parsed",
                    idx,
                    t,
                    "pay_energy expected for energy payment",
                    f"actions={acts}",
                )

    return results


def print_report(results):
    by_severity = defaultdict(list)
    for r in results:
        by_severity[r[0]].append(r)

    print("=" * 70)
    print("PARSER VALIDATION REPORT")
    print("=" * 70)
    print()

    for sev in ("ERROR", "WARNING", "INFO"):
        items = by_severity.get(sev, [])
        if not items:
            continue
        print(f"--- {sev} ({len(items)}) ---")
        print()
        for severity, rule, idx, text, expected, actual in items[:10]:
            print(f"  #{idx:3d} | {rule}")
            print(f"  text: {text[:80]}...")
            print(f"  exp:  {expected}")
            print(f"  act:  {actual}")
            print()
        if len(items) > 10:
            print(f"  ... and {len(items) - 10} more")
            print()

    total = len(results)
    errors = len(by_severity.get("ERROR", []))
    warnings = len(by_severity.get("WARNING", []))
    infos = len(by_severity.get("INFO", []))
    print("=" * 70)
    print(
        f"SUMMARY: {total} total | {errors} errors | {warnings} warnings | {infos} infos"
    )

    print()
    print("--- By Rule ---")
    by_rule = defaultdict(int)
    for r in results:
        by_rule[f"{r[0]}:{r[1]}"] += 1
    for rule, count in sorted(by_rule.items(), key=lambda x: -x[1]):
        print(f"  {count:4d} | {rule}")


def generate_report_file(results, filename="validation_report.md"):
    by_severity = defaultdict(list)
    for r in results:
        by_severity[r[0]].append(r)

    lines = [f"# Parser Validation Report\n"]
    lines.append(f"Generated: {datetime.now().isoformat()}\n")
    lines.append(f"Total abilities: 645\n")

    for sev in ("ERROR", "WARNING", "INFO"):
        items = by_severity.get(sev, [])
        if not items:
            continue
        lines.append(f"## {sev} ({len(items)})\n")
        for severity, rule, idx, text, expected, actual in items:
            lines.append(f"- **#{idx}** `{rule}`: {expected}")
            lines.append(f"  - Text: `{text[:80]}`")
            lines.append(f"  - Actual: {actual}")
            lines.append("")

    lines.append("## Summary\n")
    total = len(results)
    errors = len(by_severity.get("ERROR", []))
    warnings = len(by_severity.get("WARNING", []))
    infos = len(by_severity.get("INFO", []))
    lines.append(f"- Total: {total}")
    lines.append(f"- Errors: {errors}")
    lines.append(f"- Warnings: {warnings}")
    lines.append(f"- Infos: {infos}")

    report = "\n".join(lines)
    out = Path(__file__).parent / filename
    out.write_text(report, encoding="utf-8")
    print(f"Report written to {out}")


if __name__ == "__main__":
    with open(ABILITIES_FILE, encoding="utf-8") as f:
        data = json.load(f)
    data = parser.process_abilities(data)
    abilities = data["unique_abilities"]
    results = validate(abilities)
    print_report(results)
    generate_report_file(results)
