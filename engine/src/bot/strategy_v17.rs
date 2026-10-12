//! Strategy bot v17 — the draw-probability bot.
//!
//! v17 is built around one question the user asked: *what is the
//! percentage of getting a good card to play?* Every decision is
//! an argmax over [`v17_model::expected_placements`], the expected
//! number of cards placed over the next few checks, where the
//! number of good cards in hand is a hypergeometric random
//! variable drawn from our own (fair-information) decklist.
//!
//! The three entry points:
//!
//! * **Mulligan** — enumerate every legal discard set, model the
//!   replacement as a uniform sample of the shuffled
//!   (deck ∪ discarded) pool, and keep the opening with the
//!   highest expected placements. The mulligan fires on the
//!   PERCENTAGE the opening converts, not on a curve pattern.
//! * **Live set** — argmax over every legal set of the immediate
//!   check payoff (the rule-exact [`v8_model::check_outcome`])
//!   plus the discounted expected placements AFTER committing the
//!   ammo. Setting a life you are likely to fail is priced as the
//!   future placement it forfeits.
//! * **Main phase / ability usage** — clone-and-execute every
//!   action and keep the one that most raises the placement
//!   forecast. A draw ability raises `P(ammo)` for every future
//!   check, a buff ability raises `P(pass)`, a deploy raises
//!   both — so an ability is used exactly when it improves the
//!   percentage of getting a good card to play.
//!
//! Fairness: reads own hand / deck / stage / energy / waitroom
//! plus the opponent's PUBLIC zones (stage, success count,
//! live-zone size). Never the opponent's hand, deck, energy deck
//! or set contents.

use crate::bot::strategy_common::{acc_add, emit_live_set, emit_mulligan, Acc};
use crate::bot::v17_model::{self};
use crate::bot::v8_model::{self, PLACEMENT_CREDIT};
use crate::card::{CardDatabase, CardType};
use crate::game_setup::{Action, ActionType};
use crate::game_state::GameState;

/// How many replacement redraws the mulligan averages over.
/// The replacement is a uniform sample of the shuffled deck, so
/// the opening value is an expectation over that sample.
fn redraw_samples() -> usize {
    std::env::var("V17_REDRAW_SAMPLES")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&n: &usize| (1..=256).contains(&n))
        .unwrap_or(24)
}

// ── Main phase / ability usage ────────────────────────────────────

/// Main-phase and pending-choice decision: argmax over every legal
/// action of the placement forecast it produces.
///
/// This is the ability-usage engine. An ability is a
/// [`ActionType::UseAbility`] (or a choice option a pending
/// ability prompt offers), and it is worth activating exactly when
/// the state it produces has a higher expected placement count —
/// a draw ability because it raises `P(ammo at t)` for every
/// future check, a buff ability because it raises `P(pass at t)`,
/// a blade ability because it raises both. No ability is special-
/// cased: the forecast reads the resolved state, so whatever the
/// ability actually does is what gets priced.
///
/// `Pass` ends the Main phase and is chosen only when no action
/// improves the forecast — the same discipline as v6/v7, expressed
/// in the probability currency instead of a feature delta.
pub fn choose_action(gs: &GameState, actions: &[Action], me: u8) -> Action {
    if actions.len() == 1 {
        return actions[0].clone();
    }
    let db = &gs.card_database;
    let pending = gs.has_pending_choice();
    let base = v17_model::expected_placements(gs, me, db);

    let mut vals = vec![f64::NEG_INFINITY; actions.len()];
    let mut best_nonpass = f64::NEG_INFINITY;

    for (i, a) in actions.iter().enumerate() {
        if a.parameters.as_ref().and_then(|p| p.disabled) == Some(true) {
            continue;
        }
        if a.action_type == ActionType::Pass && !pending {
            // Status quo: Pass leaves the position exactly as it is.
            vals[i] = base;
            continue;
        }
        crate::rng::seed(0x7637);
        let mut sim = gs.clone();
        if crate::game_setup::execute_action(&mut sim, a).is_err() {
            continue;
        }
        crate::game_setup::settle_single_player_state(&mut sim);
        let val = v17_model::expected_placements(&sim, me, db);
        vals[i] = val;
        if a.action_type != ActionType::Pass {
            best_nonpass = best_nonpass.max(val);
        }
    }

    // Pass ends the development phase: chosen only when no useful
    // action raises the forecast. A no-op action scores ~base, so
    // it never beats a real improvement and never displaces Pass
    // when nothing helps.
    if !pending {
        for (i, a) in actions.iter().enumerate() {
            if a.action_type == ActionType::Pass {
                vals[i] = if best_nonpass > base {
                    f64::NEG_INFINITY
                } else {
                    base
                };
            }
        }
    }

    let mut best = 0usize;
    let mut best_val = f64::NEG_INFINITY;
    for (i, &v) in vals.iter().enumerate() {
        if v > best_val {
            best_val = v;
            best = i;
        }
    }
    log::debug!(
        "v17 main t{} me{} base={:.3} best={:.3} act={:?}",
        gs.turn_number,
        me,
        base,
        best_val,
        actions[best].action_type,
    );
    actions[best].clone()
}

