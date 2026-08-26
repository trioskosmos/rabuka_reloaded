//! Single source of truth for member stat resolution (rules 9.9.1.1–9.9.1.5).
//!
//! Three call sites previously hand-rolled this pipeline (zones.rs
//! get_available_hearts, player.rs calculate_stage_hearts, live.rs member
//! contributions) and had already drifted once — the heart override used to
//! swallow additive modifiers in two of them. ALL stat resolution must go
//! through these functions.
//!
//! Canonical order for a member's ORIGINAL hearts:
//!   1. printed base (9.9.1.1)
//!   2. heart_copy    — "元々持つハートは、下に置いたカードと同じになる"
//!                      (re-defines the base; 9.9.1.3 class)
//!   3. color multiplier — "ハートがすべてXになる" (type conversion)
//!   4. heart_override — "元々持つハートはNになる" (SET; replaces 1–3,
//!                      9.9.1.4 — Q195-class layering keeps additives alive)
//! Additive modifiers (9.9.1.5) stack ON TOP of whatever 1–4 produced.
use crate::card::{CardDatabase, HeartColor, HeartMap};
use crate::core::game_modifiers::ModifierEntry;
use crate::HashMap;

/// A member's original hearts after copy/multiplier/set layers.
pub fn member_original_hearts(
    card_db: &CardDatabase,
    card_id: i16,
    heart_override: &HashMap<i16, (HeartColor, u8)>,
    heart_copy: &HashMap<i16, i16>,
    heart_color_multiplier: &HashMap<i16, HeartColor>,
) -> HeartMap {
    // 9.9.1.4: set-type effect replaces the member's original hearts outright.
    if let Some(&(override_color, override_count)) = heart_override.get(&card_id) {
        let mut m = HeartMap::new();
        *m.entry_or_default(override_color) += override_count;
        return m;
    }

    let mut hearts = HeartMap::new();
    let base = match heart_copy.get(&card_id) {
        // heart_copy: originals become the referenced card's hearts.
        Some(&src) => card_db.get_card(src).and_then(|c| c.base_heart.clone()),
        None => card_db
            .get_card(card_id)
            .and_then(|c| c.base_heart.clone()),
    };
    if let Some(base_heart) = base {
        for (color, count) in &base_heart.hearts {
            *hearts.entry_or_default(*color) += count;
        }
    }

    // Color multiplier collapses the whole multiset into one color.
    if let Some(override_color) = heart_color_multiplier.get(&card_id) {
        let total: u8 = hearts.values_sum();
        hearts.clear();
        *hearts.entry_or_default(*override_color) += total;
    }
    hearts
}

/// 9.9.1.5: additive modifiers stack ON TOP, saturating at 0/255.
pub fn apply_additive_heart_mods(
    hearts: &mut HeartMap,
    mods: impl Iterator<Item = (HeartColor, i32)>,
) {
    for (color, delta) in mods {
        let new_val =
            crate::constants::saturate_u8(hearts.get(&color).copied().unwrap_or(0) as i32 + delta);
        if new_val > 0 {
            hearts.insert(color, new_val);
        } else {
            hearts.remove(&color);
        }
    }
}

/// Blade layering (9.9.1.4→.5): a non-zero SET replaces the printed blade;
/// additive stacks either way. Returns `(effective_base, additive_bonus)` —
/// the split exists because MemberContribution tracks them separately.
pub fn effective_blade_parts(entry: &ModifierEntry, printed_blade: u8) -> (u8, u8) {
    if entry.set != 0 {
        (
            crate::constants::saturate_u8(entry.total()),
            0,
        )
    } else {
        (
            printed_blade,
            crate::constants::saturate_u8(entry.total()),
        )
    }
}

/// Convenience: fold one stage slot's resolved hearts + additive mods into
/// `out`. Shared by every stage-fold implementation.
pub fn stage_hearts(
    stage: &[i16; 3],
    card_db: &CardDatabase,
    heart_override: &HashMap<i16, (HeartColor, u8)>,
    heart_copy: &HashMap<i16, i16>,
    heart_color_multiplier: &HashMap<i16, HeartColor>,
    heart_modifiers: &HashMap<i16, HashMap<HeartColor, ModifierEntry>>,
) -> crate::card::BaseHeart {
    let mut hearts = HeartMap::new();
    for &card_id in stage.iter() {
        if card_id == crate::constants::EMPTY_SLOT {
            continue;
        }
        let mut m = member_original_hearts(
            card_db,
            card_id,
            heart_override,
            heart_copy,
            heart_color_multiplier,
        );
        if let Some(mods) = heart_modifiers.get(&card_id) {
            // 9.9.1.5: additive modifiers stack ON TOP of the set value.
            // heart_override case already returned early from member_original_hearts
            // with the set value; we must still apply additives here.
            // For non-override path, member_original_hearts returned base/copy/multiplier
            // without additives.
            apply_additive_heart_mods(
                &mut m,
                mods.iter().map(|(c, e)| (*c, e.total())),
            );
            log::debug!(
                "[HEART_PIPELINE] card={} base+override+multiplier={:?} +mods={:?} -> {:?}",
                card_id,
                member_original_hearts(card_db, card_id, heart_override, heart_copy, heart_color_multiplier),
                mods,
                m
            );
        }
        for (color, count) in &m {
            *hearts.entry_or_default(*color) += count;
        }
    }
    crate::card::BaseHeart { hearts }
}

