"""Shared bot_arena audit reader.

Every analyzer in this directory used to carry its own copy of the JSONL
loader. They disagreed: several of them read ``view["opponent"]``, a key the
arena stopped emitting in favour of ``view["opponent_public"]``, and simply
raised ``KeyError`` or ``TypeError`` on every non-empty audit file (see the
"fix hunt_bad_plays.py" note in AGENTS.md). One loader, one schema, one place
to learn the field names.

The schema this module guarantees:

``run_start``
    ``bots`` (2 element list of CLI names, P1 then P2), ``deck``,
    ``base_seed``, ``games``, ``schema_version``, ``visibility``,
    ``identity_note``, ``iteration_cap_per_game``, ``rng_limitations``.
``game_start``
    ``game``, ``arena_seed``, ``engine_seed``.
``decision``
    ``game``, ``turn``, ``phase``, ``policy_player``, ``decision``,
    ``active_player``, ``chosen``, ``available_actions``,
    ``pending_choice``, ``pending_choice_player``,
    ``live_selected_hand_indices_before``,
    ``mulligan_selected_hand_indices_before``, ``selection_operation``,
    ``v7_live_note``, ``view``.
    ``view["own"]`` has ``player``, ``is_first_attacker``, ``hand``,
    ``stage_left_center_right``, ``energy``, ``success_count``, ``success``.
    ``view["opponent_public"]`` has ``stage_left_center_right``,
    ``success_count``, ``success`` - PUBLIC zones only, never hand or deck.
``action_result``
    ``game``, ``decision``, ``execution_ok``.
``game_end``
    ``game``, ``turn``, ``game_result``, ``success_counts``, ``decisions``,
    ``end_reason``, ``execution_errors``.

Note that the arena writes one JSON object per line and rows are NOT globally
ordered across events, so everything here groups by ``game`` explicitly rather
than relying on file order.
"""

from __future__ import annotations

import json
from collections import defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Iterable, Iterator

# Phases in which the acting player is choosing where/what to do in the Main
# phase, as opposed to the mulligan, the live-card set, or the performance.
MAIN_PHASES = ("Main",)

SET_PHASES = ("LiveCardSetFirstAttacker", "LiveCardSetSecondAttacker")
MULLIGAN_PHASES = ("MulliganFirstAttacker", "MulliganSecondAttacker")
PERFORMANCE_PHASES = (
    "FirstAttackerPerformance",
    "SecondAttackerPerformance",
    "FirstAttackerPerformanceExtra",
    "SecondAttackerPerformanceExtra",
)

LIVE_TYPE = "live_card"
MEMBER_TYPE = "member_card"
ENERGY_TYPE = "energy_card"


def load_events(path: str | Path) -> list[dict[str, Any]]:
    """Read an audit JSONL file into a list of event dicts.

    Malformed lines are reported with their line number instead of raising
    blindly, because a truncated final line is a normal thing to find after an
    interrupted run.
    """
    path = Path(path)
    if not path.exists():
        raise FileNotFoundError(f"audit file not found: {path}")
    events: list[dict[str, Any]] = []
    problems: list[str] = []
    with path.open("r", encoding="utf-8") as handle:
        for lineno, line in enumerate(handle, start=1):
            line = line.strip()
            if not line:
                continue
            try:
                events.append(json.loads(line))
            except json.JSONDecodeError as exc:
                problems.append(f"{path}:{lineno}: {exc}")
    if problems and not events:
        raise ValueError(
            "no parseable events in "
            f"{path}; first problems: {'; '.join(problems[:3])}"
        )
    for problem in problems[:5]:
        print(f"warning: skipped malformed line: {problem}")
    return events


def _own(view: dict[str, Any] | None) -> dict[str, Any]:
    return (view or {}).get("own", {}) or {}


