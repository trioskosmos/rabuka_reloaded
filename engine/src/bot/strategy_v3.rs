//! Experimental strategy bot v3  Egeneral planning.
//!
//! Adds a planning layer on top of v2's yell-math live-set policy:
//!
//! 1. **Archetype / rush window**: cheap lives in the FULL own decklist ↁE//!    play them turns 1 E ("zerg rush" the checks) while still growing the
//!    stage. High-score decks ramp instead.
//! 2. **Curve milestones**: the guides' stage-cost arc (~2 ↁE9 ↁE13/15 ↁE//!    22+) as explicit per-turn targets; the eval penalizes falling behind
//!    the curve.
//! 3. **Acquisition reasoning**: main-phase actions are valued partly by
//!    whether they *acquire wanted cards*  Elives into hand during the rush
//!    window, ammo recycled from the waitroom, playable-cost members for the
//!    next curve step. (Outcome-delta approximation; semantic ability
//!    understanding is v4/v5.)
//! 4. **Hand usefulness model**: classify every hand card as useful *right
//!    now* or dead  Elives whose heart requirements the current stage can't
//!    satisfy, members whose cost is unreachable this/next turn with no baton
//!    partner. Drives mulligan discards and live-set hand filtering (dead
//!    cards set as lives are discarded by Rule 10.5.1 and replaced by fresh
//!    draws).
//!
//! Fairness: same rules as v2  Eown deck/hand is known, opponent info is
//! public-only.

use crate::card::{CardDatabase, CardType};
use crate::game_setup::{self, Action};
use crate::game_state::GameState;
use crate::bot::strategy::StrategyWeights;
use crate::bot::strategy_v2::V2Policy;

/// Diagnostics counters (printed by the arena bin).
pub static V3_STATS: [std::sync::atomic::AtomicU64; 5] = [
    std::sync::atomic::AtomicU64::new(0),
    std::sync::atomic::AtomicU64::new(0),
    std::sync::atomic::AtomicU64::new(0),
    std::sync::atomic::AtomicU64::new(0),
    std::sync::atomic::AtomicU64::new(0),
];
// 0: live-set calls, 1: dump selects made, 2: held-line confirms,
// 3: mulligan calls, 4: mulligan discards chosen

// ── Local heart-accounting helpers (kept private to v3 so v2 stays
//    untouched) ────────────────────────────────────────────────────────────

type HeartAcc = [i32; 11];

fn heart_index(c: crate::card::HeartColor) -> usize {
    use crate::card::HeartColor as H;
    match c {
        H::Heart00 => 0,
        H::Heart01 => 1,
        H::Heart02 => 2,
        H::Heart03 => 3,
        H::Heart04 => 4,
        H::Heart05 => 5,
        H::Heart06 => 6,
        H::BAll => 7,
        H::Draw => 8,
        H::Score => 9,
        H::All => 10,
    }
}

fn acc_add_hearts(acc: &mut HeartAcc, hearts: &crate::card::HeartMap) {
    for (c, v) in hearts.iter() {
        acc[heart_index(*c)] += *v as i32;
    }
}

/// Sum of stage members' base hearts.
fn stage_base_hearts(p: &crate::player::Player, db: &CardDatabase) -> HeartAcc {
    let mut acc = [0i32; 11];
    for &cid in p.stage.stage.iter() {
        if cid < 0 {
            continue;
        }
        if let Some(card) = db.get_card(cid) {
            if let Some(bh) = &card.base_heart {
                acc_add_hearts(&mut acc, &bh.hearts);
            }
        }
    }
    acc
}

/// Heart requirement check (rule 2.1.1.2 / 2.11.3): specific colors must be
/// met exactly; All supply and BAll blade-hearts are wildcards; Heart00 is
/// the any-color bucket filled by colorless hearts and leftovers.
fn reqs_met(have: &HeartAcc, need: &HeartAcc) -> bool {
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
    let leftover = specific_surplus + wildcard.max(0) + have[0];
    leftover >= need[0]
}

/// Stage-total-cost milestones per turn (index 0 = turn 1). Derived from the
/// competitive guides' standard development: T1 opener, T2 baton to ~9,
/// T3 ~13 E5, T4 the big center (~22+), then steady growth.
const CURVE: [i32; 8] = [2, 9, 15, 24, 30, 36, 42, 48];

