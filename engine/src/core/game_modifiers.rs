use crate::card::{BladeColor, HeartColor};
use crate::types::AbilityApplication;
use std::collections::HashMap;

/// Stores both the additive delta and absolute set value for a modifier.
/// Replaces the old dual-map pattern (`blade_modifiers` + `set_blade_modifiers`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ModifierEntry {
    /// Accumulated via repeated `add_*` / `+=` calls.
    pub additive: i32,
    /// Set via `set_*` calls (absolute override).
    pub set: i32,
}

impl std::fmt::Display for ModifierEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.total())
    }
}

impl ModifierEntry {
    pub fn total(&self) -> i32 {
        self.set + self.additive
    }
}

/// Holds all modifier data for GameState.
/// Extracted to reduce the 99-field GameState struct.
#[derive(Debug, Clone)]
pub struct GameModifiers {
    pub blade_modifiers: HashMap<i16, ModifierEntry>,
    pub blade_type_modifiers: HashMap<i16, BladeColor>,
    pub heart_modifiers: HashMap<i16, HashMap<HeartColor, ModifierEntry>>,
    pub heart_override: HashMap<i16, (HeartColor, u32)>,
    pub orientation_modifiers: HashMap<i16, String>,
    pub cost_modifiers: HashMap<i16, ModifierEntry>,
    pub score_modifiers: HashMap<i16, ModifierEntry>,
    pub need_heart_modifiers: HashMap<i16, HashMap<HeartColor, ModifierEntry>>,
    pub constant_blade_bonuses: HashMap<i16, i32>,
    pub constant_cost_bonuses: HashMap<i16, i32>,
    pub constant_score_bonuses: HashMap<i16, i32>,
    pub constant_heart_bonuses: HashMap<i16, HashMap<String, i32>>,
    /// Track need_heart modifiers applied by constant ModifyRequiredHeartsGlobal effects.
    /// Key: (target_card_id, heart_color_str) → total delta applied.
    pub constant_global_need_heart: Vec<(i16, String, i32)>,
    /// Per-player global constant score bonus from GainAbility (modify_score) effects.
    /// Accumulated in recalculate_constants, added directly to each player's total live score.
    pub p1_constant_total_score_bonus: i32,
    pub p2_constant_total_score_bonus: i32,
    /// Source info for constant score bonuses: (card_id, ability_text, value)
    /// Populated by recalculate_constants for display in breakdown.scores.
    pub constant_score_sources: Vec<(i16, String, i32)>,
    pub heart_color_multiplier: HashMap<i16, HeartColor>,
    /// Number of cards moved from hand to discard by the most recent cost payment.
    pub last_cost_discard_count: u32,
    /// Number of energy cards paid by the most recent cost payment.
    pub last_cost_energy_count: u32,
    /// Per-card delayed "cannot activate" flags. Card_id → remaining turns of
    /// activation block. Decremented each Active phase; member stays wait while >0.
    pub delayed_cannot_active: HashMap<i16, u32>,
    /// The surplus heart count just before it was zeroed by gain_resource(surplus_heart).
    /// Used by `delta: true` conditions on subsequent steps.
    pub last_surplus_loss_count: u32,
}

impl Default for GameModifiers {
    fn default() -> Self {
        Self::new()
    }
}

impl GameModifiers {
    pub fn new() -> Self {
        GameModifiers {
            blade_modifiers: HashMap::new(),
            blade_type_modifiers: HashMap::new(),
            heart_modifiers: HashMap::new(),
            heart_override: HashMap::new(),
            orientation_modifiers: HashMap::new(),
            cost_modifiers: HashMap::new(),
            score_modifiers: HashMap::new(),
            need_heart_modifiers: HashMap::new(),
            constant_blade_bonuses: HashMap::new(),
            constant_cost_bonuses: HashMap::new(),
            constant_score_bonuses: HashMap::new(),
            constant_heart_bonuses: HashMap::new(),
            constant_global_need_heart: Vec::new(),
            p1_constant_total_score_bonus: 0,
            p2_constant_total_score_bonus: 0,
            constant_score_sources: Vec::new(),
            heart_color_multiplier: HashMap::new(),
            last_cost_discard_count: 0,
            last_cost_energy_count: 0,
            delayed_cannot_active: HashMap::new(),
            last_surplus_loss_count: 0,
        }
    }

    // ============== BLADE ==============

    pub fn add_blade_modifier(&mut self, card_id: i16, delta: i32) {
        self.blade_modifiers.entry(card_id).or_default().additive += delta;
    }

    /// Like add_blade_modifier but also records the source for snapshot tracing.
    pub fn add_blade_modifier_with_trace(
        &mut self,
        card_id: i16,
        delta: i32,
        trace: &mut Vec<AbilityApplication>,
        source_card_id: i16,
        ability_text: &str,
    ) {
        self.add_blade_modifier(card_id, delta);
        trace.push(AbilityApplication {
            source_card_id,
            ability_text: ability_text.to_string(),
            effect_type: "blade_bonus".to_string(),
            target_card_id: card_id,
            heart_color: None,
            amount: delta,
        });
    }

