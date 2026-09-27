use crate::card::{BaseHeart, CardDatabase, HeartColor};
use crate::core::constants::U8Count;
use crate::core::game_modifiers::ModifierEntry;
use crate::{HashMap, HashSet};
#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

/// Warn once per call site instead of on every draw. An empty deck is a legal
/// state that one effect can hit repeatedly; warning per call buried the signal
/// under ~1.2k identical lines in a full `cargo test` run.
#[cfg(target_has_atomic = "ptr")]
mod empty_deck_warn {
    use core::sync::atomic::{AtomicBool, Ordering};
    pub struct OnceFlag(AtomicBool);
    impl OnceFlag {
        pub const fn new() -> Self {
            OnceFlag(AtomicBool::new(false))
        }
        /// True on the first call only.
        pub fn first_call(&self) -> bool {
            !self.0.swap(true, Ordering::Relaxed)
        }
    }
}

/// No-ptr-atomic targets (PS1 R3000/MIPS-I): stay silent, matching the
/// compile-time-false `ABILITY_DEBUG` flag in `ability::debug`.
#[cfg(not(target_has_atomic = "ptr"))]
mod empty_deck_warn {
    pub struct OnceFlag;
    impl OnceFlag {
        pub const fn new() -> Self {
            OnceFlag
        }
        pub fn first_call(&self) -> bool {
            false
        }
    }
}

use empty_deck_warn::OnceFlag;

static MAIN_DECK_DRAW_WARNED: OnceFlag = OnceFlag::new();
static MAIN_DECK_DRAW_BOTTOM_WARNED: OnceFlag = OnceFlag::new();
static ENERGY_DECK_DRAW_WARNED: OnceFlag = OnceFlag::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum Orientation {
    Active,
    Wait,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde_support", serde(rename_all = "lowercase"))]
pub enum MemberArea {
    LeftSide,
    Center,
    RightSide,
}

impl MemberArea {
    /// All stage areas in slot order. ONE definition for the
    /// `[LeftSide, Center, RightSide]` literal previously copy-pasted at
    /// every per-area loop (game_setup estimates, phases double-baton).
    pub const ALL: [MemberArea; 3] = [
        MemberArea::LeftSide,
        MemberArea::Center,
        MemberArea::RightSide,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            MemberArea::LeftSide => "left",
            MemberArea::Center => "center",
            MemberArea::RightSide => "right",
        }
    }

    /// Returns the opposing player's front area for this area.
    /// Rule 4.5.7: Left side face opponent's right side, center faces center, right side faces opponent's left side.
    pub fn front_area(&self) -> MemberArea {
        match self {
            MemberArea::LeftSide => MemberArea::RightSide,
            MemberArea::Center => MemberArea::Center,
            MemberArea::RightSide => MemberArea::LeftSide,
        }
    }

    /// Stage slot index (0=left, 1=center, 2=right).
    pub fn to_index(&self) -> usize {
        match self {
            MemberArea::LeftSide => 0,
            MemberArea::Center => 1,
            MemberArea::RightSide => 2,
        }
    }

    /// Inverse of [`MemberArea::to_index`]. Returns None for unknown indices.
    pub fn from_index(idx: usize) -> Option<MemberArea> {
        match idx {
            0 => Some(MemberArea::LeftSide),
            1 => Some(MemberArea::Center),
            2 => Some(MemberArea::RightSide),
            _ => None,
        }
    }

    /// Wire tag used by the 3DS multiplayer protocol (1=left, 2=center, 3=right).
    pub fn to_tag(&self) -> u8 {
        match self {
            MemberArea::LeftSide => 1,
            MemberArea::Center => 2,
            MemberArea::RightSide => 3,
        }
    }

    /// Inverse of [`MemberArea::to_tag`]. Returns None for unknown tags.
    pub fn from_tag(tag: u8) -> Option<MemberArea> {
        match tag {
            1 => Some(MemberArea::LeftSide),
            2 => Some(MemberArea::Center),
            3 => Some(MemberArea::RightSide),
            _ => None,
        }
    }
}

impl core::fmt::Display for MemberArea {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            MemberArea::LeftSide => write!(f, "left"),
            MemberArea::Center => write!(f, "center"),
            MemberArea::RightSide => write!(f, "right"),
        }
    }
}

