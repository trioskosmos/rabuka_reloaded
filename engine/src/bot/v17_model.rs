//! V17 shared model: the draw-probability engine.
//!
//! v17 is the probability-first bot. Where v7 prices a Main-phase
//! action by a one-ply feature delta and v8 by a two-ply search
//! over a deterministic forward walk, v17 prices EVERY decision —
//! mulligan, live set, Main phase, ability usage — in one currency:
//! **expected placements over the next few checks**, with the number
//! of good cards in hand modelled as a HYPERGEOMETRIC random
//! variable drawn from our own decklist.
//!
//! Our own decklist is fair information (docs/BOT_STRATEGY.md
//! section 9), so the composition of the deck — how many lives,
//! how many members, how many blade-hearts — is known exactly.
//! What is NOT known is the ORDER, so every future draw is a
//! sample without replacement: P(draw >= k lives in n draws) is a
//! hypergeometric tail, not a guess.
//!
//! The currency:
//!
//! ```text
//! E[placements] = sum_t discount^t * P(ammo at t) * P(pass at t)
//! ```
//!
//! * `P(ammo at t)` — hypergeometric: we hold `L` lives now, each
//!   check spends one, each turn draws one, so the lives in hand at
//!   check `t` are `L - t + Hypergeometric(deck, lives, t)`.
//! * `P(pass at t)` — the check model: board supply projected along
//!   the guides' development ladder, yell flips a Binomial over the
//!   own-deck blade-heart density, requirement from the best
//!   settable life.
//!
//! Everything the three entry points do is an argmax over this
//! quantity. A draw ability raises `P(ammo at t)` for every future
//! `t`; a buff ability raises `P(pass at t)`; a deploy raises both.
//! That is the whole design: an action is worth taking exactly when
//! it raises the percentage of getting a good card to play.
//!
//! Fairness: reads own hand / deck / stage / energy / waitroom plus
//! the opponent's PUBLIC zones (stage, success count, live-zone
//! size). Never the opponent's hand, deck, energy deck or set
//! contents.

use crate::bot::strategy_common::Acc;
use crate::bot::v8_model::{self};
use crate::card::{CardDatabase, CardType};
use crate::game_state::{GameResult, GameState};

/// Terminal win/loss magnitude. Large enough that a won game
/// dominates any placement forecast, small enough that f64
/// arithmetic stays exact when added to one.
pub const WIN: f64 = 1000.0;
pub const LOSS: f64 = -1000.0;

/// How many checks ahead the placement forecast runs.
pub fn horizon() -> usize {
    std::env::var("V17_HORIZON")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&n: &usize| (1..=8).contains(&n))
        .unwrap_or(4)
}

/// Per-check discount factor.
pub fn discount() -> f64 {
    std::env::var("V17_DISCOUNT")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|w: &f64| w.is_finite() && *w > 0.0 && *w <= 1.0)
        .unwrap_or(0.85)
}

/// Own-deck composition. The backbone of every probability v17
/// computes: the deck is fair information, so its exact multiset
/// of card types is the population every future draw samples from.
pub struct DeckProfile {
    pub len: usize,
    pub lives: usize,
    pub members: usize,
    pub energy: usize,
    /// Cards carrying a blade heart — the yell-flip success rate.
    pub blade_heart: usize,
}

impl DeckProfile {
    pub fn build(gs: &GameState, me: u8, db: &CardDatabase) -> Self {
        let p = gs.seat_player(me);
        let (mut lives, mut members, mut energy, mut blade_heart) = (0, 0, 0, 0);
        for &cid in &p.main_deck.cards {
            let Some(card) = db.get_card(cid) else {
                continue;
            };
            match card.card_type {
                CardType::Live => lives += 1,
                CardType::Member => members += 1,
                CardType::Energy => energy += 1,
            }
            if card.blade_heart.is_some() {
                blade_heart += 1;
            }
        }
        Self {
            len: p.main_deck.cards.len(),
            lives,
            members,
            energy,
            blade_heart,
        }
    }
}

/// `P(X >= k)` for `X ~ Hypergeometric(total, successes, n)`: the
/// probability of drawing at least `k` successes in `n` draws
/// without replacement from a deck of `total` cards holding
/// `successes` of them.
///
/// Computed exactly by summing the probability mass over
/// `i in k..=min(n, successes)`. Decks are at most 60 cards and
/// draws at most a handful, so the binomial coefficients stay far
/// inside f64 range and the closed form is both exact and cheap.
pub fn hypergeometric_ge(n: usize, k: usize, successes: usize, total: usize) -> f64 {
    if k == 0 {
        return 1.0;
    }
    if total == 0 || n == 0 {
        return 0.0;
    }
    let n = n.min(total);
    let successes = successes.min(total);
    if k > successes || k > n {
        return 0.0;
    }
    let fails = total - successes;
    let denom = binom(total, n);
    if denom <= 0.0 {
        return 0.0;
    }
    let mut sum = 0.0f64;
    let max_i = n.min(successes);
    for i in k..=max_i {
        if n - i > fails {
            continue;
        }
        sum += binom(successes, i) * binom(fails, n - i) / denom;
    }
    sum
}

