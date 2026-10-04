"""Find `game.id(...)` calls used in an ASSERTION.

`TestGame::id()` hands out a FRESH pooled instance per call, so a card requested inside
an assertion is generally not the card the fixture staged, played or revealed. The
assertion then observes a card that is not in play and cannot fail for the right reason.

This is not a hypothetical: `PL!-pb2-015-P＋`'s `option_two_activates_no_member` test
did exactly this and passed for the wrong reason. The stage occupant has to be read
BACK from the zone instead.

Two lists are reported, and they are NOT the same severity:

* **suspect id()-in-assertion sites** — any `id()` on an `assert*` line, for judgement.
  A static-data read such as `assert_card_in_group(id, …)` is the *correct* pattern and
  shows up here; those are noise, not findings.
* **GAME-STATE reads through a fresh id** — the dangerous form, e.g.
  `state.mods.get_orientation_modifier(game.id(X))`, which asks about an instance that was
  never staged so the answer is vacuous. This is the list to drive to zero.

Exit status is 0 either way: the suspect list is advisory. Read the counts.
"""
import io
import re
import sys
from pathlib import Path

# Derived from this file, not hardcoded. The previous absolute path meant that on any
# other machine — CI included — `rglob` walked a directory that does not exist, matched
# nothing, and the script printed "0 findings", which reads exactly like a clean result.
TESTS = Path(__file__).resolve().parent.parent / "engine" / "tests"
if not TESTS.is_dir():
    sys.exit(f"ERROR: tests directory not found at {TESTS}")

ID_RE = re.compile(r"\bgame\.id\(\s*([A-Z0-9_]+)\s*\)")
CONST_RE = re.compile(r'const\s+([A-Z0-9_]+)\s*:\s*&str\s*=\s*"([^"]+)"')
FUNC_RE = re.compile(r"^fn\s+([A-Za-z0-9_]+)", re.M)
STATE_RE = re.compile(
    r"(?:state\.|mods\.|gs\.)[A-Za-z0-9_.]*\(?\s*game\.id\(\s*([A-Z0-9_]+)\s*\)"
)


def iter_test_bodies(path):
    """Yield (fn_name, body, consts) per `fn` in the file.

    Split by function so a HELPER's own setup is not attributed to the tests around it.
    One pass per file, shared by both lists below — they used to walk the tree and split
    on functions independently, which is the same traversal written twice.
    """
    text = io.open(path, encoding="utf-8", errors="replace").read()
    consts = dict(CONST_RE.findall(text))
    if not consts:
        return
    fns = list(FUNC_RE.finditer(text))
    for i, fm in enumerate(fns):
        end = fns[i + 1].start() if i + 1 < len(fns) else len(text)
        yield fm.group(1), text[fm.end():end], consts


paths = sorted(TESTS.rglob("*.rs"))
suspect = []
dangerous = []
for path in paths:
    for fn_name, body, consts in iter_test_bodies(path):
        for line in body.splitlines():
            if "assert" not in line:
                # Setup (pushing a card into a zone) is the CORRECT use of `id()`.
                continue
            for n in ID_RE.findall(line):
                suspect.append((path.name, fn_name, n, consts.get(n, "?"), line.strip()[:90]))
            for n in STATE_RE.findall(line):
                dangerous.append(
                    (path.name, fn_name, n, consts.get(n, "?"), line.strip()[:100])
                )

print(f"files scanned                  : {len(paths)}")
print(f"suspect id()-in-assertion sites: {len(suspect)}")
for s in suspect:
    print(f"  {s[0]}::{s[1]}")
    print(f"      {s[2]} = {s[3]}")
    print(f"      {s[4]}")

print()
print(f"GAME-STATE reads through a fresh id: {len(dangerous)}")
for d in dangerous:
    print(f"  {d[0]}::{d[1]}")
    print(f"      {d[2]} = {d[3]}")
    print(f"      {d[4]}")