//! Opening-curve reachability calculator.
//!
//! This is a measurement tool, not a bot. It answers a question no bot change
//! can answer directly: for a given 72-card list, how often is the guide's
//! curve actually reachable, and where does it break?
//!
//! Why it exists. The guide's template is explicit — "turn 1 play a 4-cost,
//! turn 2 go 4 -> 9" — and the arena measures the bot sitting at
//! T2 3.8 / T3 7.6 / T4 11.2 against a guide band of 4 / 9 / 13. The
//! move-level log then showed why: the opening hand for this list is
//!
//!     5 桜小路 | — Special | 7 鬼塚冬 | 17 澁谷 | — Special | 2 葉月 | 2 ウィー
//!
//! which contains no cost-4 at all, and no cost-9. A deck that opens 2 / 2 / 5
//! / 7 / 17 has no connected ladder, and the bot plays a 2. Whether that is a
//! mulligan bug or a deck problem is exactly what this tool separates, and it
//! does so by putting a probability on the guide's numbers instead of arguing
//! about them.
//!
//! The reachability model is the real rule, not an approximation. Batoning
//! moves a stage member to the waiting room and its cost is added to the new
//! card, so playing a card of cost C against a current maximum of M costs
//! `min(C, M)` energy and raises the board total by `C - M`. That is what makes
//! a 4 -> 9 jump possible in one turn and why a deck with no 4 and no 9 cannot
//! reach 9 on turn 2 no matter how the bot plays.
//!
//! Usage:
//!   curve_prob <deck.json|index> [trials]
//!   curve_prob list

use std::collections::HashMap;

use serde_json::Value;

#[derive(Clone, Debug)]
struct Member {
    cost: i32,
    count: usize,
}

fn load_members(path: &str) -> Vec<Member> {
    let raw = std::fs::read_to_string(path).expect("deck file");
    let cards = serde_json::from_str::<Value>(&raw).expect("deck json");
    let mut counts: HashMap<i32, usize> = HashMap::new();
    if let Some(list) = cards.as_array() {
        for card in list {
            let is_member = card
                .get("type")
                .and_then(|t| t.as_str())
                .map(|t| t.contains("member"))
                .unwrap_or(false);
            if !is_member {
                continue;
            }
            // `null` cost is an energy card that is also a member; it cannot
            // advance a cost curve, so it is excluded.
            if let Some(cost) = card.get("cost").and_then(|c| c.as_i64()) {
                *counts.entry(cost as i32).or_default() += 1;
            }
        }
    }
    let mut members: Vec<Member> = counts
        .into_iter()
        .map(|(cost, count)| Member { cost, count })
        .collect();
    members.sort_by_key(|m| m.cost);
    members
}

/// Small deterministic LCG so a run is reproducible.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 11
    }
    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
}

/// One card, drawn from the deck multiset.
#[derive(Clone, Copy)]
struct Card {
    cost: i32,
}

