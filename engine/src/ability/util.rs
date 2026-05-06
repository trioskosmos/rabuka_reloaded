use crate::card::CardDatabase;
use crate::zones::parse_heart_color;

// ============== INDIVIDUAL CARD PREDICATES ==============

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
        Some(g) => card_db.get_card(card_id).map(|c| {
            c.unit.as_deref() == Some(g)
                || c.group == g
                || card_series_matches_group(&c.series, g)
        }).unwrap_or(false),
        None => true,
    }
}

fn card_series_matches_group(series: &str, group: &str) -> bool {
    match group {
        "Aqours" => series.contains("サンシャイン"),
        "虹ヶ咲" => series.contains("虹ヶ咲"),
        "Liella!" => series.contains("スーパースター"),
        "蓮ノ空" => series.contains("蓮ノ空"),
        "μ's" => series.contains("ラブライブ！") && !series.contains("サンシャイン")
            && !series.contains("虹ヶ咲") && !series.contains("スーパースター") && !series.contains("蓮ノ空"),
        _ => false,
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
            let result = card_db.get_card(card_id).map_or(true, |card| {
                colors.iter().any(|color| {
                    let hc = parse_heart_color(color);
                    let r = card.base_heart.as_ref().map_or(
                        card.need_heart.as_ref().map_or(false, |need| need.hearts.contains_key(&hc)),
                        |base| base.hearts.contains_key(&hc),
                    );
                    r
                })
            });
            eprintln!("[HEART_COLORS] card_id={} result={}", card_id, result);
            result
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

// ============== UNIFIED FILTER STRUCT ==============

/// Unified card filter: all fields are Optional — None = match anything.
#[derive(Default, Clone)]
pub struct CardFilter<'a> {
    pub card_type: Option<&'a str>,
    pub group: Option<&'a str>,
    pub cost_limit: Option<u32>,
    pub cost_operator: Option<&'a str>,
    pub characters: Option<&'a Vec<String>>,
    pub heart_colors: Option<&'a Vec<String>>,
    pub name_fragments: Option<&'a Vec<String>>,
    pub distinct: Option<&'a str>,
    pub exclude_self: Option<i16>,
    pub original_blade_limit: Option<u32>,
    pub original_blade_operator: Option<&'a str>,
}

impl<'a> CardFilter<'a> {
    pub fn new() -> Self { Self::default() }

    /// Check whether a single card matches ALL present filter fields.
    pub fn matches(&self, db: &CardDatabase, id: i16, skip_empty: bool) -> bool {
        if skip_empty && id == -1 { return false; }
        if let Some(ct) = self.card_type { if !card_matches_type(db, id, Some(ct)) { return false; } }
        if let Some(g) = self.group { if !card_matches_group_str(db, id, Some(g)) { return false; } }
        if let Some(lim) = self.cost_limit { if !card_matches_cost_limit_op(db, id, Some(lim), self.cost_operator) { return false; } }
        if let Some(ch) = self.characters { if !card_matches_characters(db, id, Some(ch)) { return false; } }
        if let Some(hc) = self.heart_colors { if !card_matches_heart_colors(db, id, Some(hc)) { return false; } }
        if let Some(name) = self.name_fragments { if !card_matches_name_fragments(db, id, name) { return false; } }
        if let Some(ex_id) = self.exclude_self { if id == ex_id { return false; } }
        if let Some(bl) = self.original_blade_limit {
            let card_blade = db.get_card(id).map(|c| c.blade).unwrap_or(0);
            if !compare_counts(self.original_blade_operator, card_blade, bl) { return false; }
        }
        true
    }
}

fn card_matches_name_fragments(db: &CardDatabase, id: i16, fragments: &[String]) -> bool {
    db.get_card(id).map_or(false, |card| {
        fragments.iter().all(|f| card.name.contains(f.as_str()))
    })
}

// ============== QUERY FUNCTIONS ==============

/// Return indices into `cards` where cards match the filter.
pub fn matching_indices(cards: &[i16], db: &CardDatabase, filter: &CardFilter, skip_empty: bool) -> Vec<usize> {
    cards.iter().enumerate()
        .filter(|(_, &id)| filter.matches(db, id, skip_empty))
        .map(|(i, _)| i)
        .collect()
}

/// Return card IDs from `cards` that match the filter.
pub fn matching_ids(cards: &[i16], db: &CardDatabase, filter: &CardFilter, skip_empty: bool) -> Vec<i16> {
    cards.iter().filter(|&&id| filter.matches(db, id, skip_empty)).copied().collect()
}

/// Count cards matching the filter.
pub fn count_matching(cards: &[i16], db: &CardDatabase, filter: &CardFilter, skip_empty: bool) -> u32 {
    cards.iter().filter(|&&id| filter.matches(db, id, skip_empty)).count() as u32
}

/// Deduplicate by card name when `filter.distinct` is set.
/// Returns indices into `cards`, deduplicated by card name.
pub fn filter_distinct(cards: &[i16], db: &CardDatabase, filter: &CardFilter, skip_empty: bool) -> Vec<usize> {
    let ids: Vec<usize> = matching_indices(cards, db, filter, skip_empty);
    let distinct = match filter.distinct {
        Some("card_name") | Some("true") | Some("distinct") => true,
        _ => return ids,
    };
    if !distinct { return ids; }
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    ids.into_iter().filter(|&i| {
        db.get_card(cards[i])
            .map(|c| seen.insert(c.name.clone()))
            .unwrap_or(true)
    }).collect()
}

// ============== ZONE HELPERS ==============

/// Resolve a named zone to an immutable card slice.
pub fn zone_cards<'a>(player: &'a crate::player::Player, zone: &str) -> &'a [i16] {
    match zone {
        "stage" => &player.stage.stage,
        "hand" => &player.hand.cards,
        "deck" => &player.main_deck.cards,
        "discard" | "waitroom" => &player.waitroom.cards,
        "energy_zone" => &player.energy_zone.cards,
        "live_card_zone" => &player.live_card_zone.cards,
        "success_live_zone" => &player.success_live_card_zone.cards,
        _ => &[],
    }
}

