//! Bad-game report: play v2-vs-v5 games and record compact per-game telemetry
//! IN MEMORY (no per-action strings -> no trace-mode distortion), then print
//! histograms and full timelines for the worst games.
//!
//! Usage: cargo run --release --bin bad_game_report -- [games] [worst_k]

use rabuka_engine::core::constants::U8Count;
use rabuka_engine::bin_common::{deal_default_game, fresh_database, load_deck};
use rabuka_engine::bot::{strategy_v2, strategy_v4, strategy_v5};
use rabuka_engine::card::{CardDatabase, CardType};
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, Phase};
use rabuka_engine::rng::Lcg;
use rabuka_engine::turn::TurnEngine;
use std::sync::Arc;

#[derive(Default, Clone)]
struct GameRecord {
    result: String,
    turns: u8,
    aborted: bool,
    // per-turn snapshots: (succ_p1, succ_p2, hand_lives_p1, hand_lives_p2)
    timeline: Vec<(u8, u8, u8, u8)>,
    // live-set decisions: count of phases that set 0/1/2+ lives per side
    set_sizes: [Vec<u8>; 2],
}

fn lives_in_hand(p: &rabuka_engine::player::Player, db: &CardDatabase) -> u8 {
    p.hand
        .cards
        .iter()
        .filter(|&&c| db.get_card(c).is_some_and(|x| x.card_type == CardType::Live))
        .count().u8_count()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n_games: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(300);
    let worst_k: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(3);

    let db = fresh_database();
    let nums = load_deck("5CP3Z idou");
    eprintln!(
        "deck entries={} distinct={}",
        nums.len(),
        nums.iter().collect::<std::collections::HashSet<_>>().len()
    );
    let mut db_mut = Arc::clone(&db);
    let (t1, t2) =
        game_setup::build_two_decks(&mut db_mut, &nums, &nums).expect("build decks");
    drop(db_mut);

    let mut rng = Lcg(0x5EED_1234_ABCD_0001);

    let mut records: Vec<GameRecord> = Vec::with_capacity(n_games as usize);

    for game in 0..n_games {
        let mut gs = deal_default_game(&db, &t1, &t2);
        if game == 0 {
            eprintln!(
                "REPORT post-deal: en={} deck={} hand={}",
                gs.player1.energy_zone.active_count(),
                gs.player1.main_deck.cards.len(),
                gs.player1.hand.cards.len()
            );
        }
        let mut rec = GameRecord::default();
        let mut last_turn = 0u8;
        let mut stuck = 0u32;
        let mut cur_set_side: Option<u8> = None;
        let mut cur_set_count: u8 = 0;

        for _ in 0..600 {
            TurnEngine::check_victory_condition(&mut gs);
            if gs.game_result != GameResult::Ongoing {
                break;
            }
            if gs.turn_number == last_turn {
                stuck += 1;
                if stuck > 200 {
                    rec.aborted = true;
                    break;
                }
            } else {
                stuck = 0;
                last_turn = gs.turn_number;
                let s1 = gs.player1.success_live_card_zone.cards.len().u8_count();
                let s2 = gs.player2.success_live_card_zone.cards.len().u8_count();
                rec.timeline.push((
                    s1,
                    s2,
                    lives_in_hand(&gs.player1, &db),
                    lives_in_hand(&gs.player2, &db),
                ));
                if records.is_empty() && std::env::var("DUMP_STATE").is_ok() {
                    for (lbl, p) in [("P1", &gs.player1), ("P2", &gs.player2)] {
                        eprintln!(
                            "T{} {} hand={} (lives {}) deck={} wr={} livezone={} en={}",
                            gs.turn_number,
                            lbl,
                            p.hand.cards.len(),
                            lives_in_hand(p, &db),
                            p.main_deck.cards.len(),
                            p.waitroom.cards.len(),
                            p.live_card_zone.cards.len(),
                            p.energy_zone.active_count(),
                        );
                    }
                    eprintln!("phase={:?}", gs.current_phase);
                }
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
            let random_mode = std::env::var("RANDOM_MODE").is_ok();

            if random_mode {
                let a = actions[rng.range(actions.len())].clone();
                let _ = game_setup::execute_action(&mut gs, &a);
                game_setup::settle_single_player_state(&mut gs);
                continue;
            }

            match gs.current_phase {
                Phase::RockPaperScissors => {
                    let a = actions[rng.range(actions.len())].clone();
                    let _ = game_setup::execute_action(&mut gs, &a);
                    game_setup::settle_single_player_state(&mut gs);
                    continue;
                }
                Phase::ChooseFirstAttacker => {
                    let won = gs.rps_winner == Some(if active_is_p1 { 1 } else { 2 });
                    let a = if won {
                        actions
                            .iter()
                            .find(|a| {
                                a.action_type
                                    == rabuka_engine::game_setup::ActionType::ChooseSecondAttacker
                            })
                            .unwrap_or(&actions[0])
                            .clone()
                    } else {
                        actions[rng.range(actions.len())].clone()
                    };
                    let _ = game_setup::execute_action(&mut gs, &a);
                    game_setup::settle_single_player_state(&mut gs);
                    continue;
                }
                Phase::MulliganFirstAttacker | Phase::MulliganSecondAttacker => {
                    let a = strategy_v4::choose_mulligan_v4(&gs, &actions, &db);
                    let _ = game_setup::execute_action(&mut gs, &a);
                    game_setup::settle_single_player_state(&mut gs);
                    continue;
                }
                Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker => {
                    if cur_set_side != Some(me) {
                        if let Some(side) = cur_set_side {
                            rec.set_sizes[side as usize].push(cur_set_count.min(3));
                        }
                        cur_set_side = Some(me);
                        cur_set_count = 0;
                    }
                    let before = gs.live_card_selected_indices.len();
                    let a = strategy_v5::choose_live_set_v5(&gs, &actions, &db);
                    let _ = game_setup::execute_action(&mut gs, &a);
                    game_setup::settle_single_player_state(&mut gs);
                    let after = gs.live_card_selected_indices.len();
                    let advanced = !matches!(
                        gs.current_phase,
                        Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker
                    );
                    if after > before {
                        cur_set_count += (after - before) as u8;
                    }
                    if advanced && cur_set_side == Some(me) {
                        rec.set_sizes[me as usize].push(cur_set_count.min(3));
                        cur_set_side = None;
                    }
                    continue;
                }
                _ => {}
            }

            // Both sides play their assigned policy: P1 = v2, P2 = v5.
            let action = if active_is_p1 {
                strategy_v2::choose_action_heuristic_v2(&gs, &actions, me)
            } else {
                strategy_v5::choose_action_v5(&gs, &actions, me)
            };
            let _ = game_setup::execute_action(&mut gs, &action);
            game_setup::settle_single_player_state(&mut gs);
        }

        let z1 = gs.player1.success_live_card_zone.cards.len();
        let z2 = gs.player2.success_live_card_zone.cards.len();
        rec.turns = gs.turn_number;
        rec.result = if rec.aborted {
            "ABORT".into()
        } else if z1 >= 3 && z2 <= 2 {
            "P1(v2) WIN".into()
        } else if z2 >= 3 && z1 <= 2 {
            "P2(v5) WIN".into()
        } else if z1 >= 3 && z2 >= 3 {
            "DOUBLE-3 DRAW".into()
        } else {
            "STALL DRAW".into()
        };
        records.push(rec);
    }

    // ── summary ──
    let mut tally = std::collections::BTreeMap::new();
    for r in &records {
        *tally.entry(r.result.clone()).or_insert(0u32) += 1;
    }
    println!("=== {} games, P1=v2 vs P2=v5 ===", records.len());
    for (k, v) in &tally {
        println!("  {k}: {v}");
    }
    let mut lens: Vec<u8> = records.iter().map(|r| r.turns).collect();
    lens.sort_unstable();
    let pct = |p: usize| -> u8 { lens[lens.len() * p / 100] };
    println!(
        "turn length: min={} p25={} median={} p75={} p90={} max={}",
        lens[0],
        pct(25),
        pct(50),
        pct(75),
        pct(90),
        lens[lens.len() - 1]
    );

    // ── aggregate success curve ──
    println!("\nsuccess-zone curve over all games (mean s1 s2):");
    let maxt = records.iter().map(|r| r.timeline.len()).max().unwrap_or(0);
    for t in 0..maxt {
        let mut n = 0;
        let (mut a, mut b) = (0f64, 0f64);
        for r in &records {
            if let Some(&(s1, s2, _, _)) = r.timeline.get(t) {
                n += 1;
                a += s1 as f64;
                b += s2 as f64;
            }
        }
        if n > 0 {
            println!("  T{}: {:.2} {:.2} (n={})", t + 1, a / n as f64, b / n as f64, n);
        }
    }

    // ── set-size distribution per side ──
    for (side, name) in [(0usize, "v2"), (1usize, "v5")] {
        let mut c = std::collections::BTreeMap::new();
        let mut total = 0;
        for r in &records {
            for &s in &r.set_sizes[side] {
                *c.entry(s).or_insert(0u32) += 1;
                total += 1;
            }
        }
        let dist: Vec<String> = c.iter().map(|(k, v)| format!("{k}:{v}")).collect();
        println!(
            "\n{name} live-set sizes (0=fold 1=single 2/3=multi), total phases {}: {}",
            total,
            dist.join(" ")
        );
    }

    // ── worst-K timelines ──
    let mut order: Vec<usize> = (0..records.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(records[i].timeline.len()));
    println!("\n=== worst {worst_k} games (longest) ===");
    for &idx in order.iter().take(worst_k) {
        let r = &records[idx];
        println!(
            "\ngame {} | {} | {} turns",
            idx + 1,
            r.result,
            r.timeline.len()
        );
        println!("  turn | s1 s2 | handLives1 handLives2");
        for (i, &(s1, s2, l1, l2)) in r.timeline.iter().enumerate() {
            println!("   {:>2}  |  {} {}  | {} {}", i + 1, s1, s2, l1, l2);
        }
    }

    // ── typical bad-shape: games where v5 lost ──
    let losses: Vec<&GameRecord> = records.iter().filter(|r| r.result == "P1(v2) WIN").collect();
    if !losses.is_empty() {
        let mut lo: Vec<usize> = (0..records.len()).collect();
        lo.sort_by_key(|&i| std::cmp::Reverse(records[i].timeline.len()));
        println!("\n=== longest v5 LOSSES ===");
        let mut shown = 0;
        for &idx in &lo {
            let r = &records[idx];
            if r.result != "P1(v2) WIN" {
                continue;
            }
            shown += 1;
            println!("\ngame {} | {} | {} turns", idx + 1, r.result, r.timeline.len());
            for (i, &(s1, s2, l1, l2)) in r.timeline.iter().enumerate() {
                println!("   {:>2}  |  {} {}  | {} {}", i + 1, s1, s2, l1, l2);
            }
            if shown >= 2 {
                break;
            }
        }
    }
}
