//! Bot arena: run N-second matchups between bot versions and random.
//!
//! Usage: cargo run --release --bin bot_arena -- [p1] [p2] [budget_secs] [deck]
//!   p1/p2: any name in bot::registry::BotKind::ALL (default: v2 random 10)
//!
//! Moved out of tests/test_modules/strategy_bot_test.rs  Ethis is a
//! benchmark/arena, not a unit test. Run it when you want numbers.

use rabuka_engine::bot::{
    encoding::ActionEncoding, neural::PolicyNet, registry::BotKind, strategy_v2, strategy_v3,
    strategy_v6, strategy_v7, strategy_v8, PublicObservation,
};
use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader;
use rabuka_engine::deck_parser;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::turn::TurnEngine;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

type ArenaResult<T> = Result<T, Box<dyn std::error::Error>>;

/// How many turns of the development curve to report. Section 1 puts most
/// games between T5 and T8, and calls anything past T10 a sign that both
/// sides are failing checks rather than playing an archetype.
const CURVE_TURNS: usize = 11;

/// Total printed stage cost: the guides' own development metric (section 1,
/// "DERIVED QUANTIONS" / S1 cost curve).
fn stage_cost(p: &rabuka_engine::player::Player, db: &CardDatabase) -> i32 {
    p.stage
        .stage
        .iter()
        .filter(|&&c| c >= 0)
        .filter_map(|&c| db.get_card(c).and_then(|card| card.cost))
        .map(i32::from)
        .sum()
}

struct Options {
    p1: BotKind,
    p2: BotKind,
    budget: u64,
    games: Option<u32>,
    seed: u32,
    deck: String,
    audit: Option<PathBuf>,
    snapshots: Option<PathBuf>,
    compare: Option<PathBuf>,
    /// Per-game outcome log, so two runs on the same seed can be paired.
    outcomes: Option<PathBuf>,
    /// Paired significance test of this run against a baseline outcome log.
    vs: Option<PathBuf>,
    trace: bool,
    logs: bool,
}

impl Options {
    fn parse(args: &[String], env_games: Option<&str>) -> ArenaResult<Self> {
        let mut positional = Vec::new();
        let mut games = None;
        let mut seed = 1u32;
        let mut audit = None;
        let mut snapshots = None;
        let mut compare = None;
        let mut outcomes = None;
        let mut vs = None;
        let mut trace = false;
        let mut logs = false;
        let mut args = args.iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--trace" => trace = true,
                "--logs" => logs = true,
                "--games" | "--seed" | "--audit" | "--snapshots" | "--compare" | "--outcomes"
                | "--vs" => {
                    let value = args
                        .next()
                        .filter(|v| !v.starts_with("--"))
                        .ok_or_else(|| format!("missing value for {arg}"))?;
                    match arg.as_str() {
                        "--games" => games = Some(value.parse::<u32>()?),
                        "--seed" => seed = value.parse::<u32>()?,
                        "--snapshots" => snapshots = Some(PathBuf::from(value)),
                        "--compare" => compare = Some(PathBuf::from(value)),
                        "--outcomes" => outcomes = Some(PathBuf::from(value)),
                        "--vs" => vs = Some(PathBuf::from(value)),
                        _ => audit = Some(PathBuf::from(value)),
                    }
                }
                _ if arg.starts_with("--") => return Err(format!("unknown option: {arg}").into()),
                _ => positional.push(arg.as_str()),
            }
        }
        if positional.len() > 4 {
            return Err("expected [p1] [p2] [budget_secs] [deck]".into());
        }
        if games.is_none() {
            games = env_games.map(str::parse::<u32>).transpose()?;
        }
        if games == Some(0) || seed == 0 {
            return Err("game count and seed must be positive".into());
        }
        if (audit.is_some() || snapshots.is_some() || outcomes.is_some()) && games.is_none() {
            return Err(
                "--audit, --snapshots and --outcomes require --games N or ARENA_GAMES=N".into(),
            );
        }
        // A paired test is only meaningful on a fixed game count: in
        // wall-clock mode `n` differs between runs, the per-game joins stop
        // lining up, and every number produced is unreproducible.
        if vs.is_some() && games.is_none() {
            return Err("--vs requires a fixed --games N so the two runs pair game-for-game".into());
        }
        if compare.is_some()
            && (audit.is_some()
                || snapshots.is_some()
                || outcomes.is_some()
                || vs.is_some()
                || !positional.is_empty()
                || trace
                || logs)
        {
            return Err("--compare PATH is a standalone exact-state diagnostic mode".into());
        }
        let parse_kind = |name: &str| -> ArenaResult<BotKind> {
            if !BotKind::ALL.contains(&name) {
                return Err(
                    format!("unknown bot: {name}; expected {}", BotKind::ALL.join(", ")).into(),
                );
            }
            Ok(BotKind::parse(name))
        };
        Ok(Self {
            p1: parse_kind(positional.first().copied().unwrap_or("v2"))?,
            p2: parse_kind(positional.get(1).copied().unwrap_or("random"))?,
            budget: positional
                .get(2)
                .map(|s| s.parse())
                .transpose()?
                .unwrap_or(10),
            games,
            seed,
            deck: positional
                .get(3)
                .copied()
                .unwrap_or("5CP3Z idou")
                .to_string(),
            audit,
            snapshots,
            compare,
            outcomes,
            vs,
            trace,
            logs,
        })
    }

    fn should_start_game(&self, completed: u32, elapsed: std::time::Duration) -> bool {
        match self.games {
            Some(count) => completed < count,
            None => elapsed.as_secs() < self.budget,
        }
    }
}

fn game_seeds(base: u32, game: u32) -> (u32, u64) {
    let engine = ((u64::from(base) - 1 + u64::from(game) - 1) % u64::from(u32::MAX) + 1) as u32;
    (engine, 0x5EED_1234_ABCD_0001 ^ u64::from(engine))
}

// -- Paired A/B significance ------------------------------------------------
//
// The deal for game N is a pure function of (--seed, N), so two runs on the
// same seed face IDENTICAL shuffles. That makes the comparison paired, and a
// paired test is far more sensitive than two independent win-rate tallies:
// it throws away the between-game variance and only looks at the games where
// the two policies actually disagreed. Independent-tally noise is what made
// +/-2pp results unfalsifiable for this project (docs/BOT_STRATEGY.md 9.2
// shipped "+10" while discarding deltas of the same size).

/// One game's outcome, from P1's perspective. This is the unit of pairing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GameOutcome {
    engine_seed: u32,
    z1: u8,
    z2: u8,
    /// +1 P1 reached 3 first, -1 P2 did, 0 neither reached 3 (draw/stall).
    result: i8,
    turns: u8,
    /// Live phases each side actually took. The denominator of the guides'
    /// pace metric (section 1): placements per live phase.
    live_p1: u16,
    live_p2: u16,
}

impl GameOutcome {
    /// Placements per live phase, the guide's ~1.0-from-T2 target (section 1).
    /// `None` when the side never took a live phase, which would make the rate
    /// undefined rather than zero.
    fn pace(&self, side: usize) -> Option<f64> {
        let (placed, phases) = if side == 0 {
            (self.z1 as f64, self.live_p1)
        } else {
            (self.z2 as f64, self.live_p2)
        };
        (phases > 0).then(|| placed / f64::from(phases))
    }
}

fn classify(z1: usize, z2: usize) -> i8 {
    if z1 >= 3 && z2 <= 2 {
        1
    } else if z2 >= 3 && z1 <= 2 {
        -1
    } else {
        0
    }
}

/// Log-factorials up to `n`, so the binomial tail can be summed in log space
/// without ever materialising a huge integer.
fn log_factorials(n: u64) -> Vec<f64> {
    let mut lf = Vec::with_capacity(n as usize + 1);
    lf.push(0.0);
    let mut acc = 0.0f64;
    for i in 1..=n {
        acc += (i as f64).ln();
        lf.push(acc);
    }
    lf
}

/// Exact two-sided McNemar p-value. Under H0 (the two policies are
/// interchangeable) the discordant pairs split Binomial(b + c, 1/2).
fn mcnemar_exact_p(b: u64, c: u64) -> f64 {
    let n = b + c;
    if n == 0 {
        return 1.0;
    }
    let lf = log_factorials(n);
    let m = b.min(c);
    let mut tail = 0.0f64;
    for k in 0..=m {
        let k = k as usize;
        let coeff = (lf[n as usize] - lf[k] - lf[n as usize - k]).exp();
        tail += coeff / 2f64.powi(n as i32);
    }
    (2.0 * tail).min(1.0)
}

/// Wilson score interval on a proportion, which behaves near 0 and 1 where the
/// normal approximation does not.
fn wilson_interval(successes: u64, trials: u64) -> (f64, f64) {
    if trials == 0 {
        return (0.0, 0.0);
    }
    let n = trials as f64;
    let p = successes as f64 / n;
    let z = 1.959_963_984_540_054_f64; // 95%
    let denom = 1.0 + z * z / n;
    let centre = p + z * z / (2.0 * n);
    let margin = z * ((p * (1.0 - p) / n) + z * z / (4.0 * n * n)).sqrt();
    ((centre - margin) / denom, (centre + margin) / denom)
}

fn write_outcomes(path: &PathBuf, rows: &[GameOutcome]) -> ArenaResult<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let mut out =
        String::from("game,engine_seed,success_p1,success_p2,result,turns,live_p1,live_p2\n");
    for (i, r) in rows.iter().enumerate() {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            i + 1,
            r.engine_seed,
            r.z1,
            r.z2,
            r.result,
            r.turns,
            r.live_p1,
            r.live_p2
        ));
    }
    std::fs::write(path, out)?;
    Ok(())
}

fn read_outcomes(path: &PathBuf) -> ArenaResult<Vec<GameOutcome>> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read outcomes {}: {e}", path.display()))?;
    let mut rows = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if n == 0 || line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split(',').collect();
        if f.len() < 6 {
            return Err(format!("malformed outcomes line {}: {line}", n + 1).into());
        }
        rows.push(GameOutcome {
            engine_seed: f[1].parse()?,
            z1: f[2].parse()?,
            z2: f[3].parse()?,
            result: f[4].parse()?,
            turns: f[5].parse()?,
            // Tolerate logs written before these columns existed, so a
            // baseline from an older run still pairs instead of erroring.
            live_p1: f.get(6).and_then(|v| v.parse().ok()).unwrap_or(0),
            live_p2: f.get(7).and_then(|v| v.parse().ok()).unwrap_or(0),
        });
    }
    Ok(rows)
}

