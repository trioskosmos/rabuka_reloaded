use crate::card::{BaseHeart, CardDatabase, HeartColor, HeartIcon, Keyword};
use crate::core::game_modifiers::ModifierEntry;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Orientation {
    Active,
    Wait,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemberArea {
    LeftSide,
    Center,
    RightSide,
}

impl MemberArea {
    /// Returns the opposing player's front area for this area.
    /// Rule 4.5.7: Left side face opponent's right side, center faces center, right side faces opponent's left side.
    pub fn front_area(&self) -> MemberArea {
        match self {
            MemberArea::LeftSide => MemberArea::RightSide,
            MemberArea::Center => MemberArea::Center,
            MemberArea::RightSide => MemberArea::LeftSide,
        }
    }
}

impl std::fmt::Display for MemberArea {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MemberArea::LeftSide => write!(f, "left"),
            MemberArea::Center => write!(f, "center"),
            MemberArea::RightSide => write!(f, "right"),
        }
    }
}

impl std::str::FromStr for MemberArea {
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
                ("center" | "中央", MemberArea::Center)
                    | ("left" | "左" | "左側" | "left_side", MemberArea::LeftSide)
                    | (
                        "right" | "右" | "右側" | "right_side",
                        MemberArea::RightSide
                    )
            )
        });
    }
    match (pos, card_position) {
        ("center" | "中央", MemberArea::Center) => true,
        ("left" | "左" | "左側" | "left_side", MemberArea::LeftSide) => true,
        ("right" | "右" | "右側" | "right_side", MemberArea::RightSide) => true,
        _ => {
            !(pos == "center"
                || pos == "left"
                || pos == "right"
                || pos == "左"
                || pos == "右"
                || pos == "中央"
                || pos == "左側"
                || pos == "右側"
                || pos == "left_side"
                || pos == "right_side")
        }
    }
}

// CardInZone removed for performance - use i16 IDs directly
use crate::constants::{EMPTY_SLOT, STAGE_SIZE};

// Orientation and other state tracked in GameState modifiers

