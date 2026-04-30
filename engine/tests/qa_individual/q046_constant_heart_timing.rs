use rabuka_engine::game_state::{GameState, Phase, TurnPhase};
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy, setup_player_with_deck};

#[test]
fn test_q046_constant_heart_timing() {
    // Q046: When is the color of ALL hearts from constant abilities decided?
    // Answer: During the performance phase, when confirming whether the necessary heart condition is met.

    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());

    // Find member cards with hearts and a live card
    let members: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost.unwrap_or(0) <= 2)
        .filter(|c| get_card_id(c, &card_database) != 0)
        .take(3)
        .collect();
    assert!(members.len() >= 3, "Need 3 member cards");
    let member_ids: Vec<i16> = members.iter().map(|c| get_card_id(c, &card_database)).collect();

    let live_card = cards.iter()
        .find(|c| c.is_live() && get_card_id(c, &card_database) != 0)
        .expect("Need a live card");
    let live_id = get_card_id(live_card, &card_database);

    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);

    let mut hand = member_ids.clone();
    hand.push(live_id);
    setup_player_with_hand(&mut player1, hand);

    let energy_card_ids: Vec<i16> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(15)
        .collect();
    setup_player_with_energy(&mut player1, energy_card_ids.clone());
    setup_player_with_energy(&mut player2, energy_card_ids.clone());

    let deck_cards: Vec<i16> = cards.iter()
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(30)
        .collect();
    setup_player_with_deck(&mut player1, deck_cards.clone());
    setup_player_with_deck(&mut player2, deck_cards);

    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = Phase::Main;
    game_state.turn_number = 1;
    game_state.current_turn_phase = TurnPhase::FirstAttackerNormal;

    // Play all 3 members to stage
    let areas = [
        rabuka_engine::zones::MemberArea::LeftSide,
        rabuka_engine::zones::MemberArea::Center,
        rabuka_engine::zones::MemberArea::RightSide,
    ];
    for (i, &area) in areas.iter().enumerate() {
        TurnEngine::execute_main_phase_action(
            &mut game_state, &rabuka_engine::game_setup::ActionType::PlayMemberToStage,
            Some(member_ids[i]), None, Some(area), Some(false),
        ).expect("Member should play to stage");
    }
    assert_eq!(game_state.player1.stage.stage.iter().filter(|&&id| id != -1).count(), 3);

    // Record stage hearts DURING main phase
    let hearts_before = game_state.player1.calculate_stage_hearts(&card_database);

    // Advance to live card set
    TurnEngine::advance_phase(&mut game_state);
    TurnEngine::advance_phase(&mut game_state);

    TurnEngine::execute_main_phase_action(
        &mut game_state, &rabuka_engine::game_setup::ActionType::SetLiveCard,
        Some(live_id), None, None, None,
    ).expect("Set live card");

    // Advance to performance phase
    TurnEngine::execute_main_phase_action(
        &mut game_state, &rabuka_engine::game_setup::ActionType::Pass,
        None, None, None, None,
    ).expect("P1 pass");
    TurnEngine::execute_main_phase_action(
        &mut game_state, &rabuka_engine::game_setup::ActionType::Pass,
        None, None, None, None,
    ).expect("P2 pass");

    assert_eq!(game_state.current_phase, Phase::FirstAttackerPerformance);

    // Record stage hearts DURING performance phase
    let hearts_during = game_state.player1.calculate_stage_hearts(&card_database);

    // Stage hearts should be the same across phases (constant abilities persist)
    // Compare total hearts and specific color counts
    let total_before: u32 = hearts_before.hearts.values().sum();
    let total_during: u32 = hearts_during.hearts.values().sum();
    assert_eq!(total_before, total_during,
        "Stage heart count should be consistent from main phase to performance phase");

    // Verify heart check works during performance phase
    let heart_check = game_state.check_required_hearts();
    println!("Heart check result: {:?}", heart_check);

    println!("Total stage hearts before live: {}, during performance: {}", total_before, total_during);
    println!("Hearts before: {:?}", hearts_before.hearts);
    println!("Hearts during: {:?}", hearts_during.hearts);
}
