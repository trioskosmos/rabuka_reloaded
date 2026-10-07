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
    quantise(
        SCALE
            * ((our_now - their_now)
                + horizon_weight() * (our_later - their_later)
                + PLACEMENT_CREDIT * (ammo + initiative)
                + dev_ceiling_weight() * ceiling
                + dev_band_weight() * v8_model::band_progress(gs, me, db)
                + deficit)
            + blade_delta_weight() * v8_model::active_blades(gs, me, db) as f64
            + ammo_delta_weight() * ammo
            + opp_blade_delta_weight() * v8_model::active_blades(gs, 1 - me, db) as f64,
    )
}

/// `V8_OPP_BLADES=<w>`: the other half of the blade term, and the half v8 has
/// no way to express at all.
///
/// `v7_main::Search::evaluate` prices **both** sides with linear deltas:
///
/// ```text
/// - 8.0 * (opp_cost   - base.opp_cost)
/// - 6.0 * (opp_blades - base.opp_blades)
/// - 10.0 * (opp_hand   - base.opp_hand)
/// - 1000.0 * (opp_success - base.opp_success)
/// ```
///
/// So v7 is paid for a blade **differential**, not just for our own blades. That
/// is the mechanism by which developing a blade is worth more when they are
/// behind and worth less when they are ahead — the whole point of a race.
///
/// v8 cannot represent that. The opponent enters its leaf only through
/// `OppModel::pass_prob`, which is a hard threshold and takes **6 distinct
/// values** across 369 board/blade combinations
/// (`the_opponent_pass_model_has_only_a_handful_of_states`). A function that
/// coarse cannot express a marginal rate of change, so no amount of work on our
/// own side substitutes for it. This term is the missing opponent-side
/// differential, transcribed from v7's source rather than guessed at.
/// `V8_COVERAGE=<w>`: v7's second-largest term (60.0), transcribed properly.
///
/// ## This is not the failed `V8_DEFICIT` experiment
///
/// `colour_shortfall` measured as a **regression** (p=0.044 at w=0.05,
/// p=0.00015 at w=0.35). Two things were different, and the difference is the
/// point:
///
/// 1. **Delta, not level.** `v7_main::value` scores `60.0 * (now.coverage -
///    base.coverage)` — the *change* in coverage this action causes. The
///    shortfall term subtracted an absolute shortfall, which is a level, and a
///    level in a saturated leaf cannot reorder anything. A delta can, because
///    it is measured on the same axis for every candidate action.
/// 2. **v7's target, not mine.** v7's coverage is "how much of the life in
///    hand's requirement does the board already cover", evaluated per action.
///    The shortfall term aimed at the *biggest* life in hand, which early in a
///    game is usually one that cannot be set for turns — so it bought colours
///    for an unaffordable card and pace fell. That is a target-selection error
///    and it was mine, not the maths'.
///
/// ## Why it is worth another cell
///
/// v7's ablation gives coverage -0.5 on its own, which is why it was in the
/// "seven minor terms" bucket that the log was careful not to call decorative.
/// But the blade result is the reason to retest it: `6.0 * dBlades` was worth
/// **-5.1** in v7 and moving it into v8 produced the only significant win on
/// this phase in the whole file. The reading of v7's source is producing
/// predictions that hold, and coverage is the largest term still missing.
///
/// Off by default, measured on its own.
fn coverage_delta(gs: &GameState, me: u8, db: &CardDatabase) -> i32 {
    // How much of the best life in hand's coloured requirement the board
    // already satisfies. Mirrors v7's `coverage` feature rather than inventing
    // a second definition of it.
    let board = v8_model::board_pool(gs, me, db);
    let mut best: i32 = 0;
    let mut found = false;
    for &cid in &gs.seat_player(me).hand.cards {
        if !db
            .get_card(cid)
            .is_some_and(|c| c.card_type == crate::card::CardType::Live)
        {
            continue;
        }
        let need = v8_model::life_need(gs, cid);
        if v8_model::has_unpassable_icon(&need) {
            continue;
        }
        let mut covered = 0i32;
        for c in 0..=6usize {
            covered += i32::from(need[c]).min(i32::from(board[c]));
        }
        covered += i32::from(need[7]);
        if !found || covered > best {
            best = covered;
            found = true;
        }
    }
    if !found {
        0
    } else {
        best
    }
}

