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

use crate::bot::strategy_common::Acc;
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

/// Weight on the guides' continuous score-ceiling term in the Main leaf.
///
/// This is a MEASURED weight, not an asserted one. It exists because the
/// placement term alone is a threshold test: while the board is short of every
/// life in hand it is identically zero and no amount of development moves it,
/// so 81.8% of Main decisions tied and fell through to the tiebreak. The
/// ceiling term is monotone in hearts and blades, so it restores a gradient
/// exactly where the placement term has none.
///
/// The default is the largest value that still leaves the placement term
/// dominant, because the placement term is the one that is rule-exact: a
/// placement is 1/3 of a game (1.2.1.1 / 8.4.7) and a score band is worth
/// nothing on its own. `V8_DEV_CEILING` re-weights it; `V8_NO_DEV_CEILING`
/// restores the pre-fix leaf so the fix can be re-measured.
/// Default 0, measured OFF.
///
/// This was the first fix attempted, on the theory that v8's leaf was a
/// cost-curve proxy. It is the wrong proxy - cost only stands in for the
/// hearts and blades a check actually reads - and it was superseded by
/// `band_progress`. It is retained only as an ablation handle because
/// `score_ceiling` is a real, separately useful quantity, and removing the
/// function would remove the ability to test that claim again.
fn dev_ceiling_weight() -> f64 {
    if std::env::var_os("V8_NO_DEV_CEILING").is_some() {
        return 0.0;
    }
    std::env::var("V8_DEV_CEILING")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|w: &f64| w.is_finite() && *w >= 0.0)
        .unwrap_or(0.0)
}

/// Weight on the continuous band-progress term, the Main phase's development
/// signal.
///
/// The first attempt used the guides' stage-COST ladder, on the theory that a
/// one-check leaf was too myopic to see development. That theory was wrong and
/// the measurement says so plainly: with 700 games, v8's Main and v7's Main
/// reach an IDENTICAL stage-cost curve (entering T4 11.7 vs 12.2, T7 24.5 vs
/// 25.9, final 20.1 vs 21.4), both on the guide, and v8 still loses 8.7 points
/// more. Cost is a proxy for the quantity the check actually reads, which is
/// hearts and blades (3.2), and on a real decklist the proxy breaks: a
/// high-cost low-heart member scores the same as a high-cost high-heart one.
///
/// So the term is built on `hearts + Binomial(active blades, own density)` —
/// section 4's "the real scoreboard" — and expressed in score-band units by
/// interpolating the guide's own band medians, so no conversion constant
/// between hearts and probability is invented.
///
/// `V8_DEV_BAND` re-weights it; `V8_NO_DEV_CEILING` removes it.
///
/// Default 4.0, measured: removing the term costs 27 wins over 700 games and
/// a significant -0.018 placements-per-live-phase, and the curve is flat from
/// about 2 upward (2.0/4.0/8.0/16.0 all within noise of each other), so the
/// exact value is not load-bearing - the term being present at all is.
fn dev_band_weight() -> f64 {
    std::env::var("V8_DEV_BAND")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|w: &f64| w.is_finite() && *w >= 0.0)
        .unwrap_or(4.0)
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
    let (_, they_c, _) = opp.contested_masses(our_score, 1, my_success);
    let contested = p_pass * opp.pass_prob;
    let opp_place = opp.pass_prob * (1.0 - p_pass) + contested * they_c;
    (our_place.min(1.0), opp_place.min(1.0))
}

/// Weight on the forward (T+3) placement term relative to the next-check one.
///
/// The single-check leaf was the root defect. Measured over 725 decision
/// points it produced 1.74 distinct values across 5.4 offered actions and tied
/// at the top in 81.8% of decisions, because `P(place)` is a threshold test
/// against whichever lives happen to be in hand. Worse, its horizon inverted
/// the guide: it valued a certain 1-point placement NOW at `PLACEMENT_CREDIT`
/// and valued the 5-6 point board that actually wins a T4-T5 check at exactly
/// zero, because that board is not checkable yet. Section 1 is explicit that
/// most games end on a 6-8 point closeout at T5, so the leaf was pricing the
/// wrong game.
///
/// Two horizons in the same rule-exact currency fixes it without inventing a
/// weight out of nothing: a tempo grab scores on T+1, a ladder build scores on
/// T+3, and the bot has to actually choose between them. The default keeps the
/// next check dominant, because a placement is the only thing that is
/// unconditionally worth 1/3 of the game (1.2.1.1 / 8.4.7).
///
/// `V8_HORIZON` re-weights it; `V8_NO_HORIZON` restores the one-check leaf so
/// the fix can be re-measured rather than taken on trust.
///
/// Default 0, measured OFF. The two-horizon leaf was the right hypothesis and
/// it did not pay: at 0.5 and 1.0 it moved win rate by less than the noise
/// floor and never reached significance. The measurement that settled the
/// question was the development curve, which showed v8's Main and v7's reach
/// an identical stage-cost profile - so the leaf was never short of horizon,
/// it was short of hearts and blades. Kept, ablatable, and honest about having
/// been measured rather than shipped on the argument that it must help.
fn horizon_weight() -> f64 {
    if std::env::var_os("V8_NO_HORIZON").is_some() {
        return 0.0;
    }
    std::env::var("V8_HORIZON")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|w: &f64| w.is_finite() && *w >= 0.0)
        .unwrap_or(0.0)
}

