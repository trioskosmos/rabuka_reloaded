/// Q107 (黒澤ダイヤ) + Q156 (ダイスキだったらダイジョウブ！) — both trigger on same yell.
///
/// DIA ab#0 (stage): if no live card in yell reveals → optional discard → re-yell.
/// DAISUKI ab#0 (live): if ≤2 blade_heart among reveals → optional discard → re-yell.
/// Both are 自動 [1/ターン]. When both fire, player chooses order.
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

/// Both cards trigger on the same yell. Player chooses DIA first, then DAISUKI.
/// Q107: After DIA's re-yell, DAISUKI sees only the new yell's cards.
#[test]
fn dia_first_then_daisuki() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-bp2-004-R");
    let daisuki = game.id("PL!S-bp3-020-L");
    setup(&mut game, dia, daisuki);

    // Both abilities queued — SelectAutoAbility appears
    eprintln!(
        "[TEST] after setup: initial_yell={:?} revealed={:?} pending={}",
        game.state.initial_yell_revealed_cards,
        game.state.revealed_cards,
        game.has_pending_choice(),
    );
    assert!(game.has_pending_choice(), "SelectAutoAbility prompt");
    // Q107: pick DIA first (option 0 = first queued)
    game.select_option(0);

    // DIA fires: optional discard prompt
    assert!(game.has_pending_choice(), "DIA discard prompt");
    let count = if let rabuka_engine::ability::types::Choice::SelectCard { count, .. } =
        game.get_pending_choice()
    {
        *count
    } else {
        0
    };
    game.select_indices(&(0..count).collect::<Vec<_>>());

    // DIA's re_yell + perform_yell run
    // After DIA resolves, DAISUKI fires next
    while game.has_pending_choice() {
        let c = game.get_pending_choice();
        match c {
            rabuka_engine::ability::types::Choice::SelectAutoAbility { .. } => {
                game.select_option(0);
            }
            rabuka_engine::ability::types::Choice::SelectCard { .. } => {
                game.select_indices(&[]); // skip
            }
            _ => {
                game.select_indices(&[]);
            }
        }
    }

    assert!(
        !game.state.re_yell_revealed_cards.is_empty(),
        "at least one re-yell happened"
    );
}

/// Both cards trigger. Player picks DAISUKI first, then DIA.
/// Q156: DAISUKI's re-yell happens, then DIA evaluates on the new yell.
#[test]
fn daisuki_first_then_dia() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-bp2-004-R");
    let daisuki = game.id("PL!S-bp3-020-L");
    setup(&mut game, dia, daisuki);

    // Two abilities queued. We want DAISUKI first.
    // The SelectAutoAbility shows all queue entries; pick the DAISUKI one.
    // We need select_option N where N is the index of DAISUKI's queue entry.
    // DIA is enqueued first (stage card scan), DAISUKI second (live card scan).
    // So DAISUKI is at option index 1.
    let _test_pending = game.has_pending_choice();
    if !_test_pending {
        eprintln!(
            "[DBG] queue={} revealed={:?} init_yell={:?}",
            game.state.ability_queue.len(),
            game.state.revealed_cards,
            game.state.initial_yell_revealed_cards,
        );
    }
    assert!(_test_pending, "SelectAutoAbility prompt");
    game.select_option(1); // DAISUKI first

    // DAISUKI fires: optional discard prompt
    assert!(game.has_pending_choice(), "DAISUKI discard prompt");
    game.select_indices(&[]); // skip discard (re-yell still fires)

    // After DAISUKI's re-yell, DIA fires next
    while game.has_pending_choice() {
        let c = game.get_pending_choice();
        match c {
            rabuka_engine::ability::types::Choice::SelectAutoAbility { .. } => {
                game.select_option(0);
            }
            rabuka_engine::ability::types::Choice::SelectCard { .. } => {
                game.select_indices(&[]); // skip
            }
            _ => {
                game.select_indices(&[]);
            }
        }
    }

    assert!(
        !game.state.re_yell_revealed_cards.is_empty(),
        "at least one re-yell happened"
    );
}