/// Per-member heart detail: (base_hearts_array, bonus_hearts_array).
/// base = original hearts after copy/multiplier/override (no additives).
/// bonus = additive contributions (positive deltas only, matching live.rs prior behavior
/// for MemberContribution.bonus_hearts).
pub fn member_heart_detail(
    card_db: &CardDatabase,
    card_id: i16,
    heart_override: &HashMap<i16, (HeartColor, u8)>,
    heart_copy: &HashMap<i16, i16>,
    heart_color_multiplier: &HashMap<i16, HeartColor>,
    heart_modifiers: &HashMap<i16, HashMap<HeartColor, ModifierEntry>>,
) -> ([u8; 8], [u8; 8]) {
    let base_map = member_original_hearts(
        card_db,
        card_id,
        heart_override,
        heart_copy,
        heart_color_multiplier,
    );
    let mut base_arr = [0u8; 8];
    for (color, count) in &base_map {
        let idx = color.index();
        if idx < 8 {
            base_arr[idx] += count;
        }
    }
    let mut bonus_arr = [0u8; 8];
    if let Some(mods) = heart_modifiers.get(&card_id) {
        for (color, entry) in mods {
            let idx = color.index();
            if idx < 8 && entry.total() > 0 {
                // live.rs previously only counted positive deltas in bonus_hearts
                bonus_arr[idx] += entry.total() as u8;
            }
        }
        log::debug!(
            "[HEART_DETAIL] card={} base_arr={:?} bonus_arr={:?} mods={:?}",
            card_id,
            base_arr,
            bonus_arr,
            mods
        );
    }
    (base_arr, bonus_arr)
}

/// Effective blade for a single card: printed + modifiers with layering.
/// Unified helper for A2 — replaces 4 inline copies.
pub fn effective_blade(
    card_db: &CardDatabase,
    card_id: i16,
    entry: ModifierEntry,
) -> u8 {
    let printed = card_db
        .get_card(card_id)
        .map(|c| c.blade)
        .unwrap_or(0);
    if entry.set != 0 {
        crate::constants::saturate_u8(entry.total())
    } else {
        crate::constants::saturate_u8(printed as i32 + entry.total())
    }
}

/// Effective need_heart for a live card after per-color set+additive modifiers.
/// Unified helper for A4 — replaces hand-rolled loops in zones/live.
pub fn effective_need_heart(
    base: Option<&crate::card::BaseHeart>,
    card_id: i16,
    need_heart_modifiers: &HashMap<i16, HashMap<HeartColor, ModifierEntry>>,
) -> Option<crate::card::BaseHeart> {
    let base = base?;
    if base.hearts.is_empty() {
        return Some(base.clone());
    }
    let Some(card_mods) = need_heart_modifiers.get(&card_id) else {
        return Some(base.clone());
    };
    if card_mods.is_empty() {
        return Some(base.clone());
    }
    let mut adjusted = base.clone();
    // Q115/Q127: Set-to-X applies first (per-color), then additive stacks.
    for (color, me) in card_mods {
        if me.set != 0 {
            adjusted.hearts.insert(*color, me.set as u8);
            log::debug!(
                "[NEED_HEART_PIPELINE] card={} color={:?} set={} -> {:?}",
                card_id,
                color,
                me.set,
                adjusted.hearts.get(color)
            );
        }
    }
    for (color, me) in card_mods {
        if me.additive != 0 {
            let new_val = crate::constants::saturate_u8(
                adjusted.hearts.get(color).copied().unwrap_or(0) as i32 + me.additive as i32,
            );
            if new_val > 0 {
                adjusted.hearts.insert(*color, new_val);
            } else {
                adjusted.hearts.remove(color);
            }
            log::debug!(
                "[NEED_HEART_PIPELINE] card={} color={:?} additive={} -> {:?}",
                card_id,
                color,
                me.additive,
                adjusted.hearts.get(color)
            );
        }
    }
    Some(adjusted)
}

/// Convenience: check whether a live card's need is satisfied by a heart pool,
/// using the canonical `check_heart_requirement` helper. Centralizes the
/// Q115/Q127 effective-need building + final check.
pub fn need_satisfied(
    base_need: Option<&crate::card::BaseHeart>,
    provided: &crate::card::BaseHeart,
    card_id: i16,
    need_heart_modifiers: &HashMap<i16, HashMap<HeartColor, ModifierEntry>>,
) -> bool {
    match effective_need_heart(base_need, card_id, need_heart_modifiers) {
        None => true,
        Some(eff) if eff.hearts.is_empty() => true,
        Some(eff) => {
            let ok = crate::card::check_heart_requirement(&eff, provided);
            log::debug!(
                "[NEED_CHECK] card={} need={:?} provided={:?} -> {}",
                card_id,
                eff.hearts,
                provided.hearts,
                if ok { "PASS" } else { "FAIL" }
            );
            ok
        }
    }
}
