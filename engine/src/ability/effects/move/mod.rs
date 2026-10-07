use crate::core::constants::CountCast;
use super::super::enums::Zone;
use super::super::resolver::AbilityResolver;
use super::super::types::{Choice, ExecutionContext, LookAndSelectStep};
use super::super::util;
use crate::ability::queue::ConditionalChoice;
use crate::card::{AbilityEffect, CardDatabase, Operator, PlacementOrder};
use crate::game_state::GameState;

/// The two owning-effect fields `maybe_prompt_deck_order` reads.
///
/// Passed by value rather than as `&AbilityEffect` so the caller can read them
/// off the effect and then take `&mut self` for the call, which it cannot do
/// while a borrow of the effect tree is live. The effect is a decoded tree that
/// other steps re-point, so the borrow did not survive the call.
#[derive(Clone, Copy, Default)]
pub(crate) struct DeckOrderHint {
    /// 「〜すべて〜」 — the move takes every matching card, not a chosen count.
    pub all_any: Option<bool>,
    /// Explicit deck-placement order; only `AnyOrder` reaches the prompt.
    pub placement_order: Option<PlacementOrder>,
}

impl DeckOrderHint {
    pub(crate) fn of(effect: &AbilityEffect) -> Self {
        Self {
            all_any: effect.all_any(),
            placement_order: effect.placement_order_any(),
        }
    }
}

/// Where a move's cards come from and go to, and for which side.
///
/// These three are the routing decision itself: which zone to read, where the
/// cards land, and whether the destination is the acting player or the
/// opponent. They always travel together, so they are one value.
pub(crate) struct MoveRoute<'a> {
    source: &'a str,
    destination: &'a str,
    use_p2: bool,
}

/// How many cards a move takes, and under which of the selection modes.
///
/// The count and the four flags are one decision made at the call site — "take
/// up to three, not the activating card, and this is a cost payment" — not
/// five independent inputs, so they travel as one value.
pub(crate) struct SelectionSpec {
    count: usize,
    /// 「最大で」 — up to `count`, offering a choice rather than demanding.
    is_max: bool,
    /// 「すべて」 — every matching card, ignoring `count`.
    is_all: bool,
    /// Paying a cost rather than moving: resolves against the paying player
    /// and cannot pick the card that is paying.
    is_self_cost: bool,
    exclude_self: bool,
    activating_card_id: Option<i16>,
}
use crate::player::Player;
#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};

mod handlers;
mod placement;
mod selection;
pub(crate) mod source;
mod transport;

pub(crate) use transport::drain_under_cards_to_energy_zone;
use placement::log_move_result;

#[derive(Debug, Clone, PartialEq)]
pub enum MoveCardsTarget {
    PlayerSelf,
    Opponent,
}

/// Bundles the inputs to `resolve_from_zone` so per-zone resolution methods
/// have a small signature instead of 21 positional arguments. Holds only
/// borrows / `Copy` values, so the whole struct is freely reconstructable.
pub(crate) struct MoveSourceContext<'a> {
    effective_source: &'a str,
    source_str: &'a str,
    count: usize,
    effect: &'a AbilityEffect,
    card_type_filter: Option<&'a str>,
    group_name: Option<&'a str>,
    cost_limit: Option<u8>,
    cost_total: Option<u8>,
    cost_total_operator: Option<&'a str>,
    character_filter: Option<&'a Vec<String>>,
    name_fragments: Option<&'a Vec<String>>,
    is_self_cost: bool,
    is_max: bool,
    is_all: bool,
    exclude_self: bool,
    activating_card_id: Option<i16>,
    use_p2: bool,
    destination: &'a str,
    card_db: &'a crate::card::CardDatabase,
}

impl<'a> MoveSourceContext<'a> {
    /// The player whose zones this move operates on. Replaces the
    /// `let player = if c.use_p2 {...}` pick previously copy-pasted at every
    /// resolve_from_* function.
    fn player_mut<'gs>(&self, gs: &'gs mut GameState) -> &'gs mut Player {
        if self.use_p2 {
            &mut gs.player2
        } else {
            &mut gs.player1
        }
    }
}


