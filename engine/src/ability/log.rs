// Only the std verdict buffer reads the gate; the `no_std` buffer is inert.
#[cfg(not(feature = "no_std"))]
use crate::ability::debug::ABILITY_DEBUG;
#[cfg(feature = "serde_support")]
use serde::Serialize;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize))]
#[cfg_attr(feature = "serde_support", serde(tag = "kind"))]
pub enum AbilityLogItem {
    Condition {
        text: String,
        condition_type: String,
        expectation: String,
        actual: String,
        passed: bool,
        #[cfg_attr(
            feature = "serde_support",
            serde(default, skip_serializing_if = "Vec::is_empty")
        )]
        children: Vec<AbilityLogItem>,
    },
    Cost {
        text: String,
        expectation: String,
        actual: String,
        passed: bool,
        optional: bool,
    },
    Effect {
        text: String,
        action: String,
        details: String,
    },
}

#[cfg(feature = "no_std")]
mod inner {
    use super::AbilityLogItem;
    use alloc::vec::Vec;
    pub fn push_verdict(_item: AbilityLogItem) {}
    /// The `no_std` buffer is inert (`push_verdict` is a no-op), so the
    /// verdict-recording gate is always false. Callers gate on this BEFORE
    /// building their `String` arguments, so it has to exist here too — not
    /// just in the std module — or the console builds skip the gate entirely.
    #[inline]
    pub fn debug_enabled() -> bool {
        false
    }
    pub fn drain_verdicts() -> Vec<AbilityLogItem> {
        Vec::new()
    }
    pub fn buffer_len() -> usize {
        0
    }
    pub fn drain_verdicts_since(_start_index: usize) -> Vec<AbilityLogItem> {
        Vec::new()
    }
    pub fn clear_verdicts() {}
}

#[cfg(not(feature = "no_std"))]
mod inner {
    use super::{AbilityLogItem, ABILITY_DEBUG};
    use core::sync::atomic::Ordering;
    use std::cell::RefCell;

    thread_local! {
        static VERDICT_BUFFER: RefCell<Vec<AbilityLogItem>> = const { RefCell::new(Vec::new()) };
    }

    fn with_buffer<R>(f: impl FnOnce(&mut Vec<AbilityLogItem>) -> R) -> R {
        VERDICT_BUFFER.with(|buf| f(&mut buf.borrow_mut()))
    }

    /// Whether verdicts are actually being recorded.
    ///
    /// `buffer_len` is NOT a substitute for this: it returns 0 when debug is
    /// off, so a caller comparing it against a `before` snapshot also taken
    /// with debug off gets `0 <= 0` — true — and concludes it must record a
    /// verdict that `push_verdict` then discards. Gate on this instead.
    #[inline]
    pub fn debug_enabled() -> bool {
        ABILITY_DEBUG.load(Ordering::Relaxed)
    }

    pub fn push_verdict(item: AbilityLogItem) {
        if !ABILITY_DEBUG.load(Ordering::Relaxed) {
            return;
        }
        with_buffer(|buf| buf.push(item));
    }

    pub fn drain_verdicts() -> Vec<AbilityLogItem> {
        if !ABILITY_DEBUG.load(Ordering::Relaxed) {
            return vec![];
        }
        with_buffer(std::mem::take)
    }

    pub fn buffer_len() -> usize {
        if !ABILITY_DEBUG.load(Ordering::Relaxed) {
            return 0;
        }
        with_buffer(|buf| buf.len())
    }

    pub fn drain_verdicts_since(start_index: usize) -> Vec<AbilityLogItem> {
        if !ABILITY_DEBUG.load(Ordering::Relaxed) {
            return vec![];
        }
        with_buffer(|buf| {
            if start_index < buf.len() {
                buf.drain(start_index..).collect()
            } else {
                vec![]
            }
        })
    }

    pub fn clear_verdicts() {
        if !ABILITY_DEBUG.load(Ordering::Relaxed) {
            return;
        }
        with_buffer(|buf| buf.clear());
    }
}

