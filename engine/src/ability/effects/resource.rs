//! `gain_resource` - the largest single action family in the engine.
//!
//! 160 of the 936 unique abilities in `cards/abilities.json`
//! (`docs/ABILITY_MATRIX.md`), 17% of all ability behaviour. Everything
//! that grants, splits, filters or applies a granted heart or blade is here,
//! along with the request structs that turn one effect into its blade and
//! heart target lists.
use crate::core::constants::CountCast;
use super::super::enums::{TargetPlayer, Zone};
use super::super::resolver::AbilityResolver;

use super::super::util;
use crate::ability::queue::ConditionalChoice;
use crate::card::{AbilityEffect};

use crate::game_state::GameState;
use smallvec::SmallVec;

use crate::{HashMap, HashSet};

#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use super::target::{position_limited_targets, same_name_targets};

/// Normalized resource kind for gain_resource effects. Card data spells the
/// resource in English or Japanese; matching raw strings ad hoc at every use
/// site is how EN/JA divergence bugs happen. Normalize once, match on this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResourceKind {
    Blade,
    Heart,
    /// Any other resource string (surplus_heart, etc.).
    Other,
}

impl ResourceKind {
    fn from_str(s: &str) -> Self {
        match s {
            "blade" => ResourceKind::Blade,
            "heart" => ResourceKind::Heart,
            _ => ResourceKind::Other,
        }
    }
}

/// Target sets resolved by [`AbilityResolver::resolve_gain_resource_targets`].
struct GainTargets {
    blade_targets: SmallVec<[i16; 8]>,
    heart_targets: Vec<i16>,
    heart_color_str: Option<String>,
    final_count: u8,
}

/// Everything [`AbilityResolver::resolve_gain_resource_targets`] needs to turn
/// one gain-resource effect into its blade and heart target lists.
///
/// These were 26 positional parameters. At that width every call site was an
/// unlabelled wall of arguments in which two adjacent `&[i16]` or two adjacent
/// `Option<SmallVec<..>>` were indistinguishable, and reordering one was a
/// silent bug rather than a compile error. Grouping them by where each value
/// comes from makes the shape of the operation readable and makes the grouping
/// itself the documentation.
#[derive(Clone)]
struct GainResourceRequest<'a> {
    // ── What is being gained, and how much ──
    kind: ResourceKind,
    /// The resource name, for diagnostics only.
    resource: &'a str,
    count: u8,
    /// Per-unit scaling, e.g. 「1枚につき」.
    per_unit: bool,
    per_unit_type_str: Option<&'a str>,

    // ── Who it goes to ──
    target: &'a str,
    /// The effect applies to every member, not a chosen set.
    is_all: bool,
    /// The target is the activating player, so no count truncation.
    is_self_target: bool,
    card_type_filter: Option<&'a str>,
    group_filter: Option<&'a str>,

    // ── Cards a preceding step already chose ──
    activating_card_id: Option<i16>,
    exclude_self_id: Option<i16>,
    /// Everything chosen so far in this action.
    all_selected: &'a [i16],
    /// What the immediately preceding step chose.
    selected_for_current: &'a [i16],
    /// Cards that have appeared this turn.
    appeared_ids: &'a HashSet<i16>,

    // ── Multipliers measured before this step ran ──
    last_energy: u8,
    last_discard_count: u8,
    recently_moved: &'a Option<SmallVec<[i16; 4]>>,
    entry_snapshot: &'a Option<SmallVec<[i16; 4]>>,
    preceding_moved: &'a Option<SmallVec<[i16; 4]>>,
    /// Cards THIS ability moved — see `this_ability_moved` in
    /// `execute_gain_resource`. The pool 「これにより…」 clauses count over.
    this_ability_moved: &'a Option<SmallVec<[i16; 4]>>,
    orientation_modifiers: &'a HashMap<i16, crate::core::game_modifiers::CardOrientation>,

    /// A heart colour fixed by an enclosing effect (e.g. an energy placed for
    /// heart05). Wins over the effect's own heart colour.
    single_fixed_heart: Option<String>,
}

/// What [`AbilityResolver::calculate_gain_multiplier`] needs to turn a
/// per-unit 「1枚につき」 into a concrete count.
///
/// Same reasoning as [`GainResourceRequest`]: these were 12 positional
/// arguments, four of which were adjacent `&Option<SmallVec<..>>` or `&str`
/// that were indistinguishable at the call site, so a mis-ordered pair was a
/// silent wrong count rather than a compile error. Grouped by origin.
#[derive(Clone)]
pub(crate) struct GainMultiplierRequest<'a> {
    // ── The effect's own shape ──
    per_unit: bool,
    base_count: u8,
    per_unit_type_str: Option<&'a str>,
    target: &'a str,

    // ── Counts measured before this step ran ──
    recently_moved: &'a Option<SmallVec<[i16; 4]>>,
    entry_snapshot: &'a Option<SmallVec<[i16; 4]>>,
    /// Cards THIS ability moved — the pool 「これにより…」 clauses count over.
    /// Distinct from `recently_moved`, which is a game-wide movement tracker only
    /// some placement paths populate.
    this_ability_moved: &'a Option<SmallVec<[i16; 4]>>,
    last_energy: u8,
    last_discard_count: u8,

    // ── Filters and modifiers in force ──
    orientation_modifiers: &'a HashMap<i16, crate::core::game_modifiers::CardOrientation>,
    filter: &'a crate::ability::util::CardFilter<'a>,
}

/// The boolean facts about one gain-or-loss step.
///
/// These travelled as a run of adjacent `bool` parameters, where a swapped pair
/// compiles cleanly and silently means the opposite — the worst kind of bug to
/// have, because nothing about the call site says which flag was which. Grouping
/// them makes the call site name each one, and the compiler checks the grouping.
#[derive(Clone, Copy, Default)]
pub(crate) struct ResourceFlags {
    /// The target is the activating player, so the count is not truncated.
    pub(crate) self_target: bool,
    /// Applies to every member rather than a chosen set.
    pub(crate) all: bool,
    /// Undone when the effect's duration ends.
    pub(crate) temporary: bool,
    /// A loss rather than a gain.
    pub(crate) negative: bool,
}

impl ResourceFlags {
    /// The flags for a blade gain: `all` and `temporary` are the only two that
    /// apply, and both vary, so there is no shorter spelling to reach for.
    pub(crate) fn for_blade(all: bool, temporary: bool) -> Self {
        Self {
            all,
            temporary,
            ..Self::default()
        }
    }
}

/// The part of `selected` chosen BEFORE the distinct-choice save point.
///
/// A saved distinct action excludes what was already chosen before the choice
/// so a card cannot be picked twice, while a later step in the same action
/// targets what the choice added (see [`selected_after_save`]). With no save
/// point, or one at or past the end, the whole selection is on one side of the
/// split.
fn selected_before_save(selected: &[i16], save_len: Option<u8>) -> Vec<i16> {
    match save_len {
        Some(n) if (n as usize) < selected.len() => selected[..n as usize].to_vec(),
        _ => selected.to_vec(),
    }
}

/// The part of `selected` chosen AFTER the distinct-choice save point.
fn selected_after_save(selected: &[i16], save_len: Option<u8>) -> Vec<i16> {
    match save_len {
        Some(n) if (n as usize) < selected.len() => selected[n as usize..].to_vec(),
        _ => selected.to_vec(),
    }
}

impl AbilityResolver {

    /// Grant a heart-color distribution to one card through the traced
    /// modifier API. Shared by every heart-granting path so sign handling
    /// and trace attribution cannot drift apart between copies.
    fn grant_heart_distribution(
        gs: &mut GameState,
        effect_text: &str,
        card_id: i16,
        distribution: &[(crate::card::HeartColor, u8)],
        is_negative: bool,
    ) {
        for &(color, dist_count) in distribution {
            let dist_amount = if is_negative {
                -i16::from(dist_count)
            } else {
                i16::from(dist_count)
            };
            gs.mods.add_heart_modifier_with_trace(
                card_id,
                color,
                dist_amount,
                &mut gs.ability_applications,
                gs.activating_card.unwrap_or(-1),
                effect_text,
            );
        }
    }

