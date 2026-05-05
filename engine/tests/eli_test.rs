/// PL!-sd1-002-SD (絢瀬絵里) ab#0 — Q79
///
/// {{kidou.png|起動}}このメンバーをステージから控え室に置く：
/// 自分の控え室からメンバーカードを1枚手札に加える。
///
/// Q79: After using this ability (self_cost removes Eli from stage),
/// can a new member card be placed in the vacated area?
/// A: Yes — the self_cost empties the area, making it available.

mod helpers;
use helpers::*;
use rabuka_engine::zones::MemberArea;

#[test]
fn eli_q79_vacated_area_can_play_new_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let eli = game.id("PL!-sd1-002-SD");
    let target_member = game.id("PL!-sd1-001-SD");
    let new_member = game.id("PL!-sd1-003-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // Stage: 絢瀬絵里 at center (set directly, no need for play)
    game.state.player1.stage.stage = [-1, eli, -1];
    // Hand: filler (discard for hand count), new_member (for Q79 placement test)
    game.add_to_hand(filler);
    game.add_to_hand(new_member);

    // Waitroom: ONLY the target member (avoids selection choice)
    game.add_to_discard(target_member);

    // Enough energy for new_member's play cost (cost=13)
    game.give_energy(15);

    // Activate Eli's ability (self_cost → recover → vacates center)
    game.activate_ability(eli);

    // The ability creates a selection choice because waitroom has 2 member cards
    // (target_member + Eli after self_cost). Resolve the choice first.
    if game.has_pending_choice() {
        // Find target_member's index in waitroom to recover it
        let idx = game.state.player1.waitroom.cards.iter()
            .position(|&id| id == target_member)
            .expect("target_member should be in waitroom");
        game.select_indices(&[idx]);
    }

    // After activation, center should be empty (self_cost removed Eli)
    assert_eq!(game.state.player1.stage.stage[1], -1,
        "Self_cost vacated center area");

    // Q79: A new member CAN be placed in the vacated center area
    game.play_to_stage(new_member, MemberArea::Center);
    assert_eq!(game.state.player1.stage.stage[1], new_member,
        "New member placed in previously vacated center area");
}
