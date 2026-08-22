//! Calibration harness for v4 live-set "can this pass" predictions.
//!
//! Plays v4-vs-v2 games; at every v4 live-set CONFIRM captures the portfolio,
//! then measures ACTUAL placement rate across K shuffled-deck trials using
//! the engine itself. Compares two predictors:
//!   - CONSERVATIVE (0.6× flip credit) — what v4 currently uses
//!   - FULL-MEAN (1.0× flip credit)
//! Reports confusion matrices vs empirical outcomes.

use rabuka_engine::bot::{strategy_v2, strategy_v3, strategy_v4};
use rabuka_engine::card::{CardDatabase, HeartColor, HeartMap, CardType};
use rabuka_engine::player::Player;
use rabuka_engine::card_loader;
use rabuka_engine::deck_parser;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::rng::Lcg;
use rabuka_engine::turn::TurnEngine;
use std::sync::Arc;

fn fresh_database() -> Arc<CardDatabase> {
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = card_loader::CardLoader::load_cards_from_file(cards_path).expect("load cards");
    Arc::new(CardDatabase::load_or_create(cards))
}

fn load_deck(name: &str) -> Vec<String> {
    let p = std::path::Path::new("../web_ui/decks").join(format!("{name}.txt"));
    let deck = deck_parser::DeckParser::parse_deck_file(&p).expect("parse deck");
    deck_parser::DeckParser::deck_list_to_card_numbers(&deck)
}

fn deal(
    db: &Arc<CardDatabase>,
    t1: &rabuka_engine::deck_builder::Deck,
    t2: &rabuka_engine::deck_builder::Deck,
) -> GameState {
    let mut d1 = t1.clone();
    let mut d2 = t2.clone();
    d1.shuffle_main_deck();
    d1.shuffle_energy_deck();
    d2.shuffle_main_deck();
    d2.shuffle_energy_deck();
    let mut p1 = rabuka_engine::player::Player::new("p1".into(), "P1".into(), true);
    let mut p2 = rabuka_engine::player::Player::new("p2".into(), "P2".into(), false);
    p1.set_main_deck(d1.main_deck);
    p1.set_energy_deck(d1.energy_deck);
    p2.set_main_deck(d2.main_deck);
    p2.set_energy_deck(d2.energy_deck);
    let mut gs = GameState::new(p1, p2, Arc::clone(db));
    game_setup::setup_game(&mut gs);
    gs
}

type Acc = [i32; 11];

fn hc_index(c: HeartColor) -> usize {
    use HeartColor as H;
    match c {
        H::Heart00 => 0,
        H::Heart01 => 1,
        H::Heart02 => 2,
        H::Heart03 => 3,
        H::Heart04 => 4,
        H::Heart05 => 5,
        H::Heart06 => 6,
        H::BAll => 7,
        H::Draw => 8,
        H::Score => 9,
        H::All => 10,
    }
}

fn acc_add(acc: &mut Acc, hearts: &HeartMap) {
    for (c, v) in hearts.iter() {
        acc[hc_index(*c)] += *v as i32;
    }
}

fn heart_pool(gs: &GameState, me_player: u8, db: &CardDatabase, confidence: f64) -> Acc {
    let p = if me_player == 0 { &gs.player1 } else { &gs.player2 };
    let mut acc = [0i32; 11];
    let mut blades = 0i32;
    for &cid in p.stage.stage.iter() {
        if cid < 0 {
            continue;
        }
        let waiting = gs.mods.get_orientation_modifier(cid) == Some("wait");
        if let Some(card) = db.get_card(cid) {
            if let Some(bh) = &card.base_heart {
                acc_add(&mut acc, &bh.hearts);
            }
            if !waiting {
                blades += card.blade as i32;
            }
        }
    }
    let deck_len = p.main_deck.cards.len().max(1);
    let density = p
        .main_deck
        .cards
        .iter()
        .filter(|&&cid| db.get_card(cid).map_or(false, |c| c.blade_heart.is_some()))
        .count() as f64
        / deck_len as f64;
    acc[10] += ((blades as f64 * density) * confidence).floor() as i32;
    acc
}

