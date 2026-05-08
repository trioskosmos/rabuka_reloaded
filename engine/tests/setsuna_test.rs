/// Tests for 優木せつ菜 (PL!N-bp3-007-R) — Activation ability:
///
/// {{kidou.png|起動}}{{icon_energy.png|E}}{{icon_energy.png|E}}
/// このメンバーをステージから控え室に置く：
/// 自分の手札からコスト13以下の「優木せつ菜」のメンバーカードを1枚、
/// このメンバーが置かれていたエリアに登場させる。
/// その後、自分のエネルギー置き場にあるエネルギー1枚をこのメンバーの下に置く。
///
/// Q157: Wait-energy can be placed under member (any energy state works)
/// Q184: Energy under member does NOT count toward energy count

mod helpers;
use helpers::*;
/// Q157: Wait-energy can be placed under the member.
/// Cost paid with active energy; placement pops wait-energy from zone.
#[test]
fn setsuna_q157_wait_energy_can_be_placed_under_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let setsuna = game.id("PL!N-bp3-007-R");
    let target = game.id("PL!N-PR-009-PR"); // 優木せつ菜, cost=2

    game.state.player1.stage.stage[1] = setsuna;
    game.state.player1.hand.cards.push(target);

    // 2 active (for cost) + 1 wait (for placement)
    game.give_energy(2);
    game.state.player1.energy_zone.cards.push(game.id("LL-E-001-SD"));

    let total_before = game.state.player1.energy_zone.cards.len();
    let active_before = game.state.player1.energy_zone.active_energy_count;

    game.activate_ability(setsuna);
    if game.has_pending_choice() { game.select_indices(&[0]); }

    let total_after = game.state.player1.energy_zone.cards.len();
    let active_after = game.state.player1.energy_zone.active_energy_count;

    assert_eq!(active_before - active_after, 2, "2 active consumed for cost");
    assert_eq!(total_before - total_after, 1, "1 wait card popped for placement");
}

/// Q184: Energy placed under member is removed from the zone → not counted.
#[test]
fn setsuna_q184_energy_under_member_not_counted() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let setsuna = game.id("PL!N-bp3-007-R");
    let target = game.id("PL!N-PR-009-PR");

    game.state.player1.stage.stage[1] = setsuna;
    game.state.player1.hand.cards.push(target);
    game.give_energy(3);

    game.activate_ability(setsuna);
    if game.has_pending_choice() { game.select_indices(&[0]); }

    assert_eq!(game.state.player1.energy_zone.cards.len(), 2,
        "Q184: 1 card removed for placement, 2 remain (inactive)");
    assert_eq!(game.state.player1.energy_zone.active_energy_count, 1,
        "Q184: 2 active consumed, 1 remaining active");
}

/// The debut should target the SAME area the original card vacated, not
/// an arbitrary empty slot. Setsuna in Center works by accident (Center
/// is stage_first_empty's priority). Setsuna in RightSide would fail
/// without the same_area fix — the new card would land in Center instead.
#[test]
fn setsuna_debuts_to_vacated_area_not_stage_first_empty() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let setsuna = game.id("PL!N-bp3-007-R");
    let target = game.id("PL!N-PR-009-PR");

    // Setsuna on the RIGHT side — not Center
    game.state.player1.stage.stage = [-1, -1, setsuna];
    game.state.player1.hand.cards.push(target);
    game.give_energy(2);

    game.activate_ability(setsuna);
    if game.has_pending_choice() { game.select_indices(&[0]); }

    // After self_cost removes setsuna from RightSide, last_vacated_stage_area = Some(2).
    // same_area should place the new card at stage[2], NOT at stage[1] (Center).
    assert_eq!(game.state.player1.stage.stage[2], target,
        "New card should debut to RightSide (vacated area), not Center");
    assert!(!game.state.player1.stage.stage.contains(&setsuna),
        "Original setsuna is in waitroom, not on stage");
    assert!(game.state.player1.waitroom.cards.contains(&setsuna),
        "Original setsuna should be in waitroom");
}
