/// ダイスキだったらダイジョウブ！(PL!S-bp3-020-L) ab#0
///
/// 自動 [1/ターン]: エールにより自分のカードを1枚以上公開したとき、それらのカードの中に
/// ブレードハートを持つカードが2枚以下の場合、それらのカードをすべて控え室に置いてもよい。
/// そのエールで得たブレードハートを失い、もう一度エールを行う。
use crate::helpers::*;

fn fill_decks(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..60 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn advance_to_p1_performance(game: &mut TestGame, daisuki: i16) {
    game.state.player1.hand.cards.push(daisuki);
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(daisuki);
    game.pass();
    game.pass();
    game.pass();
}

/// 0 cards from yell → compound condition fails → no trigger.
#[test]
fn condition_0_revealed_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let daisuki = game.id("PL!S-bp3-020-L");
    fill_decks(&mut game);
    game.give_energy(15);
    advance_to_p1_performance(&mut game, daisuki);
    assert!(!game.has_pending_choice(), "0 revealed → no trigger");
}

/// 1 card from yell → both conditions met → fires.
#[test]
fn condition_1_revealed_triggers() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let daisuki = game.id("PL!S-bp3-020-L");
    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game);
    game.give_energy(15);
    advance_to_p1_performance(&mut game, daisuki);
    assert_eq!(game.state.initial_yell_revealed_cards.len(), 1);
    assert!(game.has_pending_choice(), "1 revealed → discard prompt");
}

/// 5 cards all with blade_heart → 5 > 2 → condition fails → no trigger.
#[test]
fn condition_5_revealed_all_blade_heart_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let daisuki = game.id("PL!S-bp3-020-L");
    let m3 = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[2] = m3;
    game.state.mods.add_blade_modifier(m3, 2);
    fill_decks(&mut game);
    game.give_energy(15);
    advance_to_p1_performance(&mut game, daisuki);
    assert!(game.state.initial_yell_revealed_cards.len() >= 5);
    assert!(!game.has_pending_choice());
    assert!(!game.state.re_yell_occurred);
}

/// 0 blade_heart → 0 ≤ 2 → conditions met → discard prompt.
#[test]
fn blade_heart_0_triggers_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let daisuki = game.id("PL!S-bp3-020-L");
    let e_card = game.id("LL-E-001-SD");
    for _ in 0..60 {
        game.state.player1.main_deck.cards.push(e_card);
        game.state.player2.main_deck.cards.push(e_card);
    }
    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = game.new_id("PL!-sd1-010-SD");
    game.give_energy(15);
    advance_to_p1_performance(&mut game, daisuki);
    assert!(game.has_pending_choice(), "0 blade_heart → discard prompt");
}

/// 2 blade_heart → 2 ≤ 2 → conditions met → discard prompt.
#[test]
fn blade_heart_2_triggers_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let daisuki = game.id("PL!S-bp3-020-L");
    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game);
    game.give_energy(15);
    advance_to_p1_performance(&mut game, daisuki);
    assert_eq!(game.state.initial_yell_revealed_cards.len(), 2);
    assert!(game.has_pending_choice(), "2 blade_heart → discard prompt");
}

/// 3 blade_heart → 3 > 2 → condition fails → no trigger.
#[test]
fn blade_heart_3_blocks_entire_ability() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let daisuki = game.id("PL!S-bp3-020-L");
    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[2] = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game);
    game.give_energy(15);
    advance_to_p1_performance(&mut game, daisuki);
    assert_eq!(game.state.initial_yell_revealed_cards.len(), 3);
    assert!(!game.has_pending_choice(), "no trigger with 3 blade_heart");
}

