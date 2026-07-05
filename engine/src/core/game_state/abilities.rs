/// Check whether a prohibition_effects entry of the form
/// "restriction:cannot_place:<destination>" blocks placing in the given zone.
/// LiveCardZone and SuccessLiveZone are interchangeable for placement purposes.
fn _prohibition_destination_blocks(prohibition: &str, zone: &str) -> bool {
    let parts: Vec<&str> = prohibition.split(':').collect();
    if parts.len() < 3 {
        return false;
    }
    let dest = parts[2];
    if dest.is_empty() {
        // No destination specified — assume the restriction targets the
        // success live card zone (the most common use case for dynamic
        // cannot_place restrictions like メビウスループ).
        return zone == Zone::SuccessLiveZone.to_str();
    }
    let dest_zone = Zone::from_str(dest);
    let target_zone = Zone::from_str(zone);
    dest_zone == target_zone
        || (dest_zone == Some(Zone::LiveCardZone) && target_zone == Some(Zone::SuccessLiveZone))
        || (dest_zone == Some(Zone::SuccessLiveZone) && target_zone == Some(Zone::LiveCardZone))
}

impl GameState {
    fn stage_card_ids(&self) -> impl Iterator<Item = i16> + '_ {
        self.player1
            .stage
            .stage
            .iter()
            .chain(self.player2.stage.stage.iter())
            .copied()
            .filter(|cid| *cid != -1)
    }

    pub(crate) fn ability_matches_trigger(
        ability: &crate::card::Ability,
        trigger: &crate::game_state::AbilityTrigger,
    ) -> bool {
        ability.triggers.as_ref().is_some_and(|t| match trigger {
            crate::game_state::AbilityTrigger::Activation => {
                t.contains(crate::triggers::ACTIVATION)
            }
            crate::game_state::AbilityTrigger::Debut => {
                t.contains(crate::triggers::DEBUT) || t.contains(crate::triggers::DEBUT_EN)
            }
            crate::game_state::AbilityTrigger::LiveStart => t.contains(crate::triggers::LIVE_START),
            crate::game_state::AbilityTrigger::LiveSuccess => {
                t.contains(crate::triggers::LIVE_SUCCESS)
            }
            crate::game_state::AbilityTrigger::Constant => t.contains(crate::triggers::CONSTANT),
            crate::game_state::AbilityTrigger::Auto => t.contains(crate::triggers::AUTO),
        })
    }

    fn build_ability_queue_entry(
        &self,
        card_no: String,
        ability_index: usize,
        ability: crate::card::Ability,
        card_id: Option<i16>,
        player_id: String,
        trigger_type: AbilityTrigger,
        trigger_moved_cards: Option<Vec<i16>>,
        triggering_member_id: Option<i16>,
    ) -> crate::ability_queue::AbilityQueueEntry {
        use crate::ability_queue::{AbilityId, AbilityQueueEntry};

        AbilityQueueEntry {
            id: AbilityId::new(&card_no, ability_index, &format!("{:?}", trigger_type)),
            card_no,
            player_id: match player_id.as_str() {
                "player1" => "p1".to_string(),
                "player2" => "p2".to_string(),
                other => other.to_string(),
            },
            ability,
            ability_index,
            card_id,
            trigger_type,
            completed: false,
            cost_paid: false,
            cost_paid_index: 0,
            pending_choice_result: None,
            choice_card_no: None,
            conditional_choice: None,
            effect_started: false,
            optional_cost_result: None,
            choice_player_id: None,
            pending_actions: Vec::new(),
            resolver: None,
            trigger_moved_cards,
            triggering_member_id,
            snapshot_movements: Vec::new(),
            snapshot_energy_placed_by_effect: false,
            snapshot_energy_placed_by_player: None,
            choice_effect_text: None,
            condition_cache: std::collections::HashMap::new(),
        }
    }

    pub(crate) fn collect_constant_hand_effects(&self) -> Vec<(i16, crate::card::AbilityEffect)> {
        let mut effects = Vec::new();
        for cid in self
            .player1
            .hand
            .cards
            .iter()
            .chain(self.player2.hand.cards.iter())
        {
            let card = match self.card_database.get_card(*cid) {
                Some(card) => card,
                None => continue,
            };
            for ability in &card.abilities {
                if Self::ability_matches_trigger(
                    ability,
                    &crate::game_state::AbilityTrigger::Constant,
                ) {
                    if let Some(ref effect) = ability.effect {
                        effects.push((*cid, effect.clone()));
                    }
                }
            }
        }
        effects
    }

    pub(crate) fn collect_constant_stage_effects(&self) -> Vec<(i16, crate::card::AbilityEffect)> {
        let mut effects = Vec::new();
        for cid in self.stage_card_ids() {
            let card = match self.card_database.get_card(cid) {
                Some(card) => card,
                None => continue,
            };
            for ability in &card.abilities {
                if Self::ability_matches_trigger(
                    ability,
                    &crate::game_state::AbilityTrigger::Constant,
                ) {
                    if let Some(ref effect) = ability.effect {
                        effects.push((cid, effect.clone()));
                    }
                }
            }
        }
        effects
    }

    /// Scan a player's stage and enqueue auto abilities for that player.
    /// Guards against triggering discard-location abilities when the card isn't in discard.
    ///
    /// Trigger types scanned:
    ///   Stage cards: all auto abilities.  The `condition` field is evaluated
    ///     during scanning for movement/appearance sub-types to ensure the
    ///     triggering event has actually occurred ("このメンバーがエリアを移動したとき"
    ///     should not queue if the card hasn't moved).
    ///   Live cards: non-each_time auto abilities only.
    ///     all auto abilities (including each_time) handled by one TAS scan.
    ///
    /// Called from:
    ///   - process_current_ability() post-resolve scan (line ~753)
    ///   - process_player_abilities() post-loop batch scan (line ~503)
    ///   - execute_performance_phase() for yell/performance triggers (line ~350)
    ///   - debut placement in phases.rs
    ///   - state change effects in effects/state.rs
    /// Check if a condition describes an event that can be evaluated at
    /// scanning time.  Event-based conditions depend on tracking flags
    /// (recently_moved_cards, cards_moved_this_turn, cards_appeared_this_turn)
    /// that are set before TAS runs.  Other types (state, position, group,
    /// comparison) depend on game state that may change between TAS and
    /// ability resolution, so they are deferred.
    fn condition_is_event_based(condition: &crate::card::Condition) -> bool {
        let movement = condition.movement.as_deref();
        // All movement types ("moved", "moves", and "position_change") are event-based.
        // "moved" → card has already moved (can be checked now).
        // "moves" → card is / was moving (checkable because we set
        //   activating_card to the scanned card, and cards_moved_this_turn
        //   is persistent across the turn).
        // "position_change" → card changed position on stage (detected via
        //   explicit PositionChangeEvent records, no snapshot dependency).
        if movement == Some("moved")
            || movement == Some("moves")
            || movement == Some("position_change")
            || movement == Some("baton_touch")
            || movement == Some("live_success")
        {
            return true;
        }
        // Appearance — NOT pre-filtered. Evaluated at resolution time via
        // can_activate_effect so that both self-triggers (Q245) and
        // group-matching are handled correctly with full resolution context.
        // card_count (all variants — zone counts are stable state checks)
        // Exception: conditions on revealed_cards are NOT event-based because
        // revealed_cards is populated during yell, not at the trigger event.
        if matches!(
            condition.condition_type,
            Some(crate::ability::enums::ConditionType::CardCountCondition)
        ) && condition.location.as_deref() != Some("revealed_cards")
        {
            return true;
        }
        // State change (active↔wait) — pre-filter so the condition only
        // fires when a recorded transition is available.
        if matches!(
            condition.condition_type,
            Some(crate::ability::enums::ConditionType::StateChangeCondition)
        ) {
            return true;
        }
        // Recurse into compound conditions — if any child is event-based,
        // the whole compound is pre-filtered.
        if let Some(ref children) = condition.conditions {
            if children.iter().any(Self::condition_is_event_based) {
                return true;
            }
        }
        false
    }

    /// Legacy wrapper: calls with default event (reads flags from self).
    pub fn trigger_auto_abilities_for_player(&mut self, player_id: &str) {
        let event = crate::ability::types::TriggerEvent {
            moved_cards: self.recently_moved_cards.clone().unwrap_or_default(),
            moved_from_zone: self.recently_moved_from_zone.clone(),
            ..Default::default()
        };
        self.trigger_auto_abilities_for_player_with_event(player_id, &event);
    }

    // Q58: Two copies of the same member with "once per turn" can each use the ability once per turn.
    // Q59: A card that changes zones (except stage-to-stage) is treated as new; its once-per-turn resets.
    // Q60: A non-once-per-turn auto ability that triggers must be used (cannot opt out).
    // Q61: A once-per-turn auto ability can be skipped at one trigger timing to save it for later.
    /// Core TAS implementation that takes an explicit TriggerEvent.
    /// Callers should construct and pass the event so the scan has
    /// accurate context about what triggered it.
    pub fn trigger_auto_abilities_for_player_with_event(
        &mut self,
        player_id: &str,
        event: &crate::ability::types::TriggerEvent,
    ) {
        let player_id_clone = player_id.to_string();
        let mut abilities_to_trigger: Vec<(String, String, i16)> = Vec::new();
        let skip_this_card_auto_key = self.just_completed_ability_key.clone();
        {
            let player = if player_id_clone == self.player1.id {
                &self.player1
            } else {
                &self.player2
            };
            // Scan stage cards for AUTO abilities
            for (stage_idx, &card_id) in player.stage.stage.iter().enumerate() {
                let card_position = match stage_idx {
                    0 => crate::zones::MemberArea::LeftSide,
                    1 => crate::zones::MemberArea::Center,
                    _ => crate::zones::MemberArea::RightSide,
                };
                if card_id == -1 {
                    continue;
                }
                if let Some(card) = self.card_database.get_card(card_id) {
                    for (ability_idx, ability) in card.abilities.iter().enumerate() {
                        if !crate::zones::check_effect_position(
                            ability
                                .effect
                                .as_ref()
                                .and_then(|e| e.activation_position.as_deref()),
                            card_position,
                        ) {
                            continue;
                        }
                        if ability
                            .triggers
                            .as_ref()
                            .is_some_and(|t| t == crate::triggers::AUTO)
                        {
                            let mut trigger_multiplicity: u32 = 1;
                            // Guard: skip discard-location abilities when the card
                            // is on stage (prevents premature triggering).
                            if let Some(ref effect) = ability.effect {
                                if crate::ability::debug::ABILITY_DEBUG
                                    .load(std::sync::atomic::Ordering::Relaxed)
                                {
                                    log::debug!(
                                        "[TAS] scanning trigger={:?} cond={}",
                                        ability.triggers,
                                        effect.condition.is_some(),
                                    );
                                }
                                if let Some(ref condition) = effect.condition {
                                    // Guard: skip discard-location abilities when the
                                    // card is on stage (prevents premature triggering
                                    // of "this card is in discard" abilities).
                                    // BUT: skip this guard for "preceding_moved"
                                    // watchers — those track OTHER cards moving to
                                    // discard, not the card itself being in discard.
                                    let cond_location = condition
                                        .location
                                        .as_deref()
                                        .or_else(|| {
                                            condition
                                                .trigger_event
                                                .as_ref()
                                                .and_then(|t| t.location.as_deref())
                                        })
                                        .unwrap_or("");
                                    if condition.source.as_deref() != Some("preceding_moved")
                                        && Zone::from_str(cond_location) == Some(Zone::Discard)
                                        && (condition.card_type.as_deref() == Some("member_card")
                                            || condition.target.as_deref() == Some("self"))
                                    {
                                        let in_discard =
                                            self.player1.waitroom.cards.contains(&card_id)
                                                || self.player2.waitroom.cards.contains(&card_id);
                                        if !in_discard {
                                            continue;
                                        }
                                    }
                                }
                                // Pre-filter: evaluate conditions during scanning
                                // to prevent queuing auto abilities whose trigger
                                // event hasn't occurred.  Only pre-filter event-
                                // based condition types:
                                //   - movement ("moved" / "moves")
                                //   - appearance
                                //   - card_count (all variants)
                                // Other types (state, position, group, comparison,
                                // state_change) depend on game state or events
                                // that may change between TAS and ability
                                // resolution, so they are deferred.
                                if let Some(ref condition) = effect.condition {
                                    let can_prefilter = Self::condition_is_event_based(condition);
                                    if can_prefilter {
                                        let saved_activating = self.activating_card;
                                        self.activating_card = Some(card_id);
                                        let ctx = crate::ability::condition::ConditionContext::with_moved_cards(self, &event.moved_cards);
                                        let passes = ctx.evaluate_condition(condition);
                                        self.activating_card = saved_activating;
                                        if crate::ability::debug::ABILITY_DEBUG
                                            .load(std::sync::atomic::Ordering::Relaxed)
                                        {
                                            log::debug!(
                                                "[TAS_COND] card={} cond_type={:?} passes={}",
                                                card.name,
                                                condition.condition_type,
                                                passes
                                            );
                                        }
                                        if !passes {
                                            continue;
                                        }
                                    }
                                    // Heuristic guard: each_time abilities whose
                                    // condition is a comparison on energy_zone must
                                    // also require energy was placed by a card effect
                                    // (the flag is consumed after every TAS scan to
                                    // prevent re-triggering on stale comparisons
                                    // like "energy_zone >= 0" during phase-based
                                    // energy placement).
                                    if effect.trigger_type.as_deref() == Some("each_time") {
                                        if condition.condition_type
                                            == Some(crate::ability::enums::ConditionType::ComparisonCondition)
                                            && condition.location.as_deref() == Some("energy_zone")
                                            && !self.last_energy_placed_by_effect()
                                        {
                                            continue;
                                        }
                                    }
                                }
                                // §9.7.2.1: Compute trigger multiplicity before
                                // the effect block closes — condition and effect
                                // are only in scope here.
                                trigger_multiplicity = Self::trigger_instance_count(
                                    &event.moved_cards,
                                    effect,
                                    &self.card_database,
                                );
                            }
                            // During in-execution scans (e.g. state.rs state-change),
                            // skip the exact same ability on the same card to prevent
                            // self-re-triggering. Different abilities on the same card
                            // (e.g. Maki debut ab#0 vs auto ab#1) can fire normally.
                            if self.activating_card == Some(card_id)
                                && self.activating_ability_index == Some(ability_idx)
                            {
                                continue;
                            }
                            // Movement gate for "was placed" (置かれた) triggers:
                            // self_target + single-location + movement:"moved" requires
                            // the card to be in event.moved_cards (recently placed).
                            if let Some(ref eff) = ability.effect {
                                if let Some(ref cond) = eff.condition {
                                    if cond.self_target.unwrap_or(false)
                                        && cond.movement.as_deref() == Some("moved")
                                        && cond.locations.as_ref().map_or(true, |l| l.len() < 2)
                                        && !event.moved_cards.contains(&card_id)
                                    {
                                        continue;
                                    }
                                }
                            }
                            let ability_id = format!("{}_{}", card.card_no, ability.full_text);
                            // Batch-scoped guard: use instance-aware key so
                            // different copies of the same card (P1 vs P2) are
                            // not blocked by each other's batch dedup.
                            let batch_key =
                                format!("{}_{}_{}", card_id, card.card_no, ability.full_text);
                            // Re-scan guard: skip re-enqueueing the exact auto
                            // ability that just completed.
                            if skip_this_card_auto_key.as_deref() == Some(&ability_id) {
                                continue;
                            }
                            // Batch-scoped guard: prevent re-enqueue of any ability
                            // already triggered during this movement batch.
                            if self.this_batch_triggered_ability_ids.contains(&batch_key) {
                                continue;
                            }
                            self.this_batch_triggered_ability_ids.insert(batch_key);
                            // §9.7.2.1: Multi-trigger — N trigger instances → N
                            // standby entries.  All entries share the same
                            // trigger_moved_cards (full batch) because each
                            // instance independently re-evaluates the condition
                            // at resolution time via can_activate_effect.
                            for _ in 0..trigger_multiplicity {
                                abilities_to_trigger.push((
                                    ability_id.clone(),
                                    card.card_no.clone(),
                                    card_id,
                                ));
                            }
                        }
                    }
                }
            }
            // Also scan live cards for AUTO abilities
            for &card_id in &player.live_card_zone.cards {
                if let Some(card) = self.card_database.get_card(card_id) {
                    for ability in &card.abilities {
                        if ability
                            .triggers
                            .as_ref()
                            .is_some_and(|t| t == crate::triggers::AUTO)
                        {
                            if let Some(ref effect) = ability.effect {
                                // Live card scan — uses the same event-based
                                // condition check as stage cards.
                                if let Some(ref condition) = effect.condition {
                                    if Self::condition_is_event_based(condition) {
                                        let saved_activating = self.activating_card;
                                        self.activating_card = Some(card_id);
                                        let ctx = crate::ability::condition::ConditionContext::with_moved_cards(self, &event.moved_cards);
                                        let passes = ctx.evaluate_condition(condition);
                                        self.activating_card = saved_activating;
                                        if !passes {
                                            continue;
                                        }
                                    } else if condition.self_target.unwrap_or(false) {
                                        if let Some(ref locs) = condition.locations {
                                            if locs.len() == 2 {
                                                continue;
                                            }
                                        }
                                    }
                                }
                            }
                            // Same movement gate for live cards:
                            if let Some(ref eff) = ability.effect {
                                if let Some(ref cond) = eff.condition {
                                    if cond.self_target.unwrap_or(false)
                                        && cond.movement.as_deref() == Some("moved")
                                        && cond.locations.as_ref().map_or(true, |l| l.len() < 2)
                                        && !event.moved_cards.contains(&card_id)
                                    {
                                        continue;
                                    }
                                }
                            }
                            let ability_id = format!("{}_{}", card.card_no, ability.full_text);
                            let batch_key =
                                format!("{}_{}_{}", card_id, card.card_no, ability.full_text);
                            if skip_this_card_auto_key.as_deref() == Some(&ability_id) {
                                continue;
                            }
                            if self.this_batch_triggered_ability_ids.contains(&batch_key) {
                                continue;
                            }
                            self.this_batch_triggered_ability_ids.insert(batch_key);
                            abilities_to_trigger.push((ability_id, card.card_no.clone(), card_id));
                        }
                    }
                }
            }
            // Also scan recently-moved cards for AUTO abilities (replaces
            // the ad-hoc trigger_auto_for_discarded_cards pattern matching).
            // Only enqueue for the card's actual owner (not the scanner).
            // Skip cards already on stage or in live zone (scanned separately).
            for &moved_card_id in &event.moved_cards {
                if self.player1.stage.stage.contains(&moved_card_id)
                    || self.player1.live_card_zone.cards.contains(&moved_card_id)
                    || self.player2.stage.stage.contains(&moved_card_id)
                    || self.player2.live_card_zone.cards.contains(&moved_card_id)
                {
                    continue;
                }
                if let Some(card) = self.card_database.get_card(moved_card_id) {
                    // Determine card owner by zone membership
                    let is_p1 = self.player1.stage.stage.contains(&moved_card_id)
                        || self.player1.hand.cards.contains(&moved_card_id)
                        || self.player1.live_card_zone.cards.contains(&moved_card_id)
                        || self.player1.energy_zone.cards.contains(&moved_card_id)
                        || self.player1.waitroom.cards.contains(&moved_card_id);
                    let is_p2 = self.player2.stage.stage.contains(&moved_card_id)
                        || self.player2.hand.cards.contains(&moved_card_id)
                        || self.player2.live_card_zone.cards.contains(&moved_card_id)
                        || self.player2.energy_zone.cards.contains(&moved_card_id)
                        || self.player2.waitroom.cards.contains(&moved_card_id);
                    let card_owner = if is_p1 {
                        "p1"
                    } else if is_p2 {
                        "p2"
                    } else {
                        continue; // can't determine owner, skip
                    };
                    if card_owner != player_id_clone {
                        continue; // card belongs to a different player
                    }
                    for ability in &card.abilities {
                        if ability
                            .triggers
                            .as_ref()
                            .is_some_and(|t| t == crate::triggers::AUTO)
                        {
                            if let Some(ref effect) = ability.effect {
                                if let Some(ref condition) = effect.condition {
                                    // Appearance conditions are for cards ON stage
                                    // (scanned by the stage loop).  Skip them in the
                                    // moved-cards scan so that cards removed from
                                    // stage (e.g. by baton touch) don't falsely fire.
                                    if matches!(
                                        condition.condition_type,
                                        Some(crate::ability::enums::ConditionType::AppearanceCondition)
                                    ) {
                                        continue;
                                    }
                                    let saved_activating = self.activating_card;
                                    self.activating_card = Some(moved_card_id);
                                    let ctx = crate::ability::condition::ConditionContext::with_moved_cards(self, &event.moved_cards);
                                    let passes = ctx.evaluate_condition(condition);
                                    self.activating_card = saved_activating;
                                    if !passes {
                                        continue;
                                    }
                                }
                            }
                            let ability_id = format!("{}_{}", card.card_no, ability.full_text);
                            let batch_key =
                                format!("{}_{}_{}", moved_card_id, card.card_no, ability.full_text);
                            if skip_this_card_auto_key.as_deref() == Some(&ability_id) {
                                continue;
                            }
                            if self.this_batch_triggered_ability_ids.contains(&batch_key) {
                                continue;
                            }
                            self.this_batch_triggered_ability_ids.insert(batch_key);
                            abilities_to_trigger.push((
                                ability_id,
                                card.card_no.clone(),
                                moved_card_id,
                            ));
                        }
                    }
                }
            }
        }
        let moved = Some(event.moved_cards.clone());
        for (ability_id, card_no, stage_card_id) in abilities_to_trigger {
            self.this_batch_triggered_ability_ids
                .insert(ability_id.clone());
            self.trigger_auto_ability(
                ability_id,
                AbilityTrigger::Auto,
                player_id_clone.clone(),
                Some(card_no),
                Some(stage_card_id),
                moved.clone(),
                None,
            );
        }
        // Consume the energy flag after every TAS scan — each event should
        // trigger at most one batch of each_time abilities.  The snapshot
        // captured in trigger_auto_ability (above) preserves the flag value
        // for abilities that need it during execution (e.g. Sumire's "moves").
    }

    /// §9.7.2.1: Count how many standby entries to create for a trigger event.
    ///
    /// For `card_count_condition` with `source: "preceding_moved"`, counts
    /// cards in the event batch matching the condition's filters.  Returns 1
    /// for batch patterns ("すべて", "1枚以上", self_target, count=1+op=>=).
    /// All other condition types return 1 (single standby instance).
    fn trigger_instance_count(
        moved_cards: &[i16],
        effect: &crate::card::AbilityEffect,
        card_db: &crate::card::CardDatabase,
    ) -> u32 {
        let condition = match &effect.condition {
            Some(c) => c,
            None => return 1,
        };
        if condition.condition_type
            != Some(crate::ability::enums::ConditionType::CardCountCondition)
            || condition.source.as_deref() != Some("preceding_moved")
        {
            return 1;
        }
        let matching: Vec<&i16> = moved_cards
            .iter()
            .filter(|&&cid| {
                if cid == -1 {
                    return false;
                }
                if let Some(ref ct) = condition.card_type {
                    if !crate::ability::util::card_matches_type(card_db, cid, Some(ct)) {
                        return false;
                    }
                }
                if let Some(ref hc) = condition.heart_colors {
                    if !hc.is_empty()
                        && !crate::ability::util::card_matches_heart_colors(card_db, cid, hc)
                    {
                        return false;
                    }
                }
                true
            })
            .collect();
        let match_count = matching.len() as u32;
        if match_count <= 1 {
            return match_count;
        }
        let ct = &condition.text;
        if ct.contains("すべて") || ct.contains("全て") || ct.contains("全部") {
            return 1;
        }
        if ct.contains("1枚以上") || ct.contains("1つ以上") {
            return 1;
        }
        if condition.count == Some(1) && condition.operator.as_deref() == Some(">=") {
            return 1;
        }
        if condition.self_target.unwrap_or(false) {
            return 1;
        }
        match_count
    }

    pub fn trigger_auto_ability(
        &mut self,
        ability_id: String,
        trigger_type: AbilityTrigger,
        player_id: String,
        source_card_id: Option<String>,
        explicit_card_id: Option<i16>,
        trigger_moved_cards: Option<Vec<i16>>,
        triggering_member_id: Option<i16>,
    ) {
        if let Some(ref card_no) = source_card_id {
            let (card, card_id) = if let Some(cid) = explicit_card_id {
                (self.card_database.get_card(cid).cloned(), Some(cid))
            } else {
                self.find_card_by_number_for_player(card_no, &player_id)
            };
            if let Some(card) = card {
                // Check original abilities
                let expected_id = |ability: &crate::card::Ability| -> String {
                    format!("{}_{}", card_no, ability.full_text)
                };
                for (ability_index, ability) in card.abilities.iter().enumerate() {
                    if Self::ability_matches_trigger(ability, &trigger_type)
                        && ability_id == expected_id(ability)
                    {
                        let entry = self.build_ability_queue_entry(
                            card_no.clone(),
                            ability_index,
                            ability.clone(),
                            card_id,
                            player_id.clone(),
                            trigger_type,
                            trigger_moved_cards.clone(),
                            triggering_member_id,
                        );
                        // Snapshot batch_movements and energy flags at enqueue
                        // time so the "moves" and energy conditions can check
                        // what triggered the ability even after
                        // clear_effect_tracking clears the global lists.
                        let mut entry = entry;
                        entry.snapshot_movements = self.batch_movements.clone();
                        entry.snapshot_energy_placed_by_effect =
                            self.last_energy_placed_by_effect();
                        entry.snapshot_energy_placed_by_player =
                            self.last_energy_placed_by_player().map(|s| s.to_string());
                        if crate::ability::debug::ABILITY_DEBUG
                            .load(std::sync::atomic::Ordering::Relaxed)
                        {
                            log::debug!(
                                "[QUEUE_DIAG] enqueue player={} card_no={}",
                                entry.player_id,
                                entry.card_no
                            );
                        }
                        self.ability_queue.enqueue(entry);
                        return;
                    }
                }
                // Check gained card abilities (ability_id format: "card_no_gained_{idx}")
                if ability_id.contains("_gained_") {
                    let cid = card_id.or(explicit_card_id);
                    if let Some(card_id_val) = cid {
                        if let Some(gained_list) = self.gained_card_abilities.get(&card_id_val) {
                            // Extract the gained index from ability_id
                            if let Some(idx_str) = ability_id.rsplit('_').next() {
                                if let Ok(gidx) = idx_str.parse::<usize>() {
                                    if let Some(gained_ability) = gained_list.get(gidx) {
                                        if Self::ability_matches_trigger(
                                            gained_ability,
                                            &trigger_type,
                                        ) {
                                            let entry = self.build_ability_queue_entry(
                                                card_no.clone(),
                                                10000 + gidx,
                                                gained_ability.clone(),
                                                Some(card_id_val),
                                                player_id.clone(),
                                                trigger_type,
                                                trigger_moved_cards.clone(),
                                                triggering_member_id,
                                            );
                                            let mut entry = entry;
                                            entry.snapshot_movements = self.batch_movements.clone();
                                            entry.snapshot_energy_placed_by_effect =
                                                self.last_energy_placed_by_effect();
                                            entry.snapshot_energy_placed_by_player = self
                                                .last_energy_placed_by_player()
                                                .map(|s| s.to_string());
                                            self.ability_queue.enqueue(entry);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Search for a card in the specified player's zones first, fall back to the other player.
    fn find_card_by_number_for_player(
        &self,
        card_no: &str,
        player_id: &str,
    ) -> (Option<crate::card::Card>, Option<i16>) {
        let preferred = if player_id == self.player1.id || player_id == "p1" {
            &self.player1
        } else {
            &self.player2
        };
        let other = if std::ptr::eq(preferred, &self.player1) {
            &self.player2
        } else {
            &self.player1
        };
        let result = self.search_player_zones_for_card(card_no, preferred);
        if result.0.is_some() {
            return result;
        }
        self.search_player_zones_for_card(card_no, other)
    }

    fn search_player_zones_for_card(
        &self,
        card_no: &str,
        player: &Player,
    ) -> (Option<crate::card::Card>, Option<i16>) {
        for id in &player.hand.cards {
            if let Some(card) = self.card_database.get_card(*id) {
                if card.card_no == card_no {
                    return (Some(card.clone()), Some(*id));
                }
            }
        }
        for stage_card_id in &player.stage.stage {
            if *stage_card_id != -1 {
                if let Some(card) = self.card_database.get_card(*stage_card_id) {
                    if card.card_no == card_no {
                        return (Some(card.clone()), Some(*stage_card_id));
                    }
                }
            }
        }
        for waitroom_card_id in &player.waitroom.cards {
            if let Some(card) = self.card_database.get_card(*waitroom_card_id) {
                if card.card_no == card_no {
                    return (Some(card.clone()), Some(*waitroom_card_id));
                }
            }
        }
        for live_card_id in &player.live_card_zone.cards {
            if let Some(card) = self.card_database.get_card(*live_card_id) {
                if card.card_no == card_no {
                    return (Some(card.clone()), Some(*live_card_id));
                }
            }
        }
        for success_card_id in &player.success_live_card_zone.cards {
            if let Some(card) = self.card_database.get_card(*success_card_id) {
                if card.card_no == card_no {
                    return (Some(card.clone()), Some(*success_card_id));
                }
            }
        }
        (None, None)
    }

    /// Internal: Process all standby abilities for a single player.
    /// Stops early if an ability creates a pending choice.
    /// Trigger each_time abilities on live cards for a specific member's resolution.
    /// Called after a LiveStart/LiveSuccess ability resolves successfully.
    /// Only fires when the resolved card is a STAGE MEMBER (not a live card in the
    /// live_card_zone), matching the "メンバーの" (member's) condition in each_time text.
    /// Enqueues each matching each_time ability with `triggering_member_id` set to `member_card_id`.
    pub fn trigger_each_time_for_member(
        &mut self,
        player_id: &str,
        trigger_substring: &str,
        member_card_id: i16,
    ) {
        // Only fire for stage member cards — live cards' own LiveStart/LiveSuccess
        // must NOT trigger each_time (each_time watches "メンバーの" = member's abilities).
        let player = if player_id == self.player1.id || player_id == "p1" {
            &self.player1
        } else {
            &self.player2
        };
        let is_on_stage = player.stage.stage.contains(&member_card_id);
        if !is_on_stage {
            return;
        }
        let player_id_clone = player_id.to_string();
        let mut abilities: Vec<(String, String, i16)> = Vec::new();
        for &card_id in &player.live_card_zone.cards {
            if let Some(card) = self.card_database.get_card(card_id) {
                for ability in &card.abilities {
                    if ability.triggers.as_deref() != Some(crate::triggers::AUTO) {
                        continue;
                    }
                    let effect = match &ability.effect {
                        Some(e) => e,
                        None => continue,
                    };
                    if effect.trigger_type.as_deref() != Some("each_time") {
                        continue;
                    }
                    let watch_text = match &effect.condition {
                        Some(c) => &c.text,
                        None => &effect.text,
                    };
                    if !watch_text.contains(trigger_substring) {
                        continue;
                    }
                    let aid = format!("{}_{}", card.card_no, ability.full_text);
                    abilities.push((aid, card.card_no.clone(), card_id));
                }
            }
        }
        for (aid, card_no, cid) in abilities {
            self.trigger_auto_ability(
                aid,
                crate::game_state::AbilityTrigger::Auto,
                player_id_clone.clone(),
                Some(card_no),
                Some(cid),
                None,
                Some(member_card_id),
            );
        }
    }

    fn process_player_abilities(&mut self, raw_player_id: &str) {
        let player_id = match raw_player_id {
            "player1" => "p1",
            "player2" => "p2",
            other => other,
        };
        if crate::ability::debug::ABILITY_DEBUG.load(std::sync::atomic::Ordering::Relaxed) {
            log::debug!(
                "[QUEUE_DIAG] process_player_abilities player={} queue_len={}",
                player_id,
                self.ability_queue.len()
            );
        }
        let mut reprocess_counts: std::collections::HashMap<(i16, usize), u32> =
            std::collections::HashMap::new();
        loop {
            if !self.ability_queue.is_idle() {
                break;
            }

            // Snapshot queue length before resolution. Entries at indices >= pre_len
            // are freshly triggered (each_time watchers) by the current resolution
            // and must be drained depth-first (§9.5.3.2→§9.5.3.1 loopback).
            let pre_len = self
                .depth_first_cutoff
                .unwrap_or_else(|| self.ability_queue.len());
            self.depth_first_cutoff = None;

            let available_indices: Vec<usize> = (0..pre_len)
                .filter(|&i| {
                    self.ability_queue.is_entry_available(i)
                        && self.ability_queue.entry_player_id(i) == Some(player_id)
                })
                .collect();

            if crate::ability::debug::ABILITY_DEBUG.load(std::sync::atomic::Ordering::Relaxed) {
                log::debug!("[QUEUE_DIAG] available_indices={:?}", available_indices);
            }

            if available_indices.is_empty() {
                break;
            }

            if available_indices.len() > 1 {
                let options = available_indices
                    .iter()
                    .filter_map(|&idx| {
                        let entry = self.ability_queue.get_entry(idx)?;
                        let cid = entry.card_id.unwrap_or(0);
                        let card_name = self
                            .card_database
                            .get_card(cid)
                            .map(|c| c.name.clone())
                            .unwrap_or_else(|| entry.card_no.clone());
                        Some(crate::ability::types::AutoAbilityOption {
                            card_name,
                            ability_text: entry.ability.full_text.clone(),
                            queue_index: idx,
                        })
                    })
                    .collect();

                let choice = crate::ability::types::Choice::SelectAutoAbility {
                    player_id: player_id.to_string(),
                    options,
                    description:
                        "複数の自動能力が同時に発動しました。使用する順番を選択してください。"
                            .to_string(),
                    description_en: Some("Multiple auto abilities triggered simultaneously. Choose the order to use them.".to_string()),
                    description_ja: Some("複数の自動能力が同時に発動しました。使用する順番を選択してください。".to_string()),
                };

                self.ability_queue.pause_for_auto_ability_choice(choice);
                break;
            }

            let idx = available_indices[0];
            self.ability_queue.promote_entry_by_abs(idx);
            if !self.ability_queue.start_next() {
                break;
            }

            // Infinite loop guard: track how many times the same ability is processed
            if let Some(entry) = self.ability_queue.current_entry() {
                let key = (entry.card_id.unwrap_or(-1), entry.ability_index);
                let count = reprocess_counts.entry(key).or_insert(0);
                *count += 1;
                if *count > 5 {
                    let card_name = entry
                        .card_id
                        .and_then(|id| self.card_database.get_card(id))
                        .map(|c| c.name.clone())
                        .unwrap_or_default();
                    log::error!(
                        "[PCA_INFINITE_LOOP] card={} ({}) ability=\"{}\" processed {} times",
                        card_name,
                        entry.card_no,
                        entry.ability.full_text,
                        *count
                    );
                    if crate::ability::debug::ABILITY_DEBUG
                        .load(std::sync::atomic::Ordering::Relaxed)
                    {
                        log::debug!(
                            "[PCA_INFINITE_LOOP] card={} ({}) ability=\"{}\" processed {} times",
                            card_name,
                            entry.card_no,
                            entry.ability.full_text,
                            *count
                        );
                    }
                    break;
                }
            }

            self.process_current_ability();
            let had_recent_moves = self.recently_moved_cards.is_some();
            let had_recent_appearances = !self.recently_appeared_cards.is_empty();
            self.recently_moved_cards = None;
            self.recently_appeared_cards.clear();
            self.recently_state_changed.clear();
            self.recently_moved_from_zone = None;
            // Save flag for the post-loop batch scan below;
            // process_current_ability's internal scan (line 742) already ran
            // before the clear above, so each_time watchers from the
            // just-resolved effect were caught. This post-loop scan catches
            // batch movements (look_and_select, etc.) that finalize card
            // movement outside individual ability resolution.
            if had_recent_moves {
                self.recently_moved_cards = Some(Vec::new());
            }
            if had_recent_appearances {
                self.recently_appeared_cards.push(-1);
            }
            if self.has_pending_choice() {
                break;
            }

            // Depth-first drain: newly-triggered each_time watchers at indices >= cutoff
            // (§9.5.3.2→§9.5.3.1 loopback) resolve immediately before the next stale entry.
            // The range widens dynamically as sub-resolutions queue deeper entries.
            let cutoff = pre_len;
            let mut drain_iters = 0;
            while !self.has_pending_choice() && self.ability_queue.is_idle() {
                drain_iters += 1;
                if drain_iters > 50 {
                    log::error!(
                        "[PCA_DRAIN_LIMIT] each_time drain exceeded 50 iterations player={}",
                        player_id
                    );
                    break;
                }
                let new_idx = (cutoff..self.ability_queue.len()).find(|&i| {
                    self.ability_queue.is_entry_available(i)
                        && self.ability_queue.entry_player_id(i) == Some(player_id)
                });
                match new_idx {
                    Some(idx) => {
                        self.ability_queue.set_current_entry(idx);
                        if !self.ability_queue.start_next() {
                            break;
                        }
                        self.process_current_ability();
                        // Sub-resolution may queue deeper entries — while loop catches them
                        // on next iteration (widening range from pre_len..len).
                        if self.has_pending_choice() {
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
        // Q86 / Q252 / Rule 9.5.3.1: Post-loop scan for batch-triggered abilities
        //
        // After the main ability loop, we scan for auto-abilities triggered by
        // batch movements from look_and_select or other compound effects that
        // finalize card movement outside individual ability resolution.
        //
        // Trigger types: each_time:discard, each_time:hand_to_discard,
        //   each_time:any_to_discard, each_time:energy_placed
        //
        // Q86: After look-and-select resolves, if all looked-at cards go to
        //   discard and deck becomes empty, the batch movement triggers
        //   each_time:discard watchers. The post-loop scan catches these.
        //
        // Q252: "When multiple live cards go to discard simultaneously, can
        //   I put all of them back with one trigger?" → No, only 1 card per
        //   trigger instance. The batch scan fires the ability once per batch,
        //   and the ability selects 1 card from the batch.
        let moved_marker = self.recently_moved_cards.is_some();
        let appeared_marker = !self.recently_appeared_cards.is_empty();
        if moved_marker || self.last_energy_placed_by_effect() || appeared_marker {
            let batch_ids: Vec<i16> = self
                .batch_movements
                .iter()
                .map(|m| m.moved_card_id)
                .collect();
            let event = crate::ability::types::TriggerEvent {
                moved_cards: batch_ids,
                energy_placed_by_effect: self.last_energy_placed_by_effect(),
                ..Default::default()
            };
            self.trigger_auto_abilities_for_player_with_event(player_id, &event);
            self.recently_moved_cards = None;
            self.recently_appeared_cards.clear();
            self.recently_state_changed.clear();
            self.recently_moved_from_zone = None;
            // Re-enter the loop to process any abilities just enqueued
            // by the watcher scan (e.g. Hazuki Ren each_time after discard).
            // Keep this_batch_triggered_ability_ids alive through the recursive
            // call so the same ability isn't enqueued twice from stale events.
            if !self.has_pending_choice() {
                self.process_player_abilities(player_id);
            }
            self.batch_movements.clear();
            self.position_change_events.clear();
            self.this_batch_triggered_ability_ids.clear();
        }
    }

    pub fn process_pending_auto_abilities(&mut self, raw_player_id: &str) {
        let active_player_id = match raw_player_id {
            "player1" => "p1",
            "player2" => "p2",
            other => other,
        };
        // Rule 9.5.3.2: Active player resolves ALL their standby abilities first
        // (one at a time, back to rule processing between each)
        self.process_player_abilities(active_player_id);
        if self.has_pending_choice() {
            return;
        }

        // Rule 9.5.3.3: Then non-active player resolves ALL theirs
        let non_active_id = {
            let pending = self.ability_queue.pending_entries();
            pending
                .iter()
                .find(|e| e.player_id != active_player_id)
                .map(|e| e.player_id.clone())
                .unwrap_or_default()
        };
        if !non_active_id.is_empty() {
            self.process_player_abilities(&non_active_id);
        }
        self.ability_queue.clear_completed();
    }

    pub(crate) fn process_current_ability(&mut self) {
        if crate::ability::debug::ABILITY_DEBUG.load(std::sync::atomic::Ordering::Relaxed) {
            log::debug!(
                "[PCA_ENTER] has_resolver={}",
                self.ability_queue.has_resolver()
            );
        }
        let (card_id, ability, ability_index, cost_already_paid) = {
            let entry = match self.ability_queue.current_entry() {
                Some(e) => e,
                None => return,
            };
            (
                entry.card_id,
                entry.ability.clone(),
                entry.ability_index,
                entry.cost_paid,
            )
        };

        // Skip resolution if this card's abilities are negated
        if card_id.is_some() && self.negated_abilities.contains(&card_id.unwrap()) {
            log::debug!(
                "[NEGATED] card_id={:?} is negated — skipping ability resolution",
                card_id
            );
            // Update the matching trigger_evaluation entry so it doesn't stay "pending"
            let card_name = card_id
                .and_then(|id| self.card_database.get_card(id))
                .map(|c| c.name.clone())
                .unwrap_or_default();
            let pp = self.player_prefix();
            let trigger_str = match ability.triggers.as_deref() {
                Some(t)
                    if t.contains(crate::triggers::DEBUT)
                        || t.contains(crate::triggers::DEBUT_EN) =>
                {
                    "debut"
                }
                Some(t) if t.contains(crate::triggers::LIVE_START) => "live_start",
                Some(t)
                    if t.contains(crate::triggers::LIVE_SUCCESS)
                        || t.contains(crate::triggers::LIVE_SUCCESS_EN) =>
                {
                    "live_success"
                }
                Some(t) if t.contains(crate::triggers::ACTIVATION) => "activation",
                Some(t) if t.contains(crate::triggers::CONSTANT) => "constant",
                Some(t) if t.contains(crate::triggers::AUTO) => "auto",
                _ => "unknown",
            };
            let ability_text = ability.full_text.clone();
            let zone = card_id
                .map(|cid| {
                    if self.player1.stage.stage.contains(&cid) {
                        "stage"
                    } else if self.player1.live_card_zone.cards.contains(&cid) {
                        "live_card_zone"
                    } else if self.player2.stage.stage.contains(&cid) {
                        "stage"
                    } else if self.player2.live_card_zone.cards.contains(&cid) {
                        "live_card_zone"
                    } else {
                        "?"
                    }
                })
                .unwrap_or("?");
            // Push ability_resolution entry
            let log_text =
                format!("{pp} {card_name} [{zone}]: 能力発動 [{trigger_str}] — skipped (negated)");
            self.rule_log.push(log_text.clone());
            self.structured_log.push(crate::types::LogEntry {
                text: log_text,
                turn: self.turn_number,
                player_label: pp.clone(),
                source_card_id: card_id,
                source_card_name: Some(card_name),
                category: "ability_resolution".to_string(),
                metadata: Some(serde_json::json!({
                    "result": "skipped",
                    "items": [],
                    "ability_text": ability_text,
                    "zone": zone,
                    "error": "card negated",
                })),
            });
            // Update matching trigger_evaluation entry
            let ability_index = ability_index;
            if let Some(cid) = card_id {
                for entry in self.structured_log.iter_mut().rev() {
                    if entry.category != "trigger_evaluation" {
                        continue;
                    }
                    if entry.source_card_id != Some(cid) {
                        continue;
                    }
                    if entry.turn != self.turn_number {
                        continue;
                    }
                    let trigger_match = entry
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("trigger"))
                        .and_then(|v| v.as_str())
                        == Some(trigger_str);
                    if !trigger_match {
                        continue;
                    }
                    let eval_idx = entry
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("ability_index"))
                        .and_then(|v| v.as_u64())
                        .map(|v| v as usize);
                    if let Some(ei) = eval_idx {
                        if ability_index != ei {
                            continue;
                        }
                    }
                    if let Some(ref mut meta) = entry.metadata {
                        if let Some(obj) = meta.as_object_mut() {
                            obj.insert("result".to_string(), serde_json::json!("skipped"));
                            obj.insert("items".to_string(), serde_json::json!([]));
                            obj.insert("ability_text".to_string(), serde_json::json!(ability_text));
                            obj.insert("resolved".to_string(), serde_json::json!(true));
                        }
                    }
                    break;
                }
            }
            self.ability_queue.complete_current();
            self.activating_card = None;
            self.activating_ability_index = None;
            return;
        }

        self.activating_card = card_id;
        self.activating_ability_index = Some(ability_index);

        // Check if a resolver already exists (e.g., cost phase completed, effect needs to run).
        // If so, reuse it — it carries state (revealed_cost_cards, etc.) needed by the effect.
        let mut resolver = if self.ability_queue.has_resolver() {
            log::debug!("[PCA] Reusing existing resolver for effect execution");
            let mut r = self.ability_queue.take_resolver().unwrap();
            // Don't reset moved_cards/selected_cards — the effect may need
            // them for cost_reference (e.g. previous_moved_card) or conditions.
            r.selected_cards.clear();
            // G1/G3: preserve spawn_context.target across resolver re-use.
            // When resume_pending_commands sets spawn_context.target via the
            // G3 fix and then process_current_ability is called again, the
            // target must survive the reset so the G1 check can route the
            // pending choice to the opponent player.
            let saved_target = r.spawn_context.target.clone();
            r.spawn_context = crate::ability::types::EffectSpawnContext::default();
            r.spawn_context.target = saved_target;
            r.pending_stage_cards = Vec::new();
            r.execution_context = crate::ability::types::ExecutionContext::None;
            r
        } else {
            crate::ability::resolver::AbilityResolver::new(self.card_database.clone(), card_id)
        };

        resolver.debug_trace =
            crate::ability::debug::ABILITY_DEBUG.load(std::sync::atomic::Ordering::Relaxed);

        if crate::ability::debug::ABILITY_DEBUG.load(std::sync::atomic::Ordering::Relaxed) {
            log::debug!("[PCA] resolve_ability start");
        }
        match resolver.resolve_ability(self, &ability, card_id, ability_index) {
            Ok(()) => {
                if crate::ability::debug::ABILITY_DEBUG.load(std::sync::atomic::Ordering::Relaxed) {
                    log::debug!("[PCA] resolve_ability OK");
                }
            }
            Err(e) => {
                if crate::ability::debug::ABILITY_DEBUG.load(std::sync::atomic::Ordering::Relaxed) {
                    log::debug!("[PCA] resolve_ability FAILED: {}", e);
                }
                log::debug!("Failed to resolve ability: {}", e);
                self.ability_queue.complete_current();
                self.clear_effect_tracking();
                return;
            }
        }
        if crate::ability::debug::ABILITY_DEBUG.load(std::sync::atomic::Ordering::Relaxed) {
            log::debug!(
                "[PCA] after resolve: pending={}",
                resolver.pending_choice.is_some()
            );
        }

        // Sync resolver state to GameState before the resolver may be dropped.
        // The condition system and other subsystems read GameState directly.
        if resolver.debug_trace {
            self.last_ability_trace = Some(resolver.pipeline.trace.clone());
        }

        if let Some(ref c) = resolver.pending_choice {
            let is_choice_type = self
                .ability_queue
                .current_entry()
                .and_then(|e| e.choice_card_no.as_ref())
                == Some(&crate::ability::types::ChoiceRoute::Choice);

            // Don't set cost_paid for is_select_action choices — those are
            // "select which target" prompts (e.g. change_state wait) where
            // the actual state change is applied during choice resolution.
            // Setting cost_paid here would prevent the cost handler from
            // re-entering to apply the change.
            let is_deferred = matches!(
                c,
                crate::ability::types::Choice::SelectCard {
                    is_select_action: true,
                    ..
                }
            );
            if !cost_already_paid && !is_deferred {
                if let Some(e) = self.ability_queue.current_entry_mut() {
                    e.cost_paid = true;
                }
            }
            if let Some(e) = self.ability_queue.current_entry_mut() {
                if (cost_already_paid || ability.cost.is_none()) && !is_choice_type {
                    e.effect_started = true;
                }
            }

            // G1/G3: if the resolver was executing for opponent (handle_both_targets,
            // execute_move_cards_both, or opponent_action wrapper), route the choice
            // to the opponent player.
            let targets_opponent = match c {
                crate::ability::types::Choice::SelectCard {
                    target_player_id: Some(tpid),
                    ..
                } if tpid == "opponent"
                    && resolver.spawn_context.target.as_deref() == Some("opponent") =>
                {
                    true
                }
                crate::ability::types::Choice::SelectPosition { .. }
                    if matches!(
                        resolver.execution_context,
                        crate::ability::types::ExecutionContext::MoveCardsPosition { ref target, .. }
                        if target == "opponent"
                    ) =>
                {
                    true
                }
                _ => false,
            };
            if crate::ability::debug::ABILITY_DEBUG.load(std::sync::atomic::Ordering::Relaxed) {
                log::debug!(
                    "[PCA_G1] targets={} spawn={:?}",
                    targets_opponent,
                    resolver.spawn_context.target
                );
            }
            if let Some(entry) = self.ability_queue.current_entry_mut() {
                if targets_opponent {
                    let current = entry.player_id.clone();
                    let opponent_id = if current == "p1" { "p2" } else { "p1" };
                    entry.choice_player_id = Some(opponent_id.to_string());
                    if crate::ability::debug::ABILITY_DEBUG
                        .load(std::sync::atomic::Ordering::Relaxed)
                    {
                        log::debug!("[PCA_G1] SET choice_player_id={}", opponent_id);
                    }
                } else if matches!(c, crate::ability::types::Choice::SelectCard { target_player_id: Some(tpid), .. } if tpid == "self")
                {
                    entry.choice_player_id = Some(entry.player_id.clone());
                    if crate::ability::debug::ABILITY_DEBUG
                        .load(std::sync::atomic::Ordering::Relaxed)
                    {
                        log::debug!(
                            "[PCA_G1] RESET choice_player_id to activator={}",
                            entry.player_id
                        );
                    }
                }
            }

            let choice = c.clone();
            resolver.store_pending_choice(self);
            self.ability_queue.set_resolver(resolver);
            self.ability_queue.pause_for_choice(choice);
        } else {
            // Capture card_no and player_id BEFORE complete_current()
            // resets the queue.
            let current_pid = self
                .ability_queue
                .current_entry()
                .map(|e| e.player_id.clone())
                .unwrap_or_default();

            // Capture info for post-resolution each_time trigger
            let resolved_trigger_type = self
                .ability_queue
                .current_entry()
                .map(|e| e.trigger_type.clone());
            let resolved_card_id = self.ability_queue.current_entry().and_then(|e| e.card_id);
            let resolved_optional_cost = self
                .ability_queue
                .current_entry()
                .and_then(|e| e.optional_cost_result);

            // Build the ability key for the just-completed ability so the
            // re-scan can skip re-enqueueing this SPECIFIC ability while
            // still allowing OTHER abilities (e.g. each_time) on the same
            // card to fire.
            let just_completed_key: Option<String> = self
                .ability_queue
                .current_entry()
                .map(|e| format!("{}_{}", e.card_no, e.ability.full_text));

            self.ability_queue.complete_current();
            // Keep activating_card/ability_index alive through the post-resolution
            // TAS scan below — the guard at line 331-335 uses them to prevent
            // re-enqueueing the exact same ability (e.g. each_time watchers that
            // would re-trigger on the same movement batch that just queued them).
            // Cleared AFTER the TAS scan, before process_pending_auto_abilities.
            // Scan stage watchers (e.g. each_time triggers) BEFORE clearing
            // recently_moved_cards so their preceding_moved conditions pass.
            // Trigger types: each_time:discard, each_time:area_move, each_time:energy_placed
            if self.recently_moved_cards.is_some()
                || self.last_energy_placed_by_effect()
                || !self.recently_appeared_cards.is_empty()
            {
                if crate::ability::debug::ABILITY_DEBUG.load(std::sync::atomic::Ordering::Relaxed) {
                    log::debug!(
                        "[PCA_TRIGGER] scanning stage watchers pid={} moved={:?}",
                        current_pid,
                        self.recently_moved_cards
                    );
                }
                let event = crate::ability::types::TriggerEvent {
                    moved_cards: self.recently_moved_cards.clone().unwrap_or_default(),
                    moved_from_zone: self.recently_moved_from_zone.clone(),
                    position_change_occurred: self.position_change_occurred_this_turn,
                    energy_placed_by_effect: self.last_energy_placed_by_effect(),
                    energy_placed_by_player: self
                        .last_energy_placed_by_player()
                        .map(|s| s.to_string()),
                    ..Default::default()
                };
                self.just_completed_ability_key = just_completed_key;
                self.trigger_auto_abilities_for_player_with_event(&current_pid, &event);
                // just_completed_ability_key intentionally NOT cleared here —
                // process_pending_auto_abilities' post-loop TAS (line ~803)
                // also needs the guard to prevent re-enqueueing the same
                // each_time watcher on stale movement data.

                // TAS scan above already caught all AUTO abilities including
                // trigger_type: "each_time". No separate each_time scan needed.
            }
            // Clear activating_card AFTER the TAS scan (the guard at line 331-335
            // uses it to prevent re-enqueueing the just-completed ability).
            self.activating_card = None;
            self.activating_ability_index = None;
            // Post-resolution each_time trigger for LiveStart/LiveSuccess.
            // Only fires if the effect was actually executed (cost was paid
            // or no optional cost was declined).
            if resolved_optional_cost != Some(false) {
                if let Some(crate::game_state::AbilityTrigger::LiveStart) = resolved_trigger_type {
                    if let Some(cid) = resolved_card_id {
                        self.trigger_each_time_for_member(
                            &current_pid,
                            crate::triggers::LIVE_START,
                            cid,
                        );
                    }
                } else if let Some(crate::game_state::AbilityTrigger::LiveSuccess) =
                    resolved_trigger_type
                {
                    if let Some(cid) = resolved_card_id {
                        self.trigger_each_time_for_member(
                            &current_pid,
                            crate::triggers::LIVE_SUCCESS,
                            cid,
                        );
                    }
                }
            }
            self.clear_effect_tracking();
        }
    }

    pub fn get_pending_choice(&self) -> Option<&crate::ability::types::Choice> {
        self.ability_queue.is_waiting_for_choice()
    }

    pub fn has_pending_choice(&self) -> bool {
        self.ability_queue.is_waiting_for_choice().is_some()
    }

    pub fn entry_effect(&self) -> Option<&crate::card::AbilityEffect> {
        self.ability_queue
            .current_entry()
            .and_then(|e| e.ability.effect.as_ref())
    }

    pub fn entry_cost(&self) -> Option<&crate::card::AbilityCost> {
        self.ability_queue
            .current_entry()
            .and_then(|e| e.ability.cost.as_ref())
    }

    pub fn entry_destination(&self) -> Option<&str> {
        if let Some(entry) = self.ability_queue.current_entry() {
            if !entry.effect_started {
                if let Some(ref cost) = entry.ability.cost {
                    if let Some(ref dest) = cost.destination {
                        return Some(dest);
                    }
                }
            }
        }
        self.entry_effect().and_then(|e| e.destination.as_deref())
    }

    pub fn entry_choice_card_no(&self) -> Option<crate::ability::types::ChoiceRoute> {
        self.ability_queue
            .current_entry()
            .and_then(|e| e.choice_card_no.clone())
    }

    pub fn entry_conditional_choice(&self) -> Option<String> {
        self.ability_queue
            .current_entry()
            .and_then(|e| e.conditional_choice.clone())
    }

    /// Read trigger_moved_cards from the current queue entry (snapshot of
    /// recently_moved_cards at enqueue time). Used by source:"those_cards"
    /// in batch conditions to resolve to only the trigger cards.
    pub fn entry_trigger_moved_cards(&self) -> Option<Vec<i16>> {
        self.ability_queue
            .current_entry()
            .and_then(|e| e.trigger_moved_cards.clone())
    }

    /// Check whether energy was placed by an effect, using the entry's snapshot
    /// of batch_movements at enqueue time. This survives per-ability clearing
    /// of the global batch_movements list in clear_effect_tracking().
    pub fn entry_snapshot_last_energy_placed_by_effect(&self) -> bool {
        self.ability_queue
            .current_entry()
            .map(|e| {
                e.snapshot_movements.iter().any(|m| {
                    (m.dest_zone == "energy" || m.dest_zone == "energy_zone") && m.effect_only
                })
            })
            .unwrap_or(false)
    }

    pub fn entry_snapshot_last_energy_placed_by_player(&self) -> Option<String> {
        self.ability_queue
            .current_entry()
            .and_then(|e| {
                e.snapshot_movements
                    .iter()
                    .find(|m| m.dest_zone == "energy" || m.dest_zone == "energy_zone")
            })
            .map(|m| m.cause_player_id.clone())
    }

    /// Find the latest area move (stage→stage) in the entry's snapshot_movements.
    pub fn entry_snapshot_last_area_move_card_id(&self) -> Option<i16> {
        self.ability_queue
            .current_entry()
            .and_then(|e| {
                e.snapshot_movements
                    .iter()
                    .rev()
                    .find(|m| m.source_zone == "stage" && m.dest_zone == "stage")
            })
            .map(|m| m.moved_card_id)
    }

    pub fn entry_snapshot_last_area_move_by_player(&self) -> Option<String> {
        self.ability_queue
            .current_entry()
            .and_then(|e| {
                e.snapshot_movements
                    .iter()
                    .rev()
                    .find(|m| m.source_zone == "stage" && m.dest_zone == "stage")
            })
            .map(|m| m.cause_player_id.clone())
    }

    /// If the pending choice is routed to a specific player (PVP), return their player_id.
    pub fn get_pending_choice_player_id(&self) -> Option<String> {
        self.ability_queue
            .current_entry()
            .and_then(|e| e.choice_player_id.as_ref().cloned())
    }

    /// Inject card and ability identity into the pending_choice JSON so the frontend
    /// can display which card+ability is responsible for the current choice prompt.
    /// Get the serialized JSON for the frontend from the ability queue's waiting choice.
    pub fn get_pending_choice_json(&self) -> Option<serde_json::Value> {
        let choice = self.ability_queue.is_waiting_for_choice()?;
        let mut json = choice.to_frontend_json()?;
        self.inject_choice_ability_context(&mut json);
        Some(json)
    }

    pub fn inject_choice_ability_context(&self, json: &mut serde_json::Value) {
        let entry = self.ability_queue.current_entry();
        let entry_ref = entry.as_ref();
        let existing_title = json
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if let Some(obj) = json.as_object_mut() {
            if let Some(entry) = entry_ref {
                obj.insert(
                    "card_no".into(),
                    serde_json::Value::String(entry.card_no.clone()),
                );
                obj.insert(
                    "ability_text".into(),
                    serde_json::Value::String(entry.ability.full_text.clone()),
                );
                // Only inject prompt_en/prompt_ja if the Choice-level fields are absent.
                // Choice-level fields are set by to_frontend_json() from description_en/description_ja.
                if !obj.contains_key("prompt_en") {
                    let prompt_en = if entry.choice_effect_text.is_some() {
                        if let Some(ref effect) = entry.ability.effect {
                            crate::ability::describe::describe_effect_en(effect)
                        } else {
                            existing_title.clone()
                        }
                    } else {
                        existing_title.clone()
                    };
                    obj.insert("prompt_en".into(), serde_json::Value::String(prompt_en));
                }
                if !obj.contains_key("prompt_ja") {
                    // Only inject prompt_ja when we can generate actual Japanese text.
                    // If the choice has a card-backed effect_text, describe_effect_ja provides
                    // real Japanese. Otherwise leave prompt_ja absent so the frontend
                    // falls back to prompt_en/title instead of getting fake Japanese.
                    if let Some(ref choice_text) = entry.choice_effect_text {
                        if !choice_text.is_empty() {
                            if let Some(ref effect) = entry.ability.effect {
                                let prompt_ja =
                                    crate::ability::describe::describe_effect_ja(effect);
                                obj.insert(
                                    "prompt_ja".into(),
                                    serde_json::Value::String(prompt_ja),
                                );
                            }
                        }
                    }
                }
                obj.insert(
                    "trigger_type".into(),
                    serde_json::Value::String(format!("{:?}", entry.trigger_type)),
                );
                if let Some(cid) = entry.card_id {
                    obj.insert(
                        "card_id".into(),
                        serde_json::Value::Number(serde_json::Number::from(cid as i64)),
                    );
                    if let Some(card) = self.card_database.get_card(cid) {
                        obj.insert(
                            "card_name".into(),
                            serde_json::Value::String(card.name.clone()),
                        );
                    }
                }
                let raw_pid = entry.choice_player_id.as_ref().unwrap_or(&entry.player_id);
                let normalized = match raw_pid.as_str() {
                    "player1" => "p1",
                    "player2" => "p2",
                    _ => raw_pid.as_str(),
                };
                obj.insert(
                    "choice_player_id".into(),
                    serde_json::Value::String(normalized.to_string()),
                );
                // Inject selection_cards for SelectCard choices so the frontend can render options.
                if let Some(choice) = self.ability_queue.is_waiting_for_choice() {
                    if let crate::ability::types::Choice::SelectCard {
                        ref zone,
                        ref card_type,
                        cost_limit,
                        ref cost_limit_operator,
                        ref target_player_id,
                        ref group,
                        ref filtered_indices,
                        ..
                    } = choice
                    {
                        let target = target_player_id.as_deref().unwrap_or("self");
                        let player = self.resolve_target_player(target);
                        let card_ids: Vec<i16> = match Zone::from_str(zone) {
                            Some(Zone::Hand) => player.hand.cards.iter().copied().collect(),
                            Some(Zone::Discard) | Some(Zone::Waitroom) => {
                                player.waitroom.cards.iter().copied().collect()
                            }
                            Some(Zone::Stage) => player
                                .stage
                                .stage
                                .iter()
                                .copied()
                                .filter(|&id| id != -1)
                                .collect(),
                            Some(Zone::Energy) | Some(Zone::EnergyZone) => {
                                player.energy_zone.cards.iter().copied().collect()
                            }
                            Some(Zone::LookedAt) => self.looked_at_cards.clone(),
                            Some(Zone::RevealedCards) => self.revealed_cards.clone(),
                            Some(Zone::Deck) => player.main_deck.cards.iter().copied().collect(),
                            Some(Zone::SelectedCards) => entry
                                .resolver
                                .as_ref()
                                .map(|r| r.selected_cards.clone())
                                .unwrap_or_default(),
                            _ => Vec::new(),
                        };
                        // When filtered_indices is set (look_and_select with greyed-out cards),
                        // include ALL cards in selection_cards — filtered_indices restricts
                        // selection on the frontend. Otherwise, filter by choice criteria.
                        let filtered: Vec<i16> = if filtered_indices.is_some() {
                            card_ids
                        } else {
                            card_ids
                                .into_iter()
                                .filter(|&cid| {
                                    let type_ok = match card_type.as_deref() {
                                        Some("member_card") => self
                                            .card_database
                                            .get_card(cid)
                                            .map(|c| c.is_member())
                                            .unwrap_or(false),
                                        Some("live_card") => self
                                            .card_database
                                            .get_card(cid)
                                            .map(|c| c.is_live())
                                            .unwrap_or(false),
                                        Some("energy_card") => self
                                            .card_database
                                            .get_card(cid)
                                            .map(|c| c.is_energy())
                                            .unwrap_or(false),
                                        None => true,
                                        _ => true,
                                    };
                                    let group_ok = match group.as_ref() {
                                        Some(g) => crate::ability::util::card_matches_group_str(
                                            &self.card_database,
                                            cid,
                                            Some(g),
                                        ),
                                        None => true,
                                    };
                                    type_ok
                                        && group_ok
                                        && if let Some(lim) = cost_limit {
                                            crate::ability::util::card_matches_cost_limit_op(
                                                &self.card_database,
                                                cid,
                                                Some(*lim),
                                                cost_limit_operator.as_deref(),
                                            )
                                        } else {
                                            true
                                        }
                                })
                                .collect()
                        };
                        let sel: Vec<serde_json::Value> = filtered.iter().map(|&cid| {
                            let card_ref = self.card_database.get_card(cid);
                            let card_type_val = card_ref.map(|c| serde_json::to_value(&c.card_type).unwrap_or_default());
                            serde_json::json!({
                                "id": cid,
                                "card_no": card_ref.map(|c| c.card_no.clone()).unwrap_or_default(),
                                "name": card_ref.map(|c| c.name.clone()).unwrap_or_default(),
                                "type": card_type_val.unwrap_or_default()
                            })
                        }).collect();
                        obj.insert("selection_cards".into(), serde_json::Value::Array(sel));
                    }
                }
            } else if let Some(choice) = self.ability_queue.is_waiting_for_choice() {
                match choice {
                    crate::ability::types::Choice::SelectAutoAbility { player_id, .. }
                    | crate::ability::types::Choice::SelectLiveSuccess { player_id, .. } => {
                        let normalized = match player_id.as_str() {
                            "player1" => "p1",
                            "player2" => "p2",
                            _ => player_id.as_str(),
                        };
                        obj.insert(
                            "choice_player_id".into(),
                            serde_json::Value::String(normalized.to_string()),
                        );
                    }
                    _ => {}
                }
            }
        }
    }

    /// Resolve which player "self" refers to based on the ability master's player_id.
    /// The ability queue entry stores which player activated this ability.
    pub fn ability_master_id(&self) -> Option<String> {
        self.ability_queue
            .current_entry()
            .map(|e| e.player_id.clone())
    }

    pub fn resolve_target_player_mut(&mut self, target: &str) -> &mut Player {
        let master = self.ability_master_id();
        match (target, master.as_deref()) {
            ("self", Some("player2") | Some("p2")) => &mut self.player2,
            ("self", _) => &mut self.player1,
            ("opponent", Some("player2") | Some("p2")) => &mut self.player1,
            ("opponent", _) => &mut self.player2,
            ("both", _) => {
                log::debug!("WARN: resolve_target_player_mut called with 'both'  Ereturning player1, use execute_for_targets instead");
                &mut self.player1
            }
            _ => &mut self.player1,
        }
    }

    pub fn resolve_target_single<'a>(
        &'a self,
        target: &str,
        perspective_player: &'a Player,
    ) -> Option<&'a Player> {
        match target {
            "self" | "自分" => Some(perspective_player),
            "opponent" | "相手" => Some(if std::ptr::eq(perspective_player, &self.player1) {
                &self.player2
            } else {
                &self.player1
            }),
            _ => None,
        }
    }

    pub fn resolve_target_single_mut<'a>(
        &'a mut self,
        target: &str,
        perspective_player: &'a Player,
    ) -> Option<&'a mut Player> {
        match target {
            "self" | "自分" => {
                if std::ptr::eq(perspective_player, &self.player1) {
                    Some(&mut self.player1)
                } else {
                    Some(&mut self.player2)
                }
            }
            "opponent" | "相手" => {
                if std::ptr::eq(perspective_player, &self.player1) {
                    Some(&mut self.player2)
                } else {
                    Some(&mut self.player1)
                }
            }
            _ => None,
        }
    }

    pub fn resolve_target_player(&self, target: &str) -> &Player {
        let master = self.ability_master_id().or_else(|| {
            self.activating_card.and_then(|cid| {
                if self.player1.stage.stage.contains(&cid)
                    || self.player1.live_card_zone.cards.contains(&cid)
                    || self.player1.success_live_card_zone.cards.contains(&cid)
                    || self.player1.hand.cards.contains(&cid)
                    || self.player1.energy_zone.cards.contains(&cid)
                    || self.player1.waitroom.cards.contains(&cid)
                {
                    Some(self.player1.id.clone())
                } else if self.player2.stage.stage.contains(&cid)
                    || self.player2.live_card_zone.cards.contains(&cid)
                    || self.player2.success_live_card_zone.cards.contains(&cid)
                    || self.player2.hand.cards.contains(&cid)
                    || self.player2.energy_zone.cards.contains(&cid)
                    || self.player2.waitroom.cards.contains(&cid)
                {
                    Some(self.player2.id.clone())
                } else {
                    None
                }
            })
        });
        match (target, master.as_deref()) {
            ("self", Some("player2") | Some("p2")) => &self.player2,
            ("self", _) => &self.player1,
            ("opponent", Some("player2") | Some("p2")) => &self.player1,
            ("opponent", _) => &self.player2,
            _ => &self.player1,
        }
    }

    pub fn resolve_target<'a>(
        &'a self,
        target: &str,
        perspective_player: &'a Player,
    ) -> Vec<&'a Player> {
        match target {
            "self" | "自分" => {
                vec![perspective_player]
            }
            "opponent" | "相手" => {
                if std::ptr::eq(perspective_player, &self.player1) {
                    vec![&self.player2]
                } else {
                    vec![&self.player1]
                }
            }
            "both" | "両方" => {
                vec![&self.player1, &self.player2]
            }
            "either" | "どちらか" => {
                vec![&self.player1, &self.player2]
            }
            _ => vec![],
        }
    }

    pub fn resolve_target_mut(
        &mut self,
        target: &str,
        perspective_player_id: &str,
    ) -> Vec<&mut Player> {
        match target {
            "self" | "自分" => {
                if perspective_player_id == self.player1.id {
                    vec![&mut self.player1]
                } else {
                    vec![&mut self.player2]
                }
            }
            "opponent" | "相手" => {
                if perspective_player_id == self.player1.id {
                    vec![&mut self.player2]
                } else {
                    vec![&mut self.player1]
                }
            }
            "both" | "両方" => {
                vec![&mut self.player1, &mut self.player2]
            }
            "either" | "どちらか" => {
                vec![&mut self.player1, &mut self.player2]
            }
            _ => vec![],
        }
    }

    pub fn get_player(&self, player_id: &str) -> Option<&Player> {
        if self.player1.id == player_id {
            Some(&self.player1)
        } else if self.player2.id == player_id {
            Some(&self.player2)
        } else {
            None
        }
    }

    pub fn get_player_mut(&mut self, player_id: &str) -> Option<&mut Player> {
        if self.player1.id == player_id {
            Some(&mut self.player1)
        } else if self.player2.id == player_id {
            Some(&mut self.player2)
        } else {
            None
        }
    }

    pub fn should_trigger_debut(&self, _player: &Player, card: &crate::card::Card) -> bool {
        card.is_member()
    }

    pub fn should_trigger_live_start(&self, _player: &Player) -> bool {
        self.current_phase == Phase::FirstAttackerPerformance
            || self.current_phase == Phase::SecondAttackerPerformance
    }

    /// Check whether a prohibition_effects entry of the form
    /// "restriction:cannot_place:<destination>" blocks placing in the given zone.
    /// LiveCardZone and SuccessLiveZone are interchangeable for placement purposes.
    fn _prohibition_destination_blocks(prohibition: &str, zone: &str) -> bool {
        let parts: Vec<&str> = prohibition.split(':').collect();
        if parts.len() < 3 {
            return false;
        }
        let dest = parts[2];
        if dest.is_empty() {
            // No destination specified — assume the restriction targets the
            // success live card zone (the most common use case for dynamic
            // cannot_place restrictions like メビウスループ).
            return zone == Zone::SuccessLiveZone.to_str();
        }
        let dest_zone = Zone::from_str(dest);
        let target_zone = Zone::from_str(zone);
        dest_zone == target_zone
            || (dest_zone == Some(Zone::LiveCardZone) && target_zone == Some(Zone::SuccessLiveZone))
            || (dest_zone == Some(Zone::SuccessLiveZone) && target_zone == Some(Zone::LiveCardZone))
    }

    pub fn should_trigger_live_success(&self, player: &Player) -> bool {
        // Rule 8.3.15-8.3.16: Heart requirements gate whether the live succeeds.
        // If the live card's need_heart isn't satisfied by stage hearts, the live fails
        // and all cards leave the zone. By the time we reach LiveVictoryDetermination
        // (8.4.4), only cards that passed the heart check remain. However, since some
        // test scenarios may skip the performance phase pipeline, we re-check here.
        if self.current_phase != Phase::LiveVictoryDetermination {
            return false;
        }
        if player.live_card_zone.cards.is_empty() {
            return false;
        }
        // stage_hearts is set in execute_live_victory_determination to include
        // yell blade hearts, matching the total the performance heart check used.
        // Fallback uses heart_color_multiplier from mods to include blade cheering.
        let stage_hearts = player.stage_hearts.clone().unwrap_or_else(|| {
            player.calculate_stage_hearts(
                &self.card_database,
                &self.mods.heart_color_multiplier,
                &self.mods.heart_override,
                &self.mods.heart_modifiers,
            )
        });
        for card_id in &player.live_card_zone.cards {
            if let Some(card) = self.card_database.get_card(*card_id) {
                if let Some(ref need_heart) = card.need_heart {
                    let effective_need = {
                        let has_set = self
                            .mods
                            .need_heart_modifiers
                            .get(card_id)
                            .is_some_and(|m| m.values().any(|e| e.set != 0));
                        if has_set {
                            let mut hearts = std::collections::HashMap::new();
                            if let Some(color_mods) = self.mods.need_heart_modifiers.get(card_id) {
                                for (color, me) in color_mods {
                                    if me.set != 0 {
                                        hearts.insert(*color, me.set as u32);
                                    }
                                }
                            }
                            crate::card::BaseHeart { hearts }
                        } else {
                            let mut hearts = need_heart.hearts.clone();
                            if let Some(color_mods) = self.mods.need_heart_modifiers.get(card_id) {
                                for (color, me) in color_mods {
                                    let entry = hearts.entry(*color).or_insert(0);
                                    *entry = ((*entry as i32) + me.additive).max(0) as u32;
                                }
                            }
                            crate::card::BaseHeart { hearts }
                        }
                    };
                    if crate::card::check_heart_requirement(&effective_need, &stage_hearts) {
                        return true;
                    }
                } else {
                    return true;
                }
            }
        }
        false
    }

    // Q89: Multi-name cards (e.g. "A&B&C") have each constituent name
    // but do NOT have unit/group names not written on the card.
    pub fn can_place_card_in_zone(&self, card_id: i16, zone: &str, _player_id: &str) -> bool {
        if let Some(card) = self.card_database.get_card(card_id) {
            for ability in &card.abilities {
                if Self::ability_matches_trigger(
                    ability,
                    &crate::game_state::AbilityTrigger::Constant,
                ) {
                    if let Some(ref effect) = ability.effect {
                        let restricted_to = effect
                            .restricted_destination
                            .as_deref()
                            .or(effect.destination.as_deref());
                        if effect.action == "restriction"
                            && effect.restriction_type.as_deref() == Some("cannot_place")
                            && {
                                let rz = restricted_to.and_then(Zone::from_str);
                                let cz = Zone::from_str(zone);
                                rz == cz
                                    || rz == Some(Zone::LiveCardZone)
                                        && cz == Some(Zone::SuccessLiveZone)
                                    || rz == Some(Zone::SuccessLiveZone)
                                        && cz == Some(Zone::LiveCardZone)
                            }
                        {
                            log::debug!("Card {} cannot be placed in {} due to constant ability restriction", card.card_no, zone);
                            return false;
                        }
                    }
                }
            }
        }
        // Also consult dynamic prohibition_effects for `cannot_place` restrictions
        // added at runtime (e.g. ライブ成功時 triggers like メビウスループ).
        if self.prohibition_effects.iter().any(|p| {
            p.starts_with("restriction:cannot_place:") && _prohibition_destination_blocks(p, zone)
        }) {
            log::debug!(
                "Card {} cannot be placed in {} due to dynamic prohibition",
                card_id,
                zone
            );
            return false;
        }
        true
    }

    pub fn enforce_constant_ability_restrictions(&mut self) {
        let p1_id = self.player1.id.clone();
        let p2_id = self.player2.id.clone();
        let p1_cards: Vec<(usize, i16)> = self
            .player1
            .live_card_zone
            .cards
            .iter()
            .enumerate()
            .map(|(i, &id)| (i, id))
            .collect();
        let p2_cards: Vec<(usize, i16)> = self
            .player2
            .live_card_zone
            .cards
            .iter()
            .enumerate()
            .map(|(i, &id)| (i, id))
            .collect();

        let mut cards_to_remove: Vec<(&str, usize)> = Vec::new();
        for (index, card_id) in p1_cards {
            if !self.can_place_card_in_zone(
                card_id,
                crate::ability::enums::Zone::LiveCardZone.to_str(),
                &p1_id,
            ) {
                cards_to_remove.push((&p1_id, index));
            }
        }
        for (index, card_id) in p2_cards {
            if !self.can_place_card_in_zone(
                card_id,
                crate::ability::enums::Zone::LiveCardZone.to_str(),
                &p2_id,
            ) {
                cards_to_remove.push((&p2_id, index));
            }
        }

        for (player_id, index) in cards_to_remove {
            let player = if *player_id == self.player1.id {
                &mut self.player1
            } else {
                &mut self.player2
            };
            let card = player.live_card_zone.cards.remove(index);
            player.waitroom.cards.push(card);
            if let Some(card_data) = self.card_database.get_card(card) {
                log::debug!(
                    "Removed card {} from live_card_zone due to constant ability restriction",
                    card_data.card_no
                );
            }
        }
    }

    pub fn get_triggerable_abilities<'a>(
        &self,
        card: &'a crate::card::Card,
        trigger: AbilityTrigger,
        player: &Player,
    ) -> Vec<&'a crate::card::Ability> {
        card.abilities
            .iter()
            .filter(|ability| {
                // Skip abilities with null triggers - they should not auto-trigger during any phase
                if ability.triggers.is_none() {
                    return false;
                }

                let trigger_match = Self::ability_matches_trigger(ability, &trigger);
                match trigger {
                    AbilityTrigger::Activation
                    | AbilityTrigger::Constant
                    | AbilityTrigger::Auto => trigger_match,
                    AbilityTrigger::Debut => {
                        trigger_match && self.should_trigger_debut(player, card)
                    }
                    AbilityTrigger::LiveStart => {
                        trigger_match && self.should_trigger_live_start(player)
                    }
                    AbilityTrigger::LiveSuccess => {
                        trigger_match && self.should_trigger_live_success(player)
                    }
                }
            })
            .collect()
    }

    pub fn check_expired_effects(&mut self) {
        let mut expired_indices = Vec::new();

        for (i, effect) in self.temporary_effects.iter().enumerate() {
            let is_expired = match effect.duration {
                Duration::LiveEnd => {
                    let expired = self.current_turn_phase != TurnPhase::Live;
                    log::debug!(
                        "[EXPIRY] LiveEnd check: phase={:?} turn_phase={:?} expired={}",
                        self.current_phase,
                        self.current_turn_phase,
                        expired
                    );
                    expired
                }
                Duration::ThisTurn => self.turn_number > effect.created_turn,
                Duration::ThisLive => self.current_turn_phase != TurnPhase::Live,
                Duration::Permanent => false,
                Duration::AsLongAs => {
                    // AsLongAs effects persist as long as their condition is true.
                    // For now, treat as ThisLive (expire when live ends) since
                    // condition re-evaluation is not yet implemented.
                    self.current_turn_phase != TurnPhase::Live
                }
                Duration::Unless => self.current_turn_phase != TurnPhase::Live,
            };

            if is_expired {
                expired_indices.push(i);
            }
        }

        if !expired_indices.is_empty() {
            // Clear heart_color_multiplier only when a live-scoped effect expires.
            // ThisTurn effects expiring between turns must NOT wipe the multiplier mid-live.
            let any_live_scoped_expired = expired_indices.iter().any(|&i| {
                matches!(
                    self.temporary_effects[i].duration,
                    Duration::LiveEnd | Duration::ThisLive | Duration::AsLongAs | Duration::Unless
                )
            });
            if any_live_scoped_expired {
                self.mods.heart_color_multiplier.clear();
            }
        }

        for i in expired_indices.into_iter().rev() {
            let effect = self.temporary_effects.remove(i);
            match effect.effect_type.as_str() {
                "activation_cost_increase" => {
                    self.prohibition_effects
                        .retain(|p| !p.contains(&effect.effect_type));
                }
                "activation_cost_decrease" => {
                    self.prohibition_effects
                        .retain(|p| !p.contains(&effect.effect_type));
                }
                "set_blade_count" => {
                    if let Some(ref data) = effect.effect_data {
                        if let Some(card_id) = data.get("card_id").and_then(|v| v.as_i64()) {
                            self.mods.clear_blade_set_modifier(card_id as i16);
                            log::debug!("Cleared set_blade_count modifier for card {}", card_id);
                        }
                    }
                }
                s if s.starts_with("gain_blade") => {
                    if let Some(ref data) = effect.effect_data {
                        if let Some(cards) = data.as_array() {
                            for card_data in cards {
                                if let Some(card_id) =
                                    card_data.get("card_id").and_then(|v| v.as_i64())
                                {
                                    if let Some(amount) =
                                        card_data.get("amount").and_then(|v| v.as_i64())
                                    {
                                        self.mods
                                            .remove_blade_modifier(card_id as i16, amount as i32);
                                        log::debug!(
                                            "Reverted {} blades from card {}",
                                            amount,
                                            card_id
                                        );
                                    }
                                }
                            }
                        } else if let Some(card_data) = data.as_object() {
                            if let Some(card_id) = card_data.get("card_id").and_then(|v| v.as_i64())
                            {
                                if let Some(amount) =
                                    card_data.get("amount").and_then(|v| v.as_i64())
                                {
                                    self.mods
                                        .remove_blade_modifier(card_id as i16, amount as i32);
                                    log::debug!("Reverted {} blades from card {}", amount, card_id);
                                }
                            }
                        }
                    }
                }
                "gain_surplus_heart" => {
                    if let Some(ref data) = effect.effect_data {
                        if let Some(old) = data.get("old_value").and_then(|v| v.as_u64()) {
                            let is_p1 = data.get("is_p1").and_then(|v| v.as_bool()).unwrap_or(true);
                            if is_p1 {
                                self.self_live_surplus_count = old as u32;
                            } else {
                                self.opponent_live_surplus_count = old as u32;
                            }
                            log::debug!("Restored surplus count (is_p1={}) to {}", is_p1, old);
                        }
                    }
                }
                s if s.starts_with("gain_heart") => {
                    if let Some(ref data) = effect.effect_data {
                        if let Some(cards) = data.as_array() {
                            for card_data in cards {
                                if let Some(card_id) =
                                    card_data.get("card_id").and_then(|v| v.as_i64())
                                {
                                    if let Some(amount) =
                                        card_data.get("amount").and_then(|v| v.as_i64())
                                    {
                                        let color_str = card_data
                                            .get("color")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("heart01");
                                        let color = crate::zones::parse_heart_color(color_str);
                                        self.mods.remove_heart_modifier(
                                            card_id as i16,
                                            color,
                                            amount as i32,
                                        );
                                        log::debug!(
                                            "Reverted {} hearts from card {} (color {:?})",
                                            amount,
                                            card_id,
                                            color
                                        );
                                    }
                                }
                            }
                        } else if let Some(card_data) = data.as_object() {
                            if let Some(card_id) = card_data.get("card_id").and_then(|v| v.as_i64())
                            {
                                if let Some(amount) =
                                    card_data.get("amount").and_then(|v| v.as_i64())
                                {
                                    let color_str = card_data
                                        .get("color")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("heart01");
                                    let color = crate::zones::parse_heart_color(color_str);
                                    self.mods.remove_heart_modifier(
                                        card_id as i16,
                                        color,
                                        amount as i32,
                                    );
                                    log::debug!(
                                        "Reverted {} hearts from card {} (color {:?})",
                                        amount,
                                        card_id,
                                        color
                                    );
                                }
                            }
                        }
                    }
                }
                "heart_override" => {
                    if let Some(ref data) = effect.effect_data {
                        if let Some(card_id) = data.get("card_id").and_then(|v| v.as_i64()) {
                            self.mods.remove_heart_override(card_id as i16);
                            log::debug!("Removed heart override for card {}", card_id);
                        }
                    }
                }
                "modify_cost" => {
                    if let Some(ref data) = effect.effect_data {
                        if let Some(cards) = data.as_array() {
                            for card_data in cards {
                                if let Some(card_id) =
                                    card_data.get("card_id").and_then(|v| v.as_i64())
                                {
                                    if let Some(amount) =
                                        card_data.get("amount").and_then(|v| v.as_i64())
                                    {
                                        self.mods
                                            .remove_cost_modifier(card_id as i16, amount as i32);
                                        log::debug!(
                                            "Reverted cost modifier {} from card {}",
                                            amount,
                                            card_id
                                        );
                                    }
                                }
                            }
                        } else if let Some(card_data) = data.as_object() {
                            if let Some(card_id) = card_data.get("card_id").and_then(|v| v.as_i64())
                            {
                                if let Some(amount) =
                                    card_data.get("amount").and_then(|v| v.as_i64())
                                {
                                    self.mods
                                        .remove_cost_modifier(card_id as i16, amount as i32);
                                    log::debug!(
                                        "Reverted cost modifier {} from card {}",
                                        amount,
                                        card_id
                                    );
                                }
                            }
                        }
                    }
                }
                s if s.starts_with("gain_ability:") => {
                    let ability_text = s.trim_start_matches("gain_ability:");
                    // Find the card that has this gained ability
                    let mut card_to_clear = None;
                    for (&cid, abilities) in &self.gained_abilities {
                        if abilities.contains(&ability_text.to_string()) {
                            card_to_clear = Some(cid);
                            break;
                        }
                    }
                    // Also check gained_card_abilities for the card
                    if card_to_clear.is_none() {
                        for (&cid, abils) in &self.gained_card_abilities {
                            if abils.iter().any(|a| {
                                a.triggerless_text == ability_text || a.full_text == ability_text
                            }) {
                                card_to_clear = Some(cid);
                                break;
                            }
                        }
                    }
                    if let Some(card_id) = card_to_clear {
                        // Revert gained score modifier if any
                        if let Some(val) = ability_text.split('+').nth(1).and_then(|s| {
                            s.chars()
                                .take_while(|c| c.is_ascii_digit())
                                .collect::<String>()
                                .parse::<i32>()
                                .ok()
                        }) {
                            self.mods.remove_score_modifier(card_id, val);
                            log::debug!(
                                "Reverted gained ability score modifier +{} for card {}",
                                val,
                                card_id
                            );
                        }
                        self.clear_gained_abilities_for_card(card_id);
                    }
                }
                "set_heart_type" => {
                    if let Some(ref data) = effect.effect_data {
                        if let Some(card_id) = data.get("card_id").and_then(|v| v.as_i64()) {
                            self.mods.heart_color_multiplier.remove(&(card_id as i16));
                            log::debug!("Removed heart color multiplier for card {}", card_id);
                        }
                    }
                }
                s if s.starts_with("set_blade_type:") => {
                    if let Some(ref data) = effect.effect_data {
                        if let Some(card_id) = data.get("card_id").and_then(|v| v.as_i64()) {
                            self.mods.clear_blade_type_modifier(card_id as i16);
                            log::debug!("Cleared blade type modifier for card {}", card_id);
                        }
                    }
                }
                s if s.starts_with("modify_score_") => {
                    if s == "modify_score_set" {
                        if let Some(ref data) = effect.effect_data {
                            if let Some(cards) = data.as_array() {
                                for card_data in cards {
                                    if let Some(card_id) =
                                        card_data.get("card_id").and_then(|v| v.as_i64())
                                    {
                                        self.mods.clear_score_set_modifier(card_id as i16);
                                        log::debug!(
                                            "Cleared score set modifier for card {}",
                                            card_id
                                        );
                                    }
                                }
                            }
                        }
                    } else if let Some(ref data) = effect.effect_data {
                        if let Some(cards) = data.as_array() {
                            for card_data in cards {
                                if let Some(card_id) =
                                    card_data.get("card_id").and_then(|v| v.as_i64())
                                {
                                    if let Some(amount) =
                                        card_data.get("amount").and_then(|v| v.as_i64())
                                    {
                                        self.mods
                                            .remove_score_modifier(card_id as i16, amount as i32);
                                        log::debug!(
                                            "Removed score modifier {} from card {}",
                                            amount,
                                            card_id
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {
                    log::debug!("Expired effect: {}", effect.description);
                }
            }
        }

        // Clear prohibition effects (e.g. "cannot_live") when the live phase ends.
        if self.current_turn_phase != TurnPhase::Live && !self.prohibition_effects.is_empty() {
            self.prohibition_effects.clear();
        }
        if self.current_turn_phase != TurnPhase::Live && !self.cannot_live_players.is_empty() {
            self.cannot_live_players.clear();
        }
    }

    pub fn add_replacement_effect(
        &mut self,
        card_id: i16,
        player_id: String,
        original_event: String,
        replacement_effects: Vec<crate::card::AbilityEffect>,
        is_choice_based: bool,
    ) {
        self.replacement_effects.push(ReplacementEffect {
            card_id,
            player_id,
            original_event,
            replacement_effects,
            is_choice_based,
            applied_this_event: false,
        });
    }

    pub fn reset_replacement_effect_flags(&mut self) {
        for effect in &mut self.replacement_effects {
            effect.applied_this_event = false;
        }
    }

    pub fn mark_replacement_effect_applied(&mut self, card_id: i16) {
        if let Some(effect) = self
            .replacement_effects
            .iter_mut()
            .find(|e| e.card_id == card_id)
        {
            effect.applied_this_event = true;
        }
    }

    pub fn set_opponent_live_success(&mut self, no_excess_heart: bool) {
        self.opponent_live_success_this_turn = true;
        self.opponent_live_no_excess_heart_this_turn = no_excess_heart;
    }

    pub fn reset_change_flags(&mut self) {
        self.position_change_occurred_this_turn = false;
        self.formation_change_occurred_this_turn = false;
        self.opponent_live_success_this_turn = false;
        self.opponent_live_no_excess_heart_this_turn = false;
        self.live_success_triggered_this_turn = false;
        self.live_success_p2_fired = false;
        self.live_success_p1_extra = 0;
        self.live_success_p2_extra = 0;
        self.last_state_change_wait_to_active_count = 0;
        self.recently_state_changed.clear();
        self.self_no_excess_heart_this_turn = false;
    }

    pub fn check_permanent_loop(&mut self) -> bool {
        let state_hash = self.generate_state_hash();

        if self.game_state_history.contains(&state_hash) {
            self.loop_detected = true;
            return true;
        }

        self.game_state_history.push(state_hash);

        if self.game_state_history.len() > self.max_state_history_size {
            self.game_state_history.remove(0);
        }

        false
    }

    fn generate_state_hash(&self) -> String {
        format!(
            "t{}_p{}_tp{}_p1h{}_p1e{}_p1w{}_p1l{}_p1su{}_p1st{:?}_p2h{}_p2e{}_p2w{}_p2l{}_p2su{}_p2st{:?}_oe{}_pro{}_tmp{}_rps{:?}",
            self.turn_number,
            self.current_phase,
            self.current_turn_phase,
            self.player1.hand.cards.len(),
            self.player1.energy_zone.cards.len(),
            self.player1.waitroom.cards.len(),
            self.player1.live_card_zone.cards.len(),
            self.player1.success_live_card_zone.cards.len(),
            self.player1.stage.stage,
            self.player2.hand.cards.len(),
            self.player2.energy_zone.cards.len(),
            self.player2.waitroom.cards.len(),
            self.player2.live_card_zone.cards.len(),
            self.player2.success_live_card_zone.cards.len(),
            self.player2.stage.stage,
            self.mods.orientation_modifiers.len(),
            self.prohibition_effects.len(),
            self.temporary_effects.len(),
            self.rps_winner
        )
    }

    pub fn reset_loop_detection(&mut self) {
        self.game_state_history.clear();
        self.loop_detected = false;
    }

    pub fn is_loop_detected(&self) -> bool {
        self.loop_detected
    }
}
