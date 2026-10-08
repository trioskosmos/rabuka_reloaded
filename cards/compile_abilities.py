"""Auto-compiler: scans abilities.json, discovers field types, generates bytecode + Rust decoder.

# THE WIRE CONTRACT

An ability is serialized as a *tagged tree*: a binary encoding of the same JSON
document the text loader would produce, with every string (object keys and string
values) interned into one table and referenced by a 2-byte index. Three tags
matter when reading a compiled blob back:

    0x08 TAG_OBJECT         a plain object — keys then values, no payload byte
    0x09 TAG_OBJECT_VARIANT a typed object — one extra payload byte, the
                            *variant tag*, that selects which decoder runs

`TAG_OBJECT_VARIANT` is what the Rust decoder can act on. A `TAG_OBJECT` is
skipped or rejected by every reader that wanted a Condition or an AbilityEffect
(`vm.rs`), so **membership in the tables below is load-bearing and the numbers
are not**: an object whose name is missing from a table is written as
`TAG_OBJECT` and its contents are dropped at decode time.

The numbers carry meaning in exactly one place. `decode_condition_direct`
(`generate_condition_decoder.py`) dispatches on the variant byte by *position*:
the generator emits `0 => build_compound, 1 => build_location, ...` in the order
the `Condition` enum is DECLARED in `engine/src/core/card.rs`. So a condition's
variant tag is its index in that enum, and the compiler and the generator must
agree on that ordering or every condition decodes into the wrong variant —
silently, because an in-range tag is never reported by the decode audit.

That is why `COND_TO_VARIANT_TAG` below is DERIVED from `card.rs` rather than
written out. It used to be a hand transcription of the serde tags, correct as of
the last review and unguarded: inserting a variant anywhere but the end of the
enum would have retagged every later condition with no test failing. The
aliases (several wire names share one variant) are read from the same
`#[serde(alias = ...)]` attributes, so they cannot drift either.

`ACTION_TO_VARIANT_TAG` is a family grouping, not a dispatch table, because
`decode_ability_effect_direct` ignores its `_variant` and rebuilds the effect
from the interned `action` string. The values are kept because they make the
encoding self-describing when a blob is read by eye, and the membership is what
keeps an effect on the `TAG_OBJECT_VARIANT` path.

`tests/test_ability_compiler.py` guards all of this: the derived table against
`card.rs`, the corpus against the tables, and the encoder against itself.
"""

import json
import os
import re
import struct
import hashlib
import zlib
from pathlib import Path
from typing import NamedTuple

from build_lib import (
    StringTable,
    write_len,
    read_len,
    compress_with_header,
    write_generation_manifest,
    write_string_blob,
    delta_encode_offsets,
    sha256_short,
)

CARDS_DIR = Path(__file__).resolve().parent
ENGINE_DIR = CARDS_DIR.parent / "engine"
CARD_RS = ENGINE_DIR / "src" / "core" / "card" / "condition.rs"
ENUMS_RS = ENGINE_DIR / "src" / "ability" / "enums.rs"


class BC:
    def __init__(self):
        self.data = bytearray()

    def u8(self, v):
        self.data.append(v & 0xFF)

    def u16(self, v):
        self.data.extend(struct.pack("<H", v))

    def __len__(self):
        return len(self.data)

    def __bytes__(self):
        return bytes(self.data)