def opponent_public(view: dict[str, Any] | None) -> dict[str, Any]:
    """Public opponent snapshot.

    The key is ``opponent_public``. Older analyzers in this directory read
    ``view["opponent"]`` and broke; this is the supported accessor.
    """
    return (view or {}).get("opponent_public", {}) or {}


def own_hand(view: dict[str, Any] | None) -> list[dict[str, Any]]:
    return _own(view).get("hand", []) or []


def own_stage(view: dict[str, Any] | None) -> list[Any]:
    return _own(view).get("stage_left_center_right", []) or []


def own_success_count(view: dict[str, Any] | None) -> int:
    return int(_own(view).get("success_count", 0) or 0)


def own_success(view: dict[str, Any] | None) -> list[dict[str, Any]]:
    return _own(view).get("success", []) or []


def opponent_success_count(view: dict[str, Any] | None) -> int:
    return int(opponent_public(view).get("success_count", 0) or 0)


def opponent_stage(view: dict[str, Any] | None) -> list[Any]:
    return opponent_public(view).get("stage_left_center_right", []) or []


def own_energy(view: dict[str, Any] | None) -> int:
    energy = _own(view).get("energy", {}) or {}
    return int(energy.get("active", 0) or 0)


def is_first_attacker(view: dict[str, Any] | None) -> bool:
    return bool(_own(view).get("is_first_attacker", False))


def chosen_card(chosen: dict[str, Any] | None) -> dict[str, Any] | None:
    return (chosen or {}).get("resolved_card")


def chosen_card_no(chosen: dict[str, Any] | None) -> str | None:
    card = chosen_card(chosen)
    return card.get("card_no") if card else None


def action_card_no(action: dict[str, Any] | None) -> str | None:
    card = chosen_card(action)
    return card.get("card_no") if card else None


def action_is_live(action: dict[str, Any] | None) -> bool:
    card = chosen_card(action)
    return bool(card) and card.get("card_type") == LIVE_TYPE


def action_is_member(action: dict[str, Any] | None) -> bool:
    card = chosen_card(action)
    return bool(card) and card.get("card_type") == MEMBER_TYPE


def card_hearts_total(card: dict[str, Any] | None) -> int:
    """Printed base hearts summed across every colour, 0 if absent."""
    if not card:
        return 0
    return int(sum((card.get("base_hearts") or {}).values()))


def card_blade_hearts_total(card: dict[str, Any] | None) -> int:
    if not card:
        return 0
    return int(sum((card.get("blade_hearts") or {}).values()))


def card_requirement_total(card: dict[str, Any] | None) -> int:
    if not card:
        return 0
    return int(sum((card.get("required_hearts") or {}).values()))


def hand_lives(hand: Iterable[dict[str, Any]]) -> list[dict[str, Any]]:
    return [card for card in hand if card.get("card_type") == LIVE_TYPE]


def hand_members(hand: Iterable[dict[str, Any]]) -> list[dict[str, Any]]:
    return [card for card in hand if card.get("card_type") == MEMBER_TYPE]


def stage_cost(view: dict[str, Any] | None) -> int:
    """Total printed cost of the three stage slots. The guides' development
    metric (docs/BOT_STRATEGY.md section 1: T1 about 4, T2 about 9, T3 about 13).
    """
    total = 0
    for card in own_stage(view):
        if card:
            total += int(card.get("base_cost") or 0)
    return total


def opponent_stage_cost(view: dict[str, Any] | None) -> int:
    total = 0
    for card in opponent_stage(view):
        if card:
            total += int(card.get("base_cost") or 0)
    return total


def board_hearts(view: dict[str, Any] | None) -> int:
    total = 0
    for card in own_stage(view):
        if card:
            total += card_hearts_total(card)
    return total


def board_blades(view: dict[str, Any] | None) -> int:
    total = 0
    for card in own_stage(view):
        if card:
            total += int(card.get("base_blades") or 0)
    return total


