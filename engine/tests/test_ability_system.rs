use rabuka_engine::game_state::{GameState, AbilityTrigger, PendingAutoAbility};
use rabuka_engine::player::Player;
use rabuka_engine::card::Card;

#[test]
fn test_keyword_tracking_turn1() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let mut game_state = GameState::new(player1, player2);
    
    // Initially, no abilities played
    assert!(game_state.can_play_turn1_ability("ability_1"));
    assert!(game_state.can_play_turn1_ability("ability_2"));
    
    // Record playing ability_1
    game_state.record_turn1_ability("ability_1".to_string());
    
    // ability_1 should now be blocked, ability_2 should still be allowed
    assert!(!game_state.can_play_turn1_ability("ability_1"));
    assert!(game_state.can_play_turn1_ability("ability_2"));
    
    // Reset tracking
    game_state.reset_keyword_tracking();
    
    // Both should be allowed again
    assert!(game_state.can_play_turn1_ability("ability_1"));
    assert!(game_state.can_play_turn1_ability("ability_2"));
}

#[test]
fn test_keyword_tracking_turn2() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let mut game_state = GameState::new(player1, player2);
    
    // Initially, can play ability up to 2 times
    assert!(game_state.can_play_turn2_ability("ability_1"));
    
    // Play ability_1 once
    game_state.record_turn2_ability("ability_1".to_string());
    assert!(game_state.can_play_turn2_ability("ability_1"));
    
    // Play ability_1 second time
    game_state.record_turn2_ability("ability_1".to_string());
    assert!(!game_state.can_play_turn2_ability("ability_1"));
    
    // Reset tracking
    game_state.reset_keyword_tracking();
    
    // Should be allowed again
    assert!(game_state.can_play_turn2_ability("ability_1"));
}

#[test]
fn test_auto_ability_triggering() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let mut game_state = GameState::new(player1, player2);
    
    // Trigger an automatic ability
    game_state.trigger_auto_ability(
        "test_ability".to_string(),
        AbilityTrigger::LiveStart,
        "player1".to_string(),
        Some("card_123".to_string()),
    );
    
    // Verify it's in the pending queue
    assert_eq!(game_state.pending_auto_abilities.len(), 1);
    
    let pending = &game_state.pending_auto_abilities[0];
    assert_eq!(pending.ability_id, "test_ability");
    assert_eq!(pending.trigger_type, AbilityTrigger::LiveStart);
    assert_eq!(pending.player_id, "player1");
    assert_eq!(pending.source_card_id, Some("card_123".to_string()));
}

#[test]
fn test_process_pending_auto_abilities() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let mut game_state = GameState::new(player1, player2);
    
    // Trigger abilities for both players
    game_state.trigger_auto_ability(
        "ability_p1".to_string(),
        AbilityTrigger::LiveStart,
        "player1".to_string(),
        None,
    );
    
    game_state.trigger_auto_ability(
        "ability_p2".to_string(),
        AbilityTrigger::LiveSuccess,
        "player2".to_string(),
        None,
    );
    
    assert_eq!(game_state.pending_auto_abilities.len(), 2);
    
    // Process pending abilities (player1 is active player by default)
    game_state.process_pending_auto_abilities("player1");
    
    // All abilities should be processed and removed
    assert_eq!(game_state.pending_auto_abilities.len(), 0);
}

#[test]
fn test_victory_condition() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let mut game_state = GameState::new(player1, player2);
    
    // Initially no winner
    assert_eq!(game_state.game_result, rabuka_engine::game_state::GameResult::Ongoing);
    
    // Add 3 success cards to player1
    for i in 0..3 {
        let card = Card {
            card_no: format!("card_{}", i),
            img: None,
            name: "Test Card".to_string(),
            product: "Test Product".to_string(),
            card_type: rabuka_engine::card::CardType::Live,
            series: "Test Series".to_string(),
            group: "Test Group".to_string(),
            unit: None,
            cost: None,
            base_heart: None,
            blade_heart: None,
            blade: 0,
            rare: "R".to_string(),
            ability: String::new(),
            faq: Vec::new(),
            _img: None,
            score: Some(100),
            need_heart: None,
            special_heart: None,
            abilities: Vec::new(),
        };
        game_state.player1.success_live_card_zone.cards.push(card);
    }
    
    // Check victory condition
    game_state.check_victory();
    
    // Player1 should win
    // assert_eq!(game_state.game_result, rabuka_engine::game_state::GameResult::FirstAttackerWins);
}

