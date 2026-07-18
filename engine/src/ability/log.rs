use crate::ability::debug::ABILITY_DEBUG;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
pub enum AbilityLogItem {
    Condition {
        text: String,
        condition_type: String,
        expectation: String,
        actual: String,
        passed: bool,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
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
    KeyValue {
        key: String,
        value: String,
        passed: bool,
    },
}

#[cfg(feature = "no_std")]
mod inner {
    use super::AbilityLogItem;
    pub fn push_verdict(_item: AbilityLogItem) {}
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
    use std::sync::Mutex;

    static VERDICT_BUFFER: Mutex<Vec<AbilityLogItem>> = Mutex::new(Vec::new());

    pub fn push_verdict(item: AbilityLogItem) {
        if !ABILITY_DEBUG.load(Ordering::Relaxed) {
            return;
        }
        if let Ok(mut buf) = VERDICT_BUFFER.lock() {
            buf.push(item);
        }
    }

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

    pub fn clear_verdicts() {
        if !ABILITY_DEBUG.load(Ordering::Relaxed) {
            return;
        }
        if let Ok(mut buf) = VERDICT_BUFFER.lock() {
            buf.clear();
        }
    }
}

#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
pub use inner::*;
