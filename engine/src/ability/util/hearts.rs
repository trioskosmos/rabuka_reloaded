// Heart-gain + constant per_unit helpers.
//
// Extracted from `ability/util.rs`. These centralize the "distribute total
// across the color multiset" rule and the per_unit zone/units resolution that
// was previously copy-pasted between blade/heart paths.
use crate::core::constants::CountCast;

use super::super::enums::Zone;
use crate::card::CardDatabase;
use crate::HashMap;
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

/// Distribute a multi-color heart gain across its color multiset.
///
/// A heart gain is described by a multiset `heart_colors` (one token per
/// granted heart, e.g. `heart02 heart02`) and a `total` number of hearts to
/// grant. Historically the engine granted `total` to EACH color, so a
/// one-of-each ability (`heart02/heart04/heart05`) became three-of-each.
///
/// We instead distribute `total` evenly across the multiset: each entry
/// receives `total / len(heart_colors)`. Because the parser sets
/// `count == len(heart_colors)`, this reduces to granting exactly 1 to each
/// entry (e.g. `[heart02, heart02]` with total 2 → 1 of heart02, total 2).
pub fn heart_gain_per_entry(total: i32, heart_colors: &[String]) -> i32 {
    let len = heart_colors.len().max(1);
    total / len.i32_count()
}

/// `heart_type == "all"` means wildcard any-color heart. The constant-heart
/// map stores it as `"heart00"` (HeartColor::Heart00 placeholder) while the
/// GainAbility path stores it as `"all"`. Keep the string check centralized.
pub fn is_all_heart_type(effect: &crate::card::AbilityEffect) -> bool {
    effect.heart_type_any() == Some("all")
}
pub const HEART_ALL_KEY: &str = "heart00";

/// Resolve the zone string for a constant per_unit gain. Mirrors the
/// `loc_b.or(per_b).unwrap_or(Hand)` logic duplicated in `modifiers.rs`.
pub fn constant_per_unit_zone(effect: &crate::card::AbilityEffect) -> &str {
    let loc_b = effect.location_any();
    let per_b = effect.per_unit_type_any();
    loc_b.or(per_b).unwrap_or(Zone::Hand.to_str())
}

/// Compute the `units` part of a constant per_unit gain (before `base`).
/// This is the `resolve_per_unit_count / per_unit_count + max cap` stanza
/// that was copy-pasted between blade/heart constant paths in `modifiers.rs`.
pub fn constant_per_unit_units(
    effect: &crate::card::AbilityEffect,
    player: &crate::player::Player,
    card_db: &CardDatabase,
    orientation_modifiers: &HashMap<i16, crate::core::game_modifiers::CardOrientation>,
    host_card_id: i16,
) -> i32 {
    let zone = constant_per_unit_zone(effect);
    let mut filter = effect.filter_subset();
    filter.exclude_self = Some(host_card_id);
    let per_count = super::resolve_per_unit_count(
        true,
        Some(zone),
        player,
        card_db,
        &filter,
        &[],
        effect.state_any(),
        orientation_modifiers,
        Some(host_card_id),
    );
    let mut units = per_count as i32 / effect.per_unit_count_any().unwrap_or(1).max(1) as i32;
    if effect.max.unwrap_or(false) {
        if let Some(cap) = effect.count_any() {
            units = units.min(cap as i32);
        }
    }
    log::debug!(
        "[CONST_PER_UNIT] host={} player={} zone={} state={:?} target={:?} raw_count={} per_unit_divisor={:?} units={} capped={}",
        host_card_id,
        player.id,
        zone,
        effect.state_any(),
        effect.target_any(),
        per_count,
        effect.per_unit_count_any(),
        per_count as i32 / effect.per_unit_count_any().unwrap_or(1).max(1) as i32,
        units
    );
    units
}
