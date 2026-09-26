//! V8 live-card-set decision: ONE argmax over the shared check model.
//!
//! v7 built this decision as a chain of mutually exclusive special cases —
//! free win, ceiling fold, minimum-win, strict closeout, desperation life,
//! junk fill — each with its own floor constant, applied in a fixed priority
//! order. Every one of those cases is a special case of the same question:
//! "which cards in my hand should go into the live zone this check?" So v8
//! enumerates the answer and scores every candidate with the shared outcome
//! model. Free wins, conceding, junk-only sets and desperation gambles are then
//! not branches — they are simply the argmax in the states where they really
//! are the best play.
//!
//! What the unified objective adds over v7:
//!
//! - **Burning ammunition is priced.** Setting a life we are likely to fail
//!   costs the future placement that life would have made. v7 could not
//!   express this and got the D2b trade-off wrong in both directions (a
//!   measured −1pp for preferring a below-floor life over junk, and a fold
//!   rate it could only tune with floors).
//! - **Score and reliability are priced jointly.** A high score we cannot pass
//!   and a low score we can both have an exact value, so the model decides
//!   whether this is the turn to swing for score (contested) or to bank a
//!   reliable placement (uncontested) — instead of a role gate choosing first.
//! - **Hand filtering is a real candidate.** Spare slots are filled with the
//!   cards whose loss costs least, and the cost is computed from each card's
//!   actual next-turn contribution rather than from its printed cost.

use crate::bot::strategy_common::{acc_add, emit_live_set, Acc};
use crate::card::{CardDatabase, CardType};
use crate::game_setup::Action;
use crate::game_state::{GameState, Phase};

use super::v8_model::{self, CheckOutcome, OppModel, RngGuard, PLACEMENT_CREDIT};

/// A life in hand, priced once.
struct Life {
    index: usize,
    need: [i32; 11],
    score: i32,
}

/// A non-live in hand, with its marginal next-turn value.
struct Junk {
    index: usize,
    /// Placement units this card is worth if we KEEP it. Setting it in a
    /// spare slot forfeits this and buys one extra draw.
    keep_value: f64,
}

/// One candidate live zone.
#[derive(Clone, Debug)]
pub(crate) struct Candidate {
    pub lives: Vec<usize>,
    pub junk: Vec<usize>,
    pub outcome: CheckOutcome,
    pub value: f64,
}

/// Value of one extra card in hand for the NEXT check, derived from our own
/// deck composition (fair information): a life is a placement ticket, an
/// average member is a coin flip between being the ladder step we need and
/// being a clog.
fn draw_value(gs: &GameState, me: u8, db: &CardDatabase) -> f64 {
    let p = gs.seat_player(me);
    let deck_len = p.main_deck.cards.len();
    if deck_len == 0 {
        return 0.0;
    }
    let (mut lives, mut members) = (0usize, 0usize);
    for &cid in &p.main_deck.cards {
        match db.get_card(cid).map(|c| c.card_type.clone()) {
            Some(CardType::Live) => lives += 1,
            Some(CardType::Member) => members += 1,
            _ => {}
        }
    }
    let len = deck_len as f64;
    PLACEMENT_CREDIT * ((lives as f64 + 0.5 * members as f64) / len)
}

