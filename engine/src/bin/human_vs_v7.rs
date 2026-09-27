//! Human-vs-bot match harness: I play one side, v7 plays the other.
//!
//! Why this exists. Every heuristic change in this project was measured as a
//! win rate over 2400 games, and eight of them turned out to be inert. A win
//! rate says *that* v8 is worse; it never says *what v8 does wrong*. Playing
//! the side directly is the missing instrument: the harness shows the decision,
//! the alternatives, and the consequence, one at a time.
//!
//! Protocol. The game is a pure function of (seed, deck) plus the sequence of
//! decisions on my side, so a run is perfectly replayable. I record my choices
//! in a script file, one action index per line. When the script runs out the
//! harness prints the pending decision - board, action list, and what v8 would
//! have played - and exits. I add a line, re-run, and continue. Nothing is lost
//! and nothing is guessed.
//!
//! Usage:
//!   human_vs_v7 --seed 11 --deck "5CP3Z idou" --side p1 --script my.txt
//!
//! Flags:
//!   --side  p1|p2          which side I play (the other side is v7)
//!   --auto                 my side plays v8 instead of me, for the control run
//!   --quiet                suppress the per-decision board dump

use rabuka_engine::bin_common::{execute_and_settle, fresh_database, load_deck};
use rabuka_engine::bot::strategy_v7;
use rabuka_engine::bot::strategy_v8;
use rabuka_engine::card::CardDatabase;
use rabuka_engine::game_state::{GameResult, GameState};
use rabuka_engine::types::Phase;
use rabuka_engine::game_setup;
use rabuka_engine::turn::TurnEngine;
use std::sync::Arc;

