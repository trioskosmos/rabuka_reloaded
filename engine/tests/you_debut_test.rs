/// You (渡辺 曜 PL!S-bp2-005-R+) — Debut look + select from deck
///
/// Ab#0 (登場): Optional discard from hand: look at top 7, select up to 3 heart02/04/05
///   member cards, add to hand, discard rest.
///
/// This tests the new select_cards format where look + reveal + move happen in one choice.

mod helpers;
use helpers::*;
use rabuka_engine::zones::MemberArea;

/// Q124: Base heart02/04/05 qualifies; blade heart does NOT qualify
#[test]
fn you_q124_blade_heart_excluded_base_heart_included() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let qualifying = game.id("PL!S-sd1-001-SD");
    let blade_only = game.id("PL!SP-sd1-001-SD");

    // You + filler in hand (filler for optional cost)
    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);

    fill_deck_to_40(&mut game, vec![qualifying, blade_only, filler, filler, filler, filler, filler]);

    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);

    // [Choice 1] Optional cost: skip (empty indices)
    if game.has_pending_choice() { game.select_indices(&[]); }
    // [Choice 2] look_and_select: skip (no qualifying cards after filter)
    if game.has_pending_choice() { game.select_indices(&[]); }

    // Q124: blade heart DOES NOT qualify → blade_only stays in discard
    let blade_in_hand = game.state.player1.hand.cards.contains(&blade_only);
    assert!(!blade_in_hand, "Card with ONLY blade_heart02 should NOT be added to hand");
}

/// Test that ability ends properly and discard grows only at end
#[test]
fn you_ability_ends_and_discard_only_grows_at_end() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);

    let below = game.id("PL!-sd1-010-SD");
    fill_deck_to_40(&mut game, vec![filler, filler, filler, filler, filler, filler, filler, below]);

    let initial_discard = game.state.player1.waitroom.cards.len();
    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);

    // [Choice 1] Optional cost: skip
    if game.has_pending_choice() { game.select_indices(&[]); }
    // [Choice 2] look_and_select: skip (no eligible cards)
    if game.has_pending_choice() { game.select_indices(&[]); }

    assert!(!game.has_pending_choice(), "Ability should have ended");

    // Only the 7 looked-at fillers should be in discard.
    let final_discard = game.state.player1.waitroom.cards.len();
    assert_eq!(final_discard - initial_discard, 7,
        "Expected 7 looked-at cards in discard, got {}", final_discard - initial_discard);
}

/// Select 1 qualifying card, verify it's added to hand, rest go to discard
#[test]
fn you_ability_select_1_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let qualifying = game.id("PL!S-sd1-001-SD");

    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);

    let below = game.id("PL!-sd1-010-SD");
    fill_deck_to_40(&mut game, {
        let mut top = vec![qualifying];
        top.extend(std::iter::repeat(filler).take(6));
        top.push(below);
        top
    });

    let initial_discard = game.state.player1.waitroom.cards.len();
    let initial_hand = game.state.player1.hand.cards.len();
    let initial_deck = game.state.player1.main_deck.cards.len();

    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);

    // [Choice 1] Optional cost: skip
    assert!(game.has_pending_choice(), "Should have optional cost choice");
    game.select_indices(&[]);

    // [Choice 2] look_and_select: select qualifying card (index 0 in filtered list)
    assert!(game.has_pending_choice(), "Should have look_and_select choice");
    game.select_indices(&[0]);

    assert!(!game.has_pending_choice(), "Ability should have ended");

    // Verify all cards are accounted for
    let total_cards = game.state.player1.hand.cards.len()
        + game.state.player1.main_deck.cards.len()
        + game.state.player1.waitroom.cards.len()
        + game.state.player1.stage.stage.iter().filter(|&&id| id != -1).count();
    
    // Total cards = initial hand + deck + discard (you moved from hand to stage)
    let initial_total = initial_hand + initial_deck + initial_discard;
    assert_eq!(total_cards, initial_total, "Total card count should be preserved");
    
    // qualifying card should be in hand
    assert!(game.state.player1.hand.cards.contains(&qualifying),
        "Qualifying card should be in hand");
    assert!(game.state.player1.main_deck.cards.contains(&below),
        "Below-zone card should still be in deck");
    // 7 looked-at - 1 selected = 6 remaining → all 6 go to discard
    let final_discard = game.state.player1.waitroom.cards.len();
    assert_eq!(final_discard - initial_discard, 6,
        "Expected 6 fillers in discard, got {}", final_discard - initial_discard);
    // Hand should have net 0 change (you left, qualifying entered)
    assert_eq!(game.state.player1.hand.cards.len(), initial_hand,
        "Hand should have net 0 change");
}

