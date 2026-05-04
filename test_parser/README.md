# Radical Parser — A Structure-First, Data-Driven Approach

## The Problem

The existing `parser.py` is **3072 lines** of procedural cascading handlers:
- ~37 effect handler functions in strict priority order
- ~25 condition handler functions in strict priority order
- ~80 dispatch rules for action type detection
- Dozens of special-case `if/elif` blocks
- Accumulated edge cases from every new card set

Ordering is critical and implicit. Adding a new pattern means writing new code,
finding the right insertion point, and hoping nothing breaks.

## The Radical Insight

**There are only 602 unique ability texts.** The text is not arbitrary natural
language — it follows a finite set of structural patterns. Most "parsing
complexity" comes from trying to extract everything at once (action + source +
destination + count + card_type + condition + ...) from raw text.

**What if we separated the problem into two phases?**

1. **Structure first** — identify the text's overall shape using pattern
   markers (cost:effect, conditional, sequential, look-and-select, etc.)
2. **Slot-filling second** — extract parameters only within the context of
   the known structure

## The Architecture

### Phase 1: Declarative Pattern Matching

Ability structures are defined as **data** (dicts), not code:

```python
{
    "name": "look_and_select",
    "priority": 60,
    "detect": {"structural": ["has_look_and_select"]},
    "split": {"custom_split": ..., "into": ["look_text", "select_text"]},
    "assemble": lambda p, t: {...},
}
```

Each pattern declares:
- **What markers to look for** (required substrings, structural features)
- **How to split the text** into sub-components
- **How to assemble** the structural annotation

The interpreter is generic — it iterates patterns, checks detection rules,
applies splits, and runs assembly. **First match wins** for the primary
pattern; later matches still add their annotations.

### Phase 2: Slot-Filling Extractors

Small, focused functions extract specific parameters from text. They are
called **only in the context of a matched structure**:

```python
def extract_source(text): ...
def extract_destination(text): ...
def extract_count(text): ...
def extract_card_type(text): ...
def extract_target(text): ...
```

No monolithic "parse everything" function. Each extractor knows its domain.

### Phase 3: Composition

`assemble_effect()` walks the structural tree and fills slots at each level.
The structure defines the TREE; slots fill the LEAVES.

## Results vs Existing Parser

| Metric | Accuracy |
|--------|----------|
| Cost type | 98.0% |
| Source location | 89.9% |
| Destination location | 86.7% |
| Action type | 67.8% |
| Exact structural match | 15.4% |
| Partial match (no errors) | 56.8% |

**Code size**: 1268 lines vs 3072 lines (**58% reduction**)

## What This Demonstrates

1. **Patterns as data** — adding a new ability structure = adding one dict,
   not writing a handler function and worrying about cascade ordering.

2. **Structure-first** — identifying the shape before extracting details
   eliminates most ambiguity and special-case logic.

3. **Separation of concerns** — structure detection, slot extraction, and
   output composition are independent layers.

4. **Composable nesting** — structures contain sub-structures (conditional
   contains condition + effect, sequential contains sub-effects, etc.),
   mirroring the output JSON.

## Remaining Work

The current implementation focuses on the high-value patterns (covering ~70%
of abilities). To reach full parity with the existing parser, you'd add:

- More condition type handlers (compound, baton_touch, energy_state, etc.)
- `activation_condition` suffix parsing
- Better `gain_ability` vs `gain_resource` disambiguation
- Per-unit timing conditions
- Answer-based choice handler
- `kore_niyori` cascade handler

Each addition follows the same pattern — add a dict entry, not a function.

## The Big Idea

The existing 3072-line parser encodes knowledge about ability structure
**implicitly** in the ordering of if/elif chains. This parser encodes the
same knowledge **explicitly** in declarative pattern descriptors.

When a new card set adds novel ability patterns, you add patterns. You don't
rewrite cascades. You don't worry about ordering. The pattern declares what
it needs; the interpreter handles the rest.
