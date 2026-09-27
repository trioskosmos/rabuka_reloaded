# Parser + Engine: The Rewrite

This document specifies **replacements, not verifications.** It says what each
subsystem becomes. It does not argue about whether the new version is safe —
the 3,000+ tests answer that, and they are the only gate.

Read `docs/REFACTORING_APPROACH.md` for the measurements. This is what to do
about them.

---

## 0. Posture

Three things this document is not doing:

**Not arguing for byte-identical artifacts.** A previous draft made
`git diff cards/abilities.json` the parser's gate. That was wrong, and worth
recording why so nobody re-adds it: `serde_json` is compiled without
`preserve_order` (`engine/Cargo.toml:15`), so `Value::Object` is a `BTreeMap`
and `av != bv` in `bytecode_deep_compare_test.rs:151` cannot see key order. The
generated decoders read **by key name**, not by position
(`condition_decoder_gen.rs:115+`). The parser↔engine contract is semantic, and a
diff in the JSON is only interesting if a test moves.

**Not listing work items.** Each section below states a new design. Where a
section lists a count — 43 sites, 6 copies, 9 entry points — that count is the
size of the thing being deleted, not a to-do list.

**Not preserving behaviour by default.** Where a rewrite removes a field that
was always empty, removes an unreachable enum variant, or deletes a code path
that could never fire, that is the point. The tests decide whether the removal
was correct.

### The one real gap, fixed first

`AbilityEffect.kind` is `#[serde(skip)]` (`core/card.rs:1340-1341`), and
`bytecode_deep_compare_test.rs:149-150` substitutes `Null` on serialization
failure. So `EffectFilter` — 159 fields, `card.rs:971-1165` — plus placement,
optional and remainder are **structurally invisible** to the 100%-coverage
deep-compare. `describe_parity_test.rs` catches some of it indirectly; not all.

This matters because 3.2 rewrites the thing `kind` holds. Make `kind` serialize
before that wave. It is one attribute and a round-trip through
`populate_from_json`, and it converts the strongest existing check from
"compares 936 abilities, minus their largest field" into "compares 936
abilities."

---

## 1. The parser

`parser.py` is 14,021 lines. Four public entry points and their direct helpers
are ~1,500. The other ~12,500 is a pipeline plus a repair system that exists
because the pipeline is not sure of itself.

### 1.1 One file becomes a module tree

Five unrelated concerns share one file: the entry points, the action rules, the
condition cascade, the effect cascade, the repair walkers, the validator.

It becomes:

- **`parser.py`** — the four entry points, shared constants, `parse_action`,
  the finalization pipeline. Roughly 1,500 lines.
- **`parser/rules.py`** — the 83 `_set_action_NNN` setters and `_ACTION_RULES`
  (`parser.py:1736-2323`).
- **`parser/conditions.py`** — the condition cascade (`parser.py:2813-5050`),
  field extraction (`:5055-5800`), type inference (`:5490-5700`).
- **`parser/effects.py`** — the effect cascade (`parser.py:6668-10109`) and its
  rule tables (`:10112-10543`).
- **`parser/normalize.py`** — the walkers (`parser.py:10544-12837`).
- **`parser/validate.py`** — `_VALIDATORS` and the semantic rules
  (`parser.py:13118-13977`).
- **`parser/vocab.py`** — every regex, every phrase table, every normalizer
  (section 1.4). The only module that spells Japanese card text.

`parser.py` keeps re-exporting, so `extract_card_abilities.py`,
`compile_abilities.py` and the test suite do not change.

### 1.2 Dicts become a typed IR

Today a parse result is a `dict` of strings, `Option`s, lists and nested dicts.
Every later stage guesses which keys exist, which is why `_fill_defaults`
(`parser.py:6555`, 111 lines) re-derives fields `parse_action` already set, why
`optional`/`max` get set at five separate sites (`:2763-2772`, `:6619-6633`,
`:8045-8056`, `:7956-7958`, `:8285`), and why 2,294 lines of repair exist.

Parse results become typed structures — `ParsedAction`, `ParsedCondition`,
`ParsedEffect` — whose constructors require their mandatory fields. A
`ParsedAction` that exists is a complete one. One serializer converts to the
wire shape at the end, and it is the only code that knows the JSON field names.

That single change makes four other things free:

- **`_fill_defaults` disappears.** A missing required field is a constructor
  error at the site that owes it, not a guess 5,000 lines downstream.
- **Field names stop being free.** They are an ABI
  (`compile_abilities.py:223,249`; `docs/ABILITY_PIPELINE.md:117-118`) and a
  typed IR turns "someone renamed this" into a compile error.
- **The condition `type` closed enum becomes a real type.** Today
  `compile_abilities.py:38-73` maps 30 names to tags 0-20 with
  `UNSUPPORTED_CONDITION_VARIANT = 20` as a silent fail-closed default
  (`:210`), and a test (`:489`) checks that new strings fail closed. Failing
  closed should be the type system, not a test.
- **The repair layer becomes deletable** (1.5).

### 1.3 Cascades become tables

Both cascades are hand-written ordered if-chains where *code position is the
priority mechanism*, documented only in a comment (`parser.py:2833-2850`,
`:6673-6706`).

`parser_utils.py` already has the abstraction — `PriorityRegistry:899` and the
`_TextRule:990` / `ActionRule:1018` / `EffectPattern:1074` /
`ConditionPattern:1119` dataclasses. The cascades ignore it and hand-roll
anyway.

They get finished. One handler contract, one priority column, one table. Order
becomes data.

Two shapes collapse into that:

**The `_set_X` / `_matches_X` triples.** Nine pairs, each a predicate and a
writer that must agree. One documents the invariant —
`parser.py:10165-10166`: *"Matcher and setter share this so 'does this phrase
match?' and 'build the effect' can never disagree."* The other eight don't, and
a fix applied to one half is a silent bug. They become one `PhraseEffect`
registration where agreement is structural: `_blade_count_match:3108` /
`_set_blade_count_condition:3123` / `_matches_:3136`; `_matches_answer_choice:7711`
/ `_set_answer_choice:7701`; `_matches_opponent_action:7833` / `_set_:7815`;
`_matches_opponent_after_conditional:7859` / `_set_:7866`;
`_matches_kore_niyori_case:7903` / `_set_:7888`;
`_matches_kore_niyori_cascade:9208` / `_set_:9190`; `_matches_furthermore:8750`
/ `_set_:8738`; `_matches_sou_shinakatta:9662` / `_set_:9650`;
`_matches_place_under_heart_copy:10186` / `_parse_:10161` / `_set_:10190`.

**`_STRUCTURAL_EFFECT_RULES`.** 49 of its 50 rows (`parser.py:10217-10267`) are
the literal string `EffectPattern(action="custom", handler=_try_X)`. `action` is
never read — dispatch is by `__call__` returning a dict
(`parser_utils.py:1131-1141`). It is a registration list wearing a dataclass,
and it becomes a tuple of functions.

Also: `_ACTION_RULES` embeds raw regex literals in ten rows (`:2254, 2279, 2280,
2285, 2286, 2287, 2320, 2322`), defeating the `match`/`match_any`/`match_all`
contract the surrounding rows use — all compiled at module scope. And
`action_039_gain_resource:2279` and `action_047_gain_resource:2287` have
byte-identical predicates, so the second can never fire.

