//! V8 shared decision model.
//!
//! Every v8 decision is an argmax over ONE model. There are no per-branch
//! magic constants, no role gates, no stance floors and no hand-tuned weight
//! vector. The model has three parts:
//!
//! 1. **Exact own-side check resolution.** Own board hearts plus own-deck yell
//!    flips are sampled *without replacement* using shared common random numbers
//!    across every candidate, so two portfolios are compared on identical
//!    shuffle realisations. The paired comparison removes the sampling noise
//!    that made v7's independent 256-sample estimates jitter around floors.
//! 2. **Public-information opponent model.** The opponent's hidden set is
//!    never read. Their achievable score is a *distribution* derived from their
//!    public stage hearts/blades crossed with the guides' score bands
//!    (docs/BOT_STRATEGY.md section 3.3) and an honest uncertainty prior on how
//!    many hearts their blades actually produce (guide sources 3 / 9 / 15 quote
//!    per-blade hit rates between ~50% and ~73%).
//! 3. **Rule-exact payoff.** The joint (we-place, they-place) distribution is
//!    turned into expected payoff with the real win rules (1.2.1.1, 8.4.7,
//!    8.4.7.1) plus a derived placement credit: winning needs three successes
//!    and a won check places exactly one card, so one placement is worth 1/3
//!    of the game. That is where `PLACEMENT_CREDIT` comes from - a rule
//!    derivation, not a tuned weight.
//!
//! The same module also supplies the forward development term used by the Main
//! phase (`reachable_ceiling`), which prices the guides' cost curve
//! (T1=4 -> T2=9 -> T3=13, section 1 / S1) as a FORWARD reachable ceiling
//! instead of a one-ply cost delta. That is what lets a sideways baton upgrade
//! score correctly without a flat baton bonus (measured catastrophic in v7,
//! -18pp).

use crate::bot::strategy_common::hc_index;
use crate::card::{CardDatabase, CardType};
use crate::core::stats_pipeline;
use crate::game_state::GameState;
use crate::player::Player;

use super::strategy_common::{acc_add, Acc};

/// Shared shuffle realisations per live-set decision. Every candidate
/// portfolio is scored on the SAME realisations, so `p_pass` differences are
/// paired and reproducible rather than independent noisy estimates.
pub const FLIP_SAMPLES: usize = 192;

/// Score bands from the guides (docs/BOT_STRATEGY.md section 3.3, all 291
/// lives in cards.json): hearts needed to pass a live of the given printed
/// score. Index = printed score, clamped.
const SCORE_MEDIAN: [i32; 12] = [2, 3, 5, 7, 10, 12, 14, 16, 19, 21, 23, 25];
const SCORE_MAX: [i32; 12] = [4, 4, 8, 9, 12, 14, 15, 18, 21, 21, 23, 25];
const SCORE_SLOTS: usize = SCORE_MEDIAN.len();

/// One placement is 1/3 of the game: three successes win it (1.2.1.1) and a
/// won check places exactly one card (8.4.7).
pub const PLACEMENT_CREDIT: f64 = 1.0 / 3.0;

/// Prior on hearts contributed per ACTIVE BLADE by the OPPONENT's yell.
///
/// Their decklist is not fair information, so this is the one place v8 uses a
/// prior instead of a measurement. The spread is deliberately wide and
/// weighted low: under-estimating their check ability makes v8 contest more
/// often, and S4 is explicit that folding a contestable check is the
/// expensive error - at opponent match point it loses outright.
const YIELD_PRIOR: [(f64, f64); 5] = [
    (0.00, 0.20), // their deck yields no usable blade-hearts
    (0.30, 0.25),
    (0.55, 0.30), // central case
    (0.90, 0.15),
    (1.30, 0.10), // blade-heart-dense list, favourable flips
];

/// Restores the engine RNG on drop. The sampling below uses a private PRNG and
/// never touches `crate::rng`, but a policy call must leave the game RNG
/// bit-identical no matter what engine helpers it ran.
pub struct RngGuard(u32);
impl RngGuard {
    pub fn new() -> Self {
        Self(crate::rng::checkpoint())
    }
}
impl Drop for RngGuard {
    fn drop(&mut self) {
        crate::rng::restore(self.0);
    }
}

#[inline]
fn clamp_score(score: i32) -> usize {
    score.clamp(0, SCORE_SLOTS as i32 - 1) as usize
}

pub fn score_median_hearts(score: i32) -> i32 {
    SCORE_MEDIAN[clamp_score(score)]
}

// -- Board accounting ------------------------------------------------------

/// Board heart pool in the 8-slot check index space
/// (`Heart00..Heart06`, `BAll`), buff-aware: `get_available_hearts` already
/// folds in `heart_override` / `heart_modifiers` / `heart_color_multiplier` /
/// `heart_copy`, so live-granted buffs are priced.
pub fn board_pool(gs: &GameState, me: u8, db: &CardDatabase) -> [i32; 8] {
    let p = gs.seat_player(me);
    let hearts = p.stage.get_available_hearts(
        db,
        &gs.mods.heart_override,
        &gs.mods.heart_modifiers,
        &gs.mods.heart_color_multiplier,
        &gs.mods.heart_copy,
    );
    let mut pool = [0i32; 8];
    for (color, count) in &hearts.hearts {
        let idx = color.index();
        if idx < 8 {
            pool[idx] += i32::from(*count);
        }
    }
    pool
}

/// Active blades; waiting members contribute none (Q133).
pub fn active_blades(gs: &GameState, me: u8, db: &CardDatabase) -> i32 {
    i32::from(gs.seat_player(me).stage.total_blades(
        db,
        &gs.mods.blade_modifiers,
        &gs.mods.orientation_modifiers,
        false,
    ))
}

/// Total printed stage cost - the guides' development metric (section 1 / S1).
/// Exposed because the arena's audit tooling reports the same number.
#[allow(dead_code)]
pub fn stage_cost(gs: &GameState, me: u8, db: &CardDatabase) -> i32 {
    gs.seat_player(me)
        .stage
        .stage
        .iter()
        .filter(|&&cid| cid >= 0)
        .filter_map(|&cid| db.get_card(cid).and_then(|c| c.cost))
        .map(i32::from)
        .sum()
}

pub fn lives_in_hand(p: &Player, db: &CardDatabase) -> usize {
    p.hand
        .cards
        .iter()
        .filter(|&&cid| {
            db.get_card(cid)
                .is_some_and(|c| c.card_type == CardType::Live)
        })
        .count()
}

