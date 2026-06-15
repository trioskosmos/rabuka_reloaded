/// PL!-bp3-002-R (絢瀬絵里) ab#0 — Q144
///
/// {{toujyou.png|登場}}手札を1枚控え室に置いてもよい：
/// 相手のステージにいるコスト4以下のメンバーを2人までウェイトにする。
///
/// Q144: When only 1 eligible member (cost ≤ 4) is on the opponent's stage,
/// can the ability still activate and put that 1 member to wait?
/// A: Yes — "まで" (up to) is an upper bound, not a requirement.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

#[test]
fn eri_q144_up_to_semantics_1_eligible_opponent_still_works() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let eri = game.id("PL!-bp3-002-R");
    let eligible = game.id("PL!-sd1-010-SD"); // cost=4, under limit
    let filler = game.id("PL!-sd1-019-SD");

    // Opponent's stage: 1 eligible member (cost=4). Self stage: empty for eri.
    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.player2.stage.stage = [eligible, -1, -1];
    game.add_to_hand(eri);
    game.add_to_hand(filler);
    game.give_energy(15);

    assert_eq!(
        game.state.mods.get_orientation_modifier(eligible),
        None,
        "Before activation: eligible member is active on opponent's stage"
    );

    game.play_to_stage(eri, MemberArea::Center);

    // Step 1: Optional cost (discard 1 from hand).
    // Hand has [filler] after playing eri. Choice is SelectCard (zone=hand, count=1, optional).
    assert!(game.has_pending_choice(), "Expected optional discard choice");
    game.assert_select_card("hand", 1, true);

    // Pay cost by discarding the filler (index 0)
    game.select_indices(&[0]);

    // Verify cost was recorded as paid (entry may be gone after resolution)
    if let Some(e) = game.state.ability_queue.current_entry() {
        if !e.completed {
            assert_eq!(e.optional_cost_result, Some(true), "optional_cost_result should be Some(true)");
        }
    }

    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        1,
        "Filler card should be in waitroom after discard"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        0,
        "Hand should be empty after playing eri + discarding filler"
    );

    // Step 2: Select opponent member(s) to put to wait (up to 2).
    assert!(game.has_pending_choice(), "Expected opponent member selection");
    game.assert_select_card("stage", 2, true);

    // Select the only eligible member (index 0)
    game.select_indices(&[0]);

    // Q144: Only 1 eligible member existed, but the ability activates
    // and puts that member to wait — "up to 2" is an upper bound
    assert!(!game.has_pending_choice(), "No pending choices remaining");
    let orientation = game.state.mods.get_orientation_modifier(eligible);
    assert_eq!(
        orientation,
        Some(&"wait".to_string()),
        "1 eligible opponent member was put to wait — 'up to 2' is an upper bound"
    );
}

/// Opponent has 2 eligible members (cost ≤ 4) and 1 ineligible (cost > 4).
/// Player skips the optional cost → no effect at all.
#[test]
fn eri_q144_skip_cost_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let eri = game.id("PL!-bp3-002-R");
    let eligible_1 = game.id("PL!-sd1-010-SD"); // cost=4
    let eligible_2 = game.id("PL!-sd1-010-SD"); // cost=4
    let too_expensive = game.id("PL!-sd1-001-SD"); // cost=5, over limit
    let filler = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.player2.stage.stage = [eligible_1, eligible_2, too_expensive];
    game.add_to_hand(eri);
    game.add_to_hand(filler);
    game.give_energy(15);

    game.play_to_stage(eri, MemberArea::Center);

    // Step 1: Optional cost — skip it.
    assert!(game.has_pending_choice(), "Expected optional discard choice");
    game.assert_select_card("hand", 1, true);
    game.select_indices(&[]);

    // Verify cost was skipped (entry may be gone after resolution)
    if let Some(e) = game.state.ability_queue.current_entry() {
        if !e.completed {
            assert_eq!(e.optional_cost_result, Some(false), "optional_cost_result should be Some(false)");
        }
    }

    // No effect — opponent members remain active
    assert!(!game.has_pending_choice(), "No pending choices (effect canceled)");
    assert_eq!(
        game.state.mods.get_orientation_modifier(eligible_1),
        None,
        "Eligible member 1 should remain active (cost skipped)"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(eligible_2),
        None,
        "Eligible member 2 should remain active (cost skipped)"
    );
}

/// Opponent has 0 eligible members → no selection prompt, cost still works.
#[test]
fn eri_q144_no_eligible_opponent_still_pays_cost() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let eri = game.id("PL!-bp3-002-R");
    let too_expensive = game.id("PL!-sd1-001-SD"); // cost=5, over limit
    let filler = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.player2.stage.stage = [too_expensive, -1, -1];
    game.add_to_hand(eri);
    game.add_to_hand(filler);
    game.give_energy(15);

    game.play_to_stage(eri, MemberArea::Center);

    // Step 1: Optional cost — pay it.
    assert!(game.has_pending_choice(), "Expected optional discard choice");
    game.assert_select_card("hand", 1, true);
    game.select_indices(&[0]); // discard filler

    // No selection prompt — opponent has no eligible members.
    // Effect completes without putting anyone to wait.
    assert!(!game.has_pending_choice(), "No pending choices (no eligible targets)");
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        1,
        "Filler was discarded (cost paid)"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(too_expensive),
        None,
        "Ineligible member should remain active"
    );
}
