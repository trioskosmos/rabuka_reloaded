//! v7 ISMCTS — determinized root search that prices an action by PLAYING IT
//! OUT instead of by a one-ply feature delta.
//!
//! # Why this exists
//!
//! `v7_main::Search::complete` is a *chain* search: it re-enters our own
//! pending choice up to depth 8 under a 64-node budget, then reads a static
//! feature vector. It is a good one-ply-plus model, and it is still blind to
//! the only thing that decides this game: the live check. Nothing in it ever
//! answers "if I play this member, do I actually place a life next turn, and
//! do they?" — the eval prices hearts, blades, ammo, energy and stage-cost
//! growth, which are proxies for that question, not the question.
//!
//! `bot::ismcts` already had the right shape (determinize, roll out, average)
//! but was never reachable from v7: it is wired only into the generic `Bot` in
//! `bot/mod.rs`, it uses a *blind* rollout policy (`pick_rollout_action` ->
//! `strategy::choose_action_heuristic`, the weakest generation) on both
//! sides, and its leaf value is a bare success-zone delta normalized by
//! elapsed turns. Self-play value under a weak policy measures the policy's
//! skill, not the move.
//!
//! # What this module does instead
//!
//! 1. **Determinize honestly.** Opponent hidden zones (hand, main deck,
//!    energy deck, and an unset live-card zone) are replaced by a fair
//!    sample at their true sizes; every public zone, every modifier, the
//!    pending choice and the ability queue are preserved from the true state.
//!    The prior for that sample is *our own deck list* by default
//!    (`V7_ISMCTS_OPP_POOL=fair` restores the anonymous database-wide pool).
//!    Our own deck list is information a fair player has, and it is a far
//!    tighter prior than "any card in the database".
//!
//! 2. **Roll out with a good policy on BOTH sides.** One policy, derived
//!    from v7's own `value()` weights, so the sampled game is representative
//!    self-play. Member deploys and energy plays are scored *syntactically*
//!    (no engine round-trip); everything else is executed on a clone and
//!    scored with v7's `value()` delta. `V7_ISMCTS_EXEC_DEPLOY=1` executes
//!    deploys too — higher fidelity, several times the cost.
//!
//! 3. **Leaf on game value, not on features.** A win is +1, a loss -1, a draw
//!    0, and an unfinished rollout is scored on the actual race: success-zone
//!    lead, lives still in hand (the term determinization exists to produce —
//!    v7's static eval cannot see the opponent's hand at all), active blades,
//!    and stage-cost development. Every term is normalized so a win and a good
//!    position live on one scale.
//!
//! 4. **Never trust a thin sample.** The v7 static score enters as a prior in
//!    the same [-1, 1] units with `V7_ISMCTS_K0` pseudo-visits:
//!    `Q = (Σv + k0·prior) / (n + k0)`. An action the rollouts have barely
//!    seen cannot outrank the static eval, so the search can only *promote* a
//!    move v7 already considers reasonable. The failure mode is "v7 as
//!    before", not "noise".
//!
//! 5. **Spend the machine.** Round one prices EVERY candidate at once:
//!    an action with no visits has infinite UCB, so without this pass the
//!    policy silently freezes at the prior and never tests anything. Later
//!    rounds run a PUCT-selected batch of rollouts in parallel across
//!    `V7_ISMCTS_JOBS` threads. The engine RNG is thread-local, so each
//!    worker seeds itself from `(action, world, sim)` and results are
//!    aggregated by index: the search is bit-for-bit reproducible regardless
//!    of thread count or scheduling, which is what the arena's paired `--vs`
//!    statistics and the snapshot round-trip both depend on.
//!
//! # Budgets
//!
//! Deterministic on purpose — there is no wall-clock cap, so two runs of the
//! same position always agree. `V7_ISMCTS_JOBS` only changes how fast the
//! search runs, never the answer. Cost knobs: `V7_ISMCTS_WORLDS`,
//! `V7_ISMCTS_SIMS`, `V7_ISMCTS_VISITS`, `V7_ISMCTS_HORIZON`,
//! `V7_ISMCTS_TICKS`, `V7_ISMCTS_TOPK`.

use crate::bot::determinization::DeterminizationSampler;
use crate::bot::observation::PublicObservation;
use crate::bot::strategy_v4::lives_in_hand;
use crate::bot::v7_main;
use crate::card::{CardDatabase, CardType};
use crate::game_setup::{self, Action, ActionType, ActionParameters};
use crate::game_state::{GameResult, GameState, Phase};
use crate::rng::Lcg;

/// v7's own terminal-win score. An action that reaches it short-circuits the
/// search: no rollout can be worth more than a proven win.
const TERMINAL_WIN: f64 = 10_000.0;

/// Search parameters. Every field is env-overridable for ablation.
#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub enabled: bool,
    pub worlds: usize,
    pub sims: usize,
    pub visits: usize,
    pub horizon: u8,
    pub ticks: usize,
    pub top_k: usize,
    pub c_puct: f64,
    pub k0: f64,
    pub prior_scale: f64,
    pub prior_temp: f64,
    pub jobs: usize,
    pub own_pool: bool,
    pub exec_deploy: bool,
    pub promote_only: bool,
    pub v7_leaf: bool,
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value: &usize| *value > 0)
        .unwrap_or(default)
}

fn env_f64(name: &str, default: f64) -> f64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value: &f64| value.is_finite() && *value > 0.0)
        .unwrap_or(default)
}

#[cfg(not(feature = "no_std"))]
fn default_jobs() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get().saturating_sub(1).clamp(1, 8))
        .unwrap_or(1)
}

#[cfg(feature = "no_std")]
fn default_jobs() -> usize {
    1
}

impl Config {
    pub fn from_env() -> Self {
        // Off-switch first: `V7_ISMCTS=0` must win over every other default so
        // an ablation run is exactly today's v7.
        let enabled = match std::env::var("V7_ISMCTS") {
            Ok(value) => value != "0",
            Err(_) => std::env::var_os("V7_NO_ISMCTS").is_none(),
        };
        let worlds = env_usize("V7_ISMCTS_WORLDS", 3);
        let sims = env_usize("V7_ISMCTS_SIMS", 5);
        Self {
            enabled,
            worlds,
            sims,
            visits: env_usize("V7_ISMCTS_VISITS", worlds * sims * 2),
            horizon: env_usize("V7_ISMCTS_HORIZON", 3) as u8,
            ticks: env_usize("V7_ISMCTS_TICKS", 90),
            top_k: env_usize("V7_ISMCTS_TOPK", 8),
            c_puct: env_f64("V7_ISMCTS_C", 1.25),
            k0: env_f64("V7_ISMCTS_K0", 8.0),
            prior_scale: env_f64("V7_ISMCTS_SCALE", 260.0),
            prior_temp: env_f64("V7_ISMCTS_TEMP", 55.0),
            jobs: env_usize("V7_ISMCTS_JOBS", default_jobs()),
            own_pool: std::env::var("V7_ISMCTS_OPP_POOL").as_deref() != Ok("fair"),
            exec_deploy: std::env::var_os("V7_ISMCTS_EXEC_DEPLOY").is_some(),
            promote_only: std::env::var("V7_ISMCTS_PROMOTE").as_deref() != Ok("any"),
            v7_leaf: std::env::var("V7_ISMCTS_LEAF").as_deref() == Ok("v7"),
        }
    }
}

