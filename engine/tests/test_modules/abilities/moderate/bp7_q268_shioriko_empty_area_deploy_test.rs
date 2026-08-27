/// Q268 — 三船栞子 PL!N-bp7-010-R ab#0 (起動, ターン1回).
///
/// 起動：エネルギー置き場にあるエネルギー1枚をこのメンバーの下に置く：自分の控え室から
/// コスト2以下の『虹ヶ咲』のメンバーカードを1枚、メンバーのいないエリアにウェイト状態で
/// 登場させる。（この効果で登場したメンバーのいるエリアには、このターンにメンバーは登場できない。）
///
/// Official QA Q268: メンバーのいないエリアがない場合でも、のコストのみを支払うことはできますか？
/// → はい。できます。  The cost (place 1 energy under self) is payable INDEPENDENTLY of whether
/// the effect can resolve. With NO empty area, the cost is still paid and the deploy fizzles.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const SHIORIKO: &str = "PL!N-bp7-010-R"; // 三船栞子, R3BIRTH — the activated member (cost 9)
const TARGET_OK: &str = "PL!N-bp7-013-N"; // 上原歩夢, 虹ヶ咲, cost 2 → deployable
const TARGET_OK2: &str = "PL!N-sd2-022-SD2"; // 三船栞子, 虹ヶ咲, cost 2 → deployable
const TOO_EXPENSIVE: &str = "PL!N-bp1-013-PR"; // 上原歩夢, 虹ヶ咲, cost 4 → excluded
const WRONG_GROUP: &str = "PL!-sd1-002-SD"; // 絢瀬 絵里, μ's, cost 2 → NOT 虹ヶ咲 → excluded
const FILLER: &str = "PL!-sd1-010-SD"; // 高坂 穂乃果, ability-free

fn on_stage(game: &TestGame, area: MemberArea, id: i16) -> bool {
    game.state.player1.stage.get_area(area) == Some(id)
}

fn energy_under_center(game: &TestGame) -> usize {
    game.state.player1.stage.get_under_cards(MemberArea::Center).len()
}

/// Put 三船栞子 on p1 center with `energy` active energy. Returns the shioriko id.
fn setup(game: &mut TestGame, energy: usize) -> i16 {
    let shioriko = game.id(SHIORIKO);
    game.state.player1.stage.stage[1] = shioriko;
    game.give_energy(energy);
    shioriko
}

/// Activate 三船栞子's 起動 ability and drive cost + target + empty-area choices.
fn run_activate(game: &mut TestGame, shioriko: i16) {
    game.activate_ability(shioriko);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 30 {
        guard += 1;
        let choice = game.get_pending_choice().clone();
        match choice {
            // Cost: select the energy card to place under self.
            rabuka_engine::ability::types::Choice::SelectCard { zone, .. }
                if zone == "energy_zone" =>
            {
                game.select_indices(&[0]);
            }
            // Pay/skip optional cost.
            rabuka_engine::ability::types::Choice::SelectTarget { .. } => {
                game.select_choice_option(1); // pay
            }
            // Select the 虹ヶ咲 member from waitroom, or the destination area.
            rabuka_engine::ability::types::Choice::SelectCard { .. } => {
                game.select_indices(&[0]);
            }
            _ => {
                game.select_choice_option(0);
            }
        }
    }
}

// ====================================================================
// QA core: NO empty area → cost is STILL paid, no deploy.
// ====================================================================
#[test]
fn q268_no_empty_area_pays_cost_no_deploy() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let shioriko = setup(&mut game, 3);
    let target = game.id(TARGET_OK);
    game.add_to_discard(target);
    // Fill all 3 stage areas (left + center shioriko + right) so there is NO empty area.
    let left = game.id(FILLER);
    let right = game.id(FILLER);
    game.state.player1.stage.stage[0] = left;
    game.state.player1.stage.stage[2] = right;

    run_activate(&mut game, shioriko);

    // Q268: cost paid (1 energy under self) even with no empty area.
    assert_eq!(
        energy_under_center(&game),
        1,
        "Q268: cost (1 energy under 三船栞子) is paid even with NO empty area"
    );
    // The deploy fizzles — target stays in waitroom.
    assert!(
        game.state.player1.waitroom.cards.contains(&target),
        "Q268: no empty area → 虹ヶ咲 member is NOT deployed, stays in waitroom"
    );
    assert!(
        !on_stage(&game, MemberArea::LeftSide, target)
            && !on_stage(&game, MemberArea::Center, target)
            && !on_stage(&game, MemberArea::RightSide, target),
        "Q268: member not placed anywhere"
    );
}

