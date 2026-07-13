use crate::helpers::*;
use rabuka_engine::ability::condition::ConditionContext;
use rabuka_engine::ability::enums::ConditionType;
use rabuka_engine::card::{Condition, ConditionCardType, ConditionTarget};

fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn source_dest_condition(source: &str, destination: &str) -> Condition {
    Condition {
        condition_type: Some(ConditionType::CardCountCondition),
        count: Some(1),
        operator: Some(">=".into()),
        source: Some(source.into()),
        destination: Some(destination.into()),
        card_type: Some(ConditionCardType::MemberCard),
        ..Default::default()
    }
}

fn source_dest_condition_targeted(source: &str, destination: &str) -> Condition {
    Condition {
        condition_type: Some(ConditionType::CardCountCondition),
        count: Some(1),
        operator: Some(">=".into()),
        source: Some(source.into()),
        destination: Some(destination.into()),
        card_type: Some(ConditionCardType::MemberCard),
        target: Some(ConditionTarget::Opponent),
        ..Default::default()
    }
}

fn push_movement_p1(game: &mut TestGame, card_id: i16, from: &str, to: &str) {
    let pid = game.state.player1.id.clone();
    game.state
        .push_movement_event(card_id, from, to, None, &pid, false);
}

fn push_movement_p2(game: &mut TestGame, card_id: i16, from: &str, to: &str) {
    let pid = game.state.player2.id.clone();
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

    assert!(!game.state.player1.live_card_zone.cards.contains(&member));
    assert!(game.state.player1.waitroom.cards.contains(&member));

    let movement = game.state.turn_movements.last();
    assert!(movement.is_some());
    let m = movement.unwrap();
    assert_eq!(m.moved_card_id, member);
    assert_eq!(m.source_zone, "live_card_zone");
    assert_eq!(m.dest_zone, "waitroom");
    assert_eq!(m.cause_player_id, game.state.player1.id);
}

/// Test: source+destination condition passes when turn_movements has a
/// matching movement event for the same player (target="self").
#[test]
fn source_dest_condition_matches_turn_movements() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, member);

    push_movement_p1(&mut game, member, "live_card_zone", "waitroom");
    game.state.player1.waitroom.cards.push(member);

    let cond = source_dest_condition("live_card_zone", "discard");
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

    push_movement_p1(&mut game, member, "stage", "waitroom");
    game.state.player1.waitroom.cards.push(member);

    let cond = source_dest_condition("live_card_zone", "discard");
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

    push_movement_p1(&mut game, member, "live_card_zone", "waitroom");

    let cond = source_dest_condition("live_card_zone", "discard");
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

    push_movement_p1(&mut game, member, "live_card_zone", "waitroom");
    game.state.player1.waitroom.cards.push(member);

    let mut cond = source_dest_condition("live_card_zone", "discard");
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

    push_movement_p1(&mut game, member, "live_card_zone", "waitroom");
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

/// Test: player-level filtering — P1's self condition does NOT count
/// movements caused by P2.
#[test]
fn source_dest_condition_excludes_other_player() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, member);

    // Movement caused by P2, not P1
    push_movement_p2(&mut game, member, "live_card_zone", "waitroom");
    // Card ends up in P2's waitroom (simulating P2's invalid live card)
    game.state.player2.waitroom.cards.push(member);

    // "self" resolves to P1 in plain ConditionContext → P2's movement should NOT count
    let cond = source_dest_condition("live_card_zone", "discard");
    let ctx = ConditionContext::new(&game.state);
    assert!(!ctx.evaluate_condition(&cond));
}

/// Test: player-level filtering — "opponent" target counts movements
/// caused by the opponent (P2 is P1's opponent).
#[test]
fn source_dest_condition_opponent_target_includes_other_player() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, member);

    // Movement caused by P2
    push_movement_p2(&mut game, member, "live_card_zone", "waitroom");
    game.state.player2.waitroom.cards.push(member);

    // target="opponent" → should count P2's movements
    let cond = source_dest_condition_targeted("live_card_zone", "discard");
    let ctx = ConditionContext::new(&game.state);
    assert!(ctx.evaluate_condition(&cond));
}

/// Test: check_invalid_live_cards correctly records P2's player ID
/// when discarding from P2's live_card_zone.
#[test]
fn check_invalid_live_cards_distinguishes_players() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member1 = game.id("PL!-sd1-010-SD");
    let member2 = game.new_id("PL!-sd1-010-SD");
    let filler = member1;

    // Put a non-live card in each player's live_card_zone
    game.state.player1.live_card_zone.cards.push(member1);
    game.state.player2.live_card_zone.cards.push(member2);
    game.state.player1.stage.stage = [filler, -1, -1];
    game.state.player2.stage.stage = [filler, -1, -1];
    fill_decks(&mut game, filler);

    rabuka_engine::turn::TurnEngine::check_timing(&mut game.state);

    // Both cards should be in their respective waitrooms
    assert!(game.state.player1.waitroom.cards.contains(&member1));
    assert!(game.state.player2.waitroom.cards.contains(&member2));

    // Two movement events recorded
    assert_eq!(game.state.turn_movements.len(), 2);

    // Each with the correct cause_player_id
    let p1_moves: Vec<_> = game
        .state
        .turn_movements
        .iter()
        .filter(|m| m.moved_card_id == member1)
        .collect();
    let p2_moves: Vec<_> = game
        .state
        .turn_movements
        .iter()
        .filter(|m| m.moved_card_id == member2)
        .collect();
    assert_eq!(p1_moves.len(), 1);
    assert_eq!(p2_moves.len(), 1);
    assert_eq!(p1_moves[0].cause_player_id, game.state.player1.id);
    assert_eq!(p2_moves[0].cause_player_id, game.state.player2.id);
}
