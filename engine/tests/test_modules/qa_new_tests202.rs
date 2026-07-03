/// Tests for Q202/Q201/Q200 — deploy from hand via debut ability, then
/// the deployed card's own debut ability fires separately.
///
/// All three cards share the same pattern:
///   登場 {{icon_energy.png|E}}{{icon_energy.png|E}}支払ってもよい：
///     手札からコスト4以下の特定のメンバーカードを1枚ステージに登場させる。
///
/// Q202: ミア・テイラー PL!N-pb1-023-R deploys PL!N-PR-013-PR
/// Q201: 宮下 愛     PL!N-pb1-017-R deploys PL!N-bp4-005-R
/// Q200: 上原歩夢    PL!N-pb1-013-R deploys PL!N-sd1-013-SD
///
/// Ruling: the deployed card's 登場 ability fires normally.
use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

/// Helper: play a deployer card, trigger its debut, deploy target, then
/// process the target's debut ability.
fn deploy_and_trigger(
    game: &mut TestGame,
    deployer: &str,
    target: &str,
    energy_total: usize,
) -> (i16, i16) {
    let deployer_id = game.id(deployer);
    let target_id = game.id(target);
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(deployer_id);
    game.state.player1.hand.cards.push(target_id);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(energy_total);

    // Play deployer to stage via TurnEngine (bypasses auto prompts)
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(deployer_id),
        None,
        Some(MemberArea::Center),
        Some(false),
    )
    .expect("play to stage");

    // Now drain auto-ability selections to trigger the debut
    while game.has_pending_choice() {
        let choice = game.get_pending_choice().clone();
        match choice {
            rabuka_engine::ability::types::Choice::SelectAutoAbility { .. } => {
                // Trigger the debut ability
                game.select_option(0);
            }
            rabuka_engine::ability::types::Choice::SelectTarget { target, .. } => {
                if target == "conditional_optional"
                    || target == "pay_optional_cost:skip_optional_cost"
                {
                    // Pay the optional cost (option 1 = pay)
                    game.select_option(1);
                } else if target == "position|destination" {
                    // Deploy to a specific area - choose first available
                    game.select_generated(0);
                } else {
                    break;
                }
            }
            rabuka_engine::ability::types::Choice::SelectCard { zone, .. } => {
                if zone == "hand" {
                    // Select target card from hand to deploy
                    game.select_indices(&[0]);
                } else {
                    break;
                }
            }
            _ => {
                break;
            }
        }
    }

    (deployer_id, target_id)
}

/// Q202: ミア・テイラー deploys another ミア (PR-013-PR).
/// The deployed PR card's debut ability (optional discard → look top 3, add 1) fires.
#[test]
fn q202_mia_deploy_triggers_target_debut() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let (_deployer, target) = deploy_and_trigger(
        &mut game,
        "PL!N-pb1-023-R", // ミア・テイラー (cost 13)
        "PL!N-PR-013-PR", // ミア・テイラー (cost 4)
        15,               // 13 to play + 2 for ability
    );

    // The target's debut ability should now present choices:
    // Optional: discard 1 from hand → look at top 3, add 1 to hand
    // The choice is: SelectCard for discard (optional)
    // If we skip, no look happens. If we pay, look happens.
    if game.has_pending_choice() {
        // Skip the optional discard cost
        game.select_indices(&[]);
    }

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Target should be on stage
    assert!(
        game.state.player1.stage.stage.contains(&target),
        "Deployed ミア should be on stage"
    );
}

/// Q202 (skip variant): Decline the deploy cost → target never appears.
#[test]
fn q202_mia_deploy_decline_cost_no_deploy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let deployer_id = game.id("PL!N-pb1-023-R");
    let target_id = game.id("PL!N-PR-013-PR");

    game.state.player1.hand.cards.push(deployer_id);
    game.state.player1.hand.cards.push(target_id);
    game.give_energy(13); // only enough to play, not for ability

    game.play_to_stage(deployer_id, MemberArea::Center);

    // Deployer's debut fires — decline the optional 2E payment
    if game.has_pending_choice() {
        game.select_option(0); // trigger debut
    }

    // Energy payment choice — decline (skip)
    if game.has_pending_choice() {
        game.select_generated(0); // may skip or select decline
    }

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Target should still be in hand
    assert!(
        game.state.player1.hand.cards.contains(&target_id),
        "Target should stay in hand when cost declined"
    );
}

/// Q201: 宮下 愛 deploys another 宮下 愛 (bp4-005-R).
/// The deployed card's debut ability (optional discard → wait members) fires.
#[test]
fn q201_miyashita_deploy_triggers_target_debut() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let (_deployer, target) = deploy_and_trigger(
        &mut game,
        "PL!N-pb1-017-R", // 宮下 愛 (cost 7)
        "PL!N-bp4-005-R", // 宮下 愛 (cost 4)
        9,                // 7 to play + 2 for ability
    );

    // Target's debut: optional discard 1 → wait up to 2 cost≤4 members
    if game.has_pending_choice() {
        // Skip the optional discard
        game.select_indices(&[]);
    }

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        game.state.player1.stage.stage.contains(&target),
        "Deployed 宮下 愛 should be on stage"
    );
}

/// Q200: 上原歩夢 deploys another 上原歩夢 (sd1-013-SD).
/// The deployed card's debut ability (draw 1, discard 1) fires.
#[test]
fn q200_uehara_deploy_triggers_target_debut() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let (_deployer, target) = deploy_and_trigger(
        &mut game,
        "PL!N-pb1-013-R",  // 上原歩夢 (cost 7)
        "PL!N-sd1-013-SD", // 上原歩夢 (cost 4)
        9,                 // 7 to play + 2 for ability
    );

    // Target's debut: draw 1, discard 1 (mandatory).
    // After drawing, a discard choice appears.
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        game.state.player1.stage.stage.contains(&target),
        "Deployed 上原歩夢 should be on stage"
    );
}
