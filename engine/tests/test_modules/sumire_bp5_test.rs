use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

fn heart02_mod(game: &TestGame, card_id: i16) -> i32 {
    game.state
        .mods
        .get_heart_modifier(card_id, HeartColor::Heart02)
}

/// Fill P1's main deck with enough filler cards for draws.
fn fill_deck(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
}

#[test]
fn test_sumire_area_move_triggers_draw_and_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp5-004-R+");
    let kinako_mover = game.id("PL!SP-bp5-006-R");
    fill_deck(&mut game);
    game.give_energy(15);
    // Place Sumire on stage, plus a mover card that can swap with her
    game.add_to_stage(rabuka_engine::zones::MemberArea::LeftSide, sumire);
    game.add_to_stage(rabuka_engine::zones::MemberArea::Center, kinako_mover);

    // Activate 起動 きな子 → swap with Sumire → Sumire moves
    game.activate_ability(kinako_mover);
    game.drain_auto_ability_choices();
    let acts = game.generated_actions();
    let left_idx = acts
        .iter()
        .position(|a| a.parameters.as_ref().and_then(|p| p.stage_area.as_deref()) == Some("left"))
        .unwrap();
    game.select_generated(left_idx);
    game.drain_auto_ability_choices();

    // Sumire moved via game action → TAS fired inside choice handler → ability resolved
    let hand_size = game.state.player1.hand.cards.len();
    assert_eq!(hand_size, 1, "One card drawn after area move");
    assert_eq!(
        heart02_mod(&game, sumire),
        1,
        "Gained 1 heart02 after area move"
    );
}

#[test]
fn test_sumire_energy_effect_triggers() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp5-004-R+");
    let hazuki = game.id("PL!SP-pb1-005-R"); // 葉月恋: debut places energy from energy deck
    fill_deck(&mut game);
    fill_energy_deck(&mut game);
    game.give_energy(15);
    // Place Sumire on stage, then play Hazuki to trigger energy placement
    game.add_to_stage(MemberArea::Center, sumire);
    game.state.player1.hand.cards.push(hazuki);

    game.play_to_stage(hazuki, MemberArea::RightSide);
    game.drain_auto_ability_choices();

    // Hazuki's debut places energy → Sumire's ability fires automatically
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "energy placement by effect should trigger draw"
    );
    assert_eq!(
        heart02_mod(&game, sumire),
        1,
        "should get +1 heart02 from energy effect"
    );
}

#[test]
fn test_sumire_energy_phase_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp5-004-R+");
    fill_deck(&mut game);
    game.give_energy(15);
    // Place Sumire directly (no auto-trigger on placement)
    game.add_to_stage(MemberArea::Center, sumire);
    // Energy phase places energy (NOT by card effect)
    game.state.player1.draw_energy();
    game.state.batch_movements.clear();

    let before_hand = game.state.player1.hand.cards.len();
    let player_id = game.state.player1.id.clone();
    let _ = rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(
        &mut game.state,
        &player_id,
    );
    assert_eq!(
        heart02_mod(&game, sumire),
        0,
        "energy phase should NOT trigger"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        before_hand,
        "no draw from energy phase"
    );
}

#[test]
fn test_sumire_opponent_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp5-004-R+");
    fill_deck(&mut game);
    game.give_energy(15);
    // Place Sumire directly (no auto-trigger on placement)
    game.add_to_stage(MemberArea::Center, sumire);
    // Simulate opponent's card effect moving Sumire
    game.state
        .push_movement_event(sumire, "stage", "stage", Some(sumire), "p2", true);
    game.state.batch_movements.clear();

    let before_hand = game.state.player1.hand.cards.len();
    let player_id = game.state.player1.id.clone();
    let _ = rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(
        &mut game.state,
        &player_id,
    );
    // Condition: push_movement_event with mover=opponent → area_ok fails
    // no energy event → energy_ok fails → condition fails
    assert_eq!(
        heart02_mod(&game, sumire),
        0,
        "opponent effect should NOT trigger"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        before_hand,
        "no draw from opponent"
    );
}

