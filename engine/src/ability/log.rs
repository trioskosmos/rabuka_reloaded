use std::sync::atomic::Ordering;
use std::sync::Mutex;

use serde::Serialize;

use crate::ability::debug::ABILITY_DEBUG;

/// A structured, serializable log item produced during ability resolution.
/// Each variant captures what was checked, what was expected, and what was found.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
pub enum AbilityLogItem {
    Condition {
        text: String,
        #[serde(rename = "type")]
        condition_type: String,
        expectation: String,
        actual: String,
        passed: bool,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        children: Vec<AbilityLogItem>,
    },
    /// A cost payment step: e.g. "discard 1 card" found "hand had 3"
    Cost {
        text: String,
        expectation: String,
        actual: String,
        passed: bool,
        optional: bool,
    },
    /// An effect execution: e.g. "modify_required_hearts" → "heart00 -3"
    Effect {
        text: String,
        action: String,
        details: String,
    },
    /// A keyword/position check
    KeyValue {
        key: String,
        value: String,
        passed: bool,
    },
}

/// Global buffer accumulating structured log items during ability resolution.
/// Draining is scoped to a single `resolve_ability()` call — the resolver
/// drains it at the start and flushes the contents as metadata at the end.
static VERDICT_BUFFER: Mutex<Vec<AbilityLogItem>> = Mutex::new(Vec::new());

/// Push a structured log item into the global buffer.
/// No-op unless `RABUKA_DEBUG` is set.
pub fn push_verdict(item: AbilityLogItem) {
    if !ABILITY_DEBUG.load(Ordering::Relaxed) {
        return;
    }
    if let Ok(mut buf) = VERDICT_BUFFER.lock() {
        buf.push(item);
    }
}

/// Drain all buffered items, returning them in order.
/// Returns empty vec unless `RABUKA_DEBUG` is set.
pub fn drain_verdicts() -> Vec<AbilityLogItem> {
    if !ABILITY_DEBUG.load(Ordering::Relaxed) {
        return vec![];
    }
    if let Ok(mut buf) = VERDICT_BUFFER.lock() {
        buf.drain(..).collect()
    } else {
        vec![]
    }
}

/// Return the current number of buffered items (for snapshoting).
/// Returns 0 unless `RABUKA_DEBUG` is set.
pub fn buffer_len() -> usize {
    if !ABILITY_DEBUG.load(Ordering::Relaxed) {
        return 0;
    }
    if let Ok(buf) = VERDICT_BUFFER.lock() {
        buf.len()
    } else {
        0
    }
}

/// Drain items from `start_index` to the end of the buffer.
/// Used when building compound verdicts: snapshot before sub-evaluation,
/// evaluate sub-conditions, then collect verdicts added during evaluation.
/// Returns empty vec unless `RABUKA_DEBUG` is set.
pub fn drain_verdicts_since(start_index: usize) -> Vec<AbilityLogItem> {
    if !ABILITY_DEBUG.load(Ordering::Relaxed) {
        return vec![];
    }
    if let Ok(mut buf) = VERDICT_BUFFER.lock() {
        if start_index < buf.len() {
            buf.drain(start_index..).collect()
        } else {
            vec![]
        }
    } else {
        vec![]
    }
}

/// Clear all buffered items (called at the start of ability resolution).
/// No-op unless `RABUKA_DEBUG` is set.
pub fn clear_verdicts() {
    if !ABILITY_DEBUG.load(Ordering::Relaxed) {
        return;
    }
    if let Ok(mut buf) = VERDICT_BUFFER.lock() {
        buf.clear();
    }
}
