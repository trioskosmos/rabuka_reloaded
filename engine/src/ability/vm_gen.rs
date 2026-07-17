// Auto-generated
fn default_condition_abilityFilter() -> Condition {
    Condition::AbilityFilter {
        ability_filter: Default::default(),
        ability_filter_triggers: Default::default(),
        cache: Default::default(),
        count: Default::default(),
        location: Default::default(),
        negation: Default::default(),
        operator: Default::default(),
        phase: Default::default(),
        phase_target: Default::default(),
        target: Default::default(),
        text: Default::default(),
        trigger_event: Default::default(),
    }
}

fn default_condition_allRevealedMatchHeartColor() -> Condition {
    Condition::AllRevealedMatchHeartColor {
        cache: Default::default(),
        count: Default::default(),
        negation: Default::default(),
        operator: Default::default(),
        phase: Default::default(),
        phase_target: Default::default(),
        text: Default::default(),
        trigger_event: Default::default(),
    }
}

fn default_condition_alwaysTrue() -> Condition {
    Condition::AlwaysTrue {
        cache: Default::default(),
        negation: Default::default(),
        phase: Default::default(),
        phase_target: Default::default(),
        text: Default::default(),
        trigger_event: Default::default(),
    }
}

fn default_condition_anyOf() -> Condition {
    Condition::AnyOf {
        any_of: Default::default(),
        cache: Default::default(),
        negation: Default::default(),
        phase: Default::default(),
        phase_target: Default::default(),
        text: Default::default(),
        trigger_event: Default::default(),
    }
}

fn default_condition_appearance() -> Condition {
    Condition::Appearance {
        activation_position: Default::default(),
        all_areas: Default::default(),
        appearance: Default::default(),
        appearance_source: Default::default(),
        baton_touch_trigger: Default::default(),
        cache: Default::default(),
        card_property: Default::default(),
        card_type: Default::default(),
        characters: Default::default(),
        cost_limit: Default::default(),
        cost_reference_character: Default::default(),
        cost_reference_operator: Default::default(),
        exclude_self: Default::default(),
        group_names: Default::default(),
        location: Default::default(),
        min_baton_touch_count: Default::default(),
        negation: Default::default(),
        operator: Default::default(),
        phase: Default::default(),
        phase_target: Default::default(),
        position: Default::default(),
        position_compare: Default::default(),
        positions_characters: Default::default(),
        target: Default::default(),
        text: Default::default(),
        trigger_event: Default::default(),
    }
}

fn default_condition_choice() -> Condition {
    Condition::Choice {
        cache: Default::default(),
        negation: Default::default(),
        options: Default::default(),
        phase: Default::default(),
        phase_target: Default::default(),
        text: Default::default(),
        trigger_event: Default::default(),
    }
}

fn default_condition_comparison() -> Condition {
    Condition::Comparison {
        ability_filter: Default::default(),
        ability_filter_triggers: Default::default(),
        aggregate: Default::default(),
        all: Default::default(),
        all_areas: Default::default(),
        baton_touch_trigger: Default::default(),
        cache: Default::default(),
        card_property: Default::default(),
        card_type: Default::default(),
        characters: Default::default(),
        comparison_source: Default::default(),
        comparison_target: Default::default(),
        comparison_type: Default::default(),
        cost_limit: Default::default(),
        cost_total: Default::default(),
        cost_total_operator: Default::default(),
        count: Default::default(),
        delta: Default::default(),
        destination: Default::default(),
        distinct: Default::default(),
        exclude_characters: Default::default(),
        exclude_group_names: Default::default(),
        exclude_self: Default::default(),
        from_state: Default::default(),
        group_names: Default::default(),
        heart_colors: Default::default(),
        location: Default::default(),
        locations: Default::default(),
        min_baton_touch_count: Default::default(),
        negation: Default::default(),
        no_excess_heart: Default::default(),
        operator: Default::default(),
        phase: Default::default(),
        phase_target: Default::default(),
        position: Default::default(),
        position_compare: Default::default(),
        require_position_cards: Default::default(),
        resource_type: Default::default(),
        same_name: Default::default(),
        scope: Default::default(),
        self_target: Default::default(),
        source: Default::default(),
        state: Default::default(),
        target: Default::default(),
        temporal: Default::default(),
        text: Default::default(),
        to_state: Default::default(),
        trigger_event: Default::default(),
        values: Default::default(),
        yell_trigger: Default::default(),
    }
}

fn default_condition_complex() -> Condition {
    Condition::Complex {
        cache: Default::default(),
        cause: Default::default(),
        effect: Default::default(),
        negation: Default::default(),
        phase: Default::default(),
        phase_target: Default::default(),
        text: Default::default(),
        trigger_event: Default::default(),
    }
}

fn default_condition_compound() -> Condition {
    Condition::Compound {
        cache: Default::default(),
        conditions: Default::default(),
        negation: Default::default(),
        operator: Default::default(),
        phase: Default::default(),
        phase_target: Default::default(),
        target: Default::default(),
        text: Default::default(),
        trigger_event: Default::default(),
    }
}

fn default_condition_group() -> Condition {
    Condition::Group {
        aggregate: Default::default(),
        all_members: Default::default(),
        cache: Default::default(),
        card_type: Default::default(),
        count: Default::default(),
        exclude_characters: Default::default(),
        exclude_self: Default::default(),
        group_names: Default::default(),
        heart_colors: Default::default(),
        heart_source: Default::default(),
        location: Default::default(),
        locations: Default::default(),
        negation: Default::default(),
        operator: Default::default(),
        phase: Default::default(),
        phase_target: Default::default(),
        position: Default::default(),
        self_target: Default::default(),
        source: Default::default(),
        target: Default::default(),
        temporal: Default::default(),
        text: Default::default(),
        trigger_event: Default::default(),
    }
}

fn default_condition_location() -> Condition {
    Condition::Location {
        activation_position: Default::default(),
        aggregate: Default::default(),
        all: Default::default(),
        all_areas: Default::default(),
        baton_touch_trigger: Default::default(),
        cache: Default::default(),
        card_property: Default::default(),
        card_type: Default::default(),
        characters: Default::default(),
        comparison_target: Default::default(),
        comparison_type: Default::default(),
        cost_limit: Default::default(),
        cost_limit_operator: Default::default(),
        count: Default::default(),
        destination: Default::default(),
        distinct: Default::default(),
        exclude_characters: Default::default(),
        exclude_group_names: Default::default(),
        exclude_self: Default::default(),
        group_names: Default::default(),
        group_reference: Default::default(),
        heart_colors: Default::default(),
        heart_source: Default::default(),
        heart_type: Default::default(),
        location: Default::default(),
        locations: Default::default(),
        min_baton_touch_count: Default::default(),
        negation: Default::default(),
        operator: Default::default(),
        phase: Default::default(),
        phase_target: Default::default(),
        position: Default::default(),
        position_compare: Default::default(),
        require_position_cards: Default::default(),
        same_name: Default::default(),
        scope: Default::default(),
        self_target: Default::default(),
        source: Default::default(),
        state: Default::default(),
        sub_checks: Default::default(),
        target: Default::default(),
        temporal: Default::default(),
        text: Default::default(),
        trigger_event: Default::default(),
        unit: Default::default(),
        yell_trigger: Default::default(),
    }
}

fn default_condition_movement() -> Condition {
    Condition::Movement {
        ability_filter: Default::default(),
        area_direction: Default::default(),
        baton_touch_source: Default::default(),
        baton_touch_trigger: Default::default(),
        cache: Default::default(),
        card_property: Default::default(),
        card_type: Default::default(),
        characters: Default::default(),
        comparison_type: Default::default(),
        cost_limit: Default::default(),
        cost_limit_operator: Default::default(),
        destination: Default::default(),
        energy_placed: Default::default(),
        exclude_self: Default::default(),
        from_state: Default::default(),
        group_names: Default::default(),
        location: Default::default(),
        min_baton_touch_count: Default::default(),
        movement: Default::default(),
        negation: Default::default(),
        operator: Default::default(),
        phase: Default::default(),
        phase_target: Default::default(),
        position: Default::default(),
        self_effect_only: Default::default(),
        self_target: Default::default(),
        source: Default::default(),
        target: Default::default(),
        text: Default::default(),
        to_state: Default::default(),
        trigger_event: Default::default(),
    }
}

fn default_condition_noExcessHeart() -> Condition {
    Condition::NoExcessHeart {
        cache: Default::default(),
        negation: Default::default(),
        phase: Default::default(),
        phase_target: Default::default(),
        target: Default::default(),
        text: Default::default(),
        trigger_event: Default::default(),
    }
}

fn default_condition_opponentChoice() -> Condition {
    Condition::OpponentChoice {
        cache: Default::default(),
        negation: Default::default(),
        phase: Default::default(),
        phase_target: Default::default(),
        target: Default::default(),
        text: Default::default(),
        trigger_event: Default::default(),
    }
}

fn default_condition_opponentLiveSuccess() -> Condition {
    Condition::OpponentLiveSuccess {
        cache: Default::default(),
        negation: Default::default(),
        no_excess_heart: Default::default(),
        phase: Default::default(),
        phase_target: Default::default(),
        text: Default::default(),
        trigger_event: Default::default(),
    }
}

fn default_condition_positionCond() -> Condition {
    Condition::PositionCond {
        cache: Default::default(),
        negation: Default::default(),
        phase: Default::default(),
        phase_target: Default::default(),
        position: Default::default(),
        target: Default::default(),
        text: Default::default(),
        trigger_event: Default::default(),
    }
}

fn default_condition_resource() -> Condition {
    Condition::Resource {
        cache: Default::default(),
        count: Default::default(),
        delta: Default::default(),
        heart_colors: Default::default(),
        location: Default::default(),
        negation: Default::default(),
        operator: Default::default(),
        phase: Default::default(),
        phase_target: Default::default(),
        position: Default::default(),
        resource_type: Default::default(),
        source: Default::default(),
        target: Default::default(),
        text: Default::default(),
        trigger_event: Default::default(),
    }
}

fn default_condition_scoreThreshold() -> Condition {
    Condition::ScoreThreshold {
        cache: Default::default(),
        count: Default::default(),
        negation: Default::default(),
        operator: Default::default(),
        phase: Default::default(),
        phase_target: Default::default(),
        target: Default::default(),
        text: Default::default(),
        trigger_event: Default::default(),
    }
}

fn default_condition_state() -> Condition {
    Condition::State {
        all: Default::default(),
        cache: Default::default(),
        card_type: Default::default(),
        characters: Default::default(),
        cost_limit: Default::default(),
        cost_limit_operator: Default::default(),
        count: Default::default(),
        energy_state: Default::default(),
        from_state: Default::default(),
        group_names: Default::default(),
        negation: Default::default(),
        operator: Default::default(),
        phase: Default::default(),
        phase_target: Default::default(),
        resource_type: Default::default(),
        state: Default::default(),
        target: Default::default(),
        text: Default::default(),
        to_state: Default::default(),
        trigger_event: Default::default(),
    }
}

fn default_condition_temporal() -> Condition {
    Condition::Temporal {
        aggregate: Default::default(),
        cache: Default::default(),
        card_type: Default::default(),
        condition: Default::default(),
        count: Default::default(),
        group_names: Default::default(),
        heart_colors: Default::default(),
        location: Default::default(),
        locations: Default::default(),
        negation: Default::default(),
        phase: Default::default(),
        phase_target: Default::default(),
        position: Default::default(),
        self_target: Default::default(),
        target: Default::default(),
        temporal: Default::default(),
        temporal_scope: Default::default(),
        text: Default::default(),
        trigger_event: Default::default(),
        turn_number: Default::default(),
    }
}

fn default_abilityOp() -> EffectKind {
    EffectKind::AbilityOp {
        ability_gain: Default::default(),
        ability_gain_trigger: Default::default(),
        ability_text: Default::default(),
        activation_condition_parsed: None,
        activation_position: Default::default(),
        all: Default::default(),
        card_type: Default::default(),
        characters: None,
        cost_limit: Default::default(),
        cost_limit_operator: Default::default(),
        destination: Default::default(),
        duration: Default::default(),
        dynamic_count: Default::default(),
        effect_type: Default::default(),
        exclude_characters: None,
        exclude_group_names: None,
        exclude_self: Default::default(),
        gained_effect: None,
        group_names: None,
        heart_colors: Box::default(),
        location: Default::default(),
        option: Default::default(),
        self_target: Default::default(),
        source: Default::default(),
        source_card: Default::default(),
        suppressed_trigger: Default::default(),
        target: Default::default(),
        target_trigger: Default::default(),
        trigger_filter: None,
        trigger_type: Default::default(),
        triggers: Default::default(),
        use_limit: Default::default(),
    }
}

