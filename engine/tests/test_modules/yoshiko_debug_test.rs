use crate::helpers::*;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

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

    // Handle all pending choices: cost (hand discard) + any effect choices
    while game.has_pending_choice() {
        println!("Resolving pending choice...");
        game.select_indices(&[0]);
    }

    println!("=== AFTER ACTIVATION ===");
    println!("Stage: {:?}", game.player().stage.stage);
    println!("Hand: {:?}", game.player().hand.cards);
    println!("Discard: {:?}", game.player().waitroom.cards);

    // Check what actually happened
    let yoshiko_still_on_stage = game.player().stage.stage[1] == yoshiko;
    let chika_still_on_stage = game.player().stage.stage[0] == chika;
    let hand_card_discarded = game.player().waitroom.cards.contains(&hand_card);

    println!("Yoshiko still on stage: {}", yoshiko_still_on_stage);
    println!("Chika still on stage: {}", chika_still_on_stage);
    println!("Hand card discarded: {}", hand_card_discarded);

    assert_eq!(
        game.player().stage.stage[1],
        yoshiko,
        "Yoshiko remains on stage after activation (wait state)"
    );
    assert!(
        game.player().waitroom.cards.contains(&hand_card),
        "Hand card was discarded to waitroom"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        0,
        "hand card was discarded as cost"
    );
    // Effect action 1: no valid Aqours stage targets (chika is SD filler, not Aqours)
    // → effect errors or skips, stage unchanged for chika
    assert_eq!(
        game.player().stage.stage[0],
        chika,
        "Chika remains on stage (not Aqours, effect has no valid targets)"
    );
}
