use crate::bot::strategy_common::{acc_add, requirements_met, Acc};
use crate::card::CardType;
use crate::core::stats_pipeline;
use crate::game_setup::{self, Action, ActionType};
use crate::game_state::{GameResult, GameState, Phase};


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
        success: p.success_live_card_zone.cards.len(),
    }
}

fn value(now: &Features, base: &Features, deploy: bool) -> f64 {
    let mut score = 8.0 * (now.cost - base.cost) as f64
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
        if gs.game_result != GameResult::Ongoing || !gs.has_pending_choice() {
            return (self.evaluate(gs), depth, "complete");
        }
        if !gs.can_player_act(self.me as i32) {
            return (self.evaluate(gs), depth, "opponent-choice");
        }
        if depth >= 8 || self.nodes >= 64 {
            return (self.evaluate(gs) - 1.0, depth, "choice-budget");
        }
        let mut best = (f64::NEG_INFINITY, depth, "execution-error");
        for action in game_setup::generate_possible_actions(gs) {
            if self.nodes >= 64 { break; }
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
    opp.hand.cards.fill(-1);
    opp.main_deck.cards.fill(-1);
    if matches!(gs.current_phase, Phase::Main | Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker) {
        opp.live_card_zone.cards.fill(-1);
    }
    sim
}

pub fn score_actions(gs: &GameState, actions: &[Action], me: u8) -> Vec<(f64, String)> {
    let _rng = RngGuard(crate::rng::checkpoint());
    let root = simulation_state(gs, me);
    let base = features(&root, me);
    let opponent = features(&root, 1 - me);
    let pending = gs.has_pending_choice();
    let mut scores = Vec::with_capacity(actions.len());
    for action in actions {
        if action.parameters.as_ref().and_then(|p| p.disabled) == Some(true) {
            scores.push((f64::NEG_INFINITY, "disabled".into()));
            continue;
        }
        if action.action_type == ActionType::Pass && !pending {
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
        if !pending && action.action_type == ActionType::UseAbility && score <= 0.0 {
            score -= 0.01;
        }
        let explanation = format!("value={score:.2} choices={} depth={depth} status={status}", search.nodes);
        if std::env::var_os("V7_DEBUG").is_some() {
            eprintln!("V7M t{} me{} {} {:?} {}", gs.turn_number, me, action.action_type, action.parameters, explanation);
        }
        scores.push((score, explanation));
    }
    scores
}

pub fn choose_action(gs: &GameState, actions: &[Action], me: u8) -> Action {
    let scores = score_actions(gs, actions, me);
    let mut best = 0;
    for i in 1..scores.len() {
        if scores[i].0 > scores[best].0 { best = i; }
    }
    actions.get(best).cloned().unwrap_or(Action {
        action_type: ActionType::Pass, description: "pass".into(),
        description_ja: None, parameters: None, selected: None,
    })
}
