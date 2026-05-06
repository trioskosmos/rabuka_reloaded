use crate::card::{Ability, AbilityEffect, Keyword};
use crate::game_state::{GameState, Phase};
use crate::zones::MemberArea;
use super::types::{Choice, ExecutionContext};
use super::util;

pub struct AbilityResolver<'a> {
    pub game_state: &'a mut GameState,
    pub pending_choice: Option<Choice>,
    pub looked_at_cards: Vec<i16>,
    pub duration_effects: Vec<(String, String)>,
    pub current_ability: Option<crate::card::Ability>,
    pub activating_card_id: Option<i16>,
    pub execution_context: ExecutionContext,
    pub current_effect: Option<AbilityEffect>,
    pub revealed_cost_cards: Vec<i16>,
    pub is_reveal_cost: bool,
    pub last_draw_count: u32,
    pub looked_at_total_count: usize,
    pub selected_cards: Vec<i16>,
}

impl<'a> AbilityResolver<'a> {
    pub fn new(game_state: &'a mut GameState) -> Self {
        let activating_card_id = game_state.activating_card;
        let looked_at_cards = std::mem::take(&mut game_state.looked_at_cards);
        let selected_cards = game_state.ability_queue.current_entry()
            .map(|e| e.selected_card_ids.clone())
            .unwrap_or_default();
        AbilityResolver {
            game_state,
            pending_choice: None,
            looked_at_cards,
            duration_effects: Vec::new(),
            current_ability: None,
            activating_card_id,
            execution_context: ExecutionContext::None,
            current_effect: None,
            revealed_cost_cards: Vec::new(),
            is_reveal_cost: false,
            last_draw_count: 0,
            looked_at_total_count: 0,
            selected_cards,
        }
    }

    pub fn take_looked_at(&mut self) -> Vec<i16> {
        std::mem::take(&mut self.looked_at_cards)
    }

    /// Find matching card indices in a zone, prompt if too many.
    /// Takes &[i16] (read-only — works with Vec, SmallVec, any container).
    /// Returns Ok(Some(indices)) if exact match or fewer.
    /// Returns Ok(None) if too many — sets pending_choice, caller should `return Ok(())`.
    pub fn match_cards_in_zone(
        &mut self,
        cards: &[i16],
        count: usize,
        card_db: &crate::card::CardDatabase,
        card_type: Option<&str>,
        group_name: Option<&str>,
        cost_limit: Option<u32>,
        zone_name: &str,
        prompt_desc: &str,
    ) -> Result<Option<Vec<usize>>, String> {
        let filter = util::CardFilter { card_type, group: group_name, cost_limit, ..util::CardFilter::default() };
        let idxs = util::matching_indices(cards, card_db, &filter, false);
        if idxs.is_empty() || idxs.len() < count {
            return Err(format!("Not enough cards in {}: need {}", zone_name, count));
        }
        if idxs.len() > count {
            self.pending_choice = Some(Choice::SelectCard {
                zone: zone_name.to_string(), card_type: card_type.map(|s| s.to_string()),
                count, description: prompt_desc.to_string(), allow_skip: false,
            });
            self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
            return Ok(None);
        }
        Ok(Some(idxs.into_iter().rev().take(count).collect()))
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
                    if !matches!(self.game_state.current_phase, Phase::LiveCardSetP1Turn | Phase::LiveCardSetP2Turn) {
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

    fn store_pending_choice(&mut self) {
        if let Some(ref choice) = self.pending_choice {
            let mut json = choice.to_frontend_json();
            if let Some(entry) = self.game_state.ability_queue.current_entry() {
                if let Some(ref effect) = entry.ability.effect {
                    if let Some(ref maker) = effect.choice_maker {
                        if let Some(ref mut j) = json {
                            if let Some(obj) = j.as_object_mut() {
                                obj.insert("choice_maker".to_string(), serde_json::Value::String(maker.clone()));
                            }
                        }
                    }
                }
            }
            self.game_state.pending_choice = json;
        }
    }

    pub fn resolve_ability(&mut self, ability: &Ability, activating_card: Option<i16>, ability_index: usize) -> Result<(), String> {

        // Check use_limit before cost, but don't insert until after effect runs
        let ability_key = activating_card.map(|card_id| {
            format!("{}_{}_{}", card_id, ability_index, self.game_state.turn_number)
        });

        if let Some(ref key) = ability_key {
            if let Some(use_limit) = ability.use_limit {
                if self.game_state.turn_limited_abilities_used.contains(key) {
                    return Err(format!("Ability has already been used this turn (use_limit: {})", use_limit));
                }
            }
        }

        self.current_ability = Some(ability.clone());
        self.game_state.activating_card = activating_card;

        eprintln!("[RESOLVE_ABILITY] ability={:?} cost_already_paid={}", ability.full_text.chars().take(60).collect::<String>(), self.game_state.ability_queue.current_entry().map_or(false, |e| e.cost_paid));
        let cost_already_paid = self.game_state.ability_queue.current_entry()
            .map_or(false, |e| e.cost_paid);

        if !cost_already_paid {
            if let Some(ref cost) = ability.cost {
                if let Err(e) = self.pay_cost(cost) {
                    return Err(e);
                }
            }
        }

        if self.pending_choice.is_some() {
            // Only mark cost as paid if the choice was created by the cost
            // (not by a subsequent effect that creates its own choice)
            if !cost_already_paid && ability.cost.is_some() {
                if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
                    entry.cost_paid = true;
                }
            }
            self.store_pending_choice();
            return Ok(());
        }

        if let Some(ref effect) = ability.effect {
            if let Err(e) = self.execute_effect(effect) {
                return Err(e);
            }

            if self.pending_choice.is_some() {
                self.store_pending_choice();
                return Ok(());
            }
        }

        // Insert use_limit key after effect has fully executed
        if let Some(key) = ability_key {
            if ability.use_limit.is_some() {
                self.game_state.turn_limited_abilities_used.insert(key);
            }
        }

        self.game_state.activating_card = None;
        self.current_ability = None;
        Ok(())
    }

    pub fn card_matches_type(&self, card_id: i16, card_type_filter: Option<&str>) -> bool {
        util::card_matches_type(&self.game_state.card_database, card_id, card_type_filter)
    }

    pub fn card_matches_group(&self, card_id: i16, group_filter: Option<&String>) -> bool {
        util::card_matches_group(&self.game_state.card_database, card_id, group_filter)
    }

    pub fn card_matches_cost_limit(&self, card_id: i16, cost_limit: Option<u32>) -> bool {
        util::card_matches_cost_limit(&self.game_state.card_database, card_id, cost_limit)
    }
}