/// Paired comparison of this run against a baseline, joined on engine seed.
/// Prints the discordant table, the exact test, and an interval on the
/// decisive-rate difference. This is the number a change should be judged on.
fn report_paired(baseline: &[GameOutcome], candidate: &[GameOutcome], base_label: &str) {
    let by_seed: std::collections::HashMap<u32, &GameOutcome> =
        baseline.iter().map(|r| (r.engine_seed, r)).collect();
    let mut joined = 0u64;
    let mut seed_mismatch = 0u64;
    // Discordant: (baseline wins / candidate loses) and the reverse.
    let mut b_only = 0u64;
    let mut c_only = 0u64;
    let mut both_win = 0u64;
    let mut both_lose = 0u64;
    let mut both_draw = 0u64;
    let mut base_decisive = 0u64;
    let mut cand_decisive = 0u64;

    for c in candidate {
        let Some(b) = by_seed.get(&c.engine_seed) else {
            seed_mismatch += 1;
            continue;
        };
        joined += 1;
        if b.result != 0 {
            base_decisive += 1;
        }
        if c.result != 0 {
            cand_decisive += 1;
        }
        match (b.result, c.result) {
            (0, 0) => both_draw += 1,
            (x, y) if x == y => {
                if x > 0 {
                    both_win += 1;
                } else {
                    both_lose += 1;
                }
            }
            (x, y) => {
                // x is the baseline's sign, y the candidate's.
                let baseline_won = x > 0;
                let candidate_won = y > 0;
                if baseline_won && !candidate_won {
                    b_only += 1;
                } else if !baseline_won && candidate_won {
                    c_only += 1;
                } else {
                    // One side decided, the other drew.
                    if baseline_won {
                        b_only += 1;
                    } else {
                        c_only += 1;
                    }
                }
            }
        }
    }

    let p = mcnemar_exact_p(b_only, c_only);
    let (blo, bhi) = wilson_interval(base_decisive, joined);
    let (clo, chi) = wilson_interval(cand_decisive, joined);

    println!("PAIRED A/B (joined on engine seed = identical deal)");
    println!("  baseline : {base_label}");
    println!(
        "  joined games {joined} | unpaired (seed absent from baseline) {seed_mismatch}"
    );
    println!("  concordant: both win {both_win} | both lose {both_lose} | both draw {both_draw}");
    println!("  DISCORDANT: baseline-only {b_only} | candidate-only {c_only}");
    println!(
        "  decisive rate  baseline {:.1}% [{:.1}, {:.1}]   candidate {:.1}% [{:.1}, {:.1}]",
        100.0 * base_decisive as f64 / joined.max(1) as f64,
        100.0 * blo,
        100.0 * bhi,
        100.0 * cand_decisive as f64 / joined.max(1) as f64,
        100.0 * clo,
        100.0 * chi
    );
    let swing = 100.0
        * (cand_decisive as f64 - base_decisive as f64)
        / joined.max(1) as f64;
    println!("  decisive-rate swing: {:+.2} pp", swing);
    println!("  exact McNemar two-sided p = {p:.5}");

    // The pace metric, paired. Win rate is a race outcome and hides why; the
    // guides' own instrument is placements per live phase (section 1), and it
    // moves several times more per game than the win/loss tally does.
    //
    // Reported for BOTH sides. It is only informative for a side whose policy
    // actually changed: in a v8-vs-v7 matchup the P2 delta is pinned near zero
    // by construction, because v7 played exactly the same games.
    for (side, label) in [(0usize, "P1"), (1usize, "P2")] {
        // (baseline place, baseline phases, candidate place, candidate phases)
        let mut pairs: Vec<(f64, f64, f64, f64)> = Vec::new();
        for c in candidate {
            let Some(b) = by_seed.get(&c.engine_seed) else {
                continue;
            };
            let pick = |g: &GameOutcome| -> (f64, f64) {
                if side == 0 {
                    (f64::from(g.z1), f64::from(g.live_p1))
                } else {
                    (f64::from(g.z2), f64::from(g.live_p2))
                }
            };
            let (bp, bq) = pick(b);
            let (cp, cq) = pick(c);
            if bq > 0.0 && cq > 0.0 {
                pairs.push((bp, bq, cp, cq));
            }
        }
        if pairs.len() < 2 {
            continue;
        }
        let pooled = |side_index: usize| -> f64 {
            let (place_at, phase_at) = if side_index == 0 { (0, 1) } else { (2, 3) };
            let place: f64 = pairs
                .iter()
                .map(|p| if place_at == 0 { p.0 } else { p.2 })
                .sum();
            let phase: f64 = pairs
                .iter()
                .map(|p| if phase_at == 1 { p.1 } else { p.3 })
                .sum();
            if phase <= 0.0 {
                0.0
            } else {
                place / phase
            }
        };
        let base_rate = pooled(0);
        let cand_rate = pooled(1);
        let observed = cand_rate - base_rate;

        // Paired bootstrap over games. Deterministic LCG so the interval is
        // reproducible: these numbers gate ship decisions, so an interval that
        // moved between runs would be worse than no interval at all.
        let n = pairs.len();
        let mut deltas: Vec<f64> = Vec::with_capacity(2000);
        let mut rng = 0x2545_F491_4F6C_DD1Du64;
        for _ in 0..2000 {
            let (mut bp, mut bq, mut cp, mut cq) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
            for _ in 0..n {
                rng ^= rng >> 12;
                rng ^= rng << 25;
                rng ^= rng >> 27;
                let p = pairs[(rng % n as u64) as usize];
                bp += p.0;
                bq += p.1;
                cp += p.2;
                cq += p.3;
            }
            deltas.push(
                (if cq > 0.0 { cp / cq } else { 0.0 }) - (if bq > 0.0 { bp / bq } else { 0.0 }),
            );
        }
        deltas.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let (lo, hi) = (deltas[deltas.len() / 40], deltas[deltas.len() * 39 / 40]);
        let significant = lo > 0.0 || hi < 0.0;
        println!(
            "  pace ({label}) baseline {base_rate:.4} -> candidate {cand_rate:.4}  delta {observed:+.4} [{lo:+.4}, {hi:+.4}]{}",
            if significant { "  SIGNIFICANT" } else { "" }
        );
    }

    println!(
        "  VERDICT: {}",
        if p < 0.05 {
            if swing > 0.0 {
                "SIGNIFICANT IMPROVEMENT"
            } else {
                "SIGNIFICANT REGRESSION"
            }
        } else {
            "no significant difference (do not ship a change on this evidence)"
        }
    );
}

/// The guides' own definition of a healthy bot (docs/BOT_STRATEGY.md section 1),
/// reported next to the win rate so a change can be judged on the quantity the
/// doctrine actually cares about:
///
/// - ~1 placement per live phase per side from T2 on
/// - a rate of ~0.33 or below is "a defect, not an archetype"
/// - median game length T5-T8, hard band 5-9
fn report_health(
    rows: &[GameOutcome],
    live_p1: u64,
    live_p2: u64,
    curve_p1: &[i32; CURVE_TURNS],
    curve_p2: &[i32; CURVE_TURNS],
    curve_p1_n: &[u64; CURVE_TURNS],
    curve_p2_n: &[u64; CURVE_TURNS],
    end_cost: (i64, i64),
) {
    if rows.is_empty() {
        return;
    }
    let placed_p1: u64 = rows.iter().map(|r| u64::from(r.z1)).sum();
    let placed_p2: u64 = rows.iter().map(|r| u64::from(r.z2)).sum();
    let rate = |placed: u64, phases: u64| {
        if phases == 0 {
            0.0
        } else {
            placed as f64 / phases as f64
        }
    };
    let p1_rate = rate(placed_p1, live_p1);
    let p2_rate = rate(placed_p2, live_p2);

    let mut turns: Vec<u64> = rows.iter().map(|r| u64::from(r.turns)).collect();
    turns.sort_unstable();
    let q = |frac: f64| -> u64 {
        let idx = ((turns.len() as f64 - 1.0) * frac).round() as usize;
        turns[idx.min(turns.len() - 1)]
    };
    let (median, p90, max) = (q(0.5), q(0.9), *turns.last().unwrap_or(&0));

    let flag = |r: f64| if r > 0.0 && r < 0.33 { " <-- DEFECT" } else { "" };
    println!("GUIDE HEALTH METRICS (docs/BOT_STRATEGY.md section 1)");
    println!(
        "  placements per live phase: P1 {p1_rate:.3}{} | P2 {p2_rate:.3}{}   (target ~1.0 from T2; <=0.33 is a defect)",
        flag(p1_rate),
        flag(p2_rate)
    );
    println!(
        "  game length: median T{median} | p90 T{p90} | max T{max}   (target median T5-T8, hard band 5-9)"
    );
    if median > 9 {
        println!("    <-- median above the hard band: games are running long");
    } else if median < 5 {
        println!("    <-- median below the band: games may be ending before development matters");
    }

    // The development curve itself, averaged over games. Section 1 gives the
    // reference points (T1 about 4, T2 about 9, T3 about 13). A bot that wins
    // while trailing this curve is winning on the other seat's mistakes, which
    // is the trap that made v6 look like a 76% improvement.
    //
    // Bucket N is the board ENTERING turn N, which is the board turn N-1 built.
    // So the guide's T1=4 line is bucket T2, T2=9 is bucket T3, and so on.
    let avg = |sum: &[i32; CURVE_TURNS], n: &[u64; CURVE_TURNS]| -> Vec<String> {
        (1..CURVE_TURNS)
            .map(|t| {
                if n[t] == 0 {
                    format!("T{t}  -")
                } else {
                    format!("T{t} {:>2.1}", sum[t] as f64 / n[t] as f64)
                }
            })
            .collect()
    };
    println!("  avg stage cost ENTERING each turn (guide board after Tn-1: T1~4, T2~9, T3~13)");
    println!("    P1 {}", avg(curve_p1, curve_p1_n).join(" "));
    println!("    P2 {}", avg(curve_p2, curve_p2_n).join(" "));
    let games = rows.len().max(1) as f64;
    println!(
        "  avg FINAL stage cost: P1 {:.1} | P2 {:.1}",
        end_cost.0 as f64 / games,
        end_cost.1 as f64 / games
    );
}

fn audit_card(db: &CardDatabase, id: i16) -> Value {
    if id < 0 {
        return Value::Null;
    }
    match db.get_card(id) {
        Some(card) => json!({
            "card_id": id,
            "card_no": card.card_no,
            "card_type": card.card_type,
            "base_cost": card.cost,
            "base_hearts": card.base_heart,
            "base_blades": card.blade,
            "blade_hearts": card.blade_heart,
            "required_hearts": card.need_heart,
            "base_score": card.score,
        }),
        None => json!({"card_id": id, "unresolved": true}),
    }
}

fn audit_cards(db: &CardDatabase, ids: &[i16]) -> Vec<Value> {
    ids.iter().map(|&id| audit_card(db, id)).collect()
}

fn decision_player(gs: &GameState) -> &rabuka_engine::player::Player {
    if gs.has_pending_choice() {
        if gs.can_player_act(0) {
            return &gs.player1;
        }
        if gs.can_player_act(1) {
            return &gs.player2;
        }
    }
    gs.active_player()
}

fn audit_view(gs: &GameState) -> Value {
    let own = decision_player(gs);
    let opponent = if own.id == gs.player1.id {
        &gs.player2
    } else {
        &gs.player1
    };
    let db = &gs.card_database;
    json!({
        "own": {
            "player": own.id,
            "is_first_attacker": own.is_first_attacker,
            "hand": audit_cards(db, &own.hand.cards),
            "stage_left_center_right": audit_cards(db, &own.stage.stage),
            "energy": {"active": own.energy_zone.active_count(), "total": own.energy_zone.cards.len()},
            "success_count": own.success_live_card_zone.cards.len(),
            "success": audit_cards(db, &own.success_live_card_zone.cards),
        },
        "opponent_public": {
            "stage_left_center_right": audit_cards(db, &opponent.stage.stage),
            "success_count": opponent.success_live_card_zone.cards.len(),
            "success": audit_cards(db, &opponent.success_live_card_zone.cards),
        },
    })
}

fn audit_action(gs: &GameState, action: &game_setup::Action) -> ArenaResult<Value> {
    use game_setup::ActionType;
    let mut value = serde_json::to_value(action)?;
    let references_card = matches!(
        action.action_type,
        ActionType::SelectMulligan
            | ActionType::SelectLiveCard
            | ActionType::PlayMemberToStage
            | ActionType::UseAbility
            | ActionType::SetLiveCard
            | ActionType::ChoiceSelect
    );
    if references_card {
        if let Some(id) = action.parameters.as_ref().and_then(|p| p.card_id) {
            let own = decision_player(gs);
            let opponent = if own.id == gs.player1.id {
                &gs.player2
            } else {
                &gs.player1
            };
            let visible = id >= 0
                && (own.hand.cards.contains(&id)
                    || own.stage.stage.contains(&id)
                    || own.success_live_card_zone.cards.contains(&id)
                    || own.waitroom.cards.contains(&id)
                    || opponent.stage.stage.contains(&id)
                    || opponent.success_live_card_zone.cards.contains(&id));
            if visible {
                value["resolved_card"] = audit_card(&gs.card_database, id);
            } else {
                value["identity_redacted"] = json!(true);
                value["description"] = Value::Null;
                value["description_ja"] = Value::Null;
                for field in [
                    "card_id",
                    "card_name",
                    "card_no",
                    "source_ability",
                    "base_cost",
                    "final_cost",
                    "available_areas",
                    "double_baton_pairs",
                ] {
                    value["parameters"][field] = Value::Null;
                }
            }
        }
    }
    Ok(value)
}

