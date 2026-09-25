//! V8 Main-phase decision: bounded search whose leaves are measured in
//! PLACEMENTS, not in counter deltas.
//!
//! v7's Main phase scored a one-ply aggregate: `8.0 × Δstage-cost + 3.0 ×
//! Δhearts + 6.0 × Δblades + 60 × Δpassable + 25 × Δammo` — ten interacting
//! weights with no objective behind them. The project's own post-mortem names
//! why that cannot be fixed by tuning: "an action that enables a better next
//! action can look worse than a marginal immediate action", and "no heuristic
//! weight can fix this. The information 'this action enables a winning line'
//! does not exist in any single resulting state."
//!
//! v8 changes what a leaf IS. Every leaf is scored by the shared check model
//! as an expected PLACEMENT ADVANTAGE across the next check and the one after:
//!
//! ```text
//! leaf = w · [ (P_us − P_them) now
//!            + (P_us after next turn's best development − P_us now) ]
//!       + w · PLACEMENT_CREDIT · (lives in hand)
//! ```
//!
//! Consequences that fall out of the formulation instead of being patched:
//!
//! - **A sideways baton upgrade scores correctly.** It may not change this
//!   check, but it raises the next turn's placement probability, which is the
//!   second term. No flat baton bonus is needed — v7 measured `+45 baton` at
//!   39.5% (catastrophic) precisely because a flat bonus double-counts
//!   development instead of pricing the trajectory.
//! - **Empty-Main spirals are impossible by construction.** `Pass` is scored by
//!   the SAME function on the SAME state, so it can never win a tie it did not
//!   earn. The one deliberate tiebreak prefers development (a higher forward
//!   ceiling, then a baton) and still lets `Pass` win against a
//!   zero-contribution member, preserving the anti-clog rule with no special
//!   case.
//! - **Energy hoarding is not priced at all.** Unspent energy has no term; it
//!   matters only through the forward projection, which is exactly the guides'
//!   "bank only when no upgrade is reachable" (§4). v7's Pass-tax experiment
//!   exploded draws and had to be reverted; there is nothing to tax here.
//! - **Ability engines are priced.** A draw is worth the life it finds
//!   (`PLACEMENT_CREDIT`) and board buffs are read buff-aware, so an activation
//!   is no longer invisible (v7's "ability blindness").
//!
//! Depth is two: after the root action the search re-offers the position and
//! takes the best follow-up, which is what makes "set up an ability, then
//! deploy at a discount" visible as a line rather than as two mediocre moves.

use crate::bot::strategy_common::{acc_add, Acc};
use crate::card::CardDatabase;
use crate::game_setup::{self, Action, ActionType};
use crate::game_state::{GameState, Phase};

use super::v8_model::{self, OppModel, RngGuard, PLACEMENT_CREDIT};

/// Scale applied to placement probabilities so leaf values read like the
/// percentages the guides talk in. Cosmetic: a common factor on every term,
/// so it cannot change a decision.
const SCALE: f64 = 1000.0;

fn node_budget() -> usize {
    std::env::var("V8_NODES")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|n: &usize| *n > 0)
        .unwrap_or(96)
}

fn followup_width() -> usize {
    std::env::var("V8_FOLLOWUP")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|n: &usize| *n > 0)
        .unwrap_or(6)
}

/// Value of attacking first next check. This is the ONE constant in v8 that is
/// not derived from a rule, so it is isolated behind its own switch and
/// ablated: the guides are genuinely split (S6 wants the turn-order win of
/// 8.4.13, and also wants the second attacker's information advantage), so the
/// honest thing is to measure it rather than argue about it.
fn initiative_weight() -> f64 {
    if std::env::var_os("V8_NO_INITIATIVE").is_some() {
        return 0.0;
    }
    std::env::var("V8_INITIATIVE_WEIGHT")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|w: &f64| w.is_finite() && *w >= 0.0)
        .unwrap_or(0.5)
}

/// The best member we could put on stage next turn. Used only to project the
/// leaf one turn forward; the real deploy is chosen by the depth-2 search, so
/// this is a smooth estimate of "how good could next turn be", not a
/// commitment.
struct Projection {
    hearts: Acc,
    blades: i32,
}

