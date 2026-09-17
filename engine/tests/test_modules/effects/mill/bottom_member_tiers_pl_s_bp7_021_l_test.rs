use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD";

fn pl_s_bp7_021_l_three_member_board(game: &mut TestGame) -> i16 {
    let live = game.id("PL!S-bp7-021-L");
    game.state.player1.live_card_zone.cards.push(live);
    for i in 0..3usize {
        let m = game.new_id(FILLER);
        game.state.player1.stage.stage[i] = m;
    }
    live
}

#[test]
fn pl_s_bp7_021_l_bottom_mill_two_members_no_draw_or_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = pl_s_bp7_021_l_three_member_board(&mut game);
    let m1 = game.id("PL!S-sd1-001-SD");
    let m2 = game.id("PL!S-sd1-001-SD");
    let l1 = game.id("PL!-sd1-019-SD");
    game.state.player1.main_deck.cards.push(m1);
    game.state.player1.main_deck.cards.push(m2);
    for _ in 0..3 {
        let l = game.new_id("PL!-sd1-019-SD");
        game.state.player1.main_deck.cards.push(l);
    }
    let _ = l1;
    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    assert_eq!(game.state.player1.waitroom.cards.len(), 5, "five cards milled to the waitroom");
    assert_eq!(game.state.player1.hand.cards.len(), 0, "only 2 members < 3 → no draw");
    assert_eq!(game.state.mods.get_score_modifier(live), 0, "not all members → no score");
}

#[test]
fn pl_s_bp7_021_l_bottom_mill_three_members_draw_only() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = pl_s_bp7_021_l_three_member_board(&mut game);
    for _ in 0..3 {
        let m = game.id("PL!S-sd1-001-SD");
        game.state.player1.main_deck.cards.push(m);
    }
    for _ in 0..2 {
        let l = game.new_id("PL!-sd1-019-SD");
        game.state.player1.main_deck.cards.push(l);
    }
    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    assert_eq!(game.state.player1.hand.cards.len(), 1, "3 members among the milled 5 → draw 1");
    assert_eq!(game.state.mods.get_score_modifier(live), 0, "not ALL five were members → no +1");
}

#[test]
fn pl_s_bp7_021_l_bottom_mill_five_members_draw_and_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = pl_s_bp7_021_l_three_member_board(&mut game);
    for _ in 0..5 {
        let m = game.id("PL!S-sd1-001-SD");
        game.state.player1.main_deck.cards.push(m);
    }
    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    assert_eq!(game.state.player1.hand.cards.len(), 1, "draw fired");
    assert_eq!(game.state.mods.get_score_modifier(live), 1, "all five were members → スコア+1");
}

#[test]
fn pl_s_bp7_021_l_two_staged_members_no_bottom_mill() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = pl_s_bp7_021_l_three_member_board(&mut game);
    game.state.player1.stage.stage[2] = -1;
    for _ in 0..5 {
        let m = game.id("PL!S-sd1-001-SD");
        game.state.player1.main_deck.cards.push(m);
    }
    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    assert_eq!(game.state.player1.waitroom.cards.len(), 0, "stage count 2 < 3 → nothing is milled");
}
