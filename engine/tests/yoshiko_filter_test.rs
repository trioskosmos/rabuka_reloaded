mod helpers;
use helpers::*;
use rabuka_engine::zones::MemberArea;
use rabuka_engine::turn::TurnEngine;

/// Test to verify filtering logic is working correctly
#[test]
fn test_yoshiko_filter_logic() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    
    let yoshiko = game.id("PL!S-bp3-006-R＋");
    let chika = game.id("PL!S-bp2-001-R");
    let riko = game.id("PL!S-bp2-002-R");
    
    // Place them on stage
    game.add_to_stage(MemberArea::Center, yoshiko);
    game.add_to_stage(MemberArea::LeftSide, chika);
    game.add_to_stage(MemberArea::RightSide, riko);
    
    let stage_data = game.player().stage.stage.clone();
    let card_db = game.state.card_database.clone();
    
    println!("Stage: {:?}", stage_data);
    
    // Test the filter that should be used by the ability
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
    
    println!("Testing filter with exclude_self={}", yoshiko);
    
    for (i, &card_id) in stage_data.iter().enumerate() {
        if card_id != -1 {
            let matches = filter.matches(&card_db, card_id, true);
            println!("Stage[{}]: {} (id: {}) matches: {}", i, 
                card_db.get_card(card_id).map(|c| &c.name).map_or("Unknown", |v| v), 
                card_id, matches);
        }
    }
    
    // Get matching indices
    let matching_indices = rabuka_engine::ability::util::matching_indices(
        &stage_data, 
        &card_db, 
        &filter, 
        true
    );
    
    println!("Matching indices: {:?}", matching_indices);
    println!("Cards that would be moved: {:?}", matching_indices.iter().map(|&i| stage_data[i]).collect::<Vec<_>>());
}