impl core::str::FromStr for MemberArea {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "left" => Ok(MemberArea::LeftSide),
            "center" => Ok(MemberArea::Center),
            "right" => Ok(MemberArea::RightSide),
            _ => Err(format!("Invalid area: {}", s)),
        }
    }
}

// Q143: Center symbol means the ability is only effective when the member is in the center area.
/// Check if a card at the given stage position can activate an ability
/// whose trigger string contains position requirements (左サイド/右サイド/センター).
pub fn check_trigger_position(triggers: Option<&str>, card_position: MemberArea) -> bool {
    let trig = match triggers {
        Some(t) => t,
        None => return true,
    };
    // Check each position requirement
    if trig.contains("左サイド") && card_position != MemberArea::LeftSide {
        return false;
    }
    if trig.contains("右サイド") && card_position != MemberArea::RightSide {
        return false;
    }
    if trig.contains("センター") && card_position != MemberArea::Center {
        return false;
    }
    true
}

/// Check if a card matches the required stage position from a parsed
/// `activation_position` field (e.g. "center", "left", "right", or comma-separated "left_side,right_side").
pub fn check_effect_position(effect_pos: Option<&str>, card_position: MemberArea) -> bool {
    let pos = match effect_pos {
        Some(p) => p,
        None => return true,
    };
    // Support comma-separated multiple positions (e.g. "left_side,right_side")
    if pos.contains(',') {
        return pos.split(',').any(|p| {
            let trimmed = p.trim();
            matches!(
                (trimmed, card_position),
                ("center", MemberArea::Center)
                    | ("left" | "left_side", MemberArea::LeftSide)
                    | ("right" | "right_side", MemberArea::RightSide)
            )
        });
    }
    match (pos, card_position) {
        ("center", MemberArea::Center) => true,
        ("left" | "left_side", MemberArea::LeftSide) => true,
        ("right" | "right_side", MemberArea::RightSide) => true,
        _ => {
            !(pos == "center"
                || pos == "left"
                || pos == "right"
                || pos == "left_side"
                || pos == "right_side")
        }
    }
}

// CardInZone removed for performance - use i16 IDs directly
use crate::constants::{EMPTY_SLOT, STAGE_SIZE};

// Orientation and other state tracked in GameState modifiers

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct Stage {
    // Rule 5.3: Stage - Where member cards are placed during Main Phase
    // Has three areas: Left Side, Center, Right Side
    // Use EMPTY_SLOT to indicate empty slot (like old engine)
    pub stage: [i16; STAGE_SIZE], // [left_side, center, right_side]
    // Rule 4.5.5: Cards (member or energy) placed under a member card
    // Index 0 = left side, 1 = center, 2 = right side
    pub under_cards: [SmallVec<[i16; 4]>; STAGE_SIZE],
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub pending_duplicate_members: [Vec<(i16, SmallVec<[i16; 4]>)>; STAGE_SIZE],
}

impl Default for Stage {
    fn default() -> Self {
        Self::new()
    }
}

impl Stage {
    pub fn new() -> Self {
        Stage {
            stage: [EMPTY_SLOT, EMPTY_SLOT, EMPTY_SLOT], // [left_side, center, right_side], EMPTY_SLOT indicates empty
            under_cards: [SmallVec::new(), SmallVec::new(), SmallVec::new()],
            pending_duplicate_members: [Vec::new(), Vec::new(), Vec::new()],
        }
    }

    /// Invariant check: stage must always have exactly STAGE_SIZE positions
    pub fn invariant(&self) -> bool {
        self.stage.len() == STAGE_SIZE
            && self.under_cards.len() == STAGE_SIZE
            && self.pending_duplicate_members.len() == STAGE_SIZE
    }

    pub fn get_area(&self, area: MemberArea) -> Option<i16> {
        debug_assert!(self.invariant(), "Stage invariant violated");
        let index = match area {
            MemberArea::LeftSide => 0,
            MemberArea::Center => 1,
            MemberArea::RightSide => 2,
        };
        let card_id = self.stage[index];
        if card_id == EMPTY_SLOT {
            None
        } else {
            Some(card_id)
        }
    }

    pub fn set_area(&mut self, area: MemberArea, card_id: i16) {
        debug_assert!(self.invariant(), "Stage invariant violated before set");
        let index = match area {
            MemberArea::LeftSide => 0,
            MemberArea::Center => 1,
            MemberArea::RightSide => 2,
        };
        self.stage[index] = card_id;
        debug_assert!(self.invariant(), "Stage invariant violated after set");
    }