fn coverage_weight() -> f64 {
    std::env::var("V8_COVERAGE")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|w: &f64| w.is_finite() && *w >= 0.0)
        .unwrap_or(0.0)
}

fn opp_blade_delta_weight() -> f64 {
    std::env::var("V8_OPP_BLADES")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|w: &f64| w.is_finite() && *w >= 0.0)
        .unwrap_or(0.0)
}

/// `V8_BLADES=<w>`: a linear blade term, in v7's own units.
///
/// ## Why v8 structurally cannot express this today
///
/// `v7_main::value` scores an action as a linear function of coarse feature
/// **deltas**: `6.0 * dBlades`, `25.0 * dAmmo`, `60.0 * dCoverage`. The
/// ablation map records blades at **-5.1** and ammo at **-2.9** — together the
/// two largest leaf terms v7 has, and the only two that survive being zeroed
/// individually.
///
/// v8's leaf is a different kind of object: a *level* in placement-probability
/// units. Blades reach it only through `placement_pair`, which routes them into
/// a binomial that **saturates** — once the check is comfortably passable, more
/// blades move the probability by nothing, so v8 is paid nothing for them.
/// v7 is paid 6 per blade whether or not the check is already safe.
///
/// That is not a weight difference, it is a difference in what the two functions
/// are able to say. No reweighting of v8's existing terms can recover a term
/// whose information the function has already thrown away, which is the same
/// reason the ten earlier term additions were inert. This one is not an
/// addition of *information*; it is an addition of a *term that was always
/// missing*, read out of v7's source rather than guessed at.
///
/// ## It is a level here on purpose
///
/// v7 uses a delta because it is scoring one action against one base inside a
/// chain. v8 evaluates each action independently and argmaxes, so a delta
/// against the pre-action board is available at the same place and is the
/// quantity v7's weight was fitted against. Using the post-action blade count as
/// a level would double-count against `band_progress`, which already reads
/// blades. Kept behind a switch, off by default, and measured on its own.
/// `V8_CONVERT_GATE=1`: is this deploy still buying anything the check can use?
///
/// False once the board has grown past what the best life in hand needs, which
/// is the point at which a player stops developing and banks the board. The
/// guide's own bands are the threshold: `v8_model::band_progress` already
/// interpolates against the per-turn guide table, so "past the band" is a
/// quantity the model has, and the question is only whether v8 consults it when
/// ranking a deploy rather than only when scoring the leaf.
///
/// Off by default. The probe evidence for this is 4 games and 3 disagreements,
/// which is a real signal and a small sample, and the whole v8 Main phase is
/// the component that eleven earlier attempts could not move. It is measured on
/// its own before it is believed.
fn apply_convert_gate(gs: &GameState, after: &GameState, me: u8, db: &CardDatabase) -> bool {
    // Nothing in hand to convert into: growing is still the only way to get
    // there, so the gate must not fire and freeze a bot that has no life yet.
    if v8_model::lives_in_hand(gs.seat_player(me), db) == 0 {
        return true;
    }
    // The board the deploy produces, against the requirement of the best life we
    // hold. This is `colour_shortfall` again but as a BOOLEAN question — "is
    // any of this usable" — where the failed experiment asked "which card should
    // I optimise for" and got the target wrong.
    let board = v8_model::board_pool(after, me, db);
    let mut best: i32 = 0;
    let mut found = false;
    for &cid in &after.seat_player(me).hand.cards {
        if !db
            .get_card(cid)
            .is_some_and(|c| c.card_type == crate::card::CardType::Live)
        {
            continue;
        }
        let need = v8_model::life_need(after, cid);
        if v8_model::has_unpassable_icon(&need) {
            continue;
        }
        let mut covered = 0i32;
        for c in 0..=6usize {
            covered += i32::from(need[c]).min(i32::from(board[c]));
        }
        covered += i32::from(need[7]);
        if !found || covered > best {
            best = covered;
            found = true;
        }
    }
    if !found {
        return true;
    }
    // Shortfall before and after. If the deploy closes none of it, the board has
    // grown past the check and the growth is not convertible.
    let before = v8_model::colour_shortfall(gs, me, db);
    let after_gap = v8_model::colour_shortfall(after, me, db);
    after_gap < before
}

