/// Tests for 澁谷かのん (PL!SP-bp2-001-R＋) — Debut invalidate ability:
///
/// 登場 自分のステージにいる『Liella!』のメンバー1人のすべての
/// ライブ開始時能力を、ライブ終了時まで、無効にしてもよい。
/// これにより無効にした場合、自分の控え室から『Liella!』の
/// カードを1枚手札に加える。
///
/// Q106: Nullifying already-nullified abilities doesn't count.

mod helpers;
use helpers::*;

/// Debut recovers Liella! card from discard when invalidate is taken.
#[test]
fn kanon_q106_debut_recover_from_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanon = game.id("PL!SP-bp2-001-R\u{ff0b}");
    let liella = game.id("PL!SP-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(kanon);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(liella);
    game.give_energy(13);

    game.state.player1.stage.stage[1] = -1;
    game.play_to_stage(kanon, rabuka_engine::zones::MemberArea::Center);

    let recovered = game.state.player1.hand.cards.contains(&liella);
    assert!(recovered, "card should have been recovered from discard");
    assert!(game.state.player1.stage.stage.contains(&kanon),
        "Kanon should be on stage after debut");
}
