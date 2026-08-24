#!/usr/bin/env python3
"""Pipeline dashboard: what is happening across
    936 Japanese ability texts -> parser -> abilities.json -> Rust engine.

Sections
  1. corpus overview
  2. action-type / condition-type histograms (what the corpus actually uses)
  3. suspicious parses (custom actions, empty effects, do_nothing)
  4. emitted-key audit vs engine/src/core/card.rs (CI gate)
  5. semantic-validation issues vs baseline
  6. abilities with no engine-test coverage (the gameplay-as-written backlog)

Run:  python cards/pipeline_report.py [--section N]
Exit codes: 0 ok, 1 unknown emitted keys or parse-integrity failure.
"""
import collections
import contextlib
import io
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ABILITIES = os.path.join(HERE, "abilities.json")
CARD_RS = os.path.join(HERE, "..", "engine", "src", "core", "card.rs")
TESTS_DIR = os.path.join(HERE, "..", "engine", "tests")
BASELINE = os.path.join(HERE, "validation_baseline.json")

sys.path.insert(0, os.path.join(HERE, "ability_extraction"))
from parser import _validate_semantic  # noqa: E402

SPECIAL_KEYS = {"cards", "costs", "options", "text", "type", "action"}

# Emitted outside card.rs but consumed elsewhere / documentary.
# Dispositions live in cards/ability_extraction/PARSER_NOTES.md.
DOCUMENTED_KEYS = {
    "zone",              # vm.rs aliases source
    "energy",            # pay_energy count via icons
    "max_repeats",       # aliased as repeat_limit in decoders/effects
    "costs",             # vm.rs aliases options
    "source_location",   # gain_ability_from_source hardcodes under-member
    "per_character",     # LL-bp7-001 play-cost marker (documented limitation)
    "action_reference",  # action_success_condition degrades to AlwaysTrue
    "baton_touch",       # appearance-node flag read via trigger_event paths
}


def load():
    return json.load(open(ABILITIES, encoding="utf-8"))


def walk(node):
    if isinstance(node, dict):
        yield node
        for v in node.values():
            yield from walk(v)
    elif isinstance(node, list):
        for item in node:
            yield from walk(item)


def s1_corpus(data):
    ab = data["unique_abilities"]
    real = [a for a in ab if not a.get("is_null")]
    print(f"unique abilities : {len(ab)} ({len(real)} real, {len(ab)-len(real)} null notes)")
    print(f"distinct cards   : {data['statistics']['cards_with_abilities']}")


def s2_histograms(data):
    from parser import _KNOWN_CONDITIONS

    acts = collections.Counter()
    conds = collections.Counter()
    for a in data["unique_abilities"]:
        for node in walk(a.get("effect") or {}):
            if node.get("action"):
                acts[node["action"]] += 1
            t = node.get("type")
            if t and (
                t in _KNOWN_CONDITIONS or t.endswith("_condition") or t == "compound"
            ):
                conds[t] += 1
        for node in walk(a.get("cost") or {}):
            if node.get("type"):
                acts[f"cost:{node['type']}"] += 1
    print("\naction types (effect + cost):")
    for k, c in acts.most_common():
        print(f"  {c:>5}  {k}")
    print("\ncondition types:")
    for k, c in conds.most_common():
        print(f"  {c:>5}  {k}")


def s3_suspicious(data):
    custom = []
    empty = []
    for a in data["unique_abilities"]:
        if a.get("is_null"):
            continue
        eff = a.get("effect") or {}
        if not eff:
            empty.append(a)
            continue
        nodes = list(walk(eff))
        if all(n.get("action") in ("custom", "do_nothing") for n in nodes):
            custom.append(a)
    print(f"effects fully custom/do_nothing : {len(custom)}")
    print(f"empty effect objects           : {len(empty)}")
    for a in (custom + empty)[:15]:
        print(f"  {(a.get('cards') or ['?'])[0]} | {a['full_text'][:70]}")


