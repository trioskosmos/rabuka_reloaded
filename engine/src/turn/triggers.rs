use crate::game_state::{AbilityTrigger, GameState};

impl super::TurnEngine {
    pub(crate) fn trigger_debut_abilities(
        game_state: &mut GameState,
        player_id: &str,
        card_no: &str,
        _cost_paid: u32,
        baton_touch_used: bool,
    ) {
        let player_id_clone = player_id.to_string();
        let card_no_clone = card_no.to_string();
        let mut abilities_to_trigger: Vec<(String, String, i16)> = Vec::new();

        let _played_card_cost = {
            let player = if player_id_clone == game_state.player1.id {
                &game_state.player1
            } else {
                &game_state.player2
            };
            let areas = [
                crate::zones::MemberArea::LeftSide,
                crate::zones::MemberArea::Center,
                crate::zones::MemberArea::RightSide,
            ];
            let mut found_cost = None;
            for area in areas {
                if let Some(card_id) = player.stage.get_area(area) {
                    if let Some(card) = game_state.card_database.get_card(card_id) {
                        if card.card_no == card_no_clone {
                            found_cost = Some(card.cost);
                            break;
                        }
                    }
                }
            }
            found_cost
        };

        {
            let player = if player_id_clone == game_state.player1.id {
                &game_state.player1
            } else {
                &game_state.player2
            };
            let areas = [
                crate::zones::MemberArea::LeftSide,
                crate::zones::MemberArea::Center,
                crate::zones::MemberArea::RightSide,
            ];
            for area in areas {
                if let Some(card_id) = player.stage.get_area(area) {
                    if let Some(card) = game_state.card_database.get_card(card_id) {
                        if card.card_no == card_no_clone {
                            for ability in &card.abilities {
                                if ability.triggers.as_ref().map_or(false, |t| {
                                    t.contains(crate::triggers::DEBUT)
                                        || t.contains(crate::triggers::DEBUT_EN)
                                }) {
                                    // Skip abilities that require baton touch if baton touch wasn't used
                                    if !baton_touch_used
                                        && ability
                                            .effect
                                            .as_ref()
                                            .and_then(|e| e.condition.as_ref())
                                            .map_or(false, |c| {
                                                c.baton_touch_trigger.unwrap_or(false)
                                            })
                                    {
                                        continue;
                                    }
                                    let ability_id =
                                        format!("{}_{}", card_no_clone, ability.full_text);
                                    abilities_to_trigger.push((
                                        ability_id,
                                        card_no_clone.clone(),
                                        card_id,
                                    ));
                                }
                            }
                            break;
                        }
                    }
                }
            }
        }

        for (ability_id, card_no, stage_card_id) in abilities_to_trigger {
            game_state.trigger_auto_ability(
                ability_id,
                AbilityTrigger::Debut,
                player_id_clone.clone(),
                Some(card_no),
                Some(stage_card_id),
            );
        }
    }

    pub fn trigger_live_start_abilities(game_state: &mut GameState, player_id: &str) {
        let player_id_clone = player_id.to_string();
        let mut abilities_to_trigger: Vec<(String, String, Option<i16>)> = Vec::new();

        {
            let player = if player_id_clone == game_state.player1.id {
                &game_state.player1
            } else {
                &game_state.player2
            };
            for card_id in &player.live_card_zone.cards {
                if let Some(card) = game_state.card_database.get_card(*card_id) {
                    for ability in &card.abilities {
                        if ability
                            .triggers
                            .as_ref()
                            .map_or(false, |t| t.contains(crate::triggers::LIVE_START))
                        {
                            let ability_id = format!("{}_{}", card.card_no, ability.full_text);
                            abilities_to_trigger.push((
                                ability_id,
                                card.card_no.clone(),
                                Some(*card_id),
                            ));
                        }
                    }
                }
            }
            for &card_id in &player.stage.stage {
                if card_id != -1 {
                    if let Some(card) = game_state.card_database.get_card(card_id) {
                        for ability in &card.abilities {
                            if ability
                                .triggers
                                .as_ref()
                                .map_or(false, |t| t.contains(crate::triggers::LIVE_START))
                            {
                                let ability_id = format!("{}_{}", card.card_no, ability.full_text);
                                abilities_to_trigger.push((
                                    ability_id,
                                    card.card_no.clone(),
                                    Some(card_id),
                                ));
                            }
                        }
                    }
                }
            }
        }

        eprintln!(
            "[LIVE_START_TRIGGER] triggering {} abilities for player {}",
            abilities_to_trigger.len(),
            player_id
        );
        for (ability_id, card_no, explicit_card_id) in abilities_to_trigger {
            eprintln!(
                "[LIVE_START_TRIGGER]   ability={} card_no={}",
                ability_id, card_no
            );
            game_state.trigger_auto_ability(
                ability_id,
                AbilityTrigger::LiveStart,
                player_id_clone.clone(),
                Some(card_no),
                explicit_card_id,
            );
        }
    }