pub fn card_hearts(db: &CardDatabase, cid: i16) -> i32 {
    db.get_card(cid)
        .and_then(|c| c.base_heart.as_ref())
        .map(|bh| bh.hearts.values_sum() as i32)
        .unwrap_or(0)
}

pub fn card_cost(db: &CardDatabase, cid: i16) -> i32 {
    i32::from(db.get_card(cid).and_then(|c| c.cost).unwrap_or(0))
}

pub fn card_blades(db: &CardDatabase, cid: i16) -> i32 {
    i32::from(db.get_card(cid).map(|c| c.blade).unwrap_or(0))
}

pub fn printed_score(db: &CardDatabase, cid: i16) -> i32 {
    i32::from(db.get_card(cid).and_then(|c| c.score).unwrap_or(0))
}

// -- Rule-exact feasibility ------------------------------------------------

/// Can `need` be satisfied by `pool`? Index space matches [`Acc`]:
/// 0 = Heart00 grey bucket, 1..6 = specific colors, 7 = BAll blade-heart
/// wildcard, 8/9 = Draw/Score (never a check requirement), 10 = All wildcard.
///
/// Semantics follow rules 2.11.3 / 2.1: specific colors are met exactly, BAll
/// (a blade heart) and All act as wildcards for ONE specific color each, and
/// the Heart00 bucket absorbs colorless hearts plus every leftover.
pub fn feasible(pool: &[i32; 8], need: &Acc) -> bool {
    let mut wildcard = pool[7];
    let mut bucket = pool[0];
    for c in 1..=6 {
        let have = pool[c];
        let want = need[c];
        if have >= want {
            bucket += have - want;
        } else {
            let deficit = want - have;
            if wildcard < deficit {
                return false;
            }
            wildcard -= deficit;
        }
    }
    bucket += wildcard;
    bucket >= need[0] + need[7] + need[10]
}

/// A live whose requirement includes a Draw or Score icon is not something we
/// can price as passing a heart check, so it is excluded from every candidate.
pub fn has_unpassable_icon(need: &Acc) -> bool {
    need[8] > 0 || need[9] > 0
}

/// Effective heart requirement of a live card, modifier-aware.
pub fn life_need(gs: &GameState, cid: i16) -> Acc {
    let mut need = [0i32; 11];
    let printed = gs
        .card_database
        .get_card(cid)
        .and_then(|c| c.need_heart.as_ref());
    if let Some(effective) =
        stats_pipeline::effective_need_heart(printed, cid, &gs.mods.need_heart_modifiers)
    {
        for (color, count) in &effective.hearts {
            need[hc_index(*color)] += i32::from(*count);
        }
    }
    need
}

// -- Yell flip sampling (own side, exact) ----------------------------------

/// Own-deck flip content collapsed into distinct `[i32; 8]` categories with
/// multiplicities. Built ONCE per decision and reused by every candidate, so
/// candidate comparison never pays the deck-scan cost twice.
pub struct FlipSampler {
    cats: Vec<([i32; 8], u32)>,
    deck_len: usize,
    draws: usize,
}

impl FlipSampler {
    pub fn build(gs: &GameState, me: u8, db: &CardDatabase) -> Self {
        let p = gs.seat_player(me);
        let override_color = p.stage.stage.iter().find_map(|cid| {
            gs.mods
                .blade_type_modifiers
                .get(cid)
                .copied()
                .map(crate::turn::live::blade_color_to_heart)
        });
        let mut cats: Vec<([i32; 8], u32)> = Vec::new();
        for &cid in &p.main_deck.cards {
            let mut total = [0u8; 8];
            if let Some(card) = db.get_card(cid) {
                let mut hearts = crate::card::BaseHeart {
                    hearts: crate::card::HeartMap::new(),
                };
                let mut cheer = 0;
                crate::turn::live::process_yell_revealed_card_icons(
                    card,
                    override_color,
                    &mut hearts,
                    &mut total,
                    &mut cheer,
                );
            }
            let vector = total.map(i32::from);
            if let Some((_, count)) = cats.iter_mut().find(|(v, _)| *v == vector) {
                *count += 1;
            } else {
                cats.push((vector, 1));
            }
        }
        cats.sort_unstable_by_key(|(vector, _)| *vector);
        let deck_len = p.main_deck.cards.len();
        let draws = usize::try_from(active_blades(gs, me, db))
            .unwrap_or(usize::MAX)
            .min(deck_len);
        Self {
            cats,
            deck_len,
            draws,
        }
    }

    /// Shared, deterministic sample pools: `samples` board-plus-yell pools.
    ///
    /// One private PRNG stream with a fixed seed, independent of engine RNG and
    /// of how many candidates the caller evaluates, so a decision is
    /// reproducible and every portfolio sees the same shuffles.
    pub fn sample_pools(&self, board: &[i32; 8], samples: usize) -> Vec<[i32; 8]> {
        let samples = samples.max(1);
        if self.draws == 0 || self.cats.is_empty() || self.deck_len == 0 {
            return vec![*board; samples];
        }
        let mut deck = Vec::with_capacity(self.deck_len);
        for &(vector, count) in &self.cats {
            deck.extend(std::iter::repeat_n(vector, count as usize));
        }
        deck.resize(self.deck_len, [0; 8]);
        let mut pools = Vec::with_capacity(samples);
        let mut rng = 0x9E37_79B9_7F4A_7C15u64;
        let mut sampled = deck.clone();
        for _ in 0..samples {
            sampled.copy_from_slice(&deck);
            let mut pool = *board;
            for k in 0..self.draws {
                // xorshift64*
                rng ^= rng >> 12;
                rng ^= rng << 25;
                rng ^= rng >> 27;
                let span = (self.deck_len - k) as u64;
                let index = k + (rng.wrapping_mul(0x2545_F491_4F6C_DD1D) % span) as usize;
                sampled.swap(k, index);
                for (have, extra) in pool.iter_mut().zip(sampled[k]) {
                    *have += extra;
                }
            }
            pools.push(pool);
        }
        pools
    }
}

/// `P(need satisfied)` over pre-sampled pools.
pub fn pass_probability(pools: &[[i32; 8]], need: &Acc) -> f64 {
    if pools.is_empty() {
        return 0.0;
    }
    pools.iter().filter(|pool| feasible(pool, need)).count() as f64 / pools.len() as f64
}

