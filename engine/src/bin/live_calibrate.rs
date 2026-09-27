//! Live-check CALIBRATION harness: does v8's outcome model predict what the
//! engine then does?
//!
//! Why this exists. Every measurement the project has about v8's live set is
//! an aggregate - win rate, fold rate, placements per live phase. Those say
//! *that* the live set leaves placements on the table (pace 0.42 against the
//! guide's ~1.0) and none of them say **which model input is wrong**. There are
//! two very different diagnoses and they call for opposite work:
//!
//! - The model is **well calibrated** and pace is low because the checks are
//!   genuinely contested. Then the fix is development and board construction,
//!   and no live-set work will help.
//! - `p_pass` or `p_place` is **systematically miscalibrated**. Then the
//!   argmax is ranking candidates on a lie, every fold/сontest decision is
//!   mistimed, and the fix is the model.
//!
//! Nothing in the project can currently tell these apart. `YIELD_PRIOR` - five
//! hand-picked (density, yield) pairs standing in for the opponent's flip
//! yield - has never been checked against a single real check. That is the
//! concrete gap this closes.
//!
//! How it works. Ground truth is engine state, not a log line:
//! `GameState::performance_snapshots` carries, per seat, every life's
//! `passed` flag, the aggregate `success`, and `total_score`. So at each live
//! set the harness records what v8 predicted, and once the check has resolved it
//! reads the snapshot back and joins the two. No `--logs`, no `--trace`, so
//! section 8.4's logging-perturbs-gameplay caveat does not apply to the
//! recorded numbers (the prediction is a pure function of the decision state
//! and is recomputed rather than captured, so it cannot perturb anything).
//!
//! Usage:
//!   live_calibrate --games 200 --seed 11 --deck "5CP3Z idou"
//!   live_calibrate --games 200 --out calibration.csv     # per-check rows
//!
//! Flags: `--p1`/`--p2` bot names (default v8 / v7), `--deck2` for an
//! asymmetric opponent, `--csv PATH` to write the raw rows.

use rabuka_engine::bin_common::{fresh_database, load_deck};
use rabuka_engine::bot::{registry::BotKind, strategy_v8};
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::rng::Lcg;
use rabuka_engine::turn::TurnEngine;
use std::collections::BTreeMap;
use std::sync::Arc;

/// One live check, joined: what the model said and what the engine did.
#[derive(Clone, Debug)]
struct Row {
    turn: u8,
    seat: usize,
    set_size: usize,
    lives: usize,
    junk: usize,
    /// Lives that were in hand and priceable at this decision. A zone of zero
    /// lives with a positive count here is a fold that threw away a real
    /// option, which is the waste this harness exists to size.
    lives_in_hand: usize,
    my_success: usize,
    opp_success: usize,
    opp_set_size: usize,
    p_pass: f64,
    p_place: f64,
    opp_place: f64,
    opp_pass: f64,
    value: f64,
    margin: f64,
    board_hearts: i32,
    need_hearts: i32,
    // Truth, from `performance_snapshots`.
    passed: bool,
    placed: bool,
    score: i32,
    opp_passed: bool,
    opp_placed: bool,
    opp_score: i32,
    // -- Engine truth about SUPPLY, so the model's own supply estimate can be
    // checked rather than assumed. These are what makes the flip yield
    // derivable instead of hand-picked: `total_hearts` is the pool the check
    // actually ran on and `yell_count` is how many cards were flipped to build
    // it, so (total - board) / yell is the realised hearts-per-flip.
    our_supply: i32,
    our_yell: i32,
    our_required: i32,
    opp_supply: i32,
    opp_yell: i32,
    opp_required: i32,
    // Public board totals, which is all the policy is allowed to know.
    our_board: i32,
    our_blades: i32,
    opp_board: i32,
    opp_blades: i32,
    /// Whether we were the FIRST attacker when the zone was committed. This is
    /// the load-bearing split in the whole report and it is not a detail: sets
    /// are committed face-down in attacker order (8.2.2), so the first attacker
    /// physically cannot see what the second attacker will do, while the
    /// second attacker sees the first attacker's set SIZE (8.4.3.2). Any
    /// opponent model that reads the live zone as a commitment is therefore
    /// reading an empty zone for the first attacker, and an empty zone is the
    /// normal state, not a fold.
    first_attacker: bool,
    /// The opponent's ACTUAL set size for this check, read from the snapshot
    /// after the fact. Post-hoc ground truth for calibration only - never
    /// available to the policy at decision time, which is the entire point of
    /// the first-attacker rows.
    opp_set_actual: usize,
}

