use crate::card::CardDatabase;
use crate::game_state::GameState;
use crate::player::Player;
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
    pub player1: PlayerDisplay,
    pub player2: PlayerDisplay,
    pub pending_choice: Option<serde_json::Value>,
    #[serde(default)]
    pub looked_cards: ZoneDisplay,
}

pub fn card_to_display(card_id: i16, card_db: &CardDatabase, orientation: Option<Orientation>, blade_modifier: i32) -> Option<CardDisplay> {
    card_db.get_card(card_id).map(|card| {
        let base_heart = card.base_heart.as_ref().map(|bh| {
            bh.hearts.iter().map(|(color, count)| {
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
                };
                (color_str.to_string(), *count)
            }).collect()
        });
        CardDisplay {
            card_no: card.card_no.clone(),
            name: card.name.clone(),
            card_type: format!("{:?}", card.card_type),
            orientation: orientation.map(|o| format!("{:?}", o)),
            base_heart,
            blade: card.blade,
            total_blade: ((card.blade as i32) + blade_modifier).max(0) as u32,
            id: card_id,
        }
    })
}

pub fn zone_to_display(card_ids: &[i16], card_db: &CardDatabase) -> ZoneDisplay {
    ZoneDisplay {
        cards: card_ids.iter().filter_map(|&id| card_to_display(id, card_db, None, 0)).collect(),
    }
}

pub fn stage_to_display(stage: &crate::zones::Stage, card_db: &CardDatabase, blade_modifiers: &std::collections::HashMap<i16, i32>) -> StageDisplay {
    let blade_mod = |cid: i16| blade_modifiers.get(&cid).copied().unwrap_or(0);
    StageDisplay {
        left_side: if stage.stage[0] != -1 { card_to_display(stage.stage[0], card_db, None, blade_mod(stage.stage[0])) } else { None },
        center: if stage.stage[1] != -1 { card_to_display(stage.stage[1], card_db, None, blade_mod(stage.stage[1])) } else { None },
        right_side: if stage.stage[2] != -1 { card_to_display(stage.stage[2], card_db, None, blade_mod(stage.stage[2])) } else { None },
        left_under: stage.under_cards[0].iter().filter_map(|&id| card_to_display(id, card_db, None, 0)).collect(),
        center_under: stage.under_cards[1].iter().filter_map(|&id| card_to_display(id, card_db, None, 0)).collect(),
        right_under: stage.under_cards[2].iter().filter_map(|&id| card_to_display(id, card_db, None, 0)).collect(),
    }
}

pub fn player_to_display(player: &Player, card_db: &CardDatabase, blade_modifiers: &std::collections::HashMap<i16, i32>) -> PlayerDisplay {
    let energy_cards: Vec<(i16, Option<Orientation>)> = player.energy_zone.cards.iter()
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
        cards: energy_cards.iter()
            .filter_map(|(card_id, orientation)| card_to_display(*card_id, card_db, *orientation, 0))
            .collect(),
    };

    let waitroom_display = zone_to_display(&player.waitroom.cards, card_db);

    PlayerDisplay {
        energy: energy_display,
        hand: zone_to_display(&player.hand.cards, card_db),
        stage: stage_to_display(&player.stage, card_db, blade_modifiers),
        live_zone: zone_to_display(&player.live_card_zone.cards, card_db),
        success_live_card_zone: zone_to_display(&player.success_live_card_zone.cards, card_db),
        waitroom: waitroom_display.clone(),
        discard: waitroom_display,
        main_deck_count: player.main_deck.len(),
        energy_deck_count: player.energy_deck.cards.len(),
        last_resolution_cards: player.last_resolution_cards.iter()
            .filter_map(|&id| card_to_display(id, card_db, None, 0)).collect(),
    }
}

pub fn game_state_to_display(game_state: &GameState) -> GameStateDisplay {
    // Collect looked-at cards: looked_at_cards + revealed_cards + pending_choice selection_cards
    let mut looked_ids: Vec<i16> = game_state.looked_at_cards.clone();
    looked_ids.extend(&game_state.revealed_cards);
    if let Some(ref pc) = game_state.pending_choice {
        if let Some(cards) = pc.get("selection_cards").and_then(|v| v.as_array()) {
            for val in cards {
                if let Some(id) = val.get("id").and_then(|v| v.as_i64()).or_else(|| val.as_i64()) {
                    looked_ids.push(id as i16);
                }
            }
        }
    }
    looked_ids.sort();
    looked_ids.dedup();

    GameStateDisplay {
        turn: game_state.turn_number,
        phase: format!("{:?}", game_state.current_phase),
        player1: player_to_display(&game_state.player1, &game_state.card_database, &game_state.mods.blade_modifiers),
        player2: player_to_display(&game_state.player2, &game_state.card_database, &game_state.mods.blade_modifiers),
        pending_choice: game_state.pending_choice.clone(),
        looked_cards: zone_to_display(&looked_ids, &game_state.card_database),
    }
}
