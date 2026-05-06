/// Tests for ミラクル STAY TUNE！ (PL!N-bp5-027-L) ab#0
///
/// LiveStart: If (either player has >= 2 cards in success_live_zone)
/// AND (self stage has >= 3 distinct-name members), this card's score +1.
///
/// Q208: Multi-name cards count as 1 member
mod helpers;
use helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 { game.pass(); }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

/// Both conditions met: success_live_zone has 2 cards AND stage has 3 distinct-name members.
#[test]
fn miracle_stay_tune_both_conditions_met_score_plus_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!N-bp5-027-L");
    let filler = game.id("PL!-sd1-010-SD");
    let member1 = game.id("PL!-sd1-002-SD");
    let member2 = game.id("PL!-sd1-005-SD");
    let member3 = game.id("PL!-sd1-008-SD");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..20 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..20 { game.state.player2.main_deck.cards.push(filler); }

    game.state.player1.hand.cards.push(card);
    game.state.player1.hand.cards.push(filler);
    // 3 distinct-name members
    game.state.player1.stage.stage = [member1, member2, member3];
    // Pre-populate success_live_zone with 2 cards (either player)
    game.state.player1.success_live_card_zone.cards.push(filler);
    game.state.player1.success_live_card_zone.cards.push(filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(card);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() { game.select_indices(&[]); }

    let mod_val = game.state.get_score_modifier(card);
    assert_eq!(mod_val, 1, "Both conditions met → score +1");
}

/// Only success_live_zone condition met (only 1 distinct member on stage).
#[test]
fn miracle_stay_tune_fewer_than_3_distinct_members_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!N-bp5-027-L");
    let filler = game.id("PL!-sd1-010-SD");
    let member1 = game.id("PL!-sd1-002-SD");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..20 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..20 { game.state.player2.main_deck.cards.push(filler); }

    game.state.player1.hand.cards.push(card);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage = [member1, member1, member1];
    game.state.player1.success_live_card_zone.cards.push(filler);
    game.state.player1.success_live_card_zone.cards.push(filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(card);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() { game.select_indices(&[]); }

    let mod_val = game.state.get_score_modifier(card);
    assert_eq!(mod_val, 0, "Only 1 distinct member → no score");
}

/// Only distinct-name condition met (0 cards in success_live_zone).
#[test]
fn miracle_stay_tune_empty_success_zone_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!N-bp5-027-L");
    let filler = game.id("PL!-sd1-010-SD");
    let member1 = game.id("PL!-sd1-002-SD");
    let member2 = game.id("PL!-sd1-005-SD");
    let member3 = game.id("PL!-sd1-008-SD");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..20 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..20 { game.state.player2.main_deck.cards.push(filler); }

    game.state.player1.hand.cards.push(card);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage = [member1, member2, member3];

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(card);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() { game.select_indices(&[]); }

    let mod_val = game.state.get_score_modifier(card);
    assert_eq!(mod_val, 0, "Empty success zone → no score");
}

/// Neither condition met.
#[test]
fn miracle_stay_tune_neither_condition_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!N-bp5-027-L");
    let filler = game.id("PL!-sd1-010-SD");
    let member1 = game.id("PL!-sd1-002-SD");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..20 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..20 { game.state.player2.main_deck.cards.push(filler); }

    game.state.player1.hand.cards.push(card);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage = [member1, member1, member1];

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(card);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() { game.select_indices(&[]); }

    let mod_val = game.state.get_score_modifier(card);
    assert_eq!(mod_val, 0, "Neither condition met → no score");
}
