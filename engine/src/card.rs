use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
pub enum AbilityType {
    Activation,  // 起動能力
    Automatic,   // 自動能力
    Continuous,  // 常時能力
}

// Rule 9.2: Effect Types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

fn default_blade() -> u32 {
    0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ability {
    pub triggerless_text: String,
    pub cost: Option<AbilityCost>,
    pub effect: Option<AbilityEffect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityCost {
    pub blades: u32,
    pub conditions: Vec<Condition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityEffect {
    pub action: String,
    pub parameters: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub condition_type: String,
    pub parameters: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialHeart {
    #[serde(flatten)]
    pub hearts: HashMap<String, u32>,
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
}
