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

### Dispatch order made honest (2026-09-28)

The four tables still exist, but the **order is now readable off one list per
registry** instead of being encoded in magic numbers nothing explained.

Effect priorities used to be `-10`, `0..4`, `100..147` and `10000..10005`, with
the `-10` and `10000+` rules registered *after* the tables were built — so
reading `_EFFECT_RULES` did not tell you the real order. They are now
`_EFFECT_DISPATCH_ORDER`, one list in the order they fire, registered
sequentially: registration index IS the priority. Same order, same output.

Conditions were `tier * 100 + index`, which silently breaks the tiering the
moment the table passes 100 rows. They are now `_CONDITION_DISPATCH_ORDER`,
extended by the one out-of-band rule and registered in a single pass.
`placed_discard_live_or_member` used to be registered at priority `1` *after*
the table was built, shadowing the whole tier-1 block it was written next to; it
is now visibly first in the list.

Rule names were the other half. Effects registered under
`getattr(rule, "__name__", f"effect_rule_{i}")`, but a rule is an
`EffectPattern` *wrapper* — the function that recognises the phrase is its
`handler`, and asking the wrapper left **30 of 61 rules named `effect_rule_N`**,
so the dead-rule test could not say which phrase had gone stale. `_rule_name`
now falls through handler → setter → action, and no registered rule is
identified only by its position.

`python parser.py --list-rules` prints the **effective** order for all layers.
Trust it over the order the tables are written in.

### Merging the four tables — NOT the right fix, don't attempt it

An earlier draft of this doc listed "collapse the four dispatch tables into one
registry" as the remaining parser untangle. On inspection that goal is
**ill-posed**, and attempting it would make things worse:

| Table | Produces | Entry point |
|---|---|---|
| `_ACTION_RULES` | the `action` dict (what the card DOES) | `parse_action` |
| `CONDITION_PATTERNS` | a `condition` dict (a sub-clause) | `parse_condition` |
| `_EFFECT_RULES` + `_STRUCTURAL_EFFECT_RULES` | the `effect` dict (effect text) | `parse_effect` |
| `_COST_HANDLERS` | the `cost` dict (payment) | `parse_cost` |

These are four different OUTPUT SHAPES parsed from four different parts of a
clause. A single dispatch loop would have to be told which shape it is
producing on every call, and the first-match-wins guarantee would have to hold
across tables whose outputs are not comparable — a condition rule that
"matched" would shadow an effect rule for reasons the reader cannot follow. The
tables stay separate.

What actually caused the pain — not the count, but the invisible *order* — has
been fixed. See "Dispatch order made honest" above: no magic priorities, no
out-of-band registrations, every rule named, and `--list-rules` printing the
effective order. The residual cost is one decision per new clause ("is this an
action, a condition, an effect, or a cost?"), which is inherent to clause
grammar and not something a merged table would remove.

### There is NO dead action-rule backlog — do not go looking for one (2026-09-28)

This section previously claimed 36 action rules were "shadowed" and needed
per-rule review. **That claim was wrong and the work it motivated was reverted.**
Corrected record:

A "does this rule ever WIN dispatch" check cannot be made sound for
`_ACTION_RULES`. The table is first-match-wins and is dispatched against many
derived clause forms — sequential steps, nested sub-actions, per-effect
re-parsing — so whichever text you drive it with, live rules go unreached.
Measured false-positive counts on the same 82 rules:

| driving input | "dead" rules reported |
|---|---|
| `parse_action` per corpus `triggerless_text` | 37 |
| full `parse_ability` per corpus text | 29 |
| raw ability strings from `cards.json` | 30 |

Three inputs, three different wrong answers. Acting on the first number meant
deleting 37 rules, which **removed `modify_limit`, `repeat_procedure` and
`draw_until_count` from the emitted corpus** (4 decoder warnings, 2 missing
mechanics, validation 0 → 2 issues, bytecode 93023 → 92509). The regenerate +
`git diff --numstat cards/abilities.json` gate caught it — no test did. The
rules were restored verbatim and the suite is green.

There is no dead-rule backlog here. The single provably dead action rule was
`action_047`, removed by static proof: its condition is a strict subset of
`action_039`'s and 039 sits earlier in the list. **Retire an action rule by
proving the shadowing statically, or not at all.** An "apparently dead" action
rule is almost always live on a clause form the probe did not produce.

`test_registry_coverage.py` carries this as a comment so the test is not
re-added.

### Still open (deliberately not done)

- **The 9 count entry points in `ability/condition/card.rs` still overlap
  structurally.** The duplicated *mechanics* were extracted (`is_moved_source`,
  `util::zone_card_ids_occupied`), but `get_count_for_condition` /
  `get_count_for_target` / `get_group_card_count` remain three parallel
  dispatchers over shared player resolution, multi-zone fan-out and zone
  dispatch. Merging them changes which dispatcher evaluates a given condition,
  so it needs characterization pins per condition shape.
- **`ConditionPattern`'s declarative fields are unused**: all 36 rows pass only
  `handler=`; the `match`/`match_any`/`exclude` half of `_TextRule` is dead
  weight on that class.
- **Engine:** `owning_effect` / `answering_effect` and the
  `pending_choice_owner` snapshot (2026-09-28) remove the guess at the three
  answer sites. Still open: the sites that read `current_effect` outright with
  no fallback, and `move_cards.rs:2900`'s hand-rolled `select_action` chain,
  which is a third resolution shape not yet folded in.

## Remaining debt (live)

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
> **Re-verified 2026-09-28 — this list is partly STALE.** Items 1 and 5 were
> already done before this session and should not be re-attempted. Read the
> verdicts, not the original claims.

1. **`_fill_defaults` re-extracts fields `parse_action` already set** —
   ~~open~~ **ALREADY FIXED (pre-2026-09-28).** `_fill_defaults` states the
   contract in its own docstring: `parse_action` owns ALL extraction, and
   `_fill_defaults` "ONLY fills defaults for fields left unset". Every
   re-extract there is guarded by `not in action` / `is none`; `source` and
   `destination` are additionally threaded in as `_cached_source` /
   `_cached_dest` so they are not recomputed. Pinned by
   `test_parse_action_scans_source_and_destination_once`. The docstring also
   forbids adding extraction here, so the confusion it caused is designed out.
2. **`_fill_defaults_move_cards` is source inference** — **still open.** ~309
   lines (the "~131" figure is stale). Takes `_cached_source`/`_cached_dest`,
   so it no longer recomputes; what remains is the inference itself.
3. **`_walk` + `_propagate_context` = two full tree walks** — **still open.**
   `_walk` (11 sub-walkers) during normalization; `_propagate_context`
   (~240 lines) after `_process_pre_fix`. Merging needs the timing dependency
   understood first — propagate consumes pre_fix output.
4. **`_process_pre_fix` is ~340 lines of compensating patches** — mostly
   **dissolved**; see the triage table above. What remains are genuine
   pipeline steps with a known ability-count blast radius.
5. **Double/triple extraction of the same fields** — ~~open~~ **ALREADY FIXED,
   same refactor as item 1.** `extract_source`/`extract_destination` are
   computed once per `parse_action` call and passed down.

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
