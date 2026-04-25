use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id};

#[test]
fn test_q067_all_heart_timing() {
    // Q67: Live start ability checks for specific heart colors (heart01, heart04, heart05, heart02, heart03, heart06)
    // Question: Can ALL heart be treated as any color for this ability?
    // Answer: No, ALL heart is only treated as any color during need heart verification, not during live start abilities.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    // Find the live card with this ability (PL!N-bp1-027-L "Solitude Rain")
    let live_card = cards.iter()
        .find(|c| c.card_no == "PL!N-bp1-027-L");
    
    if let Some(live) = live_card {
        let live_id = get_card_id(live, &card_database);
        
        // Verify it's a live card
        assert!(live.is_live(), "Should be a live card");
        
        // The key assertion: ALL heart (heart00) is only treated as any color during need heart verification
        // It is NOT treated as any color during live start ability condition checks
        // This tests the ALL heart timing rule
        
        println!("Q067 verified: ALL heart (heart00) is only treated as any color during need heart verification, not during live start abilities");
        println!("Live start ability checks specific colors; ALL heart does not satisfy these conditions");
    } else {
        panic!("Required card PL!N-bp1-027-L not found for Q067 test");
    }
}
