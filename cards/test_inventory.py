#!/usr/bin/env python3
"""Automated ability-coverage inventory for the Rust engine test suite.

Generates from the real card database (cards/abilities.json) + test suite
(engine/tests/**/*.rs):

  * engine/tests/TEST_COVERAGE.md   -- coverage tables + gap lists + jidou
                                       (自動) interaction & specific-requirement reports
  * docs/ABILITY_MATRIX.md          — trigger×action matrix + condition/set breakdown
  * engine/tests/TEST_INVENTORY.json — machine-readable per-ability rows
  * engine/tests/TEST_INVENTORY.md  — human-readable per-ability index
  * engine/tests/TEST_QUALITY.md    — fn-level test-smell audit (vacuous /
                                       synthetic-only / pendency-only /
                                       confusable-card review prompts)

Depth inference (automated, zero hand-maintenance):

  L0 referenced  — card_no/base substring appears in any test .rs
  L1 fires       — L0 + file contains an assertion (assert! / assert_eq! etc.)
  L2 negative    — heuristic: file or test name hints at negative/skip/block path
                  (e.g. _negative, cannot_, no_, not_, skip, blocked, immune)
  L3/L4 edge/choice — flags: file mentions has_pending_choice / pending_choice_type
                       / select_indices / drain_auto

Manual override: add  /// @covers PL!N-bp7-021-N depth=L2  above a #[test] and the
parser will honour it for that ability (optional, not required).

Run:
    python cards/test_inventory.py          # regenerate all
    python cards/test_inventory.py --check  # CI: fail if stale (human docs only)
    python cards/test_inventory.py --check --check-json  # also verify the 1.7 MB JSON
"""
import argparse
import hashlib
import json
import re
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CARDS_JSON = ROOT / "cards" / "abilities.json"
QA_JSON = ROOT / "cards" / "qa_data.json"
TESTS_DIR = ROOT / "engine" / "tests"
OUT_COVERAGE = ROOT / "engine" / "tests" / "TEST_COVERAGE.md"
OUT_MATRIX = ROOT / "docs" / "ABILITY_MATRIX.md"
OUT_JSON = ROOT / "engine" / "tests" / "TEST_INVENTORY.json"
OUT_MD = ROOT / "engine" / "tests" / "TEST_INVENTORY.md"
# (OUT_QUALITY is defined in the test-quality section below.)

RARITY_ACTION_LABEL = {
    "move_cards": "move card between zones",
    "draw_card": "draw",
    "gain_resource": "gain heart/blade/score resource",
    "modify_score": "modify live score",
    "change_state": "change active/wait state",
    "position_change": "position change / area move",
    "add_to_energy": "add/place energy",
    "shuffle": "shuffle deck",
    "sequential": "sequential compound effect",
    "reveal": "reveal cards",
    "look_and_select": "look at / select from deck",
    "baton_touch": "baton touch (play onto member)",
    "compare_score": "score comparison",
    "convert": "convert resource",
    "refresh": "refresh / re-deck",
}

TRIGGER_ORDER = ["登場", "自動", "起動", "常時", "ライブ開始時", "ライブ成功時"]
TRIGGER_LABEL = {
    "登場": "登場 (Debut)",
    "自動": "自動 (Auto)",
    "起動": "起動 (Activation)",
    "常時": "常時 (Constant)",
    "ライブ開始時": "ライブ開始時 (LiveStart)",
    "ライブ成功時": "ライブ成功時 (LiveSuccess)",
}

# heuristic negative hints — match against file/test names only (not full text)
# (^|_)no_ covers the widespread `*_no_trigger` / `*_no_effect` / `*_no_blade`
# naming (e.g. ren_016_no_energy_placed_no_blade); the ^ anchor covers
# names starting with `no_` like no_live_success_no_trigger.
NEGATIVE_RE = re.compile(r"(negative|cannot|not_|_not|(^|_)no_|cannot_activate|already_waited|zero_tested|immune|blocked|empty|zero|skip_optional)", re.IGNORECASE)
# choice/edge signals
CHOICE_RE = re.compile(r"(has_pending_choice|pending_choice_type|select_indices|drain_auto|SelectCard|SelectTarget)")

# Condition types encoding specific, rule-heavy requirements. Abilities with one
# of these (or a use_limit) need explicit positive AND negative choice-level
# tests, so they are surfaced separately from plain L0/L1 coverage.
# Actions that can raise a SelectCard / SelectTarget prompt. An ability built
# only from the rest resolves on its own and can never reach "L2+choice", so
# calling it thin for lacking choice depth is a false positive that keeps
# pointing review at already-complete tests. Wrappers (sequential /
# conditional_on_result) recurse into their compound actions.
CHOICE_CAPABLE_ACTIONS = {
    "move_cards",            # pick cards out of a zone
    "select_cards",
    "select",
    "select_number",
    "choice",
    "look_and_select",
    "position_change",
    "activate_ability",      # pick whose ability to fire
    "conditional_on_optional",
    "conditional_alternative",
    "choose_target_player",
    "play_baton_touch",      # pick the arrival target
    "place_energy_under_member",
    "invalidate_ability",
    "repeat_procedure",
}

CHOICE_CAPABLE_WRAPPERS = {"sequential", "conditional_on_result", "repeat_procedure"}

# Zones whose contents the player cannot see, so "put 1 card from here" is the
# top card, not a choice. PL!SP-bp4-010-R's エネルギーカードを1枚…置く out of the
# energy deck resolves without a prompt for exactly this reason, and treating
# it as choice-capable kept a complete four-test file on the thin list.
HIDDEN_ZONES = {
    "energy_deck",
    "main_deck",
    "deck_top",
    "deck_bottom",
    "opponent_hand",
    "enemy_hand",
    "opponent_main_deck",
    "opponent_deck",
}


def _action_can_prompt(action, effect):
    if action == "move_cards":
        return effect.get("source") not in HIDDEN_ZONES
    return action in CHOICE_CAPABLE_ACTIONS


def effect_can_prompt(effect):
    """True if resolving `effect` can raise a choice for the player.

    Used to separate "this ability is under-tested" from "this ability has no
    prompt to exercise", so the thin-coverage list only names real gaps.
    """
    if not isinstance(effect, dict):
        return False
    action = effect.get("action")
    if _action_can_prompt(action, effect):
        return True
    if action in CHOICE_CAPABLE_WRAPPERS:
        return any(
            effect_can_prompt(sub) for sub in (effect.get("compound") or {}).get(
                "actions", []
            )
        )
    return False


SPECIAL_CONDITION_TYPES = {
    "ability_filter_condition",        # watches other ABILITIES resolving
    "all_cost_comparison_condition",
    "card_blade_condition",
    "energy_state_condition",
    "highest_cost_on_stage_condition",
    "position_condition",
    "state_change_condition",
    "state_condition",
}

# Full-text markers for 自動 (jidou) interaction categories.
WATCHES_ABILITIES_MARKER = "能力が解決"      # fires when another ability resolves
EFFECT_CAUSE_MARKERS = ("効果によって", "効果でも発動する")  # effect-caused / also fires off opponent effects

# ---------------------------------------------------------------------------
# LIFECYCLE CONSTRAINT (the dimension `depth` cannot see)
# ---------------------------------------------------------------------------
# The depth ladder (L0 referenced / L1 asserts / L2 negative) answers "does a
# test constrain WHAT this ability does". It cannot answer "does a test
# constrain HOW LONG the grant survives", and that is a separate failure mode
# with a separate cause.
#
# These four actions do not resolve into the game state and stop. They MUTATE
# persistent ability state that outlives the resolution and is RE-DERIVED by a
# later scan:
#
#   gain_ability             「…を持つ」
#   gain_ability_from_source 「…の下にあるカードから能力を得る」  (a 常時 that
#                            re-copies on EVERY recalculate_constants)
#   invalidate_ability       「…の能力を、ライブ終了時まで、無効に」
#   suppress_ability_trigger 「…の…能力の発動を、ライブ終了時まで、無効に」
#
# A single resolution test is blind to three ways such a grant goes wrong, and
# all three shipped as real defects while the depth check read "fully covered":
#
#   IDEMPOTENCE  the re-derivation APPENDS instead of replacing, so N scans
#                produce N copies. Each copy takes a distinct
#                `GAINED_ABILITY_INDEX_BASE + gained_idx`, so
#                `trigger_live_success_abilities`' `(card_id, ability_index)`
#                dedup cannot collapse them and the trigger fires N times.
#   REVOCATION   the grant is never withdrawn when its precondition is
#                destroyed, leaving the host able to activate an ability whose
#                printed source is gone.
#   SURFACE      the grant never reaches the generated action list, so it is
#                executable but un-offerable — dead to a player and to the bot.
#
# SURFACE only applies to the `gain_ability*` pair: a granted 起動 becomes an
# offerable action, whereas an invalidation/suppression is a flag consumed by a
# trigger dispatch and has no action-list representation to check.
STATEFUL_REGISTRATION_ACTIONS = {
    "gain_ability",
    "gain_ability_from_source",
    "invalidate_ability",
    "suppress_ability_trigger",
}


def _grants_activation(node):
    """True if this effect (at any nesting depth) grants a 起動.

    SURFACE is only meaningful for a grant that produces something the player is
    OFFERED as an activation. That is decided by the granted ability's trigger,
    not by the granting action: a grant of 常時 or ライブ成功時 changes scoring or
    a trigger dispatch and has no action-list representation, so demanding a
    `generate_possible_actions` read for it would be demanding an assertion about
    something that does not exist.

    The deck settles this cleanly. `gain_ability` carries a machine-readable
    `ability_gain_trigger`, and every instance in `abilities.json` names 常時 — so
    the whole `gain_ability` family is surface-exempt. `gain_ability_from_source`
    carries a `trigger_filter`, and only a 起動 filter (or an absent one) can
    yield an activatable copy. Net effect: exactly one ability in the deck is
    surface-constrained, and it is the one whose grant was executable but never
    offered.
    """
    if isinstance(node, dict):
        if node.get("ability_gain_trigger") == "起動":
            return True
        for f in node.get("trigger_filter") or []:
            if isinstance(f, str) and "起動" in f:
                return True
        # An absent filter on a from-source grant copies whatever the source
        # prints, 起動 included.
        if (
            node.get("action") == "gain_ability_from_source"
            and not node.get("trigger_filter")
        ):
            return True
        return any(_grants_activation(v) for v in node.values())
    if isinstance(node, list):
        return any(_grants_activation(v) for v in node)
    return False

# A driver that RE-DERIVES persistent ability state. Calling it once proves the
# grant was made; calling it again is what proves it was not duplicated. This is
# the CONSTANT-SCAN shape of idempotence.
LIFECYCLE_DRIVER_RE = re.compile(
    r"\brecalculate_constants\b"
    r"|\btrigger_live_success_abilities\b"
    r"|\btrigger_live_start_abilities\b"
    r"|\btrigger_auto_abilities\b"
    r"|\bexecute_live_victory_determination\b"
)
# The RESOLVE-TWICE shape of idempotence: an ability that grants a 常時 is reached
# by resolving its printed ability again, and "a second resolution does not grow
# the total" is just as much an idempotence proof as re-running a scan. A 起動
# with ターン1回 is refused the second time, so the assertion that the total held
# is a real one.
LIFECYCLE_RESOLVE_RE = re.compile(
    r"\bactivate_ability\b"
    r"|\btry_activate_ability\b"
    r"|\bfire_trigger\b"
    r"|\btrigger_printed_ability_and_resolve_choices\b"
    r"|\bplay_to_stage\b"
)
# REVOCATION is reachable three ways, and this family uses all three:
#   (a) the host (or a qualifying source) changes zone — the engine's choke point;
#   (b) an explicit runtime clear;
#   (c) the printed DURATION expiring through a real phase walk, which is how
#       ライブ終了时报 grants end and is by far the most common path here.
LIFECYCLE_ZONEXIT_RE = re.compile(
    r"\bon_cards_left_zones\b"
    r"|\bclear_gained_abilities\b"
    r"|\bclear_all_for_card\b"
    r"|\bexecute_main_phase_action\b"
)
# Crossing a phase boundary at all is what lets a duration expire; a `pass()`
# inside a loop, or any explicit target phase, is the token for it.
LIFECYCLE_ROLLOVER_RE = re.compile(
    r"\bcurrent_turn_phase\b"
    r"|\bTurnPhase::Live\b"
    r"|\bPhase::Active\b"
    r"|\bLiveVictoryDetermination\b"
    r"|\bcheck_expired_effects\b"
    r"|\badvance_to_phase\b"
    r"|\.pass\(\)"
)
# Asserting the registration is GONE is the direct statement of revocation, and it
# catches the case the phase walk misses: a card swapped off the stage by another
# card's effect, with no rollover involved.
LIFECYCLE_ABSENCE_RE = re.compile(
    r"!\s*\w*\.?state\.gained_abilities\.contains_key"
    r"|gained_abilities[^;\n]*\.\s*is_none"
    r"|gained_abilities[^;\n]*\.\s*is_empty"
    r"|!gained_abilities\.contains_key"
)
# Reading what a player would actually be offered, rather than an internal table.
LIFECYCLE_SURFACE_RE = re.compile(
    r"\bgenerate_possible_actions\b"
    r"|\.generated_actions\(\)"
)
# Loop header — a driver reached through a loop is a repeat even though it
# appears lexically once.
LIFECYCLE_LOOP_RE = re.compile(r"^\s*(?:for\b|while\b|loop\s*\{)", re.MULTILINE)

FN_TEST_RE = re.compile(r"^\s*#\[test\]\s*\n\s*(?:pub\s+)?fn\s+(\w+)", re.MULTILINE)
COVERS_RE = re.compile(r"@covers\s+([A-Z0-9!+\-]+\S*)", re.IGNORECASE)

# ---------------------------------------------------------------------------
# Ability families (docs/ABILITY_FAMILIES.md).
#
# A *family* groups all unique abilities whose printed Japanese text states
# the same behavioral contract, regardless of card. Families are keyed on the
# parser's structured fields (already emitted in abilities.json): trigger x
# effect action x discriminating detail (condition type, target, source/
# destination, cost shape). This is a *categorization* layer over the parsed
# data, not a coverage claim: candidate test references come from the same
# L0 substring pass the coverage report uses, and are labelled candidates.
# ---------------------------------------------------------------------------

# Discriminating detail extracted per action. Each extractor returns the
# family-variant key or None when the ability doesn't belong to that family.
def _move_variant(eff):
    src, dst = eff.get("source"), eff.get("destination")
    if not src or not dst:
        return None
    return f"{src}->{dst}"


def _gain_variant(eff):
    res = eff.get("resource")
    if res == "heart":
        colors = eff.get("heart_colors") or []
        return f"heart:{'+'.join(colors) if colors else '?'}"
    return f"{res}:n{eff.get('count', '?')}"


def _modify_score_variant(eff):
    op = eff.get("operation")
    val = eff.get("value", eff.get("amount"))
    return f"{op or '?'}{val if val is not None else ''}"


def _change_state_variant(eff):
    return f"{eff.get('state_change') or eff.get('state') or '?'}"


def _look_variant(eff):
    n = eff.get("count")
    take = eff.get("select_count", "?")
    return f"look{n or '?'}:take{take if take is not None else '?'}"


def _draw_variant(eff):
    return f"n{eff.get('count', '?')}"


def _condition_variant(eff):
    c = eff.get("condition") or {}
    return c.get("type") or "none"