impl AbilityResolver {
    /// Parse the 1-based deck position from the effect into a 0-based index.
    /// Unifies the two copy-pasted `PositionInfo` parses (spawn context + placement).
    fn parse_effect_deck_pos(effect: &AbilityEffect) -> Option<usize> {
        effect
            .position_any()
            .as_ref()
            .and_then(|p| match p {
                crate::card::PositionInfo::String(s) => s.parse::<usize>().ok(),
                crate::card::PositionInfo::Struct { position, .. } => {
                    position.as_ref().and_then(|s| s.parse::<usize>().ok())
                }
            })
            .map(|p| if p > 0 { p - 1 } else { 0 })
    }

    /// Resolve `or_card_types` ("pick which type to search for"). Returns the
    /// chosen card-type filter, or `None` + `Ok(false)` when a choice was
    /// issued and the caller must return early.
    pub(crate) fn resolve_or_card_type(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(Option<String>, bool), String> {
        let Some(or_types) = effect.or_card_types_any() else {
            return Ok((effect.card_type_any().map(|s| s.to_string()), true));
        };
        if or_types.is_empty() {
            return Ok((effect.card_type_any().map(|s| s.to_string()), true));
        }
        let chosen = gs
            .ability_queue
            .current_entry()
            .and_then(|e| e.conditional_choice.clone());
        match chosen {
            Some(ConditionalChoice::Str(s)) => Ok((Some(s), true)),
            _ => {
                let type_labels: Vec<String> = or_types
                    .iter()
                    .map(|t| crate::ability::describe::card_type_label(Some(t)).to_string())
                    .collect();
                self.awaiting.choice = Some(Choice::SelectTarget {
                    target: "choice_string".to_string(),
                    description: format!("Pick card type: {}", type_labels.join(" / ")),
                    description_en: Some(format!(
                        "Pick card type: {}",
                        type_labels.join(" / ")
                    )),
                    description_ja: Some(format!(
                        "カードタイプを選択: {}",
                        type_labels.join(" / ")
                    )),
                    allow_skip: false,
                    options: Some(type_labels),
                });
                self.in_flight.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                if let Some(e) = gs.ability_queue.current_entry_mut() {
                    e.conditional_choice =
                        Some(ConditionalChoice::Strings(or_types.to_vec()));
                }
                Ok((None, false))
            }
        }
    }

    /// Resolve `name_constraint == contains_all` from revealed cards into name fragments.
    fn resolve_name_fragments(gs: &GameState, effect: &AbilityEffect) -> Option<Vec<String>> {
        if effect.name_constraint_any() != Some("contains_all")
            || effect.name_constraint_source_any() != Some("revealed_card")
        {
            return None;
        }
        // Fragments must be in `name_key` form: `card_matches_name_fragments`
        // compares them against the candidate's `name_key`, which is whitespace-
        // free. A raw `card.name` keeps its spaces ("Dream Believers"), so it was
        // never a substring of "DreamBelievers（104期Ver.）" and Q236 recovery
        // silently matched nothing. `get_card_names` is exactly `name_key` split
        // on '&', i.e. the canonical needle the matcher already expects.
        let fragments: Vec<String> = gs
            .revealed_cost_cards
            .iter()
            .chain(gs.revealed_cards.iter())
            .flat_map(|&id| gs.card_database.get_card_names(id))
            .collect();
        if fragments.is_empty() {
            None
        } else {
            Some(fragments)
        }
    }

    /// Deck-order prompt for multi-card moves to the deck with AnyOrder.
    /// Returns true when the prompt was issued (caller must return early).
    pub(crate) fn maybe_prompt_deck_order(
        &mut self,
        gs: &mut GameState,
        hint: DeckOrderHint,
        taken: &[i16],
        source: &str,
        destination: &str,
        moved_cards: &mut Vec<i16>,
    ) -> bool {
        let is_deck_dest = Zone::from_str(destination) == Some(Zone::Deck)
            || Zone::from_str(destination) == Some(Zone::DeckTop)
            // 「3枚まで好きな順番でデッキの下に置く」 is a deck placement too. Without
            // DeckBottom here the player's choice of order was silently dropped and
            // the cards landed in selection order instead.
            || Zone::from_str(destination) == Some(Zone::DeckBottom);
        let is_eligible_source = Zone::from_str(source) == Some(Zone::Discard)
            || Zone::from_str(source) == Some(Zone::SelectedCards)
            || Zone::from_str(source) == Some(Zone::Hand)
            || (Zone::from_str(source) == Some(Zone::LookedAt)
                && hint.all_any.unwrap_or(false));
        // This line answers one question: "does this move reach the deck-order
        // prompt?". Log it only when the source/destination pair can, i.e. a
        // multi-card take out of an eligible zone into the deck. Logging it for
        // every move printed ~3.5k lines to find 12 real prompts, and 215 of
        // the survivors were `eligible_source=false deck_dest=false`, which the
        // reader has to re-derive to discard.
        if taken.len() > 1 && is_eligible_source && is_deck_dest {
            log::debug!(
                "[ORDER_CHECK] source={} destination={} placement={:?} taken_len={}",
                source,
                destination,
                hint.placement_order,
                taken.len()
            );
        }
        if !(is_eligible_source
            && is_deck_dest
            && hint.placement_order == Some(PlacementOrder::AnyOrder)
            && taken.len() > 1)
        {
            return false;
        }
        let taken_count = taken.len();
        log::debug!(
            "[ORDER_BEGIN] source={} all={} cards={:?}",
            source,
            hint.all_any.unwrap_or(false),
            taken
        );
        moved_cards.extend(taken.iter().copied());
        let mut order_pool = taken.to_vec();
        if Zone::from_str(source) == Some(Zone::Hand) {
            order_pool.reverse();
        }
        log::debug!(
            "[ORDER_POOL] source={} cards={:?}",
            source,
            order_pool
        );
        gs.looked_at_cards = order_pool.into();
        self.awaiting.choice = Some(Choice::SelectTarget {
            target: "order".to_string(),
            description: format!("Choose order for cards on deck ({} cards)", taken_count),
            description_en: Some(format!(
                "Choose order for cards on deck ({} cards)",
                taken_count
            )),
            description_ja: Some(format!("山札のカード順を選択（{}枚）", taken_count)),
            allow_skip: false,
            options: None,
        });
        self.in_flight.execution_context = ExecutionContext::LookAndSelect {
            step: LookAndSelectStep::Finalize {
                destination: Zone::Deck.to_str().to_string(),
                source_zone: String::new(),
            },
        };
        true
    }

    /// Place taken cards into the destination. Returns `Ok(true)` when a
    /// sub-choice was issued and the caller must return early.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn place_taken_cards(
        &mut self,
        gs: &mut GameState,
        taken: &[i16],
        source: &str,
        destination: &str,
        target: Option<&str>,
        effect: &AbilityEffect,
        use_p2: bool,
        vacated_stage_area: Option<u8>,
        is_max: bool,
        count: usize,
        deck_pos: Option<usize>,
        card_db: &CardDatabase,
        moved_cards: &mut Vec<i16>,
    ) -> Result<bool, String> {
        for &card_id in taken {
            let was_cost_revealed = gs.revealed_cost_cards.contains(&card_id);
            if was_cost_revealed {
                gs.remove_revealed_card(card_id);
            }
            gs.remove_revealed_cost_card(card_id);
        }
        let stage_full = {
            let player = if use_p2 { &gs.player2 } else { &gs.player1 };
            Zone::from_str(destination) == Some(Zone::Stage)
                && !effect.allow_occupied_stage_any().unwrap_or(false)
                && player.stage.stage.iter().all(|&id| id != -1)
        };
        if stage_full {
            log::debug!(
                "[MOVE_CARDS] stage is full, returning {} cards to discard",
                taken.len()
            );
            let player = if use_p2 {
                &mut gs.player2
            } else {
                &mut gs.player1
            };
            for &card_id in taken {
                player.waitroom.add_card(card_id);
                log_move_result(player, card_db, card_id, source, destination);
            }
            moved_cards.extend(taken.iter().copied());
            return Ok(false);
        }
        for &card_id in taken {
            // Check for success zone replacement (e.g. 錯覚CROSSROADS)
            if self.maybe_prompt_success_replacement(
                gs,
                card_id,
                destination,
                target.unwrap_or("self"),
            ) {
                return Ok(true);
            }
            if Zone::from_str(destination) == Some(Zone::Deck) && deck_pos.is_some() && !is_max
            {
                let pos = deck_pos.unwrap_or(0);
                let player = if use_p2 {
                    &mut gs.player2
                } else {
                    &mut gs.player1
                };
                let clamped = pos.min(player.main_deck.cards.len());
                player.main_deck.cards.insert(clamped, card_id);
                log_move_result(player, card_db, card_id, source, destination);
            } else if destination == "deck_top_or_bottom" {
                self.prompt_deck_top_or_bottom(
                    card_id,
                    effect.state_change_any().map(|s| s.to_string()),
                    target.unwrap_or("self").to_string(),
                    source.to_string(),
                    effect.optional.unwrap_or(false),
                );
                return Ok(true);
            } else {
                match self.place_card_with_stage_choice(
                    gs,
                    target.unwrap_or("self"),
                    card_id,
                    destination,
                    vacated_stage_area,
                    is_max,
                    count,
                    effect.state_change_any().map(|s| s.to_string()),
                    deck_pos,
                    source,
                    effect.allow_occupied_stage_any().unwrap_or(false),
                    effect.is_under_self(),
                ) {
                    Ok(true) => {
                        return Ok(true);
                    }
                    Ok(false) => {
                        moved_cards.push(card_id);
                    }
                    Err(_) => {
                        let player = if use_p2 {
                            &mut gs.player2
                        } else {
                            &mut gs.player1
                        };
                        let src_zone = if source == "those_cards" {
                            Zone::Discard.to_str()
                        } else {
                            source
                        };
                        util::place_card_in_zone(player, card_id, src_zone, None, false, 1);
                    }
                }
            }
        }
        Ok(false)
    }