    pub fn remove_blade_modifier(&mut self, card_id: i16, delta: i32) {
        if let Some(entry) = self.blade_modifiers.get_mut(&card_id) {
            entry.additive -= delta;
            if entry.additive == 0 && entry.set == 0 {
                self.blade_modifiers.remove(&card_id);
            }
        }
    }

    pub fn get_blade_modifier(&self, card_id: i16) -> i32 {
        self.blade_modifiers.get(&card_id).map_or(0, |e| e.total())
    }

    pub fn set_blade_modifier(&mut self, card_id: i16, value: i32) {
        self.blade_modifiers.entry(card_id).or_default().set = value;
    }

    pub fn clear_blade_set_modifier(&mut self, card_id: i16) {
        if let Some(entry) = self.blade_modifiers.get_mut(&card_id) {
            entry.set = 0;
            if entry.additive == 0 && entry.set == 0 {
                self.blade_modifiers.remove(&card_id);
            }
        }
    }

    pub fn set_blade_type_modifier(&mut self, card_id: i16, blade_color: BladeColor) {
        self.blade_type_modifiers.insert(card_id, blade_color);
    }

    pub fn get_blade_type_modifier(&self, card_id: i16) -> Option<BladeColor> {
        self.blade_type_modifiers.get(&card_id).copied()
    }

    pub fn clear_blade_type_modifier(&mut self, card_id: i16) {
        self.blade_type_modifiers.remove(&card_id);
    }

    // ============== HEART ==============

    pub fn add_heart_modifier(&mut self, card_id: i16, color: HeartColor, delta: i32) {
        let colors = self.heart_modifiers.entry(card_id).or_default();
        colors.entry(color).or_default().additive += delta;
    }

    /// Like add_heart_modifier but also records the source for snapshot tracing.
    pub fn add_heart_modifier_with_trace(
        &mut self,
        card_id: i16,
        color: HeartColor,
        delta: i32,
        trace: &mut Vec<AbilityApplication>,
        source_card_id: i16,
        ability_text: &str,
    ) {
        self.add_heart_modifier(card_id, color, delta);
        trace.push(AbilityApplication {
            source_card_id,
            ability_text: ability_text.to_string(),
            effect_type: "heart_bonus".to_string(),
            target_card_id: card_id,
            heart_color: Some(color.index()),
            amount: delta,
        });
    }

    pub fn get_heart_modifier(&self, card_id: i16, color: HeartColor) -> i32 {
        let by_color = self.heart_modifiers.get(&card_id);
        let color_val = || -> i32 {
            by_color
                .and_then(|colors| colors.get(&color))
                .map_or(0, |e| e.total())
        };
        let wildcard_val = || -> i32 {
            by_color
                .and_then(|colors| colors.get(&HeartColor::Heart00))
                .map_or(0, |e| e.total())
        };
        color_val() + wildcard_val()
    }

    pub fn set_heart_modifier(&mut self, card_id: i16, color: HeartColor, value: i32) {
        self.heart_modifiers
            .entry(card_id)
            .or_default()
            .entry(color)
            .or_default()
            .set = value;
    }

    pub fn remove_heart_modifier(&mut self, card_id: i16, color: HeartColor, delta: i32) {
        if let Some(colors) = self.heart_modifiers.get_mut(&card_id) {
            if let Some(entry) = colors.get_mut(&color) {
                entry.additive -= delta;
                if entry.additive == 0 && entry.set == 0 {
                    colors.remove(&color);
                }
            }
            if colors.is_empty() {
                self.heart_modifiers.remove(&card_id);
            }
        }
    }

    pub fn get_heart_modifier_wildcard(&self, card_id: i16, color: HeartColor) -> i32 {
        let by_color = self.heart_modifiers.get(&card_id);
        let specific = by_color
            .and_then(|colors| colors.get(&color))
            .map_or(0, |e| e.total());
        let wildcard = by_color
            .and_then(|colors| colors.get(&HeartColor::Heart00))
            .map_or(0, |e| e.total());
        specific + wildcard
    }

    pub fn set_heart_override(&mut self, card_id: i16, color: HeartColor, count: u32) {
        self.heart_override.insert(card_id, (color, count));
    }

    pub fn get_heart_override(&self, card_id: i16) -> Option<&(HeartColor, u32)> {
        self.heart_override.get(&card_id)
    }

    pub fn remove_heart_override(&mut self, card_id: i16) {
        self.heart_override.remove(&card_id);
    }

    // ============== SCORE ==============

    pub fn add_score_modifier(&mut self, card_id: i16, delta: i32) {
        self.score_modifiers.entry(card_id).or_default().additive += delta;
    }

    pub fn remove_score_modifier(&mut self, card_id: i16, delta: i32) {
        if let Some(entry) = self.score_modifiers.get_mut(&card_id) {
            entry.additive -= delta;
            if entry.additive == 0 && entry.set == 0 {
                self.score_modifiers.remove(&card_id);
            }
        }
    }