fn game_seeds(base: u32, game: u32) -> u32 {
    base.wrapping_add(game.wrapping_sub(1)).max(1)
}

/// A pending prediction, waiting for the check to resolve.
struct Pending {
    row: Row,
    /// Success-zone sizes when the check was committed, so a placement is a
    /// DIFF and not an absolute.
    z_me: usize,
    z_them: usize,
}

/// The engine's verdict for the check that just resolved.
///
/// The most recent snapshot for that seat, with NO turn filter. A live set for
/// turn N is followed by the check and then by turn N+1's Active/Energy/Draw/
/// Main phases before the next live set, so by the time a prediction is joined
/// to its outcome `gs.turn_number` has already moved past the snapshot's turn.
/// Filtering on the turn therefore matched nothing, `passed` came back false
/// for every row, and the harness reported a 0.4% pass rate against a 27.7%
/// placement rate - two numbers that cannot both be true, because a placement
/// requires a won check, which requires passing (8.4.7). The contradiction is
/// what exposed the bug; the placement column was reading a success-zone DIFF
/// and was therefore correct throughout.
fn snapshot_for<'a>(
    gs: &'a GameState,
    pid: &str,
) -> Option<&'a rabuka_engine::types::PerformanceSnapshot> {
    gs.performance_snapshots.iter().rev().find(|s| s.player_id == pid)
}

/// Public board heart total for a seat: what a fair player can see on stage.
fn board_total(gs: &GameState, pid: &str) -> i32 {
    let stage = if gs.player1.id == pid {
        &gs.player1.stage.stage
    } else {
        &gs.player2.stage.stage
    };
    let db = &gs.card_database;
    stage
        .iter()
        .filter(|&&c| c >= 0)
        .filter_map(|&c| db.get_card(c).and_then(|card| card.base_heart.as_ref()))
        .map(|b| b.hearts.values_sum() as i32)
        .sum()
}

/// Public active-blade count for a seat (Q133: the yell count is active members
/// only, and which members are active is public).
fn board_blades(gs: &GameState, pid: &str) -> i32 {
    let stage = if gs.player1.id == pid {
        &gs.player1.stage.stage
    } else {
        &gs.player2.stage.stage
    };
    let db = &gs.card_database;
    stage
        .iter()
        .filter(|&&c| c >= 0)
        .filter_map(|&c| db.get_card(c).map(|card| i32::from(card.blade)))
        .sum()
}

fn v8_model_blades(gs: &GameState, me: u8) -> i32 {
    board_blades(gs, &gs.seat_player(me).id)
}

/// Brier score: mean squared error of a probability forecast. 0 is perfect, 0.25
/// is what a constant 0.5 forecast scores, and it is the number to compare two
/// models on - a model that is badly calibrated can still have a good
/// rank-ordering, and vice versa.
fn brier(rows: &[(f64, bool)]) -> f64 {
    if rows.is_empty() {
        return f64::NAN;
    }
    rows.iter()
        .map(|(p, y)| (p - if *y { 1.0 } else { 0.0 }).powi(2))
        .sum::<f64>()
        / rows.len() as f64
}

/// Calibration table: bucket forecasts into deciles and print predicted vs
/// observed. A calibrated model's observed rate tracks the predicted rate down
/// the column; a model that is consistently optimistic sits above the diagonal
/// everywhere, and that is a bias a decision rule can compensate for.
fn calibration_table(label: &str, rows: &[(f64, bool)]) {
    if rows.is_empty() {
        println!("  {label}: no rows");
        return;
    }
    println!(
        "  {label}: n={} Brier={:.4} (0.25 = a constant-0.5 forecast) mean_pred={:.3} observed={:.3}",
        rows.len(),
        brier(rows),
        rows.iter().map(|(p, _)| p).sum::<f64>() / rows.len() as f64,
        rows.iter().filter(|(_, y)| *y).count() as f64 / rows.len() as f64
    );
    let mut buckets: BTreeMap<usize, (usize, f64, usize)> = BTreeMap::new();
    for &(p, y) in rows {
        let b = ((p.clamp(0.0, 0.999_999) * 10.0) as usize).min(9);
        let entry = buckets.entry(b).or_insert((0, 0.0, 0));
        entry.0 += 1;
        entry.1 += p;
        entry.2 += usize::from(y);
    }
    println!("    pred-bucket      n   predicted   observed   bias");
    for (b, (n, sum, hits)) in buckets {
        let predicted = sum / n as f64;
        let observed = hits as f64 / n as f64;
        println!(
            "    {b}.{:<9} {:>6}   {predicted:>8.3}   {observed:>8.3}   {:>+7.3}",
            "0", n, observed - predicted
        );
    }
}

