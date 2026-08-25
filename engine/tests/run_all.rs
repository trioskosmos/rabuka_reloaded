mod helpers;
mod test_modules;

#[test]
fn test_parse_heart_color() {
    use rabuka_engine::card::{parse_heart_color, HeartColor};
    assert_eq!(parse_heart_color("heart00"), HeartColor::Heart00);
    assert_eq!(parse_heart_color("heart01"), HeartColor::Heart01);
    assert_eq!(parse_heart_color("heart06"), HeartColor::Heart06);
    assert_eq!(parse_heart_color("b_heart01"), HeartColor::Heart01);
    assert_eq!(parse_heart_color("b_heart03"), HeartColor::Heart03);
    assert_eq!(parse_heart_color("b_heart06"), HeartColor::Heart06);
    assert_eq!(parse_heart_color("b_all"), HeartColor::BAll);
    assert_eq!(parse_heart_color("draw"), HeartColor::Draw);
    assert_eq!(parse_heart_color("score"), HeartColor::Score);
    // Lenient fallback: unknown strings decode as colorless (Heart00).
    assert_eq!(parse_heart_color("bogus"), HeartColor::Heart00);
}

// NOTE: an old in-test parser-validation block used to live here; it was
// deleted because it printed gaps but never failed. CI now runs
// `extract_card_abilities.py --validate-only --check` against a seeded
// validation baseline (audit items H2/H3), which fails loudly on regressions.

/// Recursively find all keys named "action" with value "custom" in a JSON value.
fn find_custom_actions(val: &serde_json::Value, path: &str, results: &mut Vec<String>) {
    match val {
        serde_json::Value::Object(map) => {
            if let Some(action_val) = map.get("action") {
                if action_val == "custom" {
                    results.push(path.to_string());
                }
            }
            for (k, v) in map {
                let child = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", path, k)
                };
                find_custom_actions(v, &child, results);
            }
        }
        serde_json::Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                let child = if path.is_empty() {
                    format!("{}[{}]", path, i)
                } else {
                    format!("{}.{}", path, i)
                };
                find_custom_actions(v, &child, results);
            }
        }
        _ => {}
    }
}

/// Load abilities.json and check that no effect has `action: "custom"`.
/// A custom action means the parser could not determine a standard action type.
#[test]
fn test_no_custom_actions() {
    let abilities_path = std::path::Path::new("../cards/abilities.json");
    assert!(
        abilities_path.exists(),
        "abilities.json not found at {:?} — cannot validate",
        abilities_path
    );
    let contents = std::fs::read_to_string(abilities_path).expect("Failed to read abilities.json");
    let data: serde_json::Value =
        serde_json::from_str(&contents).expect("Failed to parse abilities.json");

    let mut custom_actions = Vec::new();
    if let Some(unique_abilities) = data.get("unique_abilities").and_then(|v| v.as_array()) {
        for (i, entry) in unique_abilities.iter().enumerate() {
            let prefix = format!("unique_abilities[{}]", i);
            find_custom_actions(entry, &prefix, &mut custom_actions);
        }
    }

    if !custom_actions.is_empty() {
        eprintln!("\nCUSTOM ACTIONS DETECTED — parser could not determine action type:");
        for ca in &custom_actions {
            eprintln!("  {}", ca);
        }
        panic!(
            "{} custom action(s) found. These need parser updates.",
            custom_actions.len()
        );
    }
}