### 1.4 Vocabulary becomes one module

Every one of these is the same idea re-implemented, and after this section
there is exactly one home for each.

**Phrase tables.** Eight hand-rolled `(tuple_of_phrases, value)` tables across
three modules, all with first-match-wins semantics: `parser.py:3941, 3950, 5861,
6132, 6138, 6335, 6343`; `parser_utils.py:416, 468, 540, 551, 562, 569`;
`cost_parser.py:31, 306, 351`. One `PhraseTable` type. Three of them carry a
comment that the predicates run in the legacy if-chain's order
(`_CONDITION_MOVEMENT_RULES:5203`, `_DISTINCT_MARKER_RULES:5210`,
`cost_parser.py:306`) — that order is the contract.

**Card types.** Four copies: `parser_utils.CARD_TYPE_PATTERNS:562` (4 rows),
`parser.CARD_TYPE_KEYWORDS:5775` (3 rows), a hand-written 7-branch if/elif in
`_infer_card_type:5815-5830`, and `cost_parser.py:251-255`. One table,
longest-match-first, which is the order `_CARD_TYPE_LONGEST_FIRST:592` already
implements.

**Cost-modification phrasings.** Four copies: `COST_MODIFICATION_PATTERNS:375-384`
(whose comment at `:372-374` claims *"the only copy of these regexes"*),
`_COST_SET_RE:1536`, `_COST_DELTA_RE:1539`, and one inlined in a rule at
`:2254`.

**Full-width digits.** Three implementations.
`parser_utils.normalize_fullwidth_digits:83` handles `＋ − －`;
`parser.py:11372` re-implements a narrower version inside `_fw_int`, dropping
all three; and four regexes hand-spell the signs and bypass normalization
outright (`:1539, :1540, :6053, :6054`).

**Heart icon ids.** Seven spellings of one format string — `parser.py:291, 316,
332, 5733, 6999, 9823, 10079`. `parser_utils.py:33-36` declares itself the
single source and owns the regex half of that contract but not the formatting
half.

**Position keywords.** Three tables — `parser_utils.POSITION_KEYWORDS:581`
(called *"the only copy of the map"* at `:579`),
`parser_fields.ICON_POSITION_TEMPLATES:14`,
`parser_fields.NOTE_POSITION_KEYWORDS:35`.

**Nesting-aware splitting.** Three implementations of depth-0 scanning:
`split_cost_effect:648-685` hand-rolls a `while` loop with paren/bracket/template
counters (and at `:652-654` names the same three bracket classes the others
use), `_ir_depth_scan:706` + `_find_depth0:746` + `_split_marker_depth0:755` is
a second, `_split_sentences_nesting:730` a third.

**`SPLIT_LIMIT = 1`.** Two definitions — `parser.py:174`, `cost_parser.py:29`.

**`BLADE_ICON_HELD_RE`.** Compiled at `parser_utils.py:67`, imported at
`parser.py:111`, never used; `parser.py:2336` re-types it.

### 1.5 The repair layer is deleted

`parser.py:10544-12837`. 2,294 lines that walk a finished tree and mutate it.

Each repairer goes the way its producer gets fixed: find the stage that emits
the shape, make it emit the right shape, delete the repairer. In dependency
order, one at a time — `_propagate_context:11917` (166 lines),
`_fix_conditional_on_result:12477` (183), `_repair_corpus:12660` (168),
`_apply_recursive_fixes:12083`, `_infer_missing_action:12148`,
`_fix_condition_enrichment:12239`, `_fix_sequential_chain:12183`,
`_prefix_condition_reparse:12293`, `_fix_primary_negation:12345`,
`_fix_compound_gain_split:12383`, `_fix_spurious_sequential_change_state:12424`,
`_fix_auto_condition:12460`, `_collapse_position_changes:11787`, the ~18
`_walk_*` functions, and the rest.

`_repair_corpus` needs one step first. It contains **six card-specific patches**
(DOLLCHRESTRA `:12709`, surplus_heart `:12744`,
`_fix_change_state_self_cost:12766`, Q76 `:12786`,
既にメンバーがいるエリア `:12794`, live-card-set-phase `:12801`) that directly
contradict `card_overrides.py:4-5` — *"Every card-ID-specific patch lives here
instead of being scattered through parser.py ... never in parser.py"* — while
`parser.py:12816-12817` asserts the opposite policy **in the same function**.
They go to `card_overrides.py`, which then has a real reason to exist.

### 1.6 Tree walking becomes one mechanism

`parser_utils.py` has `iter_dict_nodes:242`, `walk_dict_tree:265`,
`transform_child_lists:333`, used at seven sites. Four hand-rolled copies of the
same loop exist: `parser.py:12883 _clean`, `:12911 _propagate_optional`,
`:13223 _json_has`, `extract_card_abilities.py:619 _walk_filters`.

Seven hand-written "is this child list, and does it hold an action of kind X"
guards at `parser.py:11770, 11776, 11967, 12516, 12529, 12778, 13351` become
one, using the `list_keys` / `after_key` hooks that already exist.

### 1.7 The 38 integer sites

`parser.py:429, 459, 480, 619, 621, 2001, 2156, 2165, 2376, 3017, 3093, 3129,
3379, 3635, 3636, 4472, 4539, 4543, 4790, 5022, 5138, 5510, 5511, 5562, 5947,
5956, 6216, 6460, 6763, 7604, 9335, 9711, 9909, 9963, 10056, 10365, 11373, 12272`.

Thirty-six of them read `\d+` off unnormalized text; only two (`:4539`, `:4543`)
apply `normalize_fullwidth_digits` — while `parse_action:2593` normalizes the
whole string for a different purpose in the same function.

One normalizing integer parser, used everywhere.

### 1.8 Parser leftovers

**21 unused imports.** From `cost_parser` (`parser.py:142-156`), 15 of 17 are
dead: `_COST_FLAG_RULES`, `_COST_BATON_TOUCH_PATTERNS`, `_register_cost`,
`_cost_verb_choice`, `_cost_energy`, `_cost_sequential`, `_cost_reveal`,
`_cost_choice_comma`, `_classify_cost`, `_extract_basic_cost_fields`,
`_fill_cost_source`, `_fill_cost_destination`,
`_infer_destination_from_source`, `_mark_discard_all_hand`, `_mark_self_cost`.
Only `parse_cost:157` and `_COST_HANDLERS:141` are live. From `parser_utils`
(`parser.py:92-102`), 6 are dead: `BLADE_ICON_HELD_RE`, `count_icons`,
`extract_cost_values`, `FieldContext`, `ACTION_DESTINATION_POLICY`,
`ACTION_SOURCE_POLICY` (the last two dead repo-wide). All 11 `parser_fields`
imports are live.

**Dead functions.** `parser_utils.annotate_tree:398` (only self-references),
`parser_utils.count_icons:73`.

**A test redeclares a parser constant.** `_SELF_APPEARANCE_PATTERNS` exists in
both `parser.py:12825` and `tests/test_ability_invariants.py:27`. It gets
imported.

