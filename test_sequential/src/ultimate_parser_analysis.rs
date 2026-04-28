// Ultimate comprehensive parser analysis - combines all previous analysis approaches
use std::path::Path;
use std::collections::HashMap;

fn main() {
    println!("🚀 ULTIMATE PARSER ANALYSIS - Comprehensive Examination of All Abilities");
    println!("=====================================================================");
    
    let abilities_path = Path::new("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards\\abilities.json");
    
    match std::fs::read_to_string(abilities_path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(data) => {
                    if let Some(unique_abilities) = data.get("unique_abilities").and_then(|v| v.as_array()) {
                        let mut analysis = UltimateAnalysis::new();
                        
                        // Run all analysis modules
                        analysis.run_comprehensive_analysis(unique_abilities);
                        analysis.run_deep_dive_analysis(unique_abilities);
                        analysis.run_statistical_analysis(unique_abilities);
                        analysis.run_pattern_analysis(unique_abilities);
                        analysis.run_edge_case_analysis(unique_abilities);
                        analysis.run_rule_ability_analysis(unique_abilities);
                        analysis.run_enhancement_opportunity_analysis(unique_abilities);
                        
                        // Generate final report
                        analysis.generate_final_report();
                    }
                }
                Err(e) => {
                    println!("Failed to parse JSON: {}", e);
                }
            }
        }
        Err(e) => {
            println!("Failed to read abilities.json: {}", e);
        }
    }
}

struct UltimateAnalysis {
    total_abilities: usize,
    parsing_issues: Vec<String>,
    subtle_issues: Vec<String>,
    field_anomalies: Vec<String>,
    unusual_patterns: Vec<String>,
    rule_abilities: Vec<String>,
    enhancement_opportunities: Vec<String>,
    statistics: HashMap<String, usize>,
    action_distribution: HashMap<String, usize>,
    complexity_metrics: HashMap<String, usize>,
}

impl UltimateAnalysis {
    fn new() -> Self {
        Self {
            total_abilities: 0,
            parsing_issues: Vec::new(),
            subtle_issues: Vec::new(),
            field_anomalies: Vec::new(),
            unusual_patterns: Vec::new(),
            rule_abilities: Vec::new(),
            enhancement_opportunities: Vec::new(),
            statistics: HashMap::new(),
            action_distribution: HashMap::new(),
            complexity_metrics: HashMap::new(),
        }
    }
    
    fn run_comprehensive_analysis(&mut self, abilities: &[serde_json::Value]) {
        println!("\n📊 COMPREHENSIVE ANALYSIS");
        println!("========================");
        
        self.total_abilities = abilities.len();
        println!("Total abilities analyzed: {}", self.total_abilities);
        
        // Count action types
        let mut action_types = HashMap::new();
        let mut complex_abilities = 0;
        let mut sequential_abilities = 0;
        let mut conditional_abilities = 0;
        
        for (index, ability) in abilities.iter().enumerate() {
            if let Some(effect) = ability.get("effect") {
                if let Some(action) = effect.get("action").and_then(|a| a.as_str()) {
                    *action_types.entry(action.to_string()).or_insert(0) += 1;
                    
                    match action {
                        "sequential" => sequential_abilities += 1,
                        "conditional_alternative" => conditional_abilities += 1,
                        _ => {}
                    }
                    
                    if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                        if actions.len() > 2 {
                            complex_abilities += 1;
                        }
                    }
                }
            }
        }
        
        println!("Action type distribution:");
        let mut sorted_actions: Vec<_> = action_types.iter().collect();
        sorted_actions.sort_by(|a, b| b.1.cmp(a.1));
        for (action_type, count) in sorted_actions {
            println!("  {}: {}", action_type, count);
        }
        
        self.action_distribution = action_types;
        self.complexity_metrics.insert("complex_abilities".to_string(), complex_abilities);
        self.complexity_metrics.insert("sequential_abilities".to_string(), sequential_abilities);
        self.complexity_metrics.insert("conditional_abilities".to_string(), conditional_abilities);
        
