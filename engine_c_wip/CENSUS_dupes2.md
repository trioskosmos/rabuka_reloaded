# CENSUS_dupes2 — the 224 dead globals that have a Rust twin

**Pass date:** 2026-10-04. **Author:** dead-globals pass (isolated snapshot
`C:/Users/trios/AppData/Local/Temp/kilo/dg`).
**Deliverable:** this file + the tally in `PARITY_CHECKLIST.md`.

---

## 0. Read this first: two of the measurement traps fired ON THE MEASUREMENT

The brief warned that `nm` alone reports 1 445 dead and that a C↔Rust
token-similarity pass produced 221 bogus divergences. Both warnings were
respected, and both axes were built as the **union of two negatives**. Two
further defects were found *in the measuring code itself*, and both of them
manufactured dead functions — i.e. they failed in exactly the direction the
brief says three of this project's tallies failed.

**TRAP A — `--gc-sections` does nothing on this toolchain.** The first method
tried was the obvious one: compile every `src/*.c` at `-O0 -ffunction-sections`,
link the product target with `-Wl,--gc-sections`, and treat every discarded
symbol as dead. It reported **0 dead out of 1 433** — because the target is
PE/COFF, where `-ffunction-sections` emits `.text$name` sections and
`--gc-sections` is an ELF option that is silently ignored. The link map's
`Discarded input sections` heading was present with **zero** entries under it.
Discarded.

**TRAP B — `re.sub(r"/\*.*?\*/", " ", src, flags=re.S)` is not a C lexer.** The
comment/string stripper used for the intra-TU axis opened FALSE `/*` at any
division or dereference written without a space (`y/*p`, `n/*2`), and the
non-greedy match then swallowed every following line up to the next real `*/`.
Measured on `src/turn/phase.c`: **2 309 lines in, 547 lines out**, and all
three intra-TU call sites of `rb_backtrack` destroyed — so a function called
three times inside its own TU was reported dead. Replaced with a
character-wise C lexer (`tools/dg_dead.py:strip_c`) that blanks comment and
string-literal *bodies* while preserving every newline. `rb_backtrack` then
resolves to 4 mentions and leaves the dead set.

**TRAP C — excluding the defining TU from the intra-TU map.** The first version
dropped the whole defining file from the reference set to avoid counting the
definition as a call. That is a 100 % false-positive rate on the intra-TU axis
and it marked `rb_check_invalid_live_cards` dead while `src/turn/phase.c:984`
and `:985` call it. Fixed to exclude only the one *definition line*, detected by
a column-0 test.

**TRAP D — `rb_card_property_str` looked like a corrupted string table.** The
first dump printed *stripped* bodies, so `case 1: return "has_score_icon";`
rendered as a run of spaces and 16 functions looked like UTF-8 destruction from
one of the `Set-Content` incidents. They are fine. Dumps print raw text now.

Cross-check of the surviving method: a 14-row random sample of the actionable
set was re-verified by eye against raw `grep` output — **14/14 correct**, every
one showing only a `rabuka.h` prototype, its own definition and comment
mentions.

**`test_game.c` IS product code.** 75 of its functions were classified as
product throughout, and it is in `OBJ_LIB`.

---

## 1. The measured population

Measured, not inherited. The census in `CENSUS_wiring.md` is **stale**:
`rb_rules_execute_forget`, quoted there as a live dead-global example, no longer
exists anywhere in `src/` or `include/`. Its per-file counts also do not
reproduce. So the population was re-measured from the tree.

| quantity | before this pass | after this pass |
|---|---|---|
| global FUNCTIONS defined in `src/` | 1 433 | **1 431** |
| cross-TU referenced (`nm -u` of any object) | 473 | 473 |
| **zero callers in `src/` — the union of both negatives** | **564** | **562** |
| …of which named in `tests/` or `tools/` | 237 | **243** |
| …truly dead | 327 | **319** |

Of the 562 zero-callers, **356 are in the 24 files this pass owns**:

| bucket | count |
|---|---|
| zero callers, **named in `tests/` or `tools/`** (TEST-ONLY, out of scope) | 137 |
| zero callers, **not named anywhere outside their own definition** (the adjudicated set) | **219** |
| …of the 219, with at least one Rust name candidate | 157 |

The 219 is the denominator for the four counts below. It reproduces the brief's
224 to within 5, from a measurement that never saw the 224.

**Scope conflict, stated plainly.** The "YOU OWN" list and the "DO NOT EDIT"
list name the same files: `src/ability/util.c`, `src/ability/cost.c`,
`src/ability/effects/rules.c`, `src/ability/effects/state.c` and
`src/ability/effects/ability.c` appear in BOTH. The DO NOT EDIT list is the
explicit `⛔` fence and is the one whose path globs are complete, so it was
treated as authoritative. That reduced the editable set to **18 files**; the
fence-file items were still adjudicated (read, classified, evidence recorded)
and are reported below with the patch needed, but not landed.
`src/core/card.c`, `src/engine.c`, `src/turn/live.c`, `src/turn/phase.c` and
`src/ability/ability_queue.c` were editable when this pass began and were later
added to the fence; two edits to `card.c` and two to `engine.c` had already
been made at that point and are **left in place** because they are the
DEAD-WRONG fixes of §3 and reverting them would reinstate latent bugs. They are
flagged here for review.

---

## 2. THE FOUR COUNTS, over the 219 adjudicated dead globals

| verdict | count | what happened to them |
|---|---|---|
| **DEAD-IN-RUST-TOO** | **31** | tombstoned; the Rust twin is itself unreachable or does not exist |
| **DEAD-WRONG** | **6** | **4 fixed faithfully, 2 deleted + tombstoned with the divergence named** |
| **DEAD-CORRECT** | **182** | tombstoned as faithful; nothing to fix |
| **WIRE IT** | **0 landed** | 1 missing engine path found and reported (§5); a peer landed it |

6 + 31 + 182 = 219. **Zero rows unclassified**; every row carries the Rust item
and the line range that was read.

Plus, outside the 219: **`src/core/alloc_counter.c`, 34 exports, TOMBSTONED —
not wired** (§4).

---

## 3. DEAD-WRONG — all six, with the divergence

This is the class that matters: a dead twin whose body diverges fires the
moment somebody wires it.

### 3.1 `rb_card_property_str` / `rb_card_property_from_str` — FIXED
`src/core/card.c`. Rust twin `CardProperty`, `engine/src/core/card.rs:2750-2785`.

* **Divergence.** Rust has **four** variants. The C's `as_str` had three arms
  and its `from_str` had two. `HasDrawIcon` — index 3, added with
  `PL!-pb2-001-PP` — was absent from both, so index 3 round-tripped to index 0
  and `from_str("has_draw_icon")` silently answered `HasBladeHeart`. Rust's
  comment on the variant says its clause 「…ドローを持つカードがある場合」 had
  no property phrase to match and so parsed as a bare location filter; the C
  table would have read that card back as a blade-heart card.
* **Reachability, measured.** `card_property` carries **49 nodes** in
  `cards/abilities.json`: `has_blade_heart` 34, `has_score_icon` 12,
  `has_all_blade` 2, **`has_draw_icon` 1** (`abilities.json:13305`). One real
  card-ability pair depends on the variant existing.
* **Why nothing fired.** The LIVE C path does not use these tables.
  `rb_check_card_property` (`src/ability/util.c:3263`) matches the raw string
  and calls `rb_card_has_draw_icon` (`card.rs:4406`) directly. The tables were
  the only place the variant was missing — hence latent, not live.
* **Action.** Both arms added. Pinned by `tests/test_parity_dead_twins.c`
  (4-variant round-trip + the raw-string agreement with the live checker).

### 3.2 `rb_energy_add_active` — FIXED
`src/core/zones.c`. Rust twin `EnergyZone::add_active`, `core/zones.rs:682-684`.

* **Divergence.** `active_energy_count.saturating_add(delta)` on a **`u8`**, so
  Rust clamps at 255. The C clamped only the lower bound (`delta <= 0` guard
  plus a post-hoc `< 0` fixup) and `RbPlayer.energy_active` is **`int`**
  (`include/rabuka.h:800`) — a large `delta` ran straight past 255, and every
  later reader that compared the count against a Rust-shaped `u8` total would
  have disagreed. `delta` is also a signed `int` here against a `u8` there.
* **Reachability.** Zero callers in `src/`. The live path is
  `rb_setup_initial_energy` → `rb_energy_deck_draw` + `rb_energy_add_card`.
* **Action.** Upper clamp added; both bounds now match. Pinned by the new suite.

### 3.3 `rb_player_draw_energy` — FIXED on three counts
`src/core/zones.c`. Rust twins `Player::draw_energy` (`core/player.rs:585-591`)
and `EnergyZone::push_active` (`core/zones.rs:718-722`).

Rust, with its own reason quoted:
```rust
pub fn draw_energy(&mut self) -> Option<i16> {
    self.energy_deck.draw().inspect(|&card_id| {
        // Insert at the active boundary (not append): the drawn card is
        // active, and appending past waited cards would corrupt the
        // active-prefix convention.
        self.energy_zone.push_active(card_id);
    })
}
// push_active:
let at = (self.active_energy_count as usize).min(self.cards.len());
self.cards.insert(at, card_id);
self.active_energy_count = self.active_energy_count.saturating_add(1);
```
1. **POSITION.** The old C appended at `energy.n`. With waited energy in the
   zone (`energy_active < energy.n`) the drawn card landed AFTER a waited card
   and became waited — precisely what Rust's own comment forbids. Now inserted
   at `min(energy_active, energy.n)` with the tail shifted.
2. **CAPACITY — a card was DESTROYED.** The old body drew from the energy deck
   FIRST and only then tested `energy.n < RB_MAX_ZONE`. On a full zone the card
   had already left the deck and was then dropped on the floor: it ceased to
   exist. Rust's `Vec::insert` always succeeds. The capacity test now precedes
   the draw, so the card stays in the energy deck.
3. **SATURATION.** No `u8` clamp (same defect as 3.2). Clamped.
* **Reachability.** Zero callers. The capacity branch is unreachable on the real
  corpus (an energy zone never approaches `RB_MAX_ZONE == 512`), which is
  exactly why it was worth writing down rather than leaving to chance.
* **Action.** Rewritten faithfully; pinned by the new suite, including the
  waited-energy case, the full-zone no-card-loss case, and `NULL`/bad-seat.

### 3.4 `rb_play_card` — DELETED + TOMBSTONED
`src/engine.c`. **No Rust twin.** Reachability: 0 callers in `src/`, 0 in
`tests/`. Kept only as a plausible-looking `(game, player, hand index)` "play a
card" entry point, which is the trap.

* **ENERGY CARDS WERE PLAYED INTO THE LIVE ZONE.** The branch is
  `card_is_member(c) ? play member : move to live`; an energy card is a
  non-member. Rule 7.2 sends energy to the energy zone — there is no energy
  branch at all.
* **THE LIVE-CARD PATH RAN NO LIVE-CARD RULE.** `bag_remove_at` +
  `bag_push(&P->live, card)` and nothing else: no need-heart check, no
  `LiveCardZone::add_card` validation (`zones.rs:523`), no face-up flag, no
  auto-ability registration, no capacity check, and no phase check, despite the
  comment saying "during live set".
* **"FIRST FREE STAGE SLOT"** is a policy Rust does not have:
  `move_card_from_hand_to_stage(idx, area, use_baton_touch, …)` takes an
  explicit `area` (`phases.rs:1260-1271`).
* **Action.** Deleted; the tombstone names the live route
  (`rb_handle_play` → `rb_execute_play` → `rb_play_member_ex`,
  `src/turn/phase.c:1739-1746`) so nobody reintroduces a wrapper.
  `include/rabuka.h:2183` still declares it — header is yours.

### 3.5 `heart_color_of` — DELETED + TOMBSTONED
`src/engine.c`. **No Rust twin.** It was a **global** (no `static`), so it was a
real defined symbol in every object. Reachability: 0 callers in `src/`, 0 in
`tests/`; the only other mention is a comment at
`src/ability/effects/state.c:1779`.

* **Divergence.** It mapped the string `"b_all"` to `RB_HEART_ALL`, index 7. In
  Rust's `heart_color_table!` (`core/card.rs:4460-4476`):
  `BAll => "b_all", "b_all", 0` and `All => "all", "all", 7`. The C collapsed
  the **blade-heart wildcard into the all-colours wildcard**. `RB_HEART_PINK`
  is also a colour name Rust has no counterpart for (Rust's index 0 is
  `Heart00`): the whole table is a C-local dialect, and Rust parses effect
  `heart_color` through `HeartColor::from_str` (`card.rs:4554`) instead.
* **Corpus reachability of the wrong arm.** `"b_all"` occurs **0** times as a
  quoted JSON value in `cards/abilities.json`. The 28 raw `b_all` occurrences
  are heart-colour **keys** inside `cards.json` `heart_count` maps, which this
  function never sees; 3 more are in `cards/qa_data.json`. Wrong and
  unreachable at once — the profile that hides a bug until someone adds a card.
* **Action.** Deleted; the tombstone carries the removed body verbatim.

