use super::super::enums::Zone;
use super::super::resolver::AbilityResolver;
use super::super::types::{Choice, ExecutionContext};
use super::super::util;
use crate::card::AbilityEffect;
use crate::game_state::GameState;

pub(crate) fn draw_cards_for_player(
    player: &mut crate::player::Player,
    count: u32,
    _source: &str,
    destination: &str,
    card_type_filter: Option<&str>,
    is_any_number: bool,
    _distinct: Option<&str>,
    card_db: &crate::card::CardDatabase,
    self_target_id: Option<i16>,
) -> Result<(), String> {
    if is_any_number {
        return Ok(());
    }
    let mut drawn = 0;
    while drawn < count as usize {
        if let Some(card) = player.main_deck.draw() {
            let matches_type = util::CardFilter::new()
                .card_type_opt(card_type_filter)
                .exclude_self_opt(self_target_id)
                .matches_card(card_db, card);
            if matches_type {
                match Zone::from_str(destination) {
                    Some(
                        Zone::Hand
                        | Zone::Discard
                        | Zone::DeckTop
                        | Zone::DeckBottom
                        | Zone::Deck
                        | Zone::Energy
                        | Zone::LiveCardZone
                        | Zone::SuccessLiveZone
                        | Zone::Stage,
                    ) => {
                        util::place_card_in_zone(player, card, destination, None, false, 1);
                    }
                    _ => {
                        player.hand.add_card(card);
                    }
                }
                drawn += 1;
            } else {
                player.main_deck.cards.push(card);
            }
        } else {
            break;
        }
    }
    Ok(())
}

impl AbilityResolver {
    pub(crate) fn resolve_dynamic_count(
        &self,
        gs: &mut GameState,
        dc: &crate::card::DynamicCount,
    ) -> u32 {
        let get_revealed_count = |gs: &crate::game_state::GameState| {
            let cheer = gs.cheer_revealed_cards();
            if !cheer.is_empty() {
                cheer.len() as u32
            } else {
                gs.revealed_cards.len() as u32
            }
        };

        let reference_text = dc.reference.as_deref().or(dc.base_reference.as_deref());

        let mut count = match reference_text {
            Some("previous_moved_cards") | Some("previous_move") => {
                if !self.moved_cards.is_empty() {
                    self.moved_cards.len() as u32
                } else if let Some(ref moved_cards) = gs.recently_moved_cards {
                    moved_cards.len() as u32
                } else {
                    gs.mods.last_cost_discard_count
                }
            }
            Some("previous_draw") => {
                let last_draw = self.step_state.last_draw_count;
                if last_draw > 0 {
                    last_draw
                } else if let Some(ref moved_cards) = gs.recently_moved_cards {
                    moved_cards.len() as u32
                } else {
                    0
                }
            }
            Some("revealed_cards") | Some("previous_reveal") => get_revealed_count(gs),
            Some("unit_count") => {
                let player = gs.resolve_target_player("self");
                player.stage.stage.iter().filter(|&&c| c != -1).count() as u32
            }
            Some("its_difference") | Some("その差") => {
                let self_score: u32 = gs
                    .player1
                    .success_live_card_zone
                    .cards
                    .iter()
                    .filter_map(|&id| gs.card_database.get_card(id).and_then(|c| c.score))
                    .sum();
                let opp_score: u32 = gs
                    .player2
                    .success_live_card_zone
                    .cards
                    .iter()
                    .filter_map(|&id| gs.card_database.get_card(id).and_then(|c| c.score))
                    .sum();
                self_score.abs_diff(opp_score)
            }
            Some(reference) if reference.contains("これにより控え室に置いた数") => {
                if let Some(ref moved) = gs.recently_moved_cards {
                    moved.len() as u32
                } else {
                    self.moved_cards.len() as u32
                }
            }
            Some(reference) if reference.contains("合計スコア") => {
                let player = gs.resolve_target_player("self");
                player
                    .success_live_card_zone
                    .cards
                    .iter()
                    .filter_map(|&id| gs.card_database.get_card(id).and_then(|c| c.score))
                    .sum::<u32>()
            }
            Some(reference) if reference.contains("ステージ") && reference.contains("メンバー") =>
            {
                let target = if reference.contains("相手") || reference.contains("opponent") {
                    "opponent"
                } else {
                    "self"
                };
                let player = gs.resolve_target_player(target);
                player.stage.stage.iter().filter(|&&c| c != -1).count() as u32
            }
            _ => match dc.count_type.as_str() {
                "revealed_cards" => get_revealed_count(gs),
                _ => 0,
            },
        };
        if let Some(ref calculation) = dc.calculation {
            if calculation == "add" {
                count += dc.calculation_value.unwrap_or(0);
            }
        }
        count
    }
    pub(crate) fn execute_draw_wrapper(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        // Optional draw: create a yes/no choice instead of drawing unconditionally.
        // The sequential compound handler (compound.rs) saves remaining actions when
        // pending_choice is set, and skips them if the optional action is declined.
        if effect.optional.unwrap_or(false) {
            let count = effect.count_or(1);
            self.pending_choice = Some(crate::ability::types::Choice::select_target(
                "pay_optional_cost:skip_optional_cost",
                format!("Draw {} card(s)?", count),
                false,
            ));
            return Ok(());
        }
        let draw_count = if let Some(ref dc) = effect.dynamic_count {
            self.resolve_dynamic_count(gs, dc)
        } else if effect.count == Some(0) {
            log::debug!("[DRAW_ZERO] self.moved_cards={:?}", self.moved_cards);
            log::debug!(
                "[DRAW_ZERO] gs.recently_moved_cards={:?}",
                gs.recently_moved_cards
            );
            log::debug!(
                "[DRAW_ZERO] last_cost_discard_count={}",
                gs.mods.last_cost_discard_count
            );
            if !self.moved_cards.is_empty() {
                self.moved_cards.len() as u32
            } else if let Some(ref moved_cards) = gs.recently_moved_cards {
                moved_cards.len() as u32
            } else {
                gs.mods.last_cost_discard_count
            }
        } else {
            effect.count_or(1)
        };
        self.execute_draw(
            gs,
            effect,
            draw_count,
            effect.target_name(),
            effect.source_or(Zone::Deck.to_str()),
            effect.destination.as_deref().unwrap_or(Zone::Hand.to_str()),
            effect.card_type.as_deref(),
            effect.per_unit.unwrap_or(false),
            effect.per_unit_count.unwrap_or(1),
            effect.per_unit_type.as_deref(),
        )
    }

