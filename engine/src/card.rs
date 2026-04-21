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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
    #[serde(rename = "b_all")]
    BAll,  // Blade heart wildcard
    #[serde(rename = "draw")]
    Draw,  // Special heart type for drawing cards
    #[serde(rename = "score")]
    Score,  // Special heart type for score bonus
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

#[derive(Debug, Clone, Serialize)]
pub struct BladeHeart {
    pub hearts: HashMap<HeartColor, u32>,
}

impl<'de> Deserialize<'de> for BladeHeart {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawBladeHeart {
            #[serde(flatten)]
            hearts: HashMap<String, u32>,
        }
        
        let raw = RawBladeHeart::deserialize(deserializer)?;
        let hearts = raw.hearts.into_iter().map(|(k, v)| {
            let color = match k.as_str() {
                "heart00" => HeartColor::Heart00,
                "heart01" => HeartColor::Heart01,
                "heart02" => HeartColor::Heart02,
                "heart03" => HeartColor::Heart03,
                "heart04" => HeartColor::Heart04,
                "heart05" => HeartColor::Heart05,
                "heart06" => HeartColor::Heart06,
                "b_all" => HeartColor::BAll,
                "draw" => HeartColor::Draw,
                "score" => HeartColor::Score,
                _ => HeartColor::Heart00,
            };
            (color, v)
        }).collect();
        
        Ok(BladeHeart { hearts })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BaseHeart {
    pub hearts: HashMap<HeartColor, u32>,
}

impl<'de> Deserialize<'de> for BaseHeart {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawBaseHeart {
            #[serde(flatten)]
            hearts: HashMap<String, u32>,
        }
        
        let raw = RawBaseHeart::deserialize(deserializer)?;
        let hearts = raw.hearts.into_iter().map(|(k, v)| {
            let color = match k.as_str() {
                "heart00" => HeartColor::Heart00,
                "heart01" => HeartColor::Heart01,
                "heart02" => HeartColor::Heart02,
                "heart03" => HeartColor::Heart03,
                "heart04" => HeartColor::Heart04,
                "heart05" => HeartColor::Heart05,
                "heart06" => HeartColor::Heart06,
                "b_all" => HeartColor::BAll,
                "draw" => HeartColor::Draw,
                "score" => HeartColor::Score,
                _ => HeartColor::Heart00,
            };
            (color, v)
        }).collect();
        
        Ok(BaseHeart { hearts })
    }
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
    #[serde(skip)]
    pub card_id: i16,  // Database ID for optimization
}

#[derive(Debug, Clone)]
pub struct CardDatabase {
    pub cards: HashMap<i16, Card>,
    pub card_no_to_id: HashMap<String, i16>,
    pub next_id: i16,
}