def _condition_variant_tags() -> dict[str, int]:
    """`{"wire_name": variant_index}` for the serde-tagged `Condition` enum.

    Reads `engine/src/core/card.rs` and returns, for every variant, the index the
    condition decoder generator will emit for it — the variant's POSITION in the
    declaration. A variant's accepted wire names are its `#[serde(rename = ...)]`
    (or its Rust name when it has none) plus every `#[serde(alias = ...)]`.

    This is the one place the condition wire vocabulary is defined. See the module
    docstring for why it is derived rather than transcribed.
    """
    text = CARD_RS.read_text(encoding="utf-8")
    m = re.search(r"pub enum Condition\s*\{", text)
    if m is None:
        raise SystemExit(
            f"could not find `pub enum Condition` in {CARD_RS}. The condition wire "
            "tags are derived from it; fix the enum name or this extractor."
        )
    depth, pos = 1, m.end()
    while depth > 0:
        if text[pos] == "{":
            depth += 1
        elif text[pos] == "}":
            depth -= 1
        pos += 1
    body = text[m.end() : pos - 1]

    tags: dict[str, int] = {}
    index = -1
    depth = 0  # brace depth inside the current variant
    attr: list[str] = []
    attr_depth = 0  # bracket depth inside the current `#[...]` block
    for raw in body.split("\n"):
        line = raw.strip()
        if depth > 0:
            depth += line.count("{") - line.count("}")
            continue
        # Between variants: an attribute block or a variant header. The serde
        # attributes are NOT always on the `#[` line — a multi-line
        # `#[cfg_attr(\n feature = ...,\n serde(rename = "...", alias = "...")\n)]`
        # puts them on their own lines — so a whole block is accumulated before
        # it is searched. Reading only the `#[` line silently drops the rename
        # and leaves the variant keyed by its Rust name, which classifies nothing.
        if attr_depth > 0:
            attr.append(line)
            attr_depth += line.count("[") - line.count("]")
            continue
        if line.startswith("#["):
            attr = [line]
            attr_depth = line.count("[") - line.count("]")
            continue
        v = re.match(r"(\w+)\s*\{", line)
        if v is None:
            continue
        index += 1
        depth = 1
        text = " ".join(attr)
        rename = re.search(r'rename\s*=\s*"([^"]+)"', text)
        names = [rename.group(1) if rename else v.group(1)]
        names += re.findall(r'alias\s*=\s*"([^"]+)"', text)
        for name in names:
            tags[name] = index
        attr, attr_depth = [], 0
    return tags


COND_TO_VARIANT_TAG = _condition_variant_tags()

#: The fail-closed variant every unrecognised condition name is compiled to.
#: `Condition::Unsupported` matches no card text by construction, so a condition
#: the compiler cannot classify decodes to something that never fires rather
#: than to a plausible-looking wrong condition.
UNSUPPORTED_CONDITION_VARIANT = COND_TO_VARIANT_TAG["custom"]

#: `action` name -> effect family tag. See the module docstring: only membership
#: is load-bearing, the number is a readable grouping.
#:
#: Every name here must also be a string `ActionType::from_str` accepts, because
#: the decoded action comes from the interned string and the family tag is
#: ignored. The table used to also carry five older spellings — `draw`, `look`,
#: `reveal_effect`, `double_baton_touch`, `set_card_identity_all_regions` — which
#: no card has emitted for some time and which `ActionType` cannot decode. An
#: effect carrying one compiled, decoded, logged `unknown action string`, and
#: executed as `Custom`: a no-op that looked live. They were deleted rather than
#: kept as aliases because an alias here is not a synonym the engine resolves, it
#: is a spelling that produces a broken ability. If a future parser emits one,
#: `test_no_corpus_name_is_unclassified` fails and the right fix is to emit the
#: canonical name (`draw_card`, `look_at`, ...), not to re-add the spelling.
ACTION_TO_VARIANT_TAG = {
    "": 3,
    "move_cards": 1,
    "discard_card": 1,
    "discard_until_count": 1,
    "place_energy_under_member": 1,
    "re_yell": 1,
    "shuffle": 1,
    "play_baton_touch": 1,
    "draw_card": 2,
    "draw_until_count": 2,
    "select": 3,
    "select_cards": 3,
    "select_number": 3,
    "choose_target_player": 3,
    "look_at": 4,
    "reveal": 4,
    "reveal_per_group": 4,
    "reveal_until_live_card": 4,
    "reveal_until_chosen_card": 4,
    "look_and_select": 4,
    "modify_score": 5,
    "modify_required_hearts": 6,
    "modify_required_hearts_global": 6,
    "modify_required_hearts_success": 6,
    "gain_resource": 7,
    "pay_energy": 7,
    "change_state": 8,
    "set_card_identity": 8,
    "gain_ability": 9,
    "gain_ability_from_source": 9,
    "invalidate_ability": 9,
    "suppress_ability_trigger": 9,
    "activate_ability": 9,
    "sequential": 10,
    "sequential_cost": 10,
    "choice": 10,
    "choice_condition": 10,
    "repeat_procedure": 10,
    "conditional_alternative": 10,
    "conditional_on_optional": 10,
    "conditional_on_result": 10,
    "restriction": 11,
    "activation_restriction": 11,
    "modify_limit": 11,
    "all_blade_timing": 11,
    "reduce_live_card_set_limit": 11,
    "position_change": 12,
    "rotation": 12,
    "set_cost": 13,
    "set_cost_to_use": 13,
    "modify_cost": 13,
    "activation_cost": 13,
    "set_blade_type": 13,
    "set_blade_count": 13,
    "set_heart_type": 13,
    "specify_heart_color": 13,
    "choose_required_hearts": 13,
    "perform_yell": 13,
    "modify_yell_count": 13,
    "modify_yell_source": 14,
    # Replacement rules are a state change TO WAIT plus the same filters
    # (card_type, position, heart/blade bound), so they share the change_state
    # opcode. This is not cosmetic: the bytecode rewriter keeps a fixed key set
    # per opcode, and the no-op bucket keeps only text/count/action — under it the
    # filters were silently dropped and the rule could never match a card.
    "replacement_appear_wait": 8,
    # Counting weight: a filter-only action, consumed by the count path rather
    # than by dispatch, so it shares the change_state bucket for the same reason.
    "count_as_multiple": 8,
    "custom": 14,
    "do_nothing": 14,
    "action_by": 14,
    "opponent_action": 14,
}

