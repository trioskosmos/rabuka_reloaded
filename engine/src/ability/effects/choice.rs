//! The `choice` action - building the question.
//!
//! 24 of the 936 unique abilities emit `choice`. The answering side is
//! `ability/choice/`: this file asks, that one answers.


use super::super::resolver::AbilityResolver;
use super::super::types::{Choice, ChoiceRoute};

use crate::ability::queue::ConditionalChoice;
use crate::card::{AbilityEffect};

use crate::game_state::GameState;




#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};

impl AbilityResolver {

    pub(crate) fn execute_choice(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let co_binding = effect.choice_options_any();
        let choice_options = co_binding.as_ref();
        let ct_binding = effect.choice_type_any();
        let choice_type = ct_binding;
        let opt_binding = effect.options_any();
        let options = opt_binding.as_ref();
        let cm_binding = effect.choice_maker_any();
        let choice_maker = cm_binding;
        let heart_colors = effect.heart_colors_any();
        if !heart_colors.is_empty()
            && options.is_none()
            && choice_options.is_none()
            && choice_type.is_none()
        {
            log::debug!(
                "[CHOICE_HEART] colors={:?} count={} source={:?}",
                heart_colors,
                effect.count_or(1),
                effect.source_any()
            );
            self.execute_select_heart_color(
                gs,
                effect.count_or(1),
                heart_colors,
                effect.target_name(),
            );
            return Ok(());
        }
        // If a selection was already made (from a prior choice resolution),
        // execute the selected option's effect instead of creating another choice.
        if let Some(effect_options) = options {
            if let Some(entry) = gs.ability_queue.current_entry() {
                if let Some(ConditionalChoice::Str(ref cc)) = entry.conditional_choice {
                    if let Ok(idx) = cc.parse::<usize>() {
                        if idx < effect_options.len() {
                            let selected = &effect_options[idx];
                            return self.execute_effect(gs, selected);
                        }
                    }
                }
            }
        }
        if let Some(alt_options) = choice_options {
            if let Some(entry) = gs.ability_queue.current_entry() {
                if let Some(ConditionalChoice::Str(ref cc)) = entry.conditional_choice {
                    if alt_options.contains(cc) {
                        // String-based choice matched — nothing to execute, choice was just informational
                        return Ok(());
                    }
                }
            }
        }
        // Propagate parent choice's group_names to each child option when the
        // group_names is a selection filter (not a condition threshold).  We detect
        // this by checking for alternative_count_type — when set, the group_names is
        // used as a condition (e.g. "if X group is present, pick any number"), not
        // as a filter on the options themselves.  Fixes Kanon pb2-001-R's followup
        // choice where group_names=["Liella!"] was lost after option selection.
        //
        // BUT: if ANY option defines its own group_names, the parent's copy is
        // contextual (per-option scoping), not a shared filter — propagating it
        // would wrongly restrict sibling options (ド！ド！ド！ cl1-011-CL: option A
        // retrieves ANY member while option B is 『蓮ノ空』-only).
        let options_have_own_groups = options
            .map(|opts| opts.iter().any(|o| o.group_names_any().is_some()))
            .unwrap_or(false);
        // 「…のカードN枚につき、以下から1つを選ぶ」 (PL!-pb2-016-P＋) also carries
        // group_names, but there they scope HOW MANY TIMES the choice is offered, not
        // what each option may touch. Propagating them made option 1
        // (「自分のステージにいるメンバー1人をアクティブにする」) require a *lily white*
        // member, so it found no candidates and silently did nothing. The per-card
        // reading is the one in this codebase where the choice text carries につき, so
        // that is the discriminator — the other choice effects with group_names
        // (PL!S-bp7-003-P, PL!S-bp5-004-AR, PL!N-bp7-005-P, PL!HS-bp5-022-L, …) all
        // genuinely mean a filter on the options and still propagate.
        let should_propagate_group = effect.group_names_any().is_some()
            && effect.alternative_count_type_any().is_none()
            && effect.alternative_condition().is_none()
            && !options_have_own_groups
            && !effect.text.contains("につき");
        let propagated_options: Option<Vec<Box<AbilityEffect>>> = if should_propagate_group {
            options.map(|opts| {
                opts.iter()
                    .map(|opt| {
                        let mut p = opt.clone();
                        if p.group_names_any().is_none() {
                            p.set_group_names(effect.group_names_any().cloned().map(Box::new));
                        }
                        p
                    })
                    .collect()
            })
        } else {
            None
        };
        let conditional_choice_val = if let Some(ref opts) = propagated_options {
            Some(ConditionalChoice::Effects(opts.clone()))
        } else if let Some(opts) = options {
            Some(ConditionalChoice::Effects(opts.to_vec()))
        } else { choice_options.map(|opts| ConditionalChoice::Strings(opts.to_vec())) };
        if let Some(entry) = gs.ability_queue.current_entry_mut() {
            entry.choice_card_no = if options.is_some() {
                Some(ChoiceRoute::Choice)
            } else if choice_options.is_some() {
                Some(ChoiceRoute::ChoiceString)
            } else {
                Some(ChoiceRoute::Choice)
            };
            entry.conditional_choice = conditional_choice_val;

            // Set choice_player_id based on choice_maker
            if choice_maker == Some("opponent") {
                let current_player_id = entry.player_id.clone();
                let opponent_id = if current_player_id == "p1" {
                    crate::core::player::PlayerId::p2().clone()
                } else {
                    crate::core::player::PlayerId::p1().clone()
                };
                entry.choice_player_id = Some(opponent_id);
            } else {
                entry.choice_player_id = Some(entry.player_id.clone());
            }
        }
        if let Some(effect_options) = options {
            let opt_ja: Vec<String> = effect_options
                .iter()
                .map(|o| {
                    o.answers_any()
                        .as_ref()
                        .map(|a| a.join(", "))
                        .unwrap_or_else(|| o.text.to_string())
                })
                .collect();
            let description = opt_ja.join(" / ");
            // Per-option English for the "choice" option actions: generated
            // from each option effect so EN UIs don't show Japanese options.
            // Falls back to the JA text per option when generation is empty.
            let opt_en: Vec<String> = effect_options
                .iter()
                .zip(opt_ja.iter())
                .map(|(o, ja)| {
                    let en = crate::ability::describe::describe_effect_en(o);
                    if en.trim().is_empty() {
                        ja.clone()
                    } else {
                        en
                    }
                })
                .collect();
            log::debug!(
                "[CHOICE_BILINGUAL] options={} en={:?}",
                opt_en.len(),
                opt_en
            );
            self.awaiting.choice = Some(Choice::SelectTarget {
                target: "choice".to_string(),
                description: description.clone(),
                description_en: Some(opt_en.join(" / ")),
                description_ja: Some(format!("選択: {}", description)),
                allow_skip: effect.optional.unwrap_or(false),
                options: None,
            });
            // 「『X』のカード1枚につき、以下から1つを選ぶ」 — the choice is offered once per
            // qualifying card, so store how many FURTHER offerings are owed. The multiplier
            // is resolved BEFORE the entry is borrowed, and before any option runs, because
            // an earlier option can move cards into or out of the counted zone.
            let repeats = self.per_card_choice_repeats(gs, effect);
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.choice_effect_text = Some(effect.text.to_string());
                entry.choice_repeats_remaining = repeats;
            }
        } else if let Some(string_options) = choice_options {
            self.awaiting.choice = Some(Choice::SelectTarget {
                target: "choice_string".to_string(),
                description: format!("Choose one: {}", string_options.join(", ")),
                description_en: Some(format!("Choose one: {}", string_options.join(", "))),
                description_ja: Some(format!("選択: {}", string_options.join(", "))),
                allow_skip: effect.optional.unwrap_or(false),
                options: None,
            });
        } else if let Some(ct) = choice_type {
            self.awaiting.choice = Some(Choice::SelectTarget {
                target: "choice".to_string(),
                description: format!("Choose: {}", ct),
                description_en: Some(format!("Choose: {}", ct)),
                description_ja: Some(format!("選択: {}", ct)),
                allow_skip: effect.optional.unwrap_or(false),
                options: None,
            });
        }
        Ok(())
    }

    /// Further offerings owed for a 「…のカード1枚につき、以下から1つを選ぶ」 choice.
    ///
    /// Returns `count - 1`, because the choice about to be presented IS the first of the
    /// `count` offerings. A non-per-unit choice owes nothing, and so does one with no
    /// qualifying card at all (the choice still runs once — the player may decline it —
    /// but 「1枚につき」 gives no extra offerings).
    fn per_card_choice_repeats(&mut self, gs: &GameState, effect: &AbilityEffect) -> u8 {
        if !effect.per_unit_any().unwrap_or(false) {
            return 0;
        }
        let player = gs.resolve_target_player(effect.target_name());
        let filter = crate::ability::util::CardFilter::from_effect(effect);
        // The counted zone is `location`, with per_unit_type only naming the counter unit
        // (枚) — the same `location.or(per_unit_type)` precedence the score modifier uses.
        // Without the location a bare 枚 resolves to the hand, which would count the
        // player's hand instead of the 置き場 the clause names.
        let zone = effect.location_any().or(effect.per_unit_type_any());
        let qualifying = crate::ability::util::resolve_per_unit_count(
            true,
            zone,
            player,
            &gs.card_database,
            &filter,
            effect.heart_colors_any(),
            effect.state_any(),
            &gs.mods.orientation_modifiers,
            None,
        );
        let per = effect.per_unit_count_any().unwrap_or(1).max(1);
        let offerings = (qualifying / per).saturating_sub(1);
        log::debug!(
            "[CHOICE_PER_CARD] qualifying={} per={} offerings={} remaining={}",
            qualifying,
            per,
            offerings + 1,
            offerings
        );
        offerings
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