**The baseline mechanism is deleted.** `validation_baseline.json` is `{}` and
`extract_card_abilities.py:545-557` counts any newly-firing rule as a
regression, so `--check` (`.github/workflows/coverage.yml:52`) is permanently
red or trivially green. The 43 semantic rules become assertions with their
expected outcome stated in the rule. A rule that has never fired is a rule to
justify.

---

## 2. The engine

### 2.1 Six zone vocabularies become one

| Vocabulary | At | Variants |
|---|---|---:|
| `Zone` | `ability/enums.rs:35` | 34 |
| `ZoneId` | `core/types.rs:691` | 29 |
| `Location` | `core/card.rs:2957` | 10 |
| `ZoneId` | `bot/encoding.rs:22` | 15 |
| zone→label EN | `describe.rs:22-65` | — |
| zone→label i18n | `condition.rs:247-264` | — |
| zone→cards | `ability/util.rs:1879` | — |

The repo names the problem itself at `core/types.rs:757`: *"B3: single
conversion layer for the four zone vocabularies."*

One `Zone` enum. `Location` folds in — the condition JSON wire name is preserved
by the serializer. `bot/encoding::ZoneId` goes; the bot's `NUM_ZONES = 15`
layout is derived from `Zone` at the encoding boundary, in one file, instead of
being a parallel enum kept in sync by hand. `core/types::ZoneId` merges in.
`Zone` gains cards-of-a-player, EN label, JA label, i18n key, and the
event-log discriminant.

Then `ability/util.rs:1879 zone_cards` is the only way to read cards out of a
zone, and the **43 `match Zone::from_str` sites in `ability/`** stop existing as
special cases. The ~28 inline zone-to-`Vec` tables become one call:
`condition/card.rs:288, 575, 1523, 1779, 3071, 3102, 3143, 3221, 3285, 4214`;
`look.rs:212, 355, 414, 1148`; `util.rs:1885, 1984, 2109`;
`move_cards.rs:1126, 1352`; `choice.rs:1068`; `cost/handlers.rs:1208`;
`condition/state.rs:1084`; `condition/card/comparison.rs:331`;
`condition.rs:1004`; `effects/draw.rs:608`; `util/selection.rs:131`;
`core/game_state/abilities.rs:2480`; `game/game_setup.rs:1170`.

The stage idiom — `player.stage.stage.iter().filter(|&&id| id != -1).copied()
.collect()` — appears at 20+ sites (`condition.rs:726-731, 800-805, 925-931,
968-974, 1062-1068`; `condition/card.rs:106-112, 289-295, 380-386, 456-462,
506-512, 569-574, 1610-1616 ×2, 2043-2051, 3062-3068, 3136-3142, 3214-3220`;
`condition/state.rs:46-52, 455-461`; `move_cards.rs:983-989, 1240-1242`;
`cost/handlers.rs:327-332`) and becomes one named function.

**The lossiness becomes visible instead of silent.** Today
`from_ability_zone` (`types.rs:760`) maps 22 of 34 variants and collapses the
rest to `ZoneId::Unknown:783`; `to_ability_zone:786` returns `None` for three.
Merging the enums turns those edge cases into a compile error at the boundary
rather than a default nobody notices.

**Two zone counters stop disagreeing.** `util::get_zone_card_count:2488` returns
a card count and is dead; `condition/card.rs:3277 zone_len` returns `u8` and its
`Stage` arm returns `player.stage.total_blades(...)` — a blade count. One
counter, named for what it counts.

### 2.2 Two wide structs become keyed attribute maps

`core/card.rs` is 4,395 lines and is not one concept — it is the card wire
format, the effect wire format, the condition wire format, and two very large
hand-maintained accessor surfaces.

- **`EffectFilter`, `card.rs:971-1165` — 159 fields**, 195 lines.
- **`ConditionCommon`, `card.rs:3008-3148` — ~112 `Option<T>` fields**, each with
  a `serde(default)`.

And the accessors they generate: ~70 `<field>_any()` / `set_*()` methods at
`card.rs:1821-2524` (~700 lines) and ~105 `get_*()` methods at `card.rs:3394-3690`
plus `card.rs:3736-4134` (~700 lines).

Both become a keyed attribute store with typed accessors derived once. The JSON
wire format is unaffected.

What that buys:

**`EffectKind` collapses.** Its 16 variants are *all* structurally identical
(`card.rs:1172-1217`, every one `{ filter: Option<Box<EffectFilter>> }`), so
`filter():1220` and `filter_mut():1240` are 17-arm matches whose every arm does
the same thing. Its own doc at `:1167-1169` says so: *"each variant is just a
marker carrying the filter box so the action-type dispatch in `filter()`/
`filter_mut()` keeps working."* It becomes a struct. `from_action:1267` — which
maps ~60 wire strings onto 14 of the 16 — stays, as the wire→kind table.

**Four tables over `Condition`'s 21 variants become one.** Evaluator dispatch
`ability/condition.rs:519-556`, expectation `:145-243`, log string `:287-309`,
and `Condition::condition_type()` `card.rs:3768-3801`. The first two are
arm-for-arm twins. A fifth, `describe_condition_actual:688-884`, works from the
resulting discriminant and re-derives "all members on stage" three times inside
itself (`:724-733, 749-758, 798-807`).

**Two of those 21-arm matches stop existing.** All 21 `Condition` variants carry
`common: Box<ConditionCommon>`, so `common():3396` and `common_mut():3423` are two
matches whose every arm destructures a field every variant already has.

**`EffectFilter` stops carrying `self_target:977` and `under_self:982`.** They
exist only because `PlacementTarget` (`enums.rs:261`) was never finished folding
them — its doc at `:255-259` says the ambiguity is the reason it exists.

**Adding a field becomes one line** instead of: struct field, `get_*`, `set_*`,
an entry in `ConditionLocals` (`condition_decoder_gen.rs:16-105`, 84 fields) or
`EffectKindLocals` (`effect_decoder_gen.rs:235-395`, 160 fields), the
deep-compare normalizer, and the describe table.

**`CompoundBranch:935`** drops its 11 legacy fields.

The generated decoders are `include!`d, not `mod`-declared (`vm.rs:21-22`), so
they resolve in `vm`'s scope and depend on its exact 24-method `BcReader`
surface. Regenerate via `cards/generate_condition_decoder.py` and
`cards/generate_effect_decoder.py`. Never hand-edit
`condition_decoder_gen.rs` or `effect_decoder_gen.rs` — their headers say so.

### 2.3 Three movement mechanisms become one

Five exist:

1. `util.rs:1977 remove_card_from_zone` / `:2100 place_card_in_zone` / `:2046
   move_card` / `:2074 move_cards` — string zones, re-parsed every call.
2. `util/selection.rs:121 zone_remove_at_indices` — a second zone table.
3. `move_cards/transport.rs:40 remove_card_from_any_zone` (4 zones) and
   `transport.rs:82 remove_from_physical_zones` (2 zones, a different set) — same
   name, different behaviour, difference recorded only as prose at
   `transport.rs:77-81`.
4. `move_cards.rs:264 resolve_cards_from_source` — **19 parameters**, dispatching
   by `if source_str == "<literal>"` compares (`:301, 313, 326, 343, 346, 363`)
   rather than a `Zone` match, so `"those_cards"` and `"recently_moved"` take
   different paths from their `Zone` counterparts. Then `move_cards.rs:755-808`
   is an 11-arm `match Zone::from_str`, whose `Energy` arm (`:780-796`) inlines
   48 lines of energy-deck pop logic that bypasses the shared path.
