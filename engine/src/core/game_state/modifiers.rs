extern "C" {
    // Outputs to both debug console (svcOutputDebugString) AND top screen (consoleSelect+printf)
    fn _3ds_tdbg(msg: *const u8);
}

#[cfg(feature = "3ds")]
macro_rules! tdbg {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        let s = format!("{}\0", msg);
        unsafe { _3ds_tdbg(s.as_ptr()); }
    }};
}
#[cfg(not(feature = "3ds"))]
macro_rules! tdbg {
    ($($arg:tt)*) => {};
}
impl GameState {
    /// Re-evaluate all constant (常時) abilities on all stage members.
    /// Handles gain_resource(blade, heart), modify_score, modify_cost.
    /// Clears old constant-derived values and re-applies those whose conditions pass.
    #[inline(never)]
    pub fn recalculate_constants(&mut self) {
        tdbg!("RC:0 ENTERED");
        // HANG WORKAROUND (3DS ARMv6K): AtomicBool::load uses 8-bit atomics
        // that may deadlock via Mutex fallback. Use a plain bool on GameState
        // OR skip the debug check entirely on 3DS.
        #[cfg(not(feature = "3ds"))]
        if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
            log::debug!("[SZ_DEBUG] recalculate_constants ENTERED");
        }
        tdbg!("RC:1 ATOMIC_LOAD_OK");
        let entries = self.collect_constant_stage_effect_ids();
        tdbg!("RC:2 COLLECT_EFFECTS_OK len={}", entries.len());
        self.mods.constant_score_sources.clear();

        // Clone the Arc once (cheap: atomic increment) — all effect lookups
        // go through this local reference instead of self.card_database,
        // avoiding 152B × N AbilityEffect clones per recalculation.
        let card_db = self.card_database.clone();

        // Reuse pre-allocated scratch buffers to avoid allocation storm
        let mut exp_blade = core::mem::take(&mut self.scratch_exp_blade);
        exp_blade.clear();
        let mut exp_cost = core::mem::take(&mut self.scratch_exp_cost);
        exp_cost.clear();
        let mut exp_score = core::mem::take(&mut self.scratch_exp_score);
        exp_score.clear();
        let mut exp_heart = core::mem::take(&mut self.scratch_exp_heart);
        exp_heart.clear();
        let mut exp_prohibition: Vec<String> = Vec::new();
        self.constant_cannot_activate_members.clear();
        let mut exp_global_need_heart: Vec<(i16, String, i32)> = Vec::new();
        let mut p1_constant_score_bonus: i32 = 0;
        let mut p2_constant_score_bonus: i32 = 0;
        let mut jyouji_statuses: Vec<crate::types::ConstantAbilityStatus> = Vec::new();
        tdbg!("RC:3 VEC_HASHMAP_INIT_OK");

        let mut entry_positions = core::mem::take(&mut self.scratch_entry_positions);
        entry_positions.clear();
        for (pos, &cid) in self.player1.stage.stage.iter().enumerate() {
            if cid != -1 {
                entry_positions.insert(cid, Some(pos));
            }
        }
        for (pos, &cid) in self.player2.stage.stage.iter().enumerate() {
            if cid != -1 {
                entry_positions.entry(cid).or_insert(Some(pos));
            }
        }
        tdbg!("RC:4 ENTRY_POSITIONS_DONE count={}", entry_positions.len());

