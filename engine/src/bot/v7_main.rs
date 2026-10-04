use crate::core::constants::CountCast;
use crate::bot::determinization::DeterminizationSampler;
use crate::bot::observation::PublicObservation;
use crate::bot::strategy_common::{acc_add, narrow_hearts, requirements_met, Acc};
use crate::card::CardType;
use crate::core::stats_pipeline;
use crate::game_setup::{self, Action, ActionParameters, ActionType};
use crate::game_state::{GameResult, GameState, Phase};

/// D1: baton vision from action params, not the unset `use_baton_touch`.
/// A PlayMemberToStage targeting an occupied stage slot (or a double-baton
/// `card_indices` pair, or an `available_areas` entry flagged
/// `is_baton_touch` for the chosen `stage_area`) IS a baton touch.
/// Generator never sets `use_baton_touch` (link.rs matches on it); occupied
/// target ⇁Ebaton is the generator's own rule (game_setup.rs:1504-1541).
/// Reverse-gates for ablation: `V7_PRE_D=1` or `V7_NO_BATON=1`.
pub(crate) fn is_baton_action(gs: &GameState, me: u8, a: &Action) -> bool {
    if std::env::var_os("V7_PRE_D").is_some() || std::env::var_os("V7_NO_BATON").is_some() {
        return a.parameters.as_ref().and_then(|p| p.use_baton_touch) == Some(true);
    }
    if a.action_type != ActionType::PlayMemberToStage {
        return false;
    }
    let Some(p) = a.parameters.as_ref() else {
        return false;
    };
    if p.use_baton_touch == Some(true) {
        return true;
    }
    baton_from_params(gs, me, p)
}

pub(crate) fn baton_from_params(gs: &GameState, me: u8, p: &ActionParameters) -> bool {
    if p.card_indices.is_some() {
        return true;
    }
    let stage = p
        .stage_area_index
        .and_then(|index| crate::zones::MemberArea::from_index(index as usize))
        .or_else(|| p.stage_area.as_deref()?.parse().ok());
    let Some(stage) = stage else {
        return p
            .available_areas
            .as_ref()
            .is_some_and(|areas| areas.iter().any(|area| area.is_baton_touch));
    };
    if let Some(areas) = p.available_areas.as_ref() {
        if areas
            .iter()
            .any(|area| area.area.as_ref() == stage.as_str() && area.is_baton_touch)
        {
            return true;
        }
    }
    gs.seat_player(me)
        .stage
        .stage
        .get(stage.to_index())
        .copied()
        .unwrap_or(-1)
        != -1
}

fn board(gs: &GameState, me: u8) -> (Acc, i32, i32) {
    let p = gs.seat_player(me);
    let hearts = stats_pipeline::stage_hearts(
        &p.stage.stage,
        &gs.card_database,
        &gs.mods.heart_override,
        &gs.mods.heart_copy,
        &gs.mods.heart_color_multiplier,
        &gs.mods.heart_modifiers,
    );
    let mut pool = [0; 11];
    acc_add(&mut pool, &hearts.hearts);
    let mut blades = 0;
    let mut cost = 0;
    for &id in &p.stage.stage {
        if id < 0 {
            continue;
        }
        cost += gs
            .card_database
            .get_card(id)
            .and_then(|c| c.cost)
            .unwrap_or(0) as i32;
        if gs.mods.get_orientation_modifier(id) != Some("wait") {
            blades += i32::from(stats_pipeline::effective_blade(
                &gs.card_database,
                id,
                gs.mods
                    .blade_modifiers
                    .get(&id)
                    .cloned()
                    .unwrap_or_default(),
            ));
        }
    }
    (pool, blades, cost)
}

fn member_reserve(gs: &GameState, me: u8, id: i16) -> f64 {
    let p = gs.seat_player(me);
    let Some(card) = gs.card_database.get_card(id) else {
        return 0.0;
    };
    if card.card_type != CardType::Member {
        return 0.0;
    }
    let cost = i32::from(card.cost.unwrap_or(0));
    let discount = p
        .stage
        .stage
        .iter()
        .filter_map(|&cid| gs.card_database.get_card(cid))
        .filter_map(|c| c.cost)
        .max()
        .unwrap_or(0) as i32;
    let budget = p.energy_zone.cards.len().i32_count() + 1;
    let delay = (cost - discount - budget).max(0) as f64;
    let hearts = card
        .base_heart
        .as_ref()
        .map(|h| h.hearts.values_sum())
        .unwrap_or(0);
    (2.0 * cost as f64 + 2.0 * card.blade as f64 + hearts as f64) / (1.0 + delay)
}

fn future_development(gs: &GameState, me: u8) -> f64 {
    let p = gs.seat_player(me);
    let db = &gs.card_database;
    let budget = usize::from(p.energy_zone.active_count()) + 1;
    let stage_discount = p
        .stage
        .stage
        .iter()
        .filter_map(|&cid| db.get_card(cid))
        .filter_map(|card| card.cost)
        .max()
        .unwrap_or(0) as usize;
    p.hand
        .cards
        .iter()
        .filter_map(|&cid| db.get_card(cid))
        .filter(|card| card.card_type == CardType::Member)
        .filter_map(|card| card.cost)
        .filter(|&cost| usize::from(cost) <= budget + stage_discount)
        .map(|cost| cost as f64)
        .fold(0.0, f64::max)
}

#[derive(Clone)]
pub(crate) struct Features {
    hearts: i32,
    blades: i32,
    cost: i32,
    ammo: usize,
    coverage: f64,
    coverage_graded: f64,
    hand: usize,
    energy: usize,
    reserve: f64,
    future: f64,
    success: usize,
}

pub(crate) fn features(gs: &GameState, me: u8) -> Features {
    let p = gs.seat_player(me);
    let (pool, blades, cost) = board(gs, me);
    let mut mean = pool;
    let mut units = [0.0; 11];
    for &id in &p.main_deck.cards {
        if let Some(card) = gs.card_database.get_card(id) {
            if let Some(icons) = &card.blade_heart {
                for &(color, count) in &icons.hearts {
                    let i = crate::bot::strategy_common::hc_index(color);
                    if i != 8 && i != 9 {
                        units[i] += count as f64 * if i == 0 { 2.0 } else { 1.0 };
                    }
                }
            }
        }
    }
    for i in 0..11 {
        mean[i] += narrow_hearts(crate::constants::score_to_i32(
            (units[i] * blades as f64 / p.main_deck.cards.len().max(1) as f64).floor(),
        ));
    }
    let mut ammo = 0;
    let mut coverage = 0.0;
    let mut coverage_graded = 0.0f64;
    let mut reserves = Vec::new();
    for &id in &p.hand.cards {
        let Some(card) = gs.card_database.get_card(id) else {
            continue;
        };
        if card.card_type == CardType::Live {
            ammo += 1;
            let need = stats_pipeline::effective_need_heart(
                card.need_heart.as_ref(),
                id,
                &gs.mods.need_heart_modifiers,
            );
            let mut required: Acc = [0i16; 11];
            if let Some(need) = need {
                acc_add(&mut required, &need.hearts);
            }
            if requirements_met(&mean, &required) {
                coverage += 1.0;
            }
            coverage_graded = coverage_graded.max(graded_coverage(&mean, &required));
        } else if card.card_type == CardType::Member {
            reserves.push(member_reserve(gs, me, id));
        }
    }
    reserves.sort_by(|a, b| b.total_cmp(a));
    Features {
        hearts: pool
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != 8 && *i != 9)
            .map(|(_, v)| i32::from(*v))
            .sum(),
        blades,
        cost,
        ammo,
        coverage,
        coverage_graded,
        hand: p.hand.cards.len(),
        energy: usize::from(p.energy_zone.active_count()),
        reserve: reserves.iter().take(2).sum(),
        future: future_development(gs, me),
        success: p.success_live_card_zone.cards.len(),
    }
}

