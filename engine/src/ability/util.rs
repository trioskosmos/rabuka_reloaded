use crate::card::CardDatabase;
use crate::card::parse_heart_color;
use crate::game_state::Duration;

// ============== MODIFY COST ==============

pub fn find_modify_cost<'a>(effect: &'a crate::card::AbilityEffect, op: Option<&str>, loc: Option<&str>) -> Option<&'a crate::card::AbilityEffect> {
    if effect.action == "modify_cost"
        && op.map_or(true, |o| effect.operation.as_deref() == Some(o))
        && loc.map_or(true, |l| effect.location.as_deref() == Some(l))
    {
        return Some(effect);
    }
    if effect.action == "sequential" {
        if let Some(ref actions) = effect.compound.actions {
            for sub in actions {
                if let Some(found) = find_modify_cost(sub, op, loc) {
                    return Some(found);
                }
            }
        }
    }
    None
}

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
                // Check name fragments for multi-name cards (e.g. "にこ" in "矢澤にこ")
                || card_db.get_card_names(card_id).iter().any(|n| n.contains(g))
                // For multi-series cards (containing \n), don't match individual series
                // Multi-name cards' individuals each have their own series but the card
                // as a whole should not match group conditions (Q212)
                || (!c.series.contains('\n') && card_series_matches_group(&c.series, g))
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

pub fn card_matches_heart_colors(card_db: &CardDatabase, card_id: i16, heart_colors: &[String]) -> bool {
    if heart_colors.is_empty() { return true; }
    let result = card_db.get_card(card_id).map_or(true, |card| {
        heart_colors.iter().any(|color| {
            let hc = parse_heart_color(color);
            card.base_heart.as_ref().map_or(
                card.need_heart.as_ref().map_or(false, |need| need.hearts.contains_key(&hc)),
                |base| base.hearts.contains_key(&hc),
            )
        })
    });
    eprintln!("[HEART_COLORS] card_id={} result={}", card_id, result);
    result
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
    pub heart_colors: &'a [String],
    pub name_fragments: Option<&'a Vec<String>>,
    pub distinct: Option<&'a str>,
    pub exclude_self: Option<i16>,
    pub original_blade_limit: Option<u32>,
    pub original_blade_operator: Option<&'a str>,
}

impl<'a> CardFilter<'a> {
    pub fn new() -> Self { Self::default() }

