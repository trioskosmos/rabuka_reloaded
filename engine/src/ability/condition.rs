use crate::card::Condition;
use crate::game_state::Phase;
use super::util;
use super::util::compare_counts;

impl<'a> super::resolver::AbilityResolver<'a> {
    pub fn evaluate_condition(&self, condition: &Condition) -> bool {
        let result = match condition.condition_type.as_deref() {
            Some("compound") => self.evaluate_compound_condition(condition),
            Some("comparison_condition") => self.evaluate_comparison_condition(condition),
            Some("location_condition") => self.evaluate_location_condition(condition),
            Some("card_count_condition") => self.evaluate_card_count_condition(condition),
            Some("group_condition") => self.evaluate_group_condition(condition),
            Some("position_condition") => self.evaluate_position_condition(condition),
            Some("appearance_condition") => self.evaluate_appearance_condition(condition),
            Some("temporal_condition") => self.evaluate_temporal_condition(condition),
            Some("state_condition") => self.evaluate_state_condition(condition),
            Some("energy_state_condition") => self.evaluate_energy_state_condition(condition),
            Some("movement_condition") => self.evaluate_movement_condition(condition),
            Some("ability_negation_condition") => self.evaluate_ability_negation_condition(condition),
            Some("or_condition") => self.evaluate_or_condition(condition),
            Some("any_of_condition") => self.evaluate_any_of_condition(condition),
            Some("score_threshold_condition") => self.evaluate_score_threshold_condition(condition),
            Some("choice_condition") => self.evaluate_choice_condition(condition),
            Some("position_change_condition") => self.evaluate_position_change_condition(condition),
            Some("state_change_condition") => self.evaluate_state_change_condition(condition),
            Some("opponent_choice_condition") => self.evaluate_opponent_choice_condition(condition),
            Some("opponent_live_success") => self.evaluate_opponent_live_success_condition(condition),
            Some("complex_condition") => self.evaluate_complex_condition(condition),
            Some("no_excess_heart") => self.evaluate_no_excess_heart_condition(condition),
            _ => false,
        };

        // Check ability_negation field on any condition type
        if condition.ability_negation.unwrap_or(false) {
            let negated = self.evaluate_ability_negation_condition(condition);
            if !negated { return false; }
        }

        if condition.negation.unwrap_or(false) {
            !result
        } else {
            result
        }
    }

    fn evaluate_compound_condition(&self, condition: &Condition) -> bool {
        if let Some(ref conditions) = condition.conditions {
            match condition.operator.as_deref() {
                Some("and") => conditions.iter().all(|c| self.evaluate_condition(c)),
                Some("or") => conditions.iter().any(|c| self.evaluate_condition(c)),
                _ => true,
            }
        } else {
            true
        }
    }

    fn evaluate_comparison_condition(&self, condition: &Condition) -> bool {
        let count = self.get_count_for_condition(condition);

        if let Some(ref values) = condition.values {
            return values.contains(&(count as u32));
        }

        let default_count = if condition.location.is_some() || condition.card_type.is_some() { 1 } else { 0 };
        let target_count = if let Some(ref comparison_target) = condition.comparison_target {
            if comparison_target == "opponent" {
                self.get_count_for_target(condition, "opponent")
            } else {
                condition.count.unwrap_or(default_count)
            }
        } else {
            condition.count.unwrap_or(default_count)
        };

        compare_counts(condition.operator.as_deref(), count, target_count)
    }

