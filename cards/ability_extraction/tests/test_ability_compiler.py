"""
tests/test_ability_compiler.py

The compiler's WIRE CONTRACT: the tables in `cards/compile_abilities.py` that
decide whether a compiled ability is decodable at all.

This is the one place where a name can be *silently* wrong. A condition compiled
with the wrong variant tag decodes into a different `Condition` variant; an effect
whose `action` is missing from `ACTION_TO_VARIANT_TAG` is written as a plain
object and its contents are dropped. Neither shows up as an error at load time,
and neither is caught by the engine's decode-fallback audit, because an in-range
tag is not a fallback. The corpus is large enough that no gameplay test covers
all of it, so the guard has to be structural and corpus-wide.

The tests below are ordered by what they protect:

    1. the ABI between the compiler and the generated decoders
    2. the tables against the corpus they classify
    3. the encoder against itself (it must not rewrite its input)

Run: python cards/ability_extraction/tests/test_ability_compiler.py
"""

import json
import os
import re
import struct
import sys
from pathlib import Path

sys.path.insert(0, os.path.dirname(__file__))
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))

import compile_abilities as compiler
from compile_abilities import (
    ACTION_TO_VARIANT_TAG,
    CARD_RS,
    COND_TO_VARIANT_TAG,
    COST_TYPE_TO_ACTION,
    ENUMS_RS,
    UNSUPPORTED_CONDITION_VARIANT,
    compile_all,
)

ABILITIES_JSON = Path(__file__).resolve().parents[2] / "abilities.json"

#: Wire tags, mirroring the `TAG_*` consts in `engine/src/ability/vm.rs`.
TAG_NULL = 0x00
TAG_FALSE = 0x01
TAG_TRUE = 0x02
TAG_I64 = 0x03
TAG_F64 = 0x04
TAG_STR = 0x06
TAG_ARRAY = 0x07
TAG_OBJECT = 0x08
TAG_OBJECT_VARIANT = 0x09

_CORPUS = None


def corpus():
    """The parsed corpus, read once and deep-copied per test that mutates."""
    global _CORPUS
    if _CORPUS is None:
        _CORPUS = json.loads(ABILITIES_JSON.read_text(encoding="utf-8"))
    return _CORPUS


def fresh_corpus():
    """A private deep copy, for the tests that compile and must not perturb it."""
    return json.loads(json.dumps(corpus()["unique_abilities"]))


_COMPILED = None


def compiled_corpus():
    """The corpus compiled once, shared by every test that only reads the result.

    `compile_all` interns 6030 strings and rewrites the blob, which is not free;
    nine recompiles is nine times the work and nine times the progress output for
    the same answer. Tests that care about the compiler's side effects compile
    their own copy instead.
    """
    global _COMPILED
    if _COMPILED is None:
        _COMPILED = compile_all(fresh_corpus())
    return _COMPILED


def walk_nodes(obj, in_effect_vec=False, in_condition=False):
    """Yield `(node, is_condition_position)` for every dict in the tree.

    Condition position is the compiler's own rule, not a paraphrase of it: a
    child's flags depend ONLY on its key, and are not inherited from the parent.
    That distinction is load-bearing. `condition.trigger_event` is one level down
    from a condition, but `trigger_event` is not a condition key, so its
    `{"type": "appearance"}` is a `TriggerEvent`, not a `Condition::Appearance`.
    Inheriting the flag instead makes every trigger event look like an
    unclassifiable condition.

    The flags do pass through a list unchanged, which is how `conditions[i]`
    stays a condition.
    """
    if isinstance(obj, dict):
        yield obj, in_condition
        for k, v in obj.items():
            yield from walk_nodes(
                v, k in compiler.EFFECT_LIST_KEYS, k in compiler.CONDITION_KEYS
            )
    elif isinstance(obj, list):
        for item in obj:
            yield from walk_nodes(item, in_effect_vec, in_condition)


# ─── 1. The ABI: compiler <-> generated decoders ─────────────────────────────
#
# `generate_condition_decoder.py` emits `0 => build_compound, 1 => ...` in the
# order the `Condition` enum is declared in core/card/condition.rs, and
# `decode_condition_direct` dispatches on that byte. The compiler writes the same
# byte. These tests assert the two agree by reading condition.rs, so reordering
# the enum cannot leave the compiler writing yesterday's numbers.


