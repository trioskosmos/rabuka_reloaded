/// Tests for PL!SP-bp2-004-R (平安名すみれ) ab#0
/// 常時: 自分のステージにいるメンバーのうち、センターエリアにいるメンバーが最も大きいコストを持つ場合、heart03を得る。
/// Constant: if the center member has the highest cost among your stage members, gain heart03.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

fn game_id(card_no: &str) -> i16 {
    let db = load_real_database();
    card_id(&db, card_no)
}

fn heart03_mod(game: &TestGame, card_id: i16) -> i32 {
    game.state
        .mods
        .get_heart_modifier(card_id, HeartColor::Heart03)
}

fn setup_and_check(
    _sumire_pos: usize,
    sumire_id: i16,
    center_id: i16,
    right_id: i16,
) -> (i32, i32) {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mut stage = [-1i16; 3];
    stage[0] = sumire_id;
    stage[1] = center_id;
    if right_id != -1 {
        stage[2] = right_id;
    }
    game.state.player1.stage.stage = stage;

    let before = heart03_mod(&game, sumire_id);
    game.state.recalculate_constants();
    let after = heart03_mod(&game, sumire_id);
    (before, after)
}

// ======================================================================
// PL!SP-bp2-004-R: cost=9
// PL!SP-pb1-001-R: cost=11
// PL!HS-PR-001-PR: cost=10
// PL!SP-bp5-111-R: cost=8
// PL!N-PR-021-PR: cost=7
// PL!-PR-005-PR: cost=9
// PL!-sd1-010-SD: cost=4
// ======================================================================

/// Center cost=11 > left=9 (sumire) and right=4 → condition passes → sumire gains heart03.
#[test]
fn center_highest_cost_gains_heart() {
    let sumire = game_id("PL!SP-bp2-004-R"); // cost=9
    let center_high = game_id("PL!SP-pb1-001-R"); // cost=11
    let right_low = game_id("PL!-sd1-010-SD"); // cost=4
    let (before, after) = setup_and_check(0, sumire, center_high, right_low);
    assert_eq!(
        after - before,
        1,
        "Center(11) > Sumire(9) and Right(4) → should gain +1 heart03"
    );
}

/// Center cost=7 < left=9 (sumire) → condition fails → no heart03.
#[test]
fn center_lower_cost_no_heart() {
    let sumire = game_id("PL!SP-bp2-004-R"); // cost=9
    let center_low = game_id("PL!N-PR-021-PR"); // cost=7
    let right = game_id("PL!-sd1-010-SD"); // cost=4
    let (before, after) = setup_and_check(0, sumire, center_low, right);
    assert_eq!(
        after, before,
        "Center(7) < Sumire(9) → should NOT gain heart03"
    );
}

/// Center cost=9 == left=9 (sumire) → strict > fails → no heart03.
#[test]
fn center_equal_cost_no_heart() {
    let sumire = game_id("PL!SP-bp2-004-R"); // cost=9
    let center_equal = game_id("PL!-PR-005-PR"); // cost=9
    let right = game_id("PL!-sd1-010-SD"); // cost=4
    let (before, after) = setup_and_check(0, sumire, center_equal, right);
    assert_eq!(
        after, before,
        "Center(9) == Sumire(9) → strict > means no heart03"
    );
}

/// Center cost=11 > left=9, right=10 → center is highest → gains heart03.
#[test]
fn center_highest_among_three_different_costs() {
    let sumire = game_id("PL!SP-bp2-004-R"); // cost=9
    let center_high = game_id("PL!SP-pb1-001-R"); // cost=11
    let right_high = game_id("PL!HS-PR-001-PR"); // cost=10
    let (before, after) = setup_and_check(0, sumire, center_high, right_high);
    assert_eq!(
        after - before,
        1,
        "Center(11) > Sumire(9) and Right(10) → should gain +1 heart03"
    );
}

/// Center cost=8 < left=9 < right=10 → fails.
#[test]
fn center_not_highest_right_is() {
    let sumire = game_id("PL!SP-bp2-004-R"); // cost=9
    let center = game_id("PL!SP-bp5-111-R"); // cost=8
    let right = game_id("PL!HS-PR-001-PR"); // cost=10
    let (before, after) = setup_and_check(0, sumire, center, right);
    assert_eq!(
        after, before,
        "Center(8) < Sumire(9) < Right(10) → should NOT gain heart03"
    );
}

/// Only two members: sumire(left)=9, center=11 → center is highest → gains heart03.
#[test]
fn center_highest_two_members_only() {
    let sumire = game_id("PL!SP-bp2-004-R"); // cost=9
    let center_high = game_id("PL!SP-pb1-001-R"); // cost=11
    let (before, after) = setup_and_check(0, sumire, center_high, -1);
    assert_eq!(
        after - before,
        1,
        "Center(11) > Sumire(9), right empty → should gain +1 heart03"
    );
}

/// Sumire IS at center with cost=9, left=4 → center is highest → gains heart03.
#[test]
fn sumire_at_center_highest_gains_heart() {
    let sumire = game_id("PL!SP-bp2-004-R"); // cost=9
    let left_low = game_id("PL!-sd1-010-SD"); // cost=4
    let right_low = game_id("PL!-sd1-010-SD"); // cost=4
    let mut stage = [-1i16; 3];
    stage[0] = left_low;
    stage[1] = sumire;
    stage[2] = right_low;
    let db = load_real_database();
    let mut game = TestGame::new(db);
    game.state.player1.stage.stage = stage;
    let before = heart03_mod(&game, sumire);
    game.state.recalculate_constants();
    let after = heart03_mod(&game, sumire);
    assert_eq!(
        after - before,
        1,
        "Sumire at center(9) > Left(4) and Right(4) → should gain +1 heart03"
    );
}

/// Center=11 > sumire(left)=9 > right=8 → center is highest → gains heart03.
#[test]
fn three_distinct_costs_center_highest() {
    let sumire = game_id("PL!SP-bp2-004-R"); // cost=9
    let center_high = game_id("PL!SP-pb1-001-R"); // cost=11
    let right_mid = game_id("PL!SP-bp5-111-R"); // cost=8
    let (before, after) = setup_and_check(0, sumire, center_high, right_mid);
    assert_eq!(
        after - before,
        1,
        "Center(11) > Sumire(9) > Right(8) → should gain +1 heart03"
    );
}

/// Center=9 == right=9, sumire(left)=9 → center is NOT strictly highest (tied) → no heart03.
#[test]
fn center_tied_with_right_no_heart() {
    let sumire = game_id("PL!SP-bp2-004-R"); // cost=9
    let center = game_id("PL!-PR-005-PR"); // cost=9
    let right = game_id("PL!-PR-005-PR"); // cost=9
    let (before, after) = setup_and_check(0, sumire, center, right);
    assert_eq!(
        after, before,
        "Center(9) == Right(9) → not strictly highest → no heart03"
    );
}

/// All three cost=9 → center is NOT strictly highest → no heart03.
#[test]
fn all_equal_cost_no_heart() {
    let sumire = game_id("PL!SP-bp2-004-R"); // cost=9
    let center = game_id("PL!-PR-005-PR"); // cost=9
    let right = game_id("PL!-PR-005-PR"); // cost=9
    let (before, after) = setup_and_check(0, sumire, center, right);
    assert_eq!(
        after, before,
        "All cost=9 → center is not strictly highest → no heart03"
    );
}
