#!/usr/bin/env python3
"""Generate a direct Condition decoder from the `Condition` enum in card.rs.

Mirrors generate_effect_decoder.py but for the serde internally-tagged
`Condition` enum. Field name collisions across variants (the "field-type trap":
state = EffectState|CardState, ability_filter = AbilityFilter|ArcStr) are handled
by storing the raw ArcStr in the superset ConditionLocals and converting in the
per-variant build_* constructors.

Run: python cards/generate_condition_decoder.py
Output: engine/src/ability/condition_decoder_gen.rs
"""

import os
import sys

CARD_RS = os.path.join(
    os.path.dirname(__file__), "..", "engine", "src", "core", "card.rs"
)
OUT_RS = os.path.join(
    os.path.dirname(__file__),
    "..",
    "engine",
    "src",
    "ability",
    "condition_decoder_gen.rs",
)

# --- Rust field type -> BcReader method ---
READER_MAP = {
    "Option<bool>": "bc.read_bool_value()",
    "Option<u8>": "bc.read_u8_value()",
    "Option<ArcStr>": "bc.read_arc_str_value()",
    "Option<String>": "bc.read_string_value()",
    "Option<Operator>": "bc.read_operator_value()",
    "Option<Box<Vec<String>>>": "bc.read_opt_str_vec_value()",
    "Option<Vec<String>>": "bc.read_opt_str_vec_value().map(|b| *b)",
    "Option<Vec<Box<Condition>>>": "bc.read_condition_vec_value()",
    "Option<Box<Vec<u8>>>": "bc.read_opt_u8_vec_value()",
    "Option<Box<Vec<Box<AbilityEffect>>>>": "bc.read_effect_vec_boxed_value()",
    "Option<Box<Condition>>": "bc.read_condition_value()",
    "Option<Box<AbilityEffect>>": "bc.read_effect_value()",
    "Option<Box<PositionInfo>>": "bc.read_position_value()",
    "Option<Box<DistinctInfo>>": "bc.read_distinct_info_value()",
    "Option<Box<Vec<PositionCharacter>>>": "bc.read_positions_characters_value()",
    "Option<Box<LocationSubChecks>>": "bc.read_location_sub_checks_value()",
    "Option<Box<TriggerEvent>>": "bc.read_trigger_event_value()",
}

# Fields stored as raw ArcStr in the superset locals; converted per-variant in
# build_* (the serde field types differ across variants for the same key).
CONVERSION_FIELDS = {
    "card_type": "card_type",  # -> Option<ConditionCardType>
    "card_property": "card_property",  # -> Option<CardProperty>
    "comparison_type": "comparison_type",  # -> Option<ComparisonType>
    "comparison_target": "comparison_target",  # -> Option<ComparisonTarget>
    "state": "state",  # -> Option<EffectState> | Option<CardState>
    "ability_filter": "ability_filter",  # -> Option<AbilityFilter> | Option<ArcStr>
}

# Common fields on every variant (cfg-gated where the enum field is cfg-gated).
COMMON_FIELDS = ["text", "negation", "phase", "phase_target", "cache", "trigger_event"]


def parse_condition_enum(text):
    m = re_search(r"pub enum Condition\s*\{", text)
    if not m:
        print("ERROR: Could not find Condition enum", file=sys.stderr)
        return {}
    start = m.end()
    depth = 1
    pos = start
    while pos < len(text) and depth > 0:
        if text[pos] == "{":
            depth += 1
        elif text[pos] == "}":
            depth -= 1
        pos += 1
    body = text[start : pos - 1]

    variants = {}
    cur = None
    bd = 0
    for line in body.split("\n"):
        s = line.strip()
        vm = re_match(r"(\w+)\s*\{", s)
        if vm and bd == 0:
            cur = vm.group(1)
            variants[cur] = []
            bd = 1
            continue
        if cur is None:
            continue
        bd += s.count("{") - s.count("}")
        if s.startswith("#"):
            continue
        fm = re_match(r"(\w+)\s*:\s*(.+?)(?:,|$)", s)
        if fm and bd == 1:
            fname = fm.group(1)
            ftype = fm.group(2).strip().rstrip(",")
            if fname not in ("pub", "struct", "enum", "fn", "let", "mut", "if", "else", "common"):
                variants[cur].append((fname, ftype))
        if bd <= 0 and cur:
            cur = None
            bd = 0
    return variants