impl CardDatabase {
    pub fn new() -> Self {
        Self {
            cards: HashMap::new(),
            card_no_to_id: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn load_or_create(cards: Vec<Card>) -> Self {
        let mut db = Self::new();
        
        // Try to load existing mapping
        if let Ok(mapping) = std::fs::read_to_string("card_id_mapping.json") {
            if let Ok(loaded_mapping) = serde_json::from_str::<HashMap<String, i16>>(&mapping) {
                db.card_no_to_id = loaded_mapping;
                db.next_id = db.card_no_to_id.values().max().copied().unwrap_or(0) + 1;
            }
        }
        
        // Add cards, assigning IDs if not already mapped
        for card in cards {
            if !db.card_no_to_id.contains_key(&card.card_no) {
                db.card_no_to_id.insert(card.card_no.clone(), db.next_id);
                db.next_id += 1;
            }
            let card_id = db.card_no_to_id[&card.card_no];
            db.cards.insert(card_id, card);
        }
        
        // Save mapping
        db.save_mapping();
        
        db
    }

    pub fn save_mapping(&self) {
        if let Ok(mapping) = serde_json::to_string_pretty(&self.card_no_to_id) {
            let _ = std::fs::write("card_id_mapping.json", mapping);
        }
    }

    pub fn get_card(&self, card_id: i16) -> Option<&Card> {
        self.cards.get(&card_id)
    }

    pub fn get_card_by_no(&self, card_no: &str) -> Option<&Card> {
        if let Some(&card_id) = self.card_no_to_id.get(card_no) {
            self.cards.get(&card_id)
        } else {
            None
        }
    }

    pub fn get_card_id(&self, card_no: &str) -> Option<i16> {
        self.card_no_to_id.get(card_no).copied()
    }
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
            card_id: 0,
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

#[derive(Debug, Clone, Serialize)]
pub struct SpecialHeart {
    pub hearts: HashMap<HeartColor, u32>,
}

impl<'de> Deserialize<'de> for SpecialHeart {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawSpecialHeart {
            #[serde(flatten)]
            hearts: HashMap<String, u32>,
        }
        
        let raw = RawSpecialHeart::deserialize(deserializer)?;
        let hearts = raw.hearts.into_iter().map(|(k, v)| {
            let color = match k.as_str() {
                "heart00" => HeartColor::Heart00,
                "heart01" => HeartColor::Heart01,
                "heart02" => HeartColor::Heart02,
                "heart03" => HeartColor::Heart03,
                "heart04" => HeartColor::Heart04,
                "heart05" => HeartColor::Heart05,
                "heart06" => HeartColor::Heart06,
                "b_all" => HeartColor::BAll,
                "draw" => HeartColor::Draw,
                "score" => HeartColor::Score,
                _ => HeartColor::Heart00,
            };
            (color, v)
        }).collect();
        
        Ok(SpecialHeart { hearts })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ability {
    #[serde(default = "default_empty_string")]
    pub full_text: String,
    #[serde(default = "default_empty_string")]
    pub triggerless_text: String,
    pub triggers: Option<String>,
    pub use_limit: Option<u32>,
    #[serde(default)]
    pub is_null: bool,
    pub cost: Option<AbilityCost>,
    pub effect: Option<AbilityEffect>,
    pub keywords: Option<Vec<Keyword>>,
}

fn default_empty_string() -> String {
    String::new()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityCost {
    #[serde(default = "default_empty_string")]
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
    #[serde(default = "default_empty_string")]
    pub text: String,
    #[serde(default = "default_empty_string")]
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
    pub max: Option<bool>,
    pub effect_constraint: Option<String>,
    pub shuffle_target: Option<String>,
    pub icon_count: Option<IconCount>,
    pub resource_icon_count: Option<u32>,
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
    // Subvariable fields for ability effects
    pub heart_color: Option<String>,
    pub blade_type: Option<String>,
    pub energy_count: Option<u32>,
    pub target_member: Option<String>,
    // New fields from parser improvements
    pub choice_options: Option<Vec<String>>,
    pub group: Option<GroupInfo>,
    pub per_unit_count: Option<u32>,
    pub per_unit_type: Option<String>,
    pub per_unit_reference: Option<String>,
    pub group_matching: Option<bool>,
    pub repeat_limit: Option<u32>,
    pub repeat_optional: Option<bool>,
    pub is_further: Option<bool>,
    pub cost_result_reference: Option<bool>,
    pub dynamic_count: Option<DynamicCount>,
    pub placement_order: Option<String>,
    pub cost_limit: Option<u32>,
    pub unit: Option<String>,
    pub distinct: Option<String>,
    pub target_player: Option<String>,
    pub target_location: Option<String>,
    pub target_scope: Option<String>,
    pub target_card_type: Option<String>,
    pub activation_condition: Option<String>,
    pub activation_condition_parsed: Option<Condition>,
    pub gained_ability: Option<Box<AbilityEffect>>,
    pub ability_text: Option<String>,
    pub swap_action: Option<String>,
    pub has_member_swapping: Option<bool>,
    pub group_options: Option<Vec<String>>,
    pub card_count: Option<u32>,
    pub use_limit: Option<u32>,
    pub triggers: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionInfo {
    pub position: Option<String>,
    pub target: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupInfo {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicCount {
    #[serde(rename = "type")]
    pub count_type: String,
    pub reference: String,
    pub mode: String,
    pub base_reference: Option<String>,
    pub calculation: Option<String>,
    pub calculation_value: Option<u32>,
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
    // New fields from parser improvements
    pub all_areas: Option<bool>,
    pub exclude_this_member: Option<bool>,
    pub resource_type: Option<String>,
    pub unit: Option<String>,
    pub location_condition: Option<bool>,
    pub cost_result_reference: Option<bool>,
    pub cost_result_group_match: Option<bool>,
    pub group_matching: Option<bool>,
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
            let wildcard_count = *provided_hearts.hearts.get(&HeartColor::Heart00).unwrap_or(&0);
            
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
        if let Some(ref mut base_heart) = self.base_heart {
            let color = crate::zones::parse_heart_color(heart_color);
            *base_heart.hearts.entry(color).or_insert(0) += amount;
        }
    }

    /// Remove hearts of specific color (minimum 0)
    pub fn remove_heart(&mut self, heart_color: &str, amount: u32) {
        if let Some(ref mut base_heart) = self.base_heart {
            let color = crate::zones::parse_heart_color(heart_color);
            let current = base_heart.hearts.get(&color).copied().unwrap_or(0);
            if current <= amount {
                base_heart.hearts.remove(&color);
            } else {
                base_heart.hearts.insert(color, current - amount);
            }
        }
    }

    /// Set hearts to specific value
    pub fn set_heart(&mut self, heart_color: &str, amount: u32) {
        if let Some(ref mut base_heart) = self.base_heart {
            let color = crate::zones::parse_heart_color(heart_color);
            base_heart.hearts.insert(color, amount);
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