/// Active blades on a seat's stage (Q133: the yell count is active members
/// only). Public information, so this is what a fair player can count.
fn board_blades(gs: &GameState, pid: &str) -> i32 {
    let stage = if gs.player1.id == pid {
        &gs.player1.stage.stage
    } else {
        &gs.player2.stage.stage
    };
    stage
        .iter()
        .filter(|&&c| c >= 0)
        .filter_map(|&c| gs.card_database.get_card(c).map(|k| i32::from(k.blade)))
        .sum()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut seed = 11u32;
    let mut deck = "5CP3Z idou".to_string();
    let mut side = "p1".to_string();
    let mut script = String::from("my_script.txt");
    let mut auto = false;
    let mut quiet = false;
    let mut peek = false;
    let mut diverge_games = 0u32;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--seed" => {
                i += 1;
                seed = args[i].parse().unwrap();
            }
            "--deck" => {
                i += 1;
                deck = args[i].clone();
            }
            "--side" => {
                i += 1;
                side = args[i].clone();
            }
            "--script" => {
                i += 1;
                script = args[i].clone();
            }
            "--auto" => auto = true,
            "--quiet" => quiet = true,
            "--peek" => peek = true,
            "--diverge" => {
                i += 1;
                diverge_games = args[i].parse().unwrap_or(1);
            }
            other => panic!("unknown flag {other}"),
        }
        i += 1;
    }

    game_setup::set_action_display(false);
    game_setup::set_logging_enabled(false);

    let mut db: Arc<CardDatabase> = fresh_database();
    let nums = load_deck(&deck);
    let (t1, t2) = game_setup::build_two_decks(&mut db, &nums, &nums).expect("decks");

    let my_me: u8 = if side == "p1" { 0 } else { 1 };
    let opp_me: u8 = 1 - my_me;

    // Determinism: the same seed reproduces the same deal, so a run is a pure
    // function of (seed, my choices). This is what makes the script/continue
    // loop sound.
    rabuka_engine::rng::seed(seed);
    // --diverge: play N games with v8 on my side and v7 on the other, and
    // record every decision where v8's pick differs from v7's argmax. Playing
    // a side by hand surfaces divergences one at a time and only for the games
    // I happen to reach; this reports the same thing across many games, grouped
    // by the kind of decision, which is what actually points at a fix.
    if diverge_games > 0 {
        use std::collections::BTreeMap;
        let mut tally: BTreeMap<String, usize> = BTreeMap::new();
        let mut samples: BTreeMap<String, String> = BTreeMap::new();
        let mut phase_tally: BTreeMap<String, usize> = BTreeMap::new();
        // (phase, life v7 picked, life v8 picked) on SelectLiveCard steps.
        let mut live_picks: Vec<(String, String, String)> = Vec::new();
        let mut total_decisions = 0usize;
        let mut total_diverged = 0usize;
        let mut my_wins = 0usize;

        for game in 0..diverge_games {
            rabuka_engine::rng::seed(seed.wrapping_add(game));
            let mut gs = deal(&db, &t1, &t2);
            let mut last_key = String::new();
            let mut repeats = 0u32;
            for _ in 0..3000 {
                TurnEngine::check_victory_condition(&mut gs);
                if gs.game_result != GameResult::Ongoing {
                    break;
                }
                // Without a repeat guard a phase that never advances burns the
                // whole step budget and the game silently "finishes" unfinished.
                let key = format!("{:?}:{}", gs.current_phase, gs.active_player().id);
                if key == last_key {
                    repeats += 1;
                    if repeats > 200 {
                        break;
                    }
                } else {
                    last_key = key;
                    repeats = 0;
                }
                if game_setup::auto_advance_one(&mut gs) {
                    continue;
                }
                let actions = game_setup::generate_possible_actions(&gs);
                if actions.is_empty() {
                    TurnEngine::advance_phase(&mut gs);
                    continue;
                }
                let active_me = if gs.active_player().id == "p1" { 0u8 } else { 1u8 };
                let chosen = route(&gs, &actions, active_me, &db, active_me == my_me);
                if active_me == my_me {
                    total_decisions += 1;
                    *phase_tally
                        .entry(format!("{:?}", gs.current_phase))
                        .or_insert(0usize) += 1;
                    let scored = strategy_v7::score_actions(&gs, &actions, my_me);
                    let mut v7_best = 0usize;
                    for i in 0..scored.len() {
                        if scored[i].0 > scored[v7_best].0 {
                            v7_best = i;
                        }
                    }
                    // Record the life each side picked on a SelectLiveCard
                    // step, so "picked a different life" is separable from
                    // "reached the decision at a different point in the search".
                    if actions[v7_best].action_type
                        == game_setup::ActionType::SelectLiveCard
                        && chosen.action_type == game_setup::ActionType::SelectLiveCard
                    {
                        live_picks.push((
                            format!("{:?}", gs.current_phase),
                            describe(&actions[v7_best], &db),
                            describe(&chosen, &db),
                        ));
                    }
                    // Compare signatures, not indices: v8 may return a
                    // canonicalised action whose parameters differ cosmetically
                    // from the generated one, and an index lookup would then
                    // report "no divergence" for every single decision.
                    if signature(&actions[v7_best]) != signature(&chosen) {
                        total_diverged += 1;
                        let me_p = gs.seat_player(my_me);
                        let key = format!(
                            "{:?} -> v7:{} | v8:{}",
                            actions[v7_best].action_type,
                            describe(&actions[v7_best], &db),
                            describe(&chosen, &db)
                        );
                        *tally.entry(key.clone()).or_insert(0) += 1;
                        samples.entry(key).or_insert_with(|| {
                            format!(
                                "  seed {} turn {} energy {} stage cost {}",
                                seed.wrapping_add(game),
                                gs.turn_number,
                                me_p.energy_zone.active_count(),
                                me_p.stage
                                    .stage
                                    .iter()
                                    .filter(|&&c| c >= 0)
                                    .filter_map(|&c| db.get_card(c).and_then(|x| x.cost))
                                    .map(i32::from)
                                    .sum::<i32>()
                            )
                        });
                    }
                }
                let _ = execute_and_settle(&mut gs, &chosen);
            }
            let (a, b) = if my_me == 0 {
                (
                    gs.player1.success_live_card_zone.cards.len(),
                    gs.player2.success_live_card_zone.cards.len(),
                )
            } else {
                (
                    gs.player2.success_live_card_zone.cards.len(),
                    gs.player1.success_live_card_zone.cards.len(),
                )
            };
            if a >= 3 && b <= 2 {
                my_wins += 1;
            }
        }

        println!("=== DIVERGENCE: v8 vs v7 argmax over {diverge_games} games ===");
        println!("v8 (my side) won {my_wins}/{diverge_games}");
        println!(
            "decisions {total_decisions}, v8 disagreed with v7 on {total_diverged} ({:.1}%)",
            100.0 * total_diverged as f64 / total_decisions.max(1) as f64
        );
        println!("decisions by phase: {:?}", phase_tally);
        let mut rows: Vec<(&String, &usize)> = tally.iter().collect();
        rows.sort_by(|a, b| b.1.cmp(a.1));
        for (k, n) in rows.iter().take(16) {
            println!("  {n:>5}x  {k}");
            if let Some(s) = samples.get(*k) {
                println!("        e.g.{s}");
            }
        }

        // Which life each bot actually picks. A `ConfirmLiveSet` vs
        // `SelectLive` disagreement counts a sequencing difference in the
        // search, not a different choice, so the question worth answering is
        // narrower: where the two ARE picking a life, do they pick the same
        // one? That separates "v8 chose differently" from "v8 chose worse",
        // which the aggregate tally above cannot do.
        println!("\n=== life selection: same or different ===");
        let mut picked: BTreeMap<String, (usize, usize)> = BTreeMap::new();
        let mut pick_decisions = 0usize;
        let mut same = 0usize;
        for c in &live_picks {
            pick_decisions += 1;
            if c.1 == c.2 {
                same += 1;
            }
            picked.entry(c.1.clone()).or_insert((0, 0)).0 += 1;
            picked.entry(c.2.clone()).or_insert((0, 0)).1 += 1;
        }
        println!(
            "  life-picking decisions {pick_decisions}, same life chosen {same} ({:.1}%)",
            100.0 * same as f64 / pick_decisions.max(1) as f64
        );
        let mut pr: Vec<(&String, &(usize, usize))> = picked.iter().collect();
        pr.sort_by(|a, b| (b.1.0 + b.1.1).cmp(&(a.1.0 + a.1.1)));
        println!("  {:<24} {:>5} {:>5}", "life", "v7", "v8");
        for (life, (v7, v8)) in pr.iter().take(12) {
            println!("  {life:<24} {v7:>5} {v8:>5}");
        }
        return;
    }


    let mut gs = deal(&db, &t1, &t2);

    let choices: Vec<usize> = std::fs::read_to_string(&script)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.trim().parse::<usize>().ok())
        .collect();

    // --peek: report the opening hand and stop. Two of the first hands dealt
    // had no castable Live at all, which makes the game unwinnable regardless
    // of how it is played. Discovering that one decision at a time is wasteful,
    // so this answers "is this seed worth playing" in one call.
    if peek {
        // Stop where a mulligan decision is actually on offer, rather than
        // inferring it from the phase: `auto_advance_one` can walk past a
        // mulligan in a single call, and a peek that samples after that sees
        // a post-mulligan hand and reports a different game.
        for _ in 0..60 {
            if game_setup::auto_advance_one(&mut gs) {
                continue;
            }
            let actions = game_setup::generate_possible_actions(&gs);
            if actions.is_empty() {
                // Early in setup the hand is not dealt yet, so "no actions"
                // means "not ready", not "finished". Keep going.
                continue;
            }
            if actions
                .iter()
                .any(|a| a.action_type == game_setup::ActionType::SelectMulligan)
            {
                break;
            }
            let pick = actions
                .iter()
                .find(|a| {
                    matches!(
                        a.action_type,
                        game_setup::ActionType::RockChoice
                            | game_setup::ActionType::PaperChoice
                            | game_setup::ActionType::ScissorsChoice
                            | game_setup::ActionType::ChooseFirstAttacker
                            | game_setup::ActionType::ChooseSecondAttacker
                    )
                })
                .cloned();
            match pick {
                Some(p) => {
                    let _ = execute_and_settle(&mut gs, &p);
                }
                None => break,
            }
        }
        let p = if my_me == 0 { &gs.player1 } else { &gs.player2 };
        let mut total = 0usize;
        for &id in p.hand.cards.iter() {
            let Some(c) = db.get_card(id) else { continue };
            if c.card_type != rabuka_engine::card::CardType::Live {
                continue;
            }
            total += 1;
            println!(
                "  live {}  score={}  need={}",
                short(&db, id),
                c.score.unwrap_or(0),
                need_of(&db, id)
            );
        }
        if total == 0 {
            println!("  (no live cards in the opening hand)");
        }
        println!("seed {seed}: {total} live(s) in hand");
        return;
    }

    let mut used = 0usize;
    let mut decision = 0usize;
    let mut my_log: Vec<String> = Vec::new();
    let mut agree = 0usize;
    let mut differ = 0usize;
    let mut diverged: Vec<String> = Vec::new();

    for _step in 0..3000 {
        TurnEngine::check_victory_condition(&mut gs);
        if gs.game_result != GameResult::Ongoing {
            report(&gs, &db, my_me, &my_log, agree, differ, &diverged, auto);
            return;
        }
        if game_setup::auto_advance_one(&mut gs) {
            continue;
        }
        let actions = game_setup::generate_possible_actions(&gs);
        if actions.is_empty() {
            TurnEngine::advance_phase(&mut gs);
            continue;
        }

        let active_me = if gs.active_player().id == "p1" { 0u8 } else { 1u8 };

        if active_me != my_me {
            // v7's turn. Deterministic, so no script needed.
            let a = route(&gs, &actions, opp_me, &db, false);
            if !quiet {
                println!("  v7 plays {}", describe(&a, &db));
            }
            let _ = execute_and_settle(&mut gs, &a);
            continue;
        }

        // My turn.
        decision += 1;
        {
            let p = if my_me == 0 { &gs.player1 } else { &gs.player2 };
            publish_hand(&p.hand.cards);
        }
        let picked = if auto {
            let a = strategy_v8::choose_action_v8_entry(&gs, &actions, my_me);
            let idx = actions
                .iter()
                .position(|x| signature(x) == signature(&a))
                .unwrap_or(0);
            idx
        } else if used < choices.len() {
            let idx = choices[used];
            used += 1;
            idx.min(actions.len() - 1)
        } else {
            // Out of script: hand control back and stop.
            println!("\n############ DECISION {decision} NEEDED ############");
            render(&gs, &db, my_me);
            println!("\nOPTIONS (type the index you want into the script):");
            for (n, a) in actions.iter().enumerate() {
                println!("  [{n:>2}] {}", describe(a, &db));
                if std::env::var_os("HUMAN_RAW").is_some() {
                    println!("        raw params: {:?}", a.parameters);
                }
            }
            let v8a = strategy_v8::choose_action_v8_entry(&gs, &actions, my_me);
            let v8_idx = actions
                .iter()
                .position(|x| signature(x) == signature(&v8a));
            if let Some(idx) = v8_idx {
                println!(
                    "\n  v8 would play [{idx}]: {}   <-- v8's pick",
                    describe(&v8a, &db)
                );
            }

            // v8's own live-set model, printed with the numbers it uses.
            //
            // The point of a hand-played game is seeing WHY the opponent took
            // the action it took, in its own units, rather than only what it
            // did. Every previous look at v8's live set was an aggregate -
            // fold rate, pace - and an aggregate cannot distinguish "v8 folded
            // because it thought the check would fail" from "v8 folded because
            // it thought the comparison was lost", which are opposite problems.
            // Calibrating that model against the engine's own verdicts showed
            // the second case was the live one for the FIRST attacker: v8 read
            // the not-yet-set opponent zone as a fold and believed a passed
            // check was a free placement.
            if std::env::var_os("V8_MODEL").is_some() {
                let live_phase = matches!(
                    gs.current_phase,
                    Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker
                );
                if live_phase {
                    if let Some(pred) = strategy_v8::predict_live_set_v8(&gs, &db) {
                        let role = if gs.current_phase == Phase::LiveCardSetFirstAttacker {
                            "FIRST attacker - their set does not exist yet, so it is FORECAST"
                        } else {
                            "SECOND attacker - their set size is readable (8.4.3.2)"
                        };
                        println!("\n  === v8's live-set model [{role}] ===");
                        println!(
                            "    successes: ours {}  theirs {}   their set size {}",
                            pred.my_success, pred.opp_success, pred.opp_set_size
                        );
                        println!(
                            "    our board {} hearts, {} active blades, the zone needs {}",
                            pred.board_hearts,
                            board_blades(&gs, &gs.seat_player(my_me).id),
                            pred.need_hearts
                        );
                        println!(
                            "    P(our check passes) {:.3}   P(they pass) {:.3}   P(WE place) {:.3}   \
                             P(THEY place) {:.3}",
                            pred.p_pass, pred.opp_pass, pred.p_place, pred.opp_place
                        );
                        println!(
                            "    chosen value {:.4} over {} candidate zones; margin over runner-up {}",
                            pred.value,
                            pred.n_candidates,
                            pred.runner_up.as_ref().map_or("n/a".to_string(), |(_, v, _)| {
                                format!("{:.4}", (pred.value - v).abs())
                            })
                        );
                        println!(
                            "    its choice: lives at hand {:?}, junk at {:?}; {} life(s) priceable in hand",
                            pred.lives, pred.junk, pred.n_lives_in_hand
                        );
                        if let Some((idx, v, pp)) = &pred.runner_up {
                            println!("    runner-up: lives at {idx:?}, value {v:.4}, P(place) {pp:.3}");
                        }
                    }
                }
            }

            // v7's own scoring of every option, as a reference ranking. v7's
            // Main phase is the best-measured one in the project, so where its
            // argmax disagrees with v8's pick is where to look for a real
            // defect rather than a matter of taste.
            if std::env::var_os("V7_SCORES").is_some() {
                let scored = strategy_v7::score_actions(&gs, &actions, my_me);
                let mut best = 0usize;
                for (i, (_, _)) in scored.iter().enumerate() {
                    if scored[i].0 > scored[best].0 {
                        best = i;
                    }
                }
                println!("\n  v7's ranking (its argmax is [{best}]):");
                let mut rows: Vec<(usize, f64, String)> = scored
                    .iter()
                    .enumerate()
                    .map(|(i, (v, label))| (i, *v, label.clone()))
                    .collect();
                rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                for (i, v, label) in rows.iter().take(8) {
                    let mark = if *i == best { " <= v7 argmax" } else { "" };
                    let v8mark = if Some(*i) == v8_idx { "   [v8's pick]" } else { "" };
                    println!("    [{i:>2}] {v:>12.4}  {label}{mark}{v8mark}");
                }
            }
            println!("\n(append ONE more choice to the script, then re-run)");
            println!("choices already consumed: {used}");
            return;
        };

        let a = &actions[picked];
        let v8a = strategy_v8::choose_action_v8_entry(&gs, &actions, my_me);
        let v8_matches = actions
            .iter()
            .position(|x| signature(x) == signature(&v8a))
            .is_some_and(|idx| idx == picked);
        if v8_matches {
            agree += 1;
        } else {
            differ += 1;
            if diverged.len() < 40 {
                diverged.push(format!(
                    "  decision {decision}: v8 chose {} | I/v8-alt chose {}",
                    describe(&v8a, &db),
                    describe(a, &db)
                ));
            }
        }
        let line = format!("D{decision} {}", describe(a, &db));
        println!("  >> {line}");
        my_log.push(line);
        let _ = execute_and_settle(&mut gs, a);
        // Selection diagnostics: the mulligan/live selections are engine-side
        // state, not carried on the action, so a wrong pick here is invisible
        // unless it is printed. This caught a mulligan that silently selected
        // nothing.
        if std::env::var_os("HUMAN_SEL").is_some() {
            let me_p = if my_me == 0 { &gs.player1 } else { &gs.player2 };
            let op_p = if my_me == 0 { &gs.player2 } else { &gs.player1 };
            println!(
                "     [state] phase={:?} mull_sel={:?} my_hand={:?} opp_hand={:?} active={}",
                gs.current_phase,
                gs.mulligan_selected_indices,
                me_p.hand.cards,
                op_p.hand.cards,
                gs.active_player().id
            );
        }
    }
    report(&gs, &db, my_me, &my_log, agree, differ, &diverged, auto);
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