/// `V8_HAND_QUALITY=<w>`: value the hand's *contents*, not its size.
///
/// `v7_term_score` already pays `3.0 * dHand`, so every option of a tutor choice
/// scores +3.0 and the pick is arbitrary. The deck the bots play has two tutor
/// bodies — `葉月 恋` `PL!SP-BP1-005-R` (cost 2, 登場: look at the top 5 and
/// reveal one Liella card to hand) and `米女メイ` `PL!SP-BP2-007-R` (cost 5, the
/// same for members) — so the bot is choosing a cost-11 body over a cost-2
/// vanilla on a coin flip, at the exact moment it is setting up the turn.
///
/// This is the only term in the leaf that reads the HAND's contents rather than
/// the board, which is why it can separate candidates that every board term
/// proves equal. Gated, because eleven prior additive terms to this leaf were
/// inert and the prior is deserved — the difference is that this one is not a
/// function of the same quantities.
fn hand_quality_weight() -> f64 {
    std::env::var("V8_HAND_QUALITY")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|w: &f64| w.is_finite())
        .unwrap_or(0.0)
}

fn blade_delta_weight() -> f64 {
    std::env::var("V8_BLADES")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|w: &f64| w.is_finite() && *w >= 0.0)
        .unwrap_or(0.0)
}

/// `V8_AMMO=<w>`: the same idea for the second-largest v7 term (25.0, -2.9).
///
/// Ammo already appears in v8's leaf through `PLACEMENT_CREDIT`, but only as
/// part of a saturated `p_place`, so a deployment that adds ammo without
/// crossing a probability threshold is invisible to it. v7 pays 25 per ammo
/// unconditionally.
fn ammo_delta_weight() -> f64 {
    std::env::var("V8_AMMO")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|w: &f64| w.is_finite() && *w >= 0.0)
        .unwrap_or(0.0)
}

/// `V8_QUANT=<step>`: round the leaf onto a coarse grid, in `SCALE` units.
///
/// ## Why coarsen a value function that is already right
///
/// v7's leaf is coarse and ties constantly; the baton-over-Pass tie-break is
/// worth **+16.8 points** to it, the largest entry in the ablation table. v8's
/// leaf is a smooth sum of many continuous terms and essentially never ties —
/// `V8_TIE_BATON` changed 23 games in 1200 — so v8 does not merely lack the
/// tie-break's value, it lacks the mechanism the tie-break works through.
///
/// That reframes the whole Main-phase gap. It may not be that v7 scores actions
/// better; it may be that v7's value function is **granular enough to abstain**,
/// and its tie-break then supplies the decision. Between two near-equal actions a
/// continuous leaf emits a small real-number difference that is mostly noise, and
/// a 0.3-point perturbation wins or loses on nothing. A quantised leaf refuses
/// to distinguish what it cannot actually know, and hands those cases to a key
/// that encodes guide knowledge (cost, ceiling, baton) instead of to float noise.
///
/// This is the inverse of every other attempt in the file, all of which added
/// precision. It is also falsifiable in one run: if the gap is really about
/// *which* actions are preferred, quantising cannot recover it and this goes
/// null like the chain did.
fn quantise(value: f64) -> f64 {
    let step = std::env::var("V8_QUANT")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|s| s.is_finite() && *s > 0.0)
        .unwrap_or(0.0);
    if step <= 0.0 {
        return value;
    }
    (value / step).round() * step
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
    /// The state the decision started from, for `V8_CHAIN_ROOT`.
    root: GameState,
    /// The root's own leaf value, so `scored_leaf` does not recompute it.
    root_value: f64,
}