    fn evaluate_location_condition(&self, condition: &Condition) -> bool {
        // Handle multiple locations (e.g. "stage and waitroom")
        if let Some(ref locs) = condition.locations {
            if locs.len() >= 2 {
                return self.evaluate_multi_location_condition(condition);
            }
        }
        let location = condition.location.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let card_type_filter = condition.card_type.as_deref();
        let comparison_type = condition.comparison_type.as_deref();
        let operator = condition.operator.as_deref();
        let original_value = condition.original_value.unwrap_or(false);
        // When count is not specified but filters are set, default to 1
        // (checking "at least one matching card exists") instead of 0
        // which would make >= always true.
        let count_threshold = condition.count.unwrap_or(if condition.aggregate.is_some() { 0 } else if condition.cost_limit.is_some() || condition.card_type.is_some() || condition.group_names.is_some() || condition.characters.is_some() || condition.distinct.unwrap_or(false) || condition.appearance.unwrap_or(false) { 1 } else { 0 });
        let distinct = condition.distinct.unwrap_or(false);
        let all_areas = condition.all_areas.unwrap_or(false);
        let no_excess_heart = condition.no_excess_heart.unwrap_or(false);
        let baton_touch_trigger = condition.baton_touch_trigger.unwrap_or(false);
        let cost_limit = condition.cost_limit;
        let group_names = condition.group_names.as_ref();
        let _characters = condition.characters.as_ref();
        let heart_type = condition.heart_type.as_deref();
        let card_property = condition.card_property.as_deref();

        // Heart type check (e.g., heart_type = "all" means check for heart00/ALL heart)
        if let Some(ht) = heart_type {
            match ht {
                "all" => {
                    let card_db = &self.game_state.card_database;
                    let player = self.game_state.resolve_target_player(target);
                    let has_heart00 = player.stage.stage.iter().any(|&id| {
                        if id == -1 { false } else {
                            card_db.get_card(id).map_or(false, |c| {
                                c.base_heart.as_ref().map_or(false, |bh| bh.hearts.contains_key(&crate::card::HeartColor::Heart00))
                            })
                        }
                    });
                    // negation is checked separately below, so we return the raw result
                    if !has_heart00 { return false; }
                }
                _ => {}
            }
        }

        // Collective heart colors check: all required colors must be present on stage members
        if let Some(ref req_colors) = condition.heart_colors {
            if !req_colors.is_empty() {
                let card_db = &self.game_state.card_database;
                let player = self.game_state.resolve_target_player(target);
                let stage_cards: Vec<i16> = player.stage.stage.iter().filter(|&&id| id != -1).copied().collect();
                for color_str in req_colors {
                    let color = crate::zones::parse_heart_color(color_str);
                    let found = stage_cards.iter().any(|&id| {
                        card_db.get_card(id).map_or(false, |c| {
                            c.base_heart.as_ref().map_or(false, |bh| bh.hearts.contains_key(&color))
                        })
                    });
                    if !found { return false; }
                }
            }
        }

        // Card property check (e.g., "has_blade_heart")
        if let Some(prop) = card_property {
            match prop {
                "has_blade_heart" => {
                    let card_db = &self.game_state.card_database;
                    let player = self.game_state.resolve_target_player(target);
                    let cards: Vec<i16> = match location {
                        "revealed_cards" => self.game_state.revealed_cards.iter().copied().collect(),
                        "stage" => player.stage.stage.iter().filter(|&&id| id != -1).copied().collect(),
                        _ => vec![],
                    };
                    if !cards.is_empty() {
                        let has_property = cards.iter().any(|&id| {
                            card_db.get_card(id).map_or(false, |c| c.has_blade_heart())
                        });
                        if !has_property { return false; }
                    }
                }
                _ => {}
            }
        }

        if baton_touch_trigger {
            if self.game_state.baton_touch_count == 0 {
                return false;
            }
            // Check specific baton_touch_source if specified
            if let Some(source_name) = condition.baton_touch_source.as_deref() {
                let source_found = self.game_state.player1.waitroom.cards.iter()
                    .chain(self.game_state.player2.waitroom.cards.iter())
                    .any(|&id| self.game_state.card_database.get_card(id)
                        .map_or(false, |c| c.name.contains(source_name)));
                if !source_found {
                    return false;
                }
            }
            // Cost comparison: e.g. operator "<" means replaced cost < new card cost
            if comparison_type == Some("cost") {
                if let Some(replaced_cost) = self.game_state.baton_touch_replaced_member_cost {
                    if let Some(activating_id) = self.game_state.activating_card {
                        if let Some(card) = self.game_state.card_database.get_card(activating_id) {
                            if let Some(current_cost) = card.cost {
                                if !compare_counts(operator, replaced_cost, current_cost) {
                                    return false;
                                }
                            }
                        }
                    }
                }
            }
            // operator/comparison_type consumed by baton touch, skip compare_counts
            return true;
        }

        let card_db = &self.game_state.card_database;

        let check_distinct_names = |card_ids: &[i16]| -> bool {
            let mut names = std::collections::HashSet::new();
            for &card_id in card_ids {
                if card_id == -1 { continue; }
                if let Some(card) = card_db.get_card(card_id) {
                    let card_names = card_db.get_card_names(card_id);
                    for name in &card_names {
                        if !names.insert(name.clone()) {
                            eprintln!("[DISTINCT] duplicate name '{}' found for card_id={} ({})",
                                name, card_id, card.name);
                            return false;
                        }
                    }
                }
            }
            eprintln!("[DISTINCT] all names unique: {:?} total={}", names, card_ids.iter().filter(|&&id| id != -1).count());
            true
        };

        // Collect both players' cards for targets that need cross-player comparison
        let p1_cards: &[i16] = util::zone_cards(&self.game_state.player1, location);
        let p2_cards: &[i16] = util::zone_cards(&self.game_state.player2, location);
        fn player_cards_for_target<'b>(p1: &'b [i16], p2: &'b [i16], is_p1: bool) -> &'b [i16] { if is_p1 { p1 } else { p2 } }

        let original_blade_filter = |card_id: i16| -> bool {
            if !original_value { return true; }
            if let Some(op) = operator {
                let threshold = condition.count.unwrap_or(0) as u32;
                let card_blade = card_db.get_card(card_id).map(|c| c.blade).unwrap_or(0);
                compare_counts(Some(op), card_blade, threshold)
            } else { true }
        };

        let location_value = match target {
            "either" => {
                if comparison_type == Some("score") || comparison_type == Some("cost") {
                    let v1 = self.get_count_for_target(condition, "self");
                    let v2 = self.get_count_for_target(condition, "opponent");
                    v1.max(v2)
                } else {
                    let c1 = util::count_matching_with_blade(p1_cards, &card_db, card_type_filter, group_names.and_then(|g| g.first().map(|s| s.as_str())), cost_limit, operator, original_blade_filter);
                    let c2 = util::count_matching_with_blade(p2_cards, &card_db, card_type_filter, group_names.and_then(|g| g.first().map(|s| s.as_str())), cost_limit, operator, original_blade_filter);
                    if all_areas {
                        let p1_stage: &[i16] = &self.game_state.player1.stage.stage;
                        let p2_stage: &[i16] = &self.game_state.player2.stage.stage;
                        if p1_stage.iter().filter(|&&c| c != -1).count() != 3
                            && p2_stage.iter().filter(|&&c| c != -1).count() != 3 {
                            return false;
                        }
                    }
                    c1.max(c2)
                }
            }
            "both" => {
                let blade_count = |cards: &[i16]| -> u32 {
                    util::count_matching_with_blade(cards, &card_db, card_type_filter, group_names.and_then(|g| g.first().map(|s| s.as_str())), cost_limit, operator, original_blade_filter)
                };
                if let Some(op) = operator {
                    let self_is_p1 = std::ptr::eq(self.game_state.resolve_target_player("self"), &self.game_state.player1);
                    let self_count = blade_count(player_cards_for_target(p1_cards, p2_cards, self_is_p1));
                    let opp_count = blade_count(player_cards_for_target(p1_cards, p2_cards, !self_is_p1));
                    return compare_counts(Some(op), self_count, opp_count);
                }
                let self_is_p1 = std::ptr::eq(self.game_state.resolve_target_player("self"), &self.game_state.player1);
                blade_count(player_cards_for_target(p1_cards, p2_cards, self_is_p1))
            }
            _ => {
                let player = self.game_state.resolve_target_player(target);
                let revealed_vec: Vec<i16> = if location == "revealed_cards" {
                    self.game_state.revealed_cards.iter().copied().collect()
                } else { vec![] };
                let cards: &[i16] = if location == "revealed_cards" { &revealed_vec } else { util::zone_cards(player, location) };
                if comparison_type == Some("score") || comparison_type == Some("cost") || comparison_type == Some("energy") {
                    self.get_count_for_target(condition, target)
                } else {
                    let c = util::count_matching_with_blade(cards, &card_db, card_type_filter, group_names.and_then(|g| g.first().map(|s| s.as_str())), cost_limit, operator, original_blade_filter);
                    if all_areas {
                        let stage_slice: &[i16] = &player.stage.stage;
                        if stage_slice.iter().filter(|&&c| c != -1).count() != 3 {
                            return false;
                        }
                    }
                    c
                }
            }
        };

        if distinct {
            eprintln!("[DISTINCT] check: target={target} location={location} group_names={group_names:?}");
            let player = self.game_state.resolve_target_player(if target == "either" { "self" } else { target });
            let mut card_ids: Vec<i16> = match location {
                "stage" => player.stage.stage.to_vec(),
                "hand" => player.hand.cards.to_vec(),
                "deck" => player.main_deck.cards.to_vec(),
                "discard" | "waitroom" => player.waitroom.cards.to_vec(),
                "energy_zone" => player.energy_zone.cards.to_vec(),
                "live_card_zone" => player.live_card_zone.cards.to_vec(),
                "success_live_zone" => player.success_live_card_zone.cards.to_vec(),
                _ => vec![],
            };
            // When group_names is set, only check distinctness among group members
            if let Some(ref grp_names) = group_names {
                if let Some(first_group) = grp_names.first() {
                    let before_count = card_ids.iter().filter(|&&id| id != -1).count();
                    card_ids = card_ids.into_iter().filter(|&id| {
                        id != -1 && card_db.get_card(id).map(|c| {
                            c.unit.as_deref() == Some(first_group.as_str()) || c.group == *first_group
                        }).unwrap_or(false)
                    }).collect();
                    let after_count = card_ids.iter().filter(|&&id| id != -1).count();
                    eprintln!("[DISTINCT] group filter '{}': {} stage cards -> {} group members",
                        first_group, before_count, after_count);
                    for &id in &card_ids {
                        if id != -1 {
                            if let Some(c) = card_db.get_card(id) {
                                eprintln!("[DISTINCT]   group member: card_id={} name={} group={}", id, c.name, c.group);
                            }
                        }
                    }
                }
            }
            if !check_distinct_names(&card_ids) {
                return false;
            }
        }

        if no_excess_heart {
            let opponent = self.game_state.resolve_target_player(if target == "self" { "opponent" } else { "self" });
            let total_hearts: u32 = opponent.stage.stage.iter()
                .filter(|&&card_id| card_id != -1)
                .map(|&card_id| card_db.get_card(card_id).map(|c| c.total_hearts()).unwrap_or(0))
                .sum();
            let needed_hearts: u32 = opponent.live_card_zone.cards.iter()
                .map(|&card_id| card_db.get_card(card_id).map(|c| c.total_hearts()).unwrap_or(0))
                .sum();
            if total_hearts > needed_hearts {
                return false;
            }
        }

        compare_counts(operator, location_value, count_threshold)
    }

