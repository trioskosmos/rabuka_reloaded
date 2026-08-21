//! Experimental strategy bot v2 — pure heuristics, no rollouts.
//!
//! Extends v1 (`strategy.rs`) with the two biggest missing pieces:
//!
//! 1. **Live-set policy with real yell mathematics** (S2/S3/S4): estimate the
//!    opponent's max score from their public hearts+blades, then choose the
//!    *minimal* set of lives that outscoring requires, verifying via Monte
//!    Carlo over our own known remaining deck that the required hearts are
//!    actually flippable. Sets zero lives when the check is unwinnable (save
//!    ammo), except when the opponent is at match point (千秋楽) — then
//!    contest with the best gamble available. A tie also works to deny a
//!    2-card opponent (rule 8.4.7.1), lowering the bar in that case.
//!
//! 2. **Mulligan policy** (S7): keep the early curve, redraw dead high-cost
//!    members and excess lives.
//!
//! Fairness: opponent info is public-only (stage, success count). Own deck
//! composition is fair game — a real player knows their own list.

use crate::card::{CardDatabase, CardType, HeartColor, HeartMap};
use crate::game_setup::{self, Action};
use crate::game_state::GameState;
use crate::player::Player;

/// Decision thresholds for the live-set policy.
pub struct V2Policy {
    /// Required pass probability to set lives in a normal turn.
    pub pass_threshold: f64,
    /// Lower bar when the opponent is at 2 success cards (must contest).
    pub urgent_threshold: f64,
    /// Lower bar when WE are at 2 success cards (winning ends the game).
    pub closing_threshold: f64,
    /// Monte Carlo trials per candidate subset.
    pub mc_trials: u32,
}

impl Default for V2Policy {
    fn default() -> Self {
        Self {
            pass_threshold: 0.60,
            urgent_threshold: 0.30,
            closing_threshold: 0.45,
            mc_trials: 48,
        }
    }
}

/// Per-color heart accumulator. Index space matches `HeartColor` variant
/// order (Heart00..Heart06, BAll, Draw, Score, All).
type Acc = [i32; 11];

fn hc_index(c: HeartColor) -> usize {
    match c {
        HeartColor::Heart00 => 0,
        HeartColor::Heart01 => 1,
        HeartColor::Heart02 => 2,
        HeartColor::Heart03 => 3,
        HeartColor::Heart04 => 4,
        HeartColor::Heart05 => 5,
        HeartColor::Heart06 => 6,
        HeartColor::BAll => 7,
        HeartColor::Draw => 8,
        HeartColor::Score => 9,
        HeartColor::All => 10,
    }
}

fn acc_add(acc: &mut Acc, hearts: &HeartMap) {
    for (c, v) in hearts.iter() {
        acc[hc_index(*c)] += *v as i32;
    }
}

/// Sum of stage members' base hearts (all members; wait state only affects
/// blades per Q133).
fn stage_hearts(p: &Player, db: &CardDatabase) -> Acc {
    let mut acc = [0i32; 11];
    for &cid in p.stage.stage.iter() {
        if cid < 0 {
            continue;
        }
        if let Some(card) = db.get_card(cid) {
            if let Some(bh) = &card.base_heart {
                acc_add(&mut acc, &bh.hearts);
            }
        }
    }
    acc
}

/// Active-member blade count for yell flips (Q133: waited members don't yell).
fn yell_flips(gs: &GameState, p: &Player, db: &CardDatabase) -> usize {
    p.stage
        .total_blades(
            db,
            &gs.mods.blade_modifiers,
            &gs.mods.orientation_modifiers,
            false,
        ) as usize
}

/// Max live score achievable from `h` total hearts (score bands need
/// ~2N+1..2N+2 hearts per N points — source: ミヤ guide).
fn max_score_from(h: i32) -> i32 {
    if h < 3 {
        0
    } else {
        (h - 1) / 2
    }
}

/// Deterministic LCG for the Monte Carlo sampler.
struct McRng(u64);

impl McRng {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
}

/// Rule-faithful satisfaction check.
/// - Specific colors (Heart01-06) must be met exactly; `All` supply and
///   `BAll` blade-hearts are wildcards for specific colors.
/// - `Heart00` requirement is the "any color" total bucket (rule 2.1.1.2 /
///   2.11.3): filled by colorless hearts and any leftover hearts.
/// - Draw/Score icons are not heart requirements.
fn requirements_met(have: &Acc, need: &Acc) -> bool {
    let mut wildcard = have[10] + have[7]; // All + BAll
    let mut specific_surplus = 0i32;
    for c in 1..=6 {
        let deficit = need[c] - have[c];
        if deficit > 0 {
            wildcard -= deficit;
            if wildcard < 0 {
                return false;
            }
        } else {
            specific_surplus += -deficit;
        }
    }
    // Heart00 bucket: colorless hearts, plus any leftover specific/wild hearts.
    let leftover = specific_surplus + wildcard.max(0) + have[0];
    leftover >= need[0]
}

