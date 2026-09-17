use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

#[test]
fn s_bp5_015_debut_twelve_card_deck_mills_ten_keeps_two() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nico = game.id("PL!S-bp5-015-N");
    let filler = game.new_id("PL!-sd1-010-SD");

    game.add_to_hand(nico);
    game.give_energy(12);
    // 12 fillers: 10 get milled, 2 remain so the deck never runs dry mid-effect.
    for _ in 0..12 {
        game.state.player1.main_deck.cards.push(filler);
    }

    game.play_to_stage(nico, MemberArea::Center);

    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        10,
        "debut mills exactly 10 cards to the waitroom"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        2,
        "deck keeps the 2 unmilled cards"
    );
}