impl Search<'_> {
    /// `V8_DELTA_LEAF=1`: score the leaf as a change from the root rather than
    /// as a level.
    ///
    /// ## Why this is coupled to the chain and not a separate idea
    ///
    /// `V8_CHAIN` measured null, and the chain is the reason. `with_followup`
    /// takes `max(base, followup)` and my `chain` committed on a *strict
    /// improvement*, so a depth-3 and a depth-6 chain returned bit-identical
    /// results — it stopped before extending.
    ///
    /// The cause is the shape of the leaf. `v7_main::value` is linear in feature
    /// **deltas**, so along a path the deltas compose additively and taking a max
    /// at each level is a real backup: a move that looks neutral on its own can
    /// be correct because of what it enables next. v8's leaf is a **level** in
    /// placement-probability units, so every node's value already contains
    /// everything before it, the max is nearly the level, and "strictly better
    /// than the state I came from" is almost never true after one step.
    ///
    /// So the chain and the leaf are one change, not two. Subtracting the root's
    /// value makes the leaf additive along the path, which is the property the
    /// search actually needs. The root's own placement level is a constant
    /// across all candidate actions, so it cannot change the argmax — this is
    /// the same argument as `V8_NEXT_METHODS` §6.4, and unlike the eleven
    /// earlier attempts this one is not adding information, it is putting
    /// existing information in the form the search can use.
    fn scored_leaf(&mut self, gs: &GameState) -> f64 {
        let value = self.leaf(gs);
        if !delta_leaf_enabled() {
            return value;
        }
        value - self.root_value
    }

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

    /// `V8_CHAIN=<depth>`: walk a greedy chain instead of one breadth step.
    ///
    /// ## What is different, structurally
    ///
    /// `with_followup` is a **breadth** step: from the state the root action
    /// produced, it scores every follow-up's leaf and takes the maximum. It
    /// never commits to one, so the line it reports is "the best single next
    /// move", and two moves that only pay off together are invisible to it.
    ///
    /// v7's Main phase wins by re-offering and re-evaluating after **every**
    /// action. That is a different object: at each step it picks one action,
    /// commits to the real resulting state, and re-offers *from there*. The
    /// point is not depth — §2.4 ruled that out, and `V8_NODES=192` measured
    /// 3 discordant games in 400. The point is that the choice at step 2 is made
    /// knowing what step 1 actually did, which a max-of-leaves cannot do
    /// because it throws the chosen line away.
    ///
    /// ## Why the value is the END state, not a sum
    ///
    /// Summing per-step leaves double-counts: step 2's leaf already contains
    /// step 1's development. The chain therefore reports the leaf of the final
    /// state it committed to, and only commits to a step that *strictly*
    /// improves on the state it came from, so a chain that has run out of good
    /// moves stops rather than wandering. That makes it a strictly better search
    /// than the breadth step at equal node cost, not a different weighting of
    /// the same one.
    ///
    /// ## The budget is shared
    ///
    /// `self.nodes` is shared with the root expansion, so a chain that asks for
    /// depth 4 simply runs out of budget and returns what it has. Depth is
    /// therefore a ceiling, not a request, which is the right failure mode.
    fn chain(&mut self, sim: &GameState) -> f64 {
        let depth = chain_depth();
        let mut value = self.scored_leaf(sim);
        let mut cur = sim.clone();
        for _ in 0..depth {
            if self.nodes >= self.budget {
                break;
            }
            if cur.current_phase != Phase::Main
                || cur.has_pending_choice()
                || !cur.can_player_act(self.me as i32)
            {
                break;
            }
            let mut followups: Vec<Action> = game_setup::generate_possible_actions(&cur)
                .into_iter()
                .filter(|a| a.parameters.as_ref().and_then(|p| p.disabled) != Some(true))
                .filter(|a| a.action_type != ActionType::Pass)
                .collect();
            followups.sort_by_key(|a| {
                u8::from(a.action_type == ActionType::PlayMemberToStage) * 2
                    + u8::from(is_baton(&cur, self.me, a))
            });
            followups.truncate(followup_width());

            // Commit to the single best next state, then re-offer from it.
            let mut chosen: Option<(GameState, f64)> = None;
            for action in followups {
                if self.nodes >= self.budget {
                    break;
                }
                let _guard = RngGuard::new();
                let mut next = cur.clone();
                if game_setup::execute_action(&mut next, &action).is_err() {
                    continue;
                }
                game_setup::settle_single_player_state(&mut next);
                // The node's leaf, PLUS the v7 feature-delta terms measured
                // against the DECISION ROOT rather than against the previous
                // step. This is the change that makes the chain worth running at
                // all.
                //
                // `V8_CHAIN` measured null (§2.7) and the diagnosis was the
                // leaf's shape: on a saturated LEVEL, every node already contains
                // everything before it, so the max is nearly the level and no
                // step ever "strictly improves", which is why depth 3 and depth 6
                // returned bit-identical results.
                //
                // `v7_term_score` is the opposite kind of quantity — a linear
                // function of coarse deltas, and deltas COMPOSE. Measured from
                // `root` instead of from the parent step, the score of a node is
                // the value of the whole line so far, so a move that pays nothing
                // by itself is still visible when it enables a later one. That is
                // the property the backup needs, and it is the one v7's
                // depth-8 chain has and v8's single breadth step does not.
                //
                // Root-relative is also the only correct choice for a term vector
                // whose weights are 8/3/6/60/25/1000: a step measured against its
                // parent reports only that step's contribution and the total
                // never grows, so the chain sees a flat landscape.
                // `V8_CHAIN_ROOT`: score this node against the DECISION ROOT,
                // not against its parent, so the chain becomes a real backup.
                //
                // §2.7 measured `V8_CHAIN` null and the diagnosis was the leaf's
                // SHAPE. On a saturated level every node already contains
                // everything that led there, so the max over children is nearly
                // the level, no step ever "strictly improves", and depth 3 and
                // depth 6 returned bit-identical results. A linear delta has the
                // opposite property and COMPOSES: a node scored from the root
                // carries the value of the whole line so far, so a move that
                // pays nothing by itself is still visible when it enables a later
                // one. That is the property a backup needs.
                //
                // The scorer is `v8_model::v7_delta_score` rather than the copy
                // in this file, because that copy is a nested item inside
                // `evaluate_actions` and cannot be reached from here. Coverage
                // is added here because `coverage_delta` is module-level in this
                // file (it sits above `Search`, not inside anything).
                let v = self.scored_leaf(&next)
                    + if std::env::var_os("V8_CHAIN_ROOT").is_some() {
                        super::v8_model::v7_delta_score(&self.root, &next, self.me, self.db)
                            + v8_model::v7_term_weight("COVERAGE", 60.0)
                                * (coverage_delta(&next, self.me, self.db)
                                    - coverage_delta(&self.root, self.me, self.db)) as f64
                    } else {
                        0.0
                    };
                if chosen.as_ref().is_none_or(|(_, best)| v > *best) {
                    chosen = Some((next, v));
                }
            }
            // Strict improvement only: a step that does not help is where the
            // line stops being worth extending.
            match chosen {
                Some((next, v)) if v > value => {
                    value = v;
                    cur = next;
                }
                _ => break,
            }
        }
        value
    }
}

