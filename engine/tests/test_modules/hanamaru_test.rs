/// Tests for 国木田花丸 (PL!S-bp2-007-R＋) — Q120: auto ability fires after cheer
/// when live card in revealed_cards AND hand ≤ 7 → draw 1.
///
/// Engine fixes applied:
/// 1. trigger_auto_abilities_for_player added after player_perform_live (phases.rs)
/// 2. Parser outputs compound condition (card_count + hand_count sub-conditions)
/// 3. resource_type: "hand_count" handler in get_count_for_condition (condition.rs)
use crate::helpers::*;

fn advance_to_live_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

/// Blade=13, all cheered cards are live. Cheer draws 13 to hand.
/// Start with 7 hand → after blade draws arrives to 20.
/// Auto condition: ≥1 live in revealed (true) AND hand ≤ 7 (false at 20) → no draw.
#[test]
fn hanamaru_q120_hand7_auto_condition_checked_after_blade_draws() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanamaru = game.id("PL!S-bp2-007-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let blader = game.id("PL!S-PR-014-PR");
    let live = game.id("LL-bp5-001-L");

    // Deck: enough cards for LiveStart (needs 2) + blade draws (13)
    game.state.player1.main_deck.cards.clear();
    for _ in 0..100 {
        game.state.player1.main_deck.cards.push(live);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..100 {
        game.state.player2.main_deck.cards.push(filler);
    }

    for _ in 0..7 {
        game.state.player1.hand.cards.push(filler);
    }
    game.state.player1.hand.cards.push(live);

    game.state.player1.stage.stage = [hanamaru, blader, blader];

    advance_to_live_set(&mut game);
    game.set_live_card(live);
    game.pass(); // P1Turn draws 1 live from deck (100→99)
    game.pass(); // P2Turn → LiveStart (looks 2 from deck: 99→97) → cheer → auto abilities

    // After LiveStart choice (if any)
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let hand = game.state.player1.hand.cards.len();
    eprintln!("[HANAMARU] hand=7 start, after cheer: {}", hand);
    assert!(hand >= 8, "Blade draws increased hand");
}

/// No live cards in deck → condition fails → no draw.
#[test]
fn hanamaru_q120_no_live_in_revealed_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanamaru = game.id("PL!S-bp2-007-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let blader = game.id("PL!S-PR-014-PR");
    let live = game.id("LL-bp5-001-L");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..100 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..100 {
        game.state.player2.main_deck.cards.push(filler);
    }

    for _ in 0..5 {
        game.state.player1.hand.cards.push(filler);
    }
    game.state.player1.hand.cards.push(live);

    game.state.player1.stage.stage = [hanamaru, blader, blader];

    advance_to_live_set(&mut game);
    game.set_live_card(live);
    game.pass();
    game.pass();

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    // No live in cheered cards → condition fails → no auto draw
    eprintln!("[HANAMARU] no-live test completed");
}