/// Greedy develop for one turn, carrying the board across turns.
///
/// Batoning rule: replacing the current maximum `m` with a card of cost `c`
/// costs `min(c, m)` energy and moves the BOARD TOTAL by `c - m`. That is
/// what makes 4 -> 9 possible in a single turn, and it is why the guide's
/// numbers are cumulative board totals rather than per-turn spend.
///
/// The first version reset the board every turn and reported `0.000` for
/// "reach 9 by T2" even on a deck containing four 4s and two 9s — it was
/// measuring per-turn spend, not the curve. Passing the board through fixes
/// it, and the passing-in-energy model is the arena's: the bot starts with 4
/// active and gains 1 per turn.
fn develop(board: &mut Vec<i32>, hand: &[Card], energy: i32) -> i32 {
    let mut left = energy;
    let mut order: Vec<Card> = hand.to_vec();
    // Highest cost first: a big member held to late is dead weight, a big
    // member deployed early is the whole point of the curve.
    order.sort_by(|a, b| b.cost.cmp(&a.cost));
    for card in order {
        if card.cost <= 0 {
            continue;
        }
        let max_board = board.iter().copied().max().unwrap_or(0);
        let free_slot = board.len() < 3;
        let (price, replaced) = if free_slot && card.cost <= left {
            (card.cost, None)
        } else if max_board > 0 {
            let price = card.cost.min(max_board);
            if price <= left {
                (price, Some(max_board))
            } else {
                continue;
            }
        } else {
            continue;
        };
        if price > left {
            continue;
        }
        if let Some(old) = replaced {
            if let Some(slot) = board.iter().position(|c| *c == old) {
                board.remove(slot);
            }
        }
        board.push(card.cost);
        left -= price;
    }
    board.iter().sum()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: curve_prob <deck.json|index> [trials]");
        eprintln!("       curve_prob list");
        return;
    }
    if args[1] == "list" {
        for entry in std::fs::read_dir("baked").expect("baked/") {
            if let Ok(e) = entry {
                let path = e.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    let len = e.metadata().map(|m| m.len()).unwrap_or(0);
                    if len > 0 {
                        println!("{}", path.display());
                    }
                }
            }
        }
        return;
    }

    let path = if args[1].ends_with(".json") {
        args[1].clone()
    } else {
        let idx: usize = args[1].parse().expect("deck index");
        format!("baked/deck_{idx}_cards.json")
    };
    let trials: usize = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(200_000);
    let members = load_members(&path);
    if members.is_empty() {
        eprintln!("no members with a cost found in {path}");
        return;
    }

    println!("=== curve reachability: {path} ===");
    let total: usize = members.iter().map(|m| m.count).sum();
    println!("members with a cost: {total}");
    print!("cost histogram:");
    for m in &members {
        print!(" {}x{}", m.cost, m.count);
    }
    println!();

    // Deck of individual cards for drawing.
    let mut deck: Vec<Card> = Vec::with_capacity(total);
    for m in &members {
        for _ in 0..m.count {
            deck.push(Card { cost: m.cost });
        }
    }

    // The mulligan: discard up to 3, redraw. v7's default rule keeps at most
    // 3 lives and dumps the most expensive members, which is modelled here as
    // the "stock" policy; `keep_4` is the guide's policy — dump anything that
    // is not on a path to 4.
    let mut rng = Rng(0x5EED_C0DE_C0DE_1234);
    let mut reach = [0u32; 3]; // P(cost >= 4 by T1, >= 9 by T2, >= 13 by T3)
    let mut exact_t1 = [0u32; 5]; // 0..=4
    let mut sum_t1 = 0i64;
    let mut sum_t2 = 0i64;
    let mut sum_t3 = 0i64;

    for _ in 0..trials {
        // Opening hand: 5 from the deck.
        let mut hand: Vec<Card> = (0..5).map(|_| deck[rng.below(deck.len())]).collect();
        // Mulligan: dump the most expensive non-live members, up to 3, redraw.
        hand.sort_by(|a, b| b.cost.cmp(&a.cost));
        let mut keep: Vec<Card> = hand.clone();
        // Keep the cheapest 2 (the curve floor), discard the rest up to 3.
        let keep_n = keep.len().saturating_sub(3);
        keep.truncate(keep_n.max(2));
        for _ in 0..(5 - keep.len()) {
            keep.push(deck[rng.below(deck.len())]);
        }
        hand = keep;

        // Turn 1: 4 active energy. Board persists from here on.
        let mut board: Vec<i32> = Vec::new();
        let t1 = develop(&mut board, &hand, 4);
        exact_t1[(t1.min(4)) as usize] += 1;
        sum_t1 += i64::from(t1);
        if t1 >= 4 {
            reach[0] += 1;
        }

        // Turn 2: draw 1, energy 5, board carried.
        let mut hand2 = hand.clone();
        hand2.push(deck[rng.below(deck.len())]);
        let t2 = develop(&mut board, &hand2, 5);
        sum_t2 += i64::from(t2);
        if t2 >= 9 {
            reach[1] += 1;
        }

        // Turn 3: draw 1, energy 6, board carried.
        let mut hand3 = hand2.clone();
        hand3.push(deck[rng.below(deck.len())]);
        let t3 = develop(&mut board, &hand3, 6);
        sum_t3 += i64::from(t3);
        if t3 >= 13 {
            reach[2] += 1;
        }
    }

    let n = trials as f64;
    println!();
    println!("  P(reach >= 4  by T1) = {:.3}   guide band: 4", reach[0] as f64 / n);
    println!("  P(reach >= 9  by T2) = {:.3}   guide band: 9", reach[1] as f64 / n);
    println!("  P(reach >= 13 by T3) = {:.3}   guide band: 13", reach[2] as f64 / n);
    println!();
    println!("  mean cost  T1 {:.2}   T2 {:.2}   T3 {:.2}", sum_t1 as f64 / n, sum_t2 as f64 / n, sum_t3 as f64 / n);
    println!("  T1 distribution: 0:{:.3} 1:{:.3} 2:{:.3} 3:{:.3} 4+:{:.3}",
        exact_t1[0] as f64 / n, exact_t1[1] as f64 / n, exact_t1[2] as f64 / n,
        exact_t1[3] as f64 / n, exact_t1[4] as f64 / n);
    println!();
    println!("  arena measures, v7 vs v6: T2 3.8  T3 7.6  T4 11.2");
}