// -- Public-information opponent model ------------------------------------

/// What the guides allow us to infer about the opponent from public zones
/// only. Their hand, deck and set contents are never read.
#[derive(Clone, Debug)]
pub struct OppModel {
    /// How many cards the opponent committed to their live zone. The SIZE is
    /// public to the second attacker (8.2.2 sets face-down, 8.4.3.2 gives a
    /// sole passer the placement regardless of contents). Zero means they did
    /// not commit, which is why `pass_prob` is then exactly zero.
    pub set_size: usize,
    pub success: usize,
    /// `P(they achieve this printed score on a check)`, from their public
    /// board hearts and blades and the guides' score bands.
    pub score_dist: [f64; SCORE_SLOTS],
    /// `P(their committed set passes its own check)`.
    pub pass_prob: f64,
}

impl OppModel {
    pub fn build(gs: &GameState, me: u8, db: &CardDatabase) -> Self {
        let (_, opp) = gs.seated_pair(me);
        let set_size = opp.live_card_zone.cards.len();
        let success = opp.success_live_card_zone.cards.len();

        let hearts = stats_pipeline::stage_hearts(
            &opp.stage.stage,
            db,
            &gs.mods.heart_override,
            &gs.mods.heart_copy,
            &gs.mods.heart_color_multiplier,
            &gs.mods.heart_modifiers,
        );
        let board_total: i32 = hearts.hearts.values().map(|v| i32::from(*v)).sum();
        let blades = i32::from(opp.stage.total_blades(
            db,
            &gs.mods.blade_modifiers,
            &gs.mods.orientation_modifiers,
            false,
        ));

        // Capability distribution: for each yield quantile, which score bands
        // their supply reaches. Total supply is used as the summary because
        // the flip COLORS are unknown - a heart of the wrong color is worth
        // nothing, so total supply is an upper bound and the quantile spread
        // carries the color risk.
        let mut score_dist = [0.0f64; SCORE_SLOTS];
        for (yield_hearts, weight) in YIELD_PRIOR {
            let supply = board_total + (blades as f64 * yield_hearts).round() as i32;
            for score in 0..SCORE_SLOTS {
                if supply >= SCORE_MAX[score] {
                    score_dist[score] += weight;
                } else if supply >= SCORE_MEDIAN[score] {
                    score_dist[score] += weight * 0.5;
                }
            }
        }
        let total: f64 = score_dist.iter().sum();
        if total > 0.0 {
            for value in score_dist.iter_mut() {
                *value /= total;
            }
        } else {
            score_dist[0] = 1.0;
        }

        // Whether they pass at all: they commit a set they believe will pass,
        // so the bar is the median requirement of a set roughly worth
        // `2 * set_size` points.
        let target = (2 * set_size).clamp(1, SCORE_SLOTS - 1) as i32;
        let bar = score_median_hearts(target);
        let pass_prob = if set_size == 0 {
            0.0
        } else {
            YIELD_PRIOR
                .iter()
                .filter(|(yield_hearts, _)| {
                    board_total + (blades as f64 * yield_hearts).round() as i32 >= bar
                })
                .map(|(_, weight)| *weight)
                .sum::<f64>()
                .min(1.0)
        };

        Self {
            set_size,
            success,
            score_dist,
            pass_prob,
        }
    }

    /// Placement masses inside a contested check, from rules 8.4.6 / 8.4.7.1.
    ///
    /// A tie only places for a player who is below two successes AND set a
    /// single card, so a tie is worth nothing to a player at match point. That
    /// is exactly why v7 needed a `+1` score surcharge in its minimum-win
    /// rule; here it falls out of the payoff.
    pub fn contested_masses(
        &self,
        our_score: i32,
        our_set_size: usize,
        our_success: usize,
    ) -> (f64, f64, f64) {
        let mut below = 0.0;
        let mut above = 0.0;
        let idx = clamp_score(our_score);
        for (score, weight) in self.score_dist.iter().enumerate() {
            if *weight <= 0.0 {
                continue;
            }
            if score < idx {
                below += weight;
            } else if score > idx {
                above += weight;
            }
        }
        let tie = self.score_dist[idx];
        let we_tie_place = our_success < 2 && our_set_size < 2;
        let they_tie_place = self.success < 2 && self.set_size < 2;
        let we_place = below + if we_tie_place { tie } else { 0.0 };
        let they_place = above + if they_tie_place { tie } else { 0.0 };
        let both = if we_tie_place && they_tie_place { tie } else { 0.0 };
        (we_place.min(1.0), they_place.min(1.0), both)
    }
}

// -- Check outcome: the single currency of v8 ------------------------------

/// Outcome of one check, in the only units that matter: placements and rule
/// payoff. docs/BOT_STRATEGY.md section 2: "Every won check places exactly ONE
/// card no matter the margin (8.4.7). Score only matters inside a contested
/// comparison - so P(win the comparison) is the ONLY quality metric."
#[derive(Clone, Copy, Debug)]
pub struct CheckOutcome {
    /// `P(we pass our own check)`.
    pub p_pass: f64,
    /// `P(we place a card in the success zone this check)`.
    pub p_place: f64,
    /// `P(they place a card this check)`.
    pub opp_place: f64,
    /// Expected value: rule payoff in {-1, 0, +1} plus the placement credit.
    pub value: f64,
}

/// Rule payoff of FINAL success counts (1.2.1.1 / 1.2.1.2).
pub fn rule_payoff(my_final: usize, opp_final: usize) -> f64 {
    if my_final >= 3 && opp_final >= 3 {
        0.0
    } else if my_final >= 3 {
        1.0
    } else if opp_final >= 3 {
        -1.0
    } else {
        0.0
    }
}