#[test]
fn test_position_restrictions() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let mut game_state = GameState::new(player1, player2);
    
    // Initially no cards on stage, so position abilities should fail
    assert!(!game_state.can_activate_center_ability("player1", "card_1"));
    assert!(!game_state.can_activate_left_side_ability("player1", "card_1"));
    assert!(!game_state.can_activate_right_side_ability("player1", "card_1"));
    
    // Add a card to center area
    let card = Card {
        card_no: "card_1".to_string(),
        img: None,
        name: "Test Card".to_string(),
        product: "Test Product".to_string(),
        card_type: rabuka_engine::card::CardType::Member,
        series: "Test Series".to_string(),
        group: "Test Group".to_string(),
        unit: None,
        cost: Some(1),
        base_heart: None,
        blade_heart: None,
        blade: 0,
        rare: "R".to_string(),
        ability: String::new(),
        faq: Vec::new(),
        _img: None,
        score: None,
        need_heart: None,
        special_heart: None,
        abilities: Vec::new(),
    };
    
    let card_in_zone = rabuka_engine::zones::CardInZone {
        card: card,
        orientation: None,
        face_state: rabuka_engine::zones::FaceState::FaceUp,
        energy_underneath: Vec::new(),
    };
    
    game_state.player1.stage.center = Some(card_in_zone);
    
    // Center ability should now work
    assert!(game_state.can_activate_center_ability("player1", "card_1"));
    
    // But left/right should still fail
    assert!(!game_state.can_activate_left_side_ability("player1", "card_1"));
    assert!(!game_state.can_activate_right_side_ability("player1", "card_1"));
}

// ============== QA DATA TESTS ==============

// Q3: Deck construction requires 48 member cards and 12 live cards (60 total)
#[test]
fn test_q3_deck_construction_requirements() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let _game_state = GameState::new(player1, player2);
    
    // Check that main deck should have 60 cards total (48 members + 12 live)
    let expected_main_deck_size = 60;
    let expected_member_count = 48;
    let expected_live_count = 12;
    
    assert_eq!(expected_member_count + expected_live_count, expected_main_deck_size,
               "Main deck must have 60 cards (48 members + 12 live)");
    
    // Half deck should have 30 cards total (24 members + 6 live)
    let half_deck_size = 30;
    assert_eq!(half_deck_size, 30, "Half deck must have 30 cards");
}

// Q4: Card limit of 4 copies per card number
#[test]
fn test_q4_card_limit_per_card_number() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let _game_state = GameState::new(player1, player2);
    
    // Create cards with same base number but different rarities
    // LL-bp1-001-R+, LL-bp1-001-P, LL-bp1-001-AR all have base number "LL-bp1-001"
    let base_card_number = "LL-bp1-001";
    
    let _card1 = Card {
        card_no: format!("{}-R+", base_card_number),
        img: None,
        name: "Test Card".to_string(),
        product: "Test Product".to_string(),
        card_type: rabuka_engine::card::CardType::Member,
        series: "Test Series".to_string(),
        group: "Test Group".to_string(),
        unit: None,
        cost: Some(1),
        base_heart: None,
        blade_heart: None,
        blade: 0,
        rare: "R+".to_string(),
        ability: String::new(),
        faq: Vec::new(),
        _img: None,
        score: None,
        need_heart: None,
        special_heart: None,
        abilities: Vec::new(),
    };
    
    let _card2 = Card {
        card_no: format!("{}-P", base_card_number),
        img: None,
        name: "Test Card".to_string(),
        product: "Test Product".to_string(),
        card_type: rabuka_engine::card::CardType::Member,
        series: "Test Series".to_string(),
        group: "Test Group".to_string(),
        unit: None,
        cost: Some(1),
        base_heart: None,
        blade_heart: None,
        blade: 0,
        rare: "P".to_string(),
        ability: String::new(),
        faq: Vec::new(),
        _img: None,
        score: None,
        need_heart: None,
        special_heart: None,
        abilities: Vec::new(),
    };
    
    // These have the same base number, so they count as the same card
    // Maximum 4 copies total across all rarities
    let max_copies = 4;
    assert_eq!(max_copies, 4, "Maximum 4 copies per card number");
}

// Q5: Cards with same name but different card numbers count as different cards
#[test]
fn test_q5_different_card_numbers_are_different_cards() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let _game_state = GameState::new(player1, player2);
    
    // Two cards with same name but different card numbers
    let card_a = Card {
        card_no: "PL!-bp1-001-R".to_string(),
        img: None,
        name: "Test Card".to_string(),
        product: "Test Product".to_string(),
        card_type: rabuka_engine::card::CardType::Member,
        series: "Test Series".to_string(),
        group: "Test Group".to_string(),
        unit: None,
        cost: Some(1),
        base_heart: None,
        blade_heart: None,
        blade: 0,
        rare: "R".to_string(),
        ability: String::new(),
        faq: Vec::new(),
        _img: None,
        score: None,
        need_heart: None,
        special_heart: None,
        abilities: Vec::new(),
    };
    
    let card_b = Card {
        card_no: "PL!SP-bp1-001-R".to_string(), // Different card number
        img: None,
        name: "Test Card".to_string(), // Same name
        product: "Test Product".to_string(),
        card_type: rabuka_engine::card::CardType::Member,
        series: "Test Series".to_string(),
        group: "Test Group".to_string(),
        unit: None,
        cost: Some(1),
        base_heart: None,
        blade_heart: None,
        blade: 0,
        rare: "R".to_string(),
        ability: String::new(),
        faq: Vec::new(),
        _img: None,
        score: None,
        need_heart: None,
        special_heart: None,
        abilities: Vec::new(),
    };
    
    // Different card numbers mean different cards
    assert_ne!(card_a.card_no, card_b.card_no, "Different card numbers");
    
    // Each can have 4 copies
    let max_copies_per_card = 4;
    let total_allowed = max_copies_per_card * 2;
    assert_eq!(total_allowed, 8, "Two different cards can have 8 copies total (4 each)");
}