def card_rs_condition_variants():
    """`[(variant_name, [wire names])]` in declaration order, read from condition.rs."""
    text = CARD_RS.read_text(encoding="utf-8")
    m = re.search(r"pub enum Condition\s*\{", text)
    assert m, f"no `pub enum Condition` in {CARD_RS}"
    depth, pos = 1, m.end()
    while depth > 0:
        if text[pos] == "{":
            depth += 1
        elif text[pos] == "}":
            depth -= 1
        pos += 1
    body = text[m.end() : pos - 1]

    variants, attr, attr_depth, depth = [], [], 0, 0
    for raw in body.split("\n"):
        line = raw.strip()
        if depth > 0:
            depth += line.count("{") - line.count("}")
            continue
        if attr_depth > 0:
            attr.append(line)
            attr_depth += line.count("[") - line.count("]")
            continue
        if line.startswith("#["):
            attr = [line]
            attr_depth = line.count("[") - line.count("]")
            continue
        v = re.match(r"(\w+)\s*\{", line)
        if not v:
            continue
        depth = 1
        block = " ".join(attr)
        rename = re.search(r'rename\s*=\s*"([^"]+)"', block)
        names = [rename.group(1) if rename else v.group(1)]
        names += re.findall(r'alias\s*=\s*"([^"]+)"', block)
        variants.append((v.group(1), names))
        attr, attr_depth = [], 0
    return variants


def test_condition_variant_index_is_the_enum_position():
    """The tag must be the variant's INDEX, not any stable id of our own.

    This is the whole reason the table is derived. A hand-written table is
    correct until someone inserts a variant mid-enum, at which point every
    condition from that point on decodes as the wrong variant and no test fails:
    the tag is in range, so the decode audit never sees a fallback.
    """
    variants = card_rs_condition_variants()
    assert len(variants) == 21, (
        f"expected 21 Condition variants, found {len(variants)}: "
        f"{[v for v, _ in variants]} — if this is an intended change, the "
        "condition wire tags were re-derived with it, which is the point"
    )
    expected = {
        name: index for index, (_, names) in enumerate(variants) for name in names
    }
    assert COND_TO_VARIANT_TAG == expected, (
        "COND_TO_VARIANT_TAG does not match the serde tags in condition.rs: "
        f"missing={sorted(set(expected) - set(COND_TO_VARIANT_TAG))} "
        f"extra={sorted(set(COND_TO_VARIANT_TAG) - set(expected))} "
        f"wrong={ {k: (COND_TO_VARIANT_TAG[k], expected[k]) for k in expected if k in COND_TO_VARIANT_TAG and COND_TO_VARIANT_TAG[k] != expected[k]} }"
    )


def test_every_condition_wire_name_has_a_tag():
    """No name in the enum may be left unclassified.

    An unclassified name cannot happen while the table is derived — that is the
    point of deriving it — so this is the assertion that fails first if the
    derivation silently drops a variant (a parse change in condition.rs, a renamed
    variant) rather than quietly compiling conditions to a wrong index.
    """
    variants = card_rs_condition_variants()
    total_names = sum(len(names) for _, names in variants)
    assert len(COND_TO_VARIANT_TAG) == total_names, (
        f"{len(COND_TO_VARIANT_TAG)} classified names vs {total_names} in condition.rs — "
        "a multi-line `#[cfg_attr(...)]` block probably lost its serde attributes"
    )
    for index, (variant, names) in enumerate(variants):
        for name in names:
            assert COND_TO_VARIANT_TAG.get(name) == index, (
                f"{name!r} (Condition::{variant}) must be tag {index}"
            )


def test_unsupported_variant_is_the_fail_closed_one():
    """`custom` is the name a parser failure emits; it must land on Unsupported.

    The point of the fail-closed variant is that a condition the compiler cannot
    classify decodes to something that never fires, rather than to a
    plausible-looking wrong condition that silently changes a card. `custom` is
    the only name that means "I could not parse this", so it is pinned here
    explicitly rather than left to whatever index `Unsupported` happens to have.
    """
    assert COND_TO_VARIANT_TAG["custom"] == UNSUPPORTED_CONDITION_VARIANT
    unsupported = [
        i for i, (variant, _) in enumerate(card_rs_condition_variants())
        if variant == "Unsupported"
    ]
    assert unsupported == [UNSUPPORTED_CONDITION_VARIANT], (
        "UNSUPPORTED_CONDITION_VARIANT must be Condition::Unsupported's index, "
        f"which is {unsupported}"
    )


