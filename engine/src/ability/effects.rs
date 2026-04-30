use crate::card::AbilityEffect;
use super::types::{Choice, ExecutionContext, LookAndSelectStep};
use super::resolver::AbilityResolver;
use super::cost::ae_from_ir;

#[allow(dead_code)]
impl<'a> AbilityResolver<'a> {
    pub fn execute_effect_ir(&mut self, effect: &crate::ir::effect::Effect) -> Result<(), String> {
        use crate::ir::effect::{Effect as E, Count as C, Target as T, Resource as R, StateChange as SC};

        let variant_tag = match effect {
            E::MoveCards { .. } => "move_cards",
            E::DrawCards { .. } => "draw_card",
            E::GainResource { .. } => "gain_resource",
            E::ChangeState { .. } => "change_state",
            E::Reveal { .. } => "reveal",
            E::Appear { .. } => "appear",
            E::ModifyScore { .. } => "modify_score",
            E::Sequential(_) | E::Choice { .. } | E::ConditionalAlternative { .. } => "",
            _ => "",
        };

        if !variant_tag.is_empty() {
            self.game_state.reset_replacement_effect_flags();
            let replacements: Vec<crate::game_state::ReplacementEffect> = self.game_state.get_replacement_effects_for_event(variant_tag)
                .iter().map(|r| (*r).clone()).collect();
            if !replacements.is_empty() {
                for replacement in &replacements {
                    if replacement.is_choice_based {
                        self.pending_choice = Some(Choice::SelectTarget {
                            target: "apply_replacement".to_string(),
                            description: format!("Apply replacement effect for '{}'?", variant_tag),
                        });
                        return Err("Pending choice required: apply replacement effect".to_string());
                    }
                    for re in &replacement.replacement_effects {
                        self.execute_effect(re)?;
                    }
                    self.game_state.mark_replacement_effect_applied(replacement.card_id);
                }
                return Ok(());
            }
        }

        match effect {
            E::Custom { text } => {
                eprintln!("Custom effect (unparseable): {}", text);
                Ok(())
            }
            E::Sequential(actions) => {
                for action in actions {
                    if let Err(e) = self.execute_effect_ir(action) {
                        if e.contains("Pending choice required") { return Ok(()); }
                        return Err(e);
                    }
                    if self.pending_choice.is_some() { return Ok(()); }
                }
                Ok(())
            }
            E::Choice { options, choice_type, target: _ } => {
                let flat_options: Vec<AbilityEffect> = options.iter().map(|opt| {
                    AbilityEffect {
                        text: format!("{:?}", opt), action: "custom".into(), ..Default::default()
                    }
                }).collect();
                let ae = AbilityEffect {
                    action: "choice".into(), options: Some(flat_options),
                    choice_type: choice_type.clone(), ..Default::default()
                };
                self.execute_effect(&ae)
            }
            E::ConditionalAlternative { condition, primary, alternative, alternative_condition } => {
                let mut ae = AbilityEffect {
                    action: "conditional_alternative".into(),
                    condition: Some(condition.clone().into()),
                    primary_effect: Some(Box::new(ae_from_ir(primary))),
                    ..Default::default()
                };
                if let Some(alt) = alternative {
                    ae.alternative_effect = Some(Box::new(ae_from_ir(alt)));
                }
                if let Some(altc) = alternative_condition {
                    ae.alternative_condition = Some(altc.clone().into());
                }
                self.execute_effect(&ae)
            }
            E::MoveCards { source, destination, count, card_type, target, group, cost_limit, total_cost_limit, placement_order, state_change, shuffle, optional, distinct, self_cost, exclude_self } => {
                let source_str = format!("{:?}", source).to_lowercase();
                let dst_str = format!("{:?}", destination).to_lowercase();
                let count_val = match count { C::Fixed(n) => *n, C::UpTo(n) => *n, _ => 1 };
                let target_str = match target { T::Self_ => "self", T::Opponent => "opponent", T::Both => "both", T::Either => "either", T::Player(p) => p.as_str() };
                let ae = AbilityEffect {
                    action: "move_cards".into(), source: Some(source_str), destination: Some(dst_str),
                    count: Some(count_val), card_type: card_type.as_ref().map(|ct| format!("{:?}", ct)),
                    target: Some(target_str.into()), group: group.as_ref().map(|g| crate::card::GroupInfo { name: g.clone() }),
                    cost_limit: *cost_limit, total_cost_limit: *total_cost_limit, placement_order: placement_order.clone(),
                    state_change: state_change.as_ref().map(|s| match s { SC::Wait => "wait", SC::Active => "active" }.into()),
                    shuffle_target: if *shuffle { Some("deck".into()) } else { None },
                    optional: Some(*optional), distinct: distinct.clone(), self_cost: Some(*self_cost), exclude_self: Some(*exclude_self),
                    max: matches!(count, C::UpTo(_)).then_some(true), ..Default::default()
                };
                self.execute_effect(&ae)
            }
            E::DrawCards { count, target, optional } => {
                let count_val = match count { C::Fixed(n) => *n, _ => 1 };
                let target_str = match target { T::Self_ => "self", T::Opponent => "opponent", _ => "self" };
                let ae = AbilityEffect { action: "draw_card".into(), count: Some(count_val), target: Some(target_str.into()), source: Some("deck".into()), destination: Some("hand".into()), optional: Some(*optional), ..Default::default() };
                self.execute_effect(&ae)
            }
            E::GainResource { resource, count, target, duration, group, per_unit } => {
                let count_val = match count { C::Fixed(n) => *n, _ => 1 };
                let target_str = match target { T::Self_ => "self", T::Opponent => "opponent", _ => "self" };
                let mut ae = AbilityEffect {
                    action: "gain_resource".into(), count: Some(count_val), target: Some(target_str.into()),
                    resource: Some(match resource { R::Blade => "blade", R::Energy => "energy", R::Score => "score", R::Draw => "draw", _ => "blade" }.into()),
                    duration: duration.as_ref().map(|d| match d { crate::ir::effect::Duration::ThisTurn => "this_turn", crate::ir::effect::Duration::ThisLive => "this_live", crate::ir::effect::Duration::LiveEnd => "live_end", _ => "permanent" }.into()),
                    group: group.as_ref().map(|g| crate::card::GroupInfo { name: g.clone() }),
                    per_unit: Some(per_unit.is_some()), ..Default::default()
                };
                if let R::Heart(hc) = resource {
                    ae.resource = Some(format!("{:?}", hc));
                    ae.heart_color = Some(format!("{:?}", hc));
                }
                self.execute_effect(&ae)
            }
            E::ChangeState { state_change, count, target, card_type, cost_limit, group, optional } => {
                let count_val = match count { C::Fixed(n) => *n, _ => 1 };
                let target_str = match target { T::Self_ => "self", T::Opponent => "opponent", _ => "self" };
                let ae = AbilityEffect { action: "change_state".into(), state_change: Some(match state_change { SC::Wait => "wait", SC::Active => "active" }.into()), count: Some(count_val), target: Some(target_str.into()), card_type: card_type.as_ref().map(|ct| format!("{:?}", ct)), cost_limit: *cost_limit, group: group.as_ref().map(|g| crate::card::GroupInfo { name: g.clone() }), optional: Some(*optional), ..Default::default() };
                self.execute_effect(&ae)
            }
            other => {
                let ae = ae_from_ir(other);
                self.execute_effect(&ae)
            }
        }
    }

