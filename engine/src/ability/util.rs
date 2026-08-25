use crate::core::constants::U8Count;
use super::enums::{ActionType, Zone};
use crate::card::{parse_heart_color, AbilityFilter, CardDatabase, DistinctType, Operator};
use crate::game_state::Duration;
use crate::{HashMap, HashSet};
#[cfg(feature = "no_std")]
use alloc::{
    borrow::Cow,
    string::{String, ToString},
    vec::Vec,
};
use smallvec::SmallVec;
#[cfg(not(feature = "no_std"))]
use std::borrow::Cow;

// ============== HEART GAIN DISTRIBUTION ==============

/// Distribute a multi-color heart gain across its color multiset.
///
/// A heart gain is described by a multiset `heart_colors` (one token per
/// granted heart, e.g. `heart02 heart02`) and a `total` number of hearts to
/// grant. Historically the engine granted `total` to EACH color, so a
/// one-of-each ability (`heart02/heart04/heart05`) became three-of-each.
///
/// We instead distribute `total` evenly across the multiset: each entry
/// receives `total / len(heart_colors)`. Because the parser sets
/// `count == len(heart_colors)`, this reduces to granting exactly 1 to each
/// entry (e.g. `[heart02, heart02]` with total 2 → 1 of heart02, total 2).
pub fn heart_gain_per_entry(total: i32, heart_colors: &[String]) -> i32 {
    let len = heart_colors.len().max(1);
    total / len as i32
}

/// `heart_type == "all"` means wildcard any-color heart. The constant-heart
/// map stores it as `"heart00"` (HeartColor::Heart00 placeholder) while the
/// GainAbility path stores it as `"all"`. Keep the string check centralized.
pub fn is_all_heart_type(effect: &crate::card::AbilityEffect) -> bool {
    effect.heart_type_any().as_deref() == Some("all")
}
pub const HEART_ALL_KEY: &str = "heart00";

// ============== CONSTANT PER_UNIT ==============

/// Resolve the zone string for a constant per_unit gain. Mirrors the
/// `loc_b.or(per_b).unwrap_or(Hand)` logic duplicated in `modifiers.rs`.
pub fn constant_per_unit_zone(effect: &crate::card::AbilityEffect) -> &str {
    let loc_b = effect.location_any();
    let per_b = effect.per_unit_type_any();
    loc_b.or(per_b).unwrap_or(Zone::Hand.to_str())
}

/// Compute the `units` part of a constant per_unit gain (before `base`).
/// This is the `resolve_per_unit_count / per_unit_count + max cap` stanza
/// that was copy-pasted between blade/heart constant paths in `modifiers.rs`.
pub fn constant_per_unit_units(
    effect: &crate::card::AbilityEffect,
    player: &crate::player::Player,
    card_db: &CardDatabase,
    orientation_modifiers: &HashMap<i16, crate::core::game_modifiers::CardOrientation>,
    host_card_id: i16,
) -> i32 {
    let zone = constant_per_unit_zone(effect);
    let mut filter = effect.filter_subset();
    filter.exclude_self = Some(host_card_id);
    let per_count = resolve_per_unit_count(
        true,
        Some(zone),
        player,
        card_db,
        &filter,
        &[],
        effect.state_any().as_deref(),
        orientation_modifiers,
        Some(host_card_id),
    );
    let mut units = per_count as i32 / effect.per_unit_count_any().unwrap_or(1).max(1) as i32;
    if effect.max.unwrap_or(false) {
        if let Some(cap) = effect.count_any() {
            units = units.min(cap as i32);
        }
    }
    log::debug!(
        "[CONST_PER_UNIT] host={} player={} zone={} state={:?} target={:?} raw_count={} per_unit_divisor={:?} units={} capped={}",
        host_card_id,
        player.id,
        zone,
        effect.state_any().as_deref(),
        effect.target_any(),
        per_count,
        effect.per_unit_count_any(),
        per_count as i32 / effect.per_unit_count_any().unwrap_or(1).max(1) as i32,
        units
    );
    units
}

// ============== MODIFY COST ==============