    pub fn execute_move_cards(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        // Multiple-target move to the deck is handled by the dedicated both path.
        if effect.multiple_targets_any().unwrap_or(false)
            && effect.target.as_deref() == Some("deck")
        {
            return self.execute_move_cards_both(gs, effect);
        }
        let count = if let Some(c) = effect.count {
            c as usize
        } else if let Some(dc) = effect.dynamic_count_any() {
            self.resolve_dynamic_count(gs, dc) as usize
        } else {
            0
        };
        let cost_limit = self.resolve_cost_limit_reference(gs, effect)?;
        let cost_total = effect.cost_total_any();
        let cto_binding = effect.cost_total_operator_any();
        let cost_total_operator = cto_binding.as_deref();
        let group_name = effect.group_name();

        // Handle or_card_types: let the player pick which type to search for
        let (card_type_owned, or_resolved) = self.resolve_or_card_type(gs, effect)?;
        if !or_resolved {
            return Ok(());
        }
        let card_type_filter: Option<&str> = card_type_owned.as_deref();
        let tgt = effect.target.clone();
        let is_self_cost = effect.self_cost_any().unwrap_or(false);
        let exclude_self = effect.exclude_self_any().unwrap_or(false);
        let is_max = effect.max.unwrap_or(false);
        let is_all = effect.all_any().unwrap_or(false);
        let card_db = gs.card_database.clone();
        let activating_card_id = gs.activating_card;
        let mut vacated_stage_area = gs.last_vacated_stage_area;
        log::debug!(
            "[MOVE_EFFECT_READ] activating={:?} dest={:?} source={:?} last_vacated={:?} \
             stage={:?}",
            gs.activating_card,
            effect.destination,
            effect.source,
            vacated_stage_area,
            gs.resolve_target_player("self").stage.stage
        );

        // Character name filter from the effect
        let character_filter: Option<Vec<String>> = effect.characters_any().cloned();

        // Resolve name_constraint (e.g. "contains_all" from a revealed card)
        let name_fragments: Option<Vec<String>> = Self::resolve_name_fragments(gs, effect);

        let mut moved_cards: Vec<i16> = Vec::new();
        let source = effect
            .source_any()
            .map(|s| s.to_string())
            .unwrap_or_default();
        let destination = effect
            .destination
            .map(|z| z.to_str().to_string())
            .unwrap_or_default();

        let target = effect.target.as_deref().unwrap_or("self");
        let use_p2 = match target {
            "self" => matches!(
                gs.ability_master_id(),
                Some("player2") | Some("p2")
            ),
            "opponent" => !matches!(
                gs.ability_master_id(),
                Some("player2") | Some("p2")
            ),
            _ => false,
        };
        let baton_arrival_area = if Zone::from_str(&destination) == Some(Zone::UnderMember)
            && source == "discard"
            && effect.is_self_target()
            && effect
                .condition
                .as_ref()
                .and_then(|condition| condition.get_baton_touch_trigger())
                .unwrap_or(false)
        {
            let player = gs.resolve_target_player(target);
            gs.baton_touch_arriving_card_id
                .and_then(|arriving| player.stage.stage.iter().position(|&id| id == arriving))
                .map(|area| area.u8_count())
        } else {
            None
        };
        if vacated_stage_area.is_none() {
            vacated_stage_area = baton_arrival_area;
        }
        // Only the baton-touch path is interesting here. `vacated_stage_area` is
        // Some on essentially every move (measured 893/893), so gating on it
        // logged every move; the baton condition itself is true on 3. Gate on
        // that, and on a resolved arrival area, which is the actual signal.
        let baton_condition = effect
            .condition
            .as_ref()
            .and_then(|condition| condition.get_baton_touch_trigger())
            .unwrap_or(false);
        if baton_condition || baton_arrival_area.is_some() {
            log::debug!(
                "[BATON_UNDER] destination={} source={} self_target={} baton_condition={} arriving={:?} arrival_area={:?} vacated={:?}",
                destination,
                source,
                effect.is_self_target(),
                baton_condition,
                gs.baton_touch_arriving_card_id,
                baton_arrival_area,
                vacated_stage_area,
            );
        }
        // A `same_area` destination CONSUMES the vacated area, and the consuming
        // placement can happen after this frame has suspended: the effect raises a
        // selection prompt and returns early, and on resume `execute_selected_cards_from_zone`
        // (move/mod.rs:2666) reads `gs.last_vacated_stage_area` to forward it through
        // `execute_stage_placement_choices` (move/placement.rs:381).
        //
        // Clearing unconditionally here destroyed the value on exactly that path.
        // PL!HS-bp1-002-RM 「…このメンバーがいたエリアに登場させる」 placed the card in the
        // FIRST FREE SLOT (the centre) whenever its waitroom held two candidates and the
        // player was prompted, but placed it correctly in the vacated area when there was
        // a single candidate and no prompt was raised — the same card, same effect, one
        // behavioural difference with one cause.
        //
        // So clear unless THIS move is the consumer. Everything else keeps the staleness
        // guard the unconditional clear was there for.
        let consumes_vacated_area = effect.destination == Some(Zone::SameArea);
        log::debug!(
            "[VACATED_CLEAR] dest={:?} vacated={:?} consuming={} -> keep={}",
            effect.destination,
            vacated_stage_area,
            consumes_vacated_area,
            consumes_vacated_area
        );
        if !consumes_vacated_area {
            gs.last_vacated_stage_area = None;
        }

        // Store destination for execute_selected_cards_from_zone to read later
        // (needed when the resolve creates a card selection choice and the destination
        // is not accessible via entry_destination, e.g. for sequential sub-actions).
        self.in_flight.spawn_context.destination = effect.destination.map(|z| z.to_str().to_string());
        self.in_flight.spawn_context.source = effect.source_any().map(|s| s.to_string());
        self.in_flight.spawn_context.position = effect.position_any().and_then(|p| match p {
            crate::card::PositionInfo::String(s) => s.parse::<u8>().ok(),
            crate::card::PositionInfo::Struct { position, .. } => {
                position.as_ref().and_then(|s| s.parse::<u8>().ok())
            }
        });

        // For empty_area / stage destinations: skip selection prompt entirely
        // if the target has no empty slots (card text says "メンバーのいないエリアに").
        if Zone::from_str(&destination) == Some(Zone::EmptyArea) {
            let player = gs.resolve_target_player(target);
            let has_empty_slot = (0..3).any(|i| player.stage.stage[i] == -1);
            if !has_empty_slot {
                return Ok(());
            }
        }

        let mut taken = self.resolve_cards_from_source(
            gs,
            effect,
            SelectionSpec {
                count,
                is_max,
                is_all,
                is_self_cost,
                exclude_self,
                activating_card_id,
            },
            &util::filter_from_parts_full(
                card_type_filter,
                group_name,
                cost_limit,
                // Honor the operator (e.g. 「コスト9以上」 >=) — dropping it let
                // below-threshold cards through the selection.
                effect.cost_limit_operator_any().map(Operator::as_str),
                character_filter.as_ref(),
                name_fragments.as_ref(),
                None, // distinct
                None, // exclude_self
                cost_total,
                cost_total_operator,
                effect.exclude_characters_any(),
            ),
            MoveRoute {
                source: &source,
                destination: &destination,
                use_p2,
            },
            &card_db,
        )?;
        if effect.optional.unwrap_or(false) && taken.is_empty() && self.awaiting.choice.is_none() {
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.optional_cost_result = Some(false);
            }
        }
        // Q118 all-or-nothing: inside an accepted conditional_on_optional placement,
        // a move that found no card AND offered no choice (e.g. a group missing from
        // the discard pile) makes the placement incomplete, so the trailing
        // "そうしたとき" consequence must not fire. A successful move either returns
        // cards here or creates a pending_choice — only a genuine miss has neither.
        let armed = gs
            .ability_queue
            .current_entry()
            .and_then(|e| e.optional_moves_all_moved)
            .is_some();
        if armed && taken.is_empty() && self.awaiting.choice.is_none() {
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.optional_moves_all_moved = Some(false);
            }
        }
        if self.maybe_prompt_deck_order(
            gs,
            DeckOrderHint::of(effect),
            &taken,
            &source,
            &destination,
            &mut moved_cards,
        ) {
            return Ok(());
        }