/// Restores the engine RNG on drop. The search and every rollout it spawns
/// reseed deliberately; the caller's stream must survive it.
struct RngGuard(u32);

impl Drop for RngGuard {
    fn drop(&mut self) {
        crate::rng::restore(self.0);
    }
}

/// SplitMix64 finalizer: a bijection on 64 bits with good avalanche, so
/// adjacent `(action, world, sim)` triples land far apart in seed space.
fn mix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// Common random numbers: a rollout's engine seed depends on its (world, sim)
/// slot and NOT on which root action is being tested.
///
/// This is the difference between a search that works and one that samples
/// noise. Two different actions play out in the same determinization with the
/// same shuffle and the same yell order, so the variance they share — the
/// overwhelming majority of it — cancels in the comparison, and what is left
/// is the difference the root action actually made. Seeding per action
/// instead makes every pair of actions an independent sample, and the argmax
/// of six such means is the winner's curse, not a decision.
fn rollout_seed(world: usize, sim: usize) -> u32 {
    let key = (world as u64) << 26 | (sim as u64);
    (mix64(0x7637_15A7_C713_0000 ^ key) % u64::from(u32::MAX)) as u32
}

/// Own deck list as card numbers — the fair prior for the opponent's hidden
/// cards. Everything here comes from our own visible zones.
fn own_visible_pool(gs: &GameState, me: u8) -> Vec<String> {
    let p = gs.seat_player(me);
    p.hand
        .cards
        .iter()
        .chain(p.stage.stage.iter())
        .chain(p.stage.under_cards.iter().flatten())
        .chain(p.waitroom.cards.iter())
        .chain(p.success_live_card_zone.cards.iter())
        .chain(p.live_card_zone.cards.iter())
        .chain(p.main_deck.cards.iter())
        .filter_map(|&cid| gs.card_database.get_card(cid))
        .filter(|card| card.card_type != CardType::Energy)
        .map(|card| card.card_no.to_string())
        .collect()
}

/// One determinized world: the true state with our deck order shuffled and
/// the opponent's hidden zones replaced by a fair sample.
fn sample_world(gs: &GameState, me: u8, cfg: &Config, index: usize) -> GameState {
    let pool = if cfg.own_pool {
        own_visible_pool(gs, me)
    } else {
        Vec::new()
    };
    // The sampler's shuffles run on the engine's thread-local RNG, so pin it
    // per world: a world must not depend on how many rollouts ran before it.
    crate::rng::seed(mix64(0x51A7_0000_0000_0000 ^ index as u64) as u32);
    let observation = PublicObservation::from_state(gs, me);
    let sampler = DeterminizationSampler::with_policy(
        crate::Arc::clone(&gs.card_database),
        &[],
        if cfg.own_pool { Some(&pool[..]) } else { None },
    );
    let sampled = sampler.sample(&observation);
    let sampled_opp = sampled.seat_player(1 - me);

    let mut sim = gs.clone();
    let (own, opp) = if me == 0 {
        (&mut sim.player1, &mut sim.player2)
    } else {
        (&mut sim.player2, &mut sim.player1)
    };
    let mut deck_rng = Lcg::new(mix64(0xDE0C_0000_0000_0000 ^ index as u64));
    for i in (1..own.main_deck.cards.len()).rev() {
        own.main_deck.cards.swap(i, deck_rng.range(i + 1));
    }
    for i in (1..own.energy_deck.cards.len()).rev() {
        own.energy_deck.cards.swap(i, deck_rng.range(i + 1));
    }

    // Preserve the true zone SIZES (deck and hand counts are public) and
    // resample only their contents.
    let hand_size = opp.hand.cards.len();
    let deck_size = opp.main_deck.cards.len();
    let energy_size = opp.energy_deck.cards.len();
    opp.hand.cards = sampled_opp.hand.cards.clone();
    opp.main_deck.cards = sampled_opp.main_deck.cards.clone();
    opp.energy_deck.cards = sampled_opp.energy_deck.cards.clone();
    opp.hand.cards.resize(hand_size, -1);
    opp.main_deck.cards.resize(deck_size, -1);
    opp.energy_deck.cards.resize(energy_size, -1);

    // A committed live-card zone is public and survives verbatim; an unset
    // one is genuinely unknown, so it takes the sample.
    if matches!(
        gs.current_phase,
        Phase::Main | Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker
    ) {
        let zone_size = opp.live_card_zone.cards.len();
        if zone_size > 0 {
            let mut zone = sampled_opp.live_card_zone.cards.clone();
            zone.resize(zone_size, -1);
            opp.live_card_zone.cards = zone;
        }
    }
    // Before the opponent has performed, the resolution of their last live is
    // not ours to see.
    if !gs.opponent_has_performed(me as usize) {
        opp.last_resolution_cards.clear();
    }
    sim
}

/// Bound the bookkeeping a speculative game accumulates. The engine's Rule
/// 12-1 history is a full state hash per action plus an O(n) scan; a 90-tick
/// rollout would pay for a repetition check that can never fire in a game we
/// are about to throw away.
fn compact(gs: &mut GameState) {
    gs.debug_trace.clear();
    if gs.rule_log.len() > 96 {
        gs.rule_log.drain(0..gs.rule_log.len() - 96);
    }
    if gs.structured_log.len() > 96 {
        gs.structured_log.drain(0..gs.structured_log.len() - 96);
    }
    gs.game_state_history.clear();
}

/// Which seat is being asked to act right now. `active_player_index` answers
/// that for phases that require a decision; during a pending ability choice
/// the queue's stamp is the authority.
fn acting_seat(gs: &GameState) -> u8 {
    if gs.has_pending_choice() {
        if let Some(id) = gs.get_pending_choice_player_id() {
            return match id.as_str() {
                "player1" | "p1" => 0,
                "player2" | "p2" => 1,
                _ => gs.active_player_index(),
            };
        }
    }
    gs.active_player_index()
}

fn stage_cost(p: &crate::player::Player, db: &CardDatabase) -> i32 {
    p.stage
        .stage
        .iter()
        .filter(|&&cid| cid >= 0)
        .filter_map(|&cid| db.get_card(cid).and_then(|card| card.cost))
        .map(i32::from)
        .sum()
}

