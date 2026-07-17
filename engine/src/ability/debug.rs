use core::sync::atomic::{AtomicBool, Ordering};

pub static ABILITY_DEBUG: AtomicBool = AtomicBool::new(false);

pub fn set_debug(enabled: bool) {
    ABILITY_DEBUG.store(enabled, Ordering::SeqCst);
}

#[cfg(feature = "psp")]
use alloc::{string::String, vec::Vec};
#[cfg(not(feature = "psp"))]
pub use inner::AbDebug;
#[cfg(feature = "psp")]
pub struct AbDebug;

#[cfg(feature = "psp")]
impl AbDebug {
    pub fn new() -> Self {
        AbDebug
    }
    pub fn flush_to_rule_log(_rule_log: &mut Vec<String>) {}
    pub fn flush_to_structured_log(_structured_log: &mut Vec<crate::types::LogEntry>, _turn: u32) {}
    pub fn p(&mut self, _tag: &str, _msg: impl core::fmt::Display) {}
    pub fn ability(
        &mut self,
        _card_name: &str,
        _card_no: &str,
        _card_id: &str,
        _ability: &crate::card::Ability,
    ) {
    }
    pub fn condition(
        &mut self,
        _cond: &crate::card::Condition,
        _actual: u32,
        _threshold: u32,
        _passed: bool,
    ) {
    }
    pub fn cost_pay(&mut self, _cost: &crate::card::AbilityEffect, _ok: bool) {}
    pub fn effect(&mut self, _effect: &crate::card::AbilityEffect) {}
    pub fn print_cost(&mut self, _cost: &crate::card::AbilityEffect, _prefix: &str) {}
}

#[cfg(not(feature = "psp"))]
mod inner {
    use crate::card::{Ability, AbilityEffect, Condition};
    use core::sync::atomic::Ordering;
    use std::sync::Mutex;

    static ABILITY_LOG_BUFFER: Mutex<Vec<String>> = Mutex::new(Vec::new());
    static COVERAGE_LOG: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());

    pub struct AbDebug {
        pub indent: usize,
    }

    impl AbDebug {
        pub fn new() -> Self {
            AbDebug { indent: 0 }
        }

        pub fn flush_to_rule_log(rule_log: &mut Vec<String>) {
            if let Ok(mut buffer) = ABILITY_LOG_BUFFER.lock() {
                rule_log.extend(buffer.drain(..));
            }
        }

        pub fn flush_to_structured_log(
            structured_log: &mut Vec<crate::types::LogEntry>,
            turn: u32,
        ) {
            if let Ok(mut buffer) = ABILITY_LOG_BUFFER.lock() {
                for line in buffer.drain(..) {
                    structured_log.push(crate::types::LogEntry {
                        text: line,
                        turn,
                        player_label: String::new(),
                        source_card_id: None,
                        source_card_name: None,
                        category: "debug".to_string(),
                        metadata: None,
                    });
                }
            }
        }

        pub fn p(&mut self, tag: &str, msg: impl core::fmt::Display) {
            if !super::ABILITY_DEBUG.load(Ordering::Relaxed) {
                return;
            }
            let pad = "  ".repeat(self.indent);
            let log_entry = format!("[AB]{pad}{tag} {msg}");
            if let Ok(mut buffer) = ABILITY_LOG_BUFFER.lock() {
                buffer.push(log_entry);
            }
        }

        pub fn ability(
            &mut self,
            card_name: &str,
            card_no: &str,
            card_id: &str,
            ability: &Ability,
        ) {
            self.p("ABILITY", format_args!("\"{}\" ({})", card_name, card_id));
            self.indent += 1;
            let trigger_str = ability.triggers.as_deref().unwrap_or("none");
            let limit_str = ability
                .use_limit
                .map(|l| format!("{}/turn", l))
                .unwrap_or_default();
            self.p("TRIGGER", format_args!("{} {}", trigger_str, limit_str));
            if !ability.full_text.is_empty() {
                self.p("TEXT", &ability.full_text);
            }
            if super::ABILITY_DEBUG.load(Ordering::Relaxed) && !ability.full_text.is_empty() {
                if let Ok(mut cov) = COVERAGE_LOG.lock() {
                    cov.push((card_no.to_string(), ability.full_text.clone()));
                }
            }
        }

        pub fn condition(&mut self, cond: &Condition, actual: u32, threshold: u32, passed: bool) {
            if !super::ABILITY_DEBUG.load(Ordering::Relaxed) {
                return;
            }
            let pass = if passed { "PASS" } else { "FAIL" };
            let ct = cond.condition_type().map(|c| c.to_str()).unwrap_or("?");
            self.p(
                "COND",
                format_args!("{ct} actual={actual} threshold={threshold} {pass}"),
            );
        }

        pub fn cost_pay(&mut self, cost: &AbilityEffect, ok: bool) {
            if !super::ABILITY_DEBUG.load(Ordering::Relaxed) {
                return;
            }
            let status = if ok { "OK" } else { "FAIL" };
            self.p("COST", format_args!("{} → {status}", cost.action));
        }

        pub fn effect(&mut self, effect: &AbilityEffect) {
            if !super::ABILITY_DEBUG.load(Ordering::Relaxed) {
                return;
            }
            self.p("EFFECT", format_args!("{}", effect.action));
        }

        pub fn print_cost(&mut self, cost: &AbilityEffect, _prefix: &str) {
            if !super::ABILITY_DEBUG.load(Ordering::Relaxed) {
                return;
            }
            self.p("COST", format_args!("{}", cost.action));
        }
    }
}
