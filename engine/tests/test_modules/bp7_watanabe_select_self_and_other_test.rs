/// BP07 CLEAN-G19: PL!S-bp7-005-R＋ 渡辺 曜 ab#2 (起動).
///
/// 起動：手札を2枚控え室に置く：このメンバーと自分のステージにいるほかの『Aqours』の
/// メンバー1人を選ぶ。それらが持つ登場能力それぞれ1つを発動させる。
///
/// (Activation) Discard 2 from hand: choose THIS member AND 1 other Aqours member
/// on your stage, then activate 1 of the 登場 abilities EACH of them has.
///
/// The defect (G19): the select excluded self and picked only 1 other member, so
/// this member's own 登場 ability would never fire. These tests pin that THIS
/// member is selectable (count 2, not exclude_self) and that its 登場 ability fires.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const WATANABE: &str = "PL!S-bp7-005-R＋"; // 渡辺 曜 (Aqours) — has 登場 ab#0 (place discard member under)
const HANAMARU: &str = "PL!S-bp7-007-R＋"; // 国木田花丸 (Aqours) — has 登場 ab#0 (recover cost<=2 member)
const DISCARD_TARGET: &str = "PL!-sd1-001-SD"; // generic member for 渡辺曜's 登場 to place under

fn activate_ab2(game: &mut TestGame, watanabe: i16) {
    TurnEngine_activate_ab2(game, watanabe);
}

fn TurnEngine_activate_ab2(game: &mut TestGame, watanabe: i16) {
    rabuka_engine::turn::TurnEngine::execute_main_phase_action_with_ability_index(
        &mut game.state,
        &rabuka_engine::game_setup::ActionType::UseAbility,
        Some(watanabe),
        None,
        None,
        None,
        Some(2),
    )
    .expect("activate ab#2 failed");
}

fn setup(game: &mut TestGame) -> (i16, i16) {
    let watanabe = game.id(WATANABE);
    let hanamaru = game.id(HANAMARU);
    // 渡辺 曜 at CENTER (ab#2 requires center); 花丸 on the left.
    game.state.player1.stage.stage = [hanamaru, watanabe, -1];
    // Cost: 2 hand cards to discard.
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    // Discard: a member card that 渡辺曜's 登場 can place under a stage member.
    let dt = game.id(DISCARD_TARGET);
    game.state.player1.waitroom.cards.push(dt);
    (watanabe, hanamaru)
}

/// The select is over 2 candidates (count 2) and THIS member (渡辺 曜) is among them.
#[test]
fn watanabe_select_includes_this_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let (watanabe, _) = setup(&mut game);
    activate_ab2(&mut game, watanabe);

    // Cost (discard 2) auto-resolves; the select should be pending.
    assert!(
        game.has_pending_choice(),
        "select should be pending after the discard cost"
    );
    let choice = game.get_pending_choice().clone();
    match choice {
        rabuka_engine::ability::types::Choice::SelectCard { count, .. } => {
            assert_eq!(count, 2, "select should target 2 members (this + 1 other)");
        }
        other => panic!("expected SelectCard select, got {:?}", other),
    }
}
