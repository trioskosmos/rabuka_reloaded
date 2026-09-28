# Parser Notes — canonical debt, history, and status

**File**: `cards/ability_extraction/parser.py` (~14.0K lines)
This is the single maintained parser maintenance document. It consolidates
the former `PARSER_REFACTOR.md`, `PARSER_DEEP_REFACTOR.md`,
`PARSER_DEBT.md`, and `PARSER_UNTANGLE_PLAN.md` (all completed or rescoped;
history remains in Git).

## Session: untangling pass (2026-09-28)

Every change below was proved output-neutral by the usual method: regenerate,
then `git diff --numstat cards/abilities.json` must show only the
`generated_at` and `engine_commit` lines. Parser suites
(`test_registry_coverage` 4, `test_parse_action` 46, `test_parser_coverage` 44)
and the engine suite (3729) all pass.

### Removed

- `action_047_gain_resource` — unreachable. `action_039`'s condition is a strict
  superset and sits earlier in the first-match-wins list; 047 also had no
  `setter`, so it would have emitted a worse shape than 039 had it ever fired.
  It survived because the dead-rule test only covers the effect and condition
  registries, not `_ACTION_RULES`.
- `ActionRule.priority` / `.order` — decoration. Dispatch (`parse_action`) walks
  `_ACTION_RULES` positionally and breaks on first match; the only read of
  `priority` was the `--list-rules` dump. Removing them from all 83 rows is the
  point: a hand-written `priority=39` implies a ranking the table does not have.
  **Rule order in the list IS the priority.** The `--list-rules` header now says so.
- `build_lib.BinaryEncoder` (~78 lines), `generate_rust_include_bytes`,
  `load_json`, `save_json` — zero references. `BinaryEncoder` was a full second
  copy of the `BC` encoder inlined in `compile_abilities.py`.
- The `git_hash` block in `compile_abilities.py.main` — computed, then never
  passed anywhere; `write_generation_manifest` resolves `engine_commit` itself.
- The headerless `abilities.bin` write in `compile_abilities.py.main` — a 93KB
  write immediately overwritten by the headered one 6 lines later.
- `test_parsing()` and its unconditional call in `main()` — debug scaffolding
  printing a `=== Test Parsing ===` block on every production run.
- `DECOMPRESSED_LEN` from the generated `abilities_gen.rs` — emitted, read by
  nothing in `engine/src` (only the separate `engine_c_wip` C tree reads its own
  copy), and wrong by 8: it said 93023 while the real decompressed size is 93031,
  because the compressed blob carries the 8-byte `RBKA` header.

### Fixed

- `extract_card_abilities.py` deleted `build/abilities.bin` immediately after
  regenerating it, and `abilities_gen.rs` `include_bytes!`s that path under the
  `gba` feature — so the documented entry point broke the GBA build. The
  compiler's exit code is also no longer swallowed, and decoder failure is now
  fatal rather than a printed warning.

### Deduplicated

- Cost-change verbs `(減る|減らす|増える|増やす)` were spelled in three places with
  a **real** divergence: the `action_014` dispatch guard accepts `コスト[はが]`,
  `_COST_DELTA_RE` accepts `コスト[はがを]`, and `COST_MODIFICATION_PATTERNS`
  only `コストは`. Factored to `COST_CHANGE_VERBS` with the divergence documented
  as deliberate — the guard is narrower on purpose so it cannot claim a form the
  extractor would then mis-type. Do not "fix" one to match another.
- The trigger-gate separator list was duplicated verbatim in `card_overrides.py`.
  Now `parser.TRIGGER_GATE_SEPARATORS`, imported by both.
- `SPLIT_LIMIT = 1` was defined in both `parser.py` and `cost_parser.py`. Now in
  `parser_utils`, the module that already owns shared parser vocabulary.

### NOT dead — do not "clean these up"

An earlier review flagged `cards/build/abilities_gen.rs`,
`cards/build/bytecode_data.c` and `cards/build/cards_gen.rs` as unreferenced
duplicates of the engine copies. That is wrong:

- all three are **gitignored** (`.gitignore:112,113,126`), so they cannot drift
  in the repo in the first place;
- `cards/build/abilities_gen.rs` is consumed by the C port
  (`engine_c_wip/Makefile:49` → `tools/gen_from_rs.py`);
- `cards/build/bytecode_data.c` is consumed by
  `engine_c_wip/tools/gen_bytecode.py:11`.

`platforms/snes/bytecode_data.c` is the third, separately-generated copy that
`platforms/snes/build.rs:26` compiles.

## Remaining debt (live)

### Four dispatch tables, still not one (partially done)

