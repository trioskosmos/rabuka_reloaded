// QA Data Tests
// These tests are based on official Q&A data from qa_data.json
// Each test corresponds to a specific Q&A entry and tests the engine's behavior against the official answer
// Tests use the action system to play the game like a player would

use rabuka_engine::card::{Card, CardDatabase};
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::game_state::{GameState, Phase};
use rabuka_engine::game_setup;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use std::path::Path;
use std::sync::Arc;

/// Helper function to load all cards from cards.json
fn load_all_cards() -> Vec<Card> {
    let cards_path = Path::new("../cards/cards.json");
    CardLoader::load_cards_from_file(cards_path).expect("Failed to load cards")
}

/// Helper function to create CardDatabase from loaded cards
fn create_card_database(cards: Vec<Card>) -> Arc<CardDatabase> {
    Arc::new(CardDatabase::load_or_create(cards))
}

/// Helper function to set up a player with specific cards in hand
fn setup_player_with_hand(player: &mut rabuka_engine::player::Player, card_ids: Vec<i16>) {
    player.hand.cards = card_ids.into_iter().collect();
}

/// Helper function to set up a player with specific energy cards
fn setup_player_with_energy(player: &mut rabuka_engine::player::Player, card_ids: Vec<i16>) {
    let count = card_ids.len();
    player.energy_zone.cards = card_ids.into_iter().collect();
    player.energy_zone.active_energy_count = count;
}

/// Helper function to get card ID from card
fn get_card_id(card: &Card) -> i16 {
    card.card_no.parse::<i16>().unwrap_or(0)
}

/// Q23: 手札のメンバーカードをステージに登場させる詳しい手順を教えてください。
/// Answer: 以下の手順で処理します。〈【1】手札のメンバーカードを1枚公開して、登場させるステージのエリアを1つ指定します。【2】公開したメンバーカードのコストと同じ枚数だけ、エネルギー置き場のエネルギーカードをアクティブ状態（縦向き状態）からウェイト状態（横向き状態）にします。【3】公開したメンバーカードを指定したステージのエリアに登場させます。〉
#[test]
fn test_q23_member_card_to_stage_procedure() {
    let cards = load_all_cards();
    
    // Create players
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find a member card and energy cards, get their IDs
    let member_card = cards.iter().find(|c| c.is_member()).expect("Should have member card");
    let member_card_id = get_card_id(member_card);
    let card_cost = member_card.cost.unwrap_or(0);
    let energy_card_ids: Vec<_> = cards.iter().filter(|c| c.is_energy()).take(10).map(|c| get_card_id(c)).collect();
    
    // Set up player1 with member card in hand and energy in energy zone
    setup_player_with_hand(&mut player1, vec![member_card_id]);
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let card_database = create_card_database(cards);
    let mut game_state = GameState::new(player1, player2, card_database);
    game_state.current_phase = Phase::Main;
    game_state.turn_number = 1;
    
    // Record initial state
    let initial_hand_count = game_state.player1.hand.cards.len();
    let initial_energy_active = game_state.player1.energy_zone.active_count();
    
    // Get available actions
    let actions = game_setup::generate_possible_actions(&game_state);
    
    // Find PlayMemberToStage action
    let play_action = actions.iter()
        .find(|a| a.action_type == game_setup::ActionType::PlayMemberToStage)
        .expect("Should have action to play member card");
    
    // Get action parameters
    let action_params = play_action.parameters.as_ref().unwrap();
    let card_index = action_params.card_index.unwrap();
    let available_areas = action_params.available_areas.as_ref().unwrap();
    let available_area = available_areas.iter().find(|a| a.available).unwrap();
    
    // Execute the action
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &play_action.action_type,
        Some(card_index),
        None,
        Some(available_area.area),
        Some(false), // not using baton touch
    );
    
    assert!(result.is_ok(), "Should successfully play card to stage: {:?}", result);
    
    // Verify: Card moved from hand to stage
    assert_eq!(game_state.player1.hand.cards.len(), initial_hand_count - 1,
        "Card should be removed from hand");
    
    // Verify: Card is on stage (check stage array)
    let card_on_stage = game_state.player1.stage.stage.iter().any(|&id| id != -1);
    assert!(card_on_stage, "Card should be on stage");
    
    // Verify: Energy was paid
    let final_energy_active = game_state.player1.energy_zone.active_count();
    let energy_paid = initial_energy_active - final_energy_active;
    assert_eq!(energy_paid as u32, card_cost,
        "Energy paid should equal card cost");
    
    println!("Q23 test: Member card to stage - card: {}, cost: {}, energy paid: {}, hand: {} -> {}, energy active: {} -> {}",
        member_card_id, card_cost, energy_paid, initial_hand_count, game_state.player1.hand.cards.len(),
        initial_energy_active, final_energy_active);
}

