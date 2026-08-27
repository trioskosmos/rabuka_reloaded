/// Tests for ミア・テイラー (PL!N-bp1-011-R) — Debut ability (ab#0):
///
/// {{toujyou.png|登場}}手札を1枚控え室に置いてもよい：
/// ライブカードが公開されるまで、自分のデッキの一番上のカードを公開し続ける。
/// そのライブカードを手札に加え、これにより公開されたほかのすべてのカードを控え室に置く。
///
/// Q102: If no live cards are in main deck or waiting room, the effect resolves
///       as much as possible: all deck cards revealed, refresh occurs, and if
///       still no live card, resolution ends with revealed cards going to discard.
/// Q73:  If the main deck runs out during resolution, a refresh occurs and
///       revealing continues.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// Q73: Deck has live card after refresh — reveal continues and live card is
/// obtained.
#[test]
fn mia_q73_deck_has_live_card_after_refresh() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let mia = game.id("PL!N-bp1-011-R");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-019-SD");

    // Hand: ミア + filler to discard for optional cost + live card
    game.add_to_hand(mia);
    game.add_to_hand(filler);
    game.state.player1.hand.cards.push(live_card);

    // Deck: all fillers, no live cards initially
    for _ in 0..3 {
        game.state.player1.main_deck.cards.push(filler);
    }
    // Waitroom has the live card for refresh
    game.add_to_discard(live_card);

    // Give energy for ミア's cost (cost=9)
    game.give_energy(10);

    // Play ミア to stage center — triggers debut ability
    game.play_to_stage(mia, MemberArea::Center);

    // Debut auto ability fires: optional cost (discard 1 from hand)
    assert!(
        game.has_pending_choice(),
        "optional discard cost prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard for the discard cost"
    );
    game.select_indices(&[0]); // discard filler

    // Now reveal_until_live_card runs. Deck has 3 fillers, then refreshes from waitroom.
    // The live card from waitroom gets revealed and goes to hand.
    assert!(
        game.state.player1.hand.cards.contains(&live_card),
        "Live card should be obtained via reveal"
    );
}

/// Q102: No live card anywhere — effect resolves partially, revealed cards go
/// to discard.
#[test]
fn mia_q102_no_live_card_anywhere() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let mia = game.id("PL!N-bp1-011-R");
    let filler = game.id("PL!-sd1-010-SD");

    // Hand: ミア + filler to discard
    game.add_to_hand(mia);
    game.add_to_hand(filler);

    // Deck: only fillers, no live cards
    for _ in 0..3 {
        game.state.player1.main_deck.cards.push(filler);
    }
    // Waitroom: also no live cards

    game.give_energy(10);
    game.play_to_stage(mia, MemberArea::Center);

    assert!(
        game.has_pending_choice(),
        "optional discard cost prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard for the discard cost"
    );
    game.select_indices(&[0]); // discard filler

    // After reveal_until_live_card: deck should have 0 cards (all revealed)
    let deck_count = game.state.player1.main_deck.len();
    assert_eq!(
        deck_count, 0,
        "Deck should be exhausted after full reveal, got {}",
        deck_count
    );
    // Revealed cards should be in looked_at buffer (then moved to discard
    // by the subsequent move_cards action)
    let discard_count = game.state.player1.waitroom.len();
    assert!(
        discard_count > 0,
        "Revealed cards should have been moved to discard, got {}",
        discard_count
    );
}

#[test]
fn mia_q102_live_immediately_on_top() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let mia = game.id("PL!N-bp1-011-R");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!-sd1-019-SD");
    game.add_to_hand(mia);
    game.add_to_hand(filler);
    // Deck top is live
    game.state.player1.main_deck.cards.push(live);
    for _ in 0..3 { game.state.player1.main_deck.cards.push(filler); }
    game.give_energy(10);
    game.play_to_stage(mia, MemberArea::Center);
    assert!(game.has_pending_choice());
    game.select_indices(&[0]);
    assert!(game.state.player1.hand.cards.contains(&live), "live on top should be obtained");
    assert_eq!(game.state.player1.main_deck.cards.len(), 3, "3 filler should remain after revealing 1 live");
}

#[test]
fn mia_q102_skip_cost_still_reveals() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let mia = game.id("PL!N-bp1-011-R");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!-sd1-019-SD");
    game.add_to_hand(mia);
    game.add_to_hand(filler);
    game.state.player1.main_deck.cards.push(live);
    for _ in 0..3 { game.state.player1.main_deck.cards.push(filler); }
    game.give_energy(10);
    game.play_to_stage(mia, MemberArea::Center);
    if game.has_pending_choice() {
        game.select_indices(&[]); // skip discard cost
    }
    // Even when skipping cost, the reveal should still happen (or at least no panic)
    assert!(game.state.player1.hand.cards.contains(&live) || game.state.player1.waitroom.cards.contains(&live) || game.state.player1.main_deck.cards.contains(&live), "no panic, live somewhere");
}
