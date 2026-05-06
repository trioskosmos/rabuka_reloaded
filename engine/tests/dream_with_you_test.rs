/// Q116: Dream with You (PL!N-sd1-028-SD) — LiveStart: total blade ≥ 10 → score +1.
/// The check is at LiveStart timing, independent of actual cheer count.
mod helpers;
use helpers::*;

fn advance_to_live_set(game: &mut TestGame) {
    for _ in 0..5 { game.pass(); }
}

#[test]
fn dream_with_you_q116_blade_10_score_plus_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let dream = game.id("PL!N-sd1-028-SD");
    let filler = game.id("PL!-sd1-010-SD");
    // 3 members with total blade ≥ 10
    let blader = game.id("PL!S-PR-014-PR"); // blade=6 each

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 { game.state.player2.main_deck.cards.push(filler); }

    // 2 bladers = 12 blade ≥ 10
    game.state.player1.stage.stage = [blader, blader, filler];
    game.state.player1.hand.cards.push(dream);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_set(&mut game);
    game.set_live_card(dream);
    game.pass();
    game.pass();

    while game.has_pending_choice() { game.select_indices(&[]); }

    let mod_val = game.state.get_score_modifier(dream);
    assert_eq!(mod_val, 1, "Blade ≥10 → score +1");
}

#[test]
fn dream_with_you_q116_blade_6_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let dream = game.id("PL!N-sd1-028-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let blader = game.id("PL!S-PR-014-PR"); // blade=6
    let low = game.id("PL!-sd1-002-SD"); // blade=1

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 { game.state.player2.main_deck.cards.push(filler); }

    // 1 blader + 1 low = 7 blade < 10
    game.state.player1.stage.stage = [blader, low, filler];
    game.state.player1.hand.cards.push(dream);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_set(&mut game);
    game.set_live_card(dream);
    game.pass();
    game.pass();

    while game.has_pending_choice() { game.select_indices(&[]); }

    let mod_val = game.state.get_score_modifier(dream);
    eprintln!("[DREAM] score_mod={}", mod_val);
    assert_eq!(mod_val, 0, "Blade <10 → no score");
}
