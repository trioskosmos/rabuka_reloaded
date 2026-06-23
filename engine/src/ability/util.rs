use super::enums::{ActionType, Zone};
use crate::card::parse_heart_color;
use crate::card::CardDatabase;
use crate::game_state::Duration;

// ============== MODIFY COST ==============

pub fn find_modify_cost<'a>(
    effect: &'a crate::card::AbilityEffect,
    op: Option<&str>,
    loc: Option<&str>,
) -> Option<&'a crate::card::AbilityEffect> {
    if effect.action == "modify_cost"
        && op.is_none_or(|o| effect.operation.as_deref() == Some(o))
        && loc.is_none_or(|l| effect.location.as_deref() == Some(l))
    {
        return Some(effect);
    }
    if ActionType::from_str(&effect.action) == Some(ActionType::Sequential) {
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

fn play_cost_reduction_matches(
    effect: &crate::card::AbilityEffect,
    card_id: i16,
    card: &crate::card::Card,
    card_db: &CardDatabase,
) -> bool {
    let group_matches = effect
        .group_names
        .as_ref()
        .and_then(|gn| {
            gn.first()
                .map(|g| card_matches_group_str(card_db, card_id, Some(g)))
        })
        .unwrap_or(true);
    if !group_matches {
        return false;
    }
    if let Some(limit) = effect.cost_limit {
        if card.cost != Some(limit) {
            return false;
        }
    }
    if !cost_threshold_met(card, effect) {
        return false;
    }
    if let Some(ref ct) = effect.card_type {
        if ct != "member_card" && ct != "card" && ct != "member" {
            return false;
        }
    }
    true
}

fn per_unit_cost_reduction(
    effect: &crate::card::AbilityEffect,
    stage: &crate::core::zones::Stage,
    hand_count: usize,
    card_db: &CardDatabase,
) -> u32 {
    let count_zone = effect
        .per_unit_location
        .as_deref()
        .or(effect.location.as_deref())
        .unwrap_or("hand");

    let raw_count = if count_zone == "stage" && effect.group_names.is_some() {
        let group_name = effect.group_name();
        stage
            .stage
            .iter()
            .filter(|&&id| id != -1)
            .filter(|&&id| card_matches_group_str(card_db, id, group_name))
            .count()
    } else {
        hand_count
    };

    let per_unit_count = effect.per_unit_count.unwrap_or(1).max(1) as usize;
    let exclude_self = effect.exclude_self.unwrap_or(false);
    let effective = if exclude_self {
        raw_count.saturating_sub(1)
    } else {
        raw_count
    };
    let value = effect.value.unwrap_or(1) as u32;
    ((effective / per_unit_count) as u32) * value
}

pub fn calculate_play_cost_reduction(
    stage: &crate::core::zones::Stage,
    success_live_cards: &[i16],
    hand_count: usize,
    card_id: i16,
    card_db: &CardDatabase,
) -> u32 {
    let card = match card_db.get_card(card_id) {
        Some(c) => c,
        None => return 0,
    };

    let mut cost_reduction: u32 = 0;
    for ability in &card.abilities {
        if let Some(ref effect) = ability.effect {
            if let Some(mod_cost) = find_modify_cost(effect, Some("subtract"), Some("hand")) {
                if !play_cost_reduction_matches(mod_cost, card_id, card, card_db) {
                    continue;
                }
                if mod_cost.per_unit.unwrap_or(false) {
                    cost_reduction = per_unit_cost_reduction(mod_cost, stage, hand_count, card_db);
                } else {
                    let reduction = mod_cost.value.unwrap_or(1);
                    cost_reduction = cost_reduction.max(reduction);
                }
                break;
            }
        }
    }

    if cost_reduction == 0 {
        for &stage_id in &stage.stage {
            if stage_id == -1 {
                continue;
            }
            if let Some(stage_card) = card_db.get_card(stage_id) {
                for ability in &stage_card.abilities {
                    if let Some(ref effect) = ability.effect {
                        if ActionType::from_str(&effect.action) == Some(ActionType::ModifyCost)
                            && effect.operation.as_deref() == Some("subtract")
                            && effect.location.as_deref().and_then(Zone::from_str)
                                == Some(Zone::Hand)
                        {
                            // Skip effects with a location condition requiring hand
                            // (e.g. "手札にあるこのカード") — the card is on stage, so
                            // the condition is not met.
                            if let Some(ref cond) = effect.condition {
                                if cond.location.as_deref() == Some("hand") {
                                    continue;
                                }
                            }
                            let group_matches = effect
                                .group_names
                                .as_ref()
                                .and_then(|gn| {
                                    gn.first()
                                        .map(|g| card_matches_group_str(card_db, card_id, Some(g)))
                                })
                                .unwrap_or(true);
                            if !group_matches {
                                continue;
                            }
                            if let Some(limit) = effect.cost_limit {
                                if card.cost != Some(limit) {
                                    continue;
                                }
                            }
                            if !cost_threshold_met(card, effect) {
                                continue;
                            }
                            if let Some(ref ct) = effect.card_type {
                                if ct != "member_card" && ct != "card" && ct != "member" {
                                    continue;
                                }
                            }
                            let reduction = if effect.per_unit.unwrap_or(false) {
                                per_unit_cost_reduction(effect, stage, hand_count, card_db)
                            } else {
                                effect.value.unwrap_or(1)
                            };
                            cost_reduction = cost_reduction.max(reduction);
                            break;
                        }
                    }
                }
            }
            if cost_reduction > 0 {
                break;
            }
        }
    }

    if cost_reduction == 0 {
        for &live_id in success_live_cards {
            if let Some(live_card) = card_db.get_card(live_id) {
                for ability in &live_card.abilities {
                    if let Some(ref effect) = ability.effect {
                        if ActionType::from_str(&effect.action) == Some(ActionType::ModifyCost)
                            && effect.operation.as_deref() == Some("subtract")
                            && effect.location.as_deref().and_then(Zone::from_str)
                                == Some(Zone::Hand)
                        {
                            if let Some(ref cond) = effect.condition {
                                if cond.location.as_deref() == Some("hand") {
                                    continue;
                                }
                            }
                            let group_matches = effect
                                .group_names
                                .as_ref()
                                .and_then(|gn| {
                                    gn.first()
                                        .map(|g| card_matches_group_str(card_db, card_id, Some(g)))
                                })
                                .unwrap_or(true);
                            if !group_matches {
                                continue;
                            }
                            if !cost_threshold_met(card, effect) {
                                continue;
                            }
                            if let Some(ref ct) = effect.card_type {
                                if ct != "member_card" && ct != "card" && ct != "member" {
                                    continue;
                                }
                            }
                            let reduction = if effect.per_unit.unwrap_or(false) {
                                per_unit_cost_reduction(effect, stage, hand_count, card_db)
                            } else {
                                effect.value.unwrap_or(1)
                            };
                            cost_reduction = cost_reduction.max(reduction);
                            break;
                        }
                    }
                }
            }
            if cost_reduction > 0 {
                break;
            }
        }
    }

    cost_reduction
}

fn cost_threshold_met(card: &crate::card::Card, effect: &crate::card::AbilityEffect) -> bool {
    match (effect.original_count, effect.original_operator.as_deref()) {
        (Some(threshold), Some(op)) => {
            let cost = card.cost.unwrap_or(0);
            let met = match op {
                ">=" => cost >= threshold,
                "<=" => cost <= threshold,
                ">" => cost > threshold,
                "<" => cost < threshold,
                "==" => cost == threshold,
                "!=" => cost != threshold,
                _ => true,
            };
            if !met {
                return false;
            }
        }
        (Some(threshold), None) if card.cost != Some(threshold) => {
            return false;
        }
        _ => {}
    }
    true
}

pub fn target_player_label(target: &str, master: Option<&str>) -> &'static str {
    match (target, master) {
        ("self", Some("player2") | Some("p2")) => "P2",
        ("self", _) => "P1",
        ("opponent", Some("player2") | Some("p2")) => "P1",
        ("opponent", _) => "P2",
        (_, _) => "P1",
    }
}