/// Fraction of a life's heart requirement the board can actually supply,
/// 0.0 to 1.0.
///
/// `coverage` is a **step**: `requirements_met` is all-or-nothing, so a board
/// holding 6 of the 7 icons a life needs scores exactly the same as one holding
/// 0 of 7. §17.3 measured that flatness and it is why the weight-60 term cannot
/// express "get closer": every position short of the bar is indistinguishable,
/// so the argmax has no reason to prefer 6-of-7 over 3-of-7 and settles wherever
/// the other terms happen to land.
///
/// This is the gradient that term is missing. It follows the engine's own
/// allocation order - specific colours first, then whatever is left over plus the
/// any-colour pool plus the wildcard bucket, exactly as `requirements_met` and
/// the engine's `check_heart_requirement` do - and reports the ratio instead of
/// the verdict, so 6-of-7 beats 0-of-7 and 11-of-11 beats 6-of-7.
///
/// The pool passed in is `features`' `mean`, which already includes the sampled
/// yell yield. Nothing here reads a new source of truth: this is a different
/// FUNCTION of the same features, which is why it can be switched on and off
/// without touching anything upstream of it.
fn graded_coverage(mean: &Acc, required: &Acc) -> f64 {
    let total: i32 = (0..11)
        .filter(|&i| i != 8 && i != 9)
        .map(|i| i32::from(required[i]))
        .sum();
    if total <= 0 {
        // A life with no heart requirement passes unconditionally.
        return 1.0;
    }
    let mut have = *mean;
    let mut satisfied = 0i32;
    // 1..=6 are the specific colours; 7 is the wildcard bucket, which stands in
    // for one missing heart of any colour; 10 is the any-colour pool itself.
    for c in 1..8usize {
        let want = i32::from(required[c]);
        if want <= 0 {
            continue;
        }
        let take = want.min(i32::from(have[c]));
        have[c] -= i16::try_from(take).unwrap_or(0);
        satisfied += take;
    }
    let leftover: i32 = (0..8).map(|c| i32::from(have[c])).sum::<i32>() + i32::from(have[10]);
    let want_any = i32::from(required[0]);
    satisfied += want_any.min(leftover);
    (satisfied as f64 / total as f64).clamp(0.0, 1.0)
}

fn terminal_win(gs: &GameState, me: u8) -> bool {
    match gs.game_result {
        GameResult::FirstAttackerWins => gs.player1.is_first_attacker == (me == 0),
        GameResult::SecondAttackerWins => gs.player1.is_first_attacker != (me == 0),
        _ => false,
    }
}

/// Leaf value of the v7 chain search, in v7's own units. `v7_ismcts` reuses
/// it to grade rollout actions so the policy and the prior it refines agree.
pub(crate) fn value(now: &Features, base: &Features, deploy: bool) -> f64 {
    // Development weight: default 8.0 (historical). V7_DEV_WEIGHT for
    // ablation  Elosses still cluster on dev_gap (audit 2026-09-23).
    let dev_w: f64 = std::env::var("V7_DEV_WEIGHT")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|w: &f64| w.is_finite() && *w >= 0.0)
        .unwrap_or(8.0);
    // The remaining features weights are hard-coded historically. Knobs added
    // so the ablation map can be completed rather than assumed: the dev weight
    // turned out to carry no signal across a 4x range, and the natural
    // question is whether any of the other eight do either.
    let w = |name: &str, default: f64| -> f64 {
        std::env::var(name)
            .ok()
            .and_then(|v| v.parse().ok())
            .filter(|x: &f64| x.is_finite() && *x >= 0.0)
            .unwrap_or(default)
    };
    let hearts_w = w("V7_HEARTS_WEIGHT", 3.0);
    let blades_w = w("V7_BLADES_WEIGHT", 6.0);
    let coverage_w = w("V7_COVERAGE_WEIGHT", 60.0);
    let ammo_w = w("V7_AMMO_WEIGHT", 25.0);
    let energy_w = w("V7_ENERGY_WEIGHT", 2.0);
    let hand_w = w("V7_HAND_WEIGHT", 3.0);
    let reserve_w = w("V7_RESERVE_WEIGHT", 0.5);
    // LEAN: keep only the two terms that dominate INDIVIDUALLY, plus the win
    // term, and zero the other seven.
    //
    // The per-term ablation at 3000 games x 2 seeds:
    //
    //     blades (6)  -5.1      reserve (0.5) -0.5     hands (3)   ~0
    //     ammo  (25)  -2.9      coverage(60)  -0.5     dev (8)     ~0
    //     chain search   -2.3    energy  (2)   -0.3     hearts(3)   -0.2
    //     future(2)   +0.5
    //
    // It is tempting to read that as "seven of the ten terms are decorative,
    // and the evaluation is functionally 6*dBlades + 25*dAmmo". That reading
    // is WRONG and this flag is the test that refutes it: zeroing all seven
    // together costs 14 points (1291-1709 against a 1709-1291 baseline,
    // 43.0% vs 57.0%).
    //
    // The terms are MUTUALLY REDUNDANT, not inert. Zeroing any one is absorbed
    // by the others; zeroing the bulk is not. So single-term ablation measures
    // dominance, not dispensability, and "this term barely mattered" is not
    // evidence that it can be removed. That is the one methodological trap in
    // this file worth writing down, because the natural next step from a
    // per-term table is exactly the wrong one.
    let lean = std::env::var_os("V7_LEAN").is_some();
    let zero = if lean { 0.0 } else { 1.0 };
    let mut score = zero * dev_w * (now.cost - base.cost) as f64
        + zero * hearts_w * (now.hearts - base.hearts) as f64
        + blades_w * (now.blades - base.blades) as f64
        + zero * coverage_w * (now.coverage - base.coverage)
        + ammo_w * (now.ammo as f64 - base.ammo as f64)
        + 1000.0 * (now.success as f64 - base.success as f64);
    // `V7_COVERAGE_GRADIENT=<w>`: the gradient `coverage` does not have. Off by
    // default, so the shipped leaf stays byte-identical; the weight is a ratio in
    // 0..1, so it is comparable against the other terms without rescaling them.
    //
    // The case for it is the only bucket left with measured headroom. Over 200
    // games, 88 of v9's 413 contested checks were lost after PASSING because the
    // committed life scored below the opponent's, and 104 of the 152 passed-but-
    // lost checks had a strictly higher-scoring life in hand. The 4pt and 7pt
    // lives need 11 and 15 icons against the 7 the 2pt needs, so the only way to
    // make them playable earlier is to build supply earlier - and the term that
    // models "this board can pass a life" is a step function that cannot see
    // progress toward the bar. See `graded_coverage`.
    if let Some(w) = coverage_gradient_weight() {
        score += w * (now.coverage_graded - base.coverage_graded);
    }
    if !deploy {
        score += zero * energy_w * (now.energy as f64 - base.energy as f64)
            + zero * hand_w * (now.hand as f64 - base.hand as f64)
            + zero * reserve_w * (now.reserve - base.reserve);
    }
    let future_enabled = !lean
        && (std::env::var_os("V7_FUTURE").is_none()
            || std::env::var("V7_FUTURE").ok().as_deref() == Some("1"));
    let future_weight: f64 = std::env::var("V7_FUTURE_WEIGHT")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value: &f64| value.is_finite() && *value >= 0.0)
        .unwrap_or(2.0);
    if future_enabled {
        score += (now.future - base.future) * future_weight;
    }
    // Flat override penalties. OFF by default.
    //
    // Same shape as the baton-over-Pass tie-break that cost v7 16.8 points
    // (1563-2437 against 2234-1766, two seeds, 8000 games a side): a
    // hand-chosen constant that overrides the model's own arithmetic, in a term
    // the linear score already accounts for. Spending your last live card is
    // already worth something  E`1000 * d_success` and the fold decision
    // price it  Eand a flat 120 is not derived from anything in the game.
    // `V7_AMMO_PENALTY=1` / `V7_HAND_PENALTY=1` restore them.
    let ammo_penalty: f64 = std::env::var("V7_AMMO_PENALTY")
        .as_deref()
        .map(|v| v == "1")
        .unwrap_or(false)
        .then_some(120.0)
        .unwrap_or(0.0);
    let hand_penalty: f64 = std::env::var("V7_HAND_PENALTY")
        .as_deref()
        .map(|v| v == "1")
        .unwrap_or(false)
        .then_some(60.0)
        .unwrap_or(0.0);
    if now.ammo == 0 && base.ammo > 0 {
        score -= ammo_penalty;
    }
    if now.hand <= 1 && base.hand > 1 {
        score -= hand_penalty;
    }
    score
}

