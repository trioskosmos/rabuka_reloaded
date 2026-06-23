use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

fn heart02_mod(game: &TestGame, card_id: i16) -> i32 {
    game.state
        .mods
        .get_heart_modifier(card_id, HeartColor::Heart02)
}

#[test]
fn test_sumire_play_triggers_draw_and_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp5-004-R+");
    game.give_energy(15);
    game.state.player1.hand.cards.push(sumire);
    game.play_to_stage(sumire, MemberArea::Center);
    let before_hand = game.state.player1.hand.cards.len();
    let player_id = game.state.player1.id.clone();
    let _ = rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(
        &mut game.state,
        &player_id,
    );
    let hand_after = game.state.player1.hand.cards.len();
    assert_eq!(
        hand_after,
        before_hand + 1,
        "play should trigger draw. before={} after={}",
        before_hand,
        hand_after
    );
    assert_eq!(
        heart02_mod(&game, sumire),
        1,
        "should get +1 heart02. got={}",
        heart02_mod(&game, sumire)
    );
}

#[test]
fn test_sumire_energy_effect_triggers() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp5-004-R+");
    game.give_energy(15);
    game.state.player1.hand.cards.push(sumire);
    game.play_to_stage(sumire, MemberArea::Center);
    game.state
        .push_movement_event(-1, "energy_deck", "energy", None, "player1", true);
    game.state.batch_movements.clear();
    let before_hand = game.state.player1.hand.cards.len();
    let player_id = game.state.player1.id.clone();
    let _ = rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(
        &mut game.state,
        &player_id,
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        before_hand + 1,
        "energy placement should trigger draw"
    );
    assert_eq!(
        heart02_mod(&game, sumire),
        1,
        "should get +1 heart02 from energy"
    );
}

#[test]
fn test_sumire_energy_phase_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp5-004-R+");
    game.give_energy(15);
    game.state.player1.hand.cards.push(sumire);
    game.play_to_stage(sumire, MemberArea::Center);
    game.state.batch_movements.clear();
    game.state.player1.draw_energy();
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
    game.give_energy(15);
    game.state.player1.hand.cards.push(sumire);
    game.play_to_stage(sumire, MemberArea::Center);
    game.state
        .push_movement_event(sumire, "stage", "stage", Some(sumire), "player2", true);
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
        "opponent effect should NOT trigger"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        before_hand,
        "no draw from opponent"
    );
}

#[test]
fn test_sumire_use_limit_blocks_second() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp5-004-R+");
    game.give_energy(15);
    game.state.player1.hand.cards.push(sumire);
    game.play_to_stage(sumire, MemberArea::Center);
    let player_id = game.state.player1.id.clone();
    game.state.last_energy_placed_by_effect = true;
    game.state.last_area_move_card_id = None;
    game.state.last_area_move_by_player = None;
    let _ = rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(
        &mut game.state,
        &player_id,
    );
    assert_eq!(heart02_mod(&game, sumire), 1, "first trigger");
    let after_first = game.state.player1.hand.cards.len();
    game.state.last_energy_placed_by_effect = true;
    let _ = rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(
        &mut game.state,
        &player_id,
    );
    assert_eq!(
        heart02_mod(&game, sumire),
        1,
        "second trigger blocked by use_limit"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        after_first,
        "second draw blocked"
    );
}

#[test]
fn test_sumire_no_event_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp5-004-R+");
    game.give_energy(15);
    game.state.player1.hand.cards.push(sumire);
    game.play_to_stage(sumire, MemberArea::Center);
    game.state.last_area_move_card_id = None;
    game.state.last_area_move_by_player = None;
    game.state.last_energy_placed_by_effect = false;
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