fn default_changeState() -> EffectKind {
    EffectKind::ChangeState {
        ability_filter: Default::default(),
        ability_filter_triggers: Default::default(),
        action_by: Default::default(),
        activation_condition_parsed: None,
        activation_position: Default::default(),
        all: Default::default(),
        all_regions: Default::default(),
        blade_limit: Default::default(),
        blade_limit_operator: Default::default(),
        card_names: Box::default(),
        card_property: Default::default(),
        card_type: Default::default(),
        characters: None,
        cost_from_revealed: Default::default(),
        cost_limit: Default::default(),
        cost_limit_operator: Default::default(),
        cost_total: Default::default(),
        cost_total_operator: Default::default(),
        destination: Default::default(),
        distinct: Default::default(),
        exclude_characters: None,
        exclude_group_names: None,
        exclude_heart_colors: None,
        exclude_self: Default::default(),
        filter_targets_by_heart_colors: Default::default(),
        group_names: None,
        group_reference: Default::default(),
        heart_colors: Box::default(),
        identities: None,
        location: Default::default(),
        name_constraint: Default::default(),
        name_constraint_source: Default::default(),
        negation: Default::default(),
        optional: Default::default(),
        or_ability_filters: Default::default(),
        original_value: Default::default(),
        per_unit: Default::default(),
        per_unit_count: Default::default(),
        per_unit_heart_colors: Box::default(),
        per_unit_location: Default::default(),
        per_unit_type: Default::default(),
        position: Default::default(),
        self_cost: Default::default(),
        self_target: Default::default(),
        source: Default::default(),
        state: Default::default(),
        state_change: Default::default(),
        target: Default::default(),
    }
}

fn default_compoundEffect() -> EffectKind {
    EffectKind::CompoundEffect {
        activation_condition_parsed: None,
        activation_position: Default::default(),
        all: Default::default(),
        alternative_count_type: Default::default(),
        alternative_effect: None,
        answers: None,
        card_type: Default::default(),
        choice_maker: Default::default(),
        choice_options: None,
        choice_type: Default::default(),
        destination: Default::default(),
        distinct: Default::default(),
        duration: Default::default(),
        exclude_self: Default::default(),
        group_names: None,
        group_reference: Default::default(),
        heart_colors: Box::default(),
        optional: Default::default(),
        options: Default::default(),
        original_value: Default::default(),
        per_unit: Default::default(),
        per_unit_count: Default::default(),
        per_unit_type: Default::default(),
        position: Default::default(),
        question: Default::default(),
        repeat_limit: Default::default(),
        shuffle: Default::default(),
        source: Default::default(),
        target: Default::default(),
        target_count: Default::default(),
        trigger_type: Default::default(),
    }
}

fn default_customOp() -> EffectKind {
    EffectKind::CustomOp {
        action_by: Default::default(),
        activation_condition_parsed: None,
        all_regions: Default::default(),
        answers: None,
        card_type: Default::default(),
        characters: None,
        choice_based: Default::default(),
        choice_maker: Default::default(),
        duration: Default::default(),
        effect_type: Default::default(),
        exclude_characters: None,
        exclude_group_names: None,
        exclude_self: Default::default(),
        group_names: None,
        identities: None,
        location: Default::default(),
        opponent_action: None,
        options: Default::default(),
        original_value: Default::default(),
        question: Default::default(),
        replaces_event: Default::default(),
        self_target: Default::default(),
        timing: Default::default(),
        treat_as: Default::default(),
        trigger_filter: None,
        trigger_type: Default::default(),
        triggers: Default::default(),
        use_limit: Default::default(),
    }
}

fn default_drawCards() -> EffectKind {
    EffectKind::DrawCards {
        action_by: Default::default(),
        card_names: Box::default(),
        card_type: Default::default(),
        destination: Default::default(),
        dynamic_count: Default::default(),
        exclude_self: Default::default(),
        heart_colors: Box::default(),
        location: Default::default(),
        original_value: Default::default(),
        per_unit: Default::default(),
        per_unit_count: Default::default(),
        per_unit_heart_colors: Box::default(),
        per_unit_location: Default::default(),
        per_unit_type: Default::default(),
        position: Default::default(),
        source: Default::default(),
        state: Default::default(),
        target: Default::default(),
        target_count: Default::default(),
        trigger_type: Default::default(),
    }
}

fn default_gainResource() -> EffectKind {
    EffectKind::GainResource {
        action_by: Default::default(),
        activation_condition_parsed: None,
        activation_position: Default::default(),
        all: Default::default(),
        any_number: Default::default(),
        card_property: Default::default(),
        card_type: Default::default(),
        characters: None,
        cost_limit: Default::default(),
        cost_limit_operator: Default::default(),
        distinct: Default::default(),
        duration: Default::default(),
        dynamic_count: Default::default(),
        energy_count: Default::default(),
        exclude_characters: None,
        exclude_group_names: None,
        exclude_self: Default::default(),
        filter_targets_by_heart_colors: Default::default(),
        group_names: None,
        group_reference: Default::default(),
        heart_color: Default::default(),
        heart_color_count: Default::default(),
        heart_colors: Box::default(),
        heart_colors_from_selected_card: Default::default(),
        heart_type: Default::default(),
        location: Default::default(),
        multiple_targets: Default::default(),
        negation: Default::default(),
        operation: Default::default(),
        optional: Default::default(),
        original_value: Default::default(),
        per_unit: Default::default(),
        per_unit_count: Default::default(),
        per_unit_heart_colors: Box::default(),
        per_unit_location: Default::default(),
        per_unit_type: Default::default(),
        position: Default::default(),
        repeat_limit: Default::default(),
        require_all_heart_colors: Default::default(),
        resource: Default::default(),
        same_name: Default::default(),
        self_target: Default::default(),
        sign: Default::default(),
        state: Default::default(),
        target_count: Default::default(),
        target_from_selection: Default::default(),
        timing_condition: Default::default(),
        trigger_type: Default::default(),
        value: Default::default(),
    }
}

fn default_lookReveal() -> EffectKind {
    EffectKind::LookReveal {
        ability_filter: Default::default(),
        ability_filter_triggers: Default::default(),
        activation_position: Default::default(),
        blind: Default::default(),
        card_names: Box::default(),
        card_property: Default::default(),
        card_type: Default::default(),
        characters: None,
        cost_limit: Default::default(),
        cost_limit_max: Default::default(),
        cost_limit_min: Default::default(),
        cost_limit_operator: Default::default(),
        destination: Default::default(),
        distinct: Default::default(),
        dynamic_count: Default::default(),
        exclude_characters: None,
        exclude_group_names: None,
        exclude_self: Default::default(),
        filter_targets_by_heart_colors: Default::default(),
        group_names: None,
        group_reference: Default::default(),
        heart_color_count: Default::default(),
        heart_colors: Box::default(),
        is_reveal: Default::default(),
        location: Default::default(),
        multiple_targets: Default::default(),
        name_constraint: Default::default(),
        name_constraint_source: Default::default(),
        negation: Default::default(),
        optional: Default::default(),
        options: Default::default(),
        or_ability_filters: Default::default(),
        original_value: Default::default(),
        per_unit: Default::default(),
        per_unit_count: Default::default(),
        per_unit_heart_colors: Box::default(),
        per_unit_location: Default::default(),
        per_unit_type: Default::default(),
        picker: Default::default(),
        require_all_heart_colors: Default::default(),
        resource_on_select: None,
        reveal: Default::default(),
        self_target: Default::default(),
        source: Default::default(),
        state: Default::default(),
        target: Default::default(),
    }
}

fn default_miscOp() -> EffectKind {
    EffectKind::MiscOp {
        ability_filter: Default::default(),
        activation_position: Default::default(),
        all: Default::default(),
        all_regions: Default::default(),
        alternative_count_type: Default::default(),
        blade_limit: Default::default(),
        blade_limit_operator: Default::default(),
        blade_type: Default::default(),
        blind: Default::default(),
        card_names: Box::default(),
        card_type: Default::default(),
        character_effects: None,
        characters: None,
        choice: Default::default(),
        cost_limit: Default::default(),
        cost_limit_operator: Default::default(),
        cost_offset: Default::default(),
        cost_reference: Default::default(),
        cost_total: Default::default(),
        cost_total_operator: Default::default(),
        destination: Default::default(),
        duration: Default::default(),
        dynamic_count: Default::default(),
        effect_constraint: Default::default(),
        energy_count: Default::default(),
        exclude_characters: None,
        exclude_group_names: None,
        exclude_self: Default::default(),
        group_names: None,
        group_reference: Default::default(),
        heart_color_count: Default::default(),
        heart_colors: Box::default(),
        heart_selection: Default::default(),
        heart_type: Default::default(),
        id: Default::default(),
        identities: None,
        location: Default::default(),
        lose_blade_hearts: Default::default(),
        negation: Default::default(),
        operation: Default::default(),
        options: Default::default(),
        or_card_types: None,
        original_cost: Default::default(),
        original_count: Default::default(),
        original_operator: Default::default(),
        original_value: Default::default(),
        parenthetical: Default::default(),
        per_group: Default::default(),
        per_group_count: Default::default(),
        per_unit: Default::default(),
        per_unit_count: Default::default(),
        per_unit_heart_colors: Box::default(),
        per_unit_location: Default::default(),
        per_unit_type: Default::default(),
        picker: Default::default(),
        placement_order: Default::default(),
        position: Default::default(),
        quoted_text: Default::default(),
        ref_offset: Default::default(),
        ref_value: Default::default(),
        repeat_limit: Default::default(),
        require_all_heart_colors: Default::default(),
        resource_icon_count: Default::default(),
        same_unit_name: Default::default(),
        self_target: Default::default(),
        sign: Default::default(),
        source: Default::default(),
        target: Default::default(),
        target_count: Default::default(),
        timing: Default::default(),
        treat_as: Default::default(),
        value: Default::default(),
    }
}

fn default_modifyHearts() -> EffectKind {
    EffectKind::ModifyHearts {
        all: Default::default(),
        card_type: Default::default(),
        cost_total: Default::default(),
        cost_total_operator: Default::default(),
        distinct: Default::default(),
        duration: Default::default(),
        exclude_heart_colors: None,
        exclude_self: Default::default(),
        filter_targets_by_heart_colors: Default::default(),
        group_names: None,
        group_reference: Default::default(),
        heart_colors: Box::default(),
        location: Default::default(),
        negation: Default::default(),
        operation: Default::default(),
        original_count: Default::default(),
        original_operator: Default::default(),
        original_value: Default::default(),
        per_unit: Default::default(),
        per_unit_count: Default::default(),
        per_unit_heart_colors: Box::default(),
        per_unit_type: Default::default(),
        position: Default::default(),
        repeat_limit: Default::default(),
        replace_all: Default::default(),
        self_target: Default::default(),
        target_count: Default::default(),
        timing_condition: Default::default(),
        value: Default::default(),
    }
}

fn default_modifyScore() -> EffectKind {
    EffectKind::ModifyScore {
        activation_position: Default::default(),
        card_names: Box::default(),
        card_property: Default::default(),
        card_type: Default::default(),
        cost_total: Default::default(),
        cost_total_operator: Default::default(),
        destination: Default::default(),
        distinct: Default::default(),
        duration: Default::default(),
        effect_constraint: Default::default(),
        exclude_self: Default::default(),
        filter_targets_by_heart_colors: Default::default(),
        group_names: None,
        heart_colors: Box::default(),
        location: Default::default(),
        max_repeats: Default::default(),
        need_heart_operator: Default::default(),
        need_heart_total: Default::default(),
        negation: Default::default(),
        operation: Default::default(),
        per_unit: Default::default(),
        per_unit_count: Default::default(),
        per_unit_heart_colors: Box::default(),
        per_unit_location: Default::default(),
        per_unit_type: Default::default(),
        position: Default::default(),
        repeat_limit: Default::default(),
        self_target: Default::default(),
        source: Default::default(),
        state: Default::default(),
        target: Default::default(),
        target_count: Default::default(),
        value: Default::default(),
    }
}

fn default_moveCards() -> EffectKind {
    EffectKind::MoveCards {
        ability_filter: Default::default(),
        ability_filter_triggers: Default::default(),
        action_by: Default::default(),
        activation_condition_parsed: None,
        activation_position: Default::default(),
        all: Default::default(),
        allow_occupied_stage: Default::default(),
        any_number: Default::default(),
        baton_touch_trigger: Default::default(),
        card_names: Box::default(),
        card_property: Default::default(),
        card_type: Default::default(),
        characters: None,
        cost_from_revealed: Default::default(),
        cost_limit: Default::default(),
        cost_limit_max: Default::default(),
        cost_limit_min: Default::default(),
        cost_limit_operator: Default::default(),
        cost_offset: Default::default(),
        cost_reference: Default::default(),
        cost_total: Default::default(),
        cost_total_operator: Default::default(),
        count: Default::default(),
        destination: Default::default(),
        discard_remaining: Default::default(),
        distinct: Default::default(),
        dynamic_count: Default::default(),
        energy_count: Default::default(),
        exclude_by_name_source: Default::default(),
        exclude_characters: None,
        exclude_group_names: None,
        exclude_heart_colors: None,
        exclude_position: Default::default(),
        exclude_selected: Default::default(),
        exclude_self: Default::default(),
        filter_targets_by_heart_colors: Default::default(),
        group_names: None,
        group_reference: Default::default(),
        heart_colors: Box::default(),
        location: Default::default(),
        multiple_targets: Default::default(),
        name_constraint: Default::default(),
        name_constraint_source: Default::default(),
        need_heart_color: Default::default(),
        need_heart_operator: Default::default(),
        need_heart_total: Default::default(),
        negation: Default::default(),
        or_ability_filters: Default::default(),
        or_card_types: None,
        original_value: Default::default(),
        per_group: Default::default(),
        per_group_count: Default::default(),
        placement_order: Default::default(),
        position: Default::default(),
        quoted_text: Default::default(),
        same_unit_name: Default::default(),
        self_cost: Default::default(),
        self_target: Default::default(),
        shuffle: Default::default(),
        source: Default::default(),
        source_position: Default::default(),
        state: Default::default(),
        state_change: Default::default(),
        target: Default::default(),
        target_count: Default::default(),
        target_from_selection: Default::default(),
        target_member: Default::default(),
    }
}

