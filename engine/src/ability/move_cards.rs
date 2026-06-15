use super::resolver::AbilityResolver;
use super::types::{Choice, ExecutionContext, LookAndSelectStep};
use super::util;
use crate::card::{AbilityEffect, CardDatabase};
use crate::zones;

enum SelectionOutcome {
    Exact(Vec<usize>),
    Prompt,
    Skip,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MoveCardsTarget {
    PlayerSelf,
    Opponent,
}

enum InsufficientBehavior {
    Silent,
    Error(&'static str),
}

fn classify_selection(
    idxs: &[usize],
    count: usize,
    is_all: bool,
    on_insufficient: InsufficientBehavior,
) -> Result<SelectionOutcome, String> {
    if is_all {
        return Ok(SelectionOutcome::Exact(idxs.to_vec()));
    }
    if idxs.len() < count {
        return match on_insufficient {
            InsufficientBehavior::Silent => Ok(SelectionOutcome::Skip),
            InsufficientBehavior::Error(msg) => Err(msg.to_string()),
        };
    }
    if idxs.len() > count {
        return Ok(SelectionOutcome::Prompt);
    }
    Ok(SelectionOutcome::Exact(idxs.to_vec()))
}

macro_rules! remove_by_indices {
    ($zone:expr, $indices:expr) => {
        $indices
            .iter()
            .rev()
            .map(|&i| $zone.remove(i))
            .collect::<Vec<i16>>()
    };
}

macro_rules! resolve_zone {
    ($resolver:expr, $player:expr, $card_db:expr, $activating_id:expr, $zone_name:expr, $cards:expr, $count:expr, $is_all:expr, $filter:expr, $effect:expr, $behavior:expr, $can_skip:expr, $skip_empty:expr, $indices:ident, $exact_block:block) => {
        match resolve_selection(
            $cards,
            $card_db,
            $activating_id,
            $count,
            $is_all,
            $filter,
            $effect,
            $behavior,
            $skip_empty,
        )? {
            SelectionOutcome::Exact($indices) => $exact_block,
            SelectionOutcome::Prompt => {
                $resolver.prompt_card_selection($zone_name, $count, $can_skip, $effect, $filter);
                return Ok(());
            }
            SelectionOutcome::Skip => vec![],
        }
    };
}

fn remove_cards_from_hand(player: &mut crate::player::Player, indices: &[usize]) -> Vec<i16> {
    remove_by_indices!(player.hand.cards, indices)
}

fn get_selection_indices(
    cards: &[i16],
    card_db: &CardDatabase,
    activating_card: Option<i16>,
    filter: &util::CardFilter,
    effect: &AbilityEffect,
    skip_empty: bool,
) -> Vec<usize> {
    let mut idxs = util::matching_indices(cards, card_db, filter, skip_empty);
    if effect.self_target.unwrap_or(false) {
        if let Some(aid) = activating_card {
            idxs.retain(|&i| i < cards.len() && cards[i] == aid);
        }
    }
    idxs
}

fn resolve_selection(
    cards: &[i16],
    card_db: &CardDatabase,
    activating_card: Option<i16>,
    count: usize,
    is_all: bool,
    filter: &util::CardFilter,
    effect: &AbilityEffect,
    behavior: InsufficientBehavior,
    skip_empty: bool,
) -> Result<SelectionOutcome, String> {
    let idxs = get_selection_indices(cards, card_db, activating_card, filter, effect, skip_empty);
    classify_selection(&idxs, count, is_all, behavior)
}

fn remove_card_from_any_zone(
    player: &mut crate::player::Player,
    last_vacated_stage_area: &mut Option<usize>,
    card_id: i16,
) {
    if let Some(pos) = player.hand.cards.iter().position(|&id| id == card_id) {
        player.hand.cards.remove(pos);
    } else if let Some(pos) = player.waitroom.cards.iter().position(|&id| id == card_id) {
        player.waitroom.cards.remove(pos);
    } else if let Some(pos) = player.stage.stage.iter().position(|&id| id == card_id) {
        player.stage.stage[pos] = -1;
        *last_vacated_stage_area = Some(pos);
    } else if let Some(pos) = player
        .energy_zone
        .cards
        .iter()
        .position(|&id| id == card_id)
    {
        player.energy_zone.cards.remove(pos);
    }
}

impl<'a> AbilityResolver<'a> {
    fn resolve_cost_limit_reference(&self, effect: &AbilityEffect) -> Result<Option<u32>, String> {
        let reference = match effect.cost_reference.as_deref() {
            Some(r) => r,
            None => return Ok(effect.cost_limit),
        };

        let offset = effect.cost_offset.unwrap_or(0);
        let referenced_id = match reference {
            "previous_moved_card" => self.moved_cards.last().copied().or_else(|| {
                self.game_state
                    .recently_moved_cards
                    .as_ref()
                    .and_then(|cards| cards.last().copied())
            }),
            _ => None,
        }
        .ok_or_else(|| format!("Unknown or unresolved cost reference: {}", reference))?;

        let base_cost = self
            .game_state
            .card_database
            .get_card(referenced_id)
            .and_then(|c| c.cost)
            .ok_or_else(|| {
                format!(
                    "Referenced card {} has no base cost for relative cost filter",
                    referenced_id
                )
            })?;

        let resolved = (base_cost as i32).saturating_add(offset).max(0) as u32;
        Ok(Some(resolved))
    }

    fn prompt_card_selection(
        &mut self,
        zone: &str,
        count: usize,
        can_skip: bool,
        effect: &AbilityEffect,
        filter: &util::CardFilter,
    ) {
        self.pending_choice = Some(
            Choice::select_cards(
                zone,
                count,
                format!("Select {} card(s) from {}", count, zone.replace("_", " ")),
                can_skip,
            )
            .card_type(filter.card_type.map(|s| s.to_string()))
            .cost_limit(filter.cost_limit, effect.cost_limit_operator.clone())
            .group(filter.group.map(|s| s.to_string()))
            .characters(filter.characters.cloned())
            .build(),
        );
        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
    }

