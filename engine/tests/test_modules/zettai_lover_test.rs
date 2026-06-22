use crate::helpers::*;

fn advance_to_live_start(game: &mut TestGame) {
    game.pass(); // → ActivePhase
    game.pass(); // → EnergyPhase
    game.pass(); // → DrawPhase
    game.pass(); // → MainPhase
    game.pass(); // → LiveCardSetP1
}

fn finish_live_setup(game: &mut TestGame) {
    game.pass(); // LiveCardSetP1 → LiveCardSetP2
    game.pass(); // LiveCardSetP2 → LiveStart
}

/// 絶対的LOVER (PL!SP-pb2-045-L): LiveStart → score +1 per Liella! member
/// with 4+ total hearts. One member has 4 hearts (qualifies), one has 3 (doesn't).
#[test]
fn zettai_lover_heart4_member_gets_score_one_heart3_does_not() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let lover = game.id("PL!SP-pb2-045-L");
    let kanon = game.id("PL!SP-sd1-001-SD"); // 澁谷かのん, hearts=4 (2+2) → qualifies
    let keke = game.id("PL!SP-sd1-002-SD"); // 唐可可, hearts=3 (1+2) → doesn't qualify
    let filler = game.id("PL!-sd1-010-SD");

    // Stage: Kanon (4 hearts) at center, Keke (3 hearts) at left
    game.state.player1.stage.stage = [keke, kanon, -1];

    // Live card in hand
    game.state.player1.hand.cards.push(lover);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_start(&mut game);
    game.set_live_card(lover);
    finish_live_setup(&mut game);

    // The live start ability fires: score +1 per Liella! member with 4+ hearts.
    // Only Kanon (4 hearts) qualifies → score +1 total.
    let live_id = game.state.player1.live_card_zone.cards[0];
    let score_mod = game.state.mods.get_score_modifier(live_id);
    assert!(
        score_mod >= 1,
        "Live card should have at least +1 score from 1 qualifying member (got {})",
        score_mod
    );
    // Keke (3 hearts) should NOT qualify — but this is harder to assert directly.
    // We can verify the total is only +1, not +2.
    assert!(
        score_mod <= 1,
        "Live card should have at most +1 score (only 1 member with 4+ hearts, got {})",
        score_mod
    );
}

/// All Liella! members have <4 hearts → no score bonus.
#[test]
fn zettai_lover_no_member_meets_heart_threshold() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let lover = game.id("PL!SP-pb2-045-L");
    let keke = game.id("PL!SP-sd1-002-SD"); // hearts=3 → <4
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, keke, -1];
    game.state.player1.hand.cards.push(lover);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_start(&mut game);
    game.set_live_card(lover);
    finish_live_setup(&mut game);

    let live_id = game.state.player1.live_card_zone.cards[0];
    let score_mod = game.state.mods.get_score_modifier(live_id);
    assert_eq!(
        score_mod, 0,
        "No score bonus when no member has 4+ hearts (got {})",
        score_mod
    );
}

/// Non-Liella! member with 4+ hearts must NOT be counted.
#[test]
fn zettai_lover_non_liella_high_heart_not_counted() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let lover = game.id("PL!SP-pb2-045-L");
    let kanon = game.id("PL!SP-sd1-001-SD"); // Liella!, hearts=4 → qualifies
    let honoka = game.id("PL!-sd1-010-SD"); // μ's (Printemps), hearts=5+ → NOT Liella!
    let filler = game.id("PL!-sd1-013-SD");

    // Both on stage: Kanon (Liella!, 4 hearts) and Honoka (μ's, 5+ hearts)
    game.state.player1.stage.stage = [kanon, honoka, -1];
    game.state.player1.hand.cards.push(lover);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_start(&mut game);
    game.set_live_card(lover);
    finish_live_setup(&mut game);

    let live_id = game.state.player1.live_card_zone.cards[0];
    let score_mod = game.state.mods.get_score_modifier(live_id);
    // Only Kanon (Liella! 4 hearts) should count, not Honoka (non-Liella!)
    assert_eq!(
        score_mod, 1,
        "Only 1 Liella! member should count (non-Liella! excluded), got {}",
        score_mod
    );
}
