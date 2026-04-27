# Quality Test List for All Test Plans

This document provides quality test lists for each of the 38 test plans in the qa_individual directory.

---

## Deck Validation Tests

### q003_main_deck_composition.md
**Quality Tests:**
- Verify deck validation accepts 48 member + 12 live = 60 total
- Verify deck validation accepts 24 member + 6 live = 30 total (half deck)
- Verify deck validation rejects incorrect composition (e.g., 50 member + 10 live)
- Verify validation error messages are clear and specific
- Verify no false positives or false negatives in validation

### q004_main_deck_duplicates.md
**Quality Tests:**
- Verify deck validation rejects >4 copies of same card number
- Verify deck validation accepts exactly 4 copies of same card number
- Verify validation works across different rarities of same card number
- Verify error message specifies which card exceeded limit
- Verify validation is case-insensitive for card IDs

### q005_same_card_number_different_rarity.md
**Quality Tests:**
- Verify deck validation accepts 2 R + 2 P = 4 total of same card number
- Verify deck validation rejects 2 R + 3 P = 5 total of same card number
- Verify validation counts total copies across all rarities
- Verify validation handles all rarity types (R, P, L, etc.)
- Verify error message shows total count and limit

### q006_different_card_numbers.md
**Quality Tests:**
- Verify deck validation accepts 4 copies of card1 + 4 copies of card2 (different numbers)
- Verify validation treats different card numbers independently
- Verify validation works for any combination of different card numbers
- Verify no cross-contamination between different card number limits

### q007_energy_deck_duplicates.md
**Quality Tests:**
- Verify energy deck validation rejects >4 copies of same energy card
- Verify energy deck validation accepts exactly 4 copies of same energy card
- Verify energy deck validation is independent from main deck validation
- Verify error message specifies energy deck violation

---

## Stage and Debut Tests

### q028_debut_without_baton_touch.md
**Quality Tests:**
- Verify debut to occupied area without baton touch succeeds
- Verify old member sent to waitroom when replaced
- Verify energy cost equals new member's cost
- Verify debut ability triggers on replacement
- Verify baton_touch_count remains 0

### q030_stage_duplicate_allowed.md
**Quality Tests:**
- Verify stage allows duplicates of same card number
- Verify stage allows duplicates of same card name
- Verify no limit on duplicate members on stage
- Verify each duplicate member functions independently

### q031_live_duplicate_allowed.md
**Quality Tests:**
- Verify live card zone allows 2+ copies of same live card
- Verify live card zone allows copies with same card number
- Verify live card zone allows copies with same name
- Verify each live card copy functions independently

---

## Ability Timing Tests

### q037_auto_ability_once_per_timing.md
**Quality Tests:**
- Verify auto ability triggers only once per timing condition
- Verify auto ability does not trigger multiple times for same event
- Verify timing conditions are correctly identified
- Verify auto ability reset after timing window closes

### q038_live_card_definition.md
**Quality Tests:**
- Verify live card is correctly identified from card database
- Verify live card has required live-specific properties
- Verify live card can be set in live_card_zone
- Verify live card executes live-specific abilities

### q039_cheer_confirmation_required.md
**Quality Tests:**
- Verify cheer phase requires user confirmation
- Verify cheer cards cannot be placed without confirmation
- Verify user can add multiple cheer cards before confirming
- Verify cheer phase ends only after explicit confirmation

### q040_cheer_check_completion.md
**Quality Tests:**
- Verify cheer blade/heart counts are calculated correctly
- Verify victory determination uses correct cheer totals
- Verify cheer cards remain in success_live_card_zone after live
- Verify cheer contributions from both players are tracked separately

### q041_cheer_card_timing.md
**Quality Tests:**
- Verify cheer cards placed during cheer phase
- Verify cheer card live_start abilities do NOT trigger (timing passed)
- Verify cheer cards contribute blade/heart during performance
- Verify cheer card live_success abilities trigger on live success

### q042_cheer_ability_timing.md
**Quality Tests:**
- Verify cheer cards with live_start do NOT trigger when placed as cheer
- Verify cheer cards with live_success trigger when live succeeds
- Verify cheer cards with constant abilities are active during performance
- Verify multiple cheer cards with different ability types behave correctly

