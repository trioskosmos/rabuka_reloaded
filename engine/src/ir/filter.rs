use serde::{Deserialize, Serialize};
use crate::card::{CardType, HeartColor};

/// A statically-typed filter for selecting cards.
/// Composable via `And` / `Or`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum CardFilter {
    /// No filter; matches everything.
    Any,
    /// Match a specific card type.
    CardType(CardType),
    /// Match cards belonging to a group name (e.g. "μ's", "虹ヶ咲").
    Group(String),
    /// Match cards belonging to a unit name.
    Unit(String),
    /// Match cards with a specific heart color.
    HasHeartColor(HeartColor),
    /// Match cards with total cost <= limit.
    CostMax(u32),
    /// Match cards with total cost >= limit.
    CostMin(u32),
    /// Match cards with blade >= limit.
    BladeMin(u32),
    /// Match cards whose name contains a fragment.
    NameContains(String),
    /// Match cards whose name is exactly this.
    NameExact(String),
    /// Match cards with at least one of the given names (for multi-name cards).
    NameAnyOf(Vec<String>),
    /// Match cards that have a blade heart.
    HasBladeHeart,
    /// Match cards that are in a specific orientation.
    Orientation(OrientationFilter),
    /// Logical AND of multiple filters.
    And(Vec<CardFilter>),
    /// Logical OR of multiple filters.
    Or(Vec<CardFilter>),
    /// Negation.
    Not(Box<CardFilter>),
    /// Match the activating card itself.
    SelfCard,
    /// Match all cards except the activating card.
    NotSelf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrientationFilter {
    Active,
    Wait,
}
