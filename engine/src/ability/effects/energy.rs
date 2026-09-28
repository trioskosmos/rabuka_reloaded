//! `place_energy_under_member` and `pay_energy` - the energy zone.
//!
//! 6 of the 936 unique abilities emit `place_energy_under_member`
//! (`docs/ABILITY_MATRIX.md`). The energy-zone state writes and the energy
//! spending action share the zone, so they live here with it.
use crate::core::constants::CountCast;
use super::super::enums::{Zone};
use super::super::resolver::AbilityResolver;
use super::super::types::{Choice, ChoiceRoute, ExecutionContext};
use super::super::util;

use crate::card::{AbilityEffect};

use crate::game_state::GameState;



#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

impl AbilityResolver {

    /// Place energy from deck to energy zone with specific state (wait/active).
    pub(crate) fn execute_energy_placement(
        &mut self,
        gs: &mut GameState,
        state_change: &str,
        target: &str,
        count: u8,
    ) {
        let cause_cid = gs.activating_card;
        let mut placed_energy: Vec<i16> = Vec::new();
        let player_id = {
            let player = gs.resolve_target_player_mut(target);
            for _ in 0..count {
                if let Some(energy_id) = player.energy_deck.draw() {
                    player.energy_zone.cards.push(energy_id);
                    if state_change == "active" {
                        player.energy_zone.add_active(1);
                    }
                    placed_energy.push(energy_id);
                }
            }
            player.id.clone()
        };
        for &eid in &placed_energy {
            gs.push_movement_event_typed(
                eid,
                crate::core::types::ZoneId::EnergyDeck,
                crate::core::types::ZoneId::EnergyZone,
                cause_cid,
                &player_id,
                true,
            );
        }
    }

