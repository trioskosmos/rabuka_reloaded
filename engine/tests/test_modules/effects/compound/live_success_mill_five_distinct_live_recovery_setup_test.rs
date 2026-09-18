use crate::helpers::*;

#[test]
fn live_success_mill_distinct_live_recovery_three_same_name_copies_resolves() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let mia = game.id("PL!N-bp4-011-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let l1 = game.id("PL!N-bp1-028-L");
    let l2 = game.new_id("PL!N-bp1-028-L");
    let l3 = game.new_id("PL!N-bp1-028-L");
    game.state.player1.waitroom.cards.push(l1);
    game.state.player1.waitroom.cards.push(l2);
    game.state.player1.waitroom.cards.push(l3);
    game.state.player1.stage.stage = [-1, mia, -1];
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); game.state.player2.main_deck.cards.push(filler); }
    let live = game.id("PL!-sd1-019-SD");
    game.state.player1.hand.cards.push(live);
    for _ in 0..5 { game.pass(); }
    game.set_live_card(live);
    for _ in 0..7 {
        game.pass();
        if let Some(choice) = game.state.get_pending_choice() {
            match choice {
                rabuka_engine::ability::types::Choice::SelectCard { .. } => game.select_indices(&[]),
                _ => game.select_indices(&[]),
            }
        }
    }
    assert!(!game.has_pending_choice());
}

#[test]
fn live_success_mill_distinct_live_recovery_two_same_name_copies_resolves() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let mia = game.id("PL!N-bp4-011-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let l1 = game.id("PL!N-bp1-028-L");
    let l2 = game.new_id("PL!N-bp1-028-L");
    game.state.player1.waitroom.cards.push(l1);
    game.state.player1.waitroom.cards.push(l2);
    game.state.player1.stage.stage = [-1, mia, -1];
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); }
    let live = game.id("PL!-sd1-019-SD");
    game.state.player1.hand.cards.push(live);
    for _ in 0..5 { game.pass(); }
    game.set_live_card(live);
    for _ in 0..7 {
        game.pass();
        if let Some(choice) = game.state.get_pending_choice() {
            match choice {
                rabuka_engine::ability::types::Choice::SelectCard { .. } => game.select_indices(&[]),
                _ => game.select_indices(&[]),
            }
        }
    }
    assert!(!game.has_pending_choice());
}