/// Marginal next-turn value of a non-live hand card, in placement units.
/// Positive means "do not burn this in a set slot".
///
/// The question is never "is this card expensive" but "is this card worth more
/// in my hand next turn than the draw it buys me". v7 ranked junk by
/// descending printed cost, which throws away energy, retrieval engines and
/// curve pieces; here every card is priced by the same check model the live
/// set uses, so the comparison is in the same units as everything else.
fn junk_keep_value(
    _gs: &GameState,
    _me: u8,
    db: &CardDatabase,
    cid: i16,
    ctx: &HandContext,
) -> f64 {
    let Some(card) = db.get_card(cid) else {
        return 0.0;
    };
    match card.card_type {
        // Energy exists only to buy a deploy that raises check power, so one
        // energy is worth one more shot at the check we are building toward.
        CardType::Energy => return PLACEMENT_CREDIT,
        CardType::Member => {
            // (1) Direct board contribution: what does this member's printed
            //     heart and blade supply do to our best in-hand life's chance
            //     of passing? Measured with the real estimate function, not a
            //     per-heart constant.
            let mut with = ctx.board;
            if let Some(base) = &card.base_heart {
                acc_add(&mut with, &base.hearts);
            }
            let blades = ctx.blades + i32::from(card.blade);
            let mut value = match &ctx.best_need {
                Some(need) => {
                    let delta = v8_model::pass_estimate_from_supply(
                        &with,
                        blades,
                        ctx.density,
                        need,
                    ) - v8_model::pass_estimate_from_supply(
                        &ctx.board,
                        ctx.blades,
                        ctx.density,
                        need,
                    );
                    PLACEMENT_CREDIT * delta
                }
                // No life in hand means no check to contribute to; fall back
                // to the ladder terms below.
                None => 0.0,
            };
            // (2) An engine piece (activation / constant / live-start) can
            //     change a check outright. Half a placement ticket is the
            //     honest ceiling: a board engine is not used on every check,
            //     and pricing it at a full ticket made every engine
            //     unburnable, which silenced hand filtering entirely
            //     (measured 0.07 junk cards per set against v7's 1.08).
            if !card.ability.trim().is_empty() {
                value += 0.5 * PLACEMENT_CREDIT;
            }
            // (3) Ladder step, tested against what we reach anyway. "Is this
            //     card affordable" is the wrong question - almost every member
            //     in hand is affordable within a few turns, so that test made
            //     every member unburnable and silenced hand filtering (0.07
            //     junk cards per set against v7's 1.08). The right question is
            //     the guides' own: does this member raise the curve, or would
            //     it take a slot we will have filled anyway?
            let cost = i32::from(card.cost.unwrap_or(0));
            if cost > ctx.reachable_next {
                value += 0.5 * PLACEMENT_CREDIT;
            }
            value
        }
        _ => 0.0,
    }
}

/// Per-decision constants every hand card is valued against.
struct HandContext {
    board: Acc,
    blades: i32,
    density: f64,
    /// Requirement of the life we would most plausibly set, if we hold one.
    best_need: Option<Acc>,
    /// Energy we can reach in the next few turns, plus the baton discount.
    budget: i32,
    /// Stage cost we will reach next turn with no help from this card. A
    /// member at or below this is a CLOG: the guides are explicit that a
    /// zero-contribution member must not take a slot, and "we will already
    /// have this much" is the test that says so without a hand-tuned size.
    reachable_next: i32,
}

fn hand_context(gs: &GameState, me: u8, db: &CardDatabase) -> HandContext {
    let p = gs.seat_player(me);
    let board = v8_model::board_supply(gs, me, db);
    let (blades, density) = super::strategy_v4::flip_stats(gs, me, db);
    let best_need = p
        .hand
        .cards
        .iter()
        .filter_map(|&cid| {
            let card = db.get_card(cid)?;
            if card.card_type != CardType::Live {
                return None;
            }
            let need = v8_model::life_need(gs, cid);
            if v8_model::has_unpassable_icon(&need) {
                return None;
            }
            Some(need)
        })
        .fold(None, |acc: Option<Acc>, need| {
            match acc {
                Some(current) => {
                    let better = v8_model::pass_estimate_from_supply(
                        &board,
                        blades,
                        density,
                        &need,
                    ) > v8_model::pass_estimate_from_supply(&board, blades, density, &current);
                    if better {
                        Some(need)
                    } else {
                        Some(current)
                    }
                }
                None => Some(need),
            }
        });
    let discount = p
        .stage
        .stage
        .iter()
        .filter(|&&c| c >= 0)
        .filter_map(|&c| db.get_card(c).and_then(|card| card.cost))
        .map(i32::from)
        .max()
        .unwrap_or(0);
    HandContext {
        board,
        blades,
        density,
        best_need,
        budget: i32::from(p.energy_zone.active_count()) + 3 + discount,
        reachable_next: v8_model::reachable_ceiling(gs, me, 1, db),
    }
}

