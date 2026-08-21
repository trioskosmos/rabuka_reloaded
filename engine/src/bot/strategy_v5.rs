//! Strategy bot v5 — v4's relentless placement execution + comparison awareness.
//!
//! Doctrine: a placement requires WINNING the comparison (8.4.6), so among
//! live portfolios that pass, v5 maximizes TOTAL SCORE subject to a
//! stance-dependent binomial pass-probability floor (yell hits are
//! Binomial(blades, density), not a mean). Concede clearly-lost contested
//! checks (温存), steal free wins vs an empty opponent zone (8.4.3.2),
//! contest at opponent match point (S4 / 8.4.7.1). Everything else is
//! inherited from v4: hearts-based development, life-acquisition digging,
//! no-op/hand-reserve breakers.

use crate::bot::strategy_v4::{alloc, flip_stats, hand_lives, heart_pool};
use crate::card::CardDatabase;
use crate::game_setup::{self, Action};
use crate::game_state::{GameState, Phase};

/// P(Bin(n, p) >= k), exact for small n (blade counts stay ≤ ~20).
fn binom_ge(n: i32, k: i32, p: f64) -> f64 {
    if k <= 0 {
        return 1.0;
    }
    if n <= 0 || k > n {
        return 0.0;
    }
    if p >= 1.0 {
        return 1.0;
    }
    if p <= 0.0 {
        return 0.0;
    }
    let q = 1.0 - p;
    // term_i = C(n,i) p^i q^(n-i)
    let mut term = q.powi(n);
    let mut prob = 0.0f64;
    for i in 0..=n {
        if i >= k {
            prob += term;
        }
        if i == n || term < 1e-12 {
            break;
        }
        term *= (n - i) as f64 / (i + 1) as f64 * (p / q);
    }
    prob.clamp(0.0, 1.0)
}

fn player_ref(gs: &GameState, me: u8) -> (&crate::player::Player, &crate::player::Player) {
    if me == 0 {
        (&gs.player1, &gs.player2)
    } else {
        (&gs.player2, &gs.player1)
    }
}

/// Opponent's achievable live score this turn, from their PUBLIC board
/// (S2: median hearts ≈ 2·score + 1..2). Own deck density proxies their
/// flips — fair information only.
pub fn estimate_opp_score(gs: &GameState, me: u8, db: &CardDatabase) -> i32 {
    let (_, opp) = player_ref(gs, me);
    let mut pool = 0i32;
    for &cid in opp.stage.stage.iter() {
        if cid < 0 {
            continue;
        }
        if let Some(c) = db.get_card(cid) {
            if let Some(bh) = &c.base_heart {
                pool += bh.hearts.values_sum() as i32;
            }
        }
    }
    let (_, density) = flip_stats(gs, me, db);
    let mut blades = 0i32;
    for &cid in opp.stage.stage.iter() {
        if cid < 0 {
            continue;
        }
        let waiting = gs.mods.get_orientation_modifier(cid) == Some("wait");
        if !waiting {
            if let Some(c) = db.get_card(cid) {
                blades += c.blade as i32;
            }
        }
    }
    pool += (blades as f64 * density).floor() as i32;
    if pool < 3 {
        0
    } else {
        ((pool - 1) / 2).min(12)
    }
}

/// Exhaustive portfolio search (≤3 lives).
///
/// Ranking doctrine (tree §1.3/§L4): every won check places exactly ONE card
/// no matter the margin, so the ONLY thing that matters is P(win the
/// comparison) — maximize TOTAL SCORE among portfolios whose all-or-nothing
/// pass probability (8.3.15→8.3.16) clears the stance floor.
///
/// Pass probability: yell hits are Binomial(blades, density), NOT the mean.
/// Only reliance on EXPECTED-HIT wildcards is stochastic; board BAll
/// wildcards are guaranteed. A portfolio sized exactly to the mean fails
/// ~half the time, which is how v4/v5 lost contested checks en masse.
pub fn best_portfolio(gs: &GameState, me: u8, db: &CardDatabase) -> Vec<usize> {
    best_portfolio_scored(gs, me, db).0
}