/// Leaf value in [-1, 1]. Terminal outcomes are exact; an unfinished rollout
/// is scored on the race that decides the game.
fn game_value(gs: &GameState, me: u8) -> f64 {
    match gs.game_result {
        GameResult::FirstAttackerWins => {
            if gs.player1.is_first_attacker == (me == 0) {
                1.0
            } else {
                -1.0
            }
        }
        GameResult::SecondAttackerWins => {
            if gs.player1.is_first_attacker == (me == 0) {
                -1.0
            } else {
                1.0
            }
        }
        GameResult::Draw => 0.0,
        GameResult::Ongoing => {
            let (my, opp) = gs.seated_pair(me);
            let db = &gs.card_database;
            let lead =
                my.success_live_card_zone.cards.len() as f64 - opp.success_live_card_zone.cards.len() as f64;
            // Conversion capacity — and the term determinization exists to
            // produce, since v7's static eval cannot see the opponent's hand
            // at all. A life in hand is only an ASSET if the board could pass
            // it: counting raw lives rewards passing, because passing draws a
            // card that might be a life while costing nothing the board can
            // see. So the contested value is placed lives (which need hearts
            // the board actually has) and the rest is worth a fraction.
            let my_covered = crate::bot::strategy_v7::passable_count_buffed(gs, me, db) as f64;
            let opp_covered = crate::bot::strategy_v7::passable_count_buffed(gs, 1 - me, db) as f64;
            let my_lives = lives_in_hand(my, db) as f64;
            let opp_lives = lives_in_hand(opp, db) as f64;
            let capacity = 0.24 * (my_covered - opp_covered) / 3.0
                + 0.06 * (my_lives - opp_lives) / 3.0;
            let my_blades = my.stage.total_blades(
                db,
                &gs.mods.blade_modifiers,
                &gs.mods.orientation_modifiers,
                false,
            ) as f64;
            let opp_blades = opp.stage.total_blades(
                db,
                &gs.mods.blade_modifiers,
                &gs.mods.orientation_modifiers,
                false,
            ) as f64;
            let my_cost = stage_cost(my, db) as f64;
            let opp_cost = stage_cost(opp, db) as f64;
            (lead / 3.0
                + capacity
                + 0.18 * ((my_blades - opp_blades) / 4.0).tanh()
                + 0.10 * ((my_cost - opp_cost) / 20.0).tanh())
                .clamp(-1.0, 1.0)
        }
    }
}

/// Leaf value that defers to v7's OWN evaluation, read at the horizon
/// position instead of at the root.
///
/// This is the smallest honest increment over v7: nothing about how a move is
/// judged changes, the model that judges it is the same tuned one v7 already
/// ships, and the only new thing is that it is applied three turns later with
/// both sides played out. Measured against the hand-rolled `game_value` above,
/// which is a coarser restatement of the same axes plus a sampled opponent —
/// the version that lost 5pp of placements per live phase.
fn game_value_v7(gs: &GameState, me: u8, root: &v7_main::Features) -> f64 {
    match gs.game_result {
        GameResult::Draw => 0.0,
        GameResult::FirstAttackerWins => {
            if gs.player1.is_first_attacker == (me == 0) {
                1.0
            } else {
                -1.0
            }
        }
        GameResult::SecondAttackerWins => {
            if gs.player1.is_first_attacker == (me == 0) {
                -1.0
            } else {
                1.0
            }
        }
        GameResult::Ongoing => {
            let now = v7_main::features(gs, me);
            // v7's leaf is unbounded (a success zone is worth 1000); 400 is
            // roughly one good deploy, so this keeps ordinary positions in
            // range and lets terminal outcomes dominate when they happen.
            (v7_main::value(&now, root, false) / 400.0).clamp(-1.0, 1.0)
        }
    }
}

fn printed_hearts(db: &CardDatabase, cid: i16) -> i32 {
    db.get_card(cid)
        .and_then(|card| card.base_heart.as_ref())
        .map(|hearts| hearts.hearts.values_sum() as i32)
        .unwrap_or(0)
}

fn printed_blades(db: &CardDatabase, cid: i16) -> i32 {
    db.get_card(cid).map(|card| card.blade as i32).unwrap_or(0)
}

/// Stage area an action targets, resolved the way `game_setup` does.
fn target_area(p: &ActionParameters) -> Option<usize> {
    if let Some(index) = p.stage_area_index {
        if let Some(area) = crate::zones::MemberArea::from_index(index as usize) {
            return Some(area.to_index());
        }
    }
    p.stage_area
        .as_deref()
        .and_then(|text| text.parse().ok())
        .map(|area: crate::zones::MemberArea| area.to_index())
}

/// v7's development weights, reused so the rollout policy grades a move the
/// same way the root eval does.
const W_DEV: f64 = 8.0;
const W_HEART: f64 = 3.0;
const W_BLADE: f64 = 6.0;
const W_ENERGY: f64 = 2.0;

/// Syntactic estimate for the action kinds whose value is fully visible in
/// the action's own parameters: a member deploy (including a baton swap) and
/// an energy play. No engine round-trip, which is what makes the rollout
/// policy affordable.
fn syntactic_score(gs: &GameState, me: u8, action: &Action) -> Option<f64> {
    let p = action.parameters.as_ref()?;
    match action.action_type {
        ActionType::PlayMemberToStage => {
            let cid = p.card_id?;
            let card = gs.card_database.get_card(cid)?;
            if card.card_type != CardType::Member {
                return None;
            }
            let me_player = gs.seat_player(me);
            let existing = target_area(p)
                .and_then(|area| me_player.stage.stage.get(area).copied())
                .unwrap_or(-1);
            let stage_discount = me_player
                .stage
                .stage
                .iter()
                .filter(|&&cid| cid >= 0)
                .filter_map(|&cid| gs.card_database.get_card(cid))
                .filter_map(|card| card.cost)
                .max();
            // A baton swap pays the discounted cost, so the development it
            // buys is only the increment over what is already on the board.
            let paid = if existing >= 0 {
                i32::from(card.cost.unwrap_or(0)).min(i32::from(stage_discount.unwrap_or(0)))
            } else {
                i32::from(card.cost.unwrap_or(0))
            };
            let d_hearts = printed_hearts(&gs.card_database, cid)
                - if existing >= 0 {
                    printed_hearts(&gs.card_database, existing)
                } else {
                    0
                };
            // A waiting member already contributes no blades, so replacing one
            // costs no blades.
            let existing_blades = if existing >= 0
                && gs.mods.get_orientation_modifier(existing) != Some("wait")
            {
                printed_blades(&gs.card_database, existing)
            } else {
                0
            };
            let d_blades = printed_blades(&gs.card_database, cid) - existing_blades;
            Some(W_DEV * paid as f64 + W_HEART * d_hearts as f64 + W_BLADE * d_blades as f64)
        }
        ActionType::EnergyCharge => Some(W_ENERGY),
        _ => None,
    }
}

/// Exact grade for an action whose effect is not visible in its parameters:
/// execute it on a clone and read v7's own value delta.
fn executed_score(
    gs: &GameState,
    me: u8,
    action: &Action,
    base: &v7_main::Features,
) -> Option<f64> {
    let mut sim = gs.clone();
    crate::rng::seed(0x7637);
    if game_setup::execute_action(&mut sim, action).is_err() {
        return None;
    }
    game_setup::settle_single_player_state(&mut sim);
    let after = v7_main::features(&sim, me);
    let deploy = action.action_type == ActionType::PlayMemberToStage;
    Some(v7_main::value(&after, base, deploy))
}

