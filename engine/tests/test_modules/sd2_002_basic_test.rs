use crate::helpers::*;
use rabuka_engine::card::HeartColor;

/// Test: PL!SP-sd2-002-SD2 (唐 可可)
///
/// ab#0 (起動): {{kidou.png|起動}}{{turn1.png|ターン1回}}{{icon_energy.png|E}}{{icon_energy.png|E}}
///   ：このメンバーをポジションチェンジする。
///   Cost: 2 energy → Effect: position change
///
/// ab#1 (自動): {{jidou.png|自動}}{{turn1.png|ターン1回}}
///   このメンバーがエリアを移動したとき、ライブ終了時まで、{{heart_06.png|heart06}}を得る。
///   Trigger: when this member moves area
///   Effect: gain heart06 (×1, duration: live_end)
///
/// Scenario: Play in main phase → activate kidou (position change) →
///   auto-ability fires on area move → card gains heart06 modifier.
#[test]
fn sd2_002_kidou_position_change_grants_heart_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let keke = game.id("PL!SP-sd2-002-SD2");
    let filler = game.id("PL!-sd1-010-SD");

    // Place keke on center stage and filler on left (swap target)
    game.add_to_stage(rabuka_engine::zones::MemberArea::Center, keke);
    game.add_to_stage(rabuka_engine::zones::MemberArea::LeftSide, filler);

    // Give 2 energy for the kidou cost
    game.give_energy(2);

    // Activate kidou (ab#0: position change)
    game.activate_ability(keke);

    // Should prompt for position|destination choice
    assert!(
        game.has_pending_choice(),
        "Expected position choice after kidou activation"
    );

    // Select the left position to swap with filler
    let actions = game.generated_actions();
    let left_idx = actions
        .iter()
        .position(|a| {
            a.parameters
                .as_ref()
                .and_then(|p| p.stage_area.as_deref())
                .is_some_and(|area| area == "left")
        })
        .expect("No left position option");
    game.select_generated(left_idx);

    // Drain any auto-ability choices (ab#1 fires automatically)
    game.drain_auto_ability_choices();

    // Verify: keke moved to left, filler moved to center
    assert_eq!(
        game.state.player1.stage.stage[0], keke,
        "keke should be on left after swap"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], filler,
        "filler should be on center after swap"
    );

    // Verify: heart06 modifier was granted by the auto-ability
    let heart_mod = game
        .state
        .mods
        .get_heart_modifier(keke, HeartColor::Heart06);
    assert_eq!(
        heart_mod, 1,
        "keke should have heart06 ×1 modifier from area move auto-ability"
    );
}