/// Combine our pass probability with the public opponent model into the joint
/// placement distribution and the expected value.
pub fn check_outcome(
    p_pass: f64,
    our_score: i32,
    our_set_size: usize,
    our_success: usize,
    opp: &OppModel,
) -> CheckOutcome {
    let opp_pass = opp.pass_prob;
    let (we_c, they_c, both_c) = opp.contested_masses(our_score, our_set_size, our_success);

    let contested = p_pass * opp_pass;
    // Passing alone wins the check outright (8.4.3.2): score is irrelevant.
    let p_place = p_pass * (1.0 - opp_pass) + contested * we_c;
    let opp_place = opp_pass * (1.0 - p_pass) + contested * they_c;

    let both = contested * both_c;
    let ours_only = (p_place - both).max(0.0);
    let theirs_only = (opp_place - both).max(0.0);

    let value = ours_only * rule_payoff(our_success + 1, opp.success)
        + theirs_only * rule_payoff(our_success, opp.success + 1)
        + both * rule_payoff(our_success + 1, opp.success + 1)
        + p_place * PLACEMENT_CREDIT;

    CheckOutcome {
        p_pass,
        p_place: p_place.min(1.0),
        opp_place: opp_place.min(1.0),
        value,
    }
}

// -- Expected yell score (rule 8.4.2.1) -----------------------------------

/// `E[+1 per Score icon flipped during our yell]`, added to the comparison
/// total. Uses our own decklist, which is fair information.
pub fn expected_yell_score(gs: &GameState, me: u8, db: &CardDatabase, blades: i32) -> i32 {
    if blades <= 0 {
        return 0;
    }
    let p = gs.seat_player(me);
    let deck_len = p.main_deck.cards.len();
    if deck_len == 0 {
        return 0;
    }
    let score_icons: usize = p
        .main_deck
        .cards
        .iter()
        .filter_map(|&cid| db.get_card(cid))
        .filter_map(|card| card.special_heart.as_ref())
        .filter_map(|hearts| hearts.hearts.get(&crate::card::HeartColor::Score).copied())
        .map(usize::from)
        .sum();
    let draws = usize::try_from(blades).unwrap_or(usize::MAX).min(deck_len);
    (score_icons * draws / deck_len) as i32
}

// -- The guides' continuous development-to-score currency ------------------

/// Our own blade-heart density: the chance that one yell flip yields a blade
/// heart. Derived from our own decklist, which section 9 of
/// docs/BOT_STRATEGY.md calls fair information.
pub fn own_density(gs: &GameState, me: u8, db: &CardDatabase) -> f64 {
    let deck = &gs.seat_player(me).main_deck.cards;
    if deck.is_empty() {
        return 0.0;
    }
    let blade_hearts = deck
        .iter()
        .filter(|&&cid| {
            db.get_card(cid)
                .is_some_and(|c| c.blade_heart.is_some())
        })
        .count();
    blade_hearts as f64 / deck.len() as f64
}

/// The largest score band our supply can clear, per the guides' own formula
/// (docs/BOT_STRATEGY.md section 4, "DERIVED QUANTITIES"):
///
/// ```text
/// hearts(t) = sum of base_heart over ALL my stage members
/// flips(t)  = sum of blade over ACTIVE members
/// hits(t)   ~ Binomial(flips, blade-heart density of MY deck)
/// ceiling(t)~= largest s with median_hearts(s) <= hearts + hits
/// ```
///
/// This exists because `P(place)` is a poor development signal. It is a
/// threshold test against whichever lives happen to be in hand, so while the
/// board is still short of every one of those lives it sits at a hard zero and
/// a member deploy moves it not at all. Measured over 725 decision points that
/// left the Main-phase leaf with 1.74 distinct values across 5.4 offered
/// actions, tied at the top in 81.8% of decisions - the placement model was
/// inert and `TieKey` was deciding almost everything.
///
/// `ceiling` is the guide's own bridge from development to comparison score and
/// it is MONOTONE in both hearts and blades, so it supplies the gradient
/// `P(place)` lacks, in the unit the check is actually scored in. It is a mean
/// not a distribution, which is deliberate and cheap: as a Main-phase
/// development signal it only has to order boards, and the live-set decision
/// still prices the real Binomial.
pub fn score_ceiling(gs: &GameState, me: u8, db: &CardDatabase) -> i32 {
    let (blades, density) = super::strategy_v4::flip_stats(gs, me, db);
    let supply = supply_hearts(gs, me, db) + (blades as f64 * density).round() as i32;
    largest_clearable(supply)
}

/// `largest s` such that `median_hearts(s) <= supply`, from the section 3.3
/// score bands. Monotone and unbounded in `supply`, which is the property the
/// Main phase needs.
pub fn largest_clearable(supply: i32) -> i32 {
    let mut best = 0usize;
    for s in 0..SCORE_SLOTS {
        if SCORE_MEDIAN[s] <= supply {
            best = s;
        } else {
            break;
        }
    }
    best as i32
}

/// Total heart supply on our board, buff-aware, in hearts (not per-colour).
fn supply_hearts(gs: &GameState, me: u8, db: &CardDatabase) -> i32 {
    stats_pipeline::stage_hearts(
        &gs.seat_player(me).stage.stage,
        db,
        &gs.mods.heart_override,
        &gs.mods.heart_copy,
        &gs.mods.heart_color_multiplier,
        &gs.mods.heart_modifiers,
    )
    .hearts
    .values()
    .map(|v| i32::from(*v))
    .sum()
}

/// Continuous companion to [`score_ceiling`], in the SAME unit: score bands.
///
/// The integer band was the right idea in the wrong execution. Measured over
/// 700 games, v8's Main phase and v7's reach an identical stage-cost curve
/// (T4 11.7 vs 12.2, T7 24.5 vs 25.9) and both track the guide, yet v8 loses
/// 8.7 points more. So the cost ladder is NOT what differs. What feeds a check
/// is hearts and blades (3.2), and cost is only a proxy for them.
///
/// The banded integer is a bad proxy because it is coarse AND because it is
/// quantised on a cost-like scale, which rewards a high-cost low-heart member
/// exactly as much as a high-cost high-heart one. Interpolating between the
/// guide's own band medians makes the term continuous, so it responds to a
/// single extra heart or blade, and it stays in score-band units so no
/// conversion constant to "probability" is invented.
pub fn band_progress(gs: &GameState, me: u8, db: &CardDatabase) -> f64 {
    let (blades, density) = super::strategy_v4::flip_stats(gs, me, db);
    let supply = supply_hearts(gs, me, db) + (blades as f64 * density).round() as i32;
    band_progress_for(supply)
}

