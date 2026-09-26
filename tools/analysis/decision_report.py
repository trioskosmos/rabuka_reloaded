"""What did the bot actually do, and does it match the guide?

`hunt_bad_plays.py` and `analyze_success_zones.py` answer narrower questions and
each re-derives the audit schema. This is the one that reports a run's decisions
against `guide_model.py`: action mix by phase and turn, live-set composition,
the success-zone timeline, and the rule-backed violations from the guide's
decision tree.

The interesting comparisons are between two runs of the same experiment, which
is what `--baseline` is for: "v8 folds 31% of checks where v7 folds 0.3%" is
the sentence that explains a scoreboard, and it has to be readable off the data
rather than reconstructed from a memory of a debug run.

    python tools/analysis/decision_report.py audit_v8.jsonl
    python tools/analysis/decision_report.py audit_v8.jsonl --baseline audit_v7.jsonl
"""

from __future__ import annotations

import argparse
import importlib.util
import os
import sys
from collections import Counter, defaultdict
from pathlib import Path
from types import ModuleType
from typing import Any


def _sibling(name: str) -> str:
    """Locate a sibling module next to this file.

    The repo's default interpreter is the MSYS2 build, whose path handling for
    a script launched with Windows-style separators resolves `__file__`'s
    directory to the REPO root rather than to `tools/analysis`. Rather than
    depend on one of those behaviours, every plausible location is tried and
    the first existing file wins.
    """
    bases = []
    for raw in (__file__, sys.argv[0]):
        if not raw:
            continue
        absolute = os.path.abspath(raw)
        bases.append(os.path.dirname(absolute))
        bases.append(os.path.dirname(os.path.realpath(absolute)))
    # Also walk up from the current directory, which is correct however the
    # script was invoked.
    cwd = os.getcwd()
    for _ in range(6):
        bases.append(cwd)
        cwd = os.path.dirname(cwd)
    seen: set[str] = set()
    for base in bases:
        if not base or base in seen:
            continue
        seen.add(base)
        direct = os.path.join(base, f"{name}.py")
        if os.path.isfile(direct):
            return direct
        nested = os.path.join(base, "tools", "analysis", f"{name}.py")
        if os.path.isfile(nested):
            return nested
    raise ImportError(f"could not locate {name}.py next to {__file__}")