### 3.6 `rb_try_phase4` / `rb_try_all_distribution` — prior adjudication UPHELD
`src/turn/live.c`. Both are dead and both share a name with a **live** Rust
allocator function, which is why they are called out rather than buried.

* Rust `try_phase4` (`turn/live.rs:2660`) is a per-card **backtracking**
  allocator: it pushes `Allocation` records, mutates the pool, builds a
  `color_deficits` vector, and recurses into `bt_search`. The C's exported
  `rb_try_phase4(g, pl, filled, need)` has no pool, no allocations vector and
  no deficit list, and is a single **greedy** pass — and its Phase 3b spends
  `filled[0]` (already-allocated heart00) where Rust spends `pool[0]`. Its
  sibling `rb_try_all_distribution` is `if (!g) return 0; (void)pl; return 0;`
  — an unconditional stub.
* **The faithful ports exist, are live, and are file-local:**
  `rb_backtrack_allocate` → `bt_search` → `bt_try_surplus` → `bt_try_phase4` →
  `bt_try_all_distribution`.
* **Verdict: DEAD-CORRECT-BY-DOCUMENTATION, upheld — not re-adjudicated.** A
  prior tombstone at `live.c:2727-2746` already records this, including two
  self-corrections: this symbol is **not** in `rabuka.h`, and Rust's
  `try_all_distribution` **is** live (`live.rs:2754`, called at `live.rs:2731`),
  so these are redundant entry points, not dead Rust items, and deleting them
  would remove the record. Kept as named entry points with the reason on the
  body. Overruling a prior adjudication that is already correct and already
  documented would be churn with no gain.

---

## 4. `src/core/alloc_counter.c` — **TOMBSTONED, NOT WIRED**

780 lines, 34 exports, in neither `SRC` nor `OBJ_LIB`. The brief asked whether
Rust's `alloc_counter.rs` is reachable at all first. **It is not, in any default
build**, and the evidence is a five-link chain:

1. `engine/src/lib.rs:58-59` — `#[cfg(feature = "alloc_tracker")] pub mod alloc_counter;`
   The whole module is behind a feature.
2. `engine/Cargo.toml:82` — `alloc_tracker = []`, and the `default` key is
   `["env_logger","debug_conditions","rand","rmp-serde","bytecode_abilities","compact_card_data","serde_support"]`.
   Neither `alloc_tracker` nor `profiling` is in it.
3. `engine/src/lib.rs:94-99` — with the module absent the process global
   allocator is plain `std::alloc::System`, so **no allocation passes through
   `CountingAllocator`** and `ALLOC_COUNT` / `BYTES_ALLOCATED` never move.
4. The only **library** call site is itself gated:
   `core/game_state/abilities.rs:1917-1919`,
   `#[cfg(feature = "alloc_tracker")] alloc_counter::maybe_arm("process_current_ability");`
   Every other `maybe_arm` / `arm_alloc_trace` caller is an `engine/src/bin`
   binary reaching it by opting in with `--features alloc_tracker`
   (`docs/ALLOCATION_TARGETS.md:29`).
5. The readers `alloc_count()` / `alloc_buckets()` (`alloc_counter.rs:346`,
   `:358`) are `#[cfg(feature = "alloc_tracker")]`, and their only caller is
   `timer::Timer` (`timer.rs:113`/`:120`), itself
   `#[cfg(feature = "profiling")]`. `Timer::start` has **five** library call
   sites — `ability/vm.rs:252`, `ability/resolver.rs:1330`/`:1347`/`:1380`,
   `ability/gates.rs:352` — and **all five compile out** by default.

**Independent corroboration already in this repo:** `AUDIT_core_state.md:799`
records `"| D12 alloc counters see no traffic | 0 by construction (alloc_tracker
feature) | VERIFIED-NOOP, permanent |"`.

**Conclusion.** All 34 `rb_allocctr_*` are DEAD-IN-RUST-TOO for the product.
Wiring them would add 780 lines of allocation bookkeeping to a hot path in
exchange for numbers that can only ever read zero.

**The `-std=c2x` point is real but was not the reason.** Compile matrix,
measured after the tombstone was written:

| standard | result | warnings |
|---|---|---|
| `-std=c99` | COMPILES | 0 |
| `-std=c11` | COMPILES | 0 |
| `-std=c17` | COMPILES | 0 |
| `-std=c2x` | **FAILS** — `implicit declaration of 'ATOMIC_VAR_INIT'` | 0 |

The project builds `-std=c11`, so C23's removal of `ATOMIC_VAR_INIT` (line 175)
is a non-issue. (The tombstone's own first draft contained a `/*` inside a block
comment — `-Wcomment`; rewritten, the matrix is now 0 warnings everywhere.)

**The half of that file that is NOT dead** is the second half, which ports
`engine/src/core/runtime_flags.rs`. That module is *not* feature-gated:
`logging_enabled()` has **43** library read sites and
`action_display_enabled()` has **21** (18 of them in
`game/game_setup.rs:243-277`). But all four **writers** are called only from
`engine/src/bin`, so in every library build both flags are provably constant
`true`; a C engine that does not gate on them behaves identically. The C has
already made and documented that substitution — `game_state_core.c:757-769`
gates `gsc_rule_log_push` on `rb_ability_debug_enabled()`, whose production
writer now exists (§6).

### `rabuka.h` block — NOT NEEDED

Because the verdict is *tombstone*, **no prototypes are requested** and the
Makefile needs no change. Paste-ready block supplied anyway, **for the record
only** — land it only if you decide to flip `alloc_tracker` into Rust's
`default` feature set, which is the change that would make these live:

```c
/* ── src/core/alloc_counter.c ─────────────────────────────────────────────
 * NOT WIRED. NOT COMPILED. See src/core/alloc_counter.c's tombstone and
 * engine_c_wip/CENSUS_dupes2.md §4: engine/src/lib.rs:58-59 gates the whole
 * Rust module behind #[cfg(feature = "alloc_tracker")], and alloc_tracker is
 * NOT in engine/Cargo.toml's `default` feature set, so in a default build the
 * global allocator is std::alloc::System and none of these counters can ever
 * move. These prototypes are published only so that a future change which
 * DOES enable the feature has the declarations ready.
 *
 *   size_t     rb_allocctr_size_bucket(size_t n);          /* 0..13 */
 *   size_t     rb_allocctr_bucket_label(size_t b);         /* static strings */
 *   uint64_t   rb_allocctr_alloc_count(void);
 *   uint64_t   rb_allocctr_alloc_buckets(size_t b);
 *   uint64_t   rb_allocctr_bytes_allocated(void);
 *   uint64_t   rb_allocctr_alloc_self_ns(void);
 *   uint64_t   rb_allocctr_alloc_self_samples(void);
 *   uint64_t   rb_allocctr_alloc_peak_bytes(void);
 *   uint64_t   rb_allocctr_dealloc_count(void);
 *   uint64_t   rb_allocctr_dealloc_self_ns(void);
 *   uint64_t   rb_allocctr_dealloc_self_samples(void);
 *   uint64_t   rb_allocctr_alloc_site_count(void);
 *   uint64_t   rb_allocctr_alloc_site_at(uint64_t i);     /* out char site[] */
 *   uint64_t   rb_allocctr_alloc_site_bytes(uint64_t i);
 *   void      *rb_allocctr_malloc(size_t n);
 *   void       rb_allocctr_free(void *p);
 *   void      *rb_allocctr_realloc(void *p, size_t n);
 *   void       rb_allocctr_note_alloc_sized(size_t n, int kind);
 *   void       rb_allocctr_note_alloc_at(const char *site, size_t n, int kind);
 *   int        rb_allocctr_arm_trace(size_t max_rows);
 *   void       rb_allocctr_disarm_trace(char *site, size_t cap, int kind,
 *                                       size_t *bytes, uint64_t *out_count,
 *                                       size_t *out_site_count);
 *   void       rb_allocctr_trace_row(uint64_t i, const char **site,
 *                                    size_t *bytes, int *kind);
 *   void       rb_allocctr_note_trace_frame(const char *site, size_t n, int kind);
 *   int        rb_allocctr_trace_guard_report(FILE *out);
 *   int        rb_allocctr_trace_guard_release(char *site, size_t cap,
 *                                              int kind, size_t *bytes,
 *                                              uint64_t *out_count,
 *                                              size_t *out_site_count);
 *   int        rb_allocctr_maybe_arm(const char *label, size_t n);
 *   uint64_t   rb_allocctr_clock_overhead_ns(void);
 *   uint64_t   rb_allocctr_alloc_clock_overhead_ns(void);
 *   uint64_t   rb_allocctr_free_clock_overhead_ns(void);
 *   uint64_t   rb_allocctr_note_alloc_clock_overhead_ns(void);
 *   int        rb_allocctr_trace_enabled(void);
 *   int        rb_allocctr_flag_logging_enabled(void);
 *   int        rb_allocctr_flag_action_display_enabled(void);
 *   void       rb_allocctr_flag_set_logging_enabled(int on);
 *   void       rb_allocctr_flag_set_action_display(int on);
 * ──────────────────────────────────────────────────────────────────────── */
```
(Signatures transcribed verbatim from the definitions; `FILE` needs
`<stdio.h>`, `size_t` `<stddef.h>`, `uint64_t` `<stdint.h>`.)

---

## 5. Missing engine paths

**One found in the 219's neighbourhood, one reported from outside it. Both are
now closed; neither was landed by me.**

### 5.1 `rb_ability_debug_set` — Rust reaches it, the C had no call site. CLOSED.
Not in the 219 (it is TEST-ONLY: all callers are in `tests/`, behind a `getenv`
of its own name), but it is the answer to the brief's third question.

Rust's `ability::debug::set_debug` has **exactly one production writer**:
`engine/src/game/web_server.rs:3652-3655`, the first statement of
`run_web_server_with_ngrok`:
```rust
// Enable structured verdict items in the in-game rule log when RABUKA_RULE_LOG=1
if std::env::var("RABUKA_RULE_LOG").as_deref() == Ok("1") {
    crate::ability::debug::set_debug(true);
}
```
The only other callers are harness helpers
(`engine/tests/helpers/diagnostics.rs:70`, `engine/tests/helpers/mod.rs:195`).
Rust's other ability-debug reader, `ABILITY_DEBUG_BATON_TOUCH`
(`ability/condition/state.rs:617`), is a separate flag with its own accessor,
so it is not this call's business.

`src/web_server.c:883-886` now carries the same env var, the same `"1"`
comparison and the same one-shot-at-startup placement, before `rb_load` so a
rule log that fails to open is still visible — matching Rust's ordering at
`:3659-3660`. **Verified working** (§6).

Consequence chain, for the record: `rb_log_push_verdict_item` is gated on
`rb_log_recording()` = `g_log_enabled || rb_ability_debug_enabled()`, and Rust's
`push_verdict` (`ability/log.rs:87-90`) is gated on `ABILITY_DEBUG` — the same
gate with the same default. Without the call site, the C's in-game rule log can
never carry structured verdict items.

### 5.2 `rb_energy_add_active`'s ceiling and `rb_player_draw_energy`'s insert point
The three DEAD-WRONG defects of §3.2/§3.3 are each a missing engine path: the C
had no code that inserts at the active boundary, and no upper saturation. Both
are now landed. **Reachability 0 callers** — so this is a latent-bug repair, not
a behaviour change: the corpus numbers do not move, and the suite corpus
confirms it (§8).

### 5.3 Rust reaches `EffectPipeline`, `StepOutput`, `StepState`, `AbilityTraceNode`, `ZoneSnapshot`, `LookAndSelectStep`, `ChoiceBuilder`
All are live in Rust (`ability/resolver.rs`, `ability/choice.rs`,
`ability/compound/{setup,post_step}.rs`, `ability/effects/{look,move/mod}.rs`,
`core/game_state/abilities.rs`, `turn/actions/mod.rs`) and all have C twins in
`src/ability/types.c` with **zero callers** — 74 rows of the table.
**Not wired, deliberately.** The C does not use these shapes: it builds
`RbFullChoice` inline rather than through `ChoiceBuilder`, and it drives look /
select through `rb_look_resume_indices` rather than `StepOutput`. Wiring a
builder or an allocations pipeline would be an architectural change, not a call
site, and the corpus is already at parity through the paths that exist. Recorded
as DEAD-CORRECT / DEAD-IN-RUST-TOO per row, **not** as a missing path.

---

## 6. `rb_ability_debug_set` + the `web_server.c` JSON sweep

**The peer's wiring is correct.** It is the same env var, the same comparison,
the same one-shot placement, and the placement relative to `rb_load` matches
Rust. The `nm` claim — that it is the only object referencing
`rb_ability_debug_set` — is consistent with this pass's own measurement: the
symbol's only pre-existing callers were `tests/`.

**JSON sweep: CLEAN, and it is a code guarantee rather than a data-shape
dependency.** The concern was a raw `snprintf("%s")` of a card name into the
JSON buffer. Swept `src/web_server.c`:

