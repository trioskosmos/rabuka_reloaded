#[derive(Debug, Clone)]
pub struct RuleConfig {
    pub partial_resolution_allowed: bool,
    pub full_cost_payment_required: bool,
    pub auto_abilities_mandatory: bool,
    pub search_count_adjustment_enabled: bool,
    pub allow_replacement_placement: bool,
    pub allow_live_without_stage_members: bool,
    pub prohibition_precedence_enabled: bool,
    pub card_set_search_enabled: bool,
    pub multi_victory_selection_enabled: bool,
    pub turn_player_priority_enabled: bool,
    pub arbitrary_actions_restricted: bool,
    pub optional_cost_behavior: String,
    pub effect_resumption_state: String,
}

impl Default for RuleConfig {
    fn default() -> Self {
        Self {
            partial_resolution_allowed: true,
            full_cost_payment_required: true,
            auto_abilities_mandatory: true,
            search_count_adjustment_enabled: true,
            allow_replacement_placement: true,
            allow_live_without_stage_members: true,
            prohibition_precedence_enabled: true,
            card_set_search_enabled: true,
            multi_victory_selection_enabled: true,
            turn_player_priority_enabled: true,
            arbitrary_actions_restricted: true,
            optional_cost_behavior: "always_pay".to_string(),
            effect_resumption_state: "none".to_string(),
        }
    }
}
