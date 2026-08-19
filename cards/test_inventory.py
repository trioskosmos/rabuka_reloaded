#!/usr/bin/env python3
"""Automated ability-coverage inventory for the Rust engine test suite.

Generates from the real card database (cards/abilities.json) + test suite
(engine/tests/**/*.rs):

  * engine/tests/TEST_COVERAGE.md   — enhanced coverage_report (backwards-compat)
  * docs/ABILITY_MATRIX.md          — trigger×action matrix + condition/set breakdown
  * engine/tests/TEST_INVENTORY.json — machine-readable per-ability rows
  * engine/tests/TEST_INVENTORY.md  — human-readable per-ability index

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
import json
import re
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CARDS_JSON = ROOT / "cards" / "abilities.json"
TESTS_DIR = ROOT / "engine" / "tests"
OUT_COVERAGE = ROOT / "engine" / "tests" / "TEST_COVERAGE.md"
OUT_MATRIX = ROOT / "docs" / "ABILITY_MATRIX.md"
OUT_JSON = ROOT / "engine" / "tests" / "TEST_INVENTORY.json"
OUT_MD = ROOT / "engine" / "tests" / "TEST_INVENTORY.md"

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
NEGATIVE_RE = re.compile(r"(negative|cannot|not_|_not|cannot_activate|already_waited|zero_tested|immune|blocked|empty|zero|skip_optional)", re.IGNORECASE)
# choice/edge signals
CHOICE_RE = re.compile(r"(has_pending_choice|pending_choice_type|select_indices|drain_auto|SelectCard|SelectTarget)")

FN_TEST_RE = re.compile(r"^\s*#\[test\]\s*\n\s*(?:pub\s+)?fn\s+(\w+)", re.MULTILINE)
COVERS_RE = re.compile(r"@covers\s+([A-Z0-9!+\-]+\S*)", re.IGNORECASE)


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
            if hit:
                covered_rels.append(rel)
                covering_texts.append(text)
                covering_fns.extend(fns)

        covered = bool(covered_rels) or covers_override is not None
        depth, flags = infer_ability_depth(covering_texts, covered_rels, covering_fns)
        if not covered:
            depth = "none"

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

    return {
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
        "all_src_len": len(all_src),
    }


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
    w(f"_Auto-generated by `cards/test_inventory.py` (wraps `cards/coverage_report.py`) — do not edit by hand. Rerun `python cards/test_inventory.py` after changing the parser, card data, or tests._")
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
    args = ap.parse_args()

    inv = build_inventory()

    # render
    coverage_text = render_coverage(inv)
    matrix_text = render_matrix(inv)
    inventory_md_text = render_inventory_md(inv)
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
        })

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
        "abilities": json_rows,
    }, ensure_ascii=False, indent=2) + "\n"

    if args.check:
        def strip_generated_at(text):
            # ignore volatile timestamp for check
            return re.sub(r'"generated_at":\s*"[^"]*"', '"generated_at": "CHECK"', text)
        ok = True
        for path, new_text in [(OUT_COVERAGE, coverage_text), (OUT_MATRIX, matrix_text), (OUT_MD, inventory_md_text), (OUT_JSON, json_text)]:
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
        sys.exit(0 if ok else 1)

    if not args.json_only:
        OUT_COVERAGE.parent.mkdir(parents=True, exist_ok=True)
        OUT_COVERAGE.write_text(coverage_text, encoding="utf-8")
        print(f"Wrote {OUT_COVERAGE.relative_to(ROOT)} ({len(coverage_text)} bytes)")
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
    return 0


if __name__ == "__main__":
    sys.exit(main())