// ============== INDIVIDUAL CARD PREDICATES ==============

pub fn card_matches_type(
    card_db: &CardDatabase,
    card_id: i16,
    card_type_filter: Option<&str>,
) -> bool {
    match card_type_filter {
        Some("live_card") => card_db
            .get_card(card_id)
            .map(|c| c.is_live())
            .unwrap_or(false),
        Some("member_card") => card_db
            .get_card(card_id)
            .map(|c| c.is_member())
            .unwrap_or(false),
        Some("energy_card") => card_db
            .get_card(card_id)
            .map(|c| c.is_energy())
            .unwrap_or(false),
        None => true,
        _ => true,
    }
}

pub fn card_matches_group(
    card_db: &CardDatabase,
    card_id: i16,
    group_filter: Option<&String>,
) -> bool {
    match group_filter {
        Some(group_name) => card_db
            .get_card(card_id)
            .map(|c| c.group == *group_name)
            .unwrap_or(false),
        None => true,
    }
}

/// Like `card_matches_group_str` but returns a vec of (reason, result) pairs
/// for each check so callers can log detailed diagnostics. Disabled by default;
/// enable via `RABUKA_DEBUG_GROUP=1`.
fn debug_group_match(card_db: &CardDatabase, card_id: i16, group_name: Option<&str>, result: bool) {
    static DEBUG_GROUP: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if !*DEBUG_GROUP.get_or_init(|| std::env::var("RABUKA_DEBUG_GROUP").as_deref() == Ok("1")) {
        return;
    }
    let card = card_db.get_card(card_id);
    let card_name = card.as_ref().map(|c| c.name.as_str()).unwrap_or("?");
    let series = card.as_ref().map(|c| c.series.as_str()).unwrap_or("?");
    let unit = card.as_ref().and_then(|c| c.unit.as_deref()).unwrap_or("?");
    let _group = group_name.unwrap_or("None");
    let checks = match group_name {
        Some(g) => {
            fn norm(s: &str) -> String {
                s.replace('\u{FF01}', "!")
            }
            let gn = norm(g);
            card.as_ref()
                .map(|c| {
                    let unit_ok = norm(c.unit.as_deref().unwrap_or("")) == gn;
                    let group_ok = c.group == g;
                    let name_ok = card_db
                        .get_card_names(card_id)
                        .iter()
                        .any(|n| norm(n).contains(&gn));
                    let series_ok =
                        !c.series.contains('\n') && card_series_matches_group(&c.series, g);
                    format!(
                        "unit={} ser={} grp={:?} | unit_ok={} grp_ok={} name_ok={} series_ok={}",
                        unit, series, c.group, unit_ok, group_ok, name_ok, series_ok
                    )
                })
                .unwrap_or_default()
        }
        None => String::new(),
    };
    log::debug!(
        "[GROUP_MATCH] card={}[{}] group={:?} result={} {}",
        card_name,
        card_id,
        group_name,
        result,
        checks
    );
}

pub fn card_matches_group_str(
    card_db: &CardDatabase,
    card_id: i16,
    group_name: Option<&str>,
) -> bool {
    let result = match group_name {
        Some(g) => {
            // Normalize full-width/half-width exclamation marks so that
            // group names like "みらくらぱーく！" match unit fields using
            // either ！(U+FF01) or !(U+0021).
            // Also normalize µ (micro sign U+00B5) to μ (mu U+03BC) for
            // μ's group matching.
            fn norm(s: &str) -> String {
                s.replace('\u{FF01}', "!").replace('\u{00B5}', "\u{03BC}")
            }
            let gn = norm(g);
            card_db
                .get_card(card_id)
                .map(|c| {
                    let unit = c.unit.as_deref().unwrap_or("");
                    let unit_match = unit == gn || ((unit.contains('\u{FF01}') || unit.contains('\u{00B5}')) && norm(unit) == gn);
                    unit_match
                || c.group == g
                // Check name fragments for multi-name cards (e.g. "にこ" in "矢澤にこ")
                || card_db.get_card_names(card_id).iter().any(|n| n.contains(&gn) || ((n.contains('\u{FF01}') || n.contains('\u{00B5}')) && norm(n).contains(&gn)))
                // Multi-name cards (e.g. 渡辺曜&鬼塚夏美&大沢瑠璃乃) should match
                // group names through any of their constituent series (Q105).
                // Example: LL-bp2-001-R+ matches "Aqours" via ラブライブ！サンシャイン!!
                || card_series_matches_group(&c.series, &gn)
                // Constant `set_card_identity` ("treated as") abilities give the
                // card additional group memberships in all zones. Examples:
                //   AURORA FLOWER (PL!HS-bp5-018-L) is "スリーズブーケ" /
                //   "DOLLCHESTRA" / "みらくらぱーく！" everywhere.
                || c.abilities.iter().any(|ab| {
                    ab.effect.as_ref().is_some_and(|eff| {
                        eff.action == "set_card_identity"
                            && eff.identities.as_ref().is_some_and(|ids| {
                                ids.iter().any(|id| id == &gn || ((id.contains('\u{FF01}') || id.contains('\u{00B5}')) && norm(id) == gn))
                            })
                    })
                })
                })
                .unwrap_or(false)
        }
        None => true,
    };
    debug_group_match(card_db, card_id, group_name, result);
    result
}

fn card_series_matches_group(series: &str, group: &str) -> bool {
    if group == "μ's" {
        // For μ's, check each series line individually to handle multi-series
        // joint cards (e.g. LL-bp3-001-R+ 園田海未&津島善子&天王寺璃奈 whose
        // series includes a bare "ラブライブ！" line among other group lines).
        return series.split('\n').any(|line| {
            line.contains("ラブライブ！")
                && !line.contains("サンシャイン")
                && !line.contains("虹ヶ咲")
                && !line.contains("スーパースター")
                && !line.contains("蓮ノ空")
        });
    }
    match group {
        "Aqours" => series.contains("サンシャイン"),
        "虹ヶ咲" => series.contains("虹ヶ咲"),
        "Liella!" => series.contains("スーパースター"),
        "蓮ノ空" => series.contains("蓮ノ空"),
        _ => false,
    }
}

pub fn card_matches_characters(
    card_db: &CardDatabase,
    card_id: i16,
    characters: Option<&Vec<String>>,
) -> bool {
    match characters {
        Some(names) if !names.is_empty() => {
            let card_names = card_db.get_card_names(card_id);
            names.iter().any(|name| {
                let clean_name = CardDatabase::normalize_name(name);
                card_names.iter().any(|cn| cn.contains(&clean_name))
            })
        }
        _ => true,
    }
}

