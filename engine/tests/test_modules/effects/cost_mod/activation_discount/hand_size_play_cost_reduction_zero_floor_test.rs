use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

#[test]
fn ll_bp2_001_r_plus_hand_size_reduces_play_cost_with_zero_floor() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let kotori = game.id("LL-bp2-001-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let _base_cost = game.db.get_card(kotori).unwrap().cost.unwrap();
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(kotori);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.energy_zone.cards.clear();
    game.state.player1.energy_zone.set_active_count(0);
    game.give_energy(17);
    let res = game.try_play_to_stage(kotori, MemberArea::Center);
    assert!(
        res.is_err(),
        "Should fail to play Kotori with only 17 energy (cost should be 18)"
    );
    game.give_energy(1);
    let res = game.try_play_to_stage(kotori, MemberArea::Center);
    assert!(
        res.is_ok(),
        "Should successfully play Kotori with 18 energy (base 20 - 2 reduction)"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "All 18 energy should have been consumed"
    );
    let mut game2 = TestGame::new(db.clone());
    game2.state.player1.hand.cards.clear();
    game2.state.player1.hand.cards.push(kotori);
    for _ in 0..25 {
        game2.state.player1.hand.cards.push(filler);
    }
    game2.state.player1.energy_zone.cards.clear();
    game2.state.player1.energy_zone.set_active_count(0);
    let res = game2.try_play_to_stage(kotori, MemberArea::Center);
    assert!(
        res.is_ok(),
        "Should play Kotori for 0 energy with 25 cards in hand"
    );
}