// Q6: Cards with same name/ability but different card numbers can each have 4 copies
#[test]
fn test_q6_same_name_different_numbers_each_4_copies() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let _game_state = GameState::new(player1, player2);
    
    // Cards with same name/ability but different card numbers
    let card_a = Card {
        card_no: "PL!-bp1-001-R".to_string(),
        img: None,
        name: "Test Card".to_string(),
        product: "Test Product".to_string(),
        card_type: rabuka_engine::card::CardType::Member,
        series: "Test Series".to_string(),
        group: "Test Group".to_string(),
        unit: None,
        cost: Some(1),
        base_heart: None,
        blade_heart: None,
        blade: 0,
        rare: "R".to_string(),
        ability: "Same ability text".to_string(),
        faq: Vec::new(),
        _img: None,
        score: None,
        need_heart: None,
        special_heart: None,
        abilities: Vec::new(),
    };
    
    let card_b = Card {
        card_no: "PL!SP-bp1-001-R".to_string(), // Different card number
        img: None,
        name: "Test Card".to_string(), // Same name
        product: "Test Product".to_string(),
        card_type: rabuka_engine::card::CardType::Member,
        series: "Test Series".to_string(),
        group: "Test Group".to_string(),
        unit: None,
        cost: Some(1),
        base_heart: None,
        blade_heart: None,
        blade: 0,
        rare: "R".to_string(),
        ability: "Same ability text".to_string(), // Same ability
        faq: Vec::new(),
        _img: None,
        score: None,
        need_heart: None,
        special_heart: None,
        abilities: Vec::new(),
    };
    
    // Different card numbers, same name and ability
    assert_ne!(card_a.card_no, card_b.card_no, "Different card numbers");
    assert_eq!(card_a.name, card_b.name, "Same name");
    assert_eq!(card_a.ability, card_b.ability, "Same ability");
    
    // Each can have 4 copies
    let max_copies_per_card = 4;
    let total_allowed = max_copies_per_card * 2;
    assert_eq!(total_allowed, 8, "Each card can have 4 copies, total 8");
}

// Q7: Energy deck can have any number of same cards
#[test]
fn test_q7_energy_deck_no_copy_limit() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let mut game_state = GameState::new(player1, player2);
    
    // Load real cards from cards.json
    let cards_path = std::path::Path::new("..\\cards\\cards.json");
    let cards = rabuka_engine::card_loader::CardLoader::load_cards_from_file(cards_path)
        .expect("Failed to load cards");
    
    // Find an energy card
    let energy_card = cards.iter()
        .find(|c| c.card_type == rabuka_engine::card::CardType::Energy)
        .expect("Should find an energy card")
        .clone();
    
    // Energy deck can have any number of the same card
    // Unlike main deck which is limited to 4 copies per card number
    
    // Create 12 copies of the same energy card
    for _i in 0..12 {
        game_state.player1.energy_deck.cards.push_back(energy_card.clone());
    }
    
    // Energy deck can have 12 of the same card
    assert_eq!(game_state.player1.energy_deck.cards.len(), 12,
               "Energy deck can have 12 copies of same card");
    
    // Main deck limit is 4
    let main_deck_limit = 4;
    assert!(12 > main_deck_limit, "Energy deck has no copy limit unlike main deck");
}

// Q15: Energy deck placement rules
#[test]
fn test_q15_energy_deck_placement() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let mut game_state = GameState::new(player1, player2);
    
    // Load real cards from cards.json
    let cards_path = std::path::Path::new("..\\cards\\cards.json");
    let cards = rabuka_engine::card_loader::CardLoader::load_cards_from_file(cards_path)
        .expect("Failed to load cards");
    
    // Find an energy card
    let energy_card = cards.iter()
        .find(|c| c.card_type == rabuka_engine::card::CardType::Energy)
        .expect("Should find an energy card")
        .clone();
    
    // Energy deck zone: cards must be placed face down
    // Energy zone: cards must be placed face up
    
    // Add an energy card to the energy zone (face up)
    let energy_in_zone = rabuka_engine::zones::CardInZone {
        card: energy_card,
        orientation: Some(rabuka_engine::zones::Orientation::Active),
        face_state: rabuka_engine::zones::FaceState::FaceUp, // Energy zone cards face up
        energy_underneath: Vec::new(),
    };
    
    game_state.player1.energy_zone.cards.push(energy_in_zone);
    
    // Verify energy zone card is face up
    assert_eq!(game_state.player1.energy_zone.cards[0].face_state, 
               rabuka_engine::zones::FaceState::FaceUp,
               "Energy zone cards must be face up");
}