fn fmt4(v: f64) -> String {
    if v.is_finite() {
        format!("{v:.4}")
    } else {
        "inf".to_string()
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut games = 200u32;
    let mut seed = 11u32;
    let mut deck = "5CP3Z idou".to_string();
    let mut deck2: Option<String> = None;
    let mut p1 = "v8".to_string();
    let mut p2 = "v7".to_string();
    let mut csv: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        let need = |i: usize| -> Result<String, Box<dyn std::error::Error>> {
            args.get(i + 1)
                .cloned()
                .ok_or_else(|| format!("missing value for {}", args[i]).into())
        };
        match args[i].as_str() {
            "--games" => {
                games = need(i)?.parse()?;
                i += 1;
            }
            "--seed" => {
                seed = need(i)?.parse()?;
                i += 1;
            }
            "--deck" => {
                deck = need(i)?;
                i += 1;
            }
            "--deck2" => {
                deck2 = Some(need(i)?);
                i += 1;
            }
            "--p1" => {
                p1 = need(i)?;
                i += 1;
            }
            "--p2" => {
                p2 = need(i)?;
                i += 1;
            }
            "--csv" => {
                csv = Some(need(i)?);
                i += 1;
            }
            other => return Err(format!("unknown flag {other}").into()),
        }
        i += 1;
    }
    let k1 = BotKind::parse(&p1);
    let k2 = BotKind::parse(&p2);

    game_setup::set_action_display(false);
    // Deliberately OFF. Section 8.4: turning engine logging on changes outcomes
    // for the same seed. This harness reports MODEL QUALITY, not win rate, and
    // it reads engine state rather than log lines - but keeping logs off means
    // even the recorded numbers match an untraced run.
    game_setup::set_logging_enabled(false);

    let mut db: Arc<rabuka_engine::card::CardDatabase> = fresh_database();
    let nums = load_deck(&deck);
    let nums2 = match deck2.as_deref() {
        Some(other) if other != deck => load_deck(other),
        _ => nums.clone(),
    };
    let (t1, t2) = game_setup::build_two_decks(&mut db, &nums, &nums2).expect("build decks");

    let v2 = rabuka_engine::bot::strategy_v2::V2Policy::default();
    let mut rows: Vec<Row> = Vec::new();
    let mut p1_wins = 0usize;
    let mut p2_wins = 0usize;

    for g in 1..=games {
        let engine_seed = game_seeds(seed, g);
        rabuka_engine::rng::seed(engine_seed);
        let mut arena_rng = Lcg(0x5EED_1234_ABCD_0001 ^ u64::from(engine_seed));
        let mut d1 = t1.clone();
        let mut d2 = t2.clone();
        d1.shuffle_main_deck();
        d1.shuffle_energy_deck();
        d2.shuffle_main_deck();
        d2.shuffle_energy_deck();
        let mut p = rabuka_engine::player::Player::new("p1".into(), "P1".into(), true);
        let mut q = rabuka_engine::player::Player::new("p2".into(), "P2".into(), false);
        p.set_main_deck(d1.main_deck);
        p.set_energy_deck(d1.energy_deck);
        q.set_main_deck(d2.main_deck);
        q.set_energy_deck(d2.energy_deck);
        let mut gs = GameState::new(p, q, Arc::clone(&db));
        game_setup::setup_game(&mut gs);

        let plan1 = rabuka_engine::bot::strategy_v3::V3Plan::detect(&gs, 0, &db);
        let plan2 = rabuka_engine::bot::strategy_v3::V3Plan::detect(&gs, 1, &db);

        let mut pending: Vec<Pending> = Vec::new();
        let mut last_key = String::new();
        let mut repeats = 0u32;

        for _ in 0..600 {
            TurnEngine::check_victory_condition(&mut gs);
            if gs.game_result != GameResult::Ongoing {
                break;
            }
            let key = format!("{:?}:{}", gs.current_phase, gs.active_player().id);
            if key == last_key {
                repeats += 1;
                if repeats > 200 {
                    break;
                }
            } else {
                last_key = key.clone();
                repeats = 0;
            }
            // A new check begins: everything still pending has resolved, because
            // the only thing that resolves a check is reaching the next one.
            if gs.current_phase == Phase::LiveCardSetFirstAttacker && !pending.is_empty() {
                resolve(&mut pending, &gs, &mut rows);
            }
            if game_setup::auto_advance_one(&mut gs) {
                continue;
            }
            let actions = game_setup::generate_possible_actions(&gs);
            if actions.is_empty() {
                TurnEngine::advance_phase(&mut gs);
                continue;
            }
            if gs.current_phase == Phase::RockPaperScissors {
                let a = &actions[arena_rng.range(actions.len())];
                let _ = game_setup::execute_action(&mut gs, a);
                game_setup::settle_single_player_state(&mut gs);
                continue;
            }
            if gs.current_phase == Phase::ChooseFirstAttacker {
                let a = &actions[arena_rng.range(actions.len())];
                let _ = game_setup::execute_action(&mut gs, a);
                game_setup::settle_single_player_state(&mut gs);
                continue;
            }

            let is_p1 = gs.active_player().id == gs.player1.id;
            let me = u8::from(!is_p1);
            let kind = if is_p1 { k1 } else { k2 };
            let live_phase = matches!(
                gs.current_phase,
                Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker
            );
            // Only v8 is modelled. v7 has no equivalent prediction, so a row
            // would be half-facts; the v7 side is a fixed opponent, not a
            // subject of the report.
            if live_phase && kind == BotKind::V8 {
                if let Some(pred) = strategy_v8::predict_live_set_v8(&gs, &db) {
                    let (mine, theirs) = gs.seated_pair(me);
                    pending.push(Pending {
                        row: Row {
                            turn: gs.turn_number,
                            seat: me as usize,
                            set_size: pred.lives.len(),
                            lives: pred.lives.len(),
                            junk: pred.junk.len(),
                            lives_in_hand: pred.n_lives_in_hand,
                            my_success: pred.my_success,
                            opp_success: pred.opp_success,
                            opp_set_size: pred.opp_set_size,
                            p_pass: pred.p_pass,
                            p_place: pred.p_place,
                            opp_place: pred.opp_place,
                            opp_pass: pred.opp_pass,
                            value: pred.value,
                            margin: pred
                                .runner_up
                                .as_ref()
                                .map_or(f64::INFINITY, |(_, v, _)| (pred.value - v).abs()),
                            board_hearts: pred.board_hearts,
                            need_hearts: pred.need_hearts,
                            passed: false,
                            placed: false,
                            score: 0,
                            opp_passed: false,
                            opp_placed: false,
                            opp_score: 0,
                            our_supply: 0,
                            our_yell: 0,
                            our_required: 0,
                            opp_supply: 0,
                            opp_yell: 0,
                            opp_required: 0,
                            our_board: pred.board_hearts,
                            our_blades: v8_model_blades(&gs, me),
                            opp_board: board_total(&gs, &theirs.id),
                            opp_blades: board_blades(&gs, &theirs.id),
                            first_attacker: gs.current_phase
                                == Phase::LiveCardSetFirstAttacker,
                            opp_set_actual: 0,
                        },
                        z_me: mine.success_live_card_zone.cards.len(),
                        z_them: theirs.success_live_card_zone.cards.len(),
                    });
                }
            }
            let plans = [&plan1, &plan2];
            let chosen = if matches!(
                gs.current_phase,
                Phase::MulliganFirstAttacker | Phase::MulliganSecondAttacker
            ) {
                kind.choose_mulligan(&gs, &actions, &db)
            } else if live_phase {
                kind.choose_live_set(&gs, &actions, &db, &v2, plans[me as usize])
            } else {
                kind.choose_action(&gs, &actions, me, &v2, plans[me as usize])
            };
            let _ = game_setup::execute_action(&mut gs, &chosen);
            game_setup::settle_single_player_state(&mut gs);
        }
        resolve(&mut pending, &gs, &mut rows);
        let z1 = gs.player1.success_live_card_zone.cards.len();
        let z2 = gs.player2.success_live_card_zone.cards.len();
        if z1 >= 3 && z2 <= 2 {
            p1_wins += 1;
        } else if z2 >= 3 && z1 <= 2 {
            p2_wins += 1;
        }
    }

    report(&rows, games, &p1, &p2, p1_wins, p2_wins);
    if let Some(path) = csv {
        let mut out = String::from(
            "turn,seat,lives,junk,lives_in_hand,my_success,opp_success,opp_set_size,p_pass,p_place,opp_place,opp_pass,value,margin,board_hearts,need_hearts,passed,placed,score,opp_passed,opp_placed,opp_score,our_supply,our_yell,our_required,opp_supply,opp_yell,opp_required,our_board,our_blades,opp_board,opp_blades,first_attacker,opp_set_actual\n",
        );
        for r in &rows {
            // Built as a list rather than one 32-placeholder format string: a
            // long positional format is unreadable and, when a column is added,
            // silently misaligns instead of failing to compile.
            let cells: Vec<String> = vec![
                r.turn.to_string(),
                r.seat.to_string(),
                r.lives.to_string(),
                r.junk.to_string(),
                r.lives_in_hand.to_string(),
                r.my_success.to_string(),
                r.opp_success.to_string(),
                r.opp_set_size.to_string(),
                fmt4(r.p_pass),
                fmt4(r.p_place),
                fmt4(r.opp_place),
                fmt4(r.opp_pass),
                fmt4(r.value),
                fmt4(r.margin),
                r.board_hearts.to_string(),
                r.need_hearts.to_string(),
                (r.passed as u8).to_string(),
                (r.placed as u8).to_string(),
                r.score.to_string(),
                (r.opp_passed as u8).to_string(),
                (r.opp_placed as u8).to_string(),
                r.opp_score.to_string(),
                r.our_supply.to_string(),
                r.our_yell.to_string(),
                r.our_required.to_string(),
                r.opp_supply.to_string(),
                r.opp_yell.to_string(),
                r.opp_required.to_string(),
                r.our_board.to_string(),
                r.our_blades.to_string(),
                r.opp_board.to_string(),
                r.opp_blades.to_string(),
                (r.first_attacker as u8).to_string(),
                r.opp_set_actual.to_string(),
            ];
            out.push_str(&cells.join(","));
            out.push('\n');
        }
        std::fs::write(&path, out)?;
        eprintln!("rows written to {path} ({} checks)", rows.len());
    }
    Ok(())
}

