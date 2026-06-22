use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn fill_decks(game: &mut TestGame) {
    for _ in 0..10 {
        let f = game.new_id("PL!-sd1-010-SD");
        game.state.player1.main_deck.cards.push(f);
        let f = game.new_id("PL!-sd1-010-SD");
        game.state.player2.main_deck.cards.push(f);
    }
}

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

// ─── Target card: PL!N-pb1-004-R (朝香果林 ab#1) ───
//
// Live_start ability:
//   Reveal top card of deck.
//   If revealed card is a member card with cost <= 9:
//     add it to hand AND position change this member.
//   Otherwise (otherwise_condition):
//     discard the revealed card.
//
// The sequential structure is:
//   1. reveal (always)
//   2. move_cards → hand  (condition: member, cost <= 9)
//   3. position_change    (condition: member, cost <= 9)
//   4. move_cards → discard (otherwise_condition)
//
// Edge cases tested: condition met, condition not met (non-member),
//                    condition not met (expensive member),
//                    position change, hand/discard mutual exclusion,
//                    no position change when condition fails.

// ── Case 1: Condition met (member, cost <= 9) ──

#[test]
fn karin_reveal_condition_met_cheap_member_goes_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let karin = game.id("PL!N-pb1-004-R");
    let cheap = game.new_id("PL!-sd1-010-SD"); // member, cost 4
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-020-SD");

    fill_decks(&mut game);
    game.state.player1.main_deck.cards.insert(1, cheap);

    game.add_to_hand(live_card);
    game.add_to_hand(live_card);
    game.add_to_hand(karin);
    game.add_to_hand(filler);
    game.give_energy(20);
    game.play_to_stage(filler, MemberArea::LeftSide);
    game.play_to_stage(karin, MemberArea::Center);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&cheap),
        "Revealed card should be in hand when condition is met"
    );
}

#[test]
fn karin_reveal_condition_met_cheap_member_not_in_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let karin = game.id("PL!N-pb1-004-R");
    let cheap = game.new_id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-020-SD");

    fill_decks(&mut game);
    game.state.player1.main_deck.cards.insert(1, cheap);

    game.add_to_hand(live_card);
    game.add_to_hand(karin);
    game.add_to_hand(filler);
    game.give_energy(20);
    game.play_to_stage(filler, MemberArea::LeftSide);
    game.play_to_stage(karin, MemberArea::Center);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        !game.state.player1.waitroom.cards.contains(&cheap),
        "Revealed card should NOT be discarded when condition is met"
    );
}

#[test]
fn karin_reveal_condition_met_card_removed_from_deck_top() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let karin = game.id("PL!N-pb1-004-R");
    let cheap = game.new_id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-020-SD");

    fill_decks(&mut game);
    game.state.player1.main_deck.cards.insert(1, cheap);

    game.add_to_hand(live_card);
    game.add_to_hand(karin);
    game.add_to_hand(filler);
    game.give_energy(20);
    game.play_to_stage(filler, MemberArea::LeftSide);
    game.play_to_stage(karin, MemberArea::Center);

    assert_eq!(
        game.state.player1.main_deck.cards[1], cheap,
        "Precondition: cheap card is at index 1 (1 filler above it)"
    );

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        !game.state.player1.main_deck.cards.contains(&cheap),
        "Revealed card should be removed from deck (revealed then moved)"
    );
}