/// Turns during which a cheap-life rush is viable.
const RUSH_WINDOW_END: u8 = 4;

pub struct V3Plan {
    /// Deck/hand skews cheap ↁEflood early checks.
    pub rush_archetype: bool,
}

impl V3Plan {
    /// Archetype detection over the FULL own deck list.
    ///
    /// Knowing your own 50-card list is not cheating (fairness rules: own
    /// deck is public to you)  Ethe old version judged the archetype from
    /// ~10 visible opening cards, so a rush deck that drew no cheap lives
    /// misidentified as ramp. Scan main deck + hand + waitroom + stage +
    /// live zone: every card we legitimately own knowledge of.
    ///
    /// Call ONCE per game (game start); the archetype doesn't change mid-game.
    pub fn detect(gs: &GameState, me_player: u8, db: &CardDatabase) -> Self {
        let my = if me_player == 0 {
            &gs.player1
        } else {
            &gs.player2
        };
        let mut cheap = 0usize;
        let mut total_lives = 0usize;
        let mut scan = |cards: &[i16]| {
            for &cid in cards {
                if let Some(card) = db.get_card(cid) {
                    if matches!(card.card_type, CardType::Live) {
                        total_lives += 1;
                        if card.score.unwrap_or(9) <= 2 {
                            cheap += 1;
                        }
                    }
                }
            }
        };
        scan(&my.main_deck.cards);
        scan(&my.hand.cards);
        scan(&my.waitroom.cards);
        scan(&my.live_card_zone.cards);
        scan(&my.success_live_card_zone.cards);
        // Stage members are also ours; lives never sit on stage but include
        // them for completeness of the "every card we own" scan.
        scan(&my.stage.stage);
        let rush_archetype = total_lives > 0 && cheap * 2 >= total_lives;
        Self { rush_archetype }
    }

    pub fn in_rush_window(&self, turn: u8) -> bool {
        self.rush_archetype && turn <= RUSH_WINDOW_END
    }

    pub fn milestone(&self, turn: u8) -> i32 {
        CURVE[(turn as usize - 1).min(CURVE.len() - 1)]
    }
}

/// Cards-we-want snapshot used for acquisition deltas.
#[derive(Debug, Clone, Copy)]
struct AcqFeatures {
    lives_in_hand: usize,
    playable_members_in_hand: usize,
    lives_in_waitroom: usize,
}

fn acq_features(p: &crate::player::Player, db: &CardDatabase) -> AcqFeatures {
    let mut f = AcqFeatures {
        lives_in_hand: 0,
        playable_members_in_hand: 0,
        lives_in_waitroom: 0,
    };
    for &cid in p.hand.cards.iter() {
        if let Some(card) = db.get_card(cid) {
            match card.card_type {
                CardType::Live => f.lives_in_hand += 1,
                CardType::Member => {
                    if card.cost.unwrap_or(99) <= 15 {
                        f.playable_members_in_hand += 1;
                    }
                }
                CardType::Energy => {}
            }
        }
    }
    for &cid in p.waitroom.cards.iter() {
        if let Some(card) = db.get_card(cid) {
            if matches!(card.card_type, CardType::Live) {
                f.lives_in_waitroom += 1;
            }
        }
    }
    f
}

/// Curve adherence: penalize being UNDER the milestone (over-curving is fine
///  Eextra stage cost is never bad, per S1).
fn curve_term(gs: &GameState, me_player: u8, plan: &V3Plan, w: &StrategyWeights) -> f64 {
    let my = if me_player == 0 {
        &gs.player1
    } else {
        &gs.player2
    };
    let mut stage_cost = 0i32;
    for &cid in my.stage.stage.iter() {
        if cid >= 0 {
            if let Some(card) = gs.card_database.get_card(cid) {
                stage_cost += card.cost.unwrap_or(0) as i32;
            }
        }
    }
    let milestone = plan.milestone(gs.turn_number);
    let shortfall = (milestone - stage_cost).max(0);
    -(shortfall as f64) * w.stage_cost * 0.8
}