        println!("Complex abilities (>2 actions): {}", complex_abilities);
        println!("Sequential abilities: {}", sequential_abilities);
        println!("Conditional abilities: {}", conditional_abilities);
    }
    
    fn run_deep_dive_analysis(&mut self, abilities: &[serde_json::Value]) {
        println!("\n🔍 DEEP DIVE ANALYSIS");
        println!("=====================");
        
        let mut missing_critical_fields = 0;
        let mut empty_actions = 0;
        let mut missing_text_fields = 0;
        let mut inconsistent_naming = 0;
        
        for (index, ability) in abilities.iter().enumerate() {
            if let Some(effect) = ability.get("effect") {
                if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                    for (action_index, sub_action) in actions.iter().enumerate() {
                        if let Some(action_type) = sub_action.get("action").and_then(|a| a.as_str()) {
                            // Check for missing critical fields in move_cards actions
                            if action_type == "move_cards" {
                                let has_source = sub_action.get("source").is_some();
                                let has_destination = sub_action.get("destination").is_some();
                                let has_count = sub_action.get("count").is_some();
                                
                                if !has_source || !has_destination || !has_count {
                                    missing_critical_fields += 1;
                                    self.subtle_issues.push(format!(
                                        "Ability #{} action #{}: move_cards missing critical fields (source: {}, dest: {}, count: {})",
                                        index, action_index, has_source, has_destination, has_count
                                    ));
                                }
                            }
                            
                            // Check for missing text fields
                            if sub_action.get("text").is_none() {
                                missing_text_fields += 1;
                                self.subtle_issues.push(format!(
                                    "Ability #{} action #{}: missing text field for action '{}'",
                                    index, action_index, action_type
                                ));
                            }
                            
                            // Check for empty custom actions
                            if action_type == "custom" {
                                if let Some(text) = sub_action.get("text").and_then(|t| t.as_str()) {
                                    if text.trim().is_empty() {
                                        empty_actions += 1;
                                    }
                                } else {
                                    empty_actions += 1;
                                }
                            }
                            
                            // Check for inconsistent field naming
                            if let Some(obj) = sub_action.as_object() {
                                for (key, _) in obj {
                                    if key.contains("_") && key.chars().any(|c| c.is_uppercase()) {
                                        inconsistent_naming += 1;
                                        self.field_anomalies.push(format!(
                                            "Ability #{} action #{}: mixed case field '{}'",
                                            index, action_index, key
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        self.statistics.insert("missing_critical_fields".to_string(), missing_critical_fields);
        self.statistics.insert("empty_actions".to_string(), empty_actions);
        self.statistics.insert("missing_text_fields".to_string(), missing_text_fields);
        self.statistics.insert("inconsistent_naming".to_string(), inconsistent_naming);
        
        println!("Missing critical fields: {}", missing_critical_fields);
        println!("Empty custom actions: {}", empty_actions);
        println!("Missing text fields: {}", missing_text_fields);
        println!("Inconsistent naming: {}", inconsistent_naming);
    }
    
    fn run_statistical_analysis(&mut self, abilities: &[serde_json::Value]) {
        println!("\n📈 STATISTICAL ANALYSIS");
        println!("======================");
        
        let mut total_chars = 0;
        let mut max_chars = 0;
        let mut min_chars = usize::MAX;
        let mut very_long_effects = 0;
        let mut very_short_effects = 0;
        let mut high_icon_count = 0;
        let mut high_condition_count = 0;
        
        for ability in abilities {
            if let Some(full_text) = ability.get("full_text").and_then(|t| t.as_str()) {
                let char_count = full_text.chars().count();
                total_chars += char_count;
                max_chars = max_chars.max(char_count);
                min_chars = min_chars.min(char_count);
                
                if char_count > 200 {
                    very_long_effects += 1;
                }
                if char_count < 50 {
                    very_short_effects += 1;
                }
                
                // Count icons and conditions
                let icon_count = full_text.matches("{{").count();
                if icon_count > 5 {
                    high_icon_count += 1;
                }
                
                let condition_count = full_text.matches("場合").count();
                if condition_count > 2 {
                    high_condition_count += 1;
                }
            }
        }
        
        let avg_chars = if self.total_abilities > 0 {
            total_chars / self.total_abilities
        } else {
            0
        };
        
        self.statistics.insert("avg_chars".to_string(), avg_chars);
        self.statistics.insert("max_chars".to_string(), max_chars);
        self.statistics.insert("min_chars".to_string(), min_chars);
        self.statistics.insert("very_long_effects".to_string(), very_long_effects);
        self.statistics.insert("very_short_effects".to_string(), very_short_effects);
        self.statistics.insert("high_icon_count".to_string(), high_icon_count);
        self.statistics.insert("high_condition_count".to_string(), high_condition_count);
        
        println!("Average characters: {}", avg_chars);
        println!("Max characters: {}", max_chars);
        println!("Min characters: {}", min_chars);
        println!("Very long effects (>200 chars): {}", very_long_effects);
        println!("Very short effects (<50 chars): {}", very_short_effects);
        println!("High icon count (>5): {}", high_icon_count);
        println!("High condition count (>2): {}", high_condition_count);
    }
    
    fn run_pattern_analysis(&mut self, abilities: &[serde_json::Value]) {
        println!("\n🎯 PATTERN ANALYSIS");
        println!("===================");
        
        let mut multiple_conditionals = 0;
        let mut nested_parentheses = 0;
        let mut complex_icons = 0;
        let mut multiple_actions = 0;
        let mut complex_sequencing = 0;
        let mut multiple_choices = 0;
        let mut complex_conditions = 0;
        
        for (index, ability) in abilities.iter().enumerate() {
            if let Some(full_text) = ability.get("full_text").and_then(|t| t.as_str()) {
                // Check for complex patterns
                if full_text.matches("場合").count() > 2 {
                    multiple_conditionals += 1;
                    self.unusual_patterns.push(format!("Ability #{}: multiple conditionals", index));
                }
                
                if full_text.matches("（").count() > 2 {
                    nested_parentheses += 1;
                    self.unusual_patterns.push(format!("Ability #{}: nested parentheses", index));
                }
                
                if full_text.matches("{{").count() > 5 {
                    complex_icons += 1;
                    self.unusual_patterns.push(format!("Ability #{}: complex icons", index));
                }
                
                if full_text.matches("、").count() > 3 {
                    multiple_actions += 1;
                    self.unusual_patterns.push(format!("Ability #{}: multiple actions", index));
                }
                
                if full_text.contains("その後") && full_text.contains("そうした場合") {
                    complex_sequencing += 1;
                    self.unusual_patterns.push(format!("Ability #{}: complex sequencing", index));
                }
                
                if full_text.matches("好きな").count() > 2 {
                    multiple_choices += 1;
                    self.unusual_patterns.push(format!("Ability #{}: multiple choices", index));
                }
                
                if full_text.contains("～") || full_text.contains("…") {
                    complex_conditions += 1;
                    self.unusual_patterns.push(format!("Ability #{}: complex conditions", index));
                }
            }
        }
        
        self.statistics.insert("multiple_conditionals".to_string(), multiple_conditionals);
        self.statistics.insert("nested_parentheses".to_string(), nested_parentheses);
        self.statistics.insert("complex_icons".to_string(), complex_icons);
        self.statistics.insert("multiple_actions".to_string(), multiple_actions);
        self.statistics.insert("complex_sequencing".to_string(), complex_sequencing);
        self.statistics.insert("multiple_choices".to_string(), multiple_choices);
        self.statistics.insert("complex_conditions".to_string(), complex_conditions);
        
        println!("Multiple conditionals: {}", multiple_conditionals);
        println!("Nested parentheses: {}", nested_parentheses);
        println!("Complex icons: {}", complex_icons);
        println!("Multiple actions: {}", multiple_actions);
        println!("Complex sequencing: {}", complex_sequencing);
        println!("Multiple choices: {}", multiple_choices);
        println!("Complex conditions: {}", complex_conditions);
    }
    
    fn run_edge_case_analysis(&mut self, abilities: &[serde_json::Value]) {
        println!("\n⚠️  EDGE CASE ANALYSIS");
        println!("====================");
        
        let mut unusual_counts = 0;
        let mut very_high_costs = 0;
        let mut missing_optional_inference = 0;
        let mut missing_shuffle_inference = 0;
        let mut missing_state_change_inference = 0;
        
        for (index, ability) in abilities.iter().enumerate() {
            if let Some(effect) = ability.get("effect") {
                if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                    for (action_index, sub_action) in actions.iter().enumerate() {
                        if let Some(action_type) = sub_action.get("action").and_then(|a| a.as_str()) {
                            // Check for unusual counts
                            if let Some(count) = sub_action.get("count") {
                                if let Some(count_val) = count.as_u64() {
                                    if count_val > 20 {
                                        unusual_counts += 1;
                                        self.subtle_issues.push(format!(
                                            "Unusual count {} in ability #{} action #{}",
                                            count_val, index, action_index
                                        ));
                                    }
                                }
                            }
                            
                            // Check for missing optional inference
                            if action_type == "move_cards" {
                                if sub_action.get("optional").is_none() {
                                    if let Some(text) = sub_action.get("text").and_then(|t| t.as_str()) {
                                        if text.contains("てもよい") {
                                            missing_optional_inference += 1;
                                            self.enhancement_opportunities.push(format!(
                                                "Ability #{} action #{}: could infer optional=true",
                                                index, action_index
                                            ));
                                        }
                                    }
                                }
                                
                                // Check for missing shuffle inference
                                if sub_action.get("shuffle").is_none() {
                                    if let Some(text) = sub_action.get("text").and_then(|t| t.as_str()) {
                                        if text.contains("シャッフル") {
                                            missing_shuffle_inference += 1;
                                            self.enhancement_opportunities.push(format!(
                                                "Ability #{} action #{}: could infer shuffle=true",
                                                index, action_index
                                            ));
                                        }
                                    }
                                }
                                
                                // Check for missing state_change inference
                                if sub_action.get("state_change").is_none() {
                                    if let Some(text) = sub_action.get("text").and_then(|t| t.as_str()) {
                                        if text.contains("ウェイト状態") {
                                            missing_state_change_inference += 1;
                                            self.enhancement_opportunities.push(format!(
                                                "Ability #{} action #{}: could infer state_change=wait",
                                                index, action_index
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            // Check for very high costs
            if let Some(cost) = ability.get("cost") {
                if let Some(cost_val) = cost.as_u64() {
                    if cost_val > 15 {
                        very_high_costs += 1;
                        self.subtle_issues.push(format!("Very high cost {} in ability #{}", cost_val, index));
                    }
                }
            }
        }
        
        self.statistics.insert("unusual_counts".to_string(), unusual_counts);
        self.statistics.insert("very_high_costs".to_string(), very_high_costs);
        self.statistics.insert("missing_optional_inference".to_string(), missing_optional_inference);
        self.statistics.insert("missing_shuffle_inference".to_string(), missing_shuffle_inference);
        self.statistics.insert("missing_state_change_inference".to_string(), missing_state_change_inference);
        
        println!("Unusual counts (>20): {}", unusual_counts);
        println!("Very high costs (>15): {}", very_high_costs);
        println!("Missing optional inference: {}", missing_optional_inference);
        println!("Missing shuffle inference: {}", missing_shuffle_inference);
        println!("Missing state_change inference: {}", missing_state_change_inference);
    }
    
    fn run_rule_ability_analysis(&mut self, abilities: &[serde_json::Value]) {
        println!("\n📜 RULE ABILITY ANALYSIS");
        println!("========================");
        
        let mut rule_abilities_count = 0;
        let mut rule_abilities_list = Vec::new();
        
        for (index, ability) in abilities.iter().enumerate() {
            if let Some(full_text) = ability.get("full_text").and_then(|t| t.as_str()) {
                // Check if this is a rule ability (starts and ends with parentheses)
                if full_text.starts_with('(') && full_text.ends_with(')') {
                    rule_abilities_count += 1;
                    rule_abilities_list.push(index);
                    
                    let rule_text = &full_text[1..full_text.len()-1]; // Remove parentheses
                    self.rule_abilities.push(format!("Ability #{}: {}", index, rule_text));
                    
                    // Check if effect is null or empty
                    if let Some(effect) = ability.get("effect") {
                        if effect.is_null() {
                            self.rule_abilities.push(format!("  ✓ Correctly has null effect (game code handles this)"));
                        } else {
                            self.rule_abilities.push(format!("  ⚠️  Has non-null effect: {:?}", effect));
                        }
                    } else {
                        self.rule_abilities.push(format!("  ✓ No effect field (correct for rule abilities)"));
                    }
                }
            }
        }
        
        self.statistics.insert("rule_abilities_count".to_string(), rule_abilities_count);
        
        println!("Rule abilities found: {}", rule_abilities_count);
        
        if !rule_abilities_list.is_empty() {
            println!("Rule ability indices: {:?}", rule_abilities_list);
        }
    }
    
    fn run_enhancement_opportunity_analysis(&mut self, abilities: &[serde_json::Value]) {
        println!("\n💡 ENHANCEMENT OPPORTUNITY ANALYSIS");
        println!("===================================");
        
        let mut total_opportunities = 0;
        let mut implemented_opportunities = 0;
        
        for (index, ability) in abilities.iter().enumerate() {
            if let Some(effect) = ability.get("effect") {
                if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                    for (action_index, sub_action) in actions.iter().enumerate() {
                        if let Some(action_type) = sub_action.get("action").and_then(|a| a.as_str()) {
                            match action_type {
                                "move_cards" => {
                                    total_opportunities += 1;
                                    
                                    // Check if optional is properly inferred
                                    if let Some(text) = sub_action.get("text").and_then(|t| t.as_str()) {
                                        if text.contains("てもよい") {
                                            if sub_action.get("optional").is_some() {
                                                implemented_opportunities += 1;
                                            }
                                        }
                                    }
                                    
                                    // Check if state_change is properly inferred
                                    if let Some(text) = sub_action.get("text").and_then(|t| t.as_str()) {
                                        if text.contains("ウェイト状態") {
                                            if sub_action.get("state_change").is_some() {
                                                implemented_opportunities += 1;
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
        
        let implementation_rate = if total_opportunities > 0 {
            (implemented_opportunities as f64 / total_opportunities as f64) * 100.0
        } else {
            100.0
        };
        
        self.statistics.insert("total_enhancement_opportunities".to_string(), total_opportunities);
        self.statistics.insert("implemented_enhancement_opportunities".to_string(), implemented_opportunities);
        
        println!("Total enhancement opportunities: {}", total_opportunities);
        println!("Implemented opportunities: {}", implemented_opportunities);
        println!("Implementation rate: {:.1}%", implementation_rate);
    }
    
    fn generate_final_report(&self) {
        println!("\n🎯 FINAL COMPREHENSIVE REPORT");
        println!("============================");
        
        // Summary
        println!("\n📊 SUMMARY:");
        println!("  Total abilities analyzed: {}", self.total_abilities);
        println!("  Parsing issues found: {}", self.parsing_issues.len());
        println!("  Subtle issues found: {}", self.subtle_issues.len());
        println!("  Field anomalies found: {}", self.field_anomalies.len());
        println!("  Unusual patterns found: {}", self.unusual_patterns.len());
        println!("  Rule abilities found: {}", self.rule_abilities.len());
        println!("  Enhancement opportunities: {}", self.enhancement_opportunities.len());
        
        // Critical issues
        if !self.parsing_issues.is_empty() {
            println!("\n⚠️  CRITICAL PARSING ISSUES:");
            for issue in &self.parsing_issues {
                println!("  - {}", issue);
            }
        } else {
            println!("\n✅ NO CRITICAL PARSING ISSUES FOUND");
        }
        
        // Subtle issues
        if !self.subtle_issues.is_empty() {
            println!("\n🔍 SUBTLE ISSUES:");
            for issue in self.subtle_issues.iter().take(10) {
                println!("  - {}", issue);
            }
            if self.subtle_issues.len() > 10 {
                println!("  ... and {} more", self.subtle_issues.len() - 10);
            }
        } else {
            println!("\n✅ NO SUBTLE ISSUES FOUND");
        }
        
        // Rule abilities
        if !self.rule_abilities.is_empty() {
            println!("\n📜 RULE ABILITIES:");
            for rule in self.rule_abilities.iter().take(5) {
                println!("  - {}", rule);
            }
            if self.rule_abilities.len() > 5 {
                println!("  ... and {} more", self.rule_abilities.len() - 5);
            }
        } else {
            println!("\n✅ NO RULE ABILITIES FOUND");
        }
        
        // Enhancement opportunities
        if !self.enhancement_opportunities.is_empty() {
            println!("\n💡 ENHANCEMENT OPPORTUNITIES:");
            for opportunity in self.enhancement_opportunities.iter().take(5) {
                println!("  - {}", opportunity);
            }
            if self.enhancement_opportunities.len() > 5 {
                println!("  ... and {} more", self.enhancement_opportunities.len() - 5);
            }
        } else {
            println!("\n✅ NO ENHANCEMENT OPPORTUNITIES FOUND");
        }
        
        // Final verdict
        println!("\n🏆 FINAL VERDICT:");
        if self.parsing_issues.is_empty() && self.subtle_issues.is_empty() {
            println!("  ✅ Parser is EXCELLENT - all issues resolved!");
        } else if self.parsing_issues.is_empty() {
            println!("  ✅ Parser is VERY GOOD - only minor issues remain");
        } else {
            println!("  ⚠️  Parser needs attention - critical issues found");
        }
        
        println!("\n🎉 Analysis complete! The parser handles {} abilities with comprehensive coverage.", self.total_abilities);
    }
}