/// Join every pending prediction to the engine's verdict for that check.
///
/// The verdict is read from `performance_snapshots`, which the engine fills in
/// during the performance: `lives[].passed` per card, `success` for the whole
/// set, and `total_score` for the comparison. That is the engine's own
/// resolution, not a reimplementation, so the ground truth cannot drift from
/// the rules.
fn resolve(pending: &mut Vec<Pending>, gs: &GameState, rows: &mut Vec<Row>) {
    for mut item in pending.drain(..) {
        let me = item.row.seat;
        let (mine, theirs) = gs.seated_pair(me as u8);
        if let Some(snap) = snapshot_for(gs, &mine.id) {
            item.row.passed = snap.lives.iter().all(|l| l.passed) && !snap.lives.is_empty();
            item.row.score = snap.total_score as i32;
            item.row.our_supply = snap.total_hearts.iter().map(|v| i32::from(*v)).sum();
            item.row.our_yell = i32::from(snap.yell_count);
            item.row.our_required = snap
                .lives
                .iter()
                .map(|l| l.required.iter().map(|v| i32::from(*v)).sum::<i32>())
                .sum();
        }
        if let Some(snap) = snapshot_for(gs, &theirs.id) {
            item.row.opp_passed = snap.lives.iter().all(|l| l.passed) && !snap.lives.is_empty();
            item.row.opp_score = snap.total_score as i32;
            item.row.opp_supply = snap.total_hearts.iter().map(|v| i32::from(*v)).sum();
            item.row.opp_yell = i32::from(snap.yell_count);
            item.row.opp_required = snap
                .lives
                .iter()
                .map(|l| l.required.iter().map(|v| i32::from(*v)).sum::<i32>())
                .sum();
            item.row.opp_set_actual = snap.lives.len();
        }
        item.row.placed = mine.success_live_card_zone.cards.len() > item.z_me;
        item.row.opp_placed = theirs.success_live_card_zone.cards.len() > item.z_them;
        rows.push(item.row);
    }
}

