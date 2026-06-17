use crate::helpers::*;
use rabuka_engine::card::HeartColor;

fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player2.main_deck.cards.push(filler);
    }
}

/// 桂城 泉 (PL!HS-bp5-016-N): "常時: 相手のステージにウェイト状態のメンバーが2人以上いるかぎり、heart06を得る。"
/// 2 opponent wait members → condition met → heart06 gained.
#[test]
fn opponent_has_2_wait_members_gains_heart06() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let izumi = game.id("PL!HS-bp5-016-N");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [izumi, -1, -1];
    fill_decks(&mut game, filler);

    let opp_member1 = game.id("PL!-sd1-010-SD");
    let opp_member2 = game.id("PL!-sd1-010-SD");
    game.state.player2.stage.stage = [opp_member1, opp_member2, -1];
    game.state
        .mods
        .add_orientation_modifier(opp_member1, "wait");
    game.state
        .mods
        .add_orientation_modifier(opp_member2, "wait");
    game.state.recalculate_constants();

    let heart_mod = game
        .state
        .mods
        .get_heart_modifier(izumi, HeartColor::Heart06);
    assert!(
        heart_mod >= 1,
        "heart06 should be applied, got {}",
        heart_mod
    );
}

/// 0 opponent wait members → condition NOT met → no heart06.
#[test]
fn opponent_has_0_wait_members_no_heart06() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let izumi = game.id("PL!HS-bp5-016-N");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [izumi, -1, -1];
    fill_decks(&mut game, filler);

    let opp_member1 = game.id("PL!-sd1-010-SD");
    let opp_member2 = game.id("PL!-sd1-010-SD");
    game.state.player2.stage.stage = [opp_member1, opp_member2, -1];
    game.state.recalculate_constants();

    let heart_mod = game
        .state
        .mods
        .get_heart_modifier(izumi, HeartColor::Heart06);
    assert_eq!(
        heart_mod, 0,
        "heart06 should NOT be applied, got {}",
        heart_mod
    );
}

/// 1 opponent wait member → condition NOT met (need 2+) → no heart06.
#[test]
fn opponent_has_1_wait_member_no_heart06() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let izumi = game.id("PL!HS-bp5-016-N");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [izumi, -1, -1];
    fill_decks(&mut game, filler);

    let opp_member1 = game.id("PL!-sd1-010-SD");
    let opp_member2 = game.id("PL!-sd1-010-SD");
    game.state.player2.stage.stage = [opp_member1, opp_member2, -1];
    game.state
        .mods
        .add_orientation_modifier(opp_member1, "wait");
    // opp_member2 defaults to active
    game.state.recalculate_constants();

    let heart_mod = game
        .state
        .mods
        .get_heart_modifier(izumi, HeartColor::Heart06);
    assert_eq!(
        heart_mod, 0,
        "heart06 should NOT be applied with 1 wait, got {}",
        heart_mod
    );
}

/// Own wait members should NOT count toward a condition targeting opponent.
#[test]
fn own_wait_members_do_not_count_for_opponent_condition() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let izumi = game.id("PL!HS-bp5-016-N");
    let filler = game.id("PL!-sd1-010-SD");

    let own_member1 = game.id("PL!-sd1-010-SD");
    let own_member2 = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [izumi, own_member1, own_member2];
    game.state
        .mods
        .add_orientation_modifier(own_member1, "wait");
    game.state
        .mods
        .add_orientation_modifier(own_member2, "wait");
    fill_decks(&mut game, filler);

    game.state.player2.stage.stage = [-1, -1, -1];
    game.state.recalculate_constants();

    let heart_mod = game
        .state
        .mods
        .get_heart_modifier(izumi, HeartColor::Heart06);
    assert_eq!(
        heart_mod, 0,
        "heart06 should NOT come from own stage, got {}",
        heart_mod
    );
}

/// 2 wait + 1 active on opponent → condition met (only wait members counted).
#[test]
fn opponent_has_2_wait_and_1_active_condition_met() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let izumi = game.id("PL!HS-bp5-016-N");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [izumi, -1, -1];
    fill_decks(&mut game, filler);

    let opp_member1 = game.id("PL!-sd1-010-SD");
    let opp_member2 = game.id("PL!-sd1-010-SD");
    let opp_member3 = game.id("PL!-sd1-010-SD");
    game.state.player2.stage.stage = [opp_member1, opp_member2, opp_member3];
    game.state
        .mods
        .add_orientation_modifier(opp_member1, "wait");
    game.state
        .mods
        .add_orientation_modifier(opp_member2, "wait");
    game.state.recalculate_constants();

    let heart_mod = game
        .state
        .mods
        .get_heart_modifier(izumi, HeartColor::Heart06);
    assert!(
        heart_mod >= 1,
        "heart06 should apply with 2 wait (1 active irrelevant), got {}",
        heart_mod
    );
}

/// Dynamic: 2 wait → condition met; change one to active → only 1 wait remaining → condition lost.
#[test]
fn opponent_goes_from_2_wait_to_1_wait_loses_heart06() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let izumi = game.id("PL!HS-bp5-016-N");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [izumi, -1, -1];
    fill_decks(&mut game, filler);

    let opp_member1 = game.id("PL!-sd1-010-SD");
    let opp_member2 = game.id("PL!-sd1-010-SD");
    game.state.player2.stage.stage = [opp_member1, opp_member2, -1];
    game.state
        .mods
        .add_orientation_modifier(opp_member1, "wait");
    game.state
        .mods
        .add_orientation_modifier(opp_member2, "wait");
    game.state.recalculate_constants();

    // Condition met: 2 wait members
    let heart_mod = game
        .state
        .mods
        .get_heart_modifier(izumi, HeartColor::Heart06);
    assert!(
        heart_mod >= 1,
        "should have heart06 with 2 wait, got {}",
        heart_mod
    );

    // Change one to active → only 1 wait remaining
    game.state
        .mods
        .add_orientation_modifier(opp_member2, "active");
    game.state.recalculate_constants();

    let heart_mod_after = game
        .state
        .mods
        .get_heart_modifier(izumi, HeartColor::Heart06);
    assert_eq!(
        heart_mod_after, 0,
        "should lose heart06 when only 1 wait remains, got {}",
        heart_mod_after
    );
}
