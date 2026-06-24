/// Tests for PL!SP-bp4-025-L (Special Color) ab#0 — Q195
///
/// ab#0 (ライブ開始時): ライブ終了まで、自分のステージのセンターエリアにいる
///   Liella!のメンバーが持つブレードの数が3つになる。
/// ab#1 (ライブ成功時): センターのLiella!がこのターン移動してたら+1スコア
///
/// Q195: 既に+1ブレードを持っているメンバーにset_blade(3)を使うと？
/// Answer: 4。set_blade(3)で3になってから、既存の+1が乗る。
use crate::helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

fn fill_decks(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

/// Happy path: Liella! member at center gets blade set to 3 (modifier = value - original).
#[test]
fn special_color_q195_set_blade_liella_at_center() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let special = game.id("PL!SP-bp4-025-L");
    let liella = game.id("PL!SP-bp1-001-R"); // blade=3
    let non_liella = game.id("PL!-sd1-010-SD"); // blade=1, not Liella!

    game.state.player1.stage.stage = [non_liella, liella, -1];
    game.state.player1.hand.cards.push(special);
    game.state.player1.hand.cards.push(non_liella);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(special);
    advance_to_live_start(&mut game);

    assert_eq!(
        game.state.mods.get_blade_modifier(liella),
        0,
        "Liella! at center: modifier = 3 - 3 = 0"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(non_liella),
        0,
        "Non-Liella! should not get modifier"
    );
}

/// Liella! at left side should be excluded by position filter.
#[test]
fn liella_at_left_side_excluded_by_position() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let special = game.id("PL!SP-bp4-025-L");
    let liella = game.id("PL!SP-bp1-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [liella, filler, -1];
    game.state.player1.hand.cards.push(special);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(special);
    advance_to_live_start(&mut game);

    assert_eq!(
        game.state.mods.get_blade_modifier(liella),
        0,
        "Liella! at left side should NOT get set_blade_count (position=center filter)"
    );
}

/// Liella! at right side should be excluded by position filter.
#[test]
fn liella_at_right_side_excluded_by_position() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let special = game.id("PL!SP-bp4-025-L");
    let liella = game.id("PL!SP-bp1-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, filler, liella];
    game.state.player1.hand.cards.push(special);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(special);
    advance_to_live_start(&mut game);

    assert_eq!(
        game.state.mods.get_blade_modifier(liella),
        0,
        "Liella! at right side should NOT get set_blade_count (position=center filter)"
    );
}

/// Non-Liella! at center should be excluded by group filter.
#[test]
fn non_liella_at_center_excluded_by_group() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let special = game.id("PL!SP-bp4-025-L");
    let non_liella = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, non_liella, -1];
    game.state.player1.hand.cards.push(special);
    game.state.player1.hand.cards.push(non_liella);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(special);
    advance_to_live_start(&mut game);

    assert_eq!(
        game.state.mods.get_blade_modifier(non_liella),
        0,
        "Non-Liella! at center should NOT get set_blade_count (group=Liella! filter)"
    );
}

/// Multiple Liella! on stage: only the one at center gets the modifier.
#[test]
fn multiple_liella_only_center_gets_modifier() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let special = game.id("PL!SP-bp4-025-L");
    let liella_left = game.id("PL!SP-bp1-001-R");
    let liella_center = game.id("PL!SP-bp1-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [liella_left, liella_center, -1];
    game.state.player1.hand.cards.push(special);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(special);
    advance_to_live_start(&mut game);

    assert_eq!(
        game.state.mods.get_blade_modifier(liella_center),
        0,
        "Liella! at center: modifier = 0 (3-3), total blade = 3"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(liella_left),
        0,
        "Liella! at left side should NOT get modifier (position filter)"
    );
}

/// Center position empty — no one gets the blade modifier.
#[test]
fn center_empty_no_one_gets_modifier() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let special = game.id("PL!SP-bp4-025-L");
    let liella_left = game.id("PL!SP-bp1-001-R");
    let liella_right = game.id("PL!SP-bp1-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [liella_left, -1, liella_right];
    game.state.player1.hand.cards.push(special);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(special);
    advance_to_live_start(&mut game);

    assert_eq!(
        game.state.mods.get_blade_modifier(liella_left),
        0,
        "Liella! at left should not get modifier (center empty, no target)"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(liella_right),
        0,
        "Liella! at right should not get modifier (center empty, no target)"
    );
}

