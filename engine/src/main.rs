mod card;
mod zones;
mod player;
mod game_state;
mod turn;
mod card_loader;
mod deck_builder;
mod deck_parser;

use player::Player;
use game_state::GameState;

fn main() {
    println!("Love Live! Card Game Engine");
    
    // Load cards from cards.json
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = match card_loader::CardLoader::load_cards_from_file(cards_path) {
        Ok(cards) => {
            println!("Loaded {} cards", cards.len());
            cards
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
    
    // TODO: Run game loop
}

fn choose_deck(deck_lists: &[deck_parser::DeckList], player_name: &str) -> deck_parser::DeckList {
    // For now, just pick the first deck
    // TODO: Implement actual player choice
    println!("{} chose: {}", player_name, deck_lists[0].name);
    deck_lists[0].clone()
}

fn setup_game(game_state: &mut GameState) {
    // Rule 6.2: Pre-Game Procedure
    
    // 5. Initial draw: Each player draws 6 cards from main deck to hand
    for _ in 0..6 {
        game_state.player1.draw_card();
        game_state.player2.draw_card();
    }
    
    // 6. Mulligan (simplified - no mulligan for now)
    // TODO: Implement mulligan logic
    
    // 7. Initial energy: Each player draws 3 cards from energy deck to Energy Zone
    for _ in 0..3 {
        game_state.player1.draw_energy();
        game_state.player2.draw_energy();
    }
}
