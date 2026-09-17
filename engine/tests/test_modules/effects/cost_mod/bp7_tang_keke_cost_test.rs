/// BP07 唐 可可 PL!SP-bp7-002-R ab#0 (常時).
///
/// 常時 自分のエネルギーが7枚以上あり、かつ自分のエネルギーが相手より多いかぎり、
/// ステージにいるこのメンバーのコストを＋２する。
///
/// Constant: as long as your energy is >= 7 AND your energy is MORE than the
/// opponent's, this member's cost is +2.
///
/// Rules under test:
///  - The +2 applies at most ONCE per copy (no double-application).
///  - The "more energy than opponent" gate: when energies are equal (or self is
///    lower), the +2 must NOT apply even if self >= 7.
///  - Mirror match: each side evaluates its OWN condition, so only the side with
///    more energy gets +2; the lower side gets none.
///  - The effective (modified) cost is what reduces the energy needed to baton
///    pass a new member onto the zone: 唐可可 base cost 2 → +2 = 4, so a 9-cost
///    card needs 9 - 4 = 5 energy (not 9 - 2 = 7).
use crate::helpers::*;
use rabuka_engine::game_setup::{generate_possible_actions, ActionType};
use rabuka_engine::zones::MemberArea;

const TANG: &str = "PL!SP-bp7-002-R"; // 唐 可可, base cost 2
const ENERGY: &str = "LL-E-001-SD";
// A 9-cost member used for baton-pass cost arithmetic.
const COST9: &str = "PL!-sd1-006-SD"; // 西木野 真姫, cost 9

fn cost_mod(game: &TestGame, id: i16) -> i32 {
    game.state.mods.get_cost_modifier(id)
}

/// Place 唐可可 on P1's center, set P1's active energy count, recalculate.
fn place_tang(game: &mut TestGame, p1_energy: u32) -> i16 {
    let t = game.id(TANG);
    game.state.player1.stage.stage = [-1, t, -1];
    for _ in 0..p1_energy {
        game.state.player1.energy_zone.cards.push(game.id(ENERGY));
    }
    game.state.player1.energy_zone.add_active(p1_energy as u8);
    game.state.recalculate_constants();
    t
}

// ═════════════════════════════════════════════════════════════════════════
// Single-copy application + the "more energy" gate
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn tang_condition_met_applies_plus2_once() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    // P2 has 0 energy, so P1 (7) has more → +2 applies exactly once.
    let t = place_tang(&mut game, 7);
    assert_eq!(cost_mod(&game, t), 2, "7 >= 7 and 7 > 0 → cost +2 exactly once");
}

#[test]
fn tang_more_energy_high_margin_applies_once() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let t = place_tang(&mut game, 10);
    assert_eq!(cost_mod(&game, t), 2, "10 >= 7 and 10 > 0 → +2 exactly once");
}

#[test]
fn tang_equal_energy_denies_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    // P1 and P2 both have 7 → self is NOT more than opp → no +2.
    let t = game.id(TANG);
    game.state.player1.stage.stage = [-1, t, -1];
    for _ in 0..7 {
        game.state.player1.energy_zone.cards.push(game.id(ENERGY));
        game.state.player2.energy_zone.cards.push(game.id(ENERGY));
    }
    game.state.player1.energy_zone.add_active(7);
    game.state.player2.energy_zone.add_active(7);
    game.state.recalculate_constants();
    assert_eq!(cost_mod(&game, t), 0, "7 == 7 (not more) → no +2");
}

#[test]
fn tang_less_energy_denies_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    // P1 has 7, P2 has 9 → P1 is NOT more → no +2.
    let t = game.id(TANG);
    game.state.player1.stage.stage = [-1, t, -1];
    for _ in 0..7 {
        game.state.player1.energy_zone.cards.push(game.id(ENERGY));
    }
    for _ in 0..9 {
        game.state.player2.energy_zone.cards.push(game.id(ENERGY));
    }
    game.state.player1.energy_zone.add_active(7);
    game.state.player2.energy_zone.add_active(9);
    game.state.recalculate_constants();
    assert_eq!(cost_mod(&game, t), 0, "7 < 9 (not more) → no +2");
}

#[test]
fn tang_below_seven_denies_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let t = place_tang(&mut game, 6);
    assert_eq!(cost_mod(&game, t), 0, "6 < 7 → no +2");
}

