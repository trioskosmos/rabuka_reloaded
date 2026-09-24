use crate::bot::determinization::DeterminizationSampler;
use crate::bot::observation::PublicObservation;
use crate::bot::strategy_common::{acc_add, requirements_met, Acc};
use crate::card::CardType;
use crate::core::stats_pipeline;
use crate::game_setup::{self, Action, ActionType, ActionParameters};
use crate::game_state::{GameResult, GameState, Phase};

/// D1: baton vision from action params, not the unset `use_baton_touch`.
/// A PlayMemberToStage targeting an occupied stage slot (or a double-baton
/// `card_indices` pair, or an `available_areas` entry flagged
/// `is_baton_touch` for the chosen `stage_area`) IS a baton touch.
/// Generator never sets `use_baton_touch` (link.rs matches on it); occupied
/// target ⇒ baton is the generator's own rule (game_setup.rs:1504-1541).
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
        return p.available_areas.as_ref().is_some_and(|areas| {
            areas.iter().any(|area| area.is_baton_touch)
        });
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
        &p.stage.stage, &gs.card_database, &gs.mods.heart_override,
        &gs.mods.heart_copy, &gs.mods.heart_color_multiplier, &gs.mods.heart_modifiers,
    );
    let mut pool = [0; 11];
    acc_add(&mut pool, &hearts.hearts);
    let mut blades = 0;
    let mut cost = 0;
    for &id in &p.stage.stage {
        if id < 0 { continue; }
        cost += gs.card_database.get_card(id).and_then(|c| c.cost).unwrap_or(0) as i32;
        if gs.mods.get_orientation_modifier(id) != Some("wait") {
            blades += i32::from(stats_pipeline::effective_blade(
                &gs.card_database, id,
                gs.mods.blade_modifiers.get(&id).cloned().unwrap_or_default(),
            ));
        }
    }
    (pool, blades, cost)
}

fn member_reserve(gs: &GameState, me: u8, id: i16) -> f64 {
    let p = gs.seat_player(me);
    let Some(card) = gs.card_database.get_card(id) else { return 0.0; };
    if card.card_type != CardType::Member { return 0.0; }
    let cost = i32::from(card.cost.unwrap_or(0));
    let discount = p.stage.stage.iter().filter_map(|&cid| gs.card_database.get_card(cid))
        .filter_map(|c| c.cost).max().unwrap_or(0) as i32;
    let budget = p.energy_zone.cards.len() as i32 + 1;
    let delay = (cost - discount - budget).max(0) as f64;
    let hearts = card.base_heart.as_ref().map(|h| h.hearts.values_sum()).unwrap_or(0);
    (2.0 * cost as f64 + 2.0 * card.blade as f64 + hearts as f64) / (1.0 + delay)
}

fn future_development(gs: &GameState, me: u8) -> f64 {
    let p = gs.seat_player(me);
    let db = &gs.card_database;
    let budget = usize::from(p.energy_zone.active_count()) + 1;
    let stage_discount = p.stage.stage.iter()
        .filter_map(|&cid| db.get_card(cid))
        .filter_map(|card| card.cost)
        .max()
        .unwrap_or(0) as usize;
    p.hand.cards.iter()
        .filter_map(|&cid| db.get_card(cid))
        .filter(|card| card.card_type == CardType::Member)
        .filter_map(|card| card.cost)
        .filter(|&cost| usize::from(cost) <= budget + stage_discount)
        .map(|cost| cost as f64)
        .fold(0.0, f64::max)
}

#[derive(Clone)]
struct Features {
    hearts: i32,
    blades: i32,
    cost: i32,
    ammo: usize,
    coverage: f64,
    hand: usize,
    energy: usize,
    reserve: f64,
    future: f64,
    success: usize,
}

fn features(gs: &GameState, me: u8) -> Features {
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
        mean[i] += (units[i] * blades as f64 / p.main_deck.cards.len().max(1) as f64).floor() as i32;
    }
    let mut ammo = 0;
    let mut coverage = 0.0;
    let mut reserves = Vec::new();
    for &id in &p.hand.cards {
        let Some(card) = gs.card_database.get_card(id) else { continue; };
        if card.card_type == CardType::Live {
            ammo += 1;
            let need = stats_pipeline::effective_need_heart(card.need_heart.as_ref(), id, &gs.mods.need_heart_modifiers);
            let mut required = [0; 11];
            if let Some(need) = need { acc_add(&mut required, &need.hearts); }
            if requirements_met(&mean, &required) { coverage += 1.0; }
        } else if card.card_type == CardType::Member {
            reserves.push(member_reserve(gs, me, id));
        }
    }
    reserves.sort_by(|a, b| b.total_cmp(a));
    Features {
        hearts: pool.iter().enumerate().filter(|(i, _)| *i != 8 && *i != 9).map(|(_, v)| v).sum(),
        blades, cost, ammo, coverage, hand: p.hand.cards.len(),
        energy: usize::from(p.energy_zone.active_count()),
        reserve: reserves.iter().take(2).sum(),
        future: future_development(gs, me),
        success: p.success_live_card_zone.cards.len(),
    }
}

