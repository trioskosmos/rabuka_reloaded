use crate::card::{BladeColor, HeartColor};
use std::collections::HashMap;

/// Holds all modifier data for GameState.
/// Extracted to reduce the 99-field GameState struct.
#[derive(Debug, Clone)]
pub struct GameModifiers {
    pub blade_modifiers: HashMap<i16, i32>,
    pub blade_type_modifiers: HashMap<i16, BladeColor>,
    pub heart_modifiers: HashMap<i16, HashMap<HeartColor, i32>>,
    pub heart_override: HashMap<i16, (HeartColor, u32)>,
    pub orientation_modifiers: HashMap<i16, String>,
    pub cost_modifiers: HashMap<i16, i32>,
    pub score_modifiers: HashMap<i16, i32>,
    pub need_heart_modifiers: HashMap<i16, HashMap<HeartColor, i32>>,
    pub constant_blade_bonuses: HashMap<i16, i32>,
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
        }
    }

    pub fn add_blade_modifier(&mut self, card_id: i16, delta: i32) {
        *self.blade_modifiers.entry(card_id).or_insert(0) += delta;
    }

    pub fn remove_blade_modifier(&mut self, card_id: i16, delta: i32) {
        let val = self.blade_modifiers.entry(card_id).or_insert(0);
        *val -= delta;
        if *val == 0 {
            self.blade_modifiers.remove(&card_id);
        }
    }

    pub fn get_blade_modifier(&self, card_id: i16) -> i32 {
        self.blade_modifiers.get(&card_id).copied().unwrap_or(0)
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

    pub fn add_heart_modifier(&mut self, card_id: i16, color: HeartColor, delta: i32) {
        let colors = self.heart_modifiers.entry(card_id).or_insert_with(HashMap::new);
        *colors.entry(color).or_insert(0) += delta;
    }

    pub fn remove_heart_modifier(&mut self, card_id: i16, color: HeartColor, delta: i32) {
        if let Some(colors) = self.heart_modifiers.get_mut(&card_id) {
            if let Some(modifier) = colors.get_mut(&color) {
                *modifier -= delta;
                if *modifier == 0 {
                    colors.remove(&color);
                }
            }
            if colors.is_empty() {
                self.heart_modifiers.remove(&card_id);
            }
        }
    }

    pub fn get_heart_modifier(&self, card_id: i16, color: HeartColor) -> i32 {
        self.heart_modifiers.get(&card_id)
            .and_then(|colors| colors.get(&color))
            .copied()
            .unwrap_or(0)
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

    pub fn add_score_modifier(&mut self, card_id: i16, delta: i32) {
        *self.score_modifiers.entry(card_id).or_insert(0) += delta;
    }

    pub fn get_score_modifier(&self, card_id: i16) -> i32 {
        self.score_modifiers.get(&card_id).copied().unwrap_or(0)
    }

    pub fn set_score_modifier(&mut self, card_id: i16, value: i32) {
        self.score_modifiers.insert(card_id, value);
    }

    pub fn add_need_heart_modifier(&mut self, card_id: i16, color: HeartColor, delta: i32) {
        let colors = self.need_heart_modifiers.entry(card_id).or_insert_with(HashMap::new);
        *colors.entry(color).or_insert(0) += delta;
    }

    pub fn get_need_heart_modifier(&self, card_id: i16, color: HeartColor) -> i32 {
        self.need_heart_modifiers.get(&card_id)
            .and_then(|colors| colors.get(&color))
            .copied()
            .unwrap_or(0)
    }

    pub fn set_need_heart_modifier(&mut self, card_id: i16, color: HeartColor, value: i32) {
        self.need_heart_modifiers.entry(card_id).or_default().insert(color, value);
    }

    pub fn add_orientation_modifier(&mut self, card_id: i16, orientation: &str) {
        self.orientation_modifiers.insert(card_id, orientation.to_string());
    }

    pub fn add_cost_modifier(&mut self, card_id: i16, delta: i32) {
        *self.cost_modifiers.entry(card_id).or_insert(0) += delta;
    }

    pub fn set_cost_modifier(&mut self, card_id: i16, value: i32) {
        self.cost_modifiers.insert(card_id, value);
    }

    pub fn get_cost_modifier(&self, card_id: i16) -> i32 {
        self.cost_modifiers.get(&card_id).copied().unwrap_or(0)
    }

    pub fn get_orientation_modifier(&self, card_id: i16) -> Option<&String> {
        self.orientation_modifiers.get(&card_id)
    }

    pub fn clear_all_for_card(&mut self, card_id: i16) {
        self.blade_modifiers.remove(&card_id);
        self.heart_modifiers.remove(&card_id);
        self.heart_override.remove(&card_id);
        self.score_modifiers.remove(&card_id);
        self.need_heart_modifiers.remove(&card_id);
        self.orientation_modifiers.remove(&card_id);
        self.cost_modifiers.remove(&card_id);
    }
}
