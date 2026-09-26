"""The strategy guide, as data.

`docs/BOT_STRATEGY.md` is a prose document, so every analyzer that wanted to
check a game against it re-typed the same numbers, usually from memory, usually
differently. This module is the single machine-readable copy of the guide's
QUANTITATIVE claims, each with a section reference so a reader can check it
against the prose.

What is here is only what a decision can be judged against: bands and rules.
Judgement, ordering and game-feel stay in the prose; the checks that encode
them are listed as VIOLATIONS with a rule citation and are deliberately
conservative - a check fires only on a clear, rule-backed deviation, so a
violation count is evidence rather than noise.

Source: docs/BOT_STRATEGY.md, sourced in turn from the official JP guide,
event-seiki 2nd edition, and the deck lists in cards/.
"""

from __future__ import annotations

from dataclasses import dataclass

# --- section 1: turn structure -------------------------------------------
# "5-9 turns. Opening (T1-T2) build the curve to 4 / 9. Mid (T3-T5) pivot to
#  the live check. Late (T6+) all-in on success zones."
TURN_MIN = 5
TURN_MAX = 9
TURN_BANDS = (
    (1, 2, "opening: curve to 4 then 9"),
    (3, 5, "mid: pivot to the live check"),
    (6, 9, "late: all-in on success zones"),
)

# The cost curve the guides aim for. Total printed stage cost.
CURVE_TARGETS = {1: 4, 2: 9, 3: 13}
# Tolerant band around each target, because a list cannot hit it exactly.
CURVE_TOLERANCE = 3

# --- section 2: what actually wins ---------------------------------------
# "Every won check places exactly ONE card no matter the margin (8.4.7). Score
#  only matters inside a contested comparison - so P(win the comparison) is
#  the ONLY quality metric."
PLACEMENTS_TO_WIN = 3
PLACEMENTS_PER_WON_CHECK = 1
# With one placement per live phase per side and 5-9 turns, a live phase
# happens on roughly every other turn.
EXPECTED_PLACEMENTS_PER_LIVE_PHASE = 1.0

# --- section 3.3: hearts required to pass a live, by printed score -------
# Measured across all 291 lives in cards.json. Index = printed score.
SCORE_REQUIREMENT_MEDIAN = {0: 2, 1: 3, 2: 5, 3: 7, 4: 10, 5: 12, 6: 14, 7: 16, 8: 19, 9: 21}
SCORE_REQUIREMENT_MAX = {0: 4, 1: 4, 2: 8, 3: 9, 4: 12, 5: 14, 6: 15, 7: 18, 8: 21, 9: 21}

# --- section 4 / S1: energy and the card economy -------------------------
# 7.5: +1 active energy per turn. 7.6: +1 card per turn. 7.4.1: active energy
# persists. 8.3.4: setting a live card discards it and draws a replacement,
# which is why hand-filtering is a real play and not a cost.
ENERGY_PER_TURN = 1
DRAW_PER_TURN = 1
HAND_SIZE = 6
# 9.6.2.3.2: a baton play costs own_cost - sent_cost.
BATON_DISCOUNT = True
# Stage slots.
STAGE_SLOTS = 3

# --- section 8.4: the live check ----------------------------------------
# 8.2.2: the second attacker sees the set SIZE, not its contents.
SET_SIZE_PUBLIC = True
# 8.3.15 -> 8.3.16: the check is all-or-nothing; a failure discards the zone.
CHECK_IS_ALL_OR_NOTHING = True
# 8.3.4: spare slots draw a replacement.
SPARE_SLOT_DRAWS = True
# 8.4.3.2: the sole passer places, whatever the score.
SOLE_PASSER_PLACES = True
# 8.4.7.1: a tie places only for a player below two successes who set a
# single card. At two each, a tie is a draw game (1.2.1.2).
TIE_PLACES_BELOW_TWO = True
TIE_PLACES_ONLY_SINGLE_CARD_SET = True

# --- section 8.4.2.1: yell ----------------------------------------------
# Blades flip cards from your own deck; Score icons add +1 to your total.
YELL_SCORE_ICON_BONUS = 1
# 8.4.13: the sole placer becomes first attacker next turn.
SOLE_PLACER_BECOMES_FIRST_ATTACKER = True

# --- section 6: the decision tree, as checkable classes -----------------


