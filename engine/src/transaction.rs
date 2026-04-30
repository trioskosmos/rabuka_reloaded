//! Transaction and rollback support for GameState.
//!
//! Rule 9.6.2: If a cost cannot be paid, the ability is not activated
//! and the game state must not be modified.
//!
//! This module provides `with_rollback` which snapshots mutable state
//! before an operation and restores it on failure.

use crate::game_state::GameState;

pub trait Transactional {
    /// Execute `f` inside a transaction. If `f` returns `Err`, all
    /// mutations to `self` are rolled back to the pre-transaction state.
    fn with_rollback<F, R>(&mut self, f: F) -> Result<R, String>
    where
        F: FnOnce(&mut Self) -> Result<R, String>;
}

impl Transactional for GameState {
    fn with_rollback<F, R>(&mut self, f: F) -> Result<R, String>
    where
        F: FnOnce(&mut Self) -> Result<R, String>,
    {
        let snapshot = self.clone();
        match f(self) {
            Ok(r) => Ok(r),
            Err(e) => {
                // Restore from snapshot
                *self = snapshot;
                Err(e)
            }
        }
    }
}

/// Helper for the ability resolver: pay cost + execute effect atomically.
pub fn resolve_ability_atomic<F>(game_state: &mut GameState, f: F) -> Result<(), String>
where
    F: FnOnce(&mut GameState) -> Result<(), String>,
{
    game_state.with_rollback(f)
}
