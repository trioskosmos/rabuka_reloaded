use crate::card::AbilityCost;

#[derive(Debug, Clone)]
pub enum CostEnum {
    Sequential { costs: Vec<CostEnum> },
    ChoiceCondition { options: Vec<AbilityCost>, texts: Vec<String> },
    MoveCards {
        source: String,
        destination: Option<String>,
        count: u32,
        target: String,
        card_type: Option<String>,
        cost_limit: Option<u32>,
        optional: bool,
        self_cost: Option<bool>,
        exclude_self: Option<bool>,
        characters: Option<Vec<String>>,
        state_change: Option<String>,
        position: Option<crate::card::PositionInfo>,
        text: String,
    },
    ChangeState {
        state_change: String,
        target: String,
        optional: bool,
    },
    PayEnergy {
        energy: u32,
        target: String,
        optional: bool,
    },
    EnergyCondition {
        count: u32,
        target: String,
    },
    Reveal {
        source: Option<String>,
        destination: Option<String>,
        count: Option<u32>,
        card_type: Option<String>,
        target: Option<String>,
        text: String,
    },
    PlaceEnergyUnderMember {
        source: Option<String>,
        destination: Option<String>,
        count: Option<u32>,
        card_type: Option<String>,
        target: Option<String>,
        text: String,
    },
}

impl CostEnum {
    pub fn from_ability_cost(cost: &AbilityCost) -> Self {
        match cost.cost_type.as_deref() {
            Some("sequential_cost") => {
                let costs = cost.costs.as_ref()
                    .map(|c| c.iter().map(Self::from_ability_cost).collect())
                    .unwrap_or_default();
                CostEnum::Sequential { costs }
            }
            Some("choice_condition") => {
                let texts = cost.options.as_ref()
                    .map(|o| o.iter().map(|opt| opt.text.clone()).collect())
                    .unwrap_or_default();
                CostEnum::ChoiceCondition {
                    options: cost.options.clone().unwrap_or_default(),
                    texts,
                }
            }
            Some("move_cards") => CostEnum::MoveCards {
                source: cost.source.as_deref().unwrap_or_default().to_string(),
                destination: cost.destination.clone(),
                count: cost.count.unwrap_or(1),
                target: cost.target.as_deref().unwrap_or("self").to_string(),
                card_type: cost.card_type.clone(),
                cost_limit: cost.cost_limit,
                optional: cost.optional.unwrap_or(false),
                self_cost: cost.self_cost,
                exclude_self: cost.exclude_self,
                characters: cost.characters.clone(),
                state_change: cost.state_change.clone(),
                position: cost.position.clone(),
                text: cost.text.clone(),
            },
            Some("change_state") => CostEnum::ChangeState {
                state_change: cost.state_change.as_deref().unwrap_or_default().to_string(),
                target: cost.target.as_deref().unwrap_or("self").to_string(),
                optional: cost.optional.unwrap_or(false),
            },
            Some("pay_energy") => CostEnum::PayEnergy {
                energy: cost.energy.unwrap_or(0),
                target: cost.target.as_deref().unwrap_or("self").to_string(),
                optional: cost.optional.unwrap_or(false),
            },
            Some("energy_condition") => CostEnum::EnergyCondition {
                count: cost.count.unwrap_or(1),
                target: cost.target.as_deref().unwrap_or("self").to_string(),
            },
            Some("reveal") => CostEnum::Reveal {
                source: cost.source.clone(),
                destination: cost.destination.clone(),
                count: cost.count,
                card_type: cost.card_type.clone(),
                target: cost.target.clone(),
                text: cost.text.clone(),
            },
            Some("place_energy_under_member") => CostEnum::PlaceEnergyUnderMember {
                source: cost.source.clone(),
                destination: cost.destination.clone(),
                count: cost.count,
                card_type: cost.card_type.clone(),
                target: cost.target.clone(),
                text: cost.text.clone(),
            },
            ct => {
                eprintln!("Unhandled cost type: {:?}", ct);
                CostEnum::MoveCards {
                    source: String::new(),
                    destination: None,
                    count: 0,
                    target: "self".to_string(),
                    card_type: None,
                    cost_limit: None,
                    optional: false,
                    self_cost: None,
                    exclude_self: None,
                    characters: None,
                    state_change: None,
                    position: None,
                    text: String::new(),
                }
            }
        }
    }
}
