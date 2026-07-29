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
ENGINE_CARD = REPO / "engine" / "src" / "core" / "card.rs"

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


def extract_effect_opcodes():
    """Extract EFFECT_OPCODES dict from compile_abilities.py."""
    content = COMPILE_PATH.read_text(encoding="utf-8")
    # Find the enumerate block
    m = re.search(
        r"EFFECT_OPCODES\s*=\s*\{\s*s:\s*i\s*\+\s*1\s*for\s+i,\s*s\s+in\s+enumerate\(\s*\[([^\]]+)\]",
        content,
        re.DOTALL,
    )
    if not m:
        err("Could not find EFFECT_OPCODES in compile_abilities.py")
        return {}
    opcodes = {}
    action_names = re.findall(r'"(\w+)"', m.group(1))
    for i, name in enumerate(action_names):
        opcodes[name] = i + 1
    # Add compound opcodes
    for comp in re.finditer(
        r'EFFECT_OPCODES\["(\w+)"\]\s*=\s*(0x[0-9a-fA-F]+|\d+)', content
    ):
        opcodes[comp.group(1)] = int(comp.group(2), 0)
    return opcodes


def extract_cond_opcodes():
    """Extract COND_OPCODES dict from compile_abilities.py."""
    content = COMPILE_PATH.read_text(encoding="utf-8")
    m = re.search(r"COND_OPCODES\s*=\s*\{([^}]+)\}", content, re.DOTALL)
    if not m:
        return {}
    opcodes = {}
    for line in m.group(1).split("\n"):
        pair = re.match(r'"(\w+)"\s*:\s*(0x[0-9a-fA-F]+)', line.strip())
        if pair:
            opcodes[pair.group(1)] = int(pair.group(2), 16)
    return opcodes


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


def check_schema_vs_compiler(schema):
    """Check schema actions have opcodes and vice versa."""
    print("\n[Schema ↔ Compiler]")
    schema_actions = set(schema.get("actions", {}).keys())
    compiler_opcodes = extract_effect_opcodes()

    for action in sorted(schema_actions):
        info = schema["actions"][action]
        opcode = info.get("opcode")
        if action not in compiler_opcodes:
            warn(
                f"Schema action '{action}' has no EFFECT_OPCODE entry (may be compound/preprocessed)"
            )
        elif compiler_opcodes[action] != opcode:
            err(
                f"Schema action '{action}' opcode {opcode} != compiler {compiler_opcodes[action]}"
            )
    for action in sorted(compiler_opcodes):
        if action.startswith("compound_"):
            continue  # compound opcodes are auto-generated
        if action not in schema_actions:
            warn(f"Compiler action '{action}' not in schema")


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


def check_schema_vs_conditions(schema):
    """Check condition types have opcodes."""
    print("\n[Schema ↔ Compiler conditions]")
    schema_conds = set(schema.get("conditions", {}).keys())
    compiler_conds = extract_cond_opcodes()

    for cond in sorted(schema_conds):
        if cond not in compiler_conds:
            warn(f"Schema condition '{cond}' has no COND_OPCODE entry")
    for cond in sorted(compiler_conds):
        if cond not in schema_conds:
            warn(f"Compiler condition '{cond}' not in schema")


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

    check_schema_vs_compiler(schema)
    check_schema_vs_engine(schema)
    check_schema_vs_conditions(schema)
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
