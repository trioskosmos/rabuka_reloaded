#!/usr/bin/env python3
"""Parse EffectKind/AbilityEffect/CompoundBranch from card.rs and generate
a Rust direct decoder for ability effects.

Run: python generate_effect_decoder.py
Output: engine/src/ability/effect_decoder_gen.rs
"""

import re, os, sys

CARD_RS = os.path.join(
    os.path.dirname(__file__), "..", "engine", "src", "core", "card.rs"
)
OUT_RS = os.path.join(
    os.path.dirname(__file__), "..", "engine", "src", "ability", "effect_decoder_gen.rs"
)


def parse_struct_fields(text, struct_name):
    """Extract fields from a struct definition. Returns [(field_name, rust_type_str, [aliases])]."""
    m = re.search(rf"pub struct {struct_name}\s*\{{([^}}]+)\}}", text, re.DOTALL)
    if not m:
        return []
    body = m.group(1)
    fields = []
    pending_aliases = []
    for line in body.split("\n"):
        stripped = line.strip()
        # Detect alias attribute: #[serde(default, alias = "energy")]
        alias_m = re.search(r'alias\s*=\s*"(\w+)"', stripped)
        if alias_m:
            pending_aliases.append(alias_m.group(1))
            continue
        if not stripped or stripped.startswith("//") or stripped.startswith("pub"):
            continue
        fm = re.match(r"(\w+)\s*:\s*(.+?)(?:,|$)", stripped)
        if fm:
            fields.append((fm.group(1), fm.group(2).strip(), list(pending_aliases)))
            pending_aliases = []
    return fields


def parse_enum_variants(text):
    """Extract EffectKind enum variants with their fields.
    Returns {variant_name: [(field_name, rust_type_str)]}"""
    # Find the enum body
    m = re.search(r"pub enum EffectKind\s*\{(.+?)\n\}", text, re.DOTALL)
    if not m:
        print("ERROR: Could not find EffectKind enum", file=sys.stderr)
        return {}

    body = m.group(1)
    variants = {}
    current_variant = None
    brace_depth = 0
    pending_aliases = []

    for line in body.split("\n"):
        stripped = line.strip()

        # New variant: "VariantName {" or "VariantName{"
        vm = re.match(r"(\w+)\s*\{", stripped)
        if vm and brace_depth == 0:
            current_variant = vm.group(1)
            variants[current_variant] = []
            brace_depth = 1
            pending_aliases = []
            continue

        if current_variant is None:
            continue

        # Count braces for nested types
        brace_depth += stripped.count("{") - stripped.count("}")

        # Detect alias attribute
        alias_m = re.search(r'alias\s*=\s*"(\w+)"', stripped)
        if alias_m:
            pending_aliases.append(alias_m.group(1))
            continue

        # Field: "field_name: Type," or "field_name: Type"
        fm = re.match(r"#\[serde\(.*\)\]\s*", stripped)
        if fm:
            stripped = stripped[fm.end() :]

        fm = re.match(r"(\w+)\s*:\s*(.+?)(?:,|$)", stripped)
        if fm and brace_depth == 1:
            field_name = fm.group(1)
            field_type = fm.group(2).strip().rstrip(",")
            if field_name not in (
                "pub",
                "struct",
                "enum",
                "fn",
                "let",
                "mut",
                "if",
                "else",
            ):
                variants[current_variant].append(
                    (field_name, field_type, list(pending_aliases))
                )
                pending_aliases = []

        if brace_depth <= 0 and current_variant:
            current_variant = None
            brace_depth = 0

    return variants


