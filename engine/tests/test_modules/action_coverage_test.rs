/// Integration test that tries to fire every unique action type from abilities.json
/// through the engine's ability resolver, reporting which actions fail to resolve.
use crate::helpers::*;
use rabuka_engine::ability::enums::ActionType;
use rabuka_engine::game_setup::ActionType as GameAction;
use rabuka_engine::turn::TurnEngine;

use std::collections::HashSet;

fn has_action(card: &rabuka_engine::card::Card, target: &ActionType) -> bool {
    for ab in card.resolved_abilities() {
        if let Some(ref eff) = ab.effect {
            if eff.action == *target {
                return true;
            }
            if let Some(ref actions) = eff.compound.actions {
                for sub in actions {
                    if sub.action == *target {
                        return true;
                    }
                }
            }
        }
    }
    false
}

#[test]
fn all_action_types_fire_without_crash() {
    let db = load_real_database();

    // Collect all unique action types found on member cards
    let mut action_types: HashSet<ActionType> = HashSet::new();
    for (_tid, card) in db.cards.iter() {
        if !matches!(card.card_type, rabuka_engine::card::CardType::Member) {
            continue;
        }
        for ab in card.resolved_abilities() {
            if let Some(ref eff) = ab.effect {
                action_types.insert(eff.action);
                if let Some(ref actions) = eff.compound.actions {
                    for sub in actions {
                        action_types.insert(sub.action);
                    }
                }
            }
        }
    }

    let mut action_list: Vec<&ActionType> = action_types.iter().collect();
    action_list.sort_by_key(|a| a.to_str());

    eprintln!(
        "\n=== Testing {} unique action types ===\n",
        action_list.len()
    );

    let mut ok = 0;
    let mut fail_count = 0;

    for action in &action_list {
        let mut test_card_no: Option<String> = None;
        for (_tid, card) in db.cards.iter() {
            if !matches!(card.card_type, rabuka_engine::card::CardType::Member) {
                continue;
            }
            if has_action(card, action) {
                test_card_no = Some(card.card_no.to_string());
                break;
            }
        }

        let card_no = match test_card_no {
            Some(c) => c,
            None => {
                eprintln!("  [SKIP] {} -- no member card found", action);
                continue;
            }
        };

        let mut game = TestGame::new(db.clone());
        let cid = game.id(&card_no);

        game.state.player1.stage.stage = [-1, cid, -1];
        game.give_energy(20);

        let result = TurnEngine::execute_main_phase_action(
            &mut game.state,
            &GameAction::UseAbility,
            Some(cid),
            None,
            None,
            None,
        );

        match result {
            Ok(_) => {
                ok += 1;
                eprintln!("  [OK] {}", action);
            }
            Err(e) => {
                if e.contains("No abilities")
                    || e.contains("unknown action")
                    || e.contains("not implemented")
                {
                    fail_count += 1;
                    eprintln!("  [FAIL] {} -- {}", action, e);
                } else {
                    ok += 1;
                    eprintln!("  [OK] {} (handled: {})", action, e);
                }
            }
        }
    }

    eprintln!("\n=== Results: {} OK, {} FAIL ===\n", ok, fail_count);
    assert_eq!(
        fail_count, 0,
        "{} action types failed to resolve",
        fail_count
    );
}
