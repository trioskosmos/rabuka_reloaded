/// Tests for set_card_identity constant ability (Q-pattern gameplay tests).
///
/// Card tested:
/// - PL!HS-bp5-018-L (AURORA FLOWER) — 常時: すべての領域にあるこのカードは
///   『スリーズブーケ』、『DOLLCHESTRA』、『みらくらぱーく！』として扱う。
/// - Shared with PL!HS-bp2-020-L (Link to the FUTURE) and PL!HS-sd1-020-SD
///   (Link to the FUTURE 104期Ver.).
///
/// Engine fix: `card_matches_group_str` (engine/src/ability/util.rs:272) now also
/// checks each card's `set_card_identity` abilities for additional group identities
/// so the effect is honored in all zones. Same fix applied to the inline check in
/// `evaluate_multi_count_condition` (engine/src/ability/condition/card.rs:650).
use crate::helpers::*;
use rabuka_engine::ability::util::card_matches_group_str;

#[test]
fn aurora_flower_treated_as_three_groups_in_stage() {
    // Direct lookup test: place AURORA FLOWER on stage, recalc constants,
    // verify it matches each of the 3 identities granted by set_card_identity.
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let aurora = game.id("PL!HS-bp5-018-L");
    game.state.player1.stage.stage[1] = aurora;
    game.state.recalculate_constants();

    for group in &["スリーズブーケ", "DOLLCHESTRA", "みらくらぱーく！"] {
        assert!(
            card_matches_group_str(&game.db, aurora, Some(group)),
            "AURORA FLOWER should be treated as {} via set_card_identity",
            group
        );
    }
}

#[test]
fn aurora_flower_does_not_match_unrelated_groups() {
    // Per-identity check: set_card_identity grants SPECIFIC identities, not
    // a magic "match everything" flag. AURORA FLOWER's static group is 蓮ノ空
    // and its set_card_identity is [スリーズブーケ, DOLLCHESTRA, みらくらぱーく！].
    // It must NOT match unrelated groups like Aqours or μ's.
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let aurora = game.id("PL!HS-bp5-018-L");
    game.state.player1.stage.stage[1] = aurora;
    game.state.recalculate_constants();

    assert!(
        !card_matches_group_str(&game.db, aurora, Some("Aqours")),
        "AURORA FLOWER has no set_card_identity for Aqours and is not in Aqours series"
    );
    assert!(
        !card_matches_group_str(&game.db, aurora, Some("μ's")),
        "AURORA FLOWER has no set_card_identity for μ's"
    );
    assert!(
        !card_matches_group_str(&game.db, aurora, Some("虹ヶ咲")),
        "AURORA FLOWER has no set_card_identity for 虹ヶ咲"
    );
    assert!(
        !card_matches_group_str(&game.db, aurora, Some("Liella!")),
        "AURORA FLOWER has no set_card_identity for Liella!"
    );
    // And its real group still works (no regression on the existing static path).
    assert!(
        card_matches_group_str(&game.db, aurora, Some("蓮ノ空")),
        "AURORA FLOWER's static group 蓮ノ空 must still match"
    );
}

#[test]
fn aoku_haruka_live_start_scores_with_aurora_in_discard() {
    // Full integration test: PL!HS-bp2-022-L (アオクハルカ) live start ability:
    //   "自分の控え室に『スリーズブーケ』のライブカードが3枚以上ある場合、
    //    このカードのスコアを＋１する。"
    //
    // Setup: 2 static スリーズブーケ live cards + AURORA FLOWER (whose set_card_identity
    // makes it a 3rd スリーズブーケ live card) in the discard. Without the fix,
    // AURORA FLOWER wouldn't count and the condition would fail (count = 2 < 3).
    // With the fix, count = 3, condition passes, +1 score modifier applies.
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let aoku = game.id("PL!HS-bp2-022-L");
    let aurora = game.id("PL!HS-bp5-018-L");
    let holiday = game.id("PL!HS-bp1-021-L"); // Holiday∞Holiday (スリーズブーケ live)
    let kagayaku = game.id("PL!HS-bp2-021-L"); // 眩耀夜行 (スリーズブーケ live)
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(aoku);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.waitroom.cards.push(aurora);
    game.state.player1.waitroom.cards.push(holiday);
    game.state.player1.waitroom.cards.push(kagayaku);
    game.give_energy(10);

    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
    game.set_live_card(aoku);
    game.pass();
    game.pass();
    while game.has_pending_choice() {
        game.select_option(0);
    }

    let score_mod = game.state.mods.get_score_modifier(aoku);
    assert_eq!(
        score_mod, 1,
        "アオクハルカ should gain +1 score: 2 static スリーズブーケ live cards (holiday, \
         kagayaku) + AURORA FLOWER (スリーズブーケ via set_card_identity) = 3 cards in discard"
    );
}

#[test]
fn aoku_haruka_live_start_no_score_when_aurora_missing() {
    // Negative integration test: same setup as above but without AURORA FLOWER.
    // Only 2 static スリーズブーケ live cards in discard, below the threshold of 3.
    // The condition fails and no score modifier is applied — proving the previous
    // test is meaningful (it doesn't trivially pass).
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let aoku = game.id("PL!HS-bp2-022-L");
    let holiday = game.id("PL!HS-bp1-021-L");
    let kagayaku = game.id("PL!HS-bp2-021-L");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(aoku);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.waitroom.cards.push(holiday);
    game.state.player1.waitroom.cards.push(kagayaku);
    game.give_energy(10);

    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
    game.set_live_card(aoku);
    game.pass();
    game.pass();
    while game.has_pending_choice() {
        game.select_option(0);
    }

    let score_mod = game.state.mods.get_score_modifier(aoku);
    assert_eq!(
        score_mod, 0,
        "Without AURORA FLOWER, only 2 static スリーズブーケ live cards in discard — \
         threshold of 3 not met, no score modifier"
    );
}

#[test]
fn aoku_haruka_live_start_fails_with_only_non_suzu_live_cards() {
    // Proves the condition requires スリーズブーケ group — replacing the 2 static
    // スリーズブーケ live cards with non-スリーズブーケ ones should make the
    // condition fail even with AURORA FLOWER (which would be the only match).
    // Count = 1 (just AURORA FLOWER) < 3 → no score.
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let aoku = game.id("PL!HS-bp2-022-L");
    let aurora = game.id("PL!HS-bp5-018-L");
    let start_dash = game.id("PL!-sd1-019-SD"); // μ's live, NOT スリーズブーケ
    let colorful = game.id("PL!N-sd1-025-SD"); // 虹ヶ咲 live, NOT スリーズブーケ
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(aoku);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.waitroom.cards.push(aurora);
    game.state.player1.waitroom.cards.push(start_dash);
    game.state.player1.waitroom.cards.push(colorful);
    game.give_energy(10);

    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
    game.set_live_card(aoku);
    game.pass();
    game.pass();
    while game.has_pending_choice() {
        game.select_option(0);
    }

    let score_mod = game.state.mods.get_score_modifier(aoku);
    assert_eq!(
        score_mod, 0,
        "Only AURORA FLOWER matches スリーズブーケ (1 card), below threshold 3 — \
         no score modifier even with the set_card_identity fix"
    );
}