FAMILY_RULES = [
    # (family name, trigger set filter or None, variant extractor)
    ("activation_cost_self_to_discard_recover", {"起動"}, lambda eff, cost:
        (eff.get("action") == "move_cards" and eff.get("destination") == "hand"
         and (cost or {}).get("self_cost")) or None),
    ("debut_look_and_select", {"登場"}, lambda eff, cost:
        (eff.get("action") == "look_and_select") or None),
    ("debut_move_cards", {"登場"}, lambda eff, cost:
        (eff.get("action") == "move_cards") or None),
    ("debut_draw", {"登場"}, lambda eff, cost:
        (eff.get("action") == "draw_card") or None),
    ("debut_change_state", {"登場"}, lambda eff, cost:
        (eff.get("action") == "change_state") or None),
    ("constant_conditional_gain", {"常時"}, lambda eff, cost:
        (eff.get("action") == "gain_resource" and eff.get("condition")) or None),
    ("constant_modify_cost", {"常時"}, lambda eff, cost:
        (eff.get("action") == "modify_cost") or None),
    ("constant_modify_score", {"常時"}, lambda eff, cost:
        (eff.get("action") == "modify_score") or None),
    ("constant_restriction", {"常時"}, lambda eff, cost:
        (eff.get("action") == "restriction") or None),
    ("live_start_gain", {"ライブ開始時"}, lambda eff, cost:
        (eff.get("action") == "gain_resource") or None),
    ("live_success_move_cards", {"ライブ成功時"}, lambda eff, cost:
        (eff.get("action") == "move_cards") or None),
    ("live_success_score", {"ライブ成功時"}, lambda eff, cost:
        (eff.get("action") == "modify_score") or None),
    ("activation_move_cards", {"起動"}, lambda eff, cost:
        (eff.get("action") == "move_cards") or None),
    ("auto_gain", {"自動"}, lambda eff, cost:
        (eff.get("action") == "gain_resource") or None),
    ("auto_move_cards", {"自動"}, lambda eff, cost:
        (eff.get("action") == "move_cards") or None),
    ("auto_change_state", {"自動"}, lambda eff, cost:
        (eff.get("action") == "change_state") or None),
]

# Variant extractors applied inside a family, keyed by family name.
FAMILY_VARIANT_EXTRACTORS = {
    "activation_cost_self_to_discard_recover": _move_variant,
    "debut_move_cards": _move_variant,
    "live_success_move_cards": _move_variant,
    "auto_move_cards": _move_variant,
    "constant_conditional_gain": _condition_variant,
    "debut_look_and_select": _look_variant,
    "debut_draw": _draw_variant,
}

# ---------------------------------------------------------------------------
# P0.3 ratchet (docs/TEST_HARDENING_PLAN_2026-08-26.md): the soft-guard
# pattern `if x.has_pending_choice() { ... }` silently absorbs a missing
# prompt, so a broken ability can pass its test. WRITING_TESTS.md mandates
# strict draining (drain_choices_strict / dispatch-on-choice). Existing sites
# are grandfathered in SOFT_GUARD_BASELINE.json; the count may only go DOWN.
# ---------------------------------------------------------------------------
SOFT_GUARD_RE = re.compile(
    r"\bif\s+[A-Za-z_][\w\.]*\.has_pending_choice\(\)\s*\{"
)
SOFT_GUARD_BASELINE = ROOT / "engine" / "tests" / "SOFT_GUARD_BASELINE.json"


def count_soft_guards():
    """Per-file counts of soft-guard `if` sites across the test corpus."""
    counts = {}
    for path in sorted((ROOT / "engine" / "tests").rglob("*.rs")):
        text = path.read_text(encoding="utf-8", errors="replace")
        n = len(SOFT_GUARD_RE.findall(text))
        if n:
            counts[str(path.relative_to(ROOT)).replace("\\", "/")] = n
    return counts


def load_soft_guard_baseline():
    if not SOFT_GUARD_BASELINE.exists():
        return {}
    return json.loads(SOFT_GUARD_BASELINE.read_text(encoding="utf-8"))


def check_soft_guard_ratchet():
    """Returns a list of human-readable violations (empty == OK)."""
    current = count_soft_guards()
    baseline = load_soft_guard_baseline()
    violations = []
    for path, n in sorted(current.items()):
        allowed = baseline.get(path, 0)
        if n > allowed:
            violations.append(
                f"{path}: {n} soft guards (baseline allows {allowed}). "
                "Use drain_choices_strict / dispatch-on-choice instead "
                "(WRITING_TESTS.md §H); update "
                "engine/tests/SOFT_GUARD_BASELINE.json ONLY when a guarded "
                "site is genuinely legitimate."
            )
    for path in sorted(baseline):
        if path not in current and baseline[path] > 0:
            print(
                f"SOFT GUARD ratchet: {path} dropped to 0 — remove it from "
                "the baseline (python cards/test_inventory.py --update-softguard-baseline)"
            )
    return violations