5. `core/player.rs:197 move_card_from_hand_to_stage` — 168 lines with its own
   cost computation, baton-touch, under-card recycling and waitroom push.

One `GameState::transfer_cards`, taking `Zone` (never `&str`), a source
selection, a count, and a destination with placement. (1) absorbs (2); (3) folds
in as an explicit zone list; (4)'s string compares become a `Zone` match and
the `Energy` arm calls the shared pop; (5) becomes a caller.

**The 48 bypasses close.** There are 48 direct writes to `waitroom` across 14
files that never touch either movement-event entry point: `util.rs:2009, 2114,
2128, 2197, 2206`; `move_cards.rs:461, 575, 577, 674, 1102, 1912, 2236, 2286`;
`transport.rs:21, 48, 85, 87`; `placement.rs:155`; `choice.rs:1725, 3334, 3426,
3433`; `look.rs:74, 809`; `effects/draw.rs:572`; `turn/phases.rs:1042, 1649`;
`turn/actions/mod.rs:572, 929, 935, 1736, 1767, 1776, 1800, 1823, 1859`;
`turn/live.rs:1481, 2974`; `core/player.rs:170, 338, 490`;
`game/web_server.rs:1786, 1862`; `bot/determinization.rs:105`.
`last_vacated_stage_area` has six or more independent writers.

`game_state/modifiers.rs:1456 push_movement_event_typed` and `:1477
push_movement_event` become the only writers, and the fields they own become
private to `modifiers.rs` so the compiler finds the other 48.
`push_movement_event` writes 6 of 10 tracking fields and documents its own
un-unified shadow fields at `:1557-1561`; that gets finished.

### 2.4 Six player tables become one, and five card-lookups become one

**`"self"`/`"opponent"` → player: 6 copies of the same 4-arm table.**
`util.rs:456`, `util.rs:475`, `game_state/abilities.rs:2615` and `:2659` (the
last two identical apart from `&mut`), `game_setup.rs:1162` (inline),
`ability/condition/card.rs:30`. One function; the `&` version borrows from the
`&mut` one.

Player selection itself has two more copies of the same `match Phase`:
`game_state/mod.rs:686 active_player` / `:704 active_player_mut`, and four
identical 6-line bodies at `:745, :753, :761, :769`
(`first_attacker` / `first_attacker_mut` / `second_attacker` /
`second_attacker_mut`).

**Card → zone: 5 implementations over the same `Player`.**
`core/player.rs:504 contains_card` and `game_state/abilities.rs:2641
owner_of_card` enumerate the same six zones in a different order —
`owner_of_card`'s doc claims to be the single source of truth and
`contains_card` is not routed through it. `core/player.rs:410 all_card_ids`
walks ten. `abilities.rs:1399 search_player_zones_for_card` walks five by
`card_no`. `ability/resolver.rs:723 zone_for_card` and a hand-inlined duplicate
at `abilities.rs:1883-1897` do the same job.

`owner_of_card` is the source. The other four call it. The inline copy dies.

### 2.5 Two describe tables become one

`describe_effect_en:185-628` and `describe_effect_ja:859-1280` are a **44-arm
match written twice**, key-for-key identical, differing only in which label
helper they call. `describe_cost_en:632-667` / `describe_cost_ja:672-722` are
the same shape at 4 arms. And `:752-786` is a third table.

One arm per `ActionType`, with EN/JA as paired labels at each terminal. Roughly
440 lines become 230, there is one place a new action type gets described, and
a missing JA string becomes a compile error rather than a runtime `String::new()`.

Same shape in `cost/handlers.rs:108` / `:143` (`cost_filter_desc_en/ja`).

### 2.6 Nine filter entry points become one

Nine wrap `CardFilter::matches`: `util.rs:1697 matching_indices`, `:1712
matching_ids`, `:1729 build_candidate_pool`, `:1739 matching_ids_filtered`,
`:1779 count_matching`, `:1795 count_matching_distinct`, `:1863
filter_distinct`, `:2433 apply_distinct_filter`, `:1844
dedupe_by_normalized_name`. Plus `condition/card.rs:3296
card_matches_count_filters`, which builds a `CardFilter` by hand to reuse one of
them.

One query object with `ids` / `indices` / `count` / `distinct`.

Three `filter_subset` builders mirror each other and each documents that it
mirrors the others: `core/card.rs:913` (AbilityCost), `:2363` (AbilityEffect),
`:3739` (Condition). `AbilityCost`'s doc (`:910-918`) records that the
hand-maintained version *"silently dropped fields (e.g. `characters`), causing
condition/effect mismatches."* Two of them delegate to the third, and a test
asserts a field added to `EffectFilter` reaches all three — that bug already
happened once.

### 2.7 Five queue levels become one driver

| Level | At | Terminates at |
|---|---|---|
| FIFO advance | `ability_queue.rs:255 start_next` | — |
| resolve one entry | `abilities.rs:1825 process_current_ability` | 200,000 calls (`:1832-1839`, a `static AtomicU32`) |
| depth-first drain | `abilities.rs:1582 process_player_abilities_depth` | 50 iters (`:1714-1716`), 5 re-entries (`:1670`) |
| pending-auto pass | `abilities.rs:1791 process_pending_auto_abilities` | — |
| phase boundary | `turn/actions/mod.rs:1594 check_timing` | — |

Three termination strategies for one hazard. One driver, one budget. The
200,000-call `static AtomicU32` is also a process global that leaks across
tests.

**The post-resolution TAS scan is written twice.** `abilities.rs:2171-2201` and
`turn/actions/mod.rs:1463-1480` build the same `TriggerEvent` from the same four
`GameState` fields and call the same function; `turn/actions/mod.rs:1459-1462`
says so outright: *"Mirrors process_current_ability's post-resolution scan."*
Same for `trigger_each_time_for_member` at `abilities.rs:2209-2229` and
`turn/actions/mod.rs:1502-1519`. `resume_with_choice` exists at
`ability_queue.rs:385` and `turn/actions/mod.rs:766` (100 lines).

**The fake queue entries go.** `ability_queue.rs:429 push_constant_context`
pushes a synthetic dummy `AbilityQueueEntry` — a third hand-written 26-field
literal at `:431` — whose only job is to make `resolve_target_player` return
the right player, per its own doc at `:425-428`. `clear_completed:418` then
special-cases it. The player context becomes a real field on the queue. A stack
of fake abilities carrying one string is a workaround wearing data structure.

`AbilityQueueEntry` gets `#[derive(Default)]`; it is written out in full three
times (`abilities.rs:225`, `ability_queue.rs:333`, `:431`).

### 2.8 The engine stops knowing about the UI

`game/game_setup.rs` is 2,405 lines; its header (`:1-2`) describes 30 of them.
**~1,050 are action generators** (`:682-1734`) —
`generate_pending_choice_actions:767-1493` is 727 lines,
`generate_main_phase_actions:1735-2326` is 592.

