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

    fn json_path_decode(entry: &serde_json::Value) -> Option<Ability> {
        let mut normalized = entry.clone();
        if let Some(cost_val) = normalized.get_mut("cost") {
            if let Some(obj) = cost_val.as_object_mut() {
                rabuka_engine::ability::vm::normalize_cost_keys(obj);
            }
        }
        let mut ab: Ability = serde_json::from_value::<Ability>(normalized.clone()).ok()?;
        if let Some(ref mut effect) = ab.effect {
            if let Some(ref je) = normalized.get("effect") {
                effect.populate_from_json(je);
            }
        }
        if let Some(ref mut cost) = ab.cost {
            if let Some(ref ce) = normalized.get("cost") {
                cost.0.populate_from_json(ce);
            }
        }
        if let Some(ref mut effect) = ab.effect {
            if let Some(ref actions) = effect.compound.actions.clone() {
                let fixed: Vec<Box<AbilityEffect>> = actions
                    .iter()
                    .map(|a| {
                        let mut f = a.clone();
                        if (f.action == ActionType::DrawCard)
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

    fn collect_diffs(
        a: &serde_json::Value,
        b: &serde_json::Value,
        path: &str,
        out: &mut Vec<String>,
    ) {
        match (a, b) {
            (serde_json::Value::Object(ao), serde_json::Value::Object(bo)) => {
                let mut all_keys: Vec<&String> = ao.keys().chain(bo.keys()).collect();
                all_keys.sort();
                all_keys.dedup();
                for k in all_keys {
                    let p = if path.is_empty() {
                        k.clone()
                    } else {
                        format!("{path}.{k}")
                    };
                    match (ao.get(k), bo.get(k)) {
                        (Some(av), Some(bv)) => collect_diffs(av, bv, &p, out),
                        (Some(av), None) => out.push(format!("MISSING_IN_JSON {p} = {av}")),
                        (None, Some(bv)) => out.push(format!("MISSING_IN_BC {p} = {bv}")),
                        (None, None) => {}
                    }
                }
            }
            (serde_json::Value::Array(ao), serde_json::Value::Array(bo)) => {
                for i in 0..ao.len().max(bo.len()) {
                    let p = format!("{path}[{i}]");
                    match (ao.get(i), bo.get(i)) {
                        (Some(av), Some(bv)) => collect_diffs(av, bv, &p, out),
                        (Some(_), None) => out.push(format!("EXTRA_IN_JSON {p}")),
                        (None, Some(_)) => out.push(format!("EXTRA_IN_BC {p}")),
                        (None, None) => {}
                    }
                }
            }
            _ if a != b => {
                let a_s = a.to_string();
                let b_s = b.to_string();
                fn truncate(s: &str, max: usize) -> String {
                    let mut end = s.len().min(max);
                    while end > 0 && !s.is_char_boundary(end) {
                        end -= 1;
                    }
                    format!("{}...", &s[..end])
                }
                let a_trunc = truncate(&a_s, 100);
                let b_trunc = truncate(&b_s, 100);
                out.push(format!("{path}: BC={a_trunc} JSON={b_trunc}"));
            }
            _ => {}
        }
    }

    #[test]
    fn bytecode_deep_matches_json_path() {
        let json_abilities = load_json_abilities();
        assert_eq!(ability_count(), json_abilities.len());

        let mut mismatches = 0usize;
        let mut all_diffs: Vec<(usize, Vec<String>)> = Vec::new();
        for (idx, entry) in json_abilities.iter().enumerate() {
            let bc = get_ability(idx).ok();
            let jp = json_path_decode(entry);
            match (bc, jp) {
                (Some(a), Some(b)) => {
                    let av = serde_json::to_value(&a).unwrap_or(serde_json::Value::Null);
                    let bv = serde_json::to_value(&b).unwrap_or(serde_json::Value::Null);
                    if av != bv {
                        mismatches += 1;
                        if all_diffs.len() < 5 {
                            let mut diffs = Vec::new();
                            collect_diffs(&av, &bv, "", &mut diffs);
                            all_diffs.push((idx, diffs));
                        }
                    }
                }
                (a, b) => {
                    mismatches += 1;
                    if all_diffs.len() < 5 {
                        all_diffs
                            .push((idx, vec![format!("bc={} jp={}", a.is_some(), b.is_some())]));
                    }
                }
            }
        }

        if !all_diffs.is_empty() {
            let mut report = String::new();
            for (idx, diffs) in &all_diffs {
                report.push_str(&format!("Ability {idx} ({} diffs):\n", diffs.len()));
                for d in diffs.iter().take(20) {
                    report.push_str(&format!("  {d}\n"));
                }
                report.push('\n');
            }
            let _ = std::fs::write(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .unwrap()
                    .join("deep_diffs.txt"),
                &report,
            );
            eprintln!("{report}");
        }
        assert_eq!(
            mismatches, 0,
            "{mismatches} abilities differ between bytecode and json path"
        );
    }
}