fn default_positionOp() -> EffectKind {
    EffectKind::PositionOp {
        activation_position: Default::default(),
        allow_occupied_stage: Default::default(),
        any_number: Default::default(),
        card_type: Default::default(),
        characters: None,
        cost_from_revealed: Default::default(),
        cost_limit: Default::default(),
        cost_limit_operator: Default::default(),
        destination: Default::default(),
        dynamic_count: Default::default(),
        energy_count: Default::default(),
        exclude_characters: None,
        exclude_group_names: None,
        exclude_position: Default::default(),
        exclude_self: Default::default(),
        group_names: None,
        group_reference: Default::default(),
        multiple_targets: Default::default(),
        optional: Default::default(),
        position: Default::default(),
        self_target: Default::default(),
        source: Default::default(),
        source_position: Default::default(),
        state: Default::default(),
        target: Default::default(),
        target_member: Default::default(),
    }
}

fn default_restrictionOp() -> EffectKind {
    EffectKind::RestrictionOp {
        card_type: Default::default(),
        characters: None,
        choice_based: Default::default(),
        delayed: Default::default(),
        duration: Default::default(),
        effect_type: Default::default(),
        exclude_characters: None,
        exclude_group_names: None,
        exclude_self: Default::default(),
        group_names: None,
        location: Default::default(),
        non_stackable: Default::default(),
        operation: Default::default(),
        phase: Default::default(),
        replaces_event: Default::default(),
        restricted_destination: Default::default(),
        restriction_type: Default::default(),
        self_target: Default::default(),
        timing: Default::default(),
        timing_condition: Default::default(),
        treat_as: Default::default(),
        trigger_filter: None,
        trigger_type: Default::default(),
    }
}

fn default_selectTarget() -> EffectKind {
    EffectKind::SelectTarget {
        ability_filter: Default::default(),
        ability_filter_triggers: Default::default(),
        action_by: Default::default(),
        activation_position: Default::default(),
        answers: None,
        any_number: Default::default(),
        card_names: Box::default(),
        card_property: Default::default(),
        card_type: Default::default(),
        characters: None,
        choice_maker: Default::default(),
        choice_options: None,
        choice_type: Default::default(),
        cost_limit: Default::default(),
        cost_limit_max: Default::default(),
        cost_limit_min: Default::default(),
        cost_limit_operator: Default::default(),
        cost_total: Default::default(),
        cost_total_operator: Default::default(),
        destination: Default::default(),
        discard_remaining: Default::default(),
        distinct: Default::default(),
        exclude_characters: None,
        exclude_group_names: None,
        exclude_selected: Default::default(),
        exclude_self: Default::default(),
        filter_targets_by_heart_colors: Default::default(),
        group_names: None,
        group_reference: Default::default(),
        heart_color_count: Default::default(),
        heart_colors: Box::default(),
        location: Default::default(),
        multiple_targets: Default::default(),
        name_constraint: Default::default(),
        name_constraint_source: Default::default(),
        negation: Default::default(),
        optional: Default::default(),
        options: Default::default(),
        or_ability_filters: Default::default(),
        or_card_types: None,
        original_value: Default::default(),
        per_group: Default::default(),
        per_group_count: Default::default(),
        per_unit: Default::default(),
        per_unit_count: Default::default(),
        per_unit_heart_colors: Box::default(),
        per_unit_location: Default::default(),
        per_unit_type: Default::default(),
        placement_order: Default::default(),
        question: Default::default(),
        require_all_heart_colors: Default::default(),
        reveal: Default::default(),
        self_target: Default::default(),
        source: Default::default(),
        state: Default::default(),
        target: Default::default(),
        target_count: Default::default(),
    }
}

