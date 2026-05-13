/// Tests for 村野さやか (PL!HS-bp1-002-R) — Activation ability:
///
/// {{kidou.png|起動}}このメンバーをステージから控え室に置く：
/// 自分の控え室からコスト15以下の「蓮ノ空」のメンバーカードを1枚、
/// このメンバーが置かれていたエリアに登場させる。
///
/// Q63: Ability debuts don't require paying the card's cost
/// Q80: Can debut to the area vacated by this same ability this turn
use crate::helpers::*;
/// Q63: Debuting via ability effect does not require paying the card's
/// cost. Activate 村野さやか with a matching 蓮ノ空 member in discard,
/// verify it appears on stage without requiring extra energy.
#[test]
fn sayaka_q63_ability_debut_no_cost_payment() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sayaka = game.id("PL!HS-bp1-002-R");
    let hasuno_member = game.id("PL!HS-bp2-004-R"); // 藤島慈, cost=2, group=蓮ノ空

    // Stage: 村野さやか in Center
    game.state.player1.stage.stage[1] = sayaka;

    // Hand: nothing needed
    // Discard: a 蓮ノ空 member
    game.state.player1.waitroom.cards.push(hasuno_member);

    // Energy: 2 for the activation cost
    game.give_energy(2);

    game.activate_ability(sayaka);

    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.state.player1.energy_zone.active_energy_count, 0,
        "Q63: 2E should be fully consumed by activation cost"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&sayaka),
        "sayaka should be in waitroom after activation cost"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], hasuno_member,
        "Q63: 蓮ノ空 member should appear in sayaka's former area without cost payment"
    );
}

/// Q80: Can debut to an area that was vacated by this same ability this
/// turn. The self_cost puts sayaka in waitroom, freeing the area, and
/// the effect debuts a new member to the same area — all within one
/// ability activation.
#[test]
fn sayaka_q80_debut_to_vacated_area_same_turn() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sayaka = game.id("PL!HS-bp1-002-R");
    let hasuno_member = game.id("PL!HS-bp2-004-R");

    game.state.player1.stage.stage[1] = sayaka;
    game.state.player1.waitroom.cards.push(hasuno_member);
    game.give_energy(2);

    game.activate_ability(sayaka);

    if game.has_pending_choice() {
        // Select the only eligible card in discard
        game.select_indices(&[0]);
    }

    // The area vacated by sayaka should now be filled by the new member
    assert_eq!(
        game.state.player1.stage.stage[1], hasuno_member,
        "Q80: New member should debut to the vacated area (same_area)"
    );
    assert!(
        !game.state.player1.stage.stage.contains(&sayaka),
        "sayaka should no longer be on stage"
    );
}