    pub fn execute_move_cards(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // Resolve dynamic_count if count is not explicitly set
        let count = if effect.count.is_some() {
            effect.count.unwrap() as usize
        } else if let Some(ref dc) = effect.dynamic_count {
            match dc.count_type.as_str() {
                "revealed_cards" => {
                    // Count cards revealed by a previous cost/effect step
                    let revealed = if !self.game_state.player1_cheer_revealed_cards.is_empty() {
                        &self.game_state.player1_cheer_revealed_cards
                    } else {
                        &self.game_state.revealed_cards
                    };
                    revealed.len()
                }
                _ => 0,
            }
        } else {
            0
        };
        let cost_limit = self.resolve_cost_limit_reference(effect)?;
        let group_name = effect.group_name();

        // Handle or_card_types: let the player pick which type to search for
        let card_type_owned: Option<String> = if let Some(or_types) = &effect.or_card_types {
            if or_types.is_empty() {
                effect.card_type.clone()
            } else {
                let chosen = self
                    .game_state
                    .ability_queue
                    .current_entry()
                    .and_then(|e| e.conditional_choice.clone());
                match chosen {
                    Some(s) => Some(s),
                    None => {
                        self.pending_choice = Some(Choice::SelectTarget {
                            target: "choice_string".to_string(),
                            description: format!("Pick card type: {:?}", or_types),
                            allow_skip: false,
                            options: None,
                        });
                        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                        self.game_state.ability_queue.current_entry_mut().map(|e| {
                            e.conditional_choice = Some(serde_json::to_string(&or_types).unwrap());
                        });
                        return Ok(());
                    }
                }
            }
        } else {
            effect.card_type.clone()
        };
        let card_type_filter: Option<&str> = card_type_owned.as_deref();
        let tgt = effect.target.clone();
        let is_self_cost = effect.self_cost.unwrap_or(false);
        let exclude_self = effect.exclude_self.unwrap_or(false);
        let is_max = effect.max.unwrap_or(false);
        let is_all = effect.all.unwrap_or(false);
        let card_db = self.game_state.card_database.clone();
        let activating_card_id = self.game_state.activating_card;
        let vacated_stage_area = self.game_state.last_vacated_stage_area;
        self.game_state.last_vacated_stage_area = None;

        // Character name filter from the effect
        let character_filter: Option<Vec<String>> = effect.characters.clone();

        // Resolve name_constraint (e.g. "contains_all" from a revealed card)
        let name_fragments: Option<Vec<String>> = if effect.name_constraint.as_deref()
            == Some("contains_all")
            && effect.name_constraint_source.as_deref() == Some("revealed_card")
        {
            let fragments: Vec<String> = self
                .game_state
                .revealed_cost_cards
                .iter()
                .chain(self.game_state.revealed_cards.iter())
                .filter_map(|&id| {
                    let card = self.game_state.card_database.get_card(id)?;
                    Some(
                        card.name
                            .replace("\u{FF06}", "&")
                            .split('&')
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>(),
                    )
                })
                .flatten()
                .collect();
            if fragments.is_empty() {
                None
            } else {
                Some(fragments)
            }
        } else {
            None
        };

        let _name_filter = |card_db: &crate::card::CardDatabase, card_id: i16| -> bool {
            match &name_fragments {
                Some(fragments) => {
                    if let Some(card) = card_db.get_card(card_id) {
                        fragments.iter().all(|f| card.name.contains(f.as_str()))
                    } else {
                        false
                    }
                }
                None => true,
            }
        };

        let mut moved_cards: Vec<i16> = Vec::new();
        let source = effect.source.clone().unwrap_or_default();
        let destination = effect.destination.clone().unwrap_or_default();

        {
            let player = match tgt.as_ref().map(|s: &String| s.as_str()).unwrap_or("self") {
                "self" => &mut self.game_state.player1,
                "opponent" => &mut self.game_state.player2,
                _ => &mut self.game_state.player1,
            };

            // --- STEP 1: Get cards from source ---
            let source_str = if source.is_empty() {
                ""
            } else {
                source.as_str()
            };
            let mut taken: Vec<i16> = if !self.selected_cards.is_empty()
                && source_str == "selected_cards"
            {
                let selected = self.selected_cards.clone();
                for &card_id in &selected {
                    remove_card_from_any_zone(
                        player,
                        &mut self.game_state.last_vacated_stage_area,
                        card_id,
                    );
                }
                selected
            } else {
                match source_str {
                    // Deck → anything (sequential draw, no selection prompt)
                    "deck" | "deck_top" => {
                        let mut drawn = Vec::new();
                        let mut attempts = 0u32;
                        while drawn.len() < count
                            && attempts < (count as u32 + player.main_deck.cards.len() as u32)
                        {
                            if let Some(card) = player.main_deck.draw() {
                                attempts += 1;
                                if !util::card_matches_type(&card_db, card, card_type_filter) {
                                    player.main_deck.cards.push(card);
                                    continue;
                                }
                                if !util::card_matches_group_str(&card_db, card, group_name) {
                                    player.main_deck.cards.push(card);
                                    continue;
                                }
                                drawn.push(card);
                            } else {
                                break;
                            }
                        }
                        drawn
                    }
                    "energy_deck" => {
                        let mut drawn = Vec::new();
                        for _i in 0..count {
                            if let Some(card) = player.energy_deck.draw() {
                                drawn.push(card);
                            } else {
                                break;
                            }
                        }
                        drawn
                    }

                    // Stage → anything
                    "stage" => {
                        if is_self_cost {
                            let mut cards = Vec::new();
                            let mut found = false;
                            if let Some(activating_id) = activating_card_id {
                                for i in 0..3 {
                                    if player.stage.stage[i] == activating_id {
                                        self.game_state.last_vacated_stage_area = Some(i);
                                        // Only recycle under-cards if the card is actually leaving (not same_area)
                                        if destination != "same_area" {
                                            if let Some(cid) = player
                                                .remove_member_from_stage_with_recycling(
                                                    i, &card_db,
                                                )
                                            {
                                                cards.push(cid);
                                            }
                                        } else {
                                            cards.push(player.stage.stage[i]);
                                            player.stage.stage[i] = activating_id;
                                            self.game_state.last_vacated_stage_area = None;
                                        }
                                        found = true;
                                        break;
                                    }
                                }
                            }
                            if !found {
                                return Err("Activating card not found at stage".to_string());
                            }
                            cards
                        } else {
                            let filter = util::filter_from_parts_full(
                                card_type_filter,
                                group_name,
                                cost_limit,
                                None,
                                character_filter.as_ref(),
                                name_fragments.as_ref(),
                                None,
                                if exclude_self {
                                    activating_card_id
                                } else {
                                    None
                                },
                            );
                            let idxs = get_selection_indices(
                                &player.stage.stage,
                                &card_db,
                                activating_card_id,
                                &filter,
                                effect,
                                true,
                            );
                            match classify_selection(
                                &idxs,
                                count,
                                is_all,
                                InsufficientBehavior::Silent,
                            )? {
                                SelectionOutcome::Exact(indices) => {
                                    let (cards, vacated) =
                                        Self::stage_remove_with_vacated(player, &indices, &card_db);
                                    self.game_state.last_vacated_stage_area = vacated;
                                    cards
                                }
                                SelectionOutcome::Prompt => {
                                    self.prompt_card_selection(
                                        "stage", count, false, effect, &filter,
                                    );
                                    return Ok(());
                                }
                                SelectionOutcome::Skip => vec![],
                            }
                        }
                    }

                    // Template zones: CardFilter → matching_indices → classify_selection
                    "hand" => {
                        let filter = util::filter_from_parts_full(
                            card_type_filter,
                            group_name,
                            cost_limit,
                            None,
                            character_filter.as_ref(),
                            name_fragments.as_ref(),
                            None,
                            None,
                        );
                        resolve_zone!(
                            self,
                            player,
                            &card_db,
                            activating_card_id,
                            "hand",
                            &player.hand.cards,
                            count,
                            is_all,
                            &filter,
                            effect,
                            InsufficientBehavior::Silent,
                            effect.optional.unwrap_or(false),
                            false,
                            indices,
                            { remove_cards_from_hand(player, &indices) }
                        )
                    }
                    "discard" => {
                        let filter = util::filter_from_parts_full(
                            card_type_filter,
                            group_name,
                            cost_limit,
                            effect.cost_limit_operator.as_deref(),
                            character_filter.as_ref(),
                            name_fragments.as_ref(),
                            None,
                            None,
                        );
                        let can_skip = is_max || effect.optional.unwrap_or(false);
                        resolve_zone!(
                            self,
                            player,
                            &card_db,
                            activating_card_id,
                            "discard",
                            &player.waitroom.cards,
                            count,
                            is_all,
                            &filter,
                            effect,
                            InsufficientBehavior::Silent,
                            can_skip,
                            false,
                            indices,
                            {
                                if vacated_stage_area.is_some() {
                                    self.game_state.last_vacated_stage_area = vacated_stage_area;
                                }
                                remove_by_indices!(player.waitroom.cards, &indices)
                            }
                        )
                    }
                    "energy_zone" => {
                        let filter = util::filter_from_parts(
                            card_type_filter,
                            None,
                            None,
                            None,
                            character_filter.as_ref(),
                            None,
                            None,
                        );
                        resolve_zone!(
                            self,
                            player,
                            &card_db,
                            activating_card_id,
                            "energy_zone",
                            &player.energy_zone.cards,
                            count,
                            is_all,
                            &filter,
                            effect,
                            InsufficientBehavior::Error("Not enough cards in energy zone"),
                            false,
                            false,
                            indices,
                            { remove_by_indices!(player.energy_zone.cards, &indices) }
                        )
                    }
                    "live_card_zone" => {
                        let filter = util::filter_from_parts(
                            Some("live_card"),
                            group_name,
                            cost_limit,
                            None,
                            character_filter.as_ref(),
                            None,
                            None,
                        );
                        resolve_zone!(
                            self,
                            player,
                            &card_db,
                            activating_card_id,
                            "live_card_zone",
                            &player.live_card_zone.cards,
                            count,
                            false,
                            &filter,
                            effect,
                            InsufficientBehavior::Error("Not enough cards in live card zone"),
                            false,
                            false,
                            indices,
                            { remove_by_indices!(player.live_card_zone.cards, &indices) }
                        )
                    }
                    "success_live_zone" => {
                        let filter = util::filter_from_parts(
                            None,
                            None,
                            None,
                            None,
                            character_filter.as_ref(),
                            None,
                            None,
                        );
                        resolve_zone!(
                            self,
                            player,
                            &card_db,
                            activating_card_id,
                            "success_live_zone",
                            &player.success_live_card_zone.cards,
                            count,
                            false,
                            &filter,
                            effect,
                            InsufficientBehavior::Error("Not enough cards in success live zone"),
                            false,
                            false,
                            indices,
                            { remove_by_indices!(player.success_live_card_zone.cards, &indices) }
                        )
                    }
                    "those_cards" => {
                        let filter = util::filter_from_parts(
                            card_type_filter,
                            group_name,
                            None,
                            None,
                            character_filter.as_ref(),
                            None,
                            None,
                        );
                        resolve_zone!(
                            self,
                            player,
                            &card_db,
                            activating_card_id,
                            "discard",
                            &player.waitroom.cards,
                            count,
                            false,
                            &filter,
                            effect,
                            InsufficientBehavior::Silent,
                            false,
                            false,
                            indices,
                            {
                                if vacated_stage_area.is_some() {
                                    self.game_state.last_vacated_stage_area = vacated_stage_area;
                                }
                                remove_by_indices!(player.waitroom.cards, &indices)
                            }
                        )
                    }
                    "looked_at" => {
                        let matching: Vec<usize> = (0..self.looked_at_cards.len())
                            .filter(|&i| {
                                let cid = self.looked_at_cards[i];
                                util::card_matches_type(&card_db, cid, card_type_filter)
                                    && util::card_matches_group_str(&card_db, cid, group_name)
                                    && util::card_matches_cost_limit(&card_db, cid, cost_limit)
                            })
                            .collect();

                        let take_fn = |cards: &mut Vec<i16>, idxs: &[usize]| -> Vec<i16> {
                            let mut sorted = idxs.to_vec();
                            sorted.sort_unstable_by(|a, b| b.cmp(a));
                            sorted.iter().map(|&i| cards.remove(i)).collect()
                        };

                        if matching.is_empty() {
                            vec![]
                        } else if matching.len() > count && !is_all {
                            // Too many candidates — prompt the user
                            self.pending_choice = Some(
                                Choice::select_cards(
                                    "looked_at",
                                    count,
                                    format!("Select {} card(s) from looked-at cards", count),
                                    false,
                                )
                                .card_type(card_type_filter.map(|s| s.to_string()))
                                .cost_limit(cost_limit, effect.cost_limit_operator.clone())
                                .group(group_name.map(|s| s.to_string()))
                                .characters(character_filter.clone())
                                .build(),
                            );
                            self.execution_context =
                                ExecutionContext::SingleEffect { effect_index: 0 };
                            return Ok(());
                        } else {
                            take_fn(&mut self.looked_at_cards, &matching)
                        }
                    }
                    "looked_at_remaining" => {
                        // Only move remaining cards after the chosen card (first card)
                        let cards: Vec<i16> = if self.looked_at_cards.len() > 1 {
                            self.looked_at_cards.drain(1..).collect()
                        } else {
                            self.looked_at_cards.drain(..).collect()
                        };
                        for &card in &cards {
                            player.waitroom.add_card(card);
                        }
                        cards
                    }
                    "selected_cards" => {
                        let selected = self.selected_cards.clone();
                        let idxs: Vec<usize> = (0..selected.len()).collect();
                        match classify_selection(
                            &idxs,
                            count,
                            is_all,
                            InsufficientBehavior::Silent,
                        )? {
                            SelectionOutcome::Exact(indices) => {
                                let taken: Vec<i16> =
                                    indices.iter().map(|&i| selected[i]).collect();
                                for &card_id in &taken {
                                    remove_card_from_any_zone(
                                        player,
                                        &mut self.game_state.last_vacated_stage_area,
                                        card_id,
                                    );
                                    moved_cards.push(card_id);
                                }
                                taken
                            }
                            SelectionOutcome::Prompt => {
                                let filter = util::CardFilter::from_effect(effect);
                                self.prompt_card_selection(
                                    "selected_cards",
                                    count,
                                    false,
                                    effect,
                                    &filter,
                                );
                                return Ok(());
                            }
                            SelectionOutcome::Skip => vec![],
                        }
                    }
                    "revealed_cards" => {
                        let is_self = tgt.as_ref().map(|s| s.as_str()).unwrap_or("self") == "self";
                        let cards: Vec<i16> = {
                            let cheer = if is_self {
                                &mut self.game_state.player1_cheer_revealed_cards
                            } else {
                                &mut self.game_state.player2_cheer_revealed_cards
                            };
                            if !cheer.is_empty() {
                                cheer.drain(..).collect()
                            } else {
                                self.game_state.revealed_cards.drain(..).collect()
                            }
                        };
                        if cards.len() > count {
                            let cheer = if is_self {
                                &mut self.game_state.player1_cheer_revealed_cards
                            } else {
                                &mut self.game_state.player2_cheer_revealed_cards
                            };
                            for &c in &cards {
                                cheer.push(c);
                                self.game_state.revealed_cards.push(c);
                            }
                            self.pending_choice = Some(
                                Choice::select_cards(
                                    "revealed_cards",
                                    count,
                                    format!("Select {} card(s) from revealed cards", count),
                                    false,
                                )
                                .card_type(card_type_filter.map(|s| s.to_string()))
                                .cost_limit(cost_limit, effect.cost_limit_operator.clone())
                                .group(group_name.map(|s| s.to_string()))
                                .characters(character_filter.clone())
                                .build(),
                            );
                            self.execution_context =
                                ExecutionContext::SingleEffect { effect_index: 0 };
                            return Ok(());
                        }
                        for &cid in &cards {
                            if let Some(idx) = player.hand.cards.iter().position(|&c| c == cid) {
                                player.hand.cards.remove(idx);
                            }
                        }
                        cards
                    }
                    "under_member" => {
                        // Move from under_member to destination needs a choice.
                        // Delegate to place_energy_under_member for choice creation.
                        self.execute_place_energy_under_member(
                            count as u32,
                            effect.target_name(),
                            effect.position.as_ref(),
                            effect.optional.unwrap_or(false),
                            Some("under_member"),
                        );
                        return Ok(());
                    }
                    _ => {
                        return Err(format!("Unknown source zone: {}", source));
                    }
                }
            };

            // --- STEP 2: Any-order deck placement (before consuming taken) ---
            if source == "discard"
                && destination == "deck"
                && effect.placement_order.as_deref() == Some("any_order")
                && taken.len() > 1
            {
                let taken_count = taken.len();
                for &c in &taken {
                    moved_cards.push(c);
                }
                self.looked_at_cards = taken.clone();
                self.pending_choice = Some(Choice::SelectTarget {
                    target: "order".to_string(),
                    description: format!("Choose order for cards on deck ({} cards)", taken_count),
                    allow_skip: false,
                    options: None,
                });
                self.execution_context = ExecutionContext::LookAndSelect {
                    step: LookAndSelectStep::Finalize {
                        destination: "deck".to_string(),
                    },
                };
                return Ok(());
            }

            // Apply distinct card name filter if specified
            let distinct = effect.distinct.as_deref();
            if distinct == Some("card_name")
                || distinct == Some("true")
                || distinct == Some("distinct")
            {
                let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
                taken.retain(|&id| {
                    card_db
                        .get_card(id)
                        .map(|c| seen.insert(c.name.clone()))
                        .unwrap_or(true)
                });
                if taken.len() < count {
                    taken.clear(); // Not enough distinct cards — skip
                }
            }

            // --- STEP 3: Place cards in destination ---
            let deck_pos = effect
                .position
                .as_ref()
                .and_then(|p| match p {
                    crate::card::PositionInfo::String(s) => s.parse::<usize>().ok(),
                    crate::card::PositionInfo::Struct { position, .. } => {
                        position.as_ref().and_then(|s| s.parse::<usize>().ok())
                    }
                })
                .map(|p| if p > 0 { p - 1 } else { 0 });
            // Pre-flight: if destination is stage and no room, silently return cards to source
            if destination == "stage" && player.stage.stage.iter().all(|&id| id != -1) {
                eprintln!(
                    "[MOVE_CARDS] stage is full, returning {} cards to discard",
                    taken.len()
                );
                for &card_id in &taken {
                    player.waitroom.add_card(card_id);
                }
                moved_cards.extend(taken);
            } else {
                for card_id in &taken {
                    if destination == "deck" && !is_max {
                        if let Some(pos) = deck_pos {
                            let clamped = pos.min(player.main_deck.cards.len());
                            player.main_deck.cards.insert(clamped, *card_id);
                        } else {
                            player.main_deck.cards.insert(0, *card_id);
                        }
                    } else if destination == "empty_area" {
                        // Check if multiple empty slots exist for position choice
                        let empty_slots: Vec<usize> =
                            (0..3).filter(|&i| player.stage.stage[i] == -1).collect();
                        if empty_slots.len() > 1 {
                            // Create position choice
                            self.pending_choice =
                                Some(crate::ability::types::Choice::SelectPosition {
                                    position: "center".to_string(), // Default position
                                    description: format!(
                                        "Choose position for {}",
                                        self.game_state
                                            .card_database
                                            .get_card(*card_id)
                                            .map(|c| &c.name)
                                            .map_or("card", |v| v)
                                    ),
                                    allow_skip: false,
                                });
                            let pos_target = tgt.clone().unwrap_or_else(|| "self".to_string());
                            self.execution_context =
                                crate::ability::types::ExecutionContext::MoveCardsPosition {
                                    card_id: *card_id,
                                    state_change: effect.state_change.clone(),
                                    target: pos_target,
                                };
                            return Ok(());
                        } else {
                            // Single empty slot - place directly
                            util::place_card_in_zone(
                                player,
                                *card_id,
                                destination.as_str(),
                                vacated_stage_area,
                                is_max,
                                count,
                            );
                        }
                    } else {
                        // For "both" targeting or other cases, use normal placement
                        util::place_card_in_zone(
                            player,
                            *card_id,
                            destination.as_str(),
                            vacated_stage_area,
                            is_max,
                            count,
                        );
                    }
                    moved_cards.push(*card_id);
                }
            }
        }

        self.finalize_card_movement(
            &moved_cards,
            &destination,
            &effect.state_change,
            tgt.as_deref(),
        );
        Ok(())
    }

