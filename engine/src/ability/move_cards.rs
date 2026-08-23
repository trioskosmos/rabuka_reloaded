use super::enums::Zone;
use super::resolver::AbilityResolver;
use super::types::{Choice, ExecutionContext, LookAndSelectStep};
use super::util;
use crate::ability_queue::ConditionalChoice;
use crate::card::{AbilityEffect, CardDatabase, DistinctType, Operator, PlacementOrder};
use crate::game_state::GameState;
use crate::player::Player;
use crate::{HashMap, HashSet};
#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use smallvec::SmallVec;

/// Helper: drain all cards under `stage_idx` for `target` player and move them
/// to energy_zone (wait) if they are energy cards, otherwise to waitroom.
/// Returns the list of moved card ids. Shared by `resolve_from_under_member`
/// and `choice::handle_stage_selection` to avoid duplication.
pub(crate) fn drain_under_cards_to_energy_zone(
    gs: &mut GameState,
    target: &str,
    stage_idx: usize,
) -> Vec<i16> {
    let under = core::mem::take(
        &mut gs
            .resolve_target_player_mut(target)
            .stage
            .under_cards[stage_idx],
    );
    let mut moved = Vec::new();
    for cid in under {
        let is_energy = gs
            .card_database
            .get_card(cid)
            .is_some_and(|c| c.is_energy());
        if is_energy {
            gs.resolve_target_player_mut(target)
                .energy_zone
                .cards
                .push(cid);
            gs.mods.add_orientation_modifier(cid, "wait");
        } else {
            gs.resolve_target_player_mut(target).waitroom.add_card(cid);
        }
        moved.push(cid);
        // Record movement for condition checks (preceding_moved)
        let pid = gs
            .ability_queue
            .current_entry()
            .map(|e| e.player_id.clone())
            .unwrap_or_else(|| "p1".to_string());
        gs.push_movement_event(cid, "under_member", "energy_zone", gs.activating_card, &pid, true);
    }
    gs.recalculate_constants();
    moved
}

#[derive(Debug, Clone, PartialEq)]
pub enum MoveCardsTarget {
    PlayerSelf,
    Opponent,
}

/// Bundles the inputs to `resolve_from_zone` so per-zone resolution methods
/// have a small signature instead of 21 positional arguments. Holds only
/// borrows / `Copy` values, so the whole struct is freely reconstructable.
struct MoveSourceContext<'a> {
    effective_source: &'a str,
    source_str: &'a str,
    count: usize,
    effect: &'a AbilityEffect,
    card_type_filter: Option<&'a str>,
    group_name: Option<&'a str>,
    cost_limit: Option<u8>,
    cost_total: Option<u8>,
    cost_total_operator: Option<&'a str>,
    character_filter: Option<&'a Vec<String>>,
    name_fragments: Option<&'a Vec<String>>,
    is_self_cost: bool,
    is_max: bool,
    is_all: bool,
    exclude_self: bool,
    activating_card_id: Option<i16>,
    use_p2: bool,
    destination: &'a str,
    card_db: &'a crate::card::CardDatabase,
}

fn remove_card_from_any_zone(
    player: &mut crate::player::Player,
    last_vacated_stage_area: &mut Option<u8>,
    card_id: i16,
) {
    if let Some(pos) = player.hand.cards.iter().position(|&id| id == card_id) {
        player.hand.cards.remove(pos);
    } else if let Some(pos) = player.waitroom.cards.iter().position(|&id| id == card_id) {
        player.waitroom.cards.remove(pos);
    } else if let Some(pos) = player.stage.stage.iter().position(|&id| id == card_id) {
        player.stage.stage[pos] = -1;
        // Rule 9.6.2.1.2.1: Card left stage, clean up tracking.
        player.deployed_this_turn.retain(|id| *id != card_id);
        *last_vacated_stage_area = Some(pos as u8);
    } else if let Some(pos) = player
        .energy_zone
        .cards
        .iter()
        .position(|&id| id == card_id)
    {
        player.energy_zone.cards.remove(pos);
    }
}

