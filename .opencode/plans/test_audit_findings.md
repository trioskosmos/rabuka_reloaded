# Test Quality Audit

## Date: 2026-05-18
## Total tests: 471 (all passing)

---

## 1. Zero-assertion tests (CRITICAL)

These tests set up a scenario and advance through phases but **never assert any behavior**. They pass even if the ability is silently broken.

### 1.1 `strawberry_q132_first_attacker_evaluated`
**File:** `engine/tests/test_modules/strawberry_test.rs:68`
**Problem:** Sets `opponent_live_success_this_turn = true` and `opponent_live_no_excess_heart_this_turn = true`, passes through two phases. **Zero assertions.**
**Fix:** Assert score modifier value on the strawberry card after the phase pass.

### 1.2 `strawberry_q142_excess_heart_prevents_score`
**File:** `engine/tests/test_modules/strawberry_test.rs:100`
**Problem:** Sets `opponent_live_no_excess_heart_this_turn = false`, passes through phases. **Zero assertions.**
**Fix:** Assert score modifier is 0 (excess heart blocks bonus).

### 1.3 `strawberry_q142_wrong_group_prevents_score`
**File:** `engine/tests/test_modules/strawberry_test.rs:132`
**Problem:** Sets up non-matching group, passes through phases. **Zero assertions.**
**Fix:** Assert score modifier is 0 (wrong group blocks bonus).