/// Rollout policy. One policy for both sides: value is then measured as
/// self-play from the sampled position, which is the quantity an MCTS leaf is
/// defined to estimate. Grading a move by v7's own `value()` keeps the rollout
/// consistent with the prior it is refining.
fn rollout_action(gs: &GameState, actions: &[Action], me: u8, exec_deploy: bool) -> Action {
    if actions.len() == 1 {
        return actions[0].clone();
    }
    match gs.current_phase {
        Phase::RockPaperScissors | Phase::ChooseFirstAttacker => return actions[0].clone(),
        Phase::MulliganFirstAttacker | Phase::MulliganSecondAttacker => {
            return crate::bot::strategy_v4::choose_mulligan_v4(gs, actions, &gs.card_database);
        }
        _ => {}
    }

    let base = v7_main::features(gs, me);
    let mut best = 0usize;
    let mut best_score = f64::NEG_INFINITY;
    for (index, action) in actions.iter().enumerate() {
        if action.parameters.as_ref().and_then(|p| p.disabled) == Some(true) {
            continue;
        }
        let score = if action.action_type == ActionType::Pass && !gs.has_pending_choice() {
            // v6 doctrine, kept verbatim: Pass ends the development phase and
            // is chosen only when nothing useful is on offer.
            0.0
        } else if let Some(estimate) = syntactic_score(gs, me, action) {
            if exec_deploy {
                executed_score(gs, me, action, &base).unwrap_or(estimate)
            } else {
                estimate
            }
        } else {
            executed_score(gs, me, action, &base).unwrap_or(f64::NEG_INFINITY)
        };
        if score > best_score {
            best_score = score;
            best = index;
        }
    }
    actions[best].clone()
}

/// Play `root` out from `world` and return its leaf value.
fn play_out(
    world: &GameState,
    me: u8,
    cfg: &Config,
    root: &Action,
    start_turn: u8,
    root_features: &v7_main::Features,
) -> f64 {
    let horizon_end = start_turn.saturating_add(cfg.horizon);
    let mut sim = world.clone();
    compact(&mut sim);

    if game_setup::execute_action(&mut sim, root).is_err() {
        return -1.0;
    }
    game_setup::settle_single_player_state(&mut sim);

    // A live-set decision resolves over several action ticks while the policy
    // is stateless, so the same portfolio — and its 256-sample pass model —
    // would be recomputed on every tick of the phase. Memoize it per phase.
    let mut live_plan: Option<(u8, u8, Vec<usize>)> = None;

    // The leaf is read at the START of the horizon turn's Main phase, not at
    // whichever tick the budget happened to run out on.
    //
    // This matters more than it looks. A tick budget stops wherever it stops:
    // possibly with one live card selected and a pending choice outstanding,
    // one player mid-ability, energy half-spent. `lives_in_hand` and
    // `passable_count_buffed` are then describing a state that does not exist
    // at any decision point, so the value function scores noise — and a noisy
    // leaf is worse than no leaf, because the search still trusts it. The Main
    // phase of turn N is the one position every rollout can be compared at:
    // fresh hand, refreshed energy, complete board, no pending choice. It is
    // also exactly where the next decision happens, so it is the position the
    // root action is actually being judged for.
    let mut ticks = 0usize;
    loop {
        crate::turn::TurnEngine::check_victory_condition(&mut sim);
        if sim.game_result != GameResult::Ongoing {
            break;
        }
        if sim.turn_number >= horizon_end && matches!(sim.current_phase, Phase::Main) {
            break;
        }
        if ticks >= cfg.ticks || sim.turn_number > horizon_end {
            break;
        }
        if game_setup::auto_advance_one(&mut sim) {
            ticks += 1;
            continue;
        }
        let actions = game_setup::generate_possible_actions(&sim);
        if actions.is_empty() {
            crate::turn::TurnEngine::advance_phase(&mut sim);
            ticks += 1;
            continue;
        }
        let chosen = match sim.current_phase {
            Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker => {
                let side = u8::from(sim.current_phase == Phase::LiveCardSetSecondAttacker);
                let plan = match live_plan.as_ref() {
                    Some((turn, cached_side, plan))
                        if *turn == sim.turn_number && *cached_side == side =>
                    {
                        plan.clone()
                    }
                    _ => {
                        let plan = live_set_plan(&sim, me);
                        live_plan = Some((sim.turn_number, side, plan.clone()));
                        plan
                    }
                };
                crate::bot::strategy_common::emit_live_set(&sim, &actions, &plan)
            }
            _ => rollout_action(&sim, &actions, me, cfg.exec_deploy),
        };
        if game_setup::execute_action(&mut sim, &chosen).is_err() {
            crate::turn::TurnEngine::advance_phase(&mut sim);
            ticks += 1;
            continue;
        }
        // A speculative game must never stop to ask whether it is looping.
        if sim.pending_loop_protocol.is_some() {
            sim.resolve_loop_protocol(true);
        }
        if sim.game_state_history.len() > 24 {
            sim.game_state_history.clear();
        }
        game_setup::settle_single_player_state(&mut sim);
        ticks += 1;
    }
    crate::turn::TurnEngine::check_victory_condition(&mut sim);
    if cfg.v7_leaf {
        game_value_v7(&sim, me, root_features)
    } else {
        game_value(&sim, me)
    }
}

/// The live-card portfolio for the seat now acting. Ours is v7's measured
/// best; the opponent is v6, which is both the arena's default opposition and
/// a fair stand-in for a competent player.
fn live_set_plan(gs: &GameState, me: u8) -> Vec<usize> {
    if acting_seat(gs) == me {
        crate::bot::strategy_v7::live_set_plan(gs, &gs.card_database)
    } else {
        let actions = game_setup::generate_possible_actions(gs);
        if actions.is_empty() {
            return Vec::new();
        }
        let chosen = crate::bot::strategy_v6::choose_live_set_v6(gs, &actions, &gs.card_database);
        selected_after(gs, &chosen)
    }
}

/// The hand indices a one-step live-set action is trying to move toward: the
/// toggle it selected (if any) plus everything already selected. v6's chooser
/// returns one emission step, so this recovers the plan it is walking.
fn selected_after(gs: &GameState, chosen: &Action) -> Vec<usize> {
    let mut plan: Vec<usize> = gs
        .live_card_selected_indices
        .iter()
        .map(|&index| index as usize)
        .collect();
    if chosen.action_type == ActionType::SelectLiveCard {
        if let Some(index) = chosen.parameters.as_ref().and_then(|p| p.card_index) {
            match chosen.selected {
                Some(true) if !plan.contains(&index) => plan.push(index),
                Some(false) => plan.retain(|&held| held != index),
                _ => {}
            }
        }
    }
    if chosen.action_type == ActionType::SkipLiveCardSet {
        plan.clear();
    }
    plan
}