pub fn card_matches_cost_limit(
    card_db: &CardDatabase,
    card_id: i16,
    cost_limit: Option<u32>,
) -> bool {
    card_matches_cost_limit_op(card_db, card_id, cost_limit, None)
}

pub fn card_matches_cost_limit_op(
    card_db: &CardDatabase,
    card_id: i16,
    cost_limit: Option<u32>,
    comparison: Option<&str>,
) -> bool {
    match cost_limit {
        Some(limit) => card_db
            .get_card(card_id)
            .map(|c| {
                // Use score for live cards, cost for members
                c.cost.or(c.score)
            })
            .flatten()
            .map(|value| match comparison {
                Some("min") | Some(">=") => value >= limit,
                Some("exact") | Some("=") => value == limit,
                Some(">") => value > limit,
                Some("<") => value < limit,
                _ => value <= limit,
            })
            .unwrap_or(false),
        None => true,
    }
}

pub fn card_matches_heart_colors(
    card_db: &CardDatabase,
    card_id: i16,
    heart_colors: &[String],
) -> bool {
    if heart_colors.is_empty() {
        return true;
    }
    let result = card_db.get_card(card_id).is_none_or(|card| {
        heart_colors.iter().any(|color| {
            let hc = parse_heart_color(color);
            card.base_heart.as_ref().map_or(
                card.need_heart
                    .as_ref()
                    .is_some_and(|need| need.hearts.contains_key(&hc)),
                |base| base.hearts.contains_key(&hc),
            )
        })
    });
    result
}

pub fn card_matches_name_constraint(
    card_db: &CardDatabase,
    card_id: i16,
    name_constraint: Option<&str>,
) -> bool {
    match name_constraint {
        Some(name) => card_db
            .get_card(card_id)
            .map(|c| CardDatabase::normalize_name(&c.name) == CardDatabase::normalize_name(name))
            .unwrap_or(false),
        None => true,
    }
}

// ============== UNIFIED FILTER STRUCT ==============

/// Unified card filter: all fields are Optional — None = match anything.
#[derive(Default, Clone)]
pub struct CardFilter<'a> {
    pub card_type: Option<&'a str>,
    pub group: Option<&'a str>,
    pub groups: Option<&'a Vec<String>>,
    pub cost_limit: Option<u32>,
    pub cost_operator: Option<&'a str>,
    /// Minimum cost bound for range filters (e.g. cost >= 4)
    pub cost_limit_min: Option<u32>,
    /// Sum-total cost constraint — checked post-selection, not in per-card matches()
    pub cost_total: Option<u32>,
    pub cost_total_operator: Option<&'a str>,
    pub characters: Option<&'a Vec<String>>,
    pub exclude_characters: Option<&'a Vec<String>>,
    pub heart_colors: &'a [String],
    pub need_heart_total: Option<u32>,
    pub need_heart_operator: Option<&'a str>,
    pub need_heart_color: Option<&'a str>,
    pub name_fragments: Option<&'a Vec<String>>,
    pub distinct: Option<&'a str>,
    pub exclude_self: Option<i16>,
    /// Group names to exclude from matching (e.g. 「スリーズブーケ」以外)
    pub exclude_group_names: Option<&'a Vec<String>>,
    pub original_blade_limit: Option<u32>,
    pub original_blade_operator: Option<&'a str>,
    /// Card IDs to exclude from matching (e.g. previously selected by a prior sequential action)
    pub exclude_cards: Option<&'a [i16]>,
    /// Ability filter: "no_ability" / "has_ability" / "no_ability_type"
    pub ability_filter: Option<&'a str>,
    /// Trigger types to check when ability_filter is "no_ability_type"
    pub ability_filter_triggers: Option<&'a [String]>,
    /// OR'd ability filter branches — card passes if ANY branch matches.
    pub or_ability_filters: Option<&'a [crate::card::AbilityFilterBranch]>,
    /// Card property filter (e.g. "has_blade_heart")
    pub card_property: Option<&'a str>,
    /// Negate the card_property check (e.g. "does NOT have blade heart")
    pub negation: bool,
}

