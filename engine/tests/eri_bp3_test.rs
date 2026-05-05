/// PL!-bp3-002-R (絢瀬絵里) ab#0 — Q144
///
/// {{toujyou.png|登場}}手札を1枚控え室に置いてもよい：
/// 自分のステージにいるコスト4以下のメンバーを2人までウェイトにする。
///
/// Q144: When only 1 eligible member (cost ≤ 4) is on stage,
/// can the ability still activate and put that 1 member to wait?
/// A: Yes — "まで" (up to) is an upper bound, not a requirement.

mod helpers;
use helpers::*;
use rabuka_engine::zones::MemberArea;

#[test]
fn eri_q144_up_to_semantics_1_eligible_still_works() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let eri = game.id("PL!-bp3-002-R");
    let eligible = game.id("PL!-sd1-010-SD"); // cost=4, under limit
    let filler = game.id("PL!-sd1-019-SD");

    // Stage: eligible member on left, empty center for eri
    game.state.player1.stage.stage = [eligible, -1, -1];
    game.add_to_hand(eri);
    game.add_to_hand(filler);
    game.give_energy(15);

    assert_eq!(game.state.get_orientation_modifier(eligible), None,
        "Before activation: eligible member is active (no wait modifier)");

    game.play_to_stage(eri, MemberArea::Center);

    // Debut fires: optional cost (discard 1)
    if game.has_pending_choice() {
        game.select_option(1); // pay cost
    }

    // Selection prompt: choose which member(s) to put to wait
    if game.has_pending_choice() {
        game.select_indices(&[0]); // select the only eligible member
    }

    // Q144: Only 1 eligible member existed, but the ability still
    // activates and puts that member to wait
    let orientation = game.state.get_orientation_modifier(eligible);
    assert_eq!(orientation, Some(&"wait".to_string()),
        "1 eligible member was put to wait — 'up to 2' is an upper bound");
}
