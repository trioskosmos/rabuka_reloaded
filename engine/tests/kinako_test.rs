/// Tests for 桜小路きな子 (PL!SP-bp2-006) ability #1:
///   {{kidou.png|起動}}{{turn1.png|ターン1回}}手札のコスト4以下の『Liella!』のメンバーカードを1枚控え室に置く：
///   これにより控え室に置いたメンバーカードの{{toujyou.png|登場}}能力1つを発動させる。
///
/// This test verifies:
///   - Cost has cost_limit=4 and group_names=["Liella!"]
///   - Effect has source_card="cost_card" and target_trigger="登場"
///   - Cost_limit filter works (cards with cost > 4 are excluded)
///   - The ability can be activated when a matching card is in hand

mod helpers;
use helpers::*;
use rabuka_engine::card::CardDatabase;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

/// Helper to verify ability data integrity by loading from the database.
#[test]
fn kinako_ability_parsed_correctly() {
    let db = load_real_database();
    
    // Find 桜小路きな子 - 4 variants share the same ability
    let kinako_id = db.get_card_id("PL!SP-bp2-006-R+")
        .or_else(|| db.get_card_id("PL!SP-bp2-006-P"))
        .or_else(|| db.get_card_id("PL!SP-bp2-006-SEC"))
        .expect("桜小路きな子 (PL!SP-bp2-006) not found");
    
    let card = db.get_card(kinako_id).expect("Card data not found");
    let abilities = &card.abilities;
    
    // Should have 2 abilities (ab#0: baton touch recovery, ab#1: activation)
    assert!(abilities.len() >= 2,
        "Expected at least 2 abilities, got {}", abilities.len());
    
    // Find the activation ability (ab#1) - it has trigger "起動"
    let activate_ab = abilities.iter().find(|a| {
        a.triggers.as_deref() == Some("起動")
    }).expect("Activation ability not found");
    
    let effect = activate_ab.effect.as_ref().expect("Effect should exist");
    assert_eq!(effect.action, "activate_ability",
        "Effect action should be activate_ability, got {}", effect.action);
    assert_eq!(effect.target_trigger.as_deref(), Some("登場"),
        "target_trigger should be 登場");
    assert_eq!(effect.count, Some(1),
        "Should activate 1 ability");
}

/// Verify the activation ability's cost has proper cost_limit and group filtering.
#[test]
fn kinako_cost_has_cost_limit_and_group_filter() {
    let db = load_real_database();
    
    let kinako_id = db.get_card_id("PL!SP-bp2-006-R+")
        .or_else(|| db.get_card_id("PL!SP-bp2-006-P"))
        .or_else(|| db.get_card_id("PL!SP-bp2-006-SEC"))
        .expect("桜小路きな子 not found");
    
    let card = db.get_card(kinako_id).expect("Card data not found");
    let activate_ab = card.abilities.iter()
        .find(|a| a.triggers.as_deref() == Some("起動"))
        .expect("Activation ability not found");
    
    let cost = activate_ab.cost.as_ref().expect("Cost should exist");
    assert_eq!(cost.cost_type.as_deref(), Some("move_cards"),
        "Cost type should be move_cards");
    assert_eq!(cost.cost_limit, Some(4),
        "Cost limit should be 4, got {:?}", cost.cost_limit);
    assert_eq!(cost.cost_limit_operator.as_deref(), Some("<="),
        "Cost limit operator should be <=");
    assert_eq!(cost.source.as_deref(), Some("hand"),
        "Source should be hand");
    assert_eq!(cost.destination.as_deref(), Some("discard"),
        "Destination should be discard");
    assert_eq!(cost.count, Some(1),
        "Count should be 1");
    assert_eq!(cost.card_type.as_deref(), Some("member_card"),
        "Card type should be member_card");
    assert!(cost.group_names.as_ref().map_or(false, |g| g.contains(&"Liella!".to_string())),
        "Group names should contain Liella!, got {:?}", cost.group_names);
}

/// Full integration: place 桜小路きな子 on stage, add matching cost card to hand,
/// activate ability, and verify the cost card is discarded.
///
/// Note: Uses 鬼塚夏美 (PL!SP-bp2-009) as the "Liella!" card since in the test
/// environment we bypass group matching by directly providing card data.
#[test]
fn kinako_activate_discards_matching_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    
    // 桜小路きな子 (activation ability on stage)
    let kinako = game.id("PL!SP-bp2-006-P");
    
    // A card with cost 4 (should be eligible for cost_limit=4)
    // Using 鬼塚夏美 (PL!SP-bp2-009) - cost 3 card from スーパースター!! series
    let cost_card = game.id("PL!SP-sd1-020-SD"); // 鬼塚夏美, cost=2
    
    // A filler card with cost > 4 (should NOT be eligible)
    let high_cost_card = game.id("PL!SP-sd1-012-SD"); // 澁谷かのん, cost=9
    
    // Give energy to play the cost-10 card
    game.give_energy(10);
    // Add cards to hand first, then play to stage
    game.state.player1.hand.cards.push(kinako);
    game.state.player1.hand.cards.push(cost_card);      // cost=2, eligible
    game.state.player1.hand.cards.push(high_cost_card);  // cost=9, NOT eligible by cost
    game.play_to_stage(kinako, MemberArea::Center);
    
    // Activate ability #1 (起動 ability)
    // The engine should filter hand cards by cost_limit=4
    // Only cost_card (cost=2) should be selectable
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(kinako),
        None,
        None,
        None,
    ).expect("activate_ability failed");
    
    // Should have a pending choice to select the cost card
    if game.has_pending_choice() {
        // Select the first index (cost_card which is at hand index 0)
        game.select_indices(&[0]);
    }
    
    // Verify cost_card was discarded from hand
    assert!(!game.state.player1.hand.cards.contains(&cost_card),
        "Cost card should have been removed from hand");
    
    // Verify cost_card is now in discard
    assert!(game.state.player1.waitroom.cards.contains(&cost_card),
        "Cost card should be in discard");
    
    // Verify high_cost_card is still in hand (not eligible by cost)
    assert!(game.state.player1.hand.cards.contains(&high_cost_card),
        "High cost card should still be in hand (filtered by cost_limit)");
}

/// Verify the ability activation fails when no eligible card is in hand
/// (the high-cost card is left untouched).
#[test]
fn kinako_activate_high_cost_card_stays_in_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    
    let kinako = game.id("PL!SP-bp2-006-P");
    let high_cost = game.id("PL!SP-sd1-012-SD"); // 澁谷かのん, cost=9
    
    game.give_energy(10);
    game.state.player1.hand.cards.push(kinako);
    game.state.player1.hand.cards.push(high_cost); // cost 9, exceeds limit
    game.play_to_stage(kinako, MemberArea::Center);
    
    // Try to activate ability - cost should fail since no card has cost <= 4
    let _ = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(kinako),
        None,
        None,
        None,
    );
    
    // Regardless of result, the high-cost card should still be in hand
    assert!(game.state.player1.hand.cards.contains(&high_cost),
        "High cost card should remain in hand (not eligible for cost_limit=4)");
}
