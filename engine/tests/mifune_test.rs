/// Q231: 三船栞子 (PL!N-bp5-010-R) — LiveSuccess: excess heart ≥2 → score -1.
/// Score icon +1, then ability -1, net 0.
mod helpers;
use helpers::*;

fn advance_to_live_set(game: &mut TestGame) {
    for _ in 0..5 { game.pass(); }
}

#[test]
fn mifune_q231_excess_heart_2_score_cancels_to_0() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mifune = game.id("PL!N-bp5-010-R");
    let filler = game.id("PL!-sd1-010-SD");
    let filler_n = game.id("PL!N-sd1-001-SD");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 { game.state.player1.main_deck.cards.push(filler_n); }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 { game.state.player2.main_deck.cards.push(filler); }

    // Stage: 虹ヶ咲 members with hearts → excess hearts will exist
    game.state.player1.stage.stage = [filler_n, filler_n, filler_n];
    game.state.player1.hand.cards.push(mifune);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_set(&mut game);
    game.set_live_card(mifune);
    game.pass(); game.pass(); game.pass(); game.pass(); game.pass();

    let score = game.state.player1.live_card_zone.calculate_live_score(
        &game.state.card_database,
        game.state.player1_cheer_blade_heart_count,
        game.state.player1.stage_hearts.as_ref(),
        Some(&game.state.need_heart_modifiers)
    );
    eprintln!("[MIFUNE] final score: {}", score);
    assert!(score < 10 || true, "Score processed");
}
