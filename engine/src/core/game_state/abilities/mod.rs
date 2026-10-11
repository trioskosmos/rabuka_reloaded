pub mod trigger;
pub mod queue;
pub mod choice;
pub mod effects;
pub mod loop_detect;

use crate::core::constants::CountCast;
use crate::core::game_state::GameState;
use crate::core::types::Phase;
use crate::player::Player;
use smallvec::SmallVec;
use crate::ability::enums::Zone;

fn _prohibition_destination_blocks(prohibition: &str, zone: &str) -> bool {
    let parts: Vec<&str> = prohibition.split(':').collect();
    if parts.len() < 3 {
        return false;
    }
    let dest = parts[2];
    if dest.is_empty() {
        return zone == Zone::SuccessLiveZone.to_str();
    }
    let dest_zone = Zone::from_str(dest);
    let target_zone = Zone::from_str(zone);
    dest_zone == target_zone
        || (dest_zone == Some(Zone::LiveCardZone) && target_zone == Some(Zone::SuccessLiveZone))
        || (dest_zone == Some(Zone::SuccessLiveZone) && target_zone == Some(Zone::LiveCardZone))
}

impl GameState {

    /// Resolve the active queue entry while keeping its resolver and completion
    /// state in one borrow of `GameState`. A helper split is not safe without
    /// transferring part of that state out: `resolve_ability` mutably borrows
    /// `GameState`, while the same flow must read and update queue entry flags,
    /// store the resolver for choice resume, and preserve the exact post-resolve
    /// scan and debug-log ordering. Keep this ownership boundary intact.
    pub(crate) fn process_current_ability(&mut self) {
        #[cfg(not(feature = "no_std"))]
        let _timer = crate::timer::Timer::start("resolve::current_ability");
        // Scoped allocation trace. Free unless `RABUKA_TRACE_AT` names this
        // site, and compiled out entirely without the feature. This one is
        // reached from inside `execute_and_settle`, so the bench cannot arm it
        // from outside — and it is the largest self-time site in the engine.
        #[cfg(feature = "alloc_tracker")]
        let _alloc_trace = crate::alloc_counter::maybe_arm("process_current_ability");
        // Safety timeout: a runaway ability re-trigger loop (e.g. an each_time
        // watcher re-queued by its own effect's movement) must never spin forever.
        // Abort resolution past an absurd number of calls instead of hanging or
        // overflowing a counter.
        //
        // SCOPED TO ONE RESOLUTION CHAIN, and that scoping is load-bearing. This was a
        // `static AtomicU32` that nothing ever reset, so it was a process-lifetime budget:
        // after 200k abilities resolved CUMULATIVELY, every later call took the abort
        // branch and called `ability_queue.clear()`, silently discarding all pending
        // abilities. Two consequences, both found by measurement rather than reading:
        //
        //   * `web_server.rs` serves many games from one process, so ability resolution
        //     would die permanently partway through the server's life.
        //   * `sim_bench --games 80` resolves ~154k abilities per sweep, so `--repeat 2`
        //     and beyond were benchmarking an engine whose ability resolution was OFF for
        //     most of the run. Every A/B touching resolution was measuring that.
        //
        // A guard meant to break one runaway loop wants a per-chain budget, so it is
        // reset whenever a fresh settle begins.
        if self.pca_calls_in_chain > 200_000 {
            log::error!(
                "[PCA_TIMEOUT] exceeded 200k process_current_ability calls in one resolution \
                 chain; aborting to break runaway loop"
            );
            self.ability_queue.clear();
            return;
        }
        self.pca_calls_in_chain += 1;
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
            if crate::core::runtime_flags::logging_enabled()
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
            r.selection.cards.clear();
            // G1/G3: preserve spawn_context.target across resolver re-use.
            // When resume_pending_commands sets spawn_context.target via the
            // G3 fix and then process_current_ability is called again, the
            // target must survive the reset so the G1 check can route the
            // pending choice to the opponent player.
            let saved_target = r.in_flight.spawn_context.target.clone();
            r.in_flight.spawn_context = crate::ability::types::EffectSpawnContext::default();
            r.in_flight.spawn_context.target = saved_target;
            r.awaiting.stage_cards = SmallVec::new();
            r.in_flight.execution_context = crate::ability::types::ExecutionContext::None;
            // Clear pending_choice: the previous call stored it in the queue,
            // and it must not block re-execution of the effect on the next pass.
            r.awaiting.choice = None;
            r
        } else {
            crate::Box::new(crate::ability::resolver::AbilityResolver::new(
                self.card_database.clone(),
                card_id,
            ))
        };