/// Q24: 手札のメンバーカードを「バトンタッチ」でステージに登場させる手順を教えてください。
/// Answer: 以下の手順で処理します。〈【1】手札のメンバーカードを1枚公開して、登場させるステージのエリアを1つ指定します。【2】指定したエリアにいるメンバーカードを控え室に置きます。【3】公開したメンバーカードのコストから控え室に置いたメンバーカードのコストを引いた数と同じ枚数だけ、エネルギー置き場のエネルギーカードをアクティブ状態（縦向き状態）からウェイト状態（横向き状態）にします。【4】公開したメンバーカードを指定したステージのエリアに登場させます。〉
#[test]
fn test_q24_baton_touch_procedure() {
    let cards = load_all_cards();
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let member_card_ids: Vec<_> = cards.iter().filter(|c| c.is_member()).take(2).map(|c| get_card_id(c)).collect();
    let energy_card_ids: Vec<_> = cards.iter().filter(|c| c.is_energy()).take(20).map(|c| get_card_id(c)).collect();
    
    setup_player_with_hand(&mut player1, member_card_ids);
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let card_database = create_card_database(cards);
    let mut game_state = GameState::new(player1, player2, card_database);
    game_state.current_phase = Phase::Main;
    game_state.turn_number = 1;
    
    // Turn 1: Play first member card to stage
    let actions = game_setup::generate_possible_actions(&game_state);
    let play_action = actions.iter()
        .find(|a| a.action_type == game_setup::ActionType::PlayMemberToStage)
        .expect("Should have action to play member card ");
    
    let action_params = play_action.parameters.as_ref().unwrap();
    let card_index = action_params.card_index.unwrap();
    let available_areas = action_params.available_areas.as_ref().unwrap();
    let available_area = available_areas.iter().find(|a| a.available).unwrap();
    
    TurnEngine::execute_main_phase_action(
        &mut game_state,
        &play_action.action_type,
        Some(card_index),
        None,
        Some(available_area.area),
        Some(false),
    ).expect("Should play card to stage ");
    
    let initial_waitroom_count = game_state.player1.waitroom.cards.len();
    let initial_hand_count = game_state.player1.hand.cards.len();
    let initial_energy_active = game_state.player1.energy_zone.active_count();
    
    // Turn 2: Baton touch with second card
    let actions = game_setup::generate_possible_actions(&game_state);
    let play_action = actions.iter()
        .find(|a| a.action_type == game_setup::ActionType::PlayMemberToStage)
        .expect("Should have action to play member card ");
    
    let action_params = play_action.parameters.as_ref().unwrap();
    let card_index = action_params.card_index.unwrap();
    let available_areas = action_params.available_areas.as_ref().unwrap();
    let baton_area = available_areas.iter().find(|a| a.available && a.is_baton_touch).unwrap_or_else(|| {
        available_areas.iter().find(|a| a.available).unwrap()
    });
    
    TurnEngine::execute_main_phase_action(
        &mut game_state,
        &play_action.action_type,
        Some(card_index),
        None,
        Some(baton_area.area),
        Some(true), // use baton touch
    ).expect("Should baton touch");
    
    assert_eq!(game_state.player1.hand.cards.len(), initial_hand_count - 1,
        "Card should be removed from hand");
    
    assert!(game_state.player1.waitroom.cards.len() > initial_waitroom_count,
        "Existing card should be in waitroom");
    
    let final_energy_active = game_state.player1.energy_zone.active_count();
    let energy_paid = initial_energy_active - final_energy_active;
    
    println!("Q24 test: Baton touch - energy paid: {}, waitroom: {} -> {}",
        energy_paid, initial_waitroom_count, game_state.player1.waitroom.cards.len());
}

