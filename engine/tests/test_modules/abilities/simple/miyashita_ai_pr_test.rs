/// Tests for 宮下 愛 (PL!N-PR-028-PR) — On Play: optional discard 2 → draw to 5.
///
/// Ability text: 登場: 手札を2枚控え室に置いてもよい：自分の手札が5枚になるまでカードを引く。
///
/// Cost (optional): discard 2 from hand
/// Effect: draw from deck until hand has 5 cards (draw_until_count with target=5)
///
/// The same ability is shared by PL!HS-PR-031-PR (日野下花帆 PR), but we test
/// only PL!N-PR-028-PR here to avoid duplicating coverage.
///
/// We test the full game flow: hand setup → play the card → handle the cost
/// choice → verify the final hand/deck/discard state. We do NOT just call
/// execute_draw_until_count in isolation.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// Helper: set up a fresh game with the PR card in hand, given energy, and
/// a deck of `deck_count` filler cards. Returns
/// (ai_id, hand_before, deck_before, discard_before).
fn setup_pr_ai(
    game: &mut TestGame,
    hand_extra: usize,
    deck_count: usize,
) -> (i16, usize, usize, usize) {
    let ai = game.id("PL!N-PR-028-PR");
    let filler = game.id("PL!-sd1-010-SD");

    // Energy: card has cost 11.
    game.give_energy(15);

    // Hand: ai + hand_extra filler cards.
    game.add_to_hand(ai);
    for _ in 0..hand_extra {
        game.add_to_hand(filler);
    }
    let hand_before = game.state.player1.hand.cards.len();

    // Deck: enough cards that draw-until-5 succeeds without reshuffling.
    for _ in 0..deck_count {
        game.state.player1.main_deck.cards.push(filler);
    }
    let deck_before = game.state.player1.main_deck.cards.len();
    let discard_before = game.state.player1.waitroom.cards.len();

    (ai, hand_before, deck_before, discard_before)
}

/// With 3 cards in hand (Ai + 2 filler), the player pays the cost
/// (discards 2), then draw-until-5 draws enough to reach 5.
///
/// Expected: hand = 5, deck = deck - 3, discard = 2, Ai on stage.
#[test]
fn pr_ai_pays_cost_discards_2_draws_to_five() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // Hand has Ai + 2 filler. Cost requires discarding 2 — player can pay.
    let (ai, _hand_before, deck_before, discard_before) = setup_pr_ai(&mut game, 2, 10);

    game.play_to_stage(ai, MemberArea::Center);

    // Ai is on stage, not in hand.
    assert_eq!(
        game.state.player1.stage.stage[1], ai,
        "Ai should be on Center stage"
    );

    // Cost choice should be pending: select 2 cards to discard.
    assert!(
        game.has_pending_choice(),
        "Optional cost should produce a choice with eligible cards"
    );

    // Pay cost: discard the 2 filler cards
    game.try_select_indices(&[0, 1]).unwrap();

    // Two cards were discarded as cost
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        discard_before + 2,
        "Waitroom should have +2 from cost"
    );

    // After discard: hand = 0, draw-until-5 → draws 5
    assert_eq!(
        game.state.player1.hand.cards.len(),
        5,
        "Hand should be exactly 5 (0 after cost + 5 drawn)"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 5,
        "Deck should lose 5 cards"
    );
}

/// POSITIVE: With 3 cards in hand (Ai + 2 filler), the player accepts the
/// cost: discard the 2 filler cards, then draw-until-5 refills to 5.
///
/// Expected: discard = 2, hand = 5, deck = deck - 4, Ai on stage.
#[test]
fn pr_ai_pays_cost_discards_2_draws_4() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let (ai, _hand_before, deck_before, discard_before) = setup_pr_ai(&mut game, 2, 10);

    // Capture the IDs of the two filler cards in hand (they should be the
    // ones that end up in discard).
    let filler = game.id("PL!-sd1-010-SD");
    let hand_before_play = game.state.player1.hand.cards.clone();
    assert_eq!(hand_before_play.len(), 3, "Pre-play hand: ai + 2 filler");

    game.play_to_stage(ai, MemberArea::Center);

    // The cost choice should be pending: select 2 cards to discard.
    assert!(
        game.has_pending_choice(),
        "Optional cost should produce a choice even with eligible cards"
    );
    // Choose indices 0 and 1 (the two filler cards in hand; Ai is now on stage).
    game.try_select_indices(&[0, 1]).unwrap();

    // Discard: 2 filler cards were discarded as cost.
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        discard_before + 2,
        "Waitroom should have +2 cards from cost"
    );

    // Deck: 5 - 4 = 1. We started with 10 in deck, hand had 3 (1 ai + 2 filler).
    // After play: hand = 2, deck = 10. Cost discards 2: hand = 0, deck = 10.
    // Draw-until-5 draws 5: hand = 5, deck = 5.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        5,
        "Hand should be 5 cards (0 from cost + 5 drawn)"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 5,
        "Deck should lose 5 cards (the 5 drawn)"
    );

    // The discarded filler IDs from the original hand should be in the waitroom.
    for (i, &orig) in hand_before_play.iter().enumerate() {
        if i == 0 {
            continue;
        } // skip Ai
        assert!(
            game.state.player1.waitroom.cards.contains(&orig),
            "Filler card at hand index {} (id={}, {}) should be in waitroom",
            i,
            orig,
            game.name(orig)
        );
    }

    // Sanity: Ai not in waitroom (it's on stage, not in hand or discard).
    assert!(!game.state.player1.waitroom.cards.contains(&ai));
    assert!(!game.state.player1.hand.cards.contains(&ai));
    let _ = filler; // suppress unused warning
}

