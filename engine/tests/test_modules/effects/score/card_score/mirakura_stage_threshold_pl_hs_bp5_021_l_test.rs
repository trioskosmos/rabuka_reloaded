use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn pl_hs_bp5_021_l_advance_to_live_card_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

#[test]
fn pl_hs_bp5_021_l_three_mirakura_members_grant_score_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live_card = game.id("PL!HS-bp5-021-L");
    let mirakura = game.id("PL!HS-sd1-003-SD");
    let filler = game.id("PL!-sd1-010-SD");
    game.give_energy(30);
    fill_decks(&mut game, filler);
    let mirakura_b = game.new_id("PL!HS-sd1-003-SD");
    let mirakura_c = game.new_id("PL!HS-sd1-003-SD");
    game.state.player1.hand.cards.push(mirakura);
    game.state.player1.hand.cards.push(mirakura_b);
    game.state.player1.hand.cards.push(mirakura_c);
    game.play_to_stage(mirakura, MemberArea::LeftSide);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.play_to_stage(mirakura_b, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.play_to_stage(mirakura_c, MemberArea::RightSide);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.state.player1.hand.cards.push(live_card);
    pl_hs_bp5_021_l_advance_to_live_card_set(&mut game);
    game.set_live_card(live_card);
    game.pass();
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    let score_mod = game.state.mods.get_score_modifier(live_card);
    assert!(
        score_mod >= 1,
        "Score should be +1 with 3 みらくらぱーく！ members (got {})",
        score_mod
    );
}

#[test]
fn pl_hs_bp5_021_l_one_mirakura_member_no_score_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live_card = game.id("PL!HS-bp5-021-L");
    let mirakura = game.id("PL!HS-sd1-003-SD");
    let other = game.id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-010-SD");
    game.give_energy(20);
    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(mirakura);
    game.state.player1.hand.cards.push(other);
    game.play_to_stage(mirakura, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.play_to_stage(other, MemberArea::LeftSide);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.state.player1.hand.cards.push(live_card);
    pl_hs_bp5_021_l_advance_to_live_card_set(&mut game);
    game.set_live_card(live_card);
    game.pass();
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    let score_mod = game.state.mods.get_score_modifier(live_card);
    assert_eq!(
        score_mod, 0,
        "Score should be +0 with only 1 みらくらぱーく！ member (got {})",
        score_mod
    );
}
