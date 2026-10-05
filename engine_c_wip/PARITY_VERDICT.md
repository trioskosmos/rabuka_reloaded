# PARITY VERDICT - C engine vs the Rust reference
Measured 2026-10-05 against `aed8bd0d6`, 172 suites, two sequential runs that
agreed **exactly**. Evidence: `BASELINE4.md`, `BASELINE4_top25.txt`,
`BASELINE4_manifest_sha256.txt` (269 files,
manifest-sha256 `a022f5431015ad9ee509ad0af59aeda573e6698bfefc3c099bedca21283dd4d2`).
**VERDICT: NOT AT PARITY.** A large, flat, unresolved divergence remains.
---
## 1. What fraction of recorded assertions is verified equivalent, and on what basis
| view | suites | cases | failing | **verified equivalent** |
|---|---|---|---|---|
| A - all | 172 | 5 316 | 1 204 | **4 112 (77.35 %)** |
| B - minus the 2 non-metrics | 170 | 5 264 | 1 193 | **4 071 (77.34 %)** |
| C - minus those + the 6 gen-pinned | 164 | 5 125 | 1 157 | **3 968 (77.42 %)** |
**Basis.** A case counts as verified equivalent when the C engine produces the
same observable outcome the Rust reference is asserted to produce, recorded by a
suite that **asserts a case count** (170/172 do) and reaches a **verdict decided
on a non-zero result-line count** (not exit code - 12 suites exit 0 carrying FAIL
lines, and 1 exits 139 carrying none).
**This is a floor, not a proof of equivalence.** It is agreement on the *observed
outputs the harness compares*. It is **not** proof that the two engines are
behaviourally identical: a passing assertion bounds one input, and the corpus is
characterisation-heavy rather than exhaustive. **Read 77 % as "77 % of recorded
assertions agree", never as "77 % of the engine agrees".**
## 2. What is KNOWN not to be at parity
* **1 157 failing cases / 2 420 failing assertions in view C (22.6 %).** No
  clustering: the largest single failing signature occurs **5** times, and the
  top-25 cover **3.7 %**. This is a broad front, not a few bugs.
* **Concentrated by module:** `core/zones` 26.5 %, `ability/choice` 21.5 %,
  `modifiers` 15.7 %, `ability/cost` 8.9 % - **72.6 %** of all failing detail
  lines sit in four modules.
* **`audit_conditions` is BROKEN:** exits 2 on `cannot load card db`, **0
  assertions run**. Whatever it was going to check is unchecked.
* **`make warnings-audit` is RED:** one genuinely new bug-predicting diagnostic
  (`src/ability/effects/look.c:1577`, verified by direct compile) and one stale
  pin (`look.c:1486`). The gate that exists to predict crashes is itself red.
* **Alias-class lifetime errors are NOT cleared: 6 hits across 4 suites.** The
  mandatory negative control changes nothing (3/1/1/1 with and without the fix
  reverted), so these 6 are **unexplained**, not explained by the known-reverted
  `pca_resolve_entry` defect. Plus 4 suites where forked-child and parent
  summaries disagree, and 12 probe arms that ran no cases at all.
* **Japanese text is emitted as mojibake** (UTF-8 read as Shift-JIS) throughout
  the C suite output, e.g. `郢晢ｽｩ郢ｧ・､郢晏､懷ｹ戊沂蛹ｺ蜃ｾ`. Logs are not human-reviewable and
  any text-based audit of them is reading garbage.