    pub fn handle_select_position(
        &mut self,
        position: &str,
        context: ExecutionContext,
    ) -> Result<(), String> {
        match &context {
            ExecutionContext::LookAndSelect { step } => {
                if let LookAndSelectStep::Finalize { destination } = step {
                    if destination == "stage" {
                        if let Some(&card_id) = self.looked_at_cards.last() {
                            let player = &mut self.game_state.player1;
                            let pos_idx = match position {
                                "center" => Some(1usize),
                                "left_side" | "left" => Some(0),
                                "right_side" | "right" => Some(2),
                                _ => None,
                            };
                            match pos_idx {
                                Some(0) => {
                                    player.stage.stage[0] = card_id;
                                    player
                                        .areas_locked_this_turn
                                        .insert(zones::MemberArea::LeftSide);
                                }
                                Some(1) => {
                                    player.stage.stage[1] = card_id;
                                    player
                                        .areas_locked_this_turn
                                        .insert(zones::MemberArea::Center);
                                }
                                Some(2) => {
                                    player.stage.stage[2] = card_id;
                                    player
                                        .areas_locked_this_turn
                                        .insert(zones::MemberArea::RightSide);
                                }
                                _ => {
                                    player.hand.add_card(card_id);
                                }
                            }
                            self.looked_at_cards.clear();
                        }
                    }
                }
            }
            ExecutionContext::MoveCardsPosition {
                card_id,
                state_change,
                target,
            } => {
                let card_id = *card_id;
                let state_change = state_change.clone();
                let target = target.clone();

                let pos_idx = match position {
                    "center" => Some(1usize),
                    "left_side" | "left" => Some(0),
                    "right_side" | "right" => Some(2),
                    _ => None,
                };

                let player = self.game_state.resolve_target_player_mut(&target);
                match pos_idx {
                    Some(idx) if idx < 3 && player.stage.stage[idx] == -1 => {
                        player.stage.stage[idx] = card_id;
                        let area = match idx {
                            0 => zones::MemberArea::LeftSide,
                            1 => zones::MemberArea::Center,
                            _ => zones::MemberArea::RightSide,
                        };
                        player.areas_locked_this_turn.insert(area);
                    }
                    _ => {
                        player.hand.add_card(card_id);
                    }
                }

                self.game_state.mods.clear_all_for_card(card_id);
                self.game_state.record_card_movement(card_id);
                if state_change.as_deref() == Some("wait") {
                    self.game_state
                        .mods
                        .add_orientation_modifier(card_id, "wait");
                }
            }
            _ => {}
        }
        self.pending_choice = None;
        self.execution_context = ExecutionContext::None;
        self.resume_pending_commands()?;
        Ok(())
    }