fn value(now: &Features, base: &Features, deploy: bool) -> f64 {
    // Development weight: default 8.0 (historical). V7_DEV_WEIGHT for
    // ablation — losses still cluster on dev_gap (audit 2026-09-23).
    let dev_w: f64 = std::env::var("V7_DEV_WEIGHT")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|w: &f64| w.is_finite() && *w >= 0.0)
        .unwrap_or(8.0);
    let mut score = dev_w * (now.cost - base.cost) as f64
        + 3.0 * (now.hearts - base.hearts) as f64
        + 6.0 * (now.blades - base.blades) as f64
        + 60.0 * (now.coverage - base.coverage)
        + 25.0 * (now.ammo as f64 - base.ammo as f64)
        + 1000.0 * (now.success as f64 - base.success as f64);
    if !deploy {
        score += 2.0 * (now.energy as f64 - base.energy as f64)
            + 3.0 * (now.hand as f64 - base.hand as f64)
            + 0.5 * (now.reserve - base.reserve);
    }
    let future_enabled = std::env::var_os("V7_FUTURE").is_none()
        || std::env::var("V7_FUTURE").ok().as_deref() == Some("1");
    let future_weight: f64 = std::env::var("V7_FUTURE_WEIGHT")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value: &f64| value.is_finite() && *value >= 0.0)
        .unwrap_or(2.0);
    if future_enabled {
        score += (now.future - base.future) * future_weight;
    }
    if now.ammo == 0 && base.ammo > 0 { score -= 120.0; }
    if now.hand <= 1 && base.hand > 1 { score -= 60.0; }
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

impl Search<'_> {
    /// Node budget per root action. Default 64 (historical); V7_NODES env
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
        let own = features(gs, self.me);
        let opp = gs.seat_player(1 - self.me);
        let original_opp = self.root.seat_player(1 - self.me);
        let (_, opp_blades, opp_cost) = board(gs, 1 - self.me);
        value(&own, &self.base, self.deploy)
            - 8.0 * (opp_cost - self.opponent.cost) as f64
            - 6.0 * (opp_blades - self.opponent.blades) as f64
            - 10.0 * (opp.hand.cards.len() as f64 - original_opp.hand.cards.len() as f64)
            - 1000.0 * (opp.success_live_card_zone.cards.len() as f64 - self.opponent.success as f64)
    }

    fn complete(&mut self, gs: &GameState, depth: usize) -> (f64, usize, &'static str) {
        if gs.game_result != GameResult::Ongoing {
            return (self.evaluate(gs), depth, "complete");
        }
        if !gs.has_pending_choice() {
            let beam = std::env::var_os("V7_BEAM").is_some();
            if beam
                && depth < 1
                && gs.current_phase == Phase::Main
                && gs.can_player_act(self.me as i32)
                && self.nodes < Self::node_budget()
            {
                let mut best = (f64::NEG_INFINITY, depth, "beam-empty");
                for action in game_setup::generate_possible_actions(gs) {
                    if self.nodes >= Self::node_budget() { break; }
                    if action.parameters.as_ref().and_then(|p| p.disabled) == Some(true) { continue; }
                    self.nodes += 1;
                    crate::rng::seed(0x7637 + depth as u32);
                    let mut next = gs.clone();
                    if game_setup::execute_action(&mut next, &action).is_err() { continue; }
                    let candidate = self.complete(&next, depth + 1);
                    if candidate.0 > best.0 { best = candidate; }
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
            return (self.evaluate(gs) - 1.0, depth, "choice-budget");
        }
        let mut best = (f64::NEG_INFINITY, depth, "execution-error");
        for action in game_setup::generate_possible_actions(gs) {
            if self.nodes >= Self::node_budget() { break; }
            if action.parameters.as_ref().and_then(|p| p.disabled) == Some(true) { continue; }
            self.nodes += 1;
            crate::rng::seed(0x7637 + depth as u32);
            let mut next = gs.clone();
            if game_setup::execute_action(&mut next, &action).is_err() { continue; }
            let candidate = self.complete(&next, depth + 1);
            if candidate.0 > best.0 { best = candidate; }
        }
        best
    }
}

struct RngGuard(u32);
impl Drop for RngGuard {
    fn drop(&mut self) { crate::rng::restore(self.0); }
}

fn simulation_state(gs: &GameState, me: u8) -> GameState {
    let mut sim = gs.clone();
    let (own, opp) = if me == 0 { (&mut sim.player1, &mut sim.player2) } else { (&mut sim.player2, &mut sim.player1) };
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
        if matches!(gs.current_phase, Phase::Main | Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker) {
            opp.live_card_zone.cards = sampled_opp.live_card_zone.cards.clone();
        }
    } else {
        opp.hand.cards.fill(-1);
        opp.main_deck.cards.fill(-1);
        if matches!(gs.current_phase, Phase::Main | Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker) {
            opp.live_card_zone.cards.fill(-1);
        }
    }
    sim
}

