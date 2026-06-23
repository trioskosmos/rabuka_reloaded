use crate::ability::debug::AbDebug;
use crate::ability_queue::QueueState;
use crate::card::CardDatabase;
use crate::card::HeartColor;
use crate::game_state::GameState;
use crate::player::Player;
use crate::types::PerformanceSnapshot;
use crate::zones::Orientation;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn heart_color_index(color: &HeartColor) -> Option<usize> {
    Some(match color {
        HeartColor::Heart00 => 0,
        HeartColor::Heart01 => 1,
        HeartColor::Heart02 => 2,
        HeartColor::Heart03 => 3,
        HeartColor::Heart04 => 4,
        HeartColor::Heart05 => 5,
        HeartColor::Heart06 => 6,
        HeartColor::All => 7,
        _ => return None,
    })
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TempEffectDisplay {
    pub effect_type: String,
    pub duration: String,
    pub created_turn: u32,
    pub target_player_id: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ReplacementEffectDisplay {
    pub card_id: i16,
    pub player_id: String,
    pub original_event: String,
    pub is_choice_based: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AbilityQueueEntryDisplay {
    pub card_no: String,
    pub player_id: String,
    pub trigger_type: String,
    pub completed: bool,
    pub cost_paid: bool,
    pub effect_started: bool,
    pub choice_player_id: Option<String>,
    pub ability_text: String,
    pub card_id: Option<i16>,
    pub ability_index: usize,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DebutTriggerDisplay {
    pub ability_key: String,
    pub card_id: i16,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AbilityApplicationDisplay {
    pub source_card_id: i16,
    pub effect_type: String,
    pub target_card_id: i16,
    pub amount: i32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CardDisplay {
    pub card_no: String,
    pub name: String,
    #[serde(rename = "type")]
    pub card_type: String,
    pub orientation: Option<String>,
    pub base_heart: Option<HashMap<String, u32>>,
    pub blade: u32,
    pub total_blade: u32,
    pub id: i16,
    pub ability_text: Option<String>,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub bonus_blade: i32,
    #[serde(default)]
    pub bonus_hearts: Vec<i32>,
    #[serde(default)]
    pub bonus_score: i32,
    #[serde(default)]
    pub bonus_cost: i32,
    #[serde(default)]
    pub heart_transform: Option<String>,
    #[serde(default)]
    pub cost: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ZoneDisplay {
    pub cards: Vec<CardDisplay>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlayerDisplay {
    pub hand: ZoneDisplay,
    pub energy: ZoneDisplay,
    pub stage: StageDisplay,
    pub live_zone: ZoneDisplay,
    pub success_live_card_zone: ZoneDisplay,
    pub waitroom: ZoneDisplay,
    pub discard: ZoneDisplay,
    pub main_deck_count: usize,
    pub energy_deck_count: usize,
    #[serde(default)]
    pub last_resolution_cards: Vec<CardDisplay>,
    #[serde(default)]
    pub score_modifiers: std::collections::HashMap<i16, i32>,
    #[serde(default)]
    pub total_hearts: Vec<u32>,
    #[serde(default)]
    pub live_card_scores: std::collections::HashMap<String, u32>,
    #[serde(default)]
    pub gained_abilities: Vec<String>,
    #[serde(default)]
    pub active_restrictions: Vec<String>,
    #[serde(default)]
    pub need_heart_modifiers: std::collections::HashMap<String, Vec<i32>>,
    #[serde(default)]
    pub mulligan_selection: Option<Vec<usize>>,
    #[serde(default)]
    pub blade_buffs: Vec<i32>,
    #[serde(default)]
    pub heart_buffs: Vec<Vec<i32>>,
    #[serde(default)]
    pub cost_reduction: i32,
    #[serde(default)]
    pub prevent_baton_touch: i32,
    #[serde(default)]
    pub prevent_baton: i32,
    #[serde(default)]
    pub areas_locked_this_turn: Vec<String>,
    #[serde(default)]
    pub debut_count_this_turn: u32,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub is_first_attacker: bool,
    #[serde(default)]
    pub exclusion_zone: ZoneDisplay,
    #[serde(default)]
    pub energy_active_count: usize,
    #[serde(default)]
    pub stage_hearts: Option<HashMap<String, u32>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StageDisplay {
    pub left_side: Option<CardDisplay>,
    pub center: Option<CardDisplay>,
    pub right_side: Option<CardDisplay>,
    #[serde(default)]
    pub left_under: Vec<CardDisplay>,
    #[serde(default)]
    pub center_under: Vec<CardDisplay>,
    #[serde(default)]
    pub right_under: Vec<CardDisplay>,
}

#[derive(Serialize, Deserialize)]
pub struct GameStateDisplay {
    pub turn: u32,
    pub phase: String,
    #[serde(default)]
    pub active_player: String,
    pub player1: PlayerDisplay,
    pub player2: PlayerDisplay,
    pub pending_choice: Option<serde_json::Value>,
    #[serde(default)]
    pub looked_cards: ZoneDisplay,
    #[serde(default)]
    pub rule_log: Vec<String>,
    #[serde(default)]
    pub structured_log: Vec<crate::types::LogEntry>,
    #[serde(default)]
    pub performance_results: Option<std::collections::HashMap<String, PerformanceSnapshot>>,
    #[serde(default)]
    pub performance_history: Vec<PerformanceSnapshot>,
    #[serde(default)]
    pub game_over: bool,
    #[serde(default)]
    pub winner: Option<String>,
    #[serde(default)]
    pub waiting_for_opponent: bool,
    #[serde(default)]
    pub mode: String,
    // --- Internal tracking fields for game state modal ---
    #[serde(default)]
    pub current_turn_phase: String,
    #[serde(default)]
    pub game_result: String,
    #[serde(default)]
    pub is_first_turn: bool,
    #[serde(default)]
    pub turn_order_changed: bool,
    #[serde(default)]
    pub baton_touch_count: u32,
    #[serde(default)]
    pub baton_touch_zero_cost: bool,
    #[serde(default)]
    pub baton_touch_replaced_member_cost: Option<u32>,
    #[serde(default)]
    pub baton_touch_replaced_member_id: Option<i16>,
    #[serde(default)]
    pub baton_touch_arriving_card_id: Option<i16>,
    #[serde(default)]
    pub deck_refresh_pending: bool,
    #[serde(default)]
    pub loop_detected: bool,
    #[serde(default)]
    pub draw_state: bool,
    #[serde(default)]
    pub live_being_performed: bool,
    #[serde(default)]
    pub cards_moved_this_turn: Vec<i16>,
    #[serde(default)]
    pub cards_appeared_this_turn: Vec<i16>,
    #[serde(default)]
    pub areas_placed_this_turn: Vec<String>,
    #[serde(default)]
    pub last_area_move_card_id: Option<i16>,
    #[serde(default)]
    pub last_area_move_by_player: Option<String>,
    #[serde(default)]
    pub last_energy_placed_by_effect: bool,
    #[serde(default)]
    pub last_energy_placed_by_player: Option<String>,
    #[serde(default)]
    pub position_change_occurred_this_turn: bool,
    #[serde(default)]
    pub formation_change_occurred_this_turn: bool,
    #[serde(default)]
    pub opponent_live_success_this_turn: bool,
    #[serde(default)]
    pub opponent_live_no_excess_heart_this_turn: bool,
    #[serde(default)]
    pub self_no_excess_heart_this_turn: bool,
    #[serde(default)]
    pub opponent_live_surplus_count: u32,
    #[serde(default)]
    pub self_live_surplus_count: u32,
    #[serde(default)]
    pub live_success_triggered_this_turn: bool,
    #[serde(default)]
    pub live_surplus_ready_this_turn: bool,
    #[serde(default)]
    pub cheer_checks_required: u32,
    #[serde(default)]
    pub cheer_checks_done: u32,
    #[serde(default)]
    pub turn_limited_abilities_used: Vec<String>,
    #[serde(default)]
    pub auto_ability_trigger_counts: std::collections::HashMap<String, u32>,
    #[serde(default)]
    pub turn_limit_usage: std::collections::HashMap<String, u32>,
    #[serde(default)]
    pub non_stackable_effects: Vec<String>,
    #[serde(default)]
    pub prohibition_effects: Vec<String>,
    #[serde(default)]
    pub delayed_prohibition_effects: Vec<String>,
    #[serde(default)]
    pub cannot_activate_members: Vec<String>,
    #[serde(default)]
    pub constant_cannot_activate_members: Vec<String>,
    #[serde(default)]
    pub negated_abilities: Vec<i16>,
    #[serde(default)]
    pub temporary_effects: Vec<TempEffectDisplay>,
    #[serde(default)]
    pub replacement_effects: Vec<ReplacementEffectDisplay>,

    // === New comprehensive fields ===
    // Ability Queue
    #[serde(default)]
    pub ability_queue_state: String,
    #[serde(default)]
    pub ability_queue_current_index: usize,
    #[serde(default)]
    pub ability_queue_entries: Vec<AbilityQueueEntryDisplay>,

    // RPS
    #[serde(default)]
    pub rps_winner: Option<u8>,
    #[serde(default)]
    pub player1_rps_choice: Option<i32>,
    #[serde(default)]
    pub player2_rps_choice: Option<i32>,
    #[serde(default)]
    pub pending_rps_player_id: Option<i32>,

    // Card/Ability Runtime
    #[serde(default)]
    pub activating_card: Option<i16>,
    #[serde(default)]
    pub activating_ability_index: Option<usize>,
    #[serde(default)]
    pub just_completed_ability_key: Option<String>,
    #[serde(default)]
    pub turn1_abilities_played: Vec<String>,
    #[serde(default)]
    pub turn2_abilities_played: std::collections::HashMap<String, u32>,
    #[serde(default)]
    pub card_instance_mapping: std::collections::HashMap<String, u32>,
    #[serde(default)]
    pub card_instance_counter: u32,

    // Move Tracking
    #[serde(default)]
    pub recently_moved_cards: Vec<i16>,
    #[serde(default)]
    pub recently_moved_from_zone: Option<String>,
    #[serde(default)]
    pub last_vacated_stage_area: Option<String>,
    #[serde(default)]
    pub debut_ability_triggers: Vec<DebutTriggerDisplay>,

    // Live/Cheer
    #[serde(default)]
    pub live_cheer_count: u32,
    #[serde(default)]
    pub cheer_check_completed: bool,
    #[serde(default)]
    pub player1_cheer_blade_heart_count: u32,
    #[serde(default)]
    pub player2_cheer_blade_heart_count: u32,
    #[serde(default)]
    pub player1_cheer_revealed_cards: Vec<i16>,
    #[serde(default)]
    pub player2_cheer_revealed_cards: Vec<i16>,
    #[serde(default)]
    pub revealed_cards: Vec<i16>,
    #[serde(default)]
    pub heart_color_decision_phase: String,
    #[serde(default)]
    pub live_owned_hearts: std::collections::HashMap<String, Vec<[String; 2]>>,
    #[serde(default)]
    pub opponent_choice_declined: bool,
    #[serde(default)]
    pub pending_success_replacement_card_id: Option<i16>,
    #[serde(default)]
    pub pending_success_replacement_player_id: Option<String>,

    // Resolution/Misc
    #[serde(default)]
    pub resolution_zone_cards: Vec<i16>,
    #[serde(default)]
    pub revealed_cost_cards: Vec<i16>,
    #[serde(default)]
    pub ability_applications: Vec<AbilityApplicationDisplay>,
    #[serde(default)]
    pub effect_creation_counter: u32,
    #[serde(default)]
    pub last_state_change_wait_to_active_count: u32,

    // GameModifiers constant* breakdown
    #[serde(default)]
    pub constant_blade_bonuses: std::collections::HashMap<i16, i32>,
    #[serde(default)]
    pub constant_cost_bonuses: std::collections::HashMap<i16, i32>,
    #[serde(default)]
    pub constant_score_bonuses: std::collections::HashMap<i16, i32>,
    #[serde(default)]
    pub constant_heart_bonuses:
        std::collections::HashMap<i16, std::collections::HashMap<String, i32>>,
    #[serde(default)]
    pub constant_global_need_heart: Vec<[String; 3]>,
    #[serde(default)]
    pub constant_score_sources: Vec<[String; 3]>,
    #[serde(default)]
    pub blade_type_modifiers: std::collections::HashMap<i16, String>,
    #[serde(default)]
    pub heart_override: std::collections::HashMap<i16, [String; 2]>,
    #[serde(default)]
    pub delayed_cannot_active: std::collections::HashMap<i16, u32>,
    #[serde(default)]
    pub last_cost_discard_count: u32,
    #[serde(default)]
    pub last_cost_energy_count: u32,

    // Cheer/Blade heart tracking
    #[serde(default)]
    pub mulligan_selected_indices: Vec<usize>,
    #[serde(default)]
    pub live_success_total_score: Option<u32>,
}

pub fn card_to_display(
    card_id: i16,
    card_db: &CardDatabase,
    orientation: Option<Orientation>,
    blade_modifier: i32,
) -> Option<CardDisplay> {
    card_db.get_card(card_id).map(|card| {
        let base_heart = card.base_heart.as_ref().map(|bh| {
            bh.hearts
                .iter()
                .map(|(color, count)| {
                    let color_str = match color {
                        crate::card::HeartColor::Heart00 => "heart00",
                        crate::card::HeartColor::Heart01 => "heart01",
                        crate::card::HeartColor::Heart02 => "heart02",
                        crate::card::HeartColor::Heart03 => "heart03",
                        crate::card::HeartColor::Heart04 => "heart04",
                        crate::card::HeartColor::Heart05 => "heart05",
                        crate::card::HeartColor::Heart06 => "heart06",
                        crate::card::HeartColor::BAll => "b_all",
                        crate::card::HeartColor::Draw => "draw",
                        crate::card::HeartColor::Score => "score",
                        crate::card::HeartColor::All => "all",
                    };
                    (color_str.to_string(), *count)
                })
                .collect()
        });
        CardDisplay {
            card_no: card.card_no.clone(),
            name: card.name.clone(),
            card_type: format!("{:?}", card.card_type),
            orientation: orientation.map(|o| format!("{:?}", o)),
            base_heart,
            blade: card.blade,
            total_blade: if orientation == Some(Orientation::Wait) {
                0
            } else {
                ((card.blade as i32) + blade_modifier).max(0) as u32
            },
            id: card_id,
            ability_text: Some(card.ability.clone()),
            bonus_blade: blade_modifier,
            bonus_hearts: Vec::new(),
            bonus_score: 0,
            bonus_cost: 0,
            heart_transform: None,
            hidden: false,
            cost: card.cost,
        }
    })
}

pub fn card_to_display_full(
    card_id: i16,
    card_db: &CardDatabase,
    orientation: Option<Orientation>,
    blade_modifier: i32,
    score_modifier: i32,
    heart_modifiers: &std::collections::HashMap<crate::card::HeartColor, i32>,
    heart_transform: Option<crate::card::HeartColor>,
    cost_modifier: i32,
) -> Option<CardDisplay> {
    card_db.get_card(card_id).map(|card| {
        let base_heart = card.base_heart.as_ref().map(|bh| {
            bh.hearts
                .iter()
                .map(|(color, count)| {
                    let color_str = match color {
                        crate::card::HeartColor::Heart00 => "heart00",
                        crate::card::HeartColor::Heart01 => "heart01",
                        crate::card::HeartColor::Heart02 => "heart02",
                        crate::card::HeartColor::Heart03 => "heart03",
                        crate::card::HeartColor::Heart04 => "heart04",
                        crate::card::HeartColor::Heart05 => "heart05",
                        crate::card::HeartColor::Heart06 => "heart06",
                        crate::card::HeartColor::BAll => "b_all",
                        crate::card::HeartColor::Draw => "draw",
                        crate::card::HeartColor::Score => "score",
                        crate::card::HeartColor::All => "all",
                    };
                    (color_str.to_string(), *count)
                })
                .collect()
        });
        let mut bonus_hearts = vec![0i32; 8];
        for (color, &val) in heart_modifiers {
            let idx = match color {
                crate::card::HeartColor::Heart00 => 0,
                crate::card::HeartColor::Heart01 => 1,
                crate::card::HeartColor::Heart02 => 2,
                crate::card::HeartColor::Heart03 => 3,
                crate::card::HeartColor::Heart04 => 4,
                crate::card::HeartColor::Heart05 => 5,
                crate::card::HeartColor::Heart06 => 6,
                crate::card::HeartColor::All => 7,
                _ => continue,
            };
            bonus_hearts[idx] += val;
        }
        let transform_str = heart_transform.map(|hc| {
            let s = match hc {
                crate::card::HeartColor::Heart00 => "heart00",
                crate::card::HeartColor::Heart01 => "heart01",
                crate::card::HeartColor::Heart02 => "heart02",
                crate::card::HeartColor::Heart03 => "heart03",
                crate::card::HeartColor::Heart04 => "heart04",
                crate::card::HeartColor::Heart05 => "heart05",
                crate::card::HeartColor::Heart06 => "heart06",
                crate::card::HeartColor::All => "all",
                _ => "heart00",
            };
            s.to_string()
        });
        CardDisplay {
            card_no: card.card_no.clone(),
            name: card.name.clone(),
            card_type: format!("{:?}", card.card_type),
            orientation: orientation.map(|o| format!("{:?}", o)),
            base_heart,
            blade: card.blade,
            total_blade: if orientation == Some(Orientation::Wait) {
                0
            } else {
                ((card.blade as i32) + blade_modifier).max(0) as u32
            },
            id: card_id,
            ability_text: Some(card.ability.clone()),
            bonus_blade: blade_modifier,
            bonus_hearts,
            bonus_score: score_modifier,
            bonus_cost: cost_modifier,
            heart_transform: transform_str,
            hidden: false,
            cost: card.cost,
        }
    })
}

pub fn zone_to_display(card_ids: &[i16], card_db: &CardDatabase) -> ZoneDisplay {
    ZoneDisplay {
        cards: card_ids
            .iter()
            .filter_map(|&id| card_to_display(id, card_db, None, 0))
            .collect(),
    }
}

pub fn zone_to_display_full(
    card_ids: &[i16],
    card_db: &CardDatabase,
    blade_modifiers: &std::collections::HashMap<i16, i32>,
    score_modifiers: &std::collections::HashMap<i16, i32>,
    heart_modifiers: &std::collections::HashMap<
        i16,
        std::collections::HashMap<crate::card::HeartColor, i32>,
    >,
    heart_color_multiplier: &std::collections::HashMap<i16, crate::card::HeartColor>,
    cost_modifiers: &std::collections::HashMap<i16, i32>,
) -> ZoneDisplay {
    ZoneDisplay {
        cards: card_ids
            .iter()
            .filter_map(|&id| {
                card_to_display_full(
                    id,
                    card_db,
                    None,
                    blade_modifiers.get(&id).copied().unwrap_or(0),
                    score_modifiers.get(&id).copied().unwrap_or(0),
                    &heart_modifiers.get(&id).cloned().unwrap_or_default(),
                    heart_color_multiplier.get(&id).copied(),
                    cost_modifiers.get(&id).copied().unwrap_or(0),
                )
            })
            .collect(),
    }
}

pub fn stage_to_display(
    stage: &crate::zones::Stage,
    card_db: &CardDatabase,
    blade_modifiers: &std::collections::HashMap<i16, i32>,
    orientation_modifiers: &std::collections::HashMap<i16, String>,
    heart_modifiers: &std::collections::HashMap<
        i16,
        std::collections::HashMap<crate::card::HeartColor, i32>,
    >,
    score_modifiers: &std::collections::HashMap<i16, i32>,
    heart_color_multiplier: &std::collections::HashMap<i16, crate::card::HeartColor>,
    cost_modifiers: &std::collections::HashMap<i16, i32>,
) -> StageDisplay {
    let blade_mod = |cid: i16| blade_modifiers.get(&cid).copied().unwrap_or(0);
    let score_mod = |cid: i16| score_modifiers.get(&cid).copied().unwrap_or(0);
    let heart_mod = |cid: i16| heart_modifiers.get(&cid).cloned().unwrap_or_default();
    let heart_xform = |cid: i16| heart_color_multiplier.get(&cid).copied();
    let cost_mod = |cid: i16| cost_modifiers.get(&cid).copied().unwrap_or(0);
    let orientation = |cid: i16| {
        orientation_modifiers.get(&cid).map(|o| match o.as_str() {
            "wait" => Orientation::Wait,
            _ => Orientation::Active,
        })
    };
    StageDisplay {
        left_side: if stage.stage[0] != -1 {
            card_to_display_full(
                stage.stage[0],
                card_db,
                orientation(stage.stage[0]),
                blade_mod(stage.stage[0]),
                score_mod(stage.stage[0]),
                &heart_mod(stage.stage[0]),
                heart_xform(stage.stage[0]),
                cost_mod(stage.stage[0]),
            )
        } else {
            None
        },
        center: if stage.stage[1] != -1 {
            card_to_display_full(
                stage.stage[1],
                card_db,
                orientation(stage.stage[1]),
                blade_mod(stage.stage[1]),
                score_mod(stage.stage[1]),
                &heart_mod(stage.stage[1]),
                heart_xform(stage.stage[1]),
                cost_mod(stage.stage[1]),
            )
        } else {
            None
        },
        right_side: if stage.stage[2] != -1 {
            card_to_display_full(
                stage.stage[2],
                card_db,
                orientation(stage.stage[2]),
                blade_mod(stage.stage[2]),
                score_mod(stage.stage[2]),
                &heart_mod(stage.stage[2]),
                heart_xform(stage.stage[2]),
                cost_mod(stage.stage[2]),
            )
        } else {
            None
        },
        left_under: stage.under_cards[0]
            .iter()
            .filter_map(|&id| card_to_display(id, card_db, None, 0))
            .collect(),
        center_under: stage.under_cards[1]
            .iter()
            .filter_map(|&id| card_to_display(id, card_db, None, 0))
            .collect(),
        right_under: stage.under_cards[2]
            .iter()
            .filter_map(|&id| card_to_display(id, card_db, None, 0))
            .collect(),
    }
}

pub fn player_to_display(
    player: &Player,
    card_db: &CardDatabase,
    blade_modifiers: &std::collections::HashMap<i16, i32>,
    score_modifiers: &std::collections::HashMap<i16, i32>,
    heart_modifiers: &std::collections::HashMap<
        i16,
        std::collections::HashMap<crate::card::HeartColor, i32>,
    >,
    orientation_modifiers: &std::collections::HashMap<i16, String>,
    gained_abilities: &std::collections::HashMap<i16, Vec<String>>,
    need_heart_modifiers: &std::collections::HashMap<
        i16,
        std::collections::HashMap<crate::card::HeartColor, i32>,
    >,
    prohibition_effects: &[String],
    cannot_activate_members: &[String],
    mulligan_selection: Option<&[usize]>,
    heart_color_multiplier: &std::collections::HashMap<i16, crate::card::HeartColor>,
    cost_modifiers: &std::collections::HashMap<i16, i32>,
) -> PlayerDisplay {
    let energy_cards: Vec<(i16, Option<Orientation>)> = player
        .energy_zone
        .cards
        .iter()
        .enumerate()
        .map(|(i, &card_id)| {
            let orientation = if i < player.energy_zone.active_energy_count {
                Some(Orientation::Active)
            } else {
                Some(Orientation::Wait)
            };
            (card_id, orientation)
        })
        .collect();

    let energy_display = ZoneDisplay {
        cards: energy_cards
            .iter()
            .filter_map(|(card_id, orientation)| {
                card_to_display(*card_id, card_db, *orientation, 0)
            })
            .collect(),
    };

    let waitroom_display = zone_to_display(&player.waitroom.cards, card_db);

    // Calculate total hearts including modifiers (7 elements: heart00-heart06)
    let mut total_hearts = vec![0u32; 8];

    // Add base hearts from stage cards (accounting for heart_color_multiplier transforms)
    for &card_id in &player.stage.stage {
        if card_id == -1 {
            continue;
        }
        if let Some(card) = card_db.get_card(card_id) {
            if let Some(ref base_heart) = card.base_heart {
                if let Some(&override_color) = heart_color_multiplier.get(&card_id) {
                    // Heart transform: sum all base hearts into the override color
                    if let Some(idx) = heart_color_index(&override_color) {
                        let total: u32 = base_heart.hearts.values().sum();
                        total_hearts[idx] += total;
                    }
                } else {
                    for (color, count) in &base_heart.hearts {
                        if let Some(idx) = heart_color_index(color) {
                            total_hearts[idx] += count;
                        }
                    }
                }
            }
        }
    }

    // Add heart modifiers from stage cards
    for &card_id in &player.stage.stage {
        if card_id != -1 {
            if let Some(card_heart_modifiers) = heart_modifiers.get(&card_id) {
                for (color, modifier) in card_heart_modifiers {
                    if let Some(index) = heart_color_index(color) {
                        total_hearts[index] = (total_hearts[index] as i32 + modifier).max(0) as u32;
                    }
                }
            }
        }
    }

    // Compute live_card_scores: card_no -> total score
    let mut live_card_scores = std::collections::HashMap::new();
    for &cid in &player.live_card_zone.cards {
        if let Some(card) = card_db.get_card(cid) {
            let base = card.score.unwrap_or(0);
            let bonus = score_modifiers.get(&cid).copied().unwrap_or(0);
            live_card_scores.insert(card.card_no.clone(), (base as i32 + bonus).max(0) as u32);
        }
    }
    for &cid in &player.success_live_card_zone.cards {
        if let Some(card) = card_db.get_card(cid) {
            let base = card.score.unwrap_or(0);
            let bonus = score_modifiers.get(&cid).copied().unwrap_or(0);
            live_card_scores.insert(card.card_no.clone(), (base as i32 + bonus).max(0) as u32);
        }
    }

    // Collect gained abilities for this player's cards
    let player_card_ids: std::collections::HashSet<i16> = player
        .stage
        .stage
        .iter()
        .chain(&player.hand.cards)
        .chain(&player.live_card_zone.cards)
        .chain(&player.success_live_card_zone.cards)
        .copied()
        .filter(|&id| id != -1)
        .collect();
    let mut my_gained: Vec<String> = Vec::new();
    for (cid, abilities) in gained_abilities {
        if player_card_ids.contains(cid) {
            for a in abilities {
                my_gained.push(format!("Card#{}: {}", cid, a));
            }
        }
    }

    // Collect need_heart_modifiers for live cards (card_no -> [h00..h06] modifiers)
    let mut nh_mods = std::collections::HashMap::new();
    for (&cid, colors) in need_heart_modifiers {
        if player.live_card_zone.cards.contains(&cid)
            || player.success_live_card_zone.cards.contains(&cid)
        {
            if let Some(card) = card_db.get_card(cid) {
                let mut arr = vec![0i32; 8];
                for (color, &val) in colors {
                    let idx = match color {
                        crate::card::HeartColor::Heart00 => 0,
                        crate::card::HeartColor::Heart01 => 1,
                        crate::card::HeartColor::Heart02 => 2,
                        crate::card::HeartColor::Heart03 => 3,
                        crate::card::HeartColor::Heart04 => 4,
                        crate::card::HeartColor::Heart05 => 5,
                        crate::card::HeartColor::Heart06 => 6,
                        _ => continue,
                    };
                    arr[idx] = val;
                }
                nh_mods.insert(card.card_no.clone(), arr);
            }
        }
    }

    // Collect active restrictions for this player
    let mut restrictions: Vec<String> = Vec::new();
    for pe in prohibition_effects.iter() {
        restrictions.push(pe.clone());
    }
    if cannot_activate_members
        .iter()
        .any(|t| t == "self" || t == &player.id)
    {
        restrictions.push("cannot_activate_members".to_string());
    }

    let stage_hearts_display = player.stage_hearts.as_ref().map(|sh| {
        sh.hearts
            .iter()
            .map(|(color, count)| {
                let color_str = match color {
                    crate::card::HeartColor::Heart00 => "heart00",
                    crate::card::HeartColor::Heart01 => "heart01",
                    crate::card::HeartColor::Heart02 => "heart02",
                    crate::card::HeartColor::Heart03 => "heart03",
                    crate::card::HeartColor::Heart04 => "heart04",
                    crate::card::HeartColor::Heart05 => "heart05",
                    crate::card::HeartColor::Heart06 => "heart06",
                    crate::card::HeartColor::BAll => "b_all",
                    crate::card::HeartColor::Draw => "draw",
                    crate::card::HeartColor::Score => "score",
                    crate::card::HeartColor::All => "all",
                };
                (color_str.to_string(), *count)
            })
            .collect()
    });

    PlayerDisplay {
        energy: energy_display,
        hand: zone_to_display_full(
            &player.hand.cards,
            card_db,
            blade_modifiers,
            score_modifiers,
            heart_modifiers,
            heart_color_multiplier,
            cost_modifiers,
        ),
        stage: stage_to_display(
            &player.stage,
            card_db,
            blade_modifiers,
            orientation_modifiers,
            heart_modifiers,
            score_modifiers,
            heart_color_multiplier,
            cost_modifiers,
        ),
        live_zone: zone_to_display_full(
            &player.live_card_zone.cards,
            card_db,
            blade_modifiers,
            score_modifiers,
            heart_modifiers,
            heart_color_multiplier,
            cost_modifiers,
        ),
        success_live_card_zone: zone_to_display_full(
            &player.success_live_card_zone.cards,
            card_db,
            blade_modifiers,
            score_modifiers,
            heart_modifiers,
            heart_color_multiplier,
            cost_modifiers,
        ),
        waitroom: waitroom_display.clone(),
        discard: waitroom_display,
        main_deck_count: player.main_deck.len(),
        energy_deck_count: player.energy_deck.cards.len(),
        last_resolution_cards: player
            .last_resolution_cards
            .iter()
            .filter_map(|&id| card_to_display(id, card_db, None, 0))
            .collect(),
        score_modifiers: score_modifiers.clone(),
        total_hearts,
        live_card_scores,
        gained_abilities: my_gained,
        active_restrictions: restrictions.clone(),
        need_heart_modifiers: nh_mods,
        mulligan_selection: mulligan_selection.map(|v| v.to_vec()),
        // Derive display fields from existing modifier data
        blade_buffs: player
            .stage
            .stage
            .iter()
            .map(|&cid| {
                if cid != -1 {
                    *blade_modifiers.get(&cid).unwrap_or(&0)
                } else {
                    0
                }
            })
            .collect(),
        heart_buffs: player
            .stage
            .stage
            .iter()
            .map(|&cid| {
                if cid == -1 {
                    return vec![0i32; 6];
                }
                let mut arr = vec![0i32; 6];
                if let Some(card_hm) = heart_modifiers.get(&cid) {
                    for (color, modifier) in card_hm {
                        let idx = match color {
                            crate::card::HeartColor::Heart01 => 0,
                            crate::card::HeartColor::Heart02 => 1,
                            crate::card::HeartColor::Heart03 => 2,
                            crate::card::HeartColor::Heart04 => 3,
                            crate::card::HeartColor::Heart05 => 4,
                            crate::card::HeartColor::Heart06 => 5,
                            _ => continue,
                        };
                        arr[idx] = *modifier;
                    }
                }
                arr
            })
            .collect(),
        cost_reduction: 0,
        prevent_baton_touch: if restrictions
            .iter()
            .any(|r| r.contains("cannot_baton") || r.contains("prevent_baton"))
        {
            1
        } else {
            0
        },
        prevent_baton: if restrictions
            .iter()
            .any(|r| r.contains("cannot_baton") || r.contains("prevent_baton"))
        {
            1
        } else {
            0
        },
        areas_locked_this_turn: player
            .areas_locked_this_turn
            .iter()
            .map(|a| format!("{:?}", a))
            .collect(),
        debut_count_this_turn: player.debut_count_this_turn,
        id: player.id.clone(),
        name: player.name.clone(),
        is_first_attacker: player.is_first_attacker,
        exclusion_zone: zone_to_display(&player.exclusion_zone.cards, card_db),
        energy_active_count: player.energy_zone.active_energy_count,
        stage_hearts: stage_hearts_display,
    }
}

pub fn game_state_to_display(game_state: &GameState) -> GameStateDisplay {
    // Collect publicly visible revealed cards + pending_choice selection cards
    let mut looked_ids: Vec<i16> = game_state.revealed_cards.clone();
    if let Some(ref pc) = game_state.get_pending_choice_json() {
        if let Some(cards) = pc.get("selection_cards").and_then(|v| v.as_array()) {
            for val in cards {
                if let Some(id) = val
                    .get("id")
                    .and_then(|v| v.as_i64())
                    .or_else(|| val.as_i64())
                {
                    looked_ids.push(id as i16);
                }
            }
        }
    }
    looked_ids.sort();
    looked_ids.dedup();

    // Create a mutable copy of rule_log to add ability debug logs
    let mut rule_log = game_state.rule_log.clone();
    AbDebug::flush_to_rule_log(&mut rule_log);
    // Cap rule_log to prevent unbounded growth
    if rule_log.len() > 500 {
        rule_log.drain(0..rule_log.len() - 500);
    }

    // Structured log for rich UI rendering
    let mut structured_log = game_state.structured_log.clone();
    AbDebug::flush_to_structured_log(&mut structured_log, game_state.turn_number);
    if structured_log.len() > 500 {
        structured_log.drain(0..structured_log.len() - 500);
    }

    // Build performance results (grouped by player_id)
    let perf_history = game_state.performance_snapshots.clone();
    let mut perf_results: Option<std::collections::HashMap<String, PerformanceSnapshot>> = None;
    if !perf_history.is_empty() {
        let mut map = std::collections::HashMap::new();
        for snap in &perf_history {
            map.insert(snap.player_id.clone(), snap.clone());
        }
        perf_results = Some(map);
    }

    let mulligan_player_id = match game_state.current_phase {
        crate::game_state::Phase::MulliganFirstAttacker => {
            Some(game_state.first_attacker().id.clone())
        }
        crate::game_state::Phase::MulliganSecondAttacker => {
            Some(if game_state.first_attacker().id == game_state.player1.id {
                game_state.player2.id.clone()
            } else {
                game_state.player1.id.clone()
            })
        }
        _ => None,
    };
    let p1_mulligan = mulligan_player_id
        .as_ref()
        .is_some_and(|id| *id == game_state.player1.id)
        .then_some(game_state.mulligan_selected_indices.as_slice());
    let p2_mulligan = mulligan_player_id
        .as_ref()
        .is_some_and(|id| *id == game_state.player2.id)
        .then_some(game_state.mulligan_selected_indices.as_slice());

    let blade_flat: std::collections::HashMap<i16, i32> = game_state
        .mods
        .blade_modifiers
        .iter()
        .map(|(&k, v)| (k, v.total()))
        .collect();
    let score_flat: std::collections::HashMap<i16, i32> = game_state
        .mods
        .score_modifiers
        .iter()
        .map(|(&k, v)| (k, v.total()))
        .collect();
    let heart_flat: std::collections::HashMap<
        i16,
        std::collections::HashMap<crate::card::HeartColor, i32>,
    > = game_state
        .mods
        .heart_modifiers
        .iter()
        .map(|(&k, colors)| {
            let flat: std::collections::HashMap<crate::card::HeartColor, i32> =
                colors.iter().map(|(&c, e)| (c, e.total())).collect();
            (k, flat)
        })
        .collect();
    let need_heart_flat: std::collections::HashMap<
        i16,
        std::collections::HashMap<crate::card::HeartColor, i32>,
    > = game_state
        .mods
        .need_heart_modifiers
        .iter()
        .map(|(&k, colors)| {
            let flat: std::collections::HashMap<crate::card::HeartColor, i32> =
                colors.iter().map(|(&c, e)| (c, e.total())).collect();
            (k, flat)
        })
        .collect();

    let blade_flat2 = blade_flat.clone();
    let score_flat2 = score_flat.clone();
    let cost_flat: std::collections::HashMap<i16, i32> = game_state
        .mods
        .cost_modifiers
        .iter()
        .map(|(&k, v)| (k, v.total()))
        .collect();
    let cost_flat2 = cost_flat.clone();
    let heart_flat2 = heart_flat.clone();
    let need_heart_flat2 = need_heart_flat.clone();

    let temp_effects: Vec<TempEffectDisplay> = game_state
        .temporary_effects
        .iter()
        .map(|te| TempEffectDisplay {
            effect_type: te.effect_type.clone(),
            duration: format!("{:?}", te.duration),
            created_turn: te.created_turn,
            target_player_id: te.target_player_id.clone(),
            description: te.description.clone(),
        })
        .collect();

    let repl_effects: Vec<ReplacementEffectDisplay> = game_state
        .replacement_effects
        .iter()
        .map(|re| ReplacementEffectDisplay {
            card_id: re.card_id,
            player_id: re.player_id.clone(),
            original_event: re.original_event.clone(),
            is_choice_based: re.is_choice_based,
        })
        .collect();

    let turn_phase_str = format!("{:?}", game_state.current_turn_phase);
    let game_result_str = format!("{:?}", game_state.game_result);

    // Ability queue
    let queue_entries: Vec<AbilityQueueEntryDisplay> = game_state
        .ability_queue
        .iter()
        .map(|entry| {
            let trigger_str = format!("{:?}", entry.trigger_type);
            AbilityQueueEntryDisplay {
                card_no: entry.card_no.clone(),
                player_id: entry.player_id.clone(),
                trigger_type: trigger_str,
                completed: entry.completed,
                cost_paid: entry.cost_paid,
                effect_started: entry.effect_started,
                choice_player_id: entry.choice_player_id.clone(),
                ability_text: entry.ability.full_text.clone(),
                card_id: entry.card_id,
                ability_index: entry.ability_index,
            }
        })
        .collect();
    let queue_state_str = format!("{:?}", game_state.ability_queue.get_state());
    let queue_current_idx = match game_state.ability_queue.get_state() {
        QueueState::Idle => 0,
        QueueState::WaitingForAutoAbilityChoice { .. } => 0,
        QueueState::PayingCost { entry_index } => *entry_index,
        QueueState::WaitingForChoice { entry_index, .. } => *entry_index,
        QueueState::ExecutingEffect { entry_index } => *entry_index,
        QueueState::Completed { entry_index } => *entry_index,
    };

    // Debut triggers
    let debut_triggers: Vec<DebutTriggerDisplay> = game_state
        .debut_ability_triggers
        .iter()
        .map(|(key, cid)| DebutTriggerDisplay {
            ability_key: key.clone(),
            card_id: *cid,
        })
        .collect();

    // Ability applications
    let ability_apps: Vec<AbilityApplicationDisplay> = game_state
        .ability_applications
        .iter()
        .map(|app| AbilityApplicationDisplay {
            source_card_id: app.source_card_id,
            effect_type: app.effect_type.clone(),
            target_card_id: app.target_card_id,
            amount: app.amount,
        })
        .collect();

    // Live owned hearts: HashMap<String, Vec<(String, u32)>> -> HashMap<String, Vec<[String; 2]>>
    let live_owned: HashMap<String, Vec<[String; 2]>> = game_state
        .live_owned_hearts
        .iter()
        .map(|(pid, pairs)| {
            let converted: Vec<[String; 2]> = pairs
                .iter()
                .map(|(color, count)| [color.clone(), count.to_string()])
                .collect();
            (pid.clone(), converted)
        })
        .collect();

    // Constant heart bonuses: HashMap<i16, HashMap<String, i32>>
    let const_heart: HashMap<i16, HashMap<String, i32>> = game_state
        .mods
        .constant_heart_bonuses
        .iter()
        .map(|(cid, map)| (*cid, map.clone()))
        .collect();

    // Delayed cannot active: HashMap<i16, u32>
    let delayed_cannot: HashMap<i16, u32> = game_state.mods.delayed_cannot_active.clone();

    // Last vacated stage area
    let last_vacated = game_state.last_vacated_stage_area.map(|idx| match idx {
        0 => "LeftSide".to_string(),
        1 => "Center".to_string(),
        2 => "RightSide".to_string(),
        _ => format!("Slot{}", idx),
    });

    // Mulligan indices
    let mulligan_indices: Vec<usize> = game_state.mulligan_selected_indices.clone();

    GameStateDisplay {
        turn: game_state.turn_number,
        phase: format!("{:?}", game_state.current_phase),
        active_player: game_state.active_player().id.clone(),
        player1: player_to_display(
            &game_state.player1,
            &game_state.card_database,
            &blade_flat,
            &score_flat,
            &heart_flat,
            &game_state.mods.orientation_modifiers,
            &game_state.gained_abilities,
            &need_heart_flat,
            &game_state.prohibition_effects,
            &game_state.cannot_activate_members,
            p1_mulligan,
            &game_state.mods.heart_color_multiplier,
            &cost_flat,
        ),
        player2: player_to_display(
            &game_state.player2,
            &game_state.card_database,
            &blade_flat2,
            &score_flat2,
            &heart_flat2,
            &game_state.mods.orientation_modifiers,
            &game_state.gained_abilities,
            &need_heart_flat2,
            &game_state.prohibition_effects,
            &game_state.cannot_activate_members,
            p2_mulligan,
            &game_state.mods.heart_color_multiplier,
            &cost_flat2,
        ),
        pending_choice: game_state.get_pending_choice_json(),
        looked_cards: zone_to_display(&looked_ids, &game_state.card_database),
        rule_log,
        structured_log,
        performance_results: perf_results,
        performance_history: perf_history,
        game_over: game_state.game_ended,
        waiting_for_opponent: false,
        mode: String::new(),
        winner: match game_state.game_result {
            crate::types::GameResult::FirstAttackerWins => {
                Some(game_state.first_attacker().id.clone())
            }
            crate::types::GameResult::SecondAttackerWins => {
                Some(if game_state.player1.is_first_attacker {
                    game_state.player2.id.clone()
                } else {
                    game_state.player1.id.clone()
                })
            }
            _ => None,
        },
        current_turn_phase: turn_phase_str,
        game_result: game_result_str,
        is_first_turn: game_state.is_first_turn,
        turn_order_changed: game_state.turn_order_changed,
        baton_touch_count: game_state.baton_touch_count,
        baton_touch_zero_cost: game_state.baton_touch_zero_cost,
        baton_touch_replaced_member_cost: game_state.baton_touch_replaced_member_cost,
        baton_touch_replaced_member_id: game_state.baton_touch_replaced_member_id,
        baton_touch_arriving_card_id: game_state.baton_touch_arriving_card_id,
        deck_refresh_pending: game_state.deck_refresh_pending,
        loop_detected: game_state.loop_detected,
        draw_state: game_state.draw_state,
        live_being_performed: game_state.live_being_performed,
        cards_moved_this_turn: game_state.cards_moved_this_turn.iter().copied().collect(),
        cards_appeared_this_turn: game_state
            .cards_appeared_this_turn
            .iter()
            .copied()
            .collect(),
        areas_placed_this_turn: game_state.areas_placed_this_turn.iter().cloned().collect(),
        last_area_move_card_id: game_state.last_area_move_card_id,
        last_area_move_by_player: game_state.last_area_move_by_player.clone(),
        last_energy_placed_by_effect: game_state.last_energy_placed_by_effect,
        last_energy_placed_by_player: game_state.last_energy_placed_by_player.clone(),
        position_change_occurred_this_turn: game_state.position_change_occurred_this_turn,
        formation_change_occurred_this_turn: game_state.formation_change_occurred_this_turn,
        opponent_live_success_this_turn: game_state.opponent_live_success_this_turn,
        opponent_live_no_excess_heart_this_turn: game_state.opponent_live_no_excess_heart_this_turn,
        self_no_excess_heart_this_turn: game_state.self_no_excess_heart_this_turn,
        opponent_live_surplus_count: game_state.opponent_live_surplus_count,
        self_live_surplus_count: game_state.self_live_surplus_count,
        live_success_triggered_this_turn: game_state.live_success_triggered_this_turn,
        live_surplus_ready_this_turn: game_state.live_surplus_ready_this_turn,
        cheer_checks_required: game_state.cheer_checks_required,
        cheer_checks_done: game_state.cheer_checks_done,
        turn_limited_abilities_used: game_state
            .turn_limited_abilities_used
            .iter()
            .cloned()
            .collect(),
        auto_ability_trigger_counts: game_state.auto_ability_trigger_counts.clone(),
        turn_limit_usage: game_state.turn_limit_usage.clone(),
        non_stackable_effects: game_state.non_stackable_effects.iter().cloned().collect(),
        prohibition_effects: game_state.prohibition_effects.clone(),
        delayed_prohibition_effects: game_state.delayed_prohibition_effects.clone(),
        cannot_activate_members: game_state.cannot_activate_members.clone(),
        constant_cannot_activate_members: game_state.constant_cannot_activate_members.clone(),
        negated_abilities: game_state.negated_abilities.iter().copied().collect(),
        temporary_effects: temp_effects,
        replacement_effects: repl_effects,
        ability_queue_state: queue_state_str,
        ability_queue_current_index: queue_current_idx,
        ability_queue_entries: queue_entries,
        rps_winner: game_state.rps_winner,
        player1_rps_choice: game_state.player1_rps_choice,
        player2_rps_choice: game_state.player2_rps_choice,
        pending_rps_player_id: game_state.pending_rps_player_id,
        activating_card: game_state.activating_card,
        activating_ability_index: game_state.activating_ability_index,
        just_completed_ability_key: game_state.just_completed_ability_key.clone(),
        turn1_abilities_played: game_state.turn1_abilities_played.iter().cloned().collect(),
        turn2_abilities_played: game_state.turn2_abilities_played.clone(),
        card_instance_mapping: game_state
            .card_instance_mapping
            .iter()
            .map(|(k, v)| (k.to_string(), *v))
            .collect(),
        card_instance_counter: game_state.card_instance_counter,
        recently_moved_cards: game_state.recently_moved_cards.clone().unwrap_or_default(),
        recently_moved_from_zone: game_state.recently_moved_from_zone.clone(),
        last_vacated_stage_area: last_vacated,
        debut_ability_triggers: debut_triggers,
        live_cheer_count: game_state.live_cheer_count,
        cheer_check_completed: game_state.cheer_check_completed,
        player1_cheer_blade_heart_count: game_state.player1_cheer_blade_heart_count,
        player2_cheer_blade_heart_count: game_state.player2_cheer_blade_heart_count,
        player1_cheer_revealed_cards: game_state.player1_cheer_revealed_cards.clone(),
        player2_cheer_revealed_cards: game_state.player2_cheer_revealed_cards.clone(),
        revealed_cards: game_state.revealed_cards.clone(),
        heart_color_decision_phase: game_state.heart_color_decision_phase.clone(),
        live_owned_hearts: live_owned,
        opponent_choice_declined: game_state.opponent_choice_declined,
        pending_success_replacement_card_id: game_state.pending_success_replacement_card_id,
        pending_success_replacement_player_id: game_state
            .pending_success_replacement_player_id
            .clone(),
        resolution_zone_cards: game_state.resolution_zone.cards.iter().copied().collect(),
        revealed_cost_cards: game_state.revealed_cost_cards.clone(),
        ability_applications: ability_apps,
        effect_creation_counter: game_state.effect_creation_counter,
        last_state_change_wait_to_active_count: game_state.last_state_change_wait_to_active_count,
        constant_blade_bonuses: game_state.mods.constant_blade_bonuses.clone(),
        constant_cost_bonuses: game_state.mods.constant_cost_bonuses.clone(),
        constant_score_bonuses: game_state.mods.constant_score_bonuses.clone(),
        constant_heart_bonuses: const_heart,
        constant_global_need_heart: game_state
            .mods
            .constant_global_need_heart
            .iter()
            .map(|(cid, s, v)| [cid.to_string(), s.clone(), v.to_string()])
            .collect(),
        constant_score_sources: game_state
            .mods
            .constant_score_sources
            .iter()
            .map(|(cid, s, v)| [cid.to_string(), s.clone(), v.to_string()])
            .collect(),
        blade_type_modifiers: game_state
            .mods
            .blade_type_modifiers
            .iter()
            .map(|(k, v)| (*k, format!("{:?}", v)))
            .collect(),
        heart_override: game_state
            .mods
            .heart_override
            .iter()
            .map(|(k, (hc, v))| (*k, [format!("{:?}", hc), v.to_string()]))
            .collect(),
        delayed_cannot_active: delayed_cannot,
        last_cost_discard_count: game_state.mods.last_cost_discard_count,
        last_cost_energy_count: game_state.mods.last_cost_energy_count,
        mulligan_selected_indices: mulligan_indices,
        live_success_total_score: None,
    }
}
