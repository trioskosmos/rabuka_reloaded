use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen;
use rabuka_engine::{
    core::game_state::{GameState},
    core::player::Player,
    core::card::CardDatabase,
    core::card_binary::load_cards_from_blob,
    game_setup::ActionType,
    turn::TurnEngine,
    rng::{seed},
};
use std::sync::Arc;
use std::collections::VecDeque;

// Regular Rust structs (serialized via serde_wasm_bindgen)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JsAction {
    pub action_type: String,
    pub card_id: Option<i16>,
    pub card_indices: Option<Vec<usize>>,
    pub stage_area: Option<String>,
    pub use_baton_touch: Option<bool>,
    pub ability_index: Option<usize>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JsActionResult {
    pub success: bool,
    pub error: Option<String>,
    pub frame_id: u64,
    pub state_delta: Option<JsStateDelta>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JsStateDelta {
    pub frame_id: u64,
    pub phase: Option<String>,
    pub active_player: Option<u8>,
    pub rps_winner: Option<u8>,
    pub player1_rps_choice: Option<u8>,
    pub player2_rps_choice: Option<u8>,
    pub pending_choice: Option<serde_json::Value>,
    pub legal_actions: Option<Vec<JsActionIndex>>,
    pub executed_action: Option<JsFrameAction>,
    pub rng_results: Option<Vec<JsRngResult>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JsFrameAction {
    pub action_type: String,
    pub player_id: u8,
    pub card_id: Option<i16>,
    pub card_indices: Option<Vec<usize>>,
    pub stage_area: Option<String>,
    pub use_baton_touch: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JsActionIndex {
    pub description: String,
    pub action_type: String,
    pub index: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JsRngResult {
    pub rng_type: String,
    pub result: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JsGameConfig {
    pub seed: u64,
    pub p1_deck: Vec<i16>,
    pub p2_deck: Vec<i16>,
    pub p1_energy: Vec<i16>,
    pub p2_energy: Vec<i16>,
}

#[wasm_bindgen]
pub struct WasmGameEngine {
    game_state: GameState,
    frame_counter: u64,
    card_database: Arc<CardDatabase>,
}

#[wasm_bindgen]
impl WasmGameEngine {
    #[wasm_bindgen(constructor)]
    pub fn new(config: JsValue) -> Result<WasmGameEngine, JsValue> {
        let config: JsGameConfig = serde_wasm_bindgen::from_value(config)?;
        
        // Load ALL cards from the embedded blob (for full card lookup during gameplay)
        let card_count = rabuka_engine::core::card_binary::blob_card_count();
        let all_indices: Vec<usize> = (0..card_count).collect();
        let card_database = Arc::new(load_cards_from_blob(&all_indices));
        
        let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
        let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
        
        player1.set_main_deck(config.p1_deck.into_iter().collect::<VecDeque<_>>());
        player1.set_energy_deck(config.p1_energy.into_iter().collect::<VecDeque<_>>());
        player2.set_main_deck(config.p2_deck.into_iter().collect::<VecDeque<_>>());
        player2.set_energy_deck(config.p2_energy.into_iter().collect::<VecDeque<_>>());
        
        seed(config.seed as u32);
        
        let game_state = GameState::new(player1, player2, card_database.clone());
        
        Ok(WasmGameEngine {
            game_state,
            frame_counter: 0,
            card_database,
        })
    }
    
    pub fn execute_action(&mut self, action: JsValue) -> Result<JsValue, JsValue> {
        let action: JsAction = serde_wasm_bindgen::from_value(action)?;
        
        let action_type = action.action_type.parse::<ActionType>()
            .map_err(|e| JsValue::from_str(&format!("Invalid action type: {}", e)))?;
        
        let stage_area = action.stage_area.as_ref()
            .and_then(|s| s.parse::<rabuka_engine::zones::MemberArea>().ok());
        
        let result = TurnEngine::execute_main_phase_action_with_ability_index(
            &mut self.game_state,
            &action_type,
            action.card_id,
            action.card_indices.clone(),
            stage_area,
            action.use_baton_touch,
            action.ability_index,
        );
        
        match result {
            Ok(_) => {
                self.frame_counter += 1;
                
                let frame_action = JsFrameAction {
                    action_type: action.action_type,
                    player_id: if self.game_state.active_player().id == "player1" { 0 } else { 1 },
                    card_id: action.card_id,
                    card_indices: action.card_indices,
                    stage_area: action.stage_area,
                    use_baton_touch: action.use_baton_touch.unwrap_or(false),
                };
                
                let state_delta = self.build_state_delta(Some(frame_action));
                
                let result = JsActionResult {
                    success: true,
                    error: None,
                    frame_id: self.frame_counter,
                    state_delta: Some(state_delta),
                };
                
                Ok(serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))?)
            }
            Err(e) => {
                let result = JsActionResult {
                    success: false,
                    error: Some(e),
                    frame_id: self.frame_counter,
                    state_delta: None,
                };
                Ok(serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))?)
            }
        }
    }
    
    pub fn get_state_delta(&self, _since_frame: u64) -> JsValue {
        let delta = self.build_state_delta(None);
        serde_wasm_bindgen::to_value(&delta).unwrap_or(JsValue::NULL)
    }
    
    pub fn get_legal_actions(&self) -> JsValue {
        let actions = rabuka_engine::game_setup::generate_possible_actions(&self.game_state);
        let js_actions: Vec<JsActionIndex> = actions.into_iter().enumerate().map(|(i, a)| JsActionIndex {
            description: a.description,
            action_type: a.action_type.to_string(),
            index: i,
        }).collect();
        serde_wasm_bindgen::to_value(&js_actions).unwrap_or(JsValue::NULL)
    }
    
    pub fn serialize(&self) -> Vec<u8> {
        rmp_serde::to_vec(&self.game_state).unwrap_or_default()
    }
    
    pub fn get_frame_counter(&self) -> u64 {
        self.frame_counter
    }
    
    pub fn get_phase(&self) -> String {
        format!("{:?}", self.game_state.current_phase)
    }
    
    pub fn get_game_result(&self) -> String {
        format!("{:?}", self.game_state.game_result)
    }
    
    fn build_state_delta(&self, executed_action: Option<JsFrameAction>) -> JsStateDelta {
        let active_player = if self.game_state.active_player().id == "player1" { 0 } else { 1 };
        
        JsStateDelta {
            frame_id: self.frame_counter,
            phase: Some(format!("{:?}", self.game_state.current_phase)),
            active_player: Some(active_player),
            rps_winner: self.game_state.rps_winner,
            player1_rps_choice: self.game_state.player1_rps_choice,
            player2_rps_choice: self.game_state.player2_rps_choice,
            pending_choice: self.game_state.get_pending_choice_json(),
            legal_actions: None,
            executed_action,
            rng_results: None,
        }
    }
}