    fn evaluate_multi_location_condition(&self, condition: &Condition) -> bool {
        let target = condition.target.as_deref().unwrap_or("self");
        let player = self.game_state.resolve_target_player(target);
        let card_db = &self.game_state.card_database;
        let card_type_filter = condition.card_type.as_deref();
        let group_names = condition.group_names.as_ref();
        let operator = condition.operator.as_deref();
        let count_threshold = condition.count.unwrap_or(1);
        let locs = condition.locations.as_ref().unwrap();

        let mut combined: Vec<i16> = Vec::new();
        for loc in locs {
            let cards = util::zone_cards(player, loc.as_str());
            combined.extend_from_slice(cards);
        }
        eprintln!("[MULTI] combined {} cards", combined.len());

        if condition.distinct.unwrap_or(false) {
            let mut distinct_names: std::collections::HashSet<String> = std::collections::HashSet::new();
            for &cid in &combined {
                if cid == -1 { eprintln!("[MULTI]   skipping -1"); continue; }
                let passes_type = card_type_filter.map_or(true, |f| util::card_matches_type(card_db, cid, Some(f)));
                let passes_group = group_names.map_or(true, |gn| {
                    card_db.get_card(cid).map(|c| gn.iter().any(|g| c.group == *g || c.unit.as_deref() == Some(g.as_str()))).unwrap_or(false)
                });
                eprintln!("[MULTI]   card={} type_pass={} group_pass={}", cid, passes_type, passes_group);
                if !passes_type || !passes_group { continue; }
                let names = card_db.get_card_names(cid);
                for name in &names {
                    eprintln!("[MULTI]   name='{}'", name);
                    distinct_names.insert(name.clone());
                }
            }
            let count = distinct_names.len() as u32;
            eprintln!("[MULTI] distinct_names={} threshold={}", count, count_threshold);
            compare_counts(operator, count, count_threshold)
        } else {
            let filter = util::CardFilter {
                card_type: card_type_filter,
                group: group_names.and_then(|g| g.first().map(|s| s.as_str())),
                cost_limit: condition.cost_limit,
                cost_operator: operator,
                ..util::CardFilter::default()
            };
            let matching_count = util::count_matching(&combined, card_db, &filter, false);
            compare_counts(operator, matching_count, count_threshold)
        }
    }