/// Energy placed by opponent's effect → self_effect_only check must reject it.
#[test]
fn test_sumire_opponent_energy_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp5-004-R+");
    fill_deck(&mut game);
    game.give_energy(15);
    game.add_to_stage(MemberArea::Center, sumire);
    // Simulate opponent's effect placing energy
    game.state
        .push_movement_event(-1, "energy_deck", "energy", None, "p2", true);

    let before_hand = game.state.player1.hand.cards.len();
    let player_id = game.state.player1.id.clone();
    let _ = rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(
        &mut game.state,
        &player_id,
    );
    // self_effect_only check: movement event player must be "p1" for energy_ok
    assert_eq!(
        heart02_mod(&game, sumire),
        0,
        "opponent energy must NOT trigger"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        before_hand,
        "no draw from opponent energy"
    );
}

/// Different card area move by own effect → DOES trigger (engine checks
/// push_movement_event but not WHICH card moved). Card text says
/// "このメンバー" (this member) but the movement condition doesn't validate it.
#[test]
fn test_sumire_other_card_move_triggers() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp5-004-R+");
    let other = game.id("PL!-sd1-010-SD");
    fill_deck(&mut game);
    game.give_energy(15);
    game.add_to_stage(MemberArea::Center, sumire);
    // Sumire's ability says "このメンバーがエリアを移動する" = THIS MEMBER must
    // move.  When a different card moves, Sumire should NOT trigger.
    game.state
        .push_movement_event(other, "stage", "stage", Some(other), "p1", true);
    game.state.batch_movements.clear();

    let before_hand = game.state.player1.hand.cards.len();
    let player_id = game.state.player1.id.clone();
    let _ = rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(
        &mut game.state,
        &player_id,
    );
    game.state.process_pending_auto_abilities(&player_id);

    // Since a different card moved, Sumire's "this member" check should NOT trigger.
    assert_eq!(
        heart02_mod(&game, sumire),
        0,
        "different-card move should NOT trigger Sumire (card identity check)"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        before_hand,
        "no draw from different-card move"
    );
}

#[test]
fn test_sumire_use_limit_blocks_second() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp5-004-R+");
    fill_deck(&mut game);
    game.give_energy(15);
    game.add_to_stage(MemberArea::Center, sumire);

    let player_id = game.state.player1.id.clone();
    // First trigger via area move (simulated own-card-effect area move)
    game.state.cards_moved_this_turn.push(sumire);
    game.state
        .push_movement_event(sumire, "stage", "stage", Some(sumire), "p1", true);
    game.state.batch_movements.clear();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &player_id);
    game.state.process_pending_auto_abilities(&player_id);
    // First trigger consumed use_limit and granted heart02.
    assert_eq!(
        heart02_mod(&game, sumire),
        1,
        "heart02 from area move trigger"
    );

    // Now try to trigger via energy — should be blocked by use_limit.
    game.state
        .push_movement_event(-1, "energy_deck", "energy", None, "p1", true);
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &player_id);
    assert_eq!(
        heart02_mod(&game, sumire),
        1,
        "heart02 from first trigger persists, second blocked"
    );
}

#[test]
fn test_sumire_no_event_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp5-004-R+");
    fill_deck(&mut game);
    game.give_energy(15);
    // Place Sumire directly, no play trigger
    game.add_to_stage(MemberArea::Center, sumire);
    // Clear all event tracking
    game.state.batch_movements.clear();

    let before_hand = game.state.player1.hand.cards.len();
    let player_id = game.state.player1.id.clone();
    let _ = rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(
        &mut game.state,
        &player_id,
    );
    assert_eq!(heart02_mod(&game, sumire), 0, "no event = no trigger");
    assert_eq!(
        game.state.player1.hand.cards.len(),
        before_hand,
        "no event = no draw"
    );
}

/// Helper: fill main deck and give energy for activation-cost cards.
fn fill_deck_and_energy(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..15 {
        game.state.player1.energy_zone.cards.push(filler);
    }
    game.state.player1.energy_zone.set_active_count(15);
}

fn fill_energy_deck(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.energy_deck.cards.push(filler);
    }
}

