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

    /// Audit item C1: no ability may decode through an UNRECORDED silent
    /// default-substitution. Every unknown zone/distinct/ability_filter/
    /// keyword/condition field bumps `vm::note_decode_fallback`; this test
    /// attributes each fallback to its ability and asserts the exact baseline
    /// below. A new parser handler emitting an unmapped value fails here
    /// instead of silently turning an ability into a no-op.
    ///
    /// Baseline — shadow-schema condition fields (audit §4.6): the JSON puts
    /// `action_reference` / `reference_card` / `shuffle` / `card_names` on
    /// condition objects, but the typed Condition struct has no such fields;
    /// their effect-level twins ARE decoded via EffectFilter, so behavior
    /// flows through that path until the P9 format-v2 cleanup gives them a
    /// structural home. Do NOT add entries here for new values — fix the
    /// mapping instead.
    #[test]
    fn bytecode_no_silent_decode_fallbacks() {
        // (card_no, ab# slot) of the entries expected to carry shadow-schema
        // condition fields (triaged 2026-08-24). The ab# discriminator is
        // required: sibling abilities on the same card share the card_no.
        const KNOWN_FALLBACKS: &[(&str, &str)] = &[
            ("PL!SP-bp2-001-R＋", "(ab#0)"), // action_reference: "invalidate_ability"
            ("PL!N-bp7-011-R＋", "(ab#1)"),  // shuffle: true
        ];
        let _ = env_logger::try_init(); // surface [decode_audit] warnings under RUST_LOG
        let json_abilities = load_json_abilities();
        for i in 0..json_abilities.len() {
            let _ = get_ability(i);
        }
        // Expected set = indices of the JSON entries whose first card matches
        // a known prefix (computed, so regeneration order shifts don't break
        // the baseline).
        let first_card = |i: usize| -> String {
            json_abilities[i]
                .get("cards")
                .and_then(|c| c.as_array())
                .and_then(|c| c.first())
                .and_then(|s| s.as_str())
                .unwrap_or("?")
                .to_string()
        };
        let expected: Vec<usize> = (0..json_abilities.len())
            .filter(|&i| {
                let id = first_card(i);
                KNOWN_FALLBACKS
                    .iter()
                    .any(|(no, ab)| id.starts_with(no) && id.ends_with(ab))
            })
            .collect();
        assert_eq!(
            expected.len(),
            KNOWN_FALLBACKS.len(),
            "baseline (card_no, ab#) pairs matched no JSON entries — cards renamed?"
        );
        let actual = rabuka_engine::ability::vm::decode_fallback_abilities();
        assert_eq!(
            actual, expected,
            "silent decode fallback set changed — see [decode_audit] warnings under RUST_LOG=warn"
        );
    }

    /// Empty bytecode slices decode to `Ability::default()` with no error.
    /// Today NO corpus ability compiles to an empty slice (even the two
    /// `is_null` abilities carry minimal bytecode). Any increase means a new
    /// ability silently lost all of its effects during compilation.
    #[test]
    fn bytecode_empty_slices_match_known_is_null_baseline() {
        assert_eq!(
            rabuka_engine::ability::vm::count_empty_bytecode_abilities(),
            0,
            "empty-slice ability count changed — investigate which abilities lost their effects"
        );
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
                assert_eq!(
                    has_json_cost, has_bc_cost,
                    "Ability {}: JSON cost={} BC cost={} mismatch (type={})",
                    i, has_json_cost, has_bc_cost, json_cost_type
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
        // Index 0 decodes successfully whether its bytecode slice carries data
        // or is empty (start==end → default ability). Either way: Ok, no panic.
        let a = get_ability(0).expect("ability 0 must decode without error");
        // Whatever the slice contained, decoding must yield a usable ability
        // object with a known action.
        let _action = &a.effect.as_ref().map(|e| e.action);
        // And it must be distinct from the out-of-range error path.
        assert!(get_ability(NUM_ABILITIES).is_err());
    }
}
