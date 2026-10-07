use super::ConditionContext;
use crate::ability::enums::Zone;
use crate::ability::util::{self, compare_counts};
use crate::card::Condition;
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

impl<'a> ConditionContext<'a> {
    pub(crate) fn evaluate_group_condition(&self, condition: &Condition) -> bool {
        if condition.get_all_members().unwrap_or(false) {
            // `unwrap_or(false)`: an `all_members` over a zone this helper does not
            // enumerate has always answered false on the Group path. Falling through
            // to the ordinary count path instead would silently start passing.
            return self
                .evaluate_all_members_condition(condition)
                .unwrap_or(false);
        }
        let g_target = condition.get_target().unwrap_or("self");
        let g_player = self.resolve_condition_player(g_target);
        let g_location = condition.get_location().unwrap_or("");

        if condition.get_all_areas().unwrap_or(false)
            && Zone::from_str(g_location) == Some(Zone::Stage)
        {
            let card_db = &self.game_state.card_database;
            let groups = condition.get_group_names().unwrap_or(&[]);
            let card_type = condition.get_card_type().map(|ct| ct.as_str());
            let excluded = condition.get_exclude_characters().unwrap_or(&[]);
            let result = g_player.stage.stage.iter().all(|&card_id| {
                card_id != -1
                    && (groups.is_empty()
                        || groups.iter().any(|group| {
                            util::card_matches_group_str(card_db, card_id, Some(group))
                        }))
                    && card_type
                        .is_none_or(|ct| util::card_matches_type(card_db, card_id, Some(ct)))
                    && !excluded.iter().any(|name| {
                        card_db.get_card(card_id).is_some_and(|card| {
                            card.name_key.contains(name.as_str())
                                || card.card_no.contains(name)
                        })
                    })
            });
            log::debug!(
                "[GROUP_ALL_AREAS] target={} result={} stage={:?}",
                g_target,
                result,
                g_player.stage.stage
            );
            return result;
        }

        // Aggregate total check (sum of heart values, e.g. heart02 >= 6)
        if let Some(res) = self.check_aggregate_total(condition, g_player, g_location) {
            return res;
        }

        // When heart_colors are specified, check that the collective cards in the
        // target zone cover ALL required heart colors (e.g. yell-revealed cards).
        let hc: &[String] = condition.get_heart_colors().unwrap_or(&[]);
        if !hc.is_empty() {
            let target = condition.get_target().unwrap_or("self");
            let player = self.resolve_condition_player(target);
            let group_name = condition
                .get_group_names()
                .and_then(|gn| gn.first().map(|s| s.as_str()));
            let location = condition.get_location().unwrap_or("");
            let source_cards: Vec<i16> = match Zone::from_str(location) {
                Some(Zone::RevealedCards) => self.revealed_cards_scoped(condition),
                Some(Zone::Stage) => player
                    .stage
                    .stage
                    .iter()
                    .copied()
                    .filter(|&id| id != -1)
                    .collect(),
                _ => return false,
            };
            let required_colors: Vec<crate::card::HeartColor> = hc
                .iter()
                .map(|s| crate::card::parse_heart_color(s))
                .collect();
            let is_blade = condition.get_heart_source() == Some("blade");
            let card_type = condition.get_card_type().map(|ct| ct.as_str());
            let present = self.count_distinct_heart_types(
                &source_cards,
                &required_colors,
                is_blade,
                group_name,
                card_type,
            );
            return present >= required_colors.len();
        }
        // When multiple group_names are specified, check EACH group has at least `count` members
        if let Some(groups) = condition.get_group_names() {
            if groups.len() > 1 && condition.get_aggregate() != Some("total") {
                let target_count = condition.get_count().unwrap_or(1);
                let ct = condition.get_card_type().map(|ct| ct.as_str());
                let exc = condition.get_exclude_characters();
                let location = condition.get_location().unwrap_or("");
                let cards: &[i16] = match Zone::from_str(location) {
                    Some(Zone::Stage) => &g_player.stage.stage,
                    Some(Zone::Hand) => &g_player.hand.cards,
                    Some(Zone::Discard) | Some(Zone::Waitroom) => &g_player.waitroom.cards,
                    Some(Zone::LiveCardZone) => &g_player.live_card_zone.cards,
                    Some(Zone::SuccessLiveZone) => &g_player.success_live_card_zone.cards,
                    Some(Zone::Energy) => &g_player.energy_zone.cards,
                    _ => return false,
                };
                for group_name in groups {
                    let group_slice = [group_name.clone()];
                    let count =
                        self.count_group_cards_in_cards(cards, Some(&group_slice[..]), ct, exc);
                    if count < target_count {
                        return false;
                    }
                }
                return true;
            }
        }
        if condition.get_temporal() == Some("this_turn")
            && condition.get_self_target().unwrap_or(false)
            && condition.get_group_names().is_some_and(|g| !g.is_empty())
            && condition.get_card_type().map(|ct| ct.as_str()) == Some("member_card")
        {
            if let Some(activating_card_id) = self.activating_card_id {
                let target = condition.get_target().unwrap_or("self");
                let player = self.resolve_condition_player(target);
                let card_db = &self.game_state.card_database;
                let groups = condition.get_group_names().unwrap();
                return self.game_state.turn_area_movements.iter().any(|m| {
                    m.moved_card_id == activating_card_id
                        && m.effect_only
                        && m.cause_player_id == player.id.as_str()
                        && m.cause_card_id.is_some_and(|cause_cid| {
                            groups.iter().any(|g| {
                                crate::ability::util::card_matches_group_str(
                                    card_db,
                                    cause_cid,
                                    Some(g),
                                )
                            })
                        })
                });
            }
            return false;
        }

        let mut count = self.get_group_card_count(condition);
        if condition.get_exclude_self().unwrap_or(false) {
            if let Some(aid) = self.activating_card_id {
                let target = condition.get_target().unwrap_or("self");
                let player = self.resolve_condition_player(target);
                if player.stage.stage.contains(&aid) {
                    count = count.saturating_sub(1);
                }
            }
        }
        let target_count = condition.get_count().unwrap_or(1);
        let operator = condition.get_operator().or(Some(">="));
        compare_counts(operator, count, target_count)
    }

    /// Apply the shared card_type/group filter to a zone's cards and sum a
    /// per-card value. Used by aggregate zone counts (live_card_zone and
    /// success_live_card_zone).
    pub(crate) fn sum_group_filtered_zone(
        &self,
        cards: &[i16],
        card_type: Option<&str>,
        group_name: Option<&str>,
        value: impl Fn(&crate::card::Card) -> u8,
    ) -> u8 {
        let card_db = &self.game_state.card_database;
        cards
            .iter()
            .filter(|&&cid| cid != -1)
            .filter(|&&cid| card_type.is_none() || util::card_matches_type(card_db, cid, card_type))
            .filter(|&&cid| {
                group_name.is_none() || util::card_matches_group_str(card_db, cid, group_name)
            })
            .filter_map(|&cid| card_db.get_card(cid))
            .map(value)
            .sum()
    }

}
