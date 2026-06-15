"""Analyze ability coverage from test_debug.log against abilities.json.
Reports coverage by trigger, action type, condition type, and identifies
truly untested action types (no tests exist vs tests exist but are invisible
to the resolver-based tracking).

Usage:
    cd engine && cargo test --test run_all -- --nocapture 2> ../test_debug.log
    cd .. && python scripts/analyze_coverage.py [--full]
"""

import json, re, sys, os
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ABILITIES_JSON = ROOT / "cards" / "abilities.json"
TEST_LOG = ROOT / "test_debug.log"
TEST_DIR = ROOT / "engine" / "tests"

SKIP_IF_IN_LOG = object()  # sentinel: skip because tracked by resolver log

# ============== PARSING ==============


def parse_tested_texts(log_path):
    """Return set of full_text strings that were activated during tests."""
    tested = set()
    abi = re.compile(r'\[AB\]\s*ABILITY\s+"(.+?)"\s+\(\d+\)')
    txt = re.compile(r"\[AB\]\s+TEXT\s+(.*)")
    with open(log_path, encoding="utf-8", errors="replace") as f:
        lines = f.readlines()
    i = 0
    while i < len(lines):
        m = abi.search(lines[i])
        if m:
            for j in range(i + 1, min(i + 10, len(lines))):
                mt = txt.search(lines[j])
                if mt:
                    tested.add(mt.group(1).strip())
                    break
            i += 1
            continue
        i += 1
    return tested


def load_abilities(path):
    """Return unique_abilities list from abilities.json."""
    with open(path, encoding="utf-8") as f:
        data = json.load(f)
    return data.get("unique_abilities", [])


def test_file_references(action_type):
    """Count references to an action type in test files."""
    count = 0
    if not TEST_DIR.exists():
        return 0
    for f in TEST_DIR.rglob("*.rs"):
        try:
            text = f.read_text(encoding="utf-8", errors="replace")
            count += text.count(action_type)
        except:
            pass
    return count


def resolve_action_type(action_type, in_test_files, in_log, status):
    """Determine if an action type is truly untested or just invisible."""
    if in_log > 0:
        return "tested (log)"
    if in_test_files > 0:
        return "tested (file refs)"
    return "UNTESTED"


# ============== REPORT ==============


def write_report(text):
    path = ROOT / "coverage_report.txt"
    with open(path, "w", encoding="utf-8") as f:
        f.write(text)
    print(f"Report: {path}")
    # Try to print first 10 lines to console
    for line in text.split("\n")[:15]:
        try:
            print(line)
        except UnicodeEncodeError:
            print(line.encode("ascii", "replace").decode())