    fn stage_remove_with_vacated(
        player: &mut crate::player::Player,
        idxs: &[usize],
        card_db: &crate::card::CardDatabase,
    ) -> (Vec<i16>, Option<usize>) {
        let mut vacated = None;
        let cards: Vec<i16> = idxs
            .iter()
            .rev()
            .filter_map(|&i| {
                let cid = player.remove_member_from_stage_with_recycling(i, card_db);
                if cid.is_some() {
                    vacated = Some(i);
                }
                cid
            })
            .collect();
        (cards, vacated)
    }

    /// Apply post-move side effects: clear_all_for_card, state_change, record_card_movement, tracking.
    fn finalize_card_movement(
        &mut self,
        moved_cards: &[i16],
        destination: &str,
        state_change: &Option<String>,
        target: Option<&str>,
    ) {
        for card_id in moved_cards {
            self.game_state.mods.clear_all_for_card(*card_id);
        }

        if let Some(ref sc) = state_change {
            if sc == "wait" {
                for card_id in moved_cards {
                    self.game_state
                        .mods
                        .add_orientation_modifier(*card_id, "wait");
                }
                if destination == "energy_zone" {
                    let p = match target.unwrap_or("self") {
                        "self" => &mut self.game_state.player1,
                        "opponent" => &mut self.game_state.player2,
                        _ => &mut self.game_state.player1,
                    };
                    for _ in moved_cards {
                        p.energy_zone.active_energy_count =
                            p.energy_zone.active_energy_count.saturating_sub(1);
                    }
                }
            } else if sc == "active" {
                for card_id in moved_cards {
                    self.game_state
                        .mods
                        .add_orientation_modifier(*card_id, "active");
                }
                if destination == "energy_zone" {
                    let p = match target.unwrap_or("self") {
                        "self" => &mut self.game_state.player1,
                        "opponent" => &mut self.game_state.player2,
                        _ => &mut self.game_state.player1,
                    };
                    p.energy_zone.active_energy_count += moved_cards.len();
                }
            }
        }

        for card_id in moved_cards {
            self.game_state.record_card_movement(*card_id);
        }

        if destination == "discard" {
            self.moved_cards = moved_cards.to_vec();
            self.game_state.recently_moved_cards = Some(moved_cards.to_vec());
        }

        if !moved_cards.is_empty() {
            self.game_state.last_area_move_card_id = moved_cards.last().copied();
            self.game_state.last_area_move_by_player = self
                .game_state
                .ability_queue
                .current_entry()
                .map(|e| e.player_id.clone());
        }
    }

