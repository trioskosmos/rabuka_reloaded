use crate::constants::MAX_LIVE_CARDS;
use crate::game_state::{GameState, Phase};

impl super::TurnEngine {
    pub fn advance_phase(game_state: &mut GameState) {
        debug_assert!(
            game_state.phase_invariant(),
            "Phase invariant violated before advance_phase"
        );

        if matches!(
            game_state.current_phase,
            Phase::MulliganFirstAttacker | Phase::MulliganSecondAttacker
        ) {
            return;
        }

        if game_state.current_turn_phase == crate::game_state::TurnPhase::FirstAttackerNormal
            || game_state.current_turn_phase == crate::game_state::TurnPhase::SecondAttackerNormal
        {
            match game_state.current_phase {
                Phase::Active => {
                    game_state.reset_keyword_tracking();
                    game_state.recalculate_constants();
                    // Rule 7.4.1: Only the turn player activates their wait cards
                    // Check if the turn player's activation is restricted
                    let turn_player_id = &game_state.active_player().id.clone();
                    let is_activation_blocked = game_state
                        .cannot_activate_members
                        .iter()
                        .any(|t| t == turn_player_id)
                        || game_state
                            .constant_cannot_activate_members
                            .iter()
                            .any(|t| t == turn_player_id);
                    let to_activate: Vec<i16> = if is_activation_blocked {
                        Vec::new()
                    } else {
                        let turn_player = game_state.active_player();
                        turn_player
                            .stage
                            .stage
                            .iter()
                            .filter_map(|&cid| {
                                if cid != -1
                                    && game_state.mods.get_orientation_modifier(cid)
                                        == Some(&"wait".to_string())
                                {
                                    Some(cid)
                                } else {
                                    None
                                }
                            })
                            .collect()
                    };
                    for &cid in &to_activate {
                        game_state.mods.add_orientation_modifier(cid, "active");
                    }
                    game_state.active_player_mut().activate_all_energy();
                    Self::check_timing(game_state);
                    game_state.current_phase = Phase::Energy;
                }
                Phase::Energy => {
                    game_state.recalculate_constants();
                    Self::check_timing(game_state);
                    let _drawn_card = game_state.active_player_mut().draw_energy();
                    Self::check_timing(game_state);
                    game_state.current_phase = Phase::Draw;
                }
                Phase::Draw => {
                    Self::check_timing(game_state);
                    let _drawn = game_state.active_player_mut().draw_card();
                    game_state.recalculate_constants();
                    Self::check_timing(game_state);
                    game_state.current_phase = Phase::Main;
                }
                Phase::Main => {
                    game_state.recalculate_constants();
                    Self::check_timing(game_state);
                    if game_state.current_turn_phase
                        == crate::game_state::TurnPhase::FirstAttackerNormal
                    {
                        game_state.current_turn_phase =
                            crate::game_state::TurnPhase::SecondAttackerNormal;
                        game_state.current_phase = Phase::Active;
                    } else {
                        game_state.current_turn_phase = crate::game_state::TurnPhase::Live;
                        game_state.current_phase = Phase::LiveCardSetFirstAttacker;
                    }
                }
                _ => {}
            }
        } else if game_state.current_turn_phase == crate::game_state::TurnPhase::Live {
            match game_state.current_phase {
                Phase::LiveCardSetFirstAttacker => {
                    game_state.current_phase = Phase::LiveCardSetSecondAttacker;
                    return;
                }
                Phase::LiveCardSetSecondAttacker => {
                    game_state.recalculate_constants();
                    Self::check_timing(game_state);
                    game_state.current_phase = Phase::FirstAttackerPerformance;
                    let first_attacker_id = game_state.first_attacker().id.clone();
                    Self::trigger_live_start_abilities(game_state, &first_attacker_id);
                    game_state.process_pending_auto_abilities(&first_attacker_id);
                    return;
                }
                Phase::FirstAttackerPerformance | Phase::SecondAttackerPerformance => {
                    let is_first =
                        matches!(game_state.current_phase, Phase::FirstAttackerPerformance);
                    Self::execute_performance_phase(game_state, is_first);
                }
                Phase::LiveVictoryDetermination => {
                    Self::execute_live_victory_determination(game_state);
                    if game_state.has_pending_choice() {
                        return;
                    }
                    game_state.clear_revealed_cards();
                    game_state.revealed_cost_cards.clear();
                    game_state.turn_limited_abilities_used.clear();
                    game_state.turn_number += 1;
                    game_state.current_turn_phase =
                        crate::game_state::TurnPhase::FirstAttackerNormal;
                    game_state.current_phase = Phase::Active;
                    game_state.check_expired_effects();
                }
                _ => {}
            }
        }
    }