Four couplings keep that in the rules layer: `turn/actions/mod.rs:161, 194, 243,
255, 273, 289` match on `game_setup::ActionType`; **`core/game_state/mod.rs:342`
holds `Option<game_setup::ActionType>` as a `GameState` field**;
`abilities.rs:3202 record_action_boundary`; and `turn/live.rs:1450`,
`phases.rs:34, 57, 1743` call `game_setup::logging_enabled()` from the hot path.

`core` gets a small `ActionId` newtype. The frontend converts `ActionType` to
and from it in one place. Then the generators become `game/actions/`, and
`game_setup.rs` keeps only deck loading, `setup_game`, `settle_*` and
`execute_action`.

`game/web_server.rs` (3,710 lines, Actix, already `#[cfg(feature = "server")]`)
is transport and stays out. It does duplicate three `game_setup` functions
rather than call them — `web_server.rs:1065 is_automatic_phase`, `:1069
is_live_card_set_phase`, `:1073 settle_single_player_state`, against
`game_setup.rs:413, 444, 456` — and re-implements the player table at `:1162`.
Three-line deletions, taken when convenient.

### 2.9 Big functions get seams

- **`effects/executor.rs:47-190`** — a 60-arm `match` on `ActionType` where 20
  arms are the identical 4-line "call and discard `Result`" (`:106-121`). Six
  no-op arms already collapse into one group (`:184-189`); the rest follow.
  `MoveCards` and `DiscardCard` (`:53-54`) differ only by a log tag.
  `ActionType::ModifyYellSource` (`enums.rs:411`) is dispatched at
  `executor.rs:98` as a no-op and gets implemented or deleted.
- **`condition/card.rs:3773-4000 get_count_for_condition`** — 230 lines, 8 early
  returns, no `match`. Every branch re-derives `card_db` +
  `resolve_condition_player` and re-implements the filter. One function per
  resource kind, one shared preamble.
- **`condition/card.rs:2016-2276 resolve_zone_card_count`** — 260 lines, 3-4
  nesting levels; the `Stage` arm (`:2042`) rebinds `stage_cards` four times
  (`:2043` → `:2053` → `:2073` → …). One filter pipeline per zone attribute,
  folded left to right.
- **`condition/state.rs:35-423 evaluate_temporal_condition`** — one arm is 155
  lines at 4 levels (`:36-192`), another 151 (`:197-347`) with 6
  `ABILITY_DEBUG.load()` calls interleaved, and the `_` arm (`:367-422`) repeats
  the same 5-variant `matches!` six times. One function per temporal kind.
- **`cost/handlers.rs:959-1329 pay_cost_inner`** — 375 lines in 9 arms; three are
  56/96/119 lines. `validate_cost:169` and `validate_mandatory_cost:281` are two
  near-identical `SequentialCost`-unwrapping dispatchers over the same action
  set.
- **`choice.rs:1067-1134`** — an 11-arm `match Zone::from_str(zone)` with four
  separate reads of the same discriminant before it (`:991` reveal, `:1030` hand
  cost, `:1046` energy cost). One match, arms carrying the sub-logic.
- **`choice.rs:2699-2803`** — 15 arms; `SelfOrOpponent` alone is 51 lines at 5
  levels of nesting.
- **`condition/card.rs:2839-3010`** — 7 `stage_satisfies_*` calls in sequence.

`recalculate_constants` (734 lines, with a 3DS hang workaround and 17 `tdbg!`
probes) and `execute_change_state` (643) are last. The zone and data-model work
rewrites their surroundings anyway, so they get extracted in that wave. The
workaround and the probes stay.

### 2.10 Deletions

**Dead `pub` items, zero references:** `vm.rs:114 decode_fallback_count` ·
`util.rs:1916 waited_energy_indices` (documented as the complement of
`active_energy_indices:1911`; the complement is never taken) · `util.rs:1729
build_candidate_pool` · `util.rs:1712 matching_ids` · `util.rs:877
card_matches_name_constraint` · `util.rs:2488 get_zone_card_count` ·
`util.rs:1156 CardFilter::has_filter` · `gates.rs:127 effect_condition_gate`
(in none of the three gate lists at `gates.rs:250-253, 259-262, 268`) ·
`gates.rs:145 record_use_limit_if_activation` (sole caller is the dead one
above) · `util/labels.rs:15 member_plural`, `:20 energy_plural` ·
`ability_store.rs:33 AbilityRef::index`, `:37 idx` · `core/card.rs:4230
need_heart_satisfied` (a 2-line alias of `check_heart_requirement:4235`, no
callers).

**Test-pinned but production-dead:** `vm.rs:119 decode_fallback_abilities` ·
`vm.rs:132 count_empty_bytecode_abilities` · `vm.rs:1350 normalize_cost_keys` ·
`condition.rs:123 ConditionContext::allows` (its doc at `:118-121` calls it
*"a single-expression guard in sequential and handler loops"*; no `ability/`
call site uses it).

**A test module inside a production file.** `core/card.rs:410` embeds
`#[cfg(all(test,…))] mod card_id_tests` in shipped source. It moves to
`engine/tests/`.

**Wrappers whose comments are now wrong.** `resolver.rs:897-901` and
`:1010-1013` say *"stays for direct callers that resolve an ability without the
full pipeline"* — but `resolve_ability:1109,1114` is their only caller, so they
**are** the live path. `effects/mod.rs:69-92 prepare_opponent_routing` is a
*"legacy `opponent_action` wrapper (pre-parser-flatten)"* surviving only because
`ActionType::OpponentAction` still exists, and that variant is itself a parser
leftover. `ability_store.rs:74-77 to_arc` is a *"legacy alias"* still called at
`game_state/abilities.rs:1205, 1346`. `util.rs:797-803 card_matches_cost_limit`
is a 3-line wrapper.

**Unreachable variants.** `ConditionType`'s `OrCondition`,
`PositionChangeCondition`, `ActionSuccessCondition` (`enums.rs:569, 578, 579,
581`) have no producer; `NotMoved`/`HasMoved` are produced by 2 of 5 arms;
`HighestCostOnStageCondition:583` is never returned.

**Byte-identical duplicates.** `effect_uses_selected_cards` in `choice.rs:23-34`
and `cost/handlers.rs:16-27`. `evaluate_compound_condition`
`condition/compound.rs:65-113` and `evaluate_or_condition` `:115-134` — the same
function with a different operator. Two arms of
`choice/result_handlers.rs:204-249` with identical `Err(...)` bodies.

**One intent, two behaviours, no comment.** Use-limit recording at
`gates.rs:319-328` vs `resolver.rs:966-992` — the second matches
`"position|destination"` unconditionally, the first requires
`is_optional_effect`.

**"ONE definition" that isn't.** `resolver.rs:1142` says *"ONE definition of the
reduction math"* and then inlines a fourth copy. Sites: `resolver.rs:1145`,
`util.rs:163`, `effects/score.rs:17, 27`, plus a Python generator.

**Two comparison tables, different defaults.** Canonical
`util.rs:1958-1975 compare_with_operator`; inlined `>=` fallbacks at
`condition.rs:654-655` and `condition/card.rs:3273-3274`; a hard-coded default
at `condition/card.rs:642`. And `util.rs:819-825 card_matches_cost_limit_op` has
its own operator vocabulary that **defaults to `<=`** where
`compare_with_operator` defaults to `>=`.

