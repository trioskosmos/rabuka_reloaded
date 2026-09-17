use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn setup_pl_bp4_006_r_success_score_look(game: &mut TestGame) -> i16 {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let me = game.id("PL!-bp4-006-R");
    game.add_to_hand(me);
    game.give_energy(30);
    me
}

#[test]
fn pl_bp4_006_r_score_three_or_more_fetches_mus_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = setup_pl_bp4_006_r_success_score_look(&mut game);

    for _ in 0..2 {
        let s = game.id("PL!S-pb1-023-L");
        game.state.player1.success_live_card_zone.cards.push(s);
    }
    let mus_member = game.new_id("PL!-sd1-007-SD");
    game.state.player1.main_deck.cards.insert(0, mus_member);

    game.play_to_stage(me, MemberArea::Center);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&mus_member),
        "score gate met -> μ's member fetched to hand"
    );
}

#[test]
fn pl_bp4_006_r_low_success_score_does_not_fetch_mus_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = setup_pl_bp4_006_r_success_score_look(&mut game);

    let s1 = game.id("PL!HS-bp2-020-L");
    game.state.player1.success_live_card_zone.cards.push(s1);
    let mus_member = game.new_id("PL!-sd1-007-SD");
    game.state.player1.main_deck.cards.insert(0, mus_member);

    game.play_to_stage(me, MemberArea::Center);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[]);
    }

    assert!(
        !game.state.player1.hand.cards.contains(&mus_member),
        "score total < 3 -> no look, no fetch"
    );
}
