use rabuka_engine::ability::debug::AbDebug;
use rabuka_engine::ability::types::AbilityTraceNode;

use super::TestGame;

#[allow(dead_code)]
pub fn count_debug_trace(game: &TestGame, needle: &str) -> usize {
    game.state
        .debug_trace
        .iter()
        .filter(|e| e.contains(needle))
        .count()
}

pub fn ability_verdicts(game: &mut TestGame, player: &str) -> String {
    use rabuka_engine::ability::debug::set_debug;
    use rabuka_engine::ability::log::{clear_verdicts, drain_verdicts, AbilityLogItem};

    fn fmt(items: &[AbilityLogItem], indent: usize) -> String {
        let mut out = String::new();
        for it in items {
            match it {
                AbilityLogItem::Condition {
                    condition_type,
                    expectation,
                    actual,
                    passed,
                    children,
                    text,
                } => {
                    out.push_str(&"  ".repeat(indent));
                    out.push_str(&format!(
                        "[{}] {} => {} ({} / {}?)\n",
                        if *passed { "PASS" } else { "FAIL" },
                        condition_type,
                        actual,
                        expectation,
                        text
                    ));
                    out.push_str(&fmt(children, indent + 1));
                }
                AbilityLogItem::Cost {
                    text,
                    expectation,
                    actual,
                    passed,
                    optional,
                } => {
                    out.push_str(&"  ".repeat(indent));
                    out.push_str(&format!(
                        "[{}] cost '{}' {} vs {} (optional={})\n",
                        if *passed { "PASS" } else { "FAIL" },
                        text,
                        actual,
                        expectation,
                        optional
                    ));
                }
                AbilityLogItem::Effect {
                    action, details, ..
                } => {
                    out.push_str(&"  ".repeat(indent));
                    out.push_str(&format!("[EFFECT] {} — {}\n", action, details));
                }
            }
        }
        out
    }

    set_debug(true);
    clear_verdicts();
    let pid = game
        .state
        .try_player_by_id(player)
        .map(|p| p.id.clone())
        .unwrap_or_else(|| game.state.player2.id.clone());
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    let scan_verdicts = drain_verdicts();
    game.state.process_pending_auto_abilities(&pid);
    game.drain_auto_ability_choices();
    let mut all = scan_verdicts;
    all.extend(drain_verdicts());
    set_debug(false);
    fmt(&all, 0)
}

impl Drop for TestGame {
    fn drop(&mut self) {
        self.trace.flush();
        if self.debug_enabled {
            AbDebug::flush_to_rule_log(&mut self.state.rule_log);
        }
        if std::env::var("RABUKA_RULE_LOG").as_deref() == Ok("1")
            && !self.state.structured_log.is_empty()
        {
            let test_name = std::thread::current()
                .name()
                .unwrap_or("unknown")
                .to_string();
            eprintln!("\n=== STRUCTURED LOG: {} ===", test_name);
            for entry in &self.state.structured_log {
                let meta_str = entry
                    .metadata
                    .as_ref()
                    .map(|m| serde_json::to_string(m).unwrap_or_default())
                    .unwrap_or_else(|| "null".to_string());
                eprintln!("[{}] {}", entry.category, entry.text);
                if meta_str != "null" {
                    eprintln!("  {}", meta_str);
                }
            }
            eprintln!("=== END STRUCTURED LOG ===\n");
        }
    }
}

#[allow(dead_code)]
impl TestGame {
    pub fn dump_structured_log(&self) {
        self.dump_structured_entries(None);
    }

    pub fn dump_structured_log_category(&self, category: &str) {
        self.dump_structured_entries(Some(category));
    }

    fn dump_structured_entries(&self, category: Option<&str>) {
        for entry in &self.state.structured_log {
            if category.is_some_and(|category| entry.category != category) {
                continue;
            }
            let meta_str = entry
                .metadata
                .as_ref()
                .map(|m| serde_json::to_string_pretty(m).unwrap_or_default())
                .unwrap_or_else(|| "null".to_string());
            eprintln!(
                "[LOG cat={} turn={} player={} card={:?} name={:?}] {}",
                entry.category,
                entry.turn,
                entry.player_label,
                entry.source_card_id,
                entry.source_card_name,
                entry.text
            );
            if meta_str != "null" {
                eprintln!("  metadata: {}", meta_str);
            }
        }
    }

    pub fn dump_rule_log(&self) {
        for line in &self.state.rule_log {
            eprintln!("[RULE_LOG] {}", line);
        }
    }

    pub fn dump_queue(&self) {
        let state_str = self.state.ability_queue.dump_state();
        eprintln!("[QUEUE_DUMP]\n{}", state_str);
    }

    pub fn name(&self, id: i16) -> String {
        self.state
            .card_database
            .get_card(id)
            .map(|c| format!("{} ({})", c.name, c.card_no))
            .unwrap_or_else(|| format!("#{}", id))
    }

    pub fn dbg_hand(&self) {
        self.dbg_zone("[HAND]", &self.state.player1.hand.cards, false);
    }

    pub fn dbg_discard(&self) {
        self.dbg_zone("[DISCARD]", &self.state.player1.waitroom.cards, false);
    }