// ── Live set ──────────────────────────────────────────────────────

/// A life in hand, priced once: its hand index, its heart
/// requirement and its printed score.
struct Life {
    index: usize,
    need: Acc,
    score: i32,
}

/// Live-card-set decision: argmax over every legal set of the
/// immediate check payoff plus the discounted future.
///
/// The immediate term is the rule-exact
/// [`v8_model::check_outcome`] — the joint (we-place, they-place)
/// distribution turned into the real win-rule payoff. The future
/// term is [`v17_model::expected_placements`] computed on the
/// state AFTER the set is committed, so a life spent on a check
/// we are likely to fail is priced as the future placement it
/// forfeits. The fold (set nothing) is a candidate too, so the
/// bot only commits ammo when the expected value beats keeping it.
pub fn choose_live_set(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    let me = gs.active_player_index();
    let lives = settable_lives(gs, me, db);
    let max_slots = (3i32 - i32::from(gs.seat_player(me).live_card_set_limit_reduction).max(0))
        .max(0) as usize;

    // The fold: commit nothing, keep the ammo, take the discounted
    // future. Every non-empty set has to beat this to be worth it.
    let mut best_value = v17_model::discount() * v17_model::expected_placements(gs, me, db);
    let mut best_set: Vec<usize> = Vec::new();

    if !lives.is_empty() && max_slots > 0 {
        let opp = v8_model::OppModel::build(gs, me, db);
        let my_success = gs.seat_player(me).success_live_card_zone.cards.len();
        let sampler = v8_model::FlipSampler::build(gs, me, db);
        let board = v8_model::board_pool(gs, me, db);
        let pools = sampler.sample_pools(&board, v8_model::flip_samples());
        let blades = v8_model::active_blades(gs, me, db);
        let yell = v8_model::expected_yell_score(gs, me, db, blades);

        let n = lives.len().min(8);
        for mask in 1u32..(1u32 << n) {
            let size = mask.count_ones() as usize;
            if size > max_slots {
                continue;
            }
            let mut need: Acc = [0i16; 11];
            let mut idxs = Vec::new();
            let mut score = 0i32;
            for bit in 0..n {
                if mask >> bit & 1 == 1 {
                    let life = &lives[bit];
                    for k in 0..11 {
                        need[k] = need[k].saturating_add(life.need[k]);
                    }
                    score += life.score;
                    idxs.push(life.index);
                }
            }
            if v8_model::has_unpassable_icon(&need) {
                continue;
            }
            let p_pass = v8_model::pass_probability(&pools, &need);
            let outcome =
                v8_model::check_outcome(p_pass, score + yell, size, my_success, &opp);
            let future = expected_placements_after_set(gs, me, db, &idxs);
            // `check_outcome.value` is in v8 units, where one
            // placement is worth `PLACEMENT_CREDIT` (1/3 of the
            // game) and a rule payoff is worth 1.0. The future
            // term is in placements, where one placement is
            // 1.0. Divide the immediate term by
            // `PLACEMENT_CREDIT` to put both in the same
            // currency — otherwise the future always dominates
            // and the bot never commits a life.
            let value =
                outcome.value / PLACEMENT_CREDIT + v17_model::discount() * future;
            if value > best_value {
                best_value = value;
                best_set = idxs;
            }
        }
    }

    // Spare slots take the cards whose loss costs least — a set
    // card is discarded before the check and draws a replacement,
    // so a dead non-life in a spare slot is free digging.
    junk_fill(gs, me, db, &mut best_set, max_slots);

    log::debug!(
        "v17 live t{} me{} set={} value={:.3}",
        gs.turn_number,
        me,
        best_set.len(),
        best_value,
    );
    emit_live_set(gs, actions, &best_set)
}

/// Lives in hand that can be set, with their requirements and
/// printed scores. Lives whose requirement carries a Draw or Score
/// icon are excluded — they are not heart checks and cannot be
/// priced by the pass model.
fn settable_lives(gs: &GameState, me: u8, db: &CardDatabase) -> Vec<Life> {
    let p = gs.seat_player(me);
    let mut out = Vec::new();
    for (index, &cid) in p.hand.cards.iter().enumerate() {
        if !db.get_card(cid).is_some_and(|c| c.card_type == CardType::Live) {
            continue;
        }
        let need = v8_model::life_need(gs, cid);
        if v8_model::has_unpassable_icon(&need) {
            continue;
        }
        let score = db.get_card(cid).and_then(|c| c.score).unwrap_or(0) as i32;
        out.push(Life { index, need, score });
    }
    out
}