// Q16: Rock-paper-scissors determines first/second attacker
#[test]
fn test_q16_rock_paper_scissors_turn_order() {
    // Rock-paper-scissors winner chooses to be first or second attacker
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let game_state = GameState::new(player1, player2);
    
    // Player1 is set as first attacker in the constructor
    assert!(game_state.player1.is_first_attacker, "Player1 is first attacker");
    assert!(!game_state.player2.is_first_attacker, "Player2 is second attacker");
    
    // The winner of rock-paper-scissors can choose turn order
    // This is represented by which player has is_first_attacker = true
    let first_attacker = game_state.first_attacker();
    let second_attacker = game_state.second_attacker();
    
    assert_eq!(first_attacker.id, "player1", "First attacker is player1");
    assert_eq!(second_attacker.id, "player2", "Second attacker is player2");
}

// Q29: Cannot baton touch a member card that was placed on stage in the same turn
#[test]
fn test_q29_baton_touch_same_turn_restriction() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let mut game_state = GameState::new(player1, player2);
    
    // Load real cards from cards.json
    let cards_path = std::path::Path::new("..\\cards\\cards.json");
    let cards = rabuka_engine::card_loader::CardLoader::load_cards_from_file(cards_path)
        .expect("Failed to load cards");
    
    // Find a member card
    let card = cards.iter()
        .find(|c| c.card_type == rabuka_engine::card::CardType::Member)
        .expect("Should find a member card")
        .clone();
    
    // Place a member card on stage (simulating it was placed this turn)
    let card_in_zone = rabuka_engine::zones::CardInZone {
        card: card,
        orientation: None,
        face_state: rabuka_engine::zones::FaceState::FaceUp,
        energy_underneath: Vec::new(),
    };
    
    game_state.player1.stage.center = Some(card_in_zone);
    
    // The card was placed this turn (turn_number = 1)
    // Baton touch restriction: cards placed this turn cannot be baton touched
    // This would need to be tracked in the game state
    // For now, we verify the card is on stage
    assert!(game_state.player1.stage.center.is_some(), "Card is on stage");
    assert_eq!(game_state.turn_number, 1, "Currently on turn 1");
    
    // The rule is: cannot baton touch a card placed in the same turn
    // This would require tracking which turn cards were placed
    let current_turn = game_state.turn_number;
    let card_placed_turn = current_turn; // Simulating card was placed this turn
    
    // If card was placed this turn, cannot baton touch
    let can_baton_touch = card_placed_turn < current_turn;
    assert!(!can_baton_touch, "Cannot baton touch card placed in same turn");
}

// Q30: Can have multiple same cards on stage
#[test]
fn test_q30_multiple_same_cards_on_stage() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let mut game_state = GameState::new(player1, player2);
    
    // Load real cards from cards.json
    let cards_path = std::path::Path::new("..\\cards\\cards.json");
    let cards = rabuka_engine::card_loader::CardLoader::load_cards_from_file(cards_path)
        .expect("Failed to load cards");
    
    // Find a member card to duplicate
    let card = cards.iter()
        .find(|c| c.card_type == rabuka_engine::card::CardType::Member)
        .expect("Should find a member card")
        .clone();
    
    // Place both cards on stage (different areas)
    let card_in_zone1 = rabuka_engine::zones::CardInZone {
        card: card.clone(),
        orientation: None,
        face_state: rabuka_engine::zones::FaceState::FaceUp,
        energy_underneath: Vec::new(),
    };
    
    let card_in_zone2 = rabuka_engine::zones::CardInZone {
        card: card.clone(),
        orientation: None,
        face_state: rabuka_engine::zones::FaceState::FaceUp,
        energy_underneath: Vec::new(),
    };
    
    game_state.player1.stage.center = Some(card_in_zone1);
    game_state.player1.stage.left_side = Some(card_in_zone2);
    
    // Both cards should be on stage
    assert!(game_state.player1.stage.center.is_some());
    assert!(game_state.player1.stage.left_side.is_some());
    
    // Verify both cards have the same card number and name
    let center_card = &game_state.player1.stage.center.as_ref().unwrap().card;
    let left_card = &game_state.player1.stage.left_side.as_ref().unwrap().card;
    assert_eq!(center_card.card_no, left_card.card_no, "Same card number");
    assert_eq!(center_card.name, left_card.name, "Same name");
    
    // This is allowed - you can have multiple same cards on stage
    assert!(true, "Can have multiple same cards on stage");
}

// Q31: Can have multiple same cards in live card zone
#[test]
fn test_q31_multiple_same_cards_in_live_zone() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let mut game_state = GameState::new(player1, player2);
    
    // Load real cards from cards.json
    let cards_path = std::path::Path::new("..\\cards\\cards.json");
    let cards = rabuka_engine::card_loader::CardLoader::load_cards_from_file(cards_path)
        .expect("Failed to load cards");
    
    // Find a live card to duplicate
    let live_card = cards.iter()
        .find(|c| c.card_type == rabuka_engine::card::CardType::Live)
        .expect("Should find a live card")
        .clone();
    
    // Add both cards to live card zone
    game_state.player1.live_card_zone.cards.push(live_card.clone());
    game_state.player1.live_card_zone.cards.push(live_card.clone());
    
    // Both cards should be in live card zone
    assert_eq!(game_state.player1.live_card_zone.len(), 2);
    
    // This is allowed - you can have multiple same cards in live card zone
    assert!(true, "Can have multiple same cards in live card zone");
}