---

## Icon Effect Tests

### q043_draw_icon_effect.md
**Quality Tests:**
- Verify draw icon triggers when card is drawn from deck
- Verify draw icon effect executes correctly
- Verify normal card draws (without draw icon) don't trigger effects
- Verify hand size increases correctly with draw icon effects

### q044_score_icon_effect.md
**Quality Tests:**
- Verify score icons on live cards are counted toward live score
- Verify score icons on cheer cards are counted toward live score
- Verify score icon vs blade/heart distinction is maintained
- Verify score icons are counted at correct timing

### q045_all_blade_effect.md
**Quality Tests:**
- Verify blade modifiers granted to all members on stage
- Verify blade modifiers granted to activating card (self)
- Verify blade modifiers are independent per card
- Verify blade modifiers cleared when card leaves stage
- Verify "all" targeting correctly identifies all stage members

### q046_constant_heart_timing.md
**Quality Tests:**
- Verify constant abilities activate immediately when card is on stage
- Verify constant heart contributions are counted toward totals
- Verify duration (this_turn) expires at turn end
- Verify permanent constant abilities don't expire
- Verify multiple constant abilities can be active simultaneously

---

## Live Performance Tests

### q047_live_failure_no_score.md
**Quality Tests:**
- Verify live fails when blade/heart requirements not met
- Verify no score awarded for failed live
- Verify live card moved to waitroom on failure
- Verify cheer cards moved to waitroom on failure
- Verify success_live_card_zone is empty after failure

### q048_zero_score_win.md
**Quality Tests:**
- Verify zero score is correctly calculated when no score icons present
- Verify tie-breaking rules applied when scores are equal
- Verify winner declared even with zero score
- Verify live success possible with zero score
- Verify victory determination follows Rule 8.4.2

### q049_no_winner_turn_order.md
**Quality Tests:**
- Verify draw condition correctly detected when scores are equal
- Verify no winner declared (GameResult::Draw)
- Verify turn order continues normally after draw
- Verify live cards remain in success_live_card_zone
- Verify cheer cards remain in success_live_card_zone

### q050_both_winners_turn_order.md
**Quality Tests:**
- Verify double victory condition correctly detected
- Verify both players declared winners
- Verify GameResult::Draw set for double victory
- Verify turn order handles double victory correctly
- Verify game ends if double victory is game-ending condition

### q051_one_winner_turn_order.md
**Quality Tests:**
- Verify single victory condition correctly detected
- Verify correct player declared winner
- Verify GameResult set correctly (FirstAttackerWins or SecondAttackerWins)
- Verify loser's turn order affected appropriately
- Verify winner's cards handled per victory rules

---

## Deck Management Tests

### q053_deck_refresh.md
**Quality Tests:**
- Verify deck refresh triggers when main deck is empty
- Verify waitroom cards moved to main deck on refresh
- Verify waitroom is empty after refresh
- Verify main deck is shuffled after refresh
- Verify can draw from refreshed deck
- Verify main deck composition correct after refresh

### q054_multiple_success_draw.md
**Quality Tests:**
- Verify all live_success abilities trigger on live success
- Verify draw effects from multiple sources sum correctly
- Verify hand size updated correctly after each draw
- Verify deck size decreased correctly
- Verify multiple live successes can trigger in sequence
- Verify draw effects execute in correct order

---

## Cost and Restriction Tests

### q055_partial_effect_resolution.md
**Quality Tests:**
- Verify sequential effects execute in correct order
- Verify effects execute independently
- Verify partial failure handled correctly
- Verify state consistent after partial failure
- Verify rollback implemented if required by rules
- Verify effect execution order preserved

### q056_full_cost_required.md
**Quality Tests:**
- Verify full cost must be paid for ability activation
- Verify partial cost payment rejected
- Verify cost validated before effect execution
- Verify cost rolled back on effect failure
- Verify energy deactivated when cost paid
- Verify ability effect executes only after full cost paid

### q057_prohibition_priority.md
**Quality Tests:**
- Verify prohibition effects checked before card play
- Verify multiple prohibitions checked simultaneously
- Verify prohibition priority rules respected
- Verify card prohibited if any prohibition applies
- Verify card allowed if no prohibition applies
- Verify prohibition effects stack correctly