def opponent_board_hearts(view: dict[str, Any] | None) -> int:
    total = 0
    for card in opponent_stage(view):
        if card:
            total += card_hearts_total(card)
    return total


def opponent_board_blades(view: dict[str, Any] | None) -> int:
    total = 0
    for card in opponent_stage(view):
        if card:
            total += int(card.get("base_blades") or 0)
    return total


@dataclass
class Game:
    """One game, assembled from its decision/action_result/game_end rows."""

    game: int
    decisions: list[dict[str, Any]] = field(default_factory=list)
    action_results: dict[int, bool] = field(default_factory=dict)
    end: dict[str, Any] | None = None

    @property
    def result(self) -> str | None:
        return (self.end or {}).get("game_result")

    @property
    def success_counts(self) -> tuple[int, int]:
        counts = (self.end or {}).get("success_counts") or [0, 0]
        return (int(counts[0]), int(counts[1]))

    def decisions_for(self, player: str) -> list[dict[str, Any]]:
        return [d for d in self.decisions if d.get("policy_player") == player]

    def main_decisions(self, player: str) -> list[dict[str, Any]]:
        return [
            d
            for d in self.decisions_for(player)
            if d.get("phase") in MAIN_PHASES
            and not d.get("pending_choice")
        ]

    def set_decisions(self, player: str) -> list[dict[str, Any]]:
        return [d for d in self.decisions_for(player) if d.get("phase") in SET_PHASES]

    def mulligan_decisions(self, player: str) -> list[dict[str, Any]]:
        return [d for d in self.decisions_for(player) if d.get("phase") in MULLIGAN_PHASES]

    def set_commits(self, player: str) -> list[dict[str, Any]]:
        """One record per live-card-set resolution: the confirm action."""
        return [
            d
            for d in self.set_decisions(player)
            if (d.get("chosen") or {}).get("action_type") == "ConfirmLiveCardSet"
        ]


@dataclass
class Run:
    """A whole audit file: header, games, and the failures worth surfacing."""

    path: Path
    header: dict[str, Any]
    games: dict[int, Game]

    @property
    def bots(self) -> list[str]:
        return list(self.header.get("bots") or ["p1", "p2"])

    @property
    def p1_bot(self) -> str:
        return self.bots[0] if self.bots else "p1"

    @property
    def p2_bot(self) -> str:
        return self.bots[1] if len(self.bots) > 1 else "p2"

    @property
    def game_numbers(self) -> list[int]:
        return sorted(self.games)

    def decisions(self, player: str | None = None) -> Iterator[dict[str, Any]]:
        for number in self.game_numbers:
            for decision in self.games[number].decisions:
                if player is None or decision.get("policy_player") == player:
                    yield decision

    def execution_errors(self) -> list[tuple[int, int]]:
        """(game, decision) pairs whose chosen action the engine rejected."""
        return [
            (number, decision_id)
            for number in self.game_numbers
            for decision_id, ok in self.games[number].action_results.items()
            if not ok
        ]

    def scoreboard(self) -> dict[str, Any]:
        """Win / loss / draw, read off ``game_end.success_counts``.

        The winner is whoever reached three successes (1.2.1.1). Deriving it
        from the counts rather than from ``game_result`` plus a
        ``is_first_attacker`` flag avoids a real trap: ``is_first_attacker`` is
        not final during the mulligan, so "first p1 decision wins" resolves
        every game to the same attacker and silently inverts the scoreboard.
        ``game_result`` is still reported as a raw histogram so the two can be
        cross-checked.
        """
        out = {"p1": {"win": 0, "loss": 0, "draw": 0}, "p2": {"win": 0, "loss": 0, "draw": 0}}
        results: dict[str, int] = defaultdict(int)
        mismatches: list[int] = []
        for number in self.game_numbers:
            game = self.games[number]
            result = game.result
            if result is None:
                continue
            results[result] += 1
            p1_count, p2_count = game.success_counts
            if p1_count >= 3 and p2_count >= 3:
                out["p1"]["draw"] += 1
                out["p2"]["draw"] += 1
                p1_won = None
            elif p1_count >= 3:
                out["p1"]["win"] += 1
                out["p2"]["loss"] += 1
                p1_won = True
            elif p2_count >= 3:
                out["p1"]["loss"] += 1
                out["p2"]["win"] += 1
                p1_won = False
            else:
                out["p1"]["draw"] += 1
                out["p2"]["draw"] += 1
                p1_won = None
            if p1_won is not None:
                expected = (
                    "FirstAttackerWins" if p1_won else "SecondAttackerWins"
                )
                if result != expected:
                    mismatches.append(number)
        return {
            "seats": out,
            "results": dict(results),
            "result_mismatches": mismatches,
        }


