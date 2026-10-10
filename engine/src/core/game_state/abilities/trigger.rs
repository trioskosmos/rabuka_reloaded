use crate::core::constants::CountCast;
use crate::core::game_state::GameState;
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use crate::ability::enums::Zone;
use crate::card::Condition;
use crate::core::types::{AbilityTrigger, Duration};
use crate::player::Player;
use smallvec::SmallVec;

/// What caused an ability to be enqueued, carried as one value rather than
/// three positional parameters that only ever travel together.
struct AbilityTriggerContext {
    trigger_type: AbilityTrigger,
    trigger_moved_cards: Option<SmallVec<[i16; 4]>>,
    triggering_member_id: Option<i16>,
}

/// Where an AUTO scan found a card, and whose it is.
///
/// The moved-card branch of the AUTO scan used to answer "is it on a board?" with four
/// `Vec::contains` calls and then "who owns it?" with ten more — and four of those ten
/// re-tested zones the first four had already ruled out. A card is in exactly ONE place,
/// so the whole question is a single pass, and the answer drives both decisions at once:
/// on-stage and on-live cards are scanned by their own dedicated loops, and everything
/// else must belong to the player being scanned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AutoScanZone {
    /// On a stage — the stage loop owns it.
    Stage(&'static str),
    /// In a live card zone — the live loop owns it.
    Live(&'static str),
    /// In a hand, energy zone or 控え室 — the moved-card loop owns it.
    OffBoard(&'static str),
    /// In no zone this scan can see.
    Unknown,
}

impl AutoScanZone {
    /// True when a dedicated loop already covers this card, so the moved-card loop must
    /// skip it.
    fn is_scanned_elsewhere(self) -> bool {
        matches!(self, AutoScanZone::Stage(_) | AutoScanZone::Live(_))
    }

    /// The owning player's short id, when the card is the moved-card loop's to scan.
    fn off_board_owner(self) -> Option<&'static str> {
        match self {
            AutoScanZone::OffBoard(p) => Some(p),
            _ => None,
        }
    }
}

impl GameState {
    /// Classify `card_id` in one pass over both players' relevant zones.
    ///
    /// Order is deliberate and matches the old short-circuit behaviour: stage first, then
    /// live, then the off-board zones, because a card on stage was reported as `Stage`
    /// before ownership was ever considered. Within a zone class, p1 is tested first so a
    /// card that somehow sits in both seats' zones still resolves the way the old
    /// `is_p1`-before-`is_p2` ordering did.
    fn auto_scan_zone(&self, card_id: i16) -> AutoScanZone {
        for (player, id) in [(&self.player1, "p1"), (&self.player2, "p2")] {
            if player.stage.stage.contains(&card_id) {
                return AutoScanZone::Stage(id);
            }
        }
        for (player, id) in [(&self.player1, "p1"), (&self.player2, "p2")] {
            if player.live_card_zone.cards.contains(&card_id) {
                return AutoScanZone::Live(id);
            }
        }
        for (player, id) in [(&self.player1, "p1"), (&self.player2, "p2")] {
            if player.hand.cards.contains(&card_id)
                || player.energy_zone.cards.contains(&card_id)
                || player.waitroom.cards.contains(&card_id)
            {
                return AutoScanZone::OffBoard(id);
            }
        }
        AutoScanZone::Unknown
    }

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
        // Negation is recorded in `ability_invalidations` (see
        // `record_ability_invalidation`). This used to OR in a second store,
        // `negated_abilities`, that nothing ever populated — so the term was
        // permanently false, and its presence implied negation worked two ways
        // when it never did.
        self.ability_invalidations
            .iter()
            .any(|entry| entry.card_id == card_id && &entry.trigger == trigger)
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
        // `peek()` rather than `resolve()`: this reads a single `u8`, and
        // `resolve()` would pay an `Arc::clone` (an atomic RMW on a line shared
        // by every worker thread) for it. Both arms copy the `u8` out rather
        // than binding a reference, because the `Arc` from `resolve()` is a
        // temporary that dies at the end of the statement. On `no_std` `peek()`
        // always returns `None`, so the `resolve()` arm is the only live path.
        let limit = self
            .card_database
            .get_card(card_id)
            .and_then(|c| c.abilities.get(ability_index))
            .and_then(|ar| match ar.peek() {
                Some(a) => a.use_limit,
                None => ar.resolve().use_limit,
            });
        let Some(limit) = limit else {
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
        trigger: AbilityTriggerContext,
    ) -> crate::ability::queue::AbilityQueueEntry {
        let AbilityTriggerContext {
            trigger_type,
            trigger_moved_cards,
            triggering_member_id,
        } = trigger;
        crate::ability::queue::AbilityQueueEntry {
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
            // Enqueue-time snapshot of the appearance batch. This is the single
            // central capture point: every ability queue entry is built here, so
            // the snapshot always reflects the event that triggered THIS entry.
            // A card played to stage records an appearance but no movement, and
            // the effect-time condition re-check runs after the live appearance
            // batch is consumed — so this is what lets a self-referential
            // 「このメンバーが登場」 condition still see its own event. See
            // docs/JIDOU_COMBINATION_WORK.md §6.
            trigger_appeared_cards: self.recently_appeared_cards.clone(),
            triggering_member_id,
            snapshot_movements: SmallVec::new(),
            choice_effect_text: None,
            choice_repeats_remaining: 0,
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
                        && ability.effect.is_some()
                    {
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
                .map(crate::Arc::clone);
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
            if let crate::card::Condition::Compound {
                conditions: Some(ref children),
                ..
            } = condition
            {
                if children.iter().any(|c| Self::condition_is_event_based(c)) {
                    return true;
                }
            }
        false
    }

    /// Wrapper: scans with the `recently_moved_cards` batch (default event).
    pub fn trigger_auto_abilities_for_player(&mut self, player_id: &str) {
        // Inline for batches of <= 4 cards, so this is allocation-free in
        // practice — and it replaces a struct that carried a guaranteed
        // `Option<String>` allocation to no effect.
        let moved = self.recently_moved_cards.clone().unwrap_or_default();
        self.trigger_auto_abilities_for_player_with_event(player_id, &moved);
    }

    /// Post-movement scan wrapper: fires the TAS with the standard
    /// post-movement snapshot (recently moved cards + position-change flag).
    pub fn trigger_auto_abilities_for_movement(&mut self, player_id: &str) {
        let moved = self.recently_moved_cards.clone().unwrap_or_default();
        self.trigger_auto_abilities_for_player_with_event(player_id, &moved);
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

    /// Core TAS implementation.
    ///
    /// Takes ONLY the moved-card batch. This used to take a `&TriggerEvent`
    /// with six fields, but every read inside this function was
    /// `event.moved_cards` — `moved_from_zone`, `position_change_occurred`,
    /// `appeared_cards`, `energy_placed_by_effect` and `energy_placed_by_player`
    /// were populated by all six call sites and **read by none of them**. In
    /// particular `moved_from_zone: Option<String>` cost a guaranteed heap
    /// allocation per call (a `String` clone) to carry nothing.
    ///
    /// The batch is passed in rather than read from `self` because one call site
    /// (the post-loop batch scan) builds it from `self.batch_movements`, not
    /// from the `recently_moved_cards` shadow field. The other five pass that
    /// field, materialised into a local first — inline for batches of <= 4
    /// cards, so allocation-free in practice.
    pub fn trigger_auto_abilities_for_player_with_event(
        &mut self,
        player_id: &str,
        moved_cards: &[i16],
    ) {
        // Called 4x per card play (self + opponent, before and after the
        // placement). This is the scan, not the resolve: separating it from
        // trig::process_pending is what tells you whether a slow turn is
        // spent LOOKING for triggers or RUNNING them.
        #[cfg(not(feature = "no_std"))]
        let _timer = crate::timer::Timer::start("trig::auto_scan");
        let queued_before = self.ability_queue.len();
        let mut abilities_to_trigger: Vec<(i16, usize, i16)> = Vec::new();
        let skip_this_card_auto_key = self.just_completed_ability_key;
        let just_completed_batch_matches =
            self.just_completed_batch_matches(moved_cards);
        {
            // Copy the two id lists instead of borrowing the player. Both scans
            // need `&mut self` to evaluate conditions under a temporary
            // activating card, and a borrow of self.player1 held across the loop
            // body would forbid that.
            let scanning_player = if player_id == self.player1.id {
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
                let auto_indices = self.card_database.auto_ability_indices(card_id);
                if auto_indices.is_empty() {
                    continue;
                }
                if let Some(card) = self.card_database.get_card(card_id) {
                    for &ability_idx_u16 in auto_indices {
                        let ability_idx = usize::from(ability_idx_u16);
                        let Some(ar) = card.abilities.get(ability_idx) else {
                            log::debug!(
                                "[AUTO_INDEX] stage card={} idx={} out of range (len={})",
                                card_id,
                                ability_idx,
                                card.abilities.len()
                            );
                            continue;
                        };
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
                        // The index is built from `triggers == "自動"`,
                        // so every ability it yields is an 自動 — the
                        // `triggers == AUTO` gate this loop used to run
                        // per ability is already answered. The position
                        // and watcher checks above still run: they filter
                        // the 自動 abilities the index selected, and were
                        // a no-op discard for the non-自動 abilities the
                        // index now skips.
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
                                //     state_change) depend on game state or events
                                //     that may change between TAS and ability
                                //     resolution, so they are deferred.
                                if Self::condition_is_event_based(condition) {
                                    let saved_activating = self.activating_card;
                                    self.activating_card = Some(card_id);
                                    let ctx = crate::ability::condition::ConditionContext::with_moved_cards(self, moved_cards);
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
                            }
                            // §9.7.2.1: Compute trigger multiplicity before
                            // the effect block closes -- condition and effect
                            // are only in scope here.
                            trigger_multiplicity = self.trigger_instance_count(
                                moved_cards,
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
                                .is_some_and(|c| Self::requires_this_move(c, card_id, moved_cards))
                        }) {
                            continue;
                        }
                        // Re-scan guard: skip re-enqueueing the exact auto
                        // ability that just completed (numeric key).
                        let num_key = (card_id.u32_count() << 16) | ability_idx.u32_count();
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
                            player_id,
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
            // Also scan live cards for AUTO abilities
            for &card_id in &live_ids {
                // Same index as the stage loop: only 8 live cards in the pool carry a
                // 自動, so a resolved live card almost never has one to find.
                let auto_indices = self.card_database.auto_ability_indices(card_id);
                if auto_indices.is_empty() {
                    continue;
                }
                // A/B: see `ab_flag`. Set `RABUKA_AB_AUTO_INDEX=off` to restore the
                // pre-index behaviour (resolve every ability of every card, compare its
                // trigger string) and measure the index on identical games.
                if let Some(card) = self.card_database.get_card(card_id) {
                    for &ability_idx in auto_indices {
                        let ability_idx = ability_idx as usize;
                        // Bounds-checked, same reason as the stage loop above.
                        let Some(ar) = card.abilities.get(ability_idx) else {
                            log::debug!(
                                "[AUTO_INDEX] live card={} idx={} out of range (len={})",
                                card_id,
                                ability_idx,
                                card.abilities.len()
                            );
                            continue;
                        };
                        let ability = ar.resolve();
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
                                        let ctx = crate::ability::condition::ConditionContext::with_moved_cards(self, moved_cards);
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
                                    Self::requires_this_move(c, card_id, moved_cards)
                                })
                            }) {
                                continue;
                            }
                            let num_key = (card_id.u32_count() << 16) | ability_idx.u32_count();
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
            for &moved_card_id in moved_cards {
                // ONE pass answers both questions this used to ask with fourteen linear
                // scans: is the card already covered by the stage or live loop, and whose
                // is it if not. Four of the old ownership checks re-tested zones the skip
                // test had already ruled out.
                let zone = self.auto_scan_zone(moved_card_id);
                if zone.is_scanned_elsewhere() {
                    continue;
                }
                // Only enqueue for the card's actual owner (not the scanner).
                match zone.off_board_owner() {
                    // can't determine owner, skip
                    None => continue,
                    Some(owner) if owner != player_id => continue,
                    Some(_) => {}
                }
                if let Some(card) = self.card_database.get_card(moved_card_id) {
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
                                    let ctx = crate::ability::condition::ConditionContext::with_moved_cards(self, moved_cards);
                                    let passes = ctx.evaluate_condition(condition);
                                    self.activating_card = saved_activating;
                                    if !passes {
                                        continue;
                                    }
                                }
                            }
                            let num_key =
                (moved_card_id.u32_count() << 16) | ability_idx.u32_count();
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
        // `from_slice`, not `to_vec` + wrap: this keeps the batch inline for
        // <= 4 cards, which is the overwhelmingly common case.
        let moved = Some(smallvec::SmallVec::<[i16; 4]>::from_slice(moved_cards));
        let mut cached_card_id = None;
        let mut cached_card_no = String::new();
        for (card_id, ability_idx, _stage_card_id) in abilities_to_trigger {
            let num_key = (card_id.u32_count() << 16) | ability_idx.u32_count();
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
                player_id,
                Some(cached_card_no.as_str()),
                Some(card_id),
                ability_idx,
                moved.as_ref(),
                None,
            );
        }
        // Consume the energy flag after every TAS scan -- each event should
        // trigger at most one batch of each_time abilities.  The snapshot
        // captured in trigger_auto_ability (above) preserves the flag value
        // for abilities that need it during execution (e.g. Sumire's "moves").
        let queued_new = self.ability_queue.len() > queued_before;
        let deck_emptied_for_player =
            self.deck_emptied_by_effect.as_deref() == Some(player_id);
        if deck_emptied_for_player {
            if queued_new {
                if player_id == self.player1.id {
                    self.player1.refresh();
                } else {
                    self.player2.refresh();
                }
                log::debug!(
                    "[AUTO_REFRESH] queued_new={} player={} deck_emptied_by_effect=true",
                    self.ability_queue.len() - queued_before,
                    player_id
                );
            }
            self.deck_emptied_by_effect = None;
        }
    }

    /// §9.7.2.1: Count how many standby entries to create for a trigger event.
    ///
    /// For `state_change_condition`, counts the transitions in the current
    /// batch that match its from/to — 「メンバーがウェイト状態になるたび、ブレードを
    /// 得る」 (PL!-PR-023-PR) awards ONE blade per member that turned wait,
    /// including several turned wait by a single effect, so a single standby
    /// entry would pay only once for the whole batch. Scope/group filters are
    /// re-checked at resolution (can_activate_effect), so counting on
    /// from/to + "still on a stage" can cost an extra no-op entry, never a
    /// missed trigger.
    ///
    /// For `card_count_condition` with `source: "preceding_moved"`, counts
    /// cards in the event batch matching the condition's filters.  Returns 1
    /// for batch patterns ("すべて", "1枚以丁E, self_target, count=1+op=>=).
    /// All other condition types return 1 (single standby instance).
    fn trigger_instance_count(
        &self,
        moved_cards: &[i16],
        effect: &crate::card::AbilityEffect,
        card_db: &crate::card::CardDatabase,
    ) -> u8 {
        let condition = match &effect.condition {
            Some(c) => c,
            None => return 1,
        };
        if matches!(condition.as_ref(), crate::card::Condition::State { .. })
            && (condition.get_from_state().is_some() || condition.get_to_state().is_some())
            // Event triggers only: a temporal look-back (「このターン中に…アクティブに
            // していたメンバー1人につき」) counts its own members, not this batch.
            && condition.get_temporal().is_none()
        {
            let from = condition.get_from_state();
            let to = condition.get_to_state();
            let matched = self
                .recently_state_changed
                .iter()
                .filter(|(cid, cfrom, cto, _)| {
                    *cid >= 0
                        && from.is_some_and(|f| f == cfrom.as_str())
                        && to.is_some_and(|t| t == cto.as_str())
                        && (self.player1.stage.stage.contains(cid)
                            || self.player2.stage.stage.contains(cid))
                })
                .count()
                .u8_count();
            log::debug!(
                "[TRIGGER_MULT] state_change {}→{} matched={} in batch of {}",
                from.unwrap_or("?"),
                to.unwrap_or("?"),
                matched,
                self.recently_state_changed.len()
            );
            return matched.max(1);
        }
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
                    Some(&smallvec::smallvec![moved_card_id]),
                    None,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
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
                // `resolved_ability` is already the `Arc<Ability>`; calling
                // `ability.to_arc()` here re-ran the decode-slot lookup and a
                // second `Arc::clone` (an atomic increment) for a value already
                // in hand. One refcount bump per enqueue, removed.
                return Some((ability_index, resolved_ability));
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
            AbilityTriggerContext {
                trigger_type: trigger_type.clone(),
                trigger_moved_cards: trigger_moved_cards.clone(),
                triggering_member_id,
            },
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
                .then(|| crate::Arc::clone(ability))
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
            AbilityTriggerContext {
                trigger_type: trigger_type.clone(),
                trigger_moved_cards: trigger_moved_cards.clone(),
                triggering_member_id,
            },
        );
        let mut entry = entry;
        entry.snapshot_movements = self.batch_movements.clone();
        self.ability_queue.enqueue(entry);
    }

    /// Hot-path version of trigger_auto_ability that takes a numeric ability index
    /// instead of a string key. Avoids format!() allocations in the TAS scan loop.
    ///
    /// `source_card_id` is borrowed (`&str`) rather than owned: every caller
    /// already holds the card number as an `ArcStr`/`String` it is about to
    /// drop, and `build_ability_queue_entry` re-materialises an owned `String`
    /// for the entry anyway — so an owned `Option<String>` here would be a
    /// throwaway allocation that is only ever read as `&str`.
    ///
    /// `trigger_moved_cards` is likewise borrowed (`&SmallVec`): the scan
    /// loop clones the movement snapshot once, then re-cloned it per
    /// triggered ability only for `_refs` to clone it a second time into
    /// the queue entry. Passing a reference removes that per-ability clone
    /// (the entry's owned copy is the one that is actually stored).
    #[allow(clippy::too_many_arguments)]
pub fn trigger_auto_ability_by_index(
        &mut self,
        trigger_type: AbilityTrigger,
        player_id: impl Into<crate::core::player::PlayerId>,
        source_card_id: Option<&str>,
        explicit_card_id: Option<i16>,
        ability_index: usize,
        trigger_moved_cards: Option<&SmallVec<[i16; 4]>>,
        triggering_member_id: Option<i16>,
    ) {
        self.trigger_auto_ability_by_index_refs(
            trigger_type,
            &player_id.into(),
            source_card_id,
            explicit_card_id,
            ability_index,
            trigger_moved_cards,
            triggering_member_id,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn trigger_auto_ability_by_index_refs(
        &mut self,
        trigger_type: AbilityTrigger,
        player_id: &str,
        source_card_id: Option<&str>,
        explicit_card_id: Option<i16>,
        ability_index: usize,
        trigger_moved_cards: Option<&SmallVec<[i16; 4]>>,
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
                        AbilityTriggerContext {
                            trigger_type: trigger_type.clone(),
                            trigger_moved_cards: trigger_moved_cards.map(|mc| mc.clone()),
                            triggering_member_id,
                        },
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
                player_id,
                Some(card_no.as_str()),
                Some(cid),
                ability_idx,
                None,
                Some(member_card_id),
            );
        }
    }
}
