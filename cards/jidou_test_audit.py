#!/usr/bin/env python
"""Jidou test-quality sweeps — the mechanical detectors behind
`docs/JIDOU_COMBINATION_WORK.md` §7.

Per-card coverage counts (`test_inventory.py`) cannot see a test that *exists but
does not check the thing it is named for*. These three sweeps find that shape.
Run from the repo root:

    python cards/jidou_test_audit.py            # all sweeps
    python cards/jidou_test_audit.py --sweep a  # just one

Exit code is 0 even when candidates are found — these are review candidates, not
build failures. Read each hit before changing anything: several are legitimate
(poked-flag tests paired with a real-pipeline test elsewhere).
"""

import argparse
import io
import json
import os
import re
import sys

sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8")

ROOT = "engine/tests/test_modules"
JIDOU = os.path.join(ROOT, "jidou")

# A test name that commits to a count or a limit. If the body has <= 1
# assertion, it is a candidate for asserting nothing about that count.
COUNT_NAME = re.compile(
    r"(twice|second|2回|turn2|budget|ceiling|refus|blocked|per_turn|"
    r"stacks|repeat|both_|once_per|limit)", re.I)

# An assertion that accepts more than one value. True for every outcome.
# Anchored to an assert! call so a test *named* "..._no_crash" or "..._no_draw"
# (a perfectly good name) is not reported.
#
# IMPORTANT: the disjunction must be over the SAME variable. `x == 1 || x == 2`
# is vacuous (any value), but `a == 2 || b == 2` legitimately expresses "either
# of these two may be selected" and is a good assertion. Only the former is
# reported here.
LENIENT = [
    (re.compile(r"assert!\(\s*(\w+)\s*==\s*\d+\s*\|\|\s*\1\s*==\s*\d+"), "or-equal-self"),
    (re.compile(r"assert!\(\s*(\w+)\s*>=\s*\d+\s*&&\s*\1\s*<=\s*\d+"), "range-self"),
    (re.compile(r"assert!\(\s*(\w+)\s*<=\s*\d+\s*,"), "upper-bound-only"),
    (re.compile(r"assert!\([^)]*==\s*1\s*\|\|"), "one-or-two"),
    (re.compile(r"assert!\([^)]*important thing is no crash", re.I), "assert-no-crash"),
    (re.compile(r"for simplicity, just verify", re.I), "gave-up"),
]

# A jidou shape the re-scan-guard leak silently under-fired: can fire more than
# once per turn AND its effect is not a card movement.
NONSELF_MOVEMENT_EFFECT = ("recover_to_hand", "draw_card", "gain_resource")


def read(p):
    return open(p, encoding="utf-8", errors="replace").read()


def code_only(src):
    """Strip // and /* */ comments.

    Without this, a doc comment that QUOTES a bad assertion (e.g. the rewrite of
    `turn_limited_yell_second_activation_test.rs`, which documents the `1 or 2`
    it replaced) is reported as if the bad assertion were still there.
    """
    src = re.sub(r"/\*.*?\*/", "", src, flags=re.S)
    return re.sub(r"//[^\n]*", "", src)


# A call that drives one real, distinct triggering event. Two of these in a
# single #[test] body (or inside a loop) is the "second firing" evidence Sweep B
# needs: one batch asserted twice is not a repeat.
DRIVERS = [
    "play_to_stage", "try_play_to_stage", "activate_ability",
    "replace_member_by_baton_touch", "set_recently_moved_cards",
    "record_card_movement", "push_movement_event", "trigger_auto_abilities_for_player",
]

_TEST_FILES = None


def all_test_files():
    global _TEST_FILES
    if _TEST_FILES is None:
        _TEST_FILES = []
        for dp, _dn, fn in os.walk(ROOT):
            for f in fn:
                if f.endswith(".rs"):
                    _TEST_FILES.append(os.path.join(dp, f))
    return _TEST_FILES