/// `C(n, k)` as f64. `n` is a deck size (<= 60), so the largest
/// value, `C(60, 30) ~= 1.2e17`, is far inside f64 range.
fn binom(n: usize, k: usize) -> f64 {
    if k > n {
        return 0.0;
    }
    let k = k.min(n - k);
    let mut r = 1.0f64;
    for i in 0..k {
        r *= (n - i) as f64 / (i + 1) as f64;
    }
    r
}

/// The v17 currency: expected placements over the next
/// [`horizon`] checks, from `me`'s seat.
///
/// Every term is a probability computed from fair information:
///
/// * `P(ammo at t)` — hypergeometric draw of lives from our own
///   deck, spending one per check;
/// * `P(pass at t)` — the check model at the board projected
///   `t` Main phases ahead along the guides' development ladder,
///   with yell flips a Binomial over the own-deck blade-heart
///   density and the requirement read off the best settable life.
///
/// A terminal position is priced at [`WIN`] / [`LOSS`] so an
/// action that ends the game dominates any forecast.
pub fn expected_placements(gs: &GameState, me: u8, db: &CardDatabase) -> f64 {
    let _guard = v8_model::RngGuard::new();
    match gs.game_result {
        GameResult::Ongoing => {}
        GameResult::Draw => return 0.0,
        GameResult::FirstAttackerWins => {
            let i_am_first = if me == 0 {
                gs.player1.is_first_attacker
            } else {
                gs.player2.is_first_attacker
            };
            return if i_am_first { WIN } else { LOSS };
        }
        GameResult::SecondAttackerWins => {
            let i_am_first = if me == 0 {
                gs.player1.is_first_attacker
            } else {
                gs.player2.is_first_attacker
            };
            return if i_am_first { LOSS } else { WIN };
        }
    }

    let profile = DeckProfile::build(gs, me, db);
    let p = gs.seat_player(me);
    let lives_in_hand = p
        .hand
        .cards
        .iter()
        .filter(|&&cid| db.get_card(cid).is_some_and(|c| c.card_type == CardType::Live))
        .count();

    // The development ladder: one Forward walk, stepped once per
    // check, projects the board `t` Main phases ahead. `t == 0` is
    // the current board (Forward::from_state seeds it there).
    let mut forward = v8_model::Forward::from_state(gs, me, db);
    let density = forward.density;
    let h = horizon();
    let disc = discount();

    let mut total = 0.0f64;
    for t in 0..h {
        let board_t = forward.hearts;
        let blades_t = forward.blades;

        let p_ammo = ammo_probability(lives_in_hand, t, &profile);
        let p_pass = if lives_in_hand > 0 {
            best_life_pass(&board_t, blades_t, density, gs, me, db)
        } else {
            deck_life_pass(&board_t, blades_t, density, gs, me, db)
        };
        total += disc.powi(t as i32) * p_ammo * p_pass;
        forward.step(db);
    }
    total
}

/// `P(we still hold a settable life at check t)`.
///
/// We hold `lives_now` lives, each check spends one, and each of
/// the `t` turns since now drew one card from a deck of
/// `profile.len` cards holding `profile.lives` lives. The number
/// drawn is hypergeometric, so:
///
/// ```text
/// P(ammo at t) = P(lives_now - t + X >= 1),  X ~ Hypergeometric
/// ```
fn ammo_probability(lives_now: usize, t: usize, profile: &DeckProfile) -> f64 {
    if lives_now > t {
        return 1.0;
    }
    // Need at least (t - lives_now + 1) lives among the t draws.
    let need = t - lives_now + 1;
    hypergeometric_ge(t, need, profile.lives, profile.len)
}

/// `P(the best settable life in hand passes)` against a projected
/// board. The best life is the one with the highest pass
/// probability, not the highest score: a certain 1-point placement
/// beats a coin-flip 7-point one that never resolves.
fn best_life_pass(
    board: &Acc,
    blades: i32,
    density: f64,
    gs: &GameState,
    me: u8,
    db: &CardDatabase,
) -> f64 {
    let p = gs.seat_player(me);
    let mut best = 0.0f64;
    for &cid in &p.hand.cards {
        if !db.get_card(cid).is_some_and(|c| c.card_type == CardType::Live) {
            continue;
        }
        let need = v8_model::life_need(gs, cid);
        if v8_model::has_unpassable_icon(&need) {
            continue;
        }
        let p_pass = v8_model::pass_estimate_from_supply(board, blades, density, &need);
        if p_pass > best {
            best = p_pass;
        }
    }
    best
}

/// `E[P(pass)]` over the lives remaining in the deck — the pass
/// probability of a life we have not drawn yet, averaged over the
/// deck's lives. Used when the hand holds no life, so the ammo
/// axis already carries the probability of drawing one.
fn deck_life_pass(
    board: &Acc,
    blades: i32,
    density: f64,
    gs: &GameState,
    me: u8,
    db: &CardDatabase,
) -> f64 {
    let p = gs.seat_player(me);
    let mut sum = 0.0f64;
    let mut count = 0usize;
    for &cid in &p.main_deck.cards {
        if !db.get_card(cid).is_some_and(|c| c.card_type == CardType::Live) {
            continue;
        }
        let need = v8_model::life_need(gs, cid);
        if v8_model::has_unpassable_icon(&need) {
            continue;
        }
        sum += v8_model::pass_estimate_from_supply(board, blades, density, &need);
        count += 1;
    }
    if count == 0 {
        0.0
    } else {
        sum / count as f64
    }
}