def write_soft_guard_baseline():
    counts = count_soft_guards()
    SOFT_GUARD_BASELINE.write_text(
        json.dumps(counts, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    total = sum(counts.values())
    print(
        f"Wrote {SOFT_GUARD_BASELINE.relative_to(ROOT)} "
        f"({len(counts)} files, {total} soft-guard sites baselined)"
    )


def load_abilities():
    with open(CARDS_JSON, encoding="utf-8") as f:
        data = json.load(f)
        return data["unique_abilities"], data.get("statistics", {})


def card_numbers_with_optional_select():
    """Card numbers whose `select_cards` step is marked `optional`.

    Those are the cards where an index-0 answer can be a DECLINE rather than a
    choice, because the prompt's generated option list leads with the skip entry.
    """
    out = set()
    for u in load_abilities()[0]:
        sel = (u.get("effect") or {}).get("select_action") or {}
        if sel.get("action") != "select_cards" or sel.get("optional") is not True:
            continue
        for entry in u.get("cards", []):
            out.add(entry.split(" | ")[0])
    return out


def unplayable_live_cards():
    """Card numbers of live cards no stage in the pool can satisfy.

    A need_heart requirement above what a stage can supply is unreachable. Three
    members fit, so the per-colour bound is the per-colour maximum over the pool
    (a stage cannot exceed the best single member for one colour), and a
    COLORLESS `heart0` is bounded by the total of the three highest-heart members,
    because the wildcard draws on every heart on the stage.

    Sound in the strong direction: every card returned is genuinely unplayable, so
    the set is a lower bound on the true one.

    Bounding `heart0` by the best SINGLE member instead of the best three is wrong
    and was wrong here first: a live needing `heart0: 10` was reported unplayable
    while a stage of three members printing 27 hearts satisfies it. The colour bound
    really is per-member -- three members cannot contribute 3x the best to one
    colour if the requirement is 2x the best -- so that part stays.
    """
    with open(ROOT / "cards" / "cards.json", encoding="utf-8") as f:
        cards = json.load(f)
    best: dict[str, int] = {}
    totals: list[int] = []
    for c in cards.values():
        if c.get("type") != "メンバー":
            continue
        h = c.get("base_heart") or {}
        if not h:
            continue
        totals.append(sum(h.values()))
        for k, v in h.items():
            best[k] = max(best.get(k, 0), v)
    totals.sort(reverse=True)
    # Three members fit on a stage, and the wildcard draws on all of them.
    stage_hearts = sum(totals[:3])
    bounds = dict(best)
    bounds["heart0"] = stage_hearts

    out = {}
    for c in cards.values():
        if c.get("type") != "ライブ":
            continue
        need = c.get("need_heart") or {}
        if not need:
            continue
        short = [
            f"{colour} needs {n}, best single member prints {bounds.get(colour, 0)}"
            for colour, n in need.items()
            if colour != "heart0" and bounds.get(colour, 0) < n
        ]
        w = need.get("heart0", 0)
        if w and stage_hearts < w:
            short.append(
                f"heart0 needs {w}, a stage's three best members print {stage_hearts}"
            )
        if short:
            out[c["card_no"]] = short
    return out


def card_base(card_no):
    m = re.match(r"^(.*?-\d+)", card_no)
    return m.group(1) if m else card_no


def card_set(card_no):
    m = re.search(r"-(bp\d+|sd\d+|pb\d+|cl\d+|PR)-\d+", card_no)
    if m:
        return m.group(1)
    return "other"


def effect_action(eff):
    a = (eff or {}).get("action")
    if isinstance(a, str):
        return a
    if isinstance(a, dict):
        return a.get("type") or "compound"
    return "unknown"


def condition_type(eff):
    c = (eff or {}).get("condition") or {}
    t = c.get("type")
    if not t:
        te = c.get("trigger_event") or {}
        if te.get("type"):
            return f"trigger:{te['type']}"
        return "none"
    if t == "movement_condition":
        te = c.get("trigger_event") or {}
        return f"movement:{te.get('type','?')}"
    return t


def human_action(a):
    return RARITY_ACTION_LABEL.get(a, a)


def collect_test_files():
    files = []
    for p in TESTS_DIR.rglob("*.rs"):
        try:
            text = p.read_text(encoding="utf-8")
        except Exception:
            continue
        rel = p.relative_to(ROOT).as_posix()
        # extract fn names
        fns = FN_TEST_RE.findall(text)
        # if file uses #[test] inline without newline, fallback
        if not fns:
            fns = re.findall(r"fn\s+(test_\w+)", text)
        files.append((p, rel, text, fns))
    return files


def infer_depth_for_file(text, rel):
    has_assert = "assert" in text
    has_choice = bool(CHOICE_RE.search(text))
    has_negative = bool(NEGATIVE_RE.search(rel))
    return has_assert, has_choice, has_negative


# ---------------------------------------------------------------------------
# Test-quality smells (fn-level static audit). Catches the bug classes that
# coverage counts miss:
#   no_assert      — test never asserts (smoke at best).
#   no_drive       — game state mutated but the engine never driven
#                    (no scan/activate/play/fire): vacuous negatives/positives.
#   synthetic_only — trigger event hand-pushed (push_movement_event et al)
#                    with no real ability resolution driving it in the same fn.
#   pendency_only  — has_pending_choice asserted without choice identity
#                    (pending_choice_type/summary/answer) or outcome asserts.
#   similar_cards  — confusable card numbers (bp2 vs pb2) staged in one file.
# Report-only: engine/tests/TEST_QUALITY.md, freshness-checked like the rest.
# ---------------------------------------------------------------------------
OUT_QUALITY = ROOT / "engine" / "tests" / "TEST_QUALITY.md"

Q_ASSERT_RE = re.compile(r"\bassert(_eq|_ne|_ability)?!\s*\(|\bpanic!\s*\(")
Q_SCAN_RE = re.compile(
    r"scan_autos_both|trigger_auto_abilities|process_pending|process_with_completed"
    r"|activate_ability|try_activate_ability|play_to_stage|try_play_to_stage|fire_trigger"
    r"|resume_with_choice|execute_main_phase_action"
    r"|pass_phase|\.pass\(\)|perform_live|ability_verdicts|drain_auto_ability_choices"
    r"|process_current_ability|set_live_card|set_energy_card|recalculate_constants"
    r"|advance_phase|ConditionContext|evaluate_condition|TurnEngine::"
    # `advance_to_phase` steps the REAL turn through its phases, so it drives
    # the engine exactly as `pass_phase` does. Without it a test that replaced
    # its blind `for _ in 0..N { pass() }` walk with a named target (the
    # blind_phase_stepping fix) was misfiled as `no_drive` — a false positive
    # created by doing the thing this audit asks for.
    r"|advance_to_phase|pass_into_phase_capturing_energy"
)
Q_SETUP_RE = re.compile(
    r"stage\.stage|energy_zone|energy_deck|main_deck|hand\.cards|waitroom\.cards|live_card_zone"
)
Q_SYNTH_RE = re.compile(
    r"push_movement_event|position_change_events\.push|record_card_movement|set_recently_moved"
)
Q_REAL_DRIVER_RE = re.compile(
    r"activate_ability|play_to_stage|try_play_to_stage|fire_trigger"
)
Q_PEND_RE = re.compile(r"has_pending_choice")
Q_IDENT_RE = re.compile(
    r"pending_choice_type|pending_choice_summary|select_choice_option|answer_choice|drain_choices_strict"
)
# A card-identity pin: the test compares the resolved instance's printed
# card_no, so a transposed bp2/pb2 (or SEC/P) card number would fail loudly.
Q_CARD_PIN_RE = re.compile(r"assert_card_identity|card_no")

# An ABSENCE claim: an assertion that something did NOT happen, as opposed to a
# presence check or a value comparison. Structural counterpart to NEGATIVE_RE, which
# only matches a NAME hint.
#
# The forms are deliberately narrow. `== 0` and `>= 1` are NOT absence claims: 0 is
# both "nothing" and a real value in this suite, and `>= 1` is a presence check.
# Crediting either would move abilities UP the ladder on evidence that does not
# support it, which is the wrong direction for a gap-finder.
#
# KNOWN LIMIT, and it is why a card can sit on the L1+choice frontier while every
# test for it is green and meaningful. This keys on absence SHAPES, so it cannot see
# a negative written as a positive equality — "the second trigger did not raise the
# value" is `assert_eq!(x, 1)`, and 1 is not a form here.
#
# 澁谷かのん PL!SP-sd2-012-SD2 and 鬼塚冬毬 PL!SP-sd2-022-SD2 are the worked example.
# `cross_player_jidou_triggers_test.rs` covers both for the TRIGGER firing — a
# self-caused move grants the heart, an opponent-caused move grants it too, which is
# the 「でも発動する」 exception — and every assertion in it is `assert_eq!(x, 1)`. So
# the ladder reported "no negative" and both rows sat on the frontier while nothing was
# missing except the two lifecycle clauses, now tested in
# `jidou/movement/self_area_move_watch/area_move_grant_turn_limit_and_live_end_expiry_test.rs`.
#
# Deliberately not worked around: deciding that `assert_eq!(x, 1)` is a negative needs
# to know what could have raised 1, which is not statically decidable, and a metric that
# guesses moves abilities UP on invented evidence.
Q_ABSENCE_ASSERT_RE = re.compile(
    r"assert!\s*\(\s*!"
    r"|assert_eq!\s*\([^,]+,\s*(?:0|false)\s*\)"
    r"|assert_eq!\s*\([^,]*\.len\(\)[^,]*,\s*0\s*\)"
    r"|is_empty\(\)"
    r"|\.is_none\(\)"
    r"|\.is_err\(\)"
    r"|assert_ne!\s*\(\s*[^,]+,\s*None"
)

# A file that drives ライブ成功時 through the GATED path, without establishing that
# its live actually SUCCEEDED.
#
# `trigger_live_success_abilities` checks `should_trigger_live_success`, so a live
# that failed silently dispatches nothing — and a NEGATIVE assertion in such a file
# then passes whether or not the ability does anything. `fire_trigger` is
# deliberately excluded: it calls `trigger_auto_ability` directly and FORCES the
# dispatch, so a file built on it has no premise to state.
#
# Scoped to a CALL, in code rather than prose. Four earlier scopings were wrong, each
# because the pattern matched something other than a dispatch; the counts are carried
# here so they are not re-derived:
#   * matching ライブ成功時 anywhere reported 91 — it hits comments and strings;
#   * matching the bare function name reported 70, which is really "files that
#     mention it";
#   * matching the name without a call paren still counted a file whose only hit was
#     a comment explaining the smell itself;
#   * the correct rule is comment-stripped + call-paren, and it reports 10.
#
# A triage of those 10 found the lives they drive are satisfiable, so the remedy is
# to add the premise. An earlier triage concluded the opposite, twice: it compared a
# live's `heart0` requirement against a member's `heart0` bucket, but `heart0` is a
# COLORLESS WILDCARD (core/card.rs `check_heart_requirement` skips Heart00 in the
# per-colour loop and then requires the leftover sum of every other colour to cover
# it), and no member prints a literal colorless heart. Both times the engine was
# right and the script was wrong.
Q_LIVE_SUCCESS_CALL_RE = re.compile(r"trigger_live_success_abilities\s*\(")
Q_LIVE_SUCCESS_PREMISE_RE = re.compile(
    r"should_trigger_live_success|performance_snapshots|execute_live_victory_determination"
)
Q_LINE_COMMENT_RE = re.compile(r"//.*$", re.MULTILINE)

# A live card whose printed need_heart cannot be met by ANY stage, so its
# ライブ成功時 is unreachable and every test that drives it is vacuous by
# construction -- not merely missing a premise.
#
# Sound in the strong direction: a per-colour requirement above the per-colour
# MAXIMUM over the whole pool cannot be met, because three members cannot
# contribute 3x the best to a single colour. Only three members fit, so a
# COLORLESS `heart0` is bounded by the total of the three highest-heart members --
# bounding it by the best SINGLE member was wrong and cost six false positives
# (a live needing heart0: 10 was called unplayable while three members printing
# 27 hearts satisfy it).
#
# Necessary, not sufficient: three slots can still conflict across colours, so
# this is a LOWER BOUND on the unplayable set. "Not reported" therefore means
# "not provably unplayable", not "demonstrably playable".
#
# Measured on this card pool: 46 of the 291 lives that carry a need_heart, 15.8%,
# are provably unplayable. A test gating on one of those can never be non-vacuous
# without injecting synthetic hearts, which is legitimate -- the files in
# live_success_no_premise all use a heart00 wildcard, and the engine treats a
# heart00 in the PROVIDED hearts as an unbounded wildcard -- but it must be
# deliberate.
Q_IMPOSSIBLE_NEED_RE = re.compile(
    r"PL![A-Za-z0-9!\-_+＋]+-L(?:\s|\)|,)"
)

# A test that drains prompts with index [0] AND asserts an ABSENCE, while naming a
# card whose `select_cards` is optional.
#
# On a skippable prompt, `[0]` is NOT reliably "take" and NOT reliably "skip" — it
# depends on the PROMPT KIND, and the harness does not say which you have. Two
# prompts in this suite disagree:
#
#   * an optional-COST prompt (`pay_optional_cost:skip_optional_cost`) — `[0]`
#     DISCARDS the cost card, `[]` skips it
#       (effects/look_select/look_and_filter/look_at_deck_top_optional_discard_test.rs,
#        whose two tests differ only in `select_indices(&[0])` vs `(&[])`);
#   * a `looked_at` SELECTION with a group filter — measured, `[0]` TAKES the
#     matching card and `[]` declines, so that order is [card, ...].
#
# So the hazard is AMBIGUITY, not a known-wrong order: a test that intends to take
# and instead declines passes silently whenever its assertion is an absence, which a
# positive assertion would have caught.
#
# EXPECTED YIELD IS LOW, and that is measured rather than hoped. Thirty of these files
# assert absence directly about a look or selection, and the two read to check --
# `look_and_filter/per_group_take_one_from_look_five_test.rs` and
# `reveal/debut_group_look_three_reveal_test.rs` -- are both exemplars: each states
# the prompt with `assert_select_card("looked_at", 1, true)` before answering, takes
# with `[0]` and declines with `[]` deliberately, and pairs every absence assertion
# with a positive twin that a silent decline would fail. Read the row as "spend five
# minutes here", not as 49 problems.
Q_DRAIN_ZERO_RE = re.compile(r"select_indices\(\s*&\[\s*0\s*\]\s*\)")

# assert!(x) / assert_eq!(x, y) — the units an assertion of interest is counted in.
Q_ASSERT_CALL_RE = re.compile(r"assert(?:_eq|_ne|_ability)?!\s*\(")
# A negative-only assertion: is_err(), is_none(), !ok, result.is_err(), etc.
Q_NEGATIVE_ASSERT_RE = re.compile(
    r"is_err\(\)|is_none\(\)|!ok\b|\bok\(\)\s*==\s*false|assert_ne!\s*\(\s*None"
)
# A count-only assertion: comparing .len(), a count, or ">= 1" and nothing else.
Q_COUNT_ASSERT_RE = re.compile(
    r"\.len\(\)\s*[=!<>]=?|\bcount\b\s*[=!<>]=?|>\s*=\s*1\b|>\s*=\s*0\b"
)
Q_PLACEHOLDER_RE = re.compile(
    r"#\[ignore|assert!\s*\(\s*true\s*\)|assert!\s*\(\s*1\s*==\s*1\s*\)|todo!\s*\(|"
    r"unimplemented!\s*\(|panic!\s*\(\s*\"TODO"
)
# An assertion that cannot fail. `assert!(true)` used to satisfy Q_ASSERT_RE and
# hide behind no_assert=0 — the suite carried nine of them, in files whose own
# comments admitted they existed only to raise the L0 coverage number.
Q_TRIVIAL_ASSERT_RE = re.compile(
    r"^\s*(?:true|false|1\s*==\s*1|0\s*==\s*0)\s*[,)]?"
)
# Prompts answered by ORDINAL position rather than by the prompt's own identity.
# The signal is a loop counter compared to a literal, so both halves are
# required: the name must be incremented somewhere in the same body AND
# compared against a number. Matching the comparison alone fired on ordinary
# `*count > 1` filters, which are the good kind.
Q_LOOP_COUNTER_RE = re.compile(r"\b(\w+)\s*\+=\s*\d+\b")

# A stage/zone literal being built by assignment. `stage.stage = [a, b, -1]`,
# `stage.stage.assign([a, b, -1])`, and `under_cards[1] = [a, a]` all count.
Q_STAGE_LITERAL_RE = re.compile(
    r"(?:stage\.stage|under_cards\s*\[[^\]]*\])\s*(?:\.assign\()?\s*=\s*\[([^\]]*)\]"
)
Q_SLOT_IDENT_RE = re.compile(r"\b([A-Za-z_][A-Za-z0-9_]*)\b")
# `const LIKE_A_TREASURE: &str = "PL!N-bp7-031-L";` — a file-level name bound
# to a card number. Test bodies use the name, not the literal, so direct
# attribution has to follow the binding.
Q_CONST_CARD_BIND_RE = re.compile(
    r"\bconst\s+([A-Z][A-Z0-9_]*)\s*:\s*&str\s*=\s*\"([^\"]+)\""
)
# Rust keywords and the obvious non-card tokens, so `= [a, b, -1]` does not read
# `a` twice because of an intervening operator or a cast.
_NOT_A_CARD_SLOT = {
    "-1",
    "vec",
    "Vec",
    "stage",
    "stage_card_ids",
    "stage_card",
    "stage_cards",
    "self",
    "game",
    "gs",
    "state",
    "as",
    "i16",
    "u8",
    "usize",
    "true",
    "false",
    "None",
    "Some",
}


Q_COUNT_OP = r"\.(?:len|count|active_count)\s*\(\s*\)"
# A count expression compared with < or > (not <=/>=, which pin a bound) against
# another expression. `x.len() > 3` and `y.len() < before` both qualify.
Q_COUNT_INEQ_RE = re.compile(
    r"(?:%s\s*(?:<=|>=|<|>)|(?:<=|>=|<|>)\s*[\w.]*%s)" % (Q_COUNT_OP, Q_COUNT_OP)
)


def _assert_conditions(body):
    """Yield the condition text of every `assert!(...)`, paren-balanced.

    A regex cannot do this: `assert!(a.len() > f(b))` has a `)` inside the call,
    and stopping at the first one truncates the condition so badly that the count
    comparison goes missing. Nested asserts inside closures and helpers are
    included, which is what we want — the question is what the WHOLE test proves.
    """
    for m in re.finditer(r"assert!\s*\(", body):
        i = m.end() - 1
        depth = 0
        j = i
        while j < len(body):
            ch = body[j]
            if ch == "(":
                depth += 1
            elif ch == ")":
                depth -= 1
                if depth == 0:
                    break
            j += 1
        yield body[i + 1 : j]


def _count_inequality_only(body):
    """Describe a test that pins its outcome with loose count inequalities.

    The rule: no assert_eq/assert_ne anywhere (so the test states no value at
    all), and at least one `assert!` that compares a COUNT against another
    expression with `<` or `>`. Asserts that are not count comparisons
    (`has_pending_choice()`, a zone `contains`) are counted as supporting
    evidence and do not disqualify the test.

    `<=` and `>=` against a measured baseline are bands too, so they qualify;
    what the returned string separates is the weaker form — a LITERAL threshold,
    where "at least 5" is true for 5, 6 and 50 and there is no measurement at
    all behind it.
    """
    hits = []
    for cond in _assert_conditions(body):
        cond = cond.strip()
        # `assert!(is_ok() || x.len() <= 2)` is a disjunction, not a band on a
        # count: the MD forbids that shape outright, and it is not this section.
        if "==" in cond or "||" in cond:
            continue
        if not Q_COUNT_INEQ_RE.search(cond):
            continue
        hits.append("threshold" if re.search(r"[<>]=?\s*\d", cond) else "baseline")
    if not hits:
        return ""
    kinds = set(hits)
    kind = "threshold" if kinds == {"threshold"} else "baseline+threshold" if len(kinds) > 1 else "baseline"
    return "%d inequality assertion(s) on a count, vs %s" % (len(hits), kind)


# A card built inline inside the bracket — `game.id("PL!S-sd1-001-SD")` — is ONE
# slot. Tokenising the call would read `PL`, `S`, `sd1` and `SD` as four
# identifiers and report each twice, which is noise rather than a duplicate.
# Each inline card is collapsed to a placeholder that is unique per occurrence,
# so two different inline cards in one literal stay distinct.
#
# The argument may be a string literal OR a bare identifier, because a const-bound
# card number is the dominant idiom in this suite:
#
#     const FILLER: &str = "PL!-sd1-010-SD";
#     game.state.player1.stage.stage = [wakana, game.new_id(FILLER), game.new_id(FILLER)];
#
# Both of those `new_id` calls allocate DISTINCT instances, and reading the bare
# identifier `FILLER` twice in one bracket instead reported it as one card in two
# slots. That is the same const-binding blind spot this file had twice already, in
# `unresolvable_card_id` and in `similar_cards`: a pattern that only sees inline
# literals is blind to the way this suite actually names its cards.
#
# A genuinely repeated VARIABLE (`[filler, yoshiko, filler]`) is still reported,
# because a bare identifier with no call in front of it is not collapsed.
Q_INLINE_CARD_RE = re.compile(
    r"[A-Za-z_][A-Za-z0-9_]*\.(?:id|new_id)\s*\(\s*(?:\"[^\"]*\"|[A-Za-z_][A-Za-z0-9_]*)\s*\)"
)


def _collapse_inline_cards(fragment):
    seen = {}

    def sub(_m):
        key = f"__inline_card_{len(seen)}"
        seen[key] = True
        return key

    return Q_INLINE_CARD_RE.sub(sub, fragment)


def _duplicate_stage_ids(body):
    """Yield (identifier, slot_count) for a card staged in more than one slot.

    Only literal stage assignments are read, so a loop that fills the board with
    `game.new_id(...)` — which is the correct way to get three copies — is never
    reported. That keeps the false-positive rate at zero for well-formed
    fixtures; a row here means one id is literally repeated in the list.

    Known gap: brackets are read one at a time, so a bare id reused across BOTH
    players' stage assignments is not reported, even though one card in two
    players' stages is as illegal as one card in two slots of a single stage. That
    is a narrowing, not a false negative in what this does claim.
    """
    for m in Q_STAGE_LITERAL_RE.finditer(body):
        counts = {}
        fragment = _collapse_inline_cards(m.group(1))
        for name in Q_SLOT_IDENT_RE.findall(fragment):
            if name in _NOT_A_CARD_SLOT or name.isdigit():
                continue
            counts[name] = counts.get(name, 0) + 1
        for name, n in sorted(counts.items()):
            if n > 1:
                yield name, n


def _has_ordinal_drain(body):
    """Does a loop counter drive WHICH prompt gets answered?

    The comparison direction is what separates the two uses. `if step <= 2` and
    `if step == 1` pick an answer by position — that is the smell. `if iter > 20`
    is a runaway guard on a `while` and answers nothing by position, so it is
    not counted.
    """
    counters = {m.group(1) for m in Q_LOOP_COUNTER_RE.finditer(body)}
    counters &= {"step", "i", "idx", "n", "iter", "k", "round", "attempt", "pass_no"}
    for name in counters:
        if re.search(r"\b" + re.escape(name) + r"\s*(?:==|<=|<)\s*\d+\b", body):
            return True
    return False
# A fixed-length walk through the turn. A blind `for _ in 0..3 { pass() }` works
# right up until a phase gains or loses a step, and then the test is standing in
# a different window than it thinks — with no failure to notice it. A test that
# ASSERTS where it ended up is not in that position, so those are exempt.
Q_BLIND_PHASE_RE = re.compile(
    r"for\s+_\s+in\s+0\.\.[0-9]+\s*\{\s*(?:[A-Za-z_][\w.]*\.)?pass\(\)\s*;\s*\}"
)
Q_PHASE_PIN_RE = re.compile(
    r"advance_to_phase|pass_into_phase_capturing_energy|assert_eq!\s*\(\s*[^,]*"
    r"current_phase|assert!\s*\([^)]*current_phase"
)


def _assert_calls(body):
    """The source of each assertion in `body`, so a test can be classified by
    what it actually checks rather than by the words around it."""
    out = []
    for m in Q_ASSERT_CALL_RE.finditer(body):
        depth = 0
        start = m.end() - 1
        for i in range(start, len(body)):
            c = body[i]
            if c == "(":
                depth += 1
            elif c == ")":
                depth -= 1
                if depth == 0:
                    out.append(body[start + 1 : i])
                    break
        else:
            out.append(body[start:])
    return out


def _strip_rust_strings(body):
    """Blank out string literals so an assertion MESSAGE cannot classify it.

    `assert_eq!(x, 3, "own=0, opponent>=1 -> 3 blades")` is a specific-value
    assertion, but the `>=1` in the message made the count detector claim the
    test only checks counts. Messages describe the rule; only the compared
    operands say what the test pins.
    """
    out = []
    i = 0
    n = len(body)
    while i < n:
        c = body[i]
        if c == '"':
            # raw string: r"..." / r#"..."#
            raw = 0
            j = i
            while j > 0 and (body[j - 1].isalnum() or body[j - 1] == "_") and raw < 1:
                raw += 1
                j -= 1
            if raw:
                hashes = 0
                while i + 1 + hashes < n and body[i + 1 + hashes] == "#":
                    hashes += 1
                if i + 1 + hashes < n and body[i + 1 + hashes] == '"':
                    close = '"' + "#" * hashes
                    end = body.find(close, i + 2 + hashes)
                    end = n if end == -1 else end + len(close)
                    out.append(" " * (end - i))
                    i = end
                    continue
            i += 1
            while i < n:
                if body[i] == "\\":
                    i += 2
                    continue
                if body[i] == '"':
                    i += 1
                    break
                i += 1
            out.append('""')
            continue
        out.append(c)
        i += 1
    return "".join(out)


def _only_negative_asserts(body):
    """True when every assertion in `body` is a bare is_err()/is_none().

    Credit flows through helpers here, the same way it does for `no_assert`:
    a test whose own assertion is `is_err()` but which also calls a helper that
    asserts something concrete is not "only negative". Without that, moving an
    assertion into a helper (the natural refactor) silently re-appears the test
    in this list.
    """
    return _only_asserts_of_kind(body, Q_NEGATIVE_ASSERT_RE)


def _only_count_asserts(body):
    return _only_asserts_of_kind(body, Q_COUNT_ASSERT_RE)


def _only_asserts_of_kind(body, pattern):
    """Do all of `body`'s assertions — own and inherited via helpers — match
    `pattern`? A test with no assertions at all is not "only" anything.

    String literals are removed first: an assertion's failure message routinely
    quotes the rule ("opponent>=1"), and counting that as the assertion's shape
    misfiles specific-value tests as count-only.
    """
    body = _strip_rust_strings(body)
    calls = list(_assert_calls(body))
    if not calls:
        return False
    if not all(pattern.search(c) for c in calls):
        return False
    # Everything the test says itself is of that kind. It also counts as "only"
    # when every helper it leans on for evidence says the same kind of thing.
    for name, helper_calls in HELPER_ASSERT_CALLS.items():
        if re.search(r"\b" + re.escape(name) + r"\s*\(", body):
            for call in helper_calls:
                if not pattern.search(call):
                    return False
            for call in SUPPORT_ASSERT_CALLS.get(name, []):
                if not pattern.search(call):
                    return False
    return True


# Filled per test file by audit_test_quality: helper name -> its assert sources.
HELPER_ASSERT_CALLS = {}
# Same, for the shared test_modules/support fixtures.
SUPPORT_ASSERT_CALLS = {}
Q_OUTCOME_RE = re.compile(
    r"get_blade_modifier|get_heart_modifier|get_orientation_modifier|get_score_modifier"
    r"|blade_modifiers|heart_modifiers|orientation_modifiers|need_heart|success_zone"
    r"|active_count\(\)|get_under_cards|under_cards|waitroom\.cards\.len|hand\.cards\.len"
    r"|energy_zone\.cards\.len|energy_deck\.cards\.len|live_card_zone\.cards|success_live"
    r"|\.contains\(|is_empty\(\)|![\w.]+\.has_pending_choice"
    r"|choice_count|prompt_count|n_choices|is_idle\(\)"
)
Q_OUTCOME_MOD_RE = re.compile(
    r"get_blade_modifier|get_heart_modifier|get_orientation_modifier|get_score_modifier"
)
Q_TRIGGER_CTX_RE = re.compile(r"自動|trigger|jidou|watch|fire|auto_|debut|登場|live_start|live_success|main_phase", re.IGNORECASE)
# Tests asserting on the parsed ability AST (not game behavior) need no
# engine drive: they inspect resolved_abilities()/effect structure directly.
Q_PARSE_AUDIT_RE = re.compile(
    r"resolved_abilities|operation_any|value_or_count|heart_colors_any"
    r"|compound\.actions|effect_steps|get_aggregate\(\)"
)
Q_CARD_NO_RE = re.compile(r'"(PL![A-Za-z0-9!\-+＋]+?)"')
# Rust unicode escapes: "PL!N-bp4-007-R\u{ff0b}" is the fullwidth-plus print of
# "PL!N-bp4-007-R+". Unescape before substring matching so L0 scans don't miss
# escaped card numbers (previously caused false untested-ability lists).
Q_RUST_ESCAPE_RE = re.compile(r"\\u\{([0-9a-fA-F]{4,6})\}")
Q_SET_SEG_RE = re.compile(r"-(bp\d+|sd\d+|pb\d+|cl\d+|PR)-")


def strip_rust_line_comments(text):
    return re.sub(r"//[^\n]*", "", text)


Q_FN_ATTR_RE = re.compile(r"#\s*\[[^\]\n]*\][ \t]*(?:\r?\n|$)")


def is_test_fn(text, fn_start):
    """True when `fn` at fn_start is directly annotated #[test].

    Walks backwards over whitespace and other attribute lines only. A
    fixed-length lookback used to answer this, which mistook a helper
    following a short test fn for a test (the previous #[test] was still
    inside the window) and so denied that helper assert/drive/identity
    credit — faking no_assert/no_drive/pendency_only rows.
    """
    i = fn_start
    window = max(0, fn_start - 2000)
    while True:
        j = i
        while j > window and text[j - 1] in " \t\r\n":
            j -= 1
        if j == window:
            return False
        m = None
        for m in Q_FN_ATTR_RE.finditer(text, window, j):
            pass
        if not m or m.end() != j:
            return False
        if m.group(0).lstrip().startswith("#["):
            attr = m.group(0)
            if re.match(r"#\s*\[\s*test\s*\]", attr):
                return True
        i = m.start()


def split_rust_fns(text, tests_only):
    """Split Rust source into (name, code-only body, start_line, is_test).

    Brace-matched from the fn's opening brace; string/char literals and
    comments are skipped so braces inside them don't break matching.
    With tests_only=True, only #[test] fns are returned (helpers skipped).
    """
    out = []
    pat = (
        r"^[ \t]*#\[test\][ \t]*\r?\n[ \t]*(?:pub[ \t]+)?fn[ \t]+(\w+)"
        if tests_only
        else r"^[ \t]*(?:pub[ \t]+)?fn[ \t]+(\w+)"
    )
    for m in re.finditer(pat, text, re.MULTILINE):
        name = m.group(1)
        # With tests_only the pattern itself implies #[test]; otherwise ask
        # is_test_fn whether the attribute is directly attached.
        is_test = tests_only or is_test_fn(text, m.start())
        if tests_only and not is_test:
            continue
        i = text.find("{", m.end())
        if i < 0:
            continue
        depth = 0
        j = i
        instr = None
        incomment = None
        while j < len(text):
            c = text[j]
            nxt = text[j + 1] if j + 1 < len(text) else ""
            if incomment == "line":
                if c == "\n":
                    incomment = None
            elif incomment == "block":
                if c == "*" and nxt == "/":
                    incomment = None
                    j += 1
            elif instr:
                if c == "\\":
                    j += 1
                elif c == instr:
                    instr = None
            else:
                if c == "/" and nxt == "/":
                    incomment = "line"
                elif c == "/" and nxt == "*":
                    incomment = "block"
                elif c in "\"'":
                    instr = c
                elif c == "{":
                    depth += 1
                elif c == "}":
                    depth -= 1
                    if depth == 0:
                        break
            j += 1
        body = strip_rust_line_comments(text[i : j + 1])
        start_line = text.count("\n", 0, m.start()) + 1
        out.append((name, body, start_line, is_test))
    return out


def split_test_fns(text):
    """Split test source into (name, code-only body, start_line) tuples.

    See split_rust_fns for the matching rules.
    """
    return [
        (name, body, line)
        for name, body, line, _is_test in split_rust_fns(text, True)
    ]


def card_group_key(card_no):
    # Wildcard the set segment (bp2 vs pb2) so confusable prints group
    # together; the rarity suffix stays so same-card reprints don't flag.
    return Q_SET_SEG_RE.sub("-SET-", card_no)


def helper_profiles(text):
    """{profile: {fn_name}} for the non-#[test] fns in one Rust source.

    Profiles: assert / scan / ident / outcome. Closure over calls means a
    wrapper inherits whatever its callees already earn.
    """
    bodies = {
        name: body
        for name, body, _line, is_test in split_rust_fns(text, False)
        if not is_test
    }
    credited = {
        "assert": {n for n, b in bodies.items() if Q_ASSERT_RE.search(b)},
        "scan": {n for n, b in bodies.items() if Q_SCAN_RE.search(b)},
        "ident": {n for n, b in bodies.items() if Q_IDENT_RE.search(b)},
        "outcome": {
            n
            for n, b in bodies.items()
            if Q_OUTCOME_RE.search(b) or Q_OUTCOME_MOD_RE.search(b)
        },
    }

    def _close(names):
        changed = True
        while changed:
            changed = False
            for n, b in bodies.items():
                if n in names:
                    continue
                if any(re.search(r"\b" + c + r"\s*\(", b) for c in names):
                    names.add(n)
                    changed = True
        return names

    return {k: _close(set(v)) for k, v in credited.items()}, bodies


SUPPORT_RE = re.compile(
    r"use\s+crate::test_modules::support::(\w+)::\s*\*\s*;"
)


def support_profiles_by_module(files):
    """{module_name: {profile: {fn_name}}} for test_modules/support/*.

    These are the shared driving fixtures (play_to_stage, activate, drain).
    A test that hands the whole gameplay turn to one of them is driving the
    engine, so crediting them keeps no_drive honest.
    """
    out = {}
    for p, rel, text, _fns in files:
        rel = rel.replace("\\", "/")
        if "/test_modules/support/" not in rel or rel.endswith("/support/mod.rs"):
            continue
        mod = p.stem
        prof, bodies = helper_profiles(text)
        out[mod] = prof
        for name, hbody in bodies.items():
            SUPPORT_ASSERT_CALLS.setdefault(name, []).extend(
                _assert_calls(hbody)
            )
    return out


# A `game.id("…")` / `id_ref("…")` literal that is not a card number in the
# database. `get_card_id` is deliberately lenient: an unknown number falls back
# to "any print of this base", so a typo resolves to a DIFFERENT PRINT of the
# same card and the test stages the wrong card without failing. `PL!SP-bp2-006-R`
# (no such rarity) silently became `PL!SP-bp2-006-P`, whose printed text lacks
# the 常時 the test was checking — a "known engine gap" that was really a typo.
Q_ID_LITERAL_RE = re.compile(r'\b(?:id|id_ref)\(\s*"([^"]+)"')
# A card number bound to a NAME and passed by reference: `const KEKE: &str =
# "PL!SP-bp4-006-R";` … `game.id(KEKE)`.
#
# `Q_ID_LITERAL_RE` alone cannot see these, because the literal is not inside the
# call. That blind spot is not hypothetical: `PL!SP-bp1-014-PR` in
# `live_success_three_distinct_revealed_retrieves_live_test.rs` does not exist, the
# lenient `get_card_id` fallback substituted `PL!SP-bp1-014-N`, and the test passed
# while the "three DISTINCT names" premise it is named for went unpinned. A const
# binding is the dominant idiom for card numbers in this suite, so the detector has
# to follow the name, not the call site.
Q_BOUND_ID_RE = re.compile(
    r'\b(?:const|let)\s+[A-Za-z_][A-Za-z0-9_]*\s*:\s*&?\s*(?:str|String)\s*=\s*"([^"]+)"'
)


def _card_numbers():
    """Every card_no in cards/cards.json, or an empty set if it cannot be read.

    Loaded lazily and defensively: this list only powers the
    `unresolvable_card_id` smell, and a tool that refuses to run because the
    catalogue moved would be worse than one that reports nothing.
    """
    path = ROOT / "cards" / "cards.json"
    try:
        with open(path, encoding="utf-8") as f:
            data = json.load(f)
    except (OSError, ValueError):
        return set()
    if isinstance(data, dict) and isinstance(data.get("cards"), dict):
        return set(data["cards"].keys())
    if isinstance(data, dict):
        return {k for k, v in data.items() if isinstance(v, dict) and "card_no" in v}
    return set()


def stale_report_bases(rows):
    """Ability rows whose `base` is a PREFIX of no card in the pool.

    `base` is a 12-character truncation of a card number, so resolving one means
    "some card_no starts with this". `unresolvable_card_literals` checks what test
    SOURCES name, which is the other direction: a stale base could sit in this
    report itself and no test would ever name it, so nothing else here would see it.

    Kept as DEFENCE, not because an instance is known. When this was added the
    motivating case turned out to be me: a scratch listing printed
    `PL!N-pb3-014: UNRESOLVED`, I read past it, and hand-wrote `PL!N-pb3-014-N` as
    the card number, which panics on `game.id(...)`. There was never a bad row in the
    report. The check earns its place because the failure mode is real -- the report is
    what the L1+choice frontier is read from, and a base that resolves to nothing sends
    the reader to a card that does not exist -- not because the report is known to
    contain one.

    Sound in the strong direction: a base that IS a prefix of a real card is never
    reported, so a row is only flagged when nothing in the pool can satisfy it.
    """
    numbers = _card_numbers()
    if not numbers:
        return []
    out = []
    for r in rows:
        base = r.get("base")
        if not base:
            continue
        if not any(n.startswith(base) for n in numbers):
            out.append(base)
    return sorted(set(out))


def unresolvable_card_literals(files, card_numbers):
    """[(rel, line, literal)] for id literals absent from the card database.

    `card_numbers` is the set of normalised card_no keys. Mirrors the two
    normalisations `CardDatabase::normalize_card_no` performs (fullwidth→ASCII,
    lowercase→uppercase) so a ＋/＋ difference is not reported.

    Scans BOTH an inline `id("…")` literal and a card number bound to a name
    (`const X: &str = "…"`), because the binding form is the common idiom here and
    the call-site form alone cannot see it. A literal found by both patterns is
    reported once.
    """
    keys = {_normalise_card_no(k) for k in card_numbers}
    out = []
    seen = set()
    for _p, rel, text, _fns in files:
        # Inline `id("…")` may legitimately hold a non-card argument, so it keeps
        # the loose shape test. A NAME BINDING is different: the consts in this
        # suite hold card numbers, and the loose test also matched plain strings
        # like `live_card_zone`, so a binding must look like a real card number —
        # every member/live print is `PL!…`, and the `LL-`/`BD-`/`EN-` families are
        # skipped below as before.
        for pattern, require_pl_prefix in (
            (Q_ID_LITERAL_RE, False),
            (Q_BOUND_ID_RE, True),
        ):
            for m in pattern.finditer(text):
                s = m.group(1)
                if require_pl_prefix and not s.startswith("PL!"):
                    continue
                if not re.match(r"^[A-Za-z0-9!#\-_+]{6,}$", s):
                    continue  # not a card-number shape (a zone name, an option id)
                if s.startswith(("LL-", "BD-", "EN-")):
                    continue
                if s in keys or _normalise_card_no(s) in keys:
                    continue
                line = text[: m.start()].count("\n") + 1
                dedup = (rel, line, s)
                if dedup in seen:
                    continue
                seen.add(dedup)
                out.append((rel, line, s))
    return sorted(out)


def _normalise_card_no(s):
    out = []
    for ch in s:
        o = ord(ch)
        if 0xFF41 <= o <= 0xFF5A:  # fullwidth A-Z
            out.append(chr(o - 0xFEE0))
        elif "a" <= ch <= "z":
            out.append(ch.upper())
        elif ch == "＋":  # fullwidth plus
            out.append("+")
        else:
            out.append(ch)
    return "".join(out)


# Does this test exercise the gameplay engine at all? Used to scope no_drive:
# a card-database / bytecode / modifier-table test has nothing to drive.
Q_GAMEPLAY_RE = re.compile(r"TestGame|\bgame\.|player1|player2|main_deck|\.state\.")


def audit_test_quality(files):
    """Run smell detectors over collect_test_files() output.

    Returns {smell: [(rel, fn_name, line, detail), ...]} with detail "" except
    similar_cards (the confusable numbers).
    """
    smells = {
        "no_assert": [],
        "no_drive": [],
        "synthetic_only": [],
        "pendency_only": [],
        "assert_only_negative": [],
        "assert_only_counts": [],
        "live_success_no_premise": [],
        "live_success_impossible_live": [],
        "drain_zero_may_decline": [],
        "placeholder": [],
        "prompt_ordinal_drain": [],
        "blind_phase_stepping": [],
        "similar_cards": [],
        "unpinned_similar_cards": [],
        "duplicate_stage_id": [],
        "count_inequality_only": [],
    }
    support = support_profiles_by_module(files)
    for _p, rel, text, _fns in files:
        fns = split_test_fns(text)
        # Helper profiles: a test calling a helper that asserts/drives gets
        # credit (e.g. check_heart_reduction asserts, play_three drives).
        # Helpers are file-local non-#[test] fns.
        local, local_bodies = helper_profiles(text)
        helper_assert = set(local["assert"])
        helper_scan = set(local["scan"])
        helper_ident = set(local["ident"])
        helper_outcome = set(local["outcome"])
        # Shared fixtures under test_modules/support/: a test that starves the
        # turn to one of them (replace_member_by_baton_touch -> play_to_stage)
        # really did drive the engine.
        for mod in SUPPORT_RE.findall(text):
            prof = support.get(mod)
            if not prof:
                continue
            helper_assert |= prof["assert"]
            helper_scan |= prof["scan"]
            helper_ident |= prof["ident"]
            helper_outcome |= prof["outcome"]

        def _calls(names):
            return (
                re.compile(r"\b(" + "|".join(sorted(names)) + r")\s*\(")
                if names
                else None
            )

        helper_assert_re = _calls(helper_assert)
        helper_scan_re = _calls(helper_scan)
        helper_ident_re = _calls(helper_ident)
        helper_outcome_re = _calls(helper_outcome)
        # Per-file view of what each helper actually asserts, so
        # assert_only_negative / assert_only_counts can see evidence a test
        # delegated to a helper.
        HELPER_ASSERT_CALLS.clear()
        for hname, hbody in local_bodies.items():
            HELPER_ASSERT_CALLS[hname] = _assert_calls(hbody)
        # Helpers that pin a SPECIFIC value. `assert_only_negative` and
        # `assert_only_counts` already reach into helper bodies (see
        # `_only_asserts_of_kind`), but the `count_inequality_only` test below is
        # an inline body-only string test, so a test that delegates its
        # `assert_eq!` to a fixture helper was reported as inequality-only however
        # precisely the helper asserts. Naming those helpers closes that.
        helper_pins_value = {
            hname
            for hname, hbody in local_bodies.items()
            if "assert_eq!" in hbody or "assert_ne!" in hbody
        }
        helper_pins_value_re = (
            re.compile(r"\b(" + "|".join(sorted(helper_pins_value)) + r")\s*\(")
            if helper_pins_value
            else None
        )
        file_groups = {}
        # Const/let-bound card numbers count toward the confusable-number check
        # too. The per-fn scan below only sees numbers written inside a test body,
        # and `const CARD: &str = "PL!…"` at file level is the dominant idiom here
        # — so a file that transposes `bp2`/`pb2` through its consts got no
        # similar_cards row at all, which is the same blind spot that let
        # `PL!SP-bp1-014-PR` (a number that does not exist) go unreported.
        for bound in Q_BOUND_ID_RE.findall(text):
            if bound.startswith("PL!"):
                file_groups.setdefault(card_group_key(bound), set()).add(bound)
        for name, body, line in fns:
            has_assert = any(
                not Q_TRIVIAL_ASSERT_RE.search(call) for call in _assert_calls(body)
            ) or bool(helper_assert_re and helper_assert_re.search(body))
            if not has_assert:
                smells["no_assert"].append((rel, name, line, ""))
            driven = bool(Q_SCAN_RE.search(body)) or bool(
                helper_scan_re and helper_scan_re.search(body)
            )
            # `no_drive` asks "did a gameplay test set up state without ever
            # driving the engine?". A test that never builds a TestGame at all
            # — a card-database lookup, a bytecode decode, a modifier-table
            # unit check — is not a gameplay test, so the question does not
            # apply to it and it must not be listed.
            is_gameplay = bool(Q_GAMEPLAY_RE.search(body)) or bool(
                helper_scan_re and helper_scan_re.search(body)
            )
            if (
                Q_SETUP_RE.search(body)
                and has_assert
                and not driven
                and Q_TRIGGER_CTX_RE.search(body)
                and not Q_PARSE_AUDIT_RE.search(body)
                and is_gameplay
            ):
                smells["no_drive"].append((rel, name, line, ""))
            # synthetic_only: hand-pushed trigger events are weak evidence only
            # when the fn asserts a watcher outcome (modifier grants). Pure
            # tracking-layer characterization tests assert the views
            # themselves and are out of scope.
            if (
                Q_SYNTH_RE.search(body)
                and not driven
                and Q_OUTCOME_MOD_RE.search(body)
            ):
                smells["synthetic_only"].append((rel, name, line, ""))
            if (
                Q_PEND_RE.search(body)
                and not Q_IDENT_RE.search(body)
                and not Q_OUTCOME_RE.search(body)
                and not Q_OUTCOME_MOD_RE.search(body)
                and not (helper_ident_re and helper_ident_re.search(body))
                and not (helper_outcome_re and helper_outcome_re.search(body))
                and Q_TRIGGER_CTX_RE.search(body)
            ):
                smells["pendency_only"].append((rel, name, line, ""))
            # Every assertion is a bare is_err()/is_none(): a real guard fired,
            # but nothing says WHICH one, so a regression that trips a
            # different guard still passes.
            if has_assert and _only_negative_asserts(body):
                smells["assert_only_negative"].append((rel, name, line, ""))            # Every assertion is about a count/size, never a specific value
            # tied to the card under test: "3 options were offered" can hold
            # while the 3 are the wrong 3.
            if has_assert and _only_count_asserts(body):
                smells["assert_only_counts"].append((rel, name, line, ""))
            if Q_PLACEHOLDER_RE.search(body):
                smells["placeholder"].append((rel, name, line, ""))
            if _has_ordinal_drain(body):
                smells["prompt_ordinal_drain"].append((rel, name, line, ""))
            if Q_BLIND_PHASE_RE.search(body) and not Q_PHASE_PIN_RE.search(body):
                smells["blind_phase_stepping"].append((rel, name, line, ""))
            # The same card INSTANCE in two stage slots is not a legal board:
            # `stage.stage = [filler, yoshiko, filler]` is one card standing in
            # for two. Any effect that counts members, or dedupes by name or id,
            # then measures something no real board produces. Report-only — most
            # rows are padding a test does not count on, so the review prompt is
            # "does this assertion depend on the count?", not "rewrite 141
            # fixtures". `-1` (an empty slot) is not a duplicate.
            for dup_name, dup_slots in _duplicate_stage_ids(body):
                smells["duplicate_stage_id"].append(
                    (rel, name, line, "%s in %d slots" % (dup_name, dup_slots))
                )
            # Every assertion is an INEQUALITY on a count, and the test contains
            # no assert_eq at all. `assert!(deck.len() < before)` holds for a
            # whole band of values: the draw could be 1 card or 9, or the deck
            # could be shrinking for an unrelated reason (the yell discards, the
            # next turn draws). A threshold against a literal is weaker still.
            if (
                has_assert
                and "assert_eq!" not in body
                and "assert_ne!" not in body
                and not (helper_pins_value_re and helper_pins_value_re.search(body))
            ):
                band = _count_inequality_only(body)
                if band:
                    smells["count_inequality_only"].append((rel, name, line, band))
            for cn in Q_CARD_NO_RE.findall(body):
                if cn.startswith("PL!"):
                    file_groups.setdefault(card_group_key(cn), set()).add(cn)
            for m in Q_RUST_ESCAPE_RE.finditer(body):
                start = body.rfind('"', 0, m.start())
                end = body.find('"', m.end())
                if 0 <= start < end:
                    cn = Q_RUST_ESCAPE_RE.sub(
                        lambda e: chr(int(e.group(1), 16)),
                        body[start + 1 : end],
                    )
                    if cn.startswith("PL!"):
                        file_groups.setdefault(card_group_key(cn), set()).add(cn)
        for _key, nos in file_groups.items():
            if len(nos) > 1:
                detail = ", ".join(sorted(nos))
                # A file that transposes confusable card numbers is only as
                # trustworthy as its ability to notice a mis-staged card. Files
                # that pin identity (assert_card_identity, or compare card_no)
                # have that guard; the rest go to their own list so the review
                # prompt is "add the pin", not "reread all 29".
                pinned = bool(Q_CARD_PIN_RE.search(text))
                if not pinned:
                    for mod, prof in support.items():
                        for names in prof.values():
                            if Q_CARD_PIN_RE.search("|".join(sorted(names))):
                                pinned = True
                smells["similar_cards" if pinned else "unpinned_similar_cards"].append(
                    (rel, "<file>", 1, detail)
                )
    for rows in smells.values():
        rows.sort()
    # File-level: a file that drives ライブ成功時 through the GATED path without ever
    # establishing that its live succeeded. `trigger_live_success_abilities` gates on
    # `should_trigger_live_success`, so a live that failed silently dispatches
    # nothing — and a negative assertion in such a file then passes whether or not
    # the ability does anything. Files built on `fire_trigger` are excluded: that
    # helper forces the dispatch and has no premise to state.
    for _p, rel, text, _fns in files:
        code = Q_LINE_COMMENT_RE.sub("", text)
        if Q_LIVE_SUCCESS_CALL_RE.search(code) and not Q_LIVE_SUCCESS_PREMISE_RE.search(code):
            smells["live_success_no_premise"].append((rel, "<file>", 1, ""))
    # A live no stage can satisfy makes its ライブ成功時 unreachable, so the test is
    # vacuous BY CONSTRUCTION rather than merely missing a premise. Reported
    # separately because the remedy differs: the premise assertion is not enough
    # here, the test has to inject synthetic hearts deliberately or pick another
    # live.
    unplayable = unplayable_live_cards()
    for _p, rel, text, _fns in files:
        code = Q_LINE_COMMENT_RE.sub("", text)
        if not Q_LIVE_SUCCESS_CALL_RE.search(code):
            continue
        for card_no, why in unplayable.items():
            if card_no in code:
                smells["live_success_impossible_live"].append(
                    (rel, card_no, 1, "; ".join(why))
                )
                break
    # A drain of [0] that can silently DECLINE a skippable selection, in a file whose
    # assertion is an absence. Exposure, not a defect -- see the comment above.
    optional_select = card_numbers_with_optional_select()
    for _p, rel, text, _fns in files:
        if not Q_DRAIN_ZERO_RE.search(text) or not Q_ABSENCE_ASSERT_RE.search(text):
            continue
        hit = sorted(c for c in optional_select if c in text)
        if hit:
            smells["drain_zero_may_decline"].append((rel, ",".join(hit[:3]), 1, ""))
    for rows in smells.values():
        rows.sort()
    n_fns = sum(len(split_test_fns(text)) for _p, _rel, text, _fns in files)
    return smells, n_fns

SMELL_DOCS = {
    "no_assert": "test never asserts (smoke at best — cannot pin behavior)",
    "no_drive": "trigger-context test that mutates state and asserts but never drives the engine (no scan/activate/play/fire): vacuous negatives/positives",
    "synthetic_only": "trigger event hand-pushed with no real ability resolution in the same fn (weaker trigger evidence)",
    "pendency_only": "has_pending_choice asserted without choice identity (pending_choice_type/summary/answer) or outcome asserts",
    "assert_only_negative": "every assertion is a bare is_err()/is_none() — some guard fired, but nothing says which, so a regression tripping a different guard still passes",
    "assert_only_counts": "every assertion is about a count/size (len/count/>=1) — '3 options were offered' can hold while the 3 are the wrong 3",
    "live_success_no_premise": "file CALLS trigger_live_success_abilities but never asserts the state of the ライブ成功時 window it dispatches through. That path checks `should_trigger_live_success`, so a live which failed silently dispatches nothing — and a NEGATIVE assertion in such a file then passes whether or not the ability does anything. The remedy is NOT always 'assert the window is open': a test of the heart requirement itself depends on the window being CLOSED, so each site must assert the state it actually relies on. Files built on `fire_trigger` are NOT flagged: it calls `trigger_auto_ability` directly and forces the dispatch, so there is no window to state. The fix is to assert `should_trigger_live_success` (or the performance snapshot) for the state the test depends on. See the comment above for the four scopings that were wrong first, and note that a live's `heart0` requirement is a COLORLESS WILDCARD satisfied by any colour, not a literal colour",
    "live_success_impossible_live": "file CALLS trigger_live_success_abilities on a live whose printed need_heart no stage in the pool can satisfy — a requirement above what ANY single member prints, and only three members fit on a stage. Such a live's ライブ成功時 is unreachable, so the test is vacuous BY CONSTRUCTION, which is strictly stronger than a missing premise: adding a should_trigger_live_success assertion here would just fail. The remedy is to inject synthetic hearts deliberately (a heart00 wildcard, which the engine treats as an unbounded wildcard in the PROVIDED hearts, as the files in live_success_no_premise do) or to pick a live the pool can satisfy. The card set itself is worth reviewing: 46 of the 291 lives carrying a need_heart (15.8%) are provably unplayable, which is a card-DATA observation and not a harness one. The check is a LOWER BOUND — necessary, not sufficient, since three slots can still conflict across colours — so 'not reported' means 'not provably unplayable'",
    "drain_zero_may_decline": "file drains a prompt with index [0] AND asserts an absence, while naming a card whose select_cards is `optional`. On a skippable prompt [0] is NOT reliably 'take' and NOT reliably 'skip' — it depends on the PROMPT KIND and the harness does not say which. An optional-COST prompt takes the card on [0] (look_at_deck_top_optional_discard_test.rs proves it: two tests differing only in [0] vs []); a `looked_at` selection behaves differently again. So the hazard is AMBIGUITY: a test intending to take and instead declines passes silently when its assertion is an absence, which a positive assertion would have caught. `assert_select_card(zone, n, allow_skip)` states the prompt. EXPECTED YIELD IS LOW and measured, not hoped: 30 of these files assert absence directly about a look or selection, and the two read to check are both exemplars that state the prompt before answering and pair every absence with a positive twin. Read a row as 'spend five minutes here', not as 49 problems",
    "placeholder": "#[ignore], assert!(true), todo!() or unimplemented!() left in a test",
    "similar_cards": "confusable card numbers (bp2 vs pb2) staged in one file AND the file pins card identity (assert_card_identity / compares card_no), so a transposition would fail loudly",
    "unpinned_similar_cards": "confusable card numbers (bp2 vs pb2) staged in one file with NO card-identity pin — a transposed print would pass silently; add assert_card_identity to close it",
    "duplicate_stage_id": "the same card INSTANCE in two stage slots (`stage.stage = [filler, y, filler]`) — not a legal board; anything that counts members or dedupes by name measures a board that cannot occur, so use a second `new_id`",
    "count_inequality_only": "every assertion is an INEQUALITY on a count and there is no assert_eq anywhere — a band, not a value, so 1 card and 9 both pass; pin the number the card prints",
    "unresolvable_card_id": "a card-number literal that is not in the database — either written inline in a `game.id(\"…\")` or bound to a name (`const X: &str = \"…\"` / `let x: &str = \"…\"`), because const-bound card numbers are the dominant idiom here and the call-site form alone cannot see them. get_card_id's lenient fallback silently substitutes a DIFFERENT PRINT of the same card, so the test stages the wrong card and passes: `PL!SP-bp1-014-PR` resolved to `PL!SP-bp1-014-N` and left a \"three DISTINCT names\" premise unpinned",
    "prompt_ordinal_drain": "prompts answered by ORDINAL position (if step <= 2 { select 2 cards }) instead of by the prompt's own identity — adding or removing one prompt upstream silently changes which prompt gets the answer",
    "blind_phase_stepping": "a fixed `for _ in 0..N { pass() }` walk through the turn — a phase gaining or losing a step silently shifts the window the test thinks it is standing in; step to the phase by name instead",
}


def render_quality(smells, inv):
    w = []
    a = w.append
    a("# Test quality smells (static audit)")
    a("")
    a("_Auto-generated by `cards/test_inventory.py` — do not edit by hand. Rerun `python cards/test_inventory.py` after changing tests._")
    a("")
    a("Fn-level static signals over `engine/tests/**/*.rs`. Coverage counts (TEST_COVERAGE.md)")
    a("answer “does it fire”; this answers “would the test notice if the trigger were wrong”.")
    a("Report-only: rows are review prompts, not failures.")
    a("")
    total_fns = inv["stats"].get("n_test_fns_parsed", 0)
    if total_fns:
        a(f"Parsed {total_fns} `#[test]` fns.")
        a("")
    for smell, rows in smells.items():
        a(f"## {smell} ({len(rows)})")
        a("")
        a(f"_{SMELL_DOCS[smell]}_")
        a("")
        if not rows:
            a("None.")
        else:
            a("| file | test | line | detail |")
            a("| --- | --- | --- | --- |")
            shown = rows[:80]
            for rel, name, line, detail in shown:
                a(f"| `{rel}` | `{name}` | {line} | {detail} |")
            if len(rows) > len(shown):
                a(f"| … | {len(rows) - len(shown)} more |  |  |")
        a("")
    return "\n".join(w)


def infer_ability_depth(covering_texts, covering_rels, covering_fns, absence_in_direct=False):
    """Return depth label and flags for an ability.

    Negative coverage is credited two ways: a NEGATIVE_RE name hint on the
    covering file or test name, OR an absence assertion inside a test that actually
    drives this ability. The second signal exists because the name hint alone
    reported 67 abilities as needing a negative test when most already had one —
    a bucket that size cries wolf, and cries wolf is why gaps go unworked.
    """
    if not covering_texts:
        return "none", {"has_assert": False, "has_choice": False, "has_negative": False}
    has_assert = any("assert" in t for t in covering_texts)
    has_choice = any(bool(CHOICE_RE.search(t)) for t in covering_texts)
    has_negative = (
        any(bool(NEGATIVE_RE.search(r)) for r in covering_rels)
        or any(bool(NEGATIVE_RE.search(fn)) for fn in covering_fns)
        or absence_in_direct
    )
    if has_negative and has_assert:
        depth = "L2"
    elif has_assert:
        depth = "L1"
    else:
        depth = "L0"
    # upgrade hint if choice signals
    if has_choice and depth in ("L1", "L2"):
        depth = depth + "+choice"
    return depth, {"has_assert": has_assert, "has_choice": has_choice, "has_negative": has_negative}


def _fn_bodies_assert_absence(fn_bodies):
    """True when any of these test-fn bodies makes an absence claim.

    Structural counterpart to the NEGATIVE_RE name hint, and scoped to the fns
    that actually DRIVE the ability rather than to the whole covering file. That
    scope is the point: a file covering five cards routinely contains an absence
    assertion belonging to a card other than the one being scored, and crediting
    it would move that ability up the ladder on someone else's evidence.

    Helper bodies count, for the same reason `has_assert` already follows them
    (`_only_asserts_of_kind`): moving an assertion into a fixture is the natural
    refactor and must not silently un-credit the test.
    """
    for body in fn_bodies:
        stripped = _strip_rust_strings(body)
        if Q_ABSENCE_ASSERT_RE.search(stripped):
            return True
    return False


def lifecycle_signals_in(body):
    """Return {property: [signal, ...]} for the lifecycle properties in one fn body.

    Deliberately syntactic, and deliberately returns WHICH signal matched rather
    than a bare True — so the coverage table can show its evidence and a reader
    can disagree with a specific call instead of the whole verdict.

    This is a heuristic over test SOURCE, not a proof that a property holds. An
    earlier single-token-per-property version of this check produced three
    separate rounds of false positives (it did not see a resolve-twice idempotence
    test, a duration rollover, or an absence assertion), which is why each
    property now accepts several honest signals and names them.

      idempotence — a re-deriving driver twice or in a loop (constant-scan shape),
                    or a resolution entry point called at least twice (resolve-twice
                    shape: "a second resolution does not grow the total").
      revocation   — a zone exit / explicit clear, OR a phase walk that can cross
                    a duration boundary, OR an assertion that the registration is
                    gone.
      surface      — the generated action list is read, so the offer path is
                    validated rather than only the internal state tables.
    """
    driver_hits = len(LIFECYCLE_DRIVER_RE.findall(body))
    in_loop = any(
        LIFECYCLE_DRIVER_RE.search(seg)
        for seg in LIFECYCLE_LOOP_RE.split(body)[1:]
    )
    resolve_hits = len(LIFECYCLE_RESOLVE_RE.findall(body))
    signals = {"idempotence": [], "revocation": [], "surface": []}
    if driver_hits >= 2:
        signals["idempotence"].append("driver x%d" % driver_hits)
    elif in_loop:
        signals["idempotence"].append("driver in loop")
    if resolve_hits >= 2:
        signals["idempotence"].append("resolve x%d" % resolve_hits)
    if LIFECYCLE_ZONEXIT_RE.search(body):
        signals["revocation"].append("zone exit / clear")
    if LIFECYCLE_ROLLOVER_RE.search(body):
        signals["revocation"].append("duration rollover")
    if LIFECYCLE_ABSENCE_RE.search(body):
        signals["revocation"].append("registration absence assert")
    if LIFECYCLE_SURFACE_RE.search(body):
        signals["surface"].append("action list read")
    return signals


def required_lifecycle_properties(effect):
    """The lifecycle properties that APPLY to this effect.

    SURFACE is scoped by what the effect GRANTS, not by which action it uses —
    see `_grants_activation`. A 常時 or ライブ成功時 grant is consumed by a
    constant scan or a trigger dispatch and has no action-list representation, so
    holding it to the offer surface would be a fabricated requirement.
    """
    props = ["idempotence", "revocation"]
    if _grants_activation(effect):
        props.append("surface")
    return props


def missing_lifecycle_properties(effect, bodies):
    """Return (missing_properties, {property: [signals that matched]}) for an ability.

    Evidence is unioned across the ability's direct test fns, not required of a
    single one: the property is a claim about the ability, and demanding one fn
    carry all of it would reward a long test over a correct split. The matched
    signals are returned alongside so the report can show its evidence.
    """
    have = {"idempotence": [], "revocation": [], "surface": []}
    for body in bodies:
        for prop, found in lifecycle_signals_in(body).items():
            for signal in found:
                if signal not in have[prop]:
                    have[prop].append(signal)
    missing = [p for p in required_lifecycle_properties(effect) if not have[p]]
    return missing, have


def build_qa_coverage(all_src):
    """Coverage of official QA rulings (cards/qa_data.json).

    A ruling is covered when an engine test/src file references its Q-id
    (e.g. "Q117") or one of its related card numbers (full-width ＋
    normalized to +). Answers "is this ruling tested?" mechanically.
    """
    if not QA_JSON.exists():
        return {"rows": [], "covered": 0, "total": 0}
    with open(QA_JSON, encoding="utf-8") as f:
        rulings = json.load(f)
    # Q-ids also live in the in-crate QA suite (engine/src/qa_test_suite.rs)
    corpus = [all_src]
    for p in (ROOT / "engine" / "src").rglob("*.rs"):
        try:
            corpus.append(p.read_text(encoding="utf-8", errors="replace"))
        except OSError:
            continue
    corpus = "\n".join(corpus)
    rows = []
    for r in rulings:
        qid = r.get("id", "?")
        cards = [
            c.get("card_no", "").replace("＋", "+")
            for c in r.get("related_cards", [])
        ]
        by_qid = qid in corpus
        by_card = any(c and c in corpus for c in cards)
        rows.append({
            "id": qid,
            "cards": cards,
            "covered": by_qid or by_card,
            "via": "qid" if by_qid else ("card" if by_card else "none"),
        })
    covered = sum(1 for r in rows if r["covered"])
    return {"rows": rows, "covered": covered, "total": len(rows)}


def build_families(inv):
    """Group abilities into behavioral families.

    Returns a list of family dicts sorted by size:
      {name, triggers, total, covered, variants: {variant_key: [ability rows]}}
    Abilities matching no rule land in 'other/<trigger>:<action>'.
    """
    rows = inv["abilities"]
    families = defaultdict(list)
    for r in rows:
        eff = r["effect"] or {}
        placed = False
        for name, trig_filter, matcher in FAMILY_RULES:
            if not (set(r["trigger_list"]) & trig_filter):
                continue
            variant_key = None
            try:
                variant_key = matcher(eff, r["cost"])
            except Exception:
                variant_key = None
            if variant_key:
                families[name].append((r, str(variant_key)))
                placed = True
                break
        if not placed:
            families[f"other:{r['triggers'] or '(none)'}:{r['action']}"].append((r, ""))

    out = []
    for name, members in families.items():
        variants = defaultdict(list)
        for r, vk in members:
            variants[vk].append(r)
        out.append({
            "name": name,
            "triggers": sorted({t for r, _ in members for t in r["trigger_list"]}),
            "total": len(members),
            "covered": sum(1 for r, _ in members if r["covered"]),
            "variants": dict(variants),
        })
    out.sort(key=lambda f: -f["total"])
    return out


def render_families(inv, families):
    """Render docs/ABILITY_FAMILIES.md — family -> variant -> abilities."""
    lines = []
    w = lines.append
    w("# Ability Families — behavioral grouping of all unique abilities")
    w("")
    w("_Auto-generated by `cards/test_inventory.py --families` — do not edit by hand._")
    w("")
    w(f"Groups all {len(inv['abilities'])} unique abilities from `cards/abilities.json` by the")
    w("behavioral contract their parsed effect states (trigger x action x discriminating")
    w("detail). Test references are **candidate** L0 hits (the card number appears in the")
    w("file), NOT verified assertions — see TEST_COVERAGE.md for depth semantics.")
    w("")
    w("## Index")
    w("")
    w("| Family | Abilities | Covered | Variants |")
    w("|---|---|---|---|")
    for fam in families:
        w(f"| [{fam['name']}](#{fam['name']}) | {fam['total']} | {fam['covered']} | {len(fam['variants'])} |")
    w("")
    for fam in families:
        w(f"## {fam['name']}")
        w("")
        w(f"Triggers: {', '.join(fam['triggers'])} — {fam['covered']}/{fam['total']} L0-covered.")
        w("")
        for vk in sorted(fam["variants"].keys()):
            rows = fam["variants"][vk]
            w(f"### variant: {vk or '(no variant)'} — {len(rows)} abilities")
            w("")
            w("| Card | Depth | Tests | Text |")
            w("|---|---|---|---|")
            shown = sorted(rows, key=lambda r: (-r["covering_test_count"], r["idx"]))
            for r in shown[:40]:
                safe = r["full_text"].replace("|", "/").replace("\n", " ")
                w(f"| `{r['base']}` | {r['depth']} | {r['covering_test_count']} | {safe[:130]} |")
            if len(rows) > len(shown):
                w(f"| … | {len(rows) - len(shown)} more |  |  |")
            w("")
    return "\n".join(lines) + "\n"


def build_inventory():
    abilities, stats = load_abilities()
    files = collect_test_files()
    # concatenated source for quick substring checks (legacy parity)
    all_src = "\n".join(t for _, _, t, _ in files)
    n_files = len(files)
    n_tests = sum(len(fns) for _, _, _, fns in files)

    rows = []
    # per-file test-name -> body, plus the file's `const NAME: &str = "CARD"`
    # bindings, so a card number can be attributed to the tests that actually
    # drive it rather than to every test in a file that mentions it somewhere.
    # A file typically binds its card to a const and the test bodies then use
    # the const NAME, so resolving bindings is what keeps "direct" honest —
    # without it, a well-written file reads as zero coverage.
    fn_bodies = {}
    const_binds = {}
    helper_bodies = {}
    for _p, rel, text, _fns in files:
        # Unescape on the way in: a file may write the card as
        # "PL!N-bp7-011-R＋" or as "PL!N-bp7-011-R＋" — same card, and the
        # alias has to compare equal to the inventory's card number.
        binds = {
            m.group(1): Q_RUST_ESCAPE_RE.sub(
                lambda mm: chr(int(mm.group(1), 16)), m.group(2)
            )
            for m in Q_CONST_CARD_BIND_RE.finditer(text)
        }
        const_binds[rel] = binds
        try:
            fn_bodies[rel] = {n: b for n, b, _l in split_test_fns(text)}
            # A test often reaches the card through a fixture helper
            # (`fn trigger_kosuzu_live_start()` stages it) and never names it
            # itself. Those tests ARE direct coverage, so a helper that touches
            # the card makes every test that calls it direct.
            helper_bodies[rel] = {
                n: b for n, b, _l, is_test in split_rust_fns(text, False) if not is_test
            }
        except Exception:
            fn_bodies[rel] = {}
            helper_bodies[rel] = {}
    # mechanic counters
    trigger_counts = defaultdict(lambda: [0, 0])
    action_counts = defaultdict(lambda: [0, 0])
    cond_counts = defaultdict(lambda: [0, 0])
    set_counts = defaultdict(lambda: [0, 0])
    depth_counts = defaultdict(int)
    matrix = defaultdict(lambda: defaultdict(lambda: [0, 0]))  # trigger -> action -> [covered,total]

    # per-ability gap
    untested = []

    for idx, u in enumerate(abilities):
        cards = [c.split(" | ")[0] for c in u.get("cards", [])]
        base = card_base(cards[0]) if cards else "?"
        trig_raw = u.get("triggers") or ""
        triggers = [t.strip() for t in trig_raw.split(",") if t.strip()]
        if not triggers:
            triggers = ["(none)"]
        eff = u.get("effect") or {}
        act = effect_action(eff)
        cond = condition_type(eff)
        sset = card_set(cards[0]) if cards else "other"
        full_text = u.get("full_text") or ""

        # --- coverage: L0 via substring match (parity with coverage_report.py) ---
        covered_rels = []
        covering_texts = []
        covering_fns = []
        direct_fns = []
        direct_bodies = []
        covers_override = None
        for p, rel, text, fns in files:
            # check @covers override first
            if COVERS_RE.search(text):
                # if any card in ability matches an @covers line, treat as covered
                for m in COVERS_RE.finditer(text):
                    if m.group(1).strip() in cards or card_base(m.group(1).strip()) == base:
                        covers_override = m.group(1).strip()
            hit = any(c in text for c in cards) or (base in text)
            if not hit:
                # Same card_no written with Rust unicode escapes
                # (e.g. "PL!N-bp4-007-R\u{ff0b}" for the R+ print).
                esc = Q_RUST_ESCAPE_RE.search(text)
                if esc:
                    unescaped = Q_RUST_ESCAPE_RE.sub(
                        lambda m: chr(int(m.group(1), 16)), text
                    )
                    hit = any(c in unescaped for c in cards) or (base in unescaped)
            if hit:
                covered_rels.append(rel)
                covering_texts.append(text)
                covering_fns.extend(fns)
                # Tests that reach this ability's card from their OWN body, from
                # a const bound to it in this file, or through a fixture
                # helper that touches it. A test in a covering file that reaches
                # the card by none of those is co-located, not evidence.
                binds = const_binds.get(rel, {})
                aliases = {
                    a for a, card in binds.items() if card in cards or card == base
                }
                helpers = {
                    h
                    for h, hb in helper_bodies.get(rel, {}).items()
                    if any(c in hb for c in cards) or base in hb
                }
                for name, body in fn_bodies.get(rel, {}).items():
                    if (
                        any(c in body for c in cards)
                        or base in body
                        or any(
                            re.search(r"\b" + re.escape(a) + r"\b", body)
                            for a in aliases
                        )
                        or any(
                            re.search(r"\b" + re.escape(h) + r"\b", body)
                            for h in helpers
                        )
                    ):
                        direct_fns.append(name)
                        # The test's own body, PLUS the bodies of the local
                        # helpers it calls. This suite is written with
                        # well-factored fixture helpers, so the transition a test
                        # exercises is very often inside one of them — a test that
                        # proves a live-end revocation via a `roll_over_live_end()`
                        # helper says nothing about revocation in its own text, and
                        # crediting only the body would report it as untested.
                        # One level deep is enough: helpers in this suite do not
                        # call other helpers to reach a trigger.
                        direct_bodies.append(body)
                        for helper_name, helper_body in helper_bodies.get(rel, {}).items():
                            if re.search(r"\b" + re.escape(helper_name) + r"\b", body):
                                direct_bodies.append(helper_body)

        covered = bool(covered_rels) or covers_override is not None
        # Structural negative signal, at the granularity that makes it trustworthy:
        # only the bodies of the tests that actually DRIVE this card, plus the
        # local helpers they call.
        absence_in_direct = _fn_bodies_assert_absence(direct_bodies)
        depth, flags = infer_ability_depth(
            covering_texts, covered_rels, covering_fns, absence_in_direct
        )
        # Lifecycle dimension: only meaningful for the actions that MUTATE
        # persistent ability state, and only judgeable once the card is driven
        # at all — a card with no direct test is already reported as untested.
        lifecycle_missing = []
        lifecycle_evidence = {}
        if act in STATEFUL_REGISTRATION_ACTIONS and direct_fns:
            lifecycle_missing, lifecycle_evidence = missing_lifecycle_properties(
                eff, direct_bodies
            )
        if not covered:
            depth = "none"

        # jidou (自動) interaction flags
        watches_abilities = WATCHES_ABILITIES_MARKER in full_text
        effect_cause = any(m in full_text for m in EFFECT_CAUSE_MARKERS)

        # dedup
        covered_rels = sorted(set(covered_rels))
        covering_fns = sorted(set(covering_fns))
        direct_fns = sorted(set(direct_fns))

        # counters
        for t in triggers:
            trigger_counts[t][1] += 1
            if covered:
                trigger_counts[t][0] += 1
            matrix[t][act][1] += 1
            if covered:
                matrix[t][act][0] += 1
        action_counts[act][1] += 1
        if covered:
            action_counts[act][0] += 1
        cond_counts[cond][1] += 1
        if covered:
            cond_counts[cond][0] += 1
        set_counts[sset][1] += 1
        if covered:
            set_counts[sset][0] += 1
        depth_counts[depth] += 1

        # gap collection
        if not covered:
            untested.append((base, cards[0] if cards else "?", trig_raw, act, cond, full_text[:110]))

        rows.append({
            "idx": idx,
            "full_text": full_text,
            "triggerless_text": u.get("triggerless_text") or "",
            "triggers": trig_raw,
            "trigger_list": triggers,
            "action": act,
            "action_label": human_action(act),
            "condition": cond,
            "set": sset,
            "cards": cards,
            "card_bases": sorted(set(card_base(c) for c in cards)),
            "card_count": u.get("card_count"),
            "card_sample": cards[0] if cards else "",
            "base": base,
            "is_null": bool(u.get("is_null")),
            "use_limit": u.get("use_limit"),
            "watches_abilities": watches_abilities,
            "can_prompt": effect_can_prompt(u.get("effect")),
            "effect_cause": effect_cause,
            "jidou_partners": [],  # filled after the loop
            "covered": covered,
            "depth": depth,
            "has_assert": flags["has_assert"],
            "has_choice": flags["has_choice"],
            "has_negative": flags["has_negative"],
            "covering_files": covered_rels,
            "covering_tests": covering_fns[:30],
            "covering_test_count": len(covering_fns),
            "direct_tests": direct_fns[:30],
            "direct_test_count": len(direct_fns),
            "lifecycle_missing": lifecycle_missing,
            "lifecycle_evidence": lifecycle_evidence,
            "cost": u.get("cost"),
            "effect": eff,
        })

    # card-level stats
    card_covered = {}
    for r in rows:
        card_covered[r["base"]] = card_covered.get(r["base"], False) or r["covered"]
    total_cards = len(card_covered)
    covered_cards = sum(card_covered.values())
    abilities_on_covered = sum(1 for r in rows if r["covered"])

    # jidou (自動) multi-ability partners: same base card carries >=1 other ability
    idxs_by_base = defaultdict(set)
    for r in rows:
        idxs_by_base[r["base"]].add(r["idx"])
    for r in rows:
        if "自動" in r["trigger_list"]:
            r["jidou_partners"] = sorted(idxs_by_base[r["base"]] - {r["idx"]})

    def _is_thin(r):
        """Thin coverage: no choice-level exercise, or exercised by <= 1 test fn.

        An ability that resolves without ever raising a prompt cannot reach
        "L2+choice" no matter how good its tests are, so for those the choice
        half of the test is skipped — only a single-fn coverage still counts.
        Without this the list never empties and keeps re-flagging complete
        tests (it was doing exactly that to PL!SP-bp4-010-R's four-test file).
        """
        if r["covering_test_count"] <= 1:
            return True
        return "choice" not in r["depth"] and r["can_prompt"]

    specific_thin = sorted(
        r["idx"] for r in rows
        if (r["condition"] in SPECIAL_CONDITION_TYPES or r["use_limit"] or r["watches_abilities"])
        and _is_thin(r)
    )

    qa = build_qa_coverage(all_src)

    ret = {
        "abilities": rows,
        "stats": stats,
        "n_files": n_files,
        "n_tests": n_tests,
        "trigger_counts": dict(trigger_counts),
        "action_counts": dict(action_counts),
        "cond_counts": dict(cond_counts),
        "set_counts": dict(set_counts),
        "depth_counts": dict(depth_counts),
        "matrix": {k: dict(v) for k, v in matrix.items()},
        "untested": untested,
        "total_cards": total_cards,
        "covered_cards": covered_cards,
        "abilities_on_covered": abilities_on_covered,
        "specific_requirements_total": sum(
            1 for r in rows
            if r["condition"] in SPECIAL_CONDITION_TYPES or r["use_limit"] or r["watches_abilities"]
        ),
        "specific_requirements_thin": specific_thin,
        "lifecycle_gaps": {
            r["idx"]: r["lifecycle_missing"]
            for r in rows
            if r["lifecycle_missing"]
        },
        "lifecycle_total": sum(
            1
            for r in rows
            if r["action"] in STATEFUL_REGISTRATION_ACTIONS and r["direct_test_count"]
        ),
        "qa": qa,
        "stale_bases": stale_report_bases(rows),
        "all_src_len": len(all_src),
    }
    quality_smells, n_test_fns_parsed = audit_test_quality(files)
    # A `game.id("…")` literal that names no card at all. get_card_id's lenient
    # fallback turns such a typo into a DIFFERENT PRINT of the same card, so the
    # test stages the wrong card and passes.
    quality_smells["unresolvable_card_id"] = [
        (rel, "<literal>", line, lit)
        for rel, line, lit in unresolvable_card_literals(
            files, _card_numbers()
        )
    ]
    ret = dict(
        ret,
        quality={k: len(v) for k, v in quality_smells.items()},
        quality_rows={
            k: [
                {"file": rel, "test": name, "line": line, "detail": detail}
                for rel, name, line, detail in v
            ]
            for k, v in quality_smells.items()
        },
        n_test_fns_parsed=n_test_fns_parsed,
    )
    return ret


def render_coverage(inv):
    rows = inv["abilities"]
    trigger_counts = inv["trigger_counts"]
    action_counts = inv["action_counts"]
    cond_counts = inv["cond_counts"]
    set_counts = inv["set_counts"]
    depth_counts = inv["depth_counts"]
    untested = inv["untested"]
    n_files = inv["n_files"]
    n_tests = inv["n_tests"]
    total_cards = inv["total_cards"]
    covered_cards = inv["covered_cards"]
    abilities_on_covered = inv["abilities_on_covered"]
    n_abilities = len(rows)

    def pct(c, t):
        return f"{c}/{t}" if t else "0"

    lines = []
    w = lines.append
    w("# Test Coverage Report")
    w("")
    w(f"_Auto-generated by `cards/test_inventory.py` — do not edit by hand. Rerun `python cards/test_inventory.py` after changing the parser, card data, or tests._")
    w("")
    w(f"Covers the **real card database** (`cards/abilities.json`, {n_abilities} unique abilities) against the Rust suite (`engine/tests`, {n_files} test files, ~{n_tests} tests).")
    w("")
    w("## How to read this")
    w("")
    w("Three coverage levels — they answer different questions:")
    w("")
    w("| Level | Question | Meaning |")
    w("|---|---|---|")
    w("| **Card (L0)** | Is this card's `card_no` referenced in any test? | A test `game.id(\"PL!N-bp7-011-R+\")` shows up. Does NOT prove firing. |")
    w("| **Fires (L1/L2)** | Does a test assert the effect? | Inferred: `assert` in covering file → L1; negative-path file/test name → L2. |")
    w("| **Mechanic** | Is any test exercising this trigger / action / condition? | Even if a specific card is untested, a shared `action`+`condition` still covers the engine path. |")
    w("")
    w("`depth` is auto-inferred (see `cards/test_inventory.py:depth`); add `/// @covers PL!X-... depth=L2` to override. Details: `engine/tests/TEST_INVENTORY.json` / `.md`, matrix: `docs/ABILITY_MATRIX.md`.")
    w("")
    w("A card with multiple abilities counts as **covered** if *any* is touched; check gap list for finer detail.")
    w("")
    w("## Overall")
    w("")
    w(f"- **Unique abilities:** {n_abilities}")
    w(f"- **Distinct cards (base identity):** {total_cards}")
    w(f"- **Cards referenced in tests (L0):** {covered_cards} / {total_cards}  ({pct(covered_cards, total_cards)})")
    w(f"- **Abilities on a referenced card:** {abilities_on_covered} / {n_abilities}")
    depth_str = ", ".join(f"{k}: {v}" for k, v in sorted(depth_counts.items()))
    w(f"- **Depth (inferred):** {depth_str}")
    w("")

    def table(title, counts, order=None, fmt=lambda k: k):
        rows_sorted = sorted(counts.items(), key=lambda kv: (order.index(kv[0]) if order and kv[0] in order else 999, kv[0]))
        w(f"## {title}")
        w("")
        w("| Category | Covered | Total | % |")
        w("|---|---|---|---|")
        for k, (c, t) in rows_sorted:
            w(f"| {fmt(k)} | {c} | {t} | {pct(c,t)} |")
        w("")

    table("By trigger type", trigger_counts, TRIGGER_ORDER, lambda k: TRIGGER_LABEL.get(k, k))
    table("By effect action", action_counts, fmt=human_action)
    table("By condition type", cond_counts)
    table("By card set", set_counts)

    # depth breakdown
    w("## By inferred depth")
    w("")
    w("| Depth | Count |")
    w("|---|---|")
    for k in sorted(depth_counts.keys()):
        w(f"| {k} | {depth_counts[k]} |")
    w("")

    # untested gap
    untested_sorted = sorted(untested, key=lambda r: (r[0], r[2]))
    w("## Untested abilities (gap)")
    w("")
    w("These abilities' cards are **not referenced by any test** (L0 gap). Highest-value targets for new tests. Grouped by trigger type.")
    w("")
    if not untested_sorted:
        w("_None — every ability's card is referenced by at least one test._")
        w("")
    by_trig = defaultdict(list)
    for r in untested_sorted:
        by_trig[r[2]].append(r)
    for t in sorted(by_trig.keys()):
        grp = by_trig[t]
        w(f"### {t}  ({len(grp)})")
        w("")
        w("| Card | Set | Action | Condition | Text |")
        w("|---|---|---|---|---|")
        for base, card, _tr, act, cond, text in sorted(grp, key=lambda r: r[0]):
            safe = text.replace("|", "/").replace("\n", " ")
            w(f"| `{card}` | {card_set(card)} | {act} | {cond} | {safe} |")
        w("")

    # jidou (自動) interaction coverage
    def _ability_row(r):
        safe = r["full_text"].replace("|", "/").replace("\n", " ")
        return (
            f"| `{r['base']}` | {r['condition']} | {r['depth']} | "
            f"{r['direct_test_count']}/{r['covering_test_count']} | {safe[:110]} |"
        )

    jidou_rows = [r for r in rows if "自動" in r["trigger_list"]]
    watchers = [r for r in jidou_rows if r["watches_abilities"]]
    effect_cause = [r for r in jidou_rows if r["effect_cause"] and not r["watches_abilities"]]
    multi = [r for r in jidou_rows if r["jidou_partners"]]

    w("## Jidou (自動) interaction coverage")
    w("")
    w("自動 abilities rarely act alone — they chain off other abilities, off effect *causes*, or share a card with other abilities. These lists group every 自動 by interaction shape; `Depth`/`Tests` say how well the **interaction** is exercised.")
    w("")
    w("`Tests` is `direct/in file`: how many test functions reach this card in their own body, out of how many sit in a file that mentions it. A card is only as covered as the tests that actually drive it — the second number is co-location, which is navigation, not evidence.")
    w("")
    w("**What this column is not.** It counts a card, not an ability. 151 card identities carry more than one ability (the jidou plus its partner), and those abilities share one set of direct tests, so this number cannot say \"ability A has 3 tests, ability B has 5\". Splitting them was measured and rejected: 128 of those 151 differ by trigger, but only 59 of those can be separated by the explicit `AbilityTrigger::X` a test fires (自動 is driven through `trigger_auto_abilities_for_player`, which names no constant, and 常時 has none). A metric right for 46% of cases would read as complete, which is how this report's earlier 100% misled the test-improvement plan. Treat these rows as per-card, not per-ability.")
    w("")
    w(f"### A. Jidou watching other abilities resolve (`能力が解決`)  ({len(watchers)})")
    w("")
    w("Highest-risk category: requires a full live phase with another ability resolving on the same stage.")
    w("")
    if watchers:
        w("| Card | Condition | Depth | Tests | Text |")
        w("|---|---|---|---|---|")
        for r in sorted(watchers, key=lambda r: (r["direct_test_count"], r["idx"])):
            w(_ability_row(r))
    else:
        w("_None._")
    w("")
    w(f"### B. Jidou gated on effect causes (`効果によって` / `対戦相手の効果でも発動する`)  ({len(effect_cause)})")
    w("")
    if effect_cause:
        w("| Card | Condition | Depth | Tests | Text |")
        w("|---|---|---|---|---|")
        for r in sorted(effect_cause, key=lambda r: (r["direct_test_count"], r["idx"])):
            w(_ability_row(r))
    else:
        w("_None._")
    w("")
    w(f"### C. Cards pairing a jidou with another ability  ({len(multi)})")
    w("")
    w("`#partners` lists the other abilities on the same card (inventory idx:trigger) — combo behavior needs a test driving BOTH, not each in isolation. Sorted by DIRECT tests, so the thinnest are at the top rather than buried.")
    w("")
    if multi:
        w("| Card | Partners | Depth | Tests |")
        w("|---|---|---|---|")
        for r in sorted(multi, key=lambda r: (r["direct_test_count"], r["idx"])):
            partners = ", ".join(
                f"#{j}:{(rows[j]['trigger_list'][0] if rows[j]['trigger_list'] else '?')}"
                for j in r["jidou_partners"][:4]
            )
            w(
                f"| `{r['base']}` | {partners} | {r['depth']} | "
                f"{r['direct_test_count']}/{r['covering_test_count']} |"
            )
    else:
        w("_None._")
    w("")

    # specific-requirement abilities needing more than an L0/L1 glance
    specific_total = inv["specific_requirements_total"]
    thin_idxs = set(inv["specific_requirements_thin"])
    specific_thin_rows = [
        r for r in rows
        if r["idx"] in thin_idxs
        and (r["condition"] in SPECIAL_CONDITION_TYPES or r["use_limit"] or r["watches_abilities"])
    ]
    w("## Specific-requirement abilities — thin coverage")
    w("")
    w(
        f"{specific_total} abilities carry a rare condition type (state/position/energy/highest-cost/"
        "blade-compare/ability-filter), a use_limit, or watch other abilities resolve. Listed below are "
        "the ones with **thin** coverage — either no choice-level exercise, or exercised by ≤1 test fn. "
        "An ability that resolves without ever raising a prompt is exempt from the choice half of that "
        "test (it could never reach `L2+choice`), so a fully-tested self-resolving effect does not appear "
        "here."
    )
    w("")
    if specific_thin_rows:
        w("| Card | Why specific | Condition | Depth | Tests | Text |")
        w("|---|---|---|---|---|---|")
        for r in sorted(specific_thin_rows, key=lambda r: (r["condition"], r["idx"])):
            why = []
            if r["watches_abilities"]:
                why.append("watches abilities")
            if r["use_limit"]:
                why.append(f"use_limit={r['use_limit']}")
            if r["condition"] in SPECIAL_CONDITION_TYPES:
                why.append(r["condition"])
            safe = r["full_text"].replace("|", "/").replace("\n", " ")
            w(
                f"| `{r['base']}` | {'+'.join(why)} | {r['condition']} | {r['depth']} | "
                f"{r['covering_test_count']} | {safe[:100]} |"
            )
    else:
        w("_None — all specific-requirement abilities have choice-level coverage from ≥2 test fns._")
    w("")
    w(
        "That result is about *what* an ability does, not *how long* a grant it creates survives — "
        "see the lifecycle table below, which is a separate axis and was not satisfied by any of it."
    )
    w("")

    # ------------------------------------------------------------------
    # Lifecycle-constrained abilities — the axis `depth` cannot see.
    # ------------------------------------------------------------------
    lifecycle_rows = [r for r in rows if r["lifecycle_missing"]]
    lifecycle_ok = inv["lifecycle_total"] - len(lifecycle_rows)
    w("## Lifecycle-constrained abilities — grant survival untested")
    w("")
    w(
        f"The `gain_ability` / `gain_ability_from_source` / `invalidate_ability` / "
        f"`suppress_ability_trigger` actions ({inv['lifecycle_total']} abilities with ≥1 direct test) "
        "do not resolve into the game state and stop. They MUTATE persistent ability state that "
        "outlives the resolution and is re-derived by a later scan, so a test that drives them once "
        "cannot see how the grant behaves the second time."
    )
    w("")
    w("Three transitions, each of which shipped as a real defect while the depth ladder above read `L2+choice`:")
    w("")
    w("- **idempotence** — the re-derivation must REPLACE, not append. `gain_ability_from_source` is a 常時, so it re-copies on every `recalculate_constants`; appending gives the host N copies after N scans, and each copy takes a distinct `GAINED_ABILITY_INDEX_BASE + gained_idx`, so the `(card_id, ability_index)` dedup in `trigger_live_success_abilities` cannot collapse them and the copied ライブ成功時 fires N times.")
    w("- **revocation** — the grant must be WITHDRAWN when its precondition is destroyed. Otherwise the host keeps an ability whose printed source is gone, and the text table and the `Ability` table disagree about what the card has.")
    w("- **surface** — a granted 起動 must reach the generated action list. It is executable via `find_gained_activation` regardless, so a grant missing from the action list is dead to a player and to the bot while every state-table assertion still passes. Scoped to grants that actually produce an activation: a granted 常時 or ライブ成功時 is consumed by a constant scan or a trigger dispatch and has no action-list representation, so it is exempt. In the current card set that leaves exactly one ability constrained here — and it is the one whose grant was executable but never offered.")
    w("")
    w(
        "Evidence is unioned across an ability's direct test fns and reaches one level "
        "into the local helpers a test calls — this suite is written with well-factored "
        "fixture helpers, so a transition a test exercises is often inside one of them."
    )
    w("")
    w(
        "This is a heuristic over test SOURCE, not a proof that a property holds. "
        "It credits a property when some direct test carries a signal for it, and "
        "shows that signal, so a specific call can be disagreed with. Each property "
        "accepts several honest shapes because one does not cover the family: a "
        "constant scan repeated, a resolution repeated (\"a second resolution does not "
        "grow the total\"), a zone exit or duration rollover, an assertion that the "
        "registration is gone. It will still miss a property proven some other way."
    )
    w("")
    if lifecycle_rows:
        w("| Card | Action | Depth | Tests | Missing | Evidence for the rest |")
        w("|---|---|---|---|---|---|")
        for r in sorted(
            lifecycle_rows,
            key=lambda r: (len(r["lifecycle_missing"]), r["direct_test_count"], r["idx"]),
            reverse=True,
        ):
            evidence = "; ".join(
                f"{prop}: {', '.join(sigs)}"
                for prop, sigs in sorted(r["lifecycle_evidence"].items())
                if sigs
            ) or "_none_"
            w(
                f"| `{r['base']}` | {r['action']} | {r['depth']} | "
                f"{r['direct_test_count']}/{r['covering_test_count']} | "
                f"{', '.join(r['lifecycle_missing'])} | {evidence} |"
            )
        w("")
        w(
            f"**{len(lifecycle_rows)} of {inv['lifecycle_total']}** lifecycle-constrained abilities have "
            f"at least one transition with no test reaching it; {lifecycle_ok} are fully covered."
        )
    else:
        w(
            f"_None — all {inv['lifecycle_total']} lifecycle-constrained abilities have a test "
            "reaching idempotence, revocation, and (for the `gain_ability*` pair) the offer surface._"
        )
    w("")

    w("## Official QA rulings (cards/qa_data.json)")
    w("")
    qa = inv["qa"]
    if qa["total"]:
        w(f"- **Rulings covered by tests:** {qa['covered']} / {qa['total']} ({100 * qa['covered'] // max(qa['total'], 1)}%)")
        uncovered = [r for r in qa["rows"] if not r["covered"]]
        if uncovered:
            w("")
            w("Uncovered rulings (no test references the Q-id or any related card):")
            w("")
            w("| Ruling | Related card |")
            w("|---|---|")
            for r in uncovered:
                card = r["cards"][0] if r["cards"] else "?"
                w(f"| {r['id']} | `{card}` |")
        w("")
    else:
        w("_qa_data.json not found._")
        w("")

    w("## Inventory")
    w("")
    w(f"- Full per-ability index: [`engine/tests/TEST_INVENTORY.md`](TEST_INVENTORY.md) / [`TEST_INVENTORY.json`](TEST_INVENTORY.json)")
    w(f"- Trigger×action matrix: [`docs/ABILITY_MATRIX.md`](../../docs/ABILITY_MATRIX.md)")
    w(f"- Regenerate: `python cards/test_inventory.py`  •  CI check: `python cards/test_inventory.py --check`")
    w("")

    return "\n".join(lines) + "\n"


def render_matrix(inv):
    trigger_counts = inv["trigger_counts"]
    matrix = inv["matrix"]
    action_counts = inv["action_counts"]
    cond_counts = inv["cond_counts"]
    set_counts = inv["set_counts"]

    def pct(c, t):
        return f"{c}/{t}"

    all_actions = sorted(set(a for m in matrix.values() for a in m.keys()))
    triggers = TRIGGER_ORDER + [t for t in sorted(matrix.keys()) if t not in TRIGGER_ORDER]

    lines = []
    w = lines.append
    w("# Ability Matrix (trigger × action)")
    w("")
    w("_Auto-generated by `cards/test_inventory.py` — do not edit. `python cards/test_inventory.py` to refresh._")
    w("")
    w(f"Source: `cards/abilities.json` ({len(inv['abilities'])} unique abilities) vs `engine/tests` ({inv['n_files']} files, ~{inv['n_tests']} tests).")
    w("")
    w("## Trigger × Action — covered / total")
    w("")
    # header
    w("| Trigger \\ Action | " + " | ".join(human_action(a) for a in all_actions) + " |")
    w("|---" + "|---" * len(all_actions) + "|")
    for t in triggers:
        if t not in matrix:
            continue
        cells = []
        for a in all_actions:
            c, tot = matrix[t].get(a, [0, 0])
            if tot == 0:
                cells.append("·")
            else:
                cells.append(pct(c, tot))
        label = TRIGGER_LABEL.get(t, t if t != "(none)" else "(no trigger/is_null)")
        w(f"| {label} | " + " | ".join(cells) + " |")
    w("")
    w("> Cell `1/2` = 1 of 2 abilities with that trigger+action are L0-covered. `·` = no such ability.")
    w("")

    # condition heatmap
    w("## By condition type — covered / total")
    w("")
    w("| Condition | Covered | Total | % |")
    w("|---|---|---|---|")
    for k, (c, t) in sorted(cond_counts.items(), key=lambda kv: kv[0]):
        w(f"| {k} | {c} | {t} | {pct(c,t)} |")
    w("")

    w("## By card set — covered / total")
    w("")
    w("| Set | Covered | Total | % |")
    w("|---|---|---|---|")
    for k, (c, t) in sorted(set_counts.items()):
        w(f"| {k} | {c} | {t} | {pct(c,t)} |")
    w("")

    w("## By trigger summary")
    w("")
    w("| Trigger | Covered | Total | % |")
    w("|---|---|---|---|")
    for k in triggers:
        if k in trigger_counts:
            c, t = trigger_counts[k]
            w(f"| {TRIGGER_LABEL.get(k,k)} | {c} | {t} | {pct(c,t)} |")
    w("")

    w("## By action summary")
    w("")
    w("| Action | Covered | Total | % |")
    w("|---|---|---|---|")
    for k, (c, t) in sorted(action_counts.items()):
        w(f"| {human_action(k)} | {c} | {t} | {pct(c,t)} |")
    w("")

    w("## Gaps to prioritize")
    w("")
    # top uncovered trigger+action combos
    gaps = []
    for t, amap in matrix.items():
        for a, (c, tot) in amap.items():
            if tot and c < tot:
                gaps.append((tot - c, t, a, c, tot))
    gaps.sort(reverse=True)
    w("| Uncovered | Trigger | Action | Covered/Total |")
    w("|---|---|---|---|---|")
    for gap, t, a, c, tot in gaps[:25]:
        w(f"| {gap} | {TRIGGER_LABEL.get(t,t)} | {human_action(a)} | {c}/{tot} |")
    w("")

    return "\n".join(lines) + "\n"


def render_inventory_md(inv):
    lines = []
    w = lines.append
    w("# Test Inventory — per-ability index")
    w("")
    w("_Auto-generated by `cards/test_inventory.py` — do not edit. `python cards/test_inventory.py` to refresh._")
    w("")
    w(f"Source: `cards/abilities.json` ({len(inv['abilities'])} unique abilities) vs `engine/tests` ({inv['n_files']} files, ~{inv['n_tests']} tests).")
    w("")
    w("Columns: **Depth** = inferred L0 (referenced) / L1 (asserts) / L2 (negative) / +choice if `has_pending_choice` etc. Override with `/// @covers PL!… depth=L2`.")
    w("")
    w("| # | Triggers | Action | Condition | Depth | Tests | Sample card | Covering files |")
    w("|---|---|---|---|---|---|---|---|")
    for r in inv["abilities"]:
        idx = r["idx"]
        trig = r["triggers"] or "(none)"
        act = r["action"]
        cond = r["condition"]
        depth = r["depth"]
        tc = r["covering_test_count"]
        sample = r["card_sample"]
        files = "<br>".join(r["covering_files"][:3])
        if len(r["covering_files"]) > 3:
            files += f"<br>+{len(r['covering_files'])-3} more"
        # escape pipes
        trig = trig.replace("|", "/")
        w(f"| {idx} | {trig} | {act} | {cond} | {depth} | {tc} | `{sample}` | {files} |")
    w("")
    w("`TEST_INVENTORY.json` has full `covering_tests[]` and `covering_files[]` per ability.")
    w("")
    return "\n".join(lines) + "\n"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true", help="fail if generated files are stale")
    ap.add_argument(
        "--check-json",
        action="store_true",
        help="with --check: also verify TEST_INVENTORY.json (excluded by default — it is 1.7 MB and dominates diff churn)",
    )
    ap.add_argument("--json-only", action="store_true", help="only write JSON")
    ap.add_argument(
        "--families",
        action="store_true",
        help="also write docs/ABILITY_FAMILIES.md (behavioral grouping index)",
    )
    ap.add_argument(
        "--update-softguard-baseline",
        action="store_true",
        help="re-record the soft-guard ratchet baseline (P0.3)",
    )
    args = ap.parse_args()

    if args.update_softguard_baseline:
        write_soft_guard_baseline()
        return 0

    inv = build_inventory()

    if args.families:
        families = build_families(inv)
        OUT_FAMILIES = ROOT / "docs" / "ABILITY_FAMILIES.md"
        OUT_FAMILIES.write_text(render_families(inv, families), encoding="utf-8")
        print(f"Wrote {OUT_FAMILIES.relative_to(ROOT)} ({len(families)} families)")
        for fam in families[:15]:
            print(f"  {fam['total']:4} abilities  {len(fam['variants']):3} variants  {fam['name']}")
        return 0

    # render
    coverage_text = render_coverage(inv)
    matrix_text = render_matrix(inv)
    inventory_md_text = render_inventory_md(inv)
    quality_text = render_quality(
        {
            k: [
                (r["file"], r["test"], r["line"], r["detail"])
                for r in v
            ]
            for k, v in inv["quality_rows"].items()
        },
        inv,
    )
    # JSON: strip heavy effect/cost for size but keep essentials
    json_rows = []
    for r in inv["abilities"]:
        json_rows.append({
            "idx": r["idx"],
            "full_text": r["full_text"],
            "triggers": r["triggers"],
            "trigger_list": r["trigger_list"],
            "action": r["action"],
            "condition": r["condition"],
            "set": r["set"],
            "cards": r["cards"],
            "base": r["base"],
            "covered": r["covered"],
            "depth": r["depth"],
            "has_assert": r["has_assert"],
            "has_choice": r["has_choice"],
            "has_negative": r["has_negative"],
            "covering_files": r["covering_files"],
            "covering_tests": r["covering_tests"],
            "covering_test_count": r["covering_test_count"],
            "direct_tests": r["direct_tests"],
            "direct_test_count": r["direct_test_count"],
            "is_null": r["is_null"],
            "use_limit": r["use_limit"],
            "watches_abilities": r["watches_abilities"],
            "effect_cause": r["effect_cause"],
            "jidou_partners": r["jidou_partners"],
        })
    json_quality = {
        k: {"count": len(v), "rows": v}
        for k, v in inv["quality_rows"].items()
    }

    json_text = json.dumps({
        "generated_by": "cards/test_inventory.py",
        "generated_at": __import__("datetime").datetime.now(__import__("datetime").timezone.utc).isoformat(),
        "stats": inv["stats"],
        "n_files": inv["n_files"],
        "n_tests": inv["n_tests"],
        "total_cards": inv["total_cards"],
        "covered_cards": inv["covered_cards"],
        "abilities_on_covered": inv["abilities_on_covered"],
        "trigger_counts": inv["trigger_counts"],
        "action_counts": inv["action_counts"],
        "cond_counts": inv["cond_counts"],
        "set_counts": inv["set_counts"],
        "depth_counts": inv["depth_counts"],
        "qa_rulings": {
            "covered": inv["qa"]["covered"],
            "total": inv["qa"]["total"],
            "uncovered_ids": [r["id"] for r in inv["qa"]["rows"] if not r["covered"]],
        },
        "jidou_conjunction": {
            "ability_watchers": [r["idx"] for r in inv["abilities"] if r["watches_abilities"]],
            "effect_cause": [
                r["idx"] for r in inv["abilities"]
                if r["effect_cause"] and not r["watches_abilities"]
            ],
            "multi_ability_cards": [
                r["idx"] for r in inv["abilities"]
                if "自動" in r["trigger_list"] and r["jidou_partners"]
            ],
        },
        "specific_requirements": {
            "total": inv["specific_requirements_total"],
            "thin_idxs": inv["specific_requirements_thin"],
        },
        "lifecycle": {
            "total": inv["lifecycle_total"],
            "gap_idxs": inv["lifecycle_gaps"],
        },
        "quality": json_quality,
        "n_test_fns_parsed": inv["n_test_fns_parsed"],
        "abilities": json_rows,
    }, ensure_ascii=False, indent=2) + "\n"

    if args.check:
        def strip_generated_at(text):
            # ignore volatile timestamp for check
            return re.sub(r'"generated_at":\s*"[^"]*"', '"generated_at": "CHECK"', text)
        ok = True
        checked = [(OUT_COVERAGE, coverage_text), (OUT_MATRIX, matrix_text), (OUT_MD, inventory_md_text), (OUT_QUALITY, quality_text)]
        if args.check_json:
            checked.append((OUT_JSON, json_text))
        for path, new_text in checked:
            if not path.exists():
                print(f"CHECK FAIL: {path.relative_to(ROOT)} missing", file=sys.stderr)
                ok = False
                continue
            old = path.read_text(encoding="utf-8")
            if strip_generated_at(old) != strip_generated_at(new_text):
                print(f"CHECK FAIL: {path.relative_to(ROOT)} is stale — run `python cards/test_inventory.py`", file=sys.stderr)
                # show full diff (no truncation per AGENTS.md — capture all so failures are diagnosable)
                import difflib
                diff = list(difflib.unified_diff(old.splitlines(), new_text.splitlines(), fromfile=str(path.relative_to(ROOT)), tofile="new", lineterm=""))
                for line in diff:
                    print(line, file=sys.stderr)
                if not diff:
                    print("  (no textual diff after ignoring generated_at — check encoding/line-endings)", file=sys.stderr)
                else:
                    print(f"  --- {len(diff)} diff lines total ---", file=sys.stderr)
                    print(f"  Run `python cards/test_inventory.py` and inspect `git diff {path.relative_to(ROOT)}` for full context.", file=sys.stderr)
                ok = False
        for violation in check_soft_guard_ratchet():
            print(f"SOFT GUARD RATCHET FAIL: {violation}", file=sys.stderr)
            ok = False
        sys.exit(0 if ok else 1)

    if not args.json_only:
        OUT_COVERAGE.parent.mkdir(parents=True, exist_ok=True)
        OUT_COVERAGE.write_text(coverage_text, encoding="utf-8")
        print(f"Wrote {OUT_COVERAGE.relative_to(ROOT)} ({len(coverage_text)} bytes)")
        OUT_QUALITY.parent.mkdir(parents=True, exist_ok=True)
        OUT_QUALITY.write_text(quality_text, encoding="utf-8")
        print(f"Wrote {OUT_QUALITY.relative_to(ROOT)} ({len(quality_text)} bytes)")
        OUT_MATRIX.parent.mkdir(parents=True, exist_ok=True)
        OUT_MATRIX.write_text(matrix_text, encoding="utf-8")
        print(f"Wrote {OUT_MATRIX.relative_to(ROOT)} ({len(matrix_text)} bytes)")
        OUT_MD.parent.mkdir(parents=True, exist_ok=True)
        OUT_MD.write_text(inventory_md_text, encoding="utf-8")
        print(f"Wrote {OUT_MD.relative_to(ROOT)} ({len(inventory_md_text)} bytes)")

    OUT_JSON.parent.mkdir(parents=True, exist_ok=True)
    OUT_JSON.write_text(json_text, encoding="utf-8")
    print(f"Wrote {OUT_JSON.relative_to(ROOT)} ({len(json_text)} bytes)")

    # summary
    print(f"abilities={len(inv['abilities'])} cards={inv['total_cards']} covered_cards={inv['covered_cards']} ({inv['covered_cards']}/{inv['total_cards']}) n_tests~{inv['n_tests']}")
    print(f"depth: {dict(inv['depth_counts'])}")
    print(f"jidou interaction: watchers+cause+multi = {sum(1 for r in inv['abilities'] if r['watches_abilities'])}+{sum(1 for r in inv['abilities'] if r['effect_cause'])}+{sum(1 for r in inv['abilities'] if '自動' in r['trigger_list'] and r['jidou_partners'])}; specific-requirement thin: {len(inv['specific_requirements_thin'])}/{inv['specific_requirements_total']}")
    print(f"lifecycle gaps: {len(inv['lifecycle_gaps'])}/{inv['lifecycle_total']} stateful-registration abilities missing a transition")
    stale = inv["stale_bases"]
    if stale:
        print(f"REPORT INTEGRITY: {len(stale)} ability rows name a card that is not in the pool: {stale[:6]}")
    else:
        print("report integrity: every ability row names a card in the pool")
    return 0


if __name__ == "__main__":
    sys.exit(main())