The 2026-09-28 pass removed the phantom `priority`/`order` fields, so list order
now honestly *is* the priority. What remains is the table count. A new clause
still has to pick one of five places:

| Clause kind | Where | Mechanism |
|---|---|---|
| action | `_ACTION_RULES` | `ActionRule` row + optional `_set_action_NNN` |
| cost | `cost_parser.py` | `@_register_cost` handler |
| effect | `_EFFECT_RULES` | `EffectPattern` row |
| effect (structural) | `_STRUCTURAL_EFFECT_RULES` | `EffectPattern(handler=...)` |
| condition | `CONDITION_PATTERNS` | `ConditionPattern(name, tier, handler)` |

The awkward parts, in rough priority order for whoever picks this up next:

1. **Magic priorities.** Effects register at `-10`, `0..4`, `100..147` and
   `10000..10005`; conditions at `tier * 100 + index`. The `10000` block and the
   `play_time_cost_set` rule at `-10` are registered *after* the tables are
   built, so reading the table does not tell you the real order. `parser.py --list-rules`
   prints the effective order for all five layers — use that, not the source order.
2. **One out-of-band condition registration.** `placed_discard_live_or_member` is
   registered at priority `1` after the table (`parser.py:4664`), which shadows
   the whole tier-1 block it is written next to.
3. **`ConditionPattern`'s declarative fields are unused.** All 35 rows pass only
   `handler=`; the class contributes a name and a tier and nothing else.
4. **Rule names are synthetic for effects.** Effects register under
   `getattr(_h, "__name__", f"effect_rule_{_ri}")` and `EffectPattern` has no
   `__name__`, so every effect rule is named `effect_rule_N` and the dead-rule
   test cannot say which phrase died.
5. **`_ACTION_RULES` is not covered by the dead-rule test.** `test_registry_coverage`
   checks the effect and condition registries only — which is how `action_047`
   stayed hidden.

Consolidating these is a real change to first-match-wins order, so it needs the
byte-diff gate run per table, not once at the end.

### Engine-side key audit (2026-08-24)
Cross-referenced every JSON key emitted into `abilities.json` against
`card.rs` structs + engine handlers. Findings:

| Key (count) | Disposition |
|---|---|
| `check_self` (6 conditions) | **IMPLEMENTED in engine** — new `ConditionCommon.check_self`, decoded + evaluated in `evaluate_check_self_condition`; presence of the ACTIVATING card in the location instead of counting matching cards. Pinned by `check_self_condition_test.rs` (2 tests) |
| `zone`/`energy`/`costs`/`max_repeats` | handled by vm.rs aliases / decoder (audit false positives) |
| `source_location` (2 gain_ability_from_source) | engine already hardcodes under-member sourcing; documentary |
| `action_reference` (1) | decodes as AlwaysTrue alias; acceptable degradation |
| `action_reference` (1) | decodes as AlwaysTrue alias; acceptable degradation |
| `choice_modifier` (2 choices) | **removed from emission** — structured choice_condition/alternative_condition/alternative_count_type were always present and are what the engine's tiered-choice evaluation reads (compound.rs) |
| `target_event` (1) | **removed from emission** — engine keys replacement effects off destination=success_live_zone (pinned by replacement_destination validation rule) |
| `per_character` (1, LL-bp7-001 play cost) | known limitation: engine selects count=len(characters) restricted to those names but cannot enforce exactly-one-per-name; single card, low impact |
| `baton_touch`(on appearance), `energy_state`, `comparison_source`, `area_direction`, `positions_characters`, `turn_number`, `cost_reference_*` | decoded or harmless documentary |

### Phase 8 — Standardize `_ACTION_RULES` format — DONE (stale entry)
All registrations already go through the ActionRule-normalizing
`_register_action` (`__post_init__` arity normalization, E2a session); the
dispatch loop has no TypeError workaround. Nothing to do.

### `_process_pre_fix` FIX-block triage (2026-08-24 session)
Empirically verified by removal + regen byte-diff, one block at a time:

