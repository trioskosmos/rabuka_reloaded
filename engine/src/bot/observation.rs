use std::hash::{Hash, Hasher};

use crate::ability::types::Choice;
use crate::game_state::{GameResult, GameState, Phase, TurnPhase};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde_support", derive(serde::Serialize, serde::Deserialize))]
pub struct PublicObservation {
    pub me: PlayerView,
    pub opp: PlayerView,
    pub current_phase: Phase,
    pub current_turn_phase: TurnPhase,
    pub turn_number: u8,
    pub game_result: GameResult,
    pub active_player: u8,
    pub can_current_player_act: bool,
    pub pending_choice: bool,
    pub pending_choice_kind: Option<String>,
    pub pending_choice_option_count: usize,
    pub pending_choice_allow_skip: bool,
    pub pending_choice_player: Option<u8>,
    pub resolution_zone: Vec<i16>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde_support", derive(serde::Serialize, serde::Deserialize))]
pub struct PlayerView {
    pub hand: Vec<i16>,
    pub hand_size: usize,
    pub stage: [i16; 3],
    pub stage_waited: [bool; 3],
    pub under_cards: [Vec<i16>; 3],
    pub energy_zone: Vec<i16>,
    pub active_energy_count: usize,
    pub waitroom: Vec<i16>,
    pub success_zone: Vec<i16>,
    pub live_zone: Vec<i16>,
    pub main_deck_size: usize,
    pub energy_deck_size: usize,
    pub is_first_attacker: bool,
    pub features: PlayerFeatures,
}

#[derive(Debug, Clone, Default, Hash)]
#[cfg_attr(feature = "serde_support", derive(serde::Serialize, serde::Deserialize))]
pub struct PlayerFeatures {
    pub stage_cost: u16,
    pub active_blades: u8,
    pub total_blades: u8,
    pub stage_hearts: u16,
    pub active_stage_slots: u8,
    pub waited_stage_slots: u8,
    pub under_member_count: u8,
    pub under_energy_count: u8,
    pub live_score: u16,
    pub success_count: u8,
}

fn player_features(
    state: &GameState,
    player: &crate::player::Player,
    live_cards: &[i16],
) -> PlayerFeatures {
    let stage = &player.stage;
    let stage_cost = stage
        .stage
        .iter()
        .filter_map(|&card_id| {
            state
                .card_database
                .get_card(card_id)
                .and_then(|card| card.cost)
        })
        .fold(0u16, |sum, cost| sum.saturating_add(u16::from(cost)));
    let active_blades = stage.total_blades(
        &state.card_database,
        &state.mods.blade_modifiers,
        &state.mods.orientation_modifiers,
        false,
    );
    let total_blades = stage.total_blades(
        &state.card_database,
        &state.mods.blade_modifiers,
        &state.mods.orientation_modifiers,
        true,
    );
    let hearts = player.calculate_stage_hearts(
        &state.card_database,
        &state.mods.heart_color_multiplier,
        &state.mods.heart_override,
        &state.mods.heart_modifiers,
        &state.mods.heart_copy,
    );
    let mut features = PlayerFeatures {
        stage_cost,
        active_blades,
        total_blades,
        stage_hearts: u16::from(hearts.hearts.values_sum()),
        ..PlayerFeatures::default()
    };
    for &card_id in &stage.stage {
        if card_id == -1 {
            continue;
        }
        let waited = state
            .mods
            .orientation_modifiers
            .get(&card_id)
            .map(|orientation| *orientation == crate::core::game_modifiers::CardOrientation::Wait)
            .unwrap_or(false);
        if waited {
            features.waited_stage_slots = features.waited_stage_slots.saturating_add(1);
        } else {
            features.active_stage_slots = features.active_stage_slots.saturating_add(1);
        }
    }
    for under in &stage.under_cards {
        for &card_id in under {
            if state
                .card_database
                .get_card(card_id)
                .is_some_and(|card| card.is_member())
            {
                features.under_member_count = features.under_member_count.saturating_add(1);
            } else {
                features.under_energy_count = features.under_energy_count.saturating_add(1);
            }
        }
    }
    for &card_id in live_cards {
        let Some(card) = state.card_database.get_card(card_id) else {
            continue;
        };
        let base = i32::from(card.get_score());
        let set = i32::from(state.mods.get_score_set_modifier(card_id));
        let additive = i32::from(
            state.mods.get_score_modifier(card_id) - state.mods.get_score_set_modifier(card_id),
        );
        let effective = if set != 0 { set } else { base + additive };
        features.live_score = features
            .live_score
            .saturating_add(effective.max(0) as u16);
    }
    features.success_count = player.success_live_card_zone.cards.len().min(u8::MAX as usize) as u8;
    features
}

fn pending_choice_features(state: &GameState) -> (Option<&'static str>, usize, bool) {
    match state.get_pending_choice() {
        Some(Choice::SelectCard { count, allow_skip, .. }) => {
            (Some("SelectCard"), *count, *allow_skip)
        }
        Some(Choice::SelectTarget { options, allow_skip, .. }) => {
            (Some("SelectTarget"), options.as_ref().map_or(0, Vec::len), *allow_skip)
        }
        Some(Choice::SelectPosition { allow_skip, .. }) => {
            (Some("SelectPosition"), 3, *allow_skip)
        }
        Some(Choice::SelectHeartColor { options, .. }) => {
            (Some("SelectHeartColor"), options.len(), false)
        }
        Some(Choice::SelectHeartType { options, .. }) => {
            (Some("SelectHeartType"), options.len(), false)
        }
        Some(Choice::SelectAutoAbility { options, .. }) => {
            (Some("SelectAutoAbility"), options.len(), false)
        }
        Some(Choice::SelectLiveSuccess { options, .. }) => {
            (Some("SelectLiveSuccess"), options.len(), false)
        }
        None => (None, 0, false),
    }
}

fn is_waited(state: &GameState, card_id: i16) -> bool {
    state
        .mods
        .orientation_modifiers
        .get(&card_id)
        .map(|orientation| *orientation == crate::core::game_modifiers::CardOrientation::Wait)
        .unwrap_or(false)
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

        let me_live = my_data.live_card_zone.cards.clone();
        let me_features = player_features(state, my_data, &me_live);
        let me_waited = [
            is_waited(state, my_data.stage.stage[0]),
            is_waited(state, my_data.stage.stage[1]),
            is_waited(state, my_data.stage.stage[2]),
        ];
        let me = PlayerView {
            hand: my_data.hand.cards.to_vec(),
            hand_size: my_data.hand.cards.len(),
            stage: my_data.stage.stage,
            stage_waited: me_waited,
            under_cards: [
                my_data.stage.under_cards[0].to_vec(),
                my_data.stage.under_cards[1].to_vec(),
                my_data.stage.under_cards[2].to_vec(),
            ],
            energy_zone: my_data.energy_zone.cards.to_vec(),
            active_energy_count: my_data.energy_zone.active_count() as usize,
            waitroom: my_data.waitroom.cards.to_vec(),
            success_zone: my_data.success_live_card_zone.cards.to_vec(),
            live_zone: me_live.to_vec(),
            main_deck_size: my_data.main_deck.cards.len(),
            energy_deck_size: my_data.energy_deck.cards.len(),
            is_first_attacker: my_data.is_first_attacker,
            features: me_features,
        };

        let opp_live = if opponent_performed {
            opp_data.live_card_zone.cards.to_vec()
        } else {
            Vec::new()
        };
        let opp_features = player_features(state, opp_data, &opp_live);
        let opp_waited = [
            is_waited(state, opp_data.stage.stage[0]),
            is_waited(state, opp_data.stage.stage[1]),
            is_waited(state, opp_data.stage.stage[2]),
        ];

        let opp = PlayerView {
            hand: Vec::new(),
            hand_size: opp_data.hand.cards.len(),
            stage: opp_data.stage.stage,
            stage_waited: opp_waited,
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
            features: opp_features,
        };

        let active_player = if state.active_player().id == state.player1.id {
            0
        } else {
            1
        };
        let (pending_choice_kind, pending_choice_option_count, pending_choice_allow_skip) =
            pending_choice_features(state);
        let pending_choice_player = state
            .get_pending_choice_player_id()
            .and_then(|id| match id.as_str() {
                "p1" | "player1" => Some(0),
                "p2" | "player2" => Some(1),
                _ => None,
            });

        Self {
            me,
            opp,
            current_phase: state.current_phase.clone(),
            current_turn_phase: state.current_turn_phase,
            turn_number: state.turn_number,
            game_result: state.game_result.clone(),
            active_player,
            can_current_player_act: state.can_player_act(i32::from(perspective_player)),
            pending_choice: state.has_pending_choice(),
            pending_choice_kind: pending_choice_kind.map(str::to_owned),
            pending_choice_option_count,
            pending_choice_allow_skip,
            pending_choice_player,
            resolution_zone: state.resolution_zone.cards.to_vec(),
        }
    }
}

impl Hash for PublicObservation {
    fn hash<H: Hasher>(&self, state: &mut H) {
        phase_u8(&self.current_phase).hash(state);
        self.current_turn_phase.hash(state);
        self.turn_number.hash(state);
        game_result_u8(&self.game_result).hash(state);
        self.active_player.hash(state);
        self.can_current_player_act.hash(state);
        self.pending_choice.hash(state);
        self.pending_choice_kind.hash(state);
        self.pending_choice_option_count.hash(state);
        self.pending_choice_allow_skip.hash(state);
        self.pending_choice_player.hash(state);
        self.resolution_zone.hash(state);
        hash_view(&self.me, state);
        hash_view(&self.opp, state);
    }
}

fn hash_view<H: Hasher>(p: &PlayerView, state: &mut H) {
    p.hand.hash(state);
    p.hand_size.hash(state);
    p.stage.hash(state);
    p.stage_waited.hash(state);
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
    p.features.hash(state);
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