def _load(name: str) -> ModuleType:
    """Import a sibling module by explicit path, independent of sys.path."""
    path = _sibling(name)
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise ImportError(f"cannot load {name} from {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


aio = _load("audit_io")
guide = _load("guide_model")


def _pct(part: int, whole: int) -> str:
    if not whole:
        return "  n/a"
    return f"{100.0 * part / whole:5.1f}%"


class SideStats:
    """Everything measured for one bot across a whole run."""

    def __init__(self, label: str) -> None:
        self.label = label
        self.main_decisions = 0
        self.main_pass = 0
        self.main_by_turn: dict[int, Counter] = defaultdict(Counter)
        self.main_action_types: Counter = Counter()
        self.main_cost_by_turn: dict[int, list[int]] = defaultdict(list)
        self.ability_uses = 0
        self.draws_offered = 0
        self.set_decisions = 0
        self.set_commits = 0
        self.set_lives = 0
        self.set_junk = 0
        self.set_empty = 0
        self.lives_per_set: list[int] = []
        self.junk_per_set: list[int] = []
        self.set_life_scores: list[int] = []
        self.set_board_hearts: list[int] = []
        self.set_requirement: list[int] = []
        self.set_contested = 0
        self.set_blades: list[int] = []
        self.mulligans = 0
        self.mulligan_replacements: list[int] = []
        self.success_at_end: list[int] = []
        self.placements: list[int] = []
        self.violations: Counter = Counter()
        self.violation_examples: dict[str, list[str]] = defaultdict(list)

    def note(self, code: str, detail: str) -> None:
        self.violations[code] += 1
        if len(self.violation_examples[code]) < 3:
            self.violation_examples[code].append(detail)


def collect(run: aio.Run, label: str) -> dict[str, SideStats]:
    # `policy_player` in the audit is the SEAT ("p1"/"p2"), not the bot name;
    # the name comes from the run header. Keying by name and looking up by
    # seat silently matches nothing.
    seat_names = {"p1": run.p1_bot, "p2": run.p2_bot}
    stats = {seat: SideStats(f"{label}:{name} as {seat}") for seat, name in seat_names.items()}

    for decision in run.decisions():
        seat = decision.get("policy_player")
        if seat not in stats:
            continue
        st = stats[seat]
        view = decision.get("view") or {}
        chosen = decision.get("chosen") or {}
        kind = chosen.get("action_type")
        turn = int(decision.get("turn") or 0)

        if decision.get("phase") in aio.MULLIGAN_PHASES:
            if kind == "SelectMulligan":
                st.mulligans += 1
                st.mulligan_replacements.append(
                    len(decision.get("mulligan_selected_hand_indices_before") or []) + 1
                )
            continue

        if decision.get("phase") in aio.SET_PHASES:
            st.set_decisions += 1
            if kind != "ConfirmLiveCardSet":
                continue
            st.set_commits += 1
            selected = decision.get("live_selected_hand_indices_before") or []
            hand = aio.own_hand(view)
            lives = [
                hand[i]
                for i in selected
                if i < len(hand) and hand[i].get("card_type") == aio.LIVE_TYPE
            ]
            st.lives_per_set.append(len(lives))
            st.junk_per_set.append(len(selected) - len(lives))
            if not selected:
                st.set_empty += 1
                if aio.hand_lives(hand):
                    st.note(
                        "set_nothing_when_contestable",
                        f"T{turn} folded holding "
                        f"{len(aio.hand_lives(hand))} life(s)",
                    )
            else:
                st.set_life_scores.append(sum(c.get("base_score") or 0 for c in lives))
                st.set_requirement.append(sum(aio.card_requirement_total(c) for c in lives))
            st.set_board_hearts.append(aio.board_hearts(view))
            st.set_blades.append(aio.board_blades(view))
            if _opponent_committed(decision):
                st.set_contested += 1
            continue

        if decision.get("phase") not in aio.MAIN_PHASES:
            continue
        if decision.get("pending_choice"):
            continue

        st.main_decisions += 1
        st.main_action_types[kind or "?"] += 1
        st.main_by_turn[turn][kind or "?"] += 1
        st.main_cost_by_turn[turn].append(aio.stage_cost(view))
        if kind == "Pass":
            st.main_pass += 1
            affordable = _affordable_member_in_hand(view)
            if affordable is not None:
                st.note(
                    "pass_while_member_affordable",
                    f"T{turn} passed with a cost-{affordable} member and "
                    f"{aio.own_energy(view)} energy",
                )
        if kind == "UseAbility":
            st.ability_uses += 1

    for number in run.game_numbers:
        game = run.games[number]
        if game.end is None:
            continue
        for seat, player in (("p1", "p1"), ("p2", "p2")):
            stats[seat].success_at_end.append(game.success_counts[0 if seat == "p1" else 1])
            stats[seat].placements.append(
                len(game.set_commits(player))
            )
    return stats


def _opponent_committed(decision: dict[str, Any]) -> bool:
    """Did the opponent have a set to contest with?

    The arena's public view exposes the opponent's success zone but not their
    live zone size, so a contested check is inferred the only way the audit
    allows: the opponent had already reached a success, or the decision is the
    second attacker's own set phase (they can see the size).
    """
    view = decision.get("view") or {}
    if aio.opponent_success_count(view) > 0:
        return True
    return decision.get("phase") == "LiveCardSetSecondAttacker"


def _affordable_member_in_hand(view: dict[str, Any]) -> int | None:
    energy = aio.own_energy(view)
    stage = [c for c in aio.own_stage(view) if c]
    discount = max((int(c.get("base_cost") or 0) for c in stage), default=0)
    for card in aio.hand_members(aio.own_hand(view)):
        cost = int(card.get("base_cost") or 0)
        if cost <= 0:
            continue
        if stage:
            if cost - discount <= energy:
                return cost
        elif cost <= energy:
            return cost
    return None


def _mean(values: list[int]) -> float:
    return sum(values) / len(values) if values else 0.0


def render(stats: dict[str, SideStats]) -> str:
    lines: list[str] = []
    for seat in ("p1", "p2"):
        st = stats.get(seat)
        if st is None:
            continue
        lines.append("=" * 78)
        lines.append(st.label)
        lines.append("=" * 78)

        lines.append("-- Main phase " + "-" * 62)
        lines.append(
            f"  decisions {st.main_decisions}   pass {_pct(st.main_pass, st.main_decisions)}"
            f"   abilities {st.ability_uses}"
        )
        top = ", ".join(
            f"{k}={v}" for k, v in st.main_action_types.most_common(6)
        )
        lines.append(f"  action mix  {top}")
        curve_rows = []
        for turn in sorted(st.main_cost_by_turn):
            costs = st.main_cost_by_turn[turn]
            target = guide.CURVE_TARGETS.get(turn)
            band = f"target {target}" if target else "no target"
            ok = guide.curve_ok(turn, _mean(costs))
            curve_rows.append(
                f"    T{turn}: cost {_mean(costs):5.1f}  n={len(costs):3d}  {band}"
                f"  {'OK' if ok else 'BEHIND'}"
            )
        lines.append("  cost curve (guides: T1=4, T2=9, T3=13)")
        lines += curve_rows[:6]

        lines.append("-- Live set " + "-" * 65)
        lines.append(
            f"  commits {st.set_commits}   empty {st.set_empty} "
            f"({_pct(st.set_empty, st.set_commits)})"
        )
        lines.append(
            f"  contested {st.set_contested} ({_pct(st.set_contested, st.set_commits)})"
        )
        lines.append(
            f"  mean lives {_mean(st.lives_per_set):.2f}   mean junk {_mean(st.junk_per_set):.2f}"
            f"   mean set score {_mean(st.set_life_scores):.2f}"
        )
        lines.append(
            f"  mean board hearts {_mean(st.set_board_hearts):.1f}"
            f"   mean blades {_mean(st.set_blades):.1f}"
            f"   mean requirement {_mean(st.set_requirement):.1f}"
        )

        lines.append("-- Mulligan " + "-" * 65)
        if st.mulligans:
            lines.append(
                f"  hands {st.mulligans}   mean replacements "
                f"{_mean(st.mulligan_replacements):.2f}"
            )
        else:
            lines.append("  none recorded (needs --audit)")

        lines.append("-- Guide violations " + "-" * 55)
        if not st.violations:
            lines.append("  none")
        else:
            for code, count in st.violations.most_common():
                meta = guide.VIOLATION_BY_CODE.get(code)
                section = f" [{meta.section}]" if meta else ""
                lines.append(f"  {count:5d}  {code}{section}")
                for example in st.violation_examples[code]:
                    lines.append(f"           e.g. {example}")
        lines.append("")
    return "\n".join(lines)


def delta_lines(new: dict[str, SideStats], old: dict[str, SideStats]) -> list[str]:
    rows = [
        ("main pass rate", lambda s: _pct(s.main_pass, s.main_decisions).strip()),
        ("abilities used", lambda s: str(s.ability_uses)),
        ("live set empty rate", lambda s: _pct(s.set_empty, s.set_commits).strip()),
        ("contested rate", lambda s: _pct(s.set_contested, s.set_commits).strip()),
        ("mean lives per set", lambda s: f"{_mean(s.lives_per_set):.2f}"),
        ("mean junk per set", lambda s: f"{_mean(s.junk_per_set):.2f}"),
        ("mean set score", lambda s: f"{_mean(s.set_life_scores):.2f}"),
        ("mean board hearts", lambda s: f"{_mean(s.set_board_hearts):.1f}"),
        ("mean blades", lambda s: f"{_mean(s.set_blades):.1f}"),
        ("mean replacements", lambda s: f"{_mean(s.mulligan_replacements):.2f}"),
    ]
    lines = ["", "metric                      candidate   baseline   change"]
    lines.append("-" * 78)
    for seat in ("p1", "p2"):
        a = new.get(seat)
        b = old.get(seat)
        if a is None or b is None:
            continue
        lines.append(f"[{seat}]")
        for name, extract in rows:
            try:
                left = float(extract(a))
                right = float(extract(b))
            except ValueError:
                lines.append(f"  {name:<24} {extract(a):>9}   {extract(b):>9}")
                continue
            change = left - right
            lines.append(
                f"  {name:<24} {left:9.2f}   {right:9.2f}   {change:+9.2f}"
            )
    return lines


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Report a run's decisions and how they line up with the guide."
    )
    parser.add_argument("audit", help="audit .jsonl for the candidate run")
    parser.add_argument(
        "--baseline",
        default=None,
        help="audit .jsonl for the baseline run, for a side-by-side delta",
    )
    args = parser.parse_args(argv)

    run = aio.build_run(args.audit)
    print(f"# {run.path.name}: P1={run.p1_bot} P2={run.p2_bot} deck={run.header.get('deck')}")
    print(f"# {len(run.game_numbers)} games")
    board = run.scoreboard()
    print(
        f"# outcome p1 {board['seats']['p1']} p2 {board['seats']['p2']}"
    )
    print(render(collect(run, run.p1_bot)))

    if args.baseline:
        base = aio.build_run(args.baseline)
        print(f"\n# baseline {base.path.name}: P1={base.p1_bot} P2={base.p2_bot}")
        print("\n".join(delta_lines(collect(run, run.p1_bot), collect(base, base.p1_bot))))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