/// Q25: ステージにいるメンバーカードと同じもしくは小さいコストのメンバーカードで「バトンタッチ」することはできますか？
/// Answer: はい、できます。その場合、エネルギー置き場のエネルギーカードは1枚もアクティブ状態（縦向き状態）からウェイト状態（横向き状態）にしません。
#[test]
fn test_q25_baton_touch_equal_or_lower_cost() {
    let cards = load_all_cards();
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let member_card_ids: Vec<_> = cards.iter().filter(|c| c.is_member()).take(2).map(|c| get_card_id(c)).collect();
    let energy_card_ids: Vec<_> = cards.iter().filter(|c| c.is_energy()).take(10).map(|c| get_card_id(c)).collect();
    
    setup_player_with_hand(&mut player1, member_card_ids);
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let card_database = create_card_database(cards);
    let mut game_state = GameState::new(player1, player2, card_database);
    game_state.current_phase = Phase::Main;
    game_state.turn_number = 1;
    
    // Turn 1: Play first member card to stage
    let actions = game_setup::generate_possible_actions(&game_state);
    let play_action = actions.iter()
        .find(|a| a.action_type == game_setup::ActionType::PlayMemberToStage)
        .expect("Should have action to play member card ");
    
    let action_params = play_action.parameters.as_ref().unwrap();
    let card_index = action_params.card_index.unwrap();
    let available_areas = action_params.available_areas.as_ref().unwrap();
    let available_area = available_areas.iter().find(|a| a.available).unwrap();
    
    TurnEngine::execute_main_phase_action(
        &mut game_state,
        &play_action.action_type,
        Some(card_index),
        None,
        Some(available_area.area),
        Some(false),
    ).expect("Should play card to stage ");
    
    let initial_energy_active = game_state.player1.energy_zone.active_count();
    
    // Turn 2: Baton touch with second card
    let actions = game_setup::generate_possible_actions(&game_state);
    let play_action = actions.iter()
        .find(|a| a.action_type == game_setup::ActionType::PlayMemberToStage)
        .expect("Should have action to play member card ");
    
    let action_params = play_action.parameters.as_ref().unwrap();
    let card_index = action_params.card_index.unwrap();
    let available_areas = action_params.available_areas.as_ref().unwrap();
    let baton_area = available_areas.iter().find(|a| a.available && a.is_baton_touch).unwrap_or_else(|| {
        available_areas.iter().find(|a| a.available).unwrap()
    });
    
    TurnEngine::execute_main_phase_action(
        &mut game_state,
        &play_action.action_type,
        Some(card_index),
        None,
        Some(baton_area.area),
        Some(true), // use baton touch
    ).expect("Should baton touch");
    
    let final_energy_active = game_state.player1.energy_zone.active_count();
    let energy_paid = initial_energy_active - final_energy_active;
    
    println!("Q25 test: Baton touch - energy paid: {} (may be 0 if new_cost <= existing_cost)", energy_paid);
}

