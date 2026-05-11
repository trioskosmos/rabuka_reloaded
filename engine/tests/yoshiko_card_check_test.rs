mod helpers;
use helpers::*;
use rabuka_engine::zones::MemberArea;
use rabuka_engine::turn::TurnEngine;

/// Test to verify card identities and groups
#[test]
fn test_yoshiko_card_identities() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    
    let yoshiko = game.id("PL!S-bp3-006-R＋");
    let chika = game.id("PL!S-bp2-001-R");
    let riko = game.id("PL!S-bp2-002-R");
    
    println!("Yoshiko ID: {}", yoshiko);
    println!("Chika ID: {}", chika);
    println!("Riko ID: {}", riko);
    
    // Check card details
    if let Some(yoshiko_card) = game.state.card_database.get_card(yoshiko) {
        println!("Yoshiko: {} - {} - unit: {:?}", yoshiko_card.name, yoshiko_card.card_no, yoshiko_card.unit);
    }
    
    if let Some(chika_card) = game.state.card_database.get_card(chika) {
        println!("Chika: {} - {} - unit: {:?}", chika_card.name, chika_card.card_no, chika_card.unit);
    }
    
    if let Some(riko_card) = game.state.card_database.get_card(riko) {
        println!("Riko: {} - {} - unit: {:?}", riko_card.name, riko_card.card_no, riko_card.unit);
    }
    
    // Place them on stage
    game.add_to_stage(MemberArea::Center, yoshiko);
    game.add_to_stage(MemberArea::LeftSide, chika);
    game.add_to_stage(MemberArea::RightSide, riko);
    
    println!("Stage setup: {:?}", game.player().stage.stage);
}
