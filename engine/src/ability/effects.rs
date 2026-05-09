use crate::card::{AbilityEffect, PositionInfo};
use super::types::{Choice, ExecutionContext};
use super::resolver::AbilityResolver;
use super::util;
use super::debug::AbDebug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EffectAction {
    Sequential, ConditionalAlternative, LookAndSelect,
    Draw, DrawCard, DrawUntilCount, DiscardCard, MoveCards,
    GainResource, ChangeState, ModifyScore, ModifyRequiredHearts,
    SetCost, SetBladeType, SetHeartType, ActivateAbility,
    InvalidateAbility, GainAbility, PlayBatonTouch, Reveal,
    Select, LookAt, ModifyRequiredHeartsGlobal, ModifyYellCount,
    PlaceEnergyUnderMember, ActivationCost, PositionChange, FormationChange, Appear,
    Choice, PayEnergy, SetCardIdentity, RepeatProcedure,
    DiscardUntilCount, Restriction, ReYell, ActivationRestriction,
    ChooseRequiredHearts, ModifyLimit, SetBladeCount, DoNothing,
    SetRequiredHearts, SetScore, SpecifyHeartColor,
    ModifyRequiredHeartsSuccess, SetCostToUse, AllBladeTiming,
    SetCardIdentityAllRegions, Shuffle, RevealPerGroup,
    ConditionalOnResult, ConditionalOnOptional, ModifyCost,
    RevealUntilLiveCard, Custom,
}

impl EffectAction {
    fn from_action(s: &str) -> Self {
        match s {
            "sequential" => Self::Sequential,
            "conditional_alternative" => Self::ConditionalAlternative,
            "look_and_select" => Self::LookAndSelect,
            "draw" => Self::Draw,
            "draw_card" => Self::DrawCard,
            "draw_until_count" => Self::DrawUntilCount,
            "discard_card" => Self::DiscardCard,
            "move_cards" => Self::MoveCards,
            "gain_resource" => Self::GainResource,
            "change_state" => Self::ChangeState,
            "modify_score" => Self::ModifyScore,
            "modify_required_hearts" => Self::ModifyRequiredHearts,
            "set_cost" => Self::SetCost,
            "set_blade_type" => Self::SetBladeType,
            "set_heart_type" => Self::SetHeartType,
            "activate_ability" => Self::ActivateAbility,
            "invalidate_ability" => Self::InvalidateAbility,
            "gain_ability" => Self::GainAbility,
            "play_baton_touch" => Self::PlayBatonTouch,
            "reveal" => Self::Reveal,
            "select" => Self::Select,
            "look_at" => Self::LookAt,
            "modify_required_hearts_global" => Self::ModifyRequiredHeartsGlobal,
            "modify_yell_count" => Self::ModifyYellCount,
            "place_energy_under_member" => Self::PlaceEnergyUnderMember,
            "activation_cost" => Self::ActivationCost,
            "position_change" => Self::PositionChange,
            "formation_change" => Self::FormationChange,
            "appear" => Self::Appear,
            "choice" => Self::Choice,
            "pay_energy" => Self::PayEnergy,
            "set_card_identity" => Self::SetCardIdentity,
            "repeat_procedure" => Self::RepeatProcedure,
            "discard_until_count" => Self::DiscardUntilCount,
            "restriction" => Self::Restriction,
            "re_yell" => Self::ReYell,
            "activation_restriction" => Self::ActivationRestriction,
            "choose_required_hearts" => Self::ChooseRequiredHearts,
            "modify_limit" => Self::ModifyLimit,
            "set_blade_count" => Self::SetBladeCount,
            "do_nothing" => Self::DoNothing,
            "set_required_hearts" => Self::SetRequiredHearts,
            "set_score" => Self::SetScore,
            "specify_heart_color" => Self::SpecifyHeartColor,
            "modify_required_hearts_success" => Self::ModifyRequiredHeartsSuccess,
            "set_cost_to_use" => Self::SetCostToUse,
            "all_blade_timing" => Self::AllBladeTiming,
            "set_card_identity_all_regions" => Self::SetCardIdentityAllRegions,
            "shuffle" => Self::Shuffle,
            "reveal_per_group" => Self::RevealPerGroup,
            "conditional_on_result" => Self::ConditionalOnResult,
            "conditional_on_optional" => Self::ConditionalOnOptional,
            "modify_cost" => Self::ModifyCost,
            "reveal_until_live_card" => Self::RevealUntilLiveCard,
            "custom" => Self::Custom,
            _ => { eprintln!("Unknown effect action: '{}'", s); Self::DoNothing }
        }
    }
}