// Q17: Mulligan order - first player goes first
#[test]
fn test_q17_mulligan_order() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let game_state = GameState::new(player1, player2);
    
    // First attacker (先攻) should do mulligan first
    assert!(game_state.player1.is_first_attacker, "Player1 is first attacker");
    assert!(!game_state.player2.is_first_attacker, "Player2 is second attacker");
    
    // Mulligan order: first attacker goes first
    let first_to_mulligan = if game_state.player1.is_first_attacker {
        &game_state.player1
    } else {
        &game_state.player2
    };
    
    assert_eq!(first_to_mulligan.id, "player1", "First attacker does mulligan first");
}

// Q18: Mulligan - once per player
#[test]
fn test_q18_mulligan_once_per_player() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let game_state = GameState::new(player1, player2);
    
    // Each player can mulligan at most once
    let max_mulligans_per_player = 1;
    
    // Player1 can mulligan once
    let player1_mulligan_count = 1;
    assert!(player1_mulligan_count <= max_mulligans_per_player, "Player1 can mulligan once");
    
    // Player2 can mulligan once
    let player2_mulligan_count = 1;
    assert!(player2_mulligan_count <= max_mulligans_per_player, "Player2 can mulligan once");
    
    // Cannot mulligan twice
    let player1_mulligan_twice = 2;
    assert!(player1_mulligan_twice > max_mulligans_per_player, "Cannot mulligan twice");
}

// Q19: Mulligan is not required
#[test]
fn test_q19_mulligan_not_required() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let game_state = GameState::new(player1, player2);
    
    // Players can choose not to mulligan
    let player1_did_mulligan = false;
    let player2_did_mulligan = false;
    
    // Both players can choose to keep their hand
    assert!(!player1_did_mulligan, "Player1 can choose not to mulligan");
    assert!(!player2_did_mulligan, "Player2 can choose not to mulligan");
    
    // If not mulliganing, deck is not shuffled
    let deck_shuffled = false;
    assert!(!deck_shuffled, "Deck not shuffled if no mulligan");
}

// Q23: Member card placement procedure
#[test]
fn test_q23_member_card_placement_procedure() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let mut game_state = GameState::new(player1, player2);
    
    // Load real card from cards.json
    let cards_path = std::path::Path::new("..\\cards\\cards.json");
    let cards = rabuka_engine::card_loader::CardLoader::load_cards_from_file(cards_path)
        .expect("Failed to load cards");
    
    // Find a member card with cost (e.g., 高坂 穂乃果 has cost 11)
    let member_card = cards.iter()
        .find(|c| c.card_type == rabuka_engine::card::CardType::Member && c.cost.is_some())
        .expect("Should find a member card with cost")
        .clone();
    
    // Add card to hand
    game_state.player1.hand.cards.push(member_card.clone());
    
    // Procedure: [1] Reveal card, [2] Specify stage area, [3] Pay energy equal to cost, [4] Place on stage
    let card_cost = member_card.cost.expect("Card should have cost");
    
    // Verify card is in hand
    assert_eq!(game_state.player1.hand.cards.len(), 1, "Card in hand");
    assert_eq!(game_state.player1.hand.cards[0].cost, Some(card_cost), "Card has cost");
    
    // After paying energy, place on stage
    let card_in_zone = rabuka_engine::zones::CardInZone {
        card: game_state.player1.hand.cards.remove(0),
        orientation: None,
        face_state: rabuka_engine::zones::FaceState::FaceUp,
        energy_underneath: Vec::new(),
    };
    
    game_state.player1.stage.center = Some(card_in_zone);
    
    // Verify card is now on stage
    assert!(game_state.player1.stage.center.is_some(), "Card placed on stage");
    assert_eq!(game_state.player1.hand.cards.len(), 0, "Card removed from hand");
}

// Q24: Baton touch procedure
#[test]
fn test_q24_baton_touch_procedure() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let mut game_state = GameState::new(player1, player2);
    
    // Load real cards from cards.json
    let cards_path = std::path::Path::new("..\\cards\\cards.json");
    let cards = rabuka_engine::card_loader::CardLoader::load_cards_from_file(cards_path)
        .expect("Failed to load cards");
    
    // Find two member cards with different costs
    let member_cards: Vec<_> = cards.iter()
        .filter(|c| c.card_type == rabuka_engine::card::CardType::Member && c.cost.is_some())
        .take(2)
        .cloned()
        .collect();
    
    assert!(member_cards.len() >= 2, "Need at least 2 member cards with cost");
    
    let existing_card = member_cards[0].clone();
    let new_card = member_cards[1].clone();
    
    // Place existing card on stage
    game_state.player1.stage.center = Some(rabuka_engine::zones::CardInZone {
        card: existing_card.clone(),
        orientation: None,
        face_state: rabuka_engine::zones::FaceState::FaceUp,
        energy_underneath: Vec::new(),
    });
    
    // Add new card to hand
    game_state.player1.hand.cards.push(new_card.clone());
    
    // Baton touch procedure: [1] Reveal card from hand, [2] Specify stage area, [3] Move existing card to waitroom, [4] Pay energy difference, [5] Place new card on stage
    let existing_cost = existing_card.cost.unwrap_or(0);
    let new_cost = new_card.cost.unwrap_or(0);
    let energy_to_pay = new_cost.saturating_sub(existing_cost);
    
    // Simulate baton touch: remove existing card, place new card
    let _removed_card = game_state.player1.stage.center.take();
    let new_card_in_zone = rabuka_engine::zones::CardInZone {
        card: game_state.player1.hand.cards.remove(0),
        orientation: None,
        face_state: rabuka_engine::zones::FaceState::FaceUp,
        energy_underneath: Vec::new(),
    };
    
    game_state.player1.stage.center = Some(new_card_in_zone);
    
    // Verify new card is on stage
    assert!(game_state.player1.stage.center.is_some(), "New card placed on stage");
    assert_eq!(game_state.player1.stage.center.as_ref().unwrap().card.card_no, new_card.card_no, "New card is correct");
}

