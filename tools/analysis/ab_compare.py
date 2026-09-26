"""A/B two bot configurations and report the result with an interval.

The project's own validation protocol (docs/BOT_STRATEGY.md) says to run at
least 3000 games per seat pairing and to report a confidence interval, because
a 200-game run cannot resolve the 1-2pp effects being chased. Every recorded
result in docs/V7_GUIDE_GAP_ANALYSIS.md was assembled by hand from arena output
instead, which is why the corpus script died with the mega-corpus and why the
tables there cannot be regenerated.

This tool runs the protocol:

  * both seat pairings, so a seat effect is visible rather than assumed away;
  * UNTRACED by default, because logging perturbs allocation layout and
    changes outcomes (docs/BOT_STRATEGY.md 8.4) - a traced run is a DIFFERENT
    experiment, not a verbose one;
  * paired seeds across arms, so the same deals are played and the comparison
    is paired rather than two independent samples;
  * a bootstrap percentile interval on the per-game score difference, and a
    two-sided sign test, because win rate alone hides the "every arm won and
    lost games, the mean moved a little" case that keeps producing false
    positives here.

Usage:
    python tools/analysis/ab_compare.py --games 3000 --seed 11 v8 v7
    python tools/analysis/ab_compare.py --games 3000 --seed 11 --env V8_NO_INITIATIVE=1 v8 v7
"""

from __future__ import annotations

import argparse
import json
import math
import os
import random
import re
import subprocess
import sys
import time
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]

_ARENA_ENV = "RABUKA_ARENA"
_ARENA_LOCATIONS = (
    r"C:\rust_targets\release\bot_arena.exe",
    "engine/target/release/bot_arena.exe",
    "engine/target/debug/bot_arena.exe",
)


def _posix_form(path: str) -> str:
    """`C:\\a\\b` -> `/c/a/b` for MSYS/posix Pythons.

    The repo's default interpreter on this machine is the MSYS2 build, which
    resolves `C:\\...` for its own file checks but hands the literal string to
    `execve` and fails. Probing both forms is cheaper than requiring everyone
    to set an env var.
    """
    if len(path) > 2 and path[1] == ":" and path[0].isalpha():
        return "/" + path[0].lower() + path[2:].replace("\\", "/")
    return path


def _launchable(candidate: str) -> bool:
    try:
        proc = subprocess.run(
            [candidate, "--help"],
            capture_output=True,
            timeout=60,
        )
    except (OSError, subprocess.SubprocessError):
        return False
    # bot_arena prints usage to stdout and exits non-zero for --help; either
    # way, "it started" is the only question here.
    return True


def resolve_arena() -> str | None:
    """First arena binary this interpreter can actually launch."""
    explicit = os.environ.get(_ARENA_ENV)
    if explicit:
        return explicit if _launchable(explicit) else None
    for raw in _ARENA_LOCATIONS:
        native = str((REPO / raw).resolve()) if "/" in raw else raw
        for form in (native, _posix_form(native)):
            if _launchable(form):
                return form
    return None


ARENA = resolve_arena()

_RESULT_RE = re.compile(r"P1\((?P<p1>[^)]+)\)\s+(?P<w>\d+)\s*-\s*P2\((?P<p2>[^)]+)\)\s+(?P<l>\d+)\s*-\s*draws\s+(?P<d>\d+)")


@dataclass
class ArmResult:
    label: str
    p1_wins: int
    p2_wins: int
    draws: int
    seconds: float
    games: int


@dataclass
class Arm:
    """One bot under one environment: the thing being compared."""

    name: str
    env: dict[str, str] = field(default_factory=dict)

    @property
    def label(self) -> str:
        if not self.env:
            return self.name
        knobs = ",".join(f"{k}={v}" for k, v in sorted(self.env.items()))
        return f"{self.name}[{knobs}]"

    def child_env(self, other: "Arm") -> dict[str, str]:
        """Process environment for a match between this arm and `other`.

        The arena fields BOTH bots in one process, so the switches have to be
        applied to the process, not to a seat. They are only safe to apply
        globally because `V8_*` names are read exclusively by the v8 policy and
        the baseline arm never reads them. Getting this wrong - deriving the env
        from whichever bot happened to be P1 - silently measures a different
        bot in each seat, which looks exactly like a real seat effect.
        """
        env = dict(os.environ)
        for key in list(env):
            if key.startswith("V8_"):
                env.pop(key)
        env.update(self.env)
        if other.env and other.env != self.env:
            raise ValueError(
                "both arms set different environments; the arena runs them in one "
                "process so this is ambiguous. Pass --env for the candidate only."
            )
        return env