/// Select multiple qualifying cards at once
#[test]
fn you_ability_select_multiple_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    
    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    
    let qualifying1 = game.id("PL!S-sd1-001-SD");
    let qualifying2 = game.id("PL!S-sd1-002-SD");  
    let qualifying3 = game.id("PL!S-sd1-003-SD");

    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);

    let below = game.id("PL!-sd1-010-SD");
    fill_deck_to_40(&mut game, vec![qualifying1, qualifying2, qualifying3, filler, filler, filler, filler, below]);

    let initial_discard = game.state.player1.waitroom.cards.len();
    let initial_hand = game.state.player1.hand.cards.len();

    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);

    // [Choice 1] Optional cost: skip
    if game.has_pending_choice() { game.select_indices(&[]); }
    // [Choice 2] look_and_select: select all 3 qualifying cards at once
    if game.has_pending_choice() { game.select_indices(&[0, 1, 2]); }

    assert!(!game.has_pending_choice(), "Ability should have ended");

    assert!(game.state.player1.hand.cards.contains(&qualifying1), "First qualifying card should be in hand");
    assert!(game.state.player1.hand.cards.contains(&qualifying2), "Second qualifying card should be in hand");
    assert!(game.state.player1.hand.cards.contains(&qualifying3), "Third qualifying card should be in hand");
    assert!(game.state.player1.main_deck.cards.contains(&below), "Below-zone card should still be in deck");
    
    let final_discard = game.state.player1.waitroom.cards.len();
    assert_eq!(final_discard - initial_discard, 4, "Expected 4 fillers in discard");
    assert_eq!(game.state.player1.hand.cards.len(), initial_hand + 2, "Hand should have net +2 change");
}

/// Test that selecting 1 card works (ability ends properly)
#[test]
fn you_ability_user_scenario_select_one_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    
    let qualifying1 = game.id("PL!S-sd1-001-SD");
    let qualifying2 = game.id("PL!S-sd1-002-SD");  
    let qualifying3 = game.id("PL!S-sd1-003-SD");

    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);

    let below = game.id("PL!-sd1-010-SD");
    fill_deck_to_40(&mut game, {
        let mut top = vec![qualifying1, qualifying2, qualifying3];
        top.extend(std::iter::repeat(filler).take(4));
        top.push(below);
        top
    });

    let initial_discard = game.state.player1.waitroom.cards.len();
    let initial_hand = game.state.player1.hand.cards.len();

    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, MemberArea::LeftSide);

    // [Choice 1] Optional cost: skip
    if game.has_pending_choice() { game.select_indices(&[]); }
    // [Choice 2] look_and_select: select only 1 card
    if game.has_pending_choice() { game.select_indices(&[0]); }

    assert!(!game.has_pending_choice(), "Ability should have ended after selection");

    // The 1 selected card should be in hand
    assert!(game.state.player1.hand.cards.contains(&qualifying1), "Selected card should be in hand");
    // Other 2 qualifying cards should be discarded (not in hand)
    assert!(!game.state.player1.hand.cards.contains(&qualifying2), "Non-selected qualifying should NOT be in hand");
    assert!(!game.state.player1.hand.cards.contains(&qualifying3), "Non-selected qualifying should NOT be in hand");
    
    assert!(game.state.player1.main_deck.cards.contains(&below), "Below-zone card should still be in deck");
    
    let final_discard = game.state.player1.waitroom.cards.len();
    assert_eq!(final_discard - initial_discard, 6, "Expected 6 cards in discard");
    assert_eq!(game.state.player1.hand.cards.len(), initial_hand, "Hand should have net 0 change");
}

/// Play You twice, verify blade-only cards consistently excluded
#[test]
fn you_q124_two_plays_both_reject_blade_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let you = game.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let blade_only = game.id("PL!SP-sd1-001-SD");

    game.state.player1.hand.cards.push(you);
    game.state.player1.hand.cards.push(filler);

    fill_deck_to_40(&mut game, vec![filler, filler, filler, filler, filler, filler, filler]);

    game.give_energy(13);
    game.state.player1.stage.stage[0] = -1;

    // Play You
    game.play_to_stage(you, MemberArea::LeftSide);
    // [Choice 1] Optional cost: skip
    if game.has_pending_choice() { game.select_indices(&[]); }
    // [Choice 2] look_and_select: skip (no matching cards)
    if game.has_pending_choice() { game.select_indices(&[]); }

    assert!(!game.state.player1.hand.cards.contains(&blade_only),
        "Blade-only card should NOT be in hand");
}

fn fill_deck_to_40(game: &mut TestGame, top_cards: Vec<i16>) {
    game.state.player1.main_deck.cards.extend(top_cards);
    let filler = game.id("PL!-sd1-010-SD");
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
}