    pub fn stage_member_for_check_timing(&mut self, area: MemberArea, card_id: i16) {
        let index = area.to_index();
        log::debug!(
            "[DUPLICATE_STAGE] area={} older={} newest={}",
            area,
            self.stage[index],
            card_id
        );
        if self.stage[index] != EMPTY_SLOT {
            let older = self.stage[index];
            let under_cards = core::mem::take(&mut self.under_cards[index]);
            self.pending_duplicate_members[index].push((older, under_cards));
        }
        self.stage[index] = card_id;
    }

    pub fn take_pending_duplicate_members(
        &mut self,
        area: MemberArea,
    ) -> Vec<(i16, SmallVec<[i16; 4]>)> {
        core::mem::take(&mut self.pending_duplicate_members[area.to_index()])
    }

    pub fn place_under_card(&mut self, area: MemberArea, card_id: i16) {
        let index = match area {
            MemberArea::LeftSide => 0,
            MemberArea::Center => 1,
            MemberArea::RightSide => 2,
        };
        self.under_cards[index].push(card_id);
    }

    /// Get all cards under the member at the given area.
    pub fn get_under_cards(&self, area: MemberArea) -> &[i16] {
        let index = match area {
            MemberArea::LeftSide => 0,
            MemberArea::Center => 1,
            MemberArea::RightSide => 2,
        };
        &self.under_cards[index]
    }

    /// Iterate (under_card, host_member) pairs for every card stacked beneath a
    /// member on this stage.
    pub fn under_cards_with_hosts(&self) -> Vec<(i16, i16)>
    where
        Self: Sized,
    {
        let mut out = Vec::new();
        for idx in 0..STAGE_SIZE {
            let host = self.stage[idx];
            if host == EMPTY_SLOT {
                continue;
            }
            for &uc in &self.under_cards[idx] {
                out.push((uc, host));
            }
        }
        out
    }

    // Q140/Q141: Energy under a member goes to energy deck when the member leaves stage.
    /// Rule 10.5.3-10.5.4: When a member leaves its area, recycle under-cards:
    /// - Member cards under → go to waitroom
    /// - Energy cards under → go to energy deck
    /// Returns (waitroom_cards, energy_deck_cards)
    pub fn recycle_under_cards(
        &mut self,
        area: MemberArea,
        card_db: &CardDatabase,
    ) -> (SmallVec<[i16; 4]>, SmallVec<[i16; 4]>) {
        let index = match area {
            MemberArea::LeftSide => 0,
            MemberArea::Center => 1,
            MemberArea::RightSide => 2,
        };
        let cards = core::mem::take(&mut self.under_cards[index]);
        let mut waitroom = SmallVec::new();
        let mut energy_deck = SmallVec::new();
        for card_id in cards {
            if card_db.get_card(card_id).is_some_and(|c| c.is_energy()) {
                energy_deck.push(card_id);
            } else {
                waitroom.push(card_id);
            }
        }
        (waitroom, energy_deck)
    }

    pub fn position_change(
        &mut self,
        from_area: MemberArea,
        to_area: MemberArea,
    ) -> Result<i16, String> {
        // Rule 11.10: Position Change - move member to different area
        // Rule 11.10.2: If destination has a member, it swaps positions
        // Rule 4.5.5.3: Under-cards move with the member
        if from_area == to_area {
            return Err("Cannot move to same area".to_string());
        }

        let from_index = match from_area {
            MemberArea::LeftSide => 0,
            MemberArea::Center => 1,
            MemberArea::RightSide => 2,
        };
        let to_index = match to_area {
            MemberArea::LeftSide => 0,
            MemberArea::Center => 1,
            MemberArea::RightSide => 2,
        };

        let card_id = self.stage[from_index];
        if card_id == -1 {
            return Err("No card in source area".to_string());
        }

        // Swap under-cards along with the members (Rule 4.5.5.3)
        let from_under = core::mem::take(&mut self.under_cards[from_index]);
        let to_under = core::mem::take(&mut self.under_cards[to_index]);
        self.under_cards[from_index] = to_under;
        self.under_cards[to_index] = from_under;

        let dest_card_id = self.stage[to_index];

        if dest_card_id != -1 {
            // Swap: move destination card to source
            self.stage[from_index] = dest_card_id;
            self.stage[to_index] = card_id;
        } else {
            // Move: place source card in destination
            self.stage[to_index] = card_id;
            self.stage[from_index] = -1;
        }

        Ok(card_id)
    }