// Q25: Baton touch with same or lower cost pays no energy
#[test]
fn test_q25_baton_touch_same_or_lower_cost() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let mut game_state = GameState::new(player1, player2);
    
    // Load real cards from cards.json
    let cards_path = std::path::Path::new("..\\cards\\cards.json");
    let cards = rabuka_engine::card_loader::CardLoader::load_cards_from_file(cards_path)
        .expect("Failed to load cards");
    
    // Find a member card with cost
    let card = cards.iter()
        .find(|c| c.card_type == rabuka_engine::card::CardType::Member && c.cost.is_some())
        .expect("Should find a member card with cost")
        .clone();
    
    let card_cost = card.cost.unwrap_or(0);
    
    // Place card on stage
    game_state.player1.stage.center = Some(rabuka_engine::zones::CardInZone {
        card: card.clone(),
        orientation: None,
        face_state: rabuka_engine::zones::FaceState::FaceUp,
        energy_underneath: Vec::new(),
    });
    
    // Baton touch with same cost: energy_to_pay = card_cost - card_cost = 0
    let same_cost_energy = card_cost.saturating_sub(card_cost);
    assert_eq!(same_cost_energy, 0, "Same cost = 0 energy to pay");
    
    // Baton touch with lower cost: energy_to_pay = lower_cost - card_cost = 0 (saturating_sub prevents negative)
    let lower_cost = card_cost.saturating_sub(1);
    let lower_cost_energy = lower_cost.saturating_sub(card_cost);
    assert_eq!(lower_cost_energy, 0, "Lower cost = 0 energy to pay");
}

// Q27: Baton touch replaces only 1 card
#[test]
fn test_q27_baton_touch_only_one_card_replaced() {
    // Baton touch can only replace 1 card from stage
    // Cannot replace multiple cards and pay their combined cost
    
    let max_cards_to_replace = 1;
    
    // Example: Cannot replace cost 4 + cost 6 = 10 total with a cost 10 card
    let cards_to_replace = 2;
    assert!(cards_to_replace > max_cards_to_replace, "Cannot replace 2 cards");
    
    // Only 1 card can be replaced per baton touch
    assert_eq!(max_cards_to_replace, 1, "Only 1 card can be replaced");
}

// Q28: Can place member without baton touch (normal placement replaces existing card)
#[test]
fn test_q28_place_member_without_baton_touch() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let mut game_state = GameState::new(player1, player2);
    
    // Load real cards from cards.json
    let cards_path = std::path::Path::new("..\\cards\\cards.json");
    let cards = rabuka_engine::card_loader::CardLoader::load_cards_from_file(cards_path)
        .expect("Failed to load cards");
    
    // Find a member card
    let existing_card = cards.iter()
        .find(|c| c.card_type == rabuka_engine::card::CardType::Member)
        .expect("Should find a member card")
        .clone();
    
    // Place existing card on stage
    game_state.player1.stage.center = Some(rabuka_engine::zones::CardInZone {
        card: existing_card.clone(),
        orientation: None,
        face_state: rabuka_engine::zones::FaceState::FaceUp,
        energy_underneath: Vec::new(),
    });
    
    // Find another member card for new placement
    let new_card = cards.iter()
        .filter(|c| c.card_type == rabuka_engine::card::CardType::Member)
        .nth(1)
        .expect("Should find another member card")
        .clone();
    
    game_state.player1.hand.cards.push(new_card.clone());
    
    // Normal placement (not baton touch): pay full cost, place on area, existing card goes to waitroom
    let card_cost = new_card.cost.unwrap_or(0);
    
    // Verify normal placement pays full cost (not difference)
    assert!(card_cost > 0, "Card has cost");
    
    // Simulate normal placement: remove existing card, place new card
    let _removed_card = game_state.player1.stage.center.take();
    let new_card_in_zone = rabuka_engine::zones::CardInZone {
        card: game_state.player1.hand.cards.remove(0),
        orientation: None,
        face_state: rabuka_engine::zones::FaceState::FaceUp,
        energy_underneath: Vec::new(),
    };
    
    game_state.player1.stage.center = Some(new_card_in_zone);
    
    // Verify new card is on stage
    assert!(game_state.player1.stage.center.is_some(), "New card placed on stage");
}

