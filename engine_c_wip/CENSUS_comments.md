# CENSUS 3 of 5 - STALE COMMENTS

Comments in this tree assert facts about code that is false. Two machine-
classes plus a hand-adjudicated set.

| class | count |
|---|---|
| cites a `file:line` whose file does not exist anywhere in `engine_c_wip/` or `engine/` | **538** |
| cites a `file:line` past the end of that file | **57** |
| hand-proven false factual claims (below) | **6** |
| **total false comments** | **601** |

The 538 machine hits are not cosmetic. A comment that says "mirrors
`move_cards.rs:2490`" is a reader's only route to the Rust twin; when the file
has been renamed or split, the citation is a dead end and the port cannot be
verified from the comment at all. Rust has been reorganised (`move_cards.rs` is
now `ability/effects/move/mod.rs`, `ability_queue.rs` files no longer exist,
`main.rs` no longer exists) and the C comments still point at the old names.

## 3a. The 6 hand-proven false claims

Each of these was checked against the field census in `CENSUS_fields.md`.

### `src/core/game_state_core.c:13`

- **Comment says:** claims `cards_paid_cost_this_turn` has "no GameState field (see report)"
- **Reality:** FALSE. `int cards_paid_cost_this_turn[RB_MAX_ZONE];` is declared at rabuka.h:1490 and has 2 write sites and 1 read site.

### `src/core/game_state_core.c:14`

- **Comment says:** claims `player1/2_cheer_revealed_cards` have "no GameState field"
- **Reality:** FALSE. Declared at rabuka.h:1518-1523. Both are 0 writers / 0 readers, so the comment reached the right operational conclusion by asserting something false about the header.

### `src/core/game_state_core.c:15`

- **Comment says:** claims `revealed_cost_cards / *_card_meta` have "no GameState field"
- **Reality:** FALSE. `int revealed_cost_cards[RB_MAX_ZONE];` declared at rabuka.h:1527, 0 writers / 0 readers.

### `include/rabuka.h:240`

- **Comment says:** asserts the three `success_zone_*_bonuses` ledgers are load-bearing: "Rust CLEARS and re-applies them on every live ... so in C a previous live's success-zone bonus can leak into the next one", and "Reachability: 214 constant effect nodes"
- **Reality:** FALSE on all counts. All three are 0 writers and 0 readers (rabuka.h:249-251). Nothing clears them, nothing applies them, nothing reads them. The leak it warns about cannot occur, because the ledger is never written at all.

### `include/rabuka.h:1824`

- **Comment says:** asserts `constant_need_heart_global` is "consumed by modify_required_hearts_global which is reachable from 3 abilities - NOT 0 as AUDIT_constraint_condition.md claimed"
- **Reality:** FALSE. 0 writers, 0 readers. It corrects one audit by asserting the existence of three consumers; there are none. The field is defined next to this comment at rabuka.h:1828 and nothing can observe it.

### `src/core/modifiers.c:904`

- **Comment says:** states `:2018 mods.opp_cause_fired_keys -> LANDED, field at rabuka.h:270-271`
- **Reality:** FALSE twice. The field is at rabuka.h:282-283, not 270-271 (stale citation, 12 lines of drift), and `opp_cause_fired_keys` / `n_opp_cause_fired_keys` are 0 writers / 0 readers, so "LANDED" is false - only the declaration landed.

### Also load-bearing and already self-corrected

`src/core/game_state_abilities.c:2338` reads: *"the `C has no such vector` note
above this function was false from A8 onward"*. The lie is recorded as having
been a lie, which is the right outcome - it is listed here because the original
false note was in this tree and was load-bearing for at least one wave.

### Claims checked and found TRUE (not counted)

- `game_state_core.c:16` "structured_log / debug_trace rings - no GameState fields" - TRUE, no such field anywhere.
- `game_state_core.c:17` "last_ability_trace - no GameState field" - TRUE.
- `game_state_core.c:18` "current_turn_phase - no GameState field (derived here)" - TRUE.
- `rabuka.h:3396` "there is no `immune` symbol anywhere in the tree ... it has no writer yet" - TRUE and self-aware.
- `turn/phase.c:129` "include/rabuka.h - which has no such field" - TRUE.
- `turn/live.c:1969` "breakdown.scores / breakdown.transforms / triggered_abilities remain unwritable because RbLiveSnapshot has no such fields" - TRUE.

## 3b. Cited-file-missing, grouped (all 538)

| file | stale citations |
|---|---|
| `src/ability/choice.c` | 30 |
| `tests/test_parity_executor_dispatch.c` | 25 |
| `tests:test_parity_executor_dispatch.c` | 25 |
| `src/ability/effects/misc.c` | 21 |
| `tests/test_parity_deploy_recover.c` | 21 |
| `tests:test_parity_deploy_recover.c` | 21 |
| `src/ability/effects/move.c` | 16 |
| `tests/test_parity_draw_until_count.c` | 14 |
| `tests:test_parity_draw_until_count.c` | 14 |
| `src/ability/effects/draw.c` | 13 |
| `tests/test_parity_custom_misc.c` | 13 |
| `tests/test_parity_queue_resume.c` | 13 |
| `tests:test_parity_custom_misc.c` | 13 |
| `tests:test_parity_queue_resume.c` | 13 |
| `src/ability/effects/move_missing.c` | 11 |
| `tests/test_parity_draw_delta.c` | 11 |
| `tests:test_parity_draw_delta.c` | 11 |
| `src/engine.c` | 10 |
| `tests/test_parity_characterization.c` | 9 |
| `tests:test_parity_characterization.c` | 9 |
| `tests/test_parity_deploy_move_missing.c` | 8 |
| `tests:test_parity_deploy_move_missing.c` | 8 |
| `src/ability/effects/executor.c` | 7 |
| `tests/test_choice_renderer.c` | 7 |
| `tests:test_choice_renderer.c` | 7 |
| `src/ability/choice_renderer.c` | 6 |
| `tests/test_parity_card_score.c` | 5 |
| `tests/test_parity_char2.c` | 5 |
| `tests/test_parity_draw_flat.c` | 5 |
| `tests/test_parity_realcard_walkthrough.c` | 5 |