    pub fn execute_effect(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        eprintln!("DEBUG: EXECUTING EFFECT - action: {}, text: {}, target: {:?}",
            effect.action, effect.text, effect.target);

        if !self.can_activate_effect(effect) {
            eprintln!("DEBUG: Activation condition not met, skipping effect");
            return Ok(());
        }

        if let Some(ref condition) = effect.condition {
            eprintln!("DEBUG: Checking effect condition: {:?}", condition);
            if !self.evaluate_condition(condition) {
                eprintln!("DEBUG: Effect condition not met, skipping effect");
                return Ok(());
            }
        }

        if effect.action_by.as_deref() == Some("opponent") {
            if let Some(ref opponent_action) = effect.opponent_action {
                self.execute_effect(opponent_action)?;
            }
        }

        self.game_state.reset_replacement_effect_flags();
        let action_to_use = effect.action.clone();

        let replacement_effects: Vec<crate::game_state::ReplacementEffect> = self.game_state.get_replacement_effects_for_event(&action_to_use)
            .iter().map(|r| (*r).clone()).collect();
        if !replacement_effects.is_empty() {
            for replacement in &replacement_effects {
                if replacement.is_choice_based {
                    let description = format!("Apply replacement effect for action '{}'?", action_to_use);
                    self.pending_choice = Some(Choice::SelectTarget { target: "apply_replacement".to_string(), description });
                    return Err("Pending choice required: apply replacement effect".to_string());
                } else {
                    for replacement_effect in &replacement.replacement_effects {
                        self.execute_effect(replacement_effect)?;
                    }
                    self.game_state.mark_replacement_effect_applied(replacement.card_id);
                }
            }
            return Ok(());
        }

        if let Some(ref effect_type) = effect.effect_type {
            if effect_type == "replacement" {
                let original_event = effect.replaces_event.clone();
                let is_choice_based = effect.choice_based.unwrap_or(false);
                let card_id = self.game_state.activating_card.unwrap_or(-1);
                let player_id = if self.game_state.current_turn_phase == crate::game_state::TurnPhase::FirstAttackerNormal {
                    self.game_state.player1.id.clone()
                } else {
                    self.game_state.player2.id.clone()
                };
                if let Some(event) = original_event {
                    self.game_state.add_replacement_effect(card_id, player_id, event.clone(), vec![effect.clone()], is_choice_based);
                }
                return Ok(());
            }
        }

        eprintln!("DEBUG: Executing effect action: '{}'", action_to_use);
        match action_to_use.as_str() {
            "sequential" => self.execute_sequential_effect(effect),
            "conditional_alternative" => self.execute_conditional_alternative(effect),
            "look_and_select" => self.execute_look_and_select(effect),
            "draw" | "draw_card" => self.execute_draw(effect),
            "draw_until_count" => self.execute_draw_until_count(effect),
            "move_cards" => self.execute_move_cards(effect),
            "gain_resource" => self.execute_gain_resource(effect),
            "change_state" => self.execute_change_state(effect),
            "modify_score" => self.execute_modify_score(effect),
            "modify_required_hearts" => self.execute_modify_required_hearts(effect),
            "set_cost" => self.execute_set_cost(effect),
            "set_blade_type" => self.execute_set_blade_type(effect),
            "set_heart_type" => self.execute_set_heart_type(effect),
            "activate_ability" => self.execute_activate_ability(effect),
            "invalidate_ability" => self.execute_invalidate_ability(effect),
            "gain_ability" => self.execute_gain_ability(effect),
            "play_baton_touch" => self.execute_play_baton_touch(effect),
            "reveal" => self.execute_reveal(effect),
            "select" => self.execute_select(effect),
            "look_at" => self.execute_look_at(effect),
            "modify_required_hearts_global" => self.execute_modify_required_hearts_global(effect),
            "modify_yell_count" => self.execute_modify_yell_count(effect),
            "place_energy_under_member" => self.execute_place_energy_under_member(effect),
            "activation_cost" => self.execute_activation_cost(effect),
            "position_change" => self.execute_position_change(effect),
            "appear" => self.execute_appear(effect),
            "choice" => self.execute_choice(effect),
            "pay_energy" => self.execute_pay_energy(effect),
            "set_card_identity" => self.execute_set_card_identity(effect),
            "discard_until_count" => self.execute_discard_until_count(effect),
            "restriction" => self.execute_restriction(effect),
            "re_yell" => self.execute_re_yell(effect),
            "modify_cost" => self.execute_modify_cost(effect),
            "activation_restriction" => self.execute_activation_restriction(effect),
            "choose_required_hearts" => self.execute_choose_required_hearts(effect),
            "modify_limit" => self.execute_modify_limit(effect),
            "set_blade_count" => self.execute_set_blade_count(effect),
            "set_required_hearts" => self.execute_set_required_hearts(effect),
            "set_score" => self.execute_set_score(effect),
            "specify_heart_color" => self.execute_specify_heart_color(effect),
            "modify_required_hearts_success" => self.execute_modify_required_hearts_success(effect),
            "set_cost_to_use" => self.execute_set_cost_to_use(effect),
            "all_blade_timing" => self.execute_all_blade_timing(effect),
            "set_card_identity_all_regions" => self.execute_set_card_identity_all_regions(effect),
            "shuffle" => self.execute_shuffle(effect),
            "reveal_per_group" => self.execute_reveal_per_group(effect),
            "conditional_on_result" => self.execute_conditional_on_result(effect),
            "conditional_on_optional" => self.execute_conditional_on_optional(effect),
            "custom" => { eprintln!("DEBUG: Executing custom effect (not implemented)"); Ok(()) }
            unknown_action => { eprintln!("DEBUG: Unknown action type: '{}', skipping", unknown_action); Ok(()) }
        }
    }

    fn execute_sequential_effect(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let conditional = effect.conditional.unwrap_or(false) || effect.condition.is_some();
        let condition = effect.condition.as_ref();
        let is_further = effect.is_further.unwrap_or(false);

        if conditional {
            if let Some(cond) = condition {
                let condition_met = self.evaluate_condition(cond);
                if !condition_met { return Ok(()); }
            }
        }

        if is_further { eprintln!("Further conditional effect (さらに) - executing additional actions"); }

        if let Some(ref actions) = effect.actions {
            for (i, action) in actions.iter().enumerate() {
                let mut action_to_execute = action.clone();
                if action_to_execute.per_unit.is_none() && effect.per_unit.is_some() {
                    action_to_execute.per_unit = effect.per_unit;
                }
                if action_to_execute.per_unit_count.is_none() && effect.per_unit_count.is_some() {
                    action_to_execute.per_unit_count = effect.per_unit_count;
                }
                if action_to_execute.per_unit_type.is_none() && effect.per_unit_type.is_some() {
                    action_to_execute.per_unit_type = effect.per_unit_type.clone();
                }

                match self.execute_effect(&action_to_execute) {
                    Ok(_) => {},
                    Err(e) if e.contains("Pending choice required") => {
                        let remaining_actions: Vec<AbilityEffect> = actions[i + 1..].to_vec();
                        if !remaining_actions.is_empty() {
                            self.game_state.pending_sequential_actions = Some(remaining_actions);
                        }
                        return Ok(());
                    }
                    Err(e) => return Err(e),
                }
            }
        }
        Ok(())
    }

    fn execute_conditional_alternative(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let has_primary = effect.primary_effect.is_some();
        let has_alternative = effect.alternative_effect.is_some();

        if has_primary && has_alternative {
            let primary_text = effect.primary_effect.as_ref().map(|e| e.text.as_str()).unwrap_or("Primary effect");
            let alternative_text = effect.alternative_effect.as_ref().map(|e| e.text.as_str()).unwrap_or("Alternative effect");
            let description = format!("Choose effect:\nPrimary: {}\nAlternative: {}", primary_text, alternative_text);
            self.pending_choice = Some(Choice::SelectTarget { target: "primary|alternative".to_string(), description });
            self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
            return Ok(());
        }

        if let Some(ref alt_condition) = effect.alternative_condition {
            if self.evaluate_condition(alt_condition) {
                if let Some(ref alt_effect) = effect.alternative_effect {
                    return self.execute_effect(alt_effect);
                }
            }
        }

        if let Some(ref primary_effect) = effect.primary_effect {
            self.execute_effect(primary_effect)
        } else { Ok(()) }
    }

    fn execute_look_and_select(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        self.current_effect = Some(effect.clone());

        if let Some(ref look_action) = effect.look_action {
            self.execute_effect(look_action)?;
        }

        if let Some(ref select_action) = effect.select_action {
            let placement_order = select_action.placement_order.as_deref();
            let count = select_action.count.unwrap_or(1);
            let optional = select_action.optional.unwrap_or(false);
            let any_number = select_action.any_number.unwrap_or(false);

            let available_count = self.looked_at_cards.len();
            let max_select = if any_number { available_count } else { count as usize };

            let description = if any_number {
                format!("Select any number of cards from the {} looked-at cards (or skip) (placement_order: {})",
                    available_count, placement_order.unwrap_or("default"))
            } else if optional {
                format!("Select up to {} card(s) from the {} looked-at cards (or skip) (placement_order: {})",
                    count, available_count, placement_order.unwrap_or("default"))
            } else {
                format!("Select {} card(s) from the {} looked-at cards (placement_order: {})",
                    count, available_count, placement_order.unwrap_or("default"))
            };

            let choice = Choice::SelectCard {
                zone: "looked_at".to_string(), card_type: None, count: max_select,
                description, allow_skip: optional || any_number,
            };
            self.pending_choice = Some(choice);
            self.execution_context = ExecutionContext::LookAndSelect {
                step: LookAndSelectStep::Select { count: max_select },
            };
            return Ok(());
        }

        if let Some(ref select_action) = effect.select_action {
            self.execute_effect(select_action)?;
        }

        self.current_effect = None;
        Ok(())
    }