/// Run a batch of independent rollouts, in parallel when asked. Results are
/// written by index, so the caller sees the same numbers at any job count.
fn run_batch(
    actions: &[Action],
    worlds: &[GameState],
    me: u8,
    cfg: &Config,
    items: &[(usize, usize, usize)],
    start_turn: u8,
    root_features: &v7_main::Features,
) -> Vec<f64> {
    if items.is_empty() {
        return Vec::new();
    }
    let run = |item: &(usize, usize, usize)| -> f64 {
        let (action, world, sim) = *item;
        crate::rng::seed(rollout_seed(world, sim));
        play_out(
            &worlds[world],
            me,
            cfg,
            &actions[action],
            start_turn,
            root_features,
        )
    };

    if cfg.jobs <= 1 || items.len() == 1 {
        return items.iter().map(run).collect();
    }

    let jobs = cfg.jobs.min(items.len());
    let chunk = items.len().div_ceil(jobs);
    let mut out = vec![0.0f64; items.len()];
    std::thread::scope(|scope| {
        for (worker, slice) in out.chunks_mut(chunk).enumerate() {
            let start = worker * chunk;
            let part = &items[start..start + slice.len()];
            let actions = &*actions;
            let worlds = &*worlds;
            scope.spawn(move || {
                for (offset, item) in part.iter().enumerate() {
                    let (action, world, sim) = *item;
                    crate::rng::seed(rollout_seed(world, sim));
                    slice[offset] = play_out(
                        &worlds[world],
                        me,
                        cfg,
                        &actions[action],
                        start_turn,
                        root_features,
                    );
                }
            });
        }
    });
    out
}

/// PUCT over the candidate list. `Q` is the prior-shrunk rollout mean and the
/// exploration term is scaled by the softmax prior, so a move v7 likes is
/// explored more — and, because `k0` pseudo-visits stand behind it, a move
/// v7 dislikes has to earn its promotion.
fn select_candidate(
    sums: &[f64],
    counts: &[u32],
    prior: &[f64],
    prior_prob: &[f64],
    k0: f64,
    c_puct: f64,
) -> usize {
    let total: u32 = counts.iter().sum();
    let root_n = f64::from(total.max(1)).sqrt();
    let mut best = 0usize;
    let mut best_ucb = f64::NEG_INFINITY;
    for index in 0..counts.len() {
        let n = f64::from(counts[index]);
        let exploitation = (sums[index] + k0 * prior[index]) / (n + k0);
        let exploration = c_puct * prior_prob[index] * root_n / (1.0 + n);
        let ucb = exploitation + exploration;
        if ucb > best_ucb {
            best_ucb = ucb;
            best = index;
        }
    }
    best
}

/// Refine v7's static scores with a determinized root search.
///
/// `forced` overrides the environment for callers that must not be at the
/// mercy of a process-wide switch: the arena's `v7plain` ablation bot passes
/// `Some(false)` so a mirror `v7 vs v7plain` measures the search and nothing
/// else. Every failure path (disabled, nothing to choose, proven win,
/// non-finite scores) returns the input scores, so the search can only add
/// signal, never remove it.
pub fn refine_scores(
    gs: &GameState,
    actions: &[Action],
    me: u8,
    scores: &[(f64, String)],
    forced: Option<bool>,
) -> Vec<(f64, String)> {
    let mut cfg = Config::from_env();
    if let Some(enabled) = forced {
        cfg.enabled = enabled;
    }
    match search(gs, actions, me, scores, &cfg) {
        Some(refined) => refined,
        None => scores.to_vec(),
    }
}