def known_card_rs_keys():
    src = open(CARD_RS, encoding="utf-8").read()
    known = {"type", "action", "text"}
    for m in re.finditer(r"^\s*pub (\w+)\s*:", src, re.M):
        known.add(m.group(1))
    for m in re.finditer(r"^\s+(\w+)\s*:\s*[A-Z<\[\(]", src, re.M):
        known.add(m.group(1))
    for m in re.finditer(r'(?:alias|rename)\s*=\s*"([^"]+)"', src):
        known.add(m.group(1))
    return known


def s4_key_audit(data):
    known = known_card_rs_keys() | SPECIAL_KEYS | DOCUMENTED_KEYS
    emitted = set()

    def collect(node):
        if isinstance(node, dict):
            emitted.update(node.keys())
            for v in node.values():
                collect(v)
        elif isinstance(node, list):
            for i in node:
                collect(i)

    for a in data["unique_abilities"]:
        collect(a.get("cost") or {})
        collect(a.get("effect") or {})
    unknown = sorted(emitted - known)
    print(f"emitted keys: {len(emitted)}; outside card.rs: {len(unknown)}")
    for k in unknown:
        print(f"  {k}")
    return unknown


def s5_validation(data):
    baseline = {}
    if os.path.exists(BASELINE):
        baseline = json.load(open(BASELINE, encoding="utf-8"))
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        issues = _validate_semantic(data["unique_abilities"])
    counts = collections.Counter(r[0] for r in issues)
    regressions = {
        r: (baseline.get(r, 0), c) for r, c in counts.items() if c > baseline.get(r, 0)
    }
    print(f"semantic issues: {sum(counts.values())} across {len(counts)} rules")
    for r, c in sorted(counts.items(), key=lambda x: -x[1]):
        flag = "REGRESSION" if r in regressions else ("known" if baseline.get(r) else "")
        print(f"  {c:>4}  {r:24} {flag}")
    if regressions:
        print("FAIL: validation regressions vs baseline")
        return True
    return False


def s6_untested(data):
    # Collect every card id quoted in the Rust test suite.
    pat = re.compile(r'"([A-Z]{1,4}![A-Za-z0-9!\.\-]+-[A-Za-z]{2}\d-\d{3}[A-Za-z\-]*)"?')
    covered = set()
    for root, _dirs, files in os.walk(TESTS_DIR):
        for fn in files:
            if not fn.endswith(".rs") or fn == TEST_INVENTORY_NAME:
                continue
            txt = open(os.path.join(root, fn), encoding="utf-8", errors="replace").read()
            covered |= set(pat.findall(txt))

    def base_ids(cards):
        out = set()
        for c in cards or []:
            out.add(c.split(" | ")[0].strip())
        return out

    missing = []
    for a in data["unique_abilities"]:
        if a.get("is_null"):
            continue
        ids = base_ids(a.get("cards"))
        if ids and not (ids & covered):
            missing.append(a)
    print(f"abilities whose cards appear in NO rust test: {len(missing)} / 936")
    for a in sorted(missing, key=lambda x: -len(x["full_text"]))[:30]:
        print(f"  {a['full_text'][:78]}")


TEST_INVENTORY_NAME = "TEST_INVENTORY.json"


def main():
    data = load()
    want = None
    if "--section" in sys.argv:
        want = set(sys.argv[sys.argv.index("--section") + 1 :])
        want = {int(x) for x in want}
    runs = {1: s1_corpus, 2: s2_histograms, 3: s3_suspicious, 4: None,
            5: s5_validation, 6: s6_untested}

    for n, fn in runs.items():
        if want and n not in want and n != 4:
            continue
        if n == 4:
            unknown = s4_key_audit(data)
            if unknown:
                print("FAIL: new emitted keys not declared in card.rs (see above)")
                sys.exit(1)
            continue
        print("=" * 72)
        print(f"SECTION {n}")
        fn(data)
    sys.exit(0)


if __name__ == "__main__":
    main()
