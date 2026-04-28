// DEEP DIVE: Comprehensive parser analysis looking for subtle issues
use std::path::Path;
use std::collections::{HashMap, HashSet};

fn main() {
    println!("🔬 DEEP DIVE ANALYSIS - Looking for subtle parser issues");
    println!("===========================================================");
    
    let abilities_path = Path::new("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards\\abilities.json");
    
    match std::fs::read_to_string(abilities_path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(data) => {
                    if let Some(unique_abilities) = data.get("unique_abilities").and_then(|v| v.as_array()) {
                        let mut analysis = DeepDiveAnalysis::new();
                        
                        // Run all deep dive checks
                        analysis.check_field_type_consistency(unique_abilities);
                        analysis.check_required_fields(unique_abilities);
                        analysis.check_engine_compatibility(unique_abilities);
                        analysis.check_logical_consistency(unique_abilities);
                        analysis.check_edge_cases(unique_abilities);
                        analysis.check_naming_conventions(unique_abilities);
                        analysis.check_missing_inferences(unique_abilities);
                        analysis.check_runtime_safety(unique_abilities);
                        
                        // Generate report
                        analysis.generate_report();
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

struct DeepDiveAnalysis {
    issues: Vec<Issue>,
    warnings: Vec<String>,
    stats: HashMap<String, usize>,
}

#[derive(Debug)]
struct Issue {
    ability_index: usize,
    severity: Severity,
    category: String,
    description: String,
    action_index: Option<usize>,
}

#[derive(Debug)]
enum Severity {
    Critical,    // Will cause engine crash or wrong behavior
    High,       // Likely to cause gameplay issues
    Medium,     // Potential issues in edge cases
    Low,        // Minor inconsistency
}

impl DeepDiveAnalysis {
    fn new() -> Self {
        Self {
            issues: Vec::new(),
            warnings: Vec::new(),
            stats: HashMap::new(),
        }
    }
    
    fn add_issue(&mut self, ability_index: usize, severity: Severity, category: &str, description: &str, action_index: Option<usize>) {
        self.issues.push(Issue {
            ability_index,
            severity,
            category: category.to_string(),
            description: description.to_string(),
            action_index,
        });
    }
    
    // Check 1: Field type consistency
    fn check_field_type_consistency(&mut self, abilities: &[serde_json::Value]) {
        println!("\n🔍 CHECK 1: Field Type Consistency");
        
        for (idx, ability) in abilities.iter().enumerate() {
            if let Some(effect) = ability.get("effect") {
                if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                    for (action_idx, action) in actions.iter().enumerate() {
                        // Check count field type
                        if let Some(count) = action.get("count") {
                            if count.is_string() {
                                self.add_issue(idx, Severity::Critical, "TypeMismatch", 
                                    &format!("count is string instead of number: {:?}", count), Some(action_idx));
                            }
                        }
                        
                        // Check that boolean fields are actually booleans
                        for field in &["optional", "max", "per_unit", "any_number"] {
                            if let Some(val) = action.get(field) {
                                if !val.is_boolean() && !val.is_null() {
                                    self.add_issue(idx, Severity::High, "TypeMismatch",
                                        &format!("{} should be boolean but is: {:?}", field, val), Some(action_idx));
                                }
                            }
                        }
                    }
                }
            }
        }
        println!("  Found {} type consistency issues", self.issues.iter().filter(|i| i.category == "TypeMismatch").count());
    }
    
    // Check 2: Required fields for each action type
    fn check_required_fields(&mut self, abilities: &[serde_json::Value]) {
        println!("\n🔍 CHECK 2: Required Fields by Action Type");
        
        for (idx, ability) in abilities.iter().enumerate() {
            if let Some(effect) = ability.get("effect") {
                if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                    for (action_idx, action) in actions.iter().enumerate() {
                        if let Some(action_type) = action.get("action").and_then(|a| a.as_str()) {
                            match action_type {
                                "move_cards" => {
                                    // Required: source, destination, count
                                    if action.get("source").is_none() {
                                        self.add_issue(idx, Severity::Critical, "MissingRequired",
                                            "move_cards missing required 'source' field", Some(action_idx));
                                    }
                                    if action.get("destination").is_none() {
                                        self.add_issue(idx, Severity::Critical, "MissingRequired",
                                            "move_cards missing required 'destination' field", Some(action_idx));
                                    }
                                    if action.get("count").is_none() {
                                        self.add_issue(idx, Severity::High, "MissingRequired",
                                            "move_cards missing 'count' field (defaults to 1 may not be intended)", Some(action_idx));
                                    }
                                    // card_type is highly recommended
                                    if action.get("card_type").is_none() {
                                        self.add_issue(idx, Severity::Medium, "MissingRecommended",
                                            "move_cards missing 'card_type' field (engine may not validate correctly)", Some(action_idx));
                                    }
                                }
                                "draw_card" => {
                                    // Should have source, destination, count
                                    if action.get("source").is_none() {
                                        self.add_issue(idx, Severity::Medium, "MissingRecommended",
                                            "draw_card missing 'source' field (defaults to 'deck' may not be correct)", Some(action_idx));
                                    }
                                    if action.get("destination").is_none() {
                                        self.add_issue(idx, Severity::Medium, "MissingRecommended",
                                            "draw_card missing 'destination' field (defaults to 'hand' may not be correct)", Some(action_idx));
                                    }
                                }
                                "gain_resource" => {
                                    // Required: resource
                                    if action.get("resource").is_none() {
                                        self.add_issue(idx, Severity::Critical, "MissingRequired",
                                            "gain_resource missing required 'resource' field", Some(action_idx));
                                    }
                                    // Should have count
                                    if action.get("count").is_none() {
                                        self.add_issue(idx, Severity::High, "MissingRequired",
                                            "gain_resource missing 'count' field", Some(action_idx));
                                    }
                                }
                                "look_and_select" => {
                                    // Required: look_action, select_action
                                    if action.get("look_action").is_none() {
                                        self.add_issue(idx, Severity::Critical, "MissingRequired",
                                            "look_and_select missing required 'look_action' field", Some(action_idx));
                                    }
                                    if action.get("select_action").is_none() {
                                        self.add_issue(idx, Severity::Critical, "MissingRequired",
                                            "look_and_select missing required 'select_action' field", Some(action_idx));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
        
        let required_count = self.issues.iter().filter(|i| i.category == "MissingRequired").count();
        let recommended_count = self.issues.iter().filter(|i| i.category == "MissingRecommended").count();
        println!("  Found {} missing required fields", required_count);
        println!("  Found {} missing recommended fields", recommended_count);
    }
    
    // Check 3: Engine compatibility
    fn check_engine_compatibility(&mut self, abilities: &[serde_json::Value]) {
        println!("\n🔍 CHECK 3: Engine Compatibility");
        
        // Known valid values from engine
        let valid_sources: HashSet<&str> = ["deck", "hand", "discard", "stage", "energy_zone", 
            "live_card_zone", "success_live_zone", "deck_top", "deck_bottom", "revealed", 
            "selected_cards", "cost_paid"].iter().cloned().collect();
        
        let valid_destinations: HashSet<&str> = ["deck", "hand", "discard", "stage", "energy_zone",
            "live_card_zone", "success_live_zone", "deck_top", "deck_bottom", "under_member"].iter().cloned().collect();
        
        let valid_resources: HashSet<&str> = ["blade", "heart", "energy", "heart01", "heart02", 
            "heart03", "heart04", "heart05", "heart06", "heart00", "b_all"].iter().cloned().collect();
        
        for (idx, ability) in abilities.iter().enumerate() {
            if let Some(effect) = ability.get("effect") {
                if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                    for (action_idx, action) in actions.iter().enumerate() {
                        // Check source values
                        if let Some(source) = action.get("source").and_then(|s| s.as_str()) {
                            if !valid_sources.contains(source) && !source.is_empty() {
                                self.add_issue(idx, Severity::Medium, "EngineCompat",
                                    &format!("Unknown source value '{}', engine may not handle it", source), Some(action_idx));
                            }
                        }
                        
                        // Check destination values
                        if let Some(dest) = action.get("destination").and_then(|d| d.as_str()) {
                            if !valid_destinations.contains(dest) && !dest.is_empty() {
                                self.add_issue(idx, Severity::Medium, "EngineCompat",
                                    &format!("Unknown destination value '{}', engine may not handle it", dest), Some(action_idx));
                            }
                        }
                        
                        // Check resource values
                        if let Some(resource) = action.get("resource").and_then(|r| r.as_str()) {
                            if !valid_resources.contains(resource) && resource != "generic" {
                                self.add_issue(idx, Severity::Medium, "EngineCompat",
                                    &format!("Unknown resource value '{}', engine may not handle it", resource), Some(action_idx));
                            }
                        }
                    }
                }
            }
        }
        
        println!("  Found {} engine compatibility issues", 
            self.issues.iter().filter(|i| i.category == "EngineCompat").count());
    }
    
    // Check 4: Logical consistency
    fn check_logical_consistency(&mut self, abilities: &[serde_json::Value]) {
        println!("\n🔍 CHECK 4: Logical Consistency");
        
        for (idx, ability) in abilities.iter().enumerate() {
            if let Some(effect) = ability.get("effect") {
                if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                    for (action_idx, action) in actions.iter().enumerate() {
                        // Check source != destination
                        if let (Some(src), Some(dst)) = (
                            action.get("source").and_then(|s| s.as_str()),
                            action.get("destination").and_then(|d| d.as_str())
                        ) {
                            if src == dst && !src.is_empty() {
                                self.add_issue(idx, Severity::High, "LogicError",
                                    &format!("Source and destination are both '{}' - this is a no-op", src), Some(action_idx));
                            }
                        }
                        
                        // Check count > 0
                        if let Some(count) = action.get("count").and_then(|c| c.as_u64()) {
                            if count == 0 {
                                self.add_issue(idx, Severity::High, "LogicError",
                                    "Count is 0 - this action will do nothing", Some(action_idx));
                            }
                        }
                        
                        // Check that shuffle makes sense
                        if let Some(shuffle) = action.get("shuffle").and_then(|s| s.as_bool()) {
                            if shuffle {
                                if let Some(dst) = action.get("destination").and_then(|d| d.as_str()) {
                                    if dst != "deck" && dst != "deck_top" && dst != "deck_bottom" {
                                        self.add_issue(idx, Severity::Medium, "LogicError",
                                            &format!("Shuffle=true but destination is '{}' - shuffling only makes sense for deck", dst), Some(action_idx));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        println!("  Found {} logical consistency issues",
            self.issues.iter().filter(|i| i.category == "LogicError").count());
    }
    
    // Check 5: Edge cases
    fn check_edge_cases(&mut self, abilities: &[serde_json::Value]) {
        println!("\n🔍 CHECK 5: Edge Cases");
        
        for (idx, ability) in abilities.iter().enumerate() {
            if let Some(effect) = ability.get("effect") {
                // Check for very long texts that might cause UI issues
                if let Some(text) = effect.get("text").and_then(|t| t.as_str()) {
                    if text.len() > 500 {
                        self.add_issue(idx, Severity::Low, "EdgeCase",
                            &format!("Effect text is very long ({} chars) - may cause UI display issues", text.len()), None);
                    }
                }
                
                if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                    // Check for too many actions
                    if actions.len() > 5 {
                        self.add_issue(idx, Severity::Medium, "EdgeCase",
                            &format!("Ability has {} actions - very complex, may cause UI/UX issues", actions.len()), None);
                    }
                    
                    for (action_idx, action) in actions.iter().enumerate() {
                        // Check for empty text
                        if let Some(text) = action.get("text").and_then(|t| t.as_str()) {
                            if text.trim().is_empty() {
                                self.add_issue(idx, Severity::Medium, "EdgeCase",
                                    "Action has empty text field", Some(action_idx));
                            }
                        } else {
                            self.add_issue(idx, Severity::Medium, "EdgeCase",
                                "Action is missing text field entirely", Some(action_idx));
                        }
                    }
                }
            }
        }
        
        println!("  Found {} edge case issues",
            self.issues.iter().filter(|i| i.category == "EdgeCase").count());
    }
    
    // Check 6: Naming conventions
    fn check_naming_conventions(&mut self, abilities: &[serde_json::Value]) {
        println!("\n🔍 CHECK 6: Naming Conventions");
        
        // Check for inconsistent field naming (snake_case vs camelCase)
        for (idx, ability) in abilities.iter().enumerate() {
            if let Some(effect) = ability.get("effect") {
                if let Some(obj) = effect.as_object() {
                    for key in obj.keys() {
                        // Check for camelCase (should be snake_case)
                        if key.chars().any(|c| c.is_uppercase()) && key.contains('_') {
                            self.add_issue(idx, Severity::Low, "Naming",
                                &format!("Field '{}' uses mixed naming (should be snake_case)", key), None);
                        }
                    }
                }
            }
        }
        
        println!("  Found {} naming convention issues",
            self.issues.iter().filter(|i| i.category == "Naming").count());
    }
    
    // Check 7: Missing inferences
    fn check_missing_inferences(&mut self, abilities: &[serde_json::Value]) {
        println!("\n🔍 CHECK 7: Missing Inferences");
        
        for (idx, ability) in abilities.iter().enumerate() {
            if let Some(effect) = ability.get("effect") {
                if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                    for (action_idx, action) in actions.iter().enumerate() {
                        if let Some(action_type) = action.get("action").and_then(|a| a.as_str()) {
                            if let Some(text) = action.get("text").and_then(|t| t.as_str()) {
                                match action_type {
                                    "move_cards" => {
                                        // Could infer shuffle
                                        if text.contains("シャッフル") && action.get("shuffle").is_none() {
                                            self.warnings.push(format!(
                                                "Ability #{} action {}: Could infer shuffle=true from text", idx, action_idx));
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
        
        println!("  Found {} potential inference improvements", self.warnings.len());
    }
    
    // Check 8: Runtime safety
    fn check_runtime_safety(&mut self, abilities: &[serde_json::Value]) {
        println!("\n🔍 CHECK 8: Runtime Safety");
        
        for (idx, ability) in abilities.iter().enumerate() {
            if let Some(effect) = ability.get("effect") {
                // Check for potential panic conditions
                if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                    for (action_idx, action) in actions.iter().enumerate() {
                        // Check for None/Null in critical fields
                        if let Some(count) = action.get("count") {
                            if count.is_null() {
                                self.add_issue(idx, Severity::Critical, "RuntimeSafety",
                                    "count is explicitly null - may cause unwrap() panic in engine", Some(action_idx));
                            }
                        }
                    }
                }
            }
        }
        
        println!("  Found {} runtime safety issues",
            self.issues.iter().filter(|i| i.category == "RuntimeSafety").count());
    }
    
    fn generate_report(&self) {
        println!("\n");
        println!("===========================================================");
        println!("📊 DEEP DIVE ANALYSIS REPORT");
        println!("===========================================================");
        
        // Categorize issues
        let critical: Vec<_> = self.issues.iter().filter(|i| matches!(i.severity, Severity::Critical)).collect();
        let high: Vec<_> = self.issues.iter().filter(|i| matches!(i.severity, Severity::High)).collect();
        let medium: Vec<_> = self.issues.iter().filter(|i| matches!(i.severity, Severity::Medium)).collect();
        let low: Vec<_> = self.issues.iter().filter(|i| matches!(i.severity, Severity::Low)).collect();
        
        println!("\n📈 SUMMARY:");
        println!("  Critical issues: {}", critical.len());
        println!("  High severity: {}", high.len());
        println!("  Medium severity: {}", medium.len());
        println!("  Low severity: {}", low.len());
        println!("  Total issues: {}", self.issues.len());
        println!("  Potential improvements: {}", self.warnings.len());
        
        if !critical.is_empty() {
            println!("\n🚨 CRITICAL ISSUES (Must Fix):");
            for issue in &critical[..critical.len().min(10)] {
                let action_str = issue.action_index.map(|i| format!(" action {}", i)).unwrap_or_default();
                println!("  - Ability #{}{}: [{}] {}", issue.ability_index, action_str, issue.category, issue.description);
            }
            if critical.len() > 10 {
                println!("  ... and {} more critical issues", critical.len() - 10);
            }
        }
        
        if !high.is_empty() {
            println!("\n⚠️  HIGH SEVERITY ISSUES:");
            for issue in &high[..high.len().min(10)] {
                let action_str = issue.action_index.map(|i| format!(" action {}", i)).unwrap_or_default();
                println!("  - Ability #{}{}: [{}] {}", issue.ability_index, action_str, issue.category, issue.description);
            }
            if high.len() > 10 {
                println!("  ... and {} more high severity issues", high.len() - 10);
            }
        }
        
        if !medium.is_empty() {
            println!("\n⚡ MEDIUM SEVERITY ISSUES:");
            for issue in &medium[..medium.len().min(5)] {
                let action_str = issue.action_index.map(|i| format!(" action {}", i)).unwrap_or_default();
                println!("  - Ability #{}{}: [{}] {}", issue.ability_index, action_str, issue.category, issue.description);
            }
            if medium.len() > 5 {
                println!("  ... and {} more medium severity issues", medium.len() - 5);
            }
        }
        
        if self.issues.is_empty() {
            println!("\n✅ NO ISSUES FOUND - Parser is perfect!");
        } else {
            println!("\n🔧 VERDICT: Parser needs attention");
            if !critical.is_empty() {
                println!("   Priority: Fix {} critical issues immediately", critical.len());
            } else if !high.is_empty() {
                println!("   Priority: Address {} high severity issues", high.len());
            } else {
                println!("   Priority: Minor cleanup of {} low/medium issues", medium.len() + low.len());
            }
        }
    }
}
