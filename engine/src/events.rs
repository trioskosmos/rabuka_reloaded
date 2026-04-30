use crate::game_state::GameState;
use std::sync::Mutex;

/// Every significant change in the game produces a `GameEvent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameEvent {
    PhaseStarted { phase: String },
    TurnStarted { turn_number: u32, player_id: String },
    CardMoved { card_id: i16, from: String, to: String, player_id: String },
    CardRevealed { card_id: i16, player_id: String, zone: String },
    CardDrawn { card_id: i16, player_id: String, source: String },
    MemberDebuted { card_id: i16, player_id: String, area: String },
    MemberPositionChanged { card_id: i16, player_id: String, from_area: String, to_area: String },
    LiveStarted { player_id: String },
    LiveSucceeded { player_id: String, score: u32 },
    EnergyPaid { player_id: String, amount: u32 },
    BladeGained { card_id: i16, player_id: String, amount: u32 },
    HeartGained { card_id: i16, player_id: String, color: String, amount: u32 },
    StateChanged { card_id: i16, player_id: String, new_state: String },
    AbilityActivated { card_id: i16, player_id: String, ability_index: usize },
    AbilityResolved { card_id: i16, player_id: String, ability_index: usize },
    CostWouldBePaid { player_id: String, cost_type: String },
    Custom { name: String, data: String },
}

/// A listener that reacts to game events.
pub trait EventListener: Send {
    fn on_event(&mut self, event: &GameEvent, game_state: &mut GameState);
}

/// Global registry of event listeners (thread-safe, registered once at startup).
static LISTENER_REGISTRY: std::sync::LazyLock<Mutex<Vec<Box<dyn EventListener>>>> =
    std::sync::LazyLock::new(|| Mutex::new(Vec::new()));

pub fn register_listener<L: EventListener + 'static>(listener: L) {
    if let Ok(mut registry) = LISTENER_REGISTRY.lock() {
        registry.push(Box::new(listener));
    }
}

/// Deliver a single event to all registered global listeners.
fn dispatch_event(event: &GameEvent, game_state: &mut GameState) {
    if let Ok(mut registry) = LISTENER_REGISTRY.lock() {
        for listener in registry.iter_mut() {
            listener.on_event(event, game_state);
        }
    }
}

/// Event queue: stores pending events, delivers them in batch on `flush`.
///
/// `EventBus` does NOT take `&mut GameState` in `publish()` — events are
/// queued without borrow conflicts. Call `flush(game_state)` to deliver
/// all queued events to listeners, typically at check-timing boundaries.
#[derive(Debug, Clone)]
pub struct EventBus {
    pending: Vec<GameEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        Self { pending: Vec::new() }
    }

    /// Queue an event for later delivery. Does NOT invoke listeners.
    pub fn publish(&mut self, event: GameEvent) {
        self.pending.push(event);
    }

    /// Deliver all pending events to global listeners.
    /// Should be called at check-timing boundaries (Rule 9.5.1).
    pub fn flush(&mut self, game_state: &mut GameState) {
        let events = std::mem::take(&mut self.pending);
        for event in events {
            dispatch_event(&event, game_state);
        }
    }

    /// Cancel all pending events (e.g. on rollback).
    pub fn clear(&mut self) {
        self.pending.clear();
    }

    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }
}

/// Built-in listener that enqueues auto-abilities on game events.
pub struct AutoAbilityListener;

impl EventListener for AutoAbilityListener {
    fn on_event(&mut self, event: &GameEvent, game_state: &mut GameState) {
        match event {
            GameEvent::MemberDebuted { card_id, player_id, .. } => {
                Self::check_triggers(*card_id, player_id, &["登場", "Debut"], game_state);
            }
            GameEvent::LiveStarted { player_id } => {
                Self::check_all_stage_triggers(player_id, &["ライブ開始時", "live_start"], game_state);
            }
            GameEvent::LiveSucceeded { player_id, .. } => {
                Self::check_all_stage_triggers(player_id, &["ライブ成功時", "live_success"], game_state);
            }
            GameEvent::MemberPositionChanged { card_id, player_id, .. } => {
                Self::check_triggers(*card_id, player_id, &["ポジションチェンジ", "position_change"], game_state);
            }
            _ => {}
        }
    }
}

impl AutoAbilityListener {
    fn check_triggers(card_id: i16, player_id: &str, keywords: &[&str], game_state: &mut GameState) {
        if let Some(card) = game_state.card_database.get_card(card_id) {
            for (idx, ability) in card.abilities.iter().enumerate() {
                if let Some(ref triggers) = ability.triggers {
                    if keywords.iter().any(|kw| triggers.contains(kw)) {
                        game_state.ability_queue.enqueue(
                            crate::ability_queue::AbilityQueueEntry {
                                id: crate::ability_queue::AbilityId::new(&card.card_no, idx, keywords[0]),
                                card_no: card.card_no.clone(),
                                player_id: player_id.to_string(),
                                ability: ability.clone(),
                                ability_index: idx,
                                card_id: Some(card_id),
                                trigger_type: Self::tt(keywords[0]),
                                started: false,
                                completed: false,
                                pending_choice_result: None,
                            }
                        );
                    }
                }
            }
        }
    }

    fn check_all_stage_triggers(player_id: &str, keywords: &[&str], game_state: &mut GameState) {
        let player = if game_state.player1.id == player_id {
            &game_state.player1
        } else {
            &game_state.player2
        };
        for &card_id in &player.stage.stage {
            if card_id == -1 { continue; }
            if let Some(card) = game_state.card_database.get_card(card_id) {
                for (idx, ability) in card.abilities.iter().enumerate() {
                    if let Some(ref triggers) = ability.triggers {
                        if keywords.iter().any(|kw| triggers.contains(kw)) {
                            game_state.ability_queue.enqueue(
                                crate::ability_queue::AbilityQueueEntry {
                                    id: crate::ability_queue::AbilityId::new(&card.card_no, idx, keywords[0]),
                                    card_no: card.card_no.clone(),
                                    player_id: player_id.to_string(),
                                    ability: ability.clone(),
                                    ability_index: idx,
                                    card_id: Some(card_id),
                                    trigger_type: Self::tt(keywords[0]),
                                    started: false,
                                    completed: false,
                                    pending_choice_result: None,
                                }
                            );
                        }
                    }
                }
            }
        }
    }

    fn tt(s: &str) -> crate::game_state::AbilityTrigger {
        use crate::game_state::AbilityTrigger;
        match s {
            "登場" | "Debut" => AbilityTrigger::Debut,
            "ライブ開始時" | "live_start" => AbilityTrigger::LiveStart,
            "ライブ成功時" | "live_success" => AbilityTrigger::LiveSuccess,
            _ => AbilityTrigger::Auto,
        }
    }
}
