use super::debug::AbDebug;
use super::enums::Zone;
use super::resolver::AbilityResolver;
use super::types::{Choice, ChoiceRoute};
use super::util;
use crate::card::AbilityEffect;
use crate::game_state::GameState;

impl AbilityResolver {
    /// Pay all deferred costs that were stored during sequential_cost handler.
    /// Clears the list after paying. Returns error if any cost cannot be paid.
    pub fn pay_deferred_costs(&mut self, gs: &mut GameState) -> Result<(), String> {
        let costs = std::mem::take(&mut self.pending_deferred_costs);
        for cost in &costs {
            self.pay_cost(gs, cost)?;
        }
        Ok(())
    }
}

/// Returns true if the given cost type creates a "pay or skip" prompt.
/// Costs without a prompt (pay_energy with fixed count, change_state with
/// self_cost=true) don't require choosing — they just apply a fixed effect.
/// In sequential_cost, prompt-less costs before a choice-based cost are
/// auto-paid without a "pay or skip" prompt.
fn has_skip_prompt(cost: &AbilityEffect) -> bool {
    match cost.action.as_str() {
        "pay_energy" => !cost.any_number_any().unwrap_or(false),
        "change_state" => cost.self_cost_any() == Some(true),
        _ => false,
    }
}

fn get_change_state_candidates(
    gs: &GameState,
    target: &str,
    card_type: Option<&str>,
    group_names: Option<&Vec<String>>,
    exclude_self: bool,
    self_cost: bool,
    check_name: bool,
    state: Option<&str>,
) -> Vec<i16> {
    let player = gs.resolve_target_player(target);
    let card_db = &gs.card_database;
    let activating_id = gs.activating_card;

    player
        .stage
        .stage
        .iter()
        .filter(|&&id| id != -1)
        .copied()
        .filter(|&id| {
            if self_cost {
                activating_id == Some(id)
            } else {
                !(exclude_self && activating_id == Some(id))
            }
        })
        .filter(|&id| util::card_matches_type(card_db, id, card_type))
        .filter(|&id| match group_names {
            Some(gn) => {
                let group_ok = gn
                    .iter()
                    .any(|g| util::card_matches_group_str(card_db, id, Some(g.as_str())));
                if check_name {
                    group_ok || util::card_matches_characters(card_db, id, Some(gn))
                } else {
                    group_ok
                }
            }
            None => true,
        })
        .filter(|&id| match state {
            Some(s) => gs
                .mods
                .get_orientation_modifier(id)
                .map_or(s == "active", |o| o == s),
            None => true,
        })
        .collect()
}