/// Expected placement advantage of a position, in placement units.
///
/// ```text
/// leaf = (P(we place) - P(they place))            at the NEXT check
///      + h * (P(we place) - P(they place))        at the check after two
///                                                    more of our Main phases
///      + PLACEMENT_CREDIT * (lives in hand + initiative)
///      + w * score_ceiling                         (section 4, continuous)
/// ```
///
/// The next-check term alone was measured to be inert for 82% of decisions
/// (above). The forward term is what gives the search something to see: it
/// runs the guides' 4 -> 9 -> 13 ladder forward through the engine's own turn
/// structure, so a sideways baton that improves the board in two turns scores
/// here even though it is worth nothing right now. That is the payoff the
/// project has been trying to buy with a flat baton bonus since v7 - and a
/// flat bonus is precisely what double-counted development and measured -18pp
/// (BOT_STRATEGY.md 9.2, D1). A trajectory term priced by a simulation cannot
/// double-count, because the member is on stage in the simulated state and is
/// therefore not deployed twice.
fn leaf_value(gs: &GameState, me: u8, db: &CardDatabase, opp: &OppModel) -> f64 {
    let (_blades, density) = super::strategy_v4::flip_stats(gs, me, db);

    let supply = v8_model::board_supply(gs, me, db);
    let (our_now, their_now) =
        placement_pair(gs, me, db, opp, &supply, v8_model::active_blades(gs, me, db), density);

    // The check the guides' curve is actually aimed at: our board after two
    // more Main phases, developed by the same forward walk the doctrine
    // describes rather than by an assumed best case.
    let (future_hearts, future_blades) = v8_model::forward_supply(gs, me, 2, db);
    // ...priced against the opponent advancing by the same two turns. Both
    // players draw the same cards and gain the same energy each turn (3.2), so
    // holding them at their current board inside a term whose job is to see
    // the race in two turns systematically flatters us, and flatters us most
    // on the actions that develop hardest. Their hand and deck are hidden and
    // are not read; the growth is ours, applied to them, and that symmetry is
    // an assumption stated rather than a forecast.
    let opp_later = if opp_horizon_enabled() {
        let opp_seat = 1 - me;
        let their_now_total: i32 = v8_model::board_pool(gs, opp_seat, db)
            .iter()
            .map(|&v| i32::from(v))
            .sum();
        let their_now_blades = v8_model::active_blades(gs, opp_seat, db);
        let our_now_total: i32 = supply.iter().map(|&v| i32::from(v)).sum();
        let our_now_blades = v8_model::active_blades(gs, me, db);
        opp.with_board_sampled(
            gs,
            me,
            db,
            their_now_total
                + (future_hearts.iter().map(|&v| i32::from(v)).sum::<i32>() - our_now_total),
            their_now_blades + (future_blades - our_now_blades),
        )
    } else {
        opp.clone()
    };
    let (our_later, their_later) =
        placement_pair(gs, me, db, &opp_later, &future_hearts, future_blades, density);

    let ammo = v8_model::lives_in_hand(gs.seat_player(me), db) as f64;
    let initiative = if gs.seat_player(me).is_first_attacker {
        initiative_weight()
    } else {
        0.0
    };
    let ceiling = v8_model::score_ceiling(gs, me, db) as f64;
    // Guarded, not multiplied: `colour_shortfall` walks the hand and calls
    // `life_need` per card, and `leaf_value` runs once per search node (up to
    // 96 per decision), so evaluating it to multiply by 0.0 would tax the
    // default path to support a switch nobody is holding.
    let deficit = match deficit_weight() {
        0.0 => 0.0,
        w => w * -(v8_model::colour_shortfall(gs, me, db) as f64),
    };
    SCALE
        * ((our_now - their_now)
            + horizon_weight() * (our_later - their_later)
            + PLACEMENT_CREDIT * (ammo + initiative)
            + dev_ceiling_weight() * ceiling
            + dev_band_weight() * v8_model::band_progress(gs, me, db)
            + deficit)
}