def parse_condition_common(text):
    """Parse the shared ConditionCommon struct field names/types."""
    m = re_search(r"pub struct ConditionCommon\s*\{", text)
    if not m:
        return {}
    start = m.end()
    depth = 1
    pos = start
    while pos < len(text) and depth > 0:
        if text[pos] == "{":
            depth += 1
        elif text[pos] == "}":
            depth -= 1
        pos += 1
    body = text[start : pos - 1]
    fields = {}
    for line in body.split("\n"):
        s = line.strip()
        if s.startswith("#"):
            continue
        fm = re_match(r"(?:pub\s+)?(\w+)\s*:\s*(.+?)(?:,|$)", s)
        if fm:
            fname = fm.group(1)
            ftype = fm.group(2).strip().rstrip(",")
            if fname not in ("struct", "enum", "fn", "let", "mut", "if", "else"):
                fields[fname] = ftype
    return fields


def build_field_expr(fname, ftype):
    """Return the Condition enum field expression for a variant field."""
    if fname in CONVERSION_FIELDS:
        if fname == "state":
            enum = "CardState" if ftype == "Option<CardState>" else "EffectState"
            return f"state: l.state.as_deref().map({enum}::from_str)"
        if fname == "ability_filter":
            if ftype == "Option<AbilityFilter>":
                return "ability_filter: l.ability_filter.as_deref().map(AbilityFilter::from_str)"
            return "ability_filter: l.ability_filter.clone()"
        enum = {
            "card_type": "ConditionCardType",
            "card_property": "CardProperty",
            "comparison_type": "ComparisonType",
            "comparison_target": "ComparisonTarget",
        }[fname]
        return f"{fname}: l.{fname}.as_deref().map({enum}::from_str)"
    if fname == "text":
        return '#[cfg(feature = "debug_conditions")] text: l.text.clone()'
    if fname == "trigger_event":
        return '#[cfg(feature = "debug_conditions")] trigger_event: l.trigger_event.clone()'
    return f"{fname}: l.{fname}.clone()"


