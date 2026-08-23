# Parser Untangle Plan

**Target**: `cards/ability_extraction/parser.py` (~13.6K lines)
**Status**: plan — no code changed yet
**Approach**: in-place cleanup only. No new files, no relocations.

## Ground rules

1. **Byte-identical or skipped.** After every step, regenerate
   `abilities.json` with the old code (reference copy) and the changed
   code, then diff the bytes.
   - Identical → keep the step.
   - Any difference → revert the step and record it under "Deferred /
     skipped". No "close enough".
2. **`abilities.json` must remain valid at all times.** It is consumed by
   `cargo test --test run_all` (engine suite runs against baked bytecode).
   Never leave it deleted or half-written; restore original bytes after a
   verification run if needed.
3. Each phase ends with: engine suite green + Python unit tests under
   `tests/` green + `--check` baseline unchanged.

## Verification loop (per step)

```powershell
# 1. generate reference with CURRENT code
python cards\ability_extraction\extract_card_abilities.py
Copy-Item cards\abilities.json $env:TEMP\opencode\abilities_ref.json -Force

# 2. apply the step, regenerate
python cards\ability_extraction\extract_card_abilities.py

# 3. compare
fc.exe /b $env:TEMP\opencode\abilities_ref.json cards\abilities.json
```

Note: `generated_at` timestamp differs per run — compare everything except
that one line. Everything else must be byte-equal.

Final gate per phase:

```powershell
cd engine ; cargo test --test run_all
cd .. ; python cards\ability_extraction\tests\test_parse_action.py   # + other test files
python cards\ability_extraction\extract_card_abilities.py --check
```

## Tangles (ranked)

1. **Four competing dispatch systems** for effect text: `_EFFECT_RULES`
   rows, `_EFFECT_HANDLERS` cascade (priority 100+), `extra_checks`
   lambdas in `parse_effect` fallback (parser.py:1775), custom-refinement
   regexes in `_fill_defaults_count_and_refine` (parser.py:6249). No single
   owner for any phrase.
2. **Fields extracted 3–5× per text.** source/destination/card_type/count
   re-extracted in `parse_action`, `_fill_defaults_move_cards`, `_walk`,
   post-fixes. `FieldExtractor` (parser_utils.py:762) was built as the
   single-pass fix — **never used by parser.py**.
3. **Two overlapping tree walks.** `_walk` (parser.py:10668) and
   `_propagate_context` (parser.py:11309) propagate duration/target/
   card_type/group_names with different guards; heart_colors alone is
   set/stripped in ~6 places across the pipeline.
4. **Compensating patches instead of handler fixes.**
   `_process_pre_fix` FIX 2–15 + E0/E blocks patch downstream output.
5. **Dead / duplicated architecture.**
   - `segment_clauses` Stage-A IR (parser.py:802): built + tested, **never
     called** by `parse_ability`.
   - `FieldExtractor`: unused.
   - `_try_phase_gate` duplicates `extract_phase_gate`; phase-gate merge
     coded in both `parse_ability` and extract script's main.
   - `"登場させ"` action rule registered twice (2569 & 2816); unreachable
     code after `return` in `parse_condition` (1942); unused `categorized`
     assignment (3228); dead comment block in `_fill_defaults` region.
6. **God functions**: `parse_action` (~270 L), `_try_per_unit` (~370 L),
   `_fix_conditional_on_result` (~180 L).

## Phases

Each phase = independent change inside parser.py. Gate: byte-identical
`abilities.json` + all tests green. Non-conforming steps are skipped, not
forced.

### Phase 1 — delete dead weight

- Remove duplicate `登場させ` registration (keep one), unreachable
  `parse_condition` tail, unused local assignments.
- Decide `segment_clauses`: either wire it in as THE sentence splitter
  (replacing ad-hoc splits in `_try_implicit_sequential`) — only if still
  byte-identical — or delete it plus its test. Don't keep aspirational
  architecture.
- Make `_try_phase_gate` delegate to `extract_phase_gate`; collapse the
  duplicate phase-gate merge logic to one owner. Only if byte-identical.

### Phase 2 — unify dispatch surfaces

- Convert remaining tuple-format `_ACTION_RULES` entries to `ActionRule`
  (PARSER_NOTES Phase 8 debt; mechanical, removes the TypeError arity
  workaround).
- Fold `extra_checks` lambdas and `_fill_defaults` custom-refinement
  branches into registered rules where output is provably identical;
  skip the ones that aren't.

### Phase 3 — single-pass field extraction

Adopt `FieldExtractor` inside `parse_action`; make `_fill_defaults*`
consume cached values instead of re-running `extract_*`. Field ownership
docstring already states the intended contract (parser.py:6420).

### Phase 4 — merge the tree walks

Merge `_propagate_context` into the `_walk` schema (extend
`_CONTEXT_FIELDS`/`_BLOCKED_FOR_ACTION`). Highest-risk phase; attempt
only with the golden-file diff harness from the verification loop. Split
into field-by-field sub-steps, each gated independently.

### Phase 5 — dissolve compensating FIX blocks

For each FIX block in `_process_pre_fix`/`_process_post_fixes`, move the
correction into the handler that produced the wrong shape; delete the
patch. One FIX per step, each byte-gated. Blocks whose correction cannot
be localized stay as documented patches.

## Status (2026-08-23)

### Phase 1 — partially done
- ✅ Duplicate `登場させ` registration deleted (was 2570 & 2816; kept the first). Byte-identical.
- ✅ Unreachable comment tail after `parse_condition`'s return deleted. Byte-identical.
- ✅ Unused `categorized = categorize_quoted_text(quoted_exclusions)` deleted (parser.py ~3224). Byte-identical. NOTE: structured exclusion lists ("「X」以外") are still NOT captured anywhere — flagged as a potential future feature, not a refactor.
- ⏸ `segment_clauses` decision deferred to after Phase 2/3: pipeline-unused but has its own passing test suite (12 tests); its fate depends on whether single-pass extraction adopts it.
- ❌ `_try_phase_gate` → `extract_phase_gate` delegation SKIPPED, not byte-identical: three behavioral differences — (a) output "text" field is full text vs gate phrase only, (b) consequence-skip (次の/来) exists only in extract_phase_gate, (c) phase_target detection scope differs (full text startswith vs gate phrase). Unifying changes outputs for whichever side yields; needs an explicit decision on which behavior is correct before it can proceed.

### Phase 2/3/4/5 — not started
