# Parser Untangle Plan

**Target**: `cards/ability_extraction/parser.py` (~13.6K lines)
**Status**: in progress
**Approach**: in-place cleanup only. No new files, no relocations.

## Ground rules

1. **Tests green is the gate.** Byte-identical `abilities.json` was the
   original bar; it has been relaxed. A step is kept when the full engine
   suite (`cargo test --test run_all`) + Python unit tests + `--check`
   baseline all pass. Small output deltas are acceptable **if** they are
   understood and covered by tests.
2. **Less post-processing preferred.** When two implementations pass the
   same tests, prefer the one that removes post-hoc FIX blocks /
   compensating patches by fixing the producing handler instead.
3. Each phase ends with: engine suite green + Python unit tests under
   `tests/` green + `--check` baseline unchanged (or intentionally
   re-baselined with justification).

## Verification loop (per step)

```powershell
# 1. generate reference with CURRENT code
python cards\ability_extraction\extract_card_abilities.py
Copy-Item cards\abilities.json $env:TEMP\opencode\abilities_ref.json -Force

# 2. apply the step, regenerate
python cards\ability_extraction\extract_card_abilities.py

# 3. compare (ignoring generated_at) — delta must be explainable
```

Final gate:

```powershell
cd engine ; cargo test --test run_all
cd .. ; python cards\ability_extraction\tests\test_parse_action.py   # + other test files
python cards\ability_extraction\extract_card_abilities.py --check
```

## Tangles (ranked)

1. **Four competing dispatch systems** for effect text: `_EFFECT_RULES`
   rows, `_EFFECT_HANDLERS` cascade (priority 100+), `extra_checks`
   lambdas in `parse_effect` fallback, custom-refinement regexes in
   `_fill_defaults_count_and_refine`. No single owner for any phrase.
2. **Fields extracted 3–5× per text.** source/destination/card_type/count
   re-extracted in `parse_action`, `_fill_defaults_move_cards`, `_walk`,
   post-fixes.
3. **Two overlapping tree walks.** `_walk` (11 sub-walkers) during
   normalization and `_propagate_context` (~240 L) after pre-fix;
   heart_colors alone is set/stripped in ~6 places across the pipeline.
4. **Compensating patches instead of handler fixes.**
   `_process_pre_fix` FIX blocks patch downstream output; see triage
   table in PARSER_NOTES.md — most are load-bearing, so dissolution =
   fixing the producing handler first.
5. **God functions**: `parse_action` (~270 L), `_try_per_unit` (~370 L),
   `_fix_conditional_on_result` (~180 L).

## Phases

### Phase 1 — dead weight ✅ DONE (2026-08)

- Deleted unused `FieldExtractor`, orphaned `_DEBUG_LOG` plumbing,
  `segment_clauses` Stage-A IR (+ its test), compile_abilities vocab
  block (~470 L, byte-identical verified) — commit 213590a4 / e98c1ef8.
- Duplicate registrations removed (untangle Phase 1 commit 39daba51).
- Extract script now calls the real `parse_ability()` — single parsing
  owner (2121cacd).
- `_try_phase_gate` delegates to `extract_phase_gate` (2d159cd0).
- Generic 「プレイに際し…コストはNになる」 handler replaced the
  LL-bp7-001-specific override (6a414e0a).

### Phase 5 (moved up) — dissolve load-bearing FIX blocks into producers

Triage table lives in PARSER_NOTES.md. Order (smallest blast radius first):

| Step | FIX block | Producing handler to fix | Status |
|---|---|---|---|
| 5.1 | FIX 2 (each_time → conditional_on_optional) | `_try_each_time` via `_finish_each_time` | **done** — byte-identical |
| 5.2 | FIX 7/7b (ability_filter backfill) | `_handle_cost_modification` + `parse_action` select path via new `_apply_no_ability_filter` | **done** — byte-identical |
| 5.3 | FIX 9/9b (result_condition property, followup self_target/self_cost) | `_try_kore_niyori_result` | **done** — byte-identical |
| 5.4 | FIX 3 optional-strip (+ dead positive/negative renames) | `_strip_coo_child_optional` called in `parse_effect` after `_propagate_optional` | **done** — 1-ability delta, understood & desirable: strips the stale nested `optional:True` on shared optional_action/conditional_action children (the double-prompt ambiguity FIX 3 targeted); all suites green |
| 5.5 | FIX 10–15, N | characterized by removal-diff: FIX 11 & N dead → deleted; FIX 15 dissolved into `parse_action` (Rule 11.10.1 exclude_self now uniform — 4 nested nodes gained it, semantically correct); FIX 10/12/13a/13b remain as documented pipeline steps (1/1/1/2 abilities each, need triggers-context or shape-specific repair) | **done** |

