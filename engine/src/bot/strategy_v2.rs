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
    /// Monte Carlo trials per candidate subset.
    pub mc_trials: u32,
    /// Minimum pass probability to set lives on a normal turn (below this,
    /// hoard ammo — forfeiting is only marginally worse than a hopeless set).
    pub gamble_floor: f64,
    /// Lower bar when the opponent is at 2 success cards (must contest).
    pub urgent_gamble_floor: f64,
}

impl Default for V2Policy {
    fn default() -> Self {
        Self {
            mc_trials: 128,
            gamble_floor: 0.12,
            urgent_gamble_floor: 0.05,
        }
    }
}

impl Clone for V2Policy {
    fn clone(&self) -> Self {
        Self {
            mc_trials: self.mc_trials,
            gamble_floor: self.gamble_floor,
            urgent_gamble_floor: self.urgent_gamble_floor,
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

    let candidates = hand_live_candidates(me, db);
    let selected: Vec<usize> = gs
        .live_card_selected_indices
        .iter()
        .map(|&i| i as usize)
        .collect();

    // Plan: minimal subset of lives meeting a (target, threshold) rung of
    // the ladder; first rung that yields a feasible plan wins.
    let mut desired: Vec<usize> = Vec::new();
    if !candidates.is_empty() {
        let mut rng = McRng(
            (gs.turn_number as u64)
                .wrapping_mul(0x9E3779B97F4A7C15)
                .wrapping_add(my_flips as u64)
                | 1,
        );
        let n = candidates.len().min(8); // brute-force cap

        // Pre-compute per-subset data once. Effective score includes the
        // expected スコア+1 icons from yell flips (known deck composition).
        let deck_len = me.main_deck.cards.len().max(1);
        let score_icons_in_deck: i32 = me
            .main_deck
            .cards
            .iter()
            .filter_map(|&cid| db.get_card(cid))
            .map(|c| {
                c.blade_heart
                    .as_ref()
                    .map(|bh| bh.hearts.get(&HeartColor::Score).copied().unwrap_or(0) as i32)
                    .unwrap_or(0)
            })
            .sum();
        let expected_icons = my_flips as f64 * score_icons_in_deck as f64 / deck_len as f64;

        struct Subset {
            count: usize,
            effective_score: i32,
            need: Acc,
            indices: Vec<usize>,
            prob_milli: std::cell::Cell<u32>,
        }
        let mut subsets: Vec<Subset> = Vec::new();
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
            subsets.push(Subset {
                count,
                effective_score: score_sum + expected_icons as i32,
                need,
                indices,
                prob_milli: std::cell::Cell::new(u32::MAX), // not yet computed
            });
        }

        // Compute pass probabilities once for all subsets.
        for s in &subsets {
            let prob = pass_probability(
                db,
                &me.main_deck.cards,
                my_flips,
                &my_hearts,
                &s.need,
                policy.mc_trials,
                &mut rng,
            );
            s.prob_milli.set((prob * 1000.0) as u32);
        }

        // Policy: succeed-at-all-costs ranking, modulated by STANCE — the
        // qualitative endgame logic derived from the placement rules:
        // - Ahead (my > opp): a tie places for both, so a tie WINS us the
        //   game when we're at 2. Gamble freely.
        // - Level: tie is neutral progress, EXCEPT at 2-2 where a tie draws
        //   the game (rule 1.2.1.2) — be selective.
        // - Behind: a tie feeds their placement (at 1-2 a tie LOSES us the
        //   game); only an outright win helps, so if they're at match point
        //   we gamble desperately — losing is losing either way.
        let stance_floor: f64 = if my_success > opp_success {
            0.05
        } else if my_success == 2 && opp_success == 2 {
            0.20
        } else if my_success == opp_success {
            0.12
        } else if opp_success >= 2 {
            0.03
        } else {
            0.10
        };
        let floor = stance_floor.min(if opp_success >= 2 && my_success < 2 {
            policy.urgent_gamble_floor
        } else {
            policy.gamble_floor
        });
        let mut best: Option<(u32, usize, i32, Vec<usize>)> = None;
        for s in &subsets {
            let milli = s.prob_milli.get();
            if milli == 0 || (milli as f64 / 1000.0) < floor {
                continue;
            }
            let rank = (std::cmp::Reverse(milli), s.count, std::cmp::Reverse(-s.effective_score));
            if best
                .as_ref()
                .map_or(true, |(bp, bc, bs, _)| {
                    rank < (std::cmp::Reverse(*bp), *bc, std::cmp::Reverse(-*bs))
                })
            {
                best = Some((milli, s.count, -s.effective_score, s.indices.clone()));
            }
        }
        if let Some((_, _, _, indices)) = best {
            desired = indices;
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

/// V2 mulligan policy (S7 — curve completion, per the play guides):
/// - Keep the first 3 lives; redraw excess.
/// - Redraw dead high-cost members (cost > 15), max 2.
/// - No life in hand: fish for one by redrawing the most expensive members.
/// - No cheap opener (cost ≤ 4 member): redraw the most expensive member.
pub fn choose_mulligan_action_v2(
    gs: &GameState,
    actions: &[Action],
    db: &CardDatabase,
) -> Action {
    let me = gs.active_player();
    let mut discard: Vec<usize> = Vec::new();
    let mut live_count = 0usize;
    let mut members: Vec<(usize, u8)> = Vec::new(); // (hand_index, cost)
    let mut has_life = false;
    let mut has_cheap_opener = false;
    for (hand_index, &cid) in me.hand.cards.iter().enumerate() {
        let Some(card) = db.get_card(cid) else {
            continue;
        };
        match card.card_type {
            CardType::Live => {
                live_count += 1;
                has_life = true;
                if live_count > 3 {
                    discard.push(hand_index);
                }
            }
            CardType::Member => {
                let cost = card.cost.unwrap_or(0);
                members.push((hand_index, cost));
                if cost <= 4 {
                    has_cheap_opener = true;
                }
            }
            CardType::Energy => {}
        }
    }

    // Dead high-cost members.
    for &(hi, c) in &members {
        if c > 15 && discard.len() < 2 && !discard.contains(&hi) {
            discard.push(hi);
        }
    }

    // Fishing rules (only when the hand lacks a key piece).
    if !has_life || !has_cheap_opener {
        let mut sorted = members.clone();
        sorted.sort_by_key(|&(_, c)| std::cmp::Reverse(c));
        let want = if !has_life { 2 } else { 1 };
        for &(hi, cost) in sorted.iter() {
            if discard.len() >= 3 {
                break;
            }
            if !discard.contains(&hi) && cost > 7 {
                discard.push(hi);
                if discard.len() >= want + 2 {
                    break;
                }
            }
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

/// V2 main-phase evaluation: v1's heuristic with three adjustments.
/// - Match-point pressure: when the opponent closes on 3 success cards,
///   board-development terms are amplified (race harder or lose).
/// - Blades valued higher (they feed both yell pass probability and
///   スコア+1 icons).
/// - Color coverage: bonus for stage hearts covering the color requirements
///   of lives currently in hand (play members that enable your lives).
pub fn evaluate_state_v2(gs: &GameState, me: u8, w: &crate::bot::strategy::StrategyWeights) -> f64 {
    let base = crate::bot::strategy::evaluate_state(gs, me, w);

    let db = &gs.card_database;
    let (my, opp) = if me == 0 {
        (&gs.player1, &gs.player2)
    } else {
        (&gs.player2, &gs.player1)
    };
    let my_success = my.success_live_card_zone.cards.len();
    let opp_success = opp.success_live_card_zone.cards.len();

    let mut adjusted = w.clone();
    let pressure = match opp_success {
        2 => 1.6,
        1 => 1.2,
        _ => 1.0,
    };
    // Only amplify development terms, never the terminal/success terms.
    adjusted.stage_cost *= pressure;
    adjusted.heart *= pressure;
    adjusted.hand_size *= pressure;
    adjusted.blade = w.blade * 1.6;

    let mut val = crate::bot::strategy::evaluate_state(gs, me, &adjusted);

    // Color coverage: for each specific color required by hand lives, reward
    // having that color on stage (capped so hoarding one color doesn't stack).
    let mut need_by_color = [0i32; 7]; // Heart00..Heart06
    for &cid in my.hand.cards.iter() {
        if let Some(card) = db.get_card(cid) {
            if matches!(card.card_type, CardType::Live) {
                if let Some(nh) = &card.need_heart {
                    for (c, v) in nh.hearts.iter() {
                        let idx = hc_index(*c);
                        if idx < 7 {
                            need_by_color[idx] += *v as i32;
                        }
                    }
                }
            }
        }
    }
    if need_by_color.iter().sum::<i32>() > 0 {
        let have = stage_hearts(my, db);
        let covered = (0..7)
            .map(|i| have[i].min(need_by_color[i]))
            .sum::<i32>()
            .max(0);
        let total = need_by_color.iter().sum::<i32>().max(1);
        val += (covered as f64 / total as f64) * w.heart * 2.0
            * if my_success >= opp_success { 1.0 } else { pressure };
    }

    let _ = base;
    val
}

/// V2 main-phase choice: v2 evaluation (see `evaluate_state_v2`).
pub fn choose_action_heuristic_v2(
    gs: &GameState,
    actions: &[Action],
    me: u8,
) -> Action {
    if actions.len() == 1 {
        return actions[0].clone();
    }
    let db = &gs.card_database;
    let my_now = if me == 0 { &gs.player1 } else { &gs.player2 };
    let mut best_idx = 0usize;
    let mut best_val = f64::NEG_INFINITY;
    for (i, a) in actions.iter().enumerate() {
        let mut sim = gs.clone();
        if game_setup::execute_action(&mut sim, a).is_err() {
            continue;
        }
        game_setup::settle_single_player_state(&mut sim);
        let mut val = evaluate_state_v2(&sim, me, &crate::bot::strategy::StrategyWeights::fair());

        // No-op breaker: an activation that leaves hand/energy/stage/deck/
        // waitroom counts unchanged bought nothing (e.g. pressing an
        // unpayable 起動 whose cost fizzles). Without this the eval's
        // first-index tie-break re-picks it forever — observed as 30+
        // identical use_ability picks and arena "turn-6 draws".
        let my_sim = if me == 0 { &sim.player1 } else { &sim.player2 };
        if my_sim.hand.cards.len() == my_now.hand.cards.len()
            && my_sim.energy_zone.active_count() == my_now.energy_zone.active_count()
            && my_sim.stage.stage == my_now.stage.stage
            && my_sim.main_deck.cards.len() == my_now.main_deck.cards.len()
            && my_sim.waitroom.cards.len() == my_now.waitroom.cards.len()
        {
            val -= 1000.0;
        }
        // Hand-reserve discipline: ≤1 card can't set lives or pay discards.
        if my_sim.hand.cards.len() <= 1 {
            val -= 40.0;
        }
        let _ = db;
        if val > best_val {
            best_val = val;
            best_idx = i;
        }
    }
    actions[best_idx].clone()
}