def test_condition_aliases_share_their_canonical_variant():
    """Aliases are not a second spelling of the type — they ARE the type.

    `or_condition` is a `Compound` with `operator: "or"`; `not_moved` is a
    `Movement` with `movement: "not_moved"`. If an alias ever pointed at a
    different variant, the compiler would drop the operator/movement field the
    alias implies and the condition would read as the default conjunction or as
    the opposite movement.
    """
    for alias, canonical in (
        ("or_condition", "compound"),
        ("location_condition", "card_count_condition"),
        ("both_condition", "comparison_condition"),
        ("all_cost_comparison_condition", "comparison_condition"),
        ("highest_cost_on_stage_condition", "comparison_condition"),
        ("not_moved", "movement_condition"),
        ("has_moved", "movement_condition"),
        ("energy_state_condition", "state_condition"),
        ("state_change_condition", "state_condition"),
        ("card_blade_condition", "resource_condition"),
        ("position_change_condition", "choice_condition"),
        ("action_success_condition", "otherwise_condition"),
    ):
        assert COND_TO_VARIANT_TAG[alias] == COND_TO_VARIANT_TAG[canonical], (
            f"{alias!r} must decode as {canonical!r}"
        )


def action_type_wire_strings():
    """The set of action strings `ActionType::from_str` accepts, from enums.rs.

    An effect's decoded action comes from the interned string, not from the family
    tag, so this table — not `ACTION_TO_VARIANT_TAG` — decides whether an effect is
    executable.
    """
    text = ENUMS_RS.read_text(encoding="utf-8")
    m = re.search(r"wire_tables! \{(.*?)\n    \}", text, re.DOTALL)
    assert m, f"no `wire_tables!` block in {ENUMS_RS}"
    return set(re.findall(r'=>\s*"([^"]+)"', m.group(1)))


def test_action_type_wire_strings_are_readable():
    """The executor's vocabulary must be readable and non-trivial.

    Everything below leans on this table, so it gets its own assertion rather
    than an `assert` buried in a helper: if the regex stopped matching, the
    coverage tests would compare against an empty set and pass.
    """
    strings = action_type_wire_strings()
    assert len(strings) > 50, f"only {len(strings)} action wire strings parsed from {ENUMS_RS}"
    for required in ("move_cards", "gain_resource", "sequential", "draw_card"):
        assert required in strings, f"{required} missing from the ActionType wire table"


# ─── 2. The tables against the corpus ─────────────────────────────────────────
#
# A name in the corpus that no table lists still compiles. That is deliberate —
# see the compiler docstring — but each one degrades an ability into a no-op, so
# the set must be empty and must stay empty as cards are added.


def test_no_corpus_name_is_unclassified():
    """Compile the whole corpus and require an empty unclassified report.

    This is the end-to-end version of the three vocabulary tests below, run
    through the real encoder, so it also catches a name reached by a key this
    test's walker does not know about.
    """
    compiled = compiled_corpus()
    assert compiled.unclassified.is_empty, (
        "the compiler had no table entry for these names, so they compiled to a "
        f"fail-closed variant: {compiled.unclassified}"
    )


def test_every_corpus_condition_type_is_classified():
    unclassified = {
        node["type"]
        for entry in corpus()["unique_abilities"]
        for k in ("cost", "effect")
        if isinstance(entry.get(k), dict)
        for node, is_condition in walk_nodes(
            entry[k], k in compiler.EFFECT_LIST_KEYS, k in compiler.CONDITION_KEYS
        )
        if is_condition and isinstance(node.get("type"), str)
        and node["type"] not in COND_TO_VARIANT_TAG
    }
    assert not unclassified, (
        f"condition types with no wire tag: {sorted(unclassified)} — they compile "
        "to Condition::Unsupported, i.e. a condition that never fires"
    )