/// V3 evaluation: v2 evaluation + curve adherence + unconverted-energy
/// penalty.
///
/// The relative active-energy term in v1/v2 rewards banking energy vs the
/// opponent, but energy REGENERATES every Active phase (7.4.1): active
/// energy sitting unused when my turn ends is board I failed to buy, and it
/// will still be sitting there next turn. Penalize MY absolute held energy
/// so spending down into stage (hearts compound across all future checks)
/// wins ties. Mild weight: legitimate reservations for live-phase ability
/// costs (【自動】(コスト) abilities fire outside the main phase) must not
/// be punished into oblivion.
/// V3 evaluation: v2 evaluation + curve adherence.
///
/// The absolute held-energy penalty was ablated: it measured neutral on
/// wins while reallocating plays toward ability spam (game 1 t9–t22).
pub fn evaluate_state_v3(
    gs: &GameState,
    me: u8,
    w: &StrategyWeights,
    plan: &V3Plan,
) -> f64 {
    crate::bot::strategy_v2::evaluate_state_v2(gs, me, w) + curve_term(gs, me, plan, w)
}

/// V3 main-phase choice: v2's clone-and-eval plus acquisition deltas  E/// reward actions that obtain cards we want right now.
pub fn choose_action_heuristic_v3(
    gs: &GameState,
    actions: &[Action],
    me: u8,
    plan: &V3Plan,
) -> Action {
    if actions.len() == 1 {
        return actions[0].clone();
    }

    let db = &gs.card_database;
    let my_before = if me == 0 {
        acq_features(&gs.player1, db)
    } else {
        acq_features(&gs.player2, db)
    };
    let rush = plan.in_rush_window(gs.turn_number);

    let mut best_idx = 0usize;
    let mut best_val = f64::NEG_INFINITY;

    for (i, a) in actions.iter().enumerate() {
        let mut sim = gs.clone();
        if game_setup::execute_action(&mut sim, a).is_err() {
            continue;
        }
        game_setup::settle_single_player_state(&mut sim);

        // No-op loop breaker (game 1, t9–t22): v3 re-ran the same retrieve
        // 起動 14 turns straight because each step locally looked positive
        // (+1 hand) while nothing progressed. If an action leaves every key
        // quantity unchanged, it bought nothing — value it below Pass.
        let my_sim = if me == 0 { &sim.player1 } else { &sim.player2 };
        let my_now = if me == 0 { &gs.player1 } else { &gs.player2 };
        let no_op = my_sim.hand.cards.len() == my_now.hand.cards.len()
            && my_sim.energy_zone.active_count() == my_now.energy_zone.active_count()
            && my_sim.stage.stage == my_now.stage.stage
            && my_sim.main_deck.cards.len() == my_now.main_deck.cards.len()
            && my_sim.waitroom.cards.len() == my_now.waitroom.cards.len();

        let mut val =
            crate::bot::strategy_v2::evaluate_state_v2(&sim, me, &StrategyWeights::fair())
                + curve_term(&sim, me, plan, &StrategyWeights::fair());
        if no_op {
            val -= 1000.0;
        }
        // Hand-reserve discipline (game 14): v3 liquidated its whole hand on
        // member spam (hand 3→0 at t9) and sat unable to set lives or pay
        // discard costs for the rest of the game. A state with ≤1 hand card
        // is functionally paralyzed.
        if my_sim.hand.cards.len() <= 1 {
            val -= 40.0;
        }

        // Acquisition deltas (what did this action get me?).
        let my_after = if me == 0 {
            acq_features(&sim.player1, db)
        } else {
            acq_features(&sim.player2, db)
        };
        let d_lives = my_after.lives_in_hand as i32 - my_before.lives_in_hand as i32;
        let d_members = my_after.playable_members_in_hand as i32
            - my_before.playable_members_in_hand as i32;
        let d_wr_lives =
            my_after.lives_in_waitroom as i32 - my_before.lives_in_waitroom as i32;

        // Lives into hand: strong want during the rush window (ammo for the
        // flood), moderate otherwise — and DESPERATION-scaled when the hand
        // is already dry: game-6 autopsies showed v3 sitting at zero lives
        // for 8 straight turns while retrieval abilities went unused because
        // a flat +25 couldn't outbid other terms. With no ammo, retrieving
        // one life is worth more than any single board upgrade.
        let ammo_starved = my_before.lives_in_hand == 0;
        let per_life = if ammo_starved {
            150.0
        } else if rush {
            60.0
        } else {
            25.0
        };
        val += d_lives as f64 * per_life;
        if d_lives < 0 && my_after.lives_in_hand == 0 {
            // Never reward actions that burn the last life (costs that
            // discard lives) — that's how game-6 starvation started.
            val -= 150.0;
        }
        // Recycled ammo (waitroom -> hand shows as wr decrease + hand gain);
        // slight extra credit because waitroom lives are "free" wins.
        if d_lives > 0 && d_wr_lives < 0 {
            val += 20.0;
        }
        // Playable members keep the curve moving.
        val += d_members as f64 * 12.0;

        if val > best_val {
            best_val = val;
            best_idx = i;
        }
    }

    actions[best_idx].clone()
}

