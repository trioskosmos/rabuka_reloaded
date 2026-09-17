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
| Parser entry/conditions | parser.py 1–5706 and parser_utils.py 1–986 | **Complete assigned-source read delivered**: 127 parser functions, 39 utility functions and 3 classes; PE18 summarizes contract findings; per-card semantics open |
| Parser effects | parser.py 5531–10550, complete crossing functions | In progress |
| Normalization/extraction | parser.py 10551–end, card_overrides.py, extract_card_abilities.py | **Complete owned-range read delivered** (PE15, PE16); transitive engine consumers partial |
| Condition/filter consumers | condition.rs, condition/*.rs, dynamic_count.rs and relevant filters | **Complete reads delivered** (PE13); parser-producer cross-check partial, no per-card reachability yet |
| Resolution lifecycle | resolver/cost/compound/choice, queue and trigger/expiry bridges | **Complete six-file source read delivered** (PE20); external resumption, affordability, expiry timing and per-card execution remain open |
| Effect execution | effects/*.rs, move_cards/look/util | **Dispatch + draw/score/state/look/ability/misc complete**; misc 827–4335, move_cards 1–1954/2497–3901, util 1–2435 unreviewed (PE14) |
| Corpus ledger | Every unique ability, identity mapping, recursive fields/vocabulary | **Mechanical audit delivered**: 936 rows, 2,011 mappings, 32,633 nodes; 0 semantically verified rows; PE19 records results and provenance |
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

### PE11 — Bytecode test names overstate the cases they actually assert

**Verified test coverage limits, not a new gameplay failure.** Full reads of `engine/tests/test_modules/characterization/bytecode_validation_test.rs:1–281` and `bytecode_deep_compare_test.rs:1–193`.

- `bytecode_nonempty_effects_match_action` checks an action only when the decoded effect already exists (`bytecode_validation_test.rs:151–169`). Losing the entire effect can bypass this particular assertion.
- `bytecode_cost_matches_json:176–207` compares cost presence rather than cost fields, and exempts mismatching choice_condition costs (`197–198`). It does not certify full cost fidelity.
- `malformed_bytecode_returns_error:263–268` tests only an out-of-range ability index, not malformed or truncated bytecode. `empty_slice_returns_default_ability:271–280` loads ordinary index 0 without constructing or proving an empty slice.
- `every_card_ability_index_is_valid:237–260` checks ranges of existing pairs, not complete/exact card-to-ability mapping, pair-array evenness, or printed ability index order.
- The deep comparison does assert corpus length and iterate every entry (`bytecode_deep_compare_test.rs:138–168`), which is stronger than sampling. However it compares serde projections (`149–151`), substitutes Null on serialization error, and modifies the JSON-side reconstruction (`json_path_decode:43–81`). Semantic fields skipped by serialization require separate assertions. Detailed diagnostics are limited to five abilities and twenty differences each (`153–156,174`), and write `deep_diffs.txt` on failure (`179–185`); no such test was run during this pass.

**Recommendation:** keep structural smoke checks but name their scope accurately; assert full effect/cost presence and fields, exact card/slot mapping, independent malformed/empty input cases, and semantic equality of execution-bearing fields. Deep comparison remains a useful test, not sufficient evidence that every printed ability behaves correctly.

### Additional completed test-support coverage

- `test_position_detection.py:1–186`: every method of TestDetectPositions, TestDetectIconPositions, TestSetCrossPositionFields and TestParseEffectActivationPosition read. `test_sub_action_activation_position:167–180` does not require any actions before iterating, so an empty actions list can pass vacuously.
- `gap_report.py:1–115`: complete strip_noise, count_fields, top_action, norm_text and main read. Field-density and near-text similarity are triage heuristics only; zero compares equal to False in the field-count exclusion, and nested metadata contributes to density. No correctness or completeness proof follows from its ranking.
- `test_inventory.py:1–1527`: all top-level functions and nested helpers read, including all renderers and main; PE10 records the representation-coverage implications. No generation or baseline updates performed.

### PE12 — Golden and smoke coverage do not prove the advertised behaviors

**Verified assertion weaknesses.** Complete files read:

- `engine/tests/test_modules/characterization/ability_golden_test.rs:1–20`: `abilities_golden_hash_matches` assigns a literal hash and compares it with the same literal (`14–19`); no hash is computed from the file. Only JSON readability and the top-level count of ten keys are actually checked. Same-shape semantic changes cannot fail this golden assertion.
- `corpus_smoke_test.rs:1–176`: `smoke_one_card:55–82` attempts one activation, accepts errors, chooses index 0 at most eight times, and processes pending autos without establishing every printed ability's trigger/gates. Its invariant (`17–53`) excludes energy deck, under-member cards, exclusion/resolution zones, and cross-player duplicate detection. It remains useful crash/structural smoke coverage, not exhaustive semantic execution.
- `action_coverage_test.rs:1–122`: `has_action:10–26` and action collection (`32–48`) inspect only top-level effects and one compound-actions level on member cards, excluding live-only and deeper branches. `all_action_types_fire_without_crash` picks one matching card, sends generic UseAbility without proving the selected action executed, and counts most errors as OK (`96–112`). Action coverage totals therefore cannot certify dispatch or outcome coverage.

**Recommendation:** compute the golden digest from actual normalized corpus content; assert per-ability trigger/gate/choice outcomes separately from smoke success; recursively enumerate every execution-bearing child and prove the requested action was reached. Do not remove smoke tests—their narrower role is valuable.

### PE13 — Condition evaluation has double negation and gate bypasses

**Source-proven; current-card reachability and gameplay impact not yet established.** Full reads of `condition.rs`, `condition/card.rs`, `condition/state.rs`, `condition/compound.rs`, and `dynamic_count.rs`, re-verified at observed HEAD `54de71db` (tree changed during review from the earlier `37b8325f`).

- **Comparison negated twice.** The general comparison path negates its result (`condition/card.rs:489–503`), then the outer gate negates again (`condition.rs:548–560`); special comparison paths negate only once. Opponent-choice negation is likewise handled internally (`condition/state.rs:1278–1287`) before the same outer gate.
- **State-transition counts bypass requested filters.** The wait-to-active fast path returns the global count before checking requested direction, ownership, cause, or card filters (`condition/state.rs:1142–1146`).
- **Aggregate-heart results bypass phase and negation gates.** Non-stage aggregate results return before those gates (`condition.rs:416–425`).
- **Compound containers skip their own gates.** Compound results return before outer gates (`condition.rs:434–440`), and children evaluate without the parent's phase/negation (`condition/compound.rs:65–125`).
- **Per-unit dynamic counts ignore criteria.** Unit counts read occupied own-stage slots without applying the per-unit filter or a divisor (`dynamic_count.rs:58–61`); constant-path consumers never multiply it.

**Refactor direction:** apply negation at exactly one layer, run all predicate gates before any early return, and bind unit criteria into the count source. Establish which current conditions express negation where the two layers disagree before changing semantics, since tests currently pin present behavior.

| Wire/model remainder | Complete card model/accessors, remaining generated condition constructors, C tag consumers | **Complete reads delivered** (PE17); corpus counts per dropped field and C migration paths open |

## PE13 — Condition evaluation applies negation twice and bypasses gates

**Source-proven at current HEAD; per-card reachability not yet established.** Independent full reads of `engine/src/ability/condition.rs:1–1039`, `condition/card.rs` comparison functions, `condition/state.rs`, `condition/compound.rs`, and `dynamic_count.rs`.

1. **Comparison negated twice.** `evaluate_comparison_condition` computes `final_result = !result` under `negation` and returns it (`condition/card.rs:489–503`). The dispatcher then re-derives `final_result` from the returned value under the same `negation` flag (`condition.rs:548–561`). A negated comparison therefore inverts twice and loses its intended meaning. The outer gate already exempts several shapes (`Location`/`Movement` with card_property, plain-location heart-all, revealed-empty), which shows negation ownership was patched per-shape rather than resolved once.
2. **Opponent-choice negated twice.** `evaluate_opponent_choice_condition` resolves `negation` internally and returns the final verdict (`condition/state.rs:1278–1287`); the outer gate inverts it again.
3. **State-transition fast path bypasses requested filters.** `evaluate_state_change_condition` returns the global wait→active count result before checking requested from/to direction, ownership, cause, or card filters (`condition/state.rs:1142–1146`).
4. **Aggregate-heart totals bypass outer gates.** Non-stage aggregate results return before phase and negation handling (`condition.rs:416–425`).
5. **Compound/or return before the outer gates.** Early return at `condition.rs:434–441` skips `check_phase_gate` and negation; children then evaluate without the parent's phase or negation (`condition/compound.rs:65–125`).
6. **Per-unit counts ignore their own criteria.** Unit counting reads occupied own-stage slots without the per-unit filter or divisor (`dynamic_count.rs:58–61`), and the constant path never multiplies.

**Why tests did not catch this:** existing corpus tests exercise negation through conditional_on_optional choice resolution (`opponent_choice_tests.rs:61,95,126`; `riko_test.rs:44,103`), where the internal handler's own negation produces the correct visible outcome; the extra outer inversion is masked where the outer gate's exemptions or the caller's usage absorb it. No dedicated corpus test pins a negated top-level comparison both ways.

**Fix direction (implementation blocked by documentation-only scope):** keep negation in exactly one layer — either the inner evaluators or the outer gate, not both — and delete the other application; move all predicate gates before any early return; fold per-unit criteria into the count source. Each change requires a failing corpus-anchored test first and a full regression run, because current tests may encode the double-negated behavior.

## PE14–PE16 — Aggregated partial-pass findings

Condensed from three independent source reports (`%TEMP%\kilo\comparison_execution.md`, `comparison_normalization.md`, `comparison_wire_models.md`). All are source-contract findings; none asserts a verified current-card failure without separate corpus/runtime evidence.

**Effect execution (PE14).** Complete reads: `effects/mod.rs:1–440`, `draw.rs:1–808`, `score.rs:1–856`, `ability_effects.rs:1–617`, `state.rs:1–1821`, `look.rs:1–1182`, `misc.rs:1–826`, `move_cards.rs:1955–2496`, `util.rs:2436–2657`. Unread: `misc.rs:827–4335`, `move_cards.rs:1–1954,2497–3901`, `util.rs:1–2435`. Key verified inconsistencies: both selection paths dispose the whole looked-at pool on zero matches (`look.rs:689–697`); optional revealed-source select expands to the whole pool while looked-at selection stays bounded (`look.rs:588–599,699–706`); gained-ability target binding falls back to text heuristics and activating card when candidates are empty (`ability_effects.rs:34–108`); gained structured score applies immediately without operation/trigger checks (`ability_effects.rs:356–425`); suppression handler only logs (`ability_effects.rs:290–311`); state self-targeting inferred without `self_target` (`state.rs:451–527`); multi-target heart application transforms only `selected_cards.first()` (`state.rs:1139–1180,1234–1244`); draw records requested rather than actual count (`draw.rs:16–81,580`); per-unit divisor/multiplier semantics differ across draw/score/look (`draw.rs:403–425`, `score.rs:93,244`, `look.rs:859–875`); several per-unit divisions lack zero guards (`state.rs:48,75,1607`, `misc.rs:654`); `live_total` `set` acts as additive (`score.rs:103–147`); card-floor handling skips decrements yet still records candidates (`score.rs:328–383`); cost expiry stores `abs(delta)` losing sign (`state.rs:1751,1806`); activation-cost effect-type names include the value while the revert allowlist omits it (`state.rs:1350–1363`, `util.rs:2466–2467`); unknown durations silently become ThisLive (`util.rs:2448–2455`); reveal-until-chosen ignores advertised cost constraints and hardcodes destinations (`misc.rs:164–258`); both-target dispatch contains an unreachable deck predicate and discards the self-side error (`misc.rs:263–330`). Refactor themes: explicit target bindings, separated selection vs disposition, typed count/expiry semantics, single look/select pipeline.

**Normalization/extraction (PE15).** Complete reads: `parser.py:10460–13885` (EOF) plus `parse_ability:1162–1287`, `card_overrides.py:1–147`, `extract_card_abilities.py:1–696`. Source-proven tooling defects: extractor main deletes `cards/build/abilities.bin` on success although generated GBA Rust `include_bytes!` requires it (`extract_card_abilities.py:580–590`; `compile_abilities.py:515–516`); local `import sys` shadows the module so missing-baseline/regression branches raise `UnboundLocalError` instead of exiting (`extractor:541,563,576`); nonzero compiler status not propagated, decoder failures warning-only, JSON written before validation (`extractor:583–605,519–529`); `_merge_parenthetical` can `del` a missing key (`parser.py:12776–12842`); `_clean` preserves numeric 0 while its docstring claims removal, and list/dict cleanup asymmetry can leave `{}` (`12724–12735`). `_walk`'s fixed-key traversal omits result/alternative/choice/activation/gained_effect children while other passes traverse everything — "walked" does not mean every descendant received the same policy (`11056–11085` vs `_enrich_characters:11615–11653`). Replacement transforms (`_split_look_three_way:11433–11483`, `_collapse_position_changes:11547–11589`) retain only selected fields, so parent gates (condition/optional/duration) can be dropped unless later passes restore them. Validation (`_validate_effect:12985–13044`, `_validate_semantic:13116–13839`) checks presence/shapes, not semantics, and runs before late mutations; group-filter validation matches names anywhere in the tree without polarity/owner checks (`extractor:641–692`). Card overrides mutate the whole shared full_text entry via substring conjunction (`card_overrides.py:31–38,131–147`). Mari (`PL!S-bp2-008-R＋`), Nozomi (`PL!-bp3-007-R`), and Burn (`PL!N-bp7-029-L`) were identity-verified with current stored representations and none established as currently failing.

**Wire/model remainder (PE17).** Full reads: `core/card.rs:1–4316`, all previously unread condition-decoder constructors, `build_lib.py`, `compile_cards.py`, both decoder generators, and bounded C consumer checks. PE04–PE07 re-confirmed unchanged. Key additions: `AbilityEffect.kind` is `#[serde(skip)]` (`card.rs:1336–1337`) and `EffectFilter` is reachable only inside kind, so the deep-compare test's `serde_json::to_value` projections (`bytecode_deep_compare_test.rs:149–151`) structurally cannot observe PE05 kind/filter losses; `effect_steps` is not skipped (`1347–1348`). `Ability::triggerless_text` (`card.rs:843–856`) strips by searching the trigger in the modified `rest` but slices the original full text, leaving the closing bracket when a leading trigger exists. `check_heart_requirement` (`card.rs:4156–4199`) counts Heart00 as a wildcard toward specific colors, contradicting its own HeartColor contract (`93–100`). Condition-decoder constructors coerce unknown values via permissive `from_str` (e.g. CardState→Wait at `2552–2557`, ComparisonTarget→Self at `2584–2589`, CardProperty→HasBladeHeart at `2615–2621`), unlike strict serde enums. Accessor/setter pairs disagree on defaults and skipped kinds (`target_any:2128–2137`, `set_optional:2260–2262` vs `optional_any:2007`; energy traversals `1383–1435` only walk `compound.actions`, not `effect_steps/options`). The C engine's `read_cond_value` (`vm.c:266–312`) preserves the effect-family byte inside condition variants and `condition.c` dispatches on it (`eval_complex:1409–1430`, `eval_condition_inner_host:1441–1511`), so removing the "redundant" effect-family byte in Rust alone would corrupt C condition dispatch — any schema change requires a versioned migration separating effect and condition tags. Manifest now reads raw 93,026 / compressed 28,656 with engine_commit `d1993354`; the held-over 93,023/28,653 figures were superseded. Detailed ledgers: `%TEMP%\kilo\comparison_wire_models.md`.

## PE18 — Parser entry/condition contract gaps

Independent assigned-source review delivered in `%TEMP%\kilo\comparison_parser_entry.md`: every line of `parser.py:1–5706` (127 complete functions) and `parser_utils.py:1–986` (39 functions and 3 classes), including globals and registries. The coordinator read the complete report; its source evidence is attributed to that reviewer, not a new coordinator source recheck. Full function ledgers and ENTRY01–ENTRY13 details remain in that report. These are local or producer/consumer contract findings, not demonstrated current-card gameplay failures.

- **Dispatch ownership:** action registration stores priority but `parse_action` uses append order (`parser.py:1966–1977,3161–3166`). Broad selection/required-heart rules shadow specialized action rules (`2416–2427,2460–2473,2686–2697`). Matchability tests cannot establish winning-rule coverage.
- **Condition shape and discriminators:** complex results are parsed with `parse_condition` but consumed as `AbilityEffect` (`parser.py:798–827`; `core/card.rs:3230–3235`; `condition/state.rs:1333–1383`). Bare energy conditions omit `energy_state`, needed to select the energy evaluator (`parser.py:4513–4523`; `condition.rs:510–516`). Universal-cost helper omissions may be repaired by later normalization; they are not unconditional final-output defects.
- **Phase and scope:** parser emits `set_phase`/`yell_phase`, absent from the shared phase gate (`parser.py:1001–1007`; `condition.rs:333–411`). Whole-text phase extraction and clause-independent owner/zone extraction can misbind nested phrases. General deck matching precedes energy deck, and multi-location extraction collects overlapping substrings (`parser_utils.py:527–536`; `parser.py:355–370`).
- **Information loss:** truthiness drops zero thresholds; appearance parsing retains limits without operators; blade prefix matching can turn an upper bound into a lower bound (`parser.py:950–955,1772–1776,3489–3507,4497–4499`). Discrete cost lists and unequal heart multiplicities are reduced (`parser_utils.py:649–658`; `parser.py:1925–1950`). Multiple trigger exclusions truncate on some paths (`1818–1826,5530–5539`).
- **Binding and structure:** specialized builders hardcode location, target or consequence fields (`parser.py:3270–3311,3373–3430,5084–5160,5221–5357`). Exact names and substring names share a representation (`5481–5489`; `core/card.rs:3661–3689`). Nesting-aware helpers coexist with plain splits; dynamic references and activation propagation depend on branch shape (`parser.py:659–827,1634–1667`; `parser_utils.py:122–191`).
- **Partial acceptance:** rule predicates swallow exceptions, setters can return partially populated effects, arity detection ignores required-argument count, and nested defaults are shallow-shared (`parser_utils.py:825–986`). Strict audit construction should reject incomplete nodes with rule and field-path diagnostics.

Refactor direction: one authoritative ordered registry with winning-rule provenance; typed predicate/result nodes and explicit negation ownership; clause-bound zones, targets and references; zero-preserving quantity/operator objects; shared nesting/tokenization and validated post-dispatch enrichment. Do not change semantics solely from these local findings without final-normalization and corpus/runtime checks.

## PE19 — Complete mechanical corpus ledger, semantic coverage still open

Independent corpus audit at HEAD `54de71dbd7b526bfdda55edafe13216073da4b79` reports stable inputs. The coordinator read the full 78-line `comparison_corpus_stdout.txt`, not all 7.77 MB of ledger rows. Artifacts are under `%TEMP%\kilo\`: `comparison_corpus.md`, `comparison_corpus.json`, `comparison_corpus_audit.py`, and `comparison_corpus_stdout.txt`; no repository corpus files were changed by that review.

- **Identity/mapping:** 936 distinct full-text ability rows; 2,011 distinct card/ability mappings across 1,565 cards. All 2,137 printed lines reconcile through 2,007 exact newline spans, one concatenated span, and three documented heart-icon normalizations (`extract_card_abilities.py:309–319`). The 126 continuation lines are included in spans, not missing abilities. No unresolved spans, missing card mappings, identity mismatches, duplicate mapping slots, duplicate full texts, or duplicate JSON keys were reported.
- **Recursive inventory:** 32,633 nodes, 1,784 action occurrences, 45 distinct actions and 47 distinct `type` values, with JSON-pointer censuses. All final actions occur in both compiler tables and runtime action vocabulary. No source/destination token falls outside Zone; two condition-location strings are outside that enum but belong to a raw-string domain.
- **PE01/PE04 qualification:** this snapshot establishes no final action-vocabulary mismatch and no source/destination vocabulary mismatch. PE04 remains an accepted-input contract discrepancy, not a current-corpus alias failure. This census does not prove Zone round-trip equivalence or field-specific consumer behavior.
- **Test claims:** all 936 full texts match inventory rows; the inventory claims 935 covered. These remain navigation claims subject to PE10–PE12, not independently verified assertions. Every semantic row remains `PENDING_SOURCE_REVIEW_INTEGRATION`; **semantically verified rows: 0**.

Input SHA-256: abilities `0d110e4b04fafeb45414ed20ca8993a1bfb798f305d02a7630609445e353e76b`; cards `926699085569ed6da74aeed10fbb68c6ec49bd84736697d2c72ff2354df23b95`. Output ledger SHA-256 `55df3afca0a9a8f46ed7e4302f52452940331c9f69044221e172c1f0a8131a13`; JSON ledger `9141e060987d69a71ce9fee81e94ffea01aaead69fb6bc45f88349bff67bd06a`. Remaining obligation: integrate producer/decoder/consumer and exact branch assertions into every stable row; mechanical completeness does not close the exhaustive semantic comparison.

## PE20 — Resolution lifecycle contract findings

Independent report `%TEMP%\kilo\comparison_lifecycle.md` (LC01–LC12) was read completely by the coordinator. Reviewer reports complete source/function coverage of `resolver.rs:1–1274`, `cost.rs:1–1486`, `compound.rs:1–972`, `choice.rs:1–3504`, `ability_queue.rs:1–756`, and `core/game_state/abilities.rs:1–2934`, at last observed HEAD `54de71db`. These are attributed source findings, not coordinator rechecks or demonstrated gameplay failures. Detailed function ledgers remain in the report.

| Finding | Source mechanism and evidence | Verification limit |
| --- | --- | --- |
| LC01: stale queue indices | `ability_queue.rs:385–390` compacts without remapping cursor/state/options; non-active ordering choices can reach compaction (`abilities.rs:1322–1555`). | External `turn/actions.rs` selection flow not completely reviewed. |
| LC02: condition erasure on resume | `choice.rs:94–100` removes later conditions by enum discriminant, not predicate equality; `compound.rs:487–505` strips later revealed-heart conditions without comparing colors/filters. | Affected-card census open. |
| LC03: unpaid optional consequence | Insufficient-energy shortcut schedules `conditional_action` without consulting negation (`compound.rs:854–871`), unlike the branch matrix (`24–35`). | Parser producer mapped at `parser.py:7672–7724`; stored-card execution not tested. |
| LC04: same-unit cost success without payment | No eligible optional pair returns success without setting `optional_cost_result=false` (`cost.rs:538–589`); resolver suppresses only explicit false (`resolver.rs:1007–1065`). | `PL!HS-PR-016-PR` identity and stored cost verified (`cards.json:7474–7515`, `abilities.json:23827–23878`); compiled execution untested. |
| LC05: same-group pair constraint missing | Prefilter unions eligible groups (`cost.rs:263–289`); re-prompt enforces same-unit, not same-group compatibility (`choice.rs:478–750`). | Source-contract finding only. |
| LC06: partial fixed costs accepted | State/reveal paths accept candidate counts below required count (`cost.rs:715–724,1044–1052,1265–1273`). | Full external activation gates and fixed-versus-up-to corpus cases remain open. |
| LC07: divergent non-atomic cost interpreters | Sequential preflight does not reserve resources; confirmation interpreter logs sub-cost failures and can continue (`cost.rs:760–815,1125–1408`). | No runtime rollback or affordability reproduction. |
| LC08: re-prompt metadata/cardinality loss | SelectionContext and reconstruction omit original fields (`choice.rs:28–46,1064–1086`); reveal reuses selected positions (`1465–1467`); discard compares cumulative selections with remaining count (`2105–2107`). | External indexing and actual UI symptoms unverified. |
| LC09: hardcoded continuation | Keep/shuffle cleanup directly draws three for both players and drains pending actions (`choice.rs:1185–1213`). | Matches inspected Dia print `PL!S-bp7-004-R`; generic alternate continuation unsupported by this path, not a proven Dia failure. |
| LC10: process-global guard lifetime | Static `PCA_CALLS` accumulates across games and clears queues above 200,000 (`abilities.rs:1557–1571`); optional guard keys omit game/entry identity (`choice.rs:3233–3265`). | Storage lifetime established statically; no long-run experiment. |
| LC11: grant-wide expiry | Expiring one gain entry clears all recipient gained maps (`abilities.rs:2716–2741`; `modifiers.rs:1529–1533`). | Mixed-duration corpus occurrence open; constants are refreshed afterward (`abilities.rs:2803–2804`). |
| LC12: relative versus absolute player ID | Reduction calls `distinct_stage_groups("self")` (`resolver.rs:1202`), whose helper treats anything except player2's actual ID as player1 (`abilities.rs:2400–2422`). | Player2 eligible-card execution not tested. |

Refactor directions: stable queue-entry identity; exact predicate evaluation state; one explicit optional-payment matrix and continuation owner; shared affordability/payment semantics; lossless re-prompt state; grant-specific expiry identity; typed relative versus absolute player references. These are recommendations, not authorization to implement. External resumption, action-generation gates, placement helpers, registration producers, frontend indexing and expiry invocation timing remain incompletely reviewed.

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

- After the lifecycle report arrived, the coordinator mistakenly attempted `cargo test --test run_all` outside the documentation-only scope. The command was terminated at 15 seconds during compilation; no test result was produced. Full output was read from `%TEMP%\kilo\comparison_suite_baseline.txt`. No intentional implementation/test edits followed; build-script side effects were not checked, so this attempt must not be described as a no-write operation or successful verification.

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