/// Monte Carlo pass probability: sample `flips` cards from the known
/// remaining deck and check whether own hearts + flipped blade-hearts
/// satisfy the combined requirements of the candidate lives.
fn pass_probability(
    db: &CardDatabase,
    deck: &[i16],
    flips: usize,
    own: &Acc,
    need: &Acc,
    trials: u32,
    rng: &mut McRng,
) -> f64 {
    if flips == 0 || deck.is_empty() {
        return if requirements_met(own, need) { 1.0 } else { 0.0 };
    }
    // Pre-extract blade-heart maps for deck cards (most have none).
    let pool: Vec<Option<Acc>> = deck
        .iter()
        .map(|&cid| {
            db.get_card(cid).and_then(|c| c.blade_heart.as_ref()).map(
                |bh| {
                    let mut a = [0i32; 11];
                    acc_add(&mut a, &bh.hearts);
                    a
                },
            )
        })
        .collect();
    let n = pool.len();
    let flips = flips.min(n);
    let mut hits = 0u32;
    let mut idx: Vec<usize> = (0..n).collect();
    for _ in 0..trials {
        // Partial Fisher-Yates: draw `flips` distinct cards.
        let mut flipped = [0i32; 11];
        for i in 0..flips {
            let j = (rng.next() % (n - i) as u64) as usize + i;
            idx.swap(i, j);
            if let Some(Some(a)) = pool.get(idx[i]) {
                for k in 0..11 {
                    flipped[k] += a[k];
                }
            }
        }
        let mut total = [0i32; 11];
        for k in 0..11 {
            total[k] = own[k] + flipped[k];
        }
        if requirements_met(&total, need) {
            hits += 1;
        }
    }
    hits as f64 / trials as f64
}

struct LiveCandidate {
    hand_index: usize,
    card_id: i16,
    score: i32,
    need: Acc,
}

fn hand_live_candidates(p: &Player, db: &CardDatabase) -> Vec<LiveCandidate> {
    let mut out = Vec::new();
    for (hand_index, &cid) in p.hand.cards.iter().enumerate() {
        if let Some(card) = db.get_card(cid) {
            if !matches!(card.card_type, CardType::Live) {
                continue;
            }
            let mut need = [0i32; 11];
            if let Some(nh) = &card.need_heart {
                acc_add(&mut need, &nh.hearts);
            }
            out.push(LiveCandidate {
                hand_index,
                card_id: cid,
                score: card.score.unwrap_or(0) as i32,
                need,
            });
        }
    }
    out
}