    fn evaluate_position_condition(&self, condition: &Condition) -> bool {
        let target = condition.target.as_deref().unwrap_or("self");
        let position = condition.position.as_ref().and_then(|p| p.get_position()).unwrap_or("");
        let player = self.game_state.resolve_target_player(target);
        match position {
            "center" => player.stage.stage[1] != -1,
            "left_side" => player.stage.stage[0] != -1,
            "right_side" => player.stage.stage[2] != -1,
            "any" => player.stage.stage[0] != -1 || player.stage.stage[1] != -1 || player.stage.stage[2] != -1,
            _ => true,
        }
    }

    fn evaluate_group_condition(&self, condition: &Condition) -> bool {
        // When all_members is true, every card on stage must match the group ("のみ")
        if condition.all_members.unwrap_or(false) {
            let target = condition.target.as_deref().unwrap_or("self");
            let player = self.game_state.resolve_target_player(target);
            let group_name = condition.group.as_ref()
                .and_then(|g| g.get("name").and_then(|n| n.as_str()))
                .or_else(|| condition.group_names.as_ref().and_then(|gn| gn.first().map(|s| s.as_str())));
            let card_db = &self.game_state.card_database;
            // Every non-empty stage slot must match the group
            return player.stage.stage.iter().filter(|&&id| id != -1).all(|&id| {
                crate::ability::util::card_matches_group_str(card_db, id, group_name)
            });
        }
        let count = self.get_group_card_count(condition);
        // Default to 1 when count is not set (checking "at least one matching")
        let target_count = condition.count.unwrap_or(1);
        compare_counts(condition.operator.as_deref(), count, target_count)
    }

