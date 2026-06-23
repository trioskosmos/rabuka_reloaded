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
fn test_sumire_area_move_triggers_draw_and_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp5-004-R+");
    fill_deck(&mut game);
    game.give_energy(15);
    game.add_to_stage(MemberArea::Center, sumire);
    // Simulate area move by own card effect — this SHOULD trigger Sumire's auto ability.
    game.state.cards_moved_this_turn.insert(sumire);
    game.state
        .push_movement_event(sumire, "stage", "stage", Some(sumire), "player1", true);
    game.state.batch_movements.clear();

    let player_id = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &player_id);
    game.state.process_pending_auto_abilities(&player_id);

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
    fill_deck(&mut game);
    game.give_energy(15);
    // Place Sumire on stage WITHOUT triggering auto-abilities (add_to_stage bypasses triggers).
    // This leaves the 1/turn available for the energy trigger test.
    game.add_to_stage(MemberArea::Center, sumire);
    // Simulate energy placed by card effect
    game.state
        .push_movement_event(-1, "energy_deck", "energy", None, "player1", true);

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
        .push_movement_event(sumire, "stage", "stage", Some(sumire), "player2", true);
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
        .push_movement_event(-1, "energy_deck", "energy", None, "player2", true);

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
        .push_movement_event(other, "stage", "stage", Some(other), "player1", true);
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
    game.state.cards_moved_this_turn.insert(sumire);
    game.state
        .push_movement_event(sumire, "stage", "stage", Some(sumire), "player1", true);
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
        .push_movement_event(-1, "energy_deck", "energy", None, "player1", true);
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
