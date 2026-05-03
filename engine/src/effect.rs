use crate::card::{AbilityEffect, Condition, PositionInfo};

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
        count: u32,
        target: String,
        source: String,
        destination: String,
        card_type: Option<String>,
        per_unit: bool,
        per_unit_count: u32,
        per_unit_type: Option<String>,
    },
    DrawUntilCount {
        count: u32,
        target: String,
        destination: String,
    },
    MoveCards {
        count: u32,
        source: String,
        destination: String,
        target: String,
        card_type: Option<String>,
        group_name: Option<String>,
        cost_limit: Option<u32>,
        self_cost: bool,
        exclude_self: bool,
        position: Option<PositionInfo>,
        placement_order: Option<String>,
    },
    GainResource {
        resource: String,
        count: u32,
        target: String,
        duration: Option<String>,
        card_type: Option<String>,
        group_name: Option<String>,
        per_unit: bool,
        per_unit_count: u32,
        per_unit_type: Option<String>,
        heart_color: Option<String>,
        heart_colors: Option<Vec<String>>,
        resource_icon_count: Option<u32>,
    },
    ChangeState {
        state_change: String,
        target: String,
        count: u32,
        card_type: Option<String>,
        cost_limit: Option<u32>,
        optional: bool,
        group_name: Option<String>,
        self_cost: bool,
        source: Option<String>,
        destination: Option<String>,
    },
    ModifyScore {
        operation: String,
        value: u32,
        target: String,
        duration: Option<String>,
        card_type: Option<String>,
        group_name: Option<String>,
        per_unit: bool,
        per_unit_count: u32,
        per_unit_type: Option<String>,
        effect_constraint: Option<String>,
    },
    ModifyRequiredHearts {
        operation: String,
        value: u32,
        heart_color: String,
        target: String,
    },
    SetCost {
        value: u32,
        target: String,
        card_type: Option<String>,
    },
    SetBladeType {
        blade_type: Option<String>,
        target: String,
        duration: Option<String>,
    },
    SetHeartType {
        heart_type: Option<String>,
        target: String,
        count: u32,
    },
    ActivateAbility {
        ability_text: String,
    },
    InvalidateAbility,
    GainAbility {
        ability_text: String,
        target: String,
        duration: Option<String>,
    },
    PlayBatonTouch {
        count: u32,
        target: String,
    },
    Reveal {
        source: String,
        count: u32,
        target: String,
        card_type: Option<String>,
        heart_colors: Option<Vec<String>>,
    },
    Select {
        source: String,
        count: u32,
        target: String,
        card_type: Option<String>,
        distinct: Option<String>,
        heart_colors: Option<Vec<String>>,
    },
    LookAt {
        count: u32,
        target: String,
        source: String,
    },
    ModifyRequiredHeartsGlobal {
        operation: String,
        value: u32,
        heart_color: String,
        target: String,
    },
    ModifyYellCount {
        operation: String,
        count: u32,
    },
    PlaceEnergyUnderMember {
        count: u32,
        target: String,
        position: Option<PositionInfo>,
    },
    ActivationCost {
        operation: String,
        value: u32,
        target: String,
        duration: Option<String>,
    },
    PositionChange {
        position: Option<PositionInfo>,
        target: String,
        target_member: String,
    },
    Appear {
        source: String,
        destination: String,
        count: u32,
        target: String,
        card_type: Option<String>,
    },
    Choice {
        choice_options: Option<Vec<String>>,
        choice_type: Option<String>,
        options: Option<Vec<AbilityEffect>>,
    },
    PayEnergy {
        count: u32,
        target: String,
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
        target: String,
    },
    Restriction {
        restriction_type: Option<String>,
        restricted_destination: Option<String>,
    },
    ReYell {
        lose_blade_hearts: bool,
        target: String,
    },
    ActivationRestriction {
        target: String,
    },
    ChooseRequiredHearts,
    ModifyLimit {
        operation: String,
        count: u32,
    },
    SetBladeCount {
        value: u32,
        target: String,
    },
    DoNothing,
    SetRequiredHearts {
        count: u32,
        heart_color: String,
        target: String,
    },
    SetScore {
        value: u32,
        target: String,
    },
    SpecifyHeartColor {
        choice: bool,
        target: String,
    },
    ModifyRequiredHeartsSuccess {
        operation: String,
        value: u32,
        target: String,
        card_type: Option<String>,
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
        target: String,
    },
    Shuffle {
        target: String,
        source: String,
    },
    RevealPerGroup {
        source: String,
        count: u32,
        target: String,
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
        target: String,
        card_type: Option<String>,
    },
}

