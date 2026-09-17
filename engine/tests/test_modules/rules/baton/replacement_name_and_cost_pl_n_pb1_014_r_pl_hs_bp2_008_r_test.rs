use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD";

#[test]
fn pl_n_pb1_014_r_baton_own_name_fire_draw_two_discard_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let old_me = game.id("PL!N-pb1-014-R");
    game.state.player1.stage.stage[0] = old_me;
    let me = game.new_id("PL!N-pb1-014-R");
    game.state.player1.hand.cards.push(me);
    game.give_energy(20);
    let d1 = game.new_id(FILLER);
    let d2 = game.new_id(FILLER);
    game.state.player1.main_deck.cards.push(d1);
    game.state.player1.main_deck.cards.push(d2);
    while game.state.player1.main_deck.cards.len() < 40 {
        let f = game.new_id(FILLER);
        game.state.player1.main_deck.cards.push(f);
    }
    game.play_to_stage(me, MemberArea::LeftSide);
    assert!(
        game.state.player1.hand.cards.contains(&d1) && game.state.player1.hand.cards.contains(&d2),
        "both drawn cards are in hand"
    );
    assert!(
        game.has_pending_choice(),
        "hand-discard prompt expected after the baton debut"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard (hand, count=1)"
    );
    game.select_indices(&[0]);
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
fn pl_n_pb1_014_r_baton_other_name_no_fire_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let other = game.id(FILLER);
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

#[test]
fn pl_hs_bp2_008_r_baton_cheaper_dollchestra_fire_two_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let cheap = game.id("PL!HS-bp2-004-R");
    game.state.player1.stage.stage[0] = cheap;
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
fn pl_hs_bp2_008_r_baton_expensive_dollchestra_no_fire_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
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