    pub fn dbg_stage(&self) {
        let stage: Vec<i16> = self.state.player1.stage.stage.to_vec();
        self.dbg_zone("[STAGE]", &stage, true);
    }

    /// ONE zone printer for the `dbg_*` family: names every card id,
    /// rendering empty stage slots as "empty".
    fn dbg_zone(&self, label: &str, cards: &[i16], empty_slots: bool) {
        let named: Vec<String> = cards
            .iter()
            .map(|&id| {
                if empty_slots && id == -1 {
                    "empty".into()
                } else {
                    self.name(id)
                }
            })
            .collect();
        eprintln!("{} {:?}", label, named);
    }

    pub fn event_trace_contains(&self, needle: &str) -> bool {
        self.state.debug_trace_contains(needle)
    }

    pub fn dbg_events(&self, n: usize) {
        if self.state.debug_trace.is_empty() {
            eprintln!("[EVT] (trace empty)");
            return;
        }
        let skip = self.state.debug_trace.len().saturating_sub(n);
        for e in &self.state.debug_trace[skip..] {
            eprintln!("[EVT] {}", e);
        }
    }

    pub fn dbg_state(&self) {
        self.dbg_events(25);
        self.dbg_all();
    }

    pub fn dbg_story(&self) {
        eprintln!("──── story ────");
        eprintln!("── 1. player log (rule_log):");
        for line in &self.state.rule_log {
            eprintln!("  {}", line);
        }
        eprintln!("── 2. ability evaluations:");
        for entry in &self.state.structured_log {
            match entry.category.as_str() {
                "trigger_evaluation" | "ability_resolution" => {
                    eprintln!("  [{}] {}", entry.category, entry.text);
                }
                _ => {}
            }
        }
        eprintln!("── 3. events:");
        for e in &self.state.debug_trace {
            eprintln!("  {}", e);
        }
        eprintln!("── 4. board:");
        self.dbg_all();
        eprintln!("  choice: {}", self.pending_choice_summary());
    }

    pub fn dbg_all(&self) {
        self.dbg_hand();
        self.dbg_discard();
        self.dbg_stage();
        self.dbg_choice();
    }

    pub fn print_trace(&self) {
        if let Some(ref trace) = self.state.last_ability_trace {
            Self::print_trace_node(trace, 0);
        } else {
            eprintln!("[TRACE] no trace available (no ability resolved yet)");
        }
    }

    fn trace_node_label(node: &AbilityTraceNode, depth: usize) -> String {
        let pad = "  ".repeat(depth);
        let card_info = node
            .card
            .as_deref()
            .map(|c| format!(" [{}]", c))
            .unwrap_or_default();
        format!("{}{}▸ {}{}", pad, depth, node.label, card_info)
    }

    fn print_trace_node(node: &AbilityTraceNode, depth: usize) {
        let pad = "  ".repeat(depth);
        eprintln!("{}", Self::trace_node_label(node, depth));
        if let Some(ref before) = node.before {
            eprintln!(
                "{}  before: hand={} stage={} waitroom={} energy={} active_energy={} deck={}",
                pad,
                before.hand_count,
                before.stage_count,
                before.waitroom_count,
                before.energy_count,
                before.active_energy_count,
                before.deck_count,
            );
        }
        if let Some(ref after) = node.after {
            eprintln!(
                "{}  after:  hand={} stage={} waitroom={} energy={} active_energy={} deck={}",
                pad,
                after.hand_count,
                after.stage_count,
                after.waitroom_count,
                after.energy_count,
                after.active_energy_count,
                after.deck_count,
            );
        }
        for child in &node.children {
            Self::print_trace_node(child, depth + 1);
        }
    }

    pub fn assert_trace_contains(&self, pattern: &str, msg: &str) {
        let trace = self
            .state
            .last_ability_trace
            .as_ref()
            .expect("assert_trace_contains: no trace available");
        let found = Self::trace_node_matches(trace, pattern);
        assert!(
            found,
            "{}: expected trace to contain '{}', but no node matched.\n{}",
            msg,
            pattern,
            self.format_trace_string(),
        );
    }

    pub fn assert_trace_not_contains(&self, pattern: &str, msg: &str) {
        if let Some(ref trace) = self.state.last_ability_trace {
            let found = Self::trace_node_matches(trace, pattern);
            assert!(
                !found,
                "{}: expected trace NOT to contain '{}', but a node matched.\n{}",
                msg,
                pattern,
                self.format_trace_string(),
            );
        }
    }

    fn trace_node_matches(node: &AbilityTraceNode, pattern: &str) -> bool {
        node.label.contains(pattern)
            || node.card.as_deref().is_some_and(|c| c.contains(pattern))
            || node
                .children
                .iter()
                .any(|c| Self::trace_node_matches(c, pattern))
    }

    fn format_trace_string(&self) -> String {
        let mut buf = String::new();
        if let Some(ref trace) = self.state.last_ability_trace {
            Self::format_trace_node(trace, 0, &mut buf);
        } else {
            buf.push_str("  (no trace)\n");
        }
        buf
    }

    fn format_trace_node(node: &AbilityTraceNode, depth: usize, buf: &mut String) {
        buf.push_str(&format!("{}\n", Self::trace_node_label(node, depth)));
        for child in &node.children {
            Self::format_trace_node(child, depth, buf);
        }
    }
}