    pub fn formation_change(
        &mut self,
        assignments: Vec<(MemberArea, MemberArea)>,
    ) -> Result<(), String> {
        // Rule 11.11: Formation Change - move all members to specified areas
        // Rule 11.11.2: Cannot move multiple members to same area
        let mut target_areas = HashSet::<&MemberArea>::default();
        for (_, target) in &assignments {
            if !target_areas.insert(target) {
                return Err("Cannot move multiple members to same area".to_string());
            }
        }

        for (from, to) in assignments.clone() {
            self.position_change(from, to)?;
        }

        Ok(())
    }

    // Q133: Weighed members' blades do NOT count toward yell reveal count.
    // Q134: Baton touch with a weighed member is allowed; the new member enters active.
    // Q136: A weighed member moving areas remains weighed.
    /// Q148: `include_waited` controls whether waited members count.
    /// - `false` for yell draws (Rule 9.9: only active members yell)
    /// - `true` for condition checks ("ステージにいるメンバーが持つブレードの合計"
    ///   includes waited members per Q148)
    pub fn total_blades(
        &self,
        card_db: &CardDatabase,
        blade_entries: &HashMap<i16, ModifierEntry>,
        orientation_modifiers: &HashMap<i16, crate::core::game_modifiers::CardOrientation>,
        include_waited: bool,
    ) -> u8 {
        let mut total = 0;
        for &card_id in &self.stage {
            if card_id != -1 {
                if !include_waited
                    && orientation_modifiers
                        .get(&card_id)
                        .map(|o| *o == crate::core::game_modifiers::CardOrientation::Wait)
                        .unwrap_or(false)
                    {
                        continue;
                    }
                if card_db.get_card(card_id).is_some() {
                    // A2: use unified effective_blade
                    let entry = blade_entries.get(&card_id).copied().unwrap_or_default();
                    total += crate::core::stats_pipeline::effective_blade(card_db, card_id, entry);
                    log::debug!(
                        "[BLADE_PIPELINE] card={} entry={:?} effective={}",
                        card_id,
                        entry,
                        crate::core::stats_pipeline::effective_blade(card_db, card_id, entry)
                    );
                }
            }
        }
        total
    }

    pub fn can_place_card(&self, card_db: &CardDatabase, card_id: i16) -> bool {
        // Rule 8.2.2: Only member cards can be placed on the stage
        // Live cards cannot be played on main stage
        if let Some(card) = card_db.get_card(card_id) {
            !card.is_live()
        } else {
            false
        }
    }

    pub fn get_available_hearts(
        &self,
        card_db: &CardDatabase,
        heart_override: &HashMap<i16, (HeartColor, u8)>,
        heart_modifiers: &HashMap<
            i16,
            HashMap<HeartColor, crate::core::game_modifiers::ModifierEntry>,
        >,
        heart_color_multiplier: &HashMap<i16, HeartColor>,
        heart_copy: &HashMap<i16, i16>,
    ) -> BaseHeart {
        // A1: single source of truth via stats_pipeline::stage_hearts
        crate::core::stats_pipeline::stage_hearts(
            &self.stage,
            card_db,
            heart_override,
            heart_copy,
            heart_color_multiplier,
            heart_modifiers,
        )
    }

    /// Legacy adapter for callers still holding the old i32-valued modifier map
    /// (e.g. transient test scaffolding). Converts to the canonical ModifierEntry
    /// form and delegates.
    pub fn get_available_hearts_i32(
        &self,
        card_db: &CardDatabase,
        heart_override: &HashMap<i16, (HeartColor, u8)>,
        heart_modifiers_i32: &HashMap<i16, HashMap<HeartColor, i32>>,
        heart_color_multiplier: &HashMap<i16, HeartColor>,
        heart_copy: &HashMap<i16, i16>,
    ) -> BaseHeart {
        let converted: HashMap<
            i16,
            HashMap<HeartColor, crate::core::game_modifiers::ModifierEntry>,
        > = heart_modifiers_i32
            .iter()
            .map(|(&cid, colors)| {
                let mut m = HashMap::default();
                for (&col, &delta) in colors {
                    let e = crate::core::game_modifiers::ModifierEntry {
                        additive: delta as i16,
                        ..Default::default()
                    };
                    m.insert(col, e);
                }
                (cid, m)
            })
            .collect();
        self.get_available_hearts(
            card_db,
            heart_override,
            &converted,
            heart_color_multiplier,
            heart_copy,
        )
    }
}