/// Expected placements from the state after committing `idxs` to the
/// live zone: the set cards leave the hand, so the ammo axis of the
/// forecast drops by the set size.
fn expected_placements_after_set(
    gs: &GameState,
    me: u8,
    db: &CardDatabase,
    idxs: &[usize],
) -> f64 {
    let mut sim = gs.clone();
    {
        let p = sim.seat_player_mut(me);
        let mut skip = idxs.to_vec();
        skip.sort_unstable();
        skip.dedup();
        for &hi in skip.iter().rev() {
            if hi < p.hand.cards.len() {
                p.hand.cards.remove(hi);
            }
        }
    }
    v17_model::expected_placements(&sim, me, db)
}

/// Marginal keep-value of a non-life hand card, in pass-probability
/// units: what the card adds to the best life's chance of passing
/// if it stays in hand. The live set fills spare slots with the
/// LOWEST keep-value cards first.
fn junk_keep_value(
    db: &CardDatabase,
    cid: i16,
    board: &Acc,
    blades: i32,
    density: f64,
    best_need: Option<&Acc>,
) -> f64 {
    let Some(card) = db.get_card(cid) else {
        return 0.0;
    };
    match card.card_type {
        // Energy buys the deploy that raises the check, so one
        // energy is worth one more shot at the check.
        CardType::Energy => PLACEMENT_CREDIT,
        CardType::Member => {
            let mut with = *board;
            if let Some(base) = &card.base_heart {
                acc_add(&mut with, &base.hearts);
            }
            let new_blades = blades + i32::from(card.blade);
            match best_need {
                Some(need) => {
                    v8_model::pass_estimate_from_supply(&with, new_blades, density, need)
                }
                None => 0.0,
            }
        }
        _ => 0.0,
    }
}

/// Fill spare live-set slots with the cards whose loss costs least.
///
/// Digging only pays when the deck can actually yield a life
/// (v7's condition: a set card is discarded before the check
/// and draws a replacement, so a junk card is only worth
/// spending when a life might come back), and the player is
/// allowed to live at all.
fn junk_fill(
    gs: &GameState,
    me: u8,
    db: &CardDatabase,
    desired: &mut Vec<usize>,
    max_slots: usize,
) {
    if desired.len() >= max_slots {
        return;
    }
    let p = gs.seat_player(me);
    if gs.cannot_live_players.contains(&p.id) {
        return;
    }
    let deck_has_lives = p
        .main_deck
        .cards
        .iter()
        .any(|&cid| db.get_card(cid).is_some_and(|c| c.card_type == CardType::Live));
    if !deck_has_lives {
        return;
    }
    let board = v8_model::board_supply(gs, me, db);
    let blades = v8_model::active_blades(gs, me, db);
    let (_, density) = crate::bot::strategy_v4::flip_stats(gs, me, db);
    let best_need = settable_lives(gs, me, db)
        .iter()
        .map(|life| {
            (
                v8_model::pass_estimate_from_supply(&board, blades, density, &life.need),
                life.need,
            )
        })
        .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(_, need)| need);

    let mut junk: Vec<(usize, f64)> = p
        .hand
        .cards
        .iter()
        .enumerate()
        .filter(|(hi, _)| !desired.contains(hi))
        .filter(|(_, &cid)| {
            db.get_card(cid).is_some_and(|c| c.card_type != CardType::Live)
        })
        .map(|(hi, &cid)| {
            (
                hi,
                junk_keep_value(db, cid, &board, blades, density, best_need.as_ref()),
            )
        })
        .collect();
    junk.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    for (hi, _) in junk {
        if desired.len() >= max_slots {
            break;
        }
        desired.push(hi);
    }
}

// ── Mulligan ────────────────────────────────────────────────────────

