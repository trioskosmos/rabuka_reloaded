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

## 1. METHOD — the reachability tool, and three bugs I hit in it

Tool: `b1/reach.py` (in my snapshot dir). It builds a whole-program call graph
over `src/**/*.c` and computes the transitive closure from real program roots.

**Bug 1 — wrong root seed.** First version seeded roots as
`all_called_names - all_defined_names`, which for a self-contained C library is
**4 names** — almost every callee *is* defined somewhere. It reported 55/55 of
the `rb_setup_*` family dead, including `rb_setup_settle_single_player_state`,
which `web_server.c:542` plainly calls. Correct seed: a root is an `rb_*` call
site **not inside any recognised function-definition body**.

**Bug 2 — `extern` declarations parsed as definitions.** `web_server.c:513` is
`extern void rb_setup_settle_single_player_state(GameState *g);`. Treated as a
definition, its "body" ran to the next definition line and swallowed the three
real call sites at 542/715/739, so the function read dead *and* the whole
web_server subtree lost its edges. Fixed by excluding `extern` from the
definition pattern (55→24 dead).

**Bug 3 — edges only followed `rb_`-prefixed callees.** All of web_server.c's
own helpers are plain `static` with no prefix (`handle_request`,
`serialize_state`, `append_pending_choice`), so the walk dead-ended at the first
non-prefixed hop and reported the entire web server subtree dead — including
`rb_setup_settle_single_player_state`, which is reached *only* from there.
Fixed by collecting edges for any callee (`CALLANY`), not just `rb_*`. Also had to
seed `WinMain` by hand: it is `#ifdef __CYGWIN__` (`web_server.c:896-901`), is
not `rb_`-named, and is swallowed by the preceding definition's line span.

Comment stripping is load-bearing: `game_setup.c:30-183`, `choice.c`,
`game_state_core.c` and `choice_renderer.c` all carry large `/* */` headers that
name dozens of functions. A naive grep turns those into ~55 spurious edges and
would report several genuinely-dead functions as live.

**Validation: 18/18 anchors pass** (`b1/validate.py`). Each anchor's expected
value is a grep-confirmed call site, not a guess — including three that must be
dead (`rb_setup_settle_auto`, `rb_setup_execute_action`, `rb_setup_build_two_decks`,
`rb_setup_flatten_and_sort_for_gba`, `rb_setup_deck_build_status_str` have no
call site outside their own definition).

**Result at HEAD c9da256e:** 2 304 definitions, 89 roots.
`rb_setup_*` / `rb_handle_*` / `rb_check_*_position` / `rb_gsc_concede` family:
**55 defined, 33 live, 22 dead.** Adding test-suite callers moves **none** of the
22, so none is TEST-ONLY — they are fully dead.

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
The one genuine behavioural gap in this file is **Rule 12-1**
(`record_action_boundary`), and it is currently masked because the faithful
executor is dead.

---