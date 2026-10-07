//! Move-level statistics over a full arena run.
//!
//! Joins the per-decision trace to the per-game outcome and asks, for every
//! action type and every card: when a bot plays this, how often does it go on
//! to win? Lift against the base rate is the signal — a card with a strongly
//! negative lift is costing games, and that is a hypothesis with a number on
//! it rather than an intuition.
//!
//! The engine sustains tens of games per second, so this is a real sample:
//! 20k games is ~400k decisions, which is enough to separate a 2pp effect from
//! noise without a bot change in the loop.
//!
//! Usage: move_stats <trace.csv> <outcomes.csv> [botname]

use std::collections::HashMap;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: move_stats <trace.csv> <outcomes.csv> [bot]");
        return;
    }
    let only_bot = args.get(3).cloned();

    // The trace already carries the running success counts in its last two
    // columns, so the outcome is derivable per game from the trace itself.
    // Joining a separate outcomes CSV looked cleaner but silently produced
    // zero joined decisions on a 2.7M-line run, and a tool that reports
    // "base rate over N games" while aggregating nothing is worse than
    // useless — it looks like a null result.
    //
    // game -> (P1 won, P2 won), taken from the LAST row seen for that game,
    // where both players' success counts are final.
    let bytes = std::fs::read(&args[0]).expect("trace");
    let raw = String::from_utf8_lossy(&bytes);
    let mut first: HashMap<String, Vec<String>> = HashMap::new();
    let mut printed = false;
    for line in raw.lines().skip(1) {
        let f: Vec<&str> = line.split(',').collect();
        if !printed {
            eprintln!("first data row: {} fields = {:?}", f.len(), f);
            printed = true;
        }
        if f.len() < 6 {
            continue;
        }
        let game = f[0].to_string();
        let entry = first.entry(game).or_default();
        if f.len() >= 10 {
            // Keep the widest row seen for this game: only rows carrying the
            // final success columns are useful, and later rows in a game are
            // the ones with counts filled in.
            if f[8].trim().parse::<i32>().is_ok() && f[9].trim().parse::<i32>().is_ok() {
                *entry = f.iter().map(|s| s.to_string()).collect();
            } else if entry.is_empty() {
                *entry = f.iter().map(|s| s.to_string()).collect();
            }
        } else if entry.is_empty() {
            *entry = f.iter().map(|s| s.to_string()).collect();
        }
    }
    let mut outcome: HashMap<String, (bool, bool)> = HashMap::new();
    for (game, row) in first {
        if row.len() >= 10 {
            if let (Ok(a), Ok(b)) = (row[8].trim().parse::<i32>(), row[9].trim().parse::<i32>()) {
                outcome.insert(game, (a > b, b > a));
            }
        }
    }
    eprintln!("outcomes derived: {}", outcome.len());
    if outcome.is_empty() {
        eprintln!("no games had parseable success columns");
        return;
    }

    // (kind, card) -> [games_played, games_won]
    let mut stat: HashMap<(String, String), (u32, u32)> = HashMap::new();
    let mut by_turn: HashMap<(u8, String), (u32, u32)> = HashMap::new();
    let mut games_seen = 0usize;

    for line in raw.lines().skip(1) {
        let f: Vec<&str> = line.split(',').collect();
        if f.len() < 6 {
            continue;
        }
        let game = f[0];
        let turn: u8 = f[1].parse().unwrap_or(0);
        let player = f[3];
        let action = f[4];
        let card = f[5];
        // OPTn rows are the OFFERED actions; only the taken one is a decision.
        if action.starts_with("OPT") || action.starts_with("CHOICE") {
            continue;
        }
        let seat = match player {
            "P1" => 0,
            "P2" => 1,
            _ => continue,
        };
        if let Some(want) = &only_bot {
            // The trace does not name the bot, so `bot` filters by seat via a
            // P1/P2 argument instead; kept for symmetry with the arena flags.
            let _ = want;
        }
        let Some(&(p1_won, p2_won)) = outcome.get(game) else {
            continue;
        };
        if seat == 0 {
            games_seen += 1;
        }
        let won = if seat == 0 { p1_won } else { p2_won };
        let key = (action.to_string(), card.to_string());
        let e = stat.entry(key).or_insert((0, 0));
        e.0 += 1;
        e.1 += u32::from(won);
        let e = by_turn.entry((turn, action.to_string())).or_insert((0, 0));
        e.0 += 1;
        e.1 += u32::from(won);
    }
    eprintln!("P1 decisions joined: {games_seen}");

    // Base rate: every game, P1's win rate.
    let total_p1 = outcome.values().filter(|v| v.0).count();
    let base = total_p1 as f64 / outcome.len() as f64;
    println!("=== base P1 win rate: {:.4} over {} games ===", base, outcome.len());
    println!();
    println!("--- action type by lift (P(lose | action) - base loss rate) ---");
    let mut rows: Vec<(String, String, u32, u32, f64)> = stat
        .iter()
        .filter(|((_a, _), (n, _))| *n >= 200)
        .map(|((a, c), (n, w))| (a.clone(), c.clone(), *n, *w, *w as f64 / *n as f64))
        .collect();
    rows.sort_by(|x, y| x.4.partial_cmp(&y.4).unwrap_or(std::cmp::Ordering::Equal));
    println!("{:<26} {:<22} {:>7} {:>8} {:>9}", "action", "card", "n", "winrate", "lift");
    for (a, c, n, _, w) in rows.iter().take(10).chain(rows.iter().rev().take(10)) {
        println!("{:<26} {:<22} {:>7} {:>8.3} {:>+9.3}", a, c, n, w, w - base);
    }
    println!();
    println!("--- by turn, for use_ability / pass / play_member_to_stage ---");
    let mut trows: Vec<(u8, String, u32, f64)> = by_turn
        .iter()
        .filter(|(_, (n, _))| *n >= 200)
        .map(|((t, a), (n, w))| (*t, a.clone(), *n, *w as f64 / *n as f64))
        .collect();
    trows.sort_by_key(|(t, _, _, _)| *t);
    for (t, a, n, w) in trows.iter().filter(|(_, a, _, _)| {
        a == "use_ability" || a == "pass" || a == "play_member_to_stage"
    }) {
        println!("  T{:<2} {:<24} n={:<7} winrate={:.3} lift={:+.3}", t, a, n, w, w - base);
    }
}