#: Cost `type` -> the `action` a cost is normalized to.
#:
#: Costs and effects share one decoder, so a cost object is rewritten into the
#: effect shape before it is encoded: `type` becomes `action`, `zone` becomes
#: `source` (the two name the same thing, and `zone` is what the cost parser
#: emits), and a `costs`/`options` list becomes `actions`. A cost `type` absent
#: from this table is encoded as an effect with no action, which the decoder
#: reads as `ActionType::Custom` — a cost that parses, validates, and is never
#: charged. `tests/test_ability_compiler.py` asserts every corpus cost type is
#: listed here.
COST_TYPE_TO_ACTION = {
    "move_cards": "move_cards",
    "pay_energy": "pay_energy",
    "reveal": "reveal",
    "change_state": "change_state",
    "place_energy_under_member": "place_energy_under_member",
    "sequential_cost": "sequential_cost",
    "choice_condition": "choice_condition",
}

#: Keys whose value is a LIST of sub-effects.
EFFECT_LIST_KEYS = ("actions", "options", "effect_steps")

#: Keys whose value is a Condition. A condition is the only object in the tree
#: that is tagged by a variant byte rather than by an `action` string, so
#: reaching one is what switches `enc_val` into condition mode — including for
#: nested lists, which is why `conditions` is in the tuple.
CONDITION_KEYS = (
    "condition",
    "alternative_condition",
    "result_condition",
    "choice_condition",
    "activation_condition_parsed",
    "cause",
    "conditions",
)

#: Entry keys never written to the blob: the `cards` list is not part of the
#: ability, it is the index from card_no to ability index
#: (`CARD_ABILITY_PAIRS`).
SKIP_KEYS = frozenset({"cards"})


class UnclassifiedWireNames(NamedTuple):
    """Corpus names the compiler had no table entry for, grouped by table.

    Compiling an unknown name is not an error — see the module docstring for what
    each one degrades to — but it IS a defect in the tables, so it is collected
    and reported rather than discovered later as an ability that does nothing.
    """

    conditions: list[str]
    actions: list[str]
    cost_types: list[str]

    def deduplicated(self) -> "UnclassifiedWireNames":
        """Sorted, de-duplicated copy — one entry per name, not per occurrence.

        A name missing from a table usually occurs on every card of a shared
        ability, so the raw list would report the same defect hundreds of times
        and bury it.
        """
        return UnclassifiedWireNames(
            conditions=sorted(set(self.conditions)),
            actions=sorted(set(self.actions)),
            cost_types=sorted(set(self.cost_types)),
        )

    @property
    def is_empty(self) -> bool:
        return not (self.conditions or self.actions or self.cost_types)


class CompiledAbilities(NamedTuple):
    """`compile_all`'s output: the blob plus everything generated from it."""

    bytecode: bytes
    offsets: list
    strings: list
    card_ability_pairs: list
    unclassified: UnclassifiedWireNames


