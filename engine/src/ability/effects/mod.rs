//! `effects` - one file per action in `cards/abilities.json`.
//!
//! Each file is named for the `action` wire string it implements and holds
//! exactly that action's executor, so "which code runs `gain_resource`?" has
//! the answer `resource.rs`. The counts in each header come from
//! `docs/ABILITY_MATRIX.md`, which is generated from `cards/abilities.json`.
//!
//! `mod.rs` is what runs before dispatch (the guards and the routing
//! normalisations). `dispatch.rs` is the `ActionType` table itself. Two files
//! here are not actions and say so in their headers: `target.rs` (target
//! resolution, read by every executor) and `replacement.rs` (the
//! replacement-effect mechanism).

pub mod ability_ops;
pub mod blade;
pub mod change_state;
pub mod choice;
pub mod cost;
pub mod deck;
pub mod dispatch;
pub mod energy;
pub mod heart;
pub mod look;
pub mod position;
pub mod replacement;
pub mod required_hearts;
pub mod resource;
pub mod restriction;
pub mod reyell;
pub mod score;
pub mod selection;
pub mod target;

/// `move_cards` and its placement / selection / transport helpers. The files
/// live in `move/`; the module keeps the old name because `move` is a keyword.
#[path = "move/mod.rs"]
pub mod move_cards;

pub(crate) use deck::draw_cards_for_player;
pub(crate) use dispatch::execute_effect as execute_effect_dispatch;

use super::debug::AbDebug;
use super::enums::ActionType;
use super::resolver::AbilityResolver;
// `crate::Box` is the compat alias: `alloc::boxed::Box` under `no_std`,
// `std::boxed::Box` on host. The `Cow<[Box<AbilityEffect>]>` signature below
// needs it in scope on every target.
use crate::Box;

use super::util;
use crate::card::AbilityEffect;
#[cfg(not(feature = "no_std"))]
use std::borrow::Cow;
#[cfg(feature = "no_std")]
use alloc::borrow::Cow;
use crate::game_state::GameState;
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

impl AbilityResolver {
    /// Activation gate: skipped effects resolve to Ok without verdicts —
    /// condition failure info is captured by push_ability_result upstream.
    /// Returns true when the caller must `return Ok(())`.
    fn gate_effect_activation(&mut self, gs: &mut GameState, effect: &AbilityEffect) -> bool {
        if self.can_activate_effect(gs, effect) {
            return false;
        }
        log::trace!("[EFFECT] source={:?} action={} skipped: activation gate failed", self.session.activating_card_id, effect.action);
        // Keep verdicts — condition failure info will be captured by push_ability_result
        true
    }

    /// Q118 all-or-nothing "そうしたとき": after an accepted
    /// conditional_on_optional placement that could not place every required
    /// card, the trailing draw consequence must not fire — whether it runs
    /// in the sequential loop or via resume_pending_actions.
    fn gate_incomplete_placement(&mut self, gs: &mut GameState, effect: &AbilityEffect) -> bool {
        let stopped = super::gates::incomplete_placement_gate(self, gs, effect).is_stop();
        if stopped {
            log::debug!(
                "[EFFECT] source={:?} action={} skipped: placement incomplete (Q118)",
                self.session.activating_card_id,
                effect.action
            );
        }
        stopped
    }

    /// Non-stackable check: skip if this effect is already active.
    /// Returns true when the caller must `return Ok(())`.
    fn check_non_stackable(&mut self, gs: &mut GameState, effect: &AbilityEffect) -> bool {
        let stopped = super::gates::non_stackable_gate(self, gs, effect).is_stop();
        if stopped {
            log::debug!(
                "[EFFECT] source={:?} action={} skipped: non-stackable effect already active",
                self.session.activating_card_id,
                effect.action
            );
        }
        stopped
    }