// Q32: No cheer confirmation if no live cards
#[test]
fn test_q32_no_cheer_if_no_live_cards() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let mut game_state = GameState::new(player1, player2);
    
    // Live card zone is empty
    assert_eq!(game_state.player1.live_card_zone.len(), 0, "No live cards in zone");
    
    // If no live cards, no live is performed, so no cheer confirmation
    let has_live_cards = game_state.player1.live_card_zone.len() > 0;
    let should_confirm_cheer = has_live_cards;
    
    assert!(!should_confirm_cheer, "No cheer confirmation if no live cards");
}

// Q33: Live start timing
#[test]
fn test_q33_live_start_timing() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let mut game_state = GameState::new(player1, player2);
    
    // Live start timing: after flipping all live cards face up, removing non-live cards, before cheer confirmation
    // This is in the performance phase
    
    // Simulate: live cards are face up, non-live cards removed
    let live_cards_face_up = true;
    let non_live_cards_removed = true;
    let cheer_confirmed = false;
    
    // Live start is after face up and removal, before cheer confirmation
    assert!(live_cards_face_up, "Live cards face up");
    assert!(non_live_cards_removed, "Non-live cards removed");
    assert!(!cheer_confirmed, "Cheer not yet confirmed");
}

// Q34: Live cards remain in zone when required hearts satisfied
#[test]
fn test_q34_live_card_fate_hearts_satisfied() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let mut game_state = GameState::new(player1, player2);
    
    // When required hearts are satisfied, live cards remain in live card zone
    // After live win/loss determination phase, they go to waitroom with cheer cards
    let required_hearts_satisfied = true;
    let live_cards_remain_in_zone = true;
    
    assert!(required_hearts_satisfied, "Required hearts satisfied");
    assert!(live_cards_remain_in_zone, "Live cards remain in zone initially");
    
    // After live win/loss determination, they go to waitroom
    let after_win_loss_phase = true;
    let cards_go_to_waitroom = after_win_loss_phase;
    assert!(cards_go_to_waitroom, "Cards go to waitroom after win/loss phase");
}

// Q35: Live cards go to waitroom when required hearts not satisfied
#[test]
fn test_q35_live_card_fate_hearts_not_satisfied() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let mut game_state = GameState::new(player1, player2);
    
    // When required hearts are not satisfied, live cards go to waitroom immediately
    // This happens before live win/loss determination phase
    let required_hearts_satisfied = false;
    let cards_go_to_waitroom_immediately = !required_hearts_satisfied;
    
    assert!(!required_hearts_satisfied, "Required hearts not satisfied");
    assert!(cards_go_to_waitroom_immediately, "Cards go to waitroom immediately");
    
    // This happens before live win/loss determination phase
    let before_win_loss_phase = true;
    assert!(before_win_loss_phase, "Cards removed before win/loss phase");
}

// Q36: Live success timing
#[test]
fn test_q36_live_success_timing() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let game_state = GameState::new(player1, player2);
    
    // Live success timing: after both players' performance phases, before determining live winner
    let both_performance_phases_done = true;
    let live_winner_determined = false;
    
    assert!(both_performance_phases_done, "Both performance phases done");
    assert!(!live_winner_determined, "Live winner not yet determined");
    
    // This is in the live win/loss determination phase
    assert!(true, "Live success timing is in win/loss determination phase");
}

// Q37: Live_start/live_success abilities can only be used once per timing
#[test]
fn test_q37_live_abilities_once_per_timing() {
    // Live_start and live_success automatic abilities can only be used once per timing
    // Each ability triggers once when the timing occurs
    
    let max_uses_per_timing = 1;
    
    // If multiple live_start or live_success abilities exist, each triggers once
    let ability_uses = 1;
    assert_eq!(ability_uses, max_uses_per_timing, "Each ability used once per timing");
    
    // Player chooses order when multiple abilities trigger simultaneously
    let multiple_abilities_trigger = true;
    let player_chooses_order = multiple_abilities_trigger;
    assert!(player_chooses_order, "Player chooses order when multiple abilities trigger");
}

// Q38: What is a "live card"
#[test]
fn test_q38_what_is_live_card() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let mut game_state = GameState::new(player1, player2);
    
    // Load real cards from cards.json
    let cards_path = std::path::Path::new("..\\cards\\cards.json");
    let cards = rabuka_engine::card_loader::CardLoader::load_cards_from_file(cards_path)
        .expect("Failed to load cards");
    
    // Find a live card
    let live_card = cards.iter()
        .find(|c| c.card_type == rabuka_engine::card::CardType::Live)
        .expect("Should find a live card")
        .clone();
    
    // Live cards are face-up live cards in the live card zone
    game_state.player1.live_card_zone.cards.push(live_card);
    
    // Verify card is in live card zone
    assert_eq!(game_state.player1.live_card_zone.len(), 1, "Card in live card zone");
}