pub fn zone_card_count(cards: &[i16], card_db: &CardDatabase, card_type_filter: Option<&str>) -> u32 {
    if let Some(filter) = card_type_filter {
        cards.iter().filter(|&&id| card_matches_type(card_db, id, Some(filter))).count() as u32
    } else {
        cards.len() as u32
    }
}

/// Count matching cards with an additional blade-constraint closure.
/// Convenience for condition.rs — uses CardFilter internally.
pub fn count_matching_with_blade(
    cards: &[i16], card_db: &CardDatabase,
    card_type: Option<&str>, group: Option<&str>,
    cost_limit: Option<u32>, cost_op: Option<&str>,
    blade_filter: impl Fn(i16) -> bool,
) -> u32 {
    cards.iter().filter(|&&id| {
        id != -1
            && card_matches_type(card_db, id, card_type)
            && card_matches_group_str(card_db, id, group)
            && card_matches_cost_limit_op(card_db, id, cost_limit, cost_op)
            && blade_filter(id)
    }).count() as u32
}

// ============== UTILITY ==============

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

pub fn sum_score_in_zone(cards: &[i16], card_db: &CardDatabase, get_modifier: impl Fn(i16) -> i32) -> u32 {
    cards.iter().map(|&id| {
        let base = card_db.get_card(id).map(|c| c.get_score()).unwrap_or(0);
        (base as i32 + get_modifier(id)) as u32
    }).sum()
}

/// Place a card in the given destination zone, handling all zone types.
/// Returns true if the card was placed, false if skipped (stage full with max).
pub fn place_card_in_zone(
    player: &mut crate::player::Player,
    card_id: i16,
    destination: &str,
    vacated_stage_area: Option<usize>,
    is_max: bool,
    count: usize,
) -> bool {
    match destination {
        "hand" => { player.hand.add_card(card_id); true }
        "discard" | "" => { player.waitroom.add_card(card_id); true }
        "stage" | "empty_area" => {
            let empty_slots: Vec<usize> = (0..3).filter(|&i| player.stage.stage[i] == -1).collect();
            if is_max && empty_slots.len() < count { return false; }
            if let Some(pos) = stage_first_empty(&player.stage.stage) {
                player.stage.stage[pos] = card_id;
                player.areas_locked_this_turn.insert(pos_to_area(pos));
            } else {
                player.hand.add_card(card_id);
            }
            true
        }
        "deck_top" => { player.main_deck.cards.insert(0, card_id); true }
        "deck_bottom" => { player.main_deck.cards.push(card_id); true }
        "deck" => { player.main_deck.cards.insert(0, card_id); true }
        "energy_zone" => { player.energy_zone.cards.push(card_id); true }
        "live_card_zone" => { player.live_card_zone.cards.push(card_id); true }
        "success_live_zone" => { player.success_live_card_zone.cards.push(card_id); true }
        "same_area" => {
            if let Some(pos) = vacated_stage_area {
                if pos < 3 && player.stage.stage[pos] == -1 {
                    player.stage.stage[pos] = card_id;
                    player.areas_locked_this_turn.insert(pos_to_area(pos));
                } else if let Some(ep) = stage_first_empty(&player.stage.stage) {
                    player.stage.stage[ep] = card_id;
                    player.areas_locked_this_turn.insert(pos_to_area(ep));
                } else { player.hand.add_card(card_id); }
            } else if let Some(ep) = stage_first_empty(&player.stage.stage) {
                player.stage.stage[ep] = card_id;
                player.areas_locked_this_turn.insert(pos_to_area(ep));
            } else { player.hand.add_card(card_id); }
            true
        }
        _ => { player.hand.add_card(card_id); true }
    }
}

fn stage_first_empty(stage: &[i16; 3]) -> Option<usize> {
    if stage[1] == -1 { Some(1) }
    else if stage[0] == -1 { Some(0) }
    else if stage[2] == -1 { Some(2) }
    else { None }
}

fn pos_to_area(pos: usize) -> crate::zones::MemberArea {
    match pos { 0 => crate::zones::MemberArea::LeftSide, 1 => crate::zones::MemberArea::Center, _ => crate::zones::MemberArea::RightSide }
}