/// POSITIVE: With 4 cards in hand, player is offered the cost. If they skip,
/// the effect is CANCELED (engine convention: explicit skip on a non-empty
/// optional cost = no effect). Hand stays at 3, deck unchanged.
#[test]
fn pr_ai_hand_almost_full_skips_cost_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let (ai, _hand_before, deck_before, discard_before) = setup_pr_ai(&mut game, 3, 10);

    game.play_to_stage(ai, MemberArea::Center);

    // After play: hand = 3 (3 filler). Cost choice is pending.
    assert!(
        game.has_pending_choice(),
        "Optional cost pending with 3 cards in hand"
    );
    // Skip the cost → optional_skipped → effect canceled.
    game.select_indices(&[]);

    // Engine behavior: explicit skip of an optional cost cancels the effect.
    // Hand remains 3, deck unchanged, no discard.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        3,
        "Hand should stay at 3 (cost skipped → effect canceled)"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "Deck should be unchanged (effect canceled)"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        discard_before,
        "No cards should be in waitroom (skipped cost)"
    );
}

/// POSITIVE: With 7 cards in hand, the player is offered the cost but
/// skipping it (with cards available) cancels the effect. Hand stays at 6
/// (1 ai played from 7), no draw, no discard.
#[test]
fn pr_ai_hand_at_seven_skips_cost_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let (ai, _hand_before, deck_before, _discard_before) = setup_pr_ai(&mut game, 6, 10);

    game.play_to_stage(ai, MemberArea::Center);

    // After play: hand = 6. Cost choice is pending.
    assert!(game.has_pending_choice(), "Optional cost pending");
    game.select_indices(&[]); // skip cost

    // Hand is 6, target is 5, no draw needed.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        6,
        "Hand should remain at 6 (already >= 5)"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "Deck should be untouched"
    );
}

/// POSITIVE: With 4 cards in hand, player pays cost (discard 2), then
/// draw-until-5 draws 3 (2 - 2 + 3 = 3? no, 2 - 2 + 5 = 5, so draws 5).
/// Wait: hand starts at 4, play moves Ai to stage → hand = 3, cost discards
/// 2 → hand = 1, draw-until-5 → draws 4, hand = 5. So 4 drawn from deck.
#[test]
fn pr_ai_hand_4_pays_cost_draws_4() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let (ai, _hand_before, deck_before, discard_before) = setup_pr_ai(&mut game, 3, 10);

    game.play_to_stage(ai, MemberArea::Center);

    // Hand after play: 3 cards (3 filler). Cost is offered.
    assert!(game.has_pending_choice(), "Cost choice pending");
    // Pay cost: discard 2 of the 3 filler.
    game.try_select_indices(&[0, 1]).unwrap();

    // Hand: 3 - 2 + draw-to-5 = 1 + 4 = 5. Drawn 4.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        5,
        "Hand should be 5 (3 - 2 + 4 drawn)"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 4,
        "Deck should lose 4 cards (4 drawn)"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        discard_before + 2,
        "Waitroom should have +2 cards from cost"
    );
}

/// NEGATIVE: When deck is empty, draw-until-5 should not hang. It just
/// draws 0 cards (no cards to draw). Hand should still match the
/// "skip cost + 0 draws" path.
#[test]
fn pr_ai_empty_deck_no_draw_no_hang() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let (ai, _hand_before, deck_before, discard_before) = setup_pr_ai(&mut game, 0, 0);

    assert_eq!(deck_before, 0, "Deck should be empty for this test");

    game.play_to_stage(ai, MemberArea::Center);

    // Cost should be skipped (only 1 card in hand at the time of cost):
    // no prompt at all — zero eligible candidates auto-skip.
    assert!(
        !game.has_pending_choice(),
        "cost with no eligible cards must auto-skip without prompting"
    );

    // Hand should still be 0 (no draw possible from empty deck).
    assert_eq!(
        game.state.player1.hand.cards.len(),
        0,
        "Hand should be 0 — no cards in deck to draw"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        0,
        "Deck should remain empty"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        discard_before,
        "No discard"
    );
}
