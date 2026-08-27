/// Q267 — 天王寺璃奈 PL!N-bp7-009-R ab#0 (登場).
///
/// 登場：自分と相手はそれぞれ、自身のデッキの上からカードを7枚控え室に置く。
///
/// Official QA Q267: when the main deck hits exactly 0 DURING the mill, the
/// processing is interrupted and a refresh happens immediately; the interrupted
/// mill then resumes from the refreshed deck. Because the refresh happens
/// mid-effect, the cards already milled leave the waitroom (shuffled back into
/// the deck) before the rest of the mill is taken.
///
/// We drive the real engine rule end-to-end through the card's 登場 ability
/// (target "both" → self then opponent each mill their own deck), varying deck
/// sizes to pin down the refresh boundary:
///   deck=3 + waitroom → deck hits 0 mid-mill → refresh → mill completes to 7
///   deck=7 exactly   → no overdraw → NO refresh, even with waitroom material
///   deck=5           → refresh → mill still reaches 7 total (5 + 2)
///   both decks small → each player's deck refreshes on its own 0
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const RINA: &str = "PL!N-bp7-009-R"; // 天王寺璃奈, ab#0 登場 (mill 7 both players)
const FILLER: &str = "PL!-sd1-010-SD"; // ability-free deck/waitroom card

/// Trigger 天王寺璃奈's 登場 auto-ability. The effect targets "both", so the
/// engine resolves it for self (p1) then opponent (p2); each mill has no
/// choices, so no further prompts remain.
fn trigger_debut(game: &mut TestGame, rina: i16) {
    let card = game.db.get_card(rina).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("登場"))
        .expect("card should have a 登場 ability");
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        AbilityTrigger::Debut,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(rina),
        None,
        None,
    );
    game.state.activating_card = Some(rina);
    game.state.process_pending_auto_abilities(&pid);
    game.drain_auto_ability_choices();
}

/// Place 天王寺璃奈 on p1 center, seed each player's deck/waitroom, trigger debut.
fn setup_and_trigger(
    game: &mut TestGame,
    p1_deck: usize,
    p1_waitroom: usize,
    p2_deck: usize,
    p2_waitroom: usize,
) {
    let rina = game.id(RINA);
    game.state.player1.stage.stage[1] = rina;

    game.state.player1.main_deck.cards.clear();
    for _ in 0..p1_deck {
        game.state.player1.main_deck.cards.push(game.id(FILLER));
    }
    game.state.player1.waitroom.cards.clear();
    for _ in 0..p1_waitroom {
        game.state.player1.waitroom.cards.push(game.id(FILLER));
    }

    game.state.player2.main_deck.cards.clear();
    for _ in 0..p2_deck {
        game.state.player2.main_deck.cards.push(game.id(FILLER));
    }
    game.state.player2.waitroom.cards.clear();
    for _ in 0..p2_waitroom {
        game.state.player2.waitroom.cards.push(game.id(FILLER));
    }

    trigger_debut(game, rina);
}

// ====================================================================
// Deck hits 0 mid-mill → immediate refresh → mill completes to 7.
// p1: deck=3, waitroom=7. 3 drawn, refresh (3+7=10 back to deck), 4 more drawn.
// Final p1: waitroom=4, deck=6. p2 (30 deck): waitroom=7, deck=23, no refresh.
// ====================================================================
#[test]
fn q267_deck_exhausts_mid_mill_refreshes_and_completes() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    setup_and_trigger(&mut game, 3, 7, 30, 0);

    // Core Q267: refresh happened DURING the mill.
    assert!(
        game.state.player1.deck_refreshed_this_turn,
        "p1 deck hit 0 mid-mill → must refresh immediately (Q267)"
    );
    // The initially-milled 3 cards left the waitroom (shuffled back to deck).
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        4,
        "p1: refresh happened mid-mill, so only the 4 cards drawn from the refreshed deck remain in the waitroom"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        6,
        "p1: 3 original + 7 waitroom = 10, minus 4 drawn = 6 in deck"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len() + game.state.player1.main_deck.cards.len(),
        10,
        "p1: no cards lost (conservation)"
    );

    // p2 had a full deck → milled a clean 7, no refresh.
    assert!(
        !game.state.player2.deck_refreshed_this_turn,
        "p2 had 30 cards → must NOT refresh"
    );
    assert_eq!(
        game.state.player2.waitroom.cards.len(),
        7,
        "p2: milled a full 7 with no refresh"
    );
    assert_eq!(game.state.player2.main_deck.cards.len(), 23, "p2 deck = 30 - 7");
}

// ====================================================================
// Deck has EXACTLY 7 → the mill consumes it all, remaining hits 0 → loop
// exits before the top-of-loop refresh check → NO refresh, even though the
// waitroom is non-empty (overdraw is what triggers refresh, not deck-empty).
// ====================================================================
#[test]
fn q267_deck_exactly_seven_no_refresh() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // p1: deck=7 (exact), waitroom=3 (refresh material present but unused).
    setup_and_trigger(&mut game, 7, 3, 30, 0);

    assert!(
        !game.state.player1.deck_refreshed_this_turn,
        "deck exactly 7 → no overdraw → no refresh, despite waitroom material"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        0,
        "p1: all 7 deck cards milled"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        10,
        "p1: 3 pre-existing waitroom cards + 7 milled = 10"
    );
}

// ====================================================================
// Deck = 5 (< 7) → refresh mid-mill → mill still reaches 7 total (5 + 2).
// Final p1: waitroom=2, deck=6 (5+3=8 total).
// ====================================================================
#[test]
fn q267_deck_five_refresh_completes_to_seven() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    setup_and_trigger(&mut game, 5, 3, 30, 0);

    assert!(
        game.state.player1.deck_refreshed_this_turn,
        "p1 deck (5) ran out mid-mill → refresh"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        2,
        "p1: 2 cards drawn from the refreshed deck after refresh"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        6,
        "p1: 5 + 3 = 8 total, minus 2 drawn = 6 in deck"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len() + game.state.player1.main_deck.cards.len(),
        8,
        "p1: conservation"
    );
}

// ====================================================================
// Both players' decks are small → EACH player's deck refreshes at its own 0
// (the それぞれ "each" aspect). p1 deck=3+waitroom7, p2 deck=3+waitroom7.
// Each: waitroom=4, deck=6.
// ====================================================================
#[test]
fn q267_both_players_refresh_their_own_deck() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    setup_and_trigger(&mut game, 3, 7, 3, 7);

    assert!(
        game.state.player1.deck_refreshed_this_turn,
        "p1 deck hit 0 → refresh"
    );
    assert!(
        game.state.player2.deck_refreshed_this_turn,
        "p2 deck hit 0 during its own mill → refresh"
    );
    assert_eq!(game.state.player1.waitroom.cards.len(), 4, "p1 waitroom");
    assert_eq!(game.state.player2.waitroom.cards.len(), 4, "p2 waitroom");
    assert_eq!(game.state.player1.main_deck.cards.len(), 6, "p1 deck");
    assert_eq!(game.state.player2.main_deck.cards.len(), 6, "p2 deck");
}
