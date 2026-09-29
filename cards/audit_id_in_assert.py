"""Find `game.id(...)` calls used in an ASSERTION.

`TestGame::id()` hands out a FRESH pooled instance per call, so a card requested inside
an assertion is generally not the card the fixture staged, played or revealed. The
assertion then observes a card that is not in play and cannot fail for the right reason.

This is not a hypothetical: `PL!-pb2-015-P\u300d's `option_two_activates_no_member` test
did exactly this and passed for the wrong reason. The stage occupant has to be read
BACK from the zone instead.

Heuristic: an `id()` call that appears inside an `assert*` macro, or on the same
statement as one, is suspect. Report with the file, test and card so each can be judged.
"""
import io
import re
from pathlib import Path

TESTS = Path(r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine\tests")

ID_RE = re.compile(r"\bgame\.id\(\s*([A-Z0-9_]+)\s*\)")
CONST_RE = re.compile(r'const\s+([A-Z0-9_]+)\s*:\s*&str\s*=\s*"([^"]+)"')
FUNC_RE = re.compile(r"^fn\s+([A-Za-z0-9_]+)", re.M)

findings = []
for path in sorted(TESTS.rglob("*.rs")):
    text = io.open(path, encoding="utf-8", errors="replace").read()
    consts = dict(CONST_RE.findall(text))
    if not consts:
        continue
    # Split into test functions so a helper's own setup does not count.
    fns = list(FUNC_RE.finditer(text))
    for i, fm in enumerate(fns):
        start = fm.end()
        end = fns[i + 1].start() if i + 1 < len(fns) else len(text)
        body = text[start:end]
        if not fm.group(1).startswith(("test_", "the_", "a_", "an_", "option_", "branch_",
                                       "when_", "whenever", "selecting", "no_", "zero_",
                                       "two_", "three_", "three", "each_", "all_",
                                       "skipping", "paying", "declining", "a_card",
                                       "member_", "group_", "cost_", "blade_",
                                       "heart_", "yell_", "wait_", "draw_", "live_",
                                       "stage_", "zone_", "energy_", "reveal", "reorder",
                                       "look_", "discard", "optional_", "per_", "full_",
                                       "empty_", "first_", "second_", "third_", "one_")):
            # not a test-ish function; still scan, but note it
            pass
        # Look for id() inside an assert macro on the same line.
        for lineno, line in enumerate(body.splitlines(), 1):
            if "assert" in line and ID_RE.search(line):
                names = ID_RE.findall(line)
                for n in names:
                    findings.append((path.name, fm.group(1), n, consts.get(n, "?"),
                                     line.strip()[:90]))

print(f"suspect id()-in-assertion sites: {len(findings)}")
for f in findings:
    print(f"  {f[0]}::{f[1]}")
    print(f"      {f[2]} = {f[3]}")
    print(f"      {f[4]}")

# ---------------------------------------------------------------------------
# The dangerous form: reading GAME STATE through a freshly requested id.
# `state.mods.get_orientation_modifier(game.id(X))` asks about a card instance that was
# never staged, so the answer is vacuous. Static-data reads
# (`assert_card_in_group`) are fine and are listed above for completeness.
# ---------------------------------------------------------------------------
STATE_RE = re.compile(
    r"(?:state\.|mods\.|gs\.)[A-Za-z0-9_.]*\(?\s*game\.id\(\s*([A-Z0-9_]+)\s*\)"
)
dangerous = []
for path in sorted(TESTS.rglob("*.rs")):
    text = io.open(path, encoding="utf-8", errors="replace").read()
    consts = dict(CONST_RE.findall(text))
    if not consts:
        continue
    fns = list(FUNC_RE.finditer(text))
    for i, fm in enumerate(fns):
        start = fm.end()
        end = fns[i + 1].start() if i + 1 < len(fns) else len(text)
        for line in text[start:end].splitlines():
            if "assert" not in line:
                continue  # setup (pushing into a zone) is the correct use of id()
            for n in STATE_RE.findall(line):
                dangerous.append((path.name, fm.group(1), n, consts.get(n, "?"),
                                  line.strip()[:100]))

print()
print(f"GAME-STATE reads through a fresh id: {len(dangerous)}")
for d in dangerous:
    print(f"  {d[0]}::{d[1]}")
    print(f"      {d[2]} = {d[3]}")
    print(f"      {d[4]}")
