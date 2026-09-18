//! Unified effect executor trait and registry — replaces the massive match
//! statement in execute_effect with table-driven dispatch.

use crate::ability::enums::ActionType;
use crate::ability::resolver::AbilityResolver;
use crate::card::AbilityEffect;
use crate::game_state::GameState;

/// Trait for executing a specific action type. Each action type gets its own
/// implementation, making the dispatch table-driven instead of a 50-arm match.
pub trait EffectExecutor: Send + Sync {
    /// Execute this action type.
    fn execute(&self, resolver: &mut AbilityResolver, gs: &mut GameState, effect: &AbilityEffect) -> Result<(), String>;

    /// The action type this executor handles.
    fn action_type(&self) -> ActionType;

    /// Whether this action can produce a pending choice.
    fn can_produce_choice(&self) -> bool { true }

    /// Whether this is a structural action (compound, sequential, choice routing).
    fn is_structural(&self) -> bool { false }
}

/// Registry of all effect executors for table-driven dispatch.
pub struct ExecutorRegistry {
    executors: std::collections::HashMap<ActionType, &'static dyn EffectExecutor>,
}

impl ExecutorRegistry {
    pub fn new() -> Self {
        Self { executors: std::collections::HashMap::new() }
    }

    /// Register an executor for its action type.
    pub fn register(mut self, executor: &'static dyn EffectExecutor) -> Self {
        self.executors.insert(executor.action_type(), executor);
        self
    }

    /// Get executor for an action type.
    pub fn get(&self, action_type: ActionType) -> Option<&'static dyn EffectExecutor> {
        self.executors.get(&action_type).copied()
    }
}

/// Global registry — initialized once with all executors.
static EXECUTOR_REGISTRY: std::sync::OnceLock<ExecutorRegistry> = std::sync::OnceLock::new();

/// Initialize the executor registry.
pub fn init_executor_registry() -> &'static ExecutorRegistry {
    EXECUTOR_REGISTRY.get_or_init(|| {
        ExecutorRegistry::new()
            .register(&DrawCardExecutor)
            .register(&DrawUntilCountExecutor)
            .register(&MoveCardsExecutor)
            .register(&DiscardCardExecutor)
            .register(&GainResourceExecutor)
            .register(&ChangeStateExecutor)
            .register(&ModifyScoreExecutor)
            .register(&ModifyRequiredHeartsExecutor)
            .register(&SetCostExecutor)
            .register(&SetBladeTypeExecutor)
            .register(&SetHeartTypeExecutor)
            .register(&ActivateAbilityExecutor)
            .register(&InvalidateAbilityExecutor)
            .register(&SuppressAbilityTriggerExecutor)
            .register(&GainAbilityExecutor)
            .register(&GainAbilityFromSourceExecutor)
            .register(&PlayBatonTouchExecutor)
            .register(&RevealExecutor)
            .register(&SelectExecutor)
            .register(&SelectNumberExecutor)
            .register(&LookAtExecutor)
            .register(&ModifyRequiredHeartsGlobalExecutor)
            .register(&ModifyYellCountExecutor)
            .register(&PlaceEnergyUnderMemberExecutor)
            .register(&ActivationCostExecutor)
            .register(&PositionChangeExecutor)
            .register(&RotationExecutor)
            .register(&ChoiceExecutor)
            .register(&PayEnergyExecutor)
            .register(&SetCardIdentityExecutor)
            .register(&DiscardUntilCountExecutor)
            .register(&RestrictionExecutor)
            .register(&ReYellExecutor)
            .register(&ActivationRestrictionExecutor)
            .register(&ChooseRequiredHeartsExecutor)
            .register(&ModifyLimitExecutor)
            .register(&ReduceLiveCardSetLimitExecutor)
            .register(&SetBladeCountExecutor)
            .register(&CustomExecutor)
            .register(&DoNothingExecutor)
            .register(&ModifyYellSourceExecutor)
            .register(&SpecifyHeartColorExecutor)
            .register(&ModifyRequiredHeartsSuccessExecutor)
            .register(&SetCostToUseExecutor)
            .register(&AllBladeTimingExecutor)
            .register(&ShuffleExecutor)
            .register(&RevealPerGroupExecutor)
            .register(&ConditionalOnResultExecutor)
            .register(&ConditionalOnOptionalExecutor)
            .register(&ModifyCostExecutor)
            .register(&RevealUntilLiveCardExecutor)
            .register(&RevealUntilChosenCardExecutor)
            .register(&ChooseTargetPlayerExecutor)
            .register(&PerformYellExecutor)
            .register(&SequentialExecutor)
            .register(&ConditionalAlternativeExecutor)
            .register(&LookAndSelectExecutor)
            .register(&SelectCardsExecutor)
    })
}