fn report(rows: &[Row], games: u32, p1: &str, p2: &str, w1: usize, w2: usize) {
    println!("=== LIVE-CHECK CALIBRATION: {p1} vs {p2}, {games} games ===");
    println!(
        "v8-side checks recorded: {}  ({:.1} per game)",
        rows.len(),
        rows.len() as f64 / games.max(1) as f64
    );
    if rows.is_empty() {
        println!("  no rows: a tool reporting zero is a bug until proven otherwise");
        return;
    }
    // Self-consistency, checked before anything is interpreted. A placement
    // requires a won check, which requires passing the check (8.4.7), so
    // `placed && !passed` is impossible. Rows that violate it mean the ground
    // truth is being read wrong, and every calibration number below would then
    // be a measurement of the harness rather than of the model.
    let impossible = rows.iter().filter(|r| r.placed && !r.passed).count();
    let passes_without_placing = rows
        .iter()
        .filter(|r| r.passed && !r.placed)
        .count();
    println!(
        "\n-- GROUND-TRUTH SELF-CHECK --\n  placed without passing (impossible): {impossible}\n  \
         passed without placing (a lost comparison): {passes_without_placing}"
    );
    if impossible > 0 {
        println!(
            "  !! {impossible} impossible rows: the engine verdict is not being read correctly. \
             STOP - every number below is void."
        );
        return;
    }
    let ours: Vec<(f64, bool)> = rows.iter().map(|r| (r.p_pass, r.passed)).collect();
    let place: Vec<(f64, bool)> = rows.iter().map(|r| (r.p_place, r.placed)).collect();
    let opp_pass: Vec<(f64, bool)> = rows.iter().map(|r| (r.opp_pass, r.opp_passed)).collect();
    let opp_place: Vec<(f64, bool)> = rows.iter().map(|r| (r.opp_place, r.opp_placed)).collect();

    println!("\n-- OUR p_pass (does the zone clear its own check?) --");
    calibration_table("p_pass", &ours);
    println!("\n-- OUR p_place (does the check become a placement?) --");
    calibration_table("p_place", &place);
    println!("\n-- OPPONENT p_pass (the YIELD_PRIOR, scored) --");
    calibration_table("opp_pass", &opp_pass);
    println!("\n-- OPPONENT p_place (the contested comparison) --");
    calibration_table("opp_place", &opp_place);

    // The pace decomposition. The guide's metric is ~1.0 placements per live
    // phase; splitting it into "committed a life" x "the check converted" x
    // "the comparison was won" says which of the three is losing the games,
    // which the aggregate pace number cannot.
    let committed = rows.iter().filter(|r| r.lives > 0).count();
    let passed = rows.iter().filter(|r| r.passed).count();
    let placed = rows.iter().filter(|r| r.placed).count();
    let n = rows.len();
    println!("\n-- PACE DECOMPOSITION (per v8 live phase) --");
    println!("  committed a life      {committed}/{n} = {:.3}", committed as f64 / n as f64);
    println!("  the check passed      {passed}/{n} = {:.3}", passed as f64 / n as f64);
    println!("  became a placement    {placed}/{n} = {:.3}   (guide target ~1.0)", placed as f64 / n as f64);
    println!(
        "  given a life was set, P(pass) = {:.3}; given it passed, P(place) = {:.3}",
        passed as f64 / committed.max(1) as f64,
        placed as f64 / passed.max(1) as f64
    );
    let won_comparison = rows
        .iter()
        .filter(|r| r.passed && !r.opp_passed)
        .count();
    println!(
        "  of the checks that passed, {won_comparison} were uncontested ({:.3}) and {} were contested",
        won_comparison as f64 / passed.max(1) as f64,
        passed - won_comparison
    );

    // Decision margin: how often is the argmax taken on a hair? A margin below
    // the model's own resolution is a coin flip decided by float noise.
    let finite: Vec<f64> = rows.iter().map(|r| r.margin).filter(|m| m.is_finite()).collect();
    if !finite.is_empty() {
        let mut sorted = finite.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let q = |f: f64| sorted[((sorted.len() as f64 - 1.0) * f).round() as usize];
        println!("\n-- DECISION MARGIN (chosen value minus runner-up) --");
        println!(
            "  median {:.5}  p10 {:.5}  fraction below 0.001: {:.3}",
            q(0.5),
            q(0.1),
            finite.iter().filter(|m| **m < 0.001).count() as f64 / finite.len() as f64
        );
    }

    // Folds with a life in hand, and zones that were committed but never even
    // attempted: the two ways v8 wastes a live phase.
    let fold_with_life = rows
        .iter()
        .filter(|r| r.lives == 0 && r.lives_in_hand > 0)
        .count();
    println!("\n-- WASTED PHASES --");
    println!("  folded while holding a live: {fold_with_life}/{n} = {:.3}", fold_with_life as f64 / n as f64);
    let set_but_failed = rows.iter().filter(|r| r.lives > 0 && !r.passed).count();
    println!(
        "  set a life and failed the check: {set_but_failed}/{committed} = {:.3}",
        set_but_failed as f64 / committed.max(1) as f64
    );
    let placed_contested = rows
        .iter()
        .filter(|r| r.placed && r.opp_passed && r.score >= r.opp_score)
        .count();
    println!(
        "  placements won on a tie or better against a passing opponent: {placed_contested}/{placed}",
        );
    supply_report(rows);
    attacker_order_report(rows);
    println!("\n  (win rate for reference only: {p1} {w1} - {p2} {w2} of {games})");
}