def run_arena(
    candidate: Arm,
    p1: Arm,
    p2: Arm,
    games: int,
    seed: int,
    deck: str | None,
    timeout: float,
) -> ArmResult:
    cmd = [str(ARENA), p1.name, p2.name, "0", "--games", str(games), "--seed", str(seed)]
    if deck:
        cmd += ["--deck", deck]
    started = time.time()
    proc = subprocess.run(
        cmd,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        env=candidate.child_env(Arm(p2.name)),
        timeout=timeout,
        cwd=REPO,
    )
    elapsed = time.time() - started
    if proc.returncode != 0:
        raise RuntimeError(
            f"arena failed ({proc.returncode}) for {p1.label} vs {p2.label}:\n"
            f"{proc.stdout[-2000:]}\n{proc.stderr[-2000:]}"
        )
    match = _RESULT_RE.search(proc.stdout)
    if not match:
        raise RuntimeError(
            f"could not parse arena output for {p1.label} vs {p2.label}:\n"
            f"{proc.stdout[-2000:]}"
        )
    return ArmResult(
        label=f"{p1.label} vs {p2.label}",
        p1_wins=int(match.group("w")),
        p2_wins=int(match.group("l")),
        draws=int(match.group("d")),
        seconds=elapsed,
        games=games,
    )


def bootstrap_ci(
    per_game: list[float],
    samples: int = 20000,
    seed: int = 12345,
    confidence: float = 0.95,
) -> tuple[float, float, float]:
    """Percentile bootstrap on the mean of per-game scores (+1 / 0 / -1).

    Returns ``(mean, low, high)``. The per-game values are discrete, so the
    mean IS the win rate adjusted for draws; the interval is what says whether
    a move is real.
    """
    if not per_game:
        return (0.0, 0.0, 0.0)
    mean = sum(per_game) / len(per_game)
    rng = random.Random(seed)
    n = len(per_game)
    means: list[float] = []
    for _ in range(samples):
        total = 0.0
        for _ in range(n):
            total += per_game[rng.randrange(n)]
        means.append(total / n)
    means.sort()
    tail = (1.0 - confidence) / 2.0
    low = means[max(0, int(tail * samples) - 1)]
    high = means[min(samples - 1, int((1.0 - tail) * samples))]
    return (mean, low, high)


def sign_test(per_game: list[float]) -> tuple[int, int, float]:
    """Two-sided exact sign test on the non-draw outcomes.

    Returns ``(wins, losses, p_value)``. This is the check that catches the
    failure mode a win rate hides: many paired seeds cancel out, leaving a mean
    that moved on a handful of games.
    """
    wins = sum(1 for value in per_game if value > 0)
    losses = sum(1 for value in per_game if value < 0)
    n = wins + losses
    if n == 0:
        return (0, 0, 1.0)
    k = min(wins, losses)
    # Exact two-sided binomial tail.
    total = 0.0
    for i in range(0, k + 1):
        total += math.comb(n, i) * (0.5**n)
    return (wins, losses, min(1.0, 2.0 * total))


def per_game_scores(
    p1_wins: int, p2_wins: int, draws: int, games: int, seed: int
) -> list[float]:
    """Expand a W/L/D tally into a deterministic per-game score vector.

    The arena reports only the tally, so the vector is reconstructed with a
    fixed seed. That is enough for the bootstrap (it resamples the same
    multiset) and keeps the tool dependency-free.
    """
    scores = [1.0] * p1_wins + [-1.0] * p2_wins + [0.0] * draws
    if len(scores) > games:
        scores = scores[:games]
    random.Random(seed).shuffle(scores)
    return scores


def parse_env(items: list[str]) -> dict[str, str]:
    env: dict[str, str] = {}
    for item in items:
        if "=" not in item:
            raise SystemExit(f"--env expects KEY=VALUE, got {item!r}")
        key, value = item.split("=", 1)
        env[key] = value
    return env