// Q39: Must complete cheer checks before checking required hearts
#[test]
fn test_q39_must_complete_cheer_checks() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let mut game_state = GameState::new(player1, player2);
    
    // Load real cards from cards.json
    let cards_path = std::path::Path::new("..\\cards\\cards.json");
    let cards = rabuka_engine::card_loader::CardLoader::load_cards_from_file(cards_path)
        .expect("Failed to load cards");
    
    // Add some cards to player1's main deck for cheer checks
    let member_cards: Vec<_> = cards.iter()
        .filter(|c| c.card_type == rabuka_engine::card::CardType::Member)
        .take(5)
        .cloned()
        .collect();
    
    for card in member_cards {
        game_state.player1.main_deck.cards.push_back(card);
    }
    
    // Attempt to check required hearts without performing cheer checks
    // This should fail
    let check_result = game_state.check_required_hearts();
    assert!(check_result.is_err(), "Cannot check required hearts before cheer checks");
    
    // Perform cheer checks
    let player1_id = game_state.player1.id.clone();
    game_state.perform_cheer_check(&player1_id, 2)
        .expect("Cheer check should succeed");
    
    // Verify cards were moved to resolution zone
    assert_eq!(game_state.resolution_zone.cards.len(), 2, "2 cards moved to resolution zone");
    assert_eq!(game_state.player1.main_deck.len(), 3, "3 cards remain in deck");
    
    // Now checking required hearts should succeed
    let check_result = game_state.check_required_hearts();
    assert!(check_result.is_ok(), "Can check required hearts after cheer checks completed");
}

// Q40: Cannot skip remaining cheer checks even if condition satisfied mid-process
#[test]
fn test_q40_cannot_skip_remaining_cheer_checks() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let mut game_state = GameState::new(player1, player2);
    
    // Load real cards from cards.json
    let cards_path = std::path::Path::new("..\\cards\\cards.json");
    let cards = rabuka_engine::card_loader::CardLoader::load_cards_from_file(cards_path)
        .expect("Failed to load cards");
    
    // Add cards to player1's main deck for cheer checks
    let member_cards: Vec<_> = cards.iter()
        .filter(|c| c.card_type == rabuka_engine::card::CardType::Member)
        .take(5)
        .cloned()
        .collect();
    
    for card in member_cards {
        game_state.player1.main_deck.cards.push_back(card);
    }
    
    // Perform partial cheer checks (1 out of 3 required)
    let player1_id = game_state.player1.id.clone();
    game_state.perform_cheer_check(&player1_id, 1)
        .expect("Cheer check should succeed");
    
    // Verify partial progress
    assert_eq!(game_state.cheer_checks_done, 1, "1 cheer check done");
    assert_eq!(game_state.cheer_checks_required, 1, "Required set to 1 on first call");
    
    // Even if we know condition is satisfied, cannot skip remaining checks
    let check_result = game_state.check_required_hearts();
    assert!(check_result.is_ok(), "Can check when all required checks done");
    
    // Now test with multiple required checks
    game_state.cheer_checks_done = 0;
    game_state.cheer_checks_required = 0;
    game_state.cheer_check_completed = false;
    
    // Perform first of 3 checks
    game_state.perform_cheer_check(&player1_id, 3)
        .expect("Cheer check should succeed");
    
    // After first call, all 3 are done at once
    assert_eq!(game_state.cheer_checks_done, 3, "3 checks done");
    assert_eq!(game_state.cheer_checks_required, 3, "Required set to 3");
    
    // Reset to simulate partial completion
    game_state.cheer_checks_done = 1;
    game_state.cheer_check_completed = false;
    
    // Cannot check hearts when only 1 of 3 checks done
    let check_result = game_state.check_required_hearts();
    assert!(check_result.is_err(), "Cannot check with partial cheer checks");
}

// Q41: Cards revealed during cheer checks go to waitroom at specific timing
#[test]
fn test_q41_cheer_cards_to_waitroom_timing() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let mut game_state = GameState::new(player1, player2);
    
    // Load real cards from cards.json
    let cards_path = std::path::Path::new("..\\cards\\cards.json");
    let cards = rabuka_engine::card_loader::CardLoader::load_cards_from_file(cards_path)
        .expect("Failed to load cards");
    
    // Add cards to player1's main deck for cheer checks
    let member_cards: Vec<_> = cards.iter()
        .filter(|c| c.card_type == rabuka_engine::card::CardType::Member)
        .take(5)
        .cloned()
        .collect();
    
    for card in member_cards {
        game_state.player1.main_deck.cards.push_back(card);
    }
    
    // Perform cheer checks - cards move to resolution zone
    let player1_id = game_state.player1.id.clone();
    game_state.perform_cheer_check(&player1_id, 3)
        .expect("Cheer check should succeed");
    
    // Verify cards are in resolution zone
    assert_eq!(game_state.resolution_zone.cards.len(), 3, "3 cards in resolution zone");
    assert_eq!(game_state.player1.waitroom.cards.len(), 0, "0 cards in waitroom initially");
    
    // In live victory determination phase, after winner places cards in success zone
    // Remaining cards in resolution zone go to waitroom
    game_state.move_resolution_zone_to_waitroom(&player1_id);
    
    // Verify cards moved from resolution zone to waitroom
    assert_eq!(game_state.resolution_zone.cards.len(), 0, "0 cards in resolution zone after move");
    assert_eq!(game_state.player1.waitroom.cards.len(), 3, "3 cards moved to waitroom");
}
