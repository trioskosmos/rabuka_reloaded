/// Q231: 三船栞子 (PL!N-bp5-010-R) — LiveSuccess: excess heart ≥2 → score -1.
/// Score icon +1, then ability -1, net 0.
use crate::helpers::*;

fn advance_to_live_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

#[test]
fn mifune_q231_excess_heart_2_score_cancels_to_0() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mifune = game.id("PL!N-bp5-010-R");
    let filler = game.id("PL!-sd1-010-SD");
    let filler_n = game.id("PL!N-sd1-001-SD");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler_n);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player2.main_deck.cards.push(filler);
    }

    // Stage: 虹ヶ咲 members with hearts → excess hearts will exist
    game.state.player1.stage.stage = [filler_n, filler_n, filler_n];
    game.state.player1.hand.cards.push(mifune);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_set(&mut game);
    game.set_live_card(mifune);
    // Pass through to LiveVictoryDetermination where LiveSuccess triggers fire
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();

    // Consume any residual choices from the live phase / ability triggers
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // The ability should have applied both +1 score and -1 penalty (net 0)
    let mod_val = game.state.mods.get_score_modifier(mifune);
    eprintln!("[MIFUNE] score_modifier: {}", mod_val);
    assert_eq!(
        mod_val, 0,
        "Score modifier should be 0 (+1 add -1 remove) with 2+ surplus hearts (got {})",
        mod_val
    );
}
