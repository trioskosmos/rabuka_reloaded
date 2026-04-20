mod card;
mod zones;
mod player;
mod game_state;
mod turn;
mod card_loader;
mod deck_builder;
mod deck_parser;
mod web_server;

use player::Player;
use game_state::GameState;
use std::io::{self, BufRead};
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};
use serde_json;

#[derive(Serialize, Deserialize, Clone)]
pub struct Action {
    pub description: String,
    pub action_type: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ActionsResponse {
    pub actions: Vec<Action>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CardDisplay {
    pub card_no: String,
    pub name: String,
    #[serde(rename = "type")]
    pub card_type: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ZoneDisplay {
    pub cards: Vec<CardDisplay>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlayerDisplay {
    pub hand: ZoneDisplay,
    pub energy: ZoneDisplay,
    pub stage: StageDisplay,
    pub live_zone: ZoneDisplay,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StageDisplay {
    pub left_side: Option<CardDisplay>,
    pub center: Option<CardDisplay>,
    pub right_side: Option<CardDisplay>,
}

#[derive(Serialize, Deserialize)]
pub struct GameStateDisplay {
    pub turn: u32,
    pub phase: String,
    pub player1: PlayerDisplay,
    pub player2: PlayerDisplay,
}

pub fn card_to_display(card: &crate::card::Card) -> CardDisplay {
    CardDisplay {
        card_no: card.card_no.clone(),
        name: card.name.clone(),
        card_type: format!("{:?}", card.card_type),
    }
}

pub fn zone_to_display(cards: &[crate::card::Card]) -> ZoneDisplay {
    ZoneDisplay {
        cards: cards.iter().map(card_to_display).collect(),
    }
}

pub fn stage_to_display(stage: &crate::zones::Stage) -> StageDisplay {
    StageDisplay {
        left_side: stage.left_side.as_ref().map(|c| card_to_display(&c.card)),
        center: stage.center.as_ref().map(|c| card_to_display(&c.card)),
        right_side: stage.right_side.as_ref().map(|c| card_to_display(&c.card)),
    }
}

pub fn player_to_display(player: &crate::player::Player) -> PlayerDisplay {
    let energy_cards: Vec<crate::card::Card> = player.energy_zone.cards.iter().map(|c| c.card.clone()).collect();
    PlayerDisplay {
        hand: zone_to_display(&player.hand.cards),
        energy: zone_to_display(&energy_cards),
        stage: stage_to_display(&player.stage),
        live_zone: zone_to_display(&player.live_card_zone.cards),
    }
}

pub fn game_state_to_display(game_state: &GameState) -> GameStateDisplay {
    GameStateDisplay {
        turn: game_state.turn_number,
        phase: format!("{:?} - {:?}", game_state.current_turn_phase, game_state.current_phase),
        player1: player_to_display(&game_state.player1),
        player2: player_to_display(&game_state.player2),
    }
}

// Global game state for CLI commands
static GAME_STATE: Mutex<Option<GameState>> = Mutex::new(None);

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() > 1 {
        match args[1].as_str() {
            "get-state" => {
                output_game_state();
            }
            "get-actions" => {
                output_actions();
            }
            "execute-action" => {
                if args.len() > 2 {
                    if let Ok(index) = args[2].parse::<usize>() {
                        execute_action(index);
                    }
                }
            }
            "init" => {
                initialize_game();
            }
            "test-legal-actions" => {
                test_legal_actions();
            }
            "web-server" => {
                run_web_server();
            }
            _ => {
                eprintln!("Unknown command: {}", args[1]);
            }
        }
    } else {
        // Default: run full game initialization
        run_game();
    }
}

fn run_game() {
    println!("Love Live! Card Game Engine");
    
    // Load cards from cards.json
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = match card_loader::CardLoader::load_cards_from_file(cards_path) {
        Ok(cards) => {
            println!("Loaded {} cards", cards.len());
            // Convert Vec<Card> to HashMap<String, Card>
            let mut card_map = std::collections::HashMap::new();
            for card in cards {
                card_map.insert(card.card_no.clone(), card);
            }
            card_map
        }
        Err(e) => {
            eprintln!("Failed to load cards: {}", e);
            return;
        }
    };
    
    // Load sample decks from game/decks
    let deck_lists = match deck_parser::DeckParser::parse_all_decks() {
        Ok(decks) => {
            println!("Loaded {} sample decks:", decks.len());
            for deck in &decks {
                println!("  - {}", deck.name);
            }
            decks
        }
        Err(e) => {
            eprintln!("Failed to load decks: {}", e);
            return;
        }
    };
    
    // Let players choose decks
    let deck1 = choose_deck(&deck_lists, "Player 1");
    let deck2 = choose_deck(&deck_lists, "Player 2");
    
    // Build decks from chosen deck lists
    let card_numbers1 = deck_parser::DeckParser::deck_list_to_card_numbers(&deck1);
    let card_numbers2 = deck_parser::DeckParser::deck_list_to_card_numbers(&deck2);
    
    let mut player1_deck = match deck_builder::DeckBuilder::build_deck_from_card_map(&cards, card_numbers1) {
        Ok(mut deck) => {
            deck.shuffle_main_deck();
            deck.shuffle_energy_deck();
            deck
        }
        Err(e) => {
            eprintln!("Failed to build deck for Player 1: {}", e);
            return;
        }
    };
    
    let mut player2_deck = match deck_builder::DeckBuilder::build_deck_from_card_map(&cards, card_numbers2) {
        Ok(mut deck) => {
            deck.shuffle_main_deck();
            deck.shuffle_energy_deck();
            deck
        }
        Err(e) => {
            eprintln!("Failed to build deck for Player 2: {}", e);
            return;
        }
    };
    
    // Add default energy cards if needed
    let _ = deck_builder::DeckBuilder::add_default_energy_cards(&mut player1_deck, &cards);
    let _ = deck_builder::DeckBuilder::add_default_energy_cards(&mut player2_deck, &cards);
    
    println!("Player 1 deck: {}", deck1.name);
    println!("  Main deck: {} cards", player1_deck.main_deck.len());
    println!("  Energy deck: {} cards", player1_deck.energy_deck.len());
    
    println!("Player 2 deck: {}", deck2.name);
    println!("  Main deck: {} cards", player2_deck.main_deck.len());
    println!("  Energy deck: {} cards", player2_deck.energy_deck.len());
    
    // Initialize players with decks
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    player1.set_main_deck(player1_deck.main_deck);
    player1.set_energy_deck(player1_deck.energy_deck);
    
    player2.set_main_deck(player2_deck.main_deck);
    player2.set_energy_deck(player2_deck.energy_deck);
    
    // Initialize game state
    let mut game_state = GameState::new(player1, player2);
    
    println!("Game initialized");
    println!("Turn: {}", game_state.turn_number);
    println!("Phase: {:?}", game_state.current_phase);
    
    // Game setup (Rule 6.2)
    setup_game(&mut game_state);
    
    println!("Game setup complete");
    println!("Player 1 hand: {} cards", game_state.player1.hand.len());
    println!("Player 2 hand: {} cards", game_state.player2.hand.len());
    println!("Player 1 energy: {} cards", game_state.player1.energy_zone.cards.len());
    println!("Player 2 energy: {} cards", game_state.player2.energy_zone.cards.len());
}

fn output_game_state() {
    let game_state = GAME_STATE.lock().unwrap();
    if let Some(ref state) = *game_state {
        let display = game_state_to_display(state);
        println!("{}", serde_json::to_string(&display).unwrap());
    } else {
        eprintln!("Game not initialized. Run 'init' command first.");
    }
}

fn initialize_game() {
    // Load cards from cards.json
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = match card_loader::CardLoader::load_cards_from_file(cards_path) {
        Ok(cards) => {
            // Convert Vec<Card> to HashMap<String, Card>
            let mut card_map = std::collections::HashMap::new();
            for card in cards {
                card_map.insert(card.card_no.clone(), card);
            }
            card_map
        }
        Err(e) => {
            eprintln!("Failed to load cards: {}", e);
            return;
        }
    };
    
    // Load sample decks from game/decks
    let deck_lists = match deck_parser::DeckParser::parse_all_decks() {
        Ok(decks) => decks,
        Err(e) => {
            eprintln!("Failed to load decks: {}", e);
            return;
        }
    };
    
    // Let players choose decks
    let deck1 = choose_deck(&deck_lists, "Player 1");
    let deck2 = choose_deck(&deck_lists, "Player 2");
    
    // Build decks from chosen deck lists
    let card_numbers1 = deck_parser::DeckParser::deck_list_to_card_numbers(&deck1);
    let card_numbers2 = deck_parser::DeckParser::deck_list_to_card_numbers(&deck2);
    
    let mut player1_deck = match deck_builder::DeckBuilder::build_deck_from_card_map(&cards, card_numbers1) {
        Ok(mut deck) => {
            deck.shuffle_main_deck();
            deck.shuffle_energy_deck();
            deck
        }
        Err(e) => {
            eprintln!("Failed to build deck for Player 1: {}", e);
            return;
        }
    };
    
    let mut player2_deck = match deck_builder::DeckBuilder::build_deck_from_card_map(&cards, card_numbers2) {
        Ok(mut deck) => {
            deck.shuffle_main_deck();
            deck.shuffle_energy_deck();
            deck
        }
        Err(e) => {
            eprintln!("Failed to build deck for Player 2: {}", e);
            return;
        }
    };
    
    // Add default energy cards if needed
    let _ = deck_builder::DeckBuilder::add_default_energy_cards(&mut player1_deck, &cards);
    let _ = deck_builder::DeckBuilder::add_default_energy_cards(&mut player2_deck, &cards);
    
    // Initialize players with decks
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    player1.set_main_deck(player1_deck.main_deck);
    player1.set_energy_deck(player1_deck.energy_deck);
    
    player2.set_main_deck(player2_deck.main_deck);
    player2.set_energy_deck(player2_deck.energy_deck);
    
    // Initialize game state
    let mut game_state = GameState::new(player1, player2);
    
    // Game setup (Rule 6.2)
    setup_game(&mut game_state);
    
    // Store in global state
    *GAME_STATE.lock().unwrap() = Some(game_state);
    
    println!("Game initialized successfully");
}

fn output_actions() {
    let game_state = GAME_STATE.lock().unwrap();
    if let Some(ref state) = *game_state {
        let actions = generate_possible_actions(state);
        let response = ActionsResponse { actions };
        println!("{}", serde_json::to_string(&response).unwrap());
    } else {
        eprintln!("Game not initialized. Run 'init' command first.");
    }
}

fn execute_action(index: usize) {
    let mut game_state = GAME_STATE.lock().unwrap();
    if let Some(ref mut state) = *game_state {
        let actions = generate_possible_actions(state);
        
        if index >= actions.len() {
            eprintln!("Invalid action index");
            return;
        }
        
        let action = &actions[index];
        println!("Executing action: {}", action.description);
        
        // Execute the action (simplified - in real implementation would call execute_action from web_server)
        match action.action_type.as_str() {
            "activate_energy" => {
                state.player1.activate_all_energy();
            }
            "draw_card" => {
                state.player1.draw_card();
            }
            _ => {
                eprintln!("Action execution not implemented for: {}", action.action_type);
            }
        }
        
        println!("Action executed successfully");
    } else {
        eprintln!("Game not initialized. Run 'init' command first.");
    }
}

fn generate_possible_actions(game_state: &GameState) -> Vec<Action> {
    let mut actions = Vec::new();
    let active_player = game_state.active_player();
    
    // Filter actions based on current phase and legal action validation
    match game_state.current_phase {
        crate::game_state::Phase::Active => {
            // Rule 7.1: Active Phase - Can activate energy cards
            if active_player.can_activate_energy() {
                actions.push(Action {
                    description: "Activate all energy cards".to_string(),
                    action_type: "activate_energy".to_string(),
                });
            }
        }
        crate::game_state::Phase::Energy => {
            // Rule 7.2: Energy Phase - Can play energy cards from hand
            if active_player.can_play_energy_to_zone() {
                actions.push(Action {
                    description: "Play energy card from hand to energy zone".to_string(),
                    action_type: "play_energy_to_zone".to_string(),
                });
            }
        }
        crate::game_state::Phase::Draw => {
            // Rule 8.1: Draw Phase - Can draw card
            if active_player.can_draw_card() {
                actions.push(Action {
                    description: "Draw card from main deck".to_string(),
                    action_type: "draw_card".to_string(),
                });
            }
        }
        crate::game_state::Phase::Main => {
            // Rule 8.2: Main Phase - Can play member cards to stage
            if active_player.can_play_member_to_stage() {
                actions.push(Action {
                    description: "Play member card from hand to stage".to_string(),
                    action_type: "play_member_to_stage".to_string(),
                });
            }
            
            // Rule 5.5: Can shuffle deck if not empty
            if active_player.can_shuffle_zone("deck") {
                actions.push(Action {
                    description: "Shuffle main deck".to_string(),
                    action_type: "shuffle_deck".to_string(),
                });
            }
            
            // Rule 5.7: Can look at top cards
            if active_player.can_look_at_top(1) {
                actions.push(Action {
                    description: "Look at top card of main deck".to_string(),
                    action_type: "look_at_top".to_string(),
                });
            }
            
            // Rule 5.8: Can swap cards between areas
            if active_player.can_swap_cards(
                crate::zones::MemberArea::LeftSide,
                crate::zones::MemberArea::Center,
            ) {
                actions.push(Action {
                    description: "Swap left side and center members".to_string(),
                    action_type: "swap_left_center".to_string(),
                });
            }
            if active_player.can_swap_cards(
                crate::zones::MemberArea::Center,
                crate::zones::MemberArea::RightSide,
            ) {
                actions.push(Action {
                    description: "Swap center and right side members".to_string(),
                    action_type: "swap_center_right".to_string(),
                });
            }
            if active_player.can_swap_cards(
                crate::zones::MemberArea::LeftSide,
                crate::zones::MemberArea::RightSide,
            ) {
                actions.push(Action {
                    description: "Swap left side and right side members".to_string(),
                    action_type: "swap_left_right".to_string(),
                });
            }
            
            // Rule 5.9: Can pay energy
            if active_player.can_pay_energy(1) {
                actions.push(Action {
                    description: "Pay 1 energy".to_string(),
                    action_type: "pay_energy_1".to_string(),
                });
            }
            if active_player.can_pay_energy(2) {
                actions.push(Action {
                    description: "Pay 2 energy".to_string(),
                    action_type: "pay_energy_2".to_string(),
                });
            }
            
            // Rule 5.10: Can place energy under member
            if active_player.can_place_energy_under_member(crate::zones::MemberArea::LeftSide) {
                actions.push(Action {
                    description: "Place energy under left side member".to_string(),
                    action_type: "place_energy_under_left".to_string(),
                });
            }
            if active_player.can_place_energy_under_member(crate::zones::MemberArea::Center) {
                actions.push(Action {
                    description: "Place energy under center member".to_string(),
                    action_type: "place_energy_under_center".to_string(),
                });
            }
            if active_player.can_place_energy_under_member(crate::zones::MemberArea::RightSide) {
                actions.push(Action {
                    description: "Place energy under right side member".to_string(),
                    action_type: "place_energy_under_right".to_string(),
                });
            }
        }
        crate::game_state::Phase::LiveCardSet => {
            // Rule 9.1: Live Card Set Phase - Can place cards in live zone
            if active_player.can_place_in_live_zone() {
                actions.push(Action {
                    description: "Place card in Live Card Zone".to_string(),
                    action_type: "place_in_live_zone".to_string(),
                });
            }
        }
        crate::game_state::Phase::FirstAttackerPerformance
        | crate::game_state::Phase::SecondAttackerPerformance
        | crate::game_state::Phase::LiveVictoryDetermination => {
            // Live phase actions - currently no specific actions
        }
    }
    
    actions
}

fn choose_deck(deck_lists: &[deck_parser::DeckList], player_name: &str) -> deck_parser::DeckList {
    // For now, just pick the first deck
    // TODO: Implement actual player choice
    println!("{} chose: {}", player_name, deck_lists[0].name);
    deck_lists[0].clone()
}

fn setup_game(game_state: &mut GameState) {
    // Rule 6.2: Pre-Game Procedure
    
    // Rule 6.2.2: Rock Paper Scissors to determine who chooses to go first
    let rps_winner = play_rock_paper_scissors();
    // Winner chooses to be first or second attacker
    // For now, default: winner of RPS becomes first attacker
    if rps_winner == 1 {
        game_state.player1.is_first_attacker = true;
        game_state.player2.is_first_attacker = false;
    } else {
        game_state.player1.is_first_attacker = false;
        game_state.player2.is_first_attacker = true;
    }
    
    // Rule 6.2.5: Initial draw - Each player draws 6 cards from main deck to hand
    for _ in 0..6 {
        game_state.player1.draw_card();
        game_state.player2.draw_card();
    }
    
    // Rule 6.2.6: Mulligan - Players may return cards and draw new cards
    // Rule 6.2.1.6: Starting from first attacker, each player chooses cards to mulligan
    // Simplified: For now, skip mulligan - assume players keep their hand
    // TODO: Implement full mulligan with player choice
    
    // Rule 6.2.7: Initial energy - Each player draws 3 cards from energy deck to Energy Zone
    for _ in 0..3 {
        game_state.player1.draw_energy();
        game_state.player2.draw_energy();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RPSChoice {
    Rock,
    Paper,
    Scissors,
}

fn play_rock_paper_scissors() -> u8 {
    // Rule 6.2.2: Rock Paper Scissors to determine first attacker
    // Returns 1 if player 1 wins, 2 if player 2 wins
    use rand::Rng;
    let mut rng = rand::thread_rng();
    
    let choices = [RPSChoice::Rock, RPSChoice::Paper, RPSChoice::Scissors];
    
    let p1_choice = choices[rng.gen_range(0..3)];
    let p2_choice = choices[rng.gen_range(0..3)];
    
    match (p1_choice, p2_choice) {
        (RPSChoice::Rock, RPSChoice::Scissors) => 1,
        (RPSChoice::Paper, RPSChoice::Rock) => 1,
        (RPSChoice::Scissors, RPSChoice::Paper) => 1,
        (RPSChoice::Scissors, RPSChoice::Rock) => 2,
        (RPSChoice::Rock, RPSChoice::Paper) => 2,
        (RPSChoice::Paper, RPSChoice::Scissors) => 2,
        _ => {
            // Tie - play again
            play_rock_paper_scissors()
        }
    }
}

fn test_legal_actions() {
    println!("=== Testing Legal Action Filtering ===\n");
    
    // Load cards
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = match card_loader::CardLoader::load_cards_from_file(cards_path) {
        Ok(cards) => {
            let mut card_map = std::collections::HashMap::new();
            for card in cards {
                card_map.insert(card.card_no.clone(), card);
            }
            card_map
        }
        Err(e) => {
            eprintln!("Failed to load cards: {}", e);
            return;
        }
    };
    
    // Load decks
    let deck_lists = match deck_parser::DeckParser::parse_all_decks() {
        Ok(decks) => decks,
        Err(e) => {
            eprintln!("Failed to load decks: {}", e);
            return;
        }
    };
    
    // Use first deck for both players
    let deck1 = &deck_lists[0];
    let deck2 = &deck_lists[0];
    
    let card_numbers1 = deck_parser::DeckParser::deck_list_to_card_numbers(deck1);
    let card_numbers2 = deck_parser::DeckParser::deck_list_to_card_numbers(deck2);
    
    let mut player1_deck = match deck_builder::DeckBuilder::build_deck_from_card_map(&cards, card_numbers1) {
        Ok(mut deck) => {
            deck.shuffle_main_deck();
            deck.shuffle_energy_deck();
            deck
        }
        Err(e) => {
            eprintln!("Failed to build deck for Player 1: {}", e);
            return;
        }
    };
    
    let mut player2_deck = match deck_builder::DeckBuilder::build_deck_from_card_map(&cards, card_numbers2) {
        Ok(mut deck) => {
            deck.shuffle_main_deck();
            deck.shuffle_energy_deck();
            deck
        }
        Err(e) => {
            eprintln!("Failed to build deck for Player 2: {}", e);
            return;
        }
    };
    
    let _ = deck_builder::DeckBuilder::add_default_energy_cards(&mut player1_deck, &cards);
    let _ = deck_builder::DeckBuilder::add_default_energy_cards(&mut player2_deck, &cards);
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    player1.set_main_deck(player1_deck.main_deck);
    player1.set_energy_deck(player1_deck.energy_deck);
    
    player2.set_main_deck(player2_deck.main_deck);
    player2.set_energy_deck(player2_deck.energy_deck);
    
    let mut game_state = GameState::new(player1, player2);
    setup_game(&mut game_state);
    
    // Test legal actions in different phases
    let test_phases = vec![
        (game_state::Phase::Active, "Active Phase"),
        (game_state::Phase::Energy, "Energy Phase"),
        (game_state::Phase::Draw, "Draw Phase"),
        (game_state::Phase::Main, "Main Phase"),
        (game_state::Phase::LiveCardSet, "Live Card Set Phase"),
    ];
    
    for (phase, phase_name) in test_phases {
        game_state.current_phase = phase.clone();
        let actions = generate_possible_actions(&game_state);
        
        println!("--- {} ({:?}) ---", phase_name, phase);
        println!("Legal actions available: {}", actions.len());
        
        if actions.is_empty() {
            println!("  No legal actions in this phase");
        } else {
            for (i, action) in actions.iter().enumerate() {
                println!("  {}. {}", i + 1, action.description);
            }
        }
        println!();
    }
    
    // Test specific legal action validation methods
    println!("--- Testing Legal Action Validation Methods ---");
    let player = game_state.active_player();
    
    println!("can_activate_energy: {}", player.can_activate_energy());
    println!("can_draw_card: {}", player.can_draw_card());
    println!("can_play_member_to_stage: {}", player.can_play_member_to_stage());
    println!("can_play_energy_to_zone: {}", player.can_play_energy_to_zone());
    println!("can_shuffle_zone(deck): {}", player.can_shuffle_zone("deck"));
    println!("can_look_at_top(1): {}", player.can_look_at_top(1));
    println!("can_pay_energy(1): {}", player.can_pay_energy(1));
    
    println!("\n=== Legal Action Filtering Test Complete ===");
}

fn run_web_server() {
    println!("Starting web server...");
    
    // Load cards and initialize game state
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = match card_loader::CardLoader::load_cards_from_file(cards_path) {
        Ok(cards) => {
            let mut card_map = std::collections::HashMap::new();
            for card in cards {
                card_map.insert(card.card_no.clone(), card);
            }
            card_map
        }
        Err(e) => {
            eprintln!("Failed to load cards: {}", e);
            return;
        }
    };
    
    let deck_lists = match deck_parser::DeckParser::parse_all_decks() {
        Ok(decks) => decks,
        Err(e) => {
            eprintln!("Failed to load decks: {}", e);
            return;
        }
    };
    
    let deck1 = &deck_lists[0];
    let deck2 = &deck_lists[0];
    
    let card_numbers1 = deck_parser::DeckParser::deck_list_to_card_numbers(deck1);
    let card_numbers2 = deck_parser::DeckParser::deck_list_to_card_numbers(deck2);
    
    let mut player1_deck = match deck_builder::DeckBuilder::build_deck_from_card_map(&cards, card_numbers1) {
        Ok(mut deck) => {
            deck.shuffle_main_deck();
            deck.shuffle_energy_deck();
            deck
        }
        Err(e) => {
            eprintln!("Failed to build deck for Player 1: {}", e);
            return;
        }
    };
    
    let mut player2_deck = match deck_builder::DeckBuilder::build_deck_from_card_map(&cards, card_numbers2) {
        Ok(mut deck) => {
            deck.shuffle_main_deck();
            deck.shuffle_energy_deck();
            deck
        }
        Err(e) => {
            eprintln!("Failed to build deck for Player 2: {}", e);
            return;
        }
    };
    
    let _ = deck_builder::DeckBuilder::add_default_energy_cards(&mut player1_deck, &cards);
    let _ = deck_builder::DeckBuilder::add_default_energy_cards(&mut player2_deck, &cards);
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    player1.set_main_deck(player1_deck.main_deck);
    player1.set_energy_deck(player1_deck.energy_deck);
    
    player2.set_main_deck(player2_deck.main_deck);
    player2.set_energy_deck(player2_deck.energy_deck);
    
    let mut game_state = GameState::new(player1, player2);
    setup_game(&mut game_state);
    
    // Start web server with game state
    let game_state = Arc::new(Mutex::new(game_state));
    
    println!("Web server starting on http://127.0.0.1:8080");
    
    // This will block until the server is stopped
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(web_server::run_web_server(game_state));
}
