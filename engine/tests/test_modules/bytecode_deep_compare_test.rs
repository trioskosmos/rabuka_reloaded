#[cfg(feature = "bytecode_abilities")]
mod bytecode_deep_compare {
    use rabuka_engine::ability::enums::ActionType;
    use rabuka_engine::ability::vm::{ability_count, get_ability};
    use rabuka_engine::card::{Ability, AbilityEffect};

    fn load_json_abilities() -> Vec<serde_json::Value> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("cards/abilities.json");
        let contents = std::fs::read_to_string(path).unwrap();
        let data: serde_json::Value = serde_json::from_str(&contents).unwrap();
        data["unique_abilities"].as_array().unwrap().clone()
    }

    /// Decode an ability exactly as the default (non-bytecode) JSON loader does:
    /// serde `from_value::<Ability>` + `populate_from_json` + draw-count fix.
    fn json_path_decode(entry: &serde_json::Value) -> Option<Ability> {
        let mut ab: Ability = serde_json::from_value::<Ability>(entry.clone()).ok()?;
        if let Some(ref mut effect) = ab.effect {
            if let Some(ref je) = entry.get("effect") {
                effect.populate_from_json(je);
            }
        }
        if let Some(ref mut effect) = ab.effect {
            if let Some(ref actions) = effect.compound.actions.clone() {
                let fixed: Vec<Box<AbilityEffect>> = actions
                    .iter()
                    .map(|a| {
                        let mut f = a.clone();
                        if (f.action == ActionType::Draw || f.action == ActionType::DrawCard)
                            && f.count.is_none()
                            && f.dynamic_count_any().is_none()
                        {
                            f.count = Some(1);
                        }
                        f
                    })
                    .collect();
                effect.compound.actions = Some(fixed);
            }
        }
        Some(ab)
    }

    /// Deep-equality check that the bytecode path reconstructs the *exact* same
    /// `Ability` the JSON path produces. This is the automated guard that makes
    /// the bytecode path safe when new ability types/fields are added: any
    /// divergence from the JSON loader is caught here.
    #[test]
    fn bytecode_deep_matches_json_path() {
        let json_abilities = load_json_abilities();
        assert_eq!(ability_count(), json_abilities.len());

        let mut mismatches = 0usize;
        let mut first: Option<(usize, String, String)> = None;
        for (idx, entry) in json_abilities.iter().enumerate() {
            let bc = get_ability(idx);
            let jp = json_path_decode(entry);
            match (bc, jp) {
                (Some(a), Some(b)) => {
                    let av = serde_json::to_value(&a).unwrap_or(serde_json::Value::Null);
                    let bv = serde_json::to_value(&b).unwrap_or(serde_json::Value::Null);
                    if av != bv {
                        mismatches += 1;
                        if first.is_none() {
                            // Trim to first 4000 chars each for readability.
                            let pa = av.to_string();
                            let pb = bv.to_string();
                            first = Some((
                                idx,
                                pa.chars().take(4000).collect(),
                                pb.chars().take(4000).collect(),
                            ));
                        }
                    }
                }
                (a, b) => {
                    mismatches += 1;
                    if first.is_none() {
                        first = Some((
                            idx,
                            format!("bytecode present={}", a.is_some()),
                            format!("json present={}", b.is_some()),
                        ));
                    }
                }
            }
        }

        if let Some((idx, bc_s, jp_s)) = first {
            eprintln!("FIRST DEEP MISMATCH at ability idx {idx}");
            eprintln!("--- bytecode ---\n{bc_s}");
            eprintln!("--- json path ---\n{jp_s}");
        }
        assert_eq!(
            mismatches, 0,
            "{mismatches} abilities differ between bytecode and json path"
        );
    }
}
