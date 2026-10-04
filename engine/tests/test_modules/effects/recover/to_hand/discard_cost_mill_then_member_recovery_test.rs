use crate::helpers::*;
use rabuka_engine::card::CardType;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD";

/// 「その後」 — the recovery pool includes the two cards the mill just discarded.
///
/// The test above puts the retrieved member in the waitroom BEFORE the ability
/// resolves, so it passes whether the recovery runs before or after the mill. That
/// makes the 「その後」 the only thing the sentence contributes, and it was not
/// load-bearing anywhere.
///
/// Here the waitroom starts EMPTY and both milled cards are members, so the only
/// card the recovery can possibly return is one the mill put there a step earlier.
/// If the two clauses swapped, the recovery would find an empty waitroom, take
/// nothing, and the hand would gain nothing at all.
#[test]
fn rina_n_bp1_009_the_recovered_member_can_be_one_the_mill_just_discarded() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!N-bp1-009-R");
    // The cost card is a LIVE card on purpose. 「手札を1枚控え室に置いてもよい」 has
    // no card-type filter, so a live card is a legal cost — and the recovery's
    // `card_type: member_card` filter then excludes it. Using a member as the cost
    // puts it in the recovery pool, where it is the lowest index and therefore the
    // card that gets picked; the test then passes while pinning nothing, because the
    // card in hand is the one the ability already started with.
    let cost_card = game.id("LL-bp5-001-L"); // a live card, no 登場 of its own
    // Two DISTINCT members, so "one came back and one stayed" is checkable without
    // depending on which the engine offers first.
    let milled_a = game.id("PL!N-bp3-004-R"); // 虹ヶ咲 member
    let milled_b = game.id(FILLER); // 絢瀬絵里, also a member

    // Read the types back rather than trusting the comments above: this whole test
    // rests on the cost card being the one card the recovery cannot pick.
    let card_type = |id: i16| game.db.get_card(id).map(|c| c.card_type.clone());
    assert_eq!(
        card_type(cost_card),
        Some(CardType::Live),
        "LL-bp5-001-L must be a live card for the member filter to exclude it"
    );
    for (id, label) in [(milled_a, "milled_a"), (milled_b, "milled_b")] {
        assert_eq!(
            card_type(id),
            Some(CardType::Member),
            "{label} must be a member or there is nothing to recover"
        );
    }

    game.state.player1.hand.cards.push(me);
    game.state.player1.hand.cards.push(cost_card);
    game.give_energy(15);

    // Nothing in the waitroom: every member in it at recovery time came off the deck.
    assert!(
        game.state.player1.waitroom.cards.is_empty(),
        "fixture: the waitroom starts empty so the recovery cannot be satisfied \
         from before the mill"
    );
    game.state.player1.main_deck.cards.push(milled_a);
    game.state.player1.main_deck.cards.push(milled_b);

    game.play_to_stage(me, MemberArea::Center);
    // A loop, and the drain-loop ratchet budget for this file records it. The prompt
    // sequence here is driven by the effect's own shape — a `SelectCard` over the
    // milled cards, then whatever the recovery asks — so fixed arms are a guess. An
    // earlier attempt asserted `SelectTarget` first and failed: the recovery's select
    // comes before any cost prompt, because this ability has no cost.
    let mut guard = 0;
    while game.has_pending_choice() {
        guard += 1;
        assert!(guard < 10, "prompts did not settle");
        game.select_indices(&[0]);
    }

    let took_a = game.state.player1.hand.cards.contains(&milled_a);
    let took_b = game.state.player1.hand.cards.contains(&milled_b);
    assert_eq!(
        took_a ^ took_b,
        true,
        "exactly one of the two milled members comes back — a card that was on \
         the deck one step earlier is now in hand.\n  a={milled_a} took={took_a}\n  \
         b={milled_b} took={took_b}\n  deck={:?}\n  waitroom={:?}\n  hand={:?}",
        game.state.player1.main_deck.cards,
        game.state.player1.waitroom.cards,
        game.state.player1.hand.cards,
    );
    assert!(
        !game.state.player1.hand.cards.contains(&cost_card),
        "the cost card is a live, so the member-card filter must keep it out of \
         the recovery — otherwise this test cannot tell the two apart"
    );
    let stayed = if took_a { milled_b } else { milled_a };
    assert!(
        game.state.player1.waitroom.cards.contains(&stayed),
        "the member that was not taken stays milled; waitroom is {:?}",
        game.state.player1.waitroom.cards
    );
}

/// Declining the optional cost resolves the whole ability — the mill does not
/// happen either.
///
/// This is the same parse shape as 米女メイ (`PL!SP-bp2-007`,
/// `mei_debut_optional_cost_look_five_take_one_liella_member_test.rs`): a cost of
/// `type: move_cards`, `source: hand`, `optional: true`, and a 「：」 before the
/// effect. Two cards, one mechanic, so they are pinned together — an engine that
/// treated them differently would be making the same printed shape mean two things.
///
/// The deck is the assertion that matters. 「手札を1枚控え室に置いてもよい」 not only
/// makes the hand discard optional; the mill would still be a net gain if it ran
/// afterwards, so "the hand is unchanged" alone cannot tell a correct decline from
/// an ability that did nothing.
#[test]
fn rina_n_bp1_009_declining_the_optional_cost_stops_the_mill() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!N-bp1-009-R");
    let fodder = game.new_id(FILLER);
    game.state.player1.hand.cards.push(me);
    game.state.player1.hand.cards.push(fodder);
    game.give_energy(15);

    let milled = game.new_id("PL!N-bp3-004-R");
    game.state.player1.main_deck.cards.push(milled);
    let deck_before = game.state.player1.main_deck.cards.len();
    let waitroom_before = game.state.player1.waitroom.cards.len();

    game.play_to_stage(me, MemberArea::Center);
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "the optional cost asks which hand card to discard"
    );
    assert!(
        game.get_pending_choice().allow_skip(),
        "「…てもよい」 — the cost must be skippable"
    );
    game.select_indices(&[]); // decline
    // Nothing may follow a declined cost: 「…てもよい：」 gates the whole sentence, so
    // the mill and the 「その後」 recovery must both be silent. `drain_choices_strict`
    // with an empty `allowed` list turns any leftover prompt into this failure, which
    // is the assertion this test exists to make — and it is strictly stronger than a
    // loop that answers whatever appears.
    game.drain_choices_strict(&[], &[]);

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "declining the cost must not mill: 「…てもよい：」 gates the whole sentence"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before,
        "nor discard the cost card, nor 「その後」 recover; waitroom is {:?}",
        game.state.player1.waitroom.cards
    );
}

#[test]
fn rina_n_bp1_009_debut_discard_cost_mills_two_then_recovers_member() {
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
