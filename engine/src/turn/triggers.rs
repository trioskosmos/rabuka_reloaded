use crate::game_state::{GameState, AbilityTrigger};

impl super::TurnEngine {
    pub(crate) fn trigger_debut_abilities(game_state: &mut GameState, player_id: &str, card_no: &str, cost_paid: u32, baton_touch_used: bool) {
        if baton_touch_used && cost_paid >= 10 { return; }

        let player_id_clone = player_id.to_string();
        let card_no_clone = card_no.to_string();
        let mut abilities_to_trigger = Vec::new();

        let _played_card_cost = {
            let player = if player_id_clone == game_state.player1.id { &game_state.player1 } else { &game_state.player2 };
            let areas = [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide];
            let mut found_cost = None;
            for area in areas {
                if let Some(card_id) = player.stage.get_area(area) {
                    if let Some(card) = game_state.card_database.get_card(card_id) {
                        if card.card_no == card_no_clone { found_cost = Some(card.cost); break; }
                    }
                }
            }
            found_cost
        };

        {
            let player = if player_id_clone == game_state.player1.id { &game_state.player1 } else { &game_state.player2 };
            let areas = [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide];
            for area in areas {
                if let Some(card_id) = player.stage.get_area(area) {
                    if let Some(card) = game_state.card_database.get_card(card_id) {
                        if card.card_no == card_no_clone {
                            for ability in &card.abilities {
                                if ability.triggers.as_ref().map_or(false, |t| t.contains(crate::triggers::DEBUT) || t.contains(crate::triggers::DEBUT_EN)) {
                                    let requires_baton_touch = ability.full_text.contains(crate::triggers::BATON_TOUCH) && ability.full_text.contains(crate::triggers::DEBUT_EN);
                                    if requires_baton_touch {
                                        if !baton_touch_used { continue; }
                                        if let Some(replaced_cost) = game_state.baton_touch_replaced_member_cost {
                                            let current_cost = card.cost.unwrap_or(0);
                                            if replaced_cost >= current_cost { continue; }
                                        }
                                    }
                                    let ability_id = format!("{}_{}", card_no_clone, ability.full_text);
                                    abilities_to_trigger.push((ability_id, card_no_clone.clone()));
                                }
                            }
                            break;
                        }
                    }
                }
            }
        }

        for (ability_id, card_no) in abilities_to_trigger {
            game_state.trigger_auto_ability(ability_id, AbilityTrigger::Debut, player_id_clone.clone(), Some(card_no), None);
        }
    }

    pub fn trigger_live_start_abilities(game_state: &mut GameState, player_id: &str) {
        let player_id_clone = player_id.to_string();
        let mut abilities_to_trigger = Vec::new();

        {
            let player = if player_id_clone == game_state.player1.id { &game_state.player1 } else { &game_state.player2 };
            for card_id in &player.live_card_zone.cards {
                if let Some(card) = game_state.card_database.get_card(*card_id) {
                    for ability in &card.abilities {
                        if ability.triggers.as_ref().map_or(false, |t| t == crate::triggers::LIVE_START) {
                            let ability_id = format!("{}_{}", card.card_no, ability.full_text);
                            abilities_to_trigger.push((ability_id, card.card_no.clone()));
                        }
                    }
                }
            }
            for &card_id in &player.stage.stage {
                if card_id != -1 {
                    if let Some(card) = game_state.card_database.get_card(card_id) {
                        for ability in &card.abilities {
                            if ability.triggers.as_ref().map_or(false, |t| t == crate::triggers::LIVE_START) {
                                let ability_id = format!("{}_{}", card.card_no, ability.full_text);
                                abilities_to_trigger.push((ability_id, card.card_no.clone()));
                            }
                        }
                    }
                }
            }
        }

        for (ability_id, card_no) in abilities_to_trigger {
            game_state.trigger_auto_ability(ability_id, AbilityTrigger::LiveStart, player_id_clone.clone(), Some(card_no), None);
        }
    }

    pub fn trigger_live_success_abilities(game_state: &mut GameState, player_id: &str) {
        let player_id_clone = player_id.to_string();
        let mut abilities_to_trigger = Vec::new();

        {
            let player = if player_id_clone == game_state.player1.id { &game_state.player1 } else { &game_state.player2 };
            for (_area, index) in [(crate::zones::MemberArea::LeftSide, 0), (crate::zones::MemberArea::Center, 1), (crate::zones::MemberArea::RightSide, 2)] {
                let card_id = player.stage.stage[index];
                if card_id != -1 {
                    if let Some(card) = game_state.card_database.get_card(card_id) {
                        for ability in &card.abilities {
                            if ability.triggers.as_ref().map_or(false, |t| t == crate::triggers::LIVE_SUCCESS) {
                                let ability_id = format!("{}_{}", card.card_no, ability.full_text);
                                abilities_to_trigger.push((ability_id, card.card_no.clone()));
                            }
                        }
                    }
                }
            }
            for card_id in &player.live_card_zone.cards {
                if let Some(card) = game_state.card_database.get_card(*card_id) {
                    for ability in &card.abilities {
                        if ability.triggers.as_ref().map_or(false, |t| t == crate::triggers::LIVE_SUCCESS || t.contains(crate::triggers::LIVE_SUCCESS_EN)) {
                            let ability_id = format!("{}_{}", card.card_no, ability.full_text);
                            abilities_to_trigger.push((ability_id, card.card_no.clone()));
                        }
                    }
                }
            }
        }

        for (ability_id, card_no) in abilities_to_trigger {
            game_state.trigger_auto_ability(ability_id, AbilityTrigger::LiveSuccess, player_id_clone.clone(), Some(card_no), None);
        }
    }
}
