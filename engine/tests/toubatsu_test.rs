/// PL!SP-bp2-011-R (鬼塚冬毬) Q118
///
/// {{toujyou.png|登場}}自分の控え室にある、カード名の異なるライブカードを2枚選ぶ。
/// 選べなかった場合、このカードのうち1枚を選ぶ。これにより相手に選ばれたカードを
/// 自分の手札に加える。
///
/// Q118: If you can't select 2 different-named live cards (e.g. only 1 in discard),
/// can you still select 1 and add it to hand? A: No — the effect requires 2 distinct
/// names to proceed.

mod helpers;
use helpers::*;
use rabuka_engine::zones::MemberArea;

/// Positive: 2 distinct live cards in discard → ability proceeds.
#[test]
fn toubatsu_q118_2_distinct_live_cards_works() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let toubatsu = game.id("PL!SP-bp2-011-R");
    let live_a = game.id("PL!-sd1-019-SD");  // START:DASH!!
    let live_b = game.id("PL!N-sd1-028-SD"); // Dream with You (different name)
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_hand(toubatsu);
    game.add_to_hand(filler);

    // Discard: 2 live cards with different names
    game.add_to_discard(live_a);
    game.add_to_discard(live_b);

    game.give_energy(15);
    game.play_to_stage(toubatsu, MemberArea::Center);

    // Debut fires: select 2 distinct live cards from discard
    if game.has_pending_choice() {
        // First choice: select which 2 distinct live cards
        game.select_indices(&[0, 1]);
    }

    // After selection, the opponent chooses 1 → it goes to hand
    // Handle opponent choice if present
    if game.has_pending_choice() {
        game.select_option(0); // opponent selects first card
    }

    // One of the live cards should now be in hand
    let in_hand = game.state.player1.hand.cards.contains(&live_a)
        || game.state.player1.hand.cards.contains(&live_b);
    assert!(in_hand,
        "One of the 2 distinct live cards should be added to hand");
}

/// Q118: Only 1 live card in discard → ability fails, nothing added to hand.
#[test]
fn toubatsu_q118_1_live_card_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let toubatsu = game.id("PL!SP-bp2-011-R");
    let live_a = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_hand(toubatsu);
    game.add_to_hand(filler);
    game.give_energy(15);

    // Discard: only 1 live card → can't pick 2 distinct
    game.add_to_discard(live_a);

    game.play_to_stage(toubatsu, MemberArea::Center);

    // Debut fires: select 2 distinct live cards — only 1 available
    // Engine returns early (no choice created) since distinct filter fails

    // Q118: Live card should NOT be in hand (effect required 2 distinct)
    assert!(!game.state.player1.hand.cards.contains(&live_a),
        "Live card should not be added: effect needs 2 distinct cards");
}
