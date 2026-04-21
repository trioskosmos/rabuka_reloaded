use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rabuka_engine::card::CardDatabase;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameState, Phase};
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use std::sync::Arc;

fn setup_test_game_state() -> GameState {
    let cards = CardDatabase::new();
    let card_db = Arc::new(cards);
    
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let mut game_state = GameState::new(player1, player2, card_db);
    game_state.current_phase = Phase::Main;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    
    // Add some cards to player1's hand for benchmarking
    for i in 0..10 {
        game_state.player1.hand.cards.push(i);
    }
    
    game_state
}

fn benchmark_generate_possible_actions(c: &mut Criterion) {
    let game_state = setup_test_game_state();
    
    c.bench_function("generate_possible_actions", |b| {
        b.iter(|| {
            game_setup::generate_possible_actions(black_box(&game_state));
        });
    });
}

fn benchmark_check_timing(c: &mut Criterion) {
    c.bench_function("check_timing", |b| {
        b.iter(|| {
            let mut game_state = setup_test_game_state();
            TurnEngine::check_timing(black_box(&mut game_state));
        });
    });
}

criterion_group!(
    benches,
    benchmark_generate_possible_actions,
    benchmark_check_timing
);
criterion_main!(benches);
