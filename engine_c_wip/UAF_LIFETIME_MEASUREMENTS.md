# UAF measurement data (kept in a report file, not a snapshot dir)

## SHA-256 of the three claimed files at measurement time

src/core/game_state_abilities.c  7525305f02f31d2a788a0633babbecfd177786a9dff6af7225527c2352905872
src/ability/choice.c             (see live tree; two edits: the call-site correction
                                   at choice.c:5126-5170 and the correction to the
                                   justification comment at :4937)
src/ability/ability_queue.c      (one edit: rb_queue_resume_pending_actions, :1066)

## A/B per suite: ^FAIL (case-sensitive, no colon), stdout AND stderr

| suite | baseline | +pca lifetime fix | +「そうした場合」gate | all three |
|---|---|---|---|---|
| parity_cce_gaps | 90 | 84 | 82 | 82 |
| parity_under_member | 83 | 87 (REGRESSION) | 81 | 81 |
| parity_look_per_card | 68 | 65 | 65 | 65 |
| parity_search_reorder | 97 | 97 | 97 | 97 |
| parity_sequential | 142 | 142 | 142 | 142 |
| parity_small_clusters | 101 | 101 | 101 | 101 |
| parity_queue_resume | 5 | 5 | 5 | 5 |
| **TOTAL (24 suites)** | **1253** | 1258 | **1240** | **1240** |

## Assertions that went GREEN (Rust-asserted, previously red)

parity_cce_gaps    j2: the looked-at card goes to the WAITROOM when nothing is kept
parity_cce_gaps    j4: ALL THREE looked-at cards go to the waitroom (expected 3, actual 0)
parity_cce_gaps    j8: the three looked-at cards go to the waitroom (expected 3, actual 0)
parity_cce_gaps    j8: the REVEALED live card is on the deck top (expected 2644, actual 2643)
parity_cce_gaps    h3: declining the placement means no 「そうした場合」 branch and no blade (expected 0, actual 2)
parity_cce_gaps    [suite] h3_q158_skip_no_blade
parity_cce_gaps    [suite] j2_one_member_discard
parity_cce_gaps    [suite] j4_three_skip_all
parity_under_member ayumu: 0 cards drawn when the optional cost is skipped (got 4 expected 2)
parity_under_member [suite] ayumu_bp3n_debut_skip_no_draw
parity_look_per_card dia: exactly 1 looked-at card should be kept on the deck (got 3 expected 1)
parity_look_per_card dia: the other 2 looked-at cards go to the waitroom (got 0 expected 2)
parity_look_per_card [suite] e_dia_keeps_bottom
NEWLY RED: none, in any suite.

## Class audit (probe wraps rb_effect_free + rb_free_ability, checks 7 alias slots
## against every freed subtree while a prompt is open; 24 suites)

  negative control (pca fix reverted, probe MUST fire): 28 hits
    parity_integration 10, parity_search_reorder 8, parity_wave1b 5,
    parity_full_round 4, parity_int_gaps 1
  all three fixes in place:                              0 hits

## Consumer-side proof: saved_eff reads in rb_resume_with_choice_indices_internal

  WITHOUT fix: 0 of 10 reads still carry extra0_k=discard_remaining
  WITH fix:    7 of 10 reads still carry extra0_k=discard_remaining

## Free-site proof: the same node across pca_resolve_entry rb_free_ability

  before: action=select_cards destination=deck_top extra_n=2
          text=1枚までをデッキの上に置き、残りを控え室に置く
          extra[0] k=discard_remaining v=true ; extra[1] k=max v=true
  after : action=<garbage>  destination=deck_top (survives by luck)
          text=<garbage> ; extra[0] k=<garbage> v=<garbage>
          extra[1] k=<garbage> v=true
  survives exactly ONE prompt.