fn decode_effect_kind(op: Opcode, cursor: &mut &[u8]) -> Option<Box<EffectKind>> {
    match op {
        Opcode::ActivateAbility => {
            let ability_text = read_str(cursor);
            let count = read_u8(cursor);
            let group_names = read_str(cursor);
            let optional = read_u8(cursor) != 0;
            let parenthetical = read_str(cursor);
            let source_card = read_str(cursor);
            let target = read_str(cursor);
            let target_trigger = read_str(cursor);
            let mut ek = default_abilityOp();
            if let EffectKind::AbilityOp { ability_text: ref mut _bc_ability_text, group_names: ref mut _bc_group_names, source_card: ref mut _bc_source_card, target: ref mut _bc_target, target_trigger: ref mut _bc_target_trigger, .. } = &mut ek {
                *_bc_ability_text = ability_text.map(|s| s.into());
                *_bc_group_names = group_names.map(|s| Box::new(vec![s.to_string()]));
                *_bc_source_card = source_card.map(|s| s.into());
                *_bc_target = target.map(|s| s.into());
                *_bc_target_trigger = target_trigger.map(|s| s.into());
            }
            Some(Box::new(ek))
        }
        Opcode::ChangeState => {
            let action_by = decode_player(read_u8(cursor));
            let activation_position = decode_zone(read_u8(cursor));
            let all = read_u8(cursor) != 0;
            let blade_limit = read_u8(cursor);
            let blade_limit_operator = decode_operator(read_u8(cursor));
            let card_type = decode_card_type(read_u8(cursor));
            let cost_from_revealed = read_u8(cursor) != 0;
            let cost_limit = read_u8(cursor);
            let cost_limit_operator = decode_operator(read_u8(cursor));
            let count = read_u8(cursor);
            let distinct = read_str(cursor);
            let exclude_group_names = read_str(cursor);
            let exclude_self = read_u8(cursor) != 0;
            let group_names = read_str(cursor);
            let location = decode_zone(read_u8(cursor));
            let max = read_u8(cursor) != 0;
            let optional = read_u8(cursor) != 0;
            let original_value = read_u8(cursor) != 0;
            let parenthetical = read_str(cursor);
            let per_unit = read_u8(cursor) != 0;
            let per_unit_count = read_u8(cursor);
            let per_unit_type = read_str(cursor);
            let position = decode_zone(read_u8(cursor));
            let position_compare = read_str(cursor);
            let self_cost = read_u8(cursor) != 0;
            let self_target = read_u8(cursor) != 0;
            let source = decode_zone(read_u8(cursor));
            let state = decode_heart(read_u8(cursor));
            let state_change = read_str(cursor);
            let target = decode_player(read_u8(cursor));
            let mut ek = default_changeState();
            if let EffectKind::ChangeState { action_by: ref mut _bc_action_by, activation_position: ref mut _bc_activation_position, all: ref mut _bc_all, blade_limit: ref mut _bc_blade_limit, blade_limit_operator: ref mut _bc_blade_limit_operator, card_type: ref mut _bc_card_type, cost_from_revealed: ref mut _bc_cost_from_revealed, cost_limit: ref mut _bc_cost_limit, cost_limit_operator: ref mut _bc_cost_limit_operator, exclude_group_names: ref mut _bc_exclude_group_names, exclude_self: ref mut _bc_exclude_self, group_names: ref mut _bc_group_names, location: ref mut _bc_location, optional: ref mut _bc_optional, original_value: ref mut _bc_original_value, per_unit: ref mut _bc_per_unit, per_unit_count: ref mut _bc_per_unit_count, per_unit_type: ref mut _bc_per_unit_type, self_cost: ref mut _bc_self_cost, self_target: ref mut _bc_self_target, source: ref mut _bc_source, state: ref mut _bc_state, state_change: ref mut _bc_state_change, target: ref mut _bc_target, .. } = &mut ek {
                *_bc_action_by = Some(action_by.into());
                *_bc_activation_position = Some(activation_position.into());
                *_bc_all = Some(all);
                *_bc_blade_limit = Some(blade_limit as u32);
                *_bc_blade_limit_operator = Some(decode_operator_from_str(blade_limit_operator));
                *_bc_card_type = Some(card_type.into());
                *_bc_cost_from_revealed = Some(cost_from_revealed);
                *_bc_cost_limit = Some(cost_limit as u32);
                *_bc_cost_limit_operator = Some(decode_operator_from_str(cost_limit_operator));
                *_bc_exclude_group_names = exclude_group_names.map(|s| Box::new(vec![s.to_string()]));
                *_bc_exclude_self = Some(exclude_self);
                *_bc_group_names = group_names.map(|s| Box::new(vec![s.to_string()]));
                *_bc_location = Some(location.into());
                *_bc_optional = Some(optional);
                *_bc_original_value = Some(original_value);
                *_bc_per_unit = Some(per_unit);
                *_bc_per_unit_count = Some(per_unit_count as u32);
                *_bc_per_unit_type = per_unit_type.map(|s| s.into());
                *_bc_self_cost = Some(self_cost);
                *_bc_self_target = Some(self_target);
                *_bc_source = Some(source.into());
                *_bc_state = Some(state.into());
                *_bc_state_change = state_change.map(|s| s.into());
                *_bc_target = Some(target.into());
            }
            Some(Box::new(ek))
        }
        Opcode::ChooseTargetPlayer => {
            let choice_options = read_str(cursor);
            let optional = read_u8(cursor) != 0;
            let mut ek = default_selectTarget();
            if let EffectKind::SelectTarget { choice_options: ref mut _bc_choice_options, optional: ref mut _bc_optional, .. } = &mut ek {
                *_bc_choice_options = choice_options.map(|s| Box::new(vec![s.to_string()]));
                *_bc_optional = Some(optional);
            }
            Some(Box::new(ek))
        }
        Opcode::ConditionalAlternative => {
            let activation_position = decode_zone(read_u8(cursor));
            let group_names = read_str(cursor);
            let parenthetical = read_str(cursor);
            let position = decode_zone(read_u8(cursor));
            let mut ek = default_compoundEffect();
            if let EffectKind::CompoundEffect { activation_position: ref mut _bc_activation_position, group_names: ref mut _bc_group_names, .. } = &mut ek {
                *_bc_activation_position = Some(activation_position.into());
                *_bc_group_names = group_names.map(|s| Box::new(vec![s.to_string()]));
            }
            Some(Box::new(ek))
        }
        Opcode::DiscardUntilCount => {
            let baton_touch_trigger = read_u8(cursor) != 0;
            let count = read_u8(cursor);
            let destination = decode_zone(read_u8(cursor));
            let multiple_targets = read_u8(cursor) != 0;
            let source = decode_zone(read_u8(cursor));
            let target = decode_player(read_u8(cursor));
            let target_count = read_u8(cursor);
            let mut ek = default_moveCards();
            if let EffectKind::MoveCards { baton_touch_trigger: ref mut _bc_baton_touch_trigger, count: ref mut _bc_count, destination: ref mut _bc_destination, multiple_targets: ref mut _bc_multiple_targets, source: ref mut _bc_source, target: ref mut _bc_target, target_count: ref mut _bc_target_count, .. } = &mut ek {
                *_bc_baton_touch_trigger = Some(baton_touch_trigger);
                *_bc_count = Some(count as u32);
                *_bc_destination = Some(destination.into());
                *_bc_multiple_targets = Some(multiple_targets);
                *_bc_source = Some(source.into());
                *_bc_target = Some(target.into());
                *_bc_target_count = Some(target_count as u32);
            }
            Some(Box::new(ek))
        }
        Opcode::DoNothing => {
            let answers = read_str(cursor);
            let mut ek = default_customOp();
            if let EffectKind::CustomOp { answers: ref mut _bc_answers, .. } = &mut ek {
                *_bc_answers = answers.map(|s| Box::new(vec![s.to_string()]));
            }
            Some(Box::new(ek))
        }
        Opcode::DrawCard => {
            let action_by = decode_player(read_u8(cursor));
            let activation_position = decode_zone(read_u8(cursor));
            let answers = read_str(cursor);
            let baton_touch_trigger = read_u8(cursor) != 0;
            let card_type = decode_card_type(read_u8(cursor));
            let count = read_u8(cursor);
            let destination = decode_zone(read_u8(cursor));
            let duration = decode_duration(read_u8(cursor));
            let exclude_self = read_u8(cursor) != 0;
            let group_names = read_str(cursor);
            let heart_colors = decode_heart(read_u8(cursor));
            let location = decode_zone(read_u8(cursor));
            let multiple_targets = read_u8(cursor) != 0;
            let optional = read_u8(cursor) != 0;
            let original_value = read_u8(cursor) != 0;
            let parenthetical = read_str(cursor);
            let per_unit = read_u8(cursor) != 0;
            let per_unit_count = read_u8(cursor);
            let per_unit_type = decode_zone(read_u8(cursor));
            let position = decode_zone(read_u8(cursor));
            let position_compare = read_str(cursor);
            let source = decode_zone(read_u8(cursor));
            let state = decode_state(read_u8(cursor));
            let target = decode_player(read_u8(cursor));
            let trigger_type = read_str(cursor);
            let mut ek = default_drawCards();
            if let EffectKind::DrawCards { action_by: ref mut _bc_action_by, card_type: ref mut _bc_card_type, destination: ref mut _bc_destination, exclude_self: ref mut _bc_exclude_self, heart_colors: ref mut _bc_heart_colors, location: ref mut _bc_location, original_value: ref mut _bc_original_value, per_unit: ref mut _bc_per_unit, per_unit_count: ref mut _bc_per_unit_count, per_unit_type: ref mut _bc_per_unit_type, source: ref mut _bc_source, state: ref mut _bc_state, target: ref mut _bc_target, trigger_type: ref mut _bc_trigger_type, .. } = &mut ek {
                *_bc_action_by = Some(action_by.into());
                *_bc_card_type = Some(card_type.into());
                *_bc_destination = Some(destination.into());
                *_bc_exclude_self = Some(exclude_self);
                *_bc_heart_colors = Box::new(vec![heart_colors.to_string()]);
                *_bc_location = Some(location.into());
                *_bc_original_value = Some(original_value);
                *_bc_per_unit = Some(per_unit);
                *_bc_per_unit_count = Some(per_unit_count as u32);
                *_bc_per_unit_type = Some(per_unit_type.into());
                *_bc_source = Some(source.into());
                *_bc_state = Some(state.into());
                *_bc_target = Some(target.into());
                *_bc_trigger_type = trigger_type.map(|s| s.into());
            }
            Some(Box::new(ek))
        }
        Opcode::DrawUntilCount => {
            let count = read_u8(cursor);
            let destination = decode_zone(read_u8(cursor));
            let source = decode_zone(read_u8(cursor));
            let target = decode_player(read_u8(cursor));
            let target_count = read_u8(cursor);
            let mut ek = default_drawCards();
            if let EffectKind::DrawCards { destination: ref mut _bc_destination, source: ref mut _bc_source, target: ref mut _bc_target, target_count: ref mut _bc_target_count, .. } = &mut ek {
                *_bc_destination = Some(destination.into());
                *_bc_source = Some(source.into());
                *_bc_target = Some(target.into());
                *_bc_target_count = Some(target_count as u32);
            }
            Some(Box::new(ek))
        }
        Opcode::GainAbility => {
            let ability_gain = read_str(cursor);
            let ability_gain_trigger = read_str(cursor);
            let activation_position = decode_zone(read_u8(cursor));
            let card_type = decode_card_type(read_u8(cursor));
            let count = read_u8(cursor);
            let duration = decode_duration(read_u8(cursor));
            let group_names = read_str(cursor);
            let location = read_str(cursor);
            let max = read_u8(cursor) != 0;
            let parenthetical = read_str(cursor);
            let self_target = read_u8(cursor) != 0;
            let source = decode_zone(read_u8(cursor));
            let target = decode_player(read_u8(cursor));
            let mut ek = default_abilityOp();
            if let EffectKind::AbilityOp { ability_gain: ref mut _bc_ability_gain, ability_gain_trigger: ref mut _bc_ability_gain_trigger, activation_position: ref mut _bc_activation_position, card_type: ref mut _bc_card_type, duration: ref mut _bc_duration, group_names: ref mut _bc_group_names, location: ref mut _bc_location, self_target: ref mut _bc_self_target, source: ref mut _bc_source, target: ref mut _bc_target, .. } = &mut ek {
                *_bc_ability_gain = ability_gain.map(|s| s.into());
                *_bc_ability_gain_trigger = ability_gain_trigger.map(|s| s.into());
                *_bc_activation_position = Some(activation_position.into());
                *_bc_card_type = Some(card_type.into());
                *_bc_duration = Some(duration.into());
                *_bc_group_names = group_names.map(|s| Box::new(vec![s.to_string()]));
                *_bc_location = location.map(|s| s.into());
                *_bc_self_target = Some(self_target);
                *_bc_source = Some(source.into());
                *_bc_target = Some(target.into());
            }
            Some(Box::new(ek))
        }
        Opcode::GainAbilityFromSource => {
            let all = read_u8(cursor) != 0;
            let card_type = decode_card_type(read_u8(cursor));
            let cost_limit = read_u8(cursor);
            let cost_limit_operator = decode_operator(read_u8(cursor));
            let group_names = read_str(cursor);
            let source_location = decode_zone(read_u8(cursor));
            let trigger_filter = read_str(cursor);
            let mut ek = default_abilityOp();
            if let EffectKind::AbilityOp { all: ref mut _bc_all, card_type: ref mut _bc_card_type, cost_limit: ref mut _bc_cost_limit, cost_limit_operator: ref mut _bc_cost_limit_operator, group_names: ref mut _bc_group_names, trigger_filter: ref mut _bc_trigger_filter, .. } = &mut ek {
                *_bc_all = Some(all);
                *_bc_card_type = Some(card_type.into());
                *_bc_cost_limit = Some(cost_limit as u32);
                *_bc_cost_limit_operator = Some(decode_operator_from_str(cost_limit_operator));
                *_bc_group_names = group_names.map(|s| Box::new(vec![s.to_string()]));
                *_bc_trigger_filter = trigger_filter.map(|s| Box::new(vec![s.to_string()]));
            }
            Some(Box::new(ek))
        }
        Opcode::GainResource => {
            let action_by = decode_player(read_u8(cursor));
            let activation_position = decode_zone(read_u8(cursor));
            let all = read_u8(cursor) != 0;
            let answers = read_str(cursor);
            let baton_touch_trigger = read_u8(cursor) != 0;
            let card_property = read_str(cursor);
            let card_type = decode_card_type(read_u8(cursor));
            let characters = read_str(cursor);
            let conditional = read_u8(cursor) != 0;
            let cost_limit = read_u8(cursor);
            let cost_limit_operator = decode_operator(read_u8(cursor));
            let count = read_u8(cursor);
            let distinct = read_str(cursor);
            let duration = read_str(cursor);
            let exclude_group_names = read_str(cursor);
            let exclude_self = read_u8(cursor) != 0;
            let filter_targets_by_heart_colors = read_u8(cursor) != 0;
            let group_names = read_str(cursor);
            let group_reference = read_str(cursor);
            let heart_color = decode_heart(read_u8(cursor));
            let heart_colors = decode_heart(read_u8(cursor));
            let heart_colors_from_selected_card = read_u8(cursor) != 0;
            let heart_type = read_str(cursor);
            let location = decode_zone(read_u8(cursor));
            let max = read_u8(cursor) != 0;
            let max_repeats = read_u8(cursor);
            let multiple_targets = read_u8(cursor) != 0;
            let negation = read_u8(cursor) != 0;
            let original_value = read_u8(cursor) != 0;
            let parenthetical = read_str(cursor);
            let per_unit = read_u8(cursor) != 0;
            let per_unit_count = read_u8(cursor);
            let per_unit_source = read_str(cursor);
            let per_unit_type = decode_zone(read_u8(cursor));
            let position = decode_zone(read_u8(cursor));
            let require_all_heart_colors = read_u8(cursor) != 0;
            let resource = read_str(cursor);
            let same_name = read_u8(cursor) != 0;
            let self_target = read_u8(cursor) != 0;
            let sign = read_str(cursor);
            let state = decode_state(read_u8(cursor));
            let target = decode_player(read_u8(cursor));
            let target_count = read_u8(cursor);
            let target_from_selection = read_u8(cursor) != 0;
            let timing_condition = read_str(cursor);
            let trigger_type = read_str(cursor);
            let mut ek = default_gainResource();
            if let EffectKind::GainResource { action_by: ref mut _bc_action_by, activation_position: ref mut _bc_activation_position, all: ref mut _bc_all, card_property: ref mut _bc_card_property, card_type: ref mut _bc_card_type, characters: ref mut _bc_characters, cost_limit: ref mut _bc_cost_limit, cost_limit_operator: ref mut _bc_cost_limit_operator, duration: ref mut _bc_duration, exclude_group_names: ref mut _bc_exclude_group_names, exclude_self: ref mut _bc_exclude_self, filter_targets_by_heart_colors: ref mut _bc_filter_targets_by_heart_colors, group_names: ref mut _bc_group_names, group_reference: ref mut _bc_group_reference, heart_color: ref mut _bc_heart_color, heart_colors: ref mut _bc_heart_colors, heart_colors_from_selected_card: ref mut _bc_heart_colors_from_selected_card, heart_type: ref mut _bc_heart_type, location: ref mut _bc_location, multiple_targets: ref mut _bc_multiple_targets, negation: ref mut _bc_negation, original_value: ref mut _bc_original_value, per_unit: ref mut _bc_per_unit, per_unit_count: ref mut _bc_per_unit_count, per_unit_type: ref mut _bc_per_unit_type, require_all_heart_colors: ref mut _bc_require_all_heart_colors, resource: ref mut _bc_resource, same_name: ref mut _bc_same_name, self_target: ref mut _bc_self_target, sign: ref mut _bc_sign, state: ref mut _bc_state, target_count: ref mut _bc_target_count, target_from_selection: ref mut _bc_target_from_selection, timing_condition: ref mut _bc_timing_condition, trigger_type: ref mut _bc_trigger_type, .. } = &mut ek {
                *_bc_action_by = Some(action_by.into());
                *_bc_activation_position = Some(activation_position.into());
                *_bc_all = Some(all);
                *_bc_card_property = card_property.map(|s| s.into());
                *_bc_card_type = Some(card_type.into());
                *_bc_characters = characters.map(|s| Box::new(vec![s.to_string()]));
                *_bc_cost_limit = Some(cost_limit as u32);
                *_bc_cost_limit_operator = Some(decode_operator_from_str(cost_limit_operator));
                *_bc_duration = duration.map(|s| s.into());
                *_bc_exclude_group_names = exclude_group_names.map(|s| Box::new(vec![s.to_string()]));
                *_bc_exclude_self = Some(exclude_self);
                *_bc_filter_targets_by_heart_colors = Some(filter_targets_by_heart_colors);
                *_bc_group_names = group_names.map(|s| Box::new(vec![s.to_string()]));
                *_bc_group_reference = group_reference.map(|s| s.into());
                *_bc_heart_color = Some(heart_color.into());
                *_bc_heart_colors = Box::new(vec![heart_colors.to_string()]);
                *_bc_heart_colors_from_selected_card = Some(heart_colors_from_selected_card);
                *_bc_heart_type = heart_type.map(|s| s.into());
                *_bc_location = Some(location.into());
                *_bc_multiple_targets = Some(multiple_targets);
                *_bc_negation = Some(negation);
                *_bc_original_value = Some(original_value);
                *_bc_per_unit = Some(per_unit);
                *_bc_per_unit_count = Some(per_unit_count as u32);
                *_bc_per_unit_type = Some(per_unit_type.into());
                *_bc_require_all_heart_colors = Some(require_all_heart_colors);
                *_bc_resource = resource.map(|s| s.into());
                *_bc_same_name = Some(same_name);
                *_bc_self_target = Some(self_target);
                *_bc_sign = sign.map(|s| s.into());
                *_bc_state = Some(state.into());
                *_bc_target_count = Some(target_count as u32);
                *_bc_target_from_selection = Some(target_from_selection);
                *_bc_timing_condition = timing_condition.map(|s| s.into());
                *_bc_trigger_type = trigger_type.map(|s| s.into());
            }
            Some(Box::new(ek))
        }
        Opcode::InvalidateAbility => {
            let all = read_u8(cursor) != 0;
            let card_type = decode_card_type(read_u8(cursor));
            let count = read_u8(cursor);
            let duration = decode_duration(read_u8(cursor));
            let group_names = read_str(cursor);
            let heart_colors = decode_heart(read_u8(cursor));
            let optional = read_u8(cursor) != 0;
            let self_target = read_u8(cursor) != 0;
            let source = decode_zone(read_u8(cursor));
            let target = decode_player(read_u8(cursor));
            let mut ek = default_abilityOp();
            if let EffectKind::AbilityOp { all: ref mut _bc_all, card_type: ref mut _bc_card_type, duration: ref mut _bc_duration, group_names: ref mut _bc_group_names, heart_colors: ref mut _bc_heart_colors, self_target: ref mut _bc_self_target, source: ref mut _bc_source, target: ref mut _bc_target, .. } = &mut ek {
                *_bc_all = Some(all);
                *_bc_card_type = Some(card_type.into());
                *_bc_duration = Some(duration.into());
                *_bc_group_names = group_names.map(|s| Box::new(vec![s.to_string()]));
                *_bc_heart_colors = Box::new(vec![heart_colors.to_string()]);
                *_bc_self_target = Some(self_target);
                *_bc_source = Some(source.into());
                *_bc_target = Some(target.into());
            }
            Some(Box::new(ek))
        }
        Opcode::LookAt => {
            let card_type = decode_card_type(read_u8(cursor));
            let count = read_u8(cursor);
            let group_names = read_str(cursor);
            let location = decode_zone(read_u8(cursor));
            let per_unit = read_u8(cursor) != 0;
            let per_unit_count = read_u8(cursor);
            let per_unit_type = read_str(cursor);
            let source = decode_zone(read_u8(cursor));
            let target = decode_player(read_u8(cursor));
            let mut ek = default_lookReveal();
            if let EffectKind::LookReveal { card_type: ref mut _bc_card_type, group_names: ref mut _bc_group_names, location: ref mut _bc_location, per_unit: ref mut _bc_per_unit, per_unit_count: ref mut _bc_per_unit_count, per_unit_type: ref mut _bc_per_unit_type, source: ref mut _bc_source, target: ref mut _bc_target, .. } = &mut ek {
                *_bc_card_type = Some(card_type.into());
                *_bc_group_names = group_names.map(|s| Box::new(vec![s.to_string()]));
                *_bc_location = Some(location.into());
                *_bc_per_unit = Some(per_unit);
                *_bc_per_unit_count = Some(per_unit_count as u32);
                *_bc_per_unit_type = per_unit_type.map(|s| s.into());
                *_bc_source = Some(source.into());
                *_bc_target = Some(target.into());
            }
            Some(Box::new(ek))
        }
        Opcode::ModifyCost => {
            let ability_filter = read_str(cursor);
            let card_type = decode_card_type(read_u8(cursor));
            let conditional = read_u8(cursor) != 0;
            let cost_limit = read_u8(cursor);
            let cost_limit_operator = decode_operator(read_u8(cursor));
            let count = read_u8(cursor);
            let destination = decode_zone(read_u8(cursor));
            let duration = read_str(cursor);
            let exclude_self = read_u8(cursor) != 0;
            let group_names = read_str(cursor);
            let location = decode_zone(read_u8(cursor));
            let non_stackable = read_u8(cursor) != 0;
            let operation = read_str(cursor);
            let original_count = read_u8(cursor);
            let original_operator = decode_operator(read_u8(cursor));
            let original_value = read_u8(cursor) != 0;
            let per_unit = read_u8(cursor) != 0;
            let per_unit_count = read_u8(cursor);
            let per_unit_location = decode_zone(read_u8(cursor));
            let per_unit_type = decode_zone(read_u8(cursor));
            let self_target = read_u8(cursor) != 0;
            let source = decode_zone(read_u8(cursor));
            let target = decode_player(read_u8(cursor));
            let value = read_u8(cursor);
            let mut ek = default_customOp();
            if let EffectKind::CustomOp { card_type: ref mut _bc_card_type, duration: ref mut _bc_duration, exclude_self: ref mut _bc_exclude_self, group_names: ref mut _bc_group_names, location: ref mut _bc_location, original_value: ref mut _bc_original_value, self_target: ref mut _bc_self_target, .. } = &mut ek {
                *_bc_card_type = Some(card_type.into());
                *_bc_duration = duration.map(|s| s.into());
                *_bc_exclude_self = Some(exclude_self);
                *_bc_group_names = group_names.map(|s| Box::new(vec![s.to_string()]));
                *_bc_location = Some(location.into());
                *_bc_original_value = Some(original_value);
                *_bc_self_target = Some(self_target);
            }
            Some(Box::new(ek))
        }
        Opcode::ModifyRequiredHearts => {
            let baton_touch_trigger = read_u8(cursor) != 0;
            let card_type = decode_card_type(read_u8(cursor));
            let conditional = read_u8(cursor) != 0;
            let count = read_u8(cursor);
            let distinct = read_str(cursor);
            let duration = read_str(cursor);
            let exclude_heart_colors = decode_heart(read_u8(cursor));
            let exclude_self = read_u8(cursor) != 0;
            let group_names = read_str(cursor);
            let heart_colors = read_str(cursor);
            let location = decode_zone(read_u8(cursor));
            let max = read_u8(cursor) != 0;
            let max_repeats = read_u8(cursor);
            let non_stackable = read_u8(cursor) != 0;
            let operation = read_str(cursor);
            let original_count = read_u8(cursor);
            let original_operator = decode_operator(read_u8(cursor));
            let original_value = read_u8(cursor) != 0;
            let parenthetical = read_str(cursor);
            let per_unit = read_u8(cursor) != 0;
            let per_unit_count = read_u8(cursor);
            let per_unit_heart_colors = decode_heart(read_u8(cursor));
            let per_unit_type = read_str(cursor);
            let position = decode_zone(read_u8(cursor));
            let replace_all = read_u8(cursor) != 0;
            let self_target = read_u8(cursor) != 0;
            let target = decode_player(read_u8(cursor));
            let timing_condition = read_str(cursor);
            let value = read_u8(cursor);
            let mut ek = default_modifyHearts();
            if let EffectKind::ModifyHearts { card_type: ref mut _bc_card_type, duration: ref mut _bc_duration, exclude_heart_colors: ref mut _bc_exclude_heart_colors, exclude_self: ref mut _bc_exclude_self, group_names: ref mut _bc_group_names, heart_colors: ref mut _bc_heart_colors, location: ref mut _bc_location, operation: ref mut _bc_operation, original_count: ref mut _bc_original_count, original_operator: ref mut _bc_original_operator, original_value: ref mut _bc_original_value, per_unit: ref mut _bc_per_unit, per_unit_count: ref mut _bc_per_unit_count, per_unit_heart_colors: ref mut _bc_per_unit_heart_colors, per_unit_type: ref mut _bc_per_unit_type, replace_all: ref mut _bc_replace_all, self_target: ref mut _bc_self_target, timing_condition: ref mut _bc_timing_condition, value: ref mut _bc_value, .. } = &mut ek {
                *_bc_card_type = Some(card_type.into());
                *_bc_duration = duration.map(|s| s.into());
                *_bc_exclude_heart_colors = Some(Box::new(vec![exclude_heart_colors.to_string()]));
                *_bc_exclude_self = Some(exclude_self);
                *_bc_group_names = group_names.map(|s| Box::new(vec![s.to_string()]));
                *_bc_heart_colors = heart_colors.map_or(Default::default(), |s| Box::new(vec![s.to_string()]));
                *_bc_location = Some(location.into());
                *_bc_operation = operation.map(|s| s.into());
                *_bc_original_count = Some(original_count as u32);
                *_bc_original_operator = Some(decode_operator_from_str(original_operator));
                *_bc_original_value = Some(original_value);
                *_bc_per_unit = Some(per_unit);
                *_bc_per_unit_count = Some(per_unit_count as u32);
                *_bc_per_unit_heart_colors = Box::new(vec![per_unit_heart_colors.to_string()]);
                *_bc_per_unit_type = per_unit_type.map(|s| s.into());
                *_bc_replace_all = Some(replace_all);
                *_bc_self_target = Some(self_target);
                *_bc_timing_condition = timing_condition.map(|s| s.into());
                *_bc_value = Some(value as u32);
            }
            Some(Box::new(ek))
        }
        Opcode::ModifyRequiredHeartsGlobal => {
            let all = read_u8(cursor) != 0;
            let heart_colors = read_str(cursor);
            let operation = read_str(cursor);
            let target = decode_player(read_u8(cursor));
            let value = read_u8(cursor);
            let mut ek = default_modifyHearts();
            if let EffectKind::ModifyHearts { all: ref mut _bc_all, heart_colors: ref mut _bc_heart_colors, operation: ref mut _bc_operation, value: ref mut _bc_value, .. } = &mut ek {
                *_bc_all = Some(all);
                *_bc_heart_colors = heart_colors.map_or(Default::default(), |s| Box::new(vec![s.to_string()]));
                *_bc_operation = operation.map(|s| s.into());
                *_bc_value = Some(value as u32);
            }
            Some(Box::new(ek))
        }
        Opcode::ModifyScore => {
            let activation_position = decode_zone(read_u8(cursor));
            let card_names = read_str(cursor);
            let card_property = read_str(cursor);
            let card_type = decode_card_type(read_u8(cursor));
            let conditional = read_u8(cursor) != 0;
            let distinct = read_str(cursor);
            let duration = read_str(cursor);
            let group_names = read_str(cursor);
            let heart_colors = decode_heart(read_u8(cursor));
            let location = decode_zone(read_u8(cursor));
            let max_repeats = read_u8(cursor);
            let multiple_targets = read_u8(cursor) != 0;
            let need_heart_operator = decode_operator(read_u8(cursor));
            let need_heart_total = read_u8(cursor);
            let negation = read_u8(cursor) != 0;
            let operation = read_str(cursor);
            let parenthetical = read_str(cursor);
            let per_unit = read_u8(cursor) != 0;
            let per_unit_count = read_u8(cursor);
            let per_unit_heart_colors = decode_heart(read_u8(cursor));
            let per_unit_type = read_str(cursor);
            let position = decode_zone(read_u8(cursor));
            let self_target = read_u8(cursor) != 0;
            let source = decode_zone(read_u8(cursor));
            let state = decode_state(read_u8(cursor));
            let target = decode_player(read_u8(cursor));
            let value = read_u8(cursor);
            let mut ek = default_modifyScore();
            if let EffectKind::ModifyScore { activation_position: ref mut _bc_activation_position, card_names: ref mut _bc_card_names, card_property: ref mut _bc_card_property, card_type: ref mut _bc_card_type, duration: ref mut _bc_duration, group_names: ref mut _bc_group_names, heart_colors: ref mut _bc_heart_colors, location: ref mut _bc_location, max_repeats: ref mut _bc_max_repeats, need_heart_operator: ref mut _bc_need_heart_operator, need_heart_total: ref mut _bc_need_heart_total, negation: ref mut _bc_negation, operation: ref mut _bc_operation, per_unit: ref mut _bc_per_unit, per_unit_count: ref mut _bc_per_unit_count, per_unit_heart_colors: ref mut _bc_per_unit_heart_colors, per_unit_type: ref mut _bc_per_unit_type, self_target: ref mut _bc_self_target, source: ref mut _bc_source, state: ref mut _bc_state, target: ref mut _bc_target, value: ref mut _bc_value, .. } = &mut ek {
                *_bc_activation_position = Some(activation_position.into());
                *_bc_card_names = card_names.map_or(Default::default(), |s| Box::new(vec![s.to_string()]));
                *_bc_card_property = card_property.map(|s| s.into());
                *_bc_card_type = Some(card_type.into());
                *_bc_duration = duration.map(|s| s.into());
                *_bc_group_names = group_names.map(|s| Box::new(vec![s.to_string()]));
                *_bc_heart_colors = Box::new(vec![heart_colors.to_string()]);
                *_bc_location = Some(location.into());
                *_bc_max_repeats = Some(max_repeats as u32);
                *_bc_need_heart_operator = Some(decode_operator_from_str(need_heart_operator));
                *_bc_need_heart_total = Some(need_heart_total as u32);
                *_bc_negation = Some(negation);
                *_bc_operation = operation.map(|s| s.into());
                *_bc_per_unit = Some(per_unit);
                *_bc_per_unit_count = Some(per_unit_count as u32);
                *_bc_per_unit_heart_colors = Box::new(vec![per_unit_heart_colors.to_string()]);
                *_bc_per_unit_type = per_unit_type.map(|s| s.into());
                *_bc_self_target = Some(self_target);
                *_bc_source = Some(source.into());
                *_bc_state = Some(state.into());
                *_bc_target = Some(target.into());
                *_bc_value = Some(value as u32);
            }
            Some(Box::new(ek))
        }
        Opcode::ModifyYellCount => {
            let count = read_u8(cursor);
            let duration = decode_duration(read_u8(cursor));
            let exclude_self = read_u8(cursor) != 0;
            let group_names = read_str(cursor);
            let operation = read_str(cursor);
            let mut ek = default_modifyScore();
            if let EffectKind::ModifyScore { duration: ref mut _bc_duration, exclude_self: ref mut _bc_exclude_self, group_names: ref mut _bc_group_names, operation: ref mut _bc_operation, .. } = &mut ek {
                *_bc_duration = Some(duration.into());
                *_bc_exclude_self = Some(exclude_self);
                *_bc_group_names = group_names.map(|s| Box::new(vec![s.to_string()]));
                *_bc_operation = operation.map(|s| s.into());
            }
            Some(Box::new(ek))
        }
        Opcode::MoveCards => {
            let action_by = decode_player(read_u8(cursor));
            let activation_position = decode_zone(read_u8(cursor));
            let all = read_u8(cursor) != 0;
            let allow_occupied_stage = read_u8(cursor) != 0;
            let answers = read_str(cursor);
            let any_number = read_u8(cursor) != 0;
            let baton_touch_trigger = read_u8(cursor) != 0;
            let card_names = read_str(cursor);
            let card_property = read_str(cursor);
            let card_type = decode_card_type(read_u8(cursor));
            let characters = read_str(cursor);
            let cost_limit = read_u8(cursor);
            let cost_limit_max = read_u8(cursor);
            let cost_limit_min = read_u8(cursor);
            let cost_limit_operator = decode_operator(read_u8(cursor));
            let cost_offset = read_u8(cursor);
            let cost_reference = read_str(cursor);
            let cost_total = read_u8(cursor);
            let cost_total_operator = decode_operator(read_u8(cursor));
            let count = read_u8(cursor);
            let destination = decode_zone(read_u8(cursor));
            let discard_remaining = read_u8(cursor) != 0;
            let distinct = read_str(cursor);
            let duration = read_str(cursor);
            let exclude_self = read_u8(cursor) != 0;
            let group_names = read_str(cursor);
            let group_reference = read_str(cursor);
            let heart_colors = decode_heart(read_u8(cursor));
            let location = decode_zone(read_u8(cursor));
            let max = read_u8(cursor) != 0;
            let multiple_targets = read_u8(cursor) != 0;
            let name_constraint = read_str(cursor);
            let name_constraint_source = read_str(cursor);
            let need_heart_color = decode_heart(read_u8(cursor));
            let need_heart_operator = decode_operator(read_u8(cursor));
            let need_heart_total = read_u8(cursor);
            let negation = read_u8(cursor) != 0;
            let optional = read_u8(cursor) != 0;
            let or_card_types = decode_card_type(read_u8(cursor));
            let parenthetical = read_str(cursor);
            let placement_order = read_str(cursor);
            let position = decode_zone(read_u8(cursor));
            let self_target = read_u8(cursor) != 0;
            let shuffle = read_u8(cursor) != 0;
            let source = decode_zone(read_u8(cursor));
            let state_change = decode_state(read_u8(cursor));
            let target = decode_zone(read_u8(cursor));
            let mut ek = default_moveCards();
            if let EffectKind::MoveCards { action_by: ref mut _bc_action_by, activation_position: ref mut _bc_activation_position, all: ref mut _bc_all, allow_occupied_stage: ref mut _bc_allow_occupied_stage, any_number: ref mut _bc_any_number, baton_touch_trigger: ref mut _bc_baton_touch_trigger, card_names: ref mut _bc_card_names, card_property: ref mut _bc_card_property, card_type: ref mut _bc_card_type, characters: ref mut _bc_characters, cost_limit: ref mut _bc_cost_limit, cost_limit_max: ref mut _bc_cost_limit_max, cost_limit_min: ref mut _bc_cost_limit_min, cost_limit_operator: ref mut _bc_cost_limit_operator, cost_reference: ref mut _bc_cost_reference, cost_total: ref mut _bc_cost_total, cost_total_operator: ref mut _bc_cost_total_operator, count: ref mut _bc_count, destination: ref mut _bc_destination, discard_remaining: ref mut _bc_discard_remaining, exclude_self: ref mut _bc_exclude_self, group_names: ref mut _bc_group_names, group_reference: ref mut _bc_group_reference, heart_colors: ref mut _bc_heart_colors, location: ref mut _bc_location, multiple_targets: ref mut _bc_multiple_targets, name_constraint: ref mut _bc_name_constraint, name_constraint_source: ref mut _bc_name_constraint_source, need_heart_color: ref mut _bc_need_heart_color, need_heart_operator: ref mut _bc_need_heart_operator, need_heart_total: ref mut _bc_need_heart_total, negation: ref mut _bc_negation, or_card_types: ref mut _bc_or_card_types, self_target: ref mut _bc_self_target, shuffle: ref mut _bc_shuffle, source: ref mut _bc_source, state_change: ref mut _bc_state_change, target: ref mut _bc_target, .. } = &mut ek {
                *_bc_action_by = Some(action_by.into());
                *_bc_activation_position = Some(activation_position.into());
                *_bc_all = Some(all);
                *_bc_allow_occupied_stage = Some(allow_occupied_stage);
                *_bc_any_number = Some(any_number);
                *_bc_baton_touch_trigger = Some(baton_touch_trigger);
                *_bc_card_names = card_names.map_or(Default::default(), |s| Box::new(vec![s.to_string()]));
                *_bc_card_property = card_property.map(|s| s.into());
                *_bc_card_type = Some(card_type.into());
                *_bc_characters = characters.map(|s| Box::new(vec![s.to_string()]));
                *_bc_cost_limit = Some(cost_limit as u32);
                *_bc_cost_limit_max = Some(cost_limit_max as u32);
                *_bc_cost_limit_min = Some(cost_limit_min as u32);
                *_bc_cost_limit_operator = Some(decode_operator_from_str(cost_limit_operator));
                *_bc_cost_reference = cost_reference.map(|s| s.into());
                *_bc_cost_total = Some(cost_total as u32);
                *_bc_cost_total_operator = Some(decode_operator_from_str(cost_total_operator));
                *_bc_count = Some(count as u32);
                *_bc_destination = Some(destination.into());
                *_bc_discard_remaining = Some(discard_remaining);
                *_bc_exclude_self = Some(exclude_self);
                *_bc_group_names = group_names.map(|s| Box::new(vec![s.to_string()]));
                *_bc_group_reference = group_reference.map(|s| s.into());
                *_bc_heart_colors = Box::new(vec![heart_colors.to_string()]);
                *_bc_location = Some(location.into());
                *_bc_multiple_targets = Some(multiple_targets);
                *_bc_name_constraint = name_constraint.map(|s| s.into());
                *_bc_name_constraint_source = name_constraint_source.map(|s| s.into());
                *_bc_need_heart_color = Some(need_heart_color.into());
                *_bc_need_heart_operator = Some(decode_operator_from_str(need_heart_operator));
                *_bc_need_heart_total = Some(need_heart_total as u32);
                *_bc_negation = Some(negation);
                *_bc_or_card_types = Some(Box::new(vec![or_card_types.to_string()]));
                *_bc_self_target = Some(self_target);
                *_bc_shuffle = Some(shuffle);
                *_bc_source = Some(source.into());
                *_bc_state_change = Some(state_change.into());
                *_bc_target = Some(target.into());
            }
            Some(Box::new(ek))
        }
        Opcode::PayEnergy => {
            let count = read_u8(cursor);
            let energy = read_u8(cursor);
            let location = decode_zone(read_u8(cursor));
            let optional = read_u8(cursor) != 0;
            let target = decode_player(read_u8(cursor));
            let mut ek = default_gainResource();
            if let EffectKind::GainResource { location: ref mut _bc_location, optional: ref mut _bc_optional, .. } = &mut ek {
                *_bc_location = Some(location.into());
                *_bc_optional = Some(optional);
            }
            Some(Box::new(ek))
        }
        Opcode::PerformYell => {
            let count = read_u8(cursor);
            let group_names = read_str(cursor);
            let max = read_u8(cursor) != 0;
            let max_repeats = read_u8(cursor);
            let per_unit = read_u8(cursor) != 0;
            let per_unit_count = read_u8(cursor);
            let per_unit_source = read_str(cursor);
            let target = decode_player(read_u8(cursor));
            let mut ek = default_miscOp();
            if let EffectKind::MiscOp { group_names: ref mut _bc_group_names, per_unit: ref mut _bc_per_unit, per_unit_count: ref mut _bc_per_unit_count, target: ref mut _bc_target, .. } = &mut ek {
                *_bc_group_names = group_names.map(|s| Box::new(vec![s.to_string()]));
                *_bc_per_unit = Some(per_unit);
                *_bc_per_unit_count = Some(per_unit_count as u32);
                *_bc_target = Some(target.into());
            }
            Some(Box::new(ek))
        }
        Opcode::PlaceEnergyUnderMember => {
            let any_number = read_u8(cursor) != 0;
            let card_type = decode_card_type(read_u8(cursor));
            let cost_limit = read_u8(cursor);
            let cost_limit_operator = decode_operator(read_u8(cursor));
            let count = read_u8(cursor);
            let destination = decode_zone(read_u8(cursor));
            let energy_count = read_u8(cursor);
            let group_names = read_str(cursor);
            let optional = read_u8(cursor) != 0;
            let source = decode_zone(read_u8(cursor));
            let state_change = decode_state(read_u8(cursor));
            let target = decode_player(read_u8(cursor));
            let target_member = read_str(cursor);
            let mut ek = default_moveCards();
            if let EffectKind::MoveCards { any_number: ref mut _bc_any_number, card_type: ref mut _bc_card_type, cost_limit: ref mut _bc_cost_limit, cost_limit_operator: ref mut _bc_cost_limit_operator, count: ref mut _bc_count, destination: ref mut _bc_destination, energy_count: ref mut _bc_energy_count, group_names: ref mut _bc_group_names, source: ref mut _bc_source, state_change: ref mut _bc_state_change, target: ref mut _bc_target, target_member: ref mut _bc_target_member, .. } = &mut ek {
                *_bc_any_number = Some(any_number);
                *_bc_card_type = Some(card_type.into());
                *_bc_cost_limit = Some(cost_limit as u32);
                *_bc_cost_limit_operator = Some(decode_operator_from_str(cost_limit_operator));
                *_bc_count = Some(count as u32);
                *_bc_destination = Some(destination.into());
                *_bc_energy_count = Some(energy_count as u32);
                *_bc_group_names = group_names.map(|s| Box::new(vec![s.to_string()]));
                *_bc_source = Some(source.into());
                *_bc_state_change = Some(state_change.into());
                *_bc_target = Some(target.into());
                *_bc_target_member = target_member.map(|s| s.into());
            }
            Some(Box::new(ek))
        }
        Opcode::PlayBatonTouch => {
            let card_type = decode_card_type(read_u8(cursor));
            let count = read_u8(cursor);
            let group_names = read_str(cursor);
            let optional = read_u8(cursor) != 0;
            let source = decode_zone(read_u8(cursor));
            let mut ek = default_moveCards();
            if let EffectKind::MoveCards { card_type: ref mut _bc_card_type, count: ref mut _bc_count, group_names: ref mut _bc_group_names, source: ref mut _bc_source, .. } = &mut ek {
                *_bc_card_type = Some(card_type.into());
                *_bc_count = Some(count as u32);
                *_bc_group_names = group_names.map(|s| Box::new(vec![s.to_string()]));
                *_bc_source = Some(source.into());
            }
            Some(Box::new(ek))
        }
        Opcode::PositionChange => {
            let card_type = decode_card_type(read_u8(cursor));
            let count = read_u8(cursor);
            let destination = decode_zone(read_u8(cursor));
            let exclude_position = decode_zone(read_u8(cursor));
            let exclude_self = read_u8(cursor) != 0;
            let group_names = read_str(cursor);
            let multiple_targets = read_u8(cursor) != 0;
            let optional = read_u8(cursor) != 0;
            let parenthetical = read_str(cursor);
            let position = decode_zone(read_u8(cursor));
            let position_compare = read_str(cursor);
            let source = decode_zone(read_u8(cursor));
            let source_position = decode_zone(read_u8(cursor));
            let target = decode_player(read_u8(cursor));
            let target_member = read_str(cursor);
            let mut ek = default_positionOp();
            if let EffectKind::PositionOp { card_type: ref mut _bc_card_type, destination: ref mut _bc_destination, exclude_position: ref mut _bc_exclude_position, exclude_self: ref mut _bc_exclude_self, group_names: ref mut _bc_group_names, multiple_targets: ref mut _bc_multiple_targets, optional: ref mut _bc_optional, source: ref mut _bc_source, source_position: ref mut _bc_source_position, target: ref mut _bc_target, target_member: ref mut _bc_target_member, .. } = &mut ek {
                *_bc_card_type = Some(card_type.into());
                *_bc_destination = Some(destination.into());
                *_bc_exclude_position = Some(exclude_position.into());
                *_bc_exclude_self = Some(exclude_self);
                *_bc_group_names = group_names.map(|s| Box::new(vec![s.to_string()]));
                *_bc_multiple_targets = Some(multiple_targets);
                *_bc_optional = Some(optional);
                *_bc_source = Some(source.into());
                *_bc_source_position = Some(source_position.into());
                *_bc_target = Some(target.into());
                *_bc_target_member = target_member.map(|s| s.into());
            }
            Some(Box::new(ek))
        }
        Opcode::ReYell => {
            let lose_blade_hearts = read_u8(cursor) != 0;
            let target = decode_player(read_u8(cursor));
            let mut ek = default_miscOp();
            if let EffectKind::MiscOp { lose_blade_hearts: ref mut _bc_lose_blade_hearts, target: ref mut _bc_target, .. } = &mut ek {
                *_bc_lose_blade_hearts = Some(lose_blade_hearts);
                *_bc_target = Some(target.into());
            }
            Some(Box::new(ek))
        }
        Opcode::ReduceLiveCardSetLimit => {
            let card_type = decode_card_type(read_u8(cursor));
            let count = read_u8(cursor);
            let mut ek = default_restrictionOp();
            if let EffectKind::RestrictionOp { card_type: ref mut _bc_card_type, .. } = &mut ek {
                *_bc_card_type = Some(card_type.into());
            }
            Some(Box::new(ek))
        }
        Opcode::RepeatProcedure => {
            let max_repeats = read_u8(cursor);
            let optional = read_u8(cursor) != 0;
            let mut ek = default_compoundEffect();
            if let EffectKind::CompoundEffect { optional: ref mut _bc_optional, .. } = &mut ek {
                *_bc_optional = Some(optional);
            }
            Some(Box::new(ek))
        }
        Opcode::Restriction => {
            let card_type = decode_card_type(read_u8(cursor));
            let count = read_u8(cursor);
            let delayed = read_u8(cursor) != 0;
            let destination = decode_zone(read_u8(cursor));
            let duration = decode_duration(read_u8(cursor));
            let exclude_group_names = read_str(cursor);
            let phase = read_str(cursor);
            let restriction_type = read_str(cursor);
            let self_target = read_u8(cursor) != 0;
            let target = decode_player(read_u8(cursor));
            let mut ek = default_restrictionOp();
            if let EffectKind::RestrictionOp { card_type: ref mut _bc_card_type, delayed: ref mut _bc_delayed, duration: ref mut _bc_duration, exclude_group_names: ref mut _bc_exclude_group_names, phase: ref mut _bc_phase, restriction_type: ref mut _bc_restriction_type, self_target: ref mut _bc_self_target, .. } = &mut ek {
                *_bc_card_type = Some(card_type.into());
                *_bc_delayed = Some(delayed);
                *_bc_duration = Some(duration.into());
                *_bc_exclude_group_names = exclude_group_names.map(|s| Box::new(vec![s.to_string()]));
                *_bc_phase = phase.map(|s| s.into());
                *_bc_restriction_type = restriction_type.map(|s| s.into());
                *_bc_self_target = Some(self_target);
            }
            Some(Box::new(ek))
        }
        Opcode::Reveal => {
            let activation_position = decode_zone(read_u8(cursor));
            let all = read_u8(cursor) != 0;
            let blind = read_u8(cursor) != 0;
            let card_type = decode_card_type(read_u8(cursor));
            let cost_limit = read_u8(cursor);
            let cost_limit_operator = decode_operator(read_u8(cursor));
            let count = read_u8(cursor);
            let exclude_self = read_u8(cursor) != 0;
            let location = decode_zone(read_u8(cursor));
            let multiple_targets = read_u8(cursor) != 0;
            let per_unit = read_u8(cursor) != 0;
            let per_unit_count = read_u8(cursor);
            let per_unit_type = read_str(cursor);
            let picker = decode_player(read_u8(cursor));
            let self_target = read_u8(cursor) != 0;
            let source = decode_zone(read_u8(cursor));
            let target = decode_player(read_u8(cursor));
            let mut ek = default_lookReveal();
            if let EffectKind::LookReveal { activation_position: ref mut _bc_activation_position, blind: ref mut _bc_blind, card_type: ref mut _bc_card_type, cost_limit: ref mut _bc_cost_limit, cost_limit_operator: ref mut _bc_cost_limit_operator, exclude_self: ref mut _bc_exclude_self, location: ref mut _bc_location, multiple_targets: ref mut _bc_multiple_targets, per_unit: ref mut _bc_per_unit, per_unit_count: ref mut _bc_per_unit_count, per_unit_type: ref mut _bc_per_unit_type, picker: ref mut _bc_picker, self_target: ref mut _bc_self_target, source: ref mut _bc_source, target: ref mut _bc_target, .. } = &mut ek {
                *_bc_activation_position = Some(activation_position.into());
                *_bc_blind = Some(blind);
                *_bc_card_type = Some(card_type.into());
                *_bc_cost_limit = Some(cost_limit as u32);
                *_bc_cost_limit_operator = Some(decode_operator_from_str(cost_limit_operator));
                *_bc_exclude_self = Some(exclude_self);
                *_bc_location = Some(location.into());
                *_bc_multiple_targets = Some(multiple_targets);
                *_bc_per_unit = Some(per_unit);
                *_bc_per_unit_count = Some(per_unit_count as u32);
                *_bc_per_unit_type = per_unit_type.map(|s| s.into());
                *_bc_picker = Some(picker.into());
                *_bc_self_target = Some(self_target);
                *_bc_source = Some(source.into());
                *_bc_target = Some(target.into());
            }
            Some(Box::new(ek))
        }
        Opcode::RevealUntilLiveCard => {
            let all = read_u8(cursor) != 0;
            let source = decode_zone(read_u8(cursor));
            let target = decode_player(read_u8(cursor));
            let mut ek = default_lookReveal();
            if let EffectKind::LookReveal { source: ref mut _bc_source, target: ref mut _bc_target, .. } = &mut ek {
                *_bc_source = Some(source.into());
                *_bc_target = Some(target.into());
            }
            Some(Box::new(ek))
        }
        Opcode::Select => {
            let ability_filter = read_str(cursor);
            let ability_filter_triggers = read_str(cursor);
            let action_by = decode_player(read_u8(cursor));
            let activation_position = decode_zone(read_u8(cursor));
            let all = read_u8(cursor) != 0;
            let card_type = decode_card_type(read_u8(cursor));
            let characters = read_str(cursor);
            let cost_limit = read_u8(cursor);
            let cost_limit_operator = decode_operator(read_u8(cursor));
            let count = read_u8(cursor);
            let distinct = read_str(cursor);
            let duration = decode_duration(read_u8(cursor));
            let exclude_selected = read_u8(cursor) != 0;
            let exclude_self = read_u8(cursor) != 0;
            let group_names = read_str(cursor);
            let heart_colors = decode_heart(read_u8(cursor));
            let optional = read_u8(cursor) != 0;
            let or_card_types = decode_card_type(read_u8(cursor));
            let source = decode_zone(read_u8(cursor));
            let target = decode_player(read_u8(cursor));
            let mut ek = default_selectTarget();
            if let EffectKind::SelectTarget { ability_filter_triggers: ref mut _bc_ability_filter_triggers, action_by: ref mut _bc_action_by, activation_position: ref mut _bc_activation_position, card_type: ref mut _bc_card_type, characters: ref mut _bc_characters, cost_limit: ref mut _bc_cost_limit, cost_limit_operator: ref mut _bc_cost_limit_operator, exclude_selected: ref mut _bc_exclude_selected, exclude_self: ref mut _bc_exclude_self, group_names: ref mut _bc_group_names, heart_colors: ref mut _bc_heart_colors, optional: ref mut _bc_optional, or_card_types: ref mut _bc_or_card_types, source: ref mut _bc_source, target: ref mut _bc_target, .. } = &mut ek {
                *_bc_ability_filter_triggers = ability_filter_triggers.map(|s| vec![s.to_string()]);
                *_bc_action_by = Some(action_by.into());
                *_bc_activation_position = Some(activation_position.into());
                *_bc_card_type = Some(card_type.into());
                *_bc_characters = characters.map(|s| Box::new(vec![s.to_string()]));
                *_bc_cost_limit = Some(cost_limit as u32);
                *_bc_cost_limit_operator = Some(decode_operator_from_str(cost_limit_operator));
                *_bc_exclude_selected = Some(exclude_selected);
                *_bc_exclude_self = Some(exclude_self);
                *_bc_group_names = group_names.map(|s| Box::new(vec![s.to_string()]));
                *_bc_heart_colors = Box::new(vec![heart_colors.to_string()]);
                *_bc_optional = Some(optional);
                *_bc_or_card_types = Some(Box::new(vec![or_card_types.to_string()]));
                *_bc_source = Some(source.into());
                *_bc_target = Some(target.into());
            }
            Some(Box::new(ek))
        }
        Opcode::SelectCards => {
            let any_number = read_u8(cursor) != 0;
            let card_type = decode_card_type(read_u8(cursor));
            let characters = read_str(cursor);
            let cost_limit = read_u8(cursor);
            let cost_limit_operator = decode_operator(read_u8(cursor));
            let count = read_u8(cursor);
            let destination = decode_zone(read_u8(cursor));
            let discard_remaining = read_u8(cursor) != 0;
            let exclude_self = read_u8(cursor) != 0;
            let group_names = read_str(cursor);
            let heart_color_count = read_u8(cursor);
            let heart_colors = decode_heart(read_u8(cursor));
            let max = read_u8(cursor) != 0;
            let optional = read_u8(cursor) != 0;
            let or_card_types = decode_card_type(read_u8(cursor));
            let original_value = read_u8(cursor) != 0;
            let per_group = read_u8(cursor) != 0;
            let per_group_count = read_u8(cursor);
            let placement_order = read_str(cursor);
            let remainder_destination = decode_zone(read_u8(cursor));
            let require_all_heart_colors = read_u8(cursor) != 0;
            let reveal = read_u8(cursor) != 0;
            let source = decode_zone(read_u8(cursor));
            let mut ek = default_selectTarget();
            if let EffectKind::SelectTarget { any_number: ref mut _bc_any_number, card_type: ref mut _bc_card_type, characters: ref mut _bc_characters, cost_limit: ref mut _bc_cost_limit, cost_limit_operator: ref mut _bc_cost_limit_operator, destination: ref mut _bc_destination, discard_remaining: ref mut _bc_discard_remaining, exclude_self: ref mut _bc_exclude_self, group_names: ref mut _bc_group_names, heart_color_count: ref mut _bc_heart_color_count, heart_colors: ref mut _bc_heart_colors, optional: ref mut _bc_optional, or_card_types: ref mut _bc_or_card_types, original_value: ref mut _bc_original_value, per_group: ref mut _bc_per_group, per_group_count: ref mut _bc_per_group_count, require_all_heart_colors: ref mut _bc_require_all_heart_colors, reveal: ref mut _bc_reveal, source: ref mut _bc_source, .. } = &mut ek {
                *_bc_any_number = Some(any_number);
                *_bc_card_type = Some(card_type.into());
                *_bc_characters = characters.map(|s| Box::new(vec![s.to_string()]));
                *_bc_cost_limit = Some(cost_limit as u32);
                *_bc_cost_limit_operator = Some(decode_operator_from_str(cost_limit_operator));
                *_bc_destination = Some(destination.into());
                *_bc_discard_remaining = Some(discard_remaining);
                *_bc_exclude_self = Some(exclude_self);
                *_bc_group_names = group_names.map(|s| Box::new(vec![s.to_string()]));
                *_bc_heart_color_count = Some(heart_color_count as u32);
                *_bc_heart_colors = Box::new(vec![heart_colors.to_string()]);
                *_bc_optional = Some(optional);
                *_bc_or_card_types = Some(Box::new(vec![or_card_types.to_string()]));
                *_bc_original_value = Some(original_value);
                *_bc_per_group = Some(per_group);
                *_bc_per_group_count = Some(per_group_count as u32);
                *_bc_require_all_heart_colors = Some(require_all_heart_colors);
                *_bc_reveal = Some(reveal);
                *_bc_source = Some(source.into());
            }
            Some(Box::new(ek))
        }
        Opcode::SelectNumber => {
            let count = read_u8(cursor);
            let mut ek = default_selectTarget();
            Some(Box::new(ek))
        }
        Opcode::SetBladeCount => {
            let card_type = decode_card_type(read_u8(cursor));
            let count = read_u8(cursor);
            let duration = decode_duration(read_u8(cursor));
            let group_names = read_str(cursor);
            let original_value = read_u8(cursor) != 0;
            let position = decode_zone(read_u8(cursor));
            let target = decode_player(read_u8(cursor));
            let mut ek = default_miscOp();
            if let EffectKind::MiscOp { card_type: ref mut _bc_card_type, duration: ref mut _bc_duration, group_names: ref mut _bc_group_names, original_value: ref mut _bc_original_value, target: ref mut _bc_target, .. } = &mut ek {
                *_bc_card_type = Some(card_type.into());
                *_bc_duration = Some(duration.into());
                *_bc_group_names = group_names.map(|s| Box::new(vec![s.to_string()]));
                *_bc_original_value = Some(original_value);
                *_bc_target = Some(target.into());
            }
            Some(Box::new(ek))
        }
        Opcode::SetBladeType => {
            let blade_type = read_str(cursor);
            let duration = decode_duration(read_u8(cursor));
            let mut ek = default_customOp();
            if let EffectKind::CustomOp { duration: ref mut _bc_duration, .. } = &mut ek {
                *_bc_duration = Some(duration.into());
            }
            Some(Box::new(ek))
        }
        Opcode::SetCardIdentity => {
            let all = read_u8(cursor) != 0;
            let all_regions = read_u8(cursor) != 0;
            let group_names = read_str(cursor);
            let identities = read_str(cursor);
            let self_target = read_u8(cursor) != 0;
            let mut ek = default_changeState();
            if let EffectKind::ChangeState { all: ref mut _bc_all, all_regions: ref mut _bc_all_regions, group_names: ref mut _bc_group_names, identities: ref mut _bc_identities, self_target: ref mut _bc_self_target, .. } = &mut ek {
                *_bc_all = Some(all);
                *_bc_all_regions = Some(all_regions);
                *_bc_group_names = group_names.map(|s| Box::new(vec![s.to_string()]));
                *_bc_identities = identities.map(|s| Box::new(vec![s.to_string()]));
                *_bc_self_target = Some(self_target);
            }
            Some(Box::new(ek))
        }
        Opcode::SetHeartType => {
            let card_type = decode_card_type(read_u8(cursor));
            let count = read_u8(cursor);
            let duration = decode_duration(read_u8(cursor));
            let group_names = read_str(cursor);
            let heart_colors = decode_heart(read_u8(cursor));
            let heart_type = read_str(cursor);
            let original_value = read_u8(cursor) != 0;
            let self_target = read_u8(cursor) != 0;
            let source = decode_zone(read_u8(cursor));
            let target = decode_player(read_u8(cursor));
            let mut ek = default_miscOp();
            if let EffectKind::MiscOp { card_type: ref mut _bc_card_type, duration: ref mut _bc_duration, group_names: ref mut _bc_group_names, heart_colors: ref mut _bc_heart_colors, heart_type: ref mut _bc_heart_type, original_value: ref mut _bc_original_value, self_target: ref mut _bc_self_target, source: ref mut _bc_source, target: ref mut _bc_target, .. } = &mut ek {
                *_bc_card_type = Some(card_type.into());
                *_bc_duration = Some(duration.into());
                *_bc_group_names = group_names.map(|s| Box::new(vec![s.to_string()]));
                *_bc_heart_colors = Box::new(vec![heart_colors.to_string()]);
                *_bc_heart_type = heart_type.map(|s| s.into());
                *_bc_original_value = Some(original_value);
                *_bc_self_target = Some(self_target);
                *_bc_source = Some(source.into());
                *_bc_target = Some(target.into());
            }
            Some(Box::new(ek))
        }
        Opcode::SpecifyHeartColor => {
            let activation_position = decode_zone(read_u8(cursor));
            let choice = read_u8(cursor) != 0;
            let count = read_u8(cursor);
            let position = decode_zone(read_u8(cursor));
            let target = decode_player(read_u8(cursor));
            let mut ek = default_miscOp();
            if let EffectKind::MiscOp { activation_position: ref mut _bc_activation_position, choice: ref mut _bc_choice, target: ref mut _bc_target, .. } = &mut ek {
                *_bc_activation_position = Some(activation_position.into());
                *_bc_choice = Some(choice);
                *_bc_target = Some(target.into());
            }
            Some(Box::new(ek))
        }
        Opcode::SuppressAbilityTrigger => {
            let card_type = decode_card_type(read_u8(cursor));
            let source = decode_zone(read_u8(cursor));
            let suppressed_trigger = read_str(cursor);
            let target = decode_player(read_u8(cursor));
            let mut ek = default_abilityOp();
            if let EffectKind::AbilityOp { card_type: ref mut _bc_card_type, source: ref mut _bc_source, suppressed_trigger: ref mut _bc_suppressed_trigger, target: ref mut _bc_target, .. } = &mut ek {
                *_bc_card_type = Some(card_type.into());
                *_bc_source = Some(source.into());
                *_bc_suppressed_trigger = suppressed_trigger.map(|s| s.into());
                *_bc_target = Some(target.into());
            }
            Some(Box::new(ek))
        }
        _ => None,
    }
}

