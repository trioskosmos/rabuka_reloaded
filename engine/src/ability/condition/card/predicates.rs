use super::ConditionContext;
use crate::ability::enums::Zone;
use crate::card::{CardDatabase, Condition, HeartColor};
#[cfg(feature = "no_std")]
use alloc::vec::Vec;

impl<'a> ConditionContext<'a> {
    fn card_has_all_heart(&self, card_db: &CardDatabase, card_id: i16) -> bool {
        let mods = &self.game_state.mods;
        card_db.get_card(card_id).is_some_and(|c| {
            c.base_heart
                .as_ref()
                .is_some_and(|bh| bh.hearts.contains_key(&HeartColor::Heart00))
        }) || mods
            .heart_modifiers
            .get(&card_id)
            .and_then(|m| m.get(&HeartColor::All))
            .is_some_and(|e| e.total() > 0)
            || mods
                .constant_heart_bonuses
                .get(&card_id)
                .and_then(|cols| cols.get("all"))
                .copied()
                .unwrap_or(0)
                > 0
    }

    pub(crate) fn check_heart_type_all(
        &self,
        condition: &Condition,
        player: &crate::player::Player,
        location: &str,
    ) -> bool {
        if condition.get_heart_type() != Some("all") {
            return true;
        }
        if Zone::from_str(location) != Some(Zone::Stage) {
            return true;
        }
        let card_db = &self.game_state.card_database;
        if condition.get_negation().unwrap_or(false) {
            let target_ids: Vec<i16> = self
                .game_state
                .ability_queue
                .current_entry()
                .and_then(|e| e.triggering_member_id)
                .map(|id| vec![id])
                .unwrap_or_else(|| {
                    player
                        .stage
                        .stage
                        .iter()
                        .filter(|&&id| id != -1)
                        .copied()
                        .collect()
                });
            return target_ids
                .iter()
                .any(|&id| id != -1 && !self.card_has_all_heart(card_db, id));
        }
        player
            .stage
            .stage
            .iter()
            .any(|&id| id != -1 && self.card_has_all_heart(card_db, id))
    }

    pub(crate) fn check_heart_type_all_per_card(
        &self,
        condition: &Condition,
        card_db: &CardDatabase,
        card_id: i16,
    ) -> bool {
        if condition.get_heart_type() != Some("all") {
            return true;
        }
        let negate = condition.get_negation().unwrap_or(false);
        self.card_has_all_heart(card_db, card_id) != negate
    }

    pub(crate) fn check_heart_colors(
        &self,
        condition: &Condition,
        player: &crate::player::Player,
        location: &str,
    ) -> bool {
        let colors_binding = condition.get_heart_colors();
        let cols = match &colors_binding {
            Some(c) if !c.is_empty() => *c,
            _ => return true,
        };
        if cols
            .iter()
            .any(|cs| crate::card::parse_heart_color(cs) == HeartColor::Heart00)
        {
            return true;
        }
        if Zone::from_str(location) != Some(Zone::Stage) {
            return true;
        }
        let card_db = &self.game_state.card_database;
        cols.iter().all(|cs| {
            player.stage.stage.iter().any(|&id| {
                id != -1
                    && card_db.get_card(id).is_some_and(|c| {
                        c.base_heart.as_ref().is_some_and(|bh| {
                            bh.hearts.contains_key(&crate::card::parse_heart_color(cs))
                        })
                    })
            })
        })
    }
}

impl<'a> ConditionContext<'a> {
    pub(super) fn printed_cost(&self, card_id: i16) -> u8 {
        self.game_state
            .card_database
            .get_card(card_id)
            .and_then(|card| card.cost)
            .unwrap_or(0)
    }

    pub(super) fn effective_condition_cost(&self, card_id: i16) -> u8 {
        crate::constants::saturate_u8(crate::constants::effective_stat(
            self.printed_cost(card_id),
            self.game_state.mods.get_cost_modifier(card_id),
        ))
    }

    pub(super) fn matches_condition_groups_and_type(
        &self,
        condition: &Condition,
        card_id: i16,
    ) -> bool {
        let card_db = &self.game_state.card_database;
        if let Some(groups) = condition.get_group_names() {
            if !groups.is_empty()
                && !groups.iter().any(|g| {
                    crate::ability::util::card_matches_group_str(card_db, card_id, Some(g))
                })
            {
                return false;
            }
        }
        if let Some(ct) = condition.get_card_type() {
            if !crate::ability::util::card_matches_type(card_db, card_id, Some(ct.as_str())) {
                return false;
            }
        }
        true
    }

    pub(super) fn matches_original_value_filters(
        &self,
        condition: &Condition,
        card_id: i16,
        respect_original_value: bool,
    ) -> bool {
        if !respect_original_value || !condition.get_original_value().unwrap_or(false) {
            return true;
        }
        self.check_original_blade_filter(condition, card_id)
            && self.check_original_heart_filter(condition, card_id)
    }
}

pub(super) fn member_count_operator(condition: &Condition, count: u8) -> Option<&str> {
    if condition.get_original_value().unwrap_or(false) && condition.get_blade_limit().is_none() {
        Some(">=")
    } else {
        condition
            .get_operator()
            .or(if count == 0 { Some("==") } else { Some(">=") })
    }
}
