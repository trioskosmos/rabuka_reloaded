use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

// ─── Ability text (津島善子 PL!S-bp3-006-R＋) ───
//
// [起動][センター][ターン1回]このメンバーをウェイトにし、手札を1枚控え室に置く：
// このメンバー以外の『Aqours』のメンバー1人を自分のステージから控え室に置く。
// そうした場合、自分の控え室から、そのメンバーのコストに2を足した数に等しいコストの
// 『Aqours』のメンバーカードを1枚、そのメンバーがいたエリアに登場させる。
// （この能力はセンターエリアに登場している場合のみ起動できる。）
//
// Translation:
// [Activation][Center][Once/turn] Rest this member, discard 1 from hand:
// Put 1 other Aqours member from your stage to waitroom.
// If you do, from your waitroom, put 1 Aqours member card whose cost equals
// that member's cost + 2, into the area that member was in.
// (This ability can only be activated if this card appeared in the center area.)

/// Clause: "（この能力はセンターエリアに登場している場合のみ起動できる。）"
/// Yoshiko at LeftSide → activation blocked.
#[test]
fn yoshiko_not_at_center_activation_blocked() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let yoshiko = game.id("PL!S-bp3-006-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [yoshiko, -1, -1];
    game.state.player1.hand.cards.push(filler);
    game.give_energy(15);

    let result = game.try_activate_ability(yoshiko);
    assert!(result.is_err(), "activation should fail when not at Center");
}

/// No other Aqours on stage (empty stage aside from self).
/// Cost paid, effect 1 has no valid target → conditional prevents effect 2.
#[test]
fn yoshiko_no_other_member_on_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let yoshiko = game.id("PL!S-bp3-006-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, yoshiko, -1];
    game.state.player1.hand.cards.push(filler);
    game.give_energy(15);

    game.activate_ability(yoshiko);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.player().stage.stage,
        [-1, yoshiko, -1],
        "stage unchanged — no valid target for action 1"
    );
    assert!(
        game.player().waitroom.cards.contains(&filler),
        "hand card should be discarded (cost paid)"
    );
}

/// Only non-Aqours members besides self on stage → filtered out.
#[test]
fn yoshiko_only_non_aqours_on_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let yoshiko = game.id("PL!S-bp3-006-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, yoshiko, filler];
    game.state.player1.hand.cards.push(filler);
    game.give_energy(15);

    game.activate_ability(yoshiko);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.player().stage.stage,
        [filler, yoshiko, filler],
        "stage unchanged — fillers are not Aqours"
    );
}

/// Multiple Aqours on stage → player chooses which to sacrifice.
#[test]
fn yoshiko_multiple_aqours_choice() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let yoshiko = game.id("PL!S-bp3-006-R\u{ff0b}");
    let chika = game.id("PL!S-bp2-001-R");
    let riko = game.id("PL!S-bp2-002-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [chika, yoshiko, riko];
    game.state.player1.hand.cards.push(filler);
    game.give_energy(15);

    game.activate_ability(yoshiko);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // One Aqours member should be in discard (the one we selected via indices[0])
    // With stage = [chika, yoshiko, riko], indices[0] selects LeftSide = chika
    assert!(
        game.player().waitroom.cards.contains(&chika),
        "chika should be in discard (selected by indices[0])"
    );
    assert!(
        game.player().stage.stage.contains(&riko),
        "riko should remain on stage (not selected)"
    );
    assert!(
        game.player().stage.stage.contains(&yoshiko),
        "yoshiko should remain on stage (excluded by exclude_self)"
    );
}

