use crate::card::{AbilityEffect, ActionModifiers, Condition, PositionInfo};

#[derive(Debug, Clone)]
pub enum Effect {
    Sequential {
        actions: Vec<Effect>,
        conditional: bool,
        is_further: bool,
    },
    ConditionalAlternative {
        primary_effect: Box<Effect>,
        alternative_effect: Box<Effect>,
        alternative_condition: Option<Condition>,
    },
    LookAndSelect {
        look_action: Box<Effect>,
        select_action: Box<Effect>,
    },
    Draw {
        source: String,
        destination: String,
        modifiers: ActionModifiers,
    },
    DrawUntilCount {
        target_count: u32,
        destination: String,
        modifiers: ActionModifiers,
    },
    MoveCards {
        source: String,
        destination: String,
        self_cost: bool,
        placement_order: Option<String>,
        distinct: Option<String>,
        name_constraint: Option<String>,
        name_constraint_source: Option<String>,
        modifiers: ActionModifiers,
    },
    GainResource {
        resource: String,
        heart_color: Option<String>,
        resource_icon_count: Option<u32>,
        modifiers: ActionModifiers,
    },
    ChangeState {
        state_change: String,
        self_cost: bool,
        modifiers: ActionModifiers,
    },
    ModifyScore {
        operation: String,
        value: u32,
        effect_constraint: Option<String>,
        modifiers: ActionModifiers,
    },
    ModifyRequiredHearts {
        operation: String,
        value: u32,
        heart_color: String,
        modifiers: ActionModifiers,
    },
    SetCost {
        value: u32,
        modifiers: ActionModifiers,
    },
    SetBladeType {
        blade_type: Option<String>,
        modifiers: ActionModifiers,
    },
    SetHeartType {
        heart_type: Option<String>,
        modifiers: ActionModifiers,
    },
    ActivateAbility {
        ability_text: String,
        modifiers: ActionModifiers,
    },
    InvalidateAbility,
    GainAbility {
        ability_text: String,
        modifiers: ActionModifiers,
    },
    PlayBatonTouch {
        modifiers: ActionModifiers,
    },
    Reveal {
        source: String,
        modifiers: ActionModifiers,
    },
    Select {
        source: String,
        distinct: Option<String>,
        modifiers: ActionModifiers,
    },
    LookAt {
        source: String,
        modifiers: ActionModifiers,
    },
    ModifyRequiredHeartsGlobal {
        operation: String,
        value: u32,
        heart_color: String,
        modifiers: ActionModifiers,
    },
    ModifyYellCount {
        operation: String,
        modifiers: ActionModifiers,
    },
    PlaceEnergyUnderMember {
        energy_count: u32,
        modifiers: ActionModifiers,
    },
    ActivationCost {
        operation: String,
        value: u32,
        modifiers: ActionModifiers,
    },
    PositionChange {
        position: Option<PositionInfo>,
        target_member: String,
        modifiers: ActionModifiers,
    },
    Appear {
        source: String,
        destination: String,
        modifiers: ActionModifiers,
    },
    Choice {
        choice_options: Option<Vec<String>>,
        choice_type: Option<String>,
        options: Option<Vec<AbilityEffect>>,
    },
    PayEnergy {
        energy: u32,
        modifiers: ActionModifiers,
    },
    SetCardIdentity {
        identities: Vec<String>,
    },
    RepeatProcedure {
        repeat_limit: u32,
        actions: Vec<Effect>,
    },
    DiscardUntilCount {
        target_count: u32,
        modifiers: ActionModifiers,
    },
    Restriction {
        restriction_type: Option<String>,
        restricted_destination: Option<String>,
    },
    ReYell {
        lose_blade_hearts: bool,
        modifiers: ActionModifiers,
    },
    ActivationRestriction {
        modifiers: ActionModifiers,
    },
    ChooseRequiredHearts,
    ModifyLimit {
        operation: String,
        count: u32,
    },
    SetBladeCount {
        value: u32,
        modifiers: ActionModifiers,
    },
    DoNothing,
    SetRequiredHearts {
        count: u32,
        heart_color: String,
        modifiers: ActionModifiers,
    },
    SetScore {
        value: u32,
        modifiers: ActionModifiers,
    },
    SpecifyHeartColor {
        choice: bool,
        modifiers: ActionModifiers,
    },
    ModifyRequiredHeartsSuccess {
        operation: String,
        value: u32,
        modifiers: ActionModifiers,
    },
    SetCostToUse {
        value: u32,
    },
    AllBladeTiming {
        timing: String,
        treat_as: String,
    },
    SetCardIdentityAllRegions {
        identities: Option<Vec<String>>,
        modifiers: ActionModifiers,
    },
    Shuffle {
        source: String,
        modifiers: ActionModifiers,
    },
    RevealPerGroup {
        source: String,
        modifiers: ActionModifiers,
    },
    ConditionalOnResult {
        primary_effect: Box<Effect>,
        result_condition: Option<Condition>,
        followup_action: Option<Box<Effect>>,
    },
    ConditionalOnOptional {
        optional_action: Option<Box<Effect>>,
        conditional_action: Option<Box<Effect>>,
    },
    ModifyCost {
        operation: String,
        value: u32,
        modifiers: ActionModifiers,
    },
}

