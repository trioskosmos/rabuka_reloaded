use rand::Rng;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;

use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader;
use rabuka_engine::deck_builder;
use rabuka_engine::deck_parser::DeckParser;
use rabuka_engine::game_setup::{self, ActionType};
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

fn main() {
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = card_loader::CardLoader::load_cards_from_file(cards_path).unwrap();
    let db = Arc::new(CardDatabase::load_or_create(cards));

    let deck_path = std::path::Path::new("../web_ui/decks/fade deck.txt");
    let deck = DeckParser::parse_deck_file(deck_path).unwrap();
    let card_numbers = DeckParser::deck_list_to_card_numbers(&deck);

    let num_games: u32 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(500);
    let out_path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "../train_data.bin".into());

    let mut t1 = deck_builder::DeckBuilder::build_deck_from_database(
        &mut Arc::clone(&db),
        card_numbers.clone(),
    )
    .unwrap();
    let mut t2 =
        deck_builder::DeckBuilder::build_deck_from_database(&mut Arc::clone(&db), card_numbers)
            .unwrap();
    let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(
        &mut t1,
        &mut Arc::clone(&db),
    );
    let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(
        &mut t2,
        &mut Arc::clone(&db),
    );

    let mut out = File::create(&out_path).unwrap();
    let mut total: u64 = 0;
    let mut wins = 0u32;
    let mut losses = 0u32;
    let mut draws = 0u32;
    let mut rng: u64 = 0xDEAD_BEEF;
    let mut total_actions: u64 = 0;
    let start = std::time::Instant::now();

    for game_idx in 0..num_games {
        let mut d1 = t1.clone();
        d1.shuffle_main_deck();
        d1.shuffle_energy_deck();
        let mut d2 = t2.clone();
        d2.shuffle_main_deck();
        d2.shuffle_energy_deck();

        let mut p1 = Player::new("p1".into(), "P1".into(), true);
        let mut p2 = Player::new("p2".into(), "P2".into(), false);
        p1.set_main_deck(d1.main_deck);
        p1.set_energy_deck(d1.energy_deck);
        p2.set_main_deck(d2.main_deck);
        p2.set_energy_deck(d2.energy_deck);

        let mut gs = GameState::new(p1, p2, Arc::clone(&db));
        game_setup::setup_game(&mut gs);

        let mut examples: Vec<Example> = Vec::new();
        let mut last_turn = 0u32;
        let mut stuck = 0u32;

        for _ in 0..500 {
            TurnEngine::check_victory_condition(&mut gs);
            if gs.game_result != GameResult::Ongoing {
                break;
            }

            if gs.turn_number == last_turn {
                stuck += 1;
                if stuck > 200 {
                    break;
                }
            } else {
                stuck = 0;
                last_turn = gs.turn_number;
            }

            if !gs.has_pending_choice() {
                match gs.current_phase {
                    Phase::Active
                    | Phase::Energy
                    | Phase::Draw
                    | Phase::FirstAttackerPerformance
                    | Phase::SecondAttackerPerformance
                    | Phase::LiveVictoryDetermination => {
                        TurnEngine::advance_phase(&mut gs);
                        continue;
                    }
                    _ => {}
                }
            }

            let actions = game_setup::generate_possible_actions(&gs);
            if actions.is_empty() {
                TurnEngine::advance_phase(&mut gs);
                continue;
            }

            let is_p1 = gs.active_player().id == "p1";
            let is_main = gs.current_phase == Phase::Main;

            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let idx = (rng as usize) % actions.len();
            let action = actions[idx].clone();

            // Record example if it's P1's main phase action
            if is_p1 && is_main {
                examples.push(Example::capture(&gs, &action));
            }

            let params = action.parameters.clone();
            let _ = TurnEngine::execute_main_phase_action(
                &mut gs,
                &action.action_type,
                params.as_ref().and_then(|p| p.card_id),
                params.as_ref().and_then(|p| p.card_indices.clone()),
                params
                    .as_ref()
                    .and_then(|p| p.stage_area.as_deref().and_then(|s| s.parse().ok())),
                params.as_ref().and_then(|p| p.use_baton_touch),
            );
            game_setup::settle_single_player_state(&mut gs);
            total_actions += 1;
        }

        let p1_success = gs.player1.success_live_card_zone.cards.len();
        let p2_success = gs.player2.success_live_card_zone.cards.len();
        let outcome: f32 = if p1_success > p2_success {
            1.0
        } else if p1_success < p2_success {
            -1.0
        } else {
            0.0
        };

        if outcome > 0.0 {
            wins += 1;
        } else if outcome < 0.0 {
            losses += 1;
        } else {
            draws += 1;
        }

        // Write all examples for this game with the outcome
        for ex in &examples {
            write_example(&mut out, ex, outcome);
        }
        total += examples.len() as u64;

        if game_idx % 10 == 0 || game_idx == num_games - 1 {
            let elapsed = start.elapsed().as_secs_f64();
            let aps = total_actions as f64 / elapsed.max(0.001);
            eprintln!(
                "Game {}/{}: outcome={:+.0} ex={} total={} {:.0} act/s",
                game_idx + 1,
                num_games,
                outcome,
                examples.len(),
                total,
                aps
            );
        }
    }

    eprintln!(
        "\nDone: {} games, {} examples, wins={} losses={} draws={}",
        num_games, total, wins, losses, draws
    );
}

