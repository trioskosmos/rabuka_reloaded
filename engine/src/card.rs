use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CardType {
    #[serde(rename = "メンバー")]
    Member,
    #[serde(rename = "ライブ")]
    Live,
    #[serde(rename = "エネルギー")]
    Energy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum HeartColor {
    #[serde(rename = "heart01")]
    Pink,
    #[serde(rename = "heart02")]
    Red,
    #[serde(rename = "heart03")]
    Yellow,
    #[serde(rename = "heart04")]
    Green,
    #[serde(rename = "heart05")]
    Blue,
    #[serde(rename = "heart06")]
    Purple,
    #[serde(rename = "heart00")]
    Wild,
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
    pub product: String,
    #[serde(rename = "type")]
    pub card_type: CardType,
    pub series: String,
    pub unit: Option<String>,
    pub cost: Option<u32>,
    pub base_heart: Option<BaseHeart>,
    pub blade_heart: Option<BladeHeart>,
    pub blade: Option<u32>,
    pub rare: Option<String>,
    pub ability: Option<String>,
    pub faq: Option<Vec<FAQEntry>>,
    #[serde(rename = "_img")]
    pub _img: Option<String>,
    // Live card fields
    pub score: Option<u32>,
    pub need_heart: Option<BaseHeart>,
    pub special_heart: Option<SpecialHeart>,
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

    pub fn total_blades(&self) -> u32 {
        self.blade.unwrap_or(0)
    }

    pub fn get_score(&self) -> u32 {
        self.score.unwrap_or(0)
    }
}
