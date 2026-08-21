use std::sync::Arc;

use rabuka_engine::bot::{strategy, strategy_v2, strategy_v3, V2Policy, V3Plan};
use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::deck_parser::DeckParser;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState};
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;

fn fresh_database() -> Arc<CardDatabase> {
    let cards_json = include_str!("../../../cards/cards.json");
    let cards = CardLoader::load_cards_from_strs(cards_json).expect("cards");
    Arc::new(CardDatabase::load_or_create(cards))
}

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn range(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
}

fn main() {
    let mut db = fresh_database();
    let deck = DeckParser::parse_deck_file(std::path::Path::new("../web_ui/decks/hasunosora_cup.txt"))
        .expect("hasunosora");
    let nums = DeckParser::deck_list_to_card_numbers(&deck);
    let (t1, t2) = game_setup::build_two_decks(&mut db, &nums, &nums).expect("build");
    let policy = V2Policy::default();

    let mut rng = Lcg(0xC0FFEE_1234_5678);
    'games: for game_idx in 0..20 {
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
        let plan_p1 = V3Plan::detect(&gs, 0, &db);
        let plan_p2 = V3Plan::detect(&gs, 1, &db);
        let mut last_turn = 0u8;
        let mut stuck = 0u32;
        let mut same_action = 0u32;
        let mut last_desc = String::new();

        for iter in 0..700 {
            TurnEngine::check_victory_condition(&mut gs);
            if gs.game_result != GameResult::Ongoing {
                break;
            }
            if gs.turn_number == last_turn {
                stuck += 1;
                if stuck > 200 {
                    eprintln!("\n=== STALL game={game_idx} turn={} phase={:?} iter={iter} ===", gs.turn_number, gs.current_phase);
                    eprintln!(
                        "p1: hand={} wr={} wr_lives={} energy={}/{} | p2: hand={} wr={} wr_lives={} energy={}/{}",
                        gs.player1.hand.cards.len(),
                        gs.player1.waitroom.cards.len(),
                        gs.player1.waitroom.cards.iter().filter(|&&c| gs.card_database.get_card(c).map_or(false, |ca| ca.card_type == rabuka_engine::card::CardType::Member)).count(),
                        gs.player1.energy_zone.active_count(),
                        gs.player1.energy_zone.cards.len(),
                        gs.player2.hand.cards.len(),
                        gs.player2.waitroom.cards.len(),
                        gs.player2.waitroom.cards.iter().filter(|&&c| gs.card_database.get_card(c).map_or(false, |ca| ca.card_type == rabuka_engine::card::CardType::Member)).count(),
                        gs.player2.energy_zone.active_count(),
                        gs.player2.energy_zone.cards.len(),
                    );
                    eprintln!("baton_zero_cost={} turn_limited_size={} has_pending={}", gs.baton_touch_zero_cost, gs.turn_limited_abilities_used.len(), gs.has_pending_choice());
                    eprintln!("turn_limited: {:?}", gs.turn_limited_abilities_used);
                    let actions = game_setup::generate_possible_actions(&gs);
                    eprintln!("gen_actions: {}", actions.len());
                    for a in actions.iter().take(6) {
                        eprintln!("  - {:?} | {} | sel={:?} | idx={:?}", a.action_type, a.description, a.selected, a.parameters.as_ref().and_then(|p| p.card_index));
                    }
                    if let Some(a) = actions.first() {
                        eprintln!("  -- executing first as probe --");
                        let mut probe = gs.clone();
                        let r = game_setup::execute_action(&mut probe, a);
                        eprintln!("  result: {r:?} | pending_after={}", probe.has_pending_choice());
                    }
                    continue 'games;
                }
            } else {
                stuck = 0;
                last_turn = gs.turn_number;
            }

            if game_setup::auto_advance_one(&mut gs) {
                continue;
            }
            let actions = game_setup::generate_possible_actions(&gs);
            if actions.is_empty() {
                TurnEngine::advance_phase(&mut gs);
                continue;
            }
            let active_is_p1 = gs.active_player().id == "p1";
            let plan_me = if active_is_p1 { &plan_p1 } else { &plan_p2 };

            // pending_choice handling via resume is inside execute_main_phase_action; our driver
            // just picks among generated actions (Choice* appear when pending).
            if gs.current_phase == rabuka_engine::game_state::Phase::RockPaperScissors
                || gs.current_phase == rabuka_engine::game_state::Phase::ChooseFirstAttacker
            {
                let a = &actions[rng.range(actions.len())];
                let _ = game_setup::execute_action(&mut gs, a);
                game_setup::settle_single_player_state(&mut gs);
                continue;
            }
            if matches!(
                gs.current_phase,
                rabuka_engine::game_state::Phase::MulliganFirstAttacker
                    | rabuka_engine::game_state::Phase::MulliganSecondAttacker
            ) {
                let a = strategy_v2::choose_mulligan_action_v2(&gs, &actions, &db);
                let _ = game_setup::execute_action(&mut gs, &a);
                game_setup::settle_single_player_state(&mut gs);
                continue;
            }
            if matches!(
                gs.current_phase,
                rabuka_engine::game_state::Phase::LiveCardSetFirstAttacker
                    | rabuka_engine::game_state::Phase::LiveCardSetSecondAttacker
            ) {
                let a = if active_is_p1 {
                    strategy_v3::choose_live_set_action_v3(&gs, &actions, &db, &policy, plan_me)
                } else {
                    strategy_v2::choose_live_set_action_v2(&gs, &actions, &db, &policy)
                };
                let _ = game_setup::execute_action(&mut gs, &a);
                game_setup::settle_single_player_state(&mut gs);
                continue;
            }

            let action = if active_is_p1 {
                strategy_v3::choose_action_heuristic_v3(&gs, &actions, 0, plan_me)
            } else {
                strategy_v2::choose_action_heuristic_v2(&gs, &actions, 1)
            };
            let desc = action.description.clone();
            let r = game_setup::execute_action(&mut gs, &action);
            if desc == last_desc {
                same_action += 1;
            } else {
                same_action = 0;
            }
            last_desc = desc.clone();
            if r.is_err() && same_action > 3 {
                eprintln!("[diag] iter={iter} t={} act='{}' failed repeatedly: {r:?}", gs.turn_number, action.description);
            }
            if same_action > 3 && stuck > 160 {
                eprintln!("[trace] t={} stuck={} act='{}' -> {r:?} hand={} energy={}/{}", gs.turn_number, stuck, desc, gs.active_player().hand.cards.len(), gs.active_player().energy_zone.active_count(), gs.active_player().energy_zone.cards.len());
            }
            game_setup::settle_single_player_state(&mut gs);
        }
    }
    eprintln!("diag harness done");
}
