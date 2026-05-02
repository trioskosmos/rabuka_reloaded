use crate::card::CardDatabase;
use crate::zones::parse_heart_color;

pub fn card_matches_type(card_db: &CardDatabase, card_id: i16, card_type_filter: Option<&str>) -> bool {
    match card_type_filter {
        Some("live_card") => card_db.get_card(card_id).map(|c| c.is_live()).unwrap_or(false),
        Some("member_card") => card_db.get_card(card_id).map(|c| c.is_member()).unwrap_or(false),
        Some("energy_card") => card_db.get_card(card_id).map(|c| c.is_energy()).unwrap_or(false),
        None => true,
        _ => true,
    }
}

pub fn card_matches_group(card_db: &CardDatabase, card_id: i16, group_filter: Option<&String>) -> bool {
    match group_filter {
        Some(group_name) => card_db.get_card(card_id).map(|c| c.group == *group_name).unwrap_or(false),
        None => true,
    }
}

pub fn card_matches_group_str(card_db: &CardDatabase, card_id: i16, group_name: Option<&str>) -> bool {
    match group_name {
        Some(g) => card_db.get_card(card_id).map(|c| c.group == g).unwrap_or(false),
        None => true,
    }
}

pub fn card_matches_characters(card_db: &CardDatabase, card_id: i16, characters: Option<&Vec<String>>) -> bool {
    match characters {
        Some(names) if !names.is_empty() => {
            card_db.get_card(card_id).map_or(false, |card| {
                names.iter().any(|name| card.name.contains(name.as_str()))
            })
        }
        _ => true,
    }
}

pub fn card_matches_cost_limit(card_db: &CardDatabase, card_id: i16, cost_limit: Option<u32>) -> bool {
    card_matches_cost_limit_op(card_db, card_id, cost_limit, None)
}

pub fn card_matches_cost_limit_op(card_db: &CardDatabase, card_id: i16, cost_limit: Option<u32>, comparison: Option<&str>) -> bool {
    match cost_limit {
        Some(limit) => card_db.get_card(card_id).and_then(|c| c.cost).map(|cost| {
            match comparison {
                Some("min") | Some(">=") => cost >= limit,
                Some("exact") | Some("=") => cost == limit,
                Some(">") => cost > limit,
                Some("<") => cost < limit,
                _ => cost <= limit,
            }
        }).unwrap_or(false),
        None => true,
    }
}

pub fn card_matches_heart_colors(card_db: &CardDatabase, card_id: i16, heart_colors: Option<&Vec<String>>) -> bool {
    match heart_colors {
        Some(colors) if !colors.is_empty() => {
            card_db.get_card(card_id).map_or(true, |card| {
                colors.iter().any(|color| {
                    let hc = parse_heart_color(color);
                    card.base_heart.as_ref().map_or(false, |base| base.hearts.contains_key(&hc))
                })
            })
        }
        _ => true,
    }
}

pub fn card_matches_name_constraint(card_db: &CardDatabase, card_id: i16, name_constraint: Option<&str>) -> bool {
    match name_constraint {
        Some(name) => card_db.get_card(card_id).map(|c| c.name == name).unwrap_or(false),
        None => true,
    }
}

pub fn card_matches_all_filters(
    card_db: &CardDatabase, card_id: i16,
    card_type: Option<&str>,
    group_name: Option<&str>,
    cost_limit: Option<u32>,
    cost_comparison: Option<&str>,
    heart_colors: Option<&Vec<String>>,
    characters: Option<&Vec<String>>,
    name_constraint: Option<&str>,
) -> bool {
    if !card_matches_type(card_db, card_id, card_type) { return false; }
    if !card_matches_group_str(card_db, card_id, group_name) { return false; }
    if !card_matches_cost_limit_op(card_db, card_id, cost_limit, cost_comparison) { return false; }
    if !card_matches_heart_colors(card_db, card_id, heart_colors) { return false; }
    if !card_matches_characters(card_db, card_id, characters) { return false; }
    if !card_matches_name_constraint(card_db, card_id, name_constraint) { return false; }
    true
}

pub fn count_matching(
    cards: &[i16], card_db: &CardDatabase,
    card_type: Option<&str>,
    group_name: Option<&str>,
    cost_limit: Option<u32>,
    cost_comparison: Option<&str>,
) -> u32 {
    cards.iter()
        .filter(|&&id| card_matches_type(card_db, id, card_type)
            && card_matches_group_str(card_db, id, group_name)
            && card_matches_cost_limit_op(card_db, id, cost_limit, cost_comparison))
        .count() as u32
}

pub fn matching_indices(
    cards: &[i16], card_db: &CardDatabase,
    card_type_filter: Option<&str>,
    group_name: Option<&str>,
    cost_limit: Option<u32>,
) -> Vec<usize> {
    cards.iter().enumerate()
        .filter(|(_, &id)| card_matches_type(card_db, id, card_type_filter)
            && card_matches_group_str(card_db, id, group_name)
            && card_matches_cost_limit(card_db, id, cost_limit))
        .map(|(i, _)| i)
        .collect()
}

pub fn compare_counts(operator: Option<&str>, actual: u32, expected: u32) -> bool {
    match operator {
        Some(">=") => actual >= expected,
        Some(">") => actual > expected,
        Some("<=") => actual <= expected,
        Some("<") => actual < expected,
        Some("==") | Some("=") => actual == expected,
        Some("!=") => actual != expected,
        _ => true,
    }
}

pub fn zone_card_count(cards: &[i16], card_db: &CardDatabase, card_type_filter: Option<&str>) -> u32 {
    if let Some(filter) = card_type_filter {
        cards.iter().filter(|&&id| card_matches_type(card_db, id, Some(filter))).count() as u32
    } else {
        cards.len() as u32
    }
}

pub fn sum_score_in_zone(cards: &[i16], card_db: &CardDatabase, get_modifier: impl Fn(i16) -> i32) -> u32 {
    cards.iter().map(|&id| {
        let base = card_db.get_card(id).map(|c| c.get_score()).unwrap_or(0);
        (base as i32 + get_modifier(id)) as u32
    }).sum()
}
