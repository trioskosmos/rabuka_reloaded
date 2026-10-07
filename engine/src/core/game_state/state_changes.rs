//! State-change bookkeeping: the ONE place that records, and the ONE place
//! that clears, orientation transitions.
//!
//! A printed watcher never says *which mechanism* flipped a member —
//! 「メンバーがウェイト状態になったとき」 fires whether the wait came from
//! paying a cost, resolving an effect, a move/transport, a stage-select
//! resume, a replacement rule, or an energy flip. So every orientation write
//! in the engine goes through [`GameState::set_orientation`], which appends to
//! two logs:
//!
//! * `recently_state_changed` — (card, from, to, cause_player). The
//!   visibility WINDOW: watchers that resolve right after the transition read
//!   it, so it is cleared as soon as nothing is waiting on it. See
//!   [`GameState::clear_state_change_window`] and its deferred twin.
//! * `turn_state_changes` — (source card, target, from, to). The TURN-SCOPED
//!   attributed log used by 「このターン…」 temporal clauses (Q203). Only the
//!   turn boundary clears it.
//!
//! Cards whose wait lives outside `mods.orientation_modifiers` — waited
//! ENERGY, tracked by `energy_zone`'s waited-set — call
//! [`GameState::record_state_change`] with an explicit `from` instead, because
//! the modifier lookup would see `active→active` and record nothing.
//!
//! Invariant: a transition is recorded exactly once, by the write that makes
//! it. `if from == to` is a no-op, so re-applying "wait" to an already-waiting
//! member never double-counts (multicity like `[ターン3回]` is computed from
//! the log, not from modifier writes).

use super::GameState;

impl GameState {
    /// Apply an orientation change AND record the transition.
    ///
    /// This is the only sanctioned way to change a card's orientation. Direct
    /// `mods.add_orientation_modifier(...)` calls are a bug unless the write
    /// is a turn-boundary reset that must stay invisible to watchers (the
    /// Active-phase wake-up in `turn/phases.rs`).
    pub fn set_orientation(&mut self, card_id: i16, to: &str) {
        let from = self
            .mods
            .get_orientation_modifier(card_id)
            .unwrap_or("active")
            .to_string();
        self.mods.add_orientation_modifier(card_id, to);
        if from == to {
            return;
        }
        self.record_state_change(card_id, &from, to);
    }

    /// Apply an orientation change with the caller supplying `from`.
    ///
    /// For cards whose current state is tracked OUTSIDE the modifier table —
    /// waited ENERGY lives in `energy_zone`'s waited-set — a modifier lookup
    /// would report `active→active` and the transition would be lost. The
    /// caller knows the real origin, so it states it.
    pub fn set_orientation_from(&mut self, card_id: i16, from: &str, to: &str) {
        self.mods.add_orientation_modifier(card_id, to);
        self.record_state_change(card_id, from, to);
    }

    /// Record a transition that the modifier table cannot show on its own
    /// (energy-zone wait) or that a caller already knows precisely.
    ///
    /// `from`/`to` are `"active" | "wait" | "rest"`; `cause` (the player who
    /// owns the activating card) is derived the same way the effect path does:
    /// activating card's owner, else the queue entry's player, else "".
    pub fn record_state_change(&mut self, card_id: i16, from: &str, to: &str) {
        if from == to {
            return;
        }
        let cause_pid = self
            .activating_card
            .and_then(|cid| {
                if self.player1.contains_card(cid) {
                    Some(self.player1.id.to_string())
                } else if self.player2.contains_card(cid) {
                    Some(self.player2.id.to_string())
                } else {
                    None
                }
            })
            .or_else(|| {
                self.ability_queue
                    .current_entry()
                    .map(|e| e.player_id.to_string())
            })
            .unwrap_or_default();
        log::debug!(
            "[STATE_CHANGE] recorded: card={} {}→{} cause={}",
            card_id,
            from,
            to,
            cause_pid
        );
        self.recently_state_changed.push((
            card_id,
            from.to_string(),
            to.to_string(),
            cause_pid,
        ));
        self.turn_state_changes.push((
            self.activating_card.unwrap_or(-1),
            card_id,
            from.to_string(),
            to.to_string(),
        ));
    }

    /// Close the visibility window NOW — every watcher that could read these
    /// transitions has already resolved.
    pub fn clear_state_change_window(&mut self, reason: &str) {
        self.recently_state_changed.clear();
        log::debug!("[RSC_CLEAR] {} (state_changes.rs)", reason);
    }

    /// Close the visibility window only if nothing is waiting on it.
    ///
    /// Paying a cost inside `process_current_ability` can itself be a state
    /// change (「このメンバーをウェイトにする」), and the watcher queued
    /// behind the still-pending choice has to see that transition when the
    /// choice is answered. Clearing here would erase it first, so the clear is
    /// deferred until the queue is idle again.
    pub fn clear_state_change_window_deferred(&mut self, reason: &str) {
        if self.has_pending_choice() {
            log::debug!("[RSC_CLEAR] {} deferred, pending choice", reason);
        } else {
            self.clear_state_change_window(reason);
        }
    }
}