/// Sumire moves via きな子's 起動 position change (swap) → triggers.
/// きな子 (PL!SP-bp5-006-R) has a non-optional 起動 position change that
/// moves herself and swaps with the destination occupant.
/// Setup: きな子 at Left, Sumire at Right → きな子 moves Right → swap.
#[test]
fn sumire_kinako_swap_triggers_sumire() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sumire = game.id("PL!SP-bp5-004-R+");
    let kinako = game.id("PL!SP-bp5-006-R");
    fill_deck_and_energy(&mut game);

    game.state.player1.stage.stage = [kinako, -1, sumire];

    // Activate きな子's 起動 ability (costs deck top 3 to discard)
    game.activate_ability(kinako);
    game.drain_auto_ability_choices();

    // Should have a position|destination choice — pick "right" to swap with Sumire
    let actions = game.generated_actions();
    let right_idx = actions
        .iter()
        .position(|a| a.parameters.as_ref().and_then(|p| p.stage_area.as_deref()) == Some("right"))
        .expect("right should be a valid destination");
    game.select_generated(right_idx);
    game.drain_auto_ability_choices();

    // Sumire moved from right to left → should trigger
    // きな子 activated from Left → moved to Right → swapped with Sumire
    assert_eq!(
        game.state.player1.stage.get_area(MemberArea::LeftSide),
        Some(sumire),
        "sumire should be at left after swap"
    );
    assert_eq!(
        game.state.player1.stage.get_area(MemberArea::RightSide),
        Some(kinako),
        "kinako should be at right after swap"
    );

    let hand_size = game.state.player1.hand.cards.len();
    assert_eq!(hand_size, 1, "Sumire draws 1 card after her own area move");
    assert_eq!(
        heart02_mod(&game, sumire),
        1,
        "Sumire gains heart02 after area move"
    );
}

/// Sumire does NOT move (きな子 moves to empty slot) → no trigger.
/// きな子 at Left moves to empty Center. Sumire at Right stays → no trigger.
#[test]
fn sumire_kinako_empty_move_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sumire = game.id("PL!SP-bp5-004-R+");
    let kinako = game.id("PL!SP-bp5-006-R");
    fill_deck_and_energy(&mut game);

    game.state.player1.stage.stage = [kinako, -1, sumire];

    game.activate_ability(kinako);
    game.drain_auto_ability_choices();

    // Move きな子 to center (empty) — Sumire doesn't move
    let actions = game.generated_actions();
    let center_idx = actions
        .iter()
        .position(|a| a.parameters.as_ref().and_then(|p| p.stage_area.as_deref()) == Some("center"))
        .expect("center should be a valid destination");
    game.select_generated(center_idx);
    game.drain_auto_ability_choices();

    assert_eq!(
        heart02_mod(&game, sumire),
        0,
        "Sumire should NOT trigger — she did not move"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        0,
        "no draw — Sumire did not move"
    );
}

/// Real energy-placing card effect: Hazuki Kano (PL!SP-pb1-005-R) debut
/// places 1 energy from energy deck → Sumire's "energy placed by own effect" triggers.
#[test]
fn sumire_hazuki_energy_placement_triggers() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sumire = game.id("PL!SP-bp5-004-R+");
    let hazuki = game.id("PL!SP-pb1-005-R");
    fill_deck_and_energy(&mut game);
    fill_energy_deck(&mut game);

    // Sumire on stage, Hazuki in hand
    game.state.player1.stage.stage = [sumire, -1, -1];
    game.state.player1.hand.cards.push(hazuki);

    let hand_before = game.state.player1.hand.cards.len();

    // Play Hazuki to stage — her debut places energy from energy deck
    game.play_to_stage(hazuki, MemberArea::RightSide);
    game.drain_auto_ability_choices();

    // Sumire should have triggered from the energy placement
    assert_eq!(
        heart02_mod(&game, sumire),
        1,
        "Sumire gains heart02 from energy placement"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before, // play costs 1 hand, Sumire draws 1 = net 0
        "Sumire draws 1 card — hand count stays same after playing Hazuki"
    );
}

/// Use limit (1/turn): position change triggers once, then energy placement
/// in the same turn does NOT trigger again.
#[test]
fn sumire_position_change_then_energy_use_limit_blocks_second() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sumire = game.id("PL!SP-bp5-004-R+");
    let kinako = game.id("PL!SP-bp5-006-R");
    fill_deck_and_energy(&mut game);

    game.state.player1.stage.stage = [kinako, -1, sumire];

    // First trigger: きな子 swaps with Sumire → Sumire moves
    game.activate_ability(kinako);
    game.drain_auto_ability_choices();
    let actions = game.generated_actions();
    let right_idx = actions
        .iter()
        .position(|a| a.parameters.as_ref().and_then(|p| p.stage_area.as_deref()) == Some("right"))
        .expect("right should be valid");
    game.select_generated(right_idx);
    game.drain_auto_ability_choices();

    assert_eq!(
        heart02_mod(&game, sumire),
        1,
        "Sumire gains heart02 from first trigger (position change)"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "Sumire draws 1 from first trigger"
    );

    // Second event: opponent places energy (normally would trigger if not used)
    game.state
        .push_movement_event(-1, "energy_deck", "energy", None, "p1", true);

    let player_id = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &player_id);
    game.state.process_pending_auto_abilities(&player_id);

    // Use_limit=1 should prevent a second trigger
    assert_eq!(
        heart02_mod(&game, sumire),
        1,
        "heart02 unchanged — second trigger blocked by use_limit"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "no extra draw — second trigger blocked"
    );
}

