impl GameState {
    /// Re-evaluate all constant (常時) abilities on all stage members.
    /// Handles gain_resource(blade, heart), modify_score, modify_cost.
    /// Clears old constant-derived values and re-applies those whose conditions pass.
    pub fn recalculate_constants(&mut self) {
        let entries = self.collect_constant_stage_effects();

        let mut exp_blade: std::collections::HashMap<i16, i32> = std::collections::HashMap::new();
        let mut exp_cost: std::collections::HashMap<i16, i32> = std::collections::HashMap::new();
        let mut exp_score: std::collections::HashMap<i16, i32> = std::collections::HashMap::new();
        let mut exp_heart: std::collections::HashMap<i16, std::collections::HashMap<String, i32>> =
            std::collections::HashMap::new();
        let mut exp_prohibition: Vec<String> = Vec::new();
        let mut exp_global_need_heart: Vec<(i16, String, i32)> = Vec::new();

        // Compute stage positions for all entries before creating resolver
        let mut entry_positions: std::collections::HashMap<i16, Option<usize>> =
            std::collections::HashMap::new();
        for &cid in self
            .player1
            .stage
            .stage
            .iter()
            .chain(self.player2.stage.stage.iter())
        {
            if cid == -1 {
                continue;
            }
            let pos = self
                .player1
                .stage
                .stage
                .iter()
                .position(|&c| c == cid)
                .or_else(|| self.player2.stage.stage.iter().position(|&c| c == cid));
            entry_positions.insert(cid, pos);
        }

        for (card_id, effect) in &entries {
            // Set activating_card so condition evaluators (e.g. exclude_self in
            // location_condition) know which card is "self" for this entry.
            let prev_activating = self.activating_card;
            self.activating_card = Some(*card_id);

            {
                let ctx = crate::ability::condition::ConditionContext::new(self);

                // Check effect-level position requirement
                let pos_ok = if let Some(ref pos) = effect.position {
                    let pos_str = pos.get_position();
                    let card_pos = entry_positions.get(card_id).copied().flatten();
                    matches!(
                        (pos_str, card_pos),
                        (Some("center"), Some(1))
                            | (Some("left") | Some("left_side"), Some(0))
                            | (Some("right") | Some("right_side"), Some(2))
                            | (None, _)
                    )
                } else {
                    true
                };

                if pos_ok {
                    let cond_met = effect
                        .condition
                        .as_ref()
                        .map_or(true, |c| ctx.evaluate_condition(c));

                    if cond_met {
                        match crate::ability::enums::ActionType::from_str(&effect.action) {
                            Some(crate::ability::enums::ActionType::GainResource) => {
                                match effect.resource.as_deref().unwrap_or("") {
                                    "blade" | "ブレード" => {
                                        let n = if effect.per_unit.unwrap_or(false) {
                                            let player =
                                                if self.player1.stage.stage.contains(card_id) {
                                                    &self.player1
                                                } else {
                                                    &self.player2
                                                };
                                            let zone = effect
                                                .location
                                                .as_deref()
                                                .or(effect.per_unit_type.as_deref())
                                                .unwrap_or(Zone::Hand.to_str());
                                            let filter = crate::ability::util::CardFilter {
                                                card_type: effect.card_type.as_deref(),
                                                ..Default::default()
                                            };
                                            let per_count =
                                                crate::ability::util::resolve_per_unit_count(
                                                    true,
                                                    Some(zone),
                                                    player,
                                                    &self.card_database,
                                                    &filter,
                                                    &[],
                                                );
                                            let base = effect.resource_icon_count.unwrap_or(1);
                                            let mut units = per_count as i32
                                                / effect.per_unit_count.unwrap_or(1).max(1) as i32;
                                            if effect.max.unwrap_or(false) {
                                                if let Some(cap) = effect.count {
                                                    units = units.min(cap as i32);
                                                }
                                            }
                                            units * base as i32
                                        } else {
                                            effect
                                                .resource_icon_count
                                                .unwrap_or(effect.count.unwrap_or(1))
                                                as i32
                                        };
                                        *exp_blade.entry(*card_id).or_insert(0) += n;
                                    }
                                    "heart" | "ハート" => {
                                        let n = if effect.per_unit.unwrap_or(false) {
                                            let player =
                                                if self.player1.stage.stage.contains(card_id) {
                                                    &self.player1
                                                } else {
                                                    &self.player2
                                                };
                                            let zone = effect
                                                .location
                                                .as_deref()
                                                .or(effect.per_unit_type.as_deref())
                                                .unwrap_or(Zone::Hand.to_str());
                                            let filter = crate::ability::util::CardFilter {
                                                card_type: effect.card_type.as_deref(),
                                                ..Default::default()
                                            };
                                            let per_count =
                                                crate::ability::util::resolve_per_unit_count(
                                                    true,
                                                    Some(zone),
                                                    player,
                                                    &self.card_database,
                                                    &filter,
                                                    &effect.heart_colors,
                                                );
                                            let mut units = per_count as i32
                                                / effect.per_unit_count.unwrap_or(1).max(1) as i32;
                                            if effect.max.unwrap_or(false) {
                                                if let Some(cap) = effect.count {
                                                    units = units.min(cap as i32);
                                                }
                                            }
                                            units
                                        } else {
                                            effect.count.unwrap_or(1) as i32
                                        };
                                        for hc in &effect.heart_colors {
                                            *exp_heart
                                                .entry(*card_id)
                                                .or_default()
                                                .entry(hc.clone())
                                                .or_insert(0) += n;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            Some(crate::ability::enums::ActionType::ModifyScore) => {
                                *exp_score.entry(*card_id).or_insert(0) +=
                                    effect.value.unwrap_or(0) as i32;
                            }
                            Some(crate::ability::enums::ActionType::ModifyCost) => {
                                *exp_cost.entry(*card_id).or_insert(0) +=
                                    effect.value.unwrap_or(0) as i32;
                            }
                            Some(crate::ability::enums::ActionType::Restriction) => {
                                if let Some(ref rt) = effect.restriction_type {
                                    exp_prohibition.push(format!("const_restriction:{}:", rt));
                                }
                            }
                            Some(crate::ability::enums::ActionType::GainAbility) => {
                                if effect.ability_gain.as_deref() == Some("{{icon_all.png|ハート}}")
                                    || effect
                                        .ability_gain
                                        .as_deref()
                                        .map_or(false, |t| t.contains("ALL"))
                                {
                                    let all_colors = [
                                        "heart01", "heart02", "heart03", "heart04", "heart05",
                                        "heart06",
                                    ];
                                    for hc in &all_colors {
                                        *exp_heart
                                            .entry(*card_id)
                                            .or_default()
                                            .entry(hc.to_string())
                                            .or_insert(0) += 1i32;
                                    }
                                }
                            }
                            Some(crate::ability::enums::ActionType::GainAbilityFromSource) => {
                                let mut resolver = crate::ability::resolver::AbilityResolver::new(
                                    self.card_database.clone(),
                                    self.activating_card,
                                );
                                let _ = resolver.execute_gain_ability_from_source(self, &effect);
                            }
                            Some(crate::ability::enums::ActionType::ModifyRequiredHeartsGlobal) => {
                                let target_name = effect.target_name();
                                let target_player = self.resolve_target_player(&target_name);
                                let target_cards: Vec<i16> =
                                    target_player.live_card_zone.cards.to_vec();
                                let value = effect.value_or_count(1) as i32;
                                let op = effect.operation.as_deref().unwrap_or("increase");
                                let delta = match op {
                                    "increase" => value,
                                    "decrease" => -value,
                                    _ => value,
                                };
                                let colors: Vec<String> = if effect.heart_colors.is_empty() {
                                    vec!["heart00".to_string()]
                                } else {
                                    effect.heart_colors.clone()
                                };
                                for card_id in &target_cards {
                                    for color in &colors {
                                        exp_global_need_heart.push((
                                            *card_id,
                                            color.clone(),
                                            delta,
                                        ));
                                    }
                                }
                            }
                            Some(crate::ability::enums::ActionType::Sequential) => {
                                if let Some(ref actions) = effect.compound.actions {
                                    for sub in actions {
                                        match crate::ability::enums::ActionType::from_str(
                                            &sub.action,
                                        ) {
                                            Some(
                                                crate::ability::enums::ActionType::GainResource,
                                            ) => match sub.resource.as_deref().unwrap_or("") {
                                                "blade" | "ブレード" => {
                                                    let n = sub
                                                        .resource_icon_count
                                                        .unwrap_or(sub.count.unwrap_or(1))
                                                        as i32;
                                                    *exp_blade.entry(*card_id).or_insert(0) += n;
                                                }
                                                "heart" | "ハート" => {
                                                    let n = sub.count.unwrap_or(1) as i32;
                                                    for hc in &sub.heart_colors {
                                                        *exp_heart
                                                            .entry(*card_id)
                                                            .or_default()
                                                            .entry(hc.clone())
                                                            .or_insert(0) += n;
                                                    }
                                                }
                                                _ => {}
                                            },
                                            _ => {}
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Restore the previous activating_card
            self.activating_card = prev_activating;
        }

        // Blade
        let old_blade = std::mem::take(&mut self.mods.constant_blade_bonuses);
        for (cid, val) in &old_blade {
            self.mods.remove_blade_modifier(*cid, *val);
        }
        for (&cid, &val) in &exp_blade {
            self.mods.add_blade_modifier(cid, val);
        }
        self.mods.constant_blade_bonuses = exp_blade;

        // Cost
        let old_cost = std::mem::take(&mut self.mods.constant_cost_bonuses);
        for (cid, val) in &old_cost {
            self.mods.remove_cost_modifier(*cid, *val);
        }
        for (&cid, &val) in &exp_cost {
            self.mods.add_cost_modifier(cid, val);
        }
        self.mods.constant_cost_bonuses = exp_cost;

        // Score
        let old_score = std::mem::take(&mut self.mods.constant_score_bonuses);
        for (cid, val) in &old_score {
            self.mods.remove_score_modifier(*cid, *val);
        }
        for (&cid, &val) in &exp_score {
            self.mods.add_score_modifier(cid, val);
        }
        self.mods.constant_score_bonuses = exp_score;

        // Heart — clear old constant heart modifiers first, then re-apply new ones.
        // Must drain the OLD map so bonuses from cards that left the stage are removed.
        let old_heart = std::mem::take(&mut self.mods.constant_heart_bonuses);
        for (cid, cols) in &old_heart {
            for (color_str, &delta) in cols {
                let hc = crate::card::parse_heart_color(color_str);
                self.mods.remove_heart_modifier(*cid, hc, delta);
            }
        }
        for (cid, cols) in &exp_heart {
            for (color_str, delta) in cols {
                let hc = crate::card::parse_heart_color(color_str);
                self.mods.add_heart_modifier(*cid, hc, *delta);
            }
        }
        self.mods.constant_heart_bonuses = exp_heart;

        // Apply restriction effects from constant abilities.
        // Use "const_restriction:" prefix to distinguish from debut/live ability restrictions
        // so we can safely clear and re-add constant restrictions on each recalculate call.
        self.prohibition_effects
            .retain(|p| !p.starts_with("const_restriction:"));
        for p in &exp_prohibition {
            self.prohibition_effects.push(p.clone());
        }

        // Clear old constant global need_heart modifiers, then re-apply new ones.
        let old_global_nh = std::mem::take(&mut self.mods.constant_global_need_heart);
        for (card_id, color_str, delta) in &old_global_nh {
            let hc = crate::card::parse_heart_color(color_str);
            self.mods.add_need_heart_modifier(*card_id, hc, -*delta);
        }
        for (card_id, color_str, delta) in &exp_global_need_heart {
            let hc = crate::card::parse_heart_color(color_str);
            self.mods.add_need_heart_modifier(*card_id, hc, *delta);
        }
        self.mods.constant_global_need_heart = exp_global_need_heart;

        // Also recalculate cost modifiers from hand cards (hand-based cost reductions)
        self.recalculate_constant_cost_modifiers();
    }

    pub fn recalculate_constant_blade_modifiers(&mut self) {
        let blade_abilities: Vec<(i16, crate::card::AbilityEffect)> = self
            .collect_constant_stage_effects()
            .into_iter()
            .filter(|(_, effect)| {
                crate::ability::enums::ActionType::from_str(&effect.action)
                    == Some(crate::ability::enums::ActionType::GainResource)
                    && matches!(effect.resource.as_deref(), Some("blade") | Some("ブレード"))
            })
            .collect();

        let mut expected: std::collections::HashMap<i16, i32> = std::collections::HashMap::new();
        {
            let ctx = crate::ability::condition::ConditionContext::new(self);
            for &(cid, ref effect) in &blade_abilities {
                let cond_met = effect
                    .condition
                    .as_ref()
                    .map_or(true, |c| ctx.evaluate_condition(c));
                if cond_met {
                    let count = if effect.per_unit.unwrap_or(false) {
                        let player = if self.player1.stage.stage.contains(&cid) {
                            &self.player1
                        } else {
                            &self.player2
                        };
                        let zone = effect
                            .location
                            .as_deref()
                            .or(effect.per_unit_type.as_deref())
                            .unwrap_or(crate::ability::enums::Zone::Hand.to_str());
                        let filter = crate::ability::util::CardFilter {
                            card_type: effect.card_type.as_deref(),
                            ..Default::default()
                        };
                        let per_count = crate::ability::util::resolve_per_unit_count(
                            true,
                            Some(zone),
                            player,
                            &self.card_database,
                            &filter,
                            &[],
                        );
                        let base = effect.resource_icon_count.unwrap_or(1);
                        let mut units =
                            per_count as i32 / effect.per_unit_count.unwrap_or(1).max(1) as i32;
                        if effect.max.unwrap_or(false) {
                            if let Some(cap) = effect.count {
                                units = units.min(cap as i32);
                            }
                        }
                        units * base as i32
                    } else {
                        effect
                            .resource_icon_count
                            .unwrap_or(effect.count.unwrap_or(1)) as i32
                    };
                    *expected.entry(cid).or_insert(0) += count;
                }
            }
        }

        let old_bonuses = std::mem::take(&mut self.mods.constant_blade_bonuses);
        for (cid, old) in &old_bonuses {
            self.mods.remove_blade_modifier(*cid, *old);
        }
        for (&cid, &new_val) in &expected {
            self.mods.add_blade_modifier(cid, new_val);
        }
        self.mods.constant_blade_bonuses = expected;
        self.recalculate_constant_cost_modifiers();
    }

    pub fn recalculate_constant_cost_modifiers(&mut self) {
        let mut cost_abilities: Vec<(i16, crate::card::AbilityEffect)> = self
            .collect_constant_stage_effects()
            .into_iter()
            .filter(|(_, effect)| {
                crate::ability::enums::ActionType::from_str(&effect.action)
                    == Some(crate::ability::enums::ActionType::ModifyCost)
            })
            .collect();
        let hand_cost_abilities =
            self.collect_constant_hand_effects()
                .into_iter()
                .filter(|(_, effect)| {
                    crate::ability::enums::ActionType::from_str(&effect.action)
                        == Some(crate::ability::enums::ActionType::ModifyCost)
                });
        cost_abilities.extend(hand_cost_abilities);

        let mut expected: std::collections::HashMap<i16, i32> = std::collections::HashMap::new();
        {
            let ctx = crate::ability::condition::ConditionContext::new(self);
            for &(cid, ref effect) in &cost_abilities {
                let cond_met = effect
                    .condition
                    .as_ref()
                    .map_or(true, |c| ctx.evaluate_condition(c));
                if cond_met {
                    let mut value = effect.value.unwrap_or(0) as i32;

                    // Handle per_unit cost reduction (e.g. "1 per other card in hand")
                    if effect.per_unit.unwrap_or(false) {
                        let player = self.resolve_target_player(&effect.target_name());
                        // per_unit_location overrides the counting zone when the
                        // parser determines the per-unit count targets a different
                        // zone than the effect's location (e.g. count stage members
                        // while the cost modifier itself applies to hand cards).
                        let count_zone = effect
                            .per_unit_location
                            .as_deref()
                            .or(effect.location.as_deref())
                            .unwrap_or(Zone::Hand.to_str());
                        let count = if count_zone == "stage" && effect.group_names.is_some() {
                            let group_name = effect.group_name();
                            let card_db = &self.card_database;
                            let stage_ids: Vec<i16> = player
                                .stage
                                .stage
                                .iter()
                                .copied()
                                .filter(|&id| id != -1)
                                .collect();
                            eprintln!(
                                "[COST_MOD_PER_UNIT_DEBUG] stage_ids={:?} group_name={:?}",
                                stage_ids, group_name
                            );
                            let matches = stage_ids
                                .iter()
                                .filter(|&&id| {
                                    crate::ability::util::card_matches_group_str(
                                        card_db, id, group_name,
                                    )
                                })
                                .count();
                            eprintln!("[COST_MOD_PER_UNIT_DEBUG] group_matches={}", matches);
                            matches as u32
                        } else {
                            let cards: Vec<i16> =
                                crate::ability::util::zone_cards(player, count_zone).to_vec();
                            cards.len() as u32
                        };
                        eprintln!(
                            "[COST_MOD_PER_UNIT] cid={} count_zone={} count={}",
                            cid, count_zone, count
                        );
                        let per_unit_count = effect.per_unit_count.unwrap_or(1);
                        let exclude_self = effect.exclude_self.unwrap_or(false);
                        let effective = if exclude_self {
                            count.saturating_sub(1)
                        } else {
                            count
                        };
                        value = ((effective / per_unit_count) * (value as u32)) as i32;
                        eprintln!("[COST_MOD] cid={} zone={} count={} eff={} per_unit_cnt={} val={} exclude={}",
                            cid, count_zone, count, effective, per_unit_count, value, exclude_self);
                    }
                    eprintln!(
                        "[COST_MOD] cid={} op={:?} val={}",
                        cid,
                        effect.operation.as_deref(),
                        value
                    );

                    let op = effect.operation.as_deref().unwrap_or("add");
                    match op {
                        "add" => *expected.entry(cid).or_insert(0) += value,
                        "subtract" => *expected.entry(cid).or_insert(0) -= value,
                        "set" => {
                            expected.insert(cid, value);
                        }
                        _ => {}
                    }
                }
            }
        }

        let old_bonuses = std::mem::take(&mut self.mods.constant_cost_bonuses);
        for (cid, old) in &old_bonuses {
            self.mods.remove_cost_modifier(*cid, *old);
        }
        for (&cid, &new_val) in &expected {
            self.mods.add_cost_modifier(cid, new_val);
        }
        self.mods.constant_cost_bonuses = expected;
    }

    pub fn set_heart_override(
        &mut self,
        card_id: i16,
        color: crate::card::HeartColor,
        count: u32,
        duration: &str,
    ) {
        self.mods.set_heart_override(card_id, color, count);
        let mut data = serde_json::Map::new();
        data.insert(
            "card_id".to_string(),
            serde_json::Value::Number(card_id.into()),
        );
        data.insert(
            "color".to_string(),
            serde_json::Value::String(format!("{:?}", color)),
        );
        data.insert("count".to_string(), serde_json::Value::Number(count.into()));
        self.temporary_effects.push(TemporaryEffect {
            effect_type: "heart_override".to_string(),
            duration: match duration {
                "live_end" => Duration::LiveEnd,
                "this_turn" => Duration::ThisTurn,
                _ => Duration::ThisLive,
            },
            created_turn: self.turn_number,
            created_phase: self.current_phase.clone(),
            target_player_id: String::new(),
            description: format!("Heart override: card {} = {:?} x{}", card_id, color, count),
            creation_order: 0,
            effect_data: Some(serde_json::Value::Object(data)),
        });
    }

    pub fn record_area_placement(&mut self, player_id: &str, area: &str) {
        let key = format!("{}:{}", player_id, area);
        self.areas_placed_this_turn.insert(key);
    }

    pub fn has_area_been_placed_this_turn(&self, player_id: &str, area: &str) -> bool {
        let key = format!("{}:{}", player_id, area);
        self.areas_placed_this_turn.contains(&key)
    }

    pub fn clear_area_placement_tracking(&mut self) {
        self.areas_placed_this_turn.clear();
    }

    pub fn record_card_appearance(&mut self, card_id: i16) {
        self.cards_appeared_this_turn.insert(card_id);
    }

    pub fn has_card_appeared_this_turn(&self, card_id: i16) -> bool {
        self.cards_appeared_this_turn.contains(&card_id)
    }

    pub fn clear_card_appearance_tracking(&mut self) {
        self.cards_appeared_this_turn.clear();
    }

    pub fn set_turn_order_changed(&mut self, changed: bool) {
        self.turn_order_changed = changed;
    }

    pub fn has_turn_order_changed(&self) -> bool {
        self.turn_order_changed
    }

    pub fn record_auto_ability_trigger(&mut self, card_id: &str) {
        *self
            .auto_ability_trigger_counts
            .entry(card_id.to_string())
            .or_insert(0) += 1;
    }

    pub fn get_auto_ability_trigger_count(&self, card_id: &str) -> u32 {
        *self.auto_ability_trigger_counts.get(card_id).unwrap_or(&0)
    }

    pub fn clear_auto_ability_trigger_tracking(&mut self) {
        self.auto_ability_trigger_counts.clear();
    }

    pub fn record_turn_limit_usage(&mut self, player_id: &str, card_instance_id: u32) {
        let key = format!("{}:{}", player_id, card_instance_id);
        *self.turn_limit_usage.entry(key).or_insert(0) += 1;
    }

    pub fn get_turn_limit_usage(&self, player_id: &str, card_instance_id: u32) -> u32 {
        let key = format!("{}:{}", player_id, card_instance_id);
        *self.turn_limit_usage.get(&key).unwrap_or(&0)
    }

    pub fn clear_turn_limit_tracking(&mut self) {
        self.turn_limit_usage.clear();
    }

    pub fn assign_card_instance_id(&mut self, card_id: i16) -> u32 {
        self.card_instance_counter += 1;
        let instance_id = self.card_instance_counter;
        self.card_instance_mapping.insert(card_id, instance_id);
        instance_id
    }

    pub fn get_card_instance_id(&self, card_id: i16) -> Option<u32> {
        self.card_instance_mapping.get(&card_id).copied()
    }

    pub fn remove_card_instance(&mut self, card_id: i16) {
        self.card_instance_mapping.remove(&card_id);
    }

    pub fn clear_card_instance_tracking(&mut self) {
        self.card_instance_mapping.clear();
        self.card_instance_counter = 0;
    }

    pub fn record_baton_touch(&mut self) {
        self.baton_touch_count += 1;
    }

    pub fn get_baton_touch_count(&self) -> u32 {
        self.baton_touch_count
    }

    pub fn clear_baton_touch_tracking(&mut self) {
        self.baton_touch_count = 0;
        self.baton_touch_zero_cost = false;
        self.baton_touch_replaced_member_cost = None;
        self.baton_touch_replaced_member_id = None;
    }

    pub fn record_card_movement(&mut self, card_id: i16) {
        self.cards_moved_this_turn.insert(card_id);
    }

    pub fn has_card_moved_this_turn(&self, card_id: i16) -> bool {
        self.cards_moved_this_turn.contains(&card_id)
    }

    pub fn clear_card_movement_tracking(&mut self) {
        self.cards_moved_this_turn.clear();
    }

    pub fn set_heart_color_decision_phase(&mut self, phase: &str) {
        self.heart_color_decision_phase = phase.to_string();
    }

    pub fn get_heart_color_decision_phase(&self) -> &str {
        &self.heart_color_decision_phase
    }

    pub fn is_in_required_hearts_check_phase(&self) -> bool {
        self.heart_color_decision_phase == "required_hearts_check"
    }

    pub fn is_in_live_start_phase(&self) -> bool {
        self.heart_color_decision_phase == "live_start"
    }

    pub fn set_deck_refresh_pending(&mut self, pending: bool) {
        self.deck_refresh_pending = pending;
    }

    pub fn is_deck_refresh_pending(&self) -> bool {
        self.deck_refresh_pending
    }

    pub fn perform_deck_refresh(&mut self, player_id: &str) {
        let player = if player_id == "player1" {
            &mut self.player1
        } else {
            &mut self.player2
        };

        let waitroom_cards: Vec<i16> = player.waitroom.cards.iter().copied().collect();
        player.waitroom.cards.clear();
        for card_id in waitroom_cards {
            player.main_deck.cards.push(card_id);
        }

        player.main_deck.shuffle();
        self.deck_refresh_pending = false;
    }

    pub fn set_live_being_performed(&mut self, performed: bool) {
        self.live_being_performed = performed;
    }

    pub fn is_live_being_performed(&self) -> bool {
        self.live_being_performed
    }

    pub fn set_game_ended(&mut self, ended: bool) {
        self.game_ended = ended;
    }

    pub fn is_game_ended(&self) -> bool {
        self.game_ended
    }

    pub fn set_draw_state(&mut self, draw: bool) {
        self.draw_state = draw;
    }

    pub fn is_draw_state(&self) -> bool {
        self.draw_state
    }

    pub fn check_success_zone_draw_condition(&self, player_id: &str) -> bool {
        let player = if player_id == self.player1.id {
            &self.player1
        } else if player_id == self.player2.id {
            &self.player2
        } else {
            return false;
        };

        let success_count = player.success_live_card_zone.cards.len();
        success_count >= 3
    }

    pub fn add_revealed_card(&mut self, card_id: i16) {
        self.revealed_cards.push(card_id);
    }

    pub fn remove_revealed_card(&mut self, card_id: i16) {
        self.revealed_cards.retain(|&id| id != card_id);
    }

    pub fn is_card_revealed(&self, card_id: i16) -> bool {
        self.revealed_cards.contains(&card_id)
    }

    pub fn clear_revealed_cards(&mut self) {
        self.revealed_cards.clear();
        self.player1_cheer_revealed_cards.clear();
        self.player2_cheer_revealed_cards.clear();
    }

    pub fn add_gained_ability(&mut self, card_id: i16, ability_type: String) {
        self.gained_abilities
            .entry(card_id)
            .or_insert_with(Vec::new)
            .push(ability_type);
    }

    pub fn remove_gained_abilities(&mut self, card_id: i16) {
        self.gained_abilities.remove(&card_id);
    }

    pub fn has_gained_ability(&self, card_id: i16, ability_type: &str) -> bool {
        self.gained_abilities
            .get(&card_id)
            .map_or(false, |a| a.iter().any(|x| x == ability_type))
    }

    pub fn clear_gained_abilities_for_card(&mut self, card_id: i16) {
        self.gained_abilities.remove(&card_id);
    }
}
