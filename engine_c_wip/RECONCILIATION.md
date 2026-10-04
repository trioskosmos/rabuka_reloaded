# RECONCILIATION — ground truth vs `PARITY_CHECKLIST.md`

**Reconciler:** reconciliation agent. **No source file was modified.** Read-only
inspection of the live tree plus a byte-identical snapshot build.

- Snapshot: `C:/Users/trios/AppData/Local/Temp/kilo/recon/`, taken **2026-10-03 20:41:37**.
- Snapshot fidelity: 226/226 `.c`/`.h` files SHA-256-identical to the live tree,
  0 `.o` files, 0 engine binaries copied. Hash manifest at `recon/MANIFEST.txt`.
- Data blob: `src/abilities_strings.bin` = **726831 B / `89204df0bbde08a0…`** verified
  before the build, after `make all`, and after `make web`. `make data` was **never run**.
- ⚠ The live tree was **still being written during this reconciliation**. See §5.

---

## 1. Mechanical inventory of the checklist

`PARITY_CHECKLIST.md` contains **167 checkbox items** on lines matching
`^\s*[-*]\s*\[( |x|~)\]`:

| claimed status | count |
|---|---|
| `[x]` closed | **79** |
| `[~]` in progress | **3** (B1, D1, A19) |
| `[ ]` open | **85** |

(The brief's "80 closed / 87 open" is off by one in each direction: 79 and 85 on the
checkbox count. A 86th and 87th `[ ]` exist as *prose* inside T4-8 and T3-1b, which is
where the "87" comes from.)

**This count cannot be trusted as a record of what is in the tree, and it is not.**
24 items are in the wrong bucket — 12 closed-claimed items are not closed, and
12 open-claimed items are closed or obsolete.

---

## 2. Actual status, per item

Legend: `LANDED` / `PARTIAL` (say which half) / `OPEN` / `OBSOLETE`.
`NOT-VERIFIED` = I ran out of budget; I will not guess. It is counted as not-closed.

### A. Header — `include/rabuka.h` (owner: **main**)

| id | claimed | ACTUAL | evidence (file:line) | file that must change |
|---|---|---|---|---|
| A1 | `[x]` | LANDED | `rabuka.h:985-986`, setter decl `:3370` | — |
| A2 | `[x]` | LANDED | `rabuka.h:3410` `RbSetupActionType`, `:3431` `RbSetupAction` | — |
| A3 | `[x]` | LANDED | `rabuka.h:85` `RB_COND_UNSUPPORTED = 20` | — |
| **A4** | `[x]` | **PARTIAL** | field exists `rabuka.h:1794-1795`; **0 writers, 0 readers in all of `src/`**; `game_state_core.c:712-714` still routes `rb_gsc_push_rule_log` into `rb_log_push_verdict(text,"rule_log",1)` (the shared **32-slot** ring). Also the checklist's dimension is wrong: line is **256** (`rabuka.h:1453`), not 192. | `src/core/game_state_core.c` |
| A5 | `[x]` | LANDED | `rabuka.h:1261-1292` | — |
| A6 | `[x]` | LANDED | `rabuka.h:1342-1343` | — |
| A7 | `[x]` | LANDED | `rabuka.h:1488-1489, 1506-1509, 1512-1513` | — |
| A8 | `[x]` | LANDED (fields) | `rabuka.h:1520-1521`, `:270-271`, `:1502-1503`; **written** by `modifiers.c:769-774`, `modifiers.c:926`, `state.c:546-549`. 2 of the 3 have **no reader** — see E17, T2-12. | — |
| **A9** | `[x]` | **PARTIAL** | `int cause_card_id;` declared `rabuka.h:1440`; **0 writers and 0 readers anywhere in `src/`**. A9's own stated purpose (unblocking E16) is therefore **not** achieved. | `src/core/modifiers.c` (the `eval_group` temporal arm) |
| A10 | `[x]` | LANDED | `rabuka.h:1563-1564` | — |
| A11 | `[x]` | LANDED | `rabuka.h:944-946`, `:1121` all 512 | — |
| A12 | `[x]` | LANDED | `rabuka.h:3524`, def `card.c:79`, used `util.c:3035`, `dynamic_count.c:253` | — |
| A13 | `[x]` | LANDED | `rb_ability_ref_flush` absent from `rabuka.h` and all `src/` | — |
| A14 | `[x]` | LANDED | all 12 resolve: `rabuka.h:3508,3509,3512,3516,3518,3519,3157` + `static inline` `rb_heart_index:2107`, `rb_modifier_total:199`, `rb_saturate_i16:186`, and `rb_count_u32`/`rb_id_u32`/`rb_f32_count`/`rb_effective_stat`/`rb_overridden_stat`/`rb_zone_matches_source` | — |
| A15 | `[x]` | LANDED | `rabuka.h:3640, 3645` | — |
| A16 | `[x]` | LANDED | def `cost.c:1808`, decl `rabuka.h:3656`, **1 caller** `engine.c:1187` | — |
| **A17** | `[x]` | **PARTIAL** | six carriers declared `rabuka.h:2500-2516`. `card_filter_from_effect` in `card.c` writes **none** of them: `card.c:965` sets only `out->has_group=1`; `card.c:1107` `has_filter` reads only `card_type`/`has_group`/`has_cost_limit`. `exclude_group_names` is *read* at `card.c:644` but `->exclude_group_names`/`->original_heart_operator`/`->cost_limit_min`/`->cost_limit_max`/`->n_groups`/`->exclude_cards`/`->exclude_names` have **zero writes**. | `src/core/card.c` |
| A18 | `[x]` | LANDED | `rabuka.h:1363-1364`; written `live.c:1562-1563` | — |
| **A19** | `[~]` | **PARTIAL** | `base_score_total`/`card_bonus_total` now in header `rabuka.h:1354-1355` and **written** `live.c:982-983`; `transform_delta` in `RbMemberContribution` written `live.c:1583`. **`spare`/`live_spare` still has 0 writers** (`rabuka.h:1356` declares, nothing writes). | `src/turn/live.c` |