    pub fn card_type(mut self, ct: &'a str) -> Self { self.card_type = Some(ct); self }
    pub fn card_type_opt(mut self, ct: Option<&'a str>) -> Self { self.card_type = ct; self }
    pub fn group(mut self, g: &'a str) -> Self { self.group = Some(g); self }
    pub fn group_opt(mut self, g: Option<&'a str>) -> Self { self.group = g; self }
    pub fn heart_colors(mut self, hc: &'a [String]) -> Self { self.heart_colors = hc; self }
    pub fn distinct(mut self, d: &'a str) -> Self { self.distinct = Some(d); self }
    pub fn exclude_self(mut self, id: i16) -> Self { self.exclude_self = Some(id); self }
    pub fn exclude_self_opt(mut self, id: Option<i16>) -> Self { self.exclude_self = id; self }

    /// Check whether a single card matches ALL present filter fields.
    pub fn matches(&self, db: &CardDatabase, id: i16, skip_empty: bool) -> bool {
        if skip_empty && id == -1 { return false; }
        if let Some(ct) = self.card_type { if !card_matches_type(db, id, Some(ct)) { return false; } }
        if let Some(g) = self.group { if !card_matches_group_str(db, id, Some(g)) { return false; } }
        if let Some(lim) = self.cost_limit { if !card_matches_cost_limit_op(db, id, Some(lim), self.cost_operator) { return false; } }
        if let Some(ch) = self.characters { if !card_matches_characters(db, id, Some(ch)) { return false; } }
        if !self.heart_colors.is_empty() { if !card_matches_heart_colors(db, id, self.heart_colors) { return false; } }
        if let Some(name) = self.name_fragments { if !card_matches_name_fragments(db, id, name) { return false; } }
        if let Some(ex_id) = self.exclude_self { if id == ex_id { return false; } }
        if let Some(bl) = self.original_blade_limit {
            let card_blade = db.get_card(id).map(|c| c.blade).unwrap_or(0);
            if !compare_counts(self.original_blade_operator, card_blade, bl) { return false; }
        }
        true
    }

    pub fn matches_card(&self, db: &CardDatabase, id: i16) -> bool {
        self.matches(db, id, false)
    }

    pub fn find_ids(&self, cards: &[i16], db: &CardDatabase) -> Vec<i16> {
        cards.iter().filter(|&&id| self.matches(db, id, false)).copied().collect()
    }

    pub fn count(&self, cards: &[i16], db: &CardDatabase) -> u32 {
        cards.iter().filter(|&&id| self.matches(db, id, false)).count() as u32
    }
}

fn card_matches_name_fragments(db: &CardDatabase, id: i16, fragments: &[String]) -> bool {
    db.get_card(id).map_or(false, |card| {
        fragments.iter().all(|f| card.name.contains(f.as_str()))
    })
}

// ============== FILTER CONSTRUCTION HELPERS ==============

/// Build a CardFilter from common fields used across effect/cost handlers.
pub fn filter_from_parts<'a>(
    card_type: Option<&'a str>,
    group: Option<&'a str>,
    cost_limit: Option<u32>,
    cost_operator: Option<&'a str>,
    characters: Option<&'a Vec<String>>,
    exclude_self: Option<i16>,
) -> CardFilter<'a> {
    CardFilter {
        card_type, group, cost_limit, cost_operator, characters,
        exclude_self,
        ..CardFilter::default()
    }
}

pub fn filter_from_parts_full<'a>(
    card_type: Option<&'a str>,
    group: Option<&'a str>,
    cost_limit: Option<u32>,
    cost_operator: Option<&'a str>,
    characters: Option<&'a Vec<String>>,
    name_fragments: Option<&'a Vec<String>>,
    distinct: Option<&'a str>,
    exclude_self: Option<i16>,
) -> CardFilter<'a> {
    CardFilter {
        card_type, group, cost_limit, cost_operator, characters,
        name_fragments, distinct, exclude_self,
        ..CardFilter::default()
    }
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
        "under_member" => {
            // Return all under_cards as a flat slice - needs allocation since
            // under_cards are stored as 3 separate SmallVecs.
            // This returns an empty slice; the caller must handle under_member separately.
            &[]
        },
        _ => &[],
    }
}

/// Return owned card IDs from a named zone (avoids borrow issues).
pub fn zone_card_ids(player: &crate::player::Player, zone: &str) -> Vec<i16> {
    zone_cards(player, zone).to_vec()
}

/// Count cards matching filter in a zone for a given player.
pub fn count_in_zone(player: &crate::player::Player, zone: &str, filter: &CardFilter, card_db: &CardDatabase) -> u32 {
    count_matching(zone_cards(player, zone), card_db, filter, zone == "stage")
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
        "deck" | "deck_top" => { player.main_deck.cards.insert(0, card_id); true }
        "deck_bottom" => { player.main_deck.cards.push(card_id); true }
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
        "under_member" => {
            // Rule 4.5.5: Place card under a member
            // Find first non-empty stage slot as fallback
            let target_idx = if player.stage.stage[1] != -1 { 1 }
                else if player.stage.stage[0] != -1 { 0 }
                else if player.stage.stage[2] != -1 { 2 }
                else { player.waitroom.add_card(card_id); return true; };
            let area = pos_to_area(target_idx);
            player.stage.place_under_card(area, card_id);
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

// ============== PER-UNIT CALCULATION ==============

pub fn calculate_per_unit_multiplier(
    per_unit: bool,
    per_unit_type: Option<&str>,
    player: &crate::player::Player,
) -> u32 {
    if !per_unit {
        return 1;
    }
    match per_unit_type {
        Some("member") | Some("人") | Some("members") => {
            player.stage.stage.iter().filter(|&&c| c != -1).count() as u32
        }
        Some("hand") | Some("card") | Some("枚") => player.hand.cards.len() as u32,
        Some("energy") => player.energy_zone.cards.len() as u32,
        Some("live_card_zone") => player.live_card_zone.cards.len() as u32,
        Some("discard") => player.waitroom.cards.len() as u32,
        Some("under_member") | Some("下") => {
            player.stage.under_cards.iter().map(|sv| sv.len()).sum::<usize>() as u32
        }
        _ => 1,
    }
}

/// Resolve per-unit count with optional card type / group / heart color filtering.
/// Returns the effective count multiplier.
pub fn resolve_per_unit_count(
    per_unit: bool,
    per_unit_type: Option<&str>,
    player: &crate::player::Player,
    card_db: &CardDatabase,
    filter: &CardFilter,
    heart_colors: &[String],
) -> u32 {
    if !per_unit {
        return 1;
    }
    let zone = match per_unit_type {
        Some("stage") | Some("member") | Some("人") | Some("members") => "stage",
        Some("hand") | Some("card") => "hand",
        Some("枚") => {
            let has_member_ct = filter.card_type.map_or(false, |ct| ct == "member_card");
            if has_member_ct {
                "under_member"
            } else {
                "hand"
            }
        },
        Some("discard") => "waitroom",
        Some("live_card_zone") => "live_card_zone",
        _ => return 1,
    };
    if zone == "under_member" {
        let cards: Vec<i16> = player.stage.under_cards.iter().flat_map(|sv| sv.iter()).copied().collect();
        if heart_colors.is_empty() {
            count_matching(&cards, card_db, filter, false)
        } else {
            cards.iter()
                .filter(|&&id| filter.matches(card_db, id, false) && card_matches_heart_colors(card_db, id, heart_colors))
                .count() as u32
        }
    } else {
        let cards = zone_cards(player, zone);
        if heart_colors.is_empty() {
            count_matching(cards, card_db, filter, zone == "stage")
        } else {
            cards.iter()
                .filter(|&&id| filter.matches(card_db, id, zone == "stage") && card_matches_heart_colors(card_db, id, heart_colors))
                .count() as u32
        }
    }
}

// ============== DISTINCT FILTERING ==============

pub fn apply_distinct_filter(cards: &[i16], distinct: Option<&str>, card_db: &CardDatabase) -> Vec<i16> {
    let should = matches!(distinct, Some("card_name") | Some("true") | Some("distinct"));
    if !should {
        return cards.to_vec();
    }
    let mut seen = std::collections::HashSet::new();
    cards.iter()
        .filter(|&&id| card_db.get_card(id).map(|c| seen.insert(c.name.clone())).unwrap_or(true))
        .copied()
        .collect()
}

// ============== ZONE CARD COUNT ==============

pub fn get_zone_card_count(player: &crate::player::Player, zone: &str) -> usize {
    match zone {
        "stage" => player.stage.stage.iter().filter(|&&c| c != -1).count(),
        "hand" => player.hand.cards.len(),
        "deck" | "deck_top" => player.main_deck.cards.len(),
        "discard" | "waitroom" => player.waitroom.cards.len(),
        "energy_zone" => player.energy_zone.cards.len(),
        "energy_deck" => player.energy_deck.cards.len(),
        "live_card_zone" => player.live_card_zone.cards.len(),
        "success_live_zone" => player.success_live_card_zone.cards.len(),
        _ => 0,
    }
}

// ============== DURATION HELPERS ==============

pub fn parse_duration(s: &str) -> Duration {
    match s {
        "this_turn" => Duration::ThisTurn,
        "live_end" => Duration::LiveEnd,
        "as_long_as" => Duration::AsLongAs,
        "permanent" => Duration::Permanent,
        "this_live" => Duration::ThisLive,
        _ => Duration::ThisLive,
    }
}

pub fn push_temporary_effect(
    game_state: &mut crate::game_state::GameState,
    effect_type: &str,
    duration: Option<&str>,
    target_player_id: &str,
    description: &str,
    effect_data: Option<serde_json::Value>,
) {
    if let Some(d) = duration {
        if d != "permanent" {
            game_state.temporary_effects.push(crate::game_state::TemporaryEffect {
                effect_type: effect_type.to_string(),
                duration: parse_duration(d),
                created_turn: game_state.turn_number,
                created_phase: game_state.current_phase.clone(),
                target_player_id: target_player_id.to_string(),
                description: description.to_string(),
                creation_order: 0,
                effect_data,
            });
        }
    }
}

pub fn extract_heart_colors_from_text(text: &str) -> Vec<String> {
    let mut colors: Vec<String> = Vec::new();
    let mut pos = 0;
    while let Some(start) = text[pos..].find("heart_") {
        let nums_start = pos + start + 6;
        let end = nums_start + text[nums_start..].chars().take_while(|c| c.is_ascii_digit()).count();
        if end > nums_start {
            if let Ok(n) = text[nums_start..end].parse::<u32>() {
                let color = format!("heart{:02}", n);
                if !colors.contains(&color) {
                    colors.push(color);
                }
            }
        }
        pos = end.max(nums_start);
    }
    colors
}