/// Clauses: "そのメンバーがいたエリアに登場させる" + "コストに2を足した数に等しいコスト"
/// Sacrifice chika (cost 9) from LeftSide → deploy dia (cost 11 = 9+2) to vacated LeftSide.
#[test]
fn yoshiko_sacrifice_and_summon_cost_plus_two_to_vacated_area() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let yoshiko = game.id("PL!S-bp3-006-R\u{ff0b}");
    let chika = game.id("PL!S-bp2-001-R"); // cost 9
    let dia = game.id("PL!S-bp2-004-R"); // cost 11 = 9 + 2
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [chika, yoshiko, -1];
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(dia);
    game.give_energy(15);

    game.activate_ability(yoshiko);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // chika (cost 9) sacrificed from LeftSide (stage[0]) → vacated
    // dia (cost 11 = 9+2) deployed to LeftSide (stage[0]) per "そのメンバーがいたエリア"
    assert_eq!(
        game.player().stage.stage[0],
        dia,
        "dia (cost 11 = chika cost 9 + 2) deployed to vacated LeftSide"
    );
    assert_eq!(
        game.player().stage.stage[1],
        yoshiko,
        "yoshiko stays at Center"
    );
    assert_eq!(game.player().stage.stage[2], -1, "RightSide stays empty");
    assert!(
        !game.player().waitroom.cards.contains(&dia),
        "dia should not be in discard (deployed to stage)"
    );
    assert!(
        game.player().waitroom.cards.contains(&chika),
        "chika should be in discard (sacrificed)"
    );
}

/// Variant: sacrifice from a full stage (3 members) → same_area still deploys to
/// the vacated slot, which is empty after the sacrifice.
/// Tests that other members remain undisturbed.
#[test]
fn yoshiko_sacrifice_from_full_stage_other_members_undisturbed() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let yoshiko = game.id("PL!S-bp3-006-R\u{ff0b}");
    let chika = game.id("PL!S-bp2-001-R");
    let riko = game.id("PL!S-bp2-002-R");
    let dia = game.id("PL!S-bp2-004-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [chika, yoshiko, riko];
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(dia);
    game.give_energy(15);

    game.activate_ability(yoshiko);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // chika sacrificed from Left → Left vacated → dia deploys to Left
    // riko at Right stays unchanged
    assert_eq!(
        game.player().stage.stage[0],
        dia,
        "dia deploys to vacated LeftSide"
    );
    assert_eq!(
        game.player().stage.stage[1],
        yoshiko,
        "yoshiko stays at Center"
    );
    assert_eq!(
        game.player().stage.stage[2],
        riko,
        "riko stays at RightSide"
    );
}

/// Once per turn: second activation fails.
#[test]
fn yoshiko_once_per_turn_second_activation_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let yoshiko = game.id("PL!S-bp3-006-R\u{ff0b}");
    let chika = game.id("PL!S-bp2-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [chika, yoshiko, -1];
    // Two cards in hand for two discard costs
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(15);

    // First activation should succeed
    let r1 = game.try_activate_ability(yoshiko);
    assert!(r1.is_ok(), "first activation should succeed");
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Second activation should fail (use_limit: 1)
    let r2 = game.try_activate_ability(yoshiko);
    assert!(r2.is_err(), "second activation should fail (once per turn)");
}

/// Empty hand → cost fails (can't discard).
#[test]
fn yoshiko_empty_hand_cost_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let yoshiko = game.id("PL!S-bp3-006-R\u{ff0b}");
    let chika = game.id("PL!S-bp2-001-R");

    game.state.player1.stage.stage = [chika, yoshiko, -1];
    // No cards in hand → cost can't be paid
    game.give_energy(15);

    game.activate_ability(yoshiko);
    // Cost resolution should fail silently (not crash)
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Stage should be unchanged (cost not paid, effect not executed)
    assert_eq!(
        game.player().stage.stage,
        [chika, yoshiko, -1],
        "stage unchanged — cost couldn't be paid"
    );
}

/// No matching cost card in discard → action 1 has no valid target → ends cleanly.
#[test]
fn yoshiko_no_matching_cost_in_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let yoshiko = game.id("PL!S-bp3-006-R\u{ff0b}");
    let chika = game.id("PL!S-bp2-001-R"); // cost 9 → need cost 11
    let wrong_cost = game.id("PL!S-bp2-002-R"); // cost 4 ≠ 11
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [chika, yoshiko, -1];
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(wrong_cost);
    game.give_energy(15);

    game.activate_ability(yoshiko);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // chika was sacrificed, but no cost-11 Aqours in discard → nothing summoned
    assert!(
        game.player().waitroom.cards.contains(&chika),
        "chika should be in discard (sacrificed)"
    );
    assert!(
        game.player().waitroom.cards.contains(&wrong_cost),
        "wrong-cost member should remain in discard"
    );
    assert_eq!(
        game.player().stage.stage,
        [-1, yoshiko, -1],
        "only yoshiko on stage"
    );
}