/// `V8_DEFICIT` is OFF by default, and it is staying off.
///
/// The other terms in `leaf_value` are colour-blind: they read total supply,
/// total blades, and an interpolation of the same. Two actions that put three
/// hearts on stage score identically whether those hearts are the colours the
/// life in hand needs or the colours it does not — and the log's nine failed
/// leaf-term experiments are all that: each was a monotone function of
/// quantities `band_progress` already reads, so none could reorder the argmax
/// no matter how it was weighted.
///
/// This term is NOT one of those. `colour_shortfall` is a function of the
/// board's colour VECTOR against one card's requirement, and
/// `colour_shortfall_sees_the_vector_not_the_total` pins that as an arithmetic
/// property: two equal-total boards in different colours get different values.
/// So the usual defence of the dead ends — "it was colour-blind too" — does not
/// apply, and the term was measured on its own.
///
/// It is a regression, and worse at a larger weight:
///
///     w=0.35  148-252 vs 179-221  McNemar p=0.00015  pace 0.403 -> 0.366
///     w=0.05  165-235 vs 179-221  McNemar p=0.04356  pace 0.403 -> 0.384
///
/// Monotone in the harmful direction across a 7x range, so this is a gradient
/// pointing at zero rather than a surface with a peak the sweep missed. See
/// V8_IMPROVEMENT_LOG.md §0.7 for why the target life is the likely culprit.
/// `V8_OPP_HORIZON` is off by default.
///
/// The horizon term runs our forward walk two turns ahead and then prices the
/// opponent at the board they have *now*. Both players draw the same cards and
/// gain the same energy each turn, so inside a term whose entire purpose is to
/// see the race in two turns, the opponent never grows. That does not bias
/// every action equally: it flatters the actions that develop hardest, because
/// their measured gain is the only side of the race that moves.
///
/// `V8_NEXT_METHODS.md` §6.5 asks for the opponent model to be "refreshed
/// through the search". As written that is a no-op and worth saying why: during
/// our own Main phase the opponent's board cannot change, so re-reading it at
/// each node returns an identical model. The staleness is in the horizon, not
/// the depth.
///
/// The correction advances their public board by the same amount ours advanced.
/// Their hand and deck are hidden and are never read; this is our growth
/// applied to them, and the symmetry is an assumption being made explicit
/// rather than one being made silently.
fn opp_horizon_enabled() -> bool {
    std::env::var_os("V8_OPP_HORIZON").is_some()
}

fn deficit_weight() -> f64 {
    if std::env::var_os("V8_DEFICIT").is_some() {
        std::env::var("V8_DEFICIT_W")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.35)
    } else {
        0.0
    }
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
pub(crate) struct TieKey {
    /// `V8_TIE_BATON`: prefer a baton over ending the phase at equal value.
    ///
    /// First in the key, and zero for every action unless the switch is on, so
    /// the default ordering is bit-for-bit what it was. When enabled it is 1
    /// for a baton and 0 for everything else INCLUDING Pass, which is the whole
    /// mechanism.
    ///
    /// `Pass` is scored by the same leaf on the same state, so it wins on the
    /// tiebreak alone — and it wins every one, because ending the phase leaves
    /// `cost_now` at the current stage cost and every deploy raises it. So the
    /// leaf is free to believe ending the phase is as good as playing, and it
    /// is, and the phase ends. In v7 that single tie-break is worth **+16.8
    /// points**, the largest entry in the ablation table, and the comment on
    /// this type records that v8 has neither it nor the Pass tax. This is that
    /// fix, on a switch, to be measured rather than assumed.
    baton_over_pass: u8,
    cost_now: i32,
    ceiling: i32,
    baton: u8,
}

/// `V8_PASS_TAX`: charge for ending the Main phase. In `SCALE` units, and
/// applied to Pass's value only.
///
/// v7 has one of these. The comment above `TieKey` says it "exploded draws"
/// there, which is a warning about the *sign* of the earlier fix rather than
/// about the idea, and §0.3 of the improvement log is the reason to retry it:
/// v7 and v8 were scoring a junk-only zone as 1.000 and 0.000 respectively,
/// which means the two "committed p" figures that the fold policy was tuned
/// against were not measuring the same thing. Now that both report 0.0 for a
/// zone with no life in it, the fold decision is being made on a consistent
/// number and a tax on it can be re-measured on its merits.
///
/// Default 0 (off). The value is in `SCALE` units, so 0.05 is half a point of
/// a three-point game.
fn pass_tax() -> f64 {
    std::env::var("V8_PASS_TAX")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|w: &f64| w.is_finite() && *w >= 0.0)
        .unwrap_or(0.0)
}

fn tie_baton_over_pass_enabled() -> bool {
    std::env::var_os("V8_TIE_BATON").is_some()
}

fn tie_key(sim: &GameState, me: u8, db: &CardDatabase, action: &Action) -> TieKey {
    TieKey {
        baton_over_pass: u8::from(tie_baton_over_pass_enabled() && is_baton(sim, me, action)),
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
                baton_over_pass: 0,
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
            let value = search.leaf(gs) - pass_tax();
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
        if value > incumbent + 1e-9
            || ((value - incumbent).abs() <= 1e-9 && scores[i].tie > scores[best].tie)
        {
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
