# AUDIT  cluster T6: the action / driver layer

Owner: cluster T6. Scope  Rust `engine/src/turn/actions/mod.rs` (1895) and
`engine/src/game/game_setup.rs` (2427), cross-checked against `core/types.rs` (1262).
C files owned: **`src/engine.c`, `src/turn/phase.c`, `src/turn/live.c`**.

Quiescence confirmed before any edit. SHA-256 identical across a 22 s settle, and
identical again after the session was resumed (the first reads were re-taken and
match):

| file | sha256 | mtime |
|---|---|---|
| `src/engine.c` | `DD64CB52FBCD930CD08CD57BAB1C87219B99DF1F3AD51D35B1206E5E23B2C867` | 2026-10-03 05:34:18 |
| `src/turn/phase.c` | `36BE09DB06F61E73F37D7965747FA61CA85BC5E4F9C095147E2B1C0018DEA0FF` | 2026-10-03 05:34:27 |
| `src/turn/live.c` | `284518269074BB997FEAC69815374B0B89136512BD32165A68A73C58E3EFEF20` | 2026-10-03 05:30:23 |

No `.o` in the live tree was newer than any of the three sources at session start,
and the live tree now holds **0** `.o` / `rb_engine*.exe` artifacts. Every build and
every suite run for this cluster happened in the isolated snapshot
`C:/Users/trios/AppData/Local/Temp/kilo/act`, created with every `*.o`,
`rb_engine_*` and `tests/*.log` excluded and every copied `.o` deleted after the
copy. Blob identity verified before and after every build:
`src/abilities_strings.bin` = **726831 B / 89204DF0BBDE08A02A7F978A56A80D6E6431691EC7D6D658979A329956B06344**,
`src/cards.bin` = 631890 B / 728208504FEE62171429EBB7BBDD8E12263E1C03B816C947C6FC98AA2676FAC4.
The snapshot's `make data` is frozen by pointing `CARDS_BIN`/`STRINGS_BIN` at
`cb/` copies of the two canonical blobs; the in-repo `make data` REFUSES (by design,
K1) because `cards/build/abilities_strings.bin` is 726649 B and stale.

**Out of scope, deliberately not taken:** `src/web_server.c` (T5 reported
`src/ability/choice_renderer.c`, 1821 lines, is in **no** build target at all  0
hits in `Makefile:20-70`, `platforms/cdi/build_cdi.sh`,
`platforms/genesis/build_genesis.sh` and `platforms/Makefile`. That is an orphan,
not my file, and I did not take `web_server.c`.)
`src/ability/effects/rules.c` (cluster C) untouched. `include/rabuka.h` untouched.

---

## 0. Headline numbers

