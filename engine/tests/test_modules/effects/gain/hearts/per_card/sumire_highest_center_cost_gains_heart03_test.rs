/// 平安名すみれ PL!SP-bp2-004-R:
//
///
/// {{jyouji.png|常時}}自分のステージにいるメンバーのうち、センターエリアにいる
/// メンバーが最も大きいコストを持つ場合、{{heart_03.png|heart03}}を得る。
///
/// (Constant: if the member in your CENTRE area has the highest cost among the
/// members on your stage, gain heart03.)
///
/// The three tests this replaces were named for behaviour this card does not
/// have — "baton with energy7 places two wait", "energy10 blade threshold",
/// "baton without liella no energy place" — and every one of them was a
/// tautology that could not fail:
///
///   * `assert!(blade >= 0, …)` — true for any value, and this card grants no
///     blade at all.
///   * `assert!(b10 >= b9, …)` — monotonic, so it passes when both are 0.
///   * `assert!(energy_zone.cards.len() >= before)` — nothing had reduced it.
///
/// There is no 起動, no energy cost and no waited-energy placement on this print,
/// so no coverage was lost by replacing them: what is here now is the ability
/// the card actually prints.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

/// heart03 on the subject, after constants are recomputed.
fn heart03(game: &TestGame, cid: i16) -> i32 {
    game.state.mods.get_heart_modifier(cid, HeartColor::Heart03)
}

/// Give p1's three areas the given member ids.
fn stage(game: &mut TestGame, left: i16, center: i16, right: i16) {
    game.state.player1.stage.stage = [left, center, right];
    game.state.recalculate_constants();
}

/// The centre member has the highest cost → heart03 applies. The members are
/// three genuinely different cards whose costs are pinned, so "highest cost"
/// is being decided by the fixture and not by accident.
#[test]
fn center_has_highest_cost_gains_heart03() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let sumire = g.id("PL!SP-bp2-004-R"); // cost 9
    g.assert_card_identity(sumire, "PL!SP-bp2-004-R");
    g.assert_card_cost(sumire, 9);
    let left = g.id("PL!HS-PR-001-PR"); // cost 10
    let right = g.id("PL!-sd1-010-SD"); // cost 4
    g.assert_card_cost(left, 10);
    g.assert_card_cost(right, 4);

    // Centre is the 9-cost card, left is 10 → centre is NOT the highest.
    stage(&mut g, left, sumire, right);
    assert_eq!(
        heart03(&g, sumire),
        0,
        "left (cost 10) outranks centre (cost 9) → no heart03"
    );

    // Move the cost-10 card into the centre: now the centre IS the highest.
    stage(&mut g, right, left, sumire);
    assert_eq!(
        heart03(&g, left),
        0,
        "平安名すみれ's own 常時 is what grants heart03, so the cost-10 card in \
         the centre does not itself gain it"
    );
    stage(&mut g, sumire, left, right);
    assert_eq!(
        heart03(&g, sumire),
        1,
        "センター_areaのメンバーが最も大きいコストを持つ場合 → heart03を得る"
    );
}

/// A tie at the top must NOT count as "the centre has the highest cost":
/// the printed rule says 最も大きい, and a tie means two members share it.
#[test]
fn tie_for_highest_cost_does_not_grant_heart03() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let tie = g.new_id("PL!HS-PR-001-PR"); // cost 10
    let other_tie = g.new_id("PL!HS-PR-001-PR"); // cost 10
    let low = g.id("PL!-sd1-010-SD"); // cost 4
    g.assert_card_cost(tie, 10);
    g.assert_card_cost(other_tie, 10);
    g.assert_same_card_name(tie, other_tie, "two copies of the cost-10 print");
    assert_ne!(tie, other_tie, "two separate instances");

    stage(&mut g, tie, other_tie, low);
    assert_eq!(
        heart03(&g, other_tie),
        0,
        "the centre shares the highest cost with another member → 最も大きい is \
         not satisfied, so no heart03"
    );
    assert_eq!(
        heart03(&g, tie),
        0,
        "…and the 常時 grants to itself only, never to the tied peer"
    );
}