### B. Queue / choice — `src/ability/choice.c` (owner: **cluster B — NEVER REPORTED**)

`choice.c` is **CLEAN in git** (committed, no uncommitted work) → cluster B never wrote.

| id | claimed | ACTUAL | evidence | file that must change |
|---|---|---|---|---|
| **B1** | `[~]` | **LANDED** | `rb_queue_complete_current` def `ability_queue.c:339`; called `choice.c:4831` and `:5187`, both guarded on `!rb_has_pending_choice`. Verified by T4-4 and independently by me. | — |
| B2 | `[ ]` | OPEN | `choice.c:2144` `choice_build_reprompt(g,actor,"hand",remaining,en,ja,0,NULL,0)` — passes **no `filtered_indices`** for the chosen unit. | `src/ability/choice.c` |
| B3 | `[ ]` | NOT-VERIFIED | `choice_cost_opens_hand_prompt` def `choice.c:1114`, called `:1259` (checklist cites `:1120` — drifted). Same-unit exclusion not checked. | `src/ability/choice.c` |
| **B4** | `[ ]` | **LANDED** | `rb_resolver_spawn_target` def `choice.c:333`, **called** `choice.c:600`. It is wired, not orphaned. (Stale counter-claim at `misc.c:1856`.) | — |
| B5 | `[ ]` | PARTIAL | 1 of 3 hoisted: `rb_queue_is_entry_available` → `rabuka.h:1869`. `rb_queue_entry_player_id` and `rb_queue_clear_completed` are still **hand-written local externs** at `game_state_abilities.c:2974-2975`. | `include/rabuka.h` |

### C. Cost handlers — `cost.c`, `effects/rules.c`, `effects/misc.c` (owner: **cluster C — NEVER REPORTED**)

All three files **CLEAN in git** → cluster C never wrote.

| id | claimed | ACTUAL | evidence | file that must change |
|---|---|---|---|---|
| C1 | `[ ]` | OPEN | `game_setup.c:1199` still `if (!rb_validate_cost(g, actor, ab->cost)) return 0;` | `src/game/game_setup.c` |
| **C2** | `[ ]` | **LANDED** | `engine.c:1187` `if (ab->cost && !rb_validate_mandatory_cost(g, pl, ab->cost, card_id))`, inside `activate_resolved_ability` (`engine.c:1130`). Landed by T6, not by C. | — |
| C3 | `[ ]` | OPEN | `hand_cost_matching_indices` — **0 occurrences in the entire tree.** | `src/ability/cost.c` |
| **C4** | `[ ]` | **LANDED** | `cost.c:1418-1419` `else if (!strcmp(state_change,"rest")\|\|!strcmp(state_change,"rested")) { rb_mods_set_orientation(&g->mods, cid, "rest"); }` | — |
| C5 | `[ ]` | OPEN | all four wrappers present: `cost.c:2251, 2256, 2264, 2273`; decls `rabuka.h:2682, 2684, 2686, 2688` | `src/ability/cost.c` + `include/rabuka.h` |
| C6 | `[ ]` | OPEN | `cost.c:26` decl, written `:1444` and `:2097`, **0 readers** | `src/ability/cost.c` |
| C7 | `[ ]` | OPEN | `pay_cost_move_cards` is itself dead (C5), so the gate question is moot until C5 lands | `src/ability/cost.c` |

### D. Condition — `src/ability/condition.c` (owner: **cluster D — NEVER REPORTED**)

`condition.c` is **CLEAN in git** → cluster D never wrote.

| id | claimed | ACTUAL | evidence | file that must change |
|---|---|---|---|---|
| **D1** | `[~]` | **OPEN** | `evaluate_front_comparison` — **0 occurrences tree-wide.** The "[~] agent running" claim is false; nothing landed. | `src/ability/condition.c` |
| D2 | `[ ]` | PARTIAL | Inlined at `condition.c:2353` and `:2365` as `has_all = (cc.n_hearts>0 && cc.heart_count[0]>0)` — the **printed Heart00 base only**. The `heart_modifier(cid,All)>0` and `constant_heart_bonuses[cid][All]>0` terms are **absent**. No named `card_has_all_heart`. | `src/ability/condition.c` |
| D3 | `[ ]` | PARTIAL | `_colors` half **wired** (`condition.c:2380` def, called `:508`). `_all` half **defined at `condition.c:2336` with ZERO callers.** Its negation arm (`:2344-2358`) is the wrong shape — it implements "every member must have all-heart", not Rust's three negation-skip exceptions, and it computes `int target` at `:2346` and never uses it (gcc `-Wunused-variable`, confirmed in the build log). | `src/ability/condition.c` |
| D4 | `[ ]` | OPEN | no `evaluate_comparison_condition` symbol; only a provenance comment at `condition.c:775` | `src/ability/condition.c` |
| D5 | `[ ]` | OPEN | `condition.c:616` `int center_cid = P->stage[1];` hardcoded; `:619` `center.cost` and `:628` `cc.cost` are **printed** cost; the `position` slot is never read | `src/ability/condition.c` |
| D6 | `[ ]` | OPEN | `condition.c:3058`; its own note at `:2919-2923` records the port as "much smaller" than the 225-line Rust original | `src/ability/condition.c` |
| D7 | `[ ]` | OPEN | no `no_excess_heart_flag` in `condition.c`; only `rabuka.h:1556` (a comment) and a describe-side reimplementation `describe.c:2335` | `src/ability/condition.c` |
| D8 | `[ ]` | OPEN | `condition.c` has **0 references** to `cards_activated_this_turn` — the `activated_this_turn` reader is missing even though T2-2 landed the writer | `src/ability/condition.c` |

