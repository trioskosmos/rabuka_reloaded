# AUDIT batch 1 — "the Rust modules no agent has ever audited"

HEAD measured: **c9da256e** (`feat(engine, gba): card binary loading, deck log
tooling, and board display updates`).
Snapshot baseline: `C:/Users/trios/AppData/Local/Temp/kilo/b1/` (477 files,
`*.o` / `rb_engine_*` / `*.log` excluded).
Free disk at start: 12.78 GB.

---

## 0. PRE-FLIGHT: 8 of the 15 assigned modules DO NOT EXIST

Before any inventory, every assigned path was existence-checked against both
trees (`engine/src/**` and `engine_c_wip/src/**`), plus a recursive filename
sweep of the whole repo for the missing basenames.

| assigned module | Rust file | C file | verdict |
|---|---|---|---|
| `game/game_setup.rs` | EXISTS (2 337 L) | `src/game/game_setup.c` (1 468 L) | in scope |
| `game/game.rs` | **ABSENT** | ABSENT | no such module |
| `game/input.rs` | **ABSENT** | ABSENT | lives in `platforms/*/src/input.rs` (12 platform crates), not the engine |
| `game/harness.rs` | **ABSENT** | ABSENT | real file is `engine/src/bin/harness.rs` (178 L) — a `bin`, not a lib module |
| `game/link.rs` | EXISTS (905 L) | ABSENT | no C twin |
| `game/menu.rs` | EXISTS (1 023 L) | ABSENT | no C twin |
| `game/deck_parser.rs` | EXISTS (220 L) | `src/game/deck_parser.c` (368 L) | in scope |
| `game/deck_builder.rs` | EXISTS (181 L) | `src/game/deck_builder.c` (163 L) | in scope |
| `game/platform_ui.rs` | EXISTS (463 L) | ABSENT | in scope (scope decision) |
| `effects/inspect.rs` | **ABSENT** | ABSENT | no such module |
| `effects/write.rs` | **ABSENT** | ABSENT | no such module |
| `effects/set_card_identity.rs` | **ABSENT** | ABSENT | real home `effects/ability_ops.rs:764,779` |
| `effects/hand.rs` | **ABSENT** | ABSENT | hand logic is spread over `effects/{selection,look,move_cards,deck}.rs` |
| `core/debug_trace.rs` | **ABSENT** | ABSENT | no such module |

Confirmed by `engine/src/game/mod.rs:14-26`, `engine/src/ability/effects/mod.rs:14-37`
and `engine/src/core/mod.rs:6-27` — the authoritative module lists. None of them
declare `inspect`, `write`, `set_card_identity`, `hand`, `game`, `input`,
`harness` or `debug_trace`.

**Consequence:** the "never audited" premise for those 8 names is void — there
is no code to audit, so there is nothing that can be DEAD-WRONG in them. The
auditable surface is the **7 files that exist**, plus the relocated
`ability_ops.rs` / `bin/harness.rs` homes. Sections below follow that real
surface.

---

## 1. METHOD — the reachability tool, and four bugs I hit in it

Tool: `b1/reach.py`. It builds a whole-program call graph over `src/**/*.c` plus
`tests/**/*.c` and computes the transitive closure from real program roots.
**It found four separate bugs, each of which produced a plausible-looking wrong
answer. This is the single most important thing in this report.**

**Bug 1 — wrong root seed.** Version 1 seeded roots as
`all_called_names - all_defined_names`. For a self-contained C library that is
**4 names** — almost every callee *is* defined somewhere. It reported **55/55** of
the `rb_setup_*` family dead, including `rb_setup_settle_single_player_state`,
which `web_server.c:542` plainly calls. Correct seed: an `rb_*` call site **not
inside any recognised definition body**.

**Bug 2 — `extern` declarations parsed as definitions.** `web_server.c:513` is
`extern void rb_setup_settle_single_player_state(GameState *g);`. Treated as a
definition, its "body" ran to the next definition line and swallowed the three
real call sites at 542/715/739 — so the function read dead *and* the whole
web_server subtree lost its edges. (55 → 24 dead.)

**Bug 3 — edges only followed `rb_`-prefixed callees.** Every helper inside
web_server.c is a plain unprefixed `static` (`handle_request`, `serialize_state`,
`append_pending_choice`), so the walk dead-ended at the first non-prefixed hop
and reported the entire web server dead — including
`rb_setup_settle_single_player_state`, which is reached **only** from there.
Fixed by collecting edges for any callee. Also had to seed `WinMain` by hand: it
is `#ifdef __CYGWIN__` (`web_server.c:896-901`), unprefixed, and swallowed by the
preceding definition's line span.