struct Search<'a> {
    base: Features,
    opponent: Features,
    root: &'a GameState,
    me: u8,
    deploy: bool,
    nodes: usize,
}

/// Trained value function loaded once per process, if `V7_LEARNED_VALUE` names a
/// weights file. A `OnceLock` because `evaluate` runs tens of thousands of times
/// per decision and re-reading the file there would dominate the search.
static LEARNED_VALUE: std::sync::OnceLock<Option<super::neural::PolicyNet>> =
    std::sync::OnceLock::new();

fn learned_value_net() -> Option<&'static super::neural::PolicyNet> {
    LEARNED_VALUE
        .get_or_init(|| {
            let path = std::env::var("V7_LEARNED_VALUE").ok()?;
            if !std::path::Path::new(&path).exists() {
                eprintln!("V7_LEARNED_VALUE={path} not found; learned term disabled");
                return None;
            }
            let mut net = super::neural::PolicyNet::new();
            match net.load_weights(&path) {
                Ok(()) => {
                    eprintln!("learned value loaded from {path}");
                    Some(net)
                }
                Err(e) => {
                    eprintln!("V7_LEARNED_VALUE load failed: {e}");
                    None
                }
            }
        })
        .as_ref()
}

/// Learned value contribution to the leaf, or `None` when disabled.
///
/// The net was trained on the seat's own perspective, so the output is already
/// "how good is this for me"; no sign flip is needed. The term is a DELTA
/// against the root position rather than a level, for the same reason the hand
/// leaf is: the leaf is compared across candidate actions within one turn, so
/// only the change matters, and a level would add a constant that cancels.
fn learned_value_term(gs: &GameState, me: u8) -> Option<f64> {
    let net = learned_value_net()?;
    let obs = crate::bot::observation::PublicObservation::from_state(gs, me);
    let encoded = net.encode_state(&obs);
    let v = net.evaluate_state_value(&encoded);
    if !v.is_finite() {
        return None;
    }
    let weight: f64 = std::env::var("V7_LEARNED_VALUE_WEIGHT")
        .ok()
        .and_then(|x| x.parse().ok())
        .filter(|w: &f64| w.is_finite())
        .unwrap_or(1000.0);
    // The hand leaf works in "units of a placement" (1000 per success), so the
    // learned term is scaled to match. Default 1000 puts a +-1 net output at the
    // same magnitude as winning or losing a placement outright.
    Some(weight * v as f64)
}

impl Search<'_> {
    /// for ablation. Budget-limited leaves return eval-1.0, so a larger
    /// budget mainly deepens choice chains rather than changing scalars.
    fn node_budget() -> usize {
        std::env::var("V7_NODES")
            .ok()
            .and_then(|v| v.parse().ok())
            .filter(|n: &usize| *n > 0)
            .unwrap_or(64)
    }

fn evaluate(&self, gs: &GameState) -> f64 {
    // Terminal short-circuit, ported in shape from SabberStone's `Score.cs`:
    //
    // ```csharp
    // if (OpHeroHp < 1) return int.MaxValue;
    // if (HeroHp  < 1) return int.MinValue;
    // ```
    //
    // A decided game returns before ANY weighted term is read. That matters here
    // because this leaf's success term is `1000 * (Δsuccess)`, a hard zero for
    // every positional term except a terminal one, so a near-win can currently
    // be outvoted by a large `coverage` delta: the search has no way to express
    // "this placement ends the game" other than through the delta, which is
    // also why a 3-success lead and a 0-success game can score the same when
    // nothing else differs.
    //
    // Gated on `V7_TERMINAL` because returning a constant changes what `complete`
    // does with budget-limited leaves, and this leaf is load-bearing for v9.
    if std::env::var_os("V7_TERMINAL").is_some() {
        if gs.game_result == GameResult::Draw {
            return 0.0;
        }
        if gs.seat_player(self.me).success_live_card_zone.cards.len() >= 3 {
            return 1.0e9;
        }
        if gs.seat_player(1 - self.me).success_live_card_zone.cards.len() >= 3 {
            return -1.0e9;
        }
    }
    let own = features(gs, self.me);
        let opp = gs.seat_player(1 - self.me);
        let original_opp = self.root.seat_player(1 - self.me);
        let (_, opp_blades, opp_cost) = board(gs, 1 - self.me);
        let hand = value(&own, &self.base, self.deploy)
            - 1000.0
                * (opp.success_live_card_zone.cards.len() as f64 - self.opponent.success as f64)
            + opp_terms(
                gs,
                self.me,
                self.root,
                self.opponent.cost,
                self.opponent.blades,
            )
            // Opponent COVERAGE, the missing counterpart to our own weight-60
            // term.
            //
            // Gated on `V7_OPP_COVERAGE` (off by default) and, like the three terms
            // above it, **provably constant during a Main-phase search**: the
            // opponent takes no actions while we search, so their board and hand
            // do not change and the term cancels in the argmax. Measured: at
            // weights 10/30/60/-20 the arena reported identical records and
            // identical discordance (7/4, p=0.549) every time, which is the
            // signature of a term present in the arithmetic and absent from the
            // decision. Kept because it is the correct expression for any phase
            // where the opponent DOES move.
            - opp_coverage_weight()
                * (features(gs, 1 - self.me).coverage - self.opponent.coverage);
        // `V7_LEARNED_VALUE=<weights>` adds a trained value function to this leaf.
        //
        // This is the intervention the CCG search literature supports, and it is
        // deliberately NOT a learned policy. Two RL attempts to replace the
        // policy failed here: behaviour cloning reached 53% action accuracy but
        // played at 35.5% against its own teacher, and PPO on a measured ladder
        // rung regressed 47.3% -> 37.5%. Both failed the same way - the teacher
        // decides Pass through a DELTA leaf, which a level model cannot
        // represent, so a policy net never learns to stop.
        //
        // The search keeps doing that work. This adds one more opinion about how
        // good a position is, leaving the search and the delta leaf intact.
        // Świechowski et al. (arXiv 1808.04794) did exactly this for Hearthstone
        // MCTS and lifted the second-attacker bot from 26.5% to 53.2%; Zhang &
        // Buro (CIG 2017) used a learned net the same way for rollout guidance.
        if let Some(bonus) = learned_value_term(gs, self.me) {
            hand + bonus
        } else {
            hand + presence_term(gs, self.me)
        }
    }
}

/// Opponent terms: stage cost, blades, hand size.
///
/// ## These are constant during a Main-phase search, and that is the point
///
/// `evaluate` is called once per candidate action within ONE turn, while we
/// control the seat. The opponent takes no actions in between, so their stage
/// cost, blade count and hand size are identical for every candidate. They are
/// therefore **identical offsets across the whole candidate set and cancel in
/// the argmax** — the same shape `v8_main.rs:1058-1063` documents for
/// `initiative`, and the same class `V8_NO_ASSUME_COMMIT` exists to neutralise in
/// the live set.
///
/// They are moved behind `V7_OPP_TERMS` (default ON, so shipping behaviour is
/// unchanged bit-for-bit) so the claim can be measured rather than argued: with
/// the terms off, the leaf keeps only our own side plus the success delta, and
/// the win rate either does not move — proving they were decorative — or moves,
/// proving they were doing something this analysis missed. Either outcome is
/// information; the previous state, where they were believed to cost real weight
/// and never tested, was not.
fn opp_terms(
    gs: &GameState,
    me: u8,
    root: &GameState,
    baseline_cost: i32,
    baseline_blades: i32,
) -> f64 {
    if std::env::var_os("V7_OPP_TERMS").is_some() {
        return 0.0;
    }
    let (_, opp_blades, opp_cost) = board(gs, 1 - me);
    let opp = gs.seat_player(1 - me);
    let original_opp = root.seat_player(1 - me);
    -8.0 * (opp_cost as f64 - baseline_cost as f64)
        - 6.0 * (opp_blades as f64 - baseline_blades as f64)
        - 10.0 * (opp.hand.cards.len() as f64 - original_opp.hand.cards.len() as f64)
}

/// Weight on the opponent's live-card coverage. `V7_OPP_COVERAGE=<w>`; absent by
/// default. Negative values flip the sign, which is the test that distinguishes
/// "the term is wrong" from "the coefficient is wrong".
fn opp_coverage_weight() -> f64 {
    std::env::var("V7_OPP_COVERAGE")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|w: &f64| w.is_finite())
        .unwrap_or(0.0)
}