fn best_portfolio_scored(gs: &GameState, me: u8, db: &CardDatabase) -> (Vec<usize>, i32) {
    let (my, opp) = player_ref(gs, me);
    let pool = heart_pool(gs, me, db);
    let lives = hand_lives(my, db);
    let max_slots =
        (3i32 - i32::from(my.live_card_set_limit_reduction)).max(0) as usize;
    if lives.is_empty() || max_slots == 0 {
        return (Vec::new(), 0);
    }
    let n = lives.len().min(8);

    let my_succ = my.success_live_card_zone.cards.len() as i32;
    let opp_succ = opp.success_live_card_zone.cards.len() as i32;
    // Stance floors (tree L2/L3). Calibrated against reality: a portfolio
    // sized to the MEAN hit count sits at P(pass)≈0.5–0.7 by construction,
    // so demanding 0.75+ folds nearly every contested turn (measured:
    // 76% empty confirms, games stretching past T15).
    //  - opponent at 千秋楽 (2 successes): must CONTEST — folding loses
    //    outright (8.4.7.1), so accept coin-flip-ish portfolios;
    //  - I am at 2: one win ends the game — demand somewhat higher
    //    reliability before spending the closeout attempt;
    //  - otherwise: moderate reliability, tempo beats hoarding.
    let floor = if opp_succ >= 2 {
        0.40
    } else if my_succ >= 2 {
        0.65
    } else {
        0.55
    };

    let (blades, density) = flip_stats(gs, me, db);
    let expected_hits_start = pool[10];

    let rank = |score: i32, cnt: usize| -> (i32, std::cmp::Reverse<usize>) {
        (score, std::cmp::Reverse(cnt))
    };
    let mut best: Option<((i32, std::cmp::Reverse<usize>), Vec<usize>)> = None;
    for mask in 1..(1u32 << n) {
        let cnt = mask.count_ones() as usize;
        if cnt > max_slots {
            continue;
        }
        let mut p = pool;
        let mut score = 0i32;
        let mut idxs = Vec::with_capacity(cnt);
        let mut ok = true;
        for bit in 0..n {
            if mask & (1 << bit) != 0 {
                let (hi, cid, ref need) = lives[bit];
                match alloc(&p, need) {
                    Some(next) => {
                        p = next;
                        score += db.get_card(cid).and_then(|c| c.score).unwrap_or(0) as i32;
                        idxs.push(hi);
                    }
                    None => {
                        ok = false;
                        break;
                    }
                }
            }
        }
        if !ok {
            continue;
        }
        // Stochastic reliance = expected-hit wildcards consumed by this
        // portfolio (BAll consumption is deterministic board material).
        let relied = expected_hits_start - p[10];
        let p_pass = binom_ge(blades, relied.max(0), density);
        if p_pass < floor {
            continue;
        }
        let r = rank(score, cnt);
        if best.as_ref().map_or(true, |(br, _)| r > *br) {
            best = Some((r, idxs));
        }
    }
    let (idxs, score) = best.map(|(r, idxs)| (idxs, r.0)).unwrap_or_default();
    (idxs, score)
}

pub fn choose_action_v5(gs: &GameState, actions: &[Action], me: u8) -> Action {
    crate::bot::strategy_v4::choose_action_v4(gs, actions, me)
}

