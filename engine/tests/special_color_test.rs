/// Tests for PL!SP-bp4-025-L (Special Color) ab#0 — Q195
///
/// ab#0 (ライブ開始時): ライブ終了まで、自分のステージのセンターエリアにいる
///   Liella!のメンバーが持つブレードの数が3つになる。
/// ab#1 (ライブ成功時): センターのLiella!がこのターン移動してたら+1スコア
///
/// Q195: 既に+1ブレードを持っているメンバーにset_blade(3)を使うと？
/// Answer: 4。set_blade(3)で3になってから、既存の+1が乗る。

mod helpers;
use helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    game.pass(); game.pass(); game.pass(); game.pass(); game.pass();
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

/// Liella! member at Center. Non-Liella at LeftSide.
/// Special Color as live card. LiveStart fires set_blade_count(3).
/// Currently engine applies modifier to ALL stage cards (filter bug).
#[test]
fn special_color_q195_set_blade_liella_at_center() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let special = game.id("PL!SP-bp4-025-L");
    let liella = game.id("PL!SP-bp1-001-R");  // 澁谷かのん, CatChu!, blade=3
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, liella, -1];
    game.state.player1.hand.cards.push(special);
    game.state.player1.hand.cards.push(filler);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(special);
    advance_to_live_start(&mut game);

    // set_blade_count(3) should set target member's blade to 3
    // Engine sets blade_modifier to 3 (delta = 3 - 0 = 3)
    // Q195: if member already had +1, result should be 4 (set + ongoing)
    assert_eq!(game.state.get_blade_modifier(liella), 3,
        "Liella! member blade modifier should be 3 after set_blade_count");
}

/// Verify set_blade_count was parsed correctly.
#[test]
fn special_color_q195_parser_fields() {
    let db = load_real_database();
    let card = db.get_card_by_no("PL!SP-bp4-025-L").expect("Special Color exists");
    let ab = card.abilities.iter()
        .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
        .expect("Should have LiveStart ability");

    let effect = ab.effect.as_ref().expect("Should have effect");
    assert_eq!(effect.action, "set_blade_count");
    assert_eq!(effect.count, Some(3));
    assert_eq!(effect.duration.as_deref(), Some("live_end"));
    assert_eq!(effect.position.as_ref().and_then(|p| p.get_position()), Some("center"));
    assert_eq!(effect.group_names.as_ref().and_then(|gn| gn.first().map(|s| s.as_str())), Some("Liella!"));
    assert_eq!(effect.card_type.as_deref(), Some("member_card"));
}