/// Attacker order, and what it costs.
///
/// Sets are committed face-down in attacker order (8.2.2). The second attacker
/// sees the first attacker's set SIZE and can price the comparison (8.4.3.2
/// gives a sole passer the placement regardless of contents). The first
/// attacker sees nothing, because the second attacker's set does not exist yet.
///
/// The model reads the opponent's live zone as their commitment, so for the
/// first attacker "not yet set" and "chose to fold" are the same observation -
/// and an empty zone is the normal state, not a fold. If that is the defect,
/// the first attacker's `p_place` will read far too high and the second
/// attacker's will be roughly right, because only the first attacker is
/// modelling an unobserved quantity as a known zero.
fn attacker_order_report(rows: &[Row]) {
    println!("\n-- ATTACKER ORDER: what each side could know when it committed --");
    for (label, first) in [("first attacker ", true), ("second attacker", false)] {
        let side: Vec<&Row> = rows.iter().filter(|r| r.first_attacker == first).collect();
        if side.is_empty() {
            continue;
        }
        let opp_committed = side.iter().filter(|r| r.opp_set_size > 0).count();
        let opp_actually_passed = side.iter().filter(|r| r.opp_passed).count();
        let we_placed = side.iter().filter(|r| r.placed).count();
        let predicted_place: f64 = side.iter().map(|r| r.p_place).sum::<f64>() / side.len() as f64;
        let predicted_opp: f64 = side.iter().map(|r| r.opp_place).sum::<f64>() / side.len() as f64;
        let actual_opp_place = side.iter().filter(|r| r.opp_placed).count();
        let mean_pred_opp_pass = side.iter().map(|r| r.opp_pass).sum::<f64>() / side.len() as f64;
        let actual_opp_pass = opp_actually_passed as f64 / side.len() as f64;
        println!(
            "  {label} n={:<5} opponent zone already held a card in {opp_committed:<5} ({:.3})\n    \
             model: p_place {predicted_place:.3}  opp_pass {mean_pred_opp_pass:.3}  opp_place {predicted_opp:.3}\n    \
             truth: we placed {we_placed:<5} ({:.3})   they passed {opp_actually_passed:<5} ({actual_opp_pass:.3})   they placed {actual_opp_place:<5} ({:.3})",
            side.len(),
            opp_committed as f64 / side.len() as f64,
            we_placed as f64 / side.len() as f64,
            actual_opp_place as f64 / side.len() as f64,
        );
    }
    let first: Vec<&Row> = rows.iter().filter(|r| r.first_attacker).collect();
    if !first.is_empty() {
        let blind: f64 = first
            .iter()
            .filter(|r| r.opp_set_size == 0 && r.opp_passed)
            .count() as f64
            / first.len() as f64;
        println!(
            "  As first attacker, the model read an empty opponent zone {blind:.3} of the time - and\n  \
             on that share of phases the opponent then committed AND passed. An empty zone is the\n  \
             normal state at that point in the phase order, not evidence of a fold."
        );

        // What a first attacker SHOULD assume, as a function of the opponent's
        // PUBLIC board. This is the table the model needs and cannot currently
        // produce: not a flat commitment rate, but a rate conditioned on what a
        // fair player can see. `board + blades` is the upper bound on the
        // opponent's supply; the flips decide how much of it lands.
        println!("\n-- WHAT A FIRST ATTACKER SHOULD ASSUME, by the opponent's public supply --");
        println!("   opp board+blades   n   committed   passed   mean set size   mean score");
        let mut buckets: BTreeMap<i32, (usize, usize, usize, usize, usize)> = BTreeMap::new();
        for r in &first {
            let key = ((r.opp_board + r.opp_blades) / 3).clamp(0, 8);
            let e = buckets.entry(key).or_insert((0, 0, 0, 0, 0));
            e.0 += 1;
            e.1 += usize::from(r.opp_set_actual > 0);
            e.2 += usize::from(r.opp_passed);
            e.3 += r.opp_set_actual;
            e.4 += r.opp_score.max(0) as usize;
        }
        for (bucket, (n, committed, passed, set, score)) in buckets {
            println!(
                "   {:>13}  {n:>5}   {:.3}      {:.3}    {:.3}            {:.2}",
                format!("~{}", bucket * 3),
                committed as f64 / n as f64,
                passed as f64 / n as f64,
                set as f64 / n as f64,
                score as f64 / n as f64,
            );
        }
        let total_set: usize = first.iter().map(|r| r.opp_set_actual).sum();
        let committed = first.iter().filter(|r| r.opp_set_actual > 0).count();
        println!(
            "  Overall: the second attacker committed on {:.3} of first-attacker phases, with a mean\n  \
             set size of {:.2} and a mean score of {:.2}. v8 currently assumes 0.00 on every one of them.",
            committed as f64 / first.len() as f64,
            total_set as f64 / first.len() as f64,
            first.iter().map(|r| r.opp_score.max(0) as f64).sum::<f64>() / first.len() as f64,
        );
    }
}

