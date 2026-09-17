# Parser ↔ Engine Representation Comparison (IN PROGRESS)

**Status:** investigation started — this file is updated continuously as review proceeds.
**Created:** 2026-09-18. HEAD at creation: `d1993354` ("Fix gained-ability tracking and test choices; improve profiling, diagnostics, and CI").
**Supersedes:** the old remediation-queue framing in `AUDIT.md` for this task only; `AUDIT.md` remains the historical audit.

## Scope & Evidence Standard

Exhaustive, function-by-function / line-by-line comparison of the two representations of card abilities:

1. **Parser side** — `cards/ability_extraction/parser.py` (Python), which extracts ability text into `cards/abilities.json`.
2. **Engine side** — Rust model in `engine/src/ability/enums.rs` + `core/card.rs` and the compiled wire form consumed by `vm.rs` via generated decoders.

Goal: identify every place the representations disagree, can be merged, refactored, rewritten, or improved. Distinguish:
- **Verified** — exact file:line evidence read this pass, quoted or cited.
- **Held over** — evidence from the previous audit (marked `held over`; re-verify before relying on it).
- **Unreviewed** — explicitly listed as not yet examined.

Method: three end-to-end anchor abilities traced parser→JSON→compile→decode→resolve, then a coverage ledger walking every top-level parser function against the engine consumer. Repeated cross-check passes are recorded in the ledger with dates.

## Known-true background (verified earlier, held over)

- Corpus manifest `cards/build/generation_manifest.json`: schema `compiled_abilities.v1`, 936 unique abilities, 2011 card/ability pairs, 5719 strings; raw-with-header 93,023 B, compressed 28,653 B.
- Both `compile_cards.py` and `compile_abilities.py` write the same `cards/build/generation_manifest.json` with different schemas (last writer wins); manifest hashes truncated to 16 hex chars; abilities compiler overrides `output.sha256` with headerless payload hash.
- `engine/src/ability/ability_store.rs:45–71` (`held over`): `AbilityRef::resolve` uses cached slots on std; `decode()` logs error then returns `Ability::default()` — silent default on decode failure is the known engine-side hazard.
- `enums.rs` `wire_tables!` generates wire conversions; `from_str` returns `None` on unknown strings.
- Chika anaphora: printed text `abilities.json:2495` (waited member gains +1 until live end); parsed effect `abilities.json:2514–2543` has **no explicit target**; engine gained the `last_cost_waited_members` text heuristic in `execute_gain_ability_effect` (`ability_effects.rs`, contains `"これによ"`). This heuristic is itself an audit target: stale tracker, overmatching, explicit-target semantics.
- `check_expired_effects` (`game_state/abilities.rs:2585+`, `held over`) tracks expired `gain_ability:` effects and refreshes constants after removals.

## Coverage Ledger

### Parser (`cards/ability_extraction/parser.py`)
- Size: 578,466 bytes; PowerShell counted 12,806 nonblank lines; 266 top-level `def`/`class` (inventory at `%TEMP%\kilo\parser_inventory.txt` — **not yet** semantically reviewed).
- **Unreviewed:** all 266 definitions; previous large Read outputs were truncated and do not count as review.

### Engine enums (`engine/src/ability/enums.rs`, 863 lines)
- Enum positions: `Zone:35`, `TargetPlayer:213`, `PlacementTarget:261`, `ActionType:281`, `ConditionType:553`, `SelectTargetKind:634`, `EffectCardType:720`, `EffectState:799`.
- **Pass 1, 2026-09-18:** all 863 lines read without truncation; all conversion/serde/default/label implementations inspected. PE01/PE02 record contract-level findings. Full post-normalization corpus vocabulary comparison remains unreviewed.

### Parser helper and focused test coverage

| Function / table | Complete range read | Cross-check / status |
| --- | --- | --- |
| Source/destination/location/card/state/position tables | `parser_utils.py:392–565` | Compared zone, card-type and state values with Rust enums; normalization reachability still open. |
| `detect_require_all_hearts` | `parser_utils.py:595–607` | Read; engine consumer not reviewed. |
| `extract_position` | `parser_utils.py:609–612` | Rust accepts left/right aliases; field-specific engine consumers not reviewed. |
| `extract_cost_limit`, `extract_cost_limit_with_operator`, `extract_cost_values` | `parser_utils.py:614–658` | Read; consumer comparison pending. |
| `detect_card_property` | `parser_utils.py:661–668` | Read; consumer comparison pending. |
| `extract_source` | `parser_utils.py:671–699` | Earliest FROM-marker wins, then regex/table fallback. Rust zone conversion compared, action normalization pending. |
| `extract_destination` | `parser_utils.py:702–723` | Table precedence and special fallbacks inspected; PE01 flags noncanonical helper values. |
| `extract_target`, `extract_picker` | `parser_utils.py:726–759` | Player vocabulary accepted by Rust; full ownership/picker propagation pending. |
| `extract_card_type` | `parser_utils.py:762–765` | Known emitted types accepted by `EffectCardType`; shared matcher not yet fully read. |
| Parser classification checks | `tests/test_parse_action.py:1–160` | Complete functions within range read; PE03 identifies permissive oracles. Remaining test file unreviewed. |
| Zone conversion tests | `engine/tests/test_modules/rules/zones/zone_conversion_test.rs:1–26` | Complete file read: exercises seven physical-zone variants via ZoneId, not serde or LiveTotal. Energy alias normalization is explicitly intended in this test. |