/// Identity of an action for comparison purposes.
///
/// This must include the card AND the target area. An earlier version keyed
/// only on (type, card_id, card_indices), which made a difference in
/// `stage_area_index` - a different member slot - read as agreement. That
/// silently reported zero divergences, which is worse than no tool at all
/// because it looks like a result.
/// Route a decision to the right entry point for a bot, by phase.
///
/// Every generation exposes three separate entry points - main action, live
/// set, mulligan - and calling the main-phase one for everything is the bug
/// that made the first divergence run report 0 wins and 0 disagreements: the
/// live set and mulligan were being decided by a Main-phase chooser, so no
/// game ever resolved. `bot_arena` routes the same way.
fn route(
    gs: &GameState,
    actions: &[game_setup::Action],
    me: u8,
    db: &CardDatabase,
    v8: bool,
) -> game_setup::Action {
    use rabuka_engine::core::types::Phase;
    use rabuka_engine::bot::{choose_live_set_v7, choose_live_set_v8, choose_mulligan_v7,
                             choose_mulligan_v8};
    match gs.current_phase {
        // RPS needs special handling, and the reason is specific to how this
        // fork drives the phase headlessly: only player1 is ever offered the
        // choice, and the phase advances when player1's gesture CHANGES from
        // the previous round - that is what stands in for the second seat.
        // Re-picking the same gesture is a tie that re-offers, so an
        // unattended run sits in RockPaperScissors forever. Every earlier
        // attempt of this harness burned 100% of its decisions here.
        //
        // Cycling the gesture is therefore not a shortcut, it is the only way
        // to drive the phase from a single seat. `bot_arena` does not hit this
        // because it seeds the RNG per game and the bots' own choices vary.
        Phase::RockPaperScissors => {
            use std::sync::atomic::{AtomicUsize, Ordering};
            static RPS: AtomicUsize = AtomicUsize::new(0);
            let cycle = [
                game_setup::ActionType::RockChoice,
                game_setup::ActionType::PaperChoice,
                game_setup::ActionType::ScissorsChoice,
            ];
            let i = RPS.fetch_add(1, Ordering::Relaxed) % cycle.len();
            actions
                .iter()
                .find(|a| a.action_type == cycle[i])
                .or_else(|| actions.first())
                .cloned()
                .unwrap_or_else(|| unreachable!("RPS offers at least one gesture"))
        }
        Phase::MulliganFirstAttacker | Phase::MulliganSecondAttacker => {
            if v8 { choose_mulligan_v8(gs, actions, db) } else { choose_mulligan_v7(gs, actions, db) }
        }
        Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker => {
            if v8 { choose_live_set_v8(gs, actions, db) } else { choose_live_set_v7(gs, actions, db) }
        }
        _ => {
            if v8 {
                strategy_v8::choose_action_v8_entry(gs, actions, me)
            } else {
                strategy_v7::choose_action_v7(gs, actions, me)
            }
        }
    }
}

