# Master Refactor Plan — 2026-08-23 (rev 2)

Unified roadmap merging the original engine audit, `docs/CASTING_AUDIT.md` (~1,034 unchecked cast sites), and `cards/ability_extraction/PARSER_UNTANGLE_PLAN.md` (13.6K-line Python parser). Everything gets done eventually; order below is by risk-then-value. Every item gates on `cargo test --test run_all` (2541 baseline) and, for parser work, byte-identical `abilities.json`.

## ✅ DONE (this session)

| # | Fix | Where |
|---|-----|-------|
| 1 | Unknown keyword no longer silently becomes `Turn1` — logs + skips | vm.rs |
| 2 | Unknown action strings fail decode loudly (`""` legacy → Custom explicitly) — generator template + generated decoder in sync | effect_decoder_gen.rs + generate_effect_decoder.py |
| 3 | `parse().unwrap_or(0)` ×4 → warn + Err | choice.rs |
| 4 | `as u8`/`as i8` truncation in bytecode readers → try_from | vm.rs |
| 5 | Data-driven unwraps removed (entry-cost reveal, multi-location) | choice.rs, condition/card.rs |
| 6 | `unreachable!()` on decodable Compound → routed to compound evaluator | condition.rs |
| 7 | Fatal setup errors were invisible `debug!` logs → stderr | main.rs |
| 8 | Vestigial `constants_dirty` flag deleted (field + method + 30 call sites); recalculate_constants documented as unconditional | game_state/*, turn/*, ability/* |
| 9 | game_state `include!()` splices → real child modules with own imports | core/game_state/{tracking,modifiers,abilities}.rs |
| 10 | Post-movement TriggerEvent snapshot deduped into 2 helpers (11 copies, −110 lines) | abilities.rs, choice.rs, misc.rs |
| 11 | Distinct-count eligibility gate unified; `modified_cost()` helper replaces 3 formula copies | condition/card.rs |
| 12 | **A1** `max_distinct_names`: exact bitmask DP w/ domination pruning replaces exponential DFS + undercounting greedy; greedy only as >128-name safety net; brute-force cross-validation test (2000 cases) | util.rs + tests/test_modules/max_distinct_names_test.rs |
| 13 | **A2** web_server mutex poisoning recovered (LockRecoverExt) instead of unwrap-cascade; 52 sites converted | game/web_server.rs, main.rs |
| 14 | **A3** single shared no_std-safe `Lcg` in rng.rs — six identical binary-local copies deleted | rng.rs, src/bin/* |

---

## QUEUE A — Correctness / robustness leftovers

### A1. `max_distinct_names`: exponential DFS + undercounting greedy
`ability/util.rs:778-834`. ≤12-card branch clones a HashSet per DFS node (branching = names per card, unbounded). >12 branch is first-fit greedy which **undercounts** → wrong condition verdicts on big boards.
**Fix:** memoized bitmask DP over the name-set. Self-contained, pure function — easy to unit-test exhaustively against brute force.

### A2. web_server: ~50× `lock().unwrap()` poisoning cascade
`game/web_server.rs`. One panicking handler poisons every later request.
**Fix:** small helper that recovers from `PoisonError` (take inner guard), used everywhere.

### A3. RNG consolidation
Four families coexist: `rng.rs` xorshift32 with **constant desktop seed**, LCG copies in 5 bins (+ strategy_v2 variant), optional `rand`. Determinism claims are undermined by the constant seed.
**Fix:** one injectable engine RNG; bins take it via `bin_common`; delete copies.

### A4. Unsafe unsynchronized static cache in `no_std` vm path
`vm.rs:103-118` — plain `SyncUnsafeCell` + bool, no atomics.
**Fix:** atomic Bool + spin or critical-section crate.

### A5. qa_test_suite relocation + crate test gating
86KB regression corpus compiled into the lib while `[lib] test = false`; `run_qa_tests.rs` phantom bin; re-reads cards.json 28× via CWD-relative paths.
**Fix:** move behind `#[cfg(test)]` in tests/, shared lazy DB fixture, restore `[lib] test = true`, delete phantom bin.

## QUEUE B — Casting hygiene (from CASTING_AUDIT.md, ROI order)

### B1. Lints first (prevents regrowth)
`[lints]` / clippy warn: `cast_possible_truncation`, `cast_sign_loss`, `cast_precision_loss`. Expect a noisy first run — triage into fix/suppress.

### B2. `CardId(i16)` newtype
No ID type today; bare `i16` bounced to usize/u8 at every boundary (~250+ `as usize`). Precedent exists: `AbilityRef(u16)` in ability_store.rs.
**Fix:** newtype + `From<CardId> for usize`; compiler funnels conversions into auditable spots. Big diff — do zone-boundary-first, mechanically.

### B3. Widen modifier storage to i32
`GameModifiers` HashMap<i16, i16> forces `as i16` tolls and double conversions. Check GBA/3DS serialization before widening; confine narrowing to platform layer if needed.

### B4. Centralize the clamp idiom
`(x).max(0) as u8` scattered across condition/card.rs, live.rs, move_cards.rs → one `saturate_u8(i32)` helper (or kill u8 count fields entirely per B3).

### B5. Confine blob-decode casts
card_binary.rs/vm.rs raw-byte casts are correct but smeared → keep them only inside decoder module + round-trip tests. Floats/RNG casts: leave alone.

## QUEUE C — God-function surgery

### C1. `execute_gain_resource` split (1,162 lines)
`effects/misc.rs:740-1902`. Resource matched against JA/EN strings 17×. **Fix:** normalize once into `ResourceKind` enum at fn top; split into per-resource apply fns. Est. −500–700 lines, kills platform-dependent divergence bug class. Highest-value single refactor left in ability/.

### C2. describe.rs EN/JA parity
~400-line twin match towers (describe_effect_en/_ja). Either table-drive around shared fragments (−250–350) or add a compile-time/run-time parity test so they can't drift.

### C3. heart/blade greater_than_all merge + rule-log prefix dedup
Identical stage-scan scaffolding behind a stat-fn param; 3× rule-log prefix blocks in effects/mod.rs. Est. ~−90.

## QUEUE D — Module boundaries & tooling

### D1. vm decoder-gen modularization — RETRY, generator-first
Attempted this session, reverted (122 compile errors): moving the gen files orphaned their scope because they freeload on vm.rs imports via include!. **Lesson:** the generators must emit the import headers themselves. Steps: (a) update generate_effect_decoder.py / generate_condition_decoder.py to emit `use` header block + pub(super) entry points; (b) regenerate into place; (c) then flip include!→mod in vm.rs; (d) regen must produce byte-stable output vs old pipeline apart from the header.

### D2. bin_common migration completion
15 bins, 7 use bin_common. `struct Lcg` ×5 (+1 variant), fresh_database() ×5, deal/shuffle/setup ×8 (bot_arena/diag_stall verbatim reimplementations of bin_common::deal_game). Est. −600–800 lines. Fold A3 into this.

### D3. Bot strategy v2–v5 consolidation
Four live generations sharing scaffolding incl. 5 copies of `.expect("live set actions non-empty")`. Decide supported generation(s); extract shared action-selection; replace expect with fallback policy.

### D4. timer.rs / alloc_counter.rs hygiene
timer: `cfg!()` runtime branches paid when profiling off; ignored lock failures; println/eprintln stream mismatch. alloc_counter: env vars read twice; counting allocator overhead even when env-disabled.

## QUEUE E — Parser untangle (Python track; byte-gated per PARSER_UNTANGLE_PLAN.md)

Verification loop already specified there (regen ref → change → fc.exe /b compare minus generated_at; engine suite + python tests + --check).

- **E1. Phase 1 — dead weight**: duplicate 登場させ registration, unreachable parse_condition tail, unused locals; segment_clauses wire-or-delete; _try_phase_gate delegates to extract_phase_gate.
- **E2. Phase 2 — dispatch surfaces**: tuple `_ACTION_RULES` → ActionRule; fold extra_checks lambdas + _fill_defaults refinement branches into rules where provably identical.
- **E3. Phase 3 — single-pass field extraction**: adopt FieldExtractor in parse_action (built for this, never wired in); _fill_defaults* consumes cached values.
- **E4. Phase 4 — merge tree walks**: _propagate_context into _walk schema, field-by-field sub-steps. Highest risk — golden-file harness mandatory.
- **E5. Phase 5 — dissolve FIX blocks**: one compensating patch per step, moved into its producing handler, byte-gated; unlocalizable ones stay documented.

Non-conforming steps get skipped and logged in the plan's "Deferred" section, not forced.

---

## Execution order (rev 3 — impact-first)

**C1 → C3 → C2 → B1 → B4 → B3 → B2 → A4 → A5 → D1 → D4 → E1..E5 → D2 → D3**

Rationale: C1 (`execute_gain_resource`) is the highest-value item left — a 1,162-line god function with 17 duplicated JA/EN string comparisons; splitting it eliminates an entire divergence bug class. C-tier god-function surgery comes before B's type-system work because the split is easier while the function is still in one piece to read. A5/D2/D3 are bookkeeping and go last. The Python track (E) is independent and can interleave anytime.

### C1 progress notes
Full body read (misc.rs:740-1902). Split plan, each step test-gated:
1. `ResourceKind` enum derived once from the resource string; replaces 17 ad-hoc `"blade"|"ブレード"|"heart"|"ハート"` comparisons.
2. Extract target-resolution block (~330 lines: blade_targets/heart_targets/heart_color/final_count) into `resolve_gain_resource_targets()` returning a `GainTargets` struct; shared params bundled into a context struct.
3. Extract target_count pre-choice pass (~140 lines) into `try_create_target_selection_choice()`.
4. Extract blade application (~120 lines) → `apply_blade_resource()`.
5. Extract heart application (~140 lines) → `apply_heart_resource()`.