A source read does not count as engine semantic validation. Helper coverage is separate from the 266-definition parser inventory.

### Wire path
- `vm.rs` includes `effect_decoder_gen.rs` / `condition_decoder_gen.rs`; platform blob paths known. **Unreviewed:** decoder ↔ model field mapping.

## Initial Findings

### PE01 — Zone mixes locations, references, and destinations; conversion is lossy

**Verified API mismatch, not yet a demonstrated current-card failure.** Complete `engine/src/ability/enums.rs:1–863` read in two untruncated chunks.

- `Zone::to_str` emits `live_total` and `unknown` (`147–148`), but `Zone::from_str` (`79–112`) accepts neither. `from_source_str` (`158–160`) and serde deserialization (`201–205`) convert unrecognized strings to `Unknown`. A `LiveTotal` value therefore cannot survive serialize→deserialize unchanged.
- `EnergyZone` serializes as `energy_zone` (`125`), but that string parses as `Energy` (`88`), not `EnergyZone`. This may be intended alias canonicalization, but keeping both variants makes variant round-trip equality false.
- Parser helper tables emit `revealed_remaining` (`cards/ability_extraction/parser_utils.py:396`) and `deck_position_4` (`450–451`); neither is accepted by `Zone::from_str`. Later parser normalization may rewrite these values: current-corpus reachability remains unverified and must not be called broken gameplay yet.
- Physical zones, relative references (`those_cards`, `preceding_moved`), and placement instructions (`same_area`, `deck_top_or_bottom`) share this one enum. The type does not express which actions accept which category.

**Refactor candidate:** separate canonical physical zones from card-set references and destination instructions. Normalize aliases once at the parser/validated IR boundary; reject unknown values with field-path diagnostics rather than collapse them. Generate Python vocabulary and Rust wire conversions from the same schema, keeping Japanese phrase matching in Python. Add variant-round-trip and accepted-alias tests, followed by corpus-wide post-normalization checks before deleting variants or changing wire strings.

### PE02 — Typed vocabulary has inconsistent unknown-value policies

**Verified contract inconsistency; not every permissive conversion is itself a defect.**

- `ActionType` and `ConditionType` use `wire_tables!` (`enums.rs:15–32,377–458,589–626`), returning `None` for unknown strings. `ActionType::try_from` raises a descriptive error (`698–702`), while its default is `Custom` (`533–537`).
- `Zone` loses the original unknown string (`158–160`). `EffectCardType` and `EffectState` instead preserve it as `Other(ArcStr)` (`729–747,805–821`); their defaults preserve an empty string (`779–783,853–857`).
- `TargetPlayer` only represents the player subset of the raw overloaded target field (`208–247`); the type's own comments acknowledge that other callers use destinations and ability references in the same field. `extract_target` produces exactly `both/either/opponent/self` (`parser_utils.py:726–744`), all accepted here, but does not solve that broader overloading.
- `SelectTargetKind` defines runtime interaction tags (`634–696`), including delimiter-bearing values. These are not interchangeable with parser card/player targeting. `PAY_SKIP_TARGET` is consistently referenced in both directions (`658,680`; definition `engine/src/ability/types.rs:32`); no defect established in that constant.

**Refactor candidate:** define strict authored-IR validation separately from explicit legacy compatibility decoding. Use distinct typed player ownership, chosen-card references, destination, and UI interaction fields. Do not mechanically merge these enums just because the field is called `target`. Consolidate duplicate serde serialization bodies for card type/state (`749–777,823–851`) only after checking no_std allocation/ArcStr differences.

### PE03 — Parser tests can accept the documented wrong representation

**Verified test-oracle weakness, not evidence the parser currently takes the wrong branch.**

`cards/ability_extraction/tests/test_parse_action.py:49–67,127–152` accepts either the intended classification or a historically incorrect classification. For example, the deck-search checks return for `select` but also pass when the result is `move_cards`. Green results cannot establish selection-vs-movement fidelity. The helper `check` (`15–33`) supports precise action and field assertions and can be reused when expectations are confirmed from printed text.

**Improvement candidate:** once each phrase's intended IR is independently established, make each case assert exactly that representation and relevant fields; retain engine tests for actual choice creation/resumption. Do not treat the current parser test count as exhaustive semantic coverage.

## Open Questions

1. Does the parser emit any vocabulary strings the engine `from_str` tables reject (or vice versa)?
2. Which parser fields have no engine consumer (dead JSON fields), and which engine fields have no parser producer?
3. Can the three serialized surfaces (JSON, wire bytecode, C `bytecode_data.c`) be generated from one schema, given the manifest last-writer-wins collision?

## Working Notes

- Concurrent WIP in this tree (bot v7, ability_effects, C resolver, docs, generated inventory) — read files fresh; do not stash/reset.
- Old failing WIP tests are out of scope per user instruction.
