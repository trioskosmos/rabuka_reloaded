/// Tests for PL!SP-bp4-025-L (Special Color) ab#0 — Q195
///
/// ab#0 (ライブ開始時): ライブ終了まで、自分のステージのセンターエリアにいる
///   Liella!のメンバーが持つブレードの数が3つになる。
/// ab#1 (ライブ成功時): センターのLiella!がこのターン移動してたら+1スコア
///
/// Q195: 既に+1ブレードを持っているメンバーにset_blade(3)を使うと？
/// Answer: 4。set_blade(3)で3になってから、既存の+1が乗る。
use crate::helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

/// Liella! member at Center. Non-Liella at LeftSide.
/// Special Color as live card. LiveStart fires set_blade_count(3).
/// Only Liella! members should get the blade modifier (not ALL stage cards).
#[test]
fn special_color_q195_set_blade_liella_at_center() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let special = game.id("PL!SP-bp4-025-L");
    let liella = game.id("PL!SP-bp1-001-R"); // 澁谷かのん, CatChu!, blade=3
    let non_liella = game.id("PL!-sd1-010-SD"); // 高坂穂乃果, Printemps, not Liella!

    game.state.player1.stage.stage = [non_liella, liella, -1];
    game.state.player1.hand.cards.push(special);
    game.state.player1.hand.cards.push(non_liella);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(non_liella);
        game.state.player2.main_deck.cards.push(non_liella);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(special);
    advance_to_live_start(&mut game);

    // set_blade_count(3) should set target member's blade to 3
    // Engine sets blade_modifier to 3 (delta = 3 - 0 = 3)
    // Q195: if member already had +1, result should be 4 (set + ongoing)
    assert_eq!(
        game.state.mods.get_blade_modifier(liella),
        3,
        "Liella! member blade modifier should be 3 after set_blade_count"
    );
    // Non-Liella! member should NOT get the blade modifier
    assert_eq!(
        game.state.mods.get_blade_modifier(non_liella),
        0,
        "Non-Liella! member should NOT get blade modifier from set_blade_count"
    );
}
