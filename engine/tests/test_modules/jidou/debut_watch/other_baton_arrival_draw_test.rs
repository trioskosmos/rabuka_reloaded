use crate::helpers::*;
use crate::test_modules::support::baton_swap_auto_helpers::*;
use rabuka_engine::zones::MemberArea;

#[test]
fn other_member_baton_arrival_draws_one_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let setsuna = game.id("PL!N-PR-025-PR");
    let filler = game.id(FILLER);
    let arriver = game.id("PL!-sd1-002-SD");
    append_twenty_filler_cards(&mut game);

    // setsuna on left; a filler on center. Baton-touch arriver onto center
    // → arriver debuts via baton touch → setsuna (on stage) draws 1.
    game.state.player1.stage.set_area(MemberArea::LeftSide, setsuna);
    replace_member_by_baton_touch(&mut game, filler, arriver, MemberArea::Center);
    resolve_auto_choices_accepting_optionals(&mut game);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "せつ菜 ab#0 should draw 1 when another member debuts via baton touch"
    );
}