    pub(crate) fn execute_select_effect(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let has_heart_colors = !effect.heart_colors.is_empty();
        let has_heart_icons = effect.text.contains("heart_");
        log::debug!(
            "[SELECT_EFFECT] heart_colors={:?} has_icons={} source={:?} card_type={:?}",
            effect.heart_colors,
            has_heart_icons,
            effect.source,
            effect.card_type
        );
        if effect.source.is_none()
            && effect.heart_colors.is_empty()
            && !has_heart_icons
            && effect.or_card_types.is_none()
            && effect.characters.as_ref().is_none_or(|v| v.is_empty())
            && effect.group_names.is_none()
        {
            return self.execute_area_select(gs, effect);
        }
        if effect.source.is_none()
            && effect.card_type.is_none()
            && (has_heart_colors || has_heart_icons)
        {
            let heart_colors = if !effect.heart_colors.is_empty() {
                effect.heart_colors.clone()
            } else {
                crate::ability::util::extract_heart_colors_from_text(&effect.text)
            };
            if !heart_colors.is_empty() {
                log::debug!(
                    "[SELECT_EFFECT] calling execute_select_heart_color with colors={:?}",
                    heart_colors
                );
                self.execute_select_heart_color(
                    gs,
                    effect.count_or(1),
                    &heart_colors,
                    effect.target_name(),
                );
                log::debug!(
                    "[SELECT_EFFECT] pending_choice={:?}",
                    self.pending_choice.is_some()
                );
                return Ok(());
            }
        }
        let source = if effect.card_type.as_deref() == Some("member_card") {
            Zone::Stage.to_str()
        } else {
            effect.source_or(Zone::Hand.to_str())
        };
        self.execute_select(
            gs,
            source,
            effect.count_or(1),
            effect.target_name(),
            effect.card_type.as_deref(),
            effect.distinct.as_deref(),
            &effect.heart_colors,
            effect.or_card_types.clone(),
            effect.exclude_selected.unwrap_or(false),
            effect.characters.clone(),
            effect.group_names.clone(),
            effect.exclude_self,
        )
    }