def generate_report(full_mode=False):
    if not TEST_LOG.exists():
        print(
            f"ERROR: {TEST_LOG} not found. Run:\n  cd engine && cargo test --test run_all -- --nocapture 2> ../test_debug.log",
            file=sys.stderr,
        )
        return

    ua = load_abilities(ABILITIES_JSON)
    tested = parse_tested_texts(TEST_LOG)
    total = len(ua)
    tested_count = sum(1 for e in ua if e.get("full_text", "").strip() in tested)

    lines = []
    L = lambda s="": lines.append(s)

    L("=" * 64)
    L("ABILITY COVERAGE ANALYSIS")
    L("=" * 64)
    L(f"Unique abilities:   {total}")
    L(f"Tested (resolver):  {tested_count}  ({tested_count / total * 100:.1f}%)")
    L(
        f"Untested:           {total - tested_count}  (may include abilities tested outside resolver)"
    )
    L("")

    # === BY TRIGGER ===
    total_trigger = Counter()
    tested_trigger = Counter()
    for e in ua:
        ft = e.get("full_text", "").strip()
        t = e.get("triggers", "") or "(none)"
        total_trigger[t] += 1
        if ft in tested:
            tested_trigger[t] += 1

    L("--- BY TRIGGER TYPE ---")
    L("  Note: '常時'(continuous) abilities are handled by recalculate_constants")
    L("  outside the ability resolver, so 0% here is EXPECTED, not a gap.")
    L(f"{'Trigger':30s} {'Total':>6s} {'Tested':>6s} {'Rate':>8s}")
    L("-" * 52)
    for t in sorted(total_trigger, key=lambda x: -total_trigger[x]):
        tot = total_trigger[t]
        tes = tested_trigger[t]
        rate = tes / tot * 100 if tot else 0
        note = " (bypassed)" if t == "常時" else ""
        L(f"{t[:28]:30s} {tot:6d} {tes:6d} {rate:7.1f}%{note}")
    L()

    # === BY ACTION TYPE ===
    def collect_actions(effect, results=None):
        if results is None:
            results = set()
        if isinstance(effect, dict):
            a = effect.get("action", "")
            if a:
                results.add(a)
            for sub_key in ("actions", "conditional_action", "optional_action"):
                subs = effect.get(sub_key)
                if isinstance(subs, list):
                    for s in subs:
                        collect_actions(s, results)
                elif isinstance(subs, dict):
                    collect_actions(subs, results)
        return results

    all_actions = set()
    action_to_abilities = defaultdict(list)
    for e in ua:
        ft = e.get("full_text", "").strip()
        eff = e.get("effect")
        acts = collect_actions(eff) if isinstance(eff, dict) else set()
        for a in acts:
            action_to_abilities[a].append(ft)
            all_actions.add(a)

    L("--- BY ACTION TYPE ---")
    L("  Status legend: 'tested (log)'=in resolver log | 'tested (file refs)'=in test")
    L("  source but bypasses resolver (e.g. restriction) | 'UNTESTED'=nowhere")
    L(
        f"{'Action Type':30s} {'Count':>6s} {'Tested':>6s} {'FileRefs':>8s} {'Status':>22s}"
    )
    L("-" * 74)
    for a in sorted(all_actions):
        count = len(action_to_abilities[a])
        tested_here = sum(1 for ft in action_to_abilities[a] if ft in tested)
        file_refs = test_file_references(a)
        status = resolve_action_type(a, file_refs, tested_here, None)
        L(f"{a[:28]:30s} {count:6d} {tested_here:6d} {file_refs:8d} {status:22s}")
    L()

    # === BY CONDITION TYPE ===
    def collect_conditions(entry, results=None):
        if results is None:
            results = set()
        if isinstance(entry, dict):
            ct = entry.get("condition_type") or entry.get("type") or ""
            if ct and ct != "compound":
                results.add(ct)
            for v in entry.values():
                if isinstance(v, (dict, list)):
                    collect_conditions(v, results)
        elif isinstance(entry, list):
            for item in entry:
                collect_conditions(item, results)
        return results

    cond_counts = Counter()
    for e in ua:
        ft = e.get("full_text", "").strip()
        if ft not in tested:
            for ct in collect_conditions(e.get("effect", {})):
                cond_counts[ct] += 1

    if cond_counts:
        L("--- UNTESTED ABILITIES BY CONDITION TYPE ---")
        L(f"{'Condition Type':30s} {'Untested':>8s}")
        L("-" * 40)
        for ct in sorted(cond_counts, key=lambda x: -cond_counts[x]):
            L(f"{ct[:28]:30s} {cond_counts[ct]:8d}")
        L()

    # === ZERO-TESTED ACTION TYPES (thorough check) ===
    L("--- ZERO-TESTED ACTION TYPES (cross-referenced with test files) ---")
    L(f"{'Action Type':30s} {'InLog':>6s} {'FileRefs':>8s} {'Status':>30s}")
    L("-" * 76)
    zero_tested = []
    for a in sorted(all_actions):
        count = len(action_to_abilities[a])
        tested_here = sum(1 for ft in action_to_abilities[a] if ft in tested)
        file_refs = test_file_references(a)
        if tested_here == 0 and file_refs == 0:
            zero_tested.append((a, count))
            L(f"{a[:28]:30s} {tested_here:6d} {file_refs:8d} {'TRULY UNTESTED':>30s}")
    L()
    if zero_tested:
        L(f"Truly untested action types ({len(zero_tested)}):")
        for a, c in zero_tested:
            L(f"  {a} ({c} abilities)")
    else:
        L("Every action type has at least some coverage (log or file refs).")
    L()

    # === UNIQUE UNTESTED ABILITIES (no card expansion) ===
    L("--- UNIQUE UNTESTED ABILITIES (first 40, by trigger+action) ---")
    untested_entries = [e for e in ua if e.get("full_text", "").strip() not in tested]
    untested_entries.sort(
        key=lambda e: (
            e.get("triggers", "") or "",
            (e.get("effect") or {}).get("action", "") or "",
        )
    )
    for i, e in enumerate(untested_entries[:40]):
        ft = e.get("full_text", "").strip().replace("\n", " ")[:80]
        trig = (e.get("triggers", "") or "none")[:12]
        act = (e.get("effect") or {}).get("action", "?")[:16]
        cards_short = ", ".join(c.split("|")[0].strip() for c in e.get("cards", [])[:2])
        L(f"  [{trig:12s}][{act:16s}] {ft}")
        L(f"      cards: {cards_short}")
        L("")
    if len(untested_entries) > 40:
        L(f"  ... and {len(untested_entries) - 40} more unique untested abilities")
    L()

    L("=" * 64)
    L("Generated by scripts/analyze_coverage.py")
    L(f"Log: {TEST_LOG}")
    L("=" * 64)

    write_report("\n".join(lines))


if __name__ == "__main__":
    full = "--full" in sys.argv
    generate_report(full)
