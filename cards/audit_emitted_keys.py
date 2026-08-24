#!/usr/bin/env python3
"""CI gate: every key emitted into cards/abilities.json must be declared in
engine/src/core/card.rs (struct fields, enum variant fields, serde aliases).

A key that card.rs doesn't declare is silently dropped by serde
deserialization — parser-emitted semantics the engine never sees. This gate
fails when a NEW unknown key appears, so decoder support must land in the
same change.

Usage:
    python cards/audit_emitted_keys.py            # report + exit code
    python cards/audit_emitted_keys.py --update   # rewrite known-keys allowlist
"""
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
CARD_RS = os.path.join(HERE, "..", "engine", "src", "core", "card.rs")
ABILITIES = os.path.join(HERE, "abilities.json")
ALLOWLIST = os.path.join(HERE, ".emitted_keys_allowlist")

# Keys consumed outside card.rs (bytecode vm aliases / loader metadata).
SPECIAL_KEYS = {
    "cards",        # loader metadata, stripped pre-encode
    "costs",        # vm.rs: aliased with "options"
    "options",      # choice options vector
    "text",         # documentary source sentence
    "type",         # serde tag
    "action",
}


def known_fields():
    src = open(CARD_RS, encoding="utf-8").read()
    known = {"type", "action", "text"}
    for m in re.finditer(r"^\s*pub (\w+)\s*:", src, re.M):
        known.add(m.group(1))
    # enum variant fields: 'Variant {' body lines like 'movement: Option<..>'
    for m in re.finditer(r"^\s+(\w+)\s*:\s*[A-Z<\[\(]", src, re.M):
        known.add(m.group(1))
    for m in re.finditer(r'(?:alias|rename)\s*=\s*"([^"]+)"', src):
        known.add(m.group(1))
    return known


def emitted_keys():
    data = json.load(open(ABILITIES, encoding="utf-8"))
    keys = set()

    def walk(node):
        if isinstance(node, dict):
            keys.update(node.keys())
            for v in node.values():
                walk(v)
        elif isinstance(node, list):
            for item in node:
                walk(item)

    for ab in data["unique_abilities"]:
        walk(ab.get("cost") or {})
        walk(ab.get("effect") or {})
    return keys


def main():
    update = "--update" in sys.argv
    known = known_fields() | SPECIAL_KEYS
    emitted = emitted_keys()
    unknown = sorted(emitted - known)

    if update:
        with open(ALLOWLIST, "w", encoding="utf-8") as f:
            f.write("\n".join(unknown) + ("\n" if unknown else ""))
        print(f"allowlist written: {len(unknown)} known-unknown keys")
        return

    allowlisted = set()
    if os.path.exists(ALLOWLIST):
        allowlisted = {
            ln.strip() for ln in open(ALLOWLIST, encoding="utf-8") if ln.strip()
        }
    fresh = [k for k in unknown if k not in allowlisted]
    if fresh:
        print("FAIL: new emitted keys not declared in card.rs:")
        for k in fresh:
            print(f"  {k}")
        print("Add engine decode support (or justify + --update allowlist).")
        sys.exit(1)
    print(
        f"OK: {len(emitted)} emitted keys, {len(unknown)} outside card.rs "
        f"(all allowlisted), 0 new."
    )


if __name__ == "__main__":
    main()
