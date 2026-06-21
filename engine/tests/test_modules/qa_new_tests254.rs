use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

fn fill_decks(game: &mut TestGame) {
    let f = game.id_ref("PL!-sd1-010-SD");
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(f);
        game.state.player2.main_deck.cards.push(f);
    }
}

/// Q254 core: Live Start ability is mandatory when condition is met.
/// With 2+ cards in success_live_card_zone, the ability fires automatically
/// with no skip/optional choice offered to the player.
#[test]
fn q254_mandatory_no_skip_when_condition_met() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live_card = game.id("PL!SP-sd2-023-SD2");
    let past_1 = game.id("PL!-sd1-020-SD");
    let past_2 = game.id("PL!-sd1-021-SD");

    game.state.player1.success_live_card_zone.cards.push(past_1);
    game.state.player1.success_live_card_zone.cards.push(past_2);
    game.state.player1.hand.cards.push(live_card);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        !game.has_pending_choice(),
        "No choice should be pending — ability is mandatory"
    );

    assert_eq!(
        game.state.mods.get_score_modifier(live_card),
        5,
        "Score +5 when condition met"
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live_card, HeartColor::Heart02),
        3
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live_card, HeartColor::Heart03),
        3
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live_card, HeartColor::Heart06),
        3
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live_card, HeartColor::Heart00),
        3
    );
}

/// Threshold boundary: exactly 1 card in success zone → condition NOT met,
/// no modifiers applied.
#[test]
fn q254_one_card_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live_card = game.id("PL!SP-sd2-023-SD2");
    let past_live = game.id("PL!-sd1-020-SD");

    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(past_live);
    game.state.player1.hand.cards.push(live_card);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.mods.get_score_modifier(live_card),
        0,
        "No score mod with 1 card"
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live_card, HeartColor::Heart02),
        0,
        "No H02 mod with 1 card"
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live_card, HeartColor::Heart03),
        0,
        "No H03 mod with 1 card"
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live_card, HeartColor::Heart06),
        0,
        "No H06 mod with 1 card"
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live_card, HeartColor::Heart00),
        0,
        "No H00 mod with 1 card"
    );
}

/// Threshold boundary: 0 cards in success zone → no effect.
#[test]
fn q254_zero_cards_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live_card = game.id("PL!SP-sd2-023-SD2");

    game.state.player1.hand.cards.push(live_card);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.mods.get_score_modifier(live_card),
        0,
        "No score mod with 0 cards"
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live_card, HeartColor::Heart02),
        0
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live_card, HeartColor::Heart03),
        0
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live_card, HeartColor::Heart06),
        0
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live_card, HeartColor::Heart00),
        0
    );
}

/// 3+ cards in success zone → condition met, ability triggers (≥2 operator).
#[test]
fn q254_three_cards_triggers() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live_card = game.id("PL!SP-sd2-023-SD2");
    let past_1 = game.id("PL!-sd1-020-SD");
    let past_2 = game.id("PL!-sd1-021-SD");
    let past_3 = game.id("PL!-sd1-020-SD");

    game.state.player1.success_live_card_zone.cards.push(past_1);
    game.state.player1.success_live_card_zone.cards.push(past_2);
    game.state.player1.success_live_card_zone.cards.push(past_3);
    game.state.player1.hand.cards.push(live_card);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.mods.get_score_modifier(live_card),
        5,
        "Score +5 with 3 cards"
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live_card, HeartColor::Heart02),
        3
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live_card, HeartColor::Heart03),
        3
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live_card, HeartColor::Heart06),
        3
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live_card, HeartColor::Heart00),
        3
    );
}

