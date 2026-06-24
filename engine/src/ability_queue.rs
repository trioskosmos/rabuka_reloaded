use crate::ability::resolver::AbilityResolver;
use crate::ability::types::Choice;
use crate::card::{Ability, AbilityEffect};
use crate::game_state::AbilityTrigger;

/// Unique identifier for an ability instance in the queue
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbilityId(pub String);

impl AbilityId {
    pub fn new(card_no: &str, ability_index: usize, trigger_type: &str) -> Self {
        AbilityId(format!("{}_{}_{}", card_no, ability_index, trigger_type))
    }
}

/// Current state of ability queue processing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueState {
    /// Queue is idle, ready to process next ability
    Idle,
    /// Waiting for player to choose which auto ability resolves first (Rule 9.5.3)
    WaitingForAutoAbilityChoice { choice: Choice },
    /// Currently paying cost for an ability
    PayingCost { entry_index: usize },
    /// Waiting for user choice (cost payment, target selection, etc.)
    WaitingForChoice { entry_index: usize, choice: Choice },
    /// Executing the effect of an ability
    ExecutingEffect { entry_index: usize },
    /// Ability completed, will transition to Idle
    Completed { entry_index: usize },
}

/// Entry in the ability queue
#[derive(Debug, Clone)]
pub struct AbilityQueueEntry {
    pub id: AbilityId,
    pub card_no: String,
    pub player_id: String,
    pub ability: Ability,
    pub ability_index: usize,
    pub card_id: Option<i16>,
    pub trigger_type: AbilityTrigger,
    /// Whether this ability has been completed
    pub completed: bool,
    /// Whether the cost has been fully paid (for re-entry after cost choice)
    pub cost_paid: bool,
    /// Stored choice result for resumption
    pub cost_paid_index: usize,
    pub pending_choice_result: Option<crate::ability::types::ChoiceResult>,
    /// Discriminator for routing choice results to the correct handler.
    pub choice_card_no: Option<crate::ability::types::ChoiceRoute>,
    /// JSON-serialized options for choice/choice_string discriminators
    pub conditional_choice: Option<String>,
    /// Whether the effect has started executing (prevents re-processing)
    pub effect_started: bool,
    /// Result of the optional cost evaluation.
    /// None = not evaluated, Some(true) = paid, Some(false) = skipped.
    pub optional_cost_result: Option<bool>,
    /// Player who must make the pending choice (if different from activating player)
    pub choice_player_id: Option<String>,
    /// Card IDs that triggered this each_time auto ability (snapshot of
    /// recently_moved_cards at enqueue time). Used by source:"those_cards"
    /// to resolve to only the trigger cards, not the full discard pile.
    pub trigger_moved_cards: Option<Vec<i16>>,
    /// Snapshot of the full batch_movements vec captured at enqueue time.
    /// Used by the "moves" condition to check what triggered the ability,
    /// even after clear_effect_tracking() clears the global flags.
    pub snapshot_movements: Vec<crate::types::MovementEvent>,
    pub snapshot_energy_placed_by_effect: bool,
    pub snapshot_energy_placed_by_player: Option<String>,
    /// Snapshot of stage positions captured at enqueue time.
    /// Used by the "has_moved" condition to detect stage-area-to-stage-area
    /// position changes, even after a new process_current_ability call
    /// overwrites the GameState-wide snapshot.
    pub snapshot_stage_positions: Option<std::collections::HashMap<i16, usize>>,
    /// Actions queued for sequential execution after a choice round-trip.
    pub pending_actions: Vec<AbilityEffect>,
    /// Persistent ability resolver — stays alive across choice round-trips
    /// instead of being destroyed and recreated. Eliminates manual save/restore.
    pub resolver: Option<AbilityResolver>,
    /// For each_time triggers: the stage member card ID whose resolution
    /// caused this each_time ability to fire. Used by effects like
    /// "gain all-heart" to target the correct member.
    pub triggering_member_id: Option<i16>,
    /// The original effect text of a choice-type ability, stored so the frontend
    /// can use it as the choice prompt (separate from the option labels).
    pub choice_effect_text: Option<String>,
}

/// Unified ability queue with proper state management
#[derive(Debug, Clone)]
pub struct AbilityQueue {
    entries: Vec<AbilityQueueEntry>,
    state: QueueState,
    current_index: usize,
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

    /// Check if queue is idle (no ability being processed)
    pub fn is_idle(&self) -> bool {
        matches!(self.state, QueueState::Idle)
    }

    /// Check if queue is waiting for user choice
    pub fn is_waiting_for_choice(&self) -> Option<&Choice> {
        match &self.state {
            QueueState::WaitingForAutoAbilityChoice { choice } => Some(choice),
            QueueState::WaitingForChoice { choice, .. } => Some(choice),
            _ => None,
        }
    }

    /// Get current ability being processed
    pub fn current_entry(&self) -> Option<&AbilityQueueEntry> {
        match &self.state {
            QueueState::PayingCost { entry_index }
            | QueueState::WaitingForChoice { entry_index, .. }
            | QueueState::ExecutingEffect { entry_index }
            | QueueState::Completed { entry_index } => self.entries.get(*entry_index),
            QueueState::Idle | QueueState::WaitingForAutoAbilityChoice { .. } => None,
        }
    }

