use super::{AbilityResolver, CardDatabase, Choice, ExecutionContext, GameState, Player, Zone, util};
#[cfg(feature = "no_std")]
use alloc::{string::{String, ToString}, vec::Vec};

pub(super) fn log_move_result(player: &Player, db: &CardDatabase, card_id: i16, source: &str, requested: &str) {
    if !log::log_enabled!(log::Level::Debug)
        && !log::log_enabled!(target: "rabuka_engine::events", log::Level::Debug)
    {
        return;
    }
    let (actual, public) = if player.main_deck.cards.contains(&card_id) {
        ("deck", false)
    } else if player.energy_deck.cards.contains(&card_id) {
        ("energy_deck", false)
    } else if player.hand.cards.contains(&card_id) {
        ("hand", false)
    } else if player.live_card_zone.cards.contains(&card_id) {
        ("live_card_zone", false)
    } else if player.waitroom.cards.contains(&card_id) {
        ("discard", true)
    } else if player.stage.stage.contains(&card_id) {
        ("stage", true)
    } else if player.energy_zone.cards.contains(&card_id) {
        ("energy_zone", true)
    } else if player.success_live_card_zone.cards.contains(&card_id) {
        ("success_live_zone", true)
    } else if player.stage.under_cards.iter().any(|cards| cards.contains(&card_id)) {
        ("under_member", true)
    } else {
        log::debug!("[MOVE_RESULT] owner={} source={} requested={} outcome=not_placed", player.id, source, requested);
        log::trace!("[MOVE_RESULT_CARD] id={}", card_id);
        return;
    };
    log::trace!("[MOVE_RESULT_STATE] owner={} id={} source={} requested={} actual={}", player.id, card_id, source, requested, actual);
    if public {
        log::debug!(target: "rabuka_engine::events",
            "[MOVE_COMMITTED] {}: {} [{}] (id={}) from {} to {}",
            player.id,
            db.get_card(card_id).map_or("Unknown card", |card| card.name.as_ref()),
            db.get_card(card_id).map_or("unknown", |card| card.card_no.as_ref()),
            card_id,
            crate::ability::describe::zone_label(Some(source)),
            crate::ability::describe::zone_label(Some(actual))
        );
    } else {
        log::debug!(target: "rabuka_engine::events",
            "[MOVE_COMMITTED] {}: 1 card from {} to {} (identity hidden)",
            player.id,
            crate::ability::describe::zone_label(Some(source)),
            crate::ability::describe::zone_label(Some(actual))
        );
    }
}

impl AbilityResolver {
    /// Returns Ok(true) if a choice was created, Ok(false) if the card was placed immediately.
    pub(super) fn place_card_with_stage_choice(
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
                        log_move_result(gs.resolve_target_player(player_target), &gs.card_database, card_id, source_zone, destination);
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
                log::debug!("[MOVE_PENDING] target={} destination={} reason=position_choice options={}", player_target, destination, pos_str);
                log::trace!("[MOVE_PENDING_CARD] id={}", card_id);
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
                self.execution_context = ExecutionContext::MoveCardsPosition {
                    card_id,
                    state_change,
                    target: pos_target,
                    source_zone: source_zone.to_string(),
                };
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
                log::debug!("[DEPLOY_POSITION] target={} slot={} reason=only_available_slot", player_target, slot);
                log_move_result(gs.resolve_target_player(player_target), &gs.card_database, card_id, source_zone, destination);
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
        log::trace!(
            "[PLACE_INTENT] dest={} card={} pos_to_use={:?} vacated_area={:?} stage_before={:?}",
            destination,
            card_id,
            pos_to_use,
            vacated_area,
            player.stage.stage
        );
        let placed = util::place_card_in_zone(player, card_id, destination, pos_to_use, is_max, count);
        if placed {
            log_move_result(gs.resolve_target_player(player_target), &gs.card_database, card_id, source_zone, destination);
        } else {
            log::debug!("[MOVE_SKIPPED] target={} destination={} reason=insufficient_stage_capacity", player_target, destination);
        }
        Ok(false)
    }

    /// Success-zone replacement check (e.g. 錯覚CROSSROADS): when `card_id`
    /// would be placed into the success zone and a replacement effect binds
    /// it, offer the "pick a live card from the waitroom instead" choice.
    /// ONE construction point — used by `execute_move_cards` and
    /// `execute_selected_cards_from_zone`. Returns true when a pending choice
    /// was created and the caller must stop.
    pub(super) fn maybe_prompt_success_replacement(
        &mut self,
        gs: &mut GameState,
        card_id: i16,
        dest: &str,
        target: &str,
    ) -> bool {
        if Zone::from_str(dest) != Some(Zone::SuccessLiveZone) {
            return false;
        }
        if let Some(group_names) = crate::turn::TurnEngine::get_success_replacement_info(gs, card_id)
        {
            let player = gs.resolve_target_player(target);
            let player_id = player.id.clone();
            let filtered_indices: Vec<usize> = player
                .waitroom
                .cards
                .iter()
                .enumerate()
                .filter_map(|(index, &cid)| {
                    let matches = gs.card_database.get_card(cid).is_some_and(|c| {
                        c.is_live()
                            && group_names.iter().any(|gn| {
                                crate::ability::util::card_matches_group_str(
                                    &gs.card_database,
                                    cid,
                                    Some(gn),
                                )
                            })
                            && gs.can_place_card_in_zone(
                                cid,
                                Zone::SuccessLiveZone.to_str(),
                                &player.id,
                            )
                    });
                    matches.then_some(index)
                })
                .collect();
            if !filtered_indices.is_empty() {
                gs.pending_success_replacement_card_id = Some(card_id);
                gs.pending_success_replacement_player_id = Some(player_id.to_string());
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
                .filtered_indices(Some(filtered_indices))
                .target_player_id(Some("self".to_string()))
                .build();
                self.pending_choice = Some(choice);
                return true;
            }
        }
        false
    }

    /// Shared 「山札の上または下？」 prompt: parks `card_id`'s deck placement
    /// behind a top/bottom SelectTarget. ONE construction point — used by
    /// `execute_move_cards` and `execute_selected_cards_from_zone`.
    /// Note: when `allow_skip` is set (optional deck placement), the card was
    /// left in the waitroom by resolve_cards_from_source, so a skipped answer
    /// simply leaves it there.
    pub(super) fn prompt_deck_top_or_bottom(
        &mut self,
        card_id: i16,
        state_change: Option<String>,
        target: String,
        source_zone: String,
        allow_skip: bool,
    ) {
        self.pending_choice = Some(Choice::SelectTarget {
            target: "position|destination".to_string(),
            description: "Choose deck top or bottom".to_string(),
            description_en: Some("Choose deck top or bottom".to_string()),
            description_ja: Some("山札の上または下を選択".to_string()),
            allow_skip,
            options: Some(vec![
                Zone::DeckTop.to_str().to_string(),
                Zone::DeckBottom.to_str().to_string(),
            ]),
        });
        self.execution_context = ExecutionContext::MoveCardsPosition {
            card_id,
            state_change,
            target,
            source_zone,
        };
    }

    pub(super) fn execute_stage_placement_choices(
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
                    self.fire_debut_side_effects(gs, card_id, target);
                }
                Err(_) => {
                    let player = gs.resolve_target_player_mut(target);
                    util::place_card_in_zone(player, card_id, src_zone, None, false, 1);
                }
            }
        }
        Ok(moved)
    }
}
