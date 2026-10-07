use crate::Arc;

use smallvec::SmallVec;

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

use crate::ability::resolver::AbilityResolver;
use crate::ability::types::Choice;
use crate::card::{Ability, AbilityEffect};
use crate::game_state::AbilityTrigger;
#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};

/// Discriminator data for choice routing. Replaces the old `conditional_choice: Option<String>`
/// which serialized three different types to JSON strings at runtime.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum ConditionalChoice {
    /// Plain string value (color name, sentinel like "pay_optional_cost")
    Str(String),
    /// Vec of string options (or_card_types, numeric options, heart colors)
    Strings(Vec<String>),
    /// Vec of ability effect options (choice routes)
    Effects(Vec<Box<AbilityEffect>>),
    /// A single ability effect (conditional_on_optional sub-effect)
    Effect(AbilityEffect),
}

/// Unique identifier for an ability instance in the queue.
///
/// Built as a formatted string for callers that need to name an ability, but
/// `AbilityQueueEntry` does not store one: the identity is already carried by
/// `card_no` + `ability_index` + `trigger_type`, and eagerly formatting it
/// cost two heap allocations on every single enqueue.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct AbilityId(pub String);

impl AbilityId {
    pub fn new(card_no: &str, ability_index: usize, trigger_type: &str) -> Self {
        AbilityId(format!("{}_{}_{}", card_no, ability_index, trigger_type))
    }

    /// The canonical string form, rebuilt from an entry's own fields.
    pub fn from_parts(
        card_no: &str,
        ability_index: usize,
        trigger_type: &crate::core::types::AbilityTrigger,
    ) -> Self {
        AbilityId(format!("{}_{}_{:?}", card_no, ability_index, trigger_type))
    }
}

/// Current state of ability queue processing
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum QueueState {
    /// Queue is idle, ready to process next ability
    Idle,
    /// Waiting for player to choose which auto ability resolves first (Rule 9.5.3)
    WaitingForAutoAbilityChoice { choice: Choice },
    /// Currently paying cost for an ability
    PayingCost { entry_index: u8 },
    /// Waiting for user choice (cost payment, target selection, etc.)
    WaitingForChoice { entry_index: u8, choice: Option<Choice> },
    /// Executing the effect of an ability
    ExecutingEffect { entry_index: u8 },
    /// Ability completed, will transition to Idle
    Completed { entry_index: u8 },
}