    fn execute_draw(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let count = effect.count.unwrap_or(1);
        let target = effect.target.as_deref().unwrap_or("self");
        let source = effect.source.as_deref().unwrap_or("deck");
        let destination = effect.destination.as_deref().unwrap_or("hand");
        let card_type_filter = effect.card_type.as_deref();
        let per_unit = effect.per_unit;
        let per_unit_count = effect.per_unit_count.unwrap_or(1);
        let per_unit_type = effect.per_unit_type.as_deref();
        let card_db = self.game_state.card_database.clone();

        if target == "both" {
            let card_db1 = card_db.clone();
            let card_db2 = card_db.clone();
            { let p1 = &mut self.game_state.player1; Self::draw_cards_for_player(p1, count, source, destination, card_type_filter, &card_db1)?; }
            { let p2 = &mut self.game_state.player2; Self::draw_cards_for_player(p2, count, source, destination, card_type_filter, &card_db2)?; }
            return Ok(());
        }

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        let final_count = if per_unit.unwrap_or(false) {
            let multiplier = match per_unit_type {
                Some("member") | Some("人") => player.stage.stage.iter().filter(|&&c| c != -1).count() as u32,
                Some("energy") => player.energy_zone.cards.len() as u32,
                Some("hand") => player.hand.cards.len() as u32,
                _ => 1,
            };
            count * multiplier * per_unit_count
        } else { count };

        match source.as_ref() {
            "deck" | "deck_top" => { Self::draw_cards_for_player(player, final_count, source, destination, card_type_filter, &card_db)?; }
            _ => { eprintln!("Draw from source '{}' not yet implemented", source); }
        }
        Ok(())
    }

    fn draw_cards_for_player(player: &mut crate::player::Player, count: u32, _source: &str, destination: &str, card_type_filter: Option<&str>, card_db: &crate::card::CardDatabase) -> Result<(), String> {
        let mut drawn = 0;
        while drawn < count {
            if let Some(card) = player.main_deck.draw() {
                let matches_type = match card_type_filter {
                    Some("live_card") => card_db.get_card(card).map(|c| c.is_live()).unwrap_or(false),
                    Some("member_card") => card_db.get_card(card).map(|c| c.is_member()).unwrap_or(false),
                    Some("energy_card") => card_db.get_card(card).map(|c| c.is_energy()).unwrap_or(false),
                    None => true, _ => true,
                };
                if matches_type {
                    match destination.as_ref() {
                        "hand" => player.hand.add_card(card),
                        _ => { player.hand.add_card(card); }
                    }
                    drawn += 1;
                } else { player.main_deck.cards.push(card); }
            } else { break; }
        }
        Ok(())
    }