    pub fn current_entry_mut(&mut self) -> Option<&mut AbilityQueueEntry> {
        let idx = match &self.state {
            QueueState::PayingCost { entry_index }
            | QueueState::WaitingForChoice { entry_index, .. }
            | QueueState::ExecutingEffect { entry_index }
            | QueueState::Completed { entry_index } => *entry_index,
            QueueState::Idle | QueueState::WaitingForAutoAbilityChoice { .. } => return None,
        };
        self.entries.get_mut(idx)
    }

    pub fn get_entry(&self, index: usize) -> Option<&AbilityQueueEntry> {
        self.entries.get(index)
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

    /// Check if an entry with this ID already exists (completed or not)
    pub fn has_entry_with_id(&self, id: &AbilityId) -> bool {
        self.entries.iter().any(|e| e.id == *id)
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
        while self.current_index < self.entries.len() {
            let entry = &self.entries[self.current_index];
            if !entry.completed {
                self.state = QueueState::PayingCost {
                    entry_index: self.current_index,
                };
                return true;
            }
            self.current_index += 1;
        }
        false
    }

    /// Pause for user choice during ability execution
    pub fn pause_for_choice(&mut self, choice: Choice) {
        let choice_clone = choice.clone();
        match &mut self.state {
            QueueState::PayingCost { entry_index }
            | QueueState::ExecutingEffect { entry_index } => {
                let idx = *entry_index;
                // G1/G3: route choice to opponent when spawn context says opponent
                let opponent_id: Option<String> = self.entries.get(idx).and_then(|entry| {
                    if entry.choice_player_id.is_some() {
                        return None;
                    }
                    let is_opponent_choice = match &choice {
                        crate::ability::types::Choice::SelectCard {
                            target_player_id: Some(tpid),
                            ..
                        } if tpid == "opponent" => true,
                        _ => false,
                    };
                    if !is_opponent_choice {
                        return None;
                    }
                    let is_spawn_opponent = entry
                        .resolver
                        .as_ref()
                        .and_then(|r| r.spawn_context.target.as_deref())
                        == Some("opponent");
                    if !is_spawn_opponent {
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
                    entry_index: idx,
                    choice: choice_clone,
                };
            }
            QueueState::Idle | QueueState::Completed { .. } => {
                // Store the choice directly without an entry
                let dummy_entry = AbilityQueueEntry {
                    id: AbilityId::new("", 0, "choice"),
                    card_no: String::new(),
                    player_id: String::new(),
                    ability: Ability {
                        full_text: String::new(),
                        triggerless_text: String::new(),
                        triggers: None,
                        use_limit: None,
                        is_null: false,
                        cost: None,
                        effect: None,
                        keywords: None,
                    },
                    ability_index: 0,
                    card_id: None,
                    trigger_type: AbilityTrigger::Auto,
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
                    trigger_moved_cards: None,
                    triggering_member_id: None,
                    snapshot_movements: Vec::new(),
                    snapshot_energy_placed_by_effect: false,
                    snapshot_energy_placed_by_player: None,
                    snapshot_stage_positions: None,
                    choice_effect_text: None,
                };
                self.entries.push(dummy_entry);
                self.state = QueueState::WaitingForChoice {
                    entry_index: self.entries.len() - 1,
                    choice: choice_clone,
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
    pub fn resume_with_choice(&mut self, result: crate::ability::types::ChoiceResult) {
        match &self.state {
            QueueState::WaitingForAutoAbilityChoice { .. } => {
                self.state = QueueState::Idle;
            }
            QueueState::WaitingForChoice { entry_index, .. } => {
                if let Some(entry) = self.entries.get_mut(*entry_index) {
                    entry.pending_choice_result = Some(result);
                }
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
                if let Some(entry) = self.entries.get_mut(*entry_index) {
                    entry.completed = true;
                }
            }
            _ => {}
        }
        self.state = QueueState::Idle;
        self.current_index += 1;
    }

    /// Skip remaining abilities for a specific card (e.g., after optional cost skip)
    /// Clear completed entries to free memory
    pub fn clear_completed(&mut self) {
        self.entries.retain(|e| !e.completed);
        if self.current_index > self.entries.len() {
            self.current_index = 0;
        }
    }

    /// Get queue state for debugging
    pub fn get_state(&self) -> &QueueState {
        &self.state
    }

    /// Get all pending entries
    pub fn pending_entries(&self) -> Vec<&AbilityQueueEntry> {
        self.entries.iter().filter(|e| !e.completed).collect()
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
            std::mem::take(&mut entry.pending_actions)
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
        let absolute = self.current_index + from_index;
        if absolute >= self.entries.len() || from_index == 0 {
            return;
        }
        let entry = self.entries.remove(absolute);
        self.entries.insert(0, entry);
        if self.current_index > absolute {
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
    /// Unlike promote_entry which uses relative-from-current_index, this uses
    /// the direct absolute index in the entries array.
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
            self.current_index = absolute;
        }
    }

    /// Take the resolver out of the current entry (for use with game_state).
    pub fn take_resolver(&mut self) -> Option<AbilityResolver> {
        self.current_entry_mut().and_then(|e| e.resolver.take())
    }

    /// Put the resolver back into the current entry.
    pub fn set_resolver(&mut self, resolver: AbilityResolver) {
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
        for (_i, entry) in self.entries.iter().enumerate() {
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