/// Entry in the ability queue
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct AbilityQueueEntry {
    pub card_no: String,
    pub player_id: String,
    pub ability: Arc<Ability>,
    pub ability_index: usize,
    pub card_id: Option<i16>,
    pub trigger_type: AbilityTrigger,
    /// Whether this ability has been completed
    pub completed: bool,
    /// Whether the cost has been fully paid (for re-entry after cost choice)
    pub cost_paid: bool,
    /// Stored choice result for resumption
    pub cost_paid_index: u8,
    /// Discriminator for routing choice results to the correct handler.
    pub choice_card_no: Option<crate::ability::types::ChoiceRoute>,
    /// Discriminator data for choice routing
    pub conditional_choice: Option<ConditionalChoice>,
    /// Whether the effect has started executing (prevents re-processing)
    pub effect_started: bool,
    /// Whether this activation's use-limit consumption has already been recorded.
    /// Choice abilities resolve across multiple phases; this prevents the counter
    /// from incrementing more than once per activation.
    pub use_limit_recorded: bool,
    /// Result of the optional cost evaluation.
    /// None = not evaluated, Some(true) = paid, Some(false) = skipped.
    pub optional_cost_result: Option<bool>,
    /// For an accepted `conditional_on_optional` placement: whether every required
    /// optional move actually moved a card. `Some(true)` is armed when the player
    /// accepts; any optional move that resolves nothing (and offers no choice —
    /// e.g. a group missing from the discard pile) clears it to `Some(false)`.
    /// The trailing "そうしたとき" consequence (Q118: all-or-nothing) is gated on
    /// this, so the draw only fires if all required cards were actually placed.
    /// `None` = not inside a conditional_on_optional consequence (no gating).
    pub optional_moves_all_moved: Option<bool>,
    /// Player who must make the pending choice (if different from activating player)
    pub choice_player_id: Option<String>,
    /// Card IDs that triggered this each_time auto ability (snapshot of
    /// recently_moved_cards at enqueue time). Used by source:"those_cards"
    /// to resolve to only the trigger cards, not the full discard pile.
    pub trigger_moved_cards: Option<SmallVec<[i16; 4]>>,
    /// Snapshot of `recently_appeared_cards` at enqueue time. The mirror of
    /// `trigger_moved_cards` for APPEARANCE events: a card played to stage
    /// records an appearance but NO movement, so `trigger_moved_cards` is empty
    /// for it. An effect-time condition re-check runs after the live appearance
    /// batch has been consumed, so a self-referential 「このメンバーが登場」
    /// condition needs this snapshot to still see its own event. Mirrors the
    /// `entry_trigger_moved_cards` fallback pattern.
    pub trigger_appeared_cards: SmallVec<[i16; 4]>,
    /// Snapshot of batch_movements at enqueue time so the "moves" and energy
    /// conditions can see what triggered the ability even after
    /// clear_effect_tracking() clears the global batch_movements.
    pub snapshot_movements: SmallVec<[crate::types::MovementEvent; 4]>,
    /// Actions queued for sequential execution after a choice round-trip.
    pub pending_actions: Vec<AbilityEffect>,
    /// Persistent ability resolver — stays alive across choice round-trips
    /// instead of being destroyed and recreated. Eliminates manual save/restore.
    /// Boxed: the resolver carries ~1.9 KB of execution state that is only ever
    /// used by the CURRENT entry, so idle entries must not pay that inline.
    #[cfg_attr(feature = "serde_support", serde(skip))]
    pub resolver: Option<Box<AbilityResolver>>,
    /// For each_time triggers: the stage member card ID whose resolution
    /// caused this each_time ability to fire. Used by effects like
    /// "gain all-heart" to target the correct member.
    pub triggering_member_id: Option<i16>,
    /// The original effect text of a choice-type ability, stored so the frontend
    /// can use it as the choice prompt (separate from the option labels).
    pub choice_effect_text: Option<String>,
    /// How many MORE times a per-card choice is still to be offered.
    ///
    /// 「『X』のカード1枚につき、以下から1つを選ぶ」 offers the choice once per qualifying
    /// card, and 「同じ選択肢を2回以上選んでもよい」 confirms repeats are intended. The
    /// multiplier is resolved once when the choice is first created (the qualifying cards
    /// can change as earlier choices resolve), stored here as the number of *remaining*
    /// offerings, and decremented each time an option is answered. Zero means the last
    /// choice has been taken and no re-prompt follows.
    pub choice_repeats_remaining: u8,
    /// Cache for condition results that should be evaluated once and reused.
    /// Keyed by condition text. Populated by the first evaluation; subsequent
    /// evaluations return the cached value. The parser sets `"cache": true` on
    /// conditions that depend on mutable state (e.g. revealed_cards).
    pub condition_cache: SmallVec<[(String, bool); 2]>,
}

impl AbilityQueueEntry {
    /// The single write path for `cost_paid`.
    ///
    /// "Settled" covers both a PAID cost and a DECLINED optional one — the
    /// cost phase is over either way. Declining does NOT count as a payment
    /// for trackers like 「コストを支払うため」; payment paths pair this with
    /// `GameState::record_cost_paid`, decline paths do not.
    ///
    /// Never call on an entry whose `SequentialCost` is half-paid
    /// (`AbilityQueue::has_unpaid_sequential_sub_cost`): leaving `cost_paid`
    /// false is what lets the resume path re-enter `pay_ability_cost` at
    /// `cost_paid_index`.
    pub fn mark_cost_paid(&mut self) {
        self.cost_paid = true;
    }
}

