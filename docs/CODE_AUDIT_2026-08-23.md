# Engine Code Audit — 2026-08-23

Deep-dive audit of `engine/src` (core/, ability/, root modules + bins). Every headline finding was verified firsthand against source. Priorities: **P0** = silently corrupts gameplay, fix now. **P1** = panic/robustness risk reachable from data or users. **P2** = architecture debt. **P3** = dedup/merge opportunities.

---

## P0 — Silent fallbacks that corrupt gameplay

These all share one failure mode: bad input decodes "successfully" into a *wrong* value instead of erroring, so regressions show up as wrong game outcomes, never as errors.

### 1. Unknown keyword → `Keyword::Turn1`
`ability/vm.rs:1549`
```rust
_ => crate::card::Keyword::Turn1,
```
Any new/misspelled keyword string in the card DB silently becomes `Turn1`, and `resolver.rs:378` then makes the card dead after turn 1 (`if gs.turn_number != 1 { return false }`). A typo in the extractor = a card quietly stops working, no warning anywhere.
**Fix:** return `Option<Keyword>` and skip+log on unknown; or make the generator total over the enum so a new keyword fails the build.

### 2. Unknown action string → `ActionType::Custom` → silent no-op
`ability/effect_decoder_gen.rs:38`, `ability/enums.rs` (`Default for ActionType = Custom`), `ability/effects/mod.rs:128`
```rust
if effect.action == ActionType::Custom && effect.action_by().is_some() { return Ok(()); }
```
A card action type the engine doesn't know yet decodes fine and executes as either a no-op or falls into `execute_custom`. The whole decode path is honest `Option`-based failure everywhere *except* the single field that matters most.
**Fix:** fail the decode (or log-error + skip the whole ability) on unknown action strings.

### 3. User input parsed with `.unwrap_or(0)`
`ability/choice.rs:2407, 2595, 3377, 3395`
```rust
let idx: usize = selected.parse().unwrap_or(0);
```
A garbled client reply silently selects index 0 (left stage slot) or count 0 instead of re-prompting. Can move the wrong card.
**Fix:** propagate `Err("invalid selection")`; the choice handlers already return `Result`.

### 4. Silent integer truncation in the bytecode reader
`ability/vm.rs:315-322` — `read_u32_value()` returns `Option<u8>` via `as u8`; `read_i8_value` uses `as i8`. A value >255 in the JSON source wraps mod-256 with zero diagnostics — counts/costs wrap silently.
**Fix:** `u8::try_from(v)` and return `None` (already propagates as decode failure).

### 5. Data-driven panics: `.unwrap()` on decoded/card-data fields
- `condition/card.rs:1708` — `condition.get_locations().unwrap()`
- `choice.rs:1671` — `gs.entry_cost().cloned().unwrap()`
- `move_cards.rs:2086` (`deck_pos.unwrap()`), `:2759` (`sum_limit.unwrap()`)
- `effects/misc.rs:197` — `effect.cost_limit_any().unwrap()`

All reachable from hand-authored card data / queue state, not programmer error.
**Fix:** `unwrap_or_default` + `log::error!`, or return `Err(String)`.

### 6. `unreachable!()` on a variant the decoder can produce
`ability/condition.rs:533`: `Condition::Compound { .. } => unreachable!()`. Currently guarded by an early dispatch at `condition.rs:434-441`, but `condition_decoder_gen.rs:191,1357` can emit top-level `Compound`. Any refactor of that pre-dispatch turns this into a card-data panic.
**Fix:** route to `evaluate_compound_condition` or log-error + defined result.

