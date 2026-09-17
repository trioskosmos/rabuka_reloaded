use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

fn pl_sp_bp5_016_n_advance_to_live_card_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

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
fn pl_sp_bp5_016_n_energy_at_least_ten_grants_heart06() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!SP-bp5-016-N");
    let filler = game.id("PL!-sd1-010-SD");
    game.give_energy(15);
    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    game.give_energy(4);
    assert!(
        game.state.player1.energy_zone.active_count() >= 10,
        "Precondition: energy >= 10"
    );
    game.state.recalculate_constants();
    let h06 = game
        .state
        .mods
        .get_heart_modifier(card, HeartColor::Heart06);
    assert!(
        h06 >= 2,
        "Should gain at least heart06 ×2 with energy >= 10 (got {})",
        h06
    );
}

#[test]
fn pl_sp_bp5_016_n_energy_below_ten_no_heart06() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!SP-bp5-016-N");
    let filler = game.id("PL!-sd1-010-SD");
    game.give_energy(15);
    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    game.state.player1.energy_zone.cards.truncate(5);
    game.state.player1.energy_zone.set_active_count(5);
    assert!(
        game.state.player1.energy_zone.cards.len() < 10,
        "Precondition: total energy cards < 10 (got {})",
        game.state.player1.energy_zone.cards.len()
    );
    game.state.recalculate_constants();
    let h06 = game
        .state
        .mods
        .get_heart_modifier(card, HeartColor::Heart06);
    assert_eq!(
        h06, 0,
        "Should gain 0 heart06 with energy < 10 (got {})",
        h06
    );
}

#[test]
fn pl_sp_bp5_016_n_live_phase_evaluates_energy_threshold_constant() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!SP-bp5-016-N");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-020-SD");
    let filler_live = game.id("PL!-sd1-019-SD");
    game.give_energy(15);
    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    game.give_energy(5);
    game.state.player1.hand.cards.push(live_card);
    game.state.player2.hand.cards.push(filler_live);
    pl_sp_bp5_016_n_advance_to_live_card_set(&mut game);
    game.set_live_card(live_card);
    game.pass();
    game.pass();
    let h06 = game
        .state
        .mods
        .get_heart_modifier(card, HeartColor::Heart06);
    assert!(
        h06 >= 2,
        "Live phase: should gain heart06 ×2 from constant (got {})",
        h06
    );
}
