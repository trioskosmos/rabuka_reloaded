use crate::helpers::*;
use rabuka_engine::card::HeartColor;

/// PL!N-bp7-007-R+ 優木せつ菜: 常時 このメンバーの下にあるエネルギーカード1枚につき、
/// heart02を得る。
#[test]
fn two_under_member_energies_grant_two_heart02() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!N-bp7-007-R\u{ff0b}");
    game.state.player1.stage.stage = [-1, member, -1];
    // Place 2 energy cards under the member
    let e1 = game.id("LL-E-001-SD");
    let e2 = game.id("LL-E-001-SD");
    if let Some(idx) = game.state.player1.stage.stage.iter().position(|&x| x == member) {
        game.state.player1.stage.under_cards[idx].push(e1);
        game.state.player1.stage.under_cards[idx].push(e2);
    }
    game.state.recalculate_constants();

    let h02 = game
        .state
        .mods
        .get_heart_modifier(member, HeartColor::Heart02);
    assert_eq!(
        h02, 2,
        "2 under-member energies should grant exactly +2 heart02"
    );
}