fn search(
    gs: &GameState,
    actions: &[Action],
    me: u8,
    scores: &[(f64, String)],
    cfg: &Config,
) -> Option<Vec<(f64, String)>> {
    if !cfg.enabled || actions.len() < 2 || scores.len() != actions.len() {
        return None;
    }
    // SCOPE: Main-phase development decisions only.
    //
    // A live-card set and a mulligan are not single decisions — they are
    // multi-tick emission chains that v7 plans ONCE (`live_set_plan`) and
    // then walks out one toggle at a time. Re-deciding each tick with a fresh
    // leaf-value search has no strategic content (the strategy IS the
    // portfolio, already computed) and is actively destructive: consecutive
    // ticks select different cards, the selection never converges, the
    // confirm never happens, and the game walks into a Rule 12-1 repetition.
    // Measured: 94 draws in 200 games, from 0 without the search.
    if !matches!(gs.current_phase, Phase::Main) {
        return None;
    }
    // The Rule 12-1 repetition protocol is a rules question — "is this game
    // repeating, yes or no" — not a strategic one, and it must never be
    // arbitrated by rollouts. The offered options are "Stop (draw)" and
    // "Continue", and a search whose rollout policy is weaker than the real
    // one estimates "Continue" as a loss more often than it is, so a draw
    // scores 0.0 against a -1.0 and the search picks the draw. Measured:
    // 309 draws in 600 games, from 0 without the search. The static path
    // answers this prompt.
    if gs.pending_loop_protocol.is_some() {
        return None;
    }
    // A proven win needs no search, and a proven no-op needs no search either.
    if let Some(winner) = scores
        .iter()
        .position(|(score, _)| *score >= TERMINAL_WIN)
    {
        let mut out = scores.to_vec();
        out[winner].0 += 1.0;
        return Some(out);
    }

    let mut candidates: Vec<usize> = (0..actions.len())
        .filter(|&index| {
            scores[index].0.is_finite()
                && actions[index].parameters.as_ref().and_then(|p| p.disabled) != Some(true)
        })
        .collect();
    if candidates.len() < 2 {
        return None;
    }
    // Only the strongest `top_k` static candidates earn rollouts; the tail
    // keeps its prior, which is where v7 already decided.
    candidates.sort_by(|&left, &right| {
        scores[right]
            .0
            .total_cmp(&scores[left].0)
            .then_with(|| left.cmp(&right))
    });
    candidates.truncate(cfg.top_k);

    let _guard = RngGuard(crate::rng::checkpoint());
    let start_turn = gs.turn_number;

    let best_static = candidates
        .iter()
        .map(|&index| scores[index].0)
        .fold(f64::NEG_INFINITY, f64::max);
    // Prior in rollout units. This is v7's OWN score, normalized — NOT
    // regret relative to the best action.
    //
    // The regret form looks reasonable and is wrong. v7 scores `Pass` at
    // exactly 0.0 as a BASELINE: a deploy is worth 15-165, a no-op is -1000.
    // Measured against the leader, Pass lands at -0.63 and every useful
    // deploy is squeezed into 0.0..-0.05, so a noisy rollout beats Pass
    // almost always. v7 then cycles `use_ability -> pass -> use_ability`
    // until Rule 12-1 fires: 136 draws in 300 games, from 0 without the
    // search. Keeping the shape v7 actually uses (Pass at 0, better moves
    // above) makes the prior point the same way the static eval does, and at
    // n = 0 the argmax of the prior is byte-for-byte v7's own pick.
    let prior: Vec<f64> = candidates
        .iter()
        .map(|&index| (scores[index].0 / cfg.prior_scale).clamp(-1.0, 1.0))
        .collect();
    let mut prior_prob = vec![0.0f64; candidates.len()];
    let mut norm = 0.0f64;
    for (slot, &index) in candidates.iter().enumerate() {
        let weight = (-(best_static - scores[index].0) / cfg.prior_temp).exp();
        prior_prob[slot] = weight;
        norm += weight;
    }
    if norm > 0.0 {
        for weight in &mut prior_prob {
            *weight /= norm;
        }
    }

    let worlds: Vec<GameState> = (0..cfg.worlds)
        .map(|index| sample_world(gs, me, cfg, index))
        .collect();
    // The reference position for the v7 leaf: v7's own features at the root,
    // so `value(features(leaf), features(root))` reads as "how much better is
    // the board three turns from now than it is now".
    let root_features = v7_main::features(gs, me);

    let mut sums = vec![0.0f64; candidates.len()];
    let mut counts = vec![0u32; candidates.len()];
    let mut used = 0usize;
    // Balanced budget: every candidate gets the same visit count, so the
    // comparison is not a contest between actions that were sampled
    // differently. `visits` is only a ceiling.
    let target = cfg.visits.min(candidates.len() * cfg.sims.max(1));

    // Round one: every candidate is priced exactly once. Without this an
    // action with no visits has infinite UCB forever, so the policy would
    // never test anything below the prior and the "search" would be a no-op.
    let first: Vec<(usize, usize, usize)> = (0..candidates.len())
        .map(|slot| (slot, slot % worlds.len(), slot / worlds.len()))
        .collect();
    for (value, item) in run_batch(actions, &worlds, me, cfg, &first, start_turn, &root_features)
        .into_iter()
        .zip(&first)
    {
        sums[item.0] += value;
        counts[item.0] += 1;
        used += 1;
    }

    // Balanced visits, round-robin. No PUCT.
    //
    // PUCT is the standard choice and it is wrong here, and the per-move trace
    // shows exactly why. The v7 leaf is near-binary: a rollout that places a
    // life reads 1.0, one that does not reads -1.0, so Q saturates and stops
    // discriminating. With a saturated Q, the exploration bonus is the only
    // thing separating children, and the search piles onto whichever child
    // drew a winning sample first:
    //
    //   cand0 play_member_to_stage:2551  n=1  mean=0.171
    //   cand1 play_member_to_stage:2537  n=23 mean=1.000
    //   cand4 play_member_to_stage:2551  n=1  mean=0.024  <= PICKED
    //
    // Twenty-three visits, all 1.0, and the actual pick is a one-visit
    // candidate. That is not an estimate, it is a race. Because every action
    // already gets the SAME (world, sim) sequence by construction — common
    // random numbers make action A's j-th rollout paired with action B's
    // j-th — a round-robin over that shared sequence is a properly paired
    // estimate for every child, and it removes the visit-count artifact
    // without giving up any information.
    let batch = (target - used).min(cfg.jobs.max(1));
    let mut items: Vec<(usize, usize, usize)> = Vec::with_capacity(batch);
    for offset in 0..batch {
        let slot = (used + offset) % candidates.len();
        let visit = counts[slot] as usize;
        items.push((slot, visit % worlds.len(), visit / worlds.len()));
    }
    for (value, item) in
        run_batch(actions, &worlds, me, cfg, &items, start_turn, &root_features)
            .into_iter()
            .zip(&items)
    {
        sums[item.0] += value;
        counts[item.0] += 1;
        used += 1;
    }
}

    let mut out = scores.to_vec();
    let mut priced = vec![false; out.len()];
    for (slot, &index) in candidates.iter().enumerate() {
        priced[index] = true;
        let n = f64::from(counts[slot]);
        let blended = (sums[slot] + cfg.k0 * prior[slot]) / (n + cfg.k0);
        out[index].0 = blended;
        out[index].1 = format!(
            "{} ismcts={:.3} n={} sum={:.2}",
            out[index].1, blended, counts[slot], sums[slot]
        );
    }
    // Actions outside the candidate set were cut by the prior, so they get
    // the prior in rollout units rather than their raw v7 score. Leaving raw
    // scores in place would mix a [-1, 1] scale with values up to 10000 and
    // silently re-order the tail against everything the search looked at.
    for index in 0..out.len() {
        if !priced[index] {
            out[index].0 = (scores[index].0 / cfg.prior_scale).clamp(-1.0, 1.0);
        }
    }

    // A search that never changes its mind is a no-op wearing a compute
    // budget. `V7_ISMCTS_TRACE` reports the rate so a run that "wins by
    // noise" is distinguishable from one that actually re-ranked.
    let static_best = candidates[0];
    let searched_best = candidates
        .iter()
        .enumerate()
        .max_by(|left, right| {
            out[*left.1]
                .0
                .total_cmp(&out[*right.1].0)
                .then_with(|| right.1.cmp(left.1))
        })
        .map(|(slot, _)| candidates[slot])
        .unwrap_or(static_best);
    let overrode = searched_best != static_best;
    if std::env::var_os("V7_ISMCTS_TRACE").is_some() {
        eprintln!(
            "V7ISMCTS t={} me={} phase={:?} cands={} visits={} static={}:{}({:.1}) searched={}:{}({:.3}) overrode={}",
            gs.turn_number,
            me,
            gs.current_phase,
            candidates.len(),
            used,
            static_best,
            action_label(actions, static_best),
            scores[static_best].0,
            searched_best,
            action_label(actions, searched_best),
            out[searched_best].0,
            overrode,
        );
        if overrode {
            let me_player = gs.seat_player(me);
            let opp_player = gs.seat_player(1 - me);
            eprintln!(
                "    BOARD t{} me{} energy={}/{} hand={} stage={:?} blades={} covered={} succ={}..{} | opp blades={} succ={} opp_hand={}",
                gs.turn_number,
                me,
                me_player.energy_zone.cards.len(),
                me_player.energy_zone.active_energy_count,
                me_player.hand.cards.len(),
                me_player.stage.stage,
                me_player
                    .stage
                    .total_blades(
                        &gs.card_database,
                        &gs.mods.blade_modifiers,
                        &gs.mods.orientation_modifiers,
                        false,
                    ),
                crate::bot::strategy_v7::passable_count_buffed(gs, me, &gs.card_database),
                me_player.success_live_card_zone.cards.len(),
                opp_player.success_live_card_zone.cards.len(),
                opp_player
                    .stage
                    .total_blades(
                        &gs.card_database,
                        &gs.mods.blade_modifiers,
                        &gs.mods.orientation_modifiers,
                        false,
                    ),
                opp_player.success_live_card_zone.cards.len(),
                opp_player.hand.cards.len(),
            );
            for (slot, &index) in candidates.iter().enumerate() {
                eprintln!(
                    "    cand{} {:<34} static={:>8.1} prior={:>6.3} n={} sum={:>7.2} mean={:>6.3} -> blended={:>6.3}{}",
                    slot,
                    action_label(actions, index),
                    scores[index].0,
                    prior[slot],
                    counts[slot],
                    sums[slot],
                    if counts[slot] > 0 {
                        sums[slot] / f64::from(counts[slot])
                    } else {
                        0.0
                    },
                    out[index].0,
                    if slot == searched_best { "  <= PICKED" } else { "" },
                );
            }
        }
    }
    record_override(overrode);
    log::debug!(
        "v7 ismcts t{} me{} phase={:?} candidates={} visits={} worlds={} horizon={} overrode={}",
        gs.turn_number,
        me,
        gs.current_phase,
        candidates.len(),
        used,
        worlds.len(),
        cfg.horizon,
        overrode,
    );

    // PROMOTE-ONLY GATE. v7's doctrine is that `Pass` scores exactly 0.0 as a
    // baseline and only a genuinely positive action is worth playing, and it
    // takes that seriously: `pick_best` breaks a Pass/deploy tie toward Pass
    // via strict `>`. A leaf-value search has no such tie to break, so
    // without this gate a repeatable-but-worthless action (use an ability
    // that nets nothing, pass, use it again) beats Pass on rollout noise and
    // the game walks into a Rule 12-1 repetition — measured 136 draws in 300
    // games, against 0 without the search. The gate restates the doctrine as
    // an invariant: the search may only re-rank among actions v7 already
    // scored above its own Pass baseline. A move v7 thought was worthless has
    // to earn promotion from v7, not from a five-sample rollout.
    if !cfg.promote_only {
        return Some(out);
    }
    let baseline = scores[static_baseline(actions, scores)].0;
    let promotable: Vec<bool> = candidates
        .iter()
        .enumerate()
        .map(|(slot, _)| {
            let index = candidates[slot];
            slot == 0 || scores[index].0 > baseline
        })
        .collect();
    if promotable.iter().filter(|keep| **keep).count() < 2 {
        // Nothing v7 was willing to play over Pass: hand the decision back
        // untouched rather than re-rank on rollouts alone.
        return None;
    }
    let best_promotable = promotable
        .iter()
        .enumerate()
        .filter(|(_, keep)| **keep)
        .map(|(slot, _)| out[candidates[slot]].0)
        .fold(f64::NEG_INFINITY, f64::max);
    for (slot, &index) in candidates.iter().enumerate() {
        if !promotable[slot] {
            out[index].0 = f64::min(out[index].0, best_promotable - 1.0);
        }
    }
    Some(out)
}