fn project_one_turn(gs: &GameState, me: u8, db: &CardDatabase) -> Projection {
    let p = gs.seat_player(me);
    // Next turn's budget: this phase's energy plus the guaranteed +1 from the
    // coming Energy phase (7.5), plus the current baton discount
    // (9.6.2.3.2) when the stage already has a member to send.
    let discount = p
        .stage
        .stage
        .iter()
        .filter(|&&c| c >= 0)
        .filter_map(|&c| db.get_card(c).and_then(|card| card.cost))
        .map(i32::from)
        .max()
        .unwrap_or(0);
    let budget = i32::from(p.energy_zone.active_count()) + 1 + discount;
    let best = p
        .hand
        .cards
        .iter()
        .filter_map(|&cid| db.get_card(cid))
        .filter(|card| card.card_type == crate::card::CardType::Member)
        .filter(|card| i32::from(card.cost.unwrap_or(0)) <= budget)
        .max_by_key(|card| card.cost.unwrap_or(0));
    let mut hearts = [0i32; 11];
    let mut blades = 0;
    if let Some(card) = best {
        if let Some(base) = &card.base_heart {
            acc_add(&mut hearts, &base.hearts);
        }
        blades = i32::from(card.blade);
    }
    Projection { hearts, blades }
}

/// `P(we place a card at the next check)` and `P(they place)`, given a board
/// supply and an active-blade count.
fn placement_pair(
    gs: &GameState,
    me: u8,
    db: &CardDatabase,
    opp: &OppModel,
    supply: &Acc,
    blades: i32,
    density: f64,
) -> (f64, f64) {
    let my_success = gs.seat_player(me).success_live_card_zone.cards.len();
    let yell = v8_model::expected_yell_score(gs, me, db, blades);
    // Best single life we would set: highest placement probability, score as
    // the tiebreak. The contested swing for score is priced where a real
    // opponent model exists (the live-set decision), so the Main phase only
    // needs to know whether a placement is available at all.
    let mut best: Option<(f64, i32, f64)> = None;
    for &cid in &gs.seat_player(me).hand.cards {
        let Some(card) = db.get_card(cid) else {
            continue;
        };
        if card.card_type != crate::card::CardType::Live {
            continue;
        }
        let need = v8_model::life_need(gs, cid);
        if v8_model::has_unpassable_icon(&need) {
            continue;
        }
        let p_pass = v8_model::pass_estimate_from_supply(supply, blades, density, &need);
        let score = v8_model::printed_score(db, cid) + yell;
        let outcome = v8_model::check_outcome(p_pass, score, 1, my_success, opp);
        let entry = (outcome.p_place, score, p_pass);
        match best {
            Some((best_place, best_score, _)) if (best_place, best_score) >= (entry.0, entry.1) => {}
            _ => best = Some(entry),
        }
    }
    let Some((our_place, our_score, p_pass)) = best else {
        // No life in hand: we place nothing and they place whenever they pass.
        return (0.0, opp.pass_prob);
    };
    let (_we_c, they_c, _) = opp.contested_masses(our_score, 1, my_success);
    let contested = p_pass * opp.pass_prob;
    let opp_place = opp.pass_prob * (1.0 - p_pass) + contested * they_c;
    (our_place.min(1.0), opp_place.min(1.0))
}

/// Expected placement advantage of a position, in placement units.
fn leaf_value(gs: &GameState, me: u8, db: &CardDatabase, opp: &OppModel) -> f64 {
    let supply = v8_model::board_supply(gs, me, db);
    let (blades, density) = super::strategy_v4::flip_stats(gs, me, db);
    let (our_now, their_now) = placement_pair(gs, me, db, opp, &supply, blades, density);

    // One turn forward, after the best development we could make. This is the
    // trajectory term: it prices an action whose payoff lands on the NEXT
    // check rather than this one, which is exactly the "investment" deploy
    // class v7's one-ply eval could not see (its D2 residual).
    let projection = project_one_turn(gs, me, db);
    let mut projected = supply;
    for i in 0..11 {
        projected[i] += projection.hearts[i];
    }
    let (our_next, _) = placement_pair(
        gs,
        me,
        db,
        opp,
        &projected,
        blades + projection.blades,
        density,
    );

    let ammo = v8_model::lives_in_hand(gs.seat_player(me), db) as f64;
    let initiative = if gs.seat_player(me).is_first_attacker {
        initiative_weight()
    } else {
        0.0
    };

    SCALE * ((our_now - their_now) + (our_next - our_now) + PLACEMENT_CREDIT * (ammo + initiative))
}