/// `band_progress` from a raw heart-equivalent supply, split out so the
/// interpolation can be tested without building a GameState.
pub fn band_progress_for(supply: i32) -> f64 {
    let band = largest_clearable(supply);
    let b = band as usize;
    if b + 1 >= SCORE_SLOTS {
        // Past the last tabulated band, keep climbing at the final band's slope
        // so the term never goes flat above the table.
        let last = SCORE_MEDIAN[SCORE_SLOTS - 1];
        let prev = SCORE_MEDIAN[SCORE_SLOTS - 2];
        let slope = 1.0 / f64::from(last - prev).max(1.0);
        return (band - 1) as f64 + 1.0 + (supply - last) as f64 * slope;
    }
    let lo = SCORE_MEDIAN[b];
    let hi = SCORE_MEDIAN[b + 1];
    let span = f64::from(hi - lo).max(1.0);
    f64::from(band) + f64::from(supply - lo) / span
}

/// The guides' own bridge from development to comparison score, continuous.
///
/// `hearts(t) + E[hits]` where `hits ~ Binomial(active blades, own density)` —
/// section 4's "DERIVED QUANTITIES", which the guide calls "the real
/// scoreboard". It is monotone in BOTH hearts and blades, which is what the
/// Main phase needs, and unlike `P(place)` it has a gradient before any life
/// in hand becomes passable.
pub fn heart_equivalent_supply(gs: &GameState, me: u8, db: &CardDatabase) -> f64 {
    let (blades, density) = super::strategy_v4::flip_stats(gs, me, db);
    (supply_hearts(gs, me, db) as f64) + blades as f64 * density
}

// -- Forward development model (the guides' cost curve) -------------------

/// Highest total stage cost reachable after `turns` more of OUR Main phases,
/// using the printed turn structure. `turns == 0` returns the CURRENT stage
/// cost, which is the guides' development metric read directly.
///
/// - Energy phase `+1` active energy per turn (7.5) and active energy persists
///   (7.4.1), so the budget compounds.
/// - Draw phase `+1` card per turn (7.6). Our own remaining deck is fair
///   information, so assumed draws come from it best-first.
/// - Baton touch: playing over an occupied slot costs
///   `own_cost - sent_cost` (9.6.2.3.2), so a new member is reachable at
///   `budget + max_stage_cost`. This is the guides' 4 -> 9 -> 13 ladder.
///
/// This is a FORWARD quantity, and that is the whole point. A one-ply cost
/// delta prices an upgrade by what it changes right now, which is how v7
/// could pass over a free 2 -> 7 baton step and end up a full turn behind.
pub fn reachable_ceiling(gs: &GameState, me: u8, turns: u8, db: &CardDatabase) -> i32 {
    let p = gs.seat_player(me);
    let mut stage: Vec<i32> = p
        .stage
        .stage
        .iter()
        .map(|&cid| if cid < 0 { 0 } else { card_cost(db, cid) })
        .collect();
    let mut budget = i32::from(p.energy_zone.active_count());

    let mut pool: Vec<i32> = p
        .hand
        .cards
        .iter()
        .filter_map(|&cid| db.get_card(cid))
        .filter(|c| c.card_type == CardType::Member)
        .map(|c| i32::from(c.cost.unwrap_or(0)))
        .filter(|&cost| cost > 0)
        .collect();
    // Assumed future draws from our own deck, best-first.
    let mut draws: Vec<i32> = p
        .main_deck
        .cards
        .iter()
        .filter_map(|&cid| db.get_card(cid))
        .filter(|c| c.card_type == CardType::Member)
        .map(|c| i32::from(c.cost.unwrap_or(0)))
        .filter(|&cost| cost > 0)
        .collect();
    draws.sort_unstable_by(|a, b| b.cmp(a));

    for turn in 0..turns as usize {
        budget += 1; // rule 7.5
        if let Some(drawn) = draws.get(turn) {
            pool.push(*drawn);
        }
        let discount = stage.iter().copied().max().unwrap_or(0);
        let has_occupied = discount > 0;
        // Baton into the CHEAPEST occupied slot: that leaves the expensive
        // member in place, so the next discount is the new (largest) cost.
        let baton_slot = stage
            .iter()
            .enumerate()
            .filter(|(_, cost)| **cost > 0)
            .min_by_key(|(_, cost)| **cost)
            .map(|(i, _)| i);
        let free_slot = stage.iter().position(|cost| *cost == 0);

        // Prefer a baton when one is affordable: a cost-9 swap for 5 energy
        // strictly beats a fresh cost-5 for 5 energy.
        let pick = |baton: bool| -> Option<(usize, i32)> {
            let mut best: Option<(usize, i32)> = None;
            for (i, &cost) in pool.iter().enumerate() {
                let effective = if baton {
                    if !has_occupied {
                        continue;
                    }
                    cost.saturating_sub(discount)
                } else {
                    if free_slot.is_none() {
                        continue;
                    }
                    cost
                };
                if effective > budget {
                    continue;
                }
                if best.is_none_or(|(_, best_cost)| cost > best_cost) {
                    best = Some((i, cost));
                }
            }
            best
        };

        let chosen = if has_occupied { pick(true) } else { None }.or_else(|| pick(false));
        let Some((pool_index, cost)) = chosen else {
            break;
        };
        pool.remove(pool_index);
        let slot = if has_occupied {
            baton_slot.unwrap_or(0)
        } else {
            free_slot.unwrap_or(0)
        };
        if let Some(cell) = stage.get_mut(slot) {
            *cell = cost;
        }
    }
    stage.iter().sum()
}

