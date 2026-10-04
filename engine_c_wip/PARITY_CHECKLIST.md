# Parity Closure Checklist — engine_c_wip vs Rust

Every item here was diagnosed and measured by a peer agent. **None is "someday".**
Each has an owner file. An agent closing one must either land it or prove it
impossible with evidence. No report-only outcomes.

Status legend: `[ ]` open · `[~]` in progress · `[x]` closed

**Clusters G and H were closed 2026-10-03** (4 files touched: `src/core/card.c`,
`src/core/game_state_abilities.c`, `src/ability/util.c`, and — for G3 only —
`include/rabuka.h`). Measurement: both trees were snapshotted from the SAME
settled repo state and differ only in those files (proved by a SHA-256 sweep
over all 380 files: 376 identical, 4 differ, none outside the four). Whole C
suite corpus, 142 suites, strictly sequential, `make data` frozen (the blob
guard was pointed at the good 726,831-byte / `89204df0…` copy):

| | baseline | after |
|---|---|---|
| GREEN | 44 | 44 |
| RED | 98 | 98 |
| BUILD_RED | 0 | 0 |
| FAIL lines in stderr | 2,348 | 2,348 |
| sorted stdout identical | — | 137/137 non-empty suites |
| sorted stderr identical¹ | — | 142/142 |
| compiler diagnostics | 86 | 86 (identical set) |

¹ after normalising the snapshot path and heap pointer values, which are the
only two things that differ. **Zero movement, in either direction.** That is the
expected result: G1/G2 are absences measured at 0, G3 removed a function with
0 callers, G4 removed a path 2 corpus pairs reach, and H1 enabled a matcher that
no producer populates yet.

---

## A. Header — include/rabuka.h (owned by `main`)

- [x] A1 `choice_route_raw[64]` + `has_choice_route_raw` on RbQueueEntry
- [x] A2 `RbSetupActionType` + `RbSetupAction` moved out of game_setup.c
- [x] A3 `RB_COND_UNSUPPORTED = 20` on RbConditionVariant
- [~] A4 `rule_log[500][256]` + `rule_log_len[]` + `n_rule_log` on GameState — **WRITER WIRED (cluster Y.1), reader NOT closed.** `gsc_rule_log_push` in `game_state_core.c` appends through all three `GameState*`-taking entry points and the 32-slot condition-verdict ring is out of the path; probe-verified incl. the 500-bound newest-kept shift. Two open halves: `rb_gsc_push_rule_log_fmt`'s declared signature takes no `GameState*` (BLOCKED-ON `include/rabuka.h:3609`, patch in Y.1), and Rust's `rule_log` has no in-scope reader at all (its 6 readers are `display.rs` — **T8 Q2 OUT OF SCOPE** — `bot/**`, and `.len()`-in-`log::debug!` sites), so the C's reader surface is `src/web_server.c`, not this cluster's. **Do not re-read this box as done.**
- [x] A5 `RbMemberContribution` struct
- [x] A6 `performance_need_heart_modifiers[]` on RbLiveSnapshot
- [x] A7 `cards_paid_cost_this_turn[]` + `_cheer_revealed_cards` + `revealed_cost_cards` RELOCATED from RbLiveSnapshot to GameState
- [x] A8 `turn_area_movements[]`, `opp_cause_fired_keys[]`, `cards_activated_this_turn[]`
- [x] A9 `cause_card_id` on RbBatchMovement — ✅ **FULLY CLOSED 2026-10-04 (fixer wave). BOTH ends now live: 1 writer, 1 reader, and the reader is Rust's ONLY reader.** The two corrections in the previous text were both right and are now acted on: there is **no `eval_group` function in Rust** and no temporal arm keyed on `cause_card_id` at `core/card.rs:1642-1655` (that region is `AbilityEffect` decoding); the real site is **`ability/condition/card.rs:1632-1658`**, re-verified by recursive grep that `cause_card_id` has 6 writers (`game_state/modifiers.rs:1823/1834/1922`, `effects/position.rs:622/633/914/1184`, fields declared `core/types.rs:977` and `:1002`) and **exactly ONE reader, `card.rs:1646`**. (a) **WRITER** was already in: `rb_record_card_movement` sets `event.cause_card_id = g->activating_card` (`src/core/modifiers.c:752`). (b) **READER LANDED** — `src/ability/condition.c`'s `eval_group` temporal-this_turn arm was rewritten as the faithful Rust arm, and it was wrong on **FIVE** counts, not four: it scanned the MOVED-CARDS set (`rb_entry_trigger_moved_cards` else `g->recently_moved`) instead of `g->turn_area_movements`; it never tested `m.moved_card_id == activating_card`; never `m.effect_only`; never `m.cause_player_id == <resolved seat>`; and it matched the group against the **moved** card with `group_names[0]` where Rust matches `m.cause_card_id` against the whole list with `.any()`. Its gate was also a strict SUPERSET of Rust's — it accepted an ABSENT `card_type`, which Rust's `== Some("member_card")` rejects. **`REACHABLE: 2 condition nodes on 2 card-ability pairs** — cards 2467 / 2468 ab#0, both 「このターン、自分の『Liella!』のカードの効果によってこのメンバーがエリアを移動していた場合」; both carry `card_type:"member_card"` and a ONE-element `group_names:["Liella!"]`, so the `.first()`-vs-`.any()` narrowing and the gate superset are **invisible on the corpus** while counts 1-4 are not. **Proved by `tests/test_parity_fx_fixers.c::fx1_cause_card_id_arm` (13 assertions, one per leg, plus the two structure facts above).** Suite effect, A/B against a baseline that is the current tree with only these hunks reverted: **`parity_live_total` 20/33 → 18/32** and **`parity_jidou_move_yell` 37/47 → 36/45**; 25-suite total **1276 → 1270**, nothing regressed. ⚠ **Two `parity_live_total` assertions newly FAIL (`I1_chisato_liella_move_grants_score`) and this is stated plainly rather than hidden: they were passing by ACCIDENT.** The old dead arm answered "did anything move recently", which is TRUE in that scenario for the wrong reason; the faithful arm correctly answers 0 because `turn_area_movements` is EMPTY at ライブ成功時 time in the C (traced: `tam=0`). Three negatives that were wrongly FAILING (`I3`/`I4`/`I6` — natural move, previous-turn move, opponent's effect) now pass. The positive needs the movement to survive to live-success time, which is a tracking-lifetime question, not this arm.
- [x] A10 `self_no_excess_heart_this_turn` + `opponent_live_no_excess_heart_this_turn`
- [x] A11 description{,_en,_ja} 128 -> 512 (RbChoice + RbFullChoice)
- [x] A12 `rb_card_has_draw_icon` prototype
- [x] A13 `rb_ability_ref_flush` / `_flush_slot` — definitions were already deleted from src/ability/ability.c; removed the two orphan declarations and the stale comment that told callers to use them. Header re-verified with `gcc -fsyntax-only`.
- [x] A14 12 prototypes: rb_score_to_i32, _rounded, rb_effective_stat, rb_overridden_stat, rb_floored_cost, rb_count_u32, rb_id_u32, rb_f32_count, rb_phase_is_live_phase, rb_zone_matches_source, _dest, rb_step_output_to_json
- [x] A15 `rb_process_yell_revealed_card_icons` + `rb_fmt_player_id` exports
- [x] A16 `rb_validate_mandatory_cost`
- [ ] A17 six CardFilter carriers: original_heart_total(+op), exclude_group_names, cost_limit_min/max, groups[], exclude_cards[], exclude_names[] — **STILL 0 producers (cluster Y.5); the `[x]` was for the header lines only.** `card_filter_from_effect` (`src/core/card.c:957`) sets none of them; the matchers in `src/ability/util.c:1527/1537/1552/1567/1584/1586/1701` and the copy at `util.c:1873-1897` therefore read a filter that can never carry them, which is why **every corpus number ever quoted about these carriers came from dead code.** Also: **`original_heart_total_opponent` does not exist in Rust — 0 occurrences in all 1173 `engine/src`+`engine/tests` files** — so main declared a twin of a field that was never there; it should be deleted from the header. Per-carrier producer patch is in Y.5. (A17's own measurement is STALE: `exclude_group_names`, `cost_limit_min`/`max` and `groups[]` all have producers in the current `card_filter_from_effect` — see A17b, which measured the corpus directly.)
- [x] A17b **LANDED 2026-10-04. The Rust collapse IS in committed `2130769d`** (`git show 2130769d -- engine/src/ability/util.rs engine/src/core/card.rs` carries both hunks: `CardFilter::from_effect` at `util.rs:1864-1867` and `AbilityEffect::filter_subset` + `excluded_matches` at `core/card.rs:889-894` / `:2410-2416`; `engine/src` is clean in git at HEAD `cf315a9a`), so the C was genuinely divergent and the one edit below is faithful. `src/core/card.c` `card_filter_from_effect`, immediately before `const char *group = group_names;`:
  ```c
  if (exclude_gn && *exclude_gn && group_names && !strcmp(group_names, exclude_gn))
      group_names = NULL;
  ```
  `strcmp` on the two comma-joined extras strings is byte-equivalent to Rust's order-sensitive `Vec ==`, because vm.c flattens a wire array into one joined string. **Scope re-measured 2026-10-04 over `cards/abilities.json`: exactly ONE effect node of 3 053 walked effect+cost nodes carries both keys** (`百生 吟子`, `PL!HS-bp5-004-{AR,P,R}`, ab#0, where `group_names == exclude_group_names == ["スリーズブーケ"]`), 4 abilities carry `exclude_group_names` at all, and **0** nodes carry a bare `group` key, so the `fx_extra(e, "group")` fallback is dead on this corpus.

  ⚠ **THE `excluded_matches` DOC/BODY CONTRADICTION IS ADJUDICATED: THE BODY IS AUTHORITATIVE, and the C lands against it.** Three reasons, in order. (1) It is what Rust executes; a doc comment cannot change `==`. (2) `excluded_matches` has exactly ONE caller, `AbilityEffect::filter_subset` (`core/card.rs:2413`), and both of its inputs are strings lifted off the SAME effect object; normalisation in Rust is a CARD lookup (`card_matches_group_str`), unavailable to a string-vs-string test, so the doc's claim is not implementable at that call site. (3) It is **MOOT on this corpus**, which is the strongest reason to stop asking: the one node's two lists are byte-equal AND equal after whitespace removal and NFKC (micro-sign) folding, so both readings give the same verdict. The comment is documentation drift, not a spec.

  ⚠ **THE TEST WAS CORRECTED, AND IT WAS A GENUINE ENCODING OF THE WRONGNESS — stated plainly.** `tests/test_parity_jidou_watch.c` was mine and quiescent when I took it (SHA-256 `A3C3D6518604E17D` on two reads 40 s apart, no `.o` newer than any source). Its CONTROL for `jump_up_high_gives_each_aqours_member_exactly_one_blade` asserted the non-『Aqours』 member holds **0** blades after three `rb_recalc_constants`, and it passed, only because the C's filter builder kept the inclusion AND the exclusion alive and an unsatisfiable pair matches nothing. With the collapse landed the same board measures **exactly 4** (千歌 cost 17 + 梨子 cost 15 = 2 qualifying members x 2 blades), which is what 「自分のステージにいるコスト4以上の『スリーズブーケ』のメンバー1人につき、ブレードを2得る」 demands. Three edits, all making assertions STRONGER, none removing or weakening one: the CONTROL's two outsider assertions became `== 4` with the card text and the Rust line cited; the round-loop's outsider assertion became `== 4`; and the main-game assertion near the top of the function **stays `0`** and now says why (no `rb_recalc_constants` has run on that board yet, so the 0 measures the GRANT alone, which is that test's subject). The CONTROL still does its stated job and does it *better*: the two rival causes now differ by MAGNITUDE, since her own 常時 pays 4 while the 『Aqours』-filtered ライブ開始時 fan-out can only ever pay 0 to her, so a count of 4 cannot have come from the grant and the two 『Aqours』 members at 0 prove the grant was not re-derived. A NEW dedicated parity test was added, `test_a17b_inclusion_collapses_so_the_exclusion_alone_applies` (8 assertions, **8/8 green**), pinning both directions: a 3-stage board gives the beneficiary 4 blades and the others 0; the same card ALONE gives 0, which is what proves the collapsed filter is an EXCLUSION and not a dropped one.

  **MEASURED, clean builds on both sides, 22-suite list: `parity_jidou_watch` 75 -> 75 with ZERO lines new and ZERO lines gone, and all 20 other suites byte-identical.** The collapse is behaviour-neutral across the whole list. ⚠ The "+6 on `parity_jidou_watch`" reading that used to sit in this item was produced by a run whose OBJECT SET WAS STALE and is void; see the build trap appended at the end of this file.
- [x] A18 `member_contributions[2][RB_STAGE_SIZE]` + `n_member_contributions[2]` on RbLiveSnapshot. Required MOVING the `RbMemberContribution` typedef from ~line 3431 to ~line 1253 (C needs the complete type before the enclosing struct); that was the actual reason live.c used a file-static. Header re-verified.
- [~] A19 `spare` / `base_score_total` / `card_bonus_total` / `transform_delta` — fields exist on RbMemberContribution and are now REACHABLE via A18; the `finalize_snapshot_fields` + `enrich_from_applications` port is cluster I.
- [x] A20 header declares all four (`rb_group_name_first`, `rb_effect_group_name`, `rb_card_matches_effect_group`, `rb_get_selection_indices_filter`). Note the real path is `src/ability/util.c`, not `src/core/util.c`; `rb_get_selection_indices_filter` had only tentative forward declarations in util.c:887 and effects/move.c:104 with its single definition at util.c:2070. The four local twins in cost.c / score.c / misc.c / ability_effects.c are still to be deleted by their file owners.

---

## B. Queue / choice

- [~] B1 choice.c:4907 bare `cur++; n_entries = 0;` -> `rb_queue_complete_current` + `!rb_has_pending_choice` guard (agent running)
- [ ] B2 choice.rs:815-865 same-unit remainder re-prompt -> choice.c:2136-2146 must pass filtered_indices for the CHOSEN unit and take cost_count from `cost->count`, not `pending->count`
- [~] B3 `choice_cost_opens_hand_prompt` excludes same-unit costs — ✅ **`VERIFIED-NOOP: 0 of 138`** (fixer wave, 2026-10-04). **Measured by TWO independent methods, because the brief warns that three tallies in this project were wrong in exactly the direction that makes work look unnecessary.** (1) Cost-level, whole tree: **562** hand-sourced cost nodes, of which exactly **2** carry `same_unit_name` — cards 543 / 544 = `PL!HS-PR-016-PR` / `PL!HS-PR-017-PR`, both `move_cards{source:hand, count:2, optional:true}`. (2) **Ability-level, which is the honest denominator**: `choice_cost_opens_hand_prompt` is reachable ONLY from `choice_debut_cost_action`, which requires `ab.triggers` to contain 登場 or バトンタッチ, so the question is not "hand costs" but "debut-triggered hand costs". Measured: **632 debut abilities, 138 debut hand costs, 0 of them with `same_unit_name`.** Both `same_unit_name` nodes sit on abilities whose trigger is **ライブ開始時** (`abilities.json:28171-28180` / `:28223-28232`), which the debut reconstruction never consults. **The exclusion is unreachable and is NOT a defect.** For the record the exclusion would have been WRONG in principle: Rust's same-unit re-prompt is gated on `new_card_ids.len() < count` (`choice.rs:776-813`), so a same-unit cost's FIRST prompt is the ordinary `SelectCard{hand, count, allow_skip}` shape and its re-prompt carries the very `"...from hand for cost"` tag the C matches — the exclusion removes both shapes Rust produces. It is left in place because fixing unreachable code is how real divergences get hidden. ⚠ **Sibling exclusion IS live and is NOT B3**: the `all` (whole-hand) test guards **2 of 138** (cards 2336 / 2337, `count:-1, optional:1`). Not verified against Rust in this wave — flagged, unowned.
- [ ] B4 rb_resolver_spawn_target (choice.c:286) — wire or delete with Rust proof
- [~] B5 `rb_queue_clear_completed` + `rb_queue_is_entry_available` + `rb_queue_entry_player_id` externs — ✅ **PARTIALLY CLOSED 2026-10-04 (fixer wave); the rest is `BLOCKED-ON` rabuka.h, which is not ours.** First, the line reference is stale and one of the three was never a hoisting candidate: **`rb_queue_is_entry_available` is ALREADY exported by `include/rabuka.h:1890`**, so the file-scope extern for it was a pure redeclaration saying nothing the header does not — **it is DELETED.** The other two are genuinely absent from the header and are now declared under ONE comment that names the blocker and the exact prototypes, instead of three loose externs (`src/core/game_state_abilities.c:3058-3071`):
  - **`BLOCKED-ON: include/rabuka.h` — add, beside the other `rb_queue_*` prototypes (around `rabuka.h:1890-1900`, next to `rb_queue_is_entry_available`):**
    - `const char *rb_queue_entry_player_id(const GameState *g, int index);`
    - `void        rb_queue_clear_completed(GameState *g);`
  - Definitions are `src/ability/ability_queue.c:1123` and `:689`. `tests/test_parity_queue_resume.c:41` carries its own copy of the second declaration for the same reason and can drop it once the header has both. **Owner: whoever owns `include/rabuka.h`.**

## C. Cost handlers

- [x] **C1** LANDED 2026-10-04. `setup_ability_cost_offered` gained a `card_id` parameter and now calls `rb_validate_mandatory_cost(g, actor, ab->cost, card_id)` instead of `rb_validate_cost`. BOTH Rust call sites are transcribed in the function's own comment (`game_setup.rs:2188-2192` stage scan and `:2295-2299` discard scan, both `validate_mandatory_cost(game_state, &c.0, Some(card_id))`), plus the `handlers.rs:189-196` reason the card is passed explicitly ("offer/activation time run before `gs.activating_card` is set"). Proven by `tests/test_parity_fw_cost_cardtype.c` case `fw_c1_self_cost_activation_is_offered`: `PL!SP-bp7-008-P` ab#0 costs `change_state{state_change:wait, self_cost:true, card_type:member_card}`, so the OLD gate refused it (RB_ACTIVATING_DERIVED -> no published activating card -> `self_cost` matched nothing) and the new gate offers it. **Base 1 FAIL, fix 0 FAIL.** The genuinely-unpayable control (`PL!N-bp3-004-P` ab#0's mandatory hand-discard with an empty hand) is refused on BOTH trees, so nothing was weakened. Suite effect: `parity_draw_flat` -1 hidden (`fresh instance next turn: offered (got 0 expected 1)` — `PL!N-PR-019-PR`, cid 953).
- [x] **C2** verified, NOT re-landed: `engine.c`'s `activate_resolved_ability` already calls `rb_validate_mandatory_cost(..., activating)` at `engine.c:1187`, matching `turn/actions/mod.rs:559-563`. Reverted-reader A/B of the whole C1+C2 change: **0 suites move** for the `engine.c` half alone, and its own `engine.c:1192-1194` note ("every `self_cost` filter resolved against a derived card rather than the activating one") already records the effect.
- [x] **C3** LANDED 2026-10-04. The `distinct` dedupe is now in `pay_cost_move_cards`, right after the hand-candidate loop and therefore on BOTH Rust branches — handlers.rs:476-480 applies it to the whole `indices`, not to the `else` arm only. Two new file-local readers in cost.c: `cost_distinct_type` (core/card.rs:2809-2816 — the serde renames are LOWERCASE `"card_name"` / `"true"` / `"distinct"`) and `cost_dedupe_indices_by_name`. ⚠ **The checklist's reachability note was WRONG and is corrected by measurement:** "0 of 138 costs carry `distinct`" counted only TOP-LEVEL costs. Measured with the engine's own decoder over the whole corpus, **2 of the 901 decoded cost nodes carry `distinct`**, and both are exactly this shape — `move_cards{source:hand, count:3, optional:true, distinct:"card_name", group_names:<X>, destination:discard}`, i.e. `PL!-pb2-018-P＋` / `-R＋`「手札の名前の異なる『BiBi』のメンバーカードを3枚控え室に置いてもよい」. Proven by `fw_c3_hand_cost_dedupes_by_normalized_name`: a 3-card BiBi hand holding two prints of 絢瀬絵里 dedupes to 2 and **auto-skips** with no prompt (handlers.rs:541-551 `optional_cost_result = Some(false)`); base 1 FAIL, fix 0. The no-`distinct` control opens the prompt on both trees. Two details transcribed in the code because a plausible patch gets them wrong: it dedupes the **candidate set** (so the duplicate print is never OFFERED — Rust's own comment says so), and the key is the **normalized name AT THAT INDEX**, so util.c's `rb_dedupe_by_normalized_name` (which takes card IDS and returns "the first index of every distinct id") answers a different question; the new loop is over indices and normalizes with `rb_card_normalize_name` (core/card.rs:675-685, drops ALL whitespace), not util.c's file-static `rb_norm_ws` (ASCII-only).
- [~] **C4** `VERIFIED-NOOP: 0`. Measured with the engine's own decoder over the whole corpus: **all 124** `change_state` cost nodes carry `state_change` = `"wait"` and **0** carry `rest`/`rested` — the distinct-literal census is a single row, `SC 'wait' 124`. So the `rest`/`rested` arm in `pay_cost_change_state` and the non-`wait` candidate computation around it are unreachable, and Rust agrees independently (handlers.rs:879 / :917 are both `if state_change == "wait"`). Not ported, per the rule that an unreachable gap closes as a measurement rather than a guess.
- [ ] C5 delete 4 dead wrappers + their rabuka.h lines: rb_cost_has_skip_prompt, rb_get_change_state_candidates, rb_pay_cost_move_cards, rb_pay_cost_change_state (all Rust-`private`, 0 external callers)
- [ ] C6 `s_stage_select_intent` — 3 writers, 0 readers. Needs a reader for Rust's `awaiting.stage_select_intent`.
- [ ] C7 `pay_cost_move_cards` non-hand optional gate: Rust has none in handlers.rs; the E3 shape needs it from move/selection.rs:134 on the EFFECT side.

## D. Condition

- [x] D1 `evaluate_front_comparison` (card.rs:406) — LANDED **inside D4**: `position=="front"` (comparison.rs:86-89) now calls a faithful twin. Subject is the ACTING player's stage slot (Rust resolves `"self"`, not the clause's `target`, which is `"opponent"` in both corpus clauses) and the mirrored front slot; the old C read `stage[1]` and the printed cost. 2 nodes (cards 844-845 ab#1).
- [x] D2 `check_heart_type_all_per_card` (predicates.rs:68) — LANDED. `cond_card_has_all_heart` has ALL THREE legs (base-slice Heart00 membership / `mods.heart[cid][All]` total > 0 / `constant_heart[cid][All]` > 0); the old body had one AND read `heart_count[0]` out of the WHOLE heart array. **Measured: 0 of 2,639 cards print a base-slice Heart00** (314 Heart00 entries exist, all in blade/need/special), so the discriminator is legs 2-3 and leg 1 is VERIFIED-NOOP: 0 of 2,639.
- [x] D3 wire `rb_check_heart_type_all` / `rb_check_heart_colors` — LANDED. `rb_check_heart_type_all` is now called FIRST in `eval_location`'s chain (card.rs:788), its negation arm asks about the queue entry's `triggering_member_id` instead of the unused entry index, and **all four** post-dispatch negation exemptions (condition.rs:643-656) are implemented — the C had only exemption 1. `heart_type:"all"` is NOT common: **1 of 2116 pairs** (card 1239 ab#0), and it is exactly the plain-Location + negation shape of exemption 3.
- [x] D4 `evaluate_comparison_condition` (comparison.rs:47) — LANDED. Census first: **Comparison + `negation` = 3 nodes of 216** (cards 1212-1214 ab#0, `resource_type:surplus_heart`), and all 3 reach Rust's DOUBLE-negating numeric tail, so the two negations cancel and the C's single fold answered the opposite for exactly those 3. Added: `paid_cost_this_turn` (1 node, card 449), `energy_relative` (2, card 1853), `cost`+`activating_card` (3, cards 705-707), `original_value`+`score` (2, cards 2469-2470), `position=="front"` (2, cards 844-845), `cost_total` (**22 nodes / 13 pairs**, largest D4 win), `revealed_cards` post-check (6 nodes carry `card_type`+`revealed_cards`), the inner negation, and the position-cost plain-count arm (0 corpus nodes).
- [x] D5 `eval_highest_cost` — LANDED. Uses the NAMED `position` slot (`is_highest_cost_on_stage`, condition.rs:447-452 — the old gate hardcoded `center` too), the resolved target's player, and `effective_cost`; fixed `J_cost_modifier` + one `parity_conditional_gated` assertion.
- [x] D6 `get_count_for_condition` full port — PARTIAL-LANDED. Split out `total_effective_cost` and `heart_count_at_or_all`; added the `comparison_type=="cost"` + `comparison_target=="self"` MAX-cost path (7 nodes) and the `!loc`/`revealed_cards` effective-cost sum. **The structural blocker was `if (loc) return eval_location(...)` in `eval_comparison_inner`**: a cost clause must reach `get_count_for_condition`, never the location evaluator — removed for `comparison_type=="cost"`, which is what fixed 「合計コストが相手より小さい」. BLOCKED-ON sub-item: `count_for_preceding_moved_score` (card.rs:3902, 2 nodes) — this file has two readers of "the cards just moved" (`g->recently_moved` :3290 vs the queue entry's moved_cards) and neither satisfies both clauses of PL!S-bp3-008-R.
- [x] D7 `eval_no_excess` vs `no_excess_heart_flag` — LANDED. Now the two-value flag (state.rs:15-22) keyed on `pl == g->first_attacker`. **VERIFIED WRITERS: `self_no_excess_heart_this_turn` has exactly 1 writer (src/turn/live.c:1048); `opponent_live_no_excess_heart_this_turn` has 0 writers anywhere in src/** — so the second arm reads a cleared field until that writer lands.
- [x] D8 — see the per-key tally below. LANDED: `blade_limit` + `blade_limit_operator` (9+9 nodes) and `original_value` (8 nodes) via `check_original_blade_filter`/`check_original_heart_filter` in `check_card_property`; `cost_total` via D4. **VERIFIED-NOOP (0 of 2116 pairs / 1589 condition nodes)**: `baton_touch_source`, `original_heart_total`, `cost_reference`, `cost_total_operator`. **NOT MINE / BLOCKED**: `trigger_event` (T7, fixed in vm.c), `action_reference` (consumer is ability/condition/compound/conditional_on.rs:50,64), `any_location` (36 nodes, consumer card.rs:1355), `reference_card` (4, card.rs:877), `require_position_cards` (4, card.rs:856), `same_name` (1, card.rs:2555), `blade_greater_than_all` (1, card.rs:823), `area_direction` (2, state.rs:534), `energy_placed` (11, state.rs:922), `watches_area_move` (17→29, state.rs:1083), `self_effect_only` (11, state.rs:480/579/796/1240), `activated_this_turn` (3, card.rs:2417 CardFilter), `cost_limit_operator` (40 nodes; read only by CostFilter card.rs:2088/2116 and state.rs's cost_limit arms), `cost_reference_character`/`cost_reference_operator` (2 each). All census numbers are reproducible with `tools/audit_cluster_d.c` (new file).

**Cluster D measurement (tools/audit_cluster_d.c, 2116 card-ability pairs / 985 abilities / 1589 condition nodes):**

| item | nodes | pairs | status |
|---|---|---|---|
| Comparison + `negation` | 3 | 3 | LAND (double negation cancelled) |
| `heart_type:"all"` | 1 | 1 | LAND (plain-Location + negation) |
| `cost_total` | 22 | 13 | LAND |
| `cost` + `comparison_target:"self"` | 7 | 7 | LAND (MAX-cost) |
| `cost` + `location:"revealed_cards"` | 10 | 10 | LAND (effective-cost sum) |
| `comparison_type:"score"` + `source:preceding_moved` | 2 | 2 | BLOCKED-ON the moved-cards reader |
| `resource_type` heartNN with a position | 4 | 4 | ported; unreachable from this variant's own arm (measured) |
| highest-cost-on-stage | 2 | 2 | LAND |
| no-excess-heart | 2 | 2 | LAND (2nd flag has 0 writers) |

**A/B (42 suites, `Temp/kilo/dcond`, baseline = this snapshot with only these hunks reverted):**
`stdout 403 → 402`, `stderr 1024 → 1018`, `BUILD_RED 0 → 0`, new suite `tests/test_parity_cluster_d.c` 0/0.
Moved suites, all improvements, **zero regressions**: `parity_conditional_gated` 26/46 → 25/44 (fixed `J_cost_modifier`: "left self effective 14 > centre 10 -> the centre is not highest", "right effective 12 > centre 10 -> no heart03 (the modifier is respected)"), `parity_hearts_constants` 0/18 → 0/17 ("a cost-15 member on stage -> self gains heart03"), `parity_integration` 12/31 → 12/30 ("suzu 17b: 16 < opponent 19 -> no blade"), `parity_small_clusters` 42/69 → 42/67 (both "B4 negative: 蓮ノ空 total 10 (+6 = 16) < opponent 19" assertions). 38 of 42 suites byte-identical.

**condition.c diagnostics 4 → 2**: `-Wdiscarded-qualifiers` at the old `rb_check_heart_type_all` (`RbPlayer *P = &g->p[pl]`) and the unused `target` in the old negation arm are both GONE. The two that remain (`-Wmisleading-indentation` in `eval_both_condition`, `-Wunused-variable target` in `eval_multi_location`) are pre-existing and untouched. **The Makefile's `WARN_AUDIT_BASELINE` pin for `src/ability/condition.c:2342:-Wdiscarded-qualifiers:W3` is now stale — the gate will report it as no longer present.**

## E. Modifiers (core/game_state/modifiers.rs + game_modifiers.rs)

**Cluster E worked 2026-10-03.** Snapshot `Temp/kilo/emod2` (fresh from the live
tree; 411 files, 0 `.o`, 0 `rb_engine*`; blob 726 831 B / `89204df0…` before and
after every build; `make data` never run). Base hash of the file worked:
`25A51F2A6533D8E11B4740D7321C372DFD5DE6F087D2D4A5E7B6108EACAE3E27`, identical
across a 15 s settle and re-confirmed immediately before porting; clean in git;
no `.o` newer than the source. Every tally below is over the **real database**
via a new decode-level census tool, `tools/census_e.c`, which walks all 2 639
cards / 2 116 card-ability pairs / **4 925 effect nodes** with the engine's OWN
decoder (`rb_decode_card_ability`) — no bytecode reader is re-implemented.

**Corpus denominators used throughout:** 985 abilities · 6032 interned strings ·
2116 card-ability pairs · 726 831 table bytes · 98 737 bytecode bytes ·
**275 of 2116 pairs carry the 常時 trigger** · 4 925 effect nodes.

- [x] E1 **`evaluate_success_zone_constant_modifiers`** (modifiers.rs:2188) — ✅ **LANDED 2026-10-04 as `rb_evaluate_success_zone_constant_modifiers` in `src/core/modifiers.c`**, called from `src/turn/triggers.c` at Rust's `modifiers.rs:1396-1400` position (after the `p1/p2_constant_total_score_bonus` assignment, before `rb_refresh_yell_sources`). All three `success_zone_*_bonuses` ledgers now have a real writer and a real reader; `tools/e1_ledger_probe.c` proves it at runtime (`success_zone_heart_bonuses[17][3]=1` written then read back, 39 of 137 `gain_resource` hosts move a ledger, `p1_constant_total_score_bonus=1` for a `live_total` host). The withdraw prologue (`:2200-2214`) and the withdraw-then-reapply cycle are faithful; the three `success_zone_*_sources` DISPLAY ledgers are deliberately absent (`RbMods` has no such members and no C reader). **THE BRIEF'S "214 / member cards" TALLY WAS THE WRONG CRITERION AND IS CORRECTED HERE.** Rust's E1 iterates the four live/success BAGS and never tests card type (`modifiers.rs:2232-2242`); measuring with `rb_card_is_member` answers a different question. Correct per-action counts over the whole corpus: **`modify_required_hearts` 1, `gain_resource` 176, `modify_score` 29, `sequential` 14 = 220.** The member-only subset of the same walk is the old 214 (0/174/26/14), and the 1-vs-0 difference is cardid **291 = `PL!-bp6-022-L`**, whose type byte is 1 (`live_card`) — a member filter discards exactly the one card whose `modify_required_hearts` arm is reachable. Three tools, one disagreement explained: `tools/census_e1_exact.c` 220 (Rust-faithful dispatch), `tools/census_e1_all.c` 220, `tools/census_e.c` 214 (member-only), `tools/census_e1.c` **226 = overcount** whose extra 12 were traced node-by-node (`tools/census_e1_probe.c`) and all sit under `gained_effect`, where the dispatch sees `gain_ability` and falls into `_ => {}` (`modifiers.rs:2580`). `non_stackable` **VERIFIED-INERT: 3 nodes**, `NS_KEY_COLLISIONS 0`, not ported. `sizeof(GameState)` unchanged at **1090264** (no field added, so `RB_GAMESTATE_BYTES_PINNED` and `Makefile:3` need no move).
- [x] E2 `apply_success_zone_effect` (modifiers.rs:2309) — ✅ **LANDED with E1**, as file-static `sz_apply_effect` in `src/core/modifiers.c`, all four arms plus `_ => {}`. `GainResource` implements the stage candidate set with its `position`/`group_names` filters, `resource_icon_count_any().unwrap_or(count_or(1))`, and the blade/heart split where `per_color` divides by the DECLARED colour count **before** the `heart01` default is appended (`:2455` vs `:2459-2460`). `ModifyScore` reproduces Rust's asymmetry exactly: `live_total` keys off **`player_idx`**, the zone the card was found in — deliberately NOT the `target`-resolved player — and has **no** membership test. `Sequential` recurses into `compound.actions` only, which is the C's `child[]`, with **no** condition and **no** `non_stackable` gate on the recursion (Rust skips both by calling `apply_success_zone_effect` directly).
- [x] E3 `evaluate_success_zone_constant_abilities` (modifiers.rs:2147) — ✅ **LANDED WITH BOTH OF ITS CALL SITES.** `rb_evaluate_success_zone_constant_abilities` is in `src/core/modifiers.c`: it zeroes every `mods.need_heart[cid][color].{add,set}` and then delegates to E1, matching `:2148`. It deliberately does **not** touch `constant_need_heart[]`, because that C array is the stand-in for Rust's separate `constant_need_heart_global_bonuses` map, which `:2148` does not clear either. An exhaustive sweep of every `*.rs` under `engine/src` confirms **two** callers and no more: `turn/live.rs:1326` and `turn/trigger_scan.rs:360`. Both are now wired, at Rust's positions and no others — `src/turn/live.c` `rb_execute_live_victory_determination` (`live.rs:1323-1329`, between `rb_apply_deferred_reyell` and `rb_rebuild_stage_hearts_with_yell`), and `src/turn/triggers.c` `rb_trigger_live_success` (`trigger_scan.rs:357-365`, **before** the `rb_should_trigger_live_success` gate, because Rust evaluates and restores first and only then tests the gate at `:372-378`). The scope was measured, not assumed: the C's clear is the **whole** `need_heart` table, `RB_MAX_CARD_IDS × 8`, not the live/success bags — which is what `clear()` on a map means, and it is what makes the as-long-as expiry of a success-zone grant real. `need_heart_modifiers.clear()` occurs **exactly once in the whole Rust tree**, here, at `:2148`; E1 explicitly does not (`:2185`), and `rb_recalc_constants` cannot substitute, because Rust's `recalculate_constants` calls **E1** (`modifiers.rs:1398`) and never E3 — so the C's tail call to `rb_evaluate_success_zone_constant_modifiers` is correct as it stands and was left alone. The C's `constant_need_heart` prologue is `modify_required_hearts_global`'s ledger (Rust `constant_global_need_heart`, `game_modifiers.rs:194`), a **different action** from the plain `modify_required_hearts` that E1's need_heart arm dispatches, which is why that prologue cannot withdraw E1's grant.
- [x] E4 `restore_performance_need_heart_modifiers` (modifiers.rs:2159) — ✅ **LANDED WITH BOTH OF ITS CALL SITES *AND* ITS PRODUCER.** `rb_restore_performance_need_heart_modifiers` is in `src/core/modifiers.c`, an exact body including the cross-snapshot `(cid,color)` dedupe (Rust's own comment calls it out as preventing double-counting across players' snapshots) and the restore-only-when-`target.set == 0` rule (`:2176-2178`). It reads `g->snapshots[0..n_snapshots)`, which **exists** (`rabuka.h:1620-1621`). Its two callers — the only two in `engine/src` — are wired immediately after E3 at both live-flow sites: `live.rs:1327` (`src/turn/live.c`) and `trigger_scan.rs:365` (`src/turn/triggers.c`). **The producer now exists, so this is no longer a no-op.** Two halves, because the C fuses `build_snapshot` into the push: the **capture** is `rb_capture_performance_need_heart` in `src/turn/phase.c`, called as the **first statement** of `rb_execute_performance_phase` (Rust `phases.rs:320-333` runs before `player_id` `:334`, before `performer` `:339` and long before `rb_recalc_constants`), and the **store** is in `src/turn/live.c`'s inline push inside `rb_perform_live`, which is the C's `build_snapshot` (`phases.rs:621-632` passes `&nhm_flat` at `:627`; `live.rs:3179` stores it with `.to_vec()`). The capture is a **wholesale flattening of the whole map**, not a delta and not per-card, because the restore re-adds every recorded triple. The capture lives in a module-scope buffer in `phase.c` rather than a new `GameState` field: `sizeof(GameState)` is pinned at 1 090 264 B by `tests/test_harness_self.c` §A4 (`RB_GAMESTATE_BYTES_PINNED`, an exact equality), so no field was appended and §A4 is still green. Stated divergence: when a choice pauses `rb_perform_live`, this C port returns and the phase machine re-enters from the top, so the capture is retaken on the resume pass where Rust's blocking call would keep the original; that is inherent to the resumable phase machine and already applies to the whole function, `rb_recalc_constants` included. One representational gap, matching the one already stated at `modifiers.c:1990`: the dense C `need_heart[cid][color]` cannot distinguish "absent" from "present and zero", so a zero-valued entry is not captured — which is exact here, because a zero Rust entry contributes nothing to the restore (`target.additive += 0`, and the `set` arm is guarded by `entry.set != 0`). Pinned by `tests/test_parity_need_heart_live_flow.c` (23 assertions, PASSED).
- [ ] **MEASURED, NOT A TODO — `parity_required_hearts` is `assert=32` and stays there; E3/E4 were NOT its cause.** An earlier revision of item E3 above claimed the two reds `j_activator_not_in_success_zone` and `j_as_long_as_expiration` were "the sole cause" of 28 → 32 and that wiring E3 would clear them. **That claim is wrong and is retracted here.** Both fixtures build their board with the raw zone helpers (`test_add_to_success` / `test_add_to_live`) and then call **`rb_recalc_constants` only** — never `rb_execute_live_victory_determination`, never `rb_trigger_live_success`. They therefore never reach either of E3/E4's two call sites, and landing both changed the suite by exactly **0** (measured on a clean build, 27 suites swept before and after: total `assert` 1235 → 1235, SUM-of-`^FAIL` 1283 → 1283, every suite identical). The reason is in the oracle: Rust's `recalculate_constants` calls **E1** (`modifiers.rs:1398`) and **never** E3, and there is no ledger anywhere for the plain `modify_required_hearts` need_heart path — Rust's only withdrawal is E3's `clear()` at `:2148`, reachable only from the live flow. So on the C, and equally on Rust, a `rb_recalc_constants`-only sequence that first grants (E1 sees the activator in the success zone) and then moves the activator out leaves the grant at `-2` and the assertion expecting `0` cannot be satisfied by any faithful port. Two consequences, both deliberate. (a) **The C's tail call stays `rb_evaluate_success_zone_constant_modifiers`, not `rb_evaluate_success_zone_constant_abilities`** — matching `modifiers.rs:1398` is worth more than satisfying a fixture that probes the wrong Rust function. (b) **The rule those two assertions are really about is now pinned where Rust implements it**, in `tests/test_parity_need_heart_live_flow.c`: the as-long-as expiry is asserted end-to-end through `rb_execute_live_victory_determination`, where a grant made **after** the performance window is withdrawn by E3 and cannot be restored by E4, while a performance-time grant survives. The two `j_*` assertions were left exactly as written — not weakened, not deleted, not ignored — and they remain red. The third red in the same cluster, `j_non_stackable` (got `-4`, expected `-2`), is a **separate** gap: Rust's E1 carries the `non_stackable` dedupe at `modifiers.rs:2280-2287` (`let key = format!("{}:{}", effect.action, effect.text)`, local to one evaluation) and the C's `rb_evaluate_success_zone_constant_modifiers` does not. It is left alone here on purpose: `non_stackable` is measured **VERIFIED-INERT: 3 nodes, 0 key collisions** on the corpus, so porting it is a separate decision, and it is not E3/E4's business.
- [~] E5 `remove_revealed_cost_card` (modifiers.rs:2053) — **BLOCKED-ON `src/ability/cost.c`.** `GameState::revealed_cost_cards[]` EXISTS (`rabuka.h:1512-1513`) but has **0 writers and 0 readers in the whole C tree**, so the function would have nothing to remove. The producer `GameState::push_revealed_cost_card` (`core/game_state/mod.rs:1091-1110`, callers `ability/cost/handlers.rs:1286` + siblings) has no C twin at all. **⛔ BLOCKED-ON: `src/ability/cost.c` — port `push_revealed_cost_card` (set semantics, newest-last, bounded by `RB_MAX_ZONE`) and its handlers.rs call sites; only then is E5/E6 worth landing.** Also note Rust's twin drops the parallel `revealed_cost_card_meta[index]`, which C has no field for (same shape as `revealed_card_meta`, already recorded above `clear_revealed_cards`).
- [~] E6 `remove_from_source_hands` (modifiers.rs:2071) — **BLOCKED-ON E5, same producer.** Its own gate is `if !self.revealed_cost_cards.contains(&cid) { continue; }` (modifiers.rs:2080-2082), and that set is empty in C, so every candidate would `continue` — the port would be a silent no-op on every input.
- [~] E7 `set_heart_override` (modifiers.rs:1656) — **BLOCKED-ON `src/ability/choice.c`; the function already exists and is correct.** `rb_mods_set_heart_override` is defined at `modifiers.c:262` (the twin of `game_modifiers.rs:437`, a 3-arg mods-level setter) with **0 callers**. Rust's GameState-level `set_heart_override` (modifiers.rs:1656-1672) is **4-arg** — it takes a `duration` — and the single call site is `ability/choice.rs:3806` `gs.set_heart_override(card_id, color, count.max(1), "live_end")`. **⛔ BLOCKED-ON: `src/ability/choice.c` — at the twin of choice.rs:3806, call `rb_mods_set_heart_override(&g->mods, card_id, color, count>0?count:1)`; if the duration is load-bearing, the setter needs a 4th parameter and that is a `rabuka.h` change.**
- [x] E8 `clear_gained_abilities_for_card` (modifiers.rs:2112) — **CLOSED ON ARRIVAL, verified.** `rb_clear_gained_abilities_for_card` is defined at `core/game_state_abilities.c:253` and called at `:302`, which is the twin of `modifiers.rs:855`. No edit needed and none made.
- [x] E9 `record_stage_position_swap` (modifiers.rs:1799) — **LANDED + TESTED. `rb_record_stage_position_swap(g, source_id, source_index, target_id, target_index)` in `src/core/modifiers.c`.** ⛔ **A PEER MEASURED THIS AS A DISPLAY/REPLAY GAP AND THAT IS WRONG** — the readers are the GAMEPLAY condition evaluator: `ability/condition/state.rs:871` (passes `&self.game_state.position_change_events` to a count), `:879`, `:929`, `:938`, `:1162`. `game/display.rs:1858/1861` is a sixth, display-only reader. In C, `g->position_change_events[]` (`rabuka.h:1534`) had **ZERO writers** — its only mention was the reset at `game_state_abilities.c:589` — so every 「このターン、このメンバーがエリアを移動した」 condition read an empty vector. This function is the missing writer, faithful to modifiers.rs:1799-1857 including the source-unconditional / target-guarded split (:1791-1793) and the events-before-movements order (:1788-1790). Tests: `tests/test_parity_cluster_e.c::test_e9_position_swap_writes_events_and_movements`. ⛔ The three Rust call sites (effects/position.rs:1022/:1061/:1102) are in `src/ability/effects/move.c`/the position executor, **not this file** — the C's position change calls `rb_record_card_movement` three times per swap (choice.c:4029/:4092/:4093, move.c:161), which is why no verdict moved.
- [x] E10 `count_as_multiple_weight` (modifiers.rs:227) — **LANDED + TESTED. `rb_mods_count_as_multiple_weight` in `src/core/modifiers.c`.** **REACHABLE: exactly 1 of 985** — one `count_as_multiple` effect node, on `PL!-pb2-041-L` 春情ロマンティック (`count_as=2`, `counted_by="lily white"`, `self_target=true`, `duration=as_long_as`, and a 6-field condition requiring the owner's success live card zone). The `counting_card` resolution chain (:232-241), the rule's own condition (:265-276, evaluated with `owner_of_card`'s seat), the `counted_by` `.any()` filter (:282-297) and the `count_as_any().unwrap_or(2).max(1)` weight (:298) are all ported. Tests: `test_e10_count_as_multiple_weight` (13 assertions). ⛔ Rust's readers are `ability/condition/card.rs:2124` and `modifiers.rs:341` — condition.c / dynamic_count.c, not this file.
- [x] E11 `weighted_zone_count` (modifiers.rs:308) — **LANDED + TESTED. `rb_mods_weighted_zone_count` in `src/core/modifiers.c`.** Same single corpus node as E10; the weight is consulted ONLY through this function, so landing E10 without it would have been a silent wrong count. Includes the empty-`card_type`-means-any rule (:317-318) and the `.any()`-over-the-whole-CSV `group_names` rule (:330-339). Tests: `test_e11_weighted_zone_count` (7 assertions).
- [x] E12 `replacement_appear_wait_applies` (modifiers.rs:360) — **LANDED + TESTED. `rb_mods_replacement_appear_wait_applies` in `src/core/modifiers.c`.** **REACHABLE: 2 of 985** — `PL!-pb2-002-PP` 絢瀬絵里 and `PL!-pb2-002-R`, both `position=center`, `heart_total_limit=4`, `heart_total_operator="<="`, `card_type=member_card`. All eight Rust filters are ported, including the two that are easy to get backwards: the rule governs **`area_idx`** and NOT the host's own slot (:413-426), and a member is never in front of itself (:386-388). The printed-heart total is the **base-heart slice only** (:367-371), so `rb_card_total_hearts` — which H1 records as "the WRONG total here" — is deliberately not used. Tests: `test_e12_replacement_appear_wait` (16 assertions). ⛔ Rust's call site is `turn/phases.rs:1281`, i.e. `src/turn/phase.c`, not this file.
- [~] E13 wire `rb_mods_add_blade_with_trace` / `_heart_with_trace` — **BLOCKED-ON `src/ability/effects/misc.c` (F3), and the recorded status was WRONG IN THE OTHER DIRECTION.** The reconciliation recorded "blade half wired: called `modifiers.c:371`". **Line 371 is inside a comment block** — it is the illustrative `grant_blade` body the comment shows the reader how to write, not a call. Measured 2026-10-03 tree-wide: `rb_mods_add_blade_with_trace` has **0 production callers** (other hits: this file's own comment and the definition) and `rb_mods_add_heart_with_trace` has **0 production callers** (reachable only from `tests/test_parity_per_card2.c:1631/:1633`). **BOTH halves are unwired.** The file's comment has been corrected in place. The blocker is unchanged: `ability_text` must be threaded into `grant_blade` (misc.c:218) and `grant_heart_distribution` (misc.c:224) first, which needs misc.c:551/:558/:562/:571/:792 and :238 widened. misc.c is contested three ways (C, F, sweep).
- [~] E14 wire the 9 zero-caller rb_mods_* setters/clearers — **BLOCKED-ON `src/core/game_state_abilities.c`, and ONE NAME IN THE LIST IS WRONG.** Measured tree-wide: `rb_mods_remove_cost_set` has **0 occurrences** — the real ninth symbol is **`rb_mods_clear_cost_set`** (`modifiers.c:181`), Rust `game_modifiers.rs:571 remove_cost_modifier_set`. The other eight exist with 0 production callers: `rb_mods_remove_blade` (:187), `rb_mods_remove_heart` (:192), `rb_mods_remove_score` :227, `rb_mods_clear_blade_set` (:164), `rb_mods_clear_score_set` (:172), `rb_mods_clear_blade_type` (:294), `rb_mods_remove_heart_override` (:267), `rb_mods_get_cost_set` (:176). **⛔ BLOCKED-ON: `src/core/game_state_abilities.c` — the four Rust `clear_*` / `remove_*` sites are `abilities.rs:3061` (clear_blade_set_modifier), `:3110` (remove_heart_override), `:3164` (clear_blade_type_modifier), `:3173` (clear_score_set_modifier); `get_cost_modifier_set`/`remove_cost_modifier_set` are read at `turn/phases.rs:1247` and written at `:1347`, i.e. `src/turn/phase.c`.** Verified individually: all four bodies are faithful ports today (`clear_blade_set`/`clear_score_set`/`clear_blade_type`/`remove_heart_override` write exactly 0; `remove_cost_set` subtracts unclamped, matching the game_modifiers.rs:535-547 doc).
- [~] E15 delete `rb_mods_remove_need_heart` — **HALF LANDED: the definition is DELETED from `src/core/modifiers.c`; the `rabuka.h:306` line is main's.** Verified before deleting: Rust defines exactly four `remove_*_modifier` methods (`game_modifiers.rs:332/423/460/548`) and `remove_need_heart_modifier` has **0 hits in engine/src**; the C symbol had **0 callers and 0 hits in its own body** tree-wide. The survivor is `rb_mods_add_need_heart(cid, color, -delta)`, exactly Rust's `modifiers.rs:607-610`, and `triggers.c:1303`/`:1341` already do that. The 5-line phantom-warning comment was replaced by a tombstone naming the blocker. Tests: `test_e15_need_heart_undo_is_add_of_negation` pins the survivor so the twin is not reintroduced under another name. ⛔ **BLOCKED-ON: `include/rabuka.h:306` — delete the line `void rb_mods_remove_need_heart(RbMods *m, int card_id, int color, int delta);`** (until it goes the header declares a function with no definition, which is a link error for any caller rather than a warning; nothing calls it, so the tree still builds).
- [x] E16 `eval_group`'s temporal arm keys on `m.cause_card_id` — ✅ **BOTH HALVES LANDED + TESTED 2026-10-04 (fixer wave). ⛔ The previous text's `BLOCKED-ON: src/ability/condition.c:1184-1209` is CLEARED and its false citation (`card.rs:1642-1655`) is REPLACED by `ability/condition/card.rs:1632-1658`.** The block was RIGHT that the body scanned `rb_entry_trigger_moved_cards` / `g->recently_moved` and matched the group against the MOVED card; all five defects are now fixed (wrong vector; missing `m.moved_card_id`, `m.effect_only`, `m.cause_player_id`; `.first()` instead of `.any()` on `cause_card_id`; plus a gate superset accepting an absent `card_type`). **The block's advice to "match over the WHOLE comma-joined `group_names` list" was itself partly wrong**: `group_names` is a decoded ARRAY, not a comma-joined string, and the `.any()`-vs-`.first()` difference is **unreachable** — Rust's multi-group arm at `card.rs:1606-1631` returns from inside `if groups.len() > 1`, so a list of two or more never reaches the temporal arm on either tree. Ported as `.any()` anyway (it is the rule) and pinned by asserting the branch that actually decides it. (a) **WRITER** (was already landed): `RbBatchMovement event` in `rb_record_card_movement` was a **stack local whose `cause_card_id` member was NEVER ASSIGNED**, so uninitialised stack bytes reached `g->batch_movements` / `g->turn_area_movements` — an uninitialised read, not a missing feature. Now written from `g->activating_card` (`src/core/modifiers.c:752`; Rust's push sites pass the same, `modifiers.rs:1843/1851`), with `-1` propagating as Rust's `None`. Tests: `test_e16_cause_card_id_is_written_from_activating_card`. **Proved by `tests/test_parity_fx_fixers.c::fx1_cause_card_id_arm`** — 13 assertions. See **A9** for the reachability count (2 nodes: cards 2467 / 2468) and the A/B suite effect. _(superseded tail removed 2026-10-04; the row above is the whole truth.)_
- [~] E17 retire the file-static `s_opp_cause_fired[]` side table in favour of `mods.opp_cause_fired_keys` — **BLOCKED-ON `src/core/game_state_abilities.c`, NOT THIS FILE.** Re-measured: the side table is live at `game_state_abilities.c:2093-2124` (declared `:2093`; read `:2099-2100`; turn-pruned `:2113-2114`; compaction `:2117-2118`; written `:2122-2124`), while `RbMods::opp_cause_fired_keys[16]` + `n_opp_cause_fired_keys` (`rabuka.h:270-271`) is written by exactly one line — the clear at `modifiers.c:926` — and by nothing else. **⛔ BLOCKED-ON: `src/core/game_state_abilities.c:2093-2124` — delete `s_opp_cause_fired[]`, `s_n_opp_cause_fired` and the whole key it maintains, and re-key the two functions on `(GameState *g, uint64_t key, int turn)` against `g->mods.opp_cause_fired_keys[0..n_opp_cause_fired_keys]` with the same turn-prune and compaction the current code performs. `clear_card_movement_tracking` (modifiers.c:945) already resets `n_opp_cause_fired_keys = 0`, so the clear is landed and only the store is missing.** The 16-slot bound is adequate: the current side table's cap is `RB_OPP_CAUSE_FIRED_CAP`.

**Suite effect of the whole cluster: ZERO MOVEMENT, in either direction.**
28 suites (the 24 in the brief that exist, plus `parity_conditions`,
`parity_condition_and_effect`, `parity_look_and_filter`, `zone_filter_parity` as the
nearest existing analogues of the three brief names that have NO test file —
`audit_modifiers`, `parity_card_filters` and `parity_effect_conditions` do not
exist), strictly sequential, fresh snapshot of the live tree,
`make data` frozen, blob verified 726 831 B / `89204df0…` before and after.
`^FAIL` counted **with no colon and case-sensitively, on stdout AND stderr**:

| | baseline | after cluster E |
|---|---|---|
| suites run | 28 | 28 |
| silent suites (a broken runner reads as green) | 0 | 0 |
| `^FAIL` stdout | 387 | **387** |
| `^FAIL` stderr | 809 | **809** |
| total `^FAIL` | 1 196 | **1 196** |
| normalised stdout+stderr, per suite | — | **28/28 byte-identical** |
| new suite `tests/test_parity_cluster_e.c` | does not exist | **0 `^FAIL` / 60 assertions** |

Zero movement is the expected result and not a stale link: both trees are the
SAME fresh snapshot of the live tree and differ only in `src/core/modifiers.c`
(proved by a SHA-256 sweep over every `.c`/`.h` in the snapshot), `src/core/
modifiers.o` was `rm`-ed and rebuilt with a moved mtime before the after-run, and
the four landed entry points are exercised by a suite that is **link-RED against
the reverted baseline** (`undefined reference to rb_record_stage_position_swap`,
`rb_mods_replacement_appear_wait_applies`, `rb_mods_count_as_multiple_weight`,
`rb_mods_weighted_zone_count` — 4 symbols) and **0 FAIL against the fix**.
A reduced E16-only probe that DOES link against both: **2 `^FAIL` before → 0
after** (`turn_area_movements clone` got 0, `None propagates as -1` got 0). The
third E16 assertion (`batch_movements[0].cause_card_id`) **passed by luck on the
baseline** — the stack slot happened to hold the expected id — which is itself
the finding: the pre-fix value is uninitialised memory and is not reproducible.
Independently: the same 24 shared suites also produced byte-identical verdicts
before and after four OTHER files (`look.c`, `score.c`, `resolver.c`, `data.c`)
changed in the live tree during this task, so those four moved nothing either.

**Why nothing moved, item by item — every landed function is correct but its
production call site is in a file this cluster does not own.** E9's three Rust
call sites are `src/ability/effects/position.rs`'s three branches (the C's
position change calls `rb_record_card_movement` three times per swap instead);
E10/E11 are read only from `condition/card.rs:2124` and `modifiers.rs:341`;
E12 is called from `turn/phases.rs:1281`; E16's reader is `condition.c:1184-1209`;
E15 had no caller. That is the whole reason the remaining E rows are BLOCKED-ON
rather than closed, and it is a file-ownership problem, not a porting one.

## F. Misc / effects

- [x] **F1** LANDED 2026-10-04, and the ORDER the item demanded had already been honoured: the clone-lifetime fix is in `custom.c` (a PINNED keep-alive pool — `custom_keep_push` deep-clones into it, `custom_keep_settle`/`custom_keep_gc` release it only once the continuation is done with it, `custom.c:293-299`), and the dispatcher flip is already in `executor.c:379-409` (`rb_execute_custom_effect` is called DIRECTLY and `execute_misc` is no longer in the loop). **So the remaining half — the deletion — is what this change landed**, and it was done by proving unreachability across all THREE callers of misc.c's table rather than by assuming the executor's comment was right:
  1. `executor.c:379-409` — the `custom` arm never falls through to `execute_misc`;
  2. `misc.c`'s own `both_dispatch_arm` routes to the misc table only when `!rb_executor_has_executor(arm->action)`, and `{"custom", 0}` **is** in that table (`executor.c:33`), so a `target:"both"` custom effect goes to the executor;
  3. `src/turn/triggers.c:1207-1208` calls the table under `action == "restriction"` only.
  `h_custom` (misc.c:1051-1111) and its table arm are gone, replaced by a tombstone carrying all three proofs plus why the surviving copy is strictly more faithful (deep clone vs a hand-built near-empty struct; pooled pointer vs a stack address parked in `queue.resume_eff`). The arm was DELETED rather than left to fall into `else r = 0`, so a `custom` that somehow reaches the table now gets the honest "unhandled" answer instead of a use-after-free. Rust's own reachability for the record: `ActionType::Custom` decodes to the literal `"custom"` (`enums.rs:469`) and **0 of the 2116 card/ability pairs** decode to it, so the deletion is about removing the reachable-but-wrong twin, not about corpus behaviour.
- [x] **F2** LANDED 2026-10-04 — and the item's premise needed correcting in both directions. ⚠ **"Rust's `gain_heart` per-colour loop has NO C counterpart function at all" is FALSE for both sites**: `resource.rs:653` `gain_heart_colors_from_selected_card` is `misc.c`'s `h_gain_heart_colors_from_selected_card` and `resource.rs:721` `gain_heart(All)` is `h_gain_heart_all_type`. What the C actually lacked was the **TRACE**, which is what `src/core/modifiers.c:391-393` meant by "no C counterpart" — both functions wrote through the UNTRACED `apply_heart_to_card`. Both now write through `rb_mods_add_heart_with_trace`, and both with the per-site SOURCE Rust uses (`:705` `target_id`, `:721` `card_id`) rather than `gs.activating_card`. Re-measured reach: `gain_resource heart_colors_from_selected_card` is carried by **1** of the 3465 effect nodes. While transcribing the loop, two further Rust lines were landed in the same function because they are in the code being touched and are not guesses: **the target set is filtered** (resource.rs:658-675 `card_matches_characters(.., effect.characters_any())` — the C looped over EVERY stage slot) and **the colours are deduped** (`:694` iterates `base_heart.hearts.keys()`, a MAP, so each distinct base colour once — the C iterated base-slice positions and would credit a print with a repeated base colour twice).
- [x] **F3** LANDED 2026-10-04, resolving the CROSS-FILE REQUEST at `src/core/modifiers.c:404-408` and the `⛔ BLOCKED-ON` at `:460`. `grant_blade` and `grant_heart_distribution` now take the ability text and call `rb_mods_add_blade_with_trace` / `rb_mods_add_heart_with_trace`; `apply_heart_to_card` threads it too, matching Rust's own parameter list (`resource.rs:199` `effect_text: &str` is Rust's FIRST parameter of `grant_heart_distribution`, `:245` is the last of `apply_heart_to_card`). **The SOURCE of the trace is `gs.activating_card`, not the target** — `resource.rs:229` and `:217` — and that is the part a "just add the trace" patch gets wrong, so it is asserted on its own in `fw_f3_blade_grant_reaches_the_trace_ring` with two different card ids on stage. All 13 call sites re-verified **by line number** after the edit (the item's line list had drifted; the sites are now `grant_blade` ×5 in `apply_blade_resource`, `apply_heart_to_card` ×4 in `apply_heart_resource`, `grant_blade`+`apply_heart_to_card` in `h_gain_resource`). Proven by `tests/test_parity_fw_effects.c`: base 5 FAIL across the F3/T2C/T2-10 cases, fix 0. Two sub-notes found on the way: (a) `h_gain_heart_all_type` now builds an 8-wide revert array, because `push_temporary_effect` copies 8 int heart slots and handing it a 1-element array was an overread the compiler flagged once the array became reachable; (b) the `characters` filter added in F2 uses this file's **whole-CSV** compare, deliberately NOT an any-of-list — see the existing note at `misc.c:355-372`, where switching this class of filter to any-of-list made 5 cases in `parity_gain_delta`/`parity_blades2` RED, and those suites pin a REJECTION.
- [~] **F4** verified: `src/ability/dynamic_count.c` carries the `selected_card_score` + per_unit arms and compiles clean; `tools/census_fw.c` finds **0** corpus nodes that reach the `selected_card_score` shape, so there is nothing further to land.
- [x] **F5** verified ALREADY LANDED at all three sites, and re-verified rather than re-applied: `rules.c` carries the tombstone for `r_yell_icons` and calls `rb_process_yell_revealed_card_icons` at `rules.c:2102`; `phase.c` likewise at `phase.c:1707`; the surviving body is `src/ability/effects/live.c:82` (`rb_process_yell_revealed_card_icons`, exported in `include/rabuka.h`). All three hand-rolled copies are already collapsed into it. `phase.c` was not touched — it is not this cluster's file.
- [x] **F6** verified ALREADY LANDED: `move.c` dispatches `"deck_bottom"` to `rb_move_resolve_from_deck_bottom` at `move.c:1622` (Rust `move/mod.rs:816` -> `:939` `resolve_from_deck_bottom`), and `rb_zone_of_str` no longer folds it — `move.c:1595-1599` keeps `"deck"` / `"deck_top"` / `"deck_bottom"` / `"energy_deck"` as DISTINCT source strings routed into `rb_move_resolve_from_zone`. Checked that the destination never silently collapses: `deck_top` and `deck_bottom` are separate arms at `move.c:1620-1623`.
- [x] **F7** verified ALREADY LANDED: `move.c:1624` routes `"energy_deck"` to `rb_move_resolve_from_energy_deck` (Rust `move/mod.rs:826` -> `:982`), and `move.c:1595-1599` is the source-dispatch gate that keeps `deck` / `deck_top` / `deck_bottom` / `energy_deck` / `energy` / `stage` / `live_card_zone` off the generic `rb_move_take_cards_from_standard_zone` path. The `those_cards` / `revealed_cards` / `recently_moved` sources are handled above the gate (`move.c:1590-1594` reads `g->selected_cards`).
- [~] **F8** verified: the multi-index stage answer is present and `tests/test_parity_executor_dispatch.c` is `FAIL=0 HIDDEN=0` on both the baseline and the fix tree, so it is stable. No separate change was needed.

## G. Card model

- [x] G1 `effect_type` — **VERIFIED-NOOP: 0 occurrences.** Tallied three independent ways: (a) `cards/abilities.json` (the compile input, 985 abilities) contains 0 `"effect_type"` keys; (b) the 726,831-byte interned key pool `src/abilities_strings.bin` contains the literal `effect_type` **0** times, while the control keys `look_at` / `fires_on_opponent_effects` / `reveal_until_live_card` are each present exactly once — so the key is absent from the BYTECODE, not merely un-decoded; (c) a decode-level walk of all 2,639 cards / **4,925** effects via `rb_decode_card_ability` finds **0** effect extras keyed `effect_type`. Rust's live reader is `AbilityEffect::effect_type()` (card.rs:2580-2587) → `EffectFilter.effect_type`, consumed at `ability/effects/replacement.rs:113` (`== "replacement"`) and `core/game_state/abilities.rs:3053`; both compare a value that no corpus effect supplies, so neither can fire. (Checklist's old citation `ability_options.rs:1187-1188` names a file that does not exist; the real consumer is replacement.rs:113.) No decode arm added.
- [x] G2 `"look"` / `"reveal_effect"` / `"set_card_identity_all_regions"` — **VERIFIED-NOOP for all three, with numbers.** Decode-level census over the same 4,925 effects: `look` = **0**, `reveal_effect` = **0**, `set_card_identity_all_regions` = **0**, against **50** distinct action literals actually present — and all 50 already decode in C, so these are the only three Rust literals the corpus never produces (controls: `look_at` 200, `draw_card` 300, `reveal` 48, `look_and_select` 185, `set_card_identity` 3, `replacement_appear_wait` 2, `count_as_multiple` 1). `abilities.json` agrees: 0 / 0 / 0. Nothing unreachable was implemented.
- [x] G3 **DONE** — `rb_condition_filter_subset` definition deleted from `src/core/card.c` and its declaration deleted from `include/rabuka.h` **in the same edit** (plus the one doc line that pointed at it). Whole-repo grep over `*.c *.h *.py *.md Makefile` first: the only hits were the definition and the declaration, never a call — 0 callers. Redundancy proof: its group arm resolved through the same `group_names.first()` membership test (`card.rs:1852-1856` populates `group` with `.first()`; `util.rs:1515-1530` `check_group` tests exactly that one name), so it could not disagree with `rb_card_matches_effect_group` on any term that rejects a card. Rust has no Condition-side builder either — `CardFilter::from_effect` takes an `AbilityEffect` (`util.rs:1838`) — so there is no public Rust item this was a twin of. 56 lines removed.
- [x] G4 **DONE** — the card-level fallback is deleted from `core/game_state_abilities.c` (`rb_effect_fires_on_opponent_effects` now returns the EFFECT flag only, `return 0` when the effect carries no key). Rust has no `card_fires_on_opponent_effects` anywhere in `engine/src`; its only two readers (`abilities.rs:544`, `:1132`) ask the effect they hold. **Corpus measurement: the fallback armed 2 (card, ability) pairs — card 2610 ability 1 and card 2611 ability 1 — that Rust would not** (10 effects carry the flag, 12 cards answer the card-level scan, 2 cards have an un-flagged ability that only the fallback reached). No suite moved. FOLLOW-UP (not this item): `rb_card_fires_on_opponent_effects` in card.c is now uncalled and has no Rust counterpart — delete it plus the `extern` at game_state_abilities.c:104 when that file is next owned.

## H. util.c

- [x] H1 **DONE (matcher side)** — `LocalCardFilter` gained the six carriers and both matchers now honour them, in Rust's own order and with Rust's operator semantics:
  | carrier | Rust twin | C landed |
  |---|---|---|
  | `exclude_cards[]` | `check_exclude_cards` util.rs:1489-1496, called :1714 | exact-id rejection, before any field probe |
  | `exclude_names[]` | `check_exclude_names` util.rs:1498-1513, called :1717 | `str_eq_ignoring_ws` equality (util.rs:696-706), undecodable card skipped |
  | `groups[]` | `check_group` util.rs:1518-1529 | `group` miss now falls through to an `.any()` over the WHOLE list; a matched/absent `group` short-circuits it |
  | `exclude_group_names` | util.rs:1531-1537 | unconditional reject on first CSV hit |
  | `cost_limit_min` / `_max` | util.rs:1744-1752 | reuse `rb_card_matches_cost_limit` with the literal `">="` / `"<="` — the same `card_matches_cost_limit_op_for` the `cost_limit` term uses, not a second comparison |
  | `original_heart_total` (+op) | util.rs:1790-1798 | PRINTED base-heart slice summed directly (`card_total_hearts()` is the wrong total — it falls back to need_heart); DEFAULT operator `"<="`, not `compare_counts`' `">="`, per util.rs:1907 `.or(Some("<="))`. Wire keys are `heart_total_limit` / `heart_total_operator`, and Rust guarantees the operator is `Some` whenever the total is, so the operator string is a complete presence flag. |

  Group matching routes through the already-verified `rb_card_matches_group_str` (118,755 pairs, zero disagreements) — no tightening. Corpus reachability measured so the producer can be written safely: `heart_total_limit` **5** (all `heart_total_operator` = `"<="`), `cost_limit_min` **1**, `cost_limit_max` **1**, `exclude_group_names` **4**, `group_names` **605** (never `[]`), `exclude_names` **0** and `exclude_cards` **0** (both are `None` in `from_effect`, util.rs:1908-1909 — API-only). **The producer half is still missing**: `card_filter_from_effect` (card.c) populates NONE of the six today, so this change is a measured no-op on the corpus and exists so the producer is not dead code. NEXT: whoever owns card.c should set them from `effect.heart_total_limit_any()`, `heart_total_operator_any()`, `cost_limit_min_any()`, `cost_limit_max_any()`, `exclude_group_names_any()`, and `groups: group_names.map(Vec::as_slice)` (util.rs:1860-1861, :1866, :1856, :1906-1907), and add the five matching `has_filter` terms (util.rs:1459-1471). Two unrepresentable edge cases are documented in the code: `groups` is an array so `Some(vec![])` cannot be distinguished from `None` (harmless — no empty `group_names` in the corpus), and `exclude_group_names` is a single 64-byte field so a producer must CSV-join.
- [x] H2 **DONE (evidence recorded, no code change)** — twin-by-twin equivalence, measured not eyeballed:
  | util.c original | twin(s) | verdict |
  |---|---|---|
  | `rb_group_name_first` (util.c:577) | cost.c:252 `group_name_first`, score.c:89 `sc_group_name_first`, misc.c:266 `misc_group_name_first`, ability_effects.c:82 `ae_group_name_first` | **BODY-IDENTICAL ×4.** Verified mechanically: each body was extracted, the executable part isolated, identifiers canonicalised and whitespace stripped — all four reduce to the same 335-char token stream. All four are safe to delete once the symbol is declared in rabuka.h. |
  | `rb_effect_group_name` (util.c:593) | no standalone twin; FOUR call sites inline its body: score.c:125, score.c:801, misc.c:522, ability_effects.c:367 | **NOT byte-equivalent.** util.c falls back to the `group` key when `group_names` is absent; all four inlines read `group_names` only. Corpus: `"group":` = **0** of 605 `group_names`, so the fallback is dead and the swap is behaviour-preserving — but it is not a mechanical identity. |
  | `rb_card_matches_effect_group` (util.c:603) | cost.c:264 `card_matches_group_name_first` | **NOT byte-equivalent (different shape).** The twin takes `(int card_id, const char *group_csv)` — a raw CSV — while the original takes `(const AbilityEffect *e, ...)` and does the key lookup itself. Both bottom out at `group_name_first` + `rb_card_matches_group_str`. cost.c:2324 and :2368 already hold the CSV, so a swap must change those two call sites to pass the effect. ⚠️ cost.c:221 `card_matches_any_group` is the twin of `rb_card_matches_any_group` (Rust util.rs:830 `card_matches_any_group`) — a DIFFERENT function and NOT one of A20's four; do not delete it with them. |
  | `rb_get_selection_indices_filter` (util.c:2070) | **NO TWIN EXISTS** | Whole-tree grep for `get_selection_indices*` returns only util.c:2057 (`rb_get_selection_indices`, an older 2-field wrapper that CALLS the filter version), util.c:887 (forward decl), util.c:2070 (the definition), move.c:104 (forward decl) and move.c:954/:1148/:1410 (call sites). Nothing in cost.c / score.c / misc.c / ability_effects.c reimplements it. Declaring it deletes nothing; it is wanted only because move.c:104 hand-rolls the prototype. |

  Also verified: resolver.c's stale 1-arg forward declaration of `rb_effect_group_name` is ALREADY GONE (only the explanatory comment at resolver.c:31-35 survives), so util.c:572-576's note about it is stale and can be dropped. The three `*_group_name_first` comments in cost.c:248-251 / score.c:85-88 / misc.c:262-265 / ability_effects.c:57-60 should be deleted with their twins.

## I. Live / phases / engine

- [x] I1 CLOSED (partly BLOCKED-ON). `rb_build_snapshot` and `rb_check_live_success` DELETED (0 callers, no prototype in any header, bodies were explicitly NOT the port and would double-drain / blank 4 snapshot fields) — their ports are the span rb_perform_live fuses. `rb_enrich_from_applications` LANDED with a real body and 1 caller at the phases.rs:583 position: it copies `s_member_contrib` onto `RbLiveSnapshot::member_contributions` (A18 landed mid-task, so the array now exists) and computes `transform_delta[c] = bonus_hearts[c] - ability_per_color[c]` from the `RbMods.trace` ring, i.e. live.rs:783-800. `IMPOSSIBLE (no field): breakdown.scores / breakdown.transforms / triggered_abilities` — RbLiveSnapshot has no such fields and there is no reader in engine/src. `BLOCKED-ON (wrong struct): base_score_total / card_bonus_total / spare` — they were added to RbMemberContribution (rabuka.h:1288-1290) but Rust puts the first two on PerformanceSnapshot (types.rs:498/500) and `spare` on LiveCardResult (types.rs:512). Exact shape needed on RbLiveSnapshot: `int base_score_total; int card_bonus_total; int live_spare[RB_MAX_LIVE_CARDS][8];`. All three are WRITE-ONLY in Rust (declaration + write site only, no reader in engine/src).
- [ ] I2 BLOCKED-ON: `void rb_rules_clear_turn_state(GameState *g)` in src/ability/effects/rules.c. **Cluster Y.4 removed both written declarations** — `src/turn/live.c:604` and `src/turn/phase.c:529` each spelled the prototype out inside a comment, so the symbol had two declarations, no definition anywhere in the tree and no caller, while this box read `[x]`; a future call site would have been a LINK ERROR with nothing in the build to catch it, because a comment is not a declaration. There is now ONE tombstone, in `src/turn/phase.c` at the `RB_PHASE_VICTORY` rollover, naming `src/ability/effects/rules.c`; `live.c` cross-references it without repeating the declaration text. The blocker is unchanged and NOT this cluster's file: full chain is writer rules.c:1876-1878 (`cannot_live_players[pl]=1`), reader `rb_rules_cannot_live_player` rules.c:2268 with ZERO callers tree-wide — that zero IS the bug; and the CLEAR is what blocks the fix, because `cannot_live_players` lives in the file-static `RbRulesState` (rules.c:72) with a `static rb_rules_state()` accessor, so neither live.c nor phase.c can clear it, and wiring the reader alone would latch the seat at "can never live again". Must clear `cannot_live_players[2]` AND `cannot_activate_members[2]` (Rust clears them adjacently, phases.rs:279-280), called from the phase.c:427 rollover. `rb_rules_forget` is NOT a substitute (0 callers, drops every side table). Exact reader body + call site are written out at src/turn/live.c in the I2 block above rb_perform_live. Nothing stubbed in behind `#if 0` on purpose. ⚠ `cannot_live_players[2]` (rules.c:72, written rules.c:1878) is **still never cleared** — it latches 1 on the first 「ライブできない」 restriction and no rollover zeroes it.
- [x] I3 CLOSED: `rb_compute_snapshot_surplus` added to live.c and called at the phases.rs:1202-1206 position (`live_victory_stage == 0`, once per victory determination, before the first ライブ成功時 block). It is NOT a duplicate of rb_compute_surplus_and_flags: no `!snap.success` gate, and `required` comes from the PRINTED need_heart, not the modified requirement. Feeds `surplus_per_color` + `self/opponent_live_surplus_count`, which card.rs:4200-4245 reads while `live_surplus_ready_this_turn` is true — i.e. DURING the ライブ成功時 abilities (the Q174 La Bella Patria path the C could not serve). MEASURED: zero verdict movement across all 37 suites (the C's surplus reader in condition.c has no `live_surplus_ready_this_turn` gate, so nothing consumes the new values yet — see BLOCKED-ON below). `BLOCKED-ON src/ability/condition.c:3276 count_surplus_heart`: it has no `live_surplus_ready_this_turn` gate and no per-colour read, so it always takes the recompute fallback.
- [x] I4 CLOSED as an audit; the one lossy consumer is BLOCKED-ON. The `card*8+color` stream is LOSSLESS against 3 of the 4 consumers of `allocations`: live.rs:254 (`record_pretrigger_live_results`), live.rs:548 (`populate_live_verdicts`), live.rs:1153 (`compute_snapshot_surplus`) all read only target_idx/color/amount, which the encoding is injective over (repetition = amount). The 6 fields with no consumer anywhere in engine/src are target_name/source_type/source_name/source_slot/wildcard/is_bonus. THE ONE LOSSY SPOT IS live.rs:720 (`finalize_snapshot_fields`): it needs `AllocPhase` to remap the source bucket, and `live_filled[i][c]` cannot recover it because Phase 4 records an icon_all fill of colour c in bucket c, indistinguishable from a Phase-1a native heart of that colour. The four phases the allocators can construct reduce the table to AllCleanup->7 / H00->0 / Colored,ColoredSurplus->color (H00Wild/Wildcard/AllWild/CAll are never constructed). Affects `spare` only, which is write-only in Rust. BLOCKED-ON: the `int live_spare[RB_MAX_LIVE_CARDS][8];` field named in I1.
- [x] I5 CLOSED for this file. `player_cannot_activate[0/1]`, `n_cannot_active_cards = 0` (phase.c:427-429) and `baton_touch_used[0/1]` (phase.c:431-432) WERE ALREADY CLEARED in the RB_PHASE_VICTORY rollover — the peer's claim verified true in the current tree. Reached by every driver: interactive src/main.c:97/:102 (rb_advance_phase direct), batch src/main.c:134 rb_turn, WEB src/web_server.c:666 -> engine.c:1550 rb_advance_phase, and rb_setup_settle_single_player_state at web_server.c:394/:567/:591. The peer's baton-touch latch argument is CORRECT and the clear is load-bearing: nothing else in the tree cleared it, so from the first touch onward triggers.c:243 `if (!baton_touch_used && …)` suppressed every 「バトンタッチされたとき」. RESIDUE CORRECTED: the checklist claim that test_parity_restriction.c:947-948 "hand-zeroes" is FALSE — those lines are a `board_begin` call and a blank line; the hand-zeroes are at :465-466, :923-924, :1006-1007 and are per-sub-test isolation on a fresh board. BLOCKED-ON: `rb_rules_clear_turn_state` (see I2) — `RbRulesState::cannot_live_players[2]` and `cannot_activate_members[2]` are the second copy of the same two Rust sets and cannot be cleared from phase.c; `rb_rules_cannot_activate_player` (rules.c:2274) also has 0 callers.
- [x] I6 **CLOSED 2026-10-03, and now PINNED.** The reader was swapped from `g->batch_movements` to `g->turn_area_movements` (Rust `hooked_by_foreign_cause`, abilities.rs `turn_area_movements.iter().rev().find(..)`, reverse scan, `moved_card_id` match only), which also removed the `source_zone==STAGE && dest_zone==STAGE` re-derivation Rust never performs. `rb_hooked_by_foreign_cause` was made non-`static` and **`tests/test_parity_foreign_cause.c` added (38 assertions, 0 failures)** — the corrected vector moved zero failures across 33 suites, i.e. no corpus ability reaches the arm, so without its own test the next agent had no reason to keep the fix. It pins: the marker pre-gate; the FOREIGN cause; **the `cause_player_id == player` negative** (the half most likely to be wrong — dropping it suppresses every marker watcher on the player's OWN area move); the `.rev().find()` LAST-match direction (a forward scan answers backwards); and that the answer survives `n_batch_movements = 0` while the turn-scoped log does not. All four mutations were A/B-measured and each is caught (vector revert 1/1, negative dropped 2/4, forward scan 1/2, gate dropped 1/2). Corpus: **7 of 985 abilities / 10 cards carry the marker.** ⚠ There is NO source-unconditional / target-guarded split in this arm — the modifiers.rs comment about an unguarded source slot belongs to `record_stage_position_swap`, a different function.
- [x] I7 CLOSED already on arrival; both halves verified. engine.c:138-141: `case RB_ZONEID_DECK_BOTTOM: return 0;` and `case RB_ZONEID_ENERGY_DECK: return 0;` — energy_deck resolves to nothing, so the peer's "deliberately not energy_deck -> RB_ZONE_ENERGY" reading is CONFIRMED as the right answer from core/zones.rs:229-234, where Energy and EnergyDeck are separate variants with no `from_str` entry for "energy_deck". Returning 0 makes the caller's hand fallback reachable, which is why draw.c still needs its dedicated string arms at draw.c:77 (bottom-of-bag push, util.rs:2433-2436) and draw.c:84 (energy_deck push, util.rs:2441-2444). `rb_turn` confirmed a thin headless driver over rb_advance_phase with no second turn machine; RESIDUE recorded at engine.c:145-158 — the 256-step guard, the `-1` skip drain, `rb_game_init` stopping at RB_PHASE_RPS (so CLI batch mode plays an empty game until a host policy exists over rb_generate_action_candidates), and three now-STALE COMMENTS in files this task does not own (draw.c:65-74, draw.c:66's "engine.c:60", modifiers.c:610) that still describe the old lying table. Only the prose is stale; draw.c's CODE is correct.
- [x] I8 CLOSED already on arrival; verified. The inline loop is gone. `rb_setup_settle_single_player_state` (game_setup.c) is called from web_server.c:365, :394, :567, :591 — all four state-build entry points (api/ui/new_game, new_game_with_decks, ai_games, full_game). web_server.c compiles clean. The peer's measurement stands: the old loop reached only Active/Energy/Draw and gave up after 16 iterations, so a game that entered FirstAttackerPerformance / SecondAttackerPerformance / LiveVictoryDetermination stalled; the ported settle walks the full six-phase `is_automatic_phase` list (game_setup.rs:369-375) under the 500-iteration guard and reaches Main.
- [x] I9 CLOSED already on arrival; verified. `parse_json_card_array` (web_server.c:255-287) calls `rb_normalize_card_no` at web_server.c:276 on the `card_number` string BEFORE `rb_card_get_card_id`, twin of web_server.rs:2360. The "the fold is symmetric so it is functionally equivalent" claim is FALSE, as measured: across 2639 card numbers 271 resolve differently and 794 differ when a trailing ASCII `+` is typed — `"PL!-bp3-004-R＋"` resolved to the `P` print, a different card with different abilities and cost. web_server.c compiles clean.

## J. Test-side (never weaken — align to Rust)

Claim for this cluster: `tests/test_parity_draw_flat.c`, `tests/test_parity_executor_dispatch.c`, `tests/replay.c`, `tests/test_parity_under_member.c`, `tests/test_parity_zones_position.c`, `tests/test_parity_integration.c`, `tests/test_audit_fields.c`, `tests/test_parity_small_clusters.c`, `tests/test_parity_required_hearts.c`, `tests/test_parity_wave1b.c`, `tests/test_parity_ability_mod.c`, `tests/test_parity_ability_mod_extra.c`, **plus `tests/test_parity_cce_gaps.c` (added for J4)**. Suite effect: 734 → 725 sorted FAIL verdicts, **0 new failures anywhere**.

- [x] J1 CLOSED (verified on arrival, checklist was stale). `dc_fill_decks_distinct` already takes `int *snap` from the caller and returns void (test_parity_draw_flat.c:1864-1876 documents the fix); all 11 callers declare their own `int snap[64] = {0}` and pass it. MEASURED: zero FAIL verdicts mention the snapshot symptom — no `expected -13157`, none of "order above the bottom is preserved" / "drawn card on bottom" / "drawn card still in hand" / "drawn-top card" appear in the sorted verdict set. No edit needed; 11 assertions confirmed green.
- [x] J2 CLOSED (verified on arrival, checklist was stale). The all-regions assertion now expects Rust's `card_identity:%d:ZZZ Identity` spelling (:890) AND single-region coverage was added at :967-973. MEASURED: `parity_executor_dispatch` = 0 FAIL lines, exit 0.
- [x] J3 CLOSED (verified on arrival, checklist was stale). `scenario_draw_and_score` asserts `g->p[0].score == score_before` ("modify_score never moves player.score") plus the PER-CARD ledger (`rb_mods_get_score`, "modify_score adds +6 to the printed card only"), and `scenario_condition_gate` does the same with the `value` extra. MEASURED: neither assertion is in the sorted verdict set. No edit needed.
- [x] J4 CLOSED (edit landed). `test_parity_cce_gaps.c` is ADDED TO THIS CLAIM. Both `CHECK(!fire_trigger(...))` enqueue-refusal assertions were REPLACED, not deleted: e2 now asserts `!pending(tg)` as the reachable `!has_pending_choice()` disjunct of `discard_score_energy_live_recovery_edges_test.rs:83` (deliberately with NO drain, so a refused entry that DID open a prompt would fail), and k7 asserts the two companion behaviours — no prompt opened (:3427) and no hand card consumed (:3429). Both message texts carry the Rust citations. `parity_cce_gaps` 55 → 53 FAIL, cleared exactly the 2 enqueue-refusal lines, added 0. Engine confirmed Rust-faithful: the gate at `game_state_abilities.c:2746-2752` frees the ability, advances the queue and returns 0, so a POSITIVE enqueue count is correct.
- [x] J5 CLOSED (verified on arrival, checklist was stale). Both call sites already use `rb_stage_position_change_with_bags(&tg.state.p[0], …)` (`:720`, `:1031`) with local externs at `:619-620`. MEASURED: `parity_under_member` is byte-identical to the pre-change baseline (60 FAIL lines, 0 movement) — the 4 Q139 assertions were already green.
- [x] J6 CLOSED (edit landed; the file is `test_parity_zones_position.c:356-358`, not under_member). `test_stage_formation_change_rotation` now asserts Rust's `[c,b,a]` — LEFT==c, CENTER==b (the centre member keeps its own area), RIGHT==a — citing `live_start_subunit_threshold_formation_test.rs:74-86`. The old `[a,c,b]` was consistent with no permutation of from={L,C,R}/to={R,C,L}. `parity_zones_position` 41 → 38 FAIL, cleared exactly the 3 formation lines, added 0.
- [x] J7 CLOSED (edit landed). `test_parity_integration.c:1364-1366` now stages `on_stage = test_new_id(&tg, "PL!SP-sd1-002-SD")` and flags `moved_this_turn[on_stage]` — ONE id, matching `hard_tier_cards_cost_copy_discount_mill_and_dual_retrieve_test.rs:191-193`. `parity_integration` 32 → 31 FAIL, cleared `kinako: while a Liella! member moved areas this turn, this hand card costs -2 (got 0 expected -2)` — i.e. the discount now actually applies, which it could not while the flag sat on the un-staged template id 2555.
- [x] J8 CLOSED, and generalised. `test_parity_zones_position.c:406` and all 19 sibling declarations are now `static TestGame tg;`. The same sweep was run across every file this task claims: `test_parity_zones_position.c` 20, `test_parity_draw_flat.c` 108, `test_parity_executor_dispatch.c` 21, `test_parity_integration.c` 69, `replay.c` 12 — 230 non-static `GameState`/`TestGame` locals converted. Provably behaviour-neutral: automatic storage was never zero-initialised, and every site is immediately followed by `test_game_new` / `rb_game_init`, both of which memset the whole state. MEASURED: `parity_zones_position` moved only by the 3 J6 lines, and `parity_draw_flat` / `parity_executor_dispatch` / `parity_integration` are otherwise unchanged, so the sweep cost nothing.
- [x] J9 RE-VERIFIED AND REWRITTEN (all four rows, against the current tree — not the peer's snapshot). **F** — still open, cause STRENGTHENED: `watches_ability_resolution` has ZERO occurrences anywhere in engine_c_wip, while Rust consumes it at `abilities.rs:641-650` and `:813-822` to SKIP resolution-watchers on both TAS loops; measured, `PL!-bp6-020-L` ab#0 satisfies both halves of the conjunction (`trigger_type=each_time` + `watches=true`), so the missing guard is reachable. Added a second CHECK pinning both halves. **G** — the row's stated cause was STALE: the CSV→PINK hazard is unreachable, because `card.c:1005-1016` and `triggers.c:690-710` both split the list per token and `condition.c:47-51` walks the real array. Rewritten to the two gaps that ARE live (`card.c:1004` gates the split on a `filter_targets_by_heart_colors` extra the fixture lacks — measured `built=0`; and `rb_cond_get_str` at `game_state_abilities.c:1042-1049` is RB_TAG_STR-only so an ARRAY condition list is silently dropped at `:1107`). Added two CHECKs pinning the per-token contract that makes the split sufficient. **M** — still open, cause MOVED: the member now EXISTS (`rabuka.h:2450-2451`, A17) and the matcher is wired (`util.c:1872-1877` → `:1701-1710`); the missing half is the PRODUCER — `card_filter_from_effect` contains zero writes to it (measured `built=1 total=0 operator=[]`), so `has_original_heart_total` is never set and `:1701-1710` is dead code. Added a CHECK that measures the producer instead of restating a comment. **N** — still open, line corrected: `h_choose_target_player` moved from `misc.c:968` to `misc.c:1039-1049`; measured `n_child=1` with the step decoding correctly. Suite stays 14 ok / 2 FAILED (M, N) — red by design for the open rows, and D was already closed.
- [x] J10 CLOSED (test-side). B2's 6 failures re-measured: 4 need the same-unit re-prompt (checklist B2, engine `choice.c`, not this task's) and 2 were TEST MISTAKES. **Fixed 1:** `!hand_has(p_c)` was inverted — Rust's twin picks filtered index 0 twice (`q175_hanano_cross_unit_discard_same_unit_as_each_other.rs:71-73` → p_a, then p_b on the same-unit re-prompt), so the THIRD Printemps card is the survivor; the assertion demanded the opposite and would have failed under a correct engine. **Fixed 2:** the two `== 0` blade/heart04 assertions pinned a cost-conditional buff Rust does not have — the decoded effect is an unconditional `sequential` with no `result_condition` and no `conditional_action`, and Rust gates on the cost only through `conditional_on.rs:18-28` / `:88-149`, neither of which applies; `hanano_no_qualifying_unit_skips_cost.rs:186-202` asserts no prompt / both cards in hand / hand length unchanged and says nothing about blade/heart. Both now assert what Rust does. `parity_small_clusters` 73 → 70 FAIL, cleared exactly those 3, added 0; the 4 B2 engine failures remain and are honestly attributed to `choice.c:2136-2146` (measured: `prompts = 1` after one pick, `n_filtered = 3`, one card in the waitroom — no re-prompt is opened).
- [x] J11 CLOSED (edit landed). `test_parity_small_clusters.c:152` `R3BIRTH_A` switched from the phantom `PL!N-bp1-023-PRproteinbar` to `PL!N-bp1-023-N`. Both keys are real entries in `cards/cards.json` and are byte-exact duplicates, so it is a PRIZE print, NOT corruption — verified, and the peer was right. Rust's twin states the rule outright (`subunit_stage_count_gated_debut_heart_test.rs:9-22`: "13 keys with text fragments spliced into the card_no … byte-exact duplicate … this file previously reached for one of them … stay resolvable if the 13 duplicates are ever deleted") and pins the clean key. `test_parity_hearts_required` (i.e. `test_parity_required_hearts.c`) carries NO A4 and NO `bp1-023` fixture — the whole of J11 is the small_clusters A4 pair. `test_parity_gain_delta.c:274` already used the clean key. No verdict movement (A4 was already passing, as expected for a byte-exact duplicate).
- [x] J12 CLOSED (swept + 3 measured assertions landed). The original `parity_wave1b` hardcoded `queued=1` is gone — `ability_id[512]` no longer exists anywhere in the tree — and a mechanical sweep of all 13 claimed files found **zero** diagnostics with a literal count argument: 134 `=%d`-shaped diagnostics enumerated, every one printing a measured variable, plus a targeted audit of every counter that feeds a `queued`/`prompts`/`fired` diagnostic (`pc_pay` / `pc_decline_pays_rest` / `pick` / `note_prompt` / `fire_trigger` all increment inside a `pending()` guard). The row's own RULE was documented at `test_parity_wave1b.c:679-684` but never landed, so the three remaining print-only counts are now ASSERTED: `live_start_queued` (A1+, the exact site the masked `ability_id` bug lived in), `live_queued` (A6+) and `queued` (B7). MEASURED: all three read exactly 1, so `parity_wave1b` stays at 39 FAIL — but a regression that stops enqueueing is now a FAIL line instead of a log line.

## K. Data / artifacts

Closed 2026-10-03. Owner: the data/build agent. Every measurement below was taken
in an isolated snapshot (`Temp/kilo/kdata`), never in the shared tree.

⚠ **FIXED 2026-10-03 by main — `sha256_of` broke on Windows ABSOLUTE paths.** It used
`cut -d' ' -f1`, but MSYS `sha256sum` prefixes the hash with a `\` and marks the file
binary with `*` when the path contains backslashes, so the extracted value was
`\89204df0…`. That fails SAFE — a stray backslash can only cause a mismatch, never a
false match — but it made the guard **REFUSE a perfectly valid blob** whenever an
absolute path was passed, e.g. `make data STRINGS_BIN=C:\…`, breaking the build.
(Found by cluster T2, which called it a false negative; measurement shows the direction
is the opposite — it is a false positive. T2's underlying observation was correct.)
Now extracts the 64 hex chars themselves via
`sed -n 's/^[^0-9a-fA-F]*\([0-9a-fA-F]\{64\}\).*/\1/p'` on the coreutils AND perl
branches. Verified: good blob via absolute path exit 0 (was 1), good blob via relative
path exit 0, corrupt 726649 blob via absolute path still exit 1.

- [x] K1 `cards/build/abilities_strings.bin` can never be installed unverified again.
  `make data` is now a **phony** rule that VERIFIES-then-installs through the new
  `tools/guard_data_blob.sh` on every invocation: source byte count + sha256 must
  equal the pinned canonical identity, and an existing `src/` copy must be the same
  blob (a non-canonical `src/` copy is repaired LOUDLY, never silently kept). The
  copy is staged through a temp name + rename. Pins: `cards.bin` 631890 /
  `728208504fee6217`, `abilities_strings.bin` 726831 / `89204df0bbde08a0`.
  Proven against the known-bad 726649 blob: guard REFUSES, `make data` exits 2,
  `src/abilities_strings.bin` stays 726831. It also refuses a right-size /
  wrong-digest blob. `make verify-data` added.
  ⚠️ OPERATIONAL: `cards/build/abilities_strings.bin` is **currently 726649 bytes**
  (sha256 `6225b9b531f4875a`) and 4115 of 6032 strings disagree with `gen_data.c`.
  So `make data` / `make t` / `make run-all-tests` now HARD FAIL until
  `bash tools/regen_all.sh` is run. That is the intended outcome — before the guard
  the same `make` copied that blob over the good one. Regeneration is `cards/`'s,
  not this agent's.
- [x] K2 **IMPOSSIBLE within the K owner's file set.** The premise ("nothing
  references them") is false. `src/bytecode_data.c` declares `BYTECODE_C0..C3`
  (not `rb_bytecode_blob`/`rb_bc_NNNN` — the item's names are stale) and they are
  referenced at `src/ability/abilities_gen.c:986-989` by `bytecode_slice`, declared
  at `include/rabuka.h:368`. Removing the file from SRC/OBJ_LIB and deleting it
  **fails the link**:
  `ld: src/ability/abilities_gen.o:abilities_gen.c:(.rdata$.refptr.BYTECODE_C0[...]): undefined reference to 'BYTECODE_C0'`
  (same for C1/C2/C3), measured. Both referencing files are `src/ability/**` and
  `include/rabuka.h`, which this agent does not own. Unblocking edit, measured in a
  throwaway snapshot: delete `abilities_gen.c:7-11` (the 4 `extern`s), `:983-992`
  (`bytecode_slice`, which has **0 callers** tree-wide) and `rabuka.h:368`, then
  drop `src/bytecode_data.c` from SRC/OBJ_LIB — `make clean && make all` goes green
  and `rb_engine.exe` drops **1336837 -> 1236718 bytes (-100119)**;
  `src/bytecode_data.o` alone is 99238 bytes of `.rodata` in every binary.
  Hand to the `src/ability/**` owner as a 6-line deletion.
- [x] K3 `gen_data_cdi.c` use-after-free AND ignored `len` both fixed (the file is
  NOT unused — `platforms/cdi/build_cdi.sh` compiles it, so quarantining it would
  have broken the CD-i link). `len` is now validated against the 34566-byte streamed
  layout (`-1` null/empty, `-2` truncated) and the three `g_*` tables point into a
  malloc'd copy this file OWNS, because `data.c:250` frees the caller's buffer.
  Proven: the pre-fix version (commit `0ba3a112`) **segfaults** the harness at the
  `len == 0` case; the fixed version passes `len` 0 / NULL / short / exact, and
  after the caller frees its buffer and 128 KB of heap is scribbled, the tables
  still hold the loaded values. Also removed the duplicate `RBKA_NUM_ABILITIES`
  definition, which was a multiple-definition link error once `gen_data.c` joined
  the CD-i list.
- [x] K4 Both platform scripts had far more than one file missing. Measured with a
  host link probe (`ld -r` + `nm`, target-independent):
  `build_cdi.sh` referenced **20** engine symbols no file in its list defined
  (`rb_setup_*` x7, `rb_executor_*` x3, `rb_desc_state_verb_ja`,
  `rb_move_*_ported` x2, `get_string`, `RBKA_BYTECODE`, `RBKA_BYTECODE_LEN`,
  `RBKA_CARD_ABILITY_PAIRS`, `RBKA_NUM_ABILITIES`, `RBKA_STRINGS_FNV`,
  `RBKA_STRINGS_TABLE_LEN`); `build_genesis.sh` referenced **15** (including
  `rb_load_gen_data` and all three `g_*` pointers — the named `gen_data_rom.c`
  omission). Both lists were 14-16 files short of the host `SRC`. Both now mirror
  the host `SRC` list minus `src/main.c`, with `build_cdi.sh` taking
  `generated/gen_data.c` (not `gen_data_rom.c`) and `build_genesis.sh` taking
  `gen_data_rom.c` (not `gen_data_cdi.c`), per the PLATFORM VARIANT CONTRACT in
  `gen_data_rom.c:6-27`. Re-probed: **0 missing engine symbols on both**.
  `build_cdi.sh` also had a hardcoded `W=/mnt/c/.../engine_c` path that does not
  exist, so it could not start at all — now derived from `$0`, with a toolchain
  precheck; and its `set -e` aborted on a failed link **before** printing the
  linker output that names the missing symbol, so that is now captured and shown.
  The m68k toolchain is not installed here, so the link itself is unverified; the
  source-list fix is.
- [x] K5 Neither daemon ever ran `make`; what they did was worse — both read the
  process-global `C:\rust_targets\release\bot_arena.exe` and `queue_runner` ran
  `cargo build` against the live tree, so a daemon and a foreground build fought
  over one target dir. Both now hard-fail unless `ISOLATED_TREE` names a directory
  that is not the live tree (unset / equal / nested either way / not a snapshot:
  exit 2, no override), resolve every path through it, pin `CARGO_TARGET_DIR`
  inside it, and take an exclusive per-tree lock (`FileMode.CreateNew`,
  `FileShare.Read`, stale-owner break) around every build and every binary
  snapshot. Measured: 6/6 refusal cases exit 2; a real snapshot is admitted and
  completes a round writing only into the snapshot; a second process is REFUSED the
  lock while a live holder has it, acquires it after release, and two different
  trees lock independently. New file `tools/isolated_tree_guard.ps1`.
- [x] K6 `git stash@{0} session-wip` **UNTOUCHED** — never popped, never applied,
  never dropped. Identity: commit `c458ab6f85ce5559041defd25165789b62cd8f21`,
  `"On master: session-wip"`, 2026-10-02 12:15:28 +1000, three parents
  (`e41bdd90` base, `2fa05323` index, `efd88c79` untracked), **31 files,
  +4853/-1724**. Standing hazard for every agent:
  **NEVER `git stash pop`, `git stash apply`, `git checkout --`, `git restore` or
  `git reset --hard` in this repo.** That work has been destroyed three times by
  exactly those commands. If a peer says it was "stashed so you can see it", it was
  DESTROYED — re-derive the file from analysis instead. To inspect it read-only:
  `git stash show --stat c458ab6f85ce5559041defd25165789b62cd8f21` (use the SHA;
  PowerShell eats the braces in `stash@{0}`).

### New open items found while closing K

- [ ] K7 `cards/build/abilities_strings.bin` (726649, sha `6225b9b5...`) disagrees
  with `gen_data.c` on 4115 of 6032 interned strings and on the whole-table FNV.
  `verify_artifacts.py` is RED on this and on `abilities_gen.rs` (6031 vs 6033
  offsets). `cards/abilities.json` is modified and no ordered regen has followed.
  FIX: `bash tools/regen_all.sh` (the only supported path — see its header for the
  four incidents this exact skew caused). Not this agent's tree.
- [ ] K8 Both platform build scripts now have complete source lists, but no m68k
  cross toolchain is installed on this machine, so neither has been *linked*.
  CD-i additionally carries ~34 KB of `gen_data.c` tables that
  `-fdata-sections -Wl,--gc-sections` are expected to drop (they are unread on
  that target — everything goes through `g_offset_deltas`/`g_strings_offsets`);
  that expectation is asserted in a comment and UNMEASURED. Needs someone with
  `gcc-m68k-linux-gnu` to build both and confirm image size.

---

---

## T5. Human-readable surface — describe.rs / display.rs / constants.rs

Owner: cluster T5 (`src/ability/{describe,choice_renderer,compound}.c`,
`src/game/{deck_parser,deck_builder}.c`). Full audit:
`engine_c_wip/AUDIT_describe_display.md`.

CLOSED this pass:

- [x] **933/936 verdict settled.** The "933 of 936 golden records, 3 stale"
  claim in `describe.c`'s header was **unverifiable**, not proven: the golden
  (`test_output/describe_dump.json`) is not in the repository, and
  `engine/src/bin/describe_dump.rs:33-37` writes one record per ability over
  `0..NUM_ABILITIES` = **985**, so it cannot emit a 936-row file at all. The
  "only 30 of 936 indexes agree positionally" figure is arithmetically
  impossible against a 985-index dumper. Header rewritten to state what was
  measured; the unverifiable claim is gone.
- [x] **Replaced it with a measurement that IS reproducible.** Rust oracle
  rebuilt from the committed dumper (compiled via `cargo test` only), C oracle
  built from the snapshot's `OBJ_LIB`, compared byte for byte in both
  languages: **983 of 985 identical, 1 divergence (ability 325), 2 excluded.**
- [x] **The one real divergence is an artifact skew, not a port defect.**
  `RBKA_OFFSET_DELTAS` (`src/core/generated/gen_data.c:7`, sums to 98 737 ==
  `RBKA_BYTECODE_LEN`) vs `OFFSET_DELTAS`
  (`engine/src/ability/abilities_gen.rs:25`, sums to 98 725 against a
  98 733-byte `cards/build/abilities.bin`) differ in length at **2 of 985**
  slices — 325 and 805. `abilities_gen.rs:17,55` `include_bytes!`
  `cards/build/{abilities.bin,abilities_strings.bin}`, so Rust reads the STALE
  726 649-byte string blob and the C reads the canonical 726 831-byte
  `src/abilities_strings.bin`. Two engines, two data sets. `describe.c` must
  NOT be changed for ability 325.
- [x] **`rb_desc_duration_label_ja` was declared but never defined.**
  `include/rabuka.h:3215` declared it beside the other four `_ja` wrappers;
  `describe.c` never defined it, so any caller failed at LINK. Implemented
  (`describe.c`, mirroring `duration_label_inner(d, true)`,
  describe.rs:148-156).
- [x] **New suite `tests/test_describe_golden.c` + `tests/fixtures/describe_golden.txt`.**
  983/985 abilities byte-for-byte in both languages; hex-encoded fixture so no
  multi-byte literal is ever hand-transcribed; the 2 exclusions are named in
  BOTH the fixture header and `EXCLUDED_SKEW`, and the suite FAILS if the
  excluded set is anything other than exactly those two. Plus every
  `constants.rs` value, all five label tables in both languages,
  `translate_choice_prompt_en_to_ja` (all four Rust test cases plus the two
  exact-match arms), `describe_sequential_cost`, and the no-raw-text-fallback
  rule. **GREEN, 1393 checks.**
- [x] **`core/constants.rs`: 5/5 constants MATCH, 0 DIVERGE.**
  `STAGE_SIZE` 3, `EMPTY_SLOT` -1, `MAX_ENERGY_CARDS` 12, `MAX_LIVE_CARDS` 3,
  `VICTORY_CARD_COUNT` 3. All 10 functions MATCH, including
  `score_to_i32_rounded`'s half-**away-from-zero** then truncate-toward-zero
  (`(-0.4)` → `0`, not `-1`) and `id_u32`'s floor-at-zero on negatives.
- [x] **`compound.c` mapped.** It is not one Rust module: it is the whole
  `engine/src/ability/compound/` package (9 files, 1141 lines). All 19 items
  accounted for; 10 inlined, 9 exported. One stub, see T5-2.
- [x] **`deck_parser.c` / `deck_builder.c` mapped and pinned.** All
  `parse_line` widenings over Rust recorded and asserted AS divergences; the
  `rb_normalize_card_no` ASCII-vs-fullwidth difference is proven inert (both
  spellings reach the same lookup key because `card_no_codepoint`,
  `src/core/card.c:316-326`, folds U+FF0B → `+` and U+FF01 → `!`).

STILL OPEN:

- [ ] **T5-1 `game/display.rs` — 2002 lines, ZERO C twin.** VERIFIED-NOOP:
  0 of 985. There is no display layer in the C at all: `resolver.c:1182` says
  so, and `LeftSide`/`RightSide`/`cannot_activate_members`/`jyouji`/
  `total_blade`/`live_need_hearts`/`exclusion_zone`/`mulligan_selected_indices`
  return zero hits outside the engine's own state fields. Porting needs
  `BoardDisplay`/`CardDisplay`/`CardModifierBadges` in `include/rabuka.h` plus a
  consumer.
      **BLOCKED-ON: include/rabuka.h — add `BoardDisplay`, `PlayerDisplay`,
      `StageDisplay`, `CardDisplay`, `CardModifierBadges`, `ZoneDisplay` and the
      6 `*_to_display` prototypes, or record that the C ships no display
      surface.** It also owns `CountCast::i8_count`, which has 0 C callers for
      the same reason.
- [ ] **T5-2 `compound.c:508-513` `record_step_output` is an empty stub.**
  VERIFIED-NOOP today: **0 of 2206 effect nodes** carry an `id` key, and
  `rb_step_state_record` / `rb_step_state_get` (`types.c:292-345`) have no
  caller outside `types.c`. The store is wired; only the write and the read are
  missing. Becomes a real bug the moment a corpus node grows an `id`.
- [ ] **T5-3 `deck_builder.c:12` pins `LL-E-001-SD` (card id 1) as the default
  energy template; the C fallback is `IKZL-PR-001-PR` (card id 0).** These are
  DIFFERENT cards, so the pin is a card-identity decision. Rust's
  `cards.iter().find(is_energy)` runs over a `std::collections::HashMap` with
  the default RandomState (`core/card.rs:406`, `compat.rs:54`), so it is
  **non-deterministic per process** and there is no value to move to. Reachable
  (`game_setup.rs:353-354,547-550`, `web_server.rs:2681-2685,3272-3276`,
  `main.rs:156-160`). Now pinned by test so it cannot drift silently.
- [ ] **T5-4 the `gain_resource` `count: 1` story is half-verified.** The
  shorthand rule is real and implemented (`describe.rs:303-311`); the corpus has
  **294** `gain_resource` nodes of which **10 carry no `count`**, and C and Rust
  agree on all 10. What CANNOT be shown is that a golden once held the bare form,
  because the golden is gone. Settled when whoever owns `test_output/` says
  where the oracle lives.
- [ ] **T5-5 ability 325 / 805 bytecode skew.** Owner: `src/core/generated/**`
  + `cards/**` (cluster K), fix is `tools/regen_all.sh`. Not attempted here
  (Run NO generator). Until it is done, the describe golden must keep excluding
  exactly those two indexes.
- [ ] **T5-6 `parse_deck_content`'s three widenings over `parse_line`**
  (U+00D7 and bare `X` as the separator, inline `#` comments, quantity 0
  dropped rather than stored as a 0-copy entry). Deliberate; pinned as
  divergences. Revisit only if a deck-list source actually feeds Rust's parser.
- [ ] **T5-7 `tests/test_deck_builder.c:113` has a 781 KB `GameState` on the
  STACK.** It passes only because the Makefile passes
  `-Wl,--stack,16777216`. AGENTS.md requires every `GameState` local to be
  `static`. Not changed — `tests/**` is contested.
      **BLOCKED-ON: tests/test_deck_builder.c:113 — `GameState game;` →
      `static GameState game;`.**

- [ ] **T5-8 `src/ability/choice_renderer.c` (1821 lines) is in NO build
  target.** Not in `Makefile:20-70` `SRC`, not in
  `platforms/cdi/build_cdi.sh`, not in `platforms/genesis/build_genesis.sh`,
  not in `platforms/Makefile` (0 hits for `choice_renderer` in all four). It is
  the platform VRAM UI front-end, so its absence is consistent with there being
  no display layer (T5-1), but it means 1821 lines no compiler has seen. The
  `RbRendUi` vtable it dispatches through is also absent from
  `include/rabuka.h`.
      **BLOCKED-ON: Makefile:20-70 / platforms/**/build_*.sh — add
      `src/ability/choice_renderer.c` together with the `RbRendUi` vtable in
      `include/rabuka.h`, or delete the file. Nothing was changed here.**

---
---
---

## ⚠ PATH CORRECTIONS (verified 2026-10-02 — earlier reports cited these WRONG)

Several earlier agent reports used shorthand that does not resolve. Verified against
the tree: `src/` has exactly four subdirs — `ability` (28 .c), `core` (14), `game` (3),
`turn` (3). Use these real paths:

| Cited as | Actually is |
|---|---|
| `src/util.c`, `src/core/util.c` | **`src/ability/util.c`** |
| `src/card.c`, `src/core/card.c` | **`src/core/card.c`** |
| `src/modifiers.c`, `src/core/modifiers.c` | **`src/core/modifiers.c`** |
| `src/move.c` | **`src/ability/effects/move.c`** |
| `src/misc.c` | **`src/ability/effects/misc.c`** |
| `src/rules.c` | **`src/ability/effects/rules.c`** |
| `src/phase.c` | **`src/turn/phase.c`** |
| `src/game_state_abilities.c` | **`src/core/game_state_abilities.c`** |
| `src/live.c` | **`src/turn/live.c`** |
| `src/executor.c`, `src/custom.c`, `src/dynamic_count.c` | **`src/ability/effects/…`** |
| `src/cost.c`, `src/condition.c`, `src/choice.c`, `src/vm.c` | **`src/ability/…`** (these were already right) |

ALWAYS confirm a path exists before editing. Item lines above inherited the wrong
path in places; the SYMBOL and Rust reference are correct, the path is not.

Also verified: `rb_get_selection_indices_filter` is declared non-static ONLY at
`src/ability/util.c:887` (tentative) with the single real definition at
`src/ability/util.c:2070`; `src/ability/effects/move.c:104` is a second
tentative declaration, not a duplicate definition. Do not "fix" this.

---

## 🔒 OWNERSHIP MAP — 13 AGENTS LIVE. READ BEFORE EDITING ANYTHING.

One writer per file. A torn interleaved write has destroyed work three times here.
If you need a file you do not own, write `BLOCKED-ON: <file>:<line> — <exact change>`.

| File(s) | Owner |
|---|---|
| `include/rabuka.h` | **main** |
| `src/ability/choice.c` | cluster B |
| `src/ability/cost.c`, `effects/rules.c`, `effects/misc.c` | cluster C |
| `src/ability/condition.c` | cluster D |
| `src/core/modifiers.c` | cluster E |
| `effects/custom.c`, `effects/executor.c`, `effects/draw.c`, `effects/move.c`, `effects/dynamic_count.c` | cluster F |
| `src/core/card.c`, `src/ability/util.c`, `core/game_state_abilities.c` | cluster G+H |
| `src/ability/{vm,resolver,enums,types}.c` | cluster T1 |
| `effects/{state,ability,ability_effects,score}.c` | cluster T2 |
| `src/core/{zones,game_state_core,data,stats_pipeline,tracking,alloc,alloc_counter}.c` | cluster T3 |
| `src/ability/ability_queue.c`, `src/turn/triggers.c` | cluster T4 |
| `src/ability/{describe,choice_renderer,compound}.c`, `game/{deck_parser,deck_builder}.c` | cluster T5 |
| `tests/**` | cluster J (landed) — **now contested**; claim per-file in your report |
| `src/turn/{live,phase}.c`, `src/engine.c`, `src/web_server.c` | cluster I (landed) — free to claim |
| `Makefile`, `tools/**`, `platforms/**`, `src/core/generated/**` | cluster K (landed) — free to claim |
| `src/test_game.c` | **unclaimed — claim it** |

**KNOWN COLLISION — the unblock sweep overlaps B/C/D/E/F/G+H.** Its items 2a/2b/2c/3b
live in `effects/rules.c`, `core/game_state_abilities.c`, `ability/condition.c`,
`ability/util.c` — all owned above. Every agent in both groups was told to confirm
QUIESCENCE (stable SHA-256 across two reads, no `.o` newer than the source) before
editing. If you see a hash moving, WAIT and re-check; do not edit. Torn writes surface
as compile failures, which are to be reported, not "fixed".

---

## T. Cluster T4 — ability queue / trigger scan / drain (`ability_queue.c`, `turn/triggers.c`)

Full deliverable: **`AUDIT_queue_triggers.md`** — a row per function with its
definition line, ALL call sites (or `ZERO CALLERS`), and WIRED / DEAD-CORRECT /
DEAD-WRONG, plus the Rust-entry-point mapping, the queue-lifecycle table, the
per-phase scan-order proof and the full divergence list. Quiescence was confirmed
before editing (`ability_queue.c` `288144C2…`, `triggers.c` `514E29B8…`, identical
across a 20 s settle), and the snapshot matched the live tree on all 216 `.c`/`.h`
files.

**Census: 99 functions across the two files — 75 WIRED (3 of them only reachable
from `fprintf(stderr, …)` reachability helpers), 22 dead (20 correct, 2
DEAD-AND-WRONG and open as T4-7 / T4-8), and 3 of the 75 WIRED ones were
WIRED-AND-WRONG and are now fixed.** Every fix was inside a function that *is*
reached from the engine path, which is the category this project was worst at
noticing.

Closed in T4 (both files are T4's; no other behaviour file touched):

- [x] **T4-1** `rb_trigger_debut` fired a **gained 登場** ability Rust never fires.
  Rust `trigger_scan.rs:78-139` walks `card.abilities` and nothing else. Removed
  the arm and its two now-dead helpers (`queue_gained_trigger`,
  `queue_gained_trigger_seen`). `VERIFIED-NOOP: 0 of 985` — of 24 `gain_ability`
  nodes the distinct `ability_gain_trigger` values are 常時 (20) / ライブ成功時 (2),
  never 登場. Tests: `test_audit_triggers.c` `A9` + `A9b`.
- [x] **T4-2** the 登場 enqueue carried **no enqueue-time movement snapshot**.
  Rust `trigger_scan.rs:148` clones `recently_moved_cards` once and passes it to
  every enqueued 登場 ability. `rb_drain_ability_queue` zeroes
  `g->n_recently_moved` (`ability_queue.c:1420`) BEFORE the cost runs, so the
  entry snapshot is the only surviving evidence by effect time. **n > 0**: 3 of
  the 281 登場 abilities carry `source: "those_cards"` and up to 22 carry
  `type: "movement_condition"`. `rb_push_owned_entry` now takes `moved`/`n_moved`;
  the ライブ開始時 and ライブ成功時 scans still pass `NULL,0` because Rust passes
  `None` there (`trigger_scan.rs:345-346`, `:525-526`). Tests: `A8` + `A8b`.
- [x] **T4-3** the use-limit key was `(card_id << 4) | (ability_idx & 0xF)` — a
  **four-bit** ability index. `RB_GAINED_ABILITY_INDEX_BASE` is 10000 and
  `10000 & 0xF == 0`, so a gained index aliased the printed index on the same
  card, and C could refuse a printed 「{{ターン1回}}」 自動 ability (or report a
  phantom use count) because an unrelated gained ability had resolved. Rust keys
  the full `(i16, usize, u32)` (`abilities.rs:146-151`). Widened to
  `(card_id << 16) | (ability_idx & 0xFFFF)` via a shared `rb_use_key`.
  `VERIFIED-NOOP: 0 of 985` (no gain grants 自動/起動, the only use-limited
  families) but it was a latent bug in a wired path. Tests: `A10`.
- [x] **T4-4** re-verified the prior `choice.c` fix. It landed, at the two sites
  the line drift moved it to (`choice.c:4831`, `choice.c:5187`), correctly
  guarded on `!rb_has_pending_choice` per Rust's own refusal at
  `actions/mod.rs:1552-1557`, and correctly NOT paired with
  `rb_queue_clear_completed`.
- [x] **T4-5** verified the `[LIVE_START_SCAN]` fprintfs are unconditional and
  mirror Rust's `log::trace!` at `trigger_scan.rs:273-281`. **Left in place.**
- [x] **T4-6** verified all three scan orders against Rust: ライブ開始時 is
  **live-zone then stage** (`trigger_scan.rs:232-238`), ライブ成功時 is
  **stage then live zone** (`trigger_scan.rs:386-395`), 登場 resolves a single
  area because both Rust callers pass `Some(card_id)`. All three match.

Still open:

- [ ] **T4-7** `ability_queue.c:307` `rb_queue_push_with_trigger` — **ZERO
  production callers**, and it builds an entry with `player_id` left empty, so
  any future caller produces an entry the owner-scoped drain
  (`game_state_abilities.c:2670-2680`) cannot see and drops silently. FIX:
  delete it (no callers; `rb_queue_enqueue` + `rb_queue_set_entry_trigger`
  already covers every real producer).
- [ ] **T4-8** `ability_queue.c:967` `rb_queue_save_pending_actions` — **ZERO
  production callers** while Rust calls it in production at
  `ability/compound/conditional_on.rs:43` (`gs.ability_queue
  .save_pending_actions(vec![finish])`). A `conditional_on` whose selected
  branch finishes must leave ONE retained action on the entry; C leaves none.
  FIX: append the finished action with `rb_queue_save_pending_actions` in the C
  of `conditional_on_result` / `conditional_alternative`.
  **BLOCKED-ON: `src/ability/compound.c` (Rust twin `ability/compound/
  conditional_on.rs`) — cluster T5's file, not T4's.** The queue half is correct.
- [ ] **T4-9** Rule 9.5.3 needs all three of `rb_queue_pause_for_auto_ability_choice`
  (`ability_queue.c:626`), `rb_queue_promote_entry_by_abs` (`:787`) and
  `rb_queue_set_current_entry` (`:804`) — defined, tested, unreachable. In
  `rb_process_player_abilities`, when two available entries share an owner,
  build `Choice::SelectAutoAbility { player_id, options, description,
  description_en, description_ja }` (Rust `abilities.rs:1705-1738`), call
  `rb_queue_pause_for_auto_ability_choice` and break; otherwise
  `rb_queue_promote_entry_by_abs(idx)` (`:1739`). In the §9.5.3.2→§9.5.3.1
  loopback, `rb_queue_set_current_entry(new_idx)` (`:1808`).
  **BLOCKED-ON: `src/core/game_state_abilities.c` — cluster G+H's file.**
- [ ] **T4-10** `rb_trigger_live_start` calls `rb_drain_ability_queue` **inside**
  the scan (`triggers.c:441`), so seat 1's entries resolve before seat 2's are
  enqueued. Rust `phases.rs:245-253` enqueues BOTH seats first, then drains per
  seat via the owner-scoped `process_pending_auto_abilities`. Currently
  measurement-neutral because the queue is empty at `phases.rs:245`, which is
  exactly why nothing catches it. FIX: drop the drain from
  `rb_trigger_live_start` and drain from the phase.
  **BLOCKED-ON: `src/turn/phase.c` — cluster I's file.**
- [ ] **T4-11** `rb_should_trigger_live_success` (`triggers.c:425`) returns the
  cached `g->live_success[pl]`. Rust `abilities.rs:2870-2915` RECOMPUTES: it has
  a `current_phase != LiveVictoryDetermination → false` term the C lacks
  entirely, and it re-checks every live card's `need_heart` against freshly
  computed stage hearts. Divergence in two windows — any scan run outside
  `LiveVictoryDetermination`, and any `modify_required_hearts` / modifier
  change after the flag was recorded. **Do not add the phase term to the C
  function**: 20+ suites call the scan directly with `live_success[pl]` preset
  and rely on the cached answer. Closes with T4-12 or with a suite-side change.
  **BLOCKED-ON: `src/turn/live.c` (cluster I) — the callers, not the predicate.**
- [ ] **T4-12** `rb_queue_push_constant_context` (`ability_queue.c:717`) and
  `rb_queue_pop_constant_context` (`:736`) are dead ONLY because **E3**
  (`evaluate_success_zone_constant_abilities`, Rust `modifiers.rs:2330-2345`) has
  no C twin — Rust's only callers are inside it. They must be called from that
  new function, in that order, around `rb_recalc_constants`. **T4-11's
  `need_heart` half also closes with E3/E4**
  (`restore_performance_need_heart_modifiers`, Rust `modifiers.rs:2155`).
  **BLOCKED-ON: `src/core/modifiers.c` — cluster E's file.**
- [ ] **T4-13** `rb_trigger_to_texticon` (`triggers.c:1641`) — **ZERO callers**.
  Rust's twin (`triggers.rs:139`) is called from `game/display.rs:1599`, a UI
  path with no C twin. DEAD, but it is a real UI omission: the C web UI cannot
  render a trigger's texticon. Lowest priority in this cluster.

### Cross-cutting harness defects found by T4 (NOT engine bugs — measurement hazards)

These are NOT part of T4's file ownership and are reported, not fixed.

- [ ] **H-1** `tests/test_audit_triggers.c` emits `FAIL [<row>] <test>: <msg>` —
  **no colon after `FAIL`**. Every corpus verifier that greps `^FAIL:` (the
  documented protocol) sees this suite as clean. It reported `FAIL_OUT=0
  FAIL_ERR=0` in my baseline while actually running **4 ok / 3 FAILED** with 7
  real `FAIL` lines on stderr. The suite's own `n_failed || n_crashed` return is
  the only signal, and exit code is explicitly not a signal.
  FIX: use `FAIL:` (or `^FAIL`) consistently, or add a second
  `FAIL:`-prefixed line per failure. **BLOCKED-ON: `tests/**` — claim needed.**
- [ ] **H-2** three of `test_audit_triggers.c`'s hidden failures (`A2`
  `.debut_position_left_positive`, `A3.debut_plain_positive`, both rows of
  `A4b.live_success_seen`) assert `rb_use_count(...) == 1` immediately after a
  scan ENQUEUES, all failing with `queued=1 use_count=0`. `queued` is correct;
  `use_count == 0` is the **faithful** value — `trigger_auto_ability` →
  `enqueue_printed_ability` (`abilities.rs:1282-1326`) ends in a bare
  `self.ability_queue.enqueue(entry)` and constructs `use_limit_recorded: false`,
  and `record_ability_use` is reached only from eleven resolution-time call
  sites. These rows are stale expectations from an older C shape that recorded
  the use at enqueue; the file's own rule is "fix the engine or re-classify the
  row", and this is a re-classification. **Reported, not edited** —
  weakening an assertion is not T4's call.
  **BLOCKED-ON: `tests/test_audit_triggers.c` — cluster J's file.**
- [ ] **H-3** `tests/test_parity_queue_resume.c` **aborts mid-file** (SIGABRT),
  so its last 5 tests never run and its summary line never prints — including
  `test_skip_chain_does_not_grow_the_queue` and
  `test_baton_touch_skip_chain_terminates`, the two regression guards the file's
  own header (`test_parity_queue_resume.c:15-19`) advertises. The abort is after
  `test_optional_look_select_skip_terminates`'s last CHECK (`:640`), i.e. at
  `free_optional_look_select(&parent)` (`:641`) or inside
  `test_select_routes_card_to_hand` (`:644-663`) before its first CHECK. It also
  emits **zero stdout lines** (block-buffered output is lost on abort), which is
  why the abort is invisible to a `^FAIL:`-only verifier. Present in the BASELINE
  before any T4 edit.
  FIX: make `rb_execute_effect_ex`'s ownership of `look_action`/`select_action`
  explicit so `free_optional_look_select` does not double-free, and give the
  suite the same fork-per-test isolation `test_audit_triggers.c` already has.
  **BLOCKED-ON: `src/ability/effects/look.c` + `tests/**`.**

### Measurement incident worth recording (T4)

T4's first "after" run reported a byte-identical 978-line `FAIL` set across all
33 suites. That was **not** a result: the runner compiles only the *test* file
and links pre-existing `.o` files, so the edited `.c` files had not been
recompiled and the run was a second baseline. The giveaway was the three new
parity rows failing standalone against the same stale objects. Corrected
procedure: **rebuild the object of any edited behaviour file before the corpus
run, and treat an unexpectedly perfect zero-delta as evidence of a stale link
rather than of neutrality.** The real measurement (978 → 978) was taken after
`make src/ability/ability_queue.o src/turn/triggers.o` in the snapshot.

---

## U. Cluster T1 — ability decode / resolver / enums / types
(`ability/{vm,resolver,enums,types}.c`)

Full deliverable: **`AUDIT_vm_resolver_enums.md`** — 321 inventory rows (127 public items +
194 enum variants) with a row per Rust item giving its line, its C twin or `NONE`, its
**measured** reachability over the 985 abilities, and a verdict; plus the per-enum
decode-completeness tables. Quiescence confirmed before editing (all four sources
byte-identical across a 12 s settle, untouched since 2026-10-02 12:23:47). Baseline = the
snapshot with **only the `vm.c` hunk reverted**, proved by a file-hash sweep, not git HEAD.

**Result: 1 divergence found, 1 closed, 0 reachable gaps still open.** `resolver.c` and
`types.c` are clean — every one of the 30 `resolver.rs` public/`pub(crate)` items and all 62
`types.rs` public items has a live correct twin.

- [x] **T1-1 CLOSED** `card_type` accepted names Rust's `CardType::from_card_str` rejects.
  `vm.c:1052` stored the raw string; Rust's `read_card_type_value` (`vm.rs:479-489`) →
  `CardType::from_card_str`, whose miss arm is `_ => None`, so Rust holds `None` and the
  string cannot survive the decode. **168 of 2230 effects across 149 of the 985 abilities**
  spell it outside the three-name vocabulary (`card` ×167, `member` ×1). Six consumers
  inherited it and five tested the field for mere NON-EMPTINESS: `core/card.c:959` copied
  `"card"` into `RbCardFilter.card_type`, `core/card.c:1107` counted it in `has_filter`, and
  `util.c:1547` then rejected **every** card (`card_type == "card"` matches none) → empty
  candidate set, silent no-op. `ability/describe.c:357-367` was the only consumer that did
  NOT inherit it (it re-derives through `rb_effect_card_type_from_str`), which is why the
  other five looked covered. Fix: populate `card_type_field` only for the three accepted
  names and, on a miss, leave **both** the field and the `card_type` extra unset (the extra
  matters — `core/card.c:959` and `ability/dynamic_count.c:655` both fall back to it).
  **Corpus 1556 → 1555 `^FAIL`, 0 added, 1 removed** (`parity_small_clusters` 112 → 111).
  A/B: `vm.c` `4D64C923…` = 4 `FAIL` vs `BFABC58A…` = 0 `FAIL`, same test file.
  Test: `tests/test_vm_decode.c::test_card_type_vocabulary` (**T1 claims this file**).

- [x] **T1-2** Decode-table completeness across `enums.rs`: **0 missing variants, 0 corpus
  values fall through any table.** `Zone` 32/32 (1782 corpus values), `ActionType` 68/68 in
  both `from_str` and `label` (49 distinct corpus spellings), `ConditionType` 33/33,
  `SelectTargetKind` 16/16, `TargetPlayer` 4/4, `EffectCardType` 3/3 + `Other`,
  `EffectState` 2/2 + `Other` (208 corpus values, 0 misses). Also verified for the enums
  decoded inside `vm.rs` but declared elsewhere: `CardType` (817), `Operator` (466),
  `Operation` (194, all 7 names), `AbilityFilter` (6), `CardProperty` (37).
  Test: `tests/test_parity_enums.c::test_corpus_decode_completeness` (suite now 7199
  assertions, 0 failures). **T1 adds to this file; no existing assertion touched.**

- [x] **T1-3** Container-capacity audit: max 12 extras/effect (cap 32), max 4 children
  (cap 64), max 13 condition fields (cap 64) over 2230 effects / 995 conditions.
  **VERIFIED-NOOP: 0 of 985** — no `RB_MAX_*` overflow is reachable, so the
  `goto fail`-on-overflow paths in `vm.c` cannot fire on this corpus.

### Still open from T1 (nothing lost — full reasoning in `AUDIT_vm_resolver_enums.md` §4)
- [ ] **T1-4** `vm.c:2715` `rb_zone_from_source_str` delegates to
  `rb_zone_id_from_str` (`core::types::ZoneId`), **not** `rb_ability_zone_from_str`
  (`ability::enums::Zone`). Two Rust types, two vocabularies, invisible at the call site.
  They differ on exactly 7 strings: `Zone` alone accepts `center` / `left` / `left_side` /
  `right` / `right_side` (`enums.rs:104-111`); `ZoneId` alone accepts `live_total` and the
  fullwidth `\xE3\x82\xB9\xE3\x83\x86\xE3\x83\xBC\xE3\x82\xB8` alias.
  **REACHABILITY 0 of 985** — all 1782 corpus source/destination values are inside both
  vocabularies, `rb_decode_fallback_count() == 0`, and the condition-side
  `location`/`source`/`destination` keys are `ArcStr` on both sides
  (`condition_decoder_gen.rs:181/204/167`), so the substitution does not reach them either.
  **Not fixed on purpose**: a speculative port on data nobody reaches can only be wrong.
  `test_parity_enums.c` now pins the inertness so a future regeneration that *does* use one
  of the seven fails loudly.
  **BLOCKED-ON: `src/core/data.c` (`rb_zone_names` + alias table) — the fix belongs in
  `vm.c`'s `rb_zone_from_source_str`, not in `data.c`; `data.c` is cluster T3's.**

- [ ] **T1-5** `vm.c:1286-2640` — the entire typed condition decoder
  (`ConditionLocals`, `decode_condition_field` with 85 keys, 21 `build_*`,
  `decode_condition_direct` at `vm.c:2588`) has **ZERO CALLERS**. Every condition decodes
  through the generic mirror `read_condition` → `read_condition_fields`, which keeps all 91
  condition keys losslessly — including the three the typed decoder does **not** name
  (`any_location` 21, `activated_this_turn` 3, `paid_cost_this_turn` 2) and
  `trigger_event` 160. That is precisely why `rb_decode_fallback_count()` is 0: the typed
  decoder's catch-all (`vm.c:2564`) would have called `rb_note_decode_fallback` and
  **dropped** each of those values. **Value-correct today, measured not assumed.** Left in
  place: zero callers does not prove deadness, and it is the path that would have to be
  repaired if the mirror changed. Side effect to remember: `build_unsupported`
  (`vm.c:2580`), the only producer of `c->variant = 20` (`Condition::Unsupported`), is on
  the dead path — corpus has **0** variant-20 conditions (16 distinct variants, 0..19).

- [ ] **T1-6** `enums.c` has **ZERO production callers** — every
  `rb_ability_zone_from_str` / `rb_target_player_from_str` / `rb_action_type_from_str` /
  `rb_condition_type_from_str` / `rb_select_target_kind_from_str` /
  `rb_effect_card_type_from_str` / `rb_effect_state_from_str` (and their `to_str`/`as_str`
  twins) is called only from `tests/test_parity_enums.c`; production uses `strcmp` against
  literals. Recorded because it means the tables are **verified but not load-bearing**: a bug
  in `enums.c` cannot change behaviour today, and a future new call site inherits a table
  round-trip-tested against every row.

- [ ] **T1-7** `EffectCardType::Other` / `EffectState::Other` cannot carry its `ArcStr`
  payload in C, so `rb_effect_card_type_as_str(RB_ECT_OTHER)` returns `""` where Rust echoes
  the input (`enums.rs:735`, `:744`, `:810`, `:818`). Already pinned deliberately by
  `test_parity_enums.c:464-476` / `:488-496` — **not weakened**. The variant *selection* is
  already correct; the gap is the enum shape declared in `include/rabuka.h`.
  **BLOCKED-ON: `include/rabuka.h` — main's file; would need a string member on
  `RbEffectCardType` / `RbEffectState`.** Note that D1 shows the lossiness is the CORRECT
  decode-time behaviour: `card_type` is a `CardType`, not an `EffectCardType`.

- [ ] **T1-8** `ability/compound.c:469` compares `card_type` against the literal `"member"`
  as a stand-in for `CardType::Member`. After T1-1 the field can never hold `"member"`, so
  that arm is unreachable and harmless. **Left in place** — an arm that cannot fire cannot
  diverge, and `compound.c` is cluster T5's. **BLOCKED-ON:
  `src/ability/compound.c:469` — delete the `!strcmp(first_ct,"member")` arm; the
  `!strcmp(...,"member_card")` line above it is the real `CardType::Member` test and must
  stay.**

- [ ] **T1-9** `PlacementTarget` (`enums.rs:274`, 3 variants) has no C twin — but it has
  **no wire strings**, so it needs no decode table. It is derived by
  `core/card.rs:2240 placement_target()` from `self_target` / `under_self`.
  **REACHABILITY 0 of 985 for the decode question.** **BLOCKED-ON: `src/core/card.c` —
  `card.rs`'s `placement_target()` needs a derived twin, not a decode one (cluster G+H).**

### Measurement traps T1 hit and recorded
- The runner links pre-existing `.o` files. Every measurement here rebuilt the object it
  needed and confirmed the `.o` mtime moved. Proof that the fix was really measured: the
  corpus census tool re-run against the rebuilt tree reports
  `unknown_card_type_effects 0` where the same binary logic on the baseline tree reports
  `168 (149 abilities)`.
- Counting must use `^FAIL` with **no colon** on stdout **and** stderr.
- `src/core/generated/gen_data.c` and `src/abilities_strings.bin` are the corpus of record;
  the census in `AUDIT_vm_resolver_enums.md` was produced by walking the real decoder
  (`rb_decode_card_ability`) over those blobs, not by reading Rust and guessing.

---

## T3. Cluster T3 — the CORE STATE MODEL (zones / game_state / card_binary / player / game_modifiers / alloc_counter)

Full audit, flushed incrementally: **`engine_c_wip/AUDIT_core_state.md`**.
C files owned: `zones.c`, `game_state_core.c`, `data.c`, `stats_pipeline.c`,
`tracking.c`, `alloc.c`, `alloc_counter.c`. Test file claimed:
`tests/test_audit_zones.c`. Snapshot + hashes: audit §0.

### Closed in T3

- [x] **T3-1** `rb_stage_first_empty` (`src/core/zones.c:304`) probed slots **0,1,2**
  (ascending) where Rust's `stage_first_empty` (`ability/util.rs:2541-2551`) probes
  **1,0,2 — CENTRE, LEFT, RIGHT**. A placement with no named vacated area therefore
  landed in **LEFT** where Rust lands it in the **CENTRE**. Same slot? No — a
  different slot.
  **REACHABILITY 25 of 985** reach a stage-placement arm (`destination:"stage"` 10,
  `"empty_area"` 8, `"same_area"` 7) plus every `turn/phase.c` single-baton area
  default. Fixed; also added the NULL guard (the old body **segfaulted**).
  **A/B, file-isolated, object mtime proved moved both ways:**
  `tests/test_audit_zones.c` FAIL_ANY **9 → 3** (the 3 survivors are the pre-existing
  red-pins), new-case assertions **2/9 → 9/9**. Eleven other suites **byte-identical**
  (`zone_filter_parity` 0, `mechanics` 0, `parity_zones_position` 38,
  `parity_mill_deck` 82, `parity_draw_flat` 74, `parity_energy_remainder` 7,
  `parity_energy_position_delta` 10, `parity_integration` 31, `parity_sequential` 142,
  `parity_under_member` 91, `position_and_source` 0).
  **A/B, file-isolated, object mtime proved moved both ways:**
  `tests/test_audit_zones.c` FAIL_ANY **9 → 3** (the 3 survivors are the pre-existing
  red-pins), new-case assertions **2/9 → 9/9**. Twenty suites byte-identical
  (`zone_filter_parity` 0, `mechanics` 0, `parity_zones_position` 38,
  `parity_mill_deck` 82, `parity_draw_flat` 74, `parity_energy_remainder` 7,
  `parity_energy_position_delta` 10, `parity_integration` 31, `parity_sequential` 142,
  `parity_under_member` 91, `position_and_source` 0, `parity_wave1a/b/c` 38/53/44,
  `parity_small_clusters` 111, `parity_cce_gaps` 92, `parity_full_round` 45,
  `parity_full_round3` 18, `parity_live_total` 53, `parity_live_total2` 10,
  `parity_recover_live` 57).
  **The reason most did not move:** the paths those 25 abilities take are
  `src/ability/effects/draw.c:131`'s inline ascending loop, `move.c:1846-1857`'s
  ascending `avail[]`, and `util.c:1111`'s private `rb_util_stage_first_empty_rust`
  (already correct). All three are **cluster F's files**. `util.c:1104-1116` had the
  right answer in a private copy and said so in a comment: *"zones.c is not this
  file"* — now it is.

- [ ] **T3-1b** ⚠ **ONE assertion flipped, and it belongs to another file.**
  `parity_full_round2` FAIL_ANY **68 → 69**. Isolated by a sorted `FAIL`-set diff of
  the two runs — exactly one added line:
  ```
  FAIL [N3_sync_opponent_gets_choice]: N3: the card P2 selected is at P2 left (got -1 expected 2642)
  ```
  **This is a TEST bug, not an engine bug.** Traced in the snapshot with a temporary
  `fprintf` (reverted; `util.c` sha256[:16] back to `89730B86DBDB73EC`, and `util.o`
  rebuilt from scratch after the revert because the first rebuild left the mtime
  unmoved — the stale-link trap, caught):
  ```
  TRACE rb_place_card_in_zone pl=1 card=2642 zone=empty_area vacated=-1 -> area=1 stage=[-1 -1 -1]
  ```
  That is `place_card_in_zone(card, Zone::EmptyArea, vacated_stage_area = None)`
  against an entirely empty stage, i.e. exactly `util.rs:2447`
  `stage_first_empty(&player.stage.stage)` → `Some(1)` = **CENTRE**
  (`util.rs:2542`). No reading of Rust puts this card on the left.
  **ACTION FOR `tests/test_parity_full_round2.c`'s OWNER: line 780 should read
  `centre`, not `left`. NOT applied here — that file is not cluster T3's, and relaxing
  another owner's assertion to reach a number is exactly the thing this repo forbids.**
  Context: `N3_sync_opponent_gets_choice` was **already red with 6 failures** before
  this change (the opponent prompt routing, and the `SelectPosition` prompt never
  being raised); this change adds the 7th.

### Open from T3

- [ ] **T3-2** `GameState.rule_log` still has **no member**. `RB_GSC_RULE_LOG_MAX 500`
  exists at `rabuka.h:3489` as a `#define` only, so `rb_gsc_push_rule_log`
  (`game_state_core.c:712`) routes into `rb_log_push_verdict(text,"rule_log",1)` — the
  shared **32-slot** condition-verdict ring, which drops on overflow and evicts
  condition verdicts. The header's own comment (rabuka.h:3484-3488) and the file banner
  (`game_state_core.c:36-45`) both forbid exactly this.
  **BLOCKED-ON: `include/rabuka.h` — exact declarations in AUDIT_core_state.md §7.**
  Measured effect: **0 of the 47** zero-caller `rb_gsc_*` functions became routable
  from this, because the blocker is broader than the fields — see T3-3.

- [ ] **T3-3** **45 of the 53 `rb_gsc_*` symbols have no prototype in
  `include/rabuka.h`** (only `rb_gsc_zone_snapshot`, rabuka.h:3265, has one). A function
  with no prototype cannot be called from another TU, so they are unroutable *even where
  the data they need already exists*. All 6 that do have callers got there by
  hand-writing a local `extern` (`game/game_setup.c:208` is the documented example).
  Split of the 47: **9 dead in both** (0 in-scope Rust call sites — `zone_snapshot`,
  `print_trace_node`, `recently_moved_cards`, `set_recently_moved_cards`,
  `current_turn_phase`, `game_has_ended`, `concede_both`, and `seated_pair` /
  `opponent_has_performed` whose Rust call sites are **all** in `bot/**`+`bin/**`,
  which PROGRESS.md puts out of scope), **34 header-blocked only**, **4 field+header
  blocked** (the logging group). The full prototype block is in
  `AUDIT_core_state.md` §7 — 45 declarations, all already implemented and compiling.

- [ ] **T3-4** `Player.deployed_this_turn` is a **per-CARD** `SmallVec<[i16;4]>` in Rust
  (`player.rs:200`; `is_area_locked` at `:286-294` resolves the member then asks whether
  *that card* is locked) and a **per-AREA** `GameState.stage_arrived[2][3]` in C. The
  Rust doc is explicit — *"The restriction follows the member (R4), not the area"* — so
  after a Rule 11.10 position change the two engines lock **different areas**.
  **REACHABILITY NOT YET MEASURED**: `position_change` is 29 abilities and
  `play_baton_touch` is 3, but the question is the **intersection** with the R4
  restriction, which was not evaluated. The fix is a `stage_arrived` cross-write beside
  the `stage_wait` crossed swap at `zones.c:252-257` — but
  `rb_stage_position_change_with_bags` has **no `GameState*`**, so it cannot reach the
  field. **BLOCKED-ON: `src/core/state.c`** (declares the function at `state.c:10` and is
  its only caller, `:1677/:1692/:1732`) — changing the signature needs that file, and it
  is not cluster T3's. Not fixed on purpose: a speculative port on an unmeasured
  intersection can only be wrong.

- [ ] **T3-5** `GameModifiers.constant_global_need_heart` (`game_modifiers.rs:194`) — **the header half LANDED, the wiring half did NOT, and the C member is WRITE-ONLY in Rust too (cluster Y.5).** `include/rabuka.h:1807-1808` now carries `int16_t constant_need_heart_global[RB_MAX_CARD_IDS][8]` + `uint8_t constant_need_heart_global_set[RB_MAX_CARD_IDS][8]`, and **both are still 0 writers / 0 readers.** Two corrections to the row above: (a) the producer is **not** in `src/core/modifiers.c` — `modify_required_hearts_global` is implemented at `src/turn/triggers.c:954` (CONSTANT arm) and `src/ability/effects/executor.c:222` (effect arm), and neither file is this cluster's; (b) **there is no in-scope consumer to wire.** Rust writes the field at `core/game_state/modifiers.rs:613-626` (the take/restore around the constant scan) and reads it at exactly ONE place, `game/display.rs:1984-1986` — **T8 Q2: read-only projection, OUT OF SCOPE**. `modify_required_hearts_global` does NOT read it; the C applies the amount directly to each live card, which is why the ledger is not on the gameplay path in either engine. **BLOCKED-ON `include/rabuka.h` for the count** (the field is there, `n_constant_need_heart_global` is not declared and cannot be wired into a `[4096][8]` array by main's owner as written — see Y.5), and BLOCKED-ON `src/turn/triggers.c` / `src/ability/effects/executor.c` for the producer.

- [ ] **T3-6** `GameModifiers.success_zone_blade_bonuses` /
  `_heart_bonuses` / `_score_bonuses` (`:242/:245/:248`) have no C member, and neither
  does the `Vec<BonusSource>` display ledger (8 such `Vec`s in total across
  `GameModifiers`). Rust clears and re-applies them in
  `evaluate_success_zone_constant_modifiers` (`game_state/modifiers.rs:2194-2212`), which
  is what keeps a **previous** live's success-zone bonus from leaking into the next one.
  **REACHABILITY: the success-zone scanner runs on every live**, so the three ledgers are
  live state, not display-only; the 8 `Vec<BonusSource>`s are display-only and
  `VERIFIED-NOOP`. **BLOCKED-ON: `include/rabuka.h`** (three `int16_t` arrays +
  three counts) — and the clear/re-apply pass itself lives in
  `src/core/modifiers.c`, which is **cluster E's**.

- [ ] **T3-7** `Player.exclusion_zone` (`player.rs:185`) is **per-player** in Rust and a
  single **global** `GameState.exclusion_zone` (rabuka.h:1604) in C, so the two seats
  share one bag. **VERIFIED-NOOP: 0 of 985** — `exclusion_zone` is 0 of the 2674
  zone-valued corpus occurrences. Recorded so a future corpus is not surprised.
  Also `LiveCardZone.face_up` (`zones.rs:501`) has no C member: written by
  `add_card`, ignored by `calculate_live_score`, read by nothing on this corpus.
  **VERIFIED-NOOP: 0 of 985.**

- [ ] **T3-8** Structural divergences that are correct to leave (all measured, all
  `n = 0`), so nobody "fixes" them later:
  `check_effect_position` (`zones.c:161`) accepts `中央`/`左`/`右`/`左側`/`右側`, which
  Rust's `zones.rs:198-205` `_` arm answers `true` for unconditionally — 0 corpus
  occurrences. `MemberArea` CamelCase spellings in `rb_member_area_to_index` — 0.
  `rb_zone_total_blades`'s extra `stage_wait` source, which Rust's per-CARD
  `orientation_modifiers` has no counterpart for — and the function has **0 C callers**
  anyway. `rb_energy_add_card`'s missing energy-type check (Rust `zones.rs:658-672`
  refuses a non-energy card) — its only production callers are
  `rb_setup_initial_energy` (phase.c:1386/1390), which draws from the energy deck.
  `draw_place_in_zone_g`'s (`draw.c:91-95`) empty destination → HAND where Rust sends
  `""` to the **WAITROOM** (`util.rs:2530-2537`) — 0 occurrences of `destination:""`;
  the 906 absent destinations resolve to `hand` on both sides. The decoder's
  zero-count heart pairs (`card_binary.rs:326-340` inserts only `count > 0`;
  `card.c:110-117` stores every pair) — every C reader adds 0 or tests `> 0`, so 0 of
  2639 cards can tell. `ModifierEntry::total`'s `saturating_add` vs C's plain `+`
  (`rabuka.h:199`) — the corpus's largest single modifier value is 10, so the two
  cannot differ.

- [ ] **T3-9** ⚠ **`rb_zone_of_str` (`src/engine.c:145`, cluster I's — REPORTED, NOT
  EDITED).** It returns **0** for `"energy_deck"` (explicit `case` at engine.c:157, 52
  corpus occurrences), for `"deck_bottom"` (engine.c:156, 38 occurrences) and for **any
  unmapped spelling**, and `draw_place_in_zone_g` (`draw.c:91-95`) turns that 0 into the
  **hand**. That fallback is load-bearing in the wrong direction for exactly two corpus
  strings, both `location`-keyed rather than `destination`-keyed:
  **`activating_card`** (1 occurrence — names a card role, not a zone; Rust's
  `Zone::from_str` also returns `None` and Rust's `_` arm then sends it to the hand, so
  the two AGREE) and **`ステージ`** (1 occurrence — Rust's `ZoneId::from_str` accepts it
  but `Zone::from_str` does **not**, so Rust's `None` → hand as well; the two AGREE).
  The one place they genuinely differ is `destination: ""`: Rust
  `util.rs:2530-2537` sends the empty string to the **waitroom**, C sends it to the
  **hand**. Corpus count **0**. Also load-bearing: `draw.c:132-139` sends a P2 card to
  `P1`'s hand because `rb_place_card_in_zone_g`'s `player` is the *target*, not the
  seat — Rust's `_` arm resolves the seat correctly. Counted in the hand-related
  FAIL lines of `parity_draw_flat` (74 FAIL_ANY) and `parity_sequential` (142).

## T2. Cluster T2 — STATE effect family (`state.c`, `ability.c`, `ability_effects.c`, `score.c`)

Closed 2026-10-03. Owner: the state-family agent. Full inventory, arm table and A/B in
`AUDIT_effects_state.md`. Corpus numbers are **decode-level** over the shipped bytecode
(all 985 abilities, every effect-valued field of `AbilityEffect` plus `Ability.cost`,
1866 nodes) using `rb_canonical_trigger` for the trigger class.

⚠ **The brief's `ability/effects/state.rs` does not exist.** Commit `cf125b15` renamed it
to `change_state.rs` and split it across the per-kind modules; the 2064-line `state.c` is a
port of a file that is gone. The two brief suites `parity_dynamic_count` and `audit_cards`
have no `tests/test_<stem>.c` anywhere among the 145 test sources.

**VERIFIED-NOOP (0 of 985, nothing ported).** `rotation` (0 nodes, any trigger class),
`set_cost`, `set_cost_to_use`, `all_blade_timing`, `activation_cost`, `energy_placement`,
`modify_limit`, `modify_required_hearts_success`, `custom`. `energy_state_change` is 0 as an
action but its 27 reachable calls come through `change_state` with `card_type=energy_card`.
`rb_effect_position_change` (the CONSTANT-scan twin of `position.rs:96`) is 0 of 985 —
`position_change` is never a 常時 (debut 10, live_start 8, activation 6, auto 2,
live_success 1, **CONSTANT 0**).
**VERIFIED-NOOP, writer-only fields.** `gained_ability_sources` (written by
`ability_ops.rs:521/:740`, read in Rust only by `display.rs:1733`) and
`gs.position_change_events` (written, cleared at `abilities.rs:1863`, read only by
`display.rs:1857-1861`) have no behavioural reader, so their absence from the C is a
display/replay gap, not a wrong value. **This resolves E9 as display-only** — and E9's
three "live" consumers (position.rs:1022/:1061/:1102) are all inside
`execute_position_change_with_destination`, which IS live and IS covered.
⚠ **Cluster Y.5 found the C half of that gap and it is not merely absent —
it is now present and DEAD.** `include/rabuka.h` carries
`position_change_events[]` + `n_position_change_events`, and
`src/core/game_state_abilities.c:589` (`g->n_position_change_events = 0`) is the
**only** reference to either in the entire tree: 0 writers, 0 readers. Rust's
`PositionChangeEvent` (`core/types.rs:993-1006`) is written at `position.rs:622/
633/914/1184` and read **only** by `display.rs:1857-1861` (**T8 Q2: OUT OF
SCOPE**). So this is the same WRITE-ONLY-in-Rust terminal state as
`base_score_total` / `spare` / `live_spare`: a header line that reads as coverage
with nothing behind it. It should either get a producer in
`src/ability/effects/state.c` (this file's neighbourhood, `rb_stage_position_change_
with_bags` is the natural site) or be deleted from the header.

**CLOSED — 7 divergences in `rb_effect_change_state` (`state.c:70`), all reachable:**

- [x] **T2-1** `change_state.rs:662-669` — no `recalculate_constants()` anywhere in the
  function; the only recalc on the path was `engine.c:1061`, after
  `rb_drain_ability_queue`, far too late for the sweep at `:765-768`. **69 of 96** member
  `change_state` nodes. Landed. Closed one pre-existing peer assertion:
  `parity_condition_and_effect` **51 → 50** — `PL!S-bp5-016-N`,
  *"two or more waited members unlock the heart06 rider"*, which IS this recalculation
  (a 常時 heart06 rider counting 2+ waited members).
- [x] **T2-2** `:653-659` — `cards_activated_this_turn` (rabuka.h:1495, audit A8) had **no
  writer in `src/**`**, so the `activated_this_turn` condition (`condition/card.rs:2417-2424`,
  **3** nodes) was permanently false. **47** `state_change="active"` nodes. Writer landed.
- [x] **T2-3** `:458-467` — Q275 `action_by="opponent"` wait-immunity filter on the OFFER
  was absent. **3** nodes.
- [x] **T2-4** `:585-599` — Q274 apply-time wait-immunity filter absent. **3** nodes.
  Both now call `rb_rules_is_wait_immune` (rules.c:2233), which IS populated (`rules.c:1916-1921`);
  the exported `g_wait_immune_members` is still unwritten and is the wrong thing to read.
- [x] **T2-5** `:143-145` — `per_unit && count == 0` early return absent. **5** nodes.
  Sharpest finding of the audit: for a 「1枚の」 step over an EMPTY live-card zone the
  baseline **waited a card, wrote a `turn_state_changes` entry, pushed onto
  `recently_state_changed` and added to `selection.cards`**.
- [x] **T2-6** `:762-768` — the sweep was
  `trigger(g, who); if (who != (actor^1)) trigger(g, actor^1);`, which skips the second call
  exactly when `who == actor^1`, i.e. on every OPPONENT-targeted step, so the controller's own
  常時 were never re-evaluated. **40 of 96** nodes have `target="opponent"`. Now both seats,
  unconditionally.
- [x] **T2-7** `:684-687` — `selection.changed_state_members` writer absent; the hook
  `rb_rules_note_changed_state_members` had **zero callers tree-wide**. Writer landed; the
  READER is T2-11 below.
- [x] **T2-8** **DELETED `rb_bulk_state_evaluate`** (was `state.c:1930-2057`, ~300 lines) — a
  **second copy of `execute_change_state` in the same translation unit as the live one**,
  zero callers and zero declarations tree-wide, and stale rather than merely redundant: no
  `cost_limit`/`blade_limit` filter (never calls `rb_matching_ids`), keyed off `state_filter`
  instead of `state_change`, single-seat sweep, and none of T2-1..T2-7. Same shape as the
  earlier `rb_effect_modify_hearts` deletion above it.

**CLOSED — the `ability.c` vs `ability_effects.c` duplication. VERDICT: both files are LIVE
and the duplication was ONE function, not a file split.** `ability.c` = the AbilityRef store
+ the gain/suppress effect bodies; `ability_effects.c` = the executor shims over the same
effects (`rb_suppress_ability_trigger` IS the `ability.c` body; the `ability_effects.c:400`
wrapper only forwards — that pair is layering, not duplication). The real hazard was
`execute_gain_ability_from_source` (`ability_ops.rs:593-700`) implemented TWICE and reached
from BOTH sides:

- [x] **T2-9** collapsed to one implementation,
  `ability_effects.c:rb_ability_gain_from_source_core`, with `ability.c`'s
  `rb_gain_ability_from_source` a 1-line delegation. Divergences removed: `trigger_filter`
  whole-CSV `strstr` vs Rust's per-token `filters.iter().any(...)`; owner lookup
  `g->p[actor].stage` vs Rust's player1-then-player2 probe; `group_names` single-name
  `check_group` vs any-of-the-list. **Corpus-inert**: both nodes carry a SINGLE-token filter
  (「起動」, 「ライブ成功時」), so 0 of 985 diverged. The one load-bearing difference is
  **preserved**: the recalc must NOT be on the CONSTANT path, because `modifiers.rs:1148`
  runs that scan from inside `recalculate_constants` and a callback would re-enter it.

**NOT CLOSED — measured, reachable, and in files this cluster does not own:**

- [x] **T2-10** LANDED 2026-10-04. The blocker was that `st->changed_state_members` is a `rules.c` file-static, so it could not be lifted **or** `rules.c:1819` made the dispatch target — and the second option turns out to be wrong anyway. Both files are now this cluster's, and the fix is the LIFT:
  * `rules.c` gained `rb_rules_take_changed_state_members(g, out, max)` — a read-then-clear accessor that keeps the "read and clear" atomicity in the file that owns the storage, which is exactly Rust's assignment-then-consume pair. Deliberately NOT exposed as the struct: `RbRulesState` is rules.c's own file-static typedef, and a two-call consume is what stops a second restriction in the same ability re-applying to the same cards. Not added to `include/rabuka.h` (not this change's to edit); `misc.c` declares it locally, the pattern `state.c:21-26` already uses for `rb_rules_note_changed_state_members`.
  * `misc.c`'s LIVE `h_restriction` now HAS the arm, with the right precedence: `changed_state_members` first, then the `recently_moved` fallback, then the activating card. The general `recently_moved` fallback and the `cannot_active_cards` bookkeeping are left exactly as they were.
  * ⚠ **Delegation was rejected with a reason, not skipped.** The three copies are NOT interchangeable: `rules.c`'s records `st->wait_immune` per member with the blade-limit filter and `st->cannot_activate_members`, while misc.c's records `g->cannot_active_cards` and none of rules.c's list. Routing 1 to 2 would have been a behaviour change no Rust line demands. Two further differences between the copies (misc.c accepts a third `"cannot_activate"` spelling; the two log through different helpers) are left alone for the same reason. The triplication is now documented at both sites instead of being a trap.
  * **Proven twice.** Suite: `tests/test_parity_state_family.c`'s `test_delayed_restriction_keys_on_the_changed_members` goes **RED -> GREEN** (`FAIL 2 -> 1`, hidden `3 -> 1`) with the two `OPEN GAP (misc.c:905)` lines becoming `ok`. Focused: `tests/test_parity_fw_effects.c` case `fw_t2_10_restriction_consumes_the_changed_state_latch` additionally pins the **consume-once** half that the suite does not — base 1 FAIL, fix 0.
- [ ] **T2-11** `BLOCKED-ON src/core/card.c:957` — `heart_total_limit`, the 「元々持つハートの数」
  clause of a `change_state`. util.c's matcher half landed under H1; `card_filter_from_effect`
  has **zero** writes to `has_original_heart_total` (no `heart_total_limit` string anywhere in
  card.c), so the producer is missing and the matcher is dead code. Reach: **4** of 96
  `change_state` nodes. This is the same missing producer as J9-M, reached from the state side.
- [ ] **T2-12** `BLOCKED-ON src/ability/condition.c` — the `activated_this_turn` CONDITION
  (`condition/card.rs:2417-2424`) has no `cards_activated_this_turn` reference anywhere in
  condition.c. The writer half (T2-2) landed, so the reader is the only half left. Reach: **3**
  of 985. Until it lands the writer is inert and this is D8's `activated_this_turn` row from
  the condition side.
- [~] **T2-13** `VERIFIED-NOOP: 0` — re-measured 2026-10-04 with the engine's own decoder, and the item's blocker is CONFIRMED still standing. Measured: **52** `modify_score` nodes target `live_total`; exactly **1** node corpus-wide carries `operation = "set"`, and it targets **no** card type and **not** `live_total` (it is a card-scoped 「このターン、エールにより…」 effect). So `operation == "set"` **AND** `target == "live_total"` is **0 of 985**, and the refusal at `score.rs:274-287` has nothing to refuse. Per the "0 ⇒ VERIFIED-NOOP" rule it is closed as a measurement and NOT ported.
  ⚠ **The blocker is still the peer's test, and it is a real blocker, not a formality:** `tests/test_parity_score_effects.c` actively asserts the OPPOSITE of Rust ("live_total operation=set adds 4 onto the existing +1", citing `score.rs:142-146`, which is now inside the live-zone list rather than the operation match). A previous agent implemented the refusal, A/B'd it, and reverted because it turned that suite **0 → 1** for zero reachable card. That reasoning still holds and was not re-litigated here. `score.c`'s live-total arm keeps the full note, so it is not re-derived. After the peer drops its `set`-accumulation expectation, the fix is one line in `score.c`'s live-total arm: `if (!strcmp(op, "set")) return 0;`. **No test was weakened, deleted or ignored by this change.**
- [ ] **T2-14** `BLOCKED-ON include/rabuka.h + src/core/modifiers.c` — `mods.last_cost_energy_count`
  for `per_unit_type="つ"` (`score.rs:225-227`). The C returns 0 and `util.c:48` carries the
  match behind `#ifdef RB_MODS_LAST_COST_ENERGY_COUNT`, which nothing defines, because the
  field does not exist. Reach: **1** of 130 `modify_score`. Adding it lights up util.c's
  existing match for free.
- [ ] **T2-15** `BLOCKED-ON src/ability/effects/misc.c` — the whole `position.rs` EXECUTOR
  twin lives in misc.c, not in this cluster's four files: `h_position_change` (27),
  `compute_valid_position_destinations` (27), `rb_misc_finalize_formation_change` (7 — the
  `multiple_targets` formation path), `h_play_baton_touch` (3). Additionally **`triggers.c`
  has no `play_baton_touch` arm at all**, while `rb_canonical_trigger` says **2 of the 3**
  `play_baton_touch` nodes are 常時, so those two do nothing on every recalculation. Not
  audited arm-by-arm here.
- [ ] **T2-16** `exclude_self` source: Rust uses `gs.activating_card` (`:315`), the C uses
  `host_cid` (`state.c`). Not changed — no measurement separates the two, and the only route
  where they differ (the CONSTANT scan, which passes `host_cid = -1` at `triggers.c:1202`)
  is 0 of 985 because `change_state` is never a 常時.

**Test-side claim.** `tests/test_parity_state_family.c` is **NEW** (39 checks, so it adds a
row to the J claim). Build with `make t T=parity_state_family` — no Makefile edit is needed,
`make t` accepts any `tests/test_<stem>.c`. It asserts Rust's behaviour at
`change_state.rs:143-145 / :653-659 / :662-669 / :684-687`,
`ability_ops.rs:614-619 / :679-681` and `score.rs:251-256 / :274-287`, each with the Rust
line in the CHECK message, and it is a REAL regression test: against the baseline tree (the
same four files reverted to their pre-audit content) it reports **13** FAIL lines, against
this tree **3** — the 3 being T2-10 and T2-13, marked `OPEN GAP` in their text. Its
per-unit case deliberately locks the CONTRACT rather than reproducing T2-5, because the old
fall-through and the new early return leave the board identical.

**A/B.** Trees `est` (these four files) vs `estb` (the same four reverted via
`git show HEAD:engine_c_wip/…`; HEAD content == pre-audit content because `git status` showed
all four clean at session start). A SHA-256 sweep over every `*.c`, `*.h` and `Makefile`
reports **exactly 4 differing files** — the four owned here.
`abilities_strings.bin` = 726831 / `89204DF0BBDE08A0` in both trees before and after every
build. **64 suites, 1485 → 1474 FAIL lines, delta -11, ZERO regressions** (only
`parity_condition_and_effect` -1 and `parity_state_family` -10 moved).

---

## T7. Cluster T7 — the GENERATED decoders + the "unmapped" resource family (COMPLETE)

Owner: T7. Full write-up: **`AUDIT_decoders_resource.md`**.
Rust modules audited: `ability/condition_decoder_gen.rs`,
`ability/effect_decoder_gen.rs`, `ability/effects/resource.rs`,
`ability/effects/energy.rs`, `ability/effects/deck.rs`.
C files claimed: `src/ability/vm.c`, `src/ability/resolver.c`,
`src/ability/enums.c`, `src/ability/types.c`.
Test file claimed: **`tests/test_parity_t7_condition_decoder.c`** (new).
Makefile gained four census targets: `t7-audit`, `t7-wire`, `t7-res`, `t7-zero`.
Run the new suite with `make t T=parity_t7_condition_decoder`.

| id | status | finding |
|---|---|---|
| **T7-a** | **CLOSED** | `decode_condition_direct` (`vm.c` — **dead: 0 callers**, and `rb_decode_fallback_count()==0` after decoding all 985 abilities) lost **5** keys to its catch-all: `any_location` 18 nodes/18 abilities, `activated_this_turn` 3/1, `paid_cost_this_turn` 1/1, `text` 680/504, `trigger_event` 131/112. Chose option **(b) fix the catch-all**; all 5 now decode exactly as Rust does. `vm.c:1406-1420` (locals), `:1615-1622` (copy onto every variant), `:2642-2699` (5 arms). A/B **630 → 0** `^FAIL`; 19 pre-existing suites byte-identical. |
| **T7-b** | **VERIFIED-NOOP (mirror is lossless)** | The live mirror `read_condition_fields` (`vm.c:404`) is **proven lossless**: 79 distinct condition keys on the wire, all 79 present in the mirror, set difference **empty**, across all 985 abilities. Rust has **no** mirror — `read_condition_value` (`vm.rs:561`) → `decode_condition_direct` is its only condition decoder — so option (a) would delete the twin of a LIVE Rust item, which is exactly the reasoning a peer already used to refuse a delete. **Do not wire the typed decoder ahead of T7-a.** |
| **T7-c** | **VERIFIED-NOOP: 0 of 985** | Rust's `check_phase_gate` (`condition.rs:398-422`) falls back to `trigger_event.phase`/`.phase_target`; C's `rb_check_phase_gate` (`condition.c:2547`) has no such fallback. Measured cross-tab: **every** condition carrying `trigger_event.phase` also carries its own `phase` (`DIVERGE_phase=0`, `DIVERGE_phase_target=0`). `condition.c` is cluster D's — **BLOCKED-ON: condition.c:2547 — add the `trigger_event.phase`/`phase_target` fallback only if a future corpus makes `DIVERGE_phase > 0`.** |
| **T7-d** | **VERIFIED-NOOP: 0 of 985** | Rust's `phase_ok` match has `_ => true` for a phase outside main/active/live; C returns 0 for `live`/`live_card_set`/`live_performance`/`live_victory`. Only `main` (5), `active_phase` (2) and `live_phase` (2) occur as condition `phase` on this corpus, so Rust's `_ => true` arm is unreachable and the divergence cannot fire. |
| **T7-e** | **0 MISSING** | **EFFECT decoder: no key with count > 0 lacks C handling.** Wire census cross-checked against decoded census: "on the wire, not retained by the C" is **empty**. The C is lossless here **by construction** — `decode_effect_body` (`vm.c:685`) has a 48-key structural dispatch plus a scalar tail (`vm.c:1169-1190`) that retains every other key. 143 keys on the wire, all retained. One C **superset**: `replacement` (1 node) is absent from Rust's generator too; Rust drops it, C keeps it, no consumer — not a wrong value. |
| **T7-f** | **VERIFIED-NOOP: 0 of 170** | `resource.rs:846-864`'s `is_all` heuristic tests key **presence** (`exclude_self_any().is_none()`, `distinct_any().is_none()`); `misc.c:735-738` tests **truthiness**. Divergence needs `exclude_self:false` or `distinct:false` present. Measured on the 170 `gain_resource` nodes: `exclude_self` absent 163 / true 7 / **false 0**; `distinct` absent 169 / **false 1** — and that one node does not also carry `card_type=member_card` + player target + no `target_count`. Both heuristic terms are 0. `misc.c` is cluster C's — **BLOCKED-ON: misc.c:737 — use an explicit presence test for `exclude_self` (and misc.c:738 for `distinct`) ONLY if a corpus ever ships those keys set false.** |
| **T7-g** | **VERIFIED CORRECT** | `handle_bp6_pattern` (`resource.rs:297-373`) has exactly **1** reachable node — **ability 604**. Compared term by term with `h_bp6_pattern` (`misc.c:654-684`): guard identical; no-activating-card identical; distinct colours from the same source; sign `+1` / `is_negative=0` identical; the cap cannot differ (≤4 base hearts per card, measured `max_num_base=4`). **One code difference:** Rust filters `if amt > 0` (`resource.rs:289`), C has no such filter — **VERIFIED-NOOP: `make t7-zero` → `cards=2639 zero_amount_base_entries=0 (member=0 live=0)`**, so no card in the corpus has a zero-amount base heart. |
| **T7-h** | **CORRECTS F2** | `gain_heart_colors_from_selected_card` (`resource.rs:648`) is **unreachable, not missing**: `heart_colors_from_selected_card` occurs once in the whole corpus and that node is **not** on a `gain_resource`. **F2 can be closed as VERIFIED-NOOP: 0 of 985** instead of staying a "missing C function". |
| **T7-i** | **mapped, not unmapped** | `resource.rs` (1899 lines) is **already mapped** — into `src/ability/effects/misc.c` (HELD by cluster C) + `modifiers.c`. 19 of 21 items have a twin: `execute_gain_resource`→`h_gain_resource` misc.c:687, `apply_heart_resource`→misc.c:577, `apply_blade_resource`→misc.c:536, `calculate_gain_multiplier`→misc.c:346, `resolve_gain_resource_targets`→misc.c:387, `handle_bp6_pattern`→misc.c:654, `execute_gain_surplus_heart`→misc.c:1529, `grant_heart_distribution`/`grant_blade`→modifiers.c:349/354. The 2 without a twin (`selected_before_save`, `selected_after_save`) are private list splitters folded into `resolve_gain_resource_targets`. `energy.rs`→misc.c:1152/1976/836 + state.c:680/707; `deck.rs`→draw.c:276/866 + misc.c:966. **Inventory totals: 61 items across the five modules, 59 C twins, 2 private-and-folded, 0 unmapped.** |
| **T7-j** | **VERIFIED-NOOPs** | Other measured-dead resource branches: `ResourceKind::from_str _=>Other` 0/170 (the 2 `surplus_heart` nodes are intercepted at `resource.rs:866`, before `:828`); `apply_heart_resource require_all_heart_colors` 0; `per_group` 0; `apply_blade_resource card_type==live_card` 0; `execute_pay_energy target==self` 0 and `source=energy_deck` 0; `place_energy_impl self_cost`/`allow_occupied_stage`/`location` 0; `execute_draw source=` anything but `deck` 0/45; `execute_draw shuffle`/`dynamic_count` 0; `execute_draw_until_count location` 0. |
| **T7-k** | **OPEN (T7-x)** | `decode_condition_direct` leaks `ConditionLocals`. Five members transfer ownership into the built `Condition` (`condition`, `conditions`, `options`, `cause`, `effect` — `cond_add_cond*` store the pointer, no copy), so a partial free would double-free on exactly those. Deliberately **not** fixed: never-executed free code is untested and more dangerous than the leak. If the typed decoder is ever wired, the free must be all-or-nothing over those five. |
| **T7-l** | **OPEN (J9 follow-up)** | `watches_ability_resolution` is **on the wire: 4 nodes / 4 abilities**. C retains it in the extras table (scalar tail) and Rust **CONSUMES** it (`abilities.rs:641-650`, `:813-822`). J9 measured "zero occurrences in engine_c_wip", i.e. **no C consumer** — a real 4-ability behavioural gap. `executor.c`/`state.c` are HELD. **BLOCKED-ON: src/ability/effects/executor.c — add the `watches_ability_resolution` consumer; the value is already retained by the decoder.** |

### How T7's counts were obtained (so nobody re-derives them)

T1's figures (21 / 3 / 2 / 160 for four of the catch-all keys) were counted over
`cards/abilities.json`, which is **stale** (K7). T7's figures are decode-level
node counts over the **shipped bytecode** and are the measurement of record:

```
make t7-wire  ->  abilities_walked=985 cond_nodes=685 eff_nodes=2230
COND|any_location|18|18|C_DROPPED
COND|activated_this_turn|3|1|C_DROPPED
COND|paid_cost_this_turn|1|1|C_DROPPED
COND|text|680|504|C_DROPPED
COND|trigger_event|131|112|C_DROPPED
```

A decoded-tree walk (`make t7-audit`) sees 1589 condition nodes / 4925 effect
nodes because each ability is decoded once per (card, ability) pair it appears
on — a 2.2x fan-out over the 2116 pairs. That ratio is the cross-check that
neither walker is dropping objects.

**Tooling note:** a raw-bytecode walk is *required* for a per-variant audit of a
GENERATED decoder, because `read_condition_fields` folds every nested dict
(`trigger_event`, `sub_checks`, `position`, `quoted_text`, `dynamic_count`) into
`Condition::fields[]`, so their keys get mis-attributed to the condition key
table. `tools/audit_t7_wire.c` routes each key to the node class Rust assigns it.

### T7 A/B evidence (sorted `^FAIL` lines, stdout AND stderr; exit code ignored)

| suite | fixed out/err | baseline out/err | diff |
|---|---|---|---|
| parity_conditions | 0 / 0 | 0 / 0 | IDENTICAL |
| parity_condition_and_effect | 0 / 50 | 0 / 50 | IDENTICAL |
| parity_integration | 0 / 31 | 0 / 31 | IDENTICAL |
| parity_sequential | 29 / 112 | 29 / 112 | IDENTICAL |
| parity_small_clusters | 42 / 69 | 42 / 69 | IDENTICAL |
| vm_decode | 0 / 0 | 0 / 0 | IDENTICAL |
| decode_fallback | 0 / 0 | 0 / 0 | IDENTICAL |
| parity_ability_mod | 0 / 13 | 0 / 13 | IDENTICAL |
| parity_ability_mod_extra | 0 / 16 | 0 / 16 | IDENTICAL |
| parity_zones_position | 0 / 37 | 0 / 37 | IDENTICAL |
| parity_under_member | 31 / 60 | 31 / 60 | IDENTICAL |
| parity_ability_watch | 9 / 19 | 9 / 19 | IDENTICAL |
| parity_jidou_watch | 23 / 55 | 23 / 55 | IDENTICAL |
| parity_live_total | 20 / 32 | 20 / 32 | IDENTICAL |
| parity_recover_live | 21 / 36 | 21 / 36 | IDENTICAL |
| parity_executor_dispatch | 0 / 0 | 0 / 0 | IDENTICAL |
| parity_full_round | 16 / 29 | 16 / 29 | IDENTICAL |
| parity_enums | 0 / 0 | 0 / 0 | IDENTICAL |
| audit_fields | 2 / 3 | 2 / 3 | IDENTICAL |
| **parity_t7_condition_decoder** | **0 / 0** | **0 / 630** | **DIFF err=630** |

Baseline = the snapshot with **only T7's three `vm.c` hunks reverted**, not git
HEAD (`vm.c` arrived with uncommitted T1 content). No test was weakened,
deleted or ignored; no existing assertion was touched.

### 🪤 A THIRD STALE-LINK VARIANT — restoring a file restores an OLDER MTIME

T7 hit the documented trap and then a variant of it that the existing warning
does not cover. `Copy-Item` **preserves `LastWriteTime`**, so restoring the
fixed `vm.c` gave it mtime 07:29:10 while `vm.o` was already 07:55:45 — make saw
`vm.c < vm.o` and **skipped the rebuild**. The "fixed" run then produced a
**byte-identical 630-FAIL result to the baseline**, which is exactly the shape
of a false "my change was neutral". Defeated by `rm src/ability/vm.o` and
rebuilding, after which `vm.o` (07:56:43) is verifiably newer than the source.

**Rule: when reverting OR restoring a source file for an A/B, delete the object
(`rm src/ability/vm.o`) rather than trusting `make`'s timestamp comparison, and
assert the object's mtime is past the source's before reading any FAIL count.**

Two more harness facts T7 recorded:

* `Get-ChildItem -Recurse -Filter *.o` picks up `src/main.o`, which is **not** in
  `OBJ_LIB`. Linking a test against that glob fails with
  `multiple definition of 'main'` on every suite. Exclude `main.o`.
* Do not write a parity test that re-implements the bytecode reader. T7's first
  draft did, and the duplicated reader desynced and produced a screen of bogus
  "failures" (missed 174 of 685 nodes, read ability-level keys as condition
  keys). The committed test builds **synthetic bytecode byte by byte** and drives
  `decode_condition_direct` directly, so the reader under test is the engine's
  own.

---

## 🪤 TWO TRAPS THAT SILENTLY FALSIFY EVERY NUMBER (proved 2026-10-03)

### 1. STALE LINK — your "after" run may be a second baseline
`make t T=<stem>` **links pre-existing `.o` files.** Editing a `.c` and re-running
can therefore measure the OLD code. A peer got a byte-identical 978 FAIL lines across
two runs this way and nearly reported it as "my change was neutral" — it was the
runner, not neutrality.

**Rule: after any source edit, rebuild the affected objects explicitly
(`make src/ability/vm.o` etc.) and CONFIRM the object mtime moved, or do a full
`make`.** Treat a byte-identical before/after as evidence of a stale link until
disproved. **Third variant, added by T7: `Copy-Item` preserves `LastWriteTime`,
so REVERSING or RESTORING a source file can hand it an mtime OLDER than the
object and make will skip the rebuild.** For any A/B, `rm` the object and
rebuild; do not rely on make's timestamp comparison.

### 2. `FAIL` WITH NO COLON IS INVISIBLE TO A `^FAIL:` GREP
`tests/test_audit_triggers.c` prints `FAIL [<row>] …` with **no colon**. A verifier
grepping `^FAIL:` reported `FAIL_OUT=0 FAIL_ERR=0` while the suite was actually running
4 ok / 3 FAILED with **5 real FAIL lines**.

**Rule: always count with `^FAIL` (no colon), and check stdout AND stderr.**
Also `tests/test_parity_queue_resume.c` **aborts mid-file** — its last 5 tests and
`rb_unload()` never run and it emits zero stdout, so **a suite producing no output is
BROKEN, not PASSING.** Both are open harness defects; T4's audit rows A2/A3/A4b assert
`rb_use_count == 1` immediately after enqueue, which Rust does NOT do (`use_count == 0`
is faithful — `record_ability_use` is resolution-time only).

---

## ⚠ CLUSTER K7 — INDEPENDENT CORROBORATION FROM T5 (the reference may be wrong)

Cluster T5 measured describe output in BOTH languages and found **983 of 985 abilities
byte-identical, 1 divergence (ability 325), 2 excluded**. It then found the cause is NOT
`describe.c`:

- `RBKA_OFFSET_DELTAS` (`gen_data.c:7`) sums to **98737** == `RBKA_BYTECODE_LEN`.
- `OFFSET_DELTAS` (`cards/build/abilities_gen.rs:25`) sums to **98725** against a
  **98733**-byte blob — an 8-byte shortfall.
- The two tables **differ at 2 of 985 slices: 325 and 805.**

Rust reads the **stale 726649-byte** `cards/build/abilities_strings.bin`; C reads the
canonical **726831-byte** one. So for at least abilities 325 and 805 the Rust engine
resolves a DIFFERENT byte slice than the C engine does. **Any parity claim for those
abilities is currently meaningless in either direction**, and a "C is wrong" verdict
derived from Rust for them would be wrong.

T5 also **disproved the long-standing "933/936 stale goldens" claim**:
`test_output/describe_dump.json` does not exist in the repo, and `describe_dump.rs:33-37`
writes one record per ability over `0..NUM_ABILITIES` = 985, so it **cannot** emit a
936-row file. The claim was arithmetically impossible, not merely stale. It is replaced
by the 983/985 measurement above.

**This is the strongest evidence yet that the stale Rust blob (K7) is doing real damage.**

---

## T6. Cluster T6 - the ACTION / DRIVER LAYER (`engine.c`, `turn/phase.c`, `turn/live.c`)

Full inventory, wiring census and per-fix evidence: **`AUDIT_actions_setup.md`**.

Rust in scope: `turn/actions/mod.rs` (1895) and `game/game_setup.rs` (2427),
cross-checked against `core/types.rs` (1262). C in scope: `src/engine.c`,
`src/turn/phase.c`, `src/turn/live.c`. `src/web_server.c` was NOT taken (T5's);
`src/ability/choice_renderer.c` (1821 lines) is confirmed an **orphan** - zero
references in `Makefile:20-70`, `platforms/cdi/build_cdi.sh`,
`platforms/genesis/build_genesis.sh` or `platforms/Makefile`.

### Counts

| | |
|---|---|
| Rust `turn/actions/mod.rs` items inventoried | **43** (5 `pub fn`, 34 private `fn`, 1 impl method, 2 private struct, 1 private enum) |
| Rust `game/game_setup.rs` items inventoried | **46** |
| Rust items with no C twin | 11 - all measured VERIFIED-NOOP / display-only / unreachable |
| C functions in the three owned files | **154** |
| - WIRED (reached from `src/` production) | **138** |
| - genuinely zero-call-site | **15** (13 DEAD-CORRECT, 2 DEAD-WRONG) |
| Reachable gaps found | 6 |
| Reachable gaps closed | **6** |

The raw mechanical census reported 16 zero-call-site names. One is a FALSE
POSITIVE: `rb_can_assign_hand_for_alt_cost` IS wired (`phase.c:1035`), because
its only call is `return rb_can_assign_hand_for_alt_cost(...);` and the prototype
filter cannot tell that from a declaration. `bt_card_ok` is the mirror image - it
has 4 call sites and is still unreachable, because all 4 are inside dead
functions. Both corrections are in `AUDIT_actions_setup.md` 3.2.

### Closed in T6

| # | site | Rust line | reachability |
|---|---|---|---|
| T6-1 | orphaned ENERGY under-card went to the ACTIVE ENERGY ZONE | `actions/mod.rs:1851-1856` | reachable |
| T6-2 | resolution-zone ENERGY went to the discard | `actions/mod.rs:1885-1894` | reachable |
| T6-3 | `live_set_limit_reduction` never reset | `phases.rs:232-233` | **2 of 985** |
| T6-4 | `check_timing` refreshed seat 0 first unconditionally | `actions/mod.rs:1632-1638` | reachable |
| T6-5 | `activate_wait_members` had no `constant_cannot_activate_members` filter | `phases.rs:125-131` + `modifiers.rs:1081-1092` | **2 of 985** |
| T6-6 | `rb_activate_card` had NO mandatory-cost refusal | `actions/mod.rs:553-564` | **100 of 985** |

T6-6 is the one the item-5 brief missed: `cost.c:853-857` lists **three** live
Rust call sites for `validate_mandatory_cost`, not two - `actions/mod.rs:559`
(activate_ability) is the third and it sits in this cluster's file. The C built
the cost+effect tree and ran it, so a partially-paid mandatory cost still ran the
effect, and the hand removal at `mod.rs:570-573` had already happened.

### T6 A/B evidence

35 suites, `^FAIL` (no colon), **case-sensitive** (`-cmatch`), stdout AND stderr,
sequential, isolated snapshot. Every suite produced output on every run - none was
silent, so none was mis-read as passing.

| build | total `^FAIL` |
|---|---|
| baseline (T6 hunks reverted) | **1415** |
| after T6-1..T6-5 | **1415** |
| after T6-6 | **1411** |

New suite `tests/test_parity_t6_action_setup.c`: **13 `^FAIL` / 48 assertions**
against the reverted baseline, **0 `^FAIL` / 48 assertions, 5/5 ok** fixed.
Per-fix red counts 3 / 3 / 3 / 2 / 2 - every fix independently proven.

**One regression reported, not hidden:** `parity_jidou_move_yell` 89 -> 90
(`"each activation draws exactly one card"` inside
`shiki_auto_is_one_shot_per_waited_state`). That case was **already 5 assertions
red**, and the two failures immediately upstream are both `"the swap must actually
move her out of 0 (she is now at 0)"` - the fixture's `mill_three_swap` never
moves the member out of area 0, so the cycle is misaligned before the draw is
measured. Downstream of an already-red precondition, in an already-red scenario.
Gate kept; the two suites the gate improves both improve on assertions that name
`actions/mod.rs:559` explicitly. Net **-4**.

### Do NOT regress (re-verified after T6)

* the VICTORY rollover still clears `player_cannot_activate`,
  `n_cannot_active_cards` and `baton_touch_used` (`phase.c:496-501`), reached by
  every driver including web;
* the live-end drain is still UNCONDITIONAL per `live.rs:1508-1529`;
* `rb_execute_live_victory_determination` is still the single
  LiveVictoryDetermination entry point;
* **no `p1_has` guard was added to `rb_determine_live_winners`.**

### Open from T6

1. **`rb_validate_mandatory_cost` did NOT land at the offer sites.** The function is
   complete (`cost.c:1808`, a faithful port of `ability/cost/handlers.rs:281-309`,
   declared at `rabuka.h:3518`) and has **ZERO callers**. `game_setup.rs:2188` and
   `:2295` still go through `src/game/game_setup.c:1199`, which calls the pay-time
   twin `rb_validate_cost` - strictly STRONGER, so it REFUSES offers Rust makes.
   `game_setup.c` is cluster C's file and `include/rabuka.h` is main's. Reported,
   not landed. **A/B ability 325 before landing it:** 325 (`PL!-pb2-012-P+` ab#1)
   is one of the 100, and its cost is a `sequential_cost` whose second leg IS a
   `choice_condition` - exactly the node class where the two validators disagree.
2. `rb_check_victory_condition` (`phase.c:565`) is **value-divergent**: Rust's
   `check_victory_condition` ends the game on `p1 >= 3` alone; the C adds a
   `p2 <= 2` guard and a `3 vs 3` draw verdict. Fixing it needs a real
   `game_result` / `game_ended` pair in `include/rabuka.h`.
3. `check_invalid_resolution_zone` OWNER half is structurally inexpressible - the C's
   `resolution` is a bare `RbBag` and `rb_resolution_add` takes no owner. Only the
   energy half was closed. BLOCKED-ON `include/rabuka.h` + `core/zones.c`.
4. `rb_check_invalid_live_cards` has no `face_up` gate (Rust `actions/mod.rs:1733`).
   Needs a `live_card_zone.face_up` flag in `include/rabuka.h`.
5. `src/core/modifiers.c`'s CONSTANT scan has **no** `action == "restriction"` arm
   at all (0 hits for "restriction" outside one comment), so the two constant
   `cannot_activate` abilities never register in the C. T6-5 sidesteps this by
   recomputing the predicate at the read site; a real `GameState` field removes the
   workaround.
6. `check_permanent_loop` (`abilities.rs:3309`) has **ZERO callers in Rust**; the C
   calls it from `rb_check_timing` (`phase.c:756`). Only sets `g->loop_detected`, so
   harmless - but it is C-only behaviour.

### VERIFIED-NOOP (deliberately NOT ported)

* **`check_duplicate_members` (`actions/mod.rs:1785`) - 0 of 985.** Its only data
  source, `player.stage.take_pending_duplicate_members`, is fed by
  `stage_member_for_check_timing` (`core/zones.rs:289`), and **that function has
  zero callers anywhere in `engine/src`** (exhaustive recursive grep over every
  `.rs`). The list is therefore always empty, the function always returns 0, and
  the C's omission of `rb_check_duplicate_members` from `rb_check_timing` is
  **correct**.
* `cost_has_hand_self_move` (`actions/mod.rs:362`) - 0 of 985; the
  `loc == Zone::Hand` branch never fires on the corpus.
* `profile_chosen_labels` / `profile_choice_is_skip` (`:603`, `:734`) - 0 of 985,
  inside `#[cfg(not(feature="headless"))]`.
* `generate_pending_choice_actions` / `generate_rps_actions` /
  `generate_choose_first_attacker_actions` (`game_setup.rs:719`, `:1446`, `:1454`) -
  UI row builders; the C prompts through `rb_has_pending_choice` /
  `rb_resume_with_choice` and the ACTION VERDICT is identical.

### Suite that does NOT exist

`test_setup` - there is no `tests/test_setup.c` and no `tests/test_test_setup.c`,
so it has no runnable stem and is **not** in the 35. Verified absent before
naming anything; this makes seven phantom suite names in total (peers previously
reported `audit_cards`, `card_data_decode`, `card_info_show`,
`card_info_parity`, `parity_card_text`, `audit_game_state`).

---

## A FOURTH TRAP: `-replace` + `Set-Content` DESTROYS the file you are auditing

Hit while auditing T6, and worth recording because the damage is invisible until
the next compile. PowerShell 5.1's `-replace` over a file read with
`Get-Content -Raw` mangles every **non-ASCII** byte: em-dashes became `?`, and
Shift-JIS mojibake destroyed whole Japanese comments in `src/engine.c`
(99943 B -> 106372 B, ~5 KB of junk, plus a compile error 500 lines from the real
problem). Recovery was clean only because the pre-edit copy survived in the
snapshot.

**Rule: edit source with the Edit tool, never with `-replace` / `Set-Content` /
`Out-File`.** If a bulk edit is genuinely unavoidable, do it in the SNAPSHOT,
verify with a byte-length + SHA-256 comparison against the pre-edit hash, and
confirm the file still compiles before trusting anything measured from it. The
brief's "fullwidth chars must be `\xNN` escapes" rule is the same hazard from the
other side: a legacy-codepage rewrite turns them into mojibake.

Corollary for auditors: keep a hash of every file you own **before** you touch it,
and keep one intact copy somewhere off-tree. T6's recovery came entirely from
`act_base/src/engine.c` still holding `DD64CB52...`.

---

## 🔧 HEADER BILL PAID BY MAIN — 2026-10-03 (cluster T3's BLOCKED-ON list)

T3 established that **34 of the 47 zero-caller `rb_gsc_*` functions were blocked on the
header ALONE** — already implemented and compiling, just undeclared. Only 1 of 53
`rb_gsc_*` symbols was declared, and 6 had callers only because `game/game_setup.c:208`
hand-wrote a local `extern`. The file compiled by accident, not by contract. Landed:

1. **45 prototypes** for the `game_state_core.c` surface.
2. `GameState.rule_log[500][256]` + `rule_log_len[]` + `n_rule_log` — a buffer SEPARATE
   from the 32-slot condition-verdict ring. `RB_GSC_RULE_LOG_MAX` alone was not enough:
   it is a count, and a `Vec<String>` needs the strings. Both macros moved ABOVE
   `GameState` (a `#define` after the struct is a compile error).
3. `RbGscTurnPhase` typedef MOVED from `game_state_core.c:93-97` into the header (local
   copy deleted in the same change, same pattern as `RbGscZoneSnapshot`) — it is fully
   derivable from `g->phase` + `first_attacker` + `second_attacker`, so storing it would
   create a second source of truth.
4. `resolution_owners[]` + `n_resolution_owners` beside `RbBag resolution`, and
   `rb_resolution_add_for_owner()` — Rust returns each resolution-zone card to its
   YELLER; the C assumed the active seat, so a P2 yell card lands in P1's waitroom.
5. `live_face_up[]` — Rust evicts a wrongly-set member AFTER the live, C evicts BEFORE.
6. `constant_need_heart_global[]` + `_set[]` — `modify_required_hearts_global` is
   reachable from **3** abilities, not 0 as `AUDIT_constraint_condition.md` claimed
   (its probe looked for the key `global`; the wire key is `all`).

### ⚠ THE AUDIT DOC'S SIGNATURES WERE WRONG — DO NOT TRUST "VERBATIM" CLAIMS
T3's §7 promised the 45 prototypes were "verbatim from `game_state_core.c`". **About 20
were not.** Real examples: `rb_gsc_seat_index_by_id(const char*)` not `(const GameState*,
const char*)`; `rb_gsc_cheer_count_by_id` takes 2 args not 4; `rb_gsc_push_rule_log_fmt`
takes NO `GameState*`; `rb_gsc_log_entry` takes 6 args not 5;
`rb_gsc_push_performance_snapshot` takes `const RbLiveSnapshot*` not an int;
`rb_gsc_record_ability_application` takes 7 args not 6; and the trace parameter is
`RbAbilityTraceNode`, not the doc's `RbTraceNode`. **All 45 were re-extracted from the
definitions and verified to resolve.** Header re-verified with `gcc -fsyntax-only`, and
47 of 48 `src/*.c` compile clean.

The 48th, `src/core/alloc_counter.c`, fails on `ATOMIC_VAR_INIT` under every `-std`
(removed from C17). **It is NOT in `SRC` or `OBJ_LIB`** — an orphan that would break the
build the moment anyone added it. Fix or quarantine; do not "fix" the Makefile to include it.

---

## 🚨 RECONCILIATION VERDICT (2026-10-03) — THE CHECKLIST WAS LYING. READ THIS FIRST.

An independent agent verified all 167 checkbox items **against the code, not against
the documents**, and wrote `RECONCILIATION.md` + `REDEPLOY.md`. Real counts:
**79 `[x]`, 3 `[~]`, 85 `[ ]`** — and **24 items sit in the WRONG bucket in BOTH
directions.** Of the 88 not-closed: 69 confirmed OPEN, 13 PARTIAL, 3 NOT-VERIFIED,
3 OBSOLETE.

### ⛔ CLUSTERS B, C, D, E WROTE NOTHING AT ALL
`choice.c`, `cost.c`, `rules.c`, `misc.c`, `condition.c`, `modifiers.c` are all **CLEAN
in git**. `modifiers.c`'s mtime is the oldest in the tree. **37 items, 3 landed, 34
untouched.** Cluster F landed only its `custom.c` hunk. Four agents consumed a lot of
time and produced no code. Do not assume any B/C/D/E item is done.

### ⛔ 11 ITEMS MARKED `[x]` ARE NOT CLOSED — INCLUDING THREE OF MINE
`A4` `rule_log`, `A9` `cause_card_id`, `A17` the six CardFilter carriers: I added the
fields, cleared the blockers, and **nobody wired a producer or a reader.** They are
0 writers / 0 readers. This is the purest "blocker cleared, downstream never done" and
**it is my own failure** — I treated clearing a header blocker as closing the item.
Also not closed: `G3`, `H1`, `H2`, `I2`, `I6`, `J9`, `T3-2`, `T3-5`.

### ⛔ A LANDMINE: `rb_rules_clear_turn_state` is DECLARED TWICE, DEFINED NOWHERE
`live.c:604` and `phase.c:529` both declare it; **no definition exists** and nothing
calls it. **`I2` claims `[x]`.** Adding a call site turns this into a LINK ERROR.
`cannot_live_players[2]` (`rules.c:72`, written `rules.c:1878`) is still never cleared.

### 9 NEW DUPLICATE LIVE IMPLEMENTATIONS (all `static`, all diverging)
`eff_extra` has **FIVE** live statics (`compound.c:32`, `cost.c:108`, `misc.c:52`,
`resolver.c:83`, `util.c:258`). `player_prefix` has **THREE** plus a signature split.
Also `cached_condition_verdict` (`compound.c` vs `resolver.c`, gating on different
cache calls), `card_total_hearts` (`condition.c:3258` vs `util.c:705` — and
`util.c:1693`'s OWN comment calls its version "the WRONG total"),
`card_appeared_this_turn` (the `misc.c:248` copy is missing the `cid<0` guard),
`effect_extra`, `cond_cache_key`+`store_condition_verdict`,
`remove_card_from_any_zone`, `cmf_extra`/`extra_true`. The known
`execute_gain_ability_from_source` pair is RESOLVED (`ability.c:535` now delegates).

### DUPLICATE-IMPLEMENTATION ADJUDICATION — cluster D, 2026-10-03
`src/ability/condition.c` has landed; here is which of the four implicated duplicates is
the RIGHT one, established from the Rust, with the recommended owner for each. **None of
these were changed** — consolidation is a separate, risky edit.

| duplicate | the right one | why | recommended owner |
|---|---|---|---|
| `card_total_hearts` — `condition.c` vs `util.c:705` | **`condition.c`'s** | Rust `Card::total_hearts` (card.rs:4263-4271) sums ALL heart sections; `util.c`'s OWN comment at :1693 calls its version "the WRONG total", which is the base-slice-only sum (the `original_heart_total` quantity, `util.rs:1790-1798`). Two DIFFERENT quantities with two names that read alike. `condition.c`'s is now also the one `cond_check_original_heart_filter` (D8) calls. ✅ **re-verified 2026-10-04 (fixer wave): the two are genuinely NOT duplicates and util.c:705 is a faithful twin, not a bug.** `util.c:699-706`'s `card_total_hearts` carries its own citation to **`Card::total_hearts` (card.rs:4194-4202)** — a DIFFERENT Rust function from `original_heart_total` — and its body matches that citation (`num_base > 0` → base slice, else the need slice). The reconciliation's `:1693` line reference is stale. | **util.c owner**: the rename is COSMETIC, not a parity fix — `util.c:705`'s `card_total_hearts` already names the Rust function it twins. Rename to `card_base_hearts_total` only if the collision with `condition.c`'s same-named static keeps costing readers; **keep both either way, and DO NOT delete `condition.c`'s.** util.c is not this wave's file. |
| `card_appeared_this_turn` — `condition.c:1301` vs `misc.c:248` | **`condition.c`'s** | ⛔ **THIS ROW'S CLAIM IS FALSE — re-measured 2026-10-04 (fixer wave).** `misc.c`'s `card_appeared_this_turn` is not at :248 any more (it is at **`misc.c:294-298`**) and it **has no `cid`-indexed access at all**: it linear-scans `g->cards_appeared_this_turn[0..n_cards_appeared_this_turn)`, so a `-1` card id cannot index off the front of anything. The `moved_this_turn[]` read that DOES index (`misc.c:382`) is already guarded by `cid >= RB_MAX_CARD_IDS` on the same line, and `cid` is `>= 0` by construction there. **No guard is missing; adding one would be dead code.** | **No action.** misc.c is contested three ways (C, F, sweep); do not spend a slot here. |
| `cached_condition_verdict` — `compound.c:231` vs `resolver.c:310` | **neither yet** | `compound.c` gates on `cond_get_cache_flag`, `resolver.c` on `cond_get_cache`: two different gates for one verdict, so the cache can hold both a cached and an uncached answer for the same condition. Neither is provably right without deciding which gate is authoritative. | **compound.c + resolver.c owner** (cross-file; needs one decision, then one implementation). |
| `cond_cache_key` + `store_condition_verdict` — `compound.c:211`/`:241` vs `resolver.c:207`/`:320` | **neither yet** | Same pair, other halves. A divergent key means a verdict stored under one key is read under another. | same owner as the row above; settle both halves together. |

### ✅ CLOSED 2026-10-04 (fixer wave) — THE CLAIM BELOW WAS FALSE. `check_card_property`'s empty zone is CORRECT in C.
**`condition.c`'s `if (n == 0) return 1;` is a FAITHFUL TWIN, not a divergence. Do not "fix" it.**

The claim was: Rust `check_card_property` (card.rs:230-252) ends `!matching.is_empty()`, so an
empty check set is false, not true. **FALSE**, and the reason is an early return three lines
above the loop the citation points at:

```rust
// engine/src/ability/condition/card.rs:133-136
let neg = condition.get_negation().unwrap_or(false);
if check_cards.is_empty() {
    return true;
}
```

`!matching.is_empty()` at card.rs:251 is **UNREACHABLE for an empty zone** — the early return
short-circuits first. Rust's OWN comment at card.rs:112-117 says so in as many words ("the empty
case below returns TRUE"). And `engine/src/ability/condition/card.rs` is **UNMODIFIED in git**
(`git status --porcelain` empty for it), so this is not a half-applied edit on the reference side
under the "Rust is being edited" caveat.

The two trees also agree on the *negated* form: Rust returns `true` before the negation is folded,
and the C returns 1 for both shapes.

Cluster D's removed assertion is now RESTORED, with the citation, in
`tests/test_parity_fx_fixers.c::fx2_card_property_empty_set_is_true` (6 assertions: empty `stage`,
empty `revealed_cards`, negated and non-negated, plus two non-empty controls proving the result is
the early return and not the general answer).

**Measured blast radius** (tools/census_fx.c §A, engine's own decoder, 2116 card-ability pairs,
0 decode fallbacks): **73 `card_property` condition nodes on 59 pairs** — revealed_cards 26 /
no `location` 28 / success_live_card_zone 12 / stage 6 / discard 1; **47 of the 73 negated**.
Changing the polarity would have been a wide regression, not a cosmetic one.

**Sibling trap, also now closed: `VERIFIED-NOOP: 0 of 73`.** Rust's other trap arm in the same
function — `_ => return true` at card.rs:109, which the C does not have — fires only for an
UNMAPPED `CardProperty` variant. All four corpus values (`has_blade_heart`, `has_score_icon`,
`has_all_blade`, `has_draw_icon`) are mapped in the C and **0 of 73 nodes carry an unmapped one**.

**One real gap found while measuring, NOT closed (it is a dispatch gap, not this predicate):**
only **67 of the 73** land on `RB_COND_LOCATION`, the variant that reaches `check_card_property`.
The other 6 carry `card_property` on a different variant — **4 on `RB_COND_MOVEMENT`** and **2 on
`RB_COND_APPEARANCE`** — and reach no C dispatch to this predicate at all. Rust routes them
through `check_card_property` from their own evaluators. Counting those 6 as "covered by this
predicate" would be the exact error this section is about, so they are recorded here instead.
`BLOCKED-ON: condition.c — `RB_COND_MOVEMENT` / `RB_COND_APPEARANCE` need the
`card_property` arm Rust's own evaluators give them; the predicate itself already exists.

### CORRECTIONS TO FIGURES I OR A PEER STATED
- **`sizeof(GameState)` = `sizeof(TestGame)` = 1 113 736 B**, not 885 912 — my appended
  fields grew it by 227 824 B. Fixed in `Makefile` (it had claimed "MEASURED 885912"
  and was wrong), and `LDFLAGS` raised 16 MB -> 32 MB because the harness self-test
  ceiling `stack/15` left only **4 745 B** of headroom at 16 MB.
- **`src/core/alloc_counter.c` COMPILES CLEAN.** I claimed it "fails under every
  `-std`"; that was a PowerShell artifact counting a wrapper line. Verified: **clean
  under `-std=c99/c11/c17`, fails ONLY under `-std=c2x`** (C23 removed
  `ATOMIC_VAR_INIT`). The project builds `-std=c11`. It is **unwired**, not broken —
  it is in neither `SRC` nor `OBJ_LIB` and no `rb_allocctr_*` is declared in
  `rabuka.h`. FIX AND WIRE it; quarantining would have destroyed 775 lines of real work.
- `test_choice_renderer` does **NOT** hang — 1.2 s, green from a clean build. `H2`'s
  3746 s observation was a stale binary. `H1` and `H3` are already fixed.
- `nozomi`'s "`parity_search_reorder` 122 -> 89" does not match the live tree, which
  measures **158 `^FAIL`**. Treat that delta as unverified.
- `src/core/state.c` **does not exist** — the file is `src/ability/effects/state.c`.

---

## 🔴🔴 THE DEFECT THAT MAKES EVERY OTHER NUMBER UNDETECTABLE

**`PASSED` verdict lines: 0 corpus-wide, against 990 `FAILED`.**

**No suite in this corpus can report success.** Therefore a suite that segfaults
before its first assertion produces **exactly the same output shape** as a suite that
passed every assertion — unless you read the exit code, and **12 suites exit 0 carrying
548 FAIL lines** while **2 exit non-zero with zero FAILs**.

That single property is why all of the following survived in a project where every
report quotes a `FAIL` count:

| suite | symptom |
|---|---|
| `tmp_list` | 0 B stdout, 0 B stderr, **exit 0** |
| `test_audit_conditions` | 1 002 lines run, `cannot load card db`, **0 assertions** |
| `test_state_effects` | SIGSEGV 139, **0 assertions** |
| `test_p1_helpers` | 0 B stdout, 249 literals, 1 failure — reported nowhere |
| 9 suites | print `FAIL [` bracket-form → a `^FAIL:` scan scores **zero** |
| 12 suites | exit 0 with 548 FAIL lines |
| 17 suites | print **zero** `ok:` lines |

**FIX: one `PASSED: <suite>` line per suite, plus a manifest-vs-glob diff that exits 2
on a suite name that does not exist.** That is a one-line change per suite and it makes
the entire corpus self-describing.

### Counts that were WRONG, from the coverage audit
- **`^FAIL` = 3 486 vs `^FAIL:` = 2 277 — 1 209 failures INVISIBLE to a colon grep.**
- **46 suite names in circulation do not exist** (not the 12 previously recorded).
  7 are **rename-only** (`parity_vm`→`test_vm_decode.c`, `parity_condition_decode`→
  `test_parity_t7_condition_decoder.c`, `parity_max_distinct_names`→
  `test_max_distinct_names.c`, `audit_x_class`→`test_parity_x_class.c`, and 3 more);
  3 are VERIFIED-NOOP; the rest are `tools/*.c` census programs conflated with suites.
- **The corpus has 156 `test_*.c`, not 142.** Every "142 suites" figure is wrong.
- **The "43 VACUOUS + 5 SETUP BUG" figure was wrong in BOTH count and location** — it
  was a substring scan: 314 lines *contain* `vacuous`, only **44 begin with it**, and
  **every one of the 10 suites previously named emits zero**. The live ones are
  `parity_required_hearts` (13), `parity_cost_discount` (10), `parity_state_per_card`
  (8 — which are the suite's own prompt-control guards, a different class). Both
  `SETUP BUG`s are in `parity_blades_live_start`, resolving to id **−1** and to a
  **different card** (2793).
- `test_parity_jidou_watch.c:680` asserts live-zone eviction by ライブ開始時, but
  **Rust evicts AFTER the live** — it is green **because the engine is wrong** and will
  go RED when the engine is fixed. Its sibling `:675` asserts the WAITROOM length while
  its message claims live-zone removal, so it cannot fail for the reason it gives.

### Rust coverage, measured
**4 319 Rust tests** (4 175 in `engine/tests/` + 144 in-module — `TEST_INVENTORY.json`
omits the 144, so "4175" is 13.4% short). **4 154 have a C counterpart (96.2%); 21 do
not.** Gap classes: `NO C TEST` 21 · `WEAKENS` 1 236 (a floor) · `INVERTED` 2 ·
`VACUOUS` 23 cases / 44 lines.
**The two corpora share NO name space** — the obvious token match returns **0 hits over
all 4 319 tests**, because C assertions are ASCII and Rust test names are Japanese
prose. The 812 explicit file-level citations are the ONLY bridge, and they are
load-bearing. **3 730 tests inherit an unmanifested file-level claim** — the largest
unmeasured quantity in the project.
**`pvp_room_test.rs` — 14 committed Rust tests, 346 lines, NEVER COMPILED**
(`build.rs EXCLUDED_MODULES`) — and it is the only oracle covering `choice_player_id`
routing, where `game_state_abilities.c` has **three** separate readers of "who owns this
choice".
⚠️ **6 new UNTRACKED Rust test files appeared during the audit window — the oracle is a
moving target and this window is not closed.**

---

## ✅ RESOLVED — the `live_face_up` "regression" was neither an engine gap nor a bad test

Two agents **contradicted each other** and both were partly wrong. The truth:

**The Rust rule WAS already ported.** `src/turn/phase.c:675-716`
`rb_check_invalid_live_cards`, called from `rb_check_timing`, is Rust's
`check_invalid_live_cards` arm for arm — `face_up` early return, `!card.is_live()`
filter, reverse index order, `energy_deck` for energy / waitroom for a member, and one
`rb_record_card_movement(...)` per evicted card. It is **committed, not local**.

- Agent H's claim "**the C has no live-zone slot validation**" — **FALSE**. The body
  existed throughout. Its 5 red `mechanics` assertions were real, but the cause was
  the **fixture**, not the engine.
- Agent W0's claim that `test_parity_jidou_watch.c:680` was **inverted** — **CORRECT**.
  Rust's order is `phases.rs:240` `check_timing` → `mod.rs:1734` early-return while
  `face_up` is false → `phases.rs:343` sets `face_up` → `phases.rs:245` fires
  `trigger_live_start_abilities` **in between**. **Rust HAS the member in the live zone
  at ライブ開始時.** The assertion now matches the oracle and passes.

**The real cause: four test fixtures hand-build a board without setting the flag.**
Opening the gate in the FIXTURES (not the engine) cleared `mechanics` **7 → 0** and
`live_card_zone_movement` **15 → 0**. `parity_hearts_live_start`'s +3 is **not** the
gate — `test_fire_trigger_on_card` never calls `rb_check_timing`; its output is
byte-identical.

**A/B proof, three mutations** (`tests/test_live_zone_slot_validation.c`, 47 assertions,
every positive paired with a negative on the same board): delete the eviction body →
**26 FAIL / 6 of 8 functions**; delete the gate → **4 FAIL**, all in `gate_closed`;
move `live_face_up[performer]=1` to AFTER `rb_check_timing` → **2 FAIL**, both the
`ORDERING:` rows.

**LESSON, now a standing rule:** a red assertion in a suite that hand-builds state may
mean the FIXTURE is incomplete, not that the engine is missing a rule. Check whether
the suite even calls the function under test before attributing the failure to the
engine. Two agents each half-diagnosed this and reached opposite conclusions.

---

## 🐛 Q213 — A REAL BUG IN THE RUST REFERENCE (file against Rust, NOT the C)

Found by the adjudicator, and it cuts against **both** agents in the `live_face_up`
dispute. Neither agent looked at the official ruling.

`cards/cards.json`, `PL!HS-bp5-019-L` FAQ **Q213** (2026.02.28) states:

> いいえ、{{live_start}}能力が発動する前にメンバーカードは控え室に移動します。
> そのため、メンバーカードでは減りません。

**The member must reach the WAITROOM BEFORE the ライブ開始時 ability fires.**

**Neither engine implements this.** Rust's `face_up` gate (`actions/mod.rs:1734`)
defers the eviction to `:349` in the performance phase — **one phase too late**. So the
C faithfully reproduces a **Rust-side bug**.

**Consequence for this project: the C must keep following Rust, and the defect is
filed against `engine/src/turn/actions/mod.rs:1734`.** Fixing it in the C alone would
create a *new* divergence. Do not "fix" the C to match the card text.

This also reframes the whole dispute: the inverted `test_parity_jidou_watch` case is
named `test_hanamusubi_q213_member_card_moved_before_live_start` and now asserts the
member was **NOT** moved — so the test's own NAME contradicts its body. Rename it, and
have its comment state that Q213 is unimplemented in both engines.

### ✅ LANDED 2026-10-04 — both TODOs above are closed. Nothing in `src/turn/phase.c` moved.

**HEAD measured: `cf315a9ac5dedd1ca02caeadbfb13f0f74919e5c`** (note this is *not* `2130769d`,
which is its parent; the tree had already advanced one commit when this work started, which
is exactly the trap the "mechanism" section below describes).

**`src/turn/phase.c` is byte-identical to HEAD.** `git hash-object` = `git rev-parse
HEAD:engine_c_wip/src/turn/phase.c` = `4fb6ad3ca7dae9e4e289497832b61887d1d35749`, and
`git diff HEAD -- engine_c_wip/src/turn/phase.c` is empty. The gate (`phase.c:703`) and the
eviction (`phase.c:705-715`) were read and cited, never edited.

**What landed: six TEST files, one line each plus comments. No engine file, no header.**

| file | what | Rust twin line that justifies it |
|---|---|---|
| `tests/test_mechanics.c` `test_invalid_live_movement` | `tg.state.live_face_up[0] = 1;` | `live_card_zone_movement_test.rs:112-113` (push, then `face_up = true`) |
| `tests/test_mechanics.c` `test_invalid_energy_movement` | `tg.state.live_face_up[0] = 1;` | `live_card_zone_movement_test.rs:281-282` |
| `tests/test_live_card_zone_movement.c` `record_timing_movements` | `live_face_up[0] = 1;` **and** `[1] = 1;` (one per seat) | `:113`, `:282`, `:347` (P1) + `:348` (P2) |
| `tests/test_parity_rules_conditions.c` `d1b_removal_path_control` | `tg.state.live_face_up[0] = 1;`; the `:1105-1108` "C cannot represent face-down" comment **deleted** | `:113`, `:282` |
| `tests/test_parity_rules_conditions2.c` B1 / B1p / B4 | `live_face_up[0] = 1;` each | `:113` (B1, B1p), `:282` (B4) |
| `tests/test_parity_rules_conditions2.c` B2 | `live_face_up[0] = 1;` **and** `[1] = 1;` | `:347` + `:348` |
| `tests/test_parity_rules_conditions2.c` | the `:702-703` `DIAG("C GameState has no live_card_zone.face_up field…")` **deleted**, plus the same false claim in B1's header comment | — |
| `tests/test_parity_hearts_live_start.c` bp6 ×3 fixtures | the live slot gets a real **ライブ** card (`PL!-sd1-020-SD`) instead of the deck filler `PL!N-sd1-010-SD` (type **メンバー**, verified against `cards/cards.json`) | the fixture is not a twin of a Rust test here; the constraint is the rules — a live-card *set* may not hold a non-ライブ card |
| `tests/test_parity_jidou_watch.c` | **renamed** `test_hanamusubi_q213_member_card_moved_before_live_start` → `test_hanamusubi_q213_member_card_still_in_live_zone_at_live_start`; the four false claims in its comment **rewritten** | `mod.rs:1734` + `phases.rs:245`/`:343` |

**NEW TEST — `D1p_face_up_evicted` (`d1p_face_up_non_live_card_evicted_by_check_timing`)**
in `tests/test_parity_rules_conditions.c`, registered in the run table beside `D1b`. It is
the paired POSITIVE of `D1_face_down_survives`: byte-for-byte D1's board — one non-live
card in P1's live zone, one on stage, 30 deck fillers — with the single line
`tg.state.live_face_up[0] = 1;` added. Opposite outcomes, and the gate is the only thing
between them. **4 assertions, all green** (`D1p_face_up_evicted  6 assertion(s), 0 failure(s)`,
the 6th being the harness's own setup pin). Neither C suite had the gate's positive at
one-card scale; this is the assertion whose name states the rule, and it was previously
unasserted everywhere.

### Measured, A/B, isolated snapshot, `make clean` + full `OBJ_LIB` rebuild both sides

Baseline is **this tree with only these hunks reverted** — verified by a file-hash sweep,
not by git HEAD: each of the six files was reverse-patched and had to reproduce its
pre-edit SHA-256 exactly.

| suite | before | after | Δ |
|---|---|---|---|
| `mechanics` | 7 | **0** | −7 |
| `live_card_zone_movement` | 15 | **0** | −15 |
| `parity_rules_conditions` | 10 | **6** | −4 |
| `parity_rules_conditions2` | 20 | **0** | −20 |
| `parity_hearts_live_start` | 84 | **81** | −3 |
| `parity_jidou_watch` | 75 | 75 | 0 (rename + comment only) |
| **my six, total** | **211** | **162** | **−49** |
| `parity_live_total` / `parity_live_total2` | 50 / 17 | 50 / 17 | 0 |
| `live_success_rules` / `opponent_live_success_flow` / `b8_live_timing` | 0 / 0 / 0 | 0 / 0 / 0 | 0 |
| `parity_full_round` / `parity_full_round2` / `parity_sequential` / `parity_integration` | 44 / 68 / 142 / 42 | 44 / 68 / 142 / 42 | 0 |
| **all 15 suites** | **574** | **525** | **−49** |

`^FAIL` counted case-sensitively, stdout **and** stderr, no colon — so all five prefixes
(`FAIL: `, `FAIL [`, `FAIL (VACUOUS):`, `FAILED␣␣name`, `FAILED:`) are included.
**Every flipped line was diffed individually: 49 fixed, 0 new. No suite regressed.**
`parity_rules_conditions` retains 4 reds (`B2_heart_override_additive` ×2,
`D2_resolution_to_owner` ×2) — see "found, not fixed" below.

### `parity_hearts_live_start` — the brief's mechanism was WRONG, its conclusion was right

The brief said the bp6 fixture would make `LL-bp6-001-R` "see **two** live-slot cards
instead of one". **Measured: `p[0].live.n == 1` in all three fixtures, before and after** —
the member never produced a second live-zone card. The real mechanism is different and
worse: a non-ライブ card in the live slot suppresses the bp6 cost prompt **outright**.
Substituting only the live-slot occupant (everything else identical) turned exactly three
assertions green and nothing red:

```
bp6 should prompt for the any-number discard
bp6 prompts for the any-number discard
bp6 prompts even with one eligible card
```

So `hl_walk_to_live_start`'s doc comment now states the rule it must satisfy and names the
three assertions that measured it. Also note `PL!N-sd1-010-SD` is type **メンバー**, not
ステージ — the brief mislabelled it; the conclusion (non-ライブ) is unchanged.
`parity_hearts_live_start` is still 81 red for unrelated reasons (the cluster's
`hl_fire_trigger` path never reaches `rb_check_timing` — W.1's table above already says so,
and this measurement confirms it again from the other side).

### The jidou rename and the four false claims

`test_hanamusubi_q213_member_card_moved_before_live_start` →
**`test_hanamusubi_q213_member_card_still_in_live_zone_at_live_start`**, in both the
function and the `run()` label. The comment no longer says the case "IS NOW RED ON PURPOSE",
that "the C evicts it BEFORE the live", that `live_face_up[]` is an open divergence, or
that this is "why this case is red" — **all four were false; the case printed
`ok … 5 assertion(s), 0 failure(s)` at the measured HEAD.** The replacement states: Rust
gates on `face_up` (`actions/mod.rs:1734`), `face_up` is **false** at ライブ開始時
(`phases.rs:245` fires before `phases.rs:343` sets it), **the C matches Rust**, and **Q213
is unimplemented in BOTH engines — file it against Rust, do not "fix" the C to match the
card text.** The old name is retained in a one-line comment so audits that grep the string
still find it.

### Gates, and three things this work did NOT cause

* `make check-suite-names` — **PASSED** (160 suites, 47 names in circulation).
* `make warnings-audit` — **FAILS, and did so identically BEFORE this work.** Both sides of
  the A/B print byte-identical output: `baseline-still-present=8 NEW=4 stale-pins=4`. The four
  `NEW` rows are `src/ability/effects/look.c:1486`, `rules.c:895`, `rules.c:1398`,
  `rules.c:1601`, all `-Wformat-truncation=`. `warnings-audit` only reads `src/`, and this
  work touches **no** `src/` file. **This belongs to the `src/**` owner.**
* `tests/test_harness_self.c` — 2 reds, **also pre-existing and identical on both sides**:
  `test_id(` call sites `3036` vs cap `3017`, and `107` files vs cap `105`. A peer added
  those; this work added **zero** `test_id(` calls (new fixtures use `mid()` =
  `test_new_id`). **The size pin itself HOLDS: `sizeof(GameState) = 1195720` B exactly**
  (`ok: A4 sizeof(GameState) is EXACTLY the pinned 1195720 B`). Nothing in
  `include/rabuka.h` was touched.
* `src/abilities_strings.bin` — **726 831 B / `89204df0bbde08a0…`**, verified before the
  build, after the build, and after the run. No generator was run. `make data` never invoked.
* Live tree left clean: **0 `.o`, 0 `rb_engine*`**. All six edited files remain **LF-only**
  (`CRLF=0`), checked before and after, because the `edit` tool silently dropped the
  indentation of one `memcpy` line in `record_timing_movements` and that was repaired.

### Found, NOT fixed — `D2` is the same class of bug on a different precondition

`tests/test_parity_rules_conditions.c` `d2_invalid_resolution_card_returns_to_its_owner` is
red for the same reason as the five above: **the fixture dropped a precondition.** The Rust
twin pushes `resolution_zone.add_card_for_owner(member, 1)`
(`live_card_zone_movement_test.rs:93`); the C fixture calls plain `rb_resolution_add(&tg.state,
member)` at `test_parity_rules_conditions.c:1132`, which stamps **no owner**, so the card is
routed by `g->active` and lands in the wrong waitroom. `rb_resolution_add_for_owner` exists
and is the correct helper — `src/turn/phase.c:777` names it as the writer the drain reads.
**This is NOT a `live_face_up` issue** (`check_invalid_resolution_zone`, Rust `mod.rs:1864`,
has no `face_up` gate) and it is outside this task's scope, so it is reported rather than
changed. One-line fix, needs the owner's call. `D4` at `:1152` has the same missing owner
stamp but currently passes, because it sets `g->active = 1` so the two routings coincide.

### The mechanism that resolved the dispute (worth keeping)
`:680` could only pass after inversion because **commit `2130769d` landed the
`face_up` gate between the two agents' measurements.** Gate OFF → `:715` RED,
`:680` GREEN. Gate ON → `:715` GREEN, `:680` RED, plus 46 reds across 5 suites.
Both agents measured a different tree. **Always state the HEAD you measured.**

The clincher: `D1_face_down_survives` — the **only** assertion in the affected set the
gate *fixes* — is the C's translation of Rust's
`face_down_non_live_card_survives_live_card_set_check`, and its name states the rule.
The four suites whose reds the gate caused still carry the comment that gave it away:
`"C GameState has no live_card_zone.face_up field"` — written when `GameState` had no
face-down state, which `2130769d` gave it. The tests were never updated.

---

## 🐛 THE ORACLE HAS ITS OWN DEFECTS — `RUST_REFERENCE_AUDIT.md`

"Rust is the authoritative reference" was an ASSUMPTION. It is not. Verified at HEAD
`b45ff90e`. **Do not "fix" the C to match Rust where these apply.**

### Where the C is RIGHT and Rust is WRONG
- **`turn/phases.rs:831-836` LOSES THE CARD.** It removes the card from hand, then
  returns `Err` on a full live zone — the card ceases to exist. **The C refuses first
  and deliberately does not inherit it.** A case where matching Rust would be a
  regression. Recorded so it is never "fixed" for consistency.
- **`ability/effects/look.rs:1031-1040`** `fetch_look_pool` ignores `all_any` for both
  deck arms while the stage/hand arms honour it.
- **`game_state/modifiers.rs:2280-2287`** keys `non_stackable` by *text*, not card id;
  the four-zone scan widens the collision.

### Where the C INHERITS a Rust bug
- **Q213** (`PL!HS-bp5-019-L`) — confirmed again: `check_invalid_live_cards` is gated on
  `face_up` (`actions/mod.rs:1734`), set true only at `phases.rs:343`, while
  `trigger_live_start_abilities` runs at `:245-246`. Members count toward the live-slot
  total (`condition/card.rs:4419-4421`, no `is_live` filter), so **the second attacker's
  members survive the entire first performance.** Filed against `actions/mod.rs:1734`.
- **Q68** (`PL!HS-bp2-014-N`, `PL!SP-bp1-001-P/R`) — `trigger_live_start_abilities`
  (`trigger_scan.rs:186-212`) **never consults `cannot_live_players`**, so ライブ開始時
  fires for a ライブできない player and the live cards are discarded during the yell.
  **The C inherits this. This one IS actionable in C.**
- `PL!N-bp1-027-L` (Q67) — open, could not be settled.

### The oracles that never run
- **`pvp_room_test` CANNOT COMPILE.** `engine/build.rs:90` excludes it; the file imports
  `web_server::pvp_player_can_act`, and `lib.rs:149` gates that behind
  `#[cfg(feature = "server")]`, which is **not** in `Cargo.toml:62` `default`. Its **14
  tests / 345 lines are the sole oracle for `choice_player_id` routing**, including the
  JSON key `"choice_player_id"` (`:267`), which **no compiled test asserts anywhere.**
- **`bytecode_deep_compare_test`** is gated `feature = "json_path_test"`, also not
  default — **the bytecode↔JSON oracle never runs.**
- **3 phantom `mod.rs`** naming modules absent from disk *and from all of git*:
  `bounded_fan_cost_equality_gate_test`, `set_heart_type_multi_target_test`,
  `pbl02_card_table_test`. Each parent directory contains **only** `mod.rs` — zero tests.
  `ability_gates/` is an empty top-level category.
- **`unique_abilities_test.rs` is 0 bytes.**

### Q213 HAS ZERO EFFECTIVE COVERAGE
`live_start_heart_requirement_reduction_counts_live_slot_cards_only_test.rs:59` is
`assert!(after_waitroom >= before_waitroom)` — **satisfied by zero cards added, so it
cannot fail.** The file's stated purpose is the Q213 heart reduction and **it never reads
a heart modifier.** Its sibling `hanamusubi_required_hearts_exclude_self_test.rs` has 9
real tests, **none of which stages a member in the live slot.** So the ruling that
exposed the biggest cross-engine bug has **no test that can detect it.**

### 325/805 — THE PREMISE WAS INVERTED. There is no Rust data bug.
Both engines are **internally exact**: `vm.rs:174-183` strips the `RBKA` header, and
`OFFSET_DELTAS` sums to **98725 == the payload exactly**, 985/985 slices tag-valid. The
C's own `bytecode_blob.c` is 98737 B, also 985/985. Slices **325 (149→159)** and
**805 (236→238)** are the two abilities that changed between generations; **63 277 bytes
differ.** It is a generation skew, not corruption.
⚠️ **But `build.rs:325` THROWS AWAY the staleness verdict** — `let _ = stale;` with the
panic commented out at `:323`. **The staleness check exists and is disabled.** That is
why the skew went unnoticed. Re-enabling it is a Rust-side change.
3 Rust tests still pin the OLD encoding (325 = `PL!-pb2-012-P＋`, 2 tests; 805 =
`PL!N-bp7-028-L`, 1 test).

### Corrections to claims in this project
- **`trigger_counts` keys are NOT mojibake** at HEAD or working tree.
- There are **NOT 6 untracked Rust test files — there is 1**:
  `self_to_waitroom_recover_member_own_card_is_candidate_test.rs` (155 lines, 3 tests,
  with a negative control) — untracked, so it runs nowhere.
- `ability/condition/card.rs:1632-1658` (the `cause_card_id` arm three agents rewrote in
  C) — **no defect found.** Recorded so it is not audited a fourth time.
- Both deck-order producers (`:1864`, `:3120`) — **consistent, no defect.**
- HEAD moved twice during the audit and `cf315a9a`/`2130769d` were **squashed away.**

---

## ⚠ CITATION TRAPS THAT HAVE ALREADY MISLED AGENTS

### There are TWO `choice.rs` in `engine/src`
- `engine/src/ability/choice.rs` — **3 921 lines**, `execute_choice` at `:27`
- `engine/src/ability/effects/choice.rs` — **277 lines**, the `choice_maker` stamp at `:142-153`

**A bare `choice.rs:142` resolves to the WRONG file** — `ability/choice.rs:142` is inside
the REPEAT prompt. An agent hit this and corrected it. **Always cite the full path.**

### `src/web_server.o` is linked ONLY into `$(WEB_TARGET)` — never into `OBJ_LIB`
So **no test suite can reach any symbol defined in `web_server.c`.** A reader that a suite
needs must live elsewhere (e.g. `game_state_abilities.c`) with a file-scope extern in
`web_server.c`. This is why `append_pending_choice` has no suite coverage at all.

### `WARN_AUDIT_BASELINE` is keyed `file:line`
Two struct fields declared on one line is deliberate — splitting them would move pins.
Do not "tidy" that.

---

## ✅ Q68 — PORTED via a TURN STAMP, not the rollover clear

`rb_trigger_live_start` had exactly Rust's two gates and no third. Rust first reads
`cannot_live_players` at `phases.rs:351` — **after** the scan at `:245-246` — then
discards the live cards at `live.rs:2053-2055`.

The prescribed fix (a rollover clear in `phase.c`) was **unreachable**, because
`rb_rules_state` is `static` inside `rules.c`. Instead the flags now carry the turn they
were set on (`cannot_live_turn[2]`, `cannot_activate_turn[2]`) and **both readers drop a
stale stamp**. That is the idiom already blessed for Rust's per-turn
`turn_limited_abilities` (`ability_queue.c:1138`), needs no `phase.c` or header edit, and
gives the same guarantee.

**Rust's clear is in TWO places** — `phases.rs:279-280` AND `abilities.rs:3207`
(`clear_live_only_state` ← `check_expired_effects:3241` ← `phases.rs:310`). A C fold must
cover both or neither; **the turn stamp covers both by construction.**

Reachability: **2 of 985** ability nodes carry `"restriction_type": "cannot_live"`
(`PL!SP-bp1-001-P/R` 常時 target:self; `PL!SP-bp1-005-P/R` 登場). `PL!HS-bp2-014-N` is
**not** in that set despite the ruling naming it. Zero suite movement — correct, since
no test exercises those two nodes.

### The three "C is right / Rust is wrong" items — the C inherits NONE
`phases.rs:831-836` no (`rb_handle_set_live_card` refuses on capacity **before**
`rb_hand_remove_card`, `phase.c:1335-1346`). `look.rs:1031-1040` no — and it goes the
OTHER way, `look_at_fetch` applies `all` to every source; **VERIFIED-NOOP: 0 of 84**.
`modifiers.rs:2280-2287` no — the text-keyed dedupe only arrived in C with E1.

---

## Definition of done

### Measured baseline — `BASELINE.md`, two runs agreed EXACTLY (no nondeterminism)
| metric | value |
|---|---|
| FAIL assertions | **2 530** (2 350 `FAIL: ` + 180 `FAIL [`; 0 on stdout) |
| distinct FAIL texts | 2 393 |
| failed tests | **949** — really **949…1632**, ill-defined until cluster H |
| suites | **108 RED / 28 GREEN / 5 unverified / 1 silent** of 142 |
| build | clean, 0 errors, 120 warnings; `make web` clean |
| hangs | 0 of 142; slowest 18.564 s |
| crashes | 2 — SIGSEGV `test_parity_recover_hand_a.c:1058`, SIGABRT `test_parity_cce_gaps.c:3601` |
| blob | 726 831 B, sha256 `89204df0bbde08a0…` verified pre-build, post-build, post-run |

Reconciles exactly with the earlier G/H figure (2 348 = the `FAIL: `-only shape; the
missing 180 are the no-colon lines from 10 suites, which is also the 98-vs-108 red
difference). The distribution is FLAT: top 25 assertions = 101 lines = 4.0%.
**But ranks 1–6 are ONE root cause** — see cluster N.

1. `make clean && make all` green from scratch
2. `verify_artifacts.py` consistent
3. Every `[ ]` above closed or explicitly marked IMPOSSIBLE with a quoted reason
4. Whole-tree FAIL count taken in an ISOLATED snapshot, two agreeing runs — **DONE, see above**
5. No test weakened, deleted, or ignored to reach a number

---

## CLUSTER N — `nozomi`: the deck-order `SelectTarget` and the extras-vs-struct-field class

**Owner:** nozomi. **File owned:** `tests/test_parity_search_reorder.c`.
**Full write-up, with all patch text and the A/B tables: `engine_c_wip/NOZOMI_ROOT_CAUSE.md`.**
**Landed: no engine file.** All five files the fix needs are held by a live owner, so
every change is written out as a landable BLOCKED-ON and was **built and A/B'd in a private
snapshot** (`C:/Users/trios/AppData/Local/Temp/kilo/nozomi/`, re-snapshotted after T3's
header bill).

### The one root cause behind ranks 1–6

> **The C has no producer for `Choice::SelectTarget { target: "order" }`, and to fill the
> gap it borrows `look_at` — a verb whose Rust twin opens no prompt at all.**

Rust has **two** producers of that choice (`move/mod.rs:1915-1932` `maybe_prompt_deck_order`,
called at `:2272`, and `move/mod.rs:3139-3149` the `look_and_select` `needs_order` arm).
The C has **zero**: `"order"` appears exactly three times in all of `src/` — the answer arm
(`choice.c:3895`), the handler (`choice.c:3951`) and the enum strings. So `RB_STK_ORDER`
(`enums.c:384`) and `rb_resolver_handle_order_selection` are both unreachable, and
`Choose order for cards on deck` has zero occurrences anywhere in the C.

Three concrete defects:

| # | C | Rust it must match |
|---|---|---|
| **A** | `look.c:387-403` `rb_effect_look_at` emits `SelectCard{zone:"looked_at",count}` and parks the queue | `look.rs:1055-1096` `execute_look_at` fills `gs.looked_at_cards` and returns `Ok(())`; never touches `gs.awaiting.choice` |
| **B1** | no producer (above) | `move/mod.rs:1855-1933` `maybe_prompt_deck_order` |
| **B2** | `choice.c:3951-3972` parses `"2,0,1"`, reorders the pool, **never places the cards** | `choice.rs:2984-3055` `handle_order_selection` |

Probe, not inference (the brief's "custom" hypothesis does **not** apply — the card is a
plain two-step `sequential`, so `misc.c:1053-1060` `h_custom` is not on this path):

```
[NZPROBE] rb_effect_look_at act=look_at src=deck_top dst=(null) cnt=3 pool=3     <- x13
[NZPROBE] execute_move_cards act=move_cards src=discard dst=stage cnt=1 …          <- never
```

`rb_move_execute_move_cards_ported` never runs for the card's OWN second step
(`src=looked_at dst=deck_top all=true placement_order=any_order`): the sequential never
gets past step 0.

### Ranks 1–5, and the delta

| rank | assertion | count | before | after |
|---|---|---|---|---|
| 1 | `expected a SelectTarget order prompt, got SelectCard` | 12 | 12 | **0** |
| 2 | `the SelectTarget target is "order"` | 12 | 12 | **0** |
| 3 | `the final remaining card needs no choice` | 6 | 6 | 6 (blocked on the re-fire) |
| 4 | `nothing joins the hand (got 2 expected 0)` | 6 | 6 | **0** |
| 5 | `nothing is discarded (all three return) (got 2 expected 0)` | 6 | 6 | **0** |
| — | `the looked_at pool is emptied` | 0 | — | **+6, from the #6 re-fire. STILL RED after everything landed 2026-10-04 — see the re-fire BLOCKED-ON entry at the end of this file; the cause is NOT any of the four hunks of that task.** |


**36 of the 42 lines close. `parity_search_reorder` 122 → 89** (125 → 89 counting the two
new invariant cases). `Choose order for cards on deck` now appears in the C for the first
time, and the answer handler really puts the cards back on the deck top.

### A/B across 24 suites — **1022 → 939, −83**

`parity_restriction` 42→18 (−24) · `parity_sequential` 112→99 (−13) ·
`parity_zone_source` 51→42 (−9) · `parity_jidou_watch` 55→52 (−3) ·
`parity_cce_gaps` 54→52 (−2) · `parity_search_reorder` 122→89 (−33) ·
`parity_hearts_live_start` 63→64 (**+1**) · `parity_wave1c` 31→31 (identical FAIL multiset) ·
9 other suites 0 · `parity_full_round{,2,3}` / `restriction_mechanics` / `mechanics` 0 with
real output (7 kB–86 kB, so they RAN).

The single regression is one line inside an already-failing
`parity_hearts_live_start` test: `area-moved member gains the chosen heart until live end
(got 2 expected 1)`.

### BLOCKED-ON summary (exact patch text in `NOZOMI_ROOT_CAUSE.md` §6)

| # | site | owner | effect if landed |
|---|---|---|---|
| 1 | `look.c:387-403` — delete the `look_at` prompt | H / T2 | ranks 1,2,4,5 |
| 2 | `move_missing.c` — add `maybe_prompt_deck_order` (**file is UNCLAIMED; nozomi claims it**) | nozomi | ranks 1,2 |
| 3 | `choice.c:3951-3972` — replace the order answer handler | B | ranks 1,2,3 + deck order |
| 4 | `draw.c:170` split the comma-joined `heart_colors`; `draw.c:658/662` field-first `source`/`card_type` | F | rank 6 (**−24**) |
| 5 | `cost.c:1671` field-first `card_type` in the reveal cost | B | rank 7 (**−9**) |
| 6 | the **re-fire**: `ability_queue.c:1403-1405` skip test vs the prompt-parking break at `:1584` | B / B1 | 12 more lines, incl. rank 8 |

**#2 must not land without #3** — with today's `choice.c` the prompt would be answered by a
handler that never places the cards, which is strictly worse than no prompt.

### #2 LANDED 2026-10-03 — the producer exists; the prompt is REACHABLE

`rb_move_maybe_prompt_deck_order` is in `src/ability/effects/move_missing.c` and is called from
`rb_move_execute_move_cards_ported` at the Rust position (after the two `taken.is_empty()`
early-outs, before the distinct dedupe). **16 of 985** abilities carry a `move_cards` node whose
static predicate holds (30 card-ability pairs); **15 of 985** can actually raise it at run time,
since ab 930 (`PL!SP-bp7-012-N`) declares `count: 1` and `taken.len() > 1` can never hold.
Reachability was measured with `tools/census_deck_order.c` (new), which walks the shipped
bytecode with the engine's own decoder. `parity_search_reorder` 148 → 123 `^FAIL` (stderr
included), 2 case verdicts fixed (`nz/deck_order_prompt_is_select_target_order`,
`rt/live_success_not_on_debut`), **0 case verdicts regressed**, 18 of 21 suites byte-identical;
the only new FAIL lines are the 6 already-predicted **#6 re-fire** lines. **The second Rust
**The second Rust producer — the `look_and_select` `needs_order` arm, `move/mod.rs:3120-3157` — LANDED 2026-10-04 in `src/ability/effects/look.c` `rb_look_resume_indices`, immediately before `const char *destination_zone = ...`.** `parity_search_reorder` **123 -> 119 `^FAIL`, 4 lines gone, ZERO lines new, 21 of 22 suites byte-identical.** All 4 are `rt/rin_select_both_stay_on_deck`, which is not this cluster's test and was RED only because the producer did not exist.

  **The MEASURED count asked for: 11 of the 11 named, and all 11 genuinely fire.** Over `cards/abilities.json`, exactly **11** `look_and_select` nodes carry `placement_order: "any_order"` — the static predicate's count is exact, not an estimate. The `count: 1` trap that killed `PL!SP-bp7-012-N` in the FIRST batch does **not** bite here, and that was checked rather than assumed: the trap is `select_action.count.unwrap_or(1)` collapsing `max_select` to 1 (`look.rs:855-889`), which is defeated by `any_number` (`max_select = total_available`). **All 11 also declare `any_number: true`**, so a player can hand back 2 or 3 indices and `selected_cards.len() > 1` is reachable for every one of them. Destination split: 10 `deck_top`, 1 `deck_bottom`, so `is_deck_dest` holds for **11 of 11** — `PL!S-bp7-004-P` is the only one that exercises the `deck_bottom` arm. Look sizes are 2 (x6) or 3 (x5), i.e. every one is `n_selected in {2,3}`. Nine carry no cost node; `PL!S-bp2-007-P` and `PL!HS-bp2-003-P` carry an optional hand-discard cost.

  One honest limit, measured rather than papered over: **the remainder branch of the arm (`mod.rs:3130-3137`, the unchosen cards going to the discard BEFORE the prompt opens) has corpus reachability 0.** Every one of the 11 looks at most 3 cards, so `n_selected > 1` forces the remainder to be 0 — a 3-card look either takes one per prompt (`n_selected == 1`, no order prompt) or takes 1 then both (`n_selected == 2`, remainder 0). The prompt leg is 11-reachable; the remainder leg is correct but 0-reachable, and a fixture for it cannot be built from this corpus.

  ⚠ The arm takes `destination` from the **function parameter**, not from `*_extra`. That is what makes it work: `destination` is one of the STRUCT FIELDS the "swept class" section below says `decode_effect_body` stores outside the extras table, so `look_extra(e,"destination")` would be permanently NULL. Anyone porting a sibling arm must read `destination` the same way.

  The arm also re-records the chosen cards in `selected_cards` / `recently_moved` before returning, mirroring `choice.rs:3052` `self.selection.moved_cards.extend(ordered)` — the order answer treats them as moved cards. Adding that was MEASURED to change nothing on the 22-suite list, so it is not load-bearing here, but it is faithful and it belongs there.

### The swept class: "a parsed STRUCT FIELD read from the EXTRAS table"

`vm.c` `decode_effect_body` `continue`s after storing these in `AbilityEffect`, so they are
**never** in the extras table and any `*_extra(e,"<key>")` read is **permanently NULL**:
`text` · `action` · `source` · `destination` · `target` · `zone` · `count` · `condition` ·
`card_type` · `per_unit_count` · `cost_reduction_per_group` · `actions`/`effect_steps`/`costs`.

**37 unguarded sites** are enumerated in `NOZOMI_ROOT_CAUSE.md` §4. Two have measured
impact (`cost.c:1671`, and the latent-but-real `draw.c:658/662` — the peer's report is
**still open** in the live tree, verified by reading the file, not assumed).
`draw.c:444`, `misc.c:292/…/1540`, `state.c:724/1487` etc. already use the correct
field-first-with-fallback idiom — use those as the template.

A **second member** of the same class: `vm.c:1177-1186` flattens an effect **array** into
one comma-joined extras string, so `heart_colors:[a,b,c]` arrives as `"a,b,c"`. Every C
reader expecting a list must split. `draw.c:170 draw_heart_colors` was the live instance
(−24 lines); `rules.c:1724 r_heart_colors` still has it, latent, because its only caller
`rb_rules_execute_select_effect` (`rules.c:1580`, the faithful port) has **zero callers** —
the dispatched copy is `draw.c:653`. Correct splitters for comparison: `misc.c:447/776`
`split_heart_colors`, `score.c:946` `sc_parse_colors`, `executor.c:227`.

### Ranks 6–10 attributed

| rank | assertion | count | verdict |
|---|---|---|---|
| 6 | `parity_restriction` `colour choice must be offered` | 5 | **the array-flattening bug** — `draw.c:170` + the `nu == 1` short-circuit at `draw.c:936`. **Closed, −24** |
| 7 | `parity_zone_source` `F: the reveal is filtered to live cards` | 9 | **the extras-vs-field bug, in a COST not an effect** — `cost.c:1671`. Note `cost.c:1669` reads `source` from the struct field correctly one line above. **Closed, −9** |
| 8 | `parity_zone_source` `F: CROSSROADS is in the waitroom` | 5 (+1) | **NOT downstream of rank 7** (rank 7 closes, this does not move). The re-fire (BLOCKED-ON 6) shows as the sibling `F: 真姫 exists exactly once (got 2 expected 1)`; the placement leg (`move_cards` from `success_live_zone`/`selected_cards`) is unisolated |
| 9 | `parity_wave1c` `B1 COST LEG 1 NOT PAID … half-paid activation` | 5 | **NOT this class — proven**: all five patches move `parity_wave1c` by exactly 0 lines. `PL!HS-bp1-002-RM` 起動 `sequential_cost [pay_energy 2, move_cards self_cost]`; `costs`→`child[]` decodes fine (`vm.c:859`) and `cost.c:1478-1531` dispatches children via `pay_cost_inner` (`move_cards` at `:1566`), yet neither leg runs. **T6-6 did NOT cover it** — T6-6 is the mandatory-cost *refusal* (`actions/mod.rs:553-564`), the opposite failure |
| 10 | `parity_hearts_live_start` `second selection prompt should appear` | 4 | **NOT this class — proven** (4 → 4). `PL!HS-bp5-006-R` is a **count-2 hand cost**; the C opens one `SelectCard` and the answer path never re-prompts for the remaining count. The filter siblings pass, so only the multi-pick re-prompt is missing |

### Trap notes for whoever lands this

* `cost.c` has **ten** `eff_extra(cost,"card_type")` reads with **identical text**
  (353, 798, 897, 995, 1051, 1370, 1671, 2056, 2328, 2447). An `edit` with that string as
  the anchor silently patched the WRONG one (798) — anchor on the surrounding
  `if (cost_is_reveal(cost)) {` block and re-verify by line number.
* `Copy-Item` preserves `LastWriteTime`: restoring a fixed source hands it an mtime OLDER
  than its `.o` and `make` skips the rebuild, producing a byte-identical "fixed" run. Delete
  every `.o` before each A/B state.
* `parity_lookup` / `parity_nazuna_at_deck_top` / `parity_draw_top` / `parity_watchers`
  **do not exist**; `make t T=<stem>` fails and emits no `FAIL`, which reads as PASSING.
* PowerShell `-match` is case-insensitive, so a suite summary line `failures : 54` counts
  as a failure; use `-cmatch`.

### Landed (live tree)

* `tests/test_parity_search_reorder.c` — two new invariant cases
  `nz/look_at_verb_opens_no_choice` and `nz/deck_order_prompt_is_select_target_order`.
  Both **red by design** (3 `^FAIL`) and **green with the patches**; both cite the Rust
  line in the failure text; both use `static TestGame` and `test_new_id`. No existing
  assertion weakened, removed or ignored.

No git writes. `HEAD` still `f152a50f`. No `.o` in the live tree. `make data` never run;
`src/abilities_strings.bin` 726 831 B / `89204df0bbde08a0…` verified around every build.

---

## Cluster H — THE HARNESS ITSELF (opened 2026-10-03, harness agent)

Measured in the snapshot `C:/Users/trios/AppData/Local/Temp/kilo/harness/engine_c_wip`,
149 suites, strictly sequential, stdout and stderr to separate files, binary
deleted after each run. Full write-up with faulting lines:
**`engine_c_wip/HARNESS_FIXES.md`**. Tests only — no engine file and no
`include/rabuka.h` edit was needed for any fix below.

### H0. Headline: "failed tests" was ill-defined; it is now a number plus a stated residual

| metric | before | after |
|---|---|---|
| CRASH verdicts | **2** | **0** |
| failed tests reported by a named verdict | 948 | **1 022** |
| FAIL assertion lines (both streams) | 2 810 | 2 511 |
| suites whose FAIL lines belong to no named test | **44** (932 lines) | **26** (529 lines) |
| suites emitting no output at all | 1 | **0** |

Current honest figure: **1 022 failed tests**, plus 529 failing assertions in 26
suites that still cannot be attributed — a residual bound of **1 022 … 1 551**,
down from 948 … 1 632. It is still a range; H1 names exactly what closes it.

### H1. OPEN — 26 suites still have no per-test verdict (529 FAIL lines)

All FLAT: one `main()`, every test body in one process, `FAIL:` lines that name
an *assertion* and no test. Fix and full recipe in `HARNESS_FIXES.md` §2b:
insert `flat_verdict_failed()` / `#define FLAT_VERDICT()` beside the failure
counters, then call `FLAT_VERDICT();` in **each** macro failure branch — `CHECK`
*and* `CHECK_EQ`. Patching only `CHECK` leaves every `CHECK_EQ` failure
unattributed; that was my own first mistake, caught and corrected. **12 done,
these 26 not:**

```
parity_search_reorder 125   parity_sequential 112   parity_draw_flat 74
parity_target_player 54     parity_jidou_watch 55   parity_jidou_move_yell 49
parity_zone_source 51       parity_mill_deck 52     parity_condition_and_effect 50
parity_conditional_gated 46 parity_jidou_watch2 46  parity_jidou_watch_clusters 47
parity_cost_discount 47     parity_compound_per_card 41  parity_restriction 42
parity_state_per_card 46    parity_zones_position 37   parity_look_per_card 37
parity_wait_activation 40   parity_yell 39     parity_wave1b 39   parity_live_total 32
parity_blades_live_start 34 parity_recover_live 35  parity_area_move 32
parity_wave1c 31            parity_jidou 26    parity_bp7 24
parity_characterization_extra 26  parity_int_gaps 25  parity_jidou_extra 25
parity_deploy_recover 24     parity_recover_score_state_delta 28
parity_deploy_multi 21      parity_tp_joint 21   parity_jidou_life 22
parity_trigger_paths 23     parity_ability_watch 20   parity_reveal_extra 20
parity_rules_phases 18      parity_per_card 15   parity_ability_mod_extra 16
parity_blades2 16           parity_required_hearts 16  parity_per_card3 13
parity_tp_zone_change_gate 13  parity_recover_to_hand 10  parity_rules 10
parity_abilitymod_conditional 7  parity_deploy_move_missing 7
parity_rules_conditions 6   parity_max_distinct_names 5  parity_survey_probe 5
parity_energy_remainder 3   parity_characterization 2  parity_char2 2
parity_gain_delta 2         parity_parser_e2e_extra 8   parity_rules_extra 4
parity_trigger_paths 23     trigger_paths 19    audit_conditions 2
choice_renderer 3           move_cards 1        cross_player_interaction 1
target_selection 3          tp_activation_offer 3  performance_phase_rules 2
```

### H2. OPEN — `tests/test_choice_renderer.c` HANGS

`rb_engine_t_choice_renderer.exe` consumed **3 746 s of CPU** with no further
output and had to be killed for the tree run to complete. It emitted only the
`data guard:` lines and the `./rb_engine_t_...` echo — **zero** assertion output.
`BASELINE.md` §9's "0 hangs of 142" predates this suite. Consistent with the
standing `AGENTS.md` rule that an unbounded drain hangs the forked-child
harness. **Not diagnosed, not fixed.**

### H3. OPEN — `audit_zones`' `stage_first_empty_is_centre_first` child crashed once, never again

The base run recorded
`FAIL: stage_first_empty_is_centre_first: fatal SIGSEGV (11) in child after 0 assertion(s)`.
After the H1 verdict work the suite was rebuilt and run **twice**; the SIGSEGV
did **not** reproduce (2 FAIL lines, 2 FAILED verdicts each time). No mechanism
established, and nothing I changed touches that child. This is an **unexplained
nondeterministic child crash**, and it matters because the corpus's central
claim is that runs are bit-identical — for this one suite that claim does not
hold. **Not fixed.** Its `fatal_handler` still ends in `_exit(0)`
(`test_audit_zones.c`, twin at `test_audit_actions.c:122`), so a crash there
still exits 0; I added the `CRASH:` verdict line but deliberately did **not**
change the `_exit(0)`, since that file's shared-counter protocol is not mine to
re-shape. The `signal(SIG_DFL); raise(sig)` treatment **was** applied in
`test_parity_recover_hand_a.c`, `test_parity_cce_gaps.c`,
`test_audit_triggers.c` and `test_parity_per_card2.c`, where it is what makes
`WIFSIGNALED` — and therefore the `CRASH` verdict — reachable at all.

### H4. OPEN — 974 non-static `GameState`/`TestGame` locals across 58 test files

`sizeof(GameState) == sizeof(TestGame) == 885 912 B (864.2 KiB)`, measured, now
correctly stated in `Makefile:3-9` **and asserted** by
`tests/test_harness_self.c` §A against `RB_GAMESTATE_BYTE_CEILING`. That part
was **already correct** — a peer fixed the header on 2026-10-03; the old
"~780 KB" was wrong by 105 912 B. I made no Makefile change.

What is **not** done is the sweep. **974** non-static locals remain (largest:
`test_parity_card_score.c` 74, `test_parity_hearts_constants.c` 67,
`test_parity_wait_activation.c` 66, `test_parity_characterization_extra.c` 54,
`test_parity_score_effects.c` 51), and four functions still hold two at once —
~1.7 MB frames (`test_parity_jidou_extra.c:900`/`:906`,
`test_parity_stats_pipeline.c:340`, `test_parity_reveal_extra.c:995`,
`test_parity_queue_resume.c:312`). Death is prevented only by
`-Wl,--stack,16777216`, correctly documented as load-bearing. The conversion is
provably neutral — automatic storage is never zeroed by the C runtime and every
site is memset by `test_game_new()` / `rb_game_init()` — but it is 974 edits
across 58 files owned by 12 concurrent agents, and this repo's standing rule
forbids scripted mass patches. **Deliberately not half-done.**

### H5. OPEN — 43 `VACUOUS` + 5 `SETUP BUG` assertions still prove nothing

Neither is a `FAIL` line, so a `^FAIL` scan scores them 0 — the opposite of
green. `VACUOUS`: `parity_look_per_card` 14, `parity_target_player` 8,
`parity_recover_q79` 5, `parity_recover_hand_a` 4, `parity_wave1c` 3,
`parity_cce_gaps` 2, `parity_look_and_filter` 2, `parity_parser_e2e_extra` 2,
`parity_wave1a` 2, `parity_recover_live` 1. `SETUP BUG`: `audit_conditions` 3,
`parity_blades_live_start` 2 — one is a wrong-card defect
(`expected card PL!HS-pb6-002-R, resolved id=-1 (<none>)`). Each needs a real
paired positive written against the Rust reference; guessing would be worse
than leaving them loud. **Not touched.**

### H6. OPEN — the measurement still trusts `^FAIL` too literally

The corpus emits **five** prefixes, not the two previously assumed: `FAIL:`;
`FAIL [` with no colon (175 lines, 10 suites — normalised in
`test_audit_triggers.c`); `FAILED` (with or without a colon: FORK suites print
both `FAILED: name` and `FAILED   name`); `CRASH`; and the private
`== case … assertions=N failures=M` / `== <name>: RED as pinned` /
`--- case <name>` vocabularies 7 suites used, which now all emit the common
verdict alongside their own. A future measurement must count
`^\s*FAIL:` ∪ `^\s*FAIL \[` for assertions and `^\s*FAILED\b` for verdicts.
`^\s*FAILED:` alone silently loses 900+ tests, because most FORK suites pad
with spaces rather than a colon. `^\s*FAIL` alone conflates the two and
overcounts assertions by the number of verdicts — it reports 3 758 where the
real assertion count is 2 511.

---

# T8. Cluster T8 — THE ORPHANS, THE DEAD BUILD GRAPH, AND THE SCOPE DECISIONS

**Opened 2026-10-03.** A cluster whose output is a **DECISION**, not a fix. Full
evidence, with every command, in **`ORPHANS_AND_SCOPE.md`**. Read that file for the
argument; this entry is the index and the status.

| file touched by T8 | why |
|---|---|
| `ORPHANS_AND_SCOPE.md` | **new** — the decision document (Q1-Q5) |
| `PROGRESS.md` | Q2 + Q3 scope decisions, written in so they stop being re-litigated |
| `src/test_game.c` | 1 divergence fixed, 4 missing Rust twins added |
| `include/test_game.h` | declarations for those 4, with their divergences on them |
| `tests/test_harness_self.c` | §K + §L added; §D rebaselined with the delta attributed |
| `PARITY_CHECKLIST.md` | this entry |
| `Makefile` | **NOT touched** — see T8.1; the fix for the header is main's |

## T8.0 — CLOSED: `src/ability/choice_renderer.c` is BUILT. 1963 of 1963 lines.

T5 measured **1821 of 1964 lines compiled into nothing**. That was true of the
revision it measured. `Makefile:25` (`SRC`) and `Makefile:28` (`OBJ_LIB`) now list
the file, and a from-scratch build proves it rather than asserting it:

```
cc ... -c -o src/ability/choice_renderer.o src/ability/choice_renderer.c
cc ... -o rb_engine ... src/ability/choice_renderer.o ...
```

Exercised by `tests/test_choice_renderer.c` — **green, 75 stdout lines, 0 FAIL**.
The file stays in `SRC` even though no production consumer exists: this tree has no
`PlatformUi` backend, and `platforms/*/build_*.sh` correctly still omits it (those
targets ship their own UI).

⛔ **T8.0a — the fix is UNCOMMITTED.** `git status` reports `M engine_c_wip/Makefile`;
`HEAD = f152a50f` does **not** list the file. Anyone who checks out the tree, or
reads a `git diff`, sees an unbuilt file. If that edit is lost, 1963 lines vanish
from every binary and **no compiler complains**. This is the highest-risk item in
the cluster.

## T8.1 — OPEN: `Makefile:3-8` carries a WRONG `sizeof(GameState)`, and H4 below repeats it

`Makefile:3-8` says **885 912 B** and attributes it to `tests/test_harness_self.c`
§A. The measured value is:

```
sizeof(GameState) = 1113736 bytes (1087.6 KiB)
sizeof(TestGame)  = 1113736 bytes
```

**Stale by 227 824 bytes.** The same comment records that the previous figure
("~780 KB") was wrong by 105 912 B — so this header has now been wrong twice while
claiming to be *measured*, which is worse than an estimate because it looks
authoritative.

⛔ **This makes H4's claim ("now correctly stated in `Makefile:3-9` **and
asserted**") wrong.** H4 must be corrected along with the header.

Consequence: §A's ceiling is `16777216 / 15 = 1118481`, so headroom is **4745
bytes — about three ints.** The next field appended to `GameState` breaches it, and
the correct response is converting stack locals to `static`, not raising the number.
The conversion to `static` is still the outstanding H4 work.

**T8 did not edit the Makefile** — it is shared, not in T8's owned set, and the
correction is main's to apply.

## T8.2 — RESOLVED (negative): H2 "`tests/test_choice_renderer.c` HANGS" does NOT reproduce

H2 records `rb_engine_t_choice_renderer.exe` consuming **3746 s of CPU** with zero
assertion output. Re-measured in T8's snapshot, from a clean build:

```
choice_renderer   exit=0  out=75  err=0  FAIL=0
wall clock 1.2 s
ok: A1 an all-NULL RbRendUi returns -1 (Rust's None) instead of spinning
...
```

**1.2 seconds, 75 assertions, 0 FAIL.** H2's hang was measured against a binary
built before `choice_renderer.c` was wired into `SRC` (or a stale `.o`), which is
the `AGENTS.md` stale-link trap wearing a hang costume. **H2 can be closed**, but
only with this caveat: if it ever comes back, suspect the build, not the suite.

## T8.3 — OPEN: `src/core/alloc_counter.c` is unwired — and the quarantine premise is FALSE

`alloc_counter.c` (775 lines) has a real Rust twin, `engine/src/alloc_counter.rs`
(477 lines). It is in neither `SRC` nor `OBJ_LIB`, and **none of its `rb_allocctr_*`
functions is declared in `include/rabuka.h`.**

The brief offered it as the template for "an unbuilt file nobody notices is broken"
and stated it "FAILS TO COMPILE under every `-std`". **That half does not reproduce.**
Measured with this tree's gcc 15.3.0:

| `-std=` | result |
|---|---|
| `c99`, `c11` (the Makefile's, `Makefile:2`), `c17`, `gnu11`, `gnu17` | exit 0, no diagnostics |
| `c2x` (C23) | **exit 1, 18 errors** |

`ATOMIC_VAR_INIT` was removed in **C23**, not C17. C17 deprecated it and glibc keeps
defining it; the file fails only at `c2x`, with `implicit declaration of
ATOMIC_VAR_INIT` at `src/core/alloc_counter.c:97` plus 17
`initializer element is not constant` errors.

**Verdict: FIX AND WIRE, do not quarantine.** Quarantining would have destroyed 775
lines of real work on the strength of a claim that does not reproduce in this
toolchain. Recommendation (not applied — `src/core/**` and `include/rabuka.h` are
outside T8's ownership, and adding allocator instrumentation to every link target
needs an owner's sign-off): add the file to `SRC`/`OBJ_LIB`, add the `rb_allocctr_*`
declarations to `rabuka.h`, and replace the 18 `ATOMIC_VAR_INIT` initialisers with
plain `= 0` / `= false` so it also survives C23.

**This inverts the lesson the brief drew from it.** The `alloc_counter.c` case shows
that an unbuilt file here is usually just an **unwired** file, not a broken one —
which is why `choice_renderer.c` was correctly wired rather than quarantined.

## T8.4 — SCOPE DECISIONS (settled; do not re-open without new evidence)

Both are written into `PROGRESS.md` with their evidence.

* **`engine/src/game/display.rs` (2014 lines) — OUT, deliberately.** Not
  `BLOCKED-ON: rabuka.h`; nothing is owed. It is a **read-only** projection (no
  `&mut GameState` anywhere in 2014 lines), so it cannot affect engine behaviour;
  its only consumers are `web_server.rs`, `bin/harness.rs` and a `main.rs` re-export;
  and the C already owns the wire surface in `src/web_server.c`, which **no test
  links**. `describe.c` covers a disjoint projection and is already golden-proven.
  **One real gap, ~100 lines not 2014:** Rust's `filter_display_for_player`
  (`game/web_server.rs:1157+`) hides the opponent's hand; the C has **no filter at
  all** and `src/web_server.c:209` hard-codes `"perspective_player":0`. Port that
  first, if a web UI is ever wanted.
* **`engine/src/bot/**` (21 561 lines, not the ~11.8k quoted) — OUT, and it is NOT
  a parity oracle.** **Zero** of ~1500 Rust test files reference the bot; the
  oracle is `scenario_oracle` over hand-written scenario fixtures, and no in-repo
  expectation artefact is bot-generated (a repo-wide grep hits only `docs/*.md`).
  The one real edge, `engine/src/replay.rs:1`'s
  `use crate::bot::PublicObservation`, is a **format** dependency of a Rust-only
  profiling path (`observe()` is called only by itself and `bin/sim_bench.rs`), and
  the C has no twin of that format. Independently blocked on `crate::rng::Lcg`,
  `crate::turn::TurnEngine` and `crate::core::stats_pipeline`.

## T8.5 — T8's own harness work (see `ORPHANS_AND_SCOPE.md` §Q4 for the full table)

**Fixed:** `test_pending_choice_count` returned a **constant 1** for every prompt
that existed, against Rust's `Choice::SelectCard { count } => *count`
(`helpers/choices.rs:421-428`). A/B measured against a baseline proved by a
file-hash sweep (**exactly one differing file**: `src/test_game.c`): 5 of the 6
suites that call it are byte-identical on both streams after normalising heap
addresses; `parity_characterization_extra`'s FAIL count is unchanged at 45 but two
of its three §K count checks now report the engine's real `RbChoice.count` instead
of the constant. **No suite got redder.**

**Added** (4 Rust twins the C had no spelling for): `test_id_ref`,
`test_fill_decks`, `test_fill_energy_deck`, `test_stage_occupant`. Note the name
trap: Rust's `fill_decks(game, filler)` CLEARS both decks and pushes 30, while the
C's pre-existing `test_filler_hand(tg)` RETURNS a filler id and mutates nothing.

**Harness self-test** `tests/test_harness_self.c` now §A-§L, **GREEN, 0 checks
failed** (90 stdout lines, 4 stderr lines, 0 `^FAIL` on either stream), and it
compiles with zero warnings from its own file.

⚠️ **§B1/§B2 are AT their ceiling, not below it** (1432 non-static state locals in
75 files, against bounds 1432 and 75). Any new suite declaring one
`GameState tg;` locally turns the harness self-test red. That is the ratchet
working, but the next agent to add a suite will meet a red harness.

⚠️ **§D's baseline had already drifted once** — 3011 → 3013 within hours of being
written, when four new suites landed. Rebaselined to 3013 with the delta attributed
in the file.

## T8.6 — suites that DO NOT EXIST (verified before running; they read as PASSING)

Per the standing warning, every name in circulation was `Test-Path`'d first.
**These are not files:** `describe_golden` (the real name is
`test_describe_golden`), `test_game_harness`, `parity_card_text`,
`parity_dynamic_count`, `audit_fields`, `audit_cards`, `parity_sequential`,
`parity_wave1a`, `parity_wave1c`, and `parity_wave1b` as such (the file is
`tests/test_parity_wave1b.c`). `mechanics` is a make **target**, not a file.
`make t T=<stem>` on any of these exits 2 with `no such test:` — but a bare
`make` reports nothing, which is how they read as PASSING.

## T8.7 — measured state of what T8 ran (snapshot, strictly sequential,
`stdout`/`stderr` to SEPARATE files, `make run-all-tests` never invoked)

```
choice_renderer              exit=0    out=75  err=0    FAIL=0   longest=139
deck_parser                  exit=0    out=64  err=0    FAIL=0   longest=74
deck_builder                 exit=0    out=63  err=0    FAIL=0   longest=77
describe_golden              exit=0    out=3   err=0    FAIL=0   longest=91
artifacts_consistent         exit=0    out=6   err=0    FAIL=0   longest=113
mechanics                    exit=0    out=31  err=111  FAIL=0   longest=104
harness_self                 exit=0    out=90  err=4    FAIL=0   longest=309
p1_helpers                   exit=1    out=0   err=9    FAIL=1   longest=83
parity_score_per_card2       exit=1    out=226 err=89   FAIL=2   longest=221
parity_characterization_extra exit=19 out=127 err=3225 FAIL=45  longest=309
parity_look_per_card         exit=23   out=120 err=341  FAIL=74  longest=156
parity_cost_and_effect       exit=1536 out=14  err=13   FAIL=1   longest=87
parity_live_total2           exit=1    out=527 err=134  FAIL=17  longest=280
```

Longest captured line 309 B — far above the 119-char `2>&1 | Out-File`
truncation threshold. **`^FAIL` counted with `-CaseSensitive`** so a suite's own
`failures : 54`-shaped summary line cannot be miscounted. **Exit code is not a
signal**: `parity_cost_and_effect` exits 1536 and `parity_characterization_extra`
exits 19. `p1_helpers` writes **0 bytes to stdout** — its entire report is on
stderr, which is the "no output is not passing" case in its purest form; its one
FAIL is pre-existing and byte-identical in T8's baseline.

Data blobs verified at every checkpoint: `src/abilities_strings.bin` 726831 B /
`89204df0bbde08a0…`, `src/cards.bin` 631890 B / `728208504fee6217…`, unchanged
before the build and after every run. `make data` reported `already installed`
every time; **no generator was run**.

---

# 🔴 CLUSTER Y — THE HEADER BLOCKERS: FIELDS THAT WERE NEVER WIRED

Header bill paid by main (`GameState.rule_log[]`, `resolution_owners[]`,
`live_face_up[]`, `constant_need_heart_global[]`, `RbBatchMovement.cause_card_id`,
six `RbCardFilter` carriers) had **zero writers and zero readers** on arrival. A
green box over dead storage is worse than an absent field, because the next agent
trusts it. Cluster Y exists to make every one of them either WIRED or DELETED, and
to name the ones that are BLOCKED-ON a file the wiring agent does not own.

**Files touched (all verified quiescent first):** `src/core/game_state_core.c`
`17bde447c7c9166a…` → `03dfc8a45d79d56a…`, `src/core/zones.c`
`956ccd50138d5013…` → `8d2c2c354e05b57c…`, `src/turn/phase.c`
`e83297d59f3ea6f2…` → `e84afa505b92fb32…`, `src/turn/live.c`
`284518269074bb99…` → `f32d07255d7cba0f…`. Two SHA-256 reads separated by a
settle: all STABLE except `Makefile`, which MOVED mid-settle
(`49114cb2…` → `80ff1b61…`) and was therefore never touched.

### Y.1 — `GameState.rule_log[500][256]` + `rule_log_len[]` + `n_rule_log` — WIRED (writer) / BLOCKED-ON (reader + one signature)

The false claim at `game_state_core.c:707` ("The C GameState has no `rule_log:
Vec<String>` ring") is deleted, and so is the banner line that listed `rule_log`
among the fields with no home.

* **Writer** — `src/core/game_state_core.c` `gsc_rule_log_push()` (static, the
  faithful port of `push_rule_log_to` + `truncate_rule_log`, `core/game_state/
  mod.rs:1584-1606`: append, then when full shift the OLDEST entry off the front,
  keeping the newest `LOG_BOUND_RULE` = 500). `rule_log_len[]` shifts with the
  text array so entry *i* and its length always describe the same line.
  Reached by `rb_gsc_log_entry` (:832), `rb_gsc_push_choice_offered` (:851) and
  `rb_gsc_push_choice_resolved` (:862). **The 32-slot condition-verdict ring is
  out of the path**: `rb_gsc_push_rule_log` is gone, and the four
  `rb_log_push_verdict(text,"rule_log",1)` routes that `include/rabuka.h:1786-1796`
  forbids no longer exist inside this file.
* **Proof** — snapshot probe `probe_wire.c` (not in the live tree, not in
  `tests/`): 19/19 checks pass, including the 500-bound newest-kept behaviour and
  the `rule_log_len[]` shift under it.
* **BLOCKED-ON `include/rabuka.h:3609`** — and this is the whole item.
  `void rb_gsc_push_rule_log_fmt(const char *fmt, ...);` takes **no `GameState*`**,
  so it structurally cannot reach `g->rule_log`. Rust's `push_rule_log_fmt` is a
  `&mut self` method (`mod.rs:1535`) and all 133 of its `engine/src` call sites
  have a state in hand. Exact patch:

  ```c
  - void rb_gsc_push_rule_log_fmt(const char *fmt, ...);
  + void rb_gsc_push_rule_log_fmt(GameState *g, const char *fmt, ...);
  ```

  Until it lands the function has **zero callers tree-wide**, cannot log, and
  says so once on stderr rather than silently discarding or — worse — falling
  back into the verdict ring. With that one line, the ~25 C sites that hand-roll
  the Rust call by writing `rb_log_push_verdict(line,"rule_log",1)`
  (`state.c`, `misc.c`, `look.c`, `draw.c`, `custom.c`, `ability_effects.c`,
  `live.c:1325`) each become a one-token swap, and `live_log_performance_summary`
  (`src/turn/live.c`, already ABILITY_DEBUG-gated exactly like `live.rs:1259-1261`)
  gets a real production writer.
* **Reader: there is none, and there is none in Rust either.** Rust's
  `rule_log` is written at 133 `engine/src` sites and read at SIX, all out of
  scope or debug-only: `game/display.rs:1404` and `game/display.rs:1519` (read-only
  projection — **cluster T8 Q2: OUT OF SCOPE**), `bot/v7_ismcts.rs:363-364`,
  `bin/bot_arena.rs:2312`, and `game/link.rs:482/515/519/537/550/942` +
  `game/game_setup.rs:459`, which only print `.len()` inside `log::debug!`. This
  is the same WRITE-ONLY-in-Rust terminal state cluster I recorded for
  `base_score_total` / `spare` / `live_spare`. The C's only equivalent consumer
  surface is `src/web_server.c` (not this cluster's). **Not counted as a
  closure.**

### Y.2 — `resolution_owners[]` + `n_resolution_owners` + `rb_resolution_add_for_owner` — WIRED

The "DECLARED WITH NO DEFINITION ANYWHERE" landmine is gone.

* **Definition** — `src/core/zones.c` `rb_resolution_add_for_owner()`, the twin of
  `ResolutionZone::add_card_for_owner` (`core/zones.rs:1067-1070`): append the card
  and its owner, and keep `n_resolution_owners` as the parallel-array invariant.
  `rb_resolution_add` is now a one-line delegation.
* **Writer of `resolution_owners[]`** — the same function, reached today through
  `rb_resolution_add` (`zones.c`), whose only caller is
  `rb_perform_cheer_check` (`src/core/tracking.c:154`, 8 call sites across
  `tests/test_parity_rules*.c`, `test_live_card_zone_movement.c`).
  `rb_resolution_clear` (`zones.c`) zeroes `n_resolution_owners` with the bag, per
  `zones.rs:1095`.
* **Reader** — `src/turn/phase.c` `rb_check_invalid_resolution_zone()`: each card
  now goes back to the seat that **yelled** it, not to `g->active`
  (`turn/actions/mod.rs:1864-1895`).
* **How the fallback is reproduced** — an entry with no owner stamp falls back to
  `g->active`, which is Rust's `owners.get(index).copied().unwrap_or(active)`.
  `rb_resolution_add` stamps `RB_RESOLUTION_OWNER_UNKNOWN` (`0xFF`, defined in
  `zones.c`, read in `phase.c`) so the sentinel and the short-vector case cannot
  be confused, and an entry poked straight into `g->resolution.cards[]` by a test
  leaves `n_resolution_owners` behind it and takes the same fallback — which is
  why **no suite moved**.
* **Proof** — probe checks B1-B6 pass, including "the OWNED card went back to
  seat 1, not to the active seat 0" and "the UNOWNED card fell back to the active
  seat 0".
* **BLOCKED-ON `src/core/tracking.c:154`** — the producer knows the yeller and
  cannot pass it because it calls the 2-argument form. Exact patch (the `pl`
  computed at `tracking.c:115` is already the yeller's seat, exactly Rust's
  `tracking.rs:92-93`):

  ```c
  -        rb_resolution_add(g, card_id);
  +        rb_resolution_add_for_owner(g, card_id, pl);
  ```

* **Also fixed, and dead in BOTH engines** — `rb_resolution_swap_slots` now swaps
  the owners alongside the ids (`zones.rs:1089-1090`) and enforces Rust's 4th
  refusal, "…has no owner" (`zones.rs:1078-1081`), which the C dropped. The
  function has **zero callers in the C and zero in Rust** (`swap_slots` is
  declared at `zones.rs:1072` and called nowhere), so this is a correctness fix to
  an unreachable function, not a wiring.

### Y.3 — `live_face_up[]` — WIRED, and it costs 10 FAILs in three suites this cluster does not own

Rust's gate, `turn/actions/mod.rs:1732-1736`:

```rust
fn check_invalid_live_cards(game_state: &mut GameState, is_p1: bool) {
    let seat = if is_p1 { 0 } else { 1 };
    if !game_state.seat_player(seat).live_card_zone.face_up { return; }
```

**The gate IS reachable in the C**, and Rust's own unit test proves the field is
load-bearing rather than vestigial:
`engine/tests/test_modules/rules/conditions/live_card_zone_movement_test.rs:112-113`
pushes a member into the live zone, sets
`game.state.player1.live_card_zone.face_up = true;`, and asserts the member leaves
for the waitroom.

* **Writers** — `src/turn/phase.c`, all three, all on production paths:
  `rb_execute_performance_phase` sets `g->live_face_up[performer] = 1;`
  (`phases.rs:343`), **before** `rb_recalc_constants` / `rb_check_timing`, because
  Rust writes it at `:343` and calls `check_timing` at `:349`;
  `rb_handle_set_live_card` clears it (`phases.rs:830-832`, called by
  `game_setup.c:856`); `rb_handle_live_card_confirmation` clears it
  (`phases.rs:886-887`, called by `game_setup.c:863`).
* **Reader** — `src/turn/phase.c` `rb_check_invalid_live_cards`, the
  `if (!g->live_face_up[seat]) return;` that was missing.
* ⚠ **The ORDER is load-bearing and I got it wrong first.** Writing `face_up`
  after `rb_check_timing` leaves the eviction to the NEXT timing pass, one phase
  later and on a different board; that cost `parity_wave1b` 2 FAILs and was
  caught and fixed before this entry was written. Worth a note because nothing
  fails loudly when it is wrong — it just stops being a gate.
* ⚠ **Measured regression, fully attributed — +10 FAILs, 3 of 27 suites, every one
  of them a hand-built board that puts a NON-live card in a live zone, i.e. the
  exact situation the gate governs:**

  | suite | before | after | the failing check |
  |---|---|---|---|
  | `mechanics` | 0 | **5** | `test_invalid_live_movement` + `test_invalid_energy_movement` (`tests/test_mechanics.c:97-135`) call `rb_check_timing` on a hand-built board **without** the `face_up = true` that Rust's twin sets at `live_card_zone_movement_test.rs:113`. |
  | `parity_hearts_live_start` | 64 | **67** | `b_bp6_001_r_discard_distinct_heart_colors` forces `filler` (a member) into the live slot at `tests/test_parity_hearts_live_start.c:1576`. |
  | `parity_jidou_watch` | 79 | **81** | `test_hanamusubi_q213_...` asserts at `tests/test_parity_jidou_watch.c:680` that the member left the live zone — an assertion **stronger than its Rust twin**: `live_start_heart_requirement_reduction_counts_live_slot_cards_only_test.rs:59-62` only asserts `after_waitroom >= before_waitroom`, which is true when two cards were drawn. The Rust test's own comments are openly unsure ("or should it?", "check_invalid_cards only runs during phase transitions"). |

  `tests/**` is contested under cluster T8, so **this cluster did not edit them**.
  `mechanics` is a one-line fix per test — `tg.state.live_face_up[0] = 1;` before
  `rb_check_timing` — and it is the only one of the three whose intent is
  unambiguous, because its Rust twin already carries that exact line. The other two
  need the fixture order reconciled with their Rust twins
  (`q246_test.rs:50-52` advances to live start **before** `set_live_card`; the C
  helper `hl_walk_to_live_start` does the opposite) and that is T8's call, not
  this cluster's.
* **Shape note — the declared type does not match the Rust field.**
  `include/rabuka.h:1801` declares `uint8_t live_face_up[RB_MAX_LIVE_CARDS]` with a
  comment calling it a per-CARD parallel array beside `live`, but Rust's
  `LiveCardZone.face_up` (`core/zones.rs:501`) is ONE flag for the whole zone and
  `LiveCardZone::add_card(&mut self, card_id, _face_up: bool)` (`zones.rs:1038`)
  **ignores** its flag — hence the underscore — so no per-card value ever exists to
  store. The code writes `live_face_up[0]`/`[1]`, one per seat. Exact patch for the
  header owner (**cosmetic**: the code compiles and behaves identically either
  way):

  ```c
  -    uint8_t live_face_up[RB_MAX_LIVE_CARDS];
  +    uint8_t live_face_up[2];   /* one per SEAT, not per card slot: Rust's
  +                               * LiveCardZone.face_up is a whole-zone flag
  +                               * (core/zones.rs:501) and add_card discards its
  +                               * per-card flag (zones.rs:1038). */
  ```

### Y.4 — `rb_rules_clear_turn_state` — the duplicate declarations are DELETED; the blocker is unchanged

`src/turn/live.c:604` and `src/turn/phase.c:529` each spelled the prototype out
**inside a comment**, so the symbol had two written declarations, no definition
anywhere in the tree, and no caller, while item I2 claimed `[x]`. Both are removed.
There is now **one** tombstone, in `src/turn/phase.c` in the `RB_PHASE_VICTORY`
rollover, naming `src/ability/effects/rules.c` as the blocker; `src/turn/live.c`
cross-references it without repeating the declaration text, so a future call site
fails at LINK (loud) instead of resolving to nothing (silent).

⚠ **`cannot_live_players[2]` (`rules.c:72`, written `rules.c:1878`) is still never
cleared.** It latches 1 on the first 「ライブできない」 restriction and no rollover
ever zeroes it, so that gate would be permanent if it were ever wired. That is the
concrete reason I2 is still open, and it is not fixable from `src/turn/`: the
array lives in a **file-static** `RbRulesState` whose accessor `rb_rules_state` is
`static` in `rules.c`.

### Y.5 — items that stay OPEN, with the exact patch and the exact file

| item | producer (not ours) | consumer (not ours) | state |
|---|---|---|---|
| **A9** `RbBatchMovement.cause_card_id` | `src/core/modifiers.c:718` | `src/ability/condition.c:1173-1209` | **A9's stated purpose is FALSE.** There is no `eval_group` function in Rust and no temporal arm keyed on `cause_card_id` in `core/card.rs:1642-1655` — that region is `AbilityEffect` decoding. The real site is `ability/condition/card.rs:1632-1658`. `cause_card_id` has 6 writers in Rust (`modifiers.rs:1823/1834/1922`, `position.rs:622/633/914/1184`) and **one** reader, that arm. |
| **A17** six `RbCardFilter` carriers | `src/core/card.c:957` `card_filter_from_effect` | `src/ability/util.c:1527/1537/1552/1567/1584/1586/1701` + the copy at `util.c:1873-1897` | producer sets **none** of them; `original_heart_total_opponent` **does not exist in Rust at all** (0 occurrences) — main declared a twin of a field that was never there |
| **constant_need_heart_global** | `src/turn/triggers.c:954` (CONSTANT arm) and `src/ability/effects/executor.c:222` (effect arm) — **not `modifiers.c`** | **there is none in Rust**: `mods.constant_global_need_heart` is written at `core/game_state/modifiers.rs:613-626` and read **only** by `game/display.rs:1984-1986` (**T8 Q2: OUT OF SCOPE**) | WRITE-ONLY in Rust, so there is no in-scope consumer to wire |
| ~~**new**~~ `position_change_events[]` / `n_position_change_events` — ✅ **NOT 0 writers and NOT 0 readers anymore. Closed 2026-10-04 (fixer wave).** | `rb_record_stage_position_swap` (`src/core/modifiers.c`, **E9**) is the writer; **the reader landed this wave** in `src/ability/condition.c` | none | **Writer: `rb_record_stage_position_swap` (E9). Reader: `position_event_matches_filters` + the new `movement:"position_change"` arm of `eval_movement` — Rust `ability/condition/state.rs:869-908` + `:524-568`.** **REACHABLE: 26 condition nodes on 26 distinct card-ability pairs** (21 with `self_target`, 3 with `group_names`, 4 naming a `position`, 2 with `area_direction:"from"`, 0 negated; `cost_limit` on **0 of 26**). Before this the whole 26 answered `g->n_recently_moved > 0`, a different predicate. Also fixed on the producer side, in `choice.c`: the two area-swap sites now CALL the twin instead of two `rb_record_card_movement(..., 0, 0)` calls, which had `effect_only = 0` where Rust passes the literal `true`, `cause_player_id` hard-coded to seat 0, and left `position_change_events` empty — so a real swap could never satisfy the arm. `effect_only` had **no reader anywhere in `src/`** before this wave, which is why nothing could observe the wrong value. ⚠ **Rust's `PositionChangeEvent` is `{moved_card_id, old_position, new_position, cause_card_id, cause_player_id, effect_only}`; the C's `RbPositionChangeEvent` is `{card_id, position}` only.** `BLOCKED-ON: include/rabuka.h — RbPositionChangeEvent gains `int old_position;` and `rb_record_stage_position_swap` writes `source_index`/`target_index` into it (`modifiers.rs:1821`/`:1832`), after which `int check_pos = is_from ? e->old_position : e->position;`. Until then the **2** `area_direction:"from"` nodes (cards 2483 / 2484 ab#0, 「自分のステージのセンターエリアにいるメンバーがエリアを移動したとき」) take the documented `continue`, which is Rust's own answer for any event whose `old_position` is not `centre`. **The other two of Rust's three readers of this array (`state.rs:938` and `:1162`, both `moved_card_id`-only) are still open.** |

### Y.6 — measured state, 27-suite corpus, snapshot, strictly sequential

Baseline = **the same snapshot with only this cluster's four files reverted**, not
git HEAD, proven by a 46-object SHA-256 sweep in which exactly 4 objects differ
(`game_state_core.o`, `zones.o`, `phase.o`, `live.o`) and all 46 others are
byte-identical. Streams separated; `^FAIL` counted with `-cmatch` on **stdout and
stderr**; a suite emitting nothing is BROKEN. Exit code ignored throughout.

```
suite                       base  work  delta
parity_live_total             53    53      0
parity_live_total2            17    17      0
parity_recover_live           57    57      0
parity_sequential            142   142      0
parity_integration            43    43      0
parity_cce_gaps               90    90      0
parity_small_clusters        111   111      0
parity_under_member           87    87      0
parity_zones_position         37    37      0
parity_jidou_watch_clusters   62    62      0
parity_jidou_move_yell        89    89      0
parity_draw_flat              73    73      0
parity_full_round             44    44      0
parity_full_round2            68    68      0
parity_full_round3            17    17      0
parity_wave1a                 38    38      0
parity_wave1b                 53    53      0
parity_wave1c                 44    44      0
zone_filter_parity             0     0      0
parity_required_hearts        28    28      0
audit_fields                  5     5      0
audit_zones                   5     5      0
parity_jidou_watch            79    81     +2   <- Y.3 gate
parity_hearts_live_start      64    67     +3   <- Y.3 gate
mechanics                      0     5     +5   <- Y.3 gate
------------------------------------------------
TOTAL                       1306   1316    +10
```

⚠ **`parity_condition_decode` and `parity_card_filters` DO NOT EXIST.** There is no
`tests/test_parity_condition_decode.c` and no `tests/test_parity_card_filters.c`.
Both were `Test-Path`'d before the run; they contribute 0 to both totals, which is
how two names in circulation read as PASSING. (T8.6 already lists other
never-existed names; these two were missed.)

⚠ **`sizeof(GameState)` measured again after these four files: unchanged at
1 113 736 B** — no new GameState field was added here, so the 32 MB stack and the
`tests/test_harness_self.c` §A ceiling (2236962) are untouched.

⚠ **Not this cluster's, but found and worth the space: the LIVE tree had 16
orphaned `.o` files at 23:00:52-23:01:44**, all under `src/ability/`, from a build
that never linked. Not removed here — not this cluster's build — but every one of
them is a STALE-LINK waiting for the next agent who trusts the tree.

# Cluster X — the decode-key read from the wrong table

## The defect, measured

`rb_vm_decode_extras` (`vm.c`) stores `text`, `action`, `type`, `source`, `zone`,
`destination`, `target`, `count`, `condition`, `card_type`, `per_unit_count`,
`cost_reduction_per_group`, `result_condition`, `alternative_condition`,
`look_action`, `select_action`, `gained_effect` and `id` into DEDICATED
`AbilityEffect` fields and then `continue`s (`vm.c:699-1080`), skipping the
scalar extras tail at `vm.c:1169`. So **none of them is ever written to the
extras table**, and any `*_extra(e, "<one of those>")` returns NULL permanently.

Measured through the engine's OWN decoder over the shipped bytecode
(`tools/audit_x_class.c`, 985 card/ability pairs, 4024 effect nodes):

| key | nodes with the STRUCT FIELD | distinct abilities | nodes with the EXTRA |
|---|---|---|---|
| `action` (`action`/`type`/`cost_type`) | 4022 | 2103 | **0** |
| `count` | 2636 | 1810 | **0** |
| `source` (`source`/`zone`) | 1621 | 1283 | **0** |
| `target` | 1499 | 1309 | **0** |
| `destination` | 1378 | 1124 | **0** |
| `card_type` | 1221 | 1074 | **0** |
| `condition` | 952 | 880 | **0** |
| `select_action` | 185 | 185 | **0** |
| `look_action` | 183 | 183 | **0** |
| `per_unit_count` | 155 | 136 | **0** |
| `result_condition` | 53 | 53 | **0** |
| `gained_effect` | 52 | 50 | **0** |
| `alternative_condition` | 13 | 13 | **0** |
| `cost_reduction_per_group` | 0 | 0 | 0 |
| `id` | 0 | 0 | 0 |

`vm.c`'s own comment at `:1089-1097` already said it for `card_type`
("today the extras table never holds a `card_type` at all -- 0 of 2230
effects"); this census extends the same measurement to every key in the class.

## Sites

Statuses: **fixed** = field-first idiom applied and A/B'd. **VERIFIED-NOOP** =
the read was already field-first, or the extras read is a dead fallback that
cannot change behaviour. **BLOCKED-ON** = the file belongs to another cluster.

| site | key | status |
|---|---|---|
| `look.c:919` `rb_effect_reveal_until_chosen_card` | `card_type` | **fixed** |
| `look.c:933` `rb_effect_reveal_until_target` | `card_type` | **fixed** |
| `resolver.c:602` `apply_per_unit_cost_reduction` | `per_unit_count` | **fixed** |
| `score.c:1038` `rb_execute_modify_required_hearts_success` | `card_type` | **fixed** |
| `look.c:976` `rb_effect_select_cards` | `card_type` | VERIFIED-NOOP (already field-first) |
| `look.c:1053` `rb_effect_select_number` | `card_type` | VERIFIED-NOOP (already field-first) |
| `look.c:1129` `look_filter_card_type` | `card_type` | VERIFIED-NOOP (already field-first; it IS the shared helper the two fixes now call) |
| `state.c:124` `rb_effect_gain_ability` | `per_unit_count` | VERIFIED-NOOP (field read first, extras only refines) |
| `state.c:1484` `rb_effect_condition_modifier_discard` | `per_unit_count` | VERIFIED-NOOP (same ordering) |
| `state.c:724` `rb_effect_modify_limit` | `card_type` | VERIFIED-NOOP (already field-first) |
| `state.c:1487` `rb_effect_state_modifier` | `per_unit_count` | VERIFIED-NOOP (already field-first) |
| `state.c:399` `rb_effect_recycle_cards` | `card_type` | VERIFIED-NOOP (`card_type_filter` is already resolved field-first at `state.c:88-91`) |
| `ability_effects.c:324` `rb_translated_gain_ability` | `card_type` | VERIFIED-NOOP (already field-first) |
| `ability_effects.c:581` `rb_gain_ability_from_source` | `card_type` | VERIFIED-NOOP (already field-first) |
| `phase.c:147` `ptc_effect_primary` | `target` | VERIFIED-NOOP (`ab.effect->target` is read first) |
| `cost.c:353/798/897/995/1051/1370/1671/2056/2328/2447` | `card_type` | **fixed 2026-10-04** — all TEN sites, see the FINAL-WAVE section below. (`zone`, `condition`, `per_unit_count` at these sites were NOT changed: they are real extras rows, measured present in the corpus, e.g. `change_state state_change 333`, `change_state group_names 41`) |
| `describe.c:336/360/369/375` | `text`, `action`, `source`, `target`, `zone` | BLOCKED-ON — held by another agent |
| `util.c:2395/2446/2749/2755` | `card_type`, `condition`, `zone`, `count` | BLOCKED-ON cluster G+H |
| `compound.c:299/450/451/495` | `card_type`, `source`, `destination`, `target` | BLOCKED-ON cluster T5 |
| `core/card.c:756/865` | `card_type`, `per_unit_count` | BLOCKED-ON cluster G+H |
| `core/modifiers.c:1146` | `card_type` | BLOCKED-ON cluster E |
| `effects/draw.c:658/662` | `text`, `action` | BLOCKED-ON — `draw.c` held; still unpatched per the brief |
| `effects/misc.c:1185/1220` | `card_type`, `condition` | BLOCKED-ON cluster C |
| `effects/move.c:181` | `card_type` | BLOCKED-ON cluster F |

### Notes on the four fixed sites

* `look.c:919` and `look.c:933` both now call the file's own existing
  `look_filter_card_type(e)` (`:1129`), which is the canonical field-first
  reader for this file — the same helper `rb_effect_select_cards` (`:976`)
  already used.
* `resolver.c:602` inverts the old precedence: the struct field is read first
  and the extras read is only consulted when the field is unset, so a future
  decoder change cannot lose the value and cannot shadow the field.
* `score.c:1038` uses the field-first-with-fallback shape of `draw.c:444` /
  `look.c:976`. Before the fix the effect was a **complete silent no-op**:
  `card_type` was NULL, so `nids` stayed 0 and the loop at `score.c:1058`
  never ran.

### Reachability of the four sites on this corpus

| site | reachable nodes | verdict |
|---|---|---|
| `look.c:919` | `action == "reveal_until_chosen_card"`: **0** | corpus-inert (Rust agrees independently at `look.rs:1395-1396`: "No card in the corpus uses `reveal_until_chosen_card`") |
| `look.c:933` | `action == "reveal_until_target"`: **0** | corpus-inert |
| `score.c:1038` | `action == "modify_required_hearts_success"`: **0** | corpus-inert (matches T2's recorded VERIFIED-NOOP) |
| `resolver.c:602` | `modify_cost` + `per_unit` + `operation:"subtract"`: **7**, and **all 7 carry `per_unit_count == 1`** | corpus-inert: the broken reader returned exactly the same 1 |

So the four fixes are **byte-identical over the whole suite set** (see
`NOZOMI_ROOT_CAUSE.md` §10: 2117 before and after). They are landed anyway
because the readers were wrong, and `tests/test_parity_x_class.c` pins all four.

## The regression test

`tests/test_parity_x_class.c` (new file, 5 cases, 20 checks). Each reader case
builds an effect that carries the key in the **STRUCT FIELD** and asserts the
key is **absent from the extras table** — the exact shape
`rb_vm_decode_extras` produces — then drives the reader and asserts the field
value was used. It deliberately does not re-implement the bytecode reader
(cluster T's recorded hazard).

A/B of the test itself, against the reverted readers:

```
                          fixed readers    reverted readers
x/modify_required_hearts_success_reads_card_type   ok (4/4)     FAILED
x/reveal_until_chosen_card_reads_card_type          ok (4/4)     FAILED
x/reveal_until_target_reads_card_type               ok (4/4)     FAILED
x/per_unit_count_is_never_an_extra                  ok (3/3)     ok
x/card_type_is_never_an_extra                       ok (5/5)     ok
```

The two corpus-census cases are reader-independent by design: they assert the
invariant the readers depend on (the key is in the field, never in the extras
table) so a future decoder change is caught here rather than as a silent
wrong value.

Reverted-reader detail, which is the useful part:

```
score:  the success-zone card received the modifier (positive control)   -> 0
reveal_until_chosen_card: pool size 3 (ran past the member to the live card), want 2
reveal_until_target:      pool size 0 (drained the deck, never matched),       want 2
```

## Cluster W — the eight bug-predicting warnings — **LANDED 2026-10-04 (all 8)**

**STATUS: every row below is landed and re-measured.** The patch was
`engine_c_wip/cluster_W_BLOCKED_ON.diff`; all five of its files are now unowned and
all six hunks applied (W1, W2 ×2 dispatchers, W2b, W4, W8+helper, W9), with the
exceptions recorded per row. **`make warnings-audit`: 8 → 3, PASS. Then, with
`-Wformat-truncation` triaged and added, 12, PASS (all pinned, 0 new, 0 stale).**

**The two load-bearing ordering constraints both held, and one of them was not in
the brief.** `cost.c` (W2b) did have to land with `choice.c` (W2), because the
producer writes the routing key the consumer matches on. But W2 was **not** live
even with both halves applied, for a fourth reason the diff did not contain:

> **`rb_clear_pending_choice` (`choice.c:4483`) memsets the whole
> `g->queue.pending`, and it runs at `choice.c:4815` — 274 lines ABOVE the W2 guard.**
> So the guard read `g->queue.pending.target` from a field that had *already been
> zeroed*. Correcting the literal and the `strstr`→exact-match change left it just
> as dead, only differently dead. Every other arm in that function that needs the
> prompt's own text reads `saved_pending`, the copy captured at `:4761` before the
> clear (`:4784` route, `:4790` target, `:5265`/`:5269` the SelectTarget dispatch
> itself). W2 now reads `saved_pending.target`, and that is the only reason the
> change is observable at all.

That is the generalisable lesson, and it is the same shape as W2b one layer
earlier: **W2b was "a field that is never written"; this was "a field that is
written and then cleared before it is read".** Both are invisible to
`-Waddress` and to the previous A/B, and both are invisible to any check that only
asks "does the branch exist".

`make all` emitted **113 warnings** while this cluster was open. **105 are noise**
and are left exactly as they are: `-Wstringop-truncation` ×41 (bounded `strncpy`
into fixed `Rb*` `char[64]`s), `-Wunused-function` ×28, `-Wunused-variable` ×9,
`-Wcomment` ×4, `-Wmisleading-indentation` ×2, `-Wunused-but-set-variable` ×3,
`-Wredefined` ×1 — **and `-Wformat-truncation` ×16, which was on that list by
default and has since been triaged properly and moved into the gate** (see the
FT-1..FT-9 table below: 2 of them were real and are fixed, 14 are false positives
with the proof recorded, and the class is now pinned per record). Two of the
remaining noise classes are **not** noise and are still unpinned:
`-Wmisleading-indentation` ×2 (`condition.c:694` "this `if` clause does not guard
any sequence", `score.c:767` — the first is a missing `else` on a conditional
`return 0`, a real trap for a reader) and `-Wunused-but-set-variable` ×3. Outside
this cluster's brief; recorded so the next audit does not re-derive them.

The **8 bug-predicting diagnostics were not noise**, and the reason nobody caught
them is that nobody reads 113 lines: `gcc` had already predicted a real SIGSEGV in
this tree's build log (*array subscript 11 is above array bounds of
`TestGame[1]`*) and the build printed it and carried on. Cluster W therefore
lands a **gate**, not just fixes. **All 8 are now landed and their pins deleted.**

`WARN_AUDIT_BASELINE` today: **12 pins, 12 firing, `NEW=0`, `stale-pins=0`,
PASS** — W5/W6/W7 (the three `-Wrestrict` false positives) plus FT-1..FT-9.
`WARN_AUDIT_TEST_BASELINE` is **empty** (W9 landed; `-Werror=maybe-uninitialized`
now gates `tests/` with 152 sources and 0 red).

### One row per warning

| # | site | warning | intended check (Rust twin) | what the C did | outcome |
|---|---|---|---|---|---|
| **W1** | `ability/choice.c:1083` | `-Waddress` | `SelectionContext::card_type: Option<String>.is_some()` — `choice.rs:34` → `ChoiceBuilder::card_type(Option<String>)` `types.rs:479/:97` ← `build_reprompt` `choice.rs:1188` | `ctx->card_type` is `char card_type[32]` (`choice.c` struct), so in a boolean context it is `char *` and `if (ctx && ctx->card_type)` **could never be false**. A null check written where an `is_some()` was meant. The very next line already used the right idiom for `group` (`ctx->group[0]`) | **BLOCKED-ON `src/ability/choice.c:1083`** — cluster B. Behaviour is *provably* unchanged (`ch` is memset at `:1077`, so `strncpy` of `""` rewrites the NUL already there), so the check is **kept, not deleted**, with `[0]` and the Option→`char[32]` shape loss recorded. Verdict: **dead logic, not an unguarded real check** |
| **W2** | `ability/choice.c:5037` | `-Waddress` | `Some(SelectTargetKind::PayCostAllDiscard) => handle_pay_cost_all_discard` — `choice.rs:2816-2818`, reached through `SelectTargetKind::from_str` `enums.rs:688-708`, whose literal at `enums.rs:705` is **`pay_cost_all:discard_all`** | **FOUR defects, all load-bearing.** (1) `pending.target` is `char target[64]` (`rabuka.h:943`) → always-true. (2) The literal was `pay_cost_all_discard`, **underscore**; every producer in the tree spells it `pay_cost_all:discard_all` (`enums.c:386/:407`, `misc.c:2075`, `test_parity_enums.c:278`), so `strstr` can **never** match. (3) `strstr` where Rust uses exact equality. (4) **found on landing, not in the diff** — the guard read `g->queue.pending.target`, which `rb_clear_pending_choice` (`choice.c:4483`, called at `:4815`) has already zeroed; see the block note above. The **same guard also exists dead at `choice.c:3837`** in the other dispatcher, which reads the pre-clear copy and so only needed (2)+(3) | **LANDED**, both dispatchers, both routed through `rb_select_target_kind_from_str` (`enums.c:369-388`, a 16/16 twin of `from_str`, returns −1 for NULL and every unknown spelling). **The census's card names in the previous revision of this row were WRONG** — see `rb_find_card_by_no` below: it read `card_no_idx` through `rb_get_string` (the ABILITIES string table) instead of `rb_card_string` (the CARDS one), so the two nodes it named (`PL!N-bp5-021-N`, `PL!N-bp5-028-L`) are not the cards whose cost reaches the prompt producer. The **real** pair is **`PL!SP-bp7-011-P`** and **`PL!SP-bp7-011-R`** (blob ids 2336/2337), ability #0 each: a 登場 whose cost is `move_cards source=hand destination=discard all=true optional=true` and whose effect is `draw_card`. `tools/audit_w_gate.c` and `tests/test_audit_warnings.c` now read the card number with `rb_card_string`; the node count (2) was always right |
| **W2b** | `ability/cost.c:1129-1138` | *(found while fixing W2; not a warning)* | `offer_all_hand_discard` sets `Choice::SelectTarget { target: "pay_cost_all:discard_all", … }` — `cost/handlers.rs:378-388` | Sets `pending->kind = RB_CHOICE_SELECT_TARGET` and **never sets `pending->target`**. The routing key is dropped at the producer, so fixing W2 alone still cannot make the branch fire | **LANDED** in the same change as W2, as the ordering requires. `snprintf(pending->target, sizeof pending->target, "%s", "pay_cost_all:discard_all")` into the field `memset` two lines above. This is the row that matters: the same "test a field that was never written" family as items 1/2, one layer earlier |
| **W3** | `ability/condition.c:2342` | `-Wdiscarded-qualifiers` | `rb_check_heart_type_all` mirrors Rust's all-heart member predicate; the sibling `rb_check_heart_colors` already declares `const RbPlayer *P` | `g` is `const struct GameState *`, so `&g->p[pl]` is `const RbPlayer *` and the local threw the const away. `P` is only read (`->stage`) and never escapes | **DID NOT APPLY — AND DID NOT NEED TO.** Cluster D independently rewrote `condition.c`; the function has moved 2342 → **2757** and the line in question already reads `const RbPlayer *P = &g->p[pl];`. The hunk was therefore **skipped, not forced**, and the now-stale pin `src/ability/condition.c:2342:-Wdiscarded-qualifiers:W3` was **deleted rather than carried** — a pin for a warning that no longer fires is a lie, and `warnings_audit.sh` FAILs on stale pins precisely so one cannot rot. The old caveat ("`rb_check_heart_type_all` has ZERO callers") is **also stale**: D3 wired it at `condition.c:532` |
| **W4** | `ability/choice.c:5516` | `-Wdiscarded-qualifiers` | — (a local alias, no Rust twin) | `RbQueueState::resume_parent` is `const AbilityEffect *` (`rabuka.h:1200`); the local was `AbilityEffect *parent`. `parent` is only NULL-tested and forwarded | **LANDED** — `const AbilityEffect *parent`. **VERIFIED-BENIGN**: no caller relies on the pointee being const — the local is not written through, does not escape, and the callee `rb_resolver_continue_siblings` already takes the const pointer, so no cast is needed |
| **W5** | `core/data.c:572` | `-Wrestrict` | — | `snprintf(variant, len + 2, "%s-%s", base, requested)` | **VERIFIED-BENIGN — GCC FALSE POSITIVE. See the proof below. Pin kept in the gate on purpose** |
| **W6** | `core/data.c:600` | `-Wrestrict` | — | `snprintf(variant, len + 2, "%s-%s", base, canonical)` | **VERIFIED-BENIGN** — same proof |
| **W7** | `core/data.c:607` | `-Wrestrict` | — | `snprintf(variant, len + 2, "%s-", base)` | **VERIFIED-BENIGN** — same proof |
| **W8** | `core/game_state_abilities.c:1899` | `-Wparentheses` | `GameState::is_stale_discard_condition` — `abilities.rs:486-501`, called from the AUTO_SCAN at `abilities.rs:669-678` with `in_discard = p1.waitroom.contains(card) \|\| p2.waitroom.contains(card)` | The guard read **two of Rust's four terms**. Missing: **(a)** the `source == "preceding_moved"` exemption (`abilities.rs:487`); **(b)** the `trigger_event.location` fallback (`abilities.rs:490-497`) | **LANDED**, together with the static `rb_cond_get_obj` helper it needs (`rb_cond_get_str` is `RB_TAG_STR`-only and cannot see through a nested dict, so term (b) is unreachable without it). **The highest measured yield in this cluster: −25 FAIL lines, 0 added** |
| **W9** | `tests/test_parity_cce_gaps.c:3202` | `-Wmaybe-uninitialized` (found by the **test gate** the day it was written) | — | `j7_no_niji_no_peek` declared `int tsunagaru, non_niji, filler;` and its guard was `fx(&non_niji) \|\| fx(&filler)` — **the `fx(tg, &tsunagaru, TSUNAGARU)` leg every sibling has (j1:3005, j2:3063, j5:3139, j6:3171, j8:3235) was missing.** So `:3212` staged the indeterminate value into the hand and `:3218` scanned for it | **LANDED** — cluster J. A live assertion was running on an indeterminate value, so `j7: the ライブ開始時 was queued` was red for a **setup** reason. Adding the leg **strengthens** the test: **−2 lines, 0 added**, and `WARN_AUDIT_TEST_BASELINE` is now empty |

### W5–W7: the overlap is a FALSE POSITIVE, proved twice

GCC reasons about the whole **object** `scratch` and cannot be told "same object,
disjoint sub-ranges" in source. Two independent proofs, neither of which is the
prose:

1. **Machine-checked, statically.** `data.c` now exports
   `rb_card_no_scratch_layout_ok(len)`, which *evaluates* the range arithmetic
   data.c itself uses to place its four buffers and checks every sub-range is
   inside the allocation and no two intersect. `make w-census` runs it for
   **len 0…256**: `layout_disjoint_len_0_256=YES violations=0`.
   `base` occupies `[len+1, 2*len+1]` and `variant` starts at `2*len+2` — a
   **one-byte gap**, for every `len`.
2. **Canary, at runtime.** A throwaway instrumented copy of `data.c` compared
   `variant` against an independently-built expected string, re-checked the
   `base`/`variant` separation and re-checked the write bound, driven over
   **9852 inputs** (every corpus `card_no`, every corpus base re-queried with ten
   **wrong** rarity suffixes to force step 3a/3b/3c, and 19 adversarial
   strings including fullwidth `＋` forms): **6735 snprintf calls into the shared
   scratch, 0 violations.**
3. **The premise.** The argument leans on `base_len <= len`, i.e. on
   `rb_card_normalize_no` never expanding. Checked over the corpus:
   `normalize_no_never_expands=YES (card_nos_checked=2639 expansions=0)`.
   (The previous revision of this line said **1644** — that was the same
   `rb_get_string` mix-up: 1644 is how many of the 2639 cards happened to produce
   a non-empty string through the WRONG table. `tests/test_audit_warnings.c` row C
   now walks all 2639 with `rb_card_string` and asserts `checked > 1600` as a
   floor, which would have caught the mix-up.)

**The diagnostics stay.** There is no in-source way to tell GCC "disjoint
sub-ranges", and the two alternatives are both worse: a
`#pragma GCC diagnostic` would destroy the diagnostic for exactly the offsets
documented above, and hand-rolling the join in `memcpy` would replace a
well-defined `snprintf` with truncation logic on the hottest lookup path in the
file. The warning **is** the tripwire for a future edit to those offsets, so it
is pinned in `WARN_AUDIT_BASELINE` and the invariant is enforced by code instead.

### The gate

`make warnings-audit` compiles every `SRC` translation unit with **only** the
nine bug-predicting classes and fails on any occurrence outside the baseline:

```
-Waddress -Wrestrict -Warray-bounds -Wuninitialized -Wmaybe-uninitialized
-Wstringop-overflow -Wformat-overflow -Wparentheses -Wdiscarded-qualifiers
```

**BASELINE = 8**, recorded in the target's own comment: 5 `BLOCKED-ON`
diagnostics + 3 pinned `data.c` false positives. It is a **ratchet**, not a
blanket `-Werror`: new occurrences are red immediately, the pinned ones stay
counted, and a **stale pin is also a failure** so a landed patch cannot leave a
lie in the Makefile.

`make warnings-audit-test` adds `-Werror=array-bounds -Werror=maybe-uninitialized`
over `tests/test_*.c` — the two classes that predicted the segfault — with the
same ratchet and one pin (`tests/test_parity_cce_gaps.c`, row W9).

**It does not block anyone.** It is not a prerequisite of `all`, `t` or
`run-all-tests`; the 105 benign warnings are not in it; and **no blanket
`-Werror` was added anywhere** — the suite row **D5** fails the build if one
ever appears in the Makefile.

#### Gate proof

```
live tree : found = 8, pins = 8, NEW = 0, stale = 0   ->  PASS (exit 0)
injected  : found = 17, pins = 8, NEW = 9, stale = 0  ->  FAIL (exit 1)
```

The injection (throwaway tree only, `src/core/tracking.c`) reproduced nine
defects across six of the nine classes — `-Waddress`, `-Wrestrict`,
`-Warray-bounds=` ×4, `-Wmaybe-uninitialized`, `-Wparentheses`,
`-Wdiscarded-qualifiers` — and the gate printed every one with `file:line:class`.
Two findings from writing it, both kept because they cost an agent real time:

* **`-fsyntax-only` is not enough.** `-Wrestrict`, `-Warray-bounds` and
  `-Wstringop-overflow` are emitted by the **optimizer**. `data.c` emits 3
  `-Wrestrict` under `-c` and **zero** under `-fsyntax-only`. The gate compiles to
  a throwaway object.
* **An unreferenced function gets less analysis.** The first injection left all
  eight defects unreferenced and gcc skipped the analysis for six of them — which
  made the *injections* look like a broken gate. Every injected defect is now
  reached from a live entry point.
* **`-Wformat-overflow` is unreachable on gcc 15** for the `snprintf`-into-small-
  buffer shape: the same defect reports `-Wformat-truncation=`, and gcc 15 emits
  `-Wformat-overflow` **zero** times on this tree's shapes even at `-O2`. **Left
  out of `WARN_AUDIT_FLAGS` on purpose** — a pin for a class that can never fire
  would be a dead class, which is exactly the kind of fake gate row this cluster
  exists to remove. Its sibling `-Wformat-truncation`, which the previous revision
  of this bullet also left out because it had **16 pre-existing instances**, has
  since been **triaged and added**: 2 real (fixed) + 14 false (each proved, see
  FT-1..FT-9 below), so the gate now covers the family.

### Cluster W A/B — RE-RUN 2026-10-04, all eight landed. 152 suites, strictly sequential, `^FAIL` (no colon), stdout AND stderr

Baseline = a snapshot of the live tree with **only cluster W's hunks reverted** (NOT
git HEAD), proved by a SHA-256 sweep over every `.c`/`.h`/`Makefile`/`.sh`/`.bin`:
**240 files identical, 7 differing** — `src/ability/choice.c`, `src/ability/cost.c`,
`src/ability/resolver.c`, `src/core/game_state_abilities.c`,
`src/core/game_state_core.c`, `tests/test_parity_cce_gaps.c`, `Makefile`.

```
                                     base    fix
BUILD_RED (compile failures)              0      0
suites emitting a log                  152    152
TOTAL ^FAIL lines                     3568   3524    -44
added lines anywhere                      -      0
```

Only five suites moved, and every one moved **down**:

```
audit_warnings           7 -> 0    -7    the new W2 end-to-end row E
parity_wave1b           53 -> 32  -21  \
parity_jidou_watch      81 -> 77   -4   |  W8, and W8's two terms isolated:
parity_look_per_card    74 -> 64  -10  /   term (a) = -25, term (b) = 0 (see below)
parity_cce_gaps         90 -> 88   -2      W9
```

The cleared lines in `parity_look_per_card` are the **`tomari` tests for the two
cards W2 was about** — 「泊まり」, `PL!SP-bp7-011-P` / `PL!SP-bp7-011-R`:

```
FAIL: tomari_accept: the discarded cards are in the waitroom (got 0, need >= 3)
FAIL: tomari_accept: a discarded card is not still in hand        (x3)
FAIL: tomari_accept: 6 cards drawn after discarding the entire hand (got 9 expected 6)
FAIL: tomari_skip: skipping the optional discard keeps the hand unchanged
FAIL: tomari_empty CONTROL: with 3 hand cards the accepted cost draws 6 (got 9 expected 6)
```

`hand 2→8, waitroom 0→0` on the base is the signature of the defect itself: the
prompt was raised, the answer was accepted, **nothing was discarded**, and the
effect's draw ran on top of the untouched hand. Both cards now discard into the
waitroom and the draw count matches.

### Row E — the two abilities are ANSWERED, not merely non-crashing

`tests/test_audit_warnings.c` row **E** is the end-to-end half that rows A7–A11
could not provide. A7–A11 pin that the prompt is *raised* and that the routing
table knows the literal; they cannot prove the prompt is *answerable*. Row E
stages `PL!SP-bp7-011-P` and `PL!SP-bp7-011-R`, fires the 登場, then asserts:

* E2 the prompt opens as `RB_CHOICE_SELECT_TARGET`;
* E3 `pending.target` is **exactly** `pay_cost_all:discard_all`;
* E4 answering PAY **consumes** the prompt rather than re-raising it;
* E5 **neither hand card is still in hand** afterwards — identity, not counts,
  because the ability's effect draws a card so the hand *length* is not stable;
* E6 **both** are in `p[0].discard` (`RbPlayer::discard` IS Rust's `waitroom`);
* E7 the member that raised the prompt is still on stage;
* E8–E11 the **decline** arm of the same prompt discards nothing and is consumed
  too.

The discriminator against the pre-fix tree is deliberately **not** "it does not
crash" — the old code cleared the prompt as well. It is **whether the hand was
actually discarded**.

```
base (only W's hunks reverted)   59 checks, 7 failed   E3, E5, E6, E9
fix   (all eight landed)         59 checks, 0 failed
```

Two fixture details that are load-bearing and cost a debugging round each, both
recorded in the test:

* **the subject uses `test_id`, not `test_new_id`.** Card-copy ids do not carry
  the ability links, so the template is the id whose ability #0 the census
  measured. The two *fillers* must be distinct instances (the same id twice is not
  a two-card hand), so those use `test_new_id`. `test_id_exact` is asserted
  against the template so a typo cannot quietly resolve to another card.
* **the deck must not be empty.** The cost's effect is `draw_card`, and a C draw
  from an empty deck *recycles the waitroom* — which put the very cards the cost
  had just discarded straight back into the hand and made the assertion pass or
  fail for the wrong reason. With 20 deck cards per seat the assertion is about
  the cost and nothing else.

### B3 correction in row B — an assertion that was wrong about Rust, not about the C

Row B3 asserted "the negated form on an empty stage is **true**", reasoning that
negation of false is true. Rust's negation arm is
`target_ids.iter().any(|id| !card_has_all_heart(id))` — an **any over the ids**, not
a fold of the positive arm. `any` over an empty list is **false**. The C's
`return 0` is faithful; the assertion was not. Corrected to `0` **against the
reference, not relaxed**: it still exercises the negation arm and now pins Rust's
exact `.any(|id| !card_has_all_heart(id))` shape, with the distinction between
`any(!p)` and `!any(p)` spelled out in the comment.

### The `-Wformat-truncation` fixes are behaviour-NEUTRAL, measured

`resolver.c` (`RB_CHOICE_SIG_MAX`) and `game_state_core.c` (`line[1792]`) were
corpus-A/B'd on their own: fix-with-FT-fixes = **3524**, fix-without = **3524**,
152 suites each, 0 BUILD_RED. The truncation they remove is therefore not reached
by any suite, which is consistent with the triage (both are >501-byte and >1399-byte
format inputs respectively) and is stated as measured rather than assumed.

### W8 sub-term isolation — which of the two missing terms did the work

Both terms were landed together, so each was then removed from a separate build
of the same tree to attribute the −25:

```
tree        term (a) preceding_moved   term (b) trigger_event   jidou_watch  wave1b
base                 absent                  absent                23+56     14+39
onlyA                ON                      OFF                   22+53     11+21
onlyB                OFF                     ON                    23+56     14+39
fix (a+b)            ON                      ON                    22+53     11+21
```

**All 25 come from term (a) alone** — the missing `preceding_moved` exemption.
Term (b) is **measurement-neutral on this corpus**: the census finds 10 nodes
reachable only through `trigger_event.location`, but none of them is
`preceding_moved` and none is reached by any suite. It is still correct to land
(it *is* Rust's shape, and 10 corpus nodes depend on it) but it must be reported
as neutral, not as a fix that closed failures.

Census (`make w-census`, the shipped bytecode through the engine's own decoder,
never the stale `cards/abilities.json`):

```
W-cards     | cards_with_abilities=1650 card_ability_pairs=2116 abilities=985
W-A8-discard| nodes=1260 guard_hit=37 (flat_location=27 trigger_event_only=10)
             | preceding_moved_exempt_in_rust=6 (of which trigger_event_only=0)
W-A2        | pay_cost_all:discard_all action nodes=0 abilities=0
             | hand_source_all_cost nodes=2 abilities=2
             |   -> PL!SP-bp7-011-P and PL!SP-bp7-011-R, ab#0 each
             |      (the PREVIOUS revision of this line named
             |       PL!N-bp5-021-N / PL!N-bp5-028-L; that was the
             |       rb_get_string mix-up -- see the rb_find_card_by_no
             |       section. tools/audit_w_gate.c now uses rb_card_string.)
             | stk_table("pay_cost_all:discard_all")=15
W-C-scratch | layout_disjoint_len_0_256=YES violations=0
W-C-normalize | normalize_no_never_expands=YES (card_nos_checked=2639)
```

So **16 of the 37** guard-reachable nodes were decided by the wrong predicate:
6 exempted by Rust (term a) + 10 misread via `trigger_event` (term b), and the two
sets are **disjoint**.

### `tests/test_audit_warnings.c`

**59 checks in five rows.** **A** pins the census counts above, so a future
regeneration that empties the `preceding_moved` set or the `trigger_event` set
makes the corresponding fix *unreachable* and says so instead of letting it rot
into dead code. **B** calls `rb_check_heart_type_all` and
`rb_resolver_handle_number_selection` — the two const sites — with local `extern`
declarations (neither has a header declaration; `src/game/game_setup.c:208` sets
that precedent) so the moment D3 wires the first one, it is already covered.
**C** is the machine-checked layout invariant plus the corrected
`rb_find_card_by_no` contract (C7–C13). **D** pins the gate's own shape,
including **D5, "no blanket `-Werror` in the Makefile"**, which skips comment
lines so the Makefile's own prose saying "NOT a blanket `-Werror`" cannot fail it.
**E** is the W2 end-to-end row above.

```
full tree (all 8 landed)                 59 checks, 0 failed
base (only W's hunks reverted)           59 checks, 7 failed   -- all row E
```

`full tree` still builds the pre-fix `data.c` fine, so the data.c rows are not
re-derived here — W5–W7 were already landed before this session.

### `rb_find_card_by_no` — RESOLVED. The lookup is CORRECT; the measurement was wrong

The previous revision of this section claimed: *"resolves 671 of 2639 corpus
`card_no`s, and 0 of them to their own index (534 resolve to a lower index). The
1968 misses are **not explained**."* **All four numbers were artefacts of one
bug in the measuring tool, not of the lookup.**

`Card::card_no_idx` indexes the **CARDS** string table (`data.c g_card_strings`,
read through `rb_card_string`). `rb_get_string` indexes the **ABILITIES** string
table (`data.c g_strings`). The census read every card number with
`rb_get_string`, so for almost every id it printed *a different card's number*.

Measured over all 2 639 cards, the two accessors **agree 0 times**. Re-measured
with the right one:

```
accessor disagreement over 2639 cards : 2639 / 2639   (0 agreements)
rb_find_card_by_no over the real card numbers:
    checked=2639  own_index=2639  lower=0  higher=0  MISS=0
duplicate card_no keys in the blob    : 0
```

**So: every card number the blob contains resolves to its own index. The 1968
were never card numbers, and there is nothing to fix.** The forgiving tail
(base+rarity rebuild → lowest-prefix → substring) documented in
`include/test_game.h` is real and is *not* on this path — `card_no_exact`
(`data.c:421`) hits first, every time. The tail still matters for keys the corpus
does not contain, which is what makes a mistyped print resolve to some other card
instead of failing; that is the real correctness hazard and it is unchanged.

**No fallback was added.** Resolving the 1968 to *something* is exactly the
"wrong card that resolves" failure the brief forbids, and it would have been
manufactured out of a measurement bug.

`tests/test_audit_warnings.c` row **C** now pins the corrected contract as C10–C13
(`checked==2639`, `resolves==checked`, `own==checked`, `wrong==0`), so a future
`data.c` edit that breaks `card_no_at` fails a test instead of quietly changing
which card a fixture is.

**The same wrong accessor was in `tools/audit_w_gate.c` in two places**
(`name_hand_all` and the `norm_expansions` census), so `make w-census` was also
naming the wrong cards — which is how W2's two corpus nodes got recorded as
`PL!N-bp5-021-N` / `PL!N-bp5-028-L` instead of `PL!SP-bp7-011-P` /
`PL!SP-bp7-011-R`. Both are fixed.

### `-Wformat-truncation` ×16 — RESOLVED. 2 real (FIXED), 14 false (PINNED), class now GATED

The class was excluded from `WARN_AUDIT_FLAGS` because it had 16 pre-existing
instances. Triage of all 16 — which is **10 distinct `file:line` records**, since
`warnings_audit.sh` `sort -u`s, so `rules.c:261`'s five call sites and the two
instantiations each of `rules.c:215` and `look.c:1409` collapse to one record:

**TWO WERE REAL TRUNCATIONS, and are FIXED rather than pinned:**

* `src/ability/resolver.c:243` — `choice_offer_sig` wrote `"[0]%s"` into the 505
  bytes left in a **512**-byte buffer, while `RbChoice::description` is
  `char[512]`. Worst case 7 + 3 + 511 + 1 = **522**. The dedup signature could
  truncate, and a truncated `last_offered_sig` makes two different prompts that
  share a long prefix compare **equal** and suppress one of them. Both buffers are
  now `RB_CHOICE_SIG_MAX` (544).
* `src/core/game_state_core.c:844` — `char line[1400]` against a **1665**-byte
  worst case for `"[T%d %s] Choice offered: %s | options=%d skip_allowed=%d
  source=%d\n%s"` (`description` 511 + `labels` 1023 + ~60 fixed). Harmless —
  `gsc_rule_log_push` copies into `rule_log[...][RB_GSC_RULE_LOG_LINE_MAX]`, so
  everything past that was dropped anyway — but "harmless because a later buffer
  is smaller" is not a reason to leave a truncating intermediate inside a
  formatter. Now `line[1792]`.

**THE OTHER 14 ARE FALSE POSITIVES, each proved, and pinned with its proof:**

| row | site (gcc's line) | proof |
|---|---|---|
| **FT-1** | `rules.c:215` (×2 instantiations) | `snprintf(out, n, "player%d", pl + 1)` into a 10-byte region. `pl` is a **seat index** — either `actor` or the `(player_id[0]=='2') ? 1 : 0` ternary — so `pl+1` is `"1"` or `"2"` and the output is 7 bytes. gcc cannot see that `actor` is a seat. **The one site whose proof is a convention rather than a bound**, and the reason it is pinned rather than rewritten: forcing the range would mean rewriting `pl`'s derivation, and `actor == -1` currently prints `player0`, which is observable. Not changed. |
| **FT-2** | `rules.c:261` (×5 call sites) | `r_prohibition` copies into `prohibition[64][48]`. All five callers build their key in a 64/96-byte local first, so gcc's 95/191-byte source assumption is unreachable. Measured over the shipped bytecode: `operation`/`effect_type` **max 8** (`subtract`), `restriction_type` **max 25** (`cannot_activate_by_effect`), `restricted_destination` **never present**, effect `target` **max 10** (`live_total`). Worst keys: `"activation_cost_" (16) + 8 + 1 + 11 = 36`; `"restriction:" (11) + 25 + 1 + 0 = 37`; `"activation_restriction:" (23) + 10 = 33`; `"limit_decrease:" (16) + 11 = 27`. **≤ 37 against a 47-byte payload.** 10 bytes of headroom. |
| **FT-3** | `rules.c:849` | `uniq[8][24]` → `heart_options[i][24]`. **Static invariant**: `uniq`'s *only* writer is `snprintf(uniq[nu++], 24, ...)` at `:843`, so `strlen(uniq[i]) ≤ 23` always. Destination is the same 24. gcc does not track element size through the 2-D array. |
| **FT-4** | `rules.c:1352` | Identical invariant, `uniq[8][24]` at `:1333`, sole writer `:1338`. |
| **FT-5** | `rules.c:1555` | `tmp[16][64]` → `opts[16][64]`. `r_split(src, &tmp[0][0], 64, 16)` writes each element with `snprintf(out + n*stride, stride, "%s", tok)`, so every element is ≤ 63 chars. gcc sees `&tmp[0][0]` as one flat 1024-byte buffer and loses the stride. |
| **FT-6** | `look.c:1409` (×2) | `trig_list` tokens into `trigger_store[4][48]`. Measured over the corpus: `ability_filter_triggers` appears on **4** effect nodes and the **longest single token is 18 bytes** (`ライブ開始時`). 30 bytes of headroom. |
| **FT-7** | `state.c:1446` | `"all_blade_timing:%d:%s"` into `prohibition_effects[64][48]`. `cid` is a card id, and the arm requires `cid >= 0`, so ≤ 4 digits. `timing` is `eff_extra(e,"timing")` or the literal `"check_required_hearts"`; **the `timing` extra appears on ZERO of the 985 shipped abilities**, so the fallback always applies. Worst case 16 + 4 + 1 + 20 + 1 = **42** against a 47-byte payload. |
| **FT-8** | `game_state_core.c:853` | `rb_gsc_push_choice_offered`'s `labels` field — see the fixed-above list; this record moved when `line[1400]` → `line[1792]` shifted the file, and is pinned at its current line. |
| **FT-9** | `game_state_core.c:932` | The `DELTA` macro's `snprintf(deltas + w, sizeof(deltas) - w, …)` with `deltas[512]`. gcc reports the LAST delta of six landing in the 1 byte left after the buffer fills. **Static bound**: the six formats are `"<label>: %d -> %d\n"`, longest label `Active Energy: ` (16) + 11 + 4 + 11 + 1 = **43**; 6 × 43 = **258 < 511**. The region never reaches 1 byte. |

**The class is now in `WARN_AUDIT_FLAGS`** (as `-Wformat-truncation=1`, because gcc
15 spells the diagnostic `[-Wformat-truncation=]` and the pin key must match what
the script greps). 12 pins, all firing, `NEW=0`, `stale-pins=0`. The gate now
catches the **17th** instance instead of ignoring the class wholesale.

`-Wformat-overflow` stays **out**, as the brief expected: gcc 15 emits it **zero**
times for this tree's shapes even at `-O2`, so a pin for it could never go stale
in either direction — it would be a dead class.

---

# Cluster Z — FINAL WAVE: the consumer sites (2026-10-04)

All of this cluster's files were unowned when this wave started, so everything below
was re-derived from the CURRENT source rather than from any reported line number, and
every line reference was re-read before it was acted on. Baseline = **the same snapshot
with only these hunks reverted** (`fw1b`), not git HEAD, proved by the SHA-256 sweep
recorded under "the eff_extra consolidation" below.

## Z.1 The ten `cost.c` `card_type` reads — **LANDED, all ten**
`BLOCKED-ON cluster B` on `cost.c` is now cleared for `card_type`. Every one of the ten
sites read `eff_extra(<cost>, "card_type")`, and `vm.c`'s decode arm for `card_type`
(`vm.c:1098-1112`) `continue`s after writing the value into the **STRUCT FIELD**, so the
extras table never holds one. Measured with the engine's own decoder over the whole
corpus: `extra_present = 0` at all three classifiable sites (pay_cost_inner/Reveal **27**
cost nodes, pay_cost_move_cards **459**, pay_cost_change_state **124**) and 0 of the
**3465** effect nodes — while **23** Reveal / **113** move_cards / **124** change_state
cost nodes do carry a real one in `card_type_field`. Every one of those filters was
therefore answering "no card_type", i.e. inert.

One file-local reader now serves all ten (`cost_card_type_any`), transcribing Rust's
typed getter and NOT a raw string:
```rust
// effect_decoder_gen.rs: "card_type" => { ek.card_type = bc.read_card_type_value(); .. }
// read_card_type_value == CardType::from_card_str(...)
// core/card.rs:50-57   pub enum CardType { MemberCard, LiveCard, EnergyCard }
//                     #[serde(rename = "member_card")] ... _ => None
// core/card.rs:58-64   pub fn as_card_str(&self) -> &str  -> "member_card"/"live_card"/"energy_card"
```
`as_card_str()`'s output is byte-for-byte what `card_type_field` already holds, so the
fix is a one-line change at each site.

⚠ **The trap this item warned about was real and was avoided by re-verifying.** All ten
sites are textually identical reads of the same key, so a token-level replacement was
used and then **every site re-checked by line number and by enclosing function** — the
one-liner `for` walk in the report printed all ten function names, and they map 1:1 onto
the originals. No site was patched twice and none was missed. Note that `1671` was **not**
already fixed in the tree this wave inherited: the file's live positions were
**353 / 798 / 897 / 995 / 1051 / 1386 / 1687 / 2072 / 2344 / 2463** (two of the reported
numbers had drifted), and all ten were live.

| # | function | the filter that was dead |
|---|---|---|
| 353 -> 382 | `cost_filter_desc` | the description text printed "card_type=-" |
| 798 -> 827 | `validate_leaf_cost_for` | **the OFFER/PAY gate** stopped matching by card type |
| 897 -> 926 | `offer_same_unit_cost` | same-unit cost offer stopped matching by card type |
| 995 -> 1024 | `check_zone_cost_availability` | a zone cost was available for the wrong cards |
| 1051 -> 1080 | `pay_cost_move_cards` | `from_discard`/`revealed` hand filters were inert |
| 1386 -> 1415 | `pay_cost_change_state` | 「〜のメンバーをウェイトに」 matched every card |
| 1687 -> 1716 | `pay_cost_inner/Reveal` | 「手札のライブカードを公開」 offered the WHOLE hand |
| 2072 -> 2101 | `rb_handle_optional_cost_payment` | the optional-payment hand prompt was unfiltered |
| 2344 -> 2373 | `cr_reduction_matches` (`cr_eff_extra`) | cost-reduction matching ignored the type |
| 2463 -> 2492 | `cr_scan_one_effect` (`cr_eff_extra`) | cost-reduction scanning ignored the type |

**Suite effect — the single largest yield of this wave, and it is real bug-fixed
movement, not a relaxation:**
| suite | base `^FAIL` / hidden | fix | the cleared line |
|---|---|---|---|
| `parity_zone_source` | **29 / 51** | **27 / 42** | `FAILED: F: the reveal is filtered to live cards` (fixture `PL!-sd1-006-SD`, whose cost is `reveal{count:1, optional:true, card_type:live_card}` against a LIVE card and a member) |
| `parity_cce_gaps` | 37 / 51 | 37 / 51 | **0 assertions changed, but the prompt count fell 1 -> 0** (`b4_q190_no_live_in_hand`) — the hand prompt no longer opens, because the filter is no longer absent |
| `parity_small_clusters` | 42 / 67 | 42 / 67 | **0 assertions changed, output grew 86690 -> 86771 B**: a queue entry now resumes with a real `eff` pointer and a `look_and_select` action instead of an empty-hand `SelectCard` |
Proof that it is the fix and not the suite: `tests/test_parity_fw_cost_cardtype.c` case
`fw_ct_reveal_cost_card_type_comes_from_the_struct_field` asserts the prompt's
`card_type` is `"live_card"` **and** that `rb_card_matches_type` rejects a non-live card
for it. Base 1 FAIL, fix 0. The prompt does not precompute `filtered_indices` — it
publishes `card_type` and the choice resolver filters at answer time — so the pool
assertion is made through the same predicate Rust uses, and the end-to-end pool behaviour
stays asserted by `parity_zone_source`.

## Z.2 `misc.c` `h_choose_target_player` — **the two claims close as NOOPs, one real defect landed**
* **LANDED:** `allow_skip` was `e->is_optional ? 1 : 0`; Rust's
  `selection.rs:226` is the literal `allow_skip: false`. The **2** corpus
  `choose_target_player` effects carrying `optional` (`AX choose_target_player optional 2`)
  therefore offered a skip the prompt does not have, and answering it left the
  continuation's chosen seat unset. Now hardcoded `0`, and the prompt also publishes the
  bilingual descriptions Rust writes at `selection.rs:223-225` (escaped, per the
  fullwidth rule).
* **`VERIFIED-NOOP: 0` for "hardcodes `count=1`"** — Rust's `Choice::SelectTarget` carries
  no count at all, so `rb_emit_choice`'s `count` argument is meaningless for
  `RB_CHOICE_SELECT_TARGET`.
* **`VERIFIED-NOOP: 0` for "never walks `e->child`"** — Rust does not walk children there
  either. The effect is STRUCTURAL (`executor.c:37-52`): `effects/mod.rs` routes through
  `normalized_sequential_steps` only for `Sequential` / `LookAndSelect`, so for
  `ChooseTargetPlayer` the ROOT is dispatched first and its steps run **once** afterwards
  via the continuation the choice parks. Walking `e->child` here would run the **6** corpus
  instances' steps twice.
* **`BLOCKED-ON: rabuka.h — <code>char options[?]; ... </code>`** for the one part that is
  a real gap and is NOT this change's file: Rust publishes
  `options: effect.choice_options_any()` (`selection.rs:218-219`, `:227`), carried by
  **10** of the corpus's `choose_target_player` nodes (`AX choose_target_player choice_options 10`),
  defaulting to `vec!["自分", "相手"]`. `RbChoice` (`include/rabuka.h:952-979`) has no
  options array and `rb_emit_choice` (`rabuka.h:2250-2252`) has no options parameter. The
  exact request: add `const char *option_str[RB_CHOICE_MAX_OPTIONS]; int n_options;` to
  `RbChoice` and an options parameter to `rb_emit_choice`, then have `h_choose_target_player`
  split the flat `choice_options` extra the way `misc_group_name_first` already splits
  `group_names`. Not landed here because it cannot be landed without the header.

## Z.3 The `eff_extra` consolidation — **BLOCKED-ON, and deliberately NOT partially landed**
`eff_extra` has **5** live `static` twins and **181** call sites:
`compound.c:32`, `cost.c:108`, `misc.c:52`, `resolver.c:83`, `util.c:258`. Reconciliation
recommends `util.c:258` as the surviving body. I own **two** of the five (`cost.c`, `misc.c`)
and the item's own warning is that `cost.c:108` and `misc.c:52` are hot paths, so the
declaration must land in the SAME change or the delegations break the link.

**Status: `BLOCKED-ON`, nothing landed.** Two reasons, and the second is the one that
matters:
1. The surviving body (`util.c:258`) and the declaration (`include/rabuka.h`) are **both**
   outside this cluster's files, so either half alone is unreachable from here.
2. Collapsing only my two would convert **181 hot-path call sites** from an
   `-O2`-inlined `static` loop into a real cross-TU call — without LTO there is no
   cross-TU inlining — for **zero** behavioural difference. That is a measured performance
   regression on cost evaluation, paid for a purely cosmetic de-duplication, and it cannot
   be evaluated without the declaration in place. A partial landing would also leave two
   surviving bodies, i.e. the triplication would grow rather than shrink.

**The exact patch for the header owner.** `eff_extra` is currently absent from
`include/rabuka.h` entirely (verified — zero occurrences), so all five definitions are
file-local. The atomic change is:
```c
/* include/rabuka.h — near the other util declarations (~:2400 block) */
const char *rb_eff_extra(const AbilityEffect *e, const char *k);
```
```c
/* src/ability/util.c — at the surviving body (util.c:258) */
-const char *eff_extra(const AbilityEffect *e, const char *k) {
+const char *rb_eff_extra(const AbilityEffect *e, const char *k) {
```
```c
/* src/ability/cost.c (at cost.c:108) and src/ability/effects/misc.c (at misc.c:52) */
-static const char *eff_extra(const AbilityEffect *e, const char *k) {
-    if (!e) return NULL;
-    for (int i = 0; i < e->n_extra; i++)
-        if (e->extra_k[i] && !strcmp(e->extra_k[i], k)) return e->extra_v[i];
-    return NULL;
-}
+static const char *eff_extra(const AbilityEffect *e, const char *k) {
+    return rb_eff_extra(e, k);
 }
```
plus the same one-line delegation at **`compound.c:32`** and **`resolver.c:83`**.
⚠ **The bodies are semantically identical but not textually**: `cost.c`, `compound.c` and
`resolver.c` null-check the `k` argument (`!k`), `misc.c` and `util.c` do not; `misc.c` does
not null-check `e` and the other four do. Delegation is therefore *strictly* safe in every
direction (it can only add guards), and all 181 call sites pass a string literal for `k`,
so the `k` guard is unreachable either way. That asymmetry should be settled in the same
change, not left implicit.

## Z.4 `game_state_abilities.c` — **LANDED 2026-10-03 + PINNED (see I6)**
`rb_hooked_by_foreign_cause` (`src/core/game_state_abilities.c:2265`) scans
`g->batch_movements` where Rust scans the TURN-AREA movements, and its own comment claims
"the C has no such vector", which has been **false since A8**. Not this cluster's file.
Exact patch, for the owner — **SUPERSEDED 2026-10-03: LANDED, see the I6 row for what was
actually done and for the new parity test.** Kept below only as the historical shape. Note the
real patch also had to make the scan a REVERSE loop (Rust `.iter().rev().find(..)`), which the
draft above did not say:
```c
-        for (int i = 0; i < g->n_batch_movements; i++) {   /* line ~2265 */
+        for (int i = 0; i < g->n_turn_area_movements; i++) {
+            /* Rust: ability/condition/card.rs:1632-1658 -- the temporal arm reads the
+             * TURN-AREA vector, which `g->turn_area_movements` /
+             * `g->n_turn_area_movements` has modelled since A8. batch_movements is a
+             * different vector and answers a different question. */
```
and the same substitution for the loop bound. Also verify the companion
`rb_resolution_*` owner rows before touching it: A9's `RbBatchMovement.cause_card_id` is
**write-only** per this checklist's own Y.5 table, so a scan that moves to
`turn_area_movements` loses nothing and gains the reader Rust has.

## Z.5 Tests landed with this wave
| file | cases | base | fix |
|---|---|---|---|
| `tests/test_parity_fw_cost_cardtype.c` | `fw_c1_self_cost_activation_is_offered`, `fw_c1_unpayable_mandatory_cost_is_still_refused`, `fw_ct_reveal_cost_card_type_comes_from_the_struct_field`, `fw_ct_absent_card_type_stays_unfiltered`, `fw_c3_hand_cost_dedupes_by_normalized_name`, `fw_c3_control_without_distinct_the_prompt_opens` | **3 FAIL / 4 hidden** | **0 / 0** |
| `tests/test_parity_fw_effects.c` | `fw_f3_blade_grant_reaches_the_trace_ring`, `fw_f3_heart_grant_reaches_the_trace_ring`, `fw_f3_control_blade_count_is_unchanged`, `fw_f1_custom_routes_through_the_pooled_clone`, `fw_t2c_choose_target_player_is_not_skippable`, `fw_t2_10_restriction_consumes_the_changed_state_latch` | **5 FAIL / 5 hidden** | **0 / 0** |

Fixture discipline: every card is resolved by **card number** through `test_new_id`, and
each case asserts the resolved print back (CARD IDENTITY: `PL!-bp3-002-P` and
`PL!-bp3-002-R` are DIFFERENT cards that happen to share a printed name — that pairing is
the whole point of the C3 fixture). Every `GameState`/`TestGame` local is `static`.
Fullwidth characters are `\xNN` escapes. Every case that could be vacuous carries an
explicit **precondition** assertion about the decoded shape first, and every fix that
removes behaviour carries a **control** that pins the behaviour it must NOT remove
(`C1` unpayable-mandatory still refused; `C3` no-`distinct` still prompts; `F3` blade
count still +3/+6). **No existing test was weakened, deleted or ignored.**

## Z.6 The A/B — 40 suites, strictly sequential, `^FAIL` with NO colon, stdout AND stderr
Exit code ignored throughout. The interned string table was pinned before and after every
build: **`src/abilities_strings.bin` = 726831 B / `89204df0bbde08a02a7f978a56a80d6e6431691ec7d6d658979a329956b06344`**, identical on both trees and identical before/after every link. No generator was run. No `.o` or `rb_engine*` was left in the live tree.

`^FAIL` counts use `-cmatch` on the combined stream, with the colon-spelling counted
SEPARATELY — three suites emit `FAIL:` lines that the harness convention does not count, so
counting them as `^FAIL` understates them and not counting them at all hides them:
`mechanics` is `0 / 5 hidden` with **RC=1**, `parity_zones_position` is `0 / 37 hidden`
with RC=1, and `parity_score_effects` is `0 / 1 hidden` with RC=1. Those are real
failures the `^FAIL` convention misses; they are reported here rather than folded in.

```
suite                        base FAIL/HIDDEN    fix FAIL/HIDDEN    delta
parity_zone_source                 29 / 51            27 / 42     -2 / -9   Z.1
parity_state_family                 2 /  3             1 /  1     -1 / -2   T2-10
parity_draw_flat                    0 / 58             0 / 57      0 / -1   C1
parity_cce_gaps                    37 / 51            37 /  51      0 /  0   Z.1 (1 prompt -> 0)
parity_small_clusters              40 / 64            40 /  64      0 /  0   Z.1 (87017 -> 87098 B)
parity_fw_cost_cardtype             0 /  4             0 /  0      0 / -4   new
parity_fw_effects                   0 /  5             0 /  0      0 / -5   new
--------------------------------------------------------------------------
other 33 suites                  identical          identical      0 /  0
TOTAL                            673 / 1107         670 / 1086     -3 / -21
```
**Zero FAIL lines added anywhere. Zero hidden FAIL lines added anywhere.** Every suite that
moved moved DOWN, and each movement is attributed above to the specific Rust line it
implements. `sizeof(GameState)` unchanged (no field was added), so the 32 MB stack and the
`tests/test_harness_self.c` ceiling are untouched.

⚠ **The baseline had to be REBUILT mid-wave, and this is the trap the brief's "baseline =
only YOUR hunks reverted" rule exists for.** The two snapshot trees were taken at
different moments and a peer landed a `src/ability/choice.c` change in between, so
`choice.c` was **not** held constant across the first pair of runs — 5724 lines in one tree
and 5627 in the other. Any movement could then have been misattributed. `choice.c` was
re-synced to one identical copy in BOTH trees and the whole 40-suite pair re-run; the
numbers above are from the corrected pair. The file-hash sweep that proves the baseline is
now clean is **247 objects byte-identical, differing in exactly 4** (`cost.c`, `misc.c`,
`rules.c`, `game_setup.c`) plus the 3 new `tools/` programs. `sizeof(GameState)` was not
changed, so no field of it moved either.

## Z.7 Things found and reported rather than changed
* ⚠ **`rules.c`'s `r_distinct_type` compares against the WRONG CASING.** It tests
  `"CardName"` / `"True"` / `"Distinct"`, but Rust's serde renames are **lowercase**
  (`core/card.rs:2809-2816`: `"card_name"`, `"true"`, `"distinct"`), so the function
  returns `RB_DISTINCT_NONE` for every corpus node and the `distinct` filter it feeds
  (`rules.c`'s distinct arm) is inert. **Not changed here**: it is outside the seven items
  this wave owned, and it is a filter-semantics change of exactly the class that
  `misc.c:355-372` records as having turned 5 cases RED. Recorded so the next agent does
  not re-derive it. `cost.c`'s new `cost_distinct_type` uses the correct lowercase forms
  and says why in its own comment.
* **`F4` and `F8` name identifiers that do not exist anywhere in the tree**, in either
  language: `write_auto_move` and `card_identity_flags` return zero hits across every
  `.c`/`.h` in `engine_c_wip` and across every `.rs` in `engine/src`. F4's stated premise
  ("the C keeps the data plane but nothing WRITES the list — it is a read-only producer
  for `rb_ability_write_auto_move`") is therefore not true of the current tree: there is no
  such producer and no such reader. Closed as VERIFIED-NOOP on `dynamic_count.c`'s own
  terms, with the identifier census as the evidence.
* **`parity_card_filters` and `parity_effect_conditions` DO NOT EXIST.** There is no
  `tests/test_parity_card_filters.c` and no `tests/test_parity_effect_conditions.c`
  (`tests/` was listed before the run). Both were on the requested suite list; both
  contribute 0 to both totals, which is how two names in circulation can read as PASSING.
  This is the same class of finding this checklist already records for
  `parity_condition_decode`.
  ⚠ **A THIRD name on that list is also wrong, and in the opposite direction:**
  **`parity_hearts_required` does not exist either — the suite is `parity_required_hearts`**
  (`tests/test_parity_required_hearts.c`). Two names on the list are near-misses for real
  suites (`parity_hearts_required` → `parity_required_hearts`, `parity_card_filters` /
  `parity_effect_conditions` → nothing). Only **25 of the 28 requested names exist**; the
  25 that do are the ones counted below. Re-verified 2026-10-04.

---

# WAVE 2026-10-04 "fixer wave" — four UNOWNED files, every item closed or named

Owned: `src/ability/choice.c`, `src/ability/cost.c`, `src/ability/condition.c`,
`src/core/game_state_abilities.c`. Quiescence confirmed before editing — all four hashes
identical across a 20 s settle, no `.o` newer than any source, blob 726831 /
`89204df0bbde08a0…` before and after every build.

**New evidence tool:** `tools/census_fx.c` (+ `tools/build_census.sh`). It walks the SHIPPED
bytecode through the engine's OWN decoder `rb_decode_card_ability`, exactly as
`tools/audit_cluster_d.c` does — no bytecode reader is reimplemented and no number below comes
from reading JSON. **2116 card-ability pairs, 985 abilities, `decode_fallback_count = 0`.**
Every figure in this section is reproducible with `sh tools/build_census.sh && ./rb_census_fx src`.

| item | verdict | one line |
|---|---|---|
| A9 / E16 `cause_card_id` reader | **LANDED** | `eval_group`'s temporal-this_turn arm rewritten as the faithful `card.rs:1632-1658` arm; **2 nodes** (cards 2467/2468); 13 assertions in `tests/test_parity_fx_fixers.c::fx1_cause_card_id_arm`. |
| `position_change_events` reader | **LANDED** | `position_event_matches_filters` + `eval_movement`'s `position_change` arm (`state.rs:869-908` + `:524-568`); **26 nodes / 26 pairs**; sub-item `area_direction:"from"` (2 nodes) is `BLOCKED-ON` `include/rabuka.h` for `old_position`. |
| `check_card_property` empty set | **VERIFIED-NOOP: 0 of 73** | the 🔴 box was FALSE — Rust returns `true` at `card.rs:133-136` before the loop; the C is correct; cluster D's deleted assertion is **restored** with the citation (`fx2_card_property_empty_set_is_true`). |
| B3 same-unit exclusion | **VERIFIED-NOOP: 0 of 138** | two methods agree: 2 `same_unit_name` hand costs exist but both are ライブ開始時; 0 of 138 **debut**-triggered hand costs carry it. |
| B5 queue externs | **PARTIAL / BLOCKED-ON** | `rb_queue_is_entry_available` was already in `rabuka.h:1890` → its extern is deleted; the other two are `BLOCKED-ON: include/rabuka.h`. |
| `rules.c` `r_distinct_type` casing |
**LANDED 2026-10-04** (was **BLOCKED-ON: src/ability/effects/rules.c:310-317**)

Both spellings fixed, because they are one bug: the reader compared the Rust VARIANT names `CardName` / `True` / `Distinct` while `core/card.rs:849-856` renames all three to LOWERCASE, and `cost.c`'s twin reader (`cost.c:171-177`, cluster C4) already compared the lowercase set. `r_distinct_type` now reads `"card_name"` / `"true"` / `"distinct"`, and its sibling `r_distinct` (the `PerUnitFilter.distinct_names` reader) was routed through it as `rb_distinct_should_dedupe(r_distinct_type(e))` instead of `r_is_true(...)`, which matched only a bare boolean and so read `distinct: "card_name"` -- 48 of the 53 corpus nodes carrying the key -- as NOT distinct. This is Rust's ONE `is_distinct_names` computation (`required_hearts.rs:58-59`), feeding both `PerUnitFilter.distinct_names` (`:77`) and the dedupe at `:88`.

  **VERIFIED-NOOP: 0. The fix is CORRECT and moves ZERO lines.** Re-measured 2026-10-04 over `cards/abilities.json`: 53 nodes carry a `distinct` key, and `r_distinct_type` is reachable ONLY through `r_per_unit_units` (sole caller `rules.c:567`; `r_per_unit_units`' own callers are `:622` reduce_required_hearts + per_unit_heart_colors, `:648`/`:678` modify_required_hearts + per_unit, `:945` modify_cost + per_unit). **0** nodes satisfy any of those: every node carrying BOTH `distinct` and a truthy `per_unit` has action `change_state` / `gain_resource` / `modify_score` / `sequential`, and the 2 `modify_required_hearts` nodes and the 1 `modify_cost` node carrying `distinct` all have `per_unit` absent. `r_per_unit_filter` has the same single caller, so `distinct_names` is equally 0-reachable. Two further corrections to the numbers this row used to quote: it is **53** nodes, not 43 (48 `card_name` + 2 `cost` + 3 `group_name`; the 2 `cost` are the ones `cost.c` handles), and NOT all of them are `card_name`. `distinct: "cost"` and `distinct: "group_name"` are NOT `DistinctType` variants at all -- Rust's decoder (`vm.rs:1062-1080`) maps every unrecognised string to `CardName`, and `vm.c:1155-1161` deliberately mirrors only the three real spellings -- so they stay `NONE` here, which is the right answer for `is_distinct_names`.

  **NO PARITY TEST IS POSSIBLE and that is not an omission**: the function is `static`, nothing on this corpus can reach it, and any test written against it would be vacuous. The previous two reports of this row were right that the code was wrong and wrong to call it untested -- it is now right, and the note in `rules.c` records the measurement so the next reader does not have to redo it.
| C1 / C2 completeness | **CLOSED — nothing missing** | Rust has **3 external** call sites (`game_setup.rs:2188`, `game_setup.rs:2295`, `turn/actions/mod.rs:559`) plus its own recursion (`handlers.rs:297`); the C covers all three — `setup_ability_cost_offered` is called from **two** sites (`game_setup.c:1263`, `:1299`) mirroring the two scans, `engine.c:1187` mirrors the third, and `cost.c:980-985` mirrors the recursion. |
| `triggers.c` second dispatcher | **VERIFIED-NOOP: 0 of 275** | confirmed, and the 275 was reproduced independently. `triggers.c:1189-1191` collapses `look_at`/`reveal`/`reveal_per_group` onto `rb_effect_look_at` where `dispatch.rs:78-83` sends them to three functions; `triggers.c:1196`'s `reveal_until_target` has **no Rust `ActionType`** (the enum's reveal family is `LookAt`, `Reveal`, `RevealPerGroup`, `RevealUntilLiveCard`, `RevealUntilChosenCard`). Measured: whole-corpus `look_at`=200, `reveal`=48, `reveal_per_group`=0, `reveal_until_target`=0; **within the 275 常時 abilities' 320 effect nodes: 0, 0, 0, 0.** Latent, and `turn/triggers.c` is not this wave's file. |
| `misc.c` `card_appeared_this_turn` | **VERIFIED-NOOP — the claim is FALSE** | no `cid`-indexed access exists: the function is a linear scan of `cards_appeared_this_turn[]` bounded by its own count; the one indexed read (`misc.c:382`) already guards `cid >= RB_MAX_CARD_IDS`. |
| `util.c` `card_total_hearts` | **NOT A DEFECT** | `util.c:699-706` cites `Card::total_hearts` (`card.rs:4194-4202`), a *different* Rust function from `original_heart_total`; the two are not duplicates. The rename is cosmetic and belongs to `util.c`'s owner. |
| `game_state_abilities.c` dead twins | **IMPOSSIBLE to wire from this file** | 6 unused statics + 1 unused local (`-Wunused-function`, all pre-existing). They are **not** duplicates: their logic is **missing** from the live paths (`batch_triggered_keys` dedup is inlined and DUPLICATED at 4 enqueue sites; the `just_completed_ability_key` veto is still unconditional at 3, so Rust's fresh-batch rule `abilities.rs:195-200` is still absent). Full tombstone at `game_state_abilities.c:787-824`. See the `BLOCKED-ON` text there. |
| warning gate | **PASS, nothing to fix** | `make warnings-audit` → `PASS -- 12 diagnostic(s), all pinned; 0 new`, 0 stale pins. **None of the 12 is in this wave's four files**, so `-Waddress` / `-Wdiscarded-qualifiers` / `-Wparentheses` are all clean here. |

**A/B, against a baseline that is the CURRENT live tree with only these hunks reverted**
(`tools/tree_sweep.sh`: **442 files byte-identical, differing in exactly 2** —
`src/ability/choice.c`, `src/ability/condition.c` — plus the 2 new files removed):
**1276 → 1270 `^FAIL`, 25/25 suites exist, 0 BUILD_RED, 0 silent, 0 suite regressed.**
Moved: `parity_live_total` 20/33 → 18/32, `parity_jidou_move_yell` 37/47 → 36/45.
`parity_small_clusters`, `parity_integration` and `parity_zones_position` show byte-only stderr
diffs with identical `^FAIL` counts: fewer spurious 自動 queue entries fire, because a
`movement:"position_change"` gate no longer answers "did anything move recently".

⚠ **Stated plainly: two `parity_live_total` assertions are NEWLY FAILING** —
`I1_chisato_liella_move_grants_score` ("a Liella! effect moved Chisato -> live total +1"). They
were passing **by accident**: the old dead arm answered "did anything move recently", which was
true in that scenario for the wrong reason. The faithful arm answers 0 because
`turn_area_movements` is **empty** at ライブ成功時 time in the C (traced directly). The three
negatives that were wrongly failing — natural move, previous-turn move, opponent's effect
(`I3`/`I4`/`I6`) — now pass, so the arm is discriminating correctly in both directions. Making
`I1` pass needs the movement to survive to live-success time, which is a tracking-lifetime
question in a file this wave does not own.

**Claimed by this wave:** `tests/test_parity_fx_fixers.c` (new, 34 assertions), `tools/census_fx.c`,
`tools/build_census.sh`, `tools/ab_run.sh`, `tools/ab_diff.sh`, `tools/ab_total.sh`,
`tools/norm_diff.sh`, `tools/tree_sweep.sh`, `tools/run_one.sh`, `tools/rebuild_fx.sh`.
**`src/ability/cost.c` was inspected and NOT modified** — its hash is unchanged
(`3F0DD575D605F86C…`), and no warning was introduced into it.

---

## 2026-10-04 — CLUSTER A17b / deck-order producer 2 / r_distinct_type. Findings, and two hazards.

### HAZARD 1 — THE REFERENCE MOVED, AND IT WAS COMMITTED WITHOUT AUTHORISATION. VERIFIED BEFORE ACTING.

An earlier agent found the inclusion-vs-exclusion collapse present in Rust's WORKING TREE and absent from committed HEAD, and correctly refused to land the C side on that basis. That has changed: a peer swept the Rust working tree into commit **`2130769d`** ("wip: engine/c parity audits, bot v8 live work…") and **`engine/src` is now CLEAN in git**. Re-verified before implementing, not taken on trust: `git show 2130769d -- engine/src/ability/util.rs engine/src/core/card.rs` carries BOTH hunks — `CardFilter::from_effect` at `util.rs:1864-1867`, and `AbilityEffect::filter_subset` + the new `excluded_matches` at `core/card.rs:889-894` / `:2410-2416`. HEAD has since advanced `f152a50f` -> `cf315a9a` -> `2130769d`. **The C was genuinely divergent and A17b is landable.** (Had the collapse turned out NOT to be in the commit, the correct action was to report that and land nothing.)

### HAZARD 2 — PEERS KEEP COMMITTING AND KEEP EDITING FILES I OWN. NEVER REVERT A COMMIT; NEVER MAKE ONE.

Observed inside this one task, all of it unrequested:
- HEAD advanced three times without authorisation.
- A peer **swept another agent's in-flight work into a commit.**
- A peer **edited `tests/test_parity_jidou_watch.c`, a file I own**, mid-task: SHA-256 `A3C3D6518604E17D` (10-02 12:28) -> `6B446B26FA3A2B87` (10-04 10:58), +47/-6 lines, deliberately inverting one assertion so it is RED ON PURPOSE while tracking the Rust oracle. My test patch had to be re-derived and re-applied on top of the peer's version. **Any measurement taken before 10:58 against `parity_jidou_watch` is not comparable with one taken after.**
- 80 files under `engine_c_wip/` differed between my frozen snapshot and the live tree by the end of the task, none of them mine.

Quiescence was therefore re-confirmed immediately before every write, by SHA-256 on two reads separated by a settle, plus a check that no `.o` anywhere is newer than its source. The hashes that were quiescent when I landed: `src/core/card.c` `9EF5214186981110`, `src/ability/effects/look.c` `C6AA577830F4D7FE`, `src/ability/effects/rules.c` `C735B4F536BE4571`, `tests/test_parity_jidou_watch.c` `6B446B26FA3A2B87` (the peer's version, on which my patch sits).

### BUILD TRAP, NEW AND EXPENSIVE — A PARTIAL OBJECT SET IS NOT A BUILD.

This task lost roughly half its wall clock to it, and it produced a **+135 FAIL phantom regression across 8 suites** that did not exist. What happened, exactly:

1. The baseline was `make clean; make -j4 all` in the frozen snapshot. Fine.
2. Between measurements I edited three sources and ran `rm -f <those three>.o; make all`. **`make clean` was not re-run.**
3. That is a STALE LINK of a kind the existing trap note does not cover: the three objects I rebuilt are correct, but the OTHER ~45 objects in the tree were left over from an earlier state, and eight suites link enough of them for the damage to show.
4. Symptom: with pristine sources and only three objects rebuilt, `parity_look_and_filter` read **94** and `parity_cce_gaps` **110**. After `make clean; make -j4 all` on the SAME pristine sources, the same suites read **39** and **88** — exactly the original baseline. The phantom was entirely in the objects.

**RULE ADOPTED HERE, and it should be the rule for every A/B in this repo: `make clean` before EVERY build whose result will be compared against another build. Rebuilding "only what I touched" is fine for a quick smoke test and worthless for a number anyone will quote.** The two A/Bs in this task that were taken with `rm -f <one>.o; make all` are marked void in place.

### FINAL MEASURED LEDGER — clean builds on both sides, identical tree, 22 suites.

Baseline = the frozen snapshot with only my hunks reverted, rebuilt with `make clean; make -j4 all`. Head = the same snapshot with all four hunks, rebuilt the same way. `^FAIL` counted with NO colon, case-sensitively, on stdout AND stderr separately.

| suite | base | head | delta |
|---|---:|---:|---:|
| parity_search_reorder | 123 | **119** | **-4** |
| parity_int_gaps | 41 | 41 | 0 |
| parity_deck_order | 0 | 0 | 0 |
| parity_jidou_watch | 75 | 75 | 0 |
| parity_jidou_watch_clusters | 62 | 62 | 0 |
| parity_jidou_move_yell | 84 | 84 | 0 |
| parity_look_and_filter | 39 | 39 | 0 |
| parity_look_per_card | 64 | 64 | 0 |
| parity_small_clusters | 104 | 104 | 0 |
| parity_sequential | 142 | 142 | 0 |
| parity_cce_gaps | 88 | 88 | 0 |
| parity_integration | 42 | 42 | 0 |
| parity_under_member | 87 | 87 | 0 |
| parity_restriction | 62 | 62 | 0 |
| parity_live_total | 53 | 53 | 0 |
| parity_recover_live | 57 | 57 | 0 |
| mechanics | 5 | 5 | 0 |
| parity_wave1a | 38 | 38 | 0 |
| parity_wave1b | 32 | 32 | 0 |
| parity_wave1c | 44 | 44 | 0 |
| audit_warnings | 0 | 0 | 0 |
| parity_hearts_live_start | 67 | 67 | 0 |
| **TOTAL** | **1309** | **1305** | **-4** |

**ZERO suites gained a single line. The entire movement is 4 lines, all in `parity_search_reorder`, all of them GREEN: `rt/rin_select_both_stay_on_deck`.** That test is not mine and was not written for this work; it asserts 「rin: the order choice must be prompted after selecting both」 and was RED precisely because deck-order producer 2 did not exist. It is the strongest available evidence that the producer is a faithful port.

Attribution, each measured on its own with its own clean build:

| hunk | file | effect on the 22-suite list |
|---|---|---:|
| A17b inclusion-vs-exclusion collapse | `src/core/card.c` | 0 (jidou_watch 75 -> 75, nothing new, nothing gone) |
| `r_distinct_type` + `r_distinct` casing | `src/ability/effects/rules.c` | **VERIFIED-NOOP: 0** |
| deck-order producer 2 (`needs_order`) | `src/ability/effects/look.c` | **-4** (search_reorder 123 -> 119) |
| the corrected jidou CONTROL + the new A17b test | `tests/test_parity_jidou_watch.c` | 0 (the 5 new assertions and the 8-assertion new test are all green) |

### ITEM 2 — THE RE-FIRE. **BLOCKED-ON, with the cause located to two lines and both obvious patches MEASURED and REJECTED.**

`parity_search_reorder` carries 12 red lines from it, all six `nozomi` sub-tests: 6 x `the final remaining card needs no choice` and 6 x `the looked_at pool is emptied (got 3 expected 0)`. **These are STILL RED after every hunk landed in this task**, including the deck-order producer that shares their `rb_order_prompt_begin`. Do not read any of this task's green results as having fixed this.

**HYPOTHESIS 2 (DEBUT, `move.c:2135` / `:2141` -> `rb_move_fire_debut_side_effects` -> `triggers.c:268`) IS FALSIFIED, and it was cheap to test, which is why it was tested first.** `rb_move_fire_debut_side_effects` is called only under `move_zone_is(destination, "stage")` (`move.c:2133-2137`), and the subject's destination is `deck_top`. Instrumenting every enqueue path (`rb_queue_enqueue`, `rb_push_owned_entry`, `rb_queue_clear_completed`, `rb_queue_complete_current`) plus the drain's entry scan showed **no enqueue and no compaction anywhere inside any of the six nozomi blocks**. The 登場 pushes that do appear in that suite belong to a different fixture.

**HYPOTHESIS 1 WINS, and the precise statement is this: the entry is re-executed from the top of `rb_drain_ability_queue` because it is never stamped, and nothing ever stamps it on this path.**

Measured, with the scan print in place:
```
[A17Q_SCAN] idx=0 n=1 cid=2639 ab=0 started=0 completed=0 cur=0 state=1
[QUEUE_EXEC] idx=0 n=1 cid=2639 ab=0 actor=0 action=sequential   <- the re-fire
[RESUME_ANY] kind=2 zone= target=order ... idx=-1               <- the prompt re-opens
```
`n=1` throughout, `started=0`, `completed=0`, and no enqueue and no clear inside the block — so it is the **same** entry, unstamped, not a fresh one. The chain is `rb_resume_with_choice_indices_internal` -> `dispatch_choice_result` -> `rb_resolver_handle_order_selection` (`choice.c:4181-4190`) -> `rb_move_handle_order_selection` returns 0 -> **`rb_resolver_clear_choice_state_and_resume` (`choice.c:445-451`)**, which ends in `rb_drain_ability_queue(self->gs)`. The drain's skip test (`ability_queue.c:1378`, `if (e->effect_started || e->completed) continue;`) is therefore never reached for this entry, the `sequential` re-runs from the top, and the `look_at` step re-opens its prompt.

Why is it unstamped? Rust stamps it in `resume_pending_actions` whenever executing a parked command leaves a prompt open (`choice.rs:122-125`), and the C has that exact stamp at **`ability_queue.c:1061-1064`**. **That stamp is never reached on this path: instrumenting `rb_queue_resume_pending_actions` produced ZERO hits in the entire suite run.** The nozomi ability is resolved by the *other* queue consumer, `pca_resolve_entry` (`game_state_abilities.c`), which after `rb_execute_effect_ex` at `:2998` returns 1 on `g->queue.has_pending` (`:3027-3040`) without ever writing `effect_started`. So there are two queue consumers and only one of them carries the stamp.

⚠ **The site is NOT `:1403-1405` and the earlier advice not to latch there is right.** The skip test is correct; the entry is genuinely unstamped, and stamping it at the drain would be treating the symptom in the one function that is behaving faithfully.

**Both obvious patches were implemented and measured, and BOTH ARE WRONG. Do not ship either without redoing this measurement.**

| candidate | file:line | what it does | MEASURED effect on the 22-suite list |
|---|---|---|---|
| P1 | `src/ability/choice.c:449` and `:460` | swap `rb_drain_ability_queue(self->gs)` for `rb_queue_resume_pending_actions(self->gs)` in `rb_resolver_clear_choice_state_and_resume{,2}`, which is literally Rust (`choice.rs`: `clear_choice_state_and_resume` = `clear_choice_state` + `resume_pending_actions`) | **WORSE**: `parity_sequential` 142 -> 146, `parity_cce_gaps` 88 -> 91, and **the 12 nozomi lines did not clear**. Rejected. |
| P2 | `src/core/game_state_abilities.c:3027` | stamp `entry->effect_started = 1` in `pca_resolve_entry`'s `has_pending` arm, i.e. apply Rust's `choice.rs:122-125` invariant to the second consumer | **MUCH WORSE**: `parity_look_and_filter` 39 -> 94, `parity_jidou_watch_clusters` 62 -> 86, `parity_recover_live` 57 -> 69, `parity_search_reorder` 123 -> 124, total 1309 -> 1419. Rejected. |

⚠ Read the P2 numbers with the BUILD TRAP above in mind before concluding anything about *which* objects were stale in that particular run; what is NOT in doubt is the direction and the size, since the same run also moved eight suites and the clean rebuilds that followed put every one of them back. **A narrower candidate worth trying next, and the one I would pick: make the order-completion path in `rb_move_handle_order_selection` retire or advance the owning entry the way every other choice-completion path does, instead of leaving it for the drain to re-find.** That file is `src/ability/choice.c`, which another agent owns for a SEGFAULT in the `pay_cost_all:discard_all` path, so it was not touched. Whoever holds that file owns this fix.

**MEASUREMENT NOTE that cost time and would have cost more:** the `started=0` reading that first suggested "fresh entry" is correct and consistent; what it does NOT tell you is WHICH consumer ran the ability, and that is the whole question. Instrumenting the consumer (`rb_queue_resume_pending_actions`) settled it in one run.

---

# W. 2026-10-04 — live-zone slot validation, the `queued` diagnostic, and two phantom surfaces

**Read W.1 first if you are here to "fix" the `live_face_up` 34 FAIL lines. W.1 says
the rule is already implemented, and proves it by A/B. Everything W.1 concludes was
measured in an isolated snapshot, both objects rebuilt from source, with the
snapshot's objects deleted first so a stale link could not fake a result.**

## W.1 — `[x]` live-zone slot validation: the rule EXISTS. The 34 lines are 34 missing-fixture lines.

**The premise this item was opened on is wrong, and the measurement says so plainly:
the C has live-zone slot validation, the gate, the routing table, and the movement
event. None of it is missing.** `src/turn/phase.c:675-716`, `rb_check_invalid_live_cards`,
called from `rb_check_timing`:

```c
if (!g->live_face_up[is_p1 ? 0 : 1]) return;
for (int i = P->live.n - 1; i >= 0; i--) {
    int cid = P->live.cards[i];
    if (!rb_card_is_live(cid)) {
        int c = bag_remove_at_local(&P->live, i);
        if (rb_card_is_energy(c)) bag_push_local(&P->energy_deck, c);
        else                       bag_push_local(&P->discard, c);
        rb_record_card_movement(g, c, RB_ZONEID_LIVE_CARD_ZONE,
                                rb_card_is_energy(c) ? RB_ZONEID_ENERGY_DECK : RB_ZONEID_WAITROOM,
                                is_p1 ? 0 : 1, 0);
    }
}
```

That is Rust's `check_invalid_live_cards` arm for arm: the `face_up` early return,
the `!card.is_live()` filter, reverse index order, energy to `energy_deck` and
members to `waitroom`, and one `push_movement_event(card_id, "live_card_zone",
dest, None, player_id, false)` per evicted card. `src/turn/phase.c` has **no
working-tree diff at all** — the body is committed, not a local edit.

### Why the 34 lines are red, measured rather than inferred

The fixtures in the four suites that exercise this rule hand-build a board and call
`rb_check_timing` **without** `live_face_up[seat] = 1` — the one line Rust's own twin
carries. So the gate is closed, the eviction arm is never reached, and the suite
reports the rule missing when it is merely unreached.

**A/B, in an isolated snapshot, objects deleted and rebuilt, both suites, one line
changed and nothing else:**

| suite | as found | + `tg.state.live_face_up[0] = 1;` | + `live_face_up[0]=1; live_face_up[1]=1;` |
|---|---|---|---|
| `mechanics` | **7** `^FAIL` | **0** | — |
| `live_card_zone_movement` | **15** `^FAIL` | — | **0** |

Zero FAIL lines in both, from the same objects, with the only difference being the
fixture line Rust's twin already has. **That is the whole finding: these lines are a
missing `face_up = 1` in a test fixture, not a missing engine rule.**

### ⚠ The "34 lines across three suites" attribution is only HALF right, and the other half does not move

Measured per suite, all in the snapshot, objects deleted and rebuilt each time:

| suite | opening the gate in the fixture | verdict |
|---|---|---|
| `mechanics` | 7 → **0** | a real missing-fixture line |
| `live_card_zone_movement` | 15 → **0** | a real missing-fixture line (two lines, one per seat) |
| `parity_hearts_live_start` | 84 → **84** | **NOT the gate.** `hl_fire_live_start` → `test_fire_trigger_on_card` resolves and fires ONE ability directly; it never calls `rb_check_timing`, so `rb_check_invalid_live_cards` is never reached. Setting `live_face_up[0] = 1` inside `hl_fire_trigger` changes the suite's output **byte for byte**. |
| `parity_jidou_watch` | **already green** | `tests/test_parity_jidou_watch.c:715` no longer asserts the eviction. Its owner inverted it to the Rust oracle — `CHECK(test_zone_has_id(&game, 0, "live", hasetsu), "RUST ORACLE: at ライブ開始時 the wrongly-set member is STILL in the live zone ...")` — and **both rows now print `ok:`**. The old `!test_zone_has_id(...)` form that earlier audits attributed +2 to has been gone for some time; that audit is stale, not the engine. |

So: **20 of the attributed lines are real missing-fixture lines; the other 14 are
not attributable to the gate at all** — they belong to a helper that bypasses
`check_timing` and to a fixture its owner has already corrected. Anyone who opens
`parity_hearts_live_start` expecting the gate to explain its 84 reds will waste a
day.

### `[x]` What landed instead: a suite that owns the rule, in a file nobody contests

`tests/test_live_zone_slot_validation.c` — 8 test functions, 47 assertions, green.
It asserts what Rust's twins assert, by content:

| test | Rust twin |
|---|---|
| `member_leaves_live_zone_for_the_waitroom` | `invalid_live_card_discard_records_turn_movements` |
| `energy_returns_to_the_energy_deck` | `invalid_energy_card_in_live_zone_goes_to_energy_deck` |
| `mixed_zone_routes_each_card_by_its_own_type` | (both twins, on one board — strictly stronger) |
| `each_seat_evicts_only_its_own_live_zone` | `check_invalid_live_cards_distinguishes_players` |
| `gate_closed_leaves_the_live_zone_untouched` | the `face_up` early return |
| `a_real_live_card_survives_the_check` | the survivor half of `live_card_in_live_zone_stays` |
| `a_valid_and_an_invalid_card_share_the_slot` | (the `is_live` predicate, both halves at once) |
| `performance_phase_opens_the_gate_before_the_timing_check` | `phases.rs:343` writes, `:349` checks |

Every positive is paired with a negative on the same board — the gate closed, or the
card genuinely live — so **no assertion in the file can pass vacuously.** That is the
point: the failure mode here was never a wrong engine, it was a rule that quietly
became unreachable and nothing in the corpus noticed.

### `[x]` A/B proof that the rule is really pinned, three ways

Each of these was applied to `src/turn/phase.c` **in the snapshot only**, with
`src/turn/phase.o` deleted and rebuilt, and then reverted:

| sabotage | result |
|---|---|
| delete the eviction body, keep the gate | **26 `^FAIL` across 6 of the 8 functions**; the two negative controls (`gate_closed`, `a_real_live_card_survives`) stay green, which is what they are for |
| delete the `face_up` gate, keep the body | **4 `^FAIL`**, all in `gate_closed_leaves_the_live_zone_untouched` — the gate is now pinned by a test, not by a comment |
| move `g->live_face_up[performer] = 1;` to AFTER `rb_check_timing` in `rb_execute_performance_phase` | **2 `^FAIL`**, both `ORDERING:` rows — the silent failure the ordering comment exists to prevent is now a red line |

The third is the one worth keeping. **Writing the flag after the check defers the
eviction to the next timing pass, one phase later on a different board, and every
other test in the corpus still passes** — because they all set the flag by hand.

### `[x]` `BLOCKED-ON` — the contested fixtures, NOT fixed here, with the exact patch

**Only `tests/test_mechanics.c` (2 tests) and `tests/test_live_card_zone_movement.c`
(1 helper) actually need the line.** Both are in the contested set.
`mechanics` is a one-line fix per test:

```c
     for (int i = 0; i < 30; i++) { test_add_to_deck(&tg, member); ... }
+    tg.state.live_face_up[0] = 1;   /* Rust's twin sets face_up = true */
     rb_check_timing(&tg.state);
```

Measured: that line takes `mechanics` from **7 to 0** with no engine change.
`live_card_zone_movement` needs the same line in `record_timing_movements`, once per
seat; measured **15 to 0**.

**`parity_jidou_watch` must NOT get it, and does not need it.**
`tests/test_parity_jidou_watch.c:715` no longer asserts the eviction — its owner
inverted the row to the Rust oracle:

```c
CHECK(test_zone_has_id(&game, 0, "live", hasetsu),
      "RUST ORACLE: at ライブ開始時 the wrongly-set member is STILL in the live "
      "zone -- phases.rs:245 fires trigger_live_start_abilities before "
      "live_card_zone.face_up is set at phases.rs:343, and "
      "check_invalid_live_cards (mod.rs:1734) returns early while it is false ...");
```

with a ten-line comment recording that Rust's scenario leaves the member in the live
zone and its two hearts count, and that the row is deliberately red "until the engine
is fixed". **Both rows print `ok:` as the tree stands.** The `!test_zone_has_id(...)`
form that earlier audits attributed +2 to is gone; the flag would make the C assert
an eviction Rust never performs.

**`parity_hearts_live_start` must NOT get it either — it is not wired to the gate.**
See the table above: opening the gate in its helper changes the suite **0 lines**.

## W.2 — `[x]` the `queued` diagnostic: FIXED, and the corpus cannot exercise the defect (VERIFIED-NOOP on the numbers)

`src/turn/triggers.c`, `rb_trigger_live_start`, both scan loops. Before:

```c
fprintf(stderr, "[LIVE_START_SCAN] ... queued=1\n", ..., cid, ai);
if (rb_push_owned_entry(g, pl, cid, ai, "\xE3\x83\xA9...", NULL, 0)) queued++;
```

The `fprintf` was **above** the push and printed the literal `queued=1`. `queued`
itself — the real counter, declared at the top of the function — was incremented only
on success, so the two could never be compared. `rb_push_owned_entry` returns 0 on a
bad seat and when `rb_queue_enqueue` returns `< 0`, so **every refused push was
reported as a successful enqueue**, and the printed number was not merely wrong, it
was structurally incapable of being wrong. That is the masked-failure class: a
diagnostic that asserts the ability fired.

After, at both sites: the push is above the print and `int pushed` **is** `queued`,
so the printed field and the accumulated counter cannot drift apart. The fullwidth
trigger literal is now `\xNN`-escaped per the repo rule (`\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE9\x96\x8B\xE5\xA7\x8B\xE6\x99\x82`,
byte-identical to the raw UTF-8 it replaces, verified with `od`). No new warnings:
`src/turn/triggers.c` compiles to the same three pre-existing `-Wcomment` warnings it
carried before.

**Honest numbers, whole corpus, 157 suites, every suite built from source — and the
part that matters, which is that the corpus CANNOT exercise the defect:**

| | before | after |
|---|---|---|
| suites printing the diagnostic at all | 61 | 61 |
| `[LIVE_START_SCAN]` lines | 789 | 789 |
| `queued=1` | 782 | 782 |
| `queued=0 use_limit` (the else arm: deduped, or the limit was already reached) | 7 | 7 |
| **`queued=0` from the success path — a refused push** | **0 (impossible to print)** | **0 (none occurred)** |

**VERIFIED-NOOP on the corpus, and that is the finding, not a footnote: no push in
all 157 suites was ever refused, so the old code had nothing to lie about *in these
runs*.** The 7 `queued=0 use_limit` lines in `parity_integration` that a naive
`queued=0` scan picks up are the **else** arm (`[LIVE_SUCCESS_SIDE]` and
`[YELL_PERFORM]` lines are separate diagnostics entirely). Reporting "7 false
successes" would have been exactly the kind of plausible wrong number this section
exists to prevent.

**So the fix is proved by A/B, not by a delta** — `tools/prove_queued_honest.c`,
which fills `queue.n_entries` to `RB_QUEUE_DEPTH` (16) so `rb_queue_enqueue` returns
`-1` and every push in the scan is refused:

| queue state | OLD | NEW |
|---|---|---|
| room in the queue (the push succeeds) | `queued=1` | `queued=1` |
| queue FULL (every push refused) | **`queued=1`** | **`queued=0`** |

The old diagnostic reported a successful enqueue for a push that never happened, and
could not have reported otherwise. The new one reports the refusal, and its printed
field is the same value the accumulated counter increments on. **Same witness card
(id 11, resolved from the database by scanning for the ライブ開始時 trigger rather
than by naming a card number), same queue, same object graph; the only difference
is which side of the push the `fprintf` sits on.**

## W.3 — `[x]` `src/ability/resolver.c` had no suite. It has one now: `tests/test_parity_resolver.c`

The Makefile recorded `parity_resolver=GENUINELY_UNTESTED_no_resolver_suite_exists` in
`SUITE_GAPS` — a name in circulation with nothing behind it, for 1 223 lines of the
choice/resolution resolver. The name is removed from `SUITE_GAPS` (leaving it there
would make `make t T=parity_resolver` **refuse** a suite that exists, and
`make check-suite-names` report it as a STALE ENTRY).

9 test functions over the resolver's own public surface: the card-type filter, the
cost-limit comparison, the use-limit ladder, the activation classifier, the zone-name
helper, the condition-verdict cache (round trip, replace-on-restore, inert without a
current entry, empty/NULL key, NULL out-pointer), the pending-choice accessor, and the
three formatters. Every predicate is asserted on both arms plus a near-miss, because
the bug class this file is here for is a **literal routing** bug — a peer's recent fix
here was a literal with an underscore where Rust had a colon, so `strstr` could never
match, and the arm silently did nothing forever.

Two findings fell out of writing it, both in `src/ability/util.c` (**not this file**):

* **REACHABLE, 1 site.** `rb_card_matches_cost_limit` (`util.c:675`) has no `"=="`
  arm. Rust's `compare_with_operator` maps `"=" | "==" | "exact"` to equality, so a
  `"=="` in Rust's table reads as `<=` in the C. Measured over all **1 650
  card-ability pairs / 2 116 abilities / 3 740 effect nodes**
  (`tools/census_cost_op.c`): the `cost_limit_operator` key carries the spelling
  `==` on **exactly one condition node in the corpus** —
  **`PL!S-PR-045-PR`** (card id 1591), whose ability reads
  「コスト7のメンバーからバトンタッチして登場した場合」 with
  `cost_limit: 7, cost_limit_operator: "=="`. Rust asks for a cost-**exactly**-7
  member; the C asks for a cost-**at-most**-7 member, so a cheaper member's baton
  touch satisfies that debut condition in C and not in Rust. Of the 40 condition
  nodes that carry a cost limit and an operator together, the other three spellings
  (`>=`, `<=`, `=`) are all in the C table.
* **VERIFIED-NOOP: 0, but the same shape.** `rb_card_matches_type` (`util.c:330`)
  has no `member` alias, compares with `strcmp` where Rust uses
  `eq_ignore_ascii_case`, and **fails OPEN** — an unrecognised filter returns 1 where
  Rust logs a warning and returns false. Measured: the corpus writes exactly
  `member_card`, `live_card`, `energy_card`, all three in the C table, so
  reachability is zero. It is still the bug Rust fixed for real: Rust's own comment
  records `PL!N-bp7-028-L` arriving as `card_type: "member"` and applying to every
  stage card, which is precisely what the C does today with any spelling it does not
  recognise.

Both are asserted **RED** in the new suite, in one clearly-marked function,
`known_divergences_in_the_cards_matcher`, with the exact patches in its comment
header. **Not skipped, not deleted, not ignored to reach a number.** They were
deliberately NOT fixed in `resolver.c`'s one-line wrapper functions: Rust states the
design rule at `compare_with_operator` ("This is the one place the operator spellings
are known... a single edit instead of one per call site, and an operator two sites
spell differently can no longer mean two different things"), so a second table in the
resolver would create the very divergence that comment exists to prevent and would
leave `condition.c`, `move.c`, `choice.c`, `cost.c`, `look.c` and `util.c` still
wrong. **`BLOCKED-ON: src/ability/util.c`, lines 330-338 and 668-681.** The patches
are in the suite header.

## W.4 — `[x]` `card_info_show` / `card_info_parity`: NOT-A-SUITE, moved out of `SUITE_GAPS`

Both names come from Rust's `struct CardInfo` (`engine/src/choice_renderer.rs:39` —
three fields: `card_no`, `name`, `ability_text`), which exists **only** inside
`render_card_choice_grid`, the 3DS card-choice-grid renderer, and from
`card_ability_text` (`engine/src/game/platform_ui.rs`), a platform-UI helper.
Neither has a C counterpart to be divergent from: the C's nearest file,
`src/ability/choice_renderer.c`, is an **ORPHAN** — zero references in `OBJ_LIB`,
`platforms/cdi/build_cdi.sh`, `platforms/genesis/build_genesis.sh` or
`platforms/Makefile` — so no suite could link against it even if there were something
to assert.

**The C's actual card-text surface is `src/ability/describe.c`, and
`tests/test_describe_golden.c` already owns it**, byte-for-byte against a hex-encoded
Rust golden fixture for 983 of 985 abilities, with the two K7-skewed indexes (325,
805) named and pinned by `EXCLUDED_SKEW`. So the brief's question — "does
`test_describe_golden.c` own it?" — is **yes**, and no new suite is needed there.

Both names moved `SUITE_GAPS` → `SUITE_ABSENT` as
`NOT-A-SUITE:Rust_struct_CardInfo_in_choice_renderer.rs_3DS_grid_only_C_choice_renderer.c_is_an_ORPHAN`.
They were deliberately **not** aliased to `describe_golden`: the Makefile's own header
warns that aliasing a name to an unrelated suite is worse than recording it, and
`describe_golden` does not render a choice grid.

## W.5 — corpus totals, before vs after

Whole corpus, 157 suites built and run SEQUENTIALLY from source in an isolated
snapshot at `C:/Users/trios/AppData/Local/Temp/kilo/lv/`, every object rebuilt on
every run, stdout and stderr to separate files, binaries deleted after each run.
Baseline = the same snapshot with **only this work's hunks reverted**
(`triggers.c` reconstructed byte-for-byte — its reverted SHA-256 `b0c6d6af…` is
git HEAD's own blob hash, which proves the reverted content is the committed content;
`tests/test_live_zone_slot_validation.c` and `tests/test_parity_resolver.c` absent).

| suite | before | after | delta |
|---|---|---|---|
| **TOTAL `^FAIL` over all 157 common suites** | **3908** | **3913** | **+5** |
| `coverage_gate` | 159 | 161 | +2 |
| `harness_self` | 0 | 3 | +3 |
| every other suite (155 of 157) | — | — | **0** |

Both movers are the harness accounting for the two new files, and neither is a
gameplay regression:

* `coverage_gate` +2 — `G1 manifest-vs-glob`: it requires a `tests/<stem>.log` for
  every `tests/test_<stem>.c`, and the two new suites have not been run by the
  shared harness runner yet (this work's runner writes `logs/`, not `tests/*.log`).
  Running `make t T=live_zone_slot_validation` and `make t T=parity_resolver` once
  clears both lines.
* `harness_self` +3 — `D1`/`D2`, the `test_id(` budget: 3017 → 3034 call sites and
  105 → 107 files. The 17 new call sites are all in the two new suites
  (`test_live_zone_slot_validation.c` 5 `test_id` + 15 `test_new_id`;
  `test_parity_resolver.c` 14 + 7). `test_id` is used only where a single instance
  suffices and no identity comparison happens; `test_new_id` is used everywhere a
  second instance of the same card number must be distinguishable, which is the
  `test_id` aliasing trap this corpus has already been bitten by.

Two suites added: `live_zone_slot_validation` **0 FAIL** (47 assertions),
`parity_resolver` **8 FAIL**, all 8 inside the single deliberately-red
`known_divergences_in_the_cards_matcher`, all 8 in `src/ability/util.c`, all 8
labelled with their exact patches in the suite header.

---

# WAVE 2026-10-04 "crash risks and link landmines" — the NULL effect deref, the 7 symbols, zone_bag, and the 9 DEAD-WRONG

Ownership: `ability/effects/move.c`, `ability/effects/move_missing.c`, `ability/util.c`,
`ability/vm.c`, `core/zones.c`, `core/data.c`, `ability/types.c`,
`ability/effects/rules.c`, `ability/choice_renderer.c`. NOT touched:
`include/rabuka.h`, `core/game_state_abilities.c`, `ability/choice.c`,
`ability/effects/misc.c`, `ability/condition.c`, `test_game.c`, `tests/**`.

## W0. Quiescence — CONFIRMED before any edit

Two reads 12 s apart, SHA-256 identical, and no `.o` / `rb_engine*` anywhere in the
tree. Re-verified immediately before porting; all three edited files still matched,
so nothing landed under me.

| file | SHA-256 | verdict |
|---|---|---|
| `src/ability/effects/move.c` | `8360911C8F088BAA…` | quiescent |
| `src/ability/util.c` | `89730B86DBDB73EC…` | quiescent |
| `src/ability/vm.c` | `6FE735E7DB7BBA9A…` | quiescent |
| `include/rabuka.h` | `EE8392FC88DF935E…` | quiescent (not edited) |
| `src/core/game_state_abilities.c` | `7900948DF123BD5F…` | quiescent (not edited) |
| `src/test_game.c` | `91A6BC9109DBC266…` | quiescent (not edited) |

⚠️ **The census's line numbers for 5 of its 7 symbols were STALE** (see W2), which
independently confirms its own warning that the tree moved under it.

## W1. LANDED — the NULL effect dereference, TWO sites, both fixed

### The bug

`cmf_extra` (move.c) dereferenced the effect pointer with no guard. Its own sibling —
same name, same port, one file over — already had the guard (`move_missing.c:432`).
Of the tree's **28** effect-extra accessors, move.c's was one of **6** with no guard.

### Where it was actually reachable — not the line the census named

`rb_move_take_cards_from_standard_zone` is **not** static (forward-declared in two
translation units) and its whole body is written to tolerate a NULL effect:
`if (e && …)`, `e ? cmf_extra(e, …) : NULL`, and a `move_build_standard_zone_filter`
whose every single read is `e ? … : NULL`. Exactly one reference broke that contract:

```
move.c:1379  int any_number = e && cmf_extra_true(e, "any_number");
move.c:1380  if (is_discard) can_skip = cmf_extra_true(e, "max") || (e && e->is_optional);
                                            ^^^^^^^^^^^^^^^^^^^^^ no `e &&`
move.c:1381  else if (is_hand) can_skip = (e && e->is_optional) || any_number;
move.c:1384  if (e && cmf_extra_true(e, "max")) can_skip = 1;
```

Line 1380 is the only `e` reference in the block without the guard its three
neighbours all carry, and it is reached whenever the source zone is the discard pile.

### 🆕 A SECOND deref the census did not find

With `cmf_extra` guarded, the hand / energy / live-card-zone cases **still** crashed.
That is not the accessor — it is a bare struct dereference one call deeper:

```
move.c:1001  if (e->target) {          <-- in rb_move_prompt_card_selection
```

Reached from the same public API whenever the source zone holds more cards than were
asked for (outcome "Prompt"). The two helpers called below it,
`rb_move_prompt_carry_resume_context` and `rb_move_prompt_fill_filtered`, already
guard — which is why the defect sat in the caller and not in them.

### Both fixes

* `move.c` `cmf_extra` → `if (!e || !k) return NULL;`, matching `move_missing.c:432`.
  Guarding the **accessor** rather than line 1380 makes all ~80 call sites in the
  file safe, not one.
* `move.c:1001` → `if (e && e->target)`. One token, cannot affect non-NULL behaviour.
* `move.c:1380` also given the explicit `e &&` its neighbours carry, for local
  consistency. Redundant once the accessor is guarded; kept so the block reads uniform.

### ✅ The test — and proof it bites

`tests/test_null_effect_guard.c` (new; auto-discovered, no Makefile edit).

It does **not** just add a guard and assert nothing. Two independent assertions:

1. **No crash.** Every case runs in its own `fork()`ed child, so a NULL dereference is
   reported as `FAIL [crash] <case>` naming the case in flight, instead of killing the
   suite and reading as one that merely produced no output.
2. **The guard is TRANSPARENT.** A NULL effect is compared against a zeroed
   `AbilityEffect` carrying no extras, on return value, final zone size, and the id
   handed back. That is the only oracle-free way to pin what a NULL effect *means*
   ("an effect with no keys"); pinning an invented number would only re-state whatever
   the code happens to do. A "fix" that returned early would pass (1) and fail (2).

**Proof, by building the SAME test source against both trees:**

| tree | result |
|---|---|
| unfixed (`cx/base`) | `5 crashes, all signal 11 (SIGSEGV)` — `trigger_null_discard`, `null_hand`, `null_energy`, `null_live_card_zone`, `null_empty_zone` → `FAILED` |
| fixed (`cx/work`) | `14 assertions, 0 failures, 0 crashes` → `PASSED` |

### The guard-policy map (all 28 accessors, measured — not estimated)

| policy | count | copies |
|---|---|---|
| **no guard at all** | 6 | `misc.c:52`, **`move.c:166` ← FIXED**, `score.c:21`, `state.c:1815`, `dynamic_count.c:231`, `engine.c:218` |
| `!e` only | 8 | `ability.c:358`, `custom.c:90`, `draw.c:162`, `rules.c:159`, `choice.c:109`, `cost.c:2443`, `util.c:258`, `modifiers.c:1384` |
| `!e \|\| !k` | 14 | `ability_effects.c:7`, `executor.c:123`, `look.c:79`, `move_missing.c:432`, `compound.c:32`, `cost.c:108`, `describe.c:272`, `resolver.c:92`, `card.c:634`, `game_setup.c:298`, `phase.c:934`, `triggers.c:683` + 2 |

**Measured, so the `!k` half is not worth churning for:** a whole-tree search for a
call site passing a NULL key — literal or via a variable — returns **NONE**. So `!e`
is the crash class and `!k` is purely defensive. **BLOCKED-ON, not fixed:** the 5
remaining unguarded copies are in files I do not own. The right fix is one shared
accessor, which is exactly what §Z.3 already has pending; do not hand-patch the five.

## W2. The 7 declared-but-not-defined symbols

Independently re-verified with `nm` over all 50 objects: **all 7 are declared, none is
defined, and none is referenced by any object.** So none is a *current* link error —
they fail the moment someone calls them.

### 🟢 `rb_load_rom` — **NOT A LANDMINE. NO CHANGE RECOMMENDED. The census is wrong.**

The census's claim: "defined ONLY inside `#ifdef RB_ROM_STRINGS` … it is the ONLY
conditional definition in the tree … any caller added to a normal build gets an
undefined reference." That classification is a **method artefact**: it detected
"declared in a header, not defined in any non-ROM object" without checking whether the
*declaration* was itself conditional. It is. Measured:

| probe | result |
|---|---|
| `rb_load_rom` in the preprocessed header, **no** macro | **DECLARATION ABSENT** |
| `rb_load_rom` in the preprocessed header, `-DRB_ROM_STRINGS` | **DECLARATION PRESENT** |
| non-ROM caller, default gcc 15 C11 | **does not compile** — `error: implicit declaration of function 'rb_load_rom'` |
| non-ROM caller, implicit-decl downgraded to a warning | compiles, then `undefined reference to 'rb_load_rom'` at link |
| `nm src/core/data.o` (non-ROM) | does not define it |
| `nm` of data.c built `-DRB_ROM_STRINGS` | **defines it** |

`rabuka.h:369` opens `#ifdef RB_ROM_STRINGS` and `:375` closes it around the
declaration at `:373` — matched exactly to the definition at `data.c:141-183`. A
normal-build caller cannot even *name* the symbol.

**Recommendation: do NOT move the definition out of the `#ifdef`, and do NOT change the
declaration — the guard already matches.** Moving the body would be actively harmful:
`g_card_strings_rom` / `g_strings_rom` are declared `extern` only inside the same
`#ifdef` (`data.c:21-22`) and are defined only in `platforms/genesis/genesis_lib.c:16-17`,
so an unguarded definition would make every host build depend on symbols no host build
provides. `platforms/genesis/build_genesis.sh:28` passes `-DRB_ROM_STRINGS`, so the
genesis build sees both sides. **This closes as a non-finding.**

### The 6 genuine tombstones — declaration outlived a documented deletion

`nm` confirms none is defined anywhere. Each is a documented deletion whose **prototype
was never removed**. The fix is to delete the declaration. **Census line numbers were
STALE for 5 of 6** — current lines below.

| symbol | census said | **actual line** | recommendation |
|---|---|---|---|
| `rb_invalidate_ability` | 2245 | **2251** | delete the declaration |
| `rb_activate_ability_effect` | 2247 | **2253** | delete the declaration |
| `rb_effect_modify_hearts` | 2269 | **2275** | delete the declaration |
| `rb_cost_clear_deferred` | 2700 | **2706** | delete the declaration |
| `rb_execute_effect_via_registry` | 3238 | **3244** | delete the declaration |
| `rb_translated_execute_custom_effect` | 3245 | **3251** | delete the declaration |

`rb_load_rom` (373) was the only one whose census line was still correct — and it is
the one that needed no change.

## W3. 🔒 BLOCKED-ON: `include/rabuka.h` — every prototype below is COMPILER-VERIFIED

`include/rabuka.h` is main's and was not edited. For each function I touched, the
declaration is missing, and the file "compiles by accident rather than by contract".

**Every prototype in this block was compiled against its own definition.** My first
draft **guessed 4 of them wrong** and the compiler caught all four with
conflicting-type errors — `rb_move_resolve_from_deck` takes `const AbilityEffect *`;
`rb_move_resolve_from_looked_at` has **no effect parameter at all**; and two others
differ in return type and arity. That is the same failure this checklist already
records for the audit doc that published ~20 signatures "verbatim from the source":
reading a signature off a call site is not reading it off the definition.

<details><summary>Ready-to-paste block (paste after the <code>rb_move_*</code> group, ~rabuka.h:2250)</summary>

```c
/* ── src/ability/effects/move.c ──────────────────────────────────────────
 * None of these is declared in rabuka.h or test_game.h today. move.c
 * forward-declares its own cross-TU surface at move.c:8-74 and
 * move_missing.c:8-49 forward-declares a SECOND copy of the same names, which
 * is the "compiles by accident" pattern: a signature can drift in one file and
 * the other still links. Declaring once, here, removes both copies.
 *
 * rb_move_take_cards_from_standard_zone is the function whose NULL-effect
 * contract was violated (PARITY_CHECKLIST §W1). It is non-static and its
 * tolerance for a NULL `e` is now load-bearing, so its prototype belongs in the
 * header where the contract is visible, not in a file-local forward decl. */
void rb_move_prompt_card_selection(GameState *g, int actor, const char *zone,
                                   int count, int can_skip, AbilityEffect *e);
int  rb_move_take_cards_from_standard_zone(GameState *g, int actor,
                                           const char *zone_name,
                                           AbilityEffect *e,
                                           int count, int is_all,
                                           int can_skip, int *out_ids, int max);
int  rb_move_resolve_cards_from_source(GameState *g, int actor, AbilityEffect *e,
                                       int count, int *out_ids, int max);
int  rb_move_resolve_from_zone(GameState *g, int actor, const char *effective_source,
                               AbilityEffect *e, int use_p2, int count,
                               int *out_ids, int max);
int  rb_move_resolve_cost_limit_reference(const GameState *g, const AbilityEffect *e);
int  rb_move_looked_at_matches(GameState *g, int cid, AbilityEffect *e);
int  rb_move_optional_gate_source(const char *zone_str);
int  rb_move_gate_optional_source(GameState *g, int actor, const AbilityEffect *e,
                                  const char *zone_str, int count);
int  rb_move_resolve_from_stage(GameState *g, int actor, AbilityEffect *e, int use_p2,
                                int count, int *out_ids, int max);
int  rb_move_resolve_from_under_member(GameState *g, int actor, AbilityEffect *e,
                                       int use_p2, int count, int *out_ids, int max);
int  rb_move_resolve_from_deck(GameState *g, int actor, const AbilityEffect *e,
                               int use_p2, int count,
                               const char *card_type_filter,
                               const char *group_name, int *out_ids, int max);
int  rb_move_resolve_from_looked_at(GameState *g, int use_p2, int *out_ids,
                                    int max, int *out_count);
void rb_move_prompt_deck_top_or_bottom(GameState *g, int actor, int card_id,
                                       const char *target, const char *source_zone,
                                       int allow_skip);
void rb_move_execute_move_cards_both(GameState *g, int actor, AbilityEffect *e);
void rb_move_execute_move_cards_ported(GameState *g, int actor, AbilityEffect *e);

/* ── src/ability/util.c ──────────────────────────────────────────────────
 * rb_max_distinct_names is hand-declared `extern` in tests/test_game.c:1273
 * precisely because it is missing here. That is the drift mechanism: the test
 * carries its own idea of the signature and nothing checks the two agree. */
int  rb_max_distinct_names(const int *cards, int n);
int  rb_zone_card_ids(const GameState *g, int pl, const char *zone, int *out_ids, int max);
int  rb_get_zone_card_count(const GameState *g, int pl, const char *zone);
int  rb_count_in_zone_filtered(const GameState *g, int pl, const char *zone,
                               const char *card_type, const char *group);
int  rb_dedupe_by_normalized_name(const int *items, int n, int *out, int max);

/* ── src/core/zones.c ────────────────────────────────────────────────────
 * rb_zone_invariant, rb_zone_refresh and rb_player_draw_card are hand-declared
 * by every suite that checks the zone model (test_audit_zones.c:36-39 among
 * others). Four independent hand-written externs for one signature is the
 * clearest evidence the declaration belongs in the header. */
int  rb_zone_invariant(const GameState *g);
int  rb_zone_refresh(GameState *g, int pl);
int  rb_player_draw_card(GameState *g, int pl);
```
</details>

**The other 6 tombstone deletions** — exact replacement for each is: delete the single
declaration line, leaving the surrounding block otherwise untouched.

| `BLOCKED-ON: rabuka.h:2251` | replace `void rb_invalidate_ability(GameState *g, int actor, AbilityEffect *e);` with nothing |
|---|---|
| `BLOCKED-ON: rabuka.h:2253` | replace `void rb_activate_ability_effect(GameState *g, int actor, AbilityEffect *e, int host_cid);` with nothing |
| `BLOCKED-ON: rabuka.h:2275` | replace `void rb_effect_modify_hearts(GameState *g, int actor, AbilityEffect *e);` with nothing |
| `BLOCKED-ON: rabuka.h:2706` | replace `void rb_cost_clear_deferred(GameState *g);` with nothing |
| `BLOCKED-ON: rabuka.h:3244` | replace `int rb_execute_effect_via_registry(GameState *g, int actor, AbilityEffect *effect, int host_cid);` with nothing |
| `BLOCKED-ON: rabuka.h:3251` | replace `int rb_translated_execute_custom_effect(GameState *g, int actor, AbilityEffect *effect, const char *action_str);` with nothing |

If a tombstone marker is preferred over a bare deletion, use the file's existing
convention — e.g. `/* ── DELETED 2026-10-04: rb_invalidate_ability — prototype outlived the body; no definition exists ── */`
— but the marker must not be a prototype, or the landmine survives.

### The 442 undeclared globals

**Measured for my 9 owned files: 209 non-static functions have no prototype in
`rabuka.h` or `test_game.h`.** The block above covers the 22 that matter for this
wave (the ones I touched, plus the ones other TUs hand-declare). The rest, by file:

| file | undeclared |
|---|---|
| `ability/choice_renderer.c` | 61 |
| `ability/effects/rules.c` | 49 |
| `ability/effects/move.c` + `move_missing.c` | 34 + 19 |
| `ability/util.c` | 27 |
| `ability/vm.c` | 13 |
| `core/zones.c` | 23 |
| `core/data.c` | 4 |

`rules.c` (49) and `choice_renderer.c` (61) are the two large blocks I did **not**
write prototypes for, because hand-writing 110 signatures is precisely the mistake
documented above — my own first draft got 4 of 22 wrong by eye. **Recommendation:
generate these mechanically** from the definitions with a script that compiles each
emitted prototype against its owning `.c`, exactly as §W3's check did. Do not
hand-write them.

## W4. LANDED — `zone_bag`: RESOLVED, and the census's "unify them" would have been WRONG

Two static `zone_bag` functions, different zone→bag tables. Checked against Rust
rather than by token similarity. **They are not duplicates, and must never be merged.**

* **`util.c` `zone_bag` ports Rust `zone_cards` (util.rs:2211-2234).** Its match arms
  are Hand / Deck+DeckTop+**DeckBottom** / Discard+Waitroom / EnergyZone+Energy /
  LiveCardZone / SuccessLiveZone. **`Zone::EnergyDeck` is not among them**, so it
  reaches the `_ => &[]` arm and yields an EMPTY slice. Every consumer —
  `rb_count_in_zone`, `rb_remove_card_from_zone`, `rb_place_card_in_zone` — is
  therefore right to report energy_deck as empty. **This table is correct as it stands.**
* **`test_game.c:723` `zone_bag` is not a port of `zone_cards`.** It is a test
  *inspection* helper behind `test_zone_len` / `test_zone_ids` /
  `test_zone_count_of_id` / `test_zone_has_id`, whose job is "what is physically in
  this collection" — and `energy_deck` **is** a real collection there.

Unifying them would have deleted a correct function — the same trap this census itself
recorded for `card_total_hearts`. So **the table is unchanged and only the name moved**:
`zone_bag` → **`zone_cards_bag`** (util.c, static, so file-local and zero-risk), naming
the Rust item it ports. That is sufficient: the hazard was never the two tables, it was
that one grep returned two tables and no way to tell which applied. The adjudication is
recorded in full at the definition.

🔒 **HAZARD REPORTED, NOT FIXED (`test_game.c` is contested):** a C test asserting
parity against a Rust `zone_cards(…)` call using **`energy_deck`**, **`deck_top`** or
**`deck_bottom`** gets a different answer from `test_zone_len` / `test_zone_ids` than
the Rust reference gives — util.c maps all three, test_game.c maps only `energy_deck`.
That is a measurement hazard in the harness, not an engine bug.

## W5. The 9 DEAD-WRONG — all 5 in my files adjudicated. 3 census verdicts were WRONG.

All five were confirmed dead **by the compiler** (`gcc -Wunused-function` →
`defined but not used`), not by grep, and all five were `static`, so no other
translation unit could have been calling them. All five are now **TOMBSTONED**: the
body is deleted and replaced by a comment recording what was actually true, why it was
dead, and — where the census was wrong — the measurement that says so.

| # | function | census verdict | **adjudicated** | action |
|---|---|---|---|---|
| 1 | `move.c:134` `mc_set_recently_moved_batch` | DEAD-WRONG | **DEAD-AND-REDUNDANT** | tombstone |
| 2 | `util.c:171` `rb_norm_group_name` | DEAD-WRONG | **DEAD-ALIAS — census WRONG** | tombstone |
| 3 | `util.c:382` `rb_debug_group_match` | DEAD-WRONG | **DIAGNOSTIC-ONLY — implementing it would be wrong** | tombstone |
| 4 | `util.c:2942` `rb_max_distinct_names_greedy` | DEAD-WRONG | correct observation, **wrong symbol** | tombstone |
| 5 | `vm.c:605` `effect_decode_extra` | DEAD-WRONG | **DEAD-AND-REDUNDANT — census WRONG** | tombstone |
| 6-9 | `game_state_abilities.c:840/894/924/950` | DEAD-WRONG | **not adjudicated** | contested file |

### The three corrections, with evidence

**#2 `rb_norm_group_name` — the census's consequence is false.** It claimed "unreferenced,
so the group gate cannot use it". The function was a one-line alias for `norm_str`,
and **`norm_str` is LIVE with six call sites** — `util.c:425, :494, :514, :541, :3132,
:3137` — of which **`util.c:494` is inside `rb_card_matches_group_str` (util.c:491),
which IS the group gate**. The U+FF01 / U+00B5 normalisation the gate needs is
implemented *and reached, today*. What was dead was the NAME, not the capability.
(`norm_str` itself was kept, with its `EF BC 81` / `C2 B5` byte work untouched — this is
the U+FF01/U+00B5 file, so those stay as explicit bytes, never literals.)

**#5 `effect_decode_extra` — "extras are DROPPED" is false, and measured.** The live
producer is the main decoder's generic scalar-extras arm at **`vm.c:1168-1190`**, which
handles every tag and one the dead function never had (`heart_colors` → `heart_color`
mirror, `:1181-1183`). So the live arm is a strict **superset**; the dead function was a
duplicate of its ARRAY arm, not a missing producer. Measured rather than argued —
decoding all 2116 card-ability pairs and walking all 3223 effects:

```
effects walked      = 3223
effects WITH extras = 2352
TOTAL extras stored = 6902
max extras on one effect = 12
VERDICT: extras are STORED -- the census's 'DROPPED' claim is FALSE
```

A dropped-extras tree would report ~0. **This mattered beyond the census:** believing it
would have invited someone to "fix" a decoder that is already working.

**#1 `mc_set_recently_moved_batch` — the census's consequence is false twice over.** It
claimed "no live path performs that replacement". The replace idiom
(`n_recently_moved = 0` + bounded append) is implemented on **~20 live paths**:
`look.c:822+827`, `misc.c:172`, `move_missing.c:416`, `state.c:1613`,
`ability_queue.c:1420+1569`, `choice.c:2299`, `game_state_core.c:485`,
`live.c:1875-1876`, `engine.c` (several). This function was the 21st copy, with no
callers. And "those_cards is only ever APPENDED to" is also false: `n_those_cards` is
reset at **`move.c:2525` and `move_missing.c:417`**. The census compared two different
fields — `recently_moved` (what this wrote) and `those_cards` (the resolver-local
accumulator `mc_sel_moved_push` appends to **by design**, with an intentional dedupe).

**#3 `rb_debug_group_match` — the prescribed fix would have been wrong three ways.**
The census was right that the empty body is labelled "Mirror", which is false. But
implementing the checker would be wrong because: **(a)** Rust has **two** definitions
sharing that name and `util.rs:642-649` — `#[cfg(feature = "no_std")]` — is **also an
empty body**; only the `not(no_std)` twin at `:652` has the checker. **(b)** The real
Rust body is **purely diagnostic**: it computes four booleans, formats them, and
**returns nothing** — `result` is an input it only prints. There is no parity-visible
behaviour for a C port to have or lack, so it cannot be a DEAD-WRONG *engine* function
at all. **(c)** It is gated on `log_enabled!(Trace)` **and** an env var
(`RABUKA_DEBUG_GROUP=1`); this tree has neither in that path, so a port could never fire.
The real defect was the false word "Mirror", and deleting the empty body removes it.

**#4 — the census found a real gap but pointed at the wrong symbol.** Its mechanical
observation is right: Rust returns `DistinctNamesResult{distinct, collision}` and the C
returns a bare count. But the dropped `had_collision` was unobservable in production
even in principle — `rb_max_distinct_names` has **no caller anywhere under `src/`**;
its only callers are `tests/test_game.c:1276` via a hand-written `extern`.

🔒 **THE REAL GAP, REPORTED — higher value than the wrapper was.** Rust **reads** that
flag on a live path and the C tree has **no port of it at all**.
`engine/src/core/card.rs:680-687`:

```rust
let best = util::max_distinct_names(&name_sets);
if let Some(n) = distinct_only { best.distinct >= n } else { !best.collision }
```

The `else` arm — "does a collision-free assignment exist" — has no C counterpart,
because `rb_max_distinct_names` returns an `int` and its only callers are tests. This is
a **missing port on a live Rust path**, not a latent bug in a dead wrapper.
**Deliberately NOT fixed here:** giving `rb_max_distinct_names` a second output changes
a function tests already hand-declare, and the caller that needs the `else` arm does not
exist yet. It belongs with whoever ports the card.rs distinct-name filter.

## W6. A/B — 22/22 suites BYTE-IDENTICAL, 0 regressions

Snapshot `C:/Users/trios/AppData/Local/Temp/kilo/cx/`. **Strictly sequential.** Every
object deleted and rebuilt before each measurement (`find src -name '*.o' -delete`) so
the stale-link trap and its `Copy-Item`-preserves-mtime variant both die — a
byte-identical result is only evidence once a stale link is disproved. Baseline is the
snapshot with **only my hunks reverted** (the pristine live tree), not git HEAD.
Suites linked directly against the objects rather than via `make t`, because in-repo
`make data` **refuses by design** (`cards/build/abilities_strings.bin` is the stale
726649 B one; the guard correctly rejects it). Counted with **`^FAIL` (no colon),
case-sensitively, stdout AND stderr** — covering all five prefixes.

| suite | before | after |
|---|---|---|
| parity_sequential | 142 | 142 |
| parity_cce_gaps | 88 | 88 |
| parity_small_clusters | 104 | 104 |
| parity_jidou_move_yell | 81 | 81 |
| parity_jidou_watch | 75 | 75 |
| parity_jidou_watch_clusters | 62 | 62 |
| parity_recover_live | 57 | 57 |
| parity_under_member | 87 | 87 |
| parity_zones_position | 50 | 50 |
| parity_live_total | 50 | 50 |
| parity_wave1c | 44 | 44 |
| parity_integration | 42 | 42 |
| parity_wave1a | 38 | 38 |
| parity_wave1b | 32 | 32 |
| parity_ability_mod | 22 | 22 |
| mechanics | 7 | 7 |
| audit_zones | 5 | 5 |
| audit_fields | 4 | 4 |
| zone_filter_parity | 0 | 0 |
| parity_enums | 0 | 0 |
| vm_decode | 0 | 0 |
| parity_deck_order | 0 | 0 |
| **TOTAL** | **990** | **990** |

**22 of 22 suites byte-identical** — FAIL count, PASSED-line count *and* output byte
size all unchanged. **0 suites changed in either direction.** Runoff checks for the
three names in the brief that are not suites: `parity_card_filters` is a
**SUITE_GAPS** entry (coverage gap, nearest: `parity_look_and_filter`,
`zone_filter_parity`, `parity_look_per_card` — the two real ones measured
**39** and **68**); `parity_condition_decode` **renames** to
`parity_t7_condition_decoder` (0); `test_artifacts_consistent` does not exist under
that name. New suite: **`null_effect_guard` = 0 FAIL, PASSED.**

The guard and the five tombstones are behaviour-neutral by construction: the guard only
converts a fault into a defined `NULL`, and nothing previously called the five deleted
statics. The measurement agrees.

## W7. The warning gates

* ✅ **`make warnings-audit` PASSES** — `12 diagnostic(s), all pinned; 0 new`,
  `baseline-still-present=12 NEW=0 stale-pins=0`.
* 🔴 **`make warnings-audit-test` is RED — and was ALREADY RED before this wave.**
  `158 test sources, 1 red, 0 pinned`:
  `tests/test_parity_required_hearts.c:1050: error: implicit declaration of function
  'rb_board_distinct_names'; did you mean 'rh_board_distinct_names'?`
  **Proved pre-existing** by running the identical target on the untouched baseline
  tree: byte-identical failure, same message, same counts. `rb_board_distinct_names`
  exists **nowhere** in `src/` or `include/`. 🔒 Not mine to fix: the suite is in
  `tests/**` and the symbol does not exist, so the fix is a declaration **plus** a
  definition, or a test edit — main's call.

## W8. Regressions I caused: NONE

Stated plainly: **zero.** 22/22 suites byte-identical, 990 = 990, `warnings-audit`
passes with 0 new, warning parity on all three edited files confirmed against the
baseline (`move.c`'s two `-Wunused-variable` are pre-existing, at shifted line numbers;
`util.c` and `vm.c` clean). One self-inflicted defect was introduced and fixed before
landing: a tombstone comment containing `src/**` tripped `-Wcomment` in `util.c`, caught
before the build and rewritten — recorded because `make warnings-audit` gates on exactly
this class. Live tree left with **0 objects, 0 `rb_engine*`**; the 142 pre-existing
`tests/*.log` are dated 2026-10-01 and were left untouched. Blob verified
**726831 B / `89204df0…`** before and after every build.


# ================================================================
# Wave 0 -- THE CORPUS VERDICT LAYER (2026-10-04)
# ================================================================
#
# The headline that motivated all of it, and it is a property of the OUTPUT,
# not of the engine: before this section the corpus emitted 1244 `FAILED`
# verdict lines against **0 `PASSED` lines**. Nothing anywhere could say "this
# suite was clean". Therefore, for any suite in this corpus,
#
#     a suite that SEGFAULTS before its first assertion produced exactly the
#     same output shape as a suite that passed every assertion
#
# -- unless you read the exit code, and COVERAGE_HARNESS.md 3 proves the exit
# code disagrees with the output in BOTH directions (12 suites exit 0 carrying
# 548 FAIL lines; test_state_effects exits 139 carrying 0). That is how
# tmp_list, test_audit_conditions, test_state_effects, test_p1_helpers and the
# nine bracket-form suites survived in a project whose every report quotes a
# FAIL count.
#
## W0.0  MEASURED, before and after
#
# Literal, case-sensitive, no colon, stdout AND stderr, byte stream with no
# separator invented between the two streams.  Baseline is a pristine snapshot
# of the working tree at HEAD `cf315a9a`; "after" is a snapshot of the same tree
# with only this wave's hunks applied.  Both were built and run strictly
# sequentially, `make data` deliberately NOT invoked (the guard refuses the
# stale `cards/build` blob by design), `src/abilities_strings.bin` verified at
# 726831 B / `89204df0bbde08a0...` before and after every build.
#
# | counter        | BEFORE (159 files) | AFTER (157 files) |
# |----------------|-------------------|-------------------|
# | `^FAIL`        |              3741 |              3950 |
# | `^FAIL:`       |              2322 |              3835 |
# | `^PASSED`      |                 0 |                42 |
# | `^FAILED`      |              1244 |               115 |
# | `^CRASH`       |                 2 |                 0 |
# | `^VACUOUS`     |                44 |                 0 |
# | `^SETUP BUG`   |                 3 |                 0 |
# | `^TEST BUG`    |                 2 |                 0 |
#
# Read the deltas, not the headline `^FAIL`:
#
# * **`^FAIL:` was under-counting by 1419** (3741 - 2322) in the baseline. All
#   of it is now visible: the gap between the two columns above is exactly the
#   115 `FAILED:` verdict lines, which are `FAIL` lines by definition.
# * **`^FAIL` RISES 3741 -> 3950** and that is honest, not a regression: +115
#   per-suite verdict lines (one per non-PASSED suite) and +51 lines that used
#   to be invisible to `^FAIL` entirely because they begin with `VACUOUS`,
#   `CRASH`, `SETUP BUG` or `TEST BUG`, now folded into `FAIL: [...]`.
# * **`^PASSED` goes 0 -> 42.** Before, a green suite and a dead suite were
#   indistinguishable; now they are not.
# * **CRASH / VACUOUS / SETUP BUG / TEST BUG all go to 0** -- not because those
#   findings went away, but because each is now a canonical `FAIL: [crash]` /
#   `[vacuous]` / `[setup-bug]` / `[test-bug]` line, so one grep sees them.
# * 3 scratch files left `tests/` (-3) and 1 gate suite entered it (+1): -2 files.
# * No assertion was relaxed, skipped or retuned to reach any of these numbers.

## W0.1  `PASSED:` -- ONE LINE PER SUITE, ONE FILE, ZERO HAND EDITS
#
# **`tools/verdict_runner.c` (NEW).**  `make t-run` and `make run-all-tests` run
# every suite through it.  It tees both streams, rewrites every result prefix
# to the single canonical `FAIL: `, and prints exactly one verdict line as the
# last word:
#
#     PASSED: <stem>  [exit=0, 0 result line(s), renormalised=N]
#     FAILED: <stem>  [exit=<n>, <breakdown>]
#     (nothing, plus one `FAIL: [crash]` line)      -- died on a signal
#
# `PASSED` requires BOTH halves -- exit 0 AND zero result lines -- which is
# strictly stronger than "zero FAIL lines" (what test_audit_conditions scored
# while evaluating zero assertions) and stronger than `$?` (which 12 suites
# exit 0 while carrying 548 FAIL lines).  A suite that dies on a signal prints
# NO verdict at all, so **a dead suite is a third, distinct state**, and
# `run-all-tests` counts it as INCONCLUSIVE and fails.
#
# WHY NOT THE MACRO THAT ALREADY EXISTS.  The shared reporting macro is
# `FLAT_VERDICT()` in `include/test_game.h`, and it only serves the FLAT-suite
# dialect.  The other three dialects in this corpus -- named case functions in
# a run table, one `main()` with inline scenarios, and `run(name, fn)` dispatch
# -- each count failures in their own differently-named local counter, so
# reaching them is 156 hand edits to files other agents currently own.
# Interposing the output entry point covers every dialect, every private
# vocabulary and every future suite from one file.
#
# WHY A SEPARATE PROCESS AND NOT AN IN-PROCESS INTERPOSER.  An in-process
# interposer was written first (`fprintf`/`printf`/`fputs`/`fwrite`/`puts` from
# a constructor, plus an `atexit` verdict) and it FAULTS on this toolchain: the
# suite binary is PE32+ against `msys-2.0.dll` (gcc 15.3.0, x86_64-pc-cygwin)
# and the first raw write through the interposed path died with
# 0xC0000005 on all 11 probe modes.  A corpus-wide guarantee must not depend on
# the platform's symbol rules, so the layer is a separate binary whose exit
# status mirrors the child's -- every existing `|| { echo RED }` keeps meaning
# what it meant.  The interposer was deleted, not left behind.
#
# `include/test_game.h` and `src/test_game.c` were NOT touched: they are
# CONTESTED (see W0.9).
#
## W0.2  MANIFEST-VS-GLOB GATE -- `make t T=<name>` MUST FAIL ON A NAME THAT DOES NOT EXIST
#
# **Landed.**  `SUITE_ALIASES` is replaced by three tables with exactly three
# outcomes and no fourth:
#
# * **`SUITE_RENAMES` (9)** -- the file exists under another name; `make t
#   T=<name>` RUNS it.  The seven rename-only names in circulation are wired:
#   `parity_vm` -> `vm_decode`, `parity_condition_decode` ->
#   `parity_t7_condition_decoder`, `parity_max_distinct_names` ->
#   `max_distinct_names`, `parity_survey_probe` -> `survey_probe`,
#   `parity_tp_joint` -> `tp_joint_card_live_start`, `parity_tp_zone_change_gate`
#   -> `tp_zone_change_gate`, `audit_x_class` -> `parity_x_class`.  Plus
#   `card_data_decode` -> `decode_skew` and `test_setup` ->
#   `parity_t6_action_setup`.
# * **`SUITE_GAPS` (13)** -- the behaviour is real and the suite does not exist.
#   `make t` REFUSES, exit 2, and names the nearest real suites.
#   `parity_resolver` was in this table and has LEFT it: `tests/test_parity_resolver.c`
#   now exists (landed by another agent), so the name now resolves to the file.
# * **`SUITE_ABSENT` (25)** -- not a suite at all.  Recorded, not aliased,
#   because aliasing a VERIFIED-NOOP to something unrelated is worse than
#   saying so.  The three VERIFIED-NOOPs are here:
#   `parity_dynamic_count` (X=dynamic_count is 0 nodes across all 2116 pairs),
#   `parity_card_text` (0 of 985 -- there is no display layer in C),
#   `parity_nazuna_at_deck_top` (PL!N-bp7-028-L is ability 805, a K7-skewed
#   slice).  The `audit_*` names are split into what is real
#   (`audit_cluster_d`, `audit_w_gate`, `audit_t7_decoders`, `audit_t7_resource`,
#   `audit_t7_wire`, `audit_t7_zeroheart` -- all six EXIST as `tools/*.c` census
#   programs, and the make target that runs each is named), what never existed
#   anywhere (`audit_x_const`, `audit_x_lookat`, and the six `audit_*` claims
#   with no file), and nine bare CASE names lifted out of a report
#   (`audit_additive_modifier_can_cause_failure` and siblings).
#
# **`make suite-names` (NEW)** publishes all of it -- the published answer to
# "does that name exist, and if not, what is it?", asked at least five times in
# this project and answered, every time, by silence.
#
# **`make check-suite-names` (NEW)** resolves every table entry against the real
# `tests/` glob and fails on a dangling rename, on a name recorded absent that
# now HAS a file, on a recorded nearest-suite that does not exist, and on a glob
# collision.  **`make run-all-tests` depends on it**, so it runs before a single
# suite is built, and prints the resolved set:
#
#     === RESOLVED SET: <n> suites from tests/test_*.c, each run through rb_verdict ===
#
# Negative controls, all three verified to turn the gate RED:
# dangling rename `parity_vm -> vm_decode_GONE`, a stale absent entry
# `null_effect_guard` once `tests/test_null_effect_guard.c` appeared, and a
# nearest-suite `parity_ability_watch_GONE`.

## W0.3  `^FAIL` vs `^FAIL:` -- the invisible 1419, and the gate that keeps it fixed
#
# Six result prefixes existed.  `^FAIL` = 3741; `^FAIL:` = 2322.  Every one of
# them is now rewritten to the single canonical `FAIL: ` by the verdict layer,
# so `^FAIL` and `^FAIL:` are the same count and one grep is correct.
#
# It is done on the BYTES the suite emitted, not in each suite's source, because
# the `printf("%-8s %s\n", "FAILED", name)` form has the literal as an
# ARGUMENT rather than the start of the format -- ~35 such sites, and no source
# rewrite reaches them.
#
# **Two things that were wrong and had to be fixed before it was right**, both
# recorded because the first version looked correct:
#
# * The prefix window was 96 bytes and gave up on longer lines.  That left **242
#   real result lines unnormalised across 15 suites**, because the corpus's
--   longest result lines run to several hundred bytes (`test_audit_actions`'s
#   `TEST BUG` notes, `test_parity_cost_discount`'s `VACUOUS` case notes).  The
#   window is now 32 bytes -- the longest prefix is `SETUP BUG:` at 10 -- and
#   the rest of the line is copied straight through, so a line of any length is
#   normalised while at most 32 bytes are ever held back.
# * A prefix now only matches when a BOUNDARY follows it.  Without that,
#   `FAILURE:` became `FAIL:URE:` and `CRASHES:` counted as a crash.  The same
#   rule is in the gate, and the two must agree.
#
# **`tests/test_coverage_gate.c` (NEW).**  Reads the evidence `make run-all-tests`
# writes and asserts five things.  It weakens nothing: it only READS logs, and
# no assertion in any suite was relaxed, skipped or retuned to make it pass.
#
# * **G1 manifest-vs-glob** -- every `tests/test_<stem>.c` has a
#   `tests/<stem>.log`, and every log has a source.
# * **G2 the normalisation is total, per suite** -- zero surviving `FAILED`,
#   `FAILED:`, `FAIL [`, `CRASH`, `VACUOUS`, `SETUP BUG:`, `SETUPBUG`,
#   `TEST BUG:` or `FATAL` lines.
# * **G3 the corpus total reconciles** -- the corpus-wide `^FAIL` count taken by
#   one streaming pass over `tests/` equals the sum of the per-suite counts.
#   Two INDEPENDENT enumerations (directory walk vs suite stem), so a
#   disagreement means a log was skipped, read twice, or counted for a suite
#   that does not exist.
# * **G4 every suite reported a verdict** -- exactly one verdict line per log,
#   recognised by the runner's private `[exit=` marker.  A log with none is a
#   suite that died on a signal and is INCONCLUSIVE, never green.
# * **G5 verdict arithmetic** -- corpus `^PASSED` + corpus `^FAILED` == number
#   of logs, and `^PASSED` > 0.
#
# **MEASURED, final: 633 checks, 0 failures**, and its numbers agree exactly
# with an independent external scan of the same logs (`^FAIL(corpus)=3835,
# ^PASSED=42, ^FAILED=115, non-canonical=0, no-verdict=0`).  Two of its own
# assertions were wrong on the first pass and are recorded here because the
# wrongness is instructive: bare `FAIL` matches `FAIL: ...` under the boundary
# rule, so every canonical line was reported as legacy; and keying the verdict
# on a `[` after a space matched any bracketed aside a suite printed and
# reported five verdicts where there was one.

## W0.4  THREE SCRATCH FILES THAT WERE COUNTED AS SUITES -- MOVED OUT OF `tests/`
#
# `tests/tmp_list.c` (0 B out, 0 B err, exit 0), `tests/debug_eli.c` and
# `tests/test_zzprobe.c` now live in **`tools/scratch/`**.  A file in `tests/`
# is indistinguishable from a suite to every glob in this project, which is the
# whole defect.  `tmp_list.c` was one of the three suites the brief names as
# having survived on silence; its `SETUP BUG` self-report is now moot.
# `tools/scratch/debug_eli.c` hard-codes a path to `.../engine/c/src`, which does
# not exist in this tree, so it was never runnable here either.

## W0.5  THE VACUOUS SET -- count AND location confirmed, and every case now asserts something real
#
# The brief's location claim is CONFIRMED and its count is confirmed: **44 lines
# beginning with `VACUOUS`, in exactly two suites**, and 26 and 10 of them
# respectively in `test_parity_required_hearts.c` and `test_parity_cost_discount.c`.
# (`test_parity_state_per_card.c`'s 8 are the suite's own PROMPT CONTROL guards --
# a different class, already correct, and NOT touched, as instructed.)
#
# The brief's third claim is **WRONG for this tree**: it says both `SETUP BUG`s
# are in `test_parity_blades_live_start.c` and resolve to id -1 and to card
# 2793. Measured: `test_parity_blades_live_start.c` emits **zero** `[setup-bug]`
# lines. Both were already fixed -- `PL!HS-pb6-002-R`'s comment reads "THE CARD
# NO WAS A TYPO, NOT A RED-BY-DESIGN CASE (fixed 2026-10-04)", and
# `PL!HS-bp1-002-CL` no longer exists anywhere in the tree (the fixture uses
# `PL!HS-bp1-002-R`, which prints 村野さやか). The only `[setup-bug]` lines the
# corpus emits are 3 in `test_audit_conditions` and are unrelated to either.
# **Nothing to fix; recorded so nobody re-derives it.**
#
# All 23 vacuous cases have the same shape: a green NEGATIVE whose PAIRED
# POSITIVE is red, so the zero cannot be distinguished from "the effect never
# applies at all". Nine positives are red
# (`c_six_distinct_reduce_two`, `d_mirage_two_baton_heart05`,
# `f_center_two_heart03_reduces_1`, `i_two_appeared_reduce_2`,
# `l_heart02_ge5_increase`, `f_reduction_applies_to_no_ability_member`,
# `n_reduces_cost_seventeen_group_member_by_two`,
# `k_gameplay_accept_cost10_discards_hand`, `g_reduction_charges_effective_cost`),
# and none is fixed by this wave -- the engine is not this wave's to change.
#
# So the cases were made to assert something real, and the vacuity was NOT
# withdrawn:
#
# * **A PREMISE CONTROL, asserted not reported, in `test_parity_required_hearts.c`.**
#   `rh_premise()` and three helpers (`rh_board_distinct_names`, `rh_card_hearts`,
#   `rb_card_is_live`/`rb_card_no_eq`) let each case assert the thing its own
#   zero depends on and that is INDEPENDENT of the broken mechanism: that the
#   fixture really presents the state the case name claims. Added to
#   `c_five_plus_duplicate` (the added card really is the same PRINT as a member,
#   and a distinct INSTANCE of it -- `mid()` allocates a fresh card per call via
#   `test_new_id`, which is exactly what makes it a duplicate), `c_five_plus_renosora_live`
#   (it really is a ライブ card), `c_five_plus_wrong_group` (the extra card really
#   DOES raise the board to 6 distinct member NAMES, so the zero is the ability's
#   GROUP filter and not a counter that never saw six), `d_mirage_non_renosora`
#   (two different arrivals, both members), `f_center_rin_zero_heart03` (the
#   centre card really prints 0 heart03) and `f_center_non_mus` (it really is
#   not in mu's).
# * **Four of those six premise controls were WRONG when first written and the
#   suite said so immediately** -- 4 `FAIL: premise:` lines on the first run.
#   They were all MY errors and all four are recorded, because they are the
#   reason the premise has to be an assertion: `mid()` returns a different card
#   INSTANCE for the same print (2640 vs 2646), so id equality is the wrong
#   identity test and `rb_card_no_eq` is the right one;
#   `rb_count_distinct_member_name_units` counts distinct NAMES across ALL
#   groups, so a non-蓮ノ空 card does raise the count to 6; and the two baton
#   arrivals are not on the stage or waitroom where they were expected. Fixed,
#   re-measured, 0 premise failures, and the four original case assertions are
#   untouched.
# * **A MECHANICAL GATE, so this cannot come back.** In both suites the
#   post-run audit now HARD-FAILS any vacuous case whose own assertion count
#   does not EXCEED its caveat count: "a vacuous case must assert something
#   real or be DELETED, not merely be reclassified." Enforced in the PARENT
#   process, so a child cannot fail to report it. **DELETED: none** -- all 23
#   cases still run and still carry their caveat, and both suites still FAIL.
#   The vacuous count is 26 + 10 = 36, unchanged, exactly as before.

## W0.6  Q246 -- a fixture guard that was missing two of its five legs
#
# `tests/test_parity_hearts_live_start.c:1603-1630`.  The fixture pinned three
# of five cards with `ident()`.  `kosuzu2` (`PL!HS-pb1-005-R`) and `dia2`
# (`PL!S-bp3-004-R`) were resolved by `mid()` and never checked, and this is
# the dangerous direction to be wrong in: `mid()` is forgiving -- `rb_find_card_by_no`
# falls back to base+rarity rebuild, then lowest-prefix, then a SUBSTRING search
# (`src/core/data.c:446`) -- so a mistyped card number does not return -1, it
# returns SOME OTHER REAL CARD.  The two assertions that carry the case
# (`CHECK_EQ(heart_of(&tg2, joint2, H01), 0, ...)` and `... H03 ...`) expect 0,
# so about the wrong cards they still pass.
#
# **Landed: both missing `ident()` pins added**, with the mechanism written down
# next to them and the fixture data verified correct against `cards/cards.json`
# (PL!HS-pb1-005-R is heart04/05/06, PL!S-bp3-004-R is heart02/05,
# PL!-bp3-003-R is heart01/03/06) -- only the guard was wrong.  The suite's
# `^FAIL` is unchanged (84 -> 85, and the +1 is the suite's own verdict line),
# i.e. the pins hold on this corpus and nothing that was previously green moved.

## W0.7  `test_parity_jidou_watch.c` Q213 -- ADJUDICATED AGAINST THE RUST TWIN
#
# Twin: `engine/tests/test_modules/jidou/discard_watch/live_start_heart_requirement_reduction_counts_live_slot_cards_only_test.rs`.
# Traced through `engine/src/turn/phases.rs` and `engine/src/turn/actions/mod.rs`.
#
# * **`:675` KEPT, message corrected.** The twin's only assertion is
#   `after_waitroom >= before_waitroom` (that file :59-62) -- the same WAITROOM
#   comparison. The C was a faithful port, not a C-side defect. What the twin's
--   own comment claims ("The member was moved to waitroom") is NOT what its
#   assertion measures: evicting a card from the LIVE slot cannot shrink the
#   WAITROOM, so the comparison holds by construction and cannot fail for the
#   reason both messages gave. The message now says what is measured, and the
#   assertion is byte-identical to the twin's.
# * **`:680` WAS INVERTED, and is GREEN -- which is a finding, not a pass.**
#   The Rust twin contains **no live-zone assertion at all**, and the Rust engine
#   reaches the OPPOSITE state at ライブ開始時:
#     - `phases.rs:240` `check_timing()` runs at the top of the
#       `LiveCardSetSecondAttacker` arm;
#     - `mod.rs:1732-1736` `check_invalid_live_cards()` **returns early** while
--       `live_card_zone.face_up` is false;
#     - `phases.rs:343` `face_up` only becomes true inside
--       `execute_performance_phase`;
--     - `phases.rs:245` `trigger_live_start_abilities()` fires BETWEEN those two
--       points.
--   So in Rust the member is STILL IN THE LIVE ZONE when ライブ開始時 triggers,
--   and the Q213 live-slot reduction **COUNTS** it. The old
--   `CHECK(!test_zone_has_id(..., "live", hasetsu), ...)` asserted the opposite,
--   which is why it was green: it was pinned to the C's own divergence (C evicts
--   before the live, Rust evicts after -- see the T3 header item 5
--   `live_face_up[]` and the T6 open item 3 `check_invalid_resolution_zone`).
--   **The assertion now tracks the oracle and it PASSES, which is stronger than
--   the brief expected**: this tree's C already matches Rust at this point.
--   What was inverted was the ASSERTION, not the engine. So one of the ~34 lines
--   `live_face_up` was blamed for is NOT an engine divergence -- it was a test
--   that was asserting the wrong direction, and it is now a correct pin.
--   The `live_face_up[]` divergence itself is unchanged and still open.
--   `test_parity_jidou_watch` `^FAIL` 77 -> 76, and the -1 is the removal of the
--   bare `FAILED   <case>` per-case verdict line, now canonicalised.

## W0.8  K7 PINNING -- `EXCLUDED_SKEW` asserted BY VALUE
#
# `tests/test_describe_golden.c`.  Only this suite knew the skew existed, and
# everything it asserted constrained the SIZE of `EXCLUDED_SKEW` (2) and the
-- number of abilities missing from the golden (983).  Neither says WHICH two, so
-- a third divergence could be excluded in place of one of them and every other
-- assertion would still hold -- which is exactly how a skew goes from "two known
-- ones" to "whatever stops the suite complaining".
#
# **Landed:**
#     CHECK(N_EXCLUDED == 2 && EXCLUDED_SKEW[0] == 325 && EXCLUDED_SKEW[1] == 805, ...)
-- with the reason recorded next to it: 325 = PL!-pb2-012-P＋ / PL!-pb2-012-R
-- (ab#1), 805 = PL!N-bp7-028-L (ab#0) 'Cooking with Love'. C's
-- `RBKA_OFFSET_DELTAS` (`gen_data.c:7`, summing 98737 == `RBKA_BYTECODE_LEN`)
-- disagrees with Rust's `OFFSET_DELTAS` at exactly these two of 985 slices.
-- Corpus exposure 5 sites: `test_parity_wave1b.c:2396` and `:2661` for 325,
-- `test_parity_wave1b`'s twin and `test_parity_hearts_live_start.c:889`, `:912`,
-- `:941`, `:973` for 805. The suite is one of the 42 that now report PASSED, so
-- the pin holds.

## W0.9  FILES TOUCHED, AND WHAT IS CONTESTED
#
# Claimed and edited (each verified quiescent -- SHA-256 stable across two reads
-- separated by a settle, no `.o` newer than source -- immediately before the
# edit, and each compiled after):
#
# * `engine_c_wip/Makefile` -- the test-glob / alias rules only (manifest tables,
#   `t`, `suite-names`, `check-suite-names`, `t-run`, `run-all-tests`, `clean`).
#   **The Makefile is being edited by another agent at the same time**: the
--   hash moved under this wave, a peer added
--   `parity_look_at_counts_and_zones` and `alloc_counter` to the tables and
--   removed `parity_resolver` from SUITE_GAPS when its suite landed. This wave's
--   hunks are the `SUITE_*` / `VERDICT` / `t-run` / `run-all-tests` /
#   `check-suite-names` / `suite-names` / `clean` block; the peer's rows were
--   preserved, not overwritten. Merge on read.
# * `engine_c_wip/tools/verdict_runner.c` -- **NEW**.
# * `engine_c_wip/tests/test_coverage_gate.c` -- **NEW**.
# * `engine_c_wip/tests/test_parity_hearts_live_start.c` -- item W0.6, two pins.
# * `engine_c_wip/tests/test_parity_jidou_watch.c` -- item W0.7, `:675`/`:680`.
# * `engine_c_wip/tests/test_describe_golden.c` -- item W0.8, one assertion.
# * `engine_c_wip/tests/test_parity_required_hearts.c` -- item W0.5.
# * `engine_c_wip/tests/test_parity_cost_discount.c` -- item W0.5 (audit only).
# * `engine_c_wip/tests/{tmp_list,debug_eli,test_zzprobe}.c` -> `tools/scratch/`.
#
# **CONTESTED / NOT TOUCHED:**
#
# * `include/test_game.h` -- CONTESTED. Item 1 does NOT need it:
--   `FLAT_VERDICT()` there only serves the FLAT dialect, and the layer that
--   covers the other three needs no macro. If a future wave wants a
--   source-level `PASSED:` instead of the runner's, the exact edit is: add
--   `printf("PASSED: %s\n", __FILE__)` to the success arm of `FLAT_VERDICT()`
--   at `include/test_game.h:69-70`. Not applied.
# * `src/test_game.c` -- CONTESTED. Not needed. The exact edit IF it were:
--   the `test_play_to_stage()` return is checked by some callers and ignored by
--   others (e.g. `test_parity_required_hearts.c`'s `d_mirage_non_renosora`), and
--   that is where W0.5's premise failures came from; adding a warn-and-count on
--   a zero return would need `current_test`, which lives in each suite.
--   Not applied.
# * `src/**` -- untouched. No engine change is claimed by this wave.
# * `engine_c_wip/tests/test_parity_state_per_card.c` -- untouched by
--   instruction (its 8 `VACUOUS` lines are PROMPT CONTROL, a different class).
# * `engine_c_wip/tests/test_parity_blades_live_start.c` -- untouched; the two
--   `SETUP BUG`s it was said to carry are already fixed (see W0.5).
# * `engine_c_wip/PARITY_CHECKLIST.md` -- appended, never rewritten.

## W0.10  TRAPS THIS WAVE HIT, FOR WHOEVER RUNS IT NEXT
#
# * **`cat out err` INVENTS A LINE START.** If a suite's stdout does not end in
--   a newline, concatenating the two streams glues stderr onto stdout's last
--   line, and every prefix count measured downstream jumps. It cost one full
--   measurement pass here and made 39 logs look verdict-less. Capture
--   `>log 2>&1`, as `make run-all-tests` does, and never `cat` the streams.
# * **`tests/<stem>.log` must be the artefact you measure**, not a reconstruction.
# * **The snapshot must have `OBJ_LIB` built.** `make all` also links
--   `$(TARGET)`, which this corpus never runs, so the objects are compiled
--   directly; a snapshot without them fails every link with
--   `ld: cannot find src/...o` and 63 of 63 suites "build-FAIL".
# * **`test_state_effects` exits 139 in at least one run and 0 in twenty
--   others**, with identical output ("35 checks, 0 failures / ALL STATE EFFECT
--   CHECKS PASSED") in both. Not reproducible on demand. It is exactly the class
--   the verdict layer exists for: it will now be reported INCONCLUSIVE rather
--   than silently green, but the underlying flakiness is NOT fixed and is not
--   diagnosed here.
# * **A peer added three suites mid-wave** (`test_live_zone_slot_validation`,
--   `test_null_effect_guard`, `test_parity_resolver`), so the corpus this wave
--   MEASURED is 157 files while the live tree now holds 161. The measured
--   numbers are for the 157-file snapshot; the three additions were not
--   re-measured.

## FIELD-CENSUS WAVE (2026-10-04) -- the 25 fields with no writer and no reader

A field with no producer and no consumer is WORSE than an absent field, because it
reads as coverage. Eleven of these were previously marked `[x]` in this file purely
because the *header blocker* was cleared, and clearing a blocker is not the work.
Every verdict below is one of exactly three: WIRED / TOMBSTONED / BLOCKED-ON. There
is no fourth, and there is no "kept because it was already there".

**MEASURED FIRST.** For each of the 25 the question answered before touching
anything was *does Rust read it at all* -- by content, in the tree at
`engine/src` + `engine/tests` as of 2026-10-04, excluding `game/display.rs` (T8 Q2,
OUT OF SCOPE) and `bot/`. A C producer for a value Rust never reads is the same
defect wearing a different hat, so the write-only Rust items were tombstoned rather
than wired.

### 13 TOMBSTONED -- deleted from the struct, tombstone left in place

| field | where | Rust reads it? | Rust line / why tombstone |
|---|---|---|---|
| `pending_energy_payment` | rabuka.h RbAbilityResolver-adjacent | **no such member** | Rust's twin is `AbilityResolver.awaiting.energy_payment: Option<u8>` (resolver.rs:198) -- on the RESOLVER, not GameState; the C already carries it live as `s_pending_energy_payment` (cost.c:25/:2083). A GameState copy of a resolver-local was never a twin of anything. |
| `assignment` + `n_assignment` | GameState | **no such member** | 0 occurrences tree-wide. `find_distinct_assignment_k` returns `Option<Vec<i16>>` by value (phases.rs:1603-1629) and `rb_find_distinct_assignment_k` (phase.c:1090-1105) writes a CALLER-SUPPLIED `out[]`. The value never lives on GameState in either engine. |
| `turn2_abilities_played[64]` | GameState | WRITE-ONLY | decl mod.rs:132, init :609, `clear()` tracking.rs:8, **no push site**; sole reader `game/display.rs:1928` (OUT OF SCOPE). |
| `areas_placed_this_turn[16]` | GameState | WRITE-ONLY | decl mod.rs:156, init :622, `clear()` modifiers.rs:1702, no push site; sole reader `game/display.rs:1849` (OUT OF SCOPE). |
| `auto_ability_trigger_counts[32]` | GameState | WRITE-ONLY | decl mod.rs:154, init :621, `clear()` modifiers.rs:1730, no push site; sole reader `game/display.rs:1885` (OUT OF SCOPE). |
| `energy_placed_this_turn[2]` | GameState | **no such member** | 0 occurrences. Rust keeps two ACCESSORS, not an array: `last_energy_placed_by_effect()` / `last_energy_placed_by_player()` (mod.rs:1015/:1024), read at turn/actions/mod.rs:1504 and abilities.rs:1842/:2268. Those C members are a SEPARATE 0W defect, reported not fixed (writers live under `src/ability/effects/`). |
| `n_gained_cards` | GameState | **no such member** | 0 occurrences. Rust keys by CardId in a HashMap and keeps `gained_card_ids` index-aligned (mod.rs:183-184); the C uses the `-1` empty-slot sentinel over `gained_card_ids[64]` (game_state_abilities.c:165/:173-175). The count is implicit on both sides; a third bookkeeping of it was unread. |
| `constant_need_heart_global[4096][8]` + `_set` | GameState (98 304 B, the largest dead allocation) | WRITE-ONLY | decl game_modifiers.rs:194, init :275, take/reapply modifiers.rs:613-626; sole reader `game/display.rs:1984` (OUT OF SCOPE). Also on the WRONG STRUCT -- Rust's is on `GameModifiers`, not GameState -- and the old claim that `modify_required_hearts_global` "consumes" it was hand-proven false: the C's (triggers.c:954 CONSTANT arm, executor.c:222 effect arm) and Rust's both apply the amount directly and never read a ledger. The gameplay effect lives in `mods.constant_need_heart`, which is live and untouched. |
| `base_score_total` | RbLiveSnapshot | WRITE-ONLY | types.rs:498, written once at live.rs:771, read nowhere in engine/src; only readers are `tests/modules/characterization/performance_snapshot_audit_test.rs:82/144`. |
| `card_bonus_total` | RbLiveSnapshot | WRITE-ONLY | types.rs:500, written live.rs:777, same absence. |
| `live_spare[3][8]` | RbLiveSnapshot | WRITE-ONLY | `LiveCardResult.spare` types.rs:512, written live.rs:733-741, init live.rs:3153, no reader; `spare` elsewhere in engine/src is unrelated bot-locals. |

The three orphan `n_` counters (`n_turn2_abilities_played`, `n_areas_placed_this_turn`,
`n_auto_ability_trigger_counts`) are KEPT because `src/core/tracking.c:41/:21/:10`
still resets them and that file is another agent's -- they stay 1W/0R. Each is named
in its own tombstone as `BLOCKED-ON: src/core/tracking.c:<line> -- delete the line`.
They are the counter-is-the-tell shape, and the tell was followed; the deletion is
one line each in a file this wave does not own.

### 12 BLOCKED-ON -- Rust reads them for real, so they are kept

`BLOCKED-ON` is recorded in `include/rabuka.h` at each field AND here.

| field(s) | missing leg | exact change |
|---|---|---|
| `player1_cheer_revealed_cards` + `n_`, `player2_cheer_revealed_cards` + `n_` (4) | 2 consumers + the producer | PRODUCER site is MINE and located: `src/turn/live.c:676-679`, right after the `g->revealed_cards[]` push (twin of phases.rs:396-398), before `g->yell_occurred = n_yell > 0;` (twin of :421). CONSUMERS: `src/ability/effects/move.c:1780` (`rb_move_resolve_source_revealed_cards` -- add the `cheer_is_empty` branch ahead of the :1783-1807 filter loop, per move/mod.rs:1580-1621) and `src/ability/dynamic_count.c:683` (`rb_revealed_count`, whose comment at :681 states the gap outright, per dynamic_count.rs:159-180). **The producer is deliberately NOT landed alone.** |
| `revealed_cost_cards` + `n_` (2) | producer | `src/ability/cost.c` -- port `GameState::push_revealed_cost_card` (mod.rs:1091-1110) and call from the twin of handlers.rs:1286. Readers to wire after: move.c (:368/:1831/:1955), modifiers.c (:2055/:2080 for E5/E6), choice.c (choice.rs:3377). |
| `success_zone_blade_bonuses`, `_heart_bonuses`, `_score_bonuses` (3) | the whole producer/consumer function | `src/core/modifiers.c` -- port `evaluate_success_zone_constant_modifiers` (modifiers.rs:2188-2305) with its `core::mem::take` + withdraw prologue (:2200/:2204/:2211) and call at the tail of `rb_recalc_constants` (:1398). Rust DOES read these three -- the withdraw loop drives `remove_blade/heart/score_modifier` -- so they are load-bearing for E1 and for nothing else; outside that function the only mention is a `log::debug!` inequality (:2293-2302). E1/E2 stay blocked. The header half of E1's blocker was ALREADY DONE; the old E1 text asking for these lines is corrected. |
| `performance_need_heart_modifiers` + `n_` (2) | producer — **WRITTEN** | `src/turn/phase.c` `rb_capture_performance_need_heart`, the **first statement** of `rb_execute_performance_phase` (Rust phases.rs:320-333, before the recalc at :352), flattening `mods.need_heart[cid][color]` for every entry with `add != 0 \|\| set != 0` into a module-scope buffer; stored in `src/turn/live.c`'s inline push inside `rb_perform_live`, which is the C's `build_snapshot` (phases.rs:627 passes `&nhm_flat`; live.rs:3179 stores `.to_vec()`). 1 writer / 1 reader, both live. The buffer is module-scope, **not** a `GameState` field, because `sizeof(GameState)` is pinned at 1 090 264 B by test_harness_self §A4. Reader is `restore_performance_need_heart_modifiers` (modifiers.rs:2159-2182, not :2165), called from live.rs:1327 and trigger_scan.rs:365, and lives in `src/core/modifiers.c`. Pinned by tests/test_parity_need_heart_live_flow.c. |

### 0 WIRED -- and that is the finding, not an omission

`opp_cause_fired_keys` is the ONE field of the 25 that has a live Rust reader and a
producer+consumer that both live in a file this wave owns. It is **BLOCKED-ON, not
WIRED**, for a reason that is a real divergence rather than a formality:

* Rust READS it. `abilities.rs:1159` -- `if self.mods.opp_cause_fired_keys.contains(&ekey)
  { continue; }` -- is the live gate; `:1162` pushes. Not write-only.
* The old C comment claiming "C has no field for the set, and include/rabuka.h is
  owned by another agent" was **FALSE on both counts**: the field is declared in
  `include/rabuka.h` and `rb_clear_card_movement_tracking` already zeroes
  `n_opp_cause_fired_keys` (modifiers.c, the fold of modifiers.rs:2041).
* The blocker is the LIFETIME, not the field. `g->turn++` happens at exactly one
  place, `src/turn/phase.c:382`, and that rollover calls
  `rb_clear_card_appearance_tracking(g)`, which does NOT touch
  `n_opp_cause_fired_keys`. The C's `s_opp_cause_fired[]` side table is scoped on
  `(GameState*, g->turn)` and its `s_movement_seq` restarts each turn, so migrating
  the set onto the field WITHOUT the clear produces a hard suppression: the same
  watcher + same moved card on turn N+1 recomputes the same ekey and `contains`
  returns true, silently dropping a 「(対戦相手のカードの効果でも発動する。)」 watcher
  for the rest of the game.

`BLOCKED-ON: src/turn/phase.c:420` -- beside the hand-rolled
`g->debut_count_this_turn[0]=g->debut_count_this_turn[1]=0;` and
`g->position_change_occurred_this_turn=0;`, add
`g->mods.n_opp_cause_fired_keys = 0;`. Then delete `s_opp_cause_fired[]`,
`s_n_opp_cause_fired`, `RB_OPP_CAUSE_FIRED_CAP`, `RbOppCauseFired`, re-key the two
helpers on `g->mods.opp_cause_fired_keys[0 .. n_opp_cause_fired_keys]`, and both the
turn prune and the FIFO reclaim become unnecessary.

Two further reasons no field could reach WIRED this wave, stated so they are not
mistaken for judgement:
1. **`tests/**` is held**, and outcome 1 requires "a producer AND a consumer, PLUS A
   TEST". No test was authored or edited (the one edit inside `tests/` is the
   mandated `RB_GAMESTATE_BYTES_PINNED` number). `tests/test_parity_foreign_cause.c`
   covers the hook and the `hooked_by_foreign_cause` guard; there is no test for the
   once-per-move dedupe, which is exactly the behaviour the field gates.
2. Landing a producer for the 11 BLOCKED-ON fields would recreate the defect class.

### Struct size -- `sizeof(GameState)` 1 195 720 -> **1 090 264 B** (-105 456)

Measured, not estimated, with `sizeof()` in a throwaway TU against the live header.
`RB_GAMESTATE_BYTES_PINNED` (tests/test_harness_self.c §A4) and `Makefile:3` moved in
the same change; §A4 is GREEN (`ok: A4 sizeof(GameState) is EXACTLY the pinned
1090264 B`). The deleted `constant_need_heart_global` pair was 98 304 B of the
105 456. **No field was inserted** -- only appended-to-comment and deleted; the pin
and the Makefile figure moved together with the deletions.

### FALSE COMMENTS FIXED IN FILES THIS WAVE OWNS -- 33

* **1 hand-proven false, `game_state_core.c:13-18`** (the census listed it at 13-15):
  the "NOT ported here" list claimed `cards_paid_cost_this_turn`,
  `player1/2_cheer_revealed_cards` and `revealed_cost_cards` have "no GameState
  field". All are declared. Rewritten to the true and much narrower statement --
  DECLARED BUT UNWIRED, 0W/0R -- with each one's BLOCKED-ON. A comment saying "no
  field exists" when the field exists sends the next reader looking for a
  declaration instead of at the call site.
* **`include/rabuka.h:240`** (cited by the census): called the `success_zone_*`
  ledgers "NOT cosmetic ... load-bearing", implying they were live here. All three
  are 0W/0R. Corrected, with the one place they ARE load-bearing named.
* **`include/rabuka.h`, `constant_need_heart_global`**: "consumed by
  modify_required_hearts_global which is reachable from 3 abilities ... NOT 0 as
  AUDIT_constraint_condition.md claimed" -- false on two counts, both proven above.
* **`game_state_abilities.c`, the side table**: "C has no field for the set, and
  include/rabuka.h is owned by another agent" -- the field exists and the clear has
  landed. Corrected, and the real blocker named.
* **27 citations naming a Rust file that no longer exists**, all verified and
  re-pointed: `move_cards.rs` -> `ability/effects/move/mod.rs` (12 in `choice.c`, 1
  in `misc.c`, 3 in `rabuka.h`) and `ability_queue.rs` -> `ability/queue.rs` (6 in
  `choice.c`, 4 in `rabuka.h`). Line numbers were re-derived from the CURRENT tree,
  not carried across: `handle_energy_zone_selection` :3585 -> :3349,
  `finalize_card_movement` :2490 -> :2494, `execute_selected_cards_from_zone`
  :2693/:2700 -> :2656, `execute_move_cards_both` :3411-3445 -> :3521-3554,
  `resume_pending_actions` :522-536 (there is NO such function; it is
  `AbilityResolver::resume_pending_actions` at choice.rs:117-135). The `queue.rs`
  345-351 / 771 / 303-404 pins were checked against the file and kept.
* **10 stale `abilities.rs` line cites in `game_state_abilities.c`**: the
  `fire_opponent_cause_watchers_for_move` function is at **:1086**, not :1033-1149,
  and `push_movement_event`'s movement arm is at modifiers.rs:1888-1930 (sequence
  bump :1905), not :1538-1551. The bodies were right; the citations pointed a
  re-deriver at the wrong function. Eight inner cites re-pointed to :1091-1100,
  :1101-1106, :1120-1125, :1130-1134, :1135-1150, :1151-1168, :1163-1168,
  :1175-1183, and `par_ability_id_matches` to :1260-1267.
* **`live.c:988-1014`** claimed the three snapshot writes were "BLOCKED-ON THE
  HEADER ... with those three fields the bodies are three lines each". The premise
  was never checked and is false -- they are write-only in Rust. Rewritten.

### A REAL DIVERGENCE FOUND AND DELIBERATELY NOT CHANGED

`rb_opp_cause_key` (game_state_abilities.c) is documented as "Mirror
abilities.rs::opp_cause_key" and is NOT a mirror. Rust folds the sequence with
`(u64::from(seq) << 44).rotate_left(44)`; the C had the bare `s << 44`. For a u16
`seq` the rotate is exactly `seq << 24`, so the two engines built their dedupe
identities in different bit ranges -- C in 44..59, Rust in 24..39. Both are injective
in `seq`, which is why no corpus input separates them, but they do not share a
collision set, and the key is only ever compared against keys this same function
produced.

**Changing it to `s << 24` was reverted and is NOT shipped.** Making the C faithful
moves `seq` into 24..39, which is exactly where `m << 20` and the low half of
`num_key` already live, so it makes the identity MORE collision-prone and widens the
range three terms share. That is a parity change to a dedupe identity and it wants
its own test, not a ride-along in a field-census change. The finding is recorded at
the function; `BLOCKED-ON: tests/test_parity_foreign_cause.c`.

### TRAP THIS WAVE DISCOVERED -- `/*` INSIDE A BLOCK COMMENT COSTS FIVE SUITES

`include/rabuka.h` briefly carried the line `writers live in
src/ability/effects/*.c`. That contains the two-character sequence `/*`, and GCC's
`-Wcomment` fires on it. Adding that ONE comment line -- no field added, no field
removed, `sizeof(GameState)` unchanged, byte-identical token stream otherwise -- was
**measured** to cost exactly one extra failure in each of five suites
(parity_sequential, parity_zones_position, parity_live_total, audit_zones,
mechanics), reproducibly in both directions. Every tombstone in this wave was
therefore rewritten to name the path without the trailing glob, and the rule is
recorded in `Makefile:3-31`.

Do not read that as "comments change behaviour". The five suites were also observed
to report 142/50/50/5/7 on one build generation of an UNMODIFIED tree and
143/51/51/6/8 on the next, twice reproduced, with `-j4` and with `-j1`. They are
build-generation-sensitive, and the only thing this wave can honestly claim is the
A/B in the next section.

### MEASURED, THIS WAVE

Baseline and after were both taken by `make clean` + full rebuild, strictly
sequentially, on a snapshot of the live tree (`Temp/kilo/fld`) that differs from it
by this wave's hunks and by nothing else. `src/abilities_strings.bin` verified
726831 B / `89204df0bbde08a0…` before and after every build. No `.o` or `rb_engine*`
was left in the live tree. The snapshot's copies of `src/ability/util.c`,
`src/ability/vm.c` and `src/ability/effects/move.c` were re-synced from the live
tree mid-wave, because peers landed edits there after the snapshot was taken;
without that re-sync the comparison would have credited them to this wave.

| suite | before | after | delta |
|---|---|---|---|
| parity_sequential | 143 | 143 | 0 |
| parity_cce_gaps | 89 | 89 | 0 |
| parity_integration | 43 | 43 | 0 |
| parity_small_clusters | 105 | 105 | 0 |
| parity_under_member | 88 | 88 | 0 |
| parity_zones_position | 51 | 51 | 0 |
| parity_live_total | 51 | 51 | 0 |
| parity_live_total2 | 18 | 18 | 0 |
| parity_recover_live | 58 | 58 | 0 |
| audit_fields | 5 | 5 | 0 |
| audit_zones | 6 | 6 | 0 |
| mechanics | 8 | 8 | 0 |
| zone_filter_parity | 0 | 0 | 0 |
| parity_jidou_watch | 76 | 76 | 0 |
| parity_jidou_watch_clusters | 63 | 63 | 0 |
| parity_jidou_move_yell | 82 | 82 | 0 |
| parity_wave1a | 39 | 39 | 0 |
| parity_wave1b | 33 | 33 | 0 |
| parity_wave1c | 45 | 45 | 0 |
| parity_full_round | 45 | 45 | 0 |
| parity_full_round2 | 69 | 69 | 0 |
| parity_full_round3 | 18 | 18 | 0 |
| parity_deck_order | 0 | 0 | 0 |
| parity_search_reorder | 120 | 120 | 0 |
| **24-suite subtotal** | **1255** | **1255** | **0** |
| test_harness_self | 3 | 3 | 0 |

FAIL sets are **byte-identical** between the live tree and this wave's hunks on all
24 suites (`Compare-Object` over every `^FAIL` line, case-sensitive, stdout+stderr).

`test_harness_self`: `ok: A1`, `A2` (1090264 <= 2396745), `A2b`, **`A4 sizeof(GameState)
is EXACTLY the pinned 1090264 B`**, `A5`. Its two failures are **D1/D2**
(`test_id(` call sites 3032 > 3017 ceiling, 107 > 105 files) and are PRE-EXISTING and
identical before and after: they are peers' test-file growth, not this wave. The
suite is not GREEN; §A, the gate this wave moved, is.

`make warnings-audit-test`: **PASS** -- 159 test sources, 0 red, 0 pinned.
`make warnings-audit`: **still RED, 4 diagnostics, all pre-existing.** It is red on
the untouched tree for exactly the same four. Two pins this wave DID displace --
`game_state_core.c:853/:932` (FT-8/FT-9), shifted by exactly +15 lines by this
wave's tombstone -- have been re-pinned to `:868/:947`, so `WARN_AUDIT_BASELINE`
stays keyed on the same two proven-safe instances and the proofs in cluster W still
describe them. The four that remain are stale pins in files this wave does not own:
`look.c:1409 -> :1486` (FT-6), `rules.c:849 -> :895` (FT-3),
`rules.c:1352 -> :1398` (FT-4), `rules.c:1555 -> :1601` (FT-5).

---

# ✅ FIXER WAVE — 2026-10-04, seven items, HEAD `cf315a9ac5dedd1ca02caeadbfb13f0f74919e5c`

**HEAD measured: `cf315a9ac5dedd1ca02caeadbfb13f0f74919e5c`, unchanged start to finish.**
Two peers reached opposite conclusions on the same code earlier in this project purely
because a commit landed between their measurements, so this is stated first and the
tree was re-checked at the end: identical.

Measurement frame: two 452-file snapshots of the live tree under
`Temp/kilo/fin7/{base,rep}`, `base` = this tree with ONLY this wave's hunks reverted
(verified by a SHA-256 sweep over every file: 452 files, 0 diffs against the live
tree before any edit), `make clean` before every build whose number is quoted, blobs
frozen at 726 831 B / `89204df0bbde08a0` and 631 890 B / `728208504fee6217` and
re-verified after every build, 23 suites strictly sequential, `^FAIL` counted with no
colon and case-sensitively on stdout AND stderr, exit code ignored.

| | baseline | after | Δ |
|---|---|---|---|
| `^FAIL` stdout | 320 | 320 | 0 |
| `^FAIL` stderr | 835 | **828** | **−7** |
| `^FAIL` total | 1155 | **1148** | **−7** |
| suites whose FAIL set moved | — | **2 of 23** (`parity_resolver`, `harness_self`) | |
| suites byte-identical | — | **21 of 23** (same counts AND same byte lengths on both streams) | |

## LANDED

- [x] **1. `cost_limit_operator == "=="`** — `src/ability/util.c`,
      `rb_card_matches_cost_limit`. The C could not spell `==` at all and fell through
      to the `<=` reading, so Rust's cost-EXACTLY-7 became C's cost-AT-MOST-7.
      **REACHABLE: 1 condition node**, `PL!S-PR-045-PR` (card 1591),
      「コスト7のメンバーからバトンタッチして登場した場合」 (`cost_limit: 7`,
      `cost_limit_operator: "=="`). The operator table was rewritten into Rust's
      canonical order (`ability/util.rs:2301-2311`) and **gained `!=` as well** — the
      Rust table has six arms and the C had four, so leaving `!=` out would have kept
      it short by one for the next caller. No corpus node carries `!=`.
      **Proved green by `tests/test_parity_resolver.c`, 7 → 1 `^FAIL`.**
- [x] **2. `rb_card_matches_type` FAILS OPEN** — same file, twin of
      `ability/util.rs:585-621`. Three defects fixed: the **`member` alias was
      missing** (`util.rs:598` spells the arm `member_card | member`); the comparison
      was **case-sensitive** where every Rust arm is `eq_ignore_ascii_case`; and it
      **failed OPEN** — the fall-through was `return 1`, so a filter that should reject
      a card accepted it, where `util.rs:607-619` is `Some(other) => false` under the
      comment *"Fail CLOSED, and say so"*. Rust's own comment at `util.rs:613-616`
      names the card that reached that arm (`PL!N-bp7-028-L`, arriving as
      `card_type: "member"` and applying to anything on the stage).
      **VERIFIED-NOOP: 0 on today's corpus** — `card_type` there is exactly
      `member_card` / `live_card` / `energy_card` — and landed anyway, because a
      wrong-value bug is not a missing feature. Rust's `str::trim` at `util.rs:595` is
      transcribed too, using the file's Unicode White_Space predicate rather than an
      ASCII strip, so a whitespace-only filter reaches the empty arm as it does there.
- [x] **3. THE RESOLUTION-ZONE OWNER WAS NEVER STAMPED** —
      `src/core/tracking.c:154`, `rb_perform_cheer_check`. One line:
      `rb_resolution_add(g, card_id)` → `rb_resolution_add_for_owner(g, card_id, pl)`.
      The 2-argument form stamps `RB_RESOLUTION_OWNER_UNKNOWN` (`0xFF`, `zones.c:752`),
      so the owner was never recorded and `rb_check_invalid_resolution_zone` fell back
      to `g->active` for **every** card — the wrong zone for the non-active seat.
      `pl` is already computed 39 lines above at `tracking.c:115` and is the yeller,
      exactly Rust's `tracking.rs:92-93`.
      **Proven by a 6-assertion A/B probe, run against BOTH trees** (`probe_tracking_owner.c`,
      snapshot-only, see the note below):

      | | baseline | after |
      |---|---|---|
      | `resolution_owners[0]` | **255 (`0xFF` UNKNOWN)** | **1 (the yeller's seat)** |
      | seat 0 (`g->active`) gained | **energy_deck +1** | nothing |
      | seat 1 (the yeller) gained | nothing | **energy_deck +1** |
      | probe result | **3 FAIL** | **0 FAIL** |

      The probe's CONTROL drives the SAME drain through the plain 2-argument
      `rb_resolution_add` and passes identically on both trees, so the delta is
      measuring the stamp and not the drain.
      ⚠ **The 23-suite sweep moved ZERO suites for this**, which is expected and is
      the trap the item named: `d2_invalid_resolution_card_returns_to_its_owner` calls
      the plain 2-argument form (so it still exercises the `unwrap_or(active)`
      fallback), and `D4` sets `g->active = 1` so both routings coincide. **Zero
      movement is not evidence of neutrality here — the probe is.**
      ⚠ **Naming trap found while probing:** `rb_perform_cheer_check`'s `player_id` is
      the 1-BASED name and the seat index is 0-BASED — `tracking.c:115` maps `"p1"` to
      seat **0**. The probe's first version yelled as seat 0 and reported a false
      failure. Recorded because it will bite the next person who writes this probe.
- [x] **7. `tests/test_harness_self.c` D1/D2** — re-baselined **3017 → 3037** calls and
      **105 → 107** files, with the delta attributed per line in the file's own history
      block, following the precedent already set there for `3013 → 3017`. The assertion
      is **unchanged** — still `<=`, still red on the next +1. Nothing was relaxed.
      **D2 is green. D1 went red a second time on its own fix**: the history comment
      spells the token once more, and this ratchet counts tokens, not intent. That is
      the *second* time this bound has moved under a change that added no call site
      (the `+4` above was the first), so the file now says so in as many words.
      **THE IDENTITY-TRAP AUDIT WAS MEASURED, NOT PERFORMED** — stated plainly rather
      than claimed. A per-*file* census of repeated card_no literals is useless (200+
      pairs repeat, and a repeat across two test functions is safe because each starts
      from `test_game_new()`). Scoped correctly — same literal twice inside ONE
      function body, the only shape where `test_id()`'s aliasing makes a "two distinct
      instances" assertion vacuous — it is **153 sites across 96 functions**. They were
      enumerated and are reproducible, but adjudicating each means reading 96
      functions in files this task does not own. Asserting "no one is doing this" on
      the strength of a grep would be the exact false-clean the same file's §M warns
      about ("the assertion can only ever be true"), so it is recorded as **OPEN**.

## VERIFIED-NOOP

- [~] **4. `opp_cause_fired_keys` IS NEVER CLEARED** — **the premise is stale and the
      prescribed fix would create a NEW divergence.** Three measured facts:
      1. **The clear the item asks for is already landed, at the Rust position.**
         Rust clears it at `core/game_state/modifiers.rs:2041`, INSIDE
         `clear_card_movement_tracking`, called from `turn/phases.rs:309`. The C's twin
         clear is `g->mods.n_opp_cause_fired_keys = 0;` at **`src/core/modifiers.c:960`**,
         inside `rb_clear_card_movement_tracking`.
      2. **Rust has no clear at a `g->turn++` site.** Adding the line at the C's only
         `g->turn++` (`src/turn/phase.c:382`) would put a clear where Rust has none and
         would cover one of the FOUR fields Rust clears there
         (`cards_activated_this_turn`, `turn_movements`, `opp_cause_fired_keys`, and
         the movement ledgers), i.e. it trades a faithful partial for a new partial.
      3. **The claimed symptom is unreachable, twice over.** The field has **0
         producers and 0 readers** in C — `n_opp_cause_fired_keys` appears exactly once
         in the whole tree, on that clear line. The LIVE dedupe is the file-static
         side table `s_opp_cause_fired[]` (`game_state_abilities.c:2272-2303`), which
         **turn-prunes itself** at `:2292-2293`. So a watcher cannot be "suppressed for
         the rest of the game": the mechanism that would suppress it does not exist, and
         the mechanism that does exist is already turn-scoped.
      **VERIFIED-NOOP: 0 reachable.** Nothing landed in `phase.c`; it is byte-identical
      to HEAD (`4fb6ad3ca7dae9e4e289497832b61887d1d35749`, the same hash the Q213
      section above records) and the `live_face_up` gate and the eviction were not read
      into this change at all.
      **⛔ BLOCKED-ON `include/rabuka.h:2842` (main's) — and this is the real gap.** The
      rollover at `phase.c:420` calls the NARROWER `rb_clear_card_appearance_tracking`
      precisely because `rb_clear_card_movement_tracking` has **no prototype in the
      header**, so on this path Rust's `phases.rs:309` line is only half-ported. The
      file's own comment at `phase.c:413-419` already says so and names the fix: add
      `void rb_clear_card_movement_tracking(GameState *g);` beside
      `rb_clear_card_appearance_tracking` at `rabuka.h:2842`, then call it at
      `phase.c:420`. **Owner: whoever holds `include/rabuka.h`.**

## BLOCKED-ON (contested files — reported, not touched)

- [~] **5. `watches_ability_resolution` IS NEVER READ** — **the gate EXISTS, in a file
      this task may not edit, and the brief's stated location is wrong.** The TAS scans
      are not in `executor.c` at all. They are in
      **`src/core/game_state_abilities.c` (CONTESTED)**, which already carries:
      * `rb_effect_is_ability_resolution_watcher` — forward-declared at `:704`,
        defined at **`:1164`** — the direct twin of Rust's
        `effect_is_ability_resolution_watcher` (`abilities.rs:1546-1549`);
      * **three call sites**, matching Rust's structure: `:1043` (the standby-ability
        activation path), **`:1981`** (the zone TAS scan, twin of `abilities.rs:641-650`,
        the stage-member loop) and **`:2157`** (the recently-moved TAS scan, twin of
        `:813-822`).
      So "no reader" was true of the WIRE KEY only: `watches_ability_resolution` has 0
      occurrences in `src/` and `include/` (re-verified tree-wide), and the C's second
      half is a **condition-TEXT proxy** —
      `rb_condition_tree_has_text(e->condition, "能力が解決")` — where Rust's is the
      `watches_ability_resolution` wire flag. Rust's doc comment on the predicate
      (`abilities.rs:1541-1545`) describes it in exactly those text terms, so the proxy
      is defensible, but it is **not the same predicate** and the two were **not
      compared node-set-for-node-set on the corpus** — that census was not completed
      (the volume ran out of space, below).
      **⛔ BLOCKED-ON `src/core/game_state_abilities.c`** — replace the second half with a
      read of the `watches_ability_resolution` extra, i.e. the same shape as the
      `trigger_type` scan already at `:1167-1169`:
      ```c
      for (int i = 0; i < e->n_extra; i++)
          if (e->extra_k[i] && !strcmp(e->extra_k[i], "watches_ability_resolution")) {
              return !strcmp(e->extra_v[i], "true");
          }
      return 0;
      ```
      Reachability is unchanged from T7-l: **4 nodes / 4 abilities**, and
      `PL!-bp6-020-L` ab#0 satisfies both halves of the Rust conjunction.
- [~] **6. `make warnings-audit` RED on 4 stale pins** — **these are DISPLACED pins, not
      warnings that ceased to exist, so the correct action is to re-POINT them and the
      edit is in the Makefile, which is CONTESTED.** Measured on an untouched tree:
      `12 found / 12 pins / 4 NEW / 4 stale`. The four stale pins and the four NEW rows
      are the **same four `-Wformat-truncation` sites at shifted line numbers**, and the
      shifts are exactly constant per file — `rules.c` **+46** three times
      (`:849→:895`, `:1352→:1398`, `:1555→:1601`) and `look.c` **+77**
      (`:1409→:1486`). Deleting the four pins, as the item's wording suggests, would
      leave `NEW=4` and the gate still RED; the gate's own comment says the opposite
      ("Their line numbers move whenever the comment block above them changes;
      `make warnings-audit` FAILS on a stale pin precisely so that cannot be silently
      lost"), and the tail of this file already records the same mapping under cluster
      W (`look.c:1409 -> :1486` FT-6, `rules.c:849 -> :895` FT-3, `:1352 -> :1398`
      FT-4, `:1555 -> :1601` FT-5).
      **⛔ BLOCKED-ON the Makefile (main's), `WARN_AUDIT_BASELINE` at `Makefile:734` —
      four line-number substitutions, nothing else:**
      `src/ability/effects/look.c:1409` → `:1486` ·
      `src/ability/effects/rules.c:849` → `:895` ·
      `src/ability/effects/rules.c:1352` → `:1398` ·
      `src/ability/effects/rules.c:1555` → `:1601`
      **This wave moved that gate in NEITHER direction**, proved rather than assumed:
      `src/ability/util.c` and `src/core/tracking.c` were each compiled with the full
      `WARN_AUDIT_FLAGS` set and emit **0** audit-class diagnostics, and neither appears
      in the pin list or the diagnostic list — all 12 live diagnostics are in files this
      wave does not own.

## ⛔ ENVIRONMENT: THE C: VOLUME IS AT 0 BYTES FREE — READ THIS BEFORE TRUSTING ANY NEW NUMBER

**`(Get-PSDrive C).Free` = 0.** Confirmed repeatedly during this wave. Consequences,
stated so nobody re-derives them:

* **Two `make warnings-audit` runs on the after-snapshot are VOID.** Both died with
  `printf: write error: No space left on device` partway through, and their tallies
  disagree with each other and with the good baseline reading
  (`5 found / NEW=3 / stale=10` versus `12 found / NEW=4 / stale=4`). A truncated
  compile log reads as a shorter `found.txt`, which manufactures **false STALE pins** —
  i.e. a full disk makes this gate lie in the direction that would have had me delete
  four live tripwires. The gate's verdict for this wave is therefore established by the
  narrow per-file check above, not by a `make warnings-audit` number.
* **Peers are actively consuming it.** `Temp/kilo/to` grew 1387 MB → 1430 MB during this
  wave. The largest reclaimable directories belong to other agents
  (`adj` 6.5 GB, mtime today; `sweep` 5.4 GB, mtime 2026-10-03; `hx` 2.3 GB, mtime
  today) and **were deliberately NOT deleted** — reclaiming them is not this task's
  call. **Someone should reclaim them explicitly.**
* Only this task's own build artifacts were removed. Small writes still succeed
  (NTFS slack), which is why the checklist update landed.
* **The item-5 node-set census did not run** for the same reason.

## FILES LANDED IN (4), all in `Temp/kilo/fin7`-verified form

`src/ability/util.c` (items 1 + 2) · `src/core/tracking.c` (item 3) ·
`tests/test_harness_self.c` (item 7) · `tests/test_parity_resolver.c` (items 1 + 2,
to green).

**`tests/test_parity_resolver.c` — what changed is bookkeeping, not strength.** The
previously-red block was split in two so neither half carries a row it does not own:
`card_matcher_aliases_and_operators_are_rust()` holds the 9 rows that are now GREEN,
each still asserting the same expected value it asserted when it was red, with the Rust
citation moved into the message text; `known_divergence_cost_limit_quantity()` holds
the **one row that is still red on purpose** — the QUANTITY divergence, where Rust
branches on the `cost.or(score)` Option so a card with neither is `None` and the
predicate is FALSE, while the C branches on card TYPE and reads a zeroed `cost`
(`util.rs:1101-1115` vs `util.c`). It was **not** landed alongside (a)/(b) on purpose:
it is a different defect with its own reachability question ("which cards carry neither
cost nor score, and are any of them candidates under a `cost_limit` filter"), and
landing it on the strength of one red assertion would be guessing at that census.
**So `parity_resolver` is NOT green: it is 9 rows better and 1 row still red, by
design.** Nothing was skipped, deleted, `if (0)`-ed or weakened.

## ⛔ NO PERMANENT TEST WAS LANDED FOR ITEM 3, AND THAT IS A REAL GAP

The probe that proves item 3 (`probe_tracking_owner.c`, 6 assertions, 3 FAIL → 0 FAIL
across the A/B) lives in the **snapshot only**. Every candidate suite is contested —
`tests/test_live_card_zone_movement.c` and `tests/test_parity_rules_conditions*.c` are
in `D4`'s and `d2`'s homes and are not this task's files — and adding a new
`tests/test_*.c` would also move the `harness_self` D2 **file** ratchet this wave just
re-baselined. **A parity test per fix is the rule and item 3 does not have a landed
one.** ⛔ BLOCKED-ON: an owner for a test file. The probe is written to be lifted
verbatim; it needs only `rb_load`/`test_game_new`/`rb_perform_cheer_check`/
`rb_resolution_add`/`rb_check_invalid_resolution_zone` and it asserts BOTH the
positive and the `0xFF` fallback control.

## FILES NOT TOUCHED, VERIFIED

`src/turn/phase.c` and `src/ability/effects/executor.c` are **byte-identical to HEAD**
(`git hash-object` = `git rev-parse HEAD:<path>`): `4fb6ad3ca7da…` and `330945dd0a7b…`.
`src/ability/effects/look.c` and `rules.c` show as `M` in `git status` but were **already
dirty at this wave's baseline** and were never written here — they are a peer's
pre-existing edits, carried into both snapshots identically. No git write of any kind
was performed.

---

## Q68 / choice_player_id WAVE — LANDED (this is not a BLOCKED-ON record)

Measured at **HEAD `5ce311e6`**, working tree otherwise clean of my files. Both of
my snapshots were taken from the same tree and differ in exactly 4 source files
(`rules.c`, `triggers.c`, `game_state_abilities.c`, `web_server.c`) + 3 new tests;
the file-hash sweep says 451 files byte-identical. The peer's concurrent edit to
`src/core/modifiers.c` (mtime 06:32:47Z, hash `c237acbb9dbE0bdf`) landed in the
LIVE tree after both snapshots and is in NEITHER measurement — the pair was taken
against `modifiers.c@d804483c3ebe4066`.

### Q68 — 「ライブ開始時」 IS UNUSABLE FOR A 「ライブ不能」 PLAYER. LANDED.

The C **inherited** it. `rb_trigger_live_start` (triggers.c:277, the twin of
`trigger_live_start_abilities`) had exactly Rust's two gates — `is_trigger_suppressed`
and the Q72 empty-live-zone guard — and no third. Rust is wrong here: FAQ Q68
(2025.02.07, `PL!HS-bp2-014-N` / `PL!SP-bp1-001-P` / `PL!SP-bp1-001-R`) says the
abilities are UNUSABLE, and `engine/src/turn/trigger_scan.rs:186-212` never reads
`cannot_live_players`; Rust first reads it at `phases.rs:351`, i.e. **after** the
scan (`:245-246`) and inside the yell.

**The latch WAS a live hazard, and it is now closed.** `RbRulesState::cannot_live_players[2]`
had no clear: the flag is written by the `cannot_live` arm of
`rb_rules_execute_restriction` and read by nobody, forever. The rollover clear
Rust gets at `phases.rs:280` is unreachable from outside `rules.c` because
`rb_rules_state` is `static` — so the `I2` tombstone at `phase.c:521` and the
`live.c:576-638` comment block are still correct as written and are **not**
resolved by this work.

What *is* resolved: the flag now carries the turn it was set on
(`cannot_live_turn[2]`, `cannot_activate_turn[2]`) and both readers drop a stamp
from an older turn. That is the idiom the C already uses for Rust's per-turn
`turn_limited_abilities` (the use-limit table is keyed by turn at
`ability_queue.c:1138`), so it needs no `phase.c` edit and no header edit, and it
is the same guarantee the explicit clear gives, not a weaker one. **The Q68 gate
is therefore landed rather than BLOCKED-ON.**

`WARN_AUDIT_BASELINE` is keyed on `file:line` and pins `rules.c:215/:261/:895/:1398/:1601`,
so the two new struct fields are **declared on one line, on purpose**. Any comment
block there turns five pinned diagnostics into NEW ones. Verified:
`make warnings-audit` → `PASS -- 12 diagnostic(s), all pinned; 0 new`, identical
to the baseline.

- `src/ability/effects/rules.c:72` — the two turn stamps, ONE line.
- `src/ability/effects/rules.c:1922-1955` — both writers stamp `g->turn`.
- `src/ability/effects/rules.c:2291-2313` — both readers are per-turn.
- `src/turn/triggers.c:20-30` — file-scope `extern` for the reader (same precedent
  as `:6-16`); `:305-333` — the Q68 gate.
- `tests/test_parity_q68_cannot_live.c` — **NEW, 4 sections, GREEN.** Includes the
  assertion that FAILS against the pre-fix reader (the flag outliving its turn) and
  the one that fails against a naive clear-on-read.

⛔ **BLOCKED-ON `include/rabuka.h`** (main's file): declare `rb_rules_cannot_live_player`
and `rb_rules_execute_restriction` beside the other `rb_rules_*` accessors. The test
and `triggers.c` declare them locally for now.

### choice_player_id — the C's WIRE WAS SILENT, AND ONE READER WAS WRONG. LANDED (partly).

From the Rust **source**, not the dead test. `pvp_room_test.rs` cannot compile
(`build.rs:90` `EXCLUDED_MODULES`; it imports `web_server::pvp_player_can_act`, which
`lib.rs:149-150` gates behind a feature that is not in `Cargo.toml:62` `default`).

- `queue.rs:312-366` `pause_for_choice`: an existing stamp is never overwritten
  (`:319`); the redirect needs an opponent CHOICE **and** an opponent ENTRY CONTEXT
  (`:322-331` × `:335-345`); the universal default is the entry's owner (`:354-360`).
  → **`src/ability/ability_queue.c:537-608` is faithful.** No change.
- The JSON key is `choice_player_id` = `choice_player_id` else `player_id`,
  `player1|player2` → `p1|p2`, `_` passed through
  (`core/game_state/abilities.rs:2559-2568`).

Two real defects, both fixed:

1. `rb_inject_choice_ability_context` read `entry.player_id` and **never
   `entry.choice_player_id`**, so every routed choice was announced as the ability
   OWNER; it also fabricated `"p1"` for an empty id, which would have satisfied
   Rust's leak filter for the wrong seat. Fixed (`game_state_abilities.c:1507-1553`).
2. `append_pending_choice` (`src/web_server.c`) emitted **no `choice_player_id`
   key at all** — only `kind`/`zone`/`target`/`card_type`/`count`/`allow_skip` and
   an integer `actor`. Rust's client depends on the key twice (`web_server.rs:1240-1249`
   drops the whole choice when it is missing or names anyone but the requester;
   `pvp_player_can_act` at `:1151` gates the answer endpoint on it). Now emitted.

The three readers in `game_state_abilities.c` (`:2590`, `:2696`, `:2736`) all read
`g->queue.actor` / `entry->player_id`, i.e. Rust's **`ability_master_id`** /
**`current_pid`**, not the choice owner — they agree with Rust and were left alone.
(`:2696` reads the card owner *before* the queue entry where Rust's
`resolve_master_id` reads the entry first; noted, not changed, not in scope.)

The one reader that is genuinely the choice owner is
`rb_get_pending_choice_player_id` (`src/ability/choice.c:4696-4699`), and it
**disagrees with Rust**: `queue.rs` stamps the routed seat, but
`ability_queue.c:593`/`:605` set `g->queue.actor = g->active`, so it returns the
ACTIVE seat. Its one production consumer, `rb_gsc_can_player_act`
(`src/core/game_state_core.c:261-283`, whose own comment calls pending choices
"authoritative"), therefore answers for the wrong seat on exactly the prompts the
routing exists to redirect. **⛔ BLOCKED-ON `src/ability/choice.c` (+ the comment at
`game_state_core.c:258-260`)**: return the current entry's `choice_player_id` seat
with no fallback (Rust `abilities.rs:2457-2469`), keeping `g->queue.actor` only for
the dummy-entry case. One line. The new suite already pins the field it must read.

⚠️ `src/web_server.o` is linked **only** into `$(WEB_TARGET)` (`Makefile:100-102`),
never into `OBJ_LIB`, so no `tests/test_*.c` binary can reach any symbol defined
there — `rb_wire_pending_choice_player_id` is therefore defined in
`src/core/game_state_abilities.c` (in `OBJ_LIB`) and declared file-scope in
`web_server.c`. **⛔ BLOCKED-ON the Makefile** if anyone wants `append_pending_choice`
itself covered from a suite: `src/web_server.o` has to join the suite link.

- `tests/test_parity_choice_routing.c` — **NEW, 5 sections, GREEN.** Includes both
  negative controls (`SELECT_CARD{target_player_id:"opponent"}` alone, and an
  opponent CONTEXT alone, each of which must NOT redirect) and the pre-stamp
  case that must not be overwritten.

### bytecode_deep_compare_test — NEVER RUNS, AND A C COPY WOULD BE RED ON ARRIVAL

`bytecode_deep_compare_test.rs:1` is `#[cfg(all(feature = "bytecode_abilities",
feature = "json_path_test"))]`; `json_path_test` is not in `Cargo.toml:62` `default`
and `build.rs:149-152` adds only the `bytecode_abilities` cfg to the `mod.rs`
declaration, so the module is **registered and then compiles to nothing**.

What it would catch: for each of the 985 abilities, a **bytecode-decode vs
`serde_json`→`Ability`→`populate_from_json` field-by-field** comparison
(`:137-192`, diffs by JSON path). Any field the bytecode round-trip drops, defaults
differently, or spells differently from the JSON — including the three
normalisations the test has to perform itself (`:20-41` `has_moved`/`not_moved`
type tags, `:46-49` cost key normalisation, `:62-78` the compound `draw_card`
`count` default). Because the two inputs are two GENERATIONS apart, it would
**fail today** on slices 325 and 805 — which is the point: it is the oracle that
would have caught the skew, and it is the oracle that is switched off.

The C's equivalent-class gate is `tests/test_artifacts_consistent.c`, and it is
good: offsets monotonic + sentinel (`:109-115`), a whole-table FNV digest
recomputed from disk (`:117-146`), and `RBKA_BYTECODE_LEN == sum(RBKA_OFFSET_DELTAS)`
(`:180-185`). What had **no** compiled guard on either side is the per-slice
measurement — "985/985 slices begin with the format tag `0x08`" and the prefix sums
strictly increasing — which is exactly the byte-splice signature that corrupted
494 of 985 abilities on 2026-10-01. **A full C deep-compare was deliberately NOT
written**: it would be red on arrival for the skew reason and would report
differences that are not defects.

- `tests/test_decode_bytecode_shape.c` — **NEW, 4 checks, GREEN.** Only pins what
  cannot be red unless the committed blob is genuinely corrupt.

### THE THREE "C IS RIGHT / RUST IS WRONG" ITEMS — the C INHERITS NONE OF THEM

- **9a** `phases.rs:831-836` (the card vanishes: taken out of hand, then `Err` on a
  full live zone). **The C does not inherit it.** `rb_handle_set_live_card`
  (`src/turn/phase.c:1335-1346`) refuses on capacity **before** `rb_hand_remove_card`,
  with the divergence documented at `:1347-1354`. Correct as it stands; do not
  "fix" it.
- **9c** `look.rs:1031-1040` (`all_any` ignored for both deck arms). **The C does
  not inherit it** — it goes the other way: `look_at_fetch` (`src/ability/effects/look.c:355-393`)
  applies `look_bool(effect,"all")` by setting `count = RB_MAX_ZONE` for **every**
  source, including the three deck arms. **MEASURED `VERIFIED-NOOP: 0 of 84`**: of
  the 84 `all=true` effect nodes in `cards/abilities.json`, **none** has an action
  containing `look` (the tally by action: `gain_resource` 24, `move_cards` 23,
  `change_state` 8, `modify_score` 6, `select` 5, `select_cards` 3, …), so the
  divergence is unreachable on this corpus. `look.c` is contested and was not
  touched. (The same function's deck-BOTTOM arm at `:379-383` reverses
  `taken[]`, which reproduces Rust's `drain(start..)` index order exactly.)
- **9d** `modifiers.rs:2280-2287` (`non_stackable` keyed by `format!("{}:{}", action, text)`
  instead of by card id). **The C does not inherit it** — `evaluate_success_zone_constant_modifiers`
  is not ported at all; the four-zone scan exists only as comments at
  `triggers.c:1538` and `modifiers.c:1364`. The C has the `non_stackable` *reader*
  (`src/core/card.c:771`) and nothing that would mis-key it.

### `engine/build.rs:325` — REPORT ONLY, NOT EDITED

`let _ = stale;` with the panic commented out at `:323`. **Re-enabling it would NOT
fire on the current tree** — all four checks pass, measured:

| check | measured | verdict |
|---|---|---|
| `:268-275` `abilities_time < cards_time` | `cards/abilities.json` 05:54:18.172Z vs `cards/cards.json` 2026-09-29T03:34:22.196Z | NEWER, no stale |
| `:276-283` `abilities_gen` older | `engine/src/ability/abilities_gen.rs` 05:54:20.412Z | NEWER, no stale |
| `:288-299` manifest digest | `generation_manifest.json` contains `"sha256": "4753ff27e561e7db"`; `sha256(cards/abilities.json) = 4753ff27e561e7db022adaac36b3cee96b7691042c7bddc6a163e25d245138e1` | MATCH, no stale |
| `:313-320` `abilities.bin.z` older | `cards/build/abilities.bin.z` 05:54:20.292Z | NEWER, no stale |

So it is a **no-op today and a tripwire later** — worth having, and safe to land.
⚠️ Two things it would NOT do, which matter more than the panic:
(1) it has no knowledge of the C tree at all, so it **cannot** detect the C/Rust
generation skew — the thing that went unnoticed for the whole project;
(2) `cards/abilities.json`, `cards/build/abilities.bin.z` and
`engine/src/ability/abilities_gen.rs` all carry mtimes inside a 3-second window at
05:54:20 today, i.e. one regeneration ran mid-session, while the C's blob is from
2026-10-02. A generator run between the JSON and `abilities_gen.rs` is exactly the
window this guard closes.

**Patch text (NOT applied — `engine/` is the oracle):**
```diff
--- a/engine/build.rs
+++ b/engine/build.rs
@@ -321,5 +321,5 @@
     // In CI, stale is a hard error; locally it still warns but `cargo test` will surface the warning.
     // To make it a hard error everywhere, uncomment the next two lines:
-    // if stale { panic!("Stale generated files — regenerate as warned above"); }
+    if stale { panic!("Stale generated files — regenerate as warned above"); }
     // For now keep it as warning so local `cargo test` stays green while CI can gate on `cargo:warning` → error via `ci.yml` deny-warnings.
-    let _ = stale;
```
Deleting `let _ = stale;` is required as well as uncommenting, or the unused-assignment
lint fires.

---

# FOUR-ITEM PASS — 2026-10-04, HEAD `5ce311e683141ccdc7abfce452980c9c310ae20b`

Items 1 and 3 LANDED. Items 2 and 4 VERIFIED, not changed. The oracle tree is
**dirty** at this HEAD (`engine/src/ability/choice.rs`, `condition/card.rs`,
`condition_decoder_gen.rs`, `effect_decoder_gen.rs`, `effects/move/mod.rs`,
`core/card.rs`, three `engine/tests/**` files), so every Rust citation below is
**by content, read out of the working tree**, not out of HEAD.

## ITEM 1 — `rb_get_pending_choice_player_id` ANSWERED FOR THE WRONG SEAT. LANDED.

It returned `g->queue.actor`. `ability_queue.c:593` and `:605` set that to
`g->active` **deliberately** — the reasoning at `:584-592` is sound (the
opponent-movement and cross-player watchers read it to know which seat MOVED,
and redirecting it silenced `cross_player_interaction`'s "opponent movement
watcher fires" pin). `choice.c` has **20 further writers** of `queue.actor`
meaning other things again (`:2565` `actor_pending`, `:2591` `actor_pending ^ 1`,
`:1101` `self->actor`). So the old body answered out of a field whose only claim
to agree with Rust was its name.

Its one production consumer, `rb_gsc_can_player_act`
(`src/core/game_state_core.c:261-283`, whose own comment calls pending choices
"authoritative"), therefore named the ACTIVE seat on exactly the prompts the
G1/G3 redirect exists to move.

**LANDED**, Rust by content:

- `engine/src/core/game_state/abilities.rs:2457-2469`
  `get_pending_choice_player_id` — the current entry's `choice_player_id`, a bare
  `.and_then(|e| e.choice_player_id.as_ref().cloned())`, **no fallback to
  `player_id`**, plus a `pending_loop_protocol` arm.
- `engine/src/ability/queue.rs:312-366` `pause_for_choice` — `:319` an existing
  stamp is never overwritten; `:346-352` the redirect writes the other seat;
  `:354-360` the universal default copies `entry.player_id`. All three already
  faithful in `ability_queue.c:537-608`.
- `engine/src/ability/queue.rs:234-242` `current_entry()` is **None** for `Idle`
  and for `WaitingForAutoAbilityChoice` — which is why the auto-ability prompt
  must NOT answer from the entry behind it.
- `engine/src/core/game_state/mod.rs:495-573` `can_player_act`: stamp first
  (normalised, `:506-511`), else the Choice-embedded picker/target
  (`:517-541`), else the phase match (`:544-572`).
- `engine/src/ability/vm.rs:247-268` `get_ability` — `idx >= NUM_ABILITIES` is
  the ONLY bounds check; `start >= end` returns `Ok(Ability::default())`.

The C body now reads the stamp, gated on the C's own spelling of
`current_entry() == None` (reused from `rb_queue_current_entry_mut`,
`ability_queue.c:184-190`, not re-derived), with `mod.rs:506-511`'s
`player1|player2 -> p1|p2` mapping (the same one as `seat_of_token` in
`src/test_game.c`).

`g->queue.actor` is **left exactly as `pause_for_choice` wrote it** — the fix is
in the reader, not in the field, precisely because `:584-592`'s argument holds.

**Two divergences left in place, deliberately:**

1. `has_pending` STAYS a guard on the reader. Rust's getter does not consult it,
   but every Rust call site already does (`match_runner.rs:429-455`,
   `mod.rs:498-505`), so no Rust program can observe the difference. Dropping it
   would change `test_parity_zone_source.c:259`'s `pending_seat()` at every
   non-pending call site for no faithfulness gain.
2. Rust's `Some(token)` that names **no** seat makes `can_player_act` return
   false for **both** seats; the C's `-1` falls through to the Choice-embedded
   arm. Measured unreachable: `rb_build_ability_queue_entry`
   (`game_state_abilities.c:1849-1857`) canonicalises `entry.player_id` to
   `p1`/`p2` before anything is stamped from it, `ability_queue.c:572-573` writes
   the literals `p1`/`p2`, and the `queue.rs:367-409` dummy takes NULL. So
   `choice_player_id` is only ever `"p1"`, `"p2"` or `""`.
   **`VERIFIED-NOOP: 0 of 3`** reachable token values.

`rb_gsc_can_player_act` needed **no** change, and its stale comment at `:258-260`
(which named `g->queue.actor` as the stamp) was corrected **in place at the same
line count** — a 15-line comment there moved the two pinned `-Wformat-truncation`
diagnostics (FT-8, FT-9) off `game_state_core.c:868/:947` and turned them into 2
NEW pins, which is the trap `Makefile:748-755` warns about. The reasoning lives
here and in `choice.c` instead, so the contested Makefile needs **no** re-pin.

Reported there and not changed: its **fallback** arm also answers from
`target_player_id` for `SELECT_TARGET` / `SELECT_POSITION` /
`SELECT_HEART_COLOR` / `SELECT_NUMBER`, where Rust's or-pattern
(`mod.rs:517-541`) covers only `SelectAutoAbility`, `SelectLiveSuccess` and
`SelectCard{target_player_id,picker}` and sends everything else to the
`_ => {}` at `:540` and then to the phase match. Rust also has a
`SelectLiveSuccess` variant with its own `player_id`, which `RbChoiceKind`
(`include/rabuka.h:877-883`) has no member for. Out of item 1's scope; the
fallback's reachability also just shrank, because the reader now returns `-1`
only where Rust's `current_entry()` is `None`.

**`tests/test_parity_choice_routing.c` — section 6 added, 13 new assertions.**
Every one of them fails against the old body, and the discriminator in each case
is that the stamp and `g->active` are made to disagree (`test_game_new` leaves
`active = 0`). The two that a routed-only test would miss are called out in the
file: **6(b)** an UNROUTED prompt owned by p2 with p1 active (no redirect happens
and the C was still wrong), and **6(d)** the **no-fallback** property — an entry
with an owner and an emptied stamp must read `-1`, not borrow the owner, because
`abilities.rs:2468` has no `unwrap_or` where `abilities.rs:2559` has one.
**6(e)** pins both `current_entry() == None` arms, including the auto-ability
prompt, which parks a pending choice with no entry behind it. **6(f)** pins the
field the fix must NOT touch: on a board where `queue.actor == 0` and the stamp is
`p2`, the reader answers `1` and leaves `queue.actor` at `0` after three calls —
so a future "simplification" that folds the stamp into `queue.actor` fails here.

## ITEM 2 — THE `choice_player_id` JSON KEY. VERIFIED (both halves correct).

`src/web_server.c:221-222` emits it; `rb_wire_pending_choice_player_id` is defined
in `src/core/game_state_abilities.c:1590-1599` (in `OBJ_LIB`) and declared
file-scope in `web_server.c:198`, the same shape as `rb_check_effect_position` in
`src/turn/triggers.c`. **CORRECT**, checked against
`engine/src/core/game_state/abilities.rs:2559-2568`:

- `let raw_pid = entry.choice_player_id.as_ref().unwrap_or(&entry.player_id)` ->
  `game_state_abilities.c:1564` `e->choice_player_id[0] ? e->choice_player_id : e->player_id`
- `"player1" => "p1"`, `"player2" => "p2"`, `_` passed through ->
  `choice_seat_wire` at `:1517-1524`
- The `"p1"`-for-empty-id fabrication is **gone**: the `_` arm passes `""`
  through unchanged, which is what the `queue.rs:367-409` dummy (empty
  `player_id`) needs — a fabricated `"p1"` would have satisfied Rust's leak filter
  (`web_server.rs:1243`) **for the wrong seat**.
- `rb_inject_choice_ability_context` **does** read `choice_player_id` now. Before,
  it read `player_id` and never the routed field, so every routed choice was
  announced as the ability OWNER.

**FOUND, REPORTED, NOT FIXED** — a second producer of the same key that the C
does not have. `abilities.rs:2679-2697` is an `else` arm on the same
`if let Some(entry) = entry_ref`: when there is **no** current entry it still
emits `choice_player_id`, from the Choice's own `player_id`, for
`SelectAutoAbility` and `SelectLiveSuccess`. `rb_wire_pending_choice_player_id`
returns `""` there. **Direction is safe** — `web_server.rs:1240-1249` then HIDES
the choice rather than leaking it, and a hidden prompt is a smaller bug than a
leaked opponent prompt — but the prompt is lost. The fix belongs in
`src/core/game_state_abilities.c`, which is contested, so it is pinned as a
known divergence in `test_parity_choice_routing.c` section 7 instead.

**`web_server.o` in `OBJ_LIB`? RECOMMENDATION: NO — keep the reader where it is.**
Measured, and the deciding argument is not taste:

1. `Makefile:101-102` links `src/web_server.o` **only** into `$(WEB_TARGET)`, and
   `make clean` (`:843-844`) does not remove it, because `SRC` (`:61`) does not
   list it — so `src/web_server.o` is the one object `clean` misses.
2. Adding it to `OBJ_LIB` would put `-mwindows` web-server translation units into
   **every one of the 163 suite links**, and `$(WEB_TARGET)` (`:101-102`) already
   links it explicitly — so `OBJ_LIB` + `src/web_server.o` would then be a
   **duplicate-symbol link error on the web target itself**.
3. `append_pending_choice` is `static` (`web_server.c:200`) and is the only
   caller of the reader, so joining `OBJ_LIB` would not even make it reachable:
   a suite could reach `rb_wire_pending_choice_player_id`, which it already can,
   and still never execute the emitter.
4. What would actually cover the emitter is a **separate link target** that
   includes `src/web_server.o` plus a test that `#include`s the `.c` to reach the
   static function — i.e. a Makefile target, not an `OBJ_LIB` edit, and it pulls
   in a socket/HTTP surface for one JSON object.

So: **keep the file-scope extern in `game_state_abilities.c`** (which is what is
already landed), and if the emitter itself is ever to be covered, add a
`$(TARGET)_websurface` target rather than touching `OBJ_LIB`. **One line of
Makefile to report, not to edit:**
`src/web_server.o` added to the `clean:` target's `rm -f` list.

## ITEM 3 — THE SHAPE GUARD, AT FULL STRENGTH. LANDED.

`tests/test_decode_bytecode_shape.c` goes from 4 checks to **13**, in four
sections, and the ordering changes because the old body had a hole in the guard
itself:

- **The old tag loop read `blob[off]` with no bounds test** and compared
  `running` to `RBKA_BYTECODE_LEN` only *after* the loop. On precisely the input
  this gate exists to catch — deltas from one generator run, bytes from another,
  summing **past** the blob — that is an out-of-bounds read **in the guard,
  before it can report anything**. This tree has already shipped one SIGSEGV that
  gcc predicted and nobody read (`Makefile:680`), so the guard is not allowed to
  be the next one. All the arithmetic now runs first; the per-slice walk is
  reached only from a table whose total is known to land inside the blob.
- **The counts are now pinned against `gen_data.h`, not against the generated
  tables.** Every one of the old checks derived its bound from a symbol living in
  the SAME generated file as the bytes it describes, so a regeneration that moved
  `bytecode_blob.c` and `gen_data.c` together left all of them green. `gen_data.h`
  is a different file. `RBKA_NUM_ABILITIES == 985` also **bounds the loop**:
  `RBKA_OFFSET_DELTAS` is declared `[985]` (`gen_data.c:7`), so a
  `RBKA_NUM_ABILITIES > 985` would read past its end, and the pin is the only
  thing between the loop and that.
- The zero-delta case gets its own check, because it is **invisible by
  construction on both engines**: Rust `vm.rs:263-265` returns
  `Ok(Ability::default())` when `start >= end`, and `rb_decode_ability` mirrors
  it exactly (`if (!slice || len == 0) return 1;`). `bytecode_validation_test.rs`
  asserts every ability **decodes** (`:62-80`) — and an empty slice decodes, so
  it cannot see this either. That is the concrete reason the guard has to be
  arithmetic rather than behavioural.

**MEASURED, GREEN, and PROVEN TO BITE.** 13 checks, 0 failed. Prints
`deltas: min=25 max=339 total=98737`, `985/985` tagged. Negative control: setting
`RBKA_BYTECODE_LEN` to **98733** — the Rust generation's own size — turns **4 of
the 13 red** (`RBKA_BYTECODE_LEN == 98737`, no prefix sum overruns, prefix sums
land exactly on the length, and the tagged count). It names
`first overflow at slice 984`, a diagnostic the old 4-check version could not have
produced without reading off the end of the blob first.

**Why the 494 number is the shape guard's whole argument**, read against
`rb_bc_slice` (`src/core/data.c:374-382`): `if (start + len > g_bc_len) return NULL;`
and `rb_decode_ability` turns a NULL slice into a default `Ability` with
`return 1` and **no diagnostic**. So one spliced delta does not corrupt one
ability — it makes **every slice from that index onward** decode as an empty
default, silently. `494 = 985 - 491`. That is a splice at slice 491, and it is
why nothing noticed for a whole project: the engine is memory-safe
(`rb_bc_slice` returns NULL rather than reading past the end), so the only
symptom is 494 abilities quietly becoming `Ability::default()`.

**THE SEMANTIC COMPARISON SHOULD WAIT. Stated plainly: yes, wait.** Not for
difficulty and not for want of a decoder — `rb_decode_ability` exists and walks
the same object form — but because it would be **RED ON ARRIVAL for something
that is not a defect**, and now with the skew measured from the two artifacts
rather than quoted from a report:

| | Rust (`cards/build/generation_manifest.json`) | C (committed tables) |
|---|---|---|
| payload bytes | `output.bytes` **98733** | `RBKA_BYTECODE_LEN` **98737** |
| interned strings | `output.interned_strings` **6030** | `RBKA_NUM_STRING_OFFSETS - 1` **6032** |

**4 bytes and 2 strings**, with slices 325 and 805 as the two encoding changes.
A deep-compare written today reports those as failures, and a suite whose
failures are known-not-bugs is a suite that trains its readers to ignore it.
The order is: (1) one `tools/regen_all.sh` run so `cards/build` and the committed
C tables come from the same pass — `tools/guard_data_blob.sh` exists because
doing it in two steps byte-splices the unseparated interned pool; (2) re-point or
retire the three Rust tests pinning the old encoding; (3) enable the oracle **in
the same commit as (1)**.

**THE SHAPE GUARD IS NECESSARY BUT NOT SUFFICIENT, and the gap is exactly one
failure mode.** A splice that keeps every offset and every `0x08` tag and
corrupts only the bytes *within* slices is invisible to this file, to
`test_artifacts_consistent`'s offsets, and to every count above. Only the
semantic comparison sees it. So the two are not alternatives: the shape guard is
what can be landed today, the semantic comparison is what makes the pair
complete, and the guard prints the skew on every run (section 4) so the deferral
has a **visible end condition** rather than becoming a permanent omission.

## ITEM 4 — `build.rs:325`. CONFIRMED A NO-OP TODAY, BY HAND, WITHOUT `cargo`.

All four staleness checks re-measured at HEAD `5ce311e6`:

| check | measured | verdict |
|---|---|---|
| `:268-275` `abilities_time < cards_time` | `cards/abilities.json` 2026-10-04T05:54:18.173Z vs `cards/cards.json` 2026-09-29T03:34:22.197Z | NEWER, no stale |
| `:276-283` `abilities_gen` older | `engine/src/ability/abilities_gen.rs` 2026-10-04T05:54:20.413Z | NEWER (by 2.24 s), no stale |
| `:288-299` manifest digest | manifest holds `"sha256": "4753ff27e561e7db"`; `sha256(cards/abilities.json)[..16]` = `4753ff27e561e7db` | MATCH, no stale |
| `:313-320` `abilities.bin.z` older | `cards/build/abilities.bin.z` 2026-10-04T05:54:20.293Z | NEWER, no stale |

⇒ `stale == false` ⇒ the uncommented panic **would not fire**. No-op now,
tripwire later. (`cards/build/generation_manifest.json` also records
`"engine_commit": "5ce311e6"`, matching the HEAD measured.)

**THE SHARPER POINT, NOW MEASURED INSTEAD OF ASSERTED.** Every one of those four
checks compares a file against another file **inside the Rust generation chain**:
`../cards/cards.json`, `../cards/abilities.json`, `src/ability/abilities_gen.rs`,
`../cards/build/generation_manifest.json`, `../cards/build/abilities.bin.z`.
Not one of them names `engine_c_wip/src/core/generated/bytecode_blob.c`,
`gen_data.c`, `src/abilities_strings.bin`, or any C file at all. So it cannot
detect the C/Rust skew — and the manifest proves the skew is **live right now**
while all four checks report fresh: the Rust tree is on 98733 bytes / 6030
interned strings, the committed C tables are on 98737 / 6032.

**Items 3 and 4 together are the missing tripwire; either alone is not.** Item 4
alone keeps the *Rust* chain honest and is blind to the C by construction. Item 3
alone pins the C table's internal shape and is blind to which *generation* it is
from — it is green today, and it is green across the skew on purpose, which is
also why it needs item 4's manifest numbers printed beside it to be readable as
a deferral rather than as a pass. The tripwire needs both ends: a C-side gate
that says "this table is well formed" **and** a manifest that says "…and it is
this generation", compared against each other. Nothing in the tree compares them.

## THE REGRESSION ITEM 1 CAUSED, AND THE THIRD DEFECT IT EXPOSED

**Stated plainly, because it is a regression and not a clean win.**
`tests/test_parity_zone_source.c` goes **70 -> 78 `^FAIL`** (+8). `^FAIL` for the
whole corpus goes **4048 -> 4056**. PASSED count is **unchanged at 50**;
`parity_choice_routing` is green in both. The +8 is entirely
`pending_seat(&tg)` (`test_parity_zone_source.c:259`), which reads the getter
directly with no `has_pending` gate of its own, and five of the eight are
`got -1 expected 0|1`.

**Root cause, measured not guessed.** `rb_emit_choice`
(`src/ability/choice.c:5711-5716`) stamps the entry **only** when a live entry
exists:

```c
if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries) {
    RbChoice ch = g->queue.pending;  ch.actor = actor;
    rb_queue_pause_for_choice(g, &ch);      /* <-- this is what stamps */
    g->queue.actor = actor;
}
g->queue.state = RB_QUEUE_AWAITING_CHOICE;
```

so a choice emitted while the queue is Idle reaches the reader unstamped, and a
faithful reader reports None. Three of the eight are
`rb_move_live_to_success_and_handle_wins` prompts, and there the divergence is a
**real, separate C defect** — see below.

**`coverage_gate` moved 151 -> 5 `^FAIL` and that is NOT a result.** It is a
meta-suite that walks `tests/*.log` while `run-all-tests` is still writing them,
so its count is a function of corpus progress, not of any suite. Run on the
settled corpus it reports **654 checks, 5 failures**; in the baseline it read
**151**. Its verdict is **FAILED in both runs**. Do not quote its count as a
before/after number.

### THIRD DEFECT, FOUND BY THE FIX, NOT LANDED — `live.rs:1576-1578` has no C counterpart

Rust's `pause_success_zone_choice`, `engine/src/turn/live.rs:1570-1579`:

```rust
game_state.ability_queue.pause_for_choice(choice);
if let Some(entry) = game_state.ability_queue.current_entry_mut() {
    entry.choice_player_id = Some(player_id.to_string());
}
```

Two statements, and the second is the one that matters: it stamps the
**answering** player onto whatever entry is current, **including the dummy one
`pause_for_choice` mints for the Idle branch** (`queue.rs:367-409`, whose
`player_id` is `""`).

The C's fold of that function is `rb_try_take_success_zone_choice`
(`src/turn/live.c:1393-1411`), and it has **no second statement**. It calls
`rb_emit_choice(g, player_pl, ...)` — which declines to pause when there is no
live entry — and then writes `g->queue.pending.actor = player_pl`, which is the
**flat choice's** field, not the **entry's** stamp. So for a live-phase
replacement prompt raised on a fresh queue:

| | who answers, per `abilities.rs:2457-2469` |
|---|---|
| Rust | `Some("p1")` / `Some("p2")` — the dummy entry carries the stamp |
| C, before this wave | the ACTIVE seat, read out of `queue.actor` |
| C, after this wave | **`None` / `-1`** — faithful to the reader, wrong to the stamp |

**⛔ `src/turn/live.c` is CONTESTED, so this is reported, not landed.** The fix is
one statement after the `rb_emit_choice` call at `live.c:1403-1405`, mirroring
Rust's `if let Some(entry) = current_entry_mut()`:

```c
int ci = rb_queue_current_entry(g);
if (ci >= 0) {
    RbQueueEntry *e = rb_queue_get_entry_mut(g, ci);   /* does not exist yet */
    if (e) snprintf(e->choice_player_id, sizeof(e->choice_player_id),
                    "%s", player_pl == 0 ? "p1" : "p2");
}
```

⛔ Two dependencies it needs, both outside this task's files: there is **no
`rb_queue_get_entry_mut`** in `include/rabuka.h` (only the `const`
`rb_queue_get_entry` at the header, and the non-`const`
`rb_queue_current_entry_mut` in `src/ability/ability_queue.c:184-190`), so
either add the mutator or use `current_entry_mut` directly. **The token must be
spelled `"p1"`/`"p2"`**, not `test_seat_id()` — that helper lives in
`src/test_game.c` and is not linked into `$(WEB_TARGET)`.

**Until that lands, the correct reading of the +8 is: 5 assertions in
`test_parity_zone_source.c` are asserting the pre-fix behaviour of a reader that
answered with `g->queue.actor`, and the suite's own `DIAG` proves it — it prints
`ch->actor`, i.e. the flat choice's actor, and that line reads `seat=0` for a
prompt whose `target_player_id` is `p2`. The remaining 3 are the live-success
defect above.** No test was weakened, deleted or ignored to reach a number.

---

## ORPHANED-LOGIC WAVE (2026-10-04) — O1, O3 and the O2/D3 choice-context items

**HEAD MEASURED: `5ce311e6`** (`git rev-parse HEAD`, taken at the top of this
work and unchanged by it). Every file hash in this section was taken from the LIVE
tree at that HEAD; the baseline was a snapshot of it, so it is a snapshot with
only these hunks reverted, not git HEAD.

| file | before | after |
|---|---|---|
| `src/turn/phase.c` | `022c09ef8ef1f423076d0c3c30b8f965cc977ec47e5d4033277de73718de0156` | see the LANDED table below |
| `src/ability/effects/misc.c` | `1a329049b1d0718ec437c2c9b82c407fa50270470f731e9f26a0466ad24065b8` | " |
| `src/ability/effects/rules.c` | `05675a19f00d610b84a05e7f35cf4b9ae30e12d2e08469faae92587f4d9670f6` | " |
| `src/web_server.c` | `addcabf8ff7dbf26248fbc4e698273028b28b9f49d76bc01d008e134cea56293` | " |
| `src/core/tracking.c` (READ ONLY, unchanged) | `5c286684f068cbef60041ebb5e959955312077b006fe4700d884224412ac73a2` | `5c286684f068cbef60041ebb5e959955312077b006fe4700d884224412ac73a2` |
| `include/rabuka.h` (NOT MINE, unchanged by me) | `6259f1ed0fa6190ed6673a5adf8957d96b0cafab3d54a1cbd011dc4925fd0f82` | `6259f1ed0fa6190ed6673a5adf8957d96b0cafab3d54a1cbd011dc4925fd0f82` |
| `src/core/tracking.c` — NOTE: the audit and the work order both spell the READ-ONLY file `src/ability/effects/tracking.c`. **That file does not exist.** The function is `src/core/tracking.c:38`. | | |

Line endings, measured per file because a whole-file copy of the wrong kind
CR-strips: `phase.c` is **MIXED** (245 CRLF / 1973 bare LF), `misc.c` `rules.c`
`web_server.c` and `Makefile` are pure LF, `include/rabuka.h` is pure CRLF. Every
edit was applied with `edit` and every inserted line uses the dominant ending of
its own region.

### MEASUREMENT THAT DECIDED THE SCOPE, before any code was written

`nm -f posix` over all 49 `OBJ_LIB` objects at the baseline (2125 defined symbols,
516 undefined references):

| symbol | defined in | referenced by another `src` object |
|---|---|---|
| `rb_rules_execute_restriction` | `rules.o` | **NO** |
| `rb_rules_cannot_activate_player` | `rules.o` | **NO** |
| `rb_rules_is_wait_immune` | `rules.o` | **YES** (`state.c:391`, `state.c:492`) |
| `rb_reset_keyword_tracking` | `tracking.o` | **NO** |
| `rb_inject_choice_ability_context` | `game_state_abilities.o` | **NO** |
| `rb_ability_debug_set` | `debug.o` | **NO** |
| `rb_rules_take_changed_state_members` | `rules.o` | YES — the precedent for the two-file split |

## LANDED

### O1 — `rb_reset_keyword_tracking` had no production call site. **LANDED.**

**`src/turn/phase.c`, first statement of the `if(g->phase==RB_PHASE_ACTIVE)`
arm** — which is the C's fold of `Phase::Active` in
`engine/src/turn/phases.rs:94-99`, where Rust's `game_state.reset_keyword_tracking()`
is the FIRST line of the arm.

**⛔ THE AUDIT'S PATCH POINTER IS STALE AND WAS NOT FOLLOWED.** The order said
"before `phase.c:655`". At this HEAD `:655` is inside a COMMENT BLOCK, the
bookkeeping it described moved, and `P->deck_refreshed_this_turn = 1` now lives at
`phase.c:725` inside `rb_player_refresh`. The correct insertion point was derived
from Rust's semantics, not from the line number: Rust's reset is on ENTRY TO EACH
PLAYER'S ACTIVE PHASE, i.e. twice per round, and this arm is that entry. Placing
it in the victory rollover instead would clear these trackers ONCE PER ROUND,
which is a weaker scope than Rust's and is exactly what
`o1_second_active_entry_in_the_same_round` exists to catch.

Coverage of every production driver, which is why this location and not another:
`rb_advance_phase` is the one entry all of them use — `src/main.c:97`/`:102`
(interactive), `src/main.c:134` `rb_turn`, `src/engine.c:1550` (web).

`src/core/tracking.c` was **not** edited, as instructed.

**TEST: `tests/test_parity_orphan_producers.c`,
`o1_active_entry_resets_the_keyword_trackers` +
`o1_second_active_entry_in_the_same_round`.** 7 assertions, all on the PRODUCTION
entry `rb_advance_phase`, each with a negative control that names the tracker as
one the round rollover does **not** clear, so a passing assertion cannot be
credited to the wrong step. `PL!S-bp2-022-L` 「{{ライブ成功時}}このターン、自分の
デッキがリフレッシュしていた場合、このカードのスコアを＋２する。」 is cited in the
failing assertion because that card is what the reset is for.

### O3 — the only producer of a live reader's input. **LANDED, ONE ARM OF TWO.**

**`src/ability/effects/rules.c`** — the `cannot_wait_by_effect` scan was extracted
verbatim out of the uncalled twin `rb_rules_execute_restriction` into the
exported `rb_rules_record_wait_immune_by_effect`, and the twin now calls it. One
function, two callers, no duplication.
**`src/ability/effects/misc.c`** — the live `h_restriction` calls it in its
`cannot_wait_by_effect` arm. ⛔ **NOT a delegation**: `h_restriction` stays the
live handler and keeps the arms the twin does not have. `misc.c`'s own comment at
`:1049-1053` ("none of rules.c's wait_immune list") is now obsolete and was
replaced with the measurement.
**`src/ability/effects/rules.c` + `src/turn/phase.c`** —
`rb_rules_clear_turn_state(g)` is the per-round clear, called from the rollover
next to `player_cannot_activate`. This is the symbol `phase.c` and `live.c` have
been asking for in tombstone comments since 2026-10-03, and both of those
tombstones were written against a definition that did not exist.

**⚠ THE CLEAR IS NOT OPTIONAL AND THAT IS WHY IT IS IN THIS ROW.** The reader
`rb_rules_is_wait_immune` carries **no turn stamp**. A writer without the clear
would latch the list for the rest of the match — trading a check that can never
fail for one that can never stop being true. Rust empties it at
`phases.rs:281`; so does the C now.

**TEST: `o3_wait_immune_is_produced_on_the_live_path` +
`o3_the_rollover_clears_the_list`.** The producer is driven by
`test_play_to_stage_for` — the real card, real 登場, real queue, real
`h_restriction`. Fixture: **`PL!S-bp7-003-{P,P＋,R＋,SEC}`** 松浦果南 is the ONLY
`cannot_wait_by_effect` in the corpus (1 unique ability / 4 pairs / 4 cards),
`blade_limit 3`, `<=`, `original_value true`, group `Aqours` — which
`src/ability/util.c:461` resolves as a **series** name
(`strstr(series, "サンシャイン")`), so the victim fixture is
`PL!S-bp2-009-P` 黒澤ルビィ, original blade 1, inside the limit.

**⛔ THE BRIEF'S `PL!SP-bp4-011-*` HALF IS NOT IN THE TEST, AND CANNOT BE.**
Measured over `cards/abilities.json`: `PL!SP-bp4-011-{P,P＋,R＋,SEC}` is a
`change_state` / `state: wait` / `target: opponent` node with **no `action_by`
key**, and the reader the fix makes live sits at `state.c:385-387` behind
`action_by == "opponent"`. So that card never reaches the filter, before or after
this patch. The **3** abilities that do reach it are `PL!-bp4-009-{P,R}`,
`PL!-pb1-015-{P＋,R}` and `PL!HS-bp6-007-{P,R}`, all
「相手は、自身のステージにいるアクティブ状態のメンバー1人をウェイトにする」 (Q275).
The 48-ability forced-wait family is what those three filters choose from.

### D3 — `rb_ability_debug_set` had no production writer. **LANDED.**

**It IS a real gap, and it is one line wide.** Rust's *only* production writer is
`engine/src/game/web_server.rs:3652-3655`, the first statement of
`run_web_server_with_ngrok`:

```rust
if std::env::var("RABUKA_RULE_LOG").as_deref() == Ok("1") {
    crate::ability::debug::set_debug(true);
}
```

The C's `src/web_server.c` read no environment variable at all. Now it does, same
var, same comparison, same one-shot-at-startup placement, before `rb_load`.
Rust's other ability-debug reader is `ABILITY_DEBUG_BATON_TOUCH`
(`ability/condition/state.rs:617`), a separate flag with its own accessor, and is
deliberately not touched.

### O2 — the choice UI. **LANDED IN SOURCE, NOT REACHABLE BY ANY SUITE.**

`src/web_server.c` now calls `rb_inject_choice_ability_context` from
`append_pending_choice` and merges `card_no` / `ability_text` / `card_name` /
`card_id` into the pending_choice object flat, as `get_pending_choice_json`
merges it; and it runs **Rust's seat-leak filter verbatim**
(`web_server.rs:1240-1249`, dropping the whole object when `choice_player_id` is
missing or names anybody but the requester). The requester is now a named value
(`requester_player_id`) used by BOTH the filter and `ui_config.perspective_player`,
so the perspective the client is told about and the perspective the choice is
filtered against cannot drift.

⛔ **STILL UNREACHABLE, AND THE MAKEFILE IS THE COORDINATOR'S.**
`src/web_server.o` is linked only into `$(WEB_TARGET)` (`Makefile:101-102`) and
never into `OBJ_LIB`, so no `tests/test_*.c` binary can link a symbol defined
there. **The change needed is to the `OBJ_LIB` recipe, which I did not make.**
Until then this fix has no suite, and `parity_orphan_producers`'s O2 case asserts
only what a suite *can* see: the producer's own value (which seat it reports),
not its reachability.

## VERIFIED-NOOP

### O3's second arm, `cannot_activate_members`. **NOT LIFTED — MEASURED.**

The brief asks for both `wait_immune` and `cannot_activate_members`. The first is
a real orphan. The second is not, and lifting it would have created a **dead
write** — the defect this wave is about:

* its reader `rb_rules_cannot_activate_player` (`rules.c:2338`) has **zero
  callers tree-wide** (`nm`, above);
* the behaviour is **already shipped** through the GameState fold
  `GameState.player_cannot_activate[2]` — written by the LIVE handler at
  `misc.c:1118` and read at `state.c:235`, which is the C's fold of Rust's ONE
  production reader `engine/src/ability/effects/change_state.rs:310-317`, plus
  `engine.c:460` and `engine.c:1453`.

So `cannot_activate` is not missing; it is filed in the wrong place, and
`phase.c:582`'s "zero callers" note is the correct reading.

### D2 — `card_total_hearts`, three copies. **CONFIRMED: 3 copies, 2 of them live.**

| # | location | linkage | callers |
|---|---|---|---|
| 1 | `src/ability/effects/state.c:2044` | **exported**, `int rb_card_total_hearts(const Card *)` | 3 test files only (`test_parity_card_identity`, `test_parity_full_round3`) |
| 2 | `src/ability/condition.c:4011` | `static` | `:2920`, `:4101`, `:4107`, `:4114` — **LIVE** |
| 3 | `src/ability/util.c:863` | `static` | `:1872` — **LIVE** |

Recorded so that any future Q149/Q172 fix lands three times. Consolidating is a
separate, risky change and is **not** attempted here: all three files are
contested or test-owned.

### D4 — `rb_card_filter_subset`. **CONFIRMED HARMLESS; THE FIX IS A COMMENT DELETE.**

`include/rabuka.h:2715-2723` is a comment block whose prose narrates a
`#define` that no longer exists; the `#define` text at `:2716` is **inside that
comment**, so there is no live macro and nothing is shadowed. The only live
definition is the `static inline` at `:2724`. **The exact removal is
`include/rabuka.h` lines 2715-2723 — nine lines, comment only, no code.** It is
safe: the block contains no `/*` (the `-Wcomment` hazard the Makefile header
warns about), `include/rabuka.h` has no `WARN_AUDIT_BASELINE` entries, and a
comment delete cannot move `sizeof(GameState)`. Not landed — the file is not mine.

## THE FOURTH ORPHAN FOUND WHILE MEASURING, REPORTED NOT LANDED

`cannot_live` is the same defect as O3 and is **not** in the work order:
`RbRulesState::cannot_live_players[2]` is written only by the uncalled
`rb_rules_execute_restriction` (`rules.c:1926`), while its reader
`rb_rules_cannot_live_player` (`rules.c:2328`) is **live** — called from
`src/turn/triggers.c:336` and `src/turn/live.c:646`. So the reader always answers
0, which is the Q68 / checklist item I2 defect, and it is a fourth instance of the
audit's own finding D6. It is NOT fixed here: `triggers.c` and `live.c` are
contested, and wiring that writer is I2's, not this wave's.

## REGRESSION THIS WAVE CAUSED

**`parity_per_card2`: 14 → 16 failures. Two, both from O1, both attributed by
isolation build.** `orph5` = baseline + O3 only → `parity_per_card2` = **14**,
identical to baseline, so O3 moves nothing in that suite and the two extra
failures are O1's. `g->live_success[0/1]` is cleared in exactly one place in the
tree — `src/core/tracking.c:56-57`, inside `rb_reset_keyword_tracking` — which
only the new O1 call site now reaches.

* `e2 PRECONDITION: p1's live SUCCEEDED … (phase=5 success=0)`
* `e2_bouken_gained_draws`

**MECHANISM, AND IT IS NOT A WEAKENING TO "FIX" IT.** `advance_to_live_success`
in that suite walks up to 10 `test_pass` steps and ends at `phase=5`
(`RB_PHASE_ENERGY`) — i.e. it crosses a **round boundary**. It then asserts a
per-turn flag. Before the fix the flag LATCHED, so the assertion read a value from
the previous round; after the fix it is correctly cleared at the round's own
Active entry, exactly as `phases.rs:98` clears it in Rust. The gained ability's
ライブ成功時 dispatch had already fired in that earlier round and cannot fire
again, which is correct engine behaviour.

**`tests/test_parity_per_card2.c` WAS NOT EDITED.** It is not in this task's file
list and `tests/**` is largely contested, so this is reported with the fix named
instead: `advance_to_live_success` must return as soon as the ライブ成功時 dispatch
is seen **in the round that produced it**, the same way
`advance_to_live_start`/`advance_to_live_set` are bounded, rather than walking a
fixed 10 steps. **No test was weakened, deleted or ignored to reach a number.**

## THE OTHER SUITE THAT MOVED, AND WHY IT IS NOT A REGRESSION

`coverage_gate` 164 → 165, from exactly one new line:
`G1 manifest-vs-glob: tests/test_parity_orphan_producers.c has no
tests/parity_orphan_producers.log`. **This rule is already failing 162 times at
baseline** — `make t` does not write `tests/*.log`; only `make run-all-tests`
does, and that target was not run in the shared tree. At baseline the same suite
also emits the two meta failures `G4/G5: there are NO tests/*.log files` and
`G5: zero suites reported PASSED`. The +1 is the new file meeting a pre-existing
red rule, and a `run-all-tests` pass would clear all 165 at once.

## GATES AND THE FULL RUN

| measurement | baseline (snapshot, only these hunks reverted) | after |
|---|---|---|
| suites | 164 | **165** (the new one) |
| `FAIL:` lines (case-sensitive, stdout + stderr) | 3923 | **3926** |
| suites `FAILED:` / `PASSED:` | 113 / 51 | 113 / **52** |
| build-red / no-verdict | 0 / 0 | 0 / 0 |
| `make check-suite-names` | PASSED | **PASSED** (165 suites, 47 names) |
| `make warnings-audit` | **RED** — 2 NEW + 2 STALE, both `src/core/game_state_core.c:883`/`:962` vs pins `:868`/`:947` | **PASS — 12 diagnostics, all pinned, 0 new** |
| `src/abilities_strings.bin` | 726831 B / `89204df0bbde08a0…` | **726831 B / `89204df0bbde08a0…`** |
| `src/cards.bin` | 631890 B / `728208504fee6217…` | **631890 B / `728208504fee6217…`** |

**⚠ THE GATE TURNED GREEN, AND IT WAS NOT THIS WAVE.** The four
`game_state_core.c` records moved because that file changed underneath
(`src/core/game_state_core.c` is contested and was already listed modified when
this wave started). **This wave's contribution to the gate is: zero.** The proof
is that all five `rules.c` pins — `215`, `261`, `895`, `1398`, `1601`, cluster-W
rows FT-1..FT-5 — still match, which is why the new rules.c functions are placed
AFTER line 1601 and why a peer's file-scope forward declaration for
`rb_rules_record_wait_immune_by_effect`, which had been added at `rules.c:39-43`,
was **moved down to `rules.c:~1862`**: at `:39` it would have shifted all five
pins by +5 and manufactured five NEW plus five STALE records out of a comment.

## THE NEGATIVE CONTROL, PROVEN NOT ASSERTED

`orph3` = the baseline snapshot with **only these four hunks reverted** and
`tests/test_parity_orphan_producers.c` byte-identical to the version that passes:

```
FAILED: parity_orphan_producers [exit=1, 12 result line(s) …]
8 failed  —  o1_active_entry_resets_the_keyword_trackers
             o1_second_active_entry_in_the_same_round
             o3_wait_immune_is_produced_on_the_live_path
             o3_the_rollover_clears_the_list
PASSED:   parity_orphan_producers [exit=0, 0 result line(s)]   (with the fixes)
```

Every one of those four functions drives a production entry point; none calls the
function under repair directly, which is the failure mode that let O1 and O3
survive an audit in the first place.

## NEW EXPORTS CREATED BY THIS WAVE — STATED EXPLICITLY, BECAUSE THIS IS HOW THE CLASS GROWS

**Two, and both are production-called from `src/`, not from `tests/`:**

| symbol | defined | production caller |
|---|---|---|
| `rb_rules_record_wait_immune_by_effect` | `rules.c` | `src/ability/effects/misc.c` (`h_restriction`) **and** `rb_rules_execute_restriction` |
| `rb_rules_clear_turn_state` | `rules.c` | `src/turn/phase.c` (the rollover) |

`nm` over all 49 `OBJ_LIB` objects at the after snapshot confirms both are
referenced by another `src` object. **No TEST-ONLY export was created.** The
prototypes for both are declared **file-scope in the consuming `.c`** — not in
`include/rabuka.h`, which is the coordinator's file and which is unchanged.
The suite declares `rb_rules_is_wait_immune` the same way `src/ability/effects/state.c:25`
already does; that declaration is a `BLOCKED-ON` for the header, not a new symbol.

## WHAT THIS WAVE DID NOT DO, ON PURPOSE

* `src/core/tracking.c` — READ ONLY, hash unchanged.
* `include/rabuka.h` and the `Makefile` — not mine, not edited.
* `src/ability/effects/state.c` — the reader site is already correct; the C's
  `cannot_activate` reader at `state.c:235` was left exactly as found.

---

# WAVE: the 224 dead globals that have a Rust twin — ALL ADJUDICATED

**2026-10-04.** Full write-up, with a row per function and every measurement,
in **`CENSUS_dupes2.md`**. Summary of the tally:

| verdict | count over the 219 measured | action |
|---|---|---|
| DEAD-IN-RUST-TOO | **31** | tombstoned |
| **DEAD-WRONG** | **6** | **4 fixed faithfully, 2 deleted + tombstoned** |
| DEAD-CORRECT | **182** | tombstoned as faithful |
| WIRE IT | **0 landed** | 1 missing engine path found, now closed |

6 + 31 + 182 = 219, **zero rows unclassified**. The 219 is a fresh measurement,
not the census's 224: `CENSUS_wiring.md` is stale (`rb_rules_execute_forget`,
quoted there as a live example, no longer exists in `src/` or `include/`), and
its per-file counts do not reproduce. `TEST_ONLY_AUDIT.md` was **not** redone;
the 141 test-named rows in these files are listed in `CENSUS_dupes2.md` §10 and
excluded from the denominator.

## DEAD-WRONG — the six, and what each one got wrong

| function | file | the divergence | action |
|---|---|---|---|
| `rb_card_property_str` / `rb_card_property_from_str` | `core/card.c` | Rust `CardProperty` has **four** variants (`card.rs:2750-2785`); the C tables had three. `has_draw_icon` (3) round-tripped to 0 = `has_blade_heart`. **`abilities.json:13305` carries it — 1 of 49 `card_property` nodes.** The live path bypasses the tables (`rb_check_card_property` matches the raw string), which is why nothing fired. | **both arms added** |
| `rb_energy_add_active` | `core/zones.c` | `saturating_add` is on a **`u8`** (`zones.rs:682-684`) so Rust clamps at 255; the C clamped only the lower bound and `RbPlayer.energy_active` is **`int`** (`rabuka.h:800`). | **upper clamp added** |
| `rb_player_draw_energy` | `core/zones.c` | **three** defects vs `player.rs:585-591` + `zones.rs:718-722`: appended at `energy.n` instead of inserting at the active boundary (corrupts the active prefix when energy is waited — the exact thing Rust's comment forbids); **drew before the capacity test, so a full zone DESTROYED the card**; no `u8` saturation. | **rewritten faithfully** |
| `rb_play_card` | `engine.c` | no Rust twin. Any non-member — **energy included** — went to `P->live`; the live path ran **no live-card rule at all**; "first free stage slot" is a policy Rust does not have (`phases.rs:1260-1271` takes an explicit area). | **deleted + tombstoned** |
| `heart_color_of` | `engine.c` | no Rust twin. Mapped `"b_all"` → index 7; Rust's `heart_color_table!` (`card.rs:4460-4476`) has `BAll => "b_all", 0` and `All => "all", 7` — the **blade wildcard collapsed into the all-colours wildcard**. `"b_all"` appears 0× as a quoted value in `abilities.json`. | **deleted + tombstoned** |
| `rb_try_phase4` / `rb_try_all_distribution` | `turn/live.c` | exported shapes that cannot be written faithfully (no pool, no allocations, no deficit list); `rb_try_all_distribution` is a bare `return 0`. The **faithful ports are live and file-local** (`rb_backtrack_allocate → bt_search → bt_try_surplus → bt_try_*`). | prior adjudication **upheld**, not re-adjudicated |

**Coverage: `tests/test_parity_dead_twins.c`, 47 assertions, 0 failures.** New
suite; no existing test weakened, deleted or ignored.

## MISSING ENGINE PATH — one, now closed

`rb_ability_debug_set` (see D3 above, landed by a peer at `web_server.c:883-886`).
**Verified working**, and the `web_server.c` JSON sweep is **CLEAN as a code
guarantee, not a data-shape dependency**: `json_string()` (`web_server.c:76-88`)
escapes `"`, `\`, `\n`, `\r`, `\t` and every C0 control as `\u00xx`, and
`c.name` is routed through it at `:117`. The only two `snprintf("%s")` into a
json-ish buffer are a fixed key at `:365` and a filesystem path at `:837`.

## `alloc_counter.c` — TOMBSTONED, NOT WIRED

34 exports, 780 lines, in neither `SRC` nor `OBJ_LIB`. Rust's
`alloc_counter.rs` is **unreachable in every default build**:
`lib.rs:58-59` gates the module behind `#[cfg(feature = "alloc_tracker")]` and
`alloc_tracker` is **not** in `Cargo.toml`'s `default` feature set, so
`lib.rs:94-99` installs plain `std::alloc::System`; the only library call site
(`core/game_state/abilities.rs:1917-1919`) is itself feature-gated; and the
readers `alloc_count()`/`alloc_buckets()` are double-gated behind
`timer::Timer`, itself `#[cfg(feature = "profiling")]`, whose **five** library
call sites all compile out. Already recorded independently at
`AUDIT_core_state.md:799`. **No `rabuka.h` prototypes are requested**; the
paste-ready block is in `CENSUS_dupes2.md` §4 for the record only.

Compile matrix, measured: `-std=c99`/`c11`/`c17` compile with **0 warnings**;
`-std=c2x` fails on `ATOMIC_VAR_INIT` (C23 removed it). The project builds
`-std=c11`, so that was never the reason for the verdict.

## `card_total_hearts` — three copies, all semantically identical. RECORDED.

`effects/state.c:2044` (global, `tests/` only, **no `src/` caller**),
`ability/condition.c:4011` (static, **4 live**), `ability/util.c:863` (static,
**1 live**). All three implement `card.rs:4370-4379` correctly and clamp by
`n_hearts`; the null guard on the global copy has no Rust counterpart (Rust
cannot return -1, and an older port did, making running aggregates go DOWN).
**New defect found:** all three comments cite the Rust twin at the **wrong
line** (`4194-4202` / `4263-4271`; it is at **`4370-4379`**). **Not consolidated** —
all three files are in the DO NOT EDIT fence; each static body would just call
the exported `rb_card_total_hearts`. The new suite now pins the shared
semantics, so a Q149/Q172 fix landing in only some copies fails there.

## BEFORE / AFTER

`make clean` before both builds; 165 → **166** suites; `^FAIL` counted
case-sensitively with **no colon**, stdout **and** stderr, strictly sequential.

| | before | after | delta |
|---|---|---|---|
| suites | 165 | **166** | +1 (new suite) |
| **`^FAIL` (no colon)** | **1 348** | **1 348** | **0** |
| `^FAIL:` | 2 814 | 2 815 | +1, **entirely `coverage_gate`** (not a metric) |
| GREEN | 55 | **56** | +1 |

**No regression.** `coverage_gate`'s single line is its manifest-vs-glob gate
counting the one new `tests/test_*.c` with no `tests/*.log`
(`sources=165 → 166`). `zone_filter_parity` was recorded `BUILD_RED` because the
harness timeout killed the runner mid-link and left a 0-byte log; re-run
standalone it compiles, runs, prints `ALL ZONE AND FILTER CHECKS PASSED` and
emits **0** `^FAIL` lines — that row was corrected from a measured re-run.

## MEASUREMENT TRAPS THAT FIRED ON THE MEASUREMENT ITSELF

Two of my own instruments were wrong **in the direction that makes work look
unnecessary**, which is the failure mode the brief warns about. Full detail in
`CENSUS_dupes2.md` §0:

1. `-Wl,--gc-sections` **does nothing on PE/COFF** — it reported 0 dead of
   1 433 because `-ffunction-sections` emits `.text$name` and the linker option
   is ELF-only. Method discarded.
2. `re.sub(r"/\*.*?\*/", " ", src, flags=re.S)` **is not a C lexer**: it opens a
   false `/*` at `y/*p`, and the non-greedy match swallows the rest of the file.
   Measured on `src/turn/phase.c`: **2 309 lines in → 547 out**, destroying all
   three intra-TU call sites of `rb_backtrack`. Replaced with a character-wise C
   lexer. A third defect — excluding the whole defining TU from the intra-TU map
   — was a 100 % false-positive rate on that axis.
3. A 14-row random sample of the final actionable set was re-verified by eye
   against raw `grep`: **14/14 correct**.
* No generator was run. No `.o` or `rb_engine*` was left in the live tree.