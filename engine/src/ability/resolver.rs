use super::debug::AbDebug;
use super::types::{AbilityTraceNode, Choice, EffectPipeline, ExecutionContext, ZoneSnapshot};
use super::util;
use crate::card::{Ability, AbilityCost, AbilityEffect, CardDatabase, Keyword};
use crate::game_state::{GameState, Phase};
use crate::zones::MemberArea;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct AbilityResolver {
    pub pending_choice: Option<Choice>,
    pub looked_at_cards: Vec<i16>,
    pub card_database: Arc<CardDatabase>,
    pub duration_effects: Vec<(String, String)>,
    pub current_ability: Option<crate::card::Ability>,
    pub activating_card_id: Option<i16>,
    pub execution_context: ExecutionContext,
    pub current_effect: Option<AbilityEffect>,
    pub revealed_cost_cards: Vec<i16>,
    pub is_reveal_cost: bool,
    pub last_draw_count: u32,
    pub looked_at_total_count: usize,
    pub selected_cards: Vec<i16>,
    pub selected_area: Option<String>,
    pub moved_cards: Vec<i16>,
    /// Effect target to use in child resolution (set by resume_pending_commands
    /// for pending-command effects from "both" splits).
    pub last_effect_target: Option<String>,
    /// Set by zone selection functions when they create a sub-choice (e.g. SelectPosition
    /// for empty_area). Read by finalize_choice to decide whether to resume pending commands.
    pub sub_choice_created: bool,
    /// Cards pending stage placement after position choice for first card.
    pub pending_stage_cards: Vec<(i16, String)>,
    /// Pipeline carries effect sequencing state explicitly.
    pub pipeline: EffectPipeline,
}

impl AbilityResolver {
    pub fn new(card_database: Arc<CardDatabase>, activating_card_id: Option<i16>) -> Self {
        AbilityResolver {
            pending_choice: None,
            looked_at_cards: Vec::new(),
            card_database: card_database.clone(),
            duration_effects: Vec::new(),
            current_ability: None,
            activating_card_id,
            execution_context: ExecutionContext::None,
            current_effect: None,
            revealed_cost_cards: Vec::new(),
            is_reveal_cost: false,
            last_draw_count: 0,
            looked_at_total_count: 0,
            selected_cards: Vec::new(),
            selected_area: None,
            moved_cards: Vec::new(),
            last_effect_target: None,
            sub_choice_created: false,
            pending_stage_cards: Vec::new(),
            pipeline: {
                let mut p = EffectPipeline::new(card_database);
                p.activating_card_id = activating_card_id;
                p
            },
        }
    }

    pub fn take_looked_at(&mut self) -> Vec<i16> {
        std::mem::take(&mut self.looked_at_cards)
    }

    /// Find matching card indices in a zone, prompt if too many.
    /// Takes &[i16] (read-only — works with Vec, SmallVec, any container).
    /// Returns Ok(Some(indices)) if exact match or fewer.
    /// Returns Ok(None) if too many — sets pending_choice, caller should `return Ok(())`.
    pub fn match_cards_in_zone(
        &mut self,
        cards: &[i16],
        count: usize,
        card_db: &crate::card::CardDatabase,
        card_type: Option<&str>,
        group_name: Option<&str>,
        cost_limit: Option<u32>,
        zone_name: &str,
        _prompt_desc: &str,
    ) -> Result<Option<Vec<usize>>, String> {
        let filter =
            util::filter_from_parts(card_type, group_name, cost_limit, None, None, None, None);
        let idxs = util::matching_indices(cards, card_db, &filter, false);
        if idxs.is_empty() || idxs.len() < count {
            return Err(format!("Not enough cards in {}: need {}", zone_name, count));
        }
        if idxs.len() > count {
            self.pending_choice = Some(
                Choice::select_cards(
                    zone_name.to_string(),
                    0,
                    format!("Select {} card(s) to {} for cost", count, zone_name),
                    true,
                )
                .build(),
            );
            self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
            return Ok(None);
        }
        Ok(Some(idxs.into_iter().rev().take(count).collect()))
    }

    pub fn get_pending_choice(&self) -> Option<&Choice> {
        self.pending_choice.as_ref()
    }

