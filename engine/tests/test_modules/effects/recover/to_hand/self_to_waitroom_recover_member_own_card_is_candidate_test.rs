/// The 30-card 「このメンバーをステージから控え室に置く：自分の控え室からメンバーカードを
/// 1枚手札に加える」 family — the MEMBER-recovery variant, which has no test as a
/// subject of its own.
///
/// `cards/abilities.json`: 起動, `cost: move_cards { self_cost: true, stage -> discard }`,
/// `effect: move_cards { source: discard, destination: hand, card_type: member_card }`.
///
/// # Why the sibling variant's tests do not cover this one
///
/// There are two families with the same 30-card-shaped cost and opposite recovery
/// filters, and the LIVE one is covered
/// (`self_to_waitroom_recover_live_q79_q123_test.rs`). Its central finding is that the
/// sacrificed member is NOT a candidate there — because the filter is
/// `card_type: live_card` and the cost put a member in the waitroom.
///
/// Here the filter is `card_type: member_card`. The sacrificed member IS a member card
/// and IS in the waitroom. Nothing in the text excludes it, so it qualifies — and with
/// an empty waitroom it is the ONLY candidate, so the ability recovers the very card
/// its cost just sacrificed.
///
/// That makes the card a self-cancelling search: 「look in my waitroom for a member」
/// with a self-sacrifice that can be undone. An implementation that snapshots
/// "cards moved by this cost" and excludes the mover — a reasonable-looking
/// optimisation, and the shape `ability/effects/deck.rs:234` uses for the mirrored
/// 「これにより…ウェイト状態にした」 wording — would find nothing here.
///
/// # Fixtures verified
///
/// All 30 cards are `type: メンバー` and `cost: 2`, read from `cards.json`, so the
/// activation is affordable and every candidate genuinely satisfies the filter. Cards
/// are picked from across the family's units (Printemps/BiBi, 虹ヶ咲, Liella!,
/// 5yncri5e!, lily white, AZALEA, CatChu!) so nothing here depends on group matching.
use crate::helpers::*;

const SUBJECT: &str = "PL!S-bp2-016-N"; // 国木田 花丸, cost 2
const OTHER: &str = "PL!N-bp4-017-N"; // 宮下 愛, cost 2

/// Stage `subject`, put `waitroom` in the waitroom, activate, and report the subject's
/// id alongside the resulting hand and waitroom.
fn recovered_into_hand(subject: &str, waitroom: &[&str]) -> (i16, Vec<i16>, Vec<i16>) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id(subject);
    for card_no in waitroom {
        let id = game.id(card_no);
        assert_eq!(
            game.db.get_card(id).map(|c| c.card_type.clone()),
            Some(rabuka_engine::card::CardType::Member),
            "fixture: {card_no} must be a メンバー — the recovery filters on member_card"
        );
        game.state.player1.waitroom.cards.push(id);
    }

    game.state.player1.stage.stage[1] = me;
    game.give_energy(5);
    game.activate_ability(me);
    // Nothing is asked. With an empty waitroom the sacrificed member is the only
    // candidate, so the recovery resolves on its own; and the prompted case has its
    // own test, which asserts the prompt rather than tolerating it here. Asserting
    // the absence is stronger than a drain that answers whatever turns up
    // (docs/EDGE_CASE_FINDINGS.md C9), and it keeps this file off the soft-guard and
    // drain-loop ratchets.
    game.drain_choices_strict(&[], &[]);

    (
        me,
        game.state.player1.hand.cards.iter().copied().collect(),
        game.state.player1.waitroom.cards.iter().copied().collect(),
    )
}

/// The cost undoes itself: with an empty waitroom the sacrificed member is the only
/// candidate, so she comes straight back to hand.
#[test]
fn the_sacrificed_member_is_recovered_by_her_own_ability() {
    let (me, hand, waitroom) = recovered_into_hand(SUBJECT, &[]);

    assert!(
        hand.contains(&me),
        "the cost put her in the waitroom and 「自分の控え室からメンバーカードを1枚」 \
         takes her straight back out — nothing in the text excludes the card this \
         ability just moved; hand={hand:?}"
    );
    assert!(
        waitroom.is_empty(),
        "she left the waitroom again; waitroom={waitroom:?}"
    );
}

/// With a second member in the waitroom there are two candidates, so the engine
/// must ask — and the sacrificed member must be one of the options.
///
/// This is the test that pins candidacy rather than coincidence. Test 1 shows the
/// outcome; this one shows she is genuinely selectable, which is the property an
/// implementation excluding "the card this cost moved" would lose.
#[test]
fn the_sacrificed_member_is_offered_beside_a_pre_existing_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id(SUBJECT);
    let other = game.id(OTHER);
    game.state.player1.waitroom.cards.push(other);
    game.state.player1.stage.stage[1] = me;
    game.give_energy(5);

    game.activate_ability(me);
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "two member cards are in the waitroom, so the recovery asks which"
    );
    let me_idx = game
        .state
        .player1
        .waitroom
        .cards
        .iter()
        .position(|&id| id == me)
        .expect("the cost put her in the waitroom");
    game.select_indices(&[me_idx]);
    game.drain_choices_strict(&[], &[]);

    assert!(
        game.state.player1.hand.cards.contains(&me),
        "picking her returns her to hand — she is a candidate, not merely present"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&other),
        "the other member stays; only one card was recovered"
    );
}

/// A card that is NOT a member is not recovered, even though it is in the waitroom.
///
/// Pins the filter as the thing that excludes. Without it, the two tests above pass on
/// an implementation that recovers any card from the waitroom.
#[test]
fn a_live_in_the_waitroom_is_not_a_candidate() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id(SUBJECT);
    let live = game.id("PL!-sd1-019-SD");
    game.state.player1.waitroom.cards.push(live);
    game.state.player1.stage.stage[1] = me;
    game.give_energy(5);

    game.activate_ability(me);
    game.drain_choices_strict(&[], &[]);

    assert!(
        !game.state.player1.hand.cards.contains(&live),
        "「メンバーカードを1枚」 — a live card in the waitroom is not a candidate"
    );
}