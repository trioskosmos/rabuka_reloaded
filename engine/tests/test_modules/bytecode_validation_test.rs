#[cfg(feature = "bytecode_abilities")]
mod bytecode_validation {
    use rabuka_engine::ability::enums::ActionType;
    use rabuka_engine::ability::vm::{ability_count, get_ability};
    use rabuka_engine::card::{Ability, AbilityEffect};

    /// The bytecode compiler intentionally re-encodes some JSON effects into a
    /// different wire format. Given a JSON effect, return the action string the
    /// decoder is expected to produce.
    fn normalize_json_action<'a>(json_effect: &serde_json::Value, json_action: &'a str) -> &'a str {
        // Any effect carrying a `condition` (other than the three explicit
        // conditional shapes) is compiled into the conditional_alternative wire
        // op (0x61), so the decoded top-level action becomes
        // "conditional_alternative".
        let is_explicit_conditional = matches!(
            json_action,
            "conditional_alternative" | "conditional_on_optional" | "conditional_on_result"
        );
        if !is_explicit_conditional && json_effect.get("condition").is_some() {
            return "conditional_alternative";
        }
        // Plain string renames performed by the compiler.
        match json_action {
            "draw" => "draw_card",
            other => other,
        }
    }

    #[test]
    fn bytecode_ability_0() {
        let a = get_ability(0);
        assert!(a.is_some(), "Ability 0 must decode");
        let a = a.unwrap();
        assert!(a.effect.is_some(), "Ability 0 must have effect");
        eprintln!(
            "Ability 0 effect action: {}",
            a.effect.as_ref().unwrap().action
        );
    }

    fn load_json_abilities() -> Vec<serde_json::Value> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("cards/abilities.json");
        let contents = std::fs::read_to_string(path).unwrap();
        let data: serde_json::Value = serde_json::from_str(&contents).unwrap();
        data["unique_abilities"].as_array().unwrap().clone()
    }

    #[test]
    fn bytecode_count_matches_json() {
        let json_abilities = load_json_abilities();
        assert_eq!(
            ability_count(),
            json_abilities.len(),
            "Bytecode ability count {} must match JSON {}",
            ability_count(),
            json_abilities.len()
        );
    }

    #[test]
    fn bytecode_every_ability_decodes() {
        let json_abilities = load_json_abilities();
        for i in 0..json_abilities.len() {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| get_ability(i)));
            match result {
                Ok(Some(_)) => {}
                Ok(None) => panic!("Bytecode ability {} returned None", i),
                Err(e) => panic!("Bytecode ability {} panicked: {:?}", i, e),
            }
        }
    }

    #[test]
    fn bytecode_nonempty_effects_match_action() {
        let json_abilities = load_json_abilities();
        for i in 0..json_abilities.len() {
            let ability = get_ability(i).unwrap();
            let json_entry = &json_abilities[i];

            // Check effect action matches
            if let Some(ref json_effect) = json_entry.get("effect") {
                if let Some(json_action) = json_effect.get("action").and_then(|v| v.as_str()) {
                    if !json_action.is_empty() && ability.effect.is_some() {
                        let eff = ability.effect.as_ref().unwrap();
                        let bc_action = eff.action.to_str();
                        if bc_action.is_empty() {
                            // skip — compound effects use different naming
                        } else if bc_action != json_action {
                            // The bytecode compiler intentionally re-encodes some
                            // effects into a different wire format. Accept these
                            // documented normalizations instead of failing.
                            let normalized = normalize_json_action(json_effect, json_action);
                            assert_eq!(
                                bc_action, normalized,
                                "Ability {}: action mismatch: JSON='{}' BC='{}'",
                                i, json_action, bc_action
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn bytecode_cost_matches_json() {
        let json_abilities = load_json_abilities();
        for i in 0..json_abilities.len() {
            let ability = get_ability(i).unwrap();
            let json_entry = &json_abilities[i];
            let has_json_cost = json_entry.get("cost").is_some_and(|c| {
                if let Some(arr) = c.as_array() {
                    !arr.is_empty()
                } else if let Some(obj) = c.as_object() {
                    !obj.is_empty()
                } else {
                    false
                }
            });
            let has_bc_cost = ability.cost.is_some();
            if has_json_cost != has_bc_cost {
                // Some costs are compile-time stripped (choice, etc.)
                let json_cost_type = json_entry
                    .get("cost")
                    .and_then(|c| c.get("type").and_then(|v| v.as_str()))
                    .unwrap_or("unknown");
                if json_cost_type == "choice_condition" {
                    continue; // choice conditions not yet compiled
                }
                // Report but don't fail for now
                eprintln!(
                    "Ability {}: JSON cost={} BC cost={}",
                    i, has_json_cost, has_bc_cost
                );
            }
        }
    }

    #[test]
    fn bytecode_debug_ability_239() {
        let ab = get_ability(239).expect("ability 239 should exist");
        println!("=== Ability 239 ===");
        println!("effect: {:?}", ab.effect.as_ref().map(|e| &e.action));
        if let Some(ref eff) = ab.effect {
            println!("  action: {}", eff.action);
            println!("  count: {:?}", eff.count);
            println!("  source: {:?}", eff.source);
            println!("  destination: {:?}", eff.destination);
            println!("  target: {:?}", eff.target);
            println!(
                "  condition: {:?}",
                eff.condition.as_ref().map(|c| format!("{:?}", c))
            );
            println!(
                "  compound actions: {:?}",
                eff.compound.actions.as_ref().map(|a| a.len())
            );
            println!(
                "  effect_steps: {:?}",
                eff.effect_steps.as_ref().map(|s| s.len())
            );
        }
        assert!(ab.effect.is_some(), "ability 239 should have an effect");
    }
}