    /// Change the state of energy zone cards (wait/active).
    #[allow(clippy::too_many_arguments)]
pub(crate) fn execute_energy_state_change(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        state_change: &str,
        target: &str,
        count: u8,
        max: bool,
        card_type_filter: Option<&str>,
        group_filter: Option<&str>,
    ) -> Result<(), String> {
        let card_db = self.card_db();
        let exclude_self_id = if effect.exclude_self_any().unwrap_or(false) {
            gs.activating_card
        } else {
            None
        };
        let (wait_cards, wait_positions) = {
            let player = gs.resolve_target_player_mut(target);

            let mut filter = effect.filter_subset();
            if card_type_filter == Some("energy_card") {
                filter.group = None;
                filter.characters = None;
                filter.cost_limit = None;
                filter.cost_operator = None;
            } else {
                filter.group = group_filter;
            }
            filter.card_type = card_type_filter;
            filter.exclude_self = exclude_self_id;
            let valid_indices =
                util::matching_indices(&player.energy_zone.cards, &card_db, &filter, false);
            // State gate for the menu AND the auto-pick below: deactivating
            // offers only ACTIVE cards, activating only WAITED ones. Offering
            // (or auto-taking) an already-correct card would move the active
            // counter without changing any state.
            let is_activate = state_change == "active";
            let active_len = usize::from(player.energy_zone.active_count());
            let state_indices: Vec<usize> = valid_indices
                .iter()
                .copied()
                .filter(|&i| (i < active_len) != is_activate)
                .collect();

            let effective_count = if max {
                let available = match state_change {
                    "active" => player
                        .energy_zone
                        .cards
                        .len()
                        .saturating_sub(usize::from(player.energy_zone.active_count())),
                    _ => usize::from(player.energy_zone.active_count()),
                };
                let capped = usize::from(count).min(available).u8_count();
                log::debug!(
                    "[ENERGY] max=true: count={} available={} effective={}",
                    count,
                    available,
                    capped
                );
                capped
            } else if count == 0 {
                let val = match state_change {
                    "active" => player
                        .energy_zone
                        .cards
                        .len()
                        .saturating_sub(usize::from(player.energy_zone.active_count())),
                    _ => usize::from(player.energy_zone.active_count()),
                };
                log::debug!("[ENERGY] count=0 (all): effective={}", val);
                val.u8_count()
            } else {
                log::debug!("[ENERGY] max=false: count={} effectve={}", count, count);
                count
            };

            // Partial resolution (Q167: 「実行可能な限り解決する」): when the
            // zone holds fewer STATE-MATCHING candidates than requested, resolve
            // as many as possible instead of aborting the whole effect. Legacy
            // behavior (candidates ≥ requested) is untouched.
            let effective_count = if state_indices.len() < effective_count as usize {
                let capped = state_indices.len().u8_count();
                log::debug!(
                    "[ENERGY] partial: requested={} candidates={} effective={}",
                    effective_count,
                    state_indices.len(),
                    capped
                );
                capped
            } else {
                effective_count
            };

            if !max
                && state_indices.len() > effective_count as usize
                && state_change != "active"
            {
                let active_n = active_len;
                let waited_n = player
                    .energy_zone
                    .cards
                    .len()
                    .saturating_sub(active_len);
                let desc_en = format!(
                    "Select {} energy card(s) to deactivate (set to wait) (active: {}, waited: {})",
                    effective_count, active_n, waited_n
                );
                let desc_ja = format!("待機状態にするエネルギーカードを{}枚選択（アクティブ：{}、ウェイト：{}）", effective_count, active_n, waited_n);
                self.awaiting.choice = Some(
                    Choice::select_cards(
                        Zone::Energy.to_str(),
                        effective_count as usize,
                        desc_en,
                        false,
                    )
                    .description_ja(Some(desc_ja))
                    .card_type(card_type_filter.map(|s| s.to_string()))
                    .group(group_filter.map(|s| s.to_string()))
                    .filtered_indices(Some(state_indices.clone()))
                    .target_player_id(Some(target.to_string()))
                    .build(),
                );
                self.in_flight.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                return Ok(());
            }

            let wait_cards: Vec<i16> = state_indices
                .iter()
                .take(effective_count as usize)
                .filter_map(|i| {
                    if *i < player.energy_zone.cards.len() {
                        Some(player.energy_zone.cards[*i])
                    } else {
                        None
                    }
                })
                .collect();
            // Positional indices of the cards to change (parallel to
            // wait_cards above) for the zone rebuild below.
            let wait_positions: Vec<usize> = state_indices
                .iter()
                .take(effective_count as usize)
                .copied()
                .filter(|i| *i < player.energy_zone.cards.len())
                .collect();

            (wait_cards, wait_positions)
        };

        let active_cards: Vec<i16> = if state_change == "active" {
            // The eligible WAITING energies selected above — identical set to
            // wait_cards. Re-scanning from zone index 0 would re-include
            // already-active cards and double-count them.
            wait_cards.clone()
        } else {
            vec![]
        };

        match state_change {
            "wait" => {
                // Wait-immunity: members protected by a `cannot_wait_by_effect`
                // restriction are not put to WAIT by the OPPONENT's effects
                // ("相手の効果によってはウェイトしない").
                let controller = gs.ability_master_id();
                let skip: Vec<i16> = wait_cards
                    .iter()
                    .filter(|&&cid| {
                        let protected = gs
                            .wait_immune_members
                            .iter()
                            .any(|(m, _)| *m == cid);
                        if !protected {
                            return false;
                        }
                        let owner = gs
                            .wait_immune_members
                            .iter()
                            .find(|(m, _)| *m == cid)
                            .map(|(_, o)| o.as_str())
                            .unwrap_or_default();
                        controller.as_deref().is_some_and(|c| c != owner)
                    })
                    .copied()
                    .collect();
                for card_id in &wait_cards {
                    if skip.contains(card_id) {
                        continue;
                    }
                    gs.mods.add_orientation_modifier(*card_id, "wait");
                }
                // Counter + positions move together through the zone rebuild
                // (skipping immune members). No blind arithmetic.
                {
                    let skip_set = skip.clone();
                    let player = gs.resolve_target_player_mut(target);
                    let idx: Vec<usize> = wait_positions
                        .iter()
                        .copied()
                        .filter(|&i| {
                            i < player.energy_zone.cards.len()
                                && !skip_set.contains(&player.energy_zone.cards[i])
                        })
                        .collect();
                    player.energy_zone.set_indices_waited(&idx);
                }
            }
            "active" => {
                for card_id in &active_cards {
                    gs.mods.add_orientation_modifier(*card_id, "active");
                    // Turn-scoped attributed log (see GameState::turn_state_changes).
                    gs.turn_state_changes.push((
                        gs.activating_card.unwrap_or(-1),
                        *card_id,
                        "wait".to_string(),
                        "active".to_string(),
                    ));
                }
                // Same positions the ids were taken from (parallel arrays).
                {
                    let player = gs.resolve_target_player_mut(target);
                    player.energy_zone.set_indices_active(&wait_positions);
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn execute_place_energy_under_member(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) {
        self.execute_place_energy_under_member_impl(gs, effect, false)
    }

    /// Cost-path variant: forces optional=false to avoid re-prompting the
    /// optional-cost choice (infinite-loop guard used by cost.rs re-entry).
    pub fn execute_place_energy_under_member_non_optional(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) {
        self.execute_place_energy_under_member_impl(gs, effect, true)
    }

    fn execute_place_energy_under_member_impl(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        force_non_optional: bool,
    ) {
        // Resolve the count (dynamic_count overrides energy_count).
        let count: u8 = if let Some(dc) = effect.dynamic_count_any() {
            self.resolve_dynamic_count(gs, dc)
        } else {
            effect.energy_count_any().unwrap_or(1)
        };
        let target = effect.target_name().to_string();
        let optional = !force_non_optional && effect.optional.unwrap_or(false);
        let source = effect.source_any().map(|s| s.to_string());
        let any_number = effect.any_number_any().unwrap_or(false);

        // Special case: place (dynamic_count) energy cards from the energy
        // deck into the energy zone in WAIT state.
        // e.g. PL!N-bp5-012-R+ LiveSuccess: place (under_count + 1) from deck.
        // The parser historically encoded this inverted
        // (source=under_member, destination=energy_zone); both encodings are
        // accepted. No destination phrase in the text means the energy zone
        // by rule; "ウェイト状態で置く" = inactive, so active count is untouched.
        let wants_wait_energy_to_zone = source.as_deref() == Some("under_member")
            && effect.destination == Some(Zone::Energy);
        let canonical_energy_deck_to_zone = source.as_deref() == Some("energy_deck")
            && effect.destination == Some(Zone::Energy)
            && effect.state_change_any() == Some("wait");
        if wants_wait_energy_to_zone || canonical_energy_deck_to_zone {
            let player = gs.resolve_target_player_mut(&target);
            for _ in 0..count {
                if let Some(energy) = player.energy_deck.draw() {
                    player.energy_zone.cards.push(energy);
                    // Don't increment active_energy_count — wait state
                } else {
                    break;
                }
            }
            return;
        }

        // Special case: deploy from under_member to empty_area
        // (e.g. PL!-bp6-003-R+ LiveSuccess)
        if source.as_deref() == Some("under_member")
            && effect.destination == Some(Zone::EmptyArea)
        {
            let player = gs.resolve_target_player(&target);
            let has_empty_slot = (0..3).any(|i| player.stage.stage[i] == -1);
            if !has_empty_slot {
                return;
            }
            let pos = gs
                .activating_card
                .and_then(|c| player.stage.stage.iter().position(|&id| id == c))
                .unwrap_or(1);
            let area = util::pos_to_area(pos);
            let under_cards = player.stage.get_under_cards(area);
            if under_cards.is_empty() {
                return;
            }
            let target_str = target.clone();
            let desc_ja = "このメンバーの下から出すメンバーカードを選択".to_string();
            let mut b = Choice::select_cards(
                Zone::UnderMember.to_str(),
                count as usize,
                "Select a member card to deploy from under this member",
                optional,
            )
            .description_ja(Some(desc_ja))
            .card_type(effect.card_type_any().map(|s| s.to_string()))
            .target_player_id(Some(target_str));
            if let Some(groups) = effect.group_names_any() {
                if let Some(first) = groups.first() {
                    b = b.group(Some(first.clone()));
                }
            }
            b = b.cost_limit(
                effect.cost_limit_any(),
                effect.cost_limit_operator_any().map(|s| s.to_string()),
            );
            self.awaiting.choice = Some(b.build());
            self.in_flight.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
            return;
        }

        if source.as_deref() == Some("energy_deck") {
            // Optional "置いてもよい" placement: offer a Skip/Do choice first.
            // The accept path re-enters via handle_optional_cost_payment →
            // execute_place_energy_under_member_non_optional (optional=false), so
            // we gate only the initial prompt here.
            if optional && gs.entry_choice_card_no() != Some(ChoiceRoute::OptionalCost) {
                self.emit_pay_skip_gate(
                    gs,
                    Some(ChoiceRoute::OptionalCost),
                    "Place energy under a member (optional)?".to_string(),
                    "メンバーの下にエネルギーを置きますか？（オプション）".to_string(),
                    true,
                    None,
                );
                return;
            }
            // Move `count` energy cards from the ENERGY DECK under a member on
            // stage (e.g. "自分のエネルギーデッキから、エネルギーカード1枚を…
            // メンバーの下に置く"). Prefer the activating card when it matches the
            // effect's group filter (or there is no filter); otherwise fall back
            // to the first stage member that matches the filter.
            let card_db = gs.card_database.clone();
            let matches_group = |id: i16| -> bool {
                match effect.group_names_any() {
                    Some(groups) => groups
                        .iter()
                        .any(|g| crate::ability::util::card_matches_group_str(&card_db, id, Some(g))),
                    None => true,
                }
            };
            let player = gs.resolve_target_player_mut(&target);
            if player.energy_deck.cards.is_empty() {
                return;
            }
            let eligible: Vec<usize> = (0..3)
                .filter(|&i| player.stage.stage[i] != -1 && matches_group(player.stage.stage[i]))
                .collect();
            if eligible.is_empty() {
                return;
            }
            if eligible.len() > 1 {
                let mut builder = Choice::select_cards(
                    Zone::Stage.to_str(),
                    1,
                    "Select a member to place energy underneath",
                    false,
                )
                .card_type(Some("member_card".to_string()))
                .target_player_id(Some(target.clone()))
                .is_select_action(true);
                if let Some(group) = effect.group_names_any().and_then(|groups| groups.first()) {
                    builder = builder.group(Some(group.clone()));
                }
                self.awaiting.choice = Some(builder.build());
                self.awaiting.stage_select_intent =
                    Some(crate::ability::types::StageSelectIntent::PlaceEnergyDeckUnder);
                self.in_flight.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                return;
            }
            let idx = eligible[0];
            let area = util::pos_to_area(idx);
            for _ in 0..count {
                if let Some(energy) = player.energy_deck.draw() {
                    player.stage.place_under_card(area, energy);
                } else {
                    break;
                }
            }
            gs.recalculate_constants();
            return;
        }

        if Zone::from_str(source.as_deref().unwrap_or("")) == Some(Zone::UnderMember) {
            let player = gs.resolve_target_player_mut(&target);
            let mut all_under: Vec<i16> = Vec::new();
            for si in 0..3 {
                for &cid in &player.stage.under_cards[si] {
                    all_under.push(cid);
                }
            }
            if all_under.is_empty() {
                return;
            }

            // any_number → count=0 so the player can select any subset
            // (the choice handler re-prompts after each selection).
            // Fixed count → require exactly that many (up to max available).
            let choice_count = if any_number {
                0
            } else {
                all_under.len().min(count as usize)
            };
            self.awaiting.choice = Some(
                Choice::select_cards(
                    Zone::UnderMember.to_str(),
                    choice_count,
                    "Select a card to move from under member",
                    optional,
                )
                .description_ja(Some(
                    "メンバーの下からカードを選択".to_string(),
                ))
                .card_type(effect.card_type_any().map(|s| s.to_string()))
                .target_player_id(Some(target.to_string()))
                .build(),
            );
            self.in_flight.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
            return;
        }

        // Original logic: move from energy_zone to under_member
        // This should only execute if source is NOT "under_member"
        // Ask the player to tap the specific energy card(s) to place under the
        // member. Both active and waited energy cards are selectable. Skippable
        // only when the placement is optional (single prompt, no separate
        // pay/skip gate).
        if gs.resolve_target_player(&target).energy_zone.cards.is_empty() {
            return;
        }
        // State counts so the player can tell active apart from waited —
        // both are selectable here, unlike payment menus.
        let (active_n, waited_n) = {
            let player = gs.resolve_target_player(&target);
            let active = player.energy_zone.active_count() as usize;
            (active, player.energy_zone.cards.len().saturating_sub(active))
        };
        let state_suffix_en = format!(" (active: {}, waited: {})", active_n, waited_n);
        let state_suffix_ja = format!("（アクティブ：{}、ウェイト：{}）", active_n, waited_n);
        let desc_ja = if count == 1 {
            format!("このメンバーの下に置くエネルギーカードを選択{}", state_suffix_ja)
        } else {
            format!("このメンバーの下に置くエネルギーカードを{}枚選択{}", count, state_suffix_ja)
        };
        self.awaiting.choice = Some(
            Choice::select_cards(
                Zone::Energy.to_str(),
                count as usize,
                format!("Choose {} {} to place under member{}", count, util::card_plural(count as usize), state_suffix_en),
                optional,
            )
            .destination(Some("under_member".to_string()))
            .card_type(Some("energy_card".to_string()))
            .target_player_id(Some(target.clone()))
            .description_ja(Some(desc_ja))
            .build(),
        );
        self.in_flight.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
    }

    pub(crate) fn execute_pay_energy(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let count: u8 = if let Some(dc) = effect.dynamic_count_any() {
            self.resolve_dynamic_count(gs, dc)
        } else {
            effect
                .energy_count_any()
                .unwrap_or_else(|| effect.count_or(0))
        };
        if effect.optional.unwrap_or(false) {
            let player = gs.resolve_target_player(effect.target_name());
            if player.energy_zone.active_count() < count {
                // Insufficient energy: skip payment and clear remaining actions
                self.in_flight.cancel_remaining_commands = true;
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.pending_actions.clear();
                }
                return Ok(());
            }
            self.awaiting.energy_payment = Some(count);
            self.emit_pay_skip_gate(
                gs,
                None,
                format!("Pay {} energy?", count),
                format!("{}エネルギー支払う？", count),
                false,
                None,
            );
            return Ok(());
        }
        if count > 0 {
            let player = gs.resolve_target_player_mut(effect.target_name());
            player.energy_zone.pay_energy(count)?;
            // Energy-count constants (「エネルギーがちょうどN枚あるかぎり」)
            // are live state — re-evaluate right after the payment.
            gs.recalculate_constants();
        }
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.rule_log
            .push(format!("{} {}: {}エネルギー支払", pp, act_name, count));
        Ok(())
    }
}