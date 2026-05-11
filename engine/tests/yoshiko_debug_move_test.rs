mod helpers;
use helpers::*;
use rabuka_engine::zones::MemberArea;
use rabuka_engine::turn::TurnEngine;

/// Debug test to trace move_cards execution
#[test]
fn test_yoshiko_debug_move_execution() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    
    let yoshiko = game.id("PL!S-bp3-006-R＋");
    let chika = game.id("PL!S-bp2-001-R");
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
    
    // Manually test the move_cards logic to see what's happening
    println!("\n=== TESTING FILTER LOGIC ===");
    
    // Test the filter that should be used
    let filter = rabuka_engine::ability::util::filter_from_parts_full(
        Some("member_card"),
        Some("Aqours"),
        None,
        None,
        None,
        None,
        None,
        Some(yoshiko) // exclude_self
    );
    
    let stage_data = game.player().stage.stage.clone();
    let card_db = game.state.card_database.clone();
    
    // Get matching indices
    let matching_indices = rabuka_engine::ability::util::matching_indices(
        &stage_data, 
        &card_db, 
        &filter, 
        true
    );
    
    println!("Matching indices: {:?}", matching_indices);
    println!("Cards at those indices: {:?}", matching_indices.iter().map(|&i| stage_data[i]).collect::<Vec<_>>());
    
    // For single target, classify_selection should return Exact
    println!("For single target case, classify_selection should return Exact with indices: {:?}", matching_indices);
    
    // Activate ability to see actual execution
    println!("\n=== ACTUAL ABILITY EXECUTION ===");
    game.activate_ability(yoshiko);
    
    let player_id = game.state.player1.id.clone();
    TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &player_id);
    game.state.process_pending_auto_abilities(&player_id);
    
    println!("=== AFTER ACTIVATION ===");
    println!("Stage: {:?}", game.player().stage.stage);
    println!("Hand: {:?}", game.player().hand.cards);
    println!("Discard: {:?}", game.player().waitroom.cards);
}