/// `V7_COVERAGE_GRADIENT=<w>`, or `None` when unset so the term is not read at
/// all. A finite weight only; the value is added to the linear leaf.
fn coverage_gradient_weight() -> Option<f64> {
    std::env::var("V7_COVERAGE_GRADIENT")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|w: &f64| w.is_finite())
}

/// Diagnostic: is the opponent's coverage term even LIVE in the search?
///
/// The leaf compares candidate actions within one Main-phase turn. The opponent
/// does not move during that turn, so `opp_coverage` is a CONSTANT across every
/// candidate and cancels in the argmax exactly as `v8_main`'s `initiative` does
/// (`v8_main.rs:1058-1063` documents the same shape). That is a structural
/// property, not a tuning question, and no weight can fix it.
///
/// Emitted under `V7_DIAG_TERMS` so the claim is measured rather than argued.
pub(crate) fn diag_term_liveness(gs: &GameState, me: u8) -> Option<(f64, f64)> {
    if std::env::var_os("V7_DIAG_TERMS").is_none() {
        return None;
    }
    let mine = features(gs, me).coverage;
    let theirs = features(gs, 1 - me).coverage;
    Some((mine, theirs))
}

/// Board-PRESENCE term, ported in shape from SabberStone's competition AI
/// (`core-extensions/SabberStoneBasicAI/src/Score/MidRangeScore.cs`):
///
/// ```csharp
/// if (OpBoardZone.Count == 0 && BoardZone.Count > 0) result += 5000;
/// result += (BoardZone.Count - OpBoardZone.Count) * 5;
/// ```
///
/// The shape matters because §17.3 measured the opposite property in this
/// repo's leaf: `coverage` (weight 60, the second-largest term) is a *threshold*
/// over the whole requirement, so the gradient is flat exactly where "is there a
/// board at all" should dominate, and §15.2-A found the leaf has no presence
/// term of any kind. SabberStone treats an empty board against a non-empty one
/// as a discrete, large event rather than something that accumulates.
///
/// This game's win condition is placements, not damage, so the weights do not
/// transfer - only the shape. Gated behind `V7_PRESENCE` (off by default)
/// because a ported constant from a different game is a hypothesis, not a result,
/// and this project has sixteen consecutive live-set nulls behind it.
fn presence_term(gs: &GameState, me: u8) -> f64 {
    if std::env::var_os("V7_PRESENCE").is_none() {
        return 0.0;
    }
    let mine = gs.seat_player(me).stage.stage.iter().filter(|id| **id >= 0).count();
    let theirs = gs.seat_player(1 - me).stage.stage.iter().filter(|id| **id >= 0).count();
    let occupancy: f64 = std::env::var("V7_PRESENCE_OCCUPANCY")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|w: &f64| w.is_finite())
        .unwrap_or(5.0);
    // Same discrete event as the C#: we are on board and they are not.
    let mut result = 0.0;
    if theirs == 0 && mine > 0 {
        result += occupancy * 1000.0;
    }
    result + occupancy * (mine as f64 - theirs as f64)
}

impl Search<'_> {
    fn complete(&mut self, gs: &GameState, depth: usize) -> (f64, usize, &'static str) {
        if gs.game_result != GameResult::Ongoing {
            return (self.evaluate(gs), depth, "complete");
        }
        if !gs.has_pending_choice() {
            let beam = std::env::var("V7_BEAM")
                .map(|value| value != "0")
                .unwrap_or(false);
            if beam
                && depth < 1
                && gs.current_phase == Phase::Main
                && gs.can_player_act(self.me as i32)
                && self.nodes < Self::node_budget()
            {
                let mut best = (f64::NEG_INFINITY, depth, "beam-empty");
                for action in game_setup::generate_possible_actions(gs) {
                    if self.nodes >= Self::node_budget() {
                        break;
                    }
                    if action.parameters.as_ref().and_then(|p| p.disabled) == Some(true) {
                        continue;
                    }
                    self.nodes += 1;
                    crate::rng::seed(0x7637 + depth.u32_count());
                    let mut next = gs.clone();
                    if game_setup::execute_action(&mut next, &action).is_err() {
                        continue;
                    }
                    let candidate = self.complete(&next, depth + 1);
                    if candidate.0 > best.0 {
                        best = candidate;
                    }
                }
                if best.0.is_finite() {
                    return best;
                }
            }
            return (self.evaluate(gs), depth, "complete");
        }
        if !gs.can_player_act(self.me as i32) {
            return (self.evaluate(gs), depth, "opponent-choice");
        }
        if depth >= 8 || self.nodes >= Self::node_budget() {
            // A budget-limited leaf is UNCERTAIN, not bad.
            //
            // This was a flat -1.0, on a scale where a good deploy is worth
            // 50-165 and coverage 60. So a line that simply ran out of budget
            // was crushed far below any resolved line, which biases the whole
            // search against long chains  E`V7_NODES=1` costs 2.3 points
            // precisely because then EVERY leaf is penalised and the ordering
            // collapses. Same class as the baton-over-Pass tie-break that cost
            // 16.8 points: a hand-placed magnitude overriding the model's own
            // arithmetic.
            //
            // What the penalty is for is winning ties against a resolved line,
            // not outscoring one. `V7_CHOICE_BUDGET` restores the old 1.0;
            // the default is a small epsilon that expresses the same intent.
            let penalty: f64 = std::env::var("V7_CHOICE_BUDGET")
                .ok()
                .and_then(|value| value.parse().ok())
                .filter(|value: &f64| value.is_finite() && *value >= 0.0)
                .unwrap_or(0.01);
            return (self.evaluate(gs) - penalty, depth, "choice-budget");
        }
        let mut best = (f64::NEG_INFINITY, depth, "execution-error");
        for action in game_setup::generate_possible_actions(gs) {
            if self.nodes >= Self::node_budget() {
                break;
            }
            if action.parameters.as_ref().and_then(|p| p.disabled) == Some(true) {
                continue;
            }
            self.nodes += 1;
            crate::rng::seed(0x7637 + depth.u32_count());
            let mut next = gs.clone();
            if game_setup::execute_action(&mut next, &action).is_err() {
                continue;
            }
            let candidate = self.complete(&next, depth + 1);
            if candidate.0 > best.0 {
                best = candidate;
            }
        }
        best
    }
}

struct RngGuard(u32);
impl Drop for RngGuard {
    fn drop(&mut self) {
        crate::rng::restore(self.0);
    }
}

fn simulation_state(gs: &GameState, me: u8) -> GameState {
    let mut sim = gs.clone();
    let (own, opp) = if me == 0 {
        (&mut sim.player1, &mut sim.player2)
    } else {
        (&mut sim.player2, &mut sim.player1)
    };
    own.main_deck.cards.sort_unstable();
    let mut rng = crate::rng::Lcg::new(0x76375f6d61696e);
    for i in (1..own.main_deck.cards.len()).rev() {
        own.main_deck.cards.swap(i, rng.range(i + 1));
    }
    if std::env::var_os("V7_FAIR_DETERMINIZATION").is_some() {
        let observation = PublicObservation::from_state(gs, me);
        let sampler = DeterminizationSampler::new_fair(crate::Arc::clone(&gs.card_database), &[]);
        let sampled = sampler.sample(&observation);
        let (_, sampled_opp) = sampled.seated_pair(me);
        opp.hand.cards = sampled_opp.hand.cards.clone();
        opp.main_deck.cards = sampled_opp.main_deck.cards.clone();
        opp.energy_deck.cards = sampled_opp.energy_deck.cards.clone();
        if matches!(
            gs.current_phase,
            Phase::Main | Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker
        ) {
            opp.live_card_zone.cards = sampled_opp.live_card_zone.cards.clone();
        }
    } else {
        opp.hand.cards.fill(-1);
        opp.main_deck.cards.fill(-1);
        if matches!(
            gs.current_phase,
            Phase::Main | Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker
        ) {
            opp.live_card_zone.cards.fill(-1);
        }
    }
    sim
}