/// Accept discard → re-yell follows.
#[test]
fn discard_accept_then_re_yells() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let daisuki = game.id("PL!S-bp3-020-L");
    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game);
    game.give_energy(15);

    let deck_before = game.state.player1.main_deck.cards.len();
    advance_to_p1_performance(&mut game, daisuki);

    assert!(game.has_pending_choice(), "discard prompt");
    let count = match game.get_pending_choice() {
        rabuka_engine::ability::types::Choice::SelectCard { count, .. } => *count,
        _ => 0,
    };
    game.select_indices(&(0..count).collect::<Vec<_>>());
    assert!(
        !game.state.re_yell_revealed_cards.is_empty(),
        "re-yell happened"
    );
    assert!(
        game.state.player1.main_deck.cards.len() < deck_before,
        "re-yell consumed deck cards"
    );
}

/// Skip discard → no re-yell.
#[test]
fn discard_skip_no_re_yell() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let daisuki = game.id("PL!S-bp3-020-L");
    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game);
    game.give_energy(15);

    advance_to_p1_performance(&mut game, daisuki);

    assert!(game.has_pending_choice(), "discard prompt");
    game.select_indices(&[]);
    assert!(!game.state.re_yell_occurred, "no re-yell after skip");
}

/// Full flow: accept discard → lose blade hearts → re-yell.
#[test]
fn full_flow_accept_discard_and_re_yell() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let daisuki = game.id("PL!S-bp3-020-L");
    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game);
    game.give_energy(15);

    let deck_before = game.state.player1.main_deck.cards.len();
    advance_to_p1_performance(&mut game, daisuki);

    assert!(game.has_pending_choice(), "discard prompt");
    let count = match game.get_pending_choice() {
        rabuka_engine::ability::types::Choice::SelectCard { count, .. } => *count,
        _ => 0,
    };
    game.select_indices(&(0..count).collect::<Vec<_>>());

    assert!(
        !game.state.re_yell_revealed_cards.is_empty(),
        "re_yell populated"
    );
    assert_eq!(game.state.initial_yell_revealed_cards.len(), 2);
    assert!(
        game.state.player1.main_deck.cards.len() < deck_before,
        "re-yell consumed deck"
    );
}

/// Once-per-turn guard.
#[test]
fn once_per_turn_does_not_trigger_again_same_turn() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let daisuki = game.id("PL!S-bp3-020-L");
    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game);
    game.give_energy(15);

    advance_to_p1_performance(&mut game, daisuki);

    assert!(game.has_pending_choice(), "first trigger");
    let count = match game.get_pending_choice() {
        rabuka_engine::ability::types::Choice::SelectCard { count, .. } => *count,
        _ => 0,
    };
    game.select_indices(&(0..count).collect::<Vec<_>>());

    let mut saw_daisuki_again = false;
    while game.has_pending_choice() {
        let c = game.get_pending_choice();
        match c {
            rabuka_engine::ability::types::Choice::SelectAutoAbility { options, .. } => {
                if options.iter().any(|o| o.card_name.contains("ダイスキ")) {
                    saw_daisuki_again = true;
                }
                game.select_indices(&[]);
            }
            rabuka_engine::ability::types::Choice::SelectCard { .. } => {
                game.select_indices(&[]);
            }
            _ => break,
        }
    }
    assert!(!saw_daisuki_again, "daisuki not again same turn");
}

/// DIA + 2 fillers = 5 blade. Both queue. Pick DIA first.
#[test]
fn with_dia_only_dia_triggers() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-bp2-004-R");
    let daisuki = game.id("PL!S-bp3-020-L");

    game.state.player1.stage.stage[2] = dia;
    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(daisuki);
    fill_decks(&mut game);
    game.give_energy(15);
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(daisuki);
    game.pass();
    game.pass();
    game.pass();

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
    assert!(
        !game.state.re_yell_revealed_cards.is_empty(),
        "re-yell happened"
    );
}

/// Mixed deck: both trigger. Pick DIA first.
#[test]
fn with_dia_mixed_deck_both_trigger() {
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
    assert!(
        !game.state.re_yell_revealed_cards.is_empty(),
        "re-yell happened"
    );
}