/// With 2+ cards in success zone AND 3 members on stage, the SET modifier
/// replaces the base need_heart. Stage provides only 4 hearts (H01×1, H02×1,
/// H03×1, H06×1) which meets base (H03×1 + H00×2) but NOT modified
/// (H02×3 + H03×3 + H06×3 + H00×3 = 12). Proves player cannot opt out
/// of the harder modified requirement.
#[test]
fn q254_modified_requirement_overrides_base_with_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live_card = game.id("PL!SP-sd2-023-SD2");
    let past_1 = game.id("PL!-sd1-020-SD");
    let past_2 = game.id("PL!-sd1-021-SD");

    let stage_left = game.id("PL!-sd1-010-SD");
    let stage_center = game.id("PL!SP-sd1-020-SD");
    let stage_right = game.id("PL!SP-sd1-019-SD");

    game.add_to_stage(MemberArea::LeftSide, stage_left);
    game.add_to_stage(MemberArea::Center, stage_center);
    game.add_to_stage(MemberArea::RightSide, stage_right);

    game.state.player1.success_live_card_zone.cards.push(past_1);
    game.state.player1.success_live_card_zone.cards.push(past_2);
    game.state.player1.hand.cards.push(live_card);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let h02 = game
        .state
        .mods
        .get_need_heart_modifier(live_card, HeartColor::Heart02);
    let h03 = game
        .state
        .mods
        .get_need_heart_modifier(live_card, HeartColor::Heart03);
    let h06 = game
        .state
        .mods
        .get_need_heart_modifier(live_card, HeartColor::Heart06);
    let h00 = game
        .state
        .mods
        .get_need_heart_modifier(live_card, HeartColor::Heart00);

    assert_eq!(h02, 3, "H02 set to 3");
    assert_eq!(h03, 3, "H03 set to 3");
    assert_eq!(h06, 3, "H06 set to 3");
    assert_eq!(h00, 3, "H00 set to 3");

    let card = game.db.get_card(live_card).unwrap();
    let base = card.need_heart.as_ref().unwrap();
    assert_eq!(*base.hearts.get(&HeartColor::Heart03).unwrap_or(&0), 1);
    assert_eq!(*base.hearts.get(&HeartColor::Heart00).unwrap_or(&0), 2);
    assert_eq!(*base.hearts.get(&HeartColor::Heart02).unwrap_or(&0), 0);
    assert_eq!(*base.hearts.get(&HeartColor::Heart06).unwrap_or(&0), 0);
}

/// With 2+ success zone cards AND 3 high-heart members on stage that
/// collectively provide 15 hearts (H02×5, H03×5, H06×5), the modified
/// requirement (12 hearts) IS met. Demonstrates the live can succeed
/// under the modified requirement.
#[test]
fn q254_modified_requirement_met_with_high_heart_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live_card = game.id("PL!SP-sd2-023-SD2");
    let past_1 = game.id("PL!-sd1-020-SD");
    let past_2 = game.id("PL!-sd1-021-SD");

    let stage_left = game.id("PL!SP-bp1-022-N");
    let stage_center = game.id("PL!SP-sd1-010-SD");
    let stage_right = game.id("PL!SP-PR-005-PR");

    game.add_to_stage(MemberArea::LeftSide, stage_left);
    game.add_to_stage(MemberArea::Center, stage_center);
    game.add_to_stage(MemberArea::RightSide, stage_right);

    game.state.player1.success_live_card_zone.cards.push(past_1);
    game.state.player1.success_live_card_zone.cards.push(past_2);
    game.state.player1.hand.cards.push(live_card);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live_card, HeartColor::Heart02),
        3
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live_card, HeartColor::Heart03),
        3
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live_card, HeartColor::Heart06),
        3
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live_card, HeartColor::Heart00),
        3
    );
}

/// Without the condition (0 success zone cards), the base requirement applies.
/// Same 3-member stage providing 4 hearts passes the base check.
#[test]
fn q254_no_condition_uses_base_requirement() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live_card = game.id("PL!SP-sd2-023-SD2");

    let stage_left = game.id("PL!-sd1-010-SD");
    let stage_center = game.id("PL!SP-sd1-020-SD");
    let stage_right = game.id("PL!SP-sd1-019-SD");

    game.add_to_stage(MemberArea::LeftSide, stage_left);
    game.add_to_stage(MemberArea::Center, stage_center);
    game.add_to_stage(MemberArea::RightSide, stage_right);

    game.state.player1.hand.cards.push(live_card);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.mods.get_score_modifier(live_card),
        0,
        "No score mod without condition"
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live_card, HeartColor::Heart02),
        0
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live_card, HeartColor::Heart03),
        0
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live_card, HeartColor::Heart06),
        0
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live_card, HeartColor::Heart00),
        0
    );

    let card = game.db.get_card(live_card).unwrap();
    let base = card.need_heart.as_ref().unwrap();
    assert_eq!(*base.hearts.get(&HeartColor::Heart03).unwrap_or(&0), 1);
    assert_eq!(*base.hearts.get(&HeartColor::Heart00).unwrap_or(&0), 2);
}