def build_run(path: str | Path) -> Run:
    events = load_events(path)
    header = next((e for e in events if e.get("event") == "run_start"), {})
    games: dict[int, Game] = defaultdict(lambda: Game(game=-1))
    for event in events:
        kind = event.get("event")
        if kind == "game_start":
            games[int(event["game"])] = Game(game=int(event["game"]))
        elif kind == "decision":
            number = int(event["game"])
            games[number].game = number
            games[number].decisions.append(event)
        elif kind == "action_result":
            number = int(event["game"])
            games[number].action_results[int(event["decision"])] = bool(
                event.get("execution_ok")
            )
        elif kind == "game_end":
            number = int(event["game"])
            games[number].game = number
            games[number].end = event
    return Run(path=Path(path), header=header, games=dict(games))


def per_game_summaries(run: Run) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for number in run.game_numbers:
        game = run.games[number]
        if game.end is None:
            continue
        row: dict[str, Any] = {
            "game": number,
            "result": game.result,
            "turns": (game.end or {}).get("turn"),
            "decisions": (game.end or {}).get("decisions"),
            "p1_success": game.success_counts[0],
            "p2_success": game.success_counts[1],
        }
        for seat, player in (("p1", "p1"), ("p2", "p2")):
            mains = game.main_decisions(player)
            passes = sum(
                1
                for d in mains
                if (d.get("chosen") or {}).get("action_type") == "Pass"
            )
            commits = game.set_commits(player)
            folds = sum(1 for d in commits if not d.get("live_selected_hand_indices_before"))
            row[f"{seat}_main"] = len(mains)
            row[f"{seat}_main_pass"] = passes
            row[f"{seat}_sets"] = len(commits)
            row[f"{seat}_folds"] = folds
        rows.append(row)
    return rows


def main(argv: list[str] | None = None) -> int:
    import argparse

    parser = argparse.ArgumentParser(description="Inspect a bot_arena audit file.")
    parser.add_argument("path", help="audit .jsonl written by bot_arena --audit")
    parser.add_argument(
        "--errors", action="store_true", help="list decisions the engine rejected"
    )
    args = parser.parse_args(argv)

    run = build_run(args.path)
    print(f"file   {run.path}")
    print(f"bots   P1={run.p1_bot}  P2={run.p2_bot}")
    print(f"deck   {run.header.get('deck')}")
    print(f"seed   {run.header.get('base_seed')}  games={run.header.get('games')}")
    print(f"loaded {len(run.game_numbers)} games, {sum(len(g.decisions) for g in run.games.values())} decisions")
    errors = run.execution_errors()
    print(f"execution errors: {len(errors)}")
    if args.errors:
        for game, decision in errors:
            print(f"  game {game} decision {decision}")
    board = run.scoreboard()
    print(f"results: {board['results']}")
    print(f"p1 {board['seats']['p1']}   p2 {board['seats']['p2']}")
    if board["result_mismatches"]:
        print(
            f"warning: game_result disagrees with success_counts in "
            f"{len(board['result_mismatches'])} games: {board['result_mismatches'][:10]}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