// Removed: use crate::card::parse_heart_color directly

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct LiveCardZone {
    // Rule 5.2: Live Card Zone - Where member and live cards are placed during Live Card Set Phase
    pub cards: SmallVec<[i16; MAX_LIVE_CARDS]>, // Card IDs - stack-allocated for up to MAX_LIVE_CARDS cards
    pub face_up: bool,
}

impl Default for LiveCardZone {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveCardZone {
    pub fn new() -> Self {
        LiveCardZone {
            cards: SmallVec::new(),
            face_up: false,
        }
    }

    pub fn can_place_card(&self, _card_db: &CardDatabase, _card_id: i16) -> bool {
        // Rule 8.2: During Live Card Set Phase, any card from hand can be placed in Live Card Zone
        true
    }

    pub fn add_card(&mut self, card_id: i16, card_db: &CardDatabase) -> Result<(), String> {
        if !self.can_place_card(card_db, card_id) {
            if let Some(card) = card_db.get_card(card_id) {
                return Err(format!(
                    "Cannot place energy card '{}' in live card zone",
                    card.name
                ));
            }
            return Err("Cannot place unknown card in live card zone".to_string());
        }
        self.cards.push(card_id);
        Ok(())
    }

    pub fn clear(&mut self) -> SmallVec<[i16; 3]> {
        core::mem::take(&mut self.cards)
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn calculate_live_score(
        &self,
        card_db: &CardDatabase,
        cheer_blade_heart_count: u8,
        stage_hearts: Option<&crate::card::BaseHeart>,
        need_heart_modifiers: Option<
            &HashMap<i16, HashMap<crate::card::HeartColor, ModifierEntry>>,
        >,
        score_modifiers: Option<&HashMap<i16, i32>>,
        constant_total_score_bonus: i16,
    ) -> u8 {
        let mut total_score = 0;

        for card_id in &self.cards {
            if let Some(card) = card_db.get_card(*card_id) {
                let base_score = card.get_score() as i32;
                let modifier = score_modifiers
                    .and_then(|sm| sm.get(card_id))
                    .copied()
                    .unwrap_or(0);
                let card_score = crate::constants::saturate_u8(base_score + modifier);

                // A4: use unified effective_need_heart + check_heart_requirement
                let heart_needs_satisfied = if let Some(ref need_heart) = card.need_heart {
                    if need_heart.hearts.is_empty() {
                        true
                    } else if let Some(sh) = stage_hearts {
                        let eff = if let Some(mods) = need_heart_modifiers {
                            crate::core::stats_pipeline::effective_need_heart(
                                Some(need_heart),
                                *card_id,
                                mods,
                            )
                        } else {
                            Some(need_heart.clone())
                        };
                        eff.as_ref()
                            .is_some_and(|need| crate::card::check_heart_requirement(need, sh))
                    } else {
                        false
                    }
                } else {
                    true
                };

                if heart_needs_satisfied {
                    total_score += card_score;
                }
            }
        }

        total_score
            + cheer_blade_heart_count
            + crate::constants::saturate_u8(constant_total_score_bonus as i32)
    }
}

use crate::constants::{MAX_ENERGY_CARDS, MAX_LIVE_CARDS};
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct EnergyZone {
    // Rule 5.1: Energy Zone - Where energy cards are placed and activated
    // Q15: Energy deck cards are face-down; energy zone cards are face-up.
    //
    // POSITIONAL CONVENTION (canonical for counts): cards[0..active_energy_count]
    // are active, the rest waited. Every mutation below preserves it — state
    // changes swap cards across the boundary instead of leaving
    // active/waited cards intermixed. Menus and filters rely on
    // util::{active,waited}_energy_indices, which encode the same rule.
    pub cards: SmallVec<[i16; MAX_ENERGY_CARDS]>,
    pub(crate) active_energy_count: u8,
}

impl Default for EnergyZone {
    fn default() -> Self {
        Self::new()
    }
}

impl EnergyZone {
    pub fn new() -> Self {
        EnergyZone {
            cards: SmallVec::new(),
            active_energy_count: 0,
        }
    }

