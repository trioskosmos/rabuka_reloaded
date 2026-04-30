use crate::card::Ability;
use crate::ability_resolver::Choice;
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
    /// Whether this ability has been started (cost paid)
    pub started: bool,
    /// Whether this ability has been completed
    pub completed: bool,
    /// Stored choice result for resumption
    pub pending_choice_result: Option<crate::ability_resolver::ChoiceResult>,
}

/// Unified ability queue with proper state management
#[derive(Debug, Clone)]
pub struct AbilityQueue {
    entries: Vec<AbilityQueueEntry>,
    state: QueueState,
    current_index: usize,
}

impl AbilityQueue {
    pub fn new() -> Self {
        AbilityQueue {
            entries: Vec::new(),
            state: QueueState::Idle,
            current_index: 0,
        }
    }

    /// Check if queue is idle (no ability being processed)
    pub fn is_idle(&self) -> bool {
        matches!(self.state, QueueState::Idle)
    }

    /// Check if queue is waiting for user choice
    pub fn is_waiting_for_choice(&self) -> Option<&Choice> {
        match &self.state {
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
            QueueState::Idle => None,
        }
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
                self.state = QueueState::PayingCost { entry_index: self.current_index };
                return true;
            }
            self.current_index += 1;
        }
        false
    }

    /// Pause for user choice during ability execution
    pub fn pause_for_choice(&mut self, choice: Choice) {
        match &self.state {
            QueueState::PayingCost { entry_index } | QueueState::ExecutingEffect { entry_index } => {
                self.state = QueueState::WaitingForChoice {
                    entry_index: *entry_index,
                    choice,
                };
            }
            _ => {}
        }
    }

    /// Resume after user provides choice result
    pub fn resume_with_choice(&mut self, result: crate::ability_resolver::ChoiceResult) {
        match &self.state {
            QueueState::WaitingForChoice { entry_index, .. } => {
                if let Some(entry) = self.entries.get_mut(*entry_index) {
                    entry.pending_choice_result = Some(result);
                }
                self.state = QueueState::ExecutingEffect { entry_index: *entry_index };
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
    pub fn skip_card_abilities(&mut self, card_no: &str) {
        for entry in self.entries.iter_mut() {
            if entry.card_no == card_no && !entry.started {
                entry.completed = true;
            }
        }
    }

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

    /// Transition from paying cost to executing effect
    pub fn transition_to_executing(&mut self) {
        match &self.state {
            QueueState::PayingCost { entry_index } => {
                if let Some(entry) = self.entries.get_mut(*entry_index) {
                    entry.started = true;
                }
                self.state = QueueState::ExecutingEffect { entry_index: *entry_index };
            }
            _ => {}
        }
    }
}

impl Default for AbilityQueue {
    fn default() -> Self {
        Self::new()
    }
}