/// Turn 2: use_limit resets — Sumire can trigger again.
#[test]
fn sumire_turn_two_limit_resets() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sumire = game.id("PL!SP-bp5-004-R+");
    let kinako = game.id("PL!SP-bp5-006-R");
    fill_deck_and_energy(&mut game);

    game.state.player1.stage.stage = [kinako, -1, sumire];

    // Turn 1: trigger via position change
    game.activate_ability(kinako);
    game.drain_auto_ability_choices();
    let actions = game.generated_actions();
    let right_idx = actions
        .iter()
        .position(|a| a.parameters.as_ref().and_then(|p| p.stage_area.as_deref()) == Some("right"))
        .expect("right should be valid");
    game.select_generated(right_idx);
    game.drain_auto_ability_choices();

    assert_eq!(heart02_mod(&game, sumire), 1, "Turn 1: Sumire triggers");

    // Clear per-turn state to simulate turn boundary
    game.state.position_change_events.clear();
    game.state.position_change_occurred_this_turn = false;
    game.state.cards_moved_this_turn.clear();
    game.state.turn_area_movements.clear();

    // Increment turn to reset use_limit
    game.state.turn_number += 1;

    // Reset stage for a second swap
    game.state.player1.stage.stage = [kinako, -1, sumire];

    // Activate きな子 again on turn 2 — use_limit should have reset
    game.activate_ability(kinako);
    game.drain_auto_ability_choices();
    let actions = game.generated_actions();
    let right_idx = actions
        .iter()
        .position(|a| a.parameters.as_ref().and_then(|p| p.stage_area.as_deref()) == Some("right"))
        .expect("right should be valid");
    game.select_generated(right_idx);
    game.drain_auto_ability_choices();

    assert_eq!(
        heart02_mod(&game, sumire),
        2,
        "Turn 2: Sumire triggers again — use_limit resets across turns"
    );
}

/// Keke debut with optional position change — skip it → Sumire should not trigger.
#[test]
fn sumire_keke_skip_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sumire = game.id("PL!SP-bp5-004-R+");
    let keke = game.id("PL!SP-bp4-013-N");
    fill_deck_and_energy(&mut game);

    game.state.player1.stage.stage = [sumire, -1, -1];
    game.state.player1.hand.cards.push(keke);

    let hand_before = game.state.player1.hand.cards.len();

    // Play Keke and skip her optional position change
    game.play_to_stage(keke, MemberArea::RightSide);
    game.drain_auto_ability_choices();

    // Keke's optional position change prompt: skip it
    if game.has_pending_choice() {
        match game.get_pending_choice().clone() {
            Choice::SelectTarget {
                target, allow_skip, ..
            } if target == "position|destination" => {
                assert!(allow_skip, "Keke's position change should be skippable");
                // Find and click the Skip action
                let actions = game.generated_actions();
                let skip_action = actions.iter().find(|a| {
                    a.description.contains("Skip")
                        || a.action_type == rabuka_engine::game_setup::ActionType::ChoiceSkip
                });
                if let Some(action) = skip_action {
                    let params = action.parameters.as_ref().unwrap();
                    rabuka_engine::turn::TurnEngine::resume_with_choice(
                        &mut game.state,
                        params.card_id,
                        params.card_indices.clone(),
                    )
                    .expect("skip should succeed");
                }
                game.drain_auto_ability_choices();
            }
            _other => {
                // Not a position destination choice, drain it
                game.select_indices(&[]);
                game.drain_auto_ability_choices();
            }
        }
    }

    assert_eq!(
        heart02_mod(&game, sumire),
        0,
        "Sumire should NOT trigger — no position change occurred"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before - 1, // played Keke from hand, no draw
        "no extra draw"
    );
}