    pub fn can_place_card(&self, card_db: &CardDatabase, card_id: i16) -> bool {
        // Rule 7.2: Only energy cards can be placed in Energy Zone
        card_db
            .get_card(card_id)
            .map(|c| c.is_energy())
            .unwrap_or_else(|| false)
    }

    pub fn add_card(&mut self, card_id: i16, card_db: &CardDatabase) -> Result<(), String> {
        // Rule 7.2: Only energy cards can be placed in Energy Zone
        if !card_db
            .get_card(card_id)
            .map(|c| c.is_energy())
            .unwrap_or_else(|| false)
        {
            return Err("Only energy cards can be placed in Energy Zone".to_string());
        }

        // New energy cards start in Active state (Rule 7.4)
        self.cards.push(card_id);
        self.active_energy_count += 1;
        Ok(())
    }

    pub fn active_count(&self) -> u8 {
        self.active_energy_count
    }

    pub fn set_active_count(&mut self, count: u8) {
        self.active_energy_count = count;
    }

    pub fn add_active(&mut self, delta: u8) {
        self.active_energy_count = self.active_energy_count.saturating_add(delta);
    }

    pub fn sub_active(&mut self, delta: u8) {
        self.active_energy_count = self.active_energy_count.saturating_sub(delta);
    }

    pub fn pay_energy(&mut self, amount: u8) -> Result<(), String> {
        // Rule 5.9: Pay energy by decrementing active count
        // log::debug!("pay_energy called: amount={}, active_energy_count={}", amount, self.active_energy_count);

        if self.active_energy_count >= amount {
            self.active_energy_count -= amount;
            // log::debug!("pay_energy result: success, remaining active_energy_count={}", self.active_energy_count);
            Ok(())
        } else {
            // log::debug!("pay_energy result: failed, active_energy_count={}", self.active_energy_count);
            Err(format!(
                "Could not pay {} energy (only {} active energy available, {} total energy cards)",
                amount,
                self.active_energy_count,
                self.cards.len()
            ))
        }
    }

    pub fn activate_all(&mut self) {
        // Set all energy cards to active state
        self.active_energy_count = self.cards.len().u8_count();
        // log::debug!("Activated {} energy cards (active_energy_count={})", self.cards.len(), self.active_energy_count);
    }

    /// Push an ACTIVE card while preserving the positional convention:
    /// inserted at the active/wait boundary (not appended — appending an
    /// active card past waited ones would corrupt the prefix).
    pub fn push_active(&mut self, card_id: i16) {
        let at = (self.active_energy_count as usize).min(self.cards.len());
        self.cards.insert(at, card_id);
        self.active_energy_count = self.active_energy_count.saturating_add(1);
    }

    /// Push a WAITED card: appended past the active prefix (no count change).
    pub fn push_waited(&mut self, card_id: i16) {
        self.cards.push(card_id);
    }

    /// Mark the card at `index` waited, preserving the convention by swapping
    /// it with the last active card. No-op if already waited or out of range.
    /// Returns true when a state change happened.
    /// NOTE: prefer the batch `set_indices_*` below for multi-index changes —
    /// per-index swaps re-point later indices, which alias under duplicate ids.
    pub fn mark_waited(&mut self, index: usize) -> bool {
        let active = self.active_energy_count as usize;
        if index >= active || index >= self.cards.len() {
            return false;
        }
        let last_active = active - 1;
        self.cards.swap(index, last_active);
        self.active_energy_count = self.active_energy_count.saturating_sub(1);
        true
    }

    /// Set exactly the given INDICES to waited, preserving relative order:
    /// rebuilds as [still-active in order] ++ [rest in order] and recounts.
    /// Fully positional — duplicate ids are harmless. Indices out of range
    /// are ignored; already-waited indices are no-ops.
    pub fn set_indices_waited(&mut self, indices: &[usize]) {
        let active = self.active_energy_count as usize;
        let mut still_active = Vec::with_capacity(self.cards.len());
        let mut rest = Vec::with_capacity(self.cards.len());
        for (i, &cid) in self.cards.iter().enumerate() {
            if indices.contains(&i) || i >= active {
                rest.push(cid);
            } else {
                still_active.push(cid);
            }
        }
        let new_active = still_active.len();
        still_active.extend(rest);
        self.cards = still_active.into_iter().collect();
        self.active_energy_count = new_active.u8_count();
    }