// ═════════════════════════════════════════════════════════════════════════
// Mirror match — only the side with more energy gets +2
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn tang_mirror_only_more_energy_side_gets_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let t1 = game.id(TANG);
    let t2 = game.new_id(TANG);
    game.state.player1.stage.stage = [-1, t1, -1];
    game.state.player2.stage.stage = [-1, t2, -1];
    // P1: 8 energy, P2: 5 energy → P1 has more, P2 does not.
    for _ in 0..8 {
        game.state.player1.energy_zone.cards.push(game.id(ENERGY));
    }
    for _ in 0..5 {
        game.state.player2.energy_zone.cards.push(game.id(ENERGY));
    }
    game.state.player1.energy_zone.add_active(8);
    game.state.player2.energy_zone.add_active(5);
    game.state.recalculate_constants();
    assert_eq!(cost_mod(&game, t1), 2, "P1 has more energy → +2");
    assert_eq!(cost_mod(&game, t2), 0, "P2 has less energy → no +2");
}

#[test]
fn tang_mirror_equal_energy_neither_gets_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let t1 = game.id(TANG);
    let t2 = game.new_id(TANG);
    game.state.player1.stage.stage = [-1, t1, -1];
    game.state.player2.stage.stage = [-1, t2, -1];
    for _ in 0..7 {
        game.state.player1.energy_zone.cards.push(game.id(ENERGY));
        game.state.player2.energy_zone.cards.push(game.id(ENERGY));
    }
    game.state.player1.energy_zone.add_active(7);
    game.state.player2.energy_zone.add_active(7);
    game.state.recalculate_constants();
    assert_eq!(cost_mod(&game, t1), 0, "P1 == P2 energy → no +2");
    assert_eq!(cost_mod(&game, t2), 0, "P2 == P1 energy → no +2");
}

// ═════════════════════════════════════════════════════════════════════════
// Modified cost affects baton-pass cost (9 - 4 = 5 energy)
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn tang_modified_cost_reduces_baton_pass_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    // 唐可可 on center with +2 (7 vs 0 energy) → effective cost 4.
    let t = place_tang(&mut game, 7);
    assert_eq!(cost_mod(&game, t), 2, "setup: +2 modifier present");

    // A 9-cost member in hand; baton pass onto 唐可可's center slot.
    let nine = game.id(COST9);
    game.add_to_hand(nine);
    // Exactly 5 active energy (enough for 9 - 4 = 5), so baton pass must succeed.
    game.state.player1.energy_zone.set_active_count(5);

    let err = game.try_play_to_stage(nine, MemberArea::Center);
    assert!(err.is_ok(), "9 - 4 = 5 energy is enough to baton pass; got: {:?}", err);
}

#[test]
fn tang_baton_pass_fails_without_enough_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let t = place_tang(&mut game, 7);
    assert_eq!(cost_mod(&game, t), 2, "setup: +2 modifier present");

    let nine = game.id(COST9);
    game.add_to_hand(nine);
    // Only 4 active energy → need 5, so baton pass must FAIL.
    game.state.player1.energy_zone.set_active_count(4);

    let err = game.try_play_to_stage(nine, MemberArea::Center);
    assert!(err.is_err(), "9 - 4 = 5 energy needed, only 4 → must fail; got: {:?}", err);
}

// ═════════════════════════════════════════════════════════════════════════
// Recalculation idempotency + live flip of the condition
// ═════════════════════════════════════════════════════════════════════════

/// Re-running recalculate_constants must NOT stack the +2 (must stay 2, not 4).
#[test]
fn tang_recalculate_is_idempotent() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let t = place_tang(&mut game, 7);
    assert_eq!(cost_mod(&game, t), 2, "first recalc → +2");
    game.state.recalculate_constants();
    game.state.recalculate_constants();
    assert_eq!(cost_mod(&game, t), 2, "repeated recalc must not double → still +2");
}

/// The +2 must turn OFF once the opponent catches up (condition is re-evaluated).
#[test]
fn tang_plus2_turns_off_when_opponent_catches_up() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let t = place_tang(&mut game, 7);
    assert_eq!(cost_mod(&game, t), 2, "7 vs 0 → +2");
    // Opponent goes to 9 → self no longer has more energy.
    for _ in 0..9 {
        game.state.player2.energy_zone.cards.push(game.id(ENERGY));
    }
    game.state.player2.energy_zone.add_active(9);
    game.state.recalculate_constants();
    assert_eq!(cost_mod(&game, t), 0, "7 < 9 → +2 removed");
}

/// Without the +2 (condition not met), the 9-cost baton pass onto 唐可可 (base
/// cost 2) needs 9 - 2 = 7 energy, so 5 is NOT enough → confirms the +2 is what
/// lowers it to 5.
#[test]
fn tang_baton_pass_needs_base_cost_without_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    // 唐可可 on center with only 6 energy → condition (>=7) fails → no +2.
    let t = place_tang(&mut game, 6);
    assert_eq!(cost_mod(&game, t), 0, "setup: no +2 modifier");

    let nine = game.id(COST9);
    game.add_to_hand(nine);
    // 5 active energy: enough for the +2 case (5), but NOT for the base 7.
    game.state.player1.energy_zone.set_active_count(5);

    let err = game.try_play_to_stage(nine, MemberArea::Center);
    assert!(err.is_err(), "base cost 2 needs 9-2=7 energy, 5 not enough → fail; got: {:?}", err);
}