/// V3 live-set policy: v2's yell-math selection, with rush-window tuning  E/// during turns 1 E with a cheap-life skeleton, be more willing to gamble
/// (lower floor) since flooding early checks is the whole plan.
///
/// On top of that: **hand filtering**. When v2's plan finishes with spare
/// live-set slots and we're not behind, dump dead hand cards (uncovered
/// lives / unreachable members) into the live zone  ERule 10.5.1 discards
/// non-lives from it, and every placed card draws a replacement. Dead weight
/// becomes fresh cards.
pub fn choose_live_set_action_v3(
    gs: &GameState,
    actions: &[Action],
    db: &CardDatabase,
    _policy: &V2Policy,
    _plan: &V3Plan,
) -> Action {
    // Score-escalating portfolio planner.
    //
    // Trace evidence (78 games): both bots set exactly ONE life 100% of the
    // time and place only 0.33 successes/turn, even on 30-heart boards that
    // support 4–6 point lives. v2's ladder ranks by pass-probability then
    // smallest count — it can never discover the guides' escalating arc
    // (1点→2点→3点→multi-life). Instead: maximize TOTAL SCORE subject to
    // deterministic heart coverage from the stage pool, up to 3 lives.
    let me = if gs.active_player().id == gs.player1.id { 0u8 } else { 1u8 };
    plan_score_portfolio(gs, actions, db, me)
}