    /// Discard cards from hand to waitroom, tracking last_cost_discard_count.
    pub fn discard_from_hand(
        &mut self,
        indices: &[usize],
        validate_card: &mut impl FnMut(i16) -> bool,
    ) -> u32 {
        let player = self.game_state.active_player_mut();
        let mut count = 0u32;
        for &idx in indices.iter().rev() {
            if idx < player.hand.cards.len() && validate_card(player.hand.cards[idx]) {
                player.waitroom.add_card(player.hand.cards[idx]);
                player.hand.remove_card(idx);
                count += 1;
            }
        }
        if count > 0 {
            self.game_state.mods.last_cost_discard_count = count;
        }
        count
    }

    /// Remove members from stage with under-card recycling, tracking vacated position and moved cards.
    pub fn remove_from_stage(
        &mut self,
        indices: &[usize],
        validate_card: &mut impl FnMut(i16) -> bool,
        card_db: &CardDatabase,
    ) -> (Vec<i16>, Option<usize>) {
        let mut last_vacated = None;
        let mut moved_ids = Vec::new();
        {
            let player = self.game_state.active_player_mut();
            for &idx in indices.iter().rev() {
                if idx < 3
                    && player.stage.stage[idx] != -1
                    && validate_card(player.stage.stage[idx])
                {
                    if let Some(card_id) =
                        player.remove_member_from_stage_with_recycling(idx, card_db)
                    {
                        player.waitroom.add_card(card_id);
                        last_vacated = Some(idx);
                        moved_ids.push(card_id);
                    }
                }
            }
        }
        if let Some(pos) = last_vacated {
            self.game_state.last_vacated_stage_area = Some(pos);
        }
        if !moved_ids.is_empty() {
            self.moved_cards = moved_ids.clone();
            self.game_state.recently_moved_cards = Some(moved_ids.clone());
        }
        (moved_ids, last_vacated)
    }

