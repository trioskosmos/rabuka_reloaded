use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::zones::MemberArea;

#[test]
fn q145_empty_waitroom_still_allows_optional_self_wait() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id("PL!-bp3-003-R");

    game.state.player1.hand.cards.push(kotori);
    game.give_energy(11);
    game.play_to_stage(kotori, MemberArea::Center);

    match game.get_pending_choice() {
        Choice::SelectTarget { target, .. }
            if target.starts_with("pay_optional_cost") || target == "conditional_optional" => {
            game.select_option(1);
        }
        _ => panic!("Q145: optional self-wait cost must be offered"),
    }

    assert!(!game.has_pending_choice(), "Q145: no waitroom target resolves cleanly");
    assert_eq!(
        game.state.mods.get_orientation_modifier(kotori),
        Some("wait"),
        "Q145: accepted self-wait remains applied with an empty waitroom"
    );
}
