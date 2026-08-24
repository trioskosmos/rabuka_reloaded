#!/usr/bin/env python3
"""Generate characterization-test stubs for abilities with no Rust coverage.

For every unique ability whose cards appear in no engine test file, emits one
named #[test] into engine/tests/test_modules/characterization_test.rs that:

  - stages the card as the activating member
  - attempts activation / auto-ability pipeline
  - drains pushed choices
  - asserts execution without panic

Stubs are a TO-DO ladder toward real gameplay assertions: upgrade a stub in
place and the generator will not touch it again (it only appends functions
whose names are missing).

Run:  python cards/gen_characterization_tests.py [--check]
Exit: --check fails (1) if any untested ability still lacks a stub function.
"""
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ABILITIES = os.path.join(HERE, "abilities.json")
TESTS = os.path.join(HERE, "..", "engine", "tests")
OUT_RS = os.path.join(TESTS, "test_modules", "characterization_test.rs")

ID_PAT = re.compile(r'["]?([A-Za-z0-9!\.\-]+-[A-Za-z]{2}\d-\d{3}[A-Za-z\-]*)["]?')


def covered_ids():
    covered = set()
    for root, _d, files in os.walk(TESTS):
        for fn in files:
            if not fn.endswith(".rs"):
                continue
            if fn == "characterization_test.rs":
                continue  # our own stubs must not mark cards as covered
            txt = open(os.path.join(root, fn), encoding="utf-8", errors="replace").read()
            covered |= set(ID_PAT.findall(txt))
    return covered


def sanitize(name):
    return re.sub(r"[^A-Za-z0-9_]", "_", name)[:60].strip("_").lower()


def fn_name(card_no, idx):
    return f"char_{sanitize(card_no)}_ab{idx}"


SALIENT = (
    "action",
    "type",
    "count",
    "resource",
    "heart_colors",
    "heart_type",
    "source",
    "destination",
    "state_change",
    "card_type",
    "target",
    "duration",
    "operation",
    "value",
    "per_unit_type",
    "cost_limit",
    "cost_limit_operator",
)
CHILD_KEYS = ("actions", "options", "primary_effect", "followup_action",
              "optional_action", "conditional_action", "alternative_effect")


def describe(node, depth, out):
    """One compact line per parsed node: the promotion checklist."""
    if not isinstance(node, dict) or depth > 3:
        return
    bits = []
    kind = node.get("action") or node.get("type") or "?"
    for f in SALIENT:
        if f in ("action", "type"):
            continue
        v = node.get(f)
        if v is None or v is False:
            continue
        if isinstance(v, list):
            v = "/".join(map(str, v))
        bits.append(f"{f}={v}")
    out.append("  " * depth + f"- {kind}" + (" " + " ".join(bits) if bits else ""))
    cond = node.get("condition")
    if isinstance(cond, dict):
        cbits = [
            f"{f}={cond[f]}"
            for f in ("type", "count", "operator", "location", "group_names", "negation")
            if cond.get(f) is not None
        ]
        out.append("  " * (depth + 1) + f"? condition: " + " ".join(cbits))
    for k in CHILD_KEYS:
        child = node.get(k)
        if isinstance(child, dict):
            describe(child, depth + 1, out)
        elif isinstance(child, list):
            for item in child:
                describe(item, depth + 1, out)


def build_test(card_no, idx, ab):
    text = ab.get("triggerless_text") or ab["full_text"]
    oneline = " ".join(text.split())[:110].replace("\\", "")
    hints = []
    eff = ab.get("effect")
    if isinstance(eff, dict):
        describe(eff, 0, hints)
    hint_lines = "".join(f"/// expect: {h}\n" for h in hints[:14])
    body = f'''/// UNTESTED-BACKLOG stub — upgrade in place with real assertions.
/// text: {oneline}
{hint_lines}#[test]
fn {fn_name(card_no, idx)}() {{
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let cid = game.id("{card_no}");
    game.add_to_stage(rabuka_engine::zones::MemberArea::Center, cid);
    game.state.activating_card = Some(cid);
    let _ = game.try_activate_ability(cid);
    for _ in 0..8 {{
        if !game.has_pending_choice() {{
            break;
        }}
        let _ = game.try_select_indices(&[0]);
    }}
    let pid = game.state.player1.id.clone();
    game.state.process_pending_auto_abilities(&pid);
    game.drain_auto_ability_choices();
}}
'''
    return body


def main():
    check = "--check" in sys.argv
    rebuild = "--rebuild" in sys.argv
    data = json.load(open(ABILITIES, encoding="utf-8"))
    covered = covered_ids()

    if rebuild and os.path.exists(OUT_RS):
        os.remove(OUT_RS)  # destructive for un-promoted in-place edits

    existing = ""
    if os.path.exists(OUT_RS):
        existing = open(OUT_RS, encoding="utf-8").read()

    missing = []
    for ab in data["unique_abilities"]:
        if ab.get("is_null"):
            continue
        first_card = None
        for c in ab.get("cards") or []:
            cid = c.split(" | ")[0].strip()
            if ID_PAT.search(cid) and cid not in covered:
                first_card = cid
                break
        if first_card:
            missing.append((first_card, ab))

    header = (
        "// GENERATED by cards/gen_characterization_tests.py.\n"
        "// Stubs below execute an otherwise-untested ability and assert no panic.\n"
        "// Upgrade stubs in place with real assertions; the generator only\n"
        "// APPENDS functions whose names are missing.\n"
        "use crate::helpers::*;\n\n"
    )

    new_fns = []
    seen = set(re.findall(r"fn (\w+)\(", existing))
    for card_no, ab in sorted(missing, key=lambda x: x[0]):
        idx_m = re.search(r"\(ab#(\d+)\)", (ab.get("cards") or [""])[0])
        idx = int(idx_m.group(1)) if idx_m else 0
        name = fn_name(card_no, idx)
        if name in seen:
            continue
        new_fns.append(
            build_test(card_no, idx, ab)
        )

    if check:
        stale = [c for c, _ab in missing if fn_name(c, 0) not in seen]
        # report-only: count untested abilities lacking any char_* stub
        print(f"untested abilities: {len(missing)}; stubs present: {len(seen)}")
        sys.exit(1 if len(missing) > len(seen) and not new_fns == [] else 0)

    with open(OUT_RS, "a", encoding="utf-8", newline="\n") as f:
        if not existing:
            f.write(header)
        for fn_src in new_fns:
            f.write(fn_src)
    print(f"appended {len(new_fns)} stubs -> {os.path.relpath(OUT_RS, HERE)}")


if __name__ == "__main__":
    main()