### E. Modifiers — `src/core/modifiers.c` (owner: **cluster E — NEVER REPORTED**)

`modifiers.c` is **CLEAN in git** and its mtime is `10-03 01:49`, i.e. the oldest of
any file in the module list. **Cluster E wrote nothing at all.** 17 items, 0 landed.

| id | claimed | ACTUAL | evidence | file that must change |
|---|---|---|---|---|
| E1 | `[ ]` | OPEN | comment-only mentions: `modifiers.c:974`, `triggers.c:1505`. No definition. | `src/core/modifiers.c` |
| E2 | `[ ]` | OPEN | comment-only mention: `triggers.c:941` | `src/core/modifiers.c` |
| E3 | `[ ]` | OPEN | **0 occurrences tree-wide** | `src/core/modifiers.c` |
| E4 | `[ ]` | OPEN | comment-only mention: `rabuka.h:1328` | `src/core/modifiers.c` |
| E5 | `[ ]` | OPEN | **0 occurrences tree-wide** | `src/core/modifiers.c` |
| E6 | `[ ]` | OPEN | **0 occurrences tree-wide** | `src/core/modifiers.c` |
| E7 | `[ ]` | PARTIAL | `rb_mods_set_heart_override` def `modifiers.c:262`, decl `rabuka.h:311`, **0 callers** (never wired at `choice.c:3806`'s equivalent) | `src/core/modifiers.c` / `src/ability/choice.c` |
| **E8** | `[ ]` | **LANDED** | `rb_clear_gained_abilities_for_card` def `game_state_abilities.c:253`, called `:302`. Already present on arrival — the item was stale. | — |
| E9 | `[ ]` | OPEN | comment-only: `choice.c:858`. T2 measured it display-only; either way no writer exists. | `src/core/modifiers.c` |
| E10 | `[ ]` | OPEN | **0 occurrences tree-wide** | `src/core/modifiers.c` |
| E11 | `[ ]` | OPEN | comments only: `executor.c:87`, `rabuka.h:640` | `src/core/modifiers.c` |
| E12 | `[ ]` | OPEN | comments only: `executor.c:83`, `rabuka.h:636` | `src/core/modifiers.c` |
| E13 | `[ ]` | PARTIAL | **blade half wired**: `rb_mods_add_blade_with_trace` def `modifiers.c:395`, called `:371`. **heart half not**: `rb_mods_add_heart_with_trace` def `modifiers.c:406`, decl `rabuka.h:325`, **0 callers**. Depends on F3, which is open. | `src/core/modifiers.c` + `src/ability/effects/misc.c` |
| E14 | `[ ]` | PARTIAL | 8 of 9 setters exist with 0 callers (`rb_mods_remove_blade/_heart/_score`, `rb_mods_clear_blade_set/_score_set/_blade_type`, `rb_mods_remove_heart_override`, `rb_mods_get_cost_set`). **`rb_mods_remove_cost_set` — the 9th name — has 0 occurrences tree-wide**; the checklist name is wrong. | `src/core/modifiers.c` + `include/rabuka.h` |
| E15 | `[ ]` | OPEN | still defined `modifiers.c:222`, still declared `rabuka.h:306` | `src/core/modifiers.c` + `include/rabuka.h` |
| E16 | `[ ]` | OPEN **and blocked** | `eval_group`'s temporal arm has no key to read: `cause_card_id` (A9) has 0 writers | `src/core/card.c` first, then `modifiers.c` |
| E17 | `[ ]` | OPEN | file-static `s_opp_cause_fired[]` still live at `game_state_abilities.c:2093-2124`, while `mods.opp_cause_fired_keys` is cleared at `modifiers.c:926` | `src/core/game_state_abilities.c` |

### F. Misc / effects (owner: **cluster F — NEVER REPORTED**)

`executor.c`, `draw.c`, `move.c`, `dynamic_count.c` all **CLEAN**; only `custom.c` is dirty.

| id | claimed | ACTUAL | evidence | file that must change |
|---|---|---|---|---|
| **F1** | `[ ]` | **LANDED — and the premise is INVERTED** | The live copy has flipped: `executor.c:408` now calls `rb_execute_custom_effect` (`custom.c:278`) directly. `h_custom` (`misc.c:1052`) survives but is now only reachable through the CONSTANT scan (`triggers.c:1208` → `misc.c:2073`), which T2 measured as 0 of 985. Residue: `h_custom` not deleted. | `src/ability/effects/misc.c` (delete residue only) |
| **F2** | `[ ]` | **OBSOLETE** | T7-h measured `heart_colors_from_selected_card` at 0 of 985; not a gap. | — |
| F3 | `[ ]` | OPEN | `misc.c` has **0 occurrences** of `ability_text` — nothing is threaded. | `src/ability/effects/misc.c` |
| F4 | `[ ]` | LANDED (verified) | per-unit arms present: `dynamic_count.c:503, 655`, `score.c:575, 621, 869`, `state.c:119, 1477`, `util.c:2693, 2767, 2827`, `draw.c:501-531` | — |
| **F5** | `[ ]` | **LANDED** | one decoder: def `live.c:82`; callers `live.c:239`, `phase.c:1624`, `rules.c:2102`. `r_yell_icons` and `ptc_yell_had_draw_icon` survive only as prose (`rules.c:1993`, `live.c:74`, `phase.c:1610`). | — |
| **F6** | `[ ]` | OBSOLETE / LANDED | (a) `rb_zone_of_str` folding `deck_bottom` is **correct** (I7, `engine.c:145`); (b) the dead `rb_effect_move_cards` body is gone (`move.c:2862`). The hand fallback at `draw.c:91-94` is correct (0 corpus occurrences of `destination:""`). Residual real bug: `draw.c:136-137` derives the seat from the **target** player. | `src/ability/effects/draw.c` |
| **F7** | `[ ]` | **LANDED** | all three source arms present: `move.c:1529` `recently_moved`, `:1543` `revealed_cards`, `:1577` `those_cards` | — |
| F8 | `[ ]` | NOT-VERIFIED | claimed "DONE by agent; verify"; I did not isolate the multi-index stage answer | `src/ability/effects/move.c` |

### G / H (owner: **cluster G+H — reported inline in the checklist §"Clusters G and H were closed"**)

| id | claimed | ACTUAL | evidence |
|---|---|---|---|
| G1 | `[x]` | LANDED (no code by design) | accept |
| G2 | `[x]` | LANDED (no code by design) | accept |
| **G3** | `[x]` | **PARTIAL** | The `card.c` definition is gone and there are 0 callers — correct. But the *declaration was not deleted*: `rabuka.h:2532` now carries a `static inline int rb_card_filter_subset(...)` wrapper, and the tombstone comment directly above it (`rabuka.h:2520-2522`) still asserts "this line was removed". G3's letter is false; its substance holds. |
| **G4** | `[x]` | LANDED + open follow-up | `rb_effect_fires_on_opponent_effects` card-level fallback removed. Follow-up: `rb_card_fires_on_opponent_effects` in `card.c` and the `extern` at `game_state_abilities.c:104` are not deleted. |
| **H1** | `[x]` | **PARTIAL** | Matcher side landed: carriers `rabuka.h:2500-2516`, `util.c:1876` sets `has_original_heart_total`, `util.c:1701` reads it. **Producer side absent** (see A17). H1's own text admits this, so `[x]` is defensible, but every corpus number it quotes is therefore measured on dead code. |
| **H2** | `[x]` | **PARTIAL** | all four prototypes are in the header (`rabuka.h:3349-3357`) ✓. The four local twins it says "are still to be deleted" are still present: `cost.c:252`, `score.c:89`, `misc.c:266`, `ability_effects.c:82`. |

### I. Live / phases / engine (owner: **cluster I — reported inline**)

| id | claimed | ACTUAL | evidence |
|---|---|---|---|
| **I1** | `[x]` | LANDED (+1 blocker now cleared) | `rb_enrich_from_applications` def `live.c:1553`, called `:735`. `rb_build_snapshot`/`rb_check_live_success` absent as claimed. **`base_score_total`/`card_bonus_total` are now in the header (`rabuka.h:1354-1355`) and written (`live.c:982-983`)** — I1's BLOCKED-ON is resolved. `spare`/`live_spare` still unwritten. |
| **I2** | `[x]` | **HALF-DONE — declaration with no definition, no call** | `void rb_rules_clear_turn_state(GameState *g);` is declared **twice** — `live.c:604` and `phase.c:529` — and is **defined nowhere and called nowhere**. The reader half landed (`rules.c:2245` def, called `live.c:617`). `cannot_live_players[2]` (`rules.c:72`) written `rules.c:1878`, **never cleared**. This is a live landmine: adding the call site turns it into a link error. |
| **I3** | `[x]` | LANDED, BLOCKED-ON still open | `rb_compute_snapshot_surplus` def `live.c:1136`, called `:2137` ✓. But `count_surplus_heart` (`condition.c:3276-3300`) has **no `live_surplus_ready_this_turn` gate and no per-colour read** — it recomputes `member - need` from printed hearts. `live_surplus_ready_this_turn` has **0 hits in condition.c**. The new values are unread. |
| **I4** | `[x]` | PARTIAL | `live_spare` field now exists (`rabuka.h:1356`) — the named blocker is cleared — but has **0 writers**. |
| I5 | `[x]` | LANDED | independently verified: `phase.c:550-552` clears `player_cannot_activate[0/1]` + `n_cannot_active_cards`; `phase.c:554-555` clears `baton_touch_used[0/1]`. |
| **I6** | `[x]` | **OPEN — claimed closed, it is not** | `rb_hooked_by_foreign_cause` (`game_state_abilities.c:2265`) still scans the wrong vector; its own comment at `:2260` says *"the C has no such vector"* — which A8 made **false** (`rabuka.h:1520-1521`, written `modifiers.c:769-774`, cleared `:925`). The reader was never swapped. |
| I7 | `[x]` | LANDED + stale prose | `rb_zone_of_str` `engine.c:145`; stale comments describing the old lying table survive at `engine.c:130-136`, `draw.c:66`, `modifiers.c:610`. |
| I8 | `[x]` | LANDED | accept (web_server.c clean) |
| I9 | `[x]` | LANDED | accept (web_server.c clean) |

### J. Test-side (owner: **cluster J — landed; `tests/**` now CONTESTED by T8**)

| id | claimed | ACTUAL |
|---|---|---|
| J1–J8, J10–J12 | `[x]` | accept as landed (owning test files are dirty in git, consistent with the claims) |
| **J9** | `[x]` | **PARTIAL — 2 of its 4 rows are still open.** Row **F** (`watches_ability_resolution`): **0 occurrences in all of `src/` and `include/`**. Row **M** (`has_original_heart_total` producer): `card.c` has 0 writes. Rows D and N closed. |

### K. Data / artifacts (owner: **cluster K — reported; also the K7 agent**)

| id | claimed | ACTUAL | evidence |
|---|---|---|---|
| K1 | `[x]` | LANDED, verified by me | `guard_data_blob.sh` exits **1** and REFUSES. `make data` is correctly RED. |
| K2 | `[x]` | OPEN | `src/bytecode_data.c` still in `Makefile:25` `SRC` and `:28` `OBJ_LIB`; the `BYTECODE_C0..C3` externs still in `abilities_gen.c:7-11` |
| K3–K6 | `[x]` | accept as landed | — |
| **K7** | `[ ]` | **OPEN — unchanged** | `cards/build/abilities_strings.bin` = **726649 B / `6225B9B5…`**; `cards/build/abilities.bin` = **98733 B** vs `gen_data.c`'s `RBKA_BYTECODE_LEN = 98737`. No regeneration has run. |
| K8 | `[ ]` | OPEN | no m68k cross toolchain on this machine |

### T5 (owner: **cluster T5 — reported**)

| id | claimed | ACTUAL | evidence |
|---|---|---|---|
| T5-1 | `[ ]` | OPEN | no display layer in the C |
| T5-2 | `[ ]` | OPEN | `compound.c:508-513` `record_step_output` |
| T5-3 | `[ ]` | OPEN | documented card-identity decision, no code |
| T5-4 | `[ ]` | OPEN | unresolvable without the missing oracle |
| T5-5 | `[ ]` | OPEN | same root cause as K7 |
| T5-6 | `[ ]` | OPEN | documented divergence |
| T5-7 | `[ ]` | OPEN | `test_deck_builder.c:113` non-static `GameState`. Confirmed still unfixed: `test_harness_self` reports **1432 non-static state locals across 75 files** still present. |
| **T5-8** | `[ ]` | **LANDED / OBSOLETE** | `Makefile:25` lists `src/ability/choice_renderer.c` in `SRC`; `Makefile:28` lists its `.o` in `OBJ_LIB`. `RbRendUi` is at `rabuka.h:3269-3288`. `rb_rend_render_card_choice_grid` is defined `choice_renderer.c:1368`. `tests/test_choice_renderer.c` = **GREEN, 0 checks failed**. Independently resolved by cluster T8 (`ORPHANS_AND_SCOPE.md`, 20:33). |

### T4 + harness defects (owner: **cluster T4 — reported**)

| id | claimed | ACTUAL | evidence |
|---|---|---|---|
| T4-7 | `[ ]` | OPEN | `ability_queue.c:307` defined; only mention outside is a comment `engine.c:920`. 0 production callers. |
| T4-8 | `[ ]` | OPEN | `ability_queue.c:967` defined, `rabuka.h:1876`; **0 callers**. `compound.c` (the blocker) is **CLEAN** → free. |
| T4-9 | `[ ]` | OPEN | all three defined `ability_queue.c:626, 787, 804`; two declared `rabuka.h:1872-1873`; **0 callers** |
| T4-10 | `[ ]` | OPEN | `rb_drain_ability_queue(g)` still at `triggers.c:415`, inside `rb_trigger_live_start` |
| T4-11 | `[ ]` | OPEN | `triggers.c:425` returns cached `g->live_success[pl]`; no phase term; called `triggers.c:478` |
| T4-12 | `[ ]` | OPEN | blocked on E3, which does not exist |
| T4-13 | `[ ]` | OPEN | `triggers.c:1641` defined, `rabuka.h:419`; 0 callers |
| **H-1** | `[ ]` | **LANDED** | `test_audit_triggers.c` now emits **`FAIL:` WITH a colon** — observed verbatim in the run log. The trap it documents no longer applies to this file. |
| **H-2** | `[ ]` | **OBSOLETE** | The three rows it names (A2, A3, A4b) all report `ok`. The suite is **10 ok / 1 FAILED**, and the single failure is a different, previously unlisted row: `A7_reveal_cost_values` — *"RbChoice (rabuka.h:913-940) has cost_limit/cost_limit_op/characters/is_reveal but NO cost_values"*, reachable at `PL!SP-bp7-003-P ab#2`. |
| **H-3** | `[ ]` | **OBSOLETE** | `test_parity_queue_resume` **completes**: exit 1, **3/146 assertions failed**, 7023 B stdout, 1933 B stderr. No abort. |

### T1 (owner: **cluster T1 — reported**)

| id | claimed | ACTUAL |
|---|---|---|
| T1-4 | `[ ]` | OPEN — `vm.c:2715` still delegates to `rb_zone_id_from_str` |
| T1-5 | `[ ]` | OPEN — typed condition decoder still 0 callers |
| T1-6 | `[ ]` | OPEN — `enums.c` still 0 production callers |
| T1-7 | `[ ]` | OPEN — blocked on `include/rabuka.h` |
| T1-8 | `[ ]` | OPEN — `compound.c` is CLEAN, so the blocker is free |
| T1-9 | `[ ]` | OPEN — blocked on `src/core/card.c`, which is CLEAN |

### T3 (owner: **cluster T3 — reported**)

| id | claimed | ACTUAL | evidence |
|---|---|---|---|
| **T3-1** | `[x]` | **LANDED — verified by me** | `test_audit_zones` = **35 cases / 223 assertions / 3 failures**, matching the claimed 9 → 3 exactly. |
| **T3-1b** | `[ ]` | **OBSOLETE** | The assertion it names no longer exists. `test_parity_full_round2.c:780` is now `CHECK(!rb_has_pending_choice(&tg.state), "N3: nothing is left pending")`. The fixture still answers `left` at `:778` and `N3_sync_opponent_gets_choice` is still registered at `:1729`; I did not measure whether it is red. |
| **T3-2** | `[ ]` | **HALF-DONE** | `rabuka.h:1794-1795` now has `rule_log[500][256]` + `rule_log_len[]` (256, not the 192 claimed). **0 writers, 0 readers.** `game_state_core.c:712-714` still routes to `rb_log_push_verdict(text,"rule_log",1)` and `game_state_core.c:707` still states *"The C GameState has no `rule_log: Vec<String>` ring"*. |
| T3-3 | `[x]` | LANDED | 45 prototypes landed. 908 `rb_*` header prototypes now resolve except the 7 orphans in §4. |
| **T3-4** | `[ ]` | OPEN + **WRONG PATH** | The cited blocker `src/core/state.c` **does not exist** — `src/` has exactly `ability/ core/ game/ turn/`. The real file is `src/ability/effects/state.c` (same line numbers: `:10`, `:1677`, `:1692`, `:1732`). `rb_stage_position_change_with_bags` still takes `(RbPlayer*, …)` with no `GameState*` (`zones.c:280`). |
| **T3-5** | `[ ]` | **HALF-DONE** | `rabuka.h:1807-1808` declares `constant_need_heart_global` + `_set`. **0 writers, 0 readers**, and `n_constant_need_heart_global` **does not exist at all**. |
| T3-6 | `[ ]` | OPEN | `success_zone_blade_bonuses` / `_heart_bonuses` / `_score_bonuses` — **0 occurrences anywhere**, header or src. Not in main's header bill. |
| T3-7 | `[ ]` | PARTIAL | `exclusion_zone` still global (`rabuka.h:1604`). `live_face_up` declared `rabuka.h:1801` with **0 writers, 0 readers**. |
| T3-8 | `[ ]` | OPEN (documented-correct divergences) | accept |
| T3-9 | `[ ]` | OPEN | `rb_zone_of_str` `engine.c:145` |

### T2 (owner: **cluster T2 — reported**)

| id | claimed | ACTUAL | evidence |
|---|---|---|---|
| T2-10 | `[ ]` | OPEN | `misc.c:905-918` `h_restriction` has no `changed_state_members` arm. `rules.c:1819` `rb_rules_execute_restriction` and `:1932` `rb_rules_execute_activation_restriction` both still **0 callers**. Confirmed red by `test_parity_state_family`. |
| T2-11 | `[ ]` | OPEN | `card.c` has **0 writes** to `has_original_heart_total`; the only `heart_total_limit` reader is `describe.c:682` |
| T2-12 | `[ ]` | OPEN | `condition.c` has **0 references** to `cards_activated_this_turn` |
| T2-13 | `[ ]` | OPEN | `test_parity_state_family` §F red: `FAILED: test_live_total_set_is_refused` |
| T2-14 | `[ ]` | OPEN | `util.c:48-49` `#ifdef RB_MODS_LAST_COST_ENERGY_COUNT`, and **nothing defines it** |
| T2-15 | `[ ]` | OPEN, both halves | the position.rs executor still lives in `misc.c`; and **`play_baton_touch` has 0 occurrences in `triggers.c`** |
| T2-16 | `[ ]` | OPEN (documented, unmeasured) | accept |

### T7 (owner: **cluster T7 — reported**)

| id | claimed | ACTUAL |
|---|---|---|
| T7-k | `[ ]` | OPEN — `decode_condition_direct` still 0 callers, `ConditionLocals` still leaks |
| T7-l | `[ ]` | OPEN — `watches_ability_resolution`: **0 occurrences in all of `src/` and `include/`** |

### T6 open items (numbered list, outside the 167 checkboxes)

| # | claimed | ACTUAL | evidence |
|---|---|---|---|
| **1** | open | **LANDED** | `engine.c:1187` — see C2 |
| 2 | open | OPEN | `phase.c:645-655` still has the `p2 <= 2` guard (`:650`, `:652`) and the `winner = 2` draw (`:648-649`) that Rust does not |
| 3 | open | PARTIAL | `phase.c:712` `rb_check_invalid_resolution_zone` now exists and is called (`:779`); the **owner half is still inexpressible** (no `resolution_owners` writer). Stale comment at `zones.c:745`. |
| 4 | open | OPEN | `phase.c:657` `rb_check_invalid_live_cards` has no `face_up` gate; `live_face_up` (`rabuka.h:1801`) has 0 writers |
| 5 | open | OPEN | the CONSTANT scan in `modifiers.c` still has no `action == "restriction"` arm |
| 6 | open | OPEN | `rb_check_permanent_loop` def `game_state_abilities.c:1352`, called `phase.c:783` — still C-only |

---

## 3. Mis-statement tally

| direction | count | items |
|---|---|---|
| claimed `[ ]` / `[~]`, actually **closed or obsolete** | **12** | B1, B4, C2, C4, F1, F2, F5, F7, T5-8, H-1, H-2, H-3, T3-1b, T6-1 → 14 rows, of which **12** change the closed/not-closed count (F2, F5, F7 and T3-1b are reclassified as OBSOLETE/LANDED and were the *reason* some of the remaining rows flipped) |
| claimed `[x]`, actually **not closed** | **12** | A4, A9, A17, G3, H1, H2, I2, I5(no—I5 verified), I6, J9, T3-2, T3-5 → **11 confirmed**, plus A19 (claimed `[~]`, still partial) |

Exact arithmetic on the 167 checkbox items:

- claimed closed 79 → **actual closed 79** (79 − 11 that are not, + 11 that are)
- claimed not-closed 88 (85 open + 3 in-progress) → **actual not-closed 88** (85 − 11 + 3 + 11)
- of the 88 not-closed: **69 confirmed OPEN**, **13 PARTIAL**, **3 NOT-VERIFIED**, **3 OBSOLETE-but-still-flagged**
- outside the checkbox count: 6 T6 numbered items, **1 closed, 5 open**.

**The headline numbers coincide. The contents do not.** A redeploy built from the
checklist would send agents to re-land 11 finished items and would not send anyone
to the 11 items the checklist calls done.

---

## 4. Duplicate live implementations

`execute_gain_ability_from_source` (the pair a peer already found) is
**RESOLVED**: `ability.c:535-536` is now a 1-line delegation to
`rb_ability_gain_from_source_core` (`ability_effects.c:559`), and both live entry
points (`executor.c:476`, `triggers.c:1178`) route through it. No divergence remains.
The `restriction` handler family is still **triplicated** with the live copy the
incomplete one (T2-10, unchanged).

**New pairs found — every one is `static`, so nothing fails to link; all are LIVE
with callers in both files, and they DIVERGE:**

| # | symbol | definitions | live call sites | divergence |
|---|---|---|---|---|
| **N1** | `cached_condition_verdict` | `compound.c:231` (Rust twin `cached_condition_verdict`) · `resolver.c:310` | `compound.c:428` · `resolver.c:765` | `compound.c:233` gates on `cond_get_cache_flag(c)`; `resolver.c:312` gates on `cond_get_cache(cond)`. Two independent copies of the verdict cache; the two files can disagree about whether a verdict is cacheable. |
| **N2** | `card_total_hearts` | `condition.c:3258` · `util.c:705` | `condition.c:3348, 3354, 3361` · `util.c:1667` | `util.c:706` is `heart_count_in_range(c,-1,0,c->num_base)`; `condition.c:3259-3260` builds the sum inline over a different range. `util.c:1693`'s own comment says *"card_total_hearts() is the WRONG total here"* — and the condition.c copy is the one still in use for three call sites. |
| **N3** | `card_appeared_this_turn` | `condition.c:1301` · `misc.c:248` | `condition.c:1397, 1408, 1458` · `misc.c:334` | `condition.c:1302` has the guard `if (cid < 0) return 0;`; `misc.c:248` **has none**, so a `-1` card id falls through to the loop. |
| **N4** | `player_prefix` | `cost.c:489` · `misc.c:127` · `resolver.c:874` | 3 files | **three** copies, and `cost.c`/`misc.c` take `(g, card_id)` while `resolver.c:874` takes `(g)` — a signature split on top of the duplication. |
| **N5** | `effect_extra` | `ability_effects.c:7` · `executor.c:123` | both files | two live static copies of the same extras getter. |
| **N6** | `cond_cache_key` + `store_condition_verdict` | `compound.c:211` + `:241` · `resolver.c:207` + `:320` | both files | the other two halves of N1's duplicated verdict-cache triad. |
| **N7** | `eff_extra` | **5 live statics**: `compound.c:32`, `cost.c:108`, `misc.c:52`, `resolver.c:83`, `util.c:258` | all five | five copies of the extras getter, no shared owner. Low severity (tiny, mechanical) but it is why nozomi found 37 unguarded extras-vs-field reads. |
| **N8** | `remove_card_from_any_zone` | `move.c:476` · `move_missing.c:344` | — | `move_missing.c:344` is `static` and gcc reports it **`defined but not used`** (build log). Dead copy inside a file that *is* compiled. |
| **N9** | `cmf_extra` / `extra_true` | `move.c:166` · `move_missing.c:332, 339` | — | same shape: `move_missing.c` holds a private, unused duplicate of `move.c`'s helpers. |

Not duplicates (checked and cleared): `rb_emit_choice` (choice.c:5314 def,
engine.c:256 prototype), `rb_play_member_ex`/`rb_play_double_baton_ex`
(engine.c defs, phase.c:42-44 prototypes), `rb_setup_for_each_ability_candidate`
(game_setup.c:1323 def, phase.c:55 prototype),
`rb_move_execute_move_cards_ported` (move_missing.c:125 def, move.c:74 prototype),
`rb_get_selection_indices_filter` (util.c:2254 def, move.c:104 tentative prototype).

---

## 5. ⚠ THE TREE IS STILL MOVING

Files written **while this reconciliation was running** (now ≈ 20:41):

| time | file |
|---|---|
| 20:40:43 | `tests/test_harness_self.c` |
| 20:40:20 | `include/test_game.h` |
| 20:39:00 | `src/test_game.c` |
| 20:33:28 | `ORPHANS_AND_SCOPE.md` (new, untracked) |

A **twelfth agent, cluster T8**, is live and owns those four files right now.
`ORPHANS_AND_SCOPE.md` states that a predecessor of T8 *"died twice on infrastructure
errors and left `tests/test_harness_self.c`, `tests/test_choice_renderer.c`,
`tests/test_describe_golden.c`, `tests/fixtures/describe_golden.txt` and the
`Makefile` / `src/test_game.c` / `include/test_game.h` hunks on disk but no decision
document."*

Consequence for the build verdict: `src/test_game.c` is in `OBJ_LIB` but **not** in
`SRC`, so it cannot affect `make all` or `make web`. The build verdict below is
unaffected by the in-flight edits.

### 5a. `src/ability/effects/look.c` was edited at 20:52:20 — after my snapshot

Detected on a post-write re-verification sweep. This file is in `SRC`
(`Makefile:25`), so unlike the three test files it **can** affect a build.

| | snapshot (20:41:37) | live tree (20:52:20) |
|---|---|---|
| size | 72,719 B | **73,402 B (+683)** |
| mtime | `10-02 12:23:47` | `10-03 20:52:20` |

This is almost certainly **H-3** being fixed: H-3's blocked-on half is
`src/ability/effects/look.c` + `tests/**` ("make `rb_execute_effect_ex`'s ownership of
`look_action`/`select_action` explicit so `free_optional_look_select` does not
double-free"). H-3 is *also* one of the items this reconciliation classifies as
OBSOLETE, because `test_parity_queue_resume` no longer aborts — but the double-free
root cause can still be fixed underneath a suite that happens to stop crashing.

Read-only check of the **current** live `look.c` against the live headers:

```
gcc -std=c11 -O2 -Wall -Iinclude -Isrc -Isrc/core/generated -fsyntax-only \
    src/ability/effects/look.c        ->  exit 0, 0 errors
```

(`-Wunused-function` is only emitted with codegen, so `-fsyntax-only` does **not**
re-measure the 14 `-Wunused-function` warnings the snapshot build reported for this
file. That is unmeasured, not zero.)

**So: the build verdict in §6 is valid for the tree as of 20:41:37. One `SRC` file has
changed since, and it compiles clean.** I did not re-run the full build after it,
because doing so would have meant snapshotting a tree that is still being written.
`BUILD_BREAKS.md` is still not warranted.

---

## 6. Build verdict

`src/abilities_strings.bin` was **726831 B / `89204df0bbde08a0…`** before the build,
after `make all`, and after `make web`. `make data` was never invoked.

```
make clean   -> exit 0
make all     -> exit 0     0 errors     113 warnings     51 objects
make web     -> exit 0     0 errors
rb_engine.exe      1361193 bytes
rb_web_server.exe  1409247 bytes
```

**The tree builds. `BUILD_BREAKS.md` is therefore not written.**

Corroborating suite runs (compiled against the freshly built `OBJ_LIB`, `^FAIL`
counted **case-sensitively, no colon**, on stdout **and** stderr):

| suite | run rc | `^FAIL` out | `^FAIL` err | stdout bytes |
|---|---|---|---|---|
| `harness_self` | 0 | 0 | 0 | 4508 |
| `draw_phase_fix` | 0 | 0 | 0 | 99 |
| `parity_queue_resume` | 1 | 2 | 3 | 7023 |
| `audit_triggers` | 1 | 1 | 1 | 951 |
| `describe_golden` | 0 | 0 | 0 | 169 |
| `choice_renderer` | 0 | 0 | 0 | 3848 |
| `parity_state_family` | 1 | 2 | 3 | 2303 |
| `parity_t6_action_setup` | 0 | 0 | 0 | 864 |
| `parity_t7_condition_decoder` | 0 | 0 | 0 | 253 |
| `parity_search_reorder` | 1 | 36 | 122 | 23490 |
| `condition_evaluation` | 1 | 2 | 3 | 1179 |
| `vm_decode` | 0 | 0 | 0 | 2975 |
| `audit_zones` | 1 | 2 | 3 | 22363 |

No suite produced zero output, so no suite was mis-read as passing.

### ⚠ `sizeof(GameState)` — the Makefile's documented value is wrong by 227,824 B

`Makefile:3-8` states:

> `sizeof(GameState)` is 885912 bytes … **MEASURED 2026-10-03** by `sizeof()` in
> `tests/test_harness_self.c` §A … §A of `test_harness_self.c` fails if it moves again.

Measured by that exact suite just now:

```
sizeof: GameState=1113736 TestGame=1113736 RbPlayer=20824 RbBag=2052
GREEN: 0 checks failed
```

- Real value **1113736**, i.e. **+227,824 B** over the documented 885,912 — the growth
  from main's appended fields (`rule_log[500][256]` = 128,000 B alone, plus
  `member_contributions`, `live_spare`, `resolution_owners`, `constant_need_heart_global`).
- §A does **not** pin an absolute size. `test_harness_self.c:98-99` defines
  `RB_MIN_SAFE_LOCALS 15` and `RB_GAMESTATE_BYTE_CEILING = 16777216/15 = 1118481`,
  and `:107` asserts only `sizeof(GameState) <= ceiling`.
- **Headroom left: 1118481 − 1113736 = 4,745 bytes (0.042 %).**
  One more array of ~4.7 KB on `GameState` turns `harness_self` §A2 red.

Not fixed. Reported.

### Other defects found and NOT fixed

- `src/core/alloc_counter.c` — confirmed still an orphan: not in `Makefile:25` `SRC`
  nor `:28` `OBJ_LIB`, fails on `ATOMIC_VAR_INIT` under every `-std`.
- **7 orphan declarations in `include/rabuka.h` with no definition anywhere.**
  Six carry a matching `DELETED` tombstone and are harmless:
  `rb_invalidate_ability` (`rabuka.h:2224` / tombstone `ability.c:478`),
  `rb_effect_modify_hearts` (`:2248` / `state.c:1764`),
  `rb_execute_effect_via_registry` (`:3217` / `executor.c:592`),
  `rb_translated_execute_custom_effect` (`:3224` / `ability_effects.c:626`),
  `rb_cost_clear_deferred` (`:2679` / `cost.c:1843`),
  `rb_activate_ability_effect` (`:2226` / `ability.c:600`).
  **The seventh has no tombstone and is the dangerous one:**
  `rb_resolution_add_for_owner` (`rabuka.h:2639`) — main's header bill landed the
  declaration and the `resolution_owners[]` field (`rabuka.h:1617-1618`) but
  **never the function body, and never a writer or a reader for the field.**
- `Makefile:3-8` `sizeof` comment — wrong, above.
- `PARITY_CHECKLIST.md:424-451` PATH CORRECTIONS table is missing an entry:
  **T3-4 cites `src/core/state.c`, which does not exist**; the file is
  `src/ability/effects/state.c`.
- `PARITY_CHECKLIST.md` line-number drift throughout: B1 `:4907` → now `:4831/:5187`;
  B3 `:1120` → `:1114`; B4 `:286` → `:333`; C4 `~1520` → `:1418`;
  I6 `:2255-2265` → `:2265`; T4-7 `:307` ✓; T4-10 `:441` → `:415`;
  T4-12 `:717`/`:736` ✓; A4 `192` → `256`.
- `tests/test_harness_self.c` output contains mojibake
  (`窶・`, `繝ｩ繧､繝夜幕蟋区凾`) — the file was written through a
  non-UTF-8 round-trip. Cosmetic, but it means that file's Japanese assertions are
  unreadable.
- `ORPHANS_AND_SCOPE.md` has the same mojibake in its headings.
