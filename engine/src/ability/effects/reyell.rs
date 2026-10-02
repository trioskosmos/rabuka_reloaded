//! `re_yell`, `perform_yell` and `modify_yell_count`.
//!
//! Named for the `re_yell` wire string in `cards/abilities.json`, which is
//! the action this file is chiefly about. 1 of the 936 unique abilities
//! emits `modify_yell_count`; the other two are the yell step the
//! `live_start` trigger runs.

use super::super::resolver::AbilityResolver;



use crate::card::{AbilityEffect};

use crate::game_state::GameState;




#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

impl AbilityResolver {

    pub(crate) fn execute_re_yell(&mut self, gs: &mut GameState, effect: &AbilityEffect) {
        let lose_blade_hearts = effect.lose_blade_hearts_any().unwrap_or(false);
        let target = effect.target_name();
        log::debug!(target: "rabuka_engine::ability::effects::misc","re_yell: lose_blade_hearts={}", lose_blade_hearts);
        if lose_blade_hearts {
            // Collect card IDs to clear first (avoid borrow conflict with gs.mods)
            let cids: Vec<i16> = {
                let player = gs.resolve_target_player(target);
                player
                    .stage
                    .stage
                    .iter()
                    .copied()
                    .filter(|&id| id != -1)
                    .collect()
            };
            for cid in cids {
                gs.mods.clear_all_for_card(cid);
            }
        }
        // Clear the old yell cards so perform_yell's new cards replace them.
        // initial_yell_revealed_cards was already saved in the phase code before
        // auto abilities fired, so it still contains the full initial yell list.
        gs.clear_revealed_cards();
        gs.re_yell_occurred = true;
        gs.prohibition_effects.push("re_yell".to_string());
        let pp = self.player_prefix(gs);
        if crate::core::runtime_flags::logging_enabled() {
            let act_name = gs
                .activating_card
                .map(|c| self.card_name(c))
                .unwrap_or_default();
            gs.push_rule_log_fmt(format_args!("{} {}: [[log_re_yell]]", pp, act_name));
        }
    }