fn hand_lives(p: &Player, db: &CardDatabase) -> Vec<(usize, Acc)> {
    let mut out = Vec::new();
    for (hand_index, &cid) in p.hand.cards.iter().enumerate() {
        if let Some(card) = db.get_card(cid) {
            if !matches!(card.card_type, CardType::Live) {
                continue;
            }
            let mut need = [0i32; 11];
            if let Some(nh) = &card.need_heart {
                acc_add(&mut need, &nh.hearts);
            }
            out.push((hand_index, need));
        }
    }
    out
}

fn alloc_probe(pool: &mut Acc, need: &Acc) -> bool {
    let mut p = *pool;
    let mut wildcard_used = 0i32;
    for c in 1..=6 {
        let have = p[c];
        let want = need[c];
        if have >= want {
            p[c] = have - want;
        } else {
            let deficit = want - have;
            wildcard_used += deficit;
            if wildcard_used > p[7] + p[10] {
                return false;
            }
            p[c] = 0;
        }
    }
    let mut take = wildcard_used;
    let b = take.min(p[7]);
    p[7] -= b;
    take -= b;
    p[10] -= take.min(p[10]);
    let mut grey = need[0];
    let g0 = grey.min(p[0]);
    p[0] -= g0;
    grey -= g0;
    for c in (1..=6).rev() {
        if grey <= 0 {
            break;
        }
        let t = grey.min(p[c]);
        p[c] -= t;
        grey -= t;
    }
    if grey > 0 {
        let w = p[7] + p[10];
        if grey > w {
            return false;
        }
        let b2 = grey.min(p[7]);
        p[7] -= b2;
        p[10] -= grey - b2;
    }
    *pool = p;
    true
}