    /// Set exactly the given INDICES to active (mirror of `set_indices_waited`):
    /// rebuilds as [newly + still-active in order] ++ [rest in order].
    pub fn set_indices_active(&mut self, indices: &[usize]) {
        let active = self.active_energy_count as usize;
        let mut now_active = Vec::with_capacity(self.cards.len());
        let mut rest = Vec::with_capacity(self.cards.len());
        for (i, &cid) in self.cards.iter().enumerate() {
            if indices.contains(&i) || i < active {
                now_active.push(cid);
            } else {
                rest.push(cid);
            }
        }
        let new_active = now_active.len();
        now_active.extend(rest);
        self.cards = now_active.into_iter().collect();
        self.active_energy_count = new_active.u8_count();
    }

    /// Mark the card at `index` active, preserving the convention by swapping
    /// it with the first waited card. No-op if already active or out of range.
    /// Returns true when a state change happened.
    pub fn mark_active(&mut self, index: usize) -> bool {
        let active = self.active_energy_count as usize;
        if index < active || index >= self.cards.len() {
            return false;
        }
        self.cards.swap(index, active);
        self.active_energy_count = self.active_energy_count.saturating_add(1);
        true
    }

    /// Remove and return the card at `index`, adjusting the counter only when
    /// an actually-active card leaves (positional check BEFORE removal).
    /// Returns None when out of range.
    pub fn remove_at(&mut self, index: usize) -> Option<i16> {
        if index >= self.cards.len() {
            return None;
        }
        if index < self.active_energy_count as usize {
            self.active_energy_count = self.active_energy_count.saturating_sub(1);
        }
        Some(self.cards.remove(index))
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct MainDeck {
    /// Card IDs. **Index 0 = top of deck.** Drawing/peeking reads from index 0.
    /// Pushing to the end (`cards.push()`) adds to the bottom.
    /// To put a card on top, use `cards.insert(0, id)`.
    pub cards: SmallVec<[i16; 64]>,
}

impl Default for MainDeck {
    fn default() -> Self {
        Self::new()
    }
}

impl MainDeck {
    pub fn new() -> Self {
        MainDeck {
            cards: SmallVec::new(),
        }
    }

    pub fn shuffle(&mut self) {
        crate::rng::shuffle_slice(&mut self.cards);
    }

    /// Draw the top card (index 0). Returns None if deck is empty.
    pub fn draw(&mut self) -> Option<i16> {
        if self.cards.is_empty() {
            if MAIN_DECK_DRAW_WARNED.first_call() {
                log::warn!("[EMPTY_DECK_DRAW] main_deck.draw() on EMPTY deck — effect silently does nothing; test setups must stock main_deck");
            }
            None
        } else {
            Some(self.cards.remove(0))
        }
    }

    /// Draw the bottom card (last index). Returns None if deck is empty.
    pub fn draw_bottom(&mut self) -> Option<i16> {
        if self.cards.is_empty() {
            if MAIN_DECK_DRAW_BOTTOM_WARNED.first_call() {
                log::warn!("[EMPTY_DECK_DRAW] main_deck.draw_bottom() on EMPTY deck — effect silently does nothing; test setups must stock main_deck");
            }
            None
        } else {
            self.cards.pop()
        }
    }

    pub fn draw_multiple(&mut self, count: usize) -> Vec<i16> {
        (0..count).filter_map(|_| self.draw()).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct EnergyDeck {
    pub cards: SmallVec<[i16; 20]>,
}

impl Default for EnergyDeck {
    fn default() -> Self {
        Self::new()
    }
}

impl EnergyDeck {
    pub fn new() -> Self {
        EnergyDeck {
            cards: SmallVec::new(),
        }
    }

    pub fn draw(&mut self) -> Option<i16> {
        if self.cards.is_empty() {
            if ENERGY_DECK_DRAW_WARNED.first_call() {
                log::warn!("[EMPTY_DECK_DRAW] energy_deck.draw() on EMPTY deck — effect silently does nothing; test setups must stock energy_deck (give_energy fills the ZONE, not the deck)");
            }
            None
        } else {
            Some(self.cards.remove(0))
        }
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct Hand {
    // Rule 5.4: Hand - Where cards drawn from main deck are held
    pub cards: SmallVec<[i16; 7]>, // Card IDs - stack-allocated for up to 7 cards
}

impl Default for Hand {
    fn default() -> Self {
        Self::new()
    }
}

impl Hand {
    pub fn new() -> Self {
        Hand {
            cards: SmallVec::new(),
        }
    }

    pub fn add_card(&mut self, card_id: i16) {
        self.cards.push(card_id);
    }

    pub fn remove_card(&mut self, index: usize) -> Option<i16> {
        if index < self.cards.len() {
            Some(self.cards.remove(index))
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct Waitroom {
    // Rule 5.5: Waitroom - Where used cards are placed
    // Used for refresh when main deck is empty
    pub cards: SmallVec<[i16; 30]>, // Card IDs - stack-allocated for typical sizes
}

impl Default for Waitroom {
    fn default() -> Self {
        Self::new()
    }
}

impl Waitroom {
    pub fn new() -> Self {
        Waitroom {
            cards: SmallVec::new(),
        }
    }

    pub fn add_card(&mut self, card_id: i16) {
        self.cards.push(card_id);
    }

    pub fn take_all(&mut self) -> SmallVec<[i16; 30]> {
        core::mem::take(&mut self.cards)
    }

    pub fn shuffle(&mut self) {
        crate::rng::shuffle_slice(&mut self.cards);
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn remove_card(&mut self, card_id: i16) {
        self.cards.retain(|c| *c != card_id);
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct SuccessLiveCardZone {
    // Rule 5.6: Success Live Card Zone - Where won live cards are placed
    // Victory condition: 3 cards in this zone
    pub cards: SmallVec<[i16; 3]>, // Card IDs - stack-allocated for victory condition (max 3)
}

impl Default for SuccessLiveCardZone {
    fn default() -> Self {
        Self::new()
    }
}

impl SuccessLiveCardZone {
    pub fn new() -> Self {
        SuccessLiveCardZone {
            cards: SmallVec::new(),
        }
    }

    pub fn add_card(&mut self, card_id: i16) {
        self.cards.push(card_id);
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct ExclusionZone {
    // Rule 5.7: Exclusion Zone - Where excluded cards are placed
    pub cards: SmallVec<[i16; 10]>, // Card IDs - stack-allocated for up to 10 cards
}

impl Default for ExclusionZone {
    fn default() -> Self {
        Self::new()
    }
}

impl ExclusionZone {
    pub fn new() -> Self {
        ExclusionZone {
            cards: SmallVec::new(),
        }
    }

    pub fn add_card(&mut self, card_id: i16, _face_up: bool) {
        // Face state tracking moved to GameState modifiers
        self.cards.push(card_id);
    }
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct ResolutionZone {
    // Rule 5.8: Resolution Zone - Temporary holding area for cards being resolved
    pub cards: SmallVec<[i16; 10]>, // Card IDs - stack-allocated for up to 10 cards
    pub owners: SmallVec<[u8; 10]>,
}

impl ResolutionZone {
    pub fn new() -> Self {
        ResolutionZone {
            cards: SmallVec::new(),
            owners: SmallVec::new(),
        }
    }

    pub fn add_card(&mut self, card_id: i16) {
        self.add_card_for_owner(card_id, 0);
    }

    pub fn add_card_for_owner(&mut self, card_id: i16, owner: u8) {
        self.cards.push(card_id);
        self.owners.push(owner);
    }

    pub fn owner_at(&self, index: usize) -> Option<u8> {
        self.owners.get(index).copied()
    }

    pub fn swap_slots(&mut self, left_slot: usize, right_slot: usize) -> Result<(), String> {
        if left_slot == right_slot {
            return Err("Cannot swap the same Resolution Zone slot".to_string());
        }
        self.cards
            .get(left_slot)
            .ok_or_else(|| "Left Resolution Zone slot is out of bounds".to_string())?;
        self.cards
            .get(right_slot)
            .ok_or_else(|| "Right Resolution Zone slot is out of bounds".to_string())?;
        self.owners
            .get(left_slot)
            .ok_or_else(|| "Left Resolution Zone slot has no owner".to_string())?;
        self.owners
            .get(right_slot)
            .ok_or_else(|| "Right Resolution Zone slot has no owner".to_string())?;

        self.cards.swap(left_slot, right_slot);
        self.owners.swap(left_slot, right_slot);
        Ok(())
    }

    pub fn clear(&mut self) -> SmallVec<[i16; 10]> {
        self.owners.clear();
        core::mem::take(&mut self.cards)
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }
}