/// Mulligan decision: enumerate every legal discard set, model the
/// replacement as a uniform sample of the shuffled deck, and keep
/// the opening with the highest expected placements.
///
/// This is the "percentage of getting a good card" decision. The
/// replacement cards are NOT read off the top of the deck — the
/// deck is shuffled, so the replacement is a random sample of the
/// (deck ∪ discarded) pool, and the opening's value is the
/// expectation over that sample. A hand is kept when the percentage
/// it converts is high enough that no replacement beats it.
///
/// Ties prefer keeping more cards: a replacement is only worth it
/// when the redraw is strictly better, which is the discipline a
/// "always replace" rule lacks.
pub fn choose_mulligan(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    let me = gs.active_player_index();
    let hand = gs.seat_player(me).hand.cards.to_vec();
    if hand.is_empty() {
        return crate::bot::strategy_v4::choose_mulligan_v4(gs, actions, db);
    }
    // The engine permits zero through six replacements; anything
    // above the hand size is illegal.
    let max_replacements = hand.len().min(6);
    let mut best_discard: Vec<usize> = Vec::new();
    let mut best_value = f64::NEG_INFINITY;
    let subsets = 1usize << hand.len().min(16);
    for mask in 0..subsets {
        let discard: Vec<usize> =
            (0..hand.len()).filter(|i| mask & (1 << i) != 0).collect();
        if discard.len() > max_replacements {
            continue;
        }
        let value = opening_value(gs, me, db, &discard);
        let better = value > best_value + 1e-9
            || ((value - best_value).abs() <= 1e-9 && discard.len() < best_discard.len());
        if better {
            best_value = value;
            best_discard = discard;
        }
    }
    log::debug!(
        "v17 mulligan t{} me{} hand={} discard={:?} value={:.3}",
        gs.turn_number,
        me,
        hand.len(),
        best_discard,
        best_value,
    );
    emit_mulligan(gs, actions, &best_discard)
}

/// Expected placements of the opening that discards `discard`.
///
/// The kept hand is fixed; the `discard.len()` replacement cards are
/// a uniform sample of the shuffled (deck ∪ discarded) pool, so the
/// value is the average over [`redraw_samples`] draws.
fn opening_value(gs: &GameState, me: u8, db: &CardDatabase, discard: &[usize]) -> f64 {
    let p = gs.seat_player(me);
    let hand: Vec<i16> = p.hand.cards.to_vec();
    let keep: Vec<i16> = hand
        .iter()
        .enumerate()
        .filter(|(i, _)| !discard.contains(i))
        .map(|(_, &cid)| cid)
        .collect();
    let discarded: Vec<i16> = discard.iter().map(|&i| hand[i]).collect();

    // The shuffled pool the replacement is drawn from: everything
    // still in the deck plus the cards being returned to it.
    let mut pool: Vec<i16> = p.main_deck.cards.to_vec();
    pool.extend(discarded.iter().copied());
    let n_draw = discarded.len();
    if n_draw == 0 || pool.len() < n_draw {
        return opening_value_with(gs, me, db, &keep, &[], &discarded);
    }

    let mut total = 0.0f64;
    let samples = redraw_samples();
    let mut rng = 0x6a09e667f3bcc909u64;
    for _ in 0..samples {
        let mut sample_pool = pool.clone();
        let mut redraw = Vec::with_capacity(n_draw);
        for k in 0..n_draw {
            // xorshift64* — a private stream that never touches the
            // engine RNG, so the decision is reproducible.
            rng ^= rng >> 12;
            rng ^= rng << 25;
            rng ^= rng >> 27;
            let index = k + (rng.wrapping_mul(0x2545_F491_4F6C_DD1D)
                % (sample_pool.len() - k) as u64) as usize;
            redraw.push(sample_pool.swap_remove(index));
        }
        total += opening_value_with(gs, me, db, &keep, &redraw, &discarded);
    }
    total / samples as f64
}

/// Expected placements of a concrete opening: `keep` plus `redraw`
/// in hand, with the redrawn cards removed from the deck and the
/// non-redrawn discards returned to it.
fn opening_value_with(
    gs: &GameState,
    me: u8,
    db: &CardDatabase,
    keep: &[i16],
    redraw: &[i16],
    discarded: &[i16],
) -> f64 {
    let mut sim = gs.clone();
    {
        let p = sim.seat_player_mut(me);
        // Hand: kept cards plus the replacement draw.
        p.hand.cards.clear();
        for &cid in keep {
            p.hand.cards.push(cid);
        }
        for &cid in redraw {
            p.hand.cards.push(cid);
        }
        // Deck: (original deck ∪ discarded) minus the redrawn cards,
        // which are now in hand.
        let mut deck_pool: Vec<i16> = p.main_deck.cards.to_vec();
        deck_pool.extend(discarded.iter().copied());
        let mut remove: Vec<i16> = redraw.to_vec();
        let mut new_deck: Vec<i16> = Vec::new();
        for &cid in &deck_pool {
            if let Some(pos) = remove.iter().position(|&r| r == cid) {
                remove.swap_remove(pos);
            } else {
                new_deck.push(cid);
            }
        }
        p.main_deck.cards.clear();
        for cid in new_deck {
            p.main_deck.cards.push(cid);
        }
    }
    v17_model::expected_placements(&sim, me, db)
}
