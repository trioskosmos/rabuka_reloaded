mod helpers;
use helpers::*;
use rabuka_engine::zones::MemberArea;
use rabuka_engine::turn::TurnEngine;

/// Test to verify group matching is working
#[test]
fn test_yoshiko_group_matching() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    
    let yoshiko = game.id("PL!S-bp3-006-R＋");
    let chika = game.id("PL!S-bp2-001-R");
    let riko = game.id("PL!S-bp2-002-R");
    
    // Test group matching directly
    println!("Testing group matching for Aqours:");
    
    for (name, card_id) in [("Yoshiko", yoshiko), ("Chika", chika), ("Riko", riko)] {
        if let Some(card) = game.state.card_database.get_card(card_id) {
            let matches_unit = card.unit.as_deref() == Some("Aqours");
            let matches_group = card.group == "Aqours";
            let matches_series = card.series.contains("サンシャイン");
            
            println!("{}: unit={:?}, group={}, series={}", name, card.unit, matches_group, matches_series);
            
            // Test the actual function used by abilities
            let matches = rabuka_engine::ability::util::card_matches_group_str(&game.state.card_database, card_id, Some("Aqours"));
            println!("{} matches 'Aqours': {}", name, matches);
        }
    }
}
