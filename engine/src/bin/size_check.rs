fn main() {
    macro_rules! sz {
        ($t:ty) => {
            println!("{:<35} {}", stringify!($t), std::mem::size_of::<$t>())
        };
    }

    println!("=== The REAL offenders ===");
    sz!(rabuka_engine::card::Condition);
    sz!(Option<rabuka_engine::card::Condition>);
    sz!(Box<rabuka_engine::card::Condition>);
    sz!(Option<Box<rabuka_engine::card::Condition>>);
    println!();
    sz!(rabuka_engine::card::CompoundBranch);
    sz!(rabuka_engine::card::AbilityEffect);
    sz!(rabuka_engine::card::EffectKind);
    println!();
    sz!(rabuka_engine::ability::ability_store::AbilityRef);
    println!();
    sz!(Option<String>);
    sz!(Option<Box<str>>);
    sz!(Box<str>);
    sz!(Vec<String>);
    sz!(Option<Vec<String>>);
    sz!(Box<Vec<String>>);

    println!();
    println!("=== Field-level sizes ===");
    sz!(rabuka_engine::card::DynamicCount);
    sz!(Option<rabuka_engine::card::DynamicCount>);
    sz!(rabuka_engine::card::PositionInfo);
    sz!(Option<rabuka_engine::card::PositionInfo>);
    sz!(rabuka_engine::card::QuotedText);
    sz!(Option<rabuka_engine::card::QuotedText>);
    sz!(rabuka_engine::card::AbilityFilter);
    sz!(Option<rabuka_engine::card::AbilityFilter>);
    sz!(rabuka_engine::card::AbilityFilterBranch);
    sz!(Option<rabuka_engine::card::AbilityFilterBranch>);
    println!();
    println!("=== Top 10 longest abilities by effect text ===");
    let path = std::path::Path::new("../romfs/abilities.json");
    if !path.exists() {
        println!("abilities.json not found at {:?}", path);
        return;
    }
    let data: serde_json::Value = match std::fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s).unwrap(),
        Err(e) => {
            println!("Can't read: {}", e);
            return;
        }
    };
    let abilities = match data["unique_abilities"].as_array() {
        Some(a) => a,
        None => {
            println!("No unique_abilities array");
            return;
        }
    };

    let effect_overhead = 264i64;
    let effectkind_heap = 656i64;

    struct AbEntry {
        name: String,
        text_len: usize,
        effect_count: usize,
        est_ram: i64,
    }

    let mut entries: Vec<AbEntry> = abilities
        .iter()
        .filter_map(|ab| {
            let full = ab["full_text"].as_str().unwrap_or("").to_string();
            let count = count_effects(ab);
            let name = ab["cards"]
                .as_array()
                .and_then(|c| c.first())
                .and_then(|c| c.as_str())
                .unwrap_or("?")
                .to_string();
            Some(AbEntry {
                text_len: full.len(),
                effect_count: count,
                name,
                est_ram: effect_overhead
                    + effectkind_heap
                    + (full.len() as i64 * 2)
                    + (count as i64 * 100),
            })
        })
        .collect();

    entries.sort_by(|a, b| b.est_ram.cmp(&a.est_ram));
    println!(
        "{:<60} {:>6} {:>4} {:>8} {:>8}",
        "Name", "Text", "Eff", "EstRAM", "EffOnly"
    );
    println!("{}", "-".repeat(95));
    for e in entries.iter().take(10) {
        let eff_only = (e.effect_count as i64) * (effect_overhead + effectkind_heap);
        println!(
            "{:<60} {:>6} {:>4} {:>8} {:>8}",
            &e.name[..e.name.len().min(59)],
            e.text_len,
            e.effect_count,
            e.est_ram,
            eff_only,
        );
    }

    let total_effects: usize = abilities.iter().map(|ab| count_effects(ab)).sum();
    let total_abilities = abilities.len();
    println!();
    println!("Total abilities: {}", total_abilities);
    println!("Total effects (across all abilities): {}", total_effects);
    println!(
        "Total EffectKind heap: {} bytes",
        total_effects as i64 * effectkind_heap
    );
    println!(
        "Total AbilityEffect stack: {} bytes",
        total_effects as i64 * effect_overhead
    );
}

fn count_effects(v: &serde_json::Value) -> usize {
    if let Some(arr) = v.as_array() {
        return arr.iter().map(count_effects).sum();
    }
    if !v.is_object() {
        return 0;
    }
    let obj = v.as_object().unwrap();
    if !obj.contains_key("action") {
        return ["cost", "effect"]
            .iter()
            .filter_map(|k| obj.get(*k))
            .map(count_effects)
            .sum();
    }
    let mut count = 1;
    for key in &[
        "actions",
        "options",
        "followup_action",
        "select_action",
        "look_action",
        "optional_action",
        "conditional_action",
        "primary_effect",
    ] {
        if let Some(sub) = obj.get(*key) {
            count += count_effects(sub);
        }
    }
    count
}
