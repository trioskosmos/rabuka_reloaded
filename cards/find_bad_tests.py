#!/usr/bin/env python3
"""Find suspicious tests:
1. Abilities whose ONLY coverage comes from files with zero assertions (L0-only).
2. Individual #[test] functions whose body contains no assert!/expect.
"""
import io
import json
import re
import sys
from pathlib import Path

try:
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")
except (AttributeError, io.UnsupportedOperation):
    pass

ROOT = Path(__file__).resolve().parents[1]
inv = json.load(open(ROOT / "engine" / "tests" / "TEST_INVENTORY.json", encoding="utf-8"))

no_assert = [r for r in inv["abilities"] if r["covered"] and not r["has_assert"]]
print(f"=== abilities covered ONLY by files with zero assertions: {len(no_assert)}")
for r in no_assert[:15]:
    print(f"  {r['base']:30} {r['action']:24} files={r['covering_files'][:2]}")

fn_re = re.compile(r"#\[test\][\s\S]*?fn\s+(\w+)\s*\([^)]*\)\s*\{")
helper_re = re.compile(r"(?:pub\s+)?fn\s+(\w+)[^{]*\{")
bad = []
for p in (ROOT / "engine" / "tests").rglob("*.rs"):
    t = p.read_text(encoding="utf-8", errors="replace")
    # helper fns in this file whose body contains an assertion
    asserting_helpers = set()
    for hm in helper_re.finditer(t):
        hstart = hm.end() - 1
        depth = 1
        i = hstart + 1
        while i < len(t) and depth > 0:
            if t[i] == "{":
                depth += 1
            elif t[i] == "}":
                depth -= 1
            i += 1
        hbody = t[hstart:i]
        if "assert" in hbody or "expect" in hbody:
            asserting_helpers.add(hm.group(1))
    for m in fn_re.finditer(t):
        start = m.end()
        depth = 1
        i = start
        while i < len(t) and depth > 0:
            if t[i] == "{":
                depth += 1
            elif t[i] == "}":
                depth -= 1
            i += 1
        body = t[start:i]
        calls_helper = any(
            f"{h}(" in body for h in asserting_helpers if h != m.group(1)
        )
        if (
            "assert" not in body
            and "expect" not in body
            and "should_panic" not in body
            and "panic!" not in body
            and ".unwrap()" not in body
            and not calls_helper
        ):
            bad.append((p.name, m.group(1)))

print()
print(f"=== test functions with NO assertion/expect in body: {len(bad)}")
for f, fn in bad[:40]:
    print(f"  {f}: {fn}")