Most-cited dead references (a comment that says "mirrors X.rs:N" and X no
longer exists):


## 3c. Cited-line-past-eof (all 57)

| comment at | cites | why it cannot resolve |
|---|---|---|
| `src/ability/choice.c:5140` | `compound.rs:514` | 146 |
| `src/ability/compound.c:447` | `compound.rs:257` | 146 |
| `src/ability/compound.c:509` | `compound.rs:405` | 146 |
| `src/ability/compound.c:527` | `compound.rs:439` | 146 |
| `src/ability/compound.c:606` | `compound.rs:632` | 146 |
| `src/ability/compound.c:618` | `compound.rs:684` | 146 |
| `src/ability/compound.c:664` | `compound.rs:805` | 146 |
| `src/ability/compound.c:712` | `compound.rs:883` | 146 |
| `src/ability/compound.c:773` | `compound.rs:964` | 146 |
| `src/ability/compound.c:804` | `compound.rs:989` | 146 |
| `src/ability/resolver.c:43` | `log.rs:213` | 159 |
| `src/ability/effects/draw.c:535` | `compound.rs:506` | 146 |
| `src/ability/effects/score.c:545` | `tests/test_parity_score_effects.c:1125` | 1024 |
| `src/ability/effects/score.c:967` | `score.rs:686` | 600 |
| `src/ability/effects/score.c:992` | `score.rs:736` | 600 |
| `src/ability/effects/score.c:1003` | `score.rs:761` | 600 |
| `src/ability/effects/score.c:1025` | `score.rs:792` | 600 |
| `src/ability/effects/state.c:1377` | `state.rs:1634` | 1444 |
| `src/ability/effects/state.c:1425` | `state.rs:1629` | 1444 |
| `src/ability/effects/state.c:1510` | `state.rs:1699` | 1444 |
| `src/ability/effects/state.c:1520` | `state.rs:1722` | 1444 |
| `src/core/game_state_core.c:111` | `types.rs:1310` | 1173 |
| `src/game/game_setup.c:1` | `platforms/gba/src/deck_builder.rs:889` | 204 |
| `tests/replay.c:990` | `cost.rs:760` | 439 |
| `tests/replay.c:1025` | `cost.rs:766` | 439 |
| `tests/test_audit_fields.c:616` | `types.rs:1404` | 1173 |
| `tests/test_parity_state_effects2.c:254` | `state.rs:1570` | 1444 |
| `tests/test_parity_state_effects2.c:797` | `state.rs:1626` | 1444 |
| `tests/test_parity_state_effects2.c:808` | `state.rs:1817` | 1444 |
| `tests/test_parity_state_effects2.c:823` | `state.rs:1676` | 1444 |
| `tests/test_parity_state_effects2.c:839` | `state.rs:1718` | 1444 |
| `tests/test_parity_state_effects2.c:853` | `state.rs:1821` | 1444 |
| `tests/test_parity_state_effects2.c:886` | `state.rs:1590` | 1444 |
| `tests/test_parity_state_effects2.c:894` | `state.rs:1610` | 1444 |
| `tests/test_parity_state_effects2.c:977` | `state.rs:1521` | 1444 |
| `tests/test_parity_state_effects2.c:988` | `state.rs:1534` | 1444 |
| `tests/test_parity_state_effects2.c:1002` | `state.rs:1526` | 1444 |
| `tests/test_parity_state_family.c:578` | `tests/test_parity_score_effects.c:1125` | 1024 |
| `tests/test_parity_x_class.c:266` | `score.rs:792` | 600 |
| `tests/test_parity_x_class.c:273` | `score.rs:792` | 600 |
| `tests:replay.c:990` | `cost.rs:760` | 439 |
| `tests:replay.c:1025` | `cost.rs:766` | 439 |
| `tests:test_audit_fields.c:616` | `types.rs:1404` | 1173 |
| `tests:test_parity_state_effects2.c:254` | `state.rs:1570` | 1444 |
| `tests:test_parity_state_effects2.c:797` | `state.rs:1626` | 1444 |
| `tests:test_parity_state_effects2.c:808` | `state.rs:1817` | 1444 |
| `tests:test_parity_state_effects2.c:823` | `state.rs:1676` | 1444 |
| `tests:test_parity_state_effects2.c:839` | `state.rs:1718` | 1444 |
| `tests:test_parity_state_effects2.c:853` | `state.rs:1821` | 1444 |
| `tests:test_parity_state_effects2.c:886` | `state.rs:1590` | 1444 |
| `tests:test_parity_state_effects2.c:894` | `state.rs:1610` | 1444 |
| `tests:test_parity_state_effects2.c:977` | `state.rs:1521` | 1444 |
| `tests:test_parity_state_effects2.c:988` | `state.rs:1534` | 1444 |
| `tests:test_parity_state_effects2.c:1002` | `state.rs:1526` | 1444 |
| `tests:test_parity_state_family.c:578` | `tests/test_parity_score_effects.c:1125` | 1024 |
| `tests:test_parity_x_class.c:266` | `score.rs:792` | 600 |
| `tests:test_parity_x_class.c:273` | `score.rs:792` | 600 |

