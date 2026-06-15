#!/usr/bin/env python3
"""
Exhaustive key-combination coverage analysis for abilities.json.

For every effect action type, extracts all unique JSON key combinations used
across all abilities, then cross-references against tested abilities to find:
  - Untested key combinations (zero test coverage)
  - Least-tested action types
  - Least-tested key combinations within each action

Also analyzes cost+effect pairing coverage.

Output:
  - key_combo_coverage_report.txt - full report
  - least_tested_summary.txt     - quick-reference of top gaps
"""

import json
import re
from pathlib import Path
from collections import defaultdict


def safe_write(text, file_obj):
    try:
        file_obj.write(text)
    except UnicodeEncodeError:
        safe_text = text.encode("utf-8", errors="replace").decode("utf-8")
        file_obj.write(safe_text)


# ── Data Loading ──────────────────────────────────────────────────


def load_abilities():
    abilities_file = Path(__file__).parent.parent / "abilities.json"
    with open(abilities_file, encoding="utf-8") as f:
        return json.load(f).get("unique_abilities", [])


def load_tested_card_ids():
    """Scan test files, return set of all card IDs referenced."""
    test_dir = Path(__file__).parent.parent.parent / "engine" / "tests" / "test_modules"
    tested = set()
    for tf in sorted(test_dir.glob("*.rs")):
        content = tf.read_text(encoding="utf-8", errors="replace")
        for m in re.finditer(r'"((?:PL|LL)![^"]+)"', content):
            tested.add(m.group(1))
    return tested


def extract_card_id(ref):
    return ref.split("|")[0].strip() if "|" in ref else ref.strip()


# ── Key-combo extraction ──────────────────────────────────────────


def get_key_combo(obj, skip_keys=None):
    """Return a frozenset of field names from a JSON object."""
    if skip_keys is None:
        skip_keys = {
            "action",
            "type",
            "text",
            "full_text",
            "triggerless_text",
            "triggers",
        }
    if obj is None:
        return frozenset()
    return frozenset(k for k in obj if k not in skip_keys)


def is_tested_ability(ability, tested_ids):
    cards = ability.get("cards", [])
    return any(extract_card_id(c) in tested_ids for c in cards)