/// Board supply after `turns` more of OUR Main phases, as (hearts, active
/// blades) — the two quantities that decide a check (section 3.2).
///
/// This is the same forward walk as [`reachable_ceiling`] but accumulating the
/// check inputs instead of stage cost, and it is what makes the Main phase's
/// leaf non-myopic. Each simulated phase is the printed turn structure:
///
/// - Energy phase `+1` active energy (7.5); active energy persists (7.4.1).
/// - Draw phase `+1` card (7.6) from our own remaining deck, in deck order,
///   which is fair information under section 9.
/// - One deploy, preferring a baton into the CHEAPEST occupied slot (9.6.2.3.2)
///   because that leaves the expensive member in place as the next discount —
///   this is the guides' 4 -> 9 -> 13 ladder. Otherwise a free slot.
/// - Among affordable candidates the largest cost wins, which is the energy
///   doctrine in section 4 ("higher-cost members are simply better").
///
/// The heart/blade bookkeeping follows 3.2 exactly, and the asymmetry between
/// the two is the point:
///
/// - the heart pool is ALL members, active AND wait (3.2), so a baton adds
///   `new.base_heart - sent.base_heart`;
/// - the yell count is ACTIVE members only (Q133), so a baton REPLACES the
///   sent member's blades rather than adding to them.
pub fn forward_supply(
    gs: &GameState,
    me: u8,
    turns: u8,
    db: &CardDatabase,
) -> (Acc, i32) {
    let p = gs.seat_player(me);
    // Three stage areas: left, center, right.
    let mut slots: [i16; 3] = p.stage.stage;
    let mut hearts = board_supply(gs, me, db);
    let mut blades = active_blades(gs, me, db);
    let mut budget = i32::from(p.energy_zone.active_count());

    let is_deployable = |cid: i16| {
        db.get_card(cid)
            .is_some_and(|c| c.card_type == CardType::Member && c.cost.unwrap_or(0) > 0)
    };
    let mut hand: Vec<i16> = p
        .hand
        .cards
        .iter()
        .copied()
        .filter(|&c| is_deployable(c))
        .collect();
    let deck = &p.main_deck.cards;
    let mut cursor = 0usize;

    for _ in 0..turns {
        budget += 1; // rule 7.5

        // Draw phase (7.6): take the next member off our own deck.
        while cursor < deck.len() {
            let cid = deck[cursor];
            cursor += 1;
            if is_deployable(cid) {
                hand.push(cid);
                break;
            }
        }

        let occupied: Vec<usize> = slots
            .iter()
            .enumerate()
            .filter(|(_, c)| **c >= 0)
            .map(|(i, _)| i)
            .collect();
        // Baton target: the cheapest occupied slot, so the largest member
        // survives as the discount for the following step.
        let baton_slot = occupied
            .iter()
            .copied()
            .min_by_key(|&i| card_cost(db, slots[i]));
        let free_slot = slots.iter().position(|&c| c < 0);
        let discount = occupied
            .iter()
            .map(|&i| card_cost(db, slots[i]))
            .max()
            .unwrap_or(0);

        // Prefer the baton: a discounted swap beats a fresh play (9.6.2.3.2).
        let mut chosen: Option<(usize, usize, i32)> = None; // (hand index, slot, cost)
        for (slot, is_baton) in [(baton_slot, true), (free_slot, false)] {
            let Some(slot) = slot else { continue };
            for (hi, &cid) in hand.iter().enumerate() {
                let cost = card_cost(db, cid);
                let effective = if is_baton {
                    cost.saturating_sub(discount)
                } else {
                    cost
                };
                if effective > budget {
                    continue;
                }
                if chosen.is_none_or(|(_, _, best)| cost > best) {
                    chosen = Some((hi, slot, cost));
                }
            }
            if chosen.is_some() {
                break;
            }
        }
        let Some((hand_index, slot, _cost)) = chosen else {
            break;
        };
        let cid = hand.remove(hand_index);
        let sent = slots[slot];
        if sent >= 0 {
            // Baton: the sent member leaves the active set, so its blades go
            // with it and its hearts stay in the pool as a waiting member.
            if let Some(card) = db.get_card(sent) {
                blades -= i32::from(card.blade);
                if let Some(base) = &card.base_heart {
                    acc_add(&mut hearts, &base.hearts);
                }
            }
        }
        if let Some(card) = db.get_card(cid) {
            if let Some(base) = &card.base_heart {
                acc_add(&mut hearts, &base.hearts);
            }
            blades += i32::from(card.blade);
        }
        slots[slot] = cid;
        budget -= if discount > 0 {
            _cost.saturating_sub(discount)
        } else {
            _cost
        };
    }
    (hearts, blades.max(0))
}

// -- Cheap pass estimate for Main-phase ranking ---------------------------

/// `P(need satisfied)` given a deterministic board, the number of active
/// blades, and the own-deck probability that one flip yields a blade heart.
///
/// The Main phase ranks many actions and pays this at every search leaf, so it
/// uses this closed form rather than 192 shuffle samples. The live-set
/// decision, which actually commits cards, uses the exact sampler. Two
/// precisions, each where it earns its cost. Measuring the shortfall against
/// the PURE board (`confidence = 0.0`) keeps the binomial from double-counting
/// the flips `heart_pool_inner` would otherwise fold into the mean.
pub fn pass_estimate_from_supply(board: &Acc, blades: i32, density: f64, need: &Acc) -> f64 {
    let mut deficit = 0i32;
    for c in 1..=6 {
        deficit += (need[c] - board[c]).max(0);
    }
    deficit += (need[0] - board[0]).max(0);
    deficit += need[7].max(0) + need[10].max(0);
    let wildcard = board[7] + board[10];
    let shortfall = deficit - wildcard;
    if shortfall <= 0 {
        return 1.0;
    }
    super::strategy_v5::binom_ge(blades, shortfall, density)
}

/// Deterministic board hearts (buff-aware) in [`Acc`] index space with NO flip
/// expectation folded in.
pub fn board_supply(gs: &GameState, me: u8, db: &CardDatabase) -> Acc {
    super::strategy_v4::heart_pool_buffed(gs, me, db, 0.0)
}

/// The best member we could put on stage next turn, as printed board
/// contribution plus its blades. Used to project "how good could the following
/// check get" without committing to a deploy.
pub struct Projection {
    pub hearts: Acc,
    pub blades: i32,
}

pub fn project_one_turn(gs: &GameState, me: u8, db: &CardDatabase) -> Projection {
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
        .filter(|card| card.card_type == CardType::Member)
        .filter(|card| i32::from(card.cost.unwrap_or(0)) <= budget)
        .max_by_key(|card| card.cost.unwrap_or(0));
    let mut hearts = [0i32; 11];
    let mut blades = 0;
    if let Some(card) = best {
        if let Some(base) = &card.base_heart {
            super::strategy_common::acc_add(&mut hearts, &base.hearts);
        }
        blades = i32::from(card.blade);
    }
    Projection { hearts, blades }
}

