mod determinization;
pub mod encoding;
pub mod evaluation;
mod ismcts;
pub mod neural;
mod observation;
pub mod weights;

pub use ismcts::search;
pub use ismcts::search_1ply;
pub use neural::PolicyNet;
pub use observation::PublicObservation;

use crate::card::CardDatabase;
use crate::game_setup::{Action, ActionType};
use crate::game_state::GameState;
use crate::Arc;

use determinization::DeterminizationSampler;
use encoding::{action_target_zone, ActionEncoding};

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
            iterations: 10000,
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
    pub network: PolicyNet,
}

impl Bot {
    pub fn new(
        card_database: Arc<CardDatabase>,
        perspective_player: u8,
        our_deck: &[String],
        opp_deck: &[String],
    ) -> Self {
        let sampler = DeterminizationSampler::new(Arc::clone(&card_database), our_deck, opp_deck);
        Self {
            config: BotConfig::default(),
            card_database,
            perspective_player,
            sampler,
            network: PolicyNet::new(),
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
        search(&obs, &actions, &self.config, &self.sampler)
    }

    pub fn set_iterations(&mut self, n: u32) {
        self.config.iterations = n;
    }
}

fn action_type_idx(t: &ActionType) -> u8 {
    match t {
        ActionType::Pass => 0,
        ActionType::RockChoice => 1,
        ActionType::PaperChoice => 2,
        ActionType::ScissorsChoice => 3,
        ActionType::ChooseFirstAttacker => 4,
        ActionType::ChooseSecondAttacker => 5,
        ActionType::MulliganHeader => 6,
        ActionType::SelectMulligan => 7,
        ActionType::ConfirmMulligan => 8,
        ActionType::SkipMulligan => 9,
        ActionType::LiveCardHeader => 10,
        ActionType::SelectLiveCard => 11,
        ActionType::ConfirmLiveCardSet => 12,
        ActionType::SkipLiveCardSet => 13,
        ActionType::PlayMemberToStage => 14,
        ActionType::UseAbility => 15,
        ActionType::SetLiveCard => 16,
        ActionType::FinishLiveCardSet => 17,
        ActionType::ChoiceDecision => 18,
        ActionType::ChoiceSelect => 19,
        ActionType::ChoiceSkip => 20,
        ActionType::ChoiceOption => 21,
        ActionType::ChoicePosition => 22,
        ActionType::EnergyCharge => 23,
        ActionType::PassRemaining => 24,
    }
}
