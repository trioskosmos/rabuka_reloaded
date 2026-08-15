#!/usr/bin/env python3
"""Parse EffectKind/AbilityEffect/CompoundBranch/EffectFilter from card.rs and
generate a Rust direct decoder for ability effects.

Run: python cards/generate_effect_decoder.py
Output: engine/src/ability/effect_decoder_gen.rs
"""

import re, os, sys

CARD_RS = os.path.join(
    os.path.dirname(__file__), "..", "engine", "src", "core", "card.rs"
)
OUT_RS = os.path.join(
    os.path.dirname(__file__), "..", "engine", "src", "ability", "effect_decoder_gen.rs"
)

# --- Rust type -> BcReader method ---
READER_MAP = {
    "Option<bool>": "bc.read_bool_value()",
    "Option<u8>": "bc.read_u8_value()",
    "Option<i8>": "bc.read_i8_value()",
    "Option<ArcStr>": "bc.read_arc_str_value()",
    "Option<Zone>": "bc.read_zone_value()",
    "Option<Box<ArcStr>>": "bc.read_arc_str_value().map(|s| Box::new(s))",
    "Option<CardType>": "bc.read_card_type_value()",
    "Option<PlacementOrder>": "bc.read_placement_order_value()",
    "Option<Box<QuotedText>>": "bc.read_quoted_text_value()",
    "Option<Operator>": "bc.read_operator_value()",
    "Option<Operation>": "bc.read_operation_value()",
    "Option<Box<Vec<String>>>": "bc.read_opt_str_vec_value()",
    "Box<Vec<String>>": "bc.read_str_vec_value()",
    "Option<Box<Vec<u8>>>": "bc.read_opt_u8_vec_value()",
    "Option<Vec<u8>>": "bc.read_opt_u8_vec_value().map(|b| *b)",
    "Option<Box<Condition>>": "bc.read_condition_value()",
    "Option<Box<AbilityEffect>>": "bc.read_effect_value()",
    "Option<Vec<Box<AbilityEffect>>>": "bc.read_effect_vec_value()",
    "Option<Box<Vec<Box<AbilityEffect>>>>": "bc.read_effect_vec_boxed_value()",
    "Option<Box<PositionInfo>>": "bc.read_position_value()",
    "Option<Box<DynamicCount>>": "bc.read_dynamic_count_value()",
    "Option<Box<EffectState>>": "bc.read_effect_state_value()",
    "Option<Box<DistinctType>>": "bc.read_distinct_value()",
    "Option<AbilityFilter>": "bc.read_ability_filter_value()",
    "Option<Box<Vec<AbilityFilterBranch>>>": "bc.read_or_ability_filters_value()",
}


def rust_type_to_reader(field_type):
    """Map a Rust type to the appropriate BcReader method call."""
    return READER_MAP.get(field_type.strip())


def parse_struct_fields(text, struct_name):
    """Extract fields from a struct definition.
    Returns [(field_name, rust_type_str, [aliases])].
    Handles nested generics via brace-depth counting and pub prefixes.
    """
    m = re.search(rf"pub struct {struct_name}\s*\{{", text, re.DOTALL)
    if not m:
        return []
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

    fields = []
    pending_aliases = []
    skip_next = False  # skip #[serde(flatten)] and #[serde(skip)] fields

    for line in body.split("\n"):
        stripped = line.strip()

        # Skip serde skip/flatten attributes -> skip the next field line
        if "serde(skip" in stripped or "serde(flatten" in stripped:
            skip_next = True
            continue

        # Detect alias attribute
        alias_m = re.search(r'alias\s*=\s*"(\w+)"', stripped)
        if alias_m:
            pending_aliases.append(alias_m.group(1))
            continue

        if not stripped or stripped.startswith("//") or stripped.startswith("#["):
            continue

        fm = re.match(r"(?:pub\s+)?(\w+)\s*:\s*(.+?)(?:,|$)", stripped)
        if fm:
            if skip_next:
                skip_next = False
                continue
            fields.append((fm.group(1), fm.group(2).strip(), list(pending_aliases)))
            pending_aliases = []
        else:
            skip_next = False

    return fields


def parse_enum_variants(text):
    """Extract EffectKind enum variants with their fields.
    Returns {variant_name: [(field_name, rust_type_str, [aliases])]}
    Uses brace-depth counting for robustness with nested types.
    """
    m = re.search(r"pub enum EffectKind\s*\{", text, re.DOTALL)
    if not m:
        print("ERROR: Could not find EffectKind enum", file=sys.stderr)
        return {}

    start = m.end()
    # Find the entire enum body using brace depth
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
    current_variant = None
    brace_depth = 0
    pending_aliases = []

    for line in body.split("\n"):
        stripped = line.strip()

        # New variant: "VariantName {"
        vm = re.match(r"(\w+)\s*\{", stripped)
        if vm and brace_depth == 0:
            current_variant = vm.group(1)
            variants[current_variant] = []
            brace_depth = 1
            pending_aliases = []
            continue

        if current_variant is None:
            continue

        brace_depth += stripped.count("{") - stripped.count("}")

        # Detect alias attribute
        alias_m = re.search(r'alias\s*=\s*"(\w+)"', stripped)
        if alias_m:
            pending_aliases.append(alias_m.group(1))
            continue

        # Skip other attributes
        if stripped.startswith("#["):
            continue

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