// ====================================================================
// Individual executor implementations
// ====================================================================

macro_rules! simple_executor {
    ($name:ident, $action:expr, $method:ident) => {
        struct $name;
        impl EffectExecutor for $name {
            fn action_type(&self) -> ActionType { $action }
            fn execute(&self, resolver: &mut AbilityResolver, gs: &mut GameState, effect: &AbilityEffect) -> Result<(), String> {
                resolver.$method(gs, effect)
            }
        }
    };
}

macro_rules! void_executor {
    ($name:ident, $action:expr, $method:ident) => {
        struct $name;
        impl EffectExecutor for $name {
            fn action_type(&self) -> ActionType { $action }
            fn execute(&self, resolver: &mut AbilityResolver, gs: &mut GameState, effect: &AbilityEffect) -> Result<(), String> {
                resolver.$method(gs, effect);
                Ok(())
            }
        }
    };
}

macro_rules! void_executor_no_effect {
    ($name:ident, $action:expr, $method:ident) => {
        struct $name;
        impl EffectExecutor for $name {
            fn action_type(&self) -> ActionType { $action }
            fn execute(&self, resolver: &mut AbilityResolver, gs: &mut GameState, _effect: &AbilityEffect) -> Result<(), String> {
                resolver.$method(gs);
                Ok(())
            }
        }
    };
}

macro_rules! logged_move_executor {
    ($name:ident, $action:expr, $log_tag:expr) => {
        struct $name;
        impl EffectExecutor for $name {
            fn action_type(&self) -> ActionType { $action }
            fn execute(&self, resolver: &mut AbilityResolver, gs: &mut GameState, effect: &AbilityEffect) -> Result<(), String> {
                resolver.execute_logged_move(gs, effect, $log_tag)
            }
        }
    };
}

macro_rules! structural_executor {
    ($name:ident, $action:expr, $method:ident) => {
        struct $name;
        impl EffectExecutor for $name {
            fn action_type(&self) -> ActionType { $action }
            fn is_structural(&self) -> bool { true }
            fn execute(&self, resolver: &mut AbilityResolver, gs: &mut GameState, effect: &AbilityEffect) -> Result<(), String> {
                resolver.$method(gs, effect)
            }
        }
    };
}

macro_rules! custom_executor {
    ($name:ident, $action:expr, |$resolver:ident, $gs:ident, $effect:ident| $body:block) => {
        struct $name;
        impl EffectExecutor for $name {
            fn action_type(&self) -> ActionType { $action }
            fn execute(&self, $resolver: &mut AbilityResolver, $gs: &mut GameState, $effect: &AbilityEffect) -> Result<(), String> $body
        }
    };
}