/// Resolve the live phase from `sim` (selection already in place) and return
/// whether MY success count increased. Opponent sets nothing when it's their
/// set slot (conservative ground truth).
fn resolve_actual(
    sim: &mut GameState,
    me_is_p1: bool,
    rng: &mut dyn FnMut(usize) -> usize,
) -> Option<bool> {
    let succ_before = if me_is_p1 {
        sim.player1.success_live_card_zone.cards.len()
    } else {
        sim.player2.success_live_card_zone.cards.len()
    };
    // Execute confirm.
    let acts = game_setup::generate_possible_actions(sim);
    let conf = acts.iter().find(|a| {
        matches!(
            a.action_type,
            game_setup::ActionType::ConfirmLiveCardSet | game_setup::ActionType::SkipLiveCardSet
        )
    })?;
    let _ = game_setup::execute_action(sim, conf);

    for _ in 0..80 {
        if sim.game_result != GameResult::Ongoing {
            break;
        }
        match sim.current_phase {
            Phase::FirstAttackerPerformance
            | Phase::SecondAttackerPerformance
            | Phase::LiveVictoryDetermination => {
                if game_setup::auto_advance_one(sim) {
                    continue;
                }
                // pending choice during performance: first option
                let acts = game_setup::generate_possible_actions(sim);
                if acts.is_empty() {
                    TurnEngine::advance_phase(sim);
                    continue;
                }
                let pick = rng(acts.len());
                let _ = game_setup::execute_action(sim, &acts[pick]);
            }
            Phase::LiveCardSetSecondAttacker | Phase::LiveCardSetFirstAttacker => {
                // opponent slot: force an empty confirm so only MY check is
                // measured
                let acts = game_setup::generate_possible_actions(sim);
                if let Some(a) = acts.iter().find(|a| {
                    matches!(
                        a.action_type,
                        game_setup::ActionType::ConfirmLiveCardSet
                            | game_setup::ActionType::SkipLiveCardSet
                    )
                }) {
                    let _ = game_setup::execute_action(sim, a);
                    game_setup::settle_single_player_state(sim);
                } else if game_setup::auto_advance_one(sim) {
                } else {
                    TurnEngine::advance_phase(sim);
                }
            }
            _ => break,
        }
    }
    let succ_after = if me_is_p1 {
        sim.player1.success_live_card_zone.cards.len()
    } else {
        sim.player2.success_live_card_zone.cards.len()
    };
    Some(succ_after > succ_before)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let budget: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(20);

    // ⚠ db must be uniquely owned when build_two_decks runs — DeckBuilder
    // registers PER-COPY card IDs via Arc::make_mut. An extra Arc::clone here
    // makes make_mut clone the DB, registrations land on the throwaway copy,
    // and every drawn card becomes unresolvable (hands full of ghosts).
    let mut db = fresh_database();
    let nums = load_deck("5CP3Z idou");
    let (t1, t2) = game_setup::build_two_decks(&mut db, &nums, &nums).expect("decks");

    let mut rng_state = Lcg(0xC0FFEE);
    let v2_policy = strategy_v2::V2Policy::default();

    // confusion counters: [predicted][actual]
    // predictors: conservative (v4's choice driver), full-mean
    let mut cons = [[0u32; 2]; 2]; // [pred_pass][actual_place]
    let mut full = [[0u32; 2]; 2];
    let mut samples: Vec<String> = Vec::new();
    let mut n_confirms: u32 = 0;
    let mut n_life_samples: u32 = 0;

    let start = std::time::Instant::now();
    'games: for game in 1..=5000u32 {
        // budget handled by start_time() anchor below
        // time budget check below instead
        let mut gs = deal(&db, &t1, &t2);
        let plan_p1 = strategy_v3::V3Plan::detect(&gs, 0, &db);
        let mut last_turn = 0u8;
        let mut stuck = 0u32;

        for _ in 0..600 {
            if start.elapsed() > std::time::Duration::from_secs(budget) {
                break 'games;
            }
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
            if game_setup::auto_advance_one(&mut gs) {
                continue;
            }
            let actions = game_setup::generate_possible_actions(&gs);
            if actions.is_empty() {
                TurnEngine::advance_phase(&mut gs);
                continue;
            }
            let active_is_p1 = gs.active_player().id == "p1";
            let me = if active_is_p1 { 0u8 } else { 1u8 };

            match gs.current_phase {
                Phase::RockPaperScissors
                | Phase::ChooseFirstAttacker
                | Phase::MulliganFirstAttacker
                | Phase::MulliganSecondAttacker => {
                    let a = match gs.current_phase {
                        Phase::MulliganFirstAttacker | Phase::MulliganSecondAttacker => {
                            strategy_v3::choose_mulligan_action_v3(&gs, &actions, &db)
                        }
                        _ => actions[rng_state.range(actions.len())].clone(),
                    };
                    let _ = game_setup::execute_action(&mut gs, &a);
                    game_setup::settle_single_player_state(&mut gs);
                    continue;
                }
                Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker => {
                    let a = if active_is_p1 {
                        strategy_v3::choose_live_set_action_v3(
                            &gs, &actions, &db, &v2_policy, &plan_p1,
                        )
                    } else {
                        strategy_v2::choose_live_set_action_v2(
                            &gs, &actions, &db, &v2_policy,
                        )
                    };
                    // ── CALIBRATION POINT ──
                    // Per-life prediction audit: for each distinct hand life,
                    // ask both predictors whether it passes, then measure the
                    // EMPIRICAL pass rate across K shuffled resolutions of a
                    // set=[that life] portfolio.
                    n_confirms += 1;
                    {
                        let my = if me == 0 { &gs.player1 } else { &gs.player2 };
                        if n_confirms <= 3 {
                            let names: Vec<String> = my
                                .hand
                                .cards
                                .iter()
                                .map(|&c| {
                                    db.get_card(c)
                                        .map(|x| {
                                            format!("{}|{:?}", x.card_no, x.card_type)
                                        })
                                        .unwrap_or_default()
                                })
                                .collect();
                            println!(
                                "DIAG g{game} me={me}: hand={} {:?}",
                                my.hand.cards.len(),
                                names
                            );
                            let hl = hand_lives(my, &db);
                            println!("DIAG lives_in_hand={}", hl.len());
                        }
                        let mut tested = 0usize;
                        for &(hi, ref need) in &hand_lives(my, &db) {
                            if tested >= 3 || samples.len() >= 300 {
                                break;
                            }
                            let pc_cons =
                                alloc_probe(&mut heart_pool(&gs, me, &db, 0.6), need);
                            let pc_full =
                                alloc_probe(&mut heart_pool(&gs, me, &db, 1.0), need);

                            let mut placed_n = 0u32;
                            let k = 6;
                            let mut any_resolved = false;
                            for _ in 0..k {
                                let mut sim = gs.clone();
                                // force selection to just this life
                                sim.live_card_selected_indices.clear();
                                sim.live_card_selected_indices.push(hi as u8);
                                use rand::seq::SliceRandom;
                                sim.player1
                                    .main_deck
                                    .cards
                                    .shuffle(&mut rand::thread_rng());
                                sim.player2
                                    .main_deck
                                    .cards
                                    .shuffle(&mut rand::thread_rng());
                                let mut counter = |n: usize| rng_state.range(n);
                                if let Some(placed) =
                                    resolve_actual(&mut sim, active_is_p1, &mut counter)
                                {
                                    any_resolved = true;
                                    if placed {
                                        placed_n += 1;
                                    }
                                }
                            }
                            if !any_resolved {
                                continue;
                            }
                            tested += 1;
                            let passed = placed_n > 0;
                            n_life_samples += 1;
                            cons[if pc_cons { 1 } else { 0 }][if passed { 1 } else { 0 }]
                                += 1;
                            full[if pc_full { 1 } else { 0 }][if passed { 1 } else { 0 }]
                                += 1;
                            if samples.len() < 200 {
                                samples.push(format!(
                                    "g{game} t{} life#{} need_sum={} predC={} predF={} placed={}/{}",
                                    gs.turn_number,
                                    hi,
                                    need.iter().sum::<i32>(),
                                    pc_cons,
                                    pc_full,
                                    placed_n,
                                    k
                                ));
                            }
                        }
                    }

                    let _ = game_setup::execute_action(&mut gs, &a);
                    game_setup::settle_single_player_state(&mut gs);
                    continue;
                }
                _ => {}
            }

            // main phase: v4 for P1, v2 for P2
            let action = if active_is_p1 {
                strategy_v4::choose_action_v4(&gs, &actions, me)
            } else {
                strategy_v2::choose_action_heuristic_v2(&gs, &actions, me)
            };
            let _ = game_setup::execute_action(&mut gs, &action);
            game_setup::settle_single_player_state(&mut gs);
        }
    }

    println!(
        "live-set decisions: {} | lives tested: {}",
        n_confirms, n_life_samples
    );
    println!("=== CALIBRATION RESULTS ===");
    println!("CONSERVATIVE (0.6x) predictor:");
    println!(
        "  pred-pass: {} total | placed {} ({:.0}%)",
        cons[1][0] + cons[1][1],
        cons[1][1],
        100.0 * cons[1][1] as f64 / (cons[1][0] + cons[1][1]).max(1) as f64
    );
    println!(
        "  pred-fail: {} total | placed {} ({:.0}% underconfident)",
        cons[0][0] + cons[0][1],
        cons[0][1],
        100.0 * cons[0][1] as f64 / (cons[0][0] + cons[0][1]).max(1) as f64
    );
    println!("FULL-MEAN (1.0x) predictor:");
    println!(
        "  pred-pass: {} total | placed {} ({:.0}%)",
        full[1][0] + full[1][1],
        full[1][1],
        100.0 * full[1][1] as f64 / (full[1][0] + full[1][1]).max(1) as f64
    );
    println!(
        "  pred-fail: {} total | placed {} ({:.0}% underconfident)",
        full[0][0] + full[0][1],
        full[0][1],
        100.0 * full[0][1] as f64 / (full[0][0] + full[0][1]).max(1) as f64
    );
}

