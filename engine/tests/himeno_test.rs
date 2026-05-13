/// Tests for 安養寺姫芽 (PL!HS-bp1-009-R) — Debut look_and_select:
///
/// 登場 手札を1枚控え室に置いてもよい：
/// 自分のデッキの上からカードを5枚見る。その中から「みらくらぱーく！」の
/// カードを1枚公開して手札に加えてもよい。残りを控え室に置く。
///
/// Q82: ド！ド！ド！ (PL!HS-bp1-023-L) and アイデンティティ (PL!HS-PR-012-PR)
/// are both みらくらぱーく！ cards and can be selected by this ability.

mod helpers;
use helpers::*;

/// Edge: ド！ド！ド！ (live card, unit=みらくらぱーく！) among top 5 → selectable.
#[test]
fn himeno_q82_dodo_live_card_selectable() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let himeno = game.id("PL!HS-bp1-009-R");
    let filler = game.id("PL!-sd1-010-SD");
    let dodo = game.id("PL!HS-bp1-023-L"); // ド！ド！ド！, みらくらぱーく！

    game.state.player1.hand.cards.push(himeno);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(4);

    // Top 5: f, f, dodo, f, f
    for _ in 0..2 { game.state.player1.main_deck.cards.insert(0, filler); }
    game.state.player1.main_deck.cards.insert(0, dodo);
    for _ in 0..2 { game.state.player1.main_deck.cards.insert(0, filler); }
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); }

    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(himeno, rabuka_engine::zones::MemberArea::LeftSide);

    if game.has_pending_choice() { game.select_indices(&[]); } // skip optional cost

    // Select ド！ド！ド！ from looked_at (index 2 in the top 5)
    if game.has_pending_choice() { game.select_indices(&[2]); }

    assert!(game.state.player1.hand.cards.contains(&dodo),
        "Q82: ド！ド！ド！ (みらくらぱーく！) is selectable");
}

/// Edge: アイデンティティ (live card, unit=みらくらぱーく！) selectable.
#[test]
fn himeno_q82_identity_live_card_selectable() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let himeno = game.id("PL!HS-bp1-009-R");
    let filler = game.id("PL!-sd1-010-SD");
    let identity = game.id("PL!HS-PR-012-PR"); // アイデンティティ, みらくらぱーく！

    game.state.player1.hand.cards.push(himeno);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(4);

    for _ in 0..2 { game.state.player1.main_deck.cards.insert(0, filler); }
    game.state.player1.main_deck.cards.insert(0, identity);
    for _ in 0..2 { game.state.player1.main_deck.cards.insert(0, filler); }
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); }

    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(himeno, rabuka_engine::zones::MemberArea::LeftSide);

    if game.has_pending_choice() { game.select_indices(&[]); }
    if game.has_pending_choice() { game.select_indices(&[2]); }

    assert!(game.state.player1.hand.cards.contains(&identity),
        "Q82: アイデンティティ (みらくらぱーく！) is selectable");
}

/// Edge: No みらくらぱーく！ card among top 5 → reveal has no valid targets.
/// The look_and_select completes without adding anything.
#[test]
fn himeno_edge_no_mirakura_skips() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let himeno = game.id("PL!HS-bp1-009-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(himeno);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(4);

    for _ in 0..5 { game.state.player1.main_deck.cards.insert(0, filler); }
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); }

    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(himeno, rabuka_engine::zones::MemberArea::LeftSide);

    if game.has_pending_choice() { game.select_indices(&[]); }

    // Verify nothing was added to hand from deck (no matching cards)
    // After play_to_stage: hand had himeno + filler = 2 → himeno played → 1
    // No matching mirakura cards found → nothing added to hand
    assert_eq!(game.state.player1.hand.cards.len(), 1,
        "No cards added to hand when no mirakura card in top 5");
}
