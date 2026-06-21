use crate::helpers::*;
use rabuka_engine::card::HeartColor;

/// Test card PL!S-PR-040-PR (国木田花丸, AZALEA):
/// 自動: when you yell, if ≥3 member cards with same group name in revealed_cards,
/// gain heart01 ×1 AND heart04 ×1 until live end.
const ABILITY_CARD: &str = "PL!S-PR-040-PR";

/// AZALEA member cards for satisfying the same-group condition.
const AZALEA_MEMBER_1: &str = "PL!S-PR-015-PR";
const AZALEA_MEMBER_2: &str = "PL!S-PR-016-PR";
const AZALEA_MEMBER_3: &str = "PL!S-PR-019-PR";

fn setup(game: &mut TestGame, member_ids: &[i16]) -> i16 {
    let ability_card = game.id(ABILITY_CARD);
    game.state.player1.stage.stage = [-1, ability_card, -1];
    for &id in member_ids {
        game.state.revealed_cards.push(id);
        game.state.player1.waitroom.cards.push(id);
    }
    ability_card
}

fn get_heart_modifier(game: &TestGame, card_id: i16, color: HeartColor) -> i32 {
    game.state.mods.get_heart_modifier(card_id, color)
}

/// Condition NOT met (only 2 same-group members) → ability should NOT queue or fire.
#[test]
fn multi_color_heart_condition_not_met_does_not_queue() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let m1 = game.id(AZALEA_MEMBER_1);
    let m2 = game.id(AZALEA_MEMBER_2);
    let ability_card = setup(&mut game, &[m1, m2]);

    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");

    assert_eq!(
        get_heart_modifier(&game, ability_card, HeartColor::Heart01),
        0,
        "Should have no heart01 modifier with only 2 same-group members"
    );
    assert_eq!(
        get_heart_modifier(&game, ability_card, HeartColor::Heart04),
        0,
        "Should have no heart04 modifier with only 2 same-group members"
    );
    assert!(
        !game.has_pending_choice(),
        "Should not have a pending choice when condition not met"
    );
}

/// Condition met (3 same-group members) → ability fires, grants heart01 + heart04, no choice.
#[test]
fn multi_color_heart_condition_met_grants_both_colors() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let m1 = game.id(AZALEA_MEMBER_1);
    let m2 = game.id(AZALEA_MEMBER_2);
    let m3 = game.id(AZALEA_MEMBER_3);
    let ability_card = setup(&mut game, &[m1, m2, m3]);

    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");

    assert!(
        !game.has_pending_choice(),
        "Should NOT have a pending choice — fixed multi-color grant should not create SelectHeartColor"
    );

    assert_eq!(
        get_heart_modifier(&game, ability_card, HeartColor::Heart01),
        1,
        "Should have heart01 ×1 modifier"
    );
    assert_eq!(
        get_heart_modifier(&game, ability_card, HeartColor::Heart04),
        1,
        "Should have heart04 ×1 modifier"
    );
}