def _base_card_no(card_no):
    """Strip the print/rarity suffix: `PL!N-bp5-005-R＋` -> `PL!N-bp5-005`.

    Tests routinely reference a DIFFERENT print of the same card than the one
    abilities.json lists first (`PL!N-bp5-005-P` vs `-R＋`). Matching the exact
    string reports those as untested, which is a false positive — match the base
    so every print of a card counts as coverage.
    """
    return re.sub(r"-(?:R|P|AR|SEC|SRL|SR|N|P2)?[+＋]?$", "", card_no.strip())


def two_event_coverage(card_no):
    """Largest number of trigger-driving calls in any single #[test] that names
    `card_no` (or any print of it). Returns (count, filename). 0 when the card
    is not tested at all.
    """
    best, where = 0, ""
    # The base is a prefix of the full card_no, so this one pattern also matches
    # the exact print.
    base_pat = re.compile(re.escape(_base_card_no(card_no)))
    for p in all_test_files():
        src = read(p)
        if not base_pat.search(src):
            continue
        for m in re.finditer(r"#\[test\](.*?)(?=\n#\[test\]|\Z)", src, re.S):
            n = sum(len(re.findall(r"\b" + re.escape(d) + r"\b", m.group(1)))
                    for d in DRIVERS)
            if n > best:
                best, where = n, os.path.basename(p)
    return best, where


def test_files(subdir):
    base = os.path.join(ROOT, subdir) if subdir else ROOT
    for dp, _dn, fn in os.walk(base):
        for f in fn:
            if f.endswith(".rs"):
                yield os.path.join(dp, f)


# A test name that makes an explicit COUNT claim.
COUNT_CLAIM = re.compile(
    r"(_count|counts|count_|counts_once|second|twice|both_|_both|stacks|once_|"
    r"two_|three_|four_|1st|2nd|each)", re.I)
# Something in the assertion that actually MEASURES an exact quantity.
# `assert_eq!` / `assert_ne!` pin a value, so the test can fail on the count; a
# bare `assert!` presence check cannot. The narrower earlier versions
# (len()/count/total, then a numeric-literal regex) both mis-flagged real count
# tests — q195_existing_modifier_stacks_on_set_value does
# `assert_eq!(get_blade_modifier(liella), 4)`, which a nested-paren-free regex
# could not match. Presence and exactness are the distinction that matters.
COUNT_MEASURE = re.compile(r"assert_(?:eq|ne)!|\.len\(\)|_count\b|\brequired\b", re.I)


def sweep_a(limit=40, whole_tree=False):
    """Named-but-not-proven: a count/limit in the name, <=1 assert in the body.

    The sharp form of the check: when the name makes a COUNT claim, does the
    single assertion actually MEASURE a count? `assert_eq!(x, 2)` checks a count;
    `assert!(len(mods) >= 1)` checks presence. A name saying "counts once" whose
    assertion never looks at a count is the shape found by hand in
    rules/trigger_paths/jidou_paired_ability_combination_test.rs.
    """
    where = "" if whole_tree else "  (jidou/ only; pass --all to sweep every module)"
    print("=" * 72)
    print("SWEEP A - named but not proven" + where)
    print("  A test whose NAME promises a count/limit but whose body has <=1")
    print("  assertion. Where the name makes a COUNT claim, the assertion must")
    print("  also MEASURE a count; otherwise the count is never checked.")
    print("=" * 72)
    found = 0
    sharp = []
    for p in (test_files("") if whole_tree else test_files("jidou")):
        src = read(p)
        for chunk in re.split(r"\n(?=#\[test\])", src):
            m = re.search(r"fn\s+(\w+)\s*\(\)\s*\{", chunk)
            if not m or not COUNT_NAME.search(m.group(1)):
                continue
            name = m.group(1)
            body = chunk[m.end():]
            end = body.find("\n}")
            body = body[:end] if end != -1 else body
            n = len(re.findall(r"assert", body))
            if n > 1:
                continue
            found += 1
            # Sharp form: count-claiming name, but no count measured anywhere.
            if COUNT_CLAIM.search(name) and not COUNT_MEASURE.search(body):
                sharp.append((p.replace(ROOT + os.sep, ""), name))
            if found <= limit:
                note = "  (0 inline: usually a helper holds the assert)" if n == 0 else ""
                print(f"  asserts={n}  {p.replace(ROOT + os.sep, '')}::{name}{note}")
    print(f"  -> {found} weak out of all count/limit-naming tests scanned")
    print()
    print(f"  SHARP: {len(sharp)} make a COUNT claim but never MEASURE a count --")
    print("  the assertion cannot fail on the count, only on something else:")
    for p, name in sharp:
        print(f"      {p}::{name}")
    if sharp:
        print()
        print("  Two systematic FALSE-POSITIVE classes to check before believing any")
        print("  of these (both verified this audit):")
        print("    * helper-delegated — the body calls a local helper that does the")
        print("      asserting. Verified: rules/phases/live_success_rules_test.rs::")
        print("      two_live_cards_choose_second delegates to assert_two_live_placement(),")
        print("      which ends in assert_eq! on the success zone (a tight count check).")
        print("    * asserts=0 - the same case with the assert further up the stack.")
        print("  A real hit has a bare assert!/presence check INLINE. Verified real:")
        print("  effects/state/wait_activation/hanayo_..._test.rs::q248_hanayo_use_limit_")
        print("  blocks_second_activation, which passed for three unrelated reasons and")
        print("  never reached the use_limit it is named after.")
    return found