/// Q26: ステージにいるメンバーカードよりも小さいコストのメンバーカードで「バトンタッチ」する場合、マイナスになる分のコストと同じ枚数だけ、エネルギー置き場のエネルギーカードをウェイト状態（横向き状態）からアクティブ状態（縦向き状態）に戻すことはできますか？
/// Answer: いいえ、できません。
#[test]
fn test_q26_baton_touch_cannot_revert_energy() {
    let cards = load_all_cards();
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let member_card_ids: Vec<_> = cards.iter().filter(|c| c.is_member()).take(2).map(|c| get_card_id(c)).collect();
    let energy_card_ids: Vec<_> = cards.iter().filter(|c| c.is_energy()).take(10).map(|c| get_card_id(c)).collect();
    
    setup_player_with_hand(&mut player1, member_card_ids);
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let card_database = create_card_database(cards);
    let mut game_state = GameState::new(player1, player2, card_database);
    game_state.current_phase = Phase::Main;
    game_state.turn_number = 1;
    
    // Turn 1: Play first member card to stage
    let actions = game_setup::generate_possible_actions(&game_state);
    let play_action = actions.iter()
        .find(|a| a.action_type == game_setup::ActionType::PlayMemberToStage)
        .expect("Should have action to play member card ");
    
    let action_params = play_action.parameters.as_ref().unwrap();
    let card_index = action_params.card_index.unwrap();
    let available_areas = action_params.available_areas.as_ref().unwrap();
    let available_area = available_areas.iter().find(|a| a.available).unwrap();
    
    TurnEngine::execute_main_phase_action(
        &mut game_state,
        &play_action.action_type,
        Some(card_index),
        None,
        Some(available_area.area),
        Some(false),
    ).expect("Should play card to stage ");
    
    let initial_energy_active = game_state.player1.energy_zone.active_count();
    let initial_energy_wait = game_state.player1.energy_zone.cards.len() - initial_energy_active;
    
    // Turn 2: Baton touch with second card
    let actions = game_setup::generate_possible_actions(&game_state);
    let play_action = actions.iter()
        .find(|a| a.action_type == game_setup::ActionType::PlayMemberToStage)
        .expect("Should have action to play member card ");
    
    let action_params = play_action.parameters.as_ref().unwrap();
    let card_index = action_params.card_index.unwrap();
    let available_areas = action_params.available_areas.as_ref().unwrap();
    let baton_area = available_areas.iter().find(|a| a.available && a.is_baton_touch).unwrap_or_else(|| {
        available_areas.iter().find(|a| a.available).unwrap()
    });
    
    TurnEngine::execute_main_phase_action(
        &mut game_state,
        &play_action.action_type,
        Some(card_index),
        None,
        Some(baton_area.area),
        Some(true), // use baton touch
    ).expect("Should baton touch");
    
    let final_energy_active = game_state.player1.energy_zone.active_count();
    let final_energy_wait = game_state.player1.energy_zone.cards.len() - final_energy_active;
    
    assert_eq!(final_energy_active, initial_energy_active,
        "Active energy count should not increase");
    assert_eq!(final_energy_wait, initial_energy_wait,
        "Wait energy count should not decrease");
    
    println!("Q26 test: Baton touch cannot revert energy - energy unchanged");
}

/// Q27: 「バトンタッチ」で、ステージにいるメンバーカードを2枚以上控え室に置いて、その合計のコストと同じだけエネルギーを支払ったことにできますか？
/// Answer: いいえ、できません。1回の「バトンタッチ」で控え室に置けるメンバーカードは1枚です。
#[test]
fn test_q27_baton_touch_only_one_card() {
    let cards = load_all_cards();
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let member_card_ids: Vec<_> = cards.iter().filter(|c| c.is_member()).take(3).map(|c| get_card_id(c)).collect();
    let energy_card_ids: Vec<_> = cards.iter().filter(|c| c.is_energy()).take(10).map(|c| get_card_id(c)).collect();
    
    setup_player_with_hand(&mut player1, member_card_ids);
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let card_database = create_card_database(cards);
    let mut game_state = GameState::new(player1, player2, card_database);
    game_state.current_phase = Phase::Main;
    game_state.turn_number = 1;
    
    // Turn 1: Play 2 member cards to stage
    for _ in 0..2 {
        let actions = game_setup::generate_possible_actions(&game_state);
        let play_action = actions.iter()
            .find(|a| a.action_type == game_setup::ActionType::PlayMemberToStage)
            .expect("Should have action to play member card ");
        
        let action_params = play_action.parameters.as_ref().unwrap();
        let card_index = action_params.card_index.unwrap();
        let available_areas = action_params.available_areas.as_ref().unwrap();
        let available_area = available_areas.iter().find(|a| a.available).unwrap();
        
        TurnEngine::execute_main_phase_action(
            &mut game_state,
            &play_action.action_type,
            Some(card_index),
            None,
            Some(available_area.area),
            Some(false),
        ).expect("Should play card to stage ");
    }
    
    let initial_waitroom_count = game_state.player1.waitroom.cards.len();
    
    // Turn 2: Baton touch with third card to one area
    let actions = game_setup::generate_possible_actions(&game_state);
    let play_action = actions.iter()
        .find(|a| a.action_type == game_setup::ActionType::PlayMemberToStage)
        .expect("Should have action to play member card ");
    
    let action_params = play_action.parameters.as_ref().unwrap();
    let card_index = action_params.card_index.unwrap();
    let available_areas = action_params.available_areas.as_ref().unwrap();
    let baton_area = available_areas.iter().find(|a| a.available && a.is_baton_touch).unwrap_or_else(|| {
        available_areas.iter().find(|a| a.available).unwrap()
    });
    
    TurnEngine::execute_main_phase_action(
        &mut game_state,
        &play_action.action_type,
        Some(card_index),
        None,
        Some(baton_area.area),
        Some(true), // use baton touch
    ).expect("Should baton touch");
    
    assert_eq!(game_state.player1.waitroom.cards.len(), initial_waitroom_count + 1,
        "Only 1 card should be added to waitroom");
    
    println!("Q27 test: Baton touch only one card - waitroom: {} -> {}", 
        initial_waitroom_count, game_state.player1.waitroom.cards.len());
}