    pub fn get_score_modifier(&self, card_id: i16) -> i32 {
        self.score_modifiers.get(&card_id).map_or(0, |e| e.total())
    }

    pub fn set_score_modifier(&mut self, card_id: i16, value: i32) {
        self.score_modifiers.entry(card_id).or_default().set = value;
    }

    pub fn clear_score_set_modifier(&mut self, card_id: i16) {
        if let Some(entry) = self.score_modifiers.get_mut(&card_id) {
            entry.set = 0;
            if entry.additive == 0 {
                self.score_modifiers.remove(&card_id);
            }
        }
    }

    // ============== NEED HEART ==============

    pub fn add_need_heart_modifier(&mut self, card_id: i16, color: HeartColor, delta: i32) {
        let colors = self.need_heart_modifiers.entry(card_id).or_default();
        colors.entry(color).or_default().additive += delta;
    }

    pub fn get_need_heart_modifier(&self, card_id: i16, color: HeartColor) -> i32 {
        self.need_heart_modifiers
            .get(&card_id)
            .and_then(|colors| colors.get(&color))
            .map_or(0, |e| e.total())
    }

    pub fn set_need_heart_modifier(&mut self, card_id: i16, color: HeartColor, value: i32) {
        self.need_heart_modifiers
            .entry(card_id)
            .or_default()
            .entry(color)
            .or_default()
            .set = value;
    }

    // ============== ORIENTATION ==============

    pub fn add_orientation_modifier(&mut self, card_id: i16, orientation: &str) {
        // Idempotent: skip if card already has the target orientation.
        // This prevents setting "wait" on an already-wait card — a card
        // can only be waited once per standing.  All 11 call sites
        // (costs, effects, move_cards handlers) benefit from this guard.
        if self.orientation_modifiers.get(&card_id).map(|s| s.as_str()) == Some(orientation) {
            return;
        }
        self.orientation_modifiers
            .insert(card_id, orientation.to_string());
    }

    // ============== COST ==============

    pub fn add_cost_modifier(&mut self, card_id: i16, delta: i32) {
        self.cost_modifiers.entry(card_id).or_default().additive += delta;
    }

    pub fn remove_cost_modifier(&mut self, card_id: i16, delta: i32) {
        if let Some(entry) = self.cost_modifiers.get_mut(&card_id) {
            entry.additive = (entry.additive - delta).max(0);
            if entry.additive == 0 && entry.set == 0 {
                self.cost_modifiers.remove(&card_id);
            }
        }
    }

    pub fn get_cost_modifier(&self, card_id: i16) -> i32 {
        self.cost_modifiers.get(&card_id).map_or(0, |e| e.total())
    }

    pub fn set_cost_modifier(&mut self, card_id: i16, value: i32) {
        self.cost_modifiers.entry(card_id).or_default().set = value;
    }

    pub fn get_orientation_modifier(&self, card_id: i16) -> Option<&String> {
        self.orientation_modifiers.get(&card_id)
    }

    // ============== CLEAR ==============

    // ============== DELAYED CANNOT ACTIVATE ==============

    /// Add a per-card "cannot activate" flag for N turns.
    pub fn add_delayed_cannot_active(&mut self, card_id: i16, turns: u32) {
        // If already set, keep the larger remaining duration
        let current = self
            .delayed_cannot_active
            .get(&card_id)
            .copied()
            .unwrap_or(0);
        self.delayed_cannot_active
            .insert(card_id, current.max(turns));
    }

    /// Returns true if this card's activation is blocked by a delayed flag.
    pub fn is_delayed_cannot_active(&self, card_id: i16) -> bool {
        self.delayed_cannot_active
            .get(&card_id)
            .copied()
            .unwrap_or(0)
            > 0
    }

    /// Decrement all delayed_cannot_active counters by 1. Removes entries that reach 0.
    pub fn tick_delayed_cannot_active(&mut self) {
        self.delayed_cannot_active.retain(|_, count| {
            *count = count.saturating_sub(1);
            *count > 0
        });
    }

    pub fn clear_all_for_card(&mut self, card_id: i16) {
        self.blade_modifiers.remove(&card_id);
        self.blade_type_modifiers.remove(&card_id);
        self.heart_modifiers.remove(&card_id);
        self.heart_override.remove(&card_id);
        self.score_modifiers.remove(&card_id);
        self.need_heart_modifiers.remove(&card_id);
        self.orientation_modifiers.remove(&card_id);
        self.cost_modifiers.remove(&card_id);
        self.constant_blade_bonuses.remove(&card_id);
        self.constant_cost_bonuses.remove(&card_id);
        self.constant_score_bonuses.remove(&card_id);
        self.constant_heart_bonuses.remove(&card_id);
        self.heart_color_multiplier.remove(&card_id);
        self.delayed_cannot_active.remove(&card_id);
    }
}