impl<'a> AbilityResolver<'a> {
    pub fn execute_effect(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let mut dbg = AbDebug::new();
        dbg.effect(effect);
        if !self.can_activate_effect(effect) {
            return Ok(());
        }

        if let Some(ref condition) = effect.condition {
            let ok = self.evaluate_condition(condition);
            if !ok {
                return Ok(());
            }
        }

        if effect.action_by.as_deref() == Some("opponent") {
            if let Some(ref opponent_action) = effect.opponent_action {
                let mut modified = opponent_action.clone();
                if modified.target.is_none() || modified.target.as_deref() == Some("self") {
                    modified.target = Some("opponent".to_string());
                }
                self.execute_effect(&modified)?;
            }
        }

        self.game_state.reset_replacement_effect_flags();
        let action_str = effect.action.clone();

        // Empty action with opponent_action means it was entirely handled by opponent
        if action_str.is_empty() && effect.action_by.is_some() {
            return Ok(());
        }

        let replacement_effects: Vec<crate::game_state::ReplacementEffect> = self.game_state.get_replacement_effects_for_event(&action_str)
            .iter().map(|r| (*r).clone()).collect();
        if !replacement_effects.is_empty() {
            for replacement in &replacement_effects {
                if replacement.is_choice_based {
                    let description = format!("Apply replacement effect for action '{}'?", action_str);
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

        
        // Handle target="both" generically: execute once for each player.
        // position_change handles "both" internally (opponent choice first, then self).
        let is_position_change = effect.action.as_str() == "position_change";
        if effect.target.as_deref() == Some("both") && !is_position_change {
            let mut for_self = effect.clone();
            for_self.target = Some("self".to_string());
            self.execute_effect(&for_self)?;
            let mut for_opponent = effect.clone();
            for_opponent.target = Some("opponent".to_string());
            return self.execute_effect(&for_opponent);
        }

        let action = EffectAction::from_action(&action_str);
        match action {
            EffectAction::Sequential => self.execute_sequential_effect(effect, effect.conditional.unwrap_or(false), effect.is_further.unwrap_or(false)),
            EffectAction::ConditionalAlternative => self.execute_conditional_alternative(effect),
            EffectAction::LookAndSelect => self.execute_look_and_select(effect),
            EffectAction::Draw | EffectAction::DrawCard => self.execute_draw(effect.count_or(1), effect.target_name(), effect.source_or("deck"), effect.destination.as_deref().unwrap_or("hand"), effect.card_type.as_deref(), effect.per_unit.unwrap_or(false), effect.per_unit_count.unwrap_or(1), effect.per_unit_type.as_deref()),
            EffectAction::DrawUntilCount => self.execute_draw_until_count(effect.target_count.unwrap_or(0), effect.target_name(), effect.destination.as_deref().unwrap_or("hand")),
            EffectAction::DiscardCard | EffectAction::MoveCards => self.execute_move_cards(effect),
            EffectAction::GainResource => self.execute_gain_resource(effect.resource.as_deref().unwrap_or(""), effect.resource_icon_count.unwrap_or(effect.count_or(1)), effect.target_name(), effect.duration.as_deref(), effect.card_type.as_deref(), effect.group_name(), effect.per_unit.unwrap_or(false), effect.per_unit_count.unwrap_or(1), effect.per_unit_type.as_deref(), &effect.heart_colors, effect.resource_icon_count, effect.heart_selection.unwrap_or(false), effect.sign.as_deref()),
            EffectAction::ChangeState => self.execute_change_state(effect.state_change.as_deref().unwrap_or(""), effect.target_name(), effect.count_or(0), effect.max.unwrap_or(false), effect.card_type.as_deref(), effect.cost_limit, effect.optional.unwrap_or(false), effect.group_name(), effect.self_cost.unwrap_or(false), effect.source.as_deref(), effect.destination.as_deref(), effect.cost_limit_operator.clone()),
            EffectAction::ModifyScore => self.execute_modify_score(effect.operation.as_deref().unwrap_or("add"), effect.value.unwrap_or(0), effect.target_name(), effect.duration.as_deref(), effect.card_type.as_deref(), effect.group_name(), effect.per_unit.unwrap_or(false), effect.per_unit_count.unwrap_or(1), effect.per_unit_type.as_deref(), effect.effect_constraint.as_deref(), effect.self_target.unwrap_or(false), &effect.heart_colors),
            EffectAction::ModifyRequiredHearts => self.execute_modify_required_hearts(effect.operation.as_deref().unwrap_or("decrease"), effect.value.or(effect.count).unwrap_or(0), effect.heart_color_or("heart00"), effect.target_name(), effect.per_unit.unwrap_or(false), effect.per_unit_count.unwrap_or(1), effect.group_name(), effect.timing_condition.as_deref(), effect.location.as_deref()),
            EffectAction::SetCost => self.execute_set_cost(effect.value.unwrap_or(0), effect.target_name(), effect.card_type.as_deref()),
            EffectAction::SetBladeType => self.execute_set_blade_type(effect.blade_type.as_deref(), effect.target_name(), effect.duration.as_deref()),
            EffectAction::SetHeartType => self.execute_set_heart_type(effect.heart_type.as_deref().or(effect.heart_colors.first().map(|s| s.as_str())), effect.target_name(), effect.count_or(1) as i32),
            EffectAction::ActivateAbility => self.execute_activate_ability(effect.ability_text.as_deref().unwrap_or(""), effect.target_trigger.as_deref(), effect.count),
            EffectAction::InvalidateAbility => self.execute_invalidate_ability(),
            EffectAction::GainAbility => self.execute_gain_ability(effect.ability_gain.as_deref().filter(|s| !s.is_empty()).or_else(|| if effect.text.is_empty() { None } else { Some(effect.text.as_str()) }).unwrap_or(""), effect.target_name(), effect.duration.as_deref()),
            EffectAction::PlayBatonTouch => self.execute_play_baton_touch(effect.count_or(1), effect.target_name()),
            EffectAction::Reveal => {
                if effect.multiple_targets.unwrap_or(false) && effect.source.as_deref() == Some("deck_top") {
                    let chosen = self.game_state.ability_queue.current_entry().and_then(|e| e.conditional_choice.clone()).or_else(|| effect.card_type.clone());
                    return self.execute_reveal_until_target(effect.target_name(), chosen.as_deref());
                }
                self.execute_reveal(effect.source_or("hand"), effect.count_or(1), effect.target_name(), effect.card_type.as_deref(), &effect.heart_colors)
            }
            EffectAction::Select => self.execute_select(if effect.card_type.as_deref() == Some("member_card") { "stage" } else { effect.source_or("hand") }, effect.count_or(1), effect.target_name(), effect.card_type.as_deref(), effect.distinct.as_deref(), &effect.heart_colors, effect.or_card_types.clone(), effect.exclude_selected.unwrap_or(false)),
            EffectAction::LookAt => self.execute_look_at(effect.count_or(1), effect.target_name(), effect.source_or("deck")),
            EffectAction::ModifyRequiredHeartsGlobal => self.execute_modify_required_hearts_global(effect.operation.as_deref().unwrap_or("increase"), effect.value.unwrap_or(1), effect.heart_color_or("heart00"), effect.target_name()),
            EffectAction::ModifyYellCount => self.execute_modify_yell_count(effect.operation.as_deref().unwrap_or("subtract"), effect.count_or(0)),
            EffectAction::PlaceEnergyUnderMember => self.execute_place_energy_under_member(effect.energy_count.unwrap_or(1), effect.target_name(), effect.position.as_ref(), effect.optional.unwrap_or(false)),
            EffectAction::ActivationCost => self.execute_activation_cost(effect.operation.as_deref().unwrap_or("increase"), effect.value.unwrap_or(0), effect.target_name(), effect.duration.as_deref()),
            EffectAction::PositionChange => self.execute_position_change(effect, effect.position.clone(), effect.target_name(), effect.target_member.as_deref().unwrap_or("this_member")),
            EffectAction::FormationChange => self.execute_formation_change(effect),
            EffectAction::Appear => self.execute_appear(effect.source_or(""), effect.destination.as_deref().unwrap_or("stage"), effect.count_or(1), effect.target_name(), effect.card_type.as_deref()),
            EffectAction::Choice => self.execute_choice(effect.choice_options.as_ref(), effect.choice_type.as_deref(), effect.options.as_ref()),
            EffectAction::PayEnergy => self.execute_pay_energy(effect.count_or(0), effect.target_name()),
            EffectAction::SetCardIdentity => {
                if effect.all_regions.unwrap_or(false) {
                    self.execute_set_card_identity_all_regions(effect.identities.as_ref(), effect.target_name())
                } else {
                    self.execute_set_card_identity(&effect.identities.clone().unwrap_or_default())
                }
            },
            EffectAction::RepeatProcedure => self.execute_repeat_procedure(effect, effect.repeat_limit.unwrap_or(1)),
            EffectAction::DiscardUntilCount => self.execute_discard_until_count(effect.target_count.unwrap_or(0), effect.target_name()),
            EffectAction::Restriction => self.execute_restriction(effect.restriction_type.as_deref(), effect.restricted_destination.as_deref()),
            EffectAction::ReYell => self.execute_re_yell(effect.lose_blade_hearts.unwrap_or(false), effect.target_name()),
            EffectAction::ActivationRestriction => self.execute_activation_restriction(effect.target_name()),
            EffectAction::ChooseRequiredHearts => self.execute_choose_required_hearts(),
            EffectAction::ModifyLimit => self.execute_modify_limit(effect.operation.as_deref().unwrap_or("decrease"), effect.count_or(0)),
            EffectAction::SetBladeCount => self.execute_set_blade_count(effect.value.unwrap_or(effect.count_or(0)), effect.target_name()),
            EffectAction::Custom => self.execute_custom(effect, &action_str),
            EffectAction::DoNothing => Ok(()),
            EffectAction::SetRequiredHearts => self.execute_set_required_hearts(&effect.heart_colors, effect.target_name()),
            EffectAction::SetScore => self.execute_set_score(effect.value.unwrap_or(0), effect.target_name()),
            EffectAction::SpecifyHeartColor => self.execute_specify_heart_color(effect.choice.unwrap_or(false), effect.target_name()),
            EffectAction::ModifyRequiredHeartsSuccess => self.execute_modify_required_hearts_success(effect.operation.as_deref().unwrap_or("increase"), effect.value.unwrap_or(0), effect.target_name(), effect.card_type.as_deref()),
            EffectAction::SetCostToUse => self.execute_set_cost_to_use(effect.value.unwrap_or(0)),
            EffectAction::AllBladeTiming => self.execute_all_blade_timing(effect.timing.as_deref().unwrap_or("check_required_hearts"), effect.treat_as.as_deref().unwrap_or("any_heart_color")),
            EffectAction::SetCardIdentityAllRegions => self.execute_set_card_identity_all_regions(effect.identities.as_ref(), effect.target_name()),
            EffectAction::Shuffle => self.execute_shuffle(effect.target_name(), effect.source_or("deck")),
            EffectAction::RevealPerGroup => self.execute_reveal_per_group(effect.source_or("hand"), effect.count_or(1), effect.target_name()),
            EffectAction::ConditionalOnResult => self.execute_conditional_on_result(effect),
            EffectAction::ConditionalOnOptional => self.execute_conditional_on_optional(effect),
            EffectAction::ModifyCost => self.execute_modify_cost(effect.operation.as_deref().unwrap_or("add"), effect.value.unwrap_or(0), effect.target_name(), effect.card_type.as_deref()),
            EffectAction::RevealUntilLiveCard => self.execute_reveal_until_live_card(effect.target_name()),
        }
    }

// ===== LEAF EFFECTS (all data directly from AbilityEffect params) =====

    fn execute_custom(&mut self, effect: &AbilityEffect, action_str: &str) -> Result<(), String> {
        // Handle "custom" actions that could not be parsed into a standard action type.
        // Some custom actions have enough info to re-route to a known handler.

        // 1) Deck reordering: placement_order=any_order → route as move_cards looked_at→deck_top
        if effect.placement_order.as_deref() == Some("any_order") {
            let mut routed = effect.clone();
            routed.action = "move_cards".into();
            if routed.source.is_none() { routed.source = Some("looked_at".into()); }
            if routed.destination.is_none() { routed.destination = Some("deck_top".into()); }
            return self.execute_move_cards(&routed);
        }

        // 2) Limit modification: "枚数を1枚増やす" → modify_limit
        if effect.count == Some(1) && action_str.contains("枚数を") && (action_str.contains("増やす") || action_str.contains("増やす")) {
            return self.execute_modify_limit("increase", 1);
        }

        // 3) Complex conditional scoring / gain_ability: has duration or grants continuous effect
        if effect.duration.is_some() || action_str.contains("常時") || action_str.contains("ライブ終了まで") {
            let text = if effect.text.is_empty() { action_str } else { &effect.text };
            return self.execute_gain_ability(text, effect.target.as_deref().unwrap_or("self"), effect.duration.as_deref());
        }

        eprintln!("Unhandled custom action: {}", action_str);
        Ok(())
    }

    fn execute_draw(&mut self, count: u32, target: &str, source: &str, destination: &str, card_type: Option<&str>, per_unit: bool, per_unit_count: u32, per_unit_type: Option<&str>) -> Result<(), String> {
        let card_db = self.game_state.card_database.clone();

        if target == "both" {
            let card_db1 = card_db.clone();
            let card_db2 = card_db.clone();
            { let p1 = &mut self.game_state.player1; Self::draw_cards_for_player(p1, count, source, destination, card_type, &card_db1)?; }
            { let p2 = &mut self.game_state.player2; Self::draw_cards_for_player(p2, count, source, destination, card_type, &card_db2)?; }
            return Ok(());
        }

        let player = self.game_state.resolve_target_player_mut(target);

        let final_count = if per_unit {
            let multiplier = match per_unit_type {
                Some("member") | Some("人") => player.stage.stage.iter().filter(|&&c| c != -1).count() as u32,
                Some("energy") => player.energy_zone.cards.len() as u32,
                Some("hand") => player.hand.cards.len() as u32,
                _ => 1,
            };
            count * multiplier * per_unit_count
        } else { count };

        match source {
            "deck" | "deck_top" => { Self::draw_cards_for_player(player, final_count, source, destination, card_type, &card_db)?; }
            "discard" => {
                for _ in 0..final_count {
                    if let Some(card) = player.waitroom.cards.pop() {
                        player.hand.add_card(card);
                    } else { break; }
                }
            }
            _ => { eprintln!("Draw from source '{}' not yet implemented", source); }
        }
        Ok(())
    }

    fn draw_cards_for_player(player: &mut crate::player::Player, count: u32, _source: &str, destination: &str, card_type_filter: Option<&str>, card_db: &crate::card::CardDatabase) -> Result<(), String> {
        let mut drawn = 0;
        while drawn < count {
            if let Some(card) = player.main_deck.draw() {
                let matches_type = util::card_matches_type(card_db, card, card_type_filter);
                if matches_type {
                    match destination {
                        "hand" | "discard" | "deck_top" | "deck_bottom" | "deck" |
                        "energy_zone" | "live_card_zone" | "success_live_zone" | "stage" => {
                            util::place_card_in_zone(player, card, destination, None, false, 1);
                        }
                        _ => { player.hand.add_card(card); }
                    }
                    drawn += 1;
                } else { player.main_deck.cards.push(card); }
            } else { break; }
        }
        Ok(())
    }

    fn execute_draw_until_count(&mut self, target_count: u32, target: &str, destination: &str) -> Result<(), String> {
        let player = self.game_state.resolve_target_player_mut(target);
        let current_count = match destination {
            "hand" => player.hand.len(),
            _ => { return Ok(()); }
        };
        let to_draw = (target_count as usize).saturating_sub(current_count);
        self.execute_draw(to_draw as u32, target, "deck", destination, None, false, 1, None)
    }

    fn execute_gain_resource(
        &mut self, resource: &str, count: u32, target: &str, duration: Option<&str>,
        card_type: Option<&str>, group_name: Option<&str>, per_unit: bool, per_unit_count: u32,
        per_unit_type: Option<&str>, heart_colors: &[String],
        _resource_icon_count: Option<u32>, heart_selection: bool, sign: Option<&str>,
    ) -> Result<(), String> {
        let resource = resource.to_string();
        let target = target.to_string();
        let duration = duration.map(|s| s.to_string());
        let card_type_filter = card_type.map(|s| s.to_string());
        let group_filter = group_name.map(|s| s.to_string());
        let per_unit_count_val = per_unit_count;
        let per_unit_type_str = per_unit_type.map(|s| s.to_string());
        let is_temporary = duration.is_some() && duration.as_deref() != Some("permanent");
        let activating_card_id = self.game_state.activating_card;
        let card_db = self.game_state.card_database.clone();

        // If heart_colors is non-empty and resource is heart, this is a choose-and-replace operation
        let single_fixed_heart: Option<String> = if (resource == "heart" || resource == "ハート") && (!heart_colors.is_empty() || heart_selection) {
            let colors = if heart_colors.is_empty() {
                vec!["heart01".into(), "heart02".into(), "heart03".into(),
                     "heart04".into(), "heart05".into(), "heart06".into()]
            } else { heart_colors.to_vec() };
            let mut unique_colors: Vec<String> = Vec::new();
            for c in colors {
                if !unique_colors.contains(&c) {
                    unique_colors.push(c);
                }
            }
            if unique_colors.len() == 1 && !heart_selection {
                Some(unique_colors[0].clone())
            } else {
                self.pending_choice = Some(Choice::SelectHeartColor {
                    count: count as usize,
                    options: unique_colors,
                    description: "Choose a heart color".to_string(),
                });
                return Ok(());
            }
        } else { None };

        let (blade_targets, heart_targets, heart_color_str, final_count) = {
            let player = self.game_state.resolve_target_player_mut(&target);

            let filter = util::CardFilter {
                card_type: card_type_filter.as_deref(),
                group: group_filter.as_deref(),
                ..util::CardFilter::default()
            };

            let final_count = if per_unit {
                let matching_count = match per_unit_type_str.as_deref() {
                    Some("stage") | Some("member") | Some("人") => util::count_matching(util::zone_cards(player, "stage"), &card_db, &filter, true),
                    Some("hand") | Some("card") | Some("枚") => util::count_matching(util::zone_cards(player, "hand"), &card_db, &filter, false),
                    Some("discard") => util::count_matching(&player.waitroom.cards, &card_db, &filter, false),
                    Some("live_card_zone") => util::count_matching(&player.live_card_zone.cards, &card_db, &filter, false),
                    _ => util::count_matching(util::zone_cards(player, "stage"), &card_db, &filter, true),
                };
                (matching_count / per_unit_count_val) * count
            } else { count };

            let has_blade_filter = card_type_filter.is_some() || group_filter.is_some();
            let blade_targets: Vec<i16> = if has_blade_filter {
                util::matching_ids(util::zone_cards(player, "stage"), &card_db, &filter, true)
            } else {
                vec![]
            };

            let heart_color_inner = single_fixed_heart.clone().or_else(|| heart_colors.first().map(|s| s.to_string()));
            let heart_targets: Vec<i16> = if resource == "heart" || resource == "ハート" {
                util::matching_ids(util::zone_cards(player, "stage"), &card_db, &filter, true)
            } else { vec![] };

            (blade_targets, heart_targets, heart_color_inner, final_count)
        };

        let mut effect_data: Option<serde_json::Value> = None;
        let is_negative = sign == Some("negative");

        if resource == "blade" || resource == "ブレード" {
            let blades_to_add = if is_negative { -(final_count as i32) } else { final_count as i32 };
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
            let color = crate::zones::parse_heart_color(heart_color_str.as_deref().unwrap_or("heart00"));
            let heart_to_add = if is_negative { -(final_count as i32) } else { final_count as i32 };
            if heart_targets.is_empty() {
                if let Some(card_id) = activating_card_id {
                    self.game_state.add_heart_modifier(card_id, color, heart_to_add);
                }
            } else {
                for &card_id in &heart_targets {
                    self.game_state.add_heart_modifier(card_id, color, heart_to_add);
                }
            }
        }

        if is_temporary {
            self.game_state.temporary_effects.push(crate::game_state::TemporaryEffect {
                effect_type: format!("gain_{}", resource),
                duration: match duration.as_deref() { Some("this_turn") => crate::game_state::Duration::ThisTurn, Some("live_end") => crate::game_state::Duration::LiveEnd, Some("as_long_as") => crate::game_state::Duration::AsLongAs, _ => crate::game_state::Duration::ThisLive },
                created_turn: self.game_state.turn_number,
                created_phase: self.game_state.current_phase.clone(),
                target_player_id: target.clone(),
                description: format!("Gain {} {}", final_count, resource),
                creation_order: 0, effect_data,
            });
        }

        Ok(())
    }

    fn execute_change_state(
        &mut self, state_change: &str, target: &str, count: u32, max: bool, card_type: Option<&str>,
        cost_limit: Option<u32>, optional: bool, group_name: Option<&str>, self_cost: bool,
        source: Option<&str>, destination: Option<&str>,
        cost_limit_operator: Option<String>,
    ) -> Result<(), String> {
        let state_change = state_change.to_string();
        let target = target.to_string();
        let card_type_filter = card_type.map(|s| s.to_string());
        let group_filter = group_name.map(|s| s.to_string());

        if optional {
            self.pending_choice = Some(Choice::SelectTarget {
                target: "pay_optional_cost:skip_optional_cost".to_string(),
                description: format!("Change state to {} (pay optional cost)?", state_change),
            });
            if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
                entry.choice_card_no = Some("change_state".to_string());
            }
            return Ok(());
        }

        // Draw from energy deck and place in energy zone with state (e.g. wait)
        if source == Some("deck") && destination == Some("energy_zone") {
            let player = self.game_state.resolve_target_player_mut(&target);
            for _ in 0..count {
                if let Some(energy_id) = player.energy_deck.draw() {
                    player.energy_zone.cards.push(energy_id);
                    if state_change == "wait" {
                        // Card is placed in wait (not counted as active)
                    } else if state_change == "active" {
                        player.energy_zone.active_energy_count += 1;
                    }
                }
            }
            return Ok(());
        }

        // Member card state change — operate on stage
        let is_member_op = card_type_filter.as_deref() == Some("member_card") || self_cost;

        if is_member_op {
            let card_db = self.game_state.card_database.clone();
            let player = self.game_state.resolve_target_player_mut(&target);

            let filter = util::CardFilter {
                card_type: card_type_filter.as_deref(),
                group: group_filter.as_deref(),
                cost_limit,
                ..util::CardFilter::default()
            };
            let mut candidates: Vec<(usize, i16)> = Vec::new();
            for (i, slot_id) in player.stage.stage.iter().enumerate() {
                if *slot_id == -1 { continue; }
                if filter.matches(&card_db, *slot_id, false) {
                    candidates.push((i, *slot_id));
                }
            }

            if candidates.is_empty() {
                return Err("No matching members on stage to change state".to_string());
            }

            // Count how many are in wait state before changing (for wait→active tracking)
            let wait_before_count = candidates.iter()
                .filter(|(_, card_id)| {
                    let o = self.game_state.get_orientation_modifier(*card_id);
                    // None = active (no modifier), Some("wait") = wait
                    o.map_or(false, |o| o == "wait")
                })
                .count();

            // count=0 means "change all matching" (no limit)
            let is_change_all = count == 0;

            if !is_change_all && candidates.len() > count as usize {
                self.pending_choice = Some(Choice::SelectCard {
                    zone: "stage".to_string(),
                    card_type: card_type_filter.clone(),
                    count: count as usize,
                    description: format!("Select {} member(s) to change state", count),
                    allow_skip: false,
                    cost_limit,
                    cost_limit_operator: cost_limit_operator.clone(),
                    group: group_filter.clone(),
                    characters: None,
                });
                self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                return Ok(());
            }

            let change_count = if is_change_all { candidates.len() } else { count as usize };
            for (_, card_id) in candidates.iter().take(change_count) {
                self.game_state.add_orientation_modifier(*card_id, &state_change);
            }

            // Track how many members were changed from wait→active
            if state_change == "active" {
                self.game_state.last_state_change_wait_to_active_count = wait_before_count as u32;
            }

            return Ok(());
        }

        // Energy card state change (original behavior)
        let card_db = self.game_state.card_database.clone();
        let (wait_cards, deactivate_count) = {
            let player = self.game_state.resolve_target_player_mut(&target);

            let filter = util::CardFilter {
                card_type: card_type_filter.as_deref(),
                group: group_filter.as_deref(),
                cost_limit,
                ..util::CardFilter::default()
            };
            let valid_indices = util::matching_indices(&player.energy_zone.cards, &card_db, &filter, false);

            // When max=true, cap count to available cards (no error, no prompt)
            let effective_count = if max {
                let available = match state_change.as_str() {
                    "active" | "アクティブ" => player.energy_zone.cards.len().saturating_sub(player.energy_zone.active_energy_count),
                    _ => player.energy_zone.active_energy_count,
                };
                let capped = (count as usize).min(available) as u32;
                eprintln!("[ENERGY] max=true: count={} available={} effective={}", count, available, capped);
                capped
            } else {
                eprintln!("[ENERGY] max=false: count={} effectve={}", count, count);
                count
            };

            if valid_indices.len() < effective_count as usize {
                return Err(format!("Not enough energy cards to deactivate: need {}, have {}", effective_count, valid_indices.len()));
            }

            if !max && valid_indices.len() > effective_count as usize {
                self.pending_choice = Some(Choice::SelectCard {
                    zone: "energy_zone".to_string(), card_type: card_type_filter.clone(),
                    count: effective_count as usize,
                    description: format!("Select {} energy card(s) to deactivate (set to wait)", effective_count),
                    allow_skip: false,
                    cost_limit: None, cost_limit_operator: None, group: None, characters: None,
                });
                self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                return Ok(());
            }

            let wait_cards: Vec<i16> = valid_indices.iter().take(effective_count as usize).filter_map(|i| {
                if *i < player.energy_zone.cards.len() { Some(player.energy_zone.cards[*i]) } else { None }
            }).collect();

            (wait_cards, effective_count)
        };

        eprintln!("[ENERGY] Building active_cards: deactivate_count={} max={}", deactivate_count, max);
        // Build active_cards separately (cannot be inside the closure due to borrow conflict)
        eprintln!("[ENERGY] Building active_cards: deactivate_count={} max={}", deactivate_count, max);
        let active_cards: Vec<i16> = if state_change == "active" || state_change == "アクティブ" {
            let player = self.game_state.resolve_target_player(&target);
            let mut result = Vec::new();
            let mut active_count = 0u32;
            for i in 0..player.energy_zone.cards.len() {
                if active_count >= deactivate_count { break; }
                if let Some(&card_id) = player.energy_zone.cards.get(i) {
                    let matches_type = card_type_filter.as_deref().map_or(true, |ct| util::card_matches_type(&card_db, card_id, Some(ct)));
                    let matches_grp = group_filter.as_deref().map_or(true, |gf| util::card_matches_group_str(&card_db, card_id, Some(gf)));
                    if matches_type && matches_grp {
                        result.push(card_id);
                        active_count += 1;
                    }
                }
            }
            result
        } else { vec![] };

        match state_change.as_str() {
            "wait" | "ウェイト" => {
                for card_id in &wait_cards {
                    self.game_state.add_orientation_modifier(*card_id, "wait");
                }
                for _ in 0..deactivate_count {
                    let player = self.game_state.resolve_target_player_mut(target.as_str());
                    player.energy_zone.active_energy_count = player.energy_zone.active_energy_count.saturating_sub(1);
                }
            }
            "active" | "アクティブ" => {
                for card_id in &active_cards {
                    self.game_state.add_orientation_modifier(*card_id, "active");
                }
                let player = self.game_state.resolve_target_player_mut(target.as_str());
                player.energy_zone.active_energy_count += active_cards.len();
            }
            _ => {}
        }
        Ok(())
    }

    fn execute_modify_score(
        &mut self, operation: &str, value: u32, target: &str, duration: Option<&str>,
        card_type: Option<&str>, group_name: Option<&str>, per_unit: bool, per_unit_count: u32,
        per_unit_type: Option<&str>, effect_constraint: Option<&str>, self_target: bool,
        heart_colors: &[String],
    ) -> Result<(), String> {
        let operation = operation.to_string();
        let target = target.to_string();
        let duration = duration.map(|s| s.to_string());
        let card_type_filter = card_type.map(|s| s.to_string());
        let group_filter = group_name.map(|s| s.to_string());
        let per_unit_count_val = per_unit_count;
        let per_unit_type_str = per_unit_type.map(|s| s.to_string());
        let effect_constraint = effect_constraint.map(|s| s.to_string());
        let card_db = self.game_state.card_database.clone();

        let (live_card_ids, final_value) = {
            let player = self.game_state.resolve_target_player_mut(&target);

            let filter = util::CardFilter {
                card_type: card_type_filter.as_deref(),
                group: group_filter.as_deref(),
                ..util::CardFilter::default()
            };

            let final_value = if per_unit {
                let zone = match per_unit_type_str.as_deref() {
                    Some("hand") => "hand",
                    Some("stage") | Some("member") => "stage",
                    _ => "",
                };
                let matching_count = if zone.is_empty() { 1u32 } else {
                    if !heart_colors.is_empty() {
                        let cards = util::zone_cards(player, zone).to_vec();
                        let mut count = 0u32;
                        for &cid in &cards {
                            if util::card_matches_type(&card_db, cid, filter.card_type)
                                && util::card_matches_group_str(&card_db, cid, filter.group)
                                && util::card_matches_heart_colors(&card_db, cid, heart_colors)
                            {
                                count += 1;
                            }
                        }
                        count
                    } else {
                        util::count_matching(util::zone_cards(player, zone), &card_db, &filter, zone == "stage") as u32
                    }
                };
                value * matching_count * per_unit_count_val
            } else { value };

            let candidate_ids: Vec<i16> = match card_type_filter.as_deref() {
                Some("member_card") => util::matching_ids(util::zone_cards(player, "stage"), &card_db, &filter, true),
                _ => player.live_card_zone.cards.iter().copied().collect(),
            };
            let target_card_ids: Vec<(i16, i32)> = candidate_ids.iter()
                .filter(|&&card_id| {
                    if !filter.matches(&card_db, card_id, false) { return false; }
                    if self_target {
                        if let Some(activating_id) = self.activating_card_id {
                            if card_id != activating_id { return false; }
                        }
                    }
                    true
                })
                .map(|&card_id| {
                    let delta = match operation.as_str() {
                        "add" => final_value as i32,
                        "remove" => -(final_value as i32),
                        "set" => final_value as i32,
                        _ => 0i32,
                    };
                    (card_id, delta)
                }).collect();

            eprintln!("[SCORE] ids={:?} final_value={} self_target={} target={} activating_id={:?}",
                target_card_ids.iter().map(|(id, d)| format!("id={} delta={}", id, d)).collect::<Vec<_>>(),
                final_value, self_target, target, self.activating_card_id);
            (target_card_ids, final_value)
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
            if operation == "set" { self.game_state.set_score_modifier(*card_id, *delta); }
            else { self.game_state.add_score_modifier(*card_id, *delta); }
            count_applied += 1;
        }

        if let Some(duration_str) = &duration {
            if duration_str != "permanent" {
                let duration_enum = match duration_str.as_str() {
                    "this_turn" => crate::game_state::Duration::ThisTurn,
                    "this_live" => crate::game_state::Duration::ThisLive,
                    "live_end" => crate::game_state::Duration::LiveEnd,
                    "as_long_as" => crate::game_state::Duration::AsLongAs,
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

    fn execute_modify_required_hearts(&mut self, operation: &str, mut value: u32, heart_color: &str, target: &str, per_unit: bool, per_unit_count: u32, group_name: Option<&str>, timing_condition: Option<&str>, _location: Option<&str>) -> Result<(), String> {
        if per_unit {
            let card_db = &self.game_state.card_database;
            let player = self.game_state.resolve_target_player(target);
            let stage_cards: Vec<i16> = player.stage.stage.iter().filter(|&&id| id != -1).copied().collect();
            let mut count = 0u32;
            for &card_id in &stage_cards {
                if let Some(g) = group_name {
                    if !super::util::card_matches_group_str(card_db, card_id, Some(g)) {
                        continue;
                    }
                }
                if let Some(tc) = timing_condition {
                    match tc {
                        "appeared_or_moved_this_turn" => {
                            let moved = self.game_state.has_card_moved_this_turn(card_id);
                            let appeared = self.game_state.has_card_appeared_this_turn(card_id);
                            if !moved && !appeared { continue; }
                        }
                        _ => {}
                    }
                }
                count += 1;
            }
            value = count * per_unit_count;
            eprintln!("[MODIFY_HEARTS] per_unit count={} value={} group={:?} timing={:?}", count, value, group_name, timing_condition);
        }
        let color = crate::zones::parse_heart_color(heart_color);
        let card_ids: Vec<i16> = {
            let player = self.game_state.resolve_target_player_mut(target);
            player.live_card_zone.cards.to_vec()
        };
        for card_id in card_ids {
            match operation {
                "decrease" => { self.game_state.add_need_heart_modifier(card_id, color, -(value as i32)); }
                "increase" => { self.game_state.add_need_heart_modifier(card_id, color, value as i32); }
                "set" => { self.game_state.set_need_heart_modifier(card_id, color, value as i32); }
                _ => return Err(format!("Unknown operation: {}", operation)),
            }
        }
        Ok(())
    }

    fn execute_set_cost(&mut self, value: u32, target: &str, card_type: Option<&str>) -> Result<(), String> {
        let player = self.game_state.resolve_target_player_mut(target);
        let card_ids: Vec<i16> = if let Some("live_card") = card_type {
            player.live_card_zone.cards.iter().copied().collect()
        } else if let Some("member_card") = card_type {
            player.stage.stage.iter().filter(|&&id| id != -1).copied().collect()
        } else { player.hand.cards.iter().copied().collect() };
        for card_id in card_ids { self.game_state.set_cost_modifier(card_id, value as i32); }
        Ok(())
    }

    fn execute_set_blade_type(&mut self, blade_type: Option<&str>, target: &str, duration: Option<&str>) -> Result<(), String> {
        let current_turn = self.game_state.turn_number;
        let current_phase = self.game_state.current_phase.clone();
        let effect_duration = duration.map(|s| s.to_string());
        let card_db = self.game_state.card_database.clone();
        let stage_card_ids: Vec<(i16, String)> = {
            let player = self.game_state.resolve_target_player(target);
            (0..3).filter_map(|i| {
                let id = player.stage.stage[i];
                if id == -1 { None } else { Some((id, player.id.clone())) }
            }).collect()
        };
        for (card_id, pid) in stage_card_ids {
            let temp_effect = crate::game_state::TemporaryEffect {
                effect_type: format!("set_blade_type:{}", blade_type.unwrap_or("")),
                duration: effect_duration.as_deref().map(|d| match d {
                    "live_end" => crate::game_state::Duration::LiveEnd,
                    "this_turn" => crate::game_state::Duration::ThisTurn,
                    "this_live" => crate::game_state::Duration::ThisLive,
                    "permanent" => crate::game_state::Duration::Permanent,
                    "as_long_as" => crate::game_state::Duration::AsLongAs,
                    _ => crate::game_state::Duration::ThisLive,
                }).unwrap_or(crate::game_state::Duration::ThisLive),
                created_turn: current_turn, created_phase: current_phase.clone(),
                target_player_id: pid,
                description: format!("Set blade type to {} for {}", blade_type.unwrap_or(""), card_db.get_card(card_id).map(|c| c.name.as_str()).unwrap_or("unknown")),
                creation_order: 0, effect_data: None,
            };
            self.game_state.temporary_effects.push(temp_effect);
        }
        Ok(())
    }

    fn execute_set_heart_type(&mut self, heart_type: Option<&str>, target: &str, count: i32) -> Result<(), String> {
        let heart_type = heart_type.unwrap_or("heart00");
        let player = self.game_state.resolve_target_player_mut(target);
        let mut card_ids_to_modify: Vec<i16> = Vec::new();
        for index in 0..3 {
            let card_id = player.stage.stage[index];
            if card_id != -1 { card_ids_to_modify.push(card_id); }
        }
        let color = crate::zones::parse_heart_color(heart_type);
        for card_id in card_ids_to_modify { self.game_state.add_heart_modifier(card_id, color, count); }
        Ok(())
    }

    fn execute_activate_ability(&mut self, ability_text: &str, target_trigger: Option<&str>, _count: Option<u32>) -> Result<(), String> {
        if let Some(card_id) = self.game_state.activating_card {
            let mut text = ability_text.to_string();
            if let Some(trigger) = target_trigger {
                text = format!("{}_trigger:{}", text, trigger);
            }
            self.game_state.gained_abilities.entry(card_id).or_default().push(text);
        }
        Ok(())
    }

    fn execute_invalidate_ability(&mut self) -> Result<(), String> {
        if let Some(card_id) = self.game_state.activating_card {
            self.game_state.negated_abilities.insert(card_id);
        }
        Ok(())
    }

    fn execute_gain_ability(&mut self, ability_text: &str, target: &str, duration: Option<&str>) -> Result<(), String> {
        if let Some(card_id) = self.game_state.activating_card {
            self.game_state.gained_abilities.entry(card_id).or_default().push(ability_text.to_string());
        }
        let temp_effect = crate::game_state::TemporaryEffect {
            effect_type: format!("gain_ability:{}", ability_text),
            duration: match duration { Some("this_turn") => crate::game_state::Duration::ThisTurn, Some("live_end") => crate::game_state::Duration::LiveEnd, Some("as_long_as") => crate::game_state::Duration::AsLongAs, _ => crate::game_state::Duration::ThisLive },
            created_turn: self.game_state.turn_number, created_phase: self.game_state.current_phase.clone(),
            target_player_id: target.to_string(),
            description: format!("Gained ability: {}", ability_text),
            creation_order: 0, effect_data: None,
        };
        self.game_state.temporary_effects.push(temp_effect);
        Ok(())
    }

    fn execute_play_baton_touch(&mut self, count: u32, target: &str) -> Result<(), String> {
        eprintln!("play_baton_touch: count={}, target={}", count, target);
        self.game_state.prohibition_effects.push(format!("baton_touch_allowed:{}", count));
        Ok(())
    }



    fn execute_modify_required_hearts_global(&mut self, operation: &str, value: u32, heart_color: &str, target: &str) -> Result<(), String> {
        let color = crate::zones::parse_heart_color(heart_color);
        let card_ids: Vec<i16> = {
            let player = self.game_state.resolve_target_player_mut(target);
            player.live_card_zone.cards.to_vec()
        };
        for card_id in card_ids {
            let modifier_value = match operation { "increase" => value as i32, "decrease" => -(value as i32), _ => return Err(format!("Unknown operation: {}", operation)) };
            self.game_state.add_need_heart_modifier(card_id, color, modifier_value);
        }
        Ok(())
    }

    fn execute_modify_yell_count(&mut self, operation: &str, count: u32) -> Result<(), String> {
        match operation {
            "add" => { self.game_state.cheer_checks_required += count; }
            "subtract" => { self.game_state.cheer_checks_required = self.game_state.cheer_checks_required.saturating_sub(count); }
            "set" => { self.game_state.cheer_checks_required = count; }
            _ => return Err(format!("Unknown operation: {}", operation)),
        }
        Ok(())
    }

    pub fn execute_place_energy_under_member(&mut self, count: u32, target: &str, position: Option<&PositionInfo>, optional: bool) -> Result<(), String> {
        if optional {
            let is_activation = self.current_ability.as_ref()
                .and_then(|a| a.triggers.as_ref())
                .map_or(false, |t| t == crate::triggers::ACTIVATION);
            if !is_activation {
                self.pending_choice = Some(Choice::SelectTarget {
                    target: "pay_optional_cost:skip_optional_cost".to_string(),
                    description: "Place energy under member? (pay or skip)".to_string(),
                });
                if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
                    entry.choice_card_no = Some("optional_cost".to_string());
                }
                return Ok(());
            }
        }
        let player = self.game_state.resolve_target_player_mut(target);
        let mut energy_cards = Vec::new();
        for _ in 0..count {
            if let Some(energy_card) = player.energy_zone.cards.pop() { energy_cards.push(energy_card); }
            else { break; }
        }
        if energy_cards.is_empty() { return Ok(()); }
        let target_index = match position.and_then(|p| p.get_position()) {
            Some("center") | Some("中央") => 1,
            Some("left") | Some("左側") => 0,
            Some("right") | Some("右側") => 2,
            None => {
                if player.stage.stage[1] != -1 { 1 }
                else if player.stage.stage[0] != -1 { 0 }
                else if player.stage.stage[2] != -1 { 2 }
                else { for card in energy_cards { player.energy_deck.cards.push(card); } return Ok(()); }
            }
            _ => 1,
        };
        if player.stage.stage[target_index] == -1 {
            // Rule 10.5.4: Energy without a member goes to energy deck
            for card in energy_cards { player.energy_deck.cards.push(card); }
            return Ok(());
        }
        // Rule 10.5.3: Energy placed under member — track it for recycling
        let area = match target_index {
            0 => crate::zones::MemberArea::LeftSide,
            1 => crate::zones::MemberArea::Center,
            _ => crate::zones::MemberArea::RightSide,
        };
        for card in energy_cards {
            player.stage.place_under_card(area, card);
        }
        Ok(())
    }

    fn execute_activation_cost(&mut self, operation: &str, value: u32, target: &str, duration: Option<&str>) -> Result<(), String> {
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
                    "as_long_as" => crate::game_state::Duration::AsLongAs,
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

    fn execute_position_change(&mut self, effect: &AbilityEffect, position: Option<PositionInfo>, target: &str, target_member: &str) -> Result<(), String> {
        // Check source_position from effect (new parser field), fall back to position param
        let source_pos = effect.source_position.as_deref()
            .or_else(|| position.as_ref().and_then(|p| p.get_position()));
        let position_str = source_pos.unwrap_or("");

        // Handle "both" target: opponent first (choice), then self (choice via pending).
        if target == "both" {
            let mut opp_effect = effect.clone();
            opp_effect.target = Some("opponent".to_string());
            self.execute_position_change(&opp_effect, position.clone(), "opponent", target_member)?;
            if self.pending_choice.is_some() {
                let mut self_effect = effect.clone();
                self_effect.target = Some("self".to_string());
                self.game_state.pending_sequential_actions = Some(vec![self_effect]);
            } else {
                let mut self_effect = effect.clone();
                self_effect.target = Some("self".to_string());
                self.execute_position_change(&self_effect, position.clone(), "self", target_member)?;
            }
            return Ok(());
        }

        if target_member == "this_member" {
            if !position_str.is_empty() {
                // Position is SOURCE ("member AT center"). Find that member on
                // the target's stage and create choice to pick destination.
                let player = self.game_state.resolve_target_player_mut(target);
                let pos_idx = match position_str {
                    "center" | "センターエリア" => 1,
                    "left_side" | "左サイドエリア" => 0,
                    "right_side" | "右サイドエリア" => 2,
                    _ => return Err(format!("Unknown position: {}", position_str)),
                };
                if player.stage.stage[pos_idx] == -1 {
                    return Ok(());  // no member at source → skip this side
                }
                if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
                    entry.choice_card_no = Some(format!("position_change:{}", target));
                }
                self.pending_choice = Some(Choice::SelectTarget {
                    target: "position|destination".to_string(),
                    description: "Choose destination for position change".to_string(),
                });
                return Ok(());
            }

            // No position specified: create choice for destination (move activating card).
            if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
                entry.choice_card_no = Some("position_change:self".to_string());
            }
            self.pending_choice = Some(Choice::SelectTarget {
                target: "position|destination".to_string(),
                description: "Choose destination for position change".to_string(),
            });
            return Ok(());
        }

        let card_database = self.game_state.card_database.clone();
        let player = self.game_state.resolve_target_player_mut(target);
        let target_index = match position_str {
            "center" | "センターエリア" => 1,
            "left_side" | "左サイドエリア" => 0,
            "right_side" | "右サイドエリア" => 2,
            _ => return Err(format!("Unknown position: {}", position_str)),
        };

        let current_index = player.stage.stage.iter().position(|&card_id| {
            if card_id == -1 { false }
            else { card_database.get_card(card_id).map(|c| c.card_no == target_member).unwrap_or(false) }
        });

        if let Some(current_idx) = current_index {
            let from_area = match current_idx { 0 => crate::zones::MemberArea::LeftSide, 1 => crate::zones::MemberArea::Center, _ => crate::zones::MemberArea::RightSide };
            let to_area = match target_index { 0 => crate::zones::MemberArea::LeftSide, 1 => crate::zones::MemberArea::Center, _ => crate::zones::MemberArea::RightSide };
            let (target_id, source_id) = (player.stage.stage[target_index], player.stage.stage[current_idx]);
            if let Ok(_) = player.stage.position_change(from_area, to_area) {
                let _ = player;
                self.game_state.record_card_movement(target_id);
                if source_id != -1 {
                    self.game_state.record_card_movement(source_id);
                }
            } else { return Err(format!("Failed to move member from {:?} to {:?}", from_area, to_area)); }
        } else { return Err(format!("Member not found: {}", target_member)); }
        self.game_state.position_change_occurred_this_turn = true;
        Ok(())
    }

    pub fn execute_position_change_with_destination(&mut self, effect: &AbilityEffect, destination: &str) -> Result<(), String> {
        let raw_target = effect.target.as_deref().unwrap_or("self");
        // "both" at resolution time means "self" (the ability controller resolves choices)
        let target = if raw_target == "both" { "self" } else { raw_target };
        let target_member = effect.target_member.as_deref().unwrap_or("this_member");
        // Check source_position first (new parser field), fall back to position
        let source_position = effect.source_position.as_deref()
            .or_else(|| effect.position.as_ref().and_then(|p| p.get_position()));

        // Reject destination if it matches exclude_position
        if let Some(ref exclude) = effect.exclude_position {
            let exclude_idx = match exclude.as_str() {
                "center" | "センターエリア" => 1,
                "left_side" | "左サイドエリア" => 0,
                "right_side" | "右サイドエリア" => 2,
                _ => -1,
            };
            let dest_idx = match destination {
                "center" | "センターエリア" => 1,
                "left_side" | "左サイドエリア" => 0,
                "right_side" | "右サイドエリア" => 2,
                _ => -1,
            };
            if exclude_idx == dest_idx {
                return Err(format!("Destination {} is excluded by exclude_position={}", destination, exclude));
            }
        }

        let target_index = match destination {
            "center" | "センターエリア" => 1,
            "left_side" | "左サイドエリア" => 0,
            "right_side" | "右サイドエリア" => 2,
            _ => return Err(format!("Unknown destination: {}", destination)),
        };

        if let Some(source) = source_position {
            // Source position specified: move member AT source TO destination.
            let player = self.game_state.resolve_target_player_mut(target);
            let source_idx = match source {
                "center" | "センターエリア" => 1,
                "left_side" | "左サイドエリア" => 0,
                "right_side" | "右サイドエリア" => 2,
                _ => return Err(format!("Unknown source position: {}", source)),
            };
            if player.stage.stage[source_idx] == -1 {
                return Ok(());  // no member at source, skip
            }
            if source_idx == target_index {
                return Ok(());  // same position, no move needed
            }
            let from_area2 = match source_idx { 0 => crate::zones::MemberArea::LeftSide, 1 => crate::zones::MemberArea::Center, _ => crate::zones::MemberArea::RightSide };
            let to_area2 = match target_index { 0 => crate::zones::MemberArea::LeftSide, 1 => crate::zones::MemberArea::Center, _ => crate::zones::MemberArea::RightSide };
            let (target_id2, source_id2) = (player.stage.stage[target_index], player.stage.stage[source_idx]);
            player.stage.position_change(from_area2, to_area2)?;
            let _ = player;
            self.game_state.record_card_movement(target_id2);
            if source_id2 != -1 {
                self.game_state.record_card_movement(source_id2);
            }
            self.game_state.position_change_occurred_this_turn = true;
            return Ok(());
        }

        if target_member == "this_member" {
            if let Some(activating_card_id) = self.activating_card_id {
                let player = self.game_state.resolve_target_player_mut(target);

                let current_index = player.stage.stage.iter().position(|&card_id| card_id == activating_card_id);

                if let Some(current_idx) = current_index {
                    if current_idx == target_index { return Ok(()); }
                    let from_area3 = match current_idx { 0 => crate::zones::MemberArea::LeftSide, 1 => crate::zones::MemberArea::Center, _ => crate::zones::MemberArea::RightSide };
                    let to_area3 = match target_index { 0 => crate::zones::MemberArea::LeftSide, 1 => crate::zones::MemberArea::Center, _ => crate::zones::MemberArea::RightSide };
                    let (target_id3, source_id3) = (player.stage.stage[target_index], player.stage.stage[current_idx]);
                    player.stage.position_change(from_area3, to_area3)?;
                    let _ = player;
                    self.game_state.record_card_movement(target_id3);
                    if source_id3 != -1 {
                        self.game_state.record_card_movement(source_id3);
                    }
                } else { return Err(format!("Activating card {} not found on stage", activating_card_id)); }
            } else { return Err("No activating card for position change".to_string()); }
        }
        self.game_state.position_change_occurred_this_turn = true;
        Ok(())
    }

    fn execute_formation_change(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let target = effect.target.as_deref().unwrap_or("self");
        let player = self.game_state.resolve_target_player_mut(target);

        // Collect non-empty areas with their card IDs and names
        let members: Vec<(usize, i16)> = (0..3)
            .filter(|&i| player.stage.stage[i] != -1)
            .map(|i| (i, player.stage.stage[i]))
            .collect();

        if members.is_empty() {
            return Ok(());
        }

        // Determine which positions are already taken by these members
        let _occupied: Vec<usize> = members.iter().map(|&(idx, _)| idx).collect();

        if members.len() == 1 {
            // Single member: just position change, not formation change
            return self.execute_position_change(effect, effect.position.clone(), target, "this_member");
        }

        // Execute as sequential single position changes
        for &(src_idx, _) in &members {
            let mut sub = AbilityEffect::default();
            sub.action = "position_change".to_string();
            sub.target = Some(target.to_string());
            sub.source_position = Some(match src_idx {
                0 => "left_side".to_string(),
                1 => "center".to_string(),
                2 => "right_side".to_string(),
                _ => unreachable!(),
            });
            let _ = self.execute_effect(&sub);
        }

        self.game_state.formation_change_occurred_this_turn = true;
        Ok(())
    }

    fn execute_appear(&mut self, source: &str, destination: &str, count: u32, target: &str, card_type: Option<&str>) -> Result<(), String> {
        let card_db = self.game_state.card_database.clone();
        let player = self.game_state.resolve_target_player_mut(target);

        match source {
            "deck" => {
                let mut appeared = 0;
                let mut cards_to_record: Vec<i16> = Vec::new();
                while appeared < count {
                    if let Some(card) = player.main_deck.draw() {
                        let matches_type = util::card_matches_type(&card_db, card, card_type);
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
                    let matches_type = util::card_matches_type(&card_db, *card, card_type);
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

    fn execute_choice(&mut self, choice_options: Option<&Vec<String>>, choice_type: Option<&str>, options: Option<&Vec<AbilityEffect>>) -> Result<(), String> {
        let options_json = options
            .and_then(|opts| serde_json::to_string(opts).ok())
            .or_else(|| choice_options.and_then(|opts| serde_json::to_string(opts).ok()));
        if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
            entry.choice_card_no = if options.is_some() {
                Some("choice".to_string())
            } else if choice_options.is_some() {
                Some("choice_string".to_string())
            } else {
                Some("choice".to_string())
            };
            entry.conditional_choice = options_json;
        }
        if let Some(effect_options) = options {
            let description = effect_options.iter()
                .map(|o| o.answers.as_ref()
                    .map(|a| a.join(", "))
                    .unwrap_or_else(|| o.text.clone()))
                .collect::<Vec<_>>()
                .join(" / ");
            self.pending_choice = Some(Choice::SelectTarget {
                target: "choice".to_string(),
                description,
            });
        } else if let Some(string_options) = choice_options {
            self.pending_choice = Some(Choice::SelectTarget {
                target: "choice_string".to_string(),
                description: format!("Choose one: {}", string_options.join(", ")),
            });
        } else if let Some(ct) = choice_type {
            self.pending_choice = Some(Choice::SelectTarget {
                target: "choice".to_string(),
                description: format!("Choose: {}", ct),
            });
        }
        Ok(())
    }

    fn execute_pay_energy(&mut self, count: u32, target: &str) -> Result<(), String> {
        let player = self.game_state.resolve_target_player_mut(target);
        if count > 0 { if let Err(e) = player.energy_zone.pay_energy(count as usize) { return Err(e); } }
        Ok(())
    }

    fn execute_set_card_identity(&mut self, identities: &[String]) -> Result<(), String> {
        eprintln!("set_card_identity: identities={:?}", identities);
        if !identities.is_empty() {
            self.game_state.prohibition_effects.push(format!("card_identity:{}", identities.join(",")));
        }
        Ok(())
    }

    fn execute_discard_until_count(&mut self, target_count: u32, target: &str) -> Result<(), String> {
        let player = self.game_state.resolve_target_player_mut(target);
        let current_count = player.hand.cards.len();
        if current_count <= target_count as usize { return Ok(()); }
        let cards_to_discard = current_count - target_count as usize;
        self.pending_choice = Some(Choice::SelectCard {
            zone: "hand".to_string(), card_type: None,
            count: cards_to_discard,
            description: format!("Discard {} cards from hand (target: {} cards in hand)", cards_to_discard, target_count),
            allow_skip: false,
            cost_limit: None, cost_limit_operator: None, group: None, characters: None,
        });
        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
        Ok(())
    }

    fn execute_restriction(&mut self, restriction_type: Option<&str>, restricted_destination: Option<&str>) -> Result<(), String> {
        eprintln!("restriction: type={:?}, destination={:?}", restriction_type, restricted_destination);
        self.game_state.prohibition_effects.push(format!("restriction:{}:{}", restriction_type.unwrap_or("unknown"), restricted_destination.unwrap_or("")));
        // Handle cannot_activate restrictions — store for checking during Active phase
        if restriction_type == Some("cannot_activate") || restriction_type == Some("cannot_activate_by_effect") {
            let target = self.game_state.activating_card.map(|_| "self".to_string()).unwrap_or("opponent".to_string());
            self.game_state.cannot_activate_members.push(target);
        }
        Ok(())
    }

    fn execute_re_yell(&mut self, lose_blade_hearts: bool, target: &str) -> Result<(), String> {
        eprintln!("re_yell: lose_blade_hearts={}", lose_blade_hearts);
        let card_db = self.game_state.card_database.clone();
        let mut cards_to_clear_modifiers: Vec<i16> = Vec::new();
        {
            let player = self.game_state.resolve_target_player_mut(target);
            for i in 0..3 {
                if player.stage.stage[i] != -1 {
                    if let Some(card_id) = player.remove_member_from_stage_with_recycling(i, &card_db) {
                        if lose_blade_hearts { cards_to_clear_modifiers.push(card_id); }
                    }
                }
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

    fn execute_activation_restriction(&mut self, target: &str) -> Result<(), String> {
        eprintln!("activation_restriction: target={}", target);
        self.game_state.prohibition_effects.push(format!("activation_restriction:{}", target));
        Ok(())
    }

    fn execute_choose_required_hearts(&mut self) -> Result<(), String> {
        self.pending_choice = Some(Choice::SelectTarget {
            target: "choose_required_hearts".to_string(),
            description: "Choose required hearts".to_string(),
        });
        Ok(())
    }

    fn execute_modify_limit(&mut self, operation: &str, count: u32) -> Result<(), String> {
        eprintln!("modify_limit: operation={}, count={}", operation, count);
        match operation {
            "decrease" => { self.game_state.prohibition_effects.push(format!("limit_decrease:{}", count)); }
            "increase" => { self.game_state.prohibition_effects.push(format!("limit_increase:{}", count)); }
            _ => {}
        }
        Ok(())
    }

    fn execute_set_blade_count(&mut self, value: u32, target: &str) -> Result<(), String> {
        eprintln!("set_blade_count: value={}, target={}", value, target);
        let stage_cards: Vec<i16> = {
            let player = self.game_state.resolve_target_player_mut(target);
            player.stage.stage.to_vec()
        };
        for &card_id in stage_cards.iter().filter(|&&id| id != -1) {
            let current = self.game_state.get_blade_modifier(card_id);
            let delta = (value as i32) - current;
            self.game_state.add_blade_modifier(card_id, delta);
        }
        Ok(())
    }

    fn execute_set_required_hearts(&mut self, heart_colors: &[String], target: &str) -> Result<(), String> {
        let card_ids: Vec<i16> = {
            let player = self.game_state.resolve_target_player_mut(target);
            player.live_card_zone.cards.to_vec()
        };
        for card_id in card_ids {
            let mut color_counts: std::collections::HashMap<crate::card::HeartColor, u32> = std::collections::HashMap::new();
            for color_str in heart_colors {
                let color = crate::zones::parse_heart_color(color_str);
                *color_counts.entry(color).or_insert(0) += 1;
            }
            for (color, count) in &color_counts {
                self.game_state.set_need_heart_modifier(card_id, *color, *count as i32);
            }
        }
        Ok(())
    }

    fn execute_set_score(&mut self, value: u32, target: &str) -> Result<(), String> {
        let activating_id = self.activating_card_id;
        let card_ids: Vec<i16> = {
            let player = self.game_state.resolve_target_player_mut(target);
            player.live_card_zone.cards.iter().copied().collect()
        };
        for &card_id in &card_ids {
            if let Some(aid) = activating_id {
                if card_id != aid { continue; }
            }
            self.game_state.set_score_modifier(card_id, value as i32);
        }
        Ok(())
    }

    fn execute_specify_heart_color(&mut self, choice: bool, target: &str) -> Result<(), String> {
        eprintln!("specify_heart_color: choice={}, target={}", choice, target);
        if choice {
            self.pending_choice = Some(Choice::SelectTarget { target: "heart_color".to_string(), description: "Choose a heart color".to_string() });
        }
        Ok(())
    }

    fn execute_set_card_identity_all_regions(&mut self, identities: Option<&Vec<String>>, target: &str) -> Result<(), String> {
        let _target = target;
        let card_id = self.activating_card_id.or_else(|| self.game_state.activating_card);
        if let Some(card_id) = card_id {
            if let Some(identities) = identities {
                for identity in identities {
                    self.game_state.prohibition_effects.push(format!("card_identity:{}:{}", card_id, identity));
                }
            }
        }
        Ok(())
    }

    fn execute_modify_required_hearts_success(&mut self, operation: &str, value: u32, target: &str, card_type: Option<&str>) -> Result<(), String> {
        let player = self.game_state.resolve_target_player_mut(target);
        let card_ids: Vec<i16> = if let Some("live_card") = card_type { player.success_live_card_zone.cards.iter().copied().collect() } else { vec![] };
        let delta = match operation { "increase" => value as i32, "decrease" => -(value as i32), _ => return Err(format!("Unknown operation: {}", operation)) };
        let heart_colors = ["heart00", "heart01", "heart02", "heart03", "heart04", "heart05", "heart06"];
        for card_id in card_ids {
            for color_str in &heart_colors {
                let color = crate::zones::parse_heart_color(color_str);
                self.game_state.add_need_heart_modifier(card_id, color, delta);
            }
        }
        Ok(())
    }

    fn execute_set_cost_to_use(&mut self, value: u32) -> Result<(), String> {
        let card_id = self.activating_card_id.or_else(|| self.game_state.activating_card);
        if let Some(card_id) = card_id { self.game_state.set_cost_modifier(card_id, value as i32); }
        Ok(())
    }

    fn execute_all_blade_timing(&mut self, timing: &str, treat_as: &str) -> Result<(), String> {
        let card_id = self.activating_card_id.or_else(|| self.game_state.activating_card);
        if let Some(card_id) = card_id {
            self.game_state.prohibition_effects.push(format!("all_blade_timing:{}:{}:{}", card_id, timing, treat_as));
        }
        Ok(())
    }

    fn execute_shuffle(&mut self, target: &str, source: &str) -> Result<(), String> {
        let player = self.game_state.resolve_target_player_mut(target);
        match source {
            "deck" => { use rand::seq::SliceRandom; player.main_deck.cards.shuffle(&mut rand::thread_rng()); }
            "energy_deck" => { use rand::seq::SliceRandom; player.energy_deck.cards.shuffle(&mut rand::thread_rng()); }
            _ => { eprintln!("Unknown shuffle zone: {}", source); }
        }
        Ok(())
    }

    fn execute_modify_cost(&mut self, operation: &str, value: u32, target: &str, card_type: Option<&str>) -> Result<(), String> {
        let player = self.game_state.resolve_target_player_mut(target);
        let card_ids: Vec<i16> = if let Some("live_card") = card_type { player.live_card_zone.cards.iter().copied().collect() }
            else if let Some("member_card") = card_type { player.stage.stage.iter().filter(|&&id| id != -1).copied().collect() }
            else if let Some("energy_card") = card_type { player.energy_zone.cards.iter().copied().collect() }
            else { player.hand.cards.iter().copied().collect() };
        let delta = match operation { "add" => value as i32, "subtract" => -(value as i32), "set" => value as i32, _ => return Err(format!("Unknown operation: {}", operation)) };
        for card_id in card_ids {
            if operation == "set" { self.game_state.set_cost_modifier(card_id, delta); }
            else { self.game_state.add_cost_modifier(card_id, delta); }
        }
        Ok(())
    }
}
