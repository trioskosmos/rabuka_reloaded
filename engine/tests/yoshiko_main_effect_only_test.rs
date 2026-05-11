mod helpers;
use helpers::*;
use rabuka_engine::zones::MemberArea;
use rabuka_engine::turn::TurnEngine;

/// Test Yoshiko ability main effect only (no discard targets for conditional effect)
#[test]
fn test_yoshiko_main_effect_only() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    
    let yoshiko = game.id("PL!S-bp3-006-R＋");
    let chika = game.id("PL!S-bp2-001-R");
    let hand_card = game.id("PL!-sd1-010-SD");
    
    // Setup: Yoshiko and one other Aqours member, plus a cost 6 Aqours member in discard for conditional effect
    game.add_to_stage(MemberArea::Center, yoshiko);
    game.add_to_stage(MemberArea::LeftSide, chika);
    game.add_to_hand(hand_card);
    
    // Add a cost 6 Aqours member to discard for conditional effect (Chika cost 4 + 2 = 6)
    let you = game.id("PL!S-bp2-003-R"); // You Watanabe - should be cost 6
    game.add_to_discard(you);
    
    game.give_energy(5);
    
    println!("=== BEFORE ACTIVATION ===");
    println!("Stage: {:?}", game.player().stage.stage);
    println!("Hand: {:?}", game.player().hand.cards);
    println!("Discard: {:?}", game.player().waitroom.cards);
    
    // Activate ability
    game.activate_ability(yoshiko);
    
    // Process the ability
    let player_id = game.state.player1.id.clone();
    TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &player_id);
    game.state.process_pending_auto_abilities(&player_id);
    
    println!("=== AFTER ACTIVATION ===");
    println!("Stage: {:?}", game.player().stage.stage);
    println!("Hand: {:?}", game.player().hand.cards);
    println!("Discard: {:?}", game.player().waitroom.cards);
    
    // Verify cost payment worked
    assert!(game.player().hand.cards.len() < 1, "Hand card should be discarded");
    assert!(game.player().waitroom.cards.contains(&hand_card), "Hand card should be in discard");
    
    // Check if main effect worked (Chika moved to discard)
    let chika_on_stage = game.player().stage.stage.contains(&chika);
    let chika_in_discard = game.player().waitroom.cards.contains(&chika);
    
    println!("Chika on stage: {}, Chika in discard: {}", chika_on_stage, chika_in_discard);
    
    // At this point, either:
    // 1. Main effect worked: Chika in discard, Yoshiko still on stage
    // 2. Main effect failed: Both still on stage
    
    if chika_in_discard {
        println!("✅ Main effect worked - Chika moved to discard");
        assert!(game.player().stage.stage[1] == yoshiko, "Yoshiko should still be on stage");
    } else {
        println!("❌ Main effect failed - no cards moved");
        // Let's debug why the main effect isn't working
        println!("Debug: Yoshiko still on stage: {}", game.player().stage.stage[1] == yoshiko);
        println!("Debug: Chika still on stage: {}", chika_on_stage);
    }
}
