"""Generator for `src/ability/vm_gen.rs`.

Emits a **serde-free** decoder that reads the binary-JSON bytecode (see
compile_abilities.py) directly into the engine's `Ability` / `AbilityEffect` /
`EffectKind` / `Condition` structs. No `serde_json::Value`, no `from_value`, no
`populate_from_json` on the hot path.

The decoder is generated from the *actual* struct/enum definitions in
`engine/src/core/card.rs` and `engine/src/ability/types.rs`, so it stays in
lock-step with the engine automatically: add a field/variant/action and
regenerating rewires the decoder. Correctness is guaranteed by
`bytecode_deep_compare_test` (the decoded `Ability` must be byte-identical to
the JSON loader's `from_value` + `populate_from_json` result).
"""

import re
from pathlib import Path


def _read_file(p):
    try:
        return Path(p).read_text(encoding="utf-8")
    except OSError:
        return ""


def parse_types():
    """Parse `pub struct X {..}` and `pub enum X {..}` definitions into a
    registry: name -> dict(kind='struct'|'enum', fields=[(name,type)],
    variants=[(name, [(fname,ftype)])], renames={type_str: variant} ...)."""
    roots = [
        Path(__file__).parent.parent / "engine" / "src" / "core" / "card.rs",
        Path(__file__).parent.parent / "engine" / "src" / "ability" / "types.rs",
    ]
    src = "\n".join(_read_file(r) for r in roots)
    types = {}

    # Find top-level struct/enum blocks (brace-matched).
    # We scan for "pub struct NAME {" / "pub enum NAME {".
    idx = 0
    pat = re.compile(r"pub\s+(struct|enum)\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{")
    for m in pat.finditer(src):
        kind = m.group(1)
        name = m.group(2)
        start = m.end() - 1  # at '{'
        depth = 0
        i = start
        while i < len(src):
            c = src[i]
            if c == "{":
                depth += 1
            elif c == "}":
                depth -= 1
                if depth == 0:
                    break
            i += 1
        body = src[start + 1 : i]

        if kind == "struct":
            fields = []
            # Each field: optional attrs then `name: Type,`
            for line in body.split("\n"):
                # strip attribute lines
                if line.strip().startswith("#["):
                    continue
                fm = re.match(r"\s*pub\s+([A-Za-z_][A-Za-z0-9_]*)\s*:\s*(.+?),?\s*$", line)
                if fm:
                    fname = fm.group(1)
                    ftype = fm.group(2).strip()
                    # skip tuple-struct marker
                    if fname == "pub":
                        continue
                    fields.append((fname, ftype))
            types[name] = {"kind": "struct", "fields": fields}
        else:  # enum
            variants = []
            renames = {}  # serde rename/alias -> variant name
            # walk variant entries
            vpat = re.compile(r"#\[serde\(([^)]*)\)\]\s*([A-Za-z_][A-Za-z0-9_]*)\s*(\{)?")
            pos = 0
            for vm in vpat.finditer(body):
                attrs = vm.group(1)
                vname = vm.group(2)
                has_fields = vm.group(3) == "{"
                # parse renames/aliases
                for rm in re.finditer(r"(?:rename|alias)\s*=\s*\"([^\"]+)\"", attrs):
                    renames[rm.group(1)] = vname
                if has_fields:
                    # find matching brace block
                    bstart = body.find("{", vm.end() - 1)
                    depth = 0
                    j = bstart
                    while j < len(body):
                        c = body[j]
                        if c == "{":
                            depth += 1
                        elif c == "}":
                            depth -= 1
                            if depth == 0:
                                break
                        j += 1
                    vbody = body[bstart + 1 : j]
                    vfields = []
                    for line in vbody.split("\n"):
                        if line.strip().startswith("#["):
                            continue
                        if line.strip().startswith("},") or line.strip() == "}":
                            continue
                        fm = re.match(r"\s*pub\s+([A-Za-z_][A-Za-z0-9_]*)\s*:\s*(.+?),?\s*$", line)
                        if fm:
                            vfields.append((fm.group(1), fm.group(2).strip()))
                    variants.append((vname, vfields))
                else:
                    variants.append((vname, None))
                    # also register alias for unit variant name itself
            types[name] = {"kind": "enum", "variants": variants, "renames": renames}
    return types


def parse_action_map():
    """Parse `kind_from_action`'s action->variant mapping from card.rs."""
    src = _read_file(Path(__file__).parent.parent / "engine" / "src" / "core" / "card.rs")
    m = re.search(r"pub\(crate\) fn kind_from_action\(.*?\n(.*?)\n    \}", src, re.S)
    body = m.group(1) if m else ""
    # collect `"action" => "Variant",` and multi mappings
    mapping = {}  # action (lower) -> variant
    # Each match arm: "a" | "b" | ... => "Variant",
    for am in re.finditer(r"\"([^\"]+)\"(?:\s*\|\s*\"([^\"]+)\")*\s*=>\s*\"([^\"]+)\",", body):
        actions = [am.group(1)] + ([am.group(2)] if am.group(2) else [])
        variant = am.group(3)
        for a in actions:
            mapping[a.lower()] = variant
    # handle `"": Variant` form
    for am in re.finditer(r"\"\"\s*=>\s*\"([^\"]+)\",", body):
        mapping[""] = am.group(1)
    return mapping
