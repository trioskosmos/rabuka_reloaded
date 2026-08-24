/// Untested-abilities batch 23 — baton-touch-debut gated abilities:
/// - PL!N-pb1-014-R 中須かすみ (登場): replacing her OWN NAME via baton touch ->
///   draw 2, then discard 1 from hand
/// - PL!HS-bp2-008-R 北条そふぃ (登場): appearing over a CHEAPER DOLLCHESTRA
///   member -> +2 blades until live end
///
/// These exercise movement_condition gates driven by baton-touch metadata:
/// source_character name match and replaced-member cost comparison.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD"; // μ's member

// ====================================================================
// PL!N-pb1-014-R 中須かすみ (登場):
// 「「中須かすみ」からバトンタッチして登場した場合、カードを2枚引き、
//   手札を1枚控え室に置く。」
// Engine gate: the REPLACED member's normalized name must contain 中須かすみ.
// ====================================================================

#[test]
fn pb1014_baton_over_own_name_draws_two_discards_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // An older 中須かすみ copy occupies the left area.
    let old_me = game.id("PL!N-pb1-014-R");
    game.state.player1.stage.stage[0] = old_me;

    // The fresh copy comes from hand and replaces her.
    let me = game.new_id("PL!N-pb1-014-R");
    game.state.player1.hand.cards.push(me);
    game.give_energy(20);

    // Deck: two known draws on top.
    let d1 = game.new_id(FILLER);
    let d2 = game.new_id(FILLER);
    game.state.player1.main_deck.cards.push(d1);
    game.state.player1.main_deck.cards.push(d2);
    while game.state.player1.main_deck.cards.len() < 40 {
        let f = game.new_id(FILLER);
        game.state.player1.main_deck.cards.push(f);
    }

    game.play_to_stage(me, MemberArea::LeftSide);

    // Draw 2 landed...
    assert!(
        game.state.player1.hand.cards.contains(&d1) && game.state.player1.hand.cards.contains(&d2),
        "both drawn cards are in hand"
    );
    // ...then the hand-discard cost/effect step asks which card to bin.
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.waitroom.cards.contains(&old_me),
        "the replaced 中須かすみ sits in the waitroom"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "drew 2 then discarded 1 -> one card remains"
    );
    assert!(
        !game.state.player1.waitroom.cards.is_empty(),
        "a discard happened"
    );
}

#[test]
fn pb1014_baton_over_other_name_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let other = game.id(FILLER); // μ's — not 中須かすみ
    game.state.player1.stage.stage[0] = other;

    let me = game.new_id("PL!N-pb1-014-R");
    game.state.player1.hand.cards.push(me);
    game.give_energy(20);

    let deck_before = game.state.player1.main_deck.cards.len();

    game.play_to_stage(me, MemberArea::LeftSide);

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "wrong replaced name -> no draw, no mill"
    );
}

// ====================================================================
// PL!HS-bp2-008-R 北条そふぃ (登場, cost 4):
// 「このメンバーよりコストが低い『DOLLCHESTRA』のメンバーがバトンタッチして
//   登場した場合、ライブ終了時まで、{{blade}}{{blade}}を得る。」
// Reading: she appears by baton-touching OVER a cheaper DOLLCHESTRA teammate.
// ====================================================================

#[test]
fn bp2008_baton_over_cheaper_dollchestra_gains_two_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // Cheap DOLLCHESTRA teammate occupies the left area (cost 2 < 4).
    let cheap = game.id("PL!HS-bp2-004-R");
    game.state.player1.stage.stage[0] = cheap;

    // She arrives via baton touch.
    let me = game.new_id("PL!HS-bp2-008-R");
    game.state.player1.hand.cards.push(me);
    game.give_energy(20);

    game.play_to_stage(me, MemberArea::LeftSide);

    assert_eq!(
        game.state.mods.get_blade_modifier(me),
        2,
        "replaced cheaper DOLLCHESTRA -> +2 blades until live end"
    );
}

#[test]
fn bp2008_baton_over_expensive_dollchestra_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // Expensive DOLLCHESTRA teammate (cost 9 > 4) — gate fails.
    let pricey = game.id("PL!HS-bp1-011-PR");
    game.state.player1.stage.stage[0] = pricey;

    let me = game.new_id("PL!HS-bp2-008-R");
    game.state.player1.hand.cards.push(me);
    game.give_energy(25);

    game.play_to_stage(me, MemberArea::LeftSide);

    assert_eq!(
        game.state.mods.get_blade_modifier(me),
        0,
        "replaced member is MORE expensive -> no blades"
    );
}