def sweep_b():
    """Bug-shape victims: jidou that can fire >1x/turn with a non-movement effect."""
    print()
    print("=" * 72)
    print("SWEEP B - re-scan-guard (Bug B) victim shapes")
    print("  A jidou with use_limit >= 2 (or unlimited) and a non-movement effect")
    print("  was silently under-fired by the guard leak. For each, confirm some test")
    print("  proves a second firing in a SEPARATE event - not two asserts in one.")
    print("=" * 72)
    try:
        d = json.load(open("cards/abilities.json", encoding="utf-8"))
    except FileNotFoundError:
        print("  cards/abilities.json not found; run from the repo root")
        return 0
    ua = d["unique_abilities"]
    n = 0
    unchecked = []
    for a in ua["unique_abilities"] if isinstance(ua, dict) else ua:
        if a.get("triggers") != "自動":
            continue
        lim = a.get("use_limit")
        if isinstance(lim, int):
            k = lim
        else:
            s = lim or ""
            k = 99 if (not s or "無制限" in s) else int(
                (re.search(r"(\d+)回", s) or [0, 1])[1])
        if k < 2:
            continue
        eff = json.dumps(a.get("effect"), ensure_ascii=False)
        if not any(e in eff for e in NONSELF_MOVEMENT_EFFECT):
            continue
        n += 1
        base = a["cards"][0].split(" | ")[0]
        best, where = two_event_coverage(base)
        if best < 2:
            unchecked.append((base, best, where))
    print(f"  -> {n} victim shapes; {len(unchecked)} abilities on "
          f"{len({c for c, _, _ in unchecked})} distinct cards lack a two-event test")
    seen_cards = set()
    for base, best, where in unchecked:
        # A card can carry several multi-use jidou (e.g. one on ライブ開始時 and
        # one on ライブ成功時). The actionable unit is the card, so report each
        # once and say how many abilities it has outstanding.
        dup = "" if base not in seen_cards else "  (additional ability, same card)"
        seen_cards.add(base)
        print(f"       NEEDS  {base:22} max_drivers_in_one_test={best}{dup}")
        if not where:
            print(f"              (no test references this card)")
    if unchecked:
        print()
        print("  NOTE: 'lacks a two-event test' does NOT imply 'undertested'. For a")
        print("  shape whose second trigger is unreachable in the current card pool the")
        print("  behaviour is unobservable and no test could fail. See")
        print("  docs/JIDOU_COMBINATION_WORK.md §7 (Sweep B) for the worked case: the")
        print("  stage->waitroom family has a proven re-entry, but NO card moves a")
        print("  standing member to the 控え室 without a baton touch, and the baton")
        print("  touch is area-locked for a same-turn arrival. Confirm reachability")
        print("  before treating a hit here as a coverage gap.")