#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
pub use inner::*;

use super::resolver::AbilityResolver;
use crate::game_state::GameState;

impl AbilityResolver {
    /// Log that an activated ability dispatched an action of `label`, as
    /// "P1 <card name>: <label>". Shared by the DiscardCard/MoveCards/Select
    /// dispatch arms in effects/mod.rs.
    pub(crate) fn rule_log_activated(&self, gs: &mut GameState, label: &str) {
        // Gated HERE, not by the caller: `push_rule_log_fmt` is itself gated, but
        // the two `String` locals below (one of which does a card-database lookup)
        // are evaluated before that call, so on a training/headless run they were
        // two allocations per logged action for a line nobody reads.
        if !crate::core::runtime_flags::logging_enabled() {
            return;
        }
        let pp = self.player_prefix(gs);
        let cn = gs
            .activating_card
            .and_then(|id| gs.card_database.get_card(id))
            .map(|c| c.name.to_string())
            .unwrap_or_default();
        gs.push_rule_log_fmt(format_args!("{} {}: {}", pp, cn, label));
    }
}

/// Push a condition verdict to the structured log buffer.
///
/// The `debug_enabled()` gate lives HERE, not in the callers, because this
/// function builds 3-5 `String`s (`describe_condition_expectation` is
/// `format!`-heavy) that `push_verdict` discards when debug is off. Gating here
/// makes it impossible for a call site to forget — which is how this cost
/// survived in four places after the equivalent gate was added to
/// `describe_condition_actual`.
#[cfg(not(feature = "no_std"))]
pub fn push_cond_verdict(
    condition: &crate::card::Condition,
    extra_actual: &str,
    passed: bool,
    children: Vec<AbilityLogItem>,
) {
    if !debug_enabled() {
        return;
    }
    let condition_type = match condition {
        crate::card::Condition::Appearance { .. } => "appearance_condition",
        crate::card::Condition::Comparison { .. } => "comparison_condition",
        crate::card::Condition::Location { .. } => "card_count_condition",
        crate::card::Condition::Movement { .. } => "movement_condition",
        crate::card::Condition::Group { .. } => "group_condition",
        crate::card::Condition::PositionCond { .. } => "position_condition",
        crate::card::Condition::Resource { .. } => "resource_condition",
        crate::card::Condition::ScoreThreshold { .. } => "score_threshold_condition",
        crate::card::Condition::State { .. } => "state_condition",
        crate::card::Condition::Temporal { .. } => "temporal_condition",
        crate::card::Condition::AbilityFilter { .. } => "ability_filter_condition",
        crate::card::Condition::Choice { .. } => "choice_condition",
        crate::card::Condition::Complex { .. } => "complex_condition",
        crate::card::Condition::OpponentChoice { .. } => "opponent_choice_condition",
        crate::card::Condition::OpponentLiveSuccess { .. } => "opponent_live_success",
        crate::card::Condition::NoExcessHeart { .. } => "no_excess_heart",
        crate::card::Condition::Compound { .. } => "compound",
        crate::card::Condition::AnyOf { .. } => "any_of_condition",
        crate::card::Condition::AlwaysTrue { .. } => "otherwise_condition",
        crate::card::Condition::AllRevealedMatchHeartColor { .. } => {
            "all_revealed_match_heart_color"
        }
        crate::card::Condition::Unsupported { .. } => "custom",
    };

    // Rendered ONCE. It used to be rendered three times per verdict (once for
    // `expectation`, once for `actual` when no override was supplied, and once
    // for `text`); `text` and `expectation` are the same string here.
    let expectation = crate::ability::describe::condition::describe_condition_expectation(condition);
    let actual = if !extra_actual.is_empty() {
        extra_actual.to_string()
    } else {
        expectation.clone()
    };

    push_verdict(AbilityLogItem::Condition {
        text: expectation.clone(),
        condition_type: condition_type.to_string(),
        expectation,
        actual,
        passed,
        children,
    });
}