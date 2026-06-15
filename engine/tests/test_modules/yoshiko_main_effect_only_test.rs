use crate::helpers::*;

use rabuka_engine::zones::MemberArea;

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

    // Add a cost 11 Aqours member to discard for conditional effect (Chika cost 9 + 2 = 11)
    let dia = game.id("PL!S-bp2-004-R"); // Dia Kurosawa - cost 11
    game.add_to_discard(dia);

    game.give_energy(5);

    println!("=== BEFORE ACTIVATION ===");
    println!("Stage: {:?}", game.player().stage.stage);
    println!("Hand: {:?}", game.player().hand.cards);
    println!("Discard: {:?}", game.player().waitroom.cards);

    // Activate ability
    game.activate_ability(yoshiko);

    // Handle all pending choices: cost (hand discard) + effect (stage member selection)
    while game.has_pending_choice() {
        println!("Resolving pending choice...");
        game.select_indices(&[0]);
    }

    println!("=== AFTER ACTIVATION ===");
    println!("Stage: {:?}", game.player().stage.stage);
    println!("Hand: {:?}", game.player().hand.cards);
    println!("Discard: {:?}", game.player().waitroom.cards);

    // Verify cost: hand card discarded
    assert_eq!(
        game.player().hand.cards.len(),
        0,
        "Hand card should be discarded"
    );
    assert!(
        game.player().waitroom.cards.contains(&hand_card),
        "Hand card should be in discard"
    );

    // Effect action 1: Chika (Aqours, not self) moved from stage to discard
    let chika_on_stage = game.player().stage.stage.contains(&chika);
    let chika_in_discard = game.player().waitroom.cards.contains(&chika);

    println!(
        "Chika on stage: {}, Chika in discard: {}",
        chika_on_stage, chika_in_discard
    );

    assert!(
        chika_in_discard,
        "Chika should be in discard (moved by effect)"
    );
    assert!(!chika_on_stage, "Chika should no longer be on stage");
    assert!(
        game.player().stage.stage[1] == yoshiko,
        "Yoshiko should still be on stage"
    );

    // Effect action 2: conditional summon — Dia (cost 11) = Chika cost (9) + 2 = 11
    let dia_in_discard = game.player().waitroom.cards.contains(&dia);
    let dia_on_stage = game.player().stage.stage.contains(&dia);
    println!(
        "Dia in discard: {}, Dia on stage: {}",
        dia_in_discard, dia_on_stage
    );
    assert!(
        dia_on_stage,
        "Dia should be summoned to stage (cost 11 = Chika cost 9 + 2)"
    );
    assert!(!dia_in_discard, "Dia should no longer be in discard");
}