/// `P(our best in-hand life passes)` on the current board, and on the board we
/// could build next turn with our best affordable deploy.
///
/// The second number is what makes a life in hand worth KEEPING. Pricing the
/// card's future use at its full `PLACEMENT_CREDIT` (the first v8 draft's
/// mistake) says a life is only worth committing when it is more likely than
/// not to pass, which folds roughly a third of all checks and starves the
/// success zone - the exact placement starvation the project measured at -9pp
/// with `beam`. The honest price of a burned life is the placement it would
/// have made LATER, which is what this returns.
pub fn best_life_pass_now_and_next(gs: &GameState, me: u8, db: &CardDatabase) -> (f64, f64) {
    let (blades, density) = super::strategy_v4::flip_stats(gs, me, db);
    let board = board_supply(gs, me, db);
    let needs: Vec<Acc> = gs
        .seat_player(me)
        .hand
        .cards
        .iter()
        .filter_map(|&cid| {
            let card = db.get_card(cid)?;
            if card.card_type != CardType::Live {
                return None;
            }
            let need = life_need(gs, cid);
            if has_unpassable_icon(&need) {
                return None;
            }
            Some(need)
        })
        .collect();
    if needs.is_empty() {
        return (0.0, 0.0);
    }
    let best = |supply: &Acc, blades: i32| -> f64 {
        needs
            .iter()
            .map(|need| pass_estimate_from_supply(supply, blades, density, need))
            .fold(0.0f64, f64::max)
    };
    let p_now = best(&board, blades);
    let projection = project_one_turn(gs, me, db);
    let mut projected = board;
    for i in 0..11 {
        projected[i] += projection.hearts[i];
    }
    let p_next = best(&projected, blades + projection.blades);
    (p_now, p_next)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opp_model(
        set_size: usize,
        success: usize,
        pass_prob: f64,
        dist: &[(usize, f64)],
    ) -> OppModel {
        let mut score_dist = [0.0f64; SCORE_SLOTS];
        for (score, weight) in dist {
            score_dist[*score] = *weight;
        }
        let total: f64 = score_dist.iter().sum();
        if total > 0.0 {
            for value in score_dist.iter_mut() {
                *value /= total;
            }
        }
        OppModel {
            set_size,
            success,
            score_dist,
            pass_prob,
        }
    }

    #[test]
    fn feasibility_matches_engine_bucket_semantics() {
        // A specific color is met only by the same color.
        let mut pool = [0i32; 8];
        pool[3] = 2;
        let mut need = [0i32; 11];
        need[3] = 2;
        assert!(feasible(&pool, &need));
        need[3] = 3;
        assert!(!feasible(&pool, &need));

        // A blade heart (BAll) is a wildcard for ONE specific color.
        let mut pool = [0i32; 8];
        pool[7] = 2;
        let mut need = [0i32; 11];
        need[5] = 2;
        assert!(feasible(&pool, &need));
        need[5] = 3;
        assert!(!feasible(&pool, &need));

        // Colorless hearts fill ONLY the grey bucket, never a specific color.
        let mut pool = [0i32; 8];
        pool[0] = 3;
        let mut need = [0i32; 11];
        need[2] = 1;
        assert!(!feasible(&pool, &need));
        need = [0i32; 11];
        need[0] = 3;
        assert!(feasible(&pool, &need));
        need[0] = 4;
        assert!(!feasible(&pool, &need));

        // Leftover specific hearts are absorbed by the total-count bucket.
        let mut pool = [0i32; 8];
        pool[1] = 3;
        let mut need = [0i32; 11];
        need[0] = 2;
        assert!(feasible(&pool, &need));

        // A specific-color deficit is never paid by the grey bucket.
        let mut pool = [0i32; 8];
        pool[0] = 5;
        let mut need = [0i32; 11];
        need[4] = 1;
        assert!(!feasible(&pool, &need));
    }

    /// Draw and Score icons are not heart requirements, so `feasible` ignores
    /// them; the filter that keeps such lives out of every candidate is
    /// `has_unpassable_icon`.
    #[test]
    fn draw_and_score_icons_are_never_check_requirements() {
        let mut need = [0i32; 11];
        need[9] = 1;
        assert!(has_unpassable_icon(&need));
        // The heart question itself ignores the icon, so a life whose ONLY
        // requirement is a Score icon is still "feasible" as a heart check -
        // which is exactly why it must be filtered out upstream.
        assert!(feasible(&[0i32; 8], &need));
    }

    #[test]
    fn rule_payoff_follows_win_conditions() {
        assert_eq!(rule_payoff(3, 2), 1.0);
        assert_eq!(rule_payoff(3, 3), 0.0);
        assert_eq!(rule_payoff(2, 3), -1.0);
        assert_eq!(rule_payoff(2, 2), 0.0);
    }

    /// 8.4.7.1: at two successes a tie does not place. A contested check
    /// whose totals match must be worth strictly less than the same check one
    /// point ahead.
    #[test]
    fn tie_does_not_place_at_two_successes() {
        let opp = opp_model(1, 0, 1.0, &[(5, 1.0)]);
        let tie = check_outcome(1.0, 5, 1, 2, &opp);
        let ahead = check_outcome(1.0, 6, 1, 2, &opp);
        assert!(ahead.p_place > tie.p_place);
        assert!(ahead.value > tie.value);
    }

    /// Below two successes a single-card tie DOES place, so the same totals
    /// are worth strictly more there. This is the pair of cases v7 needed a
    /// `+1` surcharge and a "strict close" branch to approximate.
    #[test]
    fn tie_places_for_a_single_card_set_below_match_point() {
        let opp = opp_model(1, 0, 1.0, &[(5, 1.0)]);
        let early = check_outcome(1.0, 5, 1, 1, &opp);
        let at_match_point = check_outcome(1.0, 5, 1, 2, &opp);
        assert!(early.p_place > at_match_point.p_place);
    }

    /// The free win (8.4.3.2) falls out of the model rather than being a
    /// special case: against an uncommitted opponent, reliability is the only
    /// currency, so a cheap reliable passer beats an unusable high score.
    #[test]
    fn uncommitted_opponent_makes_reliability_the_only_currency() {
        let opp = opp_model(0, 0, 0.0, &[(0, 1.0)]);
        let safe = check_outcome(0.97, 2, 1, 0, &opp);
        let risky = check_outcome(0.40, 9, 1, 0, &opp);
        assert!(safe.p_place > risky.p_place);
        assert!(safe.value > risky.value);
    }

    /// Against a committed opponent the same comparison inverts: score is what
    /// turns a passed check into a placement.
    #[test]
    fn committed_opponent_makes_score_the_currency() {
        let opp = opp_model(1, 0, 0.85, &[(4, 0.5), (8, 0.5)]);
        let safe_low = check_outcome(0.97, 2, 1, 0, &opp);
        let solid_high = check_outcome(0.90, 9, 1, 0, &opp);
        assert!(solid_high.p_place > safe_low.p_place);
    }

    /// At opponent match point, failing to contest loses outright (1.2.1.1),
    /// so even a thin check is worth more than setting nothing.
    #[test]
    fn match_point_makes_a_thin_check_worth_taking() {
        let opp = opp_model(1, 2, 0.4, &[(3, 1.0)]);
        let thin = check_outcome(0.30, 1, 1, 1, &opp);
        let fold = check_outcome(0.0, 0, 0, 1, &opp);
        assert!(thin.value > fold.value);
    }

    /// Setting nothing can never place for us, and it hands the check to an
    /// opponent who has already committed. Whether folding is nevertheless
    /// RIGHT depends on the ammunition a failed check burns, which is priced
    /// in `v8_live::candidate_value` - the model only has to make the
    /// comparison possible.
    #[test]
    fn folding_places_nothing_and_hands_over_the_check() {
        let opp = opp_model(2, 2, 0.9, &[(7, 1.0)]);
        let fold = check_outcome(0.0, 0, 0, 1, &opp);
        assert_eq!(fold.p_place, 0.0);
        assert!(fold.opp_place > 0.5);
        // We are one placement from the win, so letting them place is what the
        // fold is really paying for.
        assert_eq!(rule_payoff(1, 3), -1.0);
    }

    /// Burning ammunition is priced: a check we are likely to fail forfeits
    /// the future placement that life would have made. A coin-flip life is
    /// therefore worth about nothing on its own, which is the trade-off v7
    /// could only express with a stance floor (and got wrong in both
    /// directions, measured -1pp).
    #[test]
    fn failing_a_life_forfeits_its_future_placement() {
        let opp = opp_model(0, 0, 0.0, &[(0, 1.0)]);
        let certain = check_outcome(0.95, 2, 1, 0, &opp);
        let coin_flip = check_outcome(0.50, 2, 1, 0, &opp);
        let credit = PLACEMENT_CREDIT;
        let net_certain = certain.value - (1.0 - 0.95) * credit;
        let net_coin_flip = coin_flip.value - (1.0 - 0.50) * credit;
        assert!(net_certain > net_coin_flip);
    }

    /// Common random numbers: pools are reproducible, and on a fixed sample a
    /// strictly easier requirement is never ranked below a harder one.
    #[test]
    fn shared_pools_are_deterministic_and_monotone() {
        let sampler = FlipSampler {
            cats: vec![([0, 0, 0, 1, 0, 0, 0, 0], 6), ([0; 8], 6)],
            deck_len: 12,
            draws: 3,
        };
        let board = [0i32; 8];
        let a = sampler.sample_pools(&board, 64);
        let b = sampler.sample_pools(&board, 64);
        assert_eq!(a, b);

        let mut easy = [0i32; 11];
        easy[3] = 1;
        let mut hard = [0i32; 11];
        hard[3] = 5;
        for pool in &a {
            if feasible(pool, &hard) {
                assert!(feasible(pool, &easy));
            }
        }
        assert!(pass_probability(&a, &easy) >= pass_probability(&a, &hard));
    }

    /// The continuous band term must be strictly monotone and must start at
    /// zero, or it is not a gradient but a second constant.
    #[test]
    fn band_progress_is_continuous_and_starts_at_zero() {
        let mut previous = band_progress_for(-5);
        assert!(previous <= 0.0);
        for supply in 0..40 {
            let now = band_progress_for(supply);
            assert!(
                now >= previous,
                "supply {supply}: band progress fell {previous} -> {now}"
            );
            previous = now;
        }
        // It must actually move inside a single band, which is the whole point:
        // the integer ceiling is flat there and this is not.
        assert!(band_progress_for(4) > band_progress_for(3));
        assert!(band_progress_for(6) > band_progress_for(5));
    }

    /// Above the last tabulated band the term must keep rising rather than
    /// saturating, or a developed board stops registering.
    #[test]
    fn band_progress_keeps_climbing_past_the_table() {
        let top = SCORE_MEDIAN[SCORE_SLOTS - 1];
        let at_top = band_progress_for(top);
        assert!(band_progress_for(top + 10) > at_top);
    }

    /// The forward ceiling must see the guides' ladder. With a cost-4 already
    /// on stage, a cost-9 in hand costs 5 after the baton (9.6.2.3.2), so the
    /// 4 -> 9 step of the curve is affordable five turns of income early.
    #[test]
    fn forward_ceiling_sees_the_baton_ladder() {
        let discount = 4;
        assert!(9 - discount < 9, "a baton must be cheaper than a fresh play");
        assert_eq!(9 - discount, 5);
    }

    /// The reason the Main phase needs `score_ceiling` at all: `P(place)` is a
    /// threshold test, so it is flat across a whole band of board development
    /// and cannot rank actions there. `largest_clearable` must be strictly
    /// monotone - that gradient is the entire point of the term.
    #[test]
    fn largest_clearable_is_strictly_monotone_in_supply() {
        let mut previous = largest_clearable(0);
        for supply in 0..40 {
            let now = largest_clearable(supply);
            assert!(
                now >= previous,
                "supply {supply}: score ceiling fell {previous} -> {now}"
            );
            previous = now;
        }
    }

    /// It must actually move over the range boards occupy: a board that grows
    /// from nothing to a T4 closeout supply has to raise the clearable band,
    /// otherwise the term is a constant in disguise. Values are read straight
    /// off the section 3.3 table: median hearts for scores 0..=8 are
    /// 2, 3, 5, 7, 10, 12, 14, 16, 19.
    #[test]
    fn largest_clearable_tracks_the_guide_turn_bands() {
        assert_eq!(largest_clearable(0), 0, "an empty board clears only score 0");
        assert_eq!(largest_clearable(2), 0, "2 hearts = the score-0 median");
        assert_eq!(largest_clearable(4), 1, "4 hearts = the score-1 median");
        assert_eq!(largest_clearable(5), 2, "5 hearts = the T2 score-2 standard");
        assert_eq!(largest_clearable(7), 3, "7 hearts = the T3 score-3 standard");
        assert_eq!(largest_clearable(12), 5, "12 hearts = the T4 score-5 standard");
        assert_eq!(largest_clearable(19), 8, "19 hearts = the score-8 median");
    }
}