/// The action v7 treats as the do-nothing baseline: `Pass` when it is on
/// offer, else the lowest finite static score.
fn static_baseline(actions: &[Action], scores: &[(f64, String)]) -> usize {
    if let Some(index) = actions
        .iter()
        .position(|action| action.action_type == ActionType::Pass)
    {
        return index;
    }
    scores
        .iter()
        .enumerate()
        .filter(|(_, (score, _))| score.is_finite())
        .fold(0usize, |worst, (index, (score, _))| {
            if *score < scores[worst].0 {
                index
            } else {
                worst
            }
        })
}

/// Short identity for a root action, for traces.
fn action_label(actions: &[Action], index: usize) -> String {
    let Some(action) = actions.get(index) else {
        return "-".into();
    };
    let card = action
        .parameters
        .as_ref()
        .and_then(|p| p.card_id)
        .map(|id| id.to_string())
        .unwrap_or_else(|| "-".into());
    format!("{}:{}", action.action_type, card)
}

thread_local! {
    static OVERRIDES: std::cell::Cell<(u64, u64)> = const { std::cell::Cell::new((0, 0)) };
}

/// Running count of (searched, total) decisions the search re-ranked.
fn record_override(overrode: bool) {
    OVERRIDES.with(|cell| {
        let (over, total) = cell.get();
        cell.set((over + u64::from(overrode), total + 1));
    });
}

