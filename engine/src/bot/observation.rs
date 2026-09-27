use std::hash::{Hash, Hasher};

use crate::ability::types::Choice;
use crate::game_state::{GameResult, GameState, Phase, TurnPhase};

pub const MATH_FEATURES: usize = 32;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde_support", derive(serde::Serialize, serde::Deserialize))]
pub struct PublicObservation {
    pub me: PlayerView,
    pub opp: PlayerView,
    pub current_phase: Phase,
    pub current_turn_phase: TurnPhase,
    pub turn_number: u8,
    pub game_result: GameResult,
    pub active_player: u8,
    pub can_current_player_act: bool,
    pub pending_choice: bool,
    pub pending_choice_kind: Option<String>,
    pub pending_choice_option_count: usize,
    pub pending_choice_allow_skip: bool,
    pub pending_choice_player: Option<u8>,
    pub pending_choice_for_viewer: bool,
    pub mulligan_selected_count: u8,
    pub live_card_selected_count: u8,
    pub ability_queue_len: u8,
    pub ability_queue_waiting: bool,
    pub ability_queue_current_card: Option<i16>,
    pub ability_queue_current_ability: Option<u8>,
    pub ability_queue_current_trigger: Option<u8>,
    pub math_features: [f32; MATH_FEATURES],
    pub resolution_zone: Vec<i16>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde_support", derive(serde::Serialize, serde::Deserialize))]
pub struct PlayerView {
    pub hand: Vec<i16>,
    pub hand_size: usize,
    pub stage: [i16; 3],
    pub stage_waited: [bool; 3],
    pub under_cards: [Vec<i16>; 3],
    pub energy_zone: Vec<i16>,
    pub active_energy_count: usize,
    pub waitroom: Vec<i16>,
    pub success_zone: Vec<i16>,
    pub live_zone: Vec<i16>,
    pub main_deck_size: usize,
    pub main_deck_composition: Vec<i16>,
    pub energy_deck_size: usize,
    pub is_first_attacker: bool,
    pub features: PlayerFeatures,
    pub stage_abilities: [AbilityFeatures; 3],
}

#[derive(Debug, Clone, Copy, Default, Hash)]
#[cfg_attr(feature = "serde_support", derive(serde::Serialize, serde::Deserialize))]
pub struct AbilityFeatures {
    pub printed: u8,
    pub gained: u8,
    pub activation: u8,
    pub debut: u8,
    pub live_start: u8,
    pub live_success: u8,
    pub constant: u8,
    pub auto: u8,
    pub invalidated: u8,
    pub usable_activation: u8,
    pub limited_total: u8,
    pub limited_used: u8,
    pub queued: u8,
}

#[derive(Debug, Clone, Default, Hash)]
#[cfg_attr(feature = "serde_support", derive(serde::Serialize, serde::Deserialize))]
pub struct PlayerFeatures {
    pub stage_cost: u16,
    pub active_blades: u8,
    pub total_blades: u8,
    pub stage_hearts: u16,
    pub active_stage_slots: u8,
    pub waited_stage_slots: u8,
    pub under_member_count: u8,
    pub under_energy_count: u8,
    pub live_score: u16,
    pub success_count: u8,
    pub abilities: AbilityFeatures,
}

fn add_ability_trigger_count(features: &mut AbilityFeatures, ability: &crate::card::Ability) {
    use crate::triggers::TriggerKind;
    if ability.has_trigger(TriggerKind::Activation) {
        features.activation = features.activation.saturating_add(1);
    }
    if ability.has_trigger(TriggerKind::Debut) {
        features.debut = features.debut.saturating_add(1);
    }
    if ability.has_trigger(TriggerKind::LiveStart) {
        features.live_start = features.live_start.saturating_add(1);
    }
    if ability.has_trigger(TriggerKind::LiveSuccess) {
        features.live_success = features.live_success.saturating_add(1);
    }
    if ability.has_trigger(TriggerKind::Constant) {
        features.constant = features.constant.saturating_add(1);
    }
    if ability.has_trigger(TriggerKind::Auto) {
        features.auto = features.auto.saturating_add(1);
    }
}

fn merge_ability_features(dst: &mut AbilityFeatures, src: AbilityFeatures) {
    dst.printed = dst.printed.saturating_add(src.printed);
    dst.gained = dst.gained.saturating_add(src.gained);
    dst.activation = dst.activation.saturating_add(src.activation);
    dst.debut = dst.debut.saturating_add(src.debut);
    dst.live_start = dst.live_start.saturating_add(src.live_start);
    dst.live_success = dst.live_success.saturating_add(src.live_success);
    dst.constant = dst.constant.saturating_add(src.constant);
    dst.auto = dst.auto.saturating_add(src.auto);
    dst.invalidated = dst.invalidated.saturating_add(src.invalidated);
    dst.usable_activation = dst.usable_activation.saturating_add(src.usable_activation);
    dst.limited_total = dst.limited_total.saturating_add(src.limited_total);
    dst.limited_used = dst.limited_used.saturating_add(src.limited_used);
    dst.queued = dst.queued.saturating_add(src.queued);
}

fn card_ability_features(state: &GameState, card_id: i16, include_runtime: bool) -> AbilityFeatures {
    let mut features = AbilityFeatures::default();
    let Some(card) = state.card_database.get_card(card_id) else {
        return features;
    };
    for (ability_index, ability) in card.resolved_abilities().enumerate() {
        features.printed = features.printed.saturating_add(1);
        add_ability_trigger_count(&mut features, ability.as_ref());
        if let Some(limit) = ability.use_limit {
            features.limited_total = features.limited_total.saturating_add(limit);
            features.limited_used = features
                .limited_used
                .saturating_add(state.ability_uses_used(card_id, ability_index));
        }
        if ability.has_trigger(crate::triggers::TriggerKind::Activation)
            && !state.is_ability_invalidated(card_id, &crate::core::types::AbilityTrigger::Activation)
            && state.ability_has_remaining_uses(card_id, ability_index)
        {
            features.usable_activation = features.usable_activation.saturating_add(1);
        }
        if state.is_ability_invalidated(card_id, &crate::core::types::AbilityTrigger::Activation)
            || state.is_ability_invalidated(card_id, &crate::core::types::AbilityTrigger::Debut)
            || state.is_ability_invalidated(card_id, &crate::core::types::AbilityTrigger::LiveStart)
            || state.is_ability_invalidated(card_id, &crate::core::types::AbilityTrigger::LiveSuccess)
            || state.is_ability_invalidated(card_id, &crate::core::types::AbilityTrigger::Constant)
            || state.is_ability_invalidated(card_id, &crate::core::types::AbilityTrigger::Auto)
        {
            features.invalidated = features.invalidated.saturating_add(1);
        }
    }
    if include_runtime {
        if let Some(gained) = state.gained_card_abilities.get(&card_id) {
            for (ability_index, ability) in gained.iter().enumerate() {
                features.gained = features.gained.saturating_add(1);
                add_ability_trigger_count(&mut features, ability);
                if let Some(limit) = ability.use_limit {
                    features.limited_total = features.limited_total.saturating_add(limit);
                    features.limited_used = features
                        .limited_used
                        .saturating_add(state.ability_uses_used(card_id, ability_index));
                }
                if ability.has_trigger(crate::triggers::TriggerKind::Activation)
                    && state
                        .ability_uses_used(card_id, ability_index)
                        < ability.use_limit.unwrap_or(u8::MAX)
                {
                    features.usable_activation = features.usable_activation.saturating_add(1);
                }
            }
        }
        for entry in state.ability_queue.iter() {
            if entry.card_id == Some(card_id) {
                features.queued = features.queued.saturating_add(1);
            }
        }
    }
    features
}

fn aggregate_ability_features(
    state: &GameState,
    card_ids: &[i16],
    include_runtime: bool,
) -> AbilityFeatures {
    let mut features = AbilityFeatures::default();
    for &card_id in card_ids {
        if card_id >= 0 {
            merge_ability_features(
                &mut features,
                card_ability_features(state, card_id, include_runtime),
            );
        }
    }
    features
}

fn player_features(
    state: &GameState,
    player: &crate::player::Player,
    live_cards: &[i16],
    ability_cards: &[i16],
    include_runtime: bool,
) -> PlayerFeatures {
    let stage = &player.stage;
    let stage_cost = stage
        .stage
        .iter()
        .filter_map(|&card_id| {
            state
                .card_database
                .get_card(card_id)
                .and_then(|card| card.cost)
        })
        .fold(0u16, |sum, cost| sum.saturating_add(u16::from(cost)));
    let active_blades = stage.total_blades(
        &state.card_database,
        &state.mods.blade_modifiers,
        &state.mods.orientation_modifiers,
        false,
    );
    let total_blades = stage.total_blades(
        &state.card_database,
        &state.mods.blade_modifiers,
        &state.mods.orientation_modifiers,
        true,
    );
    let hearts = player.calculate_stage_hearts(
        &state.card_database,
        &state.mods.heart_color_multiplier,
        &state.mods.heart_override,
        &state.mods.heart_modifiers,
        &state.mods.heart_copy,
    );
    let mut features = PlayerFeatures {
        stage_cost,
        active_blades,
        total_blades,
        stage_hearts: u16::from(hearts.hearts.values_sum()),
        ..PlayerFeatures::default()
    };
    features.abilities = aggregate_ability_features(state, ability_cards, include_runtime);
    for &card_id in &stage.stage {
        if card_id == -1 {
            continue;
        }
        let waited = state
            .mods
            .orientation_modifiers
            .get(&card_id)
            .map(|orientation| *orientation == crate::core::game_modifiers::CardOrientation::Wait)
            .unwrap_or(false);
        if waited {
            features.waited_stage_slots = features.waited_stage_slots.saturating_add(1);
        } else {
            features.active_stage_slots = features.active_stage_slots.saturating_add(1);
        }
    }
    for under in &stage.under_cards {
        for &card_id in under {
            if state
                .card_database
                .get_card(card_id)
                .is_some_and(|card| card.is_member())
            {
                features.under_member_count = features.under_member_count.saturating_add(1);
            } else {
                features.under_energy_count = features.under_energy_count.saturating_add(1);
            }
        }
    }
    for &card_id in live_cards {
        let Some(card) = state.card_database.get_card(card_id) else {
            continue;
        };
        let base = i32::from(card.get_score());
        let set = state.mods.get_score_set_modifier(card_id);
        let additive = state.mods.get_score_modifier(card_id) - state.mods.get_score_set_modifier(card_id);
        let effective = if set != 0 { set } else { base + additive };
        features.live_score = features
            .live_score
            .saturating_add(effective.max(0) as u16);
    }
    features.success_count = player.success_live_card_zone.cards.len().min(u8::MAX as usize) as u8;
    features
}

fn math_features(state: &GameState, perspective_player: u8) -> [f32; MATH_FEATURES] {
    use crate::bot::strategy_common::{acc_add, requirements_met, Acc};
    use crate::bot::strategy_v4;
    use crate::bot::strategy_v5;
    use crate::card::CardType;
    use crate::core::stats_pipeline;

    let me = if perspective_player == 0 { &state.player1 } else { &state.player2 };
    let opp = if perspective_player == 0 { &state.player2 } else { &state.player1 };
    let db = &state.card_database;
    let (blades, density) = strategy_v4::flip_stats(state, perspective_player, db);
    let expected = strategy_v4::expected_flip_units(state, perspective_player, db);
    let pool_board = strategy_v4::heart_pool_inner(state, perspective_player, db, 0.0);
    let mut out = [0.0f32; MATH_FEATURES];
    let stage_cost: i32 = me
        .stage
        .stage
        .iter()
        .filter_map(|&id| db.get_card(id).and_then(|card| card.cost))
        .map(i32::from)
        .sum();
    let active_slots = me
        .stage
        .stage
        .iter()
        .filter(|&&id| id >= 0)
        .filter(|&&id| state.mods.get_orientation_modifier(id) != Some("wait"))
        .count();
    let waited_slots = me.stage.stage.iter().filter(|&&id| id >= 0).count() - active_slots;
    out[0] = stage_cost as f32 / 30.0;
    out[1] = blades as f32 / 12.0;
    out[2] = me
        .stage
        .total_blades(db, &state.mods.blade_modifiers, &state.mods.orientation_modifiers, true) as f32
        / 12.0;
    out[3] = me.energy_zone.active_count() as f32 / 15.0;
    out[4] = me.energy_zone.cards.len() as f32 / 20.0;
    out[5] = me.hand.cards.len() as f32 / 10.0;
    out[7] = me.success_live_card_zone.cards.len() as f32 / 3.0;
    out[8] = opp.success_live_card_zone.cards.len() as f32 / 3.0;
    out[9] = if me.is_first_attacker { 1.0 } else { 0.0 };
    out[10] = density as f32;
    out[13] = strategy_v5::estimate_opp_score(state, perspective_player, db) as f32 / 12.0;
    out[14] = active_slots as f32 / 3.0;
    out[15] = waited_slots as f32 / 3.0;
    out[16] = (expected[0] + expected[1] + expected[2] + expected[3] + expected[4] + expected[5] + expected[6] + expected[10]) as f32 / 12.0;
    for i in 0..6 {
        out[17 + i] = expected[i + 1] as f32 / 8.0;
        out[23 + i] = pool_board[i + 1] as f32 / 8.0;
    }
    let mut live_count = 0usize;
    let mut passable_count = 0usize;
    let mut best_live_prob = 0.0f32;
    let mut max_live_score = 0u8;
    for &card_id in &me.hand.cards {
        let Some(card) = db.get_card(card_id) else { continue };
        if card.card_type != CardType::Live {
            continue;
        }
        live_count += 1;
        max_live_score = max_live_score.max(card.get_score());
        let need = stats_pipeline::effective_need_heart(
            card.need_heart.as_ref(),
            card_id,
            &state.mods.need_heart_modifiers,
        );
        let mut required: Acc = [0; 11];
        if let Some(need) = need {
            acc_add(&mut required, &need.hearts);
        }
        let deterministic = requirements_met(&pool_board, &required);
        if deterministic {
            passable_count += 1;
            best_live_prob = 1.0;
            continue;
        }
        let mut shortfall = 0i32;
        for color in 1..=6 {
            shortfall += (required[color] - pool_board[color]).max(0);
        }
        if required[0] > 0 {
            shortfall += required[0];
        }
        let probability = strategy_v5::binom_ge(blades, shortfall, density) as f32;
        best_live_prob = best_live_prob.max(probability);
    }
    out[6] = live_count as f32 / 8.0;
    out[11] = best_live_prob;
    out[12] = max_live_score as f32 / 10.0;
    out[29] = passable_count as f32 / 8.0;
    let mut hand_reserve = 0i32;
    for &card_id in &me.hand.cards {
        if let Some(card) = db.get_card(card_id) {
            if card.card_type == CardType::Member {
                let hearts = card
                    .base_heart
                    .as_ref()
                    .map(|heart| heart.hearts.values_sum())
                    .unwrap_or(0);
                hand_reserve += 2 * i32::from(card.cost.unwrap_or(0))
                    + 2 * i32::from(card.blade)
                    + i32::from(hearts);
            }
        }
    }
    out[30] = hand_reserve as f32 / 100.0;
    out
}

fn pending_choice_features(state: &GameState) -> (Option<&'static str>, usize, bool) {
    match state.get_pending_choice() {
        Some(Choice::SelectCard { count, allow_skip, .. }) => {
            (Some("SelectCard"), *count, *allow_skip)
        }
        Some(Choice::SelectTarget { options, allow_skip, .. }) => {
            (Some("SelectTarget"), options.as_ref().map_or(0, Vec::len), *allow_skip)
        }
        Some(Choice::SelectPosition { allow_skip, .. }) => {
            (Some("SelectPosition"), 3, *allow_skip)
        }
        Some(Choice::SelectHeartColor { options, .. }) => {
            (Some("SelectHeartColor"), options.len(), false)
        }
        Some(Choice::SelectHeartType { options, .. }) => {
            (Some("SelectHeartType"), options.len(), false)
        }
        Some(Choice::SelectAutoAbility { options, .. }) => {
            (Some("SelectAutoAbility"), options.len(), false)
        }
        Some(Choice::SelectLiveSuccess { options, .. }) => {
            (Some("SelectLiveSuccess"), options.len(), false)
        }
        None => (None, 0, false),
    }
}

fn pending_choice_player_index(state: &GameState) -> Option<u8> {
    state
        .get_pending_choice_player_id()
        .and_then(|id| match id.as_str() {
            "p1" | "player1" => Some(0),
            "p2" | "player2" => Some(1),
            _ => None,
        })
        .or_else(|| match state.get_pending_choice() {
            Some(Choice::SelectAutoAbility { player_id, .. })
            | Some(Choice::SelectLiveSuccess { player_id, .. }) => match player_id.as_str() {
                "p1" | "player1" => Some(0),
                "p2" | "player2" => Some(1),
                _ => None,
            },
            Some(Choice::SelectCard {
                target_player_id,
                picker,
                ..
            }) => target_player_id
                .as_deref()
                .or(picker.as_deref())
                .and_then(|id| match id {
                    "p1" | "player1" => Some(0),
                    "p2" | "player2" => Some(1),
                    _ => None,
                }),
            _ => None,
        })
}

fn ability_trigger_index(trigger: &crate::core::types::AbilityTrigger) -> u8 {
    match trigger {
        crate::core::types::AbilityTrigger::Activation => 0,
        crate::core::types::AbilityTrigger::Debut => 1,
        crate::core::types::AbilityTrigger::LiveStart => 2,
        crate::core::types::AbilityTrigger::LiveSuccess => 3,
        crate::core::types::AbilityTrigger::Constant => 4,
        crate::core::types::AbilityTrigger::Auto => 5,
    }
}

fn queue_owner_index(player_id: &str) -> Option<u8> {
    match player_id {
        "p1" | "player1" => Some(0),
        "p2" | "player2" => Some(1),
        _ => None,
    }
}

fn public_card_id(state: &GameState, card_id: Option<i16>, perspective_player: u8) -> bool {
    let Some(card_id) = card_id else {
        return false;
    };
    let players = if perspective_player == 0 {
        [&state.player1, &state.player2]
    } else {
        [&state.player2, &state.player1]
    };
    let own_id = if perspective_player == 0 {
        &state.player1.id
    } else {
        &state.player2.id
    };
    players.iter().any(|player| {
        player.stage.stage.contains(&card_id)
            || player.stage.under_cards.iter().any(|z| z.contains(&card_id))
            || player.waitroom.cards.contains(&card_id)
            || player.live_card_zone.cards.contains(&card_id)
            || player.success_live_card_zone.cards.contains(&card_id)
            || (player.id == *own_id && player.hand.cards.contains(&card_id))
    })
}

fn resolution_visible_to(state: &GameState, perspective_player: u8) -> bool {
    match state.current_phase {
        Phase::FirstAttackerPerformance => {
            (perspective_player == 0) == state.player1.is_first_attacker
        }
        Phase::SecondAttackerPerformance => {
            (perspective_player == 0) != state.player1.is_first_attacker
        }
        Phase::LiveVictoryDetermination => true,
        _ => false,
    }
}

fn is_waited(state: &GameState, card_id: i16) -> bool {
    state
        .mods
        .orientation_modifiers
        .get(&card_id)
        .map(|orientation| *orientation == crate::core::game_modifiers::CardOrientation::Wait)
        .unwrap_or(false)
}

impl PublicObservation {
    /// Build a PVP-correct observation from the perspective of `perspective_player` (0 or 1).
    pub fn from_state(state: &GameState, perspective_player: u8) -> Self {
        let (my_data, opp_data) = if perspective_player == 0 {
            (&state.player1, &state.player2)
        } else {
            (&state.player2, &state.player1)
        };

        let opponent_is_first_attacker = if perspective_player == 0 {
            state.player2.is_first_attacker
        } else {
            state.player1.is_first_attacker
        };

        let opponent_performed = match state.current_phase {
            Phase::LiveVictoryDetermination | Phase::SecondAttackerPerformance => true,
            Phase::FirstAttackerPerformance => opponent_is_first_attacker,
            _ => false,
        };

        let me_live = my_data.live_card_zone.cards.clone();
        let mut me_ability_cards = my_data.hand.cards.to_vec();
        me_ability_cards.extend_from_slice(&my_data.stage.stage);
        for under in &my_data.stage.under_cards {
            me_ability_cards.extend_from_slice(under);
        }
        me_ability_cards.extend_from_slice(&my_data.waitroom.cards);
        me_ability_cards.extend_from_slice(&me_live);
        me_ability_cards.extend_from_slice(&my_data.success_live_card_zone.cards);
        let me_features = player_features(state, my_data, &me_live, &me_ability_cards, true);
        let me_stage_abilities = [
            card_ability_features(state, my_data.stage.stage[0], true),
            card_ability_features(state, my_data.stage.stage[1], true),
            card_ability_features(state, my_data.stage.stage[2], true),
        ];
        let me_waited = [
            is_waited(state, my_data.stage.stage[0]),
            is_waited(state, my_data.stage.stage[1]),
            is_waited(state, my_data.stage.stage[2]),
        ];
        let mut my_main_deck_composition = my_data.main_deck.cards.to_vec();
        my_main_deck_composition.sort_unstable();
        let me = PlayerView {
            hand: my_data.hand.cards.to_vec(),
            hand_size: my_data.hand.cards.len(),
            stage: my_data.stage.stage,
            stage_waited: me_waited,
            under_cards: [
                my_data.stage.under_cards[0].to_vec(),
                my_data.stage.under_cards[1].to_vec(),
                my_data.stage.under_cards[2].to_vec(),
            ],
            energy_zone: my_data.energy_zone.cards.to_vec(),
            active_energy_count: my_data.energy_zone.active_count() as usize,
            waitroom: my_data.waitroom.cards.to_vec(),
            success_zone: my_data.success_live_card_zone.cards.to_vec(),
            live_zone: me_live.to_vec(),
            main_deck_size: my_data.main_deck.cards.len(),
            main_deck_composition: my_main_deck_composition,
            energy_deck_size: my_data.energy_deck.cards.len(),
            is_first_attacker: my_data.is_first_attacker,
            features: me_features,
            stage_abilities: me_stage_abilities,
        };

        let opp_live = if opponent_performed {
            opp_data.live_card_zone.cards.to_vec()
        } else {
            Vec::new()
        };
        let mut opp_ability_cards = opp_data.stage.stage.to_vec();
        for under in &opp_data.stage.under_cards {
            opp_ability_cards.extend_from_slice(under);
        }
        opp_ability_cards.extend_from_slice(&opp_data.waitroom.cards);
        opp_ability_cards.extend_from_slice(&opp_live);
        opp_ability_cards.extend_from_slice(&opp_data.success_live_card_zone.cards);
        let opp_features = player_features(state, opp_data, &opp_live, &opp_ability_cards, false);
        let opp_stage_abilities = [
            card_ability_features(state, opp_data.stage.stage[0], false),
            card_ability_features(state, opp_data.stage.stage[1], false),
            card_ability_features(state, opp_data.stage.stage[2], false),
        ];
        let opp_waited = [
            is_waited(state, opp_data.stage.stage[0]),
            is_waited(state, opp_data.stage.stage[1]),
            is_waited(state, opp_data.stage.stage[2]),
        ];

        let opp = PlayerView {
            hand: Vec::new(),
            hand_size: opp_data.hand.cards.len(),
            stage: opp_data.stage.stage,
            stage_waited: opp_waited,
            under_cards: [
                opp_data.stage.under_cards[0].to_vec(),
                opp_data.stage.under_cards[1].to_vec(),
                opp_data.stage.under_cards[2].to_vec(),
            ],
            energy_zone: opp_data.energy_zone.cards.to_vec(),
            active_energy_count: opp_data.energy_zone.active_count() as usize,
            waitroom: opp_data.waitroom.cards.to_vec(),
            success_zone: opp_data.success_live_card_zone.cards.to_vec(),
            live_zone: opp_live,
            main_deck_size: opp_data.main_deck.cards.len(),
            main_deck_composition: Vec::new(),
            energy_deck_size: opp_data.energy_deck.cards.len(),
            is_first_attacker: opp_data.is_first_attacker,
            features: opp_features,
            stage_abilities: opp_stage_abilities,
        };

        let active_player = if state.active_player().id == state.player1.id {
            0
        } else {
            1
        };
        let pending_choice_owner = pending_choice_player_index(state);
        let pending_choice_for_viewer = pending_choice_owner == Some(perspective_player);
        let pending_choice_player = pending_choice_for_viewer.then_some(perspective_player);
        let (pending_choice_kind, pending_choice_option_count, pending_choice_allow_skip) =
            if pending_choice_for_viewer {
                pending_choice_features(state)
            } else {
                (None, 0, false)
            };
        let ability_queue_current = state
            .ability_queue
            .current_entry()
            .filter(|entry| {
                queue_owner_index(&entry.player_id) == Some(perspective_player)
                    && public_card_id(state, entry.card_id, perspective_player)
            });
        let ability_queue_current_card = ability_queue_current.and_then(|entry| entry.card_id);
        let ability_queue_current_ability = ability_queue_current
            .map(|entry| entry.ability_index.min(u8::MAX as usize) as u8);
        let ability_queue_current_trigger = ability_queue_current
            .map(|entry| ability_trigger_index(&entry.trigger_type));
        let resolution_zone = if resolution_visible_to(state, perspective_player) {
            state.resolution_zone.cards.to_vec()
        } else {
            Vec::new()
        };
        let mut public_math = math_features(state, perspective_player);
        public_math[31] = if pending_choice_for_viewer { 1.0 } else { 0.0 };

        Self {
            me,
            opp,
            current_phase: state.current_phase,
            current_turn_phase: state.current_turn_phase,
            turn_number: state.turn_number,
            game_result: state.game_result.clone(),
            active_player,
            can_current_player_act: state.can_player_act(i32::from(perspective_player)),
            pending_choice: state.has_pending_choice(),
            pending_choice_kind: pending_choice_kind.map(str::to_owned),
            pending_choice_option_count,
            pending_choice_allow_skip,
            pending_choice_player,
            pending_choice_for_viewer,
            mulligan_selected_count: state.mulligan_selected_indices.len().min(u8::MAX as usize) as u8,
            live_card_selected_count: state.live_card_selected_indices.len().min(u8::MAX as usize) as u8,
            ability_queue_len: state
                .ability_queue
                .iter()
                .filter(|entry| queue_owner_index(&entry.player_id) == Some(perspective_player))
                .count()
                .min(u8::MAX as usize) as u8,
            ability_queue_waiting: pending_choice_for_viewer
                && state.ability_queue.is_waiting_for_choice().is_some(),
            ability_queue_current_card,
            ability_queue_current_ability,
            ability_queue_current_trigger,
            math_features: public_math,
            resolution_zone,
        }
    }
}

impl Hash for PublicObservation {
    fn hash<H: Hasher>(&self, state: &mut H) {
        phase_u8(&self.current_phase).hash(state);
        self.current_turn_phase.hash(state);
        self.turn_number.hash(state);
        game_result_u8(&self.game_result).hash(state);
        self.active_player.hash(state);
        self.can_current_player_act.hash(state);
        self.pending_choice.hash(state);
        self.pending_choice_kind.hash(state);
        self.pending_choice_option_count.hash(state);
        self.pending_choice_allow_skip.hash(state);
        self.pending_choice_player.hash(state);
        self.pending_choice_for_viewer.hash(state);
        self.mulligan_selected_count.hash(state);
        self.live_card_selected_count.hash(state);
        self.ability_queue_len.hash(state);
        self.ability_queue_waiting.hash(state);
        self.ability_queue_current_card.hash(state);
        self.ability_queue_current_ability.hash(state);
        self.ability_queue_current_trigger.hash(state);
        for value in self.math_features {
            value.to_bits().hash(state);
        }
        self.resolution_zone.hash(state);
        hash_view(&self.me, state);
        hash_view(&self.opp, state);
    }
}

fn hash_view<H: Hasher>(p: &PlayerView, state: &mut H) {
    p.hand.hash(state);
    p.hand_size.hash(state);
    p.stage.hash(state);
    p.stage_waited.hash(state);
    for uc in &p.under_cards {
        uc.hash(state);
    }
    p.energy_zone.hash(state);
    p.active_energy_count.hash(state);
    p.waitroom.hash(state);
    p.success_zone.hash(state);
    p.live_zone.hash(state);
    p.main_deck_size.hash(state);
    p.main_deck_composition.hash(state);
    p.energy_deck_size.hash(state);
    p.is_first_attacker.hash(state);
    p.features.hash(state);
    p.stage_abilities.hash(state);
}

fn phase_u8(p: &Phase) -> u8 {
    match p {
        Phase::RockPaperScissors => 0,
        Phase::ChooseFirstAttacker => 1,
        Phase::MulliganFirstAttacker => 2,
        Phase::MulliganSecondAttacker => 3,
        Phase::Active => 4,
        Phase::Energy => 5,
        Phase::Draw => 6,
        Phase::Main => 7,
        Phase::LiveCardSetFirstAttacker => 8,
        Phase::LiveCardSetSecondAttacker => 9,
        Phase::FirstAttackerPerformance => 10,
        Phase::SecondAttackerPerformance => 11,
        Phase::LiveVictoryDetermination => 12,
    }
}

fn game_result_u8(r: &GameResult) -> u8 {
    match r {
        GameResult::Ongoing => 0,
        GameResult::FirstAttackerWins => 1,
        GameResult::SecondAttackerWins => 2,
        GameResult::Draw => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardDatabase;
    use crate::card_loader::CardLoader;
    use crate::game_setup::{Action, ActionParameters, ActionType};
    use crate::player::Player;
    use std::path::Path;
    use std::sync::Arc;

    fn test_state() -> (Arc<CardDatabase>, GameState) {
        let cards = CardLoader::load_cards_from_file(Path::new("../cards/cards.json")).unwrap();
        let db = Arc::new(CardDatabase::load_or_create(cards));
        let p1 = Player::new("p1".into(), "P1".into(), true);
        let p2 = Player::new("p2".into(), "P2".into(), false);
        (Arc::clone(&db), GameState::new(p1, p2, db))
    }

    #[test]
    fn opponent_hand_and_hidden_choice_identity_are_not_exposed() {
        let (db, mut gs) = test_state();
        let hidden = db.get_card_id("PL!SP-bp1-005-P").unwrap();
        gs.player2.hand.cards.push(hidden);
        let obs = PublicObservation::from_state(&gs, 0);
        assert!(obs.opp.hand.is_empty());
        assert_eq!(obs.opp.hand_size, 1);
        let action = Action {
            description: "choice".into(),
            description_ja: None,
            action_type: ActionType::ChoiceSelect,
            parameters: Some(ActionParameters {
                card_id: Some(hidden),
                card_index: Some(0),
                card_indices: Some(vec![0]),
                stage_area: None,
                stage_area_index: None,
                use_baton_touch: None,
                card_name: None,
                card_no: None,
                ability_index: None,
                source_ability: None,
                base_cost: None,
                final_cost: None,
                available_areas: None,
                double_baton_pairs: None,
                disabled: None,
            }),
            selected: None,
        };
        let encoded = crate::bot::encoding::ActionEncoding::from_action(&action, &obs);
        assert_eq!(encoded.target_card_id, -1);
        assert_ne!(encoded.flags & 8, 0);
    }

    #[test]
    fn own_deck_projection_is_orderless_and_opponent_choice_is_coarse() {
        let (db, mut gs) = test_state();
        let first = db.get_card_id("PL!SP-bp1-005-P").unwrap();
        let second = db.get_card_id("PL!SP-pb1-018-N").unwrap();
        gs.player1.main_deck.cards.extend([second, first]);
        let choice = Choice::SelectCard {
            zone: "Hand".into(),
            card_type: None,
            count: 1,
            description: "hidden".into(),
            description_en: None,
            description_ja: None,
            allow_skip: false,
            cost_limit: None,
            cost_limit_operator: None,
            cost_total: None,
            cost_total_operator: None,
            cost_values: None,
            group: None,
            characters: None,
            filtered_indices: None,
            is_select_action: false,
            heart_colors: Vec::new(),
            require_all_heart_colors: None,
            name_fragments: None,
            target_player_id: Some("p2".into()),
            blind: true,
            is_reveal: false,
            picker: Some("p2".into()),
            destination: None,
            discard_remaining: None,
        };
        gs.ability_queue.pause_for_choice(choice);
        let obs = PublicObservation::from_state(&gs, 0);
        assert_eq!(obs.me.main_deck_composition, vec![first.min(second), first.max(second)]);
        assert!(obs.pending_choice);
        assert!(!obs.pending_choice_for_viewer);
        assert!(obs.pending_choice_player.is_none());
        assert!(obs.pending_choice_kind.is_none());
    }
}
