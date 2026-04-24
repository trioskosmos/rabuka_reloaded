// Q70: エリアにメンバーカードが置かれました。同じターンに、このエリアにメンバーカードを登場させたり、何らかの効果でメンバーカードを置くことはできますか？
// Answer: いいえ、できません。エリアに置かれたターンに、そのメンバーカードがあるエリアにメンバーカードを登場させたり、何らかの効果でメンバーカードを置くことはできません。

use crate::qa_individual::common::*;

#[test]
fn test_q70_area_placement_restriction_same_turn() {
    // Test: Cannot place a member in an area that already has a member placed this turn
    // This tests area placement restrictions
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find two member cards with low cost
    let member1 = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost.is_some() && c.cost.unwrap_or(0) > 0 && c.cost.unwrap_or(0) <= 2)
        .nth(0)
        .expect("Should have member card with cost <= 2");
    let member1_id = get_card_id(member1, &card_database);
    
    let member2 = cards.iter()
        .filter(|c| c.is_member() && get_card_id(c, &card_database) != member1_id)
        .filter(|c| c.cost.is_some() && c.cost.unwrap_or(0) > 0 && c.cost.unwrap_or(0) <= 2)
        .nth(0)
        .expect("Should have member card with cost <= 2");
    let member2_id = get_card_id(member2, &card_database);
    
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(30)
        .collect();
    
    setup_player_with_hand(&mut player1, vec![member1_id, member2_id]);
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, card_database);
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    
    // Play first member to center
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &rabuka_engine::game_setup::ActionType::PlayMemberToStage,
        Some(member1_id),
        None,
        Some(rabuka_engine::zones::MemberArea::Center),
        Some(false),
    );
    assert!(result.is_ok(), "Should play member to center: {:?}", result);
    
    // Verify member is on stage
    assert!(game_state.player1.stage.stage.contains(&member1_id),
        "Member should be on stage");
    
    // Verify center area is locked for this turn
    assert!(game_state.player1.areas_locked_this_turn.contains(&rabuka_engine::zones::MemberArea::Center),
        "Center area should be locked after member placement");
    
    // Try to play second member to same area (center) - should fail
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &rabuka_engine::game_setup::ActionType::PlayMemberToStage,
        Some(member2_id),
        None,
        Some(rabuka_engine::zones::MemberArea::Center),
        Some(false),
    );
    
    // Should fail because area already has a member placed this turn
    assert!(result.is_err(), "Should fail when area already has member this turn: {:?}", result);
    
    // Verify second member is still in hand
    assert!(game_state.player1.hand.cards.contains(&member2_id),
        "Second member should remain in hand");
    
    // Verify first member is still on stage
    assert!(game_state.player1.stage.stage.contains(&member1_id),
        "First member should still be on stage");
}