#[test]
fn karin_reveal_condition_met_hand_count_increases() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let karin = game.id("PL!N-pb1-004-R");
    let cheap = game.new_id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-020-SD");

    fill_decks(&mut game);
    game.state.player1.main_deck.cards.insert(1, cheap);

    game.add_to_hand(live_card);
    game.add_to_hand(karin);
    game.add_to_hand(filler);
    game.give_energy(20);
    let _hand_before = game.state.player1.hand.cards.len();
    game.play_to_stage(filler, MemberArea::LeftSide);
    game.play_to_stage(karin, MemberArea::Center);
    // After playing 2 members: hand decreased by 2
    let hand_after_play = game.state.player1.hand.cards.len();

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    // After setting live card: hand decreased by 1
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // During advance_to_live_card_set_p1 one card is drawn from the deck,
    // increasing hand size by 1.
    let expected = hand_after_play + 1; // +1 draw, -1 set_live_card, +1 reveal-add
    assert_eq!(
        game.state.player1.hand.cards.len(),
        expected,
        "Hand should gain exactly 1 card from the reveal"
    );
}

// ── Case 2: Condition not met (non-member revealed) ──

#[test]
fn karin_reveal_non_member_live_card_goes_to_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let karin = game.id("PL!N-pb1-004-R");
    let deck_top = game.new_id("PL!-sd1-020-SD"); // live card, not a member
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-020-SD");

    fill_decks(&mut game);
    game.state.player1.main_deck.cards.insert(1, deck_top);

    game.add_to_hand(live_card);
    game.add_to_hand(karin);
    game.add_to_hand(filler);
    game.give_energy(20);
    game.play_to_stage(filler, MemberArea::LeftSide);
    game.play_to_stage(karin, MemberArea::Center);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        game.state.player1.waitroom.cards.contains(&deck_top),
        "Non-member card should be discarded when condition fails"
    );
}

#[test]
fn karin_reveal_non_member_not_added_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let karin = game.id("PL!N-pb1-004-R");
    let deck_top = game.new_id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-020-SD");

    fill_decks(&mut game);
    game.state.player1.main_deck.cards.insert(1, deck_top);

    game.add_to_hand(live_card);
    game.add_to_hand(karin);
    game.add_to_hand(filler);
    game.give_energy(20);
    game.play_to_stage(filler, MemberArea::LeftSide);
    game.play_to_stage(karin, MemberArea::Center);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        !game.state.player1.hand.cards.contains(&deck_top),
        "Non-member card should NOT be added to hand"
    );
}

// ── Case 3: Condition not met (expensive member, cost > 9) ──

#[test]
fn karin_reveal_expensive_member_goes_to_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let karin = game.id("PL!N-pb1-004-R");
    let expensive = game.new_id("PL!N-pb1-004-R"); // member, cost 11 (>9)
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-020-SD");

    fill_decks(&mut game);
    game.state.player1.main_deck.cards.insert(1, expensive);

    game.add_to_hand(live_card);
    game.add_to_hand(karin);
    game.add_to_hand(filler);
    game.give_energy(20);
    game.play_to_stage(filler, MemberArea::LeftSide);
    game.play_to_stage(karin, MemberArea::Center);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        game.state.player1.waitroom.cards.contains(&expensive),
        "Card with cost >9 should be discarded when condition fails"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&expensive),
        "Expensive card should NOT be added to hand"
    );
}

// ── Case 4: Position change on condition met ──

#[test]
fn karin_reveal_condition_met_position_changes() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let karin = game.id("PL!N-pb1-004-R");
    let cheap = game.new_id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-020-SD");

    fill_decks(&mut game);
    game.state.player1.main_deck.cards.insert(1, cheap);

    game.add_to_hand(live_card);
    game.add_to_hand(karin);
    game.add_to_hand(filler);
    game.give_energy(20);
    game.play_to_stage(filler, MemberArea::LeftSide);
    game.play_to_stage(karin, MemberArea::Center);

    assert_eq!(
        game.state.player1.stage.stage[1], karin,
        "Precondition: karin starts in center"
    );

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Position change should move karin out of center
    assert!(
        game.state.player1.stage.stage[1] != karin,
        "Karin should no longer be in center after position change"
    );
    // Karin should still be on stage somewhere
    assert!(
        game.state.player1.stage.stage.contains(&karin),
        "Karin should still be on stage after position change"
    );
}