/// Baton detection from the generated action's own destination data. The
/// generator never sets `use_baton_touch` (that flag only lives in
/// `available_areas[i].is_baton_touch` and in the double-baton `card_indices`),
/// so the occupied target is the real signal — the same rule as
/// `v7_main::is_baton_action`.
pub(crate) fn is_baton(gs: &GameState, me: u8, action: &Action) -> bool {
    if action.action_type != ActionType::PlayMemberToStage {
        return false;
    }
    let Some(params) = action.parameters.as_ref() else {
        return false;
    };
    if params.use_baton_touch == Some(true) || params.card_indices.is_some() {
        return true;
    }
    let Some(area_name) = params.stage_area.as_deref() else {
        return params
            .available_areas
            .as_ref()
            .is_some_and(|areas| areas.iter().any(|area| area.is_baton_touch));
    };
    if let Some(areas) = params.available_areas.as_ref() {
        if areas
            .iter()
            .any(|area| area.area == area_name && area.is_baton_touch)
        {
            return true;
        }
    }
    let Ok(area) = area_name.parse::<crate::zones::MemberArea>() else {
        return false;
    };
    gs.seat_player(me)
        .stage
        .stage
        .get(area.to_index())
        .copied()
        .unwrap_or(-1)
        != -1
}

/// Tie-break key: a higher forward ceiling is better, and a baton ladder step
/// beats an equivalent fresh play (the guides' 4→9→13). The only place v8
/// expresses a preference beyond the model, and it exists so a free
/// development step is never discarded on a rounding tie.
fn development_key(gs: &GameState, me: u8, db: &CardDatabase, action: &Action) -> (i32, u8) {
    (
        v8_model::reachable_ceiling(gs, me, 2, db),
        u8::from(is_baton(gs, me, action)),
    )
}

struct Search<'a> {
    me: u8,
    db: &'a CardDatabase,
    opp: OppModel,
    nodes: usize,
    budget: usize,
}

impl Search<'_> {
    fn leaf(&mut self, gs: &GameState) -> f64 {
        self.nodes += 1;
        leaf_value(gs, self.me, self.db, &self.opp)
    }

    /// Best value of `sim` plus one more of OUR actions: the depth-2 step that
    /// makes ability-then-deploy lines visible.
    fn with_followup(&mut self, sim: &GameState) -> f64 {
        let base = self.leaf(sim);
        if sim.current_phase != Phase::Main
            || sim.has_pending_choice()
            || !sim.can_player_act(self.me as i32)
        {
            return base;
        }
        let mut followups: Vec<Action> = game_setup::generate_possible_actions(sim)
            .into_iter()
            .filter(|a| a.parameters.as_ref().and_then(|p| p.disabled) != Some(true))
            .filter(|a| a.action_type != ActionType::Pass)
            .collect();
        // A deploy is a better use of the follow-up than another activation,
        // and among deploys a baton is the guides' preferred upgrade.
        followups.sort_by_key(|a| {
            u8::from(a.action_type == ActionType::PlayMemberToStage) * 2
                + u8::from(is_baton(sim, self.me, a))
        });
        followups.truncate(followup_width());

        let mut best = base;
        for action in followups {
            if self.nodes >= self.budget {
                break;
            }
            let _guard = RngGuard::new();
            let mut next = sim.clone();
            if game_setup::execute_action(&mut next, &action).is_err() {
                continue;
            }
            game_setup::settle_single_player_state(&mut next);
            let value = self.leaf(&next);
            if value > best {
                best = value;
            }
        }
        best
    }
}

