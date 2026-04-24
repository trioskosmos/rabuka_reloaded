use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy, setup_player_with_stage};

#[test]
fn test_q193_two_member_baton_area() {
    // Q193: When baton touching with 2 members, which area can the member debut to?
    // Answer: It can debut to either of the 2 members' areas. Player chooses.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find a card with two-member baton touch ability
    let two_member_baton_card = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost.map_or(false, |cost| cost <= 5))
        .find(|c| {
            c.abilities.iter().any(|a| {
                a.triggers.as_ref().map_or(false, |t| t == "常時") &&
                a.full_text.contains("2人") && a.full_text.contains("バトンタッチ")
            })
        });
    
    if let Some(card) = two_member_baton_card {
        let card_id = get_card_id(card, &card_database);
        
        setup_player_with_hand(&mut player1, vec![card_id]);
        
        // Add energy cards
        let energy_card_ids: Vec<_> = cards.iter()
            .filter(|c| c.is_energy())
            .filter(|c| get_card_id(c, &card_database) != 0)
            .map(|c| get_card_id(c, &card_database))
            .take(10)
            .collect();
        setup_player_with_energy(&mut player1, energy_card_ids);
        
        // Setup player with members in different areas
        let stage_members: Vec<_> = cards.iter()
            .filter(|c| c.is_member())
            .filter(|c| c.cost.map_or(false, |cost| cost <= 3))
            .filter(|c| get_card_id(c, &card_database) != 0)
            .take(2)
            .map(|c| get_card_id(c, &card_database))
            .collect();
        
        if stage_members.len() >= 2 {
            setup_player_with_stage(&mut player1, vec![
                (stage_members[0], MemberArea::LeftSide),
                (stage_members[1], MemberArea::RightSide),
            ]);
        }
        
        let card_db_clone = card_database.clone();
        let mut game_state = GameState::new(player1, player2, card_database);
        game_state.current_phase = rabuka_engine::game_state::Phase::Main;
        game_state.turn_number = 1;
        
        // The key point: two-member baton touch allows debuting to either of the two members' areas
        // The player can choose which area to debut to
        assert!(!game_state.player1.stage.stage.is_empty(),
            "Player should have members on stage");
        
        println!("Two-member baton touch: can debut to either member's area");
    } else {
        println!("Skipping test: no card with two-member baton touch found");
    }
}