impl AbilityResolver {
    fn resolve_cost_limit_reference(
        &self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<Option<u8>, String> {
        let ref_binding = effect.cost_reference_any();
        let reference = match ref_binding.as_deref() {
            Some(r) => r,
            None => return Ok(effect.cost_limit_any()),
        };

        let offset = effect.cost_offset_any().unwrap_or(0);
        let moved = self.moved_cards.last().copied();
        let recently = gs
            .recently_moved_cards
            .as_ref()
            .and_then(|cards| cards.last().copied());
        log::debug!(
            "[COST_REF] moved_cards={:?} recently={:?} self.moved={:?}",
            moved,
            recently,
            self.moved_cards
        );
        let referenced_id = match reference {
            "previous_moved_card" => moved.or(recently),
            _ => None,
        }
        .ok_or_else(|| format!("Unknown or unresolved cost reference: {}", reference))?;

        let card_name = gs
            .card_database
            .get_card(referenced_id)
            .map(|c| c.name.to_string())
            .unwrap_or_default();
        let base_cost = gs
            .card_database
            .get_card(referenced_id)
            .and_then(|c| c.cost)
            .ok_or_else(|| {
                format!(
                    "Referenced card {} ({}) has no base cost for relative cost filter",
                    referenced_id, card_name
                )
            })?;

        let resolved =
            crate::constants::saturate_u8((base_cost as i32).saturating_add(offset as i32));
        log::debug!(
            "[COST_REF] referenced='{}' name='{}' base_cost={} offset={} resolved={}",
            reference,
            card_name,
            base_cost,
            offset,
            resolved
        );
        Ok(Some(resolved))
    }

    /// Compile a move_cards effect into a granular step.
    /// Returns Ok(Some(choice)) if a selection choice is needed,
    /// Compile a move_cards effect into a single SelectCards Choice.
    /// Returns Some(choice) when a selection choice is needed (Prompt case),
    /// None when the effect should fall through to execute_move_cards.
    fn prompt_card_selection(
        &mut self,
        zone: &str,
        count: usize,
        can_skip: bool,
        effect: &AbilityEffect,
        filter: &util::CardFilter,
        filtered_indices: Option<Vec<usize>>,
    ) {
        log::debug!(
            "[PROMPT_SEL@{}] zone={} count={} can_skip={} prop={:?} neg={:?} filtered={:?} excl_chars={:?}",
            line!(),
            zone,
            count,
            can_skip,
            filter.card_property,
            filter.negation,
            filtered_indices,
            filter.exclude_characters
        );
        let zone_display = crate::ability::describe::zone_label(Some(zone));
        let zone_display_ja = crate::ability::describe::zone_label_ja(Some(zone));
        let description = if effect.any_number_any().unwrap_or(false) {
            format!("Select any number of card(s) from {}", zone_display)
        } else {
            format!("Select {} card(s) from {}", count, zone_display)
        };
        let description_ja = if effect.any_number_any().unwrap_or(false) {
            format!("{}から任意枚選択", zone_display_ja)
        } else {
            format!("{}から{}枚選択", zone_display_ja, count)
        };
        self.pending_choice = Some(
            Choice::select_cards(zone, count, description, can_skip)
                .description_ja(Some(description_ja))
                .card_type(filter.card_type.map(|s| s.to_string()))
                .cost_limit(
                    filter.cost_limit,
                    effect.cost_limit_operator_any().map(|s| s.to_string()),
                )
                .cost_total(
                    filter.cost_total,
                    effect.cost_total_operator_any().map(|s| s.to_string()),
                )
                .group(filter.group.map(|s| s.to_string()))
                .characters(filter.characters.map(|v| v.to_vec()))
                .target_player_id(Some(effect.target.as_deref().unwrap_or("self").to_string()))
                .filtered_indices(filtered_indices)
                .destination(effect.destination.clone().map(|s| s.to_string()))
                .discard_remaining(effect.discard_remaining_any())
                .build(),
        );
        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
    }

    /// Returns Ok(true) if a choice was created, Ok(false) if the card was placed immediately.
    fn place_card_with_stage_choice(
        &mut self,
        gs: &mut GameState,
        player_target: &str,
        card_id: i16,
        destination: &str,
        vacated_area: Option<u8>,
        is_max: bool,
        count: usize,
        state_change: Option<String>,
        deck_position: Option<usize>,
        source_zone: &str,
        allow_occupied_stage: bool,
        under_self: bool,
    ) -> Result<bool, String> {
        let activating_card = gs.activating_card;
        let player = gs.resolve_target_player_mut(player_target);
        if Zone::from_str(destination) == Some(Zone::EmptyArea)
            || Zone::from_str(destination) == Some(Zone::Stage)
        {
            let empty_slots: Vec<usize> = (0..3).filter(|&i| player.stage.stage[i] == -1).collect();

            // Determine which slots are available for placement
            let available_slots: Vec<usize> = if allow_occupied_stage {
                // Q76: Include ALL positions (including occupied), excluding areas where the
                // member at that slot was deployed this turn (Rule 9.6.2.1.2.1).
                (0..3)
                    .filter(|&i| {
                        let card_id = player.stage.stage[i];
                        card_id == -1 || !player.deployed_this_turn.contains(&card_id)
                    })
                    .collect()
            } else {
                empty_slots.clone()
            };

            if available_slots.is_empty() {
                return Err("Stage is full".to_string());
            }
            if available_slots.len() > 1 {
                // Prefer the vacated area (other baton-passed position) if still empty
                if let Some(va) = vacated_area {
                    if va < 3 && player.stage.stage[va as usize] == -1 {
                        player.stage.stage[va as usize] = card_id;
                        if source_zone != Zone::Stage.to_str() {
                            // Rule 9.6.2.1.2.1: Track card deployed from non-stage.
                            player.track_deployment(card_id);
                        }
                        return Ok(false);
                    }
                }
                let pos_target = player_target.to_string();
                let pos_str = available_slots
                    .iter()
                    .map(|&i| match i {
                        0 => "left_side",
                        1 => "center",
                        _ => "right_side",
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                let pos_str_dbg = pos_str.clone();
                let card_no_dbg = gs
                    .card_database
                    .get_card(card_id)
                    .map_or("card", |c| c.card_no.as_ref())
                    .to_string();
                self.pending_choice = Some(Choice::SelectPosition {
                    position: pos_str,
                    description: format!(
                        "Choose position for {}",
                        gs.card_database
                            .get_card(card_id)
                            .map_or("card", |c| c.name.as_ref())
                    ),
                    description_en: Some(format!(
                        "Choose position for {}",
                        gs.card_database
                            .get_card(card_id)
                            .map_or("card", |c| c.name.as_ref())
                    )),
                    description_ja: Some(format!(
                        "{}の配置位置を選択",
                        gs.card_database
                            .get_card(card_id)
                            .map_or("カード", |c| c.name.as_ref())
                    )),
                    allow_skip: false,
                });
                let pos_str_dbg = pos_str_dbg;
                let card_id_dbg = card_id;
                self.execution_context = ExecutionContext::MoveCardsPosition {
                    card_id,
                    state_change,
                    target: pos_target,
                    source_zone: source_zone.to_string(),
                };
                log::debug!(
                    "[DEPLOY] {card_no_dbg} (id={card_id_dbg}) waits for the player to choose \
                     a position from [{pos_str_dbg}] — its 登場 fires after placement",
                );
                return Ok(true);
            } else {
                // Exactly 1 available slot (either empty or, with allow_occupied_stage, occupied)
                let slot = available_slots[0];
                if player.stage.stage[slot] != -1 {
                    // Replace existing card
                    player.waitroom.add_card(player.stage.stage[slot]);
                }
                player.stage.stage[slot] = card_id;
                if source_zone != Zone::Stage.to_str() {
                    // Rule 9.6.2.1.2.1: Track card deployed from non-stage.
                    player.track_deployment(card_id);
                }
                log::debug!(
                    "[DEPLOY] {} (id={card_id}) placed immediately at slot {slot}",
                    gs.card_database
                        .get_card(card_id)
                        .map_or("card", |c| c.card_no.as_ref()),
                    card_id = card_id,
                    slot = slot
                );
                return Ok(false);
            }
        }
        let pos_to_use = if Zone::from_str(destination) == Some(Zone::UnderMember) {
            // "メンバー1人の下に置く" — when the effect moves a NEW card (from a
            // non-stage zone like discard/hand) and freely picks WHICH stage
            // member to place it under, the player chooses. A card being displaced
            // from the stage (self-target, e.g. baton-touch under the arriver)
            // OR placed under "this member" (under_self, e.g. きな子/璃奈)
            // must NOT prompt — it auto-places under the target member.
            let from_self_displacement = Zone::from_str(source_zone) == Some(Zone::Stage)
                || source_zone.is_empty();
            let stage_members: Vec<usize> = (0..3)
                .filter(|&i| player.stage.stage[i] != -1)
                .collect();
            if !from_self_displacement && !under_self && stage_members.len() > 1 && vacated_area.is_none() {
                let desc = format!(
                    "Choose a member to place {} under",
                    gs.card_database
                        .get_card(card_id)
                        .map_or("card", |c| c.name.as_ref())
                );
                let desc_ja = format!(
                    "{}を下に置くメンバーを選択",
                    gs.card_database
                        .get_card(card_id)
                        .map_or("カード", |c| c.name.as_ref())
                );
                self.pending_choice = Some(
                    Choice::select_cards(Zone::Stage.to_str(), 1, desc, false)
                        .description_ja(Some(desc_ja))
                        .destination(Some(Zone::UnderMember.to_str().to_string()))
                        .target_player_id(Some(player_target.to_string()))
                        .build(),
                );
                self.execution_context = ExecutionContext::MoveCardsPosition {
                    card_id,
                    state_change: state_change.clone(),
                    target: player_target.to_string(),
                    source_zone: source_zone.to_string(),
                };
                return Ok(true);
            }
            // Use the resolver's stored activating_card_id first (survives choice
            // pauses and ability queue transitions), then fall back to gs.activating_card.
            let member_card = self.activating_card_id.or(activating_card);
            member_card
                .and_then(|cid| player.stage.stage.iter().position(|&id| id == cid))
                .or(vacated_area.map(|v| v as usize))
                .or_else(|| {
                    self.moved_cards
                        .iter()
                        .rev()
                        .find_map(|&cid| player.stage.stage.iter().position(|&id| id == cid))
                })
        } else if Zone::from_str(destination) == Some(Zone::Deck)
            || Zone::from_str(destination) == Some(Zone::DeckTop)
        {
            deck_position.or(vacated_area.map(|v| v as usize))
        } else {
            vacated_area.map(|v| v as usize)
        };
        log::debug!(
            "[TRACE_PLACE] dest={} card={} pos_to_use={:?} vacated_area={:?} stage_before={:?}",
            destination,
            card_id,
            pos_to_use,
            vacated_area,
            player.stage.stage
        );
        util::place_card_in_zone(player, card_id, destination, pos_to_use, is_max, count);
        log::debug!("[TRACE_PLACE] stage_after={:?}", player.stage.stage);
        Ok(false)
    }

    fn take_cards_from_standard_zone(
        &mut self,
        player: &mut Player,
        card_db: &CardDatabase,
        zone_name: &str,
        filter: &util::CardFilter,
        count: usize,
        is_all: bool,
        behavior: util::InsufficientBehavior,
        can_skip: bool,
        effect: &AbilityEffect,
        activating_card_id: Option<i16>,
    ) -> Result<Option<Vec<i16>>, String> {
        let cards = util::zone_card_ids(player, zone_name);
        log::debug!(
            "[TAKE] zone={zone_name} excl_chars={:?} chars={:?} group={:?} cards={:?}",
            filter.exclude_characters,
            filter.characters,
            filter.group,
            cards
        );
        let filtered_indices = util::matching_indices(&cards, card_db, filter, false);

        match util::resolve_selection(
            &cards,
            card_db,
            activating_card_id,
            count,
            is_all,
            filter,
            effect.is_self_target(),
            behavior,
            false,
        )? {
            util::SelectionOutcome::Exact(indices) if can_skip && !indices.is_empty() => {
                if is_all {
                    // all=true + optional: auto-take all matching cards; the optional
                    // "yes/no" is handled at a higher level (conditional_on_result etc).
                    let taken = util::zone_remove_at_indices(player, zone_name, &indices);
                    Ok(Some(taken))
                } else {
                    // Per-count optional: prompt so the user can choose which/skip.
                    self.prompt_card_selection(
                        zone_name,
                        indices.len(),
                        can_skip,
                        effect,
                        filter,
                        Some(filtered_indices),
                    );
                    Ok(None)
                }
            }
            util::SelectionOutcome::Exact(indices) => {
                let taken = util::zone_remove_at_indices(player, zone_name, &indices);
                Ok(Some(taken))
            }
            util::SelectionOutcome::Prompt => {
                self.prompt_card_selection(
                    zone_name,
                    count,
                    can_skip,
                    effect,
                    filter,
                    Some(filtered_indices),
                );
                Ok(None)
            }
            util::SelectionOutcome::Skip => {
                if can_skip && !filtered_indices.is_empty() {
                    self.prompt_card_selection(
                        zone_name,
                        filtered_indices.len(),
                        can_skip,
                        effect,
                        filter,
                        Some(filtered_indices),
                    );
                    Ok(None)
                } else {
                    Ok(Some(vec![]))
                }
            }
        }
    }

    /// Resolve cards from the source zone based on the effect configuration.
    /// Returns the list of card IDs taken from the source.
    fn resolve_cards_from_source(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        count: usize,
        card_type_filter: Option<&str>,
        group_name: Option<&str>,
        cost_limit: Option<u8>,
        cost_total: Option<u8>,
        cost_total_operator: Option<&str>,
        character_filter: Option<&Vec<String>>,
        name_fragments: Option<&Vec<String>>,
        is_self_cost: bool,
        is_max: bool,
        is_all: bool,
        exclude_self: bool,
        activating_card_id: Option<i16>,
        use_p2: bool,
        source: &str,
        destination: &str,
        card_db: &crate::card::CardDatabase,
    ) -> Result<Vec<i16>, String> {
        let player = if use_p2 {
            &mut gs.player2
        } else {
            &mut gs.player1
        };

        // Get cards from source
        let source_str = if source.is_empty() {
            // No source specified in the parsed ability (e.g. auto abilities
            // that trigger on zone transitions like "when X goes to waitroom").
            // The card is now in the discard/waitroom — search there.
            "discard"
        } else {
            source
        };
        if !self.selected_cards.is_empty()
            && Zone::from_str(source_str) == Some(Zone::SelectedCards)
        {
            let selected = self.selected_cards.clone();
            for &card_id in &selected {
                remove_card_from_any_zone(player, &mut gs.last_vacated_stage_area, card_id);
            }
            return Ok(selected.to_vec());
        }

        if source_str == "recently_moved" {
            return self.resolve_from_recently_moved(
                gs,
                card_type_filter,
                group_name,
                card_db,
                use_p2,
            );
        }
        // "それを手札に加える" follow-ups ("if it's X, add IT to hand") — the
        // source is the SPECIFIC card the preceding sequential step moved, not
        // any matching card in a zone. Pull from self.moved_cards (the current
        // sequential's own moves) filtered by the action's character filter.
        if source_str == "preceding_moved" {
            let cards: Vec<i16> = self
                .moved_cards
                .iter()
                .filter(|&&cid| {
                    cid != -1
                        && character_filter.map_or(true, |cf| {
                            util::card_matches_characters(card_db, cid, Some(cf))
                        })
                })
                .copied()
                .collect();
            for &card_id in &cards {
                remove_card_from_any_zone(player, &mut gs.last_vacated_stage_area, card_id);
            }
            return Ok(cards);
        }
        if source_str == "looked_at_remaining" {
            return self.resolve_from_looked_at(gs, use_p2);
        }
        if source_str == "revealed_cards" {
            return self.resolve_from_revealed_cards(
                gs,
                count,
                is_all,
                is_max,
                effect,
                card_type_filter,
                group_name,
                cost_limit,
                cost_total,
                cost_total_operator,
                character_filter,
                name_fragments,
                card_db,
            );
        }
        if source_str == "those_cards" {
            if let Some(result) = self.resolve_from_those_cards(
                gs,
                count,
                card_type_filter,
                group_name,
                destination,
                effect,
                use_p2,
                card_db,
            )? {
                return Ok(result);
            }
            // resolve_from_those_cards returned None — no explicit trigger_moved_cards
            // was recorded (e.g. legacy trigger_auto_ability with None). Fall through
            // to the discard-pile resolution below (the historic Q252 pick-card flow).
        }
        let effective_source = if source_str == "those_cards" {
            Zone::Discard.to_str()
        } else {
            source_str
        };
        self.resolve_from_zone(
            gs,
            MoveSourceContext {
                effective_source,
                source_str,
                count,
                effect,
                card_type_filter,
                group_name,
                cost_limit,
                cost_total,
                cost_total_operator,
                character_filter,
                name_fragments,
                is_self_cost,
                is_max,
                is_all,
                exclude_self,
                activating_card_id,
                use_p2,
                destination,
                card_db,
            },
        )
    }

    fn resolve_from_recently_moved(
        &mut self,
        gs: &mut GameState,
        card_type_filter: Option<&str>,
        group_name: Option<&str>,
        card_db: &crate::card::CardDatabase,
        use_p2: bool,
    ) -> Result<Vec<i16>, String> {
        let player = if use_p2 {
            &mut gs.player2
        } else {
            &mut gs.player1
        };
        // Baton touch / cost payment — target the card(s) just moved.
        // This ensures "the card placed by this baton touch" actually
        // refers to the specific card that was moved, not any matching
        // card from the zone.
        let cards: Vec<i16> = gs
            .recently_moved_cards
            .clone()
            .unwrap_or_default()
            .into_iter()
            .filter(|&cid| {
                let ty = card_type_filter.unwrap_or("");
                ty.is_empty() || util::card_matches_type(card_db, cid, Some(ty))
            })
            .filter(|&cid| {
                group_name.is_none() || util::card_matches_group_str(card_db, cid, group_name)
            })
            .collect();
        for &card_id in &cards {
            remove_card_from_any_zone(player, &mut gs.last_vacated_stage_area, card_id);
        }
        // Card left any zone → full zone-exit cleanup (rule 4.1.4)
        gs.on_cards_left_zones(&cards);
        Ok(cards)
    }

    fn resolve_from_looked_at(
        &mut self,
        gs: &mut GameState,
        use_p2: bool,
    ) -> Result<Vec<i16>, String> {
        let player = if use_p2 {
            &mut gs.player2
        } else {
            &mut gs.player1
        };
        let cards: Vec<i16> = gs.looked_at_cards.drain(..).collect();
        for &card in &cards {
            player.waitroom.add_card(card);
        }
        Ok(cards)
    }

    fn resolve_from_revealed_cards(
        &mut self,
        gs: &mut GameState,
        count: usize,
        is_all: bool,
        is_max: bool,
        effect: &AbilityEffect,
        card_type_filter: Option<&str>,
        group_name: Option<&str>,
        cost_limit: Option<u8>,
        cost_total: Option<u8>,
        cost_total_operator: Option<&str>,
        character_filter: Option<&Vec<String>>,
        name_fragments: Option<&Vec<String>>,
        card_db: &crate::card::CardDatabase,
    ) -> Result<Vec<i16>, String> {
        let take_count = if is_all {
            gs.revealed_cards.len()
        } else {
            count.min(gs.revealed_cards.len())
        };
        let can_skip = is_max || effect.optional.unwrap_or(false);
        let filter = util::filter_from_parts_full(
            card_type_filter,
            group_name,
            cost_limit,
            None, // cost_operator
            character_filter,
            name_fragments,
            None, // distinct
            None, // exclude_self
            cost_total,
            cost_total_operator,
            effect.exclude_characters_any(),
        );
        let neg = effect.negation_any().unwrap_or(false);
        let matching: Vec<usize> = (0..gs.revealed_cards.len())
            .filter(|&i| {
                let id = gs.revealed_cards[i];
                if !filter.matches(card_db, id, false) {
                    return false;
                }
                if let Some(prop) = effect.card_property_any().as_deref() {
                    let has_prop = match prop {
                        "has_blade_heart" => {
                            card_db.get_card(id).is_some_and(|c| c.has_blade_heart())
                        }
                        "has_score_icon" => {
                            card_db.get_card(id).is_some_and(|c| c.has_score_icon())
                        }
                        _ => false,
                    };
                    if neg {
                        if has_prop {
                            return false;
                        }
                    } else {
                        if !has_prop {
                            return false;
                        }
                    }
                }
                true
            })
            .collect();
        if matching.is_empty() {
            return Ok(vec![]);
        }
        if take_count < matching.len() || can_skip {
            self.prompt_card_selection(
                "revealed_cards",
                take_count,
                can_skip,
                effect,
                &filter,
                Some(matching),
            );
            return Ok(vec![]);
        }
        let actual_take = take_count.min(matching.len());
        let taken: Vec<i16> = matching[..actual_take]
            .iter()
            .rev()
            .map(|&i| {
                let id = gs.revealed_cards[i];
                gs.revealed_cards.remove(i);
                id
            })
            .collect();
        gs.remove_from_source_hands(&taken);
        // Remove from deck too — the reveal only peeked, not drained.
        // If the card was from a deck-top reveal, it's still in the
        // deck and must be removed here to avoid duplication.
        for &id in &taken {
            if let Some(pos) = gs.player1.main_deck.cards.iter().position(|&c| c == id) {
                gs.player1.main_deck.cards.remove(pos);
            } else if let Some(pos) = gs.player2.main_deck.cards.iter().position(|&c| c == id) {
                gs.player2.main_deck.cards.remove(pos);
            }
        }
        // Remove from waitroom too — yell cards went to waitroom after
        // check_live_success drained the resolution zone (Rule 8.4.7).
        for &id in &taken {
            if let Some(pos) = gs.player1.waitroom.cards.iter().position(|&c| c == id) {
                gs.player1.waitroom.cards.remove(pos);
            } else if let Some(pos) = gs.player2.waitroom.cards.iter().position(|&c| c == id) {
                gs.player2.waitroom.cards.remove(pos);
            }
        }
        Ok(taken)
    }

    fn resolve_from_those_cards(
        &mut self,
        gs: &mut GameState,
        count: usize,
        card_type_filter: Option<&str>,
        group_name: Option<&str>,
        destination: &str,
        effect: &AbilityEffect,
        use_p2: bool,
        card_db: &crate::card::CardDatabase,
    ) -> Result<Option<Vec<i16>>, String> {
        // Handle "those_cards" alias: resolve to the cards that triggered the
        // each_time, captured as `trigger_moved_cards` on THIS queue entry at
        // enqueue time (the authoritative "cards that triggered me"). If no
        // explicit snapshot was recorded (None), return None so the caller can
        // fall through to the legacy discard-pile resolution.
        let trigger_cards = gs
            .ability_queue
            .current_entry()
            .and_then(|e| e.trigger_moved_cards.clone())
            .filter(|c| !c.is_empty());
        if let Some(trigger_cards) = trigger_cards {
            if !trigger_cards.is_empty() {
                if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed)
                {
                    log::debug!(
                        "[THOSE_CARDS] trigger_cards={:?} count={}",
                        trigger_cards,
                        count
                    );
                }
                let mut all_matching: Vec<i16> = Vec::new();
                for &cid in &trigger_cards {
                    if card_type_filter
                        .map_or(true, |ct| util::card_matches_type(card_db, cid, Some(ct)))
                        && group_name.map_or(true, |gn| {
                            util::card_matches_group_str(card_db, cid, Some(gn))
                        })
                    {
                        all_matching.push(cid);
                    }
                }
                if all_matching.is_empty() {
                    // The trigger moved cards, but none matched the filters
                    // (e.g. no 虹ヶ咲 live card among the milled batch). "Those
                    // cards" means ONLY the moved cards — nothing qualifies, so
                    // the move adds nothing. Signal this explicitly so a
                    // following "…したとき" modify_score step is skipped, and do
                    // NOT fall through to the discard pile.
                    self.last_move_moved_any = Some(false);
                    return Ok(Some(vec![]));
                } else if all_matching.len() <= count as usize {
                    // Exactly `count` or fewer match — take them directly.
                    let found = all_matching[..count.min(all_matching.len())].to_vec();
                    if !effect.optional.unwrap_or(false) {
                        let player = if use_p2 {
                            &mut gs.player2
                        } else {
                            &mut gs.player1
                        };
                        for &cid in &found {
                            if let Some(pos) = player.waitroom.cards.iter().position(|&c| c == cid)
                            {
                                player.waitroom.cards.remove(pos);
                            }
                        }
                    }
                    self.last_move_moved_any = Some(!found.is_empty());
                    return Ok(Some(found));
                } else if &*destination == "deck_top_or_bottom" {
                    // Q252: more matching cards than count, player chooses which one.
                    // Directly create a SelectCard choice restricted to the
                    // trigger_moved_cards' positions in the waitroom.
                    let player = if use_p2 {
                        &mut gs.player2
                    } else {
                        &mut gs.player1
                    };
                    let filtered_indices: Vec<usize> = {
                        let mut indices = Vec::new();
                        for &cid in &all_matching {
                            for (i, &wc) in player.waitroom.cards.iter().enumerate() {
                                if wc == cid && !indices.contains(&i) {
                                    indices.push(i);
                                }
                            }
                        }
                        indices
                    };
                    let description = card_type_filter
                        .and_then(|_| group_name)
                        .map(|g| format!("Select 1 {g} card to place on deck"))
                        .unwrap_or_else(|| "Select 1 card to place on deck".to_string().into());
                    let description_ja = card_type_filter
                        .and_then(|_| group_name)
                        .map(|g| format!("{g}カードを山札に置く1枚を選択"))
                        .unwrap_or_else(|| "山札に置く1枚を選択".to_string().into());
                    self.pending_choice = Some(
                        Choice::select_cards(Zone::Discard.to_str(), 1, description, false)
                            .description_ja(Some(description_ja))
                            .card_type(card_type_filter.map(|s| s.to_string()))
                            .group(group_name.map(|s| s.to_string()))
                            .filtered_indices(Some(filtered_indices))
                            .target_player_id(Some("self".to_string()))
                            .build(),
                    );
                    return Ok(Some(vec![]));
                } else {
                    // More matching than count — player must choose which cards.
                    // Show a SelectCard choice restricted to the trigger_moved_cards'
                    // positions in the waitroom, regardless of destination.
                    let player = if use_p2 {
                        &mut gs.player2
                    } else {
                        &mut gs.player1
                    };
                    let filtered_indices: Vec<usize> = {
                        let mut indices = Vec::new();
                        for &cid in &all_matching {
                            for (i, &wc) in player.waitroom.cards.iter().enumerate() {
                                if wc == cid && !indices.contains(&i) {
                                    indices.push(i);
                                }
                            }
                        }
                        indices
                    };
                    let description = card_type_filter
                        .and_then(|_| group_name)
                        .map(|g| format!("Select {count} {g} card(s)"))
                        .unwrap_or_else(|| "Select card(s)".to_string().into());
                    let description_ja = card_type_filter
                        .and_then(|_| group_name)
                        .map(|g| format!("{g}カードを{count}枚選択"))
                        .unwrap_or_else(|| "カードを選択".to_string().into());
                    self.pending_choice = Some(
                        Choice::select_cards(
                            Zone::Discard.to_str(),
                            count,
                            description,
                            effect.optional.unwrap_or(false),
                        )
                        .description_ja(Some(description_ja))
                        .card_type(card_type_filter.map(|s| s.to_string()))
                        .group(group_name.map(|s| s.to_string()))
                        .filtered_indices(Some(filtered_indices))
                        .target_player_id(Some("self".to_string()))
                        .build(),
                    );
                    return Ok(Some(vec![]));
                }
            }
        }
        Ok(None)
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_from_zone(
        &mut self,
        gs: &mut GameState,
        c: MoveSourceContext,
    ) -> Result<Vec<i16>, String> {
        match Zone::from_str(c.effective_source) {
            Some(Zone::Deck) | Some(Zone::DeckTop) => self.resolve_from_deck(gs, &c),
            Some(Zone::DeckBottom) => self.resolve_from_deck_bottom(gs, &c),
            Some(Zone::EnergyDeck) => self.resolve_from_energy_deck(gs, &c),
            Some(Zone::Stage) => self.resolve_from_stage(gs, &c),
            Some(Zone::Hand)
            | Some(Zone::Discard)
            | Some(Zone::Energy)
            | Some(Zone::LiveCardZone)
            | Some(Zone::SuccessLiveZone) => self.resolve_from_standard_zone(gs, &c),
            Some(Zone::LookedAt) => self.resolve_source_looked_at(gs, &c),
            Some(Zone::SelectedCards) => self.resolve_from_selected_cards(gs, &c),
            Some(Zone::RevealedCards) => self.resolve_source_revealed_cards(gs, &c),
            Some(Zone::UnderMember) => self.resolve_from_under_member(gs, &c),
            _ => Err(format!("Unknown source zone: {}", c.source_str)),
        }
    }

    fn resolve_from_deck(
        &mut self,
        gs: &mut GameState,
        c: &MoveSourceContext,
    ) -> Result<Vec<i16>, String> {
        let player = if c.use_p2 {
            &mut gs.player2
        } else {
            &mut gs.player1
        };
        let count = c.count;
        let card_db = c.card_db;
        if c.effect.optional.unwrap_or(false) {
            let entry = gs.ability_queue.current_entry();
            let decided = entry
                .as_ref()
                .and_then(|e| e.conditional_choice.as_ref())
                .is_some();
            if !decided {
                if let Some(entry_mut) = gs.ability_queue.current_entry_mut() {
                    entry_mut.choice_card_no =
                        Some(crate::ability::types::ChoiceRoute::Raw(
                            "pay_optional_cost".to_string(),
                        ));
                }
                self.pending_choice = Some(Choice::SelectTarget {
                    target: "pay_optional_cost:skip_optional_cost".to_string(),
                    description: "Place top card of deck to waiting room?".to_string(),
                    description_en: Some("Place top card of deck to waiting room?".to_string()),
                    description_ja: Some("山札の上を控え室に置きますか？".to_string()),
                    allow_skip: true,
                    options: Some(vec!["No".to_string(), "Yes".to_string()]),
                });
                return Ok(vec![]);
            }
        }
        let mut drawn = Vec::new();
        let mut attempts = 0u8;
        let mut remaining = count;
        while remaining > 0
            && attempts < (count as u8 + player.main_deck.cards.len() as u8 + 10)
        {
            // Q104 / Rule 10.2.1: deck empty mid-draw → refresh from waitroom
            // and continue. This handles deck-to-discard costs/effects when the
            // deck has fewer cards than needed. E.g. deck=2, need=3:
            //   [1] draw 2 from deck → deck is empty
            //   [2] flush drawn to waitroom, refresh (shuffle waitroom into deck)
            //   [3] draw remaining 1 from new deck
            if player.main_deck.cards.is_empty() && !player.waitroom.cards.is_empty() {
                // Move already-drawn cards to waitroom so refresh includes them
                player.waitroom.cards.extend(drawn.drain(..));
                player.refresh();
            }
            if let Some(card) = player.main_deck.draw() {
                attempts += 1;
                if !util::card_matches_type(card_db, card, c.card_type_filter) {
                    player.main_deck.cards.push(card);
                    continue;
                }
                if !util::card_matches_group_str(card_db, card, c.group_name) {
                    player.main_deck.cards.push(card);
                    continue;
                }
                drawn.push(card);
                remaining = remaining.saturating_sub(1);
            } else {
                // Both deck and waitroom are empty — cannot draw more
                break;
            }
        }
        Ok(drawn)
    }

    fn resolve_from_deck_bottom(
        &mut self,
        gs: &mut GameState,
        c: &MoveSourceContext,
    ) -> Result<Vec<i16>, String> {
        let player = if c.use_p2 {
            &mut gs.player2
        } else {
            &mut gs.player1
        };
        let count = c.count;
        if c.effect.optional.unwrap_or(false) {
            let entry = gs.ability_queue.current_entry();
            let decided = entry
                .as_ref()
                .and_then(|e| e.conditional_choice.as_ref())
                .is_some();
            if !decided {
                if let Some(entry_mut) = gs.ability_queue.current_entry_mut() {
                    entry_mut.choice_card_no =
                        Some(crate::ability::types::ChoiceRoute::Raw(
                            "pay_optional_cost".to_string(),
                        ));
                }
                self.pending_choice = Some(Choice::SelectTarget {
                    target: "pay_optional_cost:skip_optional_cost".to_string(),
                    description: "Place bottom card of deck to waiting room?".to_string(),
                    description_en: Some("Place bottom card of deck to waiting room?".to_string()),
                    description_ja: Some("山札の下を控え室に置きますか？".to_string()),
                    allow_skip: true,
                    options: Some(vec!["No".to_string(), "Yes".to_string()]),
                });
                return Ok(vec![]);
            }
        }
        let mut drawn = Vec::new();
        for _i in 0..count {
            if let Some(card) = player.main_deck.draw_bottom() {
                drawn.push(card);
            } else {
                break;
            }
        }
        Ok(drawn)
    }

    fn resolve_from_energy_deck(
        &mut self,
        gs: &mut GameState,
        c: &MoveSourceContext,
    ) -> Result<Vec<i16>, String> {
        let player = if c.use_p2 {
            &mut gs.player2
        } else {
            &mut gs.player1
        };
        let count = c.count;
        let mut drawn = Vec::new();
        for _i in 0..count {
            if let Some(card) = player.energy_deck.draw() {
                drawn.push(card);
            } else {
                break;
            }
        }
        Ok(drawn)
    }

    fn resolve_from_stage(
        &mut self,
        gs: &mut GameState,
        c: &MoveSourceContext,
    ) -> Result<Vec<i16>, String> {
        let player = if c.use_p2 {
            &mut gs.player2
        } else {
            &mut gs.player1
        };
        let card_db = c.card_db;
        let effect = c.effect;
        if c.is_self_cost {
            let idx = c
                .activating_card_id
                .and_then(|act_id| player.stage.stage.iter().position(|&id| id == act_id))
                .ok_or_else(|| "Activating card not found at stage".to_string())?;
            gs.last_vacated_stage_area = Some(idx as u8);
            if c.destination != "same_area" {
                Ok(player
                    .remove_member_from_stage_with_recycling(idx, card_db)
                    .into_iter()
                    .collect())
            } else {
                gs.last_vacated_stage_area = None;
                Ok(vec![player.stage.stage[idx]])
            }
        } else {
            let filter = util::filter_from_parts_full(
                c.card_type_filter,
                c.group_name,
                c.cost_limit,
                None,
                c.character_filter,
                c.name_fragments,
                None,
                if c.exclude_self {
                    c.activating_card_id
                } else {
                    None
                },
                c.cost_total,
                c.cost_total_operator,
                c.effect.exclude_characters_any(),
            );
            match util::resolve_selection(
                &player.stage.stage,
                card_db,
                c.activating_card_id,
                c.count,
                c.is_all,
                &filter,
                effect.is_self_target(),
                util::InsufficientBehavior::Silent,
                true,
            )? {
                util::SelectionOutcome::Exact(indices) => {
                    let mut vacated = None;
                    let cards = indices
                        .iter()
                        .rev()
                        .filter_map(|&i| {
                            let cid =
                                player.remove_member_from_stage_with_recycling(i, card_db);
                            if cid.is_some() {
                                vacated = Some(i as u8);
                            }
                            cid
                        })
                        .collect();
                    gs.last_vacated_stage_area = vacated;
                    log::debug!(
                        "[STAGE_EXACT] moved cards={:?} vacated={:?} last_vacated={:?}",
                        cards,
                        vacated,
                        gs.last_vacated_stage_area
                    );
                    Ok(cards)
                }
                util::SelectionOutcome::Prompt => {
                    let filter = util::filter_from_parts_full(
                        c.card_type_filter,
                        c.group_name,
                        c.cost_limit,
                        None,
                        c.character_filter,
                        c.name_fragments,
                        None,
                        if c.exclude_self {
                            c.activating_card_id
                        } else {
                            None
                        },
                        c.cost_total,
                        c.cost_total_operator,
                        c.effect.exclude_characters_any(),
                    );
                    let stage_indices: Vec<usize> = (0..player.stage.stage.len())
                        .filter(|&i| filter.matches(card_db, player.stage.stage[i], true))
                        .collect();
                    self.prompt_card_selection(
                        Zone::Stage.to_str(),
                        c.count,
                        false,
                        effect,
                        &filter,
                        Some(stage_indices),
                    );
                    Ok(vec![])
                }
                util::SelectionOutcome::Skip => Ok(vec![]),
            }
        }
    }

    fn resolve_from_standard_zone(
        &mut self,
        gs: &mut GameState,
        c: &MoveSourceContext,
    ) -> Result<Vec<i16>, String> {
        let player = if c.use_p2 {
            &mut gs.player2
        } else {
            &mut gs.player1
        };
        let card_db = c.card_db;
        let effect = c.effect;
        let src_zone = Zone::from_str(c.effective_source);
        let actual_zone = c.effective_source;

        let insufficient_behavior = match src_zone {
            Some(Zone::Energy) => {
                // A move_cards EFFECT (e.g. "エネルギー1枚をエネルギーデッキに置く")
                // fizzles silently when the energy zone is empty, and the
                // enclosing sequential ("その後") continues with later steps.
                util::InsufficientBehavior::Silent
            }
            Some(Zone::LiveCardZone) => util::InsufficientBehavior::Error(
                "Not enough cards in live card zone".to_string(),
            ),
            Some(Zone::SuccessLiveZone) => util::InsufficientBehavior::Silent,
            _ => util::InsufficientBehavior::Silent,
        };

        let can_skip = match src_zone {
            Some(Zone::Discard) => c.is_max || effect.optional.unwrap_or(false),
            Some(Zone::Hand) => {
                effect.optional.unwrap_or(false) || effect.any_number_any().unwrap_or(false)
            }
            Some(Zone::SuccessLiveZone) => effect.optional.unwrap_or(false),
            _ => false,
        };

        let pass_is_all = match src_zone {
            Some(Zone::Hand) | Some(Zone::Discard) | Some(Zone::Energy) => c.is_all,
            _ => false,
        };

        let has_effect_groups = effect
            .group_names_any()
            .as_ref()
            .map_or(false, |g| !g.is_empty());
        let filter_group_name = if has_effect_groups { None } else { c.group_name };
        let mut filter = util::filter_from_parts_full(
            if src_zone == Some(Zone::LiveCardZone)
                || (src_zone == Some(Zone::SuccessLiveZone) && c.card_type_filter.is_some())
            {
                Some("live_card")
            } else if src_zone == Some(Zone::SuccessLiveZone) {
                None
            } else {
                c.card_type_filter
            },
            if src_zone == Some(Zone::Energy) {
                None
            } else {
                filter_group_name
            },
            if src_zone == Some(Zone::Energy) {
                None
            } else {
                c.cost_limit
            },
            if src_zone == Some(Zone::Discard) {
                effect.cost_limit_operator_any().map(Operator::as_str)
            } else {
                None
            },
            c.character_filter,
            if matches!(src_zone, Some(Zone::Hand) | Some(Zone::Discard)) {
                c.name_fragments
            } else {
                None
            },
            None,
            None,
            if matches!(src_zone, Some(Zone::Hand) | Some(Zone::Discard)) {
                c.cost_total
            } else {
                None
            },
            if matches!(src_zone, Some(Zone::Hand) | Some(Zone::Discard)) {
                c.cost_total_operator
            } else {
                None
            },
            effect.exclude_characters_any(),
        );
        filter.need_heart_total = effect.need_heart_total_any();
        let nho_binding = effect.need_heart_operator_any();
        filter.need_heart_operator = nho_binding.as_deref();
        let nhc_binding = effect.need_heart_color_any();
        filter.need_heart_color = nhc_binding.as_deref();
        filter.heart_colors = effect.heart_colors_any();
        if has_effect_groups {
            filter.groups = effect.group_names_any();
        }
        let cp_binding = effect.card_property_any();
        filter.card_property = cp_binding.as_deref();
        // The property's polarity lives in `negation` (「ブレードハートを
        // 持たない」→ negation=true). Forgetting it inverted emma bp7-008's
        // eligibility: blade-heart holders were offered instead of excluded.
        filter.negation = effect.negation_any().unwrap_or(false);

        // ── group_reference: "different_group_names" ──────────────────
        // Q89: Multi-name cards have the group name(s) from their series.
        // Q105: A multi-name card contributes ONE constituent group for
        //   "different group names" conditions.
        // Q208: When checking "different from ALL members on stage", all
        //   groups a multi-name card can match via its series are blocked.
        // → Exclude discard cards whose group matches ANY stage member's group.
        // Only apply stage-group exclusion when the effect itself has
        // group_reference and there's no condition with the same reference.
        // Bring the LOVE ab#1 has effect.group_reference without a condition
        // (genuine stage-group exclusion).  Mia option 1 has BOTH
        // condition.group_reference and effect.group_reference — the
        // condition already handles different-group counting; we should
        // NOT also exclude stage groups (which would over-filter).
        let cond_has_grp = effect
            .condition
            .as_ref()
            .and_then(|cd| {
                if cd.get_group_reference() == Some("different_group_names") {
                    Some(true)
                } else {
                    None
                }
            })
            .unwrap_or(false);
        if !cond_has_grp
            && effect.group_reference_any().as_deref() == Some("different_group_names")
            && c.source_str == "discard"
        {
            let mut stage_groups: SmallVec<[String; 8]> = SmallVec::new();
            for &cid in &player.stage.stage {
                if cid == -1 {
                    continue;
                }
                if let Some(card) = card_db.get_card(cid) {
                    if !card.group.is_empty() {
                        if !stage_groups.contains(&card.group.to_string()) {
                            stage_groups.push(card.group.to_string());
                        }
                    } else {
                        for known_group in ["μ's", "Aqours", "虹ヶ咲", "Liella!", "蓮ノ空"] {
                            if util::card_matches_group_str(card_db, cid, Some(known_group)) {
                                let s = known_group.to_string();
                                if !stage_groups.contains(&s) {
                                    stage_groups.push(s);
                                }
                            }
                        }
                    }
                }
            }
            if !stage_groups.is_empty() {
                let groups_vec: Vec<String> = stage_groups.into_iter().collect();
                // This &'static leak persists for the game's lifetime.
                let leaked = Vec::leak(groups_vec);
                filter.exclude_group_names = Some(leaked);
            }
        }

        log::debug!(
            "[NEED_HEART] filter: color={:?} total={:?} op={:?} src={:?} card_type={:?}",
            filter.need_heart_color,
            filter.need_heart_total,
            filter.need_heart_operator,
            c.source_str,
            c.card_type_filter
        );

        match self.take_cards_from_standard_zone(
            player,
            card_db,
            actual_zone,
            &filter,
            c.count,
            pass_is_all,
            insufficient_behavior,
            can_skip,
            effect,
            c.activating_card_id,
        )? {
            Some(cards) => Ok(cards),
            None => Ok(vec![]),
        }
    }

    fn resolve_source_looked_at(
        &mut self,
        gs: &mut GameState,
        c: &MoveSourceContext,
    ) -> Result<Vec<i16>, String> {
        let card_db = c.card_db;
        let count = c.count;
        let effect = c.effect;
        let matching: Vec<usize> = (0..gs.looked_at_cards.len())
            .filter(|&i| {
                let cid = gs.looked_at_cards[i];
                util::card_matches_type(card_db, cid, c.card_type_filter)
                    && util::card_matches_group_str(card_db, cid, c.group_name)
                    && util::card_matches_cost_limit(card_db, cid, c.cost_limit)
            })
            .collect();

        if matching.is_empty() {
            Ok(vec![])
        } else if effect.optional.unwrap_or(false) && count > 0 {
            // Optional discard: card selection with user-friendly description
            let max_take = count.min(matching.len());
            let description = format!("Discard up to {} looked-at card(s)?", max_take);
            let description_en = Some(description.clone());
            let description_ja = Some(if max_take == 1 {
                "見たカードを控え室に置きますか？".to_string()
            } else {
                format!("見たカードを最大{}枚まで控え室に置きますか？", max_take)
            });
            let mut filter = util::CardFilter::from_effect(effect);
            // Resolve dynamic cost limit reference (e.g. "previous_moved_card" + offset)
            if let Ok(resolved) = self.resolve_cost_limit_reference(gs, effect) {
                if let Some(cost) = resolved {
                    filter.cost_limit = Some(cost);
                }
            }
            // 「デッキの上からN番目に置いてもよい」: the answer-time handler
            // (handle_select_cards_looked_at) has no access to this effect, so
            // capture the numeric deck position now and let it ride on the
            // resolver until the selection is answered.
            self.looked_at_deck_position = effect
                .position_any()
                .and_then(|pi| pi.get_position())
                .and_then(|s| s.parse::<usize>().ok())
                .filter(|&n| n > 0)
                .filter(|_| effect.destination.as_ref() == Some(&Zone::DeckTop));
            self.pending_choice = Some(
                Choice::select_cards(Zone::LookedAt.to_str(), max_take, description, true)
                    .description_en(description_en)
                    .description_ja(description_ja)
                    .card_type(filter.card_type.map(|s| s.to_string()))
                    .cost_limit(
                        filter.cost_limit,
                        effect.cost_limit_operator_any().map(|s| s.to_string()),
                    )
                    .cost_total(
                        filter.cost_total,
                        effect.cost_total_operator_any().map(|s| s.to_string()),
                    )
                    .group(filter.group.map(|s| s.to_string()))
                    .characters(filter.characters.map(|v| v.to_vec()))
                    .target_player_id(Some(
                        effect.target.as_deref().unwrap_or("self").to_string(),
                    ))
                    .destination(effect.destination.clone().map(|s| s.to_string()))
                    .discard_remaining(effect.discard_remaining_any())
                    .build(),
            );
            self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
            Ok(vec![])
        } else {
            // For LookedAt, cards are ordered: [0] = matched target,
            // remaining = revealed but non-matching. Take first `count`
            // without prompting, since the order is meaningful.
            let take = if c.is_all {
                matching.len()
            } else {
                count.min(matching.len())
            };
            let mut taken: Vec<i16> = matching[..take]
                .iter()
                .rev()
                .map(|&i| gs.looked_at_cards.remove(i))
                .collect();
            taken.reverse();
            Ok(taken)
        }
    }

    fn resolve_from_selected_cards(
        &mut self,
        gs: &mut GameState,
        c: &MoveSourceContext,
    ) -> Result<Vec<i16>, String> {
        let player = if c.use_p2 {
            &mut gs.player2
        } else {
            &mut gs.player1
        };
        let effect = c.effect;
        let count = c.count;
        let selected = self.selected_cards.clone();
        let idxs: Vec<usize> = (0..selected.len()).collect();
        match util::classify_selection(
            &idxs,
            count,
            c.is_all,
            util::InsufficientBehavior::Silent,
        )? {
            util::SelectionOutcome::Exact(indices) => {
                let taken: Vec<i16> = indices.iter().map(|&i| selected[i]).collect();
                for &card_id in &taken {
                    remove_card_from_any_zone(
                        player,
                        &mut gs.last_vacated_stage_area,
                        card_id,
                    );
                }
                // Card left any zone → full zone-exit cleanup (rule 4.1.4)
                gs.on_cards_left_zones(&taken);
                Ok(taken)
            }
            util::SelectionOutcome::Prompt => {
                let mut filter = util::CardFilter::from_effect(effect);
                // Resolve dynamic cost limit reference (e.g. "previous_moved_card" + offset)
                if let Ok(resolved) = self.resolve_cost_limit_reference(gs, effect) {
                    if let Some(cost) = resolved {
                        filter.cost_limit = Some(cost);
                    }
                }
                self.prompt_card_selection(
                    Zone::SelectedCards.to_str(),
                    count,
                    false,
                    effect,
                    &filter,
                    None,
                );
                Ok(vec![])
            }
            util::SelectionOutcome::Skip => Ok(vec![]),
        }
    }

    fn resolve_source_revealed_cards(
        &mut self,
        gs: &mut GameState,
        c: &MoveSourceContext,
    ) -> Result<Vec<i16>, String> {
        let count = c.count;
        let player = if c.use_p2 {
            &mut gs.player2
        } else {
            &mut gs.player1
        };
        // Drain from source to prevent card duplication.
        // (line 619 cloned before, leaving originals in cheer_buf/revealed_cards,
        //  causing the returned cards to still exist in the source after move.)
        // Use gs.revealed_cards as the primary source; drain cheer_buf in sync.
        let cheer_is_empty = if c.use_p2 {
            gs.player2_cheer_revealed_cards.is_empty()
        } else {
            gs.player1_cheer_revealed_cards.is_empty()
        };
        let cards: Vec<i16> = if !cheer_is_empty {
            let cheer_buf = if c.use_p2 {
                &mut gs.player2_cheer_revealed_cards
            } else {
                &mut gs.player1_cheer_revealed_cards
            };
            let cbuf: SmallVec<[i16; 8]> = core::mem::take(cheer_buf);
            gs.revealed_cards.retain(|id| !cbuf.contains(id));
            cbuf.to_vec()
        } else {
            // Filter to only include cards owned by the target player.
            // revealed_cards is a global pool containing cards from both players'
            // yells/cheers. When the per-player cheer_buf has been consumed by
            // a prior ability, falling back to the entire pool would expose
            // the opponent's cards. Check zone ownership to filter correctly.
            let owned: Vec<i16> = gs
                .revealed_cards
                .iter()
                .filter(|&&cid| {
                    player.hand.cards.contains(&cid)
                        || player.waitroom.cards.contains(&cid)
                        || player.stage.stage.contains(&cid)
                        || player.stage.under_cards.iter().any(|v| v.contains(&cid))
                        || player.energy_zone.cards.contains(&cid)
                        || player.main_deck.cards.contains(&cid)
                        || player.energy_deck.cards.contains(&cid)
                        || player.live_card_zone.cards.contains(&cid)
                        || player.success_live_card_zone.cards.contains(&cid)
                        || gs.resolution_zone.cards.contains(&cid)
                })
                .copied()
                .collect();
            for &cid in &owned {
                gs.revealed_cards.retain(|id| *id != cid);
            }
            owned
        };
        if cards.len() > count {
            // Put cards back so prompt_card_selection can find them.
            gs.revealed_cards.extend(cards.iter().copied());
            let filter = util::filter_from_parts_full(
                c.card_type_filter,
                c.group_name,
                c.cost_limit,
                None, // cost_operator
                c.character_filter,
                None, // name_fragments
                None, // distinct
                None, // exclude_self
                None, // cost_total
                None, // cost_total_operator
                c.effect.exclude_characters_any(),
            );
            let matching: Vec<usize> = (0..gs.revealed_cards.len())
                .filter(|&i| filter.matches(&gs.card_database, gs.revealed_cards[i], false))
                .collect();
            self.prompt_card_selection(
                Zone::RevealedCards.to_str(),
                count,
                c.effect.optional.unwrap_or(false),
                c.effect,
                &filter,
                Some(matching),
            );
            Ok(vec![])
        } else {
            let player = gs.active_player_mut();
            for &cid in &cards {
                if let Some(pos) = player.hand.cards.iter().position(|&cd| cd == cid) {
                    player.hand.remove_card(pos);
                }
            }
            Ok(cards)
        }
    }

    fn resolve_from_under_member(
        &mut self,
        gs: &mut GameState,
        c: &MoveSourceContext,
    ) -> Result<Vec<i16>, String> {
        log::debug!("[UNDER_MEMBER] called selected={:?} pending={:?}", self.selected_cards, self.pending_choice.is_some());
        let target = c.effect.target.as_deref().unwrap_or("self");
        // If this is a resumed call after the player selected a stage member,
        // self.selected_cards will contain that member's ID.
        if !self.selected_cards.is_empty() {
            log::debug!("[UNDER_MEMBER] second call selected={:?}", self.selected_cards);
            let selected_member_id = self.selected_cards[0];
            let idx_opt = {
                let player = gs.resolve_target_player(target);
                player.stage.stage.iter().position(|&id| id == selected_member_id)
            };
            if let Some(idx) = idx_opt {
                let moved = drain_under_cards_to_energy_zone(gs, target, idx);
                self.last_move_moved_any = Some(!moved.is_empty());
                if !moved.is_empty() {
                    gs.recently_moved_cards = Some(moved.clone().into());
                    gs.recently_moved_from_zone = Some("under_member".to_string());
                }
                self.selected_cards.clear();
                return Ok(moved);
            }
            self.selected_cards.clear();
            return Ok(vec![]);
        }

        // First call: find candidates and optionally prompt
        let player = gs.resolve_target_player(target);
        let candidates: Vec<usize> = (0..3)
            .filter(|&i| {
                !player.stage.under_cards[i].is_empty()
                    && player.stage.under_cards[i]
                        .iter()
                        .any(|&cid| gs.card_database.get_card(cid).is_some_and(|card| card.is_energy()))
            })
            .collect();
        if candidates.is_empty() {
            self.last_move_moved_any = Some(false);
            return Ok(vec![]);
        }
        if candidates.len() == 1 {
            if c.effect.optional.unwrap_or(false) {
                // Single candidate with optional: prompt Stage selection with skip allowed
                // so player can choose to move or skip. This mirrors the multiple-candidate
                // path but with one entry, ensuring skip is possible.
                self.pending_choice = Some(
                    crate::ability::types::Choice::select_cards(
                        crate::ability::enums::Zone::Stage.to_str(),
                        1,
                        "Choose a member whose under energies to move".to_string(),
                        true,
                    )
                    .description_ja(Some("下のエネルギーを移動するメンバーを選択".to_string()))
                    .card_type(Some("member_card".to_string()))
                    .filtered_indices(Some(candidates.clone()))
                    .target_player_id(Some(target.to_string()))
                    .is_select_action(true)
                    .build(),
                );
                self.stage_select_intent =
                    Some(crate::ability::types::StageSelectIntent::UnderMemberMove);
                self.execution_context = crate::ability::types::ExecutionContext::SingleEffect { effect_index: 0 };
                return Ok(vec![]);
            }
            let idx = candidates[0];
            let moved = drain_under_cards_to_energy_zone(gs, target, idx);
            self.last_move_moved_any = Some(!moved.is_empty());
            if !moved.is_empty() {
                gs.recently_moved_cards = Some(moved.clone().into());
                gs.recently_moved_from_zone = Some("under_member".to_string());
            }
            return Ok(moved);
        }
        // Multiple candidates, non-optional: prompt to choose
        self.pending_choice = Some(
            crate::ability::types::Choice::select_cards(
                crate::ability::enums::Zone::Stage.to_str(),
                1,
                "Choose a member whose under energies to move".to_string(),
                c.effect.optional.unwrap_or(false),
            )
            .description_ja(Some("下のエネルギーを移動するメンバーを選択".to_string()))
            .card_type(Some("member_card".to_string()))
            .filtered_indices(Some(candidates))
            .target_player_id(Some(target.to_string()))
            .is_select_action(true)
            .build(),
        );
        self.stage_select_intent =
            Some(crate::ability::types::StageSelectIntent::UnderMemberMove);
        self.execution_context = crate::ability::types::ExecutionContext::SingleEffect { effect_index: 0 };
        Ok(vec![])
    }

    pub fn execute_move_cards(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        // Multiple-target move to the deck is handled by the dedicated both path.
        if effect.multiple_targets_any().unwrap_or(false)
            && effect.target.as_deref() == Some("deck")
        {
            return self.execute_move_cards_both(gs, effect);
        }
        let count = if effect.count.is_some() {
            effect.count.unwrap() as usize
        } else if let Some(ref dc) = effect.dynamic_count_any() {
            self.resolve_dynamic_count(gs, dc) as usize
        } else {
            0
        };
        let cost_limit = self.resolve_cost_limit_reference(gs, effect)?;
        let cost_total = effect.cost_total_any();
        let cto_binding = effect.cost_total_operator_any();
        let cost_total_operator = cto_binding.as_deref();
        let group_name = effect.group_name();

        // Handle or_card_types: let the player pick which type to search for
        let card_type_owned: Option<String> = if let Some(or_types) = &effect.or_card_types_any() {
            if or_types.is_empty() {
                effect.card_type_any().map(|s| s.to_string())
            } else {
                let chosen = gs
                    .ability_queue
                    .current_entry()
                    .and_then(|e| e.conditional_choice.clone());
                match chosen {
                    Some(ConditionalChoice::Str(s)) => Some(s),
                    _ => {
                        let type_labels: Vec<String> = or_types
                            .iter()
                            .map(|t| crate::ability::describe::card_type_label(Some(t)).to_string())
                            .collect();
                        self.pending_choice = Some(Choice::SelectTarget {
                            target: "choice_string".to_string(),
                            description: format!("Pick card type: {}", type_labels.join(" / ")),
                            description_en: Some(format!(
                                "Pick card type: {}",
                                type_labels.join(" / ")
                            )),
                            description_ja: Some(format!(
                                "カードタイプを選択: {}",
                                type_labels.join(" / ")
                            )),
                            allow_skip: false,
                            options: Some(type_labels),
                        });
                        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                        if let Some(e) = gs.ability_queue.current_entry_mut() {
                            e.conditional_choice =
                                Some(ConditionalChoice::Strings(or_types.to_vec()));
                        }
                        return Ok(());
                    }
                }
            }
        } else {
            effect.card_type_any().map(|s| s.to_string())
        };
        let card_type_filter: Option<&str> = card_type_owned.as_deref();
        let tgt = effect.target.clone();
        let is_self_cost = effect.self_cost_any().unwrap_or(false);
        let exclude_self = effect.exclude_self_any().unwrap_or(false);
        let is_max = effect.max.unwrap_or(false);
        let is_all = effect.all_any().unwrap_or(false);
        let card_db = gs.card_database.clone();
        let activating_card_id = gs.activating_card;
        let vacated_stage_area = gs.last_vacated_stage_area;
        gs.last_vacated_stage_area = None;

        // Character name filter from the effect
        let character_filter: Option<Vec<String>> = effect.characters_any().cloned();

        // Resolve name_constraint (e.g. "contains_all" from a revealed card)
        let name_fragments: Option<Vec<String>> = if effect.name_constraint_any().as_deref()
            == Some("contains_all")
            && effect.name_constraint_source_any().as_deref() == Some("revealed_card")
        {
            let fragments: Vec<String> = gs
                .revealed_cost_cards
                .iter()
                .chain(gs.revealed_cards.iter())
                .filter_map(|&id| {
                    let card = gs.card_database.get_card(id)?;
                    Some(
                        card.name
                            .replace("\u{FF06}", "&")
                            .split('&')
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>(),
                    )
                })
                .flatten()
                .collect();
            if fragments.is_empty() {
                None
            } else {
                Some(fragments)
            }
        } else {
            None
        };

        let mut moved_cards: Vec<i16> = Vec::new();
        let source = effect
            .source_any()
            .map(|s| s.to_string())
            .unwrap_or_default();
        let destination = effect
            .destination
            .map(|z| z.to_str().to_string())
            .unwrap_or_default();

        let raw_target = tgt.as_deref().unwrap_or("self");
        let target = raw_target;
        let use_p2 = match target {
            "self" => matches!(
                gs.ability_master_id().as_deref(),
                Some("player2") | Some("p2")
            ),
            "opponent" => !matches!(
                gs.ability_master_id().as_deref(),
                Some("player2") | Some("p2")
            ),
            _ => false,
        };

        // Store destination for execute_selected_cards_from_zone to read later
        // (needed when the resolve creates a card selection choice and the destination
        // is not accessible via entry_destination, e.g. for sequential sub-actions).
        self.spawn_context.destination = effect.destination.map(|z| z.to_str().to_string());
        self.spawn_context.source = effect.source_any().map(|s| s.to_string());
        self.spawn_context.position = effect.position_any().and_then(|p| match p {
            crate::card::PositionInfo::String(s) => s.parse::<u8>().ok(),
            crate::card::PositionInfo::Struct { position, .. } => {
                position.as_ref().and_then(|s| s.parse::<u8>().ok())
            }
        });

        // For empty_area / stage destinations: skip selection prompt entirely
        // if the target has no empty slots (card text says "メンバーのいないエリアに").
        if Zone::from_str(&destination) == Some(Zone::EmptyArea) {
            let player = gs.resolve_target_player(target);
            let has_empty_slot = (0..3).any(|i| player.stage.stage[i] == -1);
            if !has_empty_slot {
                return Ok(());
            }
        }

        let mut taken = self.resolve_cards_from_source(
            gs,
            effect,
            count,
            card_type_filter,
            group_name,
            cost_limit,
            cost_total,
            cost_total_operator,
            character_filter.as_ref(),
            name_fragments.as_ref(),
            is_self_cost,
            is_max,
            is_all,
            exclude_self,
            activating_card_id,
            use_p2,
            &source,
            &destination,
            &card_db,
        )?;
        // Q118 all-or-nothing: inside an accepted conditional_on_optional placement,
        // a move that found no card AND offered no choice (e.g. a group missing from
        // the discard pile) makes the placement incomplete, so the trailing
        // "そうしたとき" consequence must not fire. A successful move either returns
        // cards here or creates a pending_choice — only a genuine miss has neither.
        let armed = gs
            .ability_queue
            .current_entry()
            .and_then(|e| e.optional_moves_all_moved)
            .is_some();
        if armed && taken.is_empty() && self.pending_choice.is_none() {
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.optional_moves_all_moved = Some(false);
            }
        }
        let is_deck_dest = Zone::from_str(&destination) == Some(Zone::Deck)
            || Zone::from_str(&destination) == Some(Zone::DeckTop);
        let is_eligible_source = Zone::from_str(&source) == Some(Zone::Discard)
            || Zone::from_str(&source) == Some(Zone::SelectedCards);
        if is_eligible_source
            && is_deck_dest
            && effect.placement_order_any() == Some(PlacementOrder::AnyOrder)
            && taken.len() > 1
        {
            let taken_count = taken.len();
            moved_cards.extend(taken.iter().copied());
            gs.looked_at_cards = taken.clone().into();
            self.pending_choice = Some(Choice::SelectTarget {
                target: "order".to_string(),
                description: format!("Choose order for cards on deck ({} cards)", taken_count),
                description_en: Some(format!(
                    "Choose order for cards on deck ({} cards)",
                    taken_count
                )),
                description_ja: Some(format!("山札のカード順を選択（{}枚）", taken_count)),
                allow_skip: false,
                options: None,
            });
            self.execution_context = ExecutionContext::LookAndSelect {
                step: LookAndSelectStep::Finalize {
                    destination: Zone::Deck.to_str().to_string(),
                    source_zone: String::new(),
                },
            };
            return Ok(());
        }

        // Apply distinct card name filter if specified
        let distinct = effect.distinct_any();
        if distinct == Some(DistinctType::CardName)
            || distinct == Some(DistinctType::True)
            || distinct == Some(DistinctType::Distinct)
        {
            let mut seen: HashSet<String> = HashSet::default();
            taken.retain(|&id| {
                card_db
                    .get_card(id)
                    .map(|c| seen.insert(CardDatabase::normalize_name(&c.name)))
                    .unwrap_or(true)
            });
            if taken.len() < count {
                taken.clear(); // Not enough distinct cards — skip
            }
        }

        // --- STEP 3: Place cards in destination ---
        let deck_pos = effect
            .position_any()
            .as_ref()
            .and_then(|p| match p {
                crate::card::PositionInfo::String(s) => s.parse::<usize>().ok(),
                crate::card::PositionInfo::Struct { position, .. } => {
                    position.as_ref().and_then(|s| s.parse::<usize>().ok())
                }
            })
            .map(|p| if p > 0 { p - 1 } else { 0 });

        let stage_full = {
            let player = if use_p2 { &gs.player2 } else { &gs.player1 };
            Zone::from_str(&destination) == Some(Zone::Stage)
                && !effect.allow_occupied_stage_any().unwrap_or(false)
                && player.stage.stage.iter().all(|&id| id != -1)
        };

        if stage_full {
            log::debug!(
                "[MOVE_CARDS] stage is full, returning {} cards to discard",
                taken.len()
            );
            let player = if use_p2 {
                &mut gs.player2
            } else {
                &mut gs.player1
            };
            for &card_id in &taken {
                player.waitroom.add_card(card_id);
            }
            moved_cards.extend(taken);
        } else {
            for &card_id in &taken {
                // Check for success zone replacement (e.g. 錯覚CROSSROADS)
                if Zone::from_str(&destination) == Some(Zone::SuccessLiveZone) {
                    if let Some(group_names) =
                        crate::turn::TurnEngine::get_success_replacement_info(gs, card_id)
                    {
                        let player = gs.resolve_target_player(tgt.as_deref().unwrap_or("self"));
                        let player_id = player.id.clone();
                        let has_valid_targets = player.waitroom.cards.iter().any(|&cid| {
                            gs.card_database.get_card(cid).is_some_and(|c| {
                                c.is_live()
                                    && group_names.iter().any(|gn| {
                                        crate::ability::util::card_matches_group_str(
                                            &gs.card_database,
                                            cid,
                                            Some(gn),
                                        )
                                    })
                            })
                        });
                        if has_valid_targets {
                            gs.pending_success_replacement_card_id = Some(card_id);
                            gs.pending_success_replacement_player_id = Some(player_id);
                            let group_name = group_names.into_iter().next().unwrap_or_default();
                            let choice = Choice::select_cards(
                                Zone::Discard.to_str(),
                                1,
                                "Choose a live card from discard to place in your success zone (or skip to place the original card)"
                                    .to_string(),
                                true,
                            )
                            .description_ja(Some("控え室から成功ゾーンに置くライブカードを選んでください（スキップで元のカードを置きます）".to_string()))
                            .card_type(Some("live_card".to_string()))
                            .group(Some(group_name))
                            .target_player_id(Some("self".to_string()))
                            .build();
                            self.pending_choice = Some(choice);
                            return Ok(());
                        }
                    }
                }
                if Zone::from_str(&destination) == Some(Zone::Deck) && deck_pos.is_some() && !is_max
                {
                    let pos = deck_pos.unwrap();
                    let player = if use_p2 {
                        &mut gs.player2
                    } else {
                        &mut gs.player1
                    };
                    let clamped = pos.min(player.main_deck.cards.len());
                    player.main_deck.cards.insert(clamped, card_id);
                } else if &*destination == "deck_top_or_bottom" {
                    let can_skip = effect.optional.unwrap_or(false);
                    if can_skip && !taken.is_empty() {
                        // For optional deck placement, the card was left in
                        // waitroom by resolve_cards_from_source. If the player
                        // skips, it stays there.
                    }
                    self.pending_choice = Some(Choice::SelectTarget {
                        target: "position|destination".to_string(),
                        description: "Choose deck top or bottom".to_string(),
                        description_en: Some("Choose deck top or bottom".to_string()),
                        description_ja: Some("山札の上または下を選択".to_string()),
                        allow_skip: can_skip,
                        options: Some(vec![
                            Zone::DeckTop.to_str().to_string(),
                            Zone::DeckBottom.to_str().to_string(),
                        ]),
                    });
                    self.execution_context = ExecutionContext::MoveCardsPosition {
                        card_id,
                        state_change: effect.state_change_any().map(|s| s.to_string()),
                        target: tgt.as_deref().unwrap_or("self").to_string(),
                        source_zone: source.to_string(),
                    };
                    return Ok(());
                } else {
                    match self.place_card_with_stage_choice(
                        gs,
                        target,
                        card_id,
                        &destination,
                        vacated_stage_area,
                        is_max,
                        count,
                        effect.state_change_any().map(|s| s.to_string()),
                        deck_pos,
                        &source,
                        effect.allow_occupied_stage_any().unwrap_or(false),
                        effect.is_under_self(),
                    ) {
                        Ok(true) => {
                            return Ok(());
                        }
                        Ok(false) => {
                            moved_cards.push(card_id);
                        }
                        Err(_) => {
                            let player = if use_p2 {
                                &mut gs.player2
                            } else {
                                &mut gs.player1
                            };
                            let src_zone = if &*source == "those_cards" {
                                Zone::Discard.to_str()
                            } else {
                                &*source
                            };
                            util::place_card_in_zone(player, card_id, src_zone, None, false, 1);
                        }
                    }
                }
            }
        }

        let state_change = effect.state_change_any().map(|s| s.to_string());
        // Record whether this move actually moved any cards. Used by the
        // "…したとき" (when you do so) pattern: a consequence step directly
        // following a move that moved nothing must be skipped.
        self.last_move_moved_any = Some(!moved_cards.is_empty());
        self.finalize_card_movement(
            gs,
            &moved_cards,
            &destination,
            &source,
            &state_change,
            tgt.as_deref(),
        );
        Ok(())
    }

    pub fn handle_select_position(
        &mut self,
        gs: &mut GameState,
        position: &str,
        context: ExecutionContext,
    ) -> Result<(), String> {
        match &context {
            ExecutionContext::LookAndSelect { step } => {
                if let LookAndSelectStep::Finalize {
                    destination,
                    source_zone,
                } = step
                {
                    if Zone::from_str(destination) == Some(Zone::Stage) {
                        if let Some(&card_id) = gs.looked_at_cards.last() {
                            let player = &mut gs.player1;
                            let pos_idx = super::util::stage_position_index(position);
                            let should_lock = source_zone != Zone::Stage.to_str();
                            fn do_place(
                                player: &mut crate::player::Player,
                                idx: usize,
                                card_id: i16,
                                should_lock: bool,
                            ) {
                                if player.stage.stage[idx] != -1 {
                                    player.waitroom.add_card(player.stage.stage[idx]);
                                }
                                player.stage.stage[idx] = card_id;
                                if should_lock {
                                    // Rule 9.6.2.1.2.1: Track card deployed from non-stage.
                                    player.track_deployment(card_id);
                                }
                            }
                            match pos_idx {
                                Some(0) => do_place(player, 0, card_id, should_lock),
                                Some(1) => do_place(player, 1, card_id, should_lock),
                                Some(2) => do_place(player, 2, card_id, should_lock),
                                _ => {
                                    player.hand.add_card(card_id);
                                }
                            }
                            gs.looked_at_cards.clear();
                        }
                    }
                }
            }
            ExecutionContext::MoveCardsPosition {
                card_id,
                state_change,
                target,
                source_zone,
            } => {
                let card_id = *card_id;
                let state_change = state_change.clone();
                let target = target.clone();
                let should_lock = source_zone != Zone::Stage.to_str();

                let pos_idx = super::util::stage_position_index(position);

                let player = gs.resolve_target_player_mut(&target);
                let mut placed = match pos_idx {
                    Some(idx) if idx < 3 && player.stage.stage[idx] == -1 => {
                        player.stage.stage[idx] = card_id;
                        if should_lock {
                            // Rule 9.6.2.1.2.1: Track card deployed from non-stage.
                            player.track_deployment(card_id);
                        }
                        true
                    }
                    _ => false,
                };
                // If chosen position is occupied, replace (move existing to waitroom)
                if !placed {
                    if let Some(idx) = pos_idx {
                        if idx < 3 && player.stage.stage[idx] != -1 {
                            player.waitroom.add_card(player.stage.stage[idx]);
                            player.stage.stage[idx] = card_id;
                            if should_lock {
                                // Rule 9.6.2.1.2.1: Track card deployed from non-stage.
                                player.track_deployment(card_id);
                            }
                            placed = true;
                        }
                    }
                }
                if !placed {
                    player.hand.add_card(card_id);
                }

                gs.mods.clear_all_for_card(card_id);
                gs.record_card_movement(card_id);
                if !self.moved_cards.contains(&card_id) {
                    self.moved_cards.push(card_id);
                }
                if state_change.as_deref() == Some("wait") {
                    gs.mods.add_orientation_modifier(card_id, "wait");
                }
                log::debug!(
                    "[DEPLOY] player answered the position choice for id={card_id} — \
                     firing its 登場 now",
                    card_id = card_id
                );
                self.fire_debut_side_effects(gs, card_id, &target);
            }
            _ => {}
        }
        // Clear only the position choice itself. fire_debut_side_effects may
        // have queued the placed card's OWN debut (Q200/Q201/Q202), whose
        // cost/effect prompts must survive — a blanket clear here silently
        // swallowed them.
        if matches!(
            self.pending_choice,
            Some(Choice::SelectPosition { .. })
        ) {
            self.pending_choice = None;
        }
        log::debug!(
            "[DEPLOY] after position handling: pending={:?}",
            format!("{:?}", self.pending_choice)
        );
        self.execution_context = ExecutionContext::None;
        // Place remaining cards deferred by multi-card stage selection
        // BEFORE resuming pending commands, so deferred cards get their
        // position choices before the re-prompt (if any).
        if !self.pending_stage_cards.is_empty() {
            let remaining = core::mem::take(&mut self.pending_stage_cards);
            for (i, (cid, tgt)) in remaining.iter().enumerate() {
                let source = self.spawn_context.source.clone().unwrap_or_default();
                match self.place_card_with_stage_choice(
                    gs,
                    tgt,
                    *cid,
                    Zone::Stage.to_str(),
                    None,
                    false,
                    1,
                    None,
                    None,
                    &source,
                    false,
                    false,
                ) {
                    Ok(true) => {
                        if i + 1 < remaining.len() {
                            self.pending_stage_cards = remaining[i + 1..].into();
                        }
                        return Ok(());
                    }
                    Ok(false) => {
                        self.fire_debut_side_effects(gs, *cid, tgt);
                        gs.mods.clear_all_for_card(*cid);
                        gs.record_card_movement(*cid);
                    }
                    Err(_) => {}
                }
            }
        }
        self.resume_pending_actions(gs)?;
        log::debug!(
            "[DEPLOY] after resume_pending_actions: pending={:?}",
            format!("{:?}", self.pending_choice)
        );
        Ok(())
    }

    /// Apply post-move side effects: zone-exit cleanup (rule 4.1.4),
    /// state_change, record_card_movement, tracking.
    fn finalize_card_movement(
        &mut self,
        gs: &mut GameState,
        moved_cards: &[i16],
        destination: &str,
        source: &str,
        state_change: &Option<String>,
        target: Option<&str>,
    ) {
        gs.on_cards_left_zones(moved_cards);

        if let Some(ref sc) = state_change {
            if sc == "wait" {
                for card_id in moved_cards {
                    gs.mods.add_orientation_modifier(*card_id, "wait");
                }
            } else if sc == "active" {
                for card_id in moved_cards {
                    gs.mods.add_orientation_modifier(*card_id, "active");
                }
                if Zone::from_str(destination) == Some(Zone::Energy) {
                    let p = match target.unwrap_or("self") {
                        "self" => &mut gs.player1,
                        "opponent" => &mut gs.player2,
                        _ => &mut gs.player1,
                    };
                    p.energy_zone.active_energy_count += moved_cards.len() as u8;
                }
            }
        }

        for card_id in moved_cards {
            gs.record_card_movement(*card_id);
        }

        self.moved_cards.extend(moved_cards.iter().copied());
        {
            let cause_pid = gs
                .ability_queue
                .current_entry()
                .map(|e| e.player_id.clone())
                .unwrap_or_default();
            let cause_cid = gs.activating_card;
            // same_area is an internal repositioning mechanic, not a zone transition.
            // Skip push_movement_event so watcher triggers don't fire for it.
            if destination != "same_area" {
                for &cid in moved_cards {
                    gs.push_movement_event(
                        cid,
                        &source.to_string(),
                        destination,
                        cause_cid,
                        &cause_pid,
                        true,
                    );
                }
            }
        }
        log::debug!(
            "[FINALIZE_MOVE] dest={} cards={:?} -> self.moved_cards={:?}",
            destination,
            moved_cards,
            self.moved_cards
        );

        // Zone membership changed — energy-count / stage / success-zone
        // constants (「〜あるかぎり」) are live state and must re-evaluate now,
        // not at the next phase boundary.
        if !moved_cards.is_empty() {
            gs.recalculate_constants();
        }

        // Debut side effects for cards placed directly on stage (single free slot,
        // no position-choice dialog). Each card goes through the unified
        // fire_debut_side_effects which handles record_appearance, debut_count,
        // card's own debut ability, AND cascading "when ally debuts" triggers.
        if Zone::from_str(destination) == Some(Zone::Stage) && !moved_cards.is_empty() {
            let tgt = target.unwrap_or("self");
            for &card_id in moved_cards {
                self.fire_debut_side_effects(gs, card_id, tgt);
            }
        }
    }

    /// Move cards from revealed_cards to a destination zone.
    pub fn move_from_revealed(
        &mut self,
        gs: &mut GameState,
        indices: &[usize],
        validate_card: &mut impl FnMut(i16) -> bool,
        dst: &str,
    ) -> Vec<i16> {
        let cards: Vec<i16> = {
            let revealed = &mut gs.revealed_cards;
            let mut result = Vec::new();
            let mut removed: Vec<usize> = indices.to_vec();
            removed.sort_by(|a, b| b.cmp(a));
            for &i in &removed {
                if i < revealed.len() {
                    let cid = revealed.remove(i);
                    if validate_card(cid) {
                        result.push(cid);
                    }
                }
            }
            result
        };
        // Remove from physical zone (waitroom for yell cards,
        // hand for cost reveals, deck for deck-peek reveals).
        for &cid in &cards {
            if let Some(pos) = gs.player1.waitroom.cards.iter().position(|&c| c == cid) {
                gs.player1.waitroom.cards.remove(pos);
            } else if let Some(pos) = gs.player2.waitroom.cards.iter().position(|&c| c == cid) {
                gs.player2.waitroom.cards.remove(pos);
            } else if let Some(pos) = gs.player1.main_deck.cards.iter().position(|&c| c == cid) {
                gs.player1.main_deck.cards.remove(pos);
            } else if let Some(pos) = gs.player2.main_deck.cards.iter().position(|&c| c == cid) {
                gs.player2.main_deck.cards.remove(pos);
            }
        }
        // Don't set self.selected_cards here — cards moved from
        // revealed_cards are effect-internal (not user-targeted
        // selections), and would bleed into downstream gain_resource
        // via the "pure sequential select→gain_resource" path.
        let player = gs.active_player_mut();
        for &cid in &cards {
            util::place_card_in_zone(player, cid, dst, None, false, 1);
        }
        cards
    }

    /// Move cards from under_member to a destination zone, using flat 3-position indexing.
    pub fn move_from_under_member(
        &mut self,
        gs: &mut GameState,
        indices: &[usize],
        validate_card: &mut impl FnMut(i16) -> bool,
        dst: &str,
        target: &str,
    ) -> Result<Vec<i16>, String> {
        let player = gs.resolve_target_player_mut(target);
        let mut cards_to_move: Vec<(usize, i16)> = Vec::new();
        for &idx in indices.iter() {
            let mut global_idx = 0;
            let mut found = false;
            for si in 0..3 {
                if idx < global_idx + player.stage.under_cards[si].len() {
                    let card_id = player.stage.under_cards[si][idx - global_idx];
                    if !validate_card(card_id) {
                        return Err(format!(
                            "Card {:?} does not match required type filter for under_member selection",
                            card_id
                        ));
                    }
                    cards_to_move.push((si, card_id));
                    found = true;
                    break;
                }
                global_idx += player.stage.under_cards[si].len();
            }
            if !found {
                return Err(format!("Card at index {} not found in under_member", idx));
            }
        }
        for (si, card_id) in &cards_to_move {
            if let Some(pos) = player.stage.under_cards[*si]
                .iter()
                .position(|&c| c == *card_id)
            {
                player.stage.under_cards[*si].remove(pos);
                util::place_card_in_zone(player, *card_id, dst, None, false, 1);
            }
        }
        gs.recalculate_constants();
        // Don't save energy card IDs in selected_cards — they would leak
        // into downstream sequential actions (e.g. gain_resource heart targets).
        // moved_cards already tracks these via the caller.
        Ok(cards_to_move.iter().map(|&(_, cid)| cid).collect())
    }

    /// Fire all side effects for a card placed on stage: record appearance,
    /// increment debut count, fire the card's own debut ability, then cascade
    /// to "when ally debuts" triggers for other stage members.
    /// This is the single canonical function for debut processing — every
    /// code path that places a card on stage must call this.
    fn fire_debut_side_effects(&self, gs: &mut GameState, card_id: i16, target: &str) {
        let player_id = gs.resolve_target_player(target).id.clone();

        let source = self.spawn_context.source.as_deref().unwrap_or("");
        gs.record_card_appearance(card_id, source);

        let card = gs.card_database.get_card(card_id).cloned();
        if let Some(card) = card {
            let card_no = card.card_no.to_string();

            if player_id == gs.player1.id {
                gs.player1.debut_count_this_turn += 1;
            } else if player_id == gs.player2.id {
                gs.player2.debut_count_this_turn += 1;
            }

            let mut debut_abilities = 0;
            for ar in &card.abilities {
                let ability = ar.resolve();
                if GameState::ability_matches_trigger(
                    &ability,
                    &crate::core::types::AbilityTrigger::Debut,
                ) {
                    debut_abilities += 1;
                    let ability_id = format!("{}_{}", card_no, ability.full_text);
                    log::debug!(
                        "[DEBUT_CHAIN] enqueueing debut of {card_no} (id={card_id}, \
                         controller={player_id}): {ability_id}"
                    );
                    gs.trigger_auto_ability(
                        ability_id,
                        crate::core::types::AbilityTrigger::Debut,
                        player_id.clone(),
                        Some(card_no.clone()),
                        Some(card_id),
                        None,
                        None,
                    );
                }
            }
            if debut_abilities == 0 {
                log::debug!(
                    "[DEBUT_CHAIN] {card_no} (id={card_id}) has no 登場 abilities — \
                     nothing to enqueue"
                );
            }
        } else {
            log::debug!("[DEBUT_CHAIN] fire_debut_side_effects: unknown card id {card_id}");
        }

        // Cascade to other stage members that watch for ally debuts.
        gs.trigger_auto_abilities_for_player(&player_id);
        gs.process_pending_auto_abilities(&player_id);
    }

    fn execute_stage_placement_choices(
        &mut self,
        gs: &mut GameState,
        card_ids: &[i16],
        src_zone: &str,
        dest: &str,
        vacated_area: Option<u8>,
        target: &str,
    ) -> Result<Vec<i16>, String> {
        let card_db = gs.card_database.clone();
        let mut moved = Vec::new();
        for (pos, &card_id) in card_ids.iter().enumerate() {
            {
                let player = gs.resolve_target_player_mut(target);
                util::remove_card_from_zone(player, card_id, src_zone, &card_db);
            }
            let entry_effect = gs
                .ability_queue
                .current_entry()
                .and_then(|e| e.ability.effect.clone());
            let state_change = entry_effect
                .as_ref()
                .and_then(|ef| ef.state_change_any().map(|s| s.to_string()));
            let allow_occupied = entry_effect
                .as_ref()
                .and_then(|ef| ef.allow_occupied_stage_any())
                .unwrap_or(false);
            let entry_self_target = entry_effect
                .as_ref()
                .map(|ef| ef.is_under_self())
                .unwrap_or(false);
            match self.place_card_with_stage_choice(
                gs,
                target,
                card_id,
                dest,
                vacated_area,
                false,
                1,
                state_change,
                None,
                src_zone,
                allow_occupied,
                entry_self_target,
            ) {
                Ok(true) => {
                    moved.push(card_id);
                    self.sub_choice_created = true;
                    for &rcid in &card_ids[pos + 1..] {
                        let pl = gs.resolve_target_player_mut(target);
                        self.pending_stage_cards.push((rcid, target.to_string()));
                        util::remove_card_from_zone(pl, rcid, src_zone, &card_db);
                    }
                    return Ok(moved);
                }
                Ok(false) => {
                    moved.push(card_id);
                    self.fire_debut_side_effects(gs, card_id, &target);
                }
                Err(_) => {
                    let player = gs.resolve_target_player_mut(target);
                    util::place_card_in_zone(player, card_id, src_zone, None, false, 1);
                }
            }
        }
        Ok(moved)
    }

    /// Execute card movement from a zone: pre-validate filters, move cards to destination, track side effects.
    pub fn execute_selected_cards_from_zone(
        &mut self,
        gs: &mut GameState,
        zone: &str,
        indices: &[usize],
        card_type_filter: Option<&str>,
        cost_limit: Option<u8>,
        cost_limit_operator: Option<&str>,
        cost_total: Option<u8>,
        cost_total_operator: Option<&str>,
        group: Option<&str>,
        characters: Option<&Vec<String>>,
        target_player_id: Option<&str>,
    ) -> Result<(), String> {
        let destination = gs
            .entry_destination()
            .map(|s| s.to_string())
            .or_else(|| self.spawn_context.destination.clone())
            // Sub-action select_cards steps carry their own destination
            // (e.g. 希 bp3-007 「1枚をデッキの上に置き」) — without this
            // fallback they all defaulted to discard.
            .or_else(|| {
                gs.entry_effect()
                    .and_then(|e| e.destination.map(|z| z.to_str().to_string()))
            });
        let target = target_player_id
            .map(|s| s.to_string())
            .or_else(|| self.spawn_context.target.clone())
            .or_else(|| {
                gs.entry_effect()
                    .and_then(|e| e.target.clone().map(|s| s.to_string()))
            })
            .unwrap_or_else(|| "self".to_string().into());
        let card_db = gs.card_database.clone();
        let vacated_area = gs.last_vacated_stage_area;
        log::debug!(
            "[EXEC_SEL] zone={} indices={:?} dest={:?} target={}",
            zone,
            indices,
            destination,
            target
        );

        // Card-property restriction (e.g. emma bp7-008 「ブレードハートを
        // 持たないメンバーカード」→ card_property="has_blade_heart",
        // negation=true). The Choice advertised to the player does not carry
        // these fields, so read them from the activating effect.
        let card_property = gs
            .entry_effect()
            .and_then(|e| e.card_property_any().map(|s| s.to_string()));
        let property_negation = gs
            .entry_effect()
            .and_then(|e| e.negation_any())
            .unwrap_or(false);
        let passes = |cid: i16| -> bool {
            util::card_matches_type(&card_db, cid, card_type_filter)
                && util::card_matches_cost_limit_op(&card_db, cid, cost_limit, cost_limit_operator)
                && util::card_matches_group_str(&card_db, cid, group)
                && match characters {
                    Some(chars) if !chars.is_empty() => {
                        util::card_matches_characters(&card_db, cid, Some(chars))
                    }
                    _ => true,
                }
                && match card_property.as_deref() {
                    Some(prop) => {
                        let has = match prop {
                            "has_blade_heart" => card_db.get_card(cid).is_some_and(|c| c.has_blade_heart()),
                            "has_score_icon" => card_db.get_card(cid).is_some_and(|c| c.has_score_icon()),
                            "has_all_blade" => card_db.get_card(cid).is_some_and(|c| c.has_all_blade()),
                            _ => false,
                        };
                        if property_negation { !has } else { has }
                    }
                    None => true,
                }
        };

        // Filter indices to only include cards that match required filters.
        // Non-matching cards are silently skipped (consistent with cost-phase handler).
        let filtered_indices: Vec<usize> = {
            let player = gs.resolve_target_player(&target);
            let cards = util::zone_cards(player, zone);
            log::debug!(
                "[EXEC_SEL_FILTER] zone={} cards.len={} indices={:?}",
                zone,
                cards.len(),
                indices
            );
            let result: Vec<usize> = indices
                .iter()
                .filter(|&&idx| {
                    let ok = idx < cards.len() && passes(cards[idx]);
                    if idx < cards.len() {
                        log::debug!("[EXEC_SEL_PASS] idx={} cid={} pass={}", idx, cards[idx], ok);
                    }
                    ok
                })
                .copied()
                .collect();
            log::debug!("[EXEC_SEL_FILTER_RESULT] filtered_indices={:?}", result);
            result
        };

        let zone_enum = Zone::from_str(zone);
        let dest = destination
            .as_deref()
            .unwrap_or(if zone_enum == Some(Zone::Discard) {
                Zone::Hand.to_str()
            } else {
                Zone::Discard.to_str()
            });
        if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
            log::debug!(
                "[EXEC_SEL_DEST] zone={:?} destination={:?} dest={}",
                zone_enum,
                destination,
                dest
            );
        }
        let mut moved = Vec::new();
        match zone_enum {
            Some(Zone::Hand)
            | Some(Zone::Discard)
            | Some(Zone::Deck)
            | Some(Zone::LiveCardZone) => {
                if Zone::from_str(dest) == Some(Zone::Stage) {
                    let player = gs.resolve_target_player(&target);
                    if player.stage.stage.iter().all(|&id| id != -1) {
                        log::debug!("[STAGE_PLACEMENT] stage is full, cannot place cards");
                        return Ok(());
                    }
                }

                // A SUM constraint applies only when the ability text states a
                // total (「コストの合計」→ parsed cost_total). Falling back to
                // the per-card cost_limit here was wrong: 「コスト2以下の
                // メンバーを2枚まで」 limits EACH card, and eligibility is
                // already enforced by the filtered_indices pass above.
                let sum_limit = cost_total;
                let sum_operator = cost_total_operator;
                if zone_enum == Some(Zone::Discard) && sum_limit.is_some() {
                    let player = gs.resolve_target_player(&target);
                    let limit = sum_limit.unwrap();
                    let op = sum_operator.unwrap_or("<=");
                    let card_ids = util::resolve_indices_to_ids(player, zone, &filtered_indices);
                    let total_cost: u8 = card_ids
                        .iter()
                        .filter_map(|&cid| card_db.get_card(cid).and_then(|c| c.cost))
                        .sum();
                    let ok = match op {
                        ">=" => total_cost >= limit,
                        ">" => total_cost > limit,
                        "<" => total_cost < limit,
                        "exact" | "=" => total_cost == limit,
                        _ => total_cost <= limit,
                    };
                    if !ok {
                        log::debug!(
                            "Sum-total cost {} exceeds limit (max {}), selection rejected",
                            total_cost,
                            limit
                        );
                        return Ok(());
                    }
                }

                let card_ids = {
                    let player = gs.resolve_target_player(&target);
                    util::resolve_indices_to_ids(player, zone, &filtered_indices)
                };

                match Zone::from_str(dest) {
                    Some(Zone::Stage) | Some(Zone::EmptyArea) | Some(Zone::SameArea)
                        if zone_enum != Some(Zone::Deck) =>
                    {
                        moved = self.execute_stage_placement_choices(
                            gs,
                            &card_ids,
                            zone,
                            dest,
                            vacated_area,
                            &target,
                        )?;
                    }
                    Some(Zone::Deck) => {
                        // Resolve deck position from the entry effect,
                        // falling back to spawn_context.position for sub-action effects.
                        let deck_pos = gs
                            .entry_effect()
                            .and_then(|ef| {
                                ef.position_any().as_ref().and_then(|p| match p {
                                    crate::card::PositionInfo::String(s) => s.parse::<u8>().ok(),
                                    crate::card::PositionInfo::Struct { position, .. } => {
                                        position.as_ref().and_then(|s| s.parse::<u8>().ok())
                                    }
                                })
                            })
                            .or(self.spawn_context.position)
                            .map(|p| if p > 0 { p - 1 } else { 0 });
                        let player = gs.resolve_target_player_mut(&target);
                        for &cid in &card_ids {
                            if let Some(pos) = deck_pos {
                                let clamped = (pos as usize).min(player.main_deck.cards.len());
                                player.main_deck.cards.insert(clamped, cid);
                            } else {
                                util::place_card_in_zone(player, cid, dest, None, false, 1);
                            }
                        }
                        util::zone_remove_at_indices(player, zone, &filtered_indices);
                        moved = card_ids;
                    }
                    _ if dest == "deck_top_or_bottom" => {
                        if let Some(&cid) = card_ids.first() {
                            self.pending_choice = Some(Choice::SelectTarget {
                                target: "position|destination".to_string(),
                                description: "Choose deck top or bottom".to_string(),
                                description_en: Some("Choose deck top or bottom".to_string()),
                                description_ja: Some("山札の上または下を選択".to_string()),
                                allow_skip: false,
                                options: Some(vec![
                                    Zone::DeckTop.to_str().to_string(),
                                    Zone::DeckBottom.to_str().to_string(),
                                ]),
                            });
                            self.sub_choice_created = true;
                            self.execution_context = ExecutionContext::MoveCardsPosition {
                                card_id: cid,
                                state_change: None,
                                target: target.clone(),
                                source_zone: zone.to_string(),
                            };
                            return Ok(());
                        }
                    }
                    _ => {
                        log::debug!(
                            "[MOVE_CARDS] zone={} dest={} card_ids={:?} moved={:?} target={}",
                            zone,
                            dest,
                            card_ids,
                            moved,
                            target
                        );
                        // Check for success zone replacement (e.g. 錯覚CROSSROADS)
                        if Zone::from_str(dest) == Some(Zone::SuccessLiveZone) {
                            if let Some(&replaced_card_id) = card_ids.first() {
                                if let Some(group_names) =
                                    crate::turn::TurnEngine::get_success_replacement_info(
                                        gs,
                                        replaced_card_id,
                                    )
                                {
                                    let player = gs.resolve_target_player(&target);
                                    let player_id = player.id.clone();
                                    let has_valid_targets =
                                        player.waitroom.cards.iter().any(|&cid| {
                                            gs.card_database.get_card(cid).is_some_and(|c| {
                                                c.is_live() && group_names.iter().any(|gn| {
                                                    crate::ability::util::card_matches_group_str(
                                                        &gs.card_database,
                                                        cid,
                                                        Some(gn),
                                                    )
                                                })
                                            })
                                        });
                                    if has_valid_targets {
                                        gs.pending_success_replacement_card_id =
                                            Some(replaced_card_id);
                                        gs.pending_success_replacement_player_id = Some(player_id);
                                        let group_name =
                                            group_names.into_iter().next().unwrap_or_default();
                                        let choice = Choice::select_cards(
                                            Zone::Discard.to_str(),
                                            1,
                                            "Choose a live card from discard to place in your success zone (or skip to place the original card)"
                                                .to_string(),
                                            true,
                                        )
                                        .description_ja(Some("控え室から成功ゾーンに置くライブカードを選んでください（スキップで元のカードを置きます）".to_string()))
                                        .card_type(Some("live_card".to_string()))
                                        .group(Some(group_name))
                                        .target_player_id(Some("self".to_string()))
                                        .build();
                                        self.pending_choice = Some(choice);
                                        return Ok(());
                                    }
                                }
                            }
                        }
                        let player = gs.resolve_target_player_mut(&target);
                        util::move_cards(player, &card_ids, zone, dest, None, &card_db);
                        moved = card_ids;
                    }
                }

                if zone_enum == Some(Zone::Hand)
                    && (Zone::from_str(dest) == Some(Zone::Discard)
                        || Zone::from_str(dest) == Some(Zone::Waitroom))
                {
                    gs.mods.last_cost_discard_count = moved.len() as u8;
                    gs.mods.last_cost_moved_card_ids = moved.clone().into();
                }
            }
            Some(Zone::Stage) => {
                let player = gs.resolve_target_player_mut(&target);
                for &idx in &filtered_indices {
                    if idx < 3 && player.stage.stage[idx] != -1 {
                        self.selected_cards.push(player.stage.stage[idx]);
                    }
                }
            }
            Some(Zone::RevealedCards) => {
                for &idx in filtered_indices.iter().rev() {
                    if idx < gs.revealed_cards.len() {
                        let card_id = gs.revealed_cards.remove(idx);
                        self.selected_cards.push(card_id);
                    }
                }
            }
            _ => return Err(format!("Unknown zone: {}", zone)),
        }

        for cid in &moved {
            gs.mods.clear_all_for_card(*cid);
            if !self.selected_cards.contains(cid) {
                self.selected_cards.push(*cid);
            }
            if !self.moved_cards.contains(cid) {
                self.moved_cards.push(*cid);
            }
        }

        let state_change = gs
            .ability_queue
            .current_entry()
            .and_then(|e| e.ability.effect.as_ref())
            .and_then(|ef| ef.state_change_any().map(|s| s.to_string()));
        if let Some(sc) = state_change {
            if sc == "wait" {
                for &cid in &moved {
                    gs.mods.add_orientation_modifier(cid, "wait");
                }
            }
        }

        if !moved.is_empty() {
            let cause_cid = gs.activating_card;
            let cause_pid = gs
                .ability_queue
                .current_entry()
                .map(|e| e.player_id.clone())
                .unwrap_or_default();
            for &cid in &moved {
                if cid != -1 {
                    gs.push_movement_event(cid, zone, dest, cause_cid, &cause_pid, true);
                }
            }
        }
        Ok(())
    }

    /// Handle looked_at card selection: validate, move to destination, handle multi-select and remaining cards.
    // Q86 / Q122 / Rule 4.8.3: Finalize look-and-select selection
    //
    // After the user selects cards from looked_at_cards:
    //   1. Selected cards → destination (usually hand, via place_card_in_zone)
    //   2. Unselected cards → discard (if discard_remaining = true)
    //      or → deck bottom (if discard_remaining = false)
    //   3. Per-group constraint (max_per_group) enforced before moving
    //
    // Rule 4.8.3: When moving multiple cards from main deck, they are moved
    //   one at a time. Since these cards are already in looked_at_cards
    //   (not in the deck zone anymore), this rule doesn't apply here.
    //
    // Q86: After selected cards go to hand and remainder to discard, if
    //   the source deck is now empty, Rule 10.2.2.1 triggers at next
    //   check timing (refresh).
    //
    // Q122: If an effect looks at cards and puts them back on deck
    //   (rearrangement), the cards were never removed from the deck zone
    //   for refresh purposes, so no refresh occurs during the effect.
    pub fn handle_select_cards_looked_at(
        &mut self,
        gs: &mut GameState,
        indices: &[usize],
        ctx_destination: Option<String>,
        ctx_discard_remaining: Option<bool>,
    ) -> Result<(), String> {
        let target = self
            .spawn_context
            .target
            .clone()
            .or_else(|| {
                gs.entry_effect()
                    .and_then(|e| e.target.clone().map(|s| s.to_string()))
            })
            .unwrap_or_else(|| "self".to_string().into());
        let select_action = self
            .current_effect
            .as_ref()
            .and_then(|ef| ef.compound.select_action.clone())
            .or_else(|| {
                gs.ability_queue
                    .current_entry()
                    .and_then(|e| e.ability.effect.as_ref())
                    .and_then(|ef| ef.compound.select_action.clone())
            })
            .or_else(|| {
                self.current_effect.as_ref().and_then(|ef| {
                    if ef.action == crate::ability::enums::ActionType::SelectCards {
                        Some(Box::new(ef.clone()))
                    } else {
                        None
                    }
                })
            });
        let current = self.current_effect.as_ref();
        // Whether the EFFECT TEXT explicitly says what happens to the
        // unselected remainder (e.g. 「残りを控え室に置く」). When it does NOT,
        // a fully-declined optional selection must return the cards to where
        // they were looked at from — not to the discard fallback.
        let explicit_discard = select_action
            .as_ref()
            .and_then(|sa| sa.discard_remaining_any())
            .or_else(|| current.and_then(|c| c.discard_remaining_any()))
            .or_else(|| ctx_discard_remaining);
        let (destination, discard_remaining, placement_order) = (
            select_action
                .as_ref()
                .and_then(|sa| sa.destination.clone().map(|s| s.to_string()))
                .or_else(|| current.and_then(|c| c.destination.clone().map(|s| s.to_string())))
                .or_else(|| ctx_destination)
                .unwrap_or_else(|| Zone::Hand.to_str().to_string().into()),
            explicit_discard.unwrap_or(true),
            select_action
                .as_ref()
                .and_then(|sa| sa.placement_order_any().clone())
                .or_else(|| current.and_then(|c| c.placement_order_any().clone())),
        );

        if gs.looked_at_cards.is_empty() && !self.selected_cards.is_empty() {
            gs.looked_at_cards = self.selected_cards.iter().copied().collect();
        }

        log::debug!(
            "[LA_DEBUG] len={} indices={:?}",
            gs.looked_at_cards.len(),
            indices
        );
        #[cfg(not(feature = "no_std"))]
        eprintln!(
            "[LA_DEST_DBG] resolved_dest={:?} discard_remaining={:?} placement_order={:?}",
            destination, discard_remaining, placement_order
        );

        // Extract per-group constraint from execution context
        let max_per_group = match &self.execution_context {
            ExecutionContext::LookAndSelect {
                step: LookAndSelectStep::Select { max_per_group, .. },
            } => *max_per_group,
            _ => None,
        };

        let looked_at = &mut gs.looked_at_cards;

        // Validate per-group constraint before removing cards
        if let Some(mpg) = max_per_group {
            let card_db = &gs.card_database;
            let mut group_counts: HashMap<String, u8> = HashMap::default();
            for &idx in indices {
                if idx < looked_at.len() {
                    let cid = looked_at[idx];
                    if let Some(card) = card_db.get_card(cid) {
                        if !card.series.is_empty() {
                            let count = group_counts.entry(card.series.to_string()).or_insert(0);
                            *count += 1;
                            if *count > mpg {
                                return Err(format!(
                                    "Cannot select more than {} card(s) from the same series ({})",
                                    mpg, card.series
                                ));
                            }
                        }
                    }
                }
            }
        }

        let mut indices_sorted: Vec<usize> = indices.to_vec();
        indices_sorted.sort_by(|a, b| b.cmp(a));

        let mut selected_cards: Vec<i16> = Vec::new();
        for i in indices_sorted {
            if i < looked_at.len() {
                selected_cards.insert(0, looked_at.remove(i));
            }
        }
        let selected_count = selected_cards.len();

        {
            let card_db = &gs.card_database;
            let cost_limit = select_action.as_ref().and_then(|sa| sa.cost_limit_any());
            let cost_limit_operator = select_action
                .as_ref()
                .and_then(|sa| sa.cost_limit_operator_any().map(Operator::as_str));
            selected_cards.retain(|&cid| {
                util::card_matches_cost_limit_op(card_db, cid, cost_limit, cost_limit_operator)
            });
        }

        let remaining_cards: SmallVec<[i16; 8]> = core::mem::take(looked_at);

        let is_deck_dest = Zone::from_str(&destination) == Some(Zone::DeckTop)
            || Zone::from_str(&destination) == Some(Zone::Deck);
        let needs_order = is_deck_dest
            && placement_order == Some(PlacementOrder::AnyOrder)
            && selected_cards.len() > 1;

        if needs_order {
            gs.looked_at_cards = selected_cards.into();
            let player = gs.resolve_target_player_mut(&target);
            let dest_zone = if discard_remaining {
                Zone::Discard.to_str()
            } else {
                Zone::DeckBottom.to_str()
            };
            for card_id in remaining_cards {
                util::place_card_in_zone(player, card_id, dest_zone, None, false, 1);
            }
            let card_count = gs.looked_at_cards.len();
            self.pending_choice = Some(Choice::SelectTarget {
                target: "order".to_string(),
                description: format!("Choose order for cards on deck ({} cards)", card_count),
                description_en: Some(format!(
                    "Choose order for cards on deck ({} cards)",
                    card_count
                )),
                description_ja: Some(format!("山札のカード順を選択（{}枚）", card_count)),
                allow_skip: false,
                options: None,
            });
            self.execution_context = ExecutionContext::LookAndSelect {
                step: LookAndSelectStep::Finalize {
                    destination: Zone::Deck.to_str().to_string(),
                    source_zone: String::new(),
                },
            };
            return Ok(());
        }

        let player = gs.resolve_target_player_mut(&target);
        // 「デッキの上からN番目に置く」— numeric deck position insert
        // (PositionInfo position "4" = 4th from the TOP = index N-1).
        // The effect context is gone at answer time, so also honor the
        // position captured when the optional looked_at choice was spawned.
        // The stash is consumed unconditionally so a stale value can never
        // leak into an unrelated later selection.
        let stashed_deck_pos = self.looked_at_deck_position.take();
        let numeric_deck_pos: Option<usize> = select_action
            .as_ref()
            .and_then(|sa| sa.position_any())
            .or_else(|| current.and_then(|c| c.position_any()))
            .and_then(|pi| pi.get_position())
            .and_then(|s| s.parse::<usize>().ok())
            .or(stashed_deck_pos)
            .filter(|&n| n > 0)
            .filter(|_| {
                Zone::from_str(&destination) == Some(Zone::DeckTop)
                    || Zone::from_str(&destination) == Some(Zone::Deck)
            });
        if let Some(n) = numeric_deck_pos {
            for &card_id in &selected_cards {
                let idx = (n - 1).min(player.main_deck.cards.len());
                player.main_deck.cards.insert(idx, card_id);
                log::debug!(
                    "[LA_DECK_POS] inserted card {} at index {} ({}th from top)",
                    card_id, idx, n
                );
            }
        } else {
            for &card_id in &selected_cards {
                util::place_card_in_zone(player, card_id, &destination, None, false, 1);
            }
        }
        self.moved_cards.extend(selected_cards.iter().copied());

        let any_number = select_action
            .as_ref()
            .and_then(|sa| sa.any_number_any())
            .unwrap_or(false);
        let json_count = select_action.as_ref().and_then(|sa| sa.count).unwrap_or(1) as usize;
        let total_available = selected_count + remaining_cards.len();
        let max_select = if any_number {
            total_available
        } else {
            json_count
        };
        let can_select_more = select_action
            .as_ref()
            .map(|sa| {
                sa.max.unwrap_or(false)
                    || sa.optional.unwrap_or(false)
                    || any_number
                    || json_count > selected_count
            })
            .unwrap_or(false);

        if selected_count > 0
            && can_select_more
            && max_select > selected_count
            && !remaining_cards.is_empty()
        {
            gs.looked_at_cards = remaining_cards.clone();
            let remaining_available = gs.looked_at_cards.len();
            let remaining_selections = (max_select - selected_count).min(remaining_available);
            // Compute filtered_indices for remaining cards
            let remaining_indices: Vec<usize> = {
                let card_db = &gs.card_database;
                let filter = select_action
                    .as_ref()
                    .map(|sa| util::CardFilter::from_effect(sa));
                match filter {
                    Some(ref f) if f.has_filter() => gs
                        .looked_at_cards
                        .iter()
                        .enumerate()
                        .filter(|&(_, &cid)| f.matches(card_db, cid, false))
                        .map(|(i, _)| i)
                        .collect(),
                    _ => (0..gs.looked_at_cards.len()).collect(),
                }
            };
            let description = format!(
                "Select up to {} more card(s) from the {} remaining looked-at cards",
                remaining_selections, remaining_available
            );
            self.pending_choice = Some(
                Choice::select_cards(
                    Zone::LookedAt.to_str(),
                    remaining_selections,
                    description,
                    true,
                )
                .card_type(
                    select_action
                        .as_ref()
                        .and_then(|sa| sa.card_type_any().map(|s| s.to_string())),
                )
                .cost_limit(
                    select_action.as_ref().and_then(|sa| sa.cost_limit_any()),
                    select_action
                        .as_ref()
                        .and_then(|sa| sa.cost_limit_operator_any().map(|s| s.to_string())),
                )
                .group(
                    select_action
                        .as_ref()
                        .and_then(|sa| sa.group_names_any())
                        .and_then(|v| v.first().cloned()),
                )
                .characters(
                    select_action
                        .as_ref()
                        .and_then(|sa| sa.characters_any().cloned()),
                )
                .filtered_indices(Some(remaining_indices))
                .build(),
            );
            return Ok(());
        }

        let player = gs.resolve_target_player_mut(&target);

        // Optional move DECLINED outright (nothing selected, no explicit
        // remainder directive): the looked-at cards go back where they came
        // from. Rule 5.7 — 見る only informs; skipping 「置いてもよい」 must not
        // discard or reposition anything.
        if selected_cards.is_empty() && explicit_discard.is_none() {
            let origin = self
                .looked_at_origin
                .clone()
                .unwrap_or_else(|| Zone::DeckTop.to_str().to_string());
            for &card_id in &remaining_cards {
                util::place_card_in_zone(player, card_id, &origin, None, false, 1);
            }
            log::debug!(
                "[LA_SKIP_RETURN] {} card(s) returned to origin {}",
                remaining_cards.len(),
                origin
            );
            gs.looked_at_cards.clear();
            self.pending_choice = None;
            return Ok(());
        }

        // If the effect specifies where the REMAINING (unselected) looked-at cards
        // go (e.g. "残りを好きな順番でデッキの下に置く" → deck_bottom), honor that.
        // Otherwise fall back to discard_remaining (discard) or deck top.
        let remainder_dest = select_action
            .as_ref()
            .and_then(|sa| sa.remainder_destination_any())
            .map(|s| s.to_string())
            .or_else(|| current.and_then(|c| c.remainder_destination_any()).map(|s| s.to_string()));
        if remainder_dest.as_deref() == Some("looked_at") {
            // Intermediate leg of a multi-destination look split (希 bp3-007):
            // leftovers stay in the pool for the NEXT select step.
            gs.looked_at_cards = remaining_cards;
            self.pending_choice = None;
            return Ok(());
        }
        let dest_zone = if let Some(rd) = remainder_dest {
            rd
        } else if discard_remaining {
            Zone::Discard.to_str().to_string()
        } else {
            Zone::DeckTop.to_str().to_string()
        };
        for &card_id in &remaining_cards {
            util::place_card_in_zone(player, card_id, &dest_zone, None, false, 1);
        }
        if discard_remaining {
            // Track the discarded cards so each_time watchers (e.g. Hazuki Ren ab#1)
            // can react to them as a single batch discard event.
            self.finalize_card_movement(gs, &remaining_cards, &dest_zone, "deck_top", &None, None);
        }

        // Clear the stale looked_at choice now that all cards have been
        // processed (selected cards → hand, remaining → waitroom).
        // Without this, finalize_choice's should_preserve logic keeps the
        // original looked_at SelectCard alive through the followup_action
        // pending command, and resume_queue_with_choice re-stages it as a
        // spurious sub-choice prompt.
        self.pending_choice = None;

        Ok(())
    }

    /// Execute energy zone selection: optionally move cards to a destination.
    pub fn handle_energy_zone_selection(
        &mut self,
        gs: &mut GameState,
        indices: &[usize],
        count: usize,
        destination: Option<String>,
        validate_card: &mut impl FnMut(i16) -> bool,
    ) -> Result<(), String> {
        if let Some(ref dst) = destination {
            let cids: Vec<i16> = {
                let player = gs.resolve_target_player_mut("self");
                let mut removed = Vec::new();
                for &i in indices.iter().rev() {
                    if i < player.energy_zone.cards.len()
                        && validate_card(player.energy_zone.cards[i])
                    {
                        removed.push(player.energy_zone.cards.remove(i));
                    }
                }
                removed.reverse();
                player.energy_zone.active_energy_count = player
                    .energy_zone
                    .active_energy_count
                    .saturating_sub(removed.len() as u8);
                removed
            };
            if dst == Zone::UnderMember.to_str() {
                if cids.is_empty() {
                    // Optional placement was skipped (allow_skip). Clear the
                    // pending conditional actions (そうした場合) so subsequent
                    // effects (e.g. draw / gain_resource) do not still fire.
                    gs.ability_queue.take_pending_actions();
                    if let Some(entry) = gs.ability_queue.current_entry_mut() {
                        entry.optional_cost_result = Some(false);
                    }
                } else {
                    self.place_energy_under_member_selected(gs, &cids);
                }
            } else {
                let player = gs.resolve_target_player_mut("self");
                for &cid in &cids {
                    crate::ability::util::place_card_in_zone(player, cid, dst, None, false, 1);
                }
            }
            for &cid in &cids {
                gs.mods.clear_all_for_card(cid);
                gs.record_card_movement(cid);
            }
            self.moved_cards = cids.into();
        } else {
            self.execute_selected_energy_zone_cards(gs, indices, count)?;
        }
        Ok(())
    }

    /// Place the tapped energy cards under the activating member (Rule 10.5.3).
    /// Used for the energy_zone → under_member placement choice. Falls back to
    /// the moved_cards / center / left / right slots when the activating card is
    /// not on stage. If the chosen slot has no member, energy goes to the energy
    /// deck (Rule 10.5.4).
    fn place_energy_under_member_selected(
        &mut self,
        gs: &mut GameState,
        cids: &[i16],
    ) {
        if cids.is_empty() {
            return;
        }
        let activating_pos =
            gs.activating_card.and_then(|c| gs.resolve_target_player("self").stage.stage.iter().position(|&id| id == c));
        let player = gs.resolve_target_player_mut("self");
        let target_index = if let Some(idx) = activating_pos {
            if player.stage.stage[idx] != -1 {
                idx
            } else {
                for &card in cids {
                    player.energy_deck.cards.push(card);
                }
                return;
            }
        } else if let Some(idx) = self
            .moved_cards
            .iter()
            .rev()
            .find_map(|&cid| player.stage.stage.iter().position(|&id| id == cid))
        {
            idx
        } else if player.stage.stage[1] != -1 {
            1
        } else if player.stage.stage[0] != -1 {
            0
        } else if player.stage.stage[2] != -1 {
            2
        } else {
            for &card in cids {
                player.energy_deck.cards.push(card);
            }
            return;
        };
        if player.stage.stage[target_index] == -1 {
            for &card in cids {
                player.energy_deck.cards.push(card);
            }
            return;
        }
        let area = match target_index {
            0 => crate::zones::MemberArea::LeftSide,
            1 => crate::zones::MemberArea::Center,
            _ => crate::zones::MemberArea::RightSide,
        };
        for &card in cids {
            player.stage.place_under_card(area, card);
        }
        // Record the placement as a movement event so energy-placement watcher
        // triggers (e.g. "エネルギーがメンバーの下に置かれたとき") fire.
        let cause_pid = gs
            .ability_queue
            .current_entry()
            .map(|e| e.player_id.clone())
            .unwrap_or_default();
        let cause_cid = gs.activating_card;
        for &card in cids {
            gs.push_movement_event(
                card,
                "energy_zone",
                "under_member",
                cause_cid,
                &cause_pid,
                true,
            );
        }
        gs.recalculate_constants();
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.push_rule_log(format!(
            "{} {}: [[log_energy_under:n={}]]",
            pp, act_name, cids.len()
        ));
    }

    /// Execute energy zone cards: move to wait state.
    pub fn execute_selected_energy_zone_cards(
        &mut self,
        gs: &mut GameState,
        indices: &[usize],
        _count: usize,
    ) -> Result<(), String> {
        let player = gs.resolve_target_player_mut("self");
        let to_mark: Vec<i16> = indices
            .iter()
            .filter_map(|&i| player.energy_zone.cards.get(i).copied())
            .collect();
        player.energy_zone.active_energy_count = player
            .energy_zone
            .active_energy_count
            .saturating_sub(to_mark.len() as u8);
        for cid in to_mark {
            gs.mods.clear_all_for_card(cid);
            gs.mods.add_orientation_modifier(cid, "wait");
        }
        Ok(())
    }

    /// Handle "both" target for move_cards: process opponent first, queue self.
    pub fn execute_move_cards_both(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        // Override spawn_context.target before processing opponent — the generic
        // "both" handler in effects.rs may have set it to "self".
        self.spawn_context.target = Some("opponent".to_string());
        let mut opp_eff = effect.clone();
        opp_eff.target = Some("opponent".into());
        self.execute_move_cards(gs, &opp_eff)?;

        if self.pending_choice.is_some() {
            log::debug!("[MOVE_BOTH] Queueing self effect for later.");
            let mut self_eff = effect.clone();
            self_eff.target = Some("self".into());
            gs.ability_queue.set_pending_actions(vec![self_eff]);
            return Ok(());
        }
        log::debug!("[MOVE_BOTH] No choice created. Processing self now.");
        let mut self_eff = effect.clone();
        self_eff.target = Some("self".into());
        self.execute_move_cards(gs, &self_eff)
    }
}