// ── Case 5: No position change when condition fails ──

#[test]
fn karin_reveal_condition_not_met_position_unchanged() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let karin = game.id("PL!N-pb1-004-R");
    let deck_top = game.new_id("PL!-sd1-020-SD"); // live card
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-020-SD");

    fill_decks(&mut game);
    game.state.player1.main_deck.cards.insert(1, deck_top);

    game.add_to_hand(live_card);
    game.add_to_hand(karin);
    game.add_to_hand(filler);
    game.give_energy(20);
    game.play_to_stage(filler, MemberArea::LeftSide);
    game.play_to_stage(karin, MemberArea::Center);

    assert_eq!(
        game.state.player1.stage.stage[1], karin,
        "Precondition: karin starts in center"
    );

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.player1.stage.stage[1], karin,
        "Karin should remain in center when condition fails (no position change)"
    );
}

// ── Case 6: Hand and discard are mutually exclusive ──

#[test]
fn karin_reveal_card_in_exactly_one_zone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let karin = game.id("PL!N-pb1-004-R");
    let deck_top = game.new_id("PL!-sd1-010-SD"); // member, cost 4
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-020-SD");

    fill_decks(&mut game);
    game.state.player1.main_deck.cards.insert(1, deck_top);

    game.add_to_hand(live_card);
    game.add_to_hand(karin);
    game.add_to_hand(filler);
    game.give_energy(20);
    game.play_to_stage(filler, MemberArea::LeftSide);
    game.play_to_stage(karin, MemberArea::Center);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let in_hand = game.state.player1.hand.cards.contains(&deck_top);
    let in_discard = game.state.player1.waitroom.cards.contains(&deck_top);
    assert!(
        in_hand != in_discard,
        "Revealed card must be in exactly one of hand or discard (in_hand={}, in_discard={})",
        in_hand,
        in_discard
    );
}

// ── Case 7: Deck top changed correctly in both paths ──

#[test]
fn karin_reveal_condition_met_deck_top_becomes_filler() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let karin = game.id("PL!N-pb1-004-R");
    let cheap = game.new_id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-020-SD");

    fill_decks(&mut game);
    game.state.player1.main_deck.cards.insert(1, cheap);
    // After insert(1, cheap): [filler_0, cheap, filler_1, ...]
    // 1 draw removes filler_0 → [cheap, filler_1, ...]
    // After reveal removes cheap → [filler_1, ...]
    // So deck top after reveal = filler_1, which was at index 2 at capture.
    let second_card = game.state.player1.main_deck.cards[2];

    game.add_to_hand(live_card);
    game.add_to_hand(karin);
    game.add_to_hand(filler);
    game.give_energy(20);
    game.play_to_stage(filler, MemberArea::LeftSide);
    game.play_to_stage(karin, MemberArea::Center);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.player1.main_deck.cards[0], second_card,
        "Second card should become new deck top after reveal"
    );
}

#[test]
fn karin_reveal_condition_failed_deck_top_becomes_filler() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let karin = game.id("PL!N-pb1-004-R");
    let deck_top = game.new_id("PL!-sd1-020-SD"); // live card
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-020-SD");

    fill_decks(&mut game);
    game.state.player1.main_deck.cards.insert(1, deck_top);
    // After insert(1, deck_top): [filler_0, deck_top, filler_1, ...]
    // 1 draw removes filler_0 → [deck_top, filler_1, ...]
    // After reveal removes deck_top → [filler_1, ...]
    let second_card = game.state.player1.main_deck.cards[2];

    game.add_to_hand(live_card);
    game.add_to_hand(karin);
    game.add_to_hand(filler);
    game.give_energy(20);
    game.play_to_stage(filler, MemberArea::LeftSide);
    game.play_to_stage(karin, MemberArea::Center);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.player1.main_deck.cards[0], second_card,
        "Second card should become new deck top after reveal + discard"
    );
}