def _object_items(v, in_effect_vec: bool, is_condition: bool, unclassified):
    """`(variant_tag_or_None, [(key, value)])` for one object, without mutating `v`.

    `None` means "write a plain `TAG_OBJECT`". A variant tag means "write
    `TAG_OBJECT_VARIANT` plus this byte". The distinction is explicit because tag
    0 is a real variant (`Condition::Compound`) while it is NOT a real effect
    family: an effect whose action is unrecognised gets tag 0 and therefore a
    plain object, so the decoder skips it. Testing the tag for truthiness would
    collapse `Compound` into the skip case and drop every 「AかつB」 clause.

    The pairs are the object's own items with the rewritten keys removed from
    their original slot and the added ones appended in assignment order — the
    exact order the previous in-place rewrite produced. Order matters: the
    string table is interned in first-encounter order, so reordering keys
    reorders `STRINGS` and changes the blob for no semantic reason.

    A condition's tag is its `Condition` variant index and its `type` key is
    dropped: the tag carries the type, and leaving the key in would have the
    decoder read it twice.
    """
    if is_condition:
        t = v.get("type") or ""
        tag = COND_TO_VARIANT_TAG.get(t)
        if tag is None:
            unclassified.conditions.append(t)
            tag = UNSUPPORTED_CONDITION_VARIANT
        added, removed = [], {"type"}
        # `or_condition` is an alias of `compound`, and the engine reads the legs'
        # conjunction from `operator`; the two movement aliases take their
        # `movement` from the name itself.
        if t == "or_condition" and "operator" not in v:
            added.append(("operator", "or"))
        if t in ("has_moved", "not_moved") and "movement" not in v:
            added.append(("movement", t))
        items = [(k, val) for k, val in v.items() if k not in removed]
        return tag, items + added

    added, removed = [], set()
    action = v.get("action")
    if action is None:
        t = v.get("type")
        if t in COST_TYPE_TO_ACTION:
            action = COST_TYPE_TO_ACTION[t]
            added.append(("action", action))
            removed.add("type")
            if "source" not in v and "zone" in v:
                added.append(("source", v["zone"]))
                removed.add("zone")
            if "actions" not in v:
                for k in ("costs", "options"):
                    if k in v:
                        added.append(("actions", v[k]))
                        removed.add(k)
                        break
        elif in_effect_vec:
            # A member of an action list with no action of its own: the decoder
            # reads the empty action as `ActionType::Custom`.
            action = ""
            added.append(("action", action))
    if action is None:
        action = ""
    tag = ACTION_TO_VARIANT_TAG.get(action)
    if tag is None:
        unclassified.actions.append(action)
        tag = None
    items = [(k, val) for k, val in v.items() if k not in removed]
    return tag, items + added


def compile_all(abilities):
    """Store each `unique_abilities[i]` entry as a compact *binary JSON* slice.

    Binary JSON = a tagged tree with all strings (object keys AND string values)
    interned into a single `STRINGS` table and referenced by 2-byte indices.

    `abilities` is only read. Normalizing a cost into the effect shape used to be
    done by rewriting the caller's dicts in place, which made a second call over
    the same structures emit different bytes: `type` was already gone, so every
    condition fell through to the fail-closed variant and every cost lost its
    action. See `tests/test_ability_compiler.py::test_compile_all_reads_only`.
    """
    strings = StringTable()
    unclassified = UnclassifiedWireNames([], [], [])

    def enc_val(v, out: bytearray, in_effect_vec: bool = False, is_condition: bool = False):
        if v is None:
            out.append(0x00)
        elif isinstance(v, bool):
            out.append(0x02 if v else 0x01)
        elif isinstance(v, int):
            out.append(0x03)
            if v < 0:
                out.append(0xFF)
                out.extend(struct.pack("<I", v & 0xFFFFFFFF))
            elif v <= 0xFD:
                out.append(v & 0xFF)
            elif v <= 0xFFFF:
                out.append(0xFE)
                out.extend(struct.pack("<H", v))
            else:
                out.append(0xFF)
                out.extend(struct.pack("<I", v))
        elif isinstance(v, float):
            out.append(0x04)
            out.extend(struct.pack("<d", v))
        elif isinstance(v, str):
            out.append(0x06)
            out.extend(struct.pack("<H", strings.intern(v)))
        elif isinstance(v, list):
            out.append(0x07)
            write_len(out, len(v))
            for item in v:
                enc_val(item, out, in_effect_vec, is_condition)
        elif isinstance(v, dict):
            vtag, items = _object_items(v, in_effect_vec, is_condition, unclassified)
            if vtag is None:
                out.append(0x08)
            else:
                out.append(0x09)
                out.append(vtag)
            write_len(out, len(items))
            for k, val in items:
                out.extend(struct.pack("<H", strings.intern(str(k))))
                enc_val(val, out, k in EFFECT_LIST_KEYS, k in CONDITION_KEYS)
        else:
            out.append(0x00)

    def enc_entry(entry, out: bytearray):
        out.append(0x08)
        write_len(out, sum(1 for k in entry if k not in SKIP_KEYS))
        for k, val in entry.items():
            if k in SKIP_KEYS:
                continue
            out.extend(struct.pack("<H", strings.intern(str(k))))
            enc_val(val, out, k in EFFECT_LIST_KEYS, k in CONDITION_KEYS)
        cost = entry.get("cost")
        if isinstance(cost, dict) and isinstance(cost.get("type"), str):
            t = cost["type"]
            if t not in COST_TYPE_TO_ACTION:
                unclassified.cost_types.append(t)

    card_ability_pairs = []
    for idx, entry in enumerate(abilities):
        for card_entry in entry.get("cards", []):
            card_no = card_entry.split(" | ")[0] if " | " in card_entry else card_entry
            str_idx = strings.intern(card_no)
            card_ability_pairs.append((str_idx, idx))

    offsets, bytecode = [], bytearray()
    for entry in abilities:
        offsets.append(len(bytecode))
        enc_entry(entry, bytecode)
    offsets.append(len(bytecode))

    bytecode, offsets, strings_list, card_ability_pairs = compact_bytecode(
        bytes(bytecode), offsets, strings.get_strings(), card_ability_pairs
    )

    return CompiledAbilities(
        bytecode=bytes(bytecode),
        offsets=offsets,
        strings=strings_list,
        card_ability_pairs=card_ability_pairs,
        unclassified=unclassified.deduplicated(),
    )