impl Effect {
    pub fn from_ability_effect(effect: &AbilityEffect) -> Self {
        match effect.action.as_str() {
            "sequential" => {
                let actions = effect.actions.as_ref().map(|a| a.iter().map(Self::from_ability_effect).collect()).unwrap_or_default();
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
            "discard_card" => Effect::MoveCards {
                count: effect.count.unwrap_or(1),
                source: "hand".to_string(),
                destination: "discard".to_string(),
                target: effect.target.as_deref().unwrap_or("self").to_string(),
                card_type: effect.card_type.clone(),
                group_name: effect.group.as_ref().map(|g| g.name.clone()),
                cost_limit: effect.cost_limit,
                self_cost: false,
                exclude_self: false,
                position: None,
                placement_order: None,
            },
            "draw" | "draw_card" => Effect::Draw {
                count: effect.count.unwrap_or(1),
                target: effect.target.as_deref().unwrap_or("self").to_string(),
                source: effect.source.as_deref().unwrap_or("deck").to_string(),
                destination: effect.destination.as_deref().unwrap_or("hand").to_string(),
                card_type: effect.card_type.clone(),
                per_unit: effect.per_unit.unwrap_or(false),
                per_unit_count: effect.per_unit_count.unwrap_or(1),
                per_unit_type: effect.per_unit_type.clone(),
            },
            "draw_until_count" => Effect::DrawUntilCount {
                count: effect.count.unwrap_or(1),
                target: effect.target.as_deref().unwrap_or("self").to_string(),
                destination: effect.destination.as_deref().unwrap_or("hand").to_string(),
            },
            "move_cards" => Effect::MoveCards {
                count: effect.count.unwrap_or(1),
                source: effect.source.as_deref().unwrap_or_default().to_string(),
                destination: effect.destination.as_deref().unwrap_or_default().to_string(),
                target: effect.target.as_deref().unwrap_or("self").to_string(),
                card_type: effect.card_type.clone(),
                group_name: effect.group.as_ref().map(|g| g.name.clone()),
                cost_limit: effect.cost_limit,
                self_cost: effect.self_cost.unwrap_or(false),
                exclude_self: effect.exclude_self.unwrap_or(false),
                position: effect.position.clone(),
                placement_order: effect.placement_order.clone(),
            },
            "gain_resource" => Effect::GainResource {
                resource: effect.resource.as_deref().unwrap_or_default().to_string(),
                count: effect.resource_icon_count.unwrap_or(effect.count.unwrap_or(1)),
                target: effect.target.as_deref().unwrap_or("self").to_string(),
                duration: effect.duration.clone(),
                card_type: effect.card_type.clone(),
                group_name: effect.group.as_ref().map(|g| g.name.clone()),
                per_unit: effect.per_unit.unwrap_or(false),
                per_unit_count: effect.per_unit_count.unwrap_or(1),
                per_unit_type: effect.per_unit_type.clone(),
                heart_color: effect.heart_color.clone(),
                heart_colors: effect.heart_colors.clone(),
                resource_icon_count: effect.resource_icon_count,
            },
            "change_state" => Effect::ChangeState {
                state_change: effect.state_change.as_deref().unwrap_or_default().to_string(),
                target: effect.target.as_deref().unwrap_or("self").to_string(),
                count: effect.count.unwrap_or(1),
                card_type: effect.card_type.clone(),
                cost_limit: effect.cost_limit,
                optional: effect.optional.unwrap_or(false),
                group_name: effect.group.as_ref().map(|g| g.name.clone()),
                self_cost: effect.self_cost.unwrap_or(false),
                source: effect.source.clone(),
                destination: effect.destination.clone(),
            },
            "modify_score" => Effect::ModifyScore {
                operation: effect.operation.as_deref().unwrap_or("add").to_string(),
                value: effect.value.unwrap_or(0),
                target: effect.target.as_deref().unwrap_or("self").to_string(),
                duration: effect.duration.clone(),
                card_type: effect.card_type.clone(),
                group_name: effect.group.as_ref().map(|g| g.name.clone()),
                per_unit: effect.per_unit.unwrap_or(false),
                per_unit_count: effect.per_unit_count.unwrap_or(1),
                per_unit_type: effect.per_unit_type.clone(),
                effect_constraint: effect.effect_constraint.clone(),
            },
            "modify_required_hearts" => Effect::ModifyRequiredHearts {
                operation: effect.operation.as_deref().unwrap_or("decrease").to_string(),
                value: effect.value.unwrap_or(0),
                heart_color: effect.heart_color.as_deref().unwrap_or("heart00").to_string(),
                target: effect.target.as_deref().unwrap_or("self").to_string(),
            },
            "set_cost" => Effect::SetCost {
                value: effect.value.unwrap_or(0),
                target: effect.target.as_deref().unwrap_or("self").to_string(),
                card_type: effect.card_type.clone(),
            },
            "set_blade_type" => Effect::SetBladeType {
                blade_type: effect.blade_type.clone(),
                target: effect.target.as_deref().unwrap_or("self").to_string(),
                duration: effect.duration.clone(),
            },
            "set_heart_type" => Effect::SetHeartType {
                heart_type: effect.heart_type.as_deref().or(effect.heart_color.as_deref()).map(|s| s.to_string()),
                target: effect.target.as_deref().unwrap_or("self").to_string(),
                count: effect.count.unwrap_or(1),
            },
            "activate_ability" => Effect::ActivateAbility {
                ability_text: effect.ability_text.as_deref().unwrap_or_default().to_string(),
            },
            "invalidate_ability" => Effect::InvalidateAbility,
            "gain_ability" => Effect::GainAbility {
                ability_text: effect.ability_gain.as_deref().unwrap_or_default().to_string(),
                target: effect.target.as_deref().unwrap_or("self").to_string(),
                duration: effect.duration.clone(),
            },
            "play_baton_touch" => Effect::PlayBatonTouch {
                count: effect.count.unwrap_or(1),
                target: effect.target.as_deref().unwrap_or("self").to_string(),
            },
            "reveal" => Effect::Reveal {
                source: effect.source.as_deref().unwrap_or("hand").to_string(),
                count: effect.count.unwrap_or(1),
                target: effect.target.as_deref().unwrap_or("self").to_string(),
                card_type: effect.card_type.clone(),
                heart_colors: effect.heart_colors.clone(),
            },
            "select" => Effect::Select {
                source: effect.source.as_deref().unwrap_or("hand").to_string(),
                count: effect.count.unwrap_or(1),
                target: effect.target.as_deref().unwrap_or("self").to_string(),
                card_type: effect.card_type.clone(),
                distinct: effect.distinct.clone(),
                heart_colors: effect.heart_colors.clone(),
            },
            "look_at" => Effect::LookAt {
                count: effect.count.unwrap_or(1),
                target: effect.target.as_deref().unwrap_or("self").to_string(),
                source: effect.source.as_deref().unwrap_or("deck").to_string(),
            },
            "modify_required_hearts_global" => Effect::ModifyRequiredHeartsGlobal {
                operation: effect.operation.as_deref().unwrap_or("increase").to_string(),
                value: effect.value.unwrap_or(1),
                heart_color: effect.heart_color.as_deref().unwrap_or("heart00").to_string(),
                target: effect.target.as_deref().unwrap_or("opponent").to_string(),
            },
            "modify_yell_count" => Effect::ModifyYellCount {
                operation: effect.operation.as_deref().unwrap_or("subtract").to_string(),
                count: effect.count.unwrap_or(0),
            },
            "place_energy_under_member" => Effect::PlaceEnergyUnderMember {
                count: effect.count.unwrap_or(1),
                target: effect.target.as_deref().unwrap_or("self").to_string(),
                position: effect.position.clone(),
            },
            "activation_cost" => Effect::ActivationCost {
                operation: effect.operation.as_deref().unwrap_or("increase").to_string(),
                value: effect.value.unwrap_or(0),
                target: effect.target.as_deref().unwrap_or("self").to_string(),
                duration: effect.duration.clone(),
            },
            "position_change" => Effect::PositionChange {
                position: effect.position.clone(),
                target: effect.target.as_deref().unwrap_or("self").to_string(),
                target_member: effect.target_member.as_deref().unwrap_or("this_member").to_string(),
            },
            "appear" => Effect::Appear {
                source: effect.source.as_deref().unwrap_or_default().to_string(),
                destination: effect.destination.as_deref().unwrap_or("stage").to_string(),
                count: effect.count.unwrap_or(1),
                target: effect.target.as_deref().unwrap_or("self").to_string(),
                card_type: effect.card_type.clone(),
            },
            "choice" => Effect::Choice {
                choice_options: effect.choice_options.clone(),
                choice_type: effect.choice_type.clone(),
                options: effect.options.clone(),
            },
            "pay_energy" => Effect::PayEnergy {
                count: effect.count.unwrap_or(0),
                target: effect.target.as_deref().unwrap_or("self").to_string(),
            },
            "set_card_identity" => Effect::SetCardIdentity {
                identities: effect.identities.clone().unwrap_or_default(),
            },
            "repeat_procedure" => {
                let actions = effect.actions.as_ref().map(|a| a.iter().map(Self::from_ability_effect).collect()).unwrap_or_default();
                Effect::RepeatProcedure {
                    repeat_limit: effect.repeat_limit.unwrap_or(1),
                    actions,
                }
            }
            "discard_until_count" => Effect::DiscardUntilCount {
                target_count: effect.target_count.unwrap_or(0),
                target: effect.target.as_deref().unwrap_or("self").to_string(),
            },
            "restriction" => Effect::Restriction {
                restriction_type: effect.restriction_type.clone(),
                restricted_destination: effect.restricted_destination.clone(),
            },
            "re_yell" => Effect::ReYell {
                lose_blade_hearts: effect.lose_blade_hearts.unwrap_or(false),
                target: effect.target.as_deref().unwrap_or("self").to_string(),
            },
            "activation_restriction" => Effect::ActivationRestriction {
                target: effect.target.as_deref().unwrap_or("self").to_string(),
            },
            "choose_required_hearts" => Effect::ChooseRequiredHearts,
            "modify_limit" => Effect::ModifyLimit {
                operation: effect.operation.as_deref().unwrap_or("decrease").to_string(),
                count: effect.count.unwrap_or(0),
            },
            "set_blade_count" => Effect::SetBladeCount {
                value: effect.value.unwrap_or(0),
                target: effect.target.as_deref().unwrap_or("self").to_string(),
            },
            "do_nothing" => Effect::DoNothing,
            "set_required_hearts" => Effect::SetRequiredHearts {
                count: effect.count.unwrap_or(0),
                heart_color: effect.heart_color.as_deref().unwrap_or("heart00").to_string(),
                target: effect.target.as_deref().unwrap_or("self").to_string(),
            },
            "set_score" => Effect::SetScore {
                value: effect.value.unwrap_or(0),
                target: effect.target.as_deref().unwrap_or("self").to_string(),
            },
            "specify_heart_color" => Effect::SpecifyHeartColor {
                choice: effect.choice.unwrap_or(false),
                target: effect.target.as_deref().unwrap_or("self").to_string(),
            },
            "modify_required_hearts_success" => Effect::ModifyRequiredHeartsSuccess {
                operation: effect.operation.as_deref().unwrap_or("increase").to_string(),
                value: effect.value.unwrap_or(0),
                target: effect.target.as_deref().unwrap_or("self").to_string(),
                card_type: effect.card_type.clone(),
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
                target: effect.target.as_deref().unwrap_or("self").to_string(),
            },
            "shuffle" => Effect::Shuffle {
                target: effect.target.as_deref().unwrap_or("self").to_string(),
                source: effect.source.as_deref().unwrap_or("deck").to_string(),
            },
            "reveal_per_group" => Effect::RevealPerGroup {
                source: effect.source.as_deref().unwrap_or("hand").to_string(),
                count: effect.count.unwrap_or(1),
                target: effect.target.as_deref().unwrap_or("self").to_string(),
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
                target: effect.target.as_deref().unwrap_or("self").to_string(),
                card_type: effect.card_type.clone(),
            },
            "custom" => Effect::DoNothing,
            _ => {
                eprintln!("Unknown action: '{}', treating as DoNothing", effect.action);
                Effect::DoNothing
            }
        }
    }
}