/// `V8_DELTA_LEAF=1`. See `Search::scored_leaf`.
fn delta_leaf_enabled() -> bool {
    std::env::var_os("V8_DELTA_LEAF").is_some()
}

/// `V8_CHAIN=<depth>`. 0 (the default) leaves the breadth step exactly as it
/// was; 1 reproduces `with_followup`; 2 and up are the chain.
fn chain_depth() -> usize {
    std::env::var("V8_CHAIN")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0)
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
        root: gs.clone(),
        root_value: 0.0,
    };
    if delta_leaf_enabled() {
        // Charged to the budget like any other node: the root is evaluated once
        // per decision and its value is a constant across all candidate actions,
        // so it cannot change the argmax — but it must not be free either, or
        // `V8_DELTA_LEAF` silently buys depth.
        search.root_value = search.leaf(gs);
    }

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
            // `scored_leaf`, not `leaf`, and the distinction is load-bearing
            // under `V8_DELTA_LEAF`: every other action is scored as a change
            // from the root, so scoring Pass as a LEVEL would hand it a value of
            // `root_value` (a large positive number) while every deploy scored
            // near zero. Pass then wins unconditionally.
            //
            // It is easy to miss because the bug looks like a finding: v8 drops
            // from 43.7% to **6.7%** with pace 0.39 -> 0.20, which reads as
            // "delta leaves are catastrophic" until you notice the bot stopped
            // deploying entirely. It is a unit mismatch on one branch, not a
            // property of the idea.
            let value = search.scored_leaf(gs) - pass_tax();
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

        // `V8_CONVERT_GATE=1`: refuse a deploy that closes no part of the check
        // it is being built for.
        //
        // ## What the probe showed
        //
        // `PROBE=v9` puts v9's Main (v7's) and v8's Main on the same states and
        // logs every disagreement. Four games, four disagreements, **three of
        // them v9 PASSING where v8 keeps developing**:
        //
        // ```text
        // t=2 bl=4 cost=9  h=4 lives=3  v9=Pass v8=PlayMemberToStage
        // t=2 bl=6 cost=9  h=4 lives=1  v9=Pass v8=PlayMemberToStage
        // t=3 bl=8 cost=15 h=6 lives=0  v9=Pass v8=PlayMemberToStage
        // ```
        //
        // and the health metrics say v9 was right. Over those four games v9 went
        // **4-0, pace 0.800, stage-cost curve 4.0 / 9.0 / 14.0** — on the
        // guide's band. v8 on the same seeds reaches cost 30 and loses. A player
        // stops developing once the board is past what the check can use; v8
        // never does, and that is the difference.
        //
        // ## Why v8 cannot stop on its own
        //
        // Pass is scored by the SAME leaf on the UNCHANGED state, so it can only
        // win a tie, and `TieKey` breaks ties toward the lowest `cost_now` —
        // which is Pass's own value. Pass therefore needs a flat leaf, and v8's
        // leaf rises with `band_progress`, which deliberately keeps climbing
        // past the guide's turn band. A leaf that reads "more board is better"
        // cannot represent "stop". v7 escapes it with a depth-8 chain, not with
        // a Pass weight: a develop's `8.0 * dCost` is immediate, and a develop
        // that cannot be converted shows up several plies out.
        //
        // The gate states the rule directly — a deploy must close some of the
        // shortfall it is aimed at — which is what a player applies without
        // computing anything.
        //
        // Off by default: the probe evidence is 3 disagreements in 4 games,
        // which is a real signal and a small sample.
        if std::env::var_os("V8_CONVERT_GATE").is_some()
            && !apply_convert_gate(gs, &sim, me, db)
        {
            scores.push(ActionScore::rejected("past-the-check"));
            continue;
        }

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
        } else if chain_depth() > 0 {
            // The chain replaces the breadth step, it does not add to it: the
            // node budget is shared, so running both would only halve the depth
            // each gets. `V8_CHAIN=1` is exactly `with_followup` and exists so
            // the two can be compared at equal cost.
            search.chain(&sim)
        } else {
            search.with_followup(&sim)
        }
        // `V8_COVERAGE` is a DELTA, so it cannot live in `leaf_value` — that
        // function only ever sees a post-action state and has no base to
        // subtract from. Applying it here, where both `gs` and `sim` are in
        // hand, is the only place the difference is defined. The delta is
        // constant across the action's own search subtree, so it does not
        // distort anything the search chose; it only ranks the root actions,
        // which is what a tie-break term is for.
        + coverage_weight() * (coverage_delta(&sim, me, db) - coverage_delta(gs, me, db)) as f64
        // ...and the same for the features an ABILITY produced, which is where
        // more than half of v8's disagreements with v7 actually are.
        + ability_value_weight()
            * ((v8_model::active_blades(&sim, me, db) - v8_model::active_blades(gs, me, db)) as f64
                * ABILITY_BLADE_CREDIT
                + (v8_model::supply_hearts(&sim, me, db)
                    - v8_model::supply_hearts(gs, me, db)) as f64
                    * ABILITY_HEART_CREDIT)
        + v7_term_score(gs, &sim, me, db)
        + hand_quality_weight() * v8_model::hand_quality(gs, me, db) as f64;