def compact_bytecode(bytecode, offsets, strings, card_ability_pairs):
    """Reorder strings by frequency and rewrite bytecode with u8+escape indices."""
    freq = [0] * len(strings)

    def count_one(bc, pos):
        if pos >= len(bc):
            return pos
        tag = bc[pos]
        pos += 1
        if tag in (0x00, 0x01, 0x02):
            return pos
        elif tag == 0x03:
            if pos >= len(bc):
                return len(bc)
            b = bc[pos]
            if b <= 0xFD:
                return pos + 1
            elif b == 0xFE:
                return pos + 3
            elif b == 0xFF:
                return pos + 5
            return pos + 9
        elif tag == 0x04:
            return pos + 8
        elif tag == 0x06:
            if pos + 2 > len(bc):
                return len(bc)
            idx = bc[pos] | (bc[pos + 1] << 8)
            pos += 2
            if idx < len(freq):
                freq[idx] += 1
            return pos
        elif tag == 0x07:
            n, pos = read_len(bc, pos)
            for _ in range(n):
                pos = count_one(bc, pos)
            return pos
        elif tag == 0x08:
            n, pos = read_len(bc, pos)
            for _ in range(n):
                if pos + 2 > len(bc):
                    return len(bc)
                kidx = bc[pos] | (bc[pos + 1] << 8)
                pos += 2
                if kidx < len(freq):
                    freq[kidx] += 1
                pos = count_one(bc, pos)
            return pos
        elif tag == 0x09:
            pos += 1
            n, pos = read_len(bc, pos)
            for _ in range(n):
                if pos + 2 > len(bc):
                    return len(bc)
                kidx = bc[pos] | (bc[pos + 1] << 8)
                pos += 2
                if kidx < len(freq):
                    freq[kidx] += 1
                pos = count_one(bc, pos)
            return pos
        return pos

    for i in range(len(offsets) - 1):
        s, e = offsets[i], offsets[i + 1]
        if s < e:
            count_one(bytecode, s)

    indexed = list(range(len(strings)))
    indexed.sort(key=lambda i: (-freq[i], i))
    new_idx = [0] * len(strings)
    for new_pos, old_pos in enumerate(indexed):
        new_idx[old_pos] = new_pos

    new_strings = [strings[old] for old in indexed]

    new_pairs = []
    for str_idx, ability_idx in card_ability_pairs:
        new_pairs.append((new_idx[str_idx], ability_idx))

    new_bytecode = bytearray()
    new_offsets = []

    def write_idx(out, idx):
        if idx < 0xFE:
            out.append(idx)
        else:
            out.append(0xFE)
            out.extend(struct.pack("<H", idx))

    def rewrite_one(bc, pos, out):
        if pos >= len(bc):
            return pos
        tag = bc[pos]
        pos += 1
        out.append(tag)
        if tag in (0x00, 0x01, 0x02):
            return pos
        elif tag == 0x03:
            if pos >= len(bc):
                return pos
            b = bc[pos]
            if b <= 0xFD:
                out.append(b)
                return pos + 1
            elif b == 0xFE:
                out.append(b)
                out.extend(bc[pos + 1 : pos + 3])
                return pos + 3
            elif b == 0xFF:
                out.append(b)
                out.extend(bc[pos + 1 : pos + 5])
                return pos + 5
            out.extend(bc[pos : pos + 9])
            return pos + 9
        elif tag == 0x04:
            out.extend(bc[pos : pos + 8])
            return pos + 8
        elif tag == 0x06:
            if pos + 2 > len(bc):
                return len(bc)
            old_idx = bc[pos] | (bc[pos + 1] << 8)
            pos += 2
            write_idx(out, new_idx[old_idx])
            return pos
        elif tag == 0x07:
            n, pos = read_len(bc, pos)
            write_len(out, n)
            for _ in range(n):
                pos = rewrite_one(bc, pos, out)
            return pos
        elif tag == 0x08:
            n, pos = read_len(bc, pos)
            write_len(out, n)
            for _ in range(n):
                if pos + 2 > len(bc):
                    return len(bc)
                old_kidx = bc[pos] | (bc[pos + 1] << 8)
                pos += 2
                mapped = new_idx[old_kidx]
                write_idx(out, mapped)
                pos = rewrite_one(bc, pos, out)
            return pos
        elif tag == 0x09:
            vtag = bc[pos]
            pos += 1
            out.append(vtag)
            n, pos = read_len(bc, pos)
            write_len(out, n)
            for _ in range(n):
                if pos + 2 > len(bc):
                    return len(bc)
                old_kidx = bc[pos] | (bc[pos + 1] << 8)
                pos += 2
                mapped = new_idx[old_kidx]
                write_idx(out, mapped)
                pos = rewrite_one(bc, pos, out)
            return pos
        return pos

    for i in range(len(offsets) - 1):
        s, e = offsets[i], offsets[i + 1]
        new_offsets.append(len(new_bytecode))
        if s < e:
            rewrite_one(bytecode, s, new_bytecode)
    new_offsets.append(len(new_bytecode))

    print(
        f"Bytecode: {len(bytecode)} -> {len(new_bytecode)} bytes ({100 * (1 - len(new_bytecode) / len(bytecode)):.1f}% smaller)"
    )
    top_n = min(254, len(indexed))
    top_freq = sum(freq[indexed[j]] for j in range(top_n))
    total_freq = sum(freq)
    print(
        f"Strings: {len(strings)} unique, top {top_n} cover {top_freq}/{total_freq} refs ({100 * top_freq / total_freq:.1f}%)"
    )

    return new_bytecode, new_offsets, new_strings, new_pairs