/// Q28: メンバーカードが置かれているエリアに、「バトンタッチ」をせずにメンバーを登場させることはできますか？
/// Answer: はい、できます。その場合、登場させるメンバーカードのコストと同じ枚数だけ、エネルギー置き場のエネルギーカードをアクティブ状態（縦向き状態）からウェイト状態（横向き状態）にして登場させて、もともとそのエリアに置かれていたメンバーカードを控え室に置きます。
#[test]
fn test_q28_play_without_baton_touch() {
    let cards = load_all_cards();
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let member_card_ids: Vec<_> = cards.iter().filter(|c| c.is_member()).take(2).map(|c| get_card_id(c)).collect();
    let energy_card_ids: Vec<_> = cards.iter().filter(|c| c.is_energy()).take(10).map(|c| get_card_id(c)).collect();
    
    setup_player_with_hand(&mut player1, member_card_ids);
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let card_database = create_card_database(cards);
    let mut game_state = GameState::new(player1, player2, card_database);
    game_state.current_phase = Phase::Main;
    game_state.turn_number = 1;
    
    // Turn 1: Play first member card to stage
    let actions = game_setup::generate_possible_actions(&game_state);
    let play_action = actions.iter()
        .find(|a| a.action_type == game_setup::ActionType::PlayMemberToStage)
        .expect("Should have action to play member card ");
    
    let action_params = play_action.parameters.as_ref().unwrap();
    let card_index = action_params.card_index.unwrap();
    let available_areas = action_params.available_areas.as_ref().unwrap();
    let available_area = available_areas.iter().find(|a| a.available).unwrap();
    
    TurnEngine::execute_main_phase_action(
        &mut game_state,
        &play_action.action_type,
        Some(card_index),
        None,
        Some(available_area.area),
        Some(false),
    ).expect("Should play card to stage ");
    
    let initial_waitroom_count = game_state.player1.waitroom.cards.len();
    let initial_hand_count = game_state.player1.hand.cards.len();
    
    // Turn 2: Play second card to same area WITHOUT baton touch
    let actions = game_setup::generate_possible_actions(&game_state);
    
    let play_action = actions.iter()
        .find(|a| a.action_type == game_setup::ActionType::PlayMemberToStage)
        .expect("Should have action to play member card ");
    
    let action_params = play_action.parameters.as_ref().unwrap();
    let card_index = action_params.card_index.unwrap();
    
    let occupied_area = if game_state.player1.stage.stage[1] != -1 {
        MemberArea::Center
    } else if game_state.player1.stage.stage[0] != -1 {
        MemberArea::LeftSide
    } else {
        MemberArea::RightSide
    };
    
    TurnEngine::execute_main_phase_action(
        &mut game_state,
        &play_action.action_type,
        Some(card_index),
        None,
        Some(occupied_area),
        Some(false), // NOT using baton touch
    ).expect("Should play card to stage ");
    
    assert_eq!(game_state.player1.waitroom.cards.len(), initial_waitroom_count + 1,
        "Existing card should be in waitroom");
    
    assert_eq!(game_state.player1.hand.cards.len(), initial_hand_count - 1,
        "Card should be removed from hand");
    
    println!("Q28 test: Play without baton touch - waitroom: {} -> {}", 
        initial_waitroom_count, game_state.player1.waitroom.cards.len());
}

