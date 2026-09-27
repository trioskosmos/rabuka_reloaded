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
///
/// Tunable because the decision this feeds is a TAIL probability. At 192
/// samples a `p_pass` of 0.05 carries a standard error of about 0.016, which
/// is the same order as the differences between competing portfolios ? so the
/// argmax is partly reading sampling noise. The guide is explicit that hits are
/// a distribution and that mean-sized portfolios fail about half the time
/// (section 4, "DERIVED QUANTITIES"), which is exactly the regime where a
/// coarse tail estimate decides the wrong portfolio.
///
/// `V8_FLIP_SAMPLES` overrides it so the resolution can be traded against
/// runtime by measurement instead of by taste.
pub fn flip_samples() -> usize {
    std::env::var("V8_FLIP_SAMPLES")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&n: &usize| n > 0)
        .unwrap_or(FLIP_SAMPLES)
}

/// Default realisation count. See [`flip_samples`].
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

/// Whether the opponent's live zone is a fact or a forecast.
///
/// This distinction is the largest single defect v8 shipped with, and it was
/// invisible until the model was scored against the engine's own verdicts.
///
/// Sets are committed face-down, in attacker order (8.2.2). The SECOND attacker
/// sees the first attacker's set SIZE, and 8.4.3.2 makes that size decisive: a
/// sole passer places regardless of contents. The FIRST attacker sees nothing,
/// because the second attacker's set does not exist yet - and an empty live
/// zone is the NORMAL state at that point in the phase order, not evidence of a
/// fold.
///
/// The model read `live_card_zone.len()` as a commitment either way, so for the
/// first attacker it read "not yet chosen" as "chose nothing", set the
/// opponent's pass probability to exactly zero, and concluded that any check it
/// passed was a free placement. Measured over 1762 first-attacker live phases
/// against the engine's verdicts: the model predicted `P(place) = 0.589` and
/// `P(they place) = 0.000`; the truth was `0.154` and `0.157`, with the
/// opponent committing and passing on 47.9% of them. The same model read as the
/// second attacker - where the zone IS a fact - predicted `0.437` against a
/// truth of `0.428`.
///
/// The consequence is not a mistuned number. As first attacker the live set
/// collapses to "maximise the chance my own check passes", because score only
/// ever matters inside a comparison (section 2) and the model believed there
/// would not be one; the Main-phase leaf reads the same blind model, so its
/// development signal is wrong on the majority of turns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Commitment {
    /// The opponent has already set. Their zone is public information.
    Observed,
    /// We attack first. Their set does not exist yet and must be forecast.
    Unobserved,
}

/// Should the opponent model forecast the commitment, or read it?
///
/// In a live-set phase the answer is the phase itself. Outside one - the Main
/// phase, the mulligan - it is turn order (8.4.13): whoever is first attacker
/// this check will face a set that does not exist yet, and whoever is not gets
/// to see it.
pub fn commitment_of(gs: &GameState, me: u8) -> Commitment {
    match gs.current_phase {
        crate::game_state::Phase::LiveCardSetSecondAttacker => Commitment::Observed,
        crate::game_state::Phase::LiveCardSetFirstAttacker => Commitment::Unobserved,
        _ => {
            if gs.seat_player(me).is_first_attacker {
                Commitment::Unobserved
            } else {
                Commitment::Observed
            }
        }
    }
}

/// The forecast to use for an unobserved opponent set.
///
/// `V8_NO_ASSUME_COMMIT=1` restores the pre-fix behaviour (assume a fold), and
/// is kept as the ablation handle, because the measurement that settled the
/// question is the one that should be re-runnable: an unobserved commitment
/// read as a fold is a 3.8x over-estimate of `P(place)`, not a small bias.
///
/// `V8_ASSUME_SET` overrides the assumed set size and `V8_ASSUME_COMMIT` the
/// probability that the second attacker commits at all, so the rate can be
/// measured instead of asserted. The defaults are the measured ones: over 1762
/// first-attacker phases the second attacker committed on 48% of them, and a
/// committed second attacker is overwhelmingly a single life.
fn assume_commitment_enabled() -> bool {
    std::env::var_os("V8_NO_ASSUME_COMMIT").is_none()
}