/// Per-example: hand card IDs, stage card IDs, action card_id, action_type, outcome
struct Example {
    my_hand: Vec<i16>,
    my_stage: [i16; 3],
    opp_stage: [i16; 3],
    action_card_id: i16,
    action_type_idx: u8,
}

impl Example {
    fn capture(gs: &GameState, action: &game_setup::Action) -> Self {
        let action_card_id = action
            .parameters
            .as_ref()
            .and_then(|p| p.card_id)
            .unwrap_or(0);
        let action_type_idx = match action.action_type {
            ActionType::Pass => 0,
            ActionType::RockChoice => 1,
            ActionType::PaperChoice => 2,
            ActionType::ScissorsChoice => 3,
            ActionType::ChooseFirstAttacker => 4,
            ActionType::ChooseSecondAttacker => 5,
            ActionType::MulliganHeader => 6,
            ActionType::SelectMulligan => 7,
            ActionType::ConfirmMulligan => 8,
            ActionType::SkipMulligan => 9,
            ActionType::LiveCardHeader => 10,
            ActionType::SelectLiveCard => 11,
            ActionType::ConfirmLiveCardSet => 12,
            ActionType::SkipLiveCardSet => 13,
            ActionType::PlayMemberToStage => 14,
            ActionType::UseAbility => 15,
            ActionType::SetLiveCard => 16,
            ActionType::FinishLiveCardSet => 17,
            ActionType::ChoiceDecision => 18,
            ActionType::ChoiceSelect => 19,
            ActionType::ChoiceSkip => 20,
            ActionType::ChoiceOption => 21,
            ActionType::ChoicePosition => 22,
            ActionType::EnergyCharge => 23,
            ActionType::PassRemaining => 24,
        };
        Self {
            my_hand: gs.player1.hand.cards.to_vec(),
            my_stage: gs.player1.stage.stage,
            opp_stage: gs.player2.stage.stage,
            action_card_id,
            action_type_idx,
        }
    }
}

fn write_example(f: &mut File, ex: &Example, outcome: f32) {
    // hand: u8 count + i16 cards
    let hand_len = ex.my_hand.len() as u8;
    let _ = f.write_all(&[hand_len]);
    for &c in &ex.my_hand {
        let _ = f.write_all(&c.to_le_bytes());
    }
    // stage: 3 x i16
    for &c in &ex.my_stage {
        let _ = f.write_all(&c.to_le_bytes());
    }
    // opp_stage: 3 x i16
    for &c in &ex.opp_stage {
        let _ = f.write_all(&c.to_le_bytes());
    }
    // action: i16 card_id + u8 action_type
    let _ = f.write_all(&ex.action_card_id.to_le_bytes());
    let _ = f.write_all(&[ex.action_type_idx]);
    // outcome: f32
    let _ = f.write_all(&outcome.to_le_bytes());
}
