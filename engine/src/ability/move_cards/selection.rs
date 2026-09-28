use super::{AbilityEffect, AbilityResolver, CardDatabase, Choice, ExecutionContext, GameState, MoveSourceContext, Player, Zone, util};
#[cfg(feature = "no_std")]
use alloc::{string::{String, ToString}, vec::Vec};

impl AbilityResolver {
    pub(super) fn prompt_card_selection(
        &mut self,
        zone: &str,
        count: usize,
        can_skip: bool,
        effect: &AbilityEffect,
        filter: &util::CardFilter,
        filtered_indices: Option<Vec<usize>>,
    ) {
        log::debug!(target: "rabuka_engine::ability::move_cards",
            "[SELECTION_PROMPT] zone={} count={} can_skip={} prop={:?} neg={:?} filtered={:?} excl_chars={:?}",
            zone, count, can_skip, filter.card_property, filter.negation, filtered_indices, filter.exclude_characters
        );
        let zone_display = crate::ability::describe::zone_label(Some(zone));
        let zone_display_ja = crate::ability::describe::zone_label_ja(Some(zone));
        let description = if effect.any_number_any().unwrap_or(false) {
            format!("Select any number of {} from {}", util::card_plural(count), zone_display)
        } else {
            format!("Select {} {} from {}", count, util::card_plural(count), zone_display)
        };
        let description_ja = if effect.any_number_any().unwrap_or(false) {
            format!("{}から任意枚選択", zone_display_ja)
        } else {
            format!("{}から{}枚選択", zone_display_ja, count)
        };
        self.awaiting.choice = Some(
            Choice::select_cards(zone, count, description, can_skip)
                .description_ja(Some(description_ja))
                .card_type(filter.card_type.map(|s| s.to_string()))
                .cost_limit(filter.cost_limit, effect.cost_limit_operator_any().map(|s| s.to_string()))
                .cost_total(filter.cost_total, effect.cost_total_operator_any().map(|s| s.to_string()))
                .group(filter.group.map(|s| s.to_string()))
                .characters(filter.characters.map(|v| v.to_vec()))
                .target_player_id(Some(
                    effect.action_by_any().map(|ab| {
                        if ab == "opponent" { "opponent".to_string() } else { "self".to_string() }
                    }).unwrap_or_else(|| "self".to_string()),
                ))
                .filtered_indices(filtered_indices)
                .destination(effect.destination.map(|s| s.to_string()))
                .discard_remaining(effect.discard_remaining_any())
                .build(),
        );
        self.in_flight.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
    }

    #[allow(clippy::too_many_arguments)]
pub(super) fn take_cards_from_standard_zone(
        &mut self,
        player: &mut Player,
        card_db: &CardDatabase,
        zone_name: &str,
        filter: &util::CardFilter,
        count: usize,
        is_all: bool,
        behavior: util::InsufficientBehavior,
        can_skip: bool,
        effect: &AbilityEffect,
        activating_card_id: Option<i16>,
    ) -> Result<Option<Vec<i16>>, String> {
        let cards = util::zone_card_ids(player, zone_name);
        log::debug!(target: "rabuka_engine::ability::move_cards",
            "[TAKE_SOURCE] owner={} zone={} available={} count={} all={} can_skip={} self_only={}",
            player.id, zone_name, cards.len(), count, is_all, can_skip, effect.is_self_target()
        );
        log::trace!(target: "rabuka_engine::ability::move_cards", "[TAKE_SOURCE_CARDS] ids={:?}", cards);
        let filtered_indices = util::matching_indices(&cards, card_db, filter, false);
        let prompt_count = match util::resolve_selection(
            &cards, card_db, activating_card_id, count, is_all, filter,
            effect.is_self_target(), behavior, false,
        )? {
            util::SelectionOutcome::Exact(indices) if can_skip && !is_all && !indices.is_empty() => indices.len(),
            util::SelectionOutcome::Exact(indices) => {
                return Ok(Some(util::zone_remove_at_indices(player, zone_name, &indices)));
            }
            util::SelectionOutcome::Prompt => count,
            util::SelectionOutcome::Skip if can_skip && !filtered_indices.is_empty() => filtered_indices.len(),
            util::SelectionOutcome::Skip => return Ok(Some(vec![])),
        };
        self.prompt_card_selection(zone_name, prompt_count, can_skip, effect, filter, Some(filtered_indices));
        Ok(None)
    }

    pub(crate) fn optional_gate_source(zone: Zone) -> bool {
        matches!(zone, Zone::Deck | Zone::DeckTop | Zone::DeckBottom | Zone::EnergyDeck | Zone::Energy)
    }

    fn ask_optional_move_gate(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        source_zone: Zone,
        description_en: &str,
        description_ja: &str,
    ) -> bool {
        if !effect.optional.unwrap_or(false) || !Self::optional_gate_source(source_zone) {
            return false;
        }
        let decided = gs.ability_queue.current_entry()
            .and_then(|e| e.conditional_choice.as_ref()).is_some();
        if decided {
            return false;
        }
        let player = gs.resolve_target_player(effect.target.as_deref().unwrap_or("self"));
        let available = match source_zone {
            Zone::EnergyDeck => player.energy_deck.cards.len(),
            Zone::Deck | Zone::DeckTop | Zone::DeckBottom => player.main_deck.cards.len(),
            Zone::Energy => player.energy_zone.cards.len(),
            _ => usize::MAX,
        };
        if available == 0 {
            log::debug!(target: "rabuka_engine::ability::move_cards",
                "[OPTIONAL_GATE] source {:?} has no cards for {} -> skipping pay/skip gate",
                source_zone, description_en
            );
            return false;
        }
        self.emit_pay_skip_gate(
            gs,
            Some(crate::ability::types::ChoiceRoute::Raw("pay_optional_cost".to_string())),
            description_en.to_string(),
            description_ja.to_string(),
            true,
            Some(vec!["No".to_string(), "Yes".to_string()]),
        );
        true
    }

    pub(super) fn gate_optional_source(
        &mut self,
        gs: &mut GameState,
        c: &MoveSourceContext,
        source_zone: Zone,
    ) -> bool {
        let (description_en, description_ja) = match source_zone {
            Zone::Deck => ("Place top card of deck to waiting room?", "山札の上を控え室に置きますか？"),
            Zone::DeckBottom => ("Place bottom card of deck to waiting room?", "山札の下を控え室に置きますか？"),
            Zone::EnergyDeck => ("Place energy card(s) from the energy deck?", "エネルギーデッキからエネルギーを置きますか？"),
            Zone::Energy => ("Move an energy card from the energy zone to the energy deck?", "エネルギー置き場のエネルギーをエネルギーデッキに置きますか？"),
            _ => return false,
        };
        self.ask_optional_move_gate(gs, c.effect, source_zone, description_en, description_ja)
    }
}