def rust_type_to_reader(field_type):
    """Map a Rust type to the appropriate BcReader method call."""
    t = field_type.strip()

    if t == "Option<bool>":
        return "bc.read_bool_value()"
    if t == "Option<u8>":
        return "bc.read_u8_value()"
    if t == "Option<i8>":
        return "bc.read_i8_value()"
    if t == "Option<ArcStr>":
        return "bc.read_arc_str_value()"
    if t == "Option<Box<ArcStr>>":
        return "bc.read_arc_str_value().map(|s| Box::new(s))"
    if t == "Option<CardType>":
        return "bc.read_card_type_value()"
    if t == "Option<PlacementOrder>":
        return "bc.read_placement_order_value()"
    if t == "Option<Box<QuotedText>>":
        return "bc.read_quoted_text_value()"
    if t == "Option<Operator>":
        return "bc.read_operator_value()"
    if t == "Option<Box<Vec<String>>>":
        return "bc.read_opt_str_vec_value()"
    if t == "Box<Vec<String>>":
        return "bc.read_str_vec_value()"
    if t == "Option<Box<Condition>>":
        return "bc.read_condition_value()"
    if t == "Option<Box<AbilityEffect>>":
        return "bc.read_effect_value()"
    if t == "Option<Vec<Box<AbilityEffect>>>":
        return "bc.read_effect_vec_value()"
    if t == "Option<Box<Vec<Box<AbilityEffect>>>>":
        return "bc.read_effect_vec_boxed_value()"
    if t == "Option<Box<PositionInfo>>":
        return "bc.read_position_value()"
    if t == "Option<Box<DynamicCount>>":
        return "bc.read_dynamic_count_value()"
    if t == "Option<Box<EffectState>>":
        return "bc.read_effect_state_value()"
    if t == "Option<Box<DistinctType>>":
        return "bc.read_distinct_value()"
    if t == "Option<AbilityFilter>":
        return "bc.read_ability_filter_value()"
    if t == "Option<Box<Vec<String>>>":
        return "bc.read_opt_str_vec_value()"
    if t == "Option<Box<Vec<AbilityFilterBranch>>>":
        return "bc.read_or_ability_filters_value()"
    if t == "Option<ArcStr>":
        return "bc.read_arc_str_value()"
    # Default: skip
    return None


