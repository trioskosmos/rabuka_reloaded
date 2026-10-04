# PARITY VERDICT - C engine vs Rust engine

**HEAD `c9da256e5470d2fe4a47dff7daf2bb231f968e57`** | manifest `694c9ffc06d440871d87a2d64e51af7120f32dbcf6a2f2cc43300ccbade292c3` (262 files)
Both full passes agreed **EXACTLY** (0/166 suites differ, 23 386 verdict lines identical). Numbers are deterministic, not a snapshot of a moving tree.

## VERDICT: NOT AT PARITY. The evidence does not support an "at parity" claim.

### 1. What fraction is actually verified equivalent

| basis | number |
|---|---|
| suites green (of 164 metric-bearing) | **29 / 164 = 17.7 %** |
| recorded assertions passing (46 structured suites) | **913 / 1557 = 58.6 %** |
| failing assertions, corpus | **2 485** (2 387 also excluding the 6 generation-pinned suites) |
| suites red / unverified / broken | **115 / 19 / 1** |

**58.6 % of recorded assertions pass** is the most defensible figure, and it is a floor, not a
ceiling: 19 suites emit **no verdict line at all** and `audit_conditions` exits 2 silently, so their
behaviour is **unmeasured, not passing**. The 17.7 % suite figure is *not* a parity figure - a green
suite only means the C matches the expectations recorded in it.

### 2. Known NOT to be at parity

* **2 385 failing assertions across 109 red suites**, and the distribution is **flat**: top-25
  assertion signatures = **4.2 %**, largest single lexical cause = 5.3 %. There is no small set of
  defects left; this is a broad, unfinished surface.
* Concentrated by **Rust module**: `core/zones` 19.8 %, `ability/choice` 18.5 %,
  `core/game_state/modifiers` 15.1 % - **1 331 lines, 53.4 %**. **238 lines (9.5 %) match no module
  at all** and are owned by nobody.
* **Rust is wrong on at least one path, and C correctly does not inherit it** - a confirmed
  **card-loss bug at `turn/phases.rs:846-851`**: the card is `remove`d from the hand (846) *before*
  the `MAX_LIVE_CARDS` check (849), so the `Err` return at 850 **destroys the card**. The C checks
  capacity first (`src/turn/phase.c:1543`) and correctly leaves the card in hand. Any parity claim on
  that path is impossible until Rust is fixed. (The brief cited this at 831-836; those lines are a
  `log::debug!` block - the line numbers are stale, the bug is real.)
* **The Q213 citation is stale.** `turn/actions/mod.rs:1734` is the `face_up` gate that the
  surrounding comment says was the *original* defect; eviction has been split out into
  `evict_non_live_from_live_slot`, so this looks **already fixed at HEAD**. I could not run cargo to
  confirm.
* `FAIL: [VACUOUS] ... only 0 prompt(s) were offered` (46 lines, 12 suites) = the C raises **no
  prompt at all** where Rust does. E.g. **MIRAI `PL!S-bp6-021-L`**: its 自動/ターン1回 yell trigger
  fires **0 prompts** in C across every case in `parity_look_per_card`. Missing C behaviour, not a
  test artefact.

### 3. Cannot be claimed at all - the oracle never runs

| oracle | verified state |
|---|---|
| `bytecode_deep_compare_test.rs` | 7 763 B, tracked, and declared in `characterization/mod.rs` - but behind **`#[cfg(feature = "bytecode_abilities")]`, a non-default feature**. Its entire content is absent from any default `cargo test`. **Not "compiled to nothing" because it is empty - because it is feature-gated.** |
| `unique_abilities_test.rs` | **1 byte**, **not in the main tree** (only in a worktree), and **not declared** in `characterization/mod.rs`. No oracle at all. |
| `pvp_room_test.rs` | **Not in the main tree at all** (worktree only, 12 041 B), and **not declared** in `integration/mod.rs`. No oracle at all. |
| "three phantom `mod.rs` absent from all of git" | **FALSE as stated.** **141 `mod.rs` are git-tracked.** I first measured 0 - that was **my own CRLF bug** in the PowerShell filter, not a repo fact. `mod.rs` files are near-universally *legitimately empty* Rust module declarations, which is almost certainly what produced the "phantom" reading. |
| "34.8 % of lines had no Rust twin" | **Text-match artefact, not reproducible.** Same cause: grepping `mod.rs` for symbols. |
| "1 236 lines of C TEST EXISTS BUT WEAKENS" | **Not reproducible**; I could not reconstruct the method. **I do not endorse this number.** |

### 4. What a defensible "at parity" claim would require

1. **Zero red assertions** on a metric-bearing corpus. Today: 2 385. There is no shortcut - the flat
   distribution means this is ~2 385 individually-diagnosed divergences, not a handful.
2. **Close the 19 unverified suites + `audit_conditions`** so every suite emits a verdict. Until then
   a whole slice of behaviour is unmeasured and any percentage is a denominator fiction.
3. **Unify the verdict vocabulary.** 47 suites emit `name: N assertion(s), M failure(s)`; 119 emit
   `ok:`/`FAILED`. I needed two independent counting paths to cross-check; a machine-readable
   per-suite result would make the number trustworthy by construction.
4. **Re-point the 6 generation-pinned suites to derive their figures** (not new literals) - and note
   they carry **134 `FAIL` lines, not the +26 predicted**, so they are not purely pinned.
5. **Fix the Rust card-loss bug** (or document it as an intentional divergence) before any
   live-card-set path can be called equivalent.
6. **Run `cargo test` with `--features bytecode_abilities`**, and put `unique_abilities_test` and
   `pvp_room_test` into the tree and into `mod.rs`, or delete them. An oracle that cannot run is not
   evidence.
7. **Fix `t-run`/`OBJ_LIB`**: `t-run` still depends on `data`, and `OBJ_LIB` names a
   `src/web_server.o` that `make all` never builds - so `make t` only links when a stale object from
   a prior `make web` happens to be present. That is the STALE-LINK trap, live.

**Bottom line: ~58.6 % of recorded assertions are verified equivalent. The remaining ~41 % is a flat
long tail, the measurement itself has 19 blind spots, three oracles cannot run, and one confirmed
Rust-side card-loss bug means at least one path can never be at parity until Rust changes. I am not
calling this at parity, and the gap should not be rounded off.**