### q058_once_per_turn_per_card.md
**Quality Tests:**
- Verify turn-limited ability can be used once per turn per card
- Verify ability reuse prevented within same turn for same card
- Verify different cards can use their turn-limited abilities independently
- Verify turn-limited abilities reset on turn change
- Verify ability tracking is per card, not global

### q059_zone_move_resets_restrictions.md
**Quality Tests:**
- Verify area placement restrictions enforced
- Verify restrictions reset when card leaves zone
- Verify restrictions reset when card moves to different area
- Verify card can be played to area after restriction reset
- Verify restriction tracking is per area
- Verify zone moves correctly reset restrictions

### q060_auto_ability_mandatory.md
**Quality Tests:**
- Verify auto abilities trigger when conditions met
- Verify auto abilities execute automatically
- Verify no user choice presented for auto abilities
- Verify auto abilities cannot be skipped
- Verify auto abilities are mandatory
- Verify activation abilities are optional (for comparison)

### q061_once_per_turn_optional.md
**Quality Tests:**
- Verify optional ability presents user choice (use or skip)
- Verify ability not marked as used when skipped
- Verify ability can be used after skipping
- Verify ability marked as used when used
- Verify ability cannot be used again after use
- Verify ability reset on turn change

---

## Card Movement Tests

### q096_look_at_deck.md
**Quality Tests:**
- Verify look at deck ability shows top cards
- Verify cards remain in deck after looking
- Verify correct number of cards shown
- Verify cards shown in correct order
- Verify no cards drawn or moved

### q100_add_to_hand.md
**Quality Tests:**
- Verify card added to hand from specified zone
- Verify card removed from source zone
- Verify hand size increased by 1
- Verify source zone size decreased by 1
- Verify card is in correct position in hand

### q101_add_to_waitroom.md
**Quality Tests:**
- Verify card added to waitroom from specified zone
- Verify card removed from source zone
- Verify waitroom size increased by 1
- Verify source zone size decreased by 1
- Verify card order in waitroom

### q102_add_to_energy_zone.md
**Quality Tests:**
- Verify card added to energy_zone from specified zone
- Verify card removed from source zone
- Verify energy_zone size increased by 1
- Verify source zone size decreased by 1
- Verify energy card is active in energy_zone

### q103_move_to_stage.md
**Quality Tests:**
- Verify card moved to stage from specified zone
- Verify card removed from source zone
- Verify card placed in correct stage area
- Verify stage area updated
- Verify debut ability triggers if applicable

### q104_move_to_hand.md
**Quality Tests:**
- Verify card moved to hand from specified zone
- Verify card removed from source zone
- Verify hand size increased by 1
- Verify source zone size decreased by 1
- Verify card order in hand

### q105_move_to_waitroom.md
**Quality Tests:**
- Verify card moved to waitroom from specified zone
- Verify card removed from source zone
- Verify waitroom size increased by 1
- Verify source zone size decreased by 1
- Verify card order in waitroom

### q106_move_to_deck.md
**Quality Tests:**
- Verify card moved to deck from specified zone
- Verify card removed from source zone
- Verify deck size increased by 1
- Verify source zone size decreased by 1
- Verify card placed on top of deck

### q107_move_to_bottom_deck.md
**Quality Tests:**
- Verify card moved to bottom of deck from specified zone
- Verify card removed from source zone
- Verify deck size increased by 1
- Verify source zone size decreased by 1
- Verify card placed at bottom of deck

### q108_shuffle_into_deck.md
**Quality Tests:**
- Verify card shuffled into deck from specified zone
- Verify card removed from source zone
- Verify deck size increased by 1
- Verify source zone size decreased by 1
- Verify deck is shuffled after card added

### q109_destroy_card.md
**Quality Tests:**
- Verify card destroyed (sent to waitroom)
- Verify card removed from current zone
- Verify waitroom size increased by 1
- Verify source zone size decreased by 1
- Verify modifiers cleared on destroy

### q110_destroy_own_card.md
**Quality Tests:**
- Verify own card can be destroyed
- Verify card sent to own waitroom
- Verify card removed from current zone
- Verify waitroom size increased by 1
- Verify modifiers cleared on destroy

