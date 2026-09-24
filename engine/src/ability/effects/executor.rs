use crate::ability::enums::ActionType;
use crate::ability::resolver::AbilityResolver;
use crate::card::AbilityEffect;
use crate::game_state::GameState;

#[cfg(feature = "no_std")]
use alloc::string::String;

pub fn push_effect_verdict(effect: &AbilityEffect) {
    let is_structural = matches!(
        effect.action,
        ActionType::CompoundAction
            | ActionType::Sequential
            | ActionType::Choice
            | ActionType::ConditionalAlternative
            | ActionType::ConditionalOnResult
            | ActionType::ConditionalOnOptional
    );
    if is_structural {
        return;
    }
    #[cfg(not(feature = "no_std"))]
    let val = effect
        .count
        .or(effect.value_any())
        .map(|v| v.to_string())
        .unwrap_or_default();
    #[cfg(not(feature = "no_std"))]
    let details = if !val.is_empty() {
        format!("{} {}", effect.action, val)
    } else {
        effect.action.to_string()
    };
    #[cfg(not(feature = "no_std"))]
    crate::ability::log::push_verdict(crate::ability::log::AbilityLogItem::Effect {
        text: effect.text.to_string(),
        action: effect.action.to_string(),
        details,
    });
}

pub fn execute_effect(
    resolver: &mut AbilityResolver,
    gs: &mut GameState,
    effect: &AbilityEffect,
) -> Result<(), String> {
    let result = match effect.action {
        ActionType::DrawCard => resolver.execute_draw_wrapper(gs, effect),
        ActionType::DrawUntilCount => {
            resolver.execute_draw_until_count(gs, effect);
            Ok(())
        }
        ActionType::MoveCards => resolver.execute_logged_move(gs, effect, "[[log_move]]"),
        ActionType::DiscardCard => resolver.execute_logged_move(gs, effect, "[[log_discard]]"),
        ActionType::Select => {
            resolver.rule_log_activated(gs, "[[log_select]]");
            resolver.execute_select_effect(gs, effect)
        }
        ActionType::SelectCards => resolver.execute_select_cards(gs, effect),
        ActionType::LookAndSelect => resolver.execute_look_and_select(gs, effect),
        ActionType::LookAt => resolver.execute_look_at(gs, effect),
        ActionType::Reveal => resolver.execute_reveal_effect(gs, effect),
        ActionType::RevealPerGroup => resolver.execute_reveal_per_group(gs, effect),
        ActionType::RevealUntilLiveCard => resolver.execute_reveal_until_live_card(gs, effect),
        ActionType::RevealUntilChosenCard => resolver.execute_reveal_until_chosen_card(gs, effect),
        ActionType::ChangeState => resolver.execute_change_state(gs, effect),
        ActionType::PositionChange => resolver.execute_position_change(
            gs,
            effect,
            effect.position_any().cloned(),
            effect.target_name(),
            effect.target_member_any().as_deref().unwrap_or("this_member"),
        ),
        ActionType::Rotation => resolver.execute_rotation(gs, effect, effect.target_name()),
        ActionType::PlaceEnergyUnderMember => {
            resolver.execute_place_energy_under_member(gs, effect);
            Ok(())
        }
        ActionType::SetCardIdentity => resolver.execute_set_card_identity_effect(gs, effect),
        ActionType::ModifyRequiredHeartsSuccess => {
            resolver.execute_modify_required_hearts_success(gs, effect);
            Ok(())
        }
        ActionType::GainResource => resolver.execute_gain_resource(gs, effect),
        ActionType::PayEnergy => resolver.execute_pay_energy(gs, effect),
        ActionType::GainAbility => resolver.execute_gain_ability_effect(gs, effect),
        ActionType::GainAbilityFromSource => resolver.execute_gain_ability_from_source(gs, effect),
        ActionType::InvalidateAbility => resolver.execute_invalidate_ability(gs, effect),
        ActionType::SuppressAbilityTrigger => resolver.execute_suppress_ability_trigger(gs, effect),
        ActionType::ActivateAbility => {
            resolver.execute_activate_ability(gs, effect);
            Ok(())
        }
        ActionType::ModifyCost => {
            resolver.execute_modify_cost(gs, effect);
            Ok(())
        }
        ActionType::ModifyYellSource => Ok(()),
        ActionType::SetCost => {
            resolver.execute_set_cost(gs, effect);
            Ok(())
        }
        ActionType::SetCostToUse => resolver.execute_set_cost_to_use(gs, effect),
        ActionType::ModifyScore => resolver.execute_modify_score(gs, effect),
        ActionType::ModifyRequiredHearts => resolver.execute_modify_required_hearts(gs, effect),
        ActionType::SetBladeType => {
            resolver.execute_set_blade_type(gs, effect);
            Ok(())
        }
        ActionType::SetBladeCount => {
            resolver.execute_set_blade_count(gs, effect);
            Ok(())
        }
        ActionType::SetHeartType => {
            resolver.execute_set_heart_type(gs, effect);
            Ok(())
        }
        ActionType::SpecifyHeartColor => {
            resolver.execute_specify_heart_color(gs, effect);
            Ok(())
        }
        ActionType::ChooseRequiredHearts => {
            resolver.execute_choose_required_hearts(gs);
            Ok(())
        }
        ActionType::Sequential => resolver.execute_sequential_effect(gs, effect),
        ActionType::ConditionalAlternative => resolver.execute_conditional_alternative(gs, effect),
        ActionType::ConditionalOnResult => resolver.execute_conditional_on_result(gs, effect),
        ActionType::ConditionalOnOptional => resolver.execute_conditional_on_optional(gs, effect),
        ActionType::Restriction => resolver.execute_restriction(gs, effect),
        ActionType::ActivationRestriction => {
            resolver.execute_activation_restriction(gs, effect);
            Ok(())
        }
        ActionType::ModifyLimit => resolver.execute_modify_limit(gs, effect),
        ActionType::Shuffle => {
            resolver.execute_shuffle(gs, effect);
            Ok(())
        }
        ActionType::ReYell => {
            resolver.execute_re_yell(gs, effect);
            Ok(())
        }
        ActionType::Custom => resolver.execute_custom(gs, effect, effect.action.to_str()),
        ActionType::DoNothing => Ok(()),
        ActionType::Choice => resolver.execute_choice(gs, effect),
        ActionType::RepeatProcedure => Ok(()),
        ActionType::DiscardUntilCount => resolver.execute_discard_until_count(gs, effect),
        ActionType::AllBladeTiming => {
            resolver.execute_all_blade_timing(gs, effect);
            Ok(())
        }
        ActionType::ReduceLiveCardSetLimit => {
            resolver.execute_reduce_live_card_set_limit(gs, effect);
            Ok(())
        }
        ActionType::ChooseTargetPlayer => resolver.execute_choose_target_player(gs, effect),
        ActionType::SelectNumber => resolver.execute_select_number(gs, effect),
        ActionType::PlayBatonTouch => {
            let _ = resolver.execute_play_baton_touch(gs, effect);
            Ok(())
        }
        ActionType::ModifyRequiredHeartsGlobal => resolver.execute_modify_required_hearts_standard(
            gs,
            effect.operation_any().as_deref().unwrap_or("increase"),
            effect.value_or_count(1) as u8,
            effect.heart_colors_any(),
            effect.target_name(),
            &effect.text,
        ),
        ActionType::ModifyYellCount => {
            resolver.execute_modify_yell_count(gs, effect);
            Ok(())
        }
        ActionType::ActivationCost => {
            resolver.execute_activation_cost(gs, effect);
            Ok(())
        }
        ActionType::PerformYell => {
            resolver.execute_perform_yell(gs, effect);
            Ok(())
        }
        ActionType::ConditionalOptional => resolver.execute_conditional_on_optional(gs, effect),
        ActionType::CompoundAction
        | ActionType::OpponentAction
        | ActionType::ActionBy
        | ActionType::SequentialCost
        | ActionType::ChoiceCondition
        | ActionType::EnergyCondition => Ok(()),
    };
    push_effect_verdict(effect);
    result
}