/// Unified ability queue with proper state management
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct AbilityQueue {
    entries: Vec<AbilityQueueEntry>,
    pub state: QueueState,
    current_index: u8,
    /// Set by the resolver when a new pending choice is about to be stored.
    /// The web handler checks this after execute_main_phase_action to decide
    /// whether to push an additional history snapshot at the choice boundary.
    pub snapshot_requested: bool,
}

impl AbilityQueue {
    pub fn new() -> Self {
        AbilityQueue {
            entries: Vec::new(),
            state: QueueState::Idle,
            current_index: 0,
            snapshot_requested: false,
        }
    }

    /// Hard-abort: drop all pending entries and return the queue to Idle.
    /// Used as a safety valve when a runaway loop is detected.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.state = QueueState::Idle;
        self.current_index = 0;
    }

    /// Check if queue is idle (no ability being processed)
    pub fn is_idle(&self) -> bool {
        matches!(self.state, QueueState::Idle)
    }

    /// Check if queue is waiting for user choice
    pub fn is_waiting_for_choice(&self) -> Option<&Choice> {
        match &self.state {
            QueueState::WaitingForAutoAbilityChoice { choice } => Some(choice),
            QueueState::WaitingForChoice { choice, .. } => choice.as_ref(),
            _ => None,
        }
    }

    pub fn take_waiting_choice(&mut self) -> Option<Choice> {
        match &mut self.state {
            QueueState::WaitingForChoice { choice, .. } => choice.take(),
            QueueState::WaitingForAutoAbilityChoice { choice } => Some(choice.clone()),
            _ => None,
        }
    }

    pub fn restore_waiting_choice(&mut self, pending: Choice) {
        match &mut self.state {
            QueueState::WaitingForChoice { choice, .. } => {
                if choice.is_none() {
                    *choice = Some(pending);
                }
            }
            QueueState::ExecutingEffect { entry_index } => {
                self.state = QueueState::WaitingForChoice {
                    entry_index: *entry_index,
                    choice: Some(pending),
                };
            }
            _ => {}
        }
    }

    /// Get current ability being processed
    pub fn current_entry(&self) -> Option<&AbilityQueueEntry> {
        match &self.state {
            QueueState::PayingCost { entry_index }
            | QueueState::WaitingForChoice { entry_index, .. }
            | QueueState::ExecutingEffect { entry_index }
            | QueueState::Completed { entry_index } => self.entries.get(*entry_index as usize),
            QueueState::Idle | QueueState::WaitingForAutoAbilityChoice { .. } => None,
        }
    }

    pub fn current_entry_mut(&mut self) -> Option<&mut AbilityQueueEntry> {
        let idx = match &self.state {
            QueueState::PayingCost { entry_index }
            | QueueState::WaitingForChoice { entry_index, .. }
            | QueueState::ExecutingEffect { entry_index }
            | QueueState::Completed { entry_index } => *entry_index as usize,
            QueueState::Idle | QueueState::WaitingForAutoAbilityChoice { .. } => return None,
        };
        self.entries.get_mut(idx)
    }

    pub fn get_entry(&self, index: usize) -> Option<&AbilityQueueEntry> {
        self.entries.get(index)
    }

    /// Mark the CURRENT entry's cost settled (no-op when no entry is
    /// current). Single queue-level write path — see
    /// `AbilityQueueEntry::mark_cost_paid` for the rules.
    pub fn mark_cost_paid(&mut self) {
        if let Some(entry) = self.current_entry_mut() {
            entry.mark_cost_paid();
        }
    }

    /// True when the current entry's cost phase has settled (paid or
    /// declined). No current entry reads as NOT paid.
    pub fn current_cost_paid(&self) -> bool {
        self.current_entry().is_some_and(|e| e.cost_paid)
    }

    /// True while the current entry's cost is a `SequentialCost` that yielded
    /// mid-payment: at least one sub-cost has been paid
    /// (`cost_paid_index > 0`, recorded by `pay_cost_inner` before it returns
    /// on a pending choice) but not all of them.
    ///
    /// Rule 9.4.2 runs sub-costs in order, so such an entry's cost is NOT paid
    /// yet — the remaining sub-costs (e.g. the hand discard of 「メンバーを
    /// ウェイトにし、手札を1枚控え室に置く」) still have to run. Callers that
    /// would otherwise mark `cost_paid` on a cost choice must leave it false so
    /// the resume path re-enters `pay_ability_cost` at `cost_paid_index`.
    pub fn has_unpaid_sequential_sub_cost(&self) -> bool {
        let Some(entry) = self.current_entry() else {
            return false;
        };
        if entry.cost_paid_index == 0 {
            return false;
        }
        let Some(cost) = entry.ability.cost.as_ref() else {
            return false;
        };
        if !matches!(
            cost.action,
            crate::ability::enums::ActionType::SequentialCost
        ) {
            return false;
        }
        cost.compound
            .actions
            .as_ref()
            .is_some_and(|a| a.len() > usize::from(entry.cost_paid_index))
    }

    /// Number of entries pending or completed
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over entries.
    pub fn iter(&self) -> impl Iterator<Item = &AbilityQueueEntry> {
        self.entries.iter()
    }

    /// Check if an entry with this ID already exists (completed or not).
    ///
    /// The entry no longer stores a prebuilt `AbilityId`; the identity is
    /// rebuilt from the fields it does store. This is only reached from tests,
    /// so the temporary formatting cost is irrelevant on the hot enqueue path.
    pub fn has_entry_with_id(&self, id: &AbilityId) -> bool {
        self.entries
            .iter()
            .any(|e| *id == AbilityId::from_parts(&e.card_no, e.ability_index, &e.trigger_type))
    }

    /// Add ability to queue
    pub fn enqueue(&mut self, entry: AbilityQueueEntry) {
        self.entries.push(entry);
    }

    /// Start processing next ability in queue
    /// Returns true if an ability was started, false if queue is empty
    pub fn start_next(&mut self) -> bool {
        // Don't start new ability if currently processing one
        if !matches!(self.state, QueueState::Idle) {
            return false;
        }

        // Find next uncompleted entry
        while (self.current_index as usize) < self.entries.len() {
            let entry = &self.entries[self.current_index as usize];
            if !entry.completed {
                self.state = QueueState::PayingCost {
                    entry_index: self.current_index,
                };
                return true;
            }
            // Saturating: `current_index` is a `u8`, and a random-policy playout over
            // many decks reached 255 entries. Wrapping to 0 here would restart the scan
            // forever; saturating stops it.
            self.current_index = self.current_index.saturating_add(1);
        }
        false
    }

    /// Pause for user choice during ability execution
    pub fn pause_for_choice(&mut self, choice: Choice) {
        match &mut self.state {
            QueueState::PayingCost { entry_index }
            | QueueState::ExecutingEffect { entry_index } => {
                let idx = *entry_index as usize;
                // G1/G3: route choice to opponent when spawn context says opponent
                let opponent_id: Option<String> = self.entries.get(idx).and_then(|entry| {
                    if entry.choice_player_id.is_some() {
                        return None;
                    }
                    let is_opponent_choice = match &choice {
                        crate::ability::types::Choice::SelectCard {
                            target_player_id: Some(tpid),
                            ..
                        } => tpid == "opponent",
                        crate::ability::types::Choice::SelectTarget { target, .. } => {
                            target == "position|destination"
                        }
                        _ => false,
                    };
                    if !is_opponent_choice {
                        return None;
                    }
                    let is_spawn_opponent = entry
                        .resolver
                        .as_ref()
                        .and_then(|r| r.in_flight.spawn_context.target.as_deref())
                        == Some("opponent");
                    let is_route_opponent = entry.choice_card_no.as_ref().is_some_and(|route| {
                        matches!(route, crate::ability::types::ChoiceRoute::Raw(raw) if raw.starts_with("position_change:opponent:"))
                    });
                    if !is_spawn_opponent && !is_route_opponent {
                        return None;
                    }
                    let opp = if entry.player_id == "p1" { "p2" } else { "p1" };
                    Some(opp.to_string())
                });
                if let Some(pid) = opponent_id {
                    if let Some(entry) = self.entries.get_mut(idx) {
                        entry.choice_player_id = Some(pid);
                    }
                }
                // Universal default: every paused choice gets a choice_player_id
                if let Some(entry) = self.entries.get(idx) {
                    if entry.choice_player_id.is_none() {
                        if let Some(entry) = self.entries.get_mut(idx) {
                            entry.choice_player_id = Some(entry.player_id.clone());
                        }
                    }
                }
                self.state = QueueState::WaitingForChoice {
entry_index: u8::try_from(idx).unwrap(),
                    choice: Some(choice),
                };
            }
            QueueState::Idle | QueueState::Completed { .. } => {
                // Store the choice directly without an entry
                let dummy_entry = AbilityQueueEntry {
                    card_no: String::new(),
                    player_id: String::new(),
                    ability: Arc::new(Ability {
                        full_text: String::new(),
                        triggerless_text: None,
                        triggers: None,
                        use_limit: None,
                        is_null: false,
                        cost: None,
                        effect: None,
                        keywords: None,
                        #[cfg(not(feature = "no_std"))]
                        trigger_mask: Default::default(),
                        #[cfg(not(feature = "no_std"))]
                        trigger_position_mask: Default::default(),
                    }),
                    ability_index: 0,
                    card_id: None,
                    trigger_type: AbilityTrigger::Auto,
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
                    trigger_moved_cards: None,
                    trigger_appeared_cards: SmallVec::new(),
                    triggering_member_id: None,
                    snapshot_movements: SmallVec::new(),
            choice_effect_text: None,
            choice_repeats_remaining: 0,
            condition_cache: SmallVec::new(),
                };
                self.entries.push(dummy_entry);
                self.state = QueueState::WaitingForChoice {
                    entry_index: u8::try_from(self.entries.len() - 1).unwrap(),
                    choice: Some(choice),
                };
            }
            QueueState::WaitingForChoice { .. }
            | QueueState::WaitingForAutoAbilityChoice { .. } => {}
        }
    }

    pub fn pause_for_auto_ability_choice(&mut self, choice: Choice) {
        self.snapshot_requested = true;
        self.state = QueueState::WaitingForAutoAbilityChoice { choice };
    }

    /// Resume after user provides choice result
    pub fn resume_with_choice(&mut self) {
        match &self.state {
            QueueState::WaitingForAutoAbilityChoice { .. } => {
                self.state = QueueState::Idle;
            }
            QueueState::WaitingForChoice { entry_index, .. } => {
                self.state = QueueState::ExecutingEffect {
                    entry_index: *entry_index,
                };
            }
            _ => {}
        }
    }

    /// Mark current ability as completed and move to idle state
    pub fn complete_current(&mut self) {
        match &self.state {
            QueueState::PayingCost { entry_index }
            | QueueState::WaitingForChoice { entry_index, .. }
            | QueueState::ExecutingEffect { entry_index }
            | QueueState::Completed { entry_index } => {
                if let Some(entry) = self.entries.get_mut(*entry_index as usize) {
                    entry.completed = true;
                }
            }
            _ => {}
        }
        self.state = QueueState::Idle;
        // Saturating: `current_index` is a `u8`, and a random-policy playout over many
        // decks reached 255 entries.
        self.current_index = self.current_index.saturating_add(1);
    }

    /// Skip remaining abilities for a specific card (e.g., after optional cost skip)
    /// Clear completed entries to free memory
    pub fn clear_completed(&mut self) {
        self.entries.retain(|e| !e.completed);
        if (self.current_index as usize) > self.entries.len() {
            self.current_index = 0;
        }
    }

    /// Push a temporary context entry for constant ability evaluation.
    /// This gives resolve_target_player (which reads ability_master_id)
    /// the correct player context when evaluating success zone constants
    /// outside the normal ability queue flow.
    pub fn push_constant_context(&mut self, player_id: String) {
        let idx = self.entries.len();
        self.entries.push(AbilityQueueEntry {
            card_no: String::new(),
            player_id,
            ability: Arc::new(Ability {
                full_text: String::new(),
                triggerless_text: None,
                triggers: None,
                use_limit: None,
                is_null: true,
                cost: None,
                effect: None,
                keywords: None,
                #[cfg(not(feature = "no_std"))]
                trigger_mask: Default::default(),
                #[cfg(not(feature = "no_std"))]
                trigger_position_mask: Default::default(),
            }),
            ability_index: 0,
            card_id: None,
            trigger_type: AbilityTrigger::Auto,
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
            trigger_moved_cards: None,
            trigger_appeared_cards: SmallVec::new(),
            triggering_member_id: None,
            snapshot_movements: SmallVec::new(),
            choice_effect_text: None,
            choice_repeats_remaining: 0,
            condition_cache: SmallVec::new(),
        });
        self.state = QueueState::ExecutingEffect {
            entry_index: u8::try_from(idx).unwrap(),
        };
    }

    /// Pop the temporary constant evaluation context and restore idle.
    pub fn pop_constant_context(&mut self) {
        self.entries.pop();
        self.state = QueueState::Idle;
    }

    /// Get queue state for debugging
    pub fn get_state(&self) -> &QueueState {
        &self.state
    }

    /// Get all pending entries
    pub fn pending_entries(&self) -> Vec<&AbilityQueueEntry> {
        self.pending_entries_iter().collect()
    }

    /// Iterate pending entries directly without materializing an intermediate collection.
    pub fn pending_entries_iter(&self) -> impl Iterator<Item = &AbilityQueueEntry> {
        self.entries.iter().filter(|e| !e.completed)
    }

    /// Store deferred sequential commands on the current entry.
    /// Replace pending actions on the current entry.
    pub fn set_pending_actions(&mut self, actions: Vec<AbilityEffect>) {
        if let Some(entry) = self.current_entry_mut() {
            entry.pending_actions = actions;
        }
    }

    /// Append actions to the existing pending actions.
    pub fn save_pending_actions(&mut self, actions: Vec<AbilityEffect>) {
        if actions.is_empty() {
            return;
        }
        if let Some(entry) = self.current_entry_mut() {
            entry.pending_actions.extend(actions);
        }
    }

    /// Drain and return pending actions from the current entry.
    pub fn take_pending_actions(&mut self) -> Vec<AbilityEffect> {
        if let Some(entry) = self.current_entry_mut() {
            core::mem::take(&mut entry.pending_actions)
        } else {
            Vec::new()
        }
    }

    /// Check if the current entry has pending actions.
    pub fn has_pending_actions(&self) -> bool {
        self.current_entry()
            .is_some_and(|e| !e.pending_actions.is_empty())
    }

    /// Move the entry at `from_index` to the front of the queue (position 0).
    /// Used by Rule 9.5.3 auto-ability ordering: player picks which standby ability
    /// resolves first, and this moves it to the head of the queue.
    pub fn promote_entry(&mut self, from_index: usize) {
        let absolute = self.current_index as usize + from_index;
        if absolute >= self.entries.len() || from_index == 0 {
            return;
        }
        let entry = self.entries.remove(absolute);
        self.entries.insert(0, entry);
        if (self.current_index as usize) > absolute {
            self.current_index = self.current_index.saturating_sub(1);
        } else {
            self.current_index = 0;
        }
    }
    /// Check if an entry exists and is not completed.
    pub fn is_entry_available(&self, index: usize) -> bool {
        index < self.entries.len() && !self.entries[index].completed
    }

    /// Get the player_id of an entry.
    pub fn entry_player_id(&self, index: usize) -> Option<&str> {
        self.entries.get(index).map(|e| e.player_id.as_str())
    }

    /// Move an entry at an absolute index to position 0 and reset current_index.
    /// The only promotion path; it used to have a relative-from-current_index
    /// twin (`promote_entry`) that nothing called.
    pub fn promote_entry_by_abs(&mut self, absolute: usize) {
        if absolute >= self.entries.len() || absolute == 0 {
            if absolute == 0 {
                self.current_index = 0;
            }
            return;
        }
        let entry = self.entries.remove(absolute);
        self.entries.insert(0, entry);
        self.current_index = 0;
    }

    /// Set the current index to a specific entry without reordering the queue
    /// (unlike promote_entry_by_abs which removes/inserts). Used by depth-first
    /// drain loops to process newly-queued entries in-place.