    /// Mark selected energy zone cards as wait state.
    pub fn mark_energy_as_wait(
        &mut self,
        indices: &[usize],
        validate_card: &mut impl FnMut(i16) -> bool,
    ) {
        let to_mark: Vec<i16> = {
            let player = self.game_state.active_player_mut();
            indices
                .iter()
                .filter_map(|&idx| {
                    if idx < player.energy_zone.cards.len()
                        && validate_card(player.energy_zone.cards[idx])
                    {
                        Some(player.energy_zone.cards[idx])
                    } else {
                        None
                    }
                })
                .collect()
        };
        for cid in to_mark {
            self.game_state.mods.clear_all_for_card(cid);
            self.game_state.mods.add_orientation_modifier(cid, "wait");
        }
    }

    /// Move cards from revealed_cards to a destination zone.
    pub fn move_from_revealed(
        &mut self,
        indices: &[usize],
        validate_card: &mut impl FnMut(i16) -> bool,
        dst: &str,
    ) -> Vec<i16> {
        let cards: Vec<i16> = {
            let revealed = &mut self.game_state.revealed_cards;
            let mut result = Vec::new();
            let mut removed: Vec<usize> = indices.to_vec();
            removed.sort_by(|a, b| b.cmp(a));
            for &i in &removed {
                if i < revealed.len() {
                    let cid = revealed.remove(i);
                    if validate_card(cid) {
                        result.push(cid);
                    }
                }
            }
            result
        };
        self.selected_cards = cards.clone();
        let player = self.game_state.active_player_mut();
        for &cid in &cards {
            util::place_card_in_zone(player, cid, dst, None, false, 1);
        }
        cards
    }

    /// Move cards from under_member to a destination zone, using flat 3-position indexing.
    pub fn move_from_under_member(
        &mut self,
        indices: &[usize],
        validate_card: &mut impl FnMut(i16) -> bool,
        dst: &str,
    ) -> Result<Vec<i16>, String> {
        let player = self.game_state.active_player_mut();
        let mut cards_to_move: Vec<(usize, i16)> = Vec::new();
        for &idx in indices.iter() {
            let mut global_idx = 0;
            let mut found = false;
            for si in 0..3 {
                if idx < global_idx + player.stage.under_cards[si].len() {
                    let card_id = player.stage.under_cards[si][idx - global_idx];
                    if !validate_card(card_id) {
                        return Err(format!(
                            "Card {:?} does not match required type filter for under_member selection",
                            card_id
                        ));
                    }
                    cards_to_move.push((si, card_id));
                    found = true;
                    break;
                }
                global_idx += player.stage.under_cards[si].len();
            }
            if !found {
                return Err(format!("Card at index {} not found in under_member", idx));
            }
        }
        let selected_ids: Vec<i16> = cards_to_move.iter().map(|&(_, cid)| cid).collect();
        let _ = player;
        let player = self.game_state.active_player_mut();
        for (si, card_id) in &cards_to_move {
            if let Some(pos) = player.stage.under_cards[*si]
                .iter()
                .position(|&c| c == *card_id)
            {
                player.stage.under_cards[*si].remove(pos);
                util::place_card_in_zone(player, *card_id, dst, None, false, 1);
            }
        }
        self.selected_cards = selected_ids;
        Ok(cards_to_move.iter().map(|&(_, cid)| cid).collect())
    }