/// The whole of v7's value function, term by term, in one line.
///
/// ## Why this exists
///
/// `v7_main::value` scores an action as a linear function of coarse feature
/// **deltas** — cost, hearts, blades, coverage, ammo, energy, hand, reserve,
/// future, success, and the opponent's cost, blades, hand and success. v8's
/// leaf is a saturated placement-probability level, and the two successful
/// changes in this file were both "give v8 a term it structurally could not
/// express" (§3.2 blades, §3.8 abilities). This is the rest of the vector, in
/// one place, so the whole of it can be switched on and the combination
/// measured rather than each term argued about in isolation.
///
/// `V7_LEAN` in the log is the reason to be careful: zeroing the seven minor
/// terms **together** costs v7 14 points while each is worth under 1.0 alone, so
/// per-term results here will mostly be flat and the combination is the
/// measurement that matters.
///
/// Each term is gated by its own `V8_TERM_*` switch and defaults to v7's own
/// weight once that switch is set, so a run is `V8_TERM_OPP_HAND` rather than a
/// hand-typed number. A weight of zero is a legitimate result here, not a
/// neutral one: it means the term is already accounted for elsewhere.
fn v7_term_score(base: &GameState, after: &GameState, me: u8, db: &CardDatabase) -> f64 {
    let w = v7_term_weights();
    if !w.any() {
        return 0.0;
    }
    let d = |f: fn(&GameState, u8, &CardDatabase) -> i32, seat: u8| {
        (f(after, seat, db) - f(base, seat, db)) as f64
    };
    let opp = 1 - me;
    w.cost * d(v8_model::stage_cost, me)
        + w.hearts * d(v8_model::supply_hearts, me)
        + w.blades * d(v8_model::active_blades, me)
        + w.coverage
            * (coverage_delta(after, me, db) - coverage_delta(base, me, db)) as f64
        + w.ammo
            * (v8_model::lives_in_hand(after.seat_player(me), db)
                - v8_model::lives_in_hand(base.seat_player(me), db)) as f64
        + w.energy
            * (after.seat_player(me).energy_zone.active_count() as i32
                - base.seat_player(me).energy_zone.active_count() as i32) as f64
        + w.hand
            * (after.seat_player(me).hand.cards.len() as i32
                - base.seat_player(me).hand.cards.len() as i32) as f64
        + w.reserve
            * (after.seat_player(me).main_deck.cards.len() as i32
                - base.seat_player(me).main_deck.cards.len() as i32) as f64
        + w.success
            * (after.seat_player(me).success_live_card_zone.cards.len() as i32
                - base.seat_player(me).success_live_card_zone.cards.len() as i32) as f64
        - w.opp_cost * d(v8_model::stage_cost, opp)
        - w.opp_blades * d(v8_model::active_blades, opp)
        - w.opp_hand
            * (after.seat_player(opp).hand.cards.len() as i32
                - base.seat_player(opp).hand.cards.len() as i32) as f64
        - w.opp_success
            * (after.seat_player(opp).success_live_card_zone.cards.len() as i32
                - base.seat_player(opp).success_live_card_zone.cards.len() as i32) as f64
}