def test_every_corpus_action_is_classified_and_executable():
    """Two tables, because an action has to clear both.

    `ACTION_TO_VARIANT_TAG` decides whether the object is written as something
    the decoder will read. `ActionType`'s wire table decides whether the decoded
    action is one the engine can run. A name can pass the first and fail the
    second — the compiler table carries legacy spellings like `draw` and `look`
    that no longer reach the parser — and the result is an effect that compiles,
    decodes, and then logs `unknown action string` and executes as `Custom`.
    """
    executable = action_type_wire_strings()
    unknown_table, unknown_engine = set(), set()
    for entry in corpus()["unique_abilities"]:
        for k in ("cost", "effect"):
            if not isinstance(entry.get(k), dict):
                continue
            for node, _ in walk_nodes(
                entry[k], k in compiler.EFFECT_LIST_KEYS, k in compiler.CONDITION_KEYS
            ):
                action = node.get("action")
                if not isinstance(action, str):
                    continue
                if action not in ACTION_TO_VARIANT_TAG:
                    unknown_table.add(action)
                if action and action not in executable:
                    unknown_engine.add(action)
    assert not unknown_table, (
        f"actions missing from ACTION_TO_VARIANT_TAG: {sorted(unknown_table)} — "
        "each is written as a plain object and dropped at decode"
    )
    assert not unknown_engine, (
        f"actions the engine cannot execute: {sorted(unknown_engine)} — "
        "ActionType::from_str returns None, the action stays Custom, and the "
        "effect does nothing"
    )


def test_every_corpus_cost_type_is_normalized():
    """A cost with an unlisted `type` is never charged, and looks charged.

    `bytecode_cost_matches_json` (Rust) compares only whether a cost object
    exists, so an unlisted cost type passes it: the cost decodes, the card pays
    nothing, and no test notices.
    """
    unclassified = {
        entry["cost"]["type"]
        for entry in corpus()["unique_abilities"]
        if isinstance(entry.get("cost"), dict)
        and isinstance(entry["cost"].get("type"), str)
        and entry["cost"]["type"] not in COST_TYPE_TO_ACTION
    }
    assert not unclassified, (
        f"cost types not normalized to an action: {sorted(unclassified)} — the "
        "cost decodes as ActionType::Custom and is never charged"
    )


def test_action_table_entries_are_all_executable_or_documented():
    """Every name the compiler accepts must be one the engine can run.

    The corpus test above only sees names cards actually use. This one covers the
    rest of the table, which is where forward-compatible spellings live, so a
    stale entry cannot sit there looking validated until the day a card emits it.
    `""` is the one deliberate exception: it is how the compiler encodes a
    text-only effect, and `decode_effect_field` maps it to `ActionType::Custom`.
    """
    executable = action_type_wire_strings()
    assert "" in ACTION_TO_VARIANT_TAG, "the text-only effect encoding is required"
    stale = {a for a in ACTION_TO_VARIANT_TAG if a and a not in executable}
    assert not stale, (
        f"ACTION_TO_VARIANT_TAG accepts names ActionType cannot decode: "
        f"{sorted(stale)} — remove them or add the wire string to enums.rs"
    )


# ─── 3. The encoder against itself ────────────────────────────────────────────


def test_compile_all_reads_only():
    """`compile_all` must not rewrite the structures it is given.

    It used to normalize costs into the effect shape in place, which made a
    second call over the same structures emit different bytes: `type` was
    already gone, so every condition took the fail-closed variant and every
    cost lost its action. The committed blob was always built from a freshly
    loaded JSON, so nothing caught it — but any test or tool that compiled twice
    wrote a blob where nothing worked.
    """
    data = fresh_corpus()
    before = json.dumps(data, ensure_ascii=False, sort_keys=True)
    compile_all(data)
    assert json.dumps(data, ensure_ascii=False, sort_keys=True) == before, (
        "compile_all mutated its input; a second compile of these structures "
        "would produce a different blob"
    )


def test_compile_all_is_idempotent():
    """Two compiles of the same input must be the same bytes.

    This is the check that fails on the mutation above even when the first
    compile's output is discarded — it compares the two results instead of
    trusting the input to be untouched.
    """
    data = fresh_corpus()
    first = compile_all(data)
    second = compile_all(data)
    assert first.bytecode == second.bytecode
    assert first.offsets == second.offsets
    assert first.strings == second.strings
    assert first.card_ability_pairs == second.card_ability_pairs


def test_compile_all_does_not_mutate_a_deep_copy_either():
    """The result must be a function of the input VALUE, not of the object.

    The same property as `test_compile_all_is_idempotent`, from the other side:
    two independently loaded copies of the corpus must compile to the same bytes,
    so nothing carries over from a previous compile.
    """
    one = compile_all(fresh_corpus())
    two = compile_all(fresh_corpus())
    assert one.bytecode == two.bytecode


