mod determinization;
pub mod encoding;
pub mod evaluation;
pub mod ismcts;
pub mod neural;
pub mod observation;
pub mod strategy;
pub mod strategy_v2;
pub mod strategy_v3;
pub mod strategy_v4;
pub mod strategy_v5;
pub mod weights;

pub use ismcts::search;
pub use ismcts::search_1ply;
pub use neural::PolicyNet;
pub use observation::PublicObservation;
pub use strategy::{choose_action_heuristic, choose_live_set_action, evaluate_state, StrategyWeights};
pub use strategy_v2::{
    choose_action_heuristic_v2, choose_live_set_action_v2, choose_mulligan_action_v2, V2Policy,
};
pub use strategy_v3::{
    analyze_hand, choose_action_heuristic_v3, choose_live_set_action_v3,
    choose_mulligan_action_v3, evaluate_state_v3, V3Plan,
};
pub use strategy_v4::{choose_action_v4, choose_live_set_v4, choose_mulligan_v4};
pub use strategy_v5::{choose_action_v5, choose_live_set_v5, choose_mulligan_v5};

use crate::card::CardDatabase;
use crate::game_setup::{Action, ActionType};
use crate::game_state::GameState;
use crate::Arc;

use determinization::DeterminizationSampler;

pub struct BotConfig {
    pub iterations: u32,
    pub exploration_constant: f64,
    pub progressive_widening_k: f64,
    pub rollout_depth: u32,
    pub use_heuristic_rollout: bool,
    /// Tournament open-lists mode: sample the opponent's hidden cards from
    /// their actual deck list. Default false — the bot must not use
    /// information a fair player would not have.
    pub open_decklists: bool,
}

impl Default for BotConfig {
    fn default() -> Self {
        Self {
            iterations: 10000,
            exploration_constant: 1.4,
            progressive_widening_k: 5.0,
            rollout_depth: 9999,
            use_heuristic_rollout: true,
            open_decklists: false,
        }
    }
}

pub struct Bot {
    pub config: BotConfig,
    pub card_database: Arc<CardDatabase>,
    pub perspective_player: u8,
    pub sampler: DeterminizationSampler,
    pub network: PolicyNet,
}

impl Bot {
    /// Fair bot: only our own deck list is used; the opponent's hidden cards
    /// are sampled anonymously.
    pub fn new_fair(
        card_database: Arc<CardDatabase>,
        perspective_player: u8,
        our_deck: &[String],
    ) -> Self {
        let sampler = DeterminizationSampler::new_fair(Arc::clone(&card_database), our_deck);
        Self {
            config: BotConfig::default(),
            card_database,
            perspective_player,
            sampler,
            network: PolicyNet::new(),
        }
    }

    /// Open-lists bot (research/tournament mode): also uses the opponent's
    /// deck list for determinization.
    pub fn new(
        card_database: Arc<CardDatabase>,
        perspective_player: u8,
        our_deck: &[String],
        opp_deck: &[String],
    ) -> Self {
        let sampler =
            DeterminizationSampler::new(Arc::clone(&card_database), our_deck, opp_deck);
        let mut bot = Self {
            config: BotConfig::default(),
            card_database,
            perspective_player,
            sampler,
            network: PolicyNet::new(),
        };
        bot.config.open_decklists = true;
        bot
    }

    pub fn choose_action(&self, state: &GameState) -> Action {
        let actions = crate::game_setup::generate_possible_actions(state);
        if actions.is_empty() {
            return Action {
                description: "pass".into(),
                description_ja: None,
                action_type: ActionType::Pass,
                parameters: None,
                selected: None,
            };
        }
        if actions.len() == 1 {
            return actions.into_iter().next().unwrap();
        }

        let obs = PublicObservation::from_state(state, self.perspective_player);
        search(&obs, &actions, &self.config, &self.sampler)
    }

    pub fn set_iterations(&mut self, n: u32) {
        self.config.iterations = n;
    }
}
