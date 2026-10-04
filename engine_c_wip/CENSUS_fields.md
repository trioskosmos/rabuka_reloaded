# CENSUS 4 of 5 - FIELD CENSUS

`GameState` / `RbPlayer` / `RbMods` / `RbLiveSnapshot`: **269 fields**, each
with the writers and readers it actually has.

| struct | fields | 0W/0R | nW/0R | 0W/nR |
|---|---|---|---|---|
| `GameState` | 196 | 16 | 10 | 12 |
| `RbPlayer` | 17 | 0 | 1 | 1 |
| `RbMods` | 32 | 4 | 3 | 0 |
| `RbLiveSnapshot` | 24 | 5 | 2 | 3 |
| **total** | **269** | **25** | | |

## Method

- Writers and readers are counted from a comment/string-stripped scan of every
  `.c`/`.h` in `src/ include/ tests/ tools/`, resolving `->field` to a struct by
  the base variable's declared type and by walking sub-struct paths
  (`g->mods.x` -> RbMods, `g->p[i].x` -> RbPlayer, `g->snapshots[n].x` ->
  RbLiveSnapshot).
- A write is `=`, a compound assignment, `++`/`--`, an address-of handed to a
  mutator, or a `memset`/`memcpy` destination.
- A `memset(&g, 0, sizeof(*g))` on a WHOLE struct is deliberately NOT counted as
  a writer. It is real, but crediting it would make all 196 GameState fields
  look produced and hide exactly the smell being hunted. Those fields are
  flagged `bulk` in the tables instead.
- The reconciliation found 11 items marked complete that were 0W/0R. **This
  census finds 25.**

## 4a. Every field at 0 writers and 0 readers (25)

**A field with no producer and no consumer is worse than an absent field:** it
reads as coverage in the header while nothing in the tree can ever observe it.

| struct | field | declared | bulk-only | what the tree says about it |
|---|---|---|---|---|
| `GameState` | `pending_energy_payment` | rabuka.h:1487 | yes (1 bulk memset) | declared only. |
| `GameState` | `player1_cheer_revealed_cards` | rabuka.h:1521 | yes (1 bulk memset) | declared only; the counter next to it likewise. |
| `GameState` | `n_player1_cheer_revealed_cards` | rabuka.h:1522 | yes (1 bulk memset) |  |
| `GameState` | `player2_cheer_revealed_cards` | rabuka.h:1523 | yes (1 bulk memset) |  |
| `GameState` | `n_player2_cheer_revealed_cards` | rabuka.h:1524 | yes (1 bulk memset) |  |
| `GameState` | `revealed_cost_cards` | rabuka.h:1527 | yes (1 bulk memset) | declared only; the counter next to it likewise. |
| `GameState` | `n_revealed_cost_cards` | rabuka.h:1528 | yes (1 bulk memset) |  |
| `GameState` | `assignment` | rabuka.h:1553 | yes (1 bulk memset) | 20 textual occurrences, ALL of them prose in comments. Zero `->assignment` or `.assignment` accesses tree-wide. |
| `GameState` | `n_assignment` | rabuka.h:1554 | yes (1 bulk memset) | counter for the above. |
| `GameState` | `turn2_abilities_played` | rabuka.h:1655 | yes (1 bulk memset) | n_turn2_abilities_played IS written (1 site) but never read; this array is not. |
| `GameState` | `areas_placed_this_turn` | rabuka.h:1672 | yes (1 bulk memset) | n_areas_placed_this_turn IS written, never read. |
| `GameState` | `auto_ability_trigger_counts` | rabuka.h:1674 | yes (1 bulk memset) | n_auto_ability_trigger_counts IS written, never read. |
| `GameState` | `energy_placed_this_turn` | rabuka.h:1693 | yes (1 bulk memset) | declared only. |
| `GameState` | `n_gained_cards` | rabuka.h:1735 | yes (1 bulk memset) | declared only. |
| `GameState` | `constant_need_heart_global` | rabuka.h:1828 | yes (1 bulk memset) | rabuka.h:1824 asserts it is "consumed by modify_required_hearts_global which is reachable from 3 abilities". There are no consumers. |
| `GameState` | `constant_need_heart_global_set` | rabuka.h:1829 | yes (1 bulk memset) | the paired "set" flag, likewise untouched. |
| `RbLiveSnapshot` | `performance_need_heart_modifiers` | rabuka.h:1357 | yes (1 bulk memset) | declared with a comment describing its purpose; no code references it. |
| `RbLiveSnapshot` | `n_performance_need_heart_modifiers` | rabuka.h:1358 | yes (1 bulk memset) | counter for the above. |
| `RbLiveSnapshot` | `base_score_total` | rabuka.h:1369 | yes (1 bulk memset) | live.c:1002 documents the field as part of the C shape of live.rs; only the COMMENT mentions it. |
| `RbLiveSnapshot` | `card_bonus_total` | rabuka.h:1370 | yes (1 bulk memset) | live.c:1002, same. |
| `RbLiveSnapshot` | `live_spare` | rabuka.h:1371 | yes (1 bulk memset) | live.c:1011-1012 documents it; only the comment mentions it. |
| `RbMods` | `success_zone_blade_bonuses` | rabuka.h:249 | yes (1 bulk memset) | rabuka.h:240 calls these three "NOT cosmetic" and warns of cross-live leakage. Never written, never read. |
| `RbMods` | `success_zone_heart_bonuses` | rabuka.h:250 | yes (1 bulk memset) | see above. |
| `RbMods` | `success_zone_score_bonuses` | rabuka.h:251 | yes (1 bulk memset) | see above. |
| `RbMods` | `opp_cause_fired_keys` | rabuka.h:282 | yes (1 bulk memset) |  |

### The three pairs where the COUNTER is written and the DATA is not

`turn2_abilities_played` / `n_turn2_abilities_played`, `areas_placed_this_turn`
/ `n_areas_placed_this_turn`, `auto_ability_trigger_counts` /
`n_auto_ability_trigger_counts`: in each case the `n_` counter has a write site
and the array it counts has none. That is a writer that maintains a length for
a buffer nothing ever fills - the counter is provably always 0, and any reader
added later would read an empty array and conclude "no such cards", silently.

## 4b. Fields with no READER (written, never observed) - 16

