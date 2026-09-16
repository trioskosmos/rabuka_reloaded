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
    python cards/test_inventory.py --check  # CI: fail if stale
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
        # With tests_only the pattern itself implies #[test]; otherwise look
        # back for the attribute (the match starts at `fn`, after it).
        is_test = tests_only or bool(
            re.search(r"#\[test\]", text[max(0, m.start() - 160) : m.start()])
        )
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
        "similar_cards": [],
    }
    for _p, rel, text, _fns in files:
        fns = split_test_fns(text)
        # Helper profiles: a test calling a helper that asserts/drives gets
        # credit (e.g. check_heart_reduction asserts, play_three drives).
        # Helpers are file-local non-#[test] fns.
        helper_assert = set()
        helper_scan = set()
        helper_ident = set()
        helper_outcome = set()
        helper_bodies = {}
        for hname, hbody, _hline, is_test in split_rust_fns(text, False):
            if is_test:
                continue
            helper_bodies[hname] = hbody
            if Q_ASSERT_RE.search(hbody):
                helper_assert.add(hname)
            if Q_SCAN_RE.search(hbody):
                helper_scan.add(hname)
            if Q_IDENT_RE.search(hbody):
                helper_ident.add(hname)
            if Q_OUTCOME_RE.search(hbody) or Q_OUTCOME_MOD_RE.search(hbody):
                helper_outcome.add(hname)
        # Transitive closure: a helper calling a credited helper inherits the
        # credit (e.g. setup_and_trigger_live_start -> advance_to_live_start).
        def _close(credited):
            changed = True
            while changed:
                changed = False
                for hname, hbody in helper_bodies.items():
                    if hname in credited:
                        continue
                    if any(
                        re.search(r"\b" + c + r"\s*\(", hbody)
                        for c in credited
                    ):
                        credited.add(hname)
                        changed = True
            return credited

        helper_assert = _close(set(helper_assert))
        helper_scan = _close(set(helper_scan))
        helper_ident = _close(set(helper_ident))
        helper_outcome = _close(set(helper_outcome))

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
        file_groups = {}
        for name, body, line in fns:
            has_assert = bool(Q_ASSERT_RE.search(body)) or bool(
                helper_assert_re and helper_assert_re.search(body)
            )
            if not has_assert:
                smells["no_assert"].append((rel, name, line, ""))
            driven = bool(Q_SCAN_RE.search(body)) or bool(
                helper_scan_re and helper_scan_re.search(body)
            )
            if (
                Q_SETUP_RE.search(body)
                and has_assert
                and not driven
                and Q_TRIGGER_CTX_RE.search(body)
                and not Q_PARSE_AUDIT_RE.search(body)
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
                smells["similar_cards"].append((rel, "<file>", 1, ", ".join(sorted(nos))))
    for rows in smells.values():
        rows.sort()
    n_fns = sum(len(split_test_fns(text)) for _p, _rel, text, _fns in files)
    return smells, n_fns


SMELL_DOCS = {
    "no_assert": "test never asserts (smoke at best — cannot pin behavior)",
    "no_drive": "trigger-context test that mutates state and asserts but never drives the engine (no scan/activate/play/fire): vacuous negatives/positives",
    "synthetic_only": "trigger event hand-pushed with no real ability resolution in the same fn (weaker trigger evidence)",
    "pendency_only": "has_pending_choice asserted without choice identity (pending_choice_type/summary/answer) or outcome asserts",
    "similar_cards": "confusable card numbers (bp2 vs pb2) staged in one file — human review for wrong-card staging",
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


def infer_ability_depth(covering_texts, covering_rels, covering_fns):
    """Return depth label and flags for an ability. Negative = file/test name hint only."""
    if not covering_texts:
        return "none", {"has_assert": False, "has_choice": False, "has_negative": False}
    has_assert = any("assert" in t for t in covering_texts)
    has_choice = any(bool(CHOICE_RE.search(t)) for t in covering_texts)
    has_negative = any(bool(NEGATIVE_RE.search(r)) for r in covering_rels) or any(bool(NEGATIVE_RE.search(fn)) for fn in covering_fns)
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

        covered = bool(covered_rels) or covers_override is not None
        depth, flags = infer_ability_depth(covering_texts, covered_rels, covering_fns)
        if not covered:
            depth = "none"

        # jidou (自動) interaction flags
        watches_abilities = WATCHES_ABILITIES_MARKER in full_text
        effect_cause = any(m in full_text for m in EFFECT_CAUSE_MARKERS)

        # dedup
        covered_rels = sorted(set(covered_rels))
        covering_fns = sorted(set(covering_fns))

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
        """Thin coverage: no choice-level exercise or exercised by <= 1 test fn."""
        return ("choice" not in r["depth"]) or r["covering_test_count"] <= 1

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
        "qa": qa,
        "all_src_len": len(all_src),
    }
    quality_smells, n_test_fns_parsed = audit_test_quality(files)
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
            f"{r['covering_test_count']} | {safe[:110]} |"
        )

    jidou_rows = [r for r in rows if "自動" in r["trigger_list"]]
    watchers = [r for r in jidou_rows if r["watches_abilities"]]
    effect_cause = [r for r in jidou_rows if r["effect_cause"] and not r["watches_abilities"]]
    multi = [r for r in jidou_rows if r["jidou_partners"]]

    w("## Jidou (自動) interaction coverage")
    w("")
    w("自動 abilities rarely act alone — they chain off other abilities, off effect *causes*, or share a card with other abilities. These lists group every 自動 by interaction shape; `Depth`/`Tests` say how well the **interaction** is exercised.")
    w("")
    w(f"### A. Jidou watching other abilities resolve (`能力が解決`)  ({len(watchers)})")
    w("")
    w("Highest-risk category: requires a full live phase with another ability resolving on the same stage.")
    w("")
    if watchers:
        w("| Card | Condition | Depth | Tests | Text |")
        w("|---|---|---|---|---|")
        for r in sorted(watchers, key=lambda r: (r["covering_test_count"], r["idx"])):
            w(_ability_row(r))
    else:
        w("_None._")
    w("")
    w(f"### B. Jidou gated on effect causes (`効果によって` / `対戦相手の効果でも発動する`)  ({len(effect_cause)})")
    w("")
    if effect_cause:
        w("| Card | Condition | Depth | Tests | Text |")
        w("|---|---|---|---|---|")
        for r in sorted(effect_cause, key=lambda r: (r["covering_test_count"], r["idx"])):
            w(_ability_row(r))
    else:
        w("_None._")
    w("")
    w(f"### C. Cards pairing a jidou with another ability  ({len(multi)})")
    w("")
    w("`#partners` lists the other abilities on the same card (inventory idx:trigger) — combo behavior needs a test driving BOTH, not each in isolation.")
    w("")
    if multi:
        w("| Card | Partners | Depth | Tests |")
        w("|---|---|---|---|")
        for r in sorted(multi, key=lambda r: (r["covering_test_count"], r["idx"])):
            partners = ", ".join(
                f"#{j}:{(rows[j]['trigger_list'][0] if rows[j]['trigger_list'] else '?')}"
                for j in r["jidou_partners"][:4]
            )
            w(f"| `{r['base']}` | {partners} | {r['depth']} | {r['covering_test_count']} |")
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
        "the ones with **thin** coverage (no choice-level exercise, or exercised by ≤1 test fn) — these "
        "are today's highest-value new-test targets now that the L0 gap list is empty."
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
    w(f"- Trigger×action matrix: [`docs/ABILITY_MATRIX.md`](../docs/ABILITY_MATRIX.md)")
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
        "quality": json_quality,
        "n_test_fns_parsed": inv["n_test_fns_parsed"],
        "abilities": json_rows,
    }, ensure_ascii=False, indent=2) + "\n"

    if args.check:
        def strip_generated_at(text):
            # ignore volatile timestamp for check
            return re.sub(r'"generated_at":\s*"[^"]*"', '"generated_at": "CHECK"', text)
        ok = True
        for path, new_text in [(OUT_COVERAGE, coverage_text), (OUT_MATRIX, matrix_text), (OUT_MD, inventory_md_text), (OUT_QUALITY, quality_text), (OUT_JSON, json_text)]:
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
    return 0


if __name__ == "__main__":
    sys.exit(main())
