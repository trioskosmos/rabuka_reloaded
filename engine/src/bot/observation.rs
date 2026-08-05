use std::hash::{Hash, Hasher};

use crate::game_state::{GameResult, GameState, Phase};

/// Information visible to one player in PVP mode.
/// Mirrors the filtering logic in web_server.rs::filter_display_for_player().
#[derive(Debug, Clone)]
pub struct PublicObservation {
    pub me: PlayerView,
    pub opp: PlayerView,
    pub current_phase: Phase,
    pub turn_number: u8,
    pub game_result: GameResult,
    pub resolution_zone: Vec<i16>,
}

#[derive(Debug, Clone)]
pub struct PlayerView {
    pub hand: Vec<i16>,
    pub hand_size: usize,
    pub stage: [i16; 3],
    pub under_cards: [Vec<i16>; 3],
    pub energy_zone: Vec<i16>,
    pub active_energy_count: usize,
    pub waitroom: Vec<i16>,
    pub success_zone: Vec<i16>,
    pub live_zone: Vec<i16>,
    pub main_deck_size: usize,
    pub energy_deck_size: usize,
    pub is_first_attacker: bool,
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

        let me = PlayerView {
            hand: my_data.hand.cards.to_vec(),
            hand_size: my_data.hand.cards.len(),
            stage: my_data.stage.stage,
            under_cards: [
                my_data.stage.under_cards[0].to_vec(),
                my_data.stage.under_cards[1].to_vec(),
                my_data.stage.under_cards[2].to_vec(),
            ],
            energy_zone: my_data.energy_zone.cards.to_vec(),
            active_energy_count: my_data.energy_zone.active_count() as usize,
            waitroom: my_data.waitroom.cards.to_vec(),
            success_zone: my_data.success_live_card_zone.cards.to_vec(),
            live_zone: my_data.live_card_zone.cards.to_vec(),
            main_deck_size: my_data.main_deck.cards.len(),
            energy_deck_size: my_data.energy_deck.cards.len(),
            is_first_attacker: my_data.is_first_attacker,
        };

        let opp_live = if opponent_performed {
            opp_data.live_card_zone.cards.to_vec()
        } else {
            Vec::new()
        };

        let opp = PlayerView {
            hand: Vec::new(),
            hand_size: opp_data.hand.cards.len(),
            stage: opp_data.stage.stage,
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
            energy_deck_size: opp_data.energy_deck.cards.len(),
            is_first_attacker: opp_data.is_first_attacker,
        };

        Self {
            me,
            opp,
            current_phase: state.current_phase.clone(),
            turn_number: state.turn_number,
            game_result: state.game_result.clone(),
            resolution_zone: state.resolution_zone.cards.to_vec(),
        }
    }
}

impl Hash for PublicObservation {
    fn hash<H: Hasher>(&self, state: &mut H) {
        phase_u8(&self.current_phase).hash(state);
        self.turn_number.hash(state);
        game_result_u8(&self.game_result).hash(state);
        self.resolution_zone.hash(state);
        hash_view(&self.me, state);
        hash_view(&self.opp, state);
    }
}

fn hash_view<H: Hasher>(p: &PlayerView, state: &mut H) {
    p.hand_size.hash(state);
    p.stage.hash(state);
    for uc in &p.under_cards {
        uc.hash(state);
    }
    p.energy_zone.hash(state);
    p.active_energy_count.hash(state);
    p.waitroom.hash(state);
    p.success_zone.hash(state);
    p.live_zone.hash(state);
    p.main_deck_size.hash(state);
    p.energy_deck_size.hash(state);
    p.is_first_attacker.hash(state);
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
