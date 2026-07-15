use rand::Rng;

use crate::game_setup::{Action, ActionType};

use super::neural::PolicyNet;
use super::observation::PublicObservation;

/// Score actions using the policy network: one forward pass per action, no state clones.
pub fn search_1ply(
    obs: &PublicObservation,
    actions: &[Action],
    _config: &super::BotConfig,
    _sampler: &super::determinization::DeterminizationSampler,
    network: &PolicyNet,
) -> usize {
    let my_cards: Vec<i16> = obs
        .me
        .stage
        .iter()
        .chain(obs.me.hand.iter())
        .copied()
        .collect();
    let opp_cards: Vec<i16> = obs.opp.stage.iter().copied().collect();
    let mut best_idx = 0usize;
    let mut best_score = f64::NEG_INFINITY;

    for (i, action) in actions.iter().enumerate() {
        let act_type = action_type_idx(&action.action_type);
        let act_card = action
            .parameters
            .as_ref()
            .and_then(|p| p.card_id)
            .unwrap_or(0);
        let score = network.score_action(&my_cards, &opp_cards, act_type, act_card) as f64;
        if score > best_score {
            best_score = score;
            best_idx = i;
        }
    }
    best_idx
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

pub(super) fn fastrand(lo: usize, hi: usize) -> usize {
    if hi <= lo {
        return lo;
    }
    rand::thread_rng().gen_range(lo..hi)
}