    fn execute_draw_until_count(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let target_count = effect.count.unwrap_or(1) as usize;
        let target = effect.target.as_deref().unwrap_or("self");
        let destination = effect.destination.as_deref().unwrap_or("hand");
        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };
        let current_count = match destination {
            "hand" => player.hand.len(),
            _ => { return Ok(()); }
        };
        let to_draw = target_count.saturating_sub(current_count);
        let mut draw_effect = effect.clone();
        draw_effect.count = Some(to_draw as u32);
        self.execute_draw(&draw_effect)
    }

    fn execute_place_energy_under_member(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let target = effect.target.as_deref().unwrap_or("self");
        let count = effect.count.unwrap_or(1);
        let position = effect.position.as_ref().and_then(|p| p.get_position());
        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };
        let mut energy_cards = Vec::new();
        for _ in 0..count {
            if let Some(energy_card) = player.energy_zone.cards.pop() { energy_cards.push(energy_card); }
            else { break; }
        }
        let target_index = match position {
            Some("center") | Some("中央") => 1,
            Some("left") | Some("左側") => 0,
            Some("right") | Some("右側") => 2,
            None => {
                if player.stage.stage[1] != -1 { 1 }
                else if player.stage.stage[0] != -1 { 0 }
                else if player.stage.stage[2] != -1 { 2 }
                else { for card in energy_cards { player.energy_zone.cards.push(card); } return Ok(()); }
            }
            _ => 1,
        };
        if player.stage.stage[target_index] == -1 {
            for card in energy_cards { player.energy_zone.cards.push(card); }
            return Ok(());
        }
        let member_card_id = player.stage.stage[target_index];
        for _ in energy_cards { self.game_state.add_blade_modifier(member_card_id, 1); }
        Ok(())
    }

    fn execute_activation_cost(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let operation = effect.operation.as_deref().unwrap_or("increase");
        let value = effect.value.unwrap_or(0);
        let target = effect.target.as_deref().unwrap_or("self");
        let duration = effect.duration.as_deref();
        let prohibition_text = format!("activation_cost_{}_{}", operation, value);
        match target {
            "self" | "opponent" => { self.game_state.prohibition_effects.push(prohibition_text); }
            _ => {}
        }
        if let Some(duration_str) = duration {
            if duration_str != "permanent" {
                let duration_enum = match duration_str {
                    "live_end" => crate::game_state::Duration::LiveEnd,
                    "this_turn" => crate::game_state::Duration::ThisTurn,
                    "this_live" => crate::game_state::Duration::ThisLive,
                    _ => crate::game_state::Duration::ThisLive,
                };
                self.game_state.temporary_effects.push(crate::game_state::TemporaryEffect {
                    effect_type: format!("activation_cost_{}_{}", operation, value),
                    duration: duration_enum, created_turn: self.game_state.turn_number,
                    created_phase: self.game_state.current_phase.clone(), target_player_id: target.to_string(),
                    description: format!("Modify activation cost by {} {}", operation, value),
                    creation_order: 0, effect_data: None,
                });
            }
        }
        Ok(())
    }

    fn execute_gain_resource(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let resource = effect.resource.as_deref().unwrap_or("").to_string();
        let count = effect.resource_icon_count.unwrap_or(effect.count.unwrap_or(1));
        let target = effect.target.as_deref().unwrap_or("self").to_string();
        let duration = effect.duration.as_deref().map(|s| s.to_string());
        let card_type_filter = effect.card_type.as_deref().map(|s| s.to_string());
        let group_filter = effect.group.as_ref().and_then(|g| Some(g.name.clone()));
        let per_unit_count = effect.per_unit_count.unwrap_or(1);
        let per_unit_type = effect.per_unit_type.as_deref().map(|s| s.to_string());
        let is_temporary = duration.is_some() && duration.as_deref() != Some("permanent");
        let activating_card_id = self.game_state.activating_card;

        let (blade_targets, heart_targets, heart_color, final_count) = {
            let player = match target.as_str() {
                "self" => &mut self.game_state.player1,
                "opponent" => &mut self.game_state.player2,
                _ => &mut self.game_state.player1,
            };
            let card_db = self.game_state.card_database.clone();

            let matches_card_type = |card_id: i16| -> bool {
                match card_type_filter.as_deref() {
                    Some("live_card") => card_db.get_card(card_id).map(|c| c.is_live()).unwrap_or(false),
                    Some("member_card") => card_db.get_card(card_id).map(|c| c.is_member()).unwrap_or(false),
                    Some("energy_card") => card_db.get_card(card_id).map(|c| c.is_energy()).unwrap_or(false),
                    None => true, _ => true,
                }
            };

            let matches_group = |card_id: i16| -> bool {
                match &group_filter {
                    Some(group_name) => card_db.get_card(card_id).map(|c| c.group == *group_name).unwrap_or(false),
                    None => true,
                }
            };

            let per_unit = effect.per_unit;

            let final_count = if per_unit == Some(true) {
                let matching_count = match per_unit_type.as_deref() {
                    Some("stage") => { player.stage.stage.iter().filter(|&&card_id| card_id != -1).filter(|&&card_id| matches_card_type(card_id) && matches_group(card_id)).count() as u32 }
                    Some("hand") => { player.hand.cards.iter().filter(|&&card_id| matches_card_type(card_id) && matches_group(card_id)).count() as u32 }
                    _ => { player.stage.stage.iter().filter(|&&card_id| card_id != -1).filter(|&&card_id| matches_card_type(card_id) && matches_group(card_id)).count() as u32 }
                };
                matching_count * per_unit_count
            } else { count };

            let blade_targets: Vec<i16> = vec![player.stage.stage[0], player.stage.stage[1], player.stage.stage[2]]
                .into_iter().filter(|&card_id| card_id != -1)
                .filter(|&card_id| matches_card_type(card_id) && matches_group(card_id))
                .collect();

            let heart_color = effect.heart_color.clone();
            let heart_targets: Vec<i16> = if resource == "heart" || resource == "ハート" {
                (0..3).filter_map(|i| {
                    let card_id = player.stage.stage[i];
                    if card_id != -1 && matches_card_type(card_id) && matches_group(card_id) { Some(card_id) } else { None }
                }).collect()
            } else { vec![] };

            (blade_targets, heart_targets, heart_color, final_count)
        };

        let mut effect_data: Option<serde_json::Value> = None;

        if resource == "blade" || resource == "ブレード" {
            let blades_to_add = final_count as i32;
            if blade_targets.is_empty() {
                if let Some(card_id) = activating_card_id {
                    self.game_state.add_blade_modifier(card_id, blades_to_add);
                    if is_temporary {
                        let mut data = serde_json::Map::new();
                        data.insert("card_id".to_string(), serde_json::Value::Number(card_id.into()));
                        data.insert("amount".to_string(), serde_json::Value::Number(blades_to_add.into()));
                        effect_data = Some(serde_json::Value::Object(data));
                    }
                }
            } else {
                for &card_id in &blade_targets {
                    self.game_state.add_blade_modifier(card_id, blades_to_add);
                }
            }
        }

        if resource == "heart" || resource == "ハート" {
            let color = crate::zones::parse_heart_color(heart_color.as_deref().unwrap_or("heart00"));
            for card_id in heart_targets {
                self.game_state.add_heart_modifier(card_id, color, final_count as i32);
            }
        }

        if is_temporary {
            self.game_state.temporary_effects.push(crate::game_state::TemporaryEffect {
                effect_type: format!("gain_{}", resource),
                duration: match duration.as_deref() { Some("this_turn") => crate::game_state::Duration::ThisTurn, Some("live_end") => crate::game_state::Duration::LiveEnd, _ => crate::game_state::Duration::ThisLive },
                created_turn: self.game_state.turn_number,
                created_phase: self.game_state.current_phase.clone(),
                target_player_id: target.clone(),
                description: format!("Gain {} {}", final_count, resource),
                creation_order: 0, effect_data,
            });
        }

        let pid = self.game_state.active_player().id.clone();
        let card_id = activating_card_id.unwrap_or(-1);
        match resource.as_str() {
            "blade" | "ブレード" => self.game_state.publish_event(crate::events::GameEvent::BladeGained { card_id, player_id: pid, amount: final_count }),
            "heart" | "ハート" => self.game_state.publish_event(crate::events::GameEvent::HeartGained { card_id, player_id: pid, color: heart_color.unwrap_or_default(), amount: final_count }),
            _ => {}
        }
        Ok(())
    }

    fn execute_change_state(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let state_change = effect.state_change.as_deref().unwrap_or("").to_string();
        let target = effect.target.as_deref().unwrap_or("self").to_string();
        let count = effect.count.unwrap_or(1);
        let card_type_filter = effect.card_type.as_deref().map(|s| s.to_string());
        let cost_limit = effect.cost_limit;
        let optional = effect.optional.unwrap_or(false);
        let group_filter = effect.group.as_ref().and_then(|g| Some(g.name.clone()));

        if optional {
            self.pending_choice = Some(Choice::SelectTarget {
                target: "pay_optional_cost:skip_optional_cost".to_string(),
                description: format!("Change state to {} (pay optional cost)?", state_change),
            });
            self.game_state.pending_ability = Some(crate::game_state::PendingAbilityExecution {
                card_no: "change_state".to_string(), player_id: "self".to_string(),
                action_index: 0, effect: effect.clone(),
                conditional_choice: None, activating_card: None, ability_index: 0,
                cost: None, cost_choice: None,
            });
            return Ok(());
        }

        let (wait_cards, active_cards, deactivate_count) = {
            let player = match target.as_str() {
                "self" => &mut self.game_state.player1,
                "opponent" => &mut self.game_state.player2,
                _ => &mut self.game_state.player1,
            };
            let card_db = self.game_state.card_database.clone();

            let matches_card_type = |card_id: i16| -> bool {
                match card_type_filter.as_deref() {
                    Some("live_card") => card_db.get_card(card_id).map(|c| c.is_live()).unwrap_or(false),
                    Some("member_card") => card_db.get_card(card_id).map(|c| c.is_member()).unwrap_or(false),
                    Some("energy_card") => card_db.get_card(card_id).map(|c| c.is_energy()).unwrap_or(false),
                    None => true, _ => true,
                }
            };

            let matches_group = |card_id: i16| -> bool {
                match &group_filter {
                    Some(group_name) => card_db.get_card(card_id).map(|c| c.group == *group_name).unwrap_or(false),
                    None => true,
                }
            };

            let matches_cost_limit = |card_id: i16| -> bool {
                match cost_limit {
                    Some(max_cost) => card_db.get_card(card_id).map(|c| c.cost.unwrap_or(0) <= max_cost).unwrap_or(false),
                    None => true,
                }
            };

            let card_type_for_state = card_type_filter.as_deref().or(Some("energy_card"));

            let mut valid_indices: Vec<usize> = Vec::new();
            for i in 0..player.energy_zone.cards.len() {
                if let Some(&card_id) = player.energy_zone.cards.get(i) {
                    if matches_card_type(card_id) && matches_group(card_id) && matches_cost_limit(card_id) {
                        valid_indices.push(i);
                    }
                }
            }

            if valid_indices.len() < count as usize {
                // Return early via error
                return Err(format!("Not enough energy cards to deactivate: need {}, have {}", count, valid_indices.len()));
            }

            if valid_indices.len() > count as usize {
                self.pending_choice = Some(Choice::SelectCard {
                    zone: "energy_zone".to_string(), card_type: card_type_filter.clone(),
                    count: count as usize,
                    description: format!("Select {} energy card(s) to deactivate (set to wait)", count),
                    allow_skip: false,
                });
                self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                return Ok(());
            }

            let wait_cards: Vec<i16> = valid_indices.iter().take(count as usize).filter_map(|i| {
                if *i < player.energy_zone.cards.len() { Some(player.energy_zone.cards[*i]) } else { None }
            }).collect();

            let mut active_count = 0u32;
            let mut active_cards: Vec<i16> = Vec::new();
            for i in 0..player.energy_zone.cards.len() {
                if active_count >= count { break; }
                if let Some(&card_id) = player.energy_zone.cards.get(i) {
                    if matches_card_type(card_id) && matches_group(card_id) && matches_cost_limit(card_id) {
                        active_cards.push(card_id);
                        active_count += 1;
                    }
                }
            }

            (wait_cards, active_cards, count)
        };

        match state_change.as_str() {
            "wait" | "ウェイト" => {
                for card_id in &wait_cards {
                    self.game_state.add_orientation_modifier(*card_id, "wait");
                }
                // Decrease active energy count
                for _ in 0..deactivate_count {
                    let player = match target.as_str() { "self" => &mut self.game_state.player1, "opponent" => &mut self.game_state.player2, _ => &mut self.game_state.player1 };
                    player.energy_zone.active_energy_count = player.energy_zone.active_energy_count.saturating_sub(1);
                }
            }
            "active" | "アクティブ" => {
                for card_id in &active_cards {
                    self.game_state.add_orientation_modifier(*card_id, "active");
                }
                let player = match target.as_str() { "self" => &mut self.game_state.player1, "opponent" => &mut self.game_state.player2, _ => &mut self.game_state.player1 };
                player.energy_zone.active_energy_count += active_cards.len();
            }
            _ => {}
        }
        Ok(())
    }

    fn execute_modify_score(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let operation = effect.operation.as_deref().unwrap_or("add").to_string();
        let value = effect.value.unwrap_or(0);
        let target = effect.target.as_deref().unwrap_or("self").to_string();
        let duration = effect.duration.as_deref().map(|s| s.to_string());
        let card_type_filter = effect.card_type.as_deref().map(|s| s.to_string());
        let group_filter = effect.group.as_ref().and_then(|g| Some(g.name.clone()));
        let per_unit = effect.per_unit;
        let per_unit_count = effect.per_unit_count.unwrap_or(1);
        let per_unit_type = effect.per_unit_type.as_deref().map(|s| s.to_string());
        let effect_constraint = effect.effect_constraint.as_deref().map(|s| s.to_string());

        let (live_card_ids, final_value) = {
            let player = match target.as_str() {
                "self" => &mut self.game_state.player1,
                "opponent" => &mut self.game_state.player2,
                _ => &mut self.game_state.player1,
            };
            let card_db = self.game_state.card_database.clone();

            let matches_card_type = |card_id: i16| -> bool {
                match card_type_filter.as_deref() {
                    Some("live_card") => card_db.get_card(card_id).map(|c| c.is_live()).unwrap_or(false),
                    Some("member_card") => card_db.get_card(card_id).map(|c| c.is_member()).unwrap_or(false),
                    _ => true,
                }
            };

            let matches_group = |card_id: i16| -> bool {
                match &group_filter {
                    Some(group_name) => card_db.get_card(card_id).map(|c| c.group == *group_name).unwrap_or(false),
                    None => true,
                }
            };

            let final_value = if per_unit.unwrap_or(false) {
                let matching_count = match per_unit_type.as_deref() {
                    Some("hand") => player.hand.cards.iter().filter(|&&card_id| matches_card_type(card_id) && matches_group(card_id)).count() as u32,
                    Some("stage") => player.stage.stage.iter().filter(|&&card_id| card_id != -1).filter(|&&card_id| matches_card_type(card_id) && matches_group(card_id)).count() as u32,
                    _ => 1,
                };
                value * matching_count * per_unit_count
            } else { value };

            let live_card_ids: Vec<(i16, i32)> = player.live_card_zone.cards.iter()
                .filter(|&&card_id| matches_card_type(card_id) && matches_group(card_id))
                .map(|&card_id| {
                    let delta = match operation.as_str() {
                        "add" => final_value as i32,
                        "remove" => -(final_value as i32),
                        "set" => final_value as i32,
                        _ => 0i32,
                    };
                    (card_id, delta)
                }).collect();

            (live_card_ids, final_value)
        };

        let mut count_applied = 0u32;
        for (card_id, delta) in &live_card_ids {
            if let Some(constraint) = &effect_constraint {
                let current_mod = self.game_state.get_score_modifier(*card_id);
                match constraint.as_str() {
                    "min:0" => { if current_mod + delta < 0 { continue; } }
                    _ => {}
                }
            }
            if operation == "set" { self.game_state.score_modifiers.insert(*card_id, *delta); }
            else { self.game_state.add_score_modifier(*card_id, *delta); }
            count_applied += 1;
        }

        if let Some(duration_str) = &duration {
            if duration_str != "permanent" {
                let duration_enum = match duration_str.as_str() {
                    "this_turn" => crate::game_state::Duration::ThisTurn,
                    "this_live" => crate::game_state::Duration::ThisLive,
                    "live_end" => crate::game_state::Duration::LiveEnd,
                    _ => crate::game_state::Duration::ThisLive,
                };
                self.game_state.temporary_effects.push(crate::game_state::TemporaryEffect {
                    effect_type: format!("modify_score_{}", operation),
                    duration: duration_enum, created_turn: self.game_state.turn_number,
                    created_phase: self.game_state.current_phase.clone(),
                    target_player_id: target.clone(),
                    description: format!("Modify score by {} {} (applied to {} cards)", operation, final_value, count_applied),
                    creation_order: 0, effect_data: None,
                });
            }
        }
        Ok(())
    }

    fn execute_modify_required_hearts(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let operation = effect.operation.as_deref().unwrap_or("decrease").to_string();
        let value = effect.value.unwrap_or(0);
        let heart_color = effect.heart_color.as_deref().unwrap_or("heart00").to_string();
        let target = effect.target.as_deref().unwrap_or("self").to_string();
        let color = crate::zones::parse_heart_color(&heart_color);
        let card_ids: Vec<i16> = {
            let player = match target.as_str() { "self" => &mut self.game_state.player1, "opponent" => &mut self.game_state.player2, _ => &mut self.game_state.player1 };
            player.live_card_zone.cards.to_vec()
        };
        for card_id in card_ids {
            match operation.as_str() {
                "decrease" => { self.game_state.add_need_heart_modifier(card_id, color, -(value as i32)); }
                "increase" => { self.game_state.add_need_heart_modifier(card_id, color, value as i32); }
                "set" => { self.game_state.set_need_heart_modifier(card_id, color, value as i32); }
                _ => return Err(format!("Unknown operation: {}", operation)),
            }
        }
        Ok(())
    }

    fn execute_set_cost(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let value = effect.value.unwrap_or(0);
        let target = effect.target.as_deref().unwrap_or("self");
        let card_type_filter = effect.card_type.as_deref();
        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };
        let card_ids: Vec<i16> = if let Some("live_card") = card_type_filter {
            player.live_card_zone.cards.iter().copied().collect()
        } else if let Some("member_card") = card_type_filter {
            player.stage.stage.iter().filter(|&&id| id != -1).copied().collect()
        } else { player.hand.cards.iter().copied().collect() };
        for card_id in card_ids { self.game_state.set_cost_modifier(card_id, value as i32); }
        Ok(())
    }

    fn execute_set_blade_type(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let blade_type = effect.blade_type.as_deref().unwrap_or("");
        let target = effect.target.as_deref().unwrap_or("self");
        let current_turn = self.game_state.turn_number;
        let current_phase = self.game_state.current_phase.clone();
        let effect_duration = effect.duration.clone();
        let card_db = self.game_state.card_database.clone();
        let target_players = if target == "self" { vec![&self.game_state.player1] } else { vec![&self.game_state.player2] };
        for target_player in target_players {
            for index in 0..3 {
                let card_id = target_player.stage.stage[index];
                if card_id != -1 {
                    let temp_effect = crate::game_state::TemporaryEffect {
                        effect_type: format!("set_blade_type:{}", blade_type),
                        duration: effect_duration.as_ref().map(|d| match d.as_str() {
                            "live_end" => crate::game_state::Duration::LiveEnd,
                            "this_turn" => crate::game_state::Duration::ThisTurn,
                            "this_live" => crate::game_state::Duration::ThisLive,
                            "permanent" => crate::game_state::Duration::Permanent,
                            "as_long_as" => crate::game_state::Duration::ThisLive,
                            _ => crate::game_state::Duration::ThisLive,
                        }).unwrap_or(crate::game_state::Duration::ThisLive),
                        created_turn: current_turn, created_phase: current_phase.clone(),
                        target_player_id: target_player.id.clone(),
                        description: format!("Set blade type to {} for {}", blade_type, card_db.get_card(card_id).map(|c| c.name.as_str()).unwrap_or("unknown")),
                        creation_order: 0, effect_data: None,
                    };
                    self.game_state.temporary_effects.push(temp_effect);
                }
            }
        }
        Ok(())
    }

    fn execute_set_heart_type(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let heart_type = effect.heart_type.as_deref().or(effect.heart_color.as_deref()).unwrap_or("heart00");
        let target = effect.target.as_deref().unwrap_or("self");
        let count = effect.count.unwrap_or(1) as i32;
        let player = match target { "self" => &mut self.game_state.player1, "opponent" => &mut self.game_state.player2, _ => &mut self.game_state.player1 };
        let mut card_ids_to_modify: Vec<i16> = Vec::new();
        for index in 0..3 {
            let card_id = player.stage.stage[index];
            if card_id != -1 { card_ids_to_modify.push(card_id); }
        }
        let color = crate::zones::parse_heart_color(heart_type);
        for card_id in card_ids_to_modify { self.game_state.add_heart_modifier(card_id, color, count); }
        Ok(())
    }

    fn execute_activate_ability(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let ability_text = effect.ability_text.as_deref().unwrap_or("");
        let target = effect.target.as_deref().unwrap_or("self");
        eprintln!("activate_ability: target={}, ability_text={}", target, ability_text);
        if let Some(card_id) = self.game_state.activating_card {
            self.game_state.prohibition_effects.push(format!("activate_ability:{}", card_id));
        }
        Ok(())
    }

    fn execute_invalidate_ability(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        eprintln!("invalidate_ability: target={:?}", effect.target);
        self.game_state.prohibition_effects.push(format!("invalidate_ability"));
        Ok(())
    }

    fn execute_gain_ability(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let ability_text = effect.ability_gain.as_deref().unwrap_or("");
        let target = effect.target.as_deref().unwrap_or("self");
        let duration = effect.duration.as_deref();
        eprintln!("gain_ability: target={}, duration={:?}", target, duration);
        let temp_effect = crate::game_state::TemporaryEffect {
            effect_type: format!("gain_ability:{}", ability_text),
            duration: match duration { Some("this_turn") => crate::game_state::Duration::ThisTurn, Some("live_end") => crate::game_state::Duration::LiveEnd, _ => crate::game_state::Duration::ThisLive },
            created_turn: self.game_state.turn_number, created_phase: self.game_state.current_phase.clone(),
            target_player_id: target.to_string(),
            description: format!("Gained ability: {}", ability_text),
            creation_order: 0, effect_data: None,
        };
        self.game_state.temporary_effects.push(temp_effect);
        Ok(())
    }

    fn execute_play_baton_touch(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let count = effect.count.unwrap_or(1);
        let target = effect.target.as_deref().unwrap_or("self");
        eprintln!("play_baton_touch: count={}, target={}", count, target);
        self.game_state.prohibition_effects.push(format!("baton_touch_allowed:{}", count));
        Ok(())
    }

    fn execute_reveal(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let source = effect.source.as_deref().unwrap_or("hand");
        let count = effect.count.unwrap_or(1);
        let target = effect.target.as_deref().unwrap_or("self");
        let heart_colors = effect.heart_colors.as_ref();

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        let card_ids: Vec<i16> = match source {
            "hand" => player.hand.cards.iter().copied().collect(),
            "deck" => player.main_deck.cards.iter().take(count as usize).copied().collect(),
            "looked_at" => self.looked_at_cards.clone(),
            _ => vec![],
        };

        let revealed = card_ids.clone();

        for card_id in &revealed { self.game_state.revealed_cards.insert(*card_id); }
        Ok(())
    }

    fn execute_select(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let source = effect.source.as_deref().unwrap_or("hand");
        let count = effect.count.unwrap_or(1);
        let target = effect.target.as_deref().unwrap_or("self");
        let card_type_filter = effect.card_type.as_deref();
        let distinct = effect.distinct.as_deref();
        let heart_colors = effect.heart_colors.as_ref();

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        let card_ids: Vec<i16> = match source {
            "hand" => player.hand.cards.iter().copied().collect(),
            "deck" => player.main_deck.cards.iter().take(count as usize).copied().collect(),
            "looked_at" => self.looked_at_cards.clone(),
            _ => vec![],
        };

        let filtered: Vec<i16> = card_ids.iter().filter(|&&card_id| {
            match card_type_filter {
                Some("live_card") => self.game_state.card_database.get_card(card_id).map(|c| c.is_live()).unwrap_or(false),
                Some("member_card") => self.game_state.card_database.get_card(card_id).map(|c| c.is_member()).unwrap_or(false),
                None => true, _ => true,
            }
        }).copied().collect();

        if distinct == Some("true") || distinct == Some("distinct") {
            let mut names = std::collections::HashSet::new();
            let unique: Vec<i16> = filtered.into_iter().filter(|&card_id| {
                self.game_state.card_database.get_card(card_id)
                    .map(|c| names.insert(c.name.clone()))
                    .unwrap_or(false)
            }).collect();
            self.looked_at_cards = unique;
        } else { self.looked_at_cards = filtered; }

        self.pending_choice = Some(Choice::SelectCard {
            zone: source.to_string(), card_type: card_type_filter.map(|s| s.to_string()),
            count: count as usize,
            description: format!("Select {} card(s) from {}", count, source),
            allow_skip: false,
        });
        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
        Ok(())
    }

    fn execute_look_at(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let count = effect.count.unwrap_or(1);
        let target = effect.target.as_deref().unwrap_or("self");
        let source = effect.source.as_deref().unwrap_or("deck");

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        let cards = match source {
            "deck" => player.main_deck.peek_top(count as usize),
            "hand" => player.hand.cards.iter().take(count as usize).copied().collect(),
            _ => vec![],
        };

        self.looked_at_cards = cards;
        Ok(())
    }

    fn execute_modify_required_hearts_global(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let operation = effect.operation.as_deref().unwrap_or("increase").to_string();
        let value = effect.value.unwrap_or(1);
        let heart_color = effect.heart_color.as_deref().unwrap_or("heart00").to_string();
        let target = effect.target.as_deref().unwrap_or("opponent").to_string();
        let color = crate::zones::parse_heart_color(&heart_color);
        let card_ids: Vec<i16> = {
            let player = match target.as_str() { "self" => &mut self.game_state.player1, "opponent" => &mut self.game_state.player2, _ => &mut self.game_state.player1 };
            player.live_card_zone.cards.to_vec()
        };
        for card_id in card_ids {
            let modifier_value = match operation.as_str() { "increase" => value as i32, "decrease" => -(value as i32), _ => return Err(format!("Unknown operation: {}", operation)) };
            self.game_state.add_need_heart_modifier(card_id, color, modifier_value);
        }
        Ok(())
    }

    fn execute_modify_yell_count(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let operation = effect.operation.as_deref().unwrap_or("subtract");
        let count = effect.count.unwrap_or(0);
        match operation {
            "add" => { self.game_state.cheer_checks_required += count; }
            "subtract" => { self.game_state.cheer_checks_required = self.game_state.cheer_checks_required.saturating_sub(count); }
            "set" => { self.game_state.cheer_checks_required = count; }
            _ => return Err(format!("Unknown operation: {}", operation)),
        }
        Ok(())
    }

    fn execute_position_change(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let position = effect.position.as_ref().and_then(|p| p.get_position()).unwrap_or("");
        let target = effect.target.as_deref().unwrap_or("self");
        let target_member = effect.target_member.as_deref().unwrap_or("this_member");

        if target_member == "this_member" {
            if position.is_empty() {
                self.game_state.pending_ability = Some(crate::game_state::PendingAbilityExecution {
                    card_no: "position_change".to_string(), player_id: "self".to_string(),
                    action_index: 0, effect: effect.clone(),
                    conditional_choice: None, activating_card: None, ability_index: 0,
                    cost: None, cost_choice: None,
                });
                self.pending_choice = Some(Choice::SelectTarget {
                    target: "position|destination".to_string(),
                    description: "Choose destination for position change".to_string(),
                });
                return Ok(());
            }
            return self.execute_position_change_with_destination(effect, position);
        }

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        let card_database = self.game_state.card_database.clone();
        let target_index = match position {
            "center" | "センターエリア" => 1,
            "left_side" | "左サイドエリア" => 0,
            "right_side" | "右サイドエリア" => 2,
            _ => return Err(format!("Unknown position: {}", position)),
        };

        let current_index = player.stage.stage.iter().position(|&card_id| {
            if card_id == -1 { false }
            else { card_database.get_card(card_id).map(|c| c.card_no == target_member).unwrap_or(false) }
        });

        if let Some(current_idx) = current_index {
            let card_id = player.stage.stage[current_idx];
            if player.stage.stage[target_index] != -1 {
                let occupying_card = player.stage.stage[target_index];
                player.stage.stage[target_index] = card_id;
                player.stage.stage[current_idx] = occupying_card;
            } else {
                player.stage.stage[target_index] = card_id;
                player.stage.stage[current_idx] = -1;
            }
        } else { return Err(format!("Member not found: {}", target_member)); }
        Ok(())
    }

    pub fn execute_position_change_with_destination(&mut self, effect: &AbilityEffect, destination: &str) -> Result<(), String> {
        let target = effect.target.as_deref().unwrap_or("self");
        let target_member = effect.target_member.as_deref().unwrap_or("this_member");

        if target_member == "this_member" {
            if let Some(activating_card_id) = self.activating_card_id {
                let player = match target {
                    "self" => &mut self.game_state.player1,
                    "opponent" => &mut self.game_state.player2,
                    _ => &mut self.game_state.player1,
                };

                let target_index = match destination {
                    "center" | "センターエリア" => 1,
                    "left_side" | "左サイドエリア" => 0,
                    "right_side" | "右サイドエリア" => 2,
                    _ => return Err(format!("Unknown destination: {}", destination)),
                };

                let current_index = player.stage.stage.iter().position(|&card_id| card_id == activating_card_id);

                if let Some(current_idx) = current_index {
                    if current_idx == target_index { return Ok(()); }
                    let card_id = player.stage.stage[current_idx];
                    if player.stage.stage[target_index] != -1 {
                        let occupying_card = player.stage.stage[target_index];
                        player.stage.stage[target_index] = card_id;
                        player.stage.stage[current_idx] = occupying_card;
                    } else {
                        player.stage.stage[target_index] = card_id;
                        player.stage.stage[current_idx] = -1;
                    }
                    let pid = self.game_state.active_player().id.clone();
                    self.game_state.publish_event(crate::events::GameEvent::MemberPositionChanged {
                        card_id: activating_card_id, player_id: pid,
                        from_area: format!("position_{}", current_idx), to_area: format!("position_{}", target_index),
                    });
                } else { return Err(format!("Activating card {} not found on stage", activating_card_id)); }
            } else { return Err("No activating card for position change".to_string()); }
        }
        Ok(())
    }

    fn execute_appear(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let source = effect.source.as_deref().unwrap_or("");
        let destination = effect.destination.as_deref().unwrap_or("stage");
        let count = effect.count.unwrap_or(1);
        let target = effect.target.as_deref().unwrap_or("self");
        let card_type_filter = effect.card_type.as_deref();

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        match source {
            "deck" => {
                let mut appeared = 0;
                let mut cards_to_record: Vec<i16> = Vec::new();
                while appeared < count {
                    if let Some(card) = player.main_deck.draw() {
                        let matches_type = match card_type_filter {
                            Some("member_card") => self.game_state.card_database.get_card(card).map(|c| c.is_member()).unwrap_or(false),
                            Some("live_card") => self.game_state.card_database.get_card(card).map(|c| c.is_live()).unwrap_or(false),
                            Some("energy_card") => self.game_state.card_database.get_card(card).map(|c| c.is_energy()).unwrap_or(false),
                            None => true, _ => true,
                        };
                        if matches_type {
                            match destination {
                                "stage" => {
                                    if player.stage.stage[1] == -1 { player.stage.stage[1] = card; player.areas_locked_this_turn.insert(crate::zones::MemberArea::Center); }
                                    else if player.stage.stage[0] == -1 { player.stage.stage[0] = card; player.areas_locked_this_turn.insert(crate::zones::MemberArea::LeftSide); }
                                    else if player.stage.stage[2] == -1 { player.stage.stage[2] = card; player.areas_locked_this_turn.insert(crate::zones::MemberArea::RightSide); }
                                    else { player.hand.add_card(card); }
                                    cards_to_record.push(card);
                                }
                                "hand" => player.hand.add_card(card),
                                "discard" => player.waitroom.add_card(card),
                                _ => { eprintln!("Appear destination '{}' not implemented", destination); }
                            }
                            appeared += 1;
                        } else { player.main_deck.cards.push(card); }
                    } else { break; }
                }
                for card_id in cards_to_record { self.game_state.record_card_movement(card_id); }
            }
            "discard" => {
                let mut appeared = 0;
                let mut indices_to_remove = Vec::new();
                for (i, card) in player.waitroom.cards.iter().enumerate() {
                    if appeared >= count { break; }
                    let matches_type = match card_type_filter {
                        Some("member_card") => self.game_state.card_database.get_card(*card).map(|c| c.is_member()).unwrap_or(false),
                        Some("live_card") => self.game_state.card_database.get_card(*card).map(|c| c.is_live()).unwrap_or(false),
                        None => true, _ => true,
                    };
                    if matches_type { indices_to_remove.push(i); appeared += 1; }
                }
                for i in indices_to_remove.into_iter().rev() {
                    let card = player.waitroom.cards.remove(i);
                    player.hand.add_card(card);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn execute_choice(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let choice_options = if let Some(ref string_options) = effect.choice_options {
            serde_json::to_string(string_options).ok()
        } else if let Some(ref options) = effect.options {
            serde_json::to_string(options).ok()
        } else if let Some(ref option) = effect.choice_type {
            Some(option.clone())
        } else { None };

        self.game_state.pending_ability = Some(crate::game_state::PendingAbilityExecution {
            card_no: if choice_options.is_some() { "choice_string".to_string() } else { "choice".to_string() },
            player_id: "self".to_string(), action_index: 0, effect: effect.clone(),
            conditional_choice: choice_options, activating_card: None, ability_index: 0,
            cost: None, cost_choice: None,
        });

        if let Some(ref options) = effect.options {
            let option_texts: Vec<String> = options.iter().map(|o| o.text.clone()).collect();
            self.pending_choice = Some(Choice::SelectTarget {
                target: "choice".to_string(),
                description: format!("Choose one: {}", option_texts.join(" / ")),
            });
        } else if let Some(ref string_options) = effect.choice_options {
            self.pending_choice = Some(Choice::SelectTarget {
                target: "choice_string".to_string(),
                description: format!("Choose one: {}", string_options.join(", ")),
            });
        } else if let Some(ref choice_type) = effect.choice_type {
            self.pending_choice = Some(Choice::SelectTarget {
                target: "choice_type".to_string(),
                description: format!("Choose: {}", choice_type),
            });
        }
        Ok(())
    }

    fn execute_pay_energy(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let energy = effect.count.unwrap_or(0);
        let target = effect.target.as_deref().unwrap_or("self");
        let player = match target { "self" => &mut self.game_state.player1, "opponent" => &mut self.game_state.player2, _ => &mut self.game_state.player1 };
        if energy > 0 { if let Err(e) = player.energy_zone.pay_energy(energy as usize) { return Err(e); } }
        let pid = self.game_state.active_player().id.clone();
        self.game_state.publish_event(crate::events::GameEvent::EnergyPaid { player_id: pid, amount: energy });
        Ok(())
    }

    fn execute_set_card_identity(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let identities = effect.identities.as_ref();
        eprintln!("set_card_identity: identities={:?}", identities);
        if let Some(identities) = identities {
            self.game_state.prohibition_effects.push(format!("card_identity:{}", identities.join(",")));
        }
        Ok(())
    }

    fn execute_discard(&mut self, player_id: &str, target_count: usize) -> Result<(), String> {
        let player = if player_id == "self" { &mut self.game_state.player1 } else { &mut self.game_state.player2 };
        if player.hand.cards.len() > target_count {
            return Err("Pending choice required: select cards to discard from hand".to_string());
        }
        let cards_to_remove: Vec<_> = player.hand.cards.drain(..).collect();
        for card_id in cards_to_remove { player.waitroom.add_card(card_id); }
        Ok(())
    }

    fn execute_discard_until_count(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let target_count = effect.count.unwrap_or(0) as usize;
        let target = effect.target.as_deref().unwrap_or("self");
        let player = if target == "self" { &mut self.game_state.player1 } else { &mut self.game_state.player2 };
        let current_count = player.hand.cards.len();
        if current_count <= target_count { return Ok(()); }
        let cards_to_discard = current_count - target_count;
        if cards_to_discard > 0 {
            return Err("Pending choice required: select cards to discard from hand".to_string());
        }
        Ok(())
    }

    fn execute_restriction(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        eprintln!("restriction: type={:?}, destination={:?}", effect.restriction_type, effect.restricted_destination);
        self.game_state.prohibition_effects.push(format!("restriction:{}:{}", effect.restriction_type.as_deref().unwrap_or("unknown"), effect.restricted_destination.as_deref().unwrap_or("")));
        Ok(())
    }

    fn execute_re_yell(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let lose_blade_hearts = effect.lose_blade_hearts.unwrap_or(false);
        eprintln!("re_yell: lose_blade_hearts={}", lose_blade_hearts);
        let player = &mut self.game_state.player1;
        let mut cards_to_clear_modifiers: Vec<i16> = Vec::new();
        for i in 0..3 {
            if player.stage.stage[i] != -1 {
                let card_id = player.stage.stage[i];
                player.stage.stage[i] = -1;
                player.waitroom.add_card(card_id);
                if lose_blade_hearts { cards_to_clear_modifiers.push(card_id); }
            }
        }
        if lose_blade_hearts {
            for card_id in cards_to_clear_modifiers {
                self.game_state.clear_modifiers_for_card(card_id);
            }
        }
        self.game_state.prohibition_effects.push("re_yell".to_string());
        Ok(())
    }

    fn execute_activation_restriction(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let target = effect.target.as_deref().unwrap_or("self");
        eprintln!("activation_restriction: target={}", target);
        self.game_state.prohibition_effects.push(format!("activation_restriction:{}", target));
        Ok(())
    }

    fn execute_choose_required_hearts(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        eprintln!("choose_required_hearts: choice={:?}, target={:?}", effect.choice, effect.target);
        self.pending_choice = Some(Choice::SelectTarget {
            target: "choose_required_hearts".to_string(),
            description: "Choose required hearts".to_string(),
        });
        Ok(())
    }

    fn execute_modify_limit(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let operation = effect.operation.as_deref().unwrap_or("decrease");
        let count = effect.count.unwrap_or(0);
        eprintln!("modify_limit: operation={}, count={}", operation, count);
        match operation {
            "decrease" => { self.game_state.prohibition_effects.push(format!("limit_decrease:{}", count)); }
            "increase" => { self.game_state.prohibition_effects.push(format!("limit_increase:{}", count)); }
            _ => {}
        }
        Ok(())
    }

    fn execute_set_blade_count(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let value = effect.value.unwrap_or(0);
        let target = effect.target.as_deref().unwrap_or("self").to_string();
        eprintln!("set_blade_count: value={}, target={}", value, target);
        let stage_cards: Vec<i16> = {
            let player = match target.as_str() { "self" => &mut self.game_state.player1, "opponent" => &mut self.game_state.player2, _ => &mut self.game_state.player1 };
            player.stage.stage.to_vec()
        };
        for &card_id in stage_cards.iter().filter(|&&id| id != -1) {
            let current = self.game_state.get_blade_modifier(card_id);
            let delta = (value as i32) - current;
            self.game_state.add_blade_modifier(card_id, delta);
        }
        Ok(())
    }

    fn execute_set_required_hearts(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let count = effect.count.unwrap_or(0);
        let heart_color = effect.heart_color.as_deref().unwrap_or("heart00").to_string();
        let target = effect.target.as_deref().unwrap_or("self").to_string();
        let color = crate::zones::parse_heart_color(&heart_color);
        let card_ids: Vec<i16> = {
            let player = match target.as_str() { "self" => &mut self.game_state.player1, "opponent" => &mut self.game_state.player2, _ => &mut self.game_state.player1 };
            player.live_card_zone.cards.to_vec()
        };
        for card_id in card_ids {
            self.game_state.set_need_heart_modifier(card_id, color, count as i32);
        }
        Ok(())
    }

    fn execute_set_score(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let value = effect.value.unwrap_or(0);
        let target = effect.target.as_deref().unwrap_or("self");
        let player = match target { "self" => &mut self.game_state.player1, "opponent" => &mut self.game_state.player2, _ => &mut self.game_state.player1 };
        player.live_score = value as i32;
        Ok(())
    }

    fn execute_specify_heart_color(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let choice = effect.choice.unwrap_or(false);
        let target = effect.target.as_deref().unwrap_or("self");
        eprintln!("specify_heart_color: choice={}, target={}", choice, target);
        if choice {
            self.pending_choice = Some(Choice::SelectTarget { target: "heart_color".to_string(), description: "Choose a heart color".to_string() });
        }
        Ok(())
    }

    fn execute_set_card_identity_all_regions(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let target = effect.target.as_deref().unwrap_or("self");
        let identities = effect.identities.as_ref();
        let card_id = self.activating_card_id.or_else(|| self.game_state.activating_card);
        if let Some(card_id) = card_id {
            if let Some(identities) = identities {
                eprintln!("Would add identities {:?} to card {}", identities, card_id);
            }
        }
        Ok(())
    }

    fn execute_modify_required_hearts_success(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let operation = effect.operation.as_deref().unwrap_or("increase");
        let value = effect.value.unwrap_or(0);
        let target = effect.target.as_deref().unwrap_or("self");
        let card_type_filter = effect.card_type.as_deref();
        eprintln!("modify_required_hearts_success: operation={}, value={}, target={}, card_type={:?}", operation, value, target, card_type_filter);
        let player = match target { "self" => &mut self.game_state.player1, "opponent" => &mut self.game_state.player2, _ => &mut self.game_state.player1 };
        let card_ids: Vec<i16> = if let Some("live_card") = card_type_filter { player.live_card_zone.cards.iter().copied().collect() } else { vec![] };
        let delta = match operation { "increase" => value as i32, "decrease" => -(value as i32), _ => return Err(format!("Unknown operation: {}", operation)) };
        for card_id in card_ids { eprintln!("Would modify required hearts for card {} by {}", card_id, delta); }
        Ok(())
    }

    fn execute_set_cost_to_use(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let value = effect.value.unwrap_or(0);
        let card_id = self.activating_card_id.or_else(|| self.game_state.activating_card);
        if let Some(card_id) = card_id { self.game_state.set_cost_modifier(card_id, value as i32); }
        Ok(())
    }

    fn execute_all_blade_timing(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let timing = effect.timing.as_deref().unwrap_or("check_required_hearts");
        let treat_as = effect.treat_as.as_deref().unwrap_or("any_heart_color");
        eprintln!("all_blade_timing: timing={}, treat_as={}", timing, treat_as);
        Ok(())
    }

    fn execute_shuffle(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let target = effect.target.as_deref().unwrap_or("deck");
        let player = &mut self.game_state.player1;
        match target {
            "deck" => { use rand::seq::SliceRandom; player.main_deck.cards.shuffle(&mut rand::thread_rng()); }
            "energy_deck" => { use rand::seq::SliceRandom; player.energy_deck.cards.shuffle(&mut rand::thread_rng()); }
            _ => { eprintln!("Unknown shuffle target: {}", target); }
        }
        Ok(())
    }

    fn execute_reveal_per_group(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let source = effect.source.as_deref().unwrap_or("hand");
        let _count = effect.count.unwrap_or(1);
        let _per_unit = effect.per_unit.unwrap_or(false);
        eprintln!("reveal_per_group: source={}", source);
        let player = &mut self.game_state.player1;
        let card_ids: Vec<i16> = match source {
            "hand" => player.hand.cards.iter().copied().collect(),
            "deck" => player.main_deck.cards.iter().copied().collect(),
            "discard" => player.waitroom.cards.iter().copied().collect(),
            _ => vec![],
        };
        eprintln!("Revealing {} cards from {}", card_ids.len(), source);
        Ok(())
    }

    fn execute_conditional_on_result(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let primary_action = effect.primary_effect.as_ref();
        let _result_condition = effect.result_condition.as_ref();
        let followup_action = effect.followup_action.as_ref();
        if let Some(ref primary) = primary_action { self.execute_effect(primary)?; }
        if let Some(ref followup) = followup_action { self.execute_effect(followup)?; }
        Ok(())
    }

    fn execute_conditional_on_optional(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let optional_action = effect.optional_action.as_ref();
        let conditional_action = effect.conditional_action.as_ref();
        if let Some(ref optional) = optional_action { self.execute_effect(optional)?; }
        if let Some(ref conditional) = conditional_action { self.execute_effect(conditional)?; }
        Ok(())
    }

    fn execute_modify_cost(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let operation = effect.operation.as_deref().unwrap_or("add");
        let value = effect.value.unwrap_or(0);
        let target = effect.target.as_deref().unwrap_or("self");
        let card_type_filter = effect.card_type.as_deref();
        let player = match target { "self" => &mut self.game_state.player1, "opponent" => &mut self.game_state.player2, _ => &mut self.game_state.player1 };
        let card_ids: Vec<i16> = if let Some("live_card") = card_type_filter { player.live_card_zone.cards.iter().copied().collect() }
            else if let Some("member_card") = card_type_filter { player.stage.stage.iter().filter(|&&id| id != -1).copied().collect() }
            else if let Some("energy_card") = card_type_filter { player.energy_zone.cards.iter().copied().collect() }
            else { player.hand.cards.iter().copied().collect() };
        let delta = match operation { "add" => value as i32, "subtract" => -(value as i32), "set" => value as i32, _ => return Err(format!("Unknown operation: {}", operation)) };
        for card_id in card_ids {
            if operation == "set" { self.game_state.set_cost_modifier(card_id, delta); }
            else { self.game_state.add_cost_modifier(card_id, delta); }
        }
        Ok(())
    }
}