pub fn find_modify_cost<'a>(
    effect: &'a crate::card::AbilityEffect,
    op: Option<&str>,
    loc: Option<&str>,
) -> Option<&'a crate::card::AbilityEffect> {
    if effect.action == crate::ability::enums::ActionType::ModifyCost
        && op.is_none_or(|o| effect.operation_any().as_deref() == Some(o))
        && loc.is_none_or(|l| effect.location_any().as_deref() == Some(l))
    {
        return Some(effect);
    }
    if effect.action == ActionType::Sequential {
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
        .group_names_any()
        .as_ref()
        .and_then(|gn| {
            gn.first()
                .map(|g| card_matches_group_str(card_db, card_id, Some(g)))
        })
        .unwrap_or(true);
    if !group_matches {
        return false;
    }
    if let Some(limit) = effect.cost_limit_any() {
        if card.cost != Some(limit) {
            return false;
        }
    }
    if !cost_threshold_met(card, effect) {
        return false;
    }
    if let Some(ct) = effect.card_type_any() {
        if *ct != crate::card::CardType::Member {
            return false;
        }
    }
    // ability_filter: e.g. "no_ability" means only reduce cost for members
    // whose abilities list is empty.
    if effect.ability_filter_any().as_deref() == Some("no_ability") {
        if !card.abilities.is_empty() {
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
) -> u8 {
    let pul_binding = effect.per_unit_location_any();
    let loc_binding = effect.location_any();
    let count_zone = pul_binding.or(loc_binding).unwrap_or("hand");

    let raw_count = if count_zone == "stage" && effect.group_names_any().is_some() {
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

    let per_unit_count = effect.per_unit_count_any().unwrap_or(1).max(1) as usize;
    let exclude_self = effect.exclude_self_any().unwrap_or(false);
    let effective = if exclude_self {
        raw_count.saturating_sub(1)
    } else {
        raw_count
    };
    let value = effect.value_any().unwrap_or(1) as u8;
    ((effective / per_unit_count) as u8) * value
}

pub fn calculate_play_cost_reduction(
    stage: &crate::core::zones::Stage,
    success_live_cards: &[i16],
    hand_count: usize,
    card_id: i16,
    card_db: &CardDatabase,
) -> u8 {
    let card = match card_db.get_card(card_id) {
        Some(c) => c,
        None => return 0,
    };

    // ── 1. Scan the played card's own abilities for a self-reduction ──────────
    let mut cost_reduction: u8 = 0;
    for ar in &card.abilities {
        let ability = ar.resolve();
        if let Some(ref effect) = ability.effect {
            if let Some(mod_cost) = find_modify_cost(effect, Some("subtract"), Some("hand")) {
                if !play_cost_reduction_matches(mod_cost, card_id, card, card_db) {
                    continue;
                }
                if mod_cost.per_unit_any().unwrap_or(false) {
                    cost_reduction = per_unit_cost_reduction(mod_cost, stage, hand_count, card_db);
                } else {
                    let reduction = mod_cost.value_any().unwrap_or(1);
                    cost_reduction = cost_reduction.max(reduction as u8);
                }
                break;
            }
        }
    }

    // ── 2. Scan stage cards for cost-reduction auras that apply to card_id ───
    for &stage_id in &stage.stage {
        if stage_id == -1 {
            continue;
        }
        if let Some(stage_card) = card_db.get_card(stage_id) {
            if let Some(r) = scan_abilities_for_cost_reduction(
                &stage_card.abilities,
                card_id,
                card,
                card_db,
                stage,
                hand_count,
                true, // enforce hand-condition guard (card is on stage, not in hand)
            ) {
                // Stack: sum reductions from all qualifying stage cards
                cost_reduction += r;
            }
        }
    }

    // ── 3. Scan succeeded live cards for cost-reduction auras ─────────────────
    if cost_reduction == 0 {
        for &live_id in success_live_cards {
            if let Some(live_card) = card_db.get_card(live_id) {
                if let Some(r) = scan_abilities_for_cost_reduction(
                    &live_card.abilities,
                    card_id,
                    card,
                    card_db,
                    stage,
                    hand_count,
                    false, // live cards have no hand-condition guard
                ) {
                    cost_reduction = cost_reduction.max(r);
                    break;
                }
            }
            if cost_reduction > 0 {
                break;
            }
        }
    }

    cost_reduction
}

/// Compute the full play cost for a member card being played from hand:
/// `base − self/stage/live-zone reductions + success-live-zone increase`,
/// then apply a constant set-override ("このカードのコストはNになる") if present.
///
/// This is the single source of truth for play-cost — all four inputs
/// (reduction, increase, set-override, base) are combined here so the flow is
/// obvious in one place instead of split across player.rs and phases.rs.
pub fn compute_play_cost(
    player: &crate::player::Player,
    card_id: i16,
    card_db: &CardDatabase,
    set_override: i32,
) -> u8 {
    let Some(card) = card_db.get_card(card_id) else {
        return 0;
    };
    let base_cost = card.cost.unwrap_or(0);
    // Card is being removed from hand; +1 recovers the true hand count for
    // per-unit reductions.
    let hand_count = player.hand.cards.len() + 1;
    let reduction = calculate_play_cost_reduction(
        &player.stage,
        &player.success_live_card_zone.cards,
        hand_count,
        card_id,
        card_db,
    );
    // Cost increase from 常時 abilities (success_live_zone cards → +cost).
    let mut increase: u8 = 0;
    for ar in &card.abilities {
        let ability = ar.resolve();
        if let Some(ref effect) = ability.effect {
            if effect.action == crate::ability::enums::ActionType::ModifyCost
                && matches!(
                    effect.operation_any().as_deref(),
                    Some("increase") | Some("add")
                )
                && crate::ability::enums::Zone::from_str(
                    effect.location_any().as_deref().unwrap_or(""),
                ) == Some(crate::ability::enums::Zone::SuccessLiveZone)
            {
                let per_unit_count = effect.per_unit_count_any().unwrap_or(1) as usize;
                let success_count = player.success_live_card_zone.cards.len();
                let multiplier = effect.count.unwrap_or(1);
                increase = ((success_count / per_unit_count) as u8) * multiplier;
            }
        }
    }
    let mut cost = base_cost.saturating_sub(reduction).saturating_add(increase);
    // Constant set-override ("コストはNになる", e.g. LL-bp7-001-R＋ ab#0)
    // replaces the base play cost entirely. 0 means no set modifier.
    if set_override != 0 && set_override != base_cost as i32 {
        cost = crate::constants::saturate_u8(set_override);
    }
    cost
}

/// Inner predicate shared across the three cost-reduction source scans.
/// Returns `Some(reduction)` if the first qualifying ModifyCost-subtract-hand
/// ability is found in `abilities`, or `None` if none match.
///
/// `hand_condition_guard`: when `true`, effects whose condition explicitly
/// requires `location == "hand"` are skipped (the aura card is on stage, not
/// in hand, so the condition is not met).
fn scan_abilities_for_cost_reduction(
    abilities: &[crate::ability::ability_store::AbilityRef],
    target_id: i16,
    target_card: &crate::card::Card,
    card_db: &CardDatabase,
    stage: &crate::core::zones::Stage,
    hand_count: usize,
    hand_condition_guard: bool,
) -> Option<u8> {
    for ar in abilities {
        let ability = ar.resolve();
        if let Some(ref effect) = ability.effect {
            if effect.action != ActionType::ModifyCost
                || effect.operation_any().as_deref() != Some("subtract")
                || effect.location_any().as_deref().and_then(Zone::from_str) != Some(Zone::Hand)
            {
                continue;
            }
            // Skip if the effect requires the aura-source to be in hand
            // (the card is on stage/live zone so this condition can't be met).
            if hand_condition_guard {
                if let Some(ref cond) = effect.condition {
                    if cond.get_location() == Some("hand") {
                        continue;
                    }
                }
            }
            // Group filter: the played card must belong to the aura's group.
            let group_matches = effect
                .group_names_any()
                .as_deref()
                .map(|gns| card_matches_any_group(card_db, target_id, gns))
                .unwrap_or(true);
            if !group_matches {
                continue;
            }
            // Exact cost-limit guard (e.g. "only for cost-N cards")
            if let Some(limit) = effect.cost_limit_any() {
                if target_card.cost != Some(limit) {
                    continue;
                }
            }
            if !cost_threshold_met(target_card, effect) {
                continue;
            }
            // Card-type guard: only applies to member/card types
            if let Some(ct) = effect.card_type_any() {
                if *ct != crate::card::CardType::Member {
                    continue;
                }
            }
            // ability_filter: e.g. "no_ability" means only reduce cost for
            // members whose abilities list is empty (the TARGET card, not
            // the stage card providing the aura).
            if effect.ability_filter_any().as_deref() == Some("no_ability") {
                if !target_card.abilities.is_empty() {
                    continue;
                }
            }
            let reduction = if effect.per_unit_any().unwrap_or(false) {
                per_unit_cost_reduction(effect, stage, hand_count, card_db)
            } else {
                effect.value_any().unwrap_or(1) as u8
            };
            return Some(reduction);
        }
    }
    None
}

fn cost_threshold_met(card: &crate::card::Card, effect: &crate::card::AbilityEffect) -> bool {
    match (
        effect.original_count_any(),
        effect.original_operator_any().as_deref(),
    ) {
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

/// Returns 0 for player1, 1 for player2 based on target and ability master.
pub fn target_player_index(target: &str, master: Option<&str>) -> Option<u8> {
    match (target, master) {
        ("self", Some("player2") | Some("p2")) => Some(1),
        ("self", _) => Some(0),
        ("opponent", Some("player2") | Some("p2")) => Some(0),
        ("opponent", _) => Some(1),
        (t, _) => {
            // Try to parse player label
            if t.eq_ignore_ascii_case("player1") || t == "p1" {
                Some(0)
            } else if t.eq_ignore_ascii_case("player2") || t == "p2" {
                Some(1)
            } else {
                None
            }
        }
    }
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

/// Whether a card whose current orientation modifier is `orientation` matches a
/// requested `state` ("active"/"wait"/"rest"). Cards with no modifier are
/// treated as active (the default orientation).
#[inline]
pub fn orientation_matches_state(orientation: Option<&str>, state: &str) -> bool {
    orientation.map_or(state == "active", |o| o == state)
}

/// Like `card_matches_group_str` but returns a vec of (reason, result) pairs
/// for each check so callers can log detailed diagnostics. Disabled by default;
/// enable via `RABUKA_DEBUG_GROUP=1`.
#[cfg(feature = "no_std")]
fn debug_group_match(
    _card_db: &CardDatabase,
    _card_id: i16,
    _group_name: Option<&str>,
    _result: bool,
) {
}

#[cfg(not(feature = "no_std"))]
fn debug_group_match(card_db: &CardDatabase, card_id: i16, group_name: Option<&str>, result: bool) {
    static DEBUG_GROUP: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if !*DEBUG_GROUP.get_or_init(|| std::env::var("RABUKA_DEBUG_GROUP").as_deref() == Ok("1")) {
        return;
    }
    let card = card_db.get_card(card_id);
    let card_name = card.as_ref().map(|c| c.name.as_ref()).unwrap_or("?");
    let series = card.as_ref().map(|c| c.series.as_ref()).unwrap_or("?");
    let unit = card.as_ref().and_then(|c| c.unit.as_deref()).unwrap_or("?");
    let checks = match group_name {
        Some(g) => {
            let gn = norm_group_name(g);
            card.as_ref()
                .map(|c| {
                    let unit_ok = norm_group_name(c.unit.as_deref().unwrap_or("")).as_ref() == gn.as_ref();
                    let group_ok = c.group.as_ref() == g;
                    let name_ok = card_db
                        .get_card_names(card_id)
                        .iter()
                        .any(|n| norm_group_name(n).as_ref().contains(gn.as_ref()));
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

/// Normalize full-width/half-width exclamation marks so that group names like
/// "みらくらぱーく！" match unit fields using either ！(U+FF01) or !(U+0021).
/// Also normalize µ (micro sign U+00B5) to μ (mu U+03BC) for μ's group matching.
/// Single source of truth shared by group matching and its debug logger.
fn norm_group_name(s: &str) -> Cow<'_, str> {
    let has_ff01 = s.contains('\u{FF01}');
    let has_mu = s.contains('\u{00B5}');
    if has_ff01 || has_mu {
        let mut r = if has_ff01 {
            s.replace('\u{FF01}', "!")
        } else {
            s.to_string()
        };
        if has_mu {
            r = r.replace('\u{00B5}', "\u{03BC}");
        }
        Cow::Owned(r)
    } else {
        Cow::Borrowed(s)
    }
}

pub fn card_matches_group_str(
    card_db: &CardDatabase,
    card_id: i16,
    group_name: Option<&str>,
) -> bool {
    let result = match group_name {
        Some(g) => {
            let gn = norm_group_name(g);
            card_db
                .get_card(card_id)
                .map(|c| {
                    let unit = c.unit.as_deref().unwrap_or("");
                    let unit_match = unit == gn.as_ref()
                        || ((unit.contains('\u{FF01}') || unit.contains('\u{00B5}')) && norm_group_name(unit).as_ref() == gn.as_ref());
                    unit_match
                || c.group.as_ref() == g
                || card_db.get_card_names(card_id).iter().any(|n| n.contains(gn.as_ref())
                    || ((n.contains('\u{FF01}') || n.contains('\u{00B5}')) && norm_group_name(n).as_ref().contains(gn.as_ref())))
                // Multi-name cards (e.g. 渡辺曜&鬼塚夏美&大沢瑠璃乃) should match
                // group names through any of their constituent series (Q105).
                // Example: LL-bp2-001-R+ matches "Aqours" via ラブライブ！サンシャイン!!
                || card_series_matches_group(&c.series, gn.as_ref())
                // Constant `set_card_identity` ("treated as") abilities give the
                // card additional group memberships in all zones. Examples:
                //   AURORA FLOWER (PL!HS-bp5-018-L) is "スリーズブーケ" /
                //   "DOLLCHESTRA" / "みらくらぱーく！" everywhere.
                || c.abilities.iter().any(|ar| {
                    ar.resolve().effect.as_ref().is_some_and(|eff| {
                        eff.action == ActionType::SetCardIdentity
                            && eff.identities_any().as_ref().is_some_and(|ids| {
                                ids.iter().any(|id| id == gn.as_ref() || ((id.contains('\u{FF01}') || id.contains('\u{00B5}')) && norm_group_name(id).as_ref() == gn.as_ref()))
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

/// Returns true if `card_id` matches ANY group in `groups`, or if `groups` is
/// empty (meaning no group filter). This replaces the repeated pattern:
///   `effect.group_names.as_ref().and_then(|gn| gn.first().map(|g| card_matches_group_str(...)))`
/// across ~35 handler call-sites.
pub fn card_matches_any_group(card_db: &CardDatabase, card_id: i16, groups: &[String]) -> bool {
    groups.is_empty()
        || groups
            .iter()
            .any(|g| card_matches_group_str(card_db, card_id, Some(g)))
}

/// Map an `activation_position` token ("left"/"left_side"/"center"/
/// "right"/"right_side") to the stage slot index. Single source of truth;
/// callers decide their own policy for unknown tokens.
pub fn activation_position_index(p: &str) -> Option<usize> {
    match p.trim() {
        "left" | "left_side" => Some(0),
        "center" => Some(1),
        "right" | "right_side" => Some(2),
        _ => None,
    }
}

/// Returns true if `existing_card` (the member already on the stage area)
/// prevents a baton touch by `incoming_card_id` — i.e. it has a
/// `cannot_baton_touch` restriction that is not excluded by the incoming
/// card's groups. Single source of truth shared by hand-play validation,
/// double-baton play, and action generation.
pub fn has_cannot_baton_touch_protection(
    card_db: &CardDatabase,
    incoming_card_id: i16,
    existing_card: &crate::card::Card,
) -> bool {
    existing_card.resolved_abilities().any(|ability| {
        ability.effect.as_ref().is_some_and(|ef| {
            if ef.restriction_type_any().as_deref() != Some("cannot_baton_touch") {
                return false;
            }
            if let Some(ref exclude_groups) = ef.exclude_group_names_any() {
                if card_matches_any_group(card_db, incoming_card_id, exclude_groups) {
                    return false;
                }
            }
            true
        })
    })
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
    characters: Option<&[String]>,
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
    cost_limit: Option<u8>,
) -> bool {
    card_matches_cost_limit_op(card_db, card_id, cost_limit, None)
}

pub fn card_matches_cost_limit_op(
    card_db: &CardDatabase,
    card_id: i16,
    cost_limit: Option<u8>,
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

/// Like `card_matches_heart_colors` but uses AND logic: card must have ALL listed
/// heart colors, not just any one. Used when `require_all_heart_colors` is true.
pub fn card_matches_all_heart_colors(
    card_db: &CardDatabase,
    card_id: i16,
    heart_colors: &[String],
) -> bool {
    if heart_colors.is_empty() {
        return true;
    }
    let result = card_db.get_card(card_id).is_none_or(|card| {
        heart_colors.iter().all(|color| {
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
        Some(name) => {
            let norm = CardDatabase::normalize_name(name);
            card_db
                .get_card_names(card_id)
                .iter()
                .any(|cn| CardDatabase::normalize_name(cn) == norm)
        }
        None => true,
    }
}

/// Result of computing the maximum distinct name count across a set of cards,
/// where each multi-name card contributes exactly ONE of its constituent names
/// (the player chooses optimally).  Computed exactly via bitmask DP.
#[derive(Debug, Copy, Clone)]
pub struct DistinctNamesResult {
    /// Maximum number of distinct names achievable by choosing one name per card.
    pub distinct: usize,
    /// Whether a collision-free assignment exists (all cards can pick a unique name).
    pub collision: bool,
}

/// Optimal search for the best name assignment across cards.
///
/// Each entry in `name_sets` is the list of constituent names for one card
/// (multi-name cards produce multiple entries via get_card_names, single-name
/// cards produce one entry). Picks exactly one name per card.
///
/// Exact algorithm: layered bitmask DP over the deduplicated name universe
/// with domination pruning (a mask that is a superset of another always
/// yields an equal-or-better final count, so non-maximal masks are dropped).
/// This replaces the previous exhaustive DFS (exponential in cards, cloning a
/// HashSet per node) and the >12-card greedy fallback that provably
/// undercounted. Greedy survives only as a safety net for pathological
/// universes (> 128 unique names, where the bitmask no longer fits).
pub fn max_distinct_names(name_sets: &[Vec<String>]) -> DistinctNamesResult {
    if name_sets.is_empty() {
        return DistinctNamesResult {
            distinct: 0,
            collision: false,
        };
    }
    // A card with no names means no complete assignment exists — mirrors the
    // degenerate behavior of the old DFS (zero leaves reached).
    if name_sets.iter().any(|ns| ns.is_empty()) {
        return DistinctNamesResult {
            distinct: 0,
            collision: true,
        };
    }

    // Map the name universe to bit positions; build each card's option mask.
    let mut ids: HashMap<&str, u32> = HashMap::default();
    let mut option_masks: Vec<u128> = Vec::with_capacity(name_sets.len());
    let mut universe_len = 0u32;
    for names in name_sets {
        let mut mask = 0u128;
        for name in names {
            let next = universe_len;
            let id = *ids.entry(name.as_str()).or_insert_with(|| {
                universe_len += 1;
                next
            });
            mask |= 1u128 << id;
        }
        option_masks.push(mask);
    }

    if universe_len > 128 {
        // Pathological universe — bitmask DP impossible. First-fit greedy
        // keeps the engine running but MAY UNDERCOUNT; say so loudly.
        log::warn!(
            "max_distinct_names: {} unique names exceeds 128-bit mask; \
             falling back to greedy (result may undercount)",
            universe_len
        );
        return max_distinct_names_greedy(name_sets);
    }

    let n_cards = option_masks.len();
    // Frontier = antichain of maximal reachable picked-masks.
    let mut frontier: Vec<u128> = vec![0u128];
    for mask in &option_masks {
        let mut next: Vec<u128> = Vec::with_capacity(frontier.len());
        for &f in &frontier {
            // Each candidate extends the frontier mask by one of the card's names.
            let mut rest = *mask;
            while rest != 0 {
                let bit = rest & (!rest + 1); // lowest set bit
                rest &= !bit;
                next.push(f | bit);
            }
        }
        prune_dominated(&mut next);
        frontier = next;
    }

    let best = frontier
        .iter()
        .map(|m| m.count_ones() as usize)
        .max()
        .unwrap_or(0);
    let collision_free_exists = frontier
        .iter()
        .any(|m| m.count_ones() as usize == n_cards);
    DistinctNamesResult {
        distinct: best,
        collision: !collision_free_exists,
    }
}

/// Removes any mask that is a strict subset of another mask in the list.
/// Dominated masks can never lead to a better outcome (picking a superset now
/// dominates every future continuation), so they are safe to drop.
fn prune_dominated(masks: &mut Vec<u128>) {
    masks.sort_unstable();
    masks.dedup();
    let mut kept: Vec<u128> = Vec::with_capacity(masks.len());
    'outer: for &m in masks.iter() {
        for &k in kept.iter() {
            if k & m == m {
                continue 'outer; // m ⊆ k → dominated
            }
        }
        // m may dominate entries already kept.
        kept.retain(|&k| m & k != k);
        kept.push(m);
    }
    *masks = kept;
}

/// First-fit greedy fallback. Correct only when a collision-free assignment
/// exists trivially; otherwise the distinct count can be an undercount.
fn max_distinct_names_greedy(name_sets: &[Vec<String>]) -> DistinctNamesResult {
    let mut seen: HashSet<String> = HashSet::default();
    let mut had_collision = false;
    for names in name_sets {
        let mut picked = false;
        for name in names {
            if seen.insert(name.clone()) {
                picked = true;
                break;
            }
        }
        if !picked {
            // All names already in set — take the first (already seen).
            if let Some(first) = names.first() {
                seen.insert(first.clone());
            }
            had_collision = true;
        }
    }
    DistinctNamesResult {
        distinct: seen.len(),
        collision: had_collision,
    }
}

// ============== UNIFIED FILTER STRUCT ==============

/// Unified card filter: all fields are Optional — None = match anything.
#[derive(Default, Clone)]
pub struct CardFilter<'a> {
    pub card_type: Option<&'a str>,
    pub group: Option<&'a str>,
    pub groups: Option<&'a Vec<String>>,
    pub cost_limit: Option<u8>,
    pub cost_operator: Option<&'a str>,
    /// Discrete set of allowed cost values (OR) — e.g. "コストが10か20" → [10, 20].
    pub cost_values: Option<&'a Vec<u8>>,
    /// Minimum cost bound for range filters (e.g. cost >= 4)
    pub cost_limit_min: Option<u8>,
    /// Maximum cost bound for range filters (e.g. 「コスト4以上9以下」 -> 9)
    pub cost_limit_max: Option<u8>,
    /// Sum-total cost constraint — checked post-selection, not in per-card matches()
    pub cost_total: Option<u8>,
    pub cost_total_operator: Option<&'a str>,
    /// "元々持つブレード" — printed/base blade value filter (checked in matches()).
    pub original_blade_limit: Option<u8>,
    pub original_blade_operator: Option<&'a str>,
    /// "ブレードをNつ以上持つ" (no 元々) — CURRENT blade total filter (base or
    /// set + additive modifiers). Cannot be evaluated in matches() (no mods
    /// access), so it is applied as a post-filter by the caller via
    /// filter_current_blade().
    pub current_blade_limit: Option<u8>,
    pub current_blade_operator: Option<&'a str>,
    pub characters: Option<&'a Vec<String>>,
    pub exclude_characters: Option<&'a Vec<String>>,
    pub heart_colors: &'a [String],
    /// When true, card must match ALL heart_colors (AND).
    /// When false, card matches ANY heart_colors (OR, default).
    pub require_all_heart_colors: bool,
    /// Minimum count per heart color (e.g. 2 for "heart05を2個以上").
    pub heart_color_count: Option<u8>,
    pub need_heart_total: Option<u8>,
    pub need_heart_operator: Option<&'a str>,
    pub need_heart_color: Option<&'a str>,
    pub name_fragments: Option<&'a Vec<String>>,
    pub distinct: Option<DistinctType>,
    pub exclude_self: Option<i16>,
    /// Group names to exclude from matching (e.g. 「スリーズブーケ」以外)
    pub exclude_group_names: Option<&'a [String]>,
    /// Card IDs to exclude from matching (e.g. previously selected by a prior sequential action)
    pub exclude_cards: Option<&'a [i16]>,
    /// Card names to exclude from matching (resolved at runtime from
    /// exclude_by_name_source on the AbilityEffect).
    pub exclude_names: Option<&'a Vec<String>>,
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
    pub fn heart_colors(mut self, hc: &'a [String]) -> Self {
        self.heart_colors = hc;
        self
    }
    pub fn distinct(mut self, d: DistinctType) -> Self {
        self.distinct = Some(d);
        self
    }
    pub fn original_blade_limit(mut self, obl: Option<u8>, obo: Option<&'a str>) -> Self {
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
            || self.cost_limit_max.is_some()
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

    fn check_exclude_self(&self, id: i16) -> bool {
        if let Some(exclude_id) = self.exclude_self {
            if id == exclude_id {
                log::debug!("[DBG matches] exclude_self id={} matched, excluding", id);
                return false;
            }
        }
        true
    }

    fn check_exclude_cards(&self, id: i16) -> bool {
        if let Some(ex) = self.exclude_cards {
            if ex.contains(&id) {
                return false;
            }
        }
        true
    }

    fn check_exclude_names(&self, db: &CardDatabase, id: i16) -> bool {
        if let Some(ex_names) = self.exclude_names {
            if let Some(card) = db.get_card(id) {
                let normalized_name: String =
                    card.name.chars().filter(|c| !c.is_whitespace()).collect();
                if ex_names.iter().any(|n| {
                    n.chars()
                        .filter(|c| !c.is_whitespace())
                        .eq(normalized_name.chars())
                }) {
                    return false;
                }
            }
        }
        true
    }

    fn check_group(&self, db: &CardDatabase, id: i16) -> bool {
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
        if let Some(ex_gns) = self.exclude_group_names {
            for g in ex_gns {
                if card_matches_group_str(db, id, Some(g.as_str())) {
                    return false;
                }
            }
        }
        true
    }

    fn check_heart_colors(&self, db: &CardDatabase, id: i16) -> bool {
        if !self.heart_colors.is_empty() {
            let matches = if self.require_all_heart_colors {
                card_matches_all_heart_colors(db, id, self.heart_colors)
            } else {
                card_matches_heart_colors(db, id, self.heart_colors)
            };
            if !matches {
                return false;
            }
        }
        // Heart color count threshold check (e.g. "heart05を2個以上").
        if let Some(min_count) = self.heart_color_count {
            let passes = if self.require_all_heart_colors {
                self.heart_colors.iter().all(|color| {
                    let hc = crate::card::parse_heart_color(color);
                    let base_amount = db
                        .get_card(id)
                        .and_then(|c| c.base_heart.as_ref())
                        .map(|bh| *bh.hearts.get(&hc).unwrap_or(&0))
                        .unwrap_or(0);
                    let need_amount = db
                        .get_card(id)
                        .and_then(|c| c.need_heart.as_ref())
                        .map(|nh| *nh.hearts.get(&hc).unwrap_or(&0))
                        .unwrap_or(0);
                    base_amount.max(need_amount) >= min_count as u8
                })
            } else {
                self.heart_colors.iter().any(|color| {
                    let hc = crate::card::parse_heart_color(color);
                    let base_amount = db
                        .get_card(id)
                        .and_then(|c| c.base_heart.as_ref())
                        .map(|bh| *bh.hearts.get(&hc).unwrap_or(&0))
                        .unwrap_or(0);
                    let need_amount = db
                        .get_card(id)
                        .and_then(|c| c.need_heart.as_ref())
                        .map(|nh| *nh.hearts.get(&hc).unwrap_or(&0))
                        .unwrap_or(0);
                    base_amount.max(need_amount) >= min_count as u8
                })
            };
            if !passes {
                return false;
            }
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
                let color = crate::card::parse_heart_color(color_str);
                let card_amount = db
                    .get_card(id)
                    .and_then(|c| c.need_heart.as_ref())
                    .map(|nh| *nh.hearts.get(&color).unwrap_or(&0))
                    .unwrap_or(0);
                let op = self.need_heart_operator.unwrap_or(">=");
                if !compare_counts(Some(op), card_amount.into(), need_total.into()) {
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
                if !compare_counts(Some(op), card_total, need_total.into()) {
                    return false;
                }
            }
        }
        true
    }

    fn check_ability_filter(&self, db: &CardDatabase, id: i16) -> bool {
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
                                if card.abilities.iter().any(|ar| {
                                    ar.resolve().triggers.as_ref().is_some_and(|t| {
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
                                if !card.abilities.iter().any(|ar| {
                                    ar.resolve().triggers.as_ref().is_some_and(|t| {
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
                                        !card.abilities.iter().any(|ar| {
                                            ar.resolve().triggers.as_ref().is_some_and(|t| {
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
                                        card.abilities.iter().any(|ar| {
                                            ar.resolve().triggers.as_ref().is_some_and(|t| {
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
        true
    }

    fn check_card_property(&self, db: &CardDatabase, id: i16) -> bool {
        if let Some(prop) = self.card_property {
            let has_property = match prop {
                "has_blade_heart" => db.get_card(id).is_some_and(|c| c.has_blade_heart()),
                "has_score_icon" => db.get_card(id).is_some_and(|c| c.has_score_icon()),
                "has_all_blade" => db.get_card(id).is_some_and(|c| c.has_all_blade()),
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

    /// Check whether a single card matches ALL present filter fields.
    pub fn matches(&self, db: &CardDatabase, id: i16, skip_empty: bool) -> bool {
        if skip_empty && id == -1 {
            return false;
        }
        if !self.check_exclude_self(id) {
            return false;
        }
        if !self.check_exclude_cards(id) {
            return false;
        }
        if !self.check_exclude_names(db, id) {
            return false;
        }
        if let Some(ct) = self.card_type {
            if !card_matches_type(db, id, Some(ct)) {
                return false;
            }
        }
        if !self.check_group(db, id) {
            return false;
        }
        if let Some(lim) = self.cost_limit {
            if !card_matches_cost_limit_op(db, id, Some(lim), self.cost_operator) {
                return false;
            }
        }
        if let Some(vals) = self.cost_values {
            let card_cost = db.get_card(id).and_then(|c| c.cost.or(c.score));
            if !card_cost.is_some_and(|v| vals.contains(&v)) {
                return false;
            }
        }
        if let Some(min) = self.cost_limit_min {
            if !card_matches_cost_limit_op(db, id, Some(min), Some(">=")) {
                return false;
            }
        }
        if let Some(max) = self.cost_limit_max {
            if !card_matches_cost_limit_op(db, id, Some(max), Some("<=")) {
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
        if !self.check_heart_colors(db, id) {
            return false;
        }
        if let Some(name) = self.name_fragments {
            if !card_matches_name_fragments(db, id, name) {
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
            if !compare_counts(self.original_blade_operator, card_blade.into(), bl.into()) {
                return false;
            }
        }
        // Per-card cost_total check — each individual card's cost must
        // satisfy the total-budget comparison (e.g. card.cost <= 4).
        if let Some(ct) = self.cost_total {
            if let Some(op) = self.cost_total_operator {
                let card_cost = db.get_card(id).and_then(|c| c.cost).unwrap_or(99);
                if !compare_counts(Some(op), card_cost.into(), ct.into()) {
                    return false;
                }
            }
        }
        if !self.check_ability_filter(db, id) {
            return false;
        }
        if !self.check_card_property(db, id) {
            return false;
        }
        true
    }

    pub fn matches_card(&self, db: &CardDatabase, id: i16) -> bool {
        self.matches(db, id, false)
    }

    pub fn count(&self, cards: &[i16], db: &CardDatabase) -> u8 {
        cards
            .iter()
            .filter(|&&id| self.matches(db, id, false))
            .count().u8_count()
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
        let card_type = effect.card_type_any().map(|ct| ct.as_card_str());
        let group_names = effect.group_names_any();

        let cost_operator = effect.cost_limit_operator_any().map(Operator::as_str);
        let cost_total_operator = effect.cost_total_operator_any().map(Operator::as_str);
        let need_heart_operator = effect.need_heart_operator_any().map(Operator::as_str);
        let need_heart_color = effect.need_heart_color_any();
        let distinct = effect.distinct_any();
        let original_blade_operator = effect.blade_limit_operator_any().map(Operator::as_str);
        let ability_filter = effect.ability_filter_any().map(AbilityFilter::as_str);
        let card_property = effect.card_property_any();
        CardFilter {
            card_type,
            group: group_names
                .as_ref()
                .and_then(|v| v.first())
                .map(|s| s.as_str()),
            groups: group_names.as_ref().map(|v| &**v),
            cost_limit: effect.cost_limit_any(),
            cost_operator,
            cost_values: None,
            cost_limit_min: effect.cost_limit_min_any(),
            cost_limit_max: effect.cost_limit_max_any(),
            cost_total: effect.cost_total_any(),
            cost_total_operator,
            characters: effect.characters_any(),
            exclude_characters: effect.exclude_characters_any(),
            exclude_group_names: effect.exclude_group_names_any().map(Vec::as_slice),
            heart_colors: &effect.heart_colors_any(),
            require_all_heart_colors: effect.require_all_heart_colors_any().unwrap_or(false),
            heart_color_count: effect.heart_color_count_any(),
            need_heart_total: effect.need_heart_total_any(),
            need_heart_operator,
            need_heart_color,
            name_fragments: if effect.card_names_any().map_or(true, |v| v.is_empty()) {
                None
            } else {
                effect.card_names_any()
            },
            distinct,
            exclude_self: if effect.exclude_self_any().unwrap_or(false) {
                Some(-1)
            } else {
                None
            },
            original_blade_operator: if effect.original_value_any().unwrap_or(false) {
                original_blade_operator
            } else {
                None
            },
            original_blade_limit: if effect.original_value_any().unwrap_or(false) {
                effect.blade_limit_any()
            } else {
                None
            },
            current_blade_operator: if effect.original_value_any().unwrap_or(false) {
                None
            } else {
                original_blade_operator
            },
            current_blade_limit: if effect.original_value_any().unwrap_or(false) {
                None
            } else {
                effect.blade_limit_any()
            },
            exclude_cards: None,
            exclude_names: None,
            ability_filter,
            ability_filter_triggers: effect.ability_filter_triggers_any().map(|v| &**v),
            or_ability_filters: effect.or_ability_filters_any().map(|v| &**v),
            card_property,
            negation: effect.negation_any().unwrap_or(false),
        }
    }

    /// Whether this filter needs a post-`matches()` current-blade check.
    pub fn has_current_blade_filter(&self) -> bool {
        self.current_blade_limit.is_some()
    }
}

/// Filter member candidates by their CURRENT blade total (base or set +
/// additive modifiers), matching "ブレードをNつ以上持つ" (no 元々) semantics.
/// `matches()` cannot evaluate this (no modifier access), so the gain/change
/// resolution calls this post-filter after computing candidates.
pub fn filter_current_blade(
    candidates: Vec<i16>,
    gs: &crate::game_state::GameState,
    blade_limit: Option<u8>,
    blade_operator: Option<&str>,
) -> Vec<i16> {
    let Some(bl) = blade_limit else {
        return candidates;
    };
    let op = blade_operator.unwrap_or(">=");
    candidates
        .into_iter()
        .filter(|&cid| {
            let base = gs
                .card_database
                .get_card(cid)
                .map(|c| c.blade as i32)
                .unwrap_or(0);
            let set = gs.mods.get_blade_set_modifier(cid);
            let effective = if set != 0 { set as i32 } else { base };
            let additive = gs.mods.get_blade_modifier(cid) - set as i32;
            let total = crate::constants::saturate_u8(effective + additive);
            compare_counts(Some(op), total, bl)
        })
        .collect()
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
    cost_limit: Option<u8>,
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
    cost_limit: Option<u8>,
    cost_operator: Option<&'a str>,
    characters: Option<&'a Vec<String>>,
    name_fragments: Option<&'a Vec<String>>,
    distinct: Option<DistinctType>,
    exclude_self: Option<i16>,
    cost_total: Option<u8>,
    cost_total_operator: Option<&'a str>,
    exclude_characters: Option<&'a Vec<String>>,
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
        exclude_characters,
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
    target_count: Option<u8>,
    distinct: Option<DistinctType>,
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
                let excluded_names: HashSet<String> = ids
                    .iter()
                    .filter_map(|id| db.get_card(*id).map(|c| c.name.to_string()))
                    .collect();
                if !excluded_names.is_empty() {
                    results.retain(|id| {
                        db.get_card(*id)
                            .map_or(true, |c| !excluded_names.contains(c.name.as_ref()))
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
) -> u8 {
    cards
        .iter()
        .filter(|&&id| filter.matches(db, id, skip_empty))
        .count().u8_count()
}

/// Count cards matching a filter, deduplicating by card name when
/// `filter.distinct` is set. Distinct is a set-level operation and cannot be
/// expressed inside the per-card `matches()` predicate, so zone-wide per-unit
/// counts must go through this instead of `count_matching`.
pub fn count_matching_distinct(
    cards: &[i16],
    db: &CardDatabase,
    filter: &CardFilter,
    skip_empty: bool,
) -> u8 {
    if filter.distinct.is_none() {
        return count_matching(cards, db, filter, skip_empty);
    }
    let matching: Vec<i16> = cards
        .iter()
        .filter(|&&id| filter.matches(db, id, skip_empty))
        .copied()
        .collect();
    // Joint-aware count for "名前の異なるカード" mechanics (Q278/Q279): ordinary cards
    // dedupe by name, and a joint (multi-name) card contributes one unit only when it
    // introduces a name not already present as a single-name card.
    if matches!(filter.distinct, Some(DistinctType::CardName)) {
        count_distinct_member_name_units(&matching, db) as u8
    } else {
        apply_distinct_filter(&matching, filter.distinct, db).len().u8_count()
    }
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

/// Deduplicate `items` by each card's normalized name, keeping first
/// occurrences. Cards missing from the database are kept. Single shared
/// implementation of the distinct-by-card-name pattern previously copied at
/// filter_distinct / apply_distinct_filter / the move_cards take path.
pub fn dedupe_by_normalized_name<T: Copy>(
    items: &[T],
    card_id: impl Fn(&T) -> i16,
    db: &CardDatabase,
) -> Vec<T> {
    let mut seen = HashSet::<String>::default();
    items
        .iter()
        .filter(|item| {
            db.get_card(card_id(item))
                .map(|c| seen.insert(CardDatabase::normalize_name(&c.name)))
                .unwrap_or(true)
        })
        .copied()
        .collect()
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
if !distinct_should_dedupe(filter.distinct) {
return ids;
}
dedupe_by_normalized_name(&ids, |&i| cards[i], db)
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
) -> u8 {
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

pub fn compare_counts(operator: Option<&str>, actual: u8, expected: u8) -> bool {
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
                // Rule 9.6.2.1.2.1: Card deployed to stage, track it.
                player.track_deployment(card_id);
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
            log::debug!(
                "[TRACE_SAME_AREA] card={} vacated={:?} stage_before={:?}",
                card_id,
                vacated_stage_area,
                player.stage.stage
            );
            if let Some(pos) = vacated_stage_area {
                if pos < 3 && player.stage.stage[pos] == -1 {
                    player.stage.stage[pos] = card_id;
                    player.track_deployment(card_id);
                } else if let Some(ep) = stage_first_empty(&player.stage.stage) {
                    player.stage.stage[ep] = card_id;
                    player.track_deployment(card_id);
                } else {
                    player.hand.add_card(card_id);
                }
            } else if let Some(ep) = stage_first_empty(&player.stage.stage) {
                player.stage.stage[ep] = card_id;
                player.track_deployment(card_id);
            } else {
                player.hand.add_card(card_id);
            }
            log::debug!("[TRACE_SAME_AREA] stage_after={:?}", player.stage.stage);
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

/// For per_unit_type="discard": count recently-moved cards matching a filter,
/// falling back to last_cost_discard_count when no recent moves are tracked.
/// This is the correct behavior for both draw and gain_resource — they should
/// count only cards moved by the current cost/effect batch, not the full waitroom.
pub fn resolve_discard_per_unit_count(
    recently_moved: Option<&SmallVec<[i16; 4]>>,
    last_discard_count: u8,
    card_db: &CardDatabase,
    filter: &CardFilter,
) -> u8 {
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
    orientation_modifiers: &HashMap<i16, crate::core::game_modifiers::CardOrientation>,
    state_filter: Option<&str>,
) -> u8 {
    if !per_unit {
        return 1;
    }
    let stage_count = |state: Option<&str>| -> u8 {
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
            .count().u8_count()
    };
    match per_unit_type {
        Some("member") | Some("人") | Some("members") => stage_count(state_filter),
        Some("hand") | Some("card") | Some("枚") => player.hand.cards.len().u8_count(),
        Some("energy") => player.energy_zone.cards.len().u8_count(),
        Some("live_card_zone") => player.live_card_zone.cards.len().u8_count(),
        Some("discard") => player.waitroom.cards.len().u8_count(),
        Some("under_member") | Some("下") => player
            .stage
            .under_cards
            .iter()
            .map(|sv| sv.len())
            .sum::<usize>() as u8,
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
    orientation_modifiers: &HashMap<i16, crate::core::game_modifiers::CardOrientation>,
    host_card_id: Option<i16>,
) -> u8 {
    if !per_unit {
        return 1;
    }
    // heart_colors: count unique heart colors across matching stage cards
    if per_unit_type == Some("heart_colors") {
        let mut colors_found: HashSet<crate::card::HeartColor> = HashSet::default();
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
        return colors_found.len().u8_count();
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
        // "このメンバーの下に置かれているカード1枚につき" is scoped to the HOST
        // member (whose ability this is), not to every member's under-cards.
        // Callers with a known host pass it explicitly; otherwise fall back to
        // counting under-cards of all members.
        let cards: Vec<i16> = if let Some(host_id) = host_card_id {
            player
                .stage
                .stage
                .iter()
                .position(|&id| id == host_id)
                .map(|idx| player.stage.under_cards[idx].iter().copied().collect())
                .unwrap_or_default()
        } else {
            player
                .stage
                .under_cards
                .iter()
                .flat_map(|sv| sv.iter())
                .copied()
                .collect()
        };
        if heart_colors.is_empty() {
            count_matching_distinct(&cards, card_db, filter, false)
        } else {
            let mut matching: Vec<i16> = cards
                .iter()
                .filter(|&&id| {
                    filter.matches(card_db, id, false)
                        && card_matches_heart_colors(card_db, id, heart_colors)
                })
                .copied()
                .collect();
            matching = apply_distinct_filter(&matching, filter.distinct, card_db);
            matching.len().u8_count()
        }
    } else {
        let mut cards: Vec<i16> = zone_cards(player, zone).to_vec();
        // Apply state filter (wait/active) for stage cards
        let is_stage = Zone::from_str(zone) == Some(Zone::Stage);
        if is_stage {
            if let Some(state) = state_filter {
                cards.retain(|&cid| {
                    orientation_matches_state(
                        orientation_modifiers.get(&cid).map(|o| o.as_str()),
                        state,
                    )
                });
            }
        }
        if heart_colors.is_empty() {
            count_matching_distinct(&cards, card_db, filter, is_stage)
        } else {
            let mut matching: Vec<i16> = cards
                .iter()
                .filter(|&&id| {
                    filter.matches(card_db, id, is_stage)
                        && card_matches_heart_colors(card_db, id, heart_colors)
                })
                .copied()
                .collect();
            matching = apply_distinct_filter(&matching, filter.distinct, card_db);
            matching.len().u8_count()
        }
    }
}

// ============== DISTINCT FILTERING ==============

#[inline]
pub(crate) fn distinct_should_dedupe(distinct: Option<DistinctType>) -> bool {
    matches!(
        distinct,
        Some(DistinctType::CardName) | Some(DistinctType::True) | Some(DistinctType::Distinct)
    )
}

pub fn apply_distinct_filter(
    cards: &[i16],
    distinct: Option<DistinctType>,
    card_db: &CardDatabase,
) -> Vec<i16> {
    if !distinct_should_dedupe(distinct) {
        return cards.to_vec();
    }
    dedupe_by_normalized_name(cards, |&id| id, card_db)
}

/// Count of distinct names among member cards for a "名前の異なるメンバーカード1枚につき"
/// (per different-named member card) effect, handling JOINT (多種統合, name "A&B&C")
/// cards correctly. Matches official rulings Q278/Q279:
///   - each ordinary single-name card contributes its name once (dedup);
///   - a joint (multi-name) card ("A&...") adds one additional unit ONLY when it
///     introduces at least one name not already present as a single-name card.
///   Q278 (歩 + joint{歩,かのん,花帆}) = 2; Q279 (歩+かのん+花帆 + same joint) = 3,
///   because the joint's constituent names are already present as standalones.
pub fn count_distinct_member_name_units(cards: &[i16], card_db: &CardDatabase) -> usize {
    let mut single_names: HashSet<String> = HashSet::default();
    let mut joints: Vec<i16> = Vec::new();
    for &id in cards {
        let Some(card) = card_db.get_card(id) else { continue };
        let raw = card.name.trim();
        if raw.contains('&') {
            joints.push(id);
        } else {
            single_names.insert(CardDatabase::normalize_name(&card.name));
        }
    }
    let mut count = single_names.len();
    for id in joints {
        let Some(card) = card_db.get_card(id) else { continue };
        let has_new = card
            .name
            .split('&')
            .any(|part| !single_names.contains(&CardDatabase::normalize_name(part)));
        if has_new {
            count += 1;
        }
    }
    count
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

/// Effect kinds that GameState::check_expired_effects knows how to REVERT.
/// Registering a temporary effect outside this set means its modifiers will
/// silently leak past expiry — a new kind must extend BOTH this list and the
/// expiry match. push_temporary_effect warns on violations.
fn is_revertable_effect_type(effect_type: &str) -> bool {
    matches!(
        effect_type,
        "activation_cost_increase"
            | "activation_cost_decrease"
            | "set_blade_count"
            | "gain_surplus_heart"
            | "heart_override"
            | "modify_cost"
            | "set_heart_type"
    ) || effect_type.starts_with("gain_blade")
        || effect_type.starts_with("gain_heart")
        || effect_type.starts_with("gain_ability:")
        || effect_type.starts_with("set_blade_type:")
        || effect_type.starts_with("modify_score_")
}

pub fn push_temporary_effect(
    game_state: &mut crate::game_state::GameState,
    effect_type: &str,
    duration: Option<&str>,
    target_player_id: &str,
    description: &str,
    effect_data: Option<crate::core::types::EffectData>,
) {
    if let Some(d) = duration {
        if d != "permanent" {
            if !is_revertable_effect_type(effect_type) {
                log::warn!(
                    "temporary effect type '{}' has no expiry revert handler; \
                     its modifiers will LEAK past expiry. Extend \
                     GameState::check_expired_effects and is_revertable_effect_type. \
                     description={}",
                    effect_type,
                    description
                );
            }
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
        "[GET_SEL] cards.len={} filter.nh_color={:?} filter.nh_total={:?} ct={:?} group={:?} groups={:?} chars={:?} excl_chars={:?} cost_lim={:?} cost_op={:?} cost_vals={:?} cost_min={:?} cost_max={:?} excl_self={:?} names={:?} hearts={:?} nhc_count={:?} distinct={:?} excl_groups={:?} excl_cards={:?}",
        cards.len(),
        filter.need_heart_color,
        filter.need_heart_total,
        filter.card_type,
        filter.group,
        filter.groups,
        filter.characters,
        filter.exclude_characters,
        filter.cost_limit,
        filter.cost_operator,
        filter.cost_values,
        filter.cost_limit_min,
        filter.cost_limit_max,
        filter.exclude_self,
        filter.name_fragments,
        filter.heart_colors,
        filter.heart_color_count,
        filter.distinct,
        filter.exclude_group_names,
        filter.exclude_cards,
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