pub fn score_actions_v8(gs: &GameState, actions: &[Action], me: u8) -> Vec<(f64, String)> {
    let db = &gs.card_database;
    let pending = gs.has_pending_choice();
    let opp = OppModel::build(gs, me, db);
    let mut search = Search {
        me,
        db,
        opp,
        nodes: 0,
        budget: node_budget(),
    };

    let mut scores = Vec::with_capacity(actions.len());
    for action in actions {
        if action.parameters.as_ref().and_then(|p| p.disabled) == Some(true) {
            scores.push((f64::NEG_INFINITY, "disabled".into()));
            continue;
        }
        // `Pass` is scored by the SAME leaf on the SAME state, so it can only
        // win when ending the phase really is the best play. v7 needed two
        // special cases here (a −0.05 Pass tax that exploded draws, and a
        // baton tie-break that was a bug fix); neither exists in v8.
        if action.action_type == ActionType::Pass && !pending {
            let value = search.leaf(gs);
            scores.push((value, "end-main".into()));
            continue;
        }
        let _guard = RngGuard::new();
        let mut sim = gs.clone();
        if game_setup::execute_action(&mut sim, action).is_err() {
            scores.push((f64::NEG_INFINITY, "execution-error".into()));
            continue;
        }
        game_setup::settle_single_player_state(&mut sim);
        crate::turn::TurnEngine::check_victory_condition(&mut sim);

        let value = if sim.game_result != crate::game_state::GameResult::Ongoing {
            match sim.game_result {
                crate::game_state::GameResult::Draw => 0.0,
                crate::game_state::GameResult::FirstAttackerWins
                    if gs.seat_player(me).is_first_attacker =>
                {
                    SCALE * 3.0
                }
                crate::game_state::GameResult::SecondAttackerWins
                    if !gs.seat_player(me).is_first_attacker =>
                {
                    SCALE * 3.0
                }
                _ => -SCALE * 3.0,
            }
        } else {
            search.with_followup(&sim)
        };
        let baton = is_baton(gs, me, action);
        scores.push((
            value,
            format!("leaf={value:.1} nodes={} baton={baton}", search.nodes),
        ));
    }

    if std::env::var_os("V8_DEBUG").is_some() {
        for (i, (value, note)) in scores.iter().enumerate() {
            eprintln!(
                "V8M t{} me{} idx{} act={:?} value={:.2} note={}",
                gs.turn_number,
                me,
                i,
                actions[i].action_type,
                value,
                note
            );
        }
    }
    scores
}

/// Index selection: strict improvement, then the development tiebreak. An
/// action that ties the leaf exactly is taken only when it actually develops
/// the board, so a zero-contribution member still loses to `Pass`.
fn pick_best(gs: &GameState, me: u8, actions: &[Action], scores: &[(f64, String)]) -> usize {
    let db = &gs.card_database;
    let mut best = 0usize;
    for i in 0..scores.len() {
        let (value, _) = &scores[i];
        if !value.is_finite() {
            continue;
        }
        let (current, _) = &scores[best];
        if *value > *current + 1e-9 {
            best = i;
        } else if (*value - *current).abs() <= 1e-9
            && actions[i].action_type != ActionType::Pass
            && actions[best].action_type == ActionType::Pass
        {
            // Only a Pass can be displaced on a tie, and only by development.
            if development_key(gs, me, db, &actions[i])
                > development_key(gs, me, db, &actions[best])
            {
                best = i;
            }
        }
    }
    best
}

pub fn choose_action_v8(gs: &GameState, actions: &[Action], me: u8) -> Action {
    if actions.len() == 1 {
        return actions[0].clone();
    }
    let scores = score_actions_v8(gs, actions, me);
    let best = pick_best(gs, me, actions, &scores);
    let chosen = actions.get(best).cloned().unwrap_or(Action {
        action_type: ActionType::Pass,
        description: "pass".into(),
        description_ja: None,
        parameters: None,
        selected: None,
    });
    log::debug!(
        "v8 main t{} me{} act={:?} params={:?} score={:.2} note={}",
        gs.turn_number,
        me,
        chosen.action_type,
        chosen.parameters,
        scores.get(best).map(|(s, _)| *s).unwrap_or(f64::NEG_INFINITY),
        scores.get(best).map(|(_, n)| n.as_str()).unwrap_or(""),
    );
    chosen
}
