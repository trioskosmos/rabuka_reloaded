use crate::card::{Ability, AbilityEffect, Keyword};
use crate::game_state::{GameState, Phase};
use crate::zones::MemberArea;
use super::types::{Choice, ExecutionContext};

#[allow(dead_code)]
pub struct AbilityResolver<'a> {
    pub game_state: &'a mut GameState,
    pub pending_choice: Option<Choice>,
    pub looked_at_cards: Vec<i16>,
    pub duration_effects: Vec<(String, String)>,
    pub current_ability: Option<crate::card::Ability>,
    pub activating_card_id: Option<i16>,
    pub execution_context: ExecutionContext,
    pub current_effect: Option<AbilityEffect>,
    pre_resolution_snapshot: Option<GameState>,
}

#[allow(dead_code)]
impl<'a> AbilityResolver<'a> {
    pub fn new(game_state: &'a mut GameState) -> Self {
        let activating_card_id = game_state.activating_card;
        AbilityResolver {
            game_state,
            pending_choice: None,
            looked_at_cards: Vec::new(),
            duration_effects: Vec::new(),
            current_ability: None,
            activating_card_id,
            execution_context: ExecutionContext::None,
            current_effect: None,
            pre_resolution_snapshot: None,
        }
    }

    pub fn capture_snapshot(&mut self) {
        self.pre_resolution_snapshot = Some(self.game_state.clone());
    }

    pub fn rollback(&mut self) {
        if let Some(snapshot) = self.pre_resolution_snapshot.take() {
            *self.game_state = snapshot;
            eprintln!("DEBUG: Rolled back game state due to resolution failure");
        }
    }

    pub fn clear_snapshot(&mut self) {
        self.pre_resolution_snapshot = None;
    }

    pub fn get_pending_choice(&self) -> Option<&Choice> {
        self.pending_choice.as_ref()
    }

    pub fn can_activate_effect(&self, effect: &AbilityEffect) -> bool {
        if let Some(ref activation_condition) = effect.activation_condition_parsed {
            if !self.evaluate_condition(activation_condition) {
                return false;
            }
        }
        if let Some(ref _activation_text) = effect.activation_condition {
            eprintln!("Activation condition: {}", _activation_text);
        }
        true
    }

    pub fn check_keywords(&self, keywords: &[Keyword], card_position: Option<MemberArea>) -> bool {
        for keyword in keywords {
            match keyword {
                Keyword::Center => {
                    if card_position != Some(MemberArea::Center) {
                        return false;
                    }
                }
                Keyword::LeftSide => {
                    if card_position != Some(MemberArea::LeftSide) {
                        return false;
                    }
                }
                Keyword::RightSide => {
                    if card_position != Some(MemberArea::RightSide) {
                        return false;
                    }
                }
                Keyword::Turn1 => {
                    if self.game_state.turn_number != 1 {
                        return false;
                    }
                }
                Keyword::Turn2 => {
                    if self.game_state.turn_number != 2 {
                        return false;
                    }
                }
                Keyword::Debut => {
                    if let Some(pos) = card_position {
                        let card_id = match pos {
                            MemberArea::Center => self.game_state.player1.stage.stage[1],
                            MemberArea::LeftSide => self.game_state.player1.stage.stage[0],
                            MemberArea::RightSide => self.game_state.player1.stage.stage[2],
                        };
                        if card_id == -1 {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                Keyword::LiveStart => {
                    if !matches!(self.game_state.current_phase, Phase::LiveCardSet) {
                        return false;
                    }
                }
                Keyword::LiveSuccess => {
                    if !matches!(self.game_state.current_phase, Phase::LiveVictoryDetermination) {
                        return false;
                    }
                }
                Keyword::PositionChange => {
                    return self.game_state.position_change_occurred_this_turn;
                }
                Keyword::FormationChange => {
                    return self.game_state.formation_change_occurred_this_turn;
                }
            }
        }
        true
    }

    pub fn resolve_ability(&mut self, ability: &Ability, activating_card: Option<i16>, ability_index: usize) -> Result<(), String> {
        eprintln!("DEBUG: RESOLVING ABILITY - triggers: {:?}, full_text: {}, activating_card: {:?}",
            ability.triggers, ability.full_text, activating_card);

        if let Some(card_id) = activating_card {
            let pid = self.game_state.active_player().id.clone();
            self.game_state.publish_event(crate::events::GameEvent::AbilityActivated {
                card_id, player_id: pid, ability_index,
            });
        }

        if let Some(use_limit) = ability.use_limit {
            if let Some(card_id) = activating_card {
                let ability_key = format!("{}_{}_{}", card_id, ability_index, self.game_state.turn_number);
                if self.game_state.turn_limited_abilities_used.contains(&ability_key) {
                    eprintln!("DEBUG: Ability already used this turn (use_limit: {})", use_limit);
                    return Err(format!("Ability has already been used this turn (use_limit: {})", use_limit));
                }
                self.game_state.turn_limited_abilities_used.insert(ability_key);
            }
        }

        self.current_ability = Some(ability.clone());
        self.game_state.activating_card = activating_card;
        self.capture_snapshot();

        if let Some(ref cost) = ability.cost {
            if let Err(e) = self.pay_cost(cost) {
                self.rollback();
                return Err(e);
            }
        }

        if self.pending_choice.is_some() {
            eprintln!("DEBUG: Pending choice from cost payment, pausing ability execution");
            if let Ok(json) = serde_json::to_value(&self.pending_choice) {
                self.game_state.pending_choice = Some(json);
            }
            return Ok(());
        }

        if let Some(ref effect) = ability.effect {
            let ir_effect = crate::ir::effect::Effect::from_ability_effect(effect);
            if let Err(e) = self.execute_effect_ir(&ir_effect) {
                self.rollback();
                return Err(e);
            }

            if self.pending_choice.is_some() {
                if let Ok(json) = serde_json::to_value(&self.pending_choice) {
                    self.game_state.pending_choice = Some(json);
                }
                return Ok(());
            }
        }

        self.clear_snapshot();

        if let Some(card_id) = self.game_state.activating_card {
            let pid = self.game_state.active_player().id.clone();
            self.game_state.publish_event(crate::events::GameEvent::AbilityResolved {
                card_id, player_id: pid, ability_index,
            });
        }

        self.game_state.flush_events();
        self.game_state.activating_card = None;
        self.current_ability = None;
        Ok(())
    }

    pub fn card_matches_type(&self, card_id: i16, card_type_filter: Option<&str>) -> bool {
        match card_type_filter {
            Some("live_card") => self.game_state.card_database.get_card(card_id).map(|c| c.is_live()).unwrap_or(false),
            Some("member_card") => self.game_state.card_database.get_card(card_id).map(|c| c.is_member()).unwrap_or(false),
            Some("energy_card") => self.game_state.card_database.get_card(card_id).map(|c| c.is_energy()).unwrap_or(false),
            None => true,
            _ => true,
        }
    }

    pub fn card_matches_group(&self, card_id: i16, group_filter: Option<&String>) -> bool {
        match group_filter {
            Some(group_name) => self.game_state.card_database.get_card(card_id).map(|c| c.group == *group_name).unwrap_or(false),
            None => true,
        }
    }

    pub fn card_matches_cost_limit(&self, card_id: i16, cost_limit: Option<u32>) -> bool {
        match cost_limit {
            Some(max_cost) => self.game_state.card_database.get_card(card_id).and_then(|c| c.cost).map(|c| c <= max_cost).unwrap_or(false),
            None => true,
        }
    }
}