// ═════════════════════════════════════════════════════════════════════════
// Action gate (game_setup) + paid-energy checks
// ═════════════════════════════════════════════════════════════════════════
/// Find the PlayMemberToStage action for `card_id` in the generated gate.
fn gate_action_for(game: &TestGame, card_id: i16) -> rabuka_engine::game_setup::Action {
    generate_possible_actions(&game.state)
        .into_iter()
        .find(|a| {
            a.action_type == ActionType::PlayMemberToStage
                && a.parameters.as_ref().and_then(|p| p.card_id) == Some(card_id)
        })
        .expect("PlayMemberToStage action must be offered")
}

/// The UI gate must offer the DISCOUNTED baton cost (9 - 4 = 5), not printed 7.
#[test]
fn tang_gate_offers_discounted_baton_cost() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let t = place_tang(&mut game, 7);
    assert_eq!(cost_mod(&game, t), 2, "setup: +2 modifier present");

    let nine = game.id(COST9);
    game.add_to_hand(nine);

    let action = gate_action_for(&game, nine);
    let areas = action.parameters.as_ref().and_then(|p| p.available_areas.as_ref()).expect("areas present");
    let center = areas.iter().find(|a| a.area == "center").expect("center area present");
    assert!(center.available && center.is_baton_touch, "center baton must be available");
    assert_eq!(center.cost, 5, "gate must show discounted 9-4=5, not printed 9-2=7");
    // Mini buttons / headers read final_cost first — must carry the area price.
    assert_eq!(
        action.parameters.as_ref().and_then(|p| p.final_cost),
        Some(5),
        "action final_cost must be the discounted 5 so .btn.action-btn.mini shows it"
    );
}

/// Without the bonus (equal energy → condition fails) the gate shows base 9-2=7.
#[test]
fn tang_gate_without_bonus_offers_base_cost() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let t = game.id(TANG);
    game.state.player1.stage.stage = [-1, t, -1];
    for _ in 0..7 {
        game.state.player1.energy_zone.cards.push(game.id(ENERGY));
        game.state.player2.energy_zone.cards.push(game.id(ENERGY));
    }
    game.state.player1.energy_zone.add_active(7);
    game.state.player2.energy_zone.add_active(7);
    game.state.recalculate_constants();
    assert_eq!(cost_mod(&game, t), 0, "setup: no +2 (equal energy)");

    let nine = game.id(COST9);
    game.add_to_hand(nine);

    let action = gate_action_for(&game, nine);
    let areas = action.parameters.as_ref().and_then(|p| p.available_areas.as_ref()).expect("areas present");
    let center = areas.iter().find(|a| a.area == "center").expect("center area present");
    assert_eq!(center.cost, 7, "gate must show base 9-2=7 without bonus");
    assert_eq!(
        action.parameters.as_ref().and_then(|p| p.final_cost),
        Some(7),
        "action final_cost must be base 7 without bonus"
    );
}

/// Paid energy must equal the discounted price: 5 active → 0 remaining.
#[test]
fn tang_baton_pays_discounted_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let t = place_tang(&mut game, 7);
    assert_eq!(cost_mod(&game, t), 2, "setup: +2 modifier present");

    let nine = game.id(COST9);
    game.add_to_hand(nine);
    game.state.player1.energy_zone.set_active_count(5);
    assert_eq!(game.state.player1.energy_zone.active_count(), 5);

    game.try_play_to_stage(nine, MemberArea::Center).expect("9-4=5 must succeed");
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "paid exactly 5 (discounted), not 7"
    );
    assert!(game.state.player1.waitroom.cards.contains(&t), "Keke baton-touched to waitroom");
    assert_eq!(game.state.player1.stage.stage[1], nine, "9-cost occupies center");
}

/// 唐可可's own play cost from hand is unaffected (base 2): the +2 applies only
/// once she is on stage, so deploying her to an empty area costs 2 and the
/// gate offers 2.
#[test]
fn tang_own_play_cost_unaffected() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    game.give_energy(7);
    game.state.recalculate_constants();

    let t = game.id(TANG);
    game.add_to_hand(t);

    let action = gate_action_for(&game, t);
    let areas = action.parameters.as_ref().and_then(|p| p.available_areas.as_ref()).expect("areas present");
    let center = areas.iter().find(|a| a.area == "center").expect("center area present");
    assert!(center.available && !center.is_baton_touch, "empty center: normal play");
    assert_eq!(center.cost, 2, "own play cost is base 2, not +2");

    game.try_play_to_stage(t, MemberArea::Center).expect("play Keke for base cost");
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        5,
        "paid base 2 from 7 → 5 remaining"
    );
}