**Two definitions of "has an all-heart."** `condition/card/predicates.rs:8-26`
(base OR modifier OR constant bonus) vs `condition/card.rs:1015-1019` (base
only). The 3-call chain `check_original_blade_filter &&
check_original_heart_filter && check_heart_type_all_per_card` is typed out at
`condition/card.rs:183-188` and `:904-909` while the extracted helper sits unused
at `condition/card/predicates.rs:154`.

**Two "reset" methods and a trigger-scan family.**
`game_state/mod.rs:927 clear_effect_tracking`, `abilities.rs:2745
clear_movement_tracking` (a pure delegate), `modifiers.rs:1574
clear_recently_moved_batch`. And `abilities.rs:407` / `:418` / `:430`, where
the first two differ only in whether `position_change_occurred` is set and the
third is a delegate.
`card_id_lookup_determinism_test.rs:5-12` proves this class of bug reaches the
card-resolution hot path.

**A field and a method with the same name.** `card.rs:834 pub
triggerless_text: Option<String>` and `:863 pub fn triggerless_text(&self) ->
&str`. Same concept, disambiguated only by `Option`, no compiler signal.

**`BladeHeart` and `BaseHeart` are byte-identical** — `card.rs:292` and `:299`,
both a single `HeartMap`.

**13 `#[allow(clippy::too_many_arguments)]` in `ability/`** —
`effects/misc.rs` (5), `condition/card.rs` (3), `move_cards.rs:749, 1871`,
`cost/handlers.rs:472`, `look.rs:526`, `condition/card.rs:1584, 1973, 2838`.
Each is a parameter bundle that wants to be a struct.

**`ConditionContext` visibility.** Methods spread over 5 files with three
visibility levels (`pub`, `pub(crate)`, `pub(super)`) and no coherent surface —
`zone_len` (`condition/card.rs:3277`) is `pub(crate)` despite being the module's
own zone-count table. One visibility per module. Also `condition.rs:29-31
stage_has_any_member` is a free function used only by
`condition/compound.rs:37`, and `compound.rs:40-48` re-implements its body
inline in the adjacent arm.

**Two position mappers, both documented as "single source of truth", in the same
file.** `util.rs:653 activation_position_index` and `util.rs:1821
stage_position_index` differ only by `.trim()` and the Japanese aliases, plus 7
`MemberArea` codecs at `core/zones.rs:74-141`.

**`normalize_card_no` in three implementations, one running backwards.**
`core/card.rs:637` and `core/card_binary.rs:431` go fullwidth→halfwidth;
`game/deck_parser.rs:202` goes halfwidth→fullwidth, the exact inverse. The
direction lives in the name.

**Card type in four enums with three unknown-policies.** `card.rs:34
CardType` (serde takes JA aliases, errors on unknown), `card.rs:2926
ConditionCardType` (unknown → `MemberCard`), `enums.rs:720 EffectCardType`
(unknown → `Other(ArcStr)`). The policies are deliberate and stay; the
spelling unifies.

**Two trigger enums that never meet by value.** `core/types.rs:98
AbilityTrigger` (6 variants, no `Main`/`BatonTouch`, derives Serialize so it can
be a queue field) and `triggers.rs:32 TriggerKind` (8 variants, no derive),
joined only by a debug string — `ability_queue.rs:40` formats `trigger_type`
with `{:?}`. One enum that derives serde.

---

## 2.11 The rewrites that are actually big

Everything in 2.1-2.10 is consolidation: unify this, delete that. These five
are structural, and they are where the engine's real shape changes. Ranked by
value.

### A. `describe.rs`: 68% of the file is a hand-maintained bilingual mirror

`describe_effect_en:174-629` is 46 match arms. `describe_effect_ja:848-1281` is
44. They are **1:1 mirrors** — same arms, same order, `move_cards` at `:185`/`:859`,
`draw_card` `:202`/`:884`, `gain_resource` `:214`/`:896`, `change_state`
`:234`/`:919`, `look_at` `:285`/`:975`, `reveal` `:287`/`:983`, `pay_energy`
`:289`/`:991`, `restriction` `:327`/`:1023`, `activate_ability` `:399`/`:1084`,
and so on through `suppress_ability_trigger` `:620`/`:1269`. The supporting
label tables are duplicated the same way: `zone_label_inner:22` /
`zone_label_ja:752`, `card_type_label_inner:71` / `:756`, `state_verb_inner:97`
/ `:760`, `resource_label_inner:119` / `:764`, `duration_label_inner:141` /
`:768`.

~890 of 1,312 lines. It becomes one table keyed by `ActionType` carrying
`(en, ja)` pairs plus a small slot language for the interpolated fields
(`{count}`, `{src}`, `{dest}`, `{gn}`, `{lim}`, `{dur}`).

Three reasons this is first on the list:

1. **The pattern is already proven in the same file.** `translate_choice_prompt_en_to_ja:789-846`
   (58 lines) is exactly this idea, already applied once. The engine's own
   maintainers did the hard part and left it unapplied to the big table.
2. **There is an oracle for this specific change.** `bin/describe_dump.rs:12-20`
   dumps both languages for diffing. Nothing else in the engine has that.
3. It is a pure function of its input, so the only failure mode is a string
   diff, which is trivially reviewable.

### B. `ActionType`: 66 variants, 9 of which do nothing

`ability/enums.rs:281-375` has 66 variants, and its own comment at `:360` admits
`// Missing variants from effects/mod.rs dispatch`. `effects/executor.rs:98, 145,
147, 184-189` dispatch **nine of them to `Ok(())`**: `ModifyYellSource`,
`DoNothing`, `RepeatProcedure`, `CompoundAction`, `OpponentAction`, `ActionBy`,
`SequentialCost`, `ChoiceCondition`, `EnergyCondition`. All nine have wire
strings from `wire_tables!` (`:379`) and all nine resolve to nothing.

Real families collapse into parameterized actions:

| family | variants | today |
|---|---:|---|
| `set_*` | 7 | 7 near-identical `execute_set_*` in `effects/state.rs:1023, 1079, 1169, 1442, 1475, 1584, 1610` — ~370 lines that all "resolve count/target, write into `gs.mods.<x>_modifiers`" |
| `reveal_*` | 4 | 4 fns in `look.rs:279, 1136, 1313, 1326`; `look.rs:1201 reveal_until_check` / `:1224 reveal_until` are already generic |
| `conditional_*` | 4 | 4 fns in `compound/resolver_impl.rs:19, 28, 37` — and `ConditionalOptional` and `ConditionalOnOptional` are **the same handler** (`executor.rs:129` and `:183`) |
| `modify_*` | 8 | `ModifyRequiredHearts` and `...Global` already share a handler (`executor.rs:105`, `:163`) |

`ActionType` goes 66 → ~45 and ~14 handler functions disappear.

**What not to do:** the `{validate, execute, describe_en, describe_ja}` registry.
`validate` already lives in two places — `cost/handlers.rs:169` and
`resolver.rs:358` — and a registry makes three. Do A and the `set_*` collapse;
leave dispatch in `executor.rs`.

### C. `AbilityResolver`: 44 fields, 258 methods, 18 impl blocks