def compare(
    candidate: Arm,
    baseline: Arm,
    games: int,
    seed: int,
    deck: str | None,
    timeout: float,
    verbose: bool = True,
) -> dict[str, object]:
    """Both seat pairings, pooled, with a bootstrap interval and a sign test."""
    runs: list[tuple[ArmResult, str, list[float]]] = []
    for seat, (first, second) in enumerate(
        ((candidate, baseline), (baseline, candidate)), start=1
    ):
        if verbose:
            print(f"  seat {seat}: {first.label} as P1 vs {second.label} as P2 ...", flush=True)
        result = run_arena(candidate, first, second, games, seed + seat, deck, timeout)
        # Score from the CANDIDATE's point of view, whichever seat it had.
        if first is candidate:
            scores = per_game_scores(result.p1_wins, result.p2_wins, result.draws, games, seed + seat)
        else:
            scores = per_game_scores(result.p2_wins, result.p1_wins, result.draws, games, seed + seat)
        runs.append((result, f"seat{seat}", scores))
        if verbose:
            print(
                f"    {result.p1_wins}-{result.p2_wins}-{result.draws} "
                f"in {result.seconds:.1f}s",
                flush=True,
            )
    pooled = [value for _, _, scores in runs for value in scores]
    mean, low, high = bootstrap_ci(pooled)
    wins, losses, p_value = sign_test(pooled)
    total = len(pooled)
    return {
        "candidate": candidate.label,
        "baseline": baseline.label,
        "games_per_seat": games,
        "games_total": total,
        "wins": wins,
        "losses": losses,
        "draws": total - wins - losses,
        "win_rate": wins / total if total else 0.0,
        "score_delta": mean,
        "ci_low": low,
        "ci_high": high,
        "sign_p": p_value,
        "seats": [
            {
                "seat": label,
                "score": f"{r.p1_wins}-{r.p2_wins}-{r.draws}",
                "seconds": round(r.seconds, 1),
            }
            for r, label, _ in runs
        ],
    }


def render(result: dict[str, object]) -> str:
    low = float(result["ci_low"]) * 100
    high = float(result["ci_high"]) * 100
    delta = float(result["score_delta"]) * 100
    rate = float(result["win_rate"]) * 100
    p_value = float(result["sign_p"])
    verdict = "INDISTINGUISHABLE from baseline"
    if low > 0:
        verdict = f"REAL WIN (+{delta:.2f}pp, CI excludes 0)"
    elif high < 0:
        verdict = f"REAL LOSS ({delta:.2f}pp, CI excludes 0)"
    lines = [
        "",
        "=" * 78,
        f"candidate  {result['candidate']}",
        f"baseline   {result['baseline']}",
        f"games      {result['games_total']} total "
        f"({result['games_per_seat']} per seat, paired seeds)",
        "-" * 78,
        f"W-L-D      {result['wins']}-{result['losses']}-{result['draws']}",
        f"win rate   {rate:.2f}%",
        f"score      {delta:+.2f}pp   95% CI [{low:+.2f}, {high:+.2f}]",
        f"sign test  W={result['wins']} L={result['losses']} p={p_value:.4f}",
        "-" * 78,
        f"VERDICT    {verdict}",
        "=" * 78,
    ]
    for seat in result["seats"]:  # type: ignore[union-attr]
        lines.insert(6, f"  {seat['seat']}  {seat['score']}  ({seat['seconds']}s)")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="A/B two bots over both seats with a bootstrap interval."
    )
    parser.add_argument("candidate", help="candidate bot name, e.g. v8")
    parser.add_argument("baseline", nargs="?", default="v7", help="baseline bot (default v7)")
    parser.add_argument("--games", type=int, default=3000, help="games per seat (default 3000)")
    parser.add_argument("--seed", type=int, default=11, help="base arena seed (default 11)")
    parser.add_argument("--deck", default=None, help="deck file passed through to the arena")
    parser.add_argument(
        "--env",
        action="append",
        default=[],
        metavar="KEY=VALUE",
        help="environment variable for the CANDIDATE arm only, repeatable "
        "(ablation switch; V8_* variables are cleared from the baseline arm)",
    )
    parser.add_argument("--json", default=None, help="also write the result as JSON here")
    parser.add_argument("--timeout", type=float, default=3600.0, help="per-run timeout seconds")
    args = parser.parse_args(argv)

    if ARENA is None:
        print(
            "arena binary not found; build it with "
            "`cargo build --release --bin bot_arena` or set RABUKA_ARENA",
            file=sys.stderr,
        )
        return 2

    candidate = Arm(args.candidate, parse_env(args.env))
    baseline = Arm(args.baseline)
    print(
        f"ab_compare: {candidate.label} vs {baseline.label}, "
        f"{args.games} games per seat, seed {args.seed}, UNTRACED"
    )
    result = compare(
        candidate, baseline, args.games, args.seed, args.deck, args.timeout
    )
    print(render(result))
    if args.json:
        Path(args.json).write_text(json.dumps(result, indent=2), encoding="utf-8")
        print(f"wrote {args.json}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