#[derive(Debug, Clone)]
pub struct Stage {
    // Rule 5.3: Stage - Where member cards are placed during Main Phase
    // Has three areas: Left Side, Center, Right Side
    // Use EMPTY_SLOT to indicate empty slot (like old engine)
    pub stage: [i16; STAGE_SIZE], // [left_side, center, right_side]
    // Rule 4.5.5: Cards (member or energy) placed under a member card
    // Index 0 = left side, 1 = center, 2 = right side
    pub under_cards: [SmallVec<[i16; 4]>; STAGE_SIZE],
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
        }
    }

    /// Invariant check: stage must always have exactly STAGE_SIZE positions
    pub fn invariant(&self) -> bool {
        self.stage.len() == STAGE_SIZE && self.under_cards.len() == STAGE_SIZE
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

    // Q138: Energy under a member cannot be used to pay costs
    // (under-cards have no active/wait state).
    // Q139: Energy under a member moves with them when changing areas on stage.
    // Q140: When member with energy underneath moves to waitroom/hand,
    // the energy goes to the energy deck.
    // Q141: When baton-touching with a member that has energy underneath,
    // the energy goes to the energy deck.
    /// Place a card (energy or member) under the member at the given area.
    /// Rule 4.5.5: Cards can be stacked beneath member cards.
    /// Swap the contents (member card + under-cards) of two stage slots by index.
    /// Rule 4.5.5.3: Under-cards move with the member when changing areas.
    pub fn swap_stage_slots(&mut self, from_idx: usize, to_idx: usize) {
        if from_idx >= STAGE_SIZE || to_idx >= STAGE_SIZE || from_idx == to_idx {
            return;
        }
        self.stage.swap(from_idx, to_idx);
        self.under_cards.swap(from_idx, to_idx);
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
        let cards = std::mem::take(&mut self.under_cards[index]);
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

    // Q137: A member already weighed cannot be "weighed" again as a cost
    // (weigh means changing from active to weighed state).
    pub fn clear_area(&mut self, area: MemberArea) {
        debug_assert!(self.invariant(), "Stage invariant violated before clear");
        let index = match area {
            MemberArea::LeftSide => 0,
            MemberArea::Center => 1,
            MemberArea::RightSide => 2,
        };
        self.stage[index] = EMPTY_SLOT;
        debug_assert!(self.invariant(), "Stage invariant violated after clear");
    }

    pub fn member_in_position(&self, position: Keyword) -> bool {
        // Check if a member is in the specified position (Center, LeftSide, RightSide)
        let index = match position {
            Keyword::Center => 1,
            Keyword::LeftSide => 0,
            Keyword::RightSide => 2,
            _ => return false,
        };
        self.stage[index] != -1
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
        let from_under = std::mem::take(&mut self.under_cards[from_index]);
        let to_under = std::mem::take(&mut self.under_cards[to_index]);
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
        let mut target_areas = std::collections::HashSet::new();
        for (_, target) in &assignments {
            if !target_areas.insert(target) {
                return Err("Cannot move multiple members to same area".to_string());
            }
        }

        for (from, to) in assignments {
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
        orientation_modifiers: &HashMap<i16, String>,
        include_waited: bool,
    ) -> u32 {
        let mut total = 0;
        for &card_id in &self.stage {
            if card_id != -1 {
                if !include_waited {
                    if orientation_modifiers
                        .get(&card_id)
                        .map(|o| o == "wait")
                        .unwrap_or(false)
                    {
                        continue;
                    }
                }
                if let Some(card) = card_db.get_card(card_id) {
                    let entry = blade_entries.get(&card_id).copied().unwrap_or_default();
                    if entry.set != 0 {
                        total += entry.total().max(0) as u32;
                    } else {
                        total += (card.blade as i32 + entry.total()).max(0) as u32;
                    }
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

    pub fn all_heart_icons(&self, card_db: &CardDatabase) -> Vec<HeartIcon> {
        let mut hearts = Vec::new();
        for &card_id in &self.stage {
            if card_id != -1 {
                if let Some(card) = card_db.get_card(card_id) {
                    if let Some(ref base_heart) = card.base_heart {
                        for (color, count) in &base_heart.hearts {
                            hearts.push(HeartIcon {
                                color: *color,
                                count: *count,
                            });
                        }
                    }
                }
            }
        }
        hearts
    }

    pub fn get_available_hearts(
        &self,
        card_db: &CardDatabase,
        heart_override: &HashMap<i16, (HeartColor, u32)>,
        heart_modifiers: &HashMap<i16, HashMap<HeartColor, i32>>,
        heart_color_multiplier: &HashMap<i16, HeartColor>,
    ) -> BaseHeart {
        let mut hearts = HashMap::new();

        for &card_id in &self.stage {
            if card_id == -1 {
                continue;
            }

            if let Some(&(override_color, override_count)) = heart_override.get(&card_id) {
                *hearts.entry(override_color).or_insert(0) += override_count;
                continue;
            }

            let mut card_hearts: HashMap<HeartColor, u32> = HashMap::new();
            if let Some(card) = card_db.get_card(card_id) {
                if let Some(ref base_heart) = card.base_heart {
                    for (color, count) in &base_heart.hearts {
                        *card_hearts.entry(*color).or_insert(0) += count;
                    }
                }
            }

            // Apply heart_color_multiplier: transform all this card's hearts to one color
            if let Some(override_color) = heart_color_multiplier.get(&card_id) {
                let total: u32 = card_hearts.values().sum();
                card_hearts.clear();
                card_hearts.insert(*override_color, total);
            }

            for (color, count) in &card_hearts {
                *hearts.entry(*color).or_insert(0) += count;
            }

            if let Some(mods) = heart_modifiers.get(&card_id) {
                for (color, delta) in mods {
                    let new_val = (*hearts.get(color).unwrap_or(&0) as i32 + *delta).max(0) as u32;
                    if new_val > 0 {
                        hearts.insert(*color, new_val);
                    } else {
                        hearts.remove(color);
                    }
                }
            }
        }

        BaseHeart { hearts }
    }
}

pub fn parse_heart_color(s: &str) -> HeartColor {
    crate::card::parse_heart_color(s)
}

#[derive(Debug, Clone)]
pub struct LiveCardZone {
    // Rule 5.2: Live Card Zone - Where member and live cards are placed during Live Card Set Phase
    pub cards: SmallVec<[i16; MAX_LIVE_CARDS]>, // Card IDs - stack-allocated for up to MAX_LIVE_CARDS cards
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
        }
    }

    pub fn can_place_card(&self, _card_db: &CardDatabase, _card_id: i16) -> bool {
        // Rule 8.2: During Live Card Set Phase, any card from hand can be placed in Live Card Zone
        true
    }

    pub fn add_card(&mut self, card_id: i16, _card_db: &CardDatabase) -> Result<(), String> {
        if !self.can_place_card(_card_db, card_id) {
            if let Some(card) = _card_db.get_card(card_id) {
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

    pub fn get_live_cards(&self, card_db: &CardDatabase) -> Vec<i16> {
        self.cards
            .iter()
            .filter(|&&card_id| {
                card_db
                    .get_card(card_id)
                    .map(|c| c.is_live())
                    .unwrap_or(false)
            })
            .copied()
            .collect()
    }

    pub fn clear(&mut self) -> SmallVec<[i16; 3]> {
        std::mem::take(&mut self.cards)
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn calculate_live_score(
        &self,
        card_db: &CardDatabase,
        cheer_blade_heart_count: u32,
        stage_hearts: Option<&crate::card::BaseHeart>,
        need_heart_modifiers: Option<
            &std::collections::HashMap<
                i16,
                std::collections::HashMap<crate::card::HeartColor, ModifierEntry>,
            >,
        >,
        score_modifiers: Option<&std::collections::HashMap<i16, i32>>,
        constant_total_score_bonus: i32,
    ) -> u32 {
        let mut total_score = 0;

        for card_id in &self.cards {
            if let Some(card) = card_db.get_card(*card_id) {
                let base_score = card.get_score() as i32;
                let modifier = score_modifiers
                    .and_then(|sm| sm.get(card_id))
                    .copied()
                    .unwrap_or(0);
                let card_score = (base_score + modifier).max(0) as u32;

                let heart_needs_satisfied = if let Some(ref need_heart) = card.need_heart {
                    if !need_heart.hearts.is_empty() {
                        let effective_need = if let Some(modifiers) = need_heart_modifiers {
                            if let Some(card_mods) = modifiers.get(card_id) {
                                let has_set = card_mods.values().any(|e| e.set != 0);
                                let mut adjusted = if has_set {
                                    BaseHeart {
                                        hearts: HashMap::new(),
                                    }
                                } else {
                                    need_heart.clone()
                                };
                                for (color, me) in card_mods {
                                    if me.set != 0 {
                                        adjusted.hearts.insert(*color, me.set as u32);
                                    }
                                    if me.additive != 0 {
                                        let entry = adjusted.hearts.entry(*color).or_insert(0);
                                        *entry = (*entry as i32 + me.additive).max(0) as u32;
                                    }
                                }
                                Some(adjusted)
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        let ref_need = effective_need.as_ref().unwrap_or(need_heart);
                        stage_hearts
                            .is_some_and(|sh| crate::card::Card::need_heart_satisfied(ref_need, sh))
                    } else {
                        true
                    }
                } else {
                    true
                };

                if heart_needs_satisfied {
                    total_score += card_score;
                }
            }
        }

        total_score + cheer_blade_heart_count + constant_total_score_bonus.max(0) as u32
    }

    pub fn get_top_card(&self) -> Option<i16> {
        // Rule 9.3.1: Get top card from Live Card Zone for victory determination
        self.cards.first().copied()
    }

    pub fn remove_top_card(&mut self) -> Option<i16> {
        // Rule 9.3.1: Remove top card from Live Card Zone
        if self.cards.is_empty() {
            None
        } else {
            self.cards.drain(..1).next()
        }
    }
}

use crate::constants::{MAX_ENERGY_CARDS, MAX_LIVE_CARDS};

#[derive(Debug, Clone)]
pub struct EnergyZone {
    // Rule 5.1: Energy Zone - Where energy cards are placed and activated
    // Q15: Energy deck cards are face-down; energy zone cards are face-up.
    pub cards: SmallVec<[i16; MAX_ENERGY_CARDS]>,
    pub(crate) active_energy_count: usize,
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

    pub fn active_count(&self) -> usize {
        self.active_energy_count
    }

    pub fn set_active_count(&mut self, count: usize) {
        self.active_energy_count = count;
    }

    pub fn add_active(&mut self, delta: usize) {
        self.active_energy_count = self.active_energy_count.saturating_add(delta);
    }

    pub fn sub_active(&mut self, delta: usize) {
        self.active_energy_count = self.active_energy_count.saturating_sub(delta);
    }

    // Q56/Q138: Cost payment — full amount required; under-member energy cannot pay costs.
    pub fn can_pay_energy(&self, amount: usize) -> bool {
        // Rule 5.9: Check if player has enough active energy cards
        self.active_energy_count >= amount
    }

    pub fn pay_energy_count(&mut self, amount: usize) -> bool {
        // Rule 5.9: Pay energy by decrementing active count
        if self.active_energy_count >= amount {
            self.active_energy_count -= amount;
            true
        } else {
            false
        }
    }

    pub fn pay_energy(&mut self, amount: usize) -> Result<(), String> {
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
        self.active_energy_count = self.cards.len();
        // log::debug!("Activated {} energy cards (active_energy_count={})", self.cards.len(), self.active_energy_count);
    }
}

#[derive(Debug, Clone)]
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
            None
        } else {
            Some(self.cards.remove(0))
        }
    }

    pub fn draw_multiple(&mut self, count: usize) -> Vec<i16> {
        (0..count).filter_map(|_| self.draw()).collect()
    }

    /// Peek at the top `count` cards (indices 0..count). Does not remove them.
    pub fn peek_top(&self, count: usize) -> Vec<i16> {
        self.cards.iter().take(count).copied().collect()
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }
}

#[derive(Debug, Clone)]
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
        std::mem::take(&mut self.cards)
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
pub struct ResolutionZone {
    // Rule 5.8: Resolution Zone - Temporary holding area for cards being resolved
    pub cards: SmallVec<[i16; 10]>, // Card IDs - stack-allocated for up to 10 cards
}

impl ResolutionZone {
    pub fn new() -> Self {
        ResolutionZone {
            cards: SmallVec::new(),
        }
    }

    pub fn add_card(&mut self, card_id: i16) {
        self.cards.push(card_id);
    }

    pub fn clear(&mut self) -> SmallVec<[i16; 10]> {
        std::mem::take(&mut self.cards)
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }
}