Struct at `resolver.rs:78-177` is 44 fields. Its `impl` blocks are in **18 places
across 17 files**: `choice.rs:72` (48 methods), `effects/misc.rs:174` (36),
`move_cards.rs:200` (31), `look.rs:16` (25), `resolver.rs:179` (23),
`effects/state.rs:19` (19), `cost/handlers.rs:29`+`:168` (18),
`effects/draw.rs:83` (13), `effects/mod.rs:26` (9),
`effects/ability_effects.rs:12` (7), `compound/resolver_impl.rs:8` (6),
`effects/score.rs:163` (6), `effects/reveal.rs:10` (5),
`move_cards/selection.rs:5` (5), `move_cards/placement.rs:55` (4),
`move_cards/transport.rs:96` (2), `effects/custom.rs:14` (1). Roughly 17,500
lines, seven concerns on one type.

**The decomposition is already written — as field clusters.** The 44 fields
group by what they exist for, and three groups are *already separate types the
resolver holds by value*:

- **Cross-step data flow (mechanical).** `execution_context:88`, `pipeline:127`,
  `step_state:130`, `spawn_context:113`, `current_effect:89`,
  `parent_effect:92` → one `EffectRun`. `StepState` and `EffectSpawnContext` are
  already types (`types.rs:1091`, `:882`).
- **C6 keep-N-shuffle-rest (5 fields, one card pattern).**
  `keep_shuffle_under_phase:106`, `_count:107`, `_snapshots:108`,
  `keep_shuffle_selected:112`. 4 of 44 fields = 9% of the struct for one
  mechanic.
- **Look/reveal origin (4 fields, one reason).** `looked_at_origin:160`,
  `looked_at_deck_position:165`, `stage_select_intent:168`,
  `selected_count_at_save:124` — all exist because, per the comment at
  `:161-164`, *"The answer-time handler has no effect context."* A struct with a
  **builder the producer fills** removes the need for these fields at all.
- **Replacement effects (3).** `pending_replacement:173`,
  `resolving_replacement:174`, `replacement_original_suppressed:175`.
- **Logging (3, not gameplay).** `log_items:142`, `debug_trace:126`,
  `last_offered_sig:155`.
- **The rest (29)** is per-ability scratch that `resolve_ability` already resets
  — which `resolver.rs:58-70` promises in its own doc.

44 fields → ~7 groups. That also kills most of the 14
`#[allow(clippy::too_many_arguments)]` suppressions, because a parameter bundle
that is 3 of the resolver's fields becomes one argument. The newest code already
does this: `choice.rs:660-672 reprompt_hand_cost` has 11 params but 7 of them
are `SelectionContext`-shaped (`types.rs:476-550`, a 20-field builder), and
`handle_hand_selection:1169` is down to 5-6. **The `too_many_arguments` list is
the migration backlog, not a lint.**

**Do not reorganise the 18 impl blocks** — they are already split by file. The
win is nesting, not moving.

### D. `GameState` is 140 fields and `GameModifiers` is 36

`core/game_state/mod.rs:81-359` — 140 fields. `core/game_modifiers.rs:131-204` —
36 fields, of which **19 are `HashMap`**.

Two slices are clearly worth doing on their own merits:

- **8 of the 36 (22%) are pure UI attribution, not gameplay.** The 7
  `*_sources: Vec<BonusSource>` plus `constant_score_sources:155`.
  `BonusSource`'s own doc (`game_modifiers.rs:100-104`) says it exists so *"the UI
  can show WHERE each bonus on a card comes from."* They are rebuilt every
  `recalculate_constants` pass purely to feed `game_state_to_display`. They move
  to a telemetry struct. No rule is touched.
- **8 of the 19 maps are derived shadows.** `constant_{blade,cost,cost_set,
  score,heart}_bonuses:140-145` and `success_zone_{blade,heart,score}_bonuses:198-204`.

Those shadows are the subject of a real correctness question, below. Fix that
first — it may make the full decomposition unnecessary.

**The full 4-way split is not in this plan.** `BoardState` / `TurnScratch` /
`GameModifiers` / `Telemetry` is the right shape, but it is ~200 `gs.<field>`
sites and it breaks `bot/v8_model.rs:121-137`,
`bot/determinization.rs:87-109`, `bot/observation.rs:810, 847` (which write
through public engine fields) and the serde wire format the 3DS client reads.

### E. `bot/`: 13,538 lines, 4,183 of them are six dead generations

| file | lines | |
|---|---:|---|
| `strategy_v7.rs` | 2008 | current-ish |
| `v8_model.rs` | 1802 | current (~1280 prod, 29 tests at `:1346-1800`) |
| `v8_live.rs` | 976 | current |
| `strategy_v3.rs` | 879 | **dead** |
| `observation.rs` | 883 | current |
| `v7_main.rs` | 717 | superseded by `v8_main` |
| `rollout.rs` | 624 | v7-era |
| `strategy_v4.rs` | 623 | **dead** |
| `v8_main.rs` | 592 | current |
| `strategy_v2.rs` | 491 | **dead** |
| `encoding.rs` | 441 | current |
| `strategy_v5.rs` | 448 | **dead** |
| `strategy_v6.rs` | 397 | **dead** |
| `neural.rs` | 364 | PPO only |
| `v7_live.rs` | 359 | superseded by `v8_live` |
| `conductor.rs` / `strategy.rs` / `determinization.rs` / `strategy_common.rs` / `registry.rs` / `strategy_v8.rs` / `ismcts.rs` | 282-197 | current |
| `evaluation.rs` | **13** | **empty stub** |

v1-v6 is 3,107 lines and `registry.rs:96-101` **still dispatches to it**;
`v7_live.rs` + `v7_main.rs` add 1,076 more. Deleting them removes 4,183 lines
(31%) and is a `registry.rs` enum-narrowing. `bot/` is `#[cfg(not(feature =
"no_std"))]` (`lib.rs:54-55`), so it is absent from every console build and
nothing outside the engine's own graph is affected.

Two live defects sit in here:

- **`encoding.rs:402` aliases three actions to one feature.**
  `action_type_index` maps `ActionType::Pass => 0` (`:377`) and then
  `ActionType::Concede | ActionType::DrawCard => 0`. Index 0 *is* Pass, so
  **conceding, drawing and passing are indistinguishable to the policy net** —
  and a wrong call there is unrecoverable. Related: `ACTION_TYPE_COUNT = 25`
  (`:7`) against 28 `ActionType` variants (`game_setup.rs:97-126`), with the
  `.min(ACTION_TYPE_COUNT - 1)` clamp at `:222` and
  `train_ppo.py:80`'s `clamp(0, NUM_ACTION_TYPES - 1)` both silently folding
  any future variant into row 24 instead of failing.
- **`encoding.rs` is maintained in two hand-synced places** — `encoding.rs:4-19`
  and `training/train_ppo.py:22-48`, which restates `CARD_EMBED_DIM`,
  `NUM_ZONES`, `ACTION_TYPE_COUNT`, `GLOBAL_FEATURES`, `SCHEMA_VERSION` and
  `ACTION_STRUCT` by hand. `save_weights:97-114` likewise must match
  `bot/neural.rs`'s load order by hand.

