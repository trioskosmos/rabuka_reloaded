use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader;
use rabuka_engine::deck_builder;
use rabuka_engine::deck_parser;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use std::sync::{Arc, OnceLock};

struct BenchContext {
    card_database: Arc<CardDatabase>,
    deck_templates: Vec<(String, deck_builder::Deck, deck_builder::Deck)>,
}

static CTX: OnceLock<BenchContext> = OnceLock::new();

fn ctx() -> &'static BenchContext {
    CTX.get().expect("BenchContext not initialized")
}

fn init_ctx() {
    let _ = CTX.get_or_init(|| {
        let cards_path = std::path::Path::new("../cards/cards.json");
        let cards =
            card_loader::CardLoader::load_cards_from_file(cards_path).expect("Failed to load cards");
        let card_database = Arc::new(CardDatabase::load_or_create(cards));

        let deck_lists =
            deck_parser::DeckParser::parse_all_decks().expect("Failed to load decks");

        let mut deck_templates = Vec::new();
        for deck_list in &deck_lists {
            let card_numbers = deck_parser::DeckParser::deck_list_to_card_numbers(deck_list);
            let mut p1 = deck_builder::DeckBuilder::build_deck_from_database(
                &mut Arc::clone(&card_database),
                card_numbers.clone(),
            )
            .expect("Failed to build P1 deck");
            let mut p2 = deck_builder::DeckBuilder::build_deck_from_database(
                &mut Arc::clone(&card_database),
                card_numbers,
            )
            .expect("Failed to build P2 deck");
            let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(
                &mut p1,
                &mut Arc::clone(&card_database),
            );
            let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(
                &mut p2,
                &mut Arc::clone(&card_database),
            );
            deck_templates.push((deck_list.name.clone(), p1, p2));
        }

        BenchContext {
            card_database,
            deck_templates,
        }
    });
}

fn build_game(deck_idx: usize) -> GameState {
    let c = ctx();
    let (_, p1_template, p2_template) = &c.deck_templates[deck_idx];
    let mut p1_deck = p1_template.clone();
    let mut p2_deck = p2_template.clone();
    p1_deck.shuffle_main_deck();
    p1_deck.shuffle_energy_deck();
    p2_deck.shuffle_main_deck();
    p2_deck.shuffle_energy_deck();

    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    player1.set_main_deck(p1_deck.main_deck);
    player1.set_energy_deck(p1_deck.energy_deck);
    player2.set_main_deck(p2_deck.main_deck);
    player2.set_energy_deck(p2_deck.energy_deck);

    let mut gs = GameState::new(player1, player2, Arc::clone(&c.card_database));
    game_setup::setup_game(&mut gs);
    gs
}

fn parse_stage_area(s: &str) -> Option<MemberArea> {
    match s {
        "left" => Some(MemberArea::LeftSide),
        "center" => Some(MemberArea::Center),
        "right" => Some(MemberArea::RightSide),
        _ => None,
    }
}

fn run_game_to_completion(gs: &mut GameState) -> u64 {
    let mut actions = 0u64;
    let mut last_turn = 0u32;
    let mut stuck = 0u32;

    for _ in 0..2000 {
        TurnEngine::check_victory_condition(gs);
        if gs.game_result != GameResult::Ongoing {
            break;
        }
        if gs.turn_number == last_turn {
            stuck += 1;
            if stuck > 300 {
                break;
            }
        } else {
            stuck = 0;
            last_turn = gs.turn_number;
        }

        match gs.current_phase {
            Phase::Active
            | Phase::Energy
            | Phase::Draw
            | Phase::FirstAttackerPerformance
            | Phase::SecondAttackerPerformance
            | Phase::LiveVictoryDetermination => {
                TurnEngine::advance_phase(gs);
                continue;
            }
            _ => {}
        }

        let action_list = game_setup::generate_possible_actions(gs);
        if action_list.is_empty() {
            TurnEngine::advance_phase(gs);
            continue;
        }

        use rand::seq::SliceRandom;
        let action = action_list.choose(&mut rand::thread_rng()).unwrap();

        let _ = TurnEngine::execute_main_phase_action(
            gs,
            &action.action_type,
            action.parameters.as_ref().and_then(|p| p.card_id),
            action.parameters.as_ref().and_then(|p| p.card_indices.clone()),
            action
                .parameters
                .as_ref()
                .and_then(|p| p.stage_area.as_deref().and_then(parse_stage_area)),
            action.parameters.as_ref().and_then(|p| p.use_baton_touch),
        );
        actions += 1;
    }
    actions
}

fn bench_micro(c: &mut Criterion) {
    init_ctx();
    let mut gs = build_game(0);
    gs.current_phase = Phase::Main;
    gs.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;

    c.bench_function("generate_possible_actions", |b| {
        b.iter(|| game_setup::generate_possible_actions(black_box(&gs)));
    });

    c.bench_function("check_timing", |b| {
        b.iter(|| {
            let mut gs = build_game(0);
            TurnEngine::check_timing(black_box(&mut gs))
        });
    });
}

fn bench_single_game(c: &mut Criterion) {
    init_ctx();
    let mut group = c.benchmark_group("single_game");
    group.sample_size(20);

    for (i, (name, _, _)) in ctx().deck_templates.iter().enumerate() {
        group.bench_with_input(BenchmarkId::from_parameter(name), &name, |b, _| {
            b.iter(|| {
                let mut gs = build_game(i);
                run_game_to_completion(&mut gs)
            });
        });
    }
    group.finish();
}

fn bench_game_throughput(c: &mut Criterion) {
    init_ctx();
    let mut group = c.benchmark_group("game_throughput");
    group.sample_size(10);

    for (i, (name, _, _)) in ctx().deck_templates.iter().enumerate() {
        group.bench_with_input(BenchmarkId::from_parameter(name), &name, |b, _| {
            b.iter(|| {
                let mut total = 0u64;
                for _ in 0..5 {
                    let mut gs = build_game(i);
                    total += run_game_to_completion(&mut gs);
                }
                total
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_micro,
    bench_single_game,
    bench_game_throughput
);
criterion_main!(benches);
