/// Tests for 国木田花丸 (PL!S-bp2-007-R＋) — Q120: after a revealed live card
/// resolves, the automatic draw only applies when the hand is still at most 7.
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

/// Starting at seven cards, resolving a revealed live reaches eight before the
/// automatic ability resolves, so the automatic draw must not fire.
#[test]
fn revealed_live_above_hand_threshold_does_not_draw_again() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanamaru = game.id("PL!S-bp2-007-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let blader = game.id("PL!S-PR-014-PR");
    let live = game.id("LL-bp5-001-L");
    let other_live = game.id("LL-bp5-002-L");

    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(other_live);
    game.state.player1.main_deck.cards.push(live);
    for _ in 0..98 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..100 {
        game.state.player2.main_deck.cards.push(filler);
    }

    for _ in 0..7 {
        game.state.player1.hand.cards.push(filler);
    }
    game.state.player1.hand.cards.push(live);

    let blader_copy2 = game.new_id("PL!S-PR-014-PR");
    game.state.player1.stage.stage = [hanamaru, blader, blader_copy2];

    advance_to_live_set(&mut game);
    game.set_live_card(live);
    game.pass(); // P1Turn draws 1 live from deck (100→99)
    game.pass(); // P2Turn → LiveStart (looks 2 from deck: 99→97) → cheer → auto abilities

    // After LiveStart choice (if any)
    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();

    let hand = game.state.player1.hand.cards.as_slice();
    assert_eq!(hand.len(), 8, "only the selected live should enter hand");
    assert_eq!(
        hand.iter()
            .filter(|&&id| id == live || id == other_live)
            .count(),
        1,
        "exactly one distinct revealed live should enter hand"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        99,
        "one of the two revealed cards should be removed from the deck"
    );
}

/// No live cards in deck → condition fails → no draw.
#[test]
fn revealed_filler_does_not_satisfy_live_condition() {
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

    let blader_copy2 = game.new_id("PL!S-PR-014-PR");
    game.state.player1.stage.stage = [hanamaru, blader, blader_copy2];

    advance_to_live_set(&mut game);
    game.set_live_card(live);
    game.pass();
    game.pass();

    // No live cards in revealed (deck is all filler) → condition (live in revealed) fails → no auto draw
    let hand = game.state.player1.hand.cards.len();
    assert_eq!(
        hand, 6,
        "no live in revealed → condition fails → no auto draw (hand should stay 6, got {})",
        hand
    );
}