**The `bot/encoding.rs` `ZoneId` is not a duplicate of the engine's `Zone`** and
2.1 must not absorb it. It flattens `(player × zone × stage-slot)` into a fixed
15-row tensor index with `MyStagePos0/1/2` as three distinct rows, and
`is_sum_zone:61` is a pooling concern. That is the right shape for a tensor. It
does go stale against the engine's zone set, so it gets its own
compile-time-derived index rather than a hand-synced one.

**No full `bot/` rewrite.** `v8_model.rs:1-30` is the best documentation in the
repo, it carries 29 tests, and the PPO↔Python hand-sync is a contract that no
refactor improves.

### F. `turn/`: 7,343 lines, and the phase machine is the soft spot

`live.rs` 3161 (25 helpers + 4 free functions — **well decomposed**, biggest is
`check_live_success` at 164), `actions/mod.rs` 1863, `phases.rs` 1797,
`triggers.rs` 504, `mod.rs` 18 (literally `pub struct TurnEngine;` with the
comment "Stateless by design").

The problem is `advance_phase`, `phases.rs:75-316` — 241 lines holding **two
parallel `match` trees over the same enum** (one for
`{FirstAttackerNormal, SecondAttackerNormal}`, one for `Live`) with the
34-line turn-rollover reset (`:278-311`) inlined inside the
`LiveVictoryDetermination` arm. `Phase::Active` alone is 88 lines (`:94-181`).

It becomes a phase table. It should not, though: the arms are not homogeneous
(`Phase::Energy:182-192` is 10 lines, `Phase::Draw:193-201` is 8,
`Phase::Active` is 88), and making them homogeneous to fit a table is branch
regrouping. **Extract the rollover reset into `end_turn(&mut GameState)` and
leave the rest.** That 34-line move is worth doing and nothing else here is.

Also in scope by size, not by rewrite: `actions/mod.rs:766-997
resume_with_choice` (231 lines) owns the `ResumeSnapshot` protocol
(`actions/mod.rs:59`) that 8 other functions replay (`:1409, 1426, 1455, 1488,
1530`); and the two biggest functions in the crate after
`recalculate_constants` are `core/game_state/abilities.rs:543-962` (419 lines)
and `:1825-2234` (409 lines) — which belong to 2.7.

---

## 2.12 Two real defects found while planning

Neither is a refactor. Both are worth more than most of the above.

### `bot/encoding.rs:402` — Concede/DrawCard/Pass share a neural feature

Described in 2.11E. One line.

### `GameModifiers`: `set_*` writes are invisible to the constant-recalc diff

`core/game_state/modifiers.rs:214-249` `commit_constant_results` reconciles by
diffing a shadow map against a freshly computed one:

```rust
let old_blade = core::mem::take(&mut self.mods.constant_blade_bonuses);
if old_blade != exp_blade {
    for (cid, val) in &old_blade { self.mods.remove_blade_modifier(*cid, *val); }
    for (&cid, &val) in &exp_blade { self.mods.add_blade_modifier(cid, val); }
}
self.mods.constant_blade_bonuses = exp_blade;
```

`add_score_modifier` / `remove_score_modifier` (`game_modifiers.rs:406-418`)
touch **only `additive`, never `set`**, while `ModifierEntry::total()` is
`set + additive` (`:94-97`). So a modifier written through the `set` path is not
in the shadow map, is not in `old_score`, and is therefore **never removed** by a
later `recalculate_constants`.

Four independent write paths reach `score_modifiers`, and none of them update
`constant_score_bonuses`:

1. `ability/effects/score.rs:488` `set_score_modifier` (direct effect,
   `operation == "set"`) and `:498` `add_score_modifier`
2. `ability/effects/ability_effects.rs:485` `add_score_modifier`
3. `core/game_state/modifiers.rs:2069` `add_score_modifier` (success zone)
4. `turn/live.rs:808` `add_score_modifier(cid, -delta)` and `:815`
   `set_score_modifier` (`revert_live_success_score_modifiers`)

`clear_all_for_card` (`game_modifiers.rs:567-583`) is a hand-written 15-field
reset that clears the shadow maps but not `constant_*_sources`,
`success_zone_*_sources`, `constant_cost_set_bonuses`, or
`success_zone_*_bonuses`.

**This is a hypothesis, not a proven bug** — I have not run the suite, and
`AGENTS.md:44` is explicit that no global-state theory is worth anything without
a direct probe. It is, however, exactly the class of defect that a 19-`HashMap`
struct exists to cause, and it is worth the half-day to write the probe: set a
score modifier via the `set` path, force a `recalculate_constants`, and read
`score_modifiers` back. If the modifier survives, the shadow map is unsound and
the fix is to make the `*_bonuses` maps genuinely derived instead of
diff-reconciled.

---

## 3. Order


| Wave | Content |
|---|---|
| 0 | The two live defects (2.12), deletions that cannot change behaviour (1.8, 2.10), `kind` made serializable, `build.rs:295-299` uncommented so a stale generated artifact is an error rather than a `println!` |
| 1 | `describe.rs` bilingual mirror → one table (2.11A), `ActionType` family collapse and the 9 dead variants (2.11B) |
| 2 | The 38 integer sites (1.7), `bot/` dead generations (2.11E) |
| 3 | The parser module tree (1.1) |
| 4 | Parser vocabulary (1.4, 1.6) |
| 5 | Parser cascades, then the typed IR (1.3, 1.2) |
| 6 | Engine zones, players, card lookup, filters (2.1, 2.4, 2.6) |
| 7 | Engine movement, queue, UI unweld (2.3, 2.7, 2.8), `GameModifiers` attribution split (2.11D) |
| 8 | Engine data model, `AbilityResolver` field-nesting, the big functions (2.2, 2.11C, 2.9) |
| 9 | Retire the repairers (1.5) |

Order is chosen so each wave makes the next cheaper, and so the two things with
a real oracle go first. `describe_dump` covers 2.11A and the `set_*` half of
2.11B exactly, which is why they are wave 1 rather than wave 8. The cascades are
declarative before the typed IR lands on them; the typed IR exists before the
repairers are retired; the zone model lands before the data model that depends
on it; and the resolver is nested last because nesting 44 fields is only worth
doing once the data model underneath has stopped moving.

Rust engine only. `engine_c_wip/` is a separate deliverable with its own
instructions and is not an input to anything here.

---

## 4. What this leaves

`parser.py` becomes a ~1,500-line entry point over a module tree, with the
repair layer at zero. The engine has one `Zone`, one `transfer_cards`, one data
model, one describe table, one filter query, one queue driver, and 43 → 0 inline
zone tables.

The reason to do it is not the line count. It is that "how does a card get
moved" currently takes a dozen files and a research pass, and afterwards it is
one function. "What are the zones" is one enum. "What can a condition be" is one
type.

**Not in this plan.** `game/web_server.rs` — transport, already feature-gated.
`kind_from_action` (`card.rs:1467`, 287 lines) — it looks like an if-ladder and
is a 282-line `match` with one shared filter over 60 heterogeneous variants; a
table cannot help, measured before editing per
`REFACTORING_APPROACH.md:116-118`. Leaving it alone is a decision. There is no
commented-out code to find in either subsystem; both were cleaned, and the "129
findings, 2 real" figure in `REFACTORING_APPROACH.md:83` was a bad heuristic
rather than a code problem.