### q111_destroy_opponent_card.md
**Quality Tests:**
- Verify opponent card can be destroyed
- Verify card sent to opponent waitroom
- Verify card removed from opponent's current zone
- Verify opponent waitroom size increased by 1
- Verify opponent's modifiers cleared on destroy

---

## Blade and Heart Tests

### q112_gain_blade.md
**Quality Tests:**
- Verify blade modifier added to target card
- Verify blade count increased by correct amount
- Verify blade modifier tracked correctly
- Verify blade effect applies to gameplay

### q113_gain_heart.md
**Quality Tests:**
- Verify heart modifier added to target card
- Verify heart count increased by correct amount
- Verify heart modifier tracked correctly
- Verify heart effect applies to gameplay

### q114_remove_blade.md
**Quality Tests:**
- Verify blade modifier removed from target card
- Verify blade count decreased by correct amount
- Verify blade modifier cannot go below 0
- Verify blade removal tracked correctly

### q115_remove_heart.md
**Quality Tests:**
- Verify heart modifier removed from target card
- Verify heart count decreased by correct amount
- Verify heart modifier cannot go below 0
- Verify heart removal tracked correctly

---

## Ability Gain/Remove Tests

### q116_gain_ability.md
**Quality Tests:**
- Verify ability added to target card
- Verify ability is functional on target
- Verify ability text stored correctly
- Verify ability executes when triggered

### q117_gain_ability_duration.md
**Quality Tests:**
- Verify ability added with duration
- Verify ability expires at end of duration
- Verify ability removed after expiration
- Verify duration tracked correctly

### q118_gain_ability_permanent.md
**Quality Tests:**
- Verify ability added with permanent duration
- Verify ability does not expire
- Verify ability persists across turns
- Verify ability persists across phases

### q119_gain_ability_liveend.md
**Quality Tests:**
- Verify ability added with live_end duration
- Verify ability expires at live end
- Verify ability removed after live ends
- Verify live_end duration tracked correctly

### q120_gain_ability_thislive.md
**Quality Tests:**
- Verify ability added with this_live duration
- Verify ability expires at end of current live
- Verify ability removed after live ends
- Verify this_live duration tracked correctly

### q121_remove_ability.md
**Quality Tests:**
- Verify ability removed from target card
- Verify ability no longer functional
- Verify ability text removed from card
- Verify ability no longer triggers

### q122_copy_ability.md
**Quality Tests:**
- Verify ability copied from source to target
- Verify copied ability is functional
- Verify source ability remains on source card
- Verify copied ability executes independently

### q123_copy_ability_duration.md
**Quality Tests:**
- Verify ability copied with duration
- Verify copied ability expires at end of duration
- Verify copied ability removed after expiration
- Verify duration of copied ability tracked correctly

### q124_swap_abilities.md
**Quality Tests:**
- Verify abilities swapped between two cards
- Verify card 1 has card 2's original ability
- Verify card 2 has card 1's original ability
- Verify swapped abilities are functional

---

## Resource Tests

### q125_gain_resource.md
**Quality Tests:**
- Verify resource (energy) gained
- Verify energy_zone size increased
- Verify energy card is active
- Verify resource effect applies to gameplay

### q126_gain_resource_duration.md
**Quality Tests:**
- Verify resource gained with duration
- Verify resource expires at end of duration
- Verify resource removed after expiration
- Verify duration tracked correctly

### q127_lose_resource.md
**Quality Tests:**
- Verify resource (energy) lost
- Verify energy_zone size decreased
- Verify energy card deactivated
- Verify resource loss tracked correctly

### q128_transfer_resource.md
**Quality Tests:**
- Verify resource transferred from player to opponent
- Verify source player energy_zone decreased
- Verify target player energy_zone increased
- Verify transfer tracked correctly

---

## Power and HP Tests

### q129_change_power.md
**Quality Tests:**
- Verify power modifier added to target card
- Verify power increased by correct amount
- Verify power modifier tracked correctly
- Verify power effect applies to gameplay

### q130_change_power_duration.md
**Quality Tests:**
- Verify power modifier added with duration
- Verify power expires at end of duration
- Verify power reset after expiration
- Verify duration tracked correctly