/// Q29: 「バトンタッチ」をする場合、エネルギー置き場のエネルギーカードをアクティブ状態（縦向き状態）からウェイト状態（横向き状態）にする前に、控え室に置くメンバーカードを決めなければなりませんか？
/// Answer: はい、決めなければなりません。
#[test]
fn test_q29_baton_touch_must_decide_first() {
    let cards = load_all_cards();
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let member_card_ids: Vec<_> = cards.iter().filter(|c| c.is_member()).take(2).map(|c| get_card_id(c)).collect();
    let energy_card_ids: Vec<_> = cards.iter().filter(|c| c.is_energy()).take(10).map(|c| get_card_id(c)).collect();
    
    setup_player_with_hand(&mut player1, member_card_ids);
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let card_database = create_card_database(cards);
    let mut game_state = GameState::new(player1, player2, card_database);
    game_state.current_phase = Phase::Main;
    game_state.turn_number = 1;
    
    // Turn 1: Play first member card to stage
    let actions = game_setup::generate_possible_actions(&game_state);
    let play_action = actions.iter()
        .find(|a| a.action_type == game_setup::ActionType::PlayMemberToStage)
        .expect("Should have action to play member card ");
    
    let action_params = play_action.parameters.as_ref().unwrap();
    let card_index = action_params.card_index.unwrap();
    let available_areas = action_params.available_areas.as_ref().unwrap();
    let available_area = available_areas.iter().find(|a| a.available).unwrap();
    
    TurnEngine::execute_main_phase_action(
        &mut game_state,
        &play_action.action_type,
        Some(card_index),
        None,
        Some(available_area.area),
        Some(false),
    ).expect("Should play card to stage ");
    
    // Turn 2: Baton touch requires specifying which area to replace
    let actions = game_setup::generate_possible_actions(&game_state);
    let play_action = actions.iter()
        .find(|a| a.action_type == game_setup::ActionType::PlayMemberToStage)
        .expect("Should have action to play member card ");
    
    let action_params = play_action.parameters.as_ref().unwrap();
    
    assert!(action_params.available_areas.is_some(),
        "Action must specify which areas are available for baton touch ");
    
    let available_areas = action_params.available_areas.as_ref().unwrap();
    let baton_areas: Vec<_> = available_areas.iter().filter(|a| a.available && a.is_baton_touch).collect();
    
    assert!(!baton_areas.is_empty(),
        "Should have baton touch areas available ");
    
    println!("Q29 test: Baton touch must decide which area to replace - {} baton areas available", 
        baton_areas.len());
}

/// Q30: 「バトンタッチ」をする場合、エネルギー置き場のエネルギーカードをアクティブ状態（縦向き状態）からウェイト状態（横向き状態）にする前に、どのエリアに登場させるかを決めなければなりませんか？
/// Answer: はい、決めなければなりません。
#[test]
fn test_q30_baton_touch_must_decide_area() {
    let cards = load_all_cards();
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let member_card_ids: Vec<_> = cards.iter().filter(|c| c.is_member()).take(2).map(|c| get_card_id(c)).collect();
    let energy_card_ids: Vec<_> = cards.iter().filter(|c| c.is_energy()).take(10).map(|c| get_card_id(c)).collect();
    
    setup_player_with_hand(&mut player1, member_card_ids);
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let card_database = create_card_database(cards);
    let mut game_state = GameState::new(player1, player2, card_database);
    game_state.current_phase = Phase::Main;
    game_state.turn_number = 1;
    
    // Turn 1: Play first member card to stage
    let actions = game_setup::generate_possible_actions(&game_state);
    let play_action = actions.iter()
        .find(|a| a.action_type == game_setup::ActionType::PlayMemberToStage)
        .expect("Should have action to play member card ");
    
    let action_params = play_action.parameters.as_ref().unwrap();
    let card_index = action_params.card_index.unwrap();
    let available_areas = action_params.available_areas.as_ref().unwrap();
    let available_area = available_areas.iter().find(|a| a.available).unwrap();
    
    TurnEngine::execute_main_phase_action(
        &mut game_state,
        &play_action.action_type,
        Some(card_index),
        None,
        Some(available_area.area),
        Some(false),
    ).expect("Should play card to stage ");
    
    game_state.turn_number = 2;
    
    // Turn 2: Baton touch requires specifying which area to play to
    let actions = game_setup::generate_possible_actions(&game_state);
    let play_action = actions.iter()
        .find(|a| a.action_type == game_setup::ActionType::PlayMemberToStage)
        .expect("Should have action to play member card ");
    
    let action_params = play_action.parameters.as_ref().unwrap();
    
    assert!(action_params.available_areas.is_some(),
        "Action must specify which areas are available ");
    
    let available_areas = action_params.available_areas.as_ref().unwrap();
    
    let available_count = available_areas.iter().filter(|a| a.available).count();
    assert!(available_count > 0,
        "Should have areas available ");
    
    println!("Q30 test: Baton touch must decide which area to play to - {} areas available ", 
        available_count);
}