## 3. What CANNOT be claimed at all, because its oracle never ran
The instruction not to run `cargo` was honoured, so **every Rust item below is a
source reading, not a measurement.** Four of the five premises in the brief were
**confirmed**; two were **wrong and are corrected**.
| claim | status |
|---|---|
| `pvp_room_test` could not compile - sole `choice_player_id` oracle | **CONFIRMED, sharper:** `pvp_room_test.rs` does **not exist anywhere** in the main tree. `choice_player_id` appears only in *production* code (`src/ability/queue.rs` x10, `src/ability/effects/choice.rs`). **There is no test for it at all**, so that behaviour has no oracle. |
| `bytecode_deep_compare_test` compiles to nothing | **CORRECTED.** The file **exists**, 193 lines, with a real `#[test] fn bytecode_deep_matches_json_path` and two `assert_eq!`. It compiles to nothing because it is **`#[cfg(feature = "bytecode_abilities")]`-gated** in `tests/test_modules/characterization/mod.rs`, not because it is empty. **The same gate hides 5 more modules, including `src/ability/vm.rs` itself** (`bytecode_abilities`, `compact_card_data` x2, `server`). A default build compiles the entire VM module to nothing. |
| `unique_abilities_test.rs` effectively empty | **CORRECTED:** it does **not exist** in the main tree. Only `engine/baked/decks/bp7_unique_abilities.bin` exists - a data blob with no test. |
| three `mod.rs` named modules absent from git | **PARTLY CORRECTED, and the mechanism is OneDrive.** 133 `mod.rs` on disk vs 135 tracked; the 2 "absent" files (`effects/choice/target_player/mod.rs`, `rules/targeting/mod.rs`) **materialised between two checks minutes apart** - a sync race, not a real absence. Exactly **one** named module genuinely cannot resolve: **`src/ability/effects/move_cards`**, declared in `src/ability/effects/mod.rs` with no such directory. |
| the Q213 test **cannot fail** (`after_waitroom >= before_waitroom`) | **NO LONGER TRUE.** That tautology is gone. The test now carries three real assertions, including `waitroom.cards.contains(&member)` and `!live_card_zone.cards.contains(&member)`. It is falsifiable. |
So the un-runnable surface is: **no `choice_player_id` oracle, no
`unique_abilities` test, one unresolvable module, and 6 feature-gated modules
(including the whole VM) invisible to a default `cargo test`.** Any parity claim
about those behaviours is **vacuous**, not weak.
## 4. Where card text and the reference disagree, and the reference is wrong
**"At parity with Rust" and "correct" are different targets.** Two places where
the C engine is right and Rust is not:
**(a) Q213 - Rust defers the eviction one phase late. CONFIRMED, and
self-documented.** `src/ability/turn/actions/mod.rs` (the brief's `:1734` is
**stale** - line 1734 is now a bare `}`). `check_invalid_live_cards` (`:1746`)
gates `evict_non_live_from_live_slot` behind `live_card_zone.face_up` (`:1751`),
and `face_up` is set at the top of `execute_performance_phase`, which runs
**after** `trigger_live_start_abilities`. Its own comment at `:1758-1760` says so:
the eviction "used to be reachable only behind `face_up` ... which runs *after*
`trigger_live_start_abilities`". The card text requires the member to be in the
waitroom **before** live-start abilities resolve. **The C engine correctly does
not inherit this ordering bug**, so the C suite is red here *because the reference
is wrong*.
**(b) The card-loss at `turn/phases.rs:846-851`: NOT RE-CONFIRMED.** The cited
range has moved; `:843-856` is now `handle_set_live_card` - a null-check, a debug
log, and `hand.cards.remove(idx)` at `:867`. I could not locate the described
card-loss at the cited lines and I am **not** endorsing that claim. It may have
been fixed, or the citation was stale. **This needs a re-locate before anyone
relies on it.**
## 5. What would have to happen for a defensible "at parity" claim
Not a rounding-off. Six concrete pieces of work:
1. **Clear 1 157 failing cases / 2 420 assertions**, starting with the four
   modules holding 72.6 % of the mass (`core/zones`, `ability/choice`,
   `modifiers`, `ability/cost`). The flat distribution means no shortcut.
2. **Fix `audit_conditions`** so it loads the card DB and asserts something.
   Today its 0 assertions are silently excluded from every total.
3. **Make `warnings-audit` green** - re-pin `FT-6` to `look.c:1577` and prove
   that diagnostic benign, or fix it. Do not silence it.
4. **Triage the 6 alias hits + 4 summary disagreements + 12 no-case probe arms.**
   Extend the negative control until it actually separates, so "alias-clean"
   means something.
5. **Close the oracle holes**: add a `choice_player_id` test; add a
   `unique_abilities` test or retire the blob; resolve
   `src/ability/effects/move_cards`; and decide whether the 6 feature-gated
   modules - **including `vm.rs`** - are meant to be invisible. If not, the
   default `cargo test` is not testing the engine it ships.
6. **Run `cargo test` on both sides and compare the same corpus**, and re-locate
   the `phases.rs` card-loss claim. Until Rust is executed, "parity" has no
   oracle behind it at all.
**Plus one thing that is not work but must be said:** the "card text vs reference"
axis needs its own column. Several C failures are cases where **Rust is wrong**.
Converging on Rust would make the engine *wrong*. Q213 is one confirmed case;
a defect-for-defect port to a buggy reference is a regression dressed as parity.
