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

/// Q156: condition met (≤2 blade_heart) → ability triggers → discard → re-yell draws new cards.
#[test]
fn q156_daisuki_re_yell_works() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let daisuki = game.id("PL!S-bp3-020-L");

    // 2 members with blade=1 each → total_blade=2 → yell draws 2 cards
    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game);
    // Default fillers have blade_heart=yes, but condition is ≤2 → always met
    // since only 2 cards are drawn and both have blade_heart (2 ≤ 2 = true)
    game.give_energy(15);

    let deck_before = game.state.player1.main_deck.cards.len();
    advance_to_p1_performance(&mut game, daisuki);

    // Ability triggers → optional discard prompt
    assert!(game.has_pending_choice(), "Optional discard prompt");
    game.select_indices(&[0]); // accept: discard all

    // After re-yell, more deck cards should have been consumed
    let deck_after = game.state.player1.main_deck.cards.len();
    assert!(
        deck_after < deck_before,
        "re-yell should consume additional deck cards"
    );
    assert!(
        !game.state.re_yell_revealed_cards.is_empty(),
        "re-yell drew cards"
    );
    // The initial yell cards were saved
    assert!(
        !game.state.initial_yell_revealed_cards.is_empty(),
        "initial yell cards saved"
    );
}

/// Q156: condition NOT met (>2 blade_heart) → ability does not trigger.
#[test]
fn q156_daisuki_too_many_blade_heart_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let daisuki = game.id("PL!S-bp3-020-L");

    // 3 members with blade=1 each → total_blade=3 → draws 3 cards
    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[2] = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game);
    // 3 blade_heart=yes cards > 2 → condition fails
    game.give_energy(15);

    advance_to_p1_performance(&mut game, daisuki);

    // Ability should NOT trigger — no optional discard prompt
    assert!(!game.has_pending_choice(), "no prompt when >2 blade_heart");
}

/// Q156: skip optional discard → re-yell still happens (unconditional in this card).
#[test]
fn q156_daisuki_skip_discard_still_re_yells() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let daisuki = game.id("PL!S-bp3-020-L");

    game.state.player1.stage.stage[0] = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game);
    game.give_energy(15);

    advance_to_p1_performance(&mut game, daisuki);

    assert!(game.has_pending_choice(), "Optional discard prompt");
    game.select_indices(&[]); // skip discard

    // re_yell fires regardless
    assert!(
        game.state.re_yell_occurred,
        "re_yell should fire even when discard skipped"
    );
}
