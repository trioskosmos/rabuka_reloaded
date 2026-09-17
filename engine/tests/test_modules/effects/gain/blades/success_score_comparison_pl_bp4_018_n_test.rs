use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }
}

#[test]
fn pl_bp4_018_n_higher_own_success_score_grants_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!-bp4-018-N");
    let filler = game.id("PL!-sd1-010-SD");
    let own_live = game.id("PL!-sd1-019-SD");
    let _opp_live = game.id("PL!-sd1-020-SD");
    game.give_energy(15);
    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(own_live);
    game.state.recalculate_constants();
    let blade = game.state.mods.get_blade_modifier(card);
    assert!(
        blade >= 2,
        "Should gain blade ×2 when own score {} > opponent score {} (got {})",
        1,
        0,
        blade
    );
}

#[test]
fn pl_bp4_018_n_higher_opponent_success_score_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!-bp4-018-N");
    let filler = game.id("PL!-sd1-010-SD");
    let own_live = game.id("PL!-sd1-019-SD");
    let opp_live = game.id("PL!-sd1-020-SD");
    game.give_energy(15);
    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(own_live);
    game.state
        .player2
        .success_live_card_zone
        .cards
        .push(opp_live);
    game.state.recalculate_constants();
    let blade = game.state.mods.get_blade_modifier(card);
    assert_eq!(
        blade, 0,
        "Should gain 0 blade when own score {} < opponent score {} (got {})",
        1, 2, blade
    );
}

#[test]
fn pl_bp4_018_n_success_score_changes_update_blades_strictly_greater() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!-bp4-018-N");
    let filler = game.id("PL!-sd1-010-SD");
    let own_live_a = game.id("PL!-sd1-019-SD");
    let own_live_b = game.id("PL!-sd1-019-SD");
    let opp_live = game.id("PL!-sd1-020-SD");
    game.give_energy(15);
    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(card),
        0,
        "Equal scores (0=0) should give 0 blade"
    );
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(own_live_a);
    game.state.recalculate_constants();
    assert!(
        game.state.mods.get_blade_modifier(card) >= 2,
        "Own score 1 > opp score 0 should give blade ×2"
    );
    game.state
        .player2
        .success_live_card_zone
        .cards
        .push(opp_live);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(card),
        0,
        "Own score 1 < opp score 2 should give 0 blade"
    );
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(own_live_b);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(card),
        0,
        "Equal scores (2=2) should give 0 blade"
    );
}