/// Best-index selection with a baton-over-Pass tie-break.
///
/// Pass is always `actions[0]` and a free equal-stat baton swap scores
/// exactly 0.00 (Δcost=Δhearts=Δblades=0), so strict `>` kept Pass on every
/// 0–0 tie → empty-main spiral (seed 42: t3/t4/t6, empty Main 7/20=35%).
/// On an exact finite tie, prefer a baton `PlayMemberToStage` over Pass.
/// Pass still wins ties against non-baton 0s (anti-clog for empty-slot bodies).
fn is_free_baton(gs: &GameState, me: u8, action: &Action) -> bool {
    if !is_baton_action(gs, me, action) {
        return false;
    }
    action.parameters.as_ref().is_some_and(|params| {
        params.final_cost == Some(0)
            || params.available_areas.as_ref().is_some_and(|areas| {
                areas.iter().any(|area| area.is_baton_touch && area.cost == 0)
            })
    })
}

fn pick_best(gs: &GameState, me: u8, actions: &[Action], scores: &[(f64, String)]) -> usize {
    let mut best = 0usize;
    for i in 1..scores.len() {
        let si = scores[i].0;
        let sb = scores[best].0;
        if si > sb {
            best = i;
        } else if si.is_finite()
            && actions[best].action_type == ActionType::Pass
            && actions[i].action_type != ActionType::Pass
            && is_baton_action(gs, me, &actions[i])
            && (si == sb || (std::env::var_os("V7_FREE_BATON").is_some()
                && is_free_baton(gs, me, &actions[i])))
        {
            log::debug!(
                "v7_main baton-tie-break t{} me{} idx{} {:?} score={:.2} beats Pass",
                gs.turn_number, me, i, actions[i].action_type, si
            );
            best = i;
        }
    }
    best
}

pub fn score_actions(gs: &GameState, actions: &[Action], me: u8) -> Vec<(f64, String)> {
    let _rng = RngGuard(crate::rng::checkpoint());
    let root = simulation_state(gs, me);
    let base = features(&root, me);
    let opponent = features(&root, 1 - me);
    let pending = gs.has_pending_choice();
    // MEASURED 2026-09-23: Pass=-0.05 + energy tax exploded draws
    // (1569/3000, avg turns 3.7) — every UseAbility at score≈0 then beat
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
        let mut search = Search {
            base: base.clone(), opponent: opponent.clone(), root: &root, me,
            deploy: action.action_type == ActionType::PlayMemberToStage, nodes: 0,
        };
        let (mut score, depth, status) = search.complete(&sim, 0);
        // D1 baton vision: DETECT for logging/[BATON] marks, but do NOT add a
        // flat bonus. MEASURED 2026-09-23 (seed 11, 3000×2): +45 → 39.5% vs
        // v6 (folds 3.7%); +12 with ammo-guard → 50.5%; flat 0 → baseline.
        // The `value()` development term (8× stage-cost growth) already prices
        // discounted upgrades — a flat baton bonus double-counts and over-dumps
        // the hand (live-set folds 0.3%→4.1% at +45).
        let baton = is_baton_action(gs, me, action);
        if !pending && action.action_type == ActionType::UseAbility && score <= 0.0 {
            score -= 0.01;
        }
        let explanation = format!(
            "value={score:.2} choices={} depth={depth} status={status}{}",
            search.nodes,
            if baton { " [BATON]" } else { "" }
        );
        if std::env::var_os("V7_DEBUG").is_some() {
            let (_, blades, cost) = board(&root, me);
            let card = action
                .parameters
                .as_ref()
                .and_then(|p| p.card_no.clone())
                .unwrap_or_else(|| "-".into());
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
            .and_then(|a| a.parameters.as_ref())
            .and_then(|p| p.card_no.clone())
            .unwrap_or_else(|| "-".into());
        eprintln!(
            "V7CH t={} me={} act={} card={} score={:.2} note={}",
            gs.turn_number,
            me,
            chosen.map(|a| a.action_type).unwrap_or(ActionType::Pass),
            card,
            scores.get(best).map(|(s, _)| *s).unwrap_or(f64::NEG_INFINITY),
            scores.get(best).map(|(_, n)| n.as_str()).unwrap_or(""),
        );
    }
    scores
}

pub fn choose_action(gs: &GameState, actions: &[Action], me: u8) -> Action {
    let scores = score_actions(gs, actions, me);
    let best = pick_best(gs, me, actions, &scores);
    let chosen = actions.get(best).cloned().unwrap_or(Action {
        action_type: ActionType::Pass, description: "pass".into(),
        description_ja: None, parameters: None, selected: None,
    });
    log::debug!(
        "v7_main chosen t{} me{} {:?} params={:?} score={:.2} note={}",
        gs.turn_number,
        me,
        chosen.action_type,
        chosen.parameters,
        scores.get(best).map(|(s, _)| *s).unwrap_or(f64::NEG_INFINITY),
        scores.get(best).map(|(_, n)| n.as_str()).unwrap_or(""),
    );
    chosen
}
