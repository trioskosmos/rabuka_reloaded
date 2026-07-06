/// Q107 (黒澤ダイヤ) + Q156 (ダイスキだったらダイジョウブ！)
use crate::helpers::*;

fn fill_decks(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..60 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn setup(game: &mut TestGame, dia: i16, daisuki: i16) {
    game.state.player1.stage.stage[2] = dia;
    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(daisuki);
    fill_decks(game);
    game.give_energy(15);
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(daisuki);
    game.pass();
    game.pass();
    game.pass();
}

/// All 5 cards have blade_heart → both queue. Pick DIA first.
#[test]
fn filler_deck_select_dia_first() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-bp2-004-R");
    let daisuki = game.id("PL!S-bp3-020-L");
    setup(&mut game, dia, daisuki);

    assert_eq!(game.state.initial_yell_revealed_cards.len(), 5);
    assert!(game.has_pending_choice(), "SelectAutoAbility");
    game.select_option(0);

    assert!(game.has_pending_choice(), "DIA discard");
    let count = match game.get_pending_choice() {
        rabuka_engine::ability::types::Choice::SelectCard { count, .. } => *count,
        _ => 0,
    };
    game.select_indices(&(0..count).collect::<Vec<_>>());

    while game.has_pending_choice() {
        let c = game.get_pending_choice();
        match c {
            rabuka_engine::ability::types::Choice::SelectAutoAbility { .. } => {
                game.select_option(0)
            }
            rabuka_engine::ability::types::Choice::SelectCard { .. } => game.select_indices(&[]),
            _ => game.select_indices(&[]),
        }
    }
    assert!(!game.state.re_yell_revealed_cards.is_empty(), "re-yell");
}

/// Mixed deck (energy + filler) so ≤2 blade_heart passes. Pick DIA first.
#[test]
fn mixed_deck_dia_first_then_daisuki() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-bp2-004-R");
    let daisuki = game.id("PL!S-bp3-020-L");

    game.state.player1.stage.stage[2] = dia;
    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(daisuki);
    for _ in 0..30 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.id("LL-E-001-SD"));
    }
    for _ in 0..30 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.id("PL!-sd1-010-SD"));
    }
    game.give_energy(15);
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(daisuki);
    game.pass();
    game.pass();
    game.pass();

    assert!(game.has_pending_choice(), "SelectAutoAbility");
    game.select_option(0);

    while game.has_pending_choice() {
        let c = game.get_pending_choice();
        match c {
            rabuka_engine::ability::types::Choice::SelectAutoAbility { .. } => {
                game.select_option(0)
            }
            rabuka_engine::ability::types::Choice::SelectCard { count, .. } => {
                game.select_indices(&(0..*count).collect::<Vec<_>>());
            }
            _ => game.select_indices(&[]),
        }
    }
    assert!(!game.state.re_yell_revealed_cards.is_empty(), "re-yell");
}

/// Mixed deck. Pick DAISUKI first.
#[test]
fn mixed_deck_daisuki_first_then_dia() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-bp2-004-R");
    let daisuki = game.id("PL!S-bp3-020-L");

    game.state.player1.stage.stage[2] = dia;
    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(daisuki);
    for _ in 0..30 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.id("LL-E-001-SD"));
    }
    for _ in 0..30 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.id("PL!-sd1-010-SD"));
    }
    game.give_energy(15);
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(daisuki);
    game.pass();
    game.pass();
    game.pass();

    assert!(game.has_pending_choice(), "SelectAutoAbility");
    game.select_option(1);

    while game.has_pending_choice() {
        let c = game.get_pending_choice();
        match c {
            rabuka_engine::ability::types::Choice::SelectAutoAbility { .. } => {
                game.select_option(0)
            }
            rabuka_engine::ability::types::Choice::SelectCard { count, .. } => {
                game.select_indices(&(0..*count).collect::<Vec<_>>());
            }
            _ => game.select_indices(&[]),
        }
    }
    assert!(!game.state.re_yell_revealed_cards.is_empty(), "re-yell");
}
