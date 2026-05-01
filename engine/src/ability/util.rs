use crate::card::CardDatabase;

#[allow(dead_code)]
pub fn card_matches_type(card_db: &CardDatabase, card_id: i16, card_type_filter: Option<&str>) -> bool {
    match card_type_filter {
        Some("live_card") => card_db.get_card(card_id).map(|c| c.is_live()).unwrap_or(false),
        Some("member_card") => card_db.get_card(card_id).map(|c| c.is_member()).unwrap_or(false),
        Some("energy_card") => card_db.get_card(card_id).map(|c| c.is_energy()).unwrap_or(false),
        None => true,
        _ => true,
    }
}

#[allow(dead_code)]
pub fn card_matches_group(card_db: &CardDatabase, card_id: i16, group_filter: Option<&String>) -> bool {
    match group_filter {
        Some(group_name) => card_db.get_card(card_id).map(|c| c.group == *group_name).unwrap_or(false),
        None => true,
    }
}

#[allow(dead_code)]
pub fn card_matches_cost_limit(card_db: &CardDatabase, card_id: i16, cost_limit: Option<u32>) -> bool {
    match cost_limit {
        Some(max_cost) => card_db.get_card(card_id).and_then(|c| c.cost).map(|c| c <= max_cost).unwrap_or(false),
        None => true,
    }
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

#[allow(dead_code)]
pub fn zone_card_count(cards: &[i16], card_db: &CardDatabase, card_type_filter: Option<&str>) -> u32 {
    if let Some(filter) = card_type_filter {
        cards.iter().filter(|&&id| card_matches_type(card_db, id, Some(filter))).count() as u32
    } else {
        cards.len() as u32
    }
}

#[allow(dead_code)]
pub fn sum_score_in_zone(cards: &[i16], card_db: &CardDatabase, get_modifier: impl Fn(i16) -> i32) -> u32 {
    cards.iter().map(|&id| {
        let base = card_db.get_card(id).map(|c| c.get_score()).unwrap_or(0);
        (base as i32 + get_modifier(id)) as u32
    }).sum()
}
