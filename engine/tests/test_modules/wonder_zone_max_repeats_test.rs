use crate::helpers::*;
use rabuka_engine::card::HeartColor;

fn advance_to_live_card_set(game: &mut TestGame) {
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();
}

fn finish_live_setup(game: &mut TestGame) {
    game.pass();
    game.pass();
    game.drain_auto_ability_choices();
}

#[test]
fn center_mus_with_2_heart03_reduces_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let wonder = game.id("PL!-bp5-020-L");
    let honoka = game.id("PL!-sd1-001-SD");
    let filler = game.id("LL-E-001-SD");

    game.state.player1.stage.stage = [honoka, -1, -1];
    game.state.player1.hand.cards.push(wonder);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(10);

    advance_to_live_card_set(&mut game);
    game.set_live_card(wonder);
    finish_live_setup(&mut game);

    let mod_val = game
        .state
        .mods
        .get_need_heart_modifier(wonder, HeartColor::Heart00);
    assert_eq!(
        mod_val, -1,
        "honoka has 2 heart03, per_unit_count=2 -> 1 unit -> -1 heart00, got {mod_val}"
    );
}

#[test]
fn center_member_with_0_heart03_no_reduction() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let wonder = game.id("PL!-bp5-020-L");
    let rin = game.id("PL!-sd1-005-SD");
    let filler = game.id("LL-E-001-SD");

    game.state.player1.stage.stage = [-1, rin, -1];
    game.state.player1.hand.cards.push(wonder);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(10);

    advance_to_live_card_set(&mut game);
    game.set_live_card(wonder);
    finish_live_setup(&mut game);

    let mod_val = game
        .state
        .mods
        .get_need_heart_modifier(wonder, HeartColor::Heart00);
    assert_eq!(
        mod_val, 0,
        "rin has 0 heart03 -> 0 reduction, got {mod_val}"
    );
}

#[test]
fn no_center_member_no_reduction() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let wonder = game.id("PL!-bp5-020-L");
    let filler = game.id("LL-E-001-SD");

    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.player1.hand.cards.push(wonder);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(10);

    advance_to_live_card_set(&mut game);
    game.set_live_card(wonder);
    finish_live_setup(&mut game);

    let mod_val = game
        .state
        .mods
        .get_need_heart_modifier(wonder, HeartColor::Heart00);
    assert_eq!(
        mod_val, 0,
        "no center member -> condition fails -> 0 reduction, got {mod_val}"
    );
}

#[test]
fn center_non_mus_no_reduction() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let wonder = game.id("PL!-bp5-020-L");
    let non_mus = game.id("PL!SP-sd1-001-SD");
    let filler = game.id("LL-E-001-SD");

    game.state.player1.stage.stage = [-1, non_mus, -1];
    game.state.player1.hand.cards.push(wonder);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(10);

    advance_to_live_card_set(&mut game);
    game.set_live_card(wonder);
    finish_live_setup(&mut game);

    let mod_val = game
        .state
        .mods
        .get_need_heart_modifier(wonder, HeartColor::Heart00);
    assert_eq!(
        mod_val, 0,
        "center non-mu's member -> condition fails -> 0 reduction, got {mod_val}"
    );
}