pub fn set_current_entry(&mut self, absolute: usize) {
        if absolute < self.entries.len() {
            // `current_index` is a `u8`, so the queue can only ADDRESS 256 entries.
            // A random-policy playout over many decks reached 256 and panicked here
            // (`TryFromIntError(PosOverflow)`) with all 4173 tests green — nothing
            // builds a queue that deep.
            //
            // Saturating rather than unwrapping: below the limit this is identical, and
            // above it the engine degrades to the last addressable entry and says so
            // instead of dying in a shipped binary. The real fix is widening
            // `current_index` and the entries' `entry_index` to `usize`, which reaches
            // the chooser payload and the web JSON.
            self.current_index = u8::try_from(absolute).unwrap_or_else(|_| {
                log::error!(
                    "[QUEUE_CAP] set_current_entry({absolute}) with {} entries: \
                     current_index is a u8 and saturates at 255. Widen the index type.",
                    self.entries.len()
                );
                u8::MAX
            });
        }
    }

    /// Take the resolver out of the current entry (for use with game_state).
    pub fn take_resolver(&mut self) -> Option<Box<AbilityResolver>> {
        self.current_entry_mut().and_then(|e| e.resolver.take())
    }

    /// Put the resolver back into the current entry.
    pub fn set_resolver(&mut self, resolver: Box<AbilityResolver>) {
        if let Some(entry) = self.current_entry_mut() {
            entry.resolver = Some(resolver);
        }
    }

    /// Check if the current entry has a resolver (i.e. ability execution is in progress).
    pub fn has_resolver(&self) -> bool {
        self.current_entry()
            .and_then(|e| e.resolver.as_ref())
            .is_some()
    }

    /// Dump the current queue state as a multi-line string (for test debug).
    pub fn dump_state(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("state={:?}", self.state));
        s.push('\n');
        s.push_str(&format!("current_index={}", self.current_index));
        s.push('\n');
        s.push_str(&format!("entries={}", self.entries.len()));
        s.push('\n');
        for entry in self.entries.iter() {
            s.push_str(&format!(
                "  [{}] card={} ab#{} player={} completed={} cost_paid={} effect_started={} optional_cost_result={:?} pending_actions={}\n",
                entry.ability_index,
                entry.card_no,
                entry.ability_index,
                entry.player_id,
                entry.completed,
                entry.cost_paid,
                entry.effect_started,
                entry.optional_cost_result,
                entry.pending_actions.len(),
            ));
        }
        s
    }
}

impl Default for AbilityQueue {
    fn default() -> Self {
        Self::new()
    }
}