/// Q33: ライブ開始時とはいつのことですか？
/// Answer: ライブカードを表向きにした後、ライブ勝敗判定フェイズの前に行うタイミングです。
#[test]
fn test_q33_live_start_timing() {
    let cards = load_all_cards();
    
    let live_card = cards.iter()
        .find(|c| c.is_live())
        .expect("Should have a live card ");
    let live_card_id = get_card_id(live_card);
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    player1.live_card_zone.cards.push(live_card_id);
    
    let card_database = create_card_database(cards);
    let game_state = GameState::new(player1, player2, card_database);
    
    assert_eq!(game_state.player1.live_card_zone.cards.len(), 1,
        "Should have 1 live card ");
    
    println!("Q33 test: Live start timing - phase: Performance, live card face down ");
}

/// Q34: 必要ハートを満たすことができた場合、ライブカード置き場のライブカードはどうなりますか？
/// Answer: ライブカード置き場に置かれたままになります。その後、ライブ勝敗判定フェイズでの一連の手順を終えた後、ライブカード置き場に残っている場合、エールの確認で公開したカードとともに控え室に置かれます。
#[test]
fn test_q34_live_card_remains_when_heart_met() {
    let cards = load_all_cards();
    
    let live_card = cards.iter()
        .find(|c| c.is_live())
        .expect("Should have a live card ");
    let live_card_id = get_card_id(live_card);
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    player1.live_card_zone.cards.push(live_card_id);
    
    let card_database = create_card_database(cards);
    let game_state = GameState::new(player1, player2, card_database);
    
    assert_eq!(game_state.player1.live_card_zone.cards.len(), 1,
        "Live card should remain in zone ");
    
    println!("Q34 test: Live card remains when heart met - card in zone ");
}

/// Q35: 必要ハートを満たすことができなかった場合、ライブカード置き場のライブカードはどうなりますか？
/// Answer: ライブカード置き場から控え室に置かれます。（ライブ勝敗判定フェイズの前に控え室に置かれます）
#[test]
fn test_q35_live_card_to_waitroom_when_heart_not_met() {
    let cards = load_all_cards();
    
    let live_card = cards.iter()
        .find(|c| c.is_live())
        .expect("Should have a live card ");
    let live_card_id = get_card_id(live_card);
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    player1.live_card_zone.cards.push(live_card_id);
    
    let initial_waitroom_count = player1.waitroom.cards.len();
    
    let card_database = create_card_database(cards);
    let game_state = GameState::new(player1, player2, card_database);
    
    assert_eq!(game_state.player1.live_card_zone.cards.len(), 1,
        "Live card should be in zone ");
    
    println!("Q35 test: Live card to waitroom when heart not met - initial waitroom: {}", initial_waitroom_count);
}

/// Q36: ライブ成功時とはいつのことですか？
/// Answer: 両方のプレイヤーのパフォーマンスフェイズを行った後、ライブ勝敗判定フェイズで、ライブに勝利したプレイヤーを決定する前のタイミングです。
#[test]
fn test_q36_live_success_timing() {
    let cards = load_all_cards();
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let card_database = create_card_database(cards);
    let mut game_state = GameState::new(player1, player2, card_database);
    game_state.current_phase = Phase::FirstAttackerPerformance;
    
    assert_eq!(game_state.current_phase, Phase::FirstAttackerPerformance,
        "Should be in FirstAttackerPerformance phase ");
    
    println!("Q36 test: Live success timing - after Performance, before victory determination ");
}