/// Push the post-dispatch effect verdict for non-structural action types.
pub fn push_effect_verdict(effect: &AbilityEffect) {
    // Push effect verdict for non-structural action types
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

// Simple one-liner executors (return Result)
simple_executor!(DrawCardExecutor, ActionType::DrawCard, execute_draw_wrapper);
simple_executor!(GainResourceExecutor, ActionType::GainResource, execute_gain_resource);
simple_executor!(SetCostToUseExecutor, ActionType::SetCostToUse, execute_set_cost_to_use);
simple_executor!(ChangeStateExecutor, ActionType::ChangeState, execute_change_state);
simple_executor!(ModifyScoreExecutor, ActionType::ModifyScore, execute_modify_score);
simple_executor!(ModifyRequiredHeartsExecutor, ActionType::ModifyRequiredHearts, execute_modify_required_hearts);
simple_executor!(InvalidateAbilityExecutor, ActionType::InvalidateAbility, execute_invalidate_ability);
simple_executor!(SuppressAbilityTriggerExecutor, ActionType::SuppressAbilityTrigger, execute_suppress_ability_trigger);
simple_executor!(GainAbilityExecutor, ActionType::GainAbility, execute_gain_ability_effect);
simple_executor!(GainAbilityFromSourceExecutor, ActionType::GainAbilityFromSource, execute_gain_ability_from_source);
simple_executor!(PlayBatonTouchExecutor, ActionType::PlayBatonTouch, execute_play_baton_touch);
simple_executor!(RevealExecutor, ActionType::Reveal, execute_reveal_effect);
simple_executor!(SelectExecutor, ActionType::Select, execute_select_effect);
simple_executor!(SelectNumberExecutor, ActionType::SelectNumber, execute_select_number);
simple_executor!(LookAtExecutor, ActionType::LookAt, execute_look_at);
simple_executor!(RestrictionExecutor, ActionType::Restriction, execute_restriction);
simple_executor!(ModifyLimitExecutor, ActionType::ModifyLimit, execute_modify_limit);
simple_executor!(RevealPerGroupExecutor, ActionType::RevealPerGroup, execute_reveal_per_group);
simple_executor!(ConditionalOnResultExecutor, ActionType::ConditionalOnResult, execute_conditional_on_result);
simple_executor!(ConditionalOnOptionalExecutor, ActionType::ConditionalOnOptional, execute_conditional_on_optional);
simple_executor!(RevealUntilLiveCardExecutor, ActionType::RevealUntilLiveCard, execute_reveal_until_live_card);
simple_executor!(RevealUntilChosenCardExecutor, ActionType::RevealUntilChosenCard, execute_reveal_until_chosen_card);
simple_executor!(ChooseTargetPlayerExecutor, ActionType::ChooseTargetPlayer, execute_choose_target_player);

// Void-returning executors (methods returning (); the trailing `;` discards
// the unit value — this arm shape exists because simple_executor's `?`
// cannot apply to non-Result returns)
void_executor!(DrawUntilCountExecutor, ActionType::DrawUntilCount, execute_draw_until_count);
void_executor!(SetCostExecutor, ActionType::SetCost, execute_set_cost);
void_executor!(SetBladeTypeExecutor, ActionType::SetBladeType, execute_set_blade_type);
void_executor!(SetHeartTypeExecutor, ActionType::SetHeartType, execute_set_heart_type);
void_executor!(ActivateAbilityExecutor, ActionType::ActivateAbility, execute_activate_ability);
void_executor!(ModifyYellCountExecutor, ActionType::ModifyYellCount, execute_modify_yell_count);
void_executor!(PlaceEnergyUnderMemberExecutor, ActionType::PlaceEnergyUnderMember, execute_place_energy_under_member);
void_executor!(ActivationCostExecutor, ActionType::ActivationCost, execute_activation_cost);
void_executor!(ReYellExecutor, ActionType::ReYell, execute_re_yell);
void_executor!(ActivationRestrictionExecutor, ActionType::ActivationRestriction, execute_activation_restriction);
void_executor!(ReduceLiveCardSetLimitExecutor, ActionType::ReduceLiveCardSetLimit, execute_reduce_live_card_set_limit);
void_executor!(SetBladeCountExecutor, ActionType::SetBladeCount, execute_set_blade_count);
void_executor!(SpecifyHeartColorExecutor, ActionType::SpecifyHeartColor, execute_specify_heart_color);
void_executor!(ModifyRequiredHeartsSuccessExecutor, ActionType::ModifyRequiredHeartsSuccess, execute_modify_required_hearts_success);
void_executor!(AllBladeTimingExecutor, ActionType::AllBladeTiming, execute_all_blade_timing);
void_executor!(ShuffleExecutor, ActionType::Shuffle, execute_shuffle);
void_executor!(PerformYellExecutor, ActionType::PerformYell, execute_perform_yell);

// Void-returning executors that take effect as parameter
void_executor!(ModifyCostExecutor, ActionType::ModifyCost, execute_modify_cost);

// Void-returning executors with no effect parameter
void_executor_no_effect!(ChooseRequiredHeartsExecutor, ActionType::ChooseRequiredHearts, execute_choose_required_hearts);

// Custom executors with extra parameters
custom_executor!(CustomExecutor, ActionType::Custom, |resolver, gs, effect| {
    resolver.execute_custom(gs, effect, effect.action.to_str())
});

custom_executor!(PositionChangeExecutor, ActionType::PositionChange, |resolver, gs, effect| {
    resolver.execute_position_change(
        gs,
        effect,
        effect.position_any().cloned(),
        effect.target_name(),
        effect.target_member_any().as_deref().unwrap_or("this_member"),
    )
});

custom_executor!(RotationExecutor, ActionType::Rotation, |resolver, gs, effect| {
    resolver.execute_rotation(gs, effect, effect.target_name())
});

custom_executor!(ModifyRequiredHeartsGlobalExecutor, ActionType::ModifyRequiredHeartsGlobal, |resolver, gs, effect| {
    resolver.execute_modify_required_hearts_standard(
        gs,
        effect.operation_any().as_deref().unwrap_or("increase"),
        effect.value_or_count(1) as u8,
        effect.heart_colors_any(),
        effect.target_name(),
        &effect.text,
    )
});

// No-op executors
struct DoNothingExecutor;
impl EffectExecutor for DoNothingExecutor {
    fn action_type(&self) -> ActionType { ActionType::DoNothing }
    fn execute(&self, _resolver: &mut AbilityResolver, _gs: &mut GameState, _effect: &AbilityEffect) -> Result<(), String> { Ok(()) }
}

struct ModifyYellSourceExecutor;
impl EffectExecutor for ModifyYellSourceExecutor {
    fn action_type(&self) -> ActionType { ActionType::ModifyYellSource }
    fn execute(&self, _resolver: &mut AbilityResolver, _gs: &mut GameState, _effect: &AbilityEffect) -> Result<(), String> { Ok(()) }
}

// Logged move executors (MoveCards/DiscardCard share logic, only log tag differs)
logged_move_executor!(MoveCardsExecutor, ActionType::MoveCards, "[[log_move]]");
logged_move_executor!(DiscardCardExecutor, ActionType::DiscardCard, "[[log_discard]]");

// Structural executors
structural_executor!(SequentialExecutor, ActionType::Sequential, execute_sequential_effect);
structural_executor!(ConditionalAlternativeExecutor, ActionType::ConditionalAlternative, execute_conditional_alternative);
structural_executor!(LookAndSelectExecutor, ActionType::LookAndSelect, execute_look_and_select);
structural_executor!(SelectCardsExecutor, ActionType::SelectCards, execute_select_cards);

// Choice executor
struct ChoiceExecutor;
impl EffectExecutor for ChoiceExecutor {
    fn action_type(&self) -> ActionType { ActionType::Choice }
    fn execute(&self, resolver: &mut AbilityResolver, gs: &mut GameState, effect: &AbilityEffect) -> Result<(), String> {
        resolver.execute_choice(gs, effect)
    }
}

// PayEnergy executor
struct PayEnergyExecutor;
impl EffectExecutor for PayEnergyExecutor {
    fn action_type(&self) -> ActionType { ActionType::PayEnergy }
    fn execute(&self, resolver: &mut AbilityResolver, gs: &mut GameState, effect: &AbilityEffect) -> Result<(), String> {
        resolver.execute_pay_energy(gs, effect)
    }
}

// SetCardIdentity executor
struct SetCardIdentityExecutor;
impl EffectExecutor for SetCardIdentityExecutor {
    fn action_type(&self) -> ActionType { ActionType::SetCardIdentity }
    fn execute(&self, resolver: &mut AbilityResolver, gs: &mut GameState, effect: &AbilityEffect) -> Result<(), String> {
        resolver.execute_set_card_identity_effect(gs, effect)
    }
}

// DiscardUntilCount executor
struct DiscardUntilCountExecutor;
impl EffectExecutor for DiscardUntilCountExecutor {
    fn action_type(&self) -> ActionType { ActionType::DiscardUntilCount }
    fn execute(&self, resolver: &mut AbilityResolver, gs: &mut GameState, effect: &AbilityEffect) -> Result<(), String> {
        resolver.execute_discard_until_count(gs, effect)
    }
}

/// Execute an effect using the registry — replaces the massive match statement.
pub fn execute_effect_via_registry(
    resolver: &mut AbilityResolver,
    gs: &mut GameState,
    effect: &AbilityEffect,
) -> Result<(), String> {
    let registry = init_executor_registry();
    if let Some(executor) = registry.get(effect.action) {
        let result = executor.execute(resolver, gs, effect);
        push_effect_verdict(effect);
        result
    } else {
        log::warn!("No executor registered for action type: {:?}", effect.action);
        Ok(())
    }
}