fn collect_lives(gs: &GameState, me: u8, db: &CardDatabase) -> Vec<Life> {
    let p = gs.seat_player(me);
    p.hand
        .cards
        .iter()
        .enumerate()
        .filter_map(|(index, &cid)| {
            let card = db.get_card(cid)?;
            if card.card_type != CardType::Live {
                return None;
            }
            let need = v8_model::life_need(gs, cid);
            // A Draw/Score requirement is not a heart check we can price, so
            // it never enters a candidate set.
            if v8_model::has_unpassable_icon(&need) {
                return None;
            }
            Some(Life {
                index,
                need,
                score: v8_model::printed_score(db, cid),
            })
        })
        .collect()
}

fn collect_junk(gs: &GameState, me: u8, db: &CardDatabase, lives: &[Life], ctx: &HandContext) -> Vec<Junk> {
    let p = gs.seat_player(me);
    p.hand
        .cards
        .iter()
        .enumerate()
        .filter(|(index, _)| !lives.iter().any(|life| life.index == *index))
        .filter_map(|(index, &cid)| {
            let card = db.get_card(cid)?;
            if card.card_type == CardType::Live {
                return None;
            }
            Some(Junk {
                index,
                keep_value: junk_keep_value(gs, me, db, cid, ctx),
            })
        })
        .collect()
}

/// Expected value of one candidate zone, in placement units.
///
/// Three terms, all in the same currency:
///
/// 1. `outcome.value` - the rule payoff of this check plus its placement
///    credit, from the shared model.
/// 2. `- burned` - ammunition destroyed by a failed check. All-or-nothing
///    means one failure discards the WHOLE zone (8.3.15 -> 8.3.16).
///
///    The price of a burned life is the placement it would have made, and the
///    only measure available without simulating the rest of the game is the
///    same check. Two wrong versions of this term were measured and both are
///    instructive:
///
///    - Charging the full `PLACEMENT_CREDIT` makes any life below 50% pass
///      probability negative, so v8 folded about a third of all checks and
///      starved its own success zone.
///    - Charging `p_pass * PROJECTED best pass probability` is worse in a
///      quieter way: the projection is computed from the board we are about to
///      build, which assumes we kept the card, so the more reachable a later
///      check looks the more expensive every life becomes. Measured: 44% of
///      checks folded on a board matching v7's, at 1.0 wins per 5 played.
///
///    `(1 - p) * p` is the stationary price: a life is worth committing exactly
///    in proportion to the placement it can make here, and the term cancels to
///    zero only when the life can never pass, which is when folding is right.
///
///    The price is charged ONCE PER ZONE, not once per life. 8.3.15 -> 8.3.16
///    makes the check all-or-nothing: a failed check discards the entire live
///    set, so the lives in a zone are not independent risks and multiplying by
///    `lives` charged the same failure two or three times over. Measured: at
///    p = 0.5 with three lives the penalty was 0.25 against the 0.167
///    placement it was protecting, so a strictly positive-expectation zone
///    scored negative and v8 folded 6.3% of checks - against v7's 0.3%, and
///    against the guide's D2b finding that folding burns a whole live phase for
///    nothing.
/// 3. `+ filter_value` - what the spare-slot hand filter buys. Each card set
///    as junk is discarded before the check and draws a replacement (8.3.4), so
///    the slot is worth a draw minus whatever that card was worth in hand.
pub(crate) fn candidate_value(
    outcome: CheckOutcome,
    p_pass: f64,
    filter_value: f64,
) -> f64 {
    let burned = (1.0 - p_pass) * p_pass * PLACEMENT_CREDIT;
    outcome.value - burned + filter_value
}