    /// Legacy opponent_action wrapper (pre-parser-flatten) + flat
    /// `action_by: opponent` tagging. Returns `Ok(true)` when the caller
    /// must `return Ok(())` (unwrapped recursion finished, or a fully
    /// opponent-handled Custom needs nothing).
    fn prepare_opponent_routing(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<bool, String> {
        // Flat effects carry target="opponent" directly and dispatch via
        // ActionType; only the legacy wrapper needs unwrapping.
        if effect.action == ActionType::OpponentAction {
            if let Some(opponent_action) = effect.opponent_action() {
                // G3: tag spawn context so choices created for this
                // opponent action are routed to the opponent player.
                self.in_flight.spawn_context.target = Some("opponent".to_string());
                let mut modified = opponent_action.clone();
                if modified.target.is_none() || modified.target.as_deref() == Some("self") {
                    modified.target = Some("opponent".into());
                }
                self.execute_effect(gs, &modified)?;
                return Ok(true);
            }
        }

        // Empty action (default) with action_by means it was entirely handled by opponent
        if effect.action == ActionType::Custom && effect.action_by().is_some() {
            return Ok(true);
        }

        // G3: for non-empty actions with action_by: opponent, tag spawn context
        // so choices created inside are routed to the opponent player.
        if effect.action_by() == Some("opponent") {
            self.in_flight.spawn_context.target = Some("opponent".to_string());
        }
        Ok(false)
    }

    /// Rule 9.2.1 compound routing: Sequential and LookAndSelect carrying
    /// `effect_steps` collapse into the generic sequential pipeline
    /// ([look, select, move] in order).
    ///
    /// Returns the normalized step list, borrowed whenever the effect already
    /// holds it. It used to return a synthetic `AbilityEffect` built by
    /// deep-cloning the parent and attaching a cloned step list — ~4
    /// allocations and ~1.7 KB of memcpy per step, ~128 for a typical 3-step
    /// ability — and the only three fields that clone actually changed
    /// (`effect_steps`, `compound.actions`, `action`) are either never read back
    /// or are read only by a debug log, which the caller reproduces via
    /// `action_for_log`.
    fn normalized_sequential_steps(
        effect: &AbilityEffect,
        action_type: ActionType,
    ) -> Option<Cow<'_, [Box<AbilityEffect>]>> {
        // Rule 9.8.1 / Q85 / Q86: Sequential/LookAndSelect routing
        //
        // Q85: LookAndSelect with insufficient deck → refresh mid-look
        //   (handled inside execute_look_at, which implements the 4-step
        //   procedure: look available → refresh → look remaining → resolve)
        //
        // Q86: LookAndSelect with exactly enough cards → no refresh during
        //   look. After resolution, if deck is 0, refresh on next check timing.
        if action_type != ActionType::Sequential && action_type != ActionType::LookAndSelect {
            return None;
        }
        let steps = effect.normalized_steps_cow();
        if steps.is_empty() {
            return None;
        }
        Some(steps)
    }

    // Q55: Effects resolve as much as possible; partial resolution required when full is impossible.
    pub fn execute_effect(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let mut dbg = AbDebug::new();
        dbg.effect(effect);
        log::trace!(
            "[EFFECT] source={:?} action={} from={} to={} has_steps={} has_actions={}",
            self.session.activating_card_id,
            effect.action,
            effect.source_or("none"),
            effect.destination.map(|z| z.as_str()).unwrap_or("none"),
            effect.effect_steps.is_some(),
            effect.compound.actions.is_some()
        );
        #[cfg(not(feature = "no_std"))]
        let exec_snapshot = crate::ability::log::buffer_len();
        if self.gate_effect_activation(gs, effect) {
            return Ok(());
        }
        if self.gate_incomplete_placement(gs, effect) {
            return Ok(());
        }
        // Drain condition verdicts from the can_activate_effect pre-check;
        // the effect execution will produce its own items and we don't want duplicates.
        #[cfg(not(feature = "no_std"))]
        {
            let _pre = crate::ability::log::drain_verdicts_since(exec_snapshot);
        }

        // non_stackable check: skip if this effect is already active
        if self.check_non_stackable(gs, effect) {
            return Ok(());
        }

        // Effect details are captured in the structured ability_resolution entry

        #[cfg(not(feature = "no_std"))]
        let _t = crate::timer::Timer::start("effect::gates");
        if self.prepare_opponent_routing(gs, effect)? {
            #[cfg(not(feature = "no_std"))]
            drop(_t);
            return Ok(());
        }

        if self.run_replacement_effects(gs, effect)? {
            #[cfg(not(feature = "no_std"))]
            drop(_t);
            return Ok(());
        }
        if self.awaiting.replacement_original_suppressed {
            self.awaiting.replacement_original_suppressed = false;
            log::debug!(
                "[REPLACEMENT] action={} original_suppressed=true",
                effect.action
            );
            #[cfg(not(feature = "no_std"))]
            drop(_t);
            return Ok(());
        }

        if self.register_replacement_effect(gs, effect) {
            #[cfg(not(feature = "no_std"))]
            drop(_t);
            return Ok(());
        }

        // Rule 9.8 / Q158: Handle target="both" generically
        //   execute once for self, then opponent.
        //   position_change handles "both" internally (opponent first, then self).
        //
        // Q158: "Does blade+2 from the effect apply to all members on stage?"
        //   → Yes. "自分のステージにいるメンバー" means all members.
        if self.handle_both_targets(gs, effect)? {
            return Ok(());
        }

        // Rule 9.2.1: Effect dispatch — table-driven via executor registry
        let action_type = effect.action;

        // Compound routing: Sequential and LookAndSelect both route through
        // the generic sequential pipeline when they carry `effect_steps`.
        if let Some(steps) = Self::normalized_sequential_steps(effect, action_type) {
            return crate::ability::compound::sequential::execute_sequential_effect(
                self,
                gs,
                effect,
                &steps,
                // The old synthetic parent had `action` forced to Sequential, so
                // that is what the `[SEQUENCE]` debug line printed for routed
                // effects. Keep the output byte-identical.
                ActionType::Sequential,
            );
        }

        #[cfg(not(feature = "no_std"))]
        let _t = crate::timer::Timer::start("effect::dispatch");
        execute_effect_dispatch(self, gs, effect)
    }
}