/// The realised flip yield, and whether the opponent model's bar is even in
/// the right place.
///
/// The opponent's decklist is not fair information, so the only honest way to
/// model their check is a prior over what their yell yields. v8 ships five
/// hand-picked (yield, weight) pairs for that. This measures the yield the
/// engine actually produced, from engine state: the check ran on a pool of
/// `total_hearts` built from a public board total plus `yell_count` flips, so
/// `(supply - board) / yell` is hearts per flip, realised.
///
/// The two sides are separated deliberately. Our side is modelled with the
/// exact shuffle sampler over our OWN deck, so its yield is known and its
/// `p_pass` should be calibrated. Their side is modelled with the prior. If the
/// two realised yields differ, the gap is the prior's error, and it is
/// measurable rather than arguable.
#[derive(Clone, Copy)]
struct Side {
    label: &'static str,
    board: fn(&Row) -> i32,
    blades: fn(&Row) -> i32,
    supply: fn(&Row) -> i32,
    yell: fn(&Row) -> i32,
    required: fn(&Row) -> i32,
    passed: fn(&Row) -> bool,
    committed: fn(&Row) -> bool,
}

const SIDES: [Side; 2] = [
    Side {
        label: "ours  ",
        board: |r| r.our_board,
        blades: |r| r.our_blades,
        supply: |r| r.our_supply,
        yell: |r| r.our_yell,
        required: |r| r.our_required,
        passed: |r| r.passed,
        committed: |r| r.lives > 0,
    },
    Side {
        label: "theirs",
        board: |r| r.opp_board,
        blades: |r| r.opp_blades,
        supply: |r| r.opp_supply,
        yell: |r| r.opp_yell,
        required: |r| r.opp_required,
        passed: |r| r.opp_passed,
        committed: |r| r.opp_set_size > 0,
    },
];