def atomic_write_bytes(path, data):
    """Write `data` to `path` so a reader never sees a half-written file.

    `Path.write_bytes` truncates first and then streams, so an interrupted run
    leaves a PARTIAL artifact behind. For these blobs that is not a small
    problem: the engine loads `abilities.bin.z` at compile time, and a truncated
    one does not fail cleanly — it decodes to garbage strings and the test
    binary dies with STATUS_STACK_BUFFER_OVERRUN and a log full of
    `unknown action string "<fragment>"`. Observed after a run was interrupted
    mid-write.

    Writing to a sibling temp file and renaming means the destination is either
    the old content or the new content, never a prefix of either.
    """
    tmp = path.with_name(path.name + ".tmp")
    try:
        tmp.write_bytes(data)
        os.replace(tmp, path)
    finally:
        if tmp.exists():
            tmp.unlink()


def atomic_write_text(path, text):
    """`atomic_write_bytes` for the generated sources.

    `abilities_gen.rs` is tracked, so a partial write there is at least loud —
    the crate stops compiling — but it leaves the tree in a state that looks like
    a deliberate edit rather than an interrupted run.
    """
    atomic_write_bytes(path, text.encode("utf-8"))


def generate_abilities_gen(bytecode, offsets, strings, card_ability_pairs, build_dir):
    assert (build_dir / "abilities.bin.z").exists(), (
        "run main() first: abilities.bin.z must exist before generating abilities_gen.rs"
    )

    blob_bytes, str_offsets = write_string_blob(strings, build_dir / "abilities_strings.bin")
    offsets_hex_str = ", ".join(str(o) for o in str_offsets)
    pair_strs = ", ".join(f"{s},{a}" for s, a in card_ability_pairs)

    offset_deltas = delta_encode_offsets(offsets)
    delta_strs = ", ".join(str(d) for d in offset_deltas)

    CHUNK_CAP = 30000
    snes_chunks, snes_locs, snes_sizes = [], [], []
    cur, cur_start = bytearray(), 0
    for i in range(len(offsets) - 1):
        d = offsets[i + 1] - offsets[i]
        if len(cur) + d > CHUNK_CAP and len(cur) > 0:
            snes_chunks.append(bytes(cur))
            snes_sizes.append(len(cur))
            cur = bytearray()
            cur_start = 0
        snes_locs.append((len(snes_chunks), cur_start, d))
        cur += bytecode[offsets[i] : offsets[i + 1]]
        cur_start += d
    if len(cur) > 0 or not snes_chunks:
        snes_chunks.append(bytes(cur))
        snes_sizes.append(len(cur))
    assert all(s <= 0xFFFF for s in snes_sizes), "chunk too big for u16 offset"

    snes_chunk_decls = "\n".join(
        f'extern "C" {{ pub static BYTECODE_C{ci}: [u8; {len(c)}]; }}' for ci, c in enumerate(snes_chunks)
    )
    snes_loc_strs = ", ".join(f"({c},{s},{l})" for c, s, l in snes_locs)
    snes_slice_arms = "\n".join(
        f'        {ci} => unsafe {{ &BYTECODE_C{ci}[start..start + len] }},' for ci in range(len(snes_chunks))
    )

    src = f"""// Auto-generated by compile_abilities.py
//
// Each ability in `unique_abilities` is stored as a compact *binary JSON* slice
// (tagged tree with interned strings). `get_ability(idx)` decodes
// `BYTECODE[OFFSETS[idx]..OFFSETS[idx+1]]` back into the SAME `serde_json::Value`
// the text loader would produce, then runs the identical post-processing
// (`from_value::<Ability>` + `populate_from_json` + draw-count fix). Field names
// are interned into `STRINGS` so the blob is far smaller than text JSON, while
// remaining fully data-driven: new action types / fields need zero decoder
// changes.

pub const NUM_ABILITIES: usize = {len(offsets) - 1};

#[cfg(all(not(feature = "snes"), not(feature = "gba")))]
pub const COMPRESSED_BYTECODE: &[u8] = include_bytes!("../../../cards/build/abilities.bin.z");
#[cfg(feature = "gba")]
pub const BYTECODE: &[u8] = include_bytes!("../../../cards/build/abilities.bin");

/// Per-ability byte lengths (deltas between consecutive absolute offsets).
/// `OFFSET_DELTAS[i]` is the size of the binary-JSON slice for
/// `unique_abilities[i]`. Absolute positions are rebuilt as a running prefix
/// sum (see `offset_of` in vm.rs). Stored as `u16` because slice lengths are
/// small; this is half the size of the old absolute u32 offset table.
#[cfg(not(feature = "snes"))]
pub const OFFSET_DELTAS: &[u16] = &[{delta_strs}];

/// SNES/16-bit path: the bytecode is split into <=64KB chunks (a single larger
/// object cannot exist on a 16-bit target). Data lives in ROM via these extern
/// symbols; `ABILITY_LOCS[i]` = (chunk_idx:u8, start:u16, len:u16) for ability i.
#[cfg(feature = "snes")]
{snes_chunk_decls}
#[cfg(feature = "snes")]
pub const ABILITY_LOCS: &[(u8, u16, u16)] = &[{snes_loc_strs}];

/// Return the bytecode slice for ability `idx`'s chunk at (start, len). 16-bit
/// safe: `start`/`len` are within a single <=64KB chunk.
#[cfg(feature = "snes")]
pub fn bytecode_slice(ci: u8, start: usize, len: usize) -> &'static [u8] {{
    match ci {{
{snes_slice_arms}
        _ => &[],
    }}
}}

/// Interned strings: object keys and string values. Stored as a single blob
/// with u32 offsets to save the 16-byte per-entry fat pointer overhead of
/// `&[&str]` (saves ~68KB for {len(strings)} strings). Indexed by the 2-byte `u16`
/// references inside `BYTECODE` via `get_string(idx)`.
pub const STRINGS_BLOB: &[u8] = include_bytes!("../../../cards/build/abilities_strings.bin");
pub const STRINGS_OFFSETS: &[u32] = &[{offsets_hex_str}];
#[inline]
pub fn get_string(idx: usize) -> Option<&'static str> {{
    if idx + 1 >= STRINGS_OFFSETS.len() {{ return None; }}
    let start = STRINGS_OFFSETS[idx] as usize;
    let end = STRINGS_OFFSETS[idx + 1] as usize;
    unsafe {{ Some(core::str::from_utf8_unchecked(&STRINGS_BLOB[start..end])) }}
}}

/// Card_no -> ability index pairs. Each entry is (card_no_string_index, ability_index).
/// Generated from the `cards` field of `unique_abilities`. Used at load time by
/// `CardLoader::build_abilities_map_shared` to build the card_no -> Vec<AbilityRef>
/// mapping without parsing abilities.json into a `serde_json::Value`.
///
/// Format: flat array of [str_idx, ability_idx, str_idx, ability_idx, ...]
pub const CARD_ABILITY_PAIRS: &[u16] = &[{pair_strs}];
"""
    atomic_write_text(build_dir / "abilities_gen.rs", src)
    engine_dir = ENGINE_DIR / "src" / "ability"
    if engine_dir.exists():
        atomic_write_text(engine_dir / "abilities_gen.rs", src)

    c_lines = ['/* Auto-generated by compile_abilities.py: snes BYTECODE chunk data */']
    for ci, c in enumerate(snes_chunks):
        hexs = ", ".join(f"0x{b:02x}" for b in c)
        c_lines.append(f"const unsigned char BYTECODE_C{ci}[{len(c)}] = {{{hexs}}};")
    c_src = "\n".join(c_lines) + "\n"
    atomic_write_text(build_dir / "bytecode_data.c", c_src)
    atomic_write_text(ENGINE_DIR.parent / "platforms" / "snes" / "bytecode_data.c", c_src)
    print(f"  bytecode_data.c: {len(snes_chunks)} chunks")