fn action_for_op(op: Opcode) -> &'static str {
    match op {
        Opcode::ActivateAbility => "activate_ability",
        Opcode::ChangeState => "change_state",
        Opcode::ChooseTargetPlayer => "choose_target_player",
        Opcode::ConditionalAlternative => "conditional_alternative",
        Opcode::ConditionalOnOptional => "conditional_on_optional",
        Opcode::ConditionalOnResult => "conditional_on_result",
        Opcode::DiscardUntilCount => "discard_until_count",
        Opcode::DoNothing => "do_nothing",
        Opcode::DrawCard => "draw_card",
        Opcode::DrawUntilCount => "draw_until_count",
        Opcode::GainAbility => "gain_ability",
        Opcode::GainAbilityFromSource => "gain_ability_from_source",
        Opcode::GainResource => "gain_resource",
        Opcode::InvalidateAbility => "invalidate_ability",
        Opcode::LookAt => "look_at",
        Opcode::ModifyCost => "modify_cost",
        Opcode::ModifyRequiredHearts => "modify_required_hearts",
        Opcode::ModifyRequiredHeartsGlobal => "modify_required_hearts_global",
        Opcode::ModifyScore => "modify_score",
        Opcode::ModifyYellCount => "modify_yell_count",
        Opcode::MoveCards => "move_cards",
        Opcode::PayEnergy => "pay_energy",
        Opcode::PerformYell => "perform_yell",
        Opcode::PlaceEnergyUnderMember => "place_energy_under_member",
        Opcode::PlayBatonTouch => "play_baton_touch",
        Opcode::PositionChange => "position_change",
        Opcode::ReYell => "re_yell",
        Opcode::ReduceLiveCardSetLimit => "reduce_live_card_set_limit",
        Opcode::RepeatProcedure => "repeat_procedure",
        Opcode::Restriction => "restriction",
        Opcode::Reveal => "reveal",
        Opcode::RevealUntilLiveCard => "reveal_until_live_card",
        Opcode::Select => "select",
        Opcode::SelectCards => "select_cards",
        Opcode::SelectNumber => "select_number",
        Opcode::SetBladeCount => "set_blade_count",
        Opcode::SetBladeType => "set_blade_type",
        Opcode::SetCardIdentity => "set_card_identity",
        Opcode::SetHeartType => "set_heart_type",
        Opcode::SpecifyHeartColor => "specify_heart_color",
        Opcode::SuppressAbilityTrigger => "suppress_ability_trigger",
        _ => "",
    }
}