    pub fn can_activate_effect(&self, gs: &mut GameState, effect: &AbilityEffect) -> bool {
        let ctx = super::condition::ConditionContext::with_moved_and_selected(
            gs,
            &self.moved_cards,
            &self.selected_cards,
        );
        let mut dbg = AbDebug::new();
        dbg.effect(effect);
        if let Some(ref activation_condition) = effect.activation_condition_parsed {
            let mut merged_cond = activation_condition.clone();
            // Merge the effect's position info into the condition so it's checked.
            if merged_cond.position.is_none() {
                if let Some(ref pos) = effect.position {
                    merged_cond.position = Some(pos.clone());
                } else if let Some(ref act_pos) = effect.activation_position {
                    merged_cond.activation_position = Some(act_pos.clone());
                }
            }
            if !ctx.evaluate_condition(&merged_cond) {
                return false;
            }
        }
        if let Some(ref condition) = effect.condition {
            if effect.action == "conditional_alternative" {
                // skip — condition is a branch selector, not a gate
            } else {
                let mut cond = condition.clone();
                if cond.position.is_none() {
                    if let Some(ref pos) = effect.position {
                        cond.position = Some(pos.clone());
                    } else if let Some(ref act_pos) = effect.activation_position {
                        cond.activation_position = Some(act_pos.clone());
                    }
                }
                if !ctx.evaluate_condition(&cond) {
                    eprintln!("[CAN_ACTIVATE] condition FAILED for {}: type={:?} location={:?} group={:?} exclude={:?}",
                        effect.action, condition.condition_type, condition.location, condition.group_names, condition.exclude_characters);
                    return false;
                }
            }
        }
        true
    }