/// The baton-over-Pass tie-break is OFF by default.
///
/// HISTORY. Pass is always `actions[0]` and a free equal-stat baton swap
/// scores exactly 0.00, so strict `>` kept Pass on every 0-0 tie and produced
/// an "empty-main spiral" (seed 42: t3/t4/t6, empty Main 7/20 = 35%). A
/// tie-break preferring a baton over Pass was added to cure that symptom.
///
/// Two measurements retire it. Restoring the full tie-break
/// (`V7_FREE_BATON=1`) costs 14 points  E1563-2437 against a 2135-1865 default
/// at 4000 games, reproducible  Ebecause it also lets a ZERO-COST baton beat
/// Pass even when it scores lower, which is a much broader override than the
/// original fix. And the spiral it was written for is gone: v7's empty-Main
/// rate is 11.1%, not 35%.
///
/// The mechanism it rewards is also the one the rest of the night's
/// measurements say is harmful. A swap throws away a board whose heart colours
/// v7 chose to build, and this game is decided by heart coverage: the guide's
/// 4->9->13 curve is a score doctrine that costs 9.6pp here, and v7 enters a
/// contested check in 0.5% of cases, so there is no score being bought.
///
/// The original fix was validated against a symptom (empty-Main %) rather than
/// against the objective, and it outlived the symptom it was written for.
///
/// NOTE: gating only `is_free_baton` is not enough. The `si == sb` clause
/// fires on its own, so an earlier version of this change left the default
/// behaviour byte-identical and measured a perfect null  Ethe same dead-branch
/// trap as `V7_LIVE_LEGACY`.
fn is_free_baton(gs: &GameState, me: u8, action: &Action) -> bool {
    if !is_baton_action(gs, me, action) {
        return false;
    }
    action.parameters.as_ref().is_some_and(|params| {
        params.final_cost == Some(0)
            || params.available_areas.as_ref().is_some_and(|areas| {
                areas
                    .iter()
                    .any(|area| area.is_baton_touch && area.cost == 0)
            })
    })
}

fn action_card_label(gs: &GameState, action: &Action) -> String {
    action
        .parameters
        .as_ref()
        .and_then(|p| p.card_id)
        .and_then(|id| gs.card_database.get_card(id))
        .map(|card| format!("{}:{:?}", card.card_no, card.card_type))
        .or_else(|| {
            action
                .parameters
                .as_ref()
                .and_then(|p| p.card_id.map(|id| id.to_string()))
        })
        .unwrap_or_else(|| "-".into())
}

fn rollout_suggestion(gs: &GameState, actions: &[Action], me: u8) -> Option<Action> {
    if gs.current_phase != Phase::Main {
        return None;
    }
    let simulations = std::env::var("V7_ROLLOUT_SIMS")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value: &usize| *value > 0)
        .unwrap_or(8);
    let horizon = std::env::var("V7_ROLLOUT_HORIZON")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value: &u8| *value > 0)
        .unwrap_or(1);
    let values = crate::bot::rollout::price_main_actions(gs, me, actions, simulations, horizon);
    let (index, _) = values
        .iter()
        .enumerate()
        .max_by(|(_, left), (_, right)| left.total_cmp(right))?;
    actions.get(index).cloned()
}
fn pick_best(gs: &GameState, me: u8, actions: &[Action], scores: &[(f64, String)]) -> usize {
    let mut best = 0usize;
    for i in 1..scores.len() {
        let si = scores[i].0;
        let sb = scores[best].0;
        if si > sb {
            best = i;
        } else if std::env::var_os("V7_FREE_BATON").is_some()
            && si.is_finite()
            && actions[best].action_type == ActionType::Pass
            && actions[i].action_type != ActionType::Pass
            && is_baton_action(gs, me, &actions[i])
            && (si == sb || is_free_baton(gs, me, &actions[i]))
        {
            log::debug!(
                "v7_main baton-tie-break t{} me{} idx{} {:?} score={:.2} beats Pass",
                gs.turn_number,
                me,
                i,
                actions[i].action_type,
                si
            );
            best = i;
        }
    }
    best
}

/// Active blades on a seat's stage  Ethe check's draw count, and the resource
/// `useful_blades` prices.
fn stage_blades_of(gs: &GameState, me: u8, db: &crate::card::CardDatabase) -> u32 {
    (gs.seat_player(me)
        .stage
        .total_blades(
            db,
            &gs.mods.blade_modifiers,
            &gs.mods.orientation_modifiers,
            false,
        )) as u32
}

// A per-bot override for the two live-set-lookahead weights, so a candidate can
// be measured on ONE seat. `V7_CHECK_PRESSURE` and `V7_BLADE_ARITH` are read
// from the environment, which is process-global — both seats of a head-to-head
// would change together and the matchup would measure nothing. Same pattern as
// `RngGuard`: set it at the candidate's entry point, restore on the way out.
thread_local! {
    static MAIN_LOOKAHEAD: std::cell::Cell<f64> = const { std::cell::Cell::new(-1.0) };
    static MAIN_BLADE_ARITH: std::cell::Cell<f64> = const { std::cell::Cell::new(-1.0) };
}

/// Installs a per-bot Main-phase lookahead weight for the duration of a call.
pub struct LookaheadGuard {
    prev_pressure: f64,
    prev_blade: f64,
}

impl LookaheadGuard {
    pub fn new(pressure: f64, blade_arith: f64) -> Self {
        let prev_pressure = MAIN_LOOKAHEAD.with(|c| c.replace(pressure));
        let prev_blade = MAIN_BLADE_ARITH.with(|c| c.replace(blade_arith));
        Self {
            prev_pressure,
            prev_blade,
        }
    }
}

impl Drop for LookaheadGuard {
    fn drop(&mut self) {
        MAIN_LOOKAHEAD.with(|c| c.set(self.prev_pressure));
        MAIN_BLADE_ARITH.with(|c| c.set(self.prev_blade));
    }
}