    pub fn execute_draw(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        count: u32,
        target: &str,
        source: &str,
        destination: &str,
        card_type: Option<&str>,
        per_unit: bool,
        per_unit_count: u32,
        per_unit_type: Option<&str>,
    ) -> Result<(), String> {
        let card_db = self.card_db();
        let is_any_number = effect.any_number.unwrap_or(false);
        let is_distinct = effect.distinct.as_deref();
        let is_self_target = effect.self_target.unwrap_or(false);

        if target == "both" {
            for player in [&mut gs.player1, &mut gs.player2] {
                draw_cards_for_player(
                    player,
                    count,
                    source,
                    destination,
                    card_type,
                    is_any_number,
                    is_distinct,
                    &card_db,
                    None,
                )?;
            }
            return Ok(());
        }

        let activating_id = gs.activating_card;
        let orientation_modifiers = gs.mods.orientation_modifiers.clone();
        let player = gs.resolve_target_player_mut(target);

        let final_count = if per_unit {
            let multiplier = util::calculate_per_unit_multiplier(
                per_unit,
                per_unit_type,
                player,
                &orientation_modifiers,
                effect.state.as_deref(),
            );
            count * multiplier * per_unit_count
        } else {
            count
        };

        if is_self_target {
            if let Some(activating_id) = activating_id {
                if !player.stage.stage.contains(&activating_id) {
                    return Err(
                        "Cannot draw with self_target: activating card not on target's stage"
                            .to_string(),
                    );
                }
                draw_cards_for_player(
                    player,
                    final_count,
                    source,
                    destination,
                    card_type,
                    is_any_number,
                    is_distinct,
                    &card_db,
                    Some(activating_id),
                )?;
                return Ok(());
            }
        }

        if is_any_number {
            let available = util::get_zone_card_count(player, source);
            if available == 0 {
                return Ok(());
            }
            self.pending_choice = Some(Choice::SelectTarget {
                target: "draw_any_number".to_string(),
                description: format!("Choose how many cards to draw (0-{})", available),
                allow_skip: effect.optional.unwrap_or(false),
                options: None,
            });
            self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
            return Ok(());
        }

        match Zone::from_str(source) {
            Some(Zone::Deck | Zone::DeckTop) => {
                if let Some(distinct) = is_distinct {
                    let mut drawn: Vec<i16> = Vec::new();
                    let mut attempts = 0;
                    let max_attempts = player.main_deck.cards.len();
                    while drawn.len() < final_count as usize && attempts < max_attempts {
                        attempts += 1;
                        if let Some(card) = player.main_deck.draw() {
                            let matches_type = util::CardFilter::new()
                                .card_type_opt(card_type)
                                .matches_card(&card_db, card);
                            if matches_type {
                                drawn.push(card);
                            } else {
                                player.main_deck.cards.push(card);
                            }
                        } else {
                            break;
                        }
                    }
                    let drawn_distinct =
                        util::apply_distinct_filter(&drawn, Some(distinct), &card_db);
                    for card in drawn_distinct {
                        util::place_card_in_zone(player, card, destination, None, false, 1);
                    }
                } else {
                    draw_cards_for_player(
                        player,
                        final_count,
                        source,
                        destination,
                        card_type,
                        is_any_number,
                        is_distinct,
                        &card_db,
                        None,
                    )?;
                }
            }
            Some(Zone::Discard) => {
                let mut cards: Vec<i16> = (0..final_count as usize)
                    .filter_map(|_| player.waitroom.cards.pop())
                    .collect();
                if let Some(distinct) = is_distinct {
                    cards = util::apply_distinct_filter(&cards, Some(distinct), &card_db);
                }
                for card in cards {
                    util::place_card_in_zone(player, card, destination, None, false, 1);
                }
            }
            _ => {
                log::debug!("Draw from source '{}' not yet implemented", source);
            }
        }
        Ok(())
    }

