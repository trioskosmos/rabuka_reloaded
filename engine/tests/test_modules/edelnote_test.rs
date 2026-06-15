/// Tests for Edelied (PL!HS-pb1-030-L) — a LIVE card with live start ability:
///
/// ab#0 (ライブ開始時): Until live end, 1 EdelNote member on your stage gains 2 blades,
///   and 1 EdelNote member with a different name gains 2 heart06.
///
/// Fixed by parser + engine:
/// - Parser now extracts target_count from "1人" → target_count=1
/// - Parser now extracts "名前の異なる" → distinct="card_name"
/// - Filter system now supports exclude_cards (cards from prior actions are excluded)
/// - matching_ids_filtered applies target_count + distinct + exclude_cards properly
/// - gain_resource stores selected card IDs in the ability queue for chainability
///
/// Expected correct behavior:
/// - 2 different-named EdelNote members: 1 gets +2 blades, the other gets +2 heart06
/// - 1 EdelNote member: gets +2 blades only (no other name available for hearts)
/// - 3 EdelNote members (2 same name, 1 different): 1 of unique name gets blades,
///   the other unique name gets hearts, same-name member gets nothing
/// - 0 EdelNote members: no effect
use crate::helpers::*;

fn advance_to_live_start(game: &mut TestGame) {
    for _ in 0..20 {
        if game.has_pending_choice() {
            break;
        }
        game.pass();
    }
}

/// 2 EdelNote members with different names:
/// Action 0 (gain 2 blades for 1 EdelNote) selects first matching
/// Action 1 (gain 2 heart06 for 1 DIFFERENT-named EdelNote) excludes first → selects second
#[test]
fn edelnote_two_members_different_names() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let edelied = game.id("PL!HS-pb1-030-L");
    let filler = game.id("PL!-sd1-010-SD");
    let edel1 = game.id("PL!HS-PR-022-PR");
    let edel2 = game.id("PL!HS-PR-023-PR");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.live_card_zone.cards.push(edelied);
    game.state.player1.stage.stage = [edel1, edel2, -1];
    game.state.player1.is_first_attacker = true;
    game.state.player2.is_first_attacker = false;

    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let b1 = game.state.mods.get_blade_modifier(edel1);
    let b2 = game.state.mods.get_blade_modifier(edel2);
    let h1 = game
        .state
        .mods
        .get_heart_modifier(edel1, rabuka_engine::card::HeartColor::Heart06);
    let h2 = game
        .state
        .mods
        .get_heart_modifier(edel2, rabuka_engine::card::HeartColor::Heart06);

    let got_blade = [b1 > 0, b2 > 0];
    let got_heart = [h1 > 0, h2 > 0];
    assert_eq!(
        got_blade.iter().filter(|&&b| b).count(),
        1,
        "Exactly 1 member got blades (target_count=1)"
    );
    assert_eq!(
        got_heart.iter().filter(|&&h| h).count(),
        1,
        "Exactly 1 member got heart06 (target_count=1)"
    );
    let blade_idx = got_blade.iter().position(|&b| b).unwrap();
    let heart_idx = got_heart.iter().position(|&h| h).unwrap();
    assert_ne!(
        blade_idx, heart_idx,
        "Blade and heart go to different members (distinct=card_name)"
    );
}

/// 1 EdelNote member: first action gives +2 blades, second finds no different name → no hearts.
#[test]
fn edelnote_single_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let edelied = game.id("PL!HS-pb1-030-L");
    let filler = game.id("PL!-sd1-010-SD");
    let edel1 = game.id("PL!HS-PR-022-PR");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.live_card_zone.cards.push(edelied);
    game.state.player1.stage.stage = [edel1, -1, -1];
    game.state.player1.is_first_attacker = true;
    game.state.player2.is_first_attacker = false;

    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let blade = game.state.mods.get_blade_modifier(edel1);
    let heart = game
        .state
        .mods
        .get_heart_modifier(edel1, rabuka_engine::card::HeartColor::Heart06);

    assert_eq!(blade, 2, "Only member gets +2 blades");
    assert_eq!(heart, 0, "No heart06 — no different-named member exists");
}

/// 0 EdelNote members: ability fires harmlessly.
#[test]
fn edelnote_no_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let edelied = game.id("PL!HS-pb1-030-L");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.live_card_zone.cards.push(edelied);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.player1.is_first_attacker = true;
    game.state.player2.is_first_attacker = false;

    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
}

/// 3 EdelNote members (2 same name, 1 different):
/// Blade picks 1 unique name, heart picks the other unique name.
/// The duplicate-name member gets nothing (only 2 unique names available).
#[test]
fn edelnote_three_members_two_same_name() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let edelied = game.id("PL!HS-pb1-030-L");
    let filler = game.id("PL!-sd1-010-SD");
    let edel1 = game.id("PL!HS-PR-022-PR");
    let edel2 = game.id("PL!HS-PR-023-PR");
    let edel3 = game.id("PL!HS-PR-032-PR");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.live_card_zone.cards.push(edelied);
    game.state.player1.stage.stage = [edel1, edel2, edel3];
    game.state.player1.is_first_attacker = true;
    game.state.player2.is_first_attacker = false;

    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let got_blade = [
        game.state.mods.get_blade_modifier(edel1),
        game.state.mods.get_blade_modifier(edel2),
        game.state.mods.get_blade_modifier(edel3),
    ];
    let got_heart = [
        game.state
            .mods
            .get_heart_modifier(edel1, rabuka_engine::card::HeartColor::Heart06),
        game.state
            .mods
            .get_heart_modifier(edel2, rabuka_engine::card::HeartColor::Heart06),
        game.state
            .mods
            .get_heart_modifier(edel3, rabuka_engine::card::HeartColor::Heart06),
    ];

    let blade_count = got_blade.iter().filter(|&&b| b > 0).count();
    let heart_count = got_heart.iter().filter(|&&h| h > 0).count();
    let both_count = got_blade
        .iter()
        .zip(got_heart.iter())
        .filter(|(&b, &h)| b > 0 && h > 0)
        .count();

    assert_eq!(
        blade_count, 1,
        "Exactly 1 member gets blades (target_count=1)"
    );
    assert_eq!(
        heart_count, 1,
        "Exactly 1 member gets heart06 (target_count=1)"
    );
    assert_eq!(
        both_count, 0,
        "No member gets both resources (distinct=card_name)"
    );
    assert_eq!(got_blade[2], 0, "Third (duplicate name) gets no blades");
    assert_eq!(got_heart[2], 0, "Third (duplicate name) gets no heart06");
}