    /// Grant `amount` blades to one card through the traced modifier API.
    fn grant_blade(gs: &mut GameState, effect: &AbilityEffect, card_id: i16, amount: i16) {
        gs.mods.add_blade_modifier_with_trace(
            card_id,
            amount,
            &mut gs.ability_applications,
            gs.activating_card.unwrap_or(-1),
            &effect.text,
        );
    }

    #[allow(clippy::too_many_arguments)]
fn apply_heart_to_card(
        &mut self,
        gs: &mut GameState,
        card_id: i16,
        heart_distribution: &[(crate::card::HeartColor, u8)],
        is_negative: bool,
        is_temporary: bool,
        effect_data: &mut Option<crate::core::types::EffectData>,
        heart_color_str: &Option<String>,
        heart_to_add: i16,
        effect_text: &str,
    ) {
        Self::grant_heart_distribution(gs, effect_text, card_id, heart_distribution, is_negative);
        if is_temporary && effect_data.is_none() {
            if heart_distribution.len() > 1 {
                let items: Vec<crate::core::types::CardEffectItem> = heart_distribution
                    .iter()
                    .map(|&(c, dc)| {
                        let amount = if is_negative { -(dc as i16) } else { dc as i16 };
                        crate::core::types::CardEffectItem {
                            card_id,
                            amount,
                            color: Some(format!("{:?}", c)),
                        }
                    })
                    .collect();
                *effect_data = Some(crate::core::types::EffectData::MultiCard { items });
            } else {
                let color_name = heart_color_str.as_deref().unwrap_or("heart01");
                *effect_data = Some(Self::make_card_effect_data(
                    card_id,
                    heart_to_add,
                    Some(color_name),
                ));
            }
        }
    }

    /// The distinct heart colours among cards that recently reached the
    /// discard. Member cards carry their colours in `base_heart`; `need_heart`
    /// exists only on live cards and would miss every member.
    fn distinct_discarded_colors(
        card_db: &crate::card::CardDatabase,
        moved: Option<&SmallVec<[i16; 4]>>,
    ) -> SmallVec<[crate::card::HeartColor; 8]> {
        let mut distinct_colors: SmallVec<[crate::card::HeartColor; 8]> = SmallVec::new();
        let Some(moved) = moved else {
            return distinct_colors;
        };
        for &cid in moved {
            let Some(bh) = card_db.get_card(cid).and_then(|c| c.base_heart.as_ref()) else {
                continue;
            };
            for &(color, amt) in &bh.hearts {
                if amt > 0 && !distinct_colors.contains(&color) {
                    distinct_colors.push(color);
                }
            }
        }
        distinct_colors
    }