// Apply distinct card name filter if specified
let distinct = effect.distinct_any();
if util::distinct_should_dedupe(distinct) {
    let deduped = util::dedupe_by_normalized_name(&taken, |&id| id, &card_db);
    taken = deduped;
    if taken.len() < count {
        taken.clear(); // Not enough distinct cards ? skip
    }
}

        // --- STEP 3: Place cards in destination ---
        let deck_pos = Self::parse_effect_deck_pos(effect);

        if self.place_taken_cards(
            gs,
            &taken,
            &source,
            &destination,
            tgt.as_deref(),
            effect,
            use_p2,
            vacated_stage_area,
            is_max,
            count,
            deck_pos,
            &card_db,
            &mut moved_cards,
        )? {
            return Ok(());
        }

        let state_change = effect.state_change_any().map(|s| s.to_string());
        // Record whether this move actually moved any cards. Used by the
        // "…したとき" (when you do so) pattern: a consequence step directly
        // following a move that moved nothing must be skipped.
        self.carried.last_move_moved_any = Some(!moved_cards.is_empty());
        self.finalize_card_movement(
            gs,
            &moved_cards,
            &destination,
            &source,
            &state_change,
            tgt.as_deref(),
        );
        Ok(())
    }


    /// Apply post-move side effects: zone-exit cleanup (rule 4.1.4),
    /// state_change, record_card_movement, tracking.
    pub(crate) fn finalize_card_movement(
        &mut self,
        gs: &mut GameState,
        moved_cards: &[i16],
        destination: &str,
        source: &str,
        state_change: &Option<String>,
        target: Option<&str>,
    ) {
        gs.on_cards_left_zones(moved_cards);

        if let Some(ref sc) = state_change {
            if sc == "wait" {
                for card_id in moved_cards {
                    gs.set_orientation(*card_id, "wait");
                }
            } else if sc == "active" {
                for card_id in moved_cards {
                    gs.set_orientation(*card_id, "active");
                }
                if Zone::from_str(destination) == Some(Zone::Energy) {
                    let p = match target.unwrap_or("self") {
                        "self" => &mut gs.player1,
                        "opponent" => &mut gs.player2,
                        _ => &mut gs.player1,
                    };
                    p.energy_zone.active_energy_count += moved_cards.len().u8_count();
                }
            }
        }

        for card_id in moved_cards {
            gs.record_card_movement(*card_id);
        }

        self.selection.moved_cards.extend(moved_cards.iter().copied());
        {
            let cause_pid = gs
                .ability_queue
                .current_entry()
                .map(|e| e.player_id.clone())
                .unwrap_or_default();
            let cause_cid = gs.activating_card;
            // same_area is an internal repositioning mechanic, not a zone transition.
            // Skip push_movement_event so watcher triggers don't fire for it.
            if destination != "same_area" {
                for &cid in moved_cards {
                    gs.push_movement_event(
                        cid,
                        source,
                        destination,
                        cause_cid,
                        &cause_pid,
                        true,
                    );
                }
            }
        }
        log::trace!(
            "[MOVE_TRACKING] requested_destination={} recorded_cards={:?} accumulated_cards={:?}",
            destination,
            moved_cards,
            self.selection.moved_cards
        );

        // Zone membership changed — energy-count / stage / success-zone
        // constants (「〜あるかぎり」) are live state and must re-evaluate now,
        // not at the next phase boundary.
        if !moved_cards.is_empty() {
            gs.recalculate_constants();
        }

        // Debut side effects for cards placed directly on stage (single free slot,
        // no position-choice dialog). Each card goes through the unified
        // fire_debut_side_effects which handles record_appearance, debut_count,
        // card's own debut ability, AND cascading "when ally debuts" triggers.
        if Zone::from_str(destination) == Some(Zone::Stage) && !moved_cards.is_empty() {
            let tgt = target.unwrap_or("self");
            for &card_id in moved_cards {
                self.fire_debut_side_effects(gs, card_id, tgt);
            }
        }
    }

    /// Fire all side effects for a card placed on stage: record appearance,
    /// increment debut count, fire the card's own debut ability, then cascade
    /// to "when ally debuts" triggers for other stage members.
    /// This is the single canonical function for debut processing — every
    /// code path that places a card on stage must call this.
    pub(crate) fn fire_debut_side_effects(&self, gs: &mut GameState, card_id: i16, target: &str) {
        let player_id = gs.resolve_target_player(target).id.clone();

        let source = self.in_flight.spawn_context.source.as_deref().unwrap_or("");
        gs.record_card_appearance(card_id, source);

        // Borrowed, not cloned. The loop below needs `&mut gs`, so the borrow of
        // the card cannot stay live across it — but only the enqueue ids are
        // actually needed, and collecting those ends the borrow in one place.
        // `.cloned()` here copied the whole Card (328 B, up to ten heap
        // allocations for the display strings) on every stage placement.
        let mut card_no = String::new();
        let mut known_card = true;
        let debut_ids: Vec<String> = match gs.card_database.get_card(card_id) {
            None => {
                log::debug!("[DEBUT_CHAIN] fire_debut_side_effects: unknown card id {card_id}");
                known_card = false;
                Vec::new()
            }
            Some(card) => {
                card_no = card.card_no.to_string();
                let mut ids = Vec::new();
                for ar in &card.abilities {
                    let ability = ar.resolve();
                    if GameState::ability_matches_trigger(
                        &ability,
                        &crate::core::types::AbilityTrigger::Debut,
                    ) {
                        ids.push(format!("{}_{}", card_no, ability.full_text));
                    }
                }
                if ids.is_empty() {
                    log::debug!(
                        "[DEBUT_CHAIN] {card_no} (id={card_id}) has no 登場 abilities — \
                         nothing to enqueue"
                    );
                }
                ids
            }
        };
        if !known_card {
            gs.trigger_auto_abilities_for_player(&player_id);
            gs.process_pending_auto_abilities(&player_id);
            return;
        }

        if let Some(player) = gs.try_player_by_id_mut(&player_id) {
            player.debut_count_this_turn += 1;
        }

        for ability_id in debut_ids {
            log::debug!(
                "[DEBUT_CHAIN] enqueueing debut of {card_no} (id={card_id}, \
                 controller={player_id}): {ability_id}"
            );
            gs.trigger_auto_ability(
                ability_id,
                crate::core::types::AbilityTrigger::Debut,
                player_id.as_str(),
                Some(card_no.clone()),
                Some(card_id),
                None,
                None,
            );
        }

        // Cascade to other stage members that watch for ally debuts.
        gs.trigger_auto_abilities_for_player(&player_id);
        gs.process_pending_auto_abilities(&player_id);
    }

    /// Execute card movement from a zone: pre-validate filters, move cards to destination, track side effects.
    #[allow(clippy::too_many_arguments)]

    /// Handle "both" target for move_cards: process opponent first, queue self.
    pub fn execute_move_cards_both(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        // Save original spawn_context.target to restore later
        let original_target = self.in_flight.spawn_context.target.clone();

        // Override spawn_context.target before processing opponent — the generic
        // "both" handler in effects.rs may have set it to "self".
        self.in_flight.spawn_context.target = Some("opponent".to_string());
        let mut opp_eff = effect.clone();
        opp_eff.target = Some("opponent".into());
        self.execute_move_cards(gs, &opp_eff)?;

        if self.awaiting.choice.is_some() {
            log::debug!("[MOVE_BOTH] Queueing self effect for later.");
            let mut self_eff = effect.clone();
            self_eff.target = Some("self".into());
            gs.ability_queue.set_pending_actions(vec![self_eff]);
            // Restore original target before returning
            self.in_flight.spawn_context.target = original_target;
            return Ok(());
        }
        log::debug!("[MOVE_BOTH] No choice created. Processing self now.");
        // Reset spawn_context.target for self phase
        self.in_flight.spawn_context.target = Some("self".to_string());
        let mut self_eff = effect.clone();
        self_eff.target = Some("self".into());
        let result = self.execute_move_cards(gs, &self_eff);
        // Restore original target
        self.in_flight.spawn_context.target = original_target;
        result
    }

    pub(crate) fn execute_discard_until_count(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let target_count: u8 = effect.target_count_any().unwrap_or(0);
        let target = effect.target_name();
        let player = gs.resolve_target_player_mut(target);
        let current_count = player.hand.cards.len();
        if current_count <= target_count as usize {
            return Ok(());
        }
        let cards_to_discard = current_count - target_count as usize;
        let desc_en = format!(
            "Discard {} cards from hand (target: {} cards in hand)",
            cards_to_discard, target_count
        );
        let desc_ja = format!(
            "手札から{}枚捨てる（目標: 手札{}枚）",
            cards_to_discard, target_count
        );
        self.awaiting.choice = Some(
            Choice::select_cards(Zone::Hand.to_str(), cards_to_discard, desc_en, false)
                .description_ja(Some(desc_ja))
                .target_player_id(Some(target.to_string()))
                .build(),
        );
        self.in_flight.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.push_rule_log_fmt(format_args!(
            "{} {}: [[log_discard_until:n={}]]",
            pp, act_name, target_count
        ));
        Ok(())
    }

    /// Shared MoveCards/DiscardCard tail (rule log + current-effect pinning
    /// and move execution) — the registry's logged-move arms delegate here so
    /// the 3-line sequence lives in exactly one place.
    pub(crate) fn execute_logged_move(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        log_tag: &str,
    ) -> Result<(), String> {
        self.rule_log_activated(gs, log_tag);
        self.owner.executing = Some(effect.clone());
        self.execute_move_cards(gs, effect)
    }

    // Q55: Effects resolve as much as possible; partial resolution required when full is impossible.
}