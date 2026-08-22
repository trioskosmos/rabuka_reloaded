//! Strategy bot v5  Ev4's relentless placement execution + comparison awareness.
//!
//! Doctrine: a placement requires WINNING the comparison (8.4.6), so among
//! live portfolios that pass, v5 maximizes TOTAL SCORE subject to a
//! stance-dependent binomial pass-probability floor (yell hits are
//! Binomial(blades, density), not a mean). Concede clearly-lost contested
//! checks (温孁E, steal free wins vs an empty opponent zone (8.4.3.2),
//! contest at opponent match point (S4 / 8.4.7.1). Everything else is
//! inherited from v4: hearts-based development, life-acquisition digging,
//! no-op/hand-reserve breakers.

use crate::bot::strategy_v4::{alloc, flip_stats, hand_lives, heart_pool};
use crate::card::{CardDatabase, CardType};
use crate::game_setup::{self, Action};
use crate::game_state::{GameState, Phase};

/// P(Bin(n, p) >= k), exact for small n (blade counts stay ≤ ~20).
pub(crate) fn binom_ge(n: i32, k: i32, p: f64) -> f64 {
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

pub(crate) fn player_ref(gs: &GameState, me: u8) -> (&crate::player::Player, &crate::player::Player) {
    if me == 0 {
        (&gs.player1, &gs.player2)
    } else {
        (&gs.player2, &gs.player1)
    }
}

/// Opponent's achievable live score this turn, from their PUBLIC board
/// (S2: median hearts ≁E2·score + 1..2). Own deck density proxies their
/// flips  Efair information only.
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
/// comparison)  Emaximize TOTAL SCORE among portfolios whose all-or-nothing
/// pass probability (8.3.15ↁE.3.16) clears the stance floor.
///
/// Pass probability: yell hits are Binomial(blades, density), NOT the mean.
/// Only reliance on EXPECTED-HIT wildcards is stochastic; board BAll
/// wildcards are guaranteed. A portfolio sized exactly to the mean fails
/// ~half the time, which is how v4/v5 lost contested checks en masse.
pub fn best_portfolio(gs: &GameState, me: u8, db: &CardDatabase) -> Vec<usize> {
    best_portfolio_scored(gs, me, db).0
}

/// Honest opponent term  ERULE FACTS only, no invented probabilities.
/// E = their public-board ceiling (hearts+blades ↁEscore band). The single
/// behavioral use: if I sit at 2 successes, a tie places NOTHING for me
/// (8.4.7.1), so a portfolio that cannot strictly beat their ceiling can
/// only win when their check fails outright  Ediscount it. Everything else
/// ranks by expected placed-score, chess-style: play my best, no despair.
pub(crate) fn best_portfolio_scored(gs: &GameState, me: u8, db: &CardDatabase) -> (Vec<usize>, i32, f64) {
    let (my, opp) = player_ref(gs, me);
    let pool = heart_pool(gs, me, db);
    let lives = hand_lives(my, db);
    let max_slots =
        (3i32 - i32::from(my.live_card_set_limit_reduction)).max(0) as usize;
    if lives.is_empty() || max_slots == 0 {
        return (Vec::new(), 0, 0.0);
    }
    let n = lives.len().min(8);

    let my_succ = my.success_live_card_zone.cards.len() as i32;
    let opp_succ = opp.success_live_card_zone.cards.len() as i32;
    // Stance floors (tree L2/L3). Calibrated against reality: a portfolio
    // sized to the MEAN hit count sits at P(pass)≁E.5 E.7 by construction,
    // so demanding 0.75+ folds nearly every contested turn (measured:
    // 76% empty confirms, games stretching past T15).
    //  - opponent ahead (2 successes): must CONTEST — folding loses
    //    outright (8.4.7.1), so accept coin-flip-ish portfolios;
    //  - I am at 2: one win ends the game  Edemand somewhat higher
    //    reliability before spending the closeout attempt;
    //  - otherwise: moderate reliability, tempo beats hoarding.
    let floor = if opp_succ >= 2 {
        0.35
    } else if my_succ >= 2 {
        0.60
    } else {
        0.45
    };

    let e_opp = estimate_opp_score(gs, me, db);

    let (blades, density) = flip_stats(gs, me, db);
    // Two-pool pass model (per-color flip reality):
    //   Pool F (mean flips, per printed color) must pass at all;
    //   Pool B (board hearts only) passing ⇁Edeterministic;
    //   otherwise shortfall units must come from yell flips ⇁Ebinomial.
    let pool_board = crate::bot::strategy_v4::heart_pool_inner(gs, me, db, 0.0);
    let board_supply: i32 = (0..=7).chain(std::iter::once(10)).map(|i| pool_board[i]).sum();

    let mut best: Option<(f64, usize, i32, Vec<usize>)> = None;
    for mask in 1..(1u32 << n) {
        let cnt = mask.count_ones() as usize;
        if cnt > max_slots {
            continue;
        }
        let mut p = pool;
        let mut score = 0i32;
        let mut total_req = 0i32;
        let mut idxs = Vec::with_capacity(cnt);
        let mut ok = true;
        for bit in 0..n {
            if mask & (1 << bit) != 0 {
                let (hi, cid, ref need) = lives[bit];
                match alloc(&p, need) {
                    Some(next) => {
                        p = next;
                        score += db.get_card(cid).and_then(|c| c.score).unwrap_or(0) as i32;
                        total_req += (0..=7).chain(std::iter::once(10)).map(|i| need[i]).sum::<i32>();
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
        let shortfall = (total_req - board_supply.min(total_req)).max(0);
        let p_pass = if shortfall == 0 {
            1.0
        } else {
            binom_ge(blades, shortfall, density)
        };
        if p_pass < floor {
            continue;
        }
        let tie_worthless = score <= e_opp && my_succ >= 2;
        let ev =
            p_pass * (score as f64) * if tie_worthless { 0.5 } else { 1.0 };
        let better = best.as_ref().map_or(true, |(be, bc, _, _)| {
            ev > *be + f64::EPSILON || ((ev - *be).abs() <= f64::EPSILON && cnt < *bc)
        });
        if better {
            best = Some((ev, cnt, score, idxs));
        }
    }
    best.map(|(ev, _cnt, score, idxs)| (idxs, score, ev))
        .unwrap_or_default()
}

pub fn choose_action_v5(gs: &GameState, actions: &[Action], me: u8) -> Action {
    crate::bot::strategy_v4::choose_action_v4(gs, actions, me)
}

/// Main phase v6  EEXPERIMENT RESULT (2026-08-22): pricing actions by
/// next-check equity delta LOST to plain v4 heuristics (~46% head-to-head,
/// games stretching to ~10.4 turns). Root cause hypothesis: the projected
/// portfolio score is dominated by hand-luck noise during Main phase (floors
/// filter most candidates early game), so its delta drowns the reliable
/// passable/ammo signals and delays board growth when `behind` inflates its
/// weight. Kept as documentation; delegates to the proven v4 policy.
pub fn choose_action_v6(gs: &GameState, actions: &[Action], me: u8) -> Action {
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

    // COMMITMENT RULE for everything below: whatever we decide to set joins
    // `desired`, so once selected it stays selected and emit() confirms.
    // SelectLiveCard is a TOGGLE in the engine  Ea stateless
    // select/deselect cycle here used to spin forever inside LiveCardSet
    // (the great 92%-draw stall).
    if desired.is_empty() {
        // Free win (8.4.3.2): as SECOND attacker with the opponent's zone
        // still empty, ANY sole passer places regardless of score  Eset the
        // cheapest deterministic life even if it scores 0.
        if gs.current_phase == Phase::LiveCardSetSecondAttacker
            && opp.live_card_zone.cards.is_empty()
        {
            if let Some(hi) = cheapest_deterministic_life(gs, me, db) {
                desired.push(hi);
                return emit(gs, actions, &desired);
            }
        }

        // Gamble fallback: bet ONE near-miss life, chosen by estimated pass
        // probability (per-color binomial over our own deck), not paper
        // deficit. Measured: the old ≤4-deficit cap failed 97% of the time  E        // expected flips aren't wildcards. At opponent match point folding
        // loses outright (S4), so accept longer odds there.
        let p_floor = if opp_succ >= 2 { 0.10 } else { 0.25 };
        if let Some((p, _deficit, hi)) = nearest_miss_life(gs, me, db) {
            if p >= p_floor {
                desired.push(hi);
            }
        }

        // Hand filtering (8.2 + 8.3.4): with no live worth setting, fill the
        // slots with dead NON-live cards instead of confirming an empty zone.
        // They are discarded at performance start BEFORE any check (so they
        // can never fail it) and each placed card draws a replacement  E        // trading up to 3 dead hand cards for 3 fresh deck draws per turn.
        // This is the cure for the measured starvation stretch (14 turns at
        // zero hand lives while the board outgrew the game). Own decklist is
        // fair information: skip the churn once no lives remain to find.
        let deck_lives = my
            .main_deck
            .cards
            .iter()
            .filter(|&&cid| {
                db.get_card(cid).map_or(false, |c| c.card_type == CardType::Live)
            })
            .count();
        let max_slots =
            (3i32 - i32::from(my.live_card_set_limit_reduction)).max(0) as usize;
        if desired.len() < max_slots && deck_lives > 0 {
            let mut junk: Vec<(usize, u8)> = my
                .hand
                .cards
                .iter()
                .enumerate()
                .filter(|&(i, &cid)| {
                    !desired.contains(&i)
                        && db
                            .get_card(cid)
                            .map_or(false, |c| c.card_type != CardType::Live)
                })
                .map(|(i, &cid)| {
                    (
                        i,
                        db.get_card(cid).and_then(|c| c.cost).unwrap_or(0),
                    )
                })
                .collect();
            junk.sort_by_key(|&(_, cost)| std::cmp::Reverse(cost));
            for &(hi, _) in &junk {
                if desired.len() >= max_slots {
                    break;
                }
                desired.push(hi);
            }
        }
        if std::env::var("V5_TRACE").is_ok() {
            let n_lives = desired
                .iter()
                .filter(|&&hi| {
                    my.hand.cards.get(hi).copied().map_or(false, |cid| {
                        db.get_card(cid).map_or(false, |c| c.card_type == CardType::Live)
                    })
                })
                .count();
            eprintln!(
                "V5L t{} me{} EMPTY->{} lives={} junk={} (my_succ={} opp_succ={})",
                gs.turn_number,
                me,
                if desired.is_empty() { "FOLD" } else { "GAMBLE" },
                n_lives,
                desired.len() - n_lives,
                my_succ,
                opp_succ
            );
        }
    } else if std::env::var("V5_TRACE").is_ok() {
        eprintln!(
            "V5L t{} me{} SET n={} score={}",
            gs.turn_number,
            me,
            desired.len(),
            my_score
        );
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

/// Most-probable near-miss life: per-color deficits against BOARD-ONLY
/// hearts, each covered by a per-color binomial over our own deck's
/// blade-heart distribution. Measured pathology of the old paper-deficit
/// gamble: 97% failure with ~11.7 unmet hearts  Eit counted expected flips
/// as any-color wildcards, so "≤4 short on paper" was routinely 10+ short
/// in the only colors that mattered.
/// Returns (estimated pass probability, paper deficit, hand index).
pub(crate) fn nearest_miss_life(gs: &GameState, me: u8, db: &CardDatabase) -> Option<(f64, i32, usize)> {
    let (my, _) = player_ref(gs, me);
    let board = crate::bot::strategy_v4::heart_pool_inner(gs, me, db, 0.0);
    let dens = crate::bot::strategy_v4::blade_unit_densities(gs, me, db);
    let (blades, _) = flip_stats(gs, me, db);
    let dens_any: f64 = (0..=7).map(|i| dens[i]).sum();
    let mut best: Option<(f64, i32, usize)> = None;
    for &(hi, _cid, ref need) in &hand_lives(my, db) {
        if alloc(&board, need).is_some() {
            continue; // deterministic  Ehandled by the portfolio path
        }
        let mut p = 1.0f64;
        let mut deficit = 0i32;
        for c in 1..=6 {
            let d = need[c] - board[c];
            if d > 0 {
                deficit += d;
                p *= binom_ge(blades, d, dens[c]);
            }
        }
        // Grey bucket: colorless + leftover specifics + wildcards.
        let grey_have = board[0]
            + (1..=6)
                .map(|c| board[c].max(0))
                .sum::<i32>()
            + board[7]
            + board[10];
        let specific_used: i32 = (1..=6).map(|c| need[c].min(board[c])).sum::<i32>();
        let grey_short =
            need[0] - (grey_have - specific_used).min(need[0]);
        if grey_short > 0 {
            deficit += grey_short;
            p *= binom_ge(blades, grey_short, dens_any);
        }
        if deficit == 0 {
            continue;
        }
        if best.as_ref().map_or(true, |(bp, bd, _)| p > *bp + f64::EPSILON || ((p - *bp).abs() <= f64::EPSILON && deficit < *bd)) {
            best = Some((p, deficit, hi));
        }
    }
    best
}

pub(crate) fn emit(gs: &GameState, actions: &[Action], desired: &[usize]) -> Action {
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