**Bug 4 — TEST-ONLY collapsed into DEAD (the dangerous one).** I seeded test
reachability with `set(CALL.findall(testtext)) - defined`. Since the test *calls*
functions that are themselves defined, subtracting `defined` removed every seed —
0 test seeds survived, and **all 83 `rb_rend_*` reported DEAD**. That directly
contradicted the Makefile, which states `choice_renderer.c` is "Exercised by
tests/test_choice_renderer.c". Two further gaps: a test's own helper bodies must
contribute edges (test_choice_renderer.c's `t_*` shims call the `static`
`rb_rend_text_*` helpers), and **function-pointer wiring is invisible to a
call-site regex** — `t->ext.println = t_println;` (`choice_renderer.c:1617`) is a
real edge with no `t_println(` anywhere. After all three fixes: **83/83
`rb_rend_*` are TEST-ONLY**, matching the Makefile. This is the mirror image of
the `nm` caveat in the brief: a call-site-only scan cannot see a function-pointer
consumer, in the same way `nm` cannot see an intra-TU call.

Comment stripping is load-bearing throughout: `game_setup.c:30-183`, `choice.c`,
`game_state_core.c` and `choice_renderer.c` all carry large `/* */` headers that
name dozens of functions. A naive grep turns those into ~55 spurious edges and
would report genuinely-dead functions as live.

**Validation: 20/20 anchors** (`b1/validate.py`), each anchor's expected value a
grep-confirmed call site rather than a guess — including five that must be DEAD
(`rb_setup_settle_auto`, `rb_setup_execute_action`, `rb_setup_build_two_decks`,
`rb_setup_flatten_and_sort_for_gba`, `rb_setup_is_live_card_set_phase`) and one
that must be LIVE-by-function-pointer (`rb_rend_render_choice`).

**Final counts at HEAD c9da256e:** 2 304 definitions, 22 production roots,
5 649 live from production, 6 287 live including test callers.
- `rb_setup_*` / `rb_handle_*` / `rb_check_*_position` / `rb_gsc_concede`: **55
  defined, 33 LIVE, 22 DEAD, 0 TEST-ONLY.** Adding test callers moves none of the
  22, so none is merely test-only.
- `rb_rend_*`: **83 defined, 0 LIVE, 83 TEST-ONLY, 0 DEAD.**

---

## 2. `game/game_setup.rs` (2 337 L) ↔ `src/game/game_setup.c` (1 468 L)

36 Rust fns + 2 enums + 3 structs + 1 trait-ish `Action`. 55 C entry points
(including the `extern` declarations that resolve to `phase.c` / `zones.c`).

### 2a. The structural discovery: **C has TWO action executors, and the faithful one is DEAD**

Rust has exactly one: `game_setup.rs:474 execute_action` → delegates everything
(except Concede) to `TurnEngine::execute_main_phase_action_with_ability_index`
(`game_setup.rs:493`), then calls `gs.record_action_boundary(...)`
(`game_setup.rs:509`).

C has **two**:
1. `rb_setup_execute_action` (`game_setup.c:779`) — a faithful re-dispatch of the
   Rust switch. **DEAD.** Nothing calls it.
2. `rb_execute_main_phase_action` (`engine.c:1663`) — a second, independently
   written executor, and **this is the live one**: `main.c:70`,
   `web_server.c:814,818`, `phase.c:672`, plus 8 test suites.

So `rb_setup_execute_action` is not merely dead, it is **dead-wrong in three
specific ways** if anyone wires it "to activate the code":