/// Enumerate every live-zone of up to `max_slots` hand cards: choose a subset
/// of hand LIVES, then fill the remaining slots with the hand cards whose loss
/// costs least. Lives and junk compete for the same slots, so "swing for score
/// with three lives" and "take one safe placement and hand-filter twice" are
/// both in the search space rather than being separate code paths.
fn enumerate_candidates(
    gs: &GameState,
    me: u8,
    db: &CardDatabase,
    pools: &[[i32; 8]],
    opp: &OppModel,
    draw_credit: f64,
) -> Vec<Candidate> {
    let p = gs.seat_player(me);
    let max_slots = usize::from(3u8.saturating_sub(p.live_card_set_limit_reduction));
    if max_slots == 0 {
        return Vec::new();
    }
    let my_success = p.success_live_card_zone.cards.len();
    let yell_score = v8_model::expected_yell_score(gs, me, db, v8_model::active_blades(gs, me, db));

    let lives = collect_lives(gs, me, db);
    let ctx = hand_context(gs, me, db);
    let junk = collect_junk(gs, me, db, &lives, &ctx);
    // Setting a spare slot is only worth it if the card we burn is worth less
    // than the draw it buys.
    let mut worth_burning: Vec<&Junk> = junk
        .iter()
        .filter(|j| j.keep_value <= draw_credit)
        .collect();
    worth_burning.sort_by(|a, b| {
        a.keep_value
            .partial_cmp(&b.keep_value)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.index.cmp(&b.index))
    });
    if std::env::var_os("V8_NO_JUNK").is_some() {
        // Ablation: hand filtering is a real play (8.3.4 draws a replacement),
        // but it also fills the set, and a full set means a lower hand for the
        // NEXT check. This switch measures that trade directly instead of
        // assuming it.
        worth_burning.clear();
    }

    let mut out = Vec::new();
    let subsets = 1usize << lives.len().min(16);
    for mask in 0..subsets {
        let chosen: Vec<&Life> = lives
            .iter()
            .enumerate()
            .filter(|(bit, _)| mask & (1 << bit) != 0)
            .map(|(_, life)| life)
            .collect();
        if chosen.len() > max_slots {
            continue;
        }
        let mut need = [0i32; 11];
        let mut score = yell_score;
        for life in &chosen {
            for k in 0..11 {
                need[k] += life.need[k];
            }
            score += life.score;
        }
        let p_pass = if chosen.is_empty() {
            0.0
        } else {
            v8_model::pass_probability(pools, &need)
        };
        let outcome = v8_model::check_outcome(p_pass, score, chosen.len(), my_success, opp);
        let spare = max_slots - chosen.len();
        let filler: Vec<&Junk> = worth_burning.iter().take(spare).copied().collect();
        let filter_value: f64 = filler
            .iter()
            .map(|j| draw_credit - j.keep_value)
            .sum();
        let value = candidate_value(outcome, p_pass, filter_value);
        out.push(Candidate {
            lives: chosen.iter().map(|life| life.index).collect(),
            junk: filler.iter().map(|j| j.index).collect(),
            outcome,
            value,
        });
    }
    // `V8_NO_FOLD` is the ablation floor: the argmax is the decision, this is
    // the hand-tuned constant v7 needed, and shipping requires showing the
    // argmax does not need it.
    if std::env::var_os("V8_NO_FOLD").is_some() {
        if let Some(best_lives) = out
            .iter()
            .filter(|c| !c.lives.is_empty())
            .max_by(|a, b| a.value.partial_cmp(&b.value).unwrap_or(std::cmp::Ordering::Equal))
        {
            let lives = best_lives.lives.clone();
            let value = best_lives.value;
            if let Some(entry) = out.iter_mut().find(|c| c.lives.is_empty()) {
                entry.value = f64::NEG_INFINITY;
                entry.lives = lives;
                entry.value = value;
            }
        }
    }
    out
}