def sweep_c():
    """Premise-never-happened: lenient asserts, and fillers that cannot yell."""
    print()
    print("=" * 72)
    print("SWEEP C - did the premise event actually happen?")
    print("  A test can be internally consistent and still prove nothing if the")
    print("  event that should drive it never fired. Two tells: (i) an assertion")
    print("  that accepts any value, (ii) a live driven by a 0-score FILLER card,")
    print("  which produces no yell at all.")
    print("=" * 72)
    lenient = []
    for p in test_files(""):
        src = code_only(read(p))
        for pat, kind in LENIENT:
            for m in re.finditer(pat, src):
                line = src[:m.start()].count("\n") + 1
                fn = None
                for fm in re.finditer(r"fn\s+(\w+)\s*\(\)\s*\{", src[:m.start()]):
                    fn = fm.group(1)
                lenient.append((kind, p.replace(ROOT + os.sep, ""), line, fn))
    print(f"  (i) lenient assertions anywhere in the tree: {len(lenient)}")
    for kind, p, line, fn in lenient[:30]:
        print(f"      {kind:18} {p}:{line}  fn={fn}")

    print()
    print("  (ii) yell/live-named test files that set a live card but NEVER a real one")
    n = 0
    for p in test_files(""):
        rel = p.replace(ROOT + os.sep, "")
        if not re.search(r"(yell|cheer|re_yell|reveal)", rel, re.I):
            continue
        src = code_only(read(p))
        lives = re.findall(r"set_live_card\(([^)]*)\)", src)
        if not lives:
            continue
        real = [x for x in lives
                if "filler" not in x.lower() and "sd1-010" not in x]
        if not real:
            n += 1
            print(f"      {rel}")
    print(f"  -> {n} file(s) cannot produce the event they are named for")
    return len(lenient) + n


# Sweep D — "absolute, not delta". A predicate over a modifier collection that
# the CARD'S OWN PRINTED STATS can satisfy on their own, so it passes whether or
# not the ability under test ever fired. The tell is the absence of a
# subtraction: `!blade_modifiers.is_empty()` where a correct test reads
# `blade_modifiers.len() - before == 1`.
#
# Verified to match the known-bad form and NOT the known-good delta form; see
# the note in sweep_d that a detector which silently reports zero is worse than
# no detector.
NEG_EMPTY = re.compile(r"!\s*[\w.]*_modifiers[\w.()]*\.is_empty\(\)")
LEN_GT0 = re.compile(r"_modifiers[\w.()]*\.len\(\)\s*>\s*0")


def sweep_d():
    """Absolute-not-delta predicates on modifier collections."""
    print()
    print("=" * 72)
    print("SWEEP D - absolute predicates a card's own stats can satisfy")
    print("  `!blade_modifiers.is_empty()` passes when the member is merely ON")
    print("  STAGE - printed blade/heart satisfy it with or without the jidou.")
    print("  A real test subtracts a baseline taken before the trigger.")
    print("=" * 72)
    hits = []
    for p in test_files(""):
        src = code_only(read(p))
        for pat in (NEG_EMPTY, LEN_GT0):
            for m in pat.finditer(src):
                # Only count it inside an assert!() and only when the enclosing
                # test takes no baseline that it subtracts.
                head = src[max(0, m.start() - 200):m.start()]
                if "assert!" not in head.split("\n")[-1] and "assert!(" not in head:
                    continue
                test_start = src.rfind("#[test]", 0, m.start())
                if test_start == -1:
                    continue
                body = src[test_start:m.start()]
                if re.search(r"_modifiers\.len\(\)\s*-", body) or \
                   re.search(r"-\s*\w*before", body):
                    continue  # this test does take a delta baseline
                line = src[:m.start()].count("\n") + 1
                fn = None
                for fm in re.finditer(r"fn\s+(\w+)\s*\(\)\s*\{", src[:m.start()]):
                    fn = fm.group(1)
                hits.append((p.replace(ROOT + os.sep, ""), line, fn))
    for p, line, fn in hits:
        print(f"      {p}:{line}  fn={fn}")
    print(f"  -> {len(hits)} absolute predicate(s) with no baseline")
    return len(hits)


