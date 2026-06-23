impl GameState {
    /// Re-evaluate all constant (常時) abilities on all stage members.
    /// Handles gain_resource(blade, heart), modify_score, modify_cost.
    /// Clears old constant-derived values and re-applies those whose conditions pass.
    pub fn recalculate_constants(&mut self) {
        let entries = self.collect_constant_stage_effects();
        self.mods.constant_score_sources.clear();

        let mut exp_blade: std::collections::HashMap<i16, i32> = std::collections::HashMap::new();
        let mut exp_cost: std::collections::HashMap<i16, i32> = std::collections::HashMap::new();
        let mut exp_score: std::collections::HashMap<i16, i32> = std::collections::HashMap::new();
        let mut exp_heart: std::collections::HashMap<i16, std::collections::HashMap<String, i32>> =
            std::collections::HashMap::new();
        let mut exp_prohibition: Vec<String> = Vec::new();
        let mut exp_global_need_heart: Vec<(i16, String, i32)> = Vec::new();
        let mut p1_constant_score_bonus: i32 = 0;
        let mut p2_constant_score_bonus: i32 = 0;
        let mut jyouji_statuses: Vec<crate::types::ConstantAbilityStatus> = Vec::new();

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
            // Capture card info and owner for jyouji status tracking before the ctx borrow
            let status_card_name = self
                .card_database
                .get_card(*card_id)
                .map(|c| c.name.clone())
                .unwrap_or_default();
            let status_owner = if self.player1.stage.stage.contains(card_id) {
                self.player1.id.clone()
            } else {
                self.player2.id.clone()
            };

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
                        .is_none_or(|c| ctx.evaluate_condition(c));

                    if cond_met {
                        // Record jyouji status for this card
                        jyouji_statuses.push(crate::types::ConstantAbilityStatus {
                            card_id: *card_id,
                            card_name: status_card_name.clone(),
                            owner: status_owner.clone(),
                            zone: "stage".to_string(),
                            ability_text: effect.text.clone(),
                            all_conditions_met: pos_ok && cond_met,
                            conditions: vec![crate::types::ConditionResult {
                                text: "条件".to_string(),
                                passed: cond_met,
                            }],
                        });
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
                                            let mut filter = effect.filter_subset();
                                            if filter.exclude_self == Some(-1) {
                                                filter.exclude_self = Some(*card_id);
                                            }
                                            let per_count =
                                                crate::ability::util::resolve_per_unit_count(
                                                    true,
                                                    Some(zone),
                                                    player,
                                                    &self.card_database,
                                                    &filter,
                                                    &[],
                                                    effect.state.as_deref(),
                                                    &self.mods.orientation_modifiers,
                                                );
                                            let base = if effect.max.unwrap_or(false) {
                                                1
                                            } else {
                                                effect
                                                    .resource_icon_count
                                                    .unwrap_or(effect.count.unwrap_or(1))
                                            };
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
                                            let mut filter = effect.filter_subset();
                                            if filter.exclude_self == Some(-1) {
                                                filter.exclude_self = Some(*card_id);
                                            }
                                            let per_count =
                                                crate::ability::util::resolve_per_unit_count(
                                                    true,
                                                    Some(zone),
                                                    player,
                                                    &self.card_database,
                                                    &filter,
                                                    &effect.heart_colors,
                                                    effect.state.as_deref(),
                                                    &self.mods.orientation_modifiers,
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
                                        if effect.heart_type.as_deref() == Some("all") {
                                            *exp_heart
                                                .entry(*card_id)
                                                .or_default()
                                                .entry("heart00".to_string())
                                                .or_insert(0) += n;
                                        } else {
                                            for hc in &effect.heart_colors {
                                                *exp_heart
                                                    .entry(*card_id)
                                                    .or_default()
                                                    .entry(hc.clone())
                                                    .or_insert(0) += n;
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            Some(crate::ability::enums::ActionType::ModifyScore) => {
                                let sv = effect.value.unwrap_or(0) as i32;
                                *exp_score.entry(*card_id).or_insert(0) += sv;
                                if sv != 0 {
                                    self.mods.constant_score_sources.push((
                                        *card_id,
                                        effect.text.clone(),
                                        sv,
                                    ));
                                }
                            }
                            Some(crate::ability::enums::ActionType::ModifyCost) => {
                                *exp_cost.entry(*card_id).or_insert(0) +=
                                    effect.value.unwrap_or(0) as i32;
                            }
                            Some(crate::ability::enums::ActionType::Restriction) => {
                                if let Some(ref rt) = effect.restriction_type {
                                    exp_prohibition.push(format!("const_restriction:{}:", rt));
                                    let tgt = effect.target.as_deref().unwrap_or("self");
                                    if rt == "cannot_activate" || rt == "cannot_activate_by_effect"
                                    {
                                        let resolved = self.resolve_target_player(tgt).id.clone();
                                        if !self.cannot_activate_members.contains(&resolved) {
                                            self.cannot_activate_members.push(resolved);
                                        }
                                    }
                                    if rt == "cannot_live" {
                                        let resolved = self.resolve_target_player(tgt).id.clone();
                                        if !self.cannot_live_players.contains(&resolved) {
                                            self.cannot_live_players.push(resolved);
                                        }
                                    }
                                }
                            }
                            Some(crate::ability::enums::ActionType::GainAbility) => {
                                if effect.ability_gain.as_deref() == Some("{{icon_all.png|ハート}}")
                                    || effect
                                        .ability_gain
                                        .as_deref()
                                        .is_some_and(|t| t.contains("ALL"))
                                {
                                    // All-heart: store as single "all" entry (HeartColor::All)
                                    *exp_heart
                                        .entry(*card_id)
                                        .or_default()
                                        .entry("all".to_string())
                                        .or_insert(0) += 1i32;
                                } else if let Some(gain_text) = effect.ability_gain.as_deref() {
                                    // Determine which player this card belongs to
                                    let belongs_to_p1 = self.player1.stage.stage.contains(card_id);
                                    let bonus_target = if belongs_to_p1 {
                                        &mut p1_constant_score_bonus
                                    } else {
                                        &mut p2_constant_score_bonus
                                    };

                                    // Record the gained ability for tracking
                                    self.add_gained_ability(*card_id, gain_text.to_string());

                                    // Use gained_effect if available (structured data from parser)
                                    if let Some(ref gained) = effect.gained_effect {
                                        let action = crate::ability::enums::ActionType::from_str(
                                            &gained.action,
                                        );
                                        if action
                                            == Some(crate::ability::enums::ActionType::ModifyScore)
                                        {
                                            let val = gained.value.unwrap_or(0) as i32;
                                            *bonus_target += val;
                                            if val != 0 {
                                                self.mods.constant_score_sources.push((
                                                    *card_id,
                                                    gain_text.to_string(),
                                                    val,
                                                ));
                                            }
                                        } else if action
                                            == Some(
                                                crate::ability::enums::ActionType::ConditionalAlternative,
                                            )
                                        {
                                            // Conditional gained effects (e.g. live_success score
                                            // based on revealed card count) can't be evaluated at
                                            // constant evaluation time.  Store them for later
                                            // evaluation during execute_live_victory_determination.
                                            self.delayed_gained_effects
                                                .push((*card_id, (**gained).clone()));
                                        }
                                    } else {
                                        // Fallback: parse value from text (legacy path)
                                        if let Some(val) =
                                            gain_text.split('+').nth(1).and_then(|s| {
                                                s.chars()
                                                    .take_while(|c| c.is_ascii_digit())
                                                    .collect::<String>()
                                                    .parse::<i32>()
                                                    .ok()
                                            })
                                        {
                                            *bonus_target += val;
                                            if val != 0 {
                                                self.mods.constant_score_sources.push((
                                                    *card_id,
                                                    gain_text.to_string(),
                                                    val,
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                            Some(crate::ability::enums::ActionType::GainAbilityFromSource) => {
                                let mut resolver = crate::ability::resolver::AbilityResolver::new(
                                    self.card_database.clone(),
                                    self.activating_card,
                                );
                                let _ = resolver.execute_gain_ability_from_source(self, effect);
                            }
                            Some(crate::ability::enums::ActionType::ModifyRequiredHeartsGlobal) => {
                                let target_name = effect.target_name();
                                let target_player = self.resolve_target_player(target_name);
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
                                        let sub_cond = sub
                                            .condition
                                            .as_ref()
                                            .is_none_or(|c| ctx.evaluate_condition(c));
                                        if !sub_cond {
                                            continue;
                                        }
                                        if let Some(
                                            crate::ability::enums::ActionType::GainResource,
                                        ) =
                                            crate::ability::enums::ActionType::from_str(&sub.action)
                                        {
                                            match sub.resource.as_deref().unwrap_or("") {
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
                                            }
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
        self.constant_ability_statuses = jyouji_statuses;

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

        // Per-player global score bonus (from GainAbility modify_score)
        self.mods.p1_constant_total_score_bonus = p1_constant_score_bonus;
        self.mods.p2_constant_total_score_bonus = p2_constant_score_bonus;

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
                    .is_none_or(|c| ctx.evaluate_condition(c));
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
                        let mut filter = effect.filter_subset();
                        if filter.exclude_self == Some(-1) {
                            filter.exclude_self = Some(cid);
                        }
                        let per_count = crate::ability::util::resolve_per_unit_count(
                            true,
                            Some(zone),
                            player,
                            &self.card_database,
                            &filter,
                            &[],
                            effect.state.as_deref(),
                            &self.mods.orientation_modifiers,
                        );
                        let base = if effect.max.unwrap_or(false) {
                            1
                        } else {
                            effect
                                .resource_icon_count
                                .unwrap_or(effect.count.unwrap_or(1))
                        };
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
                    .is_none_or(|c| ctx.evaluate_condition(c));
                if cond_met {
                    let mut value = effect.value.unwrap_or(0) as i32;

                    // Handle per_unit cost reduction (e.g. "1 per other card in hand")
                    if effect.per_unit.unwrap_or(false) {
                        let player = self.resolve_target_player(effect.target_name());
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
                            log::debug!(
                                "[COST_MOD_PER_UNIT_DEBUG] stage_ids={:?} group_name={:?}",
                                stage_ids,
                                group_name
                            );
                            let matches = stage_ids
                                .iter()
                                .filter(|&&id| {
                                    crate::ability::util::card_matches_group_str(
                                        card_db, id, group_name,
                                    )
                                })
                                .count();
                            log::debug!("[COST_MOD_PER_UNIT_DEBUG] group_matches={}", matches);
                            matches as u32
                        } else {
                            let cards: Vec<i16> =
                                crate::ability::util::zone_cards(player, count_zone).to_vec();
                            cards.len() as u32
                        };
                        log::debug!(
                            "[COST_MOD_PER_UNIT] cid={} count_zone={} count={}",
                            cid,
                            count_zone,
                            count
                        );
                        let per_unit_count = effect.per_unit_count.unwrap_or(1);
                        let exclude_self = effect.exclude_self.unwrap_or(false);
                        let effective = if exclude_self {
                            count.saturating_sub(1)
                        } else {
                            count
                        };
                        value = ((effective / per_unit_count) * (value as u32)) as i32;
                        log::debug!("[COST_MOD] cid={} zone={} count={} eff={} per_unit_cnt={} val={} exclude={}",
                            cid, count_zone, count, effective, per_unit_count, value, exclude_self);
                    }
                    log::debug!(
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

    pub fn record_card_appearance(&mut self, card_id: i16, source: &str) {
        self.cards_appeared_this_turn.insert(card_id);
        if !source.is_empty() {
            self.card_appearance_source
                .insert(card_id, source.to_string());
        }
    }

    pub fn has_card_appeared_this_turn(&self, card_id: i16) -> bool {
        self.cards_appeared_this_turn.contains(&card_id)
    }

    pub fn get_card_appearance_source(&self, card_id: i16) -> Option<&str> {
        self.card_appearance_source
            .get(&card_id)
            .map(|s| s.as_str())
    }

    pub fn clear_card_appearance_tracking(&mut self) {
        self.cards_appeared_this_turn.clear();
        self.card_appearance_source.clear();
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
        self.baton_touch_arriving_card_id = None;
    }

    pub fn record_card_movement(&mut self, card_id: i16) {
        self.cards_moved_this_turn.insert(card_id);
    }

    /// Push a MovementEvent recording the movement of a card, tracking what caused it.
    /// Also syncs `recently_moved_cards`/`recently_moved_from_zone` for backward compat.
    pub fn push_movement_event(
        &mut self,
        moved_card_id: i16,
        source_zone: &str,
        dest_zone: &str,
        cause_card_id: Option<i16>,
        cause_player_id: &str,
        effect_only: bool,
    ) {
        self.movement_event_counter += 1;
        let event = crate::types::MovementEvent {
            moved_card_id,
            source_zone: source_zone.to_string(),
            dest_zone: dest_zone.to_string(),
            cause_card_id,
            cause_player_id: cause_player_id.to_string(),
            effect_only,
            timestamp: self.movement_event_counter,
        };
        // Track in the current batch + sync old fields (append, don't rebuild
        // from batch_movements — it accumulates across ability batches).
        self.batch_movements.push(event.clone());
        let cards = self.recently_moved_cards.get_or_insert_with(Vec::new);
        cards.push(moved_card_id);
        self.recently_moved_from_zone = Some(source_zone.to_string());
        // Track turn-level area movement (stage-area-to-stage-area)
        let is_area_move = source_zone == "stage" && dest_zone == "stage";
        if is_area_move {
            self.turn_area_movements.push(event);
            self.position_change_occurred_this_turn = true;
        }
        // Track in cards_moved_this_turn for fast O(1) lookups
        self.cards_moved_this_turn.insert(moved_card_id);
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

    pub fn remove_from_source_hands(&mut self, card_ids: &[i16]) {
        let mut seen = std::collections::HashSet::new();
        for &cid in card_ids {
            if !seen.insert(cid) {
                continue;
            }
            // Only remove from hand if the card was from a cost reveal
            // (tracked in revealed_cost_cards). Non-cost reveals (deck peek, etc.)
            // should NOT remove from hand.
            if !self.revealed_cost_cards.contains(&cid) {
                continue;
            }
            for player in [&mut self.player1, &mut self.player2] {
                if let Some(pos) = player.hand.cards.iter().position(|&c| c == cid) {
                    player.hand.remove_card(pos);
                    break;
                }
            }
        }
    }
    pub fn add_gained_ability(&mut self, card_id: i16, ability_type: String) {
        self.gained_abilities
            .entry(card_id)
            .or_default()
            .push(ability_type);
    }

    pub fn remove_gained_abilities(&mut self, card_id: i16) {
        self.gained_abilities.remove(&card_id);
    }

    pub fn has_gained_ability(&self, card_id: i16, ability_type: &str) -> bool {
        self.gained_abilities
            .get(&card_id)
            .is_some_and(|a| a.iter().any(|x| x == ability_type))
    }

    pub fn clear_gained_abilities_for_card(&mut self, card_id: i16) {
        self.gained_abilities.remove(&card_id);
    }

    /// Evaluate all constant (常時) `modify_required_hearts` abilities on cards
    /// in the success_live_card_zone. Used by tests and by the live success flow
    /// to apply ongoing heart requirement reductions (e.g. PL!-bp6-022-L).
    /// Created as a focused scan because `recalculate_constants` only evaluates
    /// stage-based constant abilities. This function:
    ///   1. Scans all cards in the current player's success_live_card_zone
    ///   2. For each card with a constant `modify_required_hearts` ability whose
    ///      location_condition passes (self is in success_live_card_zone), executes
    ///      the effect through the AbilityResolver to handle original_value filter,
    ///      group_names filter, per_unit counting, etc.
    pub fn evaluate_success_zone_heart_reductions(&mut self) {
        use crate::ability::condition::ConditionContext;
        use crate::ability::enums::ActionType;
        use crate::ability::resolver::AbilityResolver;

        // Clear all need_heart_modifiers first, then re-evaluate from current zone state.
        // This ensures as_long_as semantics: when a card leaves the success zone, its
        // constant modifier is not re-applied on the next evaluation.
        self.mods.need_heart_modifiers.clear();

        // Track non-stackable effects locally so they are reset each evaluation
        let mut local_non_stackable: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        // Process both players' success_live_card_zones
        for player_idx in 0..2 {
            let zone_cards = if player_idx == 0 {
                self.player1.success_live_card_zone.cards.clone()
            } else {
                self.player2.success_live_card_zone.cards.clone()
            };

            for cid in &zone_cards {
                let card = match self.card_database.get_card(*cid) {
                    Some(c) => c.clone(),
                    None => continue,
                };
                for ability in &card.abilities {
                    let is_constant = ability
                        .triggers
                        .as_ref()
                        .is_some_and(|t| t.contains(crate::triggers::CONSTANT));
                    if !is_constant {
                        continue;
                    }
                    let effect = match ability.effect.as_ref() {
                        Some(e) => e.clone(),
                        None => continue,
                    };
                    if ActionType::from_str(&effect.action)
                        != Some(ActionType::ModifyRequiredHearts)
                    {
                        continue;
                    }
                    // Evaluate location condition
                    let prev_activating = self.activating_card;
                    self.activating_card = Some(*cid);
                    let ctx = ConditionContext::new(self);
                    let cond_met = effect
                        .condition
                        .as_ref()
                        .is_none_or(|c| ctx.evaluate_condition(c));
                    if !cond_met {
                        self.activating_card = prev_activating;
                        continue;
                    }
                    // Non-stackable check: skip if this effect was already applied
                    // in this evaluation pass. Uses a local set so the check is
                    // reset each time the function is called.
                    if effect.non_stackable.unwrap_or(false) {
                        let effect_key = format!("{}:{}", effect.action, effect.text);
                        if local_non_stackable.contains(&effect_key) {
                            self.activating_card = prev_activating;
                            continue;
                        }
                        local_non_stackable.insert(effect_key);
                    }

                    // Execute the effect through the resolver
                    let mut resolver = AbilityResolver::new(self.card_database.clone(), Some(*cid));
                    let _ = resolver.execute_modify_required_hearts(
                        self,
                        effect.operation.as_deref().unwrap_or("decrease"),
                        effect.value_or_count(0),
                        &effect.heart_colors,
                        effect.target_name(),
                        effect.per_unit.unwrap_or(false),
                        effect.per_unit_count.unwrap_or(1),
                        effect.group_name(),
                        effect.timing_condition.as_deref(),
                        effect.location.as_deref(),
                        effect.original_value,
                        effect.original_count,
                        effect.original_operator.as_deref(),
                        effect.exclude_self.unwrap_or(false),
                        effect.self_target.unwrap_or(false),
                    );
                    self.activating_card = prev_activating;
                }
            }
        }
    }
}