    fn evaluate_card_count_condition(&self, condition: &Condition) -> bool {
        let card_type = condition.card_type.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let count = condition.count.unwrap_or(1);
        let player = self.game_state.resolve_target_player(target);
        let exclude_self = condition.exclude_self.unwrap_or(false);
        let activating_id = self.activating_card_id;
        let card_db = &self.game_state.card_database;

        let location = condition.location.as_deref().unwrap_or("");
        let group_names = condition.group_names.as_ref();
        let group = condition.group.as_ref()
            .and_then(|g| g.as_object().and_then(|o| o.get("name").and_then(|n| n.as_str())));

        if location == "revealed_cards" {
            let actual = self.game_state.revealed_cards.iter().filter(|&&cid| {
                let type_ok = match card_type {
                    "live_card" => card_db.get_card(cid).map(|c| c.is_live()).unwrap_or(false),
                    "member_card" => card_db.get_card(cid).map(|c| c.is_member()).unwrap_or(false),
                    "energy_card" => card_db.get_card(cid).map(|c| c.is_energy()).unwrap_or(false),
                    _ => true,
                };
                if !type_ok { return false; }
                if let Some(g) = group {
                    crate::ability::util::card_matches_group_str(card_db, cid, Some(g))
                } else if let Some(gn) = group_names {
                    gn.iter().any(|g| crate::ability::util::card_matches_group_str(card_db, cid, Some(g)))
                } else { true }
            }).count() as u32;
            return compare_counts(condition.operator.as_deref(), actual, count);
        }

        let stage_member_count = |p: &crate::player::Player| -> usize {
            p.stage.stage.iter().filter(|&&id| id != -1).count()
        };

        let actual_count = match condition.unit.as_deref() {
            Some("types") => {
                // Count distinct heart color types among stage members
                let mut color_types = std::collections::HashSet::new();
                let card_db = &self.game_state.card_database;
                for &cid in &player.stage.stage {
                    if cid == -1 { continue; }
                    if let Some(card) = card_db.get_card(cid) {
                        if let Some(ref base_heart) = card.base_heart {
                            for (color, _) in &base_heart.hearts {
                                color_types.insert(*color);
                            }
                        }
                    }
                }
                color_types.len() as u32
            }
            _ => {
                let count = match card_type {
                    "live_card" => player.live_card_zone.len(),
                    "member_card" => {
                        if condition.aggregate.as_deref() == Some("total") {
                            // Sum blade values across stage members
                            player.stage.total_blades(card_db, &self.game_state.blade_modifiers) as usize
                        } else {
                            let mut member_count = stage_member_count(player);
                            if exclude_self {
                                if let Some(cid) = activating_id {
                                    if player.stage.stage.iter().any(|&id| id == cid) {
                                        member_count = member_count.saturating_sub(1);
                                    }
                                }
                            }
                            member_count
                        }
                    }
                    "energy_card" => player.energy_zone.cards.len(),
                    _ => {
                        // Check for special locations
                        if condition.location.as_deref() == Some("revealed_cards") {
                            self.game_state.revealed_cards.len() as usize
                        } else {
                            0 as usize
                        }
                    }
                };
                count as u32
            }
        };
        compare_counts(condition.operator.as_deref(), actual_count, count)
    }

    fn evaluate_appearance_condition(&self, condition: &Condition) -> bool {
        let appearance = condition.appearance.unwrap_or(false);
        let location = condition.location.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let baton_touch_trigger = condition.baton_touch_trigger.unwrap_or(false);
        let player = self.game_state.resolve_target_player(target);

        if baton_touch_trigger {
            if let Some(ref _activating_card) = self.game_state.activating_card {
                return self.game_state.baton_touch_count > 0;
            }
            return false;
        }

        if appearance {
            match location {
                "stage" => player.stage.stage.iter().any(|&card_id| card_id != -1),
                "hand" => !player.hand.cards.is_empty(),
                "discard" => !player.waitroom.cards.is_empty(),
                _ => true,
            }
        } else {
            match location {
                "stage" => player.stage.stage[0] == -1 && player.stage.stage[1] == -1 && player.stage.stage[2] == -1,
                "hand" => player.hand.cards.is_empty(),
                "discard" => player.waitroom.cards.is_empty(),
                _ => true,
            }
        }
    }

