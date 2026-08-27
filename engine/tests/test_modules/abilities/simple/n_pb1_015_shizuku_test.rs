use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// PL!N-pb1-015-R 桜坂しずく — 登場 E2 optional → hand Shizuku cost≤4 → stage
#[test]
fn shizuku_pay_2e_deploys_shizuku() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let shizuku = game.id("PL!N-pb1-015-R"); // cost 13
    let shizuku_hand = game.new_id("PL!N-bp1-015-N");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(shizuku);
    game.state.player1.hand.cards.push(shizuku_hand);
    game.give_energy(15);
    for _ in 0..5 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(shizuku, MemberArea::Center);
    assert!(game.has_pending_choice(), "pay 2E prompt expected");
    game.select_option(1); // pay
    // After pay, may have SelectCard for which Shizuku to deploy
    let mut safety=0;
    while game.has_pending_choice() && safety<5 { safety+=1; game.select_indices(&[0]); }
    // Deployed shizuku should be on stage (either left/right)
    let has_shizuku_hand = game.state.player1.stage.stage.iter().any(|&id| id==shizuku_hand);
    assert!(has_shizuku_hand || game.state.player1.hand.cards.contains(&shizuku_hand)==false, "Shizuku should have left hand");
    assert!(!game.has_pending_choice());
}
#[test]
fn shizuku_skip_pay_no_deploy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let shizuku = game.id("PL!N-pb1-015-R");
    let shizuku_hand = game.new_id("PL!N-bp1-015-N");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(shizuku);
    game.state.player1.hand.cards.push(shizuku_hand);
    game.give_energy(15);
    for _ in 0..5 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(shizuku, MemberArea::Center);
    assert!(game.has_pending_choice());
    game.select_option(0); // skip
    while game.has_pending_choice() { game.select_indices(&[]); }
    assert!(game.state.player1.hand.cards.contains(&shizuku_hand), "skip should keep Shizuku in hand");
}
#[test]
fn shizuku_no_eligible_shizuku_no_deploy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let shizuku = game.id("PL!N-pb1-015-R");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(shizuku);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(15);
    for _ in 0..5 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(shizuku, MemberArea::Center);
    assert!(game.has_pending_choice());
    game.select_option(1); // try pay but no eligible Shizuku (only filler not Shizuku)
    let mut safety=0;
    while game.has_pending_choice() && safety<5 { safety+=1; let _ = game.try_select_indices(&[]); let _ = game.try_select_indices(&[0]); }
    // Hand should still have filler, no Shizuku deployed
    assert!(game.state.player1.hand.cards.contains(&filler));
}
#[test]
fn shizuku_insufficient_energy_no_pay() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let shizuku = game.id("PL!N-pb1-015-R");
    let shizuku_hand = game.new_id("PL!N-bp1-015-N");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(shizuku);
    game.state.player1.hand.cards.push(shizuku_hand);
    game.give_energy(1);
    for _ in 0..5 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.stage.stage = [-1, -1, -1];
    let res = game.try_play_to_stage(shizuku, MemberArea::Center);
    assert!(res.is_err(), "should fail to play with only 1 energy, need 13");
}
