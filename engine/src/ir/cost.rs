use serde::{Deserialize, Serialize};
use crate::card::CardType;

/// A typed cost for ability activation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Cost {
    /// Pay energy (E icons)
    PayEnergy { energy: u32, optional: bool },
    /// Move cards from one zone to another as cost
    MoveCards {
        source: super::Zone,
        destination: super::Zone,
        count: u32,
        card_type: Option<CardType>,
        target: super::Target,
        cost_limit: Option<u32>,
        optional: bool,
        self_cost: bool,
        exclude_self: bool,
    },
    /// Change state (e.g., put this member to wait)
    ChangeState {
        state_change: StateChange,
        target: super::Target,
        card_type: Option<CardType>,
        optional: bool,
        self_cost: bool,
    },
    /// Choice between multiple costs (A OR B)
    Choice(Vec<Cost>),
    /// Sequential costs (A then B)
    Sequential(Vec<Cost>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateChange {
    Wait,
    Active,
}

impl From<crate::card::AbilityCost> for Cost {
    fn from(c: crate::card::AbilityCost) -> Self {
        let target = match c.target.as_deref() {
            Some("self") => super::Target::Self_,
            Some("opponent") => super::Target::Opponent,
            Some("both") => super::Target::Both,
            _ => super::Target::Self_,
        };
        let card_type = c.card_type.as_deref().and_then(parse_card_type);
        let state_change = match c.state_change.as_deref() {
            Some("wait") => StateChange::Wait,
            Some("active") => StateChange::Active,
            _ => StateChange::Wait,
        };

        match c.cost_type.as_deref() {
            Some("pay_energy") => Cost::PayEnergy {
                energy: c.energy.unwrap_or(1),
                optional: c.optional.unwrap_or(false),
            },
            Some("move_cards") => Cost::MoveCards {
                source: super::zone_from_str(c.source.as_deref().unwrap_or("")),
                destination: super::zone_from_str(c.destination.as_deref().unwrap_or("")),
                count: c.count.unwrap_or(1),
                card_type,
                target,
                cost_limit: c.cost_limit,
                optional: c.optional.unwrap_or(false),
                self_cost: c.self_cost.unwrap_or(false),
                exclude_self: c.exclude_self.unwrap_or(false),
            },
            Some("change_state") => Cost::ChangeState {
                state_change,
                target,
                card_type,
                optional: c.optional.unwrap_or(false),
                self_cost: c.self_cost.unwrap_or(false),
            },
            Some("choice_condition") => Cost::Choice(
                c.options.unwrap_or_default().into_iter().map(|o| {
                    Cost::MoveCards {
                        source: super::zone_from_str(o.source.as_deref().unwrap_or("")),
                        destination: super::zone_from_str(o.destination.as_deref().unwrap_or("")),
                        count: o.count.unwrap_or(1),
                        card_type: o.card_type.as_deref().and_then(parse_card_type),
                        target: super::Target::Self_,
                        cost_limit: None,
                        optional: o.optional.unwrap_or(false),
                        self_cost: false,
                        exclude_self: false,
                    }
                }).collect()
            ),
            Some("sequential_cost") => Cost::Sequential(
                c.costs.unwrap_or_default().into_iter().map(|sc| sc.into()).collect()
            ),
            _ => Cost::PayEnergy { energy: 0, optional: false },
        }
    }
}

fn parse_card_type(s: &str) -> Option<crate::card::CardType> {
    match s {
        "member_card" | "メンバー" => Some(crate::card::CardType::Member),
        "live_card" | "ライブ" => Some(crate::card::CardType::Live),
        "energy_card" | "エネルギー" => Some(crate::card::CardType::Energy),
        _ => None,
    }
}