    fn evaluate_temporal_condition(&self, condition: &Condition) -> bool {
        let temporal = condition.temporal.as_deref().unwrap_or("");
        let phase = condition.phase.as_deref();

        match temporal {
            "this_turn" => {
                if let Some(count) = condition.count {
                    if condition.location.as_deref() == Some("stage")
                        && condition.card_type.as_deref() == Some("member_card")
                    {
                        let target = condition.target.as_deref().unwrap_or("self");
                        let player = self.game_state.resolve_target_player(target);
                        return player.debut_count_this_turn >= count;
                    }
                }
                if let Some(created_turn) = condition.temporal_scope.as_ref().and_then(|s| s.parse::<u32>().ok()) {
                    created_turn == self.game_state.turn_number
                } else {
                    if let Some(nested_condition) = &condition.condition {
                        match nested_condition.condition_type.as_deref() {
                            Some("not_moved") => {
                                if let Some(activating_card_id) = self.activating_card_id {
                                    !self.game_state.has_card_moved_this_turn(activating_card_id)
                                } else { true }
                            }
                            Some("has_moved") => {
                                if let Some(activating_card_id) = self.activating_card_id {
                                    self.game_state.has_card_moved_this_turn(activating_card_id)
                                } else { false }
                            }
                            _ => self.evaluate_condition(nested_condition),
                        }
                    } else { true }
                }
            }
            "live_end" => matches!(self.game_state.current_phase, crate::game_state::Phase::LiveVictoryDetermination),
            "this_live" => {
                matches!(self.game_state.current_phase, crate::game_state::Phase::LiveCardSetP1Turn) ||
                matches!(self.game_state.current_phase, crate::game_state::Phase::LiveCardSetP2Turn) ||
                matches!(self.game_state.current_phase, crate::game_state::Phase::FirstAttackerPerformance) ||
                matches!(self.game_state.current_phase, crate::game_state::Phase::SecondAttackerPerformance) ||
                matches!(self.game_state.current_phase, crate::game_state::Phase::LiveVictoryDetermination)
            }
            "before_live" => {
                !matches!(self.game_state.current_phase, crate::game_state::Phase::LiveCardSetP1Turn) &&
                !matches!(self.game_state.current_phase, crate::game_state::Phase::LiveCardSetP2Turn) &&
                !matches!(self.game_state.current_phase, crate::game_state::Phase::FirstAttackerPerformance) &&
                !matches!(self.game_state.current_phase, crate::game_state::Phase::SecondAttackerPerformance) &&
                !matches!(self.game_state.current_phase, crate::game_state::Phase::LiveVictoryDetermination)
            }
            "first_turn" => self.game_state.is_first_turn,
            _ => {
                if let Some(phase_str) = phase {
                    match phase_str {
                        "active" => matches!(self.game_state.current_phase, crate::game_state::Phase::Active),
                        "live_card_set" => matches!(self.game_state.current_phase, crate::game_state::Phase::LiveCardSetP1Turn | crate::game_state::Phase::LiveCardSetP2Turn),
                        "live_performance" => matches!(self.game_state.current_phase, crate::game_state::Phase::FirstAttackerPerformance) ||
                                               matches!(self.game_state.current_phase, crate::game_state::Phase::SecondAttackerPerformance),
                        "live_victory" => matches!(self.game_state.current_phase, crate::game_state::Phase::LiveVictoryDetermination),
                        _ => true,
                    }
                } else { true }
            }
        }
    }

    fn evaluate_state_condition(&self, condition: &Condition) -> bool {
        let state = condition.state.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let resource_type = condition.resource_type.as_deref();
        let all_cards = condition.all.unwrap_or(false);
        let player = self.game_state.resolve_target_player(target);

        if resource_type == Some("energy") {
            match state {
                "active" => {
                    if all_cards {
                        player.energy_zone.active_energy_count == player.energy_zone.cards.len()
                    } else {
                        player.energy_zone.active_energy_count > 0
                    }
                }
                "wait" => {
                    if all_cards {
                        player.energy_zone.active_energy_count == 0
                    } else {
                        player.energy_zone.active_energy_count < player.energy_zone.cards.len()
                    }
                }
                _ => true,
            }
        } else {
            match state {
                "active" | "wait" => player.stage.stage.iter().any(|&card_id| card_id != -1),
                _ => true,
            }
        }
    }

    fn evaluate_energy_state_condition(&self, condition: &Condition) -> bool {
        let energy_state = condition.energy_state.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let player = self.game_state.resolve_target_player(target);
        match energy_state {
            "active" => player.energy_zone.active_count() > 0,
            _ => true,
        }
    }

    fn evaluate_movement_condition(&self, condition: &Condition) -> bool {
        let movement = condition.movement.as_deref().unwrap_or("");
        let movement_state = condition.movement_state.as_deref();
        let location = condition.location.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let player = self.game_state.resolve_target_player(target);

        match movement {
            "moved" => {
                if let Some(state) = movement_state {
                    match state {
                        "to_stage" => player.stage.stage[0] != -1 || player.stage.stage[1] != -1 || player.stage.stage[2] != -1,
                        "from_stage" => !player.waitroom.cards.is_empty(),
                        "to_discard" => !player.waitroom.cards.is_empty(),
                        _ => true,
                    }
                } else { true }
            }
            "notmoved" => true,
            "baton_touch" => condition.baton_touch_trigger.unwrap_or(false),
            _ => {
                match location {
                    "stage" => player.stage.stage[0] != -1 || player.stage.stage[1] != -1 || player.stage.stage[2] != -1,
                    "hand" => !player.hand.cards.is_empty(),
                    "discard" => !player.waitroom.cards.is_empty(),
                    _ => true,
                }
            }
        }
    }

    fn evaluate_ability_negation_condition(&self, condition: &Condition) -> bool {
        let negation = condition.negation.unwrap_or(false);
        if negation {
            self.game_state.prohibition_effects.is_empty()
        } else { true }
    }

    fn evaluate_or_condition(&self, condition: &Condition) -> bool {
        if let Some(ref conditions) = condition.conditions {
            conditions.iter().any(|c| self.evaluate_condition(c))
        } else { true }
    }

