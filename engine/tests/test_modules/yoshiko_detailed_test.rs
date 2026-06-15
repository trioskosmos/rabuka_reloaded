/// Detailed debug test for Yoshiko ability execution flow
/// This test helps verify the complete ability execution including choice handling
use crate::helpers::*;

use rabuka_engine::zones::MemberArea;

#[test]
fn test_yoshiko_detailed_debug() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // Setup: Get Yoshiko and other Aqours members
    let yoshiko = game.id("PL!S-bp3-006-R＋"); // Yoshiko
    let chika = game.id("PL!S-bp2-001-R"); // Aqours Chika
    let riko = game.id("PL!S-bp2-002-R"); // Aqours Riko
    let hand_card = game.id("PL!-sd1-010-SD");

    // Setup stage with Yoshiko in center and other Aqours members
    game.add_to_stage(MemberArea::Center, yoshiko);
    game.add_to_stage(MemberArea::LeftSide, chika);
    game.add_to_stage(MemberArea::RightSide, riko);
    game.add_to_hand(hand_card);

    // Add a valid target for conditional effect (need cost = moved_card.cost + 2)
    let dia = game.id("PL!S-bp2-004-R"); // Dia Kurosawa (cost 11 = Chika cost 9 + 2)
    game.add_to_discard(dia);

    game.give_energy(5);

    println!("=== BEFORE ACTIVATION ===");
    println!("Stage: {:?}", game.player().stage.stage);
    println!("Hand: {:?}", game.player().hand.cards);
    println!("Discard: {:?}", game.player().waitroom.cards);

    // Activate ability
    game.activate_ability(yoshiko);

    // Handle all pending choices iteratively
    while game.has_pending_choice() {
        println!("Resolving pending choice...");
        game.select_indices(&[0]);
    }

    println!("=== AFTER ACTIVATION ===");
    println!("Stage: {:?}", game.player().stage.stage);
    println!("Hand: {:?}", game.player().hand.cards);
    println!("Discard: {:?}", game.player().waitroom.cards);

    // Check which cards are where
    let chika_on_stage = game.player().stage.stage.contains(&chika);
    let riko_on_stage = game.player().stage.stage.contains(&riko);
    let chika_in_discard = game.player().waitroom.cards.contains(&chika);
    let riko_in_discard = game.player().waitroom.cards.contains(&riko);

    println!(
        "Chika on stage: {}, Chika in discard: {}",
        chika_on_stage, chika_in_discard
    );
    println!(
        "Riko on stage: {}, Riko in discard: {}",
        riko_on_stage, riko_in_discard
    );

    // Verify cost: hand card discarded, Yoshiko in wait state
    assert!(
        game.player().waitroom.cards.contains(&hand_card),
        "Hand card should be in discard (cost paid)"
    );
    assert_eq!(
        game.player().hand.cards.len(),
        0,
        "Hand card should be discarded"
    );

    // Effect action 1: one Aqours member moved from stage to discard (2 candidates → choice selects index 0)
    assert!(
        chika_in_discard || riko_in_discard,
        "At least one Aqours member should be moved to discard"
    );
    assert!(
        !chika_on_stage || !riko_on_stage,
        "At least one Aqours member should be removed from stage"
    );

    // Effect action 2: conditional summon — Dia (cost 11) = Chika cost (9) + 2 = 11
    let dia_on_stage = game.player().stage.stage.contains(&dia);
    assert!(
        dia_on_stage,
        "Dia should be summoned to stage (conditional effect)"
    );

    println!("✅ Test passed - Yoshiko ability mechanics working correctly!");
}