    pub(crate) fn handle_bp6_pattern(
        &self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<bool, String> {
        // bp6 pattern: "gain 1 heart per distinct color among discarded cards"
        // Detected by: resource=heart, per_unit=true, per_unit_type="discard", multiple_targets=true
        // For each distinct heart color present among recently_moved_cards, grant 1 heart of that color.
        let is_bp6 = effect.resource_any() == Some("heart")
            && effect.per_unit_any().unwrap_or(false)
            && Zone::from_str(effect.per_unit_type_any().unwrap_or(""))
                == Some(Zone::Discard)
            && effect.multiple_targets_any().unwrap_or(false);
        if !is_bp6 {
            return Ok(false);
        }

        let card_db = self.card_db();
        let duration = effect.duration_any();
        let is_temporary = duration.is_some() && duration != Some("permanent");
        let target = effect.target_name().to_string();
        let activating_card_id = gs.activating_card;

        let recently_moved = gs.recently_moved_cards.clone();
        let distinct_colors = Self::distinct_discarded_colors(&card_db, recently_moved.as_ref());

        log::debug!(
            "[BP6_HEART] distinct colors from {} discarded cards: {:?}",
            recently_moved.as_ref().map(|v| v.len()).unwrap_or(0),
            distinct_colors
        );

        let Some(card_id) = activating_card_id else {
            return Ok(true);
        };
        for color in &distinct_colors {
            gs.mods.add_heart_modifier_with_trace(
                card_id,
                *color,
                1,
                &mut gs.ability_applications,
                gs.activating_card.unwrap_or(-1),
                &effect.text,
            );
            gs.record_ability_application(
                card_id,
                effect.text.to_string(),
                "heart_bonus",
                card_id,
                Some(color.index().u8_count()),
                1,
            );
        }
        if is_temporary && !distinct_colors.is_empty() {
            let color_names: Vec<String> = distinct_colors
                .iter()
                .map(|c| format!("{:?}", c).to_lowercase())
                .collect();
            let items: Vec<crate::core::types::CardEffectItem> = distinct_colors
                .iter()
                .map(|color| crate::core::types::CardEffectItem {
                    card_id,
                    amount: 1,
                    color: Some(color.to_string()),
                })
                .collect();
            util::push_temporary_effect(
                gs,
                "gain_heart",
                duration,
                &target,
                &format!("Gain 1 heart of each color: {}", color_names.join(", ")),
                Some(crate::core::types::EffectData::MultiCard { items }),
            );
        }
        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
pub(crate) fn execute_gain_surplus_heart(
        &self,
        gs: &mut GameState,
        _effect: &AbilityEffect,
        target: &str,
        is_temporary: bool,
        duration: Option<&str>,
        sign: Option<&str>,
        is_all: bool,
    ) -> Result<(), String> {
        let player = gs.resolve_target_player(target);
        let is_p1 = player.id == gs.player1.id;
        let old = if sign == Some("negative") && is_all {
            // Compute surplus from snapshot total_hearts minus requirements,
            // so test modifications to the snapshot (e.g. total_hearts)
            // are reflected in last_surplus_loss_count.
            let pid = if is_p1 {
                &gs.player1.id
            } else {
                &gs.player2.id
            };
            let v = gs
                .performance_snapshots
                .iter()
                .find(|s| &s.player_id == pid)
                .map(|s| {
                    s.total_hearts.iter().sum::<u8>()
                        - s.lives.iter().flat_map(|l| l.required.iter()).sum::<u8>()
                })
                .unwrap_or(if is_p1 {
                    gs.self_live_surplus_count
                } else {
                    gs.opponent_live_surplus_count
                });
            gs.mods.last_surplus_loss_count = v;
            if is_p1 {
                gs.self_live_surplus_count = 0;
            } else {
                gs.opponent_live_surplus_count = 0;
            }
            Some(v)
        } else {
            None
        };
        if is_temporary {
            let desc = format!(
                "{} all surplus hearts",
                if sign == Some("negative") {
                    "Lose"
                } else {
                    "Gain"
                }
            );
            let effect_data = old.map(|v| crate::core::types::EffectData::SurplusHeart {
                is_p1,
                old_value: v,
            });
            util::push_temporary_effect(
                gs,
                "gain_surplus_heart",
                duration,
                target,
                &desc,
                effect_data,
            );
        }

        Ok(())
    }

    pub(crate) fn calculate_gain_multiplier(
        &self,
        gs: &GameState,
        effect: &AbilityEffect,
        req: &GainMultiplierRequest<'_>,
    ) -> u8 {
        let GainMultiplierRequest {
            per_unit,
            base_count,
            per_unit_type_str,
            target,
            recently_moved,
            entry_snapshot,
            last_energy,
            last_discard_count,
            this_ability_moved,
            orientation_modifiers,
            filter,
        } = req.clone();
        if !per_unit {
            return base_count;
        }
        let card_db = self.card_db();
        let player = gs.resolve_target_player(target);

        // If the effect has an explicit location, use that as the count zone
        // instead of the generic per_unit_type → zone mapping.
        let loc_binding = effect.location_any();
        let effective_per_unit_type = loc_binding.or(per_unit_type_str);
        // 「…のスコアの合計Nにつき」 measures a POINTS total, not a card tally, so
        // the same zone is walked but the cards' printed scores are summed. Same
        // zone resolution and same filter as the tally path, so both agree on which
        // cards are in scope.
        // 「このターン中に…アクティブにしていたメンバー」 is a per-turn HISTORY
        // clause, not a state filter. Counting members currently IN WAIT is a
        // different set of cards and is what this used to do, which is why
        // PL!-pb2-010-P＋ always granted nothing. The counted set is the
        // activation history, filtered to the same zone and group.
        let mut matching_count = if effect.activated_this_turn_any() == Some(true) {
            let card_db = self.card_db();
            gs.cards_activated_this_turn
                .iter()
                .copied()
                .filter(|cid| match effective_per_unit_type {
                    Some("member") | Some("members") | Some("人") | None => {
                        card_db.get_card(*cid).is_some_and(|c| c.is_member())
                    }
                    _ => true,
                })
                .filter(|cid| {
                    effect
                        .group_names_any()
                        .is_none_or(|gs_| gs_.is_empty())
                        || effect.group_names_any().is_some_and(|gs_| {
                            gs_.iter().any(|g| {
                                crate::ability::util::card_matches_group_str(
                                    &card_db,
                                    *cid,
                                    Some(g),
                                )
                            })
                        })
                })
                .count() as u8
        } else if effect.per_unit_aggregate_any().as_deref() == Some("score") {
            util::resolve_per_unit_score_sum(
                effective_per_unit_type,
                player,
                &card_db,
                filter,
                effect.state_any(),
                orientation_modifiers,
                None,
            )
        } else if effective_per_unit_type == Some("つ") {
            last_energy
        } else {
            util::resolve_per_unit_count(
                true,
                effective_per_unit_type,
                player,
                &card_db,
                filter,
                &[],
                effect.state_any(),
                orientation_modifiers,
                None,
            )
        };
        // For per_unit_type="discard": always use tracked move/cost counts,
        // never the full waitroom. Checks recently_moved first, then the
        // enqueue-time snapshot (trigger_moved_cards) as fallback.
        let tracked_moved = recently_moved.as_ref().or(entry_snapshot.as_ref());
        // 「これにより…}:\u3000 「これにより控え室に置いたカード1枚につき」 and its
        // siblings name the BATCH the preceding step just produced, and the parser
        // records that as `per_unit_source: "previous_moved_cards"`. Key off it
        // FIRST, because `per_unit_type` cannot carry it: it is a counting noun
        // (「枚」/「人」/「つ」), not a zone, and the branches below only recognise
        // `"discard"`, `"waitroom_card"` and `"energy_deck"`.
        //
        // PL!S-bp7-007-P 国木田花丸 is the case that shows the cost of the omission.
        // Its blade clause is 「これによりデッキに置かれたカード1枚につき、ブレードを
        // 得る」 with `per_unit_type: "枚"`, no `location` and no `card_type` filter,
        // so the count fell through to `resolve_per_unit_count("枚", …)` and
        // `per_unit_zone` mapped 「枚」 to `Zone::Hand`. The blades were counted from
        // the hand — a zone the clause never mentions — while the cards actually
        // placed earned nothing. With two members in hand and two placed it read as
        // correct by coincidence, which is why no test had caught it.
        //
        // Keyed on the field rather than on a new `per_unit_type` spelling because
        // the field is what the parser already emits for all three affected cards:
        // PL!S-bp7-007-P (枚), PL!N-bp7-008-P (枚) and PL!S-bp6-021-L (none).
        // 「これにより…}: the cards THIS ability moved, and the clause is
        // authoritative about them. An empty batch is ZERO, never a fall-through to
        // a zone-derived count: 「3枚まで」 is an upper bound, so when nothing is
        // placeable the set of 「これにより…置かれたカード」 is empty and the effect
        // earns nothing. Guarding this on the pool being present would hand the
        // clause back to `per_unit_type`, which is a counting noun that resolves to
        // the hand — the exact defect the branch exists to remove.
        if effect.per_unit_source_any() == Some("previous_moved_cards") {
            matching_count = util::resolve_discard_per_unit_count(
                this_ability_moved.as_ref(),
                last_discard_count,
                &card_db,
                filter,
            );
            log::debug!(
                "[GAIN_MULT] per_unit_source=previous_moved_cards over {} cards this \
                 ability moved -> {}",
                this_ability_moved.as_ref().map(|v| v.len()).unwrap_or(0),
                matching_count
            );
        } else if Zone::from_str(per_unit_type_str.unwrap_or("")) == Some(Zone::Discard) {
            let tm_len = tracked_moved.map(|v| v.len()).unwrap_or(0);
            log::debug!(
                "[DBG_GR] tracked_moved.len={} last_discard_count={}",
                tm_len,
                last_discard_count,
            );
            matching_count = util::resolve_discard_per_unit_count(
                tracked_moved,
                last_discard_count,
                &card_db,
                filter,
            );
            log::debug!("[DBG_GR] resolve result: matching_count={}", matching_count);
        } else if (Zone::from_str(per_unit_type_str.unwrap_or("")) == Some(Zone::Waitroom)
            || per_unit_type_str == Some("waitroom_card"))
            && (last_discard_count > 0 || recently_moved.is_some())
        {
            matching_count = util::resolve_discard_per_unit_count(
                recently_moved.as_ref(),
                last_discard_count,
                &card_db,
                filter,
            );
        } else if per_unit_type_str == Some("energy_deck") {
            // 「これによって置いたエネルギーカード1枚につき」 (PL!N-bp3-025-L Awakening
            // Promise). Cards placed in the energy deck are always energy cards, so the
            // count is the moved-card pool's length and `card_type` must not filter it —
            // the effect's own `card_type` may be about the RECIPIENT (a member), not the
            // cards being counted.
            //
            // `recently_moved_cards` is the right pool and NOT `entry_snapshot`: the
            // snapshot is taken at enqueue time and can hold trigger-setup cards this
            // ability never moved.
            if let Some(moved) = recently_moved.as_ref() {
                matching_count = moved.len().u8_count();
            } else {
                matching_count = 0;
            }
        }
        // `.max(1)`: `unwrap_or(1)` only covers an ABSENT field, so a decoded
        // `per_unit_count: 0` reached the division and panicked. Every other
        // per-unit site in the engine (ability/util.rs, effects/deck.rs) already
        // clamps; this was the odd one out.
        let per_unit_count_val = effect.per_unit_count_any().unwrap_or(1).max(1);
        let mut units = matching_count / per_unit_count_val;
        if effect.max.unwrap_or(false) {
            if let Some(cap) = effect.count_any() {
                units = units.min(cap);
            }
        }
        // Also cap by max_repeats (aliased as repeat_limit), used
        // when the parser sets it as the sole cap on per_unit effects
        // (e.g. "N枚までしか数えない" text constraints).
        if let Some(cap) = effect.repeat_limit_any() {
            units = units.min(cap);
        }
        let per_unit_base = if effect.max.unwrap_or(false) {
            1
        } else {
            effect
                .resource_icon_count_any()
                .unwrap_or(effect.count_or(1))
        };
        log::debug!(
            "[GAIN_MULT] matching={matching_count} per_unit_count_val={per_unit_count_val} units={units} base={per_unit_base}"
        );
        units * per_unit_base
    }

    fn gain_heart_colors_from_selected_card(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let card_db = self.card_db();
        // Read the debug flag once: it is a relaxed atomic load and this path
        // reads it on every traced mutation below.
        let debug =
            crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed);
        let target_ids: Vec<i16> = {
            let target_str = effect.target_name().to_string();
            let player = gs.resolve_target_player_mut(&target_str);
            player
                .stage
                .stage
                .iter()
                .cloned()
                .filter(|&tid| {
                    tid != -1
                        && crate::ability::util::card_matches_characters(
                            &card_db,
                            tid,
                            effect.characters_any().map(|v| &**v),
                        )
                })
                .collect()
        };
        if debug {
            log::debug!("[GR_SELECTED_CARD] entering branch. selected_cards={:?} target_ids={:?} gs.activating={:?}",
                self.selection.cards, target_ids, gs.activating_card);
        }
        if let Some(&selected_id) = self.selection.cards.first() {
            if let Some(selected_card) = card_db.get_card(selected_id) {
                if debug {
                    log::debug!("[GR_SELECTED_BH] selected_id={} has_base_heart={} hearts_count={}",
                        selected_id,
                        selected_card.base_heart.is_some(),
                        selected_card
                            .base_heart
                            .as_ref()
                            .map(|bh| bh.hearts.len())
                            .unwrap_or(0));
                }
                if let Some(ref base_heart) = selected_card.base_heart {
                    for &target_id in &target_ids {
                        for &color in base_heart.hearts.keys() {
                            if debug {
                                log::debug!("[GR_APPLY] target={} color={:?} before={}",
                                    target_id,
                                    color,
                                    gs.mods.get_heart_modifier(target_id, color));
                            }
                            gs.mods.add_heart_modifier_with_trace(
                                target_id,
                                color,
                                1,
                                &mut gs.ability_applications,
                                target_id,
                                &effect.text,
                            );
                            if debug {
                                log::debug!("[GR_APPLY] target={} color={:?} after={}",
                                    target_id,
                                    color,
                                    gs.mods.get_heart_modifier(target_id, color));
                            }
                            if effect.duration_any() == Some("live_end") {
                                let effect_data = crate::core::types::EffectData::SingleCard {
                                    card_id: target_id,
                                    amount: 1,
                                    color: Some(format!("{:?}", color)),
                                };
                                crate::ability::util::push_temporary_effect(
                                    gs,
                                    "gain_heart",
                                    Some("live_end"),
                                    "self",
                                    &format!("Gain +1 {:?} from selected card", color),
                                    Some(effect_data),
                                );
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn gain_heart_all_type(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let target_str = effect.target_name().to_string();
        // Capture values before mutable borrow of gs
        let triggering_member = gs
            .ability_queue
            .current_entry()
            .and_then(|e| e.triggering_member_id);
        let activating = gs.activating_card;
        let has_explicit_target = effect.target_any().is_some();
        let player = gs.resolve_target_player_mut(&target_str);
        let card_id = triggering_member.or_else(|| {
            if let Some(pos) = effect.position_any() {
                let pos_str = pos.get_position()?;
                let idx = crate::ability::util::stage_position_index(pos_str)?;
                if idx < player.stage.stage.len() && player.stage.stage[idx] != -1 {
                    Some(player.stage.stage[idx])
                } else {
                    None
                }
            } else if has_explicit_target {
                player.stage.stage.iter().find(|&&id| id != -1).copied()
            } else {
                activating
            }
        });
        if let Some(card_id) = card_id {
            let amount = i16::from(effect.count_or(1));
            gs.mods.add_heart_modifier_with_trace(
                card_id,
                crate::card::HeartColor::All,
                amount,
                &mut gs.ability_applications,
                card_id,
                &effect.text,
            );
            if effect.duration_any() == Some("live_end") {
                let effect_data = crate::core::types::EffectData::SingleCard {
                    card_id,
                    amount,
                    color: Some("all".to_string()),
                };
                crate::ability::util::push_temporary_effect(
                    gs,
                    "gain_heart",
                    Some("live_end"),
                    "self",
                    &format!("Gain {} all-heart", amount),
                    Some(effect_data),
                );
            }
        }
        Ok(())
    }

    pub fn execute_gain_resource(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
            log::debug!("[GR_ENTER] resource={:?} count={:?} target_count={:?} source={:?} card_type={:?} target={:?} exclude_self={:?} target_from_sel={:?}",
                effect.resource_any(), effect.count_any(), effect.target_count_any(), effect.source_any(), effect.card_type_any(), effect.target_any(), effect.exclude_self_any(), effect.target_from_selection_any());
        }
        // heart_colors_from_selected_card: gain 1 heart of each color
        // that the previously-selected card in the sequential has (base_heart).
        if effect.resource_any() == Some("heart")
            && effect
                .heart_colors_from_selected_card_any()
                .unwrap_or(false)
        {
            return self.gain_heart_colors_from_selected_card(gs, effect);
        }

        if effect.resource_any() == Some("heart")
            && effect.heart_type_any() == Some("all")
        {
            return self.gain_heart_all_type(gs, effect);
        }

        if self.handle_bp6_pattern(gs, effect)? {
            return Ok(());
        }
        let resource = effect.resource_any().unwrap_or("").to_string();
        // Normalize once — card data spells resources in EN or JA; ad-hoc
        // string matching scattered through this function is how the two
        // spellings drift apart.
        let kind = ResourceKind::from_str(&resource);
        let count = effect
            .resource_icon_count_any()
            .unwrap_or(effect.count_or(1));
        let target = effect.target_name().to_string();
        let duration = effect.duration_any();
        let is_temporary = duration.is_some() && duration != Some("permanent");
        let card_type_filter = effect.card_type_any().map(|s| s.to_string());
        let group_filter = effect.group_name().map(|s| s.to_string());
        let per_unit_type_str = effect.per_unit_type_any();
        let heart_selection = effect.heart_selection_any().unwrap_or(false);
        let per_unit = effect.per_unit_any().unwrap_or(false);
        let sign_binding = effect.sign_any();
        let sign = sign_binding;
        let activating_card_id = gs.activating_card;
        let card_db = self.card_db();
        let is_self_target = effect.is_self_target();
        let last_discard_count = gs.mods.last_cost_discard_count;
        let is_all = effect.all_any().unwrap_or(false)
            || (effect.source_any().is_none()
                && effect.card_type_any() == Some(&crate::card::CardType::Member)
                && matches!(
                    effect.target_name_player(),
                    Some(TargetPlayer::Self_) | Some(TargetPlayer::Opponent)
                )
                && !is_self_target
                && effect.exclude_self_any().is_none()
                && effect.target_count_any().is_none())
            // Also detect "all members" when the effect has no target_count limit
            // and targets "self"/"opponent" members (e.g. "自分のステージにいるメンバーは")
            || (effect.card_type_any() == Some(&crate::card::CardType::Member)
                && matches!(
                    effect.target_name_player(),
                    Some(TargetPlayer::Self_) | Some(TargetPlayer::Opponent)
                )
                && effect.target_count_any().is_none()
                && effect.distinct_any().is_none());

        if resource == "surplus_heart" {
            return self.execute_gain_surplus_heart(
                gs,
                effect,
                &target,
                is_temporary,
                duration,
                sign,
                is_all,
            );
        }

        // If a preceding select action already set the heart color in
        // conditional_choice, use it directly — skip any heart_colors prompt.
        // This handles both per_unit and non-per_unit gain_resource that
        // follow a select in a sequential effect.
        let existing_choice =
            gs.ability_queue
                .current_entry()
                .and_then(|e| match &e.conditional_choice {
                    Some(ConditionalChoice::Str(s)) => Some(s.clone()),
                    _ => None,
                });
        let single_fixed_heart = if let Some(chosen) = existing_choice {
            Some(chosen)
        } else if per_unit && kind == ResourceKind::Heart {
            gs.ability_queue
                .current_entry()
                .and_then(|e| match &e.conditional_choice {
                    Some(ConditionalChoice::Str(s)) => Some(s.clone()),
                    _ => None,
                })
        } else {
            let effective_heart_colors: Vec<String> = effect
                .heart_color_any()
                .map(|hc| vec![hc.to_string()])
                .unwrap_or_else(|| effect.heart_colors_any().to_vec());
            let result = self.resolve_gain_heart_color(
                gs,
                effect,
                resource.as_str(),
                count,
                &effective_heart_colors,
                heart_selection,
            )?;
            if self.awaiting.choice.is_some() {
                return Ok(());
            }
            result.or_else(|| {
                if kind == ResourceKind::Heart {
                    gs.ability_queue
                        .current_entry()
                        .and_then(|e| match &e.conditional_choice {
                            Some(ConditionalChoice::Str(s)) => Some(s.clone()),
                            _ => None,
                        })
                        .or_else(|| {
                            // Fallback: handle_heart_color_selection stores the color in
                            // prohibition_effects as "selected_heart_color:{color}" before
                            // clear_choice_meta resets conditional_choice.
                            gs.prohibition_effects
                                .iter()
                                .find_map(|e| e.strip_prefix("selected_heart_color:"))
                                .map(|s| s.to_string())
                        })
                } else {
                    None
                }
            })
        };

        // Extract accumulated selected card IDs from resolver
        let all_selected: SmallVec<[i16; 8]> = self.selection.cards.iter().copied().collect();

        // Pre-filter selected_cards by current character/card_type to prevent
        // cross-character leakage in sequential (e.g. blade for char A leaks
        // into blade for char B).
        let selected_for_current: Vec<i16> = if !all_selected.is_empty() {
            if let Some(chars) = effect.characters_any() {
                all_selected
                    .iter()
                    .filter(|&&cid| {
                        crate::ability::util::card_matches_characters(&card_db, cid, Some(chars))
                    })
                    .copied()
                    .collect()
            } else {
                all_selected.iter().copied().collect()
            }
        } else {
            Vec::new()
        };

        let recently_moved = gs.recently_moved_cards.clone();
        let entry_snapshot = gs.entry_trigger_moved_cards();
        let last_cost_moved = gs.mods.last_cost_moved_card_ids.clone();
        // 「これにより…}: the cards THIS ability moved, read from the resolver's own
        // record rather than from the global movement tracker.
        //
        // `selection.moved_cards` is where every placement path deposits what it
        // moved - the plain move path extends it with the batch, and the
        // 好きな順番 / any-order deck path extends it once the player finishes
        // picking the order (choice.rs). It is deliberately NOT reset between an
        // ability's steps, so for 「…置いて、…これにより置いたカード1枚につき」 the
        // second step sees exactly the first step's cards.
        //
        // The global `gs.recently_moved_cards` is the wrong source: it is populated
        // only by the paths that call `push_movement_event`, so for an any-order
        // deck placement it was `None` and the count fell back to `per_unit_type`,
        // which is a counting noun and resolved to the hand. PL!S-bp7-007-P 国木田
        // 花丸 credited a blade for every card in hand instead of the two 『Aqours』
        // members it had just placed.
        let this_ability_moved: Option<SmallVec<[i16; 4]>> =
            if self.selection.moved_cards.is_empty() {
                None
            } else {
                Some(self.selection.moved_cards.iter().copied().collect())
            };
        let preceding_moved: Option<SmallVec<[i16; 4]>> = entry_snapshot
            .clone()
            .or_else(|| {
                let rm = recently_moved.clone()?;
                if rm.is_empty() {
                    None
                } else {
                    Some(rm)
                }
            })
            .or_else(|| {
                if last_cost_moved.is_empty() {
                    None
                } else {
                    Some(last_cost_moved.clone())
                }
            });

        let exclude_self_id = if effect.exclude_self_any().unwrap_or(false) {
            gs.activating_card
        } else {
            None
        };

        let choice_created = self.try_create_target_selection_choice(
            gs,
            effect,
            kind,
            &resource,
            count,
            &target,
            is_self_target,
            per_unit,
            exclude_self_id,
            &preceding_moved,
            all_selected.as_slice(),
            &selected_for_current,
        )?;
        if choice_created {
            return Ok(());
        }

        let orientation_modifiers = gs.mods.orientation_modifiers.clone();
        let last_energy = gs.mods.last_cost_energy_count;
        // Issue 6: Pre-compute appeared/moved-this-turn sets before mutable borrow
        let appeared_ids: HashSet<i16> =
            if effect.timing_condition_any() == Some("appeared_this_turn") {
                let p = gs.resolve_target_player(&target);
                p.stage
                    .stage
                    .iter()
                    .filter(|&&cid| cid != -1 && gs.has_card_appeared_this_turn(cid))
                    .copied()
                    .collect()
            } else if effect.timing_condition_any() == Some("moved_this_turn") {
                let area_moved_ids: HashSet<i16> = gs
                    .turn_area_movements
                    .iter()
                    .map(|m| m.moved_card_id)
                    .collect();
                let p = gs.resolve_target_player(&target);
                p.stage
                    .stage
                    .iter()
                    .filter(|&&cid| {
                        if cid == -1 {
                            return false;
                        }
                        // Prefer turn_area_movements (precise area-move tracking),
                        // fall back to cards_moved_this_turn for backward compat.
                        if !area_moved_ids.is_empty() {
                            area_moved_ids.contains(&cid)
                        } else {
                            gs.cards_moved_this_turn.iter().any(|x| x == &cid)
                        }
                    })
                    .copied()
                    .collect()
            } else {
                HashSet::default()
            };
        let GainTargets {
            blade_targets,
            mut heart_targets,
            heart_color_str,
            final_count,
        } = self.resolve_gain_resource_targets(
            gs,
            effect,
            &GainResourceRequest {
                kind,
                resource: resource.as_str(),
                count,
                per_unit,
                per_unit_type_str,
                this_ability_moved: &this_ability_moved,
                target: &target,
                is_all,
                is_self_target,
                card_type_filter: card_type_filter.as_deref(),
                group_filter: group_filter.as_deref(),
                activating_card_id,
                exclude_self_id,
                all_selected: &all_selected,
                selected_for_current: &selected_for_current,
                appeared_ids: &appeared_ids,
                last_energy,
                last_discard_count,
                recently_moved: &recently_moved,
                entry_snapshot: &entry_snapshot,
                preceding_moved: &preceding_moved,
                orientation_modifiers: &orientation_modifiers,
                single_fixed_heart: single_fixed_heart.clone(),
            },
        );

        // Store selected card IDs when target_count/distinct is set
        // so the next sequential action can exclude these cards.
        // Only store when we have explicit selection limits to avoid
        // polluting all_selected for blanket effects like "both players gain blade".
        if effect.target_count_any().is_some() || effect.distinct_any().is_some() {
            let selected_targets: Vec<i16> = if kind == ResourceKind::Blade {
                blade_targets.to_vec()
            } else {
                heart_targets.clone()
            };
            for &cid in &selected_targets {
                if !self.selection.cards.contains(&cid) {
                    self.selection.cards.push(cid);
                }
            }
        }

        let mut effect_data: Option<crate::core::types::EffectData> = None;
        let is_negative = sign == Some("negative");
        let blades_to_add = if is_negative {
            -i16::from(final_count)
        } else {
            i16::from(final_count)
        };
        let heart_to_add = if is_negative {
            -i16::from(final_count)
        } else {
            i16::from(final_count)
        };
        let heart_color_val = crate::card::parse_heart_color(
            heart_color_str
                .as_deref()
                .unwrap_or(crate::ability::util::HEART_ALL_KEY),
        );

        // Build heart distribution: for fixed multi-color grants, distribute count
        // across all specified colors instead of using a single color.
        let heart_distribution: Vec<(crate::card::HeartColor, u8)> = if resource == "heart"
            && !heart_selection
            && !effect.heart_colors_any().is_empty()
            && (final_count as usize) >= effect.heart_colors_any().len()
        {
            let colors = effect.heart_colors_any();
            let per_color = crate::ability::util::heart_gain_per_entry(
                i32::from(final_count),
                colors,
            )
            .u8_count();
            colors
                .iter()
                .map(|c| (crate::card::parse_heart_color(c), per_color))
                .collect()
        } else {
            vec![(heart_color_val, final_count)]
        };

        if is_self_target {
            if let Some(card_id) = activating_card_id {
                if !gs
                    .resolve_target_player_mut(&target)
                    .stage
                    .stage
                    .contains(&card_id)
                {
                    return Err(
                        "Cannot use self_target on gain_resource: activating card not on stage"
                            .to_string(),
                    );
                }
                if kind == ResourceKind::Blade {
                    Self::grant_blade(gs, effect, card_id, blades_to_add);
                    if is_temporary {
                        effect_data =
                            Some(Self::make_card_effect_data(card_id, blades_to_add, None));
                    }
                }
                if kind == ResourceKind::Heart {
                    Self::grant_heart_distribution(
                        gs,
                        &effect.text,
                        card_id,
                        &heart_distribution,
                        is_negative,
                    );
                    if is_temporary && effect_data.is_none() {
                        let mut items: Vec<crate::core::types::CardEffectItem> = Vec::new();
                        for (color, color_amount) in &heart_distribution {
                            let color_name = format!("{:?}", color).to_lowercase();
                            items.push(crate::core::types::CardEffectItem {
                                card_id,
                                amount: *color_amount as i16,
                                color: Some(color_name),
                            });
                        }
                        effect_data = Some(crate::core::types::EffectData::MultiCard { items });
                    }
                }
                if is_temporary {
                    util::push_temporary_effect(
                        gs,
                        &format!("gain_{}", resource),
                        duration,
                        &target,
                        &format!("Gain {} {}", final_count, resource),
                        effect_data,
                    );
                }
                return Ok(());
            }
        }

        let blade_targets_save = blade_targets.clone();
        if let Some(ed) = self.apply_blade_resource(
            gs,
            effect,
            kind,
            &blade_targets,
            activating_card_id,
            &all_selected,
            ResourceFlags::for_blade(is_all, is_temporary),
            final_count,
            blades_to_add,
        ) {
            effect_data = Some(ed);
        }

        // group_reference: "same_group_name" — filter heart targets to only
        // include cards whose group name (c.group, card position ②) matches the
        // group of the card that was discarded as cost (tracked in self.moved_cards).
        if effect.group_reference_any() == Some("same_group_name") {
            log::debug!("[SAME_GROUP] moved_cards={:?}", self.selection.moved_cards);
            let ref_group: Option<String> = self
                .selection.moved_cards
                .first()
                .and_then(|cid| {
                    let c = gs.card_database.get_card(*cid);
                    log::debug!(
                        "[SAME_GROUP] cid={} card={:?}",
                        cid,
                        c.as_ref().map(|c| (&c.name, &c.group))
                    );
                    c
                })
                .map(|c| c.group.to_string());
            log::debug!("[SAME_GROUP] ref_group={:?}", ref_group);
            if let Some(ref group) = ref_group {
                let before = heart_targets.len();
                heart_targets.retain(|cid: &i16| {
                    let matches =
                        util::card_matches_group_str(&gs.card_database, *cid, Some(group.as_str()));
                    log::debug!(
                        "[SAME_GROUP] cid={} matches={} group={}",
                        cid,
                        matches,
                        group
                    );
                    matches
                });
                log::debug!(
                    "[SAME_GROUP] heart_targets: {} -> {}",
                    before,
                    heart_targets.len()
                );
            }
        }

        if let Some(ed) = self.apply_heart_resource(
            gs,
            effect,
            kind,
            heart_targets,
            activating_card_id,
            ResourceFlags {
                self_target: is_self_target,
                all: is_all,
                temporary: is_temporary,
                negative: is_negative,
            },
            &heart_distribution,
            &heart_color_str,
            heart_to_add,
            final_count,
        ) {
            effect_data = Some(ed);
        }

        // Store effect_data for blade cleanup.
        if is_temporary && effect_data.is_none() && kind == ResourceKind::Blade {
            let items: Vec<crate::core::types::CardEffectItem> = blade_targets_save
                .iter()
                .map(|&cid| crate::core::types::CardEffectItem {
                    card_id: cid,
                    amount: final_count as i16,
                    color: None,
                })
                .collect();
            effect_data = Some(crate::core::types::EffectData::MultiCard { items });
        }

        // Resource gain details captured in structured ability_resolution entry
        if is_temporary {
            util::push_temporary_effect(
                gs,
                &format!("gain_{}", resource),
                duration,
                &target,
                &format!("Gain {} {}", final_count, resource),
                effect_data,
            );
        }
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.push_rule_log_fmt(format_args!(
            "{} {}: [[log_gain_resource:n={},type={}]]",
            pp,
            act_name,
            effect.count_any().unwrap_or(1),
            effect.resource_any().unwrap_or("?")
        ));
        Ok(())
    }

    /// Apply heart modifiers to the resolved targets. Returns the effect data
    /// to register for temporary-effect cleanup, if any. Extracted verbatim
    /// from `execute_gain_resource` — see docs/CODE_AUDIT_2026-08-23.md (C1).
    #[allow(clippy::too_many_arguments)]
    fn apply_heart_resource(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        kind: ResourceKind,
        heart_targets: Vec<i16>,
        activating_card_id: Option<i16>,
        flags: ResourceFlags,
        heart_distribution: &[(crate::card::HeartColor, u8)],
        heart_color_str: &Option<String>,
        heart_to_add: i16,
        final_count: u8,
    ) -> Option<crate::core::types::EffectData> {
        let ResourceFlags {
            self_target: is_self_target,
            all: is_all,
            temporary: is_temporary,
            negative: is_negative,
        } = flags;
        if kind != ResourceKind::Heart {
            return None;
        }
        let mut effect_data: Option<crate::core::types::EffectData> = None;
        if heart_targets.is_empty() {
            if let Some(card_id) =
                Self::implicit_heart_target(gs, effect, is_self_target, activating_card_id)
            {
                self.apply_heart_to_card(
                    gs,
                    card_id,
                    heart_distribution,
                    is_negative,
                    is_temporary,
                    &mut effect_data,
                    heart_color_str,
                    heart_to_add,
                    &effect.text,
                );
            }
        } else if is_self_target
            || Self::is_broadly_self(effect, is_self_target, activating_card_id)
        {
            if let Some(card_id) = activating_card_id {
                self.apply_heart_to_card(
                    gs,
                    card_id,
                    heart_distribution,
                    is_negative,
                    is_temporary,
                    &mut effect_data,
                    heart_color_str,
                    heart_to_add,
                    &effect.text,
                );
            }
        } else {
            let targets: Vec<i16> =
                if is_all || effect.multiple_targets_any().unwrap_or(false) {
                    heart_targets.clone()
                } else {
                    heart_targets
                        .into_iter()
                        .take(final_count as usize)
                        .collect()
                };
            log::debug!(
                "[HEART_APPLY] targets={:?} is_all={} final_count={}",
                targets,
                is_all,
                final_count
            );
            for &card_id in &targets {
                log::debug!(
                    "[HEART_APPLY] adding heart04 to card_id={}, activating={:?}",
                    card_id,
                    gs.activating_card
                );
                for &(color, _) in heart_distribution {
                    log::debug!(
                        "[HEART_APPLY]   color={:?} current_mod={}",
                        color,
                        gs.mods.get_heart_modifier(card_id, color)
                    );
                }
                Self::grant_heart_distribution(gs, &effect.text, card_id, heart_distribution, is_negative);
            }
            // Build effect_data for heart cleanup on expiry
            if is_temporary && effect_data.is_none() && !targets.is_empty() {
                if heart_distribution.len() > 1 {
                    let items: Vec<crate::core::types::CardEffectItem> = targets
                        .iter()
                        .flat_map(|&cid| {
                            heart_distribution.iter().map(move |&(c, dc)| {
                        let amount = if is_negative { -i16::from(dc) } else { i16::from(dc) };
                                crate::core::types::CardEffectItem {
                                    card_id: cid,
                                    amount,
                                    color: Some(format!("{:?}", c)),
                                }
                            })
                        })
                        .collect();
                    effect_data = Some(crate::core::types::EffectData::MultiCard { items });
                } else {
                    let color_name = heart_color_str.as_deref().unwrap_or("heart01");
                    let items: Vec<crate::core::types::CardEffectItem> = targets
                        .iter()
                        .map(|&cid| crate::core::types::CardEffectItem {
                            card_id: cid,
                            amount: heart_to_add,
                            color: Some(color_name.to_string()),
                        })
                        .collect();
                    effect_data = Some(crate::core::types::EffectData::MultiCard { items });
                }
            }
        }
        effect_data
    }

    /// Apply blade modifiers to the resolved targets. Returns the effect data
    /// to register for temporary-effect cleanup, if any. Extracted verbatim
    /// from `execute_gain_resource` — see docs/CODE_AUDIT_2026-08-23.md (C1).
    #[allow(clippy::too_many_arguments)]
    fn apply_blade_resource(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        kind: ResourceKind,
        blade_targets: &SmallVec<[i16; 8]>,
        activating_card_id: Option<i16>,
        all_selected: &[i16],
        flags: ResourceFlags,
        final_count: u8,
        blades_to_add: i16,
    ) -> Option<crate::core::types::EffectData> {
        let ResourceFlags {
            all: is_all,
            temporary: is_temporary,
            ..
        } = flags;
        if kind != ResourceKind::Blade {
            return None;
        }
        let mut effect_data: Option<crate::core::types::EffectData> = None;
        if blade_targets.is_empty() {
            if is_all
                && effect.group_names_any().is_none()
                && effect.card_type_any().is_none()
                && effect.characters_any().is_none()
                && effect.timing_condition_any().is_none()
                && effect.position_any().is_none()
            {
                let stage_ids: Vec<i16> = {
                    let player = gs.resolve_target_player(effect.target_name());
                    player
                        .stage
                        .stage
                        .iter()
                        .copied()
                        .filter(|&id| id != -1)
                        .collect()
                };
                for card_id in stage_ids {
                    Self::grant_blade(gs, effect, card_id, blades_to_add);
                }
                if is_temporary {
                    effect_data = Some(crate::core::types::EffectData::AllCards {
                        amount: blades_to_add,
                    });
                }
            } else if effect.position_any().is_some() {
                // Position-based target: apply to the stage member at that position
                if let Some(pos_info) = effect.position_any().as_ref() {
                    if let Some(p) = pos_info.get_position() {
                        if let Some(stage_idx) = util::stage_position_index(p) {
                            let player = gs.resolve_target_player_mut(effect.target_name());
                            let card_id = player.stage.stage[stage_idx];
                            if card_id != -1 {
                                Self::grant_blade(gs, effect, card_id, blades_to_add);
                                if is_temporary {
                                    effect_data = Some(Self::make_card_effect_data(
                                        card_id,
                                        blades_to_add,
                                        None,
                                    ));
                                }
                            }
                        }
                    }
                }
            } else if effect.target_count_any().is_none()
                // Fallback-to-self is for plain "this member gains N" shapes.
                // exclude_self effects (「ほかのメンバーは…」) must NEVER land
                // on the activating card when no other member qualifies.
                && !effect.exclude_self_any().unwrap_or(false)
            {
                if let Some(card_id) = activating_card_id {
                    Self::grant_blade(gs, effect, card_id, blades_to_add);
                    if is_temporary {
                        effect_data =
                            Some(Self::make_card_effect_data(card_id, blades_to_add, None));
                    }
                }
            }
        } else if !all_selected.is_empty() && effect.source_any().is_none() {
            // Pure sequential select→gain_resource: apply to ALL selected cards with full count
            for &card_id in blade_targets.iter() {
                Self::grant_blade(gs, effect, card_id, blades_to_add);
            }
        } else {
            let targets: Vec<i16> = if is_all {
                blade_targets.iter().copied().collect()
            } else {
                blade_targets
                    .iter()
                    .take(final_count as usize)
                    .copied()
                    .collect()
            };
            if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
                log::debug!(
                    "[BLADE_APPLY] targets={:?} is_all={} final_count={} blades_to_add={}",
                    targets,
                    is_all,
                    final_count,
                    blades_to_add
                );
            }
            for &card_id in &targets {
                Self::grant_blade(gs, effect, card_id, blades_to_add);
            }
        }
        effect_data
    }

    /// Resolve which stage members receive the resource and in what amount.
    /// Extracted verbatim from `execute_gain_resource` — see the audit doc
    /// (docs/CODE_AUDIT_2026-08-23.md, C1).
    fn resolve_gain_resource_targets(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        req: &GainResourceRequest<'_>,
    ) -> GainTargets {
        // Unpacked by value so the body below reads exactly as it did when
        // these were 26 positional parameters. The struct is the documentation.
        let GainResourceRequest {
            kind,
            resource,
            count,
            per_unit,
            per_unit_type_str,
            this_ability_moved,
            target,
            is_all,
            is_self_target,
            card_type_filter,
            group_filter,
            activating_card_id,
            exclude_self_id,
            all_selected,
            selected_for_current,
            appeared_ids,
            last_energy,
            last_discard_count,
            recently_moved,
            entry_snapshot,
            preceding_moved,
            orientation_modifiers,
            single_fixed_heart,
        } = req.clone();
        let card_db = self.card_db();
        // 「そのバトンタッチで登場したメンバーは…を得る」 (PL!-PR-025-PR, PL!S-PR-047-PR,
        // PL!HS-PR-040-PR): the activating member is the REPLACED one — her condition
        // carries trigger_event.location=discard — so the printed target is the member
        // that ARRIVED in that baton touch, never every member standing on stage.
        let baton_arriver: SmallVec<[i16; 8]> = match effect.condition.as_deref() {
            Some(cond) if cond.get_movement() == Some("baton_touch") => {
                let loc = cond
                    .get_location()
                    .or_else(|| cond.get_trigger_event().and_then(|t| t.location.as_deref()))
                    .unwrap_or("");
                let self_replaced =
                    matches!(Zone::from_str(loc), Some(Zone::Discard | Zone::Waitroom));
                let arriving = gs.baton_touch_arriving_card_id;
                log::debug!(
                    "[BATON_ARRIVER] loc={} self_replaced={} arriving={:?}",
                    loc,
                    self_replaced,
                    arriving
                );
                let mut only: SmallVec<[i16; 8]> = SmallVec::new();
                if self_replaced {
                    if let Some(id) = arriving {
                        only.push(id);
                    }
                }
                only
            }
            _ => SmallVec::new(),
        };
        let mut filter = effect.filter_subset();
        filter.exclude_self = exclude_self_id;
        let exclude_names: Vec<String> = effect
            .exclude_by_name_source_any()
            .filter(|&s| s == "preceding_moved")
            .and(preceding_moved.as_ref())
            .map(|moved| {
                moved
                    .iter()
                    .filter_map(|&cid| card_db.get_card(cid).map(|c| c.name.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        if !exclude_names.is_empty() {
            filter.exclude_names = Some(&exclude_names);
        }

        let final_count = self.calculate_gain_multiplier(
            gs,
            effect,
            &GainMultiplierRequest {
                per_unit,
                base_count: count,
                per_unit_type_str,
                target,
                recently_moved,
                entry_snapshot,
                last_energy,
                last_discard_count,
                this_ability_moved: &this_ability_moved,
                orientation_modifiers,
                filter: &filter,
            },
        );

        // "ブレードをNつ以上持つ" (no 元々) — CURRENT blade total filter
        // (base or set + modifiers). Snapshot the eligible stage members
        // BEFORE the mutable `player` borrow (matches() only handles
        // printed/original blade values).
        let current_blade_eligible: SmallVec<[i16; 8]> = if filter.has_current_blade_filter() {
            let stage_ids: Vec<i16> = {
                let p = gs.resolve_target_player(target);
                util::zone_cards(p, Zone::Stage.to_str()).to_vec()
            };
            util::filter_current_blade(
                stage_ids,
                gs,
                filter.current_blade_limit,
                filter.current_blade_operator,
            )
            .into()
        } else {
            SmallVec::new()
        };

        // 「そうした場合、そのメンバーは…」 (e.g. PL!N-bp3-025-L Awakening
        // Promise): a per-unit gain that follows an under_member→energy_deck
        // move targets THE HOST MEMBER(S) of the moved energy — never the
        // whole stage card_type-filter match set. Snapshot before the mutable
        // `player` borrow below.
        let under_move_hosts: SmallVec<[i16; 4]> = if kind == ResourceKind::Heart
            && per_unit
            && per_unit_type_str == Some("energy_deck")
        {
            gs.mods.last_under_move_host_ids.clone()
        } else {
            SmallVec::new()
        };
        if !under_move_hosts.is_empty() {
            log::debug!(
                "[UNDER_MOVE_HOSTS] targeting members {:?} for per-unit heart gain",
                under_move_hosts
            );
        }

        let player = gs.resolve_target_player_mut(target);

        let has_selection_filter =
            effect.target_count_any().is_some() || effect.distinct_any().is_some();
        // When a distinct choice was saved (target_count cleared), exclude
        // only cards selected BEFORE the choice, not the card selected BY it.
        let saved_exclude: Option<SmallVec<[i16; 8]>> = if effect.target_count_any().is_none()
            && effect.distinct_any().is_some()
            && !all_selected.is_empty()
        {
            let before = selected_before_save(all_selected, self.selection.count_at_save);
            (!before.is_empty()).then(|| before.into())
        } else {
            None
        };
        let exclude: Option<&[i16]> = if let Some(ref saved) = saved_exclude {
            Some(saved.as_slice())
        } else if has_selection_filter && !all_selected.is_empty() {
            Some(all_selected)
        } else {
            None
        };
        let tc = effect.target_count_any();
        let dn = effect.distinct_any();

        let has_characters = effect.characters_any().is_some_and(|c| !c.is_empty());
        let has_blade_filter =
            card_type_filter.is_some() || group_filter.is_some() || has_characters;
        // When has_selection_filter is set (target_count/distinct), don't blindly
        // use all_selected — apply the filter with exclusion to find the right targets.
        // Only use all_selected directly for pure sequential select→gain_resource.
        // When distinct is set, the saved action must filter by exclude to
        // prevent cards selected in previous steps from also getting the resource.
        let use_raw = !all_selected.is_empty() && !has_selection_filter && effect.distinct_any().is_none();
        let mut all_candidates: SmallVec<[i16; 8]> = if use_raw {
            all_selected.iter().copied().collect()
        } else if has_blade_filter || is_all {
            util::matching_ids_filtered(
                util::zone_cards(player, Zone::Stage.to_str()),
                &card_db,
                &filter,
                true,
                None, // don't truncate yet — we may need a player choice
                if kind == ResourceKind::Blade { dn } else { None },
                exclude,
            )
            .into()
        } else {
            SmallVec::new()
        };

        // "ブレードをNつ以上持つ" (no 元々) — restrict to the snapshot of
        // current-blade-eligible stage members computed before the borrow.
        if filter.has_current_blade_filter() {
            all_candidates.retain(|cid| current_blade_eligible.contains(cid));
        }

        if !baton_arriver.is_empty() {
            log::debug!(
                "[BATON_ARRIVER] overriding {} stage candidate(s) with {:?}",
                all_candidates.len(),
                baton_arriver
            );
            all_candidates = baton_arriver;
        }
        // Filter by position if the effect specifies one (e.g. "center"). Read
        // once and applied to both the blade and the heart target list, so the
        // constraint cannot hold for one and be skipped for the other.
        let effect_position: Option<String> =
            effect.position_any().and_then(|p| p.get_position().map(str::to_string));
        all_candidates = position_limited_targets(player, effect_position.as_deref(), &all_candidates)
            .into();
        // Issue 6: Filter by timing_condition (e.g. "appeared_this_turn")
        log::debug!(
            "[APP_IDS] appeared_ids={:?} all_candidates before={:?}",
            appeared_ids,
            all_candidates
        );
        if effect.timing_condition_any().is_some() {
            all_candidates.retain(|cid| appeared_ids.contains(cid));
            log::debug!("[APP_IDS] all_candidates after={:?}", all_candidates);
        }
        // same_name: filter candidates to only members whose name matches
        // the card(s) moved as cost (self.moved_cards).
        if effect.same_name_any().unwrap_or(false) {
            log::debug!(
                "[SAME_NAME] before={} candidates={}",
                self.selection.moved_cards.len(),
                all_candidates.len()
            );
            all_candidates = same_name_targets(&card_db, &self.selection.moved_cards, &all_candidates).into();
            log::debug!("[SAME_NAME] after={}", all_candidates.len());
        }

        // If target_count is set and more candidates than needed,
        // create a choice for the player (unless already selected via previous choice).
        log::debug!("[GAIN_RESOURCE] res={} is_all={} has_filter={} tc={:?} dn={:?} all_cand={} selected={}",
            resource, is_all, has_blade_filter, tc, dn, all_candidates.len(), self.selection.cards.len());
        log::debug!(
            "[GAIN_RESOURCE] blade_targets computation starts ({} candidates)",
            all_candidates.len()
        );
        let blade_targets: SmallVec<[i16; 8]> =
            if effect.target_from_selection_any().unwrap_or(false) {
                selected_for_current.iter().copied().collect()
            } else if let Some(tgt_count) = tc {
                if !selected_for_current.is_empty() && all_candidates.is_empty() {
                    // Pure-selection shapes with no filter-matched candidates:
                    // inherit the preceding step's selection.
                    selected_for_current
                        .iter()
                        .take(tgt_count as usize)
                        .copied()
                        .collect()
                } else {
                    // The effect's own filter (card_type/group/location)
                    // describes fresh targets — a preceding step's selection
                    // must NOT leak into them (「そうした場合」 chains where
                    // step 1's choice would otherwise hijack step 2).
                    all_candidates.truncate(tgt_count as usize);
                    all_candidates
                }
            } else if !selected_for_current.is_empty() && effect.distinct_any().is_none() {
                selected_for_current.iter().copied().collect()
            } else {
                all_candidates
            };

        let heart_color_inner = single_fixed_heart
            .clone()
            .or_else(|| effect.heart_color_any().map(|s| s.to_string()))
            .or_else(|| effect.heart_colors_any().first().map(|s| s.to_string()));
        let mut heart_targets: Vec<i16> = if !under_move_hosts.is_empty() {
            under_move_hosts.to_vec()
        } else if effect.target_from_selection_any().unwrap_or(false) {
            // Explicitly target only the cards selected by the preceding
            // sequential action (e.g. a change_state that activated a member).
            log::debug!(
                "[TARGET_FROM_SEL] heart: selected_cards={:?} selected_for_current={:?} all_selected={:?}",
                self.selection.cards, selected_for_current, all_selected
            );
            selected_for_current.to_vec()
        } else if use_raw && !selected_for_current.is_empty() && effect.distinct_any().is_none() {
            if effect.multiple_targets_any().unwrap_or(false) {
                let mut targets = selected_for_current.to_vec();
                if let Some(aid) = activating_card_id {
                    if !targets.contains(&aid) {
                        targets.push(aid);
                    }
                }
                targets
            } else {
                selected_for_current.to_vec()
            }
        } else if use_raw {
            all_selected.to_vec()
        } else if kind == ResourceKind::Heart {
            let mut h = if !selected_for_current.is_empty() && effect.distinct_any().is_none() {
                selected_for_current.to_vec()
            } else if !selected_for_current.is_empty()
                && effect.target_count_any().is_none()
                && effect.distinct_any().is_some()
            {
                // Saved action from distinct choice: target only the
                // NEWLY selected cards (after the pre-choice save point).
                selected_after_save(selected_for_current, self.selection.count_at_save)
            } else if effect.card_type_any().is_none()
                && effect.group_names_any().is_none()
                && effect.characters_any().is_none()
                && effect.target_count_any().is_none()
                && effect.distinct_any().is_none()
                && !is_all
            {
                // No targeting info: default to activating card only.
                // Prevents heart from leaking to all stage members when
                // no card_type/group/characters/target_count filter is set.
                activating_card_id.map_or(vec![], |id| vec![id])
            } else {
                util::matching_ids_filtered(
                    util::zone_cards(player, Zone::Stage.to_str()),
                    &card_db,
                    &filter,
                    true,
                    if is_self_target { None } else { tc },
                    dn,
                    exclude,
                )
                .to_vec()
            };
            if effect.timing_condition_any().is_some() {
                h.retain(|cid| appeared_ids.contains(cid));
            }
            // same_name: filter heart_targets to same-name members
            if effect.same_name_any().unwrap_or(false) {
                h = same_name_targets(&card_db, &self.selection.moved_cards, &h);
            }
            h
        } else {
            vec![]
        };
        heart_targets = position_limited_targets(player, effect_position.as_deref(), &heart_targets);

        // "ブレードをNつ以上持つ" (no 元々) — heart targets are resolved via
        // a SEPARATE matching_ids_filtered call, so apply the current-blade
        // post-filter here as well (the blade_targets path already filters).
        if filter.has_current_blade_filter() {
            heart_targets.retain(|cid| current_blade_eligible.contains(cid));
        }

        // Apply heart_colors as a target filter when the effect
        // specifies that targets must already possess the heart color.
        if effect.filter_targets_by_heart_colors_any().unwrap_or(false)
            && !effect.heart_colors_any().is_empty()
        {
            heart_targets.retain(|&id| {
                if effect.require_all_heart_colors_any().unwrap_or(false) {
                    util::card_matches_all_heart_colors(&card_db, id, effect.heart_colors_any())
                } else {
                    util::card_matches_heart_colors(&card_db, id, effect.heart_colors_any())
                }
            });
        }

        GainTargets {
            blade_targets,
            heart_targets,
            heart_color_str: heart_color_inner,
            final_count,
        }
    }
}