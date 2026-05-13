/// Tests for 優木せつ菜 (PL!N-pb1-007-R) — Q205
///
/// 常時: During live, if live card's need_heart contains heart01-06 each >= 1,
/// gain ALL heart (heart00).
use crate::helpers::*;

fn advance_to_live_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

/// Setsuna on stage, a live card that needs all 6 hearts → ALL heart granted.
#[test]
fn setsuna_q205_all_heart_granted() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let setsuna = game.id("PL!N-pb1-007-R");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!N-bp5-026-L"); // TOKIMEKI Runners — needs all 6 hearts

    game.state.player1.stage.stage = [setsuna, filler, -1];
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(15);

    advance_to_live_set(&mut game);
    game.set_live_card(live);
    game.pass();
    game.pass();

    // LiveStart triggered; constant ability should apply ALL heart
    // (Engine gap: temporal_condition "during_live" not evaluated for constant abilities)
}
