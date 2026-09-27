use crate::helpers::*;
use rabuka_engine::card::HeartColor;

fn setup_blade_heartless_member_reveals(game: &mut TestGame, member_ids: &[i16]) {
    let source = game.id("PL!-bp5-004-R\u{ff0b}");
    game.state.player1.stage.stage = [-1, source, -1];
    for &id in member_ids {
        game.state.revealed_cards.push(id);
        game.state.player1.waitroom.cards.push(id);
    }
}

fn has_all_heart_modifier(game: &TestGame) -> bool {
    game.state
        .mods
        .heart_modifiers
        .iter()
        .any(|(_, hm)| hm.get(&HeartColor::All).is_some_and(|e| e.total() > 0))
}

#[test]
fn yell_three_members_without_blade_heart_gains_all_heart_modifier() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let m1 = game.id("PL!S-bp2-002-R");
    let m2 = game.id("PL!S-PR-013-PR");
    let m3 = game.id("PL!S-sd1-006-SD");
    setup_blade_heartless_member_reveals(&mut game, &[m1, m2, m3]);

    assert!(
        !has_all_heart_modifier(&game),
        "HeartColor::All must not be present before"
    );
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");

    assert!(
        has_all_heart_modifier(&game),
        "Three qualifying members in yell must set HeartColor::All"
    );
}

#[test]
fn yell_three_members_with_blade_hearts_does_not_gain_all_heart_modifier() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let m1 = game.id("PL!-pb1-014-R");
    let m2 = game.id("PL!-pb1-014-R");
    let m3 = game.id("PL!-pb1-014-R");
    setup_blade_heartless_member_reveals(&mut game, &[m1, m2, m3]);

    let before = has_all_heart_modifier(&game);
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
    let after = has_all_heart_modifier(&game);

    assert_eq!(
        before, after,
        "Three members with blade hearts must not set HeartColor::All"
    );
}

#[test]
fn yell_two_members_without_blade_heart_does_not_gain_all_heart_modifier() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let m1 = game.id("PL!S-bp2-002-R");
    let m2 = game.id("PL!S-PR-013-PR");
    setup_blade_heartless_member_reveals(&mut game, &[m1, m2]);

    let before = has_all_heart_modifier(&game);
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
    let after = has_all_heart_modifier(&game);

    assert_eq!(
        before, after,
        "Only two qualifying members must not set HeartColor::All"
    );
}