    /// Execute card movement from a zone: pre-validate filters, move cards to destination, track side effects.
    pub fn execute_selected_cards_from_zone(
        &mut self,
        zone: &str,
        indices: &[usize],
        _count: usize,
        card_type_filter: Option<&str>,
        cost_limit: Option<u32>,
        cost_limit_operator: Option<&str>,
        group: Option<&str>,
        characters: Option<&Vec<String>>,
    ) -> Result<(), String> {
        eprintln!("[EXEC_ZONE] enter zone={} indices={:?}", zone, indices);
        let destination = self.game_state.entry_destination().map(|s| s.to_string());
        let target = self
            .game_state
            .entry_effect()
            .and_then(|e| e.target.clone())
            .unwrap_or_else(|| "self".to_string());
        let card_db = self.game_state.card_database.clone();
        let vacated_area = self.game_state.last_vacated_stage_area;
        let player = self.game_state.resolve_target_player_mut(&target);

        let mut moved: Vec<i16> = Vec::new();

        let passes = |cid: i16| -> bool {
            util::card_matches_type(&card_db, cid, card_type_filter)
                && util::card_matches_cost_limit_op(&card_db, cid, cost_limit, cost_limit_operator)
                && util::card_matches_group_str(&card_db, cid, group)
                && match characters {
                    Some(chars) if !chars.is_empty() => {
                        util::card_matches_characters(&card_db, cid, Some(chars))
                    }
                    _ => true,
                }
        };

        match zone {
            "hand" => {
                for &i in indices {
                    if i < player.hand.cards.len() && !passes(player.hand.cards[i]) {
                        return Err(
                            "Selected hand card does not match required filters".to_string()
                        );
                    }
                }
                let dest = destination.as_deref().unwrap_or("discard");
                let mut idxs: Vec<usize> = indices.iter().copied().collect();
                idxs.sort_by(|a, b| b.cmp(a));
                let mut moved_cards: Vec<i16> = Vec::new();
                for i in idxs {
                    if i < player.hand.cards.len() {
                        let card_id = player.hand.cards.remove(i);
                        if passes(card_id) {
                            match dest {
                                "stage" | "empty_area" => {
                                    if player.stage.stage[1] == -1 {
                                        player.stage.stage[1] = card_id;
                                        player
                                            .areas_locked_this_turn
                                            .insert(zones::MemberArea::Center);
                                    } else if player.stage.stage[0] == -1 {
                                        player.stage.stage[0] = card_id;
                                        player
                                            .areas_locked_this_turn
                                            .insert(zones::MemberArea::LeftSide);
                                    } else if player.stage.stage[2] == -1 {
                                        player.stage.stage[2] = card_id;
                                        player
                                            .areas_locked_this_turn
                                            .insert(zones::MemberArea::RightSide);
                                    } else {
                                        player.hand.add_card(card_id);
                                    }
                                }
                                "same_area" => {
                                    util::place_card_in_zone(
                                        player,
                                        card_id,
                                        "same_area",
                                        vacated_area,
                                        false,
                                        1,
                                    );
                                }
                                _ => player.waitroom.add_card(card_id),
                            }
                            moved_cards.push(card_id);
                        } else {
                            player.hand.cards.insert(i, card_id);
                        }
                    }
                }
                if dest == "discard" || dest == "waitroom" {
                    let count = moved_cards.len() as u32;
                    eprintln!("[DISCARD_TRACK] setting last_cost_discard_count={}", count);
                    self.game_state.mods.last_cost_discard_count = count;
                }
                if !moved_cards.is_empty() {
                    self.game_state.recently_moved_cards = Some(moved_cards.clone());
                }
            }
            "deck" => {
                for &i in indices {
                    if i < player.main_deck.cards.len() && !passes(player.main_deck.cards[i]) {
                        return Err(
                            "Selected deck card does not match required filters".to_string()
                        );
                    }
                }
                let mut idxs: Vec<usize> = indices.iter().copied().collect();
                idxs.sort_by(|a, b| b.cmp(a));
                for i in idxs {
                    if i < player.main_deck.cards.len() {
                        let card_id = player.main_deck.cards.remove(i);
                        if passes(card_id) {
                            player.hand.add_card(card_id);
                            moved.push(card_id);
                        } else {
                            player.main_deck.cards.insert(i, card_id);
                        }
                    }
                }
            }
            "discard" => {
                let dest = destination.as_deref().unwrap_or("hand");
                if dest == "stage" && player.stage.stage.iter().all(|&id| id != -1) {
                    eprintln!("[DISCARD_ZONE] stage is full, cannot place cards from discard");
                    return Ok(());
                }
                for &i in indices {
                    if i < player.waitroom.cards.len() && !passes(player.waitroom.cards[i]) {
                        return Err(
                            "Selected discard card does not match required filters".to_string()
                        );
                    }
                }
                let mut idxs: Vec<usize> = indices.iter().copied().collect();
                idxs.sort_by(|a, b| b.cmp(a));
                if let Some(limit) = cost_limit {
                    let total_cost: u32 = idxs
                        .iter()
                        .filter_map(|&i| {
                            if i < player.waitroom.cards.len() {
                                card_db
                                    .get_card(player.waitroom.cards[i])
                                    .and_then(|c| c.cost)
                            } else {
                                None
                            }
                        })
                        .sum();
                    let op = cost_limit_operator.unwrap_or("<=");
                    let ok = match op {
                        ">=" => total_cost >= limit,
                        ">" => total_cost > limit,
                        "<" => total_cost < limit,
                        "exact" | "=" => total_cost == limit,
                        _ => total_cost <= limit,
                    };
                    if !ok {
                        return Ok(());
                    }
                }
                let mut card_ids_moved: Vec<i16> = Vec::new();
                for i in idxs {
                    if i < player.waitroom.cards.len() {
                        let card_id = player.waitroom.cards.remove(i);
                        if passes(card_id) {
                            match dest {
                                "stage" | "empty_area" => {
                                    if player.stage.stage[1] == -1 {
                                        player.stage.stage[1] = card_id;
                                        player
                                            .areas_locked_this_turn
                                            .insert(zones::MemberArea::Center);
                                    } else if player.stage.stage[0] == -1 {
                                        player.stage.stage[0] = card_id;
                                        player
                                            .areas_locked_this_turn
                                            .insert(zones::MemberArea::LeftSide);
                                    } else if player.stage.stage[2] == -1 {
                                        player.stage.stage[2] = card_id;
                                        player
                                            .areas_locked_this_turn
                                            .insert(zones::MemberArea::RightSide);
                                    } else {
                                        player.waitroom.cards.insert(i, card_id);
                                        continue;
                                    }
                                }
                                "same_area" => {
                                    util::place_card_in_zone(
                                        player,
                                        card_id,
                                        "same_area",
                                        vacated_area,
                                        false,
                                        1,
                                    );
                                }
                                _ => player.hand.add_card(card_id),
                            }
                            card_ids_moved.push(card_id);
                            moved.push(card_id);
                        } else {
                            player.waitroom.cards.insert(i, card_id);
                        }
                    }
                }
                let state_change = self
                    .game_state
                    .ability_queue
                    .current_entry()
                    .and_then(|e| e.ability.effect.as_ref())
                    .and_then(|ef| ef.state_change.clone());
                if let Some(sc) = state_change {
                    if sc == "wait" {
                        for &cid in &card_ids_moved {
                            self.game_state.mods.add_orientation_modifier(cid, "wait");
                        }
                    }
                }
                if !card_ids_moved.is_empty() {
                    self.game_state.recently_moved_cards = Some(card_ids_moved.clone());
                }
            }
            "stage" => {
                for &idx in indices {
                    if idx < 3 && player.stage.stage[idx] != -1 && !passes(player.stage.stage[idx])
                    {
                        return Err(
                            "Selected stage card does not match required filters".to_string()
                        );
                    }
                    if idx < 3 && player.stage.stage[idx] != -1 {
                        self.selected_cards.push(player.stage.stage[idx]);
                    }
                }
            }
            "revealed_cards" => {
                for &idx in indices {
                    if idx < self.game_state.revealed_cards.len()
                        && !passes(self.game_state.revealed_cards[idx])
                    {
                        return Err(
                            "Selected revealed card does not match required filters".to_string()
                        );
                    }
                }
                for &idx in indices.iter().rev() {
                    if idx < self.game_state.revealed_cards.len() {
                        let card_id = self.game_state.revealed_cards.remove(idx);
                        if passes(card_id) {
                            self.selected_cards.push(card_id);
                        }
                    }
                }
            }
            _ => return Err(format!("Unknown zone: {}", zone)),
        }
        for cid in moved {
            self.game_state.mods.clear_all_for_card(cid);
        }
        Ok(())
    }