/// Opponent's card effect moves Sumire → NOT triggered (self_effect_only).
#[test]
fn sumire_opponent_effect_move_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sumire = game.id("PL!SP-bp5-004-R+");
    fill_deck_and_energy(&mut game);
    game.state.player1.stage.stage = [sumire, -1, -1];

    // Simulate opponent's card effect causing an area move
    game.state.cards_moved_this_turn.push(sumire);
    game.state
        .push_movement_event(sumire, "stage", "stage", Some(sumire), "p2", true);
    game.state.batch_movements.clear();

    let player_id = game.state.player1.id.clone();
    let _ = rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(
        &mut game.state,
        &player_id,
    );
    game.state.process_pending_auto_abilities(&player_id);

    assert_eq!(
        heart02_mod(&game, sumire),
        0,
        "opponent effect moving Sumire should NOT trigger (self_effect_only)"
    );
}

/// Formation change (藤島慈 PL!HS-bp2-006-R) — 3 members all move.
/// Sumire moves from Left → Center → triggers.
#[test]
fn sumire_formation_change_triggers() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sumire = game.id("PL!SP-bp5-004-R+");
    let chii = game.id("PL!HS-bp2-006-R");
    // Use a filler with an auto-ability that will help verify movement
    let filler = game.id("PL!-sd1-010-SD");
    fill_deck_and_energy(&mut game);

    // Setup: Sumire Left, filler Center; 慈 in hand for debut
    game.state.player1.stage.stage = [sumire, filler, -1];
    game.state.player1.hand.cards.push(chii);

    // Play 慈 to stage — debut triggers formation change
    game.play_to_stage(chii, MemberArea::RightSide);
    game.drain_auto_ability_choices();

    // Formation change: 3 members on stage, 3 sequential destination choices
    // Choice 1: Sumire at Left — pick Center to move her
    if game.has_pending_choice() {
        match game.get_pending_choice().clone() {
            Choice::SelectTarget { target, .. } if target == "position|destination" => {
                let acts = game.generated_actions();
                // Pick index 1 = "center" (skip "left" which is her current position)
                assert!(acts.len() >= 2, "need at least 2 options");
                // Find the option with stage_area="center"
                let idx = acts
                    .iter()
                    .position(|a| {
                        a.parameters.as_ref().and_then(|p| p.stage_area.as_deref())
                            == Some("center")
                    })
                    .unwrap_or(1);
                game.select_generated(idx);
                game.drain_auto_ability_choices();
            }
            _ => {
                game.select_indices(&[]);
                game.drain_auto_ability_choices();
            }
        }
    }

    // Choice 2: filler at Center — pick Right
    if game.has_pending_choice() {
        match game.get_pending_choice().clone() {
            Choice::SelectTarget { target, .. } if target == "position|destination" => {
                let acts = game.generated_actions();
                let idx = acts
                    .iter()
                    .position(|a| {
                        a.parameters.as_ref().and_then(|p| p.stage_area.as_deref()) == Some("right")
                    })
                    .unwrap_or(0);
                game.select_generated(idx);
                game.drain_auto_ability_choices();
            }
            _ => {
                game.select_indices(&[]);
                game.drain_auto_ability_choices();
            }
        }
    }

    // Choice 3: 慈 at Right — pick Left (remaining)
    if game.has_pending_choice() {
        match game.get_pending_choice().clone() {
            Choice::SelectTarget { target, .. } if target == "position|destination" => {
                let acts = game.generated_actions();
                let idx = acts
                    .iter()
                    .position(|a| {
                        a.parameters.as_ref().and_then(|p| p.stage_area.as_deref()) == Some("left")
                    })
                    .unwrap_or(0);
                game.select_generated(idx);
                game.drain_auto_ability_choices();
            }
            _ => {
                game.select_indices(&[]);
                game.drain_auto_ability_choices();
            }
        }
    }

    // Sumire moved from Left → Center → triggers
    assert_eq!(
        heart02_mod(&game, sumire),
        1,
        "Sumire moved during formation change → triggers"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "Sumire draws 1 from formation change movement"
    );
    // Verify final positions match player's selections:
    //   Sumire: Left → Center
    //   Filler: Center → Right
    //   慈:     Right → Left
    assert_eq!(
        game.state.player1.stage.get_area(MemberArea::LeftSide),
        Some(chii),
        "慈 should be at Left after formation change"
    );
    assert_eq!(
        game.state.player1.stage.get_area(MemberArea::Center),
        Some(sumire),
        "Sumire should be at Center after formation change"
    );
    assert_eq!(
        game.state.player1.stage.get_area(MemberArea::RightSide),
        Some(filler),
        "Filler should be at Right after formation change"
    );
}