### 1.4 `strawberry_opponent_didnt_win_no_score`
**File:** `engine/tests/test_modules/strawberry_test.rs:164`
**Problem:** Sets `opponent_live_success_this_turn = false`, passes through phases. **Zero assertions.**
**Fix:** Assert score modifier is 0 (opponent didn't win → no bonus).

### 1.5 `you_live_start_optional_cost_triggers`
**File:** `engine/tests/test_modules/gameplay_test.rs:1376`
**Problem:** Plays You to stage, then does `if game.has_pending_choice() { eprintln!(...) }` — no assertion.
**Fix:** Assert `has_pending_choice()` is true, pay the cost, verify blade modifier applied.

### 1.6 `you_abilityless_card_not_in_choice`
**File:** `engine/tests/test_modules/gameplay_test.rs:1405`
**Problem:** Same pattern — `if has_pending_choice() { eprintln!(...) }`.
**Fix:** Assert pending choice exists and filler card is excluded from options.

### 1.7 `mari_q131_one_card_deck_blocked`
**File:** `engine/tests/test_modules/mari_test.rs:43`
**Problem:** Only checks `main_deck.cards.is_empty()` — never asserts ability was actually blocked.
**Fix:** Assert hand count unchanged, no look-at occurred.

### 1.8 `mari_q131_zero_deck_blocks_live_start`
**File:** `engine/tests/test_modules/mari_test.rs:14`
**Problem:** Only checks `main_deck.cards.is_empty()`.
**Fix:** Assert no cards drawn, no reorder, ability correctly blocked.

### 1.9 `turn_limit_prevents_second_activation`
**File:** `engine/tests/test_modules/gameplay_test.rs:1897`
**Problem:** Activates Chika's ability twice, discards second Result with `let _result = ...`. Never checks `_result.is_err()`. Never asserts first activation had an effect.
**Fix:** Check first activation state change, assert second activation returns `Err`.

---

## 2. Tests that stop before the actual effect (HIGH)

These tests set up correctly but never trigger/verify the actual ability effect.

### 2.1 `rurino_ozora_cost_payment_success`
**File:** `engine/tests/test_modules/card_ability_tests.rs:240`
**Problem:** Sets up Rurino Ozora with other Mirakura members, places cost/live cards. Never actually triggers the ability. Stops at initial state assertions.
**Fix:** After setup, assert pending choice, pay cost, select target from discard, verify it moved to hand.

### 2.2 `rurino_ozora_only_selects_mirakura_cards`
**File:** `engine/tests/test_modules/card_ability_tests.rs:111`
**Problem:** Sets up Mirakura and non-Mirakura cards in discard. Never triggers the ability to verify the filter.
**Fix:** Trigger the ability, inspect choice options, confirm non-Mirakura cards excluded.

### 2.3 `ayumu_kanon_koko_debut_recover_from_discard`
**File:** `engine/tests/test_modules/gameplay_test.rs:647`
**Problem:** Loads card data, checks metadata. Never plays the card, triggers debut, or recovers anything from discard.
**Fix:** Play the card to stage, trigger debut, pay cost, select target from discard.

### 2.4 `mari_live_start_sufficient_deck_fires`
**File:** `engine/tests/test_modules/mari_test.rs:71`
**Problem:** Only checks `main_deck.cards.len() > 0`. Never verifies look-at-reorder effect.
**Fix:** After resolving look choice, verify deck order changed, waitroom got remaining cards.

### 2.5 `rurino_ozora_no_trigger_when_alone_on_stage`
**File:** `engine/tests/test_modules/card_ability_tests.rs:24`
**Problem:** Tests failure case (no other members), but the success case at line 68 never goes end-to-end (lines 105-107 say "Actual effect execution would depend on player choosing to pay cost").
**Fix:** Complete the success path through cost payment, target selection, verify card moved to hand.

---

## 3. Single-path tests missing the opposite path (MEDIUM)

These tests only test one scenario (success OR failure) but not both.

### 3.1 `eli_bp5_sequential_wait_then_discard_works`
**File:** `engine/tests/test_modules/eli_sequential_cost_test.rs:7`
**Problem:** Only tests success case. Missing: what if can't pay wait cost? What if nothing to discard? What if card already in wait?
**Fix:** Add failure variants — insufficient wait targets, empty hand to discard.

### 3.2 `eri_q144_up_to_semantics_1_eligible_opponent_still_works`
**File:** `engine/tests/test_modules/eri_bp3_test.rs:13`
**Problem:** Tests only "1 eligible opponent." Missing: 0 eligible, 2+ eligible (cap at 2?), mixed eligible/ineligible.
**Fix:** Add edge cases for 0, 2, and mixed eligibility.

### 3.3 `cara_tesoro_q203_live_start_fires`
**File:** `engine/tests/test_modules/remaining_quick_test.rs:93`
**Problem:** Name says "fires" but only asserts `mod_val == 0` — the case where the alternative branch is taken (no score). Missing: the positive case where condition is met and score IS applied.
**Fix:** Add a test where the condition passes and score is non-zero.

### 3.4 `kanon_ab1_live_success_fires_but_live_fails_no_hearts`
**File:** `engine/tests/test_modules/kanon_test.rs:37`
**Problem:** Tests only "live fails" (no hearts). Missing: the positive case where hearts ARE met.
**Fix:** Add a test where heart requirements are satisfied, optional cost appears, score bonus applies.

### 3.5 `kanon_q93_partial_resolution_one_card` / `zero_cards`
**File:** `engine/tests/test_modules/kanon_test.rs:90,134`
**Problem:** Test partial resolution (1 card, 0 cards) but only verify Kanon still on stage. Never check discard count or draw count.
**Fix:** Verify hand count changed appropriately.

### 3.6 `hazuki_activate_two_liella_discarded_gain_2_blade`
**File:** `engine/tests/test_modules/hazuki_test.rs:11`
**Problem:** Has success case (2 Liella → 2 blade) and two failures (0 Liella, empty deck). Missing: `ターン1` use-limit enforcement (second activation in same turn should fail).
**Fix:** Add a second activation assertion that it fails.

### 3.7 `ayumu_bp5n_heart01_condition_passes_but_modifier_not_found`
**File:** `engine/tests/test_modules/energy_and_member_under_test.rs:477`
**Problem:** Tests only the bug scenario (condition passes but modifier unfindable). Missing: success once fixed, and true failure (no energy under member).
**Fix:** Add success and failure tests.

---

## 4. "Both" target tests that don't verify both players (MEDIUM)

### 4.1 `setsuna_q230_both_zero_heart02_gained`
**File:** `engine/tests/test_modules/setsuna_bp5_test.rs:21`
**Problem:** Only checks `get_heart_modifier(setsuna, Heart02)` for P1. Never checks P2.
**Fix:** Also verify P2 received the same heart modifier.

### 4.2 `wien_bp5_q223_both_centers_empty_no_moves`
**File:** `engine/tests/test_modules/wien_bp5_test.rs:113`
**Problem:** Checks both centers empty but never asserts the ability produced no pending choices for either side.
**Fix:** Assert `!game.has_pending_choice()` and that neither player's position change occurred.

---

## 5. Weak assertion tests (MEDIUM)

These assertions would pass with absurdly wrong values.

### 5.1 `hareruya heart modifier tests`
**File:** `engine/tests/test_modules/gameplay_test.rs:1746-1771`
**Problem:** Uses `>= 2` instead of `== 2`. Would pass if engine applied 999 hearts.
**Fix:** Use `==` for exact value verification.

### 5.2 `energy_zone_capacity_handled`
**File:** `engine/tests/test_modules/gameplay_test.rs:1929`
**Problem:** Named like an edge-case test but only tests happy path (20 energy). Missing 0, 1, 21, 30, spending at cap.
**Fix:** Add tests for capacity boundaries.

---

## 6. Misplaced or incomplete tests (LOW)

### 6.1 `ayumu_q62_and_name_has_individual_names`
**File:** `engine/tests/test_modules/gameplay_test.rs:694`
**Problem:** Database parser test disguised as gameplay. No gameplay verification.
**Fix:** Move to parser test module or add gameplay assertions.

### 6.2 `you_q129_cost_reduction_self_only`
**File:** `engine/tests/test_modules/gameplay_test.rs:1365`
**Problem:** Just checks `card.cost == Some(20)` from database. Not an integration test.
**Fix:** Add full scenario with cost-reducing cards on stage.

### 6.3 `action_coverage_test.rs`
**File:** `engine/tests/test_modules/action_coverage_test.rs:28`
**Problem:** Iterates all action types, only checks no crash. Never verifies side effects.
**Fix:** For each action type, verify at least one state change (modifier applied, card moved).

### 6.4 `e2e_basic_game_test.rs`
**File:** `engine/tests/test_modules/e2e_basic_game_test.rs:316`
**Problem:** Discards `Result` from phase transitions with `.ok()`.
**Fix:** Check each phase transition succeeds (`unwrap()` or `expect()`).

---

## Summary

| # | Category | Count | Tests |
|---|----------|-------|-------|
| 1 | Zero-assertion tests | 9 | strawberry x4, you x2, mari x2, turn_limit |
| 2 | Stop before effect | 5 | rurino x3, ayumu, mari |
| 3 | Missing opposite path | 7 | eli, eri, cara_tesoro, kanon x2, hazuki, ayumu |
| 4 | "Both" incomplete | 2 | setsuna, wien_x4 (partial) |
| 5 | Weak assertions | 2 | hareruya, energy_zone |
| 6 | Misplaced/incomplete | 4 | ayumu, you, action_coverage, e2e |

**Unique test files affected:** ~20 files
**Tests needing fixes:** ~29 tests (of 471)
**Estimated effort:** 2-3 hours for all
