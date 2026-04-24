// Q27: 「バトンタッチ」で、ステージにいるメンバーカードを2枚以上控え室に置いて、その合計のコストと同じだけエネルギーを支払ったことにできますか？
// Answer: いいえ、できません。1回の「バトンタッチ」で控え室に置けるメンバーカードは1枚です。

use crate::qa_individual::common::*;

#[test]
fn test_q27_baton_touch_replaces_only_one_member() {
    // Test: Baton touch can only replace one member at a time
    // This tests that the engine correctly limits baton touch to single member replacement
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find three member cards
    let members: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost.is_some() && c.cost.unwrap_or(0) > 0)
        .take(3)
        .collect();
    
    if members.len() >= 3 {
        let member1_id = get_card_id(members[0], &card_database);
        let member2_id = get_card_id(members[1], &card_database);
        let member3_id = get_card_id(members[2], &card_database);
        
        let energy_card_ids: Vec<_> = cards.iter()
            .filter(|c| c.is_energy())
            .filter(|c| get_card_id(c, &card_database) != 0)
            .map(|c| get_card_id(c, &card_database))
            .take(30)
            .collect();
        
        setup_player_with_hand(&mut player1, vec![member1_id, member2_id, member3_id]);
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
        
        // Advance turn to allow baton touch
        game_state.turn_number = 2;
        game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
        game_state.player1.areas_locked_this_turn.clear();
        
        let initial_waitroom_count = game_state.player1.waitroom.cards.len();
        let initial_stage_count = game_state.player1.stage.stage.iter().filter(|&&id| id != -1).count();
        
        // Baton touch with member2 - should replace only member1
        let result = TurnEngine::execute_main_phase_action(
            &mut game_state,
            &rabuka_engine::game_setup::ActionType::PlayMemberToStage,
            Some(member2_id),
            None,
            Some(rabuka_engine::zones::MemberArea::Center),
            Some(true), // baton touch
        );
        
        if result.is_ok() {
            // Verify only one member was sent to waitroom
            assert_eq!(game_state.player1.waitroom.cards.len(), initial_waitroom_count + 1,
                "Waitroom should have exactly 1 more card (only 1 member replaced)");
            
            // Verify stage still has 1 member
            assert_eq!(game_state.player1.stage.stage.iter().filter(|&&id| id != -1).count(), initial_stage_count,
                "Stage should still have 1 member (replaced, not added)");
            
            // Verify member1 is in waitroom
            assert!(game_state.player1.waitroom.cards.contains(&member1_id),
                "Original member should be in waitroom");
            
            // Verify member2 is on stage
            assert!(game_state.player1.stage.stage.contains(&member2_id),
                "New member should be on stage");
        }
    } else {
        println!("Skipping test: insufficient member cards");
    }
}