fn decode_operator_from_str(s: &str) -> Operator {
    match s { ">=" => Operator::Gte, "<=" => Operator::Lte, ">" => Operator::Gt, "<" => Operator::Lt, "=" => Operator::Eq, _ => Operator::Eq }
}

fn decode_cond_card_type(v: u8) -> ConditionCardType {
    match v { 1 => ConditionCardType::MemberCard, 2 => ConditionCardType::LiveCard, 3 => ConditionCardType::EnergyCard, _ => ConditionCardType::MemberCard }
}

pub fn decode_condition(cursor: &mut &[u8]) -> Condition {
    if cursor.is_empty() { return default_condition_alwaysTrue(); }
    let op_val = cursor[0];
    match op_val {
        81 => {
            let _ = read_u8(cursor);
            let text = read_str(cursor);
            let mut c = default_condition_abilityFilter();
            if let Condition::AbilityFilter { text: ref mut _bc_text, .. } = &mut c {
                *_bc_text = text.map(|s| s.to_string());
            }
            c
        }
        80 => {
            let _ = read_u8(cursor);
            let operator = decode_operator(read_u8(cursor));
            let count = read_u16(cursor);
            let mut c = default_condition_comparison();
            if let Condition::Comparison { operator: ref mut _bc_operator, count: ref mut _bc_count, .. } = &mut c {
                *_bc_operator = Some(operator.into());
                *_bc_count = Some(count as u32);
            }
            c
        }
        70 => {
            let _ = read_u8(cursor);
            let location = decode_zone(read_u8(cursor));
            let count = read_u8(cursor);
            let mut c = default_condition_appearance();
            if let Condition::Appearance { location: ref mut _bc_location, .. } = &mut c {
                *_bc_location = Some(location.into());
            }
            c
        }
        79 => {
            let _ = read_u8(cursor);
            let operator = decode_operator(read_u8(cursor));
            let count = read_u8(cursor);
            let mut c = default_condition_resource();
            if let Condition::Resource { operator: ref mut _bc_operator, count: ref mut _bc_count, .. } = &mut c {
                *_bc_operator = Some(operator.into());
                *_bc_count = Some(count as u32);
            }
            c
        }
        64 => {
            let _ = read_u8(cursor);
            let location = decode_zone(read_u8(cursor));
            let operator = decode_operator(read_u8(cursor));
            let count = read_u8(cursor);
            let card_type = read_u8(cursor);
            let group_names = read_str(cursor);
            let target = decode_player(read_u8(cursor));
            let mut c = default_condition_location();
            if let Condition::Location { location: ref mut _bc_location, operator: ref mut _bc_operator, count: ref mut _bc_count, card_type: ref mut _bc_card_type, group_names: ref mut _bc_group_names, target: ref mut _bc_target, .. } = &mut c {
                *_bc_location = Some(location.into());
                *_bc_operator = Some(operator.into());
                *_bc_count = Some(count as u32);
                *_bc_card_type = Some(decode_cond_card_type(card_type));
                *_bc_group_names = group_names.map(|s| Box::new(vec![s.to_string()]));
                *_bc_target = Some(target.into());
            }
            c
        }
        66 => {
            let _ = read_u8(cursor);
            let location = decode_zone(read_u8(cursor));
            let comparison_type = read_str(cursor);
            let aggregate = read_str(cursor);
            let operator = decode_operator(read_u8(cursor));
            let count = read_u16(cursor);
            let target = decode_player(read_u8(cursor));
            let resource_type = decode_resource(read_u8(cursor));
            let card_type = read_u8(cursor);
            let group_names = read_str(cursor);
            let mut c = default_condition_comparison();
            if let Condition::Comparison { location: ref mut _bc_location, aggregate: ref mut _bc_aggregate, operator: ref mut _bc_operator, count: ref mut _bc_count, target: ref mut _bc_target, resource_type: ref mut _bc_resource_type, card_type: ref mut _bc_card_type, group_names: ref mut _bc_group_names, .. } = &mut c {
                *_bc_location = Some(location.into());
                *_bc_aggregate = aggregate.map(|s| s.into());
                *_bc_operator = Some(operator.into());
                *_bc_count = Some(count as u32);
                *_bc_target = Some(target.into());
                *_bc_resource_type = Some(resource_type.into());
                *_bc_card_type = Some(decode_cond_card_type(card_type));
                *_bc_group_names = group_names.map(|s| Box::new(vec![s.to_string()]));
            }
            c
        }
        72 => {
            let _ = read_u8(cursor);
            let operator = decode_operator(read_u8(cursor));
            let count = read_u8(cursor);
            let mut c = default_condition_state();
            if let Condition::State { operator: ref mut _bc_operator, count: ref mut _bc_count, .. } = &mut c {
                *_bc_operator = Some(operator.into());
                *_bc_count = Some(count as u32);
            }
            c
        }
        67 => {
            let _ = read_u8(cursor);
            let group_names = read_str(cursor);
            let count = read_u8(cursor);
            let operator = decode_operator(read_u8(cursor));
            let mut c = default_condition_group();
            if let Condition::Group { group_names: ref mut _bc_group_names, count: ref mut _bc_count, operator: ref mut _bc_operator, .. } = &mut c {
                *_bc_group_names = group_names.map(|s| Box::new(vec![s.to_string()]));
                *_bc_count = Some(count as u32);
                *_bc_operator = Some(operator.into());
            }
            c
        }
        82 => {
            let _ = read_u8(cursor);
            let position = decode_zone(read_u8(cursor));
            let group_names = read_str(cursor);
            let mut c = default_condition_movement();
            if let Condition::Movement { group_names: ref mut _bc_group_names, .. } = &mut c {
                *_bc_group_names = group_names.map(|s| Box::new(vec![s.to_string()]));
            }
            c
        }
        77 => {
            let _ = read_u8(cursor);
            let mut c = default_condition_scoreThreshold();
            c
        }
        65 => {
            let _ = read_u8(cursor);
            let location = decode_zone(read_u8(cursor));
            let card_type = read_u8(cursor);
            let exclude_self = read_u8(cursor) != 0;
            let target = decode_player(read_u8(cursor));
            let mut c = default_condition_location();
            if let Condition::Location { location: ref mut _bc_location, card_type: ref mut _bc_card_type, exclude_self: ref mut _bc_exclude_self, target: ref mut _bc_target, .. } = &mut c {
                *_bc_location = Some(location.into());
                *_bc_card_type = Some(decode_cond_card_type(card_type));
                *_bc_exclude_self = Some(exclude_self);
                *_bc_target = Some(target.into());
            }
            c
        }
        68 => {
            let _ = read_u8(cursor);
            let location = decode_zone(read_u8(cursor));
            let card_type = read_u8(cursor);
            let count = read_u8(cursor);
            let operator = decode_operator(read_u8(cursor));
            let mut c = default_condition_movement();
            if let Condition::Movement { location: ref mut _bc_location, card_type: ref mut _bc_card_type, operator: ref mut _bc_operator, .. } = &mut c {
                *_bc_location = Some(location.into());
                *_bc_card_type = Some(decode_cond_card_type(card_type));
                *_bc_operator = Some(operator.into());
            }
            c
        }
        85 => {
            let _ = read_u8(cursor);
            let mut c = default_condition_noExcessHeart();
            c
        }
        83 => {
            let _ = read_u8(cursor);
            let mut c = default_condition_movement();
            c
        }
        84 => {
            let _ = read_u8(cursor);
            let no_excess_heart = read_u8(cursor) != 0;
            let mut c = default_condition_opponentLiveSuccess();
            if let Condition::OpponentLiveSuccess { no_excess_heart: ref mut _bc_no_excess_heart, .. } = &mut c {
                *_bc_no_excess_heart = Some(no_excess_heart);
            }
            c
        }
        73 => {
            let _ = read_u8(cursor);
            let location = decode_zone(read_u8(cursor));
            let mut c = default_condition_positionCond();
            c
        }
        78 => {
            let _ = read_u8(cursor);
            let state_change = decode_state(read_u8(cursor));
            let mut c = default_condition_state();
            c
        }
        71 => {
            let _ = read_u8(cursor);
            let state = decode_state(read_u8(cursor));
            let operator = decode_operator(read_u8(cursor));
            let value = read_u8(cursor) != 0;
            let mut c = default_condition_state();
            if let Condition::State { state: ref mut _bc_state, operator: ref mut _bc_operator, .. } = &mut c {
                *_bc_state = Some(state.into());
                *_bc_operator = Some(operator.into());
            }
            c
        }
        69 => {
            let _ = read_u8(cursor);
            let count = read_u8(cursor);
            let operator = decode_operator(read_u8(cursor));
            let mut c = default_condition_temporal();
            if let Condition::Temporal { count: ref mut _bc_count, .. } = &mut c {
                *_bc_count = Some(count as u32);
            }
            c
        }
        0x4A | 0x4B => {
            let _ = read_u8(cursor);
            let op_str = if op_val == 0x4A { "or" } else { "and" };
            let mut conditions = Vec::new();
            loop {
                if cursor.is_empty() || cursor[0] == 0x4C {
                    if !cursor.is_empty() { let _ = read_u8(cursor); }
                    break;
                }
                conditions.push(Box::new(decode_condition(cursor)));
            }
            if conditions.is_empty() { default_condition_alwaysTrue() }
            else if conditions.len() == 1 { *conditions.into_iter().next().unwrap() }
            else { let mut c = default_condition_compound();
                if let Condition::Compound { operator: ref mut _bc_o, conditions: ref mut _bc_cond, .. } = &mut c {
                    *_bc_o = Some(op_str.into()); *_bc_cond = Some(conditions);
                } c
            }
        }
        _ => default_condition_alwaysTrue(),
    }
}
