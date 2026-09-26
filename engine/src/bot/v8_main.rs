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
    let (we_c, they_c, _) = opp.contested_masses(our_score, 1, my_success);
    let contested = p_pass * opp.pass_prob;
    let opp_place = opp.pass_prob * (1.0 - p_pass) + contested * they_c;
    (our_place.min(1.0), opp_place.min(1.0))
}

/// Expected placement advantage of a position, in placement units.
///
/// The leaf is deliberately the NEXT CHECK and nothing further:
///
/// ```text
/// leaf = w * (P(we place) - P(they place)) + w * PLACEMENT_CREDIT * (lives in hand + initiative)
/// ```
///
/// The first draft also carried a one-turn-forward development term. It is
/// gone, and the reason matters. A forward projection adds "our best
/// affordable member" to the projected board, and it does that whether or not
/// we already played that member this phase - so deploying now and passing
/// both project onto the same ceiling, the leaf comes out EXACTLY equal, and
/// the tie falls to list order, where `Pass` is index 0. Measured, that was a
/// 60% pass rate, a stage cost of 0.8 at T1 against the guide's 4, and a
/// 1.3-heart average board. The whole game was played a turn behind.
///
/// The two regimes are what the guide actually describes, so v8 now uses two
/// mechanisms instead of one blurred one:
///
/// - where a check is live, this leaf decides, in placement units;
/// - where no check is live yet, [`TieKey`] decides, and `TieKey`'s first
///   component is the guides' own development metric (section 1: T1=4, T2=9,
///   T3=13). No conversion constant between "stage cost" and "hearts" is
///   needed, because none is invented.
fn leaf_value(gs: &GameState, me: u8, db: &CardDatabase, opp: &OppModel) -> f64 {
    let supply = v8_model::board_supply(gs, me, db);
    let (blades, density) = super::strategy_v4::flip_stats(gs, me, db);
    let (our_place, their_place) = placement_pair(gs, me, db, opp, &supply, blades, density);
    let ammo = v8_model::lives_in_hand(gs.seat_player(me), db) as f64;
    let initiative = if gs.seat_player(me).is_first_attacker {
        initiative_weight()
    } else {
        0.0
    };
    SCALE * ((our_place - their_place) + PLACEMENT_CREDIT * (ammo + initiative))
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

/// Tie-break key, in the guides' own units.
///
/// 1. `cost_now` - the current board's total printed stage cost. This is the
///    guide's development metric (section 1: T1 about 4, T2 about 9, T3 about
///    13) and it is what decides the game while no check is live. Because it
///    is measured on the state the action PRODUCES, deploying a member beats
///    passing, and a member that contributes nothing - a clog - does not, which
///    is the anti-clog rule with no separate case for it.
/// 2. `ceiling` - the forward reachable cost after two more of our Main
///    phases, so a step that unlocks the 4 -> 9 baton ladder is preferred over
///    an equivalent one that does not.
/// 3. `baton` - a swap is the guides' preferred upgrade (9.6.2.3.2) at equal
///    cost and ceiling.
///
/// The arena's `ScoreFn` type only carries `(f64, String)`, so the key is kept
/// here and flattened into the note for debug output.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct TieKey {
    cost_now: i32,
    ceiling: i32,
    baton: u8,
}

fn tie_key(sim: &GameState, me: u8, db: &CardDatabase, action: &Action) -> TieKey {
    TieKey {
        cost_now: v8_model::stage_cost(sim, me, db),
        ceiling: v8_model::reachable_ceiling(sim, me, 2, db),
        baton: u8::from(is_baton(sim, me, action)),
    }
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

/// One action's evaluation.
#[derive(Clone, Debug)]
pub(crate) struct ActionScore {
    pub value: f64,
    pub tie: TieKey,
    pub note: String,
}

impl ActionScore {
    fn rejected(note: &str) -> Self {
        Self {
            value: f64::NEG_INFINITY,
            tie: TieKey {
                cost_now: i32::MIN,
                ceiling: i32::MIN,
                baton: 0,
            },
            note: note.into(),
        }
    }
}

/// Evaluate every offered action. Used by the entry point and by the arena's
/// decision trace; the `ScoreFn` wrapper below flattens it to the tuple form
/// the harness expects.
pub(crate) fn evaluate_actions(gs: &GameState, actions: &[Action], me: u8) -> Vec<ActionScore> {
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
            scores.push(ActionScore::rejected("disabled"));
            continue;
        }
        // `Pass` is scored by the SAME leaf on the SAME state, so it can only
        // win on the tiebreak, which means it wins when ending the phase
        // really is the best play. v7 needed two special cases here (a -0.05
        // Pass tax that exploded draws, and a baton tie-break that was a bug
        // fix); neither exists in v8.
        if action.action_type == ActionType::Pass && !pending {
            let value = search.leaf(gs);
            scores.push(ActionScore {
                value,
                tie: tie_key(gs, me, db, action),
                note: "end-main".into(),
            });
            continue;
        }
        let _guard = RngGuard::new();
        let mut sim = gs.clone();
        if game_setup::execute_action(&mut sim, action).is_err() {
            scores.push(ActionScore::rejected("execution-error"));
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
        let nodes = search.nodes;
        let tie = tie_key(&sim, me, db, action);
        scores.push(ActionScore {
            value,
            tie,
            note: format!(
                "leaf={value:.1} nodes={nodes} baton={} cost={} ceil={}",
                tie.baton == 1,
                tie.cost_now,
                tie.ceiling
            ),
        });
    }

    if std::env::var_os("V8_DEBUG").is_some() {
        for (i, score) in scores.iter().enumerate() {
            eprintln!(
                "V8M t{} me{} idx{} act={:?} value={:.2} note={}",
                gs.turn_number,
                me,
                i,
                actions[i].action_type,
                score.value,
                score.note
            );
        }
    }
    scores
}

pub fn score_actions_v8(gs: &GameState, actions: &[Action], me: u8) -> Vec<(f64, String)> {
    evaluate_actions(gs, actions, me)
        .into_iter()
        .map(|score| (score.value, score.note))
        .collect()
}

/// Index selection, as a total order so ties are never resolved by list
/// position:
///
/// 1. strictly higher leaf value wins;
/// 2. on an exact tie, the higher development key wins (a free ladder step is
///    never thrown away on a rounding tie);
/// 3. on an exact tie in both, the earlier action wins, which keeps `Pass`
///    over a zero-contribution member - the anti-clog rule, with no special
///    case in the value function.
fn pick_best(scores: &[ActionScore]) -> usize {
    let mut best = 0usize;
    for i in 1..scores.len() {
        if !scores[i].value.is_finite() {
            continue;
        }
        if !scores[best].value.is_finite() {
            best = i;
            continue;
        }
        let (value, incumbent) = (scores[i].value, scores[best].value);
        if value > incumbent + 1e-9 {
            best = i;
        } else if (value - incumbent).abs() <= 1e-9 && scores[i].tie > scores[best].tie {
            best = i;
        }
    }
    best
}

pub fn choose_action_v8(gs: &GameState, actions: &[Action], me: u8) -> Action {
    if actions.len() == 1 {
        return actions[0].clone();
    }
    let scores = evaluate_actions(gs, actions, me);
    let best = pick_best(&scores);
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
        scores.get(best).map(|s| s.value).unwrap_or(f64::NEG_INFINITY),
        scores.get(best).map(|s| s.note.as_str()).unwrap_or(""),
    );
    chosen
}