**(DEAD-WRONG #1) `record_action_boundary` is missing — and it is NOT a
"GameState does not carry the field" XREQ.** `game_setup.c:774-778` claims
`record_action_boundary` cannot be ported because GameState lacks
`loop_last_action` / `pending_loop_protocol` / "an exported state hash". Two of
those three premises are **false**:
- the state hash **is** exported: `rb_generate_state_hash` at
  `game_state_abilities.c:1405`, and `rb_check_permanent_loop` at
  `game_state_abilities.c:1432` is the live port of
  `check_permanent_loop` (`abilities.rs:3438`), called live from `phase.c:1040`;
- `game_state_history` / `n_game_state_history` / `loop_detected` are real
  fields (`rabuka.h:1803-1804`), written by `rb_reset_loop_detection`
  (`tracking.c:15`).
Only `loop_last_action` and `pending_loop_protocol` genuinely have no C field.
So the honest XREQ is *two* fields, not three — and the omission silently drops
**Rule 12-1**, which is the "repeat this action 3× → offer Draw / Continue"
protocol (`abilities.rs:3471-3484`). Note this is a **known-acceptable** class of
gap (a missing *prompt*), but the stated reason is wrong and would mislead the
next reader.

**(DEAD-WRONG #2) `rb_setup_execute_action` drops `ability_index`.**
`game_setup.c:791-795` (an XREQ note) routes UseAbility through
`rb_execute_main_phase_action(g, 0, card_id, ...)` with **no** ability-index
parameter. Rust passes `p.ability_index` (`game_setup.rs:489`) into
`execute_main_phase_action_with_ability_index`. A gained-ability activation
(`RB_GAINED_ABILITY_INDEX_BASE + n`) therefore cannot be routed through the
faithful executor at all.

**(DEAD-WRONG #3) `UseAbility` pending-choice check is inverted relative to
Rust.** `game_setup.c:789-790` returns 0 if `rb_has_pending_choice(g)`, i.e.
UseAbility **fails** while a choice is pending. Rust `actions/mod.rs:256-265`
(the cited twin) checks UseAbility *independently* precisely so that it does
**not** go through the pending-choice resume — it is allowed to proceed, not
rejected. The C comment says "must never route through the pending-choice
resume" and then implements that as a hard reject. I have **not** changed this:
`game_setup.c` documents the intent, `actions/mod.rs` is not in my ownership,
and the function is dead, so fixing it has no behavioural effect today. Flagged
as an ownership request.

### 2b. The other 27 dead `rb_setup_*` are a coherent, documented dead island

`rb_setup_execute_action` is dead, and it was the **only** consumer of:
`rb_setup_concede`, `rb_setup_reset_loop_detection`, `rb_setup_is_automatic_phase`,
`rb_setup_is_live_card_set_phase`, `rb_setup_handle_mulligan_{selection,confirmation}`,
`rb_setup_resolve_hand_answer_index`, `rb_setup_deck_build_status_str`,
`rb_setup_extract_{series,rarity,base}`, `rb_setup_series_priority`,
`rb_setup_rarity_priority`, `rb_setup_gba_sort_key`, `rb_setup_flatten_and_sort_for_gba`,
`rb_setup_build_two_decks`, `rb_setup_execute_action_for_player`.

That is **not** an accident and not new work — `game_setup.c:60-177` is a
long-standing header that already explains each cluster:
- the GBA sort helpers serve `rb_setup_flatten_and_sort_for_gba`, whose only
  caller is a **GBA-side** target that does not exist in `engine_c_wip`;
- `rb_setup_build_two_decks` exists in Rust purely to sequence
  `setup_game` + default-energy, and `engine_c_wip` folds the default-energy
  step into `rb_build_deck_from_card_numbers` itself (`deck_builder.c:54-58`);
- the mulligan window is unreachable because `rb_generate_action_candidates`
  only offers rows when `phase == RB_PHASE_MAIN`, so
  `rb_push_mulligan_rows` / `rb_push_live_set_rows` are called
  (`phase.c:2198,2210`) but can never return a row.

These are **DEAD-CORRECT** (faithful, unreachable, and each is a deliberate
platform-layer surface), except where §2a says otherwise.

### 2c. The remaining 22 dead, classified

| function | why dead | class |
|---|---|---|
| `rb_setup_execute_action` | superseded by `rb_execute_main_phase_action` (engine.c:1663) | **DEAD-WRONG** (§2a) |
| `rb_setup_execute_action_for_player` | only caller is the dead one above | DEAD-CORRECT |
| `rb_setup_concede` | only caller is the dead one above | DEAD-CORRECT |
| `rb_setup_reset_loop_detection` | only caller is the dead one above | DEAD-CORRECT |
| `rb_setup_is_automatic_phase` | reachable, but via `settle_single_player_state` — **LIVE**, see correction below | — |
| `rb_setup_settle_single_player_state` | **LIVE** via `rb_inject_choice_ability_context` (game_state_abilities.c:1526) ← `append_pending_choice` (web_server.c:291) ← `handle_request` | — |
| `rb_setup_settle_auto` | no call site anywhere in `src/` or `tests/` | DEAD-CORRECT |
| `rb_setup_auto_advance_one` | no call site; Rust's 26 call sites are all `src/bin/*` harnesses | DEAD-CORRECT |
| `rb_setup_build_two_decks` | no call site; Rust's 20 are all bins. `game_setup.c:173-177` explains the C folds default-energy into `rb_build_deck_from_card_numbers` | DEAD-CORRECT |
| `rb_setup_handle_mulligan_{selection,confirmation}` | no call site; Rust's sole caller is itself uncallable (mulligan rows never offered) | DEAD-CORRECT |
| `rb_setup_resolve_hand_answer_index` | only caller is `rb_setup_handle_mulligan_selection` (dead) | DEAD-CORRECT |
| `rb_setup_deck_build_status_str` | intra-TU only, from `rb_setup_build_two_decks` (dead) | DEAD-CORRECT |
| `rb_setup_extract_{series,rarity,base}`, `rb_setup_{series,rarity}_priority`, `rb_setup_gba_sort_key`, `rb_setup_sort_deck_for_gba`, `rb_setup_flatten_and_sort_for_gba`, `rb_setup_str_slice` | the GBA deck-sort cluster; only entry is `rb_setup_flatten_and_sort_for_gba`, whose target is not in the Makefile | DEAD-CORRECT (platform surface, correctly out of scope) |
| `rb_setup_is_live_card_set_phase` | no call site | DEAD-CORRECT |
| `rb_setup_stage_area_from_parameters` | only caller is the dead `rb_setup_execute_action` | DEAD-CORRECT |

**Nothing in `game_setup.c` is a missing engine path.** Every Rust function in
`game_setup.rs` has a C counterpart; the C's only *excess* is the GBA sort
cluster (7 functions) and the mulligan window, both deliberate platform surfaces.

### 2d. 🔴 FOUND WITHOUT FIXING (ownership): `rb_check_permanent_loop` truncates the state hash — loop detection can NEVER fire

This is a **LIVE** engine bug, in the same Rule 12-1 area as §2a, and it is
already red in the baseline (`parity_rules_extra`, 2 assertions):

```
FAIL: ENGINE BUG: an immediately repeated, otherwise identical state IS a
repetition, so rb_check_permanent_loop must return 1. It returns 0 because the
64-bit fingerprint is stored into the 32-bit `int game_state_history[64]`
(rabuka.h:1464) and then compared sign-extended as 64-bit ...
```

Verified independently:
- `rb_generate_state_hash` (`game_state_abilities.c:1405-1430`) returns
  `uint64_t` (FNV-1a).
- `rb_check_permanent_loop` (`game_state_abilities.c:1439`) stores it as
  `(int)h` — a **32-bit truncation**.
- The field is `int game_state_history[64]` (`rabuka.h:1803`); Rust's is
  `pub game_state_history: Vec<u64>` (`core/game_state/mod.rs:117`).
- The comparison at `game_state_abilities.c:1436` is
  `(uint64_t)g->game_state_history[i] == h`, i.e. sign-extend-then-compare against
  the **full 64-bit** hash. A sign-extended 32-bit value can only equal a 64-bit
  value that actually fits in 32 bits, and an FNV-1a digest essentially never
  does. So the equality is unreachable in practice and the function always
  returns 0 on a repeat.

Consequence: the whole Rule 12-1 / repetition protocol is inert in C.
`rb_check_permanent_loop` is LIVE (`phase.c:1040`) and `rb_is_loop_detected`
(`game_state_abilities.c:1445`) is exported, so callers believe detection works.

**Not fixed: `src/core/game_state_abilities.c` is explicitly not mine** (the
brief lists `src/core/game_state_abilities.c` under DO-NOT-EDIT). The fix is
one line — widen the field to `uint64_t` in `rabuka.h` (which is *also* not
mine, and is pinned: `sizeof(GameState)` = 1 090 264 B must not move). So this
needs an owner who can change both files together and re-pin the struct size.
Raising it as the highest-value item in this report.

⚠️ Note the interaction with §2a: fixing the truncation alone would NOT restore
Rule 12-1, because `rb_setup_execute_action` — the only place that would call
`record_action_boundary` — is dead, and `rb_setup_reset_loop_detection` clears
`n_game_state_history`/`loop_detected` but has no `loop_last_action` or
`pending_loop_protocol` equivalent. Both halves are needed.

---

## 3. `game/link.rs` · `game/menu.rs` · `game/{game,input,harness}.rs`

| Rust item | line | C twin | reachability in Rust | verdict |
|---|---|---|---|---|
| `LinkAction` (struct) | 80 | NONE | 1 ref, `platform_ui.rs` | **OUT OF SCOPE** — link-play transport |
| `LinkAction::from_action` | 113 | NONE | 8 refs (bot/PPO/observation) | OUT OF SCOPE |
| `LinkAction::encode` | 129 | NONE | via `encode_setup` chain | OUT OF SCOPE |
| `LinkAction::decode` | 158 | NONE | — | OUT OF SCOPE |
| `find_local_action` | 226 | NONE | **0 callers** | DEAD in Rust too |
| `encode_setup` | 250 | NONE | 2 refs, **both `platforms/gba`** | OUT OF SCOPE |
| `decode_setup` | 262 | NONE | 2 refs, **both `platforms/gba`** | OUT OF SCOPE |
| `LinkMatchReport` (struct) | 433 | NONE | — | OUT OF SCOPE |
| `run_link_match` | 446 | NONE | 3 refs: `platform_ui.rs`, `platforms/gba/src/bin/rabuka_gba.rs` | OUT OF SCOPE |
| `show_result` | 91 | NONE | 6 refs (`link.rs`, `match_runner.rs`) | OUT OF SCOPE |
| `select_with_initial` | 138 | NONE | 1 ref (`language.rs`) | OUT OF SCOPE |
| `select_opt` | 150 | NONE | 3 refs, **all `platforms/gba`** | OUT OF SCOPE |
| `select` | 221 | NONE | internal | OUT OF SCOPE |
| `menu_select` | 226 | NONE | **0 callers** | DEAD in Rust too |
| `menu_select_with_cards` | 306 | NONE | **0 callers** | DEAD in Rust too |
| `menu_select_detailed` | 417 | NONE | **0 callers** | DEAD in Rust too |
| `select_action` | 536 | NONE | 1 ref (`link.rs`) | OUT OF SCOPE |
| `human_turn` | 707 | NONE | 1 ref (`match_runner.rs`) | OUT OF SCOPE |
| `handle_choice` | 719 | NONE | 1 ref (`match_runner.rs`) | OUT OF SCOPE |

**`link.rs` and `menu.rs` are console/UI front-ends and have no C twin by
design.** Every consumer is either `platforms/gba`, `match_runner.rs`, or
`link.rs` itself — i.e. code that exists only to drive a `PlatformUi` backend.
`engine_c_wip` has **no `PlatformUi` implementation at all** (Makefile:56:
"this tree has no PlatformUi backend"). `match_runner.rs` has no C twin either.
Porting them would mean inventing a C UI abstraction that nothing would call.

**Nothing to close here, and nothing DEAD-WRONG.** The four zero-caller items
(`find_local_action`, `menu_select`, `menu_select_with_cards`,
`menu_select_detailed`) are dead **in Rust too**, so they are not divergences.

`game/game.rs`, `game/input.rs`, `game/harness.rs` do not exist (§0). The real
`harness.rs` is `engine/src/bin/harness.rs` (178 L) — a `bin`, not a library
module, with no C counterpart by design (its role is `src/test_game.c` +
`rb_engine`'s `main.c`).

---

## 4. `game/deck_parser.rs` (248 L) ↔ `src/game/deck_parser.c` (368 L)

12 C functions, all with Rust counterparts. Classification:

| C function | class |
|---|---|
| `rb_normalize_card_no` | **LIVE** |
| `rb_parse_deck_line` | **LIVE** |
| `rb_parse_deck_content` | **LIVE** |
| `rb_parse_deck_file_ex` | **LIVE** |
| `rb_parse_deck_file` | **LIVE** |
| `rb_deck_card_numbers_free` | **LIVE** |
| `rb_deck_list_free` | **LIVE** |
| `rb_deck_status_str` | TEST-ONLY |
| `rb_deck_list_to_card_numbers` | TEST-ONLY |
| `rb_deck_set_error`, `rb_deck_append_entry`, `rb_deck_basename_without_txt` | `static`, folded into their callers — LIVE via the above |

Rust-only, correctly unported: `load_two_decks` / `load_two_decks_with_abilities`
(`deck_parser.rs:38,81`) read `baked/deck_*.json` / `DECK_CARD_BLOBS` via
`include_str!`, which is a **build-time Rust embedding**. `engine_c_wip` has its
own baked decks (`web_server.c:588-648` `append_deck_presets`,
`append_deck_file_content`, `parse_deck_content`). Different mechanism, same job —
not a gap.

**No DEAD-WRONG, no missing engine path.** `deck_parser` is in good shape.

---

## 5. `game/deck_builder.rs` (181 L) ↔ `src/game/deck_builder.c` (163 L) — the non-determinism verdict

| C function | class |
|---|---|
| `rb_build_deck_from_card_ids` | **LIVE** (`web_server.c:533,567`) |
| `rb_init_game_from_built_decks` | **LIVE** (`web_server.c:541,710,735`) |
| `rb_built_deck_clear` | **LIVE** (`web_server.c:690,691`) |
| `rb_default_energy_template` (static) | LIVE, reached from `rb_build_deck_from_card_ids` |
| `rb_valid_template_id` (static) | LIVE, same |
| `rb_build_deck_from_card_numbers` | **TEST-ONLY** — its only production caller is the dead `rb_setup_build_two_decks` |
| `rb_build_deck_from_list` | **TEST-ONLY** |

### 5a. VERDICT on the `deck_builder.c:12` landmine: **the C is right, the Rust
cannot be matched, and the contract should be pinned as-is**

Facts, all verified against `cards/cards.json` (not inferred):
- `cards.json` index 0 = `IKZL-PR-001-PR`, index 1 = `LL-E-001-SD`. **Both are
  `type: 'エネルギー'`** — both are genuinely energy cards, and they are
  *different* cards (PR rarity vs SD starter-deck rarity).
- `rb_default_energy_template` (`deck_builder.c:10-18`) tries
  `rb_find_card_by_no("LL-E-001-SD")` **first**, and only falls back to
  "lowest-index energy card" — which for this corpus is index 0.
  So the C's *effective* answer today is id **1**, and its fallback would be
  id **0**.
- Rust `add_default_energy_cards_from_database` (`deck_builder.rs:186-190`) does
  `card_db.cards.iter().find(|(_, card)| card.is_energy())`. `cards` is a
  `HashMap` with `RandomState` (`crate::HashMap` = `std::collections::HashMap`
  on std targets), so **iteration order is randomised per process**. Rust picks
  an arbitrary energy card, differently on every run.

⛔ **I did not "fix" the C to agree with an observed Rust answer, and I did not
change either side.** Per the brief: there is nothing to match. Any single
observed Rust answer would have been an artefact of one process's hash seed, and
"fixing" the C to it would have baked a coin flip into the port.

**What I recommend instead (a contract pin, not a behaviour change):** the C's
two-step order — *named card first, lowest-index energy as fallback* — is the
only deterministic, reviewable contract available, and it is what
`deck_builder.c:12` already encodes. `LL-E-001-SD` is a **starter-deck**
(`product: 'スタートデッキラブライブ！'`), which is the correct default for a
game that boots into a starter deck; falling back to a *PR* card would be the
wrong default. So: **keep the C as-is, and record that the Rust side is
non-deterministic by construction** so no future reader mistakes it for a parity
bug.

⚠️ One genuine reachability consequence worth recording: because
`rb_build_deck_from_card_numbers` is TEST-ONLY, the default-energy step is
reached in production only through `rb_build_deck_from_card_ids` when the
caller passes **zero** energy ids (`web_server.c:533` builds from `main_ids`
only). That path *is* live and *does* hit `rb_default_energy_template`. So the
contract is load-bearing in production — it is not merely theoretical.

---

## 6. `core/debug_trace.rs` → the real twin is `src/ability/debug.c`

`core/debug_trace.rs` does not exist. The Rust module is
`engine/src/ability/debug.rs`; its C twin is `src/ability/debug.c` (207 L),
exported through `rabuka.h:2525-2526`.

**Verification requested — does the peer's `web_server.c` wiring work, and does
the trace land somewhere observable? YES on both counts:**

- Rust: `web_server.rs:3653` `if std::env::var("RABUKA_RULE_LOG").as_deref() == Ok("1")`
  → `crate::ability::debug::set_debug(true)`.
- C: `web_server.c:884-885` `const char *rl = getenv("RABUKA_RULE_LOG"); if (rl &&
  strcmp(rl, "1") == 0) rb_ability_debug_set(1);` — **same env var, same
  comparison, same position** (before `rb_load` at `:887`, mirroring Rust's
  placement before the card-database load at `:3659-3661`).
- The gate is a real `atomic_bool` (`debug.c:6`), matching Rust's `AtomicBool`.
- `rb_ability_debug_enabled()` is consumed by **~30 sites** across
  `ability_queue.c`, `compound.c`, `condition.c`, `cost.c`, `describe.c`,
  `resolver.c`, `data.c`, `game_state_abilities.c`, `stats_pipeline.c`, `live.c`,
  `triggers.c`, `engine.c` — so the flag genuinely reaches the engine.
- Trace output lands in **two** places, both observable: the in-game rule log
  (`rb_abdebug_flush_to_rule_log`) and the structured log
  (`rb_abdebug_flush_to_structured_log`), per `debug.c:109,114`.

Classification: `rb_ability_debug_enabled` **LIVE**, `rb_ability_debug_set`
**LIVE**, `rb_abdebug_init`/`_p`/`_condition`/`_cost_pay`/`_effect` **LIVE**,
`rb_abdebug_flush_to_rule_log` and `rb_abdebug_ability` **TEST-ONLY**,
`rb_abdebug_dropped_count` **DEAD** (an observability counter nothing reads).

**Nothing to fix. The peer's wiring is faithful.**

---

## 7. `game/platform_ui.rs` vs `choice_renderer.c` — SCOPE DECISION

### Decision: **`platform_ui.rs` is correctly OUT of scope — same test as `display.rs`.** But `choice_renderer.c` is NOT display-only; it is TEST-ONLY and correctly retained.

Applying the `display.rs` test literally — *can this code affect behaviour?* —
requires care, because the two files are **not** in the same category:

**(a) `platform_ui.rs` — OUT.** 463 L, and every function is one of:
- a **trait method declaration with a default no-op body** (`render_board`
  `:45`, `set_actionable_cards` `:53`, `show_board_overlay` `:119`,
  `open_start_menu` `:174`, …) — 17 `&mut` occurrences in the file are **all
  `&mut self`**, i.e. the UI object mutating itself, never `&mut GameState`;
- a **pure text formatter** taking `&GameState`/`&Card` by shared reference:
  `wrap_text` `:291`, `one_line` `:319`, `heart_str` `:353`, `card_stat_text`
  `:375`, `card_detail_title` `:467`, `card_ability_text` `:474`.

So `platform_ui.rs` **cannot** mutate game state: it has no `&mut GameState` in
463 lines. By the `display.rs` precedent (2 014 lines, no `&mut GameState`, ruled
OUT), it is **OUT**. And it has no C consumer to port *to* — `engine_c_wip` has
no `PlatformUi` backend at all (Makefile:56). Every real consumer is
`platforms/gba`, `platforms/*/src`, or `match_runner.rs`. **Verdict: OUT, no
action.**

**(b) `choice_renderer.c` — different, and it is NOT display-only.** I checked
the same test rather than assuming, because the brief pairs these two files and
they are easy to conflate. Every one of its 21 `GameState` occurrences is
`const GameState *` — `rb_rend_show_board_overlay(const GameState *gs)`,
`rb_rend_render_card_choice_grid(..., const GameState *gs, ...)`,
`rb_rend_open_start_menu(const GameState *gs)`. So it *also* passes the mutation
test…

…but the mutation test is **necessary, not sufficient**. What makes
`choice_renderer.c` different is that it is **not merely declared — it is
compiled into `OBJ_LIB` and it is the only place the C implements
`platform_ui.rs`'s text contract.** The C's ports of `platform_ui.rs`'s helpers
live *inside* it: `rb_rend_token_cols` (`:598`) ↔ `token_cols`
(`platform_ui.rs:259`), `rb_rend_text_units` (`:624`) ↔ `text_units` (`:267`),
`rb_rend_wrap_text` (`:700`) ↔ `wrap_text` (`:291`), `rb_rend_heart_str` (`:805`)
↔ `heart_str` (`:353`), `rb_rend_card_stat_text` (`:821`) ↔ `card_stat_text`
(`:375`), `rb_rend_card_detail_title` (`:868`) ↔ `card_detail_title` (`:467`),
`rb_rend_card_ability_text` (`:884`) ↔ `card_ability_text` (`:474`).

So the correct classification for the 14 (measured: **83**, see §1) `rb_rend_*`
entry points is **TEST-ONLY — 83/83, 0 DEAD** — not OUT. They are exercised by
`tests/test_choice_renderer.c`, which ships a text-only backend that captures and
replays every screen. That is exactly what Makefile:60 states
("Exercised by tests/test_choice_renderer.c"), and my measurement now confirms it
independently.

⚠️ **Why the brief's "14 `rb_rend_*`" undercounts.** There are **83**
`rb_rend_*` definitions, not 14 — 27 are exported (`rb_rend_*`) and the rest are
`static` helpers. If a future audit takes "14" as the denominator it will
under-report the retained surface by ~6×. Also note these 83 were nearly
misclassified as DEAD by my own tool (bug 4, §1) before I handled
function-pointer wiring — the text backend is reached only through
`t->ext.println = t_println` assignments (`choice_renderer.c:1617+`), which no
call-site scan can see.

**Answer to the question asked:** a render surface is *already* present and
compiled (`choice_renderer.c`); what is missing is only a `PlatformUi` backend to
drive it from a console, which `engine_c_wip` deliberately does not have. So
**no new render surface is needed, and none should be added.**

---

## 8. MEASUREMENT — method, HEADs, and what I changed

**Method used: `make clean` + `make all`, then linked each suite directly** with
the Makefile's own `OBJ_LIB`, exactly as `make t-run T=<stem>` does
(`Makefile:513-515`). I did **not** use `make t-nogate`, because it delegates to
`make t GATE_DEP=` and `t` **requires an explicit `T=<stem>`** — `t-nogate` with
no `T` exits 2 with "no suite named". There is no all-suites target in the
Makefile, so the sweep is necessarily a driver loop. I did not weaken the data
guard, regenerate anything, or touch `guard_data_blob.sh` / `regen_all.sh`.

**FAIL counting:** `grep -c '^FAIL'` — no colon, case-sensitive, on stdout and
stderr captured together. Exit code not used as a signal.

⚠️ **Two different HEADs — the count difference is a file-hash sweep, not me.**

| run | HEAD | result |
|---|---|---|
| baseline | `c9da256e` | **48 PASS / 118 FAIL** (166 suites) |
| after | `aed8bd0d` | **36 PASS / 130 FAIL** (166 suites) |

**I made no code change.** `git diff --stat engine_c_wip` shows my only edit is
this file; `src/ability/effects/move.c`, `src/core/modifiers.c`,
`src/core/game_state_abilities.c`, six `tests/*`, and `tools/verdict_runner.c`
are all peers' in-flight work, and `HEAD` itself advanced `c9da256e → aed8bd0d`
mid-task.

**The 12-suite swing is attributable to exactly one cause, and it is not a test
regression:** in the "after" run **57 suites fail to LINK**, versus 0 in the
baseline. The link error is
`undefined reference to 'BYTECODE_C0'..'BYTECODE_C3'` from
`src/ability/abilities_gen.o` — a **stale/mid-regeneration data blob**, i.e.
another agent's territory. Suites that previously failed on assertions and now
fail at link time flip PASS→FAIL in my table even though nothing about them
changed. Suites whose *pass/fail state* genuinely changed without a link error
are the peers' deliberate edits.

**Per-suite counts are therefore NOT comparable between the two runs**, and I am
not claiming a regression or an improvement. The baseline number to compare
against for *my* work is 48 PASS / 118 FAIL at `c9da256e`; the after-run
degradation is a peer's regeneration in flight. `parity_rules_extra` — the suite
carrying the §2d loop-detection bug — was red in **both** runs, which is the
relevant point: that bug is pre-existing and not something I introduced.

**Quiescence confirmed** for every file in my scope (SHA-256 identical across two
reads separated by a settle), at HEAD `aed8bd0d`:

```
A830208AE3142B73  src/game/game_setup.c
1B7ABDBD7D5E0422  src/game/deck_parser.c
94BFC202337BFF1C  src/game/deck_builder.c
747BF25D7C341621  src/ability/choice_renderer.c
88027F844771466B  src/ability/debug.c
4CB799557B838FBD  src/core/game_state_abilities.c
6259F1ED0FA6190E  include/rabuka.h
```

I did **not** verify quiescence for the two 2026-10-04 red suites the brief calls
the card-generation skew; per instructions I have not attributed those to my work.

---

## 9. SUMMARY OF THE REACHABILITY HISTOGRAM

Over my assigned surface, measured with the validated tool:

| class | count | what |
|---|---|---|
| **WIRED / LIVE** | 42 | `rb_setup_*` live (33) + `deck_parser` live (7) + `deck_builder` live (3)… see note |
| **TEST-ONLY** | 86 | `rb_rend_*` (83) + `deck_parser` (2) + `debug.c` (2) |
| **DEAD-CORRECT** | 21 | the dead `game_setup` island (§2c), GBA sort cluster, mulligan window |
| **DEAD-WRONG** | 1 | `rb_setup_execute_action` (§2a) — 3 concrete divergences |
| **OUT OF SCOPE** | 19 | `link.rs` + `menu.rs` — no `PlatformUi` backend exists in the C port |
| **VERIFIED-NOOP** | 0 | n/a — no assigned module turned out to be unreachable data |

*(The LIVE row is the `game_setup`-family count 33 plus the separately-tallied
`deck_parser`/`deck_builder`/`debug.c` entries; those groups are disjoint and are
listed per-table in §2c, §4, §5, §6 rather than summed into a single figure,
because the families were measured with different reachability roots.)*

**Net code changes: none.** No reachable gap was found that I own. The three
things worth an owner's time are §2d (live loop-detection bug, high value),
§2a (dead-wrong executor that will misbehave the moment someone wires it), and
§5a (the deck_builder contract pin).

---

## 10. FOUND WITHOUT FIXING — index

1. 🔴 **§2d** `rb_check_permanent_loop` truncates a `uint64_t` hash into
   `int game_state_history[64]` (`game_state_abilities.c:1439` vs
   `rabuka.h:1803`; Rust is `Vec<u64>`, `core/game_state/mod.rs:117`), so
   repetition detection can never fire. LIVE, already red in `parity_rules_extra`.
   Needs `game_state_abilities.c` + `rabuka.h` together, and would move the
   pinned `sizeof(GameState)`. **Not mine.**
2. 🟠 **§2a** `rb_setup_execute_action` is DEAD-WRONG in 3 ways (missing
   `record_action_boundary`, dropped `ability_index`, inverted UseAbility
   pending-choice check). Its "cannot be ported, GameState lacks the fields" note
   is **factually wrong** on 2 of 3 fields. `game_setup.c` **is** mine but
   fixing it has zero behavioural effect while dead, so I left the code and
   documented the divergence rather than silently editing a dead path.
3. 🟡 **§5a** Rust `deck_builder.rs:186` iterates a `RandomState` HashMap, so the
   default-energy card is non-deterministic per process. C's
   `LL-E-001-SD`-then-lowest-energy contract is correct and left unchanged.
4. 🟡 **§1 bug 4** a call-site-only reachability scan classifies all 83
   `rb_rend_*` as DEAD, because the text backend is wired purely through
   function-pointer assignments. Any future audit reusing this method must handle
   `= fn;` edges or it will delete a working test backend.
5. 🟡 **§1 bug 2** `extern` declarations parse as definitions and silently
   swallow following call sites. This cost 31 false DEAD verdicts before it was
   caught, and it is the same failure shape as the brief's `nm` caveat.
6. ℹ️ **§7** the brief's "14 `rb_rend_*`" is an undercount; there are **83**.

---