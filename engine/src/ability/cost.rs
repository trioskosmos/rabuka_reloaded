use super::debug::AbDebug;
use super::resolver::AbilityResolver;
use super::types::Choice;
use super::util;
use crate::card::{AbilityCost, AbilityEffect};

impl<'a> AbilityResolver<'a> {
    pub fn validate_cost(&self, cost: &AbilityCost) -> Result<(), String> {
        match cost.cost_type.as_deref() {
            Some("sequential_cost") => {
                if let Some(ref costs) = cost.costs {
                    for sub_cost in costs {
                        self.validate_cost(sub_cost)?;
                    }
                }
                Ok(())
            }
            Some("choice_condition") => Ok(()),
            Some("move_cards") => {
                let count = cost.count.unwrap_or(1) as usize;
                let source = cost.source.as_deref().unwrap_or("");
                let player = self.game_state.active_player();
                let available = match source {
                    "hand" => player.hand.cards.len(),
                    "stage" => player.stage.stage.iter().filter(|&&id| id != -1).count(),
                    "waitroom" => player.waitroom.cards.len(),
                    "energy_zone" => player.energy_zone.cards.len(),
                    _ => return Ok(()),
                };
                if available < count {
                    return Err(format!(
                        "Not enough cards in {}: need {}, have {}",
                        source, count, available
                    ));
                }
                Ok(())
            }
            Some("energy_condition") => {
                let count = cost.count.unwrap_or(1) as usize;
                let player = self.game_state.active_player();
                if player.energy_zone.cards.len() < count {
                    return Err(format!(
                        "Not enough energy cards: need {}, have {}",
                        count,
                        player.energy_zone.cards.len()
                    ));
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn pay_cost_inner(&mut self, cost: &AbilityCost) -> Result<(), String> {
        let mut dbg = AbDebug::new();
        dbg.cost_pay(cost, true);
        match cost.cost_type.as_deref() {
            Some("sequential_cost") => {
                if let Some(ref costs) = cost.costs {
                    let start_idx = self
                        .game_state
                        .ability_queue
                        .current_entry()
                        .map_or(0, |e| e.cost_paid_index);
                    for i in start_idx..costs.len() {
                        if let Err(e) = self.validate_cost(&costs[i]) {
                            return Err(format!("Cannot pay sequential cost: {}", e));
                        }
                    }
                    for i in start_idx..costs.len() {
                        self.pay_cost(&costs[i])?;
                        if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
                            entry.cost_paid_index = i + 1;
                        }
                        if self.pending_choice.is_some() {
                            return Ok(());
                        }
                    }
                }
                Ok(())
            }
            Some("choice_condition") => {
                let texts: Vec<String> = cost
                    .options
                    .as_ref()
                    .map(|o| o.iter().map(|opt| opt.text.clone()).collect())
                    .unwrap_or_default();
                self.pending_choice = Some(Choice::SelectTarget {
                    target: "choice_condition".to_string(),
                    description: format!("Choose cost option: {}", texts.join(" OR ")),
                    allow_skip: false,
                    options: None,
                });
                if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
                    entry.choice_card_no = Some("choice_cost".to_string());
                }
                Ok(())
            }
            Some("move_cards") => {
                let source = cost.source.as_deref().unwrap_or("");
                let count = cost.count.unwrap_or(1) as usize;
                let card_type = cost.card_type.clone();
                let optional = cost.optional.unwrap_or(false);
                let _text = &cost.text;
                let is_activation = self
                    .current_ability
                    .as_ref()
                    .and_then(|a| a.triggers.as_ref())
                    .map_or(false, |t| t == crate::triggers::ACTIVATION);

                let same_unit = cost.same_unit_name.unwrap_or(false);
                let is_from_hand = source == "hand" && !same_unit;
                if is_from_hand {
                    let target_str = cost.target.as_deref().unwrap_or("self");
                    let pl = self.game_state.resolve_target_player(target_str);
                    let card_db = &self.game_state.card_database;
                    let cost_limit = cost.cost_limit;
                    let card_type_filter = card_type.as_deref();
                    let filter = util::filter_from_parts(
                        card_type_filter,
                        None,
                        cost_limit,
                        None,
                        cost.characters.as_ref(),
                        None,
                        None,
                    );
                    let matching_indices: Vec<usize> = pl
                        .hand
                        .cards
                        .iter()
                        .enumerate()
                        .filter(|(_, &cid)| filter.matches(card_db, cid, false))
                        .map(|(i, _)| i)
                        .collect();
                    let is_optional = optional && !is_activation;

                    if !is_optional
                        && matching_indices.len() >= count as usize
                        && matching_indices.len() <= count as usize
                    {
                        // Non-optional exact match: auto-select (fall through to old behavior)
                    } else if is_optional && matching_indices.is_empty() {
                        // Optional cost with no matching cards: skip silently
                        return Ok(());
                    } else if !matching_indices.is_empty() {
                        // Choice needed: multiple matches, or optional with some cards
                        self.pending_choice = Some(
                            Choice::select_cards(
                                source.to_string(),
                                count,
                                format!(
                                    "Select {} card(s) from hand{}",
                                    count,
                                    if is_optional { " (or skip)" } else { "" }
                                ),
                                is_optional,
                            )
                            .card_type(card_type.clone())
                            .cost_limit(cost.cost_limit, cost.cost_limit_operator.clone())
                            .group(cost.group_names.clone().map(|v| v.join(",")))
                            .characters(cost.characters.clone())
                            .build(),
                        );
                        return Ok(());
                    }
                }
                if !source.is_empty() {
                    let target = cost.target.as_deref().unwrap_or("self");
                    let cost_limit = cost.cost_limit;
                    let card_type_filter = card_type.as_deref();

                    let player = &*self.game_state.resolve_target_player(target);
                    let card_db = &self.game_state.card_database;
                    let filter = util::filter_from_parts(
                        card_type_filter,
                        None,
                        cost_limit,
                        None,
                        cost.characters.as_ref(),
                        None,
                        None,
                    );
                    let zone_cards = util::zone_cards(player, &source);

                    if same_unit {
                        // Group source cards by unit, keep only the largest unit group
                        let mut unit_groups: std::collections::BTreeMap<String, Vec<i16>> =
                            std::collections::BTreeMap::new();
                        for &cid in zone_cards {
                            if filter.matches(card_db, cid, false) {
                                let unit = card_db
                                    .get_card(cid)
                                    .and_then(|c| c.unit.clone())
                                    .unwrap_or_default();
                                unit_groups.entry(unit).or_default().push(cid);
                            }
                        }
                        // Find the largest group
                        let best = unit_groups.into_iter().max_by_key(|(_, cards)| cards.len());
                        match best {
                            Some((_, cards)) if cards.len() >= count => {
                                // Found a unit with enough cards — modify flow to use only these cards
                                if cards.len() > count {
                                    self.pending_choice = Some(Choice::select_cards(
                                        "hand",
                                        count,
                                        format!("Select {} card(s) from same-unit group ({} available in unit {})", count, cards.len(), card_db.get_card(cards[0]).and_then(|c| c.unit.clone()).unwrap_or_default()),
                                        false,
                                    )
                                    .card_type(cost.card_type.clone())
                                    .build());
                                    return Ok(());
                                }
                                // Exactly match count — auto-select
                                for &cid in &cards {
                                    let player = self.game_state.resolve_target_player_mut(target);
                                    if let Some(idx) =
                                        player.hand.cards.iter().position(|&c| c == cid)
                                    {
                                        player.hand.cards.remove(idx);
                                        player.waitroom.cards.push(cid);
                                    }
                                }
                                return Ok(());
                            }
                            _ => {
                                return Err(format!(
                                    "Cannot pay cost: no unit has {} cards matching filter",
                                    count
                                ));
                            }
                        }
                    }

                    let matching_count = match source {
                        "deck" | "deck_top" => util::count_matching(
                            util::zone_cards(player, "deck"),
                            card_db,
                            &filter,
                            false,
                        ) as usize,
                        "hand" => util::count_matching(
                            util::zone_cards(player, "hand"),
                            card_db,
                            &filter,
                            false,
                        ) as usize,
                        "discard" => util::count_matching(
                            util::zone_cards(player, "discard"),
                            card_db,
                            &filter,
                            false,
                        ) as usize,
                        "energy_zone" => util::count_matching(
                            util::zone_cards(player, "energy_zone"),
                            card_db,
                            &filter,
                            false,
                        ) as usize,
                        _ => usize::MAX,
                    };

                    if matching_count < count {
                        return Err(format!(
                            "Cannot pay cost: {} has only {} cards matching cost limit {}, need {}",
                            source,
                            matching_count,
                            cost_limit
                                .map(|l| l.to_string())
                                .unwrap_or("none".to_string()),
                            count
                        ));
                    }
                }

                let effect = AbilityEffect {
                    text: cost.text.clone(),
                    action: cost.cost_type.clone().unwrap_or_default(),
                    source: cost.source.clone(),
                    destination: cost.destination.clone(),
                    count: cost.count,
                    card_type: cost.card_type.clone(),
                    target: cost.target.clone(),
                    self_cost: cost.self_cost,
                    exclude_self: cost.exclude_self,
                    cost_limit: cost.cost_limit,
                    state_change: cost.state_change.clone(),
                    position: cost.position.clone(),
                    effect_type: None,
                    ..Default::default()
                };
                self.execute_move_cards(&effect)
            }
            Some("change_state") => {
                let state_change = cost.state_change.as_deref().unwrap_or("");
                let target = cost.target.as_deref().unwrap_or("self");
                let optional = cost.optional.unwrap_or(false);
                let is_activation = self
                    .current_ability
                    .as_ref()
                    .and_then(|a| a.triggers.as_ref())
                    .map_or(false, |t| t == crate::triggers::ACTIVATION);

                if optional && !is_activation {
                    // For non-self_cost change_state, verify candidates exist before prompting
                    if state_change == "wait" && cost.self_cost != Some(true) {
                        let count = cost.count.unwrap_or(1) as usize;
                        let exclude_self = cost.exclude_self.unwrap_or(false);
                        let activating_id = self.game_state.activating_card;
                        let card_db = &self.game_state.card_database;
                        let group_names = cost.group_names.as_ref();

                        let stage_cards: Vec<i16> = self
                            .game_state
                            .resolve_target_player(target)
                            .stage
                            .stage
                            .iter()
                            .filter(|&&id| id != -1)
                            .copied()
                            .collect();
                        let candidates: Vec<i16> = stage_cards
                            .into_iter()
                            .filter(|id| !(exclude_self && activating_id == Some(*id)))
                            .filter(|id| {
                                super::util::card_matches_type(
                                    card_db,
                                    *id,
                                    cost.card_type.as_deref(),
                                )
                            })
                            .filter(|id| {
                                group_names.as_ref().map_or(true, |gn| {
                                    gn.iter().any(|g| {
                                        super::util::card_matches_group_str(
                                            card_db,
                                            *id,
                                            Some(g.as_str()),
                                        )
                                    })
                                })
                            })
                            .collect();
                        if candidates.is_empty() {
                            return Ok(());
                        }
                    }
                    let cost_description = if state_change == "wait" {
                        "Put this member to wait state"
                    } else {
                        "Pay cost"
                    };
                    self.pending_choice = Some(Choice::SelectTarget {
                        target: "pay_optional_cost:skip_optional_cost".to_string(),
                        description: format!(
                            "Pay optional cost: {}? (pay or skip)",
                            cost_description
                        ),
                        allow_skip: true,
                        options: None,
                    });
                    if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
                        entry.choice_card_no = Some("optional_cost".to_string());
                    }
                    return Ok(());
                }

                if state_change == "wait" {
                    let count = cost.count.unwrap_or(1) as usize;
                    let exclude_self = cost.exclude_self.unwrap_or(false);
                    let activating_id = self.game_state.activating_card;
                    let card_db = &self.game_state.card_database;
                    let group_names = cost.group_names.as_ref();

                    let stage_cards: Vec<i16> = self
                        .game_state
                        .resolve_target_player(target)
                        .stage
                        .stage
                        .iter()
                        .filter(|&&id| id != -1)
                        .copied()
                        .collect();
                    eprintln!("[CHANGE_STATE] stage={:?} card_type={:?} exclude_self={} activating_id={:?} self_cost={:?}",
                        stage_cards, cost.card_type, exclude_self, activating_id, cost.self_cost);
                    let candidates: Vec<i16> = stage_cards
                        .into_iter()
                        .filter(|&id| {
                            // When self_cost is true, only include the activating card
                            if cost.self_cost.unwrap_or(false) {
                                activating_id == Some(id)
                            } else {
                                // When exclude_self is true, exclude the activating card
                                !(exclude_self && activating_id == Some(id))
                            }
                        })
                        .filter(|&id| {
                            if !super::util::card_matches_type(
                                card_db,
                                id,
                                cost.card_type.as_deref(),
                            ) {
                                eprintln!("[CHANGE_STATE] id={} FAIL type match", id);
                                return false;
                            }
                            let group_ok = match group_names {
                                Some(gn) => gn.iter().any(|g| {
                                    super::util::card_matches_group_str(
                                        card_db,
                                        id,
                                        Some(g.as_str()),
                                    )
                                }),
                                None => true,
                            };
                            let name_ok =
                                super::util::card_matches_characters(card_db, id, group_names);
                            let ok = group_ok || name_ok;
                            eprintln!(
                                "[CHANGE_STATE] id={} group_ok={} name_ok={} ok={}",
                                id, group_ok, name_ok, ok
                            );
                            ok
                        })
                        .collect();
                    eprintln!("[CHANGE_STATE] candidates={:?}", candidates);

                    if candidates.is_empty() {
                        return Err("No matching members on stage to change state".to_string());
                    }

                    if candidates.len() <= count {
                        for &card_id in &candidates {
                            self.game_state
                                .mods
                                .add_orientation_modifier(card_id, "wait");
                        }
                    } else {
                        self.pending_choice = Some(
                            Choice::select_cards(
                                "stage",
                                count,
                                format!("Select {} stage member(s) to wait", count),
                                false,
                            )
                            .card_type(cost.card_type.clone())
                            .build(),
                        );
                        return Ok(());
                    }
                }
                Ok(())
            }
            Some("pay_energy") => {
                let energy = cost.energy.unwrap_or(0);
                let target = cost.target.as_deref().unwrap_or("self");
                let optional = cost.optional.unwrap_or(false);
                let is_activation = self
                    .current_ability
                    .as_ref()
                    .and_then(|a| a.triggers.as_ref())
                    .map_or(false, |t| t == crate::triggers::ACTIVATION);

                if optional && !is_activation {
                    self.pending_choice = Some(Choice::SelectTarget {
                        target: "pay_optional_cost:skip_optional_cost".to_string(),
                        description: format!("Pay {} energy (or skip)?", energy),
                        allow_skip: true,
                        options: None,
                    });
                    if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
                        entry.choice_card_no = Some("optional_cost".to_string());
                    }
                    return Ok(());
                }

                if self.game_state.baton_touch_zero_cost && energy > 0 {
                    eprintln!(
                        "Skipping pay_energy cost of {} due to baton touch zero cost",
                        energy
                    );
                    return Ok(());
                }

                let player = self.game_state.resolve_target_player_mut(target);

                if energy > 0 {
                    if let Err(e) = player.energy_zone.pay_energy(energy as usize) {
                        return Err(e);
                    }
                }
                Ok(())
            }
            Some("energy_condition") => {
                let count = cost.count.unwrap_or(1) as usize;
                let target = cost.target.as_deref().unwrap_or("self");
                let player = self.game_state.resolve_target_player_mut(target);
                if player.energy_zone.cards.len() < count {
                    return Err(format!(
                        "Not enough energy cards: need {}, have {}",
                        count,
                        player.energy_zone.cards.len()
                    ));
                }
                for _ in 0..count {
                    if let Some(card) = player.energy_zone.cards.pop() {
                        player.energy_deck.cards.push(card);
                    }
                }
                player.energy_zone.active_energy_count =
                    player.energy_zone.active_energy_count.saturating_sub(count);
                Ok(())
            }
            Some("reveal") => {
                let source = cost.source.as_deref().unwrap_or("hand");
                let target = cost.target.as_deref().unwrap_or("self");
                let card_type = cost.card_type.clone();

                let card_ids: Vec<i16> = {
                    let player = &*self.game_state.resolve_target_player(target);
                    let card_db = &self.game_state.card_database;
                    match source {
                        "hand" => player
                            .hand
                            .cards
                            .iter()
                            .filter(|&&id| {
                                super::util::card_matches_type(card_db, id, card_type.as_deref())
                            })
                            .copied()
                            .collect(),
                        _ => vec![],
                    }
                };

                if card_ids.is_empty() {
                    return Err("No cards to reveal".to_string());
                }

                let has_explicit_count = cost.count.is_some();
                let explicit_count = cost.count.unwrap_or(1) as usize;

                if has_explicit_count && card_ids.len() <= explicit_count {
                    for card_id in card_ids {
                        self.game_state.revealed_cards.push(card_id);
                        self.revealed_cost_cards.push(card_id);
                    }
                    Ok(())
                } else {
                    let group = cost.group_names.as_ref().and_then(|gn| gn.first().cloned());
                    self.pending_choice = Some(
                        Choice::select_cards(
                            source.to_string(),
                            if has_explicit_count {
                                explicit_count
                            } else {
                                0
                            },
                            "Select cards to reveal from hand".to_string(),
                            true,
                        )
                        .card_type(card_type.clone())
                        .cost_limit(cost.cost_limit, cost.cost_limit_operator.clone())
                        .group(group)
                        .characters(cost.characters.clone())
                        .build(),
                    );
                    Ok(())
                }
            }
            Some("place_energy_under_member") => {
                self.execute_place_energy_under_member(
                    cost.count.unwrap_or(1),
                    cost.target.as_deref().unwrap_or("self"),
                    cost.position.as_ref(),
                    cost.optional.unwrap_or(false),
                    cost.source.as_deref(),
                );
                Ok(())
            }
            Some("custom") => {
                if cost.destination.as_deref() == Some("under_member") {
                    self.execute_place_energy_under_member(
                        cost.count.unwrap_or(1),
                        cost.target.as_deref().unwrap_or("self"),
                        cost.position.as_ref(),
                        cost.optional.unwrap_or(false),
                        None,
                    );
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn pay_cost(&mut self, cost: &AbilityCost) -> Result<(), String> {
        self.pay_cost_inner(cost)
    }

    pub fn handle_optional_cost_payment(&mut self, selected: &str) -> Result<(), String> {
        if selected == "skip_optional_cost" || selected == "0" {
            self.pending_choice = None;
            if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
                entry.cost_paid = true;
                entry.effect_started = true;
                entry.optional_cost_was_paid = false;
            }
            return Ok(());
        }
        // "pay_optional_cost" or "1" from select_option(1)
        self.pending_choice = None;
        if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
            entry.optional_cost_was_paid = true;
        }
        let is_pay = true;
        if is_pay {
            if let Some(cost) = self.game_state.entry_cost().cloned() {
                if let Some(energy) = cost.energy {
                    if energy > 0 {
                        let tgt = cost.target.as_deref().unwrap_or("self");
                        self.game_state
                            .resolve_target_player_mut(tgt)
                            .energy_zone
                            .pay_energy(energy as usize)
                            .map_err(|e| e)?;
                    }
                }
                if cost.state_change.as_deref() == Some("wait") {
                    if cost.self_cost == Some(true) {
                        if let Some(id) = self.game_state.activating_card {
                            self.game_state.mods.add_orientation_modifier(id, "wait");
                        }
                    } else {
                        let target = cost.target.as_deref().unwrap_or("self");
                        let count = cost.count.unwrap_or(1) as usize;
                        let card_db = &self.game_state.card_database;
                        let group_names = cost.group_names.as_ref();
                        let exclude_self = cost.exclude_self.unwrap_or(false);
                        let activating_id = self.game_state.activating_card;
                        let state_change = cost.state_change.as_deref().unwrap_or("");

                        let stage_cards: Vec<i16> = self
                            .game_state
                            .resolve_target_player(target)
                            .stage
                            .stage
                            .iter()
                            .filter(|&&id| id != -1)
                            .copied()
                            .collect();
                        let candidates: Vec<i16> = stage_cards
                            .into_iter()
                            .filter(|id| !(exclude_self && activating_id == Some(*id)))
                            .filter(|id| {
                                super::util::card_matches_type(
                                    card_db,
                                    *id,
                                    cost.card_type.as_deref(),
                                )
                            })
                            .filter(|id| {
                                let group_ok = match group_names {
                                    Some(gn) => gn.iter().any(|g| {
                                        super::util::card_matches_group_str(
                                            card_db,
                                            *id,
                                            Some(g.as_str()),
                                        )
                                    }),
                                    None => true,
                                };
                                group_ok
                            })
                            .collect();

                        if candidates.is_empty() {
                            return Err("No matching members on stage to change state".to_string());
                        }

                        let to_wait: Vec<i16> = candidates.into_iter().take(count).collect();
                        for &card_id in &to_wait {
                            if state_change == "wait" {
                                self.game_state
                                    .mods
                                    .add_orientation_modifier(card_id, "wait");
                            } else if state_change == "rest" || state_change == "rested" {
                                self.game_state
                                    .mods
                                    .add_orientation_modifier(card_id, "rest");
                            }
                        }
                    }
                }
                // Handle sequential_cost sub-costs — pay each after user confirmed
                if let Some(ref costs) = cost.costs {
                    for sub_cost in costs {
                        if sub_cost.state_change.as_deref() == Some("wait")
                            && sub_cost.self_cost == Some(true)
                        {
                            if let Some(id) = self.game_state.activating_card {
                                self.game_state.mods.add_orientation_modifier(id, "wait");
                            }
                        } else if let Err(e) = self.pay_cost(sub_cost) {
                            eprintln!("Warning: sub-cost payment error: {}", e);
                        }
                        if self.pending_choice.is_some() {
                            return Ok(());
                        }
                    }
                }
                eprintln!("[OPT_COST] checking cost_type: {:?}, entry_cost: {:?}, entry_effect_action: {:?}", 
                    cost.cost_type, self.game_state.entry_cost().is_some(), 
                    self.game_state.entry_effect().map(|e| e.action.clone()));
                if cost.cost_type.as_deref() == Some("place_energy_under_member") {
                    self.execute_place_energy_under_member(
                        cost.count.unwrap_or(1),
                        cost.target.as_deref().unwrap_or("self"),
                        cost.position.as_ref(),
                        false,
                        cost.source.as_deref(),
                    );
                }
            }
            self.pending_choice = None;
            let is_effect_optional =
                self.game_state.entry_choice_card_no().as_deref() == Some("optional_cost");
            if self.game_state.entry_cost().is_some() {
                if let Some(effect) = self.game_state.entry_effect().cloned() {
                    if let Err(e) = self.execute_effect(&effect) {
                        eprintln!("Failed to execute effect after optional cost: {}", e);
                    }
                    if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
                        entry.effect_started = true;
                    }
                }
            } else if self.game_state.ability_queue.has_pending_commands() {
                if let Err(e) = self.resume_pending_commands() {
                    eprintln!("Failed to execute action after optional: {}", e);
                }
            } else if is_effect_optional {
                if let Some(effect) = self.game_state.entry_effect().cloned() {
                    let new_count = effect.energy_count.unwrap_or(effect.count_or(1));
                    self.execute_place_energy_under_member(
                        new_count,
                        effect.target_name(),
                        effect.position.as_ref(),
                        false,
                        effect.source.as_deref(),
                    );
                }
            }
        }
        Ok(())
    }
}