| Block | Verdict | Evidence |
|---|---|---|
| FIX 6 (opponent_action flatten) | **DEAD — removed** | 0 output change; no producer emits the wrapper |
| FIX 2 (each_time → conditional_on_optional) | **DISSOLVED into producer** | `_try_each_time`/`_finish_each_time` reshapes at parse time; byte-identical |
| FIX 3 (conditional_on_optional cleanup) | **DISSOLVED into parse_effect** | `_strip_coo_child_optional` after `_propagate_optional`; 1-ability delta (stale nested optional on shared children now stripped uniformly — the intended semantics). Dead positive/negative renames dropped |
| FIX 7/7b (ability_filter backfill) | **DISSOLVED into producers** | `_apply_no_ability_filter` in `_handle_cost_modification` + `parse_action` select path; byte-identical |
| FIX 9 (result_condition card_property) | **DISSOLVED into producer** | enrichment moved into `_try_kore_niyori_result`; byte-identical |
| FIX 9b (followup self_target/self_cost) | **DISSOLVED into producer** | moved into `_try_kore_niyori_result` followup construction; byte-identical |
| FIX 10–15, N | characterized 2026-08 by per-block removal-diff | FIX 11 & FIX N: **DEAD — deleted** (0 abilities each). FIX 15: **DISSOLVED into parse_action** (Rule 11.10.1 exclude_self now uniform; 4 nested nodes gained it, semantically correct). FIX 10/12/13a/13b: live pipeline steps (1/1/1/2 abilities), kept with characterization |

Conclusion: every verified-load-bearing compensation block has been
dissolved into its producer or removed as dead. Remaining FIX blocks are
genuine pipeline steps, each with a known ability-count blast radius.

## Fundamental structural issues (from the deep-refactor review)
1. **`_fill_defaults` re-extracts fields `parse_action` already set**
   Re-extracts source, destination, cost_limit, optional, max, position,
   group_names, heart_colors. Reader can't tell which function sets which field.
   *Fix*: move all extraction into `parse_action`; `_fill_defaults` only sets
   action-type-specific defaults (draw→deck/hand, shuffle→move_cards).
2. **`_fill_defaults_move_cards` is ~131 lines of source inference**
   Complex source→destination inference that belongs in the dispatch table or
   `parse_action` itself.
3. **`_walk` + `_propagate_context` = two full tree walks**
   `_walk` (11 sub-walkers) runs during normalization; `_propagate_context`
   (~240 lines) runs after `_process_pre_fix`. Overlapping work; merging requires
   understanding the timing dependency (propagate needs pre_fix output).
4. **`_process_pre_fix` is ~340 lines of compensating patches**
   See triage table above: most are load-bearing; dissolution = producer fixes.
5. **Double/triple extraction of the same fields**
   `extract_source`, `extract_destination`, `extract_card_type`, etc. are called
   3–4 times on the same text across `parse_action`, `_fill_defaults`, `_walk`.

## Completed work (history)

### Session: extract script single-owner parsing (2026-08-24)
- extract_card_abilities.py now calls parser's real `parse_ability()`;
  deleted the divergent weaker inline copy
- Added `normalize_multiline()` (old normalize collapsed `\n` choice bullets)
- Hardened condition back-fill: effect text only, only when no condition
  exists, leading-gate only (was re-scanning cost text and double-gating)
- Generic 「プレイに際し…コストはNになる」 handler (`_try_play_time_cost_set`)
  replaced the LL-bp7-001-specific override
- Deleted dead code: FieldExtractor, _DEBUG_LOG plumbing, segment_clauses
  Stage-A IR (+tests), compile_abilities vocab/encode block, typo'd patterns

### Session: dedup + helper extraction (PARSER_DEBT)
- Extracted shared helpers: `detect_exclude_self()`, `extract_heart_colors_from_text()`,
  `detect_duration_code()`, `extract_cost_operator()`; replaced all inline copies
- Unified dual-path condition enrichment → `_enrich_condition_common()`
- Broke up god-functions into named helpers:
  - `_fill_defaults` 498 → ~260 lines (`_fill_defaults_move_cards`, `_fill_defaults_count_and_refine`)
  - `_extract_generic_fields` 366 → ~250 (`_extract_comparison_fields`, `_extract_resource_fields`)
  - `_process_pre_fix` 476 → ~340 (`_fix_sequential_chain`, `_fix_condition_enrichment`)
  - `_process_post_fixes` ~338 → ~175 (`_fix_conditional_on_result`)
- Removed dead code (duplicate re_yell registration, dead `dur_effect` check)

### Session: overlap cleanup (PARSER_REFACTOR)
- Removed `parser_utils`/`parser.py` overlaps (dead cost pattern, local POSITION_KEYWORDS)
- Deduplicated `_walk` sub-walkers (removed superseded propagation paths)

### Session: validation merge + deep refactor (PARSER_DEEP_REFACTOR)
- Deleted `_validate_output()` from extract_card_abilities.py (~240 lines);
  merged into single `_validate_semantic()` with recursive tree walking
- `_try_per_unit` 350 → 25 lines; `parse_action` reduced by 130 lines

## Untangle plan status

The former `PARSER_UNTANGLE_PLAN.md` is consolidated here. All planned phases
were executed or explicitly rescoped with byte-diff and test evidence. The
remaining live debt is the structural work listed above; the old phase-by-phase
plan is retained in Git history rather than maintained as a second status file.