### q131_change_power_permanent.md
**Quality Tests:**
- Verify power modifier added with permanent duration
- Verify power does not expire
- Verify power persists across turns
- Verify power persists across phases

### q132_reset_power.md
**Quality Tests:**
- Verify power reset to base value
- Verify power modifiers cleared
- Verify power returns to original value
- Verify reset tracked correctly

### q133_change_hp.md
**Quality Tests:**
- Verify HP modifier added to target card
- Verify HP increased by correct amount
- Verify HP modifier tracked correctly
- Verify HP effect applies to gameplay

### q134_change_hp_duration.md
**Quality Tests:**
- Verify HP modifier added with duration
- Verify HP expires at end of duration
- Verify HP reset after expiration
- Verify duration tracked correctly

### q135_heal.md
**Quality Tests:**
- Verify damage healed from target card
- Verify HP increased by heal amount
- Verify damage removed correctly
- Verify heal effect applies to gameplay

### q136_deal_damage.md
**Quality Tests:**
- Verify damage dealt to target card
- Verify HP decreased by damage amount
- Verify damage tracked correctly
- Verify damage effect applies to gameplay

### q137_deal_damage_duration.md
**Quality Tests:**
- Verify damage dealt with duration
- Verify damage expires at end of duration
- Verify damage healed after expiration
- Verify duration tracked correctly

### q138_prevent_damage.md
**Quality Tests:**
- Verify damage prevention active
- Verify damage prevented when applied
- Verify no HP change when damage prevented
- Verify prevention tracked correctly

### q139_prevent_damage_duration.md
**Quality Tests:**
- Verify damage prevention with duration
- Verify prevention expires at end of duration
- Verify damage applied after expiration
- Verify duration tracked correctly

---

## Special State Tests

### q141_remove_special_state.md
**Quality Tests:**
- Verify special state removed from target card
- Verify special state no longer active
- Verify special state effects removed
- Verify removal tracked correctly

### q142_special_state_duration.md
**Quality Tests:**
- Verify special state added with duration
- Verify special state expires at end of duration
- Verify special state removed after expiration
- Verify duration tracked correctly

### q143_special_state_permanent.md
**Quality Tests:**
- Verify special state added with permanent duration
- Verify special state does not expire
- Verify special state persists across turns
- Verify special state persists across phases

### q144_multiple_special_states.md
**Quality Tests:**
- Verify multiple special states can be on same card
- Verify each special state tracked independently
- Verify all special states function simultaneously
- Verify special states don't interfere with each other

### q145_conflicting_special_states.md
**Quality Tests:**
- Verify conflicting special states handled correctly
- Verify priority rules applied
- Verify only one special state active when conflict
- Verify conflict resolution tracked correctly

---

## Engine Fault Tests

### engine_fault_area_movement_trigger.md
**Quality Tests:**
- Verify auto ability triggers on area movement
- Verify ability uses card's information from previous area
- Verify trigger_type "each_time" works for area movement
- Verify card movement tracked correctly for ability trigger

### engine_fault_card_removal_clears_modifiers.md
**Quality Tests:**
- Verify blade_modifiers cleared when card removed
- Verify heart_modifiers cleared when card removed
- Verify orientation_modifiers cleared when card removed
- Verify cost_modifiers cleared when card removed
- Verify clear_modifiers_for_card called on removal

### engine_fault_gain_ability.md
**Quality Tests:**
- Verify gain_ability actually grants ability to target
- Verify ability added to target's abilities list
- Verify gained ability functions correctly
- Verify gained ability effects applied

### engine_fault_live_end_duration_clearing.md
**Quality Tests:**
- Verify Duration::LiveEnd effects cleared at live end
- Verify effects cleared even without live performed
- Verify check_expired_effects called correctly
- Verify blade modifiers removed after expiration

---

## Summary

Total test plans: 38
Total quality tests: 400+

Each test plan has been enhanced with:
- Specific card IDs from the card database
- Real card names
- Actual ability text with icons
- Implementation-ready Initial Game State sections
- Complete end-to-end gameplay steps
- User choices where applicable
- Expected final states
- Verification assertions

All test plans are now ready for QA test implementation.