impl<'a> CardFilter<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn card_type(mut self, ct: &'a str) -> Self {
        self.card_type = Some(ct);
        self
    }
    pub fn card_type_opt(mut self, ct: Option<&'a str>) -> Self {
        self.card_type = ct;
        self
    }
    pub fn group(mut self, g: &'a str) -> Self {
        self.group = Some(g);
        self
    }
    pub fn group_opt(mut self, g: Option<&'a str>) -> Self {
        self.group = g;
        self
    }
    pub fn heart_colors(mut self, hc: &'a [String]) -> Self {
        self.heart_colors = hc;
        self
    }
    pub fn distinct(mut self, d: &'a str) -> Self {
        self.distinct = Some(d);
        self
    }
    pub fn exclude_cards_opt(mut self, ids: Option<&'a [i16]>) -> Self {
        self.exclude_cards = ids;
        self
    }
    pub fn original_blade_limit(mut self, obl: Option<u32>, obo: Option<&'a str>) -> Self {
        self.original_blade_limit = obl;
        self.original_blade_operator = obo;
        self
    }
    pub fn exclude_self(mut self, id: i16) -> Self {
        self.exclude_self = Some(id);
        self
    }
    pub fn exclude_self_opt(mut self, id: Option<i16>) -> Self {
        self.exclude_self = id;
        self
    }

    /// Returns true if any filter field is set that could cause cards to be rejected.
    pub fn has_filter(&self) -> bool {
        self.card_type.is_some()
            || self.group.is_some()
            || self.groups.is_some()
            || self.cost_limit.is_some()
            || self.cost_limit_min.is_some()
            || self.characters.is_some()
            || self.exclude_characters.is_some()
            || !self.heart_colors.is_empty()
            || self.need_heart_total.is_some()
            || self.need_heart_color.is_some()
            || self.name_fragments.is_some()
            || self.original_blade_limit.is_some()
            || self.ability_filter.is_some()
            || self.ability_filter_triggers.is_some()
            || self.or_ability_filters.is_some()
            || self.card_property.is_some()
            || self.distinct.is_some()
    }

    /// Check whether a single card matches ALL present filter fields.
    pub fn matches(&self, db: &CardDatabase, id: i16, skip_empty: bool) -> bool {
        if skip_empty && id == -1 {
            return false;
        }
        if let Some(exclude_id) = self.exclude_self {
            if id == exclude_id {
                log::debug!("[DBG matches] exclude_self id={} matched, excluding", id);
                return false;
            }
        }
        if let Some(ex) = self.exclude_cards {
            if ex.contains(&id) {
                return false;
            }
        }
        if let Some(ct) = self.card_type {
            if !card_matches_type(db, id, Some(ct)) {
                return false;
            }
        }
        if let Some(g) = self.group {
            if !card_matches_group_str(db, id, Some(g)) {
                if let Some(gs) = self.groups {
                    if !gs.iter().any(|gn| card_matches_group_str(db, id, Some(gn))) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        } else if let Some(gs) = self.groups {
            if !gs.iter().any(|gn| card_matches_group_str(db, id, Some(gn))) {
                return false;
            }
        }
        // exclude_group_names: card passes if its group is NOT in the excluded list
        if let Some(ex_gns) = self.exclude_group_names {
            for g in ex_gns {
                if card_matches_group_str(db, id, Some(g.as_str())) {
                    return false;
                }
            }
        }
        if let Some(lim) = self.cost_limit {
            if !card_matches_cost_limit_op(db, id, Some(lim), self.cost_operator) {
                return false;
            }
        }
        if let Some(min) = self.cost_limit_min {
            if !card_matches_cost_limit_op(db, id, Some(min), Some(">=")) {
                return false;
            }
        }
        if let Some(ch) = self.characters {
            if !card_matches_characters(db, id, Some(ch)) {
                return false;
            }
        }
        if let Some(ex_ch) = self.exclude_characters {
            if card_matches_characters(db, id, Some(ex_ch)) {
                return false;
            }
        }
        if !self.heart_colors.is_empty() && !card_matches_heart_colors(db, id, self.heart_colors) {
            return false;
        }
        // Heart threshold check.
        // Per Q149 (qa_data.json:1957-1958): "ハートの総数" = 基本ハート
        // (basic hearts counted regardless of color). Per Q172 (lines 1405-1406):
        // ability-granted hearts ARE included but blade hearts from yell are NOT.
        // total_hearts() returns base_heart (printed) for member cards, which
        // matches "基本ハート". Note: this does NOT include ability-granted
        // heart modifiers (heart_modifiers in GameModifiers) — those require
        // game-state access which CardFilter::matches() doesn't have.
        // Rules 9.9.1.4→9.9.1.5 (rules.txt:1196-1212) defines the application
        // order: printed base → set-to-value → add/subtract.
        if let Some(need_total) = self.need_heart_total {
            if let Some(color_str) = self.need_heart_color {
                // Per-color check (e.g. heart06 >= 3 for specific-color
                // live-card require conditions, not member base hearts).
                let color = crate::zones::parse_heart_color(color_str);
                let card_amount = db
                    .get_card(id)
                    .and_then(|c| c.need_heart.as_ref())
                    .map(|nh| *nh.hearts.get(&color).unwrap_or(&0))
                    .unwrap_or(0);
                let op = self.need_heart_operator.unwrap_or(">=");
                if !compare_counts(Some(op), card_amount, need_total) {
                    return false;
                }
            } else {
                // Total sum check — use total_hearts() which returns base_heart
                // for member cards (the card's printed hearts) and falls back to
                // need_heart for live cards. need_heart_total() only checks the
                // live card cost field which is always 0 for members, so we use
                // total_hearts() instead. Per Q149 + Q172.
                let card_total = db.get_card(id).map(|c| c.total_hearts()).unwrap_or(0);
                let op = self.need_heart_operator.unwrap_or(">=");
                if !compare_counts(Some(op), card_total, need_total) {
                    return false;
                }
            }
        }
        if let Some(name) = self.name_fragments {
            if !card_matches_name_fragments(db, id, name) {
                return false;
            }
        }
        if let Some(ex_id) = self.exclude_self {
            if id == ex_id {
                return false;
            }
        }
        // "元々持つブレード" — checks the card's base/printed blade value
        // (card.blade from DB, no modifiers applied). Per Q195 (qa_data.json:1071-1074):
        // "元々持つブレードの数を変更した後、ブレードを得る効果が適用される" —
        // setting the original blade changes the base, then +blade effects stack
        // on top. Rules 9.9.1.4→9.9.1.5 (rules.txt:1196-1212) defines this order:
        // printed base → set-to-value → add/subtract.
        // For current/modified blade checks (e.g. "ブレードの合計"), use
        // evaluate_card_blade_condition() which sums base + blade_modifiers.
        // Per Q116 (lines 2487-2488): current total blade ≥ 10 condition uses
        // modified values.
        if let Some(bl) = self.original_blade_limit {
            let card_blade = db.get_card(id).map(|c| c.blade).unwrap_or(0);
            if !compare_counts(self.original_blade_operator, card_blade, bl) {
                return false;
            }
        }
        // Per-card cost_total check — each individual card's cost must
        // satisfy the total-budget comparison (e.g. card.cost <= 4).
        if let Some(ct) = self.cost_total {
            if let Some(op) = self.cost_total_operator {
                let card_cost = db.get_card(id).and_then(|c| c.cost).unwrap_or(99);
                if !compare_counts(Some(op), card_cost, ct) {
                    return false;
                }
            }
        }
        // ability_filter: filter by presence/absence of abilities or trigger types
        if let Some(af) = self.ability_filter {
            if let Some(card) = db.get_card(id) {
                let has_ability = !card.abilities.is_empty();
                match af {
                    "no_ability" => {
                        if has_ability {
                            return false;
                        }
                    }
                    "has_ability" => {
                        if !has_ability {
                            return false;
                        }
                    }
                    "no_ability_type" => {
                        if let Some(excluded) = self.ability_filter_triggers {
                            if !excluded.is_empty() {
                                // Card passes only if it has NO ability matching any excluded trigger
                                if card.abilities.iter().any(|a| {
                                    a.triggers.as_ref().is_some_and(|t| {
                                        excluded.iter().any(|et| t.starts_with(et.as_str()))
                                    })
                                }) {
                                    return false;
                                }
                            }
                        }
                    }
                    "has_ability_type" => {
                        if let Some(included) = self.ability_filter_triggers {
                            if !included.is_empty() {
                                // Card passes if it has ANY ability matching included triggers
                                if !card.abilities.iter().any(|a| {
                                    a.triggers.as_ref().is_some_and(|t| {
                                        included.iter().any(|it| t.starts_with(it.as_str()))
                                    })
                                }) {
                                    return false;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        // or_ability_filters: card passes if ANY branch matches.
        // When present, the single ability_filter above (if any) is ignored
        // — the OR branches define the complete filter.
        if let Some(branches) = self.or_ability_filters {
            if !branches.is_empty() {
                if let Some(card) = db.get_card(id) {
                    let passes_or = branches.iter().any(|branch| {
                        let af = branch.ability_filter.as_deref().unwrap_or("");
                        let has_ability = !card.abilities.is_empty();
                        match af {
                            "no_ability" => !has_ability,
                            "has_ability" => has_ability,
                            "no_ability_type" => {
                                if let Some(excluded) = &branch.ability_filter_triggers {
                                    if !excluded.is_empty() {
                                        // Card passes if it has NO ability matching excluded triggers
                                        !card.abilities.iter().any(|a| {
                                            a.triggers.as_ref().is_some_and(|t| {
                                                excluded.iter().any(|et| t.starts_with(et))
                                            })
                                        })
                                    } else {
                                        has_ability
                                    }
                                } else {
                                    has_ability
                                }
                            }
                            "has_ability_type" => {
                                if let Some(included) = &branch.ability_filter_triggers {
                                    if !included.is_empty() {
                                        // Card passes if it has ANY ability matching included triggers
                                        card.abilities.iter().any(|a| {
                                            a.triggers.as_ref().is_some_and(|t| {
                                                included.iter().any(|it| t.starts_with(it))
                                            })
                                        })
                                    } else {
                                        has_ability
                                    }
                                } else {
                                    has_ability
                                }
                            }
                            _ => true,
                        }
                    });
                    if !passes_or {
                        return false;
                    }
                }
            }
        }
        // card_property filter (e.g. "has_blade_heart")
        if let Some(prop) = self.card_property {
            let has_property = match prop {
                "has_blade_heart" => db.get_card(id).is_some_and(|c| c.has_blade_heart()),
                "has_score_icon" => db.get_card(id).is_some_and(|c| c.has_score_icon()),
                _ => false,
            };
            let passes = if self.negation {
                !has_property
            } else {
                has_property
            };
            if !passes {
                return false;
            }
        }
        true
    }

    pub fn matches_card(&self, db: &CardDatabase, id: i16) -> bool {
        self.matches(db, id, false)
    }

    pub fn find_ids(&self, cards: &[i16], db: &CardDatabase) -> Vec<i16> {
        cards
            .iter()
            .filter(|&&id| self.matches(db, id, false))
            .copied()
            .collect()
    }

    pub fn count(&self, cards: &[i16], db: &CardDatabase) -> u32 {
        cards
            .iter()
            .filter(|&&id| self.matches(db, id, false))
            .count() as u32
    }

    /// Build a full CardFilter from all AbilityEffect fields.
    ///
    /// This is the complete filter including heart thresholds (Q149/Q172:
    /// need_heart_total uses total_hearts() for member base_heart checks),
    /// blade limits (Q195: original_blade_limit checks card.blade from DB),
    /// cost totals (Q129: modified/current cost is used for cost conditions),
    /// ability filters, card properties, distinct, etc.
    /// Use filter_subset() only for minimal zone lookups.
    pub fn from_effect(effect: &'a crate::card::AbilityEffect) -> Self {
        CardFilter {
            card_type: effect.card_type.as_deref(),
            group: effect
                .group_names
                .as_ref()
                .and_then(|v| v.first())
                .map(|s| s.as_str()),
            groups: effect.group_names.as_ref().map(|v| v),
            cost_limit: effect.cost_limit,
            cost_operator: effect.cost_limit_operator.as_deref(),
            cost_limit_min: effect.cost_limit_min,
            cost_total: effect.cost_total,
            cost_total_operator: effect.cost_total_operator.as_deref(),
            characters: effect.characters.as_ref(),
            exclude_characters: effect.exclude_characters.as_ref(),
            exclude_group_names: effect.exclude_group_names.as_ref(),
            heart_colors: &effect.heart_colors,
            need_heart_total: effect.need_heart_total,
            need_heart_operator: effect.need_heart_operator.as_deref(),
            need_heart_color: effect.need_heart_color.as_deref(),
            name_fragments: None,
            distinct: effect.distinct.as_deref(),
            exclude_self: if effect.exclude_self.unwrap_or(false) {
                Some(-1)
            } else {
                None
            },
            original_blade_limit: effect.blade_limit,
            original_blade_operator: effect.blade_limit_operator.as_deref(),
            exclude_cards: None,
            ability_filter: effect.ability_filter.as_deref(),
            ability_filter_triggers: effect.ability_filter_triggers.as_ref().map(|v| &**v),
            or_ability_filters: effect.or_ability_filters.as_ref().map(|v| &**v),
            card_property: effect.card_property.as_deref(),
            negation: false,
        }
    }

    /// Build from a Choice::SelectCard — reads filter fields the choice advertised.
    pub fn from_choice(choice: &'a crate::ability::types::Choice) -> Self {
        match choice {
            crate::ability::types::Choice::SelectCard {
                card_type,
                cost_limit,
                cost_limit_operator,
                cost_total: _,
                cost_total_operator: _,
                group,
                characters,
                ..
            } => CardFilter {
                card_type: card_type.as_deref(),
                group: group.as_deref(),
                groups: None,
                cost_limit: *cost_limit,
                cost_operator: cost_limit_operator.as_deref(),
                cost_limit_min: None,
                cost_total: None,
                cost_total_operator: None,
                need_heart_total: None,
                need_heart_operator: None,
                need_heart_color: None,
                characters: characters.as_ref(),
                exclude_characters: None,
                exclude_group_names: None,
                heart_colors: &[],
                name_fragments: None,
                distinct: None,
                exclude_self: None,
                original_blade_limit: None,
                original_blade_operator: None,
                exclude_cards: None,
                ability_filter: None,
                ability_filter_triggers: None,
                or_ability_filters: None,
                card_property: None,
                negation: false,
            },
            _ => CardFilter::default(),
        }
    }
}

fn card_matches_name_fragments(db: &CardDatabase, id: i16, fragments: &[String]) -> bool {
    db.get_card(id).is_some_and(|card| {
        let norm_name = CardDatabase::normalize_name(&card.name);
        fragments
            .iter()
            .all(|f| norm_name.contains(&CardDatabase::normalize_name(f)))
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
    exclude_characters: Option<&'a Vec<String>>,
    exclude_self: Option<i16>,
) -> CardFilter<'a> {
    CardFilter {
        card_type,
        group,
        cost_limit,
        cost_operator,
        characters,
        exclude_characters,
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
    cost_total: Option<u32>,
    cost_total_operator: Option<&'a str>,
) -> CardFilter<'a> {
    CardFilter {
        card_type,
        group,
        cost_limit,
        cost_operator,
        cost_total,
        cost_total_operator,
        characters,
        name_fragments,
        distinct,
        exclude_self,
        ..CardFilter::default()
    }
}

// ============== QUERY FUNCTIONS ==============

/// Return indices into `cards` where cards match the filter.
pub fn matching_indices(
    cards: &[i16],
    db: &CardDatabase,
    filter: &CardFilter,
    skip_empty: bool,
) -> Vec<usize> {
    cards
        .iter()
        .enumerate()
        .filter(|(_, &id)| filter.matches(db, id, skip_empty))
        .map(|(i, _)| i)
        .collect()
}

/// Return card IDs from `cards` that match the filter.
pub fn matching_ids(
    cards: &[i16],
    db: &CardDatabase,
    filter: &CardFilter,
    skip_empty: bool,
) -> Vec<i16> {
    cards
        .iter()
        .filter(|&&id| filter.matches(db, id, skip_empty))
        .copied()
        .collect()
}

pub fn matching_ids_filtered(
    cards: &[i16],
    db: &CardDatabase,
    filter: &CardFilter,
    skip_empty: bool,
    target_count: Option<u32>,
    distinct: Option<&str>,
    exclude_ids: Option<&[i16]>,
) -> Vec<i16> {
    let mut filter = filter.clone();
    if let Some(ids) = exclude_ids {
        filter.exclude_cards = Some(ids);
    }
    let mut results = matching_ids(cards, db, &filter, skip_empty);
    if let Some(d) = distinct {
        results = apply_distinct_filter(&results, Some(d), db);
        // After distinct dedup, also exclude results whose names match any
        // excluded card's name (e.g. "different name from that member").
        if let Some(ids) = exclude_ids {
            if !ids.is_empty() {
                let excluded_names: std::collections::HashSet<String> = ids
                    .iter()
                    .filter_map(|id| db.get_card(*id).map(|c| c.name.clone()))
                    .collect();
                if !excluded_names.is_empty() {
                    results.retain(|id| {
                        db.get_card(*id)
                            .map_or(true, |c| !excluded_names.contains(&c.name))
                    });
                }
            }
        }
    }
    if let Some(tc) = target_count {
        results.truncate(tc as usize);
    }
    results
}

/// Count cards matching the filter.
pub fn count_matching(
    cards: &[i16],
    db: &CardDatabase,
    filter: &CardFilter,
    skip_empty: bool,
) -> u32 {
    cards
        .iter()
        .filter(|&&id| filter.matches(db, id, skip_empty))
        .count() as u32
}

/// Map a stage position string to its array index (0=left, 1=center, 2=right).
/// Accepts English, Japanese, and shorthand forms.
pub fn stage_position_index(pos: &str) -> Option<usize> {
    match pos {
        "center" | "センターエリア" => Some(1),
        "left_side" | "左サイドエリア" | "left" => Some(0),
        "right_side" | "右サイドエリア" | "right" => Some(2),
        _ => None,
    }
}

pub fn card_at_position(player: &crate::player::Player, pos: &str) -> Option<i16> {
    let idx = stage_position_index(pos)?;
    let card_id = player.stage.stage.get(idx).copied().unwrap_or(-1);
    if card_id != -1 {
        Some(card_id)
    } else {
        None
    }
}

/// Deduplicate by card name when `filter.distinct` is set.
/// Returns indices into `cards`, deduplicated by card name.
pub fn filter_distinct(
    cards: &[i16],
    db: &CardDatabase,
    filter: &CardFilter,
    skip_empty: bool,
) -> Vec<usize> {
    let ids: Vec<usize> = matching_indices(cards, db, filter, skip_empty);
    let distinct = match filter.distinct {
        Some("card_name") | Some("true") | Some("distinct") => true,
        _ => return ids,
    };
    if !distinct {
        return ids;
    }
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    ids.into_iter()
        .filter(|&i| {
            db.get_card(cards[i])
                .map(|c| seen.insert(CardDatabase::normalize_name(&c.name)))
                .unwrap_or(true)
        })
        .collect()
}

// ============== ZONE HELPERS ==============

/// Resolve a named zone to an immutable card slice.
pub fn zone_cards<'a>(player: &'a crate::player::Player, zone: &str) -> &'a [i16] {
    // Try to parse as typed Zone enum for safety
    let zone_enum = Zone::from_str(zone);

    match zone_enum {
        Some(Zone::Stage) => &player.stage.stage,
        Some(Zone::Hand) => &player.hand.cards,
        Some(Zone::Deck) | Some(Zone::DeckTop) | Some(Zone::DeckBottom) => &player.main_deck.cards,
        Some(Zone::Discard) | Some(Zone::Waitroom) => &player.waitroom.cards,
        Some(Zone::EnergyZone) | Some(Zone::Energy) => &player.energy_zone.cards,
        Some(Zone::LiveCardZone) => &player.live_card_zone.cards,
        Some(Zone::SuccessLiveZone) => &player.success_live_card_zone.cards,
        // UnderMember is a 2D structure (Vec<Vec<i16>>) and cannot be
        // returned as a flat slice. Callers must use resolve_per_unit_count
        // or direct iteration instead.
        // SuccessLiveZone cards are already handled above.
        Some(Zone::UnderMember) => &[], // Use resolve_per_unit_count instead
        // Legacy string matches for strings that don't parse to Zone enum
        None => &[],
        // All other Zone variants not explicitly listed above
        _ => &[],
    }
}

/// Return owned card IDs from a named zone (avoids borrow issues).
pub fn zone_card_ids(player: &crate::player::Player, zone: &str) -> Vec<i16> {
    zone_cards(player, zone).to_vec()
}

/// Count cards matching filter in a zone for a given player.
pub fn count_in_zone(
    player: &crate::player::Player,
    zone: &str,
    filter: &CardFilter,
    card_db: &CardDatabase,
) -> u32 {
    if Zone::from_str(zone) == Some(Zone::UnderMember) {
        let cards: Vec<i16> = player
            .stage
            .under_cards
            .iter()
            .flat_map(|sv| sv.iter())
            .copied()
            .collect();
        return count_matching(&cards, card_db, filter, false);
    }
    count_matching(
        zone_cards(player, zone),
        card_db,
        filter,
        Zone::from_str(zone) == Some(Zone::Stage),
    )
}

// ============== UTILITY ==============

pub fn compare_counts(operator: Option<&str>, actual: u32, expected: u32) -> bool {
    let op = operator.unwrap_or(">=");
    match op {
        ">=" => actual >= expected,
        ">" => actual > expected,
        "<=" => actual <= expected,
        "<" => actual < expected,
        "==" | "=" => actual == expected,
        "!=" => actual != expected,
        _ => true,
    }
}

pub fn sum_score_in_zone(
    cards: &[i16],
    card_db: &CardDatabase,
    get_modifier: impl Fn(i16) -> i32,
) -> u32 {
    cards
        .iter()
        .map(|&id| {
            let base = card_db.get_card(id).map(|c| c.get_score()).unwrap_or(0);
            (base as i32 + get_modifier(id)) as u32
        })
        .sum()
}

pub fn remove_card_from_zone(
    player: &mut crate::player::Player,
    card_id: i16,
    zone: &str,
    card_db: &CardDatabase,
) -> bool {
    match Zone::from_str(zone) {
        Some(Zone::Hand) => {
            if let Some(pos) = player.hand.cards.iter().position(|&id| id == card_id) {
                player.hand.cards.remove(pos);
                return true;
            }
        }
        Some(Zone::Stage) => {
            if let Some(pos) = player.stage.stage.iter().position(|&id| id == card_id) {
                player.remove_member_from_stage_with_recycling(pos, card_db);
                return true;
            }
        }
        Some(Zone::Energy) => {
            if let Some(pos) = player
                .energy_zone
                .cards
                .iter()
                .position(|&id| id == card_id)
            {
                player.energy_zone.cards.remove(pos);
                return true;
            }
        }
        Some(Zone::Discard) | Some(Zone::Waitroom) => {
            if let Some(pos) = player.waitroom.cards.iter().position(|&id| id == card_id) {
                player.waitroom.cards.remove(pos);
                return true;
            }
        }
        Some(Zone::Deck) => {
            if let Some(pos) = player.main_deck.cards.iter().position(|&id| id == card_id) {
                player.main_deck.cards.remove(pos);
                return true;
            }
        }
        Some(Zone::LiveCardZone) => {
            if let Some(pos) = player
                .live_card_zone
                .cards
                .iter()
                .position(|&id| id == card_id)
            {
                player.live_card_zone.cards.remove(pos);
                return true;
            }
        }
        Some(Zone::SuccessLiveZone) => {
            if let Some(pos) = player
                .success_live_card_zone
                .cards
                .iter()
                .position(|&id| id == card_id)
            {
                player.success_live_card_zone.cards.remove(pos);
                return true;
            }
        }
        _ => {}
    }
    false
}

pub fn move_card(
    player: &mut crate::player::Player,
    card_id: i16,
    src_zone: &str,
    dst_zone: &str,
    vacated_stage_area: Option<usize>,
    card_db: &CardDatabase,
) -> bool {
    // Attempt to remove from source zone
    if remove_card_from_zone(player, card_id, src_zone, card_db) {
        // Place in destination zone
        return place_card_in_zone(player, card_id, dst_zone, vacated_stage_area, false, 1);
    }
    false
}

pub fn resolve_indices_to_ids(
    player: &crate::player::Player,
    zone: &str,
    indices: &[usize],
) -> Vec<i16> {
    let cards = zone_cards(player, zone);
    indices
        .iter()
        .filter_map(|&idx| cards.get(idx).copied())
        .collect()
}

pub fn move_cards(
    player: &mut crate::player::Player,
    card_ids: &[i16],
    src_zone: &str,
    dst_zone: &str,
    vacated_stage_area: Option<usize>,
    card_db: &CardDatabase,
) -> usize {
    let mut count = 0;
    for &card_id in card_ids {
        if move_card(
            player,
            card_id,
            src_zone,
            dst_zone,
            vacated_stage_area,
            card_db,
        ) {
            count += 1;
        }
    }
    count
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
    match Zone::from_str(destination) {
        Some(Zone::Hand) => {
            player.hand.add_card(card_id);
            true
        }
        Some(Zone::Discard) | Some(Zone::Waitroom) => {
            player.waitroom.add_card(card_id);
            true
        }
        Some(Zone::Stage) | Some(Zone::EmptyArea) => {
            let empty_slots: Vec<usize> = (0..3).filter(|&i| player.stage.stage[i] == -1).collect();
            if is_max && empty_slots.len() < count {
                return false;
            }
            if let Some(pos) = stage_first_empty(&player.stage.stage) {
                player.stage.stage[pos] = card_id;
                player.areas_locked_this_turn.insert(pos_to_area(pos));
            } else {
                // Stage full — return card to discard instead of hand
                player.waitroom.add_card(card_id);
            }
            true
        }
        Some(Zone::Deck) | Some(Zone::DeckTop) => {
            let idx = vacated_stage_area
                .unwrap_or(0)
                .min(player.main_deck.cards.len());
            player.main_deck.cards.insert(idx, card_id);
            true
        }
        Some(Zone::DeckBottom) => {
            player.main_deck.cards.push(card_id);
            true
        }
        Some(Zone::Energy) => {
            player.energy_zone.cards.push(card_id);
            true
        }
        Some(Zone::EnergyDeck) => {
            player.energy_deck.cards.push(card_id);
            true
        }
        Some(Zone::LiveCardZone) => {
            player.live_card_zone.cards.push(card_id);
            true
        }
        Some(Zone::SuccessLiveZone) => {
            player.success_live_card_zone.cards.push(card_id);
            true
        }
        Some(Zone::SameArea) => {
            if let Some(pos) = vacated_stage_area {
                if pos < 3 && player.stage.stage[pos] == -1 {
                    player.stage.stage[pos] = card_id;
                    player.areas_locked_this_turn.insert(pos_to_area(pos));
                } else if let Some(ep) = stage_first_empty(&player.stage.stage) {
                    player.stage.stage[ep] = card_id;
                    player.areas_locked_this_turn.insert(pos_to_area(ep));
                } else {
                    player.hand.add_card(card_id);
                }
            } else if let Some(ep) = stage_first_empty(&player.stage.stage) {
                player.stage.stage[ep] = card_id;
                player.areas_locked_this_turn.insert(pos_to_area(ep));
            } else {
                player.hand.add_card(card_id);
            }
            true
        }
        Some(Zone::UnderMember) => {
            // Rule 4.5.5: Place card under a member
            // Fallback: prefer center, then left, then right
            let target_idx = if let Some(pos) = vacated_stage_area {
                pos
            } else if player.stage.stage[1] != -1 {
                1
            } else if player.stage.stage[0] != -1 {
                0
            } else if player.stage.stage[2] != -1 {
                2
            } else {
                player.waitroom.add_card(card_id);
                return true;
            };
            let area = pos_to_area(target_idx);
            player.stage.place_under_card(area, card_id);
            true
        }
        _ => {
            if destination.is_empty() {
                player.waitroom.add_card(card_id);
            } else {
                player.hand.add_card(card_id);
            }
            true
        }
    }
}

pub fn stage_first_empty(stage: &[i16; 3]) -> Option<usize> {
    if stage[1] == -1 {
        Some(1)
    } else if stage[0] == -1 {
        Some(0)
    } else if stage[2] == -1 {
        Some(2)
    } else {
        None
    }
}

pub fn pos_to_area(pos: usize) -> crate::zones::MemberArea {
    match pos {
        0 => crate::zones::MemberArea::LeftSide,
        1 => crate::zones::MemberArea::Center,
        _ => crate::zones::MemberArea::RightSide,
    }
}

pub fn area_to_index(area: &crate::zones::MemberArea) -> Option<usize> {
    match area {
        crate::zones::MemberArea::LeftSide => Some(0),
        crate::zones::MemberArea::Center => Some(1),
        crate::zones::MemberArea::RightSide => Some(2),
    }
}

/// For per_unit_type="discard": count recently-moved cards matching a filter,
/// falling back to last_cost_discard_count when no recent moves are tracked.
/// This is the correct behavior for both draw and gain_resource — they should
/// count only cards moved by the current cost/effect batch, not the full waitroom.
pub fn resolve_discard_per_unit_count(
    recently_moved: Option<&Vec<i16>>,
    last_discard_count: u32,
    card_db: &CardDatabase,
    filter: &CardFilter,
) -> u32 {
    if let Some(moved) = recently_moved {
        count_matching(moved, card_db, filter, false)
    } else {
        last_discard_count
    }
}

// ============== PER-UNIT CALCULATION ==============

pub fn calculate_per_unit_multiplier(
    per_unit: bool,
    per_unit_type: Option<&str>,
    player: &crate::player::Player,
    orientation_modifiers: &std::collections::HashMap<i16, String>,
    state_filter: Option<&str>,
) -> u32 {
    if !per_unit {
        return 1;
    }
    let stage_count = |state: Option<&str>| -> u32 {
        player
            .stage
            .stage
            .iter()
            .filter(|&&c| c != -1)
            .filter(|&&cid| match state {
                Some(s) => orientation_modifiers
                    .get(&cid)
                    .map_or(s == "active", |o| o.as_str() == s),
                None => true,
            })
            .count() as u32
    };
    match per_unit_type {
        Some("member") | Some("人") | Some("members") => stage_count(state_filter),
        Some("hand") | Some("card") | Some("枚") => player.hand.cards.len() as u32,
        Some("energy") => player.energy_zone.cards.len() as u32,
        Some("live_card_zone") => player.live_card_zone.cards.len() as u32,
        Some("discard") => player.waitroom.cards.len() as u32,
        Some("under_member") | Some("下") => player
            .stage
            .under_cards
            .iter()
            .map(|sv| sv.len())
            .sum::<usize>() as u32,
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
    state_filter: Option<&str>,
    orientation_modifiers: &std::collections::HashMap<i16, String>,
) -> u32 {
    if !per_unit {
        return 1;
    }
    // heart_colors: count unique heart colors across matching stage cards
    if per_unit_type == Some("heart_colors") {
        let mut colors_found: std::collections::HashSet<crate::card::HeartColor> =
            std::collections::HashSet::new();
        let stage_cards = zone_cards(player, Zone::Stage.to_str());
        for &cid in stage_cards {
            if filter.matches(card_db, cid, true) {
                if let Some(card) = card_db.get_card(cid) {
                    for color_str in heart_colors {
                        let hc = parse_heart_color(color_str);
                        let has = card
                            .base_heart
                            .as_ref()
                            .map_or(false, |bh| bh.hearts.contains_key(&hc))
                            || card
                                .need_heart
                                .as_ref()
                                .map_or(false, |nh| nh.hearts.contains_key(&hc));
                        if has {
                            colors_found.insert(hc);
                        }
                    }
                }
            }
        }
        return colors_found.len() as u32;
    }

    let zone = match per_unit_type {
        Some("stage") | Some("member") | Some("人") | Some("members") => Zone::Stage.to_str(),
        Some("hand") | Some("card") => Zone::Hand.to_str(),
        Some("under_member") => Zone::UnderMember.to_str(),
        Some("枚") => {
            let has_member_ct = filter.card_type == Some("member_card");
            if has_member_ct {
                Zone::UnderMember.to_str()
            } else {
                Zone::Hand.to_str()
            }
        }
        Some("discard") => Zone::Waitroom.to_str(),
        Some("live_card_zone") => Zone::LiveCardZone.to_str(),
        Some("success_live_zone") | Some("success_live_card_zone") => {
            Zone::SuccessLiveZone.to_str()
        }
        _ => return 1,
    };
    if Zone::from_str(zone) == Some(Zone::UnderMember) {
        let cards: Vec<i16> = player
            .stage
            .under_cards
            .iter()
            .flat_map(|sv| sv.iter())
            .copied()
            .collect();
        if heart_colors.is_empty() {
            count_matching(&cards, card_db, filter, false)
        } else {
            cards
                .iter()
                .filter(|&&id| {
                    filter.matches(card_db, id, false)
                        && card_matches_heart_colors(card_db, id, heart_colors)
                })
                .count() as u32
        }
    } else {
        let mut cards: Vec<i16> = zone_cards(player, zone).to_vec();
        // Apply state filter (wait/active) for stage cards
        let is_stage = Zone::from_str(zone) == Some(Zone::Stage);
        if is_stage {
            if let Some(state) = state_filter {
                cards.retain(|&cid| {
                    orientation_modifiers
                        .get(&cid)
                        .map_or(state == "active", |o| o.as_str() == state)
                });
            }
        }
        if heart_colors.is_empty() {
            count_matching(&cards, card_db, filter, is_stage)
        } else {
            cards
                .iter()
                .filter(|&&id| {
                    filter.matches(card_db, id, is_stage)
                        && card_matches_heart_colors(card_db, id, heart_colors)
                })
                .count() as u32
        }
    }
}

// ============== DISTINCT FILTERING ==============

pub fn apply_distinct_filter(
    cards: &[i16],
    distinct: Option<&str>,
    card_db: &CardDatabase,
) -> Vec<i16> {
    let should = matches!(
        distinct,
        Some("card_name") | Some("true") | Some("distinct")
    );
    if !should {
        return cards.to_vec();
    }
    let mut seen = std::collections::HashSet::new();
    cards
        .iter()
        .filter(|&&id| {
            card_db
                .get_card(id)
                .map(|c| seen.insert(CardDatabase::normalize_name(&c.name)))
                .unwrap_or(true)
        })
        .copied()
        .collect()
}

// ============== ZONE CARD COUNT ==============

pub fn get_zone_card_count(player: &crate::player::Player, zone: &str) -> usize {
    if Zone::from_str(zone) == Some(Zone::Stage) {
        return player.stage.stage.iter().filter(|&&c| c != -1).count();
    }
    if Zone::from_str(zone) == Some(Zone::UnderMember) {
        return player.stage.under_cards.iter().map(|sv| sv.len()).sum();
    }
    zone_cards(player, zone).len()
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
            game_state
                .temporary_effects
                .push(crate::game_state::TemporaryEffect {
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
        let end = nums_start
            + text[nums_start..]
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .count();
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

// ============== SELECTION PRIMITIVES ==============
// Shared across move_cards, cost, and any other zone-selection logic.

/// How a zone selection resolves when there aren't enough matching cards.
#[derive(Clone)]
pub enum InsufficientBehavior {
    /// Silently skip (treat as zero cards taken).
    Silent,
    /// Return an error with the given message.
    Error(String),
}

/// The outcome of resolving a card selection from a zone.
#[derive(Debug, Clone)]
pub enum SelectionOutcome {
    /// Exact match — the indices to take.
    Exact(Vec<usize>),
    /// Too many candidates — the caller must prompt the player.
    Prompt,
    /// Too few candidates — skip silently.
    Skip,
}

/// Classify a set of candidate indices against a required count.
pub fn classify_selection(
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
            InsufficientBehavior::Error(msg) => Err(msg),
        };
    }
    if idxs.len() > count {
        return Ok(SelectionOutcome::Prompt);
    }
    Ok(SelectionOutcome::Exact(idxs.to_vec()))
}

/// Return indices into `cards` matching the filter, with optional self-target pinning.
pub fn get_selection_indices(
    cards: &[i16],
    card_db: &CardDatabase,
    activating_card: Option<i16>,
    filter: &CardFilter,
    self_target_only: bool,
    skip_empty: bool,
) -> Vec<usize> {
    log::debug!(
        "[GET_SEL] cards.len={} filter.nh_color={:?} filter.nh_total={:?}",
        cards.len(),
        filter.need_heart_color,
        filter.need_heart_total
    );
    let mut idxs = matching_indices(cards, card_db, filter, skip_empty);
    if self_target_only {
        if let Some(aid) = activating_card {
            idxs.retain(|&i| i < cards.len() && cards[i] == aid);
        }
    }
    idxs
}

/// Full selection resolution: filter → classify → SelectionOutcome.
pub fn resolve_selection(
    cards: &[i16],
    card_db: &CardDatabase,
    activating_card: Option<i16>,
    count: usize,
    is_all: bool,
    filter: &CardFilter,
    self_target_only: bool,
    behavior: InsufficientBehavior,
    skip_empty: bool,
) -> Result<SelectionOutcome, String> {
    let idxs = get_selection_indices(
        cards,
        card_db,
        activating_card,
        filter,
        self_target_only,
        skip_empty,
    );
    classify_selection(&idxs, count, is_all, behavior)
}

/// Remove cards from a standard (non-stage, non-deck) zone at the given indices.
/// Indices are sorted descending so earlier removals don't shift later ones.
/// Returns the removed card IDs.
/// Remove cards from a named zone by indices (indices processed in descending order).
pub fn zone_remove_at_indices(
    player: &mut crate::player::Player,
    zone: &str,
    indices: &[usize],
) -> Vec<i16> {
    let mut sorted = indices.to_vec();
    sorted.sort_unstable_by(|a, b| b.cmp(a));
    sorted
        .iter()
        .map(|&i| match Zone::from_str(zone) {
            Some(Zone::Hand) => player.hand.cards.remove(i),
            Some(Zone::Discard) | Some(Zone::Waitroom) => player.waitroom.cards.remove(i),
            Some(Zone::Energy) => player.energy_zone.cards.remove(i),
            Some(Zone::LiveCardZone) => player.live_card_zone.cards.remove(i),
            Some(Zone::SuccessLiveZone) => player.success_live_card_zone.cards.remove(i),
            Some(Zone::EnergyDeck) => player.energy_deck.cards.remove(i),
            _ => {
                if zone == "those_cards" {
                    player.waitroom.cards.remove(i)
                } else {
                    -1
                }
            }
        })
        .filter(|&c| c != -1)
        .collect()
}