    /// Perform N additional yells.
    /// A yell reveals cards from deck top until a live card is found.
    /// Perform an actual yell: draw total_blade cards from the player's deck
    /// and add them to revealed_cards. The yell count is the number of times
    /// to repeat this draw-and-reveal process (calculated from per_unit for
    /// MIRAI TICKET's "for every 5 cost, perform 1 additional yell").
    pub(crate) fn execute_perform_yell(&mut self, gs: &mut GameState, effect: &AbilityEffect) {
        let count: u8 = if effect.per_unit_any().unwrap_or(false) {
            // per_unit with per_unit_source = "previous_moved_cards":
            // sum costs of cards moved by the preceding action,
            // divide by per_unit_count, cap at repeat_limit.
            let total_cost: u8 = self
                .selection.moved_cards
                .iter()
                .filter_map(|&cid| gs.card_database.get_card(cid).and_then(|c| c.cost))
                .sum();
            // Clamped for the same reason as the other per-unit divisions: an
            // absent field defaults to 1, but a decoded `per_unit_count: 0` reached
            // the division and panicked.
            let divisor = effect.per_unit_count_any().unwrap_or(1).max(1);
            let mut c = total_cost / divisor;
            if let Some(cap) = effect.repeat_limit_any() {
                c = c.min(cap);
            }
            c
        } else if let Some(dc) = effect.dynamic_count_any() {
            self.resolve_dynamic_count(gs, dc)
        } else {
            effect.count_or(1)
        };
        let target = effect.target_name();
        let card_db = gs.card_database.clone();
        let bm = gs.mods.blade_modifiers.clone();
        let om = gs.mods.orientation_modifiers.clone();
        // set_blade_type recoloring for the performing player, identical to
        // the primary yell path in player_perform_live.
        let override_color = {
            let btm = &gs.mods.blade_type_modifiers;
            let performer = gs.resolve_target_player(target);
            (0..3)
                .filter_map(|i| {
                    let cid = performer.stage.stage[i];
                    if cid == -1 {
                        None
                    } else {
                        btm.get(&cid).copied().map(crate::turn::live::blade_color_to_heart)
                    }
                })
                .next()
        };
        let mut all_drawn: Vec<i16> = Vec::new();
        for _ in 0..count {
            let total_blade = {
                let player = gs.resolve_target_player_mut(target);
                let tb = player.stage.total_blades(&card_db, &bm, &om, false);
                let mut drawn: Vec<i16> = Vec::new();
                // Q104 / Rule 10.2.1: refresh from waitroom mid-draw
                for _ in 0..tb {
                    if player.main_deck.cards.is_empty() && !player.waitroom.cards.is_empty() {
                        player.refresh();
                    }
                    if let Some(cid) = player.main_deck.draw() {
                        drawn.push(cid);
                    }
                }
                drawn
            };
            let reyell_source = gs.current_ability_source_card_id();
            let reyell_owner = crate::ability::util::target_player_index(
                target,
                gs.ability_master_id(),
            );
            for cid in &total_blade {
                gs.push_revealed_card(*cid, reyell_source, false, reyell_owner, "re_yell");
            }
            all_drawn.extend(total_blade);
        }
        gs.re_yell_revealed_cards = gs.revealed_cards.clone();

        // Rule 8.3.12 / 8.3.15.1.1: process the re-yelled cards' blade-heart
        // and special-heart icons through the SAME shared helper as the
        // primary yell, then execute their draws immediately ("エールをすべ
        // て行った後", rule 8.3.12.1). The tallies are stashed for the phase
        // code, which applies them before its success check — it may already
        // have computed its original yell data because this ability can pause
        // on the optional discard choice.
        {
            let mut yell_cards: Vec<crate::core::types::YellCardResult> =
                Vec::with_capacity(all_drawn.len());
            let mut total_hearts = [0u8; 8];
            let mut note_icons = 0u8;
            let mut draw_total = 0u8;
            let mut card_nos: Vec<&str> = Vec::with_capacity(all_drawn.len());
            let mut scratch =
                crate::card::BaseHeart { hearts: crate::card::HeartMap::new() };
            for cid in &all_drawn {
                if let Some(card) = gs.card_database.get_card(*cid) {
                    let outcome = crate::turn::live::process_yell_revealed_card_icons(
                        card,
                        override_color,
                        &mut scratch,
                        &mut total_hearts,
                        &mut note_icons,
                    );
                    draw_total += outcome.draw_icons;
                    card_nos.push(card.card_no.as_ref());
                    yell_cards.push(crate::core::types::YellCardResult {
                        card_id: *cid,
                        blade_hearts: outcome.blade_hearts,
                        note_icons: outcome.note_icons,
                        draw_icons: outcome.draw_icons,
                        card_no: card.card_no.to_string().into(),
                    });
                }
            }
            log::debug!(target: "rabuka_engine::ability::effects::misc",
                "[REYELL_REBUILD] n={} cards={:?} draws={} scores={}",
                all_drawn.len(),
                card_nos,
                draw_total,
                note_icons
            );
            // Rule 8.3.12.1: each re-yelled draw icon draws 1 card.
            {
                let player = gs.resolve_target_player_mut(target);
                // Q104 / Rule 10.2.1: refresh from waitroom when deck runs out.
                for _ in 0..draw_total {
                    if player.main_deck.cards.is_empty() && !player.waitroom.cards.is_empty() {
                        player.refresh();
                    }
                    if let Some(new_card) = player.main_deck.draw() {
                        player.hand.add_card(new_card);
                    }
                }
            }
            gs.pending_reyell_rebuild = Some(crate::types::PendingReyellRebuild {
                owner: gs.resolve_target_player(target).id.to_string(),
                prev_note_icons: {
                    let player = gs.resolve_target_player(target);
                    gs.cheer_count_by_id(&player.id)
                },
                yell_cards,
                total_hearts,
                note_icons,
            });
        }
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.push_rule_log_fmt(format_args!(
            "{} {}: [[log_yell_execute:n={}]]",
            pp, act_name, count
        ));
    }

    pub(crate) fn execute_modify_yell_count(&mut self, gs: &mut GameState, effect: &AbilityEffect) {
        let operation_binding = effect.operation_any();
        let operation = operation_binding.unwrap_or("subtract");
        let count: u8 = effect.count_or(0);
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.push_rule_log_fmt(format_args!(
            "{} {}: [[log_yell_count:op={},n={}]]",
            pp, act_name, operation, count
        ));
        // Every modifier is registered as data; the required count is derived
        // from base + Σ deltas at read time (see effective_cheer_checks_required).
        // Registration order and timing (before/after the base exists) do not
        // matter — Q111 semantics fall out naturally.
        let slot: u8 = if pp.contains('2') { 2 } else { 1 };
        match operation {
            "add" => gs.add_yell_count_modifier(slot, count as i32),
            "subtract" => gs.add_yell_count_modifier(slot, -(count as i32)),
            other => log::debug!("Unknown yell-count operation: {}", other),
        }
    }
}