fn signature(a: &game_setup::Action) -> String {
    let p = a.parameters.as_ref();
    format!(
        "{:?}|id={:?}|idx={:?}|area={:?}|baton={:?}|abilities={:?}",
        a.action_type,
        p.and_then(|p| p.card_id),
        p.and_then(|p| p.card_index),
        p.and_then(|p| p.stage_area_index),
        p.and_then(|p| p.use_baton_touch),
        p.and_then(|p| p.ability_index),
    )
}

fn short(db: &CardDatabase, id: i16) -> String {
    match db.get_card(id) {
        None => format!("#{id}"),
        Some(c) => {
            let n: String = c.name.chars().take(10).collect();
            format!("{n}")
        }
    }
}

fn describe(a: &game_setup::Action, db: &CardDatabase) -> String {
    let p = a.parameters.as_ref();
    let card = p
        .and_then(|p| p.card_id)
        .map(|c| short(db, c))
        .unwrap_or_default();
    let idx = p
        .and_then(|p| p.card_indices.clone())
        .map(|v| format!("{v:?}"))
        .unwrap_or_default();
    match a.action_type {
        game_setup::ActionType::PlayMemberToStage => {
            // PlayMember actions usually identify the card by its position in
            // hand, not by id. Resolving the id lets the option list actually
            // distinguish the members, which is the whole point of the harness.
            let hand_idx = p.and_then(|p| p.card_index);
            let resolved = p
                .and_then(|p| p.card_id)
                .map(|c| short(db, c))
                .or_else(|| {
                    hand_idx
                        .and_then(|i| {
                            CURRENT_HAND
                                .get()
                                .and_then(|h| h.lock().ok())
                                .and_then(|h| h.get(i).copied())
                        })
                        .map(|c| short(db, c))
                })
                .unwrap_or_else(|| format!("hand#{hand_idx:?}"));
            format!(
                "PlayMember {resolved} area={:?} baton={:?}",
                p.and_then(|p| p.stage_area.clone()),
                p.and_then(|p| p.use_baton_touch)
            )
        }
        game_setup::ActionType::SelectLiveCard => format!("SelectLive {card}"),
        game_setup::ActionType::ConfirmLiveCardSet => {
            format!("ConfirmLiveSet {idx}")
        }
        game_setup::ActionType::Pass => "Pass".to_string(),
        other => format!("{other:?} {card}"),
    }
}