| struct | field | write sites |
|---|---|---|
| `GameState` | `turn_state_changes` | 20 |
| `GameState` | `mulligan_selecting` | 1 |
| `GameState` | `mulligan_done` | 1 |
| `GameState` | `current_is_baton` | 4 |
| `GameState` | `n_turn2_abilities_played` | 1 |
| `GameState` | `cheer_check_completed` | 2 |
| `GameState` | `baton_touch_replaced_member_cost` | 8 |
| `GameState` | `n_areas_placed_this_turn` | 1 |
| `GameState` | `n_auto_ability_trigger_counts` | 1 |
| `GameState` | `entry_keepalive` | 4 |
| `RbPlayer` | `yell_note_icons` | 1 |
| `RbMods` | `last_cost_moved_card_ids` | 5 |
| `RbMods` | `n_last_cost_moved_card_ids` | 6 |
| `RbMods` | `n_opp_cause_fired_keys` | 1 |
| `RbLiveSnapshot` | `member_contributions` | 2 |
| `RbLiveSnapshot` | `n_member_contributions` | 1 |

## 4c. Fields with no WRITER (read, never produced)

| struct | field | read sites | where it is read |
|---|---|---|---|
| `GameState` | `yell_source` | 2 | src/ability/effects/executor.c:550, src/turn/live.c:202 |
| `GameState` | `last_energy_placed_by_effect` | 10 | src/core/game_state_abilities.c:1688, src/core/game_state_abilities.c:3575, src/core/game_state_abilities.c:3580, src/core/game_state_abilities.c:3585 |
| `GameState` | `last_energy_placed_by_player` | 2 | src/core/game_state_abilities.c:1693, src/core/game_state_core.c:492 |
| `GameState` | `last_area_move_card_id` | 3 | src/core/game_state_abilities.c:1697, src/core/game_state_abilities.c:1698, src/core/game_state_core.c:460 |
| `GameState` | `last_area_move_by_player` | 2 | src/core/game_state_abilities.c:1703, src/core/game_state_core.c:463 |
| `GameState` | `rps` | 1 | src/core/game_state_abilities.c:1390 |
| `GameState` | `live_set_player` | 1 | src/core/game_state_abilities.c:1390 |
| `GameState` | `exclusion_zone` | 5 | src/core/zones.c:961, src/core/zones.c:962, tests/test_audit_zones.c:1144, tests/test_audit_zones.c:1147 |
| `GameState` | `turn1_abilities_played` | 1 | tests/test_parity_draw_flat.c:313 |
| `GameState` | `last_resolution_cards_p1` | 1 | src/core/tracking.c:46 |
| `GameState` | `last_resolution_cards_p2` | 1 | src/core/tracking.c:47 |
| `GameState` | `yell_count_modifiers` | 7 | src/core/tracking.c:90, src/core/tracking.c:91, src/core/tracking.c:101, src/turn/live.c:164 |
| `RbPlayer` | `life` | 1 | src/web_server.c:153 |
| `RbLiveSnapshot` | `turn` | 4 | src/turn/live.c:1221, src/turn/live.c:1335, src/turn/live.c:1678, tests/test_parity_rules_phases.c:204 |
| `RbLiveSnapshot` | `live_required` | 33 | src/turn/live.c:453, src/turn/live.c:728, src/turn/live.c:942, tests/test_parity_char2.c:710 |
| `RbLiveSnapshot` | `live_filled` | 17 | src/turn/live.c:454, src/turn/live.c:941, src/turn/live.c:1085, src/turn/live.c:1091 |

## 4d. The full table (269 fields)

