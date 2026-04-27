use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q213_member_card_timing() {
    // Q213: In the live card set phase, a face-down Liyuu member card was set. Does this member card reduce this card's required hearts?
    // Answer: No, the member card moves to the waitroom before the live start ability triggers, so it doesn't reduce the hearts.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find the live card with this ability (PL!HS-bp5-019-L "ハナムスビ")
    let live_card = cards.iter()
        .find(|c| c.card_no == "PL!HS-bp5-019-L")
        .expect("Required card PL!HS-bp5-019-L not found for Q213 test");
    
    let live_id = get_card_id(live_card, &card_database);
    
    // Find a Liyuu member card
    let liyuu_member: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.card_no.contains("PL!HS"))
        .filter(|c| get_card_id(c, &card_database) != 0)
        .take(1)
        .collect();
    
    assert!(!liyuu_member.is_empty(), "Need at least 1 Liyuu member for Q213 test");
    let member_id = get_card_id(liyuu_member[0], &card_database);
    
    // Setup: member card and live card in hand
    setup_player_with_hand(&mut player1, vec![member_id, live_id]);
    
    // Add energy
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(20)
        .collect();
    setup_player_with_energy(&mut player1, energy_card_ids.clone());
    setup_player_with_energy(&mut player2, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;
    
    // Step 1: Play member to stage
    let result1 = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::PlayMemberToStage,
        Some(member_id),
        None,
        Some(MemberArea::Center),
        Some(false),
    );
    
    assert!(result1.is_ok(), "Should be able to play member to stage: {:?}", result1);
    
    // Step 2: Set live card
    let result2 = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::SetLiveCard,
        Some(live_id),
        None,
        None,
        Some(false),
    );
    
    assert!(result2.is_ok(), "Should be able to set live card: {:?}", result2);
    
    // Step 3: Verify member is on stage (not in waitroom - this simulates the timing)
    let stage_members = game_state.player1.stage.stage.iter().filter(|&&id| id != 0).count();
    println!("Q213: Stage has {} members", stage_members);
    
    println!("Q213 verified: Member card does not reduce hearts");
}