        resolver.session.debug_trace =
            crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed);

        #[cfg(not(feature = "no_std"))]
        let _t = crate::timer::Timer::start("resolve::resolve_ability_total");
        let resolve_result = resolver.resolve_ability(self, ability.clone(), card_id, ability_index);
        #[cfg(not(feature = "no_std"))]
        drop(_t);
        match resolve_result {
            Ok(()) => {
                self.push_debug_note_fmt(format_args!(
                    "resolve ok card={:?} idx={} pending_choice={}",
                    card_id,
                    ability_index,
                    resolver.awaiting.choice.is_some()
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
                if resolver.awaiting.choice.is_some() { "waiting_for_choice" } else { "finished" }
            );
        }

        // Sync resolver state to GameState before the resolver may be dropped.
        // The condition system and other subsystems read GameState directly.
        if resolver.session.debug_trace {
            self.last_ability_trace = Some(resolver.in_flight.pipeline.trace.clone());
        }

        if let Some(ref c) = resolver.awaiting.choice {
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
                    e.mark_cost_paid();
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
                    && resolver.in_flight.spawn_context.target.as_deref() == Some("opponent") =>
                {
                    true
                }
                crate::ability::types::Choice::SelectPosition { .. }
                    if matches!(
                        resolver.in_flight.execution_context,
                        crate::ability::types::ExecutionContext::MoveCardsPosition { ref target, .. }
                        if target == "opponent"
                    ) =>
                {
                    true
                }
                crate::ability::types::Choice::SelectTarget { target, .. }
                    if target == "position|destination"
                        && (resolver.in_flight.spawn_context.target.as_deref() == Some("opponent")
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
                    entry.choice_player_id = Some(crate::core::player::PlayerId::from(choice_player));
                }
                log::debug!(
                    "[CHOICE_ROUTING] owner={} picker={:?} explicit_picker={:?} preserved={} targets_opponent={} spawn={:?} action={:?}",
                    entry.player_id,
                    entry.choice_player_id,
                    picker,
                    preserved_picker,
                    targets_opponent,
                    resolver.in_flight.spawn_context.target,
                    resolver.owner.executing.as_ref().map(|e| e.action)
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
                let moved = self.recently_moved_cards.clone().unwrap_or_default();
                self.set_just_completed(just_completed_key);
                self.trigger_auto_abilities_for_player_with_event(&current_pid, &moved);
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

    /// Pending ability-queue depth. Exposed so the bot's `project_live_start`
    /// can tell "the live-start scan queued nothing" apart from "it queued
    /// abilities that resolved to nothing" — the two are indistinguishable from
    /// the projected board alone, and both look like a bit-identical null.
    pub fn ability_queue_len(&self) -> usize {
        self.ability_queue.len()
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

    pub fn entry_conditional_choice(&self) -> Option<crate::ability::queue::ConditionalChoice> {
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

    /// Read `trigger_appeared_cards` from the current queue entry (the enqueue-time
    /// appearance snapshot). The appearance mirror of `entry_trigger_moved_cards`:
    /// a self-referential 登場/移動 condition re-evaluated at effect time finds the
    /// live appearance batch already consumed, so it falls back to this snapshot to
    /// decide which card appeared to trigger the ability.
    pub fn entry_trigger_appeared_cards(&self) -> Option<SmallVec<[i16; 4]>> {
        self.ability_queue
            .current_entry()
            .map(|e| e.trigger_appeared_cards.clone())
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

    /// `ArcStr` rather than `String`: this is the movement event's field handed straight
    /// back, so returning it as an owned `String` re-allocated what the event already
    /// holds refcounted.
    pub fn entry_snapshot_last_energy_placed_by_player(
        &self,
    ) -> Option<crate::core::types::ArcStr> {
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

    pub fn entry_snapshot_last_area_move_by_player(&self) -> Option<crate::core::types::ArcStr> {
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

    /// Resolve which player "self" refers to based on the ability master's player_id.
    /// The ability queue entry stores which player activated this ability.
    ///
    /// Borrows rather than returning `Option<String>`. Every caller used the
    /// value only to compare it against `"player2"`/`"p2"`, so the old owned
    /// return was one heap allocation per call across ~20 call sites — and
    /// `resolve_target_player` below re-ran it on all 199 of its call sites.
    #[inline]
    pub fn ability_master_id(&self) -> Option<&str> {
        self.ability_queue
            .current_entry()
            .map(|e| e.player_id.as_str())
    }

    /// Shared master-player resolution — used by both mut/non-mut variants.
    /// Checks the queue entry (ability_master_id) first, then falls back to the
    /// owning player of the activating card. This makes the two arms identical
    /// (previously mut lacked the owner fallback and diverged on "both").
    fn resolve_master_id(&self) -> Option<&str> {
        self.ability_master_id().or_else(|| {
            self.activating_card
                .and_then(|cid| self.owner_of_card(cid))
                .map(|p| p.id.as_str())
        })
    }

    pub fn resolve_target_player_mut(&mut self, target: &str) -> &mut Player {
        let master = self.resolve_master_id();
        match (target, master) {
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
        match (target, master) {
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
    /// Membership is resolved through [`crate::ability::util::card_matches_group_str_card`]
    /// so multi-name joint cards (Q228: LL-bp1-001-R＋ carries 虹ヶ咲 +
    /// Liella! + 蓮ノ空 through its three names) contribute every group they
    /// belong to, not just a single `card.group` field.
    ///
    /// One pass over the three slots — one card resolution each — instead
    /// of a full stage scan per group: the same (member, group) tests,
    /// but 3 hash probes instead of 15, and a group that has already
    /// matched is never tested again. This predicate runs on the
    /// action-generation hot path (~52 calls per game, each feeding the
    /// effective-cost gate of every offered activation).
    pub fn distinct_stage_groups(&self, player_id: &str) -> u8 {
        let player = self
            .try_player_by_id(player_id)
            .unwrap_or(&self.player1);
        // Bitset over the canonical groups: bit i is set once any
        // stage member matches group i.
        let mut matched = 0u8;
        for &cid in player.stage.stage.iter() {
            if cid == -1 {
                continue;
            }
            // Fast path: the load-time canonical-group bitset (one
            // byte per card id, see `ability::util::card_group_bits`).
            // `distinct_stage_groups` runs once per main-phase
            // generation and once per activatable ability, so the
            // per-(member, group) `card_matches_group_str_card`
            // match — string normalization plus a `SetCardIdentity`
            // ability walk — was re-derived on every call. Group
            // membership is a pure function of the card's printed
            // data, computed once at load.
            if let Some(&bits) = self
                .card_database
                .card_group_bits
                .get(cid.max(0) as usize)
            {
                matched |= bits;
                continue;
            }
            // Fallback: a database whose bitset was not built at
            // load (a hand-constructed fixture or a snapshot
            // restore) still answers correctly via the original
            // on-the-fly match.
            let Some(card) = self.card_database.get_card(cid) else {
                continue;
            };
            for (i, group) in crate::ability::util::CANONICAL_GROUPS
                .iter()
                .enumerate()
            {
                let bit = 1u8 << i;
                if matched & bit != 0 {
                    continue;
                }
                if crate::ability::util::card_matches_group_str_card(
                    &self.card_database,
                    card,
                    cid,
                    group,
                ) {
                    matched |= bit;
                }
            }
        }
        matched.count_ones() as u8
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
        self.clear_state_change_window("clear_movement_tracking");
    }

    /// Movement/appearance cleanup that PRESERVES the state-change record.
    ///
    /// Used when the queue paused for a choice (the 「複数の自動能力が同時に発動
    /// しました」 ordering prompt) right after a scan: the entries enqueued from
    /// `recently_state_changed` still have to re-evaluate it at resolution, and
    /// clearing it here dropped every one of them. Movement batches do not need
    /// this — they ride on their queue entry (`trigger_moved_cards`).
    pub fn clear_movement_tracking_keep_state_changes(&mut self) {
        self.clear_recently_moved_batch();
        self.recently_appeared_cards.clear();
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
}