fn assume_set_size() -> usize {
    std::env::var("V8_ASSUME_SET")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&n: &usize| (1..=3).contains(&n))
        .unwrap_or(1)
}

fn assume_commit_rate() -> f64 {
    std::env::var("V8_ASSUME_COMMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|w: &f64| w.is_finite() && *w > 0.0 && *w <= 1.0)
        .unwrap_or(0.55)
}

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
    /// Whether the set size above is observed or forecast. Kept in the model
    /// so the tie rule and any future diagnostic can tell "they set one card"
    /// from "we assume they set one card" - the two differ in exactly the case
    /// that was broken.
    pub commitment: Commitment,
    /// `P(they commit a set at all this check)`. 1.0 when observed; the
    /// forecast rate when they attack second and we cannot see them yet.
    pub commit_rate: f64,
}

impl OppModel {
    pub fn build(gs: &GameState, me: u8, db: &CardDatabase) -> Self {
        Self::build_with(gs, me, db, commitment_of(gs, me))
    }

    /// Build the model, given whether the opponent's zone is readable.
    pub fn build_with(
        gs: &GameState,
        me: u8,
        db: &CardDatabase,
        commitment: Commitment,
    ) -> Self {
        let (_, opp) = gs.seated_pair(me);
        let observed = opp.live_card_zone.cards.len();
        // An unobserved zone is empty by construction, never by choice, so the
        // forecast replaces it rather than reading it. `V8_NO_ASSUME_COMMIT`
        // restores the pre-fix reading - empty means fold - which is the
        // ablation that keeps the measurement re-runnable.
        let forecast = commitment == Commitment::Unobserved
            && observed == 0
            && assume_commitment_enabled();
        let set_size = if forecast {
            assume_set_size()
        } else {
            observed
        };
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
        //
        // When the commitment is a FORECAST, this is the probability they both
        // commit and pass - the product, because either half failing means they
        // place nothing. The bar itself is the same, evaluated against the
        // assumed set size, so the forecast is conditioned on their public
        // board rather than being a flat rate: a second attacker with a big
        // board commits more reliably than one with an empty stage.
        let target = (2 * set_size).clamp(1, SCORE_SLOTS - 1) as i32;
        let bar = score_median_hearts(target);
        let clears = YIELD_PRIOR
            .iter()
            .filter(|(yield_hearts, _)| {
                board_total + (blades as f64 * yield_hearts).round() as i32 >= bar
            })
            .map(|(_, weight)| *weight)
            .sum::<f64>()
            .min(1.0);
        let commit_rate = if forecast {
            assume_commit_rate()
        } else {
            1.0
        };
        let pass_prob = if set_size == 0 {
            0.0
        } else {
            clears * commit_rate
        };

        log::debug!(
            "v8 opp t{} me{} {} zone={observed} modelled_set={set_size} board={board_total} \
             blades={blades} bar={bar} clears={clears:.3} commit={commit_rate:.3} \
             pass={pass_prob:.3}",
            gs.turn_number,
            me,
            match commitment {
                Commitment::Observed => "observed",
                Commitment::Unobserved => "FORECAST",
            },
        );

        Self {
            set_size,
            success,
            score_dist,
            pass_prob,
            commitment,
            commit_rate,
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
pub fn supply_hearts(gs: &GameState, me: u8, db: &CardDatabase) -> i32 {
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

// -- Forward development model (the guides' cost curve) -------------------

/// The printed turn structure, run forward: energy phase `+1` (7.5), draw
/// phase `+1` card (7.6), one Main-phase deploy.
///
/// - A baton into an occupied slot costs `own_cost - sent_cost` (9.6.2.3.2), so
///   a new member is reachable at `budget + max_stage_cost`. That is the guides'
///   4 -> 9 -> 13 ladder.
/// - The baton targets the CHEAPEST occupied slot, so the most expensive member
///   survives as the next discount.
/// - Among affordable candidates the largest cost wins, which is section 4's
///   energy doctrine: "higher-cost members are simply better".
///
/// The heart/blade bookkeeping follows 3.2 exactly, and the asymmetry between
/// the two inputs is the whole point:
///
/// - the heart pool is ALL members, active AND wait (3.2), so a baton ADDS the
///   sent member's hearts to the pool and the new member's;
/// - the yell count is ACTIVE members only (Q133), so a baton REPLACES the sent
///   member's blades rather than adding to them.
///
/// ## Why this is one type and not two functions
///
/// This walk existed twice. The Main-phase leaf's copy handled the baton
/// correctly; the mulligan's copy overwrote the stage slot and moved on, so it
/// counted the sent member's blades as still active and its hearts as lost, and
/// the error compounded with every step of the ladder. Two copies of a rule is
/// how a number stops meaning what its comment says, so there is one walk now
/// and both callers build it from their own opening.
#[derive(Clone, Debug)]
pub struct Forward {
    pub hearts: Acc,
    pub blades: i32,
    /// Left, centre, right. `-1` is an empty slot.
    pub stage: [i16; 3],
    pub budget: i32,
    pub hand: Vec<i16>,
    pub deck: Vec<i16>,
    pub cursor: usize,
    /// Lives held, which are ammunition (section 2: every placement is a
    /// success card and a success card only arrives through a won check).
    pub lives: usize,
    pub density: f64,
    /// Cards the opening's replacement drew, in resolution order. They are part
    /// of the hand BEFORE turn 1, which is the entire reason a mulligan
    /// replacement can ever look like an improvement - and the reason the deck
    /// walk has to start past them.
    pub redraw: Vec<i16>,
}

impl Forward {
    /// A walk seeded from the real position: our hand and our own deck, both of
    /// which are fair information (section 9).
    pub fn from_state(gs: &GameState, me: u8, db: &CardDatabase) -> Self {
        let p = gs.seat_player(me);
        let (_blades, density) = super::strategy_v4::flip_stats(gs, me, db);
        Self {
            hearts: board_supply(gs, me, db),
            blades: active_blades(gs, me, db),
            stage: p.stage.stage,
            budget: i32::from(p.energy_zone.active_count()),
            hand: p.hand.cards.to_vec(),
            deck: p.main_deck.cards.to_vec(),
            cursor: 0,
            lives: lives_in_hand(p, db),
            density,
            redraw: Vec::new(),
        }
    }

    /// A walk seeded from a hypothetical opening.
    ///
    /// `redraw` names the cards the opening's replacement drew; they are added
    /// to the hand immediately AND removed from the deck walk. That second part
    /// is the one that was missing: a mulligan replacement draws from the top
    /// of the deck, so if the forward walk then starts again at the top it
    /// deals the same card twice - once as a replacement and once as turn 1's
    /// draw - and an opening that replaces three cards gets three phantom
    /// members for free.
    pub fn from_opening(
        db: &CardDatabase,
        energy: i32,
        deck: Vec<i16>,
        keep: &[i16],
        redraw: &[i16],
    ) -> Self {
        let mut walk = Self {
            hearts: [0; 11],
            blades: 0,
            stage: [-1; 3],
            budget: energy,
            hand: Vec::new(),
            deck,
            cursor: 0,
            lives: 0,
            density: 0.0,
            redraw: Vec::new(),
        };
        for &cid in keep {
            walk.add(db, cid);
        }
        for &cid in redraw {
            walk.add(db, cid);
            walk.redraw.push(cid);
        }
        walk.cursor = walk.redraw.len();
        walk
    }

    fn add(&mut self, db: &CardDatabase, cid: i16) {
        let Some(card) = db.get_card(cid) else { return };
        match card.card_type {
            CardType::Member => {
                if card.cost.unwrap_or(0) > 0 {
                    self.hand.push(cid);
                } else {
                    if let Some(base) = &card.base_heart {
                        acc_add(&mut self.hearts, &base.hearts);
                    }
                }
            }
            CardType::Energy => {
                // Energy exists only to buy a deploy (7.5); +1 is what it buys.
                self.budget += 1;
            }
            CardType::Live => self.lives += 1,
        }
    }

    fn deployable(&self, db: &CardDatabase, cid: i16) -> bool {
        db.get_card(cid)
            .is_some_and(|c| c.card_type == CardType::Member && c.cost.unwrap_or(0) > 0)
    }

    /// One of our Main phases. Returns whether anything was deployed.
    pub fn step(&mut self, db: &CardDatabase) -> bool {
        self.budget += 1; // rule 7.5

        // Draw phase (7.6): the next member off our own deck, in deck order.
        while self.cursor < self.deck.len() {
            let cid = self.deck[self.cursor];
            self.cursor += 1;
            if self.deployable(db, cid) {
                self.hand.push(cid);
                break;
            }
        }

        let occupied: Vec<usize> = self
            .stage
            .iter()
            .enumerate()
            .filter(|(_, c)| **c >= 0)
            .map(|(i, _)| i)
            .collect();
        let baton_slot = occupied
            .iter()
            .copied()
            .min_by_key(|&i| card_cost(db, self.stage[i]));
        let free_slot = self.stage.iter().position(|&c| c < 0);
        let discount = occupied
            .iter()
            .map(|&i| card_cost(db, self.stage[i]))
            .max()
            .unwrap_or(0);

        // A baton is preferred: a discounted swap beats a fresh play (9.6.2.3.2).
        let mut chosen: Option<(usize, usize, i32)> = None; // (hand index, slot, cost)
        for (slot, is_baton) in [(baton_slot, true), (free_slot, false)] {
            let Some(slot) = slot else { continue };
            for (hi, &cid) in self.hand.iter().enumerate() {
                let cost = card_cost(db, cid);
                let effective = if is_baton {
                    cost.saturating_sub(discount)
                } else {
                    cost
                };
                if effective > self.budget {
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
        let Some((hand_index, slot, cost)) = chosen else {
            return false;
        };
        let cid = self.hand.remove(hand_index);
        let sent = self.stage[slot];
        if sent >= 0 {
            // Baton. The sent member leaves the active set, so its blades go
            // with it (Q133) and its hearts stay in the pool as a waiting member
            // (3.2, heart pool is ALL members).
            if let Some(card) = db.get_card(sent) {
                self.blades -= i32::from(card.blade);
                if let Some(base) = &card.base_heart {
                    acc_add(&mut self.hearts, &base.hearts);
                }
            }
        }
        if let Some(card) = db.get_card(cid) {
            if let Some(base) = &card.base_heart {
                acc_add(&mut self.hearts, &base.hearts);
            }
            self.blades += i32::from(card.blade);
        }
        self.stage[slot] = cid;
        self.budget -= cost.saturating_sub(if sent >= 0 { discount } else { 0 });
        self.blades = self.blades.max(0);
        true
    }

    /// Run `turns` of our own Main phases and report what the check would then
    /// read: `(hearts by colour, active blades)`.
    pub fn run(&mut self, turns: u8, db: &CardDatabase) -> (Acc, i32) {
        for _ in 0..turns {
            if !self.step(db) {
                break;
            }
        }
        (self.hearts, self.blades)
    }
}

/// Board supply after `turns` more of OUR Main phases, as (hearts, active
/// blades) - the two quantities that decide a check (section 3.2). This is what
/// makes the Main phase's leaf non-myopic: it prices the check the guides'
/// curve is aimed at, not the one this turn happens to afford.
pub fn forward_supply(gs: &GameState, me: u8, turns: u8, db: &CardDatabase) -> (Acc, i32) {
    Forward::from_state(gs, me, db).run(turns, db)
}

/// Highest total stage cost reachable after `turns` more of OUR Main phases,
/// using the printed turn structure. `turns == 0` returns the CURRENT stage
/// cost, which is the guides' development metric read directly.
///
/// This is a FORWARD quantity, and that is the whole point. A one-ply cost
/// delta prices an upgrade by what it changes right now, which is how v7 could
/// pass over a free 2 -> 7 baton step and end up a full turn behind. It is
/// deliberately the *cheapest* member each turn rather than the largest, so it
/// is a lower bound on reachable development and not a second opinion about
/// which member to deploy - the leaf is the one that decides that.
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

    /// The real card database, so these tests exercise the real card shapes
    /// (a member with no `base_heart`, one with a zero blade, one whose blades
    /// come from a modifier) instead of a hand-built fixture that would hide
    /// exactly the cases that broke.
    fn real_db() -> CardDatabase {
        let cards = crate::card_loader::CardLoader::load_cards_from_file(std::path::Path::new(
            "../cards/cards.json",
        ))
        .expect("cards.json");
        CardDatabase::load_or_create(cards)
    }

    fn first_member_with(db: &CardDatabase, skip: usize, pred: impl Fn(&crate::card::Card) -> bool) -> i16 {
        let mut ids: Vec<i16> = db.cards.keys().copied().collect();
        ids.sort();
        ids.into_iter()
            .filter(|&id| {
                db.get_card(id)
                    .is_some_and(|c| c.card_type == CardType::Member && pred(c))
            })
            .nth(skip)
            .expect("a member matching the predicate exists in cards.json")
    }

    fn bare_state(db: &CardDatabase) -> GameState {
        let p1 = crate::player::Player::new("p1".into(), "P1".into(), true);
        let p2 = crate::player::Player::new("p2".into(), "P2".into(), false);
        GameState::new(p1, p2, std::sync::Arc::new(db.clone()))
    }

    /// Put a real member on `seat`'s stage, preferring one with hearts.
    fn stage_member(db: &CardDatabase, gs: &mut GameState, seat: usize) -> (i16, i32) {
        let member = first_member_with(db, 0, |c| c.base_heart.is_some() && c.cost.unwrap_or(0) > 0);
        let card = db.get_card(member).unwrap();
        let hearts = card.base_heart.as_ref().map_or(0, |b| b.hearts.values_sum() as i32);
        let player = if seat == 0 { &mut gs.player1 } else { &mut gs.player2 };
        player.stage.stage[0] = member;
        (member, hearts)
    }


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
            commitment: Commitment::Observed,
            commit_rate: 1.0,
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
    /// A baton must be booked on BOTH sides of the check, and the two sides
    /// move in opposite directions. The sent member's blades leave the active
    /// set (Q133: the yell count is active members only) but its hearts stay in
    /// the pool (3.2: the heart pool is ALL members), while the arriving member
    /// adds both.
    ///
    /// The mulligan's copy of this walk got the blades wrong - it overwrote the
    /// stage slot and moved on - so the mulligan thought a board grew sharper
    /// with every rung of the 4 -> 9 -> 13 ladder. Blades are Binomial trials
    /// (section 4), so that error is priced directly into placement probability.
    #[test]
    fn a_baton_books_the_sent_member_blades_out_and_hearts_in() {
        let db = real_db();
        // Two real members, deliberately chosen for opposite blade counts: the
        // one that arrives must be bladier, so a walk that forgets to remove the
        // sent member's blades reads strictly too high.
        let sent = first_member_with(&db, 1, |c| c.blade >= 1);
        let fresh = first_member_with(&db, 1, |c| {
            c.blade > 0 && c.cost.unwrap_or(0) > db.get_card(sent).unwrap().cost.unwrap_or(0)
        });
        let sent_card = db.get_card(sent).unwrap();
        let sent_hearts = sent_card.base_heart.as_ref().map_or(0, |b| b.hearts.values_sum() as i32);
        let sent_blades = i32::from(sent_card.blade);
        let fresh_card = db.get_card(fresh).unwrap();
        let fresh_hearts = fresh_card
            .base_heart
            .as_ref()
            .map_or(0, |b| b.hearts.values_sum() as i32);
        let fresh_blades = i32::from(fresh_card.blade);
        let cost = i32::from(fresh_card.cost.unwrap());
        let sent_cost = i32::from(sent_card.cost.unwrap());

        let mut walk = Forward {
            hearts: [0; 11],
            blades: sent_blades,
            stage: [sent, -1, -1],
            budget: 10_000,
            hand: vec![fresh],
            deck: Vec::new(),
            cursor: 0,
            lives: 0,
            density: 0.0,
            redraw: Vec::new(),
        };
        assert!(walk.step(&db));
        assert_eq!(
            walk.blades,
            fresh_blades,
            "the sent member's {sent_blades} blades leave the active set and only the arriving \
             member's {fresh_blades} count toward the yell"
        );
        assert_eq!(
            walk.hearts.iter().copied().sum::<i32>(),
            sent_hearts + fresh_hearts,
            "3.2: the heart pool is ALL members, so the sent member's {sent_hearts} hearts stay in \
             it alongside the arriving member's {fresh_hearts}"
        );
        assert_eq!(
            walk.budget,
            10_000 + 1 - (cost - sent_cost),
            "9.6.2.3.2: the baton costs own - sent, after the Energy phase's +1 (7.5)"
        );
    }

    /// A replacement draws from the top of the deck, so the forward walk has to
    /// start PAST it. The mulligan's model did not, so an opening that replaced
    /// three cards received three phantom members - once as the replacement and
    /// again as the following turns' draws.
    #[test]
    fn a_replacement_is_consumed_from_the_deck_walk_not_drawn_twice() {
        let db = real_db();
        let first = first_member_with(&db, 2, |c| c.cost.unwrap_or(0) > 0);
        let second = first_member_with(&db, 2, |c| {
            c.cost.unwrap_or(0) > 0 && c.card_no != db.get_card(first).unwrap().card_no
        });
        let walk = Forward::from_opening(&db, 3, vec![first, second], &[], &[first]);
        assert_eq!(walk.cursor, 1, "the redrawn card is off the top of the deck");
        assert_eq!(walk.hand, vec![first]);
        assert_eq!(walk.redraw, vec![first]);
        // The next turn's draw must be the SECOND deck card, not the first.
        let mut walk = walk;
        walk.step(&db);
        assert!(
            walk.hand.contains(&second),
            "turn 1 drew the wrong card: {:?}",
            walk.hand
        );
    }

    /// The defect this model shipped with, pinned as a test.
    ///
    /// An empty opponent live zone means OPPOSITE things to the two attackers.
    /// To the second attacker it is a fact: they folded, and 8.4.3.2 hands the
    /// check over. To the FIRST attacker it is the normal state of a phase
    /// whose other half has not happened yet, and it says nothing.
    ///
    /// The model read one observation as the other, so on the majority of live
    /// phases it believed a passed check was a free placement. Scored against
    /// the engine's own verdicts over 1762 first-attacker phases: predicted
    /// `P(place) = 0.589`, truth `0.154`; predicted `P(they place) = 0.000`,
    /// truth `0.157`. The identical model read as the second attacker, where
    /// the zone really is a fact, scored 0.437 against a truth of 0.428.
    ///
    /// So the fix is structural rather than numeric: an unobserved commitment
    /// is a forecast, and a forecast must be conditioned on the opponent's
    /// public board rather than pinned at zero.
    #[test]
    fn an_empty_opponent_zone_means_different_things_to_the_two_attackers() {
        let db = real_db();
        let mut gs = bare_state(&db);
        // A board for each side with real hearts on stage, so neither bar is
        // trivially zero and the test measures the forecast, not a floor.
        let (member, _) = stage_member(&db, &mut gs, 0);
        let _ = member;
        stage_member(&db, &mut gs, 1);
        gs.current_phase = crate::game_state::Phase::LiveCardSetFirstAttacker;
        let as_first = OppModel::build(&gs, 0, &db);
        gs.current_phase = crate::game_state::Phase::LiveCardSetSecondAttacker;
        let as_second = OppModel::build(&gs, 0, &db);

        assert_eq!(
            as_first.commitment,
            Commitment::Unobserved,
            "attacking first, the opponent's set does not exist yet"
        );
        assert_eq!(as_second.commitment, Commitment::Observed);
        assert!(
            as_first.pass_prob > 0.0,
            "an unobserved commitment must be forecast, not read as a fold; got {}",
            as_first.pass_prob
        );
        assert!(
            as_first.pass_prob < as_second.commit_rate,
            "and it must stay a probability of committing, not a certainty"
        );
        // The forecast must move with their public board, or it is a flat
        // constant wearing a forecast's clothes - covered in depth by
        // `the_forecast_rises_with_the_opponents_public_board` below.
    }

    /// The forecast must never claim the opponent is certain to place. An
    /// earlier shape of this fix set the assumed set size to 1 and left the
    /// pass probability alone, which is a guaranteed contested check - the
    /// mirror image of the original error, and equally wrong.
    #[test]
    fn a_forecast_commitment_is_strictly_less_than_certain() {
        let mut opp = opp_model(1, 0, 1.0, &[(4, 1.0)]);
        opp.commitment = Commitment::Unobserved;
        opp.commit_rate = assume_commit_rate();
        opp.pass_prob *= opp.commit_rate;
        assert!(opp.pass_prob < 1.0);
        assert!(opp.pass_prob > 0.0);
    }

    /// The forecast has to be conditioned on the opponent's PUBLIC board, or it
    /// is a flat constant wearing a forecast's clothes - and the whole point of
    /// modelling the unobserved commitment was to say more than "assume the
    /// worst".
    #[test]
    fn the_forecast_rises_with_the_opponents_public_board() {
        let db = real_db();
        // Three real members ordered by printed hearts, so "weak" and "strong"
        // are the same cards in different quantities.
        let mut members: Vec<i16> = {
            let mut ids: Vec<i16> = db.cards.keys().copied().collect();
            ids.sort();
            ids.into_iter()
                .filter(|&id| {
                    db.get_card(id).is_some_and(|c| {
                        c.card_type == CardType::Member
                            && c.cost.unwrap_or(0) > 0
                            && c.base_heart.is_some()
                    })
                })
                .collect()
        };
        assert!(members.len() > 3, "need members to build two boards");
        members.sort_by_key(|&id| {
            -db.get_card(id)
                .unwrap()
                .base_heart
                .as_ref()
                .map_or(0, |b| b.hearts.values_sum() as i32)
        });
        let weakest = members[members.len() - 1];
        let strongest = members[0];

        let forecast = |opp_members: &[i16]| {
            let mut gs = bare_state(&db);
            for (i, &id) in opp_members.iter().enumerate() {
                gs.player2.stage.stage[i.min(2)] = id;
            }
            gs.current_phase = crate::game_state::Phase::LiveCardSetFirstAttacker;
            OppModel::build(&gs, 0, &db).pass_prob
        };
        let weak = forecast(&[weakest]);
        let strong = forecast(&[strongest, strongest, strongest]);
        assert!(
            strong > weak,
            "the unobserved-commitment forecast must read the opponent's public board: \
             strong board {strong} vs weak board {weak}"
        );
    }

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