    fn evaluate_any_of_condition(&self, condition: &Condition) -> bool {
        if let Some(ref any_of) = condition.any_of {
            any_of.iter().any(|condition_type| {
                let master = &*self.game_state.resolve_target_player("self");
                match condition_type.as_str() {
                    "has_member" => !master.stage.stage.iter().all(|&id| id == crate::constants::EMPTY_SLOT),
                    "has_energy" => !master.energy_zone.cards.is_empty(),
                    "has_hand" => !master.hand.cards.is_empty(),
                    "has_blade_heart" => master.stage.stage.iter().any(|&id| {
                        id != crate::constants::EMPTY_SLOT && self.game_state.card_database.get_card(id).map(|c| c.has_blade_heart()).unwrap_or(false)
                    }),
                    "has_live_card" => !master.live_card_zone.cards.is_empty(),
                    "is_active_phase" => matches!(self.game_state.current_phase, crate::game_state::Phase::Active),
                    "is_main_phase" => matches!(self.game_state.current_phase, crate::game_state::Phase::Main),
                    _ => { eprintln!("Unknown any_of condition type: {}", condition_type); false }
                }
            })
        } else { true }
    }

    fn evaluate_score_threshold_condition(&self, condition: &Condition) -> bool {
        // Default to 1 when count is not set (checking "score exists")
        let count = condition.count.unwrap_or(1);
        let operator = condition.operator.as_deref();
        let target = condition.target.as_deref().unwrap_or("self");
        let cheer_count = if target == "self" { self.game_state.player1_cheer_blade_heart_count } else if target == "opponent" { self.game_state.player2_cheer_blade_heart_count } else { self.game_state.player1_cheer_blade_heart_count };
        compare_counts(operator, cheer_count, count)
    }

    fn evaluate_choice_condition(&self, condition: &Condition) -> bool {
        if let Some(ref options) = condition.options {
            !options.is_empty()
        } else {
            true
        }
    }

    fn evaluate_position_change_condition(&self, condition: &Condition) -> bool {
        let optional = condition.options.as_ref().map(|_| true).unwrap_or(false);
        if optional {
            if self.game_state.position_change_occurred_this_turn {
                return true;
            }
            return false;
        }
        self.game_state.position_change_occurred_this_turn
    }

    fn evaluate_state_change_condition(&self, condition: &Condition) -> bool {
        let _during_main_phase = condition.text.contains("main_phase");
        if _during_main_phase && self.game_state.current_phase != Phase::Main {
            return false;
        }
        if let (Some(from), Some(to)) = (condition.from_state.as_deref(), condition.to_state.as_deref()) {
            if let Some(target_count) = condition.count {
                if condition.operator.as_deref() == Some(">=") {
                    let actual = self.game_state.last_state_change_wait_to_active_count;
                    return actual >= target_count;
                }
            }
            let target = condition.target.as_deref().unwrap_or("self");
            let is_opponent = target == "opponent" || condition.text.contains("相手");
            let player = if is_opponent {
                self.game_state.resolve_target_player("opponent")
            } else {
                self.game_state.resolve_target_player("self")
            };
            // Check if any member on the target player's stage has the target orientation
            let orientation_check = |card_id: i16| -> bool {
                let o = self.game_state.get_orientation_modifier(card_id);
                match (from, to) {
                    ("active", "wait") => o.map_or(false, |s| s == "wait"),
                    ("wait", "active") => o.map_or(true, |s| s == "active"),
                    _ => false,
                }
            };
            for &card_id in &player.stage.stage {
                if card_id != -1 && orientation_check(card_id) {
                    return true;
                }
            }
            return false;
        }
        true
    }

    fn evaluate_opponent_choice_condition(&self, condition: &Condition) -> bool {
        let _target = condition.target.as_deref().unwrap_or("opponent");
        let negation = condition.negation.unwrap_or(false);
        let opponent_declined = self.game_state.opponent_choice_declined;
        if negation { opponent_declined } else { !opponent_declined }
    }

    fn evaluate_opponent_live_success_condition(&self, condition: &Condition) -> bool {
        if !self.game_state.opponent_live_success_this_turn {
            return false;
        }
        // If no_excess_heart is set, also check the stored flag
        if condition.no_excess_heart.unwrap_or(false) {
            return self.game_state.opponent_live_no_excess_heart_this_turn;
        }
        true
    }

    fn evaluate_no_excess_heart_condition(&self, condition: &Condition) -> bool {
        let target = condition.target.as_deref().unwrap_or("self");
        if target == "opponent" {
            self.game_state.opponent_live_no_excess_heart_this_turn
        } else {
            self.game_state.self_no_excess_heart_this_turn
        }
    }

    fn evaluate_complex_condition(&self, condition: &Condition) -> bool {
        if let Some(ref cause) = condition.cause {
            self.evaluate_condition(cause)
        } else {
            true
        }
    }

