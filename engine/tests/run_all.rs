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

/// Scenario oracle for the C replay runner (`engine_c/tests/replay.c`
/// scenario mode). Hand-authored pipe-format scenario files describe ONE
/// migrated test: card bindings, zone setup, actions, and expects.
///
/// Only runs when `RABUKA_SCENARIO` is set (input path); writes
/// `CHECK <n> <pass|FAIL> <actual> <expected>` lines to `RABUKA_ORACLE_OUT`.
/// The C runner prints the same format; diffing the two outputs is the
/// parity check. Normal runs (env unset) are a no-op.
#[test]
fn scenario_oracle() {
    use std::collections::HashMap;

    let path = match std::env::var("RABUKA_SCENARIO") {
        Ok(p) => p,
        Err(_) => return,
    };
    let out_path =
        std::env::var("RABUKA_ORACLE_OUT").unwrap_or_else(|_| "scenario_oracle.out".into());
    let text = std::fs::read_to_string(&path).expect("read scenario file");
    let db = helpers::load_real_database();
    let mut game = helpers::TestGame::new(db);
    let mut cards: HashMap<String, i16> = HashMap::new();
    let mut out: Vec<String> = Vec::new();
    let mut n = 0;

    fn player(game: &helpers::TestGame, p: u8) -> &rabuka_engine::player::Player {
        if p == 1 {
            &game.state.player1
        } else {
            &game.state.player2
        }
    }
    let card_no = |game: &helpers::TestGame, id: i16| {
        game.db
            .get_card(id)
            .map(|c| c.card_no.to_string())
            .unwrap_or_else(|| format!("#{}", id))
    };

    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let t: Vec<&str> = line.split_whitespace().collect();
        match t[0] {
            "card" => {
                let id = game.id(t[2]);
                cards.insert(t[1].to_string(), id);
            }
            "stage" => {
                let p: u8 = t[1].parse().expect("player");
                let a: usize = t[2].parse().expect("area");
                let id = cards[t[3]];
                if p == 1 {
                    game.state.player1.stage.stage[a] = id;
                } else {
                    game.state.player2.stage.stage[a] = id;
                }
            }
            "hand" | "discard" | "deck" | "live" | "success" => {
                let p: u8 = t[1].parse().expect("player");
                let id = cards[t[2]];
                let g = &mut game.state;
                let zone = if p == 1 { &mut g.player1 } else { &mut g.player2 };
                match t[0] {
                    "hand" => zone.hand.cards.push(id),
                    "discard" => zone.waitroom.cards.push(id),
                    "deck" => zone.main_deck.cards.push(id),
                    "live" => zone.live_card_zone.cards.push(id),
                    _ => zone.success_live_card_zone.cards.push(id),
                }
            }
            "energy" => {
                let p: u8 = t[1].parse().expect("player");
                let count: usize = t[2].parse().expect("count");
                for _ in 0..count {
                    let e = game.id("LL-E-001-SD");
                    let g = &mut game.state;
                    let zone = if p == 1 {
                        &mut g.player1.energy_zone
                    } else {
                        &mut g.player2.energy_zone
                    };
                    zone.cards.push(e);
                }
                let g = &mut game.state;
                let zone = if p == 1 {
                    &mut g.player1.energy_zone
                } else {
                    &mut g.player2.energy_zone
                };
                zone.add_active(count as u8);
            }
            "wait" => {
                let id = cards[t[1]];
                game.state.mods.add_orientation_modifier(id, "wait");
            }
            "recalculate" => {
                game.state.recalculate_constants();
            }
            "play" => {
                let id = cards[t[1]];
                let area = match t[2] {
                    "0" => rabuka_engine::zones::MemberArea::LeftSide,
                    "1" => rabuka_engine::zones::MemberArea::Center,
                    _ => rabuka_engine::zones::MemberArea::RightSide,
                };
                game.play_to_stage(id, area);
            }
            "activate" => {
                game.activate_ability(cards[t[1]]);
            }
            "pass" => game.pass(),
            "select" => {
                let idxs: Vec<usize> = if t.len() > 1 && !t[1].is_empty() {
                    t[1].split(',').map(|s| s.parse().expect("idx")).collect()
                } else {
                    Vec::new()
                };
                game.select_indices(&idxs);
            }
            "select_option" => {
                game.select_option(t[1].parse().expect("option"));
            }
            "set_live" => {
                game.set_live_card(cards[t[1]]);
            }
            "drain_auto" => {
                game.drain_auto_ability_choices();
            }
            "expect" => {
                n += 1;
                let (actual, expected): (String, String) = match t[1] {
                    "blade" => (
                        game.state.mods.get_blade_modifier(cards[t[2]]).to_string(),
                        t[3].to_string(),
                    ),
                    "score" => (
                        game.state.mods.get_score_modifier(cards[t[2]]).to_string(),
                        t[3].to_string(),
                    ),
                    "cost" => (
                        game.state.mods.get_cost_modifier(cards[t[2]]).to_string(),
                        t[3].to_string(),
                    ),
                    "hand" => {
                        let p: u8 = t[2].parse().expect("player");
                        (player(&game, p).hand.cards.len().to_string(), t[3].to_string())
                    }
                    "energy" => {
                        let p: u8 = t[2].parse().expect("player");
                        (
                            player(&game, p).energy_zone.active_count().to_string(),
                            t[3].to_string(),
                        )
                    }
                    "phase" => (game.state.current_phase.to_string(), t[2].to_string()),
                    "pending" => (
                        game.pending_choice_type().unwrap_or_else(|| "none".into()),
                        t[2].to_string(),
                    ),
                    "stage" => {
                        let p: u8 = t[2].parse().expect("player");
                        let a: usize = t[3].parse().expect("area");
                        let slot = player(&game, p).stage.stage[a];
                        let act = if slot == -1 {
                            "null".to_string()
                        } else {
                            card_no(&game, slot)
                        };
                        let exp = if t[4] == "null" {
                            "null".to_string()
                        } else {
                            card_no(&game, cards[t[4]])
                        };
                        (act, exp)
                    }
                    other => panic!("unknown expect kind: {}", other),
                };
                let verdict = if actual == expected { "pass" } else { "FAIL" };
                out.push(format!("CHECK {} {} {} {}", n, verdict, actual, expected));
            }
            other => panic!("unknown scenario op: {} (line: {})", other, raw),
        }
    }
    std::fs::write(&out_path, out.join("\n") + "\n").expect("write oracle output");
}
