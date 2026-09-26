#[cfg(feature = "bytecode_abilities")]
mod bytecode_validation {
    use rabuka_engine::ability::abilities_gen::NUM_ABILITIES;
    use rabuka_engine::ability::vm::{ability_count, get_ability, DecodeError};

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
        // Count the successes explicitly instead of relying on the loop body
        // never being reached: catch_unwind swallows a panic into Err, and a
        // future edit that swallowed it silently would turn this test into a
        // no-op. The final assert_eq is the real statement.
        let mut decoded = 0usize;
        for i in 0..json_abilities.len() {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| get_ability(i)));
            match result {
                Ok(Ok(_)) => decoded += 1,
                Ok(Err(e)) => panic!("Bytecode ability {} decode error: {}", i, e),
                Err(e) => panic!("Bytecode ability {} panicked: {:?}", i, e),
            }
        }
        assert_eq!(
            decoded,
            json_abilities.len(),
            "every JSON ability must decode: {decoded} of {}",
            json_abilities.len()
        );
    }

    /// Audit item C1: no ability may decode through an UNRECORDED silent
    /// default-substitution. Every unknown zone/distinct/ability_filter/
    /// keyword/condition field bumps `vm::note_decode_fallback`; this test
    /// attributes each fallback to its ability and asserts the exact baseline
    /// below. A new parser handler emitting an unmapped value fails here
    /// instead of silently turning an ability into a no-op.
    ///
    /// Baseline — empty. The former shadow-schema condition fields
    /// (`action_reference` on PL!SP-bp2-001-R＋ ab#0, `shuffle` on
    /// PL!N-bp7-011-R＋ ab#1) now have structural homes on ConditionCommon
    /// (card.rs), so they decode instead of tripping the audit. Any NEW
    /// fallback fails here — fix the mapping, do not append entries.
    #[test]
    fn bytecode_no_silent_decode_fallbacks() {
        // (card_no, ab# slot) of entries expected to carry unmapped condition
        // fields. Empty: every known key decodes. The ab# discriminator is
        // required: sibling abilities on the same card share the card_no.
        const KNOWN_FALLBACKS: &[(&str, &str)] = &[];
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

    /// The index guard is exact, and the error names the index that was wrong.
    ///
    /// `is_err()` alone also passes if the guard ever became `idx >
    /// NUM_ABILITIES` — which would reject the LAST real ability — or if a
    /// refactor answered an out-of-range index with a different error. The
    /// variant carries both numbers, so assert them, and pin the off-by-one
    /// from the other side: the last valid index must still decode to content.
    #[test]
    fn out_of_range_index_is_named_in_the_error() {
        match get_ability(NUM_ABILITIES) {
            Err(DecodeError::IndexOutOfRange { idx, max }) => {
                assert_eq!(
                    idx, NUM_ABILITIES,
                    "the error must name the index that was asked for"
                );
                assert_eq!(
                    max, NUM_ABILITIES,
                    "the error must name the exclusive bound"
                );
            }
            Err(other) => panic!(
                "the first out-of-range index must be IndexOutOfRange, got {other:?}"
            ),
            Ok(a) => panic!(
                "index {NUM_ABILITIES} decoded, but it is one past the last ability \
                 (0..{NUM_ABILITIES}): {:?}",
                a.effect.as_ref().map(|e| e.action)
            ),
        }
        // The guard is `>=`, not `>`: the last real ability still decodes.
        let last = get_ability(NUM_ABILITIES - 1)
            .unwrap_or_else(|e| panic!("the last ability must decode, got {e}"));
        assert_ne!(
            last,
            rabuka_engine::core::card::Ability::default(),
            "the last ability decoded to the default fallback, so the guard is off \
             by one on the valid side too"
        );
        assert_eq!(
            get_ability(NUM_ABILITIES + 1000).unwrap_err().to_string(),
            format!(
                "ability index {} out of range (max {})",
                NUM_ABILITIES + 1000,
                NUM_ABILITIES
            ),
            "the error text must name the index and the bound, so a log line is \
             actionable"
        );
    }

    /// A valid index never decodes to the default fallback.
    ///
    /// The old version of this test asserted only "index 0 is Ok" and
    /// "NUM_ABILITIES is not", which holds whether the decoder returned real
    /// content, the empty-slice fallback, or a hardcoded stub — the comment even
    /// said it could not tell which. There is no public decode-from-slice entry
    /// point, so the fallback cannot be exercised on its own; what CAN be pinned
    /// is that no ability lands on it. The corpus-wide count of empty slices is
    /// `bytecode_empty_slices_match_known_is_null_baseline`; this is the other
    /// half — every valid index produces a decodable struct that is not blank.
    #[test]
    fn a_valid_index_never_decodes_to_the_default_fallback() {
        for i in 0..ability_count() {
            let a = get_ability(i).unwrap_or_else(|e| panic!("ability {i}: {e}"));
            assert_ne!(
                a,
                rabuka_engine::core::card::Ability::default(),
                "ability {i} decoded to the default fallback — its bytecode produced \
                 nothing, so a printed effect would silently do nothing"
            );
        }
        // Control: the loop above would also pass if EVERY index returned the
        // default, so pin that at least one decode carries a real effect.
        let with_effect = (0..ability_count())
            .filter(|&i| get_ability(i).is_ok_and(|a| a.effect.is_some()))
            .count();
        assert!(
            with_effect > 0,
            "no ability decoded to an effect at all, so the loop above proved nothing"
        );
    }
}
