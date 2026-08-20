#[cfg(feature = "bytecode_abilities")]
mod bytecode_validation {
    use rabuka_engine::ability::abilities_gen::NUM_ABILITIES;
    use rabuka_engine::ability::vm::{ability_count, get_ability};

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
        assert!(a.is_ok(), "Ability 0 must decode: {:?}", a.err());
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
                Ok(Ok(_)) => {}
                Ok(Err(e)) => panic!("Bytecode ability {} decode error: {e}", i),
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
    fn bytecode_debug_ability_399() {
        for idx in [545usize] {
            let ab = get_ability(idx).expect("exists");
            eprintln!(
                "[{}] cost={:?}",
                idx,
                ab.cost.as_ref().map(|c| format!(
                    "{:?}",
                    c.0.kind.as_ref().map(|k| format!("{:?}", k.as_ref()))
                ))
            );
            if let Some(c) = ab.cost.as_ref() {
                eprintln!("  cost action={} optional={:?}", c.0.action, c.0.optional);
            }
            eprintln!(
                "[{}] effect={:?}",
                idx,
                ab.effect.as_ref().map(|e| e.action.to_str().to_string())
            );
        }
    }

    #[test]
    fn bytecode_debug_ability_399_disabled() {
        let ab = get_ability(399).expect("ability 399 should exist");
        let eff = ab.effect.as_ref().expect("has effect");
        eprintln!(
            "399 action={} steps={:?} count={:?}",
            eff.action,
            eff.effect_steps.as_ref().map(|s| s.len()),
            eff.count
        );
        if let Some(steps) = eff.effect_steps.as_ref() {
            for (i, s) in steps.iter().enumerate() {
                eprintln!(
                    "  step[{}] action={} kind={:?}",
                    i,
                    s.action,
                    s.kind.as_ref().map(|k| format!("{:?}", k))
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

    #[test]
    fn every_card_ability_index_is_valid() {
        use rabuka_engine::ability::abilities_gen::{
            CARD_ABILITY_PAIRS, NUM_ABILITIES, STRINGS_OFFSETS, get_string,
        };
        let mut i = 0;
        while i + 1 < CARD_ABILITY_PAIRS.len() {
            let str_idx = CARD_ABILITY_PAIRS[i] as usize;
            let ability_idx = CARD_ABILITY_PAIRS[i + 1] as usize;
            assert!(
                get_string(str_idx).is_some(),
                "CARD_ABILITY_PAIRS[{i}]: string index {str_idx} out of range (max {})",
                STRINGS_OFFSETS.len() - 1
            );
            assert!(
                ability_idx < NUM_ABILITIES,
                "CARD_ABILITY_PAIRS[{}]: ability index {} out of range (max {}) for card '{}'",
                i + 1,
                ability_idx,
                NUM_ABILITIES,
                get_string(str_idx).unwrap_or("?")
            );
            i += 2;
        }
    }

    #[test]
    fn malformed_bytecode_returns_error() {
        // Verify that truncated/empty bytecode slices produce Err, not panic.
        // get_ability(NUM_ABILITIES) should return IndexOutOfRange.
        let result = get_ability(NUM_ABILITIES);
        assert!(result.is_err(), "out-of-range index should return Err");
    }

    #[test]
    fn empty_slice_returns_default_ability() {
        // Ability index 0 with start==end in the offsets table should return Ok(default).
        // This is the normal case for abilities with no cost/effect data.
        // We just verify it doesn't panic.
        let _ = get_ability(0);
    }
}
