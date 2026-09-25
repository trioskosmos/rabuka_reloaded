use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

#[test]
fn pl_n_pb1_022_p_plus_baton_shioriko_fire_kasumi_no_fire() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let mia = game.id("PL!N-pb1-022-P+");
    let shioriko = game.id("PL!N-sd1-010-SD");
    let kasumi = game.id("PL!N-sd1-002-SD");
    let filler = game.id("PL!-sd1-010-SD");
    game.give_energy(15);
    game.add_to_stage(MemberArea::Center, shioriko);
    game.state.player1.hand.cards.push(mia);
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(filler);
    let _hand_before = game.state.player1.hand.cards.len();
    game.play_to_stage(mia, MemberArea::Center);
    assert!(
        game.has_pending_choice(),
        "Mia should trigger and wait for discard choice after drawing 2"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        2,
        "Hand should have 2 drawn cards before discard"
    );
    game.select_indices(&[0]);
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "Hand should have 1 card after completing discard"
    );
    let mia2 = game.new_id("PL!N-pb1-022-P+");
    let center_card = game.state.player1.stage.stage[1];
    game.state.player1.stage.stage[1] = -1;
    game.state
        .player1
        .deployed_this_turn
        .retain(|id| *id != center_card);
    game.add_to_stage(MemberArea::Center, kasumi);
    game.state.player1.hand.cards.push(mia2);
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(filler);
    game.play_to_stage(mia2, MemberArea::Center);
    assert!(
        !game.has_pending_choice(),
        "Mia should NOT trigger because she did not replace Shioriko"
    );
}
