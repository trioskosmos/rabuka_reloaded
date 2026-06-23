use crate::helpers::*;
use rabuka_engine::ability::condition::ConditionContext;
use rabuka_engine::ability::enums::ConditionType;
use rabuka_engine::card::Condition;

fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn source_dest_condition(source: &str, destination: &str, card_type: &str) -> Condition {
    Condition {
        condition_type: Some(ConditionType::CardCountCondition),
        count: Some(1),
        operator: Some(">=".to_string()),
        source: Some(source.to_string()),
        destination: Some(destination.to_string()),
        card_type: Some(card_type.to_string()),
        ..Default::default()
    }
}

fn push_movement(game: &mut TestGame, card_id: i16, from: &str, to: &str) {
    let pid = game.state.player1.id.clone();
    game.state
        .push_movement_event(card_id, from, to, None, &pid, false);
}

/// Test: check_invalid_live_cards records turn_movements when discarding
/// a member card from the live card zone.
#[test]
fn invalid_live_card_discard_records_turn_movements() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!-sd1-010-SD");
    let filler = member;

    game.state.player1.live_card_zone.cards.push(member);
    game.state.player1.stage.stage = [filler, -1, -1];
    fill_decks(&mut game, filler);

    assert!(game.state.player1.live_card_zone.cards.contains(&member));
    assert!(game.state.turn_movements.is_empty());

    rabuka_engine::turn::TurnEngine::check_timing(&mut game.state);

    // Card moved from live_card_zone to waitroom
    assert!(!game.state.player1.live_card_zone.cards.contains(&member));
    assert!(game.state.player1.waitroom.cards.contains(&member));

    // turn_movements captured the event
    let movement = game.state.turn_movements.last();
    assert!(movement.is_some());
    let m = movement.unwrap();
    assert_eq!(m.moved_card_id, member);
    assert_eq!(m.source_zone, "live_card_zone");
    assert_eq!(m.dest_zone, "waitroom");
}

/// Test: source+destination condition passes when turn_movements has a
/// matching movement event.
#[test]
fn source_dest_condition_matches_turn_movements() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, member);

    push_movement(&mut game, member, "live_card_zone", "waitroom");
    game.state.player1.waitroom.cards.push(member);

    let cond = source_dest_condition("live_card_zone", "discard", "member_card");
    let ctx = ConditionContext::new(&game.state);
    assert!(ctx.evaluate_condition(&cond));
}

/// Test: source+destination condition fails when the movement source zone
/// does not match.
#[test]
fn source_dest_condition_wrong_source_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, member);

    // Movement from stage, not live_card_zone
    push_movement(&mut game, member, "stage", "waitroom");
    game.state.player1.waitroom.cards.push(member);

    let cond = source_dest_condition("live_card_zone", "discard", "member_card");
    let ctx = ConditionContext::new(&game.state);
    assert!(!ctx.evaluate_condition(&cond));
}

/// Test: source+destination condition fails when the card is not in
/// the destination zone.
#[test]
fn source_dest_condition_card_not_in_dest_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, member);

    push_movement(&mut game, member, "live_card_zone", "waitroom");
    // Card NOT placed in waitroom → zone-transition filter fails

    let cond = source_dest_condition("live_card_zone", "discard", "member_card");
    let ctx = ConditionContext::new(&game.state);
    assert!(!ctx.evaluate_condition(&cond));
}

/// Test: source+destination condition with count > 1 fails when
/// only 1 matching movement exists.
#[test]
fn source_dest_condition_needs_multiple_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, member);

    push_movement(&mut game, member, "live_card_zone", "waitroom");
    game.state.player1.waitroom.cards.push(member);

    let mut cond = source_dest_condition("live_card_zone", "discard", "member_card");
    cond.count = Some(2);
    let ctx = ConditionContext::new(&game.state);
    assert!(!ctx.evaluate_condition(&cond));
}

/// Test: turn_movements are cleared at turn start.
#[test]
fn turn_movements_cleared_at_turn_start() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, member);

    push_movement(&mut game, member, "live_card_zone", "waitroom");
    assert_eq!(game.state.turn_movements.len(), 1);

    game.state.clear_card_movement_tracking();
    assert!(game.state.turn_movements.is_empty());
}

/// Test: energy cards in live_card_zone go to energy_deck, not waitroom.
#[test]
fn invalid_energy_card_in_live_zone_goes_to_energy_deck() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let energy = game.id("LL-E-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.live_card_zone.cards.push(energy);
    game.state.player1.stage.stage = [filler, -1, -1];
    fill_decks(&mut game, filler);

    rabuka_engine::turn::TurnEngine::check_timing(&mut game.state);

    assert!(!game.state.player1.live_card_zone.cards.contains(&energy));
    assert!(game.state.player1.energy_deck.cards.contains(&energy));
}
