use super::super::enums::Zone;
use super::super::resolver::AbilityResolver;
use super::super::types::{Choice, ExecutionContext};
use super::super::util;
use crate::ability_queue::ConditionalChoice;
use crate::card::{AbilityEffect, DistinctType};
use crate::game_state::GameState;
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use smallvec::SmallVec;

pub(crate) fn draw_cards_for_player(
    player: &mut crate::player::Player,
    count: u8,
    _source: &str,
    destination: &str,
    card_type_filter: Option<&str>,
    is_any_number: bool,
    _distinct: Option<DistinctType>,
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
            // Q104 / Rule 10.2.1: deck empty mid-draw → refresh from waitroom
            if player.main_deck.cards.is_empty() && !player.waitroom.cards.is_empty() {
                player.refresh();
                continue;
            }
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
    ) -> u8 {
        // Single source of truth is GameState::resolve_dynamic_count
        // (dynamic_count.rs) — the constant path calls it too. Passing the
        // resolver's transient step context keeps one definition of each
        // dynamic reference.
        gs.resolve_dynamic_count(
            dc,
            &self.moved_cards,
            &self.selected_cards,
            self.step_state.last_draw_count,
        )
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
            self.pending_choice = Some(crate::ability::types::Choice::SelectTarget {
                target: "pay_optional_cost:skip_optional_cost".to_string(),
                description: format!("Draw {} card(s)?", count),
                description_en: Some(format!("Draw {} card(s)?", count)),
                description_ja: Some(format!("{}枚ドローする？", count)),
                allow_skip: false,
                options: None,
            });
            return Ok(());
        }
        let draw_count = if let Some(ref dc) = effect.dynamic_count_any() {
            self.resolve_dynamic_count(gs, dc)
        } else if effect.count_any() == Some(0) {
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
                self.moved_cards.len() as u8
            } else if let Some(ref moved_cards) = gs.recently_moved_cards {
                moved_cards.len() as u8
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
            effect.destination.map(|z| z.as_str()).unwrap_or(Zone::Hand.to_str()),
            effect.card_type_any().map(|ct| ct.as_card_str()),
            effect.per_unit_any().unwrap_or(false),
            effect.per_unit_count_any().unwrap_or(1),
            effect.per_unit_type_any().as_deref(),
        )
    }

    pub(crate) fn execute_select_effect(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let has_heart_colors = !effect.heart_colors_any().is_empty();
        let has_heart_icons = effect.text.contains("heart_");
        log::debug!(
            "[SELECT_EFFECT] heart_colors={:?} has_icons={} source={:?} card_type={:?}",
            effect.heart_colors_any(),
            has_heart_icons,
            effect.source_any(),
            effect.card_type_any()
        );
        if effect.source_any().is_none()
            && effect.heart_colors_any().is_empty()
            && !has_heart_icons
            && effect.or_card_types_any().is_none()
            && effect.characters_any().is_none_or(|v| v.is_empty())
            && effect.group_names_any().is_none()
        {
            return self.execute_area_select(gs, effect);
        }
        if effect.source_any().is_none()
            && effect.card_type_any().is_none()
            && (has_heart_colors || has_heart_icons)
        {
            let heart_colors = if !effect.heart_colors_any().is_empty() {
                effect.heart_colors_any().to_vec()
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
        let source = if effect.card_type_any() == Some(&crate::card::CardType::Member)
            && effect.source_any().is_none()
        {
            Zone::Stage.to_str()
        } else {
            effect.source_or(Zone::Hand.to_str())
        };
        // C6 keep-N-shuffle-rest: both players keep up to N hand cards, shuffle
        // the rest under their own deck. Handled entirely here (per-player
        // selection + move), so the engine executes it exactly as written.
        if effect.keep_shuffle_under_any().unwrap_or(false) {
            return self.execute_both_hand_keep_shuffle_under(gs, effect);
        }
        self.execute_select(gs, source, effect)
    }

    /// C6: 自分と相手はそれぞれ手札のカードをN枚まで選び(keep)、選んだカード以外を
    /// シャッフルして自身のデッキの下に置く。
    ///
    /// Phase 0 → create self's hand selection choice (count N, max).
    /// Phase 1 → self's selection resolved: move self's non-selected hand cards
    ///           under self's deck (shuffled); create opponent's selection choice.
    /// Phase 2 → opponent's selection resolved: move opponent's non-selected
    ///           hand cards under opponent's deck (shuffled). Done.
    pub fn execute_both_hand_keep_shuffle_under(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let count = effect.count_or(1);
        let phase = self.keep_shuffle_under_phase;
        if phase == 0 {
            self.keep_shuffle_under_count = count;
            let player = gs.resolve_target_player_mut("self");
            self.keep_shuffle_under_snapshots.clear();
            self.keep_shuffle_under_snapshots
                .push(player.hand.cards.to_vec());
            let c = self.make_hand_selection_choice(gs, "self", count, effect);
            self.keep_shuffle_under_phase = 1;
            self.pending_choice = Some(c);
            self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
            return Ok(());
        }
        if phase == 1 {
            // Phase 1 is now handled directly in choice.rs::handle_hand_selection
            // (moves self's non-selected under deck and prompts opponent).
            // This fallback only runs if choice.rs did not already advance the
            // phase (e.g. legacy path / direct re-entry). Guard against double
            // move by checking snapshot count.
            if self.keep_shuffle_under_snapshots.len() == 1 {
                let snapshot = self.keep_shuffle_under_snapshots[0].clone();
                self.move_non_selected_hand_to_deck_bottom(gs, "self", &snapshot);
                self.keep_shuffle_selected.clear();
                let player = gs.resolve_target_player_mut("opponent");
                self.keep_shuffle_under_snapshots
                    .push(player.hand.cards.to_vec());
                let c = self.make_hand_selection_choice(gs, "opponent", count, effect);
                self.keep_shuffle_under_phase = 2;
                self.pending_choice = Some(c);
                self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
            }
            return Ok(());
        }
        // phase == 2: opponent's selection resolved.
        if self.keep_shuffle_under_snapshots.len() >= 2 {
            let snapshot = self.keep_shuffle_under_snapshots[1].clone();
            self.move_non_selected_hand_to_deck_bottom(gs, "opponent", &snapshot);
        }
        self.keep_shuffle_under_phase = 0;
        self.keep_shuffle_under_snapshots.clear();
        self.keep_shuffle_selected.clear();
        Ok(())
    }

    fn make_hand_selection_choice(
        &mut self,
        gs: &mut GameState,
        player_target: &str,
        count: u8,
        _effect: &AbilityEffect,
    ) -> crate::ability::types::Choice {
        let player = gs.resolve_target_player_mut(player_target);
        let hand_count = player.hand.cards.len();
        let pick = count.min(hand_count as u8) as usize;
        crate::ability::types::Choice::select_cards(
            Zone::Hand.to_str(),
            pick,
            format!("Select up to {} card(s) to keep", count),
            true, // allow_skip — "まで" means up to N
        )
        .target_player_id(Some(player_target.to_string()))
        .build()
    }

    /// Move a player's hand cards that are NOT in `self.selected_cards` (their
    /// kept selection) under their own deck, shuffled. Kept cards stay in hand.
    pub(crate) fn move_non_selected_hand_to_deck_bottom(
        &mut self,
        gs: &mut GameState,
        player_target: &str,
        hand_snapshot: &[i16],
    ) {
        // hand_snapshot is the player's hand at selection time (unchanged since).
        // Keep snapshot[kept_pos] in hand; move the other positions under deck.
        let kept_positions = self.keep_shuffle_selected.to_vec();
        let player = gs.resolve_target_player_mut(player_target);
        // Remove the non-selected cards (the hand equals the snapshot here).
        for (idx, cid) in hand_snapshot.iter().enumerate() {
            if !kept_positions.contains(&(idx as u8)) {
                if let Some(pos) = player.hand.cards.iter().position(|c| c == cid) {
                    player.hand.cards.remove(pos);
                }
            }
        }
        // "シャッフルし、自身のデッキの下に置く" — shuffle the moved cards, then
        // place them at the bottom of the deck in the shuffled order.
        let mut to_move: Vec<i16> = hand_snapshot
            .iter()
            .enumerate()
            .filter(|(idx, _)| !kept_positions.contains(&(*idx as u8)))
            .map(|(_, cid)| *cid)
            .collect();
        crate::rng::shuffle_slice(&mut to_move);
        for cid in to_move {
            player.main_deck.cards.push(cid);
        }
    }

    pub fn execute_draw(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        count: u8,
        target: &str,
        source: &str,
        destination: &str,
        card_type: Option<&str>,
        per_unit: bool,
        per_unit_count: u8,
        per_unit_type: Option<&str>,
    ) -> Result<(), String> {
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        let card_db = self.card_db();
        let is_any_number = effect.any_number_any().unwrap_or(false);
        let is_distinct = effect.distinct_any();
        let is_self_target = effect.is_self_target();

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
            self.step_state.last_draw_count = count;
            return Ok(());
        }

        let activating_id = gs.activating_card;
        let orientation_modifiers = gs.mods.orientation_modifiers.clone();
        let (recently_moved_snapshot, entry_trigger_snapshot, last_discard_count) = (
            gs.recently_moved_cards.clone(),
            gs.entry_trigger_moved_cards(),
            gs.mods.last_cost_discard_count,
        );
        let player = gs.resolve_target_player_mut(target);

        let final_count = if per_unit {
            let matching_count = if Some("discard") == per_unit_type {
                let filter = util::CardFilter::from_effect(effect);
                let tracked = recently_moved_snapshot
                    .as_ref()
                    .or(entry_trigger_snapshot.as_ref());
                let discard_count = util::resolve_discard_per_unit_count(
                    tracked,
                    last_discard_count,
                    &card_db,
                    &filter,
                );
                discard_count / per_unit_count.max(1)
            } else {
                let multiplier = util::calculate_per_unit_multiplier(
                    per_unit,
                    per_unit_type,
                    player,
                    &orientation_modifiers,
                    effect.state_any().as_deref(),
                );
                multiplier * per_unit_count
            };
            count * matching_count
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
                self.step_state.last_draw_count = final_count;
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
                description_en: Some(format!("Choose how many cards to draw (0-{})", available)),
                description_ja: Some(format!("ドローする枚数を選択（0〜{}枚）", available)),
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
                let mut cards: SmallVec<[i16; 8]> = (0..final_count as usize)
                    .filter_map(|_| player.waitroom.cards.pop())
                    .collect();
                if let Some(distinct) = is_distinct {
                    cards = util::apply_distinct_filter(&cards, Some(distinct), &card_db).into();
                }
                for card in cards {
                    util::place_card_in_zone(player, card, destination, None, false, 1);
                }
            }
            _ => {
                log::debug!("Draw from source '{}' not yet implemented", source);
            }
        }
        self.step_state.last_draw_count = final_count;
        let dst = if destination.is_empty() {
            "hand"
        } else {
            destination
        };
        gs.push_rule_log(format!(
            "{} {}: [[log_draw:n={},from=zone_{},to=zone_{}]]",
            pp, act_name, final_count, source, dst
        ));
        Ok(())
    }

    pub fn execute_draw_until_count(&mut self, gs: &mut GameState, effect: &AbilityEffect) {
        let target_count: u8 = effect.target_count_any().unwrap_or(0) as u8;
        let target = effect.target_name();
        let destination = effect.destination.map(|z| z.as_str()).unwrap_or(Zone::Hand.to_str());
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
            to_draw as u8,
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
        count: u8,
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
                entry.conditional_choice = Some(ConditionalChoice::Str(unique_colors[0].clone()));
            }
            return;
        }
        self.pending_choice = Some(Choice::SelectHeartColor {
            count: count as usize,
            options: unique_colors,
            description: "Choose a heart color".to_string(),
            description_en: Some("Choose a heart color".to_string()),
            description_ja: Some("ハートの色を選択".to_string()),
        });
    }

    pub(crate) fn execute_select_number(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let max_cost = gs
            .card_database
            .cards
            .values()
            .filter_map(|c| c.cost)
            .max()
            .unwrap_or(10);
        let mut options: Vec<String> = (1..=max_cost).map(|n| n.to_string()).collect();
        options.push("67".to_string());
        let options_display = if options.len() > 10 {
            format!(
                "{}~{}",
                options.first().unwrap_or(&"1".to_string()),
                options.last().unwrap_or(&"67".to_string())
            )
        } else {
            options.join(", ")
        };
        let options_display_ja = if options.len() > 10 {
            format!(
                "{}〜{}",
                options.first().unwrap_or(&"1".to_string()),
                options.last().unwrap_or(&"67".to_string())
            )
        } else {
            options.join(", ")
        };
        let choice_options = options.clone();
        if let Some(entry) = gs.ability_queue.current_entry_mut() {
            entry.choice_card_no = None;
            entry.conditional_choice = Some(ConditionalChoice::Strings(options));
        }
        self.pending_choice = Some(Choice::SelectTarget {
            target: "choice_string".to_string(),
            description: format!("Choose a number: {}", options_display),
            description_en: Some(format!("Choose a number: {}", options_display)),
            description_ja: Some(format!("数値を選択: {}", options_display_ja)),
            allow_skip: effect.optional.unwrap_or(false),
            options: Some(choice_options),
        });
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.rule_log
            .push(format!("{} {}: [[log_number_select]]", pp, act_name));
        Ok(())
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
            description_en: Some("Choose an area".to_string()),
            description_ja: Some("エリアを選択".to_string()),
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
        count: u8,
        heart_colors: &[String],
        heart_selection: bool,
    ) -> Result<Option<String>, String> {
        if resource != "heart" && resource != "ハート" {
            return Ok(None);
        }
        if heart_colors.is_empty() && !heart_selection && effect.heart_type_any().is_none() {
            return Ok(None);
        }
        let colors: Vec<String> = if let Some(ref ht) = effect.heart_type_any() {
            vec![ht.to_string()]
        } else if !heart_colors.is_empty() {
            heart_colors.to_vec()
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
        } else if !heart_selection
            && unique_colors.len() > 1
            && count as usize >= unique_colors.len()
        {
            // Multiple fixed heart colors with matching or exceeding count.
            // Don't create a choice — caller will distribute the count across all colors.
            Ok(None)
        } else {
            self.pending_choice = Some(Choice::SelectHeartColor {
                count: count as usize,
                options: unique_colors,
                description: "Choose a heart color".to_string(),
                description_en: Some("Choose a heart color".to_string()),
                description_ja: Some("ハートの色を選択".to_string()),
            });
            Ok(None)
        }
    }

    /// Build EffectData for a single-card resource grant (blade or heart).
    pub(crate) fn make_card_effect_data(
        card_id: i16,
        amount: i16,
        color: Option<&str>,
    ) -> crate::core::types::EffectData {
        crate::core::types::EffectData::SingleCard {
            card_id,
            amount,
            color: color.map(|c| c.to_string()),
        }
    }
}