/// `V7_CHECK_PRESSURE` default: **0.0**, and that default is the finding.
///
/// The Main phase scores a candidate action by feature deltas toward a PROXY
/// for "will I place next check". The real evaluator — the same 256-draw pass
/// model the live-set chooser uses — is already available and already wired in
/// right here, one line below, and it is switched off.
///
/// That is the same shape of error `vbr` proved is wrong in the live set:
/// one-phase greedy toward place-probability is not the objective, because 8.4.13
/// makes a placement change next turn's attacker slot and the win condition is a
/// race to three (1.2.1.1). The best action for THIS check is frequently not the
/// best action for the game.
///
/// Everything on record for these two weights was read in mirrors, which cannot
/// see a one-seat change. So they have never had a valid test.
pub(crate) fn score_actions(gs: &GameState, actions: &[Action], me: u8) -> Vec<(f64, String)> {
    let _rng = RngGuard(crate::rng::checkpoint());
    let root = simulation_state(gs, me);
    let base = features(&root, me);
    let opponent = features(&root, 1 - me);
    let pending = gs.has_pending_choice();
    // Off (0.0) until measured. `V7_CHECK_PRESSURE=<weight>` enables it; the
    // weight is in v7's own score units, so 75 means "a deploy that takes the
    // best life in hand from a coin flip to near-certainty is worth about one
    // good member".
    let pressure_weight: f64 = MAIN_LOOKAHEAD.with(|c| {
        let override_w = c.get();
        if override_w >= 0.0 {
            return override_w;
        }
        std::env::var("V7_CHECK_PRESSURE")
            .ok()
            .and_then(|value| value.parse().ok())
            .filter(|value: &f64| value.is_finite() && *value >= 0.0)
            .unwrap_or(0.0)
    });
    let base_pressure = if pressure_weight > 0.0 {
        crate::bot::strategy_v7::check_pressure(&root, me, &gs.card_database)
    } else {
        0.0
    };
    // Placement forecast: expected placements over the next few checks under a
    // projected board, each evaluated with the exact 256-draw check model.
    // `V7_FORECAST=<weight>`, `V7_FORECAST_DEPTH=<n>`. Off until measured.
    let forecast_weight: f64 = std::env::var("V7_FORECAST")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value: &f64| value.is_finite() && *value >= 0.0)
        .unwrap_or(0.0);
    let forecast_depth: usize = std::env::var("V7_FORECAST_DEPTH")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value: &usize| *value > 0)
        .unwrap_or(3);
    let base_forecast = if forecast_weight > 0.0 {
        crate::bot::strategy_v7::placement_forecast(&root, me, &gs.card_database, forecast_depth)
    } else {
        0.0
    };
    // Ability activation refund. `V7_ABILITY_REFUND=<0..1.5>`; 0 = today's
    // behaviour (abilities effectively unplayable), 1.0 = full refund of the
    // development term the activation consumed.
    //
    // `V7_ABILITY_TURNS=<lo>-<hi>` gates the refund to a turn window. This is
    // NOT a retune of the blanket version, which measured null (916-584 vs a
    // 910-590 baseline, p=0.61, n=5000)  Ethat one encouraged abilities on
    // every turn including T1, where a move-level analysis over 5000 traced
    // games shows the action is worth almost nothing:
    //
    //     play_member_to_stage|1-Main   lift +0.002
    //     play_member_to_stage|3-Main   lift +0.033
    //     use_ability|3-Main            lift +0.086
    //     use_ability|6-Main            lift +0.147
    //
    // The association is correlational  Ev7 activates when its board is already
    // good  Ebut the gradient is monotone in turn for abilities and flat at
    // zero for turn-1 development, which is a specific and testable claim
    // rather than a general "use more".
    let ability_turns: Option<(u8, u8)> = std::env::var("V7_ABILITY_TURNS")
        .ok()
        .and_then(|value| {
            let (lo, hi) = value.split_once('-')?;
            Some((
                lo.trim().parse().ok()?,
                hi.trim().parse().ok()?,
            ))
        })
        .filter(|(lo, hi)| lo <= hi);
    let ability_refund: f64 = std::env::var("V7_ABILITY_REFUND")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value: &f64| value.is_finite() && *value >= 0.0)
        .unwrap_or(0.0);
    // Ability valuation by resolved effect. `V7_ABILITY_FORECAST=<weight>`.
    // Supersedes the blanket refund, which measured as a null.
    let ability_forecast_weight: f64 = std::env::var("V7_ABILITY_FORECAST")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value: &f64| value.is_finite() && *value >= 0.0)
        .unwrap_or(0.0);
    // Ability class gate. `V7_ABILITY_CLASS=<weight>`; scales `class_value`.
    let ability_class_gate: f64 = std::env::var("V7_ABILITY_CLASS")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value: &f64| value.is_finite() && *value >= 0.0)
        .unwrap_or(0.0);
    // The no-op gate is a switch, not a coefficient. `V7_NOOP_GATE=0` turns it
    // off; there is deliberately nothing to tune, because the question it asks
    // ("does this ability do anything?") has a yes/no answer.
    let ability_noop_gate = std::env::var("V7_NOOP_GATE")
        .as_deref()
        .map(|value| value != "0")
        .unwrap_or(true);
    // Qualitative ability gate. Binary, on by default, no weight.
    let ability_qualitative = std::env::var("V7_ABILITY_QUAL")
        .as_deref()
        .map(|value| value != "0")
        .unwrap_or(true);
    // Opening-curve move log. `V7_CURVE_DEBUG=1`.
    let curve_debug = std::env::var_os("V7_CURVE_DEBUG").is_some();
    // Blade arithmetic: re-price active blades by what they can actually
    // fill against the life in hand and the colours left in our own deck.
    // `V7_BLADE_ARITH=<weight>`; 0 keeps v7's flat 6/blade.
    let blade_arith: f64 = MAIN_BLADE_ARITH.with(|c| {
        let override_w = c.get();
        if override_w >= 0.0 {
            return override_w;
        }
        std::env::var("V7_BLADE_ARITH")
            .ok()
            .and_then(|value| value.parse().ok())
            .filter(|value: &f64| value.is_finite() && *value >= 0.0)
            .unwrap_or(0.0)
    });
    // MEASURED 2026-09-23: Pass=-0.05 + energy tax exploded draws
    // (1569/3000, avg turns 3.7)  Eevery UseAbility at score≁E then beat
    // Pass and main never ended cleanly. Keep flat 0.0 (v6 doctrine).
    let mut scores = Vec::with_capacity(actions.len());
    for action in actions {
        if action.parameters.as_ref().and_then(|p| p.disabled) == Some(true) {
            scores.push((f64::NEG_INFINITY, "disabled".into()));
            continue;
        }
        if action.action_type == ActionType::Pass && !pending {
            // Keep Pass visible in V7_DEBUG traces (was short-circuited).
            if std::env::var_os("V7_DEBUG").is_some() {
                eprintln!(
                "V7M t={} me={} act=Pass card=- area=- fc=- value=0.00 choices=0 depth=0 status=end-main st=- bl=- h={} en={} hand={} ammo={} succ={}",
                gs.turn_number, me, base.hearts, base.energy, base.hand, base.ammo, base.success
            );
            }
            scores.push((0.0, "end-main=0".into()));
            continue;
        }
        crate::rng::seed(0x7637);
        let mut sim = root.clone();
        if game_setup::execute_action(&mut sim, action).is_err() {
            scores.push((f64::NEG_INFINITY, "execution-error".into()));
            continue;
        }
        let immediate = {
            crate::turn::TurnEngine::check_victory_condition(&mut sim);
            terminal_win(&sim, me)
        };
        let mut search = Search {
            base: base.clone(),
            opponent: opponent.clone(),
            root: &root,
            me,
            deploy: action.action_type == ActionType::PlayMemberToStage,
            nodes: 0,
        };
        let (mut score, depth, status) = if immediate {
            (10000.0, 0, "terminal-win")
        } else {
            search.complete(&sim, 0)
        };
        // D1 baton vision: DETECT for logging/[BATON] marks, but do NOT add a
        // flat bonus. MEASURED 2026-09-23 (seed 11, 3000ÁE): +45 ↁE39.5% vs
        // v6 (folds 3.7%); +12 with ammo-guard ↁE50.5%; flat 0 ↁEbaseline.
        // The `value()` development term (8ÁEstage-cost growth) already prices
        // discounted upgrades  Ea flat baton bonus double-counts and over-dumps
        // the hand (live-set folds 0.3%ↁE.1% at +45).
        let baton = is_baton_action(gs, me, action);
        if !pending && action.action_type == ActionType::UseAbility && score <= 0.0 {
            score -= 0.01;
        }
        // ABILITY REFUND. `value()` prices stage-cost growth at 8/cost because
        // in the Main phase development IS the objective. For an activation
        // that is wrong in a way that makes abilities unplayable: the member is
        // *consumed by design*, so the stage loses its cost and blades, and the
        // term charges -8*4 = -32 plus -6*blades for it  Ewhile the ability's
        // actual production (cards drawn, energy, hearts, blades granted) is
        // worth far less than 32 in `value()` units. The bot therefore scored
        // real abilities at -25 and declined them, which the move-level trace
        // confirms across every game sampled.
        //
        // So: refund the development term for the cost the ability itself
        // consumed. What the ability PRODUCED is still priced normally, so an
        // ability that gives nothing still loses to Pass. `V7_ABILITY_REFUND`
        // scales it (0 disables, 1.0 = full refund of the dev term).
        let turn_ok = ability_turns.is_none_or(|(lo, hi)| gs.turn_number >= lo && gs.turn_number <= hi);
        if ability_refund > 0.0 && turn_ok && action.action_type == ActionType::UseAbility {
            let (_, _, cost_before) = board(&root, me);
            let (_, _, cost_after) = board(&sim, me);
            // `cost_before - cost_after` is POSITIVE when the activation
            // consumed a member, which is exactly the penalty to hand back.
            // (Getting this sign wrong is not hypothetical: the first version
            // added a further -32 to consuming abilities and measured ability
            // usage DROPPING from 185 to 128 in 200 games.)
            let refund = ability_refund * (cost_before - cost_after) as f64 * 8.0;
            score += refund;
        }
        // Check pressure: how much closer this action brings the board to
        // actually PASSING the next check, using the same 256-draw model that
        // `choose_live_set_experiment` uses to decide whether to set a life at
        // all. `value()`'s `60 ÁEΔpassable` is a one-ply coverage proxy and is
        // blind to active blades, which are the deck draws the check reads.
        //
        // Deliberately a DELTA from the root pressure, for the same reason
        // every other term here is: Pass leaves the board untouched, so its
        // pressure delta is exactly 0 and its 0.0 score is unchanged. An
        // action that buys a placement has to outbid ending the phase.
        if pressure_weight > 0.0 {
            let after = crate::bot::strategy_v7::check_pressure(&sim, me, &gs.card_database);
            score += pressure_weight * (after - base_pressure);
        }
        // BLADE ARITHMETIC.
        //
        // `value()` pays a flat 6 per blade because a blade is "a card
        // revealed from the deck", and treats that as worth a heart. It is
        // only worth a heart of the colour the life in hand is SHORT of, and
        // whether the next card is that colour is a property of our own deck,
        // which we are allowed to read.
        //
        // The measured prize: v7's committed live sets pass at 0.924 and v8's
        // at 0.485, with v8 holding MORE active blades (5.81 vs 4.95). Two
        // bots, the same game, and the one with more randomness is half as
        // good at converting  Ebecause neither of them asks what the randomness
        // is FOR. A blade drawn from a deck that holds none of the colours a
        // life needs is not a heart, it is a card you already had.
        //
        // This is arithmetic, not a tuned weight: the only number in it is
        // how much a filled requirement is worth, which the game already
        // decides by whether the check is won at all. The DEFIcit and the deck
        // distribution are both computed from the real card database.
        if blade_arith > 0.0 {
            // `value()` already charged a flat 6/blade for the delta. Take it
            // back and re-pay it at the colour-aware rate.
            let root_blades = crate::bot::strategy_v7::useful_blades(
                &root,
                me,
                &gs.card_database,
                0,
            );
            let _ = root_blades;
            let blades_before = stage_blades_of(&root, me, &gs.card_database) as i32;
            let blades_after = stage_blades_of(&sim, me, &gs.card_database) as i32;
            let flat = f64::from(blades_after - blades_before) * 6.0;
            let useful = crate::bot::strategy_v7::useful_blades(
                &sim,
                me,
                &gs.card_database,
                (blades_after - blades_before).max(0),
            ) - crate::bot::strategy_v7::useful_blades(
                &root,
                me,
                &gs.card_database,
                0,
            );
            score += blade_arith * (useful - flat);
        }
        // ABILITY VALUATION, BY WHAT IT ACTUALLY DOES.
        //
        // A blanket refund was the "all abilities are the same" mistake and it
        // measured as one: enabling 28% more ability uses changed nothing
        // (v7 vs v6, 743 -> 752, p=0.35). The abilities v7 newly reached were
        // net-negative, because the refund was not asking whether an ability
        // was worth using  Eonly whether its member cost had been charged.
        //
        // So: execute the ability (already done above  E`sim` IS the resolved
        // state) and ask what it did to the CONVERSION outlook, over several
        // turns, using the exact 256-draw check model. That is specific to the
        // ability by construction  Eno two abilities produce the same resolved
        // board, and a draw, a search, an energy swing, +2 blades and a member
        // removal are all priced by what they actually buy rather than by a
        // shared discount. It is also why the forecast was inert as a
        // deploy term (a deploy moves the board marginally) but should not be
        // inert here: an ability can move it a long way.
        //
        // Legal information only: our own hand, stage and deck, and public
        // board. Nothing here reads the opponent's hidden zones, so this is
        // future sight, not cheating.
        // ABILITY CLASS GATE. Prefer recognised trades, refuse the rest.
        //
        // v7 was using abilities essentially at random because it could not
        // tell them apart, and the cost was real: enabling ability use scaled
        // Rule 12-1 draws linearly with the weight (187 -> 267 draws, win rate
        // 816 -> 737 at n=1500). Thirty of 71 measured uses were member-
        // consuming with no measurable production  Erepeat those and the stage
        // empties, the state stops changing, and the repetition protocol fires.
        //
        // So: read the ability's printed text, and only spend the Main phase on
        // classes we recognise, each at its own price. Blades and hearts are
        // `Power` because they are what the check actually reads. An ability
        // that classifies as `Inactive` is pushed below Pass, because an
        // unreadable ability's loop risk cannot be priced.
        // STRUCTURAL ABILITY GATE  Ebinary, not a weight.
        //
        // Every policy that increased v7's ability use cost ~200 draws per 1500
        // games, at p as low as 0.00002, and three separate attempts produced
        // the identical signature. The cause is not mispricing: the engine
        // offers `UseAbility` actions whose resolution leaves the position
        // unchanged, and spending one again is the same action type on the same
        // state hash, which is exactly what Rule 12-1 detects. Same mechanism
        // as the ISMCTS's 309 draws.
        //
        // So ask the engine rather than tune a coefficient: resolve this
        // ability a SECOND time on its own result and compare the two
        // positions with the engine's own state hash. Identical means the
        // ability has no persistent effect, and a no-op is worth nothing at any
        // weight  Eit only spends a member, advances the repetition counter,
        // and hands the turn back.
        //
        // Comparing state hashes rather than six aggregate counters is the
        // point: an earlier aggregate-delta classifier called 63 of 71 measured
        // ability uses "produces nothing", because a card moving between zones
        // changes the position without changing any one counter.
        //
        // MEASURED, and this is the one ability change with a causal result:
        //   class gate, gate off : 706-549, p=0.00009, 245 draws
        //   class gate, gate on  : 831-669, p=0.00001,   0 draws
        // 245 draws eliminated and 125 wins recovered, from one binary rule.
        // And with it on, the ability-refund path stopped being a regression:
        //   refund=1.0, gate off : 752-448
        //   refund=1.0, gate on  : 916-584  (baseline 910-590, p=0.61)
        // i.e. the first ability intervention that is not worse than v7.
        //
        // Default ON, and it is a no-op on current play (910-590 either way)  E        // it changes nothing until something actually pushes the bot to spend
        // an ability, and then it is the only thing standing between that and
        // 200 draws. Deliberately untunable: the question has a yes/no answer.
        if ability_noop_gate && action.action_type == ActionType::UseAbility {
            let mut again = sim.clone();
            crate::rng::seed(0x7637);
            let mut noop = false;
            if game_setup::execute_action(&mut again, action).is_ok() {
                noop = again.generate_state_hash() == sim.generate_state_hash();
            }
            if noop {
                score = score.min(-1000.0);
            }
        }
        // QUALITATIVE ABILITY GATE  Eread off the printed card, no coefficient.
        //
        // Replaces `V7_ABILITY_CLASS`, which was tuning slop: a per-class
        // weight swept at 0.25/0.5/1.0, all three regressions, and still a
        // regression with the draw gate on (831-669 vs 910-590, p=0.00001).
        // The card already says what the ability IS, so the decision is read
        // from the trigger marker instead of fitted to the arena:
        //
        //   {{常晁E} / {{自動}} / {{常駐}}  automatic  Eresolves on its own,
        //     so spending a member on it is a category error. v7's favourite
        //     ability, 46 of 65 uses, is in this family.
        //   {{起動}}                    activation  Ethe only choosable kind.
        //     Spend it when it produces blades, hearts, a draw or a search.
        //
        // On by default, and the switch is binary: `V7_ABILITY_QUAL=0`.
        if ability_qualitative && action.action_type == ActionType::UseAbility {
            let card_id = action
                .parameters
                .as_ref()
                .and_then(|p| p.card_id)
                .unwrap_or(-1);
            let class = if card_id >= 0 {
                crate::bot::strategy_v7::best_ability_class(&gs.card_database, card_id)
            } else {
                crate::bot::strategy_v7::AbilityClass::Inactive
            };
            if !crate::bot::v7_ismcts::ability_is_worth_spending(class) {
                score = score.min(-1000.0);
            }
        }
        if ability_class_gate > 0.0 && action.action_type == ActionType::UseAbility {
            let card_id = action
                .parameters
                .as_ref()
                .and_then(|p| p.card_id)
                .or_else(|| action.parameters.as_ref().and_then(|p| p.card_index).map(|_| -1))
                .unwrap_or(-1);
            let class = if card_id >= 0 {
                crate::bot::strategy_v7::best_ability_class(&gs.card_database, card_id)
            } else {
                crate::bot::strategy_v7::AbilityClass::Inactive
            };
            score += ability_class_gate * crate::bot::v7_ismcts::class_value(class);
        }
        if ability_forecast_weight > 0.0 && action.action_type == ActionType::UseAbility {
            let after_forecast = crate::bot::strategy_v7::placement_forecast(
                &sim,
                me,
                &gs.card_database,
                forecast_depth,
            );
            score += ability_forecast_weight * 0.5 * (after_forecast - base_forecast);
            if std::env::var_os("V7_DEBUG").is_some() {
                // Name what the ability did, so "why did it use that" is
                // answerable from the trace instead of guessed at.
                let delta =
                    crate::bot::strategy_v7::measure_ability(&sim, &root, me, &gs.card_database);
                eprintln!(
                    "V7AB t={} me={} kind={} draws={} en={} bl={} h={} ammo={} cost={} d_forecast={:.3}",
                    gs.turn_number,
                    me,
                    delta.classify(),
                    delta.draws,
                    delta.energy,
                    delta.blades,
                    delta.hearts,
                    delta.ammo,
                    delta.stage_cost,
                    after_forecast - base_forecast,
                );
            }
        }
        if curve_debug && matches!(gs.turn_number, 1 | 2) && action.action_type != ActionType::Pass {
            // The opening curve is the one place with a guide number attached
            // (4 -> 9 -> 13) and it is behind: the bot sits at 3.8 / 7.6 / 11.2.
            // This answers the question that decides where to look  Eis it not
            // REACHING the band, or not being OFFERED the cards? If a cost-4
            // is in hand and affordable and v7 passes over it, this is a
            // Main-phase ranking bug. If no cost-4 is ever in hand and
            // affordable, it is a mulligan or deck problem and no amount of
            // search downstream will fix it.
            let me_p = gs.seat_player(me);
            let hand: Vec<String> = me_p
                .hand
                .cards
                .iter()
                .filter_map(|&cid| gs.card_database.get_card(cid))
                .map(|card| {
                    format!(
                        "{}{}:{}",
                        card.card_no,
                        card.cost.map(|c| c.to_string()).unwrap_or_else(|| "-".into()),
                        card.name.chars().take(3).collect::<String>()
                    )
                })
                .collect();
            eprintln!(
                "V7CURVE t={} me={} act={:?} energy={} stage={:?} hand=[{}]",
                gs.turn_number,
                me,
                action.action_type,
                me_p.energy_zone.active_count(),
                me_p.stage.stage,
                hand.join(" ")
            );
        }
        if forecast_weight > 0.0 {
            let after = crate::bot::strategy_v7::placement_forecast(
                &sim,
                me,
                &gs.card_database,
                forecast_depth,
            );
            // Scaled out of forecast units: the forecast spans roughly 0..2, so
            // divide by 2 to keep it in the same neighbourhood as the other
            // terms before the weight is applied.
            score += forecast_weight * 0.5 * (after - base_forecast);
        }
        let explanation = format!(
            "value={score:.2} choices={} depth={depth} status={status}{}",
            search.nodes,
            if baton { " [BATON]" } else { "" }
        );
        if std::env::var_os("V7_DEBUG").is_some() {
            let (_, blades, cost) = board(&root, me);
            let card = action_card_label(gs, action);
            let area = action
                .parameters
                .as_ref()
                .and_then(|p| p.stage_area.clone())
                .unwrap_or_else(|| "-".into());
            let fc = action
                .parameters
                .as_ref()
                .and_then(|p| p.final_cost)
                .map(i32::from)
                .unwrap_or(-1);
            eprintln!(
                "V7M t={} me={} act={} card={} area={} fc={} {} st={} bl={} h={} en={} hand={} ammo={} succ={}",
                gs.turn_number, me, action.action_type, card, area, fc, explanation,
                cost, blades, base.hearts, base.energy, base.hand, base.ammo, base.success,
            );
        }
        scores.push((score, explanation));
    }
    if std::env::var_os("V7_DEBUG").is_some() {
        let best = pick_best(gs, me, actions, &scores);
        let chosen = actions.get(best);
        let card = chosen
            .map(|a| action_card_label(gs, a))
            .unwrap_or_else(|| "-".into());
        eprintln!(
            "V7CH t={} me={} act={} card={} score={:.2} note={}",
            gs.turn_number,
            me,
            chosen.map(|a| a.action_type).unwrap_or(ActionType::Pass),
            card,
            scores
                .get(best)
                .map(|(s, _)| *s)
                .unwrap_or(f64::NEG_INFINITY),
            scores.get(best).map(|(_, n)| n.as_str()).unwrap_or(""),
        );
    }
    scores
}

