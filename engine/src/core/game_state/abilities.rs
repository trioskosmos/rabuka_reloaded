use crate::core::constants::CountCast;
use super::{GameResult, GameState, PermanentLoopProtocol};
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use crate::ability::enums::Zone;
use crate::card::Condition;
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
        // No destination specified -- assume the restriction targets the
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
        use crate::triggers::TriggerKind;
        match trigger {
            crate::game_state::AbilityTrigger::Activation => {
                ability.has_trigger(TriggerKind::Activation)
            }
            crate::game_state::AbilityTrigger::Debut => ability.has_trigger(TriggerKind::Debut),
            crate::game_state::AbilityTrigger::LiveStart => {
                ability.has_trigger(TriggerKind::LiveStart)
            }
            crate::game_state::AbilityTrigger::LiveSuccess => {
                ability.has_trigger(TriggerKind::LiveSuccess)
            }
            crate::game_state::AbilityTrigger::Constant => {
                ability.has_trigger(TriggerKind::Constant)
            }
            crate::game_state::AbilityTrigger::Auto => ability.has_trigger(TriggerKind::Auto),
        }
    }

    pub fn is_ability_invalidated(&self, card_id: i16, trigger: &AbilityTrigger) -> bool {
        self.ability_invalidations
            .iter()
            .any(|entry| entry.card_id == card_id && &entry.trigger == trigger)
            || self.negated_abilities.contains(&card_id)
    }

    pub fn card_has_ability_trigger(&self, card_id: i16, trigger: &AbilityTrigger) -> bool {
        self.card_database
            .get_card(card_id)
            .is_some_and(|card| {
                card.abilities
                    .iter()
                    .any(|ability| Self::ability_matches_trigger(&ability.resolve(), trigger))
            })
            || self
                .gained_card_abilities
                .get(&card_id)
                .is_some_and(|abilities| {
                    abilities
                        .iter()
                        .any(|ability| Self::ability_matches_trigger(ability, trigger))
                })
    }

    pub fn try_add_ability_invalidation(
        &mut self,
        card_id: i16,
        trigger: AbilityTrigger,
        duration: Duration,
    ) -> bool {
        let already_invalidated = self.is_ability_invalidated(card_id, &trigger);
        let has_trigger = self.card_has_ability_trigger(card_id, &trigger);
        log::debug!(
            "[INVALIDATE_REGISTER] card_id={} trigger={:?} duration={:?} already_invalidated={} has_trigger={}",
            card_id,
            trigger,
            duration,
            already_invalidated,
            has_trigger
        );
        if already_invalidated || !has_trigger {
            return false;
        }
        let created_turn = self.turn_number;
        self.ability_invalidations
            .push(crate::core::types::AbilityInvalidation {
                card_id,
                trigger: trigger.clone(),
                duration,
                created_turn,
            });
        log::debug!(
            "[INVALIDATE_REGISTERED] card_id={} trigger={:?} duration={:?} created_turn={}",
            card_id,
            trigger,
            duration,
            created_turn
        );
        true
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

    /// Record a completed ability for the re-scan guard, snapshotting the
    /// current movement batch alongside the key. The guard (see
    /// `just_completed_batch_matches`) skips only re-scans of this same
    /// batch — a fresh batch may re-fire the ability (turn2+ budgets).
    pub fn set_just_completed(&mut self, key: Option<u32>) {
        self.just_completed_ability_key = key;
        self.just_completed_moved = self.recently_moved_cards.clone().unwrap_or_default();
    }

    /// Re-scan guard predicate: skip the just-completed ability only when the
    /// scan shows the SAME movement batch it resolved on (stale re-scan) or
    /// when either side is empty (batch unknown — preserve legacy caution).
    /// A scan with a DIFFERENT non-empty batch is a fresh event and may
    /// re-fire the ability (turn2+ budgets). This is the only behavior change
    /// vs the old skip-on-key-match: same-key + different-nonempty-batch now
    /// fires instead of being silently swallowed.
    fn just_completed_batch_matches(&self, moved: &[i16]) -> bool {
        if self.just_completed_moved.is_empty() || moved.is_empty() {
            return true;
        }
        self.just_completed_moved.as_slice() == moved
    }

    /// Record one use of a limited ability this turn. This is the **only** method
    /// that mutates `turn_limited_abilities_used`.
    ///
    /// Two guarantees make the per-turn limit robust regardless of how the caller
    /// reached us:
    ///   - *Once-per-activation*: a single activation that resolves across several
    ///     phases/choices (cost -- effect -- optional follow-up) consumes exactly one
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
            log::trace!("[use_limit] {key:?} already recorded this activation -- skipping");
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
        crate::ability_queue::AbilityQueueEntry {
            card_no,
            // Normalise "player1"/"player2" to "p1"/"p2" exactly as before,
            // but move the incoming String through untouched in the common
            // case where it is already canonical — the old `match` rebuilt an
            // identical String on every single enqueue.
            player_id: if player_id == "player1" {
                "p1".to_string()
            } else if player_id == "player2" {
                "p2".to_string()
            } else {
                player_id
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
                    )
                        && ability.effect.is_some() {
                            ids.push((cid, idx));
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
    /// Returns lightweight index pairs instead of cloned AbilityEffects --
    /// callers re-lookup through the card_database Arc to avoid 152B clones.
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
    ///
    /// Check if a condition describes an event that can be evaluated at
    /// scanning time.  Event-based conditions depend on tracking flags
    /// (recently_moved_cards, cards_moved_this_turn, cards_appeared_this_turn)
    /// that are set before TAS runs.  Other types (state, position, group,
    /// comparison) depend on game state that may change between TAS and
    /// ability resolution, so they are deferred.
    fn condition_is_event_based(condition: &crate::card::Condition) -> bool {
        let movement = condition.get_movement();
        // All movement types ("moved", "moves", and "position_change") are event-based.
        // "moved" -- card has already moved (can be checked now).
        // "moves" -- card is / was moving (checkable because we set
        //   activating_card to the scanned card, and cards_moved_this_turn
        //   is persistent across the turn).
        // "position_change" -- card changed position on stage (detected via
        //   explicit PositionChangeEvent records, no snapshot dependency).
        if movement == Some("moved")
            || movement == Some("moves")
            || movement == Some("position_change")
            || movement == Some("baton_touch")
            || movement == Some("live_success")
        {
            return true;
        }
        // Appearance -- NOT pre-filtered. Evaluated at resolution time via
        // can_activate_effect so that both self-triggers (Q245) and
        // group-matching are handled correctly with full resolution context.
        // card_count (all variants -- zone counts are stable state checks)
        // Exception: conditions on revealed_cards are NOT event-based because
        // revealed_cards is populated during yell, not at the trigger event.
        if matches!(condition, crate::card::Condition::Location { .. })
            && condition.get_location() != Some("revealed_cards")
        {
            return true;
        }
        // State change (active↔wait) -- pre-filter so the condition only
        // fires when a recorded transition is available.
        if matches!(condition, crate::card::Condition::State { .. }) {
            return true;
        }
        // Recurse into compound conditions -- if any child is event-based,
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
            moved_cards: self.recently_moved_cards.clone().unwrap_or_default(),
            moved_from_zone: self.recently_moved_from_zone.clone(),
            ..Default::default()
        };
        self.trigger_auto_abilities_for_player_with_event(player_id, &event);
    }

    /// Post-movement scan wrapper: fires the TAS with the standard
    /// post-movement snapshot (recently moved cards + position-change flag).
    pub fn trigger_auto_abilities_for_movement(&mut self, player_id: &str) {
        let event = crate::ability::types::TriggerEvent {
            moved_cards: self.recently_moved_cards.clone().unwrap_or_default(),
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
    /// Can this condition never be satisfied by a card that is on stage?
    ///
    /// A 「〜が自分の控え室にある場合」 trigger belongs to a card in the
    /// discard, so a staged card carrying one would fire it permanently
    /// prematurely. `preceding_moved` watchers are exempt: they track OTHER
    /// cards reaching the discard, not this card being in it.
    fn is_stale_discard_condition(condition: &Condition, card_is_in_discard: bool) -> bool {
        if condition.get_source() == Some("preceding_moved") || card_is_in_discard {
            return false;
        }
        let location = condition
            .get_location()
            .or_else(|| {
                condition
                    .get_trigger_event()
                    .and_then(|t| t.location.as_deref())
            })
            .unwrap_or("");
        Zone::from_str(location) == Some(Zone::Discard)
            && (condition.get_card_type().as_deref() == Some("member_card")
                || condition.get_target() == Some("self"))
    }

    /// Claim `num_key` for this movement batch. False when the ability must not
    /// run again here.
    ///
    /// Two guards live here because the stage and live-card scans both need
    /// them and they must not drift: the just-completed ability re-fires only
    /// on a FRESH batch, and nothing already triggered in THIS batch may
    /// re-enqueue.
    fn claim_batch_slot(
        triggered: &mut SmallVec<[u32; 16]>,
        num_key: u32,
        skip_key: Option<u32>,
        same_batch_as_completed: bool,
    ) -> bool {
        if skip_key == Some(num_key) && same_batch_as_completed {
            return false;
        }
        if triggered.contains(&num_key) {
            return false;
        }
        triggered.push(num_key);
        true
    }

    /// Is this marker-carrying watcher already armed by the movement hook?
    ///
    /// A 「対戦相手のカードの効果でも発動する。」 watcher on an AREA MOVE is
    /// enqueued by push_movement_event, and only when the move was caused by
    /// the other player. Attribute via the watcher's own turn-scoped move
    /// record, which is stable across rescan passes unlike the batch sets.
    /// Energy-placement watchers have no such record; the generic path below
    /// handles both causes for them.
    fn hooked_by_foreign_cause(
        &self,
        ability: &crate::card::Ability,
        card_id: i16,
        player_id: &str,
        card_name: &str,
    ) -> bool {
        if !ability
            .effect
            .as_ref()
            .is_some_and(|e| e.fires_on_opponent_effects())
        {
            return false;
        }
        let Some(rec) = self
            .turn_area_movements
            .iter()
            .rev()
            .find(|m| m.moved_card_id == card_id)
        else {
            return false;
        };
        if rec.cause_player_id == player_id {
            return false;
        }
        log::debug!(
            "[TRIGGER_SCOPE] {} last move caused by {} (hook owns foreign-cause firings)",
            card_name,
            rec.cause_player_id
        );
        true
    }

    /// A 「置かれた」 trigger is about a card that was JUST placed, so it
    /// requires the card to be among this event's moves rather than merely
    /// standing on the field.
    fn requires_this_move(cond: &Condition, card_id: i16, moved_cards: &[i16]) -> bool {
        cond.get_self_target().unwrap_or(false)
            && cond.get_movement() == Some("moved")
            && cond.get_locations().is_none_or(|l| l.len() < 2)
            && !moved_cards.contains(&card_id)
    }

    pub fn trigger_auto_abilities_for_player_with_event(
        &mut self,
        player_id: &str,
        event: &crate::ability::types::TriggerEvent,
    ) {
        // Called 4x per card play (self + opponent, before and after the
        // placement). This is the scan, not the resolve: separating it from
        // trig::process_pending is what tells you whether a slow turn is
        // spent LOOKING for triggers or RUNNING them.
        #[cfg(not(feature = "no_std"))]
        let _timer = crate::timer::Timer::start("trig::auto_scan");
        let queued_before = self.ability_queue.len();
        let player_id_clone = player_id.to_string();
        let mut abilities_to_trigger: Vec<(i16, usize, i16)> = Vec::new();
        let skip_this_card_auto_key = self.just_completed_ability_key;
        let just_completed_batch_matches =
            self.just_completed_batch_matches(&event.moved_cards);
        {
            // Copy the two id lists instead of borrowing the player. Both scans
            // need `&mut self` to evaluate conditions under a temporary
            // activating card, and a borrow of self.player1 held across the loop
            // body would forbid that.
            let scanning_player = if player_id_clone == self.player1.id {
                &self.player1
            } else {
                &self.player2
            };
            let stage_ids: SmallVec<[i16; 3]> =
                scanning_player.stage.stage.iter().copied().collect();
            let live_ids: SmallVec<[i16; 8]> =
                scanning_player.live_card_zone.cards.iter().copied().collect();
            // Scan stage cards for AUTO abilities
            for (stage_idx, &card_id) in stage_ids.iter().enumerate() {
                let card_position = crate::ability::util::pos_to_area(stage_idx);
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
                        // Resolution-watchers (「…能力が解決したとき」) arm ONLY
                        // via the post-resolution hook; a board-state scan would
                        // fire them on every pass.
                        if ability
                            .effect
                            .as_ref()
                            .is_some_and(|e| Self::effect_is_ability_resolution_watcher(e))
                        {
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
                                    log::trace!(
                                        "[AUTO_SCAN] trigger={:?} has_condition={}",
                                        ability.triggers,
                                        effect.condition.is_some(),
                                    );
                                }
                                if let Some(ref condition) = effect.condition {
                                    // A 「自分の控え室にある」 condition can never be
                                    // satisfied by a card that is on stage, so such an
                                    // auto ability is skipped rather than queued and
                                    // rejected later.
                                    let in_discard = self.player1.waitroom.cards.contains(&card_id)
                                        || self.player2.waitroom.cards.contains(&card_id);
                                    if Self::is_stale_discard_condition(condition, in_discard) {
                                        continue;
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
                                    if Self::condition_is_event_based(condition) {
                                        let saved_activating = self.activating_card;
                                        self.activating_card = Some(card_id);
                                        let ctx = crate::ability::condition::ConditionContext::with_moved_cards(self, &event.moved_cards);
                                        let passes = ctx.evaluate_condition(condition);
                                        self.activating_card = saved_activating;
                                        if crate::ability::debug::ABILITY_DEBUG
                                            .load(core::sync::atomic::Ordering::Relaxed)
                                        {
                                            log::debug!(
                                                "[AUTO_CONDITION] source={} ({}, id={}) ability={} condition={:?} passes={}",
                                                card.name,
                                                card.card_no,
                                                card_id,
                                                ability_idx,
                                                condition.get_text(),
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
                                    if effect.trigger_type_any() == Some("each_time")
                                        && matches!(
                                            condition.as_ref(),
                                            crate::card::Condition::Comparison { .. }
                                        )
                                        && condition.get_location() == Some("energy_zone")
                                        && !self.last_energy_placed_by_effect()
                                    {
                                        continue;
                                    }
                                }
                                // §9.7.2.1: Compute trigger multiplicity before
                                // the effect block closes -- condition and effect
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
                            // self_target + single-location + movement:"moved"
                            // requires the card to be in this event's moves.
                            if ability.effect.as_ref().is_some_and(|eff| {
                                eff.condition
                                    .as_ref()
                                    .is_some_and(|c| Self::requires_this_move(c, card_id, &event.moved_cards))
                            }) {
                                continue;
                            }
                            // Re-scan guard: skip re-enqueueing the exact auto
                            // ability that just completed (numeric key).
                            let num_key =
            (card_id.u32_count() << 16) | ability_idx.u32_count();
                            // Marker-carrying watchers (「対戦相手のカードの
                            // 効果でも発動する。」) watching AREA MOVES are
                            // armed by the push_movement_event hook when their
                            // LAST move was caused by a foreign player.
                            // Attribute via the watcher's own turn-scoped move
                            // record — stable across rescan passes, unlike
                            // batch sets. (Energy-placement watchers have no
                            // such record; the generic path handles both
                            // causes for them.)
                            if self.hooked_by_foreign_cause(
                                &ability,
                                card_id,
                                &player_id_clone,
                                card.name.as_ref(),
                            ) {
                                continue;
                            }
                            // Re-scan guard: skip the just-completed ability only
                            // on the SAME movement batch it resolved on (stale
                            // re-scan). A fresh batch may re-fire it (turn2+).
                            if !Self::claim_batch_slot(
                                &mut self.this_batch_triggered_ability_ids,
                                num_key,
                                skip_this_card_auto_key,
                                just_completed_batch_matches,
                            ) {
                                continue;
                            }
                            // §9.7.2.1: Multi-trigger -- N trigger instances -- N
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
            for &card_id in &live_ids {
                if let Some(card) = self.card_database.get_card(card_id) {
                    for (ability_idx, ar) in card.abilities.iter().enumerate() {
                        let ability = ar.resolve();
                        if ability
                            .triggers
                            .as_ref()
                            .is_some_and(|t| &**t == crate::triggers::AUTO)
                        {
                            // Resolution-watchers (「…能力が解決したとき」) arm ONLY
                            // via the post-resolution hook; a board-state scan
                            // would fire them on every pass.
                            if ability
                                .effect
                                .as_ref()
                                .is_some_and(|e| Self::effect_is_ability_resolution_watcher(e))
                            {
                                continue;
                            }
                            if let Some(ref effect) = ability.effect {
                                // Live card scan -- uses the same event-based
                                // condition check as the stage loop, without its
                                // debug log. Deliberately NOT shared with the
                                // stage site: evaluating a condition needs
                                // `&mut self`, which the surrounding card borrow
                                // forbids, so it has to be inlined at each site.
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
                                    } else if condition.get_self_target().unwrap_or(false)
                                        && condition
                                            .get_locations()
                                            .is_some_and(|locs| locs.len() == 2)
                                    {
                                        continue;
                                    }
                                }
                            }
                            // Same movement gate for live cards:
                            if ability.effect.as_ref().is_some_and(|eff| {
                                eff.condition.as_ref().is_some_and(|c| {
                                    Self::requires_this_move(c, card_id, &event.moved_cards)
                                })
                            }) {
                                continue;
                            }
                            let num_key =
            (card_id.u32_count() << 16) | ability_idx.u32_count();
                            // Same batch-scoped re-scan guard as the stage loop.
                            if !Self::claim_batch_slot(
                                &mut self.this_batch_triggered_ability_ids,
                                num_key,
                                skip_this_card_auto_key,
                                just_completed_batch_matches,
                            ) {
                                continue;
                            }
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
                            let num_key =             (moved_card_id.u32_count() << 16) | ability_idx.u32_count();
                            // Same batch-scoped re-scan guard as the stage loop.
                            if skip_this_card_auto_key == Some(num_key)
                                && just_completed_batch_matches
                            {
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
        let mut cached_card_id = None;
        let mut cached_card_no = String::new();
        for (card_id, ability_idx, _stage_card_id) in abilities_to_trigger {
            let num_key =
            (card_id.u32_count() << 16) | ability_idx.u32_count();
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
            if cached_card_id != Some(card_id) {
                cached_card_id = Some(card_id);
                cached_card_no = self
                    .card_database
                    .get_card(card_id)
                    .map(|c| String::from(c.card_no.as_ref()))
                    .unwrap_or_default();
            }
            self.trigger_auto_ability_by_index_refs(
                AbilityTrigger::Auto,
                &player_id_clone,
                Some(cached_card_no.as_str()),
                Some(card_id),
                ability_idx,
                moved.clone(),
                None,
            );
        }
        // Consume the energy flag after every TAS scan -- each event should
        // trigger at most one batch of each_time abilities.  The snapshot
        // captured in trigger_auto_ability (above) preserves the flag value
        // for abilities that need it during execution (e.g. Sumire's "moves").
        let queued_new = self.ability_queue.len() > queued_before;
        let deck_emptied_for_player =
            self.deck_emptied_by_effect.as_deref() == Some(player_id_clone.as_str());
        if deck_emptied_for_player {
            if queued_new {
                if player_id_clone == self.player1.id {
                    self.player1.refresh();
                } else {
                    self.player2.refresh();
                }
                log::debug!(
                    "[AUTO_REFRESH] queued_new={} player={} deck_emptied_by_effect=true",
                    self.ability_queue.len() - queued_before,
                    player_id_clone
                );
            }
            self.deck_emptied_by_effect = None;
        }
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
        let match_count = moved_cards
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
            .count()
            .u8_count();
        if match_count <= 1 {
            return match_count;
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
    /// u64. The same watcher arms once PER MOVE -- distinct moves (even within
    /// one turn) each get their own key.
    pub(crate) fn opp_cause_key(num_key: u32, moved_card_id: i16, seq: u16) -> u64 {
        (num_key as u64)
            ^ (u64::from(moved_card_id.u32_count()) << 20)
            ^ (u64::from(seq) << 44).rotate_left(44)
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
                let num_key =
            (watcher_id.u32_count() << 16) | ability_idx.u32_count();
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
                self.trigger_auto_ability_by_index_refs(
                    AbilityTrigger::Auto,
                    owner_pid.as_str(),
                    Some(card_no.as_str()),
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
        // Accepts a `PlayerId`, a `String`, or a `&str` so callers can pass an
        // already-shared id instead of allocating a throwaway copy of
        // "p1"/"p2" purely to hand it over.
        player_id: impl Into<crate::core::player::PlayerId>,
        source_card_id: Option<String>,
        explicit_card_id: Option<i16>,
        trigger_moved_cards: Option<SmallVec<[i16; 4]>>,
        triggering_member_id: Option<i16>,
    ) {
        // Resolve the card once, then try the ability printed on it and finally
        // the abilities it gained. Written as early returns rather than a nest
        // of if-lets so the three steps read in order. Each step searches with
        // `&self` and enqueues with `&mut self`, so the card borrow ends before
        // anything is queued.
        let Some(card_no) = source_card_id else {
            return;
        };
        let player_id = player_id.into();
        let card_id = explicit_card_id
            .or_else(|| self.find_card_by_number_for_player(&card_no, &player_id).1);
        let Some(cid) = card_id else {
            return;
        };
        let Some(card) = self.card_database.get_card(cid) else {
            return;
        };

        if let Some((ability_index, ability)) = self.find_printed_ability(
            &ability_id,
            &card_no,
            card,
            cid,
            &trigger_type,
        ) {
            self.enqueue_printed_ability(
                &ability_id,
                &card_no,
                cid,
                ability_index,
                ability,
                &player_id,
                &trigger_type,
                &trigger_moved_cards,
                triggering_member_id,
            );
            return;
        }
        self.enqueue_gained_ability(
            &ability_id,
            &card_no,
            cid,
            &player_id,
            &trigger_type,
            &trigger_moved_cards,
            triggering_member_id,
        );
    }

    /// The printed ability `ability_id` names, if `card` has it and the trigger
    /// is live on it. `ability_id` is `{card_no}_{text}`, so the text after the
    /// card number is what distinguishes two abilities on the same card.
    fn find_printed_ability(
        &self,
        ability_id: &str,
        card_no: &str,
        card: &crate::card::Card,
        cid: i16,
        trigger_type: &AbilityTrigger,
    ) -> Option<(usize, crate::Arc<crate::card::Ability>)> {
        let requested_text = ability_id
            .strip_prefix(card_no)
            .and_then(|suffix| suffix.strip_prefix('_'));
        for (ability_index, ability) in card.abilities.iter().enumerate() {
            let resolved_ability = ability.resolve();
            if Self::ability_matches_trigger(&resolved_ability, trigger_type)
                && requested_text == Some(resolved_ability.full_text.as_str())
                && !self.is_ability_invalidated(cid, trigger_type)
            {
                return Some((ability_index, ability.to_arc()));
            }
        }
        None
    }

    #[allow(clippy::too_many_arguments)]
    fn enqueue_printed_ability(
        &mut self,
        ability_id: &str,
        card_no: &str,
        cid: i16,
        ability_index: usize,
        ability: crate::Arc<crate::card::Ability>,
        player_id: &str,
        trigger_type: &AbilityTrigger,
        trigger_moved_cards: &Option<SmallVec<[i16; 4]>>,
        triggering_member_id: Option<i16>,
    ) {
        let entry = self.build_ability_queue_entry(
            card_no.to_string(),
            ability_index,
            ability,
            Some(cid),
            player_id.to_string(),
            trigger_type.clone(),
            trigger_moved_cards.clone(),
            triggering_member_id,
        );
        self.push_debug_note_fmt(format_args!(
            "queue+ {} card={} trigger={:?}",
            ability_id, entry.card_no, entry.trigger_type
        ));
        // Snapshot batch_movements and energy flags at enqueue time so the
        // "moves" and energy conditions can check what triggered the ability
        // even after clear_effect_tracking clears the global lists.
        let mut entry = entry;
        entry.snapshot_movements = self.batch_movements.clone();
        if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
            log::debug!(
                "[ABILITY_QUEUE] enqueue owner={} card={} id={:?} ability={} trigger={:?}",
                entry.player_id,
                entry.card_no,
                entry.card_id,
                entry.ability_index,
                entry.trigger_type
            );
        }
        self.ability_queue.enqueue(entry);
    }

    /// Enqueue a gained ability `ability_id` names (`{card_no}_gained_{idx}`),
    /// if this card has it and the trigger is live.
    #[allow(clippy::too_many_arguments)]
    fn enqueue_gained_ability(
        &mut self,
        ability_id: &str,
        card_no: &str,
        cid: i16,
        player_id: &str,
        trigger_type: &AbilityTrigger,
        trigger_moved_cards: &Option<SmallVec<[i16; 4]>>,
        triggering_member_id: Option<i16>,
    ) {
        if !ability_id.contains("_gained_") {
            return;
        }
        // The gained index is the last underscore-separated field.
        let Some(idx_str) = ability_id.rsplit('_').next() else {
            return;
        };
        let Ok(gidx) = idx_str.parse::<usize>() else {
            return;
        };
        // Search before mutating: the gained list is borrowed from self.
        let matched = self.gained_card_abilities.get(&cid).and_then(|list| {
            let ability = list.get(gidx)?;
            (Self::ability_matches_trigger(ability, trigger_type)
                && !self.is_ability_invalidated(cid, trigger_type))
            .then(|| crate::Arc::new(ability.clone()))
        });
        let Some(ability) = matched else {
            return;
        };
        let entry = self.build_ability_queue_entry(
            card_no.to_string(),
            crate::ability::types::GAINED_ABILITY_INDEX_BASE + gidx,
            ability,
            Some(cid),
            player_id.to_string(),
            trigger_type.clone(),
            trigger_moved_cards.clone(),
            triggering_member_id,
        );
        let mut entry = entry;
        entry.snapshot_movements = self.batch_movements.clone();
        self.ability_queue.enqueue(entry);
    }

    /// Hot-path version of trigger_auto_ability that takes a numeric ability index
    /// instead of a string key. Avoids format!() allocations in the TAS scan loop.
    pub fn trigger_auto_ability_by_index(
        &mut self,
        trigger_type: AbilityTrigger,
        player_id: impl Into<crate::core::player::PlayerId>,
        source_card_id: Option<String>,
        explicit_card_id: Option<i16>,
        ability_index: usize,
        trigger_moved_cards: Option<SmallVec<[i16; 4]>>,
        triggering_member_id: Option<i16>,
    ) {
        self.trigger_auto_ability_by_index_refs(
            trigger_type,
            &player_id.into(),
            source_card_id.as_deref(),
            explicit_card_id,
            ability_index,
            trigger_moved_cards,
            triggering_member_id,
        )
    }

    fn trigger_auto_ability_by_index_refs(
        &mut self,
        trigger_type: AbilityTrigger,
        player_id: &str,
        source_card_id: Option<&str>,
        explicit_card_id: Option<i16>,
        ability_index: usize,
        trigger_moved_cards: Option<SmallVec<[i16; 4]>>,
        triggering_member_id: Option<i16>,
    ) {
        if let Some(card_id) = explicit_card_id {
            if self.is_ability_invalidated(card_id, &trigger_type) {
                return;
            }
            if let Some(card) = self.card_database.get_card(card_id) {
                if let Some(ar) = card.abilities.get(ability_index) {
                    let entry = self.build_ability_queue_entry(
                        source_card_id.unwrap_or_default().to_string(),
                        ability_index,
                        ar.to_arc(),
                        Some(card_id),
                        player_id.to_string(),
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
                            "[ABILITY_QUEUE] enqueue owner={} card={} id={:?} ability={} trigger={:?}",
                            entry.player_id,
                            entry.card_no,
                            entry.card_id,
                            entry.ability_index,
                            entry.trigger_type
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

    /// Recursive condition-tree search: first non-empty group filter found.
    fn condition_tree_group_names(
        cond: &crate::card::Condition,
    ) -> Option<&[String]> {
        if let Some(g) = cond.get_group_names() {
            if !g.is_empty() {
                return Some(g);
            }
        }
        if let Some(children) = cond.get_conditions() {
            for c in children {
                if let Some(g) = Self::condition_tree_group_names(c) {
                    return Some(g);
                }
            }
        }
        None
    }

    /// True for 自動 abilities whose trigger clause watches an ability
    /// RESOLUTION (「…（ライブ開始時|ライブ成功時）能力が解決したとき」).
    /// These arm ONLY via `trigger_each_time_for_member` after a real
    /// LS/LSS ability completes — their group/location condition also reads
    /// as a static board query, so the TAS must never fire them on its own.
    fn effect_is_ability_resolution_watcher(effect: &crate::card::AbilityEffect) -> bool {
        effect.trigger_type_any() == Some("each_time")
            && effect.watches_ability_resolution.unwrap_or(false)
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
        // Only fire for stage member cards -- live cards' own LiveStart/LiveSuccess
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
                    if effect.trigger_type_any() != Some("each_time") {
                        continue;
                    }
                    let watch_text = match &effect.condition {
                        Some(c) => c.get_text(),
                        None => Some(effect.text.as_ref()),
                    };
                    if !watch_text.is_some_and(|t| t.contains(trigger_substring)) {
                        continue;
                    }
                    // The triggering member must satisfy the watcher's GROUP
                    // filter (e.g. 『μ's』のメンバーの…能力が解決したとき).
                    // Q255 keeps the position requirement loose — the member
                    // may already have left center by resolution time — so
                    // only group identity is enforced here.
                    if let Some(groups) = effect
                        .condition
                        .as_ref()
                        .and_then(|c| Self::condition_tree_group_names(c))
                    {
                        let member_matches = groups.iter().any(|g| {
                            crate::ability::util::card_matches_group_str(
                                &self.card_database,
                                member_card_id,
                                Some(g.as_str()),
                            )
                        });
                        log::debug!(
                            "[RESOLVE_WATCHER] card={} member={} groups={:?} match={}",
                            card.card_no,
                            member_card_id,
                            groups,
                            member_matches
                        );
                        if !member_matches {
                            continue;
                        }
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
            self.trigger_auto_ability_by_index_refs(
                crate::game_state::AbilityTrigger::Auto,
                &player_id_clone,
                Some(card_no.as_str()),
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
                    .unwrap_or_else(|| self.ability_queue.len().u16_count())
            as usize;
            self.depth_first_cutoff = None;

            let mut available_indices = (0..pre_len).filter(|&i| {
                self.ability_queue.is_entry_available(i)
                    && self.ability_queue.entry_player_id(i) == Some(player_id)
            });
            let first = available_indices.next();
            let second = available_indices.next();

            if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
                let debug_indices: Vec<usize> = (0..pre_len)
                    .filter(|&i| {
                        self.ability_queue.is_entry_available(i)
                            && self.ability_queue.entry_player_id(i) == Some(player_id)
                    })
                    .collect();
                log::trace!("[QUEUE_SCAN] owner={} cutoff={} available_indices={:?}", player_id, pre_len, debug_indices);
            }

            let Some(idx) = first else {
                break;
            };

            if let Some(second) = second {
                let options = core::iter::once(idx)
                    .chain(core::iter::once(second))
                    .chain(available_indices)
                    .filter_map(|idx| {
                        let entry = self.ability_queue.get_entry(idx)?;
                        let cid = entry.card_id.unwrap_or(0);
                        let card_name = self
                            .card_database
                            .get_card(cid)
                            .map(|c| c.name.to_string())
                            .unwrap_or_else(|| entry.card_no.to_string());
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
                        // Sub-resolution may queue deeper entries -- while loop catches them
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
        //   I put all of them back with one trigger?" -- No, only 1 card per
        //   trigger instance. The batch scan fires the ability once per batch,
        //   and the ability selects 1 card from the batch.
        let moved_marker = self.recently_moved_cards.is_some();
        let appeared_marker = !self.recently_appeared_cards.is_empty();
        if moved_marker || self.last_energy_placed_by_effect() || appeared_marker {
            let event = crate::ability::types::TriggerEvent {
                moved_cards: self
                    .batch_movements
                    .iter()
                    .map(|m| m.moved_card_id)
                    .collect(),
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
        #[cfg(not(feature = "no_std"))]
        let _timer = crate::timer::Timer::start("trig::process_pending");
        let active_player_id = match raw_player_id {
            "player1" => "p1",
            "player2" => "p2",
            other => other,
        };
        log::trace!("[QUEUE_SCAN] raw={} active={} queue_len_before={}", raw_player_id, active_player_id, self.ability_queue.len());
        // Rule 9.5.3.2: Active player resolves ALL their standby abilities first
        // (one at a time, back to rule processing between each)
        self.process_player_abilities(active_player_id);
        log::trace!("[QUEUE_SCAN] after active: queue_len={} has_choice={}", self.ability_queue.len(), self.has_pending_choice());
        if self.has_pending_choice() {
            return;
        }

        // Rule 9.5.3.3: Then non-active player resolves ALL theirs
        let non_active_id = self
            .ability_queue
            .pending_entries_iter()
            .find(|e| e.player_id != active_player_id)
            .map(|e| e.player_id.clone())
            .unwrap_or_default();
        if !non_active_id.is_empty() {
            self.process_player_abilities(&non_active_id);
        }
        self.ability_queue.clear_completed();
    }

    /// Resolve the active queue entry while keeping its resolver and completion
    /// state in one borrow of `GameState`. A helper split is not safe without
    /// transferring part of that state out: `resolve_ability` mutably borrows
    /// `GameState`, while the same flow must read and update queue entry flags,
    /// store the resolver for choice resume, and preserve the exact post-resolve
    /// scan and debug-log ordering. Keep this ownership boundary intact.
    pub(crate) fn process_current_ability(&mut self) {
        #[cfg(not(feature = "no_std"))]
        let _timer = crate::timer::Timer::start("resolve::current_ability");
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
        let (card_id, ability, ability_index, trigger_type, cost_already_paid) = {
            let entry = match self.ability_queue.current_entry() {
                Some(e) => e,
                None => return,
            };
            log::debug!(target: "rabuka_engine::events",
                "{} resolving {} ({}, id={:?}) ability={} trigger={:?} resume={} cost_paid={} effect_started={}",
                entry.player_id,
                entry.card_id.and_then(|cid| self.card_database.get_card(cid))
                    .map(|c| c.name.as_ref()).unwrap_or("unknown"),
                entry.card_no,
                entry.card_id,
                entry.ability_index,
                entry.trigger_type,
                self.ability_queue.has_resolver(),
                entry.cost_paid,
                entry.effect_started
            );
            (
                entry.card_id,
                entry.ability.clone(),
                entry.ability_index,
                entry.trigger_type.clone(),
                entry.cost_paid,
            )
        };

        if card_id.is_some_and(|id| self.is_ability_invalidated(id, &trigger_type)) {
            log::debug!(
                "[NEGATED] card_id={:?} is negated -- skipping ability resolution",
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
            if crate::game_setup::logging_enabled()
                || crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed)
            {
                let log_text = format!(
                    "{pp} {card_name} [{zone}]: [[log_ability_result:trigger=trigger_{trigger_str},result=result_skipped_negated]]"
                );
                self.push_rule_log_fmt(format_args!("{}", log_text));
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
            }
            self.ability_queue.complete_current();
            self.activating_card = None;
            self.activating_ability_index = None;
            return;
        }

        self.activating_card = card_id;
        self.activating_ability_index = Some(ability_index);

        // Check if a resolver already exists (e.g., cost phase completed, effect needs to run).
        // If so, reuse it -- it carries state (revealed_cost_cards, etc.) needed by the effect.
        let mut resolver = if self.ability_queue.has_resolver() {
            let mut r = self.ability_queue.take_resolver().unwrap();
            // Don't reset moved_cards/selected_cards -- the effect may need
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

        #[cfg(not(feature = "no_std"))]
        let _t = crate::timer::Timer::start("resolve::resolve_ability_total");
        let resolve_result = resolver.resolve_ability(self, ability.clone(), card_id, ability_index);
        drop(_t);
        match resolve_result {
            Ok(()) => {
                self.push_debug_note_fmt(format_args!(
                    "resolve ok card={:?} idx={} pending_choice={}",
                    card_id,
                    ability_index,
                    resolver.pending_choice.is_some()
                ));
            }
            Err(e) => {
                self.push_debug_note_fmt(format_args!(
                    "resolve FAIL card={:?} idx={} err={}",
                    card_id, ability_index, e
                ));
                log::debug!("[ABILITY_RESOLUTION] card={:?} ability={} error={}", card_id, ability_index, e);
                self.ability_queue.complete_current();
                self.clear_effect_tracking();
                return;
            }
        }
        if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
            log::debug!(
                "[ABILITY_RESOLUTION] card={:?} ability={} status={}",
                card_id,
                ability_index,
                if resolver.pending_choice.is_some() { "waiting_for_choice" } else { "finished" }
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

            // Don't set cost_paid for is_select_action choices -- those are
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
            let route_targets_opponent = self
                .ability_queue
                .current_entry()
                .and_then(|entry| entry.choice_card_no.as_ref())
                .is_some_and(|route| {
                    matches!(route, crate::ability::types::ChoiceRoute::Raw(raw) if raw.starts_with("position_change:opponent:"))
                });
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
                crate::ability::types::Choice::SelectTarget { target, .. }
                    if target == "position|destination"
                        && (resolver.spawn_context.target.as_deref() == Some("opponent")
                            || route_targets_opponent) =>
                {
                    true
                }
                _ => false,
            };

            // If the choice has an explicit picker, use that to determine who makes the choice
            let picker = match c {
                crate::ability::types::Choice::SelectCard { picker: Some(ref p), .. } => Some(p.as_str()),
                _ => None,
            };
            if let Some(entry) = self.ability_queue.current_entry_mut() {
                let preserved_picker = entry.choice_player_id.is_some();
                // Preserve choice_player_id if already set by executor (e.g., execute_choice uses choice_maker)
                if entry.choice_player_id.is_none() {
                    let choice_player = if let Some(p) = picker {
                        if p == "opponent" {
                            if entry.player_id == "p1" { "p2" } else { "p1" }
                        } else {
                            &entry.player_id
                        }
                    } else if targets_opponent {
                        if entry.player_id == "p1" { "p2" } else { "p1" }
                    } else {
                        &entry.player_id
                    };
                    entry.choice_player_id = Some(choice_player.to_string());
                }
                log::debug!(
                    "[CHOICE_ROUTING] owner={} picker={:?} explicit_picker={:?} preserved={} targets_opponent={} spawn={:?} action={:?}",
                    entry.player_id,
                    entry.choice_player_id,
                    picker,
                    preserved_picker,
                    targets_opponent,
                    resolver.spawn_context.target,
                    resolver.current_effect.as_ref().map(|e| e.action)
                );
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
        let cid = e.card_id?.u32_count();
        let idx = e.ability_index.u32_count();
                    Some((cid << 16) | idx)
                });

            self.ability_queue.complete_current();
            // Keep activating_card/ability_index alive through the post-resolution
            // TAS scan below -- the guard at line 331-335 uses them to prevent
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
                    log::trace!(
                        "[AUTO_SCAN] scanning stage watchers pid={} moved={:?}",
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
                self.set_just_completed(just_completed_key);
                self.trigger_auto_abilities_for_player_with_event(&current_pid, &event);
                // just_completed_ability_key intentionally NOT cleared here --                 // process_pending_auto_abilities' post-loop TAS (line ~803)
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
        self.pending_loop_protocol
            .as_ref()
            .map(|protocol| &protocol.choice)
            .or_else(|| self.ability_queue.is_waiting_for_choice())
    }

    pub fn has_pending_choice(&self) -> bool {
        self.pending_loop_protocol.is_some() || self.ability_queue.is_waiting_for_choice().is_some()
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
                    .rev()
                    .find(|m| {
                        (m.dest_zone == "energy"
                            || m.dest_zone == "energy_zone"
                            || m.dest_zone == "under_member")
                            && m.effect_only
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
        if self.pending_loop_protocol.is_some() {
            let active = if core::ptr::eq(self.active_player(), &self.player1) {
                &self.player2.id
            } else {
                &self.player1.id
            };
            return Some(active.to_string());
        }
        self.ability_queue
            .current_entry()
            .and_then(|e| e.choice_player_id.as_ref().cloned())
    }

    /// Inject card and ability identity into the pending_choice JSON so the frontend
    /// can display which card+ability is responsible for the current choice prompt.
    /// Get the serialized JSON for the frontend from the ability queue's waiting choice.
    #[cfg(feature = "serde_support")]
    pub fn get_pending_choice_json(&self) -> Option<serde_json::Value> {
        let choice = self.get_pending_choice()?;
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
                //   3. English prompt (last resort) -- and we WARN so the gap is caught.
                if !obj.contains_key("prompt_ja") {
                    let prompt_en = obj
                        .get("prompt_en")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let prompt_ja = crate::ability::describe::translate_choice_prompt_en_to_ja(&prompt_en)
                        .or_else(|| {
                            if entry.choice_effect_text.as_deref().is_some_and(|t| !t.is_empty()) {
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
                        // include ALL cards in selection_cards -- filtered_indices restricts
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
                if let crate::ability::types::Choice::SelectAutoAbility { player_id, .. }
                | crate::ability::types::Choice::SelectLiveSuccess { player_id, .. } =
                    choice
                {
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

    /// Shared master-player resolution — used by both mut/non-mut variants.
    /// Checks the queue entry (ability_master_id) first, then falls back to the
    /// owning player of the activating card. This makes the two arms identical
    /// (previously mut lacked the owner fallback and diverged on "both").
    fn resolve_master_id(&self) -> Option<String> {
        self.ability_master_id().or_else(|| {
            self.activating_card
                .and_then(|cid| self.owner_of_card(cid))
                .map(|p| p.id.to_string())
        })
    }

    pub fn resolve_target_player_mut(&mut self, target: &str) -> &mut Player {
        let master = self.resolve_master_id();
        match (target, master.as_deref()) {
            ("self", Some("player2") | Some("p2")) => &mut self.player2,
            ("self", _) => &mut self.player1,
            ("opponent", Some("player2") | Some("p2")) => &mut self.player1,
            ("opponent", _) => &mut self.player2,
            ("both", _) => {
                log::debug!("WARN: resolve_target_player_mut called with 'both' -- returning player1, use execute_for_targets instead");
                &mut self.player1
            }
            _ => {
                log::debug!(
                    "WARN: resolve_target_player_mut unknown target '{}' -- defaulting to player1",
                    target
                );
                &mut self.player1
            }
        }
    }

    /// Which player owns `card_id`, checked across every card-holding zone.
    /// Single source of truth for owner lookups — used by
    /// [`Self::resolve_target_player`] and by the condition layer's
    /// `resolve_condition_player` (which previously re-implemented this scan,
    /// with a divergent zone list).
    pub fn owner_of_card(&self, card_id: i16) -> Option<&Player> {
        let holds = |p: &Player| {
            p.stage.stage.contains(&card_id)
                || p.hand.cards.contains(&card_id)
                || p.live_card_zone.cards.contains(&card_id)
                || p.energy_zone.cards.contains(&card_id)
                || p.success_live_card_zone.cards.contains(&card_id)
                || p.waitroom.cards.contains(&card_id)
        };
        if holds(&self.player1) {
            Some(&self.player1)
        } else if holds(&self.player2) {
            Some(&self.player2)
        } else {
            None
        }
    }

    pub fn resolve_target_player(&self, target: &str) -> &Player {
        let master = self.resolve_master_id();
        match (target, master.as_deref()) {
            ("self", Some("player2") | Some("p2")) => &self.player2,
            ("self", _) => &self.player1,
            ("opponent", Some("player2") | Some("p2")) => &self.player1,
            ("opponent", _) => &self.player2,
            ("both", _) => {
                log::debug!("WARN: resolve_target_player called with 'both' -- returning player1, use execute_for_targets instead");
                &self.player1
            }
            _ => {
                log::debug!(
                    "WARN: resolve_target_player unknown target '{}' -- defaulting to player1",
                    target
                );
                &self.player1
            }
        }
    }

    /// Return the opponent's player ID given a player ID.
    ///
    /// Returns a cloned [`PlayerId`] (an `Arc<str>` refcount bump) rather than
    /// an owned `String`: the callers hold the result across a `&mut
    /// game_state` use, so a borrow of the game state will not do, but the
    /// allocation was never necessary. This runs once per card play (see
    /// `turn::phases::handle_play_member_to_stage`) and once per
    /// `execute_performance_phase`, not merely on phase transitions.
    pub fn opponent_id(&self, player_id: &str) -> crate::core::player::PlayerId {
        if player_id == self.player1.id {
            self.player2.id.clone()
        } else {
            self.player1.id.clone()
        }
    }

    /// Number of DISTINCT group names among `player_id`'s stage members.
    /// Single source of truth for "グループ名1種類につぁE cost reductions --     /// used by the resolver's runtime cost adjustment AND by action
    /// generation's effective-cost display/gating.
    ///
    /// Membership is resolved through [`crate::ability::util::card_matches_group_str`]
    /// so multi-name joint cards (Q228: LL-bp1-001-R＋ carries 虹ヶ咲 +
    /// Liella! + 蓮ノ空 through its three names) contribute every group they
    /// belong to, not just a single `card.group` field.
    pub fn distinct_stage_groups(&self, player_id: &str) -> u8 {
        const CANONICAL_GROUPS: [&str; 5] = ["μ's", "Aqours", "虹ヶ咲", "Liella!", "蓮ノ空"];
        let player = self
            .try_player_by_id(player_id)
            .unwrap_or(&self.player1);
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
        self.set_just_completed(key);
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
                    // Q115/Q127: start from the base requirements for every
                    // colour, then apply this card's need-heart modifiers.
                    let mut per_colour = [0u8; 8];
                    for (color, count) in &need_heart.hearts {
                        if let Some(slot) = per_colour.get_mut(color.index()) {
                            *slot = *count;
                        }
                    }
                    if let Some(color_mods) = self.mods.need_heart_modifiers.get(card_id) {
                        crate::core::game_modifiers::apply_need_heart_modifiers(
                            &mut per_colour,
                            color_mods,
                        );
                    }
                    // Back to a HeartMap for check_heart_requirement. A colour
                    // at zero is dropped: it can never fail a >= check, and
                    // check_heart_requirement short-circuits on an empty table.
                    let mut effective_need = crate::card::BaseHeart {
                        hearts: crate::card::HeartMap::new(),
                    };
                    for (idx, &n) in per_colour.iter().enumerate() {
                        if n != 0 {
                            effective_need
                                .hearts
                                .insert(crate::card::HeartColor::from_index(idx), n);
                        }
                    }
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
                            && effect.restriction_type_any() == Some("cannot_place")
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

    /// Has `effect` run out of time?
    ///
    /// `AsLongAs` and `Unless` have no condition re-evaluation implemented, so
    /// both approximate to "ends with the live" and say so loudly when they
    /// actually fire — an arm that is unreachable today must not become a
    /// silent wrong answer the day a caller starts passing one.
    fn effect_has_expired(&self, effect: &crate::core::types::TemporaryEffect) -> bool {
        match effect.duration {
            Duration::LiveEnd | Duration::ThisLive => self.current_turn_phase != TurnPhase::Live,
            Duration::ThisTurn => self.turn_number > effect.created_turn,
            Duration::Permanent => false,
            Duration::AsLongAs | Duration::Unless => {
                log::warn!(
                    "{:?} temporary effect expired via live-end approximation \
                     (condition re-eval not implemented): {}",
                    effect.duration,
                    effect.description
                );
                self.current_turn_phase != TurnPhase::Live
            }
        }
    }

    /// Is this ability invalidation still in force?
    ///
    /// The inverse shape of [`Self::effect_has_expired`]: an invalidation is
    /// KEPT while its window is open, rather than dropped when one closes.
    /// Takes the two clock values rather than `&self` so the `retain` that
    /// calls it does not have to borrow the whole GameState.
    fn invalidation_still_active(phase: TurnPhase, turn: u8, entry: &crate::core::types::AbilityInvalidation) -> bool {
        match entry.duration {
            Duration::LiveEnd | Duration::ThisLive | Duration::AsLongAs | Duration::Unless => {
                phase == TurnPhase::Live
            }
            Duration::ThisTurn => turn == entry.created_turn,
            Duration::Permanent => true,
        }
    }

    /// Is this duration one that only lasts as long as the live does?
    fn is_live_scoped(duration: Duration) -> bool {
        matches!(
            duration,
            Duration::LiveEnd | Duration::ThisLive | Duration::AsLongAs | Duration::Unless
        )
    }

    /// Drop the state that belongs to a finished live, but only when a
    /// live-scoped effect is what expired. A ThisTurn effect expiring between
    /// turns must NOT wipe the multiplier mid-live.
    fn clear_finished_live_state(&mut self, expired: &[usize]) {
        let any_live_scoped_expired = expired
            .iter()
            .any(|&i| Self::is_live_scoped(self.temporary_effects[i].duration));
        if !any_live_scoped_expired {
            return;
        }
        self.mods.heart_color_multiplier.clear();
        // LiveEnd-scoped cheer-check state: the base and all
        // modify_yell_count modifiers belong to the finished live.
        self.cheer_check_base = None;
        self.yell_count_modifiers.clear();
    }

    /// Undo one expired effect's modifiers.
    ///
    /// Returns true when the effect was a gained ability: that is the one case
    /// whose removal invalidates a cached constant and so forces a
    /// `recalculate_constants` afterwards.
    fn revert_expired_effect(&mut self, effect: &crate::core::types::TemporaryEffect) -> bool {
        match effect.effect_type.as_str() {
            "activation_cost_increase" | "activation_cost_decrease" => {
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
                    {                        if !is_live_total && *amount != 0 {
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
                return true;
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
                // LEAK past expiry. Loud on purpose -- extend this match.
                log::warn!(
                    "expired temporary effect '{}' has no revert handler; \
                     its modifiers were NOT reverted. description={}",
                    effect.effect_type,
                    effect.description
                );
            }
        }
        false
    }

    /// Clear every list that only means something while a live is running.
    fn clear_live_only_state(&mut self) {
        if self.current_turn_phase == TurnPhase::Live {
            return;
        }
        // e.g. "cannot_live", wait immunities, activation-cost prohibitions.
        self.prohibition_effects.clear();
        self.cannot_live_players.clear();
        self.wait_immune_members.clear();
    }

    pub fn check_expired_effects(&mut self) {
        // A live-total gain_ability registered a 常時 into gained_card_abilities
        // and cached its +1 into p*_constant_total_score_bonus. Expiring must
        // re-derive that accumulator AFTER the registration is cleared, or the
        // cached bonus outlives the ability itself (PL!S-bp3-001-R＋: the +1
        // persisted into turn 2 after ライブ終了時まで). Mirrors the gain side,
        // which refreshes constants right after registering.
        let mut expired_gain_ability = false;

        let expired_indices: Vec<usize> = self
            .temporary_effects
            .iter()
            .enumerate()
            .filter(|(_, effect)| self.effect_has_expired(effect))
            .map(|(i, _)| i)
            .collect();

        let phase = self.current_turn_phase;
        let turn = self.turn_number;
        self.ability_invalidations
            .retain(|entry| Self::invalidation_still_active(phase, turn, entry));

        self.clear_finished_live_state(&expired_indices);

        // Reverse order so each index stays valid as earlier ones are removed.
        for i in expired_indices.into_iter().rev() {
            let effect = self.temporary_effects.remove(i);
            expired_gain_ability |= self.revert_expired_effect(&effect);
        }

        self.clear_live_only_state();

        // Refresh AFTER removals + zone-exit clears: the cached
        // p*_constant_total_score_bonus must reflect the post-expiry
        // constant landscape (see expired_gain_ability above).
        if expired_gain_ability {
            self.recalculate_constants();
        }
    }

    pub fn add_replacement_effect(
        &mut self,
        card_id: i16,
        player_id: impl Into<crate::core::player::PlayerId>,
        original_event: String,
        replacement_effects: Vec<crate::card::AbilityEffect>,
        is_choice_based: bool,
    ) {
        self.replacement_effects.push(ReplacementEffect {
            card_id,
            player_id: player_id.into().to_string(),
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

    pub fn mark_replacement_effect_applied(&mut self, index: usize) {
        if let Some(effect) = self.replacement_effects.get_mut(index) {
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
        self.live_success_p1_fired = false;
        self.live_success_p1_extra = 0;
        self.live_success_p2_extra = 0;
        self.last_state_change_wait_to_active_count = 0;
        self.recently_state_changed.clear();
        // NOTE: turn_state_changes is NOT cleared here -- this runs on every
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

    pub fn record_action_boundary(&mut self, action: crate::game_setup::ActionType) {
        // This is the only caller-visible cost of the Rule 12-1 loop protocol
        // (full-board state hash + O(n) history scan) and it is invisible in
        // profile_target, which calls execute_main_phase_action directly and
        // never reaches here. Time it or it stays invisible.
        #[cfg(not(feature = "no_std"))]
        let _timer = crate::timer::Timer::start("record_action_boundary");
        if self.loop_last_action != Some(action) {
            self.game_state_history.clear();
            self.pending_loop_protocol = None;
            self.loop_last_action = Some(action);
        }
        if self.pending_loop_protocol.is_some() {
            return;
        }
        let hash = self.generate_state_hash();
        self.game_state_history.push(hash);
        let repetition_count = self
            .game_state_history
            .iter()
            .filter(|&&seen| seen == hash)
            .count()
            .u8_count();
        if repetition_count < 3 {
            return;
        }
        self.pending_loop_protocol = Some(PermanentLoopProtocol {
            state_hash: hash,
            repetition_count,
            choice: crate::ability::types::Choice::SelectTarget {
                target: "rule_12_1".to_string(),
                description: "Stop the repeated action and draw the game?".to_string(),
                description_en: Some("Stop the repeated action and draw the game?".to_string()),
                description_ja: Some("繰り返しを停止して引き分けにしますか？".to_string()),
                allow_skip: false,
                options: Some(vec!["Stop".to_string(), "Continue".to_string()]),
            },
        });
        self.loop_detected = true;
    }

    pub fn resolve_loop_protocol(&mut self, continue_loop: bool) {
        if self.pending_loop_protocol.take().is_none() {
            return;
        }
        if !continue_loop {
            self.game_result = GameResult::Draw;
            self.game_ended = true;
            return;
        }
        self.game_state_history.clear();
        self.loop_detected = false;
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
        self.player1.hand.cards.hash(&mut hasher);
        self.player1.main_deck.cards.hash(&mut hasher);
        self.player1.energy_deck.cards.hash(&mut hasher);
        self.player1.energy_zone.cards.hash(&mut hasher);
        self.player1.waitroom.cards.hash(&mut hasher);
        self.player1.live_card_zone.cards.hash(&mut hasher);
        self.player1.success_live_card_zone.cards.hash(&mut hasher);
        self.player1.stage.stage.hash(&mut hasher);
        for under_cards in &self.player1.stage.under_cards {
            under_cards.hash(&mut hasher);
        }
        self.player2.hand.cards.hash(&mut hasher);
        self.player2.main_deck.cards.hash(&mut hasher);
        self.player2.energy_deck.cards.hash(&mut hasher);
        self.player2.energy_zone.cards.hash(&mut hasher);
        self.player2.waitroom.cards.hash(&mut hasher);
        self.player2.live_card_zone.cards.hash(&mut hasher);
        self.player2.success_live_card_zone.cards.hash(&mut hasher);
        self.player2.stage.stage.hash(&mut hasher);
        for under_cards in &self.player2.stage.under_cards {
            under_cards.hash(&mut hasher);
        }
        self.mods.orientation_modifiers.len().hash(&mut hasher);
        self.prohibition_effects.len().hash(&mut hasher);
        self.temporary_effects.len().hash(&mut hasher);
        self.mulligan_selected_indices.hash(&mut hasher);
        self.live_card_selected_indices.hash(&mut hasher);
        self.rps_winner.hash(&mut hasher);
        hasher.finish()
    }

    pub fn reset_loop_detection(&mut self) {
        self.game_state_history.clear();
        self.loop_detected = false;
        self.pending_loop_protocol = None;
        self.loop_last_action = None;
    }

    pub fn is_loop_detected(&self) -> bool {
        self.loop_detected
    }
}
