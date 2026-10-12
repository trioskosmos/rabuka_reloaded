use super::super::super::enums::Zone;
use super::super::super::resolver::AbilityResolver;
use super::super::super::types::{Choice, ExecutionContext};
use super::super::super::util;
use crate::card::{AbilityEffect, Operator};
use crate::core::constants::CountCast;
use crate::game_state::GameState;
use super::transport::{drain_under_cards_to_energy_zone, matching_waitroom_indices, remove_card_from_any_zone};
use super::{MoveRoute, MoveSourceContext, SelectionSpec};
#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use smallvec::SmallVec;

impl AbilityResolver {
    pub(crate) fn resolve_cost_limit_reference(
        &self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<Option<u8>, String> {
        let ref_binding = effect.cost_reference_any();
        let reference = match ref_binding {
            Some(r) => r,
            None => return Ok(effect.cost_limit_any()),
        };

        let offset = effect.cost_offset_any().unwrap_or(0);
        let moved = self.selection.moved_cards.last().copied();
        let recently = gs
            .recently_moved_cards
            .as_ref()
            .and_then(|cards| cards.last().copied());
        log::debug!(
            "[COST_REF] moved_cards={:?} recently={:?} self.moved={:?}",
            moved,
            recently,
            self.selection.moved_cards
        );
        let referenced_id = match reference {
            "previous_moved_card" => moved.or(recently),
            _ => None,
        }
        .ok_or_else(|| format!("Unknown or unresolved cost reference: {}", reference))?;

        let card_name = gs
            .card_database
            .get_card(referenced_id)
            .map(|c| c.name.to_string())
            .unwrap_or_default();
        let base_cost = gs
            .card_database
            .get_card(referenced_id)
            .and_then(|c| c.cost)
            .ok_or_else(|| {
                format!(
                    "Referenced card {} ({}) has no base cost for relative cost filter",
                    referenced_id, card_name
                )
            })?;

        let resolved =
            crate::constants::saturate_u8((base_cost as i32).saturating_add(offset as i32));
        log::debug!(
            "[COST_REF] referenced='{}' name='{}' base_cost={} offset={} resolved={}",
            reference,
            card_name,
            base_cost,
            offset,
            resolved
        );
        Ok(Some(resolved))
    }

    /// Compile a move_cards effect into a granular step.
    /// Returns Ok(Some(choice)) if a selection choice is needed,
    /// Compile a move_cards effect into a single SelectCards Choice.
    /// Returns Some(choice) when a selection choice is needed (Prompt case),
    /// None when the effect should fall through to execute_move_cards.
    pub(crate) fn resolve_cards_from_source(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        selection: SelectionSpec,
        filter: &crate::ability::util::CardFilter<'_>,
        route: MoveRoute<'_>,
        card_db: &crate::card::CardDatabase,
    ) -> Result<Vec<i16>, String> {
        let MoveRoute {
            source,
            destination,
            use_p2,
        } = route;
        let SelectionSpec {
            count,
            is_max,
            is_all,
            is_self_cost,
            exclude_self,
            activating_card_id,
        } = selection;
        // Unpack the filter into the names the body below already uses, so the
        // parameter list carries one value instead of seven loose strings that
        // have to travel together through every resolver in this chain.
        let card_type_filter = filter.card_type;
        let group_name = filter.group;
        let cost_limit = filter.cost_limit;
        let cost_total = filter.cost_total;
        let cost_total_operator = filter.cost_total_operator;
        let character_filter = filter.characters;
        let name_fragments = filter.name_fragments;
        let player = if use_p2 {
            &mut gs.player2
        } else {
            &mut gs.player1
        };

        // Get cards from source
        let source_str = if source.is_empty() {
            // No source specified in the parsed ability (e.g. auto abilities
            // that trigger on zone transitions like "when X goes to waitroom").
            // The card is now in the discard/waitroom — search there.
            "discard"
        } else {
            source
        };
        if Zone::from_str(source_str) == Some(Zone::SelectedCards) {
            let selected: Vec<i16> = if self.selection.cards.is_empty() {
                gs.revealed_cost_cards.iter().copied().collect()
            } else {
                self.selection.cards.iter().copied().collect()
            };
            for &card_id in &selected {
                remove_card_from_any_zone(player, &mut gs.last_vacated_stage_area, card_id);
            }
            return Ok(selected);
        }

        if source_str == "recently_moved" {
            return self.resolve_from_recently_moved(
                gs,
                card_type_filter,
                group_name,
                card_db,
                use_p2,
            );
        }
        // "それを手札に加える" follow-ups ("if it's X, add IT to hand") — the
        // source is the SPECIFIC card the preceding sequential step moved, not
        // any matching card in a zone. Pull from self.moved_cards (the current
        // sequential's own moves) filtered by the action's character filter.
        if source_str == "preceding_moved" {
            let cards: Vec<i16> = self
                .selection.moved_cards
                .iter()
                .filter(|&&cid| {
                    cid != -1
                        && character_filter.is_none_or(|cf| {
                            util::card_matches_characters(card_db, cid, Some(cf))
                        })
                })
                .copied()
                .collect();
            for &card_id in &cards {
                remove_card_from_any_zone(player, &mut gs.last_vacated_stage_area, card_id);
            }
            return Ok(cards);
        }
        if source_str == "looked_at_remaining" {
            return self.resolve_from_looked_at(gs);
        }
        if source_str == "revealed_cards" {
            return self.resolve_from_revealed_cards(gs, selection, effect, filter, card_db);
        }
        if source_str == "those_cards" {
            if let Some(result) = self.resolve_from_those_cards(
                gs,
                selection,
                filter,
                effect,
                MoveRoute {
                    source: source_str,
                    destination,
                    use_p2,
                },
                card_db,
            )? {
                return Ok(result);
            }
            // resolve_from_those_cards returned None — no explicit trigger_moved_cards
            // was recorded (e.g. legacy trigger_auto_ability with None). Fall through
            // to the discard-pile resolution below (the historic Q252 pick-card flow).
        }
        let effective_source = if source_str == "those_cards" {
            Zone::Discard.to_str()
        } else {
            source_str
        };
        self.resolve_from_zone(
            gs,
            MoveSourceContext {
                effective_source,
                source_str,
                count,
                effect,
                card_type_filter,
                group_name,
                cost_limit,
                cost_total,
                cost_total_operator,
                character_filter,
                name_fragments,
                is_self_cost,
                is_max,
                is_all,
                exclude_self,
                activating_card_id,
                use_p2,
                destination,
                card_db,
            },
        )
    }

    fn resolve_from_recently_moved(
        &mut self,
        gs: &mut GameState,
        card_type_filter: Option<&str>,
        group_name: Option<&str>,
        card_db: &crate::card::CardDatabase,
        use_p2: bool,
    ) -> Result<Vec<i16>, String> {
        let player = if use_p2 {
            &mut gs.player2
        } else {
            &mut gs.player1
        };
        // Baton touch / cost payment — target the card(s) just moved.
        // This ensures "the card placed by this baton touch" actually
        // refers to the specific card that was moved, not any matching
        // card from the zone.
        let cards: Vec<i16> = gs
            .recently_moved_cards
            .clone()
            .unwrap_or_default()
            .into_iter()
            .filter(|&cid| {
                let ty = card_type_filter.unwrap_or("");
                ty.is_empty() || util::card_matches_type(card_db, cid, Some(ty))
            })
            .filter(|&cid| {
                group_name.is_none() || util::card_matches_group_str(card_db, cid, group_name)
            })
            .collect();
        for &card_id in &cards {
            remove_card_from_any_zone(player, &mut gs.last_vacated_stage_area, card_id);
        }
        // Card left any zone → full zone-exit cleanup (rule 4.1.4)
        gs.on_cards_left_zones(&cards);
        Ok(cards)
    }

    /// Resolve the `looked_at_remaining` source: the cards left
    /// in the looked-at pool after a preceding sequential step
    /// pulled its selection out.
    ///
    /// This only RESOLVES the source — it drains the pool and
    /// returns the cards. It must NOT place them: the caller
    /// (`execute_move_cards` → `place_taken_cards`) places
    /// every resolved card into the effect's destination
    /// through `place_card_in_zone` and records the movement
    /// with `finalize_card_movement`. Banking the pool here
    /// with a bare `waitroom.add_card` used to place each
    /// card TWICE (once here, once in `place_taken_cards`)
    /// and to record no movement event, so
    /// ミア・テイラー's 「これにより公開されたほかのすべ
    /// てのカードを控え室に置く」 duplicated every revealed
    /// card in 控え室 and never armed an any-zone→waitroom
    /// watcher.
    fn resolve_from_looked_at(
        &mut self,
        gs: &mut GameState,
    ) -> Result<Vec<i16>, String> {
        Ok(gs.looked_at_cards.drain(..).collect())
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_from_revealed_cards(
        &mut self,
        gs: &mut GameState,
        selection: SelectionSpec,
        effect: &AbilityEffect,
        incoming_filter: &crate::ability::util::CardFilter<'_>,
        card_db: &crate::card::CardDatabase,
    ) -> Result<Vec<i16>, String> {
        let SelectionSpec { count, is_all, is_max, .. } = selection;
        let card_type_filter = incoming_filter.card_type;
        let group_name = incoming_filter.group;
        let cost_limit = incoming_filter.cost_limit;
        let cost_total = incoming_filter.cost_total;
        let cost_total_operator = incoming_filter.cost_total_operator;
        let character_filter = incoming_filter.characters;
        let name_fragments = incoming_filter.name_fragments;
        let take_count = if is_all {
            gs.revealed_cards.len()
        } else {
            count.min(gs.revealed_cards.len())
        };
        let can_skip = is_max || effect.optional.unwrap_or(false);
        let mut filter = util::filter_from_parts_full(
            card_type_filter,
            group_name,
            cost_limit,
            // Honor the operator (e.g. 「コスト9以上」 >=) — dropping it let
            // below-threshold cards through the selection.
            effect.cost_limit_operator_any().map(Operator::as_str),
            character_filter,
            name_fragments,
            None, // distinct
            None, // exclude_self
            cost_total,
            cost_total_operator,
            effect.exclude_characters_any(),
        );
        // Range filters (「コスト4以上9以下」) ride the min/max fields.
        filter.cost_limit_min = effect.cost_limit_min_any();
        filter.cost_limit_max = effect.cost_limit_max_any();
        let neg = effect.negation_any().unwrap_or(false);
        let matching: Vec<usize> = (0..gs.revealed_cards.len())
            .filter(|&i| {
                let id = gs.revealed_cards[i];
                if !filter.matches(card_db, id, false) {
                    return false;
                }
                if let Some(prop) = effect.card_property_any() {
                    let has_prop = match prop {
                        "has_blade_heart" => {
                            card_db.get_card(id).is_some_and(|c| c.has_blade_heart())
                        }
                        "has_score_icon" => {
                            card_db.get_card(id).is_some_and(|c| c.has_score_icon())
                        }
                        _ => false,
                    };
                    if neg {
                        if has_prop {
                            return false;
                        }
                    } else {
                        if !has_prop {
                            return false;
                        }
                    }
                }
                true
            })
            .collect();
        if matching.is_empty() {
            return Ok(vec![]);
        }
        if take_count < matching.len() || can_skip {
            self.prompt_card_selection(
                "revealed_cards",
                take_count,
                can_skip,
                effect,
                &filter,
                Some(matching),
            );
            return Ok(vec![]);
        }
        let actual_take = take_count.min(matching.len());
        let taken: Vec<i16> = matching[..actual_take]
            .iter()
            .rev()
            .map(|&i| {
                gs.remove_revealed_at(i)
                    .expect("matching index within revealed_cards")
            })
            .collect();
        gs.remove_from_source_hands(&taken);
        // Remove from deck too — the reveal only peeked, not drained.
        // If the card was from a deck-top reveal, it's still in the
        // deck and must be removed here to avoid duplication.
        for &id in &taken {
            if let Some(pos) = gs.player1.main_deck.cards.iter().position(|&c| c == id) {
                gs.player1.main_deck.cards.remove(pos);
            } else if let Some(pos) = gs.player2.main_deck.cards.iter().position(|&c| c == id) {
                gs.player2.main_deck.cards.remove(pos);
            }
        }
        // Remove from waitroom too — yell cards went to waitroom after
        // check_live_success drained the resolution zone (Rule 8.4.7).
        for &id in &taken {
            if let Some(pos) = gs.player1.waitroom.cards.iter().position(|&c| c == id) {
                gs.player1.waitroom.cards.remove(pos);
            } else if let Some(pos) = gs.player2.waitroom.cards.iter().position(|&c| c == id) {
                gs.player2.waitroom.cards.remove(pos);
            }
        }
        Ok(taken)
    }

    fn resolve_from_those_cards(
        &mut self,
        gs: &mut GameState,
        selection: SelectionSpec,
        filter: &crate::ability::util::CardFilter<'_>,
        effect: &AbilityEffect,
        route: MoveRoute<'_>,
        card_db: &crate::card::CardDatabase,
    ) -> Result<Option<Vec<i16>>, String> {
        let MoveRoute {
            source: _,
            destination,
            use_p2,
        } = route;
        let count = selection.count;
        let card_type_filter = filter.card_type;
        let group_name = filter.group;
        // Handle "those_cards" alias: resolve to the cards that triggered the
        // each_time, captured as `trigger_moved_cards` on THIS queue entry at
        // enqueue time (the authoritative "cards that triggered me"). If no
        // explicit snapshot was recorded, fall back to the cards THIS ability
        // moved earlier in its own sequential chain — exactly the
        // 「これにより控え室に置いたカード」 / 「それらのカード」 semantics.
        // Only when neither source exists return None so the caller can
        // fall through to the legacy discard-pile resolution.
        let trigger_cards = gs
            .ability_queue
            .current_entry()
            .and_then(|e| e.trigger_moved_cards.clone())
            .filter(|c| !c.is_empty())
            .or_else(|| {
                let own_moves = self.selection.moved_cards.clone();
                (!own_moves.is_empty()).then_some(own_moves)
            });
        log::debug!(
            "[THOSE_RESOLVE] entry: trigger_moved_cards(entry)={:?} pool={:?} count={} filter={:?} group={:?} dest={} optional={:?}",
            gs.ability_queue.current_entry().and_then(|e| e.trigger_moved_cards.clone()),
            trigger_cards,
            count,
            card_type_filter,
            group_name,
            destination,
            effect.optional,
        );
        if let Some(trigger_cards) = trigger_cards {
            if !trigger_cards.is_empty() {
                if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed)
                {
                    log::debug!(
                        "[THOSE_CARDS] trigger_cards={:?} count={}",
                        trigger_cards,
                        count
                    );
                }
                let mut all_matching: Vec<i16> = Vec::new();
                for &cid in &trigger_cards {
                    if card_type_filter
                        .is_none_or(|ct| util::card_matches_type(card_db, cid, Some(ct)))
                        && group_name.is_none_or(|gn| {
                            util::card_matches_group_str(card_db, cid, Some(gn))
                        })
                    {
                        all_matching.push(cid);
                    }
                }
                if all_matching.is_empty() {
                    log::debug!("[THOSE_RESOLVE] branch=no_match (pool {:?} matched nothing)", trigger_cards);
                    // The trigger moved cards, but none matched the filters
                    // (e.g. no 虹ヶ咲 live card among the milled batch). "Those
                    // cards" means ONLY the moved cards — nothing qualifies, so
                    // the move adds nothing. Signal this explicitly so a
                    // following "…したとき" modify_score step is skipped, and do
                    // NOT fall through to the discard pile.
                    self.carried.last_move_moved_any = Some(false);
                    return Ok(Some(vec![]));
                } else if all_matching.len() <= count
                    && (destination == "deck_top_or_bottom"
                        || !effect.optional.unwrap_or(false))
                {
                    log::debug!("[THOSE_RESOLVE] branch=direct_take all_matching={:?}", all_matching);
                    // Exactly `count` or fewer match — take them directly.
                    // Two guarded exceptions fall through to the choice flow:
                    // - plain-destination OPTIONAL moves: 「〜してもよい」 must
                    //   be declinable, never auto-execute;
                    // - deck_top_or_bottom with MORE matches than count is the
                    //   dedicated Q252 pick-one branch below.
                    let found = all_matching[..count.min(all_matching.len())].to_vec();
                    if !effect.optional.unwrap_or(false) {
                        let player = if use_p2 {
                            &mut gs.player2
                        } else {
                            &mut gs.player1
                        };
                        for &cid in &found {
                            if let Some(pos) =
                                player.waitroom.cards.iter().position(|&c| c == cid)
                            {
                                player.waitroom.cards.remove(pos);
                            }
                        }
                    }
                    self.carried.last_move_moved_any = Some(!found.is_empty());
                    return Ok(Some(found));
                } else if destination == "deck_top_or_bottom" {
                    log::debug!("[THOSE_RESOLVE] branch=choice_dtob all_matching={:?}", all_matching);
                    // Q252: more matching cards than count, player chooses which one.
                    // Directly create a SelectCard choice restricted to the
                    // trigger_moved_cards' positions in the waitroom.
                    let player = if use_p2 {
                        &mut gs.player2
                    } else {
                        &mut gs.player1
                    };
                    let filtered_indices = matching_waitroom_indices(player, &all_matching);
                    let description = card_type_filter
                        .and(group_name)
                        .map(|g| format!("Select 1 {g} card to place on deck"))
                        .unwrap_or_else(|| "Select 1 card to place on deck".to_string());
                    let description_ja = card_type_filter
                        .and(group_name)
                        .map(|g| format!("{g}カードを山札に置く1枚を選択"))
                        .unwrap_or_else(|| "山札に置く1枚を選択".to_string());
                    self.awaiting.choice = Some(
                        Choice::select_cards(Zone::Discard.to_str(), 1, description, false)
                            .description_ja(Some(description_ja))
                            .card_type(card_type_filter.map(|s| s.to_string()))
                            .group(group_name.map(|s| s.to_string()))
                            .filtered_indices(Some(filtered_indices))
                            .target_player_id(Some("self".to_string()))
                            .build(),
                    );
                    return Ok(Some(vec![]));
                } else {
                    log::debug!("[THOSE_RESOLVE] branch=choice_generic all_matching={:?} count={}", all_matching, count);
                    // More matching than count — player must choose which cards.
                    // Show a SelectCard choice restricted to the trigger_moved_cards'
                    // positions in the waitroom, regardless of destination.
                    let player = if use_p2 {
                        &mut gs.player2
                    } else {
                        &mut gs.player1
                    };
                    let filtered_indices = matching_waitroom_indices(player, &all_matching);
                    let description = card_type_filter
                        .and(group_name)
                        .map(|g| format!("Select {count} {g} {}", util::card_plural(count)))
                        .unwrap_or_else(|| format!("Select {}", util::card_plural(count)));
                    let description_ja = card_type_filter
                        .and(group_name)
                        .map(|g| format!("{g}カードを{count}枚選択"))
                        .unwrap_or_else(|| "カードを選択".to_string());
                    self.awaiting.choice = Some(
                        Choice::select_cards(
                            Zone::Discard.to_str(),
                            count,
                            description,
                            effect.optional.unwrap_or(false),
                        )
                        .description_ja(Some(description_ja))
                        .card_type(card_type_filter.map(|s| s.to_string()))
                        .group(group_name.map(|s| s.to_string()))
                        .filtered_indices(Some(filtered_indices))
                        .target_player_id(Some("self".to_string()))
                        .build(),
                    );
                    return Ok(Some(vec![]));
                }
            }
        }
        Ok(None)
    }

    fn resolve_from_zone(
        &mut self,
        gs: &mut GameState,
        c: MoveSourceContext,
    ) -> Result<Vec<i16>, String> {
        match Zone::from_str(c.effective_source) {
            Some(Zone::Deck) | Some(Zone::DeckTop) => self.resolve_from_deck(gs, &c),
            Some(Zone::DeckBottom) => self.resolve_from_deck_bottom(gs, &c),
            Some(Zone::EnergyDeck) => self.resolve_from_energy_deck(gs, &c),
            Some(Zone::Stage) => self.resolve_from_stage(gs, &c),
            Some(Zone::Energy) => {
                let optional = c.effect.optional.unwrap_or(false);
                let decided = gs
                    .ability_queue
                    .current_entry()
                    .and_then(|e| e.conditional_choice.as_ref())
                    .is_some();
                log::debug!(
                    "[ENERGY_ARM] optional={} decided={} count={} src={:?} dst={:?}",
                    optional,
                    decided,
                    c.count,
                    c.effective_source,
                    c.destination,
                );
                if optional && !decided
                && self.gate_optional_source(gs, &c, Zone::Energy) {
                    return Ok(vec![]);
                }
                if c.destination == "energy_deck" || c.destination == Zone::EnergyDeck.as_str() {
                    // Energy zone→energy_deck movement: take from the end of
                    // the zone directly. Energy cards are fungible; prompting
                    // would be clobbered by interleaved cost processing.
                    let player = c.player_mut(gs);
                    let mut taken = Vec::new();
                    for _ in 0..c.count {
                        if let Some(card) = player.energy_zone.cards.pop() {
                            player.energy_zone.active_energy_count = player
                                .energy_zone
                                .active_energy_count
                                .saturating_sub(1);
                            taken.push(card);
                        }
                    }
                    return Ok(taken);
                }
                self.resolve_from_standard_zone(gs, &c)
            }
            Some(Zone::Hand)
            | Some(Zone::Discard)
            | Some(Zone::LiveCardZone)
            | Some(Zone::SuccessLiveZone) => self.resolve_from_standard_zone(gs, &c),
            Some(Zone::LookedAt) => self.resolve_source_looked_at(gs, &c),
            Some(Zone::SelectedCards) => self.resolve_from_selected_cards(gs, &c),
            Some(Zone::RevealedCards) => self.resolve_source_revealed_cards(gs, &c),
            Some(Zone::UnderMember) => self.resolve_from_under_member(gs, &c),
            _ => Err(format!("Unknown source zone: {}", c.source_str)),
        }
    }

    fn resolve_from_deck(
        &mut self,
        gs: &mut GameState,
        c: &MoveSourceContext,
    ) -> Result<Vec<i16>, String> {
        if self.gate_optional_source(gs, c, Zone::Deck) {
            return Ok(vec![]);
        }
        let count = c.count;
        let card_db = c.card_db;
        let player_id = if c.use_p2 {
            gs.player2.id.clone()
        } else {
            gs.player1.id.clone()
        };
        let track_depletion = matches!(
            Zone::from_str(c.destination),
            Some(Zone::Discard | Zone::Waitroom)
        );
        let mut deck_emptied = false;
        let player = c.player_mut(gs);
        let mut drawn = Vec::new();
        let mut attempts = 0u8;
        let mut remaining = count;
        while remaining > 0
            && attempts < (count.u8_count() + player.main_deck.cards.len().u8_count() + 10)
        {
            // Q104 / Rule 10.2.1: deck empty mid-draw → refresh from waitroom
            // and continue. This handles deck-to-discard costs/effects when the
            // deck has fewer cards than needed. E.g. deck=2, need=3:
            //   [1] draw 2 from deck → deck is empty
            //   [2] flush drawn to waitroom, refresh (shuffle waitroom into deck)
            //   [3] draw remaining 1 from new deck
            if player.main_deck.cards.is_empty() && !player.waitroom.cards.is_empty() {
                // Move already-drawn cards to waitroom so refresh includes them
                player.waitroom.cards.extend(drawn.drain(..));
                player.refresh();
            }
            if let Some(card) = player.main_deck.draw() {
                attempts += 1;
                if !util::card_matches_type(card_db, card, c.card_type_filter) {
                    player.main_deck.cards.push(card);
                    continue;
                }
                if !util::card_matches_group_str(card_db, card, c.group_name) {
                    player.main_deck.cards.push(card);
                    continue;
                }
                drawn.push(card);
                if track_depletion && player.main_deck.cards.is_empty() {
                    deck_emptied = true;
                }
                remaining = remaining.saturating_sub(1);
            } else {
                // Both deck and waitroom are empty — cannot draw more
                break;
            }
        }
        if deck_emptied {
            gs.deck_emptied_by_effect = Some(player_id.to_string());
            log::debug!(
                "[DECK_EMPTIED_BY_EFFECT] player={} source={} destination={}",
                gs.deck_emptied_by_effect.as_deref().unwrap_or_default(),
                c.source_str,
                c.destination
            );
        }
        Ok(drawn)
    }

    fn resolve_from_deck_bottom(
        &mut self,
        gs: &mut GameState,
        c: &MoveSourceContext,
    ) -> Result<Vec<i16>, String> {
        if self.gate_optional_source(gs, c, Zone::DeckBottom) {
            return Ok(vec![]);
        }
        let count = c.count;
        let player_id = if c.use_p2 {
            gs.player2.id.clone()
        } else {
            gs.player1.id.clone()
        };
        let track_depletion = matches!(
            Zone::from_str(c.destination),
            Some(Zone::Discard | Zone::Waitroom)
        );
        let mut deck_emptied = false;
        let player = c.player_mut(gs);
        let mut drawn = Vec::new();
        for _i in 0..count {
            if let Some(card) = player.main_deck.draw_bottom() {
                if track_depletion && player.main_deck.cards.is_empty() {
                    deck_emptied = true;
                }
                drawn.push(card);
            } else {
                break;
            }
        }
        if deck_emptied {
            gs.deck_emptied_by_effect = Some(player_id.to_string());
            log::debug!(
                "[DECK_EMPTIED_BY_EFFECT] player={} source={} destination={}",
                gs.deck_emptied_by_effect.as_deref().unwrap_or_default(),
                c.source_str,
                c.destination
            );
        }
        Ok(drawn)
    }

    fn resolve_from_energy_deck(
        &mut self,
        gs: &mut GameState,
        c: &MoveSourceContext,
    ) -> Result<Vec<i16>, String> {
        // UnderMember destinations keep their own conditional_optional
        // machinery (e.g. 宮下 愛's PlaceEnergyUnderMember); the shared gate
        // covers plain energy-deck moves like HOT PASSION!!.
        let under_member = Zone::from_str(c.destination) == Some(Zone::UnderMember);
        if !under_member && self.gate_optional_source(gs, c, Zone::EnergyDeck) {
            return Ok(vec![]);
        }
        let player = c.player_mut(gs);
        let count = c.count;
        let mut drawn = Vec::new();
        for _i in 0..count {
            if let Some(card) = player.energy_deck.draw() {
                drawn.push(card);
            } else {
                break;
            }
        }
        Ok(drawn)
    }

    fn resolve_from_stage(
        &mut self,
        gs: &mut GameState,
        c: &MoveSourceContext,
    ) -> Result<Vec<i16>, String> {
        // Inline pick: this fn interleaves gs.last_vacated_stage_area writes
        // with player use, so a whole-gs borrow via player_mut conflicts.
        let player = if c.use_p2 {
            &mut gs.player2
        } else {
            &mut gs.player1
        };
        let card_db = c.card_db;
        let effect = c.effect;
        if c.is_self_cost {
            let idx = c
                .activating_card_id
                .and_then(|act_id| player.stage.stage.iter().position(|&id| id == act_id))
                .ok_or_else(|| "Activating card not found at stage".to_string())?;
            gs.last_vacated_stage_area = Some(idx.u8_count());
            log::debug!(
                "[STAGE_BRANCH_SET] activating={:?} idx={} last_vacated={:?} dest={}",
                c.activating_card_id,
                idx,
                gs.last_vacated_stage_area,
                c.destination
            );

            if c.destination != "same_area" {
                let (member, member_under, energy_under) =
                    player.remove_member_from_stage_with_recycling(idx, card_db);
                // Rule 10.5.3-10.5.4: the recycling just placed the
                // under-cards to the waitroom / energy deck. Record those
                // placements so the 「…控え室に置かれたとき」 watchers
                // fire. The `player` borrow ends with the removal above,
                // so `gs` is free to push the movement events here.
                let cause_pid = if c.use_p2 {
                    gs.player2.id.clone()
                } else {
                    gs.player1.id.clone()
                };
                gs.record_under_card_recycling(
                    cause_pid.as_str(),
                    &member_under,
                    &energy_under,
                    c.activating_card_id,
                );
                Ok(member.into_iter().collect())
            } else {
                Ok(vec![player.stage.stage[idx]])
            }

        } else {
            let filter = util::filter_from_parts_full(
                c.card_type_filter,
                c.group_name,
                c.cost_limit,
                None,
                c.character_filter,
                c.name_fragments,
                None,
                if c.exclude_self {
                    c.activating_card_id
                } else {
                    None
                },
                c.cost_total,
                c.cost_total_operator,
                c.effect.exclude_characters_any(),
            );
            match util::resolve_selection(
                &player.stage.stage,
                card_db,
                c.activating_card_id,
                c.count,
                c.is_all,
                &filter,
                effect.is_self_target(),
                util::InsufficientBehavior::Silent,
                true,
            )? {
                util::SelectionOutcome::Exact(indices) => {
                    let mut vacated = None;
                    let mut member_under: Vec<i16> = Vec::new();
                    let mut energy_under: Vec<i16> = Vec::new();
                    let cards = indices
                        .iter()
                        .rev()
                        .filter_map(|&i| {
                            let cid = player.stage.stage[i];
                            if let Some(area) = crate::zones::MemberArea::from_index(i) {
                                self.record_member_last_known(
                                    cid,
                                    card_db,
                                    &gs.mods.blade_modifiers,
                                    area,
                                );
                            }
                            let (cid, mu, eu) =
                                player.remove_member_from_stage_with_recycling(i, card_db);
                            member_under.extend(mu);
                            energy_under.extend(eu);
                            if cid.is_some() {
                                vacated = Some(i.u8_count());
                            }
                            cid
                        })
                        .collect();
                    gs.last_vacated_stage_area = vacated;
                    log::debug!(
                        "[STAGE_EXACT] moved cards={:?} vacated={:?} last_vacated={:?}",
                        cards,
                        vacated,
                        gs.last_vacated_stage_area
                    );
                    // Rule 10.5.3-10.5.4: record the under-card
                    // placements the recycling performed, so the
                    // 「…控え室に置かれたとき」 watchers fire.
                    let cause_pid = if c.use_p2 {
                        gs.player2.id.clone()
                    } else {
                        gs.player1.id.clone()
                    };
                    gs.record_under_card_recycling(
                        cause_pid.as_str(),
                        &member_under,
                        &energy_under,
                        c.activating_card_id,
                    );
                    Ok(cards)
                }
                util::SelectionOutcome::Prompt => {
                    let filter = util::filter_from_parts_full(
                        c.card_type_filter,
                        c.group_name,
                        c.cost_limit,
                        None,
                        c.character_filter,
                        c.name_fragments,
                        None,
                        if c.exclude_self {
                            c.activating_card_id
                        } else {
                            None
                        },
                        c.cost_total,
                        c.cost_total_operator,
                        c.effect.exclude_characters_any(),
                    );
                    let stage_indices: Vec<usize> = (0..player.stage.stage.len())
                        .filter(|&i| filter.matches(card_db, player.stage.stage[i], true))
                        .collect();
                    self.prompt_card_selection(
                        Zone::Stage.to_str(),
                        c.count,
                        // 「〜してもよい」 optional stage moves MUST be declinable;
                        // a hardcoded false forced players to pay.
                        effect.optional.unwrap_or(false),
                        effect,
                        &filter,
                        Some(stage_indices),
                    );
                    Ok(vec![])
                }
                util::SelectionOutcome::Skip => Ok(vec![]),
            }
        }
    }

    fn resolve_from_standard_zone(
        &mut self,
        gs: &mut GameState,
        c: &MoveSourceContext,
    ) -> Result<Vec<i16>, String> {
        let player = c.player_mut(gs);
        let card_db = c.card_db;
        let effect = c.effect;
        let src_zone = Zone::from_str(c.effective_source);
        let actual_zone = c.effective_source;

        if c.is_self_cost && src_zone == Some(Zone::Hand) {
            let card_id = c
                .activating_card_id
                .ok_or_else(|| "Self hand cost has no activating card".to_string())?;
            let player = c.player_mut(gs);
            if let Some(pos) = player.hand.cards.iter().position(|&id| id == card_id) {
                player.hand.cards.remove(pos);
                log::debug!(
                    "[HAND_SELF_COST_RESOLVE] card={} from=hand position={}",
                    card_id,
                    pos
                );
                return Ok(vec![card_id]);
            }
            if let Some(pos) = player.waitroom.cards.iter().position(|&id| id == card_id) {
                player.waitroom.cards.remove(pos);
                log::debug!(
                    "[HAND_SELF_COST_RESOLVE] card={} from=waitroom position={}",
                    card_id,
                    pos
                );
                return Ok(vec![card_id]);
            }
            return Err("Self hand cost card is not in hand or waitroom".to_string());
        }

        // An empty energy zone fizzles silently — an effect like
        // "エネルギー1枚をエネルギーデッキに置く" must not abort the enclosing
        // 「その後」 sequential — and so does an empty success zone. Only a
        // live-card zone reports the shortage.
        let insufficient_behavior = match src_zone {
            Some(Zone::LiveCardZone) => util::InsufficientBehavior::Error(
                "Not enough cards in live card zone".to_string(),
            ),
            _ => util::InsufficientBehavior::Silent,
        };

        let can_skip = match src_zone {
            Some(Zone::Discard) => c.is_max || effect.optional.unwrap_or(false),
            Some(Zone::Hand) => {
                effect.optional.unwrap_or(false) || effect.any_number_any().unwrap_or(false)
            }
            Some(Zone::SuccessLiveZone) => effect.optional.unwrap_or(false),
            _ => false,
        };

        let pass_is_all = match src_zone {
            Some(Zone::Hand) | Some(Zone::Discard) | Some(Zone::Energy) => c.is_all,
            _ => false,
        };

        // Which optional filters are meaningful depends on the SOURCE zone: a
        // live-card-zone move is always a live card, an energy-zone move
        // carries no group or cost filter at all, and only hand/discard moves
        // can be narrowed by name fragments or total cost. Naming them is what
        // makes the twelve-argument filter constructor below readable.
        let from_hand_or_discard =
            matches!(src_zone, Some(Zone::Hand) | Some(Zone::Discard));
        let from_energy = src_zone == Some(Zone::Energy);
        let is_live_zone_move = src_zone == Some(Zone::LiveCardZone)
            || (src_zone == Some(Zone::SuccessLiveZone) && c.card_type_filter.is_some());
        let has_effect_groups = effect
            .group_names_any()
            .as_ref()
            .is_some_and(|g| !g.is_empty());
        let card_type = if is_live_zone_move {
            Some("live_card")
        } else if src_zone == Some(Zone::SuccessLiveZone) {
            None
        } else {
            c.card_type_filter
        };
        let group = if has_effect_groups || from_energy {
            None
        } else {
            c.group_name
        };
        let cost_limit = if from_energy { None } else { c.cost_limit };
        let cost_limit_operator = match src_zone {
            Some(Zone::Discard) => effect.cost_limit_operator_any().map(Operator::as_str),
            _ => None,
        };
        let name_fragments = if from_hand_or_discard {
            c.name_fragments
        } else {
            None
        };
        let cost_total = if from_hand_or_discard { c.cost_total } else { None };
        let cost_total_operator = if from_hand_or_discard {
            c.cost_total_operator
        } else {
            None
        };

        // §7.6: the stage-group exclusion list is built per
        // call and only this function's local `filter` reads
        // it, so the list lives in a local and is borrowed —
        // the previous `Vec::leak` here persisted for the
        // game's lifetime, one leaked allocation per
        // invocation of this effect. Declared before
        // `filter` so the borrow outlives it.
        let mut stage_group_exclusion: Option<Vec<String>> = None;
        let mut filter = util::filter_from_parts_full(
            card_type,
            group,
            cost_limit,
            cost_limit_operator,
            c.character_filter,
            name_fragments,
            None,
            None,
            cost_total,
            cost_total_operator,
            effect.exclude_characters_any(),
        );
        filter.need_heart_total = effect.need_heart_total_any();
        let nho_binding = effect.need_heart_operator_any();
        filter.need_heart_operator = nho_binding.as_deref();
        let nhc_binding = effect.need_heart_color_any();
        filter.need_heart_color = nhc_binding;
        filter.heart_colors = effect.heart_colors_any();
        if has_effect_groups {
            filter.groups = effect.group_names_any().map(Vec::as_slice);
        }
        let cp_binding = effect.card_property_any();
        filter.card_property = cp_binding;
        // The property's polarity lives in `negation` (「ブレードハートを
        // 持たない」→ negation=true). Forgetting it inverted emma bp7-008's
        // eligibility: blade-heart holders were offered instead of excluded.
        filter.negation = effect.negation_any().unwrap_or(false);

        // ── group_reference: "different_group_names" ──────────────────
        // Q89: Multi-name cards have the group name(s) from their series.
        // Q105: A multi-name card contributes ONE constituent group for
        //   "different group names" conditions.
        // Q208: When checking "different from ALL members on stage", all
        //   groups a multi-name card can match via its series are blocked.
        // → Exclude discard cards whose group matches ANY stage member's group.
        // Only apply stage-group exclusion when the effect itself has
        // group_reference and there's no condition with the same reference.
        // Bring the LOVE ab#1 has effect.group_reference without a condition
        // (genuine stage-group exclusion).  Mia option 1 has BOTH
        // condition.group_reference and effect.group_reference — the
        // condition already handles different-group counting; we should
        // NOT also exclude stage groups (which would over-filter).
        let cond_has_grp = effect
            .condition
            .as_ref()
            .and_then(|cd| {
                if cd.get_group_reference() == Some("different_group_names") {
                    Some(true)
                } else {
                    None
                }
            })
            .unwrap_or(false);
        if !cond_has_grp
            && effect.group_reference_any() == Some("different_group_names")
            && c.source_str == "discard"
        {
            let mut stage_groups: SmallVec<[String; 8]> = SmallVec::new();
            for &cid in &player.stage.stage {
                if cid == -1 {
                    continue;
                }
                if let Some(card) = card_db.get_card(cid) {
                    if !card.group.is_empty() {
                        // §7.6(2): compare as &str —
                        // the to_string() allocated a
                        // String just for the contains
                        // check, then a second one for
                        // the push.
                        if !stage_groups
                            .iter()
                            .any(|g| g.as_str() == card.group.as_ref())
                        {
                            stage_groups.push(card.group.to_string());
                        }
                    } else {
                        for known_group in util::KNOWN_GROUPS {
                            if util::card_matches_group_str(card_db, cid, Some(known_group)) {
                                let s = known_group.to_string();
                                if !stage_groups.contains(&s) {
                                    stage_groups.push(s);
                                }
                            }
                        }
                    }
                }
            }
            if !stage_groups.is_empty() {
                stage_group_exclusion = Some(stage_groups.into_iter().collect());
            }
        }
        filter.exclude_group_names = stage_group_exclusion.as_deref();

        // Only log when the filter actually constrains heart selection. Every
        // unfiltered take printed an all-None line; over a full suite that was
        // ~1.7k identical no-op lines and swamped the ones that mattered.
        if filter.need_heart_color.is_some()
            || filter.need_heart_total.is_some()
            || filter.need_heart_operator.is_some()
        {
            log::debug!(
                "[NEED_HEART] filter: color={:?} total={:?} op={:?} src={:?} card_type={:?}",
                filter.need_heart_color,
                filter.need_heart_total,
                filter.need_heart_operator,
                c.source_str,
                c.card_type_filter
            );
        }

        match self.take_cards_from_standard_zone(
            player,
            card_db,
            actual_zone,
            &filter,
            c.count,
            pass_is_all,
            insufficient_behavior,
            can_skip,
            effect,
            c.activating_card_id,
        )? {
            Some(cards) => Ok(cards),
            None => Ok(vec![]),
        }
    }

    fn resolve_source_looked_at(
        &mut self,
        gs: &mut GameState,
        c: &MoveSourceContext,
    ) -> Result<Vec<i16>, String> {
        let card_db = c.card_db;
        let count = c.count;
        let effect = c.effect;
        let matching: Vec<usize> = (0..gs.looked_at_cards.len())
            .filter(|&i| {
                let cid = gs.looked_at_cards[i];
                util::card_matches_type(card_db, cid, c.card_type_filter)
                    && util::card_matches_group_str(card_db, cid, c.group_name)
                    && util::card_matches_cost_limit(card_db, cid, c.cost_limit)
            })
            .collect();

        if matching.is_empty() {
            Ok(vec![])
        } else if effect.optional.unwrap_or(false) && count > 0 {
            // Optional looked-at selection: description must match the destination,
            // not always "Discard" (e.g. Kanan: deck_bottom).
            let max_take = count.min(matching.len());
            let dest = effect.destination.as_ref().copied();
            let (description, description_ja_str) = match dest {
                Some(Zone::DeckBottom) => (
                    format!(
                        "Place up to {} looked-at {} on bottom of deck?",
                        max_take,
                        util::card_plural(max_take)
                    ),
                    if max_take == 1 {
                        "見たカードをデッキの下に置きますか？".to_string()
                    } else {
                        format!("見たカードを最大{}枚までデッキの下に置きますか？", max_take)
                    },
                ),
                Some(Zone::DeckTop) | Some(Zone::Deck) => (
                    format!(
                        "Place up to {} looked-at {} on top of deck?",
                        max_take,
                        util::card_plural(max_take)
                    ),
                    if max_take == 1 {
                        "見たカードをデッキの上に置きますか？".to_string()
                    } else {
                        format!("見たカードを最大{}枚までデッキの上に置きますか？", max_take)
                    },
                ),
                Some(Zone::Hand) => (
                    format!(
                        "Add up to {} looked-at {} to hand?",
                        max_take,
                        util::card_plural(max_take)
                    ),
                    if max_take == 1 {
                        "見たカードを手札に加えますか？".to_string()
                    } else {
                        format!("見たカードを最大{}枚まで手札に加えますか？", max_take)
                    },
                ),
                _ => (
                    format!("Discard up to {} looked-at {}?", max_take, util::card_plural(max_take)),
                    if max_take == 1 {
                        "見たカードを控え室に置きますか？".to_string()
                    } else {
                        format!("見たカードを最大{}枚まで控え室に置きますか？", max_take)
                    },
                ),
            };
            let description_en = Some(description.clone());
            let description_ja = Some(description_ja_str);
            log::debug!(
                "[LOOKED_AT_PROMPT] dest={:?} en={:?} ja={:?}",
                dest,
                description_en,
                description_ja
            );
            let mut filter = util::CardFilter::from_effect(effect);
            // Resolve dynamic cost limit reference (e.g. "previous_moved_card" + offset)
            if let Ok(Some(cost)) = self.resolve_cost_limit_reference(gs, effect) {
                filter.cost_limit = Some(cost);
            }
            // 「デッキの上からN番目に置いてもよい」: the answer-time handler
            // (handle_select_cards_looked_at) has no access to this effect, so
            // capture the numeric deck position now and let it ride on the
            // resolver until the selection is answered.
            self.carried.looked_at_deck_position = effect
                .position_any()
                .and_then(|pi| pi.get_position())
                .and_then(|s| s.parse::<usize>().ok())
                .filter(|&n| n > 0)
                .filter(|_| effect.destination.as_ref() == Some(&Zone::DeckTop));
            self.awaiting.choice = Some(
                Choice::select_cards(Zone::LookedAt.to_str(), max_take, description, true)
                    .description_en(description_en)
                    .description_ja(description_ja)
                    .card_type(filter.card_type.map(|s| s.to_string()))
                    .cost_limit(
                        filter.cost_limit,
                        effect.cost_limit_operator_any().map(|s| s.to_string()),
                    )
                    .cost_total(
                        filter.cost_total,
                        effect.cost_total_operator_any().map(|s| s.to_string()),
                    )
                    .group(filter.group.map(|s| s.to_string()))
                    .characters(filter.characters.map(|v| v.to_vec()))
                    .target_player_id(Some(
                        effect.target.as_deref().unwrap_or("self").to_string(),
                    ))
                    .destination(effect.destination.map(|s| s.to_string()))
                    .discard_remaining(effect.discard_remaining_any())
                    .build(),
            );
            self.in_flight.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
            Ok(vec![])
        } else {
            // For LookedAt, cards are ordered: [0] = matched target,
            // remaining = revealed but non-matching. Take first `count`
            // without prompting, since the order is meaningful.
            let take = if c.is_all {
                matching.len()
            } else {
                count.min(matching.len())
            };
            let mut taken: Vec<i16> = matching[..take]
                .iter()
                .rev()
                .map(|&i| gs.looked_at_cards.remove(i))
                .collect();
            taken.reverse();
            Ok(taken)
        }
    }

    fn resolve_from_selected_cards(
        &mut self,
        gs: &mut GameState,
        c: &MoveSourceContext,
    ) -> Result<Vec<i16>, String> {
        // Inline pick: writes gs.last_vacated_stage_area below conflict with
        // a whole-gs borrow.
        let effect = c.effect;
        let count = c.count;
        let selected = self.selection.cards.clone();
        let idxs: Vec<usize> = (0..selected.len()).collect();
        match util::classify_selection(
            &idxs,
            count,
            c.is_all,
            util::InsufficientBehavior::Silent,
        )? {
            util::SelectionOutcome::Exact(indices) => {
                let taken: Vec<i16> = indices.iter().map(|&i| selected[i]).collect();
                for &card_id in &taken {
                    let player = if c.use_p2 {
                        &mut gs.player2
                    } else {
                        &mut gs.player1
                    };
                    remove_card_from_any_zone(
                        player,
                        &mut gs.last_vacated_stage_area,
                        card_id,
                    );
                    gs.remove_revealed_card(card_id);
                    gs.remove_revealed_cost_card(card_id);
                }
                self.selection.cards.retain(|card_id| !taken.contains(card_id));
                // Card left any zone → full zone-exit cleanup (rule 4.1.4)
                gs.on_cards_left_zones(&taken);
                Ok(taken)
            }
            util::SelectionOutcome::Prompt => {
                let mut filter = util::CardFilter::from_effect(effect);
                // Resolve dynamic cost limit reference (e.g. "previous_moved_card" + offset)
                if let Ok(Some(cost)) = self.resolve_cost_limit_reference(gs, effect) {
                    filter.cost_limit = Some(cost);
                }
                self.prompt_card_selection(
                    Zone::SelectedCards.to_str(),
                    count,
                    false,
                    effect,
                    &filter,
                    None,
                );
                Ok(vec![])
            }
            util::SelectionOutcome::Skip => Ok(vec![]),
        }
    }

    fn resolve_source_revealed_cards(
        &mut self,
        gs: &mut GameState,
        c: &MoveSourceContext,
    ) -> Result<Vec<i16>, String> {
        let count = c.count;
        // Inline pick: interleaves cheer_buf / revealed_cards / resolution_zone
        // access with player use, so a whole-gs borrow conflicts.
        let player = if c.use_p2 {
            &mut gs.player2
        } else {
            &mut gs.player1
        };
        // Drain from source to prevent card duplication.
        // (line 619 cloned before, leaving originals in cheer_buf/revealed_cards,
        //  causing the returned cards to still exist in the source after move.)
        // Use gs.revealed_cards as the primary source; drain cheer_buf in sync.
        let cheer_is_empty = if c.use_p2 {
            gs.player2_cheer_revealed_cards.is_empty()
        } else {
            gs.player1_cheer_revealed_cards.is_empty()
        };
        let mut restored_metas = Vec::new();
        let cards: Vec<i16> = if !cheer_is_empty {
            let cheer_buf = if c.use_p2 {
                &mut gs.player2_cheer_revealed_cards
            } else {
                &mut gs.player1_cheer_revealed_cards
            };
            let cbuf: SmallVec<[i16; 8]> = core::mem::take(cheer_buf);
            for &cid in &cbuf {
                restored_metas.push(gs.remove_revealed_card_all(cid));
            }
            cbuf.to_vec()
        } else {
            // Filter to only include cards owned by the target player.
            // revealed_cards is a global pool containing cards from both players'
            // yells/cheers. When the per-player cheer_buf has been consumed by
            // a prior ability, falling back to the entire pool would expose
            // the opponent's cards. Check zone ownership to filter correctly.
            let owned: Vec<i16> = gs
                .revealed_cards
                .iter()
                .filter(|&&cid| {
                    player.hand.cards.contains(&cid)
                        || player.waitroom.cards.contains(&cid)
                        || player.stage.stage.contains(&cid)
                        || player.stage.under_cards.iter().any(|v| v.contains(&cid))
                        || player.energy_zone.cards.contains(&cid)
                        || player.main_deck.cards.contains(&cid)
                        || player.energy_deck.cards.contains(&cid)
                        || player.live_card_zone.cards.contains(&cid)
                        || player.success_live_card_zone.cards.contains(&cid)
                        || gs.resolution_zone.cards.contains(&cid)
                })
                .copied()
                .collect();
            for &cid in &owned {
                restored_metas.push(gs.remove_revealed_card_all(cid));
            }
            owned
        };
        if cards.len() > count {
            // Put cards back so prompt_card_selection can find them.
            gs.pad_revealed_meta_tail();
            gs.revealed_cards.extend(cards.iter().copied());
            gs.revealed_card_meta.extend(restored_metas.into_iter().map(|meta| {
                meta.unwrap_or_else(|| crate::game_state::RevealedCardMeta {
                    source: None,
                    source_name: None,
                    is_private: false,
                    owner: None,
                    reveal_type: String::new(),
                })
            }));
            let filter = util::filter_from_parts_full(
                c.card_type_filter,
                c.group_name,
                c.cost_limit,
                // Honor the operator (e.g. 「コスト9以上」 >=) — dropping it
                // let below-threshold cards through the selection.
                c.effect.cost_limit_operator_any().map(Operator::as_str),
                c.character_filter,
                None, // name_fragments
                None, // distinct
                None, // exclude_self
                None, // cost_total
                None, // cost_total_operator
                c.effect.exclude_characters_any(),
            );
            let matching: Vec<usize> = (0..gs.revealed_cards.len())
                .filter(|&i| filter.matches(&gs.card_database, gs.revealed_cards[i], false))
                .collect();
            self.prompt_card_selection(
                Zone::RevealedCards.to_str(),
                count,
                c.effect.optional.unwrap_or(false),
                c.effect,
                &filter,
                Some(matching),
            );
            Ok(vec![])
        } else {
            let player = gs.active_player_mut();
            for &cid in &cards {
                if let Some(pos) = player.hand.cards.iter().position(|&cd| cd == cid) {
                    player.hand.remove_card(pos);
                }
            }
            Ok(cards)
        }
    }

    fn resolve_from_under_member(
        &mut self,
        gs: &mut GameState,
        c: &MoveSourceContext,
    ) -> Result<Vec<i16>, String> {
        log::debug!("[UNDER_MEMBER] called selected={:?} pending={:?}", self.selection.cards, self.awaiting.choice.is_some());
        let target = c.effect.target.as_deref().unwrap_or("self");
        // If this is a resumed call after the player selected a stage member,
        // self.selected_cards will contain that member's ID.
        if !self.selection.cards.is_empty() {
            log::debug!("[UNDER_MEMBER] second call selected={:?}", self.selection.cards);
            let selected_member_id = self.selection.cards[0];
            let idx_opt = {
let player = gs.resolve_target_player(target);
                player.stage.stage.iter().position(|&id| id == selected_member_id)
            };
            if let Some(idx) = idx_opt {
                let moved = drain_under_cards_to_energy_zone(gs, target, idx);
                self.carried.last_move_moved_any = Some(!moved.is_empty());
                if !moved.is_empty() {
gs.set_recently_moved_batch(moved.clone().into(), Some("under_member"));
                }
                self.selection.cards.clear();
                return Ok(moved);
            }
            self.selection.cards.clear();
            return Ok(vec![]);
        }

        // First call: find candidates and optionally prompt
        let player = gs.resolve_target_player(target);
        let candidates: Vec<usize> = (0..3)
            .filter(|&i| {
                !player.stage.under_cards[i].is_empty()
                    && player.stage.under_cards[i]
                        .iter()
                        .any(|&cid| gs.card_database.get_card(cid).is_some_and(|card| card.is_energy()))
            })
            .collect();
        if candidates.is_empty() {
            self.carried.last_move_moved_any = Some(false);
            return Ok(vec![]);
        }
        if candidates.len() == 1 {
            if c.effect.optional.unwrap_or(false) {
                // Single candidate with optional: prompt Stage selection with skip allowed
                // so player can choose to move or skip. This mirrors the multiple-candidate
                // path but with one entry, ensuring skip is possible.
                self.awaiting.choice = Some(
                    crate::ability::types::Choice::select_cards(
                        crate::ability::enums::Zone::Stage.to_str(),
                        1,
                        "Choose a member whose under energies to move".to_string(),
                        true,
                    )
                    .description_ja(Some("下のエネルギーを移動するメンバーを選択".to_string()))
                    .card_type(Some("member_card".to_string()))
                    .filtered_indices(Some(candidates.clone()))
                    .target_player_id(Some(target.to_string()))
                    .is_select_action(true)
                    .build(),
                );
                self.awaiting.stage_select_intent =
                    Some(crate::ability::types::StageSelectIntent::UnderMemberMove);
                self.in_flight.execution_context = crate::ability::types::ExecutionContext::SingleEffect { effect_index: 0 };
                return Ok(vec![]);
            }
            let idx = candidates[0];
            let moved = drain_under_cards_to_energy_zone(gs, target, idx);
            self.carried.last_move_moved_any = Some(!moved.is_empty());
            if !moved.is_empty() {
gs.set_recently_moved_batch(moved.clone().into(), Some("under_member"));
            }
            return Ok(moved);
        }
        // Multiple candidates, non-optional: prompt to choose
        self.awaiting.choice = Some(
            crate::ability::types::Choice::select_cards(
                crate::ability::enums::Zone::Stage.to_str(),
                1,
                "Choose a member whose under energies to move".to_string(),
                c.effect.optional.unwrap_or(false),
            )
            .description_ja(Some("下のエネルギーを移動するメンバーを選択".to_string()))
            .card_type(Some("member_card".to_string()))
            .filtered_indices(Some(candidates))
            .target_player_id(Some(target.to_string()))
            .is_select_action(true)
            .build(),
        );
        self.awaiting.stage_select_intent =
            Some(crate::ability::types::StageSelectIntent::UnderMemberMove);
        self.in_flight.execution_context = crate::ability::types::ExecutionContext::SingleEffect { effect_index: 0 };
        Ok(vec![])
    }


}
