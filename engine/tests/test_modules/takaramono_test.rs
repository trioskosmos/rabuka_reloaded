/// Tests for タカラモノズ (PL!-bp3-025-L) — LiveSuccess ability:
///
/// {{live_success.png|ライブ成功時}}このターン、自分が余剰ハートを持たない場合、
/// このカードのスコアを＋１する。
///
/// Q142: Definition of surplus heart (data-level, not engine behavior)
/// Q36:  LiveSuccess timing definition.
use crate::helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_success(game: &mut TestGame) {
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();
}

/// When self_no_excess_heart_this_turn is set, the condition passes
/// and score +1 is applied.
#[test]
fn takaramono_no_excess_heart_score_plus_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let takaramono = game.id("PL!-bp3-025-L");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!-sd1-001-SD");

    // A member on stage (required for live)
    game.state.player1.stage.stage = [member, -1, -1];
    game.state.player1.hand.cards.push(takaramono);

    // Seed decks
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Set the self_no_excess_heart flag AFTER phase advancement (it gets reset
    // at turn start). This simulates no surplus heart during this live.
    advance_to_live_card_set_p1(&mut game);
    game.state.self_no_excess_heart_this_turn = true;
    game.set_live_card(takaramono);
    advance_to_live_success(&mut game);

    // Score should be +1 (no excess heart condition met)
    let score_mod = game.state.mods.get_score_modifier(takaramono);
    assert_eq!(score_mod, 1, "No excess heart → score +1");
}

/// When self has excess heart (flag not set), condition fails.
#[test]
fn takaramono_excess_heart_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let takaramono = game.id("PL!-bp3-025-L");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!-sd1-001-SD");

    game.state.player1.stage.stage = [member, -1, -1];
    game.state.player1.hand.cards.push(takaramono);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Flag not set — excess heart exists (default is false after reset)
    advance_to_live_card_set_p1(&mut game);
    // Confirm flag is false
    assert!(
        !game.state.self_no_excess_heart_this_turn,
        "Flag should be false after reset"
    );
    game.set_live_card(takaramono);
    advance_to_live_success(&mut game);

    let score_mod = game.state.mods.get_score_modifier(takaramono);
    assert_eq!(score_mod, 0, "Excess heart → no score boost");
}
