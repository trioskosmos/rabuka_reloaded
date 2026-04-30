use crate::card::{AbilityCost, AbilityEffect};
use super::types::Choice;
use super::resolver::AbilityResolver;

#[allow(dead_code)]
impl<'a> AbilityResolver<'a> {
    pub fn validate_cost(&self, cost: &AbilityCost) -> Result<(), String> {
        match cost.cost_type.as_deref() {
            Some("sequential_cost") => {
                if let Some(ref costs) = cost.costs {
                    for sub_cost in costs { self.validate_cost(sub_cost)?; }
                    Ok(())
                } else {
                    Err("Sequential cost has no sub-costs".to_string())
                }
            }
            Some("choice_condition") => {
                if let Some(ref options) = cost.options {
                    for option in options {
                        if self.validate_cost(option).is_ok() { return Ok(()); }
                    }
                    Err("No valid cost option available".to_string())
                } else {
                    Err("Choice condition cost has no options".to_string())
                }
            }
            Some("move_cards") => {
                let source = cost.source.as_deref().unwrap_or("");
                let count = cost.count.unwrap_or(1) as usize;
                let player = self.game_state.active_player();
                let available = match source {
                    "hand" => player.hand.cards.len(),
                    "stage" => player.stage.stage.iter().filter(|&&id| id != -1).count(),
                    "waitroom" => player.waitroom.cards.len(),
                    "energy_zone" => player.energy_zone.cards.len(),
                    _ => return Ok(()),
                };
                if available < count {
                    return Err(format!("Not enough cards in {}: need {}, have {}", source, count, available));
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn pay_cost_ir(&mut self, cost: &crate::ir::cost::Cost) -> Result<(), String> {
        use crate::ir::cost::{Cost as IRC, StateChange as IRSC};
        let ae = match cost {
            IRC::PayEnergy { energy, optional } => AbilityCost {
                cost_type: Some("pay_energy".into()),
                energy: Some(*energy),
                optional: Some(*optional),
                ..Default::default()
            },
            IRC::MoveCards { source, destination, count, card_type, target, cost_limit, optional, self_cost, exclude_self } => AbilityCost {
                cost_type: Some("move_cards".into()),
                source: Some(format!("{:?}", source).to_lowercase()),
                destination: Some(format!("{:?}", destination).to_lowercase()),
                count: Some(*count),
                card_type: card_type.as_ref().map(|ct| format!("{:?}", ct)),
                target: Some(match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" }.into()),
                cost_limit: *cost_limit,
                optional: Some(*optional),
                self_cost: Some(*self_cost),
                exclude_self: Some(*exclude_self),
                ..Default::default()
            },
            IRC::ChangeState { state_change, target, card_type, optional, self_cost } => AbilityCost {
                cost_type: Some("change_state".into()),
                state_change: Some(match state_change { IRSC::Wait => "wait", IRSC::Active => "active" }.into()),
                target: Some(match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" }.into()),
                card_type: card_type.as_ref().map(|ct| format!("{:?}", ct)),
                optional: Some(*optional),
                self_cost: Some(*self_cost),
                ..Default::default()
            },
            IRC::Choice(options) => AbilityCost {
                cost_type: Some("choice_condition".into()),
                options: Some(options.iter().map(|o| {
                    let mut sub = AbilityCost { cost_type: Some("move_cards".into()), ..Default::default() };
                    if let IRC::MoveCards { source, destination, count, card_type, target, cost_limit, optional, self_cost, exclude_self } = o {
                        sub.source = Some(format!("{:?}", source).to_lowercase());
                        sub.destination = Some(format!("{:?}", destination).to_lowercase());
                        sub.count = Some(*count);
                        sub.card_type = card_type.as_ref().map(|ct| format!("{:?}", ct));
                        sub.target = Some(match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" }.into());
                        sub.cost_limit = *cost_limit;
                        sub.optional = Some(*optional);
                        sub.self_cost = Some(*self_cost);
                        sub.exclude_self = Some(*exclude_self);
                    }
                    sub
                }).collect()),
                ..Default::default()
            },
            IRC::Sequential(costs) => AbilityCost {
                cost_type: Some("sequential_cost".into()),
                costs: Some(costs.iter().map(|c| {
                    let mut sub = AbilityCost { ..Default::default() };
                    if let IRC::PayEnergy { energy, optional } = c {
                        sub.cost_type = Some("pay_energy".into());
                        sub.energy = Some(*energy);
                        sub.optional = Some(*optional);
                    }
                    sub
                }).collect()),
                ..Default::default()
            },
        };
        self.pay_cost(&ae)
    }

    pub fn pay_cost(&mut self, cost: &AbilityCost) -> Result<(), String> {
        eprintln!("PAY_COST: cost_type={:?}, source={:?}, destination={:?}, card_type={:?}", cost.cost_type, cost.source, cost.destination, cost.card_type);
        match cost.cost_type.as_deref() {
            Some("sequential_cost") => {
                if let Some(ref costs) = cost.costs {
                    for sub_cost in costs {
                        if let Err(e) = self.validate_cost(sub_cost) {
                            return Err(format!("Cannot pay sequential cost: {}", e));
                        }
                    }
                    for sub_cost in costs { self.pay_cost(sub_cost)?; }
                    Ok(())
                } else {
                    Err("Sequential cost has no sub-costs".to_string())
                }
            }
            Some("choice_condition") => {
                if let Some(ref options) = cost.options {
                    let option_texts: Vec<String> = options.iter().map(|o| o.text.clone()).collect();
                    self.pending_choice = Some(Choice::SelectTarget {
                        target: "choice_condition".to_string(),
                        description: format!("Choose cost option: {}", option_texts.join(" OR ")),
                    });
                    self.game_state.pending_ability = Some(crate::game_state::PendingAbilityExecution {
                        card_no: "choice_cost".to_string(),
                        player_id: "self".to_string(),
                        action_index: 0,
                        effect: AbilityEffect { text: cost.text.clone(), action: "choice_condition".into(), ..Default::default() },
                        conditional_choice: None,
                        activating_card: None,
                        ability_index: 0,
                        cost: Some(cost.clone()),
                        cost_choice: None,
                    });
                    return Ok(());
                } else {
                    return Err("Choice condition cost has no options".to_string());
                }
            }
            Some("move_cards") => {
                let is_activation = self.current_ability.as_ref()
                    .and_then(|a| a.triggers.as_ref())
                    .map_or(false, |t| t == crate::triggers::ACTIVATION);

                if cost.optional == Some(true) && !is_activation {
                    let source = cost.source.as_deref().unwrap_or("");
                    let count = cost.count.unwrap_or(1);
                    self.pending_choice = Some(Choice::SelectCard {
                        zone: source.to_string(),
                        card_type: cost.card_type.clone(),
                        count: count as usize,
                        description: format!("Select card(s) to pay optional cost (or skip): {}", cost.text),
                        allow_skip: true,
                    });
                    self.game_state.pending_ability = Some(crate::game_state::PendingAbilityExecution {
                        card_no: "optional_cost".to_string(),
                        player_id: "self".to_string(),
                        action_index: 0,
                        effect: AbilityEffect {
                            text: cost.text.clone(), action: cost.cost_type.clone().unwrap_or_default(),
                            source: cost.source.clone(), destination: cost.destination.clone(),
                            count: cost.count, card_type: cost.card_type.clone(), target: cost.target.clone(),
                            effect_type: None, ..Default::default()
                        },
                        conditional_choice: None, activating_card: None, ability_index: 0,
                        cost: Some(cost.clone()), cost_choice: None,
                    });
                    return Ok(());
                }

                if let Some(ref source) = cost.source {
                    let count = cost.count.unwrap_or(1);
                    let target = cost.target.as_deref().unwrap_or("self");
                    let cost_limit = cost.cost_limit;
                    let card_type_filter = cost.card_type.as_deref();

                    let player = match target {
                        "self" => &self.game_state.player1,
                        "opponent" => &self.game_state.player2,
                        _ => &self.game_state.player1,
                    };

                    let matches_cost_limit = |card_id: i16, limit: Option<u32>| -> bool {
                        if let Some(limit_val) = limit {
                            self.game_state.card_database.get_card(card_id)
                                .and_then(|c| c.cost).map_or(false, |c| c <= limit_val)
                        } else { true }
                    };

                    let matches_card_type = |card_id: i16, filter: Option<&str>| -> bool {
                        match filter {
                            Some("live_card") => self.game_state.card_database.get_card(card_id).map(|c| c.is_live()).unwrap_or(false),
                            Some("member_card") => self.game_state.card_database.get_card(card_id).map(|c| c.is_member()).unwrap_or(false),
                            Some("energy_card") => self.game_state.card_database.get_card(card_id).map(|c| c.is_energy()).unwrap_or(false),
                            None => true, _ => true,
                        }
                    };

                    let matches_character_names = |card_id: i16, names: Option<&Vec<String>>| -> bool {
                        if let Some(ref required_names) = names {
                            self.game_state.card_database.get_card(card_id)
                                .map(|card| required_names.iter().any(|name| card.name.contains(name) || card.name == *name))
                                .unwrap_or(false)
                        } else { true }
                    };

                    let character_filter = cost.characters.as_ref();
                    let matching_count = match source.as_str() {
                        "deck" | "deck_top" => player.main_deck.cards.iter()
                            .filter(|&&card_id| matches_card_type(card_id, card_type_filter) && matches_cost_limit(card_id, cost_limit) && matches_character_names(card_id, character_filter))
                            .count(),
                        "hand" => player.hand.cards.iter()
                            .filter(|&&card_id| matches_card_type(card_id, card_type_filter) && matches_cost_limit(card_id, cost_limit) && matches_character_names(card_id, character_filter))
                            .count(),
                        "discard" => player.waitroom.cards.iter()
                            .filter(|&&card_id| matches_card_type(card_id, card_type_filter) && matches_cost_limit(card_id, cost_limit) && matches_character_names(card_id, character_filter))
                            .count(),
                        "energy_zone" => player.energy_zone.cards.iter()
                            .filter(|&&card_id| matches_card_type(card_id, card_type_filter) && matches_cost_limit(card_id, cost_limit) && matches_character_names(card_id, character_filter))
                            .count(),
                        _ => usize::MAX,
                    };

                    if matching_count < count as usize {
                        return Err(format!("Cannot pay cost: {} has only {} cards matching cost limit {}, need {}", source, matching_count, cost_limit.map(|l| l.to_string()).unwrap_or("none".to_string()), count));
                    }
                }

                let effect = AbilityEffect {
                    text: cost.text.clone(), action: cost.cost_type.clone().unwrap_or_default(),
                    source: cost.source.clone(), destination: cost.destination.clone(),
                    count: cost.count, card_type: cost.card_type.clone(), target: cost.target.clone(),
                    effect_type: None, ..Default::default()
                };
                self.execute_move_cards(&effect)
            }
            Some("change_state") => {
                let state_change = cost.state_change.as_deref().unwrap_or("");
                let is_activation = self.current_ability.as_ref()
                    .and_then(|a| a.triggers.as_ref())
                    .map_or(false, |t| t == crate::triggers::ACTIVATION);

                if cost.optional == Some(true) && !is_activation {
                    let cost_description = if state_change == "wait" { "Put this member to wait state" } else { "Pay cost" };
                    self.pending_choice = Some(Choice::SelectTarget {
                        target: "pay_optional_cost:skip_optional_cost".to_string(),
                        description: format!("Pay optional cost: {}? (pay or skip)", cost_description),
                    });
                    let actual_effect = self.current_ability.as_ref().and_then(|a| a.effect.clone());
                    self.game_state.pending_ability = Some(crate::game_state::PendingAbilityExecution {
                        card_no: "optional_cost".to_string(), player_id: "self".to_string(),
                        action_index: 0, effect: actual_effect.unwrap_or_default(),
                        conditional_choice: None, activating_card: None, ability_index: 0,
                        cost: Some(cost.clone()), cost_choice: None,
                    });
                    return Ok(());
                }

                if state_change == "wait" {
                    let target = cost.target.as_deref().unwrap_or("self");
                    let player = if target == "self" { &self.game_state.player1 } else { &self.game_state.player2 };
                    let card_ids: Vec<i16> = player.stage.stage.iter().filter(|&&id| id != -1).copied().collect();
                    for card_id in card_ids {
                        self.game_state.add_orientation_modifier(card_id, "wait");
                    }
                }
                Ok(())
            }
            Some("pay_energy") => {
                let is_activation = self.current_ability.as_ref()
                    .and_then(|a| a.triggers.as_ref())
                    .map_or(false, |t| t == crate::triggers::ACTIVATION);

                if cost.optional == Some(true) && !is_activation {
                    let energy = cost.energy.unwrap_or(0);
                    self.pending_choice = Some(Choice::SelectTarget {
                        target: "pay_optional_cost:skip_optional_cost".to_string(),
                        description: format!("Pay {} energy (or skip)?", energy),
                    });
                    let actual_effect = self.current_ability.as_ref().and_then(|a| a.effect.clone());
                    self.game_state.pending_ability = Some(crate::game_state::PendingAbilityExecution {
                        card_no: "optional_cost".to_string(), player_id: "self".to_string(),
                        action_index: 0, effect: actual_effect.unwrap_or_default(),
                        conditional_choice: None, activating_card: None, ability_index: 0,
                        cost: Some(cost.clone()), cost_choice: None,
                    });
                    return Ok(());
                }

                let energy = cost.energy.unwrap_or(0);
                let target = cost.target.as_deref().unwrap_or("self");

                if self.game_state.baton_touch_zero_cost && energy > 0 {
                    eprintln!("Skipping pay_energy cost of {} due to baton touch zero cost", energy);
                    return Ok(());
                }

                let player = match target {
                    "self" => &mut self.game_state.player1,
                    "opponent" => &mut self.game_state.player2,
                    _ => &mut self.game_state.player1,
                };

                if energy > 0 {
                    if let Err(e) = player.energy_zone.pay_energy(energy as usize) {
                        return Err(e);
                    }
                }
                let pid = self.game_state.active_player().id.clone();
                self.game_state.publish_event(crate::events::GameEvent::EnergyPaid { player_id: pid, amount: energy });
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

pub fn ae_from_ir(effect: &crate::ir::effect::Effect) -> AbilityEffect {
    use crate::ir::effect::Effect as E;
    match effect {
        E::LookAndSelect { look_action, select_action, .. } => AbilityEffect {
            action: "look_and_select".into(),
            look_action: Some(Box::new(ae_from_ir(look_action))),
            select_action: Some(Box::new(ae_from_ir(select_action))),
            ..Default::default()
        },
        E::LookAt { source, count, .. } => {
            let c = match count { crate::ir::effect::Count::Fixed(n) => *n, _ => 1 };
            AbilityEffect { action: "look_at".into(), source: Some(format!("{:?}", source).to_lowercase()), count: Some(c), ..Default::default() }
        },
        E::Reveal { source, count, target, card_type, heart_colors } => {
            let c = match count { crate::ir::effect::Count::Fixed(n) => *n, _ => 1 };
            let t = match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" };
            AbilityEffect { action: "reveal".into(), source: Some(format!("{:?}", source).to_lowercase()), count: Some(c), target: Some(t.into()), card_type: card_type.as_ref().map(|ct| format!("{:?}", ct)), heart_colors: heart_colors.as_ref().map(|colors| colors.iter().map(|hc| format!("{:?}", hc)).collect()), ..Default::default() }
        },
        E::Select { source, count, target, card_type, optional } => {
            let c = match count { crate::ir::effect::Count::Fixed(n) => *n, _ => 1 };
            let t = match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" };
            AbilityEffect { action: "select".into(), source: Some(format!("{:?}", source).to_lowercase()), count: Some(c), target: Some(t.into()), card_type: card_type.as_ref().map(|ct| format!("{:?}", ct)), optional: Some(*optional), ..Default::default() }
        },
        E::ModifyScore { operation, value, target, duration, card_type, group, per_unit } => {
            let t = match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" };
            AbilityEffect { action: "modify_score".into(), operation: Some(operation.clone()), value: Some(*value), target: Some(t.into()), duration: duration.as_ref().map(|d| match d { crate::ir::effect::Duration::ThisTurn => "this_turn", crate::ir::effect::Duration::ThisLive => "this_live", crate::ir::effect::Duration::LiveEnd => "live_end", _ => "permanent" }.into()), card_type: card_type.as_ref().map(|ct| format!("{:?}", ct)), group: group.as_ref().map(|g| crate::card::GroupInfo { name: g.clone() }), per_unit: Some(per_unit.is_some()), ..Default::default() }
        },
        E::ModifyRequiredHearts { operation, value, heart_color, target, duration } => {
            let t = match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" };
            AbilityEffect { action: "modify_required_hearts".into(), operation: Some(operation.clone()), value: Some(*value), heart_color: heart_color.as_ref().map(|hc| format!("{:?}", hc)), target: Some(t.into()), duration: duration.as_ref().map(|d| match d { crate::ir::effect::Duration::ThisTurn => "this_turn", crate::ir::effect::Duration::ThisLive => "this_live", crate::ir::effect::Duration::LiveEnd => "live_end", _ => "permanent" }.into()), ..Default::default() }
        },
        E::PositionChange { count, target, card_type, group, optional } => {
            let c = match count { crate::ir::effect::Count::Fixed(n) => *n, _ => 1 };
            let t = match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" };
            AbilityEffect { action: "position_change".into(), count: Some(c), target: Some(t.into()), card_type: card_type.as_ref().map(|ct| format!("{:?}", ct)), group: group.as_ref().map(|g| crate::card::GroupInfo { name: g.clone() }), optional: Some(*optional), ..Default::default() }
        },
        E::Appear { source, destination, count, target, card_type } => {
            let c = match count { crate::ir::effect::Count::Fixed(n) => *n, _ => 1 };
            let t = match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" };
            AbilityEffect { action: "appear".into(), source: Some(format!("{:?}", source).to_lowercase()), destination: Some(format!("{:?}", destination).to_lowercase()), count: Some(c), target: Some(t.into()), card_type: card_type.as_ref().map(|ct| format!("{:?}", ct)), ..Default::default() }
        },
        E::GainAbility { ability_text, target, duration } => {
            let t = match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" };
            AbilityEffect { action: "gain_ability".into(), ability_gain: Some(ability_text.clone()), target: Some(t.into()), duration: duration.as_ref().map(|d| match d { crate::ir::effect::Duration::ThisTurn => "this_turn", crate::ir::effect::Duration::ThisLive => "this_live", crate::ir::effect::Duration::LiveEnd => "live_end", _ => "permanent" }.into()), ..Default::default() }
        },
        E::Restriction { restriction_type, target, duration, card_type, restricted_destination, condition } => {
            let t = match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" };
            AbilityEffect { action: "restriction".into(), restriction_type: Some(restriction_type.clone()), target: Some(t.into()), duration: duration.as_ref().map(|d| match d { crate::ir::effect::Duration::ThisTurn => "this_turn", crate::ir::effect::Duration::ThisLive => "this_live", crate::ir::effect::Duration::LiveEnd => "live_end", _ => "permanent" }.into()), card_type: card_type.as_ref().map(|ct| format!("{:?}", ct)), restricted_destination: restricted_destination.as_ref().map(|z| format!("{:?}", z).to_lowercase()), condition: condition.as_ref().map(|c| c.clone().into()), ..Default::default() }
        },
        E::PayEnergy { energy, target, optional } => {
            let t = match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" };
            AbilityEffect { action: "pay_energy".into(), count: Some(*energy), target: Some(t.into()), optional: Some(*optional), ..Default::default() }
        },
        E::DrawUntilCount { target_count, source, target, condition } => {
            let t = match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" };
            AbilityEffect { action: "draw_until_count".into(), target_count: Some(*target_count), count: Some(*target_count), source: Some(format!("{:?}", source).to_lowercase()), target: Some(t.into()), condition: condition.as_ref().map(|c| c.clone().into()), ..Default::default() }
        },
        E::DiscardUntilCount { hand_size, target } => {
            let t = match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" };
            AbilityEffect { action: "discard_until_count".into(), count: Some(*hand_size), target: Some(t.into()), ..Default::default() }
        },
        E::ModifyCost { operation, value, target, per_unit, duration } => {
            let t = match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" };
            AbilityEffect { action: "modify_cost".into(), operation: Some(operation.clone()), value: Some(value.unsigned_abs()), target: Some(t.into()), per_unit: Some(per_unit.is_some()), duration: duration.as_ref().map(|d| match d { crate::ir::effect::Duration::ThisTurn => "this_turn", crate::ir::effect::Duration::ThisLive => "this_live", crate::ir::effect::Duration::LiveEnd => "live_end", _ => "permanent" }.into()), ..Default::default() }
        },
        E::ReYell { lose_blade_hearts, condition } => AbilityEffect {
            action: "re_yell".into(), lose_blade_hearts: Some(*lose_blade_hearts), condition: condition.as_ref().map(|c| c.clone().into()), ..Default::default()
        },
        E::SetBladeType { blade_type, target } => {
            let t = match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" };
            AbilityEffect { action: "set_blade_type".into(), blade_type: Some(blade_type.clone()), target: Some(t.into()), ..Default::default() }
        },
        E::SetHeartType { heart_type, target } => {
            let t = match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" };
            AbilityEffect { action: "set_heart_type".into(), heart_type: Some(heart_type.clone()), target: Some(t.into()), ..Default::default() }
        },
        E::SetScore { value, target } => {
            let t = match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" };
            AbilityEffect { action: "set_score".into(), value: Some(*value), target: Some(t.into()), ..Default::default() }
        },
        E::SetRequiredHearts { value, target } => {
            let t = match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" };
            AbilityEffect { action: "set_required_hearts".into(), value: Some(*value), target: Some(t.into()), ..Default::default() }
        },
        E::SetBladeCount { value, target } => {
            let t = match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" };
            AbilityEffect { action: "set_blade_count".into(), value: Some(*value), target: Some(t.into()), ..Default::default() }
        },
        E::SetCardIdentity { identities, target } => {
            let t = match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" };
            AbilityEffect { action: "set_card_identity".into(), identities: Some(identities.clone()), target: Some(t.into()), ..Default::default() }
        },
        E::ModifyYellCount { operation, count, target } => {
            let t = match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" };
            AbilityEffect { action: "modify_yell_count".into(), operation: Some(operation.clone()), count: Some(*count), target: Some(t.into()), ..Default::default() }
        },
        E::ModifyLimit { operation, amount, target } => {
            let t = match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" };
            AbilityEffect { action: "modify_limit".into(), operation: Some(operation.clone()), value: Some(*amount), target: Some(t.into()), ..Default::default() }
        },
        E::InvalidateAbility { optional, target } => {
            let t = match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" };
            AbilityEffect { action: "invalidate_ability".into(), optional: Some(*optional), target: Some(t.into()), ..Default::default() }
        },
        E::ChooseRequiredHearts { optional, target } => {
            let t = match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" };
            AbilityEffect { action: "choose_required_hearts".into(), optional: Some(*optional), target: Some(t.into()), ..Default::default() }
        },
        E::PlayBatonTouch { count, target } => {
            let c = match count { crate::ir::effect::Count::Fixed(n) => *n, _ => 1 };
            let t = match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" };
            AbilityEffect { action: "play_baton_touch".into(), count: Some(c), target: Some(t.into()), ..Default::default() }
        },
        E::PlaceEnergyUnderMember { count, target } => {
            let c = match count { crate::ir::effect::Count::Fixed(n) => *n, _ => 1 };
            let t = match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" };
            AbilityEffect { action: "place_energy_under_member".into(), count: Some(c), target: Some(t.into()), ..Default::default() }
        },
        E::ActivationCost { cost, target } => {
            let t = match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" };
            AbilityEffect { action: "activation_cost".into(), count: Some(*cost), target: Some(t.into()), ..Default::default() }
        },
        E::ActivationRestriction { target } => {
            let t = match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" };
            AbilityEffect { action: "activation_restriction".into(), target: Some(t.into()), ..Default::default() }
        },
        E::ModifyRequiredHeartsGlobal { operation, value, target } => {
            let t = match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" };
            AbilityEffect { action: "modify_required_hearts_global".into(), operation: Some(operation.clone()), value: Some(*value), target: Some(t.into()), ..Default::default() }
        },
        E::SpecifyHeartColor { choice, target } => {
            let t = match target { crate::ir::effect::Target::Self_ => "self", crate::ir::effect::Target::Opponent => "opponent", _ => "self" };
            AbilityEffect { action: "specify_heart_color".into(), choice: Some(*choice), target: Some(t.into()), ..Default::default() }
        },
        _ => AbilityEffect { action: "custom".into(), text: format!("{:?}", effect), ..Default::default() },
    }
}
