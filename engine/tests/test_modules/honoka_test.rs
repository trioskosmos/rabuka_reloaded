/// Tests for PL!-pb1-001-R (高坂穂乃果) ab#0 — Q166, Q167
use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;

fn setup_honoka_deck(game: &mut TestGame, honoka: i16, deck_top: Vec<i16>) {
    game.state.player1.stage.stage[1] = honoka;
    let hand_filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(hand_filler);
    game.state.player1.hand.cards.push(hand_filler);
    game.state.player1.main_deck.cards = deck_top.into();
    game.give_energy(13);
}

fn activate_and_choose_type(game: &mut TestGame, honoka: i16, type_option: i16) {
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(honoka),
        None,
        None,
        None,
    )
    .expect("activate");
    assert!(
        game.has_pending_choice(),
        "Should have hand discard cost choice"
    );
    game.select_indices(&[0]);
    assert!(game.has_pending_choice(), "Should have card type choice");
    let desc = game
        .state
        .get_pending_choice_json()
        .and_then(|j| {
            j.get("description")
                .and_then(|d| d.as_str().map(|s| s.to_string()))
        })
        .unwrap_or_default();
    assert!(
        desc.contains("Live card"),
        "Choice should mention Live card, got: {}",
        desc
    );
    assert!(
        desc.contains("Member card"),
        "Choice should mention Member card, got: {}",
        desc
    );
    game.select_option(type_option);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
}

#[test]
fn honoka_q166_member_skips_live_fillers() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let honoka = game.id("PL!-pb1-001-R");
    let live_filler = game.id("PL!-sd1-019-SD");
    let target_member = game.id("PL!SP-bp2-006-P");
    let mut deck = Vec::new();
    for _ in 0..4 {
        deck.push(live_filler);
    }
    deck.push(target_member);
    setup_honoka_deck(&mut game, honoka, deck);
    activate_and_choose_type(&mut game, honoka, 1);
    assert!(
        game.state.player1.hand.cards.contains(&target_member),
        "Target member should be in hand"
    );
    let hand_live = game
        .state
        .player1
        .hand
        .cards
        .iter()
        .filter(|&&id| id == live_filler)
        .count();
    assert_eq!(
        hand_live, 0,
        "Live fillers should NOT be in hand, found {}",
        hand_live
    );
    let disc_live = game
        .state
        .player1
        .waitroom
        .cards
        .iter()
        .filter(|&&id| id == live_filler)
        .count();
    assert_eq!(
        disc_live, 4,
        "All 4 revealed live fillers should be in discard, found {}",
        disc_live
    );
}

#[test]
fn honoka_q166_target_first_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let honoka = game.id("PL!-pb1-001-R");
    let target = game.id("PL!SP-bp2-006-P");
    let filler = game.id("PL!-sd1-019-SD");
    setup_honoka_deck(
        &mut game,
        honoka,
        vec![target, filler, filler, filler, filler, filler],
    );
    activate_and_choose_type(&mut game, honoka, 1);
    assert!(
        game.state.player1.hand.cards.contains(&target),
        "Target should be in hand"
    );
    assert!(
        !game.state.player1.main_deck.cards.contains(&target),
        "Target should not remain in deck"
    );
}

#[test]
fn honoka_q166_target_last_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let honoka = game.id("PL!-pb1-001-R");
    let target = game.id("PL!SP-bp2-006-P");
    let filler = game.id("PL!-sd1-019-SD");
    let mut deck = Vec::new();
    for _ in 0..9 {
        deck.push(filler);
    }
    deck.push(target);
    setup_honoka_deck(&mut game, honoka, deck);
    activate_and_choose_type(&mut game, honoka, 1);
    assert!(
        game.state.player1.hand.cards.contains(&target),
        "Target should be in hand"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        0,
        "All deck cards should have been revealed"
    );
}

#[test]
fn honoka_q166_live_card_skips_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let honoka = game.id("PL!-pb1-001-R");
    let member = game.id("PL!SP-bp2-006-P");
    let target_live = game.id("PL!-sd1-019-SD");
    setup_honoka_deck(&mut game, honoka, vec![member, member, member, target_live]);
    activate_and_choose_type(&mut game, honoka, 0);
    assert!(
        game.state.player1.hand.cards.contains(&target_live),
        "Live card should be in hand"
    );
    let hand_member = game
        .state
        .player1
        .hand
        .cards
        .iter()
        .filter(|&&id| id == member)
        .count();
    assert_eq!(hand_member, 0, "Member cards should NOT be in hand");
}

#[test]
fn honoka_q166_two_matches_only_one_added() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let honoka = game.id("PL!-pb1-001-R");
    let m1 = game.id("PL!SP-bp2-006-P");
    let m2 = game.id("PL!SP-bp2-006-P");
    let filler = game.id("PL!-sd1-019-SD");
    setup_honoka_deck(&mut game, honoka, vec![m1, filler, filler, m2]);
    activate_and_choose_type(&mut game, honoka, 1);
    let in_hand = game
        .state
        .player1
        .hand
        .cards
        .iter()
        .filter(|&&id| id == m1 || id == m2)
        .count();
    assert_eq!(
        in_hand, 1,
        "Only 1 of 2 matching cards added (reveal stops at first)"
    );
}

#[test]
fn honoka_q166_no_member_in_deck_refresh() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let honoka = game.id("PL!-pb1-001-R");
    let filler = game.id("PL!-sd1-019-SD");
    setup_honoka_deck(&mut game, honoka, vec![filler; 10]);
    activate_and_choose_type(&mut game, honoka, 1);
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        0,
        "Deck exhausted"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        10,
        "All cards in discard"
    );
}

#[test]
fn honoka_q167_deck_exhausted_during_reveal() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let honoka = game.id("PL!-pb1-001-R");
    let target = game.id("PL!SP-bp2-006-P");
    let filler = game.id("PL!-sd1-019-SD");
    let mut deck = Vec::new();
    for _ in 0..5 {
        deck.push(filler);
    }
    deck.push(target);
    setup_honoka_deck(&mut game, honoka, deck);
    activate_and_choose_type(&mut game, honoka, 1);
    assert!(
        game.state.player1.hand.cards.contains(&target),
        "Target should be in hand after refresh"
    );
}

#[test]
fn honoka_center_requirement_left_side_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let honoka = game.id("PL!-pb1-001-R");
    game.state.player1.stage.stage[0] = honoka;
    let hf = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(hf);
    game.state.player1.hand.cards.push(hf);
    game.give_energy(13);
    let result = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(honoka),
        None,
        None,
        None,
    );
    assert!(
        result.is_err(),
        "Ability should fail from left side (center required)"
    );
    if let Ok(_) = result {
        if game.has_pending_choice() {
            game.select_indices(&[0]);
        }
    }
}

#[test]
fn honoka_use_limit_blocks_second_activation() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let honoka = game.id("PL!-pb1-001-R");
    let filler = game.id("PL!-sd1-019-SD");
    let target = game.id("PL!SP-bp2-006-P");
    setup_honoka_deck(&mut game, honoka, vec![target, filler, filler, filler]);
    activate_and_choose_type(&mut game, honoka, 1);
    assert!(
        game.state.player1.hand.cards.contains(&target),
        "Target in hand after first activation"
    );
    let result = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(honoka),
        None,
        None,
        None,
    );
    assert!(result.is_err(), "Second activation should fail (use_limit)");
}