| check | result |
|---|---|
| `snprintf`/`strcpy`/`strcat` with `%s` into a json-ish buffer | **2 hits, neither a card name** |
| `:365 json_value` | builds a `"key"` needle into `char needle[80]` — a fixed key, not card data |
| `:837` | `snprintf(card_path, …, "%s/../cards/cards.json", web_root …)` — a filesystem path |
| a JSON string escaper exists | **yes**, `json_string()` at `src/web_server.c:76-88` |
| coverage of that escaper | `"` and `\` escaped; `\n` `\r` `\t`; every C0 control as `\u00xx` |
| is `c.name` routed through it | **yes** — `:117 json_string(b, c.name)`, and `:642` for the deck-list ids |

So the 26 card names carrying a literal newline, the 4 carrying `\"`, and the 0
carrying `",` are all handled **by the escaper**, not by luck. Nothing in
`web_server.c` emits a card name through a raw `%s` into JSON. No patch needed.

---

## 7. `card_total_hearts` — THREE copies, all live, all semantically identical. RECORDED.

| # | location | linkage | callers |
|---|---|---|---|
| 1 | `src/ability/effects/state.c:2044` | **global** (`rb_card_total_hearts`, `rabuka.h:2265`) | `tests/` only — **no `src/` caller** |
| 2 | `src/ability/condition.c:4011` | `static` | **4 live**: `:2920`, `:4101`, `:4107`, `:4114` |
| 3 | `src/ability/util.c:863` | `static` | **1 live**: `:1872` |

