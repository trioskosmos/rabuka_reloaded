use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// PL!N-pb1-017 Miyashita Ai — 登場 E2 optional → hand Miyashita cost≤4 → stage
#[test]
fn miyashita_pay_2e_deploys() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let miya = game.id("PL!N-pb1-017-R");
    let miya_hand = game.new_id("PL!N-bp1-017-N");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(miya);
    game.state.player1.hand.cards.push(miya_hand);
    game.give_energy(15);
    for _ in 0..5 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(miya, MemberArea::Center);
    assert!(game.has_pending_choice(), "pay 2E prompt");
    game.select_option(1);
    let mut s=0; while game.has_pending_choice() && s<5 { s+=1; game.select_indices(&[0]); }
    assert!(game.state.player1.stage.stage.iter().any(|&id| id==miya_hand));
}
#[test]
fn miyashita_skip_no_deploy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let miya = game.id("PL!N-pb1-017-R");
    let miya_hand = game.new_id("PL!N-bp1-017-N");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(miya);
    game.state.player1.hand.cards.push(miya_hand);
    game.give_energy(15);
    for _ in 0..5 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(miya, MemberArea::Center);
    assert!(game.has_pending_choice());
    game.select_option(0);
    while game.has_pending_choice() { game.select_indices(&[]); }
    assert!(game.state.player1.hand.cards.contains(&miya_hand));
}
#[test]
fn miyashita_no_eligible_no_deploy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let miya = game.id("PL!N-pb1-017-R");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(miya);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(15);
    for _ in 0..5 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(miya, MemberArea::Center);
    assert!(game.has_pending_choice());
    game.select_option(1);
    let mut s=0; while game.has_pending_choice() && s<5 { s+=1; let _=game.try_select_indices(&[]); let _=game.try_select_indices(&[0]); }
    assert!(game.state.player1.hand.cards.contains(&filler));
}
#[test]
fn miyashita_insufficient_energy_cannot_play() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let miya = game.id("PL!N-pb1-017-R");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(miya);
    game.give_energy(1);
    for _ in 0..5 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.stage.stage = [-1, -1, -1];
    let res = game.try_play_to_stage(miya, MemberArea::Center);
    assert!(res.is_err());
}
