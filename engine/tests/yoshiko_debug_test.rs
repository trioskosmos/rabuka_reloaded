mod helpers;
use helpers::*;
use rabuka_engine::zones::MemberArea;
use rabuka_engine::turn::TurnEngine;

/// Debug test to understand what actually happens with Yoshiko's ability
#[test]
fn test_yoshiko_debug_behavior() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    
    let yoshiko = game.id("PL!S-bp3-006-R＋");
    let chika = game.id("PL!-sd1-001-SD");
    let hand_card = game.id("PL!-sd1-010-SD");
    
    // Setup
    game.add_to_stage(MemberArea::Center, yoshiko);
    game.add_to_stage(MemberArea::LeftSide, chika);
    game.add_to_hand(hand_card);
    game.give_energy(5);
    
    println!("=== BEFORE ACTIVATION ===");
    println!("Stage: {:?}", game.player().stage.stage);
    println!("Hand: {:?}", game.player().hand.cards);
    println!("Discard: {:?}", game.player().waitroom.cards);
    
    // Activate ability
    game.activate_ability(yoshiko);
    
    let player_id = game.state.player1.id.clone();
    TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &player_id);
    game.state.process_pending_auto_abilities(&player_id);
    
    println!("=== AFTER ACTIVATION ===");
    println!("Stage: {:?}", game.player().stage.stage);
    println!("Hand: {:?}", game.player().hand.cards);
    println!("Discard: {:?}", game.player().waitroom.cards);
    
    // Check what actually happened
    let yoshiko_still_on_stage = game.player().stage.stage[1] == yoshiko;
    let chika_still_on_stage = game.player().stage.stage[0] == chika;
    let hand_card_discarded = game.player().waitroom.cards.contains(&hand_card);
    let someone_in_discard = game.player().waitroom.cards.iter().any(|&id| id == yoshiko || id == chika);
    
    println!("Yoshiko still on stage: {}", yoshiko_still_on_stage);
    println!("Chika still on stage: {}", chika_still_on_stage);
    println!("Hand card discarded: {}", hand_card_discarded);
    println!("Someone in discard: {}", someone_in_discard);
    
    assert_eq!(game.state.player1.hand.cards.len(), 0, "hand card was discarded as cost, no card drawn back");
}
