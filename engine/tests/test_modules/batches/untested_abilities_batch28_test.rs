/// Untested-abilities batch 28 — 起動 waitroom retrieval with lilywhite gate:
/// - PL!-pb1-007-R (起動 ターン1回): mill 3 from deck top -> if a lilywhite
///   member is on own stage, retrieve a μ's live card from waitroom to hand.
///   Positive (lilywhite staged) + negative (no lilywhite).
use crate::helpers::*;

const FILLER: &str = "PL!-sd1-010-SD"; // μ's member

#[test]
fn pb1007_with_lilywhite_member_retrieves_mus_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!-pb1-007-R");
    let mate = game.id("PL!-bp3-014-N"); // lilywhite member
    game.state.player1.stage.stage[0] = me;
    game.state.player1.stage.stage[1] = mate;
    game.give_energy(15);

    // Hand cost: discard 3 (reduced by success-zone cards, none staged here).
    let f1 = game.new_id(FILLER);
    let f2 = game.new_id(FILLER);
    let f3 = game.new_id(FILLER);
    game.state.player1.hand.cards.push(f1);
    game.state.player1.hand.cards.push(f2);
    game.state.player1.hand.cards.push(f3);

    // μ's live card sits in the waitroom for retrieval.
    let mus_live = game.id("PL!-sd1-020-SD");
    game.state.player1.waitroom.cards.push(mus_live);

    game.activate_ability(me);
    assert!(
        game.has_pending_choice(),
        "hand cost prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard for the 3-card hand cost"
    );
    game.select_indices(&[0, 1, 2]);

    assert!(
        game.state.player1.hand.cards.contains(&mus_live),
        "lilywhite member on stage -> retrieve μ's live card to hand"
    );
}

#[test]
fn pb1007_without_lilywhite_member_no_retrieval() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!-pb1-007-R");
    // Printemps member (μ's but NOT lilywhite).
    let printemps = game.id("PL!-sd1-001-SD");
    game.state.player1.stage.stage[0] = me;
    game.state.player1.stage.stage[1] = printemps;
    game.give_energy(15);

    let f1 = game.new_id(FILLER);
    let f2 = game.new_id(FILLER);
    let f3 = game.new_id(FILLER);
    game.state.player1.hand.cards.push(f1);
    game.state.player1.hand.cards.push(f2);
    game.state.player1.hand.cards.push(f3);

    let mus_live = game.id("PL!-sd1-020-SD");
    game.state.player1.waitroom.cards.push(mus_live);

    game.activate_ability(me);
    assert!(
        game.has_pending_choice(),
        "hand cost prompt expected even when retrieval condition fails"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard for the 3-card hand cost"
    );
    game.select_indices(&[0, 1, 2]);

    assert!(
        !game.state.player1.hand.cards.contains(&mus_live),
        "no lilywhite member -> no retrieval"
    );
}