    fn execute_performance_phase(game_state: &mut GameState, is_first: bool) {
        let mut resolution_zone = std::mem::take(&mut game_state.resolution_zone);
        let card_db = game_state.card_database.clone();
        let bm = game_state.mods.blade_modifiers.clone();
        let ho = game_state.mods.heart_override.clone();
        let hm = game_state.mods.heart_modifiers.clone();
        let btm = game_state.mods.blade_type_modifiers.clone();
        let om = game_state.mods.orientation_modifiers.clone();
        let nhm = game_state.mods.need_heart_modifiers.clone();
        let cannot_live = game_state.is_action_prohibited("cannot_live");
        let player = if is_first {
            game_state.first_attacker_mut()
        } else {
            game_state.second_attacker_mut()
        };
        let performer_id = player.id.clone();
        let perf_data = Self::player_perform_live(
            player,
            &mut resolution_zone,
            &performer_id,
            &card_db,
            &bm,
            &ho,
            &hm,
            &btm,
            &om,
            &nhm,
            cannot_live,
        );
        drop(resolution_zone);

        let turn = game_state.turn_number;
        let note_icons = perf_data.note_icons;
        for cid in &perf_data.revealed_ids {
            game_state.revealed_cards.push(*cid);
        }

        for cid in &perf_data.revealed_ids {
            game_state.cheer_revealed_cards_first(is_first).push(*cid);
        }
        *game_state.cheer_blade_heart_count_mut(is_first) = note_icons;
        let (perf_player_id, perf_player) = if is_first {
            let p = game_state.first_attacker();
            (p.id.clone(), p)
        } else {
            let p = game_state.second_attacker();
            (p.id.clone(), p)
        };
        let snap = crate::turn::live::build_snapshot(
            turn,
            &perf_player_id,
            &perf_data,
            &game_state.card_database,
            perf_player,
            note_icons,
        );
        game_state.performance_snapshots.push(snap);
        let pid = perf_player_id;
        Self::trigger_auto_abilities_for_player(game_state, &pid);
        game_state.process_pending_auto_abilities(&pid);
        if perf_data.draw_effects_occurred {
            Self::trigger_auto_abilities_for_player(game_state, &pid);
            game_state.process_pending_auto_abilities(&pid);
        }
        game_state.current_phase = if is_first {
            Phase::SecondAttackerPerformance
        } else {
            Phase::LiveVictoryDetermination
        };
    }

    pub(crate) fn handle_mulligan_selection(
        game_state: &mut GameState,
        card_id: Option<i16>,
        _card_indices: Option<Vec<usize>>,
    ) -> Result<(), String> {
        let idx = if let Some(indices) = _card_indices {
            indices.get(0).copied().unwrap_or(0)
        } else if let Some(cid) = card_id {
            game_state
                .active_player()
                .get_card_index_by_id(cid)
                .unwrap_or(0)
        } else {
            0
        };
        if let Some(pos) = game_state
            .mulligan_selected_indices
            .iter()
            .position(|&x| x == idx)
        {
            game_state.mulligan_selected_indices.remove(pos);
        } else {
            game_state.mulligan_selected_indices.push(idx);
        }
        Ok(())
    }

    pub(crate) fn handle_mulligan_confirmation(game_state: &mut GameState) -> Result<(), String> {
        let next_phase = match game_state.current_phase {
            Phase::MulliganFirstAttacker => Phase::MulliganSecondAttacker,
            Phase::MulliganSecondAttacker => Phase::Active,
            _ => return Ok(()),
        };
        let mulligan_indices: Vec<usize> = game_state.mulligan_selected_indices.clone();
        let mulligan_count = mulligan_indices.len();
        let player = game_state.active_player_mut();
        for &idx in mulligan_indices.iter().rev() {
            if idx < player.hand.cards.len() {
                let card = player.hand.cards.remove(idx);
                player.main_deck.cards.push(card);
            }
        }
        player.main_deck.shuffle();
        for _ in 0..mulligan_count {
            if let Some(card) = player.main_deck.draw() {
                player.hand.add_card(card);
            }
        }
        game_state.mulligan_selected_indices.clear();
        game_state.current_phase = next_phase;
        println!("Mulligan confirmed: {} cards mulliganed", mulligan_count);
        Ok(())
    }