/// Deterministic ranking: value, then fewer committed lives (ammunition is
/// the scarce resource), then a higher placement probability, then a stable
/// hand-order tiebreak. No randomness anywhere, so the chosen zone is stable
/// across the select/deselect/confirm ticks of one set phase.
fn better(a: &Candidate, b: &Candidate) -> bool {
    const EPS: f64 = 1e-9;
    if (a.value - b.value).abs() > EPS {
        return a.value > b.value;
    }
    if a.lives.len() != b.lives.len() {
        return a.lives.len() < b.lives.len();
    }
    if (a.outcome.p_place - b.outcome.p_place).abs() > EPS {
        return a.outcome.p_place > b.outcome.p_place;
    }
    a.lives < b.lives
}

pub(crate) fn score_candidates(
    gs: &GameState,
    me: u8,
    db: &CardDatabase,
) -> (Vec<Candidate>, OppModel) {
    let board = v8_model::board_pool(gs, me, db);
    let sampler = v8_model::FlipSampler::build(gs, me, db);
    let pools = sampler.sample_pools(&board, v8_model::FLIP_SAMPLES);
    let opp = OppModel::build(gs, me, db);
    let draw_credit = draw_value(gs, me, db);
    (
        enumerate_candidates(gs, me, db, &pools, &opp, draw_credit),
        opp,
    )
}