/// The thirteen weights, read from the environment ONCE.
///
/// The first version read the environment inside the scoring function and built
/// each key with `format!`, which meant thirteen `String` allocations per action
/// evaluation — inside the search node loop, so roughly 13 * 10 * 96 per
/// decision. It was correct and it made `v10` so slow the arena could not finish
/// 4000 games in a reasonable time, which is its own kind of evidence: a term
/// that cannot be evaluated inside a search should not be in the search.
///
/// The environment is fixed for the process (the arena sets it before any game
/// and never changes it), so caching is not a staleness risk. `OnceLock` also
/// makes it the only thread-safe thing here, which matters because a policy
/// call can run on a worker.
#[derive(Clone, Copy, Default)]
struct V7TermWeights {
    cost: f64,
    hearts: f64,
    blades: f64,
    coverage: f64,
    ammo: f64,
    energy: f64,
    hand: f64,
    reserve: f64,
    success: f64,
    opp_cost: f64,
    opp_blades: f64,
    opp_hand: f64,
    opp_success: f64,
}

impl V7TermWeights {
    fn any(&self) -> bool {
        self.cost != 0.0
            || self.hearts != 0.0
            || self.blades != 0.0
            || self.coverage != 0.0
            || self.ammo != 0.0
            || self.energy != 0.0
            || self.hand != 0.0
            || self.reserve != 0.0
            || self.success != 0.0
            || self.opp_cost != 0.0
            || self.opp_blades != 0.0
            || self.opp_hand != 0.0
            || self.opp_success != 0.0
    }
}

