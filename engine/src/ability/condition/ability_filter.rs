use super::ConditionContext;
use crate::ability::enums::Zone;
use crate::card::{AbilityFilter, Condition};
use crate::core::constants::CountCast;
#[cfg(feature = "no_std")]
use alloc::vec::Vec;

impl<'a> ConditionContext<'a> {
    /// The card set an `ability_filter` should be evaluated against.
    ///
    /// Shared by `evaluate_ability_filter_condition` and its
    /// `_with_card_check` twin, which carried byte-identical copies of this
    /// match.
    ///
    /// Deliberately narrow: only the five zones these evaluators have always
    /// considered. `util::zone_card_ids_occupied` covers a wider set (deck,
    /// deck_top, success_live_zone) and would read as the obvious helper to
    /// reach for, but adopting it would CHANGE behaviour — today a filter
    /// naming any other location falls through to the activating card, and
    /// silently starting to inspect deck contents is a semantics change, not a
    /// cleanup. Widening that is a separate decision.
    ///
    /// Returns `None` for a location outside the five when there is no
    /// activating card, which callers treat as "the filter has nothing to
    /// inspect, so it passes".
    fn ability_filter_scope(
        &self,
        player: &crate::player::Player,
        location: &str,
    ) -> Option<Vec<i16>> {
        Some(match Zone::from_str(location) {
            Some(Zone::Stage) => player
                .stage
                .stage
                .iter()
                .filter(|&&id| id != -1)
                .copied()
                .collect(),
            Some(Zone::Hand) => player.hand.cards.to_vec(),
            Some(Zone::Discard) | Some(Zone::Waitroom) => player.waitroom.cards.to_vec(),
            Some(Zone::Energy) => player.energy_zone.cards.to_vec(),
            Some(Zone::LiveCardZone) => player.live_card_zone.cards.to_vec(),
            _ => return self.game_state.activating_card.map(|id| vec![id]),
        })
    }

    pub(crate) fn evaluate_ability_filter_condition(&self, condition: &Condition) -> bool {
        log::debug!(
            "[ABILITY_FILTER_EVAL] filter={:?} triggers={:?} location={:?}",
            condition.get_ability_filter(),
            condition.get_ability_filter_triggers(),
            condition.get_location()
        );
        let target = condition.get_target().unwrap_or("self");
        let card_db = &self.game_state.card_database;
        let player = self.game_state.resolve_target_player(target);
        let filter = condition
            .get_ability_filter()
            .unwrap_or(&AbilityFilter::NoAbility);

        let location = condition.get_location().unwrap_or(Zone::Stage.to_str());
        let Some(card_ids) = self.ability_filter_scope(player, location) else {
            return true;
        };

        let has_ability = if let Some(card_id) = self.game_state.activating_card {
            if let Some(card) = card_db.get_card(card_id) {
                !card.abilities.is_empty()
            } else {
                false
            }
        } else {
            if card_ids.is_empty() {
                return false;
            }
            card_ids.iter().any(|&id| {
                card_db
                    .get_card(id)
                    .map(|c| !c.abilities.is_empty())
                    .unwrap_or(false)
            })
        };

        match filter {
            AbilityFilter::NoAbility => !has_ability,
            AbilityFilter::HasAbility => has_ability,
            AbilityFilter::NoAbilityType => {
                // 「…能力も…能力も持たないカードがある場合」: SOME card in
                // the scanned zone lacks every listed trigger type. Scans the
                // LOCATION's cards (not just the activating card).
                let excluded: &[String] = condition.get_ability_filter_triggers().unwrap_or(&[]);
                if excluded.is_empty() {
                    return false;
                }
                if card_ids.is_empty() {
                    return false;
                }
                card_ids.iter().any(|&id| {
                    card_db.get_card(id).is_some_and(|c| {
                        !c.abilities.iter().any(|ar| {
                            ar.resolve()
                                .triggers
                                .as_ref()
                                .is_some_and(|t| excluded.iter().any(|et| t.starts_with(&et[..])))
                        })
                    })
                })
            }
            _ => true,
        }
    }

    pub(crate) fn evaluate_ability_filter_condition_with_card_check(
        &self,
        condition: &Condition,
        filter: &str,
    ) -> bool {
        let card_db = &self.game_state.card_database;
        let target = condition.get_target().unwrap_or("self");
        let player = self.game_state.resolve_target_player(target);

        let location = condition.get_location().unwrap_or(Zone::Stage.to_str());
        let Some(card_ids) = self.ability_filter_scope(player, location) else {
            return true;
        };

        let operator = condition.get_operator().unwrap_or("any");
        let count_needed = condition.get_count().unwrap_or(1);

        let match_count = card_ids
            .iter()
            .filter(|&&id| {
                card_db
                    .get_card(id)
                    .map(|c| {
                        let has_ability = !c.abilities.is_empty();
                        match filter {
                            "no_ability" => !has_ability,
                            "has_ability" => has_ability,
                            "no_ability_type" if has_ability => {
                                let excluded_triggers: Vec<&str> = condition
                                    .get_ability_filter_triggers()
                                    .as_ref()
                                    .map(|t| t.iter().map(|s| s.as_str()).collect())
                                    .unwrap_or_default();
                                !c.abilities.iter().any(|ar| {
                                    ar.resolve()
                                        .triggers
                                        .as_ref()
                                        .map(|t| {
                                            excluded_triggers.iter().any(|et| t.starts_with(et))
                                        })
                                        .unwrap_or(false)
                                })
                            }
                            _ => true,
                        }
                    })
                    .unwrap_or(false)
            })
            .count()
            .u8_count();

        // An operator we do not know is a parser gap, not a count that fell
        // short, so it falls back to ">=", which is what this gate defaulted to.
        crate::ability::util::compare_with_operator(operator, match_count, count_needed)
            .unwrap_or(match_count >= count_needed)
    }

}
