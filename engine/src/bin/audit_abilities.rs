/// Audit abilities.json for known parser failure patterns.
///
/// Checks each unique ability entry against patterns in triggerless_text
/// that the parser may have mis-handled, comparing against the parsed effect.
use std::path::Path;

fn load_json(path: &str) -> serde_json::Value {
    let data = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", path, e));
    serde_json::from_str(&data)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", path, e))
}

fn main() {
    let cards_dir = "c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards";
    let ab_path = Path::new(cards_dir).join("abilities.json");
    let abilities = load_json(ab_path.to_str().unwrap());

    let entries = abilities["unique_abilities"]
        .as_array()
        .expect("unique_abilities should be an array");

    println!("Auditing {} unique ability entries...\n", entries.len());

    let mut total_issues = 0u32;

    for entry in entries {
        let text = entry["triggerless_text"].as_str().unwrap_or("");
        let effect_val = &entry["effect"];
        let cards_arr = entry["cards"].as_array();
        let cards_list: Vec<&str> = cards_arr
            .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();

        // Pattern 1: 能力を持たない / 能力を持っていない (does not have ability)
        if let Some(m) = check_ability_filter(text, effect_val) {
            print_issue("ability_filter MISSING", &cards_list, text, &m);
            total_issues += 1;
        }

        // Pattern 2: そうした場合 (conditional "if you do")
        if let Some(m) = check_conditional_followup(text, entry) {
            print_issue("conditional_followup MISSING", &cards_list, text, &m);
            total_issues += 1;
        }

        // Pattern 3: 控え室に置かれたとき (when placed to discard)
        if let Some(m) = check_movement_trigger(text, effect_val) {
            print_issue("static_zone_check vs movement_trigger", &cards_list, text, &m);
            total_issues += 1;
        }

        // Pattern 4: 合計がN以上 (total >= N) with aggregate
        if let Some(m) = check_aggregate_operator(text, effect_val) {
            print_issue("aggregate_operator WRONG", &cards_list, text, &m);
            total_issues += 1;
        }

        // Pattern 5: icon_all (all hearts icon) in gain_resource
        if let Some(m) = check_heart_type_all(text, effect_val) {
            print_issue("heart_type MISSING (icon_all)", &cards_list, text, &m);
            total_issues += 1;
        }

        // Pattern 6: 選び / 選んで (select/choose with filter)
        if let Some(m) = check_look_select_filter(text, effect_val) {
            print_issue("look_select_filter WRONG", &cards_list, text, &m);
            total_issues += 1;
        }

        // Pattern 7: のみで (only / exclusively)
        if let Some(m) = check_exclusivity(text, effect_val) {
            print_issue("exclusivity NOT ENFORCED", &cards_list, text, &m);
            total_issues += 1;
        }

        // Pattern 8: turn1 / ターン1回 with use_limit
        if let Some(m) = check_turn1_limit(text, entry) {
            print_issue("turn1_use_limit", &cards_list, text, &m);
            total_issues += 1;
        }
    }

    println!("\nTotal issues found: {}", total_issues);
    if total_issues == 0 {
        println!("All clean.");
    }
}

fn print_issue(pattern: &str, cards: &[&str], text: &str, detail: &str) {
    println!("[{}]", pattern);
    // Only show first 5 card references (it's the same ability on multiple rarities)
    for c in cards.iter().take(5) {
        println!("  Card: {}", c);
    }
    if cards.len() > 5 {
        println!("  ... and {} more", cards.len() - 5);
    }
    println!("  Detail: {}", detail);
    println!();
}

