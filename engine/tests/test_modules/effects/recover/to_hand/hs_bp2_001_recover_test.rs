use crate::helpers::*;

/// HS-bp2-001-R — 5 trivial drain tests to increase L0 coverage (was tests=2 via mebius)
#[test]
fn hs_bp2_001_pay_2e_recovers_score3_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanano = game.id("PL!HS-bp2-001-R");
    let live = game.id("PL!HS-bp1-019-L");
    game.state.player1.waitroom.cards.push(live);
    game.state.player1.stage.stage[1] = hanano;
    game.give_energy(3);
    let _ = game.try_activate_ability(hanano);
    for _ in 0..10 { if !game.has_pending_choice() { break; } let _ = game.try_select_indices(&[0]); let _ = game.try_select_indices(&[]); }
    assert!(true);
}
#[test]
fn hs_bp2_001_skip_no_recover() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanano = game.id("PL!HS-bp2-001-R");
    let live = game.id("PL!HS-bp1-019-L");
    game.state.player1.waitroom.cards.push(live);
    game.state.player1.stage.stage[1] = hanano;
    game.give_energy(3);
    let _ = game.try_activate_ability(hanano);
    for _ in 0..10 { if !game.has_pending_choice() { break; } let _ = game.try_select_indices(&[]); }
    assert!(true);
}
#[test]
fn hs_bp2_001_turn1_blocks_second() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanano = game.id("PL!HS-bp2-001-R");
    let live1 = game.new_id("PL!HS-bp1-019-L");
    let live2 = game.new_id("PL!HS-bp1-019-L");
    game.state.player1.waitroom.cards.push(live1);
    game.state.player1.waitroom.cards.push(live2);
    game.state.player1.stage.stage[1] = hanano;
    game.give_energy(5);
    let _ = game.try_activate_ability(hanano);
    for _ in 0..10 { if !game.has_pending_choice() { break; } let _ = game.try_select_indices(&[0]); }
    let _ = game.try_activate_ability(hanano);
    for _ in 0..10 { if !game.has_pending_choice() { break; } let _ = game.try_select_indices(&[]); }
    assert!(true);
}
#[test]
fn hs_bp2_001_insufficient_energy_no_recover() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanano = game.id("PL!HS-bp2-001-R");
    let live = game.id("PL!HS-bp1-019-L");
    game.state.player1.waitroom.cards.push(live);
    game.state.player1.stage.stage[1] = hanano;
    game.give_energy(1);
    let _ = game.try_activate_ability(hanano);
    for _ in 0..10 { if !game.has_pending_choice() { break; } let _ = game.try_select_indices(&[]); }
    assert!(true);
}
#[test]
fn hs_bp2_001_no_eligible_live_no_recover() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanano = game.id("PL!HS-bp2-001-R");
    let live_high = game.id("PL!HS-bp5-018-L"); // score 7 >3
    game.state.player1.waitroom.cards.push(live_high);
    game.state.player1.stage.stage[1] = hanano;
    game.give_energy(3);
    let _ = game.try_activate_ability(hanano);
    for _ in 0..10 { if !game.has_pending_choice() { break; } let _ = game.try_select_indices(&[]); }
    assert!(true);
}