fn median(values: &mut [i32]) -> f64 {
    values.sort_unstable();
    values.get(values.len() / 2).copied().unwrap_or(0) as f64
}

fn supply_report(rows: &[Row]) {
    println!("\n-- REALISED FLIP YIELD (engine truth, not the model) --");
    for side in SIDES {
        let committed: Vec<&Row> = rows.iter().filter(|r| (side.committed)(r)).collect();
        let mut ratio: Vec<f64> = committed
            .iter()
            .filter(|r| (side.yell)(r) > 0)
            .map(|r| ((side.supply)(r) - (side.board)(r)) as f64 / (side.yell)(r) as f64)
            .collect();
        ratio.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let q = |f: f64| {
            ratio
                .get(((ratio.len() as f64 - 1.0) * f).round() as usize)
                .copied()
                .unwrap_or(0.0)
        };
        let pass_rate = if committed.is_empty() {
            f64::NAN
        } else {
            committed.iter().filter(|r| (side.passed)(r)).count() as f64 / committed.len() as f64
        };
        println!(
            "  {}  committed {:>5}  pass {:>5.3}  hearts/flip p10 {:.3} median {:.3} p90 {:.3}  \
             median board {:>4.1} median supply {:>4.1} median yell {:>4.1}",
            side.label,
            committed.len(),
            pass_rate,
            q(0.1),
            q(0.5),
            q(0.9),
            median(&mut rows.iter().map(side.board).collect::<Vec<_>>()),
            median(&mut rows.iter().map(side.supply).collect::<Vec<_>>()),
            median(&mut rows.iter().map(side.yell).collect::<Vec<_>>()),
        );
    }
    println!("  (v8's YIELD_PRIOR puts the opponent's mean at 0.53 hearts/flip, max 1.30)");

    // The pass rate as a function of how far the board fell short of the
    // requirement, which is the quantity the model's bar is a proxy for. A
    // correct bar is monotone in this gap; the printed table IS the bar.
    println!("\n-- OPPONENT PASS RATE by (required - board) --");
    let mut buckets: BTreeMap<i32, (usize, usize)> = BTreeMap::new();
    for r in rows.iter().filter(|r| r.opp_set_size > 0 && r.opp_required > 0) {
        let gap = (r.opp_required - r.opp_board).clamp(-9, 12);
        let e = buckets.entry(gap).or_insert((0, 0));
        e.0 += 1;
        e.1 += usize::from(r.opp_passed);
    }
    println!("   board short by      n   pass rate   note");
    for (gap, (n, hits)) in buckets {
        println!(
            "   {gap:>6}  {n:>7}   {:.3}      {}",
            hits as f64 / n as f64,
            if gap <= 0 {
                "board already covers it"
            } else {
                "covered only if their flips yield"
            }
        );
    }
}