    pub(crate) fn handle_mulligan_skip(game_state: &mut GameState) -> Result<(), String> {
        game_state.mulligan_selected_indices.clear();
        game_state.current_phase = match game_state.current_phase {
            Phase::MulliganFirstAttacker => Phase::MulliganSecondAttacker,
            Phase::MulliganSecondAttacker => Phase::Active,
            _ => return Ok(()),
        };
        Ok(())
    }

    pub(crate) fn handle_set_live_card(
        game_state: &mut GameState,
        card_id: Option<i16>,
    ) -> Result<(), String> {
        let cid = card_id.ok_or("No card selected for live card set")?;
        let player = game_state.active_player_mut();
        let idx = player
            .get_card_index_by_id(cid)
            .ok_or("Selected card not found in hand")?;
        if !player.hand.cards.is_empty() && idx < player.hand.cards.len() {
            let card = player.hand.cards.remove(idx);
            let live_cards = &mut player.live_card_zone.cards;
            if live_cards.len() >= MAX_LIVE_CARDS {
                return Err("Live card zone is full".to_string());
            }
            live_cards.push(card);
            Ok(())
        } else {
            Err("Invalid card selection".to_string())
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn handle_play_member_to_stage(
        game_state: &mut GameState,
        card_id: Option<i16>,
        card_indices: Option<Vec<usize>>, // For double baton: [area1_idx, area2_idx]
        stage_area: Option<crate::zones::MemberArea>,
        use_baton_touch: Option<bool>,
    ) -> Result<(), String> {
        let use_baton_touch = use_baton_touch.unwrap_or(false);
        if use_baton_touch && game_state.is_action_prohibited("cannot_baton_touch") {
            return Err("Baton touch is prohibited by a restriction effect".to_string());
        }

        let card_db = game_state.card_database.clone();

        // Recalculate constant cost modifiers (hand-based cost reductions, etc.)
        // BEFORE paying cost, so the modifiers are in effect.
        game_state.recalculate_constants();

        let player = game_state.active_player_mut();
        let idx = if let Some(cid) = card_id {
            player
                .get_card_index_by_id(cid)
                .ok_or_else(|| format!("Card with id {} not found in hand", cid))?
        } else {
            player
                .hand
                .cards
                .iter()
                .position(|c| card_db.get_card(*c).map_or(false, |card| card.is_member()))
                .ok_or_else(|| "No member cards in hand".to_string())?
        };

        let card_id = player.hand.cards[idx];

        // Check if double baton: card_indices provides the 2 area indices to replace
        let double_baton_areas: Option<[crate::zones::MemberArea; 2]> =
            card_indices.as_ref().and_then(|indices| {
                if indices.len() == 2 {
                    let areas = [
                        crate::zones::MemberArea::LeftSide,
                        crate::zones::MemberArea::Center,
                        crate::zones::MemberArea::RightSide,
                    ];
                    Some([areas[indices[0]], areas[indices[1]]])
                } else {
                    None
                }
            });

        let area = if let Some(ref db_areas) = double_baton_areas {
            // For double baton, stage_area specifies which of the 2 vacated areas to place in
            stage_area.unwrap_or(db_areas[0])
        } else {
            stage_area.unwrap_or_else(|| {
                let areas = [
                    crate::zones::MemberArea::LeftSide,
                    crate::zones::MemberArea::Center,
                    crate::zones::MemberArea::RightSide,
                ];
                if let Some(empty) = areas.iter().find(|&&a| player.stage.get_area(a).is_none()) {
                    *empty
                } else if !use_baton_touch {
                    areas[0]
                } else {
                    areas[0]
                }
            })
        };

        let card_no = card_db
            .get_card(card_id)
            .map(|c| c.card_no.clone())
            .unwrap_or_default();
        let player_id = player.id.clone();

        // Check if this card has play_baton_touch with count > 1 (double baton touch)
        let has_double_baton = double_baton_areas.is_some()
            || card_db.get_card(card_id).map_or(false, |c| {
                c.abilities.iter().any(|a| {
                    a.effect.as_ref().map_or(false, |ef| {
                        ef.action == "play_baton_touch" && ef.count.unwrap_or(1) > 1
                    })
                })
            });

        // If double baton with explicit areas, replace ALL specified members BEFORE placing the card
        if let Some(db_areas) = double_baton_areas {
            // Replace both specified members first
            let double_replaced_ids: Vec<i16> = {
                let player = game_state.active_player_mut();
                let mut replaced = Vec::new();
                for &area2 in &db_areas {
                    if let Some(existing_card_id) = player.stage.get_area(area2) {
                        let _ = player
                            .remove_member_from_stage_with_recycling(area2 as usize, &card_db);
                        player.waitroom.cards.push(existing_card_id);
                        replaced.push(existing_card_id);
                    }
                }
                replaced
            };
            // Track the non-placement vacated area for empty_area deployment
            let other_vacated = if db_areas[0] != area {
                Some(db_areas[0] as usize)
            } else {
                Some(db_areas[1] as usize)
            };
            game_state.last_vacated_stage_area = other_vacated;
            // Remove card from hand
            let player = game_state.active_player_mut();
            player.hand.cards.remove(idx);
            // Place card in chosen placement area
            player.stage.stage[area as usize] = card_id;
            game_state.record_card_movement(card_id);
            // Record 2 baton touches
            for _ in 0..2 {
                game_state.record_baton_touch();
            }
            game_state.baton_touch_replaced_member_id =
                double_baton_areas.as_ref().and_then(|_areas| {
                    let player = game_state.active_player();
                    player.waitroom.cards.last().copied()
                });
            game_state.active_player_mut().debut_count_this_turn += 1;
            game_state.record_card_appearance(card_id);
            game_state.baton_touch_arriving_card_id = Some(card_id);

            Self::trigger_debut_abilities(
                game_state, &player_id, &card_no, 0,    // cost_paid
                true, // baton_touch_used
            );
            Self::trigger_auto_abilities_for_player(game_state, &player_id);
            for &replaced_id in &double_replaced_ids {
                Self::trigger_discard_auto_abilities(game_state, &player_id, replaced_id);
            }
            game_state.process_pending_auto_abilities(&player_id);
            game_state.recalculate_constants();

            for area in [
                crate::zones::MemberArea::LeftSide,
                crate::zones::MemberArea::Center,
                crate::zones::MemberArea::RightSide,
            ] {
                if game_state.active_player().stage.get_area(area).is_none() {
                    game_state
                        .active_player_mut()
                        .areas_locked_this_turn
                        .insert(area);
                }
            }

            eprintln!("[TRACK_MOVE] card_id={} player_id={}", card_id, player_id);
            game_state.last_area_move_card_id = Some(card_id);
            game_state.last_area_move_by_player = Some(player_id.clone());
            return Ok(());
        }

        let (cost_paid, baton_touch_used, replaced_member_cost, replaced_member_id) =
            player.move_card_from_hand_to_stage(idx, area, use_baton_touch, &card_db)?;
        game_state.record_card_movement(card_id);
        game_state.baton_touch_zero_cost = baton_touch_used && cost_paid == 0;
        game_state.baton_touch_replaced_member_cost = replaced_member_cost;
        game_state.baton_touch_replaced_member_id = replaced_member_id;

        game_state.active_player_mut().debut_count_this_turn += 1;
        game_state.record_card_appearance(card_id);

        if baton_touch_used {
            game_state.record_baton_touch();
            game_state.baton_touch_arriving_card_id = Some(card_id);
            if has_double_baton {
                let second_area = {
                    let player = game_state.active_player();
                    let areas = [
                        crate::zones::MemberArea::LeftSide,
                        crate::zones::MemberArea::Center,
                        crate::zones::MemberArea::RightSide,
                    ];
                    areas
                        .iter()
                        .find(|&&a| {
                            a != area
                                && !player.areas_locked_this_turn.contains(&a)
                                && player.stage.get_area(a).is_some()
                        })
                        .copied()
                };
                // Track the second vacated area for empty_area deployment
                let other_vacated = second_area.map(|a| a as usize);
                if let Some(area2) = second_area {
                    let existing_card_id = {
                        let player = game_state.active_player();
                        player.stage.get_area(area2)
                    };
                    if let Some(eid) = existing_card_id {
                        let player = game_state.active_player_mut();
                        let _ = player
                            .remove_member_from_stage_with_recycling(area2 as usize, &card_db);
                        player.waitroom.cards.push(eid);
                    }
                    game_state.record_baton_touch();
                    game_state.baton_touch_replaced_member_id =
                        Some(replaced_member_id.unwrap_or(-1));
                }
                game_state.last_vacated_stage_area = other_vacated;
            }
        }

        // Track area move for movement_condition "moves"
        eprintln!("[TRACK_MOVE] card_id={} player_id={}", card_id, player_id);
        game_state.last_area_move_card_id = Some(card_id);
        game_state.last_area_move_by_player = Some(player_id.clone());

        Self::trigger_debut_abilities(
            game_state,
            &player_id,
            &card_no,
            cost_paid,
            baton_touch_used,
        );
        Self::trigger_auto_abilities_for_player(game_state, &player_id);
        game_state.process_pending_auto_abilities(&player_id);
        game_state.recalculate_constants();

        if baton_touch_used {
            for area in [
                crate::zones::MemberArea::LeftSide,
                crate::zones::MemberArea::Center,
                crate::zones::MemberArea::RightSide,
            ] {
                let card_no = if let Some(card_id) = game_state.active_player().stage.get_area(area)
                {
                    if let Some(card) = game_state.card_database.get_card(card_id) {
                        let bt_card_id = card_id;
                        card.abilities
                            .iter()
                            .filter(|ability| {
                                ability
                                    .triggers
                                    .as_ref()
                                    .map_or(false, |t| t.contains(crate::triggers::BATON_TOUCH))
                            })
                            .map(|ability| {
                                (
                                    format!("{}_{}", card.card_no, ability.full_text),
                                    card.card_no.clone(),
                                    bt_card_id,
                                )
                            })
                            .collect()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };
                for (ability_id, card_no, bt_card_id) in card_no {
                    game_state.trigger_auto_ability(
                        ability_id,
                        crate::game_state::AbilityTrigger::Debut,
                        player_id.clone(),
                        Some(card_no),
                        Some(bt_card_id),
                    );
                }
            }
        }

        // Check the replaced member's auto abilities for movement triggers.
        if let Some(replaced_id) = replaced_member_id {
            Self::trigger_discard_auto_abilities(game_state, &player_id, replaced_id);
            game_state.process_pending_auto_abilities(&player_id);
        }

        Ok(())
    }

    /// Trigger auto abilities for a card that was placed in discard/waitroom from stage.
    /// Checks the card's auto abilities for discard/waitroom location conditions.
    pub fn trigger_discard_auto_abilities(
        game_state: &mut GameState,
        player_id: &str,
        card_id: i16,
    ) {
        let abilities: Vec<(String, String)> = game_state
            .card_database
            .get_card(card_id)
            .map(|card| {
                card.abilities
                    .iter()
                    .filter(|ability| {
                        ability.triggers.as_deref() == Some(crate::triggers::AUTO)
                            && ability
                                .effect
                                .as_ref()
                                .and_then(|e| e.condition.as_ref())
                                .map_or(false, |c| {
                                    c.location.as_deref() == Some("discard")
                                        || c.location.as_deref() == Some("waitroom")
                                })
                    })
                    .map(|ability| {
                        (
                            format!("{}_{}", card.card_no, ability.full_text),
                            card.card_no.clone(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        for (ability_id, card_no) in abilities {
            game_state.trigger_auto_ability(
                ability_id,
                crate::game_state::AbilityTrigger::Auto,
                player_id.to_string(),
                Some(card_no),
                Some(card_id),
            );
        }
    }

    pub fn setup_initial_energy(game_state: &mut GameState) {
        for _ in 0..3 {
            if let Some(card_id) = game_state.player1.energy_deck.draw() {
                let _ = game_state
                    .player1
                    .energy_zone
                    .add_card(card_id, &game_state.card_database);
            }
            if let Some(card_id) = game_state.player2.energy_deck.draw() {
                let _ = game_state
                    .player2
                    .energy_zone
                    .add_card(card_id, &game_state.card_database);
            }
        }
    }

    pub(crate) fn handle_rps_choice_p1(
        game_state: &mut GameState,
        choice: i32,
    ) -> Result<(), String> {
        game_state.player1_rps_choice = Some(choice);
        Self::resolve_rps_if_both_chosen(game_state)
    }
    pub(crate) fn handle_rps_choice_p2(
        game_state: &mut GameState,
        choice: i32,
    ) -> Result<(), String> {
        game_state.player2_rps_choice = Some(choice);
        Self::resolve_rps_if_both_chosen(game_state)
    }

    fn resolve_rps_if_both_chosen(game_state: &mut GameState) -> Result<(), String> {
        let p1_choice = match game_state.player1_rps_choice {
            Some(c) => c,
            None => return Ok(()),
        };
        let p2_choice = match game_state.player2_rps_choice {
            Some(c) => c,
            None => return Ok(()),
        };

        let rps_winner = match (p1_choice, p2_choice) {
            (0, 2) | (1, 0) | (2, 1) => 1,
            (2, 0) | (0, 1) | (1, 2) => 2,
            _ => {
                game_state.player1_rps_choice = None;
                game_state.player2_rps_choice = None;
                return Ok(());
            }
        };
        game_state.rps_winner = Some(rps_winner);
        game_state.current_phase = Phase::ChooseFirstAttacker;
        Ok(())
    }
}
