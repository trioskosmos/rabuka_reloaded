//! `draw_card`, `draw_until_count` and `shuffle` - the deck.
//!
//! 46 of the 936 unique abilities emit `draw_card` or `draw_until_count`.
//! `shuffle` is here because a shuffle is a permutation of these piles.
use crate::core::constants::CountCast;
use super::super::enums::{Zone};
use super::super::resolver::AbilityResolver;
use super::super::types::{Choice, ExecutionContext};
use super::super::util;

use crate::card::{AbilityEffect, DistinctType};

use crate::game_state::GameState;
use smallvec::SmallVec;



#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

#[allow(clippy::too_many_arguments)]
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
    let mut rejected = 0;
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
                rejected = 0;
            } else {
                player.main_deck.cards.push(card);
                // Unbounded-loop guard: counting consecutive rejects bounds the
                // scan to one full deck revolution (rejects rotate to the
                // bottom, so a full cycle with no accept means no match exists).
                rejected += 1;
                if rejected >= player.main_deck.cards.len() {
                    log::debug!("[DRAW_FILTER] no matching cards remain in deck");
                    break;
                }
            }
        } else {
            // Q104 / Rule 10.2.1: deck empty mid-draw → refresh from waitroom
            if player.main_deck.cards.is_empty() && !player.waitroom.cards.is_empty() {
                player.refresh();
                rejected = 0;
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
            &self.selection.moved_cards,
            &self.selection.cards,
            self.in_flight.step_state.last_draw_count,
            self.session.activating_card_id,
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
            self.emit_pay_skip_gate(
                gs,
                None,
                format!("Draw {} {}?", count, util::card_plural(count as usize)),
                format!("{}枚ドローする？", count),
                false,
                None,
            );
            return Ok(());
        }
        let draw_count = if let Some(dc) = effect.dynamic_count_any() {
            self.resolve_dynamic_count(gs, dc)
        } else if effect.count_any() == Some(0) {
            log::debug!("[DRAW_ZERO] moved_cards={:?}", self.selection.moved_cards);
            log::debug!(
                "[DRAW_ZERO] gs.recently_moved_cards={:?}",
                gs.recently_moved_cards
            );
            log::debug!(
                "[DRAW_ZERO] last_cost_discard_count={}",
                gs.mods.last_cost_discard_count
            );
            if !self.selection.moved_cards.is_empty() {
                self.selection.moved_cards.len().u8_count()
            } else if let Some(ref moved_cards) = gs.recently_moved_cards {
                moved_cards.len().u8_count()
            } else {
                gs.mods.last_cost_discard_count
            }
        } else {
            effect.count_or(1)
        };
        log::debug!(
            "[DRAW] source={:?} action={} count={} target={} source={} destination={}",
            self.session.activating_card_id,
            effect.action,
            draw_count,
            effect.target_name(),
            effect.source_or(Zone::Deck.to_str()),
            effect.destination.map(|z| z.as_str()).unwrap_or(Zone::Hand.to_str())
        );
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
            effect.per_unit_type_any(),
        )
    }

    #[allow(clippy::too_many_arguments)]
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
            // Measured, not assumed: each side's deck can come up short on its own.
            let before: usize = ["player1", "player2"]
                .iter()
                .map(|p| {
                    util::get_zone_card_count(gs.resolve_target_player(p), destination)
                })
                .sum();
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
            let after: usize = ["player1", "player2"]
                .iter()
                .map(|p| {
                    util::get_zone_card_count(gs.resolve_target_player(p), destination)
                })
                .sum();
            self.in_flight.step_state.last_draw_count = after.saturating_sub(before).u8_count();
            return Ok(());
        }

        let activating_id = gs.activating_card;
        let orientation_modifiers = gs.mods.orientation_modifiers.clone();
        // Snapshot of members put to wait BY THIS COST (cleared/rebuilt on
        // every cost payment). 「これにより…ウェイト状態にしたメンバー1人につき」
        // must count these — not every member that happens to already be
        // waited on stage.
        let cost_waited_members = gs.last_cost_waited_members.clone();
        let (recently_moved_snapshot, entry_trigger_snapshot, last_discard_count) = (
            gs.recently_moved_cards.clone(),
            gs.entry_trigger_moved_cards(),
            gs.mods.last_cost_discard_count,
        );
        let player = gs.resolve_target_player_mut(target);

        // Measure the destination so the step can report what it ACTUALLY drew.
        //
        // `last_draw_count` used to be the REQUESTED count, and it is read back as
        // 「ドローした枚数」 — `dynamic_count`'s `previous_draw` reference, and the
        // per-step `value` handed to a following card. Asking to draw 2 from a deck
        // holding 1 reported 2, so a following 「ドローした枚数につき」 multiplied by a
        // number that never existed. Every draw path can come up short: the deck or
        // 控え室 runs out, distinct-name dedupe rejects candidates, and a
        // `card_type` filter rejects them.
        let destination_before = util::get_zone_card_count(player, destination);

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
                // per_unit_source="this_cost_waited" (parsed from
                // 「これにより…ウェイト状態にしたメンバー1人につき」): the draw is
                // per member newly waited by this ability's own cost, so the
                // tracked list replaces the whole-stage wait scan. Effects
                // without it (e.g. 「ウェイト状態のメンバー1人につき…」) keep
                // counting all currently-waited members.
                let multiplier =
                    if effect.per_unit_source_any() == Some("this_cost_waited") {
                        cost_waited_members.len().u8_count()
                    } else {
                        util::calculate_per_unit_multiplier(
                            per_unit,
                            per_unit_type,
                            player,
                            &orientation_modifiers,
                            effect.state_any(),
                        )
                    };
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
                let before = util::get_zone_card_count(player, destination);
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
                self.in_flight.step_state.last_draw_count = util::get_zone_card_count(
                    player,
                    destination,
                )
                .saturating_sub(before)
                .u8_count();
                return Ok(());
            }
        }

        if is_any_number {
            let available = util::get_zone_card_count(player, source);
            if available == 0 {
                return Ok(());
            }
            self.awaiting.choice = Some(Choice::SelectTarget {
                target: "draw_any_number".to_string(),
                description: format!("Choose how many cards to draw (0-{})", available),
                description_en: Some(format!("Choose how many cards to draw (0-{})", available)),
                description_ja: Some(format!("ドローする枚数を選択（0〜{}枚）", available)),
                allow_skip: effect.optional.unwrap_or(false),
                options: None,
            });
            self.in_flight.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
            return Ok(());
        }

        match Zone::from_str(source) {
            Some(Zone::Deck | Zone::DeckTop) => {
                if let Some(distinct) = is_distinct {
                    // Distinct draw (「名前の異なる…」): dedupe WHILE drawing, not
                    // after removal — a post-filter leaves rejected duplicate-name
                    // cards removed from the deck but placed nowhere (vanish bug).
                    // Gate on distinct_should_dedupe so pass-through variants keep
                    // the plain sequential draw below.
                    let dedupe = util::distinct_should_dedupe(Some(distinct));
                    let mut seen_names = crate::compat::HashSet::<String>::default();
                    let mut drawn: Vec<i16> = Vec::new();
                    let mut attempts = 0;
                    let max_attempts = player.main_deck.cards.len();
                    while drawn.len() < final_count as usize && attempts < max_attempts {
                        attempts += 1;
                        if let Some(card) = player.main_deck.draw() {
                            let matches_type = util::CardFilter::new()
                                .card_type_opt(card_type)
                                .matches_card(&card_db, card);
                            if !matches_type {
                                // Type rejects return to the deck bottom; they do
                                // not consume a draw slot.
                                player.main_deck.cards.push(card);
                                continue;
                            }
                            if dedupe {
                                let name = card_db.get_card(card).map(|c| {
                                    crate::card::CardDatabase::normalize_name(&c.name)
                                });
                                let accepted = match name {
                                    Some(n) => seen_names.insert(n),
                                    None => true,
                                };
                                if !accepted {
                                    log::debug!(
                                        "[DRAW_DISTINCT] retaining duplicate {} in deck",
                                        card
                                    );
                                    player.main_deck.cards.push(card);
                                    continue;
                                }
                            }
                            drawn.push(card);
                        } else {
                            break;
                        }
                    }
                    for card in &drawn {
                        util::place_card_in_zone(player, *card, destination, None, false, 1);
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
                // Distinct from discard: popping THEN filtering would vanish
                // rejected duplicates (removed from the waitroom, placed
                // nowhere). Dedupe while popping; duplicates stay in the
                // waitroom. attempts bounds the whole-zone scan.
                let dedupe = util::distinct_should_dedupe(is_distinct);
                let mut seen_names = crate::compat::HashSet::<String>::default();
                let mut cards: SmallVec<[i16; 8]> = SmallVec::new();
                let mut attempts = 0;
                let max_attempts = player.waitroom.cards.len();
                while cards.len() < final_count as usize && attempts < max_attempts {
                    attempts += 1;
                    match player.waitroom.cards.pop() {
                        Some(card) => {
                            if dedupe {
                                let accepted = card_db
                                    .get_card(card)
                                    .map(|c| {
                                        seen_names.insert(
                                            crate::card::CardDatabase::normalize_name(&c.name),
                                        )
                                    })
                                    .unwrap_or(true);
                                if !accepted {
                                    log::debug!(
                                        "[DRAW_DISTINCT] retaining duplicate {} in discard",
                                        card
                                    );
                                    player.waitroom.cards.insert(0, card);
                                    continue;
                                }
                            }
                            cards.push(card);
                        }
                        None => break,
                    }
                }
                for card in cards {
                    util::place_card_in_zone(player, card, destination, None, false, 1);
                }
            }
            _ => {
                log::debug!("Draw from source '{}' not yet implemented", source);
            }
        }
        let drawn_now = util::get_zone_card_count(player, destination)
            .saturating_sub(destination_before)
            .u8_count();
        if drawn_now != final_count {
            log::debug!(
                "[DRAW_SHORT] asked for {} but {} reached zone {}",
                final_count,
                drawn_now,
                destination
            );
        }
        self.in_flight.step_state.last_draw_count = drawn_now;
        let dst = if destination.is_empty() {
            "hand"
        } else {
            destination
        };
        gs.push_rule_log_fmt(format_args!(
            "{} {}: [[log_draw:n={},from=zone_{},to=zone_{}]]",
            pp, act_name, drawn_now, source, dst
        ));
        Ok(())
    }

    pub fn execute_draw_until_count(&mut self, gs: &mut GameState, effect: &AbilityEffect) {
        let target_count: u8 = effect.target_count_any().unwrap_or(0);
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
            to_draw.u8_count(),
            target,
            Zone::Deck.to_str(),
            destination,
            None,
            false,
            1,
            None,
        );
    }

    pub(crate) fn execute_shuffle(&mut self, gs: &mut GameState, effect: &AbilityEffect) {
        let target = effect.target_name();
        let source = effect.source_or(Zone::Deck.to_str());
        let player = gs.resolve_target_player_mut(target);
        match Zone::from_str(source) {
            Some(Zone::Deck) => {
                crate::rng::shuffle_slice(&mut player.main_deck.cards);
            }
            Some(Zone::EnergyDeck) => {
                crate::rng::shuffle_slice(&mut player.energy_deck.cards);
            }
            _ => {
                log::debug!(target: "rabuka_engine::ability::effects::misc","Unknown shuffle zone: {}", source);
            }
        }
        let pp = self.player_prefix(gs);
        if crate::core::runtime_flags::logging_enabled() {
            let act_name = gs
                .activating_card
                .map(|c| self.card_name(c))
                .unwrap_or_default();
            gs.push_rule_log_fmt(format_args!("{} {}: [[log_shuffle]]", pp, act_name));
        }
    }
}