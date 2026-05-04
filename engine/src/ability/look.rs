use crate::card::AbilityEffect;
use super::types::{Choice, ExecutionContext, LookAndSelectStep};
use super::resolver::AbilityResolver;

impl<'a> AbilityResolver<'a> {
    pub fn execute_look_and_select(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        self.current_effect = Some(effect.clone());

        if let Some(ref look_action) = effect.look_action {
            self.execute_effect(look_action)?;
        }

        if let Some(ref select_action) = effect.select_action {
            let placement_order = select_action.placement_order.as_deref();
            let count = select_action.count.unwrap_or(1);
            let optional = select_action.optional.unwrap_or(false);
            let any_number = select_action.any_number.unwrap_or(false);

            let card_db = &self.game_state.card_database;
            let card_type_filter = select_action.card_type.as_deref();
            let heart_colors_filter = select_action.heart_colors.as_ref();
            let has_filter = card_type_filter.is_some()
                || heart_colors_filter.map_or(false, |c| !c.is_empty());
            if has_filter {
                self.looked_at_cards = self.looked_at_cards.iter().filter(|&&card_id| {
                    super::util::card_matches_type(card_db, card_id, card_type_filter)
                        && super::util::card_matches_heart_colors(card_db, card_id, heart_colors_filter)
                }).copied().collect();
            }

            let available_count = self.looked_at_cards.len();
            let max_select = if any_number { available_count } else { std::cmp::min(count as usize, available_count) };

            let description = if available_count == 0 {
                "No eligible cards found among looked-at cards".to_string()
            } else if any_number {
                format!("Select any number of cards from the {} looked-at cards (or skip) (placement_order: {})",
                    available_count, placement_order.unwrap_or("default"))
            } else if optional {
                format!("Select up to {} card(s) from the {} looked-at cards (or skip) (placement_order: {})",
                    max_select, available_count, placement_order.unwrap_or("default"))
            } else {
                format!("Select {} card(s) from the {} looked-at cards (placement_order: {})",
                    max_select, available_count, placement_order.unwrap_or("default"))
            };

            let choice = Choice::SelectCard {
                zone: "looked_at".to_string(), card_type: select_action.card_type.clone(), count: max_select,
                description, allow_skip: optional || any_number || available_count == 0,
            };
            self.pending_choice = Some(choice);
            self.execution_context = ExecutionContext::LookAndSelect {
                step: LookAndSelectStep::Select { count: max_select },
            };
            return Ok(());
        }

        self.current_effect = None;
        Ok(())
    }
    pub fn execute_reveal(&mut self, source: &str, count: u32, target: &str, card_type: Option<&str>, heart_colors: Option<&Vec<String>>) -> Result<(), String> {
        let card_db = self.game_state.card_database.clone();
        let card_ids: Vec<i16> = {
            let player = self.game_state.resolve_target_player_mut(target);
            match source {
                "hand" => player.hand.cards.iter().copied().collect(),
                "deck" => player.main_deck.cards.iter().take(count as usize).copied().collect(),
                "looked_at" => self.looked_at_cards.iter().filter(|&&card_id| {
                    super::util::card_matches_type(&card_db, card_id, card_type)
                        && super::util::card_matches_heart_colors(&card_db, card_id, heart_colors)
                }).copied().collect(),
                _ => vec![],
            }
        };

        for card_id in &card_ids { self.game_state.revealed_cards.insert(*card_id); }
        Ok(())
    }
    pub fn execute_select(&mut self, source: &str, count: u32, target: &str, card_type: Option<&str>, distinct: Option<&str>, heart_colors: Option<&Vec<String>>) -> Result<(), String> {
        let target = target.to_string();
        let card_db = self.game_state.card_database.clone();
        let player = self.game_state.resolve_target_player_mut(&target);

        let card_ids: Vec<i16> = match source {
            "hand" => player.hand.cards.iter().copied().collect(),
            "deck" => player.main_deck.cards.iter().take(count as usize).copied().collect(),
            "discard" => player.waitroom.cards.iter().copied().collect(),
            "looked_at" => self.looked_at_cards.clone(),
            _ => vec![],
        };

        let filtered: Vec<i16> = card_ids.iter().filter(|&&card_id| {
            super::util::card_matches_type(&card_db, card_id, card_type)
                && super::util::card_matches_heart_colors(&card_db, card_id, heart_colors)
        }).copied().collect();

        if distinct == Some("true") || distinct == Some("distinct") {
            let mut names = std::collections::HashSet::new();
            let unique: Vec<i16> = filtered.into_iter().filter(|&card_id| {
                card_db.get_card(card_id)
                    .map(|c| names.insert(c.name.clone()))
                    .unwrap_or(false)
            }).collect();
            self.looked_at_cards = unique;
        } else { self.looked_at_cards = filtered; }

        self.pending_choice = Some(Choice::SelectCard {
            zone: source.to_string(), card_type: card_type.map(|s| s.to_string()),
            count: count as usize,
            description: format!("Select {} card(s) from {}", count, source),
            allow_skip: false,
        });
        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
        Ok(())
    }
    pub fn execute_look_at(&mut self, count: u32, target: &str, source: &str) -> Result<(), String> {
        let player = self.game_state.resolve_target_player_mut(target);

        let cards = match source {
            "deck" | "deck_top" => player.main_deck.draw_multiple(count as usize),
            "hand" => player.hand.cards.iter().take(count as usize).copied().collect(),
            "discard" => player.waitroom.cards.iter().take(count as usize).copied().collect(),
            "stage" => player.stage.stage.iter().filter(|&&id| id != -1).take(count as usize).copied().collect(),
            "energy_zone" => player.energy_zone.cards.iter().take(count as usize).copied().collect(),
            _ => vec![],
        };

        self.looked_at_cards = cards;
        Ok(())
    }
    pub fn execute_reveal_per_group(&mut self, source: &str, count: u32, target: &str) -> Result<(), String> {
        let card_db = self.game_state.card_database.clone();
        let card_ids: Vec<i16> = {
            let player = self.game_state.resolve_target_player_mut(target);
            match source {
                "hand" => player.hand.cards.iter().copied().collect(),
                "deck" => player.main_deck.cards.iter().take(count as usize).copied().collect(),
                "discard" => player.waitroom.cards.iter().copied().collect(),
                "looked_at" => self.looked_at_cards.clone(),
                _ => vec![],
            }
        };

        let mut by_group: std::collections::HashMap<String, Vec<i16>> = std::collections::HashMap::new();
        for &card_id in &card_ids {
            let group_name = card_db.get_card(card_id).map(|c| c.group.clone()).unwrap_or_default();
            by_group.entry(group_name).or_default().push(card_id);
        }

        for (_group, members) in &by_group {
            for &card_id in members {
                self.game_state.revealed_cards.insert(card_id);
            }
        }
        Ok(())
    }
}
