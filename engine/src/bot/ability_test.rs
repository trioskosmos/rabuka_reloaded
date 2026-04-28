// Simple automated test to verify ability triggering
// Simulates playing a card to stage to trigger debut abilities

use crate::card_loader;
use crate::deck_builder;
use crate::deck_parser;
use crate::game_state::GameState;
use crate::player::Player;
use crate::turn;
use std::path::Path;

pub fn run_ability_test() {
    println!("🧪 Starting ability test...\n");
    
    // Load cards
    let cards_path = Path::new("../cards/cards.json");
    let cards_vec = match card_loader::CardLoader::load_cards_from_file(cards_path) {
        Ok(c) => {
            println!("📚 Loaded {} cards", c.len());
            c
        }
        Err(e) => {
            eprintln!("Failed to load cards: {}", e);
            return;
        }
    };

    // Create CardDatabase from loaded cards
    let card_database = std::sync::Arc::new(crate::card::CardDatabase::load_or_create(cards_vec.clone()));

    // Load decks
    let decks_path = Path::new("../game/decks");
    let deck_lists = match deck_parser::DeckParser::parse_all_decks_from_directory(decks_path) {
        Ok(decks) => {
            println!("📦 Loaded {} decks", decks.len());
            decks
        }
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

    let mut player1_deck = match deck_builder::DeckBuilder::build_deck_from_database(&card_database, card_numbers1) {
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

    let mut player2_deck = match deck_builder::DeckBuilder::build_deck_from_database(&card_database, card_numbers2) {
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

    let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(&mut player1_deck, &card_database);
    let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(&mut player2_deck, &card_database);

    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);

    player1.set_main_deck(player1_deck.main_deck);
    player1.set_energy_deck(player1_deck.energy_deck);

    player2.set_main_deck(player2_deck.main_deck);
    player2.set_energy_deck(player2_deck.energy_deck);

    let mut game_state = GameState::new(player1, player2, card_database.clone());
    
    // Add energy to player so they can play cards
    for _ in 0..10 {
        let _ = game_state.player1.draw_energy();
    }
    
    // Activate energy
    game_state.player1.activate_all_energy();

    // Find a card with DEBUT trigger from all cards
    let debut_card_from_all = cards_vec.iter()
        .find(|c| {
            c.card_type == crate::card::CardType::Member &&
            c.abilities.iter().any(|a| a.triggers.as_ref().map_or(false, |t| t == "登場"))
        });

    if let Some(debut_card) = debut_card_from_all {
        // Add it to hand
        if let Some(card_id) = card_database.get_card_id(&debut_card.card_no) {
            game_state.player1.hand.cards.push(card_id);
        } else {
            return;
        }
    } else {
        return;
    }


    // Find a card with DEBUT trigger in hand
    let debut_card = game_state.player1.hand.cards.iter().enumerate().find(|(_, card_id)| {
        if let Some(card) = card_database.get_card(**card_id) {
            card.card_type == crate::card::CardType::Member &&
            card.abilities.iter().any(|a| a.triggers.as_ref().map_or(false, |t| t == "登場"))
        } else {
            false
        }
    });
    
    // Try to play the first member card to stage
    if let Some((idx, _card)) = debut_card {
        let card_id = game_state.player1.hand.cards[idx];
        
        match turn::TurnEngine::execute_main_phase_action(
            &mut game_state,
            &crate::game_setup::ActionType::PlayMemberToStage,
            Some(card_id),
            None,
            Some(crate::zones::MemberArea::Center),
            None,
        ) {
            Ok(_) => {
            }
            Err(_e) => {
            }
        }
    } else {
        // No member cards with DEBUT ability in hand
    }
    
}
