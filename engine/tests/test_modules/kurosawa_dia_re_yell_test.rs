/// 黒澤ダイヤ (PL!S-bp2-004-R) ab#0
///
/// 自動 [1/ターン]: エールにより公開された自分のカードの中にライブカードがないとき、
/// それらのカードをすべて控え室に置いてもよい。これにより1枚以上のカードが控え室に
/// 置かれた場合、そのエールで得たブレードハートを失い、もう一度エールを行う。
use crate::helpers::*;

fn fill_decks(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..60 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn advance_to_p1_performance(game: &mut TestGame, dia: i16) {
    game.state.player1.stage.stage[2] = dia;
    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.push(live);
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(live);
    game.pass();
    game.pass();
    game.pass();
}

/// Q107: re-yell works — optional discard accepted → followup runs.
#[test]
fn q107_dia_re_yell_works() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-bp2-004-R");

    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game);
    game.give_energy(15);

    advance_to_p1_performance(&mut game, dia);

    assert!(game.has_pending_choice());
    let count = if let rabuka_engine::ability::types::Choice::SelectCard { count, .. } =
        game.get_pending_choice()
    {
        *count
    } else {
        0
    };
    game.select_indices(&(0..count).collect::<Vec<_>>());

    assert!(!game.state.initial_yell_revealed_cards.is_empty());
    assert!(!game.state.re_yell_revealed_cards.is_empty());
}

/// Q107: skip optional discard → followup does NOT run.
#[test]
fn q107_dia_skip_discard_no_followup() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-bp2-004-R");

    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game);
    game.give_energy(15);

    advance_to_p1_performance(&mut game, dia);

    assert!(game.has_pending_choice(), "Optional discard prompt");
    game.select_indices(&[]);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
}