/// Pattern 1: Text says "能力を持たない" but effect has no ability_filter
fn truncate(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

fn check_ability_filter(text: &str, effect: &serde_json::Value) -> Option<String> {
    if !text.contains("能力を持たない") && !text.contains("能力を持っていない") {
        return None;
    }
    let mut missing = true;
    if let Some(sa) = effect.get("select_action") {
        if sa.get("ability_filter").is_some() {
            missing = false;
        }
    }
    if let Some(cond) = effect.get("condition") {
        if let Some(conds) = cond.get("conditions").and_then(|c| c.as_array()) {
            for sub in conds {
                if sub.get("ability_filter").is_some() {
                    missing = false;
                }
            }
        }
        if cond.get("ability_filter").is_some() {
            missing = false;
        }
    }
    if missing {
        Some(format!("Text: '{}' but no ability_filter found", truncate(text, 60)))
    } else {
        None
    }
}

/// Pattern 2: Text says "そうした場合" but no followup_action or conditional_action
fn check_conditional_followup(text: &str, entry: &serde_json::Value) -> Option<String> {
    if !text.contains("そうした場合") {
        return None;
    }
    let effect = &entry["effect"];
    let has_followup = effect.get("followup_action").is_some()
        || effect.get("conditional_action").is_some()
        || effect.get("alternative_condition").is_some();
    if !has_followup {
        Some(format!("Text: '...そうした場合...' but no followup_action / conditional_action"))
    } else {
        None
    }
}

/// Pattern 3: Text says "から控え室に置かれた" but condition uses static locations
fn check_movement_trigger(text: &str, effect: &serde_json::Value) -> Option<String> {
    if !text.contains("から控え室に置かれた") && !text.contains("から控え室に置いた") {
        return None;
    }
    if let Some(cond) = effect.get("condition") {
        let has_locations = cond.get("locations").and_then(|l| l.as_array())
            .map(|a| a.len() >= 2).unwrap_or(false);
        let has_source_moved = cond.get("source").and_then(|s| s.as_str())
            .map(|s| s == "preceding_moved").unwrap_or(false);
        if has_locations && !has_source_moved {
            return Some(format!("Condition has static locations ({:?}), not source:preceding_moved",
                cond.get("locations")));
        }
    }
    if let Some(src) = effect.get("source").and_then(|s| s.as_str()) {
        if src != "recently_moved" && !text.contains("コスト") {
            return Some(format!("Effect source is '{}', should be 'recently_moved' for movement trigger", src));
        }
    } else if !text.contains("コスト") {
        return Some("Effect has no source (defaults to 'discard'), should be 'recently_moved'".to_string());
    }
    None
}

/// Pattern 4: Text says "合計がN以上" but operator is "=" or location is wrong
fn check_aggregate_operator(text: &str, effect: &serde_json::Value) -> Option<String> {
    if !text.contains("合計") && !text.contains("の合計が") {
        return None;
    }
    // Only check if there's an aggregate condition
    let check_cond = |cond: &serde_json::Value| -> Option<String> {
        if cond.get("aggregate").and_then(|a| a.as_str()) != Some("total") {
            return None;
        }
        let op = cond.get("operator").and_then(|o| o.as_str()).unwrap_or("");
        if op == "=" {
            return Some(format!("aggregate operator is '=', should be '>=' (text: '{}')",
                truncate(text, 30)));
        }
        if cond.get("location").is_none() {
            return Some("aggregate condition has no location field".to_string());
        }
        None
    };
    if let Some(cond) = effect.get("condition") {
        if let Some(conds) = cond.get("conditions").and_then(|c| c.as_array()) {
            for sub in conds {
                if let Some(issue) = check_cond(sub) {
                    return Some(issue);
                }
            }
        }
        if let Some(issue) = check_cond(cond) {
            return Some(issue);
        }
    }
    None
}

/// Pattern 5: gain_resource with icon_all should use heart_type:"all"
fn check_heart_type_all(text: &str, effect: &serde_json::Value) -> Option<String> {
    if !text.contains("icon_all") && !text.contains("icon_all.png") && !text.contains("ハート}}{{icon_all") {
        return None;
    }
    if effect.get("action").and_then(|a| a.as_str()) == Some("gain_resource") {
        let ht = effect.get("heart_type").and_then(|h| h.as_str());
        let hc = effect.get("heart_colors").and_then(|h| h.as_array());
        if ht != Some("all") && hc.map(|a| a.len() >= 1).unwrap_or(false) {
            return Some("gain_resource has heart_colors but should have heart_type:'all' for icon_all".to_string());
        }
    }
    None
}

/// Pattern 6: look_and_select with filter that should exclude certain abilities
fn check_look_select_filter(text: &str, effect: &serde_json::Value) -> Option<String> {
    if effect.get("action").and_then(|a| a.as_str()) != Some("look_and_select") {
        return None;
    }
    // Check if select_action has heart_colors filter that seems wrong
    if let Some(sa) = effect.get("select_action") {
        let hc = sa.get("heart_colors").and_then(|h| h.as_array());
        if hc.map(|a| !a.is_empty()).unwrap_or(false) {
            // If there's an ability_filter too, the heart_colors may be fine
            if sa.get("ability_filter").is_none() {
                // Check if text mentions an ability type to exclude
                if text.contains("能力を持たない") || text.contains("能力を持っていない") {
                    return Some("select_action has heart_colors filter but should have ability_filter to exclude abilities".to_string());
                }
            }
        }
    }
    None
}

/// Pattern 7: Text says "のみで" (only/exclusively) but condition doesn't enforce it
fn check_exclusivity(text: &str, effect: &serde_json::Value) -> Option<String> {
    if !text.contains("のみで") {
        return None;
    }
    if let Some(cond) = effect.get("condition") {
        if let Some(conds) = cond.get("conditions").and_then(|c| c.as_array()) {
            for sub in conds {
                let ct = sub.get("type").and_then(|t| t.as_str()).unwrap_or("");
                if ct == "group_condition" {
                    // group_condition with >=1 doesn't enforce exclusivity.
                    // Check if there's a negation/sub-condition for non-matching cards
                    let has_exclusivity_check = conds.iter().any(|s| {
                        s.get("negation").and_then(|n| n.as_bool()) == Some(true)
                            || s.get("type").and_then(|t| t.as_str()) == Some("comparison_condition")
                    });
                    if !has_exclusivity_check {
                        return Some("'のみで' in text but condition only checks group count >=1, doesn't enforce exclusivity".to_string());
                    }
                }
            }
        }
    }
    None
}

/// Pattern 8: Turn limit — text says ターン1回 but use_limit is missing
fn check_turn1_limit(text: &str, entry: &serde_json::Value) -> Option<String> {
    if text.contains("ターン1回") || text.contains("ターン１回") {
        if entry.get("use_limit").and_then(|u| u.as_u64()).unwrap_or(0) == 0 {
            return Some("Text says ターン1回 but use_limit is 0/null".to_string());
        }
    }
    None
}
