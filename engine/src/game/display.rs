use crate::ability::debug::AbDebug;
use crate::card::CardDatabase;
use crate::game_state::GameState;
use crate::player::Player;
use crate::types::PerformanceSnapshot;
use crate::zones::Orientation;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

    // Add base hearts from stage cards
    for &card_id in &player.stage.stage {
        if card_id != -1 {
            if let Some(card) = card_db.get_card(card_id) {
                if let Some(ref base_heart) = card.base_heart {
                    for (color, count) in &base_heart.hearts {
                        let index = match color {
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
                        total_hearts[index] += count;
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
                    let index = match color {
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
                    total_hearts[index] = (total_hearts[index] as i32 + modifier).max(0) as u32;
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

    PlayerDisplay {
        energy: energy_display,
        hand: zone_to_display(&player.hand.cards, card_db),
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
        live_zone: zone_to_display(&player.live_card_zone.cards, card_db),
        success_live_card_zone: zone_to_display(&player.success_live_card_zone.cards, card_db),
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
    }
}

pub fn game_state_to_display(game_state: &GameState) -> GameStateDisplay {
    // Collect looked-at cards: looked_at_cards + revealed_cards + pending_choice selection cards
    let mut looked_ids: Vec<i16> = game_state.looked_at_cards.clone();
    looked_ids.extend(&game_state.revealed_cards);
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
    }
}