| struct | field | hdr line | writers | readers | bulk | declaration |
|---|---|---|---|---|---|---|
| **`GameState`** | | | | | | |
| | `p` | 1471 | 379 | 7693 | 23 | `p[2]; int stage_hearts[2][8]; RbMods mods;` |
| | `stage_hearts` | 1472 | 48 | 5 | 23 | `stage_hearts[2][8]; RbMods mods; RbAbilityQueue queue; /` |
| | `mods` | 1473 | 244 | 1013 | 23 | `mods; RbAbilityQueue queue; /* Keep-alive for an in-fli` |
| | `queue` | 1475 | 311 | 1640 | 23 | `queue; /* Keep-alive for an in-flight activation that pends a` |
| | `activation_keepalive` | 1481 | 8 | 8 | 23 | `activation_keepalive; int activation_keepalive_valid; AbilityEff` |
| | `activation_keepalive_valid` | 1482 | 19 | 8 | 23 | `activation_keepalive_valid; AbilityEffect *activation_act; int l` |
| | `activation_act` | 1483 | 6 | 2 | 23 | `activation_act; int live_set_limit_reduction[2]; int yel` |
| | `live_set_limit_reduction` | 1484 | 12 | 19 | 23 | `live_set_limit_reduction[2]; int yell_count_mod[2]; /* per-playe` |
| | `yell_count_mod` | 1485 | 2 | 1 | 23 | `yell_count_mod[2]; /* per-player modify_yell_count delta (live.c` |
| | `activating_card` | 1486 | 216 | 162 | 23 | `activating_card; /* currently resolving card id (mirrors gs.acti` |
| | `pending_energy_payment` | 1487 | 0 | 0 | 23 | `pending_energy_payment; /* optional energy payment count (resolv` |
| | `last_cost_waited_members` | 1488 | 5 | 3 | 23 | `last_cost_waited_members[RB_STAGE_SIZE]; /* members put to wait ` |
| | `n_last_cost_waited_members` | 1489 | 8 | 10 | 23 | `n_last_cost_waited_members; char yell_source[2][16]; /* per-play` |
| | `yell_source` | 1490 | 0 | 2 | 23 | `yell_source[2][16]; /* per-player modify_yell_source override (l` |
| | `cards_paid_cost_this_turn` | 1503 | 2 | 1 | 23 | `cards_paid_cost_this_turn[RB_MAX_ZONE]; int n_cards_paid_cost_th` |
| | `n_cards_paid_cost_this_turn` | 1504 | 2 | 2 | 23 | `n_cards_paid_cost_this_turn; /* Rust GameState.cards_activated_t` |
| | `cards_activated_this_turn` | 1517 | 1 | 2 | 23 | `cards_activated_this_turn[RB_MAX_CARD_IDS]; int n_cards_activate` |
| | `n_cards_activated_this_turn` | 1518 | 3 | 6 | 23 | `n_cards_activated_this_turn; /* cheer_revealed_cards / _mut (mod` |
| | `player1_cheer_revealed_cards` | 1521 | 0 | 0 | 23 | `player1_cheer_revealed_cards[8]; int n_player1_cheer_revealed_ca` |
| | `n_player1_cheer_revealed_cards` | 1522 | 0 | 0 | 23 | `n_player1_cheer_revealed_cards; int player2_cheer_revealed_cards` |
| | `player2_cheer_revealed_cards` | 1523 | 0 | 0 | 23 | `player2_cheer_revealed_cards[8]; int n_player2_cheer_revealed_ca` |
| | `n_player2_cheer_revealed_cards` | 1524 | 0 | 0 | 23 | `n_player2_cheer_revealed_cards; /* push_revealed_cost_card (mod.` |
| | `revealed_cost_cards` | 1527 | 0 | 0 | 23 | `revealed_cost_cards[RB_MAX_ZONE]; int n_revealed_cost_cards; /* ` |
| | `n_revealed_cost_cards` | 1528 | 0 | 0 | 23 | `n_revealed_cost_cards; /* Rust GameState.turn_area_movements: Sm` |
| | `turn_area_movements` | 1535 | 6 | 26 | 23 | `turn_area_movements[RB_MAX_RECENTLY_MOVED]; int n_turn_area_move` |
| | `n_turn_area_movements` | 1536 | 7 | 9 | 23 | `n_turn_area_movements; RbLiveSnapshot snapshots[RB_MAX_SNAPSHOTS` |
| | `snapshots` | 1537 | 17 | 68 | 23 | `snapshots[RB_MAX_SNAPSHOTS]; int n_snapshots; int r` |
| | `n_snapshots` | 1538 | 5 | 66 | 23 | `n_snapshots; int recently_moved[RB_MAX_RECENTLY_MOVED]; in` |
| | `recently_moved` | 1539 | 57 | 47 | 23 | `recently_moved[RB_MAX_RECENTLY_MOVED]; int n_recently_moved;` |
| | `n_recently_moved` | 1540 | 103 | 93 | 23 | `n_recently_moved; int those_cards[RB_MAX_RECENTLY_MOVED]; /* car` |
| | `those_cards` | 1541 | 4 | 7 | 23 | `those_cards[RB_MAX_RECENTLY_MOVED]; /* cards moved by the immedi` |
| | `n_those_cards` | 1542 | 14 | 10 | 23 | `n_those_cards; int recently_appeared[RB_MAX_RECENTLY_MOVED]; /*` |
| | `recently_appeared` | 1543 | 1 | 1 | 23 | `recently_appeared[RB_MAX_RECENTLY_MOVED]; /* cards that appeared` |
| | `n_recently_appeared` | 1544 | 4 | 3 | 23 | `n_recently_appeared; int recently_state_changed[RB_MAX_RECENTLY_` |
| | `recently_state_changed` | 1545 | 3 | 1 | 23 | `recently_state_changed[RB_MAX_RECENTLY_MOVED]; /* cards whose st` |
| | `n_recently_state_changed` | 1546 | 7 | 12 | 23 | `n_recently_state_changed; RbBatchMovement batch_movements[16]; i` |
| | `batch_movements` | 1547 | 6 | 28 | 23 | `batch_movements[16]; int n_batch_movements; RbPositionChangeEven` |
| | `n_batch_movements` | 1548 | 21 | 35 | 23 | `n_batch_movements; RbPositionChangeEvent position_change_events[` |
| | `position_change_events` | 1549 | 3 | 15 | 23 | `position_change_events[16]; int n_position_change_events; int se` |
| | `n_position_change_events` | 1550 | 13 | 9 | 23 | `n_position_change_events; int selected_cards[RB_MAX_RECENTLY_MOV` |
| | `selected_cards` | 1551 | 54 | 64 | 23 | `selected_cards[RB_MAX_RECENTLY_MOVED]; /* cards chosen by a sele` |
| | `n_selected_cards` | 1552 | 106 | 113 | 23 | `n_selected_cards; int assignment[RB_MAX_RECENTLY_MOVED]; /* dist` |
| | `assignment` | 1553 | 0 | 0 | 23 | `assignment[RB_MAX_RECENTLY_MOVED]; /* distinct-name assignment f` |
| | `n_assignment` | 1554 | 0 | 0 | 23 | `n_assignment; int live_success[2]; int live_score[2]` |
| | `live_success` | 1555 | 74 | 19 | 23 | `live_success[2]; int live_score[2]; int live_batch_m` |
| | `live_score` | 1556 | 5 | 2 | 23 | `live_score[2]; int live_batch_mode; int live_victo` |
| | `live_batch_mode` | 1557 | 13 | 1 | 23 | `live_batch_mode; int live_victory_pending; int live_vic` |
| | `live_victory_pending` | 1558 | 2 | 2 | 23 | `live_victory_pending; int live_victory_stage; int live_pre_s` |
| | `live_victory_stage` | 1559 | 7 | 5 | 23 | `live_victory_stage; int live_pre_score[2][RB_MAX_CARD_IDS]; int` |
| | `live_pre_score` | 1560 | 1 | 3 | 23 | `live_pre_score[2][RB_MAX_CARD_IDS]; int live_pre_valid[2];` |
| | `live_pre_valid` | 1561 | 2 | 3 | 23 | `live_pre_valid[2]; int p1_live_won; /* Rule 8.4.13: P1 won` |
| | `p1_live_won` | 1563 | 11 | 12 | 23 | `p1_live_won; /* Rule 8.4.13: P1 won the live (placed to success)` |
| | `p2_live_won` | 1564 | 6 | 9 | 23 | `p2_live_won; /* Rule 8.4.13: P2 won the live (placed to success)` |
| | `self_live_surplus_count` | 1567 | 7 | 1 | 23 | `self_live_surplus_count; int opponent_live_surplus_count; int li` |
| | `opponent_live_surplus_count` | 1568 | 6 | 1 | 23 | `opponent_live_surplus_count; int live_surplus_ready_this_turn; i` |
| | `live_surplus_ready_this_turn` | 1569 | 8 | 1 | 23 | `live_surplus_ready_this_turn; int p1_live_success_no_excess; /* ` |
| | `p1_live_success_no_excess` | 1570 | 4 | 4 | 23 | `p1_live_success_no_excess; /* Rust ConditionContext::no_excess_h` |
| | `self_no_excess_heart_this_turn` | 1578 | 2 | 1 | 23 | `self_no_excess_heart_this_turn; int opponent_live_no_excess_hear` |
| | `opponent_live_no_excess_heart_this_turn` | 1579 | 2 | 1 | 23 | `opponent_live_no_excess_heart_this_turn; int p2_live_success_no_` |
| | `p2_live_success_no_excess` | 1580 | 8 | 2 | 23 | `p2_live_success_no_excess; /* state_change_condition tracking (m` |
| | `turn_state_changes` | 1585 | 20 | 0 | 23 | `turn_state_changes[64][4]; /* [activating_card, target_card, fro` |
| | `n_turn_state_changes` | 1586 | 8 | 13 | 23 | `n_turn_state_changes; int8_t state_change_from[RB_MAX_CARD_IDS];` |
| | `state_change_from` | 1587 | 3 | 5 | 23 | `state_change_from[RB_MAX_CARD_IDS]; int8_t state_change_to[RB_MA` |
| | `state_change_to` | 1588 | 6 | 2 | 23 | `state_change_to[RB_MAX_CARD_IDS]; int state_change_triggering;` |
| | `state_change_triggering` | 1589 | 24 | 1 | 23 | `state_change_triggering; int last_wait_to_active_count; /* count` |
| | `last_wait_to_active_count` | 1590 | 2 | 7 | 23 | `last_wait_to_active_count; /* count of wait→active flips this tu` |
| | `revealed_cards` | 1591 | 52 | 53 | 23 | `revealed_cards[RB_MAX_REVEALED_CARDS]; /* cards revealed by yell` |
| | `n_revealed` | 1592 | 122 | 142 | 23 | `n_revealed; int yell_occurred; int performance_res` |
| | `yell_occurred` | 1593 | 47 | 18 | 23 | `yell_occurred; int performance_resume_pending; int pe` |
| | `performance_resume_pending` | 1594 | 3 | 7 | 23 | `performance_resume_pending; int performance_resume_player; int p` |
| | `performance_resume_player` | 1595 | 1 | 1 | 23 | `performance_resume_player; int performance_resume_yell_cards[RB_` |
| | `performance_resume_yell_cards` | 1596 | 1 | 1 | 23 | `performance_resume_yell_cards[RB_MAX_ZONE]; int performance_resu` |
| | `performance_resume_n_yell_cards` | 1597 | 1 | 1 | 23 | `performance_resume_n_yell_cards; int performance_resume_blade_he` |
| | `performance_resume_blade_hearts` | 1598 | 1 | 1 | 23 | `performance_resume_blade_hearts[8]; int performance_resume_note_` |
| | `performance_resume_note_icons` | 1599 | 1 | 1 | 23 | `performance_resume_note_icons; int performance_resume_yell_flag;` |
| | `performance_resume_yell_flag` | 1600 | 1 | 1 | 23 | `performance_resume_yell_flag; int last_draw_count; /* mirror Abi` |
| | `last_draw_count` | 1601 | 16 | 22 | 23 | `last_draw_count; /* mirror AbilityResolver.step_state.last_draw_` |
| | `last_surplus_loss_count` | 1602 | 1 | 1 | 23 | `last_surplus_loss_count[2]; /* gain_surplus_heart: surplus heart` |
| | `re_yell_occurred` | 1603 | 10 | 8 | 23 | `re_yell_occurred; /* a re_yell effect fired this live */ int re_` |
| | `re_yell_blade_hearts` | 1604 | 3 | 6 | 23 | `re_yell_blade_hearts[8]; /* hearts harvested by perform_yell, ap` |
| | `re_yell_note_icons` | 1605 | 4 | 3 | 23 | `re_yell_note_icons; int re_yell_pending; int re_yell_owner` |
| | `re_yell_pending` | 1606 | 4 | 5 | 23 | `re_yell_pending; int re_yell_owner; int last_energy_plac` |
| | `re_yell_owner` | 1607 | 2 | 1 | 23 | `re_yell_owner; int last_energy_placed_by_effect; int la` |
| | `last_energy_placed_by_effect` | 1608 | 0 | 10 | 23 | `last_energy_placed_by_effect; int last_energy_placed_by_player; ` |
| | `last_energy_placed_by_player` | 1609 | 0 | 2 | 23 | `last_energy_placed_by_player; int last_area_move_card_id; int la` |
| | `last_area_move_card_id` | 1610 | 0 | 3 | 23 | `last_area_move_card_id; int last_area_move_by_player; int active` |
| | `last_area_move_by_player` | 1611 | 0 | 2 | 23 | `last_area_move_by_player; int active; int first_attacker; int` |
| | `active` | 1612 | 76 | 119 | 23 | `active; int first_attacker; int second_attacker;` |
| | `first_attacker` | 1613 | 33 | 44 | 23 | `first_attacker; int second_attacker; int turn; int` |
| | `second_attacker` | 1614 | 25 | 28 | 23 | `second_attacker; int turn; int winner; RbPhase phase` |
| | `turn` | 1615 | 43 | 86 | 23 | `turn; int winner; RbPhase phase; int rps[` |
| | `winner` | 1616 | 15 | 34 | 23 | `winner; RbPhase phase; int rps[2]; int rps_` |
| | `phase` | 1617 | 129 | 266 | 23 | `phase; int rps[2]; int rps_winner; /*` |
| | `rps` | 1618 | 0 | 1 | 23 | `rps[2]; int rps_winner; /* -1=none, 0=p1, 1=` |
| | `rps_winner` | 1619 | 2 | 6 | 23 | `rps_winner; /* -1=none, 0=p1, 1=p2, 2=tie */ int pl` |
| | `player1_rps_choice` | 1620 | 4 | 5 | 23 | `player1_rps_choice; /* RPS choice for player 1: -1=none, 0=rock,` |
| | `player2_rps_choice` | 1621 | 5 | 3 | 23 | `player2_rps_choice; /* RPS choice for player 2: -1=none, 0=rock,` |
| | `mulligan_selecting` | 1622 | 1 | 0 | 23 | `mulligan_selecting[2]; /* 1 if mulligan selection in progress pe` |
| | `mulligan_done` | 1623 | 1 | 0 | 23 | `mulligan_done[2]; /* per-player mulligan done flag */ int li` |
| | `live_set_player` | 1624 | 0 | 1 | 23 | `live_set_player; RbBag resolution; /* resolution zone (t` |
| | `resolution` | 1625 | 1 | 56 | 23 | `resolution; /* resolution zone (temp holding) */ /* Ru` |
| | `resolution_owners` | 1632 | 3 | 3 | 23 | `resolution_owners[RB_MAX_ZONE]; int n_resolution_owners; RbBag` |
| | `n_resolution_owners` | 1633 | 4 | 2 | 23 | `n_resolution_owners; RbBag exclusion_zone; /* cards excluded fro` |
| | `exclusion_zone` | 1634 | 0 | 5 | 23 | `exclusion_zone; /* cards excluded from the game */ RbTempEffe` |
| | `temp_effects` | 1635 | 10 | 4 | 23 | `temp_effects[RB_MAX_TEMP_EFFECTS]; int n_temp_effects; Rb` |
| | `n_temp_effects` | 1636 | 9 | 23 | 23 | `n_temp_effects; RbDelayedGainedEffect delayed_gained_effects[RB_` |
| | `delayed_gained_effects` | 1637 | 2 | 11 | 23 | `delayed_gained_effects[RB_MAX_DELAYED_GAINED]; int n_delayed_gai` |
| | `n_delayed_gained_effects` | 1638 | 4 | 19 | 23 | `n_delayed_gained_effects; int stage_arrived[2][RB_STAGE_SIZE]; /` |
| | `stage_arrived` | 1640 | 20 | 10 | 23 | `stage_arrived[2][RB_STAGE_SIZE]; /* set when a member was deploy` |
| | `baton_touch_used` | 1641 | 5 | 2 | 23 | `baton_touch_used[2]; /* baton used this play-action (once-per-ac` |
| | `baton_last_vacated_area` | 1642 | 11 | 2 | 23 | `baton_last_vacated_area[2]; /* stage area vacated by the most re` |
| | `current_is_baton` | 1643 | 4 | 0 | 23 | `current_is_baton; /* context flag threaded to effect/trigger` |
| | `player_cannot_activate` | 1644 | 14 | 11 | 23 | `player_cannot_activate[2]; /* restriction: that player may not a` |
| | `cannot_active_cards` | 1645 | 3 | 4 | 23 | `cannot_active_cards[RB_MAX_ZONE]; /* restriction: per-card canno` |
| | `n_cannot_active_cards` | 1646 | 9 | 9 | 23 | `n_cannot_active_cards; char prohibition[64][48]; /* restrictio` |
| | `prohibition` | 1647 | 1 | 18 | 23 | `prohibition[64][48]; /* restriction: "type:destination" proh` |
| | `n_prohibition` | 1648 | 15 | 28 | 23 | `n_prohibition; char prohibition_effects[64][48]; /* dynamic pr` |
| | `prohibition_effects` | 1649 | 1 | 16 | 23 | `prohibition_effects[64][48]; /* dynamic prohibition effects (e.g` |
| | `n_prohibition_effects` | 1650 | 9 | 16 | 23 | `n_prohibition_effects; RbReplacementEffect replacement_effects[3` |
| | `replacement_effects` | 1651 | 2 | 10 | 23 | `replacement_effects[32]; /* mirrors Rust replacement_effects */ ` |
| | `n_replacement_effects` | 1652 | 1 | 12 | 23 | `n_replacement_effects; /* ── tracking.rs (ported) ── */ int turn` |
| | `turn1_abilities_played` | 1654 | 0 | 1 | 23 | `turn1_abilities_played[64]; int n_turn1_abilities_played; int tu` |
| | `n_turn1_abilities_played` | 1654 | 1 | 2 | 23 | `n_turn1_abilities_played; int turn2_abilities_played[64]; int n_` |
| | `turn2_abilities_played` | 1655 | 0 | 0 | 23 | `turn2_abilities_played[64]; int n_turn2_abilities_played; int pl` |
| | `n_turn2_abilities_played` | 1655 | 1 | 0 | 23 | `n_turn2_abilities_played; int player1_cheer_blade_heart_count; i` |
| | `player1_cheer_blade_heart_count` | 1656 | 2 | 7 | 23 | `player1_cheer_blade_heart_count; int player2_cheer_blade_heart_c` |
| | `player2_cheer_blade_heart_count` | 1657 | 2 | 3 | 23 | `player2_cheer_blade_heart_count; RbBag last_resolution_cards_p1;` |
| | `last_resolution_cards_p1` | 1658 | 0 | 1 | 23 | `last_resolution_cards_p1; RbBag last_resolution_cards_p2; int ch` |
| | `last_resolution_cards_p2` | 1659 | 0 | 1 | 23 | `last_resolution_cards_p2; int cheer_check_base; /* -1 = None */ ` |
| | `cheer_check_base` | 1660 | 5 | 2 | 23 | `cheer_check_base; /* -1 = None */ int cheer_checks_required; in` |
| | `cheer_checks_required` | 1661 | 1 | 8 | 23 | `cheer_checks_required; int cheer_checks_done; int cheer_check_c` |
| | `cheer_checks_done` | 1662 | 1 | 2 | 23 | `cheer_checks_done; int cheer_check_completed; RbYellMod yell_cou` |
| | `cheer_check_completed` | 1663 | 2 | 0 | 23 | `cheer_check_completed; RbYellMod yell_count_modifiers[32]; int n` |
| | `yell_count_modifiers` | 1664 | 0 | 7 | 23 | `yell_count_modifiers[32]; int n_yell_count_modifiers; int baton_` |
| | `n_yell_count_modifiers` | 1664 | 1 | 7 | 23 | `n_yell_count_modifiers; int baton_touch_count_p1; int baton_touc` |
| | `baton_touch_count_p1` | 1665 | 5 | 9 | 23 | `baton_touch_count_p1; int baton_touch_count_p2; int baton_touc` |
| | `baton_touch_count_p2` | 1666 | 3 | 4 | 23 | `baton_touch_count_p2; int baton_touch_arriving_card_ids[16]; int` |
| | `baton_touch_arriving_card_ids` | 1667 | 2 | 4 | 23 | `baton_touch_arriving_card_ids[16]; int n_baton_touch_arriving_ca` |
| | `n_baton_touch_arriving_card_ids` | 1667 | 3 | 9 | 23 | `n_baton_touch_arriving_card_ids; int baton_touch_zero_cost; int ` |
| | `baton_touch_zero_cost` | 1668 | 6 | 3 | 23 | `baton_touch_zero_cost; int baton_touch_replaced_member_cost; /* ` |
| | `baton_touch_replaced_member_cost` | 1669 | 8 | 0 | 23 | `baton_touch_replaced_member_cost; /* -1 = None */ int baton_touc` |
| | `baton_touch_replaced_member_id` | 1670 | 11 | 3 | 23 | `baton_touch_replaced_member_id; /* -1 = None */ int baton_touch_` |
| | `baton_touch_arriving_card_id` | 1671 | 10 | 3 | 23 | `baton_touch_arriving_card_id; /* -1 = None */ int areas_placed_t` |
| | `areas_placed_this_turn` | 1672 | 0 | 0 | 23 | `areas_placed_this_turn[16]; int n_areas_placed_this_turn; int ca` |
| | `n_areas_placed_this_turn` | 1672 | 1 | 0 | 23 | `n_areas_placed_this_turn; int cards_appeared_this_turn[64]; int ` |
| | `cards_appeared_this_turn` | 1673 | 5 | 10 | 23 | `cards_appeared_this_turn[64]; int n_cards_appeared_this_turn; in` |
| | `n_cards_appeared_this_turn` | 1673 | 12 | 22 | 23 | `n_cards_appeared_this_turn; int auto_ability_trigger_counts[32];` |
| | `auto_ability_trigger_counts` | 1674 | 0 | 0 | 23 | `auto_ability_trigger_counts[32]; int n_auto_ability_trigger_coun` |
| | `n_auto_ability_trigger_counts` | 1674 | 1 | 0 | 23 | `n_auto_ability_trigger_counts; int position_change_occurred_this` |
| | `position_change_occurred_this_turn` | 1675 | 31 | 23 | 23 | `position_change_occurred_this_turn; int formation_change_occurre` |
| | `formation_change_occurred_this_turn` | 1676 | 2 | 3 | 23 | `formation_change_occurred_this_turn; int opponent_live_success_t` |
| | `opponent_live_success_this_turn` | 1677 | 3 | 1 | 23 | `opponent_live_success_this_turn; int opponent_choice_declined; i` |
| | `opponent_choice_declined` | 1678 | 2 | 1 | 23 | `opponent_choice_declined; int game_state_history[64]; int n_game` |
| | `game_state_history` | 1679 | 1 | 1 | 23 | `game_state_history[64]; int n_game_state_history; int loop_detec` |
| | `n_game_state_history` | 1679 | 3 | 3 | 23 | `n_game_state_history; int loop_detected; /* just_completed_abili` |
| | `loop_detected` | 1680 | 3 | 2 | 23 | `loop_detected; /* just_completed_ability_key — mirrors Rust's Ga` |
| | `just_completed_ability_key` | 1685 | 9 | 6 | 23 | `just_completed_ability_key; /* batch_triggered_keys — per auto-t` |
| | `batch_triggered_keys` | 1689 | 5 | 5 | 23 | `batch_triggered_keys[64]; int n_batch_triggered_keys; /* ── temp` |
| | `n_batch_triggered_keys` | 1689 | 8 | 11 | 23 | `n_batch_triggered_keys; /* ── temporal-condition tracking (mirro` |
| | `moved_this_turn` | 1692 | 17 | 23 | 23 | `moved_this_turn[RB_MAX_CARD_IDS]; /* per-card: moved during curr` |
| | `energy_placed_this_turn` | 1693 | 0 | 0 | 23 | `energy_placed_this_turn[2]; /* energy placed by player this turn` |
| | `debut_count_this_turn` | 1694 | 11 | 25 | 23 | `debut_count_this_turn[2]; /* members debuted this turn per playe` |
| | `auto_event_mask` | 1699 | 2 | 1 | 23 | `auto_event_mask[2]; /* play recursion depth — guards rb_play_mem` |
| | `play_depth` | 1702 | 13 | 2 | 23 | `play_depth; /* ── C6 keep-N-shuffle-rest (draw.rs::execute_both_` |
| | `keep_shuffle_under_phase` | 1709 | 5 | 4 | 23 | `keep_shuffle_under_phase; int keep_shuffle_under_count; int keep` |
| | `keep_shuffle_under_count` | 1710 | 1 | 1 | 23 | `keep_shuffle_under_count; int keep_shuffle_under_snapshot[2][RB_` |
| | `keep_shuffle_under_snapshot` | 1711 | 3 | 4 | 23 | `keep_shuffle_under_snapshot[2][RB_MAX_HAND]; int keep_shuffle_un` |
| | `keep_shuffle_under_snapshot_n` | 1712 | 6 | 5 | 23 | `keep_shuffle_under_snapshot_n[2]; int keep_shuffle_under_selecte` |
| | `keep_shuffle_under_selected` | 1713 | 2 | 2 | 23 | `keep_shuffle_under_selected[RB_MAX_HAND]; int keep_shuffle_under` |
| | `keep_shuffle_under_selected_n` | 1714 | 4 | 5 | 23 | `keep_shuffle_under_selected_n; /* Play-time alternative cost (Ru` |
| | `ptc_active` | 1720 | 5 | 20 | 23 | `ptc_active; /* a play is paused awaiting the alt-cost answer */` |
| | `ptc_resuming` | 1721 | 2 | 1 | 23 | `ptc_resuming; /* rb_play_member is completing a paused play */ i` |
| | `ptc_card` | 1722 | 4 | 1 | 23 | `ptc_card; /* card id being played */ int ptc_hand;` |
| | `ptc_hand` | 1723 | 1 | 1 | 23 | `ptc_hand; /* hand index of the card */ int ptc_area;` |
| | `ptc_area` | 1724 | 1 | 1 | 23 | `ptc_area; /* target stage area */ int ptc_set;` |
| | `ptc_set` | 1725 | 3 | 3 | 23 | `ptc_set; /* alternative cost value (accept) */ int` |
| | `ptc_base` | 1726 | 3 | 3 | 23 | `ptc_base; /* base cost (decline) */ /* ── gained_card_abil` |
| | `gained_card_ids` | 1732 | 6 | 3 | 23 | `gained_card_ids[64]; /* card_id for each gained-ability slot */` |
| | `gained_card_abilities` | 1733 | 7 | 3 | 23 | `gained_card_abilities[64][4]; /* up to 4 gained abilities per sl` |
| | `gained_card_n` | 1734 | 5 | 10 | 23 | `gained_card_n[64]; /* count of gained abilities per slot */` |
| | `n_gained_cards` | 1735 | 0 | 0 | 23 | `n_gained_cards; /* number of distinct cards with gains */` |
| | `ability_invalidations` | 1736 | 4 | 7 | 23 | `ability_invalidations[RB_MAX_ABILITY_INVALIDATIONS]; int n_abili` |
| | `n_ability_invalidations` | 1737 | 2 | 12 | 23 | `n_ability_invalidations; /* ── entry-accessor keep-alive ring (u` |
| | `entry_keepalive` | 1751 | 4 | 0 | 23 | `entry_keepalive[RB_ENTRY_KEEPALIVE_SLOTS]; unsigned char entry_k` |
| | `entry_keepalive_used` | 1752 | 2 | 1 | 23 | `entry_keepalive_used[RB_ENTRY_KEEPALIVE_SLOTS]; int entry_keepal` |
| | `entry_keepalive_next` | 1753 | 1 | 1 | 23 | `entry_keepalive_next; /* ── recently_moved_from_zone (mirrors Ga` |
| | `recently_moved_from_zone` | 1770 | 1 | 9 | 23 | `recently_moved_from_zone[RB_TE_ZONE_SZ]; /* ── card_appearance_s` |
| | `card_appearance_source_card` | 1797 | 1 | 3 | 23 | `card_appearance_source_card[32]; /* card_no index per entry */ u` |
| | `card_appearance_source_zone` | 1798 | 1 | 1 | 23 | `card_appearance_source_zone[32]; /* RbZone the card debuted from` |
| | `n_card_appearance_source` | 1799 | 4 | 6 | 23 | `n_card_appearance_source; /* Rust GameState.rule_log: Vec<String` |
| | `rule_log` | 1809 | 2 | 2 | 23 | `rule_log[RB_GSC_RULE_LOG_MAX][RB_GSC_RULE_LOG_LINE_MAX]; uint16_` |
| | `rule_log_len` | 1810 | 2 | 1 | 23 | `rule_log_len[RB_GSC_RULE_LOG_MAX]; int n_rule_log; /* Rust` |
| | `n_rule_log` | 1811 | 2 | 4 | 23 | `n_rule_log; /* Rust LiveCardZone.face_up. Rust evicts a wrongly-` |
| | `live_face_up` | 1822 | 3 | 1 | 23 | `live_face_up[2]; /* per SEAT */ /* Rust GameModifiers.constant_n` |
| | `constant_need_heart_global` | 1828 | 0 | 0 | 23 | `constant_need_heart_global[RB_MAX_CARD_IDS][8]; uint8_t constant` |
| | `constant_need_heart_global_set` | 1829 | 0 | 0 | 23 | `constant_need_heart_global_set[RB_MAX_CARD_IDS][8]; } GameState;` |
| **`RbPlayer`** | | | | | | |
| | `hand` | 775 | 32 | 1381 | 2 | `hand; RbBag deck; int stage[RB_STAGE_SIZE];` |
| | `deck` | 776 | 72 | 1411 | 2 | `deck; int stage[RB_STAGE_SIZE]; /* card_no index or` |
| | `stage` | 777 | 2473 | 1164 | 2 | `stage[RB_STAGE_SIZE]; /* card_no index or -1 */ int s` |
| | `stage_wait` | 778 | 144 | 33 | 2 | `stage_wait[RB_STAGE_SIZE]; /* 1 if member is in "wait" state */ ` |
| | `under_cards` | 779 | 23 | 231 | 2 | `under_cards[RB_STAGE_SIZE]; /* cards placed under each stage mem` |
| | `energy` | 780 | 21 | 444 | 2 | `energy; /* energy cards in energy zone */ RbBa` |
| | `energy_deck` | 781 | 11 | 260 | 2 | `energy_deck; /* energy deck (draw pile for energy) */ in` |
| | `energy_active` | 782 | 128 | 239 | 2 | `energy_active; /* count of active energy */ RbBag live;` |
| | `live` | 783 | 25 | 624 | 2 | `live; /* live card zone */ RbBag succe` |
| | `success` | 784 | 16 | 402 | 2 | `success; /* success live card zone */ RbBag` |
| | `discard` | 785 | 42 | 962 | 2 | `discard; /* waitroom */ int deck_refreshed` |
| | `deck_refreshed_this_turn` | 786 | 15 | 9 | 2 | `deck_refreshed_this_turn; /* Rule 10.2.2.1 mid-effect refresh fl` |
| | `score` | 787 | 7 | 32 | 2 | `score; int life; /* life points (HP)` |
| | `life` | 788 | 0 | 1 | 2 | `life; /* life points (HP) Etracked for parity` |
| | `hearts` | 789 | 5 | 4 | 2 | `hearts[RB_MAX_HEARTS]; /* hearts-by-color on this player */ in` |
| | `yell_note_icons` | 790 | 1 | 0 | 2 | `yell_note_icons; /* hearts produced during performance */ int` |
| | `yell_from_bottom` | 791 | 3 | 9 | 2 | `yell_from_bottom; /* G8: yell reveal from deck bottom (tracking.` |
| **`RbMods`** | | | | | | |
| | `blade` | 229 | 24 | 106 | 1 | `blade[RB_MAX_CARD_IDS]; RbModifierEntry heart[RB_MAX_CARD_IDS` |
| | `heart` | 230 | 18 | 23 | 1 | `heart[RB_MAX_CARD_IDS][8]; /* per-color (0..7) */ RbModi` |
| | `need_heart` | 231 | 6 | 12 | 1 | `need_heart[RB_MAX_CARD_IDS][8]; RbModifierEntry score[RB_MAX_CAR` |
| | `score` | 232 | 5 | 11 | 1 | `score[RB_MAX_CARD_IDS]; RbModifierEntry cost[RB_MAX_CARD_IDS]` |
| | `cost` | 233 | 68 | 428 | 1 | `cost[RB_MAX_CARD_IDS]; uint8_t orientation[RB_MAX_CA` |
| | `orientation` | 234 | 3 | 5 | 1 | `orientation[RB_MAX_CARD_IDS]; /* 0 none, 1 active, 2 wait */ uin` |
| | `delayed_cannot_active` | 235 | 3 | 4 | 1 | `delayed_cannot_active[RB_MAX_CARD_IDS]; /* remaining turns */ /*` |
| | `constant_blade` | 237 | 6 | 1 | 1 | `constant_blade[RB_MAX_CARD_IDS]; int16_t constant_score[RB_MAX_C` |
| | `constant_score` | 238 | 5 | 3 | 1 | `constant_score[RB_MAX_CARD_IDS]; int16_t constant_cost[RB_MAX_CA` |
| | `constant_cost` | 239 | 4 | 3 | 1 | `constant_cost[RB_MAX_CARD_IDS]; /* Success-zone bonus ledgers — ` |
| | `success_zone_blade_bonuses` | 249 | 0 | 0 | 1 | `success_zone_blade_bonuses[RB_MAX_CARD_IDS]; int16_t success_zon` |
| | `success_zone_heart_bonuses` | 250 | 0 | 0 | 1 | `success_zone_heart_bonuses[RB_MAX_CARD_IDS][8]; int16_t success_` |
| | `success_zone_score_bonuses` | 251 | 0 | 0 | 1 | `success_zone_score_bonuses[RB_MAX_CARD_IDS]; int16_t constant_he` |
| | `constant_heart` | 252 | 7 | 2 | 1 | `constant_heart[RB_MAX_CARD_IDS][8]; /* per-color, cleared on rec` |
| | `constant_need_heart` | 253 | 2 | 1 | 1 | `constant_need_heart[RB_MAX_CARD_IDS][8]; /* Per-player live TOTA` |
| | `p1_constant_total_score_bonus` | 258 | 6 | 82 | 1 | `p1_constant_total_score_bonus; int16_t p2_constant_total_score_b` |
| | `p2_constant_total_score_bonus` | 259 | 6 | 17 | 1 | `p2_constant_total_score_bonus; int16_t heart_copy[RB_MAX_CARD_ID` |
| | `heart_copy` | 260 | 6 | 3 | 1 | `heart_copy[RB_MAX_CARD_IDS]; /* target→source */ int8_t` |
| | `heart_multiplier` | 261 | 6 | 3 | 1 | `heart_multiplier[RB_MAX_CARD_IDS]; /* -1 none, else colour 0..7 ` |
| | `heart_multiplier_amt` | 262 | 4 | 1 | 1 | `heart_multiplier_amt[RB_MAX_CARD_IDS]; /* multiplier applied to ` |
| | `blade_type` | 263 | 6 | 2 | 1 | `blade_type[RB_MAX_CARD_IDS]; /* -1 none, else BladeColor idx */` |
| | `heart_color_override` | 264 | 4 | 4 | 1 | `heart_color_override[RB_MAX_CARD_IDS]; /* -1 none (specify_heart` |
| | `heart_override_count` | 265 | 2 | 2 | 1 | `heart_override_count[RB_MAX_CARD_IDS]; int last_cost_discard_cou` |
| | `last_cost_discard_count` | 266 | 15 | 5 | 1 | `last_cost_discard_count; /* cards discarded as part of the last ` |
| | `last_cost_moved_card_ids` | 267 | 5 | 0 | 1 | `last_cost_moved_card_ids[8]; /* ids moved for the last cost (mir` |
| | `n_last_cost_moved_card_ids` | 268 | 6 | 0 | 1 | `n_last_cost_moved_card_ids; /* last_under_move_host_ids — mirror` |
| | `last_under_move_host_ids` | 273 | 1 | 2 | 1 | `last_under_move_host_ids[4]; int n_last_under_move_host_ids; /*` |
| | `n_last_under_move_host_ids` | 274 | 3 | 4 | 1 | `n_last_under_move_host_ids; /* snapshot-trace ring (mirrors Abil` |
| | `trace` | 276 | 3 | 7 | 1 | `trace[RB_MODS_TRACE_CAP]; int n_trace; /* Rus` |
| | `n_trace` | 277 | 6 | 3 | 1 | `n_trace; /* Rust game_modifiers.rs:227: SmallVec<[u64;4]>. Keys` |
| | `opp_cause_fired_keys` | 282 | 0 | 0 | 1 | `opp_cause_fired_keys[16]; int n_opp_cause_fired_keys; } RbMods; ` |
| | `n_opp_cause_fired_keys` | 283 | 1 | 0 | 1 | `n_opp_cause_fired_keys; } RbMods; void rb_mods_init(RbMods *m); ` |
| **`RbLiveSnapshot`** | | | | | | |
| | `player` | 1311 | 3 | 34 | 1 | `player; /* 0/1 */ int turn; int total_hearts[8]; int l` |
| | `turn` | 1312 | 0 | 4 | 1 | `turn; int total_hearts[8]; int lives[RB_MAX_LIVE_CARDS];` |
| | `total_hearts` | 1313 | 4 | 25 | 1 | `total_hearts[8]; int lives[RB_MAX_LIVE_CARDS]; int n_lives;` |
| | `lives` | 1314 | 2 | 21 | 1 | `lives[RB_MAX_LIVE_CARDS]; int n_lives; int total_score;` |
| | `n_lives` | 1315 | 2 | 74 | 1 | `n_lives; int total_score; int success; /* 1 if all_pass */` |
| | `total_score` | 1316 | 6 | 34 | 1 | `total_score; int success; /* 1 if all_pass */ int surplus_heart` |
| | `success` | 1317 | 3 | 22 | 1 | `success; /* 1 if all_pass */ int surplus_hearts; /* total_pool` |
| | `surplus_hearts` | 1318 | 6 | 2 | 1 | `surplus_hearts; /* total_pool - total_required, -1 on fail */ in` |
| | `note_icons` | 1319 | 5 | 6 | 1 | `note_icons; int live_passed[RB_MAX_LIVE_CARDS]; /* per-live verd` |
| | `live_passed` | 1320 | 4 | 33 | 1 | `live_passed[RB_MAX_LIVE_CARDS]; /* per-live verdict (populate_li` |
| | `live_required` | 1323 | 0 | 33 | 1 | `live_required[RB_MAX_LIVE_CARDS][8]; int live_filled[RB_MAX_LIVE` |
| | `live_filled` | 1324 | 0 | 17 | 1 | `live_filled[RB_MAX_LIVE_CARDS][8]; int live_score_detail[RB_MAX_` |
| | `live_score_detail` | 1325 | 2 | 16 | 1 | `live_score_detail[RB_MAX_LIVE_CARDS]; int surplus_per_color[8]; ` |
| | `surplus_per_color` | 1326 | 2 | 1 | 1 | `surplus_per_color[8]; int yell_cards[RB_MAX_LIVE_CARDS * 3]; int` |
| | `yell_cards` | 1327 | 3 | 11 | 1 | `yell_cards[RB_MAX_LIVE_CARDS * 3]; int n_yell_cards; int yel` |
| | `n_yell_cards` | 1328 | 4 | 22 | 1 | `n_yell_cards; int yell_blade_hearts[8]; /* ── Verified-missing` |
| | `yell_blade_hearts` | 1329 | 1 | 17 | 1 | `yell_blade_hearts[8]; /* ── Verified-missing GameState members (` |
| | `performance_need_heart_modifiers` | 1357 | 0 | 0 | 1 | `performance_need_heart_modifiers[RB_MAX_PERF_NEED_HEART_MODS]; i` |
| | `n_performance_need_heart_modifiers` | 1358 | 0 | 0 | 1 | `n_performance_need_heart_modifiers; /* The three writes finalize` |
| | `base_score_total` | 1369 | 0 | 0 | 1 | `base_score_total; /* sum over PASSED lives of live_score_detail[` |
| | `card_bonus_total` | 1370 | 0 | 0 | 1 | `card_bonus_total; /* sum over PASSED lives of (score - printed s` |
| | `live_spare` | 1371 | 0 | 0 | 1 | `live_spare[RB_MAX_LIVE_CARDS][8]; /* Rust LivePerformanceData.me` |
| | `member_contributions` | 1378 | 2 | 0 | 1 | `member_contributions[2][RB_STAGE_SIZE]; int n_member_contributio` |
| | `n_member_contributions` | 1379 | 1 | 0 | 1 | `n_member_contributions[2]; } RbLiveSnapshot; /* A modifier grant` |

## 4e. Ambiguity notes

Three field names appear in more than one of the four structs: `score`
(RbMods, RbPlayer), `success` (RbPlayer, RbLiveSnapshot) and `turn`
(GameState, RbLiveSnapshot). They are resolved by the base variable's declared
type plus the sub-struct path. `turn` was initially mis-scored as 0W/0R for
RbLiveSnapshot because of that collision; it is read at `turn/live.c:1613`
(`snapshot->turn != g->turn`) and is correctly 0W/nR.

## 4f. Structural constraints a fixer must respect

Fields are APPENDED to these structs, never inserted: insertion shifts every
later offset and silently invalidates object files compiled against the previous
header. `sizeof(GameState)` is 1 195 656 B and the link stack is 32 MB.

Two of the four structs are declared as anonymous typedefs, not named structs:
`RbMods` at rabuka.h:228, `RbPlayer` at rabuka.h:774, `RbLiveSnapshot` at
rabuka.h:1310, `GameState` at rabuka.h:1470.