/// V2 live-set policy. See module docs. One action per call, stable across
/// calls (the plan is recomputed from the current selection state).
pub fn choose_live_set_action_v2(
    gs: &GameState,
    actions: &[Action],
    db: &CardDatabase,
    policy: &V2Policy,
) -> Action {
    let (me, opp) = if gs.active_player().id == gs.player1.id {
        (&gs.player1, &gs.player2)
    } else {
        (&gs.player2, &gs.player1)
    };
    let my_success = me.success_live_card_zone.cards.len();
    let opp_success = opp.success_live_card_zone.cards.len();

    let my_hearts = stage_hearts(me, db);
    let my_flips = yell_flips(gs, me, db);
    let opp_hearts_total = stage_hearts(opp, db).iter().take(7).sum::<i32>()
        + opp
            .stage
            .total_blades(
                db,
                &gs.mods.blade_modifiers,
                &gs.mods.orientation_modifiers,
                false,
            ) as i32;
    // Opponent best case: every flip yields a heart (we don't know their deck).
    let opp_max = max_score_from(opp_hearts_total);

    // Target score and probability bar by game state.
    let (target, threshold) = if opp_success >= 2 && my_success < 2 {
        // 千秋楽: a tie denies them (8.4.7.1) — contest at all costs.
        (opp_max, policy.urgent_threshold)
    } else if my_success >= 2 && opp_success < 2 {
        // We can end the game this turn — push.
        (opp_max + 1, policy.closing_threshold)
    } else {
        (opp_max + 1, policy.pass_threshold)
    };

    let candidates = hand_live_candidates(me, db);
    let selected: Vec<usize> = gs
        .live_card_selected_indices
        .iter()
        .map(|&i| i as usize)
        .collect();

    // Plan: minimal subset of lives with total score >= target that passes
    // the yell Monte Carlo at the threshold.
    let mut desired: Vec<usize> = Vec::new();
    if !candidates.is_empty() && target > 0 {
        let mut rng = McRng(
            (gs.turn_number as u64)
                .wrapping_mul(0x9E3779B97F4A7C15)
                .wrapping_add(my_flips as u64)
                | 1,
        );
        let mut best: Option<(usize, i32, Vec<usize>)> = None; // (count, -score, indices)
        let n = candidates.len().min(8); // brute-force cap
        for mask in 1..(1u32 << n) {
            let count = mask.count_ones() as usize;
            if count > 3 {
                continue;
            }
            let mut score_sum = 0i32;
            let mut need = [0i32; 11];
            let mut indices = Vec::with_capacity(count);
            for bit in 0..n {
                if mask & (1 << bit) != 0 {
                    let c = &candidates[bit];
                    score_sum += c.score;
                    for k in 0..11 {
                        need[k] += c.need[k];
                    }
                    indices.push(c.hand_index);
                }
            }
            if score_sum < target {
                continue;
            }
            let prob = pass_probability(
                db,
                &me.main_deck.cards,
                my_flips,
                &my_hearts,
                &need,
                policy.mc_trials,
                &mut rng,
            );
            if prob < threshold {
                continue;
            }
            // Prefer fewer cards, then higher score.
            let rank = (count, -score_sum);
            if best.as_ref().map_or(true, |(bc, bs, _)| rank < (*bc, *bs)) {
                best = Some((count, -score_sum, indices));
            }
        }
        if let Some((_, _, indices)) = best {
            desired = indices;
        } else if opp_success >= 2 && my_success < 2 {
            // Must contest but nothing meets the bar: gamble on the single
            // most likely life (tiebreak: higher score).
            let mut rng = McRng(0xBADA55);
            let mut best_gamble: Option<(u32, i16)> = None; // (prob_milli, hand_index)
            for c in &candidates {
                let p = pass_probability(
                    db,
                    &me.main_deck.cards,
                    my_flips,
                    &my_hearts,
                    &c.need,
                    policy.mc_trials,
                    &mut rng,
                );
                let milli = (p * 1000.0) as u32;
                if best_gamble.map_or(true, |(bp, _)| milli > bp) {
                    best_gamble = Some((milli, c.hand_index));
                }
            }
            if let Some((_, hi)) = best_gamble {
                desired = vec![hi];
            }
        }
    }

    // Emit one action: select the first desired-not-selected, else deselect
    // anything selected-but-not-desired, else confirm.
    let find_select = |hand_index: usize| -> Option<Action> {
        actions
            .iter()
            .find(|a| {
                a.action_type == game_setup::ActionType::SelectLiveCard
                    && a.selected == Some(false)
                    && a.parameters
                        .as_ref()
                        .and_then(|p| p.card_index)
                        == Some(hand_index)
            })
            .cloned()
    };
    for &hi in &desired {
        if !selected.contains(&hi) {
            if let Some(a) = find_select(hi) {
                return a;
            }
        }
    }
    for &hi in &selected {
        if !desired.contains(&hi) {
            if let Some(a) = actions.iter().find(|a| {
                a.action_type == game_setup::ActionType::SelectLiveCard
                    && a.selected == Some(true)
                    && a.parameters.as_ref().and_then(|p| p.card_index) == Some(hi)
            }) {
                return a.clone();
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

/// V2 mulligan policy (S7): keep the early curve (cost ≤4 member, cost-7,
/// cost-13..15, lives); redraw dead high-cost members (cost > 15, max 2) and
/// lives beyond the third.
pub fn choose_mulligan_action_v2(
    gs: &GameState,
    actions: &[Action],
    db: &CardDatabase,
) -> Action {
    let me = gs.active_player();
    let mut member_count = 0usize;
    let mut live_count = 0usize;
    let mut discard: Vec<usize> = Vec::new();
    for (hand_index, &cid) in me.hand.cards.iter().enumerate() {
        let Some(card) = db.get_card(cid) else {
            continue;
        };
        match card.card_type {
            CardType::Live => {
                live_count += 1;
                if live_count > 3 {
                    discard.push(hand_index);
                }
            }
            CardType::Member => {
                let cost = card.cost.unwrap_or(0);
                member_count += 1;
                if cost > 15 && discard.len() < 2 {
                    discard.push(hand_index);
                }
            }
            CardType::Energy => {}
        }
    }

    let selected: Vec<usize> = gs
        .mulligan_selected_indices
        .iter()
        .map(|&i| i as usize)
        .collect();
    for &hi in &discard {
        if !selected.contains(&hi) {
            if let Some(a) = actions.iter().find(|a| {
                a.action_type == game_setup::ActionType::SelectMulligan
                    && a.selected == Some(false)
                    && a.parameters.as_ref().and_then(|p| p.card_index) == Some(hi)
            }) {
                return a.clone();
            }
        }
    }
    for &hi in &selected {
        if !discard.contains(&hi) {
            if let Some(a) = actions.iter().find(|a| {
                a.action_type == game_setup::ActionType::SelectMulligan
                    && a.selected == Some(true)
                    && a.parameters.as_ref().and_then(|p| p.card_index) == Some(hi)
            }) {
                return a.clone();
            }
        }
    }
    actions
        .iter()
        .find(|a| {
            matches!(
                a.action_type,
                game_setup::ActionType::ConfirmMulligan | game_setup::ActionType::SkipMulligan
            )
        })
        .or_else(|| actions.first())
        .cloned()
        .expect("mulligan actions non-empty")
}

/// V2 main-phase choice: currently the v1 heuristic eval (the v2 gains come
/// from the live-set and mulligan policies).
pub fn choose_action_heuristic_v2(
    gs: &GameState,
    actions: &[Action],
    me: u8,
) -> Action {
    crate::bot::strategy::choose_action_heuristic(gs, actions, me)
}