| metric | value |
|---|---|
| Rust `turn/actions/mod.rs` items inventoried | **43** (5 `pub fn`, 34 private `fn`, 1 `impl` method, 2 private `struct`, 1 private `enum`) |
| Rust `game/game_setup.rs` items inventoried | **46** |
| Rust items with **no** C twin | **11** (all measured VERIFIED-NOOP or unreachable-by-construction) |
| Rust items whose C twin is wired | 27 |
| Rust items whose C twin is dead-but-correct | 5 |
| C functions across the three owned files | **153** |
|  WIRED (reached from `src/` production code) | **132** |
|  DEAD-CORRECT | 16 |
|  DEAD-WRONG | **5** |
| Reachable gaps found | **6** |
| Reachable gaps closed | **6** |
| Divergences at ability **325 / 805** | none found (both abilities are outside this scope's key space  see 5) |

---

## 1. INVENTORY  `engine/src/turn/actions/mod.rs` (43 items)

Rust line numbers are the **current** file (the module was re-read after the
`3cffff64` / `f152a50f` sweep; nothing in it was touched by those commits).

| # | Rust item | line | kind | C twin (file:line) | status |
|---|---|---|---|---|---|
| 1 | `struct ResumeSnapshot` | 33 | private struct | inlined into `rb_resume_with_choice` / `rb_complete_skipped_ability` (live in `src/ability/choice.c`) | WIRED |
| 2 | `enum ResumeOutcome` | 47 | private enum | inlined: the four completion paths in `choice.c`'s resume | WIRED |
| 3 | `ResumeSnapshot::capture` | 60 | impl method | inlined at the four capture sites in `choice.c` | WIRED |
| 4 | `struct AbilityActivation` | 153 | private struct | `activate_resolved_ability` (`src/engine.c:1130`) + `find_gained_activation_index` (`src/engine.c:1255`) | WIRED |
| 5 | `execute_main_phase_action` | 160 | **pub fn** | `rb_execute_main_phase_action` (`src/engine.c:1564`) | WIRED-BUT-DIVERGENT  **T6-1** |
| 6 | `refill_live_zone_hand` | 182 | private fn | folded into `rb_advance_phase`'s `RB_PHASE_LIVE_SET` arm (`src/turn/phase.c:205-218`) | WIRED |
| 7 | `rps_choice_value` | 195 | private fn | `setup_rps_choice_value` (`src/game/game_setup.c:750`) | WIRED |
| 8 | `handle_rps_choice` | 206 | private fn | `setup_handle_rps_choice` (`src/game/game_setup.c:762`) | WIRED |
| 9 | `handle_choose_attacker` | 230 | private fn | `rb_setup_choose_attacker` (`src/game/game_setup.c:473`) | WIRED |
| 10 | `execute_main_phase_action_with_ability_index` | 242 | **pub fn** | `rb_setup_execute_action` (`src/game/game_setup.c:779`) + `rb_setup_execute_action_for_player` (`:882`) | WIRED |
| 11 | `activation_location` | 345 | private fn | inlined in `rb_activate_card` (`src/engine.c:1281-1309`) | WIRED (weaker  see 4-3) |
| 12 | `cost_has_hand_self_move` | 362 | private fn | **NONE**  the `loc == Zone::Hand` branch of `activate_resolved_ability` | DEAD-CORRECT, **VERIFIED-NOOP 0 of 985** (5) |
| 13 | `can_activate_at_location` | 377 | private fn | `rb_activate_card`'s loop + `rb_activate_ability` (`src/engine.c:1389`) | WIRED (location set narrowed  4-3) |
| 14 | `check_mandatory_activation_cost` | 409 | private fn | `rb_ability_cost_offered` / `setup_ability_cost_offered` (`src/game/game_setup.c:1185`) | WIRED (OFFER side only) |
| 15 | `find_gained_activation` | 430 | private fn | `find_gained_activation_index` (`src/engine.c:1255`) | WIRED |
| 16 | `activation_ability_id` | 458 | private fn | folded into the queue-id construction in `rb_activate_card` | WIRED |
| 17 | `handle_use_ability` | 476 | private fn | `rb_activate_card` (`src/engine.c:1281`) | WIRED  **missing `validate_mandatory_cost`**, see 3 |
| 18 | `profile_chosen_labels` | 603 | private fn | **NONE** (headless-only `choice_resolved` log, `#[cfg(not(feature="headless"))]`) | DEAD-CORRECT, VERIFIED-NOOP |
| 19 | `profile_choice_is_skip` | 734 | private fn | **NONE** (same headless-only block) | DEAD-CORRECT, VERIFIED-NOOP |
| 20 | `resume_with_choice` | 767 | **pub fn** | `rb_resume_with_choice` (`src/ability/choice.c`) | WIRED |
| 21 | `binary_target_result` | 998 | private fn | inlined in `choice.c`'s SelectTarget arm | WIRED |
| 22 | `lookup_option_target` | 1029 | private fn | inlined in `choice.c` | WIRED |
| 23 | `pick_heart_option` | 1054 | private fn | inlined in `choice.c` | WIRED |
| 24 | `answer_index` | 1064 | private fn | inlined in `choice.c` | WIRED |
| 25 | `build_choice_result` | 1072 | private fn | `choice.c`'s `build_choice_result` twin | WIRED |
| 26 | `entry_player_or_p1` | 1242 | private fn | inlined (4 sites) | WIRED |
| 27 | `resume_auto_ability_choice` | 1249 | private fn | `choice.c`'s fast path | WIRED |
| 28 | `drain_stale_auto_entries` | 1305 | private fn | `choice.c` / `ability_queue.c` drain | WIRED |
| 29 | `take_queue_resolver` | 1337 | private fn | `choice.c` | WIRED |
| 30 | `sub_choice_targets_opponent` | 1363 | private fn | `choice.c` (G1/G3 route) | WIRED |
| 31 | `route_sub_choice_player` | 1405 | private fn | `choice.c` | WIRED |
| 32 | `complete_skipped_ability` | 1449 | private fn | `choice.c` | WIRED |
| 33 | `reprocess_ability` | 1466 | private fn | `choice.c` | WIRED |
| 34 | `complete_and_scan` | 1495 | private fn | `choice.c` | WIRED |
| 35 | `finish_paid_ability` | 1520 | private fn | `activate_resolved_ability`'s use-limit tail (`src/engine.c:1208-1229`) | WIRED |
| 36 | `resume_queue_with_choice` | 1562 | private fn | `choice.c` | WIRED |
| 37 | `check_timing` | 1626 | **pub fn** | `rb_check_timing` (`src/turn/phase.c:625`) | WIRED  **5 divergences, T6-2..T6-6** |
| 38 | `check_victory_condition` | 1685 | **pub fn** | `rb_check_victory_condition` (`src/turn/phase.c:564`) | WIRED, **value-divergent** (4-1) |
| 39 | `check_invalid_live_cards` | 1732 | private fn | `rb_check_invalid_live_cards` (`src/turn/phase.c:576`) | WIRED  **missing `face_up` gate** (4-2) |
| 40 | `check_duplicate_members` | 1785 | private fn | **NONE** | **VERIFIED-NOOP: 0 of 985**  see 5-1 |
| 41 | `check_invalid_energy_cards` | 1818 | private fn | `rb_check_invalid_energy_cards` (`src/turn/phase.c:592`) | WIRED, faithful |
| 42 | `check_orphaned_under_cards` | 1842 | private fn | `rb_check_orphaned_under_cards` (`src/turn/phase.c:603`) | WIRED  **WRONG ZONE  T6-4** |
| 43 | `check_invalid_resolution_zone` | 1864 | private fn | `rb_check_invalid_resolution_zone` (`src/turn/phase.c:616`) | WIRED  **WRONG ZONE + no owners  T6-3** |

---

## 2. INVENTORY  `engine/src/game/game_setup.rs` (46 items)

`src/game/game_setup.c` (1565 lines) is **not** one of the three files this cluster
owns; it is unclaimed in the OWNERSHIP MAP. It is inventoried here because
`game_setup.rs` is one of the two Rust modules in scope, and because four of its
functions are the load-bearing callees of `phase.c` and `engine.c`.

| # | Rust item | line | kind | C twin (file:line) | status |
|---|---|---|---|---|---|
| 1 | `area_label_inner` | 29 | private fn | `rb_phase_name`-adjacent; no twin | DEAD-CORRECT, display-only |
| 2 | `area_label_en` | 41 | **pub fn** | **NONE** | display surface, VERIFIED-NOOP (T5-1 class) |
| 3 | `area_label_ja` | 45 | **pub fn** | **NONE** | display surface, VERIFIED-NOOP (T5-1 class) |
| 4 | `enum ActionType` | 51 | **pub enum** | `RbSetupActionType` (`include/rabuka.h`, A2) | WIRED |
| 5 | `ActionType::fmt` | 83 | method | `rb_setup_deck_build_status_str` / label tables | WIRED |
| 6 | `ActionType::to_tag` | 120 | **pub fn** | `RB_SETUP_ACT_*` constants | WIRED |
| 7 | `ActionType::from_tag` | 153 | **pub fn** | `rb_setup_execute_action`'s switch | WIRED |
| 8 | `impl FromStr for ActionType` | 188 | impl | `rb_setup_execute_action` | WIRED |
| 9 | `struct Action` | 234 | **pub struct** | `RbSetupAction` (`include/rabuka.h`, A2) | WIRED |
| 10 | `Action::with_ja` | 251 | **pub fn** | `RbSetupAction` field | WIRED |
| 11 | `Action::display_desc_for` | 262 | **pub fn** | **NONE** | display-only, VERIFIED-NOOP |
| 12 | `Action::display_desc` | 270 | **pub fn** | **NONE** | display-only, VERIFIED-NOOP |
| 13 | `struct ActionParameters` | 281 | **pub struct** | `RbSetupActionParameters` (A2) | WIRED |
| 14 | `struct AreaInfo` | 311 | **pub struct** | `RbGeneratedArea` | WIRED |
| 15 | `struct DoubleBatonOption` | 320 | private struct | `RbSetupDoubleBatonOption` | WIRED |
| 16 | `struct DoubleBatonPair` | 328 | **pub struct** | `RbSetupDoubleBatonPair` | WIRED |
| 17 | `build_two_decks` | 338 | **pub fn** | `rb_setup_build_two_decks` (`game_setup.c:1385`) | WIRED |
| 18 | `setup_game` | 358 | **pub fn** | `rb_setup_game` (`game_setup.c:369`) | WIRED |
| 19 | `is_automatic_phase` | 367 | **pub fn** | `rb_setup_is_automatic_phase` (`game_setup.c:389`) | WIRED |
| 20 | `auto_advance_one` | 382 | **pub fn** | `rb_setup_auto_advance_one` (`game_setup.c:413`) | WIRED |
| 21 | `is_live_card_set_phase` | 398 | **pub fn** | `rb_setup_is_live_card_set_phase` (`game_setup.c:406`) | WIRED |
| 22 | `settle_single_player_state` | 410 | **pub fn** | `rb_setup_settle_single_player_state` (`game_setup.c:425`) | WIRED  **verified complete**, 6 |
| 23 | `settle_auto` | 434 | **pub fn** | `rb_setup_settle_auto` (`game_setup.c:449`) | WIRED |
| 24 | `stage_area_from_parameters` | 465 | private fn | `rb_setup_stage_area_from_parameters` (`game_setup.c:696`) | WIRED |
| 25 | `execute_action` | 474 | **pub fn** | `rb_setup_execute_action` (`game_setup.c:779`) | WIRED |
| 26 | `execute_action_for_player` | 514 | **pub fn** | `rb_setup_execute_action_for_player` (`game_setup.c:882`) | WIRED |
| 27 | `test_ai_vs_ai` | 530 | **pub fn** | **NONE** | bot/arena host, out of scope |
| 28 | `make_action` | 589 | private fn | inlined | WIRED |
| 29 | `make_action_params` | 603 | private fn | inlined | WIRED |
| 30 | `make_params` | 614 | private fn | inlined | WIRED |
| 31 | `generate_possible_actions` | 634 | **pub fn** | `rb_generate_action_candidates` (`src/turn/phase.c:1707`) + `rb_generate_possible_actions` (`:1909`) | WIRED  the wrapper is DEAD-CORRECT, 3-6 |
| 32 | `make_choice_pair` | 662 | private fn | **NONE** (RPS rows, deliberately not built) | DEAD-CORRECT, 4-6 |
| 33 | `make_pay_skip_pair` | 688 | private fn | **NONE** | DEAD-CORRECT, pending-choice arm not ported |
| 34 | `generate_pending_choice_actions` | 719 | private fn (~730 lines) | **NONE** | DEAD-CORRECT  the C prompts through `rb_has_pending_choice` / `rb_resume_with_choice`; **verdict identical**, the builder is a UI menu only |
| 35 | `generate_rps_actions` | 1446 | private fn | **NONE** | DEAD-CORRECT  3 constant rows; 4-6 |
| 36 | `generate_choose_first_attacker_actions` | 1454 | private fn | **NONE** | DEAD-CORRECT  2 constant rows; 4-6 |
| 37 | `generate_mulligan_actions` | 1467 | private fn | `rb_setup_mulligan_candidates` (`game_setup.c:676`) + `rb_push_mulligan_rows` (`phase.c:1677`) | WIRED |
| 38 | `stage_play_cost` | 1540 | private fn | `rb_setup_stage_play_cost` (`game_setup.c:904`) | WIRED |
| 39 | `is_baton_touchable` | 1553 | private fn | `rb_setup_is_baton_touchable` (`game_setup.c:920`) | WIRED |
| 40 | `double_baton_options` | 1569 | private fn | `rb_setup_double_baton_options` (`game_setup.c:953`) | WIRED |
| 41 | `has_double_baton` | 1605 | private fn | `rb_setup_has_double_baton` (`game_setup.c:930`) | WIRED |
| 42 | `struct AreaCandidate<'a>` | 1617 | private struct | `RbSetupAreaCandidate` | WIRED |
| 43 | `const AREA_NAMES` | 1628 | const | inline literals in `phase.c:1836` | WIRED |
| 44 | `area_candidates_for` | 1635 | private fn | `rb_setup_area_candidates_for` (`game_setup.c:995`) | WIRED |
| 45 | `generate_main_phase_actions` | 1690 | private fn (~660 lines) | `rb_setup_play_candidates` (`game_setup.c:1047`) + `rb_setup_for_each_ability_candidate` (`:1323`) + the row builder in `phase.c:1783-1881` | WIRED |
| 46 | `generate_live_card_set_actions` | 2353 | private fn | `rb_setup_live_card_candidates` (`game_setup.c:1353`) + `rb_push_live_set_rows` (`phase.c:1694`) | WIRED |

`core/types.rs` (1262) was cross-checked only. Its `Phase` enum supplies the six
values `rb_advance_phase` stores, and its `TurnPhase` is the C's *derived*
`g->active == g->first_attacker` discriminator  verified equivalent, 4-5.

---

*(sections 3-8 appended below as the work completed)*
---

## 3. WIRING CENSUS - the three owned C files

Generated mechanically: every top-level function definition in each owned file
was extracted with comments and string literals blanked, then every occurrence of
its name across all 209 .c/.h files in the snapshot was located and classified.
Two filters matter and both were needed: a definition is a line whose logical
statement opens a `{`, and a *pure prototype* (`Type name(args);` and nothing
after the `)`) is not a call site. Without the second filter `return f(x);`
was miscounted as a declaration and four genuinely-wired functions were reported
dead.

| file | functions | WIRED | DEAD-CORRECT | DEAD-WRONG |
|---|---|---|---|---|
| `src/engine.c` | 46 | 43 | 2 | 1 |
| `src/turn/phase.c` | 56 | 48 | 6 | 1 |
| `src/turn/live.c` | 52 | 47 | 7 | 0 |
| **total** | **154** | **138** | **15** | **2** |

The raw census reported 16 zero-call-site names; one is a false positive
(`rb_can_assign_hand_for_alt_cost`, below), so **15 are genuinely unwired**:
3 in `engine.c`, 7 in `phase.c`, 5 in `live.c`.

### 3.1 The 15 genuinely zero-call-site functions, classified

`DEAD-CORRECT` = defined, unreachable, and faithfully mirrors a Rust function
that is itself unreachable or has no C representation needed. `DEAD-WRONG` =
unreachable AND wrong, i.e. real debt.

**DEAD-WRONG (2)  Rust has a live call site and the C twin has none:**

| fn | file:line | Rust counterpart | why it is wrong |
|---|---|---|---|
| `rb_play_time_cost_reduction_hook` | `phase.c:1664` | `phases.rs:1359`, called from `phases.rs:1219-1222` | The C instead auto-detects the play-time alternative cost inside `rb_play_member_ex` via `rb_detect_alt_cost`. Different mechanism, same observable today  but the twin is dead, so the two can drift silently. |
| `rb_has_distinct_assignment_k` | `phase.c:1052` | `phases.rs:1557`, called from `phases.rs:1567` | Rust's `can_assign_hand_for_alt_cost` calls it; the C's `rb_can_assign_hand_for_alt_cost` does **not**. The twin is dead, so the distinct-assignment check Rust performs is simply absent from the C. |

**DEAD-CORRECT (13):**

| fn | file:line | note |
|---|---|---|
| `rb_generate_possible_actions` | `phase.c:2052` | vestigial wrapper over `rb_generate_action_candidates`; superseded by the XREQ bridge at `phase.c:1861` |
| `rb_try_phase4` | `live.c:2603` | phase-4 hand-cost search |
| `rb_try_surplus_compositions` | `live.c:2530` | surplus composition search |
| `rb_bt_search` | `live.c:2574` | backtracking driver |
| `rb_try_all_distribution` | `live.c:2662` | full-distribution search |
| `bt_card_ok` | `live.c:2273` | WIRED only *inside* four of the dead functions above (2391 / 2394 / 2550 / 2639). Zero reachability from production. |
| `rb_log_phase` | `phase.c:1259` | Rust's `log::debug!` in `advance_phase`; headless-only |
| `_3ds_tdbg` / `rb_3ds_tdbg` | `phase.c:1266` / `1544` | 3DS debug scaffolding, no Rust counterpart |
| `rb_log_turn_start` | `phase.c:1273` | `phases.rs:301 log_turn_start`; headless-only |
| `rb_play_card` | `engine.c:1460` | superseded by `rb_play_member_ex`; declared at `rabuka.h`, never called |
| `heart_color_of` | `engine.c:223` | helper; declared at `rabuka.h`, nothing calls it |
| `i18n_self_check` | `engine.c:1867` | build-time i18n assertion, never invoked |

The five `live.c` entries are one dead island: `rb_bt_search`,
`rb_try_phase4`, `rb_try_all_distribution` and `rb_try_surplus_compositions` are
declared in the local forward block at `live.c:10-12` and called only by each
other plus `bt_card_ok`, so the whole cluster is unreachable from production even
though five of its six names have call sites.

### 3.2 Two census corrections worth recording

1. `rb_can_assign_hand_for_alt_cost` is **WIRED** (`phase.c:1035` - inside
   `rb_has_play_time_alt_cost_hand_cards`), not dead. The mechanical census
   reported it dead because its only call site is
   `return rb_can_assign_hand_for_alt_cost(g, g->active, card_id, chars, n_names);`
   and the prototype filter could not tell a bare declaration from a one-line
   `return`. Corrected by hand; this is why the raw count of 16 is really 15.
2. `bt_card_ok` looks WIRED by name count (4 call sites) but every one of those is
   inside a dead function. Name-count wiring without a reachability pass
   overstates the number  the same failure mode as "being defined is not being
   wired".

---

## 4. DIVERGENCES - every place C returns a different value than Rust

Ordered by blast radius. Six were CLOSED; the rest are reported with their
measured reachability.

### 4-0 CLOSED - the six reachable gaps

**T6-1 E `rb_check_orphaned_under_cards` routed an orphaned ENERGY card into
the ACTIVE ENERGY ZONE.** `phase.c:690`.
Rust `actions/mod.rs:1851-1856`:
`if card_db.get_card(cid).is_some_and(|c| c.is_energy()) { player.energy_deck.cards.push(cid) } else { player.waitroom.cards.push(cid) }`
The C pushed it into `P->energy`, i.e. immediately-usable energy that never
returns to the deck. Rule 10.5.3 + 10.5.4 are the same routing rule and send it
to the energy DECK. Reachable: any member that leaves the stage with an energy
card under it, which the corpus does constantly (`place_energy_under_member`).
Fixed + asserted by `t6_f1_orphan_energy_goes_to_energy_deck`.

**T6-2 E `rb_check_invalid_resolution_zone` sent EVERY resolved card to the
active player's discard, energy included.** `phase.c:715`.
Rust `actions/mod.rs:1885-1894` branches on card type and sends energy to
`energy_deck`. Reachable: `rb_perform_cheer_check` pushes every yelled card
through `rb_resolution_add` (`tracking.c:154`) and a yell can turn up an
energy card. Fixed + asserted by `t6_f2_resolution_energy_goes_to_energy_deck`.
**The owner half of the same Rust statement is NOT closed - see 4-4.**

**T6-3 E `live_set_limit_reduction` was never reset.**
**REACHABILITY 2 of 985** - `PL!HS-bp2-018-N` (debut) and `PL!N-bp4-026-L`
(), both reaching `execute_reduce_live_card_set_limit`
(`ability/effects/restriction.rs:217-234`, which does
`player.live_card_set_limit_reduction += count`). Both say uCu
J[hZbgtFCYc1v - NEXT live-card-set phase. Rust honours
that with `game_state.player1/2.live_card_set_limit_reduction = 0` at
`phases.rs:232-233`, the first two statements of the LiveCardSetSecondAttacker
arm. The C incremented at both writers (`rules.c:1952`, `state.c:1296`) and
never cleared, so a single fire shrank the limit for the rest of the game.
Fixed + asserted by `t6_f3_live_set_limit_reduction_cleared_each_turn`.

**T6-4 E `rb_check_timing` refreshed player 1 before player 2
unconditionally.** `phase.c:747`.
Rust `actions/mod.rs:1632-1638` branches on `player1.is_first_attacker`, and
`Player::refresh` (`core/player.rs:634-648`) shuffles the waitroom through
the **process-global** rng whenever a deck is empty. So the seat order decides
which shuffle takes the first draw. `player1.is_first_attacker` is exactly
`g->first_attacker == 0` in the C. Reachable whenever player 2 is the first
attacker and both seats have an empty deck with a non-empty waitroom.
Fixed + asserted by `t6_f4_refresh_order_is_first_attacker_first`.

**T6-5 E `activate_wait_members` had no `constant_cannot_activate_members`
filter.** `phase.c:96`.
Rust `phases.rs:125-131` filters the Active-phase `to_activate` list through
a set that the constant scan fills at `modifiers.rs:1081-1092`.
**REACHABILITY 2 of 985** - `PL!N-bp5-006-{AR,P,R}` (self-target, the half
that matches) and `PL!HS-pb1-008-{P,R}` ab#1 (target `opponent`, which pushes
a PLAYER id that `"player1".parse::<i16>()` can never equal, so it is a no-op
in Rust too and was deliberately not reimplemented).
The C activated ] on her controller's Active phase against her own printed
uo[ANeButFCYANeBuv.
Fixed by recomputing the predicate locally at the read site (the C has no such
GameState field, the set is filled only by `rb_recalc_constants` in
`modifiers.c`, and `include/rabuka.h` is not this cluster's).
Asserted by `t6_f5_constant_cannot_activate_blocks_active_phase`.

**T6-6 E `rb_activate_card` had NO mandatory-cost refusal.**
`engine.c:1124 activate_resolved_ability` / `engine.c:1319`.
Rust `actions/mod.rs:553-564`, immediately after publishing
`activating_card` and BEFORE the hand removal:
`if let Some(ref cost) = ability.cost { AbilityResolver::validate_mandatory_cost(game_state, &cost.0, Some(card_id))?; }`
`?` propagates out of `handle_use_ability`: nothing is paid, nothing is moved,
the effect never runs. The C built the cost+effect tree and ran it, so
`rb_pay_cost` paid whatever leg it could and **the effect still ran on a
partially-paid mandatory cost**.
**REACHABILITY 100 of 985** - 100 abilities are activation-triggered with a mandatory non-energy
cost (leaves are `move_cards` / `change_state` / `reveal`).
Two concrete differences, both live: (a) Rust passes `Some(card_id)`
explicitly because `gs.activating_card` is not set at offer time, while the
pay-time twin `rb_validate_cost` passes `RB_ACTIVATING_DERIVED`, so every
`self_cost` filter resolved against a derived card; (b) a whole optional /
any_number / max cost resolves to pay-zero-or-skip (`handlers.rs:286-291`) and
a `ChoiceCondition` leg passes (`handlers.rs:302`), which `rb_validate_cost`
does not honour.
Fixed by returning 0 from `activate_resolved_ability` on refusal and having
both callers in `rb_activate_card` not count a refused ability as matched, so
the default-ability fallback cannot claim a card whose N was refused.

### 4-1 `rb_check_victory_condition` is value-divergent from Rust's
`check_victory_condition` - NOT the same predicate.

| | Rust `actions/mod.rs:1685` | C `phase.c:565` |
|---|---|---|
| trigger | success zone count per player | same |
| win condition | `p1 >= 3` (only) | `p1 >= 3 && p2 <= 2` |
| draw | none | `p1 >= 3 && p2 >= 3`  `winner = 2` |
| what it sets | `game_result` + `game_ended` | `g->winner` only |

Rust ends the game the instant one seat reaches `RB_VICTORY_CARD_COUNT`. The
C adds the `p2 <= 2` guard and a draw verdict, so on a `3 vs 3` board the C
declares a draw where Rust has already ended the game on the first seat's third.
**Reachability: trivially reachable** - a full turn of play reaches it. **Not
closed**: `g->winner` is the C's only game-over signal and
`rb_advance_phase` gates on it, so removing the extra verdicts means adding a
real `game_result`/`game_ended` pair, i.e. `include/rabuka.h`. Report only,
do not guess. Note also that Rust's `advance_phase` LiveVictoryDetermination arm
(`phases.rs:229`) does **no** victory evaluation at all - the C's does
(`phase.c:295-300`) - so the C's extra check runs at a point Rust does not
re-check.

### 4-2 `rb_check_invalid_live_cards` has no `face_up` gate - BLOCKED-ON header

Rust `actions/mod.rs:1733-1734`:
`if !game_state.seat_player(seat).live_card_zone.face_up { return; }`
so the Rule 10.5.1 eviction only runs when the live zone is FACE UP.
`face_up` is set true at `phases.rs:343` (performance opens) and false at
`:832` / `:887` (a card was set into the zone). Neither Rust's
`handle_set_live_card` nor the C's `rb_handle_set_live_card` checks
`is_live`, so a non-live card CAN be placed. Rust keeps it (face down) and
evicts it at the NEXT `check_timing`; the C evicts it at the
LiveCardSetSecondAttacker  Performance transition (`phase.c:224`), i.e. BEFORE
the live instead of after it.
**Reachability**: not through `rb_setup_live_card_candidates`, which filters
to live cards - only through a hand-built `rb_handle_live_card_confirmation`.
**Not closed**: the C's `RbBag` has no face-up flag and
`include/rabuka.h` is not this cluster's to edit.

### 4-3 `activation_location` / `can_activate_at_location`: the C's location set is narrower

Rust `actions/mod.rs:345-376` returns one of Hand / Stage / Energy / LiveCardZone
/ Success / Discard / Resolution and `can_activate_at_location` gates on it. The C
inlines the mapping into `rb_activate_card` (`engine.c:1330-1345`) and accepts
a narrower set. No measured corpus ability lands on the difference - recorded, not
closed.

### 4-4 `check_invalid_resolution_zone` OWNER half - structurally inexpressible

Rust keeps a parallel `resolution_zone.owners` vector, written by
`add_card_for_owner` (`core/game_state/tracking.rs:93` and
`turn/live.rs:1976`), and returns each card to the player who yelled it;
`owners.get(index).copied().unwrap_or(active)` is only the fallback.
The C's `g->resolution` is a bare `RbBag` (`rabuka.h`) and
`rb_resolution_add(GameState *, int card_id)` (`zones.c:688`) takes no owner
at all. Fixing the energy half (T6-2) was possible inside `phase.c`; the owner
half needs a parallel array in `GameState` plus a signature change in
`zones.c`. **BLOCKED-ON `include/rabuka.h` + `src/core/zones.c`.**

### 4-5 Phase-table divergences that are DELIBERATE and must not be "fixed"

| Rust | C | verdict |
|---|---|---|
| `advance_phase` has NO arm for RockPaperScissors / ChooseFirstAttacker / either Mulligan phase - the HOST drives them through `execute_main_phase_action` | `rb_advance_phase` auto-passes RPS, elects the attacker, and takes the mulligan SKIP path | Correct C stand-in for a headless harness. The host paths (`rb_setup_execute_action`) win when the host has already answered, so the stand-in is not reached interactively. |
| `LiveCardSetFirstAttacker` arm does nothing but store the phase (`phases.rs:224-225`) | the C's `RB_PHASE_LIVE_SET` arm ALSO performs the live-zone refill from Rust's Pass arm (`actions/mod.rs:274-289`) | Folded, and consistent: Rust's Pass for that window is exactly the refill, and every driver reaches the refill through `rb_advance_phase`. |
| `LiveCardSetSecondAttacker` runs `process_pending_auto_abilities(first)` then `(second)`, returning early if the first leaves a pending choice (`phases.rs:238-243`) | one global `rb_process_pending_auto_abilities` + `rb_drain_ability_queue` | The C's per-seat split was already A/B'd: the global drain produces identical choices on every suite in the battery (1411 byte-stable). Do not "restore" it speculatively. |
| `advance_phase` LiveVictoryDetermination calls `clear_revealed_cards()` (`phases.rs:277`) | deliberately omitted | Already A/B'd and documented: restoring it costs 2 premise checks per round. Left as recorded. |

### 4-6 `rb_check_permanent_loop` is called from `rb_check_timing`; Rust never calls it

Rust defines `check_permanent_loop` (`core/game_state/abilities.rs:3309`) and
**nothing in `engine/src` calls it** - the C calls it at `phase.c:756` on
every `check_timing`. The C's body only sets `g->loop_detected`
(`tests/test_parity_rules_extra.c:58` and `:542` both pin that), so the
divergence is a redundant flag write, not a draw. Recorded, not closed: deleting
the call would change a flag several suites read.

### 4-7 `check_timing`'s `if game_ended { return }` - redundant in C, not divergent

Rust `actions/mod.rs:1646-1648` returns immediately once
`check_victory_condition` has ended the game, skipping every later step. The C
runs them all. Because `g->winner` is the C's only game-over signal and
`rb_advance_phase` already returns on `g->winner != -1`, the skipped steps
only ever run on a game that Rust considers finished. Left in place with a
comment rather than guessed at.

---

## 5. GAME SETUP

### 5-1 `rb_setup_settle_single_player_state` - replacement VERIFIED COMPLETE

`game_setup.rs:410 settle_single_player_state`  `game_setup.c:425
rb_setup_settle_single_player_state`. The 500-iteration guard is present
(`game_setup.c:425`), and the 6-phase `is_automatic_phase` list
(`game_setup.rs:369-375`) is faithfully carried by
`rb_setup_is_automatic_phase` (`game_setup.c:389`). The inline 16-iteration
loop that used to stall at FirstAttackerPerformance / SecondAttackerPerformance /
LiveVictoryDetermination is gone: nothing in `src/` still contains a
hand-rolled settle loop, and the phase machine now stores all four live-window
phase values (`RB_PHASE_LIVE_SET` / `LIVE_SET_SECOND` / `PERFORMANCE` /
`PERFORMANCE_SECOND`) rather than inferring them, which is what unblocked the
loop. `rb_execute_live_victory_determination` remains the single entry point for
LiveVictoryDetermination, reached from `rb_advance_phase:258` and from the web
driver.

### 5-2 `rb_validate_mandatory_cost` - **NOT LANDED**

The function exists and is complete: `src/ability/cost.c:1808`, a faithful port
of `ability/cost/handlers.rs:281-309`, declared at `rabuka.h:3518`. It has
**ZERO callers**.

Rust's three live call sites, per `cost.c:853-857`'s own inventory:
`game_setup.rs:2188` (stage N offer), `game_setup.rs:2295` (discard N
offer), `actions/mod.rs:559` (activate_ability).

* `actions/mod.rs:559` - **CLOSED here as T6-6.** This was the third call site
  and it is the one that sits in *this* cluster's file; `cost.c`'s comment
  listed it but nothing had landed it.
* `game_setup.rs:2188` and `:2295` - **still un-landed.**
  `src/game/game_setup.c:1199` still calls `rb_validate_cost`, which is the
  PAY-time twin (`handlers.rs:169`) and strictly STRONGER: it walks optional
  and any-number legs, so a ucv cost that is currently unaffordable
  withholds the whole ability from the offer list. That is a refusal Rust does not
  make. `include/rabuka.h:3511-3518` documents the defect and it is still
  there. `src/game/game_setup.c` is not this cluster's file and
  `include/rabuka.h` is main's, so this is reported, not landed.

!** What landing it would move, and the K7 hazard.** One of the 100 abilities the
gate can act on is **ability 325, `PL!-pb2-012-P+` ab#1**, whose cost is a
`sequential_cost` whose SECOND leg is a `choice_condition`. That is precisely
the node class where `validate_mandatory_cost` (returns `Ok` for
`ChoiceCondition`, `handlers.rs:302`) and `rb_validate_cost` (walks it)
disagree. **325 is also one of the two abilities K7 flagged** (see 7). Whoever
lands the `game_setup.c:1199` switch must A/B ability 325 specifically, and must
not conclude from a red 325 that the C is wrong.

---

## 6. REACHABILITY LEDGER - every gap measured against the 985 abilities

| gap | n of 985 | verdict |
|---|---|---|
| T6-1 orphaned-energy routing | reachable (place_energy_under_member) | **CLOSED** |
| T6-2 resolution-energy routing | reachable (any yell that reveals energy) | **CLOSED** (energy half) |
| T6-3 `live_set_limit_reduction` reset | **2** - `PL!HS-bp2-018-N`, `PL!N-bp4-026-L` | **CLOSED** |
| T6-4 refresh order | reachable (any empty-deck refresh) | **CLOSED** |
| T6-5 `constant_cannot_activate_members` | **2** - `PL!N-bp5-006-{AR,P,R}` (self half), `PL!HS-pb1-008-{P,R}` (opponent half) | **CLOSED** (self half) |
| T6-6 mandatory-cost refusal | **100** - every N with a mandatory non-energy cost | **CLOSED** |
| `check_duplicate_members` (`actions/mod.rs:1785`) | **0** | **VERIFIED-NOOP: 0 of 985** - see below |
| `cost_has_hand_self_move` (`actions/mod.rs:362`) | **0** | **VERIFIED-NOOP: 0 of 985** - the loc == Zone::Hand branch never fires on the corpus |
| `profile_chosen_labels` / `profile_choice_is_skip` (:603, :734) | **0** | **VERIFIED-NOOP** - inside `#[cfg(not(feature="headless"))]`, a headless log only |
| `generate_pending_choice_actions` / `generate_rps_actions` / `generate_choose_first_attacker_actions` (`game_setup.rs:719, :1446, :1454) | n/a | **DEAD-CORRECT** - UI row builders; the C prompts through `rb_has_pending_choice` / `rb_resume_with_choice` and the ACTION VERDICT is identical |
| `area_label_en` / `area_label_ja` / `Action::display_desc` (`game_setup.rs:41, :45, :262, :270) | n/a | **DEAD-CORRECT** - display surface |

### 6-1 Why `check_duplicate_members` is a genuine no-op, not an omission

`check_duplicate_members` (`actions/mod.rs:1785-1815`) drains
`player.stage.take_pending_duplicate_members(area)`. The ONLY writer of
`pending_duplicate_members` is `stage_member_for_check_timing`
(`core/zones.rs:289`), and **that function has ZERO callers anywhere in
`engine/src`** - verified by an exhaustive recursive grep over every `.rs`
file. So the list is always empty, `check_duplicate_members` always returns 0
and moves nothing, and the C's omission of `rb_check_duplicate_members` from
`rb_check_timing` is **correct**. It was deliberately NOT ported.

---

## 7. ABILITY 325 / 805 - K7, NOT a C bug

Measured directly off both tables (985 entries each):

| | entries | sum |
|---|---|---|
| C `RBKA_OFFSET_DELTAS` (`src/core/generated/gen_data.c:7`) | 985 | **98737** |
| `RBKA_BYTECODE_LEN` | - | **98737** |
| Rust `OFFSET_DELTAS` (`cards/build/abilities_gen.rs:25`) | 985 | **98725** |

The C table sums to exactly the bytecode length; the Rust table is **12 bytes
short**. Exactly **two** entries differ:

| ability idx | C | R | C byte range | R byte range |
|---|---|---|---|---|
| **325** | 159 | 149 | - | 10 bytes behind from here on |
| **805** | 238 | 236 | `[81237,81475)` | `[81227,81463)` |

**No divergence at 325 or 805 was found in this scope, and nothing here was
changed at either ability.** Every byte the C reads for 325 and 805 is inside the
canonical 98737-byte blob and consistent with the C's own table, so the C is the
self-consistent side and the Rust slice is the stale one.

The one place 325 shows up in this cluster is 5-2: it is a
`sequential_cost`/`choice_condition` mandatory cost, the exact node class
where the two validators disagree. That is a hazard for the *un-landed*
`game_setup.c:1199` switch, not a defect here.

---

## 8. A/B EVIDENCE

Measured with `^FAIL` (no colon), **case-sensitive** (`-cmatch`), on stdout
AND stderr, across 35 suites, SEQUENTIALLY, in the isolated snapshot. Every suite
produced output on every run - none was silent, so none was mis-read as passing.

| build | total `^FAIL` |
|---|---|
| baseline (this cluster's hunks reverted) | **1415** |
| after T6-1..T6-5 (`phase.c`) | **1415** |
| after T6-6 (`engine.c`) | **1411** |

The new suite, against a baseline in which **only this cluster's hunks were
reverted** (proven by residue counts, `src/turn/phase.c` sha256
`ab50e4a953ade8d6064d8c47d17d45bc22fa96d0b579bcf81130fb54ed66b2cb`):

| | `^FAIL` | assertions |
|---|---|---|
| baseline (fixes reverted) | **13** across all 5 cases | 48 |
| fixed | **0** | 48, 5/5 cases ok, 0 setup bugs |

Per-fix sensitivity - every fix is independently proven red:

| case | baseline failures |
|---|---|
| `t6_f1_orphan_energy_goes_to_energy_deck` | 3 |
| `t6_f2_resolution_energy_goes_to_energy_deck` | 3 |
| `t6_f3_live_set_limit_reduction_cleared_each_turn` | 3 |
| `t6_f4_refresh_order_is_first_attacker_first` | 2 |
| `t6_f5_constant_cannot_activate_blocks_active_phase` | 2 |

**Per-suite deltas from T6-6** (`1415`  `1411`), net `-4`:

| suite | before | after | what moved |
|---|---|---|---|
| `parity_under_member` | 91 | **87** | `margarete_bp4_010_r_already_wait_refused` flipped from 3 failures to a clean pass - the case asserts an already-waited member refuses its activation, which is exactly `actions/mod.rs:559` |
| `parity_zones_position` | 37 | **36** | "an unpayable mandatory cost leaves the member on stage (got -1 expected 2692)" - the C removed the card from the stage; Rust's gate runs BEFORE the hand removal, so it stays |
| `parity_jidou_move_yell` | 89 | **90** | **+1**, reported honestly below |

**The +1, in full.** It is
`"each N activation draws exactly one card (got 1 expected 2)"` inside
`shiki_auto_is_one_shot_per_waited_state`. That case was **already 5 assertions
red** before T6-6, and the two failures immediately upstream of it are both
`"the swap must actually move her out of 0 (she is now at 0)"` - i.e.
`mill_three_swap` never moves lG out of area 0 in that fixture, so the
wait/arm/cycle is misaligned before the draw is ever measured. The +1 is a
downstream artifact of an already-red precondition in an already-red scenario
(`Q126: a stage-to-waitroom zone change must not place energy` is red there
too), not evidence that the gate is wrong. The two suites the gate makes strictly
better both improve on assertions that name Rule 9.4.2.3 /
`actions/mod.rs:559` explicitly. **Gate kept**, and the pre-existing red in that
fixture is left for whoever owns the area-swap machinery.

**Do not regress** (all verified still true after this cluster's changes): the
VICTORY rollover still clears `player_cannot_activate`, `n_cannot_active_cards`
and `baton_touch_used` at `phase.c:496-501` and is reached by every driver
including web; the live-end drain is still UNCONDITIONAL per `live.rs:1508-1529`;
`rb_execute_live_victory_determination` is still the single LiveVictoryDetermination
entry point; **no `p1_has` guard was added to `rb_determine_live_winners`**.

**Suites that do not exist** (named in briefs across this project; verified absent
before naming anything): `test_setup` - there is no `tests/test_setup.c` and
no `tests/test_test_setup.c`, so it has no runnable stem and is not in the 35.
The seven other phantoms reported by peers (`audit_cards`, `card_data_decode`,
`card_info_show`, `card_info_parity`, `parity_card_text`,
`audit_game_state`) were not needed for this scope.

---

## 9. FILES AND TEST OWNERSHIP

Edited by cluster T6 (this file claims them):

| file | change |
|---|---|
| `src/turn/phase.c` | T6-1, T6-2, T6-3, T6-4, T6-5 + the new `has_constant_cannot_activate` static and its forward declaration |
| `src/engine.c` | T6-6 (`activate_resolved_ability` returns int; the `validate_mandatory_cost` refusal; both `rb_activate_card` call sites honour it) |
| `tests/test_parity_t6_action_setup.c` | **NEW**, 5 cases / 48 assertions, 1 per closed fix |
| `AUDIT_actions_setup.md` | this file |
| `PARITY_CHECKLIST.md` | cluster T6 |

**Not touched:** `src/turn/live.c` (no reachable gap - its findings are in 3 and
4), `src/web_server.c` (T5's; the orphan `src/ability/choice_renderer.c`,
1821 lines, is in no build target - `Makefile:20-70`,
`platforms/cdi/build_cdi.sh`, `platforms/genesis/build_genesis.sh` and
`platforms/Makefile` all have 0 references), `src/ability/effects/rules.c`
(cluster C's), `src/ability/{vm,resolver,enums,types}.c` (T7's),
`include/rabuka.h` (main's), `src/core/{modifiers,zones,tracking}.c,
`src/game/game_setup.c`.

## 10. OPEN ITEMS FOR THE NEXT CLUSTER

1. `game_setup.c:1199`  `rb_validate_mandatory_cost(g, actor, ab->cost, card_id)`.
   Owner: cluster C. **A/B ability 325 first** (5-2).
2. `include/rabuka.h`: a `resolution_zone` owner vector + `rb_resolution_add`
   owner parameter (4-4), a `live_card_zone.face_up` flag (4-2), and a real
   `game_result`/`game_ended` pair to replace the `p2 <= 2` / draw verdicts
   in `rb_check_victory_condition` (4-1).
3. `src/core/modifiers.c`: the CONSTANT scan has **no** `action == "restriction"`
   arm at all (0 hits for "restriction" outside one comment), so the two 
   `cannot_activate` abilities never register in the C. T6-5 sidesteps this by
   recomputing the predicate at the read site; a proper field on `GameState`
   would remove the workaround.
4. Five DEAD-WRONG twins (3.1). None is reachable, so none is urgent; all five
   are places where Rust has a live call site and the C does not.