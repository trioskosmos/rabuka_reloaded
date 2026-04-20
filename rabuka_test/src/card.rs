use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum CardType {
    #[serde(rename = "メンバー")]
    // Rule 4.1: Member cards are placed on the stage and used for performance
    Member,
    #[serde(rename = "ライブ")]
    // Rule 4.2: Live cards are placed in Live Card Zone and used for live performance
    Live,
    #[serde(rename = "エネルギー")]
    // Rule 4.3: Energy cards are placed in Energy Zone and used to pay costs
    Energy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum HeartColor {
    #[serde(rename = "heart00")]
    Heart00,  // Index 0 - wildcard, can be treated as any heart01-heart06
    #[serde(rename = "heart01")]
    Heart01,
    #[serde(rename = "heart02")]
    Heart02,
    #[serde(rename = "heart03")]
    Heart03,
    #[serde(rename = "heart04")]
    Heart04,
    #[serde(rename = "heart05")]
    Heart05,
    #[serde(rename = "heart06")]
    Heart06,
}

// Rule 9.1: Ability Types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(dead_code)]
pub enum AbilityType {
    Activation,  // 起動能力
    Automatic,   // 自動能力
    Continuous,  // 常時能力
}

// Rule 9.2: Effect Types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(dead_code)]
pub enum EffectType {
    OneShot,        // 単発効果
    ContinuousEffect,  // 継続効果
    Replacement,   // 置換効果
}

// Rule 11: Keywords
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Keyword {
    Turn1,          // Rule 11.1: First turn only
    Turn2,          // Rule 11.2: Second turn only
    Debut,          // Rule 11.3: First time this member is placed on stage
    LiveStart,      // Rule 11.4: When live card set phase begins
    LiveSuccess,    // Rule 11.5: When live is successful
    Center,         // Rule 11.6: Center position
    LeftSide,       // Rule 11.7: Left side position
    RightSide,      // Rule 11.8: Right side position
    PositionChange, // Rule 11.9: Member moves to different position
    FormationChange, // Rule 11.10: Multiple members move simultaneously
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartIcon {
    pub color: HeartColor,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct RequiredHeart {
    pub color: HeartColor,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BladeHeart {
    #[serde(flatten)]
    pub hearts: HashMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseHeart {
    #[serde(flatten)]
    pub hearts: HashMap<String, u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Card {
    pub card_no: String,
    pub img: Option<String>,
    pub name: String,
    #[serde(default)]
    pub product: String,
    #[serde(rename = "type")]
    pub card_type: CardType,
    #[serde(default)]
    pub series: String,
    #[serde(default = "default_group_from_series")]
    pub group: String,
    pub unit: Option<String>,
    pub cost: Option<u32>,
    pub base_heart: Option<BaseHeart>,
    pub blade_heart: Option<BladeHeart>,
    #[serde(default = "default_blade")]
    pub blade: u32,
    #[serde(default)]
    pub rare: String,
    #[serde(default)]
    pub ability: String,
    #[serde(default)]
    pub faq: Vec<FAQEntry>,
    #[serde(rename = "_img")]
    pub _img: Option<String>,
    // Live card fields
    pub score: Option<u32>,
    pub need_heart: Option<BaseHeart>,
    pub special_heart: Option<SpecialHeart>,
    // Parsed abilities from abilities.json
    #[serde(skip)]
    pub abilities: Vec<Ability>,
}

#[allow(dead_code)]
fn default_group_from_series() -> String {
    String::new()
}

impl<'de> Deserialize<'de> for Card {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Debug, Clone, Deserialize)]
        struct CardHelper {
            pub card_no: String,
            pub img: Option<String>,
            pub name: String,
            #[serde(default)]
            pub product: String,
            #[serde(rename = "type")]
            pub card_type: CardType,
            #[serde(default)]
            pub series: String,
            pub unit: Option<String>,
            pub cost: Option<u32>,
            pub base_heart: Option<BaseHeart>,
            pub blade_heart: Option<BladeHeart>,
            #[serde(default = "default_blade")]
            pub blade: u32,
            #[serde(default)]
            pub rare: String,
            #[serde(default)]
            pub ability: String,
            #[serde(default)]
            pub faq: Vec<FAQEntry>,
            #[serde(rename = "_img")]
            pub _img: Option<String>,
            pub score: Option<u32>,
            pub need_heart: Option<BaseHeart>,
            pub special_heart: Option<SpecialHeart>,
        }

        let helper = CardHelper::deserialize(deserializer)?;
        let group = map_series_to_group(&helper.series);

        Ok(Card {
            card_no: helper.card_no,
            img: helper.img,
            name: helper.name,
            product: helper.product,
            card_type: helper.card_type,
            series: helper.series,
            group,
            unit: helper.unit,
            cost: helper.cost,
            base_heart: helper.base_heart,
            blade_heart: helper.blade_heart,
            blade: helper.blade,
            rare: helper.rare,
            ability: helper.ability,
            faq: helper.faq,
            _img: helper._img,
            score: helper.score,
            need_heart: helper.need_heart,
            special_heart: helper.special_heart,
            abilities: Vec::new(),
        })
    }
}

fn map_series_to_group(series: &str) -> String {
    match series {
        "ラブライブ！" => "μ's".to_string(),
        "ラブライブ！サンシャイン!!" => "Aqours".to_string(),
        "ラブライブ！虹ヶ咲学園スクールアイドル同好会" => "虹ヶ咲".to_string(),
        "ラブライブ！スーパースター!!" => "Liella!".to_string(),
        "蓮ノ空女学院スクールアイドルクラブ" => "蓮ノ空".to_string(),
        _ => String::new(),
    }
}

fn default_blade() -> u32 {
    0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialHeart {
    #[serde(flatten)]
    pub hearts: HashMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ability {
    pub full_text: String,
    pub triggerless_text: String,
    pub triggers: Option<String>,
    pub use_limit: Option<u32>,
    pub is_null: bool,
    pub cost: Option<AbilityCost>,
    pub effect: Option<AbilityEffect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityCost {
    pub text: String,
    #[serde(rename = "type")]
    pub cost_type: Option<String>,
    pub source: Option<String>,
    pub destination: Option<String>,
    pub count: Option<u32>,
    pub card_type: Option<String>,
    pub target: Option<String>,
    pub action: Option<String>,
    pub optional: Option<bool>,
    pub energy: Option<u32>,
    pub state_change: Option<String>,
    pub position: Option<PositionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityEffect {
    pub text: String,
    pub action: String,
    pub source: Option<String>,
    pub destination: Option<String>,
    pub count: Option<u32>,
    pub card_type: Option<String>,
    pub target: Option<String>,
    pub duration: Option<String>,
    pub parenthetical: Option<Vec<String>>,
    pub look_action: Option<Box<AbilityEffect>>,
    pub select_action: Option<Box<AbilityEffect>>,
    pub actions: Option<Vec<AbilityEffect>>,
    pub resource: Option<String>,
    pub position: Option<PositionInfo>,
    pub state_change: Option<String>,
    pub optional: Option<bool>,
    pub max: Option<u32>,
    pub effect_constraint: Option<String>,
    pub shuffle_target: Option<String>,
    pub icon_count: Option<IconCount>,
    pub ability_gain: Option<String>,
    pub quoted_text: Option<QuotedText>,
    pub per_unit: Option<bool>,
    pub condition: Option<Condition>,
    pub primary_effect: Option<Box<AbilityEffect>>,
    pub alternative_condition: Option<Condition>,
    pub alternative_effect: Option<Box<AbilityEffect>>,
    pub operation: Option<String>,
    pub value: Option<u32>,
    pub aggregate: Option<String>,
    pub comparison_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionInfo {
    pub position: Option<String>,
    pub target: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconCount {
    pub icon: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotedText {
    pub text: String,
    pub quoted_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Condition {
    pub text: String,
    #[serde(rename = "type")]
    pub condition_type: Option<String>,
    pub location: Option<String>,
    pub count: Option<u32>,
    pub operator: Option<String>,
    pub card_type: Option<String>,
    pub target: Option<String>,
    pub group: Option<serde_json::Value>,
    pub group_names: Option<Vec<String>>,
    pub characters: Option<Vec<String>>,
    pub state: Option<String>,
    pub position: Option<PositionInfo>,
    pub temporal_scope: Option<String>,
    pub distinct: Option<bool>,
    pub unique: Option<bool>,
    pub exclude_self: Option<bool>,
    pub any_of: Option<Vec<String>>,
    pub cost_limit: Option<u32>,
    pub exact_match: Option<bool>,
    pub negation: Option<bool>,
    pub includes_pattern: Option<String>,
    pub movement_condition: Option<String>,
    pub baton_touch_trigger: Option<bool>,
    pub movement_state: Option<String>,
    pub energy_state: Option<String>,
    pub aggregate_flags: Option<Vec<String>>,
    pub comparison_target: Option<String>,
    pub comparison_operator: Option<String>,
    pub movement: Option<String>,
    pub heart_variety: Option<bool>,
    pub activation_condition: Option<String>,
    pub activation_position: Option<String>,
    pub trigger_type: Option<String>,
    pub trigger_event: Option<String>,
    pub temporal: Option<String>,
    pub phase: Option<String>,
    pub aggregate: Option<String>,
    pub comparison_type: Option<String>,
    pub includes: Option<bool>,
    pub appearance: Option<bool>,
    pub conditions: Option<Vec<Condition>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FAQEntry {
    pub title: String,
    pub question: String,
    pub answer: String,
    pub relation: Vec<CardRelation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardRelation {
    pub card_no: String,
    pub name: String,
}

impl Card {
    pub fn is_member(&self) -> bool {
        matches!(self.card_type, CardType::Member)
    }

    pub fn is_live(&self) -> bool {
        matches!(self.card_type, CardType::Live)
    }

    pub fn is_energy(&self) -> bool {
        matches!(self.card_type, CardType::Energy)
    }

    pub fn total_hearts(&self) -> u32 {
        if let Some(ref base_heart) = self.base_heart {
            base_heart.hearts.values().sum()
        } else if let Some(ref need_heart) = self.need_heart {
            need_heart.hearts.values().sum()
        } else {
            0
        }
    }
    
    pub fn satisfies_heart_requirement(&self, provided_hearts: &BaseHeart) -> bool {
        // Rule 8.2.8: Check if provided hearts satisfy card's need_heart requirement
        // Heart00 (index 0) is wildcard and can substitute for any heart01-heart06
        if let Some(ref need_heart) = self.need_heart {
            let wildcard_count = *provided_hearts.hearts.get("heart00").unwrap_or(&0);
            
            for (color, needed_amount) in &need_heart.hearts {
                if let Some(&provided_amount) = provided_hearts.hearts.get(color) {
                    if provided_amount + wildcard_count >= *needed_amount {
                        // Subtract the specific hearts first, then use wildcard if needed
                        let remaining_needed = if provided_amount >= *needed_amount {
                            0
                        } else {
                            *needed_amount - provided_amount
                        };
                        if remaining_needed > wildcard_count {
                            return false;
                        }
                    } else {
                        // Not enough even with wildcard
                        if *needed_amount > wildcard_count {
                            return false;
                        }
                    }
                } else {
                    // No specific hearts available, use wildcard
                    if *needed_amount > wildcard_count {
                        return false;
                    }
                }
            }
            true
        } else {
            // No heart requirement
            true
        }
    }
    
    pub fn get_live_score(&self) -> u32 {
        // Rule 9.2.1: Get live score for live cards
        self.score.unwrap_or(0)
    }

    pub fn total_blades(&self) -> u32 {
        self.blade
    }

    pub fn get_score(&self) -> u32 {
        self.score.unwrap_or(0)
    }

    // ============== RESOURCE MODIFICATION METHODS ==============

    /// Add blades to card
    pub fn add_blades(&mut self, amount: u32) {
        self.blade += amount;
    }

    /// Remove blades from card (minimum 0)
    pub fn remove_blades(&mut self, amount: u32) {
        self.blade = self.blade.saturating_sub(amount);
    }

    /// Set blades to specific value
    pub fn set_blades(&mut self, amount: u32) {
        self.blade = amount;
    }

    /// Add hearts of specific color
    pub fn add_heart(&mut self, heart_color: &str, amount: u32) {
        if self.base_heart.is_none() {
            self.base_heart = Some(BaseHeart {
                hearts: std::collections::HashMap::new(),
            });
        }
        if let Some(ref mut base_heart) = self.base_heart {
            *base_heart.hearts.entry(heart_color.to_string()).or_insert(0) += amount;
        }
    }

    /// Remove hearts of specific color (minimum 0)
    pub fn remove_heart(&mut self, heart_color: &str, amount: u32) {
        if let Some(ref mut base_heart) = self.base_heart {
            let current = base_heart.hearts.get(heart_color).copied().unwrap_or(0);
            if current <= amount {
                base_heart.hearts.remove(heart_color);
            } else {
                base_heart.hearts.insert(heart_color.to_string(), current - amount);
            }
        }
    }

    /// Set hearts to specific value
    pub fn set_heart(&mut self, heart_color: &str, amount: u32) {
        if self.base_heart.is_none() {
            self.base_heart = Some(BaseHeart {
                hearts: std::collections::HashMap::new(),
            });
        }
        if let Some(ref mut base_heart) = self.base_heart {
            base_heart.hearts.insert(heart_color.to_string(), amount);
        }
    }

    /// Add score to card
    pub fn add_score(&mut self, amount: u32) {
        if self.score.is_none() {
            self.score = Some(0);
        }
        if let Some(ref mut score) = self.score {
            *score += amount;
        }
    }

    /// Remove score from card (minimum 0)
    pub fn remove_score(&mut self, amount: u32) {
        if let Some(ref mut score) = self.score {
            *score = score.saturating_sub(amount);
        }
    }

    /// Set score to specific value
    pub fn set_score(&mut self, amount: u32) {
        self.score = Some(amount);
    }

    /// Modify cost by amount (minimum 0)
    pub fn modify_cost(&mut self, amount: i32) {
        if self.cost.is_none() {
            self.cost = Some(0);
        }
        if let Some(ref mut cost) = self.cost {
            if amount >= 0 {
                *cost += amount as u32;
            } else {
                *cost = cost.saturating_sub((-amount) as u32);
            }
        }
    }

    /// Set cost to specific value
    pub fn set_cost(&mut self, amount: u32) {
        self.cost = Some(amount);
    }
}