    pub fn trigger_auto_abilities_for_player(game_state: &mut GameState, player_id: &str) {
        eprintln!("[AUTO_TRIGGER] checking stage for player {}", player_id);
        // Delegate to GameState's method, which handles the scan + enqueue
        game_state.trigger_auto_abilities_for_player(player_id);
    }

    pub fn trigger_and_process_auto_abilities(game_state: &mut GameState, player_id: &str) {
        Self::trigger_auto_abilities_for_player(game_state, player_id);
        game_state.process_pending_auto_abilities(player_id);
    }

    pub fn trigger_live_success_abilities(game_state: &mut GameState, player_id: &str) {
        let player_id_clone = player_id.to_string();
        let mut abilities_to_trigger: Vec<(String, String, i16)> = Vec::new();

        // LiveSuccess only triggers when the live card's need_heart is satisfied
        if !game_state.should_trigger_live_success(if player_id_clone == game_state.player1.id {
            &game_state.player1
        } else {
            &game_state.player2
        }) {
            return;
        }

        {
            let player = if player_id_clone == game_state.player1.id {
                &game_state.player1
            } else {
                &game_state.player2
            };
            for (_area, index) in [
                (crate::zones::MemberArea::LeftSide, 0),
                (crate::zones::MemberArea::Center, 1),
                (crate::zones::MemberArea::RightSide, 2),
            ] {
                let card_id = player.stage.stage[index];
                if card_id != -1 {
                    if let Some(card) = game_state.card_database.get_card(card_id) {
                        for ability in &card.abilities {
                            if ability
                                .triggers
                                .as_ref()
                                .map_or(false, |t| t == crate::triggers::LIVE_SUCCESS)
                            {
                                eprintln!(
                                    "[TRIGGER] live_success stage: card={} trigger={:?}",
                                    card.card_no, ability.triggers
                                );
                                let ability_id = format!("{}_{}", card.card_no, ability.full_text);
                                abilities_to_trigger.push((
                                    ability_id,
                                    card.card_no.clone(),
                                    card_id,
                                ));
                            }
                        }
                    }
                }
            }
            for card_id in &player.live_card_zone.cards {
                if let Some(card) = game_state.card_database.get_card(*card_id) {
                    for ability in &card.abilities {
                        let trigger_match = ability.triggers.as_ref().map_or(false, |t| {
                            t == crate::triggers::LIVE_SUCCESS
                                || t.contains(crate::triggers::LIVE_SUCCESS_EN)
                        });
                        eprintln!(
                            "[TRIGGER] live_success live_card: card={} trigger={:?} match={}",
                            card.card_no, ability.triggers, trigger_match
                        );
                        if trigger_match {
                            let ability_id = format!("{}_{}", card.card_no, ability.full_text);
                            abilities_to_trigger.push((ability_id, card.card_no.clone(), *card_id));
                        }
                    }
                }
            }
        }

        for (ability_id, card_no, source_card_id) in abilities_to_trigger {
            game_state.trigger_auto_ability(
                ability_id,
                AbilityTrigger::LiveSuccess,
                player_id_clone.clone(),
                Some(card_no),
                Some(source_card_id),
            );
        }
    }
}
