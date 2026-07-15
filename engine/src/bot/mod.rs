mod determinization;
pub mod evaluation;
mod ismcts;
mod neural;
mod observation;

pub use ismcts::search_1ply;
pub use neural::ValueNetwork;
pub use observation::PublicObservation;

use crate::card::CardDatabase;
use crate::game_setup::{Action, ActionType};
use crate::game_state::GameState;
use std::sync::Arc;

use determinization::DeterminizationSampler;

pub struct BotConfig {
    pub iterations: u32,
    pub exploration_constant: f64,
    pub progressive_widening_k: f64,
    pub rollout_depth: u32,
    pub use_heuristic_rollout: bool,
}

impl Default for BotConfig {
    fn default() -> Self {
        Self {
            iterations: 30,
            exploration_constant: 1.4,
            progressive_widening_k: 5.0,
            rollout_depth: 9999,
            use_heuristic_rollout: true,
        }
    }
}

pub struct Bot {
    pub config: BotConfig,
    pub card_database: Arc<CardDatabase>,
    pub perspective_player: u8,
    pub sampler: DeterminizationSampler,
    pub network: ValueNetwork,
}

impl Bot {
    pub fn new(
        card_database: Arc<CardDatabase>,
        perspective_player: u8,
        our_deck: &[String],
        opp_deck: &[String],
    ) -> Self {
        let sampler = DeterminizationSampler::new(Arc::clone(&card_database), our_deck, opp_deck);
        let num_cards = card_database.cards.len();
        Self {
            config: BotConfig::default(),
            card_database,
            perspective_player,
            sampler,
            network: ValueNetwork::new(num_cards),
        }
    }

    pub fn choose_action(&self, state: &GameState) -> Action {
        let actions = crate::game_setup::generate_possible_actions(state);
        if actions.is_empty() {
            return Action {
                description: "pass".into(),
                action_type: ActionType::Pass,
                parameters: None,
            };
        }
        if actions.len() == 1 {
            return actions.into_iter().next().unwrap();
        }

        let obs = PublicObservation::from_state(state, self.perspective_player);
        let best_idx = search_1ply(&obs, &actions, &self.config, &self.sampler, &self.network);
        actions[best_idx].clone()
    }

    pub fn set_iterations(&mut self, n: u32) {
        self.config.iterations = n;
    }
}