pub fn choose_live_set_v5(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    let me = if gs.active_player().id == gs.player1.id { 0u8 } else { 1u8 };
    let (my, opp) = player_ref(gs, me);
    let my_succ = my.success_live_card_zone.cards.len() as i32;
    let opp_succ = opp.success_live_card_zone.cards.len() as i32;
    let mut desired = best_portfolio(gs, me, db);
    let my_score: i32 = desired
        .iter()
        .filter_map(|&hi| my.hand.cards.get(hi).copied())
        .filter_map(|cid| db.get_card(cid))
        .map(|c| c.score.unwrap_or(0) as i32)
        .sum();

    // L3 concede (温存): a clearly losing contested comparison only donates a
    // life to the opponent's ライブ成功時 triggers — hold ammo instead. Ties
    // place for both below 2 successes (8.4.6.2), so only fold when we are
    // ≥2 score bands under their ceiling and NEITHER side is at match point.
    if !desired.is_empty() && my_succ < 2 && opp_succ < 2 && my_score + 2 <= estimate_opp_score(gs, me, db) {
        desired.clear();
    }

    // COMMITMENT RULE for everything below: whatever we decide to set joins
    // `desired`, so once selected it stays selected and emit() confirms.
    // SelectLiveCard is a TOGGLE in the engine — a stateless
    // select/deselect cycle here used to spin forever inside LiveCardSet
    // (the great 92%-draw stall).
    if desired.is_empty() {
        // Free win (8.4.3.2): as SECOND attacker with the opponent's zone
        // still empty, ANY sole passer places regardless of score — set the
        // cheapest deterministic life even if it scores 0.
        if gs.current_phase == Phase::LiveCardSetSecondAttacker
            && opp.live_card_zone.cards.is_empty()
        {
            if let Some(hi) = cheapest_deterministic_life(gs, me, db) {
                desired.push(hi);
                return emit(gs, actions, &desired);
            }
        }

        // Gamble fallback: when NOTHING clears its floor, bet one near-miss
        // life anyway. A failed check costs one life card (recycled via
        // refresh); passing unopposed places for free. Stalling wins nothing,
        // and at opponent match point folding loses outright (S4).
        let deficit_cap = if opp_succ >= 2 { 5 } else { 3 };
        if let Some((deficit, hi)) = nearest_miss_life(gs, me, db) {
            if deficit <= deficit_cap {
                desired.push(hi);
            }
        }
    }

    emit(gs, actions, &desired)
}

fn cheapest_deterministic_life(gs: &GameState, me: u8, db: &CardDatabase) -> Option<usize> {
    let (my, _) = player_ref(gs, me);
    let pool = heart_pool(gs, me, db);
    hand_lives(my, db)
        .into_iter()
        .filter(|(_, _, need)| alloc(&pool, need).is_some())
        .min_by_key(|(hi, cid, _)| {
            (
                db.get_card(*cid).and_then(|c| c.score).unwrap_or(0),
                *hi,
            )
        })
        .map(|(hi, _, _)| hi)
}

fn nearest_miss_life(gs: &GameState, me: u8, db: &CardDatabase) -> Option<(i32, usize)> {
    let (my, _) = player_ref(gs, me);
    let pool = heart_pool(gs, me, db);
    let mut best: Option<(i32, usize)> = None;
    for &(hi, _cid, ref need) in &hand_lives(my, db) {
        if alloc(&pool, need).is_some() {
            continue; // deterministic ones were already handled above
        }
        // deficit vs optimistic pool
        let mut p = pool;
        let mut deficit = 0i32;
        for c in 1..=6 {
            let d = need[c] - p[c];
            if d > 0 {
                deficit += d - (p[10] + p[7]).min(d);
            }
        }
        let grey_short = need[0]
            - ([p[0], p[1], p[2], p[3], p[4], p[5], p[6]].iter().sum::<i32>() + p[7] + p[10])
                .min(need[0]);
        deficit += grey_short.max(0);
        if best.map_or(true, |(bd, _)| deficit < bd) {
            best = Some((deficit, hi));
        }
    }
    best
}

fn emit(gs: &GameState, actions: &[Action], desired: &[usize]) -> Action {
    let selected: Vec<usize> = gs
        .live_card_selected_indices
        .iter()
        .map(|&i| i as usize)
        .collect();
    let find = |hi: usize, want: bool| -> Option<Action> {
        actions
            .iter()
            .find(|a| {
                a.action_type == game_setup::ActionType::SelectLiveCard
                    && a.selected == Some(want)
                    && a.parameters.as_ref().and_then(|p| p.card_index) == Some(hi)
            })
            .cloned()
    };
    for &hi in desired {
        if !selected.contains(&hi) {
            if let Some(a) = find(hi, false) {
                return a;
            }
        }
    }
    for &hi in &selected {
        if !desired.contains(&hi) {
            if let Some(a) = find(hi, true) {
                return a;
            }
        }
    }
    actions
        .iter()
        .find(|a| a.action_type == game_setup::ActionType::ConfirmLiveCardSet)
        .or_else(|| actions.first())
        .cloned()
        .expect("live set actions non-empty")
}

pub fn choose_mulligan_v5(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    crate::bot::strategy_v4::choose_mulligan_v4(gs, actions, db)
}