type ScoreFn = fn(&GameState, &[game_setup::Action], u8) -> Vec<(f64, String)>;

/// Exact-state equivalence oracle: identical legal-action offers and identical
/// v6/v7 numeric scoring behavior. Deliberately NOT a byte comparison —
/// independently rebuilt HashMaps serialize equal content in different
/// iteration orders while remaining logically identical.
fn behaviorally_equal(a: &GameState, b: &GameState) -> ArenaResult<bool> {
    let actions_a = game_setup::generate_possible_actions(a);
    let actions_b = game_setup::generate_possible_actions(b);
    if serde_json::to_value(&actions_a)? != serde_json::to_value(&actions_b)? {
        return Ok(false);
    }
    let me = if decision_player(a).id == a.player1.id {
        0u8
    } else {
        1u8
    };
    if decision_player(a).id != decision_player(b).id {
        return Ok(false);
    }
    for score in [
        strategy_v6::score_actions as ScoreFn,
        strategy_v7::score_actions as ScoreFn,
            strategy_v8::score_actions_v8 as ScoreFn,

    ] {
        let x = policy_call(|| score(a, &actions_a, me));
        let y = policy_call(|| score(b, &actions_b, me));
        if x.len() != y.len()
            || x.iter()
                .zip(&y)
                .any(|((s1, c1), (s2, c2))| s1.to_bits() != s2.to_bits() || c1 != c2)
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn corpus_fold(seed: u32) -> &'static str {
    if seed % 5 == 0 {
        "holdout"
    } else {
        "train"
    }
}

fn snapshot_eligible(gs: &GameState) -> bool {
    gs.current_phase == Phase::Main
        && gs.game_result == GameResult::Ongoing
        && gs.get_pending_choice().is_none()
        && gs.ability_queue.is_idle()
        && gs.ability_queue.is_empty()
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SavedCard {
    id: i16,
    card_no: String,
    identity: Value,
}

/// Opt-in full-hidden-state capture for offline exact-state debugging.
/// CONTAINS BOTH PLAYERS' PRIVATE INFORMATION (hands, deck order). Never a
/// fair counterfactual evaluation format; per-action scores are hindsight
/// diagnostics only.
#[derive(serde::Serialize, serde::Deserialize)]
struct SavedPosition {
    format: String,
    schema: u32,
    metadata: Value,
    /// Global engine xorshift32 state captured pre-decision; restored before
    /// each bot's scoring/selection so scoring side effects (sim draws) don't
    /// shift subsequent engine RNG.
    engine_rng: u32,
    /// Arena LCG state (instance-owned, pub field) — resumed after replay.
    arena_rng: u64,
    /// Physical card copies with exact card_no identity; supports duplicate
    /// deck copies (per-copy IDs from create_copy).
    cards: Vec<SavedCard>,
    card_no_to_id: std::collections::BTreeMap<String, i16>,
    normalized_no_to_id: std::collections::BTreeMap<String, i16>,
    next_id: i16,
    /// Full GameState (serde fields; card_database skipped, reattached here).
    state: Vec<u8>,
    /// serde-skipped engine-internal fields (loop history, logs, scratch).
    internal_state: Vec<u8>,
    /// Offered actions at capture time; restore rejects any drift.
    offers: Value,
}

fn card_identity(card: &rabuka_engine::card::Card) -> ArenaResult<Value> {
    Ok(json!({
        "card": serde_json::to_value(card)?,
        "abilities": card.abilities.iter().map(|a| serde_json::to_value(a.resolve())).collect::<Result<Vec<_>, _>>()?,
    }))
}

impl SavedPosition {
    fn capture(gs: &GameState, arena_rng: &Lcg, metadata: Value) -> ArenaResult<Self> {
        if !snapshot_eligible(gs) {
            return Err("snapshot requires idle Main with no pending choices".into());
        }
        let db = &gs.card_database;
        let mut cards = db
            .cards
            .iter()
            .map(|(&id, card)| {
                Ok(SavedCard {
                    id,
                    card_no: card.card_no.to_string(),
                    identity: card_identity(card)?,
                })
            })
            .collect::<ArenaResult<Vec<_>>>()?;
        cards.sort_by_key(|c| c.id);
        Ok(Self {
            format: "rabuka-arena-exact-state".into(),
            schema: 1,
            metadata,
            engine_rng: rabuka_engine::rng::checkpoint(),
            arena_rng: arena_rng.0,
            cards,
            card_no_to_id: db
                .card_no_to_id
                .iter()
                .map(|(k, &v)| (k.clone(), v))
                .collect(),
            normalized_no_to_id: db
                .normalized_no_to_id
                .iter()
                .map(|(k, &v)| (k.clone(), v))
                .collect(),
            next_id: db.next_id,
            state: rmp_serde::to_vec_named(gs)?,
            internal_state: rmp_serde::to_vec_named(&(
                &gs.game_state_history,
                &gs.structured_log,
                &gs.debug_trace,
                &gs.scratch_exp_blade,
                &gs.scratch_exp_score,
                &gs.scratch_exp_heart,
                &gs.scratch_entry_positions,
            ))?,
            offers: serde_json::to_value(game_setup::generate_possible_actions(gs))?,
        })
    }

    /// Rebuild the physical card database (unique per-copy IDs) and the full
    /// GameState, then verify card identity against the current card database
    /// and offers/v6/v7 scoring behavior against the captured record.
    fn restore(&self, templates: &CardDatabase) -> ArenaResult<GameState> {
        if self.format != "rabuka-arena-exact-state" || self.schema != 1 {
            return Err("unsupported snapshot format/schema".into());
        }
        let mut db = CardDatabase::new();
        for saved in &self.cards {
            let card = templates
                .card_no_to_id
                .get(&saved.card_no)
                .and_then(|&id| templates.get_card(id))
                .ok_or_else(|| format!("snapshot card unavailable: {}", saved.card_no))?;
            if card_identity(card)? != saved.identity {
                return Err(
                    format!("snapshot card/ability identity mismatch: {}", saved.card_no).into(),
                );
            }
            if saved.id < 0
                || saved.id >= self.next_id
                || db.cards.insert(saved.id, card.clone()).is_some()
            {
                return Err("invalid or duplicate physical card ID".into());
            }
        }
        for (name, &id) in &self.card_no_to_id {
            if db.get_card(id).is_none_or(|c| c.card_no.as_ref() != name) {
                return Err("invalid exact card lookup in snapshot".into());
            }
        }
        for &id in self.normalized_no_to_id.values() {
            if db.get_card(id).is_none() {
                return Err("invalid normalized card lookup in snapshot".into());
            }
        }
        db.card_no_to_id = self
            .card_no_to_id
            .iter()
            .map(|(k, &v)| (k.clone(), v))
            .collect();
        db.normalized_no_to_id = self
            .normalized_no_to_id
            .iter()
            .map(|(k, &v)| (k.clone(), v))
            .collect();
        db.next_id = self.next_id;
        let mut gs: GameState = rmp_serde::from_slice(&self.state)?;
        gs.card_database = Arc::new(db);
        (
            gs.game_state_history,
            gs.structured_log,
            gs.debug_trace,
            gs.scratch_exp_blade,
            gs.scratch_exp_score,
            gs.scratch_exp_heart,
            gs.scratch_entry_positions,
        ) = rmp_serde::from_slice(&self.internal_state)?;
        if !snapshot_eligible(&gs) {
            return Err("snapshot state is not an idle Main position".into());
        }
        let offers = serde_json::to_value(game_setup::generate_possible_actions(&gs))?;
        if offers != self.offers {
            return Err("restored offers drift from captured offers".into());
        }
        Ok(gs)
    }

    /// Atomic-ish write: create-new temp file, fsync, rename; never overwrites.
    fn write(&self, path: &std::path::Path) -> ArenaResult<()> {
        if path.exists() {
            return Err(format!("refusing to overwrite snapshot {}", path.display()).into());
        }
        let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
        let result = (|| -> ArenaResult<()> {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            file.write_all(&rmp_serde::to_vec_named(self)?)?;
            file.sync_all()?;
            std::fs::rename(&temporary, path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        result
    }
}

/// Restores the global engine RNG state on drop, including panic paths.
struct RngRestore(u32);
impl Drop for RngRestore {
    fn drop(&mut self) {
        rabuka_engine::rng::restore(self.0);
    }
}

fn policy_call<T>(call: impl FnOnce() -> T) -> T {
    let restore = RngRestore(rabuka_engine::rng::checkpoint());
    let result = call();
    drop(restore);
    result
}

#[derive(Debug, PartialEq, Eq)]
enum PolicyRoute {
    Action,
    Mulligan,
    LiveSet,
}

fn policy_route(gs: &GameState) -> PolicyRoute {
    if gs.has_pending_choice() {
        return PolicyRoute::Action;
    }
    match gs.current_phase {
        Phase::MulliganFirstAttacker | Phase::MulliganSecondAttacker => PolicyRoute::Mulligan,
        Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker => PolicyRoute::LiveSet,
        _ => PolicyRoute::Action,
    }
}

fn choose_policy_action(
    gs: &GameState,
    actions: &[game_setup::Action],
    kinds: [BotKind; 2],
    v2_policy: &strategy_v2::V2Policy,
    plans: [&strategy_v3::V3Plan; 2],
    neural: Option<&PolicyNet>,
    rng: &mut Lcg,
) -> game_setup::Action {
    let me = u8::from(decision_player(gs).id != gs.player1.id);
    let kind = kinds[me as usize];
    let plan = plans[me as usize];
    let setup_kind = if kind == BotKind::Neural {
        BotKind::V7
    } else {
        kind
    };
    if kind == BotKind::Neural && policy_route(gs) == PolicyRoute::Action {
        let Some(network) = neural else {
            return actions.first().cloned().unwrap_or(game_setup::Action {
                description: "pass".into(),
                description_ja: None,
                action_type: game_setup::ActionType::Pass,
                parameters: None,
                selected: None,
            });
        };
        let observation = PublicObservation::from_state(gs, me);
        let encoded = network.encode_state(&observation);
        let action_encodings: Vec<ActionEncoding> = actions
            .iter()
            .map(|action| ActionEncoding::from_action(action, &observation))
            .collect();
        let (logits, _) = network.evaluate_actions(&encoded, &action_encodings);
        let epsilon = std::env::var("NEURAL_EPSILON")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        if epsilon > 0 && rng.range(10_000) < epsilon {
            return actions[rng.range(actions.len())].clone();
        }
        let index = logits
            .iter()
            .enumerate()
            .max_by(|(_, left), (_, right)| left.total_cmp(right))
            .map(|(index, _)| index)
            .unwrap_or(0);
        return actions[index].clone();
    }
    match policy_route(gs) {
        PolicyRoute::Mulligan => {
            policy_call(|| setup_kind.choose_mulligan(gs, actions, &gs.card_database))
        }
        _ if kind == BotKind::Random => actions[rng.range(actions.len())].clone(),
        PolicyRoute::LiveSet => policy_call(|| {
            setup_kind.choose_live_set(gs, actions, &gs.card_database, v2_policy, plan)
        }),
        PolicyRoute::Action => policy_call(|| kind.choose_action(gs, actions, me, v2_policy, plan)),
    }
}

/// Replay one saved position offline: same RNG state restored before each
/// bot's scoring and selection; emits all available actions with card_no,
/// per-bot numeric scores (null if nonfinite), component breakdowns, chosen
/// indices. EXACT-STATE HINDSIGHT DIAGNOSTIC ONLY — not fair counterfactual
/// outcome evaluation; no win claims.
fn compare_position(saved: &SavedPosition, templates: &CardDatabase) -> ArenaResult<Value> {
    let _restore_rng = RngRestore(rabuka_engine::rng::checkpoint());
    let gs = saved.restore(templates)?;
    let actions = game_setup::generate_possible_actions(&gs);
    if actions.is_empty() {
        return Err("snapshot has no actions".into());
    }
    let me = gs.active_player_index();
    let mut bots = serde_json::Map::new();
    for (name, score, choose) in [
        (
            "v6",
            strategy_v6::score_actions as ScoreFn,
            strategy_v6::choose_action_v6
                as fn(&GameState, &[game_setup::Action], u8) -> game_setup::Action,
        ),
        (
            "v7",
            strategy_v7::score_actions as ScoreFn,
            strategy_v7::choose_action_v7
                as fn(&GameState, &[game_setup::Action], u8) -> game_setup::Action,
        ),
        (
            "v8",
        strategy_v8::score_actions_v8 as ScoreFn,

            strategy_v8::choose_action as fn(&GameState, &[game_setup::Action], u8) -> game_setup::Action,
        ),
    ] {
        rabuka_engine::rng::restore(saved.engine_rng);
        let scores = policy_call(|| score(&gs, &actions, me));
        if scores.len() != actions.len() {
            return Err("score/action length mismatch".into());
        }
        rabuka_engine::rng::restore(saved.engine_rng);
        let chosen = policy_call(|| choose(&gs, &actions, me));
        let chosen_value = serde_json::to_value(&chosen)?;
        let chosen_index = actions
            .iter()
            .position(|a| serde_json::to_value(a).ok().as_ref() == Some(&chosen_value))
            .ok_or("bot chose an action not offered")?;
        bots.insert(
            name.into(),
            json!({
                "chosen_index": chosen_index,
                "scores": scores.iter().map(|(score, components)| json!({
                    "score": if score.is_finite() { Some(*score) } else { None },
                    "components": components,
                })).collect::<Vec<_>>(),
            }),
        );
    }
    let available = actions
        .iter()
        .enumerate()
        .map(|(index, a)| {
            Ok(json!({
                "index": index,
                "card_no": a.parameters.as_ref().and_then(|p| p.card_id)
                    .and_then(|id| gs.card_database.get_card(id)).map(|c| c.card_no.as_ref()),
                "action": audit_action(&gs, a)?,
            }))
        })
        .collect::<ArenaResult<Vec<_>>>()?;
    Ok(json!({
        "format": "rabuka-arena-comparison", "schema": 1, "metadata": saved.metadata,
        "evaluation": "exact-state debugging only; hidden-state one-ply scores are not fair counterfactual outcome evaluation; no win claims",
        "snapshot_visibility": "full hidden state is stored offline only; view is policy player's hand and public opponent board",
        "score_semantics": "final native policy score after Pass override, not win probability; nonfinite values are null",
        "view": audit_view(&gs), "available_actions": available, "bots": bots,
        "engine_rng": saved.engine_rng, "arena_rng": saved.arena_rng.to_string(),
                        "policy_environment": std::env::vars().filter(|(key, _)| {
                            key.starts_with("V6_") || key.starts_with("V7_") || key.starts_with("V8_")
                        }).collect::<std::collections::BTreeMap<_, _>>(),
    }))
}

fn compare_path(path: &std::path::Path) -> ArenaResult<()> {
    let mut files = if path.is_dir() {
        std::fs::read_dir(path)?
            .map(|entry| entry.map(|e| e.path()))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .filter(|p| p.extension().is_some_and(|e| e == "rmp"))
            .collect::<Vec<_>>()
    } else {
        vec![path.to_path_buf()]
    };
    files.sort();
    if files.is_empty() {
        return Err("no .rmp snapshots found".into());
    }
    let templates = fresh_database();
    let mut stdout = std::io::stdout().lock();
    for file in files {
        let saved: SavedPosition = rmp_serde::from_slice(&std::fs::read(&file)?)?;
        write_jsonl(&mut stdout, &compare_position(&saved, &templates)?)?;
    }
    Ok(())
}

#[derive(Default)]
struct DecisionAudit {
    game: u32,
    decision: u32,
    errors: u32,
    boundary: Option<(u8, Phase, String)>,
    boundary_id: u32,
    boundary_step: u32,
    seen: HashMap<String, u32>,
}

fn write_jsonl(writer: &mut impl Write, value: &Value) -> ArenaResult<()> {
    writer.write_all(&serde_json::to_vec(value)?)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}

fn emit(audit: &mut Option<std::fs::File>, value: &Value) -> ArenaResult<()> {
    if let Some(writer) = audit {
        write_jsonl(writer, value)?;
    }
    Ok(())
}

impl DecisionAudit {
    fn record(
        &mut self,
        gs: &GameState,
        actions: &[game_setup::Action],
        chosen: &game_setup::Action,
    ) -> ArenaResult<Value> {
        let owner = decision_player(gs);
        let boundary = (gs.turn_number, gs.current_phase, owner.id.clone());
        if self.boundary.as_ref() != Some(&boundary) {
            self.boundary = Some(boundary);
            self.boundary_id += 1;
            self.boundary_step = 0;
        }
        self.boundary_step += 1;
        let available = actions
            .iter()
            .map(|a| audit_action(gs, a))
            .collect::<ArenaResult<Vec<_>>>()?;
        let chosen_value = audit_action(gs, chosen)?;
        let snapshot = audit_view(gs);
        let selected = json!(gs.live_card_selected_indices);
        let signature = serde_json::to_string(&json!([
            gs.turn_number,
            gs.current_phase,
            owner.id,
            gs.active_player().id,
            snapshot,
            available,
            chosen_value,
            selected,
            gs.mulligan_selected_indices
        ]))?;
        let repeated_from = self.seen.insert(signature, self.decision);
        let selection_operation = match chosen.action_type {
            game_setup::ActionType::SelectLiveCard | game_setup::ActionType::SelectMulligan => {
                Some(if chosen.selected == Some(true) {
                    "deselect"
                } else {
                    "select"
                })
            }
            game_setup::ActionType::ConfirmLiveCardSet
            | game_setup::ActionType::ConfirmMulligan => Some("confirm"),
            _ => None,
        };
        Ok(json!({
            "event": "decision", "game": self.game, "decision": self.decision,
            "turn": gs.turn_number, "phase": gs.current_phase,
            "policy_player": owner.id,
            "active_player": gs.active_player().id,
            "pending_choice_player": gs.get_pending_choice_player_id(),
            "pending_choice": gs.get_pending_choice().is_some(),
            "boundary_id": self.boundary_id, "boundary_step": self.boundary_step,
            "repeated_visible_decision_from": repeated_from,
            "live_selected_hand_indices_before": selected,
            "mulligan_selected_hand_indices_before": gs.mulligan_selected_indices,
            "selection_operation": selection_operation,
            "debug_note": gs.debug_trace.last().cloned(),
            "v7_live_note": rabuka_engine::bot::strategy_v7::live_set_audit_note(gs),
            "view": snapshot, "chosen": chosen_value, "available_actions": available,
        }))
    }

    fn execute(
        &mut self,
        audit: &mut Option<std::fs::File>,
        gs: &mut GameState,
        actions: &[game_setup::Action],
        chosen: &game_setup::Action,
    ) -> ArenaResult<()> {
        self.decision += 1;
        if audit.is_some() {
            emit(audit, &self.record(gs, actions, chosen)?)?;
        }
        let result = game_setup::execute_action(gs, chosen);
        if let Err(error) = &result {
            self.errors += 1;
            eprintln!(
                "ARENA game={} decision={} action={} failed: {}",
                self.game, self.decision, chosen.action_type, error
            );
        }
        emit(
            audit,
            &json!({
                "event": "action_result", "game": self.game, "decision": self.decision,
                "execution_ok": result.is_ok(),
            }),
        )?;
        game_setup::settle_single_player_state(gs);
        Ok(())
    }
}

/// Bot identity lives in [`BotKind`] (`bot/registry.rs`): adding a version
/// means a new variant + dispatch lines there, never renames here.
use rabuka_engine::rng::Lcg;

fn fresh_database() -> Arc<CardDatabase> {
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = card_loader::CardLoader::load_cards_from_file(cards_path).expect("load cards");
    Arc::new(CardDatabase::load_or_create(cards))
}

fn load_test_deck(db: &Arc<CardDatabase>, name: &str) -> Vec<String> {
    let deck_path = std::path::Path::new("../web_ui/decks").join(format!("{name}.txt"));
    if deck_path.exists() {
        let deck = deck_parser::DeckParser::parse_deck_file(&deck_path).expect("parse deck");
        return deck_parser::DeckParser::deck_list_to_card_numbers(&deck);
    }
    // Previously this fell back to a synthesized 60-card list. That was a
    // measurement trap: a mistyped deck name produced a plausible-looking
    // mirror match on a deck nobody asked for, and every number derived from
    // it was silently about the wrong game. Fail instead.
    let _ = db;
    panic!(
        "deck not found: {}\n\
         A missing deck is a hard error, not a fallback: every win rate measured \
         against a synthesized list describes a different game than the one you asked for.",
        deck_path.display()
    );
}

fn build_templates(
    db: &mut Arc<CardDatabase>,
    n1: &[String],
    n2: &[String],
) -> (
    rabuka_engine::deck_builder::Deck,
    rabuka_engine::deck_builder::Deck,
) {
    game_setup::build_two_decks(db, n1, n2).expect("build decks")
}

fn deal_from_templates(
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

fn my_hand_lives(gs: &GameState, is_p1: bool, db: &Arc<CardDatabase>) -> usize {
    let p = if is_p1 { &gs.player1 } else { &gs.player2 };
    p.hand
        .cards
        .iter()
        .filter(|&&c| {
            db.get_card(c)
                .is_some_and(|x| x.card_type == rabuka_engine::card::CardType::Live)
        })
        .count()
}

fn main() -> ArenaResult<()> {
    // Training default: strip Action display strings + log materialization.
    // `--logs` (options.logs) keeps engine logs for arena file dumps.
    // Web path never enters this binary.
    let args: Vec<String> = std::env::args().skip(1).collect();
    let env_games = std::env::var("ARENA_GAMES").ok();
    let options = Options::parse(&args, env_games.as_deref())?;
    rabuka_engine::game_setup::set_action_display(false);
    rabuka_engine::game_setup::set_logging_enabled(options.logs || options.audit.is_some());
    if let Some(path) = &options.compare {
        return compare_path(path);
    }
    if let Some(path) = &options.snapshots {
        std::fs::create_dir_all(path)?;
        eprintln!("SNAPSHOTS: full hidden state, offline exact-state debugging only; seed % 5 == 0 is held out; cap 20/game");
    }
    let p1_kind = options.p1;
    let p2_kind = options.p2;
    let neural = if p1_kind == BotKind::Neural || p2_kind == BotKind::Neural {
        let path = std::env::var("NEURAL_WEIGHTS").map_err(|_| {
            "NEURAL_WEIGHTS must point to a schema-v3 policy weights file".to_string()
        })?;
        let mut network = PolicyNet::new();
        network
            .load_weights(&path)
            .map_err(|error| format!("load NEURAL_WEIGHTS {path}: {error}"))?;
        Some(network)
    } else {
        None
    };
    let trace = options.trace;
    let logs = options.logs;
    let deck_name = &options.deck;
    let mut audit = options
        .audit
        .as_ref()
        .map(std::fs::File::create)
        .transpose()?;
    if logs {
        std::fs::create_dir_all("../test_output/arena_logs")?;
    }
    if (audit.is_some() || options.snapshots.is_some())
        && !std::path::Path::new("../web_ui/decks")
            .join(format!("{deck_name}.txt"))
            .is_file()
    {
        return Err(
            format!("audit/snapshots require an existing deck file: {deck_name}.txt").into(),
        );
    }
    emit(
        &mut audit,
        &json!({
            "event": "run_start", "schema_version": 1,
            "bots": [p1_kind.name(), p2_kind.name()], "deck": deck_name,
            "games": options.games, "base_seed": options.seed,
            "iteration_cap_per_game": 600, "same_turn_iteration_cap": 200,
            "card_stats": "printed/base, not effective modifiers",
            "identity_note": "card_no is authoritative; card_id can differ across builds for same-number sibling prints (R+/P/P+/SEC)",
            "visibility": "each row is private to policy_player; opponent snapshot contains stage and success only; non-visible action card identities are redacted",
            "rng_limitations": "engine global RNG and arena LCG reseeded before each deal; engine RNG checkpoint restored immediately after every policy call; no cross-build replay determinism guarantee",
        }),
    )?;

    let kind_name = |k: BotKind| k.name();

    let mut db = fresh_database();
    let nums = load_test_deck(&db, deck_name);
    eprintln!(
        "ARENA deck={} entries={} distinct={}",
        deck_name,
        nums.len(),
        nums.iter().collect::<std::collections::HashSet<_>>().len()
    );
    let (t1, t2) = build_templates(&mut db, &nums, &nums);

    let v2_policy = strategy_v2::V2Policy::default();

    let mut wins = [0u32; 2];
    let mut draws = 0u32;
    // Draw-type breakdown: a "draw" (z1<3 xor z2<3 is false) is either a real
    // mutual 3-3 (both >=3) or a STALL (neither reached 3 — a no-contest
    // artifact from the stuck counter / turn cap). A game that ends while
    // `game_result == Ongoing` was a stuck/timeout break, not a real result.
    let mut real_draws = 0u32;
    let mut stall_draws = 0u32;
    let mut stuck_ends = 0u32;
    let mut final_hist: std::collections::BTreeMap<(u8, u8), u32> =
        std::collections::BTreeMap::new();
    let mut games = 0u32;
    let mut total_actions = 0u64;
    let mut total_turns = 0u64;
    // Do-nothing telemetry: how often a Main phase ends with no member played
    // (ConfirmMainPhase chosen while a deploy was available) and how often a
    // Live Card Set folds to an empty zone.
    let mut _main_decisions = 0u64;
    let mut _main_confirms = 0u64;
    let mut live_decisions = 0u64;
    let mut live_folds = 0u64;
    // Per-side live phases, for the guides' own pace metric: section 1 expects
    // ~1 placement per live phase per side from T2 on, and calls a rate of
    // ~0.33 "a defect, not an archetype". This is the instrument the project
    // never had - win rate is a race outcome and hides the actual bottleneck.
    let mut live_phases_p1 = 0u64;
    let mut live_phases_p2 = 0u64;
    // Per-turn development curve, the guides' T1=4 / T2=9 / T3=13 metric.
    let mut curve_p1 = [0i32; CURVE_TURNS];
    let mut curve_p2 = [0i32; CURVE_TURNS];
    let mut curve_p1_n = [0u64; CURVE_TURNS];
    let mut curve_p2_n = [0u64; CURVE_TURNS];
    // End-of-game stage cost. A sanity check on the per-turn curve above: if
    // this is a real number but the curve reads zero, the per-turn bucketing
    // is wrong, not the bots.
    let mut end_cost_p1 = 0i64;
    let mut end_cost_p2 = 0i64;
    let mut main_phase_count = 0u64;
    let mut empty_main_count = 0u64;
    let t0 = std::time::Instant::now();
    // Fixed-n runs are reproducible and pairable; wall-clock runs are not.
    // Say so loudly rather than letting a number escape without its caveat.
    if options.games.is_none() {
        eprintln!(
            "ARENA WARNING: no --games given, so this run stops on a {}-second wall clock.\n\
             \x20           Game count will differ between runs, results are NOT reproducible,\n\
             \x20           and no paired significance test is possible. Pass --games N.",
            options.budget
        );
    }
    let mut outcome_rows: Vec<GameOutcome> = Vec::new();
    let mut trace_rows: Vec<String> = Vec::new();
    let mut game_start_idx = 0usize;
    if trace {
        trace_rows.push(
            "game,turn,phase,player,action_type,card_no,live_p1,live_p2,success_p1,success_p2"
                .into(),
        );
    }

    while options.should_start_game(games, t0.elapsed()) {
        games += 1;
        let (engine_seed, arena_seed) = game_seeds(options.seed, games);
        rabuka_engine::rng::seed(engine_seed);
        let mut rng = Lcg(arena_seed);
        let mut decisions = DecisionAudit {
            game: games,
            ..Default::default()
        };
        emit(
            &mut audit,
            &json!({
                "event": "game_start", "game": games,
                "engine_seed": engine_seed, "arena_seed": arena_seed.to_string(),
            }),
        )?;
        let mut gs = deal_from_templates(&db, &t1, &t2);
        let mut captured_turns = std::collections::HashSet::new();
        let mut end_reason = "iteration_cap";
        // Archetype detection runs once per game over the full own decklist.
        let plan_p1 = policy_call(|| strategy_v3::V3Plan::detect(&gs, 0, &db));
        let plan_p2 = policy_call(|| strategy_v3::V3Plan::detect(&gs, 1, &db));
        let mut last_turn = 0u8;
        // Per-GAME live phase counts. These must be per game, not the outer
        // run totals: recording the running total would make the paired pace
        // metric meaningless (a cumulative denominator against a per-game
        // numerator reads as ~0).
        let mut game_live_p1 = 0u64;
        let mut game_live_p2 = 0u64;
        let mut stuck = 0u32;
        // Live-phase telemetry: snapshot at every phase change so transcripts
        // show WHO set WHAT and whether checks passed.
        let mut prev_phase = gs.current_phase;
        let mut timeline: Vec<String> = Vec::new();
        // Per-main-phase deploy tracking (do-nothing detection); counters
        // themselves live in the outer scope and accumulate across games.
        let mut cur_is_main = false;
        let mut cur_main_plays = 0u64;
        let snap_row = |tag: &str, gs: &GameState| -> String {
            let lives_in_hand = |p: &rabuka_engine::player::Player| {
                p.hand
                    .cards
                    .iter()
                    .filter(|&&c| {
                        db.get_card(c)
                            .is_some_and(|x| x.card_type == rabuka_engine::card::CardType::Live)
                    })
                    .count()
            };
            format!(
                "{},{},{},active={},live_p1={},live_p2={},succ_p1={},succ_p2={},hand_p1={} ({} lives),hand_p2={} ({} lives),en_p1={},en_p2={},cost_p1={},cost_p2={}",
                games,
                gs.turn_number,
                tag,
                if gs.active_player().id == "p1" { "P1" } else { "P2" },
                gs.player1.live_card_zone.cards.len(),
                gs.player2.live_card_zone.cards.len(),
                gs.player1.success_live_card_zone.cards.len(),
                gs.player2.success_live_card_zone.cards.len(),
                gs.player1.hand.cards.len(),
                lives_in_hand(&gs.player1),
                gs.player2.hand.cards.len(),
                lives_in_hand(&gs.player2),
                gs.player1.energy_zone.active_count(),
                gs.player2.energy_zone.active_count(),
                gs.player1
                    .stage
                    .stage
                    .iter()
                    .filter(|&&c| c >= 0)
                    .map(|&c| db.get_card(c).and_then(|x| x.cost).unwrap_or(0) as u32)
                    .sum::<u32>(),
                gs.player2
                    .stage
                    .stage
                    .iter()
                    .filter(|&&c| c >= 0)
                    .map(|&c| db.get_card(c).and_then(|x| x.cost).unwrap_or(0) as u32)
                    .sum::<u32>(),
            )
        };

        for _ in 0..600 {
            if logs && gs.current_phase != prev_phase {
                trace_rows.push(snap_row(&format!("ENTER:{:?}", gs.current_phase), &gs));
                prev_phase = gs.current_phase;
            }
            TurnEngine::check_victory_condition(&mut gs);
            if gs.game_result != GameResult::Ongoing {
                end_reason = "game_result";
                break;
            }
            if gs.turn_number == last_turn {
                stuck += 1;
                if stuck > 200 {
                    end_reason = "same_turn_iteration_cap";
                    break;
                }
            } else {
                stuck = 0;
                last_turn = gs.turn_number;
                // Guide development curve (section 1: T1=4, T2=9, T3=13). We
                // sample once per turn, at the first iteration of that turn,
                // so this is the board ENTERING turn N - which is the board
                // the previous turn built, i.e. the guide's T(N-1) line.
                // Summed (not maxed) across games so the report is a mean;
                // an earlier max/count pairing silently printed max/games and
                // read as a flat zero curve.
                let t = usize::from(gs.turn_number);
                if games == 1 && std::env::var("CURVE_DEBUG").is_ok() {
                    eprintln!(
                        "CURVE turn={} phase={:?} cost_p1={} cost_p2={}",
                        gs.turn_number,
                        gs.current_phase,
                        stage_cost(&gs.player1, &db),
                        stage_cost(&gs.player2, &db),
                    );
                }
                if t < CURVE_TURNS {
                    curve_p1[t] += stage_cost(&gs.player1, &db);
                    curve_p2[t] += stage_cost(&gs.player2, &db);
                    curve_p1_n[t] += 1;
                    curve_p2_n[t] += 1;
                }
                if logs {
                    timeline.push(format!(
                        "TIMELINE t{} succ {}-{} hand {}({}L)/{}({}L) en {}/{}",
                        gs.turn_number,
                        gs.player1.success_live_card_zone.cards.len(),
                        gs.player2.success_live_card_zone.cards.len(),
                        gs.player1.hand.cards.len(),
                        my_hand_lives(&gs, true, &db),
                        gs.player2.hand.cards.len(),
                        my_hand_lives(&gs, false, &db),
                        gs.player1.energy_zone.active_count(),
                        gs.player2.energy_zone.active_count(),
                    ));
                }
                if games == 1 && std::env::var("DUMP_STATE").is_ok() {
                    for (lbl, p) in [("P1", &gs.player1), ("P2", &gs.player2)] {
                        eprintln!(
                            "T{} {} hand={} deck={} wr={} livezone={} succ={} en={}",
                            gs.turn_number,
                            lbl,
                            p.hand.cards.len(),
                            p.main_deck.cards.len(),
                            p.waitroom.cards.len(),
                            p.live_card_zone.cards.len(),
                            p.success_live_card_zone.cards.len(),
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

            let policy_is_p1 = decision_player(&gs).id == gs.player1.id;
            let me = if policy_is_p1 { 0u8 } else { 1u8 };
            // Opt-in capture: first eligible idle-Main decision per player
            // turn, hard-capped at 20 per game. RNG states captured as-is;
            // full hidden state is written offline only.
            if let Some(directory) = &options.snapshots {
                if snapshot_eligible(&gs)
                    && captured_turns.len() < 20
                    && captured_turns.insert((gs.turn_number, me))
                {
                    let metadata = json!({
                        "game": games, "game_seed": engine_seed, "base_seed": options.seed,
                        "decision": decisions.decision + 1, "turn": gs.turn_number,
                        "phase": gs.current_phase, "policy_player": decision_player(&gs).id,
                        "active_player": gs.active_player().id,
                        "fold": corpus_fold(engine_seed),
                        "split_rule": "game_seed modulo 5 == 0: holdout; otherwise train",
                        "sampling": "first eligible Main decision per player turn; at most 20 per game",
                        "deck": deck_name, "bots": [p1_kind.name(), p2_kind.name()],
        "policy_environment": std::env::vars().filter(|(key, _)| {
            key.starts_with("V6_") || key.starts_with("V7_") || key.starts_with("V8_")
        }).collect::<std::collections::BTreeMap<_, _>>(),
                    });
                    let saved = SavedPosition::capture(&gs, &rng, metadata)?;
                    let restored = saved.restore(&db)?;
                    if !behaviorally_equal(&gs, &restored)? {
                        return Err("snapshot serialization is not lossless for this state".into());
                    }
                    saved.write(&directory.join(format!(
                        "{}-seed-{engine_seed:010}-decision-{:04}.rmp",
                        corpus_fold(engine_seed),
                        decisions.decision + 1,
                    )))?;
                }
            }

            if gs.has_pending_choice() {
                let action = choose_policy_action(
                    &gs,
                    &actions,
                    [p1_kind, p2_kind],
                    &v2_policy,
                    [&plan_p1, &plan_p2],
                    neural.as_ref(),
                    &mut rng,
                );
                decisions.execute(&mut audit, &mut gs, &actions, &action)?;
                total_actions += 1;
                continue;
            }

            // RPS: random for both (no information to decide with).
            if gs.current_phase == Phase::RockPaperScissors {
                let a = &actions[rng.range(actions.len())];
                decisions.execute(&mut audit, &mut gs, &actions, a)?;
                continue;
            }

            // S6: when this side won RPS, take second attacker.
            if gs.current_phase == Phase::ChooseFirstAttacker {
                let won_rps = gs.rps_winner == Some(if policy_is_p1 { 1 } else { 2 });
                let a = if won_rps {
                    actions
                        .iter()
                        .find(|a| {
                            a.action_type
                                == rabuka_engine::game_setup::ActionType::ChooseSecondAttacker
                        })
                        .unwrap_or(&actions[0])
                } else {
                    &actions[rng.range(actions.len())]
                };
                decisions.execute(&mut audit, &mut gs, &actions, a)?;
                continue;
            }

            // Mulligan: dispatched through the registry (v1/random keep the hand).
            if matches!(
                gs.current_phase,
                Phase::MulliganFirstAttacker | Phase::MulliganSecondAttacker
            ) {
                let a = choose_policy_action(
                    &gs,
                    &actions,
                    [p1_kind, p2_kind],
                    &v2_policy,
                    [&plan_p1, &plan_p2],
                    neural.as_ref(),
                    &mut rng,
                );
                decisions.execute(&mut audit, &mut gs, &actions, &a)?;
                continue;
            }

            // Live card set: dispatched through the registry (random rolls here
            // because the choice is a raw toggle index, not a policy).
            if matches!(
                gs.current_phase,
                Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker
            ) {
                let a = choose_policy_action(
                    &gs,
                    &actions,
                    [p1_kind, p2_kind],
                    &v2_policy,
                    [&plan_p1, &plan_p2],
                    neural.as_ref(),
                    &mut rng,
                );
                if a.action_type == rabuka_engine::game_setup::ActionType::ConfirmLiveCardSet {
                    live_decisions += 1;
                    if policy_is_p1 {
                        game_live_p1 += 1;
                    } else {
                        game_live_p2 += 1;
                    }
                    if gs.live_card_selected_indices.is_empty() {
                        live_folds += 1;
                    }
                }
                if trace {
                    let card_no = a
                        .parameters
                        .as_ref()
                        .and_then(|p| p.card_id)
                        .and_then(|cid| db.get_card(cid))
                        .map(|c| c.card_no.to_string())
                        .unwrap_or_default();
                    let sel: Vec<String> = gs
                        .live_card_selected_indices
                        .iter()
                        .map(|i| i.to_string())
                        .collect();
                    trace_rows.push(format!(
                        "{},{},{:?},{},CHOICE:{},{},sel=[{}],hand_lives={},live_p1={},live_p2={},succ_p1={},succ_p2={}",
                        games,
                        gs.turn_number,
                        gs.current_phase,
                        if policy_is_p1 { "P1" } else { "P2" },
                        a.action_type,
                        card_no,
                        sel.join("+"),
                        my_hand_lives(&gs, policy_is_p1, &db),
                        gs.player1.live_card_zone.cards.len(),
                        gs.player2.live_card_zone.cards.len(),
                        gs.player1.success_live_card_zone.cards.len(),
                        gs.player2.success_live_card_zone.cards.len(),
                    ));
                }
                decisions.execute(&mut audit, &mut gs, &actions, &a)?;
                continue;
            }

            // Main phase.
            if trace && policy_is_p1 {
                for (ai, aa) in actions.iter().enumerate() {
                    let cn = aa
                        .parameters
                        .as_ref()
                        .and_then(|p| p.card_id)
                        .and_then(|cid| db.get_card(cid))
                        .map(|c| c.card_no.to_string())
                        .unwrap_or_default();
                    trace_rows.push(format!(
                        "{},{},{:?},P1,OPT{},{},{},,,,,",
                        games, gs.turn_number, gs.current_phase, ai, aa.action_type, cn
                    ));
                }
            }
            // Main phase (and everything else policy-driven): registry dispatch.
            let action = choose_policy_action(
                &gs,
                &actions,
                [p1_kind, p2_kind],
                &v2_policy,
                [&plan_p1, &plan_p2],
                neural.as_ref(),
                &mut rng,
            );
            _main_decisions += 1;
            if gs.current_phase == Phase::Main {
                if !cur_is_main {
                    cur_is_main = true;
                    cur_main_plays = 0;
                }
                let is_pass = action.action_type == rabuka_engine::game_setup::ActionType::Pass;
                let is_deploy = action.action_type
                    == rabuka_engine::game_setup::ActionType::PlayMemberToStage
                    || (action.action_type == rabuka_engine::game_setup::ActionType::UseAbility
                        && action.parameters.as_ref().and_then(|p| p.use_baton_touch)
                            == Some(true));
                if is_deploy {
                    cur_main_plays += 1;
                }
                if is_pass {
                    main_phase_count += 1;
                    if cur_main_plays == 0 {
                        empty_main_count += 1;
                    }
                    cur_is_main = false;
                }
            } else {
                cur_is_main = false;
            }
            if action.action_type == rabuka_engine::game_setup::ActionType::Pass {
                _main_confirms += 1;
            }
            if trace {
                let card_no = action
                    .parameters
                    .as_ref()
                    .and_then(|p| p.card_id)
                    .and_then(|cid| db.get_card(cid))
                    .map(|c| c.card_no.to_string())
                    .unwrap_or_default();
                trace_rows.push(format!(
                    "{},{},{:?},{},{},{},{},{},{},{}",
                    games,
                    gs.turn_number,
                    gs.current_phase,
                    if policy_is_p1 { "P1" } else { "P2" },
                    action.action_type,
                    card_no,
                    gs.player1.live_card_zone.cards.len(),
                    gs.player2.live_card_zone.cards.len(),
                    gs.player1.success_live_card_zone.cards.len(),
                    gs.player2.success_live_card_zone.cards.len(),
                ));
            }
            decisions.execute(&mut audit, &mut gs, &actions, &action)?;
            total_actions += 1;
        }
        total_turns += gs.turn_number as u64;

        let z1 = gs.player1.success_live_card_zone.cards.len();
        let z2 = gs.player2.success_live_card_zone.cards.len();
        live_phases_p1 += game_live_p1;
        live_phases_p2 += game_live_p2;
        end_cost_p1 += i64::from(stage_cost(&gs.player1, &db));
        end_cost_p2 += i64::from(stage_cost(&gs.player2, &db));
        if gs.game_result != GameResult::Ongoing {
            end_reason = "game_result";
        }
        emit(
            &mut audit,
            &json!({
                "event": "game_end", "game": games, "turn": gs.turn_number,
                "end_reason": end_reason, "game_result": gs.game_result,
                "success_counts": [z1, z2], "decisions": decisions.decision,
                "execution_errors": decisions.errors,
            }),
        )?;
        final_hist
            .entry((z1 as u8, z2 as u8))
            .and_modify(|c| *c += 1)
            .or_insert(1);
        if z1 >= 3 && z2 <= 2 {
            wins[0] += 1;
        } else if z2 >= 3 && z1 <= 2 {
            wins[1] += 1;
        } else {
            draws += 1;
            if z1 >= 3 && z2 >= 3 {
                real_draws += 1;
            } else {
                stall_draws += 1;
            }
        }
        if matches!(gs.game_result, GameResult::Ongoing) {
            stuck_ends += 1;
        }
        outcome_rows.push(GameOutcome {
            engine_seed,
            z1: z1.min(255) as u8,
            z2: z2.min(255) as u8,
            result: classify(z1, z2),
            turns: gs.turn_number.min(255),
            live_p1: game_live_p1.min(u16::MAX as u64) as u16,
            live_p2: game_live_p2.min(u16::MAX as u64) as u16,
        });

        if logs {
            let dir = std::path::Path::new("../test_output/arena_logs");
            let result = if z1 >= 3 && z2 <= 2 {
                "P1 WINS"
            } else if z2 >= 3 && z1 <= 2 {
                "P2 WINS"
            } else {
                "DRAW"
            };
            let mut out = format!(
                "game {} | {}({}) vs {}({}) | final success {z1}-{z2} | {}\n=== RULE LOG ===\n",
                games,
                kind_name(p1_kind),
                wins[0],
                kind_name(p2_kind),
                wins[1],
                result
            );
            for line in &gs.rule_log {
                out.push_str(line);
                out.push('\n');
            }
            out.push_str("=== STRUCTURED (turn|player|category|text) ===\n");
            for e in &gs.structured_log {
                out.push_str(&format!(
                    "t{}|{}|{}|{}\n",
                    e.turn, e.player_label, e.category, e.text
                ));
            }
            std::fs::write(dir.join(format!("game_{games:03}.txt")), out)?;

            // Self-contained replay: header + turn timeline + decisions.
            if trace {
                let mut replay = format!(
                    "REPLAY game {games} | {result} | final {z1}-{z2}\n\
                     LIVE rows (structured log) = engine's own check verdicts\n\
                     TIMELINE rows = per-turn board summary\n\
                     ENTER rows = board at phase entry\n\
                     other rows = chosen action\n=== TIMELINE ===\n"
                );
                for t in &timeline {
                    replay.push_str(t);
                    replay.push('\n');
                }
                replay.push_str("=== LIVE CHECKS (engine verdicts) ===\n");
                for e in &gs.structured_log {
                    if e.category == "live_result" {
                        replay.push_str(&format!("t{}|{}\n", e.turn, e.text));
                    }
                }
                replay.push_str("=== EVENTS ===\n");
                for r in &trace_rows[game_start_idx.min(trace_rows.len())..] {
                    replay.push_str(r);
                    replay.push('\n');
                }
                std::fs::write(dir.join(format!("replay_game_{games:03}.txt")), replay)?;
            }
        }
        game_start_idx = trace_rows.len();
    }

    let secs = t0.elapsed().as_secs_f64();
    println!(
        "{} vs {}  E{} games in {:.1}s ({:.1} gps)\nP1({}) {} - P2({}) {} - draws {}\ntotal actions {} | avg turns/game {:.1}",
        kind_name(p1_kind),
        kind_name(p2_kind),
        games,
        secs,
        games as f64 / secs,
        kind_name(p1_kind),
        wins[0],
        kind_name(p2_kind),
        wins[1],
        draws,
        total_actions,
        total_turns as f64 / games.max(1) as f64,
    );
    let mut hist_lines: Vec<String> = final_hist
        .iter()
        .map(|((a, b), c)| format!("    final success {a}-{b}: {c} game(s)"))
        .collect();
    hist_lines.sort();
    println!(
        "DRAW TYPES: real 3-3 draws={} | stall/no-contest draws={} | stuck/timeout ends={}",
        real_draws, stall_draws, stuck_ends,
    );
    println!("FINAL SCORE HISTOGRAM (success_p1-success_p2 : count):");
    for l in hist_lines {
        println!("{l}");
    }
    let empty_main_rate = if main_phase_count > 0 {
        empty_main_count as f64 / main_phase_count as f64
    } else {
        0.0
    };
    let live_fold_rate = if live_decisions > 0 {
        live_folds as f64 / live_decisions as f64
    } else {
        0.0
    };
    println!(
        "DO-NOTHING telemetry: empty Main phases {}/{} = {:.1}% | live-set fold {}/{} = {:.1}%",
        empty_main_count,
        main_phase_count,
        empty_main_rate * 100.0,
        live_folds,
        live_decisions,
        live_fold_rate * 100.0,
    );
    report_health(
        &outcome_rows,
        live_phases_p1,
        live_phases_p2,
        &curve_p1,
        &curve_p2,
        &curve_p1_n,
        &curve_p2_n,
        (end_cost_p1, end_cost_p2),
    );
    if let Some(path) = &options.outcomes {
        write_outcomes(path, &outcome_rows)?;
        eprintln!("outcomes written to {}", path.display());
    }
    if let Some(path) = &options.vs {
        let baseline = read_outcomes(path)?;
        let label = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        println!();
        report_paired(&baseline, &outcome_rows, &label);
    }
    if trace {
        let path = std::path::Path::new("../test_output/bot_arena_trace.csv");
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(path, trace_rows.join("\n") + "\n")?;
        eprintln!("trace written to {}", path.display());
    }
    emit(&mut audit, &json!({"event": "run_end", "games": games}))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Paired-statistics tests. These pin the machinery that every
    // "v8 change is an improvement" claim will rest on, so they check against
    // values that can be verified by hand.

    /// With no discordant pairs there is nothing to test.
    #[test]
    fn mcnemar_is_one_when_nothing_discords() {
        assert_eq!(mcnemar_exact_p(0, 0), 1.0);
    }

    /// All discordant pairs going one way is the strongest possible result:
    /// P = 2 * (1/2)^n, doubled and clamped at 1.
    #[test]
    fn mcnemar_matches_hand_computed_values() {
        // 1-0 split, n=1: 2 * 0.5 = 1.0
        assert!((mcnemar_exact_p(1, 0) - 1.0).abs() < 1e-12);
        // 2-0 split, n=2: 2 * (1/4) = 0.5
        assert!((mcnemar_exact_p(2, 0) - 0.5).abs() < 1e-12);
        // 3-0 split, n=3: 2 * (1/8) = 0.25
        assert!((mcnemar_exact_p(3, 0) - 0.25).abs() < 1e-12);
        // 5-0 split, n=5: 2 * (1/32) = 0.0625
        assert!((mcnemar_exact_p(5, 0) - 0.0625).abs() < 1e-12);
        // 6-3 split, n=9. P(X<=3) = (1+9+36+84)/512 = 130/512, doubled.
        assert!((mcnemar_exact_p(6, 3) - 0.507_812_5).abs() < 1e-6);
    }

    /// The test is symmetric and peaks at an even split: no signal, p = 1.
    #[test]
    fn mcnemar_is_symmetric_and_peaks_at_an_even_split() {
        assert!((mcnemar_exact_p(7, 3) - mcnemar_exact_p(3, 7)).abs() < 1e-12);
        assert!((mcnemar_exact_p(4, 4) - 1.0).abs() < 1e-12);
    }

    /// Enough discordant pairs the same way should clear p < 0.05, which is
    /// the bar a change has to pass to be shippable - and a near-even split
    /// must not.
    #[test]
    fn mcnemar_reaches_significance_on_a_real_skew() {
        // 70-30 of 100 discordant games: about p = 1e-4.
        assert!(mcnemar_exact_p(70, 30) < 0.001);
        // 55-45 of 100: about p = 0.37, not a finding.
        assert!(mcnemar_exact_p(55, 45) > 0.05);
        assert!(mcnemar_exact_p(100, 100) >= 0.05);
    }

    /// Wilson interval must stay inside [0,1] and must actually exclude 0.5
    /// when the observed rate is far enough from it - the property a normal
    /// approximation gets wrong at the extremes.
    #[test]
    fn wilson_interval_brackets_the_estimate_and_excludes_half() {
        let (lo, hi) = wilson_interval(700, 1000);
        assert!(lo < 0.7 && hi > 0.7);
        assert!(lo > 0.5);
        let (lo, hi) = wilson_interval(10, 1000);
        assert!(lo >= 0.0 && hi < 0.05);
        let (lo, hi) = wilson_interval(0, 0);
        assert_eq!((lo, hi), (0.0, 0.0));
    }

    /// Scoring must follow the engine rule 1.2.1.1 / 1.2.1.2 exactly, so the
    /// paired test classifies the same states the arena tallies.
    #[test]
    fn classify_follows_the_success_zone_rule() {
        assert_eq!(classify(3, 2), 1);
        assert_eq!(classify(3, 0), 1);
        assert_eq!(classify(2, 3), -1);
        assert_eq!(classify(3, 3), 0, "mutual 3-3 is a draw game");
        assert_eq!(classify(2, 2), 0, "neither reached 3");
        assert_eq!(classify(1, 0), 0);
    }

    /// The deal for game N must be a pure function of (seed, N), or pairing is
    /// meaningless. This is the assumption the whole A/B design rests on.
    #[test]
    fn game_seeds_are_a_pure_function_of_seed_and_index() {
        assert_eq!(game_seeds(11, 1), game_seeds(11, 1));
        assert_ne!(game_seeds(11, 1).0, game_seeds(11, 2).0);
        assert_ne!(game_seeds(11, 1).0, game_seeds(12, 1).0);
    }

    /// A paired test is only meaningful on a fixed game count.
    #[test]
    fn paired_mode_requires_a_fixed_game_count() {
        let Err(err) = Options::parse(&args(&["v8", "v7", "10", "--vs", "base.csv"]), None) else {
            panic!("--vs without --games must be rejected");
        };
        assert!(err.to_string().contains("--vs"), "got: {err}");
        assert!(Options::parse(
            &args(&["v8", "v7", "10", "--games", "100", "--vs", "base.csv"]),
            None
        )
        .is_ok());
    }

    /// The guide's pace metric: placements divided by live phases actually
    /// taken. A side that never took a live phase has an undefined rate, not
    /// a zero rate - conflating them would make a folded bot look "slow"
    /// instead of "absent", which is the opposite diagnosis.
    #[test]
    fn pace_is_placements_over_live_phases_and_undefined_when_absent() {
        let g = |z, lp| GameOutcome {
            engine_seed: 1,
            z1: z,
            z2: 0,
            result: 1,
            turns: 6,
            live_p1: lp,
            live_p2: 0,
        };
        assert_eq!(g(3, 3).pace(0), Some(1.0), "the guide's ~1.0 target");
        assert_eq!(g(1, 3).pace(0), Some(1.0 / 3.0), "the ~0.33 defect band");
        assert_eq!(g(3, 1).pace(0), Some(3.0), "multiple placements per phase is legal");
        assert_eq!(g(0, 4).pace(0), Some(0.0), "phases but no placement is a real zero");
        assert_eq!(g(0, 0).pace(0), None, "no phase means undefined, not zero");
        assert_eq!(g(3, 3).pace(1), None, "P2 took no phases");
    }

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn fixed_games_ignore_wall_time_and_parse_flags_without_a_deck() {
        let options = Options::parse(
            &args(&[
                "v7",
                "random",
                "0",
                "--games",
                "2",
                "--audit",
                "audit.jsonl",
            ]),
            None,
        )
        .unwrap();
        assert_eq!(options.deck, "5CP3Z idou");
        assert!(options.should_start_game(1, std::time::Duration::from_secs(9999)));
        assert!(!options.should_start_game(2, std::time::Duration::ZERO));
        let env = Options::parse(&[], Some("3")).unwrap();
        assert!(env.should_start_game(2, std::time::Duration::from_secs(9999)));
        let timed = Options::parse(&[], None).unwrap();
        assert!(!timed.should_start_game(0, std::time::Duration::from_secs(10)));
    }

    #[test]
    fn invalid_or_unbounded_audit_options_fail() {
        for values in [
            vec!["--audit"],
            vec!["--audit", "out"],
            vec!["--games", "0"],
            vec!["--seed", "0"],
            vec!["--games", "bad"],
            vec!["unknown"],
            vec!["--unknown"],
        ] {
            assert!(Options::parse(&args(&values), None).is_err());
        }
        assert!(Options::parse(&[], Some("bad")).is_err());
        assert_eq!(
            Options::parse(&args(&["--games", "2"]), Some("bad"))
                .unwrap()
                .games,
            Some(2)
        );
    }

    #[test]
    fn seeds_are_explicit_nonzero_and_wrap() {
        assert_eq!(game_seeds(1, 1).0, 1);
        assert_eq!(game_seeds(1, 2).0, 2);
        assert_eq!(game_seeds(u32::MAX, 2).0, 1);
        assert_ne!(game_seeds(1, 1).1, game_seeds(1, 2).1);
    }

    #[test]
    fn audit_redacts_hidden_action_identity_and_keeps_complete_visible_actions() {
        let db = fresh_database();
        let mut own = rabuka_engine::player::Player::new("p1".into(), "P1".into(), true);
        let mut opponent = rabuka_engine::player::Player::new("p2".into(), "P2".into(), false);
        let mut ids: Vec<i16> = db.cards.keys().copied().collect();
        ids.sort();
        own.hand.cards.push(ids[0]);
        opponent.stage.stage[0] = ids[1];
        opponent.hand.cards.push(ids[2]);
        opponent.main_deck.cards.push(ids[3]);
        opponent.live_card_zone.cards.push(ids[4]);
        let mut gs = GameState::new(own, opponent, db);
        gs.current_phase = Phase::LiveCardSetFirstAttacker;
        let actions = game_setup::generate_possible_actions(&gs);
        let chosen = actions
            .iter()
            .find(|a| a.action_type == game_setup::ActionType::SelectLiveCard)
            .unwrap();
        let mut audit = DecisionAudit {
            game: 1,
            decision: 1,
            ..Default::default()
        };
        let row = audit.record(&gs, &actions, chosen).unwrap();
        assert_eq!(
            row["available_actions"].as_array().unwrap().len(),
            actions.len()
        );
        assert_eq!(
            row["chosen"]["parameters"],
            serde_json::to_value(chosen).unwrap()["parameters"]
        );
        assert_eq!(
            row["view"]["own"]["hand"][0]["card_no"],
            gs.card_database.get_card(ids[0]).unwrap().card_no.as_ref()
        );
        assert_eq!(row["view"]["opponent_public"].as_object().unwrap().len(), 3);
        for id in &ids[2..5] {
            assert!(!row.to_string().contains(&format!("\"card_id\":{id},")));
        }
        audit.decision = 2;
        let repeated = audit.record(&gs, &actions, chosen).unwrap();
        assert_eq!(repeated["boundary_step"], 2);
        assert_eq!(repeated["repeated_visible_decision_from"], 1);
        let mut hidden = chosen.clone();
        hidden.action_type = game_setup::ActionType::ChoiceSelect;
        hidden.parameters.as_mut().unwrap().card_id = Some(ids[2]);
        hidden.description = "hidden identity".into();
        let redacted = audit_action(&gs, &hidden).unwrap();
        assert_eq!(redacted["identity_redacted"], true);
        assert!(redacted["parameters"]["card_id"].is_null());
        assert!(redacted["description"].is_null());
    }

    #[test]
    fn pending_choice_routes_policy_and_audit_to_owner_before_phase_dispatch() {
        use rabuka_engine::ability::types::Choice;

        let db = fresh_database();
        let mut ids: Vec<i16> = db.cards.keys().copied().collect();
        ids.sort();
        let p1 = rabuka_engine::player::Player::new("p1".into(), "P1".into(), true);
        let p2 = rabuka_engine::player::Player::new("p2".into(), "P2".into(), false);
        let mut gs = GameState::new(p1, p2, db);
        gs.player1.hand.cards.push(ids[0]);
        gs.player2.hand.cards.extend_from_slice(&ids[1..3]);
        gs.turn_number = 6;
        let v2 = strategy_v2::V2Policy::default();
        let plan_p1 = policy_call(|| strategy_v3::V3Plan::detect(&gs, 0, &gs.card_database));
        let plan_p2 = policy_call(|| strategy_v3::V3Plan::detect(&gs, 1, &gs.card_database));
        let choice = Choice::select_cards("hand", 1, "Select card", false)
            .target_player_id(Some("opponent".into()))
            .picker(Some("p2".into()))
            .build();
        gs.ability_queue.pause_for_choice(choice);
        gs.ability_queue
            .current_entry_mut()
            .unwrap()
            .choice_player_id = Some("p2".into());

        for phase in [
            Phase::Main,
            Phase::MulliganFirstAttacker,
            Phase::LiveCardSetFirstAttacker,
        ] {
            gs.current_phase = phase;
            assert_eq!(gs.active_player().id, "p1");
            assert!(!gs.can_player_act(0));
            assert!(gs.can_player_act(1));
            assert_eq!(decision_player(&gs).id, "p2");
            assert_eq!(policy_route(&gs), PolicyRoute::Action);
            let actions = game_setup::generate_possible_actions(&gs);
            assert_eq!(actions.len(), 2);
            assert!(actions
                .iter()
                .all(|a| a.action_type == game_setup::ActionType::ChoiceSelect));
            let wrong_policy =
                policy_call(|| BotKind::V1.choose_action(&gs, &actions, 0, &v2, &plan_p1));
            let wrong_value = serde_json::to_value(&wrong_policy).unwrap();
            let seed = (1..1000)
                .find(|&seed| {
                    let index = Lcg(seed).range(actions.len());
                    serde_json::to_value(&actions[index]).unwrap() != wrong_value
                })
                .unwrap();
            let expected = actions[Lcg(seed).range(actions.len())].clone();
            let chosen = choose_policy_action(
                &gs,
                &actions,
                [BotKind::V1, BotKind::Random],
                &v2,
                [&plan_p1, &plan_p2],
                None,
                &mut Lcg(seed),
            );
            assert_eq!(
                serde_json::to_value(&chosen).unwrap(),
                serde_json::to_value(&expected).unwrap()
            );
            assert_ne!(serde_json::to_value(&chosen).unwrap(), wrong_value);
            let mut audit = DecisionAudit {
                game: 4,
                decision: 73,
                ..Default::default()
            };
            let row = audit.record(&gs, &actions, &chosen).unwrap();
            assert_eq!(row["policy_player"], "p2");
            assert_eq!(row["active_player"], "p1");
            assert_eq!(row["pending_choice_player"], "p2");
            assert_eq!(row["view"]["own"]["player"], "p2");
            assert_eq!(row["view"]["own"]["hand"].as_array().unwrap().len(), 2);
            assert_eq!(
                row["chosen"]["resolved_card"]["card_id"],
                chosen.parameters.as_ref().unwrap().card_id.unwrap()
            );
            let mut hidden = chosen.clone();
            hidden.parameters.as_mut().unwrap().card_id = Some(ids[0]);
            assert_eq!(
                audit_action(&gs, &hidden).unwrap()["identity_redacted"],
                true
            );
            gs.ability_queue
                .current_entry_mut()
                .unwrap()
                .choice_player_id = Some("p1".into());
            let next = audit.record(&gs, &actions, &chosen).unwrap();
            assert_eq!(next["policy_player"], "p1");
            assert_eq!(next["boundary_id"], 2);
            assert_eq!(next["boundary_step"], 1);
            gs.ability_queue
                .current_entry_mut()
                .unwrap()
                .choice_player_id = Some("p2".into());
        }
        gs.ability_queue
            .pause_for_auto_ability_choice(Choice::SelectAutoAbility {
                player_id: "p2".into(),
                options: vec![],
                description: "Choose ability".into(),
                description_en: None,
                description_ja: None,
            });
        assert!(gs.get_pending_choice_player_id().is_none());
        assert_eq!(decision_player(&gs).id, "p2");
    }

    #[test]
    fn policy_calls_restore_rng_immediately_and_on_unwind() {
        let _restore_rng = RngRestore(rabuka_engine::rng::checkpoint());
        rabuka_engine::rng::seed(7123);
        let before = rabuka_engine::rng::checkpoint();
        let expected = rabuka_engine::rng::rand_range(1_000_000);
        rabuka_engine::rng::restore(before);
        for _ in 0..2 {
            let sampled = policy_call(|| rabuka_engine::rng::rand_range(1_000_000));
            assert_eq!(sampled, expected);
            assert_eq!(rabuka_engine::rng::checkpoint(), before);
        }
        let result = std::panic::catch_unwind(|| {
            policy_call(|| {
                rabuka_engine::rng::rand_range(1_000_000);
                panic!("policy failed");
            })
        });
        assert!(result.is_err());
        assert_eq!(rabuka_engine::rng::checkpoint(), before);
        assert_eq!(rabuka_engine::rng::rand_range(1_000_000), expected);
    }

    #[test]
    fn confusable_deck_line_resolves_consistently_and_audit_exposes_card_no() {
        let db = fresh_database();
        let deck = load_test_deck(&db, "5CP3Z idou");
        let requested: Vec<&String> = deck
            .iter()
            .filter(|n| n.to_uppercase().contains("BP4-011"))
            .collect();
        assert_eq!(requested.len(), 2, "deck must contain the bp4-011 pair");
        let resolved: Vec<String> = requested
            .iter()
            .map(|n| {
                db.get_card_id(n.as_str())
                    .and_then(|id| db.get_card(id))
                    .map(|c| c.card_no.to_string())
                    .unwrap_or_default()
            })
            .collect();
        assert!(
            resolved.iter().all(|r| r.starts_with("PL!SP-bp4-011-")),
            "both copies must resolve to a bp4-011 print, got {resolved:?}"
        );
        assert_eq!(
            resolved[0], resolved[1],
            "identical deck lines must resolve to the same print within a build"
        );
    }

    #[test]
    fn snapshot_options_are_bounded_and_split_is_grouped_by_seed() {
        assert!(Options::parse(&args(&["--snapshots", "positions"]), None).is_err());
        let options =
            Options::parse(&args(&["--games", "2", "--snapshots", "positions"]), None).unwrap();
        assert_eq!(options.snapshots, Some(PathBuf::from("positions")));
        assert!(options.audit.is_none());
        let compare = Options::parse(&args(&["--compare", "positions"]), None).unwrap();
        assert_eq!(compare.compare, Some(PathBuf::from("positions")));
        assert!(Options::parse(
            &args(&["--compare", "p", "--snapshots", "q", "--games", "1"]),
            None
        )
        .is_err());
        assert!(Options::parse(&args(&["--compare", "p", "--trace"]), None).is_err());
        assert!(Options::parse(&args(&["--compare", "p", "v6", "v7"]), None).is_err());
        for seed in 1..100 {
            let fold = corpus_fold(seed);
            assert_eq!(fold == "holdout", seed % 5 == 0);
        }
    }

    #[test]
    fn saved_real_deck_roundtrip_preserves_offers_choices_rng_and_original() {
        let _restore_rng = RngRestore(rabuka_engine::rng::checkpoint());
        rabuka_engine::rng::seed(17);
        let mut db = fresh_database();
        let nums = load_test_deck(&db, "5CP3Z idou");
        let (t1, t2) = build_templates(&mut db, &nums, &nums);
        let mut gs = deal_from_templates(&db, &t1, &t2);
        let mut setup_rng = Lcg(17321);
        for _ in 0..100 {
            if snapshot_eligible(&gs) {
                break;
            }
            if game_setup::auto_advance_one(&mut gs) {
                continue;
            }
            let actions = game_setup::generate_possible_actions(&gs);
            let action = actions
                .iter()
                .find(|a| a.action_type == game_setup::ActionType::ConfirmMulligan)
                .unwrap_or(&actions[setup_rng.range(actions.len())]);
            game_setup::execute_action(&mut gs, action).unwrap();
            game_setup::settle_single_player_state(&mut gs);
        }
        assert!(snapshot_eligible(&gs));
        // Non-serde engine internals must survive the roundtrip.
        gs.game_state_history.push(1234567);
        gs.debug_trace.push("snapshot test".into());
        let db_before = gs.card_database.cards.len();
        let arena_rng = Lcg(9876);
        let rng_before = rabuka_engine::rng::checkpoint();
        let saved = SavedPosition::capture(
            &gs,
            &arena_rng,
            json!({"game_seed": 17, "fold": corpus_fold(17)}),
        )
        .unwrap();
        // Capture must not disturb either RNG.
        assert_eq!(rng_before, rabuka_engine::rng::checkpoint());
        assert_eq!(arena_rng.0, saved.arena_rng);
        // Restore replays the exact global engine RNG stream.
        let expected_rng: Vec<_> = (0..8)
            .map(|_| rabuka_engine::rng::rand_range(1_000_000))
            .collect();
        rabuka_engine::rng::restore(saved.engine_rng);
        assert_eq!(
            expected_rng,
            (0..8)
                .map(|_| rabuka_engine::rng::rand_range(1_000_000))
                .collect::<Vec<_>>()
        );
        // Arena LCG state roundtrips.
        assert_eq!(Lcg(saved.arena_rng).next_u64(), Lcg(arena_rng.0).next_u64());
        // File write: atomic create, refuses overwrite, reload identical.
        let directory =
            std::env::temp_dir().join(format!("rabuka-snapshot-test-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("position.rmp");
        saved.write(&path).unwrap();
        assert!(saved.write(&path).is_err());
        let mut loaded: SavedPosition =
            rmp_serde::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        // Physical duplicate-deck IDs: real decks need far more physical IDs
        // than template card_nos.
        let template_count = fresh_database().cards.len();
        assert!(
            saved.cards.len() > template_count + 100,
            "real decks require unique physical duplicate-card IDs"
        );
        let restored = loaded.restore(&fresh_database()).unwrap();
        // Offers and v6/v7 numeric behavior identical after restore.
        let equivalence_rng = rabuka_engine::rng::checkpoint();
        assert!(behaviorally_equal(&gs, &restored).unwrap());
        assert_eq!(rabuka_engine::rng::checkpoint(), equivalence_rng);
        assert_eq!(restored.card_database.cards.len(), db_before);
        for (&id, card) in &gs.card_database.cards {
            assert_eq!(
                restored.card_database.get_card(id).unwrap().card_no,
                card.card_no
            );
        }
        assert_eq!(restored.game_state_history, gs.game_state_history);
        assert_eq!(restored.debug_trace, gs.debug_trace);
        // compare is RNG-clean, deterministic across surrounding RNG use,
        // scores every offered action, and chooses an offered index.
        let compare_before = rabuka_engine::rng::checkpoint();
        let first = compare_position(&loaded, &db).unwrap();
        assert_eq!(compare_before, rabuka_engine::rng::checkpoint());
        rabuka_engine::rng::seed(998877);
        let second = compare_position(&loaded, &db).unwrap();
        assert_eq!(first, second);
        assert_eq!(rabuka_engine::rng::checkpoint(), 998877);
        for name in ["v6", "v7"] {
            assert_eq!(
                first["bots"][name]["scores"].as_array().unwrap().len(),
                saved.offers.as_array().unwrap().len()
            );
            assert!(first["bots"][name]["chosen_index"].as_u64().is_some());
        }
        // Original card DB unmutated by capture/compare paths; capture-time
        // RNG neutrality was asserted right after SavedPosition::capture, and
        // compare-time neutrality (relative to each call's entry) above — the
        // global state here is deliberately 998877 from the determinism probe.
        assert_eq!(gs.card_database.cards.len(), db_before);
        // Rejection paths: bad schema, unknown card, ineligible phase.
        loaded.schema = 999;
        assert!(loaded.restore(&db).is_err());
        loaded.schema = 1;
        loaded.cards[0].card_no = "unsupported-card".into();
        assert!(loaded.restore(&db).is_err());
        gs.current_phase = Phase::RockPaperScissors;
        assert!(SavedPosition::capture(&gs, &arena_rng, json!({})).is_err());
        std::fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn jsonl_errors_propagate_including_flush() {
        struct Fail(bool);
        impl Write for Fail {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                if self.0 {
                    Ok(buf.len())
                } else {
                    Err(std::io::Error::other("write failure"))
                }
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Err(std::io::Error::other("flush failure"))
            }
        }
        assert!(write_jsonl(&mut Fail(false), &json!({})).is_err());
        assert!(write_jsonl(&mut Fail(true), &json!({})).is_err());
        let mut output = Vec::new();
        write_jsonl(&mut output, &json!({"event": "test"})).unwrap();
        assert_eq!(output.last(), Some(&b'\n'));
        assert_eq!(
            serde_json::from_slice::<Value>(&output).unwrap()["event"],
            "test"
        );
    }
}