def generate_decoder(variants, ability_effect_fields, compound_fields):
    """Generate the Rust decoder code."""
    lines = []
    lines.append("// AUTO-GENERATED by generate_effect_decoder.py — DO NOT EDIT")
    lines.append("// Re-run: python cards/generate_effect_decoder.py")
    lines.append("")

    # Collect all unique field names across all variants + AbilityEffect + Compound
    all_keys = set()
    for vfields in variants.values():
        for fname, ftype, aliases in vfields:
            all_keys.add(fname)
    for fname, ftype in ability_effect_fields:
        all_keys.add(fname)
    for fname, ftype in compound_fields:
        all_keys.add(fname)

    # Generate the main decoder function
    lines.append("/// Read one field from a TAG_OBJECT_VARIANT effect object.")
    lines.append(
        "/// Returns true if the field was recognized and consumed, false to skip."
    )
    lines.append("fn decode_effect_field(bc: &mut BcReader, key: &str,")
    lines.append("    // AbilityEffect fields")
    lines.append("    text: &mut ArcStr, action: &mut ActionType,")
    lines.append("    source: &mut Option<ArcStr>, destination: &mut Option<ArcStr>,")
    lines.append("    count_val: &mut Option<u8>, target: &mut Option<ArcStr>,")
    lines.append("    condition: &mut Option<Box<Condition>>,")
    lines.append(
        "    non_stackable: &mut Option<bool>, conditional: &mut Option<bool>,"
    )
    lines.append("    is_further: &mut Option<bool>, optional: &mut Option<bool>,")
    lines.append("    max: &mut Option<bool>,")
    lines.append("    effect_steps: &mut Option<Vec<Box<AbilityEffect>>>,")
    lines.append("    // CompoundBranch fields")
    lines.append("    look_action: &mut Option<Box<AbilityEffect>>,")
    lines.append("    select_action: &mut Option<Box<AbilityEffect>>,")
    lines.append("    actions: &mut Option<Vec<Box<AbilityEffect>>>,")
    lines.append("    primary_effect: &mut Option<Box<AbilityEffect>>,")
    lines.append("    alternative_condition: &mut Option<Box<Condition>>,")
    lines.append("    result_condition: &mut Option<Box<Condition>>,")
    lines.append("    followup_action: &mut Option<Box<AbilityEffect>>,")
    lines.append("    optional_action: &mut Option<Box<AbilityEffect>>,")
    lines.append("    conditional_action: &mut Option<Box<AbilityEffect>>,")
    lines.append("    conditional_negation: &mut Option<bool>,")
    lines.append("    // EffectKind shared fields (all variants)")
    lines.append("    ek: &mut EffectKindLocals,")
    lines.append(") -> Option<bool> {")
    lines.append("    match key {")

    # AbilityEffect fields
    ae_dispatch = {
        "text": (
            "*text = bc.read_string_value().map(ArcStr::from).unwrap_or_default();",
            True,
        ),
        "action": (
            "*action = ActionType::from_str(&bc.read_string_value().unwrap_or_default()).unwrap_or_default();",
            True,
        ),
        "source": ("*source = bc.read_arc_str_value();", True),
        "destination": ("*destination = bc.read_arc_str_value();", True),
        "count": ("*count_val = bc.read_u8_value();", True),
        "target": ("*target = bc.read_arc_str_value();", True),
        "condition": ("*condition = bc.read_condition_value();", True),
        "non_stackable": ("*non_stackable = bc.read_bool_value();", True),
        "conditional": ("*conditional = bc.read_bool_value();", True),
        "is_further": ("*is_further = bc.read_bool_value();", True),
        "optional": ("*optional = bc.read_bool_value();", True),
        "max": ("*max = bc.read_bool_value();", True),
        "effect_steps": ("*effect_steps = bc.read_effect_vec_value();", True),
    }
    cb_dispatch = {
        "look_action": ("*look_action = bc.read_effect_value();", True),
        "select_action": ("*select_action = bc.read_effect_value();", True),
        "actions": ("*actions = bc.read_effect_vec_value();", True),
        "primary_effect": ("*primary_effect = bc.read_effect_value();", True),
        "alternative_condition": (
            "*alternative_condition = bc.read_condition_value();",
            True,
        ),
        "result_condition": ("*result_condition = bc.read_condition_value();", True),
        "followup_action": ("*followup_action = bc.read_effect_value();", True),
        "optional_action": ("*optional_action = bc.read_effect_value();", True),
        "conditional_action": ("*conditional_action = bc.read_effect_value();", True),
        "conditional_negation": ("*conditional_negation = bc.read_bool_value();", True),
    }
    for fname, (code, _) in ae_dispatch.items():
        lines.append(f'            "{fname}" => {{ {code} return Some(true); }}')
    for fname, (code, _) in cb_dispatch.items():
        lines.append(f'            "{fname}" => {{ {code} return Some(true); }}')

    # EffectKind fields - generate from the ek struct
    # We generate match arms for ALL field names that appear in ANY variant
    ek_field_map = {}  # field_name -> reader_expression
    ek_aliases = {}  # alias_name -> field_name
    # Fields excluded from ek_field_map because they have serde aliases
    # that map JSON keys to different Rust field names
    for vfields in variants.values():
        for fname, ftype, aliases in vfields:
            if fname not in ek_field_map:
                reader = rust_type_to_reader(ftype)
                if reader:
                    ek_field_map[fname] = (reader, ftype)
            for alias in aliases:
                ek_aliases[alias] = fname

    # Fields that need extra copies: when seen in bytecode, also set a related field
    field_extra_copies = {"max_repeats": "repeat_limit"}

    for fname in sorted(ek_field_map.keys()):
        reader, ftype = ek_field_map[fname]
        extra = ""
        if fname in field_extra_copies:
            target = field_extra_copies[fname]
            if target in ek_field_map:
                extra = f" ek.{target} = ek.{fname}.clone();"
        lines.append(
            f'            "{fname}" => {{ ek.{fname} = {reader};{extra} return Some(true); }}'
        )
    # Hardcoded aliases: JSON uses different key names than Rust fields
    # These are fields where the JSON key ≠ Rust field name due to serde aliases.
    # The bytecode always uses the JSON key name, so we map it to the Rust field.
    hardcoded_aliases = {"energy": "energy_count"}
    for alias, target in hardcoded_aliases.items():
        if target in ek_field_map:
            reader, _ = ek_field_map[target]
            lines.append(
                f'            "{alias}" => {{ ek.{target} = {reader}; return Some(true); }}'
            )
    # Add alias arms for aliases that don't conflict with field names
    for alias, target in sorted(ek_aliases.items()):
        if alias in ek_field_map:
            continue  # handled by the field arm above
        reader, ftype = ek_field_map.get(target, (None, None))
        if reader:
            lines.append(
                f'            "{alias}" => {{ ek.{target} = {reader}; return Some(true); }}'
            )

    lines.append("            _ => { bc.skip_value()?; return Some(true); }")
    lines.append("        }")
    lines.append("    }")
    lines.append("")

    # EffectKindLocals struct - all fields as Option<T> defaults
    lines.append("/// Accumulator for EffectKind fields during direct decode.")
    lines.append("#[derive(Default)]")
    lines.append("pub(crate) struct EffectKindLocals {")
    # Collect ALL unique fields across ALL variants
    all_ek_fields = {}
    boxed_arcstr_fields = set()  # fields where first occurrence is Option<Box<ArcStr>>
    for vname, vfields in variants.items():
        for fname, ftype, aliases in vfields:
            if fname not in all_ek_fields:
                all_ek_fields[fname] = ftype
                if ftype == "Option<Box<ArcStr>>":
                    boxed_arcstr_fields.add(fname)

    for fname, ftype in sorted(all_ek_fields.items()):
        # Make field type always Option or Box default
        if ftype.startswith("Box<Vec<String>>"):
            lines.append(f"    pub {fname}: {ftype},")
        elif ftype.startswith("Box<Vec<"):
            lines.append(f"    pub {fname}: {ftype},")
        elif ftype == "Vec<String>":
            lines.append(f"    pub {fname}: {ftype},")
        elif ftype.startswith("Option<ArcStr>") or ftype == "Option<ArcStr>":
            lines.append(f"    pub {fname}: Option<ArcStr>,")
        elif ftype.startswith("Option<Box<") or ftype.startswith("Option<Vec<"):
            lines.append(f"    pub {fname}: {ftype},")
        elif ftype.startswith("Option<"):
            lines.append(f"    pub {fname}: {ftype},")
        else:
            lines.append(f"    pub {fname}: Option<{ftype}>,")
    lines.append("}")
    lines.append("")

    # Constructor from locals for each variant
    for vname, vfields in sorted(variants.items()):
        lines.append(
            f"fn build_{vname.lower()}(ek: &EffectKindLocals) -> EffectKind {{"
        )
        lines.append(f"    EffectKind::{vname} {{")
        for fname, ftype, aliases in vfields:
            if ftype in ("Box<Vec<String>>", "Vec<String>"):
                lines.append(f"        {fname}: ek.{fname}.clone(),")
            elif ftype == "Option<Box<ArcStr>>":
                if fname in boxed_arcstr_fields:
                    lines.append(f"        {fname}: ek.{fname}.clone(),")
                else:
                    lines.append(
                        f"        {fname}: ek.{fname}.clone().map(|s| Box::new(s)),"
                    )
            elif ftype.startswith("Option<Box<") or ftype.startswith("Option<Vec<"):
                lines.append(f"        {fname}: ek.{fname}.clone(),")
            elif ftype.startswith("Option<"):
                lines.append(f"        {fname}: ek.{fname}.clone(),")
            elif ftype in ("bool", "u8", "i8"):
                lines.append(f"        {fname}: ek.{fname},")
            else:
                lines.append(f"        {fname}: ek.{fname}.clone(),")
        lines.append("    }")
        lines.append("}")
        lines.append("")

    return "\n".join(lines)


def main():
    with open(CARD_RS, "r", encoding="utf-8") as f:
        card_rs = f.read()

    variants = parse_enum_variants(card_rs)
    ability_effect_fields = parse_struct_fields(card_rs, "AbilityEffect")
    compound_fields = parse_struct_fields(card_rs, "CompoundBranch")

    print(f"EffectKind variants: {len(variants)}")
    for vname, vfields in sorted(variants.items()):
        print(f"  {vname}: {len(vfields)} fields")

    print(f"AbilityEffect: {len(ability_effect_fields)} fields")
    print(f"CompoundBranch: {len(compound_fields)} fields")

    code = generate_decoder(variants, ability_effect_fields, compound_fields)

    with open(OUT_RS, "w", encoding="utf-8") as f:
        f.write(code)

    print(f"\nGenerated: {OUT_RS} ({len(code)} bytes)")


if __name__ == "__main__":
    main()
