/// Hierarchical debug output for ability evaluation.
/// Every line shows WHAT is being checked, WHAT the expected value is,
/// and WHAT the actual game state value is — all in one self-contained line.
use crate::card::{Ability, AbilityCost, AbilityEffect, Condition};
use std::sync::Mutex;

// Toggle: change to `false` to silence all ability debug output.
// Could also be a runtime flag or #[cfg(debug_assertions)] later.
const ABILITY_DEBUG: bool = true;

// Global buffer to collect debug logs between game state updates
static ABILITY_LOG_BUFFER: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub struct AbDebug {
    indent: usize,
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

    pub fn p(&mut self, tag: &str, msg: impl std::fmt::Display) {
        if !ABILITY_DEBUG { return; }
        let pad = "  ".repeat(self.indent);
        let log_entry = format!("[AB]{pad}{tag} {msg}");
        
        // Print to terminal for backward compatibility
        eprintln!("{}", log_entry);
        
        // Add to global buffer for later collection
        if let Ok(mut buffer) = ABILITY_LOG_BUFFER.lock() {
            buffer.push(log_entry);
        }
    }

    pub fn ability(&mut self, card_name: &str, card_id: &str, ability: &Ability) {
        self.p("ABILITY", format_args!("\"{}\" ({})", card_name, card_id));
        self.indent += 1;
        let trigger_str = ability.triggers.as_deref().unwrap_or("none");
        let limit_str = ability.use_limit.map(|l| format!("{}/turn", l)).unwrap_or_default();
        self.p("TRIGGER", format_args!("{} {}", trigger_str, limit_str));
        if !ability.full_text.is_empty() {
            self.p("TEXT", &ability.full_text);
        }
        if let Some(ref cost) = ability.cost {
            self.print_cost(cost, "");
        }
    }

    pub fn condition(&mut self, cond: &Condition, actual: u32, threshold: u32, passed: bool) {
        let ct = cond.condition_type.as_deref().unwrap_or("?");
        let loc = cond.location.as_deref().unwrap_or("");
        let tgt = cond.target.as_deref().unwrap_or("");
        let gn = cond.group_names.as_ref().map(|g| format!("{:?}", g)).unwrap_or_default();
        let op = cond.operator.as_deref().unwrap_or(">=");
        let pass = if passed { "PASS" } else { "FAIL" };
        let detail = match ct {
            "compound" => format!("{} sub-conditions", cond.conditions.as_ref().map(|c| c.len()).unwrap_or(0)),
            "card_count_condition" => {
                let mut parts = vec![format!("{}{} {}{}", op, threshold, cond.unit.as_deref().unwrap_or("x"), loc)];
                if !tgt.is_empty() { parts.push(tgt.to_string()); }
                if !gn.is_empty() { parts.push(format!("group={}", gn)); }
                parts.push(format!("actual={}", actual));
                parts.push(pass.to_string());
                parts.join(" ")
            }
            "location_condition" => {
                let extras = if cond.distinct.unwrap_or(false) { " distinct=names" } else { "" };
                format!("{}{} @{}.{}{} → actual={} {}",
                    op, threshold, loc, tgt, extras, actual, pass)
            }
            "comparison_condition" => {
                let rt = cond.resource_type.as_deref().unwrap_or("?");
                format!("{}{} {}{} → actual={} {}",
                    op, threshold, rt, loc, actual, pass)
            }
            "appearance_condition" => {
                let areas = if cond.all_areas.unwrap_or(false) { " all_areas" } else { "" };
                format!("check presence{}{} → {}", areas,
                    if cond.baton_touch_trigger.unwrap_or(false) { " baton_touch" } else { "" },
                    pass)
            }
            "movement_condition" => format!("movement={}{} → {}",
                cond.movement.as_deref().unwrap_or("?"), loc, pass),
            "or_condition" => format!("{} sub-conditions (any)", cond.conditions.as_ref().map(|c| c.len()).unwrap_or(0)),
            "otherwise_condition" => format!("else branch → {}", pass),
            _ => format!("{} ... {}", ct, pass),
        };
        self.p("COND", format_args!("{:20} {}", ct, detail));
    }

    pub fn cost_pay(&mut self, cost: &AbilityCost, ok: bool) {
        let ct = cost.cost_type.as_deref().unwrap_or("custom");
        let msg = match ct {
            "pay_energy" => format!("pay {} E{}", cost.energy.unwrap_or(1),
                if cost.optional.unwrap_or(false) { " (optional)" } else { "" }),
            "move_cards" => format!("move {} from {} to {} (src: {})",
                cost.count.unwrap_or(1),
                cost.source.as_deref().unwrap_or("?"),
                cost.destination.as_deref().unwrap_or("?"),
                trunc(&cost.text, 40)),
            "change_state" => format!("set {} → {}",
                cost.source.as_deref().unwrap_or("self"),
                cost.state_change.as_deref().unwrap_or("?")),
            "sequential_cost" => format!("compound ({} sub-costs)", cost.costs.as_ref().map(|c| c.len()).unwrap_or(0)),
            "reveal" => format!("reveal {} from {}", cost.count.unwrap_or(1), cost.source.as_deref().unwrap_or("?")),
            _ => format!("{}: {}", ct, trunc(&cost.text, 40)),
        };
        let status = if ok { "OK" } else { "FAIL" };
        self.p("COST", format_args!("{} → {}", msg, status));
    }

    pub fn effect(&mut self, effect: &AbilityEffect) {
        let a = &effect.action;
        let msg = match a.as_str() {
            "sequential" => format!("multi-step ({} sub-actions)", effect.compound.actions.as_ref().map(|v| v.len()).unwrap_or(0)),
            "draw_card" => format!("draw {} card(s){}",
                effect.count.unwrap_or(1),
                effect.optional.unwrap_or(false).then(|| " (optional)").unwrap_or("")),
            "move_cards" => format!("move {} from {} → {} (type: {})",
                effect.count.unwrap_or(1),
                effect.source.as_deref().unwrap_or("?"),
                effect.destination.as_deref().unwrap_or("?"),
                effect.card_type.as_deref().unwrap_or("card")),
            "gain_resource" => format!("gain {} {}{}",
                effect.count.unwrap_or(1),
                effect.resource.as_deref().unwrap_or("?"),
                effect.duration.as_deref().map(|d| format!(" for {}", d)).unwrap_or_default()),
            "modify_score" => format!("score {}{}",
                effect.operation.as_deref().unwrap_or("add"),
                effect.value.map(|v| format!(" {}", v)).unwrap_or_default()),
            "change_state" => format!("change state → {}",
                effect.state_change.as_deref().unwrap_or("?")),
            "select" => format!("select {} {} from {}{}",
                effect.count.unwrap_or(1),
                effect.card_type.as_deref().unwrap_or("card"),
                effect.source.as_deref().unwrap_or("?"),
                effect.optional.unwrap_or(false).then(|| " (optional)").unwrap_or("")),
            "look_and_select" => format!("look + select from deck"),
            "pay_energy" => format!("pay {} E{}",
                effect.count.unwrap_or(1),
                effect.optional.unwrap_or(false).then(|| " (optional)").unwrap_or("")),
            "reveal" => format!("reveal {}", effect.source.as_deref().unwrap_or("?")),
            "position_change" => format!("position change"),
            "gain_ability" => format!("gain ability"),
            "do_nothing" => String::new(),
            _ => format!("{}: {}", a, trunc(&effect.text, 50)),
        };
        if !msg.is_empty() {
            self.p("EFFECT", format_args!("{}", msg));
        }
        if let Some(ref cond) = effect.condition {
            self.p("COND", format_args!("(gated by: {})", trunc(&cond.text, 60)));
        }
    }

    pub fn print_cost(&mut self, cost: &AbilityCost, prefix: &str) {
        let ct = cost.cost_type.as_deref().unwrap_or("?");
        let msg = match ct {
            "pay_energy" => format!("pay {} E{}", cost.energy.unwrap_or(1),
                if cost.optional.unwrap_or(false) { " (optional)" } else { "" }),
            "move_cards" => format!("{} → {} ({} {})",
                cost.source.as_deref().unwrap_or("?"),
                cost.destination.as_deref().unwrap_or("?"),
                cost.count.unwrap_or(1),
                cost.card_type.as_deref().unwrap_or("card")),
            "change_state" => format!("set self → {}", cost.state_change.as_deref().unwrap_or("?")),
            "reveal" => format!("reveal {} {}", cost.count.unwrap_or(1), cost.card_type.as_deref().unwrap_or("card")),
            "sequential_cost" => format!("compound ({} sub-costs)", cost.costs.as_ref().map(|c| c.len()).unwrap_or(0)),
            "choice_condition" => format!("choice between costs"),
            _ => format!("{}", ct),
        };
        self.p("COST", format_args!("{}{}", prefix, msg));
    }
}

fn trunc(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max { s.to_string() }
    else { format!("{}...", chars[..max].iter().collect::<String>()) }
}