def generate_decoder(variants, ability_effect_fields, compound_fields, filter_fields):
    """Generate the Rust decoder code."""
    lines = []
    lines.append("// AUTO-GENERATED by generate_effect_decoder.py — DO NOT EDIT")
    lines.append("// Re-run: python cards/generate_effect_decoder.py")
    lines.append("")

    # === Collect all field names across all sources ===
    ae_field_names = {f[0] for f in ability_effect_fields}
    cb_field_names = {f[0] for f in compound_fields}
    ae_cb_keys = ae_field_names | cb_field_names

    # Fields from variants that overlap with AE/CB (handled by AE/CB dispatch first)
    ek_field_map = {}  # field_name -> (reader_expr, rust_type)
    ek_aliases = {}  # alias_name -> field_name

    for vfields in variants.values():
        for fname, ftype, aliases in vfields:
            if fname not in ek_field_map:
                reader = rust_type_to_reader(ftype)
                if reader:
                    ek_field_map[fname] = (reader, ftype)
            for alias in aliases:
                ek_aliases[alias] = fname

    # === Build the decode_effect_field function ===
    lines.append("/// Read one field from a TAG_OBJECT_VARIANT effect object.")
    lines.append(
        "/// Returns true if the field was recognized and consumed, false to skip."
    )
    lines.append("fn decode_effect_field(bc: &mut BcReader, key: &str,")

    # Generate function parameters from AbilityEffect fields
    lines.append("    // AbilityEffect fields")
    for fname, ftype, _ in ability_effect_fields:
        lines.append(f"    {fname}: &mut {ftype},")

    # Generate function parameters from CompoundBranch fields
    lines.append("    // CompoundBranch fields")
    for fname, ftype, _ in compound_fields:
        lines.append(f"    {fname}: &mut {ftype},")

    lines.append("    // EffectKind shared fields (all variants)")
    lines.append("    ek: &mut EffectKindLocals,")
    lines.append(") -> Option<bool> {")
    lines.append("    match key {")

    # --- AbilityEffect match arms ---
    # text and action need special readers (non-Option types)
    ae_special = {
        "text": "*text = bc.read_string_value().map(ArcStr::from).unwrap_or_default();",
        "action": "*action = ActionType::from_str(&bc.read_string_value().unwrap_or_default()).unwrap_or_default();",
    }

    for fname, ftype, _ in ability_effect_fields:
        if fname in ae_special:
            lines.append(
                f'            "{fname}" => {{ {ae_special[fname]} return Some(true); }}'
            )
        else:
            reader = rust_type_to_reader(ftype)
            if reader:
                lines.append(
                    f'            "{fname}" => {{ *{fname} = {reader}; return Some(true); }}'
                )

    # --- CompoundBranch match arms ---
    for fname, ftype, _ in compound_fields:
        reader = rust_type_to_reader(ftype)
        if reader:
            lines.append(
                f'            "{fname}" => {{ *{fname} = {reader}; return Some(true); }}'
            )

    # --- EffectKind variant fields (ek) ---
    # Fields already in ae_cb_keys get their match arms from above; skip duplicates
    for fname in sorted(ek_field_map.keys()):
        if fname in ae_cb_keys:
            continue
        reader, ftype = ek_field_map[fname]
        lines.append(
            f'            "{fname}" => {{ ek.{fname} = {reader}; return Some(true); }}'
        )

    # --- Filter-only fields (not in any variant, not in AE/CB) ---
    filter_only_keys = ae_cb_keys | set(ek_field_map.keys())
    filter_field_types = {f[0]: f[1] for f in filter_fields}
    for fname, ftype, _ in filter_fields:
        if fname not in filter_only_keys:
            reader = rust_type_to_reader(ftype)
            if reader:
                lines.append(
                    f'            "{fname}" => {{ ek.{fname} = {reader}; return Some(true); }}'
                )
                filter_only_keys.add(fname)

    # --- Field name aliases (serde aliases + hardcoded bytecode aliases) ---
    # Collect all serde aliases from variant definitions
    for alias, target in sorted(ek_aliases.items()):
        if alias in ae_cb_keys or alias in ek_field_map:
            continue  # already handled by the field arm above
        reader, ftype = ek_field_map.get(target, (None, None))
        if reader:
            lines.append(
                f'            "{alias}" => {{ ek.{target} = {reader}; return Some(true); }}'
            )

    # Bytecode uses different key names than Rust field names in some cases
    # These are fields where the parser writes "key_A" but Rust struct has "key_B"
    bytecode_key_aliases = {
        "energy": "energy_count",
        "max_repeats": "repeat_limit",
    }
    for alias, target in bytecode_key_aliases.items():
        if alias in ae_cb_keys or alias in ek_field_map or alias in ek_aliases:
            continue  # already handled
        reader = None
        if target in ek_field_map:
            reader, _ = ek_field_map[target]
        elif target in filter_field_types:
            reader = rust_type_to_reader(filter_field_types[target])
        if reader:
            lines.append(
                f'            "{alias}" => {{ ek.{target} = {reader}; return Some(true); }}'
            )

    lines.append("            _ => { bc.skip_value()?; return Some(true); }")
    lines.append("        }")
    lines.append("    }")
    lines.append("")

    # === EffectKindLocals struct ===
    # Collect ALL unique fields: variants + filter (excluding 'filter' itself)
    all_ek_fields = {}
    boxed_arcstr_fields = set()
    for vname, vfields in variants.items():
        for fname, ftype, aliases in vfields:
            if fname == "filter":
                continue
            if fname not in all_ek_fields:
                all_ek_fields[fname] = ftype
                if ftype == "Option<Box<ArcStr>>":
                    boxed_arcstr_fields.add(fname)
    for fname, ftype, _ in filter_fields:
        if fname not in all_ek_fields:
            all_ek_fields[fname] = ftype
            if ftype == "Option<Box<ArcStr>>":
                boxed_arcstr_fields.add(fname)

    lines.append("/// Accumulator for EffectKind fields during direct decode.")
    lines.append("#[derive(Default)]")
    lines.append("pub(crate) struct EffectKindLocals {")
    for fname, ftype in sorted(all_ek_fields.items()):
        if ftype.startswith("Box<Vec<String>>"):
            lines.append(f"    pub {fname}: {ftype},")
        elif ftype.startswith("Box<Vec<"):
            lines.append(f"    pub {fname}: {ftype},")
        elif ftype == "Vec<String>":
            lines.append(f"    pub {fname}: {ftype},")
        elif ftype.startswith("Option<"):
            lines.append(f"    pub {fname}: {ftype},")
        else:
            lines.append(f"    pub {fname}: Option<{ftype}>,")
    lines.append("}")
    lines.append("")

    # === build_filter function ===
    lines.append("/// Build an EffectFilter from the flat EffectKindLocals.")
    lines.append("/// Lazily allocates: returns None when every filter field is empty,")
    lines.append("/// so effects that carry no targeting/filter data pay no heap box.")
    lines.append("fn build_filter(ek: &EffectKindLocals) -> Option<Box<EffectFilter>> {")
    lines.append("    let f = EffectFilter {")
    for fname, ftype, _ in filter_fields:
        if fname in all_ek_fields:
            lines.append(f"        {fname}: ek.{fname}.clone(),")
        else:
            lines.append(f"        {fname}: Default::default(),")
    lines.append("    };")
    lines.append("    if f == EffectFilter::default() {")
    lines.append("        None")
    lines.append("    } else {")
    lines.append("        Some(Box::new(f))")
    lines.append("    }")
    lines.append("}")
    lines.append("")

    # === build_* functions for each variant ===
    for vname, vfields in sorted(variants.items()):
        lines.append(
            f"fn build_{vname.lower()}(ek: &EffectKindLocals) -> EffectKind {{"
        )
        lines.append(f"    EffectKind::{vname} {{")
        for fname, ftype, _ in vfields:
            if fname == "filter":
                lines.append("        filter: build_filter(ek),")
            elif ftype in ("Box<Vec<String>>", "Vec<String>"):
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
    filter_fields = parse_struct_fields(card_rs, "EffectFilter")

    print(f"EffectKind variants: {len(variants)}")
    for vname, vfields in sorted(variants.items()):
        print(f"  {vname}: {len(vfields)} fields")

    print(f"AbilityEffect: {len(ability_effect_fields)} fields")
    for fname, ftype, aliases in ability_effect_fields:
        alias_str = f" (aliases: {aliases})" if aliases else ""
        print(f"  {fname}: {ftype}{alias_str}")

    print(f"CompoundBranch: {len(compound_fields)} fields")
    for fname, ftype, aliases in compound_fields:
        alias_str = f" (aliases: {aliases})" if aliases else ""
        print(f"  {fname}: {ftype}{alias_str}")

    print(f"EffectFilter: {len(filter_fields)} fields")

    code = generate_decoder(
        variants, ability_effect_fields, compound_fields, filter_fields
    )

    with open(OUT_RS, "w", encoding="utf-8") as f:
        f.write(code)

    print(f"\nGenerated: {OUT_RS} ({len(code)} bytes)")


if __name__ == "__main__":
    main()