def main():
    with open(CARD_RS, "r", encoding="utf-8") as f:
        card_rs = f.read()

    variants = parse_condition_enum(card_rs)
    print(f"Condition variants: {len(variants)}")
    for vname, vfields in variants.items():
        print(f"  {vname}: {len(vfields)} fields")

    # Union of all field names across variants + ConditionCommon (superset locals).
    common_fields = parse_condition_common(card_rs)
    all_names = []
    seen = set()
    for fname in common_fields:
        if fname not in seen:
            seen.add(fname)
            all_names.append(fname)
    for vname, vfields in variants.items():
        for fname, _ftype in vfields:
            if fname not in seen:
                seen.add(fname)
                all_names.append(fname)

    # Field type lookup: preferred type is the widest; CONVERSION_FIELDS use ArcStr.
    def field_type(fname):
        if fname in CONVERSION_FIELDS:
            return "Option<ArcStr>"
        if fname in common_fields:
            return common_fields[fname]
        for vname, vfields in variants.items():
            for fn, ft in vfields:
                if fn == fname:
                    return ft
        return None

    lines = []
    lines.append("// AUTO-GENERATED by generate_condition_decoder.py — DO NOT EDIT")
    lines.append("// Re-run: python cards/generate_condition_decoder.py")
    lines.append("//")
    lines.append("// Direct decoder for the serde internally-tagged `Condition` enum.")
    lines.append(
        "// `text`/`trigger_event` exist only under the `debug_conditions` feature"
    )
    lines.append("// and are skipped otherwise.")
    lines.append("use crate::card::{ConditionCommon, DistinctInfo, TriggerEvent};")
    lines.append("")

    # === ConditionLocals accumulator ===
    lines.append("/// Accumulator for Condition fields during direct decode.")
    lines.append("#[derive(Default)]")
    lines.append("struct ConditionLocals {")
    for fname in sorted(all_names):
        ftype = field_type(fname)
        cfg = ""
        if fname in ("text", "trigger_event"):
            cfg = '#[cfg(feature = "debug_conditions")]\n    '
        lines.append(f"    {cfg}pub {fname}: {ftype},")
    lines.append("}")
    lines.append("")

    # === decode_condition_field ===
    lines.append("/// Read one field from a condition object.")
    lines.append(
        "/// Returns true if the field was recognized and consumed, false to skip."
    )
    lines.append("fn decode_condition_field(")
    lines.append("    bc: &mut BcReader,")
    lines.append("    key: &str,")
    lines.append("    l: &mut ConditionLocals,")
    lines.append(") -> Option<bool> {")
    lines.append("    match key {")
    for fname in sorted(all_names):
        if fname == "text":
            lines.append(
                '            #[cfg(feature = "debug_conditions")]'
                ' "text" => { l.text = bc.read_string_value(); return Some(true); }'
            )
            lines.append(
                '            #[cfg(not(feature = "debug_conditions"))]'
                ' "text" => { bc.skip_value()?; return Some(true); }'
            )
            continue
        if fname == "trigger_event":
            lines.append(
                '            #[cfg(feature = "debug_conditions")]'
                ' "trigger_event" => { l.trigger_event = bc.read_trigger_event_value(); return Some(true); }'
            )
            lines.append(
                '            #[cfg(not(feature = "debug_conditions"))]'
                ' "trigger_event" => { bc.skip_value()?; return Some(true); }'
            )
            continue
        reader = READER_MAP.get(field_type(fname) or "")
        if reader:
            lines.append(
                f'            "{fname}" => {{ l.{fname} = {reader}; return Some(true); }}'
            )
        else:
            # Unknown-but-in-enum field type: skip the value.
            lines.append(
                f'            "{fname}" => {{ bc.skip_value()?; return Some(true); }}'
            )
    lines.append('            "type" => { bc.skip_value()?; return Some(true); }')
    lines.append("            _ => { bc.skip_value()?; return Some(true); }")
    lines.append("        }")
    lines.append("    }")
    lines.append("")

    # === build_* per variant ===
    for vname, vfields in variants.items():
        lines.append(f"fn build_{vname.lower()}(l: &ConditionLocals) -> Condition {{")
        lines.append(f"    Condition::{vname} {{")
        lines.append("        common: Box::new(ConditionCommon {")
        for fname in sorted(common_fields):
            expr = build_field_expr(fname, common_fields[fname])
            lines.append(f"            {expr},")
        lines.append("        }),")
        for fname, ftype in vfields:
            expr = build_field_expr(fname, ftype)
            lines.append(f"        {expr},")
        lines.append("    }")
        lines.append("}")
        lines.append("")

    # === decode_condition_direct ===
    lines.append("/// Direct decoder for TAG_OBJECT_VARIANT conditions.")
    lines.append("fn decode_condition_direct(")
    lines.append("    bc: &mut BcReader,")
    lines.append("    variant: u8,")
    lines.append(") -> Option<Condition> {")
    lines.append("    let count = bc.read_len()?;")
    lines.append("    let mut l = ConditionLocals::default();")
    lines.append("    for _ in 0..count {")
    lines.append("        let key = bc.key()?;")
    lines.append("        decode_condition_field(bc, key, &mut l)?;")
    lines.append("    }")
    lines.append("    Some(match variant {")
    for i, vname in enumerate(variants):
        lines.append(f"        {i} => build_{vname.lower()}(&l),")
    lines.append("        _ => return None,")
    lines.append("    })")
    lines.append("}")
    lines.append("")

    code = "\n".join(lines)
    with open(OUT_RS, "w", encoding="utf-8") as f:
        f.write(code)
    print(f"\nGenerated: {OUT_RS} ({len(code)} bytes)")


# Minimal regex helpers (avoid importing re at module top for clarity).
def re_search(pattern, text):
    import re

    return re.search(pattern, text)


def re_match(pattern, text):
    import re

    return re.match(pattern, text)


if __name__ == "__main__":
    main()