    /// Handle looked_at card selection: validate, move to destination, handle multi-select and remaining cards.
    pub fn handle_select_cards_looked_at(&mut self, indices: &[usize]) -> Result<(), String> {
        let select_action = self
            .game_state
            .ability_queue
            .current_entry()
            .and_then(|e| e.ability.effect.as_ref())
            .and_then(|ef| ef.compound.select_action.clone());
        let (destination, discard_remaining, placement_order) = (
            select_action
                .as_ref()
                .and_then(|sa| sa.destination.clone())
                .unwrap_or_else(|| "hand".to_string()),
            select_action
                .as_ref()
                .and_then(|sa| sa.discard_remaining)
                .unwrap_or(true),
            select_action
                .as_ref()
                .and_then(|sa| sa.placement_order.clone()),
        );

        if self.game_state.looked_at_cards.is_empty() && !self.selected_cards.is_empty() {
            self.game_state.looked_at_cards = self.selected_cards.clone();
        }

        let looked_at = &mut self.game_state.looked_at_cards;
        let mut indices_sorted: Vec<usize> = indices.iter().copied().collect();
        indices_sorted.sort_by(|a, b| b.cmp(a));

        let mut selected_cards: Vec<i16> = Vec::new();
        for i in indices_sorted {
            if i < looked_at.len() {
                selected_cards.insert(0, looked_at.remove(i));
            }
        }
        let selected_count = selected_cards.len();

        {
            let card_db = &self.game_state.card_database;
            let cost_limit = select_action.as_ref().and_then(|sa| sa.cost_limit);
            let cost_limit_operator = select_action
                .as_ref()
                .and_then(|sa| sa.cost_limit_operator.as_deref());
            selected_cards.retain(|&cid| {
                util::card_matches_cost_limit_op(card_db, cid, cost_limit, cost_limit_operator)
            });
        }

        let remaining_cards: Vec<i16> = looked_at.drain(..).collect();

        let is_deck_dest = destination == "deck_top" || destination == "deck";
        let needs_order = is_deck_dest
            && placement_order.as_deref() == Some("any_order")
            && selected_cards.len() > 1;

        if needs_order {
            self.looked_at_cards = selected_cards;
            let player = self.game_state.active_player_mut();
            if discard_remaining {
                for card_id in remaining_cards {
                    player.waitroom.add_card(card_id);
                }
            } else {
                for card_id in remaining_cards {
                    player.main_deck.cards.push(card_id);
                }
            }
            let card_count = self.looked_at_cards.len();
            self.pending_choice = Some(Choice::SelectTarget {
                target: "order".to_string(),
                description: format!("Choose order for cards on deck ({} cards)", card_count),
                allow_skip: false,
                options: None,
            });
            self.execution_context = ExecutionContext::LookAndSelect {
                step: LookAndSelectStep::Finalize {
                    destination: "deck".to_string(),
                },
            };
            return Ok(());
        }

        let player = self.game_state.active_player_mut();
        for card_id in selected_cards {
            match destination.as_str() {
                "hand" => player.hand.add_card(card_id),
                "deck_top" | "deck" => player.main_deck.cards.insert(0, card_id),
                "discard" => player.waitroom.add_card(card_id),
                _ => player.hand.add_card(card_id),
            }
        }
        let _ = player;

        let any_number = select_action
            .as_ref()
            .and_then(|sa| sa.any_number)
            .unwrap_or(false);
        let json_count = select_action.as_ref().and_then(|sa| sa.count).unwrap_or(1) as usize;
        let total_available = selected_count + remaining_cards.len();
        let max_select = if any_number {
            total_available
        } else {
            json_count
        };
        let can_select_more = select_action
            .as_ref()
            .map(|sa| sa.max.unwrap_or(false) || sa.optional.unwrap_or(false) || any_number)
            .unwrap_or(false);

        if selected_count > 0
            && can_select_more
            && max_select > selected_count
            && !remaining_cards.is_empty()
        {
            self.looked_at_cards = remaining_cards.clone();
            self.game_state.looked_at_cards = remaining_cards.clone();
            if let Some(entry) = self.game_state.ability_queue.current_entry_mut() {
                entry.selected_card_ids = remaining_cards;
            }
            let remaining_available = self.game_state.looked_at_cards.len();
            let remaining_selections = (max_select - selected_count).min(remaining_available);
            let description = format!(
                "Select up to {} more card(s) from the {} remaining looked-at cards",
                remaining_selections, remaining_available
            );
            self.pending_choice = Some(
                Choice::select_cards("looked_at", remaining_selections, description, true)
                    .card_type(select_action.as_ref().and_then(|sa| sa.card_type.clone()))
                    .cost_limit(
                        select_action.as_ref().and_then(|sa| sa.cost_limit),
                        select_action
                            .as_ref()
                            .and_then(|sa| sa.cost_limit_operator.clone()),
                    )
                    .group(
                        select_action
                            .as_ref()
                            .and_then(|sa| sa.group_names.as_ref())
                            .and_then(|v| v.first().cloned()),
                    )
                    .characters(select_action.as_ref().and_then(|sa| sa.characters.clone()))
                    .build(),
            );
            return Ok(());
        }

        if discard_remaining {
            for card_id in remaining_cards {
                self.game_state
                    .active_player_mut()
                    .waitroom
                    .add_card(card_id);
            }
        } else {
            for card_id in remaining_cards {
                self.game_state
                    .active_player_mut()
                    .main_deck
                    .cards
                    .push(card_id);
            }
        }

        self.looked_at_cards = self.game_state.looked_at_cards.clone();
        Ok(())
    }

    /// Execute energy zone cards: move to wait state.
    pub fn execute_selected_energy_zone_cards(
        &mut self,
        indices: &[usize],
        _count: usize,
    ) -> Result<(), String> {
        let player = self.game_state.resolve_target_player_mut("self");
        let mut to_mark: Vec<i16> = Vec::new();
        for &i in indices {
            if i < player.energy_zone.cards.len() {
                to_mark.push(player.energy_zone.cards[i]);
            }
        }
        let deactivated_count = indices.len();
        if player.energy_zone.active_energy_count >= deactivated_count {
            player.energy_zone.active_energy_count -= deactivated_count;
        }
        let _ = player;
        for cid in to_mark {
            self.game_state.mods.clear_all_for_card(cid);
            self.game_state.mods.add_orientation_modifier(cid, "wait");
        }
        Ok(())
    }
}
