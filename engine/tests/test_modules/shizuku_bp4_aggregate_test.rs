/// Tests for PL!N-bp4-003-R (桜坂しずく / Shizuku) — LiveSuccess aggregate score comparison
///
/// Ability (ライブ成功時):
///   ライブの合計スコアが相手より高い場合、カードを1枚引く。
///
/// Parsing fix: aggregate condition now has location:"stage" (was None),
///              ensuring the score comparison evaluates against the correct zone.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

/// Full live flow that triggers LiveSuccess abilities.
fn run_live_flow_p1_only(game: &mut TestGame, p1_live_card: i16) {
    // P2 hand empty → cannot set live card → P1 auto-wins
    game.state.player2.hand.cards.clear();
    run_live_flow_both(game, p1_live_card, -1)
}

fn run_live_flow_both(game: &mut TestGame, p1_live_card: i16, p2_live_card: i16) {
    for _ in 0..5 { game.pass(); }
    game.set_live_card(p1_live_card);
    game.pass();
    if p2_live_card >= 0 {
        game.set_live_card(p2_live_card);
    }
    // FirstAttackerPerformance → triggers LiveStart
    game.pass();
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => { game.select_indices(&[]); }
            _ => break,
        }
    }
    // Performance phases → LiveVictoryDetermination → LiveSuccess
    for _ in 0..3 { game.pass(); }
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectLiveSuccess") => { game.select_indices(&[0]); }
            Some("SelectAutoAbility") => { game.select_indices(&[]); }
            Some("SelectCard") => { game.select_indices(&[0]); }
            _ => break,
        }
    }
}

// ====================================================================
// PL!N-bp4-003-R (Shizuku) — LiveSuccess aggregate score comparison
// ====================================================================

/// P1 has higher score (1) than P2 (0, no live card) → draws 1 card.
#[test]
fn shizuku_higher_score_draws_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let shizuku = game.id("PL!N-bp4-003-R");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!-sd1-001-SD");
    let live = game.id("PL!-sd1-019-SD"); // score 1

    game.state.player1.stage.stage = [member, shizuku, member];
    game.state.player1.hand.cards.push(live);
    fill_decks(&mut game, filler);
    game.give_energy(5);

    let hand_before = game.state.player1.hand.cards.len();

    run_live_flow_p1_only(&mut game, live);

    // P1 wins (higher score) → draws 1 card
    assert_eq!(game.state.player1.hand.cards.len(), hand_before + 1,
        "Higher score → draw 1 card");
}

/// P2 has same score → condition fails → no draw.
#[test]
fn shizuku_tied_score_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let shizuku = game.id("PL!N-bp4-003-R");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!-sd1-001-SD");
    let live = game.id("PL!-sd1-019-SD"); // score 1

    game.state.player1.stage.stage = [member, shizuku, member];
    game.state.player2.stage.stage = [member, member, member];
    game.state.player1.hand.cards.push(live);
    game.state.player2.hand.cards.push(live);
    fill_decks(&mut game, filler);
    game.give_energy(5);

    let hand_before = game.state.player1.hand.cards.len();

    run_live_flow_both(&mut game, live, live);

    // Tied score (1 vs 1) → condition fails → no draw
    assert_eq!(game.state.player1.hand.cards.len(), hand_before,
        "Tied score → no draw");
}

/// P2 has no live card (auto-win for P1) → score 1 > 0 → draws.
#[test]
fn shizuku_p2_no_live_card_p1_wins_and_draws() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let shizuku = game.id("PL!N-bp4-003-R");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!-sd1-001-SD");
    let live = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage = [member, shizuku, member];
    game.state.player1.hand.cards.push(live);
    game.state.player2.hand.cards.clear(); // no live card for P2
    fill_decks(&mut game, filler);
    game.give_energy(5);

    let hand_before = game.state.player1.hand.cards.len();

    run_live_flow_p1_only(&mut game, live);

    assert_eq!(game.state.player1.hand.cards.len(), hand_before + 1,
        "P2 no live card → P1 wins → draws 1");
}

/// P rarity: same behavior.
#[test]
fn shizuku_p_rarity_same_as_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let shizuku = game.id("PL!N-bp4-003-P");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!-sd1-001-SD");
    let live = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage = [member, shizuku, member];
    game.state.player1.hand.cards.push(live);
    game.state.player2.hand.cards.clear();
    fill_decks(&mut game, filler);
    game.give_energy(5);

    let hand_before = game.state.player1.hand.cards.len();
    run_live_flow_p1_only(&mut game, live);
    assert_eq!(game.state.player1.hand.cards.len(), hand_before + 1,
        "P rarity: draws 1 card on win");
}

/// LiveSuccess ability only fires on success (not on loss/ tie).
/// With tied scores, the condition fails, so no draw occurs.
/// This is covered by shizuku_tied_score_no_draw.