### 7. `max_distinct_names`: exponential DFS + undercounting greedy fallback
`ability/util.rs:778-834`. The ≤12-card branch clones a `HashSet<String>` per DFS node (branching factor = names per card, unbounded — comment says ≤3¹² but that's optimistic). The >12 branch is first-fit greedy which provably **undercounts**, i.e. can return wrong verdicts for large boards.
**Fix:** memoized bitmask DP over name-set; keep greedy only as last resort with a debug warn.

---

## P1 — Robustness / panic paths

### 8. Fatal setup errors logged at `debug!` level
`main.rs:98, 110, 133, 148, 312, 315`:
```rust
log::debug!("Failed to load cards: {}", e);
```
With default logging the binary exits **silently** when cards/decks fail to load. Wrong level for user-facing failures.
**Fix:** these are fatal → `error!` + non-zero exit.

### 9. ~50× `lock().unwrap()` in web_server.rs
Any panic in one request handler poisons the mutex and cascades panics through every later handler.
**Fix:** handle `PoisonError` (recover the inner guard) at minimum; ideally narrow lock scope.

### 10. Unsynchronized unsafe static cache in `no_std` builds
`ability/vm.rs:103-118`:
```rust
static CACHE: SyncUnsafeCell<Option<Vec<u8>>> = SyncUnsafeCell(UnsafeCell::new(None));
static INIT: SyncUnsafeCell<bool> = SyncUnsafeCell(UnsafeCell::new(false));
```
No atomics/fences — a data race on any threaded `no_std` target. Documented single-threaded targets make this survivable today, but nothing enforces that.
**Fix:** `critical-section` crate or `AtomicBool` + spin.

### 11. RNG fragmentation undermines determinism claims
`rng.rs:13` documents desktop seeding as a **constant seed** ("deterministic between runs"), plus `xorshift32` here, `struct Lcg` copies in 5 bins (+1 variant in strategy_v2), and optional `rand`. Four RNG families coexisting; every determinism claim built on LCG seeds is undermined by the constant desktop seed.
**Fix:** one engine RNG, injectable seed, delete the rest.

---

## P2 — Architecture

### 12. `GameState` sub-modules are `include!()` splices, not modules
`core/game_state/mod.rs:1242-1244`:
```rust
include!("tracking.rs");
include!("modifiers.rs");
include!("abilities.rs");
```
Same pattern in `ability/vm.rs:17-18` (decoder gen files included into vm's namespace). There are file boundaries but no encapsulation boundaries — everything is one giant scope, which is exactly how `abilities.rs` grew to 125KB and `modifiers.rs` to 79KB inside a god object.
**Fix:** convert to real `mod` declarations; fix visibility while you're there. Low risk, high payoff.

### 13. Vestigial `constants_dirty` flag
`game_state/mod.rs:88, 406, 607` sets/marks it; `modifiers.rs:214` explicitly says invalidation is "deliberately NOT gated on `constants_dirty`". The flag is written, never honored — dead state that implies an invariant that doesn't exist.
**Fix:** delete it or actually gate on it. Right now it lies to readers.

### 14. `execute_gain_resource`: 1,162-line god function, stringly-typed dispatch
`effects/misc.rs:740-1902`. Resource kind compared against raw JA/EN strings **17 times**: `resource == "blade" || resource == "ブレード" || resource == "heart" || ...` at lines 855, 930, 975, 1216, 1340, 1444, 1476, 1505, 1518, 1562, 1597, 1722, 1865... One missed JA/EN pair = platform-dependent behavior. It mixes heart-color resolution, selection filtering, blade targeting, post-filters, temp-effect bookkeeping and rule logging in one body.
**Fix:** normalize resource once into a `ResourceKind` enum at function top, split into per-resource apply fns (~500-700 lines saved, kills the entire bug class).

### 15. `qa_test_suite.rs`: 86KB regression corpus compiled into the library
`lib.rs:68` ships it in every consumer; `Cargo.toml` simultaneously has `[lib] test = false` — unit tests disabled at crate level while a test corpus lives in src/. It panics-on-first-failure with no isolation and re-reads cards.json from disk 28 times via CWD-relative paths. `run_qa_tests.rs` is a phantom bin (4 lines, orphaned).
**Fix:** move behind `#[cfg(test)]` in `tests/`, share a lazily-built DB fixture, delete `run_qa_tests.rs`, restore `[lib] test = true`.

### 16. Bot strategies v2–v5 all coexist
`bot/strategy_v2.rs` (21K), `v3` (31K), `v4` (21K), `v5` (17K) are four live generations sharing copy-paste scaffolding, including five copies of `.expect("live set actions non-empty")` — one empty-action edge case panics all of them.
**Fix:** decide the supported generation(s); extract shared action-selection scaffolding; replace the expect with a fallback policy.

---

## P3 — Dedup / merge candidates (est. savings)

| # | What | Evidence | Savings |
|---|------|----------|---------|
| 1 | `ResourceKind` enum + split of `execute_gain_resource`; kills 17 duplicated JA/EN comparisons | misc.rs:740-1902 | ~500-700 lines |
| 2 | `trigger_auto_abilities_snapshot(gs)` helper — identical pid + TriggerEvent block recurs 6× in choice.rs, 5× in misc.rs | choice.rs:2958-3048 et al. | ~110 lines |
| 3 | Unify distinct-count logic: three ~25-line arms differ only by inserted key | condition/card.rs:1734-1808, 2307, 2329 | ~80-100 lines |
| 4 | Merge `evaluate_heart_greater_than_all` / `evaluate_blade_greater_than_all` behind a stat fn param; merge 3× rule-log prefix blocks | card.rs:1475-1614; effects/mod.rs:283-337 | ~90 lines |
| 5 | Table-drive `describe_effect_en`/`_ja` around shared format fragments (or add compile-time parity test) | describe.rs:506-1161 | ~250-350 lines |
| 6 | Finish the abandoned `bin_common` migration: `struct Lcg` ×5 bins + strategy_v2 variant, `fresh_database()` ×5, deal/shuffle/setup block ×8 (`deal_from_templates` in bot_arena.rs:94-114 and diag_stall.rs:35-50 are verbatim reimplementations of `bin_common::deal_game`) | src/bin/* | ~600-800 lines |

### Smaller hygiene items
- Dead mirror code kept "for symmetry": `vm.rs:527,573` (~140 lines, `#[allow(dead_code)]`).
- `timer.rs:28,46`: `cfg!(feature = "profiling")` is a runtime branch paid on every instrumented site even with profiling off — should be `#[cfg]` on the impl or a no-op macro. Lock failures silently ignored (`timer.rs:34,52,64,138`). `print_folded()` uses `println!` while the rest uses `eprintln!`.
- `alloc_counter.rs:117-126,147`: env vars read twice (start + Drop); counting allocator adds atomics to every malloc/free whenever the feature is on, even if tracking is env-disabled.
- `bin/test_hang.rs:31`: `.ok()` discards deck-legality Result — decks silently unenforced in harnesses.
- cfg-gates live *inside* function bodies across the decoder files (47 attrs in condition_decoder_gen.rs alone). Whole-function gating would make the feature matrix testable.

---

## Overall health

The skeleton is sounder than expected: typed ActionType→handler dispatch is clean, generated decoders log unknown fields, `Result<(), String>` plumbing is pervasive, and the console/no_std porting shows real discipline. The rot is concentrated in two places:

1. **Fallback posture is inverted.** Decode failures mostly propagate honestly — but the few silent defaults (`Keyword::Turn1`, `ActionType::Custom`, `parse().unwrap_or(0)`, `i64 as u8`) sit precisely on the highest-leverage fields. That's P0 items 1-4; fixing them is small diffs with outsized correctness payoff.
2. **Files accreted instead of module boundaries holding.** Four ability files >140KB, god functions up to 1,162 lines, `include!()` pseudo-modules, and a half-finished bin_common migration. Dedup items #1/#2 above shrink misc.rs/choice.rs enough to make them reviewable again.

Suggested order: P0 #1-#4 (correctness, small) → P2 #12+#15 (module hygiene) → P3 #1-#2 (biggest merges) → P1 #8-#11 → leave the no_std cfg matrix alone until CI actually builds psp/ds/gba combos.

*Scope note: core/, ability/, root+bins were deep-dived. turn/ (230KB) and game/web_server internals beyond locking got spot-checks only.*
