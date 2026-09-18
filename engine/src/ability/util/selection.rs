// Zone-selection primitives shared across move_cards, cost, and any other
// zone-selection logic.
//
// Extracted from `ability/util.rs` unchanged apart from imports.

use super::super::enums::Zone;
use super::{matching_indices, CardFilter};
use crate::card::CardDatabase;
#[cfg(feature = "no_std")]
use alloc::{string::String, vec::Vec};

/// How a zone selection resolves when there aren't enough matching cards.
#[derive(Clone)]
pub enum InsufficientBehavior {
    /// Silently skip (treat as zero cards taken).
    Silent,
    /// Return an error with the given message.
    Error(String),
}

/// The outcome of resolving a card selection from a zone.
#[derive(Debug, Clone)]
pub enum SelectionOutcome {
    /// Exact match — the indices to take.
    Exact(Vec<usize>),
    /// Too many candidates — the caller must prompt the player.
    Prompt,
    /// Too few candidates — skip silently.
    Skip,
}

/// Classify a set of candidate indices against a required count.
pub fn classify_selection(
    idxs: &[usize],
    count: usize,
    is_all: bool,
    on_insufficient: InsufficientBehavior,
) -> Result<SelectionOutcome, String> {
    if is_all {
        return Ok(SelectionOutcome::Exact(idxs.to_vec()));
    }
    if idxs.len() < count {
        return match on_insufficient {
            InsufficientBehavior::Silent => Ok(SelectionOutcome::Skip),
            InsufficientBehavior::Error(msg) => Err(msg),
        };
    }
    if idxs.len() > count {
        return Ok(SelectionOutcome::Prompt);
    }
    Ok(SelectionOutcome::Exact(idxs.to_vec()))
}

/// Return indices into `cards` matching the filter, with optional self-target pinning.
pub fn get_selection_indices(
    cards: &[i16],
    card_db: &CardDatabase,
    activating_card: Option<i16>,
    filter: &CardFilter,
    self_target_only: bool,
    skip_empty: bool,
) -> Vec<usize> {
    log::trace!(
        "[SELECTION_FILTER] cards.len={} filter.nh_color={:?} filter.nh_total={:?} ct={:?} group={:?} groups={:?} chars={:?} excl_chars={:?} cost_lim={:?} cost_op={:?} cost_vals={:?} cost_min={:?} cost_max={:?} excl_self={:?} names={:?} hearts={:?} nhc_count={:?} distinct={:?} excl_groups={:?} excl_cards={:?}",
        cards.len(),
        filter.need_heart_color,
        filter.need_heart_total,
        filter.card_type,
        filter.group,
        filter.groups,
        filter.characters,
        filter.exclude_characters,
        filter.cost_limit,
        filter.cost_operator,
        filter.cost_values,
        filter.cost_limit_min,
        filter.cost_limit_max,
        filter.exclude_self,
        filter.name_fragments,
        filter.heart_colors,
        filter.heart_color_count,
        filter.distinct,
        filter.exclude_group_names,
        filter.exclude_cards,
    );
    let mut idxs = matching_indices(cards, card_db, filter, skip_empty);
    if self_target_only {
        if let Some(aid) = activating_card {
            idxs.retain(|&i| i < cards.len() && cards[i] == aid);
        }
    }
    idxs
}

/// Full selection resolution: filter → classify → SelectionOutcome.
pub fn resolve_selection(
    cards: &[i16],
    card_db: &CardDatabase,
    activating_card: Option<i16>,
    count: usize,
    is_all: bool,
    filter: &CardFilter,
    self_target_only: bool,
    behavior: InsufficientBehavior,
    skip_empty: bool,
) -> Result<SelectionOutcome, String> {
    let idxs = get_selection_indices(
        cards,
        card_db,
        activating_card,
        filter,
        self_target_only,
        skip_empty,
    );
    let outcome = classify_selection(&idxs, count, is_all, behavior);
    log::debug!("[SELECTION_RESULT] available={} matching={:?} requested={} all={} outcome={:?}", cards.len(), idxs, count, is_all, outcome);
    outcome
}

/// Remove cards from a named zone by indices (indices processed in descending order).
pub fn zone_remove_at_indices(
    player: &mut crate::player::Player,
    zone: &str,
    indices: &[usize],
) -> Vec<i16> {
    let mut sorted = indices.to_vec();
    sorted.sort_unstable_by(|a, b| b.cmp(a));
    sorted
        .iter()
        .map(|&i| match Zone::from_str(zone) {
            Some(Zone::Hand) => player.hand.cards.remove(i),
            Some(Zone::Discard) | Some(Zone::Waitroom) => player.waitroom.cards.remove(i),
            Some(Zone::Energy) => player.energy_zone.cards.remove(i),
            Some(Zone::LiveCardZone) => player.live_card_zone.cards.remove(i),
            Some(Zone::SuccessLiveZone) => player.success_live_card_zone.cards.remove(i),
            Some(Zone::EnergyDeck) => player.energy_deck.cards.remove(i),
            _ => {
                if zone == "those_cards" {
                    player.waitroom.cards.remove(i)
                } else {
                    -1
                }
            }
        })
        .filter(|&c| c != -1)
        .collect()
}