impl Effect {
    pub fn from_ability_effect(effect: &AbilityEffect) -> Self {
        let m = effect.extract_modifiers();
        match effect.action.as_str() {
            "sequential" => {
                let actions = effect.actions.as_ref()
                    .map(|a| a.iter().map(Self::from_ability_effect).collect())
                    .unwrap_or_default();
                Effect::Sequential {
                    actions,
                    conditional: effect.conditional.unwrap_or(false),
                    is_further: effect.is_further.unwrap_or(false),
                }
            }
            "conditional_alternative" => {
                let primary = effect.primary_effect.as_ref().map(|e| Box::new(Self::from_ability_effect(e)));
                let alternative = effect.alternative_effect.as_ref().map(|e| Box::new(Self::from_ability_effect(e)));
                Effect::ConditionalAlternative {
                    primary_effect: primary.unwrap_or_else(|| Box::new(Effect::DoNothing)),
                    alternative_effect: alternative.unwrap_or_else(|| Box::new(Effect::DoNothing)),
                    alternative_condition: effect.alternative_condition.clone(),
                }
            }
            "look_and_select" => {
                let look = effect.look_action.as_ref().map(|e| Box::new(Self::from_ability_effect(e)));
                let select = effect.select_action.as_ref().map(|e| Box::new(Self::from_ability_effect(e)));
                Effect::LookAndSelect {
                    look_action: look.unwrap_or_else(|| Box::new(Effect::DoNothing)),
                    select_action: select.unwrap_or_else(|| Box::new(Effect::DoNothing)),
                }
            }
            "discard_card" | "move_cards" => Effect::MoveCards {
                source: effect.source.as_deref().unwrap_or_default().to_string(),
                destination: effect.destination.as_deref().unwrap_or_default().to_string(),
                self_cost: effect.self_cost.unwrap_or(false),
                placement_order: effect.placement_order.clone(),
                distinct: effect.distinct.clone(),
                name_constraint: effect.name_constraint.clone(),
                name_constraint_source: effect.name_constraint_source.clone(),
                modifiers: m,
            },
            "draw" | "draw_card" => Effect::Draw {
                source: effect.source.as_deref().unwrap_or("deck").to_string(),
                destination: effect.destination.as_deref().unwrap_or("hand").to_string(),
                modifiers: m,
            },
            "draw_until_count" => Effect::DrawUntilCount {
                target_count: effect.target_count.unwrap_or(0),
                destination: effect.destination.as_deref().unwrap_or("hand").to_string(),
                modifiers: m,
            },
            "gain_resource" => Effect::GainResource {
                resource: effect.resource.as_deref().unwrap_or_default().to_string(),
                heart_color: effect.heart_color.clone(),
                resource_icon_count: effect.resource_icon_count,
                modifiers: m,
            },
            "change_state" => Effect::ChangeState {
                state_change: effect.state_change.as_deref().unwrap_or_default().to_string(),
                self_cost: effect.self_cost.unwrap_or(false),
                modifiers: m,
            },
            "modify_score" => Effect::ModifyScore {
                operation: effect.operation.as_deref().unwrap_or("add").to_string(),
                value: effect.value.unwrap_or(0),
                effect_constraint: effect.effect_constraint.clone(),
                modifiers: m,
            },
            "modify_required_hearts" => Effect::ModifyRequiredHearts {
                operation: effect.operation.as_deref().unwrap_or("decrease").to_string(),
                value: effect.value.unwrap_or(0),
                heart_color: effect.heart_color.as_deref().unwrap_or("heart00").to_string(),
                modifiers: m,
            },
            "set_cost" => Effect::SetCost {
                value: effect.value.unwrap_or(0),
                modifiers: m,
            },
            "set_blade_type" => Effect::SetBladeType {
                blade_type: effect.blade_type.clone(),
                modifiers: m,
            },
            "set_heart_type" => Effect::SetHeartType {
                heart_type: effect.heart_type.as_deref().or(effect.heart_color.as_deref()).map(|s| s.to_string()),
                modifiers: m,
            },
            "activate_ability" => Effect::ActivateAbility {
                ability_text: effect.ability_text.as_deref().unwrap_or_default().to_string(),
                modifiers: m,
            },
            "invalidate_ability" => Effect::InvalidateAbility,
            "gain_ability" => Effect::GainAbility {
                ability_text: effect.ability_gain.as_deref().unwrap_or_default().to_string(),
                modifiers: m,
            },
            "play_baton_touch" => Effect::PlayBatonTouch {
                modifiers: m,
            },
            "reveal" => Effect::Reveal {
                source: effect.source.as_deref().unwrap_or("hand").to_string(),
                modifiers: m,
            },
            "select" => Effect::Select {
                source: effect.source.as_deref().unwrap_or("hand").to_string(),
                distinct: effect.distinct.clone(),
                modifiers: m,
            },
            "look_at" => Effect::LookAt {
                source: effect.source.as_deref().unwrap_or("deck").to_string(),
                modifiers: m,
            },
            "modify_required_hearts_global" => Effect::ModifyRequiredHeartsGlobal {
                operation: effect.operation.as_deref().unwrap_or("increase").to_string(),
                value: effect.value.unwrap_or(1),
                heart_color: effect.heart_color.as_deref().unwrap_or("heart00").to_string(),
                modifiers: m,
            },
            "modify_yell_count" => Effect::ModifyYellCount {
                operation: effect.operation.as_deref().unwrap_or("subtract").to_string(),
                modifiers: m,
            },
            "place_energy_under_member" => Effect::PlaceEnergyUnderMember {
                energy_count: effect.energy_count.unwrap_or(1),
                modifiers: m,
            },
            "activation_cost" => Effect::ActivationCost {
                operation: effect.operation.as_deref().unwrap_or("increase").to_string(),
                value: effect.value.unwrap_or(0),
                modifiers: m,
            },
            "position_change" => Effect::PositionChange {
                position: effect.position.clone(),
                target_member: effect.target_member.as_deref().unwrap_or("this_member").to_string(),
                modifiers: m,
            },
            "appear" => Effect::Appear {
                source: effect.source.as_deref().unwrap_or_default().to_string(),
                destination: effect.destination.as_deref().unwrap_or("stage").to_string(),
                modifiers: m,
            },
            "choice" => Effect::Choice {
                choice_options: effect.choice_options.clone(),
                choice_type: effect.choice_type.clone(),
                options: effect.options.clone(),
            },
            "pay_energy" => Effect::PayEnergy {
                energy: effect.count.unwrap_or(0),
                modifiers: m,
            },
            "set_card_identity" => Effect::SetCardIdentity {
                identities: effect.identities.clone().unwrap_or_default(),
            },
            "repeat_procedure" => {
                let actions = effect.actions.as_ref()
                    .map(|a| a.iter().map(Self::from_ability_effect).collect())
                    .unwrap_or_default();
                Effect::RepeatProcedure {
                    repeat_limit: effect.repeat_limit.unwrap_or(1),
                    actions,
                }
            }
            "discard_until_count" => Effect::DiscardUntilCount {
                target_count: effect.target_count.unwrap_or(0),
                modifiers: m,
            },
            "restriction" => Effect::Restriction {
                restriction_type: effect.restriction_type.clone(),
                restricted_destination: effect.restricted_destination.clone(),
            },
            "re_yell" => Effect::ReYell {
                lose_blade_hearts: effect.lose_blade_hearts.unwrap_or(false),
                modifiers: m,
            },
            "activation_restriction" => Effect::ActivationRestriction {
                modifiers: m,
            },
            "choose_required_hearts" => Effect::ChooseRequiredHearts,
            "modify_limit" => Effect::ModifyLimit {
                operation: effect.operation.as_deref().unwrap_or("decrease").to_string(),
                count: effect.count.unwrap_or(0),
            },
            "set_blade_count" => Effect::SetBladeCount {
                value: effect.value.unwrap_or(0),
                modifiers: m,
            },
            "do_nothing" => Effect::DoNothing,
            "set_required_hearts" => Effect::SetRequiredHearts {
                count: effect.count.unwrap_or(0),
                heart_color: effect.heart_color.as_deref().unwrap_or("heart00").to_string(),
                modifiers: m,
            },
            "set_score" => Effect::SetScore {
                value: effect.value.unwrap_or(0),
                modifiers: m,
            },
            "specify_heart_color" => Effect::SpecifyHeartColor {
                choice: effect.choice.unwrap_or(false),
                modifiers: m,
            },
            "modify_required_hearts_success" => Effect::ModifyRequiredHeartsSuccess {
                operation: effect.operation.as_deref().unwrap_or("increase").to_string(),
                value: effect.value.unwrap_or(0),
                modifiers: m,
            },
            "set_cost_to_use" => Effect::SetCostToUse {
                value: effect.value.unwrap_or(0),
            },
            "all_blade_timing" => Effect::AllBladeTiming {
                timing: effect.timing.as_deref().unwrap_or("check_required_hearts").to_string(),
                treat_as: effect.treat_as.as_deref().unwrap_or("any_heart_color").to_string(),
            },
            "set_card_identity_all_regions" => Effect::SetCardIdentityAllRegions {
                identities: effect.identities.clone(),
                modifiers: m,
            },
            "shuffle" => Effect::Shuffle {
                source: effect.source.as_deref().unwrap_or("deck").to_string(),
                modifiers: m,
            },
            "reveal_per_group" => Effect::RevealPerGroup {
                source: effect.source.as_deref().unwrap_or("hand").to_string(),
                modifiers: m,
            },
            "conditional_on_result" => {
                let primary = effect.primary_effect.as_ref().map(|e| Box::new(Self::from_ability_effect(e)));
                let followup = effect.followup_action.as_ref().map(|e| Box::new(Self::from_ability_effect(e)));
                Effect::ConditionalOnResult {
                    primary_effect: primary.unwrap_or_else(|| Box::new(Effect::DoNothing)),
                    result_condition: effect.result_condition.clone(),
                    followup_action: followup,
                }
            }
            "conditional_on_optional" => {
                let optional = effect.optional_action.as_ref().map(|e| Box::new(Self::from_ability_effect(e)));
                let conditional = effect.conditional_action.as_ref().map(|e| Box::new(Self::from_ability_effect(e)));
                Effect::ConditionalOnOptional {
                    optional_action: optional,
                    conditional_action: conditional,
                }
            }
            "modify_cost" => Effect::ModifyCost {
                operation: effect.operation.as_deref().unwrap_or("add").to_string(),
                value: effect.value.unwrap_or(0),
                modifiers: m,
            },
            "custom" => Effect::DoNothing,
            _ => {
                eprintln!("Unknown action: '{}', treating as DoNothing", effect.action);
                Effect::DoNothing
            }
        }
    }
}