/// The acting player's hand, published so `describe` can resolve a
/// hand-indexed card to a name. Scoped to one decision point.
static CURRENT_HAND: std::sync::OnceLock<std::sync::Mutex<Vec<i16>>> = std::sync::OnceLock::new();

fn publish_hand(hand: &[i16]) {
    let cell = CURRENT_HAND.get_or_init(|| std::sync::Mutex::new(Vec::new()));
    if let Ok(mut h) = cell.lock() {
        *h = hand.to_vec();
    }
}

fn hearts_of(db: &CardDatabase, id: i16) -> String {
    db.get_card(id)
        .and_then(|c| c.base_heart.as_ref())
        .map(|h| {
            h.hearts
                .iter()
                .map(|(k, v)| format!("{:?}{v}", k.to_string().chars().last().unwrap_or('?')))
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default()
}

fn need_of(db: &CardDatabase, id: i16) -> String {
    db.get_card(id)
        .and_then(|c| c.need_heart.as_ref())
        .map(|h| {
            h.hearts
                .iter()
                .map(|(k, v)| format!("{:?}{v}", k.to_string().chars().last().unwrap_or('?')))
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default()
}

/// Render only what a fair player can see: my own zones in full, the
/// opponent's stage and zones, never their hand. The opponent's hand is not
/// shown because reading it would make the result meaningless.
fn render(gs: &GameState, db: &CardDatabase, me: u8) {
    let (mine, theirs) = if me == 0 {
        (&gs.player1, &gs.player2)
    } else {
        (&gs.player2, &gs.player1)
    };
    println!(
        "\n  TURN {}  PHASE {:?}  I am {}",
        gs.turn_number,
        gs.current_phase,
        if gs.active_player().id == (if me == 0 { "p1" } else { "p2" }) {
            " ACTIVE"
        } else {
            "waiting"
        }
    );
    println!("  ME   success zone {} / 3   energy {}", mine.success_live_card_zone.cards.len(), mine.energy_zone.active_count());
    println!("  OPP  success zone {} / 3   (their hand is hidden)", theirs.success_live_card_zone.cards.len());
    for (label, p) in [("ME ", mine), ("OPP", theirs)] {
        let stage: Vec<String> = p
            .stage
            .stage
            .iter()
            .map(|&id| {
                if id < 0 {
                    "[empty]".to_string()
                } else {
                    let c = db.get_card(id);
                    let cost = c.and_then(|c| c.cost).unwrap_or(0);
                    let blade = c.map_or(0, |c| c.blade);
                    let wait = gs.mods.get_orientation_modifier(id) == Some("wait");
                    format!(
                        "{}[c{cost} b{blade}{}{}]",
                        short(db, id),
                        if wait { " WAIT" } else { "" },
                        if hearts_of(db, id).is_empty() {
                            String::new()
                        } else {
                            format!(" h:{}", hearts_of(db, id))
                        }
                    )
                }
            })
            .collect();
        println!("  {label} stage: {}", stage.join("  |  "));
    }
    if me == 0 || true {
        let mut hand = String::new();
        for &id in mine.hand.cards.iter() {
            let c = db.get_card(id);
            let ty = c.map(|c| format!("{:?}", c.card_type)).unwrap_or_default();
            let cost = c.and_then(|c| c.cost).unwrap_or(0);
            let blade = c.map_or(0, |c| c.blade);
            if ty.contains("Member") {
                hand.push_str(&format!(
                    " {} [c{cost} b{blade}{}]",
                    short(db, id),
                    if hearts_of(db, id).is_empty() {
                        String::new()
                    } else {
                        format!(" h:{}", hearts_of(db, id))
                    }
                ));
            } else if ty.contains("Live") {
                let score = c.and_then(|c| c.score).unwrap_or(0);
                hand.push_str(&format!(" *{} s{score} n:{}", short(db, id), need_of(db, id)));
            } else {
                hand.push_str(&format!(" ?{}", short(db, id)));
            }
        }
        println!("  ME  hand:{hand}");
    }
    let live_zone: Vec<String> = mine
        .live_card_zone
        .cards
        .iter()
        .map(|&id| format!("{} n:{}", short(db, id), need_of(db, id)))
        .collect();
    if !live_zone.is_empty() {
        println!("  ME  live zone: {}", live_zone.join("  |  "));
    }
    let succ: Vec<String> = mine
        .success_live_card_zone
        .cards
        .iter()
        .map(|&id| short(db, id))
        .collect();
    if !succ.is_empty() {
        println!("  ME  placed so far: {}", succ.join(", "));
    }
}

fn report(
    gs: &GameState,
    db: &CardDatabase,
    me: u8,
    log: &[String],
    agree: usize,
    differ: usize,
    diverged: &[String],
    auto: bool,
) {
    let (mine, theirs) = if me == 0 {
        (&gs.player1, &gs.player2)
    } else {
        (&gs.player2, &gs.player1)
    };
    let a = mine.success_live_card_zone.cards.len();
    let b = theirs.success_live_card_zone.cards.len();
    println!("\n================ RESULT ================");
    println!(
        "  {:?}   ME {a} - OPP {b}   turns {}",
        gs.game_result,
        gs.turn_number
    );
    if a >= 3 && b <= 2 {
        println!("  >>> I WON");
    } else if b >= 3 && a <= 2 {
        println!("  >>> I LOST to v7");
    }
    if !auto {
        println!("  my decisions: {}", log.len());
        println!("  agreement with v8: {agree}   divergence: {differ}");
        if !diverged.is_empty() {
            println!("  where v8 differs from the played line:");
            for d in diverged.iter().take(25) {
                println!("{d}");
            }
        }
    }
    let _ = db;
}