@dataclass(frozen=True)
class Violation:
    """One rule-backed way a decision can depart from the guide.

    ``code`` is stable and safe to key reports on. ``section`` cites the guide
    so a reader can audit the check itself.
    """

    code: str
    section: str
    summary: str


VIOLATIONS = (
    Violation(
        "pass_while_member_affordable",
        "4 / S1",
        "Passed the Main phase with a member the current energy could pay for, "
        "so the turn was given away rather than banked deliberately.",
    ),
    Violation(
        "pass_while_ability_available",
        "6 (use abilities when they change the board)",
        "Passed with a usable ability that would have changed the board.",
    ),
    Violation(
        "set_nothing_when_contestable",
        "6 / S4 (do not fold a contestable check)",
        "Set nothing in the live zone while holding a life that could contest, "
        "or while the opponent was at two successes and could place freely.",
    ),
    Violation(
        "scored_life_without_pass_probability",
        "3.3 (match the score band to the hearts you have)",
        "Set a live whose heart requirement was far above the board, with no "
        "blades left to cover the gap.",
    ),
    Violation(
        "low_score_contested",
        "3.3 / 8.4.6 (score decides the comparison)",
        "Set a low-scoring life into a CONTESTED comparison, where score is the "
        "only thing that converts a passed check into a placement.",
    ),
    Violation(
        "high_score_uncontested",
        "8.4.3.2 (the sole passer places regardless of score)",
        "Set an unpassable high score into an UNCONTESTED comparison, where "
        "only reliability matters and score is wasted.",
    ),
    Violation(
        "burned_curve_piece_as_junk",
        "3.1a / 7.4 (hand filtering keeps the pieces you need)",
        "Hand-filtered a member or energy that the curve still needed.",
    ),
    Violation(
        "stage_below_curve",
        "1 / S1 (T1 4, T2 9, T3 13)",
        "Stage cost fell below the curve band well past the opening turns.",
    ),
    Violation(
        "overcommitted_hand",
        "3.1a (keep the pieces you still need)",
        "Drew or kept so many cards that the live-card set could not be formed "
        "with a life in it.",
    ),
    Violation(
        "no_success_attempts",
        "S1 (every live phase is a placement attempt)",
        "Reached the end of a live phase having set nothing at all.",
    ),
)

VIOLATION_CODES = tuple(v.code for v in VIOLATIONS)
VIOLATION_BY_CODE = {v.code: v for v in VIOLATIONS}


def curve_band(turn: int) -> str:
    for low, high, label in TURN_BANDS:
        if low <= turn <= high:
            return label
    return "out of band"


def curve_target(turn: int) -> int | None:
    return CURVE_TARGETS.get(turn)


def curve_ok(turn: int, stage_cost: int) -> bool:
    """Is `stage_cost` within tolerance of the guide's target for `turn`?

    Returns True for turns past T3, where the guide's own advice is to stop
    building the curve and pivot, so there is no target to be off.
    """
    target = CURVE_TARGETS.get(turn)
    if target is None:
        return True
    return stage_cost >= target - CURVE_TOLERANCE


def requirement_band(score: int) -> tuple[int, int]:
    """(median, max) hearts needed to pass a live of `score`, clamped."""
    keys = sorted(SCORE_REQUIREMENT_MEDIAN)
    key = min(max(score, keys[0]), keys[-1])
    return (SCORE_REQUIREMENT_MEDIAN[key], SCORE_REQUIREMENT_MAX[key])


def describe() -> str:
    lines = [
        "guide model (docs/BOT_STRATEGY.md)",
        f"  turns            {TURN_MIN}-{TURN_MAX}",
        f"  curve targets    " + ", ".join(f"T{k}={v}" for k, v in CURVE_TARGETS.items()),
        f"  placements to win {PLACEMENTS_TO_WIN} (1 per won check)",
        f"  energy/turn      +{ENERGY_PER_TURN}    draws/turn +{DRAW_PER_TURN}",
        f"  hand size        {HAND_SIZE}   stage slots {STAGE_SLOTS}",
        "  score bands      "
        + ", ".join(
            f"{score}:{SCORE_REQUIREMENT_MEDIAN[score]}/{SCORE_REQUIREMENT_MAX[score]}"
            for score in sorted(SCORE_REQUIREMENT_MEDIAN)
        ),
        f"  violations       {len(VIOLATIONS)} rule-backed checks",
    ]
    return "\n".join(lines)


if __name__ == "__main__":
    print(describe())
