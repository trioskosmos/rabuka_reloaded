use crate::helpers::*;

/// PL!S-bp7-005-R+ 渡辺 曜: 常時 メンバーカードが下に置かれている『Aqours』メンバーは、
/// ブレード+1。
/// (Deeper coverage incl. negatives lives in bp7_watanabe_under_card_blade_test.rs.)
#[test]
fn aqours_member_with_member_underneath_gains_one_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!S-bp7-005-R\u{ff0b}");
    game.state.player1.stage.stage = [-1, member, -1];
    // Place an Aqours member card under this member
    let under_card = game.id("PL!S-bp2-001-R");
    if let Some(idx) = game.state.player1.stage.stage.iter().position(|&x| x == member) {
        game.state.player1.stage.under_cards[idx].push(under_card);
    }
    game.state.recalculate_constants();

    let blade = game.state.mods.get_blade_modifier(member);
    assert_eq!(
        blade, 1,
        "under-card Aqours member should gain exactly +1 blade"
    );
}
