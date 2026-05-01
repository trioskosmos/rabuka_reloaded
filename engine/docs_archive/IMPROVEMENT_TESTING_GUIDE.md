# Ability System Improvements Testing Guide

## Improvements Implemented

### 1. Look and Select with User Choice
**What was improved:** When an ability uses `look_and_select` with `placement_order` parameter, the engine now prompts the user to select which cards to move instead of automatically selecting them.

**How to test:**
1. Play a card with `look_and_select` ability (e.g., 渡辺 曜 - PL!S-bp2-005-R+)
2. The ability text: "自分のデッキの上からカードを7枚見る。その中から{{heart_02.png|heart02}}か{{heart_04.png|heart04}}か{{heart_05.png|heart05}}を持つメンバーカードを3枚まで公開して手札に加えてもよい。残りを控え室に置く。"
3. **Expected behavior:** When the ability triggers, you should see a prompt to select cards from the looked-at cards
4. **Verify:** The prompt shows the looked-at cards and allows you to choose which ones to add to hand

### 2. Change State with Multiple Targets
**What was improved:** When an ability uses `change_state` to set cards to wait state and there are multiple valid targets, the engine now prompts the user to select which specific targets to change.

**How to test:**
1. Use an ability that changes energy cards to wait state with multiple valid targets
2. **Expected behavior:** If there are more valid targets than the count specified, you should see a prompt to select which energy cards to deactivate
3. **Verify:** The prompt shows valid energy cards and allows selection

### 3. Duration Expiration at Live End
**What was improved:** Added `expire_live_end_effects()` method to expire effects with duration "live_end" when the live ends.

**How to test:**
1. Play a card with an ability that has duration "live_end" (e.g., 夕霧綴理 ab#1: "ライブ終了時まで、{{icon_blade.png|ブレード}}を得る")
2. Start a live and gain the temporary effect
3. End the live
4. **Expected behavior:** The temporary effect (blade gain) should be removed
5. **Verify:** Check console logs for "Expired X effects with duration 'live_end'"

## Integration Tests

**File:** `tests/ability_integration_tests.rs`

These integration tests use real cards from the card database and simulate actual gameplay to expose engine faults. They test the ability system improvements by:

1. **test_look_and_select_with_placement_order** - Verifies that look_and_select with placement_order prompts user for selection
2. **test_change_state_with_multiple_targets** - Verifies that change_state with multiple valid targets prompts user
3. **test_look_and_select_without_placement** - Verifies that no placement_order does NOT prompt user
4. **test_single_target_selection** - Verifies that single target does NOT prompt user
5. **test_ability_with_duration** - Verifies that effects with duration "live_end" expire correctly
6. **test_actual_card_ability_execution** - Tests ability execution using actual card from database
7. **test_play_card_and_activate_ability** - Tests playing a card and activating its ability naturally
8. **test_natural_gameplay_with_choices** - Tests natural gameplay where player makes choices
9. **test_energy_management_in_gameplay** - Tests energy management during natural gameplay

**Test Results:** ✅ All 9 tests passed

**To run integration tests:**
```bash
# Stop the web server first if running
taskkill /F /IM rabuka_engine.exe

# Run the integration tests
cargo test --test ability_integration_tests
```

## Testing Checklist

- [x] Look and select with placement_order prompts user
- [x] Change state with multiple targets prompts user
- [x] Look and select without placement_order does NOT prompt user
- [x] Single target selection does NOT prompt user
- [x] Duration effects expire at live_end
- [x] Integration tests use real cards from database
- [x] Integration tests pass (9/9)

## Test Cards

### For Look and Select
- **渡辺 曜 (PL!S-bp2-005-R+)**: Look at top 7, select cards with hearts
- **小原鞠莉 (PL!S-bp2-008-R+)**: Look at top 2, select order for deck top

### For Change State
- Any ability that sets energy to wait with count < total available

### For Duration Expiration
- **夕霧綴理 (PL!HS-bp1-004-R+ ab#1)**: Gain blade until live_end
- **藤島 慈 (PL!HS-bp1-006-R+ ab#1)**: Gain heart until live_end

## Console Output to Monitor

Look for these messages:
- "Look at top X cards of deck for self"
- "Select X card(s) from the looked-at cards (placement_order: ...)"
- "Select X energy card(s) to set to wait state from X valid targets"
- "Expired X effects with duration 'live_end'"

## Next Steps

1. ✅ Integration tests implemented and passing (9/9)
2. Restart web server and perform manual gameplay testing
3. Document any issues found in ABILITY_GAMEPLAY_TEST_RESULTS.md