/// Greedy score-maximizing portfolio: sort hand lives by score desc, add
/// each whose requirements still fit in the remaining heart pool (rule
/// 8.3.15 order-aware allocation). Flips are upside, never load-bearing.
fn plan_score_portfolio(
    gs: &GameState,
    actions: &[Action],
    db: &CardDatabase,
    me: u8,
) -> Action {
    let my = if me == 0 { &gs.player1 } else { &gs.player2 };
    let mut pool = stage_base_hearts(my, db);

    // Expected yell hits join the pool as wildcard hearts (rule 8.3.15.1.1:
    // flipped icons act as any-one-color). Without this buffer the planner
    // refuses lives whose specific colors aren't already on stage — observed
    // as v3 setting NOTHING turns 1–4 and again turns 9–22 of game 1 while
    // holding perfectly playable lives.
    let mut blades = 0i32;
    for &cid in my.stage.stage.iter() {
        if cid < 0 {
            continue;
        }
        let waiting = gs.mods.get_orientation_modifier(cid) == Some("wait");
        if !waiting {
            if let Some(card) = db.get_card(cid) {
                blades += card.blade as i32;
            }
        }
    }
    let deck_len = my.main_deck.cards.len().max(1);
    let density = my
        .main_deck
        .cards
        .iter()
        .filter(|&&cid| db.get_card(cid).map_or(false, |c| c.blade_heart.is_some()))
        .count() as f64
        / deck_len.max(1) as f64;
    let expected_hits = (blades as f64 * density).round() as i32;
    pool[10] += expected_hits;

    let mut candidates: Vec<(usize, i32, [i32; 11])> = Vec::new();
    for (hand_index, &cid) in my.hand.cards.iter().enumerate() {
        let Some(card) = db.get_card(cid) else {
            continue;
        };
        if !matches!(card.card_type, CardType::Live) {
            continue;
        }
        let mut need = [0i32; 11];
        if let Some(nh) = &card.need_heart {
            acc_add_hearts(&mut need, &nh.hearts);
        }
        candidates.push((hand_index, card.score.unwrap_or(0) as i32, need));
    }
    candidates.sort_by(|a, b| b.1.cmp(&a.1));

    let max_slots =
        (3i32 - i32::from(my.live_card_set_limit_reduction)).max(0) as usize;
    let mut desired: Vec<usize> = Vec::new();
    for &(hi, _score, ref need) in &candidates {
        if desired.len() >= max_slots {
            break;
        }
        let mut next = pool;
        if allocate_life(&mut next, need) {
            desired.push(hi);
            pool = next;
        }
    }

    // Free-win discipline (rule 8.4.3.2): setting a life is how I win.
    // Trace evidence (game 1 t9+): both bots confirmed EMPTY sets for 9
    // straight turns — guaranteed stall. When the deterministic portfolio
    // came up empty but I hold lives below match point, set the
    // closest-to-coverable one instead of passing.
    if desired.is_empty() && !candidates.is_empty() && my.success_live_card_zone.cards.len() < 2 {
        let mut best: Option<(i32, usize)> = None; // (deficit, hand_index)
        for &(hi, _score, ref need) in &candidates {
            let d = requirement_deficit(&pool, need);
            if best.map_or(true, |(bd, _)| d < bd) {
                best = Some((d, hi));
            }
        }
        if let Some((deficit, hi)) = best {
            // Allow a small shortfall: flips overshoot the density estimate
            // half the time, and losing one life card is cheap next to a
            // guaranteed pass-fest.
            if deficit <= 3 {
                desired.push(hi);
            }
        }
    }

    // Emit one action toward `desired`.
    let selected: Vec<usize> = gs
        .live_card_selected_indices
        .iter()
        .map(|&i| i as usize)
        .collect();
    let find_select = |hand_index: usize, want: bool| -> Option<Action> {
        actions
            .iter()
            .find(|a| {
                a.action_type == game_setup::ActionType::SelectLiveCard
                    && a.selected == Some(want)
                    && a.parameters.as_ref().and_then(|p| p.card_index) == Some(hand_index)
            })
            .cloned()
    };
    for &hi in &desired {
        if !selected.contains(&hi) {
            if let Some(a) = find_select(hi, false) {
                return a;
            }
        }
    }
    for &hi in &selected {
        if !desired.contains(&hi) {
            if let Some(a) = find_select(hi, true) {
                return a;
            }
        }
    }
    confirm_live_set(actions)
}
pub fn estimate_max_score(
    gs: &GameState,
    me_player: u8,
    db: &CardDatabase,
) -> i32 {
    let my = if me_player == 0 {
        &gs.player1
    } else {
        &gs.player2
    };
    let mut hearts = 0i32;
    let mut blades = 0i32;
    for &cid in my.stage.stage.iter() {
        if cid < 0 {
            continue;
        }
        // Wait members still provide hearts but no blades (no yell).
        let waiting = gs.mods.get_orientation_modifier(cid) == Some("wait");
        if let Some(card) = db.get_card(cid) {
            if let Some(bh) = &card.base_heart {
                hearts += bh.hearts.values_sum() as i32;
            }
            if !waiting {
                blades += card.blade as i32;
            }
        }
    }
    // Blade-heart density estimate: fraction of own main deck carrying
    // blade-hearts (fair — own decklist is known).
    let deck_len = my.main_deck.cards.len().max(1);
    let density = if deck_len > 0 {
        my.main_deck
            .cards
            .iter()
            .filter(|&&cid| db.get_card(cid).map_or(false, |c| c.blade_heart.is_some()))
            .count() as f64
            / deck_len as f64
    } else {
        0.0
    };
    let total = hearts + (blades as f64 * density).round() as i32;
    // Largest score band whose median requirement fits (exact table from
    // cards.json; see docs/BOT_STRATEGY_TREE.md §1.3).
    const BAND: [i32; 10] = [0, 3, 5, 7, 10, 12, 14, 16, 19, 21];
    let mut best = 0;
    for (s, &need) in BAND.iter().enumerate().skip(1) {
        if need <= total {
            best = s as i32;
        }
    }
    best
}

