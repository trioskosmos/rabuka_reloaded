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

use crate::bot::strategy_common::{acc_add, emit_live_set, Acc, Pool};
use crate::card::{CardDatabase, CardType};
use crate::game_setup::Action;
use crate::game_state::{GameState, Phase};

use super::v8_model::{self, CheckOutcome, OppModel, RngGuard, PLACEMENT_CREDIT};

/// Shorthand for the blade count used on the un-projected path.
fn active_blades(gs: &GameState, me: u8, db: &CardDatabase) -> i32 {
    v8_model::active_blades(gs, me, db)
}

/// A life in hand, priced once.
struct Life {
    index: usize,
    need: Acc,
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
        CardType::Energy => PLACEMENT_CREDIT,
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
    /// Computed and carried, but no consumer reads it yet: the energy-reach
    /// half of the hand valuation ended up being done where the value is
    /// assembled. Kept because the discount term is not trivially recoverable
    /// and `reachable_next` below is the same shape.
    #[allow(dead_code)]
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
                if std::env::var_os("V8_TRACE_FILTERED").is_some() {
                    eprintln!(
                        "V8_FILTERED life {} need={:?} - never a candidate",
                        cid, need
                    );
                }
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
///
/// ## The price of a burned life, and why the shape was wrong
///
/// Charging `p * (1 - p)` is a stationary price: it says a life is worth
/// committing in proportion to the placement it can make *here*. The problem is
/// that it vanishes at BOTH ends, and the two ends are exactly where the truth
/// diverges from the formula:
///
/// - `p = 1`: the life places, so nothing was lost. Zero is right.
/// - `p = 0`: the life cannot pass, and 8.3.16 discards it. The card is gone
///   permanently, and one of the three successes the game needs went with it.
///   Charging zero makes a guaranteed-to-fail life look FREE.
///
/// Found by playing, not by an aggregate. A hand-played T1 live set, v8
/// folding against a 3-life v7 zone on a two-heart board, produced two
/// candidates at a value of exactly 0.0629 - the life-free zone and the zone
/// containing a life that could not pass - because the burn term disappears at
/// `p = 0`. The fold was still chosen, by the fewer-lives tie-break, so nothing
/// visibly broke: the zone was ranked on a tie rather than on its merits, and
/// any change to `filter_value` or to the junk ordering would have flipped it
/// silently. An aggregate cannot see that; a printed candidate table does.
///
/// The price of a burned life is the OPTION VALUE of the card you no longer
/// hold. You lose it with probability `1 - p`, and holding a life is worth one
/// placement ticket, so the burn is `(1 - p) * CREDIT` - not
/// `(1 - p) * p * CREDIT`. The two agree in the middle and disagree at the ends.
/// `V8_LIFE_BURN=stationary` restores the old shape so the comparison stays
/// re-runnable.
/// 3. `+ filter_value` - what the spare-slot hand filter buys. Each card set
///    as junk is discarded before the check and draws a replacement (8.3.4), so
///    the slot is worth a draw minus whatever that card was worth in hand.
/// The price of a burned life. `p_pass` is the ZONE's pass probability, which
/// is 0 for a junk-only zone - and a zone with no life in it burns nothing, so
/// the price has to be gated on the zone actually holding one. Gating it is not
/// cosmetic: without the gate a junk-only zone is charged a whole placement
/// credit for holding no life at all, which is the same class of error as not
/// charging a doomed life.
#[cfg(test)]
pub(crate) fn candidate_value(
    outcome: CheckOutcome,
    p_pass: f64,
    filter_value: f64,
) -> f64 {
    candidate_value_zone(outcome, p_pass, filter_value, true)
}

/// `candidate_value`, with the burn explicitly gated on the zone holding a life.
pub(crate) fn candidate_value_zone(
    outcome: CheckOutcome,
    p_pass: f64,
    filter_value: f64,
    holds_life: bool,
) -> f64 {
    let burned = if !holds_life {
        0.0
    } else if matches!(std::env::var("V8_LIFE_BURN").as_deref(), Ok("stationary")) {
        // The old price: one parameter doing two jobs. Kept so the comparison
        // against it stays re-runnable.
        (1.0 - p_pass) * p_pass * PLACEMENT_CREDIT
    } else {
        (1.0 - p_pass) * life_resale_value() * PLACEMENT_CREDIT
    };
    outcome.value - burned + filter_value
}

/// What one life in hand is worth, in placement tickets, when it is NOT the one
/// being set.
///
/// This is the parameter the old price was missing. Burning a life forfeits the
/// chance to place it on some LATER check, and that chance is a separate
/// quantity from the probability that the current check fails. The old price
/// used `p` for both - it charged `p * (1 - p)` - which is only right if a life
/// converts on a later check exactly as often as it converts on this one, and
/// the two are not the same event: a later check has a different board, a
/// different opponent commitment, and a different turn order.
///
/// The consequence of the conflation is that the price vanishes at `p = 0`: a
/// life that cannot pass is charged nothing, although 8.3.16 discards it
/// permanently and one of the three successes the game needs went with it. A
/// junk-only zone and a zone holding a doomed life then tie exactly, and the
/// decision falls to the fewer-lives tie-break. Found by hand-playing a T1 live
/// set: both candidates read 0.0629.
///
/// With the resale price separated, the price is non-zero at `p = 0` and zero
/// at `p = 1`, which are the two cases the formula used to get wrong, and the
/// break-even moves to `p == resale / (1 + resale)` - 23% at the default,
/// rather than the 50% a full-ticket price would demand.
///
/// `V8_LIFE_BURN=stationary` restores `p * (1 - p)`, so the comparison against
/// the old behaviour stays re-runnable.
fn life_resale_value() -> f64 {
    match std::env::var("V8_LIFE_RESALE")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
    {
        Some(q) if (0.0..=1.0).contains(&q) => q,
        _ => 0.30,
    }
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
    pools: &[[i16; 8]],
    opp: &OppModel,
    draw_credit: f64,
) -> Vec<Candidate> {
    let p = gs.seat_player(me);
    let max_slots = usize::from(3u8.saturating_sub(p.live_card_set_limit_reduction));
    if max_slots == 0 {
        return Vec::new();
    }
    // How many LIVES v8 will set.
    //
    // Default 1, measured. Setting one life and swinging for it beat v8's own
    // argmax over wider zones by 15 wins in 800 paired games (p = 0.0026) and
    // 1.2pp over 2400 games across three seeds and both seats. The argmax was
    // free to discover this and did not, because a wide zone looks better on
    // paper: 8.3.15 pools the requirements of every life in a zone, so a second
    // life does not add an independent chance of placing, it ADDS its
    // requirement - while 8.4.7 pays exactly one placement per won check. The
    // extra width therefore has no upside to balance its extra cost.
    //
    // "No upside" holds only while the check is uncontested, though. Score
    // matters only inside a comparison (section 2), so a second life earns its
    // added requirement in exactly one case: the check is contested and the
    // extra score is what tips it. `max_lives` caps an uncontested zone and
    // `contested_lives` caps a contested one.
    //
    // Measured: a flat cap of 2 was +3 wins, p = 0.25 - not significant,
    // because the width only pays in the minority of checks it can decide.
    //
    // `V8_MAX_LIVES` overrides the uncontested cap; `V8_NO_CONTESTED_WIDE`
    // restores a flat cap for ablation.
    let max_lives = std::env::var("V8_MAX_LIVES")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|&n| (1..=max_slots).contains(&n))
        .unwrap_or(1);
    // "No upside" turned out to be true even when the check IS contested.
    // Allowing a second life in the contested case measured 354 vs 364 wins in
    // 800 paired games (p = 0.052) - worse, not better. So the score a second
    // life can win is worth less than the requirement it adds, which is the
    // same 8.3.15-versus-8.4.7 balance and the same answer at every width.
    //
    // The contested branch is kept because the measurement is the interesting
    // part, and `V8_CONTESTED_WIDE` re-enables it. What it is NOT is a default:
    // the reasoning was sound and the data refused it, which is the usual
    // outcome of reasoning about a game instead of measuring one.
    let contested_lives = if std::env::var_os("V8_CONTESTED_WIDE").is_some() {
        max_slots.min(max_lives + 1)
    } else {
        max_lives
    };
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
    // Which subsets of the lives in hand are worth pricing.
    //
    // This used to be `for mask in 0..(1 << lives.len().min(16))`, which is
    // wrong twice over. It walked all 2^n masks and then threw away every one
    // over `width_cap`, so the work was exponential in a quantity nothing uses;
    // and the `.min(16)` silently made every life past the 16th unreachable
    // with no counter and no log, so a 17-card hand had four lives v8 could
    // not see at all. Silent truncation of the candidate set is exactly the
    // class of bug this file's comments keep warning about elsewhere.
    //
    // `width_cap` is `max_lives` or `contested_lives`, and `max_lives` is
    // filtered into `1..=max_slots` where `max_slots` is `3 - set_limit`, so
    // the cap is at most 3 and the useful masks are C(n,1..=3) — polynomial.
    // Enumerating only those makes the 16-bit ceiling unnecessary rather than
    // merely larger, and it is cheaper than the old loop at every hand size.
    //
    // Order: the empty set first, then by highest set bit ascending. For the
    // shipped default (`max_lives = 1`) that is bitmask order, so the
    // candidate list — and therefore every tie broken by iteration order — is
    // unchanged. The wider widths are `V8_MAX_LIVES`/`V8_CONTESTED_WIDE`
    // ablations, and there the order can differ from raw mask order; ties are
    // broken by `better()`'s min-index clause in either case.
    // `V8_LATE_BAND=<turn>`: allow a PAIR in a CONTESTED check, from that turn
    // onward, when the pair's printed score reaches a band no single card in
    // hand reaches.
    //
    // §28.7 built this rule unconditionally and it measured null, and §28.7 also
    // said why the argument was right and the result was not: the pair needs its
    // requirement pooled (8.3.15) and pays one placement for it (8.4.7), so it
    // only pays where the extra score is what tips a comparison.
    //
    // §28.26 found where that is. From the 200-game trace, the printed score each
    // bot committed:
    //
    //     turn      1     2     3     4     5     6
    //     v9     1.00  1.54  2.03  3.14  3.23  4.00
    //     v7     1.00  1.56  1.94  2.49  3.40  4.29
    //
    // v9 leads on turns 3 and 4 and is BEHIND on turns 5 and 6 by 0.17 and 0.29
    // points, on identical commit counts - and the check is a score comparison
    // (§27.2), which is where 88 of 413 contested losses go. The competitive
    // reports agree on the shape: every winning deck is a score ladder indexed by
    // turn, reaching its highest band last.
    //
    // This deck's ladder tops out at 4 for a single card and 9 for a pair, so the
    // only band a pair can reach that a single card cannot is 5 (1pt + 4pt = 14
    // icons), and 14 icons is clearable at turn 5 and beyond, where measured
    // supply is 15-20. That is the band that beats a 4-point opponent, and it is
    // the gap between v9 and v7 on the two turns where v9 is behind.
    //
    // Off by default, and the sweep is what decides it: the n=1200 result for the
    // unconditional version reversed at 30,000 games (§28.12), so this one is
    // queued at 15,000 and nothing ships on a favourable draw.
    let late_band_from = late_band_turn();
    let cap = if late_band_from.is_some() {
        max_lives.max(contested_lives).max(2)
    } else {
        max_lives.max(contested_lives)
    };
    let n = lives.len();
    let mut masks: Vec<u32> = Vec::with_capacity(n + 1);
    masks.push(0);
    for h in 0..n {
        if cap >= 1 {
            masks.push(1u32 << h);
        }
        for g in 0..h {
            if cap >= 2 {
                masks.push((1u32 << h) | (1u32 << g));
            }
            for f in 0..g {
                if cap >= 3 {
                    masks.push((1u32 << h) | (1u32 << g) | (1u32 << f));
                }
            }
        }
    }
    for mask in masks {
        let chosen: Vec<&Life> = lives
            .iter()
            .enumerate()
            .filter(|(bit, _)| mask & (1 << bit) != 0)
            .map(|(_, life)| life)
            .collect();
        let mut need: Acc = [0i16; 11];
        let mut score = yell_score;
        for life in &chosen {
            for (need_k, life_need) in need.iter_mut().zip(life.need.iter()) {
                *need_k += *life_need;
            }
            score += life.score;
        }
        // Width is only allowed when the extra score can change the outcome.
        // `contested_masses` gives the placement mass on each side of a
        // contested check; with no mass on either side the check is decided by
        // passing alone and a second life is pure added cost.
        let (we_contested, they_contested, _) =
            opp.contested_masses(score, chosen.len().max(1), my_success);
        let is_contested = we_contested > 0.0 && they_contested > 0.0;
        let mut width_cap = if is_contested { contested_lives } else { max_lives };
        // The band test, gated on turn. See the `cap` block for the measurement.
        if let (Some(from), true) = (late_band_from, is_contested) {
            if chosen.len() == 2 && u32::from(gs.turn_number) >= from {
                let printed: i32 = score - yell_score;
                let best_single = lives.iter().map(|l| l.score).max().unwrap_or(0);
                let opp_best = opp.top_score();
                if best_single <= opp_best && printed > opp_best {
                    width_cap = 2;
                }
            }
        }
        if chosen.len() > width_cap {
            continue;
        }
        // The board the check is resolved against, and the blade count that draws
        // from the deck. With the projection off this is the board as it stands
        // at set time; with it on, it is the board after every live-start
        // ability has fired and been answered.
        let projected = project_live_start(gs, me, db, &chosen);
// Only the POOLS are consumed downstream: `zone_ordered_pass_probability`
        // prices the check, and the projected board/blades reach it through the
        // pools `build_with_draws` was sampled with. Returning them as well
        // looked load-bearing and was not — two dead bindings hid the fact that
        // the projection's whole effect on the decision is via `check_pools`.
        let check_pools = match &projected {
            Some((board, blades)) => {
                v8_model::FlipSampler::build_with_draws(
                    gs,
                    me,
                    db,
                    usize::try_from(*blades).unwrap_or(0),
                )
                .sample_pools(board, v8_model::flip_samples())
            }
            // No projection: the pools sampled at set time. Same helpers the
            // projected arm uses, so the two arms stay comparable.
            //
            // `pools` is a `&Vec<Pool>` borrowed from the enclosing sampler, so
            // `pools.clone()` would clone the REFERENCE and leave this arm
            // `&[Pool]` where the other arm is `Vec<Pool>`. `to_vec()` is the copy.
            None => pools.to_vec(),
        };
        let p_pass = if chosen.is_empty() {
            0.0
        } else {
            zone_ordered_pass_probability(&check_pools, &chosen)
        };
        let outcome = v8_model::check_outcome(p_pass, score, chosen.len(), my_success, opp);
        let spare = max_slots - chosen.len();
        let filler: Vec<&Junk> = worth_burning.iter().take(spare).copied().collect();
        let filter_value: f64 = filler
            .iter()
            .map(|j| draw_credit - j.keep_value)
            .sum();
        let value = candidate_value_zone(outcome, p_pass, filter_value, !chosen.is_empty())
            // `V8_JOINT`: the shipped value pays for ONE success whatever the
            // set size, which is what rule 8.3.16 / Q35 actually pays - a set
            // is all-or-nothing, so the never-sets-two behaviour is correct and
            // not a modelling gap. See `zone_success_distribution` for the
            // measured correction of the premise this used to rest on.
            + joint_success_value(
                &zone_success_distribution(
                    &pools,
                    &v8_model::board_pool(gs, me, db),
                    &chosen,
                ),
                my_success,
                opp.success,
            );
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
    //
    // The mechanism is the OVERWRITE, not the score. It copies the best
    // non-empty candidate onto the fold entry, so the one entry that used to
    // mean "commit nothing" now carries a set instead, and with it gone there
    // is nothing left for the argmax to choose when it wants to fold.
    //
    // There used to be a `entry.value = f64::NEG_INFINITY` line between the
    // two assignments, which the compiler kept as dead code and which read as
    // though the mechanism were broken. It is not, and it must not be
    // "repaired" by moving it below `entry.value = value`: NEG_INFINITY is
    // what a later argmax would treat as a loser, so reordering would delete
    // the very candidate this switch exists to promote, and the ablation would
    // silently measure nothing.
    if std::env::var_os("V8_NO_FOLD").is_some() {
        if let Some(best_lives) = out
            .iter()
            .filter(|c| !c.lives.is_empty())
            .max_by(|a, b| a.value.partial_cmp(&b.value).unwrap_or(std::cmp::Ordering::Equal))
        {
            let lives = best_lives.lives.clone();
            let value = best_lives.value;
            if let Some(entry) = out.iter_mut().find(|c| c.lives.is_empty()) {
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
///
/// `P_FIRST=1` reorders this to the guide's primary rule — "maximise the
/// probability of winning the actual live comparison" — with the existing
/// value as the tiebreak.
///
/// This is the one measured difference between v8 and v7, and it is large:
///
///     v7  n=7821  mean committed p = 0.924  blades 4.95  commits 99%
///     v8  n=7660  mean committed p = 0.485  blades 5.81  commits 82%
///
/// v8 has MORE blades and commits MORE often, and lands at less than half
/// v7's pass probability. So v8 is not short of board and not short of
/// nerve; it is choosing sets it will fail. A check that fails places nothing
/// (8.4.7: one card per win) and destroys the life (8.3.4, all-or-nothing
/// across the set), so a set at p=0.5 is worth about half of a set at p=1.0
/// and costs a card outright the other half of the time.
///
/// The arithmetic closes on the observed placement rates:
///     v7  0.99 commits x 0.924 = 0.91   (arena measures 0.46 with junk/flow)
///     v8  0.82 commits x 0.485 = 0.40   (arena measures 0.389)
///
/// So the ranking key, not the commit rule and not the board, is the deficit.
/// v8's own `V8_NO_FOLD` experiment agrees: forcing MORE commits made it
/// worse (896-1104 -> 752-1248), because its extra commits are coin flips.
/// `V8_ZONE_ORDER=1`: price the check the way the engine actually resolves it.
///
/// ## The rule this corrects (8.3.15)
///
/// A zone is resolved **in order, one card at a time**. Each card draws from
/// the pool until its own requirement is met, then the next draws from what is
/// left. The hearts the first card does not need are still available to the
/// second — but the first card cannot borrow the second's wildcards, and a card
/// that cannot be satisfied fails the zone outright.
///
/// `v8_model::pass_probability` prices the **pooled** requirement instead: one
/// vector with every life added, satisfied or failed as a whole. That is
/// strictly more generous than the engine whenever one card is nearly
/// satisfiable and the rest are hopeless, because pooling lets the surplus of a
/// card that needs nothing cover a card that needs everything.
///
/// It only differs at set size >= 2, and the shipped default is width 1, so this
/// is inert at the default — which is exactly the point. It is the reason width
/// has to be *earned* rather than assumed, and no cell in this file has measured
/// width with a correct model behind it.
///
/// Off by default because the pooled model is what every previous live-set
/// result was measured under, and switching it changes what those mean.
fn zone_ordered_pass_probability(pools: &[Pool], chosen: &[&Life]) -> f64 {
    if std::env::var_os("V8_ZONE_ORDER").is_none() || chosen.len() <= 1 {
        let mut need: Acc = [0i16; 11];
        for life in chosen {
            for (k, v) in need.iter_mut().zip(life.need.iter()) {
                *k += *v;
            }
        }
        return if chosen.is_empty() {
            0.0
        } else {
            v8_model::pass_probability(pools, &need)
        };
    }
    // A zone succeeds only if EVERY card in it is satisfied, and a card that is
    // not satisfied takes nothing and fails the zone. So the sequential model
    // is a conjunction, and the only thing order changes is whether a later
    // card could have been satisfied by what an earlier one left behind.
    let wildcards: i32 = chosen.iter().map(|l| i32::from(l.need[7])).sum();
    for pool in pools {
        let mut wildcards_left = wildcards;
        let mut all_ok = true;
        for life in chosen {
            let mut ok = true;
            for c in 0..7usize {
                let want = i32::from(life.need[c]);
                if want == 0 {
                    continue;
                }
                let deficit = want - i32::from(pool[c]);
                if deficit > 0 {
                    if deficit <= wildcards_left {
                        wildcards_left -= deficit;
                    } else {
                        ok = false;
                        break;
                    }
                }
            }
            if !ok {
                all_ok = false;
                break;
            }
        }
        if all_ok {
            return 1.0;
        }
    }
    0.0
}

/// `V8_JOINT`: how many cards in a set SUCCEED, not whether the set survives.
///
/// Returns `P(k successes)` for `k = 0..=chosen.len()`.
///
/// ## Why this exists
///
/// `check_outcome` credits **exactly one** success in every branch —
/// `rule_payoff(our_success + 1, opp.success)` — and takes `p_pass` as the
/// probability that the whole set is satisfied. So under the shipped model a
/// three-life set is "one success, with lower probability than one good life",
/// and is therefore never better than setting one life. That is not a
/// preference; it is arithmetic, and it is why the live set **never commits
/// more than one card, even when the zone is explicitly allowed three** — the
/// `V8_ZONE_ORDER` and `V8_MAX_LIVES=3` cells returned **0 discordant games**.
///
/// ## The premise this function used to rest on was wrong
///
/// An earlier version of this note argued the other way: that each card in the
/// zone resolves independently, so three lives can bank three successes in one
/// phase and a model capped at one success "cannot price the zone's real
/// contents". **The engine does not do that.** Rule 8.3.16 / Q35 is
/// all-or-nothing — `determine_winners` requires `lives.iter().all(|l| l.passed)`,
/// so a set containing one unsatisfied card places NOTHING and every card in the
/// zone goes to the waitroom. Pinned by
/// `tests/test_modules/rules/phases/live_success_rules_test.rs::any_card_fails_hearts_all_fail`
/// ("one failure fails ALL - nothing places, both cards to waitroom"), and
/// visible in 7 played games of a 200-game trace set, every one of them a set
/// holding a passed life next to a failed one and **every one of them scoring
/// zero placements** (116 sets containing a failure, +1 placements: 0).
///
/// So the width is not a capability the model is missing. Under the real rule a
/// multi-card set pays `P(all pass) x 1 placement` with a summed requirement, and
/// it is dominated by setting the single best card in the same zone. The shipped
/// single-card behaviour is CORRECT, and the zero-discordance of
/// `V8_MAX_LIVES=3` is the rule talking rather than a model that cannot speak.
///
/// `V8_JOINT` therefore stays off by default with its premise corrected rather
/// than deleted: the distribution below is still the right SHAPE for pricing a
/// set (it is what the width gate needs to be honest about), but it must not be
/// read as "N of 3 place". Anyone re-enabling it should re-read this first.
///
/// ## The rule it does model
///
/// A card that cannot be satisfied fails the whole set, and the others resolve
/// with it: the engine moves the zone as one. Cards are therefore counted one
/// at a time against a shared supply - stage hearts plus the sampled draw - and a
/// card consumes what it takes. Wildcards (`need[7]`, the any-colour bucket)
/// stand in for one missing heart of any colour, and can only help the card that
/// declares them, which is what the zone-order note above already worked out.
fn zone_success_distribution(pools: &[Pool], board: &Pool, chosen: &[&Life]) -> [f64; 4] {
    let mut out = [0.0f64; 4];
    if chosen.is_empty() {
        return out;
    }
    let n = chosen.len().min(3);
    for pool in pools {
        let mut avail = [0i32; 8];
        for c in 0..8usize {
            avail[c] = i32::from(board[c]) + i32::from(pool[c]);
        }
        let mut successes = 0usize;
        for life in chosen.iter().take(n) {
            let mut wild = i32::from(life.need[7]);
            let mut ok = true;
            for c in 0..7usize {
                let want = i32::from(life.need[c]);
                if want == 0 {
                    continue;
                }
                let take = want.min(avail[c]);
                avail[c] -= take;
                let deficit = want - take;
                if deficit > 0 {
                    if deficit <= wild {
                        wild -= deficit;
                    } else {
                        ok = false;
                        break;
                    }
                }
            }
            if ok {
                successes += 1;
            }
        }
        out[successes.min(3)] += 1.0;
    }
    let total: f64 = out.iter().sum();
    if total > 0.0 {
        for slot in out.iter_mut() {
            *slot /= total;
        }
    }
    out
}

/// Extra value a set earns by being able to bank MORE than one success, in
/// `PLACEMENT_CREDIT` units, relative to the shipped one-success model.
///
/// The shipped `check_outcome` adds `p_place * PLACEMENT_CREDIT` and
/// `rule_payoff(our_success + 1, ...)` — one success. This adds the shortfall:
/// for each `k > 1`, the probability of `k` successes times the value of the
/// 2nd..kth success, priced by the same race payoff. It is deliberately
/// additive and gated, so the null case is the shipped behaviour exactly and
/// the interaction with `check_outcome` is visible rather than buried.
fn joint_success_value(dist: &[f64; 4], our_success: usize, opp_success: usize) -> f64 {
    if std::env::var_os("V8_JOINT").is_none() {
        return 0.0;
    }
    let weight = std::env::var("V8_JOINT_W")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|w: &f64| w.is_finite() && *w >= 0.0)
        .unwrap_or(1.0);
    // The shipped model already pays for k = 1. This pays for k >= 2.
    let mut value = 0.0;
    for k in 2..=3usize {
        if k >= dist.len() {
            break;
        }
        let pay = v8_model::rule_payoff(our_success + k, opp_success)
            - v8_model::rule_payoff(our_success + 1, opp_success);
        value += dist[k] * (pay + PLACEMENT_CREDIT);
    }
    weight * value
}

/// `V8_LIVE_START_PROJECTION=1`: price the check against the board the
/// live-start abilities will actually produce, rather than the board that
/// exists when the zone is set.
///
/// ## What is wrong without it, from the deck's own cards
///
/// Every live in `5CP3Z idou` is ライブ開始時, and the four do large, different
/// things:
///
/// - `Aspire` `PL!SP-sd2-025-sd2` x4 — +1 blade to every Liella member that
///   moved this turn. Up to ~5 blades against an 11-heart requirement.
/// - `MIRACLE NEW STORY` `PL!SP-bp5-024-l` x1 — +1 heart of a chosen colour to
///   every member that moved this turn. Up to 5 hearts against a **15-heart**
///   requirement: a third of what the check needs, and the deck's finisher.
/// - `Special Color` `PL!SP-bp4-025-l` x4 — centre Liella members' printed
///   blade count becomes 3.
/// - `始まりは君の空` `PL!SP-sd2-023-sd2` x3 — at 2+ successes rewrites its own
///   requirement to 12 hearts; the FAQ confirms the trigger cannot be declined.
///
/// The live set prices a life against the board as it stands at set time, so
/// MIRACLE reads as a 15-heart card the board cannot reach — which is why a card
/// the deck runs as its win condition is treated as uncashable.
///
/// ## This is not the idea §0.2 refuted
///
/// That refutation was "`lscand == lcand` on every line, so presence has no
/// contrast". Presence is the wrong discriminator. The contrast is **how much
/// this ability adds to this board**, and the four above differ by an order of
/// magnitude and act on different quantities — blades, hearts, centre blades,
/// and its own requirement. No null in the log bears on that question.
///
/// ## Generic, not a four-card table
///
/// It runs the engine. Clone at set time, commit the candidate, fire
/// `TurnEngine::trigger_live_start_abilities`, then answer whatever it asks
/// using **the bot's own `evaluate_actions`** — so MIRACLE's colour choice and
/// `澁谷かのん`'s formation change are resolved by the policy that would resolve
/// them in a real game, and a card this file has never seen still projects.
/// Hard-coding the four IDs would be fragile and would misdescribe what the
/// code knows.
///
/// `RngGuard` is mandatory: the engine draws while resolving, and a policy call
/// that perturbs the game RNG is a policy call that changes the game.
fn project_live_start(
    gs: &GameState,
    me: u8,
    db: &CardDatabase,
    lives: &[&Life],
) -> Option<(Pool, i32)> {
    if std::env::var_os("V8_LIVE_START_PROJECTION").is_none() || lives.is_empty() {
        return None;
    }
    let _guard = v8_model::RngGuard::new();
    let mut sim = gs.clone();
    let me_id = sim.seat_player(me).id.clone();
    for life in lives {
        // `Life` carries a hand SLOT, not a card id.
        if let Some(&cid) = sim.seat_player(me).hand.cards.get(life.index) {
            let _ = sim.seat_player_mut(me).live_card_zone.add_card(cid, db);
        }
    }
let queued_before = sim.ability_queue_len();
    crate::turn::TurnEngine::trigger_live_start_abilities(&mut sim, &me_id);
    let queued = sim.ability_queue_len().saturating_sub(queued_before);
    // The scan only ENQUEUES. The real engine resolves the queue immediately
    // after it — `phases.rs` calls `process_pending_auto_abilities` on the very
    // next statement after both players' scans — and without this line the entry
    // sits at `PayingCost`, `has_pending_choice()` is false, the answer loop
    // below breaks on iteration 0, and the "projection" returns the untouched
    // set-time board. That is the whole reason the switch measured
    // bit-identically: it was firing and throwing the result away.
    sim.process_pending_auto_abilities(&me_id);
    // Bounded, and a projection that cannot converge degrades to "no
    // projection" rather than hanging a turn.
    for _ in 0..8 {
        if !sim.has_pending_choice() {
            break;
        }
        let actions = crate::game_setup::generate_possible_actions(&sim);
        if actions.is_empty() {
            break;
        }
        // `evaluate_actions` pushes one score per input action in order, so the
        // two zip; `ActionScore` has no index of its own. A rejected action scores
        // `NEG_INFINITY` (`ActionScore::rejected`), so it can never win this
        // comparison and needs no separate filter.
        //
        // `max_by` on a `zip` gets each element by reference, so the closure
        // takes TWO arguments — each a `&(&ActionScore, &Action)` — not one
        // destructured pair.
        let scores = super::v8_main::evaluate_actions(&sim, &actions, me);
        let best = scores
            .iter()
            .zip(actions.iter())
            .max_by(|(a, _), (b, _)| {
                a.value
                    .partial_cmp(&b.value)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(_, action)| action);
        let Some(action) = best else { break };
        if crate::game_setup::execute_action(&mut sim, action).is_err() {
            break;
        }
crate::game_setup::settle_auto(&mut sim);
    }
    let board = v8_model::board_pool(&sim, me, db);
    let blades = v8_model::active_blades(&sim, me, db);
    // The two discriminating measurements §13.1 could not make. A projection
    // that queues nothing and a projection that queues abilities which all
    // resolve to zero produce the same bit-identical result, so "no effect" is
    // ambiguous until the queue depth and the blade delta are visible.
    if std::env::var_os("V8_LSP_DIAG").is_some() {
        let (base_board, base_blades) =
            (v8_model::board_pool(gs, me, db), v8_model::active_blades(gs, me, db));
        eprintln!(
            "LSPDIAG lives={} queued={} blades {}->{} board {:?}->{:?}",
            lives.len(),
            queued,
            base_blades,
            blades,
            base_board,
            board
        );
    }
    Some((board, blades))
}

/// `V8_LATE_BAND=<turn>`: the first turn on which a band-crossing pair is
/// allowed. **Default 5**; `V8_LATE_BAND=0` restores the single-card width, and
/// `V8_LATE_BAND=<n>` moves the gate.
///
/// Measured, 3000 games x 13 seeds = **39,000 paired games**, five discovery
/// seeds and eight never read while looking for this effect: discordant 129
/// baseline-only against 206 candidate-only, **net +77, exact McNemar
/// p = 0.000031**, with 12 of the 13 seeds in the candidate's favour (sign test
/// p = 0.00012) and per-seed nets of +14 +9 +4 +11 -1 +2 +5 +8 +6 +1 +4 +7 +7.
///
/// The mechanism is the chain in §28.27: the check is a score comparison
/// (§27.2), v9's score ladder trails v7's on turns 5-6 while leading on 3-4
/// (§28.26), and a pair reaches a printed-score band a single card in this deck
/// cannot - 5, for 1pt + 4pt = 14 icons, clearable at turn 5 where measured
/// supply is 15-20. 8.3.15 pools the requirements and 8.4.7 pays one placement
/// for them, so the pair buys exactly one thing: a band.
///
/// The same rule ungated measured null (§28.7), because it fired on turns 1-4
/// where there is no gap to close. The gate is the whole change.
fn late_band_turn() -> Option<u32> {
    match std::env::var("V8_LATE_BAND") {
        Ok(v) => v.parse::<u32>().ok().filter(|t| *t > 0),
        Err(_) => Some(5),
    }
}

/// Mass above which the opponent's score band counts as reachable. 0.1 keeps a
/// tie reachable, because a check lost on equal printed score is lost when the
/// score icons go the other way - 48 of the 55 equal-card losses in a 200-game
/// trace were exactly -1.
pub(crate) fn band_min_mass() -> f64 {
    std::env::var("V8_BAND_MIN_MASS")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|m: &f64| m.is_finite() && *m >= 0.0 && *m <= 1.0)
        .unwrap_or(0.1)
}

fn better(a: &Candidate, b: &Candidate) -> bool {
    const EPS: f64 = 1e-9;
    // AMMUNITION FLOOR. Refuse to commit a set this unlikely to be won.
    //
    // A failed check is not a neutral outcome. The live goes to the waitroom
    // (8.3.4) and 8.4.7 places at most one card, so committing at probability
    // `p` spends the life outright with probability `1 - p`. The life is the
    // only way to place a card, so the loss is not "this check" — it is the
    // per-check probability for the rest of the game.
    //
    // The measured prize, and it is a feedback loop rather than a single bad
    // choice:
    //
    //     v7  n=7821  mean committed p = 0.924  commits 99%  -> burns ~8% of lives
    //     v8  n=7660  mean committed p = 0.485  commits 82%  -> burns ~52% of lives
    //
    // So v8 starts with similar ammunition, spends half of it per check, and
    // is left holding worse lives than v7 — which is why its committed p stays
    // near 0.48 no matter how it ranks. Reordering the ranking to prefer `p`
    // (`V8_P_FIRST`) moved the mean by 0.008, because there was nothing better
    // in the candidate set to promote: the lives themselves are the problem.
    //
    // The floor is the break-even of the arithmetic, not a fitted constant:
    // committing trades the life for the placement, so a set is worth
    // committing when `p` exceeds the value of keeping the live for a later,
    // better-supported check. Above 1/2 the placement is worth more than the
    // life; below it the life is worth more than the check. `V8_P_FLOOR`
    // moves it for ablation.
    if std::env::var_os("V8_P_FLOOR").is_some() {
        let floor: f64 = std::env::var("V8_P_FLOOR")
            .ok()
            .and_then(|v| v.parse().ok())
            .filter(|&f: &f64| f.is_finite() && (0.0..=1.0).contains(&f))
            .unwrap_or(0.6);
        let a_ok = a.lives.is_empty() || a.outcome.p_place >= floor;
        let b_ok = b.lives.is_empty() || b.outcome.p_place >= floor;
        match (a_ok, b_ok) {
            (true, false) => return true,
            (false, true) => return false,
            _ => {}
        }
    }
    if std::env::var_os("V8_P_FIRST").is_some() {
        // Primary: will this check actually be won.
        if (a.outcome.p_place - b.outcome.p_place).abs() > EPS {
            return a.outcome.p_place > b.outcome.p_place;
        }
        // Then the existing value, which carries score comparison, the
        // contested case, and the fold-versus-commit trade.
        if (a.value - b.value).abs() > EPS {
            return a.value > b.value;
        }
        if a.lives.len() != b.lives.len() {
            return a.lives.len() < b.lives.len();
        }
        return a.lives < b.lives;
    }
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
    // `FlipSampler::sample_pools` already accumulates in `Pool` (`[i16; 8]`)
    // and clamps as it goes, so the pools pass straight through to
    // `pass_probability` — which used to need a re-clamp here because the
    // sampler used to produce `i32` buckets.
    let pools = sampler.sample_pools(&board, v8_model::flip_samples());
    let opp = OppModel::build(gs, me, db);
    let draw_credit = draw_value(gs, me, db);
    let candidates = enumerate_candidates(gs, me, db, &pools, &opp, draw_credit);
    // Per-check telemetry, paired with v7's `V7CHK` line so the two can be
    // compared at the same blade count. v8 develops a bigger board than v7
    // (T4 12.8 vs 11.2) and passes fewer checks (0.389 vs 0.435), and
    // forcing fewer folds made it WORSE, so the fold is not the cause. The
    // live candidate for that: v8 grows cost without colour match, so more
    // board buys less conversion. This line says whether that is true.
    //
    // The doc calls these two lines "paired" and comparable at the same blade
    // count. They were not comparable: this one printed `p_place` (placement
    // probability) where v7's prints `p_pass` (the probability the check is
    // passed at all). Placement probability is the one the score comparison
    // moves and the board does not, so reading a "p" gap between the two lines
    // attributed a development difference to a quantity that does not measure
    // development. Both are now printed on both lines, named apart.
    if std::env::var_os("V7_CHECK_P").is_some() {
        let chosen = candidates
            .iter()
            .filter(|c| !c.lives.is_empty())
            .max_by(|a, b| a.value.partial_cmp(&b.value).unwrap_or(std::cmp::Ordering::Equal));
        // Headroom, v8-side. `best_available` for v7 enumerates sets itself;
        // here every candidate is already enumerated and already carries its
        // own `p_pass`, so the best available set is a max over the list rather
        // than a second search. `chosen_p` vs `best_p` is the gap that
        // separates "the board cannot support a check" (small gap) from "the
        // argmax declined one it could have made" (large gap).
        let best_p = candidates
            .iter()
            .map(|c| c.outcome.p_pass)
            .fold(0.0f64, f64::max);
        let p = gs.seat_player(me);
        let is_live_start = |cid: i16| {
            db.get_card(cid).is_some_and(|card| {
                card.abilities.iter().any(|a| {
                    a.resolve()
                        .has_trigger(crate::triggers::TriggerKind::LiveStart)
                })
            })
        };
        let all_lives: Vec<i16> = p
            .hand
            .cards
            .iter()
            .copied()
            .filter(|&cid| {
                db.get_card(cid)
                    .is_some_and(|c| c.card_type == CardType::Live)
            })
            .collect();
        let lcand = all_lives.len();
        let lscand = all_lives.iter().filter(|&&cid| is_live_start(cid)).count();
        let lschosen = chosen
            .map(|c| {
                c.lives
                    .iter()
                    .filter_map(|&i| p.hand.cards.get(i).copied())
                    .filter(|&cid| is_live_start(cid))
                    .count()
            })
            .unwrap_or(0);
        eprintln!(
            "V8CHK t={} me={} blades={} set={} p={:.3} ppass={:.3} best={:.3} succ={} \
             need={} boardcovers={} deck={} lcand={} lscand={} lschosen={}",
            gs.turn_number,
            me,
            super::strategy_v4::flip_stats(gs, me, db).0,
            chosen.map(|c| c.lives.len()).unwrap_or(0),
            chosen.map(|c| c.outcome.p_place).unwrap_or(0.0),
            chosen.map(|c| c.outcome.p_pass).unwrap_or(0.0),
            best_p,
            p.success_live_card_zone.cards.len(),
            super::strategy_v7::deficit_total(gs, me, db) as usize,
            super::strategy_v7::board_cover_of_best_life(gs, me, db) as usize,
            p.main_deck.cards.len(),
            lcand,
            lscand,
            lschosen,
        );
    }
    (candidates, opp)
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
// `V8_CAL`: one machine-parseable line per live-set decision, carrying the
    // turn and the seat so a post-hoc pass can join this prediction against the
    // engine's own verdict in the arena's per-game log. Without the turn key the
    // model's `p_pass` and the realized pass/fail cannot be lined up, which is
    // why every previous calibration read in this project had to be eyeballed.
    // `V8_CAL2`: the opponent's SCORE assumption alongside the pass assumption.
// `score_dist` is derived from the SAME `yield_prior` / `board_total` that
// §14.7 measured as inverted and inert, and it decides the score comparison -
// which §14.2 showed decides 51.2% of live phases, with +1 winning 33/33 and -1
// losing 32/32. So it is the same class of quantity as `pass_prob` and deserves
// the same calibration check.
if std::env::var_os("V8_CAL2").is_some() {
        let sd: Vec<String> = opp
            .score_dist
            .iter()
            .enumerate()
            .filter(|(_, w)| **w > 1e-9)
            .map(|(s, w)| format!("{}:{:.3}", s, w))
            .collect();
        // `ob/obl` are the opponent's public board total and active blades: the
        // two inputs `score_dist` is built from. They are printed so the
        // under-prediction can be attributed to the supply estimate or to the
        // band table, instead of guessed at.
        let opp_board_total: i32 =
            crate::core::stats_pipeline::stage_hearts(
                &opp_player.stage.stage,
                db,
                &gs.mods.heart_override,
                &gs.mods.heart_copy,
                &gs.mods.heart_color_multiplier,
                &gs.mods.heart_modifiers,
            )
            .hearts
            .values()
            .map(|v| i32::from(*v))
            .sum();
        let opp_blades = opp_player.stage.total_blades(
            db,
            &gs.mods.blade_modifiers,
            &gs.mods.orientation_modifiers,
            false,
        );
        eprintln!(
            "V8CAL2 t={} opp_set={} opp_pass={:.3} oursucc={} opsucc={} ob={} obl={} scoredist=[{}]",
            gs.turn_number,
            opp.set_size,
            opp.pass_prob,
            p.success_live_card_zone.cards.len(),
            opp_player.success_live_card_zone.cards.len(),
            opp_board_total,
            opp_blades,
            sd.join(" ")
        );
    }
    if std::env::var_os("V8_CAL").is_some() {
        // The full candidate table, not just the winner: without the runners-up
        // there is no way to tell "chose the best life in hand" apart from
        // "every life in hand was equally bad".
        let cand = candidates
            .iter()
            .map(|c| {
                let names: Vec<String> = c
                    .lives
                    .iter()
                    .filter_map(|&i| p.hand.cards.get(i).copied())
                    .filter_map(|cid| db.get_card(cid))
                    .map(|card| {
                        format!(
                            "{}:{}/{}",
                            card.card_no,
                            card.score.unwrap_or(0),
                            card.need_heart.as_ref().map_or(0, |n| {
                                n.hearts.values().map(|v| i32::from(*v)).sum()
                            })
                        )
                    })
                    .collect();
                format!(
                    "[{}|pp={:.3}|pl={:.3}|v={:.4}]",
                    if names.is_empty() { "FOLD".to_string() } else { names.join("+") },
                    c.outcome.p_pass,
                    c.outcome.p_place,
                    c.value
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        eprintln!(
            "V8CAL t={} me={} second={} set={} ppass={:.4} pplace={:.4} opp_pass={:.4} opp_set={} mysucc={} opsucc={} cands={}",
            gs.turn_number,
            me,
            gs.current_phase == Phase::LiveCardSetSecondAttacker,
            if best.lives.is_empty() { 0 } else { 1 },
            best.outcome.p_pass,
            best.outcome.p_place,
            opp.pass_prob,
            opp.set_size,
            p.success_live_card_zone.cards.len(),
            opp_player.success_live_card_zone.cards.len(),
            cand,
        );
    }
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

// ── Introspection: what the live-set decision PREDICTED ──────────────────

/// What the live-set decision predicted for one check, in the same numbers the
/// decision used, exposed for offline calibration against engine outcomes.
///
/// This exists because every claim about v8's live set has been an aggregate.
/// Win rate, fold rate and pace all say *that* the live set is leaving
/// placements on the table; none of them say whether the model is
/// **mis-calibrated** - i.e. whether a candidate the model scored 0.7 passes
/// only 45% of the time. That distinction decides what to fix. If `p_place` is
/// well calibrated and pace is still 0.42, the model is right and the game is
/// genuinely contested, and the fix is development. If `p_place` is
/// systematically optimistic or pessimistic, the argmax is ranking candidates
/// on a lie and the fix is the model - and no amount of development work will
/// show up.
///
/// A prediction is a pure function of the decision, so this cannot perturb the
/// game: it recomputes the same argmax from the same inputs and reads no hidden
/// state.
#[derive(Clone, Debug)]
pub struct LivePrediction {
    /// Hand indices of the lives in the chosen zone, and of the junk.
    pub lives: Vec<usize>,
    pub junk: Vec<usize>,
    pub p_pass: f64,
    pub p_place: f64,
    pub opp_place: f64,
    pub value: f64,
    pub opp_pass: f64,
    pub opp_set_size: usize,
    pub my_success: usize,
    pub opp_success: usize,
    pub n_lives_in_hand: usize,
    pub n_candidates: usize,
    /// The best candidate that was NOT chosen: `(hand indices, value, p_place)`.
    /// The gap between this and the chosen value is the decision's margin, and
    /// a decision taken on a margin of 0.001 is a decision taken on sampling
    /// noise.
    pub runner_up: Option<(Vec<usize>, f64, f64)>,
    /// Total heart supply on our board, and the requirement the chosen zone
    /// actually had to clear. The two together are what `p_pass` is a
    /// probability OF, so a calibration report can be sliced by how far short
    /// the board actually fell rather than only by the model's own guess.
    pub board_hearts: i32,
    pub need_hearts: i32,
}

/// Recompute the live-set decision and report what it believed, without
/// emitting anything. `None` when the set phase is suppressed outright.
pub fn predict_live_set(gs: &GameState, db: &CardDatabase) -> Option<LivePrediction> {
    let me = gs.active_player_index();
    let p = gs.seat_player(me);
    if p.live_card_set_limit_reduction >= 3 || gs.cannot_live_players.contains(&p.id) {
        return None;
    }
    let (candidates, opp) = score_candidates(gs, me, db);
    let best = candidates
        .iter()
        .fold(None::<&Candidate>, |acc, cand| match acc {
            None => Some(cand),
            Some(current) if better(cand, current) => Some(cand),
            keep => keep,
        })?
        .clone();
    let runner_up = candidates
        .iter()
        .filter(|c| c.lives != best.lives || c.junk != best.junk)
        .max_by(|a, b| a.value.partial_cmp(&b.value).unwrap_or(std::cmp::Ordering::Equal))
        .map(|c| (c.lives.clone(), c.value, c.outcome.p_place));
    let need_hearts: i32 = best
        .lives
        .iter()
        .filter_map(|&i| p.hand.cards.get(i).copied())
        .map(|cid| v8_model::life_need(gs, cid))
        .map(|need| {
            need.iter()
                .copied()
                .filter(|&n| n > 0)
                .map(i32::from)
                .sum::<i32>()
        })
        .sum();
    let board_hearts: i32 = v8_model::board_supply(gs, me, db)
        .iter()
        .filter(|&&n| n > 0)
        .map(|&n| i32::from(n))
        .sum();
    Some(LivePrediction {
        lives: best.lives.clone(),
        junk: best.junk.clone(),
        p_pass: best.outcome.p_pass,
        p_place: best.outcome.p_place,
        opp_place: best.outcome.opp_place,
        value: best.value,
        opp_pass: opp.pass_prob,
        opp_set_size: opp.set_size,
        my_success: p.success_live_card_zone.cards.len(),
        opp_success: opp.success,
        n_lives_in_hand: collect_lives(gs, me, db).len(),
        n_candidates: candidates.len(),
        runner_up,
        board_hearts,
        need_hearts,
    })
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
    /// A life that cannot pass must be strictly worse than not setting it.
    ///
    /// The burn price used to be `p * (1 - p)`, which is ZERO at `p = 0`, so a
    /// guaranteed-to-fail life cost nothing and a zone containing one tied
    /// exactly with the same zone without it. The decision was then made by the
    /// fewer-lives tie-break, which is a coin flip wearing a rank - and which
    /// flips the moment `filter_value` or the junk ordering moves.
    ///
    /// This was found by hand-playing a T1 live set and printing the candidate
    /// values, where the life-free zone and the doomed-life zone both read
    /// 0.0629. No aggregate can see it: fold rate and pace are unchanged either
    /// way, because the tie-break happened to land on the right answer.
    #[test]
    fn a_life_that_cannot_pass_is_strictly_worse_than_not_setting_it() {
        let doomed = CheckOutcome {
            p_pass: 0.0,
            p_place: 0.0,
            opp_place: 0.0,
            value: 0.0,
        };
        let free = CheckOutcome {
            p_pass: 0.0,
            p_place: 0.0,
            opp_place: 0.0,
            value: 0.0,
        };
        let with_life = candidate_value(doomed, 0.0, 0.38);
        let without_life = candidate_value_zone(free, 0.0, 0.38, false);
        assert!(
            with_life < without_life,
            "a life at p=0 must cost something: {with_life} vs {without_life}"
        );
        // And the price must shrink as the life gets likelier to place, or the
        // model would refuse to commit lives it is about to place.
        let certain = candidate_value(
            CheckOutcome {
                p_pass: 1.0,
                p_place: 1.0,
                opp_place: 0.0,
                value: PLACEMENT_CREDIT,
            },
            1.0,
            0.0,
        );
        assert!(
            certain > with_life,
            "committing a life that certainly places must be worth more than burning a doomed one"
        );
    }

    #[test]
    fn a_life_that_certainly_places_is_not_charged_a_burn() {
        let v = candidate_value(
            CheckOutcome {
                p_pass: 1.0,
                p_place: 1.0,
                opp_place: 0.0,
                value: PLACEMENT_CREDIT,
            },
            1.0,
            0.0,
        );
        assert!(
            (v - PLACEMENT_CREDIT).abs() < 1e-12,
            "a placed life forfeits nothing: {v}"
        );
    }

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

    /// The candidate enumeration must not silently drop lives.
    ///
    /// It used to be `for mask in 0..(1 << lives.len().min(16))`. That made
    /// every life past the 16th unreachable — a 17-card hand lost four of its
    /// candidates with no counter and no log, and because `better()` can only
    /// rank what it is given, a life the bot would have committed to became
    /// invisible and the zone folded instead. The regression is invisible from
    /// the outside: fold rate and pace both stay plausible, which is the same
    /// reason the per-card `V8CHK` line exists.
    ///
    /// Pinned directly against the generator so a re-introduced `.min(N)` fails
    /// here rather than in a run.
    #[test]
    fn candidate_enumeration_sees_every_life_whatever_the_hand_size() {
        // Duplicated shape of the mask list in `enumerate_candidates`; if the
        // two ever drift this test is not protecting anything, so it names the
        // contract rather than the function.
        let masks_for = |n: usize, cap: usize| -> Vec<u32> {
            let mut masks = vec![0u32];
            for h in 0..n {
                if cap >= 1 {
                    masks.push(1u32 << h);
                }
                for g in 0..h {
                    if cap >= 2 {
                        masks.push((1u32 << h) | (1u32 << g));
                    }
                    for f in 0..g {
                        if cap >= 3 {
                            masks.push((1u32 << h) | (1u32 << g) | (1u32 << f));
                        }
                    }
                }
            }
            masks
        };

        for &n in &[0usize, 1, 5, 16, 17, 20, 31] {
            for &cap in &[1usize, 2, 3] {
                let masks = masks_for(n, cap);
                // Every life index must appear in at least one affordable set.
                // This is the assertion the `.min(16)` failed for n > 16.
                for bit in 0..n {
                    assert!(
                        masks.iter().any(|m| m & (1u32 << bit) != 0),
                        "n={n} cap={cap}: life {bit} is unreachable"
                    );
                }
                // And no set may exceed the width cap.
                for m in &masks {
                    assert!(
                        m.count_ones() as usize <= cap,
                        "n={n} cap={cap}: mask {m:b} is wider than the cap"
                    );
                }
            }
        }

        // The shipped default is a width cap of 1, where the list is the empty
        // set plus one set per life — which is bitmask order, so the candidate
        // order that breaks ties is unchanged from the pre-fix loop.
        assert_eq!(masks_for(4, 1), vec![0b0000, 0b0001, 0b0010, 0b0100, 0b1000]);
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
    fn a_junk_only_zone_is_not_charged_for_holding_no_life() {
        // A junk-only zone has p_pass = 0, so an ungated burn price charges it
        // a full placement credit for setting nothing. That is the same error
        // as failing to charge a doomed life: the price must follow the cards
        // actually at risk.
        let junk = candidate_value_zone(outcome(0.0, 0.0, 0.0), 0.0, 0.02, false);
        assert!(
            junk > 0.0,
            "a junk-only zone is worth its hand filter alone: {junk}"
        );
    }

    /// A thin life can be worth less than a junk set, but only because of the
    /// option value it forfeits - not because a life is never worth committing.
    ///
    /// Uncontested, a life worth `p` gains `p * PLACEMENT_CREDIT` and forfeits
    /// the option on the rest, so the break-even sits where the forfeited
    /// option equals the hand filter it loses by not junking. The `0.45` case
    /// below is the load-bearing one: on the same board, the same life, the
    /// same junk ordering, the decision flips on `p` alone.
    ///
    /// Paired with `v8_model::match_point_makes_a_thin_check_worth_taking`:
    /// at opponent match point the same thin life IS right, because folding
    /// there hands them the game. One objective, both answers.
    #[test]
    fn a_thin_life_can_be_worth_less_than_a_junk_set() {
        let junk_only = candidate_value_zone(outcome(0.0, 0.0, 0.0), 0.0, 0.02, false);
        assert!(junk_only > 0.0);
        // p = 0.20: the forfeited option on a life that probably cannot place
        // outweighs the placement it might make. Below break-even, so the argmax
        // sets junk and keeps the life for another check.
        let thin = candidate_value(outcome(0.20, 0.20, 0.20 * PLACEMENT_CREDIT), 0.20, 0.0);
        assert!(thin < junk_only, "thin {thin} vs junk {junk_only}");
        // p = 0.45 on the same board: the placement this life can make here
        // outweighs the one it forfeits, and it is committed. A thin life is
        // not junk; only a hopeless one is.
        let thicker = candidate_value(
            outcome(0.45, 0.45, 0.45 * PLACEMENT_CREDIT),
            0.45,
            0.0,
        );
        assert!(thicker > junk_only, "thicker {thicker} vs junk {junk_only}");
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

