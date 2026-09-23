use crate::bot::PublicObservation;
use crate::game_state::GameState;
use crate::rng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

fn default_producer() -> String {
    "flamegraph_replay".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Header {
    pub version: u32,
    pub projection: String,
    pub feature_schema: u32,
    pub execution: String,
    pub profiling: bool,
    #[serde(default = "default_producer")]
    pub producer: String,
    pub deck_text: String,
    pub games: usize,
    pub engine_seed: u32,
    pub policy_seed: u64,
    pub policy: String,
    pub build_identity: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub operation: String,
    pub actions: Vec<Value>,
    pub selected: Option<Value>,
    pub result: Option<String>,
    pub state: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GameTrace {
    pub game: usize,
    pub initial: Value,
    pub steps: Vec<Step>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Completion {
    pub complete: bool,
    pub games: usize,
    pub actions: usize,
}

pub fn observe(gs: &GameState) -> Result<Value, serde_json::Error> {
    let mut usage: Vec<_> = gs
        .turn_limited_abilities_used
        .iter()
        .map(|(k, v)| (*k, *v))
        .collect();
    usage.sort_unstable();
    Ok(json!({
        "players": [serde_json::to_value(&gs.player1)?, serde_json::to_value(&gs.player2)?],
        "phase": serde_json::to_value(gs.current_phase)?,
        "turn_phase": serde_json::to_value(gs.current_turn_phase)?,
        "turn": gs.turn_number,
        "result": serde_json::to_value(&gs.game_result)?,
        "ended": gs.game_ended,
        "rng": rng::checkpoint(),
        "queue": serde_json::to_value(&gs.ability_queue)?,
        "snapshots": serde_json::to_value(&gs.performance_snapshots)?,
        "resolution_zone": serde_json::to_value(&gs.resolution_zone)?,
        "usage": usage,
        "batch_movements": serde_json::to_value(&gs.batch_movements)?,
        "turn_movements": serde_json::to_value(&gs.turn_movements)?,
        "turn_area_movements": serde_json::to_value(&gs.turn_area_movements)?,
        "depth_first_cutoff": gs.depth_first_cutoff,
        "rps": [gs.player1_rps_choice, gs.player2_rps_choice, gs.rps_winner],
        "observations": [
            serde_json::to_value(PublicObservation::from_state(gs, 0))?,
            serde_json::to_value(PublicObservation::from_state(gs, 1))?,
        ],
    }))
}