# Sweep E — "permissive bounds". `assert!(x >= 2)` / `assert!(x <= 2)` accepts a
# RANGE, so it cannot distinguish the right answer from a wrong one in either
# direction: an ability that fires twice instead of once, or never at all, both
# satisfy `>= 2` / `<= 2`. Where the exact value is knowable, assert it.
#
# Found by hand in this audit: mymai_tonight_test.rs asserted `blade >= 2` for
# two live-start copies that grant exactly 1 each, so a double-fire passed a test
# whose whole point was that each instance fires once.
BOUND_ON_STATE = re.compile(
    r"assert!\s*\([^;]*?\b\w*(?:modifier|modifiers|blade|heart|energy|blade|"
    r"score|total|count|required|_\w+)\w*\s*(?:>=|<=)\s*-?\d", re.I)


# A name that CLAIMS a range, so >= / <= is the claim itself.
HONEST_BOUND_NAME = re.compile(
    r"at_least|at_most|atleast|atmost|_ge_|_le_|no_more_than|not_more_than|"
    r"at_least_two|termination|terminates|no_crash", re.I)


# A loop-termination guard, not a count claim: `while ... { scans += 1;
# assert!(scans < 32) }` bounds an ITERATION so a stuck state machine fails
# loudly. Verified: jidou/ability_watch/copied_ability_registration_lifecycle_test.rs
# ::real_phase_walk_to_live_success_registers_and_fires_the_copy_once uses
# `scans < 32` for exactly that, and its real claims are assert_eq! on the
# registration count with a reasoned precondition.
LOOP_COUNTER = re.compile(
    r"assert!\s*\(\s*(?:scans|guard|iterations|attempts|tries|loops)\s*(?:>=|<=|<|>)")


def sweep_e():
    """Permissive >= / <= bounds on state quantities."""
    print()
    print("=" * 72)
    print("SWEEP E - permissive bounds on a state quantity")
    print("  `assert!(x >= 2)` / `(x <= 2)` accepts a RANGE: an ability that fires")
    print("  twice, or never, both pass. Where the exact value is knowable, say it.")
    print("=" * 72)
    hits = []
    for p in test_files(""):
        src = code_only(read(p))
        for m in BOUND_ON_STATE.finditer(src):
            line = src[:m.start()].count("\n") + 1
            fn = None
            for fm in re.finditer(r"fn\s+(\w+)\s*\(\)\s*\{", src[:m.start()]):
                fn = fm.group(1)
            snippet = src[m.start():m.start() + 96].split("\n")[0]
            hits.append((p.replace(ROOT + os.sep, ""), line, fn, snippet.strip()))
    for p, line, fn, sn in hits:
        print(f"      {p}:{line}  fn={fn}")
        print(f"        {sn}")
    # Honest bounds: the NAME itself claims a range, so >= / <= is the claim.
    hits = [h for h in hits if not LOOP_COUNTER.match(h[3] or "")]
    honest = [h for h in hits if HONEST_BOUND_NAME.search(h[2] or "")]
    strict = [h for h in hits if h not in honest]
    print(f"  -> {len(hits)} permissive bound(s)")
    print(f"     {len(honest)} whose NAME says 'at least'/'at most' — the bound IS the")
    print("     claim, not a weakened version of it. Not defects.")
    print(f"     {len(strict)} need reading:")
    for p, line, fn, sn in strict:
        print(f"       {p}:{line}  fn={fn}")
    return len(hits)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--sweep", choices=["a", "b", "c", "d", "e"])
    ap.add_argument("--all", action="store_true",
                    help="sweep A over every test module, not just jidou/")
    args = ap.parse_args()
    if not os.path.isdir(ROOT):
        print(f"run from the repo root: {ROOT} not found")
        return 1
    if args.sweep in (None, "a"):
        sweep_a(whole_tree=args.all)
    if args.sweep in (None, "b"):
        sweep_b()
    if args.sweep in (None, "c"):
        sweep_c()
    if args.sweep in (None, "d"):
        sweep_d()
    if args.sweep in (None, "e"):
        sweep_e()
    return 0


if __name__ == "__main__":
    sys.exit(main())
