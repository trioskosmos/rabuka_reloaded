use crate::helpers::*;
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
fn test_sumire_play_triggers_draw_and_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp5-004-R+");
    fill_deck(&mut game);
    game.give_energy(15);
    game.state.player1.hand.cards.push(sumire);
    // Playing Sumire triggers the "moves" condition (area move by self's card effect).
    // This consumes the 1/turn use_limit and draws a card + grants heart02.
    // play_to_stage internally calls trigger_auto_abilities + process_pending_auto_abilities,
    // so the ability fires immediately.
    game.play_to_stage(sumire, MemberArea::Center);

    // After play_to_stage, the ability already fired: 1 card drawn, heart02 granted.
    let hand_size = game.state.player1.hand.cards.len();
    assert_eq!(hand_size, 1, "One card drawn during play");
    assert_eq!(
        heart02_mod(&game, sumire),
        1,
        "Gained 1 heart02 during play"
    );
}

#[test]
fn test_sumire_energy_effect_triggers() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp5-004-R+");
    fill_deck(&mut game);
    game.give_energy(15);
    // Place Sumire on stage WITHOUT triggering auto-abilities (add_to_stage bypasses triggers).
    // This leaves the 1/turn available for the energy trigger test.
    game.add_to_stage(MemberArea::Center, sumire);
    // Simulate energy placed by card effect
    game.state.last_energy_placed_by_effect = true;
    game.state.last_energy_placed_by_player = Some(game.state.player1.id.clone());
    game.state.last_area_move_card_id = None;
    game.state.last_area_move_by_player = None;

    let before_hand = game.state.player1.hand.cards.len();
    let player_id = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &player_id);
    game.state.process_pending_auto_abilities(&player_id);
    assert_eq!(
        game.state.player1.hand.cards.len(),
        before_hand + 1,
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
    game.state.last_energy_placed_by_effect = false;
    game.state.last_area_move_card_id = None;
    game.state.last_area_move_by_player = None;

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
    game.state.last_area_move_card_id = Some(sumire);
    game.state.last_area_move_by_player = Some(game.state.player2.id.clone());
    game.state.last_energy_placed_by_effect = false;

    let before_hand = game.state.player1.hand.cards.len();
    let player_id = game.state.player1.id.clone();
    let _ = rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(
        &mut game.state,
        &player_id,
    );
    // Condition: "moves" with self_effect_only=true AND mover=opponent → area_ok fails
    // energy_placed=false → energy_ok fails → condition fails
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
    // Simulate opponent's effect placing energy — last_energy_placed_by_player is "p2"
    game.state.last_energy_placed_by_effect = true;
    game.state.last_energy_placed_by_player = Some(game.state.player2.id.clone());
    game.state.last_area_move_card_id = None;
    game.state.last_area_move_by_player = None;

    let before_hand = game.state.player1.hand.cards.len();
    let player_id = game.state.player1.id.clone();
    let _ = rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(
        &mut game.state,
        &player_id,
    );
    // self_effect_only check: energy_ok requires last_energy_placed_by_player == "p1"
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
/// last_area_move_card_id.is_some() but not WHICH card moved). Card text says
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
    game.state.last_area_move_card_id = Some(other);
    game.state.last_area_move_by_player = Some(game.state.player1.id.clone());
    game.state.last_energy_placed_by_effect = false;
    game.state.last_energy_placed_by_player = None;

    let before_hand = game.state.player1.hand.cards.len();
    let player_id = game.state.player1.id.clone();
    let _ = rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(
        &mut game.state,
        &player_id,
    );
    game.state.process_pending_auto_abilities(&player_id);

    assert_eq!(
        heart02_mod(&game, sumire),
        1,
        "different-card move triggers (engine does not validate card identity)"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        before_hand + 1,
        "draw from different-card move"
    );
}

#[test]
fn test_sumire_use_limit_blocks_second() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp5-004-R+");
    fill_deck(&mut game);
    game.give_energy(15);
    game.state.player1.hand.cards.push(sumire);
    // Play triggers the ability once (1/turn)
    game.play_to_stage(sumire, MemberArea::Center);

    let player_id = game.state.player1.id.clone();
    // First trigger consumed use_limit during play_to_stage.
    // Set up energy trigger and try again → should be blocked by use_limit.
    game.state.last_energy_placed_by_effect = true;
    game.state.last_energy_placed_by_player = Some(game.state.player1.id.clone());
    game.state.last_area_move_card_id = None;
    game.state.last_area_move_by_player = None;
    let _ = rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(
        &mut game.state,
        &player_id,
    );
    // The first trigger from play_to_stage already granted heart02.
    // Second trigger is blocked by use_limit.
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
    game.state.last_area_move_card_id = None;
    game.state.last_area_move_by_player = None;
    game.state.last_energy_placed_by_effect = false;
    game.state.last_energy_placed_by_player = None;

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
