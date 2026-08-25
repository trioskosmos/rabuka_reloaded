use crate::core::constants::U8Count;
use super::GameState;
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use crate::ability::enums::Zone;
use crate::core::types::{AbilityTrigger, Duration, Phase, ReplacementEffect, TurnPhase};
use crate::player::Player;
use crate::HashMap;
use smallvec::SmallVec;

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
        // No destination specified  Eassume the restriction targets the
        // success live card zone (the most common use case for dynamic
        // cannot_place restrictions like メビウスルーチE.
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

    /// Uses of `(card_id, ability_index)` already consumed this turn.
    pub fn ability_uses_used(&self, card_id: i16, ability_index: usize) -> u8 {
        self.turn_limited_abilities_used
            .get(&(card_id, ability_index, self.turn_number))
            .copied()
            .unwrap_or(0)
    }

    /// True if the ability may still be used this turn (below its per-turn limit).
    /// Abilities without a limit are always allowed. This is the single source of
    /// truth for the trigger-time gate (see `trigger_auto_abilities_for_player_with_event`)
    /// and the resolution-time gate (see `resolver.rs`): keeping them in sync is what
    /// prevents a once-per-turn each_time watcher from being re-queued forever.
    pub fn ability_has_remaining_uses(&self, card_id: i16, ability_index: usize) -> bool {
        let Some(limit) = self
            .card_database
            .get_card(card_id)
            .and_then(|c| c.abilities.get(ability_index))
            .and_then(|ar| ar.resolve().use_limit)
        else {
            return true;
        };
        self.ability_uses_used(card_id, ability_index) < limit
    }

    /// Record one use of a limited ability this turn. This is the **only** method
    /// that mutates `turn_limited_abilities_used`.
    ///
    /// Two guarantees make the per-turn limit robust regardless of how the caller
    /// reached us:
    ///   - *Once-per-activation*: a single activation that resolves across several
    ///     phases/choices (cost ↁEeffect ↁEoptional follow-up) consumes exactly one
    ///     use. Callers may invoke this from any branch point; the current queue
    ///     entry's `use_limit_recorded` flag deduplicates them.
    ///   - *Overflow-proof*: the count saturates, so a runaway caller can never
    ///     overflow the `u8` counter (and once saturated the enqueue gate keeps
    ///     rejecting the ability).
    ///
    /// Returns true iff this call actually consumed a (new) use.
    pub(crate) fn record_ability_use(&mut self, key: (i16, usize, u8)) -> bool {
        if self
            .ability_queue
            .current_entry()
            .is_some_and(|e| e.use_limit_recorded)
        {
            log::debug!("[use_limit] {key:?} already recorded this activation  Eskipping");
            return false;
        }
        let entry = self.turn_limited_abilities_used.entry(key).or_insert(0);
        *entry = entry.saturating_add(1);
        if let Some(e) = self.ability_queue.current_entry_mut() {
            e.use_limit_recorded = true;
        }
        true
    }

    fn build_ability_queue_entry(
        &self,
        card_no: String,
        ability_index: usize,
        ability: crate::Arc<crate::card::Ability>,
        card_id: Option<i16>,
        player_id: String,
        trigger_type: AbilityTrigger,
        trigger_moved_cards: Option<SmallVec<[i16; 4]>>,
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
            choice_card_no: None,
            conditional_choice: None,
            effect_started: false,
            use_limit_recorded: false,
            optional_cost_result: None,
            optional_moves_all_moved: None,
            choice_player_id: None,
            pending_actions: Vec::new(),
            resolver: None,
            trigger_moved_cards,
            triggering_member_id,
            snapshot_movements: SmallVec::new(),
            choice_effect_text: None,
            condition_cache: SmallVec::new(),
        }
    }

    fn collect_constant_ids_for(
        &self,
        cids: impl IntoIterator<Item = i16>,
    ) -> Vec<(i16, usize)> {
        let mut ids = Vec::new();
        for cid in cids {
            if let Some(card) = self.card_database.get_card(cid) {
                for (idx, ar) in card.abilities.iter().enumerate() {
                    let ability = ar.resolve();
                    if Self::ability_matches_trigger(
                        &ability,
                        &crate::game_state::AbilityTrigger::Constant,
                    ) {
                        if ability.effect.is_some() {
                            ids.push((cid, idx));
                        }
                    }
                }
            }
            // Runtime-gained abilities (「…を得る、Egrants a 常晁Eetc.) live in
            // gained_card_abilities, not the card database. Encode them with an
            // offset base so resolve_constant_ability can tell them apart;
            // resolve() filters by trigger like printed abilities.
            for (gidx, ability) in self
                .gained_card_abilities
                .get(&cid)
                .into_iter()
                .flatten()
                .enumerate()
            {
                if Self::ability_matches_trigger(
                    ability,
                    &crate::game_state::AbilityTrigger::Constant,
                ) && ability.effect.is_some()
                {
                    ids.push((cid, crate::ability::types::GAINED_ABILITY_INDEX_BASE + gidx));
                }
            }
        }
        ids
    }

    /// Like collect_constant_stage_effect_ids but for hand cards.
    pub(crate) fn collect_constant_hand_effect_ids(&self) -> Vec<(i16, usize)> {
        self.collect_constant_ids_for(
            self.player1
                .hand
                .cards
                .iter()
                .chain(self.player2.hand.cards.iter())
                .copied(),
        )
    }

    /// Collect (card_id, ability_index) pairs for constant abilities on stage.
    /// Returns lightweight index pairs instead of cloned AbilityEffects  E    /// callers re-lookup through the card_database Arc to avoid 152B ÁEN clones.
    pub(crate) fn collect_constant_stage_effect_ids(&self) -> Vec<(i16, usize)> {
        self.collect_constant_ids_for(self.stage_card_ids())
    }

    /// Helper: look up and resolve a constant ability by (card_id, ability_index).
    /// Returns the resolved Arc<Ability> so the caller can borrow effect from it.
    /// Indices >= [`Self::GAINED_ABILITY_INDEX_BASE`] address runtime-gained
    /// abilities (see collect_constant_ids_for).
    pub(crate) fn resolve_constant_ability(
        &self,
        card_id: i16,
        ability_idx: usize,
    ) -> Option<crate::Arc<crate::card::Ability>> {
        if let Some(gidx) = crate::ability::types::gained_ability_index(ability_idx) {
            return self
                .gained_card_abilities
                .get(&card_id)?
                .get(gidx)
                .map(|a| crate::Arc::new(a.clone()));
        }
        let ar = self.card_database.get_card(card_id)?.abilities.get(ability_idx)?;
        Some(ar.resolve())
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
        let movement = condition.get_movement();
        // All movement types ("moved", "moves", and "position_change") are event-based.
        // "moved" ↁEcard has already moved (can be checked now).
        // "moves" ↁEcard is / was moving (checkable because we set
        //   activating_card to the scanned card, and cards_moved_this_turn
        //   is persistent across the turn).
        // "position_change" ↁEcard changed position on stage (detected via
        //   explicit PositionChangeEvent records, no snapshot dependency).
        if movement == Some("moved")
            || movement == Some("moves")
            || movement == Some("position_change")
            || movement == Some("baton_touch")
            || movement == Some("live_success")
        {
            return true;
        }
        // Appearance  ENOT pre-filtered. Evaluated at resolution time via
        // can_activate_effect so that both self-triggers (Q245) and
        // group-matching are handled correctly with full resolution context.
        // card_count (all variants  Ezone counts are stable state checks)
        // Exception: conditions on revealed_cards are NOT event-based because
        // revealed_cards is populated during yell, not at the trigger event.
        if matches!(condition, crate::card::Condition::Location { .. })
            && condition.get_location() != Some("revealed_cards")
        {
            return true;
        }
        // State change (active↔wait)  Epre-filter so the condition only
        // fires when a recorded transition is available.
        if matches!(condition, crate::card::Condition::State { .. }) {
            return true;
        }
        // Recurse into compound conditions  Eif any child is event-based,
        // the whole compound is pre-filtered.
        if let crate::card::Condition::Compound { ref conditions, .. } = condition {
            if let Some(ref children) = conditions {
                if children.iter().any(|c| Self::condition_is_event_based(c)) {
                    return true;
                }
            }
        }
        false
    }

    /// Legacy wrapper: calls with default event (reads flags from self).
    pub fn trigger_auto_abilities_for_player(&mut self, player_id: &str) {
        let event = crate::ability::types::TriggerEvent {
            moved_cards: self.recently_moved_cards.clone().unwrap_or_default().into(),
            moved_from_zone: self.recently_moved_from_zone.clone(),
            ..Default::default()
        };
        self.trigger_auto_abilities_for_player_with_event(player_id, &event);
    }

    /// Post-movement scan wrapper: fires the TAS with the standard
    /// post-movement snapshot (recently moved cards + position-change flag).
    pub fn trigger_auto_abilities_for_movement(&mut self, player_id: &str) {
        let event = crate::ability::types::TriggerEvent {
            moved_cards: self.recently_moved_cards.clone().unwrap_or_default().into(),
            position_change_occurred: self.position_change_occurred_this_turn,
            ..Default::default()
        };
        self.trigger_auto_abilities_for_player_with_event(player_id, &event);
    }

    /// Same as [`Self::trigger_auto_abilities_for_movement`] but targets the
    /// player of the current ability-queue entry (the common case inside
    /// choice/effect handlers).
    pub fn trigger_auto_abilities_for_movement_current(&mut self) {
        let pid = self
            .ability_queue
            .current_entry()
            .map(|e| e.player_id.clone())
            .unwrap_or_default();
        self.trigger_auto_abilities_for_movement(&pid);
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
        let mut abilities_to_trigger: Vec<(i16, usize, i16)> = Vec::new();
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
                    for (ability_idx, ar) in card.abilities.iter().enumerate() {
                        let ability = ar.resolve();
                        if !crate::zones::check_effect_position(
                            ability
                                .effect
                                .as_ref()
                                .and_then(|e| e.activation_position_any()),
                            card_position,
                        ) {
                            continue;
                        }
                        if ability
                            .triggers
                            .as_ref()
                            .is_some_and(|t| &**t == crate::triggers::AUTO)
                        {
                            let mut trigger_multiplicity: u8 = 1;
                            // Guard: skip discard-location abilities when the card
                            // is on stage (prevents premature triggering).
                            if let Some(ref effect) = ability.effect {
                                if crate::ability::debug::ABILITY_DEBUG
                                    .load(core::sync::atomic::Ordering::Relaxed)
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
                                    // watchers  Ethose track OTHER cards moving to
                                    // discard, not the card itself being in discard.
                                    let cond_location = condition
                                        .get_location()
                                        .or_else(|| {
                                            condition
                                                .get_trigger_event()
                                                .and_then(|t| t.location.as_deref())
                                        })
                                        .unwrap_or("");
                                    if condition.get_source() != Some("preceding_moved")
                                        && Zone::from_str(cond_location) == Some(Zone::Discard)
                                        && (condition.get_card_type().as_deref()
                                            == Some("member_card")
                                            || condition.get_target() == Some("self"))
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
                                            .load(core::sync::atomic::Ordering::Relaxed)
                                        {
                                            log::debug!(
                                                "[TAS_COND] card={} cond_type={:?} passes={}",
                                                card.name,
                                                condition,
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
                                    if effect.trigger_type_any().as_deref() == Some("each_time") {
                                        if matches!(
                                            condition.as_ref(),
                                            crate::card::Condition::Comparison { .. }
                                        ) && condition.get_location() == Some("energy_zone")
                                            && !self.last_energy_placed_by_effect()
                                        {
                                            continue;
                                        }
                                    }
                                }
                                // §9.7.2.1: Compute trigger multiplicity before
                                // the effect block closes  Econdition and effect
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
                            // Movement gate for "was placed" (置かれぁE triggers:
                            // self_target + single-location + movement:"moved" requires
                            // the card to be in event.moved_cards (recently placed).
                            if let Some(ref eff) = ability.effect {
                                if let Some(ref cond) = eff.condition {
                                    if cond.get_self_target().unwrap_or(false)
                                        && cond.get_movement() == Some("moved")
                                        && cond.get_locations().map_or(true, |l| l.len() < 2)
                                        && !event.moved_cards.contains(&card_id)
                                    {
                                        continue;
                                    }
                                }
                            }
                            // Re-scan guard: skip re-enqueueing the exact auto
                            // ability that just completed (numeric key).
                            let num_key = ((card_id as u32) << 16) | (ability_idx as u32);
                            // Marker-carrying watchers (「対戦相手のカードの
                            // 効果でも発動する。」) watching AREA MOVES are
                            // armed by the push_movement_event hook when their
                            // LAST move was caused by a foreign player.
                            // Attribute via the watcher's own turn-scoped move
                            // record — stable across rescan passes, unlike
                            // batch sets. (Energy-placement watchers have no
                            // such record; the generic path handles both
                            // causes for them.)
                            if ability
                                .effect
                                .as_ref()
                                .is_some_and(|e| e.fires_on_opponent_effects())
                            {
                                if let Some(rec) = self
                                    .turn_area_movements
                                    .iter()
                                    .rev()
                                    .find(|m| m.moved_card_id == card_id)
                                {
                                    if rec.cause_player_id != player_id_clone {
                                        log::debug!(
                                            "[TRIGGER_SCOPE] {} last move caused by {} \
                                             (hook owns foreign-cause firings)",
                                            card.name,
                                            rec.cause_player_id
                                        );
                                        continue;
                                    }
                                }
                            }
                            if skip_this_card_auto_key == Some(num_key) {
                                continue;
                            }
                            if skip_this_card_auto_key == Some(num_key) {
                                continue;
                            }
                            // Batch-scoped guard: prevent re-enqueue of any ability
                            // already triggered during this movement batch.
                            if self.this_batch_triggered_ability_ids.contains(&num_key) {
                                continue;
                            }
                            self.this_batch_triggered_ability_ids.push(num_key);
                            // §9.7.2.1: Multi-trigger  EN trigger instances ↁEN
                            // standby entries.  All entries share the same
                            // trigger_moved_cards (full batch) because each
                            // instance independently re-evaluates the condition
                            // at resolution time via can_activate_effect.
                            for _ in 0..trigger_multiplicity {
                                abilities_to_trigger.push((card_id, ability_idx, card_id));
                            }
                        }
                    }
                }
            }
            // Also scan live cards for AUTO abilities
            for &card_id in &player.live_card_zone.cards {
                if let Some(card) = self.card_database.get_card(card_id) {
                    for (ability_idx, ar) in card.abilities.iter().enumerate() {
                        let ability = ar.resolve();
                        if ability
                            .triggers
                            .as_ref()
                            .is_some_and(|t| &**t == crate::triggers::AUTO)
                        {
                            if let Some(ref effect) = ability.effect {
                                // Live card scan  Euses the same event-based
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
                                    } else if condition.get_self_target().unwrap_or(false) {
                                        if let Some(locs) = condition.get_locations() {
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
                                    if cond.get_self_target().unwrap_or(false)
                                        && cond.get_movement() == Some("moved")
                                        && cond.get_locations().map_or(true, |l| l.len() < 2)
                                        && !event.moved_cards.contains(&card_id)
                                    {
                                        continue;
                                    }
                                }
                            }
                            let num_key = ((card_id as u32) << 16) | (ability_idx as u32);
                            if skip_this_card_auto_key == Some(num_key) {
                                continue;
                            }
                            if self.this_batch_triggered_ability_ids.contains(&num_key) {
                                continue;
                            }
                            self.this_batch_triggered_ability_ids.push(num_key);
                            abilities_to_trigger.push((card_id, ability_idx, card_id));
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
                    for (ability_idx, ar) in card.abilities.iter().enumerate() {
                        let ability = ar.resolve();
                        if ability
                            .triggers
                            .as_ref()
                            .is_some_and(|t| &**t == crate::triggers::AUTO)
                        {
                            if let Some(ref effect) = ability.effect {
                                if let Some(ref condition) = effect.condition {
                                    // Appearance conditions are for cards ON stage
                                    // (scanned by the stage loop).  Skip them in the
                                    // moved-cards scan so that cards removed from
                                    // stage (e.g. by baton touch) don't falsely fire.
                                    if matches!(
                                        condition.as_ref(),
                                        crate::card::Condition::Appearance { .. }
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
                            let num_key = ((moved_card_id as u32) << 16) | (ability_idx as u32);
                            if skip_this_card_auto_key == Some(num_key) {
                                continue;
                            }
                            if self.this_batch_triggered_ability_ids.contains(&num_key) {
                                continue;
                            }
                            self.this_batch_triggered_ability_ids.push(num_key);
                            abilities_to_trigger.push((moved_card_id, ability_idx, moved_card_id));
                        }
                    }
                }
            }
        }
        let moved = Some(event.moved_cards.clone());
        for (card_id, ability_idx, _stage_card_id) in abilities_to_trigger {
            let num_key = ((card_id as u32) << 16) | (ability_idx as u32);
            if !self.this_batch_triggered_ability_ids.contains(&num_key) {
                self.this_batch_triggered_ability_ids.push(num_key);
            }
            // §once-per-turn: skip if this ability has already consumed its
            // use_limit this turn. Each_time triggers re-scan after a triggered
            // effect's card movement (e.g. a "recover a card to hand" follow-up),
            // and without this guard a used ability gets re-queued forever,
            // flooding the queue in a runaway loop. Declined abilities are not
            // recorded as used, so they still re-trigger (Q233). This mirrors the
            // resolution-time gate in resolver.rs via the shared accessor.
            if !self.ability_has_remaining_uses(card_id, ability_idx) {
                continue;
            }
            // Look up card_no from the card_id for the queue entry
            let card_no = self
                .card_database
                .get_card(card_id)
                .map(|c| String::from(c.card_no.as_ref()))
                .unwrap_or_default();
            self.trigger_auto_ability_by_index(
                AbilityTrigger::Auto,
                player_id_clone.clone(),
                Some(card_no),
                Some(card_id),
                ability_idx,
                moved.clone(),
                None,
            );
        }
        // Consume the energy flag after every TAS scan  Eeach event should
        // trigger at most one batch of each_time abilities.  The snapshot
        // captured in trigger_auto_ability (above) preserves the flag value
        // for abilities that need it during execution (e.g. Sumire's "moves").
    }

    /// §9.7.2.1: Count how many standby entries to create for a trigger event.
    ///
    /// For `card_count_condition` with `source: "preceding_moved"`, counts
    /// cards in the event batch matching the condition's filters.  Returns 1
    /// for batch patterns ("すべて", "1枚以丁E, self_target, count=1+op=>=).
    /// All other condition types return 1 (single standby instance).
    fn trigger_instance_count(
        moved_cards: &[i16],
        effect: &crate::card::AbilityEffect,
        card_db: &crate::card::CardDatabase,
    ) -> u8 {
        let condition = match &effect.condition {
            Some(c) => c,
            None => return 1,
        };
        if !matches!(condition.as_ref(), crate::card::Condition::Location { .. })
            || condition.get_source() != Some("preceding_moved")
        {
            return 1;
        }
        let matching: Vec<&i16> = moved_cards
            .iter()
            .filter(|&&cid| {
                if cid == -1 {
                    return false;
                }
                if let Some(ct) = condition.get_card_type() {
                    if !crate::ability::util::card_matches_type(card_db, cid, Some(&*ct)) {
                        return false;
                    }
                }
                if let Some(hc) = condition.get_heart_colors() {
                    if !hc.is_empty()
                        && !crate::ability::util::card_matches_heart_colors(card_db, cid, hc)
                    {
                        return false;
                    }
                }
                true
            })
            .collect();
        let match_count = matching.len().u8_count();
        if match_count <= 1 {
            return match_count;
        }
        let ct = condition.get_text();
        if ct.is_some_and(|t| t.contains("すべて") || t.contains("全て") || t.contains("全部"))
        {
            return 1;
        }
        if ct.is_some_and(|t| t.contains("1枚以上") || t.contains("1つ以上")) {
            return 1;
        }
        if condition.get_count() == Some(1) && condition.get_operator() == Some(">=") {
            return 1;
        }
        if condition.get_self_target().unwrap_or(false) {
            return 1;
        }
        match_count
    }

    /// Per-move dedupe identity for opponent-cause watchers: folds the
    /// ability key, the moved card, and the movement sequence number into one
    /// u64. The same watcher arms once PER MOVE  Edistinct moves (even within
    /// one turn) each get their own key.
    pub(crate) fn opp_cause_key(num_key: u32, moved_card_id: i16, seq: u16) -> u64 {
        (num_key as u64)
            ^ ((moved_card_id as i64 as u64) << 20)
            ^ ((seq as u64).rotate_left(44))
    }

    /// Opponent-caused trigger arm: 「(対戦相手のカードの効果でも発動する。)」
    ///
    /// Called from `push_movement_event` for every stage→stage area move whose
    /// cause player differs from the moved card's owner. Scans the OWNER's
    /// staged AUTO abilities that carry the parenthetical extension and
    /// enqueues matching watchers with the move as their trigger batch.
    pub fn fire_opponent_cause_watchers_for_move(
        &mut self,
        moved_card_id: i16,
        causer_player_id: &str,
    ) {
        let owner_pid = if self.player1.contains_card(moved_card_id) {
            self.player1.id.clone()
        } else if self.player2.contains_card(moved_card_id) {
            self.player2.id.clone()
        } else {
            return;
        };
        if owner_pid == causer_player_id {
            return; // own-side cause: the normal owner-side TAS handles it
        }
        let stage_cards: Vec<i16> = if owner_pid == self.player1.id {
            self.player1.stage.stage.to_vec()
        } else {
            self.player2.stage.stage.to_vec()
        };
        for &watcher_id in &stage_cards {
            if watcher_id == -1 {
                continue;
            }
            let (card_name, card_no, abilities) = match self.card_database.get_card(watcher_id) {
                Some(c) => (
                    c.name.to_string(),
                    c.card_no.to_string(),
                    c.abilities.clone(),
                ),
                None => continue,
            };
            for (ability_idx, ar) in abilities.iter().enumerate() {
                let ability = ar.resolve();
                if !ability
                    .triggers
                    .as_ref()
                    .is_some_and(|t| &**t == crate::triggers::AUTO)
                {
                    continue;
                }
                let effect = match ability.effect.as_ref() {
                    Some(e) => e,
                    None => continue,
                };
                // Only effects carrying the explicit parenthetical extension.
                let also_opponent = effect.fires_on_opponent_effects();
                if !also_opponent {
                    continue;
                }
                let condition = match effect.condition.as_ref() {
                    Some(c) => c,
                    None => continue,
                };
                let saved_activating = self.activating_card;
                self.activating_card = Some(watcher_id);
                let moved_one: SmallVec<[i16; 4]> = smallvec::smallvec![moved_card_id];
                let ctx = crate::ability::condition::ConditionContext::with_moved_cards(
                    self,
                    &moved_one,
                );
                let passes = ctx.evaluate_condition(condition);
                self.activating_card = saved_activating;
                if !passes {
                    continue;
                }
                let num_key = ((watcher_id as u32) << 16) | (ability_idx as u32);
                let ekey = Self::opp_cause_key(
                    num_key,
                    moved_card_id,
                    self.movement_event_counter,
                );
                if self.mods.opp_cause_fired_keys.contains(&ekey) {
                    continue;
                }
                self.mods.opp_cause_fired_keys.push(ekey);
                // Also claim the plain batch key so empty-batch rescan passes
                // (which bypass movement gating for composites) cannot
                // re-fire this watcher after the hook already did.
                if !self.this_batch_triggered_ability_ids.contains(&num_key) {
                    self.this_batch_triggered_ability_ids.push(num_key);
                }
                log::debug!(
                    "[OPP_CAUSE_WATCHER] firing {} (seat {}) on opponent-caused move of {}",
                    card_name,
                    owner_pid,
                    moved_card_id
                );
                self.trigger_auto_ability_by_index(
                    AbilityTrigger::Auto,
                    owner_pid.clone(),
                    Some(card_no.clone()),
                    Some(watcher_id),
                    ability_idx,
                    Some(smallvec::smallvec![moved_card_id]),
                    None,
                );
            }
        }
    }

    pub fn trigger_auto_ability(
        &mut self,
        ability_id: String,
        trigger_type: AbilityTrigger,
        player_id: String,
        source_card_id: Option<String>,
        explicit_card_id: Option<i16>,
        trigger_moved_cards: Option<SmallVec<[i16; 4]>>,
        triggering_member_id: Option<i16>,
    ) {
        if let Some(ref card_no) = source_card_id {
            let card_id = if let Some(cid) = explicit_card_id {
                Some(cid)
            } else {
                self.find_card_by_number_for_player(card_no, &player_id).1
            };
            if let Some(cid) = card_id {
                if let Some(card) = self.card_database.get_card(cid) {
                    // Check original abilities
                    let expected_id = |ability: &crate::card::Ability| -> String {
                        format!("{}_{}", card_no, ability.full_text)
                    };
                    for (ability_index, ability) in card.abilities.iter().enumerate() {
                        if Self::ability_matches_trigger(&ability.resolve(), &trigger_type)
                            && ability_id == expected_id(&ability.resolve())
                        {
                            let entry = self.build_ability_queue_entry(
                                card_no.clone(),
                                ability_index,
                                ability.to_arc(),
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
                            if crate::ability::debug::ABILITY_DEBUG
                                .load(core::sync::atomic::Ordering::Relaxed)
                            {
                                log::debug!(
                                    "[QUEUE_DIAG] enqueue player={} card_no={}",
                                    entry.player_id,
                                    entry.card_no
                                );
                            }
                            self.push_debug_note(format!(
                                "queue+ {} card={} trigger={:?}",
                                ability_id,
                                entry.card_no,
                                entry.trigger_type
                            ));
                            self.ability_queue.enqueue(entry);
                            return;
                        }
                    }
                    // Check gained card abilities (ability_id format: "card_no_gained_{idx}")
                    if ability_id.contains("_gained_") {
                        let cid = card_id.or(explicit_card_id);
                        if let Some(card_id_val) = cid {
                            if let Some(gained_list) = self.gained_card_abilities.get(&card_id_val)
                            {
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
                                                    crate::ability::types::GAINED_ABILITY_INDEX_BASE + gidx,
                                                    crate::Arc::new(gained_ability.clone()),
                                                    Some(card_id_val),
                                                    player_id.clone(),
                                                    trigger_type,
                                                    trigger_moved_cards.clone(),
                                                    triggering_member_id,
                                                );
                                                let mut entry = entry;
                                                entry.snapshot_movements =
                                                    self.batch_movements.clone();
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
    }

    /// Hot-path version of trigger_auto_ability that takes a numeric ability index
    /// instead of a string key. Avoids format!() allocations in the TAS scan loop.
    pub fn trigger_auto_ability_by_index(
        &mut self,
        trigger_type: AbilityTrigger,
        player_id: String,
        source_card_id: Option<String>,
        explicit_card_id: Option<i16>,
        ability_index: usize,
        trigger_moved_cards: Option<SmallVec<[i16; 4]>>,
        triggering_member_id: Option<i16>,
    ) {
        if let Some(card_id) = explicit_card_id {
            if let Some(card) = self.card_database.get_card(card_id) {
                if let Some(ar) = card.abilities.get(ability_index) {
                    let card_no_str = source_card_id.unwrap_or_default();
                    let entry = self.build_ability_queue_entry(
                        card_no_str,
                        ability_index,
                        ar.to_arc(),
                        Some(card_id),
                        player_id,
                        trigger_type,
                        trigger_moved_cards,
                        triggering_member_id,
                    );
                    let mut entry = entry;
                    entry.snapshot_movements = self.batch_movements.clone();
                    if crate::ability::debug::ABILITY_DEBUG
                        .load(core::sync::atomic::Ordering::Relaxed)
                    {
                        log::debug!(
                            "[QUEUE_DIAG] enqueue player={} card_no={}",
                            entry.player_id,
                            entry.card_no
                        );
                    }
                    self.ability_queue.enqueue(entry);
                }
            }
        }
    }

    /// Search for a card in the specified player's zones first, fall back to the other player.
    /// Returns (card_database_id, instance_card_id) instead of cloning the full Card.
    fn find_card_by_number_for_player(
        &self,
        card_no: &str,
        player_id: &str,
    ) -> (Option<i16>, Option<i16>) {
        let preferred = if player_id == self.player1.id || player_id == "p1" {
            &self.player1
        } else {
            &self.player2
        };
        let other = if core::ptr::eq(preferred, &self.player1) {
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

    /// Search for a card by card_no in the specified player's zones.
    /// Returns (card_database_id, instance_card_id) instead of cloning the full Card.
    fn search_player_zones_for_card(
        &self,
        card_no: &str,
        player: &Player,
    ) -> (Option<i16>, Option<i16>) {
        for id in &player.hand.cards {
            if let Some(card) = self.card_database.get_card(*id) {
                if card.card_no == card_no {
                    return (Some(*id), Some(*id));
                }
            }
        }
        for stage_card_id in &player.stage.stage {
            if *stage_card_id != -1 {
                if let Some(card) = self.card_database.get_card(*stage_card_id) {
                    if card.card_no == card_no {
                        return (Some(*stage_card_id), Some(*stage_card_id));
                    }
                }
            }
        }
        for waitroom_card_id in &player.waitroom.cards {
            if let Some(card) = self.card_database.get_card(*waitroom_card_id) {
                if card.card_no == card_no {
                    return (Some(*waitroom_card_id), Some(*waitroom_card_id));
                }
            }
        }
        for live_card_id in &player.live_card_zone.cards {
            if let Some(card) = self.card_database.get_card(*live_card_id) {
                if card.card_no == card_no {
                    return (Some(*live_card_id), Some(*live_card_id));
                }
            }
        }
        for success_card_id in &player.success_live_card_zone.cards {
            if let Some(card) = self.card_database.get_card(*success_card_id) {
                if card.card_no == card_no {
                    return (Some(*success_card_id), Some(*success_card_id));
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
        // Only fire for stage member cards  Elive cards' own LiveStart/LiveSuccess
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
        let mut abilities: Vec<(i16, usize, i16)> = Vec::new();
        for &card_id in &player.live_card_zone.cards {
            if let Some(card) = self.card_database.get_card(card_id) {
                for (ability_idx, ar) in card.abilities.iter().enumerate() {
                    let ability = ar.resolve();
                    if ability.triggers.as_deref() != Some(crate::triggers::AUTO) {
                        continue;
                    }
                    let effect = match &ability.effect {
                        Some(e) => e,
                        None => continue,
                    };
                    if effect.trigger_type_any().as_deref() != Some("each_time") {
                        continue;
                    }
                    let watch_text = match &effect.condition {
                        Some(c) => c.get_text(),
                        None => Some(effect.text.as_ref()),
                    };
                    if !watch_text.is_some_and(|t| t.contains(trigger_substring)) {
                        continue;
                    }
                    abilities.push((card_id, ability_idx, card_id));
                }
            }
        }
        for (cid, ability_idx, _) in abilities {
            let card_no = self
                .card_database
                .get_card(cid)
                .map(|c| String::from(c.card_no.as_ref()))
                .unwrap_or_default();
            self.trigger_auto_ability_by_index(
                crate::game_state::AbilityTrigger::Auto,
                player_id_clone.clone(),
                Some(card_no),
                Some(cid),
                ability_idx,
                None,
                Some(member_card_id),
            );
        }
    }

    fn process_player_abilities(&mut self, raw_player_id: &str) {
        self.process_player_abilities_depth(raw_player_id)
    }

    /// Recursive auto-ability resolution with a bounded re-entry depth.
    ///
    /// `process_player_abilities` re-enters itself from its post-loop batch
    /// scan (§9.5.3.1 loopback) whenever a watcher enqueues further abilities.
    /// The per-ability `reprocess_counts` guard is local to each invocation, so
    /// runaway re-triggering would otherwise recurse without bound and overflow
    /// the stack. `max_auto_recursion` caps the depth as a last-resort safety
    /// net; well-formed games resolve in a handful of levels.
    fn process_player_abilities_depth(&mut self, raw_player_id: &str) {
        let player_id = match raw_player_id {
            "player1" => "p1",
            "player2" => "p2",
            other => other,
        };
        if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
            log::debug!(
                "[QUEUE_DIAG] process_player_abilities player={} queue_len={}",
                player_id,
                self.ability_queue.len()
            );
        }
        let mut reprocess_counts: HashMap<(i16, usize), u8> = HashMap::default();
        let mut batch_rerun = true;
        while batch_rerun {
            batch_rerun = false;
            loop {
            if !self.ability_queue.is_idle() {
                break;
            }

            // Snapshot queue length before resolution. Entries at indices >= pre_len
            // are freshly triggered (each_time watchers) by the current resolution
            // and must be drained depth-first (§9.5.3.2→§9.5.3.1 loopback).
            let pre_len =
                self.depth_first_cutoff
                    .unwrap_or_else(|| self.ability_queue.len() as u16) as usize;
            self.depth_first_cutoff = None;

            let available_indices: Vec<usize> = (0..pre_len)
                .filter(|&i| {
                    self.ability_queue.is_entry_available(i)
                        && self.ability_queue.entry_player_id(i) == Some(player_id)
                })
                .collect();

            if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
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
                            .map(|c| c.name.to_string())
                            .unwrap_or_else(|| entry.card_no.to_string().into());
                        Some(crate::ability::types::AutoAbilityOption {
                            card_name,
                            ability_text: entry.ability.full_text.clone(),
                            queue_index: idx,
                            card_id: entry.card_id,
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
                        .map(|c| c.name.to_string())
                        .unwrap_or_default();
                    log::error!(
                        "[PCA_INFINITE_LOOP] card={} ({}) ability=\"{}\" processed {} times",
                        card_name,
                        entry.card_no,
                        entry.ability.full_text,
                        *count
                    );
                    if crate::ability::debug::ABILITY_DEBUG
                        .load(core::sync::atomic::Ordering::Relaxed)
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
            self.clear_recently_moved_batch();
            self.recently_appeared_cards.clear();
            self.recently_state_changed.clear();
            // Save flag for the post-loop batch scan below;
            // process_current_ability's internal scan (line 742) already ran
            // before the clear above, so each_time watchers from the
            // just-resolved effect were caught. This post-loop scan catches
            // batch movements (look_and_select, etc.) that finalize card
            // movement outside individual ability resolution.
            if had_recent_moves {
                self.set_recently_moved_batch(SmallVec::new(), None);
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
                        // Sub-resolution may queue deeper entries  Ewhile loop catches them
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
        //   I put all of them back with one trigger?" ↁENo, only 1 card per
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
                moved_cards: batch_ids.into(),
                energy_placed_by_effect: self.last_energy_placed_by_effect(),
                ..Default::default()
            };
            self.trigger_auto_abilities_for_player_with_event(player_id, &event);
            self.clear_recently_moved_batch();
            self.recently_appeared_cards.clear();
            self.recently_state_changed.clear();
            // Re-enter the loop to process any abilities just enqueued
            // by the watcher scan (e.g. Hazuki Ren each_time after discard).
            // Keep this_batch_triggered_ability_ids alive through the recursive
            // call so the same ability isn't enqueued twice from stale events.
            if !self.has_pending_choice() {
                batch_rerun = true;
            }
            self.batch_movements.clear();
            self.position_change_events.clear();
            self.this_batch_triggered_ability_ids.clear();
        }
        } // end while batch_rerun
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
        // Safety timeout: a runaway ability re-trigger loop (e.g. an each_time
        // watcher re-queued by its own effect's movement) must never spin forever.
        // Abort resolution past an absurd number of calls instead of hanging or
        // overflowing a counter.
        use crate::compat::atomic::AtomicU32;
        use core::sync::atomic::Ordering;
        static PCA_CALLS: AtomicU32 = AtomicU32::new(0);
        if PCA_CALLS.fetch_add(1, Ordering::Relaxed) > 200_000 {
            log::error!(
                "[PCA_TIMEOUT] exceeded 200k process_current_ability calls; aborting to break runaway loop"
            );
            self.ability_queue.clear();
            return;
        }
        if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
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
                "[NEGATED] card_id={:?} is negated  Eskipping ability resolution",
                card_id
            );
            // Update the matching trigger_evaluation entry so it doesn't stay "pending"
            let card_name = card_id
                .and_then(|id| self.card_database.get_card(id))
                .map(|c| c.name.to_string())
                .unwrap_or_default();
            let pp = self.player_prefix();
            let trigger_str = match ability.triggers.as_deref() {
                Some(raw) => crate::triggers::canonical_trigger(raw),
                None => "unknown".to_string(),
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
            let log_text = format!(
                "{pp} {card_name} [{zone}]: [[log_ability_result:trigger=trigger_{trigger_str},result=result_skipped_negated]]"
            );
            self.push_rule_log(log_text.clone());
            if !crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
                self.ability_queue.complete_current();
                self.activating_card = None;
                self.activating_ability_index = None;
                return;
            }
            let fallback_entry = crate::types::LogEntry {
                text: log_text,
                turn: self.turn_number,
                player_label: pp.clone(),
                source_card_id: card_id,
                source_card_name: Some(card_name),
                category: "ability_resolution".to_string(),
                metadata: Some(crate::core::types::LogMetadata::AbilityResolution {
                    result: "skipped".to_string(),
                    trigger: trigger_str.clone(),
                    #[cfg(feature = "serde_support")]
                    items: Vec::new(),
                    ability_text: ability_text.to_string(),
                    zone: zone.to_string(),
                    error: Some("cards".to_string()),
                    resolved: Some(false),
                }),
            };
            // Commit to the matching trigger_evaluation entry, or push standalone.
            self.commit_or_push_structured(
                card_id,
                &trigger_str,
                Some(ability_index),
                crate::core::types::LogMetadata::AbilityResolution {
                    result: "skipped".to_string(),
                    trigger: trigger_str.clone(),
                    #[cfg(feature = "serde_support")]
                    items: Vec::new(),
                    ability_text: ability_text.to_string(),
                    zone: zone.to_string(),
                    error: Some("card negated".to_string()),
                    resolved: Some(false),
                },
                fallback_entry,
            );
            self.ability_queue.complete_current();
            self.activating_card = None;
            self.activating_ability_index = None;
            return;
        }

        self.activating_card = card_id;
        self.activating_ability_index = Some(ability_index);

        // Check if a resolver already exists (e.g., cost phase completed, effect needs to run).
        // If so, reuse it  Eit carries state (revealed_cost_cards, etc.) needed by the effect.
        let mut resolver = if self.ability_queue.has_resolver() {
            log::debug!("[PCA] Reusing existing resolver for effect execution");
            let mut r = self.ability_queue.take_resolver().unwrap();
            // Don't reset moved_cards/selected_cards  Ethe effect may need
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
            r.pending_stage_cards = SmallVec::new();
            r.execution_context = crate::ability::types::ExecutionContext::None;
            // Clear pending_choice: the previous call stored it in the queue,
            // and it must not block re-execution of the effect on the next pass.
            r.pending_choice = None;
            r
        } else {
            crate::Box::new(crate::ability::resolver::AbilityResolver::new(
                self.card_database.clone(),
                card_id,
            ))
        };

        resolver.debug_trace =
            crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed);

        if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
            log::debug!("[PCA] resolve_ability start");
        }
        match resolver.resolve_ability(self, &ability, card_id, ability_index) {
            Ok(()) => {
                self.push_debug_note(format!(
                    "resolve ok card={:?} idx={} pending_choice={}",
                    card_id,
                    ability_index,
                    resolver.pending_choice.is_some()
                ));
                if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed)
                {
                    log::debug!("[PCA] resolve_ability OK");
                }
            }
            Err(e) => {
                self.push_debug_note(format!(
                    "resolve FAIL card={:?} idx={} err={}",
                    card_id, ability_index, e
                ));
                if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed)
                {
                    log::debug!("[PCA] resolve_ability FAILED: {}", e);
                }
                log::debug!("Failed to resolve ability: {}", e);
                self.ability_queue.complete_current();
                self.clear_effect_tracking();
                return;
            }
        }
        if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
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

            // Don't set cost_paid for is_select_action choices  Ethose are
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
            if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
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
                        .load(core::sync::atomic::Ordering::Relaxed)
                    {
                        log::debug!("[PCA_G1] SET choice_player_id={}", opponent_id);
                    }
                } else if matches!(c, crate::ability::types::Choice::SelectCard { target_player_id: Some(tpid), .. } if tpid == "self")
                {
                    entry.choice_player_id = Some(entry.player_id.clone());
                    if crate::ability::debug::ABILITY_DEBUG
                        .load(core::sync::atomic::Ordering::Relaxed)
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
            let just_completed_key: Option<u32> =
                self.ability_queue.current_entry().and_then(|e| {
                    let cid = e.card_id? as u32;
                    let idx = e.ability_index as u32;
                    Some((cid << 16) | idx)
                });

            self.ability_queue.complete_current();
            // Keep activating_card/ability_index alive through the post-resolution
            // TAS scan below  Ethe guard at line 331-335 uses them to prevent
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
                if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed)
                {
                    log::debug!(
                        "[PCA_TRIGGER] scanning stage watchers pid={} moved={:?}",
                        current_pid,
                        self.recently_moved_cards
                    );
                }
                let event = crate::ability::types::TriggerEvent {
                    moved_cards: self.recently_moved_cards.clone().unwrap_or_default().into(),
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
                // just_completed_ability_key intentionally NOT cleared here  E                // process_pending_auto_abilities' post-loop TAS (line ~803)
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
            .and_then(|e| e.ability.effect.as_deref())
    }

    pub fn entry_cost(&self) -> Option<&crate::card::AbilityCost> {
        self.ability_queue
            .current_entry()
            .and_then(|e| e.ability.cost.as_deref())
    }

    pub fn entry_destination(&self) -> Option<&str> {
        if let Some(entry) = self.ability_queue.current_entry() {
            if !entry.effect_started {
                if let Some(ref cost) = entry.ability.cost {
                    if let Some(dest) = cost.destination {
                        return Some(dest.to_str());
                    }
                }
            }
        }
        self.entry_effect().and_then(|e| e.destination.map(|d| d.to_str()))
    }

    pub fn entry_choice_card_no(&self) -> Option<crate::ability::types::ChoiceRoute> {
        self.ability_queue
            .current_entry()
            .and_then(|e| e.choice_card_no.clone())
    }

    pub fn entry_conditional_choice(&self) -> Option<crate::ability_queue::ConditionalChoice> {
        self.ability_queue
            .current_entry()
            .and_then(|e| e.conditional_choice.clone())
    }

    /// Read trigger_moved_cards from the current queue entry (snapshot of
    /// recently_moved_cards at enqueue time). Used by source:"those_cards"
    /// in batch conditions to resolve to only the trigger cards.
    pub fn entry_trigger_moved_cards(&self) -> Option<SmallVec<[i16; 4]>> {
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
                    (m.dest_zone == crate::types::ZoneId::Energy
                        || m.dest_zone == crate::types::ZoneId::EnergyZone
                        || m.dest_zone == crate::types::ZoneId::UnderMember)
                        && m.effect_only
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
                    .find(|m| {
                        m.dest_zone == "energy"
                            || m.dest_zone == "energy_zone"
                            || m.dest_zone == "under_member"
                    })
            })
            .map(|m| m.cause_player_id.clone())
    }

    /// Find the latest area move (stage→stage) in the entry's snapshot_movements.
    pub fn entry_snapshot_last_area_move_card_id(&self) -> Option<i16> {
        self.ability_queue
            .current_entry()
            .and_then(|e| {
                e.snapshot_movements.iter().rev().find(|m| {
                    m.source_zone == crate::types::ZoneId::Stage
                        && m.dest_zone == crate::types::ZoneId::Stage
                })
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
    #[cfg(feature = "serde_support")]
    pub fn get_pending_choice_json(&self) -> Option<serde_json::Value> {
        let choice = self.ability_queue.is_waiting_for_choice()?;
        let mut json = choice.to_frontend_json()?;
        self.inject_choice_ability_context(&mut json);
        Some(json)
    }

    #[cfg(feature = "serde_support")]
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
                    serde_json::Value::String(entry.card_no.to_string()),
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
                    obj.insert("prompt_en".into(), serde_json::Value::String(prompt_en.clone()));
                }
                // Always derive a Japanese prompt so the UI never silently falls back to
                // English in Japanese mode. Engine is the single source of truth:
                //   1. generic instruction template translator (parameterized prompts)
                //   2. effect-backed Japanese description
                //   3. English prompt (last resort)  Eand we WARN so the gap is caught.
                if !obj.contains_key("prompt_ja") {
                    let prompt_en = obj
                        .get("prompt_en")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let prompt_ja = crate::ability::describe::translate_choice_prompt_en_to_ja(&prompt_en)
                        .or_else(|| {
                            if entry.choice_effect_text.as_deref().map_or(false, |t| !t.is_empty()) {
                                entry.ability.effect.as_ref().map(|e| crate::ability::describe::describe_effect_ja(e))
                            } else {
                                None
                            }
                        })
                        .unwrap_or_else(|| prompt_en.clone());
                    if !prompt_en.is_empty() && prompt_ja == prompt_en {
                        log::warn!(
                            "[i18n] choice prompt has no Japanese translation (English shown): {:?}",
                            prompt_en
                        );
                    }
                    obj.insert("prompt_ja".into(), serde_json::Value::String(prompt_ja));
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
                            serde_json::Value::String(card.name.to_string()),
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
                        ref characters,
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
                            Some(Zone::LookedAt) => self.looked_at_cards.to_vec(),
                            Some(Zone::RevealedCards) => self.revealed_cards.to_vec(),
                            Some(Zone::Deck) => player.main_deck.cards.iter().copied().collect(),
                            Some(Zone::SelectedCards) => entry
                                .resolver
                                .as_ref()
                                .map(|r| r.selected_cards.to_vec())
                                .unwrap_or_default(),
                            _ => Vec::new(),
                        };
                        // When filtered_indices is set (look_and_select with greyed-out cards),
                        // include ALL cards in selection_cards  Efiltered_indices restricts
                        // selection on the frontend. Otherwise, filter by choice criteria.
                        let filtered: Vec<i16> = if filtered_indices.is_some() {
                            card_ids
                        } else {
                            let card_db = &self.card_database;
                            card_ids
                                .into_iter()
                                .filter(|&cid| {
                                    let type_ok = match card_type.as_deref() {
                                        Some("member_card") => card_db
                                            .get_card(cid)
                                            .map(|c| c.is_member())
                                            .unwrap_or(false),
                                        Some("live_card") => card_db
                                            .get_card(cid)
                                            .map(|c| c.is_live())
                                            .unwrap_or(false),
                                        Some("energy_card") => card_db
                                            .get_card(cid)
                                            .map(|c| c.is_energy())
                                            .unwrap_or(false),
                                        None => true,
                                        _ => true,
                                    };
                                    let group_ok = match group.as_ref() {
                                        Some(g) => crate::ability::util::card_matches_group_str(
                                            card_db,
                                            cid,
                                            Some(g),
                                        ),
                                        None => true,
                                    };
                                    let chars_ok = match characters.as_ref() {
                                        Some(chars) => {
                                            crate::ability::util::card_matches_characters(
                                                card_db,
                                                cid,
                                                Some(chars),
                                            )
                                        }
                                        None => true,
                                    };
                                    let cost_ok = if let Some(lim) = cost_limit {
                                        crate::ability::util::card_matches_cost_limit_op(
                                            card_db,
                                            cid,
                                            Some(*lim),
                                            cost_limit_operator.as_deref(),
                                        )
                                    } else {
                                        true
                                    };
                                    type_ok && group_ok && chars_ok && cost_ok
                                })
                                .collect()
                        };
                        let sel: Vec<serde_json::Value> = filtered.iter().map(|&cid| {
                            let card_ref = self.card_database.get_card(cid);
                            let card_type_val = card_ref.map(|c| serde_json::to_value(&c.card_type).unwrap_or_default());
                            serde_json::json!({
                                "id": cid,
                                "card_no": card_ref.map(|c| c.card_no.to_string()).unwrap_or_default(),
                                "name": card_ref.map(|c| c.name.to_string()).unwrap_or_default(),
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

    /// Return the opponent's player ID given a player ID.
    pub fn opponent_id(&self, player_id: &str) -> String {
        if player_id == self.player1.id {
            self.player2.id.clone()
        } else {
            self.player1.id.clone()
        }
    }

    /// Number of DISTINCT group names among `player_id`'s stage members.
    /// Single source of truth for "グループ名1種類につぁE cost reductions  E    /// used by the resolver's runtime cost adjustment AND by action
    /// generation's effective-cost display/gating.
    ///
    /// Membership is resolved through [`crate::ability::util::card_matches_group_str`]
    /// so multi-name joint cards (Q228: LL-bp1-001-R＋ carries 虹ヶ咲 +
    /// Liella! + 蓮ノ空 through its three names) contribute every group they
    /// belong to, not just a single `card.group` field.
    pub fn distinct_stage_groups(&self, player_id: &str) -> u8 {
        const CANONICAL_GROUPS: [&str; 5] = ["μ's", "Aqours", "虹ヶ咲", "Liella!", "蓮ノ空"];
        let player = if player_id == self.player2.id {
            &self.player2
        } else {
            &self.player1
        };
        let mut count = 0u8;
        for group in CANONICAL_GROUPS {
            let matched = player.stage.stage.iter().any(|&cid| {
                cid != -1
                    && crate::ability::util::card_matches_group_str(
                        &self.card_database,
                        cid,
                        Some(group),
                    )
            });
            if matched {
                count += 1;
            }
        }
        count
    }

    /// Effective ACTIVE-energy cost of `cost` for `player_id` given the
    /// current board (printed total minus per-group reductions, clamped).
    /// Generation gating, cost display and the execution pre-check must all
    /// route through this so they can never diverge.
    pub fn effective_activation_cost(&self, player_id: &str, cost: &crate::card::AbilityEffect) -> u8 {
        self.effective_activation_cost_for(cost, self.distinct_stage_groups(player_id))
    }

    /// As [`Self::effective_activation_cost`] but with the group count
    /// supplied by the caller.
    pub fn effective_activation_cost_for(
        &self,
        cost: &crate::card::AbilityEffect,
        groups_on_stage: u8,
    ) -> u8 {
        cost.effective_energy_cost_total(groups_on_stage)
    }

    /// Set just_completed_ability_key, process pending auto abilities, then clear it.
    pub fn process_with_completed_key(&mut self, key: Option<u32>, player_id: &str) {
        self.just_completed_ability_key = key;
        self.process_pending_auto_abilities(player_id);
        self.just_completed_ability_key = None;
    }

    /// Clear movement tracking state after ability resolution.
    pub fn clear_movement_tracking(&mut self) {
        self.clear_recently_moved_batch();
        self.recently_appeared_cards.clear();
        self.recently_state_changed.clear();
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
                &self.mods.heart_copy,
            )
        });
        for card_id in &player.live_card_zone.cards {
            if let Some(card) = self.card_database.get_card(*card_id) {
                if let Some(ref need_heart) = card.need_heart {
                    let effective_need = {
                        // Q115/Q127: Start from base requirements for every color.
                        // A set modifier on one color does NOT erase other colors.
                        let mut hearts = need_heart.hearts.clone();
                        if let Some(color_mods) = self.mods.need_heart_modifiers.get(card_id) {
                            // Apply set overrides per-color first.
                            for (color, me) in color_mods {
                                if me.set != 0 {
                                    hearts.insert(*color, me.set as u8);
                                }
                            }
                            // Then apply additive modifiers.
                            for (color, me) in color_mods {
                                if me.additive != 0 {
                                    *hearts.entry_or_default(*color) =
                                        crate::constants::saturate_u8(
                                            hearts.get(color).copied().unwrap_or(0) as i32
                                                + me.additive as i32,
                                        );
                                }
                            }
                        }
                        crate::card::BaseHeart { hearts }
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
            for ar in &card.abilities {
                let ability = ar.resolve();
                if Self::ability_matches_trigger(
                    &ability,
                    &crate::game_state::AbilityTrigger::Constant,
                ) {
                    if let Some(ref effect) = ability.effect {
                        let res_dest = effect.restricted_destination_any();
                        let dest = effect.destination.map(|d| d.to_str());
                        let restricted_to = res_dest.or(dest);
                        if effect.action == crate::ability::enums::ActionType::Restriction
                            && effect.restriction_type_any().as_deref() == Some("cannot_place")
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
                    // UNREACHABLE today: no caller passes "as_long_as" to
                    // push_temporary_effect (the 62 「〜かぎり、Econstants run
                    // through recalculate_constants instead). If this arm ever
                    // fires, real condition re-evaluation must be implemented  E                    // expiring at live end is an approximation.
                    log::warn!(
                        "AsLongAs temporary effect expired via live-end approximation \
                         (condition re-eval not implemented): {}",
                        effect.description
                    );
                    self.current_turn_phase != TurnPhase::Live
                }
                Duration::Unless => {
                    log::warn!(
                        "Unless temporary effect expired via live-end approximation \
                         (negated condition re-eval not implemented): {}",
                        effect.description
                    );
                    self.current_turn_phase != TurnPhase::Live
                }
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
                // LiveEnd-scoped cheer-check state: the base and all
                // modify_yell_count modifiers belong to the finished live.
                self.cheer_check_base = None;
                self.yell_count_modifiers.clear();
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
                        if let Some(card_id) = data.card_id() {
                            self.mods.clear_blade_set_modifier(card_id);
                            log::debug!("Cleared set_blade_count modifier for card {}", card_id);
                        }
                    }
                }
                s if s.starts_with("gain_blade") => {
                    if let Some(ref data) = effect.effect_data {
                        for item in data.items() {
                            self.mods.remove_blade_modifier(item.card_id, item.amount);
                            log::debug!(
                                "Reverted {} blades from card {}",
                                item.amount,
                                item.card_id
                            );
                        }
                    }
                }
                "gain_surplus_heart" => {
                    if let Some(ref data) = effect.effect_data {
                        if let Some(old) = data.old_value() {
                            let is_p1 = data.is_p1().unwrap_or(true);
                            if is_p1 {
                                self.self_live_surplus_count = old;
                            } else {
                                self.opponent_live_surplus_count = old;
                            }
                            log::debug!("Restored surplus count (is_p1={}) to {}", is_p1, old);
                        }
                    }
                }
                s if s.starts_with("gain_heart") => {
                    if let Some(ref data) = effect.effect_data {
                        for item in data.items() {
                            let color_str = item.color.unwrap_or("heart01");
                            let color = crate::card::parse_heart_color(color_str);
                            self.mods
                                .remove_heart_modifier(item.card_id, color, item.amount);
                            log::debug!(
                                "Reverted {} hearts from card {} (color {:?})",
                                item.amount,
                                item.card_id,
                                color
                            );
                        }
                    }
                }
                "heart_override" => {
                    if let Some(ref data) = effect.effect_data {
                        if let Some(card_id) = data.card_id() {
                            self.mods.remove_heart_override(card_id);
                            log::debug!("Removed heart override for card {}", card_id);
                        }
                    }
                }
                "modify_cost" => {
                    if let Some(ref data) = effect.effect_data {
                        for item in data.items() {
                            self.mods.remove_cost_modifier(item.card_id, item.amount);
                            log::debug!(
                                "Reverted cost modifier {} from card {}",
                                item.amount,
                                item.card_id
                            );
                        }
                    }
                }
                s if s.starts_with("gain_ability:") => {
                    // Structured path: the registration stashed the owning
                    // card + immediate-application info in effect_data, so
                    // revert exactly what was applied. Only per-card score
                    // gains get an immediate modifier reverted; live-total
                    // gains were never applied per card (they live in the
                    // p*_constant_total_score_bonus accumulator and expire
                    // with the gained_card_abilities entry itself).
                    if let Some(ref data) = effect.effect_data {
                        if let crate::core::types::EffectData::GainAbility {
                            card_id,
                            amount,
                            is_live_total,
                        } = data
                        {
                            if !is_live_total && *amount != 0 {
                                self.mods.remove_score_modifier(*card_id, *amount);
                                log::debug!(
                                    "Reverted gained ability score modifier +{} for card {}",
                                    amount,
                                    card_id
                                );
                            }
                            self.clear_gained_abilities_for_card(*card_id);
                        }
                    }
                }
                "set_heart_type" => {
                    if let Some(ref data) = effect.effect_data {
                        if let Some(card_id) = data.card_id() {
                            self.mods.heart_color_multiplier.remove(&card_id);
                            log::debug!("Removed heart color multiplier for card {}", card_id);
                        }
                    }
                }
                s if s.starts_with("set_blade_type:") => {
                    if let Some(ref data) = effect.effect_data {
                        if let Some(card_id) = data.card_id() {
                            self.mods.clear_blade_type_modifier(card_id);
                            log::debug!("Cleared blade type modifier for card {}", card_id);
                        }
                    }
                }
                s if s.starts_with("modify_score_") => {
                    if let Some(ref data) = effect.effect_data {
                        for item in data.items() {
                            if s == "modify_score_set" {
                                self.mods.clear_score_set_modifier(item.card_id);
                                log::debug!("Cleared score set modifier for card {}", item.card_id);
                            } else {
                                self.mods.remove_score_modifier(item.card_id, item.amount);
                                log::debug!(
                                    "Removed score modifier {} from card {}",
                                    item.amount,
                                    item.card_id
                                );
                            }
                        }
                    }
                }
                _ => {
                    // An effect kind with no revert arm means its modifiers
                    // LEAK past expiry. Loud on purpose  Eextend this match.
                    log::warn!(
                        "expired temporary effect '{}' has no revert handler; \
                         its modifiers were NOT reverted. description={}",
                        effect.effect_type,
                        effect.description
                    );
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
        if self.current_turn_phase != TurnPhase::Live && !self.wait_immune_members.is_empty() {
            self.wait_immune_members.clear();
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
        self.p1_live_success_this_turn = false;
        self.p1_live_success_no_excess = false;
        self.p2_live_success_this_turn = false;
        self.p2_live_success_no_excess = false;
        self.live_success_triggered_this_turn = false;
        self.live_success_p2_fired = false;
        self.live_success_p1_extra = 0;
        self.live_success_p2_extra = 0;
        self.last_state_change_wait_to_active_count = 0;
        self.recently_state_changed.clear();
        // NOTE: turn_state_changes is NOT cleared here  Ethis runs on every
        // Active-phase entry (each player's normal phase), but 「このターン、E        // spans the whole round (both players' main phases + live). It is
        // cleared at the real turn boundary in advance_phase (victory
        // determination, turn_number increment).
        self.self_no_excess_heart_this_turn = false;
    }

    pub fn check_permanent_loop(&mut self) -> bool {
        let state_hash = self.generate_state_hash();

        if self.game_state_history.contains(&state_hash) {
            self.loop_detected = true;
            return true;
        }

        self.game_state_history.push(state_hash);

        false
    }

    fn generate_state_hash(&self) -> u64 {
        use core::hash::{Hash, Hasher};
        struct SimpleHasher(u64);
        impl Hasher for SimpleHasher {
            fn write(&mut self, bytes: &[u8]) {
                for &b in bytes {
                    self.0 = self.0.wrapping_mul(31).wrapping_add(b as u64);
                }
            }
            fn finish(&self) -> u64 {
                self.0
            }
        }
        let mut hasher = SimpleHasher(0);
        self.turn_number.hash(&mut hasher);
        self.current_phase.hash(&mut hasher);
        self.current_turn_phase.hash(&mut hasher);
        self.player1.hand.cards.len().hash(&mut hasher);
        self.player1.energy_zone.cards.len().hash(&mut hasher);
        self.player1.waitroom.cards.len().hash(&mut hasher);
        self.player1.live_card_zone.cards.len().hash(&mut hasher);
        self.player1
            .success_live_card_zone
            .cards
            .len()
            .hash(&mut hasher);
        self.player1.stage.stage.hash(&mut hasher);
        self.player2.hand.cards.len().hash(&mut hasher);
        self.player2.energy_zone.cards.len().hash(&mut hasher);
        self.player2.waitroom.cards.len().hash(&mut hasher);
        self.player2.live_card_zone.cards.len().hash(&mut hasher);
        self.player2
            .success_live_card_zone
            .cards
            .len()
            .hash(&mut hasher);
        self.player2.stage.stage.hash(&mut hasher);
        self.mods.orientation_modifiers.len().hash(&mut hasher);
        self.prohibition_effects.len().hash(&mut hasher);
        self.temporary_effects.len().hash(&mut hasher);
        self.rps_winner.hash(&mut hasher);
        hasher.finish()
    }

    pub fn reset_loop_detection(&mut self) {
        self.game_state_history.clear();
        self.loop_detected = false;
    }

    pub fn is_loop_detected(&self) -> bool {
        self.loop_detected
    }
}