/// Allocate one life's requirements from a remaining heart pool.
/// Mirrors 8.3.15: specific colors first, wildcards (All/BAll) cover
/// deficits, grey bucket takes colorless + leftovers. Returns false if the
/// life can't be satisfied from what remains.
fn allocate_life(pool: &mut HeartAcc, need: &HeartAcc) -> bool {
    // Work on a copy; commit only if fully satisfiable.
    let mut p = *pool;

    // 1) Specific colors Heart01–06: own color first, wildcards (All + BAll)
    //    cover deficits.
    let mut wildcard_used = 0i32;
    let mut wildcard = p[7] + p[10];
    for c in 1..=6 {
        let have = p[c];
        let want = need[c];
        if have >= want {
            p[c] = have - want;
        } else {
            let deficit = want - have;
            if deficit > wildcard {
                return false;
            }
            wildcard_used += deficit;
            wildcard -= deficit;
            p[c] = 0;
        }
    }
    let from_ball = wildcard_used.min(p[7]);
    p[7] -= from_ball;
    p[10] -= wildcard_used - from_ball;

    // 2) Grey bucket (Heart00): colorless hearts + any leftover specific or
    //    wildcard hearts (rule 8.3.15 order-aware leftovers).
    let mut grey = need[0];
    let t0 = grey.min(p[0]);
    p[0] -= t0;
    grey -= t0;
    for c in (1..=6).rev() {
        if grey <= 0 {
            break;
        }
        let t = grey.min(p[c]);
        p[c] -= t;
        grey -= t;
    }
    if grey > 0 {
        let w = p[7] + p[10];
        if grey > w {
            return false;
        }
        let b2 = grey.min(p[7]);
        p[7] -= b2;
        p[10] -= grey - b2;
    }

    *pool = p;
    true
}

fn confirm_live_set(actions: &[Action]) -> Action {
    actions
        .iter()
        .find(|a| a.action_type == game_setup::ActionType::ConfirmLiveCardSet)
        .or_else(|| actions.first())
        .cloned()
        .expect("live set actions non-empty")
}

/// How many heart icons short `pool` is of satisfying `need` (rough, for
/// fallback gambles): specific-color shortfalls beyond wildcards, plus grey
/// bucket shortfall after colorless+leftovers.
fn requirement_deficit(pool: &HeartAcc, need: &HeartAcc) -> i32 {
    let mut wildcard = pool[10] + pool[7];
    let mut deficit = 0i32;
    let mut leftover = 0i32;
    for c in 1..=6 {
        let d = need[c] - pool[c];
        if d > 0 {
            if d > wildcard {
                deficit += d - wildcard;
            }
            wildcard -= d.min(wildcard);
        } else {
            leftover += -d;
        }
    }
    // Grey bucket: colorless + leftovers + unused wildcards.
    let available_grey = pool[0] + leftover + wildcard.max(0);
    if need[0] > available_grey {
        deficit += need[0] - available_grey;
    }
    deficit
}

/// Classification of every hand card by whether it is usable *now*.
#[derive(Debug, Default)]
pub struct HandUsefulness {
    /// Lives whose heart requirements the current stage satisfies.
    pub covered_lives: Vec<usize>,
    /// Lives that cannot check successfully right now.
    pub uncovered_lives: Vec<usize>,
    /// Members affordable this turn or next (energy + one draw).
    pub playable_members: Vec<usize>,
    /// Members too expensive to reach within ~2 turns with no baton partner.
    pub dead_members: Vec<usize>,
}

impl HandUsefulness {
}

/// Classify the active player's hand. Mid-game signal: stage hearts for
/// lives, energy reachability for members.
pub fn analyze_hand(gs: &GameState, me_player: u8, db: &CardDatabase) -> HandUsefulness {
    let my = if me_player == 0 {
        &gs.player1
    } else {
        &gs.player2
    };
    let hearts = stage_base_hearts(my, db);
    let mut out = HandUsefulness::default();

    for (hand_index, &cid) in my.hand.cards.iter().enumerate() {
        let Some(card) = db.get_card(cid) else {
            continue;
        };
        if !matches!(card.card_type, CardType::Live) {
            continue;
        }
        let mut need = [0i32; 11];
        if let Some(nh) = &card.need_heart {
            acc_add_hearts(&mut need, &nh.hearts);
        }
        if reqs_met(&hearts, &need) {
            out.covered_lives.push(hand_index);
        } else {
            out.uncovered_lives.push(hand_index);
        }
    }

    let active_energy = i32::from(my.energy_zone.active_count());
    for (hand_index, &cid) in my.hand.cards.iter().enumerate() {
        let Some(card) = db.get_card(cid) else {
            continue;
        };
        if !matches!(card.card_type, CardType::Member) {
            continue;
        }
        let cost = i32::from(card.cost.unwrap_or(0));
        if cost <= active_energy + 1 {
            out.playable_members.push(hand_index);
            continue;
        }
        // Dead unless some stage member is expensive enough that a baton
        // touch could fund it within two turns.
        let baton_fodder = my.stage.stage.iter().any(|&sid| {
            sid >= 0
                && db
                    .get_card(sid)
                    .and_then(|c| c.cost)
                    .map_or(false, |sc| {
                        i32::from(sc) >= cost - (active_energy + 2)
                    })
        });
        if cost > active_energy + 2 && !baton_fodder {
            out.dead_members.push(hand_index);
        }
    }
    out
}

