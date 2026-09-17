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

## Exhaustive pass tracking

The user requested an exhaustive pass after the initial seven findings. Eight bounded reviews are already in progress; no further background reviews will be launched. Their assignments are coverage obligations, **not completed review**:

| Review partition | Scope | Status |
| --- | --- | --- |
| Parser entry/conditions | parser.py 1–5530 and parser_utils.py | In progress |
| Parser effects | parser.py 5531–10550, complete crossing functions | In progress |
| Normalization/extraction | parser.py 10551–end, card_overrides.py, extract_card_abilities.py | In progress |
| Condition/filter consumers | condition.rs, condition/*.rs, dynamic_count.rs and relevant filters | In progress |
| Resolution lifecycle | resolver/cost/compound/choice, queue and trigger/expiry bridges | In progress |
| Effect execution | effects/*.rs, move_cards/look/util | In progress |
| Corpus ledger | Every unique ability, identity mapping, recursive fields/vocabulary | In progress; mechanical coverage is not semantic verification |
| Wire/model remainder | Complete card model/accessors, remaining generated condition constructors, C tag consumers | In progress |

Current revision observed: `37b8325f`; concurrent runtime changes remain in the working tree. Findings from the initial `d1993354` pass require rechecking when affected code changes. Completed reports must provide function ranges and remaining gaps before integration. The eventual corpus ledger must distinguish inventoried, source-mapped, and behavior-verified rows.

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
- Historical inventory is not semantic coverage. Complete functions reviewed this pass are listed below and in Anchor A; all others remain unreviewed. Previous truncated reads do not count.
- Coordinator additionally read complete `extract_location:355–357`, `extract_locations:360–370`, `extract_state_change:373–375`, `extract_cost_range:378–384`, `_mark_live_total_score:11144–11160`, and `_mark_live_total_clamp:11163–11202`. Consumer coverage is incomplete. `_mark_live_total_score` writes `target="live_total"`, not a source/destination Zone: this does **not** establish a current-card occurrence of PE01's serde failure.

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
- Decoder/model comparison completed for the fields and functions listed under PE04–PE07 and Anchor A. Remaining variant constructors, artifact freshness, and whole-corpus parity are unreviewed.

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

### PE04 — Compiler-approved action aliases become runtime Custom

**Verified accepted-input incompatibility; current-card occurrence counts unknown.** Independent source pass reviewed `cards/compile_abilities.py`, both decoder generators, `vm.rs`, `ability_store.rs`, and the full generated effect decoder.

Compiler table `cards/compile_abilities.py:94–164` accepts `draw`, `look`, `reveal_effect`, `double_baton_touch`, and `set_card_identity_all_regions`. None appears in `ActionType`'s exact wire table (`engine/src/ability/enums.rs:377–458`). Generated action dispatch logs unknown strings but returns successful field consumption (`engine/src/ability/effect_decoder_gen.rs:38–47`; generator `cards/generate_effect_decoder.py:252–268`). The initial default action is Custom (`vm.rs:1391`, `enums.rs:533–536`). The numeric family tag is ignored (`vm.rs:1388`), and final kind construction uses the string-derived action (`1462–1465`).

**Recommendation:** centralize action names and aliases; normalize aliases before serialization or reject them. Validate the numeric family against the action or remove it in a versioned format revision. Do not assume current cards exercise these aliases, or that ordinary `draw_card` is broken.

### PE05 — JSON-path reconstruction is not a faithful bytecode oracle

**Verified JSON-path discrepancy, not equivalent field loss in bytecode.**

`engine/src/core/card.rs:1503–1532,1611–1653` (`kind_from_action`) sets card_type, distinct, quoted_text, cost_total_operator, activation_condition_parsed, ability_filter, options, or_ability_filters, and placement/remainder fields to None. Generated bytecode decoding reads these fields (`engine/src/ability/effect_decoder_gen.rs:70–96,134–175`). Nested JSON `effect_field!` creates text-only default effects (`core/card.rs:1488–1500,1676–1711`). `vm.rs:1504–1599` recursively rebuilds kind but does not restore missing action/count/source fields, and cannot recurse through discarded options.

**Recommendation:** correct JSON reconstruction before relying on deep comparison as an oracle; use independent fixtures with nested actions, filters, placement, and optional branches. Shared schema-generated field plumbing can reduce omissions, but independent printed-behavior assertions must remain to catch shared bugs.

### PE06 — Unknown action/object handling depends on nesting

**Verified behavior; malformed-payload consequences not reproduced.**

An action absent from the compiler table becomes TAG_OBJECT (`cards/compile_abilities.py:237–248`). Runtime handling rejects it as a top-level effect/cost, skips it in effect vectors, or returns None after consuming only its tag in unsupported nested effect/condition readers (`engine/src/ability/vm.rs:550–560,877–915,1316–1342`). Payload bytes may then be left unread; exact later damage depends on input position and bytes. Unknown effects log, unknown conditions increment fallback audit, and unknown ability-level fields silently skip (`generate_effect_decoder.py:346`, `generate_condition_decoder.py:270–271`, `vm.rs:1296–1298`). Decode errors finally become default abilities, cached on std (`ability_store.rs:45–70`).

**Recommendation:** distinguish missing/null from invalid input; reject unsupported authored actions at compilation with field paths, and use consistent whole-object consumption or explicit decode errors. Keep compatibility mode explicit instead of conflating it with valid decoding.

### PE07 — Condition tags currently align, but normalization and schema ownership diverge

**Confirmed tag parity plus maintenance risks and input-contract differences.**

- Compiler hardcodes condition IDs 0–19 (`cards/compile_abilities.py:58–92`); generator uses Rust declaration order (`cards/generate_condition_decoder.py:304–307`). These match current `core/card.rs:3092–3290` and `condition_decoder_gen.rs:1523–1556`. Reordering is a risk, not a current mismatch.
- Compiler cost normalization recognizes restricted `type`, not `cost_type`, prefers `costs` over `options`, and mutates source dictionaries, including removing condition type (`compile_abilities.py:166–174,205–248`). JSON normalization accepts arbitrary `type` or `cost_type` and prefers `options` over `costs` (`vm.rs:1350–1385`). Dual-key corpus inputs and repeated in-memory compilation were not exercised.
- Generators parse Rust using regex/line structure; unsupported field types are omitted or skipped (`generate_effect_decoder.py:66–196,270–322`; `generate_condition_decoder.py:65–135,185–195,260–269`). This is a future field-loss risk, not proof a currently reviewed field lacks a reader.

**Recommendation:** explicit stable wire IDs, pure shared normalization with conflict rules, generator errors for unsupported field types, and schema-level parity tests. Correct the generator documentation asserting identical serde reconstruction and zero decoder changes for new fields (`compile_abilities.py:498–507`) only when the actual contract is established.

### PE08 — Existing coverage gates do not establish cross-layer fidelity

**Verified validator blind spots; one existing gate currently fails.**

- Complete `cards/validate_schema.py:1–120` read: `COMPILE_PATH` is printed but compiler contents are never read. `extract_handler_actions:57–63` collects Rust variant names, whereas `check_schema_vs_engine:66–82` compares handler function names against that set and then does nothing on mismatch. Handler coverage only warns on absent documentation (`85–91`). The advertised schema/compiler/engine check therefore cannot verify compiler parity or handler existence.
- Complete `cards/audit_emitted_keys.py:1–98` read: declared names are unioned across all structs/enums in card.rs (`35–45`) and emitted keys across every nested cost/effect node (`48–64`). A field declared on an unrelated type can pass; field paths, values, conditional ownership, and actual decoder/consumer support are not checked. Allowlisted unknowns also pass. The non-updating command `python -B cards/audit_emitted_keys.py` returned exit 1 with `baton_touch`, `energy`, `max_repeats`, and `zone`. This proves gate failure, **not that all four are dropped gameplay semantics**; node-specific consumer checks remain required. Log: `%TEMP%\\kilo\\comparison_emitted_keys.txt`.
- Complete `cards/ability_extraction/tests/test_registry_coverage.py:1–141` read: effect/condition rules are called individually against raw or parsed texts (`62–107`), exceptions are swallowed in hit checks, and names are collected if any handler matches. This checks matchability, not whether an earlier-priority rule shadows the handler in real dispatch. `_ACTION_REGISTRY` is imported but no action-registry coverage test is implemented. The no-crash test (`110–121`) does not assert meaning.

**Recommendation:** separate vocabulary/schema conformance, actual dispatch reachability, decoded-field survival, and printed-behavior coverage. Validate fields by typed node path, instrument the actual winning rule, and make compiler/decoder unknown-input handling explicit. Preserve a clear distinction between a static gate passing and an ability behaving correctly.

### PE09 — Pipeline report can print semantic failure and still exit successfully

**Source-proven exit-status defect; not executed as a reproduction.** Complete `cards/pipeline_report.py:1–227` read. `s5_validation:149–167` returns True for regressions and prints FAIL. `main:202–223` invokes it via `fn(data)` without collecting the return, then unconditionally exits 0 unless the separate key audit exited first. Thus a semantic regression alone is not reflected in process status.

The report duplicates the emitted-key scanner (`known_card_rs_keys:114–123`, `s4_key_audit:126–146`) but includes additional DOCUMENTED_KEYS (`37–46`) not shared with `audit_emitted_keys.py`. Those include all four currently failing gate keys. Two tools can consequently disagree about the same corpus without either validating node-specific consumers. Its `s6_untested:170–196` only checks whether any associated card ID appears in test source, not whether the ability index, branch, or assertion is exercised. Its suspicious-effect classification (`95–111`) examines every dict node, including metadata without an action, so such children can prevent an all-custom tree from being reported.

**Recommendation:** unify key dispositions with field-path consumer evidence; aggregate all section failures into exit status; distinguish mention coverage, executable-path coverage, and assertion coverage. Keep full anomaly output available rather than only the first 15/30 examples.

### Test and validator function ledger (coordinator pass)

| File and complete functions | Reviewed range | Coverage conclusion |
| --- | --- | --- |
| `audit_emitted_keys.py`: known_fields, emitted_keys/walk, main | 1–98 | Full read; PE08 union-key limitation and recorded exit-1 result. |
| `validate_schema.py`: err, warn, ok, load_schema, extract_action_type_variants, extract_handler_actions, check_schema_vs_engine, check_handler_coverage, main | 1–120 | Full read; compiler not inspected, handler mismatch ignored. |
| `pipeline_report.py`: load, walk, s1_corpus, s2_histograms, s3_suspicious, known_card_rs_keys, s4_key_audit/collect, s5_validation, s6_untested/base_ids, main | 1–227 | Full read; PE09. No run claimed. |
| `test_registry_coverage.py`: load_ability_texts, _extract_effect_texts, test_all_effect_rules_triggered, test_all_condition_rules_triggered, test_parse_ability_no_crash | 1–141 | Full read; matchability is not winning-dispatch coverage. |
| `test_ability_invariants.py`: test, walk_nodes, load, test_self_appearance_has_no_card_type, test_or_condition_aggregates_trigger_event, test_appearance_has_trigger_event | 1–133 | Full read; three structural invariants, not per-card engine proof. OR invariant compares counts, not child-event identity. |
| `test_parse_action.py`: check and all test_* functions | 1–294 | Full read; precise field checks coexist with permissive known-bug checks (PE03). |
| `test_parser_coverage.py`: test and all test_* functions | 1–262 | Full read; test_cost_none has no assertion; two sequential tests allow None. Recent moved-subject tests do assert specific source/filter fields. |

### PE10 — Existing test inventory cannot certify exhaustive ability coverage

**Verified source-level limitations; no gameplay failure claimed.** Complete `cards/test_inventory.py:1–1527` read in contiguous chunks.

| Inventory claim or operation | Actual implementation | Consequence for this audit |
| --- | --- | --- |
| Ability has covering tests | `build_inventory:825–866` matches card/base substrings anywhere in a file and credits every test function in that file. | References, comments, setup-only cards, and unrelated test functions can all contribute. Each ability index and asserted behavior still needs independent mapping. |
| L1/L2/+choice depth | `infer_ability_depth:664–680` scans whole covering files for `assert`, choice tokens, and negative-looking file/function names. | A depth label is a heuristic, not evidence that the particular ability fired or its negative/choice path was checked. |
| `@covers ... depth=L2` overrides depth | `COVERS_RE:106` captures only a card token; `build_inventory:843–866` records that token but does not parse or apply depth. | The documented depth override is not implemented. |
| QA ruling is covered | `build_qa_coverage:683–718` searches all Rust source, including non-test code, using substring matches for Q-IDs or related cards. | An implementation comment or a longer Q-ID containing the shorter one can count; this does not establish a ruling assertion. |
| JSON includes full covering tests | `build_inventory:927` truncates the list to 30 before JSON rendering; `render_inventory_md:1343` calls it full. | The retained count can exceed the saved list; it cannot serve as a complete test-to-ability ledger. |
| Family variants discriminate behavior | `build_families:721–761` uses matcher return values as variant keys. `FAMILY_VARIANT_EXTRACTORS:200–208` is not used there. | Boolean matchers can group distinct draw counts or movement destinations under the same `True` variant. Family membership is not behavioral equivalence. |
| Family output lists every member | `render_families:795–800` emits at most 40 rows but compares the original rows against an equally long sorted list when checking for omitted rows. | Groups larger than 40 can omit members without an omission notice. |

**Concrete audit requirement:** use an independent per-ability ledger keyed by full-text identity and card ability index. Record exact test function, exercised branch, and assertion separately from candidate references. Existing inventory remains useful for navigation, but neither its totals nor its depth labels can close the exhaustive-review requirement. No generated inventory files were edited.

## Anchor A — One-card debut draw (source trace, not runtime-validated)

Identity: `PL!HS-bp5-011-N`, 大沢瑠璃乃, cost 13; printed `{{toujyou.png|登場}}カードを1枚引く。` (`cards/cards.json:71961–71989`). Stored shared ability is at `cards/abilities.json:17722–17741`; also used by `PL!SP-sd2-009-SD2`. Its effect has action draw_card, count 1, source deck, destination hand, and **no explicit target**.

1. `parse_ability` (`parser.py:1162–1287`) routes to `parse_effect` (`1547–1713`). The actual phrase takes the exact `EffectPattern` rule (`10218–10225`, priority 0 registration `10327–10333`), not generic `parse_action`. Earlier priority -10 play-time-cost rule rejects this phrase. `_fill_defaults` (`6362–6644`, deck/hand at `6381–6383`) supplies movement defaults. Full post-normalization helper coverage remains incomplete.
2. Compiler (`compile_abilities.py:38–278`) classifies draw_card with family 2; encodes typed object tag 0x09, integer tag 0x03/count 1, and interned strings. Full compiler read includes subsequent compaction, but no specific baked byte slice/freshness verification was performed.
3. Decoder (`vm.rs:1388–1497`) ignores family byte, recognizes action through the enum wire table, copies count/source/destination into filter (`1454–1459`), and builds kind from action (`1464–1465`; `core/card.rs:1258` yields DrawCards). Missing target stays None; `target_name` defaults to self (`core/card.rs:2273–2275`).
4. Resolver dispatch (`ability/effects/mod.rs:24–439`) reaches draw wrapper (`ability/effects/draw.rs:101–155`) and `execute_draw` (`336–591`). For this uncomplicated case the deck/no-distinct branch (`522–532`) calls the draw helper (`16–81`); `MainDeck::draw` removes index 0 (`core/zones.rs:704–711`), and placement into hand (`ability/util.rs:2057–2170`, hand branch `2066–2069`) pushes through `Hand::add_card` (`core/zones.rs:795–797`).

**Merge opportunities, not proven safe deletions:** exact phrase rules overlap generic action rules (`parser.py:2064–2069`); both generic draw setters and `_fill_defaults` specify deck/hand. Compiler family classification duplicates Rust `EffectKind::from_action`, while the decoder ignores the family. Common fields are duplicated between AbilityEffect and filter and copied during decode.

**Unclosed links:** full normalization/card-overrides tree; numeric ability index and baked artifact freshness; trigger enqueue/store-to-resolver bridge; transitive filter/refresh calls and alternate targets. Therefore this is one substantial source trace, not one fully validated ability and not corpus coverage.

## Additional coverage from independent source passes

- Complete reads: `compile_abilities.py`, `generate_effect_decoder.py`, `generate_condition_decoder.py`, `vm.rs:1–1695`, `ability_store.rs:1–78`, generated effect decoder `1–560`.
- Rust model: complete relevant Ability/Cost/CompoundBranch/EffectFilter/EffectKind/AbilityEffect and Condition definitions, plus `from_action`, `kind_from_action`, and associated field helpers; not all `core/card.rs`.
- Generated condition decoder: accumulator/dispatch and build_compound `1–271`, final dispatcher `1522–1556`; other variant constructors unreviewed.
- Draw trace: complete `parse_ability`, `parse_effect`, `parse_action:2904–3191`, `_fill_defaults`, `_normalize_effect_tree:11497–11544`; not all transitive helpers. Complete `draw.rs:1–808`, `execute_effect`, `can_activate_effect:282–439`, `handle_both_targets:263–330`; not all transitive runtime calls.
- These are source-review ranges, not executed tests or measured semantic coverage. Only one of three planned anchors has been partially traced; no complete per-ability ledger yet.

## Verification notes

- `cargo test --test run_all zone_conversion_test` was attempted without modifying the test. It waited for a build-directory lock, reached compilation, then the tool timed out at 120 seconds. No test result was produced; this is **not a pass or a reproduced failure**. Full captured output: `%TEMP%\\kilo\\zone_conversion_verification.txt`.
- Subsequent Cargo retries confirmed build-lock contention. Direct execution of `C:\rust_targets\debug\build\rabuka_engine\08b7522da5f4fe86\out\run_all-08b7522da5f4fe86.exe zone_conversion_test --nocapture` finished within the 15-second command limit: 2 passed, 0 failed, 3357 filtered out, 0.00s, exit 0. Full output `%TEMP%\\kilo\\zone_direct_result.txt` was read. This validates that existing binary only, not unbuilt source changes or LiveTotal serde behavior.
- A temporary PE01 test was briefly inserted then removed before that command; no PE01 serialization reproduction was executed. PE01 remains source-proven only.
- No parser/engine implementation change is intended by this documentation task. Runtime suites, code lint/typecheck, artifact generation, and corpus-wide probes are not claimed complete.

## Next bounded investigation passes

1. Close Anchor A's trigger/enqueue and normalization-helper gaps before counting it complete; capture actual bytecode identity without regenerating the live tree.
2. Trace Chika gained-ability anaphora and a conditional/cost/choice ability with complete suspension, resumption, and expiry paths. The historical Chika note is not a completed second anchor.
3. Enumerate the current parser definitions and every unique ability into stable review rows. Record producer, normalized fields, compiler tags, decoder fields, runtime consumer, and semantic tests; distinguish unmapped from unused.
4. Cross-check PE01 helper values after normalization, PE04 aliases in the actual corpus, and PE05 nested/filter JSON fixtures. Inventory alone is not proof of coverage.
5. Review C consumption before recommending deletion of apparently redundant effect-family bytes. Keep a migration/compatibility column for every proposed shared schema change.

## Open Questions

1. Does the parser emit any vocabulary strings the engine `from_str` tables reject (or vice versa)?
2. Which parser fields have no engine consumer (dead JSON fields), and which engine fields have no parser producer?
3. Can the three serialized surfaces (JSON, wire bytecode, C `bytecode_data.c`) be generated from one schema, given the manifest last-writer-wins collision?

## Working Notes

- Concurrent WIP in this tree (bot v7, ability_effects, C resolver, docs, generated inventory) — read files fresh; do not stash/reset.
- Old failing WIP tests are out of scope per user instruction.
