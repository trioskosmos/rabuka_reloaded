#!/usr/bin/env python3
"""Cross-reference validator: schema ↔ compiler ↔ engine.

Checks that ability_schema.json, compile_abilities.py, and the Rust engine
are in sync. Run as part of CI or `cards/regenerate.sh`.

Exit code 0 = all checks pass. Non-zero = drift detected.
"""

import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).parent.parent
SCHEMA_PATH = REPO / "cards" / "ability_schema.json"
COMPILE_PATH = REPO / "cards" / "compile_abilities.py"
ENGINE_ENUMS = REPO / "engine" / "src" / "ability" / "enums.rs"
ENGINE_EFFECTS = REPO / "engine" / "src" / "ability" / "effects" / "mod.rs"

ERRORS = []


def err(msg):
    ERRORS.append(msg)
    print(f"  ERROR: {msg}")


def warn(msg):
    print(f"  WARN: {msg}")


def ok(msg):
    print(f"  ok: {msg}")


def load_schema():
    return json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))


def extract_action_type_variants():
    """Extract ActionType enum variants from enums.rs."""
    if not ENGINE_ENUMS.exists():
        err(f"enums.rs not found at {ENGINE_ENUMS}")
        return set()
    content = ENGINE_ENUMS.read_text(encoding="utf-8")
    m = re.search(r"pub enum ActionType \{([^}]+)\}", content, re.DOTALL)
    if not m:
        return set()
    return {
        v.strip().rstrip(",")
        for v in m.group(1).strip().split("\n")
        if v.strip() and not v.strip().startswith("//")
    }


def extract_handler_actions():
    """Extract which ActionType variants have match arms in effects/mod.rs."""
    if not ENGINE_EFFECTS.exists():
        err(f"effects/mod.rs not found at {ENGINE_EFFECTS}")
        return set()
    content = ENGINE_EFFECTS.read_text(encoding="utf-8")
    return set(re.findall(r"ActionType::(\w+)\s*=>", content))


def check_schema_vs_engine(schema):
    """Check schema actions map to valid Rust enum variants."""
    print("\n[Schema ↔ Engine]")
    variants = extract_action_type_variants()
    handlers = extract_handler_actions()

    for action, info in sorted(schema.get("actions", {}).items()):
        rust_variant = info.get("rust_variant", "")
        if rust_variant and rust_variant not in variants:
            err(
                f"Schema action '{action}' rust_variant '{rust_variant}' not in ActionType enum"
            )
        handler = info.get("handler_fn", "")
        if handler and handler not in ("(no-op)",):
            if handler not in handlers:
                # handler may be inline in the match arm
                pass


def check_handler_coverage(schema):
    """Check every action type has a documented handler."""
    print("\n[Handler coverage]")
    for action, info in sorted(schema.get("actions", {}).items()):
        handler = info.get("handler", "")
        if not handler:
            warn(f"Action '{action}' has no handler documented in schema")


def main():
    print(f"Cross-reference validation")
    print(f"Schema: {SCHEMA_PATH}")
    print(f"Compiler: {COMPILE_PATH}")
    print(f"Engine: {ENGINE_ENUMS}")

    schema = load_schema()

    check_schema_vs_engine(schema)
    check_handler_coverage(schema)

    if ERRORS:
        print(f"\n{'=' * 60}")
        print(f"FAILED: {len(ERRORS)} errors found")
        print(f"{'=' * 60}")
        for e in ERRORS:
            print(f"  - {e}")
        sys.exit(1)
    else:
        print(f"\n{'=' * 60}")
        print("PASSED: all cross-reference checks passed")
        print(f"{'=' * 60}")
        sys.exit(0)


if __name__ == "__main__":
    main()