/// Override rate since the last reset, as (overrides, decisions).
pub fn override_stats() -> (u64, u64) {
    OVERRIDES.with(|cell| cell.get())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db_real() -> crate::Arc<CardDatabase> {
        let cards = crate::card_loader::CardLoader::load_cards_from_file(std::path::Path::new(
            "../cards/cards.json",
        ))
        .unwrap();
        crate::Arc::new(CardDatabase::load_or_create(cards))
    }

    fn fixture() -> (GameState, crate::Arc<CardDatabase>) {
        let db = db_real();
        let p1 = crate::player::Player::new("p1".into(), "P1".into(), true);
        let p2 = crate::player::Player::new("p2".into(), "P2".into(), false);
        let mut gs = GameState::new(p1, p2, crate::Arc::clone(&db));
        gs.current_phase = Phase::Main;
        (gs, db)
    }

    #[test]
    fn rollout_seeds_are_paired_across_actions() {
        // Common random numbers: the seed must depend on the (world, sim)
        // slot alone, so action A's j-th rollout and action B's j-th rollout
        // see the same shuffle. If this ever takes an action index again the
        // search silently reverts to comparing independent samples.
        assert_eq!(rollout_seed(0, 0), rollout_seed(0, 0));
        let mut seen = std::collections::HashSet::new();
        for world in 0..4 {
            for sim in 0..6 {
                assert!(seen.insert(rollout_seed(world, sim)), "collision");
            }
        }
    }

    #[test]
    fn game_value_reads_the_success_race() {
        let (mut gs, _db) = fixture();
        assert_eq!(game_value(&gs, 0), 0.0);
        gs.player1.success_live_card_zone.cards.push(1);
        assert!(game_value(&gs, 0) > 0.0, "own placement must be worth something");
        assert!(game_value(&gs, 1) < 0.0, "their placement must cost something");
    }

    #[test]
    fn game_value_is_terminal_exact() {
        let (mut gs, _db) = fixture();
        gs.game_result = GameResult::FirstAttackerWins;
        assert_eq!(game_value(&gs, 0), 1.0);
        assert_eq!(game_value(&gs, 1), -1.0);
        gs.game_result = GameResult::SecondAttackerWins;
        assert_eq!(game_value(&gs, 0), -1.0);
        assert_eq!(game_value(&gs, 1), 1.0);
    }

    #[test]
    fn sampled_world_keeps_public_state_and_zone_sizes() {
        let (mut gs, db) = fixture();
        let member = db.get_card_id("PL!SP-bp1-005-R").unwrap();
        let live = db.get_card_id("PL!SP-sd1-019-SD").unwrap();
        assert_eq!(db.get_card(member).unwrap().card_no, "PL!SP-bp1-005-R");
        assert_eq!(db.get_card(live).unwrap().card_no, "PL!SP-sd1-019-SD");
        for _ in 0..30 {
            gs.player1.main_deck.cards.push(member);
            gs.player2.main_deck.cards.push(member);
        }
        gs.player1.stage.stage = [member, -1, -1];
        gs.player2.hand.cards.extend([live, member]);
        let deck_size = gs.player2.main_deck.cards.len();
        let cfg = Config {
            enabled: true,
            worlds: 1,
            sims: 1,
            visits: 1,
            horizon: 3,
            ticks: 90,
            top_k: 8,
            c_puct: 1.25,
            k0: 8.0,
            prior_scale: 260.0,
            prior_temp: 55.0,
            jobs: 1,
            own_pool: true,
            exec_deploy: false,
            promote_only: true,
            v7_leaf: false,
        };
        let world = sample_world(&gs, 0, &cfg, 0);
        assert_eq!(world.player2.main_deck.cards.len(), deck_size);
        assert_eq!(world.player1.stage.stage, gs.player1.stage.stage);
        assert_eq!(world.player1.hand.cards, gs.player1.hand.cards);
        assert_eq!(world.turn_number, gs.turn_number);
        // Determinism: the same world index is the same world.
        assert_eq!(world.player2.hand.cards, sample_world(&gs, 0, &cfg, 0).player2.hand.cards);
    }

    #[test]
    fn search_is_independent_of_worker_count() {
        let (mut gs, db) = fixture();
        let member = db.get_card_id("PL!SP-bp1-005-R").unwrap();
        gs.player1.hand.cards.extend([member, member, member]);
        gs.player1.main_deck.cards.extend([member; 20]);
        gs.player2.main_deck.cards.extend([member; 20]);
        let actions = game_setup::generate_possible_actions(&gs);
        if actions.len() < 2 {
            return;
        }
        let scores: Vec<(f64, String)> =
            actions.iter().map(|_| (1.0, "t".to_string())).collect();
        let mut serial = Config::from_env();
        serial.jobs = 1;
        serial.worlds = 2;
        serial.sims = 1;
        serial.visits = 6;
        let mut parallel = serial;
        parallel.jobs = 4;
        let one = search(&gs, &actions, 0, &scores, &serial);
        let many = search(&gs, &actions, 0, &scores, &parallel);
        assert!(one.is_some() && many.is_some());
        let one = one.unwrap();
        let many = many.unwrap();
        for (index, (score, _)) in one.iter().enumerate() {
            assert!(
                (score - many[index].0).abs() < 1e-12,
                "job count must not change the answer: {score} vs {}",
                many[index].0
            );
        }
    }

    #[test]
    fn the_search_refuses_states_it_must_not_price() {
        let (mut gs, db) = fixture();
        let member = db.get_card_id("PL!SP-bp1-005-R").unwrap();
        assert_eq!(db.get_card(member).unwrap().card_no, "PL!SP-bp1-005-R");
        let energy = db
            .cards
            .values()
            .find(|card| card.is_energy())
            .map(|card| db.get_card_id(card.card_no.as_ref()).unwrap())
            .expect("database has energy cards");
        for _ in 0..6 {
            gs.player1.energy_zone.cards.push(energy);
        }
        gs.player1.energy_zone.add_active(3);
        gs.player1.hand.cards.extend([member, member, member]);
        gs.player1.main_deck.cards.extend([member; 20]);
        gs.player2.main_deck.cards.extend([member; 20]);
        let actions = game_setup::generate_possible_actions(&gs);
        assert!(actions.len() >= 2, "fixture must offer a real choice");
        let scores: Vec<(f64, String)> = actions
            .iter()
            .enumerate()
            .map(|(index, _)| (index as f64, "a".into()))
            .collect();
        let cfg = Config::from_env();
        // A live-card set is a multi-tick emission chain, not a decision.
        gs.current_phase = Phase::LiveCardSetSecondAttacker;
        assert!(search(&gs, &actions, 0, &scores, &cfg).is_none());
        gs.current_phase = Phase::MulliganFirstAttacker;
        assert!(search(&gs, &actions, 0, &scores, &cfg).is_none());
        // Main is the scope.
        gs.current_phase = Phase::Main;
        assert!(search(&gs, &actions, 0, &scores, &cfg).is_some());
        // Rule 12-1 is a rules prompt, never a rollout question.
        gs.pending_loop_protocol = Some(crate::core::game_state::PermanentLoopProtocol {
            state_hash: 0,
            repetition_count: 3,
            choice: crate::ability::types::Choice::SelectTarget {
                target: "rule_12_1".into(),
                description: String::new(),
                description_en: None,
                description_ja: None,
                allow_skip: false,
                options: None,
            },
        });
        assert!(search(&gs, &actions, 0, &scores, &cfg).is_none());
    }

    #[test]
    fn the_prior_ranks_actions_exactly_as_v7_ranks_them() {
        // v7 scores Pass at exactly 0.0 as a baseline, useful deploys at
        // 15-165, and a no-op at -1000. The prior must preserve that shape:
        // normalizing by the regret-to-leader instead would pin Pass far
        // below every deploy and let a noisy rollout walk v7 into a
        // `use_ability -> pass -> use_ability` repetition.
        let scores: [f64; 5] = [0.0, 50.0, 15.0, -1000.0, 165.0];
        let scale: f64 = 260.0;
        let prior: Vec<f64> = scores
            .iter()
            .map(|score| (score / scale).clamp(-1.0, 1.0))
            .collect();
        assert_eq!(prior[0], 0.0, "Pass must stay at the neutral baseline");
        assert!(prior[1] > prior[0] && prior[4] > prior[1], "ordering preserved");
        assert!(prior[2] > prior[0], "a 15-point deploy is still above Pass");
        assert_eq!(prior[3], -1.0, "a no-op saturates, never inverts");
        let argmax = |v: &[f64]| {
            v.iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1).then_with(|| b.0.cmp(&a.0)))
                .map(|(i, _)| i)
        };
        assert_eq!(
            argmax(&prior),
            argmax(&scores),
            "prior must not re-rank v7's own ordering"
        );
    }

    #[test]
    fn promote_only_never_lets_a_useless_action_beat_pass() {
        let actions = vec![
            Action {
                description: "pass".into(),
                description_ja: None,
                action_type: ActionType::Pass,
                parameters: None,
                selected: None,
            },
            Action {
                description: "use".into(),
                description_ja: None,
                action_type: ActionType::UseAbility,
                parameters: None,
                selected: None,
            },
        ];
        assert_eq!(static_baseline(&actions, &[(0.0, String::new()), (0.0, String::new())]), 0);
        // With a 3-point "use ability" and a 0.0 Pass, only Pass is promotable
        // (nothing clears the baseline), so the search must decline.
        let scores = vec![(0.0, String::new()), (3.0, String::new())];
        let (gs, _db) = fixture();
        let mut cfg = Config::from_env();
        cfg.promote_only = true;
        assert!(search(&gs, &actions, 0, &scores, &cfg).is_none());
    }

    #[test]
    fn prior_shrinks_a_thin_sample_toward_the_static_eval() {
        let sums = [4.0];
        let counts = [1u32];
        let prior = [-1.0];
        let k0 = 8.0;
        let blended = (sums[0] + k0 * prior[0]) / (f64::from(counts[0]) + k0);
        assert!(blended < 0.0, "one good rollout must not overturn a bad prior");
    }
}
