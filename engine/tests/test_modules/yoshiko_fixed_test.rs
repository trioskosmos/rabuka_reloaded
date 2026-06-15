use crate::helpers::*;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

/// Test Yoshiko ability with proper choice handling
#[test]
fn test_yoshiko_with_choice_handling() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let yoshiko = game.id("PL!S-bp3-006-R＋");
    let chika = game.id("PL!S-bp2-001-R");
    let riko = game.id("PL!S-bp2-002-R");
    let hand_card = game.id("PL!-sd1-010-SD");

    // Setup
    game.add_to_stage(MemberArea::Center, yoshiko);
    game.add_to_stage(MemberArea::LeftSide, chika);
    game.add_to_stage(MemberArea::RightSide, riko);
    game.add_to_hand(hand_card);
    game.give_energy(5);

    println!("=== BEFORE ACTIVATION ===");
    println!("Stage: {:?}", game.player().stage.stage);
    println!("Hand: {:?}", game.player().hand.cards);
    println!("Discard: {:?}", game.player().waitroom.cards);

    // Activate ability
    game.activate_ability(yoshiko);

    // Handle all pending choices iteratively
    while game.has_pending_choice() {
        println!("Making choice for card selection...");
        game.select_indices(&[0]);
    }

    println!("=== AFTER ACTIVATION ===");
    println!("Stage: {:?}", game.player().stage.stage);
    println!("Hand: {:?}", game.player().hand.cards);
    println!("Discard: {:?}", game.player().waitroom.cards);

    // Verify cost: Yoshiko in wait state, hand card discarded
    assert!(
        game.player().stage.stage[1] == yoshiko,
        "Yoshiko should still be on stage in wait state"
    );
    assert_eq!(
        game.player().hand.cards.len(),
        0,
        "Hand card should be discarded"
    );
    assert!(
        game.player().waitroom.cards.contains(&hand_card),
        "Hand card should be in discard"
    );

    // Effect action 1: Chika or Riko moved from stage to discard
    assert!(
        game.player().waitroom.cards.contains(&chika)
            || game.player().waitroom.cards.contains(&riko),
        "At least one Aqours member should be in discard"
    );

    println!("✅ Test passed - Yoshiko ability mechanics working correctly!");
}