impl AbilityResolver {
    pub fn validate_cost(&self, gs: &mut GameState, cost: &AbilityEffect) -> Result<(), String> {
        match cost.action.as_str() {
            "sequential_cost" => {
                if let Some(ref costs) = cost.compound.actions {
                    for sub_cost in costs {
                        self.validate_cost(gs, sub_cost)?;
                    }
                }
                Ok(())
            }
            "choice_condition" => Ok(()),
            "move_cards" => {
                let count = cost.count.unwrap_or(1) as usize;
                let source = cost.source.as_deref().unwrap_or("");
                let target_str = cost.target.as_deref().unwrap_or("self");
                let player = gs.resolve_target_player(target_str);
                if !matches!(
                    Zone::from_str(source),
                    Some(Zone::Hand | Zone::Stage | Zone::Waitroom | Zone::Energy)
                ) {
                    return Ok(());
                }
                let available = util::get_zone_card_count(player, source);
                // Rule 9.4.2.3 / Q56: Costs must be paid in full.
                // If even one sub-cost cannot be fully paid, the entire
                // cost is impossible and the ability cannot be activated.
                if available < count {
                    return Err(format!(
                        "Not enough cards in {}: need {}, have {}",
                        source, count, available
                    ));
                }
                Ok(())
            }
            "energy_condition" => {
                let count = cost.count.unwrap_or(1) as usize;
                let player = gs.resolve_target_player("self");
                if player.energy_zone.cards.len() < count {
                    return Err(format!(
                        "Not enough energy cards: need {}, have {}",
                        count,
                        player.energy_zone.cards.len()
                    ));
                }
                Ok(())
            }
            // Q137: 「ウェイトにする」はアクティブ状態のメンバーをウェイト状態にすること
            // を意味します。既にウェイト状態のメンバーをコストで「ウェイトにする」ことは
            // できません。対象がいない場合、そのコストは支払えません。
            "change_state" => {
                let state_change_binding = cost.state_change_any();
                let state_change = state_change_binding.unwrap_or("");
                if state_change == "wait" {
                    let target = cost.target.as_deref().unwrap_or("self");
                    let exclude_self = cost.exclude_self_any().unwrap_or(false);
                    let candidates = get_change_state_candidates(
                        gs,
                        target,
                        cost.card_type_any().as_deref(),
                        cost.group_names_any(),
                        exclude_self,
                        cost.self_cost_any().unwrap_or(false),
                        false,
                        Some("active"),
                    );
                    if candidates.is_empty() {
                        return Err("No active members on stage to wait (Q137)".to_string());
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn pay_cost_inner(&mut self, gs: &mut GameState, cost: &AbilityEffect) -> Result<(), String> {
        let mut dbg = AbDebug::new();
        dbg.cost_pay(cost, true);
        match cost.action.as_str() {
            // Rule 9.4.2 / Q234: Sequential cost — each sub-cost is paid in order.
            // Sub-costs are validated first (ALL must be payable), then paid one
            // by one. If any sub-cost creates a pending_choice, we yield and resume
            // from cost_paid_index on the next invocation.
            //
            // Q234: "Deck has only 2 cards, can I pay this cost?" → No.
            //   The cost requires drawing 3+ cards from deck. If the resource
            //   is insufficient, validate_cost rejects the entire cost.
            "sequential_cost" => {
                if let Some(ref costs) = cost.compound.actions {
                    let start_idx = gs
                        .ability_queue
                        .current_entry()
                        .map_or(0, |e| e.cost_paid_index);
                    for i in start_idx..costs.len() {
                        if let Err(e) = self.validate_cost(gs, &costs[i]) {
                            return Err(format!("Cannot pay sequential cost: {}", e));
                        }
                    }
                    let mut had_binary_auto_pay = false;
                    for i in start_idx..costs.len() {
                        let sub_cost = &costs[i];
                        // Rule 9.4.2.2: costs execute in order.
                        // Binary sub-costs (pay_energy w/ fixed count, change_state w/
                        // self_cost=true) that are optional AND precede a choice-based
                        // sub-cost are auto-paid without a "pay or skip" prompt.
                        // The choice sub-cost's skip handles skipping the entire cost.
                        let is_binary =
                            sub_cost.optional.unwrap_or(false) && has_skip_prompt(sub_cost);
                        let has_choice_ahead =
                            (i + 1..costs.len()).any(|j| !has_skip_prompt(&costs[j]));
                        if is_binary && has_choice_ahead {
                            let mut auto = sub_cost.clone();
                            auto.set_optional(Some(false));
                            self.pending_deferred_costs.push(auto);
                            had_binary_auto_pay = true;
                        } else {
                            self.pay_cost(gs, sub_cost)?;
                        }
                        if let Some(entry) = gs.ability_queue.current_entry_mut() {
                            entry.cost_paid_index = i + 1;
                        }
                        if self.pending_choice.is_some() {
                            // If we auto-paid binary costs before this choice, override
                            // the choice description to show the combined cost.
                            if had_binary_auto_pay {
                                let combined_en =
                                    crate::ability::describe::describe_sequential_cost_en(costs, i);
                                let combined_ja =
                                    crate::ability::describe::describe_sequential_cost_ja(costs, i);
                                if let Some(ref mut choice) = self.pending_choice {
                                    choice.set_description(combined_en.clone());
                                    choice.set_bilingual_descriptions(
                                        Some(combined_en),
                                        Some(combined_ja),
                                    );
                                }
                            }
                            return Ok(());
                        }
                    }
                }
                Ok(())
            }
            "choice_condition" => {
                let texts: Vec<String> = cost
                    .compound
                    .actions
                    .as_ref()
                    .map(|o| o.iter().map(|opt| opt.text.clone()).collect())
                    .unwrap_or_default();
                self.pending_choice = Some(Choice::SelectTarget {
                    target: "choice_condition".to_string(),
                    description: format!("Choose cost option: {}", texts.join(" OR ")),
                    description_en: Some(format!("Choose cost option: {}", texts.join(" OR "))),
                    description_ja: Some(format!(
                        "コストオプションを選択: {}",
                        texts.join(" または ")
                    )),
                    allow_skip: false,
                    options: Some(texts),
                });
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.choice_card_no = Some(ChoiceRoute::ChoiceCost);
                }
                Ok(())
            }
            "move_cards" => {
                let source = cost.source.as_deref().unwrap_or("");
                // any_number means player chooses 0..N
                let is_any_number = cost.any_number_any().unwrap_or(false);
                let count = cost.count.unwrap_or(1) as usize;
                let card_type = cost.card_type_any().map(|s| s.to_string());
                let optional = cost.optional.unwrap_or(false);
                let _text = &cost.text;
                let is_activation = self
                    .current_ability
                    .as_ref()
                    .and_then(|a| a.triggers.as_ref())
                    .is_some_and(|t| &**t == crate::triggers::ACTIVATION);

                let same_unit = cost.same_unit_name_any().unwrap_or(false);
                let is_from_hand = Zone::from_str(source) == Some(Zone::Hand) && !same_unit;
                if is_from_hand {
                    let target_str = cost.target.as_deref().unwrap_or("self");
                    let pl = gs.resolve_target_player(target_str);
                    let card_db = &gs.card_database;
                    let cost_limit = cost.cost_limit_any();
                    let card_type_filter = card_type.as_deref();
                    let mut filter = cost.filter_subset();
                    filter.card_type = card_type_filter;
                    filter.cost_limit = cost_limit;
                    let resolved_group: Option<String> =
                        if cost.group_reference_any().as_deref() == Some("same_group_name") {
                            self.activating_card_id
                                .and_then(|cid| card_db.get_card(cid))
                                .and_then(|c| {
                                    if c.group.is_empty() {
                                        None
                                    } else {
                                        Some(c.group.to_string())
                                    }
                                })
                        } else {
                            None
                        };
                    if let Some(ref g) = resolved_group {
                        filter.group = Some(g.as_str());
                    }
                    let mut matching_indices: Vec<usize> = pl
                        .hand
                        .cards
                        .iter()
                        .enumerate()
                        .filter(|(_, &cid)| filter.matches(card_db, cid, false))
                        .map(|(i, _)| i)
                        .collect();
                    // Activating card has no group — no cards can satisfy "same_group_name"
                    if cost.group_reference_any().as_deref() == Some("same_group_name")
                        && resolved_group.is_none()
                    {
                        matching_indices.clear();
                    }
                    let is_optional = (optional || is_any_number) && !is_activation;
                    let match_names: Vec<String> = matching_indices
                        .iter()
                        .filter_map(|&i| {
                            if i < pl.hand.cards.len() {
                                card_db
                                    .get_card(pl.hand.cards[i])
                                    .map(|c| c.name.to_string())
                            } else {
                                None
                            }
                        })
                        .collect();
                    // For "any number" costs, the effective count is 0 (any number)
                    // unless `max` is also set, in which case count caps the max.
                    let effective_count = if is_any_number { 0 } else { count };
                    log::debug!(
                        "▶ cost(move_cards, {}=>discard, effective_count={}, any_number={}, optional={})",
                        source, effective_count, is_any_number, is_optional
                    );
                    log::debug!(
                        "  ├─ hand[{}] → {} match{}: [{}]",
                        pl.hand.cards.len(),
                        matching_indices.len(),
                        if matching_indices.len() == 1 {
                            ""
                        } else {
                            "es"
                        },
                        match_names.join(", ")
                    );

                    let _has_restrictions = cost.characters_any().is_some()
                        || cost.group_names_any().is_some()
                        || cost.card_type_any().is_some()
                        || cost.cost_limit_any().is_some();
                    if !is_any_number && matching_indices.len() < count {
                        if is_optional {
                            // If optional cost, we should auto-skip if the hand is completely empty or doesn't have enough matching cards for name-restricted costs.
                            // But if we just don't have enough cards in hand for a general optional cost (like having 0 cards when needing 1), we should auto-skip it.
                            log::debug!("  └─ skip (optional, not enough eligible cards in hand)");
                            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                                entry.cost_paid = true;
                                entry.optional_cost_result = Some(false);
                            }
                            return Ok(());
                        } else {
                            // Non-optional and not enough matching cards -> cannot pay the cost
                            return Err(format!(
                                "Not enough matching cards in hand to pay cost. Needs {}, has {}",
                                count,
                                matching_indices.len()
                            ));
                        }
                    } else if is_any_number && matching_indices.is_empty() {
                        if is_optional {
                            log::debug!(
                                "  └─ skip (optional any_number, no eligible cards in hand)"
                            );
                            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                                entry.cost_paid = true;
                                entry.optional_cost_result = Some(false);
                            }
                            return Ok(());
                        } else {
                            // Non-optional, any_number requires at least 0? Wait, standard any_number normally allows 0. But if matching_indices is empty we just skip.
                            return Ok(());
                        }
                    } else if !matching_indices.is_empty() {
                        let desc = if is_any_number {
                            let max_str = if cost.max.unwrap_or(false) {
                                count.min(matching_indices.len())
                            } else {
                                matching_indices.len()
                            };
                            format!(
                                "Select any number of card(s) from hand (0-{}) (or skip)",
                                max_str
                            )
                        } else {
                            format!(
                                "Select {} card(s) from hand{}",
                                effective_count,
                                if is_optional { " (or skip)" } else { "" }
                            )
                        };
                        log::debug!("  └─ choice created (allow_skip={})", is_optional);
                        if optional {
                            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                                entry.choice_card_no = Some(ChoiceRoute::OptionalCost);
                            }
                        }
                        let filtered = if resolved_group.is_some() {
                            Some(matching_indices.clone())
                        } else {
                            None
                        };
                        self.pending_choice =
                            Some(
                                Choice::select_cards(
                                    source.to_string(),
                                    effective_count,
                                    desc,
                                    is_optional,
                                )
                                .description_ja(Some(if is_any_number {
                                    format!("手札から任意枚選択（スキップ可）")
                                } else {
                                    format!(
                                        "手札から{}枚選択{}",
                                        effective_count,
                                        if is_optional {
                                            "（スキップ可）"
                                        } else {
                                            ""
                                        }
                                    )
                                }))
                                .card_type(card_type.clone())
                                .cost_limit(
                                    cost.cost_limit_any(),
                                    cost.cost_limit_operator_any().map(|s| s.to_string()),
                                )
                                .group(resolved_group.clone().or_else(|| {
                                    cost.group_names_any().clone().map(|v| v.join(","))
                                }))
                                .characters(cost.characters_any().cloned())
                                .target_player_id(Some(
                                    cost.target.as_deref().unwrap_or("self").to_string(),
                                ))
                                .filtered_indices(filtered)
                                .build(),
                            );
                        return Ok(());
                    } else if !is_optional {
                        // Non-optional, no matches — fall through to error
                    }
                }
                if !source.is_empty() {
                    let target = cost.target.as_deref().unwrap_or("self");
                    let cost_limit = cost.cost_limit_any();
                    let card_type_filter = card_type.as_deref();

                    let player = gs.resolve_target_player(target);
                    let card_db = &gs.card_database;
                    let mut filter = cost.filter_subset();
                    filter.card_type = card_type_filter;
                    filter.cost_limit = cost_limit;
                    let _zone_cards = util::zone_cards(player, source);

                    if same_unit {
                        let is_optional = optional && !is_activation;
                        let player_ref = gs.resolve_target_player(target);
                        let hand_cards = &player_ref.hand.cards;
                        // Group hand cards by unit name
                        let mut unit_groups: std::collections::BTreeMap<String, Vec<i16>> =
                            std::collections::BTreeMap::new();
                        for &cid in hand_cards {
                            if filter.matches(card_db, cid, false) {
                                let unit = card_db
                                    .get_card(cid)
                                    .and_then(|c| c.unit.clone().map(|s| s.to_string()))
                                    .unwrap_or_default();
                                unit_groups.entry(unit).or_default().push(cid);
                            }
                        }
                        // Collect ALL hand indices from units with >= count members
                        let eligible_indices: Vec<usize> = hand_cards
                            .iter()
                            .enumerate()
                            .filter(|(_, &cid)| {
                                if let Some(card) = card_db.get_card(cid) {
                                    let unit = card.unit.as_deref().unwrap_or("");
                                    unit_groups.get(unit).is_some_and(|g| g.len() >= count)
                                } else {
                                    false
                                }
                            })
                            .map(|(idx, _)| idx)
                            .collect();
                        if eligible_indices.is_empty() {
                            if is_optional {
                                return Ok(());
                            }
                            return Err(format!(
                                "Cannot pay cost: no unit has {} cards matching filter",
                                count
                            ));
                        }
                        let desc_en =
                            format!("Select 1 card (need {} with the same unit name)", count);
                        let desc_ja = format!("同名ユニットが{}枚必要なカードを1枚選択", count);
                        self.pending_choice = Some(
                            Choice::select_cards(Zone::Hand.to_str(), 1, desc_en, is_optional)
                                .description_ja(Some(desc_ja))
                                .card_type(cost.card_type_any().map(|s| s.to_string()))
                                .target_player_id(Some(
                                    cost.target.as_deref().unwrap_or("self").to_string(),
                                ))
                                .filtered_indices(Some(eligible_indices))
                                .build(),
                        );
                        return Ok(());
                    }

                    let zone_name = if Zone::from_str(source) == Some(Zone::DeckTop) {
                        Zone::Deck.to_str()
                    } else {
                        source
                    };
                    let matching_count =
                        util::count_in_zone(player, zone_name, &filter, card_db) as usize;

                    // Q104 / Rule 10.2.1: For deck_top costs, if the deck has fewer
                    // cards than needed but the waitroom has cards, allow the cost to
                    // proceed — the drawing loop will perform a refresh mid-draw and
                    // continue. Only fail if both deck AND waitroom are truly empty.
                    let is_deck_top = Zone::from_str(source) == Some(Zone::DeckTop);
                    if matching_count < count && !is_deck_top {
                        return Err(format!(
                            "Cannot pay cost: {} has only {} cards matching cost limit {}, need {}",
                            source,
                            matching_count,
                            cost_limit
                                .map(|l| l.to_string())
                                .unwrap_or("none".to_string()),
                            count
                        ));
                    }
                    if matching_count < count && is_deck_top {
                        let waitroom_matching =
                            util::count_in_zone(player, Zone::Waitroom.to_str(), &filter, card_db)
                                as usize;
                        if matching_count + waitroom_matching == 0 {
                            return Err(format!(
                                "Cannot pay cost: {} and waitroom are both empty, need {}",
                                source, count
                            ));
                        }
                    }
                }

                let effect = AbilityEffect {
                    text: cost.text.clone(),
                    action: cost.action.clone(),
                    source: cost.source.clone(),
                    destination: cost.destination.clone(),
                    count: cost.count,
                    target: cost.target.clone(),
                    kind: cost.kind.clone(),
                    ..Default::default()
                };
                self.execute_move_cards(gs, &effect)
            }
            // Q144 / Q145 / Q183 / Q257: Change state cost (wait/rest)
            //
            // Q144: "Can I wait 1 member when only 1 eligible exists even
            //   though the ability says 'up to 2'?" → Yes. "Up to N" allows
            //   choosing any number ≤ N.
            //
            // Q145: "Can I activate an ability that waits a member when
            //   there are no eligible targets?" → Yes, as long as the cost
            //   is optional. The ability proceeds without effect.
            //
            // Q183: "Does change_state target my member or the opponent's?"
            //   → Always your own stage. "Select a member" without qualifier
            //   means your own stage.
            //
            // Q257: "Do I have to wait the member even if target is gone?"
            //   → Yes, if the cost is mandatory. Costs are paid regardless
            //   of whether the effect can resolve (Rule 9.4.2).
            "change_state" => {
                let state_change_binding = cost.state_change_any();
                let state_change = state_change_binding.unwrap_or("");
                let target = cost.target.as_deref().unwrap_or("self");
                let optional = cost.optional.unwrap_or(false);
                let is_activation = self
                    .current_ability
                    .as_ref()
                    .and_then(|a| a.triggers.as_ref())
                    .is_some_and(|t| &**t == crate::triggers::ACTIVATION);

                if optional && !is_activation {
                    // Q137: 「ウェイトにする」とは、アクティブ状態のメンバーをウェイト
                    // 状態にすることを意味します。既にウェイト状態のメンバーをコストで
                    // 「ウェイトにする」ことはできません。
                    // Verify active candidates exist before prompting — applies to
                    // both self_cost and non-self_cost.
                    if state_change == "wait" {
                        let exclude_self = cost.exclude_self_any().unwrap_or(false);
                        let candidates = get_change_state_candidates(
                            gs,
                            target,
                            cost.card_type_any().as_deref(),
                            cost.group_names_any(),
                            exclude_self,
                            cost.self_cost_any().unwrap_or(false),
                            false,
                            Some("active"),
                        );
                        if candidates.is_empty() {
                            return Ok(());
                        }
                    }
                    let cost_description = if state_change == "wait" {
                        "Put this member to wait state"
                    } else {
                        "Pay cost"
                    };
                    self.pending_choice = Some(Choice::SelectTarget {
                        target: "pay_optional_cost:skip_optional_cost".to_string(),
                        description: format!(
                            "Pay optional cost: {}? (pay or skip)",
                            cost_description
                        ),
                        description_en: Some(format!(
                            "Pay optional cost: {}? (pay or skip)",
                            cost_description
                        )),
                        description_ja: Some(if state_change == "wait" {
                            "このメンバーをレスト状態にする（支払う/スキップ）？".to_string()
                        } else {
                            "コストを支払う（支払う/スキップ）？".to_string()
                        }),
                        allow_skip: true,
                        options: None,
                    });
                    if let Some(entry) = gs.ability_queue.current_entry_mut() {
                        entry.choice_card_no = Some(ChoiceRoute::OptionalCost);
                    }
                    return Ok(());
                }

                if state_change == "wait" {
                    let count = cost.count.unwrap_or(1) as usize;
                    let exclude_self = cost.exclude_self_any().unwrap_or(false);
                    let candidates = get_change_state_candidates(
                        gs,
                        target,
                        cost.card_type_any().as_deref(),
                        cost.group_names_any(),
                        exclude_self,
                        cost.self_cost_any().unwrap_or(false),
                        true,
                        Some("active"),
                    );
                    log::debug!("[CHANGE_STATE] candidates={:?}", candidates);

                    // Q137: 「ウェイトにする」はアクティブ状態のメンバーをウェイト状態に
                    // することを意味します。対象がいない場合、その行為自体が行われません。
                    // For mandatory costs this means the cost cannot be paid.
                    if candidates.is_empty() {
                        return Err("No matching members on stage to change state".to_string());
                    }

                    if candidates.len() <= count {
                        for &card_id in &candidates {
                            log::debug!(
                                "[TRACE_COST_WAIT] setting card {} to wait, stage before={:?}",
                                card_id,
                                gs.player1.stage.stage
                            );
                            gs.mods.add_orientation_modifier(card_id, "wait");
                        }
                    } else {
                        let desc_ja = format!("ウェイトにするステージメンバーを{}体選択", count);
                        self.pending_choice = Some(
                            Choice::select_cards(
                                Zone::Stage.to_str(),
                                count,
                                format!("Select {} stage member(s) to wait", count),
                                false,
                            )
                            .description_ja(Some(desc_ja))
                            .card_type(cost.card_type_any().map(|s| s.to_string()))
                            .is_select_action(true)
                            .target_player_id(Some(
                                cost.target.as_deref().unwrap_or("self").to_string(),
                            ))
                            .build(),
                        );
                        return Ok(());
                    }
                }
                Ok(())
            }
            // Rule 5.9 / Rule 9.4.2 / Q215: Pay energy cost
            //
            // Rule 5.9: Paying N energy means tapping N active energy cards
            //   (changing them from active to wait state).
            //
            // Rule 9.4.2.3: If the required energy can't be paid (not enough
            //   active energy), the entire cost is impossible and the ability
            //   cannot activate. For optional costs, this means auto-skip.
            //
            // Q215: "Can I place wait-state energy under a member as cost?"
            //   → Yes, energy state (active/wait) doesn't restrict placement.
            //   However, pay_energy specifically only taps ACTIVE energy.
            "pay_energy" => {
                let energy = cost.energy_count_any().unwrap_or(0);
                let target = cost.target.as_deref().unwrap_or("self");
                let optional = cost.optional.unwrap_or(false);
                let any_number = cost.any_number_any().unwrap_or(false);
                let is_activation = self
                    .current_ability
                    .as_ref()
                    .and_then(|a| a.triggers.as_ref())
                    .is_some_and(|t| &**t == crate::triggers::ACTIVATION);

                if any_number && (optional || !is_activation) {
                    let player = gs.resolve_target_player_mut(target);
                    let active_count = player.energy_zone.active_count();
                    if active_count == 0 {
                        // No energy to pay — treat as skip
                        if let Some(entry) = gs.ability_queue.current_entry_mut() {
                            entry.cost_paid = true;
                            entry.optional_cost_result = Some(false);
                        }
                        let pp = gs.player_prefix();
                        gs.rule_log.push(format!(
                            "{}: [[log_cost_skip:reason=no_active_energy,need=any,active=0]]",
                            pp
                        ));
                        return Ok(());
                    }
                    // Show active energy cards for selection (one by one with skip)
                    let filtered_indices: Vec<usize> = (0..active_count).collect();
                    self.pending_choice = Some(
                        Choice::select_cards(
                            Zone::Energy.to_str().to_string(),
                            0,
                            format!(
                                "Select energy card to pay (active: {}). Skip when done",
                                active_count
                            ),
                            true,
                        )
                        .description_ja(Some(format!(
                            "エネルギーカードを選択（アクティブ: {}）完了でスキップ",
                            active_count
                        )))
                        .filtered_indices(Some(filtered_indices))
                        .target_player_id(Some(target.to_string()))
                        .build(),
                    );
                    if let Some(entry) = gs.ability_queue.current_entry_mut() {
                        entry.choice_card_no = Some(ChoiceRoute::OptionalCost);
                    }
                    return Ok(());
                }

                if optional && !is_activation {
                    let player = gs.resolve_target_player(target);
                    let active = player.energy_zone.active_count() as u32;
                    if active < energy {
                        if let Some(entry) = gs.ability_queue.current_entry_mut() {
                            entry.cost_paid = true;
                            entry.optional_cost_result = Some(false);
                        }
                        let pp = gs.player_prefix();
                        gs.rule_log.push(format!(
                            "{}: [[log_cost_skip:reason=insufficient_energy,need={},active={}]]",
                            pp, energy, active
                        ));
                        return Ok(());
                    }
                    self.pending_choice = Some(Choice::SelectTarget {
                        target: "pay_optional_cost:skip_optional_cost".to_string(),
                        description: format!("Pay {} energy (or skip)?", energy),
                        description_en: Some(format!("Pay {} energy (or skip)?", energy)),
                        description_ja: Some(format!("{}エネルギー支払う（スキップ可）？", energy)),
                        allow_skip: true,
                        options: None,
                    });
                    if let Some(entry) = gs.ability_queue.current_entry_mut() {
                        entry.choice_card_no = Some(ChoiceRoute::OptionalCost);
                    }
                    return Ok(());
                }

                if gs.baton_touch_zero_cost && energy > 0 {
                    log::debug!(
                        "Skipping pay_energy cost of {} due to baton touch zero cost",
                        energy
                    );
                    return Ok(());
                }

                let player = gs.resolve_target_player_mut(target);

                if energy > 0 {
                    player.energy_zone.pay_energy(energy as usize)?
                }
                Ok(())
            }
            "energy_condition" => {
                let count = cost.count.unwrap_or(1) as usize;
                let target = cost.target.as_deref().unwrap_or("self");
                let player = gs.resolve_target_player_mut(target);
                if player.energy_zone.cards.len() < count {
                    return Err(format!(
                        "Not enough energy cards: need {}, have {}",
                        count,
                        player.energy_zone.cards.len()
                    ));
                }
                for _ in 0..count {
                    if let Some(card) = player.energy_zone.cards.pop() {
                        player.energy_deck.cards.push(card);
                    }
                }
                player.energy_zone.sub_active(count);
                Ok(())
            }
            "reveal" => {
                let source = cost.source.as_deref().unwrap_or(Zone::Hand.to_str());
                let target = cost.target.as_deref().unwrap_or("self");
                let card_type = cost.card_type_any().map(|s| s.to_string());

                let mut card_ids: Vec<i16> = {
                    let player = gs.resolve_target_player(target);
                    let card_db = &gs.card_database;
                    match Zone::from_str(source) {
                        Some(Zone::Hand) => player
                            .hand
                            .cards
                            .iter()
                            .filter(|&&id| {
                                super::util::card_matches_type(card_db, id, card_type.as_deref())
                            })
                            .copied()
                            .collect(),
                        _ => vec![],
                    }
                };

                if card_ids.is_empty() {
                    return Err("No cards to reveal".to_string());
                }

                // Dedup card_ids — multiple hand entries may share the same template ID
                // when copies of the same card were created (same pool entry reused).
                card_ids.sort();
                card_ids.dedup();

                let has_explicit_count = cost.count.is_some();
                let explicit_count = cost.count.unwrap_or(1) as usize;

                if has_explicit_count && card_ids.len() <= explicit_count {
                    let cost_source = gs.current_ability_source_card_id();
                    let cost_owner =
                        util::target_player_index(target, gs.ability_master_id().as_deref());
                    for &card_id in &card_ids {
                        gs.push_revealed_card(card_id, cost_source, false, cost_owner);
                        gs.push_revealed_cost_card(card_id, cost_source, false, cost_owner);
                    }
                    Ok(())
                } else {
                    let group = cost
                        .group_names_any()
                        .as_ref()
                        .and_then(|gn| gn.first().cloned());
                    self.pending_choice = Some(
                        Choice::select_cards(
                            source.to_string(),
                            if has_explicit_count {
                                explicit_count
                            } else {
                                0
                            },
                            "Select cards to reveal from hand".to_string(),
                            true,
                        )
                        .description_ja(Some("手札からカードを選択して公開".to_string()))
                        .card_type(card_type.clone())
                        .cost_limit(
                            cost.cost_limit_any(),
                            cost.cost_limit_operator_any().map(|s| s.to_string()),
                        )
                        .group(group)
                        .characters(cost.characters_any().cloned())
                        .target_player_id(Some(
                            cost.target.as_deref().unwrap_or("self").to_string(),
                        ))
                        .is_reveal(true)
                        .build(),
                    );
                    Ok(())
                }
            }
            "place_energy_under_member" => {
                self.execute_place_energy_under_member(
                    gs,
                    cost.count.unwrap_or(1),
                    cost.target.as_deref().unwrap_or("self"),
                    cost.position_any(),
                    cost.optional.unwrap_or(false),
                    cost.source.as_deref(),
                    cost.any_number_any().unwrap_or(false),
                );
                Ok(())
            }
            "custom" => {
                if cost.destination.as_deref().and_then(Zone::from_str) == Some(Zone::UnderMember) {
                    self.execute_place_energy_under_member(
                        gs,
                        cost.count.unwrap_or(1),
                        cost.target.as_deref().unwrap_or("self"),
                        cost.position_any(),
                        cost.optional.unwrap_or(false),
                        None,
                        cost.any_number_any().unwrap_or(false),
                    );
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn pay_cost(&mut self, gs: &mut GameState, cost: &AbilityEffect) -> Result<(), String> {
        let result = self.pay_cost_inner(gs, cost);
        if result.is_ok() && self.pending_choice.is_none() {
            let pp = gs.player_prefix();
            let act_name = gs
                .activating_card
                .and_then(|id| gs.card_database.get_card(id))
                .map(|c| c.name.to_string());
            let cost_desc = cost.text.split("}}").last().unwrap_or(&cost.text).trim();
            gs.log_entry(
                format!(
                    "{pp} {}: [cost] {} ✓",
                    act_name.as_deref().unwrap_or(""),
                    cost_desc
                ),
                &pp,
                gs.activating_card,
                act_name,
                "cost",
            );
        }
        result
    }

    pub fn handle_optional_cost_payment(
        &mut self,
        gs: &mut GameState,
        selected: &str,
    ) -> Result<(), String> {
        if selected == "skip_optional_cost" || selected == "0" {
            self.pending_choice = None;
            self.pending_energy_payment = None;
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.cost_paid = true;
                // If the cost has an alternative_effect ("unless you pay"),
                // schedule it as a pending command so it fires after skip.
                let alt = entry
                    .ability
                    .cost
                    .as_ref()
                    .and_then(|c| c.alternative_effect_any().clone());
                entry.effect_started = false;
                entry.optional_cost_result = Some(false);
                if let Some(alt_effect) = alt {
                    entry.pending_actions = vec![*alt_effect.clone()];
                } else {
                    entry.pending_actions.clear();
                }
            }
            return self.resume_pending_actions(gs);
        }
        // "pay_optional_cost" or "1" from select_option(1)
        self.pending_choice = None;
        if let Some(count) = self.pending_energy_payment {
            self.pending_energy_payment = None;
            let player = gs.resolve_target_player_mut("self");
            if player.energy_zone.active_count() >= count as usize {
                player.energy_zone.pay_energy(count as usize)?;
            } else {
                // Insufficient energy: clear remaining commands and return
                self.cancel_remaining_commands = true;
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.pending_actions.clear();
                }
                return self.resume_pending_actions(gs);
            }
        }
        let has_pending = gs.ability_queue.has_pending_actions();
        let has_cost = gs.entry_cost().is_some();
        let is_deck_top = gs
            .entry_effect()
            .and_then(|e| e.source.as_deref())
            .is_some_and(|s| s == "deck_top" || s == "deck")
            || gs
                .ability_queue
                .current_entry()
                .map(|e| &e.pending_actions)
                .map(|pa| {
                    pa.iter().any(|a| {
                        a.source.as_deref() == Some("deck_top")
                            || a.source.as_deref() == Some("deck")
                    })
                })
                .unwrap_or(false);
        if let Some(entry) = gs.ability_queue.current_entry_mut() {
            entry.cost_paid = true;
            entry.optional_cost_result = Some(true);
            if !has_cost && is_deck_top {
                entry.effect_started = false;
                entry.conditional_choice = Some("pay_optional_cost".to_string());
                entry.optional_cost_result = None;
            }
        }
        if has_pending {
            return self.resume_pending_actions(gs);
        }
        let is_pay = true;
        if is_pay {
            if let Some(cost) = gs.entry_cost().cloned() {
                if let Some(energy) = cost.energy_count_any() {
                    if energy > 0 {
                        let tgt = cost.target.as_deref().unwrap_or("self");
                        gs.resolve_target_player_mut(tgt)
                            .energy_zone
                            .pay_energy(energy as usize)?;
                    }
                }
                if cost.state_change_any().as_deref() == Some("wait") {
                    if cost.self_cost_any() == Some(true) {
                        if let Some(id) = gs.activating_card {
                            // Q159: The card must be on stage to be put to wait.
                            // When activated from discard (e.g. via activate_ability),
                            // the card is not on stage, so the cost cannot be paid.
                            let on_stage = gs
                                .resolve_target_player_mut("self")
                                .stage
                                .stage
                                .iter()
                                .any(|&sid| sid == id);
                            if !on_stage {
                                return Err("Cannot pay cost: member is not on stage".to_string());
                            }
                            // Q137: 「ウェイトにする」はアクティブ状態のメンバーをウェイト
                            // 状態にすることを意味します。既にウェイト状態の場合、
                            // その行為自体が行われません（Rule 1.3.2.1）。
                            let already_waited = gs
                                .mods
                                .get_orientation_modifier(id)
                                .is_some_and(|o| o == "wait");
                            if !already_waited {
                                gs.mods.add_orientation_modifier(id, "wait");
                            }
                        }
                    } else {
                        let target = cost.target.as_deref().unwrap_or("self");
                        let count = cost.count.unwrap_or(1) as usize;
                        let exclude_self = cost.exclude_self_any().unwrap_or(false);
                        let state_change_binding = cost.state_change_any();
                        let state_change = state_change_binding.unwrap_or("");

                        let candidates = get_change_state_candidates(
                            gs,
                            target,
                            cost.card_type_any().as_deref(),
                            cost.group_names_any(),
                            exclude_self,
                            false,
                            false,
                            Some("active"),
                        );

                        // Q137 / Rule 1.3.2.1: No active members to wait — cost cannot be paid.
                        if candidates.is_empty() {
                            return Err("No matching members on stage to change state".to_string());
                        }

                        if candidates.len() <= count {
                            for &card_id in &candidates {
                                if state_change == "wait" {
                                    gs.mods.add_orientation_modifier(card_id, "wait");
                                } else if state_change == "rest" || state_change == "rested" {
                                    gs.mods.add_orientation_modifier(card_id, "rest");
                                }
                            }
                        } else {
                            self.pending_choice =
                                Some(
                                    Choice::select_cards(
                                        crate::ability::enums::Zone::Stage.to_str(),
                                        count,
                                        format!(
                                            "Select {} stage member(s) to {}",
                                            count, state_change,
                                        ),
                                        false,
                                    )
                                    .card_type(cost.card_type_any().map(|s| s.to_string()))
                                    .group(cost.group_names_any().clone().map(|v| v.join(",")))
                                    .target_player_id(Some(
                                        cost.target.as_deref().unwrap_or("self").to_string(),
                                    ))
                                    .is_reveal(true)
                                    .build(),
                                );
                            return Ok(());
                        }
                    }
                }
                // Handle sequential_cost sub-costs — pay each after user confirmed
                if let Some(ref costs) = cost.compound.actions {
                    for sub_cost in costs {
                        if sub_cost.state_change_any().as_deref() == Some("wait")
                            && sub_cost.self_cost_any() == Some(true)
                        {
                            if let Some(id) = gs.activating_card {
                                gs.mods.add_orientation_modifier(id, "wait");
                            }
                        } else if let Err(e) = self.pay_cost(gs, sub_cost) {
                            log::debug!("Warning: sub-cost payment error: {}", e);
                        }
                        if self.pending_choice.is_some() {
                            return Ok(());
                        }
                    }
                }
                log::debug!("[OPT_COST] checking cost_type: {:?}, entry_cost: {:?}, entry_effect_action: {:?}",
                    cost.action, gs.entry_cost().is_some(),
                    gs.entry_effect().map(|e| e.action.clone()));
                if cost.action == "place_energy_under_member" {
                    self.execute_place_energy_under_member(
                        gs,
                        cost.count.unwrap_or(1),
                        cost.target.as_deref().unwrap_or("self"),
                        cost.position_any(),
                        false,
                        cost.source.as_deref(),
                        cost.any_number_any().unwrap_or(false),
                    );
                }
            }
            self.pending_choice = None;
            let is_effect_optional = gs.entry_choice_card_no() == Some(ChoiceRoute::OptionalCost);
            log::debug!(
                "[HANDLE_OPT_COST] entry_cost={:?} entry_effect={:?} effect_action={:?}",
                gs.entry_cost().and_then(|c| c.state_change_any()),
                gs.entry_effect().map(|e| e.action.clone()),
                gs.entry_effect().map(|e| e.action.clone())
            );
            log::debug!(
                "[HANDLE_OPT_COST2] entering if: entry_cost.is_some={}",
                gs.entry_cost().is_some()
            );
            let effect_started = gs
                .ability_queue
                .current_entry()
                .is_some_and(|e| e.effect_started);
            if gs.entry_cost().is_some() && !effect_started {
                log::debug!(
                    "[HANDLE_OPT_COST2] inside if: entry_effect.is_some={}",
                    gs.entry_effect().is_some()
                );
                if let Some(effect) = gs.entry_effect().cloned() {
                    log::debug!(
                        "[HANDLE_OPT_COST2] calling execute_effect with action={}",
                        effect.action
                    );
                    // For PlaceEnergyUnderMember, call directly with optional=false
                    // to avoid re-creating the optional cost choice (infinite loop).
                    if effect.action == "place_energy_under_member" {
                        self.execute_place_energy_under_member(
                            gs,
                            effect.energy_count_any().unwrap_or(effect.count_or(1)),
                            effect.target_name(),
                            effect.position_any(),
                            false,
                            effect.source_any(),
                            effect.any_number_any().unwrap_or(false),
                        );
                    } else if let Err(e) = self.execute_effect(gs, &effect) {
                        log::debug!("Failed to execute effect after optional cost: {}", e);
                    }
                    if let Some(entry) = gs.ability_queue.current_entry_mut() {
                        entry.effect_started = true;
                    }
                }
            } else if gs.ability_queue.has_pending_actions() {
                if let Err(e) = self.resume_pending_actions(gs) {
                    log::debug!("Failed to execute action after optional: {}", e);
                }
            } else if is_effect_optional {
                if let Some(effect) = gs.entry_effect().cloned() {
                    let new_count = effect.energy_count_any().unwrap_or(effect.count_or(1));
                    self.execute_place_energy_under_member(
                        gs,
                        new_count,
                        effect.target_name(),
                        effect.position_any(),
                        false,
                        effect.source_any(),
                        effect.any_number_any().unwrap_or(false),
                    );
                }
            }
        }
        Ok(())
    }
}