def main():
    abilities = load_abilities()
    tested_ids = load_tested_card_ids()

    # Per-action: map key combo → {total, tested, sample_untested_ability}
    action_combo_stats = defaultdict(
        lambda: defaultdict(lambda: {"total": 0, "tested": 0, "untested_card_ids": []})
    )

    # Per-cost-type: same
    cost_combo_stats = defaultdict(
        lambda: defaultdict(lambda: {"total": 0, "tested": 0, "untested_card_ids": []})
    )

    # Action+Cost type pairings
    action_cost_pairs = defaultdict(lambda: {"total": 0, "tested": 0, "cards": []})

    # Per-trigger stats
    trigger_stats = defaultdict(lambda: {"total": 0, "tested": 0})

    # Overall per-action type stats
    per_action_total = defaultdict(int)
    per_action_tested = defaultdict(int)

    # Highest-depth effects (nested actions within sequential etc.)
    action_max_depth = defaultdict(int)

    for ab in abilities:
        effect = ab.get("effect") or {}
        cost = ab.get("cost") or {}
        action = effect.get("action", "unknown")
        cost_type = cost.get("type", "unknown")
        trigger = ab.get("triggers", "")
        tested = is_tested_ability(ab, tested_ids)
        card_ids = [extract_card_id(c) for c in ab.get("cards", [])]

        # Per-action key combo
        combo = get_key_combo(effect)
        action_combo_stats[action][combo]["total"] += 1
        if tested:
            action_combo_stats[action][combo]["tested"] += 1
        else:
            action_combo_stats[action][combo]["untested_card_ids"].extend(card_ids)

        # Per-cost-type key combo
        cost_combo = get_key_combo(cost)
        cost_combo_stats[cost_type][cost_combo]["total"] += 1
        if tested:
            cost_combo_stats[cost_type][cost_combo]["tested"] += 1
        else:
            cost_combo_stats[cost_type][cost_combo]["untested_card_ids"].extend(
                card_ids
            )

        # Action+Cost pairing
        pair_key = "{} + {}".format(action, cost_type)
        action_cost_pairs[pair_key]["total"] += 1
        action_cost_pairs[pair_key]["cards"].extend(card_ids)
        if tested:
            action_cost_pairs[pair_key]["tested"] += 1

        # Per-trigger
        for t in (trigger or "").split(","):
            t = t.strip()
            if t:
                trigger_stats[t]["total"] += 1
                if tested:
                    trigger_stats[t]["tested"] += 1

        # Per-action totals
        per_action_total[action] += 1
        if tested:
            per_action_tested[action] += 1

    # ── Write Full Report ──────────────────────────────────────────

    output_dir = Path(__file__).parent
    with open(output_dir / "key_combo_coverage_report.txt", "w", encoding="utf-8") as f:
        # ─── 1. OVERALL SUMMARY ───
        safe_write("=" * 70 + "\n", f)
        safe_write("KEY COMBINATION COVERAGE REPORT\n", f)
        safe_write("=" * 70 + "\n\n", f)

        total_abilities = len(abilities)
        total_tested = sum(1 for a in abilities if is_tested_ability(a, tested_ids))
        total_untested = total_abilities - total_tested

        safe_write("Total unique abilities:      {}\n".format(total_abilities), f)
        safe_write(
            "Tested:                      {} ({:.1f}%)\n".format(
                total_tested, 100 * total_tested / max(total_abilities, 1)
            ),
            f,
        )
        safe_write(
            "Untested:                    {} ({:.1f}%)\n".format(
                total_untested, 100 * total_untested / max(total_abilities, 1)
            ),
            f,
        )
        safe_write("Tested card IDs in tests:    {}\n\n".format(len(tested_ids)), f)

        # ─── 2. ACTION TYPE COVERAGE (sorted worst first) ───
        safe_write("─" * 70 + "\n", f)
        safe_write("ACTION TYPE COVERAGE (worst first)\n", f)
        safe_write("─" * 70 + "\n\n", f)

        action_coverage = []
        for action in per_action_total:
            t = per_action_tested.get(action, 0)
            total = per_action_total[action]
            pct = 100 * t / max(total, 1)
            combos = action_combo_stats[action]
            untested_combos = sum(1 for c in combos if combos[c]["tested"] == 0)
            total_combos = len(combos)
            action_coverage.append(
                (pct, action, t, total, total_combos, untested_combos)
            )

        safe_write(
            "  {:<30s} {:>6s} {:>6s} {:>6s} {:>8s} {:>8s}\n".format(
                "Action", "Tested", "Total", "%", "Combos", "Untested"
            ),
            f,
        )
        safe_write("  " + "-" * 70 + "\n", f)
        for pct, action, t, total, total_c, untested_c in sorted(action_coverage):
            safe_write(
                "  {:<30s} {:>6d} {:>6d} {:>5.0f}% {:>8d} {:>8d}\n".format(
                    action, t, total, pct, total_c, untested_c
                ),
                f,
            )

        safe_write("\n", f)

        # ─── 3. WORST KEY COMBOS PER ACTION ───
        safe_write("─" * 70 + "\n", f)
        safe_write("UNTESTED KEY COMBINATIONS BY ACTION\n", f)
        safe_write("─" * 70 + "\n\n", f)

        for action in sorted(action_combo_stats.keys()):
            combos = action_combo_stats[action]
            untested_combos = [
                (combo, stats)
                for combo, stats in combos.items()
                if stats["tested"] == 0
            ]
            if not untested_combos:
                continue

            safe_write(
                "### {} ({} abilities, {} untested combos)\n\n".format(
                    action, per_action_total.get(action, 0), len(untested_combos)
                ),
                f,
            )

            for combo, stats in sorted(untested_combos, key=lambda x: -x[1]["total"]):
                keys = sorted(combo) if combo else ["(no keys besides action)"]
                safe_write(
                    "  [{:3d} untested] keys: {}\n".format(
                        stats["total"], ", ".join(keys)
                    ),
                    f,
                )
                sample_ids = stats["untested_card_ids"][:5]
                if sample_ids:
                    safe_write(
                        "             sample cards: {}\n".format(", ".join(sample_ids)),
                        f,
                    )
                if len(stats["untested_card_ids"]) > 5:
                    safe_write(
                        "             ... and {} more\n".format(
                            len(stats["untested_card_ids"]) - 5
                        ),
                        f,
                    )
            safe_write("\n", f)

        # ─── 4. COST TYPE COVERAGE ───
        safe_write("─" * 70 + "\n", f)
        safe_write("COST TYPE KEY COMBO COVERAGE\n", f)
        safe_write("─" * 70 + "\n\n", f)

        cost_action_total = defaultdict(int)
        for ab in abilities:
            cost = ab.get("cost") or {}
            ct = cost.get("type", "unknown")
            cost_action_total[ct] += 1

        cost_action_tested = defaultdict(int)
        for ab in abilities:
            cost = ab.get("cost") or {}
            ct = cost.get("type", "unknown")
            if is_tested_ability(ab, tested_ids):
                cost_action_tested[ct] += 1

        for cost_type in sorted(cost_action_total.keys()):
            t = cost_action_tested.get(cost_type, 0)
            total = cost_action_total[cost_type]
            pct = 100 * t / max(total, 1)
            combos = cost_combo_stats[cost_type]
            safe_write(
                "  {} ({} abilities, {} tested = {:.0f}%)\n".format(
                    cost_type, total, t, pct
                ),
                f,
            )
            untested_combos = [(c, s) for c, s in combos.items() if s["tested"] == 0]
            if untested_combos:
                for combo, stats in sorted(
                    untested_combos, key=lambda x: -x[1]["total"]
                ):
                    keys = sorted(combo) if combo else ["(no keys)"]
                    safe_write(
                        "    [{:3d}x] keys: {}\n".format(
                            stats["total"], ", ".join(keys)
                        ),
                        f,
                    )
            safe_write("\n", f)

        # ─── 5. ACTION + COST PAIRING COVERAGE ───
        safe_write("─" * 70 + "\n", f)
        safe_write("ACTION + COST TYPE PAIRINGS (worst first)\n", f)
        safe_write("─" * 70 + "\n\n", f)

        pairs_sorted = sorted(
            action_cost_pairs.items(),
            key=lambda x: (x[1]["tested"] / max(x[1]["total"], 1), x[1]["total"]),
        )
        safe_write(
            "  {:<40s} {:>6s} {:>6s} {:>6s}\n".format(
                "Action + Cost", "Total", "Tested", "%"
            ),
            f,
        )
        safe_write("  " + "-" * 60 + "\n", f)
        for pair_key, stats in pairs_sorted:
            pct = 100 * stats["tested"] / max(stats["total"], 1)
            safe_write(
                "  {:<40s} {:>6d} {:>6d} {:>5.0f}%\n".format(
                    pair_key, stats["total"], stats["tested"], pct
                ),
                f,
            )

        safe_write("\n", f)

        # ─── 6. TRIGGER COVERAGE ───
        safe_write("─" * 70 + "\n", f)
        safe_write("TRIGGER TYPE COVERAGE\n", f)
        safe_write("─" * 70 + "\n\n", f)

        safe_write(
            "  {:<30s} {:>6s} {:>6s} {:>6s}\n".format(
                "Trigger", "Total", "Tested", "%"
            ),
            f,
        )
        safe_write("  " + "-" * 50 + "\n", f)
        for trigger in sorted(trigger_stats.keys()):
            stats = trigger_stats[trigger]
            pct = 100 * stats["tested"] / max(stats["total"], 1)
            safe_write(
                "  {:<30s} {:>6d} {:>6d} {:>5.0f}%\n".format(
                    trigger, stats["total"], stats["tested"], pct
                ),
                f,
            )

        safe_write("\n", f)

        # ─── 7. KEY COMBOS THAT *ARE* TESTED (for reference) ───
        safe_write("─" * 70 + "\n", f)
        safe_write("TESTED KEY COMBINATIONS (reference for writing new tests)\n", f)
        safe_write("─" * 70 + "\n\n", f)

        for action in sorted(action_combo_stats.keys()):
            combos = action_combo_stats[action]
            tested_combos = [
                (combo, stats) for combo, stats in combos.items() if stats["tested"] > 0
            ]
            if not tested_combos:
                continue

            safe_write(
                "  {} ({} tested combos of {}):\n".format(
                    action, len(tested_combos), len(combos)
                ),
                f,
            )
            for combo, stats in sorted(tested_combos, key=lambda x: -x[1]["total"]):
                keys = sorted(combo) if combo else ["(no keys)"]
                safe_write(
                    "    [{:3d}x tested] keys: {}\n".format(
                        stats["tested"], ", ".join(keys)
                    ),
                    f,
                )
            safe_write("\n", f)

        # ─── 8. ALL EXISTING KEY COMBOS (complete reference) ───
        safe_write("─" * 70 + "\n", f)
        safe_write("ALL EXISTING KEY COMBINATIONS (complete schema reference)\n", f)
        safe_write("─" * 70 + "\n\n", f)

        for action in sorted(action_combo_stats.keys()):
            combos = action_combo_stats[action]
            safe_write(
                "### {} ({} total abilities)\n\n".format(
                    action, per_action_total.get(action, 0)
                ),
                f,
            )
            for combo, stats in sorted(combos.items(), key=lambda x: -x[1]["total"]):
                keys = sorted(combo) if combo else ["(action only)"]
                tested_pct = 100 * stats["tested"] / max(stats["total"], 1)
                safe_write(
                    "  {:3d} abilities ({:.0f}% tested): {}\n".format(
                        stats["total"], tested_pct, ", ".join(keys)
                    ),
                    f,
                )
            safe_write("\n", f)

    # ── Write Short Summary ───────────────────────────────────────
    with open(output_dir / "least_tested_summary.txt", "w", encoding="utf-8") as f:
        safe_write("LEAST-TESTED ABILITY GAPS\n", f)
        safe_write("=" * 60 + "\n\n", f)
        safe_write(
            "Overall: {} tested / {} total = {:.0f}%\n\n".format(
                total_tested,
                total_abilities,
                100 * total_tested / max(total_abilities, 1),
            ),
            f,
        )

        # Top-10 worst action types
        safe_write("WORST-COVERED ACTION TYPES:\n", f)
        safe_write("-" * 40 + "\n", f)
        for pct, action, t, total, total_c, untested_c in sorted(action_coverage)[:10]:
            safe_write(
                "  {:6.0f}% {:30s} ({}/{} tested, {} untested key combos)\n".format(
                    pct, action, t, total, untested_c
                ),
                f,
            )

        safe_write("\nWORST-COVERED KEY COMBINATIONS (top-30 by count):\n", f)
        safe_write("-" * 40 + "\n", f)

        all_untested = []
        for action in action_combo_stats:
            for combo, stats in action_combo_stats[action].items():
                if stats["tested"] == 0:
                    all_untested.append(
                        (
                            stats["total"],
                            action,
                            sorted(combo) if combo else ["(action only)"],
                            stats["untested_card_ids"][:3],
                        )
                    )

        for count, action, keys, cards in sorted(all_untested, key=lambda x: -x[0])[
            :30
        ]:
            safe_write(
                "  {:4d}x {:30s} keys: {}\n".format(count, action, ", ".join(keys)), f
            )

        safe_write("\nTOP-10 WORST COST TYPE COVERAGE:\n", f)
        safe_write("-" * 40 + "\n", f)
        cost_coverage = []
        for ct in cost_action_total:
            t = cost_action_tested.get(ct, 0)
            total = cost_action_total[ct]
            pct = 100 * t / max(total, 1)
            cost_coverage.append((pct, ct, t, total))
        for pct, ct, t, total in sorted(cost_coverage)[:10]:
            safe_write("  {:6.0f}% {:30s} ({}/{})\n".format(pct, ct, t, total), f)

    print("Coverage report: key_combo_coverage_report.txt")
    print("Quick summary:   least_tested_summary.txt")
    print(
        "Total abilities: {} | Tested: {} ({:.0f}%) | Untested key combos: {}".format(
            total_abilities,
            total_tested,
            100 * total_tested / max(total_abilities, 1),
            sum(
                1
                for a in action_combo_stats
                for c in action_combo_stats[a]
                if action_combo_stats[a][c]["tested"] == 0
            ),
        )
    )


if __name__ == "__main__":
    main()