def main():
    root = CARDS_DIR
    with open(root / "abilities.json", encoding="utf-8") as f:
        data = json.load(f)
    abilities = data["unique_abilities"]
    print(f"Compiling {len(abilities)} abilities...")

    compiled = compile_all(abilities)
    bytecode = compiled.bytecode
    offsets = compiled.offsets
    strings = compiled.strings
    card_ability_pairs = compiled.card_ability_pairs

    # Every name in the corpus should be in the tables above. One that is not
    # still compiles — it fails closed, or lands on the no-op bucket — but the
    # ability that carried it does not do what its text says, and that is a
    # silent engine defect unless it is printed here.
    unclassified = compiled.unclassified
    if not unclassified.is_empty:
        print("\n  UNCLASSIFIED wire names (compiled to a fail-closed variant):")
        print(f"    conditions: {unclassified.conditions}")
        print(f"    actions:    {unclassified.actions}")
        print(f"    cost types: {unclassified.cost_types}")

    build_dir = root / "build"
    build_dir.mkdir(parents=True, exist_ok=True)

    MAGIC = b"RBKA"
    VERSION = 1
    header = MAGIC + VERSION.to_bytes(4, "little")
    compressed = zlib.compress(header + bytes(bytecode), level=9)
    atomic_write_bytes(build_dir / "abilities.bin.z", compressed)
    atomic_write_bytes(build_dir / "abilities.bin", header + bytes(bytecode))
    print(f"\n  abilities.bin: {len(bytecode) + len(header)} bytes ({(len(bytecode)+len(header)) / 1024:.1f}KB) with header")
    print(f"  compressed: {len(compressed)} bytes ({len(compressed) / 1024:.1f}KB) ({100*(1-len(compressed)/(len(bytecode)+len(header))):.1f}% smaller)")
    print(f"  header: magic={MAGIC!r} version={VERSION}")
    print(f"  interned strings: {len(strings)}")
    print(f"  card->ability pairs: {len(card_ability_pairs)}")

    generate_abilities_gen(bytecode, offsets, strings, card_ability_pairs, build_dir)
    print(f"  Avg: {len(bytecode) / len(abilities):.1f} bytes/ability")

    abilities_json_path = root / "abilities.json"
    abilities_hash = sha256_short(abilities_json_path)
    bytecode_hash = hashlib.sha256(bytecode).hexdigest()[:16]

    # engine_commit is stamped by write_generation_manifest, which resolves it
    # itself; do not compute a second copy here.
    write_generation_manifest(
        build_dir,
        "compiled_abilities.v1",
        "cards/compile_abilities.py",
        "cards/abilities.json",
        abilities_hash,
        len(abilities),
        len(bytecode) + len(header),
        len(compressed),
        "abilities",
        {
            "interned_strings": len(strings),
            "card_ability_pairs": len(card_ability_pairs),
            "sha256": bytecode_hash,
            "compressed_sha256": hashlib.sha256(compressed).hexdigest()[:16],
        },
    )
    print(f"  manifest: generation_manifest.json")


if __name__ == "__main__":
    main()