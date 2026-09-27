"""Type facts shared by the decoder generators.

Both `generate_condition_decoder.py` and `generate_effect_decoder.py` decide
whether a field read out of the decode accumulator needs an explicit
`.clone()`. That answer has to be the same in both, or the same field type
compiles clean in one generated file and draws a warning in the other — which
is exactly what happened while each script kept its own list.
"""

# Types that are `Copy`, so reading one out of an accumulator already copies it
# and an emitted `.clone()` is pure noise. Only the BARE type names belong here:
# `is_copy_type` peels any `Option<..>` wrappers, so one entry covers both `T`
# and `Option<T>`.
#
# Enumerating the `Option<..>` spellings alongside the bare types (the shape
# this replaced) made every new scalar a two-line change and silently missed
# ones such as `Option<Zone>`, which then reached the generated file.
COPY_TYPES = frozenset(
    {
        # Scalars.
        "bool",
        "u8",
        "u16",
        "u32",
        "u64",
        "i8",
        "i16",
        "i32",
        "i64",
        "usize",
        "isize",
        # Engine enums that are plain data with no heap members.
        "Operator",
        "Operation",
        "PlacementOrder",
        "Zone",
        "Phase",
        "CardState",
        "EffectState",
        "AbilityFilter",
        "ConditionCardType",
        "CardProperty",
        "ComparisonType",
        "ComparisonTarget",
    }
)


def is_copy_type(ftype: str) -> bool:
    """Is a field of this type `Copy`, so reading it needs no `.clone()`?

    Peels `Option<..>` wrappers before testing, so listing `bool` also covers
    `Option<bool>` rather than requiring both to be spelled out.
    """
    inner = ftype.strip()
    while inner.startswith("Option<") and inner.endswith(">"):
        inner = inner[len("Option<") : -1]
    return inner in COPY_TYPES