fn v7_term_weights() -> &'static V7TermWeights {
    static WEIGHTS: std::sync::OnceLock<V7TermWeights> = std::sync::OnceLock::new();
    WEIGHTS.get_or_init(|| {
        // `V8_ALL_TERMS` turns the whole vector on at v7's own weights, which is
        // what the confirmed +5.40 / +5.67 runs used. One switch instead of
        // thirteen, and `v10` — which exists to carry exactly this configuration
        // — does not need a second copy of the weights.
        let all = std::env::var_os("V8_ALL_TERMS").is_some();
        let weight = |name: &str, default: f64| -> f64 {
            let key = format!("V8_TERM_{name}");
            if !all && std::env::var_os(&key).is_none() {
                return 0.0;
            }
            std::env::var(&key)
                .ok()
                .and_then(|v| v.parse::<f64>().ok())
                .filter(|w: &f64| w.is_finite())
                .unwrap_or(default)
        };
        V7TermWeights {
            cost: weight("COST", 8.0),
            hearts: weight("HEARTS", 3.0),
            blades: weight("BLADES", 6.0),
            coverage: weight("COVERAGE", 60.0),
            ammo: weight("AMMO", 25.0),
            energy: weight("ENERGY", 2.0),
            hand: weight("HAND", 3.0),
            reserve: weight("RESERVE", 0.5),
            success: weight("SUCCESS", 1000.0),
            opp_cost: weight("OPP_COST", 8.0),
            opp_blades: weight("OPP_BLADES", 6.0),
            opp_hand: weight("OPP_HAND", 10.0),
            opp_success: weight("OPP_SUCCESS", 1000.0),
        }
    })
}
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

/// What a `UseAbility` is paid, per unit of what it actually produced.
///
/// ## The located defect
///
/// `V8_PROBE_V7` put the same states to both policies and found that of 14
/// disagreements in three games, **8 were v7 activating an ability and v8
/// playing something else or passing**. Not a close split: more than half.
///
/// The cause is in §3.1 of the log. `v7_main::value` is linear in feature
/// deltas, so an ability that resolves into a blade scores `6.0 * dBlades`
/// immediately, and one that resolves into a heart scores `3.0 * dHearts`.
/// v8's leaf is a saturated placement-probability level, so the identical
/// ability scores **zero** unless it happens to cross a probability threshold —
/// and most do not, because a single blade rarely moves a binomial that is
/// already comfortable. v8 is not discouraged from activating. It cannot see
/// what activating did.
///
/// ## Why this is not the "encourage abilities" experiment
///
/// That one is recorded null: a turn-window bonus for activating was worth
/// +0.086 at T3 and +0.147 at T6, but that is *encouragement* — a bonus for the
/// act. This is *pricing* — the same treatment the board's own blades already
/// get from `V8_BLADES`, applied to the blades an ability creates. Paying a
/// move for the state change it produces is an intervention that has already
/// worked once in this file; paying it for being a move is the one that
/// measured null.
///
/// The credits are v7's own weights, not tuned.
const ABILITY_BLADE_CREDIT: f64 = 6.0;
const ABILITY_HEART_CREDIT: f64 = 3.0;

fn ability_value_weight() -> f64 {
    std::env::var("V8_ABILITY_VALUE")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|w: &f64| w.is_finite() && *w >= 0.0)
        .unwrap_or(0.0)
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
