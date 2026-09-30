//! The pickers: `select`, `select_cards`, `select_number`,
//! `choose_target_player`, plus the hand reorder that follows a choice.
//!
//! 30 of the 936 unique abilities across those four action types. These ask
//! the question; `ability/choice/` answers it.
use crate::core::constants::CountCast;
use super::super::enums::{Zone};
use super::super::resolver::AbilityResolver;
use super::super::types::{Choice, ExecutionContext};

use crate::ability::queue::ConditionalChoice;
use crate::card::{AbilityEffect};

use crate::game_state::GameState;




#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

impl AbilityResolver {

    pub(crate) fn execute_select_effect(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        // Heart-color choices (「{{heart_01}}か{{heart_03}}か…のうち、1つを
        // 選ぶ」) are recognized ONLY via the parsed `heart_colors` field.
        // The previous effect-text substring check (`text.contains("heart_")`)
        // was dead weight: every heart-choice step in abilities.json carries
        // parsed heart_colors, while icon-bearing filter texts belong to other
        // actions that must NOT be routed here.
        let has_heart_colors = !effect.heart_colors_any().is_empty();
        log::debug!(
            "[SELECT_EFFECT] heart_colors={:?} source={:?} card_type={:?}",
            effect.heart_colors_any(),
            effect.source_any(),
            effect.card_type_any()
        );
        if effect.source_any().is_none()
            && effect.heart_colors_any().is_empty()
            && effect.or_card_types_any().is_none()
            && effect.characters_any().is_none_or(|v| v.is_empty())
            && effect.group_names_any().is_none()
        {
            return self.execute_area_select(gs, effect);
        }
        if effect.source_any().is_none()
            && effect.card_type_any().is_none()
            && has_heart_colors
        {
            let heart_colors = effect.heart_colors_any().to_vec();
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
                self.awaiting.choice.is_some()
            );
            return Ok(());
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
        self.awaiting.choice = Some(Choice::SelectHeartColor {
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
        self.awaiting.choice = Some(Choice::SelectTarget {
            target: "choice_string".to_string(),
            description: format!("Choose a number: {}", options_display),
            description_en: Some(format!("Choose a number: {}", options_display)),
            description_ja: Some(format!("数値を選択: {}", options_display_ja)),
            allow_skip: effect.optional.unwrap_or(false),
            options: Some(choice_options),
        });
        let pp = self.player_prefix(gs);
        if crate::core::runtime_flags::logging_enabled() {
            let act_name = gs
                .activating_card
                .map(|c| self.card_name(c))
                .unwrap_or_default();
            gs.push_rule_log_fmt(format_args!("{} {}: [[log_number_select]]", pp, act_name));
        }
        Ok(())
    }

    pub(crate) fn execute_area_select(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let target = effect.target_name();
        let player = gs.resolve_target_player(target);
        let activating_id = self.session.activating_card_id;

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

        self.awaiting.choice = Some(Choice::SelectTarget {
            target: "area_select".to_string(),
            description: "Choose an area".to_string(),
            description_en: Some("Choose an area".to_string()),
            description_ja: Some("エリアを選択".to_string()),
            allow_skip: effect.optional.unwrap_or(false),
            options: Some(valid),
        });
        Ok(())
    }

    pub(crate) fn execute_choose_target_player(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        self.owner.executing = Some(effect.clone());
        let options = effect
            .choice_options_any()
            .cloned()
            .unwrap_or_else(|| vec!["自分".to_string(), "相手".to_string()]);
        self.awaiting.choice = Some(Choice::SelectTarget {
            target: "self_or_opponent".to_string(),
            description: "Choose self or opponent".to_string(),
            description_en: Some("Choose self or opponent".to_string()),
            description_ja: Some("自分または相手を選択".to_string()),
            allow_skip: false,
            options: Some(options.to_vec()),
        });
        if crate::core::runtime_flags::logging_enabled() {
            let pp = self.player_prefix(gs);
            gs.push_rule_log_fmt(format_args!("{}: ターゲットプレイヤー選択", pp));
        }
        Ok(())
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
        let phase = self.selection.keep_shuffle_phase;
        if phase == 0 {
            self.selection.keep_shuffle_count = count;
            let player = gs.resolve_target_player_mut("self");
            self.selection.keep_shuffle_snapshots.clear();
            self.selection.keep_shuffle_snapshots
                .push(player.hand.cards.to_vec());
            let c = self.make_hand_selection_choice(gs, "self", count, effect);
            self.selection.keep_shuffle_phase = 1;
            self.awaiting.choice = Some(c);
            self.in_flight.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
            return Ok(());
        }
        if phase == 1 {
            // Phase 1 is now handled directly in choice.rs::handle_hand_selection
            // (moves self's non-selected under deck and prompts opponent).
            // This fallback only runs if choice.rs did not already advance the
            // phase (e.g. legacy path / direct re-entry). Guard against double
            // move by checking snapshot count.
            if self.selection.keep_shuffle_snapshots.len() == 1 {
                let snapshot = self.selection.keep_shuffle_snapshots[0].clone();
                self.move_non_selected_hand_to_deck_bottom(gs, "self", &snapshot);
                self.selection.keep_shuffle_selected.clear();
                let player = gs.resolve_target_player_mut("opponent");
                self.selection.keep_shuffle_snapshots
                    .push(player.hand.cards.to_vec());
                let c = self.make_hand_selection_choice(gs, "opponent", count, effect);
                self.selection.keep_shuffle_phase = 2;
                self.awaiting.choice = Some(c);
                self.in_flight.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
            }
            return Ok(());
        }
        // phase == 2: opponent's selection resolved.
        if self.selection.keep_shuffle_snapshots.len() >= 2 {
            let snapshot = self.selection.keep_shuffle_snapshots[1].clone();
            self.move_non_selected_hand_to_deck_bottom(gs, "opponent", &snapshot);
        }
        self.selection.keep_shuffle_phase = 0;
        self.selection.keep_shuffle_snapshots.clear();
        self.selection.keep_shuffle_selected.clear();
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
        let pick = count.min(hand_count.u8_count()).usize_count();
        crate::ability::types::Choice::select_cards(
            Zone::Hand.to_str(),
            pick,
            format!("Select up to {} card(s) to keep", count),
            true, // allow_skip — "まで" means up to N
        )
        .description_ja(Some(format!(
            "最大{}枚まで手札に残すカードを選択",
            count
        )))
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
        let kept_positions = self.selection.keep_shuffle_selected.to_vec();
        let player = gs.resolve_target_player_mut(player_target);
        // Remove the non-selected cards (the hand equals the snapshot here).
        for (idx, cid) in hand_snapshot.iter().enumerate() {
            if !kept_positions.contains(&idx.u8_count()) {
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
            .filter(|(idx, _)| !kept_positions.contains(&idx.u8_count()))
            .map(|(_, cid)| *cid)
            .collect();
        crate::rng::shuffle_slice(&mut to_move);
        for cid in to_move {
            player.main_deck.cards.push(cid);
        }
    }
}