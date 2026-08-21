//! Strategy bot v5 — v4's relentless placement execution + comparison awareness.
//!
//! The one doctrinal addition: a placement requires WINNING the comparison
//! (8.4.6), so among live portfolios that deterministically pass, v5
//! maximizes TOTAL SCORE (exhaustive subset search, ≤3 lives) instead of
//! cheapest-first. Everything else is inherited from v4: hearts-based
//! development, life-acquisition digging, no-op/hand-reserve breakers.

use crate::bot::strategy_v4::{alloc, heart_pool, hand_lives};
use crate::card::{CardDatabase, CardType};
use crate::game_setup::{self, Action};
use crate::game_state::GameState;

/// Exhaustive portfolio search (≤3 lives).
/// Ranking doctrine: **count of lives first, total score second.**
/// Every won check places exactly ONE card no matter the margin, so three
/// 0-point wins at t2/t4/t5 beat a single 15-point turn-12 blowout. Score
/// only matters inside a contested check, so it breaks ties between equal
/// counts.
pub fn best_portfolio(gs: &GameState, me: u8, db: &CardDatabase) -> Vec<usize> {
    let my = if me == 0 { &gs.player1 } else { &gs.player2 };
    let pool = heart_pool(gs, me, db);
    let lives = hand_lives(my, db);    let max_slots =
        (3i32 - i32::from(my.live_card_set_limit_reduction)).max(0) as usize;
    if lives.is_empty() || max_slots == 0 {
        return Vec::new();
    }
    let n = lives.len().min(8);
    // Zerg doctrine: MINIMUM lives that still all pass — fewer failure
    // points, less ammo spent, fastest repeated small wins.
    let rank = |score: i32, cnt: usize| -> (std::cmp::Reverse<usize>, i32) {
        (std::cmp::Reverse(cnt), score)
    };
    let mut best: Option<((std::cmp::Reverse<usize>, i32), Vec<usize>)> = None;
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
        if ok {
            let r = rank(score, cnt);
            if best.as_ref().map_or(true, |(br, _)| r > *br) {
                best = Some((r, idxs));
            }
        }
    }
    best.map(|(_, idxs)| idxs).unwrap_or_default()
}

pub fn choose_action_v5(gs: &GameState, actions: &[Action], me: u8) -> Action {
    crate::bot::strategy_v4::choose_action_v4(gs, actions, me)
}

pub fn choose_live_set_v5(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    let me = if gs.active_player().id == gs.player1.id { 0u8 } else { 1u8 };
    let mut desired = best_portfolio(gs, me, db);

    // Win-fast fallback: when NOTHING passes deterministically, gamble one
    // near-miss life anyway. A failed check costs one life card (recycled
    // via refresh); passing unopposed places for free. Stalling wins nothing.
    //
    // COMMITMENT RULE: the gamble target joins `desired`, so once selected it
    // stays selected and emit() confirms instead of deselecting. SelectLiveCard
    // is a TOGGLE in the engine — a stateless select/deselect cycle here used
    // to spin forever inside LiveCardSet (the great 92%-draw stall).
    if desired.is_empty() {
        let my = if me == 0 { &gs.player1 } else { &gs.player2 };
        let mut best: Option<(i32, usize)> = None;
        for &(hi, _cid, ref need) in &hand_lives(my, db) {
            let probe = heart_pool(gs, me, db);
            if alloc(&probe, need).is_some() {
                continue; // deterministic ones were already handled above
            }
            // deficit vs optimistic pool
            let mut p = probe;
            let mut deficit = 0i32;
            for c in 1..=6 {
                let d = need[c] - p[c];
                if d > 0 {
                    deficit += d - (p[10] + p[7]).min(d);
                }
            }
            let grey_short = need[0]
                - ([p[0], p[1], p[2], p[3], p[4], p[5], p[6]].iter().sum::<i32>()
                    + p[7]
                    + p[10])
                .min(need[0]);
            deficit += grey_short.max(0);
            if best.map_or(true, |(bd, _)| deficit < bd) {
                best = Some((deficit, hi));
            }
        }
        if let Some((deficit, hi)) = best {
            if deficit <= 3 {
                desired.push(hi);
            }
        }
    }

    emit(gs, actions, &desired)
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