    fn zone_len(&self, player: &crate::player::Player, location: &str) -> u32 {
        match location {
            "stage" => player.stage.total_blades(&self.game_state.card_database, &self.game_state.blade_modifiers),
            "hand" => player.hand.len() as u32,
            "deck" => player.main_deck.len() as u32,
            "discard" => player.waitroom.len() as u32,
            "energy_zone" => player.energy_zone.cards.len() as u32,
            "live_card_zone" => player.live_card_zone.len() as u32,
            "success_live_zone" => player.success_live_card_zone.len() as u32,
            _ => 0,
        }
    }

    fn get_count_for_condition(&self, condition: &Condition) -> u32 {
        let location = condition.location.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let comparison_type = condition.comparison_type.as_deref();
        let resource_type = condition.resource_type.as_deref();
        if comparison_type == Some("score") {
            return self.get_count_for_target(condition, target);
        }
        if comparison_type == Some("cost") && condition.location.is_none() {
            return self.game_state.revealed_cost_cards.iter()
                .filter_map(|&id| self.game_state.card_database.get_card(id))
                .filter_map(|c| c.cost)
                .sum();
        }
        if resource_type == Some("hand_count") {
            let player = self.game_state.resolve_target_player(target);
            return player.hand.len() as u32;
        }
        if resource_type == Some("surplus_heart") {
            let player = self.game_state.resolve_target_player(target);
            let card_db = &self.game_state.card_database;
            let member_hearts: u32 = player.stage.stage.iter()
                .filter(|&&id| id != -1)
                .map(|&id| card_db.get_card(id).map(|c| c.total_hearts()).unwrap_or(0))
                .sum();
            let needed: u32 = player.live_card_zone.cards.iter()
                .chain(player.success_live_card_zone.cards.iter())
                .map(|&id| card_db.get_card(id).map(|c| c.total_hearts()).unwrap_or(0))
                .sum();
            return member_hearts.saturating_sub(needed);
        }
        let player = self.game_state.resolve_target_player(target);
        self.zone_len(player, location)
    }

    fn get_count_for_target(&self, condition: &Condition, target: &str) -> u32 {
        let location = condition.location.as_deref().unwrap_or("");
        let comparison_type = condition.comparison_type.as_deref();
        let player = self.game_state.resolve_target_player(target);

        if let Some(comp_type) = comparison_type {
            match comp_type {
                "score" => {
                    let mut total_score = 0;
                    for card_id in player.success_live_card_zone.cards.iter().chain(player.live_card_zone.cards.iter()) {
                        if let Some(card) = self.game_state.card_database.get_card(*card_id) {
                            total_score += card.score.unwrap_or(0);
                        }
                    }
                    total_score
                }
                "cost" => {
                    let mut total_cost = 0;
                    for card_id in &player.stage.stage {
                        if *card_id != -1 {
                            if let Some(card) = self.game_state.card_database.get_card(*card_id) {
                                total_cost += card.cost.unwrap_or(0);
                            }
                        }
                    }
                    total_cost
                }
                "energy" => player.energy_zone.cards.len() as u32,
                _ => self.zone_len(player, location),
            }
        } else {
            self.zone_len(player, location)
        }
    }

    fn get_group_card_count(&self, condition: &Condition) -> u32 {
        let group_filter = condition.group_names.as_ref();
        let location = condition.location.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let player = self.game_state.resolve_target_player(target);
        let mut count = 0;
        let card_db = self.game_state.card_database.clone();

        // Use group name from either group_names or group.value.name
        let group_name = group_filter.and_then(|g| g.first().map(|s| s.as_str()))
            .or_else(|| condition.group.as_ref().and_then(|g| g.get("name").and_then(|n| n.as_str())));

        let is_aggregate = condition.aggregate.as_deref() == Some("total");

        let matches_group = |card_id: i16| -> bool {
            util::card_matches_group_str(&card_db, card_id, group_name)
        };

        // When aggregate="total", sum heart values instead of counting cards
        if is_aggregate {
            let mut total = 0u32;
            for i in 0..3 {
                let cid = player.stage.stage[i];
                if cid == -1 || !matches_group(cid) { continue; }
                if let Some(card) = card_db.get_card(cid) {
                    if let Some(ref bh) = card.base_heart {
                        for (_color, val) in &bh.hearts {
                            total += val;
                        }
                    }
                }
            }
            return total;
        }

        match location {
            "stage" => {
                for i in 0..3 {
                    if player.stage.stage[i] != -1 && matches_group(player.stage.stage[i]) {
                        count += 1;
                    }
                }
            }
            "hand" => {
                for card in util::zone_cards(player, "hand") {
                    if matches_group(*card) { count += 1; }
                }
            }
            "discard" | "waitroom" => {
                for card in util::zone_cards(player, "discard") {
                    if matches_group(*card) { count += 1; }
                }
            }
            _ => {}
        }
        count
    }
}