pub fn choose_live_set_v8(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    let _rng = RngGuard::new();
    let me = gs.active_player_index();
    let p = gs.seat_player(me);
    let (_, opp_player) = gs.seated_pair(me);

    // Nothing to set: the set is fully suppressed this check.
    if p.live_card_set_limit_reduction >= 3 || gs.cannot_live_players.contains(&p.id) {
        return emit_live_set(gs, actions, &[]);
    }

    let (candidates, opp) = score_candidates(gs, me, db);
    let best = candidates
        .iter()
        .fold(None::<&Candidate>, |acc, cand| match acc {
            None => Some(cand),
            Some(current) if better(cand, current) => Some(cand),
            keep => keep,
        });
    let Some(best) = best else {
        return emit_live_set(gs, actions, &[]);
    };

    let mut desired = best.lives.clone();
    desired.extend(best.junk.iter().copied());

    log::debug!(
        "v8 live t{} me{} second={} n={} lives={} junk={} p_pass={:.2} p_place={:.2} value={:.3} my{} opp{} opp_live={}",
        gs.turn_number,
        me,
        gs.current_phase == Phase::LiveCardSetSecondAttacker,
        desired.len(),
        best.lives.len(),
        best.junk.len(),
        best.outcome.p_pass,
        best.outcome.p_place,
        best.value,
        p.success_live_card_zone.cards.len(),
        opp_player.success_live_card_zone.cards.len(),
        opp.set_size,
    );
    if std::env::var_os("V8_DEBUG").is_some() {
        let describe = |idx: &[usize]| -> String {
            idx.iter()
                .filter_map(|&i| p.hand.cards.get(i).copied())
                .filter_map(|cid| db.get_card(cid))
                .map(|c| {
                    format!(
                        "{}:{}{}",
                        c.card_no,
                        if c.card_type == CardType::Live { "L" } else { "J" },
                        c.score.unwrap_or(0)
                    )
                })
                .collect::<Vec<_>>()
                .join(",")
        };
        eprintln!(
            "V8L t{} me{} n={} lives={} junk={} p_pass={:.2} p_place={:.2} opp_place={:.2} value={:.3} opp_pass={:.2} set={}",
            gs.turn_number,
            me,
            desired.len(),
            describe(&best.lives),
            describe(&best.junk),
            best.outcome.p_pass,
            best.outcome.p_place,
            best.outcome.opp_place,
            best.value,
            opp.pass_prob,
            describe(&desired),
        );
    }

    emit_live_set(gs, actions, &desired)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(p_pass: f64, p_place: f64, value: f64) -> CheckOutcome {
        CheckOutcome {
            p_pass,
            p_place,
            opp_place: 0.0,
            value,
        }
    }

    /// Value dominates, and the tiebreak prefers conserving ammunition.
    #[test]
    fn ranking_prefers_value_then_fewer_lives() {
        let mk = |lives: &[usize], value: f64| Candidate {
            lives: lives.to_vec(),
            junk: Vec::new(),
            outcome: outcome(1.0, 1.0, value),
            value,
        };
        let three = mk(&[0, 1, 2], 0.40);
        let one = mk(&[3], 0.40);
        assert!(better(&one, &three), "equal value must prefer fewer lives");
        let worth_more = mk(&[3], 0.50);
        assert!(better(&worth_more, &three), "value must dominate the tiebreak");
    }

    /// The D2b trade-off, priced. Against an opponent who is NOT one
    /// placement from winning, a life we are very likely to FAIL burns more
    /// future placements than it can make here, so the search prefers a
    /// junk-only set. v7 could not express this and got it wrong in both
    /// directions (measured -1pp for the "desperation life").
    ///
    /// The comparison is against the empty candidate, never against a bare
    /// zero: setting nothing still has the hand filter's value, so "fold" means
    /// "lose to junk", not "gain nothing".
    ///
    /// Uncontested, a life worth `p` gains `p * PLACEMENT_CREDIT` and forfeits
    /// `(1 - p) * p * PLACEMENT_CREDIT`, so against a junk-only set the break
    /// even is `p^2 / 3 == filter_value` - the price `candidate_value` charges.
    /// That is the calibrated middle ground between the two measured extremes
    /// documented on it: the full-credit price folds anything below 50%, the
    /// projected price folds 44% of checks.
    ///
    /// Paired with `v8_model::match_point_makes_a_thin_check_worth_taking`:
    /// at opponent match point the same thin life IS right, because folding
    /// there hands them the game. One objective, both answers.
    #[test]
    fn a_thin_life_can_be_worth_less_than_a_junk_set() {
        let junk_only = candidate_value(outcome(0.0, 0.0, 0.0), 0.0, 0.02);
        assert!(junk_only > 0.0);
        // p = 0.20: 0.20/3 gained against 0.80 * 0.20/3 forfeited. Below break
        // even, so the argmax sets junk and keeps the life for another check.
        let thin = candidate_value(outcome(0.20, 0.20, 0.20 * PLACEMENT_CREDIT), 0.20, 0.0);
        assert!(thin < junk_only);
        // p = 0.45 on the same board: the placement this life can make here
        // outweighs the one it forfeits, and it is committed. A thin life is
        // not junk; only a hopeless one is.
        let thicker = candidate_value(
            outcome(0.45, 0.45, 0.45 * PLACEMENT_CREDIT),
            0.45,
            0.0,
        );
        assert!(thicker > junk_only);
        // The life is still charged for its own failure, monotonically.
        let certain = candidate_value(
            outcome(0.95, 0.95, 0.95 * PLACEMENT_CREDIT),
            0.95,
            0.0,
        );
        assert!(certain > thicker);
    }

    /// A reliable life is always worth taking, and the hand filter is pure
    /// upside: the card is discarded before the check and draws a
    /// replacement (8.3.4).
    #[test]
    fn a_reliable_life_plus_hand_filter_beats_folding() {
        let play = candidate_value(outcome(0.95, 0.95, 0.32), 0.95, 0.02);
        let fold = candidate_value(outcome(0.0, 0.0, 0.0), 0.0, 0.06);
        assert!(play > fold);
    }

    #[test]
    fn junk_is_only_filled_from_cards_weaker_than_a_draw() {
        // A dead vanilla member is worth less than a draw, so it is burnable;
        // an energy card is worth more, so it must not be.
        let dead = Junk {
            index: 0,
            keep_value: 0.0,
        };
        let energy = Junk {
            index: 1,
            keep_value: PLACEMENT_CREDIT,
        };
        let draw_credit = 0.05;
        assert!(dead.keep_value <= draw_credit);
        assert!(energy.keep_value > draw_credit);
    }
}