// ====================================================================
// Deploy to the one empty area, in WAIT state.
// ====================================================================
#[test]
fn q268_deploy_to_empty_area_in_wait_state() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let shioriko = setup(&mut game, 3);
    let target = game.id(TARGET_OK);
    game.add_to_discard(target);
    // Fill left; only RIGHT is empty.
    let left = game.id(FILLER);
    game.state.player1.stage.stage[0] = left;

    run_activate(&mut game, shioriko);

    assert!(
        on_stage(&game, MemberArea::RightSide, target),
        "虹ヶ咲 member should deploy to the only empty area (right)"
    );
    assert_eq!(
        energy_under_center(&game),
        1,
        "cost energy placed under 三船栞子"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&target),
        "deployed member leaves the waitroom"
    );
    // Deployed member is in WAIT state.
    assert_eq!(
        game.state.mods.get_orientation_modifier(target).as_deref(),
        Some("wait"),
        "member deployed by this effect is in WAIT state"
    );
}

// ====================================================================
// Target filter: 虹ヶ咲 but cost > 2 → no selectable target → no deploy.
// ====================================================================
#[test]
fn q268_too_expensive_target_no_deploy() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let shioriko = setup(&mut game, 3);
    let tgt = game.id(TOO_EXPENSIVE);
    game.add_to_discard(tgt);
    // Right empty so a deploy WOULD be possible if a target matched.
    let left = game.id(FILLER);
    game.state.player1.stage.stage[0] = left;

    run_activate(&mut game, shioriko);

    // No valid target (cost 4 > 2) → nothing deployed.
    assert!(
        game.state.player1.waitroom.cards.contains(&tgt),
        "cost-4 虹ヶ咲 member is NOT a valid target and stays in waitroom"
    );
    assert!(
        !on_stage(&game, MemberArea::RightSide, tgt),
        "no deploy of a cost-4 member"
    );
    // Cost still paid even though the effect finds no target.
    assert_eq!(
        energy_under_center(&game),
        1,
        "cost still paid when no valid deploy target exists"
    );
}

// ====================================================================
// Target filter: cost ≤ 2 but NOT 虹ヶ咲 → excluded.
// ====================================================================
#[test]
fn q268_wrong_group_target_no_deploy() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let shioriko = setup(&mut game, 3);
    let tgt = game.id(WRONG_GROUP);
    game.add_to_discard(tgt);
    let left = game.id(FILLER);
    game.state.player1.stage.stage[0] = left;

    run_activate(&mut game, shioriko);

    assert!(
        game.state.player1.waitroom.cards.contains(&tgt),
        "μ's cost-2 member is NOT a 虹ヶ咲 target and stays in waitroom"
    );
    assert!(
        !on_stage(&game, MemberArea::RightSide, tgt),
        "non-虹ヶ咲 member not deployed"
    );
    assert_eq!(energy_under_center(&game), 1, "cost still paid");
}

// ====================================================================
// Two valid targets → exactly one deployed.
// ====================================================================
#[test]
fn q268_two_valid_targets_deploy_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let shioriko = setup(&mut game, 3);
    let t1 = game.id(TARGET_OK);
    let t2 = game.id(TARGET_OK2);
    game.add_to_discard(t1);
    game.add_to_discard(t2);
    // Both left and right empty (only center occupied by shioriko).
    run_activate(&mut game, shioriko);

    let deployed = [MemberArea::LeftSide, MemberArea::RightSide]
        .iter()
        .filter(|&&a| on_stage(&game, a, t1) || on_stage(&game, a, t2))
        .count();
    assert_eq!(
        deployed, 1,
        "exactly one 虹ヶ咲 cost-2 member is deployed from two valid targets"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        1,
        "one target remains in waitroom"
    );
    assert_eq!(energy_under_center(&game), 1, "cost paid");
}

// ====================================================================
// ターン1回 use limit: second activation in the same turn is blocked.
// ====================================================================
#[test]
fn q268_turn_limit_blocks_second_activation() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let shioriko = setup(&mut game, 3);
    let t1 = game.id(TARGET_OK);
    let t2 = game.id(TARGET_OK2);
    game.add_to_discard(t1);
    game.add_to_discard(t2);
    let left = game.id(FILLER);
    game.state.player1.stage.stage[0] = left;

    // First activation succeeds (right is empty).
    run_activate(&mut game, shioriko);

    // Second activation same turn must be blocked (ターン1回).
    let result = game.try_activate_ability(shioriko);
    assert!(
        result.is_err(),
        "ターン1回: second activation in the same turn must be blocked (got {:?})",
        result
    );
}

// ====================================================================
// No valid target AND no empty area → cost still paid, nothing happens.
// ====================================================================
#[test]
fn q268_no_target_no_empty_area_cost_still_paid() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let shioriko = setup(&mut game, 3);
    let tgt = game.id(WRONG_GROUP);
    game.add_to_discard(tgt);
    // Fill all areas so there's no empty slot at all.
    let left = game.id(FILLER);
    let right = game.id(FILLER);
    game.state.player1.stage.stage[0] = left;
    game.state.player1.stage.stage[2] = right;

    run_activate(&mut game, shioriko);

    // Cost paid regardless; no deploy.
    assert_eq!(
        energy_under_center(&game),
        1,
        "cost is paid even when there is neither a target nor an empty area (Q268)"
    );
    assert_eq!(
        game.state.player1.stage.stage[0],
        left,
        "left area unchanged"
    );
    assert_eq!(
        game.state.player1.stage.stage[2],
        right,
        "right area unchanged"
    );
}