Net effect of 5.1–5.5: every verified-load-bearing compensation block is
gone from `_process_pre_fix`. What remains there is genuine pipeline work:
condition re-parse, action inference, sequential-chain fixes, and the four
characterized shape repairs (FIX 10/12/13a/13b).

### Phase 4 — one propagation system ✅ rescoped & executed (2026-08)

`_propagate_context` and `_walk` were two parallel implementations of the
same concern (context propagation) whose guards had drifted apart. Each
block of `_propagate_context` was disabled individually and the corpus
regenerated (removal-diff triage). Result:

| Block | Δ without it | Action |
|---|---|---|
| pcA condition location/target/card_type inherit | 0 | **deleted** — subsumed by `_walk` |
| pcB action duration/target inherit | 0 | **deleted** — subsumed |
| pcC draw_card duration (+ statically-dead move_cards check) | 0 | **deleted** |
| pcD timing_condition inherit | 0 | **deleted** |
| pcI heart_type:all enrich | 0 | **deleted** |
| pcJ card_count property enrich on `node["condition"]` | 0 | **deleted** |
| pcK Q148 blade-aggregate heart_colors strip | 0 | **deleted** |
| pcE compound sub-condition location (11 abilities) | live | kept |
| pcF sequential duration restore (4) | live | kept |
| pcH conditional_on_result card_type (4) | live | kept |
| pcL node-is-condition enrich (1) | live | kept |

~5.2K chars of dead/subsumed propagation logic removed, byte-identical.
What remains in `_propagate_context` is stage-dependent work that needs
pre-fix/restructured trees and therefore genuinely cannot merge into
`_walk`.

### Phase 2 — dispatch surfaces ✅ rescoped & executed (2026-08)

The post-fallback `extra_checks` were already documented as
position-sensitive ("do not fold into the registry"). Treated them as a
triage surface instead: each check disabled individually + regen.

| Check | Δ without it | Action |
|---|---|---|
| ec1 blade_set_m → set_blade_count | 1 | kept |
| ec2 何もしない | 0 | **deleted** (dead) |
| ec3 元々のブレード same-thing | 0 | **deleted** — subsumed by generalized set_blade_count handler |
| ec4 カードを1枚引いてもよい | 0 | **deleted** — subsumed by registered ActionRule |
| ec5 per-unit hand cost | 0 | **deleted** — subsumed by `_handle_cost_modification` |
| ec6 character_effects heart+blade | 0 | **deleted** |

### Phase 3 — single-pass field extraction ✅ executed via memoization (2026-08)

Instead of threading caches through signatures, all pure scalar extractors
(`extract_count/source/destination/target/card_type/operator/cost_limit/
cost_limit_with_operator/picker`, `detect_require_all_hearts`,
`check_original_value`) are `lru_cache`-wrapped at the name level — every
call site anywhere in the pipeline now computes each field once per unique
text. List/dict-returning extractors stay unmemoized so callers can mutate
results. Byte-identical; output unchanged.

Decision on remaining FIX blocks (10/12/13a/13b): left in place. Each is
now characterized with a known ability-count blast radius (1/1/1/2), each
patches a shape produced by multiple handlers or needs corpus-level context
(triggers), so producer-side dissolution would trade one clear documented
step for several scattered copies. Revisit only if their blast radius grows.

## Deferred / explicitly skipped (updated as phases run)

_(none — all phases landed or were consciously rescoped with rationale above)_

## Appendix: full-layer removal-diff characterization (2026-08)

Same per-block disable + regen methodology applied to every remaining
layer. Result: **no dead code left anywhere in the pipeline.**

| Layer | Blocks tested | Verdict |
|---|---|---|
| `_process_pre_fix` FIX blocks | 14 | FIX 6/11/N dead (deleted); FIX 2/3/7/7b/9/9b dissolved into producers; FIX 10/12/13a/13b live (1/1/1/2 abilities), kept |
| `_propagate_context` | 11 | pcA/B/C/D/I/J/K subsumed by `_walk` (deleted); pcE (11)/pcF (4)/pcH (4)/pcL (1) stage-dependent, kept |
| `_walk` sub-walkers | 10 | **all load-bearing** — 6 to 166 abilities each |
| `_normalize_effect_tree` stages | 12 | **all load-bearing** — 1 to 47 abilities each |
| `parse_effect` extra_checks | 6 | ec2 dead, ec3–ec6 subsumed (deleted); ec1 live (1 ability) |

The `_walk`/`_normalize` results confirm the earlier dedup sessions did
their job: what remains is a minimal, fully-characterized pipeline where
every pass earns its place and every patch has a measured blast radius.
