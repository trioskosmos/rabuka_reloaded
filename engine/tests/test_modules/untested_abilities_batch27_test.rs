/// Untested-abilities batch 27 — mill-then-retrieve debuts:
/// - PL!N-bp1-009-R 新田新奈 (登場): mill 2 from deck top -> retrieve a member
///   card from the waitroom to hand. Both mill targets and the retrieved card
///   are pinned by id.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD"; // μ's member

#[test]
fn bp1009_debut_mill2_then_retrieve_member_from_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!N-bp1-009-R");
    let fodder = game.new_id(FILLER);
    game.state.player1.hand.cards.push(me);
    game.state.player1.hand.cards.push(fodder);
    game.give_energy(15);

    // The retrieval target sits in the waitroom.
    let niji = game.id("PL!N-bp3-004-R"); // 虹ヶ咲 member
    game.state.player1.waitroom.cards.push(niji);

    // Deck top: exactly the two cards that get milled.
    let m1 = game.new_id(FILLER);
    let m2 = game.new_id(FILLER);
    game.state.player1.main_deck.cards.push(m1);
    game.state.player1.main_deck.cards.push(m2);

    let waitroom_before = game.state.player1.waitroom.cards.len();

    game.play_to_stage(me, MemberArea::Center);
    // The retrieval step prompts "select member from waitroom" only AFTER
    // the mill resolves — drain every pending choice in order.
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&niji),
        "虹ヶ咲 member retrieved to hand"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&niji),
        "retrieved member left the waitroom"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&m1)
            && game.state.player1.waitroom.cards.contains(&m2),
        "both deck-top cards were milled to the waitroom"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before + 2,
        "+2 milled-in, -1 retrieved-out; waitroom={:?}",
        game.state.player1.waitroom.cards
    );
}
