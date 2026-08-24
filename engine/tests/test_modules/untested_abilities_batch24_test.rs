/// Untested-abilities batch 24 — named-baton-source debut draws:
/// - PL!N-pb1-019-R 東條希 (登場): replacing 優木せつ菜 via baton touch ->
///   draw 2, then discard 2 from hand
/// - PL!N-pb1-020-R エマ・ヴェルデ (登場): replacing エマ・ヴェルデ via baton
///   touch -> same. Negative: replacing a different name -> nothing.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD"; // μ's member

fn run_baton_draw_flow(db: std::sync::Arc<rabuka_engine::card::CardDatabase>, me_no: &str, replaced_no: &str) {
    let mut game = TestGame::new(db);
    let replaced = game.id(replaced_no);
    game.state.player1.stage.stage[0] = replaced;

    let me = game.new_id(me_no);
    game.state.player1.hand.cards.push(me);
    game.give_energy(25);

    // Two known draws on top of the deck.
    let d1 = game.new_id(FILLER);
    let d2 = game.new_id(FILLER);
    game.state.player1.main_deck.cards.push(d1);
    game.state.player1.main_deck.cards.push(d2);

    let deck_before = game.state.player1.main_deck.cards.len();
    let waitroom_before = game.state.player1.waitroom.cards.len();

    game.play_to_stage(me, MemberArea::LeftSide);

    // NOTE: the draw+discard resolves fully inside the play action —
    // no pending choice reaches the caller for this ability shape.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        0,
        "drew 2 then discarded 2 -> hand empty"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&d1)
            && game.state.player1.waitroom.cards.contains(&d2),
        "both drawn cards were discarded"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 2,
        "deck shrank by exactly the two draws"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before + 3,
        "+1 replaced member +2 discarded"
    );
}

#[test]
fn pb1019_baton_over_setsuna_draws_two_discards_two() {
    let db = load_real_database();
    run_baton_draw_flow(db, "PL!N-pb1-019-R", "PL!N-PR-009-PR"); // 優木せつ菜
}

#[test]
fn pb1020_baton_over_emma_draws_two_discards_two() {
    let db = load_real_database();
    run_baton_draw_flow(db, "PL!N-pb1-020-R", "PL!N-bp1-020-PRproteinbar"); // エマ・ヴェルデ
}

#[test]
fn pb1019_baton_over_other_name_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let other = game.id(FILLER); // μ's — not 優木せつ菜
    game.state.player1.stage.stage[0] = other;

    let me = game.new_id("PL!N-pb1-019-R");
    game.state.player1.hand.cards.push(me);
    game.give_energy(25);

    let deck_before = game.state.player1.main_deck.cards.len();

    game.play_to_stage(me, MemberArea::LeftSide);

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "wrong replaced name -> no draw"
    );
}