/// v7's main-phase decision WITH the ISMCTS refinement (the product bot).
pub fn choose_action(gs: &GameState, actions: &[Action], me: u8) -> Action {
    choose_action_impl(gs, actions, me, None)
}

/// v7's main-phase decision with the ISMCTS refinement forced OFF.
///
/// This is the arena's `v7plain` ablation seat. It exists so a mirror
/// `v7 vs v7plain` measures the search and nothing else: same build, same
/// heuristics, same live set, one variable.
pub fn choose_action_plain(gs: &GameState, actions: &[Action], me: u8) -> Action {
    choose_action_impl(gs, actions, me, Some(false))
}

fn choose_action_impl(
    gs: &GameState,
    actions: &[Action],
    me: u8,
    forced: Option<bool>,
) -> Action {
    let scores = score_actions(gs, actions, me);
    // ISMCTS refinement. The static scores above are the prior; the search
    // promotes a move only when the determinized rollouts beat it, and every
    // failure path (disabled, nothing to choose, proven win, non-finite
    // scores) returns these same scores, so this can only add signal.
    let scores = crate::bot::v7_ismcts::refine_scores(gs, actions, me, &scores, forced);
    let best = pick_best(gs, me, actions, &scores);
    let chosen = actions.get(best).cloned().unwrap_or(Action {
        action_type: ActionType::Pass,
        description: "pass".into(),
        description_ja: None,
        parameters: None,
        selected: None,
    });
    if std::env::var_os("V7_ROLLOUT_ANALYZE").is_some() {
        let simulations = std::env::var("V7_ROLLOUT_SIMS")
            .ok()
            .and_then(|value| value.parse().ok())
            .filter(|value: &usize| *value > 0)
            .unwrap_or(100);
        let horizons = [1u8, 2, 3, 5, 10];
        let values = crate::bot::rollout::price_main_actions_at_horizons(
            gs,
            me,
            actions,
            simulations,
            &horizons,
        );
        for (horizon_index, horizon) in horizons.iter().enumerate() {
            let (best, _) = values[horizon_index]
                .iter()
                .enumerate()
                .max_by(|(_, left), (_, right)| left.total_cmp(right))
                .unwrap_or((0, &0.0));
            let best_card = actions
                .get(best)
                .and_then(|a| a.parameters.as_ref())
                .and_then(|p| {
                    p.card_id
                        .map(|id| id.to_string())
                        .or_else(|| p.card_no.clone())
                })
                .unwrap_or_else(|| "-".into());
            eprintln!(
                "V7HORIZON t={} me{} horizon={} best={} card={} value={:.2}",
                gs.turn_number,
                me,
                horizon,
                actions
                    .get(best)
                    .map(|a| a.action_type)
                    .unwrap_or(ActionType::Pass),
                best_card,
                values[horizon_index][best]
            );
        }
    }
    if std::env::var_os("V7_ROLLOUT_COMPARE").is_some() {
        if let Some(suggested) = rollout_suggestion(gs, actions, me) {
            let suggested_card = suggested
                .parameters
                .as_ref()
                .and_then(|p| {
                    p.card_id
                        .map(|id| id.to_string())
                        .or_else(|| p.card_no.clone())
                })
                .unwrap_or_else(|| "-".into());
            let chosen_card = chosen
                .parameters
                .as_ref()
                .and_then(|p| {
                    p.card_id
                        .map(|id| id.to_string())
                        .or_else(|| p.card_no.clone())
                })
                .unwrap_or_else(|| "-".into());
            eprintln!(
                "V7ISM t={} me{} chosen={}:{} rollout={}:{}",
                gs.turn_number,
                me,
                chosen.action_type,
                chosen_card,
                suggested.action_type,
                suggested_card
            );
        }
    }
    log::debug!(
        "v7_main chosen t{} me{} {:?} params={:?} score={:.2} note={}",
        gs.turn_number,
        me,
        chosen.action_type,
        chosen.parameters,
        scores
            .get(best)
            .map(|(s, _)| *s)
            .unwrap_or(f64::NEG_INFINITY),
        scores.get(best).map(|(_, n)| n.as_str()).unwrap_or(""),
    );
    chosen
}