# ─── 4. The encoder's output, read back ───────────────────────────────────────
#
# The corpus tests above ask "did every name classify". These ask the question
# that actually matters at load time: does the blob the engine reads back say the
# same thing the JSON said. A decoder written here, against the format
# description in the compiler's docstring rather than against its encoder, is the
# only way to catch a misaligned length or a dropped field — an encoder bug is
# invisible to a test that only compares encoder output to itself.


class Reader:
    """Cursor over one ability's slice, following the documented wire format."""

    def __init__(self, data, strings):
        self.b = data
        self.pos = 0
        self.strings = strings

    def u8(self):
        v = self.b[self.pos]
        self.pos += 1
        return v

    def idx(self):
        """A u8 string index, or `0xFE` + u16 for the rarely-referenced tail."""
        if self.b[self.pos] == 0xFE:
            v = int.from_bytes(self.b[self.pos + 1 : self.pos + 3], "little")
            self.pos += 3
        else:
            v = self.b[self.pos]
            self.pos += 1
        return v

    def length(self):
        if self.b[self.pos] == 0xFE:
            v = int.from_bytes(self.b[self.pos + 1 : self.pos + 3], "little")
            self.pos += 3
        else:
            v = self.b[self.pos]
            self.pos += 1
        return v

    def integer_payload(self):
        """Consume a `TAG_I64` payload and return the value.

        The widths the encoder emits: one byte up to 0xFD, `0xFE` + u16, then
        `0xFF` + i32 for negatives (`cost_offset` is signed).
        """
        b = self.u8()
        if b <= 0xFD:
            return b
        if b == 0xFE:
            v = int.from_bytes(self.b[self.pos : self.pos + 2], "little")
            self.pos += 2
            return v
        if b == 0xFF:
            v = int.from_bytes(self.b[self.pos : self.pos + 4], "little", signed=True)
            self.pos += 4
            return v
        v = int.from_bytes(self.b[self.pos : self.pos + 8], "little", signed=True)
        self.pos += 8
        return v

    def value(self):
        """Decode one value; objects come back as `(variant, {key: value})`.

        `variant` is None for a plain object and the tag byte for a variant, which
        is the distinction the whole format turns on.
        """
        tag = self.u8()
        if tag == TAG_NULL:
            return None
        if tag == TAG_FALSE:
            return False
        if tag == TAG_TRUE:
            return True
        if tag == TAG_I64:
            return self.integer_payload()
        if tag == TAG_F64:
            v = struct.unpack("<d", self.b[self.pos : self.pos + 8])[0]
            self.pos += 8
            return v
        if tag == TAG_STR:
            return self.strings[self.idx()]
        if tag == TAG_ARRAY:
            return [self.value() for _ in range(self.length())]
        if tag in (TAG_OBJECT, TAG_OBJECT_VARIANT):
            variant = None
            if tag == TAG_OBJECT_VARIANT:
                variant = self.u8()
            out = {}
            for _ in range(self.length()):
                key = self.strings[self.idx()]
                out[key] = self.value()
            return variant, out
        raise AssertionError(f"unknown wire tag 0x{tag:02x} at {self.pos - 1}")


def condition_tags(value, found, path="", in_condition=False, from_blob=False):
    """Every condition node's wire tag, in document order, appended to `found`.

    Walks a JSON tree (`from_blob=False`) or a decoded blob tree (`from_blob=True`)
    using the compiler's own descent rule — a child's condition flag comes only
    from its key — so the two sides are comparable position by position. Two
    independently written walks would differ exactly where the comparison is
    supposed to be checking something.
    """
    if isinstance(value, tuple) and len(value) == 2 and isinstance(value[1], dict):
        variant, node = value
        if from_blob and in_condition and variant is not None:
            assert "type" not in node, (
                f"{path}: a condition kept its `type` key; the tag already "
                "carries the type and the decoder would read it twice"
            )
            found.append((path, variant))
    elif isinstance(value, dict):
        node = value
        if not from_blob and in_condition and isinstance(node.get("type"), str):
            found.append((path, COND_TO_VARIANT_TAG[node["type"]]))
    else:
        return
    for k, v in node.items():
        condition_tags(v, found, f"{path}.{k}", k in compiler.CONDITION_KEYS, from_blob)


