/// Test for 東條 希 (PL!-sd1-007-SD) ability fix
/// 
/// Ability: {{toujyou.png|登場}}自分のデッキの上からカードを5枚控え室に置く。それらの中にライブカードがある場合、カードを1枚引く。
/// Action: Move 5 cards from deck to discard, if any are live cards, draw 1 card
/// 
/// This test verifies the fix for card_count_condition checking discard pile for live cards
/// instead of incorrectly checking live_card_zone

mod helpers;
use helpers::*;
use rabuka_engine::zones::MemberArea;
use rabuka_engine::turn::TurnEngine;

/// Test case 1: Live card among 5 drawn cards should trigger draw
#[test]
fn test_live_card_among_five_drawn_triggers_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    
    // Setup: Get the test card and some known cards
    let nagi_card = game.id("PL!-sd1-007-SD"); // 東條 希
    let live_card = game.id("PL!-sd1-019-SD"); // Live card: START:DASH!!
    let member_card1 = game.id("PL!-sd1-010-SD"); // Member card
    let member_card2 = game.id("PL!-sd1-011-SD"); // Member card
    let member_card3 = game.id("PL!-sd1-012-SD"); // Member card
    let member_card4 = game.id("PL!-sd1-013-SD"); // Member card
    
    // Give player enough energy to play the card (automatically activated)
    game.give_energy(7);
    
    // Setup deck with known order: live card + 4 member cards on top
    game.player().main_deck.cards.clear();
    game.player().main_deck.cards.push(live_card);    // Position 0 (top)
    game.player().main_deck.cards.push(member_card1);  // Position 1
    game.player().main_deck.cards.push(member_card2);  // Position 2
    game.player().main_deck.cards.push(member_card3);  // Position 3
    game.player().main_deck.cards.push(member_card4);  // Position 4
    
    // Add more cards to deck so we don't run out
    let filler_card = game.id("PL!-sd1-014-SD");
    for _i in 0..10 {
        game.player().main_deck.cards.push(filler_card);
    }
    
    let initial_hand_size = game.player().hand.cards.len();
    
    // Play the card to stage and activate its debut ability
    game.add_to_hand(nagi_card);
    game.play_to_stage(nagi_card, MemberArea::Center);
    game.activate_ability(nagi_card);
    
    // Manually trigger auto abilities for the player
    let player_id = game.state.player1.id.clone();
    TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &player_id);
    game.state.process_pending_auto_abilities(&player_id);
    
    // Verify: 5 cards moved to discard, and since one was a live card, should draw 1
    let final_hand_size = game.player().hand.cards.len();
    let discard_size = game.player().waitroom.cards.len();
    
    assert_eq!(discard_size, 5, "Should have moved exactly 5 cards to discard");
    assert_eq!(final_hand_size, initial_hand_size + 1, "Should have drawn 1 card because live card was in discard");
    
    // Verify the live card is actually in discard
    assert!(game.player().waitroom.cards.contains(&live_card), "Live card should be in discard");
}

/// Test case 2: No live cards among 5 drawn cards should not trigger draw
#[test]
fn test_no_live_cards_among_five_drawn_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    
    // Setup: Get the test card and only member cards
    let nagi_card = game.id("PL!-sd1-007-SD"); // 東條 希
    let member_card1 = game.id("PL!-sd1-010-SD"); // Member card
    let member_card2 = game.id("PL!-sd1-011-SD"); // Member card
    let member_card3 = game.id("PL!-sd1-012-SD"); // Member card
    let member_card4 = game.id("PL!-sd1-013-SD"); // Member card
    let member_card5 = game.id("PL!-sd1-014-SD"); // Member card
    
    // Give player enough energy to play the card (automatically activated)
    game.give_energy(7);
    
    // Setup deck with only member cards on top
    game.player().main_deck.cards.clear();
    game.player().main_deck.cards.push(member_card1);  // Position 0 (top)
    game.player().main_deck.cards.push(member_card2);  // Position 1
    game.player().main_deck.cards.push(member_card3);  // Position 2
    game.player().main_deck.cards.push(member_card4);  // Position 3
    game.player().main_deck.cards.push(member_card5);  // Position 4
    
    // Add more cards to deck
    let filler_card = game.id("PL!-sd1-015-SD");
    for _i in 0..10 {
        game.player().main_deck.cards.push(filler_card);
    }
    
    let initial_hand_size = game.player().hand.cards.len();
    
    // Play the card to stage and activate its debut ability
    game.add_to_hand(nagi_card);
    game.play_to_stage(nagi_card, MemberArea::Center);
    game.activate_ability(nagi_card);
    
    // Manually trigger auto abilities for the player
    let player_id = game.state.player1.id.clone();
    TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &player_id);
    game.state.process_pending_auto_abilities(&player_id);
    
    // Verify: 5 cards moved to discard, but no draw since no live cards
    let final_hand_size = game.player().hand.cards.len();
    let discard_size = game.player().waitroom.cards.len();
    
    assert_eq!(discard_size, 5, "Should have moved exactly 5 cards to discard");
    assert_eq!(final_hand_size, initial_hand_size, "Should NOT have drawn any card since no live cards were in discard");
}

/// Test case 3: Live card already in discard but none in 5 drawn should not trigger draw
/// This ensures the condition only checks the 5 newly drawn cards, not existing discard
#[test]
fn test_live_card_in_existing_discard_but_not_in_five_drawn_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    
    // Setup: Get the test card and some known cards
    let nagi_card = game.id("PL!-sd1-007-SD"); // 東條 希
    let live_card = game.id("PL!-sd1-019-SD"); // Live card: START:DASH!!
    let member_card1 = game.id("PL!-sd1-010-SD"); // Member card
    let member_card2 = game.id("PL!-sd1-011-SD"); // Member card
    let member_card3 = game.id("PL!-sd1-012-SD"); // Member card
    let member_card4 = game.id("PL!-sd1-013-SD"); // Member card
    let member_card5 = game.id("PL!-sd1-014-SD"); // Member card
    
    // Give player enough energy to play the card (automatically activated)
    game.give_energy(7);
    
    // Pre-setup: Put a live card in discard BEFORE the ability
    game.add_to_discard(live_card);
    
    // Setup deck with only member cards on top (no live cards)
    game.player().main_deck.cards.clear();
    game.player().main_deck.cards.push(member_card1);  // Position 0 (top)
    game.player().main_deck.cards.push(member_card2);  // Position 1
    game.player().main_deck.cards.push(member_card3);  // Position 2
    game.player().main_deck.cards.push(member_card4);  // Position 3
    game.player().main_deck.cards.push(member_card5);  // Position 4
    
    // Add more cards to deck
    let filler_card = game.id("PL!-sd1-016-SD");
    for _i in 0..10 {
        game.player().main_deck.cards.push(filler_card);
    }
    
    let initial_hand_size = game.player().hand.cards.len();
    let initial_discard_size = game.player().waitroom.cards.len();
    
    // Play the card to stage and activate its debut ability
    game.add_to_hand(nagi_card);
    game.play_to_stage(nagi_card, MemberArea::Center);
    game.activate_ability(nagi_card);
    
    // Manually trigger auto abilities for the player
    let player_id = game.state.player1.id.clone();
    TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &player_id);
    game.state.process_pending_auto_abilities(&player_id);
    
    // Verify: 5 cards moved to discard, but no draw since none of the 5 were live cards
    let final_hand_size = game.player().hand.cards.len();
    let final_discard_size = game.player().waitroom.cards.len();
    
    assert_eq!(final_discard_size, initial_discard_size + 5, "Should have moved exactly 5 cards to discard");
    assert_eq!(final_hand_size, initial_hand_size, "Should NOT have drawn any card since none of the 5 drawn cards were live cards");
    
    // Verify the existing live card is still in discard
    assert!(game.player().waitroom.cards.contains(&live_card), "Existing live card should still be in discard");
}

/// Test case 4: Multiple live cards among 5 drawn should still only draw 1 card
#[test]
fn test_multiple_live_cards_among_five_drawn_still_only_draw_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    
    // Setup: Get the test card and some known cards
    let nagi_card = game.id("PL!-sd1-007-SD"); // 東條 希
    let live_card1 = game.id("PL!-sd1-019-SD"); // Live card 1: START:DASH!!
    let live_card2 = game.id("PL!-sd1-020-SD"); // Live card 2: きっと青春が聞こえる
    let member_card1 = game.id("PL!-sd1-010-SD"); // Member card
    let member_card2 = game.id("PL!-sd1-011-SD"); // Member card
    let member_card3 = game.id("PL!-sd1-012-SD"); // Member card
    
    // Give player enough energy to play the card (automatically activated)
    game.give_energy(7);
    
    // Setup deck with 2 live cards + 3 member cards on top
    game.player().main_deck.cards.clear();
    game.player().main_deck.cards.push(live_card1);   // Position 0 (top)
    game.player().main_deck.cards.push(live_card2);   // Position 1
    game.player().main_deck.cards.push(member_card1);  // Position 2
    game.player().main_deck.cards.push(member_card2);  // Position 3
    game.player().main_deck.cards.push(member_card3);  // Position 4
    
    // Add more cards to deck
    let filler_card = game.id("PL!-sd1-013-SD");
    for _i in 0..10 {
        game.player().main_deck.cards.push(filler_card);
    }
    
    let initial_hand_size = game.player().hand.cards.len();
    
    // Play the card to stage and activate its debut ability
    game.add_to_hand(nagi_card);
    game.play_to_stage(nagi_card, MemberArea::Center);
    game.activate_ability(nagi_card);
    
    // Manually trigger auto abilities for the player
    let player_id = game.state.player1.id.clone();
    TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &player_id);
    game.state.process_pending_auto_abilities(&player_id);
    
    // Verify: 5 cards moved to discard, and should draw exactly 1 card (not 2)
    let final_hand_size = game.player().hand.cards.len();
    let discard_size = game.player().waitroom.cards.len();
    
    assert_eq!(discard_size, 5, "Should have moved exactly 5 cards to discard");
    assert_eq!(final_hand_size, initial_hand_size + 1, "Should have drawn exactly 1 card even with multiple live cards");
    
    // Verify both live cards are in discard
    assert!(game.player().waitroom.cards.contains(&live_card1), "First live card should be in discard");
    assert!(game.player().waitroom.cards.contains(&live_card2), "Second live card should be in discard");
}
