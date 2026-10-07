use super::ConditionContext;
use crate::ability::enums::Zone;
use crate::ability::util::{self, compare_counts};
use crate::card::{Condition, HeartColor};
use crate::core::constants::CountCast;
use crate::HashSet;
use smallvec::SmallVec;
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

impl<'a> ConditionContext<'a> {
    pub(crate) fn resolve_condition_player(&self, target: &str) -> &crate::player::Player {
        if target == "self" {
            // Use cached player if available
            if let Some(p) = self.self_player {
                return p;
            }
            self.activating_card_id
                .and_then(|cid| self.game_state.owner_of_card(cid))
                .unwrap_or_else(|| self.game_state.resolve_target_player(target))
        } else if target == "both" {
            // For "both" scope in comparison conditions (comparing self vs opponent),
            // resolve to the activating card's owner (self). The comparison_target
            // field handles the opponent side of the comparison.
            self.self_player
                .unwrap_or_else(|| self.game_state.resolve_target_player("self"))
        } else {
            self.game_state.resolve_target_player(target)
        }
    }

    pub(crate) fn resolve_target_for_scope<'b>(&self, condition: &'b Condition) -> &'b str {
        let target = condition.get_target().unwrap_or("self");
        if target == "self" && condition.get_scope() == Some("both") {
            "both"
        } else {
            target
        }
    }

    /// `check_self` conditions (parser emits them for "このカードが控え室にある"
    /// style gates) test whether the ACTIVATING card itself is present in the
    /// location, instead of counting matching cards in that zone.
    /// Returns None when the condition is not a check_self condition or lacks
    /// a resolvable location, so callers fall through to normal evaluation.
    /// Negation is NOT applied here — callers own negation semantics.
    pub(crate) fn evaluate_check_self_condition(&self, condition: &Condition) -> Option<bool> {
        if !condition.get_check_self().unwrap_or(false) {
            return None;
        }
        let cid = self.activating_card_id?;
        let location = condition.get_location()?;
        if location.is_empty() {
            return None;
        }
        let present = match self.resolve_target_for_scope(condition) {
            "both" => {
                util::zone_cards(&self.game_state.player1, location).contains(&cid)
                    || util::zone_cards(&self.game_state.player2, location).contains(&cid)
            }
            "opponent" => {
                util::zone_cards(self.resolve_condition_player("opponent"), location).contains(&cid)
            }
            _ => util::zone_cards(self.resolve_condition_player("self"), location).contains(&cid),
        };
        Some(compare_counts(
            condition.get_operator(),
            if present { 1 } else { 0 },
            condition.get_count().unwrap_or(1),
        ))
    }

    /// The cards in `ids` that satisfy the condition's card-type and group
    /// filters, paired with their card.
    ///
    /// Three arms of `check_aggregate_total` need exactly this eligibility, and
    /// a change to what counts as eligible then has one place to change rather
    /// than three. An empty `card_type` and an absent group mean "no filter",
    /// which is why the two tests are written as disjunctions rather than as
    /// option checks.
    pub(crate) fn eligible_cards<'s>(
        &'s self,
        ids: &[i16],
        card_type: &str,
        group_name: Option<&str>,
    ) -> Vec<(i16, &'s crate::card::Card)> {
        let card_db = &self.game_state.card_database;
        ids.iter()
            .copied()
            .filter(|&cid| {
                card_type.is_empty() || util::card_matches_type(card_db, cid, Some(card_type))
            })
            .filter(|&cid| {
                group_name.is_none() || util::card_matches_group_str(card_db, cid, group_name)
            })
            .filter_map(|cid| card_db.get_card(cid).map(|card| (cid, card)))
            .collect()
    }

    /// 「自分の」/「相手の」 scope for the GAME-level revealed pool.
    ///
    /// The pool holds BOTH performers' yell reveals at
    /// LiveVictoryDetermination — nothing clears it between the two
    /// performances (`clear_revealed_cards` runs only after LVD,
    /// phases.rs:313) — and each entry is tagged with its performer's seat in
    /// `revealed_card_meta.owner` (phases.rs:437-439). An explicit
    /// `target: self|opponent` keeps only that seat's entries, so
    /// 「エールによって公開された自分のカードの中にライブカードがある場合」
    /// (`PL!S-PR-046-PR` ab#0 and the 19 other cards with this clause shape)
    /// cannot be satisfied by the opponent's reveal. Entries pushed without
    /// meta (test fixtures, effect reveals with no known owner) stay visible
    /// to either side, and a clause with no explicit target keeps the
    /// historical unscoped read.
    pub(crate) fn revealed_cards_scoped(&self, condition: &Condition) -> Vec<i16> {
        self.revealed_cards_scoped_to(condition.get_target())
    }

    /// As [`Self::revealed_cards_scoped`], for call sites whose target comes
    /// from an effect sub-condition rather than the `Condition` itself
    /// (`evaluate_complex_condition`'s `effect.target_name()`).
    pub(crate) fn revealed_cards_scoped_to(&self, target: Option<&str>) -> Vec<i16> {
        let gs = &self.game_state;
        let wanted_seat = match target {
            Some(target @ ("self" | "opponent")) => {
                let player = self.resolve_condition_player(target);
                Some(gs.seat_index_by_id(&player.id))
            }
            _ => None,
        };
        gs.revealed_cards
            .iter()
            .enumerate()
            .filter(|(index, _)| {
                match (
                    wanted_seat,
                    gs.revealed_card_meta.get(*index).and_then(|m| m.owner),
                ) {
                    (Some(want), Some(owner)) => owner == want,
                    _ => true,
                }
            })
            .map(|(_, &cid)| cid)
            .collect()
    }

    /// Card's effective cost after constant cost modifiers, clamped at 0.
    /// `None` when the card id is unknown to the database.
    pub(crate) fn modified_cost(&self, cid: i16) -> Option<u8> {
        let card = self.game_state.card_database.get_card(cid)?;
        let cost = card.cost.unwrap_or(0);
        Some(crate::constants::effective_stat(
            cost,
            self.game_state.mods.get_cost_modifier(cid),
        ))
    }

    /// Count DISTINCT heart-color types present across `cards`, gated on
    /// `is_blade` (Q271: blade-hearts are NOT heart colors — a base-heart count
    /// ignores blade-hearts, and a blade-heart count ignores base hearts).
    /// Shared by the RevealedCards and Stage "types" branches of
    /// `resolve_zone_card_count` (previously copy-pasted, one copy hiding the bug).
    pub(crate) fn count_distinct_cost(&self, cards: &[i16], card_type: Option<&str>) -> usize {
        let card_db = &self.game_state.card_database;
        let mut seen: HashSet<u8> = HashSet::default();
        for &cid in cards {
            if cid == -1 {
                continue;
            }
            if let Some(ct) = card_type {
                if !util::card_matches_type(card_db, cid, Some(ct)) {
                    continue;
                }
            }
            if let Some(modified) = self.modified_cost(cid) {
                seen.insert(modified);
            }
        }
        seen.len()
    }

    pub(crate) fn count_distinct_heart_types(
        &self,
        cards: &[i16],
        required_colors: &[crate::card::HeartColor],
        is_blade: bool,
        group_name: Option<&str>,
        card_type: Option<&str>,
    ) -> usize {
        let card_db = &self.game_state.card_database;
        let mut present = HashSet::<HeartColor>::default();
        for &cid in cards {
            if cid == -1 {
                continue;
            }
            if let Some(gn) = group_name {
                if !util::card_matches_group_str(card_db, cid, Some(gn)) {
                    continue;
                }
            }
            if let Some(ct) = card_type {
                if !util::card_matches_type(card_db, cid, Some(ct)) {
                    continue;
                }
            }
            let Some(card) = card_db.get_card(cid) else {
                continue;
            };
            // A blade condition counts the printed blade heart; a plain one
            // counts the base heart. Exactly one of the two is ever read.
            let printed: Vec<HeartColor> = if is_blade {
                card.blade_heart
                    .iter()
                    .flat_map(|b| b.hearts.keys().copied())
                    .collect()
            } else {
                card.base_heart
                    .iter()
                    .flat_map(|b| b.hearts.keys().copied())
                    .collect()
            };
            present.extend(printed.into_iter().filter(|c| required_colors.contains(c)));
        }
        present.len()
    }

    pub(crate) fn zone_len(&self, player: &crate::player::Player, location: &str) -> u8 {
        match Zone::from_str(location) {
            Some(Zone::Stage) => player.stage.total_blades(
                &self.game_state.card_database,
                &self.game_state.mods.blade_modifiers,
                &self.game_state.mods.orientation_modifiers,
                true,
            ),
            Some(Zone::Hand) => player.hand.len().u8_count(),
            Some(Zone::Deck) => player.main_deck.len().u8_count(),
            Some(Zone::Discard) => player.waitroom.len().u8_count(),
            Some(Zone::Energy) => player.energy_zone.cards.len().u8_count(),
            Some(Zone::LiveCardZone) => player.live_card_zone.len().u8_count(),
            Some(Zone::SuccessLiveZone) => player.success_live_card_zone.len().u8_count(),
            Some(Zone::RevealedCards) => {
                // Game-level pool, deliberately UNscoped here: `zone_len`
                // receives the already-resolved player, not the condition, so
                // the 「自分の」 owner tag cannot be consulted. Every corpus
                // shape that reaches this arm carries no explicit
                // `target` (which `revealed_cards_scoped` would leave
                // unscoped anyway); an explicit-target count takes
                // `resolve_zone_card_count` / `evaluate_location_condition`
                // instead, which do scope. Revisit if a printed card ever
                // counts the revealed pool per seat through this path.
                log::debug!(
                    "[ZONE_LEN] revealed_cards raw len={} (unscoped)",
                    self.game_state.revealed_cards.len()
                );
                self.game_state.revealed_cards.len().u8_count()
            }
            _ => 0,
        }
    }

    /// The cards a PRECEDING sequential step moved.
    ///
    /// Prefers this entry's own `moved_cards`, then the entry-time snapshot,
    /// then the global record. The fallback order matters for cross-player
    /// watchers (「対戦相手のカードの効果でも発動する。」): the enqueuing player's
    /// batch loop clears the global lists before the other seat's queue
    /// resolves, so for those the snapshot is the only surviving evidence.
    pub(crate) fn moved_card_pool(&self) -> SmallVec<[i16; 8]> {
        if !self.moved_cards.is_empty() {
            return self.moved_cards.iter().copied().collect();
        }
        let enqueued = self.game_state.entry_trigger_moved_cards();
        let global = self.game_state.recently_moved_cards.clone();
        match (&enqueued, &global) {
            (Some(ev), None) if !ev.is_empty() => ev.iter().copied().collect(),
            _ => global.map_or_else(SmallVec::new, |g| g.iter().copied().collect()),
        }
    }

    /// Hearts of one colour on the stage, restricted to a named slot.
    ///
    /// A named position counts only the card in that slot, and an EMPTY slot
    /// counts 0 rather than falling through to the whole stage — "the centre
    /// has no card" must not read as "the stage totals N". Anything else (no
    /// position, or a name that is not a slot) counts every member, which is
    /// what the printed clause 「ステージの」 means.
    pub(crate) fn heart_count_at_or_all(
        player: &crate::player::Player,
        card_db: &crate::card::CardDatabase,
        color: crate::card::HeartColor,
        position: Option<&str>,
    ) -> u8 {
        let of = |cid: i16| {
            card_db
                .get_card(cid)
                .and_then(|c| c.base_heart.as_ref())
                .map(|bh| bh.hearts.get(&color).copied().unwrap_or(0))
                .unwrap_or(0)
        };
        match position.and_then(util::stage_position_index) {
            Some(idx) => {
                let cid = player.stage.stage[idx];
                if cid == -1 {
                    0
                } else {
                    of(cid)
                }
            }
            None => player
                .stage
                .stage
                .iter()
                .filter(|&&id| id != -1)
                .map(|&id| of(id))
                .sum(),
        }
    }

}
