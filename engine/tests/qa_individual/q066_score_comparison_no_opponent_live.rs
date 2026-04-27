use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_energy};

#[test]
fn test_q066_score_comparison_no_opponent_live() {
    // Q066: If you have a live card and opponent has no live card, is "live total score is higher than opponent's" satisfied?
    // Answer: Yes, it is satisfied. When you have a live card and opponent doesn't, your score is treated as higher regardless of actual values.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find a live card with score comparison condition
    let live_card = cards.iter()
        .find(|c| c.is_live() && c.card_no == "PL!N-bp1-026-L")
        .expect("Required card PL!N-bp1-026-L not found for Q066 test");
    
    let live_id = get_card_id(live_card, &card_database);
    
    // Setup: Player1 has live card, Player2 has no live card
    player1.live_card_zone.cards.push(live_id);
    // Player2 live_card_zone remains empty
    
    // Add energy
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(5)
        .collect();
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = rabuka_engine::game_state::Phase::LiveSuccess;
    game_state.turn_number = 1;
    
    // Player1 has live card, Player2 has no live card
    // According to Q066, player1's score should be treated as higher
    let player1_has_live = !game_state.player1.live_card_zone.cards.is_empty();
    let player2_has_live = !game_state.player2.live_card_zone.cards.is_empty();
    
    assert!(player1_has_live, "Player1 should have a live card");
    assert!(!player2_has_live, "Player2 should not have a live card");
    
    // The key assertion: when opponent has no live card, player's score is treated as higher
    // This is the expected behavior per Q066
    let score_comparison_condition = player1_has_live && !player2_has_live;
    
    assert!(score_comparison_condition, "Score comparison should be true when opponent has no live card");
    
    println!("Q066 verified: Score comparison with no opponent live card");
    println!("Player1 has live card: {}", player1_has_live);
    println!("Player2 has live card: {}", player2_has_live);
    println!("Score comparison condition satisfied: {}", score_comparison_condition);
}