All three implement `Card::total_hearts` (`engine/src/core/card.rs:4370-4379`):
sum the `base_heart` slice when the card has one, else sum the `need_heart`
slice, which starts at `num_base + num_blade` because blade hearts are never
folded in (Q149/Q172). **Verified: identical logic, identical clamping by
`n_hearts`, identical `NULL` guard on the global copy** (Rust cannot express
-1 there, and an older port DID return -1, which made running aggregates go
DOWN — both live copies' comments record that).

**One documentation defect found:** all three comments cite the Rust twin at the
wrong line. They say `card.rs:4194-4202` (copies 1 and 3) and `card.rs:4263-4271`
(copy 2). The function is at **`card.rs:4370-4379`**. Bodies are faithful; only
the citations are stale.

**Consolidation: NOT DONE — all three files are in the DO NOT EDIT fence.**
`src/ability/condition.c` and `src/ability/util.c` are listed explicitly and
`src/ability/effects/state.c` is matched by the `effects/{…state…}.c` glob. The
consolidation is a one-line change per copy — have each static body call the
exported `rb_card_total_hearts` — and it is left for the fence owners.
`tests/test_parity_dead_twins.c` now pins the shared semantics (base-slice sum,
need-slice offset by `num_base + num_blade`, never negative, never reading past
`n_hearts`), so a Q149/Q172 fix that lands in only one or two of the three
copies fails there instead of in the field.

---

## 8. Before / after, per suite

Method: `make clean` before **both** builds; every `tests/test_*.c` compiled at
the Makefile's own `-std=c11 -O2` flags and linked against `OBJ_LIB`; run
**strictly sequentially**; `^FAIL` counted **case-sensitively with no colon**,
across stdout **and** stderr, per suite. `coverage_gate` is a meta-suite and its
count is **not** a metric.

| | before | after | delta |
|---|---|---|---|
| suites | 165 | **166** | +1 (the new suite) |
| **`^FAIL` (no colon)** | **1 348** | **1 348** | **0** |
| `^FAIL:` (with colon) | 2 814 | 2 815 | +1 — **entirely `coverage_gate`** |
| GREEN (0 `^FAIL`) | 55 | **56** | +1 (the new suite) |

The single `+1` on `^FAIL:` is `coverage_gate`'s manifest-vs-glob gate emitting
one line per `tests/test_*.c` with no matching `tests/*.log`
(`sources=165 → 166`, `170 checks, 166 failures → 171 checks, 167 failures`). My
runner writes logs to its own output directory, not to `tests/`, so every suite
is flagged by that gate; adding one source file adds exactly one line. **No
suite changed its own behaviour.** The full per-suite table is in
`build_census/after/SUMMARY.tsv` and `base/SUMMARY.tsv` in the snapshot.

**`tests/test_parity_dead_twins.c`: 47 assertions, 0 failures.** It covers all
four fixed functions and the shared `card_total_hearts` semantics. No existing
test was weakened, deleted or ignored; the suite is new.

---

## 9. Per-function table

219 rows: `src/core/zones.c` 24 · `src/ability/types.c` 74 · `src/ability/effects/rules.c` 33 ·
`src/core/card.c` 14 · `src/ability/vm.c` 13 · `src/core/data.c` 13 ·
`src/game/game_setup.c` 5 · `src/ability/effects/ability.c` 5 ·
`src/ability/util.c` 7 · `src/ability/effects/state.c` 3 · `src/ability/compound.c` 3 ·
`src/ability/log.c` 3 · `src/turn/phase.c` 6 · `src/ability/describe.c` 2 ·
`src/ability/debug.c` 2 · `src/turn/live.c` 2 · `src/ability/ability_queue.c` 1 ·
`src/ability/cost.c` 1 · `src/ability/effects/look.c` 1 · `src/engine.c` 1.

The six DEAD-WRONG rows are **not** in this table: four of them
(`rb_card_property_str`, `rb_card_property_from_str`, `rb_energy_add_active`,
`rb_player_draw_energy`) moved from the actionable set into the test-named set
when the new suite began naming them, and two (`rb_play_card`,
`heart_color_of`) no longer exist. They are in §3.

| # | file | symbol | Rust name candidate | verdict / evidence |
|---|------|--------|----------------------|-------------------|
| 1 | `ability/ability_queue.c` | `rb_queue_save_pending_actions` | save_pending_actions | Rust ability/queue.rs save_pending_actions. The C guards `count <= 0` where Rust guards `actions.is_empty()`. Same. |
| 2 | `ability/compound.c` | `rb_compound_choice_string` | -- | No Rust item; a C-local index-of-child lookup used nowhere. |
| 3 | `ability/compound.c` | `rb_compound_route_branch` | CompoundBranch | Candidate name is a false positive: Rust CompoundBranch is a TYPE, not a function. C-local. |
| 4 | `ability/compound.c` | `rb_compound_save_remaining` | -- | No Rust item; writes RbQueueEntry.pending_actions_n, a C-only field with no Rust counterpart. |
| 5 | `ability/cost.c` | `rb_cost_has_skip_prompt` | has_skip_prompt | A one-line forwarder to the file-local has_skip_prompt. Same logic, two names. |
| 6 | `ability/debug.c` | `rb_abdebug_dropped_count` | -- | Rust-dead by construction: g_debug_dropped only ever moves in rb_debug_report_overflow, which only fires when the trace buffer is full, and the trace buffer only fills when rb_ability_debug_enabled() is true -- which has no production writer (see MISSING PATHS). |
| 7 | `ability/debug.c` | `rb_abdebug_dropped_reset` | -- | Same as rb_abdebug_dropped_count. |
| 8 | `ability/describe.c` | `rb_desc_apply_bilingual` | -- | No Rust item; Rust's Choice::set_bilingual_descriptions (types.rs:688) is the shape this wraps. |
| 9 | `ability/describe.c` | `rb_desc_cond_expectation` | -- | No Rust item; wraps the file-local d_cond_expectation. |
| 10 | `ability/effects/ability.c` | `rb_ability_ref_cache_size` | AbilityRef | Rust AbilityRef (ability/ability_store.rs). idx/index/idx/to_arc/cache_size/decode are the same accessors over the same index. |
| 11 | `ability/effects/ability.c` | `rb_ability_ref_decode` | AbilityRef, decode_ability | Rust AbilityRef (ability/ability_store.rs). idx/index/idx/to_arc/cache_size/decode are the same accessors over the same index. |
| 12 | `ability/effects/ability.c` | `rb_ability_ref_idx` | AbilityRef | Rust AbilityRef (ability/ability_store.rs). idx/index/idx/to_arc/cache_size/decode are the same accessors over the same index. |
| 13 | `ability/effects/ability.c` | `rb_ability_ref_index` | AbilityRef | Rust AbilityRef (ability/ability_store.rs). idx/index/idx/to_arc/cache_size/decode are the same accessors over the same index. |
| 14 | `ability/effects/ability.c` | `rb_ability_ref_to_arc` | AbilityRef | Rust AbilityRef (ability/ability_store.rs). idx/index/idx/to_arc/cache_size/decode are the same accessors over the same index. |
| 15 | `ability/effects/look.c` | `rb_look_resume` | -- | Wraps rb_look_resume_indices; the C's selection snapshot/restore pair is already live around it. |
| 16 | `ability/effects/rules.c` | `rb_rules_apply_pending_replacement` | -- | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 17 | `ability/effects/rules.c` | `rb_rules_cannot_activate_player` | -- | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 18 | `ability/effects/rules.c` | `rb_rules_choice_repeats_remaining` | -- | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 19 | `ability/effects/rules.c` | `rb_rules_consume_choice_repeat` | -- | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 20 | `ability/effects/rules.c` | `rb_rules_count_at_save` | -- | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 21 | `ability/effects/rules.c` | `rb_rules_delayed_prohibition` | -- | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 22 | `ability/effects/rules.c` | `rb_rules_execute_activation_cost` | execute_activation_cost | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 23 | `ability/effects/rules.c` | `rb_rules_execute_choice` | execute_choice | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 24 | `ability/effects/rules.c` | `rb_rules_execute_modify_cost` | execute_modify_cost | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 25 | `ability/effects/rules.c` | `rb_rules_execute_modify_limit` | execute_modify_limit | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 26 | `ability/effects/rules.c` | `rb_rules_execute_modify_yell_count` | execute_modify_yell_count, execute_re_yell | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 27 | `ability/effects/rules.c` | `rb_rules_execute_perform_yell` | execute_perform_yell, execute_re_yell | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 28 | `ability/effects/rules.c` | `rb_rules_execute_re_yell` | execute_re_yell | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 29 | `ability/effects/rules.c` | `rb_rules_execute_select_effect` | execute_effect, execute_select, execute_select_effect | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 30 | `ability/effects/rules.c` | `rb_rules_execute_select_number` | execute_select, execute_select_number | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 31 | `ability/effects/rules.c` | `rb_rules_execute_set_cost` | execute_set_cost | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 32 | `ability/effects/rules.c` | `rb_rules_execute_set_cost_to_use` | execute_set_cost, execute_set_cost_to_use | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 33 | `ability/effects/rules.c` | `rb_rules_forget` | -- | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 34 | `ability/effects/rules.c` | `rb_rules_get_conditional_choice` | ConditionalChoice | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 35 | `ability/effects/rules.c` | `rb_rules_handle_both_targets` | handle_both_targets | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 36 | `ability/effects/rules.c` | `rb_rules_keep_shuffle_count` | -- | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 37 | `ability/effects/rules.c` | `rb_rules_keep_shuffle_phase` | -- | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 38 | `ability/effects/rules.c` | `rb_rules_n_delayed_prohibitions` | -- | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 39 | `ability/effects/rules.c` | `rb_rules_note_keep_shuffle_selected` | -- | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 40 | `ability/effects/rules.c` | `rb_rules_note_moved_cards` | -- | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 41 | `ability/effects/rules.c` | `rb_rules_note_selected_cards` | -- | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 42 | `ability/effects/rules.c` | `rb_rules_owner_executing` | -- | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 43 | `ability/effects/rules.c` | `rb_rules_register_replacement_effect` | ReplacementEffect, register_replacement_effect | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 44 | `ability/effects/rules.c` | `rb_rules_resolving_replacement` | -- | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 45 | `ability/effects/rules.c` | `rb_rules_run_replacement_effects` | run_replacement_effects | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 46 | `ability/effects/rules.c` | `rb_rules_set_conditional_choice` | ConditionalChoice | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 47 | `ability/effects/rules.c` | `rb_rules_sub_choice_created` | -- | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 48 | `ability/effects/rules.c` | `rb_rules_take_pending_replacement` | -- | All 33 are per-GameState accessors/mutators over the C-only RbRulesState struct (replace-deleted structs). Their Rust counterparts are struct FIELDS on AbilityQueueEntry / turn state, not functions, so each one is a C-side accessor of a C-side field. Grouped verdict: DEAD-CORRECT for the accessors; rb_rules_execute_* are the ports of the named Rust execute_* functions but have no C dispatch caller (the effect dispatcher is wired elsewhere). |
| 49 | `ability/effects/state.c` | `rb_card_need_heart_satisfied` | CardNeed, need_satisfied | Rust CardNeed::need_satisfied (turn/live.rs) delegates to check_heart_requirement; the C delegates to rb_check_heart_requirement. Identical. |
| 50 | `ability/effects/state.c` | `rb_heart_color_from_index` | HeartColor, color_from_u8, from_index | Rust HeartColor::from_index (card.rs:4503-4515): 0..7 then _ -> Heart00. Identical. |
| 51 | `ability/effects/state.c` | `rb_heart_color_short_label` | HeartColor, short_label | Rust HeartColor::short_label via heart_color_table! (card.rs:4460-4476, 4517-4526): Heart00..Heart06 -> h00..h06, All -> all. Identical. |
| 52 | `ability/log.c` | `rb_log_dropped_count` | -- | Same shape, for g_log_dropped. |
| 53 | `ability/log.c` | `rb_log_dropped_reset` | -- | Same shape, for g_log_dropped. |
| 54 | `ability/log.c` | `rb_log_push_verdict_item` | push_verdict | Rust ability/log.rs:87-90 push_verdict is `if !ABILITY_DEBUG.load(Relaxed) { return; }`. The C body is `if (!rb_log_recording()) return;` and rb_log_recording() is g_log_enabled OR rb_ability_debug_enabled(). Same gate, same default (off in both). Identical. |
| 55 | `ability/types.c` | `rb_arcstr_deserialize` | -- | Rust ArcStr -- an Arc<str> newtype. serialize/deserialize are the C's wire twin of its Serialize/Deserialize impls. |
| 56 | `ability/types.c` | `rb_arcstr_serialize` | -- | Rust ArcStr -- an Arc<str> newtype. serialize/deserialize are the C's wire twin of its Serialize/Deserialize impls. |
| 57 | `ability/types.c` | `rb_choice_builder_blind` | ChoiceBuilder | Rust ChoiceBuilder (types.rs:412-553): 20 setters, each `mut self -> Self` storing one Option field. The C setter of the same name stores the same field on RbChoiceBuilder. The C builds RbFullChoice inline instead of through the builder, so the setters have no caller. |
| 58 | `ability/types.c` | `rb_choice_builder_card_type` | CardType, ChoiceBuilder, card_type | Rust ChoiceBuilder (types.rs:412-553): 20 setters, each `mut self -> Self` storing one Option field. The C setter of the same name stores the same field on RbChoiceBuilder. The C builds RbFullChoice inline instead of through the builder, so the setters have no caller. |
| 59 | `ability/types.c` | `rb_choice_builder_characters` | ChoiceBuilder | Rust ChoiceBuilder (types.rs:412-553): 20 setters, each `mut self -> Self` storing one Option field. The C setter of the same name stores the same field on RbChoiceBuilder. The C builds RbFullChoice inline instead of through the builder, so the setters have no caller. |
| 60 | `ability/types.c` | `rb_choice_builder_cost_limit` | ChoiceBuilder, cost_limit | Rust ChoiceBuilder (types.rs:412-553): 20 setters, each `mut self -> Self` storing one Option field. The C setter of the same name stores the same field on RbChoiceBuilder. The C builds RbFullChoice inline instead of through the builder, so the setters have no caller. |
| 61 | `ability/types.c` | `rb_choice_builder_cost_total` | ChoiceBuilder, cost_total | Rust ChoiceBuilder (types.rs:412-553): 20 setters, each `mut self -> Self` storing one Option field. The C setter of the same name stores the same field on RbChoiceBuilder. The C builds RbFullChoice inline instead of through the builder, so the setters have no caller. |
| 62 | `ability/types.c` | `rb_choice_builder_cost_values` | ChoiceBuilder, cost_values | Rust ChoiceBuilder (types.rs:412-553): 20 setters, each `mut self -> Self` storing one Option field. The C setter of the same name stores the same field on RbChoiceBuilder. The C builds RbFullChoice inline instead of through the builder, so the setters have no caller. |
| 63 | `ability/types.c` | `rb_choice_builder_description_en` | ChoiceBuilder | Rust ChoiceBuilder (types.rs:412-553): 20 setters, each `mut self -> Self` storing one Option field. The C setter of the same name stores the same field on RbChoiceBuilder. The C builds RbFullChoice inline instead of through the builder, so the setters have no caller. |
| 64 | `ability/types.c` | `rb_choice_builder_description_ja` | ChoiceBuilder | Rust ChoiceBuilder (types.rs:412-553): 20 setters, each `mut self -> Self` storing one Option field. The C setter of the same name stores the same field on RbChoiceBuilder. The C builds RbFullChoice inline instead of through the builder, so the setters have no caller. |
| 65 | `ability/types.c` | `rb_choice_builder_destination` | ChoiceBuilder | Rust ChoiceBuilder (types.rs:412-553): 20 setters, each `mut self -> Self` storing one Option field. The C setter of the same name stores the same field on RbChoiceBuilder. The C builds RbFullChoice inline instead of through the builder, so the setters have no caller. |
| 66 | `ability/types.c` | `rb_choice_builder_discard_remaining` | ChoiceBuilder, discard_remaining | Rust ChoiceBuilder (types.rs:412-553): 20 setters, each `mut self -> Self` storing one Option field. The C setter of the same name stores the same field on RbChoiceBuilder. The C builds RbFullChoice inline instead of through the builder, so the setters have no caller. |
| 67 | `ability/types.c` | `rb_choice_builder_filtered_indices` | ChoiceBuilder, filtered_indices | Rust ChoiceBuilder (types.rs:412-553): 20 setters, each `mut self -> Self` storing one Option field. The C setter of the same name stores the same field on RbChoiceBuilder. The C builds RbFullChoice inline instead of through the builder, so the setters have no caller. |
| 68 | `ability/types.c` | `rb_choice_builder_free` | ChoiceBuilder | Rust ChoiceBuilder (types.rs:412-553): 20 setters, each `mut self -> Self` storing one Option field. The C setter of the same name stores the same field on RbChoiceBuilder. The C builds RbFullChoice inline instead of through the builder, so the setters have no caller. |
| 69 | `ability/types.c` | `rb_choice_builder_group` | ChoiceBuilder | Rust ChoiceBuilder (types.rs:412-553): 20 setters, each `mut self -> Self` storing one Option field. The C setter of the same name stores the same field on RbChoiceBuilder. The C builds RbFullChoice inline instead of through the builder, so the setters have no caller. |
| 70 | `ability/types.c` | `rb_choice_builder_heart_colors` | ChoiceBuilder, heart_colors | Rust ChoiceBuilder (types.rs:412-553): 20 setters, each `mut self -> Self` storing one Option field. The C setter of the same name stores the same field on RbChoiceBuilder. The C builds RbFullChoice inline instead of through the builder, so the setters have no caller. |
| 71 | `ability/types.c` | `rb_choice_builder_is_reveal` | ChoiceBuilder | Rust ChoiceBuilder (types.rs:412-553): 20 setters, each `mut self -> Self` storing one Option field. The C setter of the same name stores the same field on RbChoiceBuilder. The C builds RbFullChoice inline instead of through the builder, so the setters have no caller. |
| 72 | `ability/types.c` | `rb_choice_builder_is_select_action` | ChoiceBuilder, is_select_action, select_action | Rust ChoiceBuilder (types.rs:412-553): 20 setters, each `mut self -> Self` storing one Option field. The C setter of the same name stores the same field on RbChoiceBuilder. The C builds RbFullChoice inline instead of through the builder, so the setters have no caller. |
| 73 | `ability/types.c` | `rb_choice_builder_name_fragments` | ChoiceBuilder, name_fragments | Rust ChoiceBuilder (types.rs:412-553): 20 setters, each `mut self -> Self` storing one Option field. The C setter of the same name stores the same field on RbChoiceBuilder. The C builds RbFullChoice inline instead of through the builder, so the setters have no caller. |
| 74 | `ability/types.c` | `rb_choice_builder_require_all_heart_colors` | ChoiceBuilder, heart_colors, require_all_heart_colors | Rust ChoiceBuilder (types.rs:412-553): 20 setters, each `mut self -> Self` storing one Option field. The C setter of the same name stores the same field on RbChoiceBuilder. The C builds RbFullChoice inline instead of through the builder, so the setters have no caller. |
| 75 | `ability/types.c` | `rb_choice_builder_target_player_id` | ChoiceBuilder, TargetPlayer, target_player | Rust ChoiceBuilder (types.rs:412-553): 20 setters, each `mut self -> Self` storing one Option field. The C setter of the same name stores the same field on RbChoiceBuilder. The C builds RbFullChoice inline instead of through the builder, so the setters have no caller. |
| 76 | `ability/types.c` | `rb_choice_result_free` | ChoiceResult | Rust ChoiceResult (types.rs:325): a 3-variant enum (CardSelected/Skip/Target). The C mirrors it as tag+payload; the four constructors and the three accessors read the same variant. |
| 77 | `ability/types.c` | `rb_choice_result_is_skip` | ChoiceResult | Rust ChoiceResult (types.rs:325): a 3-variant enum (CardSelected/Skip/Target). The C mirrors it as tag+payload; the four constructors and the three accessors read the same variant. |
| 78 | `ability/types.c` | `rb_choice_result_kind` | ChoiceResult | Rust ChoiceResult (types.rs:325): a 3-variant enum (CardSelected/Skip/Target). The C mirrors it as tag+payload; the four constructors and the three accessors read the same variant. |
| 79 | `ability/types.c` | `rb_choice_result_kind_from_str` | ChoiceResult, from_str | Rust ChoiceResult (types.rs:325): a 3-variant enum (CardSelected/Skip/Target). The C mirrors it as tag+payload; the four constructors and the three accessors read the same variant. |
| 80 | `ability/types.c` | `rb_choice_result_kind_to_str` | ChoiceResult | Rust ChoiceResult (types.rs:325): a 3-variant enum (CardSelected/Skip/Target). The C mirrors it as tag+payload; the four constructors and the three accessors read the same variant. |
| 81 | `ability/types.c` | `rb_choice_result_new_card_selected` | ChoiceResult | Rust ChoiceResult (types.rs:325): a 3-variant enum (CardSelected/Skip/Target). The C mirrors it as tag+payload; the four constructors and the three accessors read the same variant. |
| 82 | `ability/types.c` | `rb_choice_result_new_skip` | ChoiceResult | Rust ChoiceResult (types.rs:325): a 3-variant enum (CardSelected/Skip/Target). The C mirrors it as tag+payload; the four constructors and the three accessors read the same variant. |
| 83 | `ability/types.c` | `rb_choice_result_new_target` | ChoiceResult | Rust ChoiceResult (types.rs:325): a 3-variant enum (CardSelected/Skip/Target). The C mirrors it as tag+payload; the four constructors and the three accessors read the same variant. |
| 84 | `ability/types.c` | `rb_choice_route_from_kind` | ChoiceRoute | Rust ChoiceRoute (types.rs:55) + its Display impl (types.rs:66). The C's to_str/from_kind/from_str/kind_from_header round-trip the same four route tags. |
| 85 | `ability/types.c` | `rb_choice_route_kind_from_header` | ChoiceRoute | Rust ChoiceRoute (types.rs:55) + its Display impl (types.rs:66). The C's to_str/from_kind/from_str/kind_from_header round-trip the same four route tags. |
| 86 | `ability/types.c` | `rb_choice_route_new` | ChoiceRoute | Rust ChoiceRoute (types.rs:55) + its Display impl (types.rs:66). The C's to_str/from_kind/from_str/kind_from_header round-trip the same four route tags. |
| 87 | `ability/types.c` | `rb_choice_route_to_str` | ChoiceRoute | Rust ChoiceRoute (types.rs:55) + its Display impl (types.rs:66). The C's to_str/from_kind/from_str/kind_from_header round-trip the same four route tags. |
| 88 | `ability/types.c` | `rb_effect_pipeline_free` | EffectPipeline | Rust EffectPipeline (types.rs:1062-1094): new() + Default. The C's new/free is the same empty container. |
| 89 | `ability/types.c` | `rb_effect_pipeline_new` | EffectPipeline | Rust EffectPipeline (types.rs:1062-1094): new() + Default. The C's new/free is the same empty container. |
| 90 | `ability/types.c` | `rb_effect_spawn_context_init` | EffectSpawnContext | Rust EffectSpawnContext (types.rs:887-901), reached from ability/resolver.rs and core/game_state/abilities.rs. The C initialises the same struct in place. |
| 91 | `ability/types.c` | `rb_exec_context_kind_from_str` | from_str | Rust ExecutionContext (types.rs:342), 78 mentions across 17 Rust files. to_str/from_str are the same tag table. |
| 92 | `ability/types.c` | `rb_exec_context_kind_to_str` | -- | Rust ExecutionContext (types.rs:342), 78 mentions across 17 Rust files. to_str/from_str are the same tag table. |
| 93 | `ability/types.c` | `rb_full_choice_allow_skip` | allow_skip | Rust Choice (types.rs:94-302): allow_skip/description_ja/to_frontend_json. The C's accessors read the same fields off RbFullChoice; to_header is the C's wire downgrade. |
| 94 | `ability/types.c` | `rb_full_choice_description_ja` | -- | Rust Choice (types.rs:94-302): allow_skip/description_ja/to_frontend_json. The C's accessors read the same fields off RbFullChoice; to_header is the C's wire downgrade. |
| 95 | `ability/types.c` | `rb_full_choice_free` | -- | Rust Choice (types.rs:94-302): allow_skip/description_ja/to_frontend_json. The C's accessors read the same fields off RbFullChoice; to_header is the C's wire downgrade. |
| 96 | `ability/types.c` | `rb_full_choice_set_aa_options` | -- | Rust Choice (types.rs:94-302): allow_skip/description_ja/to_frontend_json. The C's accessors read the same fields off RbFullChoice; to_header is the C's wire downgrade. |
| 97 | `ability/types.c` | `rb_full_choice_set_ls_options` | -- | Rust Choice (types.rs:94-302): allow_skip/description_ja/to_frontend_json. The C's accessors read the same fields off RbFullChoice; to_header is the C's wire downgrade. |
| 98 | `ability/types.c` | `rb_full_choice_to_header` | -- | Rust Choice (types.rs:94-302): allow_skip/description_ja/to_frontend_json. The C's accessors read the same fields off RbFullChoice; to_header is the C's wire downgrade. |
| 99 | `ability/types.c` | `rb_full_choice_to_json` | -- | Rust Choice (types.rs:94-302): allow_skip/description_ja/to_frontend_json. The C's accessors read the same fields off RbFullChoice; to_header is the C's wire downgrade. |
| 100 | `ability/types.c` | `rb_las_kind_from_str` | from_str | Rust LookAndSelectStep (types.rs:394). to_str/from_str are the same tag table. |
| 101 | `ability/types.c` | `rb_las_kind_to_str` | -- | Rust LookAndSelectStep (types.rs:394). to_str/from_str are the same tag table. |
| 102 | `ability/types.c` | `rb_look_and_select_step_finalize` | LookAndSelectStep | Rust LookAndSelectStep (types.rs:394-410), used by ability/choice.rs, ability/effects/look.rs and ability/effects/move/mod.rs. The C's look/select/finalize drive the same three sub-steps. |
| 103 | `ability/types.c` | `rb_look_and_select_step_look` | LookAndSelectStep | Rust LookAndSelectStep (types.rs:394-410), used by ability/choice.rs, ability/effects/look.rs and ability/effects/move/mod.rs. The C's look/select/finalize drive the same three sub-steps. |
| 104 | `ability/types.c` | `rb_look_and_select_step_select` | LookAndSelectStep | Rust LookAndSelectStep (types.rs:394-410), used by ability/choice.rs, ability/effects/look.rs and ability/effects/move/mod.rs. The C's look/select/finalize drive the same three sub-steps. |
| 105 | `ability/types.c` | `rb_repeat_prompt_choice` | choice_prompt, repeat_prompt_choice | Rust types.rs:36 `pub fn repeat_prompt_choice() -> Choice`. The C returns the same prompt. |
| 106 | `ability/types.c` | `rb_step_output_accepted` | StepOutput | Rust StepOutput (types.rs:903-935): cards/value/accepted. from_value/merge/value/accepted/has_cards/to_json read and write the same three fields. |
| 107 | `ability/types.c` | `rb_step_output_from_value` | StepOutput, from_value | Rust StepOutput (types.rs:903-935): cards/value/accepted. from_value/merge/value/accepted/has_cards/to_json read and write the same three fields. |
| 108 | `ability/types.c` | `rb_step_output_has_cards` | StepOutput | Rust StepOutput (types.rs:903-935): cards/value/accepted. from_value/merge/value/accepted/has_cards/to_json read and write the same three fields. |
| 109 | `ability/types.c` | `rb_step_output_init` | StepOutput | Rust StepOutput (types.rs:903-935): cards/value/accepted. from_value/merge/value/accepted/has_cards/to_json read and write the same three fields. |
| 110 | `ability/types.c` | `rb_step_output_to_json` | StepOutput | Rust StepOutput (types.rs:903-935): cards/value/accepted. from_value/merge/value/accepted/has_cards/to_json read and write the same three fields. |
| 111 | `ability/types.c` | `rb_step_output_value` | StepOutput | Rust StepOutput (types.rs:903-935): cards/value/accepted. from_value/merge/value/accepted/has_cards/to_json read and write the same three fields. |
| 112 | `ability/types.c` | `rb_step_state_free` | StepState | Rust StepState (types.rs:1096-1137): new/record/get/clear. The C's init/new/record/get/clear are the same map over the same key. |
| 113 | `ability/types.c` | `rb_step_state_get` | StepState, get_state, get_to_state | Rust StepState (types.rs:1096-1137): new/record/get/clear. The C's init/new/record/get/clear are the same map over the same key. |
| 114 | `ability/types.c` | `rb_step_state_new` | StepState | Rust StepState (types.rs:1096-1137): new/record/get/clear. The C's init/new/record/get/clear are the same map over the same key. |
| 115 | `ability/types.c` | `rb_step_state_record_value` | StepState, state_value | Rust StepState (types.rs:1096-1137): new/record/get/clear. The C's init/new/record/get/clear are the same map over the same key. |
| 116 | `ability/types.c` | `rb_trace_node_add_child` | -- | Rust AbilityTraceNode (types.rs:1025-1060) + ZoneSnapshot (types.rs:993-1023), used by ability/compound/setup.rs and core/game_state/mod.rs. The C's with_card/with_before/with_after/add_child/from_game_state build the same tree. |
| 117 | `ability/types.c` | `rb_trace_node_from_game_state` | GameState, GameTrace, from_game_state | Rust AbilityTraceNode (types.rs:1025-1060) + ZoneSnapshot (types.rs:993-1023), used by ability/compound/setup.rs and core/game_state/mod.rs. The C's with_card/with_before/with_after/add_child/from_game_state build the same tree. |
| 118 | `ability/types.c` | `rb_trace_node_with_after` | -- | Rust AbilityTraceNode (types.rs:1025-1060) + ZoneSnapshot (types.rs:993-1023), used by ability/compound/setup.rs and core/game_state/mod.rs. The C's with_card/with_before/with_after/add_child/from_game_state build the same tree. |
| 119 | `ability/types.c` | `rb_trace_node_with_card` | with_card | Rust AbilityTraceNode (types.rs:1025-1060) + ZoneSnapshot (types.rs:993-1023), used by ability/compound/setup.rs and core/game_state/mod.rs. The C's with_card/with_before/with_after/add_child/from_game_state build the same tree. |
| 120 | `ability/types.c` | `rb_trigger_event_add_appeared` | TriggerEvent | Rust TriggerEvent (core/types.rs / core/card.rs): a bitflag set with has_trigger. The C's has_* read the same bits and add_*/copy/init set the same ones. |
| 121 | `ability/types.c` | `rb_trigger_event_add_moved` | TriggerEvent | Rust TriggerEvent (core/types.rs / core/card.rs): a bitflag set with has_trigger. The C's has_* read the same bits and add_*/copy/init set the same ones. |
| 122 | `ability/types.c` | `rb_trigger_event_copy` | TriggerEvent | Rust TriggerEvent (core/types.rs / core/card.rs): a bitflag set with has_trigger. The C's has_* read the same bits and add_*/copy/init set the same ones. |
| 123 | `ability/types.c` | `rb_trigger_event_has_appeared` | TriggerEvent, has_trigger | Rust TriggerEvent (core/types.rs / core/card.rs): a bitflag set with has_trigger. The C's has_* read the same bits and add_*/copy/init set the same ones. |
| 124 | `ability/types.c` | `rb_trigger_event_has_energy_placed` | TriggerEvent, has_trigger | Rust TriggerEvent (core/types.rs / core/card.rs): a bitflag set with has_trigger. The C's has_* read the same bits and add_*/copy/init set the same ones. |
| 125 | `ability/types.c` | `rb_trigger_event_has_moved` | TriggerEvent, has_trigger | Rust TriggerEvent (core/types.rs / core/card.rs): a bitflag set with has_trigger. The C's has_* read the same bits and add_*/copy/init set the same ones. |
| 126 | `ability/types.c` | `rb_trigger_event_has_position_change` | PositionChangeEvent, TriggerEvent, has_trigger | Rust TriggerEvent (core/types.rs / core/card.rs): a bitflag set with has_trigger. The C's has_* read the same bits and add_*/copy/init set the same ones. |
| 127 | `ability/types.c` | `rb_trigger_event_init` | TriggerEvent | Rust TriggerEvent (core/types.rs / core/card.rs): a bitflag set with has_trigger. The C's has_* read the same bits and add_*/copy/init set the same ones. |
| 128 | `ability/types.c` | `rb_value_ref_merge` | ValueRef | Rust-dead. `ValueRef` occurs in exactly one Rust file (engine/src/ability/types.rs); resolve/StepValue/StepAccepted/StepValueOffset have no caller anywhere in engine/src or engine/tests. The C's whole ValueRef family is test-only machinery Rust does not have. |
| 129 | `ability/util.c` | `rb_card_matches_cost_limit_op` | card_cost, card_matches_cost_limit, card_matches_cost_limit_op | One-line forwarders to the live rb_card_matches_cost_limit / rb_has_cannot_baton_touch_protection / rb_zone_cards. Identical by construction. |
| 130 | `ability/util.c` | `rb_exclude_self` | exclude_self | NOT A TWIN. Rust's exclude_self (ability/util.rs:1446-1449) is a CardFilter BUILDER method that stores `self.exclude_self = Some(id)`. The C's returns the COUNT of non-matching entries. A name collision, not a port. |
| 131 | `ability/util.c` | `rb_has_cannot_baton_touch_protection_util` | BatonTouchProtection, has_cannot_baton_touch_protection | One-line forwarders to the live rb_card_matches_cost_limit / rb_has_cannot_baton_touch_protection / rb_zone_cards. Identical by construction. |
| 132 | `ability/util.c` | `rb_prune_dominated` | prune_dominated | Rust util.rs prune_dominated. Same qsort + subset-dominance filter over a u64 mask set. |
| 133 | `ability/util.c` | `rb_set_card_identity` | card_identity | C-only bot identity table (Rust's twin lives in src/bin/bot_arena.rs, a binary the C does not link). |
| 134 | `ability/util.c` | `rb_target_player_label` | TargetPlayer, target_player, target_player_id | Display label; Rust's TargetPlayer (ability/enums.rs) has no display arm. |
| 135 | `ability/util.c` | `rb_zone_card_ids` | zone_card_ids | One-line forwarders to the live rb_card_matches_cost_limit / rb_has_cannot_baton_touch_protection / rb_zone_cards. Identical by construction. |
| 136 | `ability/vm.c` | `build_filter` | build_filter | Returns a Condition whose variant is RB_COND_ABILITY_FILTER and copies the common fields. Rust builds the same variant through its condition parser. |
| 137 | `ability/vm.c` | `offset_of` | -- | C-local helpers of the bytecode decoder (offsetof shim and a generic JSON field filler). No Rust twin. |
| 138 | `ability/vm.c` | `populate_from_json` | populate_from_json | C-local helpers of the bytecode decoder (offsetof shim and a generic JSON field filler). No Rust twin. |
| 139 | `ability/vm.c` | `rb_probe_dropped_key` | -- | A C-local probe/dropped-key instrumentation ring. Rust has no probe ring (no name candidate in engine/src), so there is no twin to diverge from. The instrumentation itself is reported. |
| 140 | `ability/vm.c` | `rb_probe_dropped_tag` | -- | A C-local probe/dropped-key instrumentation ring. Rust has no probe ring (no name candidate in engine/src), so there is no twin to diverge from. The instrumentation itself is reported. |
| 141 | `ability/vm.c` | `rb_probe_key` | -- | A C-local probe/dropped-key instrumentation ring. Rust has no probe ring (no name candidate in engine/src), so there is no twin to diverge from. The instrumentation itself is reported. |
| 142 | `ability/vm.c` | `rb_probe_keycount` | -- | A C-local probe/dropped-key instrumentation ring. Rust has no probe ring (no name candidate in engine/src), so there is no twin to diverge from. The instrumentation itself is reported. |
| 143 | `ability/vm.c` | `rb_probe_ndropped_keys` | -- | A C-local probe/dropped-key instrumentation ring. Rust has no probe ring (no name candidate in engine/src), so there is no twin to diverge from. The instrumentation itself is reported. |
| 144 | `ability/vm.c` | `rb_probe_nkeys` | -- | A C-local probe/dropped-key instrumentation ring. Rust has no probe ring (no name candidate in engine/src), so there is no twin to diverge from. The instrumentation itself is reported. |
| 145 | `ability/vm.c` | `rb_probe_reset` | -- | A C-local probe/dropped-key instrumentation ring. Rust has no probe ring (no name candidate in engine/src), so there is no twin to diverge from. The instrumentation itself is reported. |
| 146 | `ability/vm.c` | `rb_zone_as_str` | -- | C-local zone tag helper. |
| 147 | `ability/vm.c` | `rd_i64` | -- | C-local little readers for the debug dump path. No Rust twin. |
| 148 | `ability/vm.c` | `rd_key` | -- | C-local little readers for the debug dump path. No Rust twin. |
| 149 | `core/card.c` | `rb_ability_filter_from_str` | AbilityFilter, from_str | Enum <-> wire-tag tables. Each was read against the Rust enum plus its as_str/from_str at card.rs:2688-3028 (CardState 2688-2711, ComparisonTarget 2718-2743, ComparisonType 2924, CardProperty 2750-2788, PlacementOrder 2792-2807, DistinctType 2809-2830, AbilityFilter 2968-2997, ConditionTarget 3006-3028, CardType 50-65). Variant counts, order and the default arm all match. |
| 150 | `core/card.c` | `rb_ability_filter_str` | AbilityFilter | Enum <-> wire-tag tables. Each was read against the Rust enum plus its as_str/from_str at card.rs:2688-3028 (CardState 2688-2711, ComparisonTarget 2718-2743, ComparisonType 2924, CardProperty 2750-2788, PlacementOrder 2792-2807, DistinctType 2809-2830, AbilityFilter 2968-2997, ConditionTarget 3006-3028, CardType 50-65). Variant counts, order and the default arm all match. |
| 151 | `core/card.c` | `rb_card_state_from_str` | CardState, as_card_str, from_card_str | Enum <-> wire-tag tables. Each was read against the Rust enum plus its as_str/from_str at card.rs:2688-3028 (CardState 2688-2711, ComparisonTarget 2718-2743, ComparisonType 2924, CardProperty 2750-2788, PlacementOrder 2792-2807, DistinctType 2809-2830, AbilityFilter 2968-2997, ConditionTarget 3006-3028, CardType 50-65). Variant counts, order and the default arm all match. |
| 152 | `core/card.c` | `rb_card_state_str` | CardState, as_card_str | Enum <-> wire-tag tables. Each was read against the Rust enum plus its as_str/from_str at card.rs:2688-3028 (CardState 2688-2711, ComparisonTarget 2718-2743, ComparisonType 2924, CardProperty 2750-2788, PlacementOrder 2792-2807, DistinctType 2809-2830, AbilityFilter 2968-2997, ConditionTarget 3006-3028, CardType 50-65). Variant counts, order and the default arm all match. |
| 153 | `core/card.c` | `rb_comparison_target_from_str` | ComparisonTarget, from_str | Enum <-> wire-tag tables. Each was read against the Rust enum plus its as_str/from_str at card.rs:2688-3028 (CardState 2688-2711, ComparisonTarget 2718-2743, ComparisonType 2924, CardProperty 2750-2788, PlacementOrder 2792-2807, DistinctType 2809-2830, AbilityFilter 2968-2997, ConditionTarget 3006-3028, CardType 50-65). Variant counts, order and the default arm all match. |
| 154 | `core/card.c` | `rb_comparison_target_str` | ComparisonTarget | Enum <-> wire-tag tables. Each was read against the Rust enum plus its as_str/from_str at card.rs:2688-3028 (CardState 2688-2711, ComparisonTarget 2718-2743, ComparisonType 2924, CardProperty 2750-2788, PlacementOrder 2792-2807, DistinctType 2809-2830, AbilityFilter 2968-2997, ConditionTarget 3006-3028, CardType 50-65). Variant counts, order and the default arm all match. |
| 155 | `core/card.c` | `rb_comparison_type_from_str` | ComparisonType, from_str | Enum <-> wire-tag tables. Each was read against the Rust enum plus its as_str/from_str at card.rs:2688-3028 (CardState 2688-2711, ComparisonTarget 2718-2743, ComparisonType 2924, CardProperty 2750-2788, PlacementOrder 2792-2807, DistinctType 2809-2830, AbilityFilter 2968-2997, ConditionTarget 3006-3028, CardType 50-65). Variant counts, order and the default arm all match. |
| 156 | `core/card.c` | `rb_comparison_type_str` | ComparisonType | Enum <-> wire-tag tables. Each was read against the Rust enum plus its as_str/from_str at card.rs:2688-3028 (CardState 2688-2711, ComparisonTarget 2718-2743, ComparisonType 2924, CardProperty 2750-2788, PlacementOrder 2792-2807, DistinctType 2809-2830, AbilityFilter 2968-2997, ConditionTarget 3006-3028, CardType 50-65). Variant counts, order and the default arm all match. |
| 157 | `core/card.c` | `rb_condition_card_type_str` | CardType, ConditionCardType, ConditionType | Enum <-> wire-tag tables. Each was read against the Rust enum plus its as_str/from_str at card.rs:2688-3028 (CardState 2688-2711, ComparisonTarget 2718-2743, ComparisonType 2924, CardProperty 2750-2788, PlacementOrder 2792-2807, DistinctType 2809-2830, AbilityFilter 2968-2997, ConditionTarget 3006-3028, CardType 50-65). Variant counts, order and the default arm all match. |
| 158 | `core/card.c` | `rb_condition_target_str` | ConditionTarget | Enum <-> wire-tag tables. Each was read against the Rust enum plus its as_str/from_str at card.rs:2688-3028 (CardState 2688-2711, ComparisonTarget 2718-2743, ComparisonType 2924, CardProperty 2750-2788, PlacementOrder 2792-2807, DistinctType 2809-2830, AbilityFilter 2968-2997, ConditionTarget 3006-3028, CardType 50-65). Variant counts, order and the default arm all match. |
| 159 | `core/card.c` | `rb_distinct_type_str` | DistinctType | Enum <-> wire-tag tables. Each was read against the Rust enum plus its as_str/from_str at card.rs:2688-3028 (CardState 2688-2711, ComparisonTarget 2718-2743, ComparisonType 2924, CardProperty 2750-2788, PlacementOrder 2792-2807, DistinctType 2809-2830, AbilityFilter 2968-2997, ConditionTarget 3006-3028, CardType 50-65). Variant counts, order and the default arm all match. |
| 160 | `core/card.c` | `rb_location_str` | -- | No Rust item. Rust has no Location enum -- zones are bare string literals matched in place (ability/describe.rs:28-58, ability/condition.rs:273-279). C-local display table. |
| 161 | `core/card.c` | `rb_parse_operation` | parse_operation | Rust parse_operation (card.rs:2907-2918): 7 names in declaration order, _ -> None. The C returns the index or -1 for None. |
| 162 | `core/card.c` | `rb_placement_order_str` | PlacementOrder | Enum <-> wire-tag tables. Each was read against the Rust enum plus its as_str/from_str at card.rs:2688-3028 (CardState 2688-2711, ComparisonTarget 2718-2743, ComparisonType 2924, CardProperty 2750-2788, PlacementOrder 2792-2807, DistinctType 2809-2830, AbilityFilter 2968-2997, ConditionTarget 3006-3028, CardType 50-65). Variant counts, order and the default arm all match. |
| 163 | `core/data.c` | `rb_effect_data_amount` | EffectData | Rust EffectData accessors (core/types.rs:327-399): card_id, items, is_p1, old_value, count, color, amount. Each C switch covers exactly the variants the Rust match does, with _ => the Rust fallback (None / empty vec). |
| 164 | `core/data.c` | `rb_effect_data_card_id` | EffectData | Rust EffectData accessors (core/types.rs:327-399): card_id, items, is_p1, old_value, count, color, amount. Each C switch covers exactly the variants the Rust match does, with _ => the Rust fallback (None / empty vec). |
| 165 | `core/data.c` | `rb_effect_data_clone` | EffectData | Rust EffectData accessors (core/types.rs:327-399): card_id, items, is_p1, old_value, count, color, amount. Each C switch covers exactly the variants the Rust match does, with _ => the Rust fallback (None / empty vec). |
| 166 | `core/data.c` | `rb_effect_data_color` | EffectData | Rust EffectData accessors (core/types.rs:327-399): card_id, items, is_p1, old_value, count, color, amount. Each C switch covers exactly the variants the Rust match does, with _ => the Rust fallback (None / empty vec). |
| 167 | `core/data.c` | `rb_effect_data_count` | EffectData | Rust EffectData accessors (core/types.rs:327-399): card_id, items, is_p1, old_value, count, color, amount. Each C switch covers exactly the variants the Rust match does, with _ => the Rust fallback (None / empty vec). |
| 168 | `core/data.c` | `rb_effect_data_is_p1` | EffectData | Rust EffectData accessors (core/types.rs:327-399): card_id, items, is_p1, old_value, count, color, amount. Each C switch covers exactly the variants the Rust match does, with _ => the Rust fallback (None / empty vec). |
| 169 | `core/data.c` | `rb_effect_data_items` | EffectData | Rust EffectData accessors (core/types.rs:327-399): card_id, items, is_p1, old_value, count, color, amount. Each C switch covers exactly the variants the Rust match does, with _ => the Rust fallback (None / empty vec). |
| 170 | `core/data.c` | `rb_effect_data_old_value` | EffectData, old_value | Rust EffectData accessors (core/types.rs:327-399): card_id, items, is_p1, old_value, count, color, amount. Each C switch covers exactly the variants the Rust match does, with _ => the Rust fallback (None / empty vec). |
| 171 | `core/data.c` | `rb_effect_type_as_str` | EffectType, effect_type | Rust EffectType::as_str (core/types.rs:1033-1048): 11 variants, verbatim order. |
| 172 | `core/data.c` | `rb_phase_display` | -- | No Rust item of that name; the phase labels live as match arms in ability/describe.rs. C-local display table. |
| 173 | `core/data.c` | `rb_phase_label_jp` | -- | Same as rb_phase_display. |
| 174 | `core/data.c` | `rb_turn_phase_display` | TurnPhase | Rust TurnPhase (core/types.rs). 3 variants, same tags. |
| 175 | `core/data.c` | `rb_zone_matches_dest` | matches_dest | Rust ZoneId::matches_dest (core/types.rs:909-911) is literally `self.equivalent(&ZoneId::from_str(dest))`, which is the C body. |
| 176 | `core/zones.c` | `rb_deck_is_empty` | -- | Rust CardZone (zones.rs:800-860: is_empty/len/shuffle/draw/draw_bottom/take_all). The C's helpers read and write the same bag. Rust has no separate waitroom zone (discard IS the waitroom), which is why the C's rb_waitroom_* read P->discard. |
| 177 | `core/zones.c` | `rb_deck_len` | -- | Rust CardZone (zones.rs:800-860: is_empty/len/shuffle/draw/draw_bottom/take_all). The C's helpers read and write the same bag. Rust has no separate waitroom zone (discard IS the waitroom), which is why the C's rb_waitroom_* read P->discard. |
| 178 | `core/zones.c` | `rb_deck_shuffle` | -- | Rust CardZone (zones.rs:800-860: is_empty/len/shuffle/draw/draw_bottom/take_all). The C's helpers read and write the same bag. Rust has no separate waitroom zone (discard IS the waitroom), which is why the C's rb_waitroom_* read P->discard. |
| 179 | `core/zones.c` | `rb_energy_can_place_card` | can_place_card | Rust EnergyZone::can_place_card (zones.rs:650-656): get_card().map(is_energy).unwrap_or(false). Identical. |
| 180 | `core/zones.c` | `rb_hand_is_empty` | -- | Rust CardZone (zones.rs:800-860: is_empty/len/shuffle/draw/draw_bottom/take_all). The C's helpers read and write the same bag. Rust has no separate waitroom zone (discard IS the waitroom), which is why the C's rb_waitroom_* read P->discard. |
| 181 | `core/zones.c` | `rb_hand_len` | -- | Rust CardZone (zones.rs:800-860: is_empty/len/shuffle/draw/draw_bottom/take_all). The C's helpers read and write the same bag. Rust has no separate waitroom zone (discard IS the waitroom), which is why the C's rb_waitroom_* read P->discard. |
| 182 | `core/zones.c` | `rb_live_clear` | -- | Rust CardZone (zones.rs:800-860: is_empty/len/shuffle/draw/draw_bottom/take_all). The C's helpers read and write the same bag. Rust has no separate waitroom zone (discard IS the waitroom), which is why the C's rb_waitroom_* read P->discard. |
| 183 | `core/zones.c` | `rb_player_activate_all_energy` | activate_all, activate_all_energy | Rust Player::activate_all_energy (player.rs). The C delegates to rb_energy_activate_all, which sets energy_active = energy.n. |
| 184 | `core/zones.c` | `rb_player_activate_all_energy_exclude` | activate_all, activate_all_energy, activate_all_energy_exclude | Rust player.rs:549-552 saturating_sub on a u8 count. The C's `active = total - excluded; if (<0) 0` is the same saturating_sub. |
| 185 | `core/zones.c` | `rb_player_all_card_ids` | all_card_ids | Rust Player::all_card_ids (player.rs). Same nine zones in the same order. |
| 186 | `core/zones.c` | `rb_player_calculate_stage_hearts` | calculate_stage_hearts, stage_hearts, stage_hearts_of | Both delegate to rb_stage_hearts_pipeline, which is the live port of StageZone::get_available_hearts (zones.rs:468). Thin wrappers over the live pipeline. |
| 187 | `core/zones.c` | `rb_player_contains_card` | contains_card | Rust Player::contains_card (player.rs). Same nine zones. |
| 188 | `core/zones.c` | `rb_player_get_card_index_by_id` | get_card, get_card_by_no, get_card_id | Rust scans the hand for a card id. The C scans the hand only; same shape. |
| 189 | `core/zones.c` | `rb_player_is_area_locked` | is_area_locked | Rust Player::is_area_locked (player.rs) reads stage_arrived[player][area]. Identical. |
| 190 | `core/zones.c` | `rb_player_set_main_deck` | MainDeck, set_deck, set_main_deck | Rust Player::set_main_deck (zones.rs) replaces the deck contents, bounded by capacity. Identical. |
| 191 | `core/zones.c` | `rb_resolution_swap_slots` | swap_slots | Rust swaps the slot AND its owner. The C swaps g->resolution.cards and g->resolution_owners together. |
| 192 | `core/zones.c` | `rb_stage_get_available_hearts` | get_available_hearts, stage_hearts, stage_hearts_of | Both delegate to rb_stage_hearts_pipeline, which is the live port of StageZone::get_available_hearts (zones.rs:468). Thin wrappers over the live pipeline. |
| 193 | `core/zones.c` | `rb_success_len` | -- | Rust CardZone (zones.rs:800-860: is_empty/len/shuffle/draw/draw_bottom/take_all). The C's helpers read and write the same bag. Rust has no separate waitroom zone (discard IS the waitroom), which is why the C's rb_waitroom_* read P->discard. |
| 194 | `core/zones.c` | `rb_waitroom_len` | -- | Rust CardZone (zones.rs:800-860: is_empty/len/shuffle/draw/draw_bottom/take_all). The C's helpers read and write the same bag. Rust has no separate waitroom zone (discard IS the waitroom), which is why the C's rb_waitroom_* read P->discard. |
| 195 | `core/zones.c` | `rb_waitroom_shuffle` | -- | Rust CardZone (zones.rs:800-860: is_empty/len/shuffle/draw/draw_bottom/take_all). The C's helpers read and write the same bag. Rust has no separate waitroom zone (discard IS the waitroom), which is why the C's rb_waitroom_* read P->discard. |
| 196 | `core/zones.c` | `rb_waitroom_take_all` | take_all | Rust CardZone (zones.rs:800-860: is_empty/len/shuffle/draw/draw_bottom/take_all). The C's helpers read and write the same bag. Rust has no separate waitroom zone (discard IS the waitroom), which is why the C's rb_waitroom_* read P->discard. |
| 197 | `core/zones.c` | `rb_zone_draw_bottom` | draw_bottom | Rust CardZone::draw_bottom (zones.rs:823-832): empty -> warn-once + None, else pop(). The C returns -1 / cards[--n]. Both pop the TOP despite the name; the missing warn-once is a log, not behaviour. |
| 198 | `core/zones.c` | `rb_zone_draw_multiple` | draw_multiple | Rust (0..count).filter_map(\|_\| self.draw()).collect() (zones.rs:834) stops adding after an empty draw without breaking; the C breaks. Same set, unobservable. |
| 199 | `core/zones.c` | `rb_zone_total_blades` | total_blades, total_blades_of | Rust StageZone::total_blades (zones.rs:424). Same waited/orientation skip and the same c.blade + mods blade sum. |
| 200 | `engine.c` | `i18n_self_check` | i18n_self_check | No Rust twin. Rust has i18n_self_check in src/main.rs (a BINARY), which the C's rb_engine does not link; there is no caller, so the C never self-checks. |
| 201 | `game/game_setup.c` | `rb_setup_auto_advance_one` | auto_advance_one | Rust game_setup.rs auto_advance_one. Same guard order (pending choice, game ongoing, automatic phase). |
| 202 | `game/game_setup.c` | `rb_setup_execute_action_for_player` | execute_action, execute_action_for_player | Rust harness.rs execute_action_for_player: concede for player_id, else execute_action. Same. |
| 203 | `game/game_setup.c` | `rb_setup_extract_base` | extract_base | Rust deck_ordering.rs extract_base: text before the last '-'. Identical. |
| 204 | `game/game_setup.c` | `rb_setup_is_live_card_set_phase` | is_live_card_set_phase, is_live_phase | Rust game_setup.rs is_live_card_set_phase: the two live-set phases. Identical. |
| 205 | `game/game_setup.c` | `rb_setup_settle_auto` | settle_auto | Rust game_setup.rs settle_auto: bounded loop over the automatic phases. Identical. |
| 206 | `turn/live.c` | `rb_try_all_distribution` | try_all_distribution | DEAD-CORRECT-BY-DOCUMENTATION, upheld. Neither can be written faithfully against its exported signature (neither has a pool, an allocations vector or a deficit list). The FAITHFUL ports exist and are live and file-local: rb_backtrack_allocate -> bt_search -> bt_try_surplus -> bt_try_phase4 -> bt_try_all_distribution. A prior tombstone at live.c:2727-2746 already records this, including the correction that this symbol is NOT in rabuka.h and that Rust's try_all_distribution IS live (live.rs:2754, called at live.rs:2731). Kept as named entry points with the reason on the body. |
| 207 | `turn/live.c` | `rb_try_phase4` | try_phase4 | DEAD-CORRECT-BY-DOCUMENTATION, upheld. Neither can be written faithfully against its exported signature (neither has a pool, an allocations vector or a deficit list). The FAITHFUL ports exist and are live and file-local: rb_backtrack_allocate -> bt_search -> bt_try_surplus -> bt_try_phase4 -> bt_try_all_distribution. A prior tombstone at live.c:2727-2746 already records this, including the correction that this symbol is NOT in rabuka.h and that Rust's try_all_distribution IS live (live.rs:2754, called at live.rs:2731). Kept as named entry points with the reason on the body. |
| 208 | `turn/phase.c` | `_3ds_tdbg` | _3ds_tdbg | Platform logging no-ops. Rust phases.rs has _3ds_tdbg and log_turn_start behind the non-default 3DS/platform feature; in the portable build they are no-ops too. The C's are `(void)` on purpose and the surrounding comment says so. |
| 209 | `turn/phase.c` | `rb_3ds_tdbg` | _3ds_tdbg | Platform logging no-ops. Rust phases.rs has _3ds_tdbg and log_turn_start behind the non-default 3DS/platform feature; in the portable build they are no-ops too. The C's are `(void)` on purpose and the surrounding comment says so. |
| 210 | `turn/phase.c` | `rb_generate_possible_actions` | generate_possible_actions | Rust emits the action list for the menu; the C's action vocabulary is built by src/game/game_setup.c's rb_setup_*_candidates. C-local menu data, no engine effect. |
| 211 | `turn/phase.c` | `rb_log_phase` | log_phase | Platform logging no-ops. Rust phases.rs has _3ds_tdbg and log_turn_start behind the non-default 3DS/platform feature; in the portable build they are no-ops too. The C's are `(void)` on purpose and the surrounding comment says so. |
| 212 | `turn/phase.c` | `rb_log_turn_start` | log_turn_start | Platform logging no-ops. Rust phases.rs has _3ds_tdbg and log_turn_start behind the non-default 3DS/platform feature; in the portable build they are no-ops too. The C's are `(void)` on purpose and the surrounding comment says so. |
| 213 | `turn/phase.c` | `rb_play_time_cost_reduction_hook` | PlayTimeCostPlay, play_time_cost_reduction_hook | C-local hook that returns the armed play-time cost-reduction for a card; the arming and consuming are live in phase.c. Reported. |


---

## 10. Test-named (141) — NOT adjudicated as dead

These have zero callers in `src/` but **are** named in `tests/`, so they are
TEST-ONLY and out of scope: `TEST_ONLY_AUDIT.md` is the authority and was not
redone. Listed only so the 356 in my files reconciles as 219 + 141.
**Watch item:** `src/test_game.c` is in `OBJ_LIB`; 75 of its functions are
product code and were treated as product throughout.

| # | file | symbol | named in |
|---|------|--------|----------|
| 1 | `ability/ability_queue.c` | `rb_queue_dump_state` | test_parity_queue_resume.c |
| 2 | `ability/ability_queue.c` | `rb_queue_has_entry_with_id` | test_parity_queue_resume.c |
| 3 | `ability/ability_queue.c` | `rb_queue_has_pending` | test_parity_per_card2.c |
| 4 | `ability/ability_queue.c` | `rb_queue_has_resolver` | test_p1_helpers.c, test_parity_queue_resume.c |
| 5 | `ability/ability_queue.c` | `rb_queue_init` | test_audit_triggers.c, test_parity_queue_resume.c |
| 6 | `ability/ability_queue.c` | `rb_queue_is_empty` | test_parity_queue_resume.c |
| 7 | `ability/ability_queue.c` | `rb_queue_is_idle` | test_parity_queue_resume.c |
| 8 | `ability/ability_queue.c` | `rb_queue_is_waiting_for_choice` | test_parity_queue_resume.c |
| 9 | `ability/ability_queue.c` | `rb_queue_iter` | test_parity_queue_resume.c |
| 10 | `ability/ability_queue.c` | `rb_queue_len` | test_audit_triggers.c, test_parity_queue_resume.c |
| 11 | `ability/ability_queue.c` | `rb_queue_pause_for_auto_ability_choice` | test_parity_choice_routing.c, test_parity_queue_resume.c |
| 12 | `ability/ability_queue.c` | `rb_queue_pending_entries` | test_parity_queue_resume.c |
| 13 | `ability/ability_queue.c` | `rb_queue_promote_entry` | test_parity_queue_resume.c |
| 14 | `ability/ability_queue.c` | `rb_queue_promote_entry_by_abs` | test_parity_queue_resume.c |
| 15 | `ability/ability_queue.c` | `rb_queue_resume_with_choice` | test_parity_queue_resume.c |
| 16 | `ability/ability_queue.c` | `rb_queue_set_current_entry` | test_parity_queue_resume.c |
| 17 | `ability/ability_queue.c` | `rb_queue_set_resolver` | test_p1_helpers.c, test_parity_queue_resume.c |
| 18 | `ability/ability_queue.c` | `rb_queue_start_next` | test_parity_queue_resume.c |
| 19 | `ability/ability_queue.c` | `rb_queue_take_resolver` | test_p1_helpers.c, test_parity_queue_resume.c |
| 20 | `ability/choice_renderer.c` | `rb_rend_text_frame_draws` | test_choice_renderer.c |
| 21 | `ability/choice_renderer.c` | `rb_rend_text_frame_line` | test_choice_renderer.c |
| 22 | `ability/choice_renderer.c` | `rb_rend_text_free` | test_choice_renderer.c |
| 23 | `ability/choice_renderer.c` | `rb_rend_text_idle` | test_choice_renderer.c |
| 24 | `ability/choice_renderer.c` | `rb_rend_text_line` | test_choice_renderer.c |
| 25 | `ability/choice_renderer.c` | `rb_rend_text_n_draws` | test_choice_renderer.c |
| 26 | `ability/choice_renderer.c` | `rb_rend_text_n_frames` | test_choice_renderer.c |
| 27 | `ability/choice_renderer.c` | `rb_rend_text_n_lines` | test_choice_renderer.c |
| 28 | `ability/choice_renderer.c` | `rb_rend_text_n_presses` | test_choice_renderer.c |
| 29 | `ability/choice_renderer.c` | `rb_rend_text_new` | test_choice_renderer.c |
| 30 | `ability/choice_renderer.c` | `rb_rend_text_overflowed` | test_choice_renderer.c |
| 31 | `ability/choice_renderer.c` | `rb_rend_text_script` | test_choice_renderer.c |
| 32 | `ability/choice_renderer.c` | `rb_rend_text_stuck` | test_choice_renderer.c |
| 33 | `ability/compound.c` | `rb_route_conditional_branch` | test_parity_cost_compound.c |
| 34 | `ability/cost.c` | `rb_validate_cost` | test_parity_cost_compound.c, test_parity_fw_cost_cardtype.c |
| 35 | `ability/debug.c` | `rb_ability_debug_set` | test_cross_player_interaction.c, test_opponent_live_success_flow.c, test_parity_ability_mod.c, test_parity_ability_mod_extra.c, test_parity_abilitymod_conditional.c, test_parity_area_move.c |
| 36 | `ability/describe.c` | `rb_desc_card_type_label` | test_describe_golden.c |
| 37 | `ability/describe.c` | `rb_desc_card_type_label_ja` | test_describe_golden.c |
| 38 | `ability/describe.c` | `rb_desc_cost_en` | test_describe_golden.c |
| 39 | `ability/describe.c` | `rb_desc_cost_ja` | test_describe_golden.c |
| 40 | `ability/describe.c` | `rb_desc_duration_label` | test_describe_golden.c |
| 41 | `ability/describe.c` | `rb_desc_duration_label_ja` | test_describe_golden.c |
| 42 | `ability/describe.c` | `rb_desc_resource_label` | test_describe_golden.c |
| 43 | `ability/describe.c` | `rb_desc_resource_label_ja` | test_describe_golden.c |
| 44 | `ability/describe.c` | `rb_desc_state_verb` | test_describe_golden.c |
| 45 | `ability/describe.c` | `rb_desc_translate_choice_prompt_en_to_ja` | test_describe_golden.c |
| 46 | `ability/describe.c` | `rb_desc_zone_label` | test_describe_golden.c |
| 47 | `ability/describe.c` | `rb_desc_zone_label_ja` | test_describe_golden.c |
| 48 | `ability/effects/rules.c` | `rb_rules_execute_restriction` | test_parity_q68_cannot_live.c |
| 49 | `ability/effects/state.c` | `rb_card_total_hearts` | test_parity_card_identity.c, test_parity_dead_twins.c, test_parity_full_round3.c |
| 50 | `ability/log.c` | `rb_log_push_verdict_child` | test_log_drain.c |
| 51 | `ability/log.c` | `rb_log_set_enabled` | test_audit_actions.c, test_log_drain.c, test_parity_char2.c, test_parity_cost_discount.c, test_parity_executor_dispatch.c, test_parity_required_hearts.c |
| 52 | `ability/resolver.c` | `rb_resolver_card_matches_cost_limit` | test_parity_resolver.c |
| 53 | `ability/resolver.c` | `rb_resolver_card_matches_type` | test_parity_resolver.c |
| 54 | `ability/resolver.c` | `rb_resolver_fmt_card` | test_parity_resolver.c |
| 55 | `ability/resolver.c` | `rb_resolver_fmt_ids` | test_parity_resolver.c |
| 56 | `ability/resolver.c` | `rb_resolver_get_pending_choice` | test_parity_resolver.c |
| 57 | `ability/resolver.c` | `rb_resolver_merge_group_names` | test_parity_resolver.c |
| 58 | `ability/resolver.c` | `rb_resolver_pending_choice` | test_parity_resolver.c |
| 59 | `ability/resolver.c` | `rb_resolver_zone_for_card` | test_parity_resolver.c |
| 60 | `ability/types.c` | `rb_choice_builder_build` | test_parity_target_player.c |
| 61 | `ability/types.c` | `rb_choice_builder_picker` | test_parity_target_player.c |
| 62 | `ability/types.c` | `rb_value_ref_init_accepted` | test_condition_evaluation.c, test_vm_decode.c |
| 63 | `ability/types.c` | `rb_value_ref_init_literal` | test_condition_evaluation.c, test_vm_decode.c |
| 64 | `ability/types.c` | `rb_value_ref_init_offset` | test_condition_evaluation.c, test_vm_decode.c |
| 65 | `ability/types.c` | `rb_value_ref_init_step` | test_condition_evaluation.c, test_vm_decode.c |
| 66 | `ability/types.c` | `rb_value_ref_is_literal` | test_vm_decode.c |
| 67 | `ability/types.c` | `rb_value_ref_kind_from_str` | test_vm_decode.c |
| 68 | `ability/types.c` | `rb_value_ref_kind_str` | test_vm_decode.c |
| 69 | `ability/util.c` | `rb_card_has_blade_heart_strict` | test_parity_blades2.c, test_parity_card_identity.c, test_parity_yell.c, test_ported_generated.c |
| 70 | `ability/util.c` | `rb_card_matches_name_constraint` | test_parity_deploy_recover.c |
| 71 | `ability/util.c` | `rb_get_zone_card_count` | test_audit_zones.c |
| 72 | `ability/vm.c` | `decode_condition_direct` | test_parity_t7_condition_decoder.c |
| 73 | `ability/vm.c` | `rb_count_empty_bytecode_abilities` | test_parity_characterization.c, test_vm_decode.c |
| 74 | `ability/vm.c` | `rb_decode_fallback_abilities` | test_decode_fallback.c, test_parity_characterization.c |
| 75 | `ability/vm.c` | `rb_decode_fallback_count` | audit_cluster_d.c, audit_t7_decoders.c, census_fx.c, test_decode_fallback.c, test_parity_characterization.c, test_parity_full_round3.c |
| 76 | `ability/vm.c` | `rb_decode_keywords` | test_vm_decode.c |
| 77 | `ability/vm.c` | `rb_get_ability` | test_describe_golden.c, test_parity_characterization.c |
| 78 | `core/card.c` | `rb_card_database_create_copy` | test_ported_generated.c |
| 79 | `core/card.c` | `rb_card_effective_energy_cost_total` | test_parity_card_identity.c |
| 80 | `core/card.c` | `rb_card_energy_cost_total` | test_parity_card_identity.c |
| 81 | `core/card.c` | `rb_card_get_card` | test_parity_card_identity.c |
| 82 | `core/card.c` | `rb_card_has_optional_payment` | test_parity_card_identity.c |
| 83 | `core/card.c` | `rb_card_has_trigger` | test_parity_card_identity.c |
| 84 | `core/card.c` | `rb_card_property_from_str` | test_parity_dead_twins.c |
| 85 | `core/card.c` | `rb_card_property_str` | test_parity_dead_twins.c |
| 86 | `core/card.c` | `rb_card_short_label` | test_parity_card_identity.c |
| 87 | `core/card.c` | `rb_card_triggerless_text` | test_parity_card_identity.c |
| 88 | `core/card.c` | `rb_card_type_from_str` | test_parity_card_identity.c |
| 89 | `core/card.c` | `rb_card_type_str` | test_parity_card_identity.c |
| 90 | `core/card.c` | `rb_effect_action_by` | test_ported_generated.c |
| 91 | `core/card.c` | `rb_effect_destination_any` | test_ported_generated.c |
| 92 | `core/card.c` | `rb_effect_is_under_self` | test_ported_generated.c |
| 93 | `core/card.c` | `rb_effect_non_stackable_any` | test_ported_generated.c |
| 94 | `core/card.c` | `rb_effect_options_any` | test_ported_generated.c |
| 95 | `core/card.c` | `rb_effect_per_unit_heart_colors_any` | test_ported_generated.c |
| 96 | `core/card.c` | `rb_effect_picker_any` | test_parity_target_player.c, test_ported_generated.c |
| 97 | `core/card.c` | `rb_heartmap_clear` | test_parity_card_identity.c |
| 98 | `core/card.c` | `rb_heartmap_contains_key` | test_parity_card_identity.c |
| 99 | `core/card.c` | `rb_heartmap_entry_or_default` | test_parity_card_identity.c |
| 100 | `core/card.c` | `rb_heartmap_is_empty` | test_parity_card_identity.c |
| 101 | `core/card.c` | `rb_heartmap_remove` | test_parity_card_identity.c |
| 102 | `core/card.c` | `rb_heartmap_values_sum` | test_parity_card_identity.c |
| 103 | `core/card.c` | `rb_map_series_to_group` | test_parity_card_identity.c, test_parity_gain_delta.c |
| 104 | `core/data.c` | `rb_card_no_scratch_layout_ok` | audit_w_gate.c, test_audit_warnings.c |
| 105 | `core/data.c` | `rb_card_stridx` | test_audit_zones.c |
| 106 | `core/data.c` | `rb_count_u32` | test_describe_golden.c |
| 107 | `core/data.c` | `rb_f32_count` | test_describe_golden.c |
| 108 | `core/data.c` | `rb_floored_cost` | test_describe_golden.c |
| 109 | `core/data.c` | `rb_id_u32` | test_describe_golden.c |
| 110 | `core/data.c` | `rb_overridden_stat` | test_describe_golden.c |
| 111 | `core/data.c` | `rb_score_to_i32` | test_describe_golden.c |
| 112 | `core/data.c` | `rb_score_to_i32_rounded` | test_describe_golden.c |
| 113 | `core/data.c` | `rb_zone_id_from_ability_zone` | test_parity_rules.c, test_zone_filter_parity.c |
| 114 | `core/data.c` | `rb_zone_id_to_ability_zone` | test_parity_rules.c, test_zone_filter_parity.c |
| 115 | `core/zones.c` | `rb_energy_add_active` | test_parity_dead_twins.c |
| 116 | `core/zones.c` | `rb_energy_deck_is_empty` | test_parity_rules.c |
| 117 | `core/zones.c` | `rb_energy_set_active_count` | replay.c, test_parity_ability_watch.c, test_parity_hearts_constants.c, test_parity_jidou_extra.c, test_parity_jidou_life.c, test_parity_jidou_watch.c |
| 118 | `core/zones.c` | `rb_exclusion_add` | test_audit_zones.c |
| 119 | `core/zones.c` | `rb_live_can_place_card` | test_audit_zones.c |
| 120 | `core/zones.c` | `rb_member_area_from_index` | test_parity_zones_position.c |
| 121 | `core/zones.c` | `rb_member_area_from_tag` | test_parity_zones_position.c |
| 122 | `core/zones.c` | `rb_member_area_front` | test_parity_zones_position.c |
| 123 | `core/zones.c` | `rb_member_area_to_str` | test_parity_zones_position.c |
| 124 | `core/zones.c` | `rb_member_area_to_tag` | test_parity_zones_position.c |
| 125 | `core/zones.c` | `rb_player_draw_card` | test_audit_zones.c |
| 126 | `core/zones.c` | `rb_player_draw_energy` | test_parity_dead_twins.c |
| 127 | `core/zones.c` | `rb_resolution_add` | test_parity_rules_conditions.c |
| 128 | `core/zones.c` | `rb_resolution_clear` | test_parity_jidou_extra.c |
| 129 | `core/zones.c` | `rb_resolution_len` | test_parity_rules_conditions.c |
| 130 | `core/zones.c` | `rb_stage_can_place_card` | test_parity_zones_position.c |
| 131 | `core/zones.c` | `rb_stage_get_area` | test_parity_zones_position.c |
| 132 | `core/zones.c` | `rb_stage_position_change` | test_parity_under_member.c, test_parity_zones_position.c |
| 133 | `core/zones.c` | `rb_stage_set_area` | test_parity_zones_position.c |
| 134 | `core/zones.c` | `rb_zone_invariant` | test_audit_zones.c |
| 135 | `core/zones.c` | `rb_zone_refresh` | test_audit_zones.c |
| 136 | `engine.c` | `rb_execute_effect` | replay.c, test_parity_int_gaps.c |
| 137 | `engine.c` | `rb_lcg_new` | replay.c |
| 138 | `engine.c` | `rb_lcg_range` | replay.c |
| 139 | `engine.c` | `rb_rng_checkpoint` | replay.c |
| 140 | `engine.c` | `rb_rng_restore` | replay.c |
| 141 | `turn/live.c` | `rb_calc_stage_hearts` | replay.c, test_parity_blade_under.c, test_parity_characterization_extra.c, test_parity_compound_per_card.c, test_parity_full_round3.c, test_parity_hearts_constants.c |


---

## 11. Hygiene

| item | state |
|---|---|
| `sizeof(GameState)` | **1 090 264 B**, untouched. No field added; `RB_GAMESTATE_BYTES_PINNED` and `Makefile:3` not moved. The one `GameState` local in the new suite is `static`. |
| `src/abilities_strings.bin` | **726 831 B / `89204df0bbde08a02a7f978a56a80d6e6431691ec7d6d658979a329956b06344`** — byte-identical to the pin. No generator run. |
| `cards/build/*` | frozen in the snapshot, untouched. `make data` not invoked. |
| `make run-all-tests` | never run in the shared tree. All 166 suites run in the snapshot. |
| `.o` / `rb_engine*` in the **live** tree | **none written by me.** Builds confined to the snapshot. |
| `tests/*.log` in the snapshot | 0. |
| Line endings | `zones.c` LF (CR=0), `card.c` LF (CR=0), `engine.c` LF (CR=0), new suite LF (CR=0). No whole-file rewrite; every change via `edit`. |
| Quiescence | live tree stable across a 15 s settle on all three files before editing. |

### SHA-256 of every file modified

| file | SHA-256 |
|---|---|
| `src/core/zones.c` | `6ea125616d54db8f258c50bf633d15c5fea487567046cae62897142579e6f4bb` |
| `src/core/card.c` | `d334703c94884dee2c1061fa871656b8bb44051c4a2f984a8c81bc25c968a2b4` |
| `src/engine.c` | `068e103bf3eaff9e386ed21d34277afff5ffa3b01da86399e675843f53fdf9e7` (comment wording differs from the landed copy; code byte-identical) |
| `tests/test_parity_dead_twins.c` | `803b0ed35ada7a327f6430171d6f99d758dc0a108b2364164fc4e82aa442baa8` |
| `src/core/alloc_counter.c` | tombstone prepended; not compiled, so no build number depends on it |

Landed-live equivalence was proved by a **code-only** comparison (comments and
blank lines stripped with the same lexer): `src/core/zones.c` **783 lines
identical**, `src/core/card.c` **1 085 identical**, `src/engine.c` **936
identical**. The only `engine.c` delta is one word of my own comment.

---

## 12. Regressions I caused

**None.** Stated plainly and with the evidence:

* `^FAIL` (no colon) is **1 348 before and 1 348 after** across 165 → 166
  suites. Not one assertion anywhere moved.
* The only per-suite movement is `coverage_gate` +1 on `^FAIL:`, which is the
  meta-suite counting my new source file and is explicitly not a metric.
* One **process** regression, self-inflicted and reported rather than hidden:
  the harness timeout killed my corpus runner mid-link and `zone_filter_parity`
  was recorded `BUILD_RED` with a **0-byte log**. Re-run standalone it compiles
  (`CC_RC=0`), runs (`RUN_RC=0`) and prints `ALL ZONE AND FILTER CHECKS PASSED`
  with **0** `^FAIL` lines. That row was corrected from a measured re-run, not
  assumed.
* Two `edit` failures were retried after re-reading the live file (the tree had
  moved); neither left a partial write.
* `card.c` and `engine.c` were editable when edited and are in the fence now.
  Their edits are left in place and flagged in §1 — they are the §3 DEAD-WRONG
  fixes, and reverting would reinstate a card-property table that misreads a
  real corpus node and a `play_card` that puts energy cards in the live zone.