/// All three positions filled with Liella! — only center gets the modifier.
#[test]
fn all_three_liella_only_center_gets_modifier() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let special = game.id("PL!SP-bp4-025-L");
    let liella_left = game.id("PL!SP-bp1-001-R");
    let liella_center = game.id("PL!SP-bp1-001-R");
    let liella_right = game.id("PL!SP-bp1-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [liella_left, liella_center, liella_right];
    game.state.player1.hand.cards.push(special);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(special);
    advance_to_live_start(&mut game);

    assert_eq!(
        game.state.mods.get_blade_modifier(liella_center),
        0,
        "Liella! at center: modifier = 0 (3-3)"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(liella_left),
        0,
        "Liella! at left: no modifier (position filter)"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(liella_right),
        0,
        "Liella! at right: no modifier (position filter)"
    );
}

/// Opponent's Liella! at center should NOT be affected (target=self).
#[test]
fn opponent_liella_not_affected_by_self_target() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let special = game.id("PL!SP-bp4-025-L");
    let self_liella = game.id("PL!SP-bp1-001-R");
    let opp_liella = game.id("PL!SP-bp1-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, self_liella, -1];
    game.state.player2.stage.stage = [-1, opp_liella, -1];
    game.state.player1.hand.cards.push(special);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(special);
    advance_to_live_start(&mut game);

    assert_eq!(
        game.state.mods.get_blade_modifier(self_liella),
        0,
        "Self Liella! at center: modifier = 0 (3-3)"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(opp_liella),
        0,
        "Opponent Liella! should NOT get modifier (target=self)"
    );
}

/// Liella! with original blade=2 at center → modifier = 3-2 = 1, total blade = 3.
/// PR-012-PR has its own LiveStart ability (optional cost) that creates a pending
/// choice — we decline it so set_blade_count can complete.
#[test]
fn liella_blade_2_gets_correct_modifier() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let special = game.id("PL!SP-bp4-025-L");
    let liella_blade2 = game.id("PL!SP-PR-012-PR"); // Liella!, blade=2
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, liella_blade2, -1];
    game.state.player1.hand.cards.push(special);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(special);
    advance_to_live_start(&mut game);

    // PR-012-PR has its own LiveStart ability (optional: discard 1 from hand,
    // gain blade). Decline it so set_blade_count's result is visible.
    if game.state.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.mods.get_blade_modifier(liella_blade2),
        1,
        "Liella! blade=2: modifier = 1 (3-2), total = 3"
    );
}

/// Liella! with original blade=1 at center → modifier = 3-1 = 2, total blade = 3.
#[test]
fn liella_blade_1_gets_correct_modifier() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let special = game.id("PL!SP-bp4-025-L");
    let liella_blade1 = game.id("PL!SP-PR-008-PR"); // Liella!, blade=1, no own abilities
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, liella_blade1, -1];
    game.state.player1.hand.cards.push(special);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(special);
    advance_to_live_start(&mut game);

    assert_eq!(
        game.state.mods.get_blade_modifier(liella_blade1),
        2,
        "Liella! blade=1: modifier = 2 (3-1), total = 3"
    );
}

/// Q195: Liella! with existing +1 blade modifier before set_blade_count(3).
/// After set: set = 0 (3-3), additive stays 1, total modifier = 1, total blade = 4.
#[test]
fn q195_existing_modifier_stacks_on_set_value() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let special = game.id("PL!SP-bp4-025-L");
    let liella = game.id("PL!SP-bp1-001-R"); // blade=3
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, liella, -1];
    game.state.player1.hand.cards.push(special);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game);

    // Add an existing +1 blade modifier (simulating an ongoing effect)
    game.state.mods.add_blade_modifier(liella, 1);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(special);
    advance_to_live_start(&mut game);

    // Q195: after set_blade(3), modifier = set(0) + additive(1) = 1, total = 3 + 1 = 4
    assert_eq!(
        game.state.mods.get_blade_modifier(liella),
        1,
        "Q195: existing +1 + set_blade(3) on blade=3 card → modifier should be 1 (set=0, additive=1)"
    );
}