def decode_all(compiled, entries):
    """`[(index, card_no, node)]` for every ability, node decoded as a tuple."""
    trees = []
    for i, entry in enumerate(entries):
        start, end = compiled.offsets[i], compiled.offsets[i + 1]
        reader = Reader(compiled.bytecode[start:end], compiled.strings)
        node = reader.value()
        assert reader.pos == end - start, (
            f"ability {i} ({entry['cards'][0]}): slice is {end - start} bytes, "
            f"decoded {reader.pos}"
        )
        trees.append((i, entry["cards"][0], node))
    return trees


def entry_without_index(entry):
    """The entry as the blob holds it: same keys, same order, `cards` dropped."""
    return {k: v for k, v in entry.items() if k not in compiler.SKIP_KEYS}


def test_every_ability_slice_decodes_exactly():
    """Each ability's slice must be one plain object that ends where it starts.

    `offsets` is what the engine seeks with, so a slice that decodes to fewer
    bytes than it holds means the next ability's index is wrong for every
    ability after it, and one that overruns means the reader walks into the next
    slice. Both surface as an arbitrary ability decoding to nonsense, which is
    why the assertion is on the byte count rather than on the decoded content.
    """
    compiled = compiled_corpus()
    entries = fresh_corpus()
    assert len(compiled.offsets) == len(entries) + 1, (
        "one more offset than abilities: the engine seeks by prefix sum"
    )
    for i, card, (variant, obj) in decode_all(compiled, entries):
        assert variant is None, f"ability {i} ({card}): the entry must be a plain object"
        assert set(obj) == set(entries[i]) - compiler.SKIP_KEYS, (
            f"ability {i} ({card}): keys {sorted(obj)} != "
            f"{sorted(set(entries[i]) - compiler.SKIP_KEYS)}"
        )


def test_every_corpus_condition_compiles_to_its_own_variant_tag():
    """The tag in the blob must be the one the decoder will build from.

    Two independent walks of the same tree, matched up path by path: for every
    condition node the corpus says `type: X`, and the blob must carry
    `COND_TO_VARIANT_TAG[X]` at the same position, with the `type` key itself
    gone. This is the assertion that would have caught a reordering of the
    `Condition` enum, and it covers every ability in the corpus rather than the
    handful any gameplay test touches.

    Cost and effect objects are also tagged, by effect family rather than by
    variant, so the comparison is restricted to condition position — otherwise
    every ability with a `cost` would report a spurious mismatch.
    """
    compiled = compiled_corpus()
    entries = fresh_corpus()

    checked = 0
    for i, card, node in decode_all(compiled, entries):
        want, got = [], []
        condition_tags(entry_without_index(entries[i]), want)
        condition_tags(node, got, from_blob=True)
        assert got == want, (
            f"ability {i} ({card}):\n  compiled: {got}\n  expected: {want}"
        )
        checked += len(want)
    assert checked > 0, "no condition nodes found — the walker found nothing"


def test_card_ability_pairs_cover_every_card_in_every_ability():
    """`CARD_ABILITY_PAIRS` is the only route from a card to its abilities.

    `every_card_ability_index_is_valid` (Rust) checks that the indices it finds
    are IN range, which a pair that was never emitted satisfies trivially. This
    checks the other direction: every card in the corpus appears, and it points at
    exactly the abilities that list it — in the corpus's order, since
    `build_abilities_map_shared` preserves it and trigger priority follows it.

    A card whose pair is missing simply has no abilities: it never triggers,
    never offers, and never shows up as a failure anywhere else in the suite.
    A card mapped to the wrong index is worse — it runs a different card's
    ability.
    """
    compiled = compiled_corpus()
    entries = fresh_corpus()

    got = {}
    for str_idx, ability_idx in compiled.card_ability_pairs:
        got.setdefault(compiled.strings[str_idx], []).append(ability_idx)

    want = {}
    for i, entry in enumerate(entries):
        for card_entry in entry.get("cards", []):
            card = card_entry.split(" | ")[0] if " | " in card_entry else card_entry
            want.setdefault(card, []).append(i)

    assert got == want, (
        f"card->ability index differs: missing={sorted(set(want) - set(got))[:10]} "
        f"extra={sorted(set(got) - set(want))[:10]} "
        f"mismatched={[c for c in set(want) & set(got) if want[c] != got[c]][:10]}"
    )
    assert any(len(v) > 1 for v in want.values()), (
        "no card owns more than one ability; the multi-ability case is untested"
    )


if __name__ == "__main__":
    from _runner import run_module

    sys.exit(run_module(globals()))