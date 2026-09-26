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
use rabuka_engine::game_setup;
use rabuka_engine::turn::TurnEngine;
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut seed = 11u32;
    let mut deck = "5CP3Z idou".to_string();
    let mut side = "p1".to_string();
    let mut script = String::from("my_script.txt");
    let mut auto = false;
    let mut quiet = false;
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
    let mut gs = deal(&db, &t1, &t2);

    let choices: Vec<usize> = std::fs::read_to_string(&script)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.trim().parse::<usize>().ok())
        .collect();

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
            let a = strategy_v7::choose_action_v7(&gs, &actions, opp_me);
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
            if let Some(idx) = actions
                .iter()
                .position(|x| signature(x) == signature(&v8a))
            {
                println!(
                    "\n  v8 would play [{idx}]: {}   <-- v8 is behind you",
                    describe(&v8a, &db)
                );
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

fn signature(a: &game_setup::Action) -> String {
    format!(
        "{:?}|{:?}|{:?}",
        a.action_type,
        a.parameters.as_ref().and_then(|p| p.card_id),
        a.parameters
            .as_ref()
            .and_then(|p| p.card_indices.clone())
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