/// V3 mulligan policy: discard what the usefulness model says is dead,
/// informed by the FULL own decklist (fair  Eit's our deck).
///
/// - A life is keepable if its needed colors are well-supported by deck
///   members (≥ `MIN_COLOR_SUPPORT` producers per color in our 50): at
///   mulligan time the stage is empty, so "covered" must mean "coverable".
/// - A member is dead if nothing else in the deck can baton-fund it
///   (cost > 12 with no cheaper partner of cost ≥ cost∁E).
/// - Fish when the hand lacks any supported life.
pub fn choose_mulligan_action_v3(
    gs: &GameState,
    actions: &[Action],
    db: &CardDatabase,
) -> Action {
    let me = gs.active_player();
    let mut discard: Vec<usize> = Vec::new();
    let mut kept_lives = 0usize;
    let mut member_costs: Vec<(usize, u8)> = Vec::new();
    let mut has_supported_life = false;

    // Color producer counts across the whole own decklist.
    let mut producers = [0i32; 7]; // Heart00..Heart06 specific colors
    for &cid in me.main_deck.cards.iter() {
        if let Some(card) = db.get_card(cid) {
            if matches!(card.card_type, CardType::Member) {
                if let Some(bh) = &card.base_heart {
                    for (c, v) in bh.hearts.iter() {
                        let idx = heart_index(*c);
                        if idx < 7 && *v > 0 {
                            producers[idx] += 1;
                        }
                    }
                }
            }
        }
    }

    const MIN_COLOR_SUPPORT: i32 = 5;
    // Grey (Heart00) requirements are the ANY-COLOR bucket — judge them
    // against total heart production across all colors, not grey-only.
    let total_producers: i32 = producers.iter().sum();
    for (hand_index, &cid) in me.hand.cards.iter().enumerate() {
        let Some(card) = db.get_card(cid) else {
            continue;
        };
        match card.card_type {
            CardType::Live => {
                // Supported iff every needed color has enough producers in
                // the deck.
                let supported = card.need_heart.as_ref().map_or(true, |nh| {
                    nh.hearts.iter().all(|(c, v)| {
                        let idx = heart_index(*c);
                        if idx >= 7 {
                            true // wildcards are always support
                        } else if idx == 0 {
                            total_producers >= MIN_COLOR_SUPPORT * (*v as i32)
                        } else {
                            producers[idx] >= MIN_COLOR_SUPPORT * (*v as i32)
                        }
                    })
                });
                if supported && kept_lives < 3 {
                    kept_lives += 1;
                    has_supported_life = true;
                } else {
                    discard.push(hand_index);
                }
            }
            CardType::Member => {
                member_costs.push((hand_index, card.cost.unwrap_or(0)));
            }
            CardType::Energy => {}
        }
    }

    // Dead members: expensive with no baton partner among kept-cost cards.
    for &(hi, c) in &member_costs {
        if c > 12
            && !discard.contains(&hi)
            && !member_costs.iter().any(|&(other_hi, oc)| {
                other_hi != hi && !discard.contains(&other_hi) && oc >= c - 4 && oc < c
            })
        {
            discard.push(hi);
        }
    }
    // Cap discards at 3 (mulligan limit).
    discard.truncate(3);

    // Fishing: no supported life ↁEredraw the most expensive members.
    if !has_supported_life {
        let mut sorted = member_costs.clone();
        sorted.sort_by_key(|&(_, c)| std::cmp::Reverse(c));
        for &(hi, cost) in sorted.iter() {
            if discard.len() >= 3 {
                break;
            }
            if !discard.contains(&hi) && cost > 7 {
                discard.push(hi);
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