    pub fn check_keywords(
        &self,
        gs: &mut GameState,
        keywords: &[Keyword],
        card_position: Option<MemberArea>,
    ) -> bool {
        for keyword in keywords {
            match keyword {
                Keyword::Center => {
                    if card_position != Some(MemberArea::Center) {
                        return false;
                    }
                }
                Keyword::LeftSide => {
                    if card_position != Some(MemberArea::LeftSide) {
                        return false;
                    }
                }
                Keyword::RightSide => {
                    if card_position != Some(MemberArea::RightSide) {
                        return false;
                    }
                }
                Keyword::Turn1 => {
                    if gs.turn_number != 1 {
                        return false;
                    }
                }
                Keyword::Turn2 => {
                    if gs.turn_number != 2 {
                        return false;
                    }
                }
                Keyword::Debut => {
                    if let Some(pos) = card_position {
                        let master = gs.ability_master_id();
                        let stage = if master.as_deref() == Some("player2")
                            || master.as_deref() == Some("p2")
                        {
                            &gs.player2.stage.stage
                        } else {
                            &gs.player1.stage.stage
                        };
                        let card_id = match pos {
                            MemberArea::Center => stage[1],
                            MemberArea::LeftSide => stage[0],
                            MemberArea::RightSide => stage[2],
                        };
                        if card_id == -1 {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                Keyword::LiveStart => {
                    if !matches!(
                        gs.current_phase,
                        Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker
                    ) {
                        return false;
                    }
                }
                Keyword::LiveSuccess => {
                    if !matches!(gs.current_phase, Phase::LiveVictoryDetermination) {
                        return false;
                    }
                }
                Keyword::PositionChange => {
                    return gs.position_change_occurred_this_turn;
                }
                Keyword::FormationChange => {
                    return gs.formation_change_occurred_this_turn;
                }
            }
        }
        true
    }

    pub(crate) fn store_pending_choice(&mut self, gs: &mut GameState) {
        if let Some(ref choice) = self.pending_choice {
            let mut json = choice.to_frontend_json();
            if let Some(ref mut j) = json {
                if let Some(entry) = gs.ability_queue.current_entry() {
                    if let Some(ref effect) = entry.ability.effect {
                        if let Some(ref maker) = effect.choice_maker {
                            if let Some(obj) = j.as_object_mut() {
                                obj.insert(
                                    "choice_maker".to_string(),
                                    serde_json::Value::String(maker.clone()),
                                );
                            }
                        }
                    }
                }
                // Inject selection_cards for SelectCard choices
                if let Choice::SelectCard {
                    ref zone,
                    ref card_type,
                    cost_limit,
                    ref cost_limit_operator,
                    ref target_player_id,
                    ..
                } = choice
                {
                    let target = target_player_id.as_deref().unwrap_or("self");
                    let card_ids: Vec<i16> = {
                        let player = gs.resolve_target_player_mut(target);
                        match zone.as_str() {
                            "hand" => player.hand.cards.iter().copied().collect(),
                            "discard" => player.waitroom.cards.iter().copied().collect(),
                            "stage" => player
                                .stage
                                .stage
                                .iter()
                                .copied()
                                .filter(|&id| id != -1)
                                .collect(),
                            "energy_zone" => player.energy_zone.cards.iter().copied().collect(),
                            _ => Vec::new(),
                        }
                    };
                    let filtered: Vec<i16> = card_ids
                        .into_iter()
                        .filter(|&cid| {
                            // card_type filter
                            let type_ok = match card_type.as_deref() {
                                Some("member_card") => gs
                                    .card_database
                                    .get_card(cid)
                                    .map(|c| c.is_member())
                                    .unwrap_or(false),
                                Some("live_card") => gs
                                    .card_database
                                    .get_card(cid)
                                    .map(|c| c.is_live())
                                    .unwrap_or(false),
                                Some("energy_card") => gs
                                    .card_database
                                    .get_card(cid)
                                    .map(|c| c.is_energy())
                                    .unwrap_or(false),
                                None => true,
                                _ => true,
                            };
                            if !type_ok {
                                return false;
                            }
                            // per-card cost_limit filter (not sum cost_total)
                            if let Some(lim) = cost_limit {
                                return crate::ability::util::card_matches_cost_limit_op(
                                    &gs.card_database,
                                    cid,
                                    Some(*lim),
                                    cost_limit_operator.as_deref(),
                                );
                            }
                            true
                        })
                        .collect();
                    let sel: Vec<serde_json::Value> = filtered
                        .iter()
                        .map(|&cid| {
                            let card = gs.card_database.get_card(cid);
                            serde_json::json!({
                                "id": cid,
                                "card_no": card.map(|c| c.card_no.clone()).unwrap_or_default(),
                                "name": card.map(|c| c.name.clone()).unwrap_or_default(),
                            })
                        })
                        .collect();
                    if let Some(obj) = j.as_object_mut() {
                        obj.insert("selection_cards".into(), serde_json::Value::Array(sel));
                    }
                }
                gs.inject_choice_ability_context(j);
            }
        }
    }

    pub fn resolve_ability(
        &mut self,
        gs: &mut GameState,
        ability: &Ability,
        activating_card: Option<i16>,
        ability_index: usize,
    ) -> Result<(), String> {
        let mut dbg = AbDebug::new();

        // Card info for debug
        let card_data = activating_card.and_then(|id| gs.card_database.get_card(id));
        let card_name = card_data.map(|c| c.name.as_str()).unwrap_or("unknown");
        let card_no = card_data.map(|c| c.card_no.as_str()).unwrap_or("");
        let card_id_str = activating_card.map(|id| id.to_string()).unwrap_or_default();

        // Initialize root trace node with ability information
        self.pipeline.trace.label = format!(
            "ability[{}]: {}",
            ability_index,
            ability.full_text.chars().take(60).collect::<String>()
        );
        self.pipeline.trace.card = Some(card_name.to_string());
        self.pipeline.trace.before = Some(ZoneSnapshot::from_game_state(gs));

        dbg.ability(card_name, card_no, &card_id_str, ability);

        // Check use_limit before cost, but don't insert until after effect runs
        let ability_key = activating_card
            .map(|card_id| format!("{}_{}_{}", card_id, ability_index, gs.turn_number));

        if let Some(ref key) = ability_key {
            if let Some(use_limit) = ability.use_limit {
                if gs.turn_limited_abilities_used.contains(key) {
                    let msg = format!("Ability already used this turn (use_limit: {})", use_limit);
                    dbg.p("RESULT", &msg);
                    return Err(msg);
                }
            }
        }

        // Check activation keywords (center/left/right/turn position restrictions)
        if let Some(card_id) = activating_card {
            let position = gs.find_card_stage_position(card_id);
            if !self.check_keywords(gs, ability.keywords.as_ref().unwrap_or(&vec![]), position) {
                return Err(
                    "Activation keywords not satisfied (e.g. card not at required position)"
                        .to_string(),
                );
            }
        }
        self.current_ability = Some(ability.clone());
        gs.activating_card = activating_card;

        let cost_already_paid = gs
            .ability_queue
            .current_entry()
            .map_or(false, |e| e.cost_paid);

        if !cost_already_paid {
            if let Some(ref cost) = ability.cost {
                let cost = self.apply_modify_cost_to_ability_cost(gs, cost, ability);
                if let Err(e) = self.pay_cost(gs, &cost) {
                    dbg.p("RESULT", format_args!("COST FAILED: {}", e));
                    return Err(e);
                }
                dbg.p("RESULT", "cost paid ✓");
            }
        }

        // Record use_limit early only if the effect's conditions can be met.
        // This prevents consuming use_limit on premature triggers (e.g. auto
        // abilities queued eagerly before their trigger condition is satisfied).
        if !cost_already_paid && self.pending_choice.is_none() {
            if let Some(ref key) = ability_key {
                if ability.use_limit.is_some() {
                    let can_activate = ability
                        .effect
                        .as_ref()
                        .map_or(true, |e| self.can_activate_effect(gs, e));
                    if can_activate {
                        eprintln!("[USE_LIMIT] inserting key={}", key);
                        gs.turn_limited_abilities_used.insert(key.clone());
                    }
                }
            }
        }

        if self.pending_choice.is_some() {
            if !cost_already_paid {
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.cost_paid = true;
                }
            }
            self.store_pending_choice(gs);
            return Ok(());
        }

        // Check position keywords (Center/LeftSide/RightSide) AFTER cost payment.
        // Position checks gate the effect, not the cost — the test expects cost to still be paid.
        if let Some(card_id) = activating_card {
            let position = gs
                .player1
                .stage
                .stage
                .iter()
                .position(|&id| id == card_id)
                .or_else(|| gs.player2.stage.stage.iter().position(|&id| id == card_id))
                .map(|idx| match idx {
                    0 => crate::zones::MemberArea::LeftSide,
                    1 => crate::zones::MemberArea::Center,
                    _ => crate::zones::MemberArea::RightSide,
                });
            if let Some(ref kws) = ability.keywords {
                for kw in kws {
                    let pos_ok = match kw {
                        Keyword::Center => position == Some(crate::zones::MemberArea::Center),
                        Keyword::LeftSide => position == Some(crate::zones::MemberArea::LeftSide),
                        Keyword::RightSide => position == Some(crate::zones::MemberArea::RightSide),
                        _ => true,
                    };
                    if !pos_ok {
                        dbg.p("RESULT", "position requirement not met — effect skipped");
                        return Ok(());
                    }
                }
            }
        }

        // Mark cost as paid when it auto-resolved without creating a pending choice.
        if !cost_already_paid && ability.cost.is_some() && self.pending_choice.is_none() {
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.cost_paid = true;
            }
        }

        if let Some(ref effect) = ability.effect {
            // Check the effect's condition BEFORE executing. The condition must
            // be met in the current game state (after cost payment). This prevents
            // effects like "choice" from being shown when the condition fails.
            if effect.condition.is_some() || effect.activation_condition_parsed.is_some() {
                if !self.can_activate_effect(gs, effect) {
                    dbg.p("RESULT", "effect condition not met — skipped");
                    return Ok(());
                }
            }
            if let Err(e) = self.execute_effect(gs, effect) {
                dbg.p("RESULT", format_args!("EFFECT FAILED: {}", e));
                return Err(e);
            }
            eprintln!(
                "[AFTER_EXEC] pending={:?} action={:?}",
                self.pending_choice.is_some(),
                &effect.action[..20.min(effect.action.len())]
            );
            if self.pending_choice.is_some() {
                if !cost_already_paid {
                    if let Some(entry) = gs.ability_queue.current_entry_mut() {
                        entry.cost_paid = true;
                    }
                    if let Some(ref key) = ability_key {
                        if ability.use_limit.is_some() {
                            eprintln!("[USE_LIMIT] inserting key={}", key);
                            gs.turn_limited_abilities_used.insert(key.clone());
                        }
                    }
                }
                // Mark effect as started when a pending choice comes from effect
                // execution (not cost). This prevents RWC from re-entering the
                // ability after the effect's choice resolves.
                let is_paid = cost_already_paid
                    || gs
                        .ability_queue
                        .current_entry()
                        .map_or(false, |e| e.cost_paid);
                if is_paid {
                    if let Some(entry) = gs.ability_queue.current_entry_mut() {
                        entry.effect_started = true;
                    }
                }
                self.store_pending_choice(gs);
                return Ok(());
            }
            dbg.p("RESULT", "effect applied ✓");
        }

        if !cost_already_paid {
            if let Some(ref key) = ability_key {
                if ability.use_limit.is_some() {
                    let can_activate = ability
                        .effect
                        .as_ref()
                        .map_or(true, |e| self.can_activate_effect(gs, e));
                    if can_activate {
                        eprintln!("[USE_LIMIT] inserting key={}", key);
                        gs.turn_limited_abilities_used.insert(key.clone());
                    }
                }
            }
        }
        if let Some(key) = ability_key {
            if ability.use_limit.is_some() {
                let can_activate = ability
                    .effect
                    .as_ref()
                    .map_or(true, |e| self.can_activate_effect(gs, e));
                if can_activate {
                    gs.turn_limited_abilities_used.insert(key);
                }
            }
        }

        gs.activating_card = None;
        self.current_ability = None;

        // Finalize root trace with final state
        self.pipeline.trace.after = Some(ZoneSnapshot::from_game_state(gs));

        Ok(())
    }

    /// Record the start of an effect execution to the trace.
    pub fn trace_effect_start(
        &mut self,
        gs: &GameState,
        effect_name: &str,
        card_name: Option<String>,
    ) {
        let before = ZoneSnapshot::from_game_state(gs);
        let node = AbilityTraceNode::new(effect_name)
            .with_card(card_name)
            .with_before(before);
        self.pipeline.trace.add_child(node);
    }

    /// Record the end of an effect execution (update after state in the last trace node).
    pub fn trace_effect_end(&mut self, gs: &GameState) {
        let after = ZoneSnapshot::from_game_state(gs);
        if let Some(last_child) = self.pipeline.trace.children.last_mut() {
            last_child.after = Some(after);
        }
    }

    /// Get a reference to the current trace node for adding details.
    pub fn get_trace_node(&mut self) -> &mut AbilityTraceNode {
        &mut self.pipeline.trace
    }

    pub fn card_matches_type(
        &self,
        gs: &mut GameState,
        card_id: i16,
        card_type_filter: Option<&str>,
    ) -> bool {
        util::card_matches_type(&gs.card_database, card_id, card_type_filter)
    }

    pub fn card_matches_group(
        &self,
        gs: &mut GameState,
        card_id: i16,
        group_filter: Option<&String>,
    ) -> bool {
        util::card_matches_group(&gs.card_database, card_id, group_filter)
    }

    pub fn card_matches_cost_limit(
        &self,
        gs: &mut GameState,
        card_id: i16,
        cost_limit: Option<u32>,
    ) -> bool {
        util::card_matches_cost_limit(&gs.card_database, card_id, cost_limit)
    }

    /// Walk the ability's effect tree to find modify_cost sub-actions and adjust the cost.
    /// Handles patterns like "コストはグループ名1種類につきE減る" (cost reduced per group name).
    fn apply_modify_cost_to_ability_cost(
        &self,
        gs: &mut GameState,
        cost: &AbilityCost,
        ability: &Ability,
    ) -> AbilityCost {
        let mut cost = cost.clone();
        if let Some(ref effect) = ability.effect {
            if let Some(mod_cost) = util::find_modify_cost(effect, None, None) {
                if mod_cost.operation.as_deref() == Some("subtract") {
                    if mod_cost.per_unit.unwrap_or(false) {
                        if mod_cost.per_unit_type.as_deref() == Some("group_name") {
                            // Count distinct group names on self's stage
                            let player = gs.resolve_target_player("self");
                            let card_db = &gs.card_database;
                            let mut groups = std::collections::HashSet::new();
                            for &cid in &player.stage.stage {
                                if cid == -1 {
                                    continue;
                                }
                                if let Some(card) = card_db.get_card(cid) {
                                    if let Some(ref unit) = card.unit {
                                        groups.insert(unit.clone());
                                    }
                                }
                            }
                            let per_unit_count = mod_cost.per_unit_count.unwrap_or(1) as u32;
                            let reduction = (groups.len() as u32 / per_unit_count)
                                * mod_cost.count.unwrap_or(1);
                            if cost.cost_type.as_deref() == Some("pay_energy") {
                                let new_energy = cost.energy.unwrap_or(0).saturating_sub(reduction);
                                cost.energy = Some(new_energy);
                            }
                        }
                    }
                }
            }
        }
        cost
    }

    pub fn card_db(&self) -> Arc<CardDatabase> {
        self.card_database.clone()
    }
}