        for &(card_id, ability_idx) in &entries {
            tdbg!("RC:5_LOOP card_id={}", card_id);
            // Re-lookup effect through the local Arc clone — avoids 152B clone.
            // The reference borrows card_db (not self), so there's no borrow
            // conflict with &mut self operations later in this iteration.
            let ability = match GameState::resolve_constant_ability(&card_db, card_id, ability_idx)
            {
                Some(a) => a,
                None => continue,
            };
            let Some(ref effect) = ability.effect else {
                continue;
            };
            // Set activating_card so condition evaluators (e.g. exclude_self in
            // location_condition) know which card is "self" for this entry.
            let prev_activating = self.activating_card;
            self.activating_card = Some(card_id);
            // Capture card info and owner for jyouji status tracking before the ctx borrow
            let status_card_name = card_db
                .get_card(card_id)
                .map(|c| c.name.to_string())
                .unwrap_or_default();
            let status_owner = if self.player1.stage.stage.contains(&card_id) {
                self.player1.id.clone()
            } else {
                self.player2.id.clone()
            };

            {
                let self_player = if self.player1.stage.stage.contains(&card_id) {
                    Some(&self.player1)
                } else {
                    Some(&self.player2)
                };
                let mut ctx =
                    crate::ability::condition::ConditionContext::new_with_self(self, self_player);
                // Constant abilities should register their effects regardless
                // of the current phase — the phase gate only matters at trigger
                // evaluation time, not during constant registration.
                ctx.skip_phase_gate = true;

                // Check effect-level position requirement
                let pos_ok = if let Some(ref pos) = effect.position_any() {
                    let pos_str = pos.get_position();
                    let card_pos = entry_positions.get(&card_id).copied().flatten();
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
                            card_id: card_id,
                            card_name: status_card_name.clone(),
                            owner: status_owner.clone(),
                            zone: "stage".to_string(),
                            ability_text: effect.text.to_string(),
                            all_conditions_met: pos_ok && cond_met,
                            conditions: vec![crate::types::ConditionResult {
                                text: "条件".to_string(),
                                passed: cond_met,
                            }],
                        });
                        match effect.action {
                            crate::ability::enums::ActionType::GainResource => {
                                match effect.resource_any().as_deref().unwrap_or("") {
                                    "blade" | "ブレード" => {
                                        let n = if effect.per_unit_any().unwrap_or(false) {
                                            let player =
                                                if self.player1.stage.stage.contains(&card_id) {
                                                    &self.player1
                                                } else {
                                                    &self.player2
                                                };
                                            let loc_b = effect.location_any();
                                            let per_b = effect.per_unit_type_any();
                                            let zone =
                                                loc_b.or(per_b).unwrap_or(Zone::Hand.to_str());
                                            let mut filter = effect.filter_subset();
                                            if filter.exclude_self == Some(-1) {
                                                filter.exclude_self = Some(card_id);
                                            }
                                            let per_count =
                                                crate::ability::util::resolve_per_unit_count(
                                                    true,
                                                    Some(zone),
                                                    player,
                                                    &self.card_database,
                                                    &filter,
                                                    &[],
                                                    effect.state_any().as_deref(),
                                                    &self.mods.orientation_modifiers,
                                                );
                                            let base = if effect.max.unwrap_or(false) {
                                                1
                                            } else {
                                                effect
                                                    .resource_icon_count_any()
                                                    .unwrap_or(effect.count_any().unwrap_or(1))
                                            };
                                            let mut units = per_count as i32
                                                / effect.per_unit_count_any().unwrap_or(1).max(1)
                                                    as i32;
                                            if effect.max.unwrap_or(false) {
                                                if let Some(cap) = effect.count_any() {
                                                    units = units.min(cap as i32);
                                                }
                                            }
                                            units * base as i32
                                        } else {
                                            effect
                                                .resource_icon_count_any()
                                                .unwrap_or(effect.count_any().unwrap_or(1))
                                                as i32
                                        };
                                        *exp_blade.entry(card_id).or_insert(0) += n;
                                    }
                                    "heart" | "ハート" => {
                                        let n = if effect.per_unit_any().unwrap_or(false) {
                                            let player =
                                                if self.player1.stage.stage.contains(&card_id) {
                                                    &self.player1
                                                } else {
                                                    &self.player2
                                                };
                                            let loc_b = effect.location_any();
                                            let per_b = effect.per_unit_type_any();
                                            let zone =
                                                loc_b.or(per_b).unwrap_or(Zone::Hand.to_str());
                                            let mut filter = effect.filter_subset();
                                            if filter.exclude_self == Some(-1) {
                                                filter.exclude_self = Some(card_id);
                                            }
                                            let per_count =
                                                crate::ability::util::resolve_per_unit_count(
                                                    true,
                                                    Some(zone),
                                                    player,
                                                    &self.card_database,
                                                    &filter,
                                                    effect.heart_colors_any(),
                                                    effect.state_any().as_deref(),
                                                    &self.mods.orientation_modifiers,
                                                );
                                            let mut units = per_count as i32
                                                / effect.per_unit_count_any().unwrap_or(1).max(1)
                                                    as i32;
                                            if effect.max.unwrap_or(false) {
                                                if let Some(cap) = effect.count_any() {
                                                    units = units.min(cap as i32);
                                                }
                                            }
                                            units
                                        } else {
                                            effect.count_any().unwrap_or(1) as i32
                                        };
                                        if effect.heart_type_any().as_deref() == Some("all") {
                                            *exp_heart
                                                .entry(card_id)
                                                .or_default()
                                                .entry("heart00".to_string())
                                                .or_insert(0) += n;
                                        } else {
                                            for hc in effect.heart_colors_any() {
                                                *exp_heart
                                                    .entry(card_id)
                                                    .or_default()
                                                    .entry(hc.clone())
                                                    .or_insert(0) += n;
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            crate::ability::enums::ActionType::ModifyScore => {
                                let sv = effect.value_any().unwrap_or(0) as i32;
                                *exp_score.entry(card_id).or_insert(0) += sv;
                                if sv != 0 {
                                    self.mods.constant_score_sources.push((
                                        card_id,
                                        effect.text.to_string(),
                                        sv,
                                    ));
                                }
                            }
                            crate::ability::enums::ActionType::ModifyCost => {
                                *exp_cost.entry(card_id).or_insert(0) +=
                                    effect.value_any().unwrap_or(0) as i32;
                            }
                            crate::ability::enums::ActionType::Restriction => {
                                if let Some(rt) = effect.restriction_type_any() {
                                    let card_name = self
                                        .card_database
                                        .get_card(card_id)
                                        .map(|c| c.name.to_string())
                                        .unwrap_or_default();
                                    // Do NOT push `cannot_activate` to prohibition_effects:
                                    // the auto-activation blocking is already handled by
                                    // constant_cannot_activate_members in phases.rs. Pushing
                                    // to prohibition_effects would incorrectly block manual
                                    // ability activation via is_action_prohibited.
                                    if rt != "cannot_activate" {
                                        exp_prohibition.push(format!(
                                            "const_restriction:{},card={},cardname={}:",
                                            rt, card_id, card_name
                                        ));
                                    }
                                    let tgt_opt = effect.target_any();
                                    let tgt = tgt_opt.unwrap_or("self");
                                    if rt == "cannot_activate_by_effect" {
                                        let resolved = self.resolve_target_player(tgt).id.clone();
                                        if !self.cannot_activate_members.contains(&resolved) {
                                            self.cannot_activate_members.push(resolved);
                                        }
                                    } else if rt == "cannot_activate" {
                                        if tgt == "self" {
                                            // Per-card: only block this specific member
                                            self.constant_cannot_activate_members
                                                .push(card_id.to_string());
                                        } else {
                                            // Player-level: block all members of the target player
                                            let resolved =
                                                self.resolve_target_player(tgt).id.clone();
                                            self.constant_cannot_activate_members.push(resolved);
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
                            // ── gain_ability ──────────────────────────────────
                            // This produces a persistent effect that shows on the
                            // card via bonus_triggers texticon (in game_state_to_display).
                            // The trigger type (常時 → jyouji.png, ライブ成功時 → live_success.png)
                            // is read from ability_gain_trigger and rendered on the card.
                            //
                            // Texticon display for this action:
                            //   - All-heart case: bonus_heart "all" → icon_all.png badge
                            //   - ModifyScore via gained_effect: bonus_score → icon_score.png badge
                            //     PLUS bonus_triggers → trigger texticon (e.g. jyouji.png)
                            //   - ConditionalAlternative: deferred, no immediate texticon
                            //   - Legacy text parse: bonus_score → icon_score.png badge
                            //     PLUS bonus_triggers → trigger texticon
                            crate::ability::enums::ActionType::GainAbility => {
                                if effect.ability_gain_any().as_deref()
                                    == Some("{{icon_all.png|ハート}}")
                                    || effect
                                        .ability_gain_any()
                                        .as_deref()
                                        .is_some_and(|t| t.contains("ALL"))
                                    || effect
                                        .ability_gain_any()
                                        .as_deref()
                                        .is_some_and(|t| t.contains("【ハート】"))
                                {
                                    // All-heart: store as single "all" entry (HeartColor::All)
                                    *exp_heart
                                        .entry(card_id)
                                        .or_default()
                                        .entry("all".to_string())
                                        .or_insert(0) += 1i32;
                                } else if let Some(gain_text) = effect.ability_gain_any().as_deref()
                                {
                                    // Determine which player this card belongs to
                                    let belongs_to_p1 = self.player1.stage.stage.contains(&card_id);
                                    let bonus_target = if belongs_to_p1 {
                                        &mut p1_constant_score_bonus
                                    } else {
                                        &mut p2_constant_score_bonus
                                    };

                                    // Record the gained ability for tracking
                                    self.add_gained_ability(card_id, gain_text.to_string());

                                    // Use gained_effect if available (structured data from parser)
                                    if let Some(ref gained) = effect.gained_effect_any() {
                                        let action = gained.action;
                                        if action
                                            == crate::ability::enums::ActionType::ModifyScore
                                        {
                                            let val = gained.value_any().unwrap_or(0) as i32;
                                            *bonus_target += val;
                                            if val != 0 {
                                                self.mods.constant_score_sources.push((
                                                    card_id,
                                                    gain_text.to_string(),
                                                    val,
                                                ));
                                            }
                                        } else if action
                                            == crate::ability::enums::ActionType::ConditionalAlternative
                                        {
                                            // Conditional gained effects (e.g. live_success score
                                            // based on revealed card count) can't be evaluated at
                                            // constant evaluation time.  Store them for later
                                            // evaluation during execute_live_victory_determination.
                                            self.delayed_gained_effects
                                                .push((card_id, *(*gained).clone()));
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
                                                    card_id,
                                                    gain_text.to_string(),
                                                    val,
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                            crate::ability::enums::ActionType::GainAbilityFromSource => {
                                let mut resolver = crate::ability::resolver::AbilityResolver::new(
                                    self.card_database.clone(),
                                    self.activating_card,
                                );
                                let _ = resolver.execute_gain_ability_from_source(self, effect);
                            }
                            crate::ability::enums::ActionType::ModifyRequiredHeartsGlobal => {
                                let target_name = effect.target_name();
                                let target_player = self.resolve_target_player(target_name);
                                let target_cards: Vec<i16> =
                                    target_player.live_card_zone.cards.to_vec();
                                let value = effect.value_or_count(1) as i32;
                                let op_str = effect.operation_any().unwrap_or("increase");
                                let op = op_str;
                                let delta = match op {
                                    "increase" => value,
                                    "decrease" => -value,
                                    _ => value,
                                };
                                let colors: Vec<String> = if effect.heart_colors_any().is_empty() {
                                    vec!["heart00".to_string()]
                                } else {
                                    effect.heart_colors_any().to_vec()
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
                            crate::ability::enums::ActionType::Sequential => {
                                if let Some(ref actions) = effect.compound.actions {
                                    for sub in actions {
                                        let sub_cond = sub
                                            .condition
                                            .as_ref()
                                            .is_none_or(|c| ctx.evaluate_condition(c));
                                        if !sub_cond {
                                            continue;
                                        }
                                        if sub.action
                                            == crate::ability::enums::ActionType::GainResource
                                        {
                                            match sub.resource_any().as_deref().unwrap_or("") {
                                                "blade" | "ブレード" => {
                                                    let n = sub
                                                        .resource_icon_count_any()
                                                        .unwrap_or(sub.count.unwrap_or(1))
                                                        as i32;
                                                    *exp_blade.entry(card_id).or_insert(0) += n;
                                                }
                                                "heart" | "ハート" => {
                                                    let n = sub.count.unwrap_or(1) as i32;
                                                    for hc in sub.heart_colors_any() {
                                                        *exp_heart
                                                            .entry(card_id)
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
        // Recycle entry_positions allocation into scratch buffer
        self.scratch_entry_positions = entry_positions;
        let _jyouji_len = jyouji_statuses.len();
        self.constant_ability_statuses = jyouji_statuses.into();
        tdbg!("RC:6 MAIN_LOOP_DONE jyouji={}", _jyouji_len);

        // Blade
        tdbg!("RC:7 BLADE");
        let old_blade = core::mem::take(&mut self.mods.constant_blade_bonuses);
        for (cid, val) in &old_blade {
            self.mods.remove_blade_modifier(*cid, *val);
        }
        for (&cid, &val) in &exp_blade {
            self.mods.add_blade_modifier(cid, val);
        }
        self.mods.constant_blade_bonuses = exp_blade;
        self.scratch_exp_blade = old_blade;

        // Cost
        tdbg!("RC:8 COST");
        let old_cost = core::mem::take(&mut self.mods.constant_cost_bonuses);
        for (cid, val) in &old_cost {
            self.mods.remove_cost_modifier(*cid, *val);
        }
        for (&cid, &val) in &exp_cost {
            self.mods.add_cost_modifier(cid, val);
        }
        self.mods.constant_cost_bonuses = exp_cost;
        self.scratch_exp_cost = old_cost;

        // Score
        tdbg!("RC:9 SCORE");
        let old_score = core::mem::take(&mut self.mods.constant_score_bonuses);
        for (cid, val) in &old_score {
            self.mods.remove_score_modifier(*cid, *val);
        }
        for (&cid, &val) in &exp_score {
            self.mods.add_score_modifier(cid, val);
        }
        self.mods.constant_score_bonuses = exp_score;
        self.scratch_exp_score = old_score;

        // Per-player global score bonus (from GainAbility modify_score)
        self.mods.p1_constant_total_score_bonus = p1_constant_score_bonus;
        self.mods.p2_constant_total_score_bonus = p2_constant_score_bonus;

        // Heart — clear old constant heart modifiers first, then re-apply new ones.
        tdbg!("RC:10 HEART");
        // Must drain the OLD map so bonuses from cards that left the stage are removed.
        {
            let old_heart = core::mem::take(&mut self.mods.constant_heart_bonuses);
            for (cid, cols) in &old_heart {
                for (color_str, &delta) in cols {
                    let hc = crate::card::parse_heart_color(color_str);
                    self.mods.remove_heart_modifier(*cid, hc, delta);
                }
            }
            self.scratch_exp_heart = old_heart;
        }
        for (cid, cols) in &exp_heart {
            for (color_str, delta) in cols {
                let hc = crate::card::parse_heart_color(color_str);
                self.mods.add_heart_modifier(*cid, hc, *delta);
            }
        }
        self.mods.constant_heart_bonuses = exp_heart;

        tdbg!("RC:11 PROHIBITION");
        // Apply restriction effects from constant abilities.
        // Use "const_restriction:" prefix to distinguish from debut/live ability restrictions
        // so we can safely clear and re-add constant restrictions on each recalculate call.
        self.prohibition_effects
            .retain(|p| !p.starts_with("const_restriction:"));
        for p in &exp_prohibition {
            self.prohibition_effects.push(p.clone());
        }

        tdbg!("RC:12 GLOBAL_NEED_HEART");
        // Clear old constant global need_heart modifiers, then re-apply new ones.
        let old_global_nh = core::mem::take(&mut self.mods.constant_global_need_heart);
        for (card_id, color_str, delta) in &old_global_nh {
            let hc = crate::card::parse_heart_color(color_str);
            self.mods.add_need_heart_modifier(*card_id, hc, -*delta);
        }
        for (card_id, color_str, delta) in &exp_global_need_heart {
            let hc = crate::card::parse_heart_color(color_str);
            self.mods.add_need_heart_modifier(*card_id, hc, *delta);
        }
        self.mods.constant_global_need_heart = exp_global_need_heart;
        tdbg!("RC:12b GLOBAL_NEED_HEART_DONE");

        // Also recalculate cost modifiers from hand cards (hand-based cost reductions)
        // Pass pre-collected stage effects to avoid re-scanning the stage
        tdbg!("RC:13 COST_MODIFIERS_WITH_ENTRIES");
        let hand_ids = self.collect_constant_hand_effect_ids();
        self.recalculate_constant_cost_modifiers_with_ids(&card_db, &entries, &hand_ids);
        tdbg!("RC:13b COST_MODIFIERS_DONE");

        // Evaluate constant abilities from success live card zone (e.g. Love wing bell)
        tdbg!("RC:14 SUCCESS_ZONE");
        self.evaluate_success_zone_constant_modifiers();
        tdbg!("RC:14b SUCCESS_ZONE_DONE");
    }

    pub fn recalculate_constant_blade_modifiers(&mut self) {
        let card_db = self.card_database.clone();
        let all_ids = self.collect_constant_stage_effect_ids();
        // Filter to blade gain_resource abilities by looking up each effect
        let blade_ids: Vec<(i16, usize)> = all_ids
            .into_iter()
            .filter(|&(cid, idx)| {
                GameState::resolve_constant_ability(&card_db, cid, idx).map_or(false, |a| {
                    a.effect.as_ref().map_or(false, |effect| {
                        effect.action == crate::ability::enums::ActionType::GainResource
                            && matches!(
                                effect.resource_any().as_deref(),
                                Some("blade") | Some("ブレード")
                            )
                    })
                })
            })
            .collect();

        let mut expected: HashMap<i16, i32> = HashMap::default();
        {
            let ctx = crate::ability::condition::ConditionContext::new(self);
            for &(cid, ability_idx) in &blade_ids {
                let Some(blade_ability) =
                    GameState::resolve_constant_ability(&card_db, cid, ability_idx)
                else {
                    continue;
                };
                let Some(ref effect) = blade_ability.effect else {
                    continue;
                };
                let cond_met = effect
                    .condition
                    .as_ref()
                    .is_none_or(|c| ctx.evaluate_condition(c));
                if cond_met {
                    let count = if effect.per_unit_any().unwrap_or(false) {
                        let player = if self.player1.stage.stage.contains(&cid) {
                            &self.player1
                        } else {
                            &self.player2
                        };
                        let loc = effect.location_any();
                        let per_type = effect.per_unit_type_any();
                        let zone = loc
                            .or(per_type)
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
                            effect.state_any().as_deref(),
                            &self.mods.orientation_modifiers,
                        );
                        let base = if effect.max.unwrap_or(false) {
                            1
                        } else {
                            effect
                                .resource_icon_count_any()
                                .unwrap_or(effect.count_any().unwrap_or(1))
                        };
                        let mut units = per_count as i32
                            / effect.per_unit_count_any().unwrap_or(1).max(1) as i32;
                        if effect.max.unwrap_or(false) {
                            if let Some(cap) = effect.count_any() {
                                units = units.min(cap as i32);
                            }
                        }
                        units * base as i32
                    } else {
                        effect
                            .resource_icon_count_any()
                            .unwrap_or(effect.count_any().unwrap_or(1))
                            as i32
                    };
                    *expected.entry(cid).or_insert(0) += count;
                }
            }
        }

        let old_bonuses = core::mem::take(&mut self.mods.constant_blade_bonuses);
        for (cid, old) in &old_bonuses {
            self.mods.remove_blade_modifier(*cid, *old);
        }
        for (&cid, &new_val) in &expected {
            self.mods.add_blade_modifier(cid, new_val);
        }
        self.mods.constant_blade_bonuses = expected;
        self.recalculate_constant_cost_modifiers();
        if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
            log::debug!("[SZ_DEBUG] about to call evaluate_success_zone_constant_modifiers from recalculate_constants");
        }
        self.evaluate_success_zone_constant_modifiers();
    }

    pub fn recalculate_constant_cost_modifiers(&mut self) {
        let card_db = self.card_database.clone();
        let stage_ids = self.collect_constant_stage_effect_ids();
        let hand_ids = self.collect_constant_hand_effect_ids();
        self.recalculate_constant_cost_modifiers_with_ids(&card_db, &stage_ids, &hand_ids);
    }

    fn recalculate_constant_cost_modifiers_with_ids(
        &mut self,
        card_db: &crate::card::CardDatabase,
        stage_ids: &[(i16, usize)],
        hand_ids: &[(i16, usize)],
    ) {
        let mut expected: HashMap<i16, i32> = HashMap::default();
        {
            let ctx = crate::ability::condition::ConditionContext::new(self);
            // Chain stage and hand ability IDs, look up each effect, filter to ModifyCost
            let all_ids = stage_ids.iter().chain(hand_ids.iter());
            for &(cid, ability_idx) in all_ids {
                let Some(cost_ability) =
                    GameState::resolve_constant_ability(card_db, cid, ability_idx)
                else {
                    continue;
                };
                let Some(ref effect) = cost_ability.effect else {
                    continue;
                };
                if effect.action != crate::ability::enums::ActionType::ModifyCost {
                    continue;
                }
                let cond_met = effect
                    .condition
                    .as_ref()
                    .is_none_or(|c| ctx.evaluate_condition(c));
                if cond_met {
                    let mut value = effect.value_any().unwrap_or(0) as i32;

                    // Handle per_unit cost reduction (e.g. "1 per other card in hand")
                    if effect.per_unit_any().unwrap_or(false) {
                        let player = self.resolve_target_player(effect.target_name());
                        // per_unit_location overrides the counting zone when the
                        // parser determines the per-unit count targets a different
                        // zone than the effect's location (e.g. count stage members
                        // while the cost modifier itself applies to hand cards).
                        let per_unit_loc = effect.per_unit_location_any();
                        let loc2 = effect.location_any();
                        let count_zone = per_unit_loc.or(loc2).unwrap_or(Zone::Hand.to_str());
                        let count = if count_zone == "stage" && effect.group_names_any().is_some() {
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
                        let per_unit_count = effect.per_unit_count_any().unwrap_or(1);
                        let exclude_self = effect.exclude_self_any().unwrap_or(false);
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
                        effect.operation_any().as_deref(),
                        value
                    );

                    let op_str = effect.operation_any().unwrap_or("add");
                    let op = op_str;
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

        let old_bonuses = core::mem::take(&mut self.mods.constant_cost_bonuses);
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
            effect_data: Some(crate::core::types::EffectData::HeartOverride {
                card_id,
                color: format!("{:?}", color),
                count,
            }),
        });
    }

    pub fn record_area_placement(&mut self, player_id: &str, area: &str) {
        let key = format!("{}:{}", player_id, area);
        if !self.areas_placed_this_turn.contains(&key) {
            self.areas_placed_this_turn.push(key);
        }
    }

    pub fn has_area_been_placed_this_turn(&self, player_id: &str, area: &str) -> bool {
        let key = format!("{}:{}", player_id, area);
        self.areas_placed_this_turn.contains(&key)
    }

    pub fn clear_area_placement_tracking(&mut self) {
        self.areas_placed_this_turn.clear();
    }

    pub fn record_card_appearance(&mut self, card_id: i16, source: &str) {
        if !self.cards_appeared_this_turn.contains(&card_id) {
            self.cards_appeared_this_turn.push(card_id);
        }
        if !self.recently_appeared_cards.contains(&card_id) {
            self.recently_appeared_cards.push(card_id);
        }
        if !source.is_empty() {
            self.card_appearance_source
                .push((card_id, source.to_string()));
        }
    }

    pub fn has_card_appeared_this_turn(&self, card_id: i16) -> bool {
        self.cards_appeared_this_turn.contains(&card_id)
    }

    pub fn get_card_appearance_source(&self, card_id: i16) -> Option<&str> {
        self.card_appearance_source
            .iter()
            .find(|(k, _)| k == &card_id)
            .map(|(_, v)| v.as_str())
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
        let key = card_id.to_string();
        match self
            .auto_ability_trigger_counts
            .iter_mut()
            .find(|(k, _)| *k == key)
        {
            Some((_, count)) => *count += 1,
            None => self.auto_ability_trigger_counts.push((key, 1)),
        }
    }

    pub fn get_auto_ability_trigger_count(&self, card_id: &str) -> u32 {
        self.auto_ability_trigger_counts
            .iter()
            .find(|(k, _)| *k == card_id)
            .map(|(_, c)| *c)
            .unwrap_or(0)
    }

    pub fn clear_auto_ability_trigger_tracking(&mut self) {
        self.auto_ability_trigger_counts.clear();
    }

    pub fn record_turn_limit_usage(&mut self, player_id: &str, card_instance_id: u32) {
        let key = format!("{}:{}", player_id, card_instance_id);
        if let Some((_, count)) = self.turn_limit_usage.iter_mut().find(|(k, _)| *k == key) {
            *count += 1;
        } else {
            self.turn_limit_usage.push((key, 1));
        }
    }

    pub fn get_turn_limit_usage(&self, player_id: &str, card_instance_id: u32) -> u32 {
        let key = format!("{}:{}", player_id, card_instance_id);
        self.turn_limit_usage
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| *v)
            .unwrap_or(0)
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

    pub fn record_baton_touch(&mut self, player_id: &str, arriving_card_id: Option<i16>) {
        if player_id == "p1" {
            self.baton_touch_count_p1 += 1;
        } else {
            self.baton_touch_count_p2 += 1;
        }
        if let Some(cid) = arriving_card_id {
            self.baton_touch_arriving_card_ids.push(cid);
        }
    }

    pub fn get_baton_touch_count(&self, player_id: &str) -> u32 {
        if player_id == "p1" {
            self.baton_touch_count_p1
        } else {
            self.baton_touch_count_p2
        }
    }

    pub fn clear_baton_touch_tracking(&mut self) {
        self.baton_touch_count_p1 = 0;
        self.baton_touch_count_p2 = 0;
        self.baton_touch_arriving_card_ids.clear();
        self.baton_touch_zero_cost = false;
        self.baton_touch_replaced_member_cost = None;
        self.baton_touch_replaced_member_id = None;
        self.baton_touch_arriving_card_id = None;
    }

    pub fn record_card_movement(&mut self, card_id: i16) {
        self.cards_moved_this_turn.push(card_id);
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
            source_zone: crate::types::ZoneId::from_str(source_zone),
            dest_zone: crate::types::ZoneId::from_str(dest_zone),
            cause_card_id,
            cause_player_id: cause_player_id.to_string(),
            effect_only,
            timestamp: self.movement_event_counter,
        };
        // Display-only tracking (skipped in profiling/bot mode):
        if cfg!(not(feature = "profiling")) {
            self.batch_movements.push(event.clone());
        }
        // Track turn-level ALL-zone movement for ability triggers
        self.turn_movements.push(event.clone());
        // Card left the stage → its gained abilities no longer apply
        if source_zone == "stage" && dest_zone != "stage" {
            self.clear_gained_abilities_for_card(moved_card_id);
        }
        let cards = self.recently_moved_cards.get_or_insert_with(SmallVec::new);
        cards.push(moved_card_id);
        self.recently_moved_from_zone = Some(source_zone.to_string());
        // Track turn-level area movement (stage-area-to-stage-area)
        let is_area_move = source_zone == "stage" && dest_zone == "stage";
        if is_area_move {
            self.turn_area_movements.push(event.clone());
            self.position_change_occurred_this_turn = true;
        }
        // Track in cards_moved_this_turn for fast O(1) lookups
        self.cards_moved_this_turn.push(moved_card_id);
    }

    pub fn has_card_moved_this_turn(&self, card_id: i16) -> bool {
        self.cards_moved_this_turn.iter().any(|x| x == &card_id)
    }

    pub fn clear_card_movement_tracking(&mut self) {
        self.cards_moved_this_turn.clear();
        self.turn_movements.clear();
        self.cards_appeared_this_turn.clear();
        self.turn_area_movements.clear();
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

    // Rule 10.2 / Q85 / Q86 / Q104: Deck refresh pending flag
    //
    // `deck_refresh_pending` is set when a mid-effect refresh condition is
    // detected (Rule 10.2.2.1 or 10.2.2.2) but the actual refresh couldn't
    // be performed inline. The flag is checked at the next safe opportunity
    // (usually in process_player_abilities or check_timing).
    pub fn set_deck_refresh_pending(&mut self, pending: bool) {
        self.deck_refresh_pending = pending;
    }

    pub fn is_deck_refresh_pending(&self) -> bool {
        self.deck_refresh_pending
    }

    // Rule 10.2.3 / Q85 / Q86 / Q104: Perform deck refresh
    //
    // Takes ALL cards from the player's waitroom, shuffles them,
    // and places them as the new main deck.
    //
    // Rule 10.2.3 specifies that refreshed cards go UNDER any existing
    // deck cards. This implementation assumes the deck is empty when
    // refresh is called (which is the common case: refresh is triggered
    // when deck = 0). If there ARE existing deck cards, they would be
    // on top and the refreshed cards below.
    //
    // Q85: During look-at-N with insufficient deck:
    //   The already-looked-at cards stay above the refreshed cards.
    //   This is correct because looked_at cards were REMOVED from deck
    //   (via draw), so the deck is empty when refresh fires.
    //
    // Q86: When deck has exactly N cards during look-at-N:
    //   No refresh during look. If the effect discards looked cards,
    //   deck might become 0, triggering refresh AFTER the effect.
    //
    // Q104: During mill-N with insufficient deck:
    //   The just-milled cards go to waitroom, then refresh moves them
    //   back to deck. The remaining N - milled cards are then milled
    //   from the refreshed deck. This is correct because the milled
    //   cards reached the waitroom before refresh was checked.
    pub fn perform_deck_refresh(&mut self, player_id: &str) {
        let player = if player_id == "player1" {
            &mut self.player1
        } else {
            &mut self.player2
        };

        // Rule 10.2.3: Take all waitroom cards
        let waitroom_cards: Vec<i16> = player.waitroom.cards.iter().copied().collect();
        player.waitroom.cards.clear();
        // Place as new deck (if deck had existing cards, these go under)
        // But per Q85/Q104, deck should be empty at this point.
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
        let src = self.current_ability_source_card_id();
        self.push_revealed_card(card_id, src, false, None);
    }

    pub fn remove_revealed_card(&mut self, card_id: i16) {
        self.revealed_cards.retain(|id| *id != card_id);
    }

    pub fn is_card_revealed(&self, card_id: i16) -> bool {
        self.revealed_cards.contains(&card_id)
    }

    pub fn clear_revealed_cards(&mut self) {
        self.revealed_cards.clear();
        self.revealed_card_meta.clear();
    }

    pub fn remove_from_source_hands(&mut self, card_ids: &[i16]) {
        let mut seen = HashSet::<i16>::default();
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
        self.gained_card_abilities.remove(&card_id);
    }

    pub fn has_gained_ability(&self, card_id: i16, ability_type: &str) -> bool {
        self.gained_abilities
            .get(&card_id)
            .is_some_and(|a| a.iter().any(|x| x == ability_type))
    }

    pub fn clear_gained_abilities_for_card(&mut self, card_id: i16) {
        self.gained_abilities.remove(&card_id);
        self.gained_card_abilities.remove(&card_id);
    }

    /// Evaluate all constant (常時) abilities on cards in the success_live_card_zone.
    /// Handles the following action types:
    ///   - modify_required_hearts: heart requirement reductions (existing behavior)
    ///   - gain_resource(blade): blade grants to stage members
    ///   - gain_resource(heart): heart grants to stage members
    ///   - modify_score: score bonuses to live cards
    ///   - sequential: recurses into sub-actions
    /// Uses a clear-and-re-evaluate pattern to ensure as_long_as semantics: when a
    /// card leaves the success zone, its modifier is not re-applied.
    /// Evaluate all constant (常時) abilities on cards in the success_live_card_zone.
    /// Used during the live flow (victory determination and live success triggering).
    /// Clears need_heart_modifiers first, then delegates to
    /// evaluate_success_zone_constant_modifiers for the tracked bonuses.
    pub fn evaluate_success_zone_constant_abilities(&mut self) {
        self.mods.need_heart_modifiers.clear();
        self.evaluate_success_zone_constant_modifiers();
    }

    /// Evaluate constant abilities on success zone cards for tracked bonuses
    /// (blade, heart, score). Does NOT touch need_heart_modifiers.
    /// Called from recalculate_constants on every state change, and from
    /// evaluate_success_zone_constant_abilities during the live flow.
    pub fn evaluate_success_zone_constant_modifiers(&mut self) {
        use crate::ability::condition::ConditionContext;

        if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
            log::debug!("[SZ_DEBUG] evaluate_success_zone_constant_modifiers called");
            log::debug!(
                "[SZ_DEBUG] p1 success zone = {:?}",
                self.player1.success_live_card_zone.cards.to_vec()
            );
            log::debug!(
                "[SZ_DEBUG] p2 success zone = {:?}",
                self.player2.success_live_card_zone.cards.to_vec()
            );
        }

        // ── Clear previously-applied success zone bonuses ──
        let old_sz_blade = core::mem::take(&mut self.mods.success_zone_blade_bonuses);
        for (cid, val) in &old_sz_blade {
            self.mods.remove_blade_modifier(*cid, *val);
        }
        let old_sz_heart = core::mem::take(&mut self.mods.success_zone_heart_bonuses);
        for (cid, cols) in &old_sz_heart {
            for (color_str, delta) in cols {
                let hc = crate::card::parse_heart_color(color_str);
                self.mods.remove_heart_modifier(*cid, hc, *delta);
            }
        }
        let old_sz_score = core::mem::take(&mut self.mods.success_zone_score_bonuses);
        for (cid, val) in &old_sz_score {
            self.mods.remove_score_modifier(*cid, *val);
        }

        // Track non-stackable effects locally so they are reset each evaluation
        let mut local_non_stackable: HashSet<String> = HashSet::default();

        let zone_cards_p1 = self.player1.success_live_card_zone.cards.clone();
        let zone_cards_p2 = self.player2.success_live_card_zone.cards.clone();

        // Collect all (cid, player_index, effect) pairs upfront to avoid borrow conflicts
        let mut entries: Vec<(i16, usize, crate::card::AbilityEffect)> = Vec::new();
        for (player_idx, zone_cards) in [(0usize, &zone_cards_p1), (1, &zone_cards_p2)] {
            for cid in zone_cards {
                let card = match self.card_database.get_card(*cid) {
                    Some(c) => c.clone(),
                    None => continue,
                };
                for ar in &card.abilities {
                    let ability = ar.resolve();
                    let is_constant = ability
                        .triggers
                        .as_ref()
                        .is_some_and(|t| t.contains(crate::triggers::CONSTANT));
                    if !is_constant {
                        continue;
                    }
                    if let Some(effect) = ability.effect.as_ref() {
                        entries.push((*cid, player_idx, (**effect).clone()));
                    }
                }
            }
        }

        for (cid, player_idx, effect) in &entries {
            let prev_activating = self.activating_card;
            self.activating_card = Some(*cid);
            let self_player = match player_idx {
                0 => Some(&self.player1),
                1 => Some(&self.player2),
                _ => None,
            };
            let ctx = ConditionContext::new_with_self(self, self_player);
            if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
                log::debug!("[SZ_DEBUG] cid={} effect={}", cid, effect.action);
            }
            let cond_met = effect
                .condition
                .as_ref()
                .is_none_or(|c| ctx.evaluate_condition(c));
            if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
                log::debug!("[SZ_DEBUG] cond_met={}", cond_met);
            }
            if !cond_met {
                self.activating_card = prev_activating;
                continue;
            }
            if effect.non_stackable.unwrap_or(false) {
                let effect_key = format!("{}:{}", effect.action, effect.text);
                if local_non_stackable.contains(&effect_key) {
                    self.activating_card = prev_activating;
                    continue;
                }
                local_non_stackable.insert(effect_key);
            }

            self.apply_success_zone_effect(*cid, *player_idx, effect);
            self.activating_card = prev_activating;
        }
    }

    /// Apply a single success zone constant effect. Called by
    /// evaluate_success_zone_constant_modifiers and recursively for sequential sub-actions.
    fn apply_success_zone_effect(
        &mut self,
        cid: i16,
        player_idx: usize,
        effect: &crate::card::AbilityEffect,
    ) {
        use crate::ability::enums::ActionType;
        use crate::ability::resolver::AbilityResolver;

        // Resolve the correct player directly since these effects don't go through the ability queue
        let owner_player = match player_idx {
            0 => &mut self.player1,
            1 => &mut self.player2,
            _ => return,
        };

        match effect.action {
            ActionType::ModifyRequiredHearts => {
                let prev = self.activating_card;
                self.activating_card = Some(cid);
                // Set queue context so resolve_target_player("self") targets
                // the correct owner, not always player1.
                let owner_id = match player_idx {
                    0 => "player1",
                    1 => "player2",
                    _ => "player1",
                };
                self.ability_queue
                    .push_constant_context(owner_id.to_string());
                let mut resolver = AbilityResolver::new(self.card_database.clone(), Some(cid));
                let _ = resolver.execute_modify_required_hearts(
                    self,
                    effect.operation_any().as_deref().unwrap_or("decrease"),
                    effect.value_or_count(0),
                    effect.heart_colors_any(),
                    effect.target_name(),
                    effect.per_unit_any().unwrap_or(false),
                    effect.per_unit_count_any().unwrap_or(1),
                    effect.group_name(),
                    effect.timing_condition_any().as_deref(),
                    effect.location_any().as_deref(),
                    effect.original_value_any(),
                    effect.original_count_any(),
                    effect.original_operator_any().as_deref(),
                    effect.exclude_self_any().unwrap_or(false),
                    effect.self_target_any().unwrap_or(false),
                    effect.exclude_heart_colors_any(),
                    effect.max.unwrap_or(false),
                    effect.repeat_limit_any(),
                    &effect.per_unit_heart_colors_any(),
                );
                self.ability_queue.pop_constant_context();
                self.activating_card = prev;
            }
            ActionType::GainResource => {
                let resource_binding = effect.resource_any();
                let resource = resource_binding.unwrap_or("");
                let amount = effect
                    .resource_icon_count_any()
                    .unwrap_or(effect.count_or(1)) as i32;
                let card_db = self.card_database.clone();
                let player = match effect.target_name() {
                    "self" | "自分" => owner_player,
                    "opponent" | "相手" => match player_idx {
                        0 => &mut self.player2,
                        1 => &mut self.player1,
                        _ => return,
                    },
                    _ => owner_player,
                };
                if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed)
                {
                    log::debug!(
                        "[SZ_DEBUG] GainResource resource={} amount={} target={} position={:?}",
                        resource,
                        amount,
                        effect.target_name(),
                        effect.position_any()
                    );
                    log::debug!("[SZ_DEBUG] stage={:?}", player.stage.stage);
                }

                let candidates: Vec<i16> = player
                    .stage
                    .stage
                    .iter()
                    .enumerate()
                    .filter(|&(_, &idx)| idx != -1)
                    .filter(|&(pos, _)| {
                        if let Some(ref pos_req) = effect.position_any() {
                            let pos_str = pos_req.get_position();
                            match pos_str {
                                Some("center") => pos == 1,
                                Some("left") | Some("left_side") => pos == 0,
                                Some("right") | Some("right_side") => pos == 2,
                                _ => true,
                            }
                        } else {
                            true
                        }
                    })
                    .filter(|&(_, &id)| {
                        if let Some(ref groups) = effect.group_names_any() {
                            groups.iter().any(|g| {
                                crate::ability::util::card_matches_group_str(
                                    &card_db,
                                    id,
                                    Some(g.as_str()),
                                )
                            })
                        } else {
                            true
                        }
                    })
                    .map(|(_, &id)| id)
                    .collect();

                if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed)
                {
                    log::debug!(
                        "[SZ_DEBUG] GainResource resource={} amount={}",
                        resource,
                        amount
                    );
                    log::debug!(
                        "[SZ_DEBUG] candidates count={} ids={:?}",
                        candidates.len(),
                        candidates
                    );
                }
                match resource {
                    "blade" | "ブレード" => {
                        for &target_id in &candidates {
                            if crate::ability::debug::ABILITY_DEBUG
                                .load(core::sync::atomic::Ordering::Relaxed)
                            {
                                log::debug!(
                                    "[SZ_DEBUG] ADDING blade {} to target {}",
                                    amount,
                                    target_id
                                );
                            }
                            self.mods.add_blade_modifier(target_id, amount);
                            *self
                                .mods
                                .success_zone_blade_bonuses
                                .entry(target_id)
                                .or_insert(0) += amount;
                        }
                    }
                    "heart" | "ハート" => {
                        let heart_colors = if effect.heart_colors_any().is_empty() {
                            vec!["heart01".to_string()]
                        } else {
                            effect.heart_colors_any().to_vec()
                        };
                        for &target_id in &candidates {
                            for color_str in &heart_colors {
                                let hc = crate::card::parse_heart_color(color_str);
                                self.mods.add_heart_modifier(target_id, hc, amount);
                                *self
                                    .mods
                                    .success_zone_heart_bonuses
                                    .entry(target_id)
                                    .or_default()
                                    .entry(color_str.clone())
                                    .or_insert(0) += amount;
                            }
                        }
                    }
                    _ => {}
                }
            }
            ActionType::ModifyScore => {
                let player = match effect.target_name() {
                    "self" | "自分" => owner_player,
                    "opponent" | "相手" => match player_idx {
                        0 => &mut self.player2,
                        1 => &mut self.player1,
                        _ => return,
                    },
                    _ => owner_player,
                };
                let value = effect.value_or_count(1) as i32;
                let op_binding = effect.operation_any();
                let op = op_binding.unwrap_or("add");
                // When self_target is true, apply the score modifier to the
                // success zone card itself (e.g. Angelic Angel's +5 self buff).
                // Otherwise, target cards in the live set zone.
                let targets: Vec<i16> = if effect.self_target_any().unwrap_or(false) {
                    vec![cid]
                } else {
                    player.live_card_zone.cards.to_vec()
                };
                for &target_id in &targets {
                    match op {
                        "set" => {
                            self.mods.set_score_modifier(target_id, value);
                            self.mods
                                .success_zone_score_bonuses
                                .insert(target_id, value);
                        }
                        _ => {
                            self.mods.add_score_modifier(target_id, value);
                            *self
                                .mods
                                .success_zone_score_bonuses
                                .entry(target_id)
                                .or_insert(0) += value;
                        }
                    }
                }
            }
            ActionType::Sequential => {
                if let Some(ref actions) = effect.compound.actions {
                    for sub in actions {
                        self.apply_success_zone_effect(cid, player_idx, sub);
                    }
                }
            }
            _ => {}
        }
    }
}