    pub fn execute_draw_until_count(
        &mut self,
        gs: &mut GameState,
        target_count: u32,
        target: &str,
        destination: &str,
    ) {
        let player = gs.resolve_target_player_mut(target);
        let current_count = match Zone::from_str(destination) {
            Some(Zone::Hand) => player.hand.len(),
            _ => {
                return;
            }
        };
        let to_draw = (target_count as usize).saturating_sub(current_count);
        let _ = self.execute_draw(
            gs,
            &AbilityEffect::default(),
            to_draw as u32,
            target,
            Zone::Deck.to_str(),
            destination,
            None,
            false,
            1,
            None,
        );
    }

    pub(crate) fn execute_select_heart_color(
        &mut self,
        gs: &mut GameState,
        count: u32,
        heart_colors: &[String],
        _target: &str,
    ) {
        let mut unique_colors: Vec<String> = Vec::new();
        for c in heart_colors {
            if !unique_colors.contains(c) {
                unique_colors.push(c.clone());
            }
        }
        if unique_colors.len() == 1 {
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.conditional_choice = Some(unique_colors[0].clone());
            }
            return;
        }
        self.pending_choice = Some(Choice::SelectHeartColor {
            count: count as usize,
            options: unique_colors,
            description: "Choose a heart color".to_string(),
        });
    }

    pub(crate) fn execute_area_select(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let target = effect.target_name();
        let player = gs.resolve_target_player(target);
        let activating_id = self.activating_card_id;

        let position_names = ["left", "center", "right"];
        let mut valid = Vec::new();

        for (i, pos_name) in position_names.iter().enumerate() {
            // Skip if this is the activating card's current position
            if let Some(aid) = activating_id {
                if player.stage.stage[i] == aid {
                    continue;
                }
            }
            valid.push(pos_name.to_string());
        }

        if valid.is_empty() {
            return Ok(());
        }

        self.pending_choice = Some(Choice::SelectTarget {
            target: "area_select".to_string(),
            description: "Choose an area".to_string(),
            allow_skip: effect.optional.unwrap_or(false),
            options: Some(valid),
        });
        Ok(())
    }

    /// Resolve heart color for gain_resource. Returns Ok(Some(color)) if fixed, Ok(None) if choice was set or not a heart resource.
    pub(crate) fn resolve_gain_heart_color(
        &mut self,
        _gs: &mut GameState,
        effect: &AbilityEffect,
        resource: &str,
        count: u32,
        heart_colors: &[String],
        heart_selection: bool,
    ) -> Result<Option<String>, String> {
        if resource != "heart" && resource != "ハート" {
            return Ok(None);
        }
        if heart_colors.is_empty() && !heart_selection && effect.heart_type.is_none() {
            return Ok(None);
        }
        let colors: Vec<String> = if !heart_colors.is_empty() {
            heart_colors.to_vec()
        } else if let Some(ref ht) = effect.heart_type {
            vec![ht.clone()]
        } else {
            vec![
                "heart01".into(),
                "heart02".into(),
                "heart03".into(),
                "heart04".into(),
                "heart05".into(),
                "heart06".into(),
            ]
        };
        let mut unique_colors: Vec<String> = Vec::new();
        for c in colors {
            if !unique_colors.contains(&c) {
                unique_colors.push(c);
            }
        }
        if unique_colors.len() == 1 && !heart_selection {
            Ok(Some(unique_colors[0].clone()))
        } else {
            self.pending_choice = Some(Choice::SelectHeartColor {
                count: count as usize,
                options: unique_colors,
                description: "Choose a heart color".to_string(),
            });
            Ok(None)
        }
    }

    /// Build serde_json effect_data for a single-card resource grant (blade or heart).
    pub(crate) fn make_card_effect_data(
        card_id: i16,
        amount: i32,
        color: Option<&str>,
    ) -> serde_json::Value {
        let mut data = serde_json::Map::new();
        data.insert("card_id".into(), serde_json::Value::Number(card_id.into()));
        data.insert("amount".into(), serde_json::Value::Number(amount.into()));
        if let Some(c) = color {
            data.insert("color".into(), serde_json::Value::String(c.to_string()));
        }
        serde_json::Value::Object(data